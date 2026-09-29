use crate::{
    SalesforceDataSensitivity, SalesforceObject, SalesforceOperationGate,
    SalesforceOperationPrimitive, SALESFORCE_REST_API_VERSION, SALESFORCE_REST_BASE_PATH,
};

use crate::registry::{
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

pub const SALESFORCE_RELEASE_PIN: &str = "Summer '26";
pub const SALESFORCE_API_VERSION_PIN: &str = "API 67.0";
pub const SALESFORCE_DOC_VERSION_PIN: &str = "262.0";
pub const SALESFORCE_VENDOR_KEY: &str = "salesforce-health-cloud";
pub const SALESFORCE_API_SPEC_PACKAGE_ID: &str = "salesforce-health-cloud-api-specs";
pub const SALESFORCE_INFORMATION_PACKAGE_VERSION: &str = "2026.06.26.1";
pub const SALESFORCE_INFORMATION_PACKAGE_VERSIONS_PATH: &str =
    "vendor_api_specs_2026-06-26/enrichment/information-package-versions.json";
pub const SALESFORCE_IMPLEMENTATION_METADATA_PATH: &str =
    "vendor_api_specs_2026-06-26/enrichment/saas-provider-implementation-metadata.json";
pub const SALESFORCE_HEALTH_CLOUD_CATALOG_PATH: &str =
    "vendor_api_specs_2026-06-26/enrichment/extracted/salesforce-health-cloud-catalog.json";
pub const SALESFORCE_HEALTH_CLOUD_OBJECT_REFERENCE_DOC_URL: &str =
    "https://developer.salesforce.com/docs/get_document/atlas.en-us.health_cloud_object_reference.meta";
pub const SALESFORCE_REST_API_DOC_URL: &str =
    "https://developer.salesforce.com/docs/get_document/atlas.en-us.api_rest.meta";

const NO_GATES: &[SalesforceOperationGate] = &[];
const HEALTH_CLOUD_DESCRIBE_GATES: &[SalesforceOperationGate] = &[
    SalesforceOperationGate::LiveOrgDescribe,
    SalesforceOperationGate::GovernanceReview,
    SalesforceOperationGate::LiveCertification,
];
const API_SNAPSHOT_SOURCES: &[&str] = &[
    SALESFORCE_INFORMATION_PACKAGE_VERSIONS_PATH,
    SALESFORCE_IMPLEMENTATION_METADATA_PATH,
    SALESFORCE_HEALTH_CLOUD_CATALOG_PATH,
    SALESFORCE_HEALTH_CLOUD_OBJECT_REFERENCE_DOC_URL,
    SALESFORCE_REST_API_DOC_URL,
];
const REST_SOURCES: &[&str] = &[
    SALESFORCE_INFORMATION_PACKAGE_VERSIONS_PATH,
    SALESFORCE_IMPLEMENTATION_METADATA_PATH,
    SALESFORCE_REST_API_DOC_URL,
];
const HEALTH_CLOUD_SOURCES: &[&str] = &[
    SALESFORCE_INFORMATION_PACKAGE_VERSIONS_PATH,
    SALESFORCE_IMPLEMENTATION_METADATA_PATH,
    SALESFORCE_HEALTH_CLOUD_CATALOG_PATH,
    SALESFORCE_HEALTH_CLOUD_OBJECT_REFERENCE_DOC_URL,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SalesforceApiSnapshotMetadata {
    pub vendor_key: &'static str,
    pub package_id: &'static str,
    pub information_package_version: &'static str,
    pub release: &'static str,
    pub api_version: &'static str,
    pub rest_api_version: &'static str,
    pub doc_version: &'static str,
    pub rest_base_path: &'static str,
    pub source_refs: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SalesforceProjectionStatus {
    Executable,
    DescribeGated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SalesforceMetadataDomain {
    Crm,
    HealthCloud,
    PlatformRest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SalesforceOperationMetadata {
    pub name: &'static str,
    pub domain: SalesforceMetadataDomain,
    pub primitive: SalesforceOperationPrimitive,
    pub object: Option<SalesforceObject>,
    pub projection_status: SalesforceProjectionStatus,
    pub sensitivity: SalesforceDataSensitivity,
    pub gates: &'static [SalesforceOperationGate],
    pub source_refs: &'static [&'static str],
    pub notes: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SalesforceCatalogObjectMetadata {
    pub object: SalesforceObject,
    pub operation_name: &'static str,
    pub title: &'static str,
    pub slug: &'static str,
    pub source_url: &'static str,
    pub supported_calls: &'static [&'static str],
    pub catalog_field_count: u16,
    pub sample_fields: &'static [&'static str],
    pub projection_status: SalesforceProjectionStatus,
    pub sensitivity: SalesforceDataSensitivity,
    pub gates: &'static [SalesforceOperationGate],
    pub source_refs: &'static [&'static str],
}

pub fn salesforce_api_snapshot_metadata() -> SalesforceApiSnapshotMetadata {
    SalesforceApiSnapshotMetadata {
        vendor_key: SALESFORCE_VENDOR_KEY,
        package_id: SALESFORCE_API_SPEC_PACKAGE_ID,
        information_package_version: SALESFORCE_INFORMATION_PACKAGE_VERSION,
        release: SALESFORCE_RELEASE_PIN,
        api_version: SALESFORCE_API_VERSION_PIN,
        rest_api_version: SALESFORCE_REST_API_VERSION,
        doc_version: SALESFORCE_DOC_VERSION_PIN,
        rest_base_path: SALESFORCE_REST_BASE_PATH,
        source_refs: API_SNAPSHOT_SOURCES,
    }
}

pub fn salesforce_operation_metadata_catalog() -> &'static [SalesforceOperationMetadata] {
    OPERATION_METADATA
}

pub fn salesforce_operation_metadata(name: &str) -> Option<&'static SalesforceOperationMetadata> {
    OPERATION_METADATA
        .iter()
        .find(|metadata| metadata.name == name)
}

pub fn salesforce_health_cloud_projection_candidates() -> &'static [SalesforceCatalogObjectMetadata]
{
    HEALTH_CLOUD_PROJECTION_CANDIDATES
}

const OPERATION_METADATA: &[SalesforceOperationMetadata] = &[
    SalesforceOperationMetadata {
        name: ACCOUNT_DESCRIBE_OBJECT_OPERATION,
        domain: SalesforceMetadataDomain::Crm,
        primitive: SalesforceOperationPrimitive::DescribeObject,
        object: Some(SalesforceObject::Account),
        projection_status: SalesforceProjectionStatus::Executable,
        sensitivity: SalesforceDataSensitivity::Operational,
        gates: NO_GATES,
        source_refs: REST_SOURCES,
        notes: "Fixed Account describe request for drift checks.",
    },
    SalesforceOperationMetadata {
        name: ACCOUNT_BY_ID_OPERATION,
        domain: SalesforceMetadataDomain::Crm,
        primitive: SalesforceOperationPrimitive::FetchByIds,
        object: Some(SalesforceObject::Account),
        projection_status: SalesforceProjectionStatus::Executable,
        sensitivity: SalesforceDataSensitivity::BusinessConfidential,
        gates: NO_GATES,
        source_refs: REST_SOURCES,
        notes: "Fixed Account projection by id.",
    },
    SalesforceOperationMetadata {
        name: ACCOUNT_FETCH_BY_IDS_OPERATION,
        domain: SalesforceMetadataDomain::Crm,
        primitive: SalesforceOperationPrimitive::FetchByIds,
        object: Some(SalesforceObject::Account),
        projection_status: SalesforceProjectionStatus::Executable,
        sensitivity: SalesforceDataSensitivity::BusinessConfidential,
        gates: NO_GATES,
        source_refs: REST_SOURCES,
        notes: "Fixed Account projection for a bounded id list.",
    },
    SalesforceOperationMetadata {
        name: ACCOUNT_GET_UPDATED_IDS_OPERATION,
        domain: SalesforceMetadataDomain::Crm,
        primitive: SalesforceOperationPrimitive::GetUpdatedIds,
        object: Some(SalesforceObject::Account),
        projection_status: SalesforceProjectionStatus::Executable,
        sensitivity: SalesforceDataSensitivity::BusinessConfidential,
        gates: NO_GATES,
        source_refs: REST_SOURCES,
        notes: "Fixed Account getUpdated ID-only read using API 67.0 REST paths.",
    },
    SalesforceOperationMetadata {
        name: ACCOUNT_GET_DELETED_IDS_OPERATION,
        domain: SalesforceMetadataDomain::Crm,
        primitive: SalesforceOperationPrimitive::GetDeletedIds,
        object: Some(SalesforceObject::Account),
        projection_status: SalesforceProjectionStatus::Executable,
        sensitivity: SalesforceDataSensitivity::BusinessConfidential,
        gates: NO_GATES,
        source_refs: REST_SOURCES,
        notes: "Fixed Account getDeleted ID-only read using API 67.0 REST paths.",
    },
    SalesforceOperationMetadata {
        name: SALESFORCE_CONTINUE_QUERY_PAGE_OPERATION,
        domain: SalesforceMetadataDomain::PlatformRest,
        primitive: SalesforceOperationPrimitive::ContinueQueryPage,
        object: None,
        projection_status: SalesforceProjectionStatus::Executable,
        sensitivity: SalesforceDataSensitivity::BusinessConfidential,
        gates: NO_GATES,
        source_refs: REST_SOURCES,
        notes: "Validated nextRecordsUrl continuation for Salesforce query pages.",
    },
    SalesforceOperationMetadata {
        name: UPDATED_ACCOUNTS_OPERATION,
        domain: SalesforceMetadataDomain::Crm,
        primitive: SalesforceOperationPrimitive::QueryObjectIncremental,
        object: Some(SalesforceObject::Account),
        projection_status: SalesforceProjectionStatus::Executable,
        sensitivity: SalesforceDataSensitivity::BusinessConfidential,
        gates: NO_GATES,
        source_refs: REST_SOURCES,
        notes: "Fixed Account SOQL incremental read with provider-owned fields and caps.",
    },
    SalesforceOperationMetadata {
        name: SALESFORCE_HEALTH_FETCH_LIMITS_OPERATION,
        domain: SalesforceMetadataDomain::PlatformRest,
        primitive: SalesforceOperationPrimitive::FetchLimits,
        object: None,
        projection_status: SalesforceProjectionStatus::Executable,
        sensitivity: SalesforceDataSensitivity::Operational,
        gates: NO_GATES,
        source_refs: REST_SOURCES,
        notes: "REST org limits read for smoke tests and rate visibility.",
    },
    SalesforceOperationMetadata {
        name: SALESFORCE_HEALTH_DESCRIBE_OBJECT_OPERATION,
        domain: SalesforceMetadataDomain::HealthCloud,
        primitive: SalesforceOperationPrimitive::DescribeObject,
        object: None,
        projection_status: SalesforceProjectionStatus::DescribeGated,
        sensitivity: SalesforceDataSensitivity::PhiPii,
        gates: HEALTH_CLOUD_DESCRIBE_GATES,
        source_refs: HEALTH_CLOUD_SOURCES,
        notes: "Generic Health Cloud describe remains gated until authenticated org describe evidence.",
    },
    SalesforceOperationMetadata {
        name: SALESFORCE_HEALTH_GET_UPDATED_IDS_OPERATION,
        domain: SalesforceMetadataDomain::HealthCloud,
        primitive: SalesforceOperationPrimitive::GetUpdatedIds,
        object: None,
        projection_status: SalesforceProjectionStatus::DescribeGated,
        sensitivity: SalesforceDataSensitivity::PhiPii,
        gates: HEALTH_CLOUD_DESCRIBE_GATES,
        source_refs: HEALTH_CLOUD_SOURCES,
        notes: "Health Cloud getUpdated remains object-allowlist and org-describe gated.",
    },
    SalesforceOperationMetadata {
        name: SALESFORCE_HEALTH_GET_DELETED_IDS_OPERATION,
        domain: SalesforceMetadataDomain::HealthCloud,
        primitive: SalesforceOperationPrimitive::GetDeletedIds,
        object: None,
        projection_status: SalesforceProjectionStatus::DescribeGated,
        sensitivity: SalesforceDataSensitivity::PhiPii,
        gates: HEALTH_CLOUD_DESCRIBE_GATES,
        source_refs: HEALTH_CLOUD_SOURCES,
        notes: "Health Cloud getDeleted remains object-allowlist and org-describe gated.",
    },
    SalesforceOperationMetadata {
        name: SALESFORCE_HEALTH_FETCH_BY_IDS_OPERATION,
        domain: SalesforceMetadataDomain::HealthCloud,
        primitive: SalesforceOperationPrimitive::FetchByIds,
        object: None,
        projection_status: SalesforceProjectionStatus::DescribeGated,
        sensitivity: SalesforceDataSensitivity::PhiPii,
        gates: HEALTH_CLOUD_DESCRIBE_GATES,
        source_refs: HEALTH_CLOUD_SOURCES,
        notes: "Generic Health Cloud fetch-by-ids needs certified fixed projections first.",
    },
    SalesforceOperationMetadata {
        name: SALESFORCE_HEALTH_QUERY_OBJECT_INCREMENTAL_OPERATION,
        domain: SalesforceMetadataDomain::HealthCloud,
        primitive: SalesforceOperationPrimitive::QueryObjectIncremental,
        object: None,
        projection_status: SalesforceProjectionStatus::DescribeGated,
        sensitivity: SalesforceDataSensitivity::PhiPii,
        gates: HEALTH_CLOUD_DESCRIBE_GATES,
        source_refs: HEALTH_CLOUD_SOURCES,
        notes: "Generic Health Cloud incremental query must not expose caller-selected objects or fields.",
    },
    SalesforceOperationMetadata {
        name: HEALTH_CLOUD_CARE_PROGRAMS_UPDATED_SINCE_OPERATION,
        domain: SalesforceMetadataDomain::HealthCloud,
        primitive: SalesforceOperationPrimitive::QueryObjectIncremental,
        object: Some(SalesforceObject::CareProgram),
        projection_status: SalesforceProjectionStatus::DescribeGated,
        sensitivity: SalesforceDataSensitivity::PhiPii,
        gates: HEALTH_CLOUD_DESCRIBE_GATES,
        source_refs: HEALTH_CLOUD_SOURCES,
        notes: "CareProgram projection candidate from the local Health Cloud catalog.",
    },
    SalesforceOperationMetadata {
        name: HEALTH_CLOUD_CARE_PROGRAM_ENROLLEES_BY_PROGRAM_OPERATION,
        domain: SalesforceMetadataDomain::HealthCloud,
        primitive: SalesforceOperationPrimitive::QueryObjectIncremental,
        object: Some(SalesforceObject::CareProgramEnrollee),
        projection_status: SalesforceProjectionStatus::DescribeGated,
        sensitivity: SalesforceDataSensitivity::PhiPii,
        gates: HEALTH_CLOUD_DESCRIBE_GATES,
        source_refs: HEALTH_CLOUD_SOURCES,
        notes: "CareProgramEnrollee projection candidate from the local Health Cloud catalog.",
    },
    SalesforceOperationMetadata {
        name: HEALTH_CLOUD_COVERAGE_BENEFIT_ITEMS_BY_MEMBER_OPERATION,
        domain: SalesforceMetadataDomain::HealthCloud,
        primitive: SalesforceOperationPrimitive::QueryObjectIncremental,
        object: Some(SalesforceObject::CoverageBenefitItem),
        projection_status: SalesforceProjectionStatus::DescribeGated,
        sensitivity: SalesforceDataSensitivity::PhiPii,
        gates: HEALTH_CLOUD_DESCRIBE_GATES,
        source_refs: HEALTH_CLOUD_SOURCES,
        notes: "CoverageBenefitItem projection candidate from the local Health Cloud catalog.",
    },
    SalesforceOperationMetadata {
        name: HEALTH_CLOUD_CLINICAL_ENCOUNTERS_UPDATED_SINCE_OPERATION,
        domain: SalesforceMetadataDomain::HealthCloud,
        primitive: SalesforceOperationPrimitive::QueryObjectIncremental,
        object: Some(SalesforceObject::ClinicalEncounter),
        projection_status: SalesforceProjectionStatus::DescribeGated,
        sensitivity: SalesforceDataSensitivity::PhiPii,
        gates: HEALTH_CLOUD_DESCRIBE_GATES,
        source_refs: HEALTH_CLOUD_SOURCES,
        notes: "ClinicalEncounter projection candidate from the local Health Cloud catalog.",
    },
];

const HEALTH_CLOUD_CATALOG_SUPPORTED_CALLS: &[&str] = &[
    "create()",
    "delete()",
    "describeLayout()",
    "describeSObjects()",
    "getDeleted()",
    "getUpdated()",
    "query()",
    "retrieve()",
    "search()",
    "undelete()",
    "update()",
    "upsert()",
];

const CARE_PROGRAM_SAMPLE_FIELDS: &[&str] = &[
    "CareProgramName",
    "Category",
    "CurrentEnrolleeCount",
    "Description",
    "EndDate",
    "Name",
    "OwnerId",
    "SourceSystem",
    "SourceSystemIdentifier",
    "StartDate",
    "Status",
];
const CARE_PROGRAM_ENROLLEE_SAMPLE_FIELDS: &[&str] = &[
    "AccountId",
    "CareProgramId",
    "ClinicalServiceRequestId",
    "EnrolleeType",
    "IsActive",
    "Name",
    "SourceSystem",
    "SourceSystemIdentifier",
    "Status",
    "UserId",
];
const COVERAGE_BENEFIT_ITEM_SAMPLE_FIELDS: &[&str] = &[
    "BenefitCategory",
    "CoverageBenefitId",
    "CoverageLevel",
    "IsActive",
    "IsPreauthorizationRequired",
    "MemberId",
    "Name",
    "Notes",
    "SourceSystem",
    "SourceSystemIdentifier",
    "SourceSystemModified",
];
const CLINICAL_ENCOUNTER_SAMPLE_FIELDS: &[&str] = &[
    "AdmissionSource",
    "CaseId",
    "Category",
    "EndDate",
    "FacilityId",
    "Name",
    "PatientId",
    "ServiceType",
    "SourceSystem",
    "SourceSystemIdentifier",
    "SourceSystemModified",
    "StartDate",
    "Status",
];

const HEALTH_CLOUD_PROJECTION_CANDIDATES: &[SalesforceCatalogObjectMetadata] = &[
    SalesforceCatalogObjectMetadata {
        object: SalesforceObject::CareProgram,
        operation_name: HEALTH_CLOUD_CARE_PROGRAMS_UPDATED_SINCE_OPERATION,
        title: "CareProgram",
        slug: "sforce_api_objects_careprogram.htm",
        source_url:
            "https://developer.salesforce.com/docs/get_document_content/health_cloud_object_reference/sforce_api_objects_careprogram.htm/en-us/262.0",
        supported_calls: HEALTH_CLOUD_CATALOG_SUPPORTED_CALLS,
        catalog_field_count: 32,
        sample_fields: CARE_PROGRAM_SAMPLE_FIELDS,
        projection_status: SalesforceProjectionStatus::DescribeGated,
        sensitivity: SalesforceDataSensitivity::PhiPii,
        gates: HEALTH_CLOUD_DESCRIBE_GATES,
        source_refs: HEALTH_CLOUD_SOURCES,
    },
    SalesforceCatalogObjectMetadata {
        object: SalesforceObject::CareProgramEnrollee,
        operation_name: HEALTH_CLOUD_CARE_PROGRAM_ENROLLEES_BY_PROGRAM_OPERATION,
        title: "CareProgramEnrollee",
        slug: "sforce_api_objects_careprogramenrollee.htm",
        source_url:
            "https://developer.salesforce.com/docs/get_document_content/health_cloud_object_reference/sforce_api_objects_careprogramenrollee.htm/en-us/262.0",
        supported_calls: HEALTH_CLOUD_CATALOG_SUPPORTED_CALLS,
        catalog_field_count: 25,
        sample_fields: CARE_PROGRAM_ENROLLEE_SAMPLE_FIELDS,
        projection_status: SalesforceProjectionStatus::DescribeGated,
        sensitivity: SalesforceDataSensitivity::PhiPii,
        gates: HEALTH_CLOUD_DESCRIBE_GATES,
        source_refs: HEALTH_CLOUD_SOURCES,
    },
    SalesforceCatalogObjectMetadata {
        object: SalesforceObject::CoverageBenefitItem,
        operation_name: HEALTH_CLOUD_COVERAGE_BENEFIT_ITEMS_BY_MEMBER_OPERATION,
        title: "CoverageBenefitItem",
        slug: "sforce_api_objects_coveragebenefititem.htm",
        source_url:
            "https://developer.salesforce.com/docs/get_document_content/health_cloud_object_reference/sforce_api_objects_coveragebenefititem.htm/en-us/262.0",
        supported_calls: HEALTH_CLOUD_CATALOG_SUPPORTED_CALLS,
        catalog_field_count: 23,
        sample_fields: COVERAGE_BENEFIT_ITEM_SAMPLE_FIELDS,
        projection_status: SalesforceProjectionStatus::DescribeGated,
        sensitivity: SalesforceDataSensitivity::PhiPii,
        gates: HEALTH_CLOUD_DESCRIBE_GATES,
        source_refs: HEALTH_CLOUD_SOURCES,
    },
    SalesforceCatalogObjectMetadata {
        object: SalesforceObject::ClinicalEncounter,
        operation_name: HEALTH_CLOUD_CLINICAL_ENCOUNTERS_UPDATED_SINCE_OPERATION,
        title: "ClinicalEncounter",
        slug: "sforce_api_objects_clinicalencounter.htm",
        source_url:
            "https://developer.salesforce.com/docs/get_document_content/health_cloud_object_reference/sforce_api_objects_clinicalencounter.htm/en-us/262.0",
        supported_calls: HEALTH_CLOUD_CATALOG_SUPPORTED_CALLS,
        catalog_field_count: 30,
        sample_fields: CLINICAL_ENCOUNTER_SAMPLE_FIELDS,
        projection_status: SalesforceProjectionStatus::DescribeGated,
        sensitivity: SalesforceDataSensitivity::PhiPii,
        gates: HEALTH_CLOUD_DESCRIBE_GATES,
        source_refs: HEALTH_CLOUD_SOURCES,
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SalesforceOperationRegistry;

    #[test]
    fn api_67_metadata_is_exposed_for_named_operations() {
        let snapshot = salesforce_api_snapshot_metadata();

        assert_eq!(snapshot.vendor_key, "salesforce-health-cloud");
        assert_eq!(snapshot.package_id, "salesforce-health-cloud-api-specs");
        assert_eq!(snapshot.information_package_version, "2026.06.26.1");
        assert_eq!(snapshot.release, "Summer '26");
        assert_eq!(snapshot.api_version, "API 67.0");
        assert_eq!(snapshot.rest_api_version, "v67.0");
        assert_eq!(snapshot.doc_version, "262.0");
        assert_eq!(snapshot.rest_base_path, "/services/data/v67.0");
        assert!(snapshot
            .source_refs
            .contains(&SALESFORCE_HEALTH_CLOUD_CATALOG_PATH));

        let account_updates =
            salesforce_operation_metadata(ACCOUNT_GET_UPDATED_IDS_OPERATION).expect("metadata");
        assert_eq!(
            account_updates.primitive,
            SalesforceOperationPrimitive::GetUpdatedIds
        );
        assert_eq!(account_updates.object, Some(SalesforceObject::Account));
        assert_eq!(
            account_updates.projection_status,
            SalesforceProjectionStatus::Executable
        );
        assert!(account_updates.gates.is_empty());

        let health_query =
            salesforce_operation_metadata(SALESFORCE_HEALTH_QUERY_OBJECT_INCREMENTAL_OPERATION)
                .expect("metadata");
        assert_eq!(
            health_query.projection_status,
            SalesforceProjectionStatus::DescribeGated
        );
        assert!(health_query
            .gates
            .contains(&SalesforceOperationGate::LiveOrgDescribe));
    }

    #[test]
    fn health_cloud_projection_metadata_is_catalog_sourced_and_describe_gated() {
        let candidates = salesforce_health_cloud_projection_candidates();

        assert_eq!(candidates.len(), 4);
        for candidate in candidates {
            assert_eq!(
                candidate.projection_status,
                SalesforceProjectionStatus::DescribeGated
            );
            assert_eq!(candidate.sensitivity, SalesforceDataSensitivity::PhiPii);
            assert!(candidate
                .source_refs
                .contains(&SALESFORCE_HEALTH_CLOUD_CATALOG_PATH));
            assert!(candidate.supported_calls.contains(&"describeSObjects()"));
            assert!(candidate.supported_calls.contains(&"getUpdated()"));
            assert!(candidate.supported_calls.contains(&"getDeleted()"));
            assert!(candidate.supported_calls.contains(&"query()"));
            assert!(candidate.source_url.ends_with("/262.0"));
            assert!(candidate
                .gates
                .contains(&SalesforceOperationGate::LiveOrgDescribe));
        }

        let coverage = candidates
            .iter()
            .find(|candidate| candidate.object == SalesforceObject::CoverageBenefitItem)
            .expect("coverage benefit metadata");
        assert_eq!(coverage.catalog_field_count, 23);
        assert!(coverage.sample_fields.contains(&"MemberId"));
        assert!(coverage.sample_fields.contains(&"SourceSystemModified"));

        let encounter = candidates
            .iter()
            .find(|candidate| candidate.object == SalesforceObject::ClinicalEncounter)
            .expect("clinical encounter metadata");
        assert!(encounter.sample_fields.contains(&"PatientId"));
    }

    #[test]
    fn metadata_operation_names_match_registry_constants() {
        let registry = SalesforceOperationRegistry::default();
        let metadata = salesforce_operation_metadata_catalog();

        assert_eq!(metadata.len(), registry.operation_names().count());
        for operation_metadata in metadata {
            let registered = registry
                .get(operation_metadata.name)
                .expect("metadata operation is registered");
            assert_eq!(operation_metadata.primitive, registered.primitive);
            assert_eq!(operation_metadata.object, registered.object);
            assert_eq!(operation_metadata.sensitivity, registered.sensitivity);
            assert_eq!(operation_metadata.gates, registered.gates);
        }
    }
}
