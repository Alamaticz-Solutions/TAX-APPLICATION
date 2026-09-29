use std::collections::BTreeMap;

use appfw_saas_core::{
    OffsetLimitContinuation, SaasHttpMethod, SaasRequestPlan, SaasResponseCaps,
    SaasTransportProtocol,
};

use crate::{OracleFinancialsProviderError, OracleFinancialsTenantBinding};

pub const ORACLE_FINANCIALS_REST_RESOURCES_BASE_PATH: &str = "/fscmRestApi/resources/11.13.18.05";
pub const ORACLE_FINANCIALS_ONLY_DATA_QUERY: &str = "onlyData";
pub const ORACLE_FINANCIALS_LINKS_QUERY: &str = "links";
pub const ORACLE_FINANCIALS_TOTAL_RESULTS_QUERY: &str = "totalResults";
pub const ORACLE_FINANCIALS_LIMIT_QUERY: &str = "limit";
pub const ORACLE_FINANCIALS_OFFSET_QUERY: &str = "offset";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OracleFinancialsQueryLimits {
    pub max_results: u32,
    pub timeout_ms: u64,
    pub offset: u64,
}

impl OracleFinancialsQueryLimits {
    pub const DEFAULT_MAX_RESULTS: u32 = 100;
    pub const DEFAULT_TIMEOUT_MS: u64 = 3_000;
    pub const MAX_PAGE_LIMIT: u32 = 500;

    pub fn effective_max_results(self) -> Result<u32, OracleFinancialsProviderError> {
        if self.max_results == 0 {
            return Err(OracleFinancialsProviderError::InvalidParameter {
                name: "limits.max_results",
                reason: "must be greater than zero",
            });
        }

        Ok(self.max_results.min(Self::MAX_PAGE_LIMIT))
    }

    pub fn default_page(self) -> Result<OffsetLimitContinuation, OracleFinancialsProviderError> {
        Ok(OffsetLimitContinuation::new(
            self.offset,
            self.effective_max_results()?,
        ))
    }
}

impl Default for OracleFinancialsQueryLimits {
    fn default() -> Self {
        Self {
            max_results: Self::DEFAULT_MAX_RESULTS,
            timeout_ms: Self::DEFAULT_TIMEOUT_MS,
            offset: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OracleFinancialsParameterValue {
    OffsetPage(OffsetLimitContinuation),
    String(String),
}

impl OracleFinancialsParameterValue {
    pub fn as_str(&self) -> &str {
        match self {
            Self::String(value) => value.as_str(),
            Self::OffsetPage(_) => "",
        }
    }

    pub fn as_offset_page(&self) -> Option<&OffsetLimitContinuation> {
        match self {
            Self::OffsetPage(value) => Some(value),
            Self::String(_) => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OracleFinancialsNamedOperationRequest {
    pub name: String,
    pub parameters: BTreeMap<String, OracleFinancialsParameterValue>,
    pub limits: OracleFinancialsQueryLimits,
}

impl OracleFinancialsNamedOperationRequest {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            parameters: BTreeMap::new(),
            limits: OracleFinancialsQueryLimits::default(),
        }
    }

    pub fn with_parameter(
        mut self,
        name: impl Into<String>,
        value: OracleFinancialsParameterValue,
    ) -> Self {
        self.parameters.insert(name.into(), value);
        self
    }

    pub fn with_limits(mut self, limits: OracleFinancialsQueryLimits) -> Self {
        self.limits = limits;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OracleFinancialsOperationParameter {
    pub name: &'static str,
    pub required: bool,
    pub description: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OracleFinancialsRestMethod {
    Get,
    Post,
}

impl OracleFinancialsRestMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
        }
    }

    pub fn to_saas_http_method(self) -> SaasHttpMethod {
        match self {
            Self::Get => SaasHttpMethod::Get,
            Self::Post => SaasHttpMethod::Post,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OracleFinancialsOperationPrimitive {
    OpenApiDescribeResource,
    CollectionQuery,
    FetchById,
    ListLov,
    SubmitErpIntegration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OracleFinancialsOperationAvailability {
    Executable,
    PlannedUnsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OracleFinancialsOperationGate {
    TenantHostAndDataSecurityScope,
    TenantOperationAllowList,
    TenantAuthCertification,
    GovernedWriteReview,
    LiveCertification,
    RuntimeExecutor,
}

impl OracleFinancialsOperationGate {
    pub fn reason(self) -> &'static str {
        match self {
            Self::TenantHostAndDataSecurityScope => {
                "requires tenant-approved Oracle host and data-security scope"
            }
            Self::TenantOperationAllowList => {
                "requires selected Oracle OpenAPI operation allow-list before execution"
            }
            Self::TenantAuthCertification => {
                "requires tenant-supported Oracle Fusion authentication evidence"
            }
            Self::GovernedWriteReview => {
                "requires governed write safety, idempotency, and audit review"
            }
            Self::LiveCertification => "requires live connector certification evidence",
            Self::RuntimeExecutor => "requires runtime HTTP executor integration",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OracleFinancialsDataSensitivity {
    Operational,
    FinancialConfidential,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OracleFinancialsRestRequestPlan {
    pub operation_name: String,
    pub primitive: OracleFinancialsOperationPrimitive,
    pub method: OracleFinancialsRestMethod,
    pub path: String,
    pub query_parameters: BTreeMap<String, String>,
    pub headers: BTreeMap<String, String>,
    pub timeout_ms: u64,
    pub caps: SaasResponseCaps,
}

impl OracleFinancialsRestRequestPlan {
    pub fn new(
        operation_name: impl Into<String>,
        primitive: OracleFinancialsOperationPrimitive,
        method: OracleFinancialsRestMethod,
        path: impl Into<String>,
        timeout_ms: u64,
    ) -> Self {
        Self {
            operation_name: operation_name.into(),
            primitive,
            method,
            path: path.into(),
            query_parameters: BTreeMap::new(),
            headers: BTreeMap::new(),
            timeout_ms,
            caps: oracle_financials_response_caps(OracleFinancialsQueryLimits::DEFAULT_MAX_RESULTS),
        }
    }

    pub fn with_query_parameter(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.query_parameters.insert(name.into(), value.into());
        self
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    pub fn with_caps(mut self, caps: SaasResponseCaps) -> Self {
        self.caps = caps;
        self
    }

    pub fn with_oracle_headers(mut self, tenant_binding: &OracleFinancialsTenantBinding) -> Self {
        self.headers.insert(
            ORACLE_FINANCIALS_REST_FRAMEWORK_VERSION_HEADER.to_string(),
            tenant_binding.rest_framework_version().to_string(),
        );

        if let Some(metadata_context) = tenant_binding.metadata_context() {
            self.headers.insert(
                ORACLE_FINANCIALS_METADATA_CONTEXT_HEADER.to_string(),
                metadata_context.to_string(),
            );
        }

        self
    }

    pub fn to_saas_request_plan(&self) -> Result<SaasRequestPlan, String> {
        let mut plan = SaasRequestPlan::new(
            self.operation_name.clone(),
            SaasTransportProtocol::RestJson,
            self.method.to_saas_http_method(),
            self.path.clone(),
        )
        .with_timeout_ms(self.timeout_ms)
        .with_caps(self.caps);

        for (name, value) in &self.query_parameters {
            plan = plan.with_query(name.clone(), value.clone());
        }

        for (name, value) in &self.headers {
            plan = plan.with_header(name.clone(), value.clone());
        }

        plan.validate()?;
        Ok(plan)
    }
}

pub fn oracle_financials_response_caps(max_rows: u32) -> SaasResponseCaps {
    SaasResponseCaps {
        max_body_bytes: SaasResponseCaps::DEFAULT_MAX_BODY_BYTES,
        max_rows: max_rows.max(1),
    }
}

pub const ORACLE_FINANCIALS_REST_FRAMEWORK_VERSION_HEADER: &str = "REST-Framework-Version";
pub const ORACLE_FINANCIALS_METADATA_CONTEXT_HEADER: &str = "Metadata-Context";

#[cfg(test)]
mod tests {
    use super::*;
    use appfw_saas_core::SaasTransportProtocol;

    #[test]
    fn resources_base_path_uses_oracle_fusion_rest_resource_root() {
        assert_eq!(
            ORACLE_FINANCIALS_REST_RESOURCES_BASE_PATH,
            "/fscmRestApi/resources/11.13.18.05"
        );
    }

    #[test]
    fn rest_plan_conversion_preserves_fixed_headers_and_offset_limit_parameters() {
        let plan = OracleFinancialsRestRequestPlan::new(
            "oracle.financials.accounting_period_status_lov.list",
            OracleFinancialsOperationPrimitive::ListLov,
            OracleFinancialsRestMethod::Get,
            "/fscmRestApi/resources/11.13.18.05/accountingPeriodStatusLOV",
            OracleFinancialsQueryLimits::DEFAULT_TIMEOUT_MS,
        )
        .with_header(ORACLE_FINANCIALS_REST_FRAMEWORK_VERSION_HEADER, "26B")
        .with_query_parameter("onlyData", "true")
        .with_query_parameter("limit", "25")
        .with_query_parameter("offset", "50")
        .with_caps(oracle_financials_response_caps(25));

        let saas_plan = plan.to_saas_request_plan().expect("valid SaaS plan");

        assert_eq!(saas_plan.protocol, SaasTransportProtocol::RestJson);
        assert_eq!(saas_plan.method, SaasHttpMethod::Get);
        assert_eq!(
            saas_plan.path,
            "/fscmRestApi/resources/11.13.18.05/accountingPeriodStatusLOV"
        );
        assert_eq!(
            saas_plan.query.get("onlyData").map(String::as_str),
            Some("true")
        );
        assert_eq!(saas_plan.query.get("limit").map(String::as_str), Some("25"));
        assert_eq!(
            saas_plan
                .headers
                .get(ORACLE_FINANCIALS_REST_FRAMEWORK_VERSION_HEADER)
                .map(String::as_str),
            Some("26B")
        );
        assert_eq!(saas_plan.caps.max_rows, 25);
    }

    #[test]
    fn rest_plan_conversion_rejects_raw_absolute_urls() {
        let plan = OracleFinancialsRestRequestPlan::new(
            "oracle.financials.collection_query",
            OracleFinancialsOperationPrimitive::CollectionQuery,
            OracleFinancialsRestMethod::Get,
            "https://tenant.fa.us2.oraclecloud.com/fscmRestApi/resources/11.13.18.05/payablesInvoices",
            OracleFinancialsQueryLimits::DEFAULT_TIMEOUT_MS,
        );

        let error = plan.to_saas_request_plan().unwrap_err();

        assert!(error.contains("path"));
    }
}
