//! Workday Human Resources WWS external API provider skeleton.
//!
//! The crate is intentionally network-free for the SaaS implementation wave. It
//! models provider identity, read-only named operations, typed Workday SOAP
//! request templates, response pagination metadata, and SOAP fault
//! classification. Runtime trait wiring should replace the local placeholder
//! identity/operation shims once the shared external API contracts are ready.
//! The offline metadata pin is Workday WWS v46.1 / 2026R1; tenant-approved WWS
//! auth, such as an ISU with WS-Security UsernameToken where approved, belongs
//! in runtime integration wiring rather than these network-free templates.

pub mod auth;
pub mod error;
pub mod fault;
pub mod identity;
pub mod metadata;
pub mod operation;
pub mod pagination;
pub mod provider;
pub mod reference;
pub mod registry;
pub mod response;
pub mod soap;

mod xml;

pub use auth::{
    WorkdayTenantBinding, WORKDAY_HOST_ENV, WORKDAY_ISU_SECRET_ENV, WORKDAY_ISU_USERNAME_ENV,
    WORKDAY_TENANT_ENV,
};
pub use error::WorkdayProviderError;
pub use fault::{WorkdayFaultClassification, WorkdayFaultKind, WorkdayFaultRetry};
pub use identity::{
    WorkdayFrameworkProvider, WorkdayProviderDescriptor, WorkdayProviderIdentity,
    WORKDAY_PROVIDER_KEY,
};
pub use metadata::{
    workday_api_snapshot_metadata, workday_operation_metadata, workday_operation_metadata_catalog,
    WorkdayApiSnapshotMetadata, WorkdayMetadataDomain, WorkdayOperationMetadata,
    WorkdayProjectionStatus, WORKDAY_API_SPEC_PACKAGE_ID, WORKDAY_API_VERSION_PIN,
    WORKDAY_IMPLEMENTATION_METADATA_PATH, WORKDAY_INFORMATION_PACKAGE_VERSION,
    WORKDAY_INFORMATION_PACKAGE_VERSIONS_PATH, WORKDAY_RELEASE_PIN, WORKDAY_SERVICE_NAME_PIN,
    WORKDAY_VENDOR_KEY, WORKDAY_WWS_SUMMARY_PATH,
};
pub use operation::{
    WorkdayNamedOperationRequest, WorkdayOperationAvailability, WorkdayOperationGate,
    WorkdayOperationParameter, WorkdayParameterValue, WorkdayQueryLimits, WorkdayRequestPlan,
    WorkdayResponseFilter, WorkdayWorkerTransactionLogCriteria,
};
pub use pagination::{
    next_workday_page_count_continuation, next_workday_pagination_continuation,
    response_filter_from_page_count_continuation, WorkdayPageCountContinuationResult,
    WorkdayPaginationContinuationResult, WorkdayResponseFilterContinuationResult,
};
pub use provider::WorkdayProvider;
pub use reference::{WorkdayObjectReference, WorkdayReferenceType};
pub use registry::{
    RegisteredWorkdayOperation, WorkdayOperationKind, WorkdayOperationRegistry,
    WORKDAY_GET_SERVER_TIMESTAMP_OPERATION, WORKDAY_GET_WORKDAY_ACCOUNTS_OPERATION,
    WORKDAY_GET_WORKER_OPERATION, WORKDAY_HCM_GET_JOB_PROFILES_PAGE_OPERATION,
    WORKDAY_HCM_GET_LOCATIONS_PAGE_OPERATION, WORKDAY_HCM_GET_ORGANIZATIONS_PAGE_OPERATION,
    WORKDAY_HCM_GET_WORKERS_PAGE_OPERATION, WORKDAY_HCM_GET_WORKER_PROFILE_OPERATION,
    WORKDAY_LIST_FORMER_WORKERS_OPERATION, WORKDAY_LIST_JOB_PROFILES_OPERATION,
    WORKDAY_LIST_LOCATIONS_OPERATION, WORKDAY_LIST_ORGANIZATIONS_OPERATION,
    WORKDAY_LIST_WORKERS_OPERATION, WORKDAY_LIST_WORKER_EVENTS_OPERATION,
};
pub use response::{WorkdayFreshnessSummary, WorkdayResponsePageSummary, WorkdayServerTimestamp};
pub use soap::{
    GetJobProfilesRequestTemplate, GetLocationsRequestTemplate, GetOrganizationsRequestTemplate,
    GetServerTimestampRequestTemplate, GetWorkerEventHistoryRequestTemplate,
    GetWorkersRequestTemplate, WorkdayEventDateRange, WorkdayRequestTemplate,
    WorkdaySoapRequestTemplate, WorkdayWwsOperation,
};
