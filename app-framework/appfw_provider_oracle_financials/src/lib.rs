//! Oracle Fusion Cloud Financials REST API provider skeleton.
//!
//! This crate is intentionally network-free. It turns a small, allowlisted
//! subset of the Oracle Financials 26B OpenAPI evidence into validated App
//! Framework SaaS request plans without implementing live auth or transport.

pub mod auth;
pub mod error;
pub mod identity;
pub mod metadata;
pub mod operation;
pub mod provider;
pub mod registry;
pub mod response;

pub use auth::{
    OracleFinancialsAuthConfigMetadata, OracleFinancialsAuthEnvVar, OracleFinancialsAuthFlow,
    OracleFinancialsTenantBinding, ORACLE_FINANCIALS_AUTHORIZATION_HEADER,
    ORACLE_FINANCIALS_AUTH_MODE_ENV, ORACLE_FINANCIALS_BASE_URL_ENV,
    ORACLE_FINANCIALS_CLIENT_ID_ENV, ORACLE_FINANCIALS_CLIENT_SECRET_ENV,
    ORACLE_FINANCIALS_DATA_SECURITY_SCOPE_ENV, ORACLE_FINANCIALS_METADATA_CONTEXT_ENV,
    ORACLE_FINANCIALS_PASSWORD_ENV, ORACLE_FINANCIALS_REDACTED_BEARER_TOKEN,
    ORACLE_FINANCIALS_REST_FRAMEWORK_VERSION_ENV, ORACLE_FINANCIALS_USERNAME_ENV,
};
pub use error::OracleFinancialsProviderError;
pub use identity::{
    OracleFinancialsProviderDescriptor, OracleFinancialsProviderIdentity,
    ORACLE_FINANCIALS_API_FAMILY, ORACLE_FINANCIALS_OPENAPI_VERSION,
    ORACLE_FINANCIALS_PROVIDER_KEY, ORACLE_FINANCIALS_RELEASE,
};
pub use metadata::{
    oracle_financials_api_snapshot_metadata, oracle_financials_operation_metadata,
    oracle_financials_operation_metadata_catalog, OracleFinancialsApiSnapshotMetadata,
    OracleFinancialsMetadataDomain, OracleFinancialsOperationMetadata,
    OracleFinancialsProjectionStatus, ORACLE_FINANCIALS_API_SPEC_PACKAGE_ID,
    ORACLE_FINANCIALS_IMPLEMENTATION_METADATA_PATH, ORACLE_FINANCIALS_INFORMATION_PACKAGE_VERSION,
    ORACLE_FINANCIALS_INFORMATION_PACKAGE_VERSIONS_PATH, ORACLE_FINANCIALS_OPENAPI_SUMMARY_PATH,
    ORACLE_FINANCIALS_VENDOR_KEY,
};
pub use operation::{
    oracle_financials_response_caps, OracleFinancialsDataSensitivity,
    OracleFinancialsNamedOperationRequest, OracleFinancialsOperationAvailability,
    OracleFinancialsOperationGate, OracleFinancialsOperationParameter,
    OracleFinancialsOperationPrimitive, OracleFinancialsParameterValue,
    OracleFinancialsQueryLimits, OracleFinancialsRestMethod, OracleFinancialsRestRequestPlan,
    ORACLE_FINANCIALS_LIMIT_QUERY, ORACLE_FINANCIALS_LINKS_QUERY,
    ORACLE_FINANCIALS_METADATA_CONTEXT_HEADER, ORACLE_FINANCIALS_OFFSET_QUERY,
    ORACLE_FINANCIALS_ONLY_DATA_QUERY, ORACLE_FINANCIALS_REST_FRAMEWORK_VERSION_HEADER,
    ORACLE_FINANCIALS_REST_RESOURCES_BASE_PATH, ORACLE_FINANCIALS_TOTAL_RESULTS_QUERY,
};
pub use provider::OracleFinancialsProvider;
pub use registry::{
    OracleFinancialsOperationKind, OracleFinancialsOperationRegistry,
    RegisteredOracleFinancialsOperation,
    ORACLE_FINANCIALS_ACCOUNTING_PERIOD_STATUS_LOV_GET_OPERATION,
    ORACLE_FINANCIALS_ACCOUNTING_PERIOD_STATUS_LOV_LIST_OPERATION,
    ORACLE_FINANCIALS_COLLECTION_QUERY_OPERATION, ORACLE_FINANCIALS_FETCH_BY_ID_OPERATION,
    ORACLE_FINANCIALS_LIST_LOV_OPERATION, ORACLE_FINANCIALS_OPENAPI_DESCRIBE_RESOURCE_OPERATION,
    ORACLE_FINANCIALS_SUBMIT_ERP_INTEGRATION_OPERATION,
};
pub use response::OracleFinancialsCollectionSummary;
