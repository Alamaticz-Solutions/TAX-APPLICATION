#![cfg(feature = "chat")]

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use appfw_runtime::{
    ingress::{assemble_runtime_router, ix_transport_routes, RuntimeRouteSet},
    ix::{
        IxArtifactBinding, IxArtifactStatus, IxAuditProjection, IxAuditProjectionState,
        IxCancellationSignal, IxClientBinding, IxClientProfile, IxConsentState, IxContextClaims,
        IxContextFreshness, IxContextPolicy, IxContextProposal, IxContractError,
        IxDataClassification, IxEgressPosture, IxJwtVerifier, IxOrchestrationDraft,
        IxPolicyVerdict, IxPreparedRun, IxPrincipalBinding, IxProductContext,
        IxProductContextResolver, IxProductOrchestrator, IxRepositoryCommitReceipt,
        IxRunCommitTransaction, IxRunCreateTransaction, IxRunRepository, IxRunRequest, IxRunState,
        IxRuntimeIntentPolicy, IxRuntimeService, IxStoredRun, IxStoredRunSealKey,
        IxTerminalOutcome, IxTransportPolicy, IX_CANCEL_REQUEST_SCHEMA_VERSION,
        IX_RUN_REQUEST_SCHEMA_VERSION,
    },
    observability::MetricsRegistry,
    security::SecurityConfig,
};
use axum::{
    body::{Body, HttpBody as _},
    http::{header, Method, Request, StatusCode},
    routing::get,
    Router,
};
use futures_util::{stream, stream::BoxStream, StreamExt};
use serde_json::{json, Value};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::Notify,
};
use tower::ServiceExt;
use tower_http::cors::CorsLayer;

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/test_fixtures/ix_rs256.rs"
));

#[derive(Default)]
struct DurableMemoryRepository {
    records: Mutex<HashMap<String, Vec<u8>>>,
    projections: Mutex<Vec<IxAuditProjection>>,
    create_calls: AtomicUsize,
    append_calls: AtomicUsize,
    fail_create: AtomicBool,
    fail_projection: AtomicBool,
    block_terminal_append: AtomicBool,
    terminal_append_entered: Notify,
    terminal_append_release: Notify,
}

impl DurableMemoryRepository {
    fn set_fail_create(&self, value: bool) {
        self.fail_create.store(value, Ordering::SeqCst);
    }

    fn set_fail_projection(&self, value: bool) {
        self.fail_projection.store(value, Ordering::SeqCst);
    }

    fn create_calls(&self) -> usize {
        self.create_calls.load(Ordering::SeqCst)
    }

    fn append_calls(&self) -> usize {
        self.append_calls.load(Ordering::SeqCst)
    }

    fn projection_attempts(&self) -> Vec<IxAuditProjection> {
        self.projections
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    fn set_block_terminal_append(&self, value: bool) {
        self.block_terminal_append.store(value, Ordering::SeqCst);
    }

    async fn wait_for_terminal_append(&self) {
        self.terminal_append_entered.notified().await;
    }

    fn release_terminal_append(&self) {
        self.terminal_append_release.notify_waiters();
    }

    fn stored(&self, run_id: &str) -> Option<IxStoredRun> {
        self.records
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(run_id)
            .map(|bytes| IxStoredRun::from_json_slice(bytes).expect("valid durable record"))
    }
}

#[async_trait::async_trait]
impl IxRunRepository for DurableMemoryRepository {
    async fn create_run(
        &self,
        transaction: IxRunCreateTransaction,
    ) -> Result<IxRepositoryCommitReceipt, IxContractError> {
        self.create_calls.fetch_add(1, Ordering::SeqCst);
        if self.fail_create.load(Ordering::SeqCst) {
            return Err(IxContractError::AuditUnavailable);
        }
        let stored = transaction.stored_run();
        let envelope = stored
            .envelopes()
            .last()
            .ok_or(IxContractError::InvalidLifecycle)?;
        let mut records = self
            .records
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if records.contains_key(&stored.snapshot().run_id) {
            return Err(IxContractError::InvalidLifecycle);
        }
        records.insert(stored.snapshot().run_id.clone(), stored.to_json_vec()?);
        repository_receipt(stored, envelope)
    }

    async fn load_run(
        &self,
        owner: &IxPrincipalBinding,
        run_id: &str,
    ) -> Result<Option<IxStoredRun>, IxContractError> {
        let stored = self
            .records
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(run_id)
            .cloned()
            .map(|bytes| IxStoredRun::from_json_slice(&bytes))
            .transpose()?;
        Ok(stored.filter(|stored| stored.snapshot().owner == *owner))
    }

    async fn compare_and_append(
        &self,
        transaction: IxRunCommitTransaction,
    ) -> Result<IxRepositoryCommitReceipt, IxContractError> {
        self.append_calls.fetch_add(1, Ordering::SeqCst);
        let is_completed = transaction.envelope().events.iter().any(|event| {
            matches!(
                &event.payload,
                appfw_runtime::ix::IxEventPayload::RunFinished {
                    outcome: IxTerminalOutcome::Completed,
                    ..
                }
            )
        });
        if is_completed && self.block_terminal_append.load(Ordering::SeqCst) {
            self.terminal_append_entered.notify_one();
            self.terminal_append_release.notified().await;
        }
        let mut records = self
            .records
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let current_bytes = records
            .get(&transaction.result_snapshot().run_id)
            .ok_or(IxContractError::RunNotFound)?;
        let current = IxStoredRun::from_json_slice(current_bytes)?;
        if current.snapshot().owner != *transaction.owner()
            || current.repository_version() != transaction.expected_repository_version()
            || current.snapshot().cursor != transaction.expected_cursor()
        {
            return Err(IxContractError::StaleAuthorization);
        }
        let stored = transaction.stored_run();
        let envelope = transaction.envelope();
        records.insert(stored.snapshot().run_id.clone(), stored.to_json_vec()?);
        repository_receipt(stored, envelope)
    }

    async fn mark_audit_projection(
        &self,
        projection: &IxAuditProjection,
        _state: IxAuditProjectionState,
    ) -> Result<(), IxContractError> {
        self.projections
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(projection.clone());
        if self.fail_projection.load(Ordering::SeqCst) {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            Err(IxContractError::AuditUnavailable)
        } else {
            Ok(())
        }
    }
}

fn repository_receipt(
    stored: &IxStoredRun,
    envelope: &appfw_runtime::ix::IxCommitEnvelope,
) -> Result<IxRepositoryCommitReceipt, IxContractError> {
    IxRepositoryCommitReceipt::new(
        stored.snapshot().run_id.clone(),
        stored.snapshot().cursor.clone(),
        stored.repository_version(),
        envelope.commit_id.clone(),
        false,
    )
}

/// Deterministic fixture seal-key identity and bytes. Every router built by
/// the shared fixture constructs its own independent `IxStoredRunSealKey`
/// object from these values; no sealer, global, or static is ever shared
/// between two service instances.
const FIXTURE_SEAL_KEY_ID: &str = "ix-local-proof-fixture";
const FIXTURE_SEAL_KEY_BYTES: [u8; 32] = *b"appfw-ix-local-proof-fixture-k1!";

fn fixture_seal_key() -> IxStoredRunSealKey {
    IxStoredRunSealKey::new(FIXTURE_SEAL_KEY_ID, FIXTURE_SEAL_KEY_BYTES)
        .expect("fixture stored-run seal key")
}

fn assemble_runtime_fixture_router(
    auth: &AuthHarness,
    repository: Arc<DurableMemoryRepository>,
    resolver: Arc<TestResolver>,
    orchestrator: Arc<TestOrchestrator>,
) -> Router {
    assemble_runtime_fixture_router_with_deadline(auth, repository, resolver, orchestrator, None)
}

fn assemble_runtime_fixture_router_with_deadline(
    auth: &AuthHarness,
    repository: Arc<DurableMemoryRepository>,
    resolver: Arc<TestResolver>,
    orchestrator: Arc<TestOrchestrator>,
    execution_deadline: Option<std::time::Duration>,
) -> Router {
    assemble_runtime_fixture_router_with_seal_key(
        auth,
        repository,
        resolver,
        orchestrator,
        execution_deadline,
        fixture_seal_key(),
    )
}

fn assemble_runtime_fixture_router_with_seal_key(
    auth: &AuthHarness,
    repository: Arc<DurableMemoryRepository>,
    resolver: Arc<TestResolver>,
    orchestrator: Arc<TestOrchestrator>,
    execution_deadline: Option<std::time::Duration>,
    seal_key: IxStoredRunSealKey,
) -> Router {
    let run_policy = IxRuntimeIntentPolicy::new(
        "Working brief",
        [IxArtifactBinding::new(
            "working-brief",
            "pds.ix.presentation@1",
            "pds.ix.working-brief@1",
        )
        .expect("artifact binding")],
    )
    .expect("run policy");
    let service = IxRuntimeService::new(
        repository,
        resolver,
        orchestrator,
        Arc::new(AllowContextPolicy),
        seal_key,
        [("working-brief".to_string(), run_policy)],
    )
    .expect("runtime service");
    let service = match execution_deadline {
        Some(deadline) => service
            .with_execution_deadline(deadline)
            .expect("bounded execution deadline"),
        None => service,
    };
    let service = Arc::new(service);
    let ix = ix_transport_routes(
        Arc::clone(&auth.verifier),
        Arc::clone(&auth.policy),
        service,
    )
    .expect("IX route group");
    let legacy = Router::new().route("/chat/stream", get(|| async { "legacy" }));
    let legacy_cors = CorsLayer::new()
        .allow_methods([Method::GET])
        .allow_origin(
            "https://legacy.example.com"
                .parse::<axum::http::HeaderValue>()
                .unwrap(),
        )
        .allow_credentials(true);
    assemble_runtime_router(
        RuntimeRouteSet::new().with_chat(legacy).with_ix_chat(ix),
        legacy_cors,
        MetricsRegistry::new("ix-test", "test"),
        &security(),
    )
}

struct AllowContextPolicy;

impl IxContextPolicy for AllowContextPolicy {
    fn decide_initial(
        &self,
        _claims: &IxContextClaims,
    ) -> Result<IxPolicyVerdict, IxContractError> {
        Ok(allow_verdict())
    }

    fn decide_reconciliation(
        &self,
        _current: &appfw_runtime::ix::IxContextRevision,
        _proposal: &IxContextProposal,
    ) -> Result<IxPolicyVerdict, IxContractError> {
        Ok(allow_verdict())
    }
}

fn allow_verdict() -> IxPolicyVerdict {
    IxPolicyVerdict {
        decision_id: "decision-1".to_string(),
        policy_version: "policy-1".to_string(),
        classification: IxDataClassification::Internal,
        consent: IxConsentState::Granted,
        egress: IxEgressPosture::LocalOnly,
    }
}

#[derive(Default)]
struct TestResolver {
    calls: AtomicUsize,
    block: AtomicBool,
}

impl TestResolver {
    fn set_block(&self, value: bool) {
        self.block.store(value, Ordering::SeqCst);
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl IxProductContextResolver for TestResolver {
    async fn resolve(
        &self,
        principal: &IxPrincipalBinding,
        request: &IxRunRequest,
        mut cancellation: IxCancellationSignal,
    ) -> Result<IxProductContext, IxContractError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(principal.tenant_id, "tenant-1");
        assert_eq!(request.question.as_deref(), Some("What should I do next?"));
        if self.block.load(Ordering::SeqCst) {
            let _ = cancellation.cancelled().await;
            return Err(IxContractError::CancellationConflict);
        }
        IxProductContext::new(
            "context-1",
            context_summary(),
            json!({"providerSafe": true}),
        )
    }
}

#[derive(Clone, Copy)]
enum OrchestratorMode {
    Complete,
    Pending,
}

struct TestOrchestrator {
    calls: AtomicUsize,
    mode: OrchestratorMode,
}

impl TestOrchestrator {
    fn new(mode: OrchestratorMode) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            mode,
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl IxProductOrchestrator for TestOrchestrator {
    async fn run(
        &self,
        prepared: IxPreparedRun,
        _cancellation: IxCancellationSignal,
    ) -> Result<BoxStream<'static, Result<IxOrchestrationDraft, IxContractError>>, IxContractError>
    {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(prepared.principal().tenant_id, "tenant-1");
        assert_eq!(
            prepared.request().question.as_deref(),
            Some("What should I do next?")
        );
        assert_eq!(
            prepared.context().expose_to_product(),
            &json!({"providerSafe": true})
        );
        Ok(match self.mode {
            OrchestratorMode::Complete => stream::iter([
                Ok(IxOrchestrationDraft::Phase {
                    phase: appfw_runtime::ix::IxPhase::Understanding,
                    reason: "Understand the bounded request.".to_string(),
                }),
                Ok(IxOrchestrationDraft::Artifact {
                    artifact_id: "fixture-working-brief".to_string(),
                    artifact_type: "working-brief".to_string(),
                    content_schema_version: "pds.ix.presentation@1".to_string(),
                    renderer_key: "pds.ix.working-brief@1".to_string(),
                    revision: 1,
                    status: IxArtifactStatus::Ready,
                    presentation: serde_json::from_str(include_str!(
                        "../contracts/pds_health/working-brief.presentation.json"
                    ))
                    .expect("canonical working-brief presentation"),
                    changed_region_ids: vec!["finding".to_string()],
                }),
                Ok(IxOrchestrationDraft::Completed {
                    message: "The bounded run completed.".to_string(),
                }),
            ])
            .boxed(),
            OrchestratorMode::Pending => {
                stream::pending::<Result<IxOrchestrationDraft, IxContractError>>().boxed()
            }
        })
    }
}

fn context_summary() -> appfw_runtime::ix::IxContextSummary {
    appfw_runtime::ix::IxContextSummary {
        focus_label: "Working brief".to_string(),
        detail: "Sanitized context summary.".to_string(),
        evaluated_at: "2026-08-14T00:00:00Z".to_string(),
        freshness: IxContextFreshness::Unknown,
        sources: Vec::new(),
        gaps: Vec::new(),
    }
}

struct AuthHarness {
    policy: Arc<IxTransportPolicy>,
    verifier: Arc<IxJwtVerifier>,
    encoding_key: jsonwebtoken::EncodingKey,
    claims: Value,
    token: String,
}

impl AuthHarness {
    fn token_for(&self, transform: impl FnOnce(&mut Value)) -> String {
        let mut claims = self.claims.clone();
        transform(&mut claims);
        sign_claims(&self.encoding_key, &claims)
    }
}

async fn generated_auth_harness() -> AuthHarness {
    let encoding_key = ix_rs256_test_encoding_key();
    let jwks = ix_rs256_test_jwks("ix-integration-generated").to_string();
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("loopback JWKS listener");
    let issuer = format!("http://{}", listener.local_addr().expect("JWKS address"));
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("JWKS request");
        let mut request = [0_u8; 4096];
        let _ = socket.read(&mut request).await.expect("read JWKS request");
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            jwks.len(),
            jwks
        );
        socket
            .write_all(response.as_bytes())
            .await
            .expect("write JWKS response");
    });
    let policy = Arc::new(
        IxTransportPolicy::new(
            issuer.clone(),
            "release-1",
            [
                IxClientBinding::new(
                    "nexus-browser",
                    "api://nexus-ix",
                    IxClientProfile::Browser,
                    ["https://nexus.example.com".to_string()],
                )
                .expect("browser binding"),
                IxClientBinding::new(
                    "nexus-native",
                    "api://nexus-ix",
                    IxClientProfile::Native,
                    Vec::<String>::new(),
                )
                .expect("native binding"),
            ],
            ["ix.user".to_string()],
            ["ix.run".to_string()],
        )
        .expect("IX transport policy"),
    );
    let verifier = Arc::new(
        IxJwtVerifier::new(Arc::clone(&policy))
            .await
            .expect("generated-key verifier"),
    );
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current epoch")
        .as_secs();
    let claims = json!({
        "iss": issuer,
        "sub": "human-1",
        "exp": now + 3600,
        "nbf": now.saturating_sub(1),
        "aud": "api://nexus-ix",
        "cid": "nexus-browser",
        "tenant": "tenant-1",
        "roles": ["ix.user"],
        "scp": "ix.run",
        "principalType": "human",
        "releaseId": "release-1",
        "sessionId": "session-1"
    });
    let token = sign_claims(&encoding_key, &claims);
    AuthHarness {
        policy,
        verifier,
        encoding_key,
        claims,
        token,
    }
}

fn sign_claims(key: &jsonwebtoken::EncodingKey, claims: &Value) -> String {
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some("ix-integration-generated".to_string());
    jsonwebtoken::encode(&header, claims, key).expect("ephemeral JWT")
}

struct RuntimeFixture {
    router: Router,
    repository: Arc<DurableMemoryRepository>,
    resolver: Arc<TestResolver>,
    orchestrator: Arc<TestOrchestrator>,
    auth: AuthHarness,
}

async fn runtime_fixture(mode: OrchestratorMode) -> RuntimeFixture {
    let auth = generated_auth_harness().await;
    let repository = Arc::new(DurableMemoryRepository::default());
    let resolver = Arc::new(TestResolver::default());
    let orchestrator = Arc::new(TestOrchestrator::new(mode));
    let router = assemble_runtime_fixture_router(
        &auth,
        repository.clone(),
        resolver.clone(),
        orchestrator.clone(),
    );
    RuntimeFixture {
        router,
        repository,
        resolver,
        orchestrator,
        auth,
    }
}

fn security() -> SecurityConfig {
    SecurityConfig {
        admin_ui_enabled: false,
        admin_troubleshooting_enabled: false,
        product_ui_enabled: false,
        chat_enabled: true,
        chat_required_roles: vec!["ix.user".to_string()],
        chat_required_scopes: vec!["ix.run".to_string()],
        chat_prompt_audit_enabled: true,
        chat_prompt_audit_siem_enabled: true,
        chat_prompt_audit_sink: Some("test".to_string()),
        chat_prompt_audit_retention_days: 90,
        chat_kill_switch_active: false,
        #[cfg(feature = "mcp")]
        mcp_enabled: false,
        #[cfg(feature = "mcp")]
        mcp_mutations_enabled: false,
        #[cfg(feature = "mcp")]
        mcp_allowed_origins: Vec::new(),
        #[cfg(feature = "mcp")]
        mcp_max_result_bytes: 256 * 1024,
        #[cfg(feature = "mcp")]
        mcp_max_resource_bytes: 256 * 1024,
        #[cfg(feature = "mcp")]
        mcp_max_batch_items: 20,
        #[cfg(feature = "mcp")]
        mcp_required_roles: vec!["admin".to_string()],
        #[cfg(feature = "mcp")]
        mcp_required_scopes: Vec::new(),
        #[cfg(feature = "mcp")]
        mcp_privileged_tool_roles: vec!["admin".to_string()],
        #[cfg(feature = "mcp")]
        mcp_privileged_tool_scopes: vec!["appfw:mcp.admin".to_string()],
        graphql_introspection_enabled: false,
        graphql_introspection_required_roles: vec!["admin".to_string()],
        graphql_introspection_required_scopes: Vec::new(),
        graphql_max_depth: 12,
        graphql_max_complexity: 500,
        request_body_limit_bytes: 1024 * 1024,
        rate_limit_per_second: 100,
        rate_limit_burst: 100,
    }
}

fn run_request() -> Value {
    json!({
        "schemaVersion": IX_RUN_REQUEST_SCHEMA_VERSION,
        "intentKey": "working-brief",
        "focus": {"kind": "case", "id": "case-1"},
        "question": "What should I do next?"
    })
}

fn post_request(token: &str, origin: Option<&str>, body: Value) -> Request<Body> {
    let mut builder = Request::builder()
        .method(Method::POST)
        .uri("/chat/stream")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(origin) = origin {
        builder = builder.header(header::ORIGIN, origin);
    }
    builder
        .body(Body::from(body.to_string()))
        .expect("IX POST request")
}

async fn start_and_read_ack(
    router: &Router,
    token: &str,
    origin: Option<&str>,
) -> (String, String) {
    let response = router
        .clone()
        .oneshot(post_request(token, origin, run_request()))
        .await
        .expect("IX start response");
    assert_eq!(response.status(), StatusCode::OK);
    let mut body = response.into_body();
    let chunk = tokio::time::timeout(std::time::Duration::from_secs(2), body.data())
        .await
        .expect("immediate acknowledgement timeout")
        .expect("acknowledgement body")
        .expect("acknowledgement bytes");
    let text = String::from_utf8(chunk.to_vec()).expect("UTF-8 SSE");
    let event = text
        .lines()
        .find_map(|line| line.strip_prefix("data:").map(str::trim))
        .and_then(|data| serde_json::from_str::<Value>(data).ok())
        .expect("canonical acknowledgement event");
    let run_id = event["runId"].as_str().expect("event run ID").to_string();
    (run_id, text)
}

async fn cancel_request(
    router: &Router,
    token: &str,
    run_id: &str,
    command_id: &str,
) -> (StatusCode, Vec<u8>) {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/chat/runs/{run_id}/cancel"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::ORIGIN, "https://nexus.example.com")
                .body(Body::from(
                    json!({
                        "schemaVersion": IX_CANCEL_REQUEST_SCHEMA_VERSION,
                        "commandId": command_id
                    })
                    .to_string(),
                ))
                .expect("cancel request"),
        )
        .await
        .expect("cancel response");
    let status = response.status();
    let bytes = collect_body(response.into_body()).await;
    (status, bytes)
}

async fn collect_body<B>(mut body: B) -> Vec<u8>
where
    B: axum::body::HttpBody<Data = axum::body::Bytes> + Unpin,
    B::Error: std::fmt::Debug,
{
    let mut bytes = Vec::new();
    while let Some(chunk) = body.data().await {
        bytes.extend_from_slice(&chunk.expect("response body chunk"));
    }
    bytes
}

async fn wait_for_terminal(repository: &DurableMemoryRepository, run_id: &str) -> IxStoredRun {
    for _ in 0..500 {
        if let Some(stored) = repository.stored(run_id) {
            if stored.snapshot().state == IxRunState::Terminal {
                return stored;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!(
        "run did not reach a terminal state: {:?}",
        repository.stored(run_id).map(|stored| (
            stored.snapshot().state,
            stored.snapshot().terminal_outcome,
            stored.repository_version(),
            stored.envelopes().len(),
            stored
                .accepted_cancel()
                .map(|cancel| cancel.command_id().to_string())
        ))
    );
}

#[tokio::test]
async fn deadline_terminalizes_and_releases_per_owner_capacity() {
    let auth = generated_auth_harness().await;
    let repository = Arc::new(DurableMemoryRepository::default());
    let resolver = Arc::new(TestResolver::default());
    resolver.set_block(true);
    let orchestrator = Arc::new(TestOrchestrator::new(OrchestratorMode::Pending));
    let router = assemble_runtime_fixture_router_with_deadline(
        &auth,
        Arc::clone(&repository),
        Arc::clone(&resolver),
        Arc::clone(&orchestrator),
        Some(std::time::Duration::from_secs(2)),
    );

    let mut run_ids = Vec::new();
    for _ in 0..4 {
        let (run_id, _) =
            start_and_read_ack(&router, &auth.token, Some("https://nexus.example.com")).await;
        run_ids.push(run_id);
    }
    let over_capacity = router
        .clone()
        .oneshot(post_request(
            &auth.token,
            Some("https://nexus.example.com"),
            run_request(),
        ))
        .await
        .expect("capacity response");
    assert_eq!(over_capacity.status(), StatusCode::CONFLICT);
    assert_eq!(repository.create_calls(), 4);

    for run_id in &run_ids {
        let terminal = wait_for_terminal(&repository, run_id).await;
        assert_eq!(
            terminal.snapshot().terminal_outcome,
            Some(IxTerminalOutcome::Failed)
        );
    }
    let (after_release, _) =
        start_and_read_ack(&router, &auth.token, Some("https://nexus.example.com")).await;
    assert_eq!(repository.create_calls(), 5);
    let terminal = wait_for_terminal(&repository, &after_release).await;
    assert_eq!(
        terminal.snapshot().terminal_outcome,
        Some(IxTerminalOutcome::Failed)
    );
    assert_eq!(orchestrator.calls(), 0);
}

#[tokio::test]
async fn token_free_verified_human_lifecycle_admission() {
    let fixture = runtime_fixture(OrchestratorMode::Pending).await;
    fixture.resolver.set_block(true);
    let (run_id, ack) = start_and_read_ack(
        &fixture.router,
        &fixture.auth.token,
        Some("https://nexus.example.com"),
    )
    .await;
    assert!(ack.contains("run_acknowledged"));
    let stored = fixture
        .repository
        .stored(&run_id)
        .expect("atomic start record");
    assert_eq!(
        stored.envelopes().len(),
        1,
        "ack is immediate before Product work"
    );
    assert!(stored.snapshot().request.question.is_none());
    assert_eq!(stored.snapshot().runner.subject, "appfw-ix-orchestrator");
    assert_eq!(
        stored
            .snapshot()
            .authorization_context
            .policy_decision()
            .authority()
            .subject,
        "appfw-ix-policy-authority"
    );
    assert_eq!(
        stored.start_audit_fact().intent_key.as_deref(),
        Some("working-brief")
    );
    assert!(!String::from_utf8(stored.to_json_vec().unwrap())
        .unwrap()
        .contains("What should I do next?"));
    assert_eq!(fixture.repository.create_calls(), 1);
    for _ in 0..100 {
        if fixture.resolver.calls() == 1 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(fixture.resolver.calls(), 1);
    assert_eq!(fixture.orchestrator.calls(), 0);

    let (status, _) = cancel_request(
        &fixture.router,
        &fixture.auth.token,
        &run_id,
        "cancel-admission",
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let terminal = wait_for_terminal(&fixture.repository, &run_id).await;
    assert_eq!(
        terminal.snapshot().terminal_outcome,
        Some(IxTerminalOutcome::Cancelled)
    );
}

#[tokio::test]
async fn durable_repository_cold_rehydration_and_atomic_create() {
    let fixture = runtime_fixture(OrchestratorMode::Complete).await;
    fixture.repository.set_fail_create(true);
    let failed = fixture
        .router
        .clone()
        .oneshot(post_request(
            &fixture.auth.token,
            Some("https://nexus.example.com"),
            run_request(),
        ))
        .await
        .expect("failed start response");
    assert_eq!(failed.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(fixture
        .repository
        .records
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .is_empty());
    assert_eq!(fixture.resolver.calls(), 0);
    assert_eq!(fixture.orchestrator.calls(), 0);

    fixture.repository.set_fail_create(false);
    let (run_id, _) = start_and_read_ack(
        &fixture.router,
        &fixture.auth.token,
        Some("https://nexus.example.com"),
    )
    .await;
    let terminal = wait_for_terminal(&fixture.repository, &run_id).await;
    let frozen = terminal.to_json_vec().expect("portable stored record");
    let cold = IxStoredRun::from_json_slice(&frozen).expect("cold record validation");
    assert_eq!(cold.snapshot(), terminal.snapshot());
    assert!(cold.envelopes().len() >= 5);
    assert_eq!(
        cold.snapshot().terminal_outcome,
        Some(IxTerminalOutcome::Completed)
    );
    let mut tampered_audit: Value = serde_json::from_slice(&frozen).expect("stored record JSON");
    tampered_audit["startAuditFact"]["requestId"] =
        Value::String("caller-controlled request id".to_string());
    assert_eq!(
        IxStoredRun::from_json_slice(
            &serde_json::to_vec(&tampered_audit).expect("tampered stored bytes")
        )
        .err(),
        Some(IxContractError::InvalidLifecycle)
    );

    let ack_cursor = cold.envelopes()[0].result_cursor.clone();
    let cold_router = assemble_runtime_fixture_router(
        &fixture.auth,
        Arc::clone(&fixture.repository),
        Arc::new(TestResolver::default()),
        Arc::new(TestOrchestrator::new(OrchestratorMode::Pending)),
    );
    let replay = cold_router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/chat/stream")
                .header(
                    header::AUTHORIZATION,
                    format!("Bearer {}", fixture.auth.token),
                )
                .header(header::ORIGIN, "https://nexus.example.com")
                .header("last-event-id", ack_cursor)
                .body(Body::empty())
                .expect("cold replay request"),
        )
        .await
        .expect("cold replay response");
    assert_eq!(replay.status(), StatusCode::OK);
    let replay_text =
        String::from_utf8(collect_body(replay.into_body()).await).expect("UTF-8 replay");
    assert!(replay_text.contains("context_resolved"));
    assert!(replay_text.contains("run_finished"));
    assert!(!replay_text.contains("What should I do next?"));
}

/// Independent per-test durable seal secrets. Each service under test
/// constructs its own `IxStoredRunSealKey` object from these fixed non-zero
/// bytes; the persisted repository bytes are the only lifecycle handoff.
const DURABLE_SEAL_KEY_A_BYTES: [u8; 32] = *b"ix-local-proof-secret-a-32bytes!";
const DURABLE_SEAL_KEY_B_BYTES: [u8; 32] = *b"ix-local-proof-secret-b-32bytes!";

async fn replay_from_cursor(router: &Router, token: &str, cursor: &str) -> (StatusCode, String) {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/chat/stream")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::ORIGIN, "https://nexus.example.com")
                .header("last-event-id", cursor)
                .body(Body::empty())
                .expect("replay request"),
        )
        .await
        .expect("replay response");
    let status = response.status();
    let body = collect_body(response.into_body()).await;
    (status, String::from_utf8(body).expect("UTF-8 replay body"))
}

fn raw_record_bytes(repository: &DurableMemoryRepository, run_id: &str) -> Vec<u8> {
    repository
        .records
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(run_id)
        .expect("persisted stored-run bytes")
        .clone()
}

fn overwrite_record_bytes(repository: &DurableMemoryRepository, run_id: &str, bytes: Vec<u8>) {
    repository
        .records
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(run_id.to_string(), bytes);
}

#[tokio::test]
async fn durable_seal_rehydrates_across_independent_service_instances() {
    let auth = generated_auth_harness().await;
    let repository = Arc::new(DurableMemoryRepository::default());

    // Service A owns its own key object; nothing about the sealer is shared.
    let router_a = assemble_runtime_fixture_router_with_seal_key(
        &auth,
        Arc::clone(&repository),
        Arc::new(TestResolver::default()),
        Arc::new(TestOrchestrator::new(OrchestratorMode::Complete)),
        None,
        IxStoredRunSealKey::new("ix-local-proof-a", DURABLE_SEAL_KEY_A_BYTES)
            .expect("service A seal key"),
    );
    let (run_id, _) =
        start_and_read_ack(&router_a, &auth.token, Some("https://nexus.example.com")).await;
    let terminal = wait_for_terminal(&repository, &run_id).await;
    assert_eq!(terminal.runtime_seal_key_id(), "ix-local-proof-a");
    let ack_cursor = terminal.envelopes()[0].result_cursor.clone();
    let terminal_cursor = terminal.snapshot().cursor.clone();
    let persisted_bytes = raw_record_bytes(&repository, &run_id);
    drop(router_a);

    // Service B is independently constructed from a separately built key
    // object with the same id and bytes; only persistence bytes carry over.
    let resolver_b = Arc::new(TestResolver::default());
    let orchestrator_b = Arc::new(TestOrchestrator::new(OrchestratorMode::Pending));
    let router_b = assemble_runtime_fixture_router_with_seal_key(
        &auth,
        Arc::clone(&repository),
        Arc::clone(&resolver_b),
        Arc::clone(&orchestrator_b),
        None,
        IxStoredRunSealKey::new("ix-local-proof-a", DURABLE_SEAL_KEY_A_BYTES)
            .expect("service B seal key"),
    );
    let (first_status, first_text) = replay_from_cursor(&router_b, &auth.token, &ack_cursor).await;
    assert_eq!(first_status, StatusCode::OK);
    assert!(first_text.contains("context_resolved"));
    assert!(first_text.contains("run_finished"));
    assert!(first_text.contains(&format!("\nid:{terminal_cursor}")));
    assert!(!first_text.contains("What should I do next?"));

    let (second_status, second_text) =
        replay_from_cursor(&router_b, &auth.token, &ack_cursor).await;
    assert_eq!(second_status, StatusCode::OK);
    assert_eq!(
        first_text, second_text,
        "independent rehydrated replays are byte-identical"
    );
    assert_eq!(
        raw_record_bytes(&repository, &run_id),
        persisted_bytes,
        "replay does not mutate the persisted record"
    );
    assert_eq!(resolver_b.calls(), 0, "replay resolves no Product context");
    assert_eq!(
        orchestrator_b.calls(),
        0,
        "replay runs no Product orchestration"
    );
}

#[tokio::test]
async fn durable_seal_rejects_fresh_service_with_wrong_key() {
    let auth = generated_auth_harness().await;
    let repository = Arc::new(DurableMemoryRepository::default());
    let router_a = assemble_runtime_fixture_router_with_seal_key(
        &auth,
        Arc::clone(&repository),
        Arc::new(TestResolver::default()),
        Arc::new(TestOrchestrator::new(OrchestratorMode::Complete)),
        None,
        IxStoredRunSealKey::new("ix-local-proof-a", DURABLE_SEAL_KEY_A_BYTES)
            .expect("service A seal key"),
    );
    let (run_id, _) =
        start_and_read_ack(&router_a, &auth.token, Some("https://nexus.example.com")).await;
    let terminal = wait_for_terminal(&repository, &run_id).await;
    let ack_cursor = terminal.envelopes()[0].result_cursor.clone();
    let persisted_bytes = raw_record_bytes(&repository, &run_id);
    let baseline_creates = repository.create_calls();
    let baseline_appends = repository.append_calls();
    drop(router_a);

    // Same key id, different non-zero secret bytes.
    let resolver_b = Arc::new(TestResolver::default());
    let orchestrator_b = Arc::new(TestOrchestrator::new(OrchestratorMode::Pending));
    let router_b = assemble_runtime_fixture_router_with_seal_key(
        &auth,
        Arc::clone(&repository),
        Arc::clone(&resolver_b),
        Arc::clone(&orchestrator_b),
        None,
        IxStoredRunSealKey::new("ix-local-proof-a", DURABLE_SEAL_KEY_B_BYTES)
            .expect("wrong-key service seal key"),
    );
    let (replay_status, replay_text) =
        replay_from_cursor(&router_b, &auth.token, &ack_cursor).await;
    assert_eq!(replay_status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(replay_text.contains("unavailable"));
    assert!(!replay_text.contains("data:"), "no replay events leak");
    let (cancel_status, _) =
        cancel_request(&router_b, &auth.token, &run_id, "cancel-wrong-key").await;
    assert_eq!(cancel_status, StatusCode::SERVICE_UNAVAILABLE);

    // Same secret bytes, different key id: key-id equality is required
    // before MAC verification.
    let resolver_c = Arc::new(TestResolver::default());
    let orchestrator_c = Arc::new(TestOrchestrator::new(OrchestratorMode::Pending));
    let router_c = assemble_runtime_fixture_router_with_seal_key(
        &auth,
        Arc::clone(&repository),
        Arc::clone(&resolver_c),
        Arc::clone(&orchestrator_c),
        None,
        IxStoredRunSealKey::new("ix-local-proof-c", DURABLE_SEAL_KEY_A_BYTES)
            .expect("mismatched-id service seal key"),
    );
    let (mismatch_status, _) = replay_from_cursor(&router_c, &auth.token, &ack_cursor).await;
    assert_eq!(mismatch_status, StatusCode::SERVICE_UNAVAILABLE);

    assert_eq!(resolver_b.calls() + resolver_c.calls(), 0);
    assert_eq!(orchestrator_b.calls() + orchestrator_c.calls(), 0);
    assert_eq!(repository.create_calls(), baseline_creates);
    assert_eq!(repository.append_calls(), baseline_appends);
    assert_eq!(
        raw_record_bytes(&repository, &run_id),
        persisted_bytes,
        "wrong-key access mutates nothing"
    );
}

#[tokio::test]
async fn durable_seal_rejects_structurally_valid_persisted_tamper() {
    let auth = generated_auth_harness().await;
    let repository = Arc::new(DurableMemoryRepository::default());
    let router_a = assemble_runtime_fixture_router_with_seal_key(
        &auth,
        Arc::clone(&repository),
        Arc::new(TestResolver::default()),
        Arc::new(TestOrchestrator::new(OrchestratorMode::Complete)),
        None,
        IxStoredRunSealKey::new("ix-local-proof-a", DURABLE_SEAL_KEY_A_BYTES)
            .expect("service A seal key"),
    );
    let (run_id, _) =
        start_and_read_ack(&router_a, &auth.token, Some("https://nexus.example.com")).await;
    let terminal = wait_for_terminal(&repository, &run_id).await;
    let ack_cursor = terminal.envelopes()[0].result_cursor.clone();
    let baseline_appends = repository.append_calls();
    drop(router_a);

    // Replace an HMAC-covered value with one that still passes its local
    // shape rule (another canonical UUID), then re-encode without changing
    // the retained seal.
    let mut tampered: Value = serde_json::from_slice(&raw_record_bytes(&repository, &run_id))
        .expect("persisted stored-run JSON");
    let replacement_request_id = "11111111-2222-4333-8444-555555555555";
    assert_ne!(
        tampered["startAuditFact"]["requestId"]
            .as_str()
            .expect("persisted request id"),
        replacement_request_id
    );
    tampered["startAuditFact"]["requestId"] = Value::String(replacement_request_id.to_string());
    let tampered_bytes = serde_json::to_vec(&tampered).expect("tampered stored bytes");
    let decoded = IxStoredRun::from_json_slice(&tampered_bytes)
        .expect("tamper stays structurally valid at repository decode");
    assert_eq!(
        decoded.start_audit_fact().request_id,
        replacement_request_id,
        "tampered value survives structural decoding"
    );
    overwrite_record_bytes(&repository, &run_id, tampered_bytes.clone());

    // A fresh service holding the CORRECT key still rejects the record
    // before any replay or mutation.
    let resolver_b = Arc::new(TestResolver::default());
    let orchestrator_b = Arc::new(TestOrchestrator::new(OrchestratorMode::Pending));
    let router_b = assemble_runtime_fixture_router_with_seal_key(
        &auth,
        Arc::clone(&repository),
        Arc::clone(&resolver_b),
        Arc::clone(&orchestrator_b),
        None,
        IxStoredRunSealKey::new("ix-local-proof-a", DURABLE_SEAL_KEY_A_BYTES)
            .expect("service B seal key"),
    );
    let (replay_status, replay_text) =
        replay_from_cursor(&router_b, &auth.token, &ack_cursor).await;
    assert_eq!(replay_status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(replay_text.contains("unavailable"));
    assert!(!replay_text.contains("data:"), "no replay events leak");
    let (cancel_status, _) =
        cancel_request(&router_b, &auth.token, &run_id, "cancel-tampered").await;
    assert_eq!(cancel_status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(resolver_b.calls(), 0);
    assert_eq!(orchestrator_b.calls(), 0);
    assert_eq!(repository.append_calls(), baseline_appends);
    assert_eq!(
        raw_record_bytes(&repository, &run_id),
        tampered_bytes,
        "rejection neither repairs nor mutates repository bytes"
    );
}

#[tokio::test]
async fn cancellation_audit_projection_matrix() {
    let fixture = runtime_fixture(OrchestratorMode::Pending).await;
    let (run_id, _) = start_and_read_ack(
        &fixture.router,
        &fixture.auth.token,
        Some("https://nexus.example.com"),
    )
    .await;
    for _ in 0..100 {
        if fixture.orchestrator.calls() == 1 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    fixture.repository.set_fail_projection(true);
    let (first_status, first_body) = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        cancel_request(&fixture.router, &fixture.auth.token, &run_id, "cancel-1"),
    )
    .await
    .expect("external audit outage must not delay accepted cancellation");
    assert_eq!(first_status, StatusCode::ACCEPTED);
    let first_receipt: Value = serde_json::from_slice(&first_body).expect("cancel receipt JSON");
    assert_eq!(first_receipt["auditRetained"], true);
    assert_eq!(first_receipt["auditProjected"], "pending");
    assert!(first_receipt.get("auditProjectionCursor").is_none());
    let terminal = wait_for_terminal(&fixture.repository, &run_id).await;
    assert_eq!(
        terminal.snapshot().terminal_outcome,
        Some(IxTerminalOutcome::Cancelled)
    );
    let accepted = terminal
        .accepted_cancel()
        .expect("accepted cancel retained");
    let cursor = accepted.cursor().to_string();
    let dedup = accepted.audit_projection().dedup_key().to_string();
    let mut tampered_projection: Value =
        serde_json::from_slice(&terminal.to_json_vec().expect("terminal stored bytes"))
            .expect("terminal stored JSON");
    tampered_projection["acceptedCancel"]["auditProjection"]["dedupKey"] =
        Value::String("different-dedup-key".to_string());
    // Repository decode rejects the tamper structurally (dedup-key
    // integrity); MAC acceptance is owned by the service sealer alone.
    assert_eq!(
        IxStoredRun::from_json_slice(
            &serde_json::to_vec(&tampered_projection).expect("tampered projection bytes")
        )
        .err(),
        Some(IxContractError::InvalidCancellationProgress)
    );
    assert_eq!(
        terminal
            .envelopes()
            .iter()
            .flat_map(|envelope| envelope.events.iter())
            .filter(|event| matches!(
                &event.payload,
                appfw_runtime::ix::IxEventPayload::CancelRequested { .. }
            ))
            .count(),
        1
    );

    fixture.repository.set_fail_projection(false);
    let cold_router = assemble_runtime_fixture_router(
        &fixture.auth,
        Arc::clone(&fixture.repository),
        Arc::new(TestResolver::default()),
        Arc::new(TestOrchestrator::new(OrchestratorMode::Pending)),
    );
    let (retry_status, retry_body) =
        cancel_request(&cold_router, &fixture.auth.token, &run_id, "cancel-1").await;
    assert_eq!(retry_status, StatusCode::ACCEPTED);
    assert_eq!(first_body, retry_body, "exact retry receipt is byte-stable");
    let attempts = wait_for_projection_attempts(&fixture.repository, 2).await;
    assert!(attempts.len() >= 2);
    assert!(attempts
        .iter()
        .all(|attempt| { attempt.cursor() == cursor && attempt.dedup_key() == dedup }));

    let (different_status, _) =
        cancel_request(&fixture.router, &fixture.auth.token, &run_id, "cancel-2").await;
    assert_eq!(different_status, StatusCode::CONFLICT);
    assert!(fixture.repository.append_calls() >= 2);
    assert_eq!(
        fixture
            .repository
            .stored(&run_id)
            .expect("durable cancel record")
            .accepted_cancel()
            .expect("accepted cancel")
            .dedup_key_for_test(),
        dedup
    );
}

#[tokio::test]
async fn completion_cancel_race_has_one_cancelled_terminal_truth() {
    let fixture = runtime_fixture(OrchestratorMode::Complete).await;
    fixture.repository.set_block_terminal_append(true);
    let (run_id, _) = start_and_read_ack(
        &fixture.router,
        &fixture.auth.token,
        Some("https://nexus.example.com"),
    )
    .await;
    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        fixture.repository.wait_for_terminal_append(),
    )
    .await
    .expect("completion append barrier");

    let (status, _) =
        cancel_request(&fixture.router, &fixture.auth.token, &run_id, "cancel-race").await;
    assert_eq!(status, StatusCode::ACCEPTED);
    fixture.repository.release_terminal_append();
    let terminal = wait_for_terminal(&fixture.repository, &run_id).await;
    assert_eq!(
        terminal.snapshot().terminal_outcome,
        Some(IxTerminalOutcome::Cancelled)
    );
    let outcomes = terminal
        .envelopes()
        .iter()
        .flat_map(|envelope| envelope.events.iter())
        .filter_map(|event| match &event.payload {
            appfw_runtime::ix::IxEventPayload::RunFinished { outcome, .. } => Some(*outcome),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(outcomes, vec![IxTerminalOutcome::Cancelled]);
    assert_eq!(
        terminal
            .envelopes()
            .iter()
            .flat_map(|envelope| envelope.events.iter())
            .filter(|event| matches!(
                &event.payload,
                appfw_runtime::ix::IxEventPayload::CancelRequested { .. }
            ))
            .count(),
        1
    );
}

async fn wait_for_projection_attempts(
    repository: &DurableMemoryRepository,
    minimum: usize,
) -> Vec<IxAuditProjection> {
    for _ in 0..100 {
        let attempts = repository.projection_attempts();
        if attempts.len() >= minimum {
            return attempts;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    repository.projection_attempts()
}

#[tokio::test]
async fn assembled_router_cors_auth_matrix() {
    let fixture = runtime_fixture(OrchestratorMode::Complete).await;
    let preflight = fixture
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/chat/stream")
                .header(header::ORIGIN, "https://nexus.example.com")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                .header(
                    header::ACCESS_CONTROL_REQUEST_HEADERS,
                    "authorization,content-type,last-event-id",
                )
                .body(Body::empty())
                .expect("IX preflight"),
        )
        .await
        .expect("preflight response");
    assert_eq!(
        preflight.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN],
        "https://nexus.example.com"
    );
    assert!(preflight
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_CREDENTIALS)
        .is_none());
    assert!(preflight.headers()[header::ACCESS_CONTROL_ALLOW_HEADERS]
        .to_str()
        .unwrap()
        .contains("last-event-id"));

    let rejected_preflight = fixture
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/chat/stream")
                .header(header::ORIGIN, "https://nexus.example.com")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                .header(
                    header::ACCESS_CONTROL_REQUEST_HEADERS,
                    "authorization,x-appfw-ix-client-profile",
                )
                .body(Body::empty())
                .expect("rejected preflight"),
        )
        .await
        .expect("rejected preflight response");
    let rejected_headers = rejected_preflight.headers()[header::ACCESS_CONTROL_ALLOW_HEADERS]
        .to_str()
        .unwrap()
        .to_ascii_lowercase();
    assert!(!rejected_headers.contains("x-appfw-ix-client-profile"));
    assert!(!rejected_headers.contains('*'));

    let baseline_create = fixture.repository.create_calls();
    let baseline_resolve = fixture.resolver.calls();
    for request in [
        post_request(
            &fixture.auth.token,
            Some("https://wrong.example.com"),
            run_request(),
        ),
        {
            let mut request = post_request(
                &fixture.auth.token,
                Some("https://nexus.example.com"),
                run_request(),
            );
            request
                .headers_mut()
                .insert(header::COOKIE, "session=forbidden".parse().unwrap());
            request
        },
        {
            let mut request = post_request(
                &fixture.auth.token,
                Some("https://nexus.example.com"),
                run_request(),
            );
            request
                .headers_mut()
                .insert("x-appfw-ix-client-profile", "ix-browser@1".parse().unwrap());
            request
        },
        post_request(
            &fixture.auth.token_for(|claims| claims["roles"] = json!([])),
            Some("https://nexus.example.com"),
            run_request(),
        ),
    ] {
        let response = fixture.router.clone().oneshot(request).await.unwrap();
        assert!(matches!(
            response.status(),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
        ));
    }
    assert_eq!(fixture.repository.create_calls(), baseline_create);
    assert_eq!(fixture.resolver.calls(), baseline_resolve);

    let native_token = fixture.auth.token_for(|claims| {
        claims["cid"] = json!("nexus-native");
    });
    let native = fixture
        .router
        .clone()
        .oneshot(post_request(&native_token, None, run_request()))
        .await
        .expect("originless native response");
    assert_eq!(native.status(), StatusCode::OK);

    let legacy = fixture
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/chat/stream")
                .header(header::ORIGIN, "https://legacy.example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(legacy.status(), StatusCode::OK);
    assert_eq!(
        legacy.headers()[header::ACCESS_CONTROL_ALLOW_CREDENTIALS],
        "true"
    );

    let raw_id = "caller-controlled-sensitive-marker";
    let response = fixture
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/chat/stream")
                .header(header::AUTHORIZATION, "Bearer invalid.token.value")
                .header(header::ORIGIN, "https://nexus.example.com")
                .header("x-request-id", raw_id)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let safe_id = response.headers()["x-request-id"].to_str().unwrap();
    assert!(uuid::Uuid::parse_str(safe_id).is_ok());
    assert_ne!(safe_id, raw_id);
    let body = collect_body(response.into_body()).await;
    assert!(!String::from_utf8_lossy(&body).contains("caller-data"));
}

#[test]
fn product_context_payload_is_bounded_without_a_second_materialized_copy() {
    IxProductContext::new(
        "context-boundary",
        context_summary(),
        Value::String("x".repeat(512 * 1024 - 2)),
    )
    .expect("exact serialized byte boundary");
    assert_eq!(
        IxProductContext::new(
            "context-overflow",
            context_summary(),
            Value::String("x".repeat(512 * 1024 - 1)),
        )
        .err(),
        Some(IxContractError::ResourceLimitExceeded)
    );
}

#[test]
fn ix_route_group_api_exclusivity() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_dir = manifest_dir.join("tests/fixtures/ix");
    let root = std::env::temp_dir().join(format!(
        "appfw-ix-compile-fixtures-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("compile-fixture root");
    compile_external_fixture(
        &manifest_dir,
        &fixture_dir.join("external_good.rs"),
        true,
        &root,
    );
    for fixture in [
        "external_bad_fields.rs",
        "external_bad_clone.rs",
        "external_bad_legacy_assembly.rs",
        "external_bad_service_substitution.rs",
        "external_bad_verified_human.rs",
        "external_bad_actor_injection.rs",
    ] {
        compile_external_fixture(&manifest_dir, &fixture_dir.join(fixture), false, &root);
    }
    fs::remove_dir_all(&root).expect("remove disposable compile fixtures");
}

fn compile_external_fixture(
    manifest_dir: &Path,
    source: &Path,
    should_compile: bool,
    fixture_root: &Path,
) {
    let root = fixture_root.join(
        source
            .file_stem()
            .and_then(|name| name.to_str())
            .expect("fixture stem"),
    );
    let package_name = source
        .file_stem()
        .and_then(|name| name.to_str())
        .expect("fixture stem")
        .replace('_', "-");
    fs::create_dir_all(root.join("src")).expect("fixture directory");
    fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{package_name}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[workspace]\n\n[dependencies]\nappfw-runtime = {{ path = {:?}, default-features = false, features = [\"chat\"] }}\n",
            manifest_dir
        ),
    )
    .expect("fixture manifest");
    fs::copy(source, root.join("src/main.rs")).expect("fixture source");
    let output = Command::new("cargo")
        .args(["check", "--offline", "--quiet", "--manifest-path"])
        .arg(root.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", fixture_root.join("target"))
        .output()
        .expect("external Cargo check");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.success(),
        should_compile,
        "external fixture {} unexpected result:\n{stderr}",
        source.display()
    );
}

trait AcceptedCancelTestExt {
    fn dedup_key_for_test(&self) -> String;
}

impl AcceptedCancelTestExt for appfw_runtime::ix::IxAcceptedCancel {
    fn dedup_key_for_test(&self) -> String {
        self.audit_projection().dedup_key().to_string()
    }
}
