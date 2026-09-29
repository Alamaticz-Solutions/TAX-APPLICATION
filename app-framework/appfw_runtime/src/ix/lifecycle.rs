use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, MutexGuard,
    },
};

#[cfg(feature = "chat")]
use std::io::{self, Write};

use chrono::Utc;
#[cfg(feature = "chat")]
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use tokio::sync::watch;
use uuid::Uuid;

use crate::{
    extension::{RuntimePrincipalType, UserAuth},
    observability::RequestContext,
};

use super::contract::{
    audit_event, cursor, runtime_actor, validate_detail, validate_json_shape, validate_key,
    validate_policy_authority, validate_principal_binding, IxArtifactBinding, IxArtifactRevision,
    IxArtifactState, IxAuditEvent, IxAuditEventKind, IxAuditSink, IxCancelDisposition,
    IxCancelReceipt, IxCancellationFence, IxCancellationStage, IxCommitEnvelope, IxContextClaims,
    IxContextProposal, IxContextRevision, IxContextSummary, IxContractError, IxEvent,
    IxEventPayload, IxExecutionAuthority, IxPhase, IxPolicyVerdict, IxPrincipalBinding,
    IxReplayBatch, IxReplayCheckpoint, IxRunPolicy, IxRunRequest, IxRunSnapshot, IxRunState,
    IxStartReceipt, IxTerminalOutcome, IX_AUDIT_SCHEMA_VERSION, IX_COMMIT_SCHEMA_VERSION,
    IX_EVENT_SCHEMA_VERSION, IX_MAX_ACTIVE_RUNS, IX_MAX_ACTIVE_RUNS_PER_OWNER, IX_MAX_COMMITS,
    IX_MAX_EVENTS_PER_COMMIT, IX_MAX_PUBLIC_INTEGER, IX_MAX_REPLAY_BYTES, IX_MAX_RETAINED_RUNS,
    IX_REPLAY_CHECKPOINT_SCHEMA_VERSION, IX_REPLAY_SCHEMA_VERSION, IX_SNAPSHOT_SCHEMA_VERSION,
};

const IX_CANCEL_COMMIT_RESERVE: usize = 5;
const IX_EFFECT_COMMIT_LIMIT: usize = IX_MAX_COMMITS - IX_CANCEL_COMMIT_RESERVE;
const IX_TERMINAL_BYTE_RESERVE: usize = 64 * 1024;
const IX_EFFECT_BYTE_LIMIT: usize = IX_MAX_REPLAY_BYTES - IX_TERMINAL_BYTE_RESERVE;
const IX_MAX_RETAINED_BYTES_PER_RUN: usize = 8 * 1024 * 1024;
const IX_MAX_RUNTIME_RETAINED_BYTES: usize = 512 * 1024 * 1024;
const IX_RETAINED_COMMIT_OVERHEAD_BYTES: usize = 16 * 1024;
pub const IX_STORED_RUN_SCHEMA_VERSION: &str = "appfw.ix_stored_run@1";

#[derive(Clone, Copy)]
enum IxCommitClass {
    Effect,
    ReservedTerminal,
}

#[derive(Clone)]
pub struct IxForegroundRuntime {
    audit: Arc<dyn IxAuditSink>,
    runs: Arc<Mutex<HashMap<String, IxRunRecord>>>,
    policy_authority: IxPrincipalBinding,
    policy: Arc<dyn IxContextPolicy>,
    retention: Arc<IxRuntimeRetentionBudget>,
}

pub trait IxContextPolicy: Send + Sync {
    fn decide_initial(&self, claims: &IxContextClaims) -> Result<IxPolicyVerdict, IxContractError>;

    fn decide_reconciliation(
        &self,
        current: &IxContextRevision,
        proposal: &IxContextProposal,
    ) -> Result<IxPolicyVerdict, IxContractError>;
}

/// Canonical durable state loaded by a Product-owned repository adapter.
///
/// The repository persists these runtime-produced values; it does not rebuild
/// events or derive a competing lifecycle projection. A Framework-owned,
/// key-identified HMAC seal rejects adapter-authored or mutated records: only
/// an `IxRuntimeService` constructed with the same explicit
/// `IxStoredRunSealKey` identity and bytes can authenticate a stored record,
/// including a fresh service instance rehydrating after process loss.
/// Repository JSON decoding performs bounds, shape, replay, and
/// cancellation-structure checks only; it never implies MAC acceptance.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IxStoredRun {
    schema_version: String,
    repository_version: u64,
    seed_snapshot: IxRunSnapshot,
    snapshot: IxRunSnapshot,
    envelopes: Vec<IxCommitEnvelope>,
    snapshots: Vec<IxRunSnapshot>,
    checkpoint: IxReplayCheckpoint,
    start_audit_fact: IxAuditEvent,
    start_idempotency_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    accepted_cancel: Option<IxAcceptedCancel>,
    runtime_seal_key_id: String,
    runtime_seal: String,
}

#[cfg(feature = "chat")]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct IxStoredRunSealMaterial<'a> {
    schema_version: &'a str,
    repository_version: u64,
    seed_snapshot: &'a IxRunSnapshot,
    snapshot: &'a IxRunSnapshot,
    envelopes: &'a [IxCommitEnvelope],
    snapshots: &'a [IxRunSnapshot],
    checkpoint: &'a IxReplayCheckpoint,
    start_audit_fact: &'a IxAuditEvent,
    start_idempotency_key: &'a str,
    accepted_cancel: &'a Option<IxAcceptedCancel>,
    runtime_seal_key_id: &'a str,
}

impl IxStoredRun {
    pub fn schema_version(&self) -> &str {
        &self.schema_version
    }

    pub fn repository_version(&self) -> u64 {
        self.repository_version
    }

    pub fn seed_snapshot(&self) -> &IxRunSnapshot {
        &self.seed_snapshot
    }

    pub fn snapshot(&self) -> &IxRunSnapshot {
        &self.snapshot
    }

    pub fn envelopes(&self) -> &[IxCommitEnvelope] {
        &self.envelopes
    }

    pub fn snapshots(&self) -> &[IxRunSnapshot] {
        &self.snapshots
    }

    pub fn checkpoint(&self) -> &IxReplayCheckpoint {
        &self.checkpoint
    }

    pub fn start_audit_fact(&self) -> &IxAuditEvent {
        &self.start_audit_fact
    }

    pub fn start_idempotency_key(&self) -> &str {
        &self.start_idempotency_key
    }

    pub fn accepted_cancel(&self) -> Option<&IxAcceptedCancel> {
        self.accepted_cancel.as_ref()
    }

    /// Non-secret identity of the Framework seal key that signed this record.
    pub fn runtime_seal_key_id(&self) -> &str {
        &self.runtime_seal_key_id
    }

    pub fn to_json_vec(&self) -> Result<Vec<u8>, IxContractError> {
        self.validate()?;
        canonical_json_bytes(self)
    }

    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, IxContractError> {
        if bytes.len() > IX_MAX_RETAINED_BYTES_PER_RUN {
            return Err(IxContractError::ResourceLimitExceeded);
        }
        let value: Self =
            serde_json::from_slice(bytes).map_err(|_| IxContractError::InvalidLifecycle)?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), IxContractError> {
        if self.schema_version != IX_STORED_RUN_SCHEMA_VERSION
            || self.repository_version == 0
            || self.repository_version > IX_MAX_PUBLIC_INTEGER
            || usize::try_from(self.repository_version).ok() != Some(self.envelopes.len())
            || self.envelopes.is_empty()
            || self.snapshots.len() != self.envelopes.len() + 1
            || self.snapshots.first() != Some(&self.seed_snapshot)
            || self.snapshots.last() != Some(&self.snapshot)
            || self.seed_snapshot.task_revision != 0
            || self.seed_snapshot.request.question.is_some()
            || self.start_audit_fact.schema_version != IX_AUDIT_SCHEMA_VERSION
            || self.start_audit_fact.event_kind != IxAuditEventKind::RunRequested
            || self.start_audit_fact.run_id != self.snapshot.run_id
            || self.start_audit_fact.principal != self.snapshot.owner
            || self.start_audit_fact.intent_key.as_deref()
                != Some(self.snapshot.request.intent_key.as_str())
            || self.start_audit_fact.cancel_disposition.is_some()
            || !canonical_uuid(&self.start_audit_fact.request_id)
            || !canonical_uuid(&self.start_audit_fact.correlation_id)
            || self.start_idempotency_key != format!("ix-start:{}", self.snapshot.run_id)
            || !self.runtime_seal.starts_with(IX_STORED_RUN_SEAL_PREFIX)
            || self.runtime_seal.len() != IX_STORED_RUN_SEAL_PREFIX.len() + 64
            || !self.runtime_seal[IX_STORED_RUN_SEAL_PREFIX.len()..]
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        {
            return Err(IxContractError::InvalidLifecycle);
        }
        // Shape-only checks: records rejected here never reach MAC
        // verification, and passing here never implies MAC acceptance. Only
        // the service-owned sealer authenticates a record.
        validate_key("repository.runtimeSealKeyId", &self.runtime_seal_key_id)
            .map_err(|_| IxContractError::InvalidLifecycle)?;
        validate_key(
            "repository.startIdempotencyKey",
            &self.start_idempotency_key,
        )?;
        let replay = IxReplayBatch {
            schema_version: IX_REPLAY_SCHEMA_VERSION.to_string(),
            base_snapshot: self.seed_snapshot.clone(),
            envelopes: self.envelopes.clone(),
            expected_snapshot: self.snapshot.clone(),
        };
        IxReplayVerifier::verify(&replay, &self.checkpoint)?;
        for (index, envelope) in self.envelopes.iter().enumerate() {
            if self.snapshots.get(index + 1)
                != Some(&IxReplayVerifier::apply(&self.snapshots[index], envelope)?)
            {
                return Err(IxContractError::DigestMismatch);
            }
        }
        match (&self.accepted_cancel, &self.snapshot.cancellation) {
            (None, None) => {}
            (Some(cancel), Some(fence)) => {
                cancel.validate(&self.snapshot.owner, &self.snapshot.run_id)?;
                if cancel.command_id != fence.command_id
                    || cancel.cursor != fence.accepted_cursor
                    || cancel.revision != fence.accepted_revision
                    || !self.envelopes.iter().any(|envelope| {
                        envelope.commit_id == cancel.commit_id
                            && envelope.result_cursor == cancel.cursor
                            && envelope.events.iter().any(|event| {
                                matches!(
                                    &event.payload,
                                    IxEventPayload::CancelRequested { command_id, .. }
                                        if command_id == &cancel.command_id
                                )
                            })
                    })
                {
                    return Err(IxContractError::InvalidCancellationProgress);
                }
            }
            _ => return Err(IxContractError::InvalidCancellationProgress),
        }
        Ok(())
    }

    #[cfg(feature = "chat")]
    fn seal_material(&self) -> IxStoredRunSealMaterial<'_> {
        IxStoredRunSealMaterial {
            schema_version: &self.schema_version,
            repository_version: self.repository_version,
            seed_snapshot: &self.seed_snapshot,
            snapshot: &self.snapshot,
            envelopes: &self.envelopes,
            snapshots: &self.snapshots,
            checkpoint: &self.checkpoint,
            start_audit_fact: &self.start_audit_fact,
            start_idempotency_key: &self.start_idempotency_key,
            accepted_cancel: &self.accepted_cancel,
            runtime_seal_key_id: &self.runtime_seal_key_id,
        }
    }

    #[cfg(feature = "chat")]
    pub(crate) fn replay_after(
        &self,
        owner: &IxPrincipalBinding,
        after_cursor: Option<&str>,
    ) -> Result<(IxReplayBatch, IxReplayCheckpoint), IxContractError> {
        self.validate()?;
        if &self.snapshot.owner != owner {
            return Err(IxContractError::RunNotFound);
        }
        let revision = match after_cursor {
            Some(value) => parse_cursor(value, &self.snapshot.run_id)?,
            None => 0,
        };
        let index = usize::try_from(revision).map_err(|_| IxContractError::InvalidReplayCursor)?;
        let base_snapshot = self
            .snapshots
            .get(index)
            .filter(|snapshot| after_cursor.is_none_or(|value| snapshot.cursor == value))
            .ok_or(IxContractError::InvalidReplayCursor)?
            .clone();
        let envelopes = self
            .envelopes
            .get(index..)
            .ok_or(IxContractError::InvalidReplayCursor)?
            .to_vec();
        let checkpoint = replay_checkpoint_for_range(&base_snapshot, &self.snapshot, &envelopes)?;
        let replay = IxReplayBatch {
            schema_version: IX_REPLAY_SCHEMA_VERSION.to_string(),
            base_snapshot,
            envelopes,
            expected_snapshot: self.snapshot.clone(),
        };
        IxReplayVerifier::verify(&replay, &checkpoint)?;
        Ok((replay, checkpoint))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IxAcceptedCancel {
    command_id: String,
    cursor: String,
    revision: u64,
    commit_id: String,
    audit_projection: IxAuditProjection,
    audit_projection_state: IxAuditProjectionState,
}

impl IxAcceptedCancel {
    pub fn command_id(&self) -> &str {
        &self.command_id
    }

    pub fn cursor(&self) -> &str {
        &self.cursor
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn commit_id(&self) -> &str {
        &self.commit_id
    }

    pub fn audit_projection(&self) -> &IxAuditProjection {
        &self.audit_projection
    }

    pub fn audit_projection_state(&self) -> IxAuditProjectionState {
        self.audit_projection_state
    }

    fn validate(&self, owner: &IxPrincipalBinding, run_id: &str) -> Result<(), IxContractError> {
        validate_key("cancel.commandId", &self.command_id)?;
        validate_key("cancel.cursor", &self.cursor)?;
        validate_key("cancel.commitId", &self.commit_id)?;
        if self.revision == 0
            || self.cursor != cursor(run_id, self.revision)
            || self.audit_projection.tenant_id != owner.tenant_id
            || self.audit_projection.run_id != run_id
            || self.audit_projection.command_id != self.command_id
            || self.audit_projection.cursor != self.cursor
        {
            return Err(IxContractError::InvalidCancellationProgress);
        }
        self.audit_projection.validate()?;
        Ok(())
    }
}

/// One atomic durable run creation. Its fields are runtime-stamped and private
/// so a repository can persist but cannot author lifecycle facts.
#[derive(Clone, Debug)]
pub struct IxRunCreateTransaction {
    stored_run: IxStoredRun,
}

impl IxRunCreateTransaction {
    pub fn stored_run(&self) -> &IxStoredRun {
        &self.stored_run
    }

    pub fn snapshot(&self) -> &IxRunSnapshot {
        self.stored_run.snapshot()
    }

    pub fn acknowledgement(&self) -> &IxCommitEnvelope {
        &self.stored_run.envelopes()[0]
    }

    pub fn audit_fact(&self) -> &IxAuditEvent {
        self.stored_run.start_audit_fact()
    }

    pub fn idempotency_key(&self) -> &str {
        self.stored_run.start_idempotency_key()
    }

    #[cfg(feature = "chat")]
    pub(crate) fn from_runtime(stored_run: IxStoredRun) -> Result<Self, IxContractError> {
        stored_run.validate()?;
        Ok(Self { stored_run })
    }
}

/// One compare-and-append transaction over the canonical cursor.
#[derive(Clone, Debug)]
pub struct IxRunCommitTransaction {
    owner: IxPrincipalBinding,
    expected_repository_version: u64,
    expected_cursor: String,
    envelope: IxCommitEnvelope,
    stored_run: IxStoredRun,
    audit_projection: Option<IxAuditProjection>,
}

impl IxRunCommitTransaction {
    pub fn owner(&self) -> &IxPrincipalBinding {
        &self.owner
    }

    pub fn expected_cursor(&self) -> &str {
        &self.expected_cursor
    }

    pub fn expected_repository_version(&self) -> u64 {
        self.expected_repository_version
    }

    pub fn envelope(&self) -> &IxCommitEnvelope {
        &self.envelope
    }

    pub fn result_snapshot(&self) -> &IxRunSnapshot {
        self.stored_run.snapshot()
    }

    pub fn checkpoint(&self) -> &IxReplayCheckpoint {
        self.stored_run.checkpoint()
    }

    pub fn stored_run(&self) -> &IxStoredRun {
        &self.stored_run
    }

    pub fn audit_projection(&self) -> Option<&IxAuditProjection> {
        self.audit_projection.as_ref()
    }

    #[cfg(feature = "chat")]
    pub(crate) fn from_runtime(
        owner: IxPrincipalBinding,
        expected_repository_version: u64,
        expected_cursor: String,
        envelope: IxCommitEnvelope,
        stored_run: IxStoredRun,
        audit_projection: Option<IxAuditProjection>,
    ) -> Result<Self, IxContractError> {
        stored_run.validate()?;
        let result_snapshot = stored_run.snapshot();
        let checkpoint = stored_run.checkpoint();
        if expected_cursor == envelope.result_cursor
            || expected_repository_version == 0
            || expected_repository_version
                .checked_add(1)
                .filter(|version| *version <= IX_MAX_PUBLIC_INTEGER)
                != Some(stored_run.repository_version)
            || envelope.run_id != result_snapshot.run_id
            || envelope.result_cursor != result_snapshot.cursor
            || owner != result_snapshot.owner
            || checkpoint.run_id != result_snapshot.run_id
            || checkpoint.result_cursor != result_snapshot.cursor
        {
            return Err(IxContractError::InvalidLifecycle);
        }
        Ok(Self {
            owner,
            expected_repository_version,
            expected_cursor,
            envelope,
            stored_run,
            audit_projection,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IxRepositoryCommitReceipt {
    run_id: String,
    cursor: String,
    revision: u64,
    commit_id: String,
    deduplicated: bool,
}

impl IxRepositoryCommitReceipt {
    pub fn new(
        run_id: impl Into<String>,
        cursor: impl Into<String>,
        revision: u64,
        commit_id: impl Into<String>,
        deduplicated: bool,
    ) -> Result<Self, IxContractError> {
        let value = Self {
            run_id: run_id.into(),
            cursor: cursor.into(),
            revision,
            commit_id: commit_id.into(),
            deduplicated,
        };
        validate_key("repository.runId", &value.run_id)?;
        validate_key("repository.cursor", &value.cursor)?;
        validate_key("repository.commitId", &value.commit_id)?;
        if value.revision == 0 || value.revision > IX_MAX_PUBLIC_INTEGER {
            return Err(IxContractError::InvalidLifecycle);
        }
        Ok(value)
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn cursor(&self) -> &str {
        &self.cursor
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn commit_id(&self) -> &str {
        &self.commit_id
    }

    pub fn deduplicated(&self) -> bool {
        self.deduplicated
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IxAuditProjectionState {
    Pending,
    Projected,
    Failed,
}

/// Stable external-audit projection identity for a retained cancellation fact.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IxAuditProjection {
    tenant_id: String,
    run_id: String,
    command_id: String,
    cursor: String,
    dedup_key: String,
}

impl IxAuditProjection {
    pub fn for_cancel(
        tenant_id: impl Into<String>,
        run_id: impl Into<String>,
        command_id: impl Into<String>,
        cursor: impl Into<String>,
    ) -> Result<Self, IxContractError> {
        let tenant_id = tenant_id.into();
        let run_id = run_id.into();
        let command_id = command_id.into();
        let cursor = cursor.into();
        for (field, value) in [
            ("auditProjection.tenantId", tenant_id.as_str()),
            ("auditProjection.runId", run_id.as_str()),
            ("auditProjection.commandId", command_id.as_str()),
            ("auditProjection.cursor", cursor.as_str()),
        ] {
            validate_key(field, value)?;
        }
        let dedup_key = cancel_projection_dedup_key(&tenant_id, &run_id, &command_id, &cursor)?;
        Ok(Self {
            tenant_id,
            run_id,
            command_id,
            cursor,
            dedup_key,
        })
    }

    fn validate(&self) -> Result<(), IxContractError> {
        for (field, value) in [
            ("auditProjection.tenantId", self.tenant_id.as_str()),
            ("auditProjection.runId", self.run_id.as_str()),
            ("auditProjection.commandId", self.command_id.as_str()),
            ("auditProjection.cursor", self.cursor.as_str()),
        ] {
            validate_key(field, value)?;
        }
        if self.dedup_key
            != cancel_projection_dedup_key(
                &self.tenant_id,
                &self.run_id,
                &self.command_id,
                &self.cursor,
            )?
        {
            return Err(IxContractError::InvalidCancellationProgress);
        }
        Ok(())
    }

    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn command_id(&self) -> &str {
        &self.command_id
    }

    pub fn cursor(&self) -> &str {
        &self.cursor
    }

    pub fn dedup_key(&self) -> &str {
        &self.dedup_key
    }
}

fn cancel_projection_dedup_key(
    tenant_id: &str,
    run_id: &str,
    command_id: &str,
    cursor: &str,
) -> Result<String, IxContractError> {
    let mut digest = Sha256::new();
    digest.update(b"appfw.ix_cancel_audit_dedup@1");
    for value in [tenant_id, run_id, command_id, cursor] {
        let length =
            u32::try_from(value.len()).map_err(|_| IxContractError::ResourceLimitExceeded)?;
        digest.update(length.to_be_bytes());
        digest.update(value.as_bytes());
    }
    Ok(format!("sha256:{:x}", digest.finalize()))
}

/// Product persistence seam. Implementations must compare-and-swap on the
/// canonical cursor and persist each supplied transaction atomically.
#[async_trait::async_trait]
pub trait IxRunRepository: Send + Sync + 'static {
    async fn create_run(
        &self,
        transaction: IxRunCreateTransaction,
    ) -> Result<IxRepositoryCommitReceipt, IxContractError>;

    async fn load_run(
        &self,
        owner: &IxPrincipalBinding,
        run_id: &str,
    ) -> Result<Option<IxStoredRun>, IxContractError>;

    async fn compare_and_append(
        &self,
        transaction: IxRunCommitTransaction,
    ) -> Result<IxRepositoryCommitReceipt, IxContractError>;

    async fn mark_audit_projection(
        &self,
        projection: &IxAuditProjection,
        state: IxAuditProjectionState,
    ) -> Result<(), IxContractError>;
}

struct IxRuntimeRetentionBudget {
    retained_bytes: AtomicUsize,
    reserved_terminal_bytes: AtomicUsize,
    limit: AtomicUsize,
}

#[derive(Clone)]
struct IxRunRecord {
    snapshot: IxRunSnapshot,
    envelopes: Vec<IxCommitEnvelope>,
    snapshots: Vec<IxRunSnapshot>,
    cancellation: watch::Sender<bool>,
    committed_envelope_bytes: usize,
    retained_bytes: usize,
    reserved_terminal_bytes: usize,
}

#[cfg(feature = "chat")]
fn record_from_stored(stored: &IxStoredRun) -> Result<IxRunRecord, IxContractError> {
    stored.validate()?;
    let (cancellation, _) = watch::channel(stored.snapshot.cancellation.is_some());
    let committed_envelope_bytes =
        stored
            .envelopes
            .iter()
            .try_fold(0_usize, |total, envelope| {
                total
                    .checked_add(canonical_json_bytes(envelope)?.len())
                    .ok_or(IxContractError::ResourceLimitExceeded)
            })?;
    let mut record = IxRunRecord {
        snapshot: stored.snapshot.clone(),
        envelopes: stored.envelopes.clone(),
        snapshots: stored.snapshots.clone(),
        cancellation,
        committed_envelope_bytes,
        retained_bytes: 0,
        reserved_terminal_bytes: 0,
    };
    record.retained_bytes = record_retained_bytes(&record)?;
    record.reserved_terminal_bytes = terminal_retained_reserve(&record.snapshot)?;
    Ok(record)
}

#[cfg(feature = "chat")]
fn stored_from_record(
    record: &IxRunRecord,
    repository_version: u64,
    start_audit_fact: IxAuditEvent,
    start_idempotency_key: String,
    accepted_cancel: Option<IxAcceptedCancel>,
    sealer: &IxStoredRunSealer,
) -> Result<IxStoredRun, IxContractError> {
    let seed_snapshot = record
        .snapshots
        .first()
        .cloned()
        .ok_or(IxContractError::InvalidLifecycle)?;
    let checkpoint =
        replay_checkpoint_for_range(&seed_snapshot, &record.snapshot, &record.envelopes)?;
    let mut stored = IxStoredRun {
        schema_version: IX_STORED_RUN_SCHEMA_VERSION.to_string(),
        repository_version,
        seed_snapshot,
        snapshot: record.snapshot.clone(),
        envelopes: record.envelopes.clone(),
        snapshots: record.snapshots.clone(),
        checkpoint,
        start_audit_fact,
        start_idempotency_key,
        accepted_cancel,
        runtime_seal_key_id: String::new(),
        runtime_seal: String::new(),
    };
    sealer.seal(&mut stored)?;
    stored.validate()?;
    if canonical_json_bytes(&stored)?.len() > IX_MAX_RETAINED_BYTES_PER_RUN {
        return Err(IxContractError::ResourceLimitExceeded);
    }
    Ok(stored)
}

#[cfg(feature = "chat")]
const IX_STORED_RUN_SEAL_DOMAIN: &[u8] = b"appfw.ix_stored_run.seal@1\0";
const IX_STORED_RUN_SEAL_PREFIX: &str = "hmac-sha256:";

#[cfg(feature = "chat")]
type IxSealHmac = Hmac<Sha256>;

/// Explicit durable seal key consumed when composing an `IxRuntimeService`.
///
/// The secret is write-only composition input: the type has no `Debug`,
/// `Clone`, serde, or secret accessor surface, rejects an all-zero secret,
/// and is never derived, generated, defaulted, serialized, or logged by the
/// Framework. Only the non-secret key identity stays readable.
#[cfg(feature = "chat")]
pub struct IxStoredRunSealKey {
    key_id: String,
    key_bytes: [u8; 32],
}

#[cfg(feature = "chat")]
impl IxStoredRunSealKey {
    pub fn new(key_id: impl Into<String>, key_bytes: [u8; 32]) -> Result<Self, IxContractError> {
        let key_id = key_id.into();
        validate_key("storedRunSealKey.keyId", &key_id)?;
        if key_bytes == [0_u8; 32] {
            return Err(IxContractError::InvalidLifecycle);
        }
        Ok(Self { key_id, key_bytes })
    }

    pub fn key_id(&self) -> &str {
        &self.key_id
    }
}

/// Crate-private stored-run sealer owned by the Framework runtime service.
///
/// Repositories and Product code never receive this value, a signing
/// callback, or the key bytes; every signing and verification decision stays
/// inside Framework lifecycle code.
#[cfg(feature = "chat")]
#[derive(Clone)]
pub(crate) struct IxStoredRunSealer {
    key_id: Arc<str>,
    key_bytes: Arc<[u8; 32]>,
}

#[cfg(feature = "chat")]
impl IxStoredRunSealer {
    pub(crate) fn from_key(key: IxStoredRunSealKey) -> Self {
        Self {
            key_id: Arc::from(key.key_id.as_str()),
            key_bytes: Arc::new(key.key_bytes),
        }
    }

    fn seal(&self, record: &mut IxStoredRun) -> Result<(), IxContractError> {
        record.runtime_seal_key_id = self.key_id.as_ref().to_string();
        record.runtime_seal = format!(
            "{IX_STORED_RUN_SEAL_PREFIX}{:x}",
            self.keyed_material_mac(record)?.finalize().into_bytes()
        );
        Ok(())
    }

    pub(crate) fn verify(&self, record: &IxStoredRun) -> Result<(), IxContractError> {
        if record.runtime_seal_key_id != self.key_id.as_ref() {
            return Err(IxContractError::InvalidLifecycle);
        }
        let expected = record
            .runtime_seal
            .strip_prefix(IX_STORED_RUN_SEAL_PREFIX)
            .ok_or(IxContractError::InvalidLifecycle)
            .and_then(decode_seal_hex)?;
        self.keyed_material_mac(record)?
            .verify_slice(&expected)
            .map_err(|_| IxContractError::InvalidLifecycle)
    }

    fn keyed_material_mac(&self, record: &IxStoredRun) -> Result<IxSealHmac, IxContractError> {
        let mut mac = <IxSealHmac as Mac>::new_from_slice(self.key_bytes.as_ref())
            .map_err(|_| IxContractError::InvalidLifecycle)?;
        mac.update(IX_STORED_RUN_SEAL_DOMAIN);
        let material = serde_json::to_value(record.seal_material())
            .map_err(|_| IxContractError::InvalidLifecycle)?;
        write_canonical_json(&mut MacWriter(&mut mac), &material)?;
        Ok(mac)
    }
}

#[cfg(feature = "chat")]
fn decode_seal_hex(value: &str) -> Result<[u8; 32], IxContractError> {
    let bytes = value.as_bytes();
    if bytes.len() != 64 {
        return Err(IxContractError::InvalidLifecycle);
    }
    let mut decoded = [0_u8; 32];
    for (index, pair) in bytes.chunks_exact(2).enumerate() {
        decoded[index] = (seal_hex_nibble(pair[0])? << 4) | seal_hex_nibble(pair[1])?;
    }
    Ok(decoded)
}

#[cfg(feature = "chat")]
fn seal_hex_nibble(byte: u8) -> Result<u8, IxContractError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err(IxContractError::InvalidLifecycle),
    }
}

#[cfg(feature = "chat")]
fn write_canonical_json(writer: &mut impl Write, value: &Value) -> Result<(), IxContractError> {
    let write = |writer: &mut dyn Write, bytes: &[u8]| {
        writer
            .write_all(bytes)
            .map_err(|_| IxContractError::InvalidLifecycle)
    };
    match value {
        Value::Null => write(writer, b"null"),
        Value::Bool(true) => write(writer, b"true"),
        Value::Bool(false) => write(writer, b"false"),
        Value::Number(number) => write(writer, number.to_string().as_bytes()),
        Value::String(text) => {
            serde_json::to_writer(writer, text).map_err(|_| IxContractError::InvalidLifecycle)
        }
        Value::Array(values) => {
            write(writer, b"[")?;
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    write(writer, b",")?;
                }
                write_canonical_json(writer, value)?;
            }
            write(writer, b"]")
        }
        Value::Object(values) => {
            write(writer, b"{")?;
            let mut keys = values.keys().collect::<Vec<_>>();
            keys.sort_unstable();
            for (index, key) in keys.into_iter().enumerate() {
                if index > 0 {
                    write(writer, b",")?;
                }
                serde_json::to_writer(&mut *writer, key)
                    .map_err(|_| IxContractError::InvalidLifecycle)?;
                write(writer, b":")?;
                write_canonical_json(writer, &values[key])?;
            }
            write(writer, b"}")
        }
    }
}

#[cfg(feature = "chat")]
struct MacWriter<'a>(&'a mut IxSealHmac);

#[cfg(feature = "chat")]
impl Write for MacWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(feature = "chat")]
pub(crate) fn prepare_durable_start(
    request_context: &RequestContext,
    owner: IxPrincipalBinding,
    initial_claims: IxContextClaims,
    request: &IxRunRequest,
    policy: &IxRunPolicy,
    context_policy: &dyn IxContextPolicy,
    policy_authority: &IxPrincipalBinding,
    sealer: &IxStoredRunSealer,
) -> Result<IxRunCreateTransaction, IxContractError> {
    request.validate()?;
    policy.validate_for_request(request)?;
    initial_claims.validate()?;
    validate_principal_binding("durable.owner", &owner)?;
    validate_principal_binding("durable.policyAuthority", policy_authority)?;
    if owner.principal_type != RuntimePrincipalType::User
        || initial_claims.principal() != &owner
        || owner.tenant_id != policy.runner.tenant_id
        || policy_authority.principal_type != RuntimePrincipalType::Service
    {
        return Err(IxContractError::UnauthorizedActor);
    }
    let verdict = context_policy.decide_initial(&initial_claims)?;
    let authorization_context = IxContextRevision::from_policy_verdict(
        1,
        initial_claims,
        policy_authority.clone(),
        verdict,
    )?;
    authorization_context.validate()?;
    if authorization_context.claims().principal() != &owner {
        return Err(IxContractError::UnauthorizedActor);
    }
    let run_id = Uuid::new_v4().to_string();
    let mut retained_request = request.clone();
    retained_request.question = None;
    let seed = IxRunSnapshot {
        schema_version: IX_SNAPSHOT_SCHEMA_VERSION.to_string(),
        run_id: run_id.clone(),
        release_id: authorization_context.release_id().to_string(),
        owner: owner.clone(),
        runner: policy.runner.clone(),
        request: retained_request,
        registered_artifacts: policy.registered_artifacts.clone(),
        recipe_registration: policy.recipe_registration.clone(),
        authorization_context,
        pending_context_proposal: None,
        state: IxRunState::Pending,
        phase: None,
        context: None,
        context_authority: None,
        artifacts: Default::default(),
        cancellation: None,
        terminal_outcome: None,
        task_revision: 0,
        last_sequence: 0,
        cursor: cursor(&run_id, 0),
        next_safe_move: "Acknowledge the requested intelligent work.".to_string(),
    };
    let (cancellation, _) = watch::channel(false);
    let mut record = IxRunRecord {
        snapshot: seed.clone(),
        envelopes: Vec::new(),
        snapshots: vec![seed],
        cancellation,
        committed_envelope_bytes: 0,
        retained_bytes: 0,
        reserved_terminal_bytes: 0,
    };
    record.retained_bytes = record_retained_bytes(&record)?;
    record.reserved_terminal_bytes = terminal_retained_reserve(&record.snapshot)?;
    commit(
        &mut record,
        vec![(
            runtime_actor(),
            IxEventPayload::RunAcknowledged {
                intent_label: policy.intent_label.clone(),
                focus: request.focus.clone(),
            },
        )],
        "Resolve the authorized context for this run.",
        IxCommitClass::Effect,
        None,
    )?;
    let audit_fact = IxAuditEvent {
        schema_version: IX_AUDIT_SCHEMA_VERSION.to_string(),
        event_kind: IxAuditEventKind::RunRequested,
        request_id: request_context.request_id.clone(),
        correlation_id: request_context.correlation_id.clone(),
        principal: owner,
        run_id: run_id.clone(),
        intent_key: Some(request.intent_key.clone()),
        cancel_disposition: None,
    };
    let stored = stored_from_record(
        &record,
        1,
        audit_fact,
        format!("ix-start:{run_id}"),
        None,
        sealer,
    )?;
    IxRunCreateTransaction::from_runtime(stored)
}

#[cfg(feature = "chat")]
pub(crate) fn prepare_durable_append(
    stored: &IxStoredRun,
    actor: IxPrincipalBinding,
    payload: IxEventPayload,
    next_safe_move: &str,
    terminal: bool,
    sealer: &IxStoredRunSealer,
) -> Result<IxRunCommitTransaction, IxContractError> {
    sealer.verify(stored)?;
    let mut record = record_from_stored(stored)?;
    let expected_cursor = record.snapshot.cursor.clone();
    let envelope = commit(
        &mut record,
        vec![(actor, payload)],
        next_safe_move,
        if terminal {
            IxCommitClass::ReservedTerminal
        } else {
            IxCommitClass::Effect
        },
        None,
    )?;
    let next_version = stored
        .repository_version
        .checked_add(1)
        .filter(|version| *version <= IX_MAX_PUBLIC_INTEGER)
        .ok_or(IxContractError::ResourceLimitExceeded)?;
    let next = stored_from_record(
        &record,
        next_version,
        stored.start_audit_fact.clone(),
        stored.start_idempotency_key.clone(),
        stored.accepted_cancel.clone(),
        sealer,
    )?;
    IxRunCommitTransaction::from_runtime(
        record.snapshot.owner.clone(),
        stored.repository_version,
        expected_cursor,
        envelope,
        next,
        None,
    )
}

#[cfg(feature = "chat")]
pub(crate) enum IxDurableCancelPreparation {
    Accepted {
        transaction: Box<IxRunCommitTransaction>,
        accepted: IxAcceptedCancel,
    },
    Retry(IxAcceptedCancel),
    AlreadyTerminal {
        cursor: String,
    },
}

#[cfg(feature = "chat")]
pub(crate) fn prepare_durable_cancel(
    stored: &IxStoredRun,
    owner: &IxPrincipalBinding,
    command_id: &str,
    sealer: &IxStoredRunSealer,
) -> Result<IxDurableCancelPreparation, IxContractError> {
    stored.validate()?;
    sealer.verify(stored)?;
    validate_key("commandId", command_id)?;
    if &stored.snapshot.owner != owner {
        return Err(IxContractError::RunNotFound);
    }
    if let Some(accepted) = &stored.accepted_cancel {
        return if accepted.command_id == command_id {
            Ok(IxDurableCancelPreparation::Retry(accepted.clone()))
        } else {
            Err(IxContractError::CancellationConflict)
        };
    }
    if stored.snapshot.state == IxRunState::Terminal {
        return Ok(IxDurableCancelPreparation::AlreadyTerminal {
            cursor: stored.snapshot.cursor.clone(),
        });
    }
    if !matches!(
        stored.snapshot.state,
        IxRunState::Running | IxRunState::WaitingForUser
    ) {
        return Err(IxContractError::InvalidLifecycle);
    }
    let accepted_revision = stored
        .snapshot
        .task_revision
        .checked_add(1)
        .ok_or(IxContractError::ResourceLimitExceeded)?;
    let accepted_cursor = cursor(&stored.snapshot.run_id, accepted_revision);
    let mut record = record_from_stored(stored)?;
    let expected_cursor = record.snapshot.cursor.clone();
    let envelope = commit(
        &mut record,
        vec![(
            owner.clone(),
            IxEventPayload::CancelRequested {
                command_id: command_id.to_string(),
                accepted_cursor: accepted_cursor.clone(),
                accepted_revision,
            },
        )],
        "Allow in-flight work to stop without committing further effects.",
        IxCommitClass::ReservedTerminal,
        None,
    )?;
    let projection = IxAuditProjection::for_cancel(
        &owner.tenant_id,
        &stored.snapshot.run_id,
        command_id,
        &accepted_cursor,
    )?;
    let accepted = IxAcceptedCancel {
        command_id: command_id.to_string(),
        cursor: accepted_cursor,
        revision: accepted_revision,
        commit_id: envelope.commit_id.clone(),
        audit_projection: projection.clone(),
        audit_projection_state: IxAuditProjectionState::Pending,
    };
    let next_version = stored
        .repository_version
        .checked_add(1)
        .filter(|version| *version <= IX_MAX_PUBLIC_INTEGER)
        .ok_or(IxContractError::ResourceLimitExceeded)?;
    let next = stored_from_record(
        &record,
        next_version,
        stored.start_audit_fact.clone(),
        stored.start_idempotency_key.clone(),
        Some(accepted.clone()),
        sealer,
    )?;
    let transaction = IxRunCommitTransaction::from_runtime(
        owner.clone(),
        stored.repository_version,
        expected_cursor,
        envelope,
        next,
        Some(projection),
    )?;
    Ok(IxDurableCancelPreparation::Accepted {
        transaction: Box::new(transaction),
        accepted,
    })
}

#[cfg(feature = "chat")]
pub(crate) fn prepare_durable_cancel_confirmation(
    stored: &IxStoredRun,
    sealer: &IxStoredRunSealer,
) -> Result<IxRunCommitTransaction, IxContractError> {
    stored.validate()?;
    sealer.verify(stored)?;
    let accepted = stored
        .accepted_cancel
        .as_ref()
        .ok_or(IxContractError::InvalidCancellationProgress)?;
    if stored.snapshot.state != IxRunState::Cancelling {
        return Err(IxContractError::InvalidCancellationProgress);
    }
    let stage = stored
        .snapshot
        .cancellation
        .as_ref()
        .ok_or(IxContractError::InvalidCancellationProgress)?
        .stage;
    let mut record = record_from_stored(stored)?;
    let expected_cursor = record.snapshot.cursor.clone();
    let (events, next_safe_move) = match stage {
        IxCancellationStage::Accepted => (
            vec![
                (
                    runtime_actor(),
                    IxEventPayload::CancellationProgress {
                        command_id: accepted.command_id.clone(),
                        stage: IxCancellationStage::Stopping,
                    },
                ),
                (
                    runtime_actor(),
                    IxEventPayload::CancellationProgress {
                        command_id: accepted.command_id.clone(),
                        stage: IxCancellationStage::Draining,
                    },
                ),
                (
                    runtime_actor(),
                    IxEventPayload::CancellationProgress {
                        command_id: accepted.command_id.clone(),
                        stage: IxCancellationStage::Stopped,
                    },
                ),
            ],
            "Confirm the stopped run and retain its terminal outcome.",
        ),
        IxCancellationStage::Stopped => (
            vec![
                (
                    runtime_actor(),
                    IxEventPayload::CancellationProgress {
                        command_id: accepted.command_id.clone(),
                        stage: IxCancellationStage::Confirmed,
                    },
                ),
                (
                    runtime_actor(),
                    IxEventPayload::RunFinished {
                        authority: None,
                        outcome: IxTerminalOutcome::Cancelled,
                        message: "Stopped. Useful work already produced remains available."
                            .to_string(),
                    },
                ),
            ],
            "Review the retained work or begin a new authorized run.",
        ),
        _ => return Err(IxContractError::InvalidCancellationProgress),
    };
    let envelope = commit(
        &mut record,
        events,
        next_safe_move,
        IxCommitClass::ReservedTerminal,
        None,
    )?;
    let next_version = stored
        .repository_version
        .checked_add(1)
        .filter(|version| *version <= IX_MAX_PUBLIC_INTEGER)
        .ok_or(IxContractError::ResourceLimitExceeded)?;
    let next = stored_from_record(
        &record,
        next_version,
        stored.start_audit_fact.clone(),
        stored.start_idempotency_key.clone(),
        Some(accepted.clone()),
        sealer,
    )?;
    IxRunCommitTransaction::from_runtime(
        record.snapshot.owner.clone(),
        stored.repository_version,
        expected_cursor,
        envelope,
        next,
        None,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IxCancellationSignalState {
    Active,
    Requested,
    ControllerClosed,
}

#[derive(Clone)]
pub struct IxCancellationSignal {
    receiver: watch::Receiver<bool>,
}

impl IxCancellationSignal {
    #[cfg(feature = "chat")]
    pub(crate) fn from_receiver(receiver: watch::Receiver<bool>) -> Self {
        Self { receiver }
    }

    pub fn state(&self) -> IxCancellationSignalState {
        if *self.receiver.borrow() {
            IxCancellationSignalState::Requested
        } else if self.receiver.has_changed().is_err() {
            IxCancellationSignalState::ControllerClosed
        } else {
            IxCancellationSignalState::Active
        }
    }

    pub async fn cancelled(&mut self) -> IxCancellationSignalState {
        loop {
            match self.state() {
                IxCancellationSignalState::Active => {}
                state => return state,
            }
            if self.receiver.changed().await.is_err() {
                return IxCancellationSignalState::ControllerClosed;
            }
        }
    }
}

impl IxForegroundRuntime {
    pub fn new(
        audit: Arc<dyn IxAuditSink>,
        policy_authority: &UserAuth,
        policy: Arc<dyn IxContextPolicy>,
    ) -> Result<Self, IxContractError> {
        let policy_authority = validate_policy_authority(policy_authority)?;
        Ok(Self {
            audit,
            runs: Arc::new(Mutex::new(HashMap::new())),
            policy_authority,
            policy,
            retention: Arc::new(IxRuntimeRetentionBudget {
                retained_bytes: AtomicUsize::new(0),
                reserved_terminal_bytes: AtomicUsize::new(0),
                limit: AtomicUsize::new(IX_MAX_RUNTIME_RETAINED_BYTES),
            }),
        })
    }

    /// Starts one foreground run after the metadata-only audit record is
    /// retained. The browser credential is never copied into lifecycle state.
    pub fn start(
        &self,
        request_context: &RequestContext,
        owner: &UserAuth,
        initial_claims: IxContextClaims,
        request: IxRunRequest,
        policy: IxRunPolicy,
    ) -> Result<IxStartReceipt, IxContractError> {
        request.validate()?;
        policy.validate_for_request(&request)?;
        let owner_binding = authenticated_owner_binding(owner)?;
        initial_claims.validate()?;
        if !initial_claims.matches_authenticated_human(owner) {
            return Err(IxContractError::UnauthorizedActor);
        }
        let initial_verdict = self.policy.decide_initial(&initial_claims)?;
        let authorization_context = IxContextRevision::from_policy_verdict(
            1,
            initial_claims,
            self.policy_authority.clone(),
            initial_verdict,
        )?;
        authorization_context.validate()?;
        if owner.tenant_id != policy.runner.tenant_id
            || !authorization_context.matches_principal_claims(owner)
        {
            return Err(IxContractError::UnauthorizedActor);
        }
        ensure_start_capacity(&lock_runs(&self.runs), &owner_binding)?;
        let run_id = Uuid::new_v4().to_string();
        self.audit
            .append(&audit_event(
                IxAuditEventKind::RunRequested,
                request_context,
                owner,
                run_id.clone(),
                Some(request.intent_key.clone()),
                None,
            ))
            .map_err(|_| IxContractError::AuditUnavailable)?;

        let mut retained_request = request.clone();
        retained_request.question = None;
        let seed = IxRunSnapshot {
            schema_version: IX_SNAPSHOT_SCHEMA_VERSION.to_string(),
            run_id: run_id.clone(),
            release_id: authorization_context.release_id().to_string(),
            owner: IxPrincipalBinding::from(owner),
            runner: policy.runner,
            request: retained_request,
            registered_artifacts: policy.registered_artifacts,
            recipe_registration: policy.recipe_registration,
            authorization_context,
            pending_context_proposal: None,
            state: IxRunState::Pending,
            phase: None,
            context: None,
            context_authority: None,
            artifacts: Default::default(),
            cancellation: None,
            terminal_outcome: None,
            task_revision: 0,
            last_sequence: 0,
            cursor: cursor(&run_id, 0),
            next_safe_move: "Acknowledge the requested intelligent work.".to_string(),
        };
        let (cancellation, _) = watch::channel(false);
        let mut record = IxRunRecord {
            snapshot: seed.clone(),
            envelopes: Vec::new(),
            snapshots: vec![seed],
            cancellation,
            committed_envelope_bytes: 0,
            retained_bytes: 0,
            reserved_terminal_bytes: 0,
        };
        record.retained_bytes = record_retained_bytes(&record)?;
        record.reserved_terminal_bytes = terminal_retained_reserve(&record.snapshot)?;
        commit(
            &mut record,
            vec![(
                runtime_actor(),
                IxEventPayload::RunAcknowledged {
                    intent_label: policy.intent_label,
                    focus: request.focus,
                },
            )],
            "Resolve the authorized context for this run.",
            IxCommitClass::Effect,
            None,
        )?;

        let receipt = IxStartReceipt {
            run_id: run_id.clone(),
            cursor: record.snapshot.cursor.clone(),
            task_revision: record.snapshot.task_revision,
        };
        let mut runs = lock_runs(&self.runs);
        ensure_start_capacity(&runs, &owner_binding)?;
        let current_retained = self.retention.retained_bytes.load(Ordering::SeqCst);
        let current_reserved = self
            .retention
            .reserved_terminal_bytes
            .load(Ordering::SeqCst);
        if current_retained
            .checked_add(record.retained_bytes)
            .and_then(|total| total.checked_add(current_reserved))
            .and_then(|total| total.checked_add(record.reserved_terminal_bytes))
            .is_none_or(|total| total > self.retention.limit.load(Ordering::SeqCst))
        {
            return Err(IxContractError::ResourceLimitExceeded);
        }
        self.retention
            .retained_bytes
            .store(current_retained + record.retained_bytes, Ordering::SeqCst);
        self.retention.reserved_terminal_bytes.store(
            current_reserved + record.reserved_terminal_bytes,
            Ordering::SeqCst,
        );
        if runs.insert(run_id, record).is_some() {
            return Err(IxContractError::InvalidLifecycle);
        }
        Ok(receipt)
    }

    pub fn resolve_context(
        &self,
        runner: &UserAuth,
        run_id: &str,
        authority: &IxExecutionAuthority,
        context: IxContextSummary,
    ) -> Result<IxCommitEnvelope, IxContractError> {
        context.validate()?;
        self.commit_as_runner(
            runner,
            run_id,
            IxEventPayload::ContextResolved {
                authority: authority.clone(),
                context,
            },
            "Begin the first meaningful phase of work.",
        )
    }

    pub fn request_context_reconciliation(
        &self,
        owner: &UserAuth,
        run_id: &str,
        proposal: IxContextProposal,
    ) -> Result<IxCommitEnvelope, IxContractError> {
        proposal.validate()?;
        if !proposal.claims().matches_authenticated_human(owner) {
            return Err(IxContractError::UnauthorizedActor);
        }
        let mut runs = lock_runs(&self.runs);
        let record = owned_record_mut(&mut runs, owner, run_id)?;
        commit(
            record,
            vec![(
                IxPrincipalBinding::from(owner),
                IxEventPayload::ContextReconciliationRequested { proposal },
            )],
            "Wait for the configured policy authority to decide the proposed context change.",
            IxCommitClass::Effect,
            Some(&self.retention),
        )
    }

    pub fn reconcile_context(&self, run_id: &str) -> Result<IxCommitEnvelope, IxContractError> {
        let (current, proposal) = {
            let runs = lock_runs(&self.runs);
            let snapshot = &runs
                .get(run_id)
                .ok_or(IxContractError::RunNotFound)?
                .snapshot;
            (
                snapshot.authorization_context.clone(),
                snapshot
                    .pending_context_proposal
                    .clone()
                    .ok_or(IxContractError::InvalidContextRevision)?,
            )
        };
        let verdict = self.policy.decide_reconciliation(&current, &proposal)?;
        let next = IxContextRevision::from_policy_verdict(
            current
                .revision()
                .checked_add(1)
                .ok_or(IxContractError::InvalidContextRevision)?,
            proposal.claims().clone(),
            self.policy_authority.clone(),
            verdict,
        )?;
        let mut runs = lock_runs(&self.runs);
        let record = runs.get_mut(run_id).ok_or(IxContractError::RunNotFound)?;
        if record.snapshot.authorization_context != current
            || record.snapshot.pending_context_proposal.as_ref() != Some(&proposal)
        {
            return Err(IxContractError::StaleAuthorization);
        }
        commit(
            record,
            vec![(
                runtime_actor(),
                IxEventPayload::ContextReconciled {
                    proposal_id: proposal.proposal_id().to_string(),
                    context_revision: next,
                },
            )],
            "Re-resolve authorized context before producing another effect.",
            IxCommitClass::Effect,
            Some(&self.retention),
        )
    }

    pub fn change_phase(
        &self,
        runner: &UserAuth,
        run_id: &str,
        authority: &IxExecutionAuthority,
        phase: IxPhase,
        reason: impl Into<String>,
    ) -> Result<IxCommitEnvelope, IxContractError> {
        if phase == IxPhase::Acknowledged {
            return Err(IxContractError::InvalidLifecycle);
        }
        let reason = reason.into();
        validate_detail("phase.reason", &reason)?;
        self.commit_as_runner(
            runner,
            run_id,
            IxEventPayload::PhaseChanged {
                authority: authority.clone(),
                phase,
                reason,
            },
            "Continue the named work or publish a useful artifact revision.",
        )
    }

    pub fn publish_artifact(
        &self,
        runner: &UserAuth,
        run_id: &str,
        authority: &IxExecutionAuthority,
        artifact: IxArtifactRevision,
    ) -> Result<IxCommitEnvelope, IxContractError> {
        artifact.validate()?;
        self.commit_as_runner(
            runner,
            run_id,
            IxEventPayload::ArtifactRevision {
                authority: authority.clone(),
                artifact,
            },
            "Inspect, continue, redirect, or stop the progressively useful work.",
        )
    }

    pub fn wait_for_user(
        &self,
        runner: &UserAuth,
        run_id: &str,
        authority: &IxExecutionAuthority,
        prompt: impl Into<String>,
    ) -> Result<IxCommitEnvelope, IxContractError> {
        let prompt = prompt.into();
        validate_detail("waiting.prompt", &prompt)?;
        self.commit_as_runner(
            runner,
            run_id,
            IxEventPayload::WaitingForUser {
                authority: authority.clone(),
                prompt,
            },
            "Answer, redirect, or stop this run.",
        )
    }

    /// Human input itself remains product-owned and is not copied into the
    /// framework event log. The accepted input and resumed phase are one
    /// atomic, multi-event commit.
    pub fn resume_after_user_input(
        &self,
        owner: &UserAuth,
        run_id: &str,
        authority: &IxExecutionAuthority,
        phase: IxPhase,
        reason: impl Into<String>,
    ) -> Result<IxCommitEnvelope, IxContractError> {
        if phase == IxPhase::Acknowledged {
            return Err(IxContractError::InvalidLifecycle);
        }
        let reason = reason.into();
        validate_detail("resume.reason", &reason)?;
        let mut runs = lock_runs(&self.runs);
        let record = owned_record_mut(&mut runs, owner, run_id)?;
        commit(
            record,
            vec![
                (
                    IxPrincipalBinding::from(owner),
                    IxEventPayload::UserInputAccepted {
                        authority: authority.clone(),
                    },
                ),
                (
                    runtime_actor(),
                    IxEventPayload::PhaseChanged {
                        authority: authority.clone(),
                        phase,
                        reason,
                    },
                ),
            ],
            "Continue the resumed work and preserve prior useful revisions.",
            IxCommitClass::Effect,
            Some(&self.retention),
        )
    }

    /// Cancellation takes effect before its best-effort audit. An audit outage
    /// can never keep authorized work running.
    pub fn cancel(
        &self,
        request_context: &RequestContext,
        owner: &UserAuth,
        run_id: &str,
        command_id: &str,
    ) -> Result<IxCancelReceipt, IxContractError> {
        validate_key("commandId", command_id)?;
        let (disposition, cursor) = {
            let mut runs = lock_runs(&self.runs);
            let owner_binding = authenticated_owner_binding(owner)?;
            match runs
                .get_mut(run_id)
                .filter(|record| record.snapshot.owner == owner_binding)
            {
                None => (IxCancelDisposition::NotFound, None),
                Some(record) => match record.snapshot.state {
                    IxRunState::Terminal => {
                        if let Some(fence) = record.snapshot.cancellation.as_ref() {
                            if fence.command_id != command_id {
                                return Err(IxContractError::CancellationConflict);
                            }
                        }
                        (
                            IxCancelDisposition::AlreadyTerminal,
                            Some(record.snapshot.cursor.clone()),
                        )
                    }
                    IxRunState::Cancelling => {
                        let fence = record
                            .snapshot
                            .cancellation
                            .as_ref()
                            .ok_or(IxContractError::InvalidLifecycle)?;
                        if fence.command_id != command_id {
                            return Err(IxContractError::CancellationConflict);
                        }
                        (
                            IxCancelDisposition::AlreadyRequested,
                            Some(record.snapshot.cursor.clone()),
                        )
                    }
                    IxRunState::Running | IxRunState::WaitingForUser => {
                        let accepted_revision = record
                            .snapshot
                            .task_revision
                            .checked_add(1)
                            .ok_or(IxContractError::ResourceLimitExceeded)?;
                        let accepted_cursor = cursor(run_id, accepted_revision);
                        let envelope = commit(
                            record,
                            vec![(
                                IxPrincipalBinding::from(owner),
                                IxEventPayload::CancelRequested {
                                    command_id: command_id.to_string(),
                                    accepted_cursor,
                                    accepted_revision,
                                },
                            )],
                            "Allow in-flight work to stop without committing further effects.",
                            IxCommitClass::ReservedTerminal,
                            Some(&self.retention),
                        )?;
                        record.cancellation.send_replace(true);
                        (IxCancelDisposition::Accepted, Some(envelope.result_cursor))
                    }
                    IxRunState::Pending => return Err(IxContractError::InvalidLifecycle),
                },
            }
        };
        Ok(self.audit_cancel(request_context, owner, run_id, disposition, cursor))
    }

    pub fn cancellation_progress(
        &self,
        runner: &UserAuth,
        run_id: &str,
        command_id: &str,
        stage: IxCancellationStage,
    ) -> Result<IxCommitEnvelope, IxContractError> {
        if matches!(
            stage,
            IxCancellationStage::Accepted | IxCancellationStage::Confirmed
        ) {
            return Err(IxContractError::InvalidCancellationProgress);
        }
        let mut runs = lock_runs(&self.runs);
        let record = runner_record_mut(&mut runs, runner, run_id)?;
        commit(
            record,
            vec![(
                runtime_actor(),
                IxEventPayload::CancellationProgress {
                    command_id: command_id.to_string(),
                    stage,
                },
            )],
            "Continue stopping safely; no work result may affect authoritative state.",
            IxCommitClass::ReservedTerminal,
            Some(&self.retention),
        )
    }

    pub fn cancellation_signal(
        &self,
        runner: &UserAuth,
        run_id: &str,
    ) -> Result<IxCancellationSignal, IxContractError> {
        let runs = lock_runs(&self.runs);
        let record = runner_record(&runs, runner, run_id)?;
        Ok(IxCancellationSignal {
            receiver: record.cancellation.subscribe(),
        })
    }

    pub fn confirm_cancelled(
        &self,
        runner: &UserAuth,
        run_id: &str,
        command_id: &str,
    ) -> Result<IxCommitEnvelope, IxContractError> {
        let mut runs = lock_runs(&self.runs);
        let record = runner_record_mut(&mut runs, runner, run_id)?;
        commit(
            record,
            vec![
                (
                    runtime_actor(),
                    IxEventPayload::CancellationProgress {
                        command_id: command_id.to_string(),
                        stage: IxCancellationStage::Confirmed,
                    },
                ),
                (
                    runtime_actor(),
                    IxEventPayload::RunFinished {
                        authority: None,
                        outcome: IxTerminalOutcome::Cancelled,
                        message: "Stopped. Useful work already produced remains available."
                            .to_string(),
                    },
                ),
            ],
            "Review the retained work or begin a new authorized run.",
            IxCommitClass::ReservedTerminal,
            Some(&self.retention),
        )
    }

    pub fn finish(
        &self,
        runner: &UserAuth,
        run_id: &str,
        authority: &IxExecutionAuthority,
        outcome: IxTerminalOutcome,
        message: impl Into<String>,
    ) -> Result<IxCommitEnvelope, IxContractError> {
        if !matches!(
            outcome,
            IxTerminalOutcome::Completed | IxTerminalOutcome::Partial
        ) {
            return Err(IxContractError::InvalidLifecycle);
        }
        let message = message.into();
        validate_detail("terminal.message", &message)?;
        let mut runs = lock_runs(&self.runs);
        let record = runner_record_mut(&mut runs, runner, run_id)?;
        commit(
            record,
            vec![(
                runtime_actor(),
                IxEventPayload::RunFinished {
                    authority: Some(authority.clone()),
                    outcome,
                    message,
                },
            )],
            "Inspect the final work and its supporting context.",
            IxCommitClass::ReservedTerminal,
            Some(&self.retention),
        )
    }

    /// Truthful failure remains possible even when the authorization context
    /// changed or consent was withdrawn. It can report no successful effect.
    pub fn fail(
        &self,
        runner: &UserAuth,
        run_id: &str,
        message: impl Into<String>,
    ) -> Result<IxCommitEnvelope, IxContractError> {
        let message = message.into();
        validate_detail("terminal.message", &message)?;
        let mut runs = lock_runs(&self.runs);
        let record = runner_record_mut(&mut runs, runner, run_id)?;
        commit(
            record,
            vec![(
                runtime_actor(),
                IxEventPayload::RunFinished {
                    authority: None,
                    outcome: IxTerminalOutcome::Failed,
                    message,
                },
            )],
            "Inspect the failure and begin a new authorized run if appropriate.",
            IxCommitClass::ReservedTerminal,
            Some(&self.retention),
        )
    }

    pub fn replay_after(
        &self,
        owner: &UserAuth,
        run_id: &str,
        after_cursor: Option<&str>,
    ) -> Result<IxReplayBatch, IxContractError> {
        let runs = lock_runs(&self.runs);
        let record = owned_record(&runs, owner, run_id)?;
        let (index, base_snapshot) = snapshot_at(record, run_id, after_cursor)?;
        let base_snapshot = base_snapshot.clone();
        let envelopes = record
            .envelopes
            .get(index..)
            .ok_or(IxContractError::InvalidReplayCursor)?
            .to_vec();
        let checkpoint = replay_checkpoint_for_range(&base_snapshot, &record.snapshot, &envelopes)?;
        let batch = IxReplayBatch {
            schema_version: IX_REPLAY_SCHEMA_VERSION.to_string(),
            base_snapshot,
            envelopes,
            expected_snapshot: record.snapshot.clone(),
        };
        IxReplayVerifier::verify(&batch, &checkpoint)?;
        Ok(batch)
    }

    /// Returns the authority binding that must be retained separately from a
    /// replay batch when deterministic evidence is verified later.
    pub fn replay_checkpoint(
        &self,
        owner: &UserAuth,
        run_id: &str,
        at_cursor: Option<&str>,
    ) -> Result<IxReplayCheckpoint, IxContractError> {
        let runs = lock_runs(&self.runs);
        let record = owned_record(&runs, owner, run_id)?;
        let (index, base_snapshot) = snapshot_at(record, run_id, at_cursor)?;
        let envelopes = record
            .envelopes
            .get(index..)
            .ok_or(IxContractError::InvalidReplayCursor)?;
        replay_checkpoint_for_range(base_snapshot, &record.snapshot, envelopes)
    }

    pub fn snapshot(
        &self,
        owner: &UserAuth,
        run_id: &str,
    ) -> Result<IxRunSnapshot, IxContractError> {
        let runs = lock_runs(&self.runs);
        Ok(owned_record(&runs, owner, run_id)?.snapshot.clone())
    }

    fn commit_as_runner(
        &self,
        runner: &UserAuth,
        run_id: &str,
        payload: IxEventPayload,
        next_safe_move: &str,
    ) -> Result<IxCommitEnvelope, IxContractError> {
        let mut runs = lock_runs(&self.runs);
        let record = runner_record_mut(&mut runs, runner, run_id)?;
        commit(
            record,
            vec![(IxPrincipalBinding::from(runner), payload)],
            next_safe_move,
            IxCommitClass::Effect,
            Some(&self.retention),
        )
    }

    fn audit_cancel(
        &self,
        request_context: &RequestContext,
        owner: &UserAuth,
        run_id: &str,
        disposition: IxCancelDisposition,
        cursor: Option<String>,
    ) -> IxCancelReceipt {
        let audit_retained = self
            .audit
            .append(&audit_event(
                IxAuditEventKind::CancelDisposition,
                request_context,
                owner,
                run_id.to_string(),
                None,
                Some(disposition.into()),
            ))
            .is_ok();
        IxCancelReceipt {
            disposition,
            audit_retained,
            cursor,
        }
    }
}

pub struct IxReplayVerifier;

impl IxReplayVerifier {
    pub fn verify(
        batch: &IxReplayBatch,
        checkpoint: &IxReplayCheckpoint,
    ) -> Result<(), IxContractError> {
        if batch.schema_version != IX_REPLAY_SCHEMA_VERSION
            || checkpoint.schema_version != IX_REPLAY_CHECKPOINT_SCHEMA_VERSION
        {
            return Err(IxContractError::UnsupportedSchema);
        }
        if batch.envelopes.len() > IX_MAX_COMMITS
            || batch.envelopes.iter().any(|envelope| {
                envelope.events.is_empty() || envelope.events.len() > IX_MAX_EVENTS_PER_COMMIT
            })
        {
            return Err(IxContractError::ResourceLimitExceeded);
        }
        validate_snapshot_projection(&batch.base_snapshot)?;
        validate_snapshot_projection(&batch.expected_snapshot)?;
        let envelope_count = u64::try_from(batch.envelopes.len())
            .map_err(|_| IxContractError::ResourceLimitExceeded)?;
        let expected_result_revision = batch
            .base_snapshot
            .task_revision
            .checked_add(envelope_count)
            .ok_or(IxContractError::ResourceLimitExceeded)?;
        if batch.base_snapshot.run_id != checkpoint.run_id
            || batch.base_snapshot.owner != checkpoint.owner
            || batch.expected_snapshot.run_id != checkpoint.run_id
            || batch.expected_snapshot.owner != checkpoint.owner
            || batch.base_snapshot.task_revision != checkpoint.base_revision
            || batch.expected_snapshot.task_revision != checkpoint.result_revision
            || expected_result_revision != checkpoint.result_revision
            || batch.base_snapshot.cursor != checkpoint.base_cursor
            || batch.expected_snapshot.cursor != checkpoint.result_cursor
            || snapshot_digest(&batch.base_snapshot)? != checkpoint.base_snapshot_sha256
            || snapshot_digest(&batch.expected_snapshot)? != checkpoint.result_snapshot_sha256
            || envelope_count != checkpoint.envelope_count
            || envelope_chain_sha256(&batch.envelopes)? != checkpoint.envelope_chain_sha256
        {
            return Err(IxContractError::DigestMismatch);
        }
        let mut snapshot = batch.base_snapshot.clone();
        for envelope in &batch.envelopes {
            snapshot = Self::apply(&snapshot, envelope)?;
        }
        if snapshot != batch.expected_snapshot {
            return Err(IxContractError::DigestMismatch);
        }
        Ok(())
    }

    pub fn apply(
        base: &IxRunSnapshot,
        envelope: &IxCommitEnvelope,
    ) -> Result<IxRunSnapshot, IxContractError> {
        validate_snapshot_projection(base)?;
        if envelope.events.is_empty() || envelope.events.len() > IX_MAX_EVENTS_PER_COMMIT {
            return Err(IxContractError::ResourceLimitExceeded);
        }
        if snapshot_digest(base)? != envelope.base_snapshot_sha256 {
            return Err(IxContractError::DigestMismatch);
        }
        let result = apply_without_result_digest(base, envelope)?;
        if snapshot_digest(&result)? != envelope.result_snapshot_sha256 {
            return Err(IxContractError::DigestMismatch);
        }
        Ok(result)
    }
}

fn commit(
    record: &mut IxRunRecord,
    payloads: Vec<(IxPrincipalBinding, IxEventPayload)>,
    next_safe_move: &str,
    class: IxCommitClass,
    runtime_retention: Option<&IxRuntimeRetentionBudget>,
) -> Result<IxCommitEnvelope, IxContractError> {
    validate_detail("nextSafeMove", next_safe_move)?;
    let commit_limit = match class {
        IxCommitClass::Effect => IX_EFFECT_COMMIT_LIMIT,
        IxCommitClass::ReservedTerminal => IX_MAX_COMMITS,
    };
    if record.envelopes.len() >= commit_limit
        || payloads.is_empty()
        || payloads.len() > IX_MAX_EVENTS_PER_COMMIT
    {
        return Err(IxContractError::ResourceLimitExceeded);
    }
    preflight_retained_budget(record, &payloads, next_safe_move, class, runtime_retention)?;
    let base = record.snapshot.clone();
    let result_revision = base
        .task_revision
        .checked_add(1)
        .filter(|value| *value <= IX_MAX_PUBLIC_INTEGER)
        .ok_or(IxContractError::ResourceLimitExceeded)?;
    let result_cursor = cursor(&base.run_id, result_revision);
    let occurred_at = Utc::now().to_rfc3339();
    let events = payloads
        .into_iter()
        .enumerate()
        .map(|(offset, (actor, payload))| {
            let sequence = base
                .last_sequence
                .checked_add(offset as u64)
                .and_then(|value| value.checked_add(1))
                .ok_or(IxContractError::ResourceLimitExceeded)?;
            Ok(IxEvent {
                schema_version: IX_EVENT_SCHEMA_VERSION.to_string(),
                event_id: format!("{}:{sequence}", base.run_id),
                run_id: base.run_id.clone(),
                sequence,
                occurred_at: occurred_at.clone(),
                actor,
                payload,
            })
        })
        .collect::<Result<Vec<_>, IxContractError>>()?;
    let mut envelope = IxCommitEnvelope {
        schema_version: IX_COMMIT_SCHEMA_VERSION.to_string(),
        commit_id: Uuid::new_v4().to_string(),
        committed_by: "appfw-runtime".to_string(),
        run_id: base.run_id.clone(),
        base_cursor: base.cursor.clone(),
        result_cursor,
        base_revision: base.task_revision,
        result_revision,
        events,
        next_safe_move: next_safe_move.to_string(),
        base_snapshot_sha256: snapshot_digest(&base)?,
        result_snapshot_sha256: String::new(),
    };
    let result = apply_without_result_digest(&base, &envelope)?;
    envelope.result_snapshot_sha256 = snapshot_digest(&result)?;
    let envelope_bytes = canonical_json_bytes(&envelope)?.len();
    let byte_limit = match class {
        IxCommitClass::Effect => IX_EFFECT_BYTE_LIMIT,
        IxCommitClass::ReservedTerminal => IX_MAX_REPLAY_BYTES,
    };
    let committed_envelope_bytes = record
        .committed_envelope_bytes
        .checked_add(envelope_bytes)
        .filter(|total| *total <= byte_limit)
        .ok_or(IxContractError::ResourceLimitExceeded)?;
    let verified = IxReplayVerifier::apply(&base, &envelope)?;
    let old_snapshot_bytes = canonical_json_bytes(&record.snapshot)?.len();
    let result_snapshot_bytes = canonical_json_bytes(&verified)?.len();
    let new_retained_bytes = record
        .retained_bytes
        .checked_sub(old_snapshot_bytes)
        .and_then(|bytes| bytes.checked_add(envelope_bytes))
        .and_then(|bytes| {
            result_snapshot_bytes
                .checked_mul(2)
                .and_then(|added| bytes.checked_add(added))
        })
        .ok_or(IxContractError::ResourceLimitExceeded)?;
    let new_reserved_terminal_bytes = if verified.state == IxRunState::Terminal {
        0
    } else {
        terminal_retained_reserve(&verified)?
    };
    enforce_exact_retained_budget(
        record,
        new_retained_bytes,
        new_reserved_terminal_bytes,
        class,
        runtime_retention,
    )?;
    record.snapshot = verified.clone();
    record.envelopes.push(envelope.clone());
    record.snapshots.push(verified);
    record.committed_envelope_bytes = committed_envelope_bytes;
    if let Some(retention) = runtime_retention {
        let other_retained = retention
            .retained_bytes
            .load(Ordering::SeqCst)
            .checked_sub(record.retained_bytes)
            .ok_or(IxContractError::ResourceLimitExceeded)?;
        let other_reserved = retention
            .reserved_terminal_bytes
            .load(Ordering::SeqCst)
            .checked_sub(record.reserved_terminal_bytes)
            .ok_or(IxContractError::ResourceLimitExceeded)?;
        retention
            .retained_bytes
            .store(other_retained + new_retained_bytes, Ordering::SeqCst);
        retention.reserved_terminal_bytes.store(
            other_reserved + new_reserved_terminal_bytes,
            Ordering::SeqCst,
        );
    }
    record.retained_bytes = new_retained_bytes;
    record.reserved_terminal_bytes = new_reserved_terminal_bytes;
    Ok(envelope)
}

fn preflight_retained_budget(
    record: &IxRunRecord,
    payloads: &[(IxPrincipalBinding, IxEventPayload)],
    next_safe_move: &str,
    class: IxCommitClass,
    runtime_retention: Option<&IxRuntimeRetentionBudget>,
) -> Result<(), IxContractError> {
    let current_snapshot_bytes = canonical_json_bytes(&record.snapshot)?.len();
    let payload_bytes = canonical_json_bytes(&payloads)?.len();
    let prospective_snapshot_upper = current_snapshot_bytes
        .checked_add(payload_bytes)
        .and_then(|bytes| bytes.checked_add(next_safe_move.len()))
        .and_then(|bytes| bytes.checked_add(IX_RETAINED_COMMIT_OVERHEAD_BYTES))
        .ok_or(IxContractError::ResourceLimitExceeded)?;
    let prospective_envelope_upper = payload_bytes
        .checked_add(next_safe_move.len())
        .and_then(|bytes| bytes.checked_add(IX_RETAINED_COMMIT_OVERHEAD_BYTES))
        .ok_or(IxContractError::ResourceLimitExceeded)?;
    let prospective_retained = record
        .retained_bytes
        .checked_sub(current_snapshot_bytes)
        .and_then(|bytes| {
            prospective_snapshot_upper
                .checked_mul(2)
                .and_then(|added| bytes.checked_add(added))
        })
        .and_then(|bytes| bytes.checked_add(prospective_envelope_upper))
        .ok_or(IxContractError::ResourceLimitExceeded)?;
    let prospective_reserved = IX_CANCEL_COMMIT_RESERVE
        .checked_mul(
            prospective_snapshot_upper
                .checked_add(IX_RETAINED_COMMIT_OVERHEAD_BYTES)
                .ok_or(IxContractError::ResourceLimitExceeded)?,
        )
        .ok_or(IxContractError::ResourceLimitExceeded)?;
    enforce_retained_totals(
        record,
        prospective_retained,
        prospective_reserved,
        class,
        runtime_retention,
    )
}

fn enforce_exact_retained_budget(
    record: &IxRunRecord,
    new_retained_bytes: usize,
    new_reserved_terminal_bytes: usize,
    class: IxCommitClass,
    runtime_retention: Option<&IxRuntimeRetentionBudget>,
) -> Result<(), IxContractError> {
    enforce_retained_totals(
        record,
        new_retained_bytes,
        new_reserved_terminal_bytes,
        class,
        runtime_retention,
    )
}

fn enforce_retained_totals(
    record: &IxRunRecord,
    new_retained_bytes: usize,
    new_reserved_terminal_bytes: usize,
    class: IxCommitClass,
    runtime_retention: Option<&IxRuntimeRetentionBudget>,
) -> Result<(), IxContractError> {
    let run_total = match class {
        IxCommitClass::Effect => new_retained_bytes.checked_add(new_reserved_terminal_bytes),
        IxCommitClass::ReservedTerminal => Some(new_retained_bytes),
    }
    .filter(|total| *total <= IX_MAX_RETAINED_BYTES_PER_RUN)
    .ok_or(IxContractError::ResourceLimitExceeded)?;
    let _ = run_total;
    if let Some(retention) = runtime_retention {
        let other_retained = retention
            .retained_bytes
            .load(Ordering::SeqCst)
            .checked_sub(record.retained_bytes)
            .ok_or(IxContractError::ResourceLimitExceeded)?;
        let other_reserved = retention
            .reserved_terminal_bytes
            .load(Ordering::SeqCst)
            .checked_sub(record.reserved_terminal_bytes)
            .ok_or(IxContractError::ResourceLimitExceeded)?;
        let runtime_total = match class {
            IxCommitClass::Effect => other_retained
                .checked_add(new_retained_bytes)
                .and_then(|bytes| bytes.checked_add(other_reserved))
                .and_then(|bytes| bytes.checked_add(new_reserved_terminal_bytes)),
            IxCommitClass::ReservedTerminal => other_retained
                .checked_add(new_retained_bytes)
                .and_then(|bytes| bytes.checked_add(other_reserved)),
        }
        .filter(|total| *total <= retention.limit.load(Ordering::SeqCst))
        .ok_or(IxContractError::ResourceLimitExceeded)?;
        let _ = runtime_total;
    }
    Ok(())
}

fn terminal_retained_reserve(snapshot: &IxRunSnapshot) -> Result<usize, IxContractError> {
    let remaining_commits = match snapshot.state {
        IxRunState::Terminal => 0,
        IxRunState::Cancelling => match snapshot.cancellation.as_ref().map(|fence| fence.stage) {
            Some(IxCancellationStage::Accepted) => 4,
            Some(IxCancellationStage::Stopping) => 3,
            Some(IxCancellationStage::Draining) => 2,
            Some(IxCancellationStage::Stopped) => 1,
            Some(IxCancellationStage::Confirmed) | None => {
                return Err(IxContractError::InvalidCancellationProgress)
            }
        },
        IxRunState::Pending | IxRunState::Running | IxRunState::WaitingForUser => {
            IX_CANCEL_COMMIT_RESERVE
        }
    };
    canonical_json_bytes(snapshot)?
        .len()
        .checked_add(IX_RETAINED_COMMIT_OVERHEAD_BYTES)
        .and_then(|bytes| bytes.checked_mul(remaining_commits))
        .ok_or(IxContractError::ResourceLimitExceeded)
}

fn record_retained_bytes(record: &IxRunRecord) -> Result<usize, IxContractError> {
    let mut total = canonical_json_bytes(&record.snapshot)?.len();
    for snapshot in &record.snapshots {
        total = total
            .checked_add(canonical_json_bytes(snapshot)?.len())
            .ok_or(IxContractError::ResourceLimitExceeded)?;
    }
    for envelope in &record.envelopes {
        total = total
            .checked_add(canonical_json_bytes(envelope)?.len())
            .ok_or(IxContractError::ResourceLimitExceeded)?;
    }
    Ok(total)
}

fn apply_without_result_digest(
    base: &IxRunSnapshot,
    envelope: &IxCommitEnvelope,
) -> Result<IxRunSnapshot, IxContractError> {
    let expected_revision = base
        .task_revision
        .checked_add(1)
        .ok_or(IxContractError::InvalidLifecycle)?;
    validate_key("commitId", &envelope.commit_id)?;
    if envelope.schema_version != IX_COMMIT_SCHEMA_VERSION
        || envelope.committed_by != "appfw-runtime"
        || envelope.run_id != base.run_id
        || envelope.base_cursor != base.cursor
        || envelope.base_revision != base.task_revision
        || envelope.result_revision != expected_revision
        || envelope.result_cursor != cursor(&base.run_id, envelope.result_revision)
        || envelope.events.is_empty()
        || envelope.events.len() > IX_MAX_EVENTS_PER_COMMIT
    {
        return Err(IxContractError::InvalidLifecycle);
    }
    validate_detail("nextSafeMove", &envelope.next_safe_move)?;
    let mut result = base.clone();
    for (offset, event) in envelope.events.iter().enumerate() {
        let expected_sequence = base
            .last_sequence
            .checked_add(offset as u64)
            .and_then(|value| value.checked_add(1))
            .ok_or(IxContractError::InvalidSequence)?;
        if event.schema_version != IX_EVENT_SCHEMA_VERSION
            || event.run_id != base.run_id
            || event.sequence != expected_sequence
            || event.event_id != format!("{}:{expected_sequence}", base.run_id)
            || chrono::DateTime::parse_from_rfc3339(&event.occurred_at).is_err()
        {
            return Err(IxContractError::InvalidSequence);
        }
        apply_event(&mut result, event, envelope, offset)?;
        result.last_sequence = event.sequence;
    }
    result.task_revision = envelope.result_revision;
    result.cursor = envelope.result_cursor.clone();
    result.next_safe_move = envelope.next_safe_move.clone();
    Ok(result)
}

fn apply_event(
    snapshot: &mut IxRunSnapshot,
    event: &IxEvent,
    envelope: &IxCommitEnvelope,
    event_offset: usize,
) -> Result<(), IxContractError> {
    match &event.payload {
        IxEventPayload::RunAcknowledged {
            intent_label,
            focus,
        } => {
            require_runtime(event)?;
            if snapshot.state != IxRunState::Pending
                || snapshot.task_revision != 0
                || &snapshot.request.focus != focus
            {
                return Err(IxContractError::InvalidLifecycle);
            }
            validate_detail("intentLabel", intent_label)?;
            snapshot.state = IxRunState::Running;
            snapshot.phase = Some(IxPhase::Acknowledged);
        }
        IxEventPayload::ContextResolved { authority, context } => {
            require_runner(snapshot, event)?;
            validate_execution_authority(snapshot, authority)?;
            if !matches!(
                snapshot.state,
                IxRunState::Running | IxRunState::WaitingForUser
            ) || snapshot.context.is_some()
                || snapshot.context_authority.is_some()
            {
                return Err(IxContractError::InvalidLifecycle);
            }
            context.validate()?;
            snapshot.context = Some(context.clone());
            snapshot.context_authority = Some(authority.clone());
        }
        IxEventPayload::ContextReconciliationRequested { proposal } => {
            require_owner(snapshot, event)?;
            proposal.validate()?;
            if !matches!(
                snapshot.state,
                IxRunState::Running | IxRunState::WaitingForUser
            ) {
                return Err(IxContractError::InvalidLifecycle);
            }
            if snapshot.pending_context_proposal.is_some() {
                return Err(IxContractError::ContextProposalAlreadyPending);
            }
            if proposal.claims().release_id() != snapshot.release_id {
                return Err(IxContractError::ImmutableRunIdentityChanged);
            }
            if proposal.base_context_revision() != snapshot.authorization_context.revision()
                || proposal.claims().principal() != &snapshot.owner
            {
                return Err(IxContractError::InvalidContextRevision);
            }
            snapshot.pending_context_proposal = Some(proposal.clone());
        }
        IxEventPayload::ContextReconciled {
            proposal_id,
            context_revision,
        } => {
            require_runtime(event)?;
            context_revision.validate()?;
            let proposal = snapshot
                .pending_context_proposal
                .as_ref()
                .ok_or(IxContractError::InvalidContextRevision)?;
            if proposal.claims().release_id() != snapshot.release_id
                || context_revision.release_id() != snapshot.release_id
            {
                return Err(IxContractError::ImmutableRunIdentityChanged);
            }
            if !matches!(
                snapshot.state,
                IxRunState::Running | IxRunState::WaitingForUser
            ) || proposal.proposal_id() != proposal_id
                || proposal.claims() != context_revision.claims()
                || !context_revision.decided_by(snapshot.authorization_context.policy_authority())
                || context_revision.revision()
                    != snapshot
                        .authorization_context
                        .revision()
                        .checked_add(1)
                        .ok_or(IxContractError::InvalidContextRevision)?
                || context_revision.same_authorization_semantics(&snapshot.authorization_context)
            {
                return Err(IxContractError::InvalidContextRevision);
            }
            snapshot.authorization_context = context_revision.clone();
            snapshot.pending_context_proposal = None;
            snapshot.context = None;
            snapshot.context_authority = None;
        }
        IxEventPayload::PhaseChanged {
            authority,
            phase,
            reason,
        } => {
            let runtime_resume = event.actor == runtime_actor()
                && event_offset == 1
                && envelope.events.len() == 2
                && matches!(
                    envelope.events.first().map(|event| &event.payload),
                    Some(IxEventPayload::UserInputAccepted { .. })
                );
            if !runtime_resume {
                require_runner(snapshot, event)?;
            }
            validate_execution_authority(snapshot, authority)?;
            if snapshot.state != IxRunState::Running
                || snapshot.context.is_none()
                || *phase == IxPhase::Acknowledged
                || snapshot.phase == Some(*phase)
            {
                return Err(IxContractError::InvalidLifecycle);
            }
            validate_detail("phase.reason", reason)?;
            snapshot.phase = Some(*phase);
        }
        IxEventPayload::ArtifactRevision {
            authority,
            artifact,
        } => {
            require_runner(snapshot, event)?;
            validate_execution_authority(snapshot, authority)?;
            if snapshot.state != IxRunState::Running || snapshot.context.is_none() {
                return Err(IxContractError::InvalidLifecycle);
            }
            validate_artifact_transition(snapshot, artifact)?;
            snapshot.artifacts.insert(
                artifact.artifact_id.clone(),
                IxArtifactState::from(artifact),
            );
        }
        IxEventPayload::WaitingForUser { authority, prompt } => {
            require_runner(snapshot, event)?;
            validate_execution_authority(snapshot, authority)?;
            if snapshot.state != IxRunState::Running || snapshot.context.is_none() {
                return Err(IxContractError::InvalidLifecycle);
            }
            validate_detail("waiting.prompt", prompt)?;
            snapshot.state = IxRunState::WaitingForUser;
        }
        IxEventPayload::UserInputAccepted { authority } => {
            require_owner(snapshot, event)?;
            validate_execution_authority(snapshot, authority)?;
            if snapshot.state != IxRunState::WaitingForUser {
                return Err(IxContractError::InvalidLifecycle);
            }
            snapshot.state = IxRunState::Running;
        }
        IxEventPayload::CancelRequested {
            command_id,
            accepted_cursor,
            accepted_revision,
        } => {
            require_owner(snapshot, event)?;
            if !matches!(
                snapshot.state,
                IxRunState::Running | IxRunState::WaitingForUser
            ) || snapshot.cancellation.is_some()
                || accepted_cursor != &envelope.result_cursor
                || *accepted_revision != envelope.result_revision
            {
                return Err(IxContractError::InvalidLifecycle);
            }
            validate_key("commandId", command_id)?;
            snapshot.state = IxRunState::Cancelling;
            snapshot.cancellation = Some(IxCancellationFence {
                command_id: command_id.clone(),
                accepted_cursor: accepted_cursor.clone(),
                accepted_revision: *accepted_revision,
                stage: IxCancellationStage::Accepted,
            });
        }
        IxEventPayload::CancellationProgress { command_id, stage } => {
            require_runtime(event)?;
            if snapshot.state != IxRunState::Cancelling {
                return Err(IxContractError::InvalidLifecycle);
            }
            let fence = snapshot
                .cancellation
                .as_mut()
                .ok_or(IxContractError::InvalidLifecycle)?;
            let exact_next = matches!(
                (fence.stage, stage),
                (IxCancellationStage::Accepted, IxCancellationStage::Stopping)
                    | (IxCancellationStage::Stopping, IxCancellationStage::Draining)
                    | (IxCancellationStage::Draining, IxCancellationStage::Stopped)
                    | (IxCancellationStage::Stopped, IxCancellationStage::Confirmed)
            );
            if fence.command_id != *command_id || !exact_next {
                return Err(IxContractError::InvalidCancellationProgress);
            }
            fence.stage = *stage;
        }
        IxEventPayload::RunFinished {
            authority,
            outcome,
            message,
        } => {
            require_runtime(event)?;
            validate_detail("terminal.message", message)?;
            match outcome {
                IxTerminalOutcome::Cancelled => {
                    if authority.is_some() {
                        return Err(IxContractError::InvalidLifecycle);
                    }
                    let fence = snapshot
                        .cancellation
                        .as_ref()
                        .ok_or(IxContractError::InvalidLifecycle)?;
                    if snapshot.state != IxRunState::Cancelling
                        || fence.stage != IxCancellationStage::Confirmed
                    {
                        return Err(IxContractError::InvalidLifecycle);
                    }
                }
                IxTerminalOutcome::Completed | IxTerminalOutcome::Partial => {
                    validate_execution_authority(
                        snapshot,
                        authority
                            .as_ref()
                            .ok_or(IxContractError::StaleAuthorization)?,
                    )?;
                    if !matches!(
                        snapshot.state,
                        IxRunState::Running | IxRunState::WaitingForUser
                    ) || snapshot.context.is_none()
                        || snapshot.artifacts.is_empty()
                    {
                        return Err(IxContractError::InvalidLifecycle);
                    }
                }
                IxTerminalOutcome::Failed => {
                    if authority.is_some() {
                        return Err(IxContractError::InvalidLifecycle);
                    }
                    if !matches!(
                        snapshot.state,
                        IxRunState::Running | IxRunState::WaitingForUser
                    ) {
                        return Err(IxContractError::InvalidLifecycle);
                    }
                }
            }
            snapshot.state = IxRunState::Terminal;
            snapshot.terminal_outcome = Some(*outcome);
            snapshot.pending_context_proposal = None;
        }
    }
    Ok(())
}

fn validate_artifact_transition(
    snapshot: &IxRunSnapshot,
    artifact: &IxArtifactRevision,
) -> Result<(), IxContractError> {
    artifact.validate()?;
    let binding = IxArtifactBinding {
        artifact_type: artifact.artifact_type.clone(),
        content_schema_version: artifact.content_schema_version.clone(),
        renderer_key: artifact.renderer_key.clone(),
    };
    if !snapshot.registered_artifacts.contains(&binding) {
        return Err(IxContractError::UnknownArtifactType);
    }
    if let Some(existing) = snapshot.artifacts.get(&artifact.artifact_id) {
        if existing.artifact_type != artifact.artifact_type
            || existing.content_schema_version != artifact.content_schema_version
            || existing.renderer_key != artifact.renderer_key
        {
            return Err(IxContractError::ArtifactIdentityChanged);
        }
        if existing.revision.checked_add(1) != Some(artifact.revision) {
            return Err(IxContractError::InvalidArtifactRevision);
        }
        return Ok(());
    }
    if let Some(related) = snapshot
        .request
        .related_artifact
        .as_ref()
        .filter(|related| !snapshot.artifacts.contains_key(&related.artifact_id))
    {
        if related.artifact_id != artifact.artifact_id
            || related.artifact_type != artifact.artifact_type
            || related.content_schema_version != artifact.content_schema_version
            || related.renderer_key != artifact.renderer_key
            || related.revision.checked_add(1) != Some(artifact.revision)
        {
            return Err(IxContractError::ArtifactIdentityChanged);
        }
    } else if artifact.revision != 1 {
        return Err(IxContractError::InvalidArtifactRevision);
    }
    Ok(())
}

fn validate_execution_authority(
    snapshot: &IxRunSnapshot,
    authority: &IxExecutionAuthority,
) -> Result<(), IxContractError> {
    snapshot.authorization_context.validate()?;
    if authority != &snapshot.authorization_context.binding() {
        return Err(IxContractError::StaleAuthorization);
    }
    if !snapshot.authorization_context.allows_effects() {
        return Err(IxContractError::EffectNotAuthorized);
    }
    Ok(())
}

fn require_runtime(event: &IxEvent) -> Result<(), IxContractError> {
    (event.actor == runtime_actor())
        .then_some(())
        .ok_or(IxContractError::UnauthorizedActor)
}

fn require_owner(snapshot: &IxRunSnapshot, event: &IxEvent) -> Result<(), IxContractError> {
    (event.actor == snapshot.owner)
        .then_some(())
        .ok_or(IxContractError::UnauthorizedActor)
}

fn require_runner(snapshot: &IxRunSnapshot, event: &IxEvent) -> Result<(), IxContractError> {
    (event.actor == snapshot.runner)
        .then_some(())
        .ok_or(IxContractError::UnauthorizedActor)
}

fn snapshot_digest(snapshot: &IxRunSnapshot) -> Result<String, IxContractError> {
    let bytes = canonical_json_bytes(snapshot)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn replay_checkpoint_for_range(
    base: &IxRunSnapshot,
    result: &IxRunSnapshot,
    envelopes: &[IxCommitEnvelope],
) -> Result<IxReplayCheckpoint, IxContractError> {
    validate_snapshot_projection(base)?;
    validate_snapshot_projection(result)?;
    let envelope_count =
        u64::try_from(envelopes.len()).map_err(|_| IxContractError::ResourceLimitExceeded)?;
    if envelopes.len() > IX_MAX_COMMITS
        || base.run_id != result.run_id
        || base.owner != result.owner
        || base.task_revision.checked_add(envelope_count) != Some(result.task_revision)
    {
        return Err(IxContractError::DigestMismatch);
    }
    Ok(IxReplayCheckpoint {
        schema_version: IX_REPLAY_CHECKPOINT_SCHEMA_VERSION.to_string(),
        run_id: base.run_id.clone(),
        owner: base.owner.clone(),
        base_revision: base.task_revision,
        result_revision: result.task_revision,
        base_cursor: base.cursor.clone(),
        result_cursor: result.cursor.clone(),
        base_snapshot_sha256: snapshot_digest(base)?,
        result_snapshot_sha256: snapshot_digest(result)?,
        envelope_count,
        envelope_chain_sha256: envelope_chain_sha256(envelopes)?,
    })
}

fn envelope_chain_sha256(envelopes: &[IxCommitEnvelope]) -> Result<String, IxContractError> {
    if envelopes.len() > IX_MAX_COMMITS {
        return Err(IxContractError::ResourceLimitExceeded);
    }
    let mut state = Sha256::digest(b"appfw.ix.replay-envelope-chain@1").to_vec();
    let mut total_bytes = 0usize;
    for (index, envelope) in envelopes.iter().enumerate() {
        if envelope.events.is_empty() || envelope.events.len() > IX_MAX_EVENTS_PER_COMMIT {
            return Err(IxContractError::ResourceLimitExceeded);
        }
        for event in &envelope.events {
            if let IxEventPayload::ArtifactRevision { artifact, .. } = &event.payload {
                artifact.validate()?;
            }
        }
        let bytes = canonical_json_bytes(envelope)?;
        total_bytes = total_bytes
            .checked_add(bytes.len())
            .filter(|total| *total <= IX_MAX_REPLAY_BYTES)
            .ok_or(IxContractError::ResourceLimitExceeded)?;
        let mut hasher = Sha256::new();
        hasher.update(b"appfw.ix.replay-envelope@1\0");
        hasher.update(&state);
        hasher.update(
            u64::try_from(index)
                .map_err(|_| IxContractError::ResourceLimitExceeded)?
                .to_be_bytes(),
        );
        hasher.update(
            u64::try_from(bytes.len())
                .map_err(|_| IxContractError::ResourceLimitExceeded)?
                .to_be_bytes(),
        );
        hasher.update(bytes);
        state = hasher.finalize().to_vec();
    }
    Ok(format!(
        "sha256:{}",
        state
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    ))
}

fn canonical_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, IxContractError> {
    let value = serde_json::to_value(value).map_err(|_| IxContractError::DigestMismatch)?;
    validate_json_shape(
        &value,
        super::contract::IX_MAX_JSON_DEPTH + 16,
        super::contract::IX_MAX_JSON_NODES * 16,
        false,
    )?;
    serde_json::to_vec(&canonical_json_value(value)).map_err(|_| IxContractError::DigestMismatch)
}

fn canonical_json_value(value: Value) -> Value {
    match value {
        Value::Array(values) => {
            Value::Array(values.into_iter().map(canonical_json_value).collect())
        }
        Value::Object(values) => {
            let mut entries = values.into_iter().collect::<Vec<_>>();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            let mut canonical = Map::new();
            for (key, value) in entries {
                canonical.insert(key, canonical_json_value(value));
            }
            Value::Object(canonical)
        }
        scalar => scalar,
    }
}

fn validate_snapshot_projection(snapshot: &IxRunSnapshot) -> Result<(), IxContractError> {
    if snapshot.schema_version != IX_SNAPSHOT_SCHEMA_VERSION
        || snapshot.task_revision > IX_MAX_COMMITS as u64
        || snapshot.cursor != cursor(&snapshot.run_id, snapshot.task_revision)
        || snapshot.request.question.is_some()
        || snapshot.registered_artifacts.is_empty()
        || snapshot.registered_artifacts.len() > 8
        || snapshot.artifacts.len() > 8
    {
        return Err(IxContractError::InvalidLifecycle);
    }
    validate_key("runId", &snapshot.run_id)?;
    validate_key("releaseId", &snapshot.release_id)?;
    validate_principal_binding("principal", &snapshot.owner)?;
    validate_principal_binding("principal", &snapshot.runner)?;
    if snapshot.owner.principal_type != RuntimePrincipalType::User
        || !matches!(
            snapshot.runner.principal_type,
            RuntimePrincipalType::Agent | RuntimePrincipalType::Service
        )
    {
        return Err(IxContractError::UnauthorizedActor);
    }
    snapshot.request.validate()?;
    snapshot.authorization_context.validate()?;
    if snapshot.authorization_context.principal() != &snapshot.owner {
        return Err(IxContractError::UnauthorizedActor);
    }
    if snapshot.authorization_context.release_id() != snapshot.release_id {
        return Err(IxContractError::ImmutableRunIdentityChanged);
    }
    if let Some(proposal) = &snapshot.pending_context_proposal {
        proposal.validate()?;
        if proposal.base_context_revision() != snapshot.authorization_context.revision()
            || proposal.claims().release_id() != snapshot.release_id
            || proposal.claims().principal() != &snapshot.owner
            || !matches!(
                snapshot.state,
                IxRunState::Running | IxRunState::WaitingForUser
            )
        {
            return Err(IxContractError::InvalidContextRevision);
        }
    }
    for binding in &snapshot.registered_artifacts {
        binding.validate()?;
    }
    super::recipe::validate_recipe_binding(
        snapshot.recipe_registration.as_ref(),
        &snapshot.request,
        &snapshot.registered_artifacts,
    )?;
    match (&snapshot.context, &snapshot.context_authority) {
        (Some(context), Some(authority)) => {
            context.validate()?;
            if authority != &snapshot.authorization_context.binding() {
                return Err(IxContractError::StaleAuthorization);
            }
        }
        (None, None) => {}
        _ => return Err(IxContractError::InvalidLifecycle),
    }
    for (artifact_id, artifact) in &snapshot.artifacts {
        let revision = IxArtifactRevision {
            artifact_id: artifact_id.clone(),
            artifact_type: artifact.artifact_type.clone(),
            content_schema_version: artifact.content_schema_version.clone(),
            renderer_key: artifact.renderer_key.clone(),
            revision: artifact.revision,
            status: artifact.status,
            changed_region_ids: artifact.changed_region_ids.clone(),
            presentation: artifact.presentation.clone(),
        };
        revision.validate()?;
        let binding = IxArtifactBinding {
            artifact_type: artifact.artifact_type.clone(),
            content_schema_version: artifact.content_schema_version.clone(),
            renderer_key: artifact.renderer_key.clone(),
        };
        if !snapshot.registered_artifacts.contains(&binding) {
            return Err(IxContractError::UnknownArtifactType);
        }
    }
    validate_detail("nextSafeMove", &snapshot.next_safe_move)?;
    let max_events = snapshot
        .task_revision
        .checked_mul(IX_MAX_EVENTS_PER_COMMIT as u64)
        .ok_or(IxContractError::ResourceLimitExceeded)?;
    if snapshot.last_sequence < snapshot.task_revision || snapshot.last_sequence > max_events {
        return Err(IxContractError::InvalidSequence);
    }
    if let Some(fence) = &snapshot.cancellation {
        validate_key("commandId", &fence.command_id)?;
        if fence.accepted_revision == 0
            || fence.accepted_revision > snapshot.task_revision
            || fence.accepted_cursor != cursor(&snapshot.run_id, fence.accepted_revision)
        {
            return Err(IxContractError::InvalidCancellationProgress);
        }
    }
    match snapshot.state {
        IxRunState::Pending => {
            if snapshot.task_revision != 0
                || snapshot.last_sequence != 0
                || snapshot.phase.is_some()
                || snapshot.context.is_some()
                || snapshot.cancellation.is_some()
                || snapshot.terminal_outcome.is_some()
            {
                return Err(IxContractError::InvalidLifecycle);
            }
        }
        IxRunState::Running => {
            if snapshot.phase.is_none()
                || snapshot.cancellation.is_some()
                || snapshot.terminal_outcome.is_some()
            {
                return Err(IxContractError::InvalidLifecycle);
            }
        }
        IxRunState::WaitingForUser => {
            if snapshot.cancellation.is_some() || snapshot.terminal_outcome.is_some() {
                return Err(IxContractError::InvalidLifecycle);
            }
        }
        IxRunState::Cancelling => {
            if snapshot.cancellation.is_none() || snapshot.terminal_outcome.is_some() {
                return Err(IxContractError::InvalidLifecycle);
            }
        }
        IxRunState::Terminal => {
            let outcome = snapshot
                .terminal_outcome
                .ok_or(IxContractError::InvalidLifecycle)?;
            if (outcome == IxTerminalOutcome::Cancelled) != snapshot.cancellation.is_some() {
                return Err(IxContractError::InvalidLifecycle);
            }
        }
    }
    Ok(())
}

fn snapshot_at<'a>(
    record: &'a IxRunRecord,
    run_id: &str,
    cursor_value: Option<&str>,
) -> Result<(usize, &'a IxRunSnapshot), IxContractError> {
    let revision = match cursor_value {
        Some(value) => parse_cursor(value, run_id)?,
        None => 0,
    };
    let index = usize::try_from(revision).map_err(|_| IxContractError::InvalidReplayCursor)?;
    let snapshot = record
        .snapshots
        .get(index)
        .filter(|snapshot| cursor_value.is_none_or(|value| snapshot.cursor == value))
        .ok_or(IxContractError::InvalidReplayCursor)?;
    Ok((index, snapshot))
}

fn parse_cursor(value: &str, run_id: &str) -> Result<u64, IxContractError> {
    let expected_prefix = format!("ix1.{run_id}.");
    let revision = value
        .strip_prefix(&expected_prefix)
        .filter(|suffix| !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit()))
        .and_then(|suffix| suffix.parse::<u64>().ok())
        .ok_or(IxContractError::InvalidReplayCursor)?;
    if value != cursor(run_id, revision) {
        return Err(IxContractError::InvalidReplayCursor);
    }
    Ok(revision)
}

fn canonical_uuid(value: &str) -> bool {
    Uuid::parse_str(value)
        .ok()
        .is_some_and(|parsed| parsed.to_string() == value)
}

fn owned_record<'a>(
    runs: &'a HashMap<String, IxRunRecord>,
    owner: &UserAuth,
    run_id: &str,
) -> Result<&'a IxRunRecord, IxContractError> {
    let owner_binding = authenticated_owner_binding(owner)?;
    runs.get(run_id)
        .filter(|record| record.snapshot.owner == owner_binding)
        .ok_or(IxContractError::RunNotFound)
}

fn owned_record_mut<'a>(
    runs: &'a mut HashMap<String, IxRunRecord>,
    owner: &UserAuth,
    run_id: &str,
) -> Result<&'a mut IxRunRecord, IxContractError> {
    let owner_binding = authenticated_owner_binding(owner)?;
    runs.get_mut(run_id)
        .filter(|record| record.snapshot.owner == owner_binding)
        .ok_or(IxContractError::RunNotFound)
}

fn runner_record<'a>(
    runs: &'a HashMap<String, IxRunRecord>,
    runner: &UserAuth,
    run_id: &str,
) -> Result<&'a IxRunRecord, IxContractError> {
    let runner_binding = qualified_runner_binding(runner)?;
    let record = runs.get(run_id).ok_or(IxContractError::RunNotFound)?;
    (record.snapshot.runner == runner_binding)
        .then_some(record)
        .ok_or(IxContractError::UnauthorizedActor)
}

fn runner_record_mut<'a>(
    runs: &'a mut HashMap<String, IxRunRecord>,
    runner: &UserAuth,
    run_id: &str,
) -> Result<&'a mut IxRunRecord, IxContractError> {
    let runner_binding = qualified_runner_binding(runner)?;
    let record = runs.get_mut(run_id).ok_or(IxContractError::RunNotFound)?;
    (record.snapshot.runner == runner_binding)
        .then_some(record)
        .ok_or(IxContractError::UnauthorizedActor)
}

fn authenticated_owner_binding(owner: &UserAuth) -> Result<IxPrincipalBinding, IxContractError> {
    if owner.principal_type != RuntimePrincipalType::User || owner.token.trim().is_empty() {
        return Err(IxContractError::UnauthorizedActor);
    }
    Ok(IxPrincipalBinding::from(owner))
}

fn qualified_runner_binding(runner: &UserAuth) -> Result<IxPrincipalBinding, IxContractError> {
    if !matches!(
        runner.principal_type,
        RuntimePrincipalType::Agent | RuntimePrincipalType::Service
    ) || !runner.has_scope("appfw:ix.progress")
    {
        return Err(IxContractError::UnauthorizedActor);
    }
    Ok(IxPrincipalBinding::from(runner))
}

fn ensure_start_capacity(
    runs: &HashMap<String, IxRunRecord>,
    owner: &IxPrincipalBinding,
) -> Result<(), IxContractError> {
    let active = runs
        .values()
        .filter(|record| record.snapshot.state != IxRunState::Terminal);
    let active_count = active.clone().count();
    let owner_active_count = active
        .filter(|record| &record.snapshot.owner == owner)
        .count();
    if runs.len() >= IX_MAX_RETAINED_RUNS
        || active_count >= IX_MAX_ACTIVE_RUNS
        || owner_active_count >= IX_MAX_ACTIVE_RUNS_PER_OWNER
    {
        return Err(IxContractError::ResourceLimitExceeded);
    }
    Ok(())
}

fn lock_runs(
    runs: &Arc<Mutex<HashMap<String, IxRunRecord>>>,
) -> MutexGuard<'_, HashMap<String, IxRunRecord>> {
    runs.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use serde_json::json;

    use super::super::contract::{
        validate_portable_json, IX_MAX_JSON_DEPTH, IX_MAX_JSON_NODES, IX_MAX_PRESENTATION_BYTES,
    };
    use super::*;
    use crate::ix::{
        IxArtifactStatus, IxConsentState, IxContextClaims, IxContextFreshness,
        IxDataClassification, IxEgressPosture, IxFocusRef, IxRelatedArtifactRef, IxSourceSummary,
        IX_RUN_REQUEST_SCHEMA_VERSION, PDS_IX_PRESENTATION_SCHEMA_VERSION,
    };

    #[derive(Default)]
    struct MemoryAudit {
        fail: AtomicBool,
        events: Mutex<Vec<super::super::IxAuditEvent>>,
    }

    impl MemoryAudit {
        fn failing() -> Self {
            Self {
                fail: AtomicBool::new(true),
                events: Mutex::new(Vec::new()),
            }
        }

        fn events(&self) -> Vec<super::super::IxAuditEvent> {
            self.events.lock().expect("audit lock").clone()
        }
    }

    impl IxAuditSink for MemoryAudit {
        fn append(&self, event: &super::super::IxAuditEvent) -> Result<(), IxContractError> {
            if self.fail.load(Ordering::SeqCst) {
                return Err(IxContractError::AuditUnavailable);
            }
            self.events.lock().expect("audit lock").push(event.clone());
            Ok(())
        }
    }

    struct TestPolicy;

    impl IxContextPolicy for TestPolicy {
        fn decide_initial(
            &self,
            _claims: &IxContextClaims,
        ) -> Result<IxPolicyVerdict, IxContractError> {
            Ok(IxPolicyVerdict {
                decision_id: "initial-decision".to_string(),
                policy_version: "ix-policy-v1".to_string(),
                classification: IxDataClassification::Confidential,
                consent: IxConsentState::Granted,
                egress: IxEgressPosture::LocalOnly,
            })
        }

        fn decide_reconciliation(
            &self,
            current: &IxContextRevision,
            proposal: &IxContextProposal,
        ) -> Result<IxPolicyVerdict, IxContractError> {
            let classification = match proposal.requested_classification() {
                Some(IxDataClassification::Restricted) => IxDataClassification::Restricted,
                _ => current.classification(),
            };
            let consent = proposal.requested_consent().unwrap_or(current.consent());
            let egress = match proposal.requested_egress_destination() {
                Some("pds-health-ai") => IxEgressPosture::Approved {
                    policy_id: "policy-1".to_string(),
                    decision_id: format!("egress-{}", proposal.proposal_id()),
                    destination: "pds-health-ai".to_string(),
                },
                _ => current.egress().clone(),
            };
            Ok(IxPolicyVerdict {
                decision_id: format!("decision-{}", proposal.proposal_id()),
                policy_version: "ix-policy-v1".to_string(),
                classification,
                consent,
                egress,
            })
        }
    }

    fn owner(name: &str) -> UserAuth {
        UserAuth::human(
            "tenant-1",
            name,
            "UTC",
            vec!["nexus-user".to_string()],
            vec!["nexus:read".to_string()],
            "secret-token",
        )
    }

    fn runner() -> UserAuth {
        UserAuth::agent(
            "tenant-1",
            "nexus-ix-runner",
            vec!["ix-runner".to_string()],
            vec!["appfw:ix.progress".to_string()],
        )
    }

    fn policy_authority() -> UserAuth {
        UserAuth::service(
            "tenant-1",
            "nexus-ix-policy",
            vec!["ix-policy".to_string()],
            vec!["appfw:ix.authorize".to_string()],
        )
    }

    fn request() -> IxRunRequest {
        IxRunRequest {
            schema_version: IX_RUN_REQUEST_SCHEMA_VERSION.to_string(),
            intent_key: "analyze_why".to_string(),
            focus: IxFocusRef {
                kind: "provider_request".to_string(),
                id: "request-42".to_string(),
            },
            question: Some("What changed?".to_string()),
            related_artifact: None,
        }
    }

    fn policy() -> IxRunPolicy {
        IxRunPolicy::new(
            "Analyze why",
            &runner(),
            [IxArtifactBinding::new(
                "nexus.ix.working_brief@1",
                PDS_IX_PRESENTATION_SCHEMA_VERSION,
                "pds.ix.working_brief",
            )
            .expect("valid artifact binding")],
        )
        .expect("valid policy")
    }

    fn recipe_request(registration: &super::super::recipe::IxRecipeRegistration) -> IxRunRequest {
        IxRunRequest {
            intent_key: registration.intent_key().to_string(),
            ..request()
        }
    }

    fn recipe_policy(
        registration: super::super::recipe::IxRecipeRegistration,
    ) -> Result<IxRunPolicy, IxContractError> {
        IxRunPolicy::new_for_recipe("Canonical IX recipe", &runner(), registration)
    }

    fn recipe_artifact(
        registration: &super::super::recipe::IxRecipeRegistration,
        revision: u64,
    ) -> IxArtifactRevision {
        let mut value = artifact(revision);
        value.artifact_type = registration.artifact_type().to_string();
        value.content_schema_version = registration.content_schema_version().to_string();
        value.renderer_key = registration.renderer_key().to_string();
        value
    }

    fn start_recipe_result(
        runtime: &IxForegroundRuntime,
        owner: &UserAuth,
        request: IxRunRequest,
        registration: super::super::recipe::IxRecipeRegistration,
    ) -> Result<IxStartReceipt, IxContractError> {
        let claims =
            IxContextClaims::from_authenticated_human("nexus-release-1", "session-1", owner)?;
        runtime.start(
            &request_context(),
            owner,
            claims,
            request,
            recipe_policy(registration)?,
        )
    }

    fn start_with_policy_result(
        runtime: &IxForegroundRuntime,
        owner: &UserAuth,
        request: IxRunRequest,
        policy: IxRunPolicy,
    ) -> Result<IxStartReceipt, IxContractError> {
        let claims =
            IxContextClaims::from_authenticated_human("nexus-release-1", "session-1", owner)?;
        runtime.start(&request_context(), owner, claims, request, policy)
    }

    fn authorization_context(owner: &UserAuth, revision: u64) -> IxContextRevision {
        custom_authorization_context(
            owner,
            revision,
            "nexus-release-1",
            "session-1",
            IxDataClassification::Confidential,
            IxConsentState::Granted,
            IxEgressPosture::LocalOnly,
        )
    }

    fn custom_authorization_context(
        owner: &UserAuth,
        revision: u64,
        release_id: &str,
        session_id: &str,
        classification: IxDataClassification,
        consent: IxConsentState,
        egress: IxEgressPosture,
    ) -> IxContextRevision {
        IxContextRevision::from_policy_verdict(
            revision,
            IxContextClaims::from_authenticated_human(release_id, session_id, owner)
                .expect("authenticated owner claims"),
            IxPrincipalBinding::from(&policy_authority()),
            IxPolicyVerdict {
                decision_id: format!("decision-{revision}"),
                policy_version: "ix-policy-v1".to_string(),
                classification,
                consent,
                egress,
            },
        )
        .expect("valid authorization context")
    }

    #[allow(clippy::too_many_arguments)]
    fn authorization_context_with_policy(
        owner: &UserAuth,
        revision: u64,
        release_id: &str,
        session_id: &str,
        authority: &UserAuth,
        decision_id: &str,
        policy_version: &str,
        classification: IxDataClassification,
        consent: IxConsentState,
        egress: IxEgressPosture,
    ) -> IxContextRevision {
        IxContextRevision::from_policy_verdict(
            revision,
            IxContextClaims::from_authenticated_human(release_id, session_id, owner)
                .expect("authenticated owner claims"),
            IxPrincipalBinding::from(authority),
            IxPolicyVerdict {
                decision_id: decision_id.to_string(),
                policy_version: policy_version.to_string(),
                classification,
                consent,
                egress,
            },
        )
        .expect("valid authorization context")
    }

    fn start_result(
        runtime: &IxForegroundRuntime,
        owner: &UserAuth,
        request: IxRunRequest,
    ) -> Result<IxStartReceipt, IxContractError> {
        let claims =
            IxContextClaims::from_authenticated_human("nexus-release-1", "session-1", owner)?;
        runtime.start(&request_context(), owner, claims, request, policy())
    }

    fn propose_and_reconcile(
        runtime: &IxForegroundRuntime,
        owner: &UserAuth,
        run_id: &str,
        next: IxContextRevision,
    ) -> Result<IxCommitEnvelope, IxContractError> {
        let proposal = IxContextProposal::from_authenticated_human(
            format!(
                "proposal-{}-{}-{}",
                next.revision(),
                next.release_id(),
                next.session_id()
            ),
            next.revision() - 1,
            next.release_id(),
            next.session_id(),
            owner,
            "The authenticated owner requests context reconciliation.",
            Some(next.classification()),
            Some(next.consent()),
            match next.egress() {
                IxEgressPosture::LocalOnly => None,
                IxEgressPosture::Approved { destination, .. } => Some(destination.clone()),
            },
        )?;
        runtime.request_context_reconciliation(owner, run_id, proposal)?;
        runtime.reconcile_context(run_id)
    }

    fn authority(
        runtime: &IxForegroundRuntime,
        owner: &UserAuth,
        run_id: &str,
    ) -> IxExecutionAuthority {
        runtime
            .snapshot(owner, run_id)
            .expect("authorized snapshot")
            .authorization_context
            .binding()
    }

    fn assert_retention_accounting(runtime: &IxForegroundRuntime) {
        let runs = lock_runs(&runtime.runs);
        let retained = runs.values().fold(0usize, |total, record| {
            assert_eq!(
                record.retained_bytes,
                record_retained_bytes(record).expect("retained bytes calculate exactly")
            );
            total + record.retained_bytes
        });
        let reserved = runs
            .values()
            .map(|record| record.reserved_terminal_bytes)
            .sum::<usize>();
        assert_eq!(
            runtime.retention.retained_bytes.load(Ordering::SeqCst),
            retained
        );
        assert_eq!(
            runtime
                .retention
                .reserved_terminal_bytes
                .load(Ordering::SeqCst),
            reserved
        );
    }

    fn request_context() -> RequestContext {
        RequestContext::new("request-1", "correlation-1")
    }

    fn context() -> IxContextSummary {
        IxContextSummary {
            focus_label: "Provider request 42".to_string(),
            detail: "Authorized projection as of the evaluation time.".to_string(),
            evaluated_at: "2026-08-11T12:00:00Z".to_string(),
            freshness: IxContextFreshness::Current,
            sources: vec![IxSourceSummary {
                source_ref: "servicenow:request-42".to_string(),
                label: "ServiceNow request".to_string(),
                refreshed_at: Some("2026-08-11T11:59:00Z".to_string()),
                freshness: IxContextFreshness::Current,
                inspectable: true,
            }],
            gaps: vec![],
        }
    }

    fn pds_presentation(presentation_id: &str, revision: u64) -> Value {
        json!({
            "identity": {
                "schemaVersion": PDS_IX_PRESENTATION_SCHEMA_VERSION,
                "presentationId": presentation_id,
                "revision": revision
            },
            "announcement": format!("Working brief revision {revision} is ready."),
            "response": {
                "eyebrow": "Progressive response",
                "title": "Working brief",
                "announcement": format!("Working brief revision {revision} is ready."),
                "active": true,
                "regions": [{
                    "id": "summary",
                    "status": "partial",
                    "title": "Summary",
                    "body": "Grounded work is in progress.",
                    "changed": true
                }],
                "emptyState": "Useful sections will appear when ready."
            }
        })
    }

    fn reviewed_pds_golden_presentation() -> Value {
        json!({
            "identity": {
                "schemaVersion": "pds.ix.presentation@1",
                "presentationId": "fixture-working-brief",
                "revision": 1
            },
            "announcement": "Working brief revision 1 is ready.",
            "context": {
                "eyebrow": "Working from",
                "title": "Sanitized reference context",
                "detail": "One current source",
                "announcement": "Context resolved from one source with one known gap.",
                "gaps": ["Approval owner is not yet resolved."],
                "evidence": {
                    "summary": "View source",
                    "items": [{
                        "id": "source-a",
                        "sourceRef": "fixture-source-a",
                        "label": "Source",
                        "value": "Sanitized fixture"
                    }]
                },
                "metaLabel": "Evaluated now"
            },
            "workStatus": {
                "label": "Evidence checked",
                "detail": "One source resolved",
                "active": false
            },
            "response": {
                "eyebrow": "Progressive response",
                "title": "Working brief",
                "metaLabel": "Revision 1",
                "announcement": "Working brief revision 1 is ready.",
                "active": false,
                "regions": [{
                    "id": "finding",
                    "status": "ready",
                    "label": "Finding",
                    "title": "A bounded finding",
                    "body": "Plain portable content.",
                    "whyItMatters": "It changes the next decision.",
                    "evidence": {
                        "summary": "Where did this come from?",
                        "items": [{
                            "sourceRef": "fixture-source-a",
                            "label": "Source",
                            "value": "Sanitized fixture"
                        }]
                    },
                    "editable": true,
                    "changed": true
                }],
                "emptyState": "Useful sections will appear when ready.",
                "footerText": "Sanitized reference data"
            }
        })
    }

    fn large_pds_presentation(presentation_id: &str, revision: u64) -> Value {
        let regions = (0..64)
            .map(|index| {
                json!({
                    "id": if index == 0 { "summary".to_string() } else { format!("region-{index}") },
                    "status": "partial",
                    "title": format!("Bounded region {index}"),
                    "body": "x".repeat(4_000),
                    "whyItMatters": "y".repeat(3_600),
                    "changed": index == 0
                })
            })
            .collect::<Vec<_>>();
        json!({
            "identity": {
                "schemaVersion": PDS_IX_PRESENTATION_SCHEMA_VERSION,
                "presentationId": presentation_id,
                "revision": revision
            },
            "announcement": format!("Large working brief revision {revision} is ready."),
            "response": {
                "eyebrow": "Progressive response",
                "title": "Large working brief",
                "announcement": format!("Large working brief revision {revision} is ready."),
                "active": true,
                "regions": regions,
                "emptyState": "Useful sections will appear when ready."
            }
        })
    }

    fn artifact(revision: u64) -> IxArtifactRevision {
        IxArtifactRevision {
            artifact_id: "brief-1".to_string(),
            artifact_type: "nexus.ix.working_brief@1".to_string(),
            content_schema_version: PDS_IX_PRESENTATION_SCHEMA_VERSION.to_string(),
            renderer_key: "pds.ix.working_brief".to_string(),
            revision,
            status: IxArtifactStatus::Partial,
            changed_region_ids: vec!["summary".to_string()],
            presentation: pds_presentation("brief-1", revision),
        }
    }

    fn started_runtime() -> (
        IxForegroundRuntime,
        Arc<MemoryAudit>,
        UserAuth,
        UserAuth,
        String,
    ) {
        let audit = Arc::new(MemoryAudit::default());
        let runtime =
            IxForegroundRuntime::new(audit.clone(), &policy_authority(), Arc::new(TestPolicy))
                .expect("policy authority configures runtime");
        let owner = owner("wayne");
        let runner = runner();
        let started = start_result(&runtime, &owner, request()).expect("run starts");
        (runtime, audit, owner, runner, started.run_id)
    }

    #[test]
    fn start_fails_closed_before_state_when_audit_is_unavailable() {
        let runtime = IxForegroundRuntime::new(
            Arc::new(MemoryAudit::failing()),
            &policy_authority(),
            Arc::new(TestPolicy),
        )
        .expect("policy authority configures runtime");
        let run_owner = owner("wayne");
        let result = start_result(&runtime, &run_owner, request());
        assert_eq!(result, Err(IxContractError::AuditUnavailable));
        assert!(lock_runs(&runtime.runs).is_empty());
    }

    #[test]
    fn recipe_bound_runtime_admits_all_eight_and_retains_registration_through_replay() {
        for registration in super::super::recipe::test_registrations() {
            let audit = Arc::new(MemoryAudit::default());
            let runtime =
                IxForegroundRuntime::new(audit.clone(), &policy_authority(), Arc::new(TestPolicy))
                    .expect("policy authority configures runtime");
            let owner = owner("wayne");
            let runner = runner();
            let request = recipe_request(&registration);
            let receipt = start_recipe_result(&runtime, &owner, request, registration.clone())
                .expect("canonical recipe starts");
            assert_eq!(audit.events().len(), 1);
            let started = runtime.snapshot(&owner, &receipt.run_id).expect("snapshot");
            assert_eq!(started.recipe_registration.as_ref(), Some(&registration));
            assert_eq!(started.request.intent_key, registration.intent_key());
            assert_eq!(started.registered_artifacts.len(), 1);
            let authority = started.authorization_context.binding();
            runtime
                .resolve_context(&runner, &receipt.run_id, &authority, context())
                .expect("context resolves");
            runtime
                .publish_artifact(
                    &runner,
                    &receipt.run_id,
                    &authority,
                    recipe_artifact(&registration, 1),
                )
                .expect("canonical recipe artifact publishes");
            let checkpoint = runtime
                .replay_checkpoint(&owner, &receipt.run_id, None)
                .expect("checkpoint");
            let replay = runtime
                .replay_after(&owner, &receipt.run_id, None)
                .expect("replay");
            IxReplayVerifier::verify(&replay, &checkpoint).expect("recipe replay verifies");
            assert_eq!(
                replay.expected_snapshot.recipe_registration.as_ref(),
                Some(&registration)
            );

            let mut tampered = replay.expected_snapshot.clone();
            tampered
                .recipe_registration
                .as_mut()
                .expect("recipe-bound snapshot")
                .renderer_key = "pds.ix.recipe.substituted@1".to_string();
            assert_eq!(
                validate_snapshot_projection(&tampered),
                Err(IxContractError::InvalidRecipeRegistration)
            );
        }
    }

    #[test]
    fn recipe_bound_start_rejects_mismatch_before_audit_or_run_mutation() {
        let canonical = super::super::recipe::test_registrations()
            .into_iter()
            .next()
            .expect("canonical recipe");
        let mut candidates = Vec::new();
        let mut wrong_intent = recipe_request(&canonical);
        wrong_intent.intent_key = "pds.ix.intent.contextual-conversation@1".to_string();
        candidates.push((wrong_intent, canonical.clone()));

        let mut wrong_renderer = canonical.clone();
        wrong_renderer.renderer_key = "pds.ix.recipe.contextual-conversation@1".to_string();
        let mut wrong_schema = canonical.clone();
        wrong_schema.content_schema_version = "product.presentation@1".to_string();
        let mut missing_capability = canonical.clone();
        missing_capability.required_capabilities.pop();
        let mut extra_capability = canonical.clone();
        extra_capability
            .required_capabilities
            .push("pds.ix.capability.contextual-follow-up@1".to_string());
        let mut reordered_capabilities = canonical.clone();
        reordered_capabilities.required_capabilities.swap(0, 1);
        let mut duplicate_capability = canonical.clone();
        let last = duplicate_capability.required_capabilities.len() - 1;
        duplicate_capability.required_capabilities[last] =
            duplicate_capability.required_capabilities[0].clone();
        let mut unknown_recipe = canonical.clone();
        unknown_recipe.recipe_id = "unknown-recipe".to_string();
        let mut invalid_artifact = canonical.clone();
        invalid_artifact.artifact_type = "contains whitespace".to_string();
        candidates.extend([
            (recipe_request(&canonical), wrong_renderer),
            (recipe_request(&canonical), wrong_schema),
            (recipe_request(&canonical), missing_capability),
            (recipe_request(&canonical), extra_capability),
            (recipe_request(&canonical), reordered_capabilities),
            (recipe_request(&canonical), duplicate_capability),
            (recipe_request(&canonical), unknown_recipe),
            (recipe_request(&canonical), invalid_artifact),
        ]);

        for (request, registration) in candidates {
            let audit = Arc::new(MemoryAudit::default());
            let runtime =
                IxForegroundRuntime::new(audit.clone(), &policy_authority(), Arc::new(TestPolicy))
                    .expect("policy authority configures runtime");
            let owner = owner("wayne");
            let result = start_recipe_result(&runtime, &owner, request, registration);
            assert_eq!(result, Err(IxContractError::InvalidRecipeRegistration));
            assert!(audit.events().is_empty());
            assert!(lock_runs(&runtime.runs).is_empty());
        }

        let canonical_related = IxRelatedArtifactRef {
            artifact_id: "brief-1".to_string(),
            artifact_type: canonical.artifact_type().to_string(),
            content_schema_version: canonical.content_schema_version().to_string(),
            renderer_key: canonical.renderer_key().to_string(),
            revision: 1,
        };
        let mut wrong_related_type = canonical_related.clone();
        wrong_related_type.artifact_type = "substituted.artifact@1".to_string();
        let mut wrong_related_schema = canonical_related.clone();
        wrong_related_schema.content_schema_version = "product.presentation@1".to_string();
        let mut wrong_related_renderer = canonical_related;
        wrong_related_renderer.renderer_key = "pds.ix.recipe.substituted@1".to_string();

        for related_artifact in [
            wrong_related_type,
            wrong_related_schema,
            wrong_related_renderer,
        ] {
            let audit = Arc::new(MemoryAudit::default());
            let runtime =
                IxForegroundRuntime::new(audit.clone(), &policy_authority(), Arc::new(TestPolicy))
                    .expect("policy authority configures runtime");
            let run_owner = owner("wayne");
            let mut continuation = recipe_request(&canonical);
            continuation.related_artifact = Some(related_artifact);
            assert_eq!(
                start_recipe_result(&runtime, &run_owner, continuation, canonical.clone()),
                Err(IxContractError::InvalidRecipeRegistration)
            );
            assert!(audit.events().is_empty());
            assert!(lock_runs(&runtime.runs).is_empty());
        }

        for registered_artifacts in [
            [IxArtifactBinding::new(
                "substituted.artifact@1",
                canonical.content_schema_version(),
                canonical.renderer_key(),
            )
            .expect("stable substituted tuple")]
            .into_iter()
            .collect(),
            [
                canonical.artifact_binding().expect("canonical binding"),
                IxArtifactBinding::new(
                    "extra.artifact@1",
                    canonical.content_schema_version(),
                    canonical.renderer_key(),
                )
                .expect("stable extra tuple"),
            ]
            .into_iter()
            .collect(),
        ] {
            let audit = Arc::new(MemoryAudit::default());
            let runtime =
                IxForegroundRuntime::new(audit.clone(), &policy_authority(), Arc::new(TestPolicy))
                    .expect("policy authority configures runtime");
            let owner = owner("wayne");
            let policy = IxRunPolicy {
                intent_label: "Canonical IX recipe".to_string(),
                runner: IxPrincipalBinding::from(&runner()),
                registered_artifacts,
                recipe_registration: Some(canonical.clone()),
            };
            assert_eq!(
                start_with_policy_result(&runtime, &owner, recipe_request(&canonical), policy,),
                Err(IxContractError::InvalidRecipeRegistration)
            );
            assert!(audit.events().is_empty());
            assert!(lock_runs(&runtime.runs).is_empty());
        }
    }

    #[test]
    fn legacy_policy_remains_unbound_and_source_compatible() {
        let (runtime, _audit, owner, _runner, run_id) = started_runtime();
        let snapshot = runtime.snapshot(&owner, &run_id).expect("legacy snapshot");
        assert!(snapshot.recipe_registration.is_none());
        assert_eq!(snapshot.registered_artifacts.len(), 1);
        let serialized = serde_json::to_value(&snapshot).expect("legacy snapshot serializes");
        assert!(
            serialized.get("recipeRegistration").is_none(),
            "the additive recipe field must remain absent from legacy snapshot bytes"
        );
    }

    #[test]
    fn recipe_bound_artifact_tuple_substitution_is_atomic() {
        let registration = super::super::recipe::test_registrations()
            .into_iter()
            .next()
            .expect("canonical recipe");
        let audit = Arc::new(MemoryAudit::default());
        let runtime = IxForegroundRuntime::new(audit, &policy_authority(), Arc::new(TestPolicy))
            .expect("policy authority configures runtime");
        let owner = owner("wayne");
        let runner = runner();
        let receipt = start_recipe_result(
            &runtime,
            &owner,
            recipe_request(&registration),
            registration.clone(),
        )
        .expect("canonical recipe starts");
        let authority = authority(&runtime, &owner, &receipt.run_id);
        runtime
            .resolve_context(&runner, &receipt.run_id, &authority, context())
            .expect("context resolves");
        let before = runtime
            .snapshot(&owner, &receipt.run_id)
            .expect("snapshot before rejected effects");

        let mut wrong_type = recipe_artifact(&registration, 1);
        wrong_type.artifact_type = "substituted.artifact@1".to_string();
        assert_eq!(
            runtime.publish_artifact(&runner, &receipt.run_id, &authority, wrong_type),
            Err(IxContractError::UnknownArtifactType)
        );

        let mut wrong_renderer = recipe_artifact(&registration, 1);
        wrong_renderer.renderer_key = "pds.ix.recipe.substituted@1".to_string();
        assert_eq!(
            runtime.publish_artifact(&runner, &receipt.run_id, &authority, wrong_renderer),
            Err(IxContractError::UnknownArtifactType)
        );

        let mut wrong_schema = recipe_artifact(&registration, 1);
        wrong_schema.content_schema_version = "product.presentation@1".to_string();
        assert_eq!(
            runtime.publish_artifact(&runner, &receipt.run_id, &authority, wrong_schema),
            Err(IxContractError::InvalidPresentation)
        );
        assert_eq!(
            runtime.snapshot(&owner, &receipt.run_id).expect("snapshot"),
            before
        );
    }

    #[test]
    fn foreground_start_requires_an_authenticated_human_boundary() {
        assert!(matches!(
            IxForegroundRuntime::new(
                Arc::new(MemoryAudit::default()),
                &owner("not-a-policy-service"),
                Arc::new(TestPolicy),
            ),
            Err(IxContractError::UnauthorizedPolicyAuthority)
        ));
        let runtime = IxForegroundRuntime::new(
            Arc::new(MemoryAudit::default()),
            &policy_authority(),
            Arc::new(TestPolicy),
        )
        .expect("policy authority configures runtime");
        let mut unauthenticated = owner("wayne");
        unauthenticated.token.clear();
        assert_eq!(
            start_result(&runtime, &unauthenticated, request()),
            Err(IxContractError::UnauthorizedActor)
        );
    }

    #[test]
    fn every_human_and_runner_entry_point_rechecks_live_authentication() {
        let (runtime, _audit, run_owner, runner, run_id) = started_runtime();
        let authority = authority(&runtime, &run_owner, &run_id);
        runtime
            .resolve_context(&runner, &run_id, &authority, context())
            .expect("qualified runner resolves context");

        let mut unauthenticated_owner = run_owner.clone();
        unauthenticated_owner.token.clear();
        assert_eq!(
            runtime.snapshot(&unauthenticated_owner, &run_id),
            Err(IxContractError::UnauthorizedActor)
        );
        assert_eq!(
            runtime.cancel(
                &request_context(),
                &unauthenticated_owner,
                &run_id,
                "cancel-request-1"
            ),
            Err(IxContractError::UnauthorizedActor)
        );

        let mut unqualified_runner = runner.clone();
        unqualified_runner.scopes.clear();
        assert_eq!(
            runtime.change_phase(
                &unqualified_runner,
                &run_id,
                &authority,
                IxPhase::Gathering,
                "An unqualified runner cannot progress the run."
            ),
            Err(IxContractError::UnauthorizedActor)
        );
        assert!(matches!(
            runtime.fail(
                &unqualified_runner,
                &run_id,
                "An unqualified runner cannot assert terminal truth."
            ),
            Err(IxContractError::UnauthorizedActor)
        ));
    }

    #[test]
    fn context_reconciliation_invalidates_stale_effects_across_every_authority_dimension() {
        let (runtime, _audit, mut run_owner, runner, run_id) = started_runtime();
        let initial = authority(&runtime, &run_owner, &run_id);
        runtime
            .resolve_context(&runner, &run_id, &initial, context())
            .expect("initial context");

        let release_changed = custom_authorization_context(
            &run_owner,
            2,
            "nexus-release-2",
            "session-1",
            IxDataClassification::Confidential,
            IxConsentState::Granted,
            IxEgressPosture::LocalOnly,
        );
        assert_eq!(
            propose_and_reconcile(&runtime, &run_owner, &run_id, release_changed),
            Err(IxContractError::ImmutableRunIdentityChanged)
        );
        assert_eq!(
            runtime
                .snapshot(&run_owner, &run_id)
                .expect("rejected release change is atomic")
                .release_id,
            "nexus-release-1"
        );

        let policy_only_changed = custom_authorization_context(
            &run_owner,
            2,
            "nexus-release-1",
            "session-1",
            IxDataClassification::Confidential,
            IxConsentState::Granted,
            IxEgressPosture::LocalOnly,
        );
        propose_and_reconcile(&runtime, &run_owner, &run_id, policy_only_changed)
            .expect("new policy decision is a revision-mutable authority change");
        assert_eq!(
            runtime.change_phase(
                &runner,
                &run_id,
                &initial,
                IxPhase::Gathering,
                "Stale policy authority must not execute."
            ),
            Err(IxContractError::StaleAuthorization)
        );

        let session_changed = custom_authorization_context(
            &run_owner,
            3,
            "nexus-release-1",
            "session-2",
            IxDataClassification::Confidential,
            IxConsentState::Granted,
            IxEgressPosture::LocalOnly,
        );
        propose_and_reconcile(&runtime, &run_owner, &run_id, session_changed)
            .expect("session reconciles");

        run_owner.roles.push("nexus-approver".to_string());
        let roles_changed = custom_authorization_context(
            &run_owner,
            4,
            "nexus-release-1",
            "session-2",
            IxDataClassification::Confidential,
            IxConsentState::Granted,
            IxEgressPosture::LocalOnly,
        );
        propose_and_reconcile(&runtime, &run_owner, &run_id, roles_changed)
            .expect("roles reconcile");

        run_owner.scopes.push("nexus:approve".to_string());
        let scopes_changed = custom_authorization_context(
            &run_owner,
            5,
            "nexus-release-1",
            "session-2",
            IxDataClassification::Confidential,
            IxConsentState::Granted,
            IxEgressPosture::LocalOnly,
        );
        propose_and_reconcile(&runtime, &run_owner, &run_id, scopes_changed)
            .expect("scopes reconcile");

        let classification_changed = custom_authorization_context(
            &run_owner,
            6,
            "nexus-release-1",
            "session-2",
            IxDataClassification::Restricted,
            IxConsentState::Granted,
            IxEgressPosture::LocalOnly,
        );
        propose_and_reconcile(&runtime, &run_owner, &run_id, classification_changed)
            .expect("classification reconciles");

        let egress_changed = custom_authorization_context(
            &run_owner,
            7,
            "nexus-release-1",
            "session-2",
            IxDataClassification::Restricted,
            IxConsentState::Granted,
            IxEgressPosture::Approved {
                policy_id: "policy-1".to_string(),
                decision_id: "decision-1".to_string(),
                destination: "pds-health-ai".to_string(),
            },
        );
        propose_and_reconcile(&runtime, &run_owner, &run_id, egress_changed)
            .expect("egress reconciles");

        let consent_withdrawn = custom_authorization_context(
            &run_owner,
            8,
            "nexus-release-1",
            "session-2",
            IxDataClassification::Restricted,
            IxConsentState::Withdrawn,
            IxEgressPosture::Approved {
                policy_id: "policy-1".to_string(),
                decision_id: "decision-1".to_string(),
                destination: "pds-health-ai".to_string(),
            },
        );
        propose_and_reconcile(&runtime, &run_owner, &run_id, consent_withdrawn)
            .expect("consent reconciles");
        let withdrawn = authority(&runtime, &run_owner, &run_id);
        assert_eq!(
            runtime.publish_artifact(&runner, &run_id, &withdrawn, artifact(1)),
            Err(IxContractError::EffectNotAuthorized)
        );

        runtime
            .cancel(
                &request_context(),
                &run_owner,
                &run_id,
                "cancel-after-reconcile",
            )
            .expect("cancellation remains available");
        for stage in [
            IxCancellationStage::Stopping,
            IxCancellationStage::Draining,
            IxCancellationStage::Stopped,
        ] {
            runtime
                .cancellation_progress(&runner, &run_id, "cancel-after-reconcile", stage)
                .expect("cancellation progresses");
        }
        runtime
            .confirm_cancelled(&runner, &run_id, "cancel-after-reconcile")
            .expect("cancel terminal remains available");

        let (failed_runtime, _audit, failed_owner, failed_runner, failed_run_id) =
            started_runtime();
        let denied = custom_authorization_context(
            &failed_owner,
            2,
            "nexus-release-1",
            "session-1",
            IxDataClassification::Confidential,
            IxConsentState::Denied,
            IxEgressPosture::LocalOnly,
        );
        propose_and_reconcile(&failed_runtime, &failed_owner, &failed_run_id, denied)
            .expect("denied consent reconciles");
        failed_runtime
            .fail(
                &failed_runner,
                &failed_run_id,
                "Authorization changed before effect execution.",
            )
            .expect("truthful failure remains available");
    }

    #[test]
    fn human_context_requests_cannot_forge_policy_approval_or_downgrade_truth() {
        let (runtime, _audit, run_owner, _runner, run_id) = started_runtime();
        let proposal = IxContextProposal::from_authenticated_human(
            "human-downgrade-request",
            1,
            "nexus-release-1",
            "session-2",
            &run_owner,
            "Request a lower classification and external destination.",
            Some(IxDataClassification::Public),
            Some(IxConsentState::Granted),
            Some("attacker-destination".to_string()),
        )
        .expect("human proposal is explicit but not authoritative");
        let proposal_json = serde_json::to_value(&proposal).expect("proposal serializes");
        assert!(proposal_json.get("policyDecision").is_none());
        assert!(proposal_json.get("decisionId").is_none());
        assert!(proposal_json.get("policyId").is_none());
        runtime
            .request_context_reconciliation(&run_owner, &run_id, proposal)
            .expect("proposal is retained as a request");
        let before_decision = runtime.snapshot(&run_owner, &run_id).expect("snapshot");
        assert_eq!(
            before_decision.authorization_context.classification(),
            IxDataClassification::Confidential
        );
        assert!(before_decision.pending_context_proposal.is_some());
        runtime
            .reconcile_context(&run_id)
            .expect("runtime invokes only its configured policy implementation");
        let decided = runtime.snapshot(&run_owner, &run_id).expect("snapshot");
        assert_eq!(
            decided.authorization_context.classification(),
            IxDataClassification::Confidential
        );
        assert_eq!(
            decided.authorization_context.egress(),
            &IxEgressPosture::LocalOnly
        );
        assert!(decided.pending_context_proposal.is_none());
        assert_eq!(
            decided.authorization_context.policy_decision().authority(),
            &IxPrincipalBinding::from(&policy_authority())
        );
    }

    #[test]
    fn reconciliation_requires_fresh_context_before_any_further_effect() {
        let (runtime, _audit, run_owner, runner, run_id) = started_runtime();
        let initial = authority(&runtime, &run_owner, &run_id);
        runtime
            .resolve_context(&runner, &run_id, &initial, context())
            .expect("initial context resolves");

        let next = custom_authorization_context(
            &run_owner,
            2,
            "nexus-release-1",
            "session-2",
            IxDataClassification::Confidential,
            IxConsentState::Granted,
            IxEgressPosture::LocalOnly,
        );
        propose_and_reconcile(&runtime, &run_owner, &run_id, next)
            .expect("session reconciliation succeeds");
        let reconciled = runtime.snapshot(&run_owner, &run_id).expect("snapshot");
        let current = reconciled.authorization_context.binding();
        assert_eq!(reconciled.release_id, "nexus-release-1");
        assert!(reconciled.context.is_none());
        assert!(reconciled.context_authority.is_none());
        assert_eq!(
            runtime.change_phase(
                &runner,
                &run_id,
                &current,
                IxPhase::Gathering,
                "Context must be fresh before this effect."
            ),
            Err(IxContractError::InvalidLifecycle)
        );
        assert_eq!(
            runtime.resolve_context(&runner, &run_id, &initial, context()),
            Err(IxContractError::StaleAuthorization)
        );
        runtime
            .resolve_context(&runner, &run_id, &current, context())
            .expect("current authority resolves fresh context");
        let resolved = runtime.snapshot(&run_owner, &run_id).expect("snapshot");
        assert_eq!(resolved.context_authority.as_ref(), Some(&current));
        assert_eq!(resolved.release_id, "nexus-release-1");
        assert_eq!(
            runtime.resolve_context(&runner, &run_id, &current, context()),
            Err(IxContractError::InvalidLifecycle),
            "current context cannot be replaced arbitrarily"
        );
        runtime
            .change_phase(
                &runner,
                &run_id,
                &current,
                IxPhase::Gathering,
                "Fresh context permits the next effect.",
            )
            .expect("effect resumes after fresh context");

        let checkpoint = runtime
            .replay_checkpoint(&run_owner, &run_id, None)
            .expect("checkpoint");
        let replay = runtime
            .replay_after(&run_owner, &run_id, None)
            .expect("replay");
        IxReplayVerifier::verify(&replay, &checkpoint).expect("replay is exact");
        assert_eq!(replay.expected_snapshot.release_id, "nexus-release-1");
        assert_eq!(
            replay.expected_snapshot.context_authority.as_ref(),
            Some(&current)
        );

        let (waiting_runtime, _audit, waiting_owner, waiting_runner, waiting_run_id) =
            started_runtime();
        let waiting_initial = authority(&waiting_runtime, &waiting_owner, &waiting_run_id);
        waiting_runtime
            .resolve_context(
                &waiting_runner,
                &waiting_run_id,
                &waiting_initial,
                context(),
            )
            .expect("waiting run resolves initial context");
        waiting_runtime
            .wait_for_user(
                &waiting_runner,
                &waiting_run_id,
                &waiting_initial,
                "Confirm the next step.",
            )
            .expect("run waits for the owner");
        let waiting_next = custom_authorization_context(
            &waiting_owner,
            2,
            "nexus-release-1",
            "session-2",
            IxDataClassification::Confidential,
            IxConsentState::Granted,
            IxEgressPosture::LocalOnly,
        );
        propose_and_reconcile(
            &waiting_runtime,
            &waiting_owner,
            &waiting_run_id,
            waiting_next,
        )
        .expect("waiting context reconciles");
        let waiting_snapshot = waiting_runtime
            .snapshot(&waiting_owner, &waiting_run_id)
            .expect("waiting snapshot");
        let waiting_current = waiting_snapshot.authorization_context.binding();
        assert_eq!(waiting_snapshot.state, IxRunState::WaitingForUser);
        assert!(waiting_snapshot.context.is_none());
        assert_eq!(
            waiting_runtime.resume_after_user_input(
                &waiting_owner,
                &waiting_run_id,
                &waiting_current,
                IxPhase::Gathering,
                "Fresh context is still required."
            ),
            Err(IxContractError::InvalidLifecycle)
        );
        waiting_runtime
            .resolve_context(
                &waiting_runner,
                &waiting_run_id,
                &waiting_current,
                context(),
            )
            .expect("waiting run re-resolves without resuming");
        assert_eq!(
            waiting_runtime
                .snapshot(&waiting_owner, &waiting_run_id)
                .expect("waiting snapshot after resolve")
                .state,
            IxRunState::WaitingForUser
        );
    }

    #[test]
    fn reconciliation_rejects_release_substitution_and_pending_proposal_overwrite() {
        let (runtime, _audit, run_owner, _runner, run_id) = started_runtime();
        let first = IxContextProposal::from_authenticated_human(
            "first-proposal",
            1,
            "nexus-release-1",
            "session-2",
            &run_owner,
            "Correct the session binding.",
            None,
            None,
            None,
        )
        .expect("first proposal");
        runtime
            .request_context_reconciliation(&run_owner, &run_id, first.clone())
            .expect("first proposal is retained");
        let before = runtime.snapshot(&run_owner, &run_id).expect("snapshot");

        let second = IxContextProposal::from_authenticated_human(
            "second-proposal",
            1,
            "nexus-release-1",
            "session-3",
            &run_owner,
            "A distinct proposal cannot overwrite the first.",
            None,
            None,
            None,
        )
        .expect("second proposal");
        assert_eq!(
            runtime.request_context_reconciliation(&run_owner, &run_id, second),
            Err(IxContractError::ContextProposalAlreadyPending)
        );
        let after = runtime.snapshot(&run_owner, &run_id).expect("snapshot");
        assert_eq!(after, before);
        assert_eq!(after.pending_context_proposal.as_ref(), Some(&first));

        let other_runtime = started_runtime();
        let release_change = IxContextProposal::from_authenticated_human(
            "release-substitution",
            1,
            "nexus-release-2",
            "session-1",
            &other_runtime.2,
            "A release change requires a new run.",
            None,
            None,
            None,
        )
        .expect("well-formed but unauthorized release substitution");
        assert_eq!(
            other_runtime.0.request_context_reconciliation(
                &other_runtime.2,
                &other_runtime.4,
                release_change
            ),
            Err(IxContractError::ImmutableRunIdentityChanged)
        );
    }

    #[test]
    fn policy_decision_changes_are_semantic_but_exact_replays_are_noops() {
        let run_owner = owner("wayne");
        let initial = authorization_context_with_policy(
            &run_owner,
            1,
            "nexus-release-1",
            "session-1",
            &policy_authority(),
            "decision-a",
            "ix-policy-v1",
            IxDataClassification::Confidential,
            IxConsentState::Granted,
            IxEgressPosture::LocalOnly,
        );
        let exact = authorization_context_with_policy(
            &run_owner,
            2,
            "nexus-release-1",
            "session-1",
            &policy_authority(),
            "decision-a",
            "ix-policy-v1",
            IxDataClassification::Confidential,
            IxConsentState::Granted,
            IxEgressPosture::LocalOnly,
        );
        let changed = authorization_context_with_policy(
            &run_owner,
            2,
            "nexus-release-1",
            "session-1",
            &policy_authority(),
            "decision-b",
            "ix-policy-v2",
            IxDataClassification::Confidential,
            IxConsentState::Granted,
            IxEgressPosture::LocalOnly,
        );
        assert!(exact.same_authorization_semantics(&initial));
        assert!(!changed.same_authorization_semantics(&initial));
    }

    #[test]
    fn foreground_runs_are_bounded_per_authenticated_owner() {
        let runtime = IxForegroundRuntime::new(
            Arc::new(MemoryAudit::default()),
            &policy_authority(),
            Arc::new(TestPolicy),
        )
        .expect("policy authority configures runtime");
        let first_owner = owner("wayne");
        for _ in 0..IX_MAX_ACTIVE_RUNS_PER_OWNER {
            start_result(&runtime, &first_owner, request())
                .expect("owner remains within active-run bound");
        }
        assert_eq!(
            start_result(&runtime, &first_owner, request()),
            Err(IxContractError::ResourceLimitExceeded)
        );
        start_result(&runtime, &owner("other"), request())
            .expect("capacity is partitioned by owner");
    }

    fn drive_to_effect_commit_limit(
        runtime: &IxForegroundRuntime,
        owner: &UserAuth,
        runner: &UserAuth,
        run_id: &str,
        authority: &IxExecutionAuthority,
    ) {
        runtime
            .resolve_context(runner, run_id, authority, context())
            .expect("context");
        loop {
            let snapshot = runtime.snapshot(owner, run_id).expect("snapshot");
            if snapshot.task_revision as usize == IX_EFFECT_COMMIT_LIMIT {
                break;
            }
            let phase = if snapshot.phase == Some(IxPhase::Gathering) {
                IxPhase::Checking
            } else {
                IxPhase::Gathering
            };
            runtime
                .change_phase(
                    runner,
                    run_id,
                    authority,
                    phase,
                    "Bounded work advances without consuming the terminal reserve.",
                )
                .expect("effect stays below the reserve");
        }
        assert_retention_accounting(runtime);
    }

    #[test]
    fn effect_boundary_always_preserves_exact_cancel_and_failure_capacity() {
        let (runtime, _audit, owner, runner, run_id) = started_runtime();
        let run_authority = authority(&runtime, &owner, &run_id);
        drive_to_effect_commit_limit(&runtime, &owner, &runner, &run_id, &run_authority);
        assert_eq!(
            runtime.change_phase(
                &runner,
                &run_id,
                &run_authority,
                IxPhase::Composing,
                "This effect would consume the cancellation reserve."
            ),
            Err(IxContractError::ResourceLimitExceeded)
        );
        runtime
            .cancel(&request_context(), &owner, &run_id, "boundary-cancel")
            .expect("accepted uses reserved commit one");
        for stage in [
            IxCancellationStage::Stopping,
            IxCancellationStage::Draining,
            IxCancellationStage::Stopped,
        ] {
            runtime
                .cancellation_progress(&runner, &run_id, "boundary-cancel", stage)
                .expect("reserved cancellation stage commits");
        }
        runtime
            .confirm_cancelled(&runner, &run_id, "boundary-cancel")
            .expect("confirmation and cancelled terminal share final commit");
        let terminal = runtime
            .snapshot(&owner, &run_id)
            .expect("terminal snapshot");
        assert_eq!(terminal.task_revision as usize, IX_MAX_COMMITS);
        assert_eq!(
            terminal.terminal_outcome,
            Some(IxTerminalOutcome::Cancelled)
        );

        let (failed_runtime, _audit, failed_owner, failed_runner, failed_run_id) =
            started_runtime();
        let failed_authority = authority(&failed_runtime, &failed_owner, &failed_run_id);
        drive_to_effect_commit_limit(
            &failed_runtime,
            &failed_owner,
            &failed_runner,
            &failed_run_id,
            &failed_authority,
        );
        failed_runtime
            .fail(
                &failed_runner,
                &failed_run_id,
                "The bounded run stopped without claiming a result.",
            )
            .expect("truthful failure uses reserved terminal capacity");
        assert_eq!(
            failed_runtime
                .snapshot(&failed_owner, &failed_run_id)
                .expect("failed snapshot")
                .terminal_outcome,
            Some(IxTerminalOutcome::Failed)
        );
    }

    #[test]
    fn byte_boundary_stops_effects_before_reserved_cancellation_capacity() {
        let (runtime, _audit, owner, runner, run_id) = started_runtime();
        let run_authority = authority(&runtime, &owner, &run_id);
        runtime
            .resolve_context(&runner, &run_id, &run_authority, context())
            .expect("context");
        let mut revision = 1u64;
        loop {
            let before_snapshot = runtime.snapshot(&owner, &run_id).expect("snapshot");
            let before_retained = runtime.retention.retained_bytes.load(Ordering::SeqCst);
            let before_reserved = runtime
                .retention
                .reserved_terminal_bytes
                .load(Ordering::SeqCst);
            let mut large = artifact(revision);
            large.presentation = large_pds_presentation("brief-1", revision);
            match runtime.publish_artifact(&runner, &run_id, &run_authority, large) {
                Ok(_) => revision += 1,
                Err(IxContractError::ResourceLimitExceeded) => {
                    assert_eq!(
                        runtime.snapshot(&owner, &run_id).expect("snapshot"),
                        before_snapshot,
                        "retained-byte failure is atomic"
                    );
                    assert_eq!(
                        runtime.retention.retained_bytes.load(Ordering::SeqCst),
                        before_retained
                    );
                    assert_eq!(
                        runtime
                            .retention
                            .reserved_terminal_bytes
                            .load(Ordering::SeqCst),
                        before_reserved
                    );
                    break;
                }
                Err(error) => panic!("unexpected byte-boundary error: {error:?}"),
            }
        }
        assert_retention_accounting(&runtime);
        assert!(
            runtime
                .snapshot(&owner, &run_id)
                .expect("snapshot")
                .task_revision
                < IX_EFFECT_COMMIT_LIMIT as u64,
            "the byte bound is independent of the commit-count bound"
        );
        runtime
            .cancel(&request_context(), &owner, &run_id, "byte-boundary-cancel")
            .expect("cancel commit has reserved bytes");
        for stage in [
            IxCancellationStage::Stopping,
            IxCancellationStage::Draining,
            IxCancellationStage::Stopped,
        ] {
            runtime
                .cancellation_progress(&runner, &run_id, "byte-boundary-cancel", stage)
                .expect("cancellation progress has reserved bytes");
        }
        runtime
            .confirm_cancelled(&runner, &run_id, "byte-boundary-cancel")
            .expect("terminal cancellation has reserved bytes");
        assert_retention_accounting(&runtime);
    }

    #[test]
    fn runtime_aggregate_budget_is_atomic_across_runs_and_preserves_stop_capacity() {
        let runtime = IxForegroundRuntime::new(
            Arc::new(MemoryAudit::default()),
            &policy_authority(),
            Arc::new(TestPolicy),
        )
        .expect("policy authority configures runtime");
        let first_owner = owner("first");
        let second_owner = owner("second");
        let first = start_result(&runtime, &first_owner, request()).expect("first run");
        let _second = start_result(&runtime, &second_owner, request()).expect("second run");
        assert_retention_accounting(&runtime);
        let before_snapshot = runtime
            .snapshot(&first_owner, &first.run_id)
            .expect("first snapshot");
        let retained = runtime.retention.retained_bytes.load(Ordering::SeqCst);
        let reserved = runtime
            .retention
            .reserved_terminal_bytes
            .load(Ordering::SeqCst);
        runtime
            .retention
            .limit
            .store(retained + reserved, Ordering::SeqCst);
        let first_authority = before_snapshot.authorization_context.binding();
        assert_eq!(
            runtime.resolve_context(&runner(), &first.run_id, &first_authority, context()),
            Err(IxContractError::ResourceLimitExceeded)
        );
        assert_eq!(
            runtime
                .snapshot(&first_owner, &first.run_id)
                .expect("first snapshot"),
            before_snapshot
        );
        assert_eq!(
            runtime.retention.retained_bytes.load(Ordering::SeqCst),
            retained
        );
        assert_eq!(
            runtime
                .retention
                .reserved_terminal_bytes
                .load(Ordering::SeqCst),
            reserved
        );

        runtime
            .cancel(
                &request_context(),
                &first_owner,
                &first.run_id,
                "aggregate-cancel",
            )
            .expect("reserved aggregate capacity accepts cancellation");
        for stage in [
            IxCancellationStage::Stopping,
            IxCancellationStage::Draining,
            IxCancellationStage::Stopped,
        ] {
            runtime
                .cancellation_progress(&runner(), &first.run_id, "aggregate-cancel", stage)
                .expect("reserved aggregate capacity progresses cancellation");
        }
        runtime
            .confirm_cancelled(&runner(), &first.run_id, "aggregate-cancel")
            .expect("reserved aggregate capacity reaches terminal truth");
        assert_retention_accounting(&runtime);
    }

    #[test]
    fn lifecycle_wait_resume_and_terminal_are_authority_bound() {
        let (runtime, _audit, run_owner, runner, run_id) = started_runtime();
        let authority = authority(&runtime, &run_owner, &run_id);
        assert_eq!(
            runtime.resolve_context(&run_owner, &run_id, &authority, context()),
            Err(IxContractError::UnauthorizedActor)
        );
        runtime
            .resolve_context(&runner, &run_id, &authority, context())
            .expect("runner resolves context");
        runtime
            .change_phase(
                &runner,
                &run_id,
                &authority,
                IxPhase::Gathering,
                "Retrieve authorized supporting records.",
            )
            .expect("runner changes phase");
        assert_eq!(
            runtime.change_phase(
                &runner,
                &run_id,
                &authority,
                IxPhase::Gathering,
                "A no-op phase must not impersonate progress."
            ),
            Err(IxContractError::InvalidLifecycle)
        );
        runtime
            .publish_artifact(&runner, &run_id, &authority, artifact(1))
            .expect("first artifact");
        runtime
            .wait_for_user(
                &runner,
                &run_id,
                &authority,
                "Which assumption should I challenge?",
            )
            .expect("waits truthfully");
        let resumed = runtime
            .resume_after_user_input(
                &run_owner,
                &run_id,
                &authority,
                IxPhase::Checking,
                "Check the selected assumption.",
            )
            .expect("owner resumes");
        assert_eq!(resumed.events.len(), 2, "resume is one atomic envelope");
        runtime
            .finish(
                &runner,
                &run_id,
                &authority,
                IxTerminalOutcome::Completed,
                "The grounded brief is ready.",
            )
            .expect("runtime finishes");
        assert_eq!(
            runtime.publish_artifact(&runner, &run_id, &authority, artifact(2)),
            Err(IxContractError::InvalidLifecycle)
        );
    }

    #[test]
    fn artifact_revision_and_renderer_identity_are_exact() {
        let (runtime, _audit, owner, runner, run_id) = started_runtime();
        let authority = authority(&runtime, &owner, &run_id);
        runtime
            .resolve_context(&runner, &run_id, &authority, context())
            .expect("context");
        let mut arbitrary_first_renderer = artifact(1);
        arbitrary_first_renderer.renderer_key = "pds.ix.attacker_selected".to_string();
        assert_eq!(
            runtime.publish_artifact(&runner, &run_id, &authority, arbitrary_first_renderer),
            Err(IxContractError::UnknownArtifactType)
        );
        assert_eq!(
            runtime.publish_artifact(&runner, &run_id, &authority, artifact(2)),
            Err(IxContractError::InvalidArtifactRevision)
        );
        runtime
            .publish_artifact(&runner, &run_id, &authority, artifact(1))
            .expect("revision one");
        let mut wrong_renderer = artifact(2);
        wrong_renderer.renderer_key = "pds.ix.other".to_string();
        assert_eq!(
            runtime.publish_artifact(&runner, &run_id, &authority, wrong_renderer),
            Err(IxContractError::UnknownArtifactType)
        );
        runtime
            .publish_artifact(&runner, &run_id, &authority, artifact(2))
            .expect("exact next revision");
    }

    #[test]
    fn artifact_requires_exact_pds_presentation_identity_envelope() {
        fn presentation_identity(
            artifact: &mut IxArtifactRevision,
        ) -> &mut serde_json::Map<String, Value> {
            artifact
                .presentation
                .get_mut("identity")
                .and_then(Value::as_object_mut)
                .expect("golden fixture has identity")
        }

        let golden = IxArtifactRevision {
            artifact_id: "fixture-working-brief".to_string(),
            artifact_type: "nexus.ix.working_brief@1".to_string(),
            content_schema_version: PDS_IX_PRESENTATION_SCHEMA_VERSION.to_string(),
            renderer_key: "pds.ix.working_brief".to_string(),
            revision: 1,
            status: IxArtifactStatus::Ready,
            changed_region_ids: vec!["finding".to_string()],
            presentation: reviewed_pds_golden_presentation(),
        };
        golden
            .validate()
            .expect("the exact reviewed PDS golden fixture is accepted");

        let mut missing_identity = golden.clone();
        missing_identity
            .presentation
            .as_object_mut()
            .expect("golden fixture is an object")
            .remove("identity");
        assert_eq!(
            missing_identity.validate(),
            Err(IxContractError::InvalidPresentation)
        );

        let mut legacy_schema = golden.clone();
        let legacy_root = legacy_schema
            .presentation
            .as_object_mut()
            .expect("golden fixture is an object");
        legacy_root.remove("identity");
        legacy_root.insert(
            "schemaVersion".to_string(),
            Value::String(PDS_IX_PRESENTATION_SCHEMA_VERSION.to_string()),
        );
        assert_eq!(
            legacy_schema.validate(),
            Err(IxContractError::InvalidPresentation),
            "a legacy top-level schemaVersion cannot replace the bound nested identity"
        );

        let mut wrong_schema = golden.clone();
        presentation_identity(&mut wrong_schema).insert(
            "schemaVersion".to_string(),
            Value::String("pds.ix.presentation@2".to_string()),
        );
        assert_eq!(
            wrong_schema.validate(),
            Err(IxContractError::InvalidPresentation)
        );

        let mut mismatched_id = golden.clone();
        presentation_identity(&mut mismatched_id).insert(
            "presentationId".to_string(),
            Value::String("different-artifact".to_string()),
        );
        assert_eq!(
            mismatched_id.validate(),
            Err(IxContractError::InvalidPresentation)
        );

        let mut mismatched_revision = golden;
        presentation_identity(&mut mismatched_revision).insert("revision".to_string(), json!(2));
        assert_eq!(
            mismatched_revision.validate(),
            Err(IxContractError::InvalidPresentation)
        );
    }

    #[test]
    fn changed_regions_are_unique_and_reference_unique_presentation_regions() {
        let mut duplicate_changed = artifact(1);
        duplicate_changed
            .changed_region_ids
            .push("summary".to_string());
        assert_eq!(
            duplicate_changed.validate(),
            Err(IxContractError::DuplicateChangedRegion)
        );

        let mut unknown_changed = artifact(1);
        unknown_changed.changed_region_ids = vec!["not-in-presentation".to_string()];
        assert_eq!(
            unknown_changed.validate(),
            Err(IxContractError::UnknownChangedRegion)
        );

        let mut duplicate_presentation = artifact(1);
        let mut duplicate = duplicate_presentation.presentation["response"]["regions"][0].clone();
        duplicate["label"] = json!("Duplicate label with the same region ID");
        duplicate_presentation.presentation["response"]["regions"] = json!([
            duplicate_presentation.presentation["response"]["regions"][0].clone(),
            duplicate
        ]);
        assert_eq!(
            duplicate_presentation.validate(),
            Err(IxContractError::DuplicatePresentationRegion)
        );

        let mut omitted_inner_change = artifact(1);
        omitted_inner_change.presentation["response"]["regions"][0]
            .as_object_mut()
            .expect("fixture region is an object")
            .remove("changed");
        assert_eq!(
            omitted_inner_change.validate(),
            Err(IxContractError::ChangedRegionMismatch)
        );

        let mut unreported_inner_change = artifact(1);
        unreported_inner_change.changed_region_ids.clear();
        assert_eq!(
            unreported_inner_change.validate(),
            Err(IxContractError::ChangedRegionMismatch)
        );
    }

    #[test]
    fn presentation_byte_limit_rejects_without_materializing_a_second_payload() {
        let mut oversized = artifact(1);
        oversized.presentation["announcement"] = json!("x".repeat(IX_MAX_PRESENTATION_BYTES + 1));
        assert_eq!(
            oversized.validate(),
            Err(IxContractError::ResourceLimitExceeded)
        );
    }

    #[test]
    fn changed_region_references_use_the_canonical_pds_identifier_profile() {
        let mut unicode_region = artifact(1);
        unicode_region.presentation["response"]["regions"][0]["id"] = json!("résumé-😀");
        unicode_region.changed_region_ids = vec!["résumé-😀".to_string()];
        unicode_region
            .validate()
            .expect("a schema-valid PDS region ID can be named as changed");

        unicode_region.changed_region_ids = vec!["😀".repeat(129)];
        assert_eq!(
            unicode_region.validate(),
            Err(IxContractError::UnknownChangedRegion)
        );
    }

    #[test]
    fn related_artifact_continuation_preserves_identity_and_exact_revision() {
        let prior = IxRelatedArtifactRef {
            artifact_id: "brief-1".to_string(),
            artifact_type: "nexus.ix.working_brief@1".to_string(),
            content_schema_version: PDS_IX_PRESENTATION_SCHEMA_VERSION.to_string(),
            renderer_key: "pds.ix.working_brief".to_string(),
            revision: 7,
        };
        let audit = Arc::new(MemoryAudit::default());
        let runtime = IxForegroundRuntime::new(audit, &policy_authority(), Arc::new(TestPolicy))
            .expect("policy authority configures runtime");
        let owner = owner("wayne");
        let runner = runner();
        let mut continued_request = request();
        continued_request.related_artifact = Some(prior);
        let started =
            start_result(&runtime, &owner, continued_request).expect("continuation starts");
        let authority = authority(&runtime, &owner, &started.run_id);
        runtime
            .resolve_context(&runner, &started.run_id, &authority, context())
            .expect("context");

        assert_eq!(
            runtime.publish_artifact(&runner, &started.run_id, &authority, artifact(1)),
            Err(IxContractError::ArtifactIdentityChanged)
        );
        runtime
            .publish_artifact(&runner, &started.run_id, &authority, artifact(8))
            .expect("exact continuation revision");
        let mut second_artifact = artifact(1);
        second_artifact.artifact_id = "brief-2".to_string();
        second_artifact.presentation = pds_presentation("brief-2", 1);
        runtime
            .publish_artifact(&runner, &started.run_id, &authority, second_artifact)
            .expect("new artifact starts at revision one after continuation is present");
    }

    #[test]
    fn cancellation_fence_blocks_effects_and_replay_keeps_atomic_envelopes() {
        let (runtime, audit, owner, runner, run_id) = started_runtime();
        let authority = authority(&runtime, &owner, &run_id);
        runtime
            .resolve_context(&runner, &run_id, &authority, context())
            .expect("context");
        runtime
            .publish_artifact(&runner, &run_id, &authority, artifact(1))
            .expect("partial work");
        let before_cancel = runtime.snapshot(&owner, &run_id).expect("snapshot").cursor;
        let cancel = runtime
            .cancel(&request_context(), &owner, &run_id, "cancel-request-1")
            .expect("cancel accepted");
        assert_eq!(cancel.disposition, IxCancelDisposition::Accepted);
        assert!(cancel.audit_retained);
        assert_eq!(
            runtime.publish_artifact(&runner, &run_id, &authority, artifact(2)),
            Err(IxContractError::InvalidLifecycle)
        );
        runtime
            .cancellation_progress(
                &runner,
                &run_id,
                "cancel-request-1",
                IxCancellationStage::Stopping,
            )
            .expect("stopping");
        runtime
            .cancellation_progress(
                &runner,
                &run_id,
                "cancel-request-1",
                IxCancellationStage::Draining,
            )
            .expect("draining");
        assert_eq!(
            runtime.confirm_cancelled(&runner, &run_id, "cancel-request-1"),
            Err(IxContractError::InvalidCancellationProgress)
        );
        runtime
            .cancellation_progress(
                &runner,
                &run_id,
                "cancel-request-1",
                IxCancellationStage::Stopped,
            )
            .expect("stopped");
        runtime
            .confirm_cancelled(&runner, &run_id, "cancel-request-1")
            .expect("confirmed terminal");
        let replay = runtime
            .replay_after(&owner, &run_id, Some(&before_cancel))
            .expect("bounded replay");
        let checkpoint = runtime
            .replay_checkpoint(&owner, &run_id, Some(&before_cancel))
            .expect("separately retained checkpoint");
        assert!(replay
            .envelopes
            .iter()
            .all(|envelope| !envelope.events.is_empty()));
        assert_eq!(
            replay
                .envelopes
                .last()
                .expect("terminal envelope")
                .events
                .len(),
            2,
            "confirmation and terminal are never split"
        );
        IxReplayVerifier::verify(&replay, &checkpoint).expect("replay is equivalent");
        assert_eq!(
            runtime.replay_after(&owner, &run_id, Some(&format!("{run_id}:4"))),
            Err(IxContractError::InvalidReplayCursor)
        );
        assert_eq!(audit.events().len(), 2);
    }

    #[test]
    fn cancel_is_idempotent_and_audit_failure_does_not_reverse_it() {
        let audit = Arc::new(MemoryAudit::default());
        let runtime =
            IxForegroundRuntime::new(audit.clone(), &policy_authority(), Arc::new(TestPolicy))
                .expect("policy authority configures runtime");
        let owner = owner("wayne");
        let started = start_result(&runtime, &owner, request()).expect("run starts");
        audit.fail.store(true, Ordering::SeqCst);
        let accepted = runtime
            .cancel(
                &request_context(),
                &owner,
                &started.run_id,
                "cancel-request-1",
            )
            .expect("cancel commits");
        assert_eq!(accepted.disposition, IxCancelDisposition::Accepted);
        assert!(!accepted.audit_retained);
        let duplicate = runtime
            .cancel(
                &request_context(),
                &owner,
                &started.run_id,
                "cancel-request-1",
            )
            .expect("exact retry");
        assert_eq!(duplicate.disposition, IxCancelDisposition::AlreadyRequested);
        assert_eq!(
            runtime.cancel(
                &request_context(),
                &owner,
                &started.run_id,
                "different-command"
            ),
            Err(IxContractError::CancellationConflict)
        );
    }

    #[tokio::test]
    async fn accepted_cancel_notifies_only_the_bound_runner() {
        let (runtime, _audit, owner, runner, run_id) = started_runtime();
        let mut signal = runtime
            .cancellation_signal(&runner, &run_id)
            .expect("bound runner receives signal");
        assert_eq!(signal.state(), IxCancellationSignalState::Active);
        assert!(matches!(
            runtime.cancellation_signal(
                &UserAuth::agent(
                    "tenant-1",
                    "different-runner",
                    vec![],
                    vec!["appfw:ix.progress".to_string()]
                ),
                &run_id
            ),
            Err(IxContractError::UnauthorizedActor)
        ));
        runtime
            .cancel(&request_context(), &owner, &run_id, "cancel-request-1")
            .expect("cancel accepted");
        assert_eq!(
            signal.cancelled().await,
            IxCancellationSignalState::Requested
        );
        let late_signal = runtime
            .cancellation_signal(&runner, &run_id)
            .expect("late subscription retains cancellation");
        assert_eq!(late_signal.state(), IxCancellationSignalState::Requested);
    }

    #[test]
    fn replay_is_byte_stable_and_principal_bound() {
        let (runtime, _audit, run_owner, runner, run_id) = started_runtime();
        let authority = authority(&runtime, &run_owner, &run_id);
        runtime
            .resolve_context(&runner, &run_id, &authority, context())
            .expect("context");
        runtime
            .publish_artifact(&runner, &run_id, &authority, artifact(1))
            .expect("artifact");
        let first = runtime
            .replay_after(&run_owner, &run_id, None)
            .expect("full replay");
        let retained_artifact = first
            .expected_snapshot
            .artifacts
            .get("brief-1")
            .expect("artifact is retained in the replay result");
        assert_eq!(retained_artifact.changed_region_ids, ["summary"]);
        assert_eq!(
            retained_artifact.presentation["response"]["regions"][0]["changed"],
            json!(true)
        );
        let second = runtime
            .replay_after(&run_owner, &run_id, None)
            .expect("same replay");
        assert_eq!(
            serde_json::to_vec(&first).expect("serialize"),
            serde_json::to_vec(&second).expect("serialize")
        );
        assert_eq!(
            runtime.replay_after(&owner("other"), &run_id, None),
            Err(IxContractError::RunNotFound)
        );
        let encoded = serde_json::to_string(&first).expect("replay serializes");
        assert!(!encoded.contains("secret-token"));
        assert!(!encoded.contains("What changed?"));
        let audit_json = serde_json::to_string(&_audit.events()).expect("audit serializes");
        assert!(!audit_json.contains("What changed?"));
        assert!(!audit_json.contains("secret-token"));
    }

    #[test]
    fn replay_rejects_tampering_in_payload_authority_and_commit_boundary() {
        let (runtime, _audit, owner, runner, run_id) = started_runtime();
        let authority = authority(&runtime, &owner, &run_id);
        runtime
            .resolve_context(&runner, &run_id, &authority, context())
            .expect("context");
        runtime
            .publish_artifact(&runner, &run_id, &authority, artifact(1))
            .expect("artifact");
        let replay = runtime.replay_after(&owner, &run_id, None).expect("replay");
        let checkpoint = runtime
            .replay_checkpoint(&owner, &run_id, None)
            .expect("separately retained checkpoint");

        let mut payload_tamper = replay.clone();
        payload_tamper.envelopes[1].events[0] = payload_tamper.envelopes[2].events[0].clone();
        assert!(IxReplayVerifier::verify(&payload_tamper, &checkpoint).is_err());

        let mut authority_tamper = replay.clone();
        authority_tamper.envelopes[1].events[0].actor = runtime_actor();
        assert_eq!(
            IxReplayVerifier::verify(&authority_tamper, &checkpoint),
            Err(IxContractError::DigestMismatch)
        );

        let mut boundary_tamper = replay.clone();
        boundary_tamper.envelopes.remove(1);
        assert!(IxReplayVerifier::verify(&boundary_tamper, &checkpoint).is_err());

        let mut self_consistent_truncation = replay.clone();
        self_consistent_truncation.envelopes.pop();
        let mut truncated_head = self_consistent_truncation.base_snapshot.clone();
        for envelope in &self_consistent_truncation.envelopes {
            truncated_head = IxReplayVerifier::apply(&truncated_head, envelope)
                .expect("retained prefix remains internally valid");
        }
        self_consistent_truncation.expected_snapshot = truncated_head;
        assert_eq!(
            IxReplayVerifier::verify(&self_consistent_truncation, &checkpoint),
            Err(IxContractError::DigestMismatch)
        );

        let mut self_consistent_replacement = replay.clone();
        self_consistent_replacement.envelopes[1].commit_id =
            "00000000-0000-4000-8000-000000000001".to_string();
        assert_eq!(
            IxReplayVerifier::verify(&self_consistent_replacement, &checkpoint),
            Err(IxContractError::DigestMismatch),
            "the retained full-envelope chain rejects recomputed metadata"
        );

        let mut too_many_events = replay.clone();
        let repeated_event = too_many_events.envelopes[0].events[0].clone();
        too_many_events.envelopes[0].events = vec![repeated_event; 5];
        assert_eq!(
            IxReplayVerifier::verify(&too_many_events, &checkpoint),
            Err(IxContractError::ResourceLimitExceeded)
        );

        let mut too_many_envelopes = replay.clone();
        too_many_envelopes.envelopes =
            vec![too_many_envelopes.envelopes[0].clone(); IX_MAX_COMMITS + 1];
        assert_eq!(
            IxReplayVerifier::verify(&too_many_envelopes, &checkpoint),
            Err(IxContractError::ResourceLimitExceeded)
        );

        let mut oversized_envelope = replay.envelopes[2].clone();
        let IxEventPayload::ArtifactRevision { artifact, .. } =
            &mut oversized_envelope.events[0].payload
        else {
            panic!("third envelope carries the artifact revision");
        };
        artifact.presentation = large_pds_presentation(&artifact.artifact_id, artifact.revision);
        assert_eq!(
            envelope_chain_sha256(&vec![oversized_envelope; 9]),
            Err(IxContractError::ResourceLimitExceeded),
            "the verifier bounds canonical envelope bytes independently"
        );

        let mut forged_checkpoint = checkpoint;
        forged_checkpoint.owner.subject = "other".to_string();
        assert_eq!(
            IxReplayVerifier::verify(&replay, &forged_checkpoint),
            Err(IxContractError::DigestMismatch)
        );
    }

    #[test]
    fn presentation_rejects_unsafe_cross_channel_numbers() {
        let mut unsafe_artifact = artifact(1);
        unsafe_artifact.presentation["response"]["regions"][0]["body"] =
            serde_json::from_str("9007199254740992").expect("JSON parses");
        assert_eq!(
            unsafe_artifact.validate(),
            Err(IxContractError::NonPortableJson)
        );
    }

    #[test]
    fn presentation_depth_and_node_bounds_are_exact_and_non_recursive() {
        fn region_body(presentation: &mut Value) -> &mut Value {
            presentation
                .get_mut("response")
                .and_then(|response| response.get_mut("regions"))
                .and_then(Value::as_array_mut)
                .and_then(|regions| regions.first_mut())
                .and_then(|region| region.get_mut("body"))
                .expect("valid presentation has a first region body")
        }

        fn nested_arrays(count: usize) -> Value {
            let mut value = Value::Null;
            for _ in 0..count {
                value = Value::Array(vec![value]);
            }
            value
        }

        fn json_node_count(value: &Value) -> usize {
            let mut stack = vec![value];
            let mut count = 0usize;
            while let Some(current) = stack.pop() {
                count += 1;
                match current {
                    Value::Array(values) => stack.extend(values),
                    Value::Object(values) => stack.extend(values.values()),
                    _ => {}
                }
            }
            count
        }

        let mut at_depth = pds_presentation("brief-1", 1);
        *region_body(&mut at_depth) = nested_arrays(IX_MAX_JSON_DEPTH - 4);
        validate_portable_json(&at_depth).expect("exact depth boundary is valid");
        let mut over_depth = pds_presentation("brief-1", 1);
        *region_body(&mut over_depth) = nested_arrays(IX_MAX_JSON_DEPTH - 3);
        assert_eq!(
            validate_portable_json(&over_depth),
            Err(IxContractError::ResourceLimitExceeded)
        );

        let mut at_nodes = pds_presentation("brief-1", 1);
        *region_body(&mut at_nodes) = Value::Array(Vec::new());
        let base_nodes = json_node_count(&at_nodes);
        *region_body(&mut at_nodes) =
            Value::Array(vec![Value::Null; IX_MAX_JSON_NODES - base_nodes]);
        assert_eq!(json_node_count(&at_nodes), IX_MAX_JSON_NODES);
        validate_portable_json(&at_nodes).expect("exact node boundary is valid");
        let mut over_nodes = at_nodes;
        region_body(&mut over_nodes)
            .as_array_mut()
            .expect("body is the node-boundary array")
            .push(Value::Null);
        assert_eq!(
            validate_portable_json(&over_nodes),
            Err(IxContractError::ResourceLimitExceeded)
        );
    }

    #[test]
    fn every_event_and_payload_rejects_secret_content_and_unknown_fields() {
        let owner = owner("wayne");
        let authority_context = authorization_context(&owner, 1);
        let authority = authority_context.binding();
        let payloads = vec![
            IxEventPayload::RunAcknowledged {
                intent_label: "Analyze why".to_string(),
                focus: request().focus,
            },
            IxEventPayload::ContextResolved {
                authority: authority.clone(),
                context: context(),
            },
            IxEventPayload::ContextReconciliationRequested {
                proposal: IxContextProposal::from_authenticated_human(
                    "proposal-2",
                    1,
                    "nexus-release-2",
                    "session-1",
                    &owner,
                    "Request a context correction.",
                    None,
                    None,
                    None,
                )
                .expect("valid proposal"),
            },
            IxEventPayload::ContextReconciled {
                proposal_id: "proposal-2".to_string(),
                context_revision: custom_authorization_context(
                    &owner,
                    2,
                    "nexus-release-2",
                    "session-1",
                    IxDataClassification::Confidential,
                    IxConsentState::Granted,
                    IxEgressPosture::LocalOnly,
                ),
            },
            IxEventPayload::PhaseChanged {
                authority: authority.clone(),
                phase: IxPhase::Gathering,
                reason: "Gather authorized context.".to_string(),
            },
            IxEventPayload::ArtifactRevision {
                authority: authority.clone(),
                artifact: artifact(1),
            },
            IxEventPayload::WaitingForUser {
                authority: authority.clone(),
                prompt: "Choose the next safe move.".to_string(),
            },
            IxEventPayload::UserInputAccepted {
                authority: authority.clone(),
            },
            IxEventPayload::CancelRequested {
                command_id: "cancel-1".to_string(),
                accepted_cursor: "ix1.run-1.2".to_string(),
                accepted_revision: 2,
            },
            IxEventPayload::CancellationProgress {
                command_id: "cancel-1".to_string(),
                stage: IxCancellationStage::Stopping,
            },
            IxEventPayload::RunFinished {
                authority: None,
                outcome: IxTerminalOutcome::Failed,
                message: "Stopped safely.".to_string(),
            },
        ];
        for payload in payloads {
            let mut encoded = serde_json::to_value(payload).expect("payload serializes");
            encoded
                .as_object_mut()
                .expect("payload object")
                .insert("secretContent".to_string(), json!("must-not-pass"));
            assert!(
                serde_json::from_value::<IxEventPayload>(encoded).is_err(),
                "all payload variants reject unknown secret content"
            );
        }

        let mut event = serde_json::to_value(IxEvent {
            schema_version: IX_EVENT_SCHEMA_VERSION.to_string(),
            event_id: "run-1:1".to_string(),
            run_id: "run-1".to_string(),
            sequence: 1,
            occurred_at: "2026-08-11T12:00:00Z".to_string(),
            actor: runtime_actor(),
            payload: IxEventPayload::RunAcknowledged {
                intent_label: "Analyze why".to_string(),
                focus: request().focus,
            },
        })
        .expect("event serializes");
        event
            .as_object_mut()
            .expect("event object")
            .insert("secretContent".to_string(), json!("must-not-pass"));
        assert!(serde_json::from_value::<IxEvent>(event).is_err());
    }

    #[test]
    fn context_rejects_future_source_time_and_false_aggregate_freshness() {
        let mut future = context();
        future.sources[0].refreshed_at = Some("2026-08-11T12:01:00Z".to_string());
        assert_eq!(
            future.validate(),
            Err(IxContractError::InvalidContextFreshness)
        );

        let mut false_aggregate = context();
        false_aggregate.freshness = IxContextFreshness::Stale;
        assert_eq!(
            false_aggregate.validate(),
            Err(IxContractError::InvalidContextFreshness)
        );
    }
}
