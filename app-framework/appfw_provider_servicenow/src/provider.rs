use crate::{
    ServiceNowNamedOperationRequest, ServiceNowOperationRegistry, ServiceNowProviderError,
    ServiceNowProviderIdentity,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceNowProvider {
    data_source_name: String,
    registry: ServiceNowOperationRegistry,
}

impl ServiceNowProvider {
    pub fn new(data_source_name: impl Into<String>) -> Self {
        Self {
            data_source_name: data_source_name.into(),
            registry: ServiceNowOperationRegistry::default(),
        }
    }

    pub fn with_registry(
        data_source_name: impl Into<String>,
        registry: ServiceNowOperationRegistry,
    ) -> Self {
        Self {
            data_source_name: data_source_name.into(),
            registry,
        }
    }

    pub fn registry(&self) -> &ServiceNowOperationRegistry {
        &self.registry
    }

    pub fn health_check_plan(&self) -> Result<(), ServiceNowProviderError> {
        Ok(())
    }

    pub fn build_named_read_plan(
        &self,
        request: &ServiceNowNamedOperationRequest,
    ) -> Result<(), ServiceNowProviderError> {
        self.registry.ensure_named_read_is_not_executable(request)
    }

    pub fn build_named_mutation_plan(
        &self,
        request: &ServiceNowNamedOperationRequest,
    ) -> Result<(), ServiceNowProviderError> {
        self.registry
            .ensure_named_mutation_is_not_executable(request)
    }
}

impl ServiceNowProviderIdentity for ServiceNowProvider {
    fn external_api_data_source_name(&self) -> &str {
        &self.data_source_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ServiceNowProviderDescriptor, SERVICENOW_API_FAMILY, SERVICENOW_INCIDENT_CREATE_OPERATION,
        SERVICENOW_PROVIDER_KEY, SERVICENOW_TABLE_QUERY_INCREMENTAL_OPERATION,
    };

    #[test]
    fn provider_identity_matches_servicenow_descriptor() {
        let provider = ServiceNowProvider::new("servicenow_primary");

        let descriptor: ServiceNowProviderDescriptor = provider.external_api_provider_descriptor();

        assert_eq!(descriptor.provider_key(), SERVICENOW_PROVIDER_KEY);
        assert_eq!(descriptor.data_source_name, "servicenow_primary");
        assert_eq!(descriptor.api_family, SERVICENOW_API_FAMILY);
    }

    #[test]
    fn provider_keeps_named_reads_evidence_gated() {
        let provider = ServiceNowProvider::new("servicenow_primary");
        let request =
            ServiceNowNamedOperationRequest::new(SERVICENOW_TABLE_QUERY_INCREMENTAL_OPERATION);

        let error = provider.build_named_read_plan(&request).unwrap_err();

        assert!(matches!(
            error,
            ServiceNowProviderError::UnsupportedOperation { reason, .. }
                if reason.contains("authenticated ServiceNow")
        ));
    }

    #[test]
    fn provider_rejects_governed_write_mutation_planning_without_g1_evidence() {
        let provider = ServiceNowProvider::new("servicenow_primary");
        let request = ServiceNowNamedOperationRequest::new(SERVICENOW_INCIDENT_CREATE_OPERATION);

        assert!(matches!(
            provider.build_named_mutation_plan(&request).unwrap_err(),
            ServiceNowProviderError::UnsupportedOperation { reason, .. }
                if reason.contains("delegated actor context")
        ));
    }
}
