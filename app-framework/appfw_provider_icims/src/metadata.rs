use crate::{
    IcimsOperationGate, IcimsOperationKind, ICIMS_API_FAMILY,
    ICIMS_DISCOVERY_FETCH_API_CONTRACT_OPERATION, ICIMS_PROVIDER_KEY,
    ICIMS_RECRUITING_QUERY_APPLICATIONS_INCREMENTAL_OPERATION,
    ICIMS_RECRUITING_QUERY_CANDIDATES_INCREMENTAL_OPERATION,
    ICIMS_RECRUITING_QUERY_JOBS_INCREMENTAL_OPERATION, ICIMS_RELEASE_BASIS,
};

pub const ICIMS_VENDOR_KEY: &str = "icims";
pub const ICIMS_API_SPEC_PACKAGE_ID: &str = "icims-public-api-docs";
pub const ICIMS_INFORMATION_PACKAGE_VERSION: &str = "2026.06.26.1";
pub const ICIMS_INFORMATION_PACKAGE_VERSIONS_PATH: &str =
    "vendor_api_specs_2026-06-26/enrichment/information-package-versions.json";
pub const ICIMS_IMPLEMENTATION_METADATA_PATH: &str =
    "vendor_api_specs_2026-06-26/enrichment/saas-provider-implementation-metadata.json";
pub const ICIMS_PUBLIC_DOCS_SUMMARY_PATH: &str =
    "vendor_api_specs_2026-06-26/enrichment/extracted/icims-public-docs-summary.json";
pub const ICIMS_INTEGRATING_WITH_ICIMS_URL: &str =
    "https://developer-community.icims.com/getting-started/integrating-icims";

const SOURCE_REFS: &[&str] = &[
    ICIMS_INFORMATION_PACKAGE_VERSIONS_PATH,
    ICIMS_IMPLEMENTATION_METADATA_PATH,
    ICIMS_PUBLIC_DOCS_SUMMARY_PATH,
    ICIMS_INTEGRATING_WITH_ICIMS_URL,
];
const DEVELOPER_DOC_GATES: &[IcimsOperationGate] = &[
    IcimsOperationGate::AuthenticatedDeveloperDocs,
    IcimsOperationGate::SandboxOrTenantExport,
    IcimsOperationGate::StandardFieldMatrix,
    IcimsOperationGate::LiveCertification,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IcimsApiSnapshotMetadata {
    pub vendor_key: &'static str,
    pub provider_key: &'static str,
    pub package_id: &'static str,
    pub information_package_version: &'static str,
    pub api_family: &'static str,
    pub release_basis: &'static str,
    pub source_refs: &'static [&'static str],
    pub blocking_gap: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IcimsProjectionStatus {
    EvidenceGated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IcimsMetadataDomain {
    Discovery,
    Recruiting,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IcimsOperationMetadata {
    pub name: &'static str,
    pub domain: IcimsMetadataDomain,
    pub kind: IcimsOperationKind,
    pub projection_status: IcimsProjectionStatus,
    pub gates: &'static [IcimsOperationGate],
    pub source_refs: &'static [&'static str],
    pub notes: &'static str,
}

pub fn icims_api_snapshot_metadata() -> IcimsApiSnapshotMetadata {
    IcimsApiSnapshotMetadata {
        vendor_key: ICIMS_VENDOR_KEY,
        provider_key: ICIMS_PROVIDER_KEY,
        package_id: ICIMS_API_SPEC_PACKAGE_ID,
        information_package_version: ICIMS_INFORMATION_PACKAGE_VERSION,
        api_family: ICIMS_API_FAMILY,
        release_basis: ICIMS_RELEASE_BASIS,
        source_refs: SOURCE_REFS,
        blocking_gap:
            "Need authenticated iCIMS Developer Community API docs plus sandbox or tenant export evidence.",
    }
}

pub fn icims_operation_metadata_catalog() -> &'static [IcimsOperationMetadata] {
    OPERATION_METADATA
}

pub fn icims_operation_metadata(name: &str) -> Option<&'static IcimsOperationMetadata> {
    OPERATION_METADATA
        .iter()
        .find(|metadata| metadata.name == name)
}

const OPERATION_METADATA: &[IcimsOperationMetadata] = &[
    IcimsOperationMetadata {
        name: ICIMS_DISCOVERY_FETCH_API_CONTRACT_OPERATION,
        domain: IcimsMetadataDomain::Discovery,
        kind: IcimsOperationKind::FetchApiContract,
        projection_status: IcimsProjectionStatus::EvidenceGated,
        gates: DEVELOPER_DOC_GATES,
        source_refs: SOURCE_REFS,
        notes: "Capture authenticated API docs before request planning or auth implementation.",
    },
    IcimsOperationMetadata {
        name: ICIMS_RECRUITING_QUERY_CANDIDATES_INCREMENTAL_OPERATION,
        domain: IcimsMetadataDomain::Recruiting,
        kind: IcimsOperationKind::QueryCandidatesIncremental,
        projection_status: IcimsProjectionStatus::EvidenceGated,
        gates: DEVELOPER_DOC_GATES,
        source_refs: SOURCE_REFS,
        notes: "Candidate/profile reads require standard-field matrix, pagination, auth, and custom-field policy evidence.",
    },
    IcimsOperationMetadata {
        name: ICIMS_RECRUITING_QUERY_JOBS_INCREMENTAL_OPERATION,
        domain: IcimsMetadataDomain::Recruiting,
        kind: IcimsOperationKind::QueryJobsIncremental,
        projection_status: IcimsProjectionStatus::EvidenceGated,
        gates: DEVELOPER_DOC_GATES,
        source_refs: SOURCE_REFS,
        notes: "Job/requisition reads require schema and repeatable standard-field evidence.",
    },
    IcimsOperationMetadata {
        name: ICIMS_RECRUITING_QUERY_APPLICATIONS_INCREMENTAL_OPERATION,
        domain: IcimsMetadataDomain::Recruiting,
        kind: IcimsOperationKind::QueryApplicationsIncremental,
        projection_status: IcimsProjectionStatus::EvidenceGated,
        gates: DEVELOPER_DOC_GATES,
        source_refs: SOURCE_REFS,
        notes: "Application/workflow reads require schema, status model, pagination, and live sample evidence.",
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IcimsOperationRegistry;

    #[test]
    fn api_snapshot_metadata_records_authenticated_docs_gap() {
        let metadata = icims_api_snapshot_metadata();

        assert_eq!(metadata.vendor_key, "icims");
        assert!(metadata.release_basis.contains("authenticated API docs"));
        assert!(metadata.blocking_gap.contains("Developer Community"));
        assert!(metadata
            .source_refs
            .contains(&ICIMS_PUBLIC_DOCS_SUMMARY_PATH));
    }

    #[test]
    fn metadata_operation_names_match_registry_constants() {
        let registry = IcimsOperationRegistry::default();
        let registry_names = registry.operation_names().collect::<Vec<_>>();
        let mut metadata_names = icims_operation_metadata_catalog()
            .iter()
            .map(|metadata| metadata.name)
            .collect::<Vec<_>>();
        metadata_names.sort_unstable();

        assert_eq!(metadata_names, registry_names);
    }
}
