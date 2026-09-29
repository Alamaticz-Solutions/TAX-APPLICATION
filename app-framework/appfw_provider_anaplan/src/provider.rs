use appfw_saas_core::SaasRequestPlan;

use crate::{
    AnaplanNamedOperationRequest, AnaplanOperationRegistry, AnaplanProviderError,
    AnaplanProviderIdentity, AnaplanRestRequestPlan, AnaplanTenantBinding,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnaplanProvider {
    data_source_name: String,
    registry: AnaplanOperationRegistry,
}

impl AnaplanProvider {
    pub fn new(data_source_name: impl Into<String>) -> Self {
        Self {
            data_source_name: data_source_name.into(),
            registry: AnaplanOperationRegistry::default(),
        }
    }

    pub fn with_registry(
        data_source_name: impl Into<String>,
        registry: AnaplanOperationRegistry,
    ) -> Self {
        Self {
            data_source_name: data_source_name.into(),
            registry,
        }
    }

    pub fn registry(&self) -> &AnaplanOperationRegistry {
        &self.registry
    }

    pub fn health_check_plan(&self) -> Result<(), AnaplanProviderError> {
        Ok(())
    }

    pub fn build_named_read_plan(
        &self,
        request: &AnaplanNamedOperationRequest,
        tenant_binding: &AnaplanTenantBinding,
    ) -> Result<AnaplanRestRequestPlan, AnaplanProviderError> {
        self.registry.build_operation_plan(request, tenant_binding)
    }

    pub fn build_named_saas_request_plan(
        &self,
        request: &AnaplanNamedOperationRequest,
        tenant_binding: &AnaplanTenantBinding,
    ) -> Result<SaasRequestPlan, AnaplanProviderError> {
        self.registry
            .build_saas_request_plan(request, tenant_binding)
    }

    pub fn build_named_mutation_plan(&self) -> Result<(), AnaplanProviderError> {
        Err(AnaplanProviderError::UnsupportedMutation(
            "Wave 1 Anaplan skeleton exposes read operation planning only",
        ))
    }
}

impl AnaplanProviderIdentity for AnaplanProvider {
    fn external_api_data_source_name(&self) -> &str {
        &self.data_source_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AnaplanProviderDescriptor, ANAPLAN_FILES_LIST_OPERATION, ANAPLAN_PROVIDER_KEY};
    use appfw_saas_core::{SaasHttpMethod, SaasTransportProtocol};

    fn tenant() -> AnaplanTenantBinding {
        AnaplanTenantBinding::new(
            "https://api.anaplan.com",
            "https://auth.anaplan.com",
            "workspace123",
            "model456",
        )
        .expect("tenant binding")
    }

    #[test]
    fn provider_identity_matches_anaplan_descriptor() {
        let provider = AnaplanProvider::new("anaplan_primary");

        let descriptor: AnaplanProviderDescriptor = provider.external_api_provider_descriptor();

        assert_eq!(descriptor.provider_key(), ANAPLAN_PROVIDER_KEY);
        assert_eq!(descriptor.data_source_name, "anaplan_primary");
        assert_eq!(descriptor.api_family, "Integration API v2");
    }

    #[test]
    fn provider_plans_named_reads_as_shared_saas_request_without_network() {
        let provider = AnaplanProvider::new("anaplan_primary");
        let request = AnaplanNamedOperationRequest::new(ANAPLAN_FILES_LIST_OPERATION);

        let plan = provider
            .build_named_saas_request_plan(&request, &tenant())
            .expect("SaaS request plan");

        assert_eq!(plan.operation_name, ANAPLAN_FILES_LIST_OPERATION);
        assert_eq!(plan.protocol, SaasTransportProtocol::RestJson);
        assert_eq!(plan.method, SaasHttpMethod::Get);
        assert_eq!(
            plan.path,
            "/2/0/workspaces/workspace123/models/model456/files"
        );
        assert_eq!(
            plan.query.keys().collect::<Vec<_>>(),
            vec!["limit", "offset", "sort"]
        );
        assert_eq!(plan.caps.max_rows, 100);
    }

    #[test]
    fn provider_rejects_mutation_planning_for_now() {
        let provider = AnaplanProvider::new("anaplan_primary");

        assert_eq!(
            provider.build_named_mutation_plan().unwrap_err(),
            AnaplanProviderError::UnsupportedMutation(
                "Wave 1 Anaplan skeleton exposes read operation planning only"
            )
        );
    }
}
