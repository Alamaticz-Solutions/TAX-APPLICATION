//! iCIMS evidence-gated provider skeleton.
//!
//! The public 2026-06-26 corpus confirms iCIMS integration posture and REST API
//! usage, but detailed auth, schemas, pagination, and field access are gated
//! behind Developer Community or tenant documentation. This crate records the
//! planned operation catalog and tenant metadata while rejecting execution until
//! authenticated evidence exists.

pub mod auth;
pub mod error;
pub mod identity;
pub mod metadata;
pub mod provider;
pub mod registry;

pub use auth::{
    IcimsAuthConfigMetadata, IcimsAuthEnvVar, IcimsAuthFlow, IcimsTenantBinding,
    ICIMS_AUTH_MODE_ENV, ICIMS_BASE_URL_ENV, ICIMS_CREDENTIAL_REFERENCE_ENV, ICIMS_CUSTOMER_ID_ENV,
    ICIMS_INTEGRATION_NAME_ENV, ICIMS_MARKETPLACE_VALIDATION_ENV,
};
pub use error::IcimsProviderError;
pub use identity::{
    IcimsProviderDescriptor, IcimsProviderIdentity, ICIMS_API_FAMILY, ICIMS_PROVIDER_KEY,
    ICIMS_RELEASE_BASIS,
};
pub use metadata::{
    icims_api_snapshot_metadata, icims_operation_metadata, icims_operation_metadata_catalog,
    IcimsApiSnapshotMetadata, IcimsMetadataDomain, IcimsOperationMetadata, IcimsProjectionStatus,
    ICIMS_API_SPEC_PACKAGE_ID, ICIMS_IMPLEMENTATION_METADATA_PATH,
    ICIMS_INFORMATION_PACKAGE_VERSION, ICIMS_INFORMATION_PACKAGE_VERSIONS_PATH,
    ICIMS_PUBLIC_DOCS_SUMMARY_PATH, ICIMS_VENDOR_KEY,
};
pub use provider::IcimsProvider;
pub use registry::{
    IcimsNamedOperationRequest, IcimsOperationGate, IcimsOperationKind, IcimsOperationRegistry,
    RegisteredIcimsOperation, ICIMS_DISCOVERY_FETCH_API_CONTRACT_OPERATION,
    ICIMS_RECRUITING_QUERY_APPLICATIONS_INCREMENTAL_OPERATION,
    ICIMS_RECRUITING_QUERY_CANDIDATES_INCREMENTAL_OPERATION,
    ICIMS_RECRUITING_QUERY_JOBS_INCREMENTAL_OPERATION,
};
