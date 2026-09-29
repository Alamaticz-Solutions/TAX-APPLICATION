//! Adapter-level proof against a real disposable PostgreSQL instance.
//!
//! These tests execute when `NEXUS_IX_PG_PROOF_URL` names a reachable
//! PostgreSQL with the generated `nexus_ix` DDL applied (the mandated
//! product migrate workflow provisions it). Without the variable each test
//! reports a loud skip and passes, so `cargo test --locked` stays runnable
//! on machines without a database; the B_PRODUCT proof run always sets the
//! variable so the PostgreSQL proofs actually execute.
//!
//! Proven here, per the re-scoped B_PRODUCT exit:
//! - stored-run serde round-trip persistence under the Framework's own
//!   decode contract, including second-connection reload of persisted rows;
//! - compare-and-swap on the canonical cursor (fresh apply, idempotent
//!   retry dedup, stale-cursor conflict, missing-run detection);
//! - idempotent audit-projection dedup keyed by the Framework dedup key;
//! - content-addressed audit-fact projection dedup through the product
//!   audit sink.
//!
//! NOT claimed here (named B_HOST claims, DEFERRED_TO_B_HOST): transport,
//! SSE, cancel-through-transport, JWT ingress, restart/replay through
//! `IxRuntimeService`, seal (HMAC) authentication, and trait-dispatch
//! integration through runtime-constructed transactions.

use appfw_runtime::ix::{
    IxAuditEventKind, IxAuditProjection, IxAuditProjectionState, IxAuditSink, IxContractError,
    IxPrincipalBinding, IxStoredRun, IX_AUDIT_SCHEMA_VERSION,
};
use tokio_postgres::NoTls;

use super::audit::NexusIxAuditSink;
use super::proof_fixtures::{harvest_run, HarvestedRun};
use super::repository::{audit_fact_identity, NexusIxRunRepository};

const PROOF_URL_ENV: &str = "NEXUS_IX_PG_PROOF_URL";

fn proof_url(test_name: &str) -> Option<String> {
    match std::env::var(PROOF_URL_ENV) {
        Ok(url) if !url.trim().is_empty() => Some(url),
        _ => {
            eprintln!(
                "SKIPPED (no real PostgreSQL): set {PROOF_URL_ENV} to run {test_name} \
                 against the disposable proof database"
            );
            None
        }
    }
}

fn repository(url: &str) -> NexusIxRunRepository {
    NexusIxRunRepository::connect_local(url, 4).expect("proof pool composes")
}

async fn persist_first_stage(
    repository: &NexusIxRunRepository,
    run: &HarvestedRun,
) -> appfw_runtime::ix::IxRepositoryCommitReceipt {
    repository
        .persist_created_run(
            &run.stages[0].stored,
            &run.start_audit_fact,
            run.stages[0].stored.start_idempotency_key(),
        )
        .await
        .expect("first durable creation persists")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stored_run_round_trips_and_reloads_on_a_second_connection() {
    let Some(url) = proof_url("stored_run_round_trips_and_reloads_on_a_second_connection") else {
        return;
    };
    let repo = repository(&url);
    let run = harvest_run().expect("run harvests");
    let stage = &run.stages[0];

    let receipt = persist_first_stage(&repo, &run).await;
    assert!(!receipt.deduplicated());
    assert_eq!(receipt.run_id(), run.run_id);
    assert_eq!(receipt.cursor(), stage.stored.snapshot().cursor);
    assert_eq!(receipt.revision(), stage.stored.repository_version());
    assert_eq!(receipt.commit_id(), stage.appended_envelope.commit_id);

    let loaded = repo
        .load_stored_run(&run.owner, &run.run_id)
        .await
        .expect("load succeeds")
        .expect("run row exists");
    assert_eq!(loaded, stage.stored, "serde round trip is lossless");

    // Second, fully independent connection (raw tokio-postgres, not the
    // adapter pool): the persisted bytes decode under the Framework decode
    // contract without any in-process state.
    let (client, connection) = tokio_postgres::connect(&url, NoTls)
        .await
        .expect("second connection opens");
    let connection_task = tokio::spawn(connection);
    let row = client
        .query_one(
            "SELECT stored_run FROM nexus_ix.ix_runs WHERE id = $1 AND tenant = $2 AND subject = $3",
            &[&run.run_id, &run.owner.tenant_id, &run.owner.subject],
        )
        .await
        .expect("row is visible to the second connection");
    let value: serde_json::Value = row.get(0);
    let bytes = serde_json::to_vec(&value).expect("jsonb re-serializes");
    let reloaded = IxStoredRun::from_json_slice(&bytes)
        .expect("persisted bytes satisfy the Framework decode contract");
    assert_eq!(reloaded, stage.stored);
    drop(client);
    connection_task.abort();

    // Unknown principals never see the row.
    let stranger = IxPrincipalBinding {
        subject: "someone.else".to_string(),
        ..run.owner.clone()
    };
    assert_eq!(
        repo.load_stored_run(&stranger, &run.run_id)
            .await
            .expect("scoped load succeeds"),
        None
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_run_is_idempotent_and_rejects_non_initial_versions() {
    let Some(url) = proof_url("create_run_is_idempotent_and_rejects_non_initial_versions") else {
        return;
    };
    let repo = repository(&url);
    let run = harvest_run().expect("run harvests");

    let first = persist_first_stage(&repo, &run).await;
    assert!(!first.deduplicated());

    let retry = repo
        .persist_created_run(
            &run.stages[0].stored,
            &run.start_audit_fact,
            run.stages[0].stored.start_idempotency_key(),
        )
        .await
        .expect("identical retry is deduplicated, not duplicated");
    assert!(retry.deduplicated());
    assert_eq!(retry.commit_id(), first.commit_id());

    // A later durable version can never masquerade as a creation.
    assert_eq!(
        repo.persist_created_run(
            &run.stages[1].stored,
            &run.start_audit_fact,
            run.stages[1].stored.start_idempotency_key(),
        )
        .await,
        Err(IxContractError::InvalidLifecycle)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn compare_and_append_swaps_only_on_the_canonical_cursor() {
    let Some(url) = proof_url("compare_and_append_swaps_only_on_the_canonical_cursor") else {
        return;
    };
    let repo = repository(&url);
    let run = harvest_run().expect("run harvests");
    let v1 = &run.stages[0];
    let v2 = &run.stages[1];
    let v3 = &run.stages[2];
    persist_first_stage(&repo, &run).await;

    // Fresh apply advances the canonical cursor.
    let fresh = repo
        .compare_and_append_stored(
            &run.owner,
            v1.stored.repository_version(),
            &v1.stored.snapshot().cursor,
            &v2.appended_envelope,
            &v2.stored,
            None,
        )
        .await
        .expect("fresh append applies");
    assert!(!fresh.deduplicated());
    assert_eq!(fresh.cursor(), v2.stored.snapshot().cursor);
    assert_eq!(fresh.revision(), 2);

    // Retrying the identical transaction deduplicates instead of forking.
    let retry = repo
        .compare_and_append_stored(
            &run.owner,
            v1.stored.repository_version(),
            &v1.stored.snapshot().cursor,
            &v2.appended_envelope,
            &v2.stored,
            None,
        )
        .await
        .expect("identical retry resolves");
    assert!(retry.deduplicated());
    assert_eq!(retry.commit_id(), v2.appended_envelope.commit_id);

    // A stale expectation (correct version arithmetic, wrong cursor) is a
    // durable conflict, detected by the database row, not by memory.
    assert_eq!(
        repo.compare_and_append_stored(
            &run.owner,
            v2.stored.repository_version(),
            &v1.stored.snapshot().cursor,
            &v3.appended_envelope,
            &v3.stored,
            None,
        )
        .await,
        Err(IxContractError::InvalidLifecycle)
    );

    // The correctly fenced append still lands afterwards.
    let advanced = repo
        .compare_and_append_stored(
            &run.owner,
            v2.stored.repository_version(),
            &v2.stored.snapshot().cursor,
            &v3.appended_envelope,
            &v3.stored,
            None,
        )
        .await
        .expect("fenced append applies");
    assert!(!advanced.deduplicated());
    assert_eq!(advanced.revision(), 3);

    // Appending onto a run that was never created is reported as missing.
    let ghost = harvest_run().expect("second run harvests");
    assert_eq!(
        repo.compare_and_append_stored(
            &ghost.owner,
            ghost.stages[0].stored.repository_version(),
            &ghost.stages[0].stored.snapshot().cursor,
            &ghost.stages[1].appended_envelope,
            &ghost.stages[1].stored,
            None,
        )
        .await,
        Err(IxContractError::RunNotFound)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn audit_projection_marking_is_idempotent_per_dedup_key() {
    let Some(url) = proof_url("audit_projection_marking_is_idempotent_per_dedup_key") else {
        return;
    };
    let repo = repository(&url);
    let run = harvest_run().expect("run harvests");
    persist_first_stage(&repo, &run).await;

    let projection = IxAuditProjection::for_cancel(
        &run.owner.tenant_id,
        &run.run_id,
        "nexus-cancel-0001",
        format!("ix1.{}.2", run.run_id),
    )
    .expect("projection identity is key-valid");

    repo.mark_audit_projection_state(&projection, IxAuditProjectionState::Pending)
        .await
        .expect("pending mark persists");
    repo.mark_audit_projection_state(&projection, IxAuditProjectionState::Projected)
        .await
        .expect("projected mark persists");
    repo.mark_audit_projection_state(&projection, IxAuditProjectionState::Projected)
        .await
        .expect("re-marking the same projection is idempotent");

    let (client, connection) = tokio_postgres::connect(&url, NoTls)
        .await
        .expect("verification connection opens");
    let connection_task = tokio::spawn(connection);
    let row = client
        .query_one(
            "SELECT count(*)::bigint, min(projection_state) \
             FROM nexus_ix.ix_projection_cursors WHERE id = $1",
            &[&projection.dedup_key()],
        )
        .await
        .expect("projection row is queryable");
    let count: i64 = row.get(0);
    let state: Option<String> = row.get(1);
    assert_eq!(count, 1, "dedup key holds exactly one row");
    assert_eq!(state.as_deref(), Some("projected"));
    drop(client);
    connection_task.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn audit_sink_projection_is_at_least_once_with_content_dedup() {
    let Some(url) = proof_url("audit_sink_projection_is_at_least_once_with_content_dedup") else {
        return;
    };
    let repo = repository(&url);
    let run = harvest_run().expect("run harvests");
    persist_first_stage(&repo, &run).await;

    let sink = NexusIxAuditSink::new();
    // The start fact was already projected during create_run: flushing it
    // again must deduplicate by content address.
    sink.append(&run.start_audit_fact).expect("fact accepted");
    // A cancel-disposition fact for the same run is new evidence.
    let cancel_fact = appfw_runtime::ix::IxAuditEvent {
        schema_version: IX_AUDIT_SCHEMA_VERSION.to_string(),
        event_kind: IxAuditEventKind::CancelDisposition,
        request_id: run.start_audit_fact.request_id.clone(),
        correlation_id: run.start_audit_fact.correlation_id.clone(),
        principal: run.owner.clone(),
        run_id: run.run_id.clone(),
        intent_key: None,
        cancel_disposition: Some(appfw_runtime::ix::IxAuditCancelDisposition::Accepted),
    };
    sink.append(&cancel_fact).expect("fact accepted");
    // A fact whose run row does not exist stays retained (foreign-key
    // posture), never silently dropped.
    let orphan = appfw_runtime::ix::IxAuditEvent {
        run_id: "f8b1f9a2-0000-4000-8000-000000000000".to_string(),
        ..cancel_fact.clone()
    };
    sink.append(&orphan).expect("fact accepted");

    let first = sink
        .flush_to_postgres(&repo)
        .await
        .expect("first projection pass succeeds");
    assert_eq!(first.inserted, 1, "cancel fact is new");
    assert_eq!(first.deduplicated, 1, "start fact already projected");
    assert_eq!(first.retained, 1, "orphan fact is retained, not dropped");
    assert_eq!(sink.buffered().len(), 1);

    let second = sink
        .flush_to_postgres(&repo)
        .await
        .expect("second projection pass succeeds");
    assert_eq!(second.inserted, 0);
    assert_eq!(second.deduplicated, 0);
    assert_eq!(second.retained, 1, "orphan is still explicit");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn supplemental_expand_constraints_are_live() {
    let Some(url) = proof_url("supplemental_expand_constraints_are_live") else {
        return;
    };
    let (client, connection) = tokio_postgres::connect(&url, NoTls)
        .await
        .expect("catalog connection opens");
    let connection_task = tokio::spawn(connection);
    for index_name in [
        "ux_ix_runs_tenant_start_idempotency_key",
        "ux_ix_commit_envelopes_run_id_sequence",
        "ux_ix_projection_cursors_dedup_identity",
    ] {
        let row = client
            .query_one(
                "SELECT count(*)::bigint FROM pg_indexes \
                 WHERE schemaname = 'nexus_ix' AND indexname = $1",
                &[&index_name],
            )
            .await
            .expect("catalog is queryable");
        let count: i64 = row.get(0);
        assert_eq!(
            count, 1,
            "supplemental constraint {index_name} must be applied by the \
             ix_real_vertical_constraints expand migration"
        );
    }
    drop(client);
    connection_task.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tenant_mismatch_flush_retains_as_missing_and_does_not_insert() {
    let Some(url) = proof_url("tenant_mismatch_flush_retains_as_missing_and_does_not_insert")
    else {
        return;
    };
    let repo = repository(&url);
    let run = harvest_run().expect("run harvests");
    persist_first_stage(&repo, &run).await;

    let stranger = IxPrincipalBinding {
        tenant_id: "tenant_b".to_string(),
        ..run.owner.clone()
    };
    let mismatch = appfw_runtime::ix::IxAuditEvent {
        schema_version: IX_AUDIT_SCHEMA_VERSION.to_string(),
        event_kind: IxAuditEventKind::CancelDisposition,
        request_id: run.start_audit_fact.request_id.clone(),
        correlation_id: run.start_audit_fact.correlation_id.clone(),
        principal: stranger,
        run_id: run.run_id.clone(),
        intent_key: None,
        cancel_disposition: Some(appfw_runtime::ix::IxAuditCancelDisposition::Accepted),
    };
    let sink = NexusIxAuditSink::new();
    sink.append(&mismatch).expect("fact accepted");
    let report = sink
        .flush_to_postgres(&repo)
        .await
        .expect("tenant-mismatch flush is retain-as-missing, not an insert");
    assert_eq!(report.inserted, 0);
    assert_eq!(report.deduplicated, 0);
    assert_eq!(report.retained, 1);
    assert_eq!(sink.buffered().len(), 1);

    let fact_id = audit_fact_identity(
        &serde_json::to_value(&mismatch).expect("mismatch fact serializes"),
    );
    let (client, connection) = tokio_postgres::connect(&url, NoTls)
        .await
        .expect("verification connection opens");
    let connection_task = tokio::spawn(connection);
    let row = client
        .query_one(
            "SELECT count(*)::bigint FROM nexus_ix.ix_audit_facts \
             WHERE id = $1 OR tenant = $2",
            &[&fact_id, &"tenant_b"],
        )
        .await
        .expect("audit facts are queryable");
    let count: i64 = row.get(0);
    assert_eq!(count, 0, "cross-tenant flush must not insert");
    drop(client);
    connection_task.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn envelope_conflict_mismatch_is_invalid_lifecycle() {
    let Some(url) = proof_url("envelope_conflict_mismatch_is_invalid_lifecycle") else {
        return;
    };
    let repo = repository(&url);
    let run = harvest_run().expect("run harvests");
    persist_first_stage(&repo, &run).await;
    let v1 = &run.stages[0];
    let v2 = &run.stages[1];

    let (client, connection) = tokio_postgres::connect(&url, NoTls)
        .await
        .expect("collision connection opens");
    let connection_task = tokio::spawn(connection);
    client
        .execute(
            "INSERT INTO nexus_ix.ix_commit_envelopes \
             (id, run_id, sequence, event_kind, envelope, committed_at) \
             VALUES ($1, $2, 99, 'colliding', $3, now())",
            &[
                &v2.appended_envelope.commit_id,
                &run.run_id,
                &serde_json::json!({"schemaVersion": "not-the-canonical-envelope"}),
            ],
        )
        .await
        .expect("colliding envelope row inserts");
    drop(client);
    connection_task.abort();

    assert_eq!(
        repo.compare_and_append_stored(
            &run.owner,
            v1.stored.repository_version(),
            &v1.stored.snapshot().cursor,
            &v2.appended_envelope,
            &v2.stored,
            None,
        )
        .await,
        Err(IxContractError::InvalidLifecycle)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn projection_conflict_mismatch_is_invalid_lifecycle() {
    let Some(url) = proof_url("projection_conflict_mismatch_is_invalid_lifecycle") else {
        return;
    };
    let repo = repository(&url);
    let run = harvest_run().expect("run harvests");
    persist_first_stage(&repo, &run).await;

    let projection = IxAuditProjection::for_cancel(
        &run.owner.tenant_id,
        &run.run_id,
        "nexus-cancel-conflict",
        format!("ix1.{}.2", run.run_id),
    )
    .expect("projection identity is key-valid");

    let (client, connection) = tokio_postgres::connect(&url, NoTls)
        .await
        .expect("collision connection opens");
    let connection_task = tokio::spawn(connection);
    client
        .execute(
            "INSERT INTO nexus_ix.ix_projection_cursors \
             (id, tenant, run_id, command_id, cursor, projection_state, updated_at) \
             VALUES ($1, $2, $3, $4, $5, 'pending', now())",
            &[
                &projection.dedup_key(),
                &"other-tenant",
                &run.run_id,
                &projection.command_id(),
                &projection.cursor(),
            ],
        )
        .await
        .expect("colliding projection row inserts");
    drop(client);
    connection_task.abort();

    assert_eq!(
        repo.mark_audit_projection_state(&projection, IxAuditProjectionState::Pending)
            .await,
        Err(IxContractError::InvalidLifecycle)
    );
}
