use crate::{
    AiSearchOperationGate, AiSearchOperationKind, AI_SEARCH_API_FAMILY, AI_SEARCH_EMBED_OPERATION,
    AI_SEARCH_RELEASE_BASIS, AI_SEARCH_SEARCH_OPERATION,
};

pub const AI_SEARCH_VENDOR_KEY: &str = "pds-ai-search";
pub const AI_SEARCH_API_SPEC_PACKAGE_ID: &str = "pds-ai-search-api-specs";
pub const AI_SEARCH_INFORMATION_PACKAGE_VERSION: &str = "absent-evidence-gated";
pub const AI_SEARCH_IMPLEMENTATION_METADATA_PATH: &str =
    "docs/runtime/ai-chat-search.md#held-spec-contract-ai-search-provider-graduation";

const SOURCE_REFS: &[&str] = &[AI_SEARCH_IMPLEMENTATION_METADATA_PATH];
const CONTRACT_GATES: &[AiSearchOperationGate] = &[
    AiSearchOperationGate::EnterpriseContractEvidence,
    AiSearchOperationGate::GatewayAuthEvidence,
    AiSearchOperationGate::SharedSaasHttpExecutor,
    AiSearchOperationGate::LiveCertification,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AiSearchApiSnapshotMetadata {
    pub vendor_key: &'static str,
    pub package_id: &'static str,
    pub information_package_version: &'static str,
    pub api_family: &'static str,
    pub release_basis: &'static str,
    pub source_refs: &'static [&'static str],
    pub blocking_gap: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiSearchProjectionStatus {
    EvidenceGated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiSearchMetadataDomain {
    Retrieval,
    Embedding,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AiSearchOperationMetadata {
    pub name: &'static str,
    pub domain: AiSearchMetadataDomain,
    pub kind: AiSearchOperationKind,
    pub projection_status: AiSearchProjectionStatus,
    pub gates: &'static [AiSearchOperationGate],
    pub source_refs: &'static [&'static str],
    pub notes: &'static str,
}

pub fn ai_search_api_snapshot_metadata() -> AiSearchApiSnapshotMetadata {
    AiSearchApiSnapshotMetadata {
        vendor_key: AI_SEARCH_VENDOR_KEY,
        package_id: AI_SEARCH_API_SPEC_PACKAGE_ID,
        information_package_version: AI_SEARCH_INFORMATION_PACKAGE_VERSION,
        api_family: AI_SEARCH_API_FAMILY,
        release_basis: AI_SEARCH_RELEASE_BASIS,
        source_refs: SOURCE_REFS,
        blocking_gap:
            "Need authenticated PDS AI search API spec/export plus recorded-live fixtures before request planning or execution.",
    }
}

pub fn ai_search_operation_metadata_catalog() -> &'static [AiSearchOperationMetadata] {
    OPERATION_METADATA
}

pub fn ai_search_operation_metadata(name: &str) -> Option<&'static AiSearchOperationMetadata> {
    OPERATION_METADATA
        .iter()
        .find(|metadata| metadata.name == name)
}

const OPERATION_METADATA: &[AiSearchOperationMetadata] = &[
    AiSearchOperationMetadata {
        name: AI_SEARCH_SEARCH_OPERATION,
        domain: AiSearchMetadataDomain::Retrieval,
        kind: AiSearchOperationKind::Search,
        projection_status: AiSearchProjectionStatus::EvidenceGated,
        gates: CONTRACT_GATES,
        source_refs: SOURCE_REFS,
        notes: "Search is a named query contract only; the chat orchestrator re-resolves returned pointers under the signed-in user's policy context.",
    },
    AiSearchOperationMetadata {
        name: AI_SEARCH_EMBED_OPERATION,
        domain: AiSearchMetadataDomain::Embedding,
        kind: AiSearchOperationKind::Embed,
        projection_status: AiSearchProjectionStatus::EvidenceGated,
        gates: CONTRACT_GATES,
        source_refs: SOURCE_REFS,
        notes: "Embedding remains evidence-gated until the approved provider contract defines request shape, limits, redaction, and provenance.",
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AiSearchOperationRegistry;

    #[test]
    fn api_snapshot_metadata_records_internal_api_gap() {
        let metadata = ai_search_api_snapshot_metadata();

        assert_eq!(metadata.vendor_key, AI_SEARCH_VENDOR_KEY);
        assert!(metadata.release_basis.contains("evidence-gated"));
        assert!(metadata
            .blocking_gap
            .contains("authenticated PDS AI search"));
        assert!(metadata
            .source_refs
            .contains(&AI_SEARCH_IMPLEMENTATION_METADATA_PATH));
    }

    #[test]
    fn metadata_operation_names_match_registry_constants() {
        let registry = AiSearchOperationRegistry::default();
        let registry_names = registry.operation_names().collect::<Vec<_>>();
        let mut metadata_names = ai_search_operation_metadata_catalog()
            .iter()
            .map(|metadata| metadata.name)
            .collect::<Vec<_>>();
        metadata_names.sort_unstable();

        assert_eq!(metadata_names, registry_names);
    }
}
