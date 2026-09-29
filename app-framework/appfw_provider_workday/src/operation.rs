use std::collections::BTreeMap;

use appfw_saas_core::{
    SaasHttpMethod, SaasRequestBody, SaasRequestPlan, SaasResponseCaps, SaasTransportProtocol,
};

use crate::{
    auth::validate_workday_tenant_segment, soap::WORKDAY_WWS_VERSION, xml, WorkdayObjectReference,
    WorkdayProviderError, WorkdayRequestTemplate, WorkdaySoapRequestTemplate, WorkdayWwsOperation,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkdayQueryLimits {
    pub timeout_ms: u64,
    pub max_pages: u32,
}

impl WorkdayQueryLimits {
    pub const DEFAULT_TIMEOUT_MS: u64 = 5_000;
    pub const DEFAULT_MAX_PAGES: u32 = 10;

    pub fn validate(self) -> Result<(), WorkdayProviderError> {
        if self.timeout_ms == 0 {
            return Err(WorkdayProviderError::InvalidParameter {
                name: "limits.timeout_ms",
                reason: "must be greater than zero",
            });
        }

        if self.max_pages == 0 {
            return Err(WorkdayProviderError::InvalidParameter {
                name: "limits.max_pages",
                reason: "must be greater than zero",
            });
        }

        Ok(())
    }
}

impl Default for WorkdayQueryLimits {
    fn default() -> Self {
        Self {
            timeout_ms: Self::DEFAULT_TIMEOUT_MS,
            max_pages: Self::DEFAULT_MAX_PAGES,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkdayResponseFilter {
    pub page: u32,
    pub count: u16,
    pub as_of_effective_date: Option<String>,
    pub as_of_entry_datetime: Option<String>,
}

impl WorkdayResponseFilter {
    pub const DEFAULT_COUNT: u16 = 100;
    pub const MAX_COUNT: u16 = 999;

    pub fn new(page: u32, count: u16) -> Self {
        Self {
            page,
            count,
            as_of_effective_date: None,
            as_of_entry_datetime: None,
        }
    }

    pub fn with_as_of_effective_date(mut self, value: impl Into<String>) -> Self {
        self.as_of_effective_date = Some(value.into());
        self
    }

    pub fn with_as_of_entry_datetime(mut self, value: impl Into<String>) -> Self {
        self.as_of_entry_datetime = Some(value.into());
        self
    }

    pub fn validate(&self) -> Result<(), WorkdayProviderError> {
        if self.page == 0 {
            return Err(WorkdayProviderError::InvalidParameter {
                name: "response_filter.page",
                reason: "must be greater than zero",
            });
        }

        if self.count == 0 || self.count > Self::MAX_COUNT {
            return Err(WorkdayProviderError::InvalidParameter {
                name: "response_filter.count",
                reason: "must be in the Workday Count range 1..999",
            });
        }

        if self.page > 1 && self.as_of_entry_datetime.is_none() {
            return Err(WorkdayProviderError::InvalidParameter {
                name: "response_filter.as_of_entry_datetime",
                reason: "must be set when traversing pages after page 1",
            });
        }

        if let Some(value) = &self.as_of_effective_date {
            xml::validate_date_literal("response_filter.as_of_effective_date", value)?;
        }

        if let Some(value) = &self.as_of_entry_datetime {
            xml::validate_datetime_literal("response_filter.as_of_entry_datetime", value)?;
        }

        Ok(())
    }

    pub(crate) fn to_xml(&self) -> Result<String, WorkdayProviderError> {
        self.validate()?;

        let mut xml = String::from("<bsvc:Response_Filter>");

        if let Some(value) = &self.as_of_effective_date {
            xml.push_str("<bsvc:As_Of_Effective_Date>");
            xml.push_str(&xml::escape_xml_text(value));
            xml.push_str("</bsvc:As_Of_Effective_Date>");
        }

        if let Some(value) = &self.as_of_entry_datetime {
            xml.push_str("<bsvc:As_Of_Entry_DateTime>");
            xml.push_str(&xml::escape_xml_text(value));
            xml.push_str("</bsvc:As_Of_Entry_DateTime>");
        }

        xml.push_str("<bsvc:Page>");
        xml.push_str(&self.page.to_string());
        xml.push_str("</bsvc:Page><bsvc:Count>");
        xml.push_str(&self.count.to_string());
        xml.push_str("</bsvc:Count></bsvc:Response_Filter>");

        Ok(xml)
    }
}

impl Default for WorkdayResponseFilter {
    fn default() -> Self {
        Self::new(1, Self::DEFAULT_COUNT)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkdayParameterValue {
    Bool(bool),
    Number(u32),
    String(String),
    Reference(WorkdayObjectReference),
}

impl WorkdayParameterValue {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            Self::Number(_) | Self::String(_) | Self::Reference(_) => None,
        }
    }

    pub fn as_number(&self) -> Option<u32> {
        match self {
            Self::Number(value) => Some(*value),
            Self::Bool(_) | Self::String(_) | Self::Reference(_) => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value.as_str()),
            Self::Bool(_) | Self::Number(_) | Self::Reference(_) => None,
        }
    }

    pub fn as_reference(&self) -> Option<&WorkdayObjectReference> {
        match self {
            Self::Reference(value) => Some(value),
            Self::Bool(_) | Self::Number(_) | Self::String(_) => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkdayNamedOperationRequest {
    pub name: String,
    pub parameters: BTreeMap<String, WorkdayParameterValue>,
    pub response_filter: WorkdayResponseFilter,
    pub limits: WorkdayQueryLimits,
}

impl WorkdayNamedOperationRequest {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            parameters: BTreeMap::new(),
            response_filter: WorkdayResponseFilter::default(),
            limits: WorkdayQueryLimits::default(),
        }
    }

    pub fn with_parameter(mut self, name: impl Into<String>, value: WorkdayParameterValue) -> Self {
        self.parameters.insert(name.into(), value);
        self
    }

    pub fn with_response_filter(mut self, response_filter: WorkdayResponseFilter) -> Self {
        self.response_filter = response_filter;
        self
    }

    pub fn with_limits(mut self, limits: WorkdayQueryLimits) -> Self {
        self.limits = limits;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkdayOperationParameter {
    pub name: &'static str,
    pub required: bool,
    pub description: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkdayOperationAvailability {
    Executable,
    PlannedUnsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkdayOperationGate {
    TenantApprovedWwsRequestShape,
    HrPiiGovernanceReview,
    IdentitySecurityGovernanceReview,
    LiveCertification,
}

impl WorkdayOperationGate {
    pub fn reason(self) -> &'static str {
        match self {
            Self::TenantApprovedWwsRequestShape => {
                "requires tenant-approved Workday WWS request-shape evidence"
            }
            Self::HrPiiGovernanceReview => "requires HR PII governance review",
            Self::IdentitySecurityGovernanceReview => {
                "requires identity/security governance review"
            }
            Self::LiveCertification => "requires live connector certification evidence",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkdayWorkerTransactionLogCriteria {
    pub updated_from: Option<String>,
    pub updated_through: Option<String>,
    pub effective_from: Option<String>,
    pub effective_through: Option<String>,
}

impl WorkdayWorkerTransactionLogCriteria {
    pub fn is_empty(&self) -> bool {
        self.updated_from.is_none()
            && self.updated_through.is_none()
            && self.effective_from.is_none()
            && self.effective_through.is_none()
    }

    pub fn validate(&self) -> Result<(), WorkdayProviderError> {
        if let Some(value) = &self.updated_from {
            xml::validate_datetime_literal("updated_from", value)?;
        }

        if let Some(value) = &self.updated_through {
            xml::validate_datetime_literal("updated_through", value)?;
        }

        if let Some(value) = &self.effective_from {
            xml::validate_date_literal("effective_from", value)?;
        }

        if let Some(value) = &self.effective_through {
            xml::validate_date_literal("effective_through", value)?;
        }

        validate_pair(
            "updated_from",
            "updated_through",
            self.updated_from.as_deref(),
            self.updated_through.as_deref(),
        )?;
        validate_pair(
            "effective_from",
            "effective_through",
            self.effective_from.as_deref(),
            self.effective_through.as_deref(),
        )?;

        Ok(())
    }

    pub(crate) fn to_xml(&self) -> Result<Option<String>, WorkdayProviderError> {
        if self.is_empty() {
            return Ok(None);
        }

        self.validate()?;

        let mut xml = String::from("<bsvc:Transaction_Log_Criteria_Data>");
        push_optional_text(&mut xml, "Updated_From", self.updated_from.as_deref());
        push_optional_text(&mut xml, "Updated_Through", self.updated_through.as_deref());
        push_optional_text(&mut xml, "Effective_From", self.effective_from.as_deref());
        push_optional_text(
            &mut xml,
            "Effective_Through",
            self.effective_through.as_deref(),
        );
        xml.push_str("</bsvc:Transaction_Log_Criteria_Data>");

        Ok(Some(xml))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkdayRequestPlan {
    pub operation_name: String,
    pub wws_operation: WorkdayWwsOperation,
    pub request_template: WorkdayRequestTemplate,
    pub soap_envelope: String,
    pub response_filter: WorkdayResponseFilter,
    pub timeout_ms: u64,
    pub max_pages: u32,
    pub read_only: bool,
}

impl WorkdayRequestPlan {
    const SOAP_XML_CONTENT_TYPE: &'static str = "text/xml";

    pub fn new(
        operation_name: impl Into<String>,
        request_template: WorkdayRequestTemplate,
        soap_envelope: impl Into<String>,
        response_filter: WorkdayResponseFilter,
        limits: WorkdayQueryLimits,
    ) -> Self {
        Self {
            operation_name: operation_name.into(),
            wws_operation: request_template.wws_operation(),
            request_template,
            soap_envelope: soap_envelope.into(),
            response_filter,
            timeout_ms: limits.timeout_ms,
            max_pages: limits.max_pages,
            read_only: true,
        }
    }

    /// Convert this network-free Workday SOAP plan into the shared SaaS request
    /// shape. The caller supplies a tenant-relative path such as
    /// `/ccx/service/{tenant}/Human_Resources/v46.1`; no host or scheme is
    /// carried in the plan.
    pub fn to_saas_request_plan(
        &self,
        human_resources_wws_path: impl Into<String>,
    ) -> Result<SaasRequestPlan, WorkdayProviderError> {
        WorkdayQueryLimits {
            timeout_ms: self.timeout_ms,
            max_pages: self.max_pages,
        }
        .validate()?;
        self.response_filter.validate()?;

        let plan = SaasRequestPlan::new(
            self.operation_name.clone(),
            SaasTransportProtocol::SoapXml,
            SaasHttpMethod::Post,
            human_resources_wws_path,
        )
        .with_body(SaasRequestBody::Text {
            content_type: Self::SOAP_XML_CONTENT_TYPE.to_string(),
            value: self.soap_envelope.clone(),
        })
        .with_timeout_ms(self.timeout_ms)
        .with_caps(self.saas_response_caps());

        plan.validate()
            .map_err(WorkdayProviderError::InvalidSaasRequestPlan)?;
        validate_human_resources_wws_path(&plan.path)?;

        Ok(plan)
    }

    pub fn saas_response_caps(&self) -> SaasResponseCaps {
        let max_rows = u32::from(self.response_filter.count).saturating_mul(self.max_pages);
        let max_body_bytes =
            SaasResponseCaps::DEFAULT_MAX_BODY_BYTES.saturating_mul(u64::from(self.max_pages));

        SaasResponseCaps {
            max_body_bytes,
            max_rows,
        }
    }
}

fn validate_human_resources_wws_path(path: &str) -> Result<(), WorkdayProviderError> {
    let segments = path.split('/').collect::<Vec<_>>();
    if let ["", "ccx", "service", tenant, "Human_Resources", version] = segments.as_slice() {
        validate_workday_tenant_segment("human_resources_wws_path.tenant", tenant)?;
        if *version == WORKDAY_WWS_VERSION {
            return Ok(());
        }
    }

    Err(WorkdayProviderError::InvalidParameter {
        name: "human_resources_wws_path",
        reason: "must be /ccx/service/{tenant}/Human_Resources/v46.1",
    })
}

pub(crate) fn optional_string_parameter<'a>(
    request: &'a WorkdayNamedOperationRequest,
    name: &'static str,
) -> Result<Option<&'a str>, WorkdayProviderError> {
    match request.parameters.get(name) {
        Some(WorkdayParameterValue::String(value)) => Ok(Some(value.as_str())),
        Some(_) => Err(WorkdayProviderError::InvalidParameter {
            name,
            reason: "must be a string",
        }),
        None => Ok(None),
    }
}

pub(crate) fn required_string_parameter<'a>(
    request: &'a WorkdayNamedOperationRequest,
    name: &'static str,
) -> Result<&'a str, WorkdayProviderError> {
    optional_string_parameter(request, name)?.ok_or(WorkdayProviderError::MissingParameter(name))
}

pub(crate) fn required_reference_parameter<'a>(
    request: &'a WorkdayNamedOperationRequest,
    name: &'static str,
) -> Result<&'a WorkdayObjectReference, WorkdayProviderError> {
    request
        .parameters
        .get(name)
        .ok_or(WorkdayProviderError::MissingParameter(name))
        .and_then(|value| {
            value
                .as_reference()
                .ok_or(WorkdayProviderError::InvalidParameter {
                    name,
                    reason: "must be a Workday object reference",
                })
        })
}

pub(crate) fn optional_bool_parameter(
    request: &WorkdayNamedOperationRequest,
    name: &'static str,
) -> Result<Option<bool>, WorkdayProviderError> {
    match request.parameters.get(name) {
        Some(WorkdayParameterValue::Bool(value)) => Ok(Some(*value)),
        Some(_) => Err(WorkdayProviderError::InvalidParameter {
            name,
            reason: "must be a boolean",
        }),
        None => Ok(None),
    }
}

pub(crate) fn push_optional_text(
    xml: &mut String,
    element_name: &'static str,
    value: Option<&str>,
) {
    if let Some(value) = value {
        xml.push_str("<bsvc:");
        xml.push_str(element_name);
        xml.push('>');
        xml.push_str(&xml::escape_xml_text(value));
        xml.push_str("</bsvc:");
        xml.push_str(element_name);
        xml.push('>');
    }
}

fn validate_pair(
    from_name: &'static str,
    through_name: &'static str,
    from: Option<&str>,
    through: Option<&str>,
) -> Result<(), WorkdayProviderError> {
    if from.is_some() != through.is_some() {
        return Err(WorkdayProviderError::InvalidParameter {
            name: from_name,
            reason: "from/through criteria must be supplied together",
        });
    }

    if from.is_none() && through.is_none() {
        return Ok(());
    }

    if from > through {
        return Err(WorkdayProviderError::InvalidParameter {
            name: through_name,
            reason: "through criteria must not sort before from criteria",
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_filter_defaults_to_first_page_count_100() {
        let filter = WorkdayResponseFilter::default();

        assert_eq!(filter.page, 1);
        assert_eq!(filter.count, 100);
        assert_eq!(filter.as_of_effective_date, None);
        assert_eq!(filter.as_of_entry_datetime, None);
    }

    #[test]
    fn response_filter_rejects_count_outside_wws_range() {
        let filter = WorkdayResponseFilter::new(1, 1_000);

        assert_eq!(
            filter.validate().unwrap_err(),
            WorkdayProviderError::InvalidParameter {
                name: "response_filter.count",
                reason: "must be in the Workday Count range 1..999"
            }
        );
    }

    #[test]
    fn response_filter_requires_as_of_entry_datetime_after_first_page() {
        let filter = WorkdayResponseFilter::new(2, 100);

        assert_eq!(
            filter.validate().unwrap_err(),
            WorkdayProviderError::InvalidParameter {
                name: "response_filter.as_of_entry_datetime",
                reason: "must be set when traversing pages after page 1"
            }
        );
    }

    #[test]
    fn transaction_log_criteria_requires_paired_updated_range() {
        let criteria = WorkdayWorkerTransactionLogCriteria {
            updated_from: Some("2026-06-26T00:00:00Z".to_string()),
            updated_through: None,
            effective_from: None,
            effective_through: None,
        };

        assert_eq!(
            criteria.validate().unwrap_err(),
            WorkdayProviderError::InvalidParameter {
                name: "updated_from",
                reason: "from/through criteria must be supplied together"
            }
        );
    }

    #[test]
    fn transaction_log_criteria_requires_normalized_utc_datetimes() {
        let criteria = WorkdayWorkerTransactionLogCriteria {
            updated_from: Some("2026-06-26T01:00:00+02:00".to_string()),
            updated_through: Some("2026-06-26T02:00:00+02:00".to_string()),
            effective_from: None,
            effective_through: None,
        };

        assert_eq!(
            criteria.validate().unwrap_err(),
            WorkdayProviderError::InvalidParameter {
                name: "updated_from",
                reason: "must be a UTC ISO-8601 datetime literal in YYYY-MM-DDTHH:MM:SSZ form"
            }
        );
    }
}
