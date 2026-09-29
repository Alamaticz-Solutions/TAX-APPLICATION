//! End-to-end contracts for the governed Neo4j graph WRITE provider.
//!
//! Exercises the write guardrail path (policy access, server-bound tenant,
//! governed-write validation, depth/result/timeout caps, redacted mutation
//! audit) without a live database, using the injected write-executor seam.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use appfw_provider_neo4j::{
    CapturingGraphAuditSink, FixtureGraphExecutor, Neo4jAccessMode, Neo4jExecutionError,
    Neo4jExecutionOutcome, Neo4jExecutionRequest, Neo4jExecutor, Neo4jGraphMutation,
    Neo4jGraphProvider, Neo4jGraphQueryLimits, Neo4jGraphReadOutcome, Neo4jParameter,
    Neo4jProviderLimits, Neo4jWriteOutcome, Neo4jWriteStats,
};
use appfw_runtime::graph_provider::{
    RuntimeGraphNamedQuery, RuntimeGraphProvider, RuntimeGraphProviderIdentity,
    RuntimeGraphQueryLimits,
};
use appfw_runtime::provider_keys::FrameworkProvider;
use appfw_runtime::{PolicyAccess, RuntimeError, UserAuth};
use async_trait::async_trait;
use serde_json::{json, Value};

const LINK_CYPHER: &str =
    "MATCH (a:Account {tenant_id: $tenant_id, record_locator: $from_account}), \
     (b:Account {tenant_id: $tenant_id, record_locator: $to_account}) \
     MERGE (a)-[r:REFERRED {tenant_id: $tenant_id}]->(b) RETURN r";

fn user(tenant: &str) -> UserAuth {
    UserAuth::human(
        tenant,
        "admin@example.com",
        "UTC",
        vec!["admin".to_string()],
        vec![],
        "",
    )
}

fn link_mutation() -> Neo4jGraphMutation {
    Neo4jGraphMutation::new(
        "link_account_referral",
        LINK_CYPHER,
        vec![
            Neo4jParameter::new("from_account", json!("rl_seed_a")),
            Neo4jParameter::new("to_account", json!("rl_seed_b")),
        ],
        Neo4jGraphQueryLimits {
            max_depth: 1,
            max_results: 25,
            timeout_ms: 1_000,
        },
    )
    .expect("governed link mutation")
    .with_start_node_parameter("from_account")
}

fn named_mutation(name: &str, parameters: BTreeMap<String, Value>) -> RuntimeGraphNamedQuery {
    RuntimeGraphNamedQuery {
        name: name.to_string(),
        query_text: String::new(),
        parameters,
        limits: RuntimeGraphQueryLimits::default(),
    }
}

fn params(pairs: &[(&str, Value)]) -> BTreeMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

/// Write executor that records the last request handed to the driver.
#[derive(Clone, Default)]
struct RecordingWriteExecutor {
    rows: Vec<Value>,
    stats: Neo4jWriteStats,
    last_request: Arc<Mutex<Option<Neo4jExecutionRequest>>>,
}

impl RecordingWriteExecutor {
    fn linked() -> Self {
        Self {
            rows: vec![json!({"linked": true})],
            stats: Neo4jWriteStats {
                relationships_created: 1,
                ..Default::default()
            },
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
impl Neo4jExecutor for RecordingWriteExecutor {
    async fn verify_connectivity(&self) -> Result<(), Neo4jExecutionError> {
        Ok(())
    }

    async fn run_read_query(
        &self,
        request: Neo4jExecutionRequest,
    ) -> Result<Neo4jExecutionOutcome, Neo4jExecutionError> {
        // Write tests never invoke reads; record and return the canned rows.
        *self.last_request.lock().unwrap() = Some(request.clone());
        Ok(Neo4jExecutionOutcome::new(self.rows.clone()))
    }

    async fn run_write_query(
        &self,
        request: Neo4jExecutionRequest,
    ) -> Result<Neo4jWriteOutcome, Neo4jExecutionError> {
        *self.last_request.lock().unwrap() = Some(request.clone());
        Ok(Neo4jWriteOutcome::new(self.rows.clone(), self.stats))
    }
}

#[tokio::test]
async fn write_provider_identity_reports_neo4j() {
    let provider = Neo4jGraphProvider::new("neo4j_graph", "neo4j", FixtureGraphExecutor::new());
    let descriptor = provider.graph_provider_descriptor();
    assert_eq!(descriptor.provider, FrameworkProvider::Neo4j);
    assert_eq!(descriptor.provider_key(), "neo4j");
}

#[tokio::test]
async fn write_health_check_follows_executor_connectivity() {
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
async fn governed_mutation_executes_and_audits() {
    let audit = Arc::new(CapturingGraphAuditSink::new());
    let provider = Neo4jGraphProvider::new(
        "neo4j_graph",
        "neo4j",
        FixtureGraphExecutor::new().with_write_result(
            "link_account_referral",
            vec![json!({"linked": true})],
            Neo4jWriteStats {
                relationships_created: 1,
                ..Default::default()
            },
        ),
    )
    .with_audit_sink(audit.clone())
    .register_mutation(link_mutation());

    let result = provider
        .execute_named_mutation(
            named_mutation(
                "link_account_referral",
                params(&[
                    ("from_account", json!("rl_acct_acme")),
                    ("to_account", json!("rl_acct_medi")),
                ]),
            ),
            &user("180000"),
            &PolicyAccess::allow_all(),
        )
        .await
        .expect("mutation succeeds");

    assert_eq!(result.rows, vec![json!({"linked": true})]);
    assert_eq!(result.stats["relationships_created"], 1);
    assert_eq!(result.metadata["operation"], "link_account_referral");
    assert_eq!(result.metadata["tenant_id"], "180000");

    let event = audit.last_mutation().expect("mutation audit recorded");
    assert_eq!(event.outcome, Neo4jGraphReadOutcome::Succeeded);
    assert_eq!(event.mutation_stats.relationships_created, 1);
    assert_eq!(event.tenant_id.as_deref(), Some("180000"));
    assert_eq!(event.start_node.as_deref(), Some("rl_acct_acme"));
    assert!(event
        .redacted_parameters
        .contains(&"from_account".to_string()));
    assert!(!event.redacted_parameters.contains(&"tenant_id".to_string()));
}

#[tokio::test]
async fn denied_policy_blocks_write_and_audits_denial() {
    let audit = Arc::new(CapturingGraphAuditSink::new());
    let executor = RecordingWriteExecutor::linked();
    let provider = Neo4jGraphProvider::new("neo4j_graph", "neo4j", executor.clone())
        .with_audit_sink(audit.clone())
        .register_mutation(link_mutation());

    let error = provider
        .execute_named_mutation(
            named_mutation(
                "link_account_referral",
                params(&[
                    ("from_account", json!("rl_a")),
                    ("to_account", json!("rl_b")),
                ]),
            ),
            &user("180000"),
            &PolicyAccess::deny(),
        )
        .await
        .expect_err("denied policy");

    assert!(matches!(error, RuntimeError::AccessDenied));
    assert_eq!(
        audit.last_mutation().expect("audit").outcome,
        Neo4jGraphReadOutcome::DeniedAccess
    );
    assert!(executor.last_request.lock().unwrap().is_none());
}

#[tokio::test]
async fn unknown_mutation_is_rejected() {
    let provider = Neo4jGraphProvider::new("neo4j_graph", "neo4j", FixtureGraphExecutor::new())
        .register_mutation(link_mutation());

    let error = provider
        .execute_named_mutation(
            named_mutation("not_registered", BTreeMap::new()),
            &user("180000"),
            &PolicyAccess::allow_all(),
        )
        .await
        .expect_err("unknown mutation");
    assert!(matches!(error, RuntimeError::Validation(m) if m.contains("unknown graph mutation")));
}

#[tokio::test]
async fn tenant_is_server_bound_for_writes() {
    let executor = RecordingWriteExecutor::linked();
    let provider = Neo4jGraphProvider::new("neo4j_graph", "neo4j", executor.clone())
        .register_mutation(link_mutation());

    provider
        .execute_named_mutation(
            named_mutation(
                "link_account_referral",
                params(&[
                    ("from_account", json!("rl_a")),
                    ("to_account", json!("rl_b")),
                ]),
            ),
            &user("tenant-7"),
            &PolicyAccess::allow_all(),
        )
        .await
        .expect("write");

    let request = executor.last_request();
    assert_eq!(request.access_mode, Neo4jAccessMode::Write);
    assert_eq!(request.parameters["tenant_id"], json!("tenant-7"));
    assert_eq!(request.parameters["from_account"], json!("rl_a"));
}

#[tokio::test]
async fn caller_cannot_override_tenant_for_writes() {
    let provider = Neo4jGraphProvider::new("neo4j_graph", "neo4j", FixtureGraphExecutor::new())
        .register_mutation(link_mutation());

    let error = provider
        .execute_named_mutation(
            named_mutation(
                "link_account_referral",
                params(&[
                    ("from_account", json!("rl_a")),
                    ("to_account", json!("rl_b")),
                    ("tenant_id", json!("tenant-evil")),
                ]),
            ),
            &user("180000"),
            &PolicyAccess::allow_all(),
        )
        .await
        .expect_err("tenant spoof rejected");
    assert!(matches!(error, RuntimeError::Validation(m) if m.contains("server-controlled")));
}

#[tokio::test]
async fn undeclared_write_parameter_is_rejected() {
    let provider = Neo4jGraphProvider::new("neo4j_graph", "neo4j", FixtureGraphExecutor::new())
        .register_mutation(link_mutation());

    let error = provider
        .execute_named_mutation(
            named_mutation(
                "link_account_referral",
                params(&[
                    ("from_account", json!("rl_a")),
                    ("to_account", json!("rl_b")),
                    ("inject", json!("x")),
                ]),
            ),
            &user("180000"),
            &PolicyAccess::allow_all(),
        )
        .await
        .expect_err("undeclared parameter");
    assert!(
        matches!(error, RuntimeError::Validation(m) if m.contains("does not declare parameter"))
    );
}

#[tokio::test]
async fn tenant_scoped_mutation_must_reference_tenant_parameter() {
    // Governed write that forgets the tenant filter is a misconfiguration.
    let leaky = Neo4jGraphMutation::new(
        "leaky_link",
        "MATCH (a:Account {record_locator: $from_account})-[r:REFERRED]->(b:Account {record_locator: $to_account}) DELETE r",
        vec![
            Neo4jParameter::new("from_account", json!("rl_a")),
            Neo4jParameter::new("to_account", json!("rl_b")),
        ],
        Neo4jGraphQueryLimits {
            max_depth: 1,
            max_results: 25,
            timeout_ms: 1_000,
        },
    )
    .expect("constructs");
    let provider = Neo4jGraphProvider::new("neo4j_graph", "neo4j", FixtureGraphExecutor::new())
        .register_mutation(leaky);

    let error = provider
        .execute_named_mutation(
            named_mutation(
                "leaky_link",
                params(&[
                    ("from_account", json!("rl_a")),
                    ("to_account", json!("rl_b")),
                ]),
            ),
            &user("180000"),
            &PolicyAccess::allow_all(),
        )
        .await
        .expect_err("missing tenant filter");
    assert!(matches!(error, RuntimeError::Validation(m) if m.contains("must reference parameter")));
}

#[tokio::test]
async fn mismatched_caller_cypher_is_rejected_for_writes() {
    let provider = Neo4jGraphProvider::new("neo4j_graph", "neo4j", FixtureGraphExecutor::new())
        .register_mutation(link_mutation());

    let mut call = named_mutation(
        "link_account_referral",
        params(&[
            ("from_account", json!("rl_a")),
            ("to_account", json!("rl_b")),
        ]),
    );
    call.query_text = "MATCH (n) DETACH DELETE n".to_string();

    let error = provider
        .execute_named_mutation(call, &user("180000"), &PolicyAccess::allow_all())
        .await
        .expect_err("cypher tampering rejected");
    assert!(
        matches!(error, RuntimeError::Validation(m) if m.contains("does not match the registered definition"))
    );
}

#[tokio::test]
async fn write_traversal_depth_above_ceiling_is_rejected() {
    let deep = Neo4jGraphMutation::new(
        "deep_link",
        "MATCH (a:Account {tenant_id: $tenant_id})-[:KNOWS*1..4]->(b:Account {tenant_id: $tenant_id}) \
         MERGE (a)-[r:REFERRED {tenant_id: $tenant_id}]->(b) RETURN r",
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
        .register_mutation(deep);

    let error = provider
        .execute_named_mutation(
            named_mutation("deep_link", BTreeMap::new()),
            &user("180000"),
            &PolicyAccess::allow_all(),
        )
        .await
        .expect_err("depth exceeded");
    assert!(matches!(error, RuntimeError::Validation(m) if m.contains("traversal depth")));
}

#[tokio::test]
async fn write_result_cap_truncates_and_flags_audit() {
    let audit = Arc::new(CapturingGraphAuditSink::new());
    let rows: Vec<Value> = (0..5).map(|i| json!({ "i": i })).collect();
    let provider = Neo4jGraphProvider::new(
        "neo4j_graph",
        "neo4j",
        FixtureGraphExecutor::new().with_write_result(
            "link_account_referral",
            rows,
            Neo4jWriteStats {
                relationships_created: 5,
                ..Default::default()
            },
        ),
    )
    .with_limits(Neo4jProviderLimits {
        max_depth: 5,
        max_results: 2,
        max_timeout_ms: 5_000,
    })
    .with_audit_sink(audit.clone())
    .register_mutation(link_mutation());

    let result = provider
        .execute_named_mutation(
            named_mutation(
                "link_account_referral",
                params(&[
                    ("from_account", json!("rl_a")),
                    ("to_account", json!("rl_b")),
                ]),
            ),
            &user("180000"),
            &PolicyAccess::allow_all(),
        )
        .await
        .expect("mutation succeeds");

    assert_eq!(result.rows.len(), 2);
    assert_eq!(result.metadata["truncated"], true);
    assert_eq!(result.metadata["result_count"], 2);

    let event = audit.last_mutation().expect("audit");
    assert!(event.truncated);
    assert_eq!(event.result_count, 2);
}

#[tokio::test(start_paused = true)]
async fn slow_write_hits_timeout_cap() {
    let audit = Arc::new(CapturingGraphAuditSink::new());
    let slow = Neo4jGraphMutation::new(
        "link_account_referral",
        LINK_CYPHER,
        vec![
            Neo4jParameter::new("from_account", json!("rl_a")),
            Neo4jParameter::new("to_account", json!("rl_b")),
        ],
        Neo4jGraphQueryLimits {
            max_depth: 1,
            max_results: 25,
            timeout_ms: 50,
        },
    )
    .expect("mutation");

    let provider = Neo4jGraphProvider::new(
        "neo4j_graph",
        "neo4j",
        FixtureGraphExecutor::new()
            .with_write_result("link_account_referral", vec![], Neo4jWriteStats::default())
            .with_delay(Duration::from_secs(5)),
    )
    .with_audit_sink(audit.clone())
    .register_mutation(slow);

    let error = provider
        .execute_named_mutation(
            named_mutation(
                "link_account_referral",
                params(&[
                    ("from_account", json!("rl_a")),
                    ("to_account", json!("rl_b")),
                ]),
            ),
            &user("180000"),
            &PolicyAccess::allow_all(),
        )
        .await
        .expect_err("timeout");

    assert!(matches!(error, RuntimeError::DataAccess(m) if m.contains("timed out")));
    assert_eq!(
        audit.last_mutation().expect("audit").outcome,
        Neo4jGraphReadOutcome::Failed
    );
}

#[tokio::test]
async fn write_executor_query_error_maps_to_data_access() {
    let provider = Neo4jGraphProvider::new(
        "neo4j_graph",
        "neo4j",
        FixtureGraphExecutor::new().failing_query("constraint violation"),
    )
    .register_mutation(link_mutation());

    let error = provider
        .execute_named_mutation(
            named_mutation(
                "link_account_referral",
                params(&[
                    ("from_account", json!("rl_a")),
                    ("to_account", json!("rl_b")),
                ]),
            ),
            &user("180000"),
            &PolicyAccess::allow_all(),
        )
        .await
        .expect_err("query error");
    assert!(matches!(error, RuntimeError::DataAccess(m) if m.contains("neo4j query error")));
}
