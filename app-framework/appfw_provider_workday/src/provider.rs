use appfw_saas_core::SaasRequestPlan;

use crate::{
    WorkdayNamedOperationRequest, WorkdayOperationRegistry, WorkdayProviderError,
    WorkdayProviderIdentity, WorkdayRequestPlan, WorkdayTenantBinding,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkdayProvider {
    data_source_name: String,
    registry: WorkdayOperationRegistry,
}

impl WorkdayProvider {
    pub fn new(data_source_name: impl Into<String>) -> Self {
        Self {
            data_source_name: data_source_name.into(),
            registry: WorkdayOperationRegistry::default(),
        }
    }

    pub fn with_registry(
        data_source_name: impl Into<String>,
        registry: WorkdayOperationRegistry,
    ) -> Self {
        Self {
            data_source_name: data_source_name.into(),
            registry,
        }
    }

    pub fn registry(&self) -> &WorkdayOperationRegistry {
        &self.registry
    }

    pub fn health_check_plan(&self) -> Result<(), WorkdayProviderError> {
        Ok(())
    }

    pub fn build_named_read_plan(
        &self,
        request: &WorkdayNamedOperationRequest,
    ) -> Result<WorkdayRequestPlan, WorkdayProviderError> {
        self.registry.build_request_plan(request)
    }

    pub fn build_named_saas_request_plan(
        &self,
        request: &WorkdayNamedOperationRequest,
        tenant_binding: &WorkdayTenantBinding,
    ) -> Result<SaasRequestPlan, WorkdayProviderError> {
        self.registry
            .build_request_plan(request)?
            .to_saas_request_plan(tenant_binding.human_resources_wws_path())
    }

    pub fn build_named_mutation_plan(&self) -> Result<(), WorkdayProviderError> {
        Err(WorkdayProviderError::UnsupportedMutation(
            "Wave 1 Workday skeleton exposes read-only WWS operation planning",
        ))
    }
}

impl WorkdayProviderIdentity for WorkdayProvider {
    fn external_api_data_source_name(&self) -> &str {
        &self.data_source_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        WorkdayObjectReference, WorkdayParameterValue, WorkdayProviderDescriptor,
        WorkdayReferenceType, WORKDAY_GET_WORKER_OPERATION, WORKDAY_HCM_GET_WORKERS_PAGE_OPERATION,
        WORKDAY_PROVIDER_KEY,
    };
    use appfw_saas_core::{SaasHttpMethod, SaasTransportProtocol};

    #[test]
    fn provider_identity_matches_workday_descriptor() {
        let provider = WorkdayProvider::new("workday_hcm");

        let descriptor: WorkdayProviderDescriptor = provider.external_api_provider_descriptor();

        assert_eq!(descriptor.provider_key(), WORKDAY_PROVIDER_KEY);
        assert_eq!(descriptor.data_source_name, "workday_hcm");
        assert_eq!(descriptor.service_name, "Human_ResourcesService");
    }

    #[test]
    fn provider_plans_named_reads_without_network() {
        let provider = WorkdayProvider::new("workday_hcm");
        let reference =
            WorkdayObjectReference::new(WorkdayReferenceType::EmployeeId, "E123").unwrap();
        let request = WorkdayNamedOperationRequest::new(WORKDAY_GET_WORKER_OPERATION)
            .with_parameter(
                "worker_reference",
                WorkdayParameterValue::Reference(reference),
            );

        let plan = provider
            .build_named_read_plan(&request)
            .expect("request plan");

        assert!(plan
            .soap_envelope
            .contains("<bsvc:Get_Workers_Request bsvc:version=\"v46.1\">"));
    }

    #[test]
    fn provider_plans_named_reads_as_shared_saas_request_from_server_bound_tenant() {
        let provider = WorkdayProvider::new("workday_hcm");
        let tenant_binding =
            WorkdayTenantBinding::new("wd2-impl-services1.workday.com", "acme").unwrap();
        let request = WorkdayNamedOperationRequest::new(WORKDAY_HCM_GET_WORKERS_PAGE_OPERATION);

        let plan = provider
            .build_named_saas_request_plan(&request, &tenant_binding)
            .expect("SaaS request plan");

        assert_eq!(plan.operation_name, WORKDAY_HCM_GET_WORKERS_PAGE_OPERATION);
        assert_eq!(plan.protocol, SaasTransportProtocol::SoapXml);
        assert_eq!(plan.method, SaasHttpMethod::Post);
        assert_eq!(plan.path, "/ccx/service/acme/Human_Resources/v46.1");
        assert_eq!(
            tenant_binding.origin(),
            "https://wd2-impl-services1.workday.com"
        );
        plan.validate().expect("shared plan validation");
    }

    #[test]
    fn provider_rejects_mutation_planning_for_now() {
        let provider = WorkdayProvider::new("workday_hcm");

        assert_eq!(
            provider.build_named_mutation_plan().unwrap_err(),
            WorkdayProviderError::UnsupportedMutation(
                "Wave 1 Workday skeleton exposes read-only WWS operation planning"
            )
        );
    }
}
