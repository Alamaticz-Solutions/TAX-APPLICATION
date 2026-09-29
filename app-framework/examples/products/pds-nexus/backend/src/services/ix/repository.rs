//! Product-owned PostgreSQL adapter for the un-gated Framework IX
//! persistence seam.
//!
//! Storage authority: the generated `nexus_ix` DDL
//! (`database/_pkg/schemas/nexus_ix/tables.pg.sql`) plus the supplemental
//! forward-only expand migration (tenant/idempotency/ordering/projection
//! constraints). The adapter writes only runtime-produced values:
//! - `nexus_ix.ix_runs` holds one row per run with the full sealed
//!   [`IxStoredRun`] JSON as document of record plus read projections
//!   (state, phase, cursor, repository version).
//! - `nexus_ix.ix_commit_envelopes` holds the append-only canonical commit
//!   envelope projection keyed by `(run_id, sequence = result_revision)`.
//! - `nexus_ix.ix_audit_facts` holds Framework-authored audit facts.
//! - `nexus_ix.ix_projection_cursors` holds the external audit-projection
//!   posture keyed by the Framework cancel dedup key.
//!
//! Error mapping note: `IxContractError` has no storage-infrastructure
//! variant, and the chat-gated runtime treats every repository error as
//! retry/unavailable uniformly. This adapter maps infrastructure failures
//! (connection, SQL, serialization-at-rest) to
//! [`IxContractError::AuditUnavailable`] — the only availability-flavored
//! variant — and reserves [`IxContractError::InvalidLifecycle`] for
//! compare-and-swap conflicts and corrupted-row detection,
//! [`IxContractError::RunNotFound`] for missing rows on append.

use std::str::FromStr;

use appfw_runtime::ix::{
    IxAuditEvent, IxAuditProjection, IxAuditProjectionState, IxCommitEnvelope, IxContractError,
    IxPrincipalBinding, IxRepositoryCommitReceipt, IxRunCommitTransaction, IxRunCreateTransaction,
    IxRunRepository, IxStoredRun,
};
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio_postgres::{IsolationLevel, NoTls};

const IX_SCHEMA: &str = "nexus_ix";

/// Product-owned PostgreSQL repository for Nexus IX durable runs.
#[derive(Clone)]
pub struct NexusIxRunRepository {
    pool: Pool,
}

impl NexusIxRunRepository {
    /// Wraps an existing product connection pool.
    pub fn from_pool(pool: Pool) -> Self {
        Self { pool }
    }

    /// Shares the pool with the sibling audit projection path.
    pub(crate) fn pool_for_audit(&self) -> &Pool {
        &self.pool
    }

    /// Builds a small dedicated pool from a PostgreSQL connection URL.
    ///
    /// Local-proof composition only (`NoTls`); the deployed host wires the
    /// adapter onto the product's governed pool instead.
    pub fn connect_local(url: &str, max_size: usize) -> Result<Self, IxContractError> {
        let config = tokio_postgres::Config::from_str(url).map_err(|_| storage_unavailable())?;
        let manager = Manager::from_config(
            config,
            NoTls,
            ManagerConfig {
                recycling_method: RecyclingMethod::Fast,
            },
        );
        let pool = Pool::builder(manager)
            .max_size(max_size.max(1))
            .build()
            .map_err(|_| storage_unavailable())?;
        Ok(Self { pool })
    }

    /// Persists one runtime-prepared run creation atomically: the sealed
    /// stored-run document of record, its acknowledgement envelope
    /// projection, and the Framework-authored start audit fact.
    ///
    /// Idempotent: retrying the identical creation returns a receipt with
    /// `deduplicated = true`. A different run claiming the same
    /// `(tenant, start_idempotency_key)` or run id is a conflict.
    pub async fn persist_created_run(
        &self,
        stored: &IxStoredRun,
        audit_fact: &IxAuditEvent,
        idempotency_key: &str,
    ) -> Result<IxRepositoryCommitReceipt, IxContractError> {
        stored.validate()?;
        let acknowledgement = stored
            .envelopes()
            .first()
            .ok_or(IxContractError::InvalidLifecycle)?
            .clone();
        if stored.repository_version() != 1
            || idempotency_key != stored.start_idempotency_key()
            || audit_fact != stored.start_audit_fact()
        {
            return Err(IxContractError::InvalidLifecycle);
        }
        let snapshot = stored.snapshot();
        let stored_json = canonical_stored_json(stored)?;
        let mut client = self.pool.get().await.map_err(|_| storage_unavailable())?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await
            .map_err(|_| storage_unavailable())?;
        let inserted = transaction
            .execute(
                &format!(
                    "INSERT INTO {IX_SCHEMA}.ix_runs \
                     (id, tenant, subject, intent_key, run_state, phase, repository_version, \
                      last_sequence, cursor, start_idempotency_key, runtime_seal_key_id, \
                      stored_run, created_at, updated_at) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, now(), now()) \
                     ON CONFLICT (id) DO NOTHING"
                ),
                &[
                    &snapshot.run_id,
                    &snapshot.owner.tenant_id,
                    &snapshot.owner.subject,
                    &snapshot.request.intent_key,
                    &enum_label(&snapshot.state)?,
                    &optional_enum_label(snapshot.phase.as_ref())?,
                    &to_db_version(stored.repository_version())?,
                    &to_db_version(snapshot.last_sequence)?,
                    &snapshot.cursor,
                    &idempotency_key,
                    &stored.runtime_seal_key_id(),
                    &stored_json,
                ],
            )
            .await
            .map_err(map_pg_error)?;
        if inserted == 0 {
            transaction
                .rollback()
                .await
                .map_err(|_| storage_unavailable())?;
            let existing = self
                .load_stored_run(&snapshot.owner, &snapshot.run_id)
                .await?
                .ok_or(IxContractError::InvalidLifecycle)?;
            if &existing == stored {
                return IxRepositoryCommitReceipt::new(
                    snapshot.run_id.clone(),
                    snapshot.cursor.clone(),
                    stored.repository_version(),
                    acknowledgement.commit_id,
                    true,
                );
            }
            return Err(IxContractError::InvalidLifecycle);
        }
        insert_envelope(&transaction, &acknowledgement).await?;
        insert_audit_fact(&transaction, audit_fact).await?;
        transaction
            .commit()
            .await
            .map_err(|_| storage_unavailable())?;
        IxRepositoryCommitReceipt::new(
            snapshot.run_id.clone(),
            snapshot.cursor.clone(),
            stored.repository_version(),
            acknowledgement.commit_id,
            false,
        )
    }

    /// Loads one owner-scoped stored run and decodes it under the
    /// Framework's own decode contract (shape, bounds, replay, and
    /// cancellation-structure checks; never HMAC acceptance).
    pub async fn load_stored_run(
        &self,
        owner: &IxPrincipalBinding,
        run_id: &str,
    ) -> Result<Option<IxStoredRun>, IxContractError> {
        let client = self.pool.get().await.map_err(|_| storage_unavailable())?;
        let row = client
            .query_opt(
                &format!(
                    "SELECT stored_run FROM {IX_SCHEMA}.ix_runs \
                     WHERE id = $1 AND tenant = $2 AND subject = $3"
                ),
                &[&run_id, &owner.tenant_id, &owner.subject],
            )
            .await
            .map_err(|_| storage_unavailable())?;
        let Some(row) = row else {
            return Ok(None);
        };
        let value: Value = row.get(0);
        let bytes = serde_json::to_vec(&value).map_err(|_| IxContractError::InvalidLifecycle)?;
        let stored = IxStoredRun::from_json_slice(&bytes)?;
        if stored.snapshot().run_id != run_id || &stored.snapshot().owner != owner {
            return Err(IxContractError::InvalidLifecycle);
        }
        Ok(Some(stored))
    }

    /// One compare-and-swap append over the canonical cursor: the run row
    /// advances from `(expected_repository_version, expected_cursor)` to the
    /// next sealed stored run atomically with its envelope projection and
    /// optional pending audit projection.
    ///
    /// Retrying an already-applied transaction returns `deduplicated = true`;
    /// any other cursor/version disagreement is a conflict.
    #[allow(clippy::too_many_arguments)]
    pub async fn compare_and_append_stored(
        &self,
        owner: &IxPrincipalBinding,
        expected_repository_version: u64,
        expected_cursor: &str,
        envelope: &IxCommitEnvelope,
        next_stored: &IxStoredRun,
        audit_projection: Option<&IxAuditProjection>,
    ) -> Result<IxRepositoryCommitReceipt, IxContractError> {
        next_stored.validate()?;
        let snapshot = next_stored.snapshot();
        if expected_repository_version == 0
            || expected_repository_version
                .checked_add(1)
                .filter(|next| *next == next_stored.repository_version())
                .is_none()
            || envelope.run_id != snapshot.run_id
            || envelope.result_cursor != snapshot.cursor
            || expected_cursor == envelope.result_cursor
            || owner != &snapshot.owner
        {
            return Err(IxContractError::InvalidLifecycle);
        }
        let stored_json = canonical_stored_json(next_stored)?;
        let mut client = self.pool.get().await.map_err(|_| storage_unavailable())?;
        let transaction = client
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await
            .map_err(|_| storage_unavailable())?;
        let updated = transaction
            .execute(
                &format!(
                    "UPDATE {IX_SCHEMA}.ix_runs \
                     SET repository_version = $1, last_sequence = $2, cursor = $3, \
                         run_state = $4, phase = $5, stored_run = $6, updated_at = now() \
                     WHERE id = $7 AND tenant = $8 AND subject = $9 \
                       AND repository_version = $10 AND cursor = $11"
                ),
                &[
                    &to_db_version(next_stored.repository_version())?,
                    &to_db_version(snapshot.last_sequence)?,
                    &snapshot.cursor,
                    &enum_label(&snapshot.state)?,
                    &optional_enum_label(snapshot.phase.as_ref())?,
                    &stored_json,
                    &snapshot.run_id,
                    &owner.tenant_id,
                    &owner.subject,
                    &to_db_version(expected_repository_version)?,
                    &expected_cursor,
                ],
            )
            .await
            .map_err(map_pg_error)?;
        if updated == 0 {
            transaction
                .rollback()
                .await
                .map_err(|_| storage_unavailable())?;
            return self
                .resolve_append_conflict(owner, envelope, next_stored)
                .await;
        }
        insert_envelope(&transaction, envelope).await?;
        if let Some(projection) = audit_projection {
            upsert_projection(&transaction, projection, IxAuditProjectionState::Pending).await?;
        }
        transaction
            .commit()
            .await
            .map_err(|_| storage_unavailable())?;
        IxRepositoryCommitReceipt::new(
            snapshot.run_id.clone(),
            snapshot.cursor.clone(),
            next_stored.repository_version(),
            envelope.commit_id.clone(),
            false,
        )
    }

    /// Records the external audit-projection posture for a retained
    /// cancellation fact, keyed by the Framework dedup key. Idempotent:
    /// re-marking the same projection never duplicates a row.
    pub async fn mark_audit_projection_state(
        &self,
        projection: &IxAuditProjection,
        state: IxAuditProjectionState,
    ) -> Result<(), IxContractError> {
        let mut client = self.pool.get().await.map_err(|_| storage_unavailable())?;
        let transaction = client
            .transaction()
            .await
            .map_err(|_| storage_unavailable())?;
        let inserted = transaction
            .execute(
                &format!(
                    "INSERT INTO {IX_SCHEMA}.ix_projection_cursors \
                     (id, tenant, run_id, command_id, cursor, projection_state, updated_at) \
                     VALUES ($1, $2, $3, $4, $5, $6, now()) \
                     ON CONFLICT (id) DO NOTHING"
                ),
                &[
                    &projection.dedup_key(),
                    &projection.tenant_id(),
                    &projection.run_id(),
                    &projection.command_id(),
                    &projection.cursor(),
                    &enum_label(&state)?,
                ],
            )
            .await
            .map_err(map_pg_error)?;
        if inserted == 0 {
            ensure_projection_matches(&transaction, projection).await?;
            transaction
                .execute(
                    &format!(
                        "UPDATE {IX_SCHEMA}.ix_projection_cursors \
                         SET projection_state = $1, updated_at = now() \
                         WHERE id = $2"
                    ),
                    &[&enum_label(&state)?, &projection.dedup_key()],
                )
                .await
                .map_err(map_pg_error)?;
        }
        transaction
            .commit()
            .await
            .map_err(|_| storage_unavailable())?;
        Ok(())
    }

    async fn resolve_append_conflict(
        &self,
        owner: &IxPrincipalBinding,
        envelope: &IxCommitEnvelope,
        next_stored: &IxStoredRun,
    ) -> Result<IxRepositoryCommitReceipt, IxContractError> {
        let current = self
            .load_stored_run(owner, &next_stored.snapshot().run_id)
            .await?
            .ok_or(IxContractError::RunNotFound)?;
        let already_applied = current.repository_version() >= next_stored.repository_version()
            && current
                .envelopes()
                .iter()
                .any(|applied| applied == envelope);
        if already_applied {
            return IxRepositoryCommitReceipt::new(
                next_stored.snapshot().run_id.clone(),
                next_stored.snapshot().cursor.clone(),
                next_stored.repository_version(),
                envelope.commit_id.clone(),
                true,
            );
        }
        Err(IxContractError::InvalidLifecycle)
    }
}

#[async_trait::async_trait]
impl IxRunRepository for NexusIxRunRepository {
    async fn create_run(
        &self,
        transaction: IxRunCreateTransaction,
    ) -> Result<IxRepositoryCommitReceipt, IxContractError> {
        // Thin delegation: runtime-constructed transactions expose read
        // accessors only; the persistence semantics live in the public
        // operations proven at adapter level. Trait dispatch driven by the
        // real IxRuntimeService is a named B_HOST claim (DEFERRED_TO_B_HOST).
        self.persist_created_run(
            transaction.stored_run(),
            transaction.audit_fact(),
            transaction.idempotency_key(),
        )
        .await
    }

    async fn load_run(
        &self,
        owner: &IxPrincipalBinding,
        run_id: &str,
    ) -> Result<Option<IxStoredRun>, IxContractError> {
        self.load_stored_run(owner, run_id).await
    }

    async fn compare_and_append(
        &self,
        transaction: IxRunCommitTransaction,
    ) -> Result<IxRepositoryCommitReceipt, IxContractError> {
        self.compare_and_append_stored(
            transaction.owner(),
            transaction.expected_repository_version(),
            transaction.expected_cursor(),
            transaction.envelope(),
            transaction.stored_run(),
            transaction.audit_projection(),
        )
        .await
    }

    async fn mark_audit_projection(
        &self,
        projection: &IxAuditProjection,
        state: IxAuditProjectionState,
    ) -> Result<(), IxContractError> {
        self.mark_audit_projection_state(projection, state).await
    }
}

async fn insert_envelope(
    transaction: &tokio_postgres::Transaction<'_>,
    envelope: &IxCommitEnvelope,
) -> Result<(), IxContractError> {
    let value = serde_json::to_value(envelope).map_err(|_| IxContractError::InvalidLifecycle)?;
    let inserted = transaction
        .execute(
            &format!(
                "INSERT INTO {IX_SCHEMA}.ix_commit_envelopes \
                 (id, run_id, sequence, event_kind, envelope, committed_at) \
                 VALUES ($1, $2, $3, $4, $5, now()) \
                 ON CONFLICT (id) DO NOTHING"
            ),
            &[
                &envelope.commit_id,
                &envelope.run_id,
                &to_db_version(envelope.result_revision)?,
                &envelope_event_kind(envelope)?,
                &value,
            ],
        )
        .await
        .map_err(map_pg_error)?;
    if inserted == 0 {
        return ensure_envelope_matches(transaction, envelope).await;
    }
    Ok(())
}

async fn ensure_envelope_matches(
    transaction: &tokio_postgres::Transaction<'_>,
    envelope: &IxCommitEnvelope,
) -> Result<(), IxContractError> {
    let row = transaction
        .query_opt(
            &format!("SELECT envelope FROM {IX_SCHEMA}.ix_commit_envelopes WHERE id = $1"),
            &[&envelope.commit_id],
        )
        .await
        .map_err(|_| storage_unavailable())?
        .ok_or(IxContractError::InvalidLifecycle)?;
    let stored: Value = row.get(0);
    let stored_envelope: IxCommitEnvelope =
        serde_json::from_value(stored).map_err(|_| IxContractError::InvalidLifecycle)?;
    if stored_envelope != *envelope {
        return Err(IxContractError::InvalidLifecycle);
    }
    Ok(())
}

pub(super) async fn insert_audit_fact(
    transaction: &tokio_postgres::Transaction<'_>,
    fact: &IxAuditEvent,
) -> Result<u64, IxContractError> {
    let value = serde_json::to_value(fact).map_err(|_| IxContractError::InvalidLifecycle)?;
    transaction
        .execute(
            &format!(
                "INSERT INTO {IX_SCHEMA}.ix_audit_facts \
                 (id, tenant, run_id, event_kind, disposition, fact, recorded_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, now()) \
                 ON CONFLICT (id) DO NOTHING"
            ),
            &[
                &audit_fact_identity(&value),
                &fact.principal.tenant_id,
                &fact.run_id,
                &enum_label(&fact.event_kind)?,
                &optional_enum_label(fact.cancel_disposition.as_ref())?,
                &value,
            ],
        )
        .await
        .map_err(map_pg_error)
}

async fn upsert_projection(
    transaction: &tokio_postgres::Transaction<'_>,
    projection: &IxAuditProjection,
    state: IxAuditProjectionState,
) -> Result<(), IxContractError> {
    let inserted = transaction
        .execute(
            &format!(
                "INSERT INTO {IX_SCHEMA}.ix_projection_cursors \
                 (id, tenant, run_id, command_id, cursor, projection_state, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, now()) \
                 ON CONFLICT (id) DO NOTHING"
            ),
            &[
                &projection.dedup_key(),
                &projection.tenant_id(),
                &projection.run_id(),
                &projection.command_id(),
                &projection.cursor(),
                &enum_label(&state)?,
            ],
        )
        .await
        .map_err(map_pg_error)?;
    if inserted == 0 {
        return ensure_projection_matches(transaction, projection).await;
    }
    Ok(())
}

async fn ensure_projection_matches(
    transaction: &tokio_postgres::Transaction<'_>,
    projection: &IxAuditProjection,
) -> Result<(), IxContractError> {
    let row = transaction
        .query_opt(
            &format!(
                "SELECT tenant, run_id, command_id, cursor \
                 FROM {IX_SCHEMA}.ix_projection_cursors WHERE id = $1"
            ),
            &[&projection.dedup_key()],
        )
        .await
        .map_err(|_| storage_unavailable())?
        .ok_or(IxContractError::InvalidLifecycle)?;
    let tenant: String = row.get(0);
    let run_id: String = row.get(1);
    let command_id: String = row.get(2);
    let cursor: String = row.get(3);
    if tenant != projection.tenant_id()
        || run_id != projection.run_id()
        || command_id != projection.command_id()
        || cursor != projection.cursor()
    {
        return Err(IxContractError::InvalidLifecycle);
    }
    Ok(())
}

/// Deterministic audit-fact row identity: the Framework fact is
/// content-addressed so at-least-once flushes stay idempotent.
pub(super) fn audit_fact_identity(fact_value: &Value) -> String {
    let canonical = canonical_value(fact_value.clone());
    let bytes = serde_json::to_vec(&canonical).unwrap_or_default();
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn canonical_value(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.into_iter().map(canonical_value).collect()),
        Value::Object(entries) => {
            let mut sorted = entries.into_iter().collect::<Vec<_>>();
            sorted.sort_by(|left, right| left.0.cmp(&right.0));
            Value::Object(
                sorted
                    .into_iter()
                    .map(|(key, value)| (key, canonical_value(value)))
                    .collect(),
            )
        }
        scalar => scalar,
    }
}

fn canonical_stored_json(stored: &IxStoredRun) -> Result<Value, IxContractError> {
    let bytes = stored.to_json_vec()?;
    serde_json::from_slice(&bytes).map_err(|_| IxContractError::InvalidLifecycle)
}

fn envelope_event_kind(envelope: &IxCommitEnvelope) -> Result<String, IxContractError> {
    let last = envelope
        .events
        .last()
        .ok_or(IxContractError::InvalidLifecycle)?;
    let value =
        serde_json::to_value(&last.payload).map_err(|_| IxContractError::InvalidLifecycle)?;
    value
        .get("type")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or(IxContractError::InvalidLifecycle)
}

/// Serializes a snake_case serde enum unit label (for example
/// `IxRunState::Running` -> `"running"`).
pub(super) fn enum_label<T: serde::Serialize>(value: &T) -> Result<String, IxContractError> {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(ToString::to_string))
        .ok_or(IxContractError::InvalidLifecycle)
}

fn optional_enum_label<T: serde::Serialize>(
    value: Option<&T>,
) -> Result<Option<String>, IxContractError> {
    value.map(enum_label).transpose()
}

fn to_db_version(value: u64) -> Result<i64, IxContractError> {
    i64::try_from(value).map_err(|_| IxContractError::ResourceLimitExceeded)
}

fn storage_unavailable() -> IxContractError {
    IxContractError::AuditUnavailable
}

fn map_pg_error(error: tokio_postgres::Error) -> IxContractError {
    if let Some(state) = error.code() {
        if state == &tokio_postgres::error::SqlState::UNIQUE_VIOLATION {
            return IxContractError::InvalidLifecycle;
        }
    }
    storage_unavailable()
}
