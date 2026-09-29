use crate::{
    AnaplanDataSensitivity, AnaplanOperationAvailability, AnaplanOperationGate,
    AnaplanOperationKind, AnaplanOperationPrimitive, ANAPLAN_API_BASE_PATH,
    ANAPLAN_FILES_LIST_OPERATION, ANAPLAN_FILE_DOWNLOAD_CHUNK_OPERATION,
    ANAPLAN_MODEL_GET_STATUS_OPERATION, ANAPLAN_VIEW_CREATE_READ_REQUEST_OPERATION,
    ANAPLAN_VIEW_GET_READ_PAGE_OPERATION, ANAPLAN_VIEW_GET_READ_REQUEST_OPERATION,
};

pub const ANAPLAN_VENDOR_KEY: &str = "anaplan-integration-api";
pub const ANAPLAN_API_SPEC_PACKAGE_ID: &str = "anaplan-integration-api-v2-specs";
pub const ANAPLAN_INFORMATION_PACKAGE_VERSION: &str = "2026.06.26.1";
pub const ANAPLAN_RELEASE_PIN: &str = "tenant-scoped workspace/model export";
pub const ANAPLAN_API_VERSION_PIN: &str = "Integration API v2";
pub const ANAPLAN_INFORMATION_PACKAGE_VERSIONS_PATH: &str =
    "vendor_api_specs_2026-06-26/enrichment/information-package-versions.json";
pub const ANAPLAN_IMPLEMENTATION_METADATA_PATH: &str =
    "vendor_api_specs_2026-06-26/enrichment/saas-provider-implementation-metadata.json";
pub const ANAPLAN_INTEGRATION_API_SUMMARY_PATH: &str =
    "vendor_api_specs_2026-06-26/enrichment/extracted/anaplan-integration-api-v2-summary.json";

const SOURCE_REFS: &[&str] = &[
    ANAPLAN_INFORMATION_PACKAGE_VERSIONS_PATH,
    ANAPLAN_IMPLEMENTATION_METADATA_PATH,
    ANAPLAN_INTEGRATION_API_SUMMARY_PATH,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnaplanApiSnapshotMetadata {
    pub vendor_key: &'static str,
    pub package_id: &'static str,
    pub information_package_version: &'static str,
    pub release: &'static str,
    pub api_version: &'static str,
    pub api_base_path: &'static str,
    pub source_refs: &'static [&'static str],
    pub blocking_gap: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnaplanProjectionStatus {
    Executable,
    EvidenceGated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnaplanMetadataDomain {
    Model,
    File,
    ViewExport,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnaplanOperationMetadata {
    pub name: &'static str,
    pub domain: AnaplanMetadataDomain,
    pub kind: AnaplanOperationKind,
    pub primitive: AnaplanOperationPrimitive,
    pub projection_status: AnaplanProjectionStatus,
    pub availability: AnaplanOperationAvailability,
    pub sensitivity: AnaplanDataSensitivity,
    pub gates: &'static [AnaplanOperationGate],
    pub source_refs: &'static [&'static str],
    pub notes: &'static str,
}

pub fn anaplan_api_snapshot_metadata() -> AnaplanApiSnapshotMetadata {
    AnaplanApiSnapshotMetadata {
        vendor_key: ANAPLAN_VENDOR_KEY,
        package_id: ANAPLAN_API_SPEC_PACKAGE_ID,
        information_package_version: ANAPLAN_INFORMATION_PACKAGE_VERSION,
        release: ANAPLAN_RELEASE_PIN,
        api_version: ANAPLAN_API_VERSION_PIN,
        api_base_path: ANAPLAN_API_BASE_PATH,
        source_refs: SOURCE_REFS,
        blocking_gap:
            "Need tenant-approved Anaplan workspace/model binding plus export/action allow-list evidence.",
    }
}

pub fn anaplan_operation_metadata_catalog() -> &'static [AnaplanOperationMetadata] {
    OPERATION_METADATA
}

pub fn anaplan_operation_metadata(name: &str) -> Option<&'static AnaplanOperationMetadata> {
    OPERATION_METADATA
        .iter()
        .find(|metadata| metadata.name == name)
}

const OPERATION_METADATA: &[AnaplanOperationMetadata] = &[
    AnaplanOperationMetadata {
        name: ANAPLAN_MODEL_GET_STATUS_OPERATION,
        domain: AnaplanMetadataDomain::Model,
        kind: AnaplanOperationKind::ModelGetStatus,
        primitive: AnaplanOperationPrimitive::GetModelStatus,
        projection_status: AnaplanProjectionStatus::Executable,
        availability: AnaplanOperationAvailability::Executable,
        sensitivity: AnaplanDataSensitivity::Operational,
        gates: &[],
        source_refs: SOURCE_REFS,
        notes: "Fixed server-bound model-status read.",
    },
    AnaplanOperationMetadata {
        name: ANAPLAN_FILES_LIST_OPERATION,
        domain: AnaplanMetadataDomain::File,
        kind: AnaplanOperationKind::FilesList,
        primitive: AnaplanOperationPrimitive::ListFiles,
        projection_status: AnaplanProjectionStatus::Executable,
        availability: AnaplanOperationAvailability::Executable,
        sensitivity: AnaplanDataSensitivity::BusinessConfidential,
        gates: &[],
        source_refs: SOURCE_REFS,
        notes: "Server-bound files list with fixed sort, offset, and limit parameters.",
    },
    AnaplanOperationMetadata {
        name: ANAPLAN_VIEW_CREATE_READ_REQUEST_OPERATION,
        domain: AnaplanMetadataDomain::ViewExport,
        kind: AnaplanOperationKind::ViewCreateReadRequest,
        primitive: AnaplanOperationPrimitive::CreateViewReadRequest,
        projection_status: AnaplanProjectionStatus::EvidenceGated,
        availability: AnaplanOperationAvailability::PlannedUnsupported,
        sensitivity: AnaplanDataSensitivity::BusinessConfidential,
        gates: &[
            AnaplanOperationGate::TenantExportAllowList,
            AnaplanOperationGate::LiveCertification,
        ],
        source_refs: SOURCE_REFS,
        notes: "View read-request creation waits for a tenant-approved view allow-list.",
    },
    AnaplanOperationMetadata {
        name: ANAPLAN_VIEW_GET_READ_REQUEST_OPERATION,
        domain: AnaplanMetadataDomain::ViewExport,
        kind: AnaplanOperationKind::ViewGetReadRequest,
        primitive: AnaplanOperationPrimitive::GetViewReadRequest,
        projection_status: AnaplanProjectionStatus::EvidenceGated,
        availability: AnaplanOperationAvailability::PlannedUnsupported,
        sensitivity: AnaplanDataSensitivity::BusinessConfidential,
        gates: &[
            AnaplanOperationGate::TenantExportAllowList,
            AnaplanOperationGate::LiveCertification,
        ],
        source_refs: SOURCE_REFS,
        notes: "Read-request status waits for a tenant-approved view allow-list.",
    },
    AnaplanOperationMetadata {
        name: ANAPLAN_VIEW_GET_READ_PAGE_OPERATION,
        domain: AnaplanMetadataDomain::ViewExport,
        kind: AnaplanOperationKind::ViewGetReadPage,
        primitive: AnaplanOperationPrimitive::GetViewReadPage,
        projection_status: AnaplanProjectionStatus::EvidenceGated,
        availability: AnaplanOperationAvailability::PlannedUnsupported,
        sensitivity: AnaplanDataSensitivity::BusinessConfidential,
        gates: &[
            AnaplanOperationGate::TenantExportAllowList,
            AnaplanOperationGate::LiveCertification,
        ],
        source_refs: SOURCE_REFS,
        notes: "Read-request page reads wait for a tenant-approved view allow-list.",
    },
    AnaplanOperationMetadata {
        name: ANAPLAN_FILE_DOWNLOAD_CHUNK_OPERATION,
        domain: AnaplanMetadataDomain::File,
        kind: AnaplanOperationKind::FileDownloadChunk,
        primitive: AnaplanOperationPrimitive::DownloadFileChunk,
        projection_status: AnaplanProjectionStatus::EvidenceGated,
        availability: AnaplanOperationAvailability::PlannedUnsupported,
        sensitivity: AnaplanDataSensitivity::BusinessConfidential,
        gates: &[
            AnaplanOperationGate::TenantExportAllowList,
            AnaplanOperationGate::LiveCertification,
        ],
        source_refs: SOURCE_REFS,
        notes: "File chunk downloads wait for a tenant-approved file allow-list.",
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AnaplanOperationRegistry;

    #[test]
    fn api_snapshot_metadata_records_anaplan_pin_and_gap() {
        let metadata = anaplan_api_snapshot_metadata();

        assert_eq!(metadata.vendor_key, "anaplan-integration-api");
        assert_eq!(metadata.api_base_path, ANAPLAN_API_BASE_PATH);
        assert!(metadata.blocking_gap.contains("workspace/model binding"));
        assert!(metadata
            .source_refs
            .contains(&ANAPLAN_INTEGRATION_API_SUMMARY_PATH));
    }

    #[test]
    fn metadata_operation_names_match_registry_constants() {
        let registry = AnaplanOperationRegistry::default_read_operations();
        let registry_names = registry.operation_names().collect::<Vec<_>>();
        let mut metadata_names = anaplan_operation_metadata_catalog()
            .iter()
            .map(|metadata| metadata.name)
            .collect::<Vec<_>>();
        metadata_names.sort_unstable();

        assert_eq!(metadata_names, registry_names);
    }
}
