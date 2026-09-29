use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use appfw_runtime::graph_provider::{
    RuntimeGraphMutationResult, RuntimeGraphNamedQuery, RuntimeGraphProvider,
    RuntimeGraphProviderIdentity, RuntimeGraphQueryResult,
};
use appfw_runtime::{PolicyAccess, RuntimeError, UserAuth};
use async_trait::async_trait;
use chrono::Utc;
use serde_json::json;
use tokio::time::Instant;
use uuid::Uuid;

use crate::audit::{
    Neo4jGraphAuditSink, Neo4jGraphMutationAudit, Neo4jGraphReadAudit, Neo4jGraphReadOutcome,
    TracingGraphAuditSink,
};
use crate::execution::{Neo4jAccessMode, Neo4jExecutionRequest, Neo4jExecutor, Neo4jWriteStats};
use crate::limits::{Neo4jEffectiveLimits, Neo4jProviderLimits};
use crate::query::{
    cypher_references_parameter, validate_governed_write_cypher, validate_read_only_cypher,
    validate_traversal_depth, Neo4jGraphMutation, Neo4jGraphQuery,
};
use crate::shared::{map_execution_error, merge_bound_parameters, normalize_ws, value_to_locator};

const PROVIDER_KEY: &str = "neo4j";
const DEFAULT_SLOW_QUERY_THRESHOLD_MS: u64 = 1_000;

/// The Neo4j graph provider.
///
/// It owns a registry of vetted named read queries and an opt-in registry of
/// governed write mutations, and is the only path to graph data. Each execution
/// is guarded end to end: policy access, tenant scoping, read-only / governed-
/// write validation, traversal-depth and result/timeout caps, an audited,
/// redacted trail, and observable projection freshness. Graph access is
/// read-first — writes are governed and gated at the operation/exposure layer
/// (a product chooses which write operations to register and expose). The driver
/// is injected through [`Neo4jExecutor`], so all of this is exercised without a
/// live database.
pub struct Neo4jGraphProvider<E: Neo4jExecutor> {
    data_source_name: String,
    database: String,
    executor: E,
    limits: Neo4jProviderLimits,
    audit_sink: Arc<dyn Neo4jGraphAuditSink>,
    slow_query_threshold_ms: u64,
    queries: BTreeMap<String, Neo4jGraphQuery>,
    mutations: BTreeMap<String, Neo4jGraphMutation>,
}

impl<E: Neo4jExecutor> Neo4jGraphProvider<E> {
    pub fn new(
        data_source_name: impl Into<String>,
        database: impl Into<String>,
        executor: E,
    ) -> Self {
        Self {
            data_source_name: data_source_name.into(),
            database: database.into(),
            executor,
            limits: Neo4jProviderLimits::default(),
            audit_sink: Arc::new(TracingGraphAuditSink),
            slow_query_threshold_ms: DEFAULT_SLOW_QUERY_THRESHOLD_MS,
            queries: BTreeMap::new(),
            mutations: BTreeMap::new(),
        }
    }

    pub fn with_limits(mut self, limits: Neo4jProviderLimits) -> Self {
        self.limits = limits;
        self
    }

    pub fn with_audit_sink(mut self, audit_sink: Arc<dyn Neo4jGraphAuditSink>) -> Self {
        self.audit_sink = audit_sink;
        self
    }

    pub fn with_slow_query_threshold_ms(mut self, threshold_ms: u64) -> Self {
        self.slow_query_threshold_ms = threshold_ms;
        self
    }

    /// Register a vetted named read query. The operation name must be unique.
    pub fn register_query(mut self, query: Neo4jGraphQuery) -> Self {
        self.queries.insert(query.name.clone(), query);
        self
    }

    pub fn register_queries(mut self, queries: impl IntoIterator<Item = Neo4jGraphQuery>) -> Self {
        for query in queries {
            self.queries.insert(query.name.clone(), query);
        }
        self
    }

    /// Register a vetted governed mutation. The operation name must be unique.
    pub fn register_mutation(mut self, mutation: Neo4jGraphMutation) -> Self {
        self.mutations.insert(mutation.name.clone(), mutation);
        self
    }

    pub fn register_mutations(
        mut self,
        mutations: impl IntoIterator<Item = Neo4jGraphMutation>,
    ) -> Self {
        for mutation in mutations {
            self.mutations.insert(mutation.name.clone(), mutation);
        }
        self
    }

    /// Names of every registered operation (read queries and write mutations).
    pub fn registered_operations(&self) -> Vec<&str> {
        self.queries
            .keys()
            .chain(self.mutations.keys())
            .map(String::as_str)
            .collect()
    }

    fn read_audit_seed(
        &self,
        operation: &str,
        user: &UserAuth,
        tenant_id: Option<String>,
    ) -> Neo4jGraphReadAudit {
        Neo4jGraphReadAudit {
            audit_id: Uuid::new_v4().to_string(),
            occurred_at: Utc::now(),
            provider: PROVIDER_KEY,
            data_source: self.data_source_name.clone(),
            operation: operation.to_string(),
            outcome: Neo4jGraphReadOutcome::Rejected,
            actor_user_name: user.user_name.clone(),
            actor_roles: user.roles.clone(),
            tenant_id,
            start_node: None,
            declared_depth: 0,
            effective_depth: 0,
            effective_max_results: 0,
            effective_timeout_ms: 0,
            limits_clamped: false,
            result_count: 0,
            truncated: false,
            graph_freshness: None,
            freshness_lag_ms: None,
            duration_ms: 0,
            redacted_parameters: Vec::new(),
            error: None,
        }
    }

    fn mutation_audit_seed(
        &self,
        operation: &str,
        user: &UserAuth,
        tenant_id: Option<String>,
    ) -> Neo4jGraphMutationAudit {
        Neo4jGraphMutationAudit {
            audit_id: Uuid::new_v4().to_string(),
            occurred_at: Utc::now(),
            provider: PROVIDER_KEY,
            data_source: self.data_source_name.clone(),
            operation: operation.to_string(),
            outcome: Neo4jGraphReadOutcome::Rejected,
            actor_user_name: user.user_name.clone(),
            actor_roles: user.roles.clone(),
            tenant_id,
            start_node: None,
            declared_depth: 0,
            effective_depth: 0,
            effective_max_results: 0,
            effective_timeout_ms: 0,
            limits_clamped: false,
            mutation_stats: Neo4jWriteStats::default(),
            result_count: 0,
            truncated: false,
            graph_freshness: None,
            freshness_lag_ms: None,
            duration_ms: 0,
            redacted_parameters: Vec::new(),
            error: None,
        }
    }

    fn reject_read(
        &self,
        mut audit: Neo4jGraphReadAudit,
        outcome: Neo4jGraphReadOutcome,
        error: RuntimeError,
    ) -> RuntimeError {
        audit.outcome = outcome;
        audit.error = Some(error.to_string());
        self.audit_sink.record(&audit);
        error
    }

    fn reject_mutation(
        &self,
        mut audit: Neo4jGraphMutationAudit,
        outcome: Neo4jGraphReadOutcome,
        error: RuntimeError,
    ) -> RuntimeError {
        audit.outcome = outcome;
        audit.error = Some(error.to_string());
        self.audit_sink.record_mutation(&audit);
        error
    }
}

impl<E: Neo4jExecutor> RuntimeGraphProviderIdentity for Neo4jGraphProvider<E> {
    fn graph_data_source_name(&self) -> &str {
        &self.data_source_name
    }
}

#[async_trait]
impl<E: Neo4jExecutor> RuntimeGraphProvider for Neo4jGraphProvider<E> {
    type Error = RuntimeError;

    async fn health_check(&self) -> Result<(), RuntimeError> {
        self.executor
            .verify_connectivity()
            .await
            .map_err(map_execution_error)
    }

    async fn execute_named_query(
        &self,
        query: RuntimeGraphNamedQuery,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<RuntimeGraphQueryResult, RuntimeError> {
        // Only registered named operations may execute: arbitrary queries are
        // never accepted from GraphQL, MCP, Kafka, or browser clients.
        let registered = match self.queries.get(&query.name) {
            Some(registered) => registered.clone(),
            None => {
                let audit = self.read_audit_seed(&query.name, user, None);
                return Err(self.reject_read(
                    audit,
                    Neo4jGraphReadOutcome::Rejected,
                    RuntimeError::Validation(format!("unknown graph operation `{}`", query.name)),
                ));
            }
        };

        let tenant_scoped = registered.is_tenant_scoped();
        let tenant_id = tenant_scoped.then(|| user.tenant_id.clone());
        let mut audit = self.read_audit_seed(&registered.name, user, tenant_id.clone());
        audit.declared_depth = registered.limits.max_depth;

        if !access.allow {
            return Err(self.reject_read(
                audit,
                Neo4jGraphReadOutcome::DeniedAccess,
                RuntimeError::AccessDenied,
            ));
        }

        if tenant_scoped && user.tenant_id.trim().is_empty() {
            return Err(self.reject_read(
                audit,
                Neo4jGraphReadOutcome::Rejected,
                RuntimeError::Validation(format!(
                    "graph operation `{}` is tenant-scoped but the caller has no tenant",
                    registered.name
                )),
            ));
        }

        if !query.query_text.trim().is_empty()
            && normalize_ws(&query.query_text) != normalize_ws(&registered.cypher)
        {
            return Err(self.reject_read(
                audit,
                Neo4jGraphReadOutcome::Rejected,
                RuntimeError::Validation(format!(
                    "graph operation `{}` cypher does not match the registered definition",
                    registered.name
                )),
            ));
        }

        if let Err(error) = validate_read_only_cypher(&registered.cypher) {
            return Err(self.reject_read(
                audit,
                Neo4jGraphReadOutcome::Rejected,
                RuntimeError::Validation(error.to_string()),
            ));
        }

        let effective = self.limits.clamp(&registered.limits);
        apply_effective_limits(&mut audit, &effective);

        if let Err(error) = validate_traversal_depth(&registered.cypher, effective.max_depth) {
            return Err(self.reject_read(
                audit,
                Neo4jGraphReadOutcome::Rejected,
                RuntimeError::Validation(error.to_string()),
            ));
        }

        if let Some(tenant_parameter) = &registered.tenant_parameter {
            if !cypher_references_parameter(&registered.cypher, tenant_parameter) {
                return Err(self.reject_read(
                    audit,
                    Neo4jGraphReadOutcome::Rejected,
                    RuntimeError::Validation(format!(
                        "tenant-scoped graph operation `{}` must reference parameter `${}`",
                        registered.name, tenant_parameter
                    )),
                ));
            }
        }

        let parameters = match merge_bound_parameters(
            &registered.name,
            registered.tenant_parameter.as_deref(),
            &registered.declared_parameter_names(),
            &registered.parameters,
            &query.parameters,
            user,
        ) {
            Ok(parameters) => parameters,
            Err(error) => {
                return Err(self.reject_read(audit, Neo4jGraphReadOutcome::Rejected, error));
            }
        };

        if let Some(start_parameter) = &registered.start_node_parameter {
            audit.start_node = parameters.get(start_parameter).map(value_to_locator);
        }
        audit.redacted_parameters = parameters
            .keys()
            .filter(|name| registered.tenant_parameter.as_deref() != Some(name.as_str()))
            .cloned()
            .collect();

        let request = Neo4jExecutionRequest {
            database: self.database.clone(),
            operation: registered.name.clone(),
            cypher: registered.cypher.clone(),
            parameters,
            access_mode: Neo4jAccessMode::Read,
            max_results: effective.max_results,
            timeout_ms: effective.timeout_ms,
        };

        let started = Instant::now();
        let outcome = tokio::time::timeout(
            Duration::from_millis(effective.timeout_ms),
            self.executor.run_read_query(request),
        )
        .await;
        let duration_ms = started.elapsed().as_millis() as u64;
        audit.duration_ms = duration_ms;

        let outcome = match outcome {
            Err(_elapsed) => {
                return Err(self.reject_read(
                    audit,
                    Neo4jGraphReadOutcome::Failed,
                    RuntimeError::DataAccess(format!(
                        "neo4j graph operation `{}` timed out after {}ms",
                        registered.name, effective.timeout_ms
                    )),
                ));
            }
            Ok(Err(error)) => {
                return Err(self.reject_read(
                    audit,
                    Neo4jGraphReadOutcome::Failed,
                    map_execution_error(error),
                ));
            }
            Ok(Ok(outcome)) => outcome,
        };

        let mut rows = outcome.rows;
        let returned = rows.len();
        let truncated = returned > effective.max_results as usize;
        rows.truncate(effective.max_results as usize);
        let result_count = rows.len() as u32;

        let graph_freshness = outcome.graph_freshness;
        let freshness_lag_ms = graph_freshness
            .map(|freshness| (Utc::now() - freshness).num_milliseconds().max(0) as u64);

        audit.outcome = Neo4jGraphReadOutcome::Succeeded;
        audit.result_count = result_count;
        audit.truncated = truncated;
        audit.graph_freshness = graph_freshness;
        audit.freshness_lag_ms = freshness_lag_ms;
        self.audit_sink.record(&audit);

        if duration_ms >= self.slow_query_threshold_ms {
            tracing::warn!(
                target: "appfw::graph_read",
                provider = PROVIDER_KEY,
                data_source = %self.data_source_name,
                operation = %registered.name,
                duration_ms,
                threshold_ms = self.slow_query_threshold_ms,
                "slow neo4j graph read"
            );
        }

        let metadata = json!({
            "provider": PROVIDER_KEY,
            "data_source": self.data_source_name,
            "operation": registered.name,
            "tenant_id": tenant_id,
            "result_count": result_count,
            "truncated": truncated,
            "start_node": audit.start_node,
            "limits": {
                "max_depth": effective.max_depth,
                "max_results": effective.max_results,
                "timeout_ms": effective.timeout_ms,
                "clamped": audit.limits_clamped,
            },
            "duration_ms": duration_ms,
            "graph_freshness": graph_freshness,
            "freshness_lag_ms": freshness_lag_ms,
        });

        Ok(RuntimeGraphQueryResult { rows, metadata })
    }

    async fn execute_named_mutation(
        &self,
        mutation: RuntimeGraphNamedQuery,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<RuntimeGraphMutationResult, RuntimeError> {
        let registered = match self.mutations.get(&mutation.name) {
            Some(registered) => registered.clone(),
            None => {
                let audit = self.mutation_audit_seed(&mutation.name, user, None);
                return Err(self.reject_mutation(
                    audit,
                    Neo4jGraphReadOutcome::Rejected,
                    RuntimeError::Validation(format!("unknown graph mutation `{}`", mutation.name)),
                ));
            }
        };

        let tenant_scoped = registered.is_tenant_scoped();
        let tenant_id = tenant_scoped.then(|| user.tenant_id.clone());
        let mut audit = self.mutation_audit_seed(&registered.name, user, tenant_id.clone());
        audit.declared_depth = registered.limits.max_depth;

        if !access.allow {
            return Err(self.reject_mutation(
                audit,
                Neo4jGraphReadOutcome::DeniedAccess,
                RuntimeError::AccessDenied,
            ));
        }

        if tenant_scoped && user.tenant_id.trim().is_empty() {
            return Err(self.reject_mutation(
                audit,
                Neo4jGraphReadOutcome::Rejected,
                RuntimeError::Validation(format!(
                    "graph mutation `{}` is tenant-scoped but the caller has no tenant",
                    registered.name
                )),
            ));
        }

        if !mutation.query_text.trim().is_empty()
            && normalize_ws(&mutation.query_text) != normalize_ws(&registered.cypher)
        {
            return Err(self.reject_mutation(
                audit,
                Neo4jGraphReadOutcome::Rejected,
                RuntimeError::Validation(format!(
                    "graph mutation `{}` cypher does not match the registered definition",
                    registered.name
                )),
            ));
        }

        if let Err(error) = validate_governed_write_cypher(&registered.cypher) {
            return Err(self.reject_mutation(
                audit,
                Neo4jGraphReadOutcome::Rejected,
                RuntimeError::Validation(error.to_string()),
            ));
        }

        let effective = self.limits.clamp(&registered.limits);
        audit.effective_depth = effective.max_depth;
        audit.effective_max_results = effective.max_results;
        audit.effective_timeout_ms = effective.timeout_ms;
        audit.limits_clamped =
            effective.depth_clamped || effective.results_clamped || effective.timeout_clamped;

        if let Err(error) = validate_traversal_depth(&registered.cypher, effective.max_depth) {
            return Err(self.reject_mutation(
                audit,
                Neo4jGraphReadOutcome::Rejected,
                RuntimeError::Validation(error.to_string()),
            ));
        }

        if let Some(tenant_parameter) = &registered.tenant_parameter {
            if !cypher_references_parameter(&registered.cypher, tenant_parameter) {
                return Err(self.reject_mutation(
                    audit,
                    Neo4jGraphReadOutcome::Rejected,
                    RuntimeError::Validation(format!(
                        "tenant-scoped graph mutation `{}` must reference parameter `${}`",
                        registered.name, tenant_parameter
                    )),
                ));
            }
        }

        let parameters = match merge_bound_parameters(
            &registered.name,
            registered.tenant_parameter.as_deref(),
            &registered.declared_parameter_names(),
            &registered.parameters,
            &mutation.parameters,
            user,
        ) {
            Ok(parameters) => parameters,
            Err(error) => {
                return Err(self.reject_mutation(audit, Neo4jGraphReadOutcome::Rejected, error));
            }
        };

        if let Some(start_parameter) = &registered.start_node_parameter {
            audit.start_node = parameters.get(start_parameter).map(value_to_locator);
        }
        audit.redacted_parameters = parameters
            .keys()
            .filter(|name| registered.tenant_parameter.as_deref() != Some(name.as_str()))
            .cloned()
            .collect();

        let request = Neo4jExecutionRequest {
            database: self.database.clone(),
            operation: registered.name.clone(),
            cypher: registered.cypher.clone(),
            parameters,
            access_mode: Neo4jAccessMode::Write,
            max_results: effective.max_results,
            timeout_ms: effective.timeout_ms,
        };

        let started = Instant::now();
        let outcome = tokio::time::timeout(
            Duration::from_millis(effective.timeout_ms),
            self.executor.run_write_query(request),
        )
        .await;
        let duration_ms = started.elapsed().as_millis() as u64;
        audit.duration_ms = duration_ms;

        let outcome = match outcome {
            Err(_elapsed) => {
                return Err(self.reject_mutation(
                    audit,
                    Neo4jGraphReadOutcome::Failed,
                    RuntimeError::DataAccess(format!(
                        "neo4j graph mutation `{}` timed out after {}ms",
                        registered.name, effective.timeout_ms
                    )),
                ));
            }
            Ok(Err(error)) => {
                return Err(self.reject_mutation(
                    audit,
                    Neo4jGraphReadOutcome::Failed,
                    map_execution_error(error),
                ));
            }
            Ok(Ok(outcome)) => outcome,
        };

        let mut rows = outcome.rows;
        let returned = rows.len();
        let truncated = returned > effective.max_results as usize;
        rows.truncate(effective.max_results as usize);
        let result_count = rows.len() as u32;

        let graph_freshness = outcome.graph_freshness;
        let freshness_lag_ms = graph_freshness
            .map(|freshness| (Utc::now() - freshness).num_milliseconds().max(0) as u64);

        audit.outcome = Neo4jGraphReadOutcome::Succeeded;
        audit.mutation_stats = outcome.stats;
        audit.result_count = result_count;
        audit.truncated = truncated;
        audit.graph_freshness = graph_freshness;
        audit.freshness_lag_ms = freshness_lag_ms;
        self.audit_sink.record_mutation(&audit);

        if duration_ms >= self.slow_query_threshold_ms {
            tracing::warn!(
                target: "appfw::graph_write",
                provider = PROVIDER_KEY,
                data_source = %self.data_source_name,
                operation = %registered.name,
                duration_ms,
                threshold_ms = self.slow_query_threshold_ms,
                "slow neo4j graph write"
            );
        }

        let stats = serde_json::to_value(outcome.stats).unwrap_or_else(|_| json!({}));
        let metadata = json!({
            "provider": PROVIDER_KEY,
            "data_source": self.data_source_name,
            "operation": registered.name,
            "tenant_id": tenant_id,
            "result_count": result_count,
            "truncated": truncated,
            "stats": stats.clone(),
            "start_node": audit.start_node,
            "limits": {
                "max_depth": effective.max_depth,
                "max_results": effective.max_results,
                "timeout_ms": effective.timeout_ms,
                "clamped": audit.limits_clamped,
            },
            "duration_ms": duration_ms,
            "graph_freshness": graph_freshness,
            "freshness_lag_ms": freshness_lag_ms,
        });

        Ok(RuntimeGraphMutationResult {
            rows,
            stats,
            metadata,
        })
    }
}

fn apply_effective_limits(audit: &mut Neo4jGraphReadAudit, effective: &Neo4jEffectiveLimits) {
    audit.effective_depth = effective.max_depth;
    audit.effective_max_results = effective.max_results;
    audit.effective_timeout_ms = effective.timeout_ms;
    audit.limits_clamped =
        effective.depth_clamped || effective.results_clamped || effective.timeout_clamped;
}
