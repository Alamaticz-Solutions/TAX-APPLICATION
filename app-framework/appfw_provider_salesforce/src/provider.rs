use appfw_saas_core::SaasRequestPlan;

use crate::{
    SalesforceNamedOperationRequest, SalesforceOperationPlan, SalesforceOperationRegistry,
    SalesforceProviderError, SalesforceProviderIdentity, SalesforceQueryPlan,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SalesforceProvider {
    data_source_name: String,
    registry: SalesforceOperationRegistry,
}

impl SalesforceProvider {
    pub fn new(data_source_name: impl Into<String>) -> Self {
        Self {
            data_source_name: data_source_name.into(),
            registry: SalesforceOperationRegistry::default(),
        }
    }

    pub fn with_registry(
        data_source_name: impl Into<String>,
        registry: SalesforceOperationRegistry,
    ) -> Self {
        Self {
            data_source_name: data_source_name.into(),
            registry,
        }
    }

    pub fn registry(&self) -> &SalesforceOperationRegistry {
        &self.registry
    }

    pub fn health_check_plan(&self) -> Result<(), SalesforceProviderError> {
        Ok(())
    }

    pub fn build_named_query_plan(
        &self,
        request: &SalesforceNamedOperationRequest,
    ) -> Result<SalesforceQueryPlan, SalesforceProviderError> {
        self.registry.build_query_plan(request)
    }

    pub fn build_named_operation_plan(
        &self,
        request: &SalesforceNamedOperationRequest,
    ) -> Result<SalesforceOperationPlan, SalesforceProviderError> {
        self.registry.build_operation_plan(request)
    }

    pub fn build_named_saas_request_plan(
        &self,
        request: &SalesforceNamedOperationRequest,
    ) -> Result<SaasRequestPlan, SalesforceProviderError> {
        self.registry.build_saas_request_plan(request)
    }

    pub fn build_named_mutation_plan(&self) -> Result<(), SalesforceProviderError> {
        Err(SalesforceProviderError::UnsupportedMutation(
            "Wave 1 Salesforce skeleton exposes read operation planning only",
        ))
    }
}

impl SalesforceProviderIdentity for SalesforceProvider {
    fn external_api_data_source_name(&self) -> &str {
        &self.data_source_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        SalesforceParameterValue, SalesforceProviderDescriptor, ACCOUNT_BY_ID_OPERATION,
        SALESFORCE_PROVIDER_KEY,
    };
    use appfw_saas_core::{SaasHttpMethod, SaasTransportProtocol};

    #[test]
    fn provider_identity_matches_salesforce_descriptor() {
        let provider = SalesforceProvider::new("salesforce_primary");

        let descriptor: SalesforceProviderDescriptor = provider.external_api_provider_descriptor();

        assert_eq!(descriptor.provider_key(), SALESFORCE_PROVIDER_KEY);
        assert_eq!(descriptor.data_source_name, "salesforce_primary");
    }

    #[test]
    fn provider_plans_named_reads_without_network() {
        let provider = SalesforceProvider::new("salesforce_primary");
        let request = SalesforceNamedOperationRequest::new(ACCOUNT_BY_ID_OPERATION).with_parameter(
            "id",
            SalesforceParameterValue::String("001000000000001AAA".to_string()),
        );

        let plan = provider
            .build_named_query_plan(&request)
            .expect("query plan");

        assert!(plan.soql.starts_with("SELECT Id, Name"));
    }

    #[test]
    fn provider_plans_named_reads_as_shared_saas_request_without_network() {
        let provider = SalesforceProvider::new("salesforce_primary");
        let request = SalesforceNamedOperationRequest::new(ACCOUNT_BY_ID_OPERATION).with_parameter(
            "id",
            SalesforceParameterValue::String("001000000000001AAA".to_string()),
        );

        let plan = provider
            .build_named_saas_request_plan(&request)
            .expect("SaaS request plan");

        assert_eq!(plan.operation_name, ACCOUNT_BY_ID_OPERATION);
        assert_eq!(plan.protocol, SaasTransportProtocol::RestJson);
        assert_eq!(plan.method, SaasHttpMethod::Get);
        assert_eq!(plan.path, "/services/data/v67.0/query");
        assert_eq!(plan.query.keys().collect::<Vec<_>>(), vec!["q"]);
        assert_eq!(plan.caps.max_rows, 1);
    }

    #[test]
    fn provider_rejects_mutation_planning_for_now() {
        let provider = SalesforceProvider::new("salesforce_primary");

        assert_eq!(
            provider.build_named_mutation_plan().unwrap_err(),
            SalesforceProviderError::UnsupportedMutation(
                "Wave 1 Salesforce skeleton exposes read operation planning only"
            )
        );
    }
}
