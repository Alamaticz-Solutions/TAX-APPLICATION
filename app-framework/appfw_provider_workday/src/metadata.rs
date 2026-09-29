use crate::{WorkdayOperationGate, WorkdayOperationKind, WorkdayWwsOperation};

use crate::registry::{
    WORKDAY_GET_SERVER_TIMESTAMP_OPERATION, WORKDAY_GET_WORKDAY_ACCOUNTS_OPERATION,
    WORKDAY_GET_WORKER_OPERATION, WORKDAY_HCM_GET_JOB_PROFILES_PAGE_OPERATION,
    WORKDAY_HCM_GET_LOCATIONS_PAGE_OPERATION, WORKDAY_HCM_GET_ORGANIZATIONS_PAGE_OPERATION,
    WORKDAY_HCM_GET_WORKERS_PAGE_OPERATION, WORKDAY_HCM_GET_WORKER_PROFILE_OPERATION,
    WORKDAY_LIST_FORMER_WORKERS_OPERATION, WORKDAY_LIST_JOB_PROFILES_OPERATION,
    WORKDAY_LIST_LOCATIONS_OPERATION, WORKDAY_LIST_ORGANIZATIONS_OPERATION,
    WORKDAY_LIST_WORKERS_OPERATION, WORKDAY_LIST_WORKER_EVENTS_OPERATION,
};
use crate::soap::WORKDAY_WWS_VERSION;

pub const WORKDAY_VENDOR_KEY: &str = "workday-hcm-wws";
pub const WORKDAY_API_SPEC_PACKAGE_ID: &str = "workday-human-resources-wws-api-specs";
pub const WORKDAY_INFORMATION_PACKAGE_VERSION: &str = "2026.06.26.1";
pub const WORKDAY_RELEASE_PIN: &str = "2026R1";
pub const WORKDAY_API_VERSION_PIN: &str = "WWS v46.1";
pub const WORKDAY_SERVICE_NAME_PIN: &str = "Human_ResourcesService";
pub const WORKDAY_INFORMATION_PACKAGE_VERSIONS_PATH: &str =
    "vendor_api_specs_2026-06-26/enrichment/information-package-versions.json";
pub const WORKDAY_IMPLEMENTATION_METADATA_PATH: &str =
    "vendor_api_specs_2026-06-26/enrichment/saas-provider-implementation-metadata.json";
pub const WORKDAY_WWS_SUMMARY_PATH: &str =
    "vendor_api_specs_2026-06-26/enrichment/extracted/workday-human-resources-wws-v46.1-summary.json";

const SOURCE_REFS: &[&str] = &[
    WORKDAY_INFORMATION_PACKAGE_VERSIONS_PATH,
    WORKDAY_IMPLEMENTATION_METADATA_PATH,
    WORKDAY_WWS_SUMMARY_PATH,
];
const NO_GATES: &[WorkdayOperationGate] = &[];
const FORMER_WORKER_GATES: &[WorkdayOperationGate] = &[
    WorkdayOperationGate::TenantApprovedWwsRequestShape,
    WorkdayOperationGate::HrPiiGovernanceReview,
    WorkdayOperationGate::LiveCertification,
];
const ACCOUNT_GATES: &[WorkdayOperationGate] = &[
    WorkdayOperationGate::TenantApprovedWwsRequestShape,
    WorkdayOperationGate::IdentitySecurityGovernanceReview,
    WorkdayOperationGate::LiveCertification,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkdayApiSnapshotMetadata {
    pub vendor_key: &'static str,
    pub package_id: &'static str,
    pub information_package_version: &'static str,
    pub release: &'static str,
    pub api_version: &'static str,
    pub service_name: &'static str,
    pub wws_version: &'static str,
    pub source_refs: &'static [&'static str],
    pub blocking_gap: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkdayProjectionStatus {
    Executable,
    EvidenceGated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkdayMetadataDomain {
    Worker,
    WorkerEvent,
    ReferenceData,
    Identity,
    Freshness,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkdayOperationMetadata {
    pub name: &'static str,
    pub domain: WorkdayMetadataDomain,
    pub kind: WorkdayOperationKind,
    pub wws_operation: WorkdayWwsOperation,
    pub projection_status: WorkdayProjectionStatus,
    pub pii_heavy: bool,
    pub gates: &'static [WorkdayOperationGate],
    pub source_refs: &'static [&'static str],
    pub notes: &'static str,
}

pub fn workday_api_snapshot_metadata() -> WorkdayApiSnapshotMetadata {
    WorkdayApiSnapshotMetadata {
        vendor_key: WORKDAY_VENDOR_KEY,
        package_id: WORKDAY_API_SPEC_PACKAGE_ID,
        information_package_version: WORKDAY_INFORMATION_PACKAGE_VERSION,
        release: WORKDAY_RELEASE_PIN,
        api_version: WORKDAY_API_VERSION_PIN,
        service_name: WORKDAY_SERVICE_NAME_PIN,
        wws_version: WORKDAY_WWS_VERSION,
        source_refs: SOURCE_REFS,
        blocking_gap:
            "Need tenant-approved Workday WWS request-shape evidence and a dated Human_Resources v46.1 documentation/XSD snapshot.",
    }
}

pub fn workday_operation_metadata_catalog() -> &'static [WorkdayOperationMetadata] {
    OPERATION_METADATA
}

pub fn workday_operation_metadata(name: &str) -> Option<&'static WorkdayOperationMetadata> {
    OPERATION_METADATA
        .iter()
        .find(|metadata| metadata.name == name)
}

const OPERATION_METADATA: &[WorkdayOperationMetadata] = &[
    WorkdayOperationMetadata {
        name: WORKDAY_LIST_WORKERS_OPERATION,
        domain: WorkdayMetadataDomain::Worker,
        kind: WorkdayOperationKind::ListWorkers,
        wws_operation: WorkdayWwsOperation::GetWorkers,
        projection_status: WorkdayProjectionStatus::Executable,
        pii_heavy: true,
        gates: NO_GATES,
        source_refs: SOURCE_REFS,
        notes: "Get_Workers page read with paired transaction-log and effective-date criteria.",
    },
    WorkdayOperationMetadata {
        name: WORKDAY_GET_WORKER_OPERATION,
        domain: WorkdayMetadataDomain::Worker,
        kind: WorkdayOperationKind::GetWorker,
        wws_operation: WorkdayWwsOperation::GetWorkers,
        projection_status: WorkdayProjectionStatus::Executable,
        pii_heavy: true,
        gates: NO_GATES,
        source_refs: SOURCE_REFS,
        notes: "Get_Workers by typed Workday worker reference.",
    },
    WorkdayOperationMetadata {
        name: WORKDAY_HCM_GET_WORKER_PROFILE_OPERATION,
        domain: WorkdayMetadataDomain::Worker,
        kind: WorkdayOperationKind::GetWorkerProfileViaGetWorkers,
        wws_operation: WorkdayWwsOperation::GetWorkers,
        projection_status: WorkdayProjectionStatus::Executable,
        pii_heavy: true,
        gates: NO_GATES,
        source_refs: SOURCE_REFS,
        notes: "Metadata alias bound through the proven Get_Workers by-reference request.",
    },
    WorkdayOperationMetadata {
        name: WORKDAY_LIST_WORKER_EVENTS_OPERATION,
        domain: WorkdayMetadataDomain::WorkerEvent,
        kind: WorkdayOperationKind::ListWorkerEvents,
        wws_operation: WorkdayWwsOperation::GetWorkerEventHistory,
        projection_status: WorkdayProjectionStatus::Executable,
        pii_heavy: true,
        gates: NO_GATES,
        source_refs: SOURCE_REFS,
        notes: "Get_Worker_Event_History by worker reference and event date range.",
    },
    WorkdayOperationMetadata {
        name: WORKDAY_LIST_ORGANIZATIONS_OPERATION,
        domain: WorkdayMetadataDomain::ReferenceData,
        kind: WorkdayOperationKind::ListOrganizations,
        wws_operation: WorkdayWwsOperation::GetOrganizations,
        projection_status: WorkdayProjectionStatus::Executable,
        pii_heavy: false,
        gates: NO_GATES,
        source_refs: SOURCE_REFS,
        notes: "Get_Organizations reference-data read.",
    },
    WorkdayOperationMetadata {
        name: WORKDAY_LIST_LOCATIONS_OPERATION,
        domain: WorkdayMetadataDomain::ReferenceData,
        kind: WorkdayOperationKind::ListLocations,
        wws_operation: WorkdayWwsOperation::GetLocations,
        projection_status: WorkdayProjectionStatus::Executable,
        pii_heavy: false,
        gates: NO_GATES,
        source_refs: SOURCE_REFS,
        notes: "Get_Locations reference-data read.",
    },
    WorkdayOperationMetadata {
        name: WORKDAY_LIST_JOB_PROFILES_OPERATION,
        domain: WorkdayMetadataDomain::ReferenceData,
        kind: WorkdayOperationKind::ListJobProfiles,
        wws_operation: WorkdayWwsOperation::GetJobProfiles,
        projection_status: WorkdayProjectionStatus::Executable,
        pii_heavy: false,
        gates: NO_GATES,
        source_refs: SOURCE_REFS,
        notes: "Get_Job_Profiles reference-data read.",
    },
    WorkdayOperationMetadata {
        name: WORKDAY_GET_SERVER_TIMESTAMP_OPERATION,
        domain: WorkdayMetadataDomain::Freshness,
        kind: WorkdayOperationKind::GetServerTimestamp,
        wws_operation: WorkdayWwsOperation::GetServerTimestamp,
        projection_status: WorkdayProjectionStatus::Executable,
        pii_heavy: false,
        gates: NO_GATES,
        source_refs: SOURCE_REFS,
        notes: "Get_Server_Timestamp freshness anchor.",
    },
    WorkdayOperationMetadata {
        name: WORKDAY_HCM_GET_WORKERS_PAGE_OPERATION,
        domain: WorkdayMetadataDomain::Worker,
        kind: WorkdayOperationKind::ListWorkers,
        wws_operation: WorkdayWwsOperation::GetWorkers,
        projection_status: WorkdayProjectionStatus::Executable,
        pii_heavy: true,
        gates: NO_GATES,
        source_refs: SOURCE_REFS,
        notes: "Stable HCM alias for list-workers paging.",
    },
    WorkdayOperationMetadata {
        name: WORKDAY_HCM_GET_ORGANIZATIONS_PAGE_OPERATION,
        domain: WorkdayMetadataDomain::ReferenceData,
        kind: WorkdayOperationKind::ListOrganizations,
        wws_operation: WorkdayWwsOperation::GetOrganizations,
        projection_status: WorkdayProjectionStatus::Executable,
        pii_heavy: false,
        gates: NO_GATES,
        source_refs: SOURCE_REFS,
        notes: "Stable HCM alias for organization paging.",
    },
    WorkdayOperationMetadata {
        name: WORKDAY_HCM_GET_LOCATIONS_PAGE_OPERATION,
        domain: WorkdayMetadataDomain::ReferenceData,
        kind: WorkdayOperationKind::ListLocations,
        wws_operation: WorkdayWwsOperation::GetLocations,
        projection_status: WorkdayProjectionStatus::Executable,
        pii_heavy: false,
        gates: NO_GATES,
        source_refs: SOURCE_REFS,
        notes: "Stable HCM alias for location paging.",
    },
    WorkdayOperationMetadata {
        name: WORKDAY_HCM_GET_JOB_PROFILES_PAGE_OPERATION,
        domain: WorkdayMetadataDomain::ReferenceData,
        kind: WorkdayOperationKind::ListJobProfiles,
        wws_operation: WorkdayWwsOperation::GetJobProfiles,
        projection_status: WorkdayProjectionStatus::Executable,
        pii_heavy: false,
        gates: NO_GATES,
        source_refs: SOURCE_REFS,
        notes: "Stable HCM alias for job-profile paging.",
    },
    WorkdayOperationMetadata {
        name: WORKDAY_LIST_FORMER_WORKERS_OPERATION,
        domain: WorkdayMetadataDomain::Worker,
        kind: WorkdayOperationKind::PlannedUnsupported,
        wws_operation: WorkdayWwsOperation::GetFormerWorkers,
        projection_status: WorkdayProjectionStatus::EvidenceGated,
        pii_heavy: true,
        gates: FORMER_WORKER_GATES,
        source_refs: SOURCE_REFS,
        notes: "Former-worker reads need tenant-approved WWS request-shape and HR PII governance evidence.",
    },
    WorkdayOperationMetadata {
        name: WORKDAY_GET_WORKDAY_ACCOUNTS_OPERATION,
        domain: WorkdayMetadataDomain::Identity,
        kind: WorkdayOperationKind::PlannedUnsupported,
        wws_operation: WorkdayWwsOperation::GetWorkdayAccount,
        projection_status: WorkdayProjectionStatus::EvidenceGated,
        pii_heavy: true,
        gates: ACCOUNT_GATES,
        source_refs: SOURCE_REFS,
        notes: "Workday account reads need tenant-approved request-shape plus identity/security governance evidence.",
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::WorkdayOperationRegistry;

    #[test]
    fn api_snapshot_metadata_records_workday_pin_and_gap() {
        let metadata = workday_api_snapshot_metadata();

        assert_eq!(metadata.vendor_key, "workday-hcm-wws");
        assert_eq!(metadata.release, "2026R1");
        assert_eq!(metadata.wws_version, WORKDAY_WWS_VERSION);
        assert!(metadata.blocking_gap.contains("tenant-approved Workday"));
        assert!(metadata.source_refs.contains(&WORKDAY_WWS_SUMMARY_PATH));
    }

    #[test]
    fn metadata_operation_names_match_registry_constants() {
        let registry = WorkdayOperationRegistry::default_read_operations();
        let registry_names = registry.operation_names().collect::<Vec<_>>();
        let mut metadata_names = workday_operation_metadata_catalog()
            .iter()
            .map(|metadata| metadata.name)
            .collect::<Vec<_>>();
        metadata_names.sort_unstable();

        assert_eq!(metadata_names, registry_names);
    }
}
