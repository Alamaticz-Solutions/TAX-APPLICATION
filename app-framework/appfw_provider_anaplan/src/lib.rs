//! Anaplan Integration API external API provider skeleton.
//!
//! The crate is intentionally network-free. It models provider identity,
//! tenant/workspace/model binding, selected read-only metadata operation
//! planning, offset pagination summaries, and future export/task gates without
//! exposing raw Anaplan paths, workspace IDs, model IDs, or task IDs to callers.

pub mod auth;
pub mod error;
pub mod identity;
pub mod metadata;
pub mod operation;
pub mod provider;
pub mod registry;
pub mod response;

pub use auth::{
    AnaplanAuthConfigMetadata, AnaplanAuthEnvVar, AnaplanAuthFlow, AnaplanTenantBinding,
    ANAPLAN_API_BASE_URL_ENV, ANAPLAN_AUTHORIZATION_HEADER, ANAPLAN_AUTH_BASE_URL_ENV,
    ANAPLAN_MODEL_ID_ENV, ANAPLAN_PASSWORD_OR_CERTIFICATE_ENV, ANAPLAN_REDACTED_AUTH_TOKEN,
    ANAPLAN_USERNAME_ENV, ANAPLAN_WORKSPACE_ID_ENV,
};
pub use error::AnaplanProviderError;
pub use identity::{
    AnaplanFrameworkProvider, AnaplanProviderDescriptor, AnaplanProviderIdentity,
    ANAPLAN_PROVIDER_KEY,
};
pub use metadata::{
    anaplan_api_snapshot_metadata, anaplan_operation_metadata, anaplan_operation_metadata_catalog,
    AnaplanApiSnapshotMetadata, AnaplanMetadataDomain, AnaplanOperationMetadata,
    AnaplanProjectionStatus, ANAPLAN_API_SPEC_PACKAGE_ID, ANAPLAN_API_VERSION_PIN,
    ANAPLAN_IMPLEMENTATION_METADATA_PATH, ANAPLAN_INFORMATION_PACKAGE_VERSION,
    ANAPLAN_INFORMATION_PACKAGE_VERSIONS_PATH, ANAPLAN_INTEGRATION_API_SUMMARY_PATH,
    ANAPLAN_RELEASE_PIN, ANAPLAN_VENDOR_KEY,
};
pub use operation::{
    AnaplanDataSensitivity, AnaplanNamedOperationRequest, AnaplanOperationAvailability,
    AnaplanOperationGate, AnaplanOperationParameter, AnaplanOperationPrimitive,
    AnaplanParameterValue, AnaplanQueryLimits, AnaplanRestMethod, AnaplanRestRequestPlan,
    ANAPLAN_API_BASE_PATH,
};
pub use provider::AnaplanProvider;
pub use registry::{
    AnaplanOperationKind, AnaplanOperationRegistry, RegisteredAnaplanOperation,
    ANAPLAN_FILES_LIST_OPERATION, ANAPLAN_FILE_DOWNLOAD_CHUNK_OPERATION,
    ANAPLAN_MODEL_GET_STATUS_OPERATION, ANAPLAN_VIEW_CREATE_READ_REQUEST_OPERATION,
    ANAPLAN_VIEW_GET_READ_PAGE_OPERATION, ANAPLAN_VIEW_GET_READ_REQUEST_OPERATION,
};
pub use response::{AnaplanCollectionSummary, AnaplanPagingSummary};
