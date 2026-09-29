use appfw_saas_core::SaasRequestPlan;

use crate::{
    OracleFinancialsNamedOperationRequest, OracleFinancialsOperationRegistry,
    OracleFinancialsProviderError, OracleFinancialsProviderIdentity,
    OracleFinancialsRestRequestPlan, OracleFinancialsTenantBinding,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OracleFinancialsProvider {
    data_source_name: String,
    registry: OracleFinancialsOperationRegistry,
}

impl OracleFinancialsProvider {
    pub fn new(data_source_name: impl Into<String>) -> Self {
        Self {
            data_source_name: data_source_name.into(),
            registry: OracleFinancialsOperationRegistry::default(),
        }
    }

    pub fn with_registry(
        data_source_name: impl Into<String>,
        registry: OracleFinancialsOperationRegistry,
    ) -> Self {
        Self {
            data_source_name: data_source_name.into(),
            registry,
        }
    }

    pub fn registry(&self) -> &OracleFinancialsOperationRegistry {
        &self.registry
    }

    pub fn health_check_plan(&self) -> Result<(), OracleFinancialsProviderError> {
        Ok(())
    }

    pub fn build_named_read_plan(
        &self,
        request: &OracleFinancialsNamedOperationRequest,
        tenant_binding: &OracleFinancialsTenantBinding,
    ) -> Result<OracleFinancialsRestRequestPlan, OracleFinancialsProviderError> {
        self.registry.build_operation_plan(request, tenant_binding)
    }

    pub fn build_named_saas_request_plan(
        &self,
        request: &OracleFinancialsNamedOperationRequest,
        tenant_binding: &OracleFinancialsTenantBinding,
    ) -> Result<SaasRequestPlan, OracleFinancialsProviderError> {
        self.registry
            .build_saas_request_plan(request, tenant_binding)
    }

    pub fn build_named_mutation_plan(&self) -> Result<(), OracleFinancialsProviderError> {
        Err(OracleFinancialsProviderError::UnsupportedMutation(
            "Wave 1 Oracle Financials skeleton exposes read operation planning only",
        ))
    }
}

impl OracleFinancialsProviderIdentity for OracleFinancialsProvider {
    fn external_api_data_source_name(&self) -> &str {
        &self.data_source_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        OracleFinancialsProviderDescriptor,
        ORACLE_FINANCIALS_ACCOUNTING_PERIOD_STATUS_LOV_LIST_OPERATION,
        ORACLE_FINANCIALS_API_FAMILY, ORACLE_FINANCIALS_PROVIDER_KEY,
    };
    use appfw_saas_core::{SaasHttpMethod, SaasTransportProtocol};

    fn tenant() -> OracleFinancialsTenantBinding {
        OracleFinancialsTenantBinding::new(
            "https://tenant.fa.us2.oraclecloud.com",
            "26B",
            "ledger:vision-operations",
        )
        .expect("tenant binding")
    }

    #[test]
    fn provider_identity_matches_oracle_descriptor() {
        let provider = OracleFinancialsProvider::new("oracle_financials_primary");

        let descriptor: OracleFinancialsProviderDescriptor =
            provider.external_api_provider_descriptor();

        assert_eq!(descriptor.provider_key(), ORACLE_FINANCIALS_PROVIDER_KEY);
        assert_eq!(descriptor.data_source_name, "oracle_financials_primary");
        assert_eq!(descriptor.api_family, ORACLE_FINANCIALS_API_FAMILY);
    }

    #[test]
    fn provider_plans_named_reads_as_shared_saas_request_without_network() {
        let provider = OracleFinancialsProvider::new("oracle_financials_primary");
        let request = OracleFinancialsNamedOperationRequest::new(
            ORACLE_FINANCIALS_ACCOUNTING_PERIOD_STATUS_LOV_LIST_OPERATION,
        );

        let plan = provider
            .build_named_saas_request_plan(&request, &tenant())
            .expect("SaaS request plan");

        assert_eq!(
            plan.operation_name,
            ORACLE_FINANCIALS_ACCOUNTING_PERIOD_STATUS_LOV_LIST_OPERATION
        );
        assert_eq!(plan.protocol, SaasTransportProtocol::RestJson);
        assert_eq!(plan.method, SaasHttpMethod::Get);
        assert_eq!(
            plan.path,
            "/fscmRestApi/resources/11.13.18.05/accountingPeriodStatusLOV"
        );
        assert_eq!(plan.query.get("onlyData").map(String::as_str), Some("true"));
        assert_eq!(plan.caps.max_rows, 100);
    }

    #[test]
    fn provider_rejects_mutation_planning_for_now() {
        let provider = OracleFinancialsProvider::new("oracle_financials_primary");

        assert_eq!(
            provider.build_named_mutation_plan().unwrap_err(),
            OracleFinancialsProviderError::UnsupportedMutation(
                "Wave 1 Oracle Financials skeleton exposes read operation planning only"
            )
        );
    }
}
