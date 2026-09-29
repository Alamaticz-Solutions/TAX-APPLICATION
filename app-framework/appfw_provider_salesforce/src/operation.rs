use std::collections::BTreeMap;

use appfw_saas_core::{SaasHttpMethod, SaasRequestPlan, SaasResponseCaps, SaasTransportProtocol};

use crate::{
    response::SalesforceNextRecordsUrl, SalesforceObject, SalesforceProviderError,
    SalesforceWatermarkRequest,
};

pub const SALESFORCE_REST_API_VERSION: &str = "v67.0";
pub const SALESFORCE_REST_BASE_PATH: &str = "/services/data/v67.0";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SalesforceQueryLimits {
    pub max_results: u32,
    pub timeout_ms: u64,
}

impl SalesforceQueryLimits {
    pub const DEFAULT_MAX_RESULTS: u32 = 250;
    pub const DEFAULT_TIMEOUT_MS: u64 = 2_000;
    pub const SOQL_MAX_LIMIT: u32 = 2_000;

    pub fn effective_max_results(self) -> Result<u32, SalesforceProviderError> {
        if self.max_results == 0 {
            return Err(SalesforceProviderError::InvalidParameter {
                name: "limits.max_results",
                reason: "must be greater than zero",
            });
        }

        Ok(self.max_results.min(Self::SOQL_MAX_LIMIT))
    }
}

impl Default for SalesforceQueryLimits {
    fn default() -> Self {
        Self {
            max_results: Self::DEFAULT_MAX_RESULTS,
            timeout_ms: Self::DEFAULT_TIMEOUT_MS,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SalesforceParameterValue {
    Bool(bool),
    Number(u32),
    String(String),
    StringList(Vec<String>),
}

impl SalesforceParameterValue {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            Self::Number(_) | Self::String(_) | Self::StringList(_) => None,
        }
    }

    pub fn as_number(&self) -> Option<u32> {
        match self {
            Self::Number(value) => Some(*value),
            Self::Bool(_) | Self::String(_) | Self::StringList(_) => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value.as_str()),
            Self::Bool(_) | Self::Number(_) | Self::StringList(_) => None,
        }
    }

    pub fn as_string_list(&self) -> Option<&[String]> {
        match self {
            Self::StringList(value) => Some(value.as_slice()),
            Self::Bool(_) | Self::Number(_) | Self::String(_) => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SalesforceNamedOperationRequest {
    pub name: String,
    pub parameters: BTreeMap<String, SalesforceParameterValue>,
    pub limits: SalesforceQueryLimits,
}

impl SalesforceNamedOperationRequest {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            parameters: BTreeMap::new(),
            limits: SalesforceQueryLimits::default(),
        }
    }

    pub fn with_parameter(
        mut self,
        name: impl Into<String>,
        value: SalesforceParameterValue,
    ) -> Self {
        self.parameters.insert(name.into(), value);
        self
    }

    pub fn with_limits(mut self, limits: SalesforceQueryLimits) -> Self {
        self.limits = limits;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SalesforceOperationParameter {
    pub name: &'static str,
    pub required: bool,
    pub description: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SalesforceRestMethod {
    Get,
}

impl SalesforceRestMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
        }
    }

    pub fn to_saas_http_method(self) -> SaasHttpMethod {
        match self {
            Self::Get => SaasHttpMethod::Get,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SalesforceOperationPrimitive {
    ContinueQueryPage,
    DescribeObject,
    FetchLimits,
    GetUpdatedIds,
    GetDeletedIds,
    FetchByIds,
    QueryObjectIncremental,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SalesforceOperationAvailability {
    Executable,
    PlannedUnsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SalesforceOperationGate {
    LiveOrgDescribe,
    GovernanceReview,
    LiveCertification,
    RuntimeExecutor,
}

impl SalesforceOperationGate {
    pub fn reason(self) -> &'static str {
        match self {
            Self::LiveOrgDescribe => "requires authenticated Salesforce org describe evidence",
            Self::GovernanceReview => "requires PHI/PII governance review",
            Self::LiveCertification => "requires live connector certification evidence",
            Self::RuntimeExecutor => "requires runtime HTTP executor integration",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SalesforceDataSensitivity {
    Operational,
    BusinessConfidential,
    PhiPii,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SalesforceQueryPlan {
    pub operation_name: String,
    pub soql: String,
    pub timeout_ms: u64,
    pub caps: SaasResponseCaps,
    pub watermark: Option<SalesforceWatermarkRequest>,
}

impl SalesforceQueryPlan {
    pub fn new(
        operation_name: impl Into<String>,
        soql: impl Into<String>,
        timeout_ms: u64,
        watermark: Option<SalesforceWatermarkRequest>,
    ) -> Self {
        Self {
            operation_name: operation_name.into(),
            soql: soql.into(),
            timeout_ms,
            caps: saas_query_response_caps(SalesforceQueryLimits::DEFAULT_MAX_RESULTS),
            watermark,
        }
    }

    pub fn with_max_rows(mut self, max_rows: u32) -> Self {
        self.caps = saas_query_response_caps(max_rows);
        self
    }

    pub fn to_saas_request_plan(&self) -> Result<SaasRequestPlan, String> {
        let plan = SaasRequestPlan::new(
            self.operation_name.clone(),
            SaasTransportProtocol::RestJson,
            SaasHttpMethod::Get,
            format!("{SALESFORCE_REST_BASE_PATH}/query"),
        )
        .with_query("q", self.soql.clone())
        .with_timeout_ms(self.timeout_ms)
        .with_caps(self.caps);

        plan.validate()?;
        Ok(plan)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SalesforceRestRequestPlan {
    pub operation_name: String,
    pub primitive: SalesforceOperationPrimitive,
    pub method: SalesforceRestMethod,
    pub path: String,
    pub query_parameters: BTreeMap<String, String>,
    pub timeout_ms: u64,
    pub object: Option<SalesforceObject>,
    pub expects_limit_header: bool,
}

impl SalesforceRestRequestPlan {
    pub fn new(
        operation_name: impl Into<String>,
        primitive: SalesforceOperationPrimitive,
        method: SalesforceRestMethod,
        path: impl Into<String>,
        timeout_ms: u64,
    ) -> Self {
        Self {
            operation_name: operation_name.into(),
            primitive,
            method,
            path: path.into(),
            query_parameters: BTreeMap::new(),
            timeout_ms,
            object: None,
            expects_limit_header: true,
        }
    }

    pub fn with_object(mut self, object: SalesforceObject) -> Self {
        self.object = Some(object);
        self
    }

    pub fn with_query_parameter(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.query_parameters.insert(name.into(), value.into());
        self
    }

    pub fn continue_query_page(
        operation_name: impl Into<String>,
        next_records_url: &SalesforceNextRecordsUrl,
        timeout_ms: u64,
    ) -> Self {
        Self::new(
            operation_name,
            SalesforceOperationPrimitive::ContinueQueryPage,
            SalesforceRestMethod::Get,
            next_records_url.as_path(),
            timeout_ms,
        )
    }

    pub fn to_saas_request_plan(&self) -> Result<SaasRequestPlan, String> {
        self.to_saas_request_plan_with_limits(SalesforceQueryLimits::default())
    }

    pub fn to_saas_request_plan_with_limits(
        &self,
        limits: SalesforceQueryLimits,
    ) -> Result<SaasRequestPlan, String> {
        let mut plan = SaasRequestPlan::new(
            self.operation_name.clone(),
            SaasTransportProtocol::RestJson,
            self.method.to_saas_http_method(),
            self.path.clone(),
        )
        .with_timeout_ms(self.timeout_ms)
        .with_caps(saas_response_caps(self.primitive, limits));

        for (name, value) in &self.query_parameters {
            plan = plan.with_query(name.clone(), value.clone());
        }

        plan.validate()?;
        Ok(plan)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SalesforceOperationPlan {
    Query(SalesforceQueryPlan),
    Rest(SalesforceRestRequestPlan),
}

impl SalesforceOperationPlan {
    pub fn to_saas_request_plan(&self) -> Result<SaasRequestPlan, String> {
        self.to_saas_request_plan_with_limits(SalesforceQueryLimits::default())
    }

    pub fn to_saas_request_plan_with_limits(
        &self,
        limits: SalesforceQueryLimits,
    ) -> Result<SaasRequestPlan, String> {
        match self {
            Self::Query(plan) => plan.to_saas_request_plan(),
            Self::Rest(plan) => plan.to_saas_request_plan_with_limits(limits),
        }
    }
}

fn saas_response_caps(
    primitive: SalesforceOperationPrimitive,
    limits: SalesforceQueryLimits,
) -> SaasResponseCaps {
    let max_rows = match primitive {
        SalesforceOperationPrimitive::DescribeObject
        | SalesforceOperationPrimitive::FetchLimits => 1,
        SalesforceOperationPrimitive::ContinueQueryPage
        | SalesforceOperationPrimitive::GetUpdatedIds
        | SalesforceOperationPrimitive::GetDeletedIds
        | SalesforceOperationPrimitive::FetchByIds
        | SalesforceOperationPrimitive::QueryObjectIncremental => {
            effective_cap_rows(limits.max_results)
        }
    };

    SaasResponseCaps {
        max_body_bytes: SaasResponseCaps::DEFAULT_MAX_BODY_BYTES,
        max_rows,
    }
}

fn saas_query_response_caps(max_rows: u32) -> SaasResponseCaps {
    SaasResponseCaps {
        max_body_bytes: SaasResponseCaps::DEFAULT_MAX_BODY_BYTES,
        max_rows: effective_cap_rows(max_rows),
    }
}

fn effective_cap_rows(max_results: u32) -> u32 {
    match max_results {
        0 => SalesforceQueryLimits::DEFAULT_MAX_RESULTS,
        value => value.min(SalesforceQueryLimits::SOQL_MAX_LIMIT),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use appfw_saas_core::{SaasHttpMethod, SaasTransportProtocol};

    #[test]
    fn rest_plan_conversion_preserves_query_parameters() {
        let plan = SalesforceRestRequestPlan::new(
            "salesforce.account.query",
            SalesforceOperationPrimitive::QueryObjectIncremental,
            SalesforceRestMethod::Get,
            "/services/data/v67.0/query",
            SalesforceQueryLimits::DEFAULT_TIMEOUT_MS,
        )
        .with_query_parameter("q", "SELECT Id, Name FROM Account LIMIT 25");

        let saas_plan = plan.to_saas_request_plan().expect("valid SaaS plan");

        assert_eq!(saas_plan.protocol, SaasTransportProtocol::RestJson);
        assert_eq!(saas_plan.method, SaasHttpMethod::Get);
        assert_eq!(saas_plan.path, "/services/data/v67.0/query");
        assert_eq!(
            saas_plan.query.get("q").map(String::as_str),
            Some("SELECT Id, Name FROM Account LIMIT 25")
        );
        assert_eq!(
            saas_plan.timeout_ms,
            SalesforceQueryLimits::DEFAULT_TIMEOUT_MS
        );
        assert_eq!(
            saas_plan.caps.max_rows,
            SalesforceQueryLimits::DEFAULT_MAX_RESULTS
        );
        saas_plan.validate().expect("shared plan accepts query");
    }

    #[test]
    fn rest_plan_conversion_rejects_raw_absolute_urls() {
        let plan = SalesforceRestRequestPlan::new(
            "salesforce.health.fetch_limits",
            SalesforceOperationPrimitive::FetchLimits,
            SalesforceRestMethod::Get,
            "https://example.my.salesforce.com/services/data/v67.0/limits",
            SalesforceQueryLimits::DEFAULT_TIMEOUT_MS,
        );

        let error = plan.to_saas_request_plan().unwrap_err();

        assert!(error.contains("path"));
    }

    #[test]
    fn query_plan_conversion_targets_salesforce_query_endpoint_with_caps() {
        let plan = SalesforceQueryPlan::new(
            "salesforce.updated_accounts",
            "SELECT Id, SystemModstamp FROM Account ORDER BY SystemModstamp ASC LIMIT 50",
            5_000,
            None,
        )
        .with_max_rows(50);

        let saas_plan = plan.to_saas_request_plan().expect("valid SaaS plan");

        assert_eq!(saas_plan.operation_name, "salesforce.updated_accounts");
        assert_eq!(saas_plan.protocol, SaasTransportProtocol::RestJson);
        assert_eq!(saas_plan.method, SaasHttpMethod::Get);
        assert_eq!(saas_plan.path, "/services/data/v67.0/query");
        assert_eq!(
            saas_plan.query.get("q").map(String::as_str),
            Some("SELECT Id, SystemModstamp FROM Account ORDER BY SystemModstamp ASC LIMIT 50")
        );
        assert_eq!(saas_plan.timeout_ms, 5_000);
        assert_eq!(saas_plan.caps.max_rows, 50);
        assert!(saas_plan.body.is_empty());
        saas_plan
            .validate()
            .expect("shared plan accepts SOQL query");
    }
}
