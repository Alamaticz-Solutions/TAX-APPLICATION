//! Adapter-level proof fixtures built through the un-gated Framework
//! surface, per the ratified decode-contract proof split (packet 000004,
//! decision-0002).
//!
//! The Framework's chat-gated runtime is the only author of durable
//! transactions, so adapter proof constructs [`IxStoredRun`] values the
//! sanctioned way: a real [`IxForegroundRuntime`] (composed with the product
//! context policy and audit sink) authors canonical envelopes, snapshots,
//! audit facts, and replay checkpoints; this module assembles stored-run
//! JSON from those harvested values and decodes it through
//! [`IxStoredRun::from_json_slice`] — the Framework's own repository decode
//! contract (shape, bounds, replay, and cancellation-structure checks).
//!
//! The `runtimeSeal` field is a format-valid placeholder by design: the
//! decode contract explicitly never implies HMAC acceptance, and seal
//! authentication (an `IxRuntimeService` behavior) is a named B_HOST claim
//! (DEFERRED_TO_B_HOST). Nothing here mints a parallel lifecycle or record
//! format.

use std::sync::Arc;

use appfw_runtime::extension::{RuntimePrincipalType, UserAuth};
use appfw_runtime::ix::{
    IxArtifactRevision, IxArtifactStatus, IxAuditEvent, IxAuditSink, IxCommitEnvelope,
    IxContextClaims, IxContextRevision, IxContractError, IxFocusRef, IxForegroundRuntime,
    IxPrincipalBinding, IxReplayCheckpoint, IxReplayVerifier, IxRunPolicy, IxRunRequest,
    IxRunSnapshot, IxStoredRun, IX_RUN_REQUEST_SCHEMA_VERSION, IX_STORED_RUN_SCHEMA_VERSION,
    PDS_IX_PRESENTATION_SCHEMA_VERSION,
};
use appfw_runtime::observability::RequestContext;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::audit::NexusIxAuditSink;
use super::context_policy::NexusIxContextPolicy;
use super::orchestration::{
    attention_items, compose_my_work_presentation, my_work_recipe_registration,
    resolve_my_work_context, sample_items, NexusIxContextResolution, NEXUS_MY_WORK_ARTIFACT_TYPE,
    NEXUS_MY_WORK_INTENT_KEY, NEXUS_MY_WORK_RENDERER_KEY,
};

pub(super) const FIXTURE_TENANT: &str = "tenant_a";
pub(super) const FIXTURE_SUBJECT: &str = "nexus.user";
const FIXTURE_SEAL_KEY_ID: &str = "nexus-ix-local-proof-seal@1";

/// One staged durable state harvested from the real foreground runtime.
pub(super) struct HarvestedStage {
    pub stored: IxStoredRun,
    /// The envelope this stage appended over the previous stage.
    pub appended_envelope: IxCommitEnvelope,
}

/// A complete harvested run: three consecutive durable states of the same
/// run (start acknowledgement, resolved context, published brief artifact).
pub(super) struct HarvestedRun {
    pub owner: IxPrincipalBinding,
    pub run_id: String,
    /// The runtime-authored start audit fact captured by the product sink.
    pub start_audit_fact: IxAuditEvent,
    pub stages: Vec<HarvestedStage>,
}

pub(super) fn owner_auth() -> UserAuth {
    UserAuth {
        tenant_id: FIXTURE_TENANT.to_string(),
        user_name: FIXTURE_SUBJECT.to_string(),
        timezone: "UTC".to_string(),
        principal_type: RuntimePrincipalType::User,
        on_behalf_of: None,
        ingress: None,
        roles: vec!["nexus-operations".to_string()],
        scopes: vec!["nexus:my-work.read".to_string()],
        token: "synthetic-local-proof".to_string(),
    }
}

fn runner_auth() -> UserAuth {
    UserAuth {
        tenant_id: FIXTURE_TENANT.to_string(),
        user_name: "nexus-ix-runner".to_string(),
        timezone: "UTC".to_string(),
        principal_type: RuntimePrincipalType::Agent,
        on_behalf_of: None,
        ingress: None,
        roles: vec![],
        scopes: vec!["appfw:ix.progress".to_string()],
        token: "synthetic-local-proof".to_string(),
    }
}

fn policy_authority_auth() -> UserAuth {
    UserAuth {
        tenant_id: FIXTURE_TENANT.to_string(),
        user_name: "nexus-ix-policy-authority".to_string(),
        timezone: "UTC".to_string(),
        principal_type: RuntimePrincipalType::Service,
        on_behalf_of: None,
        ingress: None,
        roles: vec![],
        scopes: vec!["appfw:ix.authorize".to_string()],
        token: "synthetic-local-proof".to_string(),
    }
}

fn request() -> IxRunRequest {
    IxRunRequest {
        schema_version: IX_RUN_REQUEST_SCHEMA_VERSION.to_string(),
        intent_key: NEXUS_MY_WORK_INTENT_KEY.to_string(),
        focus: IxFocusRef {
            kind: "nexus.work-queue".to_string(),
            id: "my-work".to_string(),
        },
        question: None,
        related_artifact: None,
    }
}

/// Harvests one authorized context revision through the real runtime start
/// path (used by the context-policy unit tests).
pub(crate) fn harvested_initial_context_revision(
    policy: &NexusIxContextPolicy,
) -> IxContextRevision {
    let audit: Arc<dyn IxAuditSink> = Arc::new(NexusIxAuditSink::new());
    let runtime = IxForegroundRuntime::new(audit, &policy_authority_auth(), Arc::new(*policy))
        .expect("foreground runtime composes");
    let owner = owner_auth();
    let receipt = runtime
        .start(
            &RequestContext::new(Uuid::new_v4().to_string(), Uuid::new_v4().to_string()),
            &owner,
            IxContextClaims::from_authenticated_human(
                "nexus-ix-local-release@1",
                "nexus-session-0001",
                &owner,
            )
            .expect("claims validate"),
            request(),
            IxRunPolicy::new_for_recipe(
                "Steward my work attention",
                &runner_auth(),
                my_work_recipe_registration().expect("registration validates"),
            )
            .expect("run policy validates"),
        )
        .expect("run starts");
    runtime
        .snapshot(&owner, &receipt.run_id)
        .expect("snapshot readable")
        .authorization_context
}

/// Drives one complete run through the real foreground runtime and harvests
/// three consecutive durable stages.
pub(super) fn harvest_run() -> Result<HarvestedRun, IxContractError> {
    let sink = Arc::new(NexusIxAuditSink::new());
    let runtime = IxForegroundRuntime::new(
        Arc::clone(&sink) as Arc<dyn IxAuditSink>,
        &policy_authority_auth(),
        Arc::new(NexusIxContextPolicy::new()),
    )?;
    let owner = owner_auth();
    let runner = runner_auth();
    let request_context =
        RequestContext::new(Uuid::new_v4().to_string(), Uuid::new_v4().to_string());
    let claims = IxContextClaims::from_authenticated_human(
        "nexus-ix-local-release@1",
        format!("nexus-session-{}", Uuid::new_v4()),
        &owner,
    )?;
    let policy = IxRunPolicy::new_for_recipe(
        "Steward my work attention",
        &runner,
        my_work_recipe_registration()?,
    )?;
    let receipt = runtime.start(&request_context, &owner, claims, request(), policy)?;
    let run_id = receipt.run_id.clone();
    let start_audit_fact = sink
        .buffered()
        .first()
        .cloned()
        .ok_or(IxContractError::AuditUnavailable)?;
    let mut stages = Vec::new();
    stages.push(harvest_stage(&runtime, &owner, &run_id, &start_audit_fact)?);

    // Stage 2: the product-resolved context becomes the canonical
    // ContextResolved commit.
    let items = sample_items();
    let resolution = resolve_my_work_context(
        FIXTURE_TENANT,
        FIXTURE_SUBJECT,
        "2026-08-16T00:00:00Z",
        &items,
    );
    let NexusIxContextResolution::Available(context) = resolution else {
        return Err(IxContractError::InvalidLifecycle);
    };
    let authority = runtime
        .snapshot(&owner, &run_id)?
        .authorization_context
        .binding();
    runtime.resolve_context(&runner, &run_id, &authority, context.summary.clone())?;
    stages.push(harvest_stage(&runtime, &owner, &run_id, &start_audit_fact)?);

    // Stage 3: the composed My Work brief becomes a canonical artifact
    // revision, proving the orchestration output is lifecycle-grade.
    let attention = attention_items(&items);
    let changed = attention
        .iter()
        .map(|item| item.record_locator.as_str())
        .collect::<Vec<_>>();
    let presentation_id = format!("nexus-my-work-brief-{}", &run_id[..8]);
    let presentation =
        compose_my_work_presentation(&presentation_id, 1, &context, &items, &changed);
    let artifact = IxArtifactRevision {
        artifact_id: presentation_id,
        artifact_type: NEXUS_MY_WORK_ARTIFACT_TYPE.to_string(),
        content_schema_version: PDS_IX_PRESENTATION_SCHEMA_VERSION.to_string(),
        renderer_key: NEXUS_MY_WORK_RENDERER_KEY.to_string(),
        revision: 1,
        status: IxArtifactStatus::Ready,
        changed_region_ids: changed
            .iter()
            .map(|value| format!("work-{value}"))
            .collect(),
        presentation,
    };
    runtime.publish_artifact(&runner, &run_id, &authority, artifact)?;
    stages.push(harvest_stage(&runtime, &owner, &run_id, &start_audit_fact)?);

    let owner_binding = stages[0].stored.snapshot().owner.clone();
    Ok(HarvestedRun {
        owner: owner_binding,
        run_id,
        start_audit_fact,
        stages,
    })
}

/// Assembles and decodes the durable stored-run state currently visible in
/// the runtime, exactly as a repository row would round-trip it.
fn harvest_stage(
    runtime: &IxForegroundRuntime,
    owner: &UserAuth,
    run_id: &str,
    start_audit_fact: &IxAuditEvent,
) -> Result<HarvestedStage, IxContractError> {
    let batch = runtime.replay_after(owner, run_id, None)?;
    let checkpoint = runtime.replay_checkpoint(owner, run_id, None)?;
    let stored = assemble_stored_run(
        &batch.base_snapshot,
        &batch.envelopes,
        &batch.expected_snapshot,
        &checkpoint,
        start_audit_fact,
        run_id,
    )?;
    let appended_envelope = batch
        .envelopes
        .last()
        .cloned()
        .ok_or(IxContractError::InvalidLifecycle)?;
    Ok(HarvestedStage {
        stored,
        appended_envelope,
    })
}

/// Assembles stored-run JSON from Framework-authored components and decodes
/// it under the Framework decode contract.
pub(super) fn assemble_stored_run(
    seed: &IxRunSnapshot,
    envelopes: &[IxCommitEnvelope],
    result: &IxRunSnapshot,
    checkpoint: &IxReplayCheckpoint,
    start_audit_fact: &IxAuditEvent,
    run_id: &str,
) -> Result<IxStoredRun, IxContractError> {
    let value = stored_run_value(seed, envelopes, result, checkpoint, start_audit_fact, run_id)?;
    let bytes = serde_json::to_vec(&value).map_err(|_| IxContractError::InvalidLifecycle)?;
    IxStoredRun::from_json_slice(&bytes)
}

/// Raw stored-run JSON assembly (also used to prove tamper rejection). The
/// snapshot chain is rebuilt through the public replay verifier.
pub(super) fn stored_run_value(
    seed: &IxRunSnapshot,
    envelopes: &[IxCommitEnvelope],
    result: &IxRunSnapshot,
    checkpoint: &IxReplayCheckpoint,
    start_audit_fact: &IxAuditEvent,
    run_id: &str,
) -> Result<Value, IxContractError> {
    let mut chain = vec![seed.clone()];
    for envelope in envelopes {
        let next = IxReplayVerifier::apply(chain.last().expect("chain is non-empty"), envelope)?;
        chain.push(next);
    }
    Ok(json!({
        "schemaVersion": IX_STORED_RUN_SCHEMA_VERSION,
        "repositoryVersion": envelopes.len() as u64,
        "seedSnapshot": as_json(seed)?,
        "snapshot": as_json(result)?,
        "envelopes": as_json(&envelopes)?,
        "snapshots": as_json(&chain)?,
        "checkpoint": as_json(checkpoint)?,
        "startAuditFact": as_json(start_audit_fact)?,
        "startIdempotencyKey": format!("ix-start:{run_id}"),
        "runtimeSealKeyId": FIXTURE_SEAL_KEY_ID,
        // Format-valid placeholder: repository decoding never implies MAC
        // acceptance (Framework decode contract); seal authentication is the
        // chat-gated IxRuntimeService's B_HOST-deferred claim.
        "runtimeSeal": format!(
            "hmac-sha256:{:x}",
            Sha256::digest(b"nexus-ix-adapter-shape-proof")
        ),
    }))
}

fn as_json<T: serde::Serialize>(value: &T) -> Result<Value, IxContractError> {
    serde_json::to_value(value).map_err(|_| IxContractError::InvalidLifecycle)
}

#[cfg(test)]
mod tests {
    use appfw_runtime::ix::IxRunState;

    use super::*;

    #[test]
    fn harvested_stages_decode_under_the_framework_decode_contract() {
        let run = harvest_run().expect("run harvests");
        assert_eq!(run.stages.len(), 3);
        for (index, stage) in run.stages.iter().enumerate() {
            assert_eq!(stage.stored.repository_version(), (index + 1) as u64);
            stage
                .stored
                .validate()
                .expect("assembled stored run satisfies validate()");
        }
        let final_snapshot = run.stages[2].stored.snapshot();
        assert_eq!(final_snapshot.state, IxRunState::Running);
        assert_eq!(final_snapshot.artifacts.len(), 1);
        assert_eq!(run.start_audit_fact.run_id, run.run_id);
        assert_eq!(run.owner.tenant_id, FIXTURE_TENANT);
    }

    #[test]
    fn tampered_stored_run_json_is_rejected_by_the_decode_contract() {
        let run = harvest_run().expect("run harvests");
        let stage = &run.stages[1];
        let batch_envelopes = stage.stored.envelopes().to_vec();
        let mut value = stored_run_value(
            stage.stored.seed_snapshot(),
            &batch_envelopes,
            stage.stored.snapshot(),
            stage.stored.checkpoint(),
            &run.start_audit_fact,
            &run.run_id,
        )
        .expect("value assembles");

        // Unknown fields are refused (closed decoding).
        value["adapterAuthoredField"] = json!("not-allowed");
        let bytes = serde_json::to_vec(&value).expect("serializes");
        assert!(IxStoredRun::from_json_slice(&bytes).is_err());

        // A repository version that disagrees with the envelope chain is
        // refused by the replay checks.
        let mut versions = stored_run_value(
            stage.stored.seed_snapshot(),
            &batch_envelopes,
            stage.stored.snapshot(),
            stage.stored.checkpoint(),
            &run.start_audit_fact,
            &run.run_id,
        )
        .expect("value assembles");
        versions["repositoryVersion"] = json!(7);
        let bytes = serde_json::to_vec(&versions).expect("serializes");
        assert!(IxStoredRun::from_json_slice(&bytes).is_err());
    }
}
