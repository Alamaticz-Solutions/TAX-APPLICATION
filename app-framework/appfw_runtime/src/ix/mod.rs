//! Provider-neutral Intelligent Experience foreground lifecycle.
//!
//! This module owns identity-bound lifecycle, atomic semantic commits,
//! cancellation fencing, artifact revision trust, and bounded replay. Product
//! code owns domain meaning and authorized context retrieval; PDS owns
//! presentation and rendering.

mod contract;
mod lifecycle;
#[cfg(feature = "chat")]
mod orchestration;
mod presentation;
mod recipe;
#[cfg(feature = "chat")]
mod transport;

#[cfg(feature = "chat")]
pub use crate::auth::IxJwtVerifier;
#[cfg(feature = "chat")]
pub use crate::routing::ix_transport_routes;
pub use contract::{
    IxArtifactBinding, IxArtifactRevision, IxArtifactState, IxArtifactStatus,
    IxAuditCancelDisposition, IxAuditEvent, IxAuditEventKind, IxAuditSink, IxCancelDisposition,
    IxCancelReceipt, IxCancellationFence, IxCancellationStage, IxCommitEnvelope, IxConsentState,
    IxContextClaims, IxContextFreshness, IxContextProposal, IxContextRevision, IxContextSummary,
    IxContractError, IxDataClassification, IxEgressPosture, IxEvent, IxEventPayload,
    IxExecutionAuthority, IxFocusRef, IxPhase, IxPolicyDecisionProvenance, IxPolicyVerdict,
    IxPrincipalBinding, IxRelatedArtifactRef, IxReplayBatch, IxReplayCheckpoint, IxRunPolicy,
    IxRunRequest, IxRunSnapshot, IxRunState, IxSourceSummary, IxStartReceipt, IxTerminalOutcome,
    IX_AUDIT_SCHEMA_VERSION, IX_EVENT_SCHEMA_VERSION, IX_REPLAY_CHECKPOINT_SCHEMA_VERSION,
    IX_REPLAY_SCHEMA_VERSION, IX_RUN_REQUEST_SCHEMA_VERSION, PDS_IX_PRESENTATION_SCHEMA_VERSION,
};
#[cfg(feature = "chat")]
pub use lifecycle::IxStoredRunSealKey;
pub use lifecycle::{
    IxAcceptedCancel, IxAuditProjection, IxAuditProjectionState, IxCancellationSignal,
    IxCancellationSignalState, IxContextPolicy, IxForegroundRuntime, IxReplayVerifier,
    IxRepositoryCommitReceipt, IxRunCommitTransaction, IxRunCreateTransaction, IxRunRepository,
    IxStoredRun, IX_STORED_RUN_SCHEMA_VERSION,
};
#[cfg(feature = "chat")]
pub use orchestration::{
    IxOrchestrationDraft, IxPreparedRun, IxProductContext, IxProductContextResolver,
    IxProductOrchestrator,
};
pub use recipe::{
    IxRecipeRegistration, PDS_IX_RECIPE_REGISTRATION_SCHEMA_VERSION,
    PDS_IX_RECIPE_REGISTRY_SCHEMA_VERSION,
};
#[cfg(feature = "chat")]
pub(crate) use transport::LAST_EVENT_ID_HEADER;
#[cfg(feature = "chat")]
pub use transport::{
    IxAuditProjectedPosture, IxCanonicalEnvelopeStream, IxClientBinding, IxClientProfile,
    IxReplayCursor, IxRuntimeIntentPolicy, IxRuntimeService, IxTransportCancelReceipt,
    IxTransportError, IxTransportPolicy, IxTransportStream, IxVerifiedHuman,
    IX_CALLER_PROFILE_HEADER, IX_CANCEL_PATH, IX_CANCEL_REQUEST_SCHEMA_VERSION, IX_STREAM_PATH,
};

#[cfg(test)]
mod boundary_tests {
    #[test]
    fn source_has_no_provider_chat_transport_or_pds_package_dependency() {
        let implementation = concat!(include_str!("contract.rs"), include_str!("lifecycle.rs"));
        for forbidden in [
            "crate::chat",
            "crate::ingress",
            "crate::provider",
            "crate::providers",
            "gemini",
            "axum::",
            "reqwest::",
            "eventsource",
            "pds_health",
            "@appfw/pds-health",
            "react::",
        ] {
            assert!(
                !implementation.to_ascii_lowercase().contains(forbidden),
                "IX lifecycle must not depend on excluded surface {forbidden}"
            );
        }

        let module_declarations = include_str!("mod.rs")
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with("mod ") && line.ends_with(';'))
            .collect::<Vec<_>>();
        assert_eq!(
            module_declarations,
            [
                "mod contract;",
                "mod lifecycle;",
                "mod orchestration;",
                "mod presentation;",
                "mod recipe;",
                "mod transport;"
            ]
        );
    }
}
