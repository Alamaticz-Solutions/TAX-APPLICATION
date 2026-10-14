//! This module's own G1-style governed-write gate. Every SharePoint WRITE
//! passes through [`execute`] - the ONLY function in this module that can issue
//! a non-GET request to `graph.microsoft.com`. In order:
//!
//!   1. GovernedWriteEnforcement      this module is the only mutation path
//!   2. DelegatedActorContext         [`WriteContext`] carries the acting user
//!   3. TokenStoreIsolation           [`BearerSource`]: the write-scoped Graph
//!                                    app registration (D4) - no fallback to
//!                                    the read app
//!   4. NamedMutationRegistry         [`WriteOperation`], allow-listed, no
//!                                    caller endpoints
//!   5. MutationRequestBinding        `WriteOperation::plan`, built by the
//!                                    framework crate: typed, escaped, 2 MB cap
//!   6. IdempotencyAndReplayProtection [`WriteLedger`], checked before any
//!                                    network call
//!   7. WritePolicyAndScopeEnforcement `policy_check` (scope
//!                                    `sharepoint.drive.write`)
//!   8. WriteAuditAndEvidence         [`WriteAuditSink`] + the ledger row, on
//!                                    every outcome
//!
//! The gate is written against four small traits (ledger, audit sink,
//! transport, bearer source) so it is fully unit-testable with fakes and so the
//! persistence choice stays separate from the gate. What is NOT built: a
//! durable, database-backed ledger and audit sink. Those need a new entity in
//! `.appfw/model` (a generated surface), which was deliberately not touched;
//! see `docs/architecture/sharepoint-integration-plan.md` section "Built vs
//! gated". [`InMemoryWriteLedger`] / [`InMemoryAuditSink`] are the only
//! implementations today and are NOT durable.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::Serialize;
use sha2::{Digest, Sha256};

use appfw_provider_sharepoint::{
    SharePointNamedOperationRequest, SharePointOperationRegistry, SharePointProviderError,
    PARAM_CONTENT_LENGTH, PARAM_CONTENT_TYPE, PARAM_DRIVE_ID, PARAM_FILE_PATH,
    SHAREPOINT_MAX_UPLOAD_BYTES, SHAREPOINT_UPLOAD_FILE_OPERATION,
    SHAREPOINT_UPLOAD_FILE_POLICY_SCOPE,
};

use super::{
    auth::write_config,
    identity::SHAREPOINT_GRAPH_BASE,
    request::RequestPlan,
    response::{classify, redact, GraphError},
};
use crate::services::graph::auth::GraphToken;

/// Only PDFs are accepted: the source spec routes PDF tax documents. Widening
/// this list is a product decision, not an incidental change.
pub const ALLOWED_CONTENT_TYPES: &[&str] = &["application/pdf"];

/// Roles allowed to perform `sharepoint_upload_file` (spec-2: exactly two
/// roles exist). Record-level ownership (`assigned_operator`) is the calling
/// service's check; this gate answers only "may this role write to SharePoint".
pub const UPLOAD_ALLOWED_ROLES: &[&str] = &["tax_staff", "tax_admin"];

/// G1.2 DelegatedActorContext.
#[derive(Debug, Clone)]
pub struct WriteContext {
    pub actor: String,
    pub roles: Vec<String>,
    pub tenant_id: String,
}

impl WriteContext {
    pub fn new(
        actor: impl Into<String>,
        roles: impl IntoIterator<Item = impl AsRef<str>>,
        tenant_id: impl Into<String>,
    ) -> Self {
        Self {
            actor: actor.into(),
            roles: roles
                .into_iter()
                .map(|r| r.as_ref().to_ascii_lowercase())
                .collect(),
            tenant_id: tenant_id.into(),
        }
    }

    pub fn from_user(user: &crate::product_api::UserAuth) -> Self {
        Self::new(
            user.user_name.clone(),
            user.roles.iter(),
            user.tenant_id.clone(),
        )
    }
}

/// G1.4 NamedMutationRegistry. Allow-listed write candidates only.
#[derive(Debug, Clone)]
pub enum WriteOperation {
    /// `PUT /drives/{drive}/root:/{path}:/content?@microsoft.graph.conflictBehavior=fail`.
    ///
    /// Simple (non-chunked) upload. `path` is the already-sanitised
    /// `{entity_folder}/{document_id}__{file_name}` built by [`upload_path`] -
    /// never caller-supplied raw text. `conflictBehavior=fail` makes a retry of
    /// the same path either replay through the ledger or fail loudly; it never
    /// silently overwrites. `content_sha256` is not sent to Graph; it is part
    /// of the idempotency fingerprint, so different bytes at the same path are
    /// a different write, not a replay.
    UploadFile {
        drive_id: String,
        path: String,
        content_type: String,
        content_sha256: String,
        content_length: u64,
    },
}

impl WriteOperation {
    /// Builds the operation from the bytes themselves, so the declared hash
    /// and length can never disagree with the body that is sent.
    pub fn upload_file(
        drive_id: impl Into<String>,
        path: impl Into<String>,
        content_type: impl Into<String>,
        bytes: &[u8],
    ) -> Self {
        Self::UploadFile {
            drive_id: drive_id.into(),
            path: path.into(),
            content_type: content_type.into(),
            content_sha256: sha256_hex_bytes(bytes),
            content_length: bytes.len() as u64,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::UploadFile { .. } => "sharepoint_upload_file",
        }
    }

    /// The policy scope this operation is gated behind (own scope, not shared
    /// with any other Graph write).
    pub fn policy_scope(&self) -> &'static str {
        match self {
            Self::UploadFile { .. } => SHAREPOINT_UPLOAD_FILE_POLICY_SCOPE,
        }
    }

    /// G1.5 MutationRequestBinding: the plan is built and validated by the
    /// framework crate (id/path rules, 2 MB cap), then the product-only
    /// content-type allow-list is applied.
    pub fn plan(&self) -> Result<RequestPlan, WriteError> {
        match self {
            Self::UploadFile {
                drive_id,
                path,
                content_type,
                content_length,
                ..
            } => {
                if !ALLOWED_CONTENT_TYPES.contains(&content_type.as_str()) {
                    return Err(WriteError::InvalidRequest(format!(
                        "content type `{content_type}` is not accepted (allowed: {ALLOWED_CONTENT_TYPES:?})"
                    )));
                }
                let request = SharePointNamedOperationRequest::new(
                    SHAREPOINT_UPLOAD_FILE_OPERATION,
                )
                .with_param(PARAM_DRIVE_ID, drive_id.as_str())
                .with_param(PARAM_FILE_PATH, path.as_str())
                .with_param(PARAM_CONTENT_TYPE, content_type.as_str())
                .with_param(PARAM_CONTENT_LENGTH, content_length.to_string());
                let planned = SharePointOperationRegistry::default()
                    .build_plan(&request)
                    .map_err(WriteError::from_provider)?;
                let mut plan = RequestPlan {
                    method: reqwest::Method::PUT,
                    path: planned.path,
                    query: Vec::new(),
                    headers: Vec::new(),
                    accept: vec!["application/json"],
                };
                for (key, value) in planned.query {
                    plan = plan.query(key, value);
                }
                Ok(plan)
            }
        }
    }

    /// G1.6 idempotency-key input: every bound parameter, stable order,
    /// including the content hash.
    fn fingerprint_fields(&self) -> Vec<(&'static str, String)> {
        match self {
            Self::UploadFile {
                drive_id,
                path,
                content_type,
                content_sha256,
                content_length,
            } => vec![
                ("drive_id", drive_id.clone()),
                ("path", path.clone()),
                ("content_type", content_type.clone()),
                ("content_sha256", content_sha256.clone()),
                ("content_length", content_length.to_string()),
            ],
        }
    }

    fn content_type(&self) -> &str {
        match self {
            Self::UploadFile { content_type, .. } => content_type,
        }
    }
}

fn sha256_hex_bytes(input: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    format!("{:x}", hasher.finalize())
}

/// G1.6: `sha256(operation || sorted-field=value pairs)`.
fn fingerprint(op: &WriteOperation) -> String {
    let mut fields = op.fingerprint_fields();
    fields.sort_by(|a, b| a.0.cmp(b.0));
    let joined = fields
        .into_iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");
    sha256_hex_bytes(format!("{}::{joined}", op.name()).as_bytes())
}

fn idempotency_key(op: &WriteOperation, caller_key: Option<&str>) -> String {
    match caller_key {
        Some(k) if !k.trim().is_empty() => {
            sha256_hex_bytes(format!("{}::{}", op.name(), k).as_bytes())
        }
        _ => fingerprint(op),
    }
}

// --- path construction (decision D5) ---------------------------------------

const MAX_FILE_NAME_CHARS: usize = 100;
const FORBIDDEN_NAME_CHARS: &[char] = &['\\', '/', '?', '#', ':', '*', '"', '<', '>', '|', '%'];

fn is_safe_segment(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Sanitise a user-supplied file name into a single safe path component.
pub fn sanitize_file_name(raw: &str) -> Result<String, WriteError> {
    let base = raw.rsplit(['/', '\\']).next().unwrap_or(raw);
    let cleaned: String = base
        .chars()
        .map(|c| {
            if c.is_control() || FORBIDDEN_NAME_CHARS.contains(&c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let cleaned = cleaned.trim().trim_matches('.').trim().to_string();
    let truncated: String = cleaned.chars().take(MAX_FILE_NAME_CHARS).collect();
    let truncated = truncated.trim_end_matches([' ', '.']).to_string();
    if truncated.is_empty() {
        return Err(WriteError::InvalidRequest(
            "file name is empty after sanitisation".into(),
        ));
    }
    Ok(truncated)
}

/// `{entity_folder}/{document_id}__{sanitised_file_name}`. The folder and the
/// document id must already be opaque keys (letters, digits, `-`, `_`): they
/// come from the system, never from the uploader. Only the file name is
/// user-supplied, and it is sanitised here.
pub fn upload_path(
    entity_folder: &str,
    document_id: &str,
    file_name: &str,
) -> Result<String, WriteError> {
    if !is_safe_segment(entity_folder) {
        return Err(WriteError::InvalidRequest(
            "entity folder must be an opaque key (letters, digits, '-' or '_')".into(),
        ));
    }
    if !is_safe_segment(document_id) {
        return Err(WriteError::InvalidRequest(
            "document id must be an opaque key (letters, digits, '-' or '_')".into(),
        ));
    }
    Ok(format!(
        "{entity_folder}/{document_id}__{}",
        sanitize_file_name(file_name)?
    ))
}

// --- outcome + errors -------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct WriteOutcome {
    pub operation: &'static str,
    pub graph_resource_id: Option<String>,
    pub idempotent_replay: bool,
    pub graph_response: Option<serde_json::Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    #[error("write denied: {0}")]
    PolicyDenied(String),
    #[error("invalid write request: {0}")]
    InvalidRequest(String),
    #[error("idempotency conflict: same key, different request")]
    IdempotencyConflict,
    #[error("SharePoint is not configured for writes (site/drive or write app registration missing)")]
    NotConfigured,
    #[error("sharepoint write failed: {0}")]
    Graph(#[from] GraphError),
    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl WriteError {
    fn from_provider(error: SharePointProviderError) -> Self {
        Self::InvalidRequest(match error {
            SharePointProviderError::InvalidParameter { name, reason } => {
                format!("{name}: {reason}")
            }
            SharePointProviderError::MissingParameter(name) => format!("{name}: missing"),
            SharePointProviderError::UnknownOperation(name) => format!("unknown operation {name}"),
            SharePointProviderError::UnsupportedOperation { name, reason } => {
                format!("{name}: {reason}")
            }
            SharePointProviderError::UnsupportedMutation(reason) => reason.to_string(),
        })
    }
}

// --- G1.7 policy ------------------------------------------------------------

fn policy_check(ctx: &WriteContext, op: &WriteOperation) -> Result<(), WriteError> {
    let allowed: &[&str] = match op {
        WriteOperation::UploadFile { .. } => UPLOAD_ALLOWED_ROLES,
    };
    if !ctx.roles.iter().any(|r| allowed.contains(&r.as_str())) {
        return Err(WriteError::PolicyDenied(format!(
            "actor `{}` (roles: {:?}) may not perform `{}` (scope {})",
            ctx.actor,
            ctx.roles,
            op.name(),
            op.policy_scope()
        )));
    }
    Ok(())
}

// --- seams: ledger, audit, transport, bearer --------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerRecord {
    pub id: String,
    pub idempotency_key: String,
    pub operation: String,
    pub actor: String,
    pub tenant_id: String,
    pub request_fingerprint: String,
    pub status: String, // pending | succeeded | failed
    pub graph_resource_id: Option<String>,
    pub error_code: Option<String>,
    pub subject_id: Option<String>,
}

/// G1.6 idempotency ledger. One row per idempotency key.
#[async_trait]
pub trait WriteLedger: Send + Sync {
    async fn find(&self, idempotency_key: &str) -> anyhow::Result<Option<LedgerRecord>>;
    async fn record_pending(&self, record: LedgerRecord) -> anyhow::Result<String>;
    async fn complete(
        &self,
        attempt_id: &str,
        status: &str,
        graph_resource_id: Option<&str>,
        error_code: Option<&str>,
    ) -> anyhow::Result<()>;
}

/// G1.8 audit event. Carries no file bytes and no file name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteAuditEvent {
    pub action: String,
    pub operation: &'static str,
    pub policy_scope: &'static str,
    pub actor: String,
    pub roles: Vec<String>,
    pub tenant_id: String,
    pub outcome: &'static str,
    pub graph_resource_id: Option<String>,
    pub subject_id: Option<String>,
    pub reason: Option<String>,
}

#[async_trait]
pub trait WriteAuditSink: Send + Sync {
    async fn record(&self, event: WriteAuditEvent);
}

/// The one place bytes leave for Graph.
#[async_trait]
pub trait UploadTransport: Send + Sync {
    async fn put(
        &self,
        bearer: &str,
        plan: &RequestPlan,
        content_type: &str,
        body: &[u8],
    ) -> Result<serde_json::Value, WriteError>;
}

/// G1.3 token source for the write-scoped app registration.
#[async_trait]
pub trait BearerSource: Send + Sync {
    /// `Ok(None)` means "not configured" (fail closed), distinct from a token
    /// acquisition error.
    async fn bearer(&self) -> anyhow::Result<Option<String>>;
}

pub struct WriteDeps<'a> {
    pub ledger: &'a dyn WriteLedger,
    pub audit: &'a dyn WriteAuditSink,
    pub transport: &'a dyn UploadTransport,
    pub bearer: &'a dyn BearerSource,
}

// --- production implementations of the network-facing seams -----------------

/// Raw-bytes PUT to Graph. Unlike a JSON mutation helper, this sends the file
/// body with the caller-supplied content type.
pub struct GraphUploadTransport {
    http: reqwest::Client,
}

impl GraphUploadTransport {
    pub fn new(http: reqwest::Client) -> Self {
        Self { http }
    }
}

#[async_trait]
impl UploadTransport for GraphUploadTransport {
    async fn put(
        &self,
        bearer: &str,
        plan: &RequestPlan,
        content_type: &str,
        body: &[u8],
    ) -> Result<serde_json::Value, WriteError> {
        let url = format!("{SHAREPOINT_GRAPH_BASE}{}", plan.path);
        tracing::info!(path = %plan.path, method = %plan.method, bytes = body.len(), "outbound SharePoint (Graph) write");
        let mut req = self
            .http
            .request(plan.method.clone(), &url)
            .bearer_auth(bearer)
            .header(reqwest::header::ACCEPT, plan.accept.join(", "))
            .header(reqwest::header::CONTENT_TYPE, content_type)
            .body(body.to_vec());
        for (k, v) in &plan.query {
            req = req.query(&[(k, v)]);
        }
        for (k, v) in &plan.headers {
            req = req.header(*k, v);
        }

        let resp = req.send().await.map_err(|_| {
            WriteError::Graph(GraphError::Unauthorized {
                code: "transport".to_string(),
            })
        })?;
        let status = resp.status();
        let bytes = resp.bytes().await.unwrap_or_default();
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);

        if !status.is_success() {
            tracing::warn!(status = status.as_u16(), "SharePoint (Graph) write returned an error status");
            return Err(WriteError::Graph(classify(status, &value)));
        }
        Ok(redact(value))
    }
}

/// Bearer from the write-scoped Graph app registration (D4). Constructed only
/// when the write contract is fully configured.
pub struct GraphWriteBearer {
    token: Option<GraphToken>,
}

impl GraphWriteBearer {
    pub fn from_env(http: reqwest::Client) -> Self {
        Self {
            token: write_config().map(|(cfg, _target)| GraphToken::new(cfg, http)),
        }
    }
}

#[async_trait]
impl BearerSource for GraphWriteBearer {
    async fn bearer(&self) -> anyhow::Result<Option<String>> {
        match &self.token {
            Some(token) => token.bearer().await.map(Some),
            None => Ok(None),
        }
    }
}

/// True when SharePoint writes are configured at all, so a caller can return a
/// clean "not configured yet" instead of failing mid-pipeline.
pub fn is_configured() -> bool {
    write_config().is_some()
}

// --- non-durable in-memory ledger + audit sink ------------------------------

#[derive(Default)]
pub struct InMemoryWriteLedger {
    rows: Mutex<Vec<LedgerRecord>>,
}

impl InMemoryWriteLedger {
    pub fn rows(&self) -> Vec<LedgerRecord> {
        self.rows.lock().unwrap().clone()
    }
}

#[async_trait]
impl WriteLedger for InMemoryWriteLedger {
    async fn find(&self, key: &str) -> anyhow::Result<Option<LedgerRecord>> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .find(|row| row.idempotency_key == key)
            .cloned())
    }

    async fn record_pending(&self, mut record: LedgerRecord) -> anyhow::Result<String> {
        let mut rows = self.rows.lock().unwrap();
        record.id = format!("attempt-{}", rows.len() + 1);
        let id = record.id.clone();
        rows.push(record);
        Ok(id)
    }

    async fn complete(
        &self,
        attempt_id: &str,
        status: &str,
        graph_resource_id: Option<&str>,
        error_code: Option<&str>,
    ) -> anyhow::Result<()> {
        let mut rows = self.rows.lock().unwrap();
        let row = rows
            .iter_mut()
            .find(|row| row.id == attempt_id)
            .ok_or_else(|| anyhow::anyhow!("write attempt {attempt_id} vanished"))?;
        row.status = status.to_string();
        row.graph_resource_id = graph_resource_id.map(str::to_string);
        row.error_code = error_code.map(str::to_string);
        Ok(())
    }
}

/// Writes audit events to the structured log only. NOT durable evidence.
#[derive(Default)]
pub struct InMemoryAuditSink {
    events: Mutex<Vec<WriteAuditEvent>>,
}

impl InMemoryAuditSink {
    pub fn events(&self) -> Vec<WriteAuditEvent> {
        self.events.lock().unwrap().clone()
    }
}

#[async_trait]
impl WriteAuditSink for InMemoryAuditSink {
    async fn record(&self, event: WriteAuditEvent) {
        tracing::info!(
            action = %event.action,
            actor = %event.actor,
            outcome = event.outcome,
            "sharepoint write audit event"
        );
        self.events.lock().unwrap().push(event);
    }
}

// --- the gate ---------------------------------------------------------------

fn resource_id_from_response(response: &serde_json::Value) -> Option<String> {
    response.get("id").and_then(|v| v.as_str()).map(str::to_string)
}

async fn audit(
    deps: &WriteDeps<'_>,
    ctx: &WriteContext,
    op: &WriteOperation,
    subject_id: Option<&str>,
    outcome: &'static str,
    resource_id: Option<&str>,
    reason: Option<String>,
) {
    deps.audit
        .record(WriteAuditEvent {
            action: format!("SHAREPOINT_WRITE_{}_{outcome}", op.name().to_ascii_uppercase()),
            operation: op.name(),
            policy_scope: op.policy_scope(),
            actor: ctx.actor.clone(),
            roles: ctx.roles.clone(),
            tenant_id: ctx.tenant_id.clone(),
            outcome,
            graph_resource_id: resource_id.map(str::to_string),
            subject_id: subject_id.map(str::to_string),
            reason,
        })
        .await;
}

/// G1.1 GovernedWriteEnforcement - the single entry point for every governed
/// SharePoint write. `body_bytes` is the file; `subject_id` is the record the
/// write is for (for example the `TaxDocument` id), stored on the ledger row.
#[tracing::instrument(
    name = "sharepoint.write",
    skip(deps, ctx, op, body_bytes, subject_id),
    fields(operation = op.name(), actor = %ctx.actor)
)]
pub async fn execute(
    deps: &WriteDeps<'_>,
    ctx: &WriteContext,
    op: WriteOperation,
    body_bytes: &[u8],
    subject_id: Option<&str>,
    caller_idempotency_key: Option<&str>,
) -> Result<WriteOutcome, WriteError> {
    // G1.7 policy
    if let Err(e) = policy_check(ctx, &op) {
        audit(deps, ctx, &op, subject_id, "DENIED", None, Some(e.to_string())).await;
        return Err(e);
    }

    // G1.5 request binding (validates ids, path, 2 MB cap, content type)
    let plan = match op.plan() {
        Ok(plan) => plan,
        Err(e) => {
            audit(deps, ctx, &op, subject_id, "DENIED", None, Some(e.to_string())).await;
            return Err(e);
        }
    };

    // The declared length/hash must describe the bytes actually sent.
    if let WriteOperation::UploadFile {
        content_sha256,
        content_length,
        ..
    } = &op
    {
        if body_bytes.len() as u64 != *content_length
            || body_bytes.len() as u64 > SHAREPOINT_MAX_UPLOAD_BYTES
            || sha256_hex_bytes(body_bytes) != *content_sha256
        {
            let e = WriteError::InvalidRequest(
                "body does not match the declared content length / hash".into(),
            );
            audit(deps, ctx, &op, subject_id, "DENIED", None, Some(e.to_string())).await;
            return Err(e);
        }
    }

    // G1.6 idempotency lookup, before any network call.
    let key = idempotency_key(&op, caller_idempotency_key);
    let print = fingerprint(&op);
    let mut existing_id: Option<String> = None;
    match deps.ledger.find(&key).await {
        Ok(Some(existing)) => {
            if existing.request_fingerprint != print {
                let e = WriteError::IdempotencyConflict;
                audit(deps, ctx, &op, subject_id, "DENIED", None, Some(e.to_string())).await;
                return Err(e);
            }
            if existing.status == "succeeded" {
                return Ok(WriteOutcome {
                    operation: op.name(),
                    graph_resource_id: existing.graph_resource_id,
                    idempotent_replay: true,
                    graph_response: None,
                });
            }
            existing_id = Some(existing.id);
        }
        Ok(None) => {}
        Err(e) => {
            audit(deps, ctx, &op, subject_id, "DENIED", None, Some(e.to_string())).await;
            return Err(WriteError::Internal(e));
        }
    }
    let attempt_id = match existing_id {
        Some(id) => id,
        None => {
            let pending = LedgerRecord {
                id: String::new(),
                idempotency_key: key.clone(),
                operation: op.name().to_string(),
                actor: ctx.actor.clone(),
                tenant_id: ctx.tenant_id.clone(),
                request_fingerprint: print.clone(),
                status: "pending".to_string(),
                graph_resource_id: None,
                error_code: None,
                subject_id: subject_id.map(str::to_string),
            };
            match deps.ledger.record_pending(pending).await {
                Ok(id) => id,
                Err(e) => {
                    audit(deps, ctx, &op, subject_id, "DENIED", None, Some(e.to_string())).await;
                    return Err(WriteError::Internal(e));
                }
            }
        }
    };

    // G1.3 token store isolation: write-scoped registration, no fallback.
    let bearer = match deps.bearer.bearer().await {
        Ok(Some(token)) => token,
        Ok(None) => {
            let e = WriteError::NotConfigured;
            let _ = deps
                .ledger
                .complete(&attempt_id, "failed", None, Some("not_configured"))
                .await;
            audit(deps, ctx, &op, subject_id, "FAILED", None, Some(e.to_string())).await;
            return Err(e);
        }
        Err(e) => {
            let _ = deps
                .ledger
                .complete(&attempt_id, "failed", None, Some("token_acquisition_failed"))
                .await;
            audit(deps, ctx, &op, subject_id, "FAILED", None, Some(e.to_string())).await;
            return Err(WriteError::Internal(e));
        }
    };

    // G1.1: the only call that sends bytes.
    match deps
        .transport
        .put(&bearer, &plan, op.content_type(), body_bytes)
        .await
    {
        Ok(response) => {
            let resource_id = resource_id_from_response(&response);
            let _ = deps
                .ledger
                .complete(&attempt_id, "succeeded", resource_id.as_deref(), None)
                .await;
            audit(deps, ctx, &op, subject_id, "SUCCEEDED", resource_id.as_deref(), None).await;
            Ok(WriteOutcome {
                operation: op.name(),
                graph_resource_id: resource_id,
                idempotent_replay: false,
                graph_response: Some(response),
            })
        }
        Err(e) => {
            let code = match &e {
                WriteError::Graph(g) => Some(g.to_string()),
                _ => None,
            };
            let _ = deps
                .ledger
                .complete(&attempt_id, "failed", None, code.as_deref())
                .await;
            audit(deps, ctx, &op, subject_id, "FAILED", None, Some(e.to_string())).await;
            Err(e)
        }
    }
}

/// Convenience for production wiring: the real transport and write bearer.
pub fn production_seams(
    http: reqwest::Client,
) -> (Arc<GraphUploadTransport>, Arc<GraphWriteBearer>) {
    (
        Arc::new(GraphUploadTransport::new(http.clone())),
        Arc::new(GraphWriteBearer::from_env(http)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const PDF: &[u8] = b"%PDF-1.7 fake pdf bytes";

    fn ctx(role: &str) -> WriteContext {
        WriteContext::new("user@example.com", [role], "t1")
    }

    fn op_for(bytes: &[u8]) -> WriteOperation {
        WriteOperation::upload_file("d1", "ENT-100/doc-1__k1.pdf", "application/pdf", bytes)
    }

    struct FakeTransport {
        calls: AtomicUsize,
        fail_first: AtomicUsize,
        last: Mutex<Option<(String, String, Vec<u8>)>>,
    }

    impl FakeTransport {
        fn ok() -> Self {
            Self {
                calls: AtomicUsize::new(0),
                fail_first: AtomicUsize::new(0),
                last: Mutex::new(None),
            }
        }
        fn failing_first(n: usize) -> Self {
            let t = Self::ok();
            t.fail_first.store(n, Ordering::SeqCst);
            t
        }
        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl UploadTransport for FakeTransport {
        async fn put(
            &self,
            bearer: &str,
            plan: &RequestPlan,
            _content_type: &str,
            body: &[u8],
        ) -> Result<serde_json::Value, WriteError> {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            *self.last.lock().unwrap() =
                Some((bearer.to_string(), plan.path.clone(), body.to_vec()));
            if n < self.fail_first.load(Ordering::SeqCst) {
                return Err(WriteError::Graph(GraphError::Upstream {
                    status: 500,
                    code: "generalException".into(),
                }));
            }
            Ok(serde_json::json!({ "id": "item-1", "name": "k1.pdf" }))
        }
    }

    struct FixedBearer(Option<&'static str>);

    #[async_trait]
    impl BearerSource for FixedBearer {
        async fn bearer(&self) -> anyhow::Result<Option<String>> {
            Ok(self.0.map(str::to_string))
        }
    }

    struct World {
        ledger: InMemoryWriteLedger,
        audit: InMemoryAuditSink,
        transport: FakeTransport,
        bearer: FixedBearer,
    }

    impl World {
        fn new() -> Self {
            Self {
                ledger: InMemoryWriteLedger::default(),
                audit: InMemoryAuditSink::default(),
                transport: FakeTransport::ok(),
                bearer: FixedBearer(Some("write-token")),
            }
        }
        fn deps(&self) -> WriteDeps<'_> {
            WriteDeps {
                ledger: &self.ledger,
                audit: &self.audit,
                transport: &self.transport,
                bearer: &self.bearer,
            }
        }
    }

    #[test]
    fn fingerprint_is_deterministic_and_tracks_content() {
        assert_eq!(fingerprint(&op_for(PDF)), fingerprint(&op_for(PDF)));
        assert_ne!(fingerprint(&op_for(PDF)), fingerprint(&op_for(b"%PDF-other")));
    }

    #[test]
    fn upload_plan_targets_the_content_endpoint_and_fails_on_conflict() {
        let plan = op_for(PDF).plan().unwrap();
        assert_eq!(plan.method, reqwest::Method::PUT);
        assert_eq!(plan.path, "/drives/d1/root:/ENT-100/doc-1__k1.pdf:/content");
        assert_eq!(
            plan.query,
            vec![("@microsoft.graph.conflictBehavior".to_string(), "fail".to_string())]
        );
    }

    #[test]
    fn plan_rejects_non_pdf_oversize_empty_and_traversal() {
        let big = vec![0u8; (SHAREPOINT_MAX_UPLOAD_BYTES + 1) as usize];
        assert!(op_for(&big).plan().is_err());
        assert!(op_for(&[]).plan().is_err());
        assert!(
            WriteOperation::upload_file("d1", "a/b.docx", "application/msword", PDF)
                .plan()
                .is_err()
        );
        assert!(
            WriteOperation::upload_file("d1", "../x.pdf", "application/pdf", PDF)
                .plan()
                .is_err()
        );
        // exactly 2 MB is allowed
        let max = vec![0u8; SHAREPOINT_MAX_UPLOAD_BYTES as usize];
        assert!(op_for(&max).plan().is_ok());
    }

    #[test]
    fn policy_allows_only_the_two_tax_roles() {
        for role in ["tax_staff", "tax_admin", "TAX_ADMIN"] {
            assert!(policy_check(&ctx(role), &op_for(PDF)).is_ok(), "{role}");
        }
        for role in ["viewer", "admin", "project_manager", ""] {
            assert!(policy_check(&ctx(role), &op_for(PDF)).is_err(), "{role}");
        }
    }

    #[tokio::test]
    async fn denied_write_touches_nothing_but_the_audit_log() {
        let w = World::new();
        let err = execute(&w.deps(), &ctx("viewer"), op_for(PDF), PDF, Some("doc-1"), None)
            .await
            .unwrap_err();
        assert!(matches!(err, WriteError::PolicyDenied(_)));
        assert_eq!(w.transport.calls(), 0);
        assert!(w.ledger.rows().is_empty());
        let events = w.audit.events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].action, "SHAREPOINT_WRITE_SHAREPOINT_UPLOAD_FILE_DENIED");
        assert_eq!(events[0].policy_scope, SHAREPOINT_UPLOAD_FILE_POLICY_SCOPE);
    }

    #[tokio::test]
    async fn oversized_file_is_denied_before_ledger_and_network() {
        let w = World::new();
        let big = vec![1u8; (SHAREPOINT_MAX_UPLOAD_BYTES + 1) as usize];
        let err = execute(&w.deps(), &ctx("tax_staff"), op_for(&big), &big, None, None)
            .await
            .unwrap_err();
        assert!(matches!(err, WriteError::InvalidRequest(_)));
        assert_eq!(w.transport.calls(), 0);
        assert!(w.ledger.rows().is_empty());
        assert_eq!(w.audit.events()[0].outcome, "DENIED");
    }

    #[tokio::test]
    async fn body_that_does_not_match_the_declared_hash_is_rejected() {
        let w = World::new();
        let op = op_for(PDF);
        let err = execute(&w.deps(), &ctx("tax_staff"), op, b"%PDF-tampered", None, None)
            .await
            .unwrap_err();
        assert!(matches!(err, WriteError::InvalidRequest(_)));
        assert_eq!(w.transport.calls(), 0);
    }

    #[tokio::test]
    async fn successful_upload_is_ledgered_audited_and_sent_once() {
        let w = World::new();
        let out = execute(&w.deps(), &ctx("tax_staff"), op_for(PDF), PDF, Some("doc-1"), None)
            .await
            .unwrap();
        assert_eq!(out.graph_resource_id.as_deref(), Some("item-1"));
        assert!(!out.idempotent_replay);

        assert_eq!(w.transport.calls(), 1);
        let (bearer, path, body) = w.transport.last.lock().unwrap().clone().unwrap();
        assert_eq!(bearer, "write-token");
        assert_eq!(path, "/drives/d1/root:/ENT-100/doc-1__k1.pdf:/content");
        assert_eq!(body, PDF);

        let rows = w.ledger.rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, "succeeded");
        assert_eq!(rows[0].subject_id.as_deref(), Some("doc-1"));
        let events = w.audit.events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].outcome, "SUCCEEDED");
        assert_eq!(events[0].graph_resource_id.as_deref(), Some("item-1"));
    }

    #[tokio::test]
    async fn repeating_a_succeeded_write_replays_without_a_second_upload() {
        let w = World::new();
        let ctx = ctx("tax_admin");
        execute(&w.deps(), &ctx, op_for(PDF), PDF, None, Some("k")).await.unwrap();
        let again = execute(&w.deps(), &ctx, op_for(PDF), PDF, None, Some("k")).await.unwrap();
        assert!(again.idempotent_replay);
        assert_eq!(again.graph_resource_id.as_deref(), Some("item-1"));
        assert_eq!(w.transport.calls(), 1);
        assert_eq!(w.ledger.rows().len(), 1);
    }

    #[tokio::test]
    async fn same_key_with_different_content_is_a_conflict() {
        let w = World::new();
        let ctx = ctx("tax_staff");
        execute(&w.deps(), &ctx, op_for(PDF), PDF, None, Some("k")).await.unwrap();
        let other = b"%PDF-different";
        let err = execute(&w.deps(), &ctx, op_for(other), other, None, Some("k"))
            .await
            .unwrap_err();
        assert!(matches!(err, WriteError::IdempotencyConflict));
        assert_eq!(w.transport.calls(), 1);
    }

    #[tokio::test]
    async fn failed_upload_is_recorded_and_a_retry_reuses_the_attempt() {
        let mut w = World::new();
        w.transport = FakeTransport::failing_first(1);
        let ctx = ctx("tax_staff");

        let err = execute(&w.deps(), &ctx, op_for(PDF), PDF, None, None).await.unwrap_err();
        assert!(matches!(err, WriteError::Graph(_)));
        assert_eq!(w.ledger.rows()[0].status, "failed");
        assert_eq!(w.audit.events()[0].outcome, "FAILED");

        let ok = execute(&w.deps(), &ctx, op_for(PDF), PDF, None, None).await.unwrap();
        assert!(!ok.idempotent_replay);
        assert_eq!(w.ledger.rows().len(), 1, "retry reuses the same ledger row");
        assert_eq!(w.ledger.rows()[0].status, "succeeded");
        assert_eq!(w.transport.calls(), 2);
    }

    #[tokio::test]
    async fn unconfigured_write_app_fails_closed_without_network() {
        let mut w = World::new();
        w.bearer = FixedBearer(None);
        let err = execute(&w.deps(), &ctx("tax_staff"), op_for(PDF), PDF, None, None)
            .await
            .unwrap_err();
        assert!(matches!(err, WriteError::NotConfigured));
        assert_eq!(w.transport.calls(), 0);
        assert_eq!(w.ledger.rows()[0].status, "failed");
        assert_eq!(w.ledger.rows()[0].error_code.as_deref(), Some("not_configured"));
        assert_eq!(w.audit.events()[0].outcome, "FAILED");
    }

    #[test]
    fn audit_events_carry_no_file_name_or_bytes() {
        // The event type has no path/body field by construction; assert the
        // action string is operation-level only.
        let event = WriteAuditEvent {
            action: "SHAREPOINT_WRITE_SHAREPOINT_UPLOAD_FILE_SUCCEEDED".into(),
            operation: "sharepoint_upload_file",
            policy_scope: SHAREPOINT_UPLOAD_FILE_POLICY_SCOPE,
            actor: "a".into(),
            roles: vec![],
            tenant_id: "t".into(),
            outcome: "SUCCEEDED",
            graph_resource_id: None,
            subject_id: None,
            reason: None,
        };
        assert!(!format!("{event:?}").contains(".pdf"));
    }

    #[test]
    fn upload_path_sanitises_the_file_name_and_pins_system_keys() {
        assert_eq!(
            upload_path("ENT-100", "doc_1", "K-1 (2025).pdf").unwrap(),
            "ENT-100/doc_1__K-1 (2025).pdf"
        );
        assert_eq!(
            upload_path("ENT-100", "doc_1", "..\\..\\evil:name?.pdf").unwrap(),
            "ENT-100/doc_1__evil_name_.pdf"
        );
        assert!(upload_path("ENT/100", "doc_1", "a.pdf").is_err());
        assert!(upload_path("ENT-100", "../doc", "a.pdf").is_err());
        assert!(upload_path("ENT-100", "doc_1", "...").is_err());
        assert!(upload_path("", "doc_1", "a.pdf").is_err());
    }

    #[test]
    fn long_file_names_are_truncated() {
        let long = format!("{}.pdf", "a".repeat(300));
        assert!(sanitize_file_name(&long).unwrap().chars().count() <= MAX_FILE_NAME_CHARS);
    }
}
