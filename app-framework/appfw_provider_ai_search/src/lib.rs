//! AI search provider contract skeleton for App Framework.
//!
//! This crate is intentionally network-free for Wave 4 CH3. It freezes the
//! provider identity, authentication metadata, named operation registry, and
//! evidence gates for enterprise AI search. It does not execute HTTP calls:
//! runtime execution waits for an authenticated PDS AI search contract/export
//! and the shared SaaS HTTP executor lane.

pub mod auth;
pub mod error;
pub mod foundation;
pub mod gateway;
pub mod identity;
pub mod metadata;
pub mod provider;
pub mod registry;

pub use auth::{
    AiSearchAuthConfigMetadata, AiSearchAuthEnvVar, AiSearchAuthFlow, AiSearchTenantBinding,
    AI_SEARCH_API_KEY_ENV, AI_SEARCH_AUTH_ENV_VARS, AI_SEARCH_AUTH_FLOWS, AI_SEARCH_BASE_URL_ENV,
    AI_SEARCH_BEARER_TOKEN_ENV, AI_SEARCH_CLIENT_ID_ENV, AI_SEARCH_CLIENT_SECRET_ENV,
    AI_SEARCH_TOKEN_URL_ENV,
};
pub use error::AiSearchProviderError;
pub use foundation::{
    validate_answer_envelope, validate_generated_view_metadata, validate_local_evaluation,
    validate_recommendation, FoundationAnswerEnvelope, FoundationEvaluation,
    FoundationRecommendation, FoundationReference, FoundationValidationError,
    FoundationViewSelection, GeneratedViewMetadata, AI_EVALUATION_VERSION, ANSWER_ENVELOPE_VERSION,
    GENERATED_VIEW_METADATA_VERSION, LOCAL_FIXTURE_MODE, RECOMMENDATION_VERSION,
};
pub use gateway::{
    ai_search_gateway_contract, AiSearchGatewayContract, AiSearchGatewayEvidence,
    AiSearchGatewayKind, AiSearchM2mTokenCacheKey, AI_SEARCH_GATEWAY_CONTRACT_DOC,
    AI_SEARCH_GATEWAY_CREDENTIAL_CUSTODY, AI_SEARCH_GATEWAY_DEPLOYMENT_POSTURE,
    AI_SEARCH_GATEWAY_EVIDENCE, AI_SEARCH_GATEWAY_KINDS, AI_SEARCH_GATEWAY_POLICY_AUTHORITY,
    AI_SEARCH_GATEWAY_VENDOR,
};
pub use identity::{
    AiSearchProviderDescriptor, AiSearchProviderIdentity, AI_SEARCH_API_FAMILY,
    AI_SEARCH_PROVIDER_KEY, AI_SEARCH_RELEASE_BASIS,
};
pub use metadata::{
    ai_search_api_snapshot_metadata, ai_search_operation_metadata,
    ai_search_operation_metadata_catalog, AiSearchApiSnapshotMetadata, AiSearchMetadataDomain,
    AiSearchOperationMetadata, AiSearchProjectionStatus, AI_SEARCH_API_SPEC_PACKAGE_ID,
    AI_SEARCH_IMPLEMENTATION_METADATA_PATH, AI_SEARCH_INFORMATION_PACKAGE_VERSION,
    AI_SEARCH_VENDOR_KEY,
};
pub use provider::AiSearchProvider;
pub use registry::{
    AiSearchNamedOperationRequest, AiSearchOperationGate, AiSearchOperationKind,
    AiSearchOperationRegistry, RegisteredAiSearchOperation, AI_SEARCH_EMBED_OPERATION,
    AI_SEARCH_SEARCH_OPERATION,
};
