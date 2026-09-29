//! ServiceNow evidence-gated provider skeleton.
//!
//! This crate intentionally does not build ServiceNow Table API requests yet.
//! The 2026-06-26 corpus confirms likely Table API patterns, but table fields,
//! ACLs, plugins, domain separation, and OpenAPI exports are instance-specific.
//! The crate records the planned operation catalog and validates server-owned
//! tenant metadata while rejecting reads until authenticated export evidence
//! exists and rejecting writes until live G1 governed-write evidence exists.

pub mod auth;
pub mod error;
pub mod identity;
pub mod metadata;
pub mod provider;
pub mod registry;

pub use auth::{
    ServiceNowAuthConfigMetadata, ServiceNowAuthEnvVar, ServiceNowAuthFlow,
    ServiceNowTenantBinding, SERVICENOW_AUTH_MODE_ENV, SERVICENOW_BASE_URL_ENV,
    SERVICENOW_CLIENT_ID_ENV, SERVICENOW_CLIENT_SECRET_ENV, SERVICENOW_DOMAIN_SCOPE_ENV,
    SERVICENOW_INTEGRATION_PRINCIPAL_ENV, SERVICENOW_PASSWORD_ENV, SERVICENOW_USERNAME_ENV,
};
pub use error::ServiceNowProviderError;
pub use identity::{
    ServiceNowProviderDescriptor, ServiceNowProviderIdentity, SERVICENOW_API_FAMILY,
    SERVICENOW_PROVIDER_KEY, SERVICENOW_RELEASE_BASIS,
};
pub use metadata::{
    servicenow_api_snapshot_metadata, servicenow_operation_metadata,
    servicenow_operation_metadata_catalog, ServiceNowApiSnapshotMetadata, ServiceNowMetadataDomain,
    ServiceNowOperationMetadata, ServiceNowProjectionStatus, SERVICENOW_API_SPEC_PACKAGE_ID,
    SERVICENOW_IMPLEMENTATION_METADATA_PATH, SERVICENOW_INFORMATION_PACKAGE_VERSION,
    SERVICENOW_INFORMATION_PACKAGE_VERSIONS_PATH, SERVICENOW_PUBLIC_DOCS_SUMMARY_PATH,
    SERVICENOW_VENDOR_KEY,
};
pub use provider::ServiceNowProvider;
pub use registry::{
    RegisteredServiceNowOperation, ServiceNowNamedOperationRequest, ServiceNowOperationGate,
    ServiceNowOperationKind, ServiceNowOperationRegistry, SERVICENOW_INCIDENT_CREATE_OPERATION,
    SERVICENOW_INCIDENT_CREATE_POLICY_SCOPE, SERVICENOW_TABLE_EXPORT_SCHEMA_OPERATION,
    SERVICENOW_TABLE_FETCH_BY_SYS_IDS_OPERATION, SERVICENOW_TABLE_FETCH_REFERENCE_VALUES_OPERATION,
    SERVICENOW_TABLE_QUERY_INCREMENTAL_OPERATION,
};
