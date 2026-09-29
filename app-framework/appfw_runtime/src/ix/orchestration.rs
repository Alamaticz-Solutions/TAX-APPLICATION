//! Product extension contracts for IX context and orchestration.
//!
//! Product authorization and the Product resolver own detailed context and
//! its handling. The framework lifecycle remains the only authority that
//! stamps events, revisions, cursors, cancellation fences, and terminal
//! outcomes.

use std::sync::Arc;

use futures_util::stream::BoxStream;
use serde_json::Value;

use super::{
    contract::{
        validate_detail, validate_key, validate_portable_json, validate_serialized_json_size,
        IxArtifactStatus, IxContextSummary, IxPhase, IxPrincipalBinding, IxRunRequest,
    },
    IxCancellationSignal, IxContractError,
};

const IX_MAX_PRODUCT_CONTEXT_BYTES: usize = 512 * 1024;

/// Authorized, server-resolved Product context.
///
/// Product authorization and the Product resolver decide which detailed
/// context is appropriate. The resolver must exclude credentials, access
/// tokens, API tokens, security tokens, bearer tokens, and other secrets
/// before constructing this value. Otherwise-authorized classified context
/// is not categorically forbidden by this Framework type.
///
/// Construction bounds portable JSON structure and serialized size. It does
/// not interpret Product semantics, detect secrets, or grant data-policy
/// approval. Product code can clone and retain the in-memory value and
/// therefore owns storage, classification, consent, retention, and egress.
/// The absence of `Debug` and `Serialize` reduces accidental exposure; it is
/// not a lifetime, persistence, or policy guarantee.
#[derive(Clone)]
pub struct IxProductContext {
    context_id: String,
    summary: IxContextSummary,
    payload: Arc<Value>,
}

impl IxProductContext {
    pub fn new(
        context_id: impl Into<String>,
        summary: IxContextSummary,
        payload: Value,
    ) -> Result<Self, IxContractError> {
        let context_id = context_id.into();
        validate_key("productContext.contextId", &context_id)?;
        summary.validate()?;
        validate_portable_json(&payload)?;
        validate_serialized_json_size(&payload, IX_MAX_PRODUCT_CONTEXT_BYTES)?;
        Ok(Self {
            context_id,
            summary,
            payload: Arc::new(payload),
        })
    }

    pub fn context_id(&self) -> &str {
        &self.context_id
    }

    pub fn summary(&self) -> &IxContextSummary {
        &self.summary
    }

    /// Exposes detailed context to the Product orchestrator. Product code can
    /// clone or retain the context, so storage, classification, consent,
    /// retention, and egress remain Product responsibilities.
    pub fn expose_to_product(&self) -> &Value {
        &self.payload
    }
}

/// In-memory input to a Product orchestrator. This container is not a
/// lifetime, persistence, classification, or policy guarantee.
#[derive(Clone)]
pub struct IxPreparedRun {
    run_id: String,
    principal: IxPrincipalBinding,
    request: IxRunRequest,
    context: IxProductContext,
}

impl IxPreparedRun {
    pub(crate) fn new(
        run_id: String,
        principal: IxPrincipalBinding,
        request: IxRunRequest,
        context: IxProductContext,
    ) -> Result<Self, IxContractError> {
        validate_key("preparedRun.runId", &run_id)?;
        request.validate()?;
        if principal.tenant_id.trim().is_empty() || principal.subject.trim().is_empty() {
            return Err(IxContractError::UnauthorizedActor);
        }
        Ok(Self {
            run_id,
            principal,
            request,
            context,
        })
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn principal(&self) -> &IxPrincipalBinding {
        &self.principal
    }

    pub fn request(&self) -> &IxRunRequest {
        &self.request
    }

    pub fn context(&self) -> &IxProductContext {
        &self.context
    }
}

/// Product output is a draft command, never a canonical IX event.
#[derive(Clone, Debug, PartialEq)]
pub enum IxOrchestrationDraft {
    Phase {
        phase: IxPhase,
        reason: String,
    },
    Artifact {
        artifact_id: String,
        artifact_type: String,
        content_schema_version: String,
        renderer_key: String,
        revision: u64,
        status: IxArtifactStatus,
        presentation: Value,
        changed_region_ids: Vec<String>,
    },
    WaitingForUser {
        prompt: String,
    },
    Completed {
        message: String,
    },
    Partial {
        message: String,
    },
}

impl IxOrchestrationDraft {
    pub fn validate(&self) -> Result<(), IxContractError> {
        match self {
            Self::Phase { phase, reason } => {
                if *phase == IxPhase::Acknowledged {
                    return Err(IxContractError::InvalidLifecycle);
                }
                validate_detail("orchestration.phaseReason", reason)
            }
            Self::Artifact {
                artifact_id,
                artifact_type,
                content_schema_version,
                renderer_key,
                revision,
                presentation,
                changed_region_ids,
                ..
            } => {
                validate_key("orchestration.artifactId", artifact_id)?;
                validate_key("orchestration.artifactType", artifact_type)?;
                validate_key("orchestration.contentSchemaVersion", content_schema_version)?;
                validate_key("orchestration.rendererKey", renderer_key)?;
                if *revision == 0 {
                    return Err(IxContractError::InvalidArtifactRevision);
                }
                if changed_region_ids.len() > 128 {
                    return Err(IxContractError::ResourceLimitExceeded);
                }
                for region_id in changed_region_ids {
                    validate_key("orchestration.changedRegionId", region_id)?;
                }
                validate_portable_json(presentation)
            }
            Self::WaitingForUser { prompt }
            | Self::Completed { message: prompt }
            | Self::Partial { message: prompt } => validate_detail("orchestration.message", prompt),
        }
    }
}

/// Product-owned server resolver for authorized detailed context.
///
/// The resolver applies Product authorization and context policy and excludes
/// credentials, access tokens, API tokens, security tokens, bearer tokens,
/// and other secrets before constructing `IxProductContext`.
#[async_trait::async_trait]
pub trait IxProductContextResolver: Send + Sync + 'static {
    async fn resolve(
        &self,
        principal: &IxPrincipalBinding,
        request: &IxRunRequest,
        cancellation: IxCancellationSignal,
    ) -> Result<IxProductContext, IxContractError>;
}

#[async_trait::async_trait]
pub trait IxProductOrchestrator: Send + Sync + 'static {
    async fn run(
        &self,
        prepared: IxPreparedRun,
        cancellation: IxCancellationSignal,
    ) -> Result<BoxStream<'static, Result<IxOrchestrationDraft, IxContractError>>, IxContractError>;
}
