use crate::{
    AiSearchNamedOperationRequest, AiSearchOperationRegistry, AiSearchProviderError,
    AiSearchProviderIdentity,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiSearchProvider {
    data_source_name: String,
    registry: AiSearchOperationRegistry,
}

impl AiSearchProvider {
    pub fn new(data_source_name: impl Into<String>) -> Self {
        Self {
            data_source_name: data_source_name.into(),
            registry: AiSearchOperationRegistry::default(),
        }
    }

    pub fn with_registry(
        data_source_name: impl Into<String>,
        registry: AiSearchOperationRegistry,
    ) -> Self {
        Self {
            data_source_name: data_source_name.into(),
            registry,
        }
    }

    pub fn registry(&self) -> &AiSearchOperationRegistry {
        &self.registry
    }

    pub fn health_check_plan(&self) -> Result<(), AiSearchProviderError> {
        Ok(())
    }

    pub fn build_named_query_plan(
        &self,
        request: &AiSearchNamedOperationRequest,
    ) -> Result<(), AiSearchProviderError> {
        self.registry.ensure_named_query_is_not_executable(request)
    }

    pub fn build_named_mutation_plan(
        &self,
        request: &AiSearchNamedOperationRequest,
    ) -> Result<(), AiSearchProviderError> {
        self.registry
            .ensure_named_mutation_is_not_executable(request)
    }
}

impl AiSearchProviderIdentity for AiSearchProvider {
    fn ai_search_data_source_name(&self) -> &str {
        &self.data_source_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AiSearchProviderDescriptor, AI_SEARCH_PROVIDER_KEY, AI_SEARCH_SEARCH_OPERATION};

    #[test]
    fn provider_identity_matches_ai_search_descriptor() {
        let provider = AiSearchProvider::new("ai_search_primary");

        let descriptor: AiSearchProviderDescriptor = provider.ai_search_provider_descriptor();

        assert_eq!(descriptor.provider_key(), AI_SEARCH_PROVIDER_KEY);
        assert_eq!(descriptor.data_source_name, "ai_search_primary");
    }

    #[test]
    fn provider_keeps_named_queries_evidence_gated() {
        let provider = AiSearchProvider::new("ai_search_primary");
        let request = AiSearchNamedOperationRequest::new(AI_SEARCH_SEARCH_OPERATION);

        let error = provider.build_named_query_plan(&request).unwrap_err();

        assert!(matches!(
            error,
            AiSearchProviderError::UnsupportedOperation { reason, .. }
                if reason.contains("authenticated PDS AI search")
        ));
    }
}
