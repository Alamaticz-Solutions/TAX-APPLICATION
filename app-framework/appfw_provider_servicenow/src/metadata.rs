use crate::{
    ServiceNowOperationGate, ServiceNowOperationKind, SERVICENOW_API_FAMILY,
    SERVICENOW_INCIDENT_CREATE_OPERATION, SERVICENOW_RELEASE_BASIS,
    SERVICENOW_TABLE_EXPORT_SCHEMA_OPERATION, SERVICENOW_TABLE_FETCH_BY_SYS_IDS_OPERATION,
    SERVICENOW_TABLE_FETCH_REFERENCE_VALUES_OPERATION,
    SERVICENOW_TABLE_QUERY_INCREMENTAL_OPERATION,
};

pub const SERVICENOW_VENDOR_KEY: &str = "servicenow-australia";
pub const SERVICENOW_API_SPEC_PACKAGE_ID: &str = "servicenow-australia-api-specs";
pub const SERVICENOW_INFORMATION_PACKAGE_VERSION: &str = "2026.06.26.1";
pub const SERVICENOW_INFORMATION_PACKAGE_VERSIONS_PATH: &str =
    "vendor_api_specs_2026-06-26/enrichment/information-package-versions.json";
pub const SERVICENOW_IMPLEMENTATION_METADATA_PATH: &str =
    "vendor_api_specs_2026-06-26/enrichment/saas-provider-implementation-metadata.json";
pub const SERVICENOW_PUBLIC_DOCS_SUMMARY_PATH: &str =
    "vendor_api_specs_2026-06-26/enrichment/extracted/servicenow-public-docs-summary.json";
pub const SERVICENOW_TABLE_API_DOC_URL: &str =
    "https://www.servicenow.com/docs/r/api-reference/rest-apis/c_TableAPI.html";

const SOURCE_REFS: &[&str] = &[
    SERVICENOW_INFORMATION_PACKAGE_VERSIONS_PATH,
    SERVICENOW_IMPLEMENTATION_METADATA_PATH,
    SERVICENOW_PUBLIC_DOCS_SUMMARY_PATH,
    SERVICENOW_TABLE_API_DOC_URL,
];
const INSTANCE_EXPORT_GATES: &[ServiceNowOperationGate] = &[
    ServiceNowOperationGate::AuthenticatedInstanceOpenApiExport,
    ServiceNowOperationGate::TableDictionaryAndAclExport,
    ServiceNowOperationGate::LiveCertification,
];
const GOVERNED_WRITE_GATES: &[ServiceNowOperationGate] = &[
    ServiceNowOperationGate::DelegatedActorContextEvidence,
    ServiceNowOperationGate::TokenStoreIsolationEvidence,
    ServiceNowOperationGate::NamedMutationRegistryEvidence,
    ServiceNowOperationGate::MutationRequestBindingEvidence,
    ServiceNowOperationGate::IdempotencyReplayEvidence,
    ServiceNowOperationGate::WritePolicyScopeEvidence,
    ServiceNowOperationGate::WriteAuditEvidence,
    ServiceNowOperationGate::LiveCertification,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServiceNowApiSnapshotMetadata {
    pub vendor_key: &'static str,
    pub package_id: &'static str,
    pub information_package_version: &'static str,
    pub api_family: &'static str,
    pub release_basis: &'static str,
    pub source_refs: &'static [&'static str],
    pub blocking_gap: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceNowProjectionStatus {
    EvidenceGated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceNowMetadataDomain {
    InstanceExport,
    TableRead,
    ReferenceData,
    GovernedWrite,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServiceNowOperationMetadata {
    pub name: &'static str,
    pub domain: ServiceNowMetadataDomain,
    pub kind: ServiceNowOperationKind,
    pub projection_status: ServiceNowProjectionStatus,
    pub gates: &'static [ServiceNowOperationGate],
    pub source_refs: &'static [&'static str],
    pub notes: &'static str,
}

pub fn servicenow_api_snapshot_metadata() -> ServiceNowApiSnapshotMetadata {
    ServiceNowApiSnapshotMetadata {
        vendor_key: SERVICENOW_VENDOR_KEY,
        package_id: SERVICENOW_API_SPEC_PACKAGE_ID,
        information_package_version: SERVICENOW_INFORMATION_PACKAGE_VERSION,
        api_family: SERVICENOW_API_FAMILY,
        release_basis: SERVICENOW_RELEASE_BASIS,
        source_refs: SOURCE_REFS,
        blocking_gap:
            "Need authenticated ServiceNow instance export for table fields, ACLs, plugins, domains, and OpenAPI schemas.",
    }
}

pub fn servicenow_operation_metadata_catalog() -> &'static [ServiceNowOperationMetadata] {
    OPERATION_METADATA
}

pub fn servicenow_operation_metadata(name: &str) -> Option<&'static ServiceNowOperationMetadata> {
    OPERATION_METADATA
        .iter()
        .find(|metadata| metadata.name == name)
}

const OPERATION_METADATA: &[ServiceNowOperationMetadata] = &[
    ServiceNowOperationMetadata {
        name: SERVICENOW_TABLE_EXPORT_SCHEMA_OPERATION,
        domain: ServiceNowMetadataDomain::InstanceExport,
        kind: ServiceNowOperationKind::ExportSchemaFromInstance,
        projection_status: ServiceNowProjectionStatus::EvidenceGated,
        gates: INSTANCE_EXPORT_GATES,
        source_refs: SOURCE_REFS,
        notes: "Capture authenticated REST API Explorer OpenAPI and table dictionary exports before request planning.",
    },
    ServiceNowOperationMetadata {
        name: SERVICENOW_TABLE_QUERY_INCREMENTAL_OPERATION,
        domain: ServiceNowMetadataDomain::TableRead,
        kind: ServiceNowOperationKind::QueryIncremental,
        projection_status: ServiceNowProjectionStatus::EvidenceGated,
        gates: INSTANCE_EXPORT_GATES,
        source_refs: SOURCE_REFS,
        notes: "Incremental reads require exported fields plus certified sys_updated_on/sys_id predicate building.",
    },
    ServiceNowOperationMetadata {
        name: SERVICENOW_TABLE_FETCH_BY_SYS_IDS_OPERATION,
        domain: ServiceNowMetadataDomain::TableRead,
        kind: ServiceNowOperationKind::FetchBySysIds,
        projection_status: ServiceNowProjectionStatus::EvidenceGated,
        gates: INSTANCE_EXPORT_GATES,
        source_refs: SOURCE_REFS,
        notes: "Fetch by sys_id requires selected tables and readable field projections from the target instance.",
    },
    ServiceNowOperationMetadata {
        name: SERVICENOW_TABLE_FETCH_REFERENCE_VALUES_OPERATION,
        domain: ServiceNowMetadataDomain::ReferenceData,
        kind: ServiceNowOperationKind::FetchReferenceValues,
        projection_status: ServiceNowProjectionStatus::EvidenceGated,
        gates: INSTANCE_EXPORT_GATES,
        source_refs: SOURCE_REFS,
        notes: "Reference reads require selected reference fields and display-value policy from the target instance.",
    },
    ServiceNowOperationMetadata {
        name: SERVICENOW_INCIDENT_CREATE_OPERATION,
        domain: ServiceNowMetadataDomain::GovernedWrite,
        kind: ServiceNowOperationKind::CreateIncident,
        projection_status: ServiceNowProjectionStatus::EvidenceGated,
        gates: GOVERNED_WRITE_GATES,
        source_refs: SOURCE_REFS,
        notes: "Incident creation is the first named governed-write candidate. It remains non-executable until live G1 evidence proves delegated actor context, token-store isolation, mutation request binding, idempotency, policy/scope enforcement, and audit.",
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ServiceNowOperationRegistry;

    #[test]
    fn api_snapshot_metadata_records_authenticated_export_gap() {
        let metadata = servicenow_api_snapshot_metadata();

        assert_eq!(metadata.vendor_key, "servicenow-australia");
        assert!(metadata.release_basis.contains("instance export required"));
        assert!(metadata.blocking_gap.contains("authenticated ServiceNow"));
        assert!(metadata
            .source_refs
            .contains(&SERVICENOW_PUBLIC_DOCS_SUMMARY_PATH));
    }

    #[test]
    fn metadata_operation_names_match_registry_constants() {
        let registry = ServiceNowOperationRegistry::default();
        let registry_names = registry.operation_names().collect::<Vec<_>>();
        let mut metadata_names = servicenow_operation_metadata_catalog()
            .iter()
            .map(|metadata| metadata.name)
            .collect::<Vec<_>>();
        metadata_names.sort_unstable();

        assert_eq!(metadata_names, registry_names);
    }
}
