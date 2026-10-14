use crate::{
    SharePointOperationGate, SharePointOperationKind, SHAREPOINT_API_FAMILY,
    SHAREPOINT_GET_DRIVE_ITEM_OPERATION, SHAREPOINT_LIST_DRIVE_ITEMS_OPERATION,
    SHAREPOINT_LIST_DRIVE_ITEM_VERSIONS_OPERATION, SHAREPOINT_RELEASE_BASIS,
    SHAREPOINT_UPLOAD_FILE_OPERATION,
};

pub const SHAREPOINT_VENDOR_KEY: &str = "microsoft-sharepoint-graph";
pub const SHAREPOINT_GRAPH_API_VERSION_PIN: &str = "v1.0";
pub const SHAREPOINT_GRAPH_API_DOC_URL: &str =
    "https://learn.microsoft.com/en-us/graph/api/resources/driveitem";

const SOURCE_REFS: &[&str] = &[
    SHAREPOINT_GRAPH_API_DOC_URL,
    "tax-doc-routing/docs/architecture/sharepoint-integration-plan.md",
    "tax-doc-routing/docs/architecture/sharepoint-graph-admin-handoff.md",
];
const READ_GATES: &[SharePointOperationGate] = &[
    SharePointOperationGate::FrameworkProviderIdentityEvidence,
    SharePointOperationGate::TenantSiteGrantEvidence,
    SharePointOperationGate::LiveCertification,
];
const GOVERNED_WRITE_GATES: &[SharePointOperationGate] = &[
    SharePointOperationGate::FrameworkProviderIdentityEvidence,
    SharePointOperationGate::TenantSiteGrantEvidence,
    SharePointOperationGate::DelegatedActorContextEvidence,
    SharePointOperationGate::TokenStoreIsolationEvidence,
    SharePointOperationGate::NamedMutationRegistryEvidence,
    SharePointOperationGate::MutationRequestBindingEvidence,
    SharePointOperationGate::IdempotencyReplayEvidence,
    SharePointOperationGate::WritePolicyScopeEvidence,
    SharePointOperationGate::WriteAuditEvidence,
    SharePointOperationGate::LiveCertification,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SharePointApiSnapshotMetadata {
    pub vendor_key: &'static str,
    pub api_version_pin: &'static str,
    pub api_family: &'static str,
    pub release_basis: &'static str,
    pub source_refs: &'static [&'static str],
    pub blocking_gap: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharePointProjectionStatus {
    EvidenceGated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharePointMetadataDomain {
    DriveRead,
    DriveWrite,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SharePointOperationMetadata {
    pub name: &'static str,
    pub domain: SharePointMetadataDomain,
    pub kind: SharePointOperationKind,
    pub projection_status: SharePointProjectionStatus,
    pub gates: &'static [SharePointOperationGate],
    pub source_refs: &'static [&'static str],
    pub notes: &'static str,
}

pub fn sharepoint_api_snapshot_metadata() -> SharePointApiSnapshotMetadata {
    SharePointApiSnapshotMetadata {
        vendor_key: SHAREPOINT_VENDOR_KEY,
        api_version_pin: SHAREPOINT_GRAPH_API_VERSION_PIN,
        api_family: SHAREPOINT_API_FAMILY,
        release_basis: SHAREPOINT_RELEASE_BASIS,
        source_refs: SOURCE_REFS,
        blocking_gap:
            "Need a FrameworkProvider::SharePoint identity + DataSourceType entry in app_gen, \
             plus a real tenant site/drive/role grant, before any operation here can execute.",
    }
}

pub fn sharepoint_operation_metadata_catalog() -> &'static [SharePointOperationMetadata] {
    OPERATION_METADATA
}

pub fn sharepoint_operation_metadata(name: &str) -> Option<&'static SharePointOperationMetadata> {
    OPERATION_METADATA.iter().find(|metadata| metadata.name == name)
}

const OPERATION_METADATA: &[SharePointOperationMetadata] = &[
    SharePointOperationMetadata {
        name: SHAREPOINT_LIST_DRIVE_ITEMS_OPERATION,
        domain: SharePointMetadataDomain::DriveRead,
        kind: SharePointOperationKind::ListDriveItems,
        projection_status: SharePointProjectionStatus::EvidenceGated,
        gates: READ_GATES,
        source_refs: SOURCE_REFS,
        notes: "Lists files/folders one level deep in a tax-entity folder.",
    },
    SharePointOperationMetadata {
        name: SHAREPOINT_GET_DRIVE_ITEM_OPERATION,
        domain: SharePointMetadataDomain::DriveRead,
        kind: SharePointOperationKind::GetDriveItem,
        projection_status: SharePointProjectionStatus::EvidenceGated,
        gates: READ_GATES,
        source_refs: SOURCE_REFS,
        notes: "File metadata, including the short-lived pre-authenticated download URL.",
    },
    SharePointOperationMetadata {
        name: SHAREPOINT_LIST_DRIVE_ITEM_VERSIONS_OPERATION,
        domain: SharePointMetadataDomain::DriveRead,
        kind: SharePointOperationKind::ListDriveItemVersions,
        projection_status: SharePointProjectionStatus::EvidenceGated,
        gates: READ_GATES,
        source_refs: SOURCE_REFS,
        notes: "Prior version history SharePoint keeps for a replaced file.",
    },
    SharePointOperationMetadata {
        name: SHAREPOINT_UPLOAD_FILE_OPERATION,
        domain: SharePointMetadataDomain::DriveWrite,
        kind: SharePointOperationKind::UploadFile,
        projection_status: SharePointProjectionStatus::EvidenceGated,
        gates: GOVERNED_WRITE_GATES,
        source_refs: SOURCE_REFS,
        notes: "Only named governed-write; policy scope sharepoint.drive.write; 2 MB per file. \
                Non-executable until live G1 evidence exists, on top of the shared \
                framework-wiring gap.",
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SharePointOperationRegistry;

    #[test]
    fn api_snapshot_metadata_records_the_framework_wiring_gap() {
        let metadata = sharepoint_api_snapshot_metadata();
        assert_eq!(metadata.vendor_key, SHAREPOINT_VENDOR_KEY);
        assert!(metadata.blocking_gap.contains("FrameworkProvider::SharePoint"));
        assert!(metadata.source_refs.contains(&SHAREPOINT_GRAPH_API_DOC_URL));
    }

    #[test]
    fn metadata_operation_names_match_registry_constants() {
        let registry = SharePointOperationRegistry::default();
        let registry_names = registry.operation_names().collect::<Vec<_>>();
        let mut metadata_names = sharepoint_operation_metadata_catalog()
            .iter()
            .map(|metadata| metadata.name)
            .collect::<Vec<_>>();
        metadata_names.sort_unstable();
        assert_eq!(metadata_names, registry_names);
    }

    #[test]
    fn metadata_gates_match_registry_gates() {
        let registry = SharePointOperationRegistry::default();
        for metadata in sharepoint_operation_metadata_catalog() {
            let registered = registry.get(metadata.name).expect("registered");
            assert_eq!(metadata.gates, registered.gates, "{}", metadata.name);
        }
    }
}
