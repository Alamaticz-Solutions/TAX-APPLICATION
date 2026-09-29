use std::sync::Mutex;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::execution::Neo4jWriteStats;

/// Outcome of a graph read attempt, recorded on every audit event.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Neo4jGraphReadOutcome {
    /// The named query executed and returned a result set.
    Succeeded,
    /// Policy denied the operation before execution.
    DeniedAccess,
    /// The operation failed validation/guardrails before execution.
    Rejected,
    /// Execution was attempted but the provider/driver returned an error or
    /// timed out.
    Failed,
}

impl Neo4jGraphReadOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Neo4jGraphReadOutcome::Succeeded => "succeeded",
            Neo4jGraphReadOutcome::DeniedAccess => "denied_access",
            Neo4jGraphReadOutcome::Rejected => "rejected",
            Neo4jGraphReadOutcome::Failed => "failed",
        }
    }
}

/// Structured audit record for a single graph read operation.
///
/// This record is safe to persist and to log: it carries no raw parameter
/// values. The traversal start node is kept as a public locator (it is an
/// ingress-boundary identifier, not classified payload); every other supplied
/// parameter is represented by name only in `redacted_parameters`.
#[derive(Clone, Debug, Serialize)]
pub struct Neo4jGraphReadAudit {
    pub audit_id: String,
    pub occurred_at: DateTime<Utc>,
    pub provider: &'static str,
    pub data_source: String,
    pub operation: String,
    pub outcome: Neo4jGraphReadOutcome,
    pub actor_user_name: String,
    pub actor_roles: Vec<String>,
    pub tenant_id: Option<String>,
    pub start_node: Option<String>,
    pub declared_depth: u16,
    pub effective_depth: u16,
    pub effective_max_results: u32,
    pub effective_timeout_ms: u64,
    pub limits_clamped: bool,
    pub result_count: u32,
    pub truncated: bool,
    pub graph_freshness: Option<DateTime<Utc>>,
    pub freshness_lag_ms: Option<u64>,
    pub duration_ms: u64,
    /// Names (never values) of the parameters supplied to the operation.
    pub redacted_parameters: Vec<String>,
    /// Present when the outcome is `Rejected` or `Failed`.
    pub error: Option<String>,
}

/// Structured audit record for a single governed graph write.
///
/// Like [`Neo4jGraphReadAudit`] it carries no raw parameter values, only the
/// non-classified mutation counters plus actor/tenant/operation context.
#[derive(Clone, Debug, Serialize)]
pub struct Neo4jGraphMutationAudit {
    pub audit_id: String,
    pub occurred_at: DateTime<Utc>,
    pub provider: &'static str,
    pub data_source: String,
    pub operation: String,
    pub outcome: Neo4jGraphReadOutcome,
    pub actor_user_name: String,
    pub actor_roles: Vec<String>,
    pub tenant_id: Option<String>,
    pub start_node: Option<String>,
    pub declared_depth: u16,
    pub effective_depth: u16,
    pub effective_max_results: u32,
    pub effective_timeout_ms: u64,
    pub limits_clamped: bool,
    pub mutation_stats: Neo4jWriteStats,
    pub result_count: u32,
    /// Whether returned rows exceeded the result cap (so `mutation_stats`, which
    /// reflects the executor's reported impact, may exceed `result_count`).
    pub truncated: bool,
    pub graph_freshness: Option<DateTime<Utc>>,
    pub freshness_lag_ms: Option<u64>,
    pub duration_ms: u64,
    /// Names (never values) of the parameters supplied to the operation.
    pub redacted_parameters: Vec<String>,
    /// Present when the outcome is `Rejected` or `Failed`.
    pub error: Option<String>,
}

/// Sink for graph audit + metrics events (reads and governed writes).
///
/// The provider hands a fully-redacted audit record to the sink for every
/// operation attempt (success, denial, rejection, or failure). A product
/// backend implements this to persist the audit trail and to forward timing,
/// query-count, and result-count fields to its metrics registry. The default
/// [`TracingGraphAuditSink`] emits a structured log line. `record_mutation` has
/// a default no-op so existing read-only sinks keep compiling unchanged.
pub trait Neo4jGraphAuditSink: Send + Sync {
    fn record(&self, audit: &Neo4jGraphReadAudit);

    fn record_mutation(&self, _audit: &Neo4jGraphMutationAudit) {}
}

/// Default sink: emit the audit record as a structured `tracing` event. Values
/// are already redacted in the record, so the emitted line is log-safe.
#[derive(Clone, Copy, Debug, Default)]
pub struct TracingGraphAuditSink;

impl Neo4jGraphAuditSink for TracingGraphAuditSink {
    fn record(&self, audit: &Neo4jGraphReadAudit) {
        if matches!(
            audit.outcome,
            Neo4jGraphReadOutcome::DeniedAccess | Neo4jGraphReadOutcome::Failed
        ) {
            tracing::warn!(
                target: "appfw::graph_read",
                provider = audit.provider,
                data_source = %audit.data_source,
                operation = %audit.operation,
                outcome = audit.outcome.as_str(),
                actor = %audit.actor_user_name,
                tenant = audit.tenant_id.as_deref().unwrap_or("-"),
                depth = audit.effective_depth,
                result_count = audit.result_count,
                duration_ms = audit.duration_ms,
                error = audit.error.as_deref().unwrap_or(""),
                "neo4j graph read"
            );
        } else {
            tracing::info!(
                target: "appfw::graph_read",
                provider = audit.provider,
                data_source = %audit.data_source,
                operation = %audit.operation,
                outcome = audit.outcome.as_str(),
                actor = %audit.actor_user_name,
                tenant = audit.tenant_id.as_deref().unwrap_or("-"),
                depth = audit.effective_depth,
                result_count = audit.result_count,
                truncated = audit.truncated,
                duration_ms = audit.duration_ms,
                freshness_lag_ms = audit.freshness_lag_ms.unwrap_or(0),
                "neo4j graph read"
            );
        }
    }

    fn record_mutation(&self, audit: &Neo4jGraphMutationAudit) {
        if matches!(
            audit.outcome,
            Neo4jGraphReadOutcome::DeniedAccess | Neo4jGraphReadOutcome::Failed
        ) {
            tracing::warn!(
                target: "appfw::graph_write",
                provider = audit.provider,
                data_source = %audit.data_source,
                operation = %audit.operation,
                outcome = audit.outcome.as_str(),
                actor = %audit.actor_user_name,
                tenant = audit.tenant_id.as_deref().unwrap_or("-"),
                duration_ms = audit.duration_ms,
                error = audit.error.as_deref().unwrap_or(""),
                "neo4j graph write"
            );
        } else {
            tracing::info!(
                target: "appfw::graph_write",
                provider = audit.provider,
                data_source = %audit.data_source,
                operation = %audit.operation,
                outcome = audit.outcome.as_str(),
                actor = %audit.actor_user_name,
                tenant = audit.tenant_id.as_deref().unwrap_or("-"),
                relationships_created = audit.mutation_stats.relationships_created,
                relationships_deleted = audit.mutation_stats.relationships_deleted,
                affected = audit.mutation_stats.affected(),
                duration_ms = audit.duration_ms,
                "neo4j graph write"
            );
        }
    }
}

/// Sink that drops every event. Use only where audit is handled out of band.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopGraphAuditSink;

impl Neo4jGraphAuditSink for NoopGraphAuditSink {
    fn record(&self, _audit: &Neo4jGraphReadAudit) {}
}

/// In-memory sink that captures every audit record. Intended for tests and
/// local diagnostics, where assertions inspect the recorded events.
#[derive(Default)]
pub struct CapturingGraphAuditSink {
    events: Mutex<Vec<Neo4jGraphReadAudit>>,
    mutation_events: Mutex<Vec<Neo4jGraphMutationAudit>>,
}

impl CapturingGraphAuditSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn events(&self) -> Vec<Neo4jGraphReadAudit> {
        self.events.lock().expect("audit mutex poisoned").clone()
    }

    pub fn last(&self) -> Option<Neo4jGraphReadAudit> {
        self.events
            .lock()
            .expect("audit mutex poisoned")
            .last()
            .cloned()
    }

    pub fn len(&self) -> usize {
        self.events.lock().expect("audit mutex poisoned").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn mutation_events(&self) -> Vec<Neo4jGraphMutationAudit> {
        self.mutation_events
            .lock()
            .expect("audit mutex poisoned")
            .clone()
    }

    pub fn last_mutation(&self) -> Option<Neo4jGraphMutationAudit> {
        self.mutation_events
            .lock()
            .expect("audit mutex poisoned")
            .last()
            .cloned()
    }
}

impl Neo4jGraphAuditSink for CapturingGraphAuditSink {
    fn record(&self, audit: &Neo4jGraphReadAudit) {
        self.events
            .lock()
            .expect("audit mutex poisoned")
            .push(audit.clone());
    }

    fn record_mutation(&self, audit: &Neo4jGraphMutationAudit) {
        self.mutation_events
            .lock()
            .expect("audit mutex poisoned")
            .push(audit.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Neo4jGraphReadAudit {
        Neo4jGraphReadAudit {
            audit_id: "audit-1".to_string(),
            occurred_at: Utc::now(),
            provider: "neo4j",
            data_source: "neo4j_graph".to_string(),
            operation: "account_relationship_graph".to_string(),
            outcome: Neo4jGraphReadOutcome::Succeeded,
            actor_user_name: "rep@example.com".to_string(),
            actor_roles: vec!["sales_rep".to_string()],
            tenant_id: Some("tenant-1".to_string()),
            start_node: Some("rec_123".to_string()),
            declared_depth: 2,
            effective_depth: 2,
            effective_max_results: 25,
            effective_timeout_ms: 1_000,
            limits_clamped: false,
            result_count: 4,
            truncated: false,
            graph_freshness: None,
            freshness_lag_ms: None,
            duration_ms: 7,
            redacted_parameters: vec!["account".to_string()],
            error: None,
        }
    }

    #[test]
    fn capturing_sink_records_events() {
        let sink = CapturingGraphAuditSink::new();
        assert!(sink.is_empty());
        sink.record(&sample());
        assert_eq!(sink.len(), 1);
        assert_eq!(
            sink.last().expect("event").operation,
            "account_relationship_graph"
        );
    }

    #[test]
    fn audit_record_serializes_without_raw_parameter_values() {
        let json = serde_json::to_value(sample()).expect("serialize");
        assert_eq!(json["outcome"], "succeeded");
        assert_eq!(json["redacted_parameters"], serde_json::json!(["account"]));
        assert_eq!(json["start_node"], "rec_123");
        // No raw parameter value map is ever serialized.
        assert!(json.get("parameters").is_none());
    }

    #[test]
    fn outcome_tokens_are_stable() {
        assert_eq!(Neo4jGraphReadOutcome::Succeeded.as_str(), "succeeded");
        assert_eq!(
            Neo4jGraphReadOutcome::DeniedAccess.as_str(),
            "denied_access"
        );
        assert_eq!(Neo4jGraphReadOutcome::Rejected.as_str(), "rejected");
        assert_eq!(Neo4jGraphReadOutcome::Failed.as_str(), "failed");
    }

    fn mutation_sample() -> Neo4jGraphMutationAudit {
        Neo4jGraphMutationAudit {
            audit_id: "audit-2".to_string(),
            occurred_at: Utc::now(),
            provider: "neo4j",
            data_source: "neo4j_graph".to_string(),
            operation: "link_account_referral".to_string(),
            outcome: Neo4jGraphReadOutcome::Succeeded,
            actor_user_name: "admin@example.com".to_string(),
            actor_roles: vec!["admin".to_string()],
            tenant_id: Some("180000".to_string()),
            start_node: Some("rl_acct_acme".to_string()),
            declared_depth: 1,
            effective_depth: 1,
            effective_max_results: 25,
            effective_timeout_ms: 1_000,
            limits_clamped: false,
            mutation_stats: Neo4jWriteStats {
                relationships_created: 1,
                ..Default::default()
            },
            result_count: 1,
            truncated: false,
            graph_freshness: None,
            freshness_lag_ms: None,
            duration_ms: 9,
            redacted_parameters: vec!["from_account".to_string(), "to_account".to_string()],
            error: None,
        }
    }

    #[test]
    fn capturing_sink_records_mutations_separately() {
        let sink = CapturingGraphAuditSink::new();
        sink.record(&sample());
        sink.record_mutation(&mutation_sample());
        assert_eq!(sink.len(), 1);
        assert_eq!(sink.mutation_events().len(), 1);
        assert_eq!(
            sink.last_mutation().expect("mutation").operation,
            "link_account_referral"
        );
    }

    #[test]
    fn mutation_audit_serializes_stats_without_raw_values() {
        let json = serde_json::to_value(mutation_sample()).expect("serialize");
        assert_eq!(json["outcome"], "succeeded");
        assert_eq!(json["mutation_stats"]["relationships_created"], 1);
        assert_eq!(
            json["redacted_parameters"],
            serde_json::json!(["from_account", "to_account"])
        );
        assert!(json.get("parameters").is_none());
    }
}
