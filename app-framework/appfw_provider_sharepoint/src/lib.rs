//! Microsoft SharePoint (via Microsoft Graph) provider skeleton for App
//! Framework - mirrors `appfw_provider_servicenow`'s evidence-gated shape
//! (registered operations with named gates) and `appfw_provider_salesforce`'s
//! file layout.
//!
//! SharePoint's request shapes ARE publicly documented (Microsoft Graph REST
//! API on Microsoft Learn), so request-plan construction is `compiler_contracted`
//! from day one: validated and unit-tested, network-free. What is still
//! missing is framework-level wiring - there is no
//! `FrameworkProvider::SharePoint` variant in
//! `appfw_runtime::provider_keys` and no `DataSourceType` entry in `app_gen` -
//! see `docs/vendor-contract.md` section 7 and
//! `tax-doc-routing/docs/architecture/framework-issues-for-pds.md`.
//!
//! This crate is intentionally network-free: it builds and validates request
//! plans, it does not execute them. The executing implementation is the
//! product-owned `tax-doc-routing/backend/src/services/sharepoint/` module,
//! which exists precisely because that framework wiring does not exist yet.

pub mod auth;
pub mod error;
pub mod identity;
pub mod metadata;
pub mod provider;
pub mod registry;

pub use auth::{
    SharePointAuthConfigMetadata, SharePointAuthEnvVar, SharePointAuthFlow,
    SharePointTenantBinding, SHAREPOINT_AUTH_MODE_ENV, SHAREPOINT_DRIVE_ID_ENV,
    SHAREPOINT_READ_CLIENT_ID_ENV, SHAREPOINT_READ_CLIENT_SECRET_ENV, SHAREPOINT_SITE_ID_ENV,
    SHAREPOINT_TENANT_ID_ENV, SHAREPOINT_WRITE_CLIENT_ID_ENV, SHAREPOINT_WRITE_CLIENT_SECRET_ENV,
};
pub use error::SharePointProviderError;
pub use identity::{
    SharePointProviderDescriptor, SharePointProviderIdentity, SHAREPOINT_API_FAMILY,
    SHAREPOINT_PROVIDER_KEY, SHAREPOINT_RELEASE_BASIS,
};
pub use metadata::{
    sharepoint_api_snapshot_metadata, sharepoint_operation_metadata,
    sharepoint_operation_metadata_catalog, SharePointApiSnapshotMetadata,
    SharePointMetadataDomain, SharePointOperationMetadata, SharePointProjectionStatus,
    SHAREPOINT_GRAPH_API_DOC_URL, SHAREPOINT_GRAPH_API_VERSION_PIN, SHAREPOINT_VENDOR_KEY,
};
pub use provider::SharePointProvider;
pub use registry::{
    RegisteredSharePointOperation, SharePointNamedOperationRequest, SharePointOperationGate,
    SharePointOperationKind, SharePointOperationRegistry, SharePointRequestPlan,
    PARAM_CONTENT_LENGTH, PARAM_CONTENT_TYPE, PARAM_DRIVE_ID, PARAM_FILE_PATH, PARAM_FOLDER_PATH,
    PARAM_ITEM_ID, SHAREPOINT_GET_DRIVE_ITEM_OPERATION, SHAREPOINT_LIST_DRIVE_ITEMS_OPERATION,
    SHAREPOINT_LIST_DRIVE_ITEM_VERSIONS_OPERATION, SHAREPOINT_MAX_UPLOAD_BYTES,
    SHAREPOINT_UPLOAD_FILE_OPERATION, SHAREPOINT_UPLOAD_FILE_POLICY_SCOPE,
};
