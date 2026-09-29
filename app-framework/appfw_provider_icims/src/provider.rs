use crate::{
    IcimsNamedOperationRequest, IcimsOperationRegistry, IcimsProviderError, IcimsProviderIdentity,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IcimsProvider {
    data_source_name: String,
    registry: IcimsOperationRegistry,
}

impl IcimsProvider {
    pub fn new(data_source_name: impl Into<String>) -> Self {
        Self {
            data_source_name: data_source_name.into(),
            registry: IcimsOperationRegistry::default(),
        }
    }

    pub fn with_registry(
        data_source_name: impl Into<String>,
        registry: IcimsOperationRegistry,
    ) -> Self {
        Self {
            data_source_name: data_source_name.into(),
            registry,
        }
    }

    pub fn registry(&self) -> &IcimsOperationRegistry {
        &self.registry
    }

    pub fn health_check_plan(&self) -> Result<(), IcimsProviderError> {
        Ok(())
    }

    pub fn build_named_read_plan(
        &self,
        request: &IcimsNamedOperationRequest,
    ) -> Result<(), IcimsProviderError> {
        self.registry.ensure_operation_is_not_executable(request)
    }

    pub fn build_named_mutation_plan(&self) -> Result<(), IcimsProviderError> {
        Err(IcimsProviderError::UnsupportedMutation(
            "iCIMS skeleton requires authenticated Developer Community or tenant docs before reads or writes execute",
        ))
    }
}

impl IcimsProviderIdentity for IcimsProvider {
    fn external_api_data_source_name(&self) -> &str {
        &self.data_source_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        IcimsProviderDescriptor, ICIMS_API_FAMILY, ICIMS_PROVIDER_KEY,
        ICIMS_RECRUITING_QUERY_CANDIDATES_INCREMENTAL_OPERATION,
    };

    #[test]
    fn provider_identity_matches_icims_descriptor() {
        let provider = IcimsProvider::new("icims_primary");

        let descriptor: IcimsProviderDescriptor = provider.external_api_provider_descriptor();

        assert_eq!(descriptor.provider_key(), ICIMS_PROVIDER_KEY);
        assert_eq!(descriptor.data_source_name, "icims_primary");
        assert_eq!(descriptor.api_family, ICIMS_API_FAMILY);
    }

    #[test]
    fn provider_keeps_named_reads_evidence_gated() {
        let provider = IcimsProvider::new("icims_primary");
        let request = IcimsNamedOperationRequest::new(
            ICIMS_RECRUITING_QUERY_CANDIDATES_INCREMENTAL_OPERATION,
        );

        let error = provider.build_named_read_plan(&request).unwrap_err();

        assert!(matches!(
            error,
            IcimsProviderError::UnsupportedOperation { reason, .. }
                if reason.contains("authenticated iCIMS")
        ));
    }

    #[test]
    fn provider_rejects_mutation_planning_for_now() {
        let provider = IcimsProvider::new("icims_primary");

        assert_eq!(
            provider.build_named_mutation_plan().unwrap_err(),
            IcimsProviderError::UnsupportedMutation(
                "iCIMS skeleton requires authenticated Developer Community or tenant docs before reads or writes execute"
            )
        );
    }
}
