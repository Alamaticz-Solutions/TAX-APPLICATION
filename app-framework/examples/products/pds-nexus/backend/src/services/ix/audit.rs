//! Product-owned [`IxAuditSink`] implementation with durable PostgreSQL
//! projection.
//!
//! The Framework lifecycle appends metadata-only audit facts synchronously
//! and treats retention as availability-critical, so the sink accepts facts
//! into a bounded in-process retention buffer first (never blocking the
//! lifecycle on a database round trip) and projects them into
//! `nexus_ix.ix_audit_facts` through [`NexusIxAuditSink::flush_to_postgres`].
//! Projection is at-least-once: every fact row is content-addressed, so
//! repeated flushes deduplicate instead of duplicating evidence.

use std::sync::Mutex;

use appfw_runtime::ix::{IxAuditEvent, IxAuditSink, IxContractError};

use super::repository::{insert_audit_fact, NexusIxRunRepository};

/// Bounded retention so a runaway caller cannot grow memory without bound;
/// the Framework rejects work when audit retention fails, so refusing new
/// facts at the bound keeps the fail-closed posture.
const MAX_BUFFERED_FACTS: usize = 4096;

/// Outcome of one at-least-once PostgreSQL projection pass.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NexusIxAuditFlushReport {
    /// Facts newly inserted by this pass.
    pub inserted: usize,
    /// Facts already present (content-addressed dedup hit).
    pub deduplicated: usize,
    /// Facts kept in the buffer because the referenced run row does not
    /// exist yet (foreign-key posture); they remain flushable later.
    pub retained: usize,
}

/// Product-owned audit sink for the Nexus IX vertical.
#[derive(Default)]
pub struct NexusIxAuditSink {
    buffer: Mutex<Vec<IxAuditEvent>>,
}

impl NexusIxAuditSink {
    pub fn new() -> Self {
        Self::default()
    }

    /// Facts currently retained in process (accepted but not yet projected,
    /// or unprojectable until their run row exists).
    pub fn buffered(&self) -> Vec<IxAuditEvent> {
        self.lock().clone()
    }

    /// Projects buffered facts into `nexus_ix.ix_audit_facts` through the
    /// product repository's connection pool. Successfully projected facts
    /// (inserted or deduplicated) leave the buffer; facts whose run row does
    /// not exist yet are retained for a later pass.
    pub async fn flush_to_postgres(
        &self,
        repository: &NexusIxRunRepository,
    ) -> Result<NexusIxAuditFlushReport, IxContractError> {
        let pending = self.lock().clone();
        let mut report = NexusIxAuditFlushReport::default();
        let mut projected = Vec::new();
        for fact in &pending {
            match project_fact(repository, fact).await {
                Ok(inserted) => {
                    if inserted {
                        report.inserted += 1;
                    } else {
                        report.deduplicated += 1;
                    }
                    projected.push(fact.clone());
                }
                Err(ProjectionError::RunRowMissing) => {
                    report.retained += 1;
                }
                Err(ProjectionError::Storage(error)) => return Err(error),
            }
        }
        let mut buffer = self.lock();
        buffer.retain(|fact| !projected.contains(fact));
        Ok(report)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<IxAuditEvent>> {
        self.buffer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl IxAuditSink for NexusIxAuditSink {
    fn append(&self, event: &IxAuditEvent) -> Result<(), IxContractError> {
        let mut buffer = self.lock();
        if buffer.len() >= MAX_BUFFERED_FACTS {
            return Err(IxContractError::AuditUnavailable);
        }
        buffer.push(event.clone());
        Ok(())
    }
}

enum ProjectionError {
    RunRowMissing,
    Storage(IxContractError),
}

async fn project_fact(
    repository: &NexusIxRunRepository,
    fact: &IxAuditEvent,
) -> Result<bool, ProjectionError> {
    let mut client = repository
        .pool_for_audit()
        .get()
        .await
        .map_err(|_| ProjectionError::Storage(IxContractError::AuditUnavailable))?;
    let transaction = client
        .transaction()
        .await
        .map_err(|_| ProjectionError::Storage(IxContractError::AuditUnavailable))?;
    // notes-000005 / 000004: close the id-only hole. Scope the run by id
    // AND tenant; when fact.principal is the stored owner, also require
    // subject. Any other row (cross-tenant, cross-subject, or missing) is
    // retain-as-missing — never a cross-tenant insert.
    let owner_subject = transaction
        .query_opt(
            "SELECT subject FROM nexus_ix.ix_runs WHERE id = $1 AND tenant = $2",
            &[&fact.run_id, &fact.principal.tenant_id],
        )
        .await
        .map_err(|_| ProjectionError::Storage(IxContractError::AuditUnavailable))?;
    let Some(owner_subject) = owner_subject else {
        return Err(ProjectionError::RunRowMissing);
    };
    let owner_subject: String = owner_subject.get(0);
    if fact.principal.subject.is_empty() || fact.principal.subject != owner_subject {
        return Err(ProjectionError::RunRowMissing);
    }
    let inserted = insert_audit_fact(&transaction, fact)
        .await
        .map_err(ProjectionError::Storage)?;
    transaction
        .commit()
        .await
        .map_err(|_| ProjectionError::Storage(IxContractError::AuditUnavailable))?;
    Ok(inserted == 1)
}

#[cfg(test)]
mod tests {
    use appfw_runtime::extension::RuntimePrincipalType;
    use appfw_runtime::ix::{IxAuditEventKind, IxPrincipalBinding, IX_AUDIT_SCHEMA_VERSION};

    use super::*;

    fn fact(run_id: &str) -> IxAuditEvent {
        IxAuditEvent {
            schema_version: IX_AUDIT_SCHEMA_VERSION.to_string(),
            event_kind: IxAuditEventKind::RunRequested,
            request_id: "6e0d1745-9f9f-4f7e-9f7f-3a3f0c8f4c11".to_string(),
            correlation_id: "6e0d1745-9f9f-4f7e-9f7f-3a3f0c8f4c11".to_string(),
            principal: IxPrincipalBinding {
                tenant_id: "tenant_a".to_string(),
                subject: "nexus.user".to_string(),
                principal_type: RuntimePrincipalType::User,
                on_behalf_of: None,
            },
            run_id: run_id.to_string(),
            intent_key: Some("pds.ix.intent.attention-stewardship@1".to_string()),
            cancel_disposition: None,
        }
    }

    #[test]
    fn append_retains_facts_in_order_and_bounds_memory() {
        let sink = NexusIxAuditSink::new();
        sink.append(&fact("run-a")).expect("first fact accepted");
        sink.append(&fact("run-b")).expect("second fact accepted");
        let buffered = sink.buffered();
        assert_eq!(buffered.len(), 2);
        assert_eq!(buffered[0].run_id, "run-a");
        assert_eq!(buffered[1].run_id, "run-b");
    }

    #[test]
    fn append_fails_closed_at_the_retention_bound() {
        let sink = NexusIxAuditSink::new();
        for index in 0..MAX_BUFFERED_FACTS {
            sink.append(&fact(&format!("run-{index}")))
                .expect("fact within bound accepted");
        }
        assert_eq!(
            sink.append(&fact("run-overflow")),
            Err(IxContractError::AuditUnavailable)
        );
    }
}
