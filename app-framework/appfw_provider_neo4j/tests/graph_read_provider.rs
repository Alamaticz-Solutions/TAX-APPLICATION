//! End-to-end provider contracts for the v1 Neo4j graph read provider.
//!
//! These exercise the guardrail path of the graph-read certification checklist
//! (connection/auth, read-only enforcement, named-query compilation, unsafe
//! Cypher rejection, tenant isolation, traversal/result/timeout caps, audit
//! fields, and projection freshness) without a live database, using the
//! injected executor seam.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use appfw_provider_neo4j::{
    CapturingGraphAuditSink, FixtureGraphExecutor, Neo4jAccessMode, Neo4jExecutionError,
    Neo4jExecutionOutcome, Neo4jExecutionRequest, Neo4jExecutor, Neo4jGraphProvider,
    Neo4jGraphQuery, Neo4jGraphQueryLimits, Neo4jGraphReadAudit, Neo4jGraphReadOutcome,
    Neo4jParameter, Neo4jProviderLimits, Neo4jWriteOutcome, Neo4jWriteStats,
};
use appfw_runtime::graph_provider::{
    RuntimeGraphNamedQuery, RuntimeGraphProvider, RuntimeGraphProviderIdentity,
    RuntimeGraphQueryLimits,
};
use appfw_runtime::provider_keys::FrameworkProvider;
use appfw_runtime::{PolicyAccess, RuntimeError, UserAuth};
use async_trait::async_trait;
use chrono::Utc;
use serde_json::{json, Value};

const RELATIONSHIP_CYPHER: &str =
    "MATCH (a:Account {record_locator: $account, tenant_id: $tenant_id})-[:RELATED*1..2]->(b) RETURN b";

fn user(tenant: &str) -> UserAuth {
    UserAuth::human(
        tenant,
        "rep@example.com",
        "UTC",
        vec!["sales_rep".to_string()],
        vec![],
        "",
    )
}

fn relationship_query() -> Neo4jGraphQuery {
    Neo4jGraphQuery::new(
        "account_relationship_graph",
        RELATIONSHIP_CYPHER,
        vec![Neo4jParameter::new("account", json!("rec_seed"))],
        Neo4jGraphQueryLimits {
            max_depth: 2,
            max_results: 25,
            timeout_ms: 1_000,
        },
    )
    .expect("read-only relationship query")
    .with_start_node_parameter("account")
}

fn named_call(name: &str, parameters: BTreeMap<String, Value>) -> RuntimeGraphNamedQuery {
    RuntimeGraphNamedQuery {
        name: name.to_string(),
        query_text: String::new(),
        parameters,
        limits: RuntimeGraphQueryLimits::default(),
    }
}

fn one_param(name: &str, value: Value) -> BTreeMap<String, Value> {
    BTreeMap::from([(name.to_string(), value)])
}

/// Executor that records the last request it was handed, so tests can assert
/// the exact bound parameters / access mode reaching the driver.
#[derive(Clone, Default)]
struct RecordingExecutor {
    rows: Vec<Value>,
    last_request: Arc<Mutex<Option<Neo4jExecutionRequest>>>,
}

impl RecordingExecutor {
    fn with_rows(rows: Vec<Value>) -> Self {
        Self {
            rows,
            last_request: Arc::new(Mutex::new(None)),
        }
    }

    fn last_request(&self) -> Neo4jExecutionRequest {
        self.last_request
            .lock()
            .unwrap()
            .clone()
            .expect("executor should have been called")
    }
}

#[async_trait]
impl Neo4jExecutor for RecordingExecutor {
    async fn verify_connectivity(&self) -> Result<(), Neo4jExecutionError> {
        Ok(())
    }

    async fn run_read_query(
        &self,
        request: Neo4jExecutionRequest,
    ) -> Result<Neo4jExecutionOutcome, Neo4jExecutionError> {
        *self.last_request.lock().unwrap() = Some(request.clone());
        Ok(Neo4jExecutionOutcome::new(self.rows.clone()))
    }

    async fn run_write_query(
        &self,
        request: Neo4jExecutionRequest,
    ) -> Result<Neo4jWriteOutcome, Neo4jExecutionError> {
        // Read tests never invoke writes; record and return an empty outcome.
        *self.last_request.lock().unwrap() = Some(request.clone());
        Ok(Neo4jWriteOutcome::new(
            self.rows.clone(),
            Neo4jWriteStats::default(),
        ))
    }
}

#[tokio::test]
async fn provider_identity_reports_neo4j_graph_read() {
    let provider = Neo4jGraphProvider::new("neo4j_graph", "neo4j", FixtureGraphExecutor::new());
    let descriptor = provider.graph_provider_descriptor();
    assert_eq!(descriptor.provider, FrameworkProvider::Neo4j);
    assert_eq!(descriptor.provider_key(), "neo4j");
    assert_eq!(descriptor.data_source_name, "neo4j_graph");
}

#[tokio::test]
async fn health_check_follows_executor_connectivity() {
    let healthy = Neo4jGraphProvider::new("neo4j_graph", "neo4j", FixtureGraphExecutor::new());
    assert!(healthy.health_check().await.is_ok());

    let unhealthy = Neo4jGraphProvider::new(
        "neo4j_graph",
        "neo4j",
        FixtureGraphExecutor::new().failing_connectivity("bolt refused"),
    );
    assert!(matches!(
        unhealthy.health_check().await,
        Err(RuntimeError::DataAccess(_))
    ));
}

#[tokio::test]
async fn named_query_executes_and_emits_audit_and_metadata() {
    let audit = Arc::new(CapturingGraphAuditSink::new());
    let freshness = Utc::now() - chrono::Duration::seconds(30);
    let provider = Neo4jGraphProvider::new(
        "neo4j_graph",
        "neo4j",
        FixtureGraphExecutor::new()
            .with_rows("account_relationship_graph", vec![json!({"id": "b1"})])
            .with_freshness(freshness),
    )
    .with_audit_sink(audit.clone())
    .register_query(relationship_query());

    let result = provider
        .execute_named_query(
            named_call(
                "account_relationship_graph",
                one_param("account", json!("rec_123")),
            ),
            &user("tenant-1"),
            &PolicyAccess::allow_all(),
        )
        .await
        .expect("query succeeds");

    assert_eq!(result.rows, vec![json!({"id": "b1"})]);
    assert_eq!(result.metadata["operation"], "account_relationship_graph");
    assert_eq!(result.metadata["provider"], "neo4j");
    assert_eq!(result.metadata["tenant_id"], "tenant-1");
    assert_eq!(result.metadata["result_count"], 1);
    assert_eq!(result.metadata["truncated"], false);
    assert_eq!(result.metadata["start_node"], "rec_123");
    assert!(result.metadata["freshness_lag_ms"].as_u64().unwrap() >= 30_000);

    let event = audit.last().expect("audit recorded");
    assert_eq!(event.outcome, Neo4jGraphReadOutcome::Succeeded);
    assert_eq!(event.operation, "account_relationship_graph");
    assert_eq!(event.tenant_id.as_deref(), Some("tenant-1"));
    assert_eq!(event.start_node.as_deref(), Some("rec_123"));
    assert_eq!(event.result_count, 1);
    assert!(event.graph_freshness.is_some());
    // The audit lists supplied parameter names but never the server-bound tenant.
    assert!(event.redacted_parameters.contains(&"account".to_string()));
    assert!(!event.redacted_parameters.contains(&"tenant_id".to_string()));
}

#[tokio::test]
async fn denied_policy_blocks_execution_and_audits_denial() {
    let audit = Arc::new(CapturingGraphAuditSink::new());
    let executor = RecordingExecutor::with_rows(vec![json!({"id": "b1"})]);
    let provider = Neo4jGraphProvider::new("neo4j_graph", "neo4j", executor.clone())
        .with_audit_sink(audit.clone())
        .register_query(relationship_query());

    let error = provider
        .execute_named_query(
            named_call(
                "account_relationship_graph",
                one_param("account", json!("rec_123")),
            ),
            &user("tenant-1"),
            &PolicyAccess::deny(),
        )
        .await
        .expect_err("denied policy");

    assert!(matches!(error, RuntimeError::AccessDenied));
    assert_eq!(
        audit.last().expect("audit").outcome,
        Neo4jGraphReadOutcome::DeniedAccess
    );
    // The executor must never have been reached.
    assert!(executor.last_request.lock().unwrap().is_none());
}

#[tokio::test]
async fn unknown_operation_is_rejected() {
    let audit = Arc::new(CapturingGraphAuditSink::new());
    let provider = Neo4jGraphProvider::new("neo4j_graph", "neo4j", FixtureGraphExecutor::new())
        .with_audit_sink(audit.clone())
        .register_query(relationship_query());

    let error = provider
        .execute_named_query(
            named_call("not_registered", BTreeMap::new()),
            &user("tenant-1"),
            &PolicyAccess::allow_all(),
        )
        .await
        .expect_err("unknown operation");

    assert!(
        matches!(error, RuntimeError::Validation(message) if message.contains("unknown graph operation"))
    );
    assert_eq!(
        audit.last().expect("audit").outcome,
        Neo4jGraphReadOutcome::Rejected
    );
}

#[tokio::test]
async fn tenant_is_server_bound_and_isolated_per_caller() {
    let executor = RecordingExecutor::with_rows(vec![json!({"id": "b1"})]);
    let provider = Neo4jGraphProvider::new("neo4j_graph", "neo4j", executor.clone())
        .register_query(relationship_query());

    provider
        .execute_named_query(
            named_call(
                "account_relationship_graph",
                one_param("account", json!("rec_123")),
            ),
            &user("tenant-1"),
            &PolicyAccess::allow_all(),
        )
        .await
        .expect("tenant-1 query");
    let request = executor.last_request();
    assert_eq!(request.access_mode, Neo4jAccessMode::Read);
    assert_eq!(request.parameters["tenant_id"], json!("tenant-1"));
    assert_eq!(request.parameters["account"], json!("rec_123"));

    provider
        .execute_named_query(
            named_call(
                "account_relationship_graph",
                one_param("account", json!("rec_999")),
            ),
            &user("tenant-2"),
            &PolicyAccess::allow_all(),
        )
        .await
        .expect("tenant-2 query");
    assert_eq!(
        executor.last_request().parameters["tenant_id"],
        json!("tenant-2")
    );
}

#[tokio::test]
async fn caller_cannot_override_tenant_parameter() {
    let provider = Neo4jGraphProvider::new("neo4j_graph", "neo4j", FixtureGraphExecutor::new())
        .register_query(relationship_query());

    let mut parameters = one_param("account", json!("rec_123"));
    parameters.insert("tenant_id".to_string(), json!("tenant-evil"));

    let error = provider
        .execute_named_query(
            named_call("account_relationship_graph", parameters),
            &user("tenant-1"),
            &PolicyAccess::allow_all(),
        )
        .await
        .expect_err("tenant spoof rejected");
    assert!(
        matches!(error, RuntimeError::Validation(message) if message.contains("server-controlled"))
    );
}

#[tokio::test]
async fn undeclared_parameter_is_rejected() {
    let provider = Neo4jGraphProvider::new("neo4j_graph", "neo4j", FixtureGraphExecutor::new())
        .register_query(relationship_query());

    let error = provider
        .execute_named_query(
            named_call(
                "account_relationship_graph",
                one_param("inject", json!("anything")),
            ),
            &user("tenant-1"),
            &PolicyAccess::allow_all(),
        )
        .await
        .expect_err("undeclared parameter");
    assert!(
        matches!(error, RuntimeError::Validation(message) if message.contains("does not declare parameter"))
    );
}

#[tokio::test]
async fn tenant_scoped_query_must_reference_tenant_parameter() {
    // A tenant-scoped operation whose cypher forgets the tenant filter is a
    // misconfiguration and must be rejected before execution.
    let leaky = Neo4jGraphQuery::new(
        "leaky",
        "MATCH (a:Account {record_locator: $account}) RETURN a",
        vec![Neo4jParameter::new("account", json!("rec_seed"))],
        Neo4jGraphQueryLimits {
            max_depth: 1,
            max_results: 25,
            timeout_ms: 1_000,
        },
    )
    .expect("constructs");
    let provider = Neo4jGraphProvider::new("neo4j_graph", "neo4j", FixtureGraphExecutor::new())
        .register_query(leaky);

    let error = provider
        .execute_named_query(
            named_call("leaky", one_param("account", json!("rec_123"))),
            &user("tenant-1"),
            &PolicyAccess::allow_all(),
        )
        .await
        .expect_err("missing tenant filter");
    assert!(
        matches!(error, RuntimeError::Validation(message) if message.contains("must reference parameter"))
    );
}

#[tokio::test]
async fn mismatched_caller_cypher_is_rejected() {
    let provider = Neo4jGraphProvider::new("neo4j_graph", "neo4j", FixtureGraphExecutor::new())
        .register_query(relationship_query());

    let mut call = named_call(
        "account_relationship_graph",
        one_param("account", json!("rec_123")),
    );
    call.query_text = "MATCH (n) DETACH DELETE n".to_string();

    let error = provider
        .execute_named_query(call, &user("tenant-1"), &PolicyAccess::allow_all())
        .await
        .expect_err("cypher tampering rejected");
    assert!(
        matches!(error, RuntimeError::Validation(message) if message.contains("does not match the registered definition"))
    );
}

#[tokio::test]
async fn traversal_depth_above_ceiling_is_rejected() {
    // Provider ceiling caps depth at 2; the query traverses *1..4.
    let deep = Neo4jGraphQuery::new(
        "deep",
        "MATCH (a {tenant_id: $tenant_id})-[:R*1..4]->(b) RETURN b",
        vec![],
        Neo4jGraphQueryLimits {
            max_depth: 4,
            max_results: 25,
            timeout_ms: 1_000,
        },
    )
    .expect("bounded traversal constructs");
    let provider = Neo4jGraphProvider::new("neo4j_graph", "neo4j", FixtureGraphExecutor::new())
        .with_limits(Neo4jProviderLimits {
            max_depth: 2,
            max_results: 1_000,
            max_timeout_ms: 5_000,
        })
        .register_query(deep);

    let error = provider
        .execute_named_query(
            named_call("deep", BTreeMap::new()),
            &user("tenant-1"),
            &PolicyAccess::allow_all(),
        )
        .await
        .expect_err("depth exceeded");
    assert!(
        matches!(error, RuntimeError::Validation(message) if message.contains("traversal depth"))
    );
}

#[tokio::test]
async fn result_cap_truncates_and_flags_audit() {
    let audit = Arc::new(CapturingGraphAuditSink::new());
    let rows: Vec<Value> = (0..5).map(|i| json!({ "id": i })).collect();
    let provider = Neo4jGraphProvider::new(
        "neo4j_graph",
        "neo4j",
        FixtureGraphExecutor::new().with_rows("account_relationship_graph", rows),
    )
    .with_limits(Neo4jProviderLimits {
        max_depth: 5,
        max_results: 2,
        max_timeout_ms: 5_000,
    })
    .with_audit_sink(audit.clone())
    .register_query(relationship_query());

    let result = provider
        .execute_named_query(
            named_call(
                "account_relationship_graph",
                one_param("account", json!("rec_123")),
            ),
            &user("tenant-1"),
            &PolicyAccess::allow_all(),
        )
        .await
        .expect("query succeeds");

    assert_eq!(result.rows.len(), 2);
    assert_eq!(result.metadata["truncated"], true);
    assert_eq!(result.metadata["result_count"], 2);
    assert_eq!(result.metadata["limits"]["clamped"], true);

    let event: Neo4jGraphReadAudit = audit.last().expect("audit");
    assert!(event.truncated);
    assert_eq!(event.result_count, 2);
    assert!(event.limits_clamped);
}

#[tokio::test(start_paused = true)]
async fn slow_executor_hits_timeout_cap() {
    let audit = Arc::new(CapturingGraphAuditSink::new());
    // Query requests a 50ms timeout; the executor sleeps 5s.
    let slow = Neo4jGraphQuery::new(
        "account_relationship_graph",
        RELATIONSHIP_CYPHER,
        vec![Neo4jParameter::new("account", json!("rec_seed"))],
        Neo4jGraphQueryLimits {
            max_depth: 2,
            max_results: 25,
            timeout_ms: 50,
        },
    )
    .expect("query")
    .with_start_node_parameter("account");

    let provider = Neo4jGraphProvider::new(
        "neo4j_graph",
        "neo4j",
        FixtureGraphExecutor::new()
            .with_rows("account_relationship_graph", vec![json!({"id": "b1"})])
            .with_delay(Duration::from_secs(5)),
    )
    .with_audit_sink(audit.clone())
    .register_query(slow);

    let error = provider
        .execute_named_query(
            named_call(
                "account_relationship_graph",
                one_param("account", json!("rec_123")),
            ),
            &user("tenant-1"),
            &PolicyAccess::allow_all(),
        )
        .await
        .expect_err("timeout");

    assert!(matches!(error, RuntimeError::DataAccess(message) if message.contains("timed out")));
    assert_eq!(
        audit.last().expect("audit").outcome,
        Neo4jGraphReadOutcome::Failed
    );
}

#[tokio::test]
async fn executor_query_error_maps_to_data_access() {
    let audit = Arc::new(CapturingGraphAuditSink::new());
    let provider = Neo4jGraphProvider::new(
        "neo4j_graph",
        "neo4j",
        FixtureGraphExecutor::new().failing_query("syntax error"),
    )
    .with_audit_sink(audit.clone())
    .register_query(relationship_query());

    let error = provider
        .execute_named_query(
            named_call(
                "account_relationship_graph",
                one_param("account", json!("rec_123")),
            ),
            &user("tenant-1"),
            &PolicyAccess::allow_all(),
        )
        .await
        .expect_err("query error");

    assert!(
        matches!(error, RuntimeError::DataAccess(message) if message.contains("neo4j query error"))
    );
    assert_eq!(
        audit.last().expect("audit").outcome,
        Neo4jGraphReadOutcome::Failed
    );
}
