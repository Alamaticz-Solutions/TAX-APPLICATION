pub const AI_SEARCH_PROVIDER_KEY: &str = "ai_search";
pub const AI_SEARCH_API_FAMILY: &str = "PDS AI search named-operation contract";
pub const AI_SEARCH_RELEASE_BASIS: &str = "internal PDS AI search API absent - evidence-gated";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiSearchProviderDescriptor {
    pub provider_key: &'static str,
    pub data_source_name: String,
    pub api_family: &'static str,
    pub release_basis: &'static str,
}

impl AiSearchProviderDescriptor {
    pub fn provider_key(&self) -> &'static str {
        self.provider_key
    }
}

pub trait AiSearchProviderIdentity {
    fn ai_search_data_source_name(&self) -> &str;

    fn ai_search_provider_descriptor(&self) -> AiSearchProviderDescriptor {
        AiSearchProviderDescriptor {
            provider_key: AI_SEARCH_PROVIDER_KEY,
            data_source_name: self.ai_search_data_source_name().to_string(),
            api_family: AI_SEARCH_API_FAMILY,
            release_basis: AI_SEARCH_RELEASE_BASIS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestProvider;

    impl AiSearchProviderIdentity for TestProvider {
        fn ai_search_data_source_name(&self) -> &str {
            "ai_search_primary"
        }
    }

    #[test]
    fn descriptor_uses_ai_search_provider_key_and_evidence_basis() {
        let descriptor = TestProvider.ai_search_provider_descriptor();

        assert_eq!(descriptor.provider_key(), AI_SEARCH_PROVIDER_KEY);
        assert_eq!(descriptor.data_source_name, "ai_search_primary");
        assert_eq!(descriptor.api_family, AI_SEARCH_API_FAMILY);
        assert!(descriptor.release_basis.contains("evidence-gated"));
    }
}
