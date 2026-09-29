use std::collections::BTreeMap;

use appfw_saas_core::{
    ChunkDownloadContinuation, ReadRequestPageContinuation, SaasHttpMethod, SaasRequestPlan,
    SaasResponseCaps, SaasTransportProtocol,
};

use crate::AnaplanProviderError;

pub const ANAPLAN_API_BASE_PATH: &str = "/2/0";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnaplanQueryLimits {
    pub max_results: u32,
    pub timeout_ms: u64,
    pub offset: u64,
}

impl AnaplanQueryLimits {
    pub const DEFAULT_MAX_RESULTS: u32 = 100;
    pub const DEFAULT_TIMEOUT_MS: u64 = 2_000;
    pub const MAX_PAGE_LIMIT: u32 = 1_000;

    pub fn effective_max_results(self) -> Result<u32, AnaplanProviderError> {
        if self.max_results == 0 {
            return Err(AnaplanProviderError::InvalidParameter {
                name: "limits.max_results",
                reason: "must be greater than zero",
            });
        }

        Ok(self.max_results.min(Self::MAX_PAGE_LIMIT))
    }
}

impl Default for AnaplanQueryLimits {
    fn default() -> Self {
        Self {
            max_results: Self::DEFAULT_MAX_RESULTS,
            timeout_ms: Self::DEFAULT_TIMEOUT_MS,
            offset: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnaplanParameterValue {
    ChunkDownload(ChunkDownloadContinuation),
    ReadRequestPage(ReadRequestPageContinuation),
    String(String),
}

impl AnaplanParameterValue {
    pub fn as_str(&self) -> &str {
        match self {
            Self::String(value) => value.as_str(),
            Self::ChunkDownload(_) | Self::ReadRequestPage(_) => "",
        }
    }

    pub fn as_read_request_page(&self) -> Option<&ReadRequestPageContinuation> {
        match self {
            Self::ReadRequestPage(value) => Some(value),
            Self::ChunkDownload(_) | Self::String(_) => None,
        }
    }

    pub fn as_chunk_download(&self) -> Option<&ChunkDownloadContinuation> {
        match self {
            Self::ChunkDownload(value) => Some(value),
            Self::ReadRequestPage(_) | Self::String(_) => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnaplanNamedOperationRequest {
    pub name: String,
    pub parameters: BTreeMap<String, AnaplanParameterValue>,
    pub limits: AnaplanQueryLimits,
}

impl AnaplanNamedOperationRequest {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            parameters: BTreeMap::new(),
            limits: AnaplanQueryLimits::default(),
        }
    }

    pub fn with_parameter(mut self, name: impl Into<String>, value: AnaplanParameterValue) -> Self {
        self.parameters.insert(name.into(), value);
        self
    }

    pub fn with_limits(mut self, limits: AnaplanQueryLimits) -> Self {
        self.limits = limits;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnaplanOperationParameter {
    pub name: &'static str,
    pub required: bool,
    pub description: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnaplanRestMethod {
    Get,
    Post,
}

impl AnaplanRestMethod {
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
pub enum AnaplanOperationPrimitive {
    GetModelStatus,
    ListFiles,
    CreateViewReadRequest,
    GetViewReadRequest,
    GetViewReadPage,
    DownloadFileChunk,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnaplanOperationAvailability {
    Executable,
    PlannedUnsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnaplanOperationGate {
    TenantWorkspaceModelScope,
    TenantExportAllowList,
    LiveCertification,
    RuntimeExecutor,
}

impl AnaplanOperationGate {
    pub fn reason(self) -> &'static str {
        match self {
            Self::TenantWorkspaceModelScope => {
                "requires tenant-approved workspace and model binding"
            }
            Self::TenantExportAllowList => {
                "requires tenant-approved export/action allow-list before execution"
            }
            Self::LiveCertification => "requires live connector certification evidence",
            Self::RuntimeExecutor => "requires runtime HTTP executor integration",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnaplanDataSensitivity {
    Operational,
    BusinessConfidential,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnaplanRestRequestPlan {
    pub operation_name: String,
    pub primitive: AnaplanOperationPrimitive,
    pub method: AnaplanRestMethod,
    pub path: String,
    pub query_parameters: BTreeMap<String, String>,
    pub timeout_ms: u64,
    pub caps: SaasResponseCaps,
}

impl AnaplanRestRequestPlan {
    pub fn new(
        operation_name: impl Into<String>,
        primitive: AnaplanOperationPrimitive,
        method: AnaplanRestMethod,
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
            caps: anaplan_response_caps(AnaplanQueryLimits::DEFAULT_MAX_RESULTS),
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

    pub fn with_caps(mut self, caps: SaasResponseCaps) -> Self {
        self.caps = caps;
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

        plan.validate()?;
        Ok(plan)
    }
}

pub fn anaplan_response_caps(max_rows: u32) -> SaasResponseCaps {
    SaasResponseCaps {
        max_body_bytes: SaasResponseCaps::DEFAULT_MAX_BODY_BYTES,
        max_rows: max_rows.max(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use appfw_saas_core::SaasTransportProtocol;

    #[test]
    fn api_base_path_uses_anaplan_integration_api_v2() {
        assert_eq!(ANAPLAN_API_BASE_PATH, "/2/0");
    }

    #[test]
    fn rest_plan_conversion_preserves_offset_limit_parameters() {
        let plan = AnaplanRestRequestPlan::new(
            "anaplan.model.list_files",
            AnaplanOperationPrimitive::ListFiles,
            AnaplanRestMethod::Get,
            "/2/0/workspaces/workspace/models/model/files",
            AnaplanQueryLimits::DEFAULT_TIMEOUT_MS,
        )
        .with_query_parameter("limit", "25")
        .with_query_parameter("offset", "50")
        .with_caps(anaplan_response_caps(25));

        let saas_plan = plan.to_saas_request_plan().expect("valid SaaS plan");

        assert_eq!(saas_plan.protocol, SaasTransportProtocol::RestJson);
        assert_eq!(saas_plan.method, SaasHttpMethod::Get);
        assert_eq!(
            saas_plan.path,
            "/2/0/workspaces/workspace/models/model/files"
        );
        assert_eq!(saas_plan.query.get("limit").map(String::as_str), Some("25"));
        assert_eq!(
            saas_plan.query.get("offset").map(String::as_str),
            Some("50")
        );
        assert_eq!(saas_plan.caps.max_rows, 25);
    }

    #[test]
    fn rest_plan_conversion_rejects_raw_absolute_urls() {
        let plan = AnaplanRestRequestPlan::new(
            "anaplan.model.list_files",
            AnaplanOperationPrimitive::ListFiles,
            AnaplanRestMethod::Get,
            "https://api.anaplan.com/2/0/workspaces/workspace/models/model/files",
            AnaplanQueryLimits::DEFAULT_TIMEOUT_MS,
        );

        let error = plan.to_saas_request_plan().unwrap_err();

        assert!(error.contains("path"));
    }
}
