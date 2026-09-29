//! Salesforce external API provider skeleton for App Framework.
//!
//! The crate is intentionally network-free for Wave 1. It models provider
//! identity, governed named read operations, safe SOQL construction, and
//! watermark request shapes. Runtime trait wiring should replace the local
//! placeholder identity/operation shims once `appfw_saas_core` and the
//! external API runtime contracts settle.

pub mod auth;
pub mod error;
pub mod identity;
pub mod metadata;
pub mod operation;
pub mod provider;
pub mod registry;
pub mod response;
pub mod soql;
pub mod watermark;

pub use auth::{
    SalesforceAuthConfigMetadata, SalesforceAuthEnvVar, SalesforceAuthFlow,
    SalesforceTenantBinding, SALESFORCE_AUTHORIZATION_HEADER, SALESFORCE_CLIENT_ID_ENV,
    SALESFORCE_CLIENT_SECRET_OR_PRIVATE_KEY_ENV, SALESFORCE_INSTANCE_URL_ENV,
    SALESFORCE_INTEGRATION_USER_AUTH_ENV_VARS, SALESFORCE_INTEGRATION_USER_AUTH_FLOWS,
    SALESFORCE_LOGIN_BASE_URL_ENV, SALESFORCE_REDACTED_BEARER_TOKEN,
    SALESFORCE_USERNAME_OR_SUBJECT_ENV,
};
pub use error::SalesforceProviderError;
pub use identity::{
    SalesforceFrameworkProvider, SalesforceProviderDescriptor, SalesforceProviderIdentity,
    SALESFORCE_PROVIDER_KEY,
};
pub use metadata::{
    salesforce_api_snapshot_metadata, salesforce_health_cloud_projection_candidates,
    salesforce_operation_metadata, salesforce_operation_metadata_catalog,
    SalesforceApiSnapshotMetadata, SalesforceCatalogObjectMetadata, SalesforceMetadataDomain,
    SalesforceOperationMetadata, SalesforceProjectionStatus, SALESFORCE_API_SPEC_PACKAGE_ID,
    SALESFORCE_API_VERSION_PIN, SALESFORCE_DOC_VERSION_PIN, SALESFORCE_HEALTH_CLOUD_CATALOG_PATH,
    SALESFORCE_HEALTH_CLOUD_OBJECT_REFERENCE_DOC_URL, SALESFORCE_IMPLEMENTATION_METADATA_PATH,
    SALESFORCE_INFORMATION_PACKAGE_VERSION, SALESFORCE_INFORMATION_PACKAGE_VERSIONS_PATH,
    SALESFORCE_RELEASE_PIN, SALESFORCE_REST_API_DOC_URL, SALESFORCE_VENDOR_KEY,
};
pub use operation::{
    SalesforceDataSensitivity, SalesforceNamedOperationRequest, SalesforceOperationAvailability,
    SalesforceOperationGate, SalesforceOperationParameter, SalesforceOperationPlan,
    SalesforceOperationPrimitive, SalesforceParameterValue, SalesforceQueryLimits,
    SalesforceQueryPlan, SalesforceRestMethod, SalesforceRestRequestPlan,
    SALESFORCE_REST_API_VERSION, SALESFORCE_REST_BASE_PATH,
};
pub use provider::SalesforceProvider;
pub use registry::{
    RegisteredSalesforceOperation, SalesforceOperationKind, SalesforceOperationRegistry,
    ACCOUNT_BY_ID_OPERATION, ACCOUNT_DESCRIBE_OBJECT_OPERATION, ACCOUNT_FETCH_BY_IDS_OPERATION,
    ACCOUNT_GET_DELETED_IDS_OPERATION, ACCOUNT_GET_UPDATED_IDS_OPERATION,
    HEALTH_CLOUD_CARE_PROGRAMS_UPDATED_SINCE_OPERATION,
    HEALTH_CLOUD_CARE_PROGRAM_ENROLLEES_BY_PROGRAM_OPERATION,
    HEALTH_CLOUD_CLINICAL_ENCOUNTERS_UPDATED_SINCE_OPERATION,
    HEALTH_CLOUD_COVERAGE_BENEFIT_ITEMS_BY_MEMBER_OPERATION,
    SALESFORCE_CONTINUE_QUERY_PAGE_OPERATION, SALESFORCE_HEALTH_DESCRIBE_OBJECT_OPERATION,
    SALESFORCE_HEALTH_FETCH_BY_IDS_OPERATION, SALESFORCE_HEALTH_FETCH_LIMITS_OPERATION,
    SALESFORCE_HEALTH_GET_DELETED_IDS_OPERATION, SALESFORCE_HEALTH_GET_UPDATED_IDS_OPERATION,
    SALESFORCE_HEALTH_QUERY_OBJECT_INCREMENTAL_OPERATION, UPDATED_ACCOUNTS_OPERATION,
};
pub use response::{
    classify_salesforce_rest_error, SalesforceApiUsageLimit, SalesforceDeletedIdsSummary,
    SalesforceDeletedRecordSummary, SalesforceDescribeObjectSummary, SalesforceLimitInfo,
    SalesforceLimitsSummary, SalesforceNextRecordsUrl, SalesforceOrgLimit,
    SalesforceQueryResponsePage, SalesforceRateLimitSummary, SalesforceRedactedRecordPayload,
    SalesforceRestError, SalesforceRestErrorClass, SalesforceUpdatedIdsSummary,
    REQUEST_LIMIT_EXCEEDED_ERROR_CODE, SALESFORCE_NEXT_RECORDS_URL_FIELD, SFORCE_LIMIT_INFO_HEADER,
};
pub use soql::{
    SalesforceField, SalesforceFilter, SalesforceLiteral, SalesforceObject, SalesforceOperator,
    SalesforceSoqlQuery, SalesforceSort, SalesforceSortDirection,
};
pub use watermark::{SalesforceWatermarkRequest, SalesforceWatermarkValue};
