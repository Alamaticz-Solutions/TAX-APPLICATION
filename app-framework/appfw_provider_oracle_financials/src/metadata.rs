use crate::{
    OracleFinancialsDataSensitivity, OracleFinancialsOperationGate,
    OracleFinancialsOperationPrimitive,
    ORACLE_FINANCIALS_ACCOUNTING_PERIOD_STATUS_LOV_GET_OPERATION,
    ORACLE_FINANCIALS_ACCOUNTING_PERIOD_STATUS_LOV_LIST_OPERATION, ORACLE_FINANCIALS_API_FAMILY,
    ORACLE_FINANCIALS_COLLECTION_QUERY_OPERATION, ORACLE_FINANCIALS_FETCH_BY_ID_OPERATION,
    ORACLE_FINANCIALS_LIST_LOV_OPERATION, ORACLE_FINANCIALS_OPENAPI_DESCRIBE_RESOURCE_OPERATION,
    ORACLE_FINANCIALS_OPENAPI_VERSION, ORACLE_FINANCIALS_RELEASE,
    ORACLE_FINANCIALS_REST_RESOURCES_BASE_PATH, ORACLE_FINANCIALS_SUBMIT_ERP_INTEGRATION_OPERATION,
};

pub const ORACLE_FINANCIALS_VENDOR_KEY: &str = "oracle-fusion-financials-26b";
pub const ORACLE_FINANCIALS_API_SPEC_PACKAGE_ID: &str = "oracle-fusion-financials-26b-openapi";
pub const ORACLE_FINANCIALS_INFORMATION_PACKAGE_VERSION: &str = "2026.06.26.1";
pub const ORACLE_FINANCIALS_INFORMATION_PACKAGE_VERSIONS_PATH: &str =
    "vendor_api_specs_2026-06-26/enrichment/information-package-versions.json";
pub const ORACLE_FINANCIALS_IMPLEMENTATION_METADATA_PATH: &str =
    "vendor_api_specs_2026-06-26/enrichment/saas-provider-implementation-metadata.json";
pub const ORACLE_FINANCIALS_OPENAPI_SUMMARY_PATH: &str =
    "vendor_api_specs_2026-06-26/enrichment/extracted/oracle-financials-openapi-summary.json";
pub const ORACLE_FINANCIALS_OPENAPI_SOURCE_URL: &str =
    "https://docs.oracle.com/en/cloud/saas/financials/26b/farfa/openapi.json";
pub const ORACLE_FINANCIALS_OPENAPI_PATH_COUNT: u16 = 1_391;
pub const ORACLE_FINANCIALS_OPENAPI_OPERATION_COUNT: u16 = 2_241;
pub const ORACLE_FINANCIALS_OPENAPI_GET_OPERATION_COUNT: u16 = 1_325;

const API_SNAPSHOT_SOURCES: &[&str] = &[
    ORACLE_FINANCIALS_INFORMATION_PACKAGE_VERSIONS_PATH,
    ORACLE_FINANCIALS_IMPLEMENTATION_METADATA_PATH,
    ORACLE_FINANCIALS_OPENAPI_SUMMARY_PATH,
    ORACLE_FINANCIALS_OPENAPI_SOURCE_URL,
];
const NO_GATES: &[OracleFinancialsOperationGate] = &[];
const TENANT_ALLOWLIST_GATES: &[OracleFinancialsOperationGate] = &[
    OracleFinancialsOperationGate::TenantOperationAllowList,
    OracleFinancialsOperationGate::LiveCertification,
];
const LIVE_WRITE_GATES: &[OracleFinancialsOperationGate] = &[
    OracleFinancialsOperationGate::GovernedWriteReview,
    OracleFinancialsOperationGate::TenantAuthCertification,
    OracleFinancialsOperationGate::LiveCertification,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OracleFinancialsApiSnapshotMetadata {
    pub vendor_key: &'static str,
    pub package_id: &'static str,
    pub information_package_version: &'static str,
    pub api_family: &'static str,
    pub release: &'static str,
    pub openapi_version: &'static str,
    pub rest_resources_base_path: &'static str,
    pub path_count: u16,
    pub operation_count: u16,
    pub get_operation_count: u16,
    pub source_refs: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OracleFinancialsProjectionStatus {
    Executable,
    AllowListGated,
    WriteGated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OracleFinancialsMetadataDomain {
    OpenApiMetadata,
    FinancialsLov,
    FinancialsCollection,
    ErpIntegration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OracleFinancialsOperationMetadata {
    pub name: &'static str,
    pub domain: OracleFinancialsMetadataDomain,
    pub primitive: OracleFinancialsOperationPrimitive,
    pub path: Option<&'static str>,
    pub projection_status: OracleFinancialsProjectionStatus,
    pub sensitivity: OracleFinancialsDataSensitivity,
    pub gates: &'static [OracleFinancialsOperationGate],
    pub source_refs: &'static [&'static str],
    pub notes: &'static str,
}

pub fn oracle_financials_api_snapshot_metadata() -> OracleFinancialsApiSnapshotMetadata {
    OracleFinancialsApiSnapshotMetadata {
        vendor_key: ORACLE_FINANCIALS_VENDOR_KEY,
        package_id: ORACLE_FINANCIALS_API_SPEC_PACKAGE_ID,
        information_package_version: ORACLE_FINANCIALS_INFORMATION_PACKAGE_VERSION,
        api_family: ORACLE_FINANCIALS_API_FAMILY,
        release: ORACLE_FINANCIALS_RELEASE,
        openapi_version: ORACLE_FINANCIALS_OPENAPI_VERSION,
        rest_resources_base_path: ORACLE_FINANCIALS_REST_RESOURCES_BASE_PATH,
        path_count: ORACLE_FINANCIALS_OPENAPI_PATH_COUNT,
        operation_count: ORACLE_FINANCIALS_OPENAPI_OPERATION_COUNT,
        get_operation_count: ORACLE_FINANCIALS_OPENAPI_GET_OPERATION_COUNT,
        source_refs: API_SNAPSHOT_SOURCES,
    }
}

pub fn oracle_financials_operation_metadata_catalog() -> &'static [OracleFinancialsOperationMetadata]
{
    OPERATION_METADATA
}

pub fn oracle_financials_operation_metadata(
    name: &str,
) -> Option<&'static OracleFinancialsOperationMetadata> {
    OPERATION_METADATA
        .iter()
        .find(|metadata| metadata.name == name)
}

const OPERATION_METADATA: &[OracleFinancialsOperationMetadata] = &[
    OracleFinancialsOperationMetadata {
        name: ORACLE_FINANCIALS_OPENAPI_DESCRIBE_RESOURCE_OPERATION,
        domain: OracleFinancialsMetadataDomain::OpenApiMetadata,
        primitive: OracleFinancialsOperationPrimitive::OpenApiDescribeResource,
        path: None,
        projection_status: OracleFinancialsProjectionStatus::AllowListGated,
        sensitivity: OracleFinancialsDataSensitivity::Operational,
        gates: TENANT_ALLOWLIST_GATES,
        source_refs: API_SNAPSHOT_SOURCES,
        notes: "OpenAPI describe planning stays gated until selected resource descriptions are allowlisted.",
    },
    OracleFinancialsOperationMetadata {
        name: ORACLE_FINANCIALS_COLLECTION_QUERY_OPERATION,
        domain: OracleFinancialsMetadataDomain::FinancialsCollection,
        primitive: OracleFinancialsOperationPrimitive::CollectionQuery,
        path: None,
        projection_status: OracleFinancialsProjectionStatus::AllowListGated,
        sensitivity: OracleFinancialsDataSensitivity::FinancialConfidential,
        gates: TENANT_ALLOWLIST_GATES,
        source_refs: API_SNAPSHOT_SOURCES,
        notes: "Generic collection reads must become registry-bound operations before execution.",
    },
    OracleFinancialsOperationMetadata {
        name: ORACLE_FINANCIALS_FETCH_BY_ID_OPERATION,
        domain: OracleFinancialsMetadataDomain::FinancialsCollection,
        primitive: OracleFinancialsOperationPrimitive::FetchById,
        path: None,
        projection_status: OracleFinancialsProjectionStatus::AllowListGated,
        sensitivity: OracleFinancialsDataSensitivity::FinancialConfidential,
        gates: TENANT_ALLOWLIST_GATES,
        source_refs: API_SNAPSHOT_SOURCES,
        notes: "Generic fetch-by-id must become a registry-bound path before execution.",
    },
    OracleFinancialsOperationMetadata {
        name: ORACLE_FINANCIALS_LIST_LOV_OPERATION,
        domain: OracleFinancialsMetadataDomain::FinancialsLov,
        primitive: OracleFinancialsOperationPrimitive::ListLov,
        path: None,
        projection_status: OracleFinancialsProjectionStatus::AllowListGated,
        sensitivity: OracleFinancialsDataSensitivity::Operational,
        gates: TENANT_ALLOWLIST_GATES,
        source_refs: API_SNAPSHOT_SOURCES,
        notes: "Generic LOV reads must become registry-bound operations before execution.",
    },
    OracleFinancialsOperationMetadata {
        name: ORACLE_FINANCIALS_SUBMIT_ERP_INTEGRATION_OPERATION,
        domain: OracleFinancialsMetadataDomain::ErpIntegration,
        primitive: OracleFinancialsOperationPrimitive::SubmitErpIntegration,
        path: Some("/erpintegrations"),
        projection_status: OracleFinancialsProjectionStatus::WriteGated,
        sensitivity: OracleFinancialsDataSensitivity::FinancialConfidential,
        gates: LIVE_WRITE_GATES,
        source_refs: API_SNAPSHOT_SOURCES,
        notes: "ERP integration submission is cataloged for future governed write review only.",
    },
    OracleFinancialsOperationMetadata {
        name: ORACLE_FINANCIALS_ACCOUNTING_PERIOD_STATUS_LOV_LIST_OPERATION,
        domain: OracleFinancialsMetadataDomain::FinancialsLov,
        primitive: OracleFinancialsOperationPrimitive::ListLov,
        path: Some("/accountingPeriodStatusLOV"),
        projection_status: OracleFinancialsProjectionStatus::Executable,
        sensitivity: OracleFinancialsDataSensitivity::Operational,
        gates: NO_GATES,
        source_refs: API_SNAPSHOT_SOURCES,
        notes: "Fixed accounting period status LOV collection read from the Oracle 26B OpenAPI summary.",
    },
    OracleFinancialsOperationMetadata {
        name: ORACLE_FINANCIALS_ACCOUNTING_PERIOD_STATUS_LOV_GET_OPERATION,
        domain: OracleFinancialsMetadataDomain::FinancialsLov,
        primitive: OracleFinancialsOperationPrimitive::FetchById,
        path: Some("/accountingPeriodStatusLOV/{accountingPeriodStatusLOVUniqID}"),
        projection_status: OracleFinancialsProjectionStatus::Executable,
        sensitivity: OracleFinancialsDataSensitivity::Operational,
        gates: NO_GATES,
        source_refs: API_SNAPSHOT_SOURCES,
        notes: "Fixed accounting period status LOV fetch-by-id from the Oracle 26B OpenAPI summary.",
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OracleFinancialsOperationRegistry;

    #[test]
    fn api_snapshot_metadata_pins_oracle_financials_openapi_release() {
        let metadata = oracle_financials_api_snapshot_metadata();

        assert_eq!(metadata.vendor_key, "oracle-fusion-financials-26b");
        assert_eq!(metadata.release, "26B");
        assert_eq!(metadata.openapi_version, "2026.03.27");
        assert_eq!(metadata.path_count, 1_391);
        assert_eq!(metadata.operation_count, 2_241);
        assert!(metadata
            .source_refs
            .contains(&ORACLE_FINANCIALS_OPENAPI_SUMMARY_PATH));
    }

    #[test]
    fn metadata_operation_names_match_registry_constants() {
        let registry = OracleFinancialsOperationRegistry::default();
        let registry_names = registry.operation_names().collect::<Vec<_>>();
        let mut metadata_names = oracle_financials_operation_metadata_catalog()
            .iter()
            .map(|metadata| metadata.name)
            .collect::<Vec<_>>();
        metadata_names.sort_unstable();

        assert_eq!(metadata_names, registry_names);
    }

    #[test]
    fn executable_metadata_is_limited_to_registry_bound_lov_operations() {
        let executable = oracle_financials_operation_metadata_catalog()
            .iter()
            .filter(|metadata| {
                metadata.projection_status == OracleFinancialsProjectionStatus::Executable
            })
            .map(|metadata| metadata.name)
            .collect::<Vec<_>>();

        assert_eq!(
            executable,
            vec![
                ORACLE_FINANCIALS_ACCOUNTING_PERIOD_STATUS_LOV_LIST_OPERATION,
                ORACLE_FINANCIALS_ACCOUNTING_PERIOD_STATUS_LOV_GET_OPERATION,
            ]
        );
    }
}
