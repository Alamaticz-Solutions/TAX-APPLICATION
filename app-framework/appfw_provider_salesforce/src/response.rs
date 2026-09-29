use std::collections::BTreeSet;

use appfw_saas_core::{
    PaginationContinuation, RateLimitSignal, RetryDecision, RetryPolicy, UrlCursorContinuation,
};
use serde_json::Value;

use crate::{
    operation::SALESFORCE_REST_BASE_PATH, SalesforceField, SalesforceObject,
    SalesforceProviderError,
};

pub const SFORCE_LIMIT_INFO_HEADER: &str = "Sforce-Limit-Info";
pub const REQUEST_LIMIT_EXCEEDED_ERROR_CODE: &str = "REQUEST_LIMIT_EXCEEDED";
pub const SALESFORCE_NEXT_RECORDS_URL_FIELD: &str = "nextRecordsUrl";
const SALESFORCE_TOTAL_SIZE_FIELD: &str = "totalSize";
const SALESFORCE_DONE_FIELD: &str = "done";
const SALESFORCE_RECORDS_FIELD: &str = "records";
const SALESFORCE_RECORD_ID_FIELD: &str = "Id";
const SALESFORCE_RECORD_ATTRIBUTES_FIELD: &str = "attributes";
const SALESFORCE_RECORD_TYPE_FIELD: &str = "type";
const SALESFORCE_OBJECT_NAME_FIELD: &str = "name";
const SALESFORCE_DESCRIBE_FIELDS_FIELD: &str = "fields";
const SALESFORCE_DEPRECATED_AND_HIDDEN_FIELD: &str = "deprecatedAndHidden";
const SALESFORCE_UPDATED_IDS_FIELD: &str = "ids";
const SALESFORCE_LATEST_DATE_COVERED_FIELD: &str = "latestDateCovered";
const SALESFORCE_DELETED_RECORDS_FIELD: &str = "deletedRecords";
const SALESFORCE_DELETED_RECORD_ID_FIELD: &str = "id";
const SALESFORCE_DELETED_DATE_FIELD: &str = "deletedDate";
const SALESFORCE_EARLIEST_DATE_AVAILABLE_FIELD: &str = "earliestDateAvailable";
const SALESFORCE_DAILY_API_REQUESTS_LIMIT: &str = "DailyApiRequests";
const SALESFORCE_LIMIT_MAX_FIELD: &str = "Max";
const SALESFORCE_LIMIT_REMAINING_FIELD: &str = "Remaining";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SalesforceNextRecordsUrl {
    path: String,
}

impl SalesforceNextRecordsUrl {
    pub fn parse(value: impl Into<String>) -> Result<Self, SalesforceProviderError> {
        let path = value.into();
        validate_next_records_url(&path)?;

        Ok(Self { path })
    }

    pub fn as_path(&self) -> &str {
        self.path.as_str()
    }

    pub fn is_query_all(&self) -> bool {
        self.locator()
            .map(|locator| locator.kind == SalesforceQueryLocatorKind::QueryAll)
            .unwrap_or(false)
    }

    pub fn to_pagination_continuation(&self) -> PaginationContinuation {
        PaginationContinuation::UrlCursor(UrlCursorContinuation::new(self.path.clone()))
    }

    fn locator(&self) -> Option<SalesforceQueryLocator<'_>> {
        parse_query_locator(self.path.as_str()).ok()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SalesforceRedactedRecordPayload {
    pub id: Option<String>,
    pub sobject_type: Option<String>,
    pub field_count: usize,
}

impl SalesforceRedactedRecordPayload {
    pub fn new(
        id: Option<impl Into<String>>,
        sobject_type: Option<impl Into<String>>,
        field_count: usize,
    ) -> Self {
        Self {
            id: id.map(Into::into),
            sobject_type: sobject_type.map(Into::into),
            field_count,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SalesforceQueryResponsePage {
    pub total_size: u32,
    pub done: bool,
    pub next_records_url: Option<SalesforceNextRecordsUrl>,
    pub records: Vec<SalesforceRedactedRecordPayload>,
}

impl SalesforceQueryResponsePage {
    pub fn new(
        total_size: u32,
        done: bool,
        next_records_url: Option<SalesforceNextRecordsUrl>,
        records: Vec<SalesforceRedactedRecordPayload>,
    ) -> Result<Self, SalesforceProviderError> {
        if total_size < records.len() as u32 {
            return Err(SalesforceProviderError::InvalidParameter {
                name: "totalSize",
                reason: "must be greater than or equal to the returned record count",
            });
        }

        match (done, next_records_url.as_ref()) {
            (false, None) => {
                return Err(SalesforceProviderError::InvalidParameter {
                    name: SALESFORCE_NEXT_RECORDS_URL_FIELD,
                    reason: "must be present when done is false",
                });
            }
            (true, Some(_)) => {
                return Err(SalesforceProviderError::InvalidParameter {
                    name: SALESFORCE_NEXT_RECORDS_URL_FIELD,
                    reason: "must be absent when done is true",
                });
            }
            (false, Some(_)) | (true, None) => {}
        }

        Ok(Self {
            total_size,
            done,
            next_records_url,
            records,
        })
    }

    pub fn from_salesforce_metadata(
        total_size: u32,
        done: bool,
        next_records_url: Option<&str>,
        records: Vec<SalesforceRedactedRecordPayload>,
    ) -> Result<Self, SalesforceProviderError> {
        let next_records_url = next_records_url
            .map(SalesforceNextRecordsUrl::parse)
            .transpose()?;

        Self::new(total_size, done, next_records_url, records)
    }

    pub fn from_json(value: &Value) -> Result<Self, SalesforceProviderError> {
        let total_size = required_u32_field(value, SALESFORCE_TOTAL_SIZE_FIELD)?;
        let done = required_bool_field(value, SALESFORCE_DONE_FIELD)?;
        let next_records_url = optional_str_field(value, SALESFORCE_NEXT_RECORDS_URL_FIELD)?;
        let records = required_array_field(value, SALESFORCE_RECORDS_FIELD)?
            .iter()
            .map(redacted_record_payload_from_json)
            .collect::<Result<Vec<_>, _>>()?;

        Self::from_salesforce_metadata(total_size, done, next_records_url, records)
    }

    pub fn has_next_page(&self) -> bool {
        !self.done && self.next_records_url.is_some()
    }

    pub fn next_continuation(&self) -> Option<PaginationContinuation> {
        self.next_records_url
            .as_ref()
            .map(SalesforceNextRecordsUrl::to_pagination_continuation)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SalesforceQueryLocatorKind {
    Query,
    QueryAll,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SalesforceQueryLocator<'a> {
    kind: SalesforceQueryLocatorKind,
    locator: &'a str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SalesforceApiUsageLimit {
    pub used: u32,
    pub allowed: u32,
}

impl SalesforceApiUsageLimit {
    pub fn remaining(self) -> u32 {
        self.allowed.saturating_sub(self.used)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SalesforceLimitInfo {
    pub api_usage: Option<SalesforceApiUsageLimit>,
}

impl SalesforceLimitInfo {
    pub fn parse(header_value: &str) -> Result<Self, SalesforceProviderError> {
        let mut limit_info = Self { api_usage: None };

        for part in header_value.split(',') {
            let part = part.trim();

            if let Some(value) = part.strip_prefix("api-usage=") {
                limit_info.api_usage = Some(parse_api_usage(value)?);
            }
        }

        Ok(limit_info)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SalesforceDescribeObjectSummary {
    pub object_name: String,
    pub field_count: usize,
    pub required_field_count: usize,
}

impl SalesforceDescribeObjectSummary {
    pub fn from_json_for_projection(
        value: &Value,
        object: SalesforceObject,
        required_fields: &[SalesforceField],
    ) -> Result<Self, SalesforceProviderError> {
        let object_name = required_str_field(value, SALESFORCE_OBJECT_NAME_FIELD)?;
        if object_name != object.api_name() {
            return Err(SalesforceProviderError::InvalidParameter {
                name: SALESFORCE_OBJECT_NAME_FIELD,
                reason: "must match the registered Salesforce object",
            });
        }

        let fields = required_array_field(value, SALESFORCE_DESCRIBE_FIELDS_FIELD)?;
        let available_fields = available_describe_field_names(fields)?;

        for field in required_fields {
            if field.object() != object {
                return Err(SalesforceProviderError::InvalidParameter {
                    name: SALESFORCE_DESCRIBE_FIELDS_FIELD,
                    reason: "required field must belong to the registered Salesforce object",
                });
            }
            if !available_fields.contains(field.api_name()) {
                return Err(SalesforceProviderError::InvalidParameter {
                    name: SALESFORCE_DESCRIBE_FIELDS_FIELD,
                    reason: "required field is missing or hidden",
                });
            }
        }

        Ok(Self {
            object_name: object_name.to_string(),
            field_count: fields.len(),
            required_field_count: required_fields.len(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SalesforceUpdatedIdsSummary {
    pub ids: Vec<String>,
    pub latest_date_covered: String,
}

impl SalesforceUpdatedIdsSummary {
    pub fn from_json(value: &Value) -> Result<Self, SalesforceProviderError> {
        let ids = required_array_field(value, SALESFORCE_UPDATED_IDS_FIELD)?
            .iter()
            .map(|id| {
                let id = id
                    .as_str()
                    .ok_or(SalesforceProviderError::InvalidParameter {
                        name: SALESFORCE_UPDATED_IDS_FIELD,
                        reason: "each id must be a string",
                    })?;
                validate_salesforce_id("id", id)?;
                Ok(id.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let latest_date_covered = required_str_field(value, SALESFORCE_LATEST_DATE_COVERED_FIELD)?;
        validate_salesforce_datetime(SALESFORCE_LATEST_DATE_COVERED_FIELD, latest_date_covered)?;

        Ok(Self {
            ids,
            latest_date_covered: latest_date_covered.to_string(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SalesforceDeletedRecordSummary {
    pub id: String,
    pub deleted_date: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SalesforceDeletedIdsSummary {
    pub deleted_records: Vec<SalesforceDeletedRecordSummary>,
    pub earliest_date_available: String,
    pub latest_date_covered: String,
}

impl SalesforceDeletedIdsSummary {
    pub fn from_json(value: &Value) -> Result<Self, SalesforceProviderError> {
        let deleted_records = required_array_field(value, SALESFORCE_DELETED_RECORDS_FIELD)?
            .iter()
            .map(deleted_record_summary_from_json)
            .collect::<Result<Vec<_>, _>>()?;
        let earliest_date_available =
            required_str_field(value, SALESFORCE_EARLIEST_DATE_AVAILABLE_FIELD)?;
        validate_salesforce_datetime(
            SALESFORCE_EARLIEST_DATE_AVAILABLE_FIELD,
            earliest_date_available,
        )?;
        let latest_date_covered = required_str_field(value, SALESFORCE_LATEST_DATE_COVERED_FIELD)?;
        validate_salesforce_datetime(SALESFORCE_LATEST_DATE_COVERED_FIELD, latest_date_covered)?;

        Ok(Self {
            deleted_records,
            earliest_date_available: earliest_date_available.to_string(),
            latest_date_covered: latest_date_covered.to_string(),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SalesforceOrgLimit {
    pub max: u32,
    pub remaining: u32,
}

impl SalesforceOrgLimit {
    pub fn used(self) -> u32 {
        self.max.saturating_sub(self.remaining)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SalesforceLimitsSummary {
    pub daily_api_requests: Option<SalesforceOrgLimit>,
}

impl SalesforceLimitsSummary {
    pub fn from_json(value: &Value) -> Result<Self, SalesforceProviderError> {
        let daily_api_requests = value
            .get(SALESFORCE_DAILY_API_REQUESTS_LIMIT)
            .map(org_limit_from_json)
            .transpose()?;

        Ok(Self { daily_api_requests })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SalesforceRateLimitSummary {
    pub error_class: SalesforceRestErrorClass,
    pub retryable: bool,
    pub signal: RateLimitSignal,
}

impl SalesforceRateLimitSummary {
    pub fn from_response_parts<I, K, V>(status: u16, error_code: Option<&str>, headers: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let error_class = classify_salesforce_rest_error(status, error_code);
        let signal = RateLimitSignal::from_headers(headers);
        let retryable = error_class.is_retryable() || signal.is_limited();

        Self {
            error_class,
            retryable,
            signal,
        }
    }

    pub fn retry_decision(
        &self,
        attempt: u32,
        policy: &RetryPolicy,
    ) -> Result<RetryDecision, SalesforceProviderError> {
        policy
            .validate()
            .map_err(SalesforceProviderError::InvalidSaasRequestPlan)?;

        let should_retry = self.retryable && attempt < policy.max_attempts;
        let delay_ms = if should_retry {
            self.signal.retry_after_ms.unwrap_or_else(|| {
                policy
                    .backoff
                    .delay_ms_for_attempt(attempt.saturating_add(1))
            })
        } else {
            0
        };

        Ok(RetryDecision {
            should_retry,
            delay_ms,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SalesforceRestErrorClass {
    Authentication,
    Authorization,
    ConditionalRequest,
    Conflict,
    InvalidRequest,
    NotFound,
    RateLimited,
    RequestTooLarge,
    Transient,
    Unknown,
}

impl SalesforceRestErrorClass {
    pub fn is_retryable(self) -> bool {
        matches!(self, Self::RateLimited | Self::Transient)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SalesforceRestError {
    pub status: u16,
    pub error_code: Option<String>,
    pub fields: Vec<String>,
}

impl SalesforceRestError {
    pub fn new(status: u16, error_code: Option<&str>) -> Self {
        Self {
            status,
            error_code: error_code.map(str::to_string),
            fields: Vec::new(),
        }
    }

    pub fn with_error_code(status: u16, error_code: impl Into<String>) -> Self {
        Self {
            status,
            error_code: Some(error_code.into()),
            fields: Vec::new(),
        }
    }

    pub fn with_fields(mut self, fields: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.fields = fields.into_iter().map(Into::into).collect();
        self
    }

    pub fn classify(&self) -> SalesforceRestErrorClass {
        classify_salesforce_rest_error(self.status, self.error_code.as_deref())
    }
}

pub fn classify_salesforce_rest_error(
    status: u16,
    error_code: Option<&str>,
) -> SalesforceRestErrorClass {
    if matches!(error_code, Some(REQUEST_LIMIT_EXCEEDED_ERROR_CODE)) {
        return SalesforceRestErrorClass::RateLimited;
    }

    match status {
        400 => SalesforceRestErrorClass::InvalidRequest,
        401 => SalesforceRestErrorClass::Authentication,
        403 => SalesforceRestErrorClass::Authorization,
        404 => SalesforceRestErrorClass::NotFound,
        409 => SalesforceRestErrorClass::Conflict,
        429 => SalesforceRestErrorClass::RateLimited,
        414 | 431 => SalesforceRestErrorClass::RequestTooLarge,
        304 | 412 | 428 => SalesforceRestErrorClass::ConditionalRequest,
        500 | 502 | 503 => SalesforceRestErrorClass::Transient,
        _ => SalesforceRestErrorClass::Unknown,
    }
}

fn validate_next_records_url(value: &str) -> Result<(), SalesforceProviderError> {
    let locator = parse_query_locator(value)?;

    if locator.locator.is_empty() {
        return Err(SalesforceProviderError::InvalidParameter {
            name: SALESFORCE_NEXT_RECORDS_URL_FIELD,
            reason: "locator must not be empty",
        });
    }

    if !locator
        .locator
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return Err(SalesforceProviderError::InvalidParameter {
            name: SALESFORCE_NEXT_RECORDS_URL_FIELD,
            reason: "locator contains unsupported characters",
        });
    }

    Ok(())
}

fn parse_query_locator(value: &str) -> Result<SalesforceQueryLocator<'_>, SalesforceProviderError> {
    if value.is_empty() {
        return Err(SalesforceProviderError::InvalidParameter {
            name: SALESFORCE_NEXT_RECORDS_URL_FIELD,
            reason: "must not be empty",
        });
    }

    if value.contains("://") || value.starts_with("//") {
        return Err(SalesforceProviderError::InvalidParameter {
            name: SALESFORCE_NEXT_RECORDS_URL_FIELD,
            reason: "must be a Salesforce relative REST path, not an absolute URL",
        });
    }

    if value
        .chars()
        .any(|ch| ch.is_ascii_control() || ch.is_ascii_whitespace())
        || value.contains('?')
        || value.contains('#')
        || value.contains("..")
    {
        return Err(SalesforceProviderError::InvalidParameter {
            name: SALESFORCE_NEXT_RECORDS_URL_FIELD,
            reason: "must be an unadorned Salesforce query locator path",
        });
    }

    let query_prefix = format!("{SALESFORCE_REST_BASE_PATH}/query/");
    if let Some(locator) = value.strip_prefix(query_prefix.as_str()) {
        return Ok(SalesforceQueryLocator {
            kind: SalesforceQueryLocatorKind::Query,
            locator,
        });
    }

    let query_all_prefix = format!("{SALESFORCE_REST_BASE_PATH}/queryAll/");
    if let Some(locator) = value.strip_prefix(query_all_prefix.as_str()) {
        return Ok(SalesforceQueryLocator {
            kind: SalesforceQueryLocatorKind::QueryAll,
            locator,
        });
    }

    Err(SalesforceProviderError::InvalidParameter {
        name: SALESFORCE_NEXT_RECORDS_URL_FIELD,
        reason: "must begin with the Salesforce query or queryAll REST base path",
    })
}

fn redacted_record_payload_from_json(
    value: &Value,
) -> Result<SalesforceRedactedRecordPayload, SalesforceProviderError> {
    let record = value
        .as_object()
        .ok_or(SalesforceProviderError::InvalidParameter {
            name: SALESFORCE_RECORDS_FIELD,
            reason: "each record must be a JSON object",
        })?;
    let id = record
        .get(SALESFORCE_RECORD_ID_FIELD)
        .and_then(Value::as_str)
        .map(str::to_string);
    let sobject_type = record
        .get(SALESFORCE_RECORD_ATTRIBUTES_FIELD)
        .and_then(Value::as_object)
        .and_then(|attributes| attributes.get(SALESFORCE_RECORD_TYPE_FIELD))
        .and_then(Value::as_str)
        .map(str::to_string);

    Ok(SalesforceRedactedRecordPayload::new(
        id,
        sobject_type,
        record.len(),
    ))
}

fn available_describe_field_names(
    fields: &[Value],
) -> Result<BTreeSet<String>, SalesforceProviderError> {
    let mut names = BTreeSet::new();

    for field in fields {
        let field_name = required_str_field(field, SALESFORCE_OBJECT_NAME_FIELD)?;
        let deprecated_and_hidden =
            optional_bool_field(field, SALESFORCE_DEPRECATED_AND_HIDDEN_FIELD)?.unwrap_or(false);
        if !deprecated_and_hidden {
            names.insert(field_name.to_string());
        }
    }

    Ok(names)
}

fn org_limit_from_json(value: &Value) -> Result<SalesforceOrgLimit, SalesforceProviderError> {
    let max = required_u32_field(value, SALESFORCE_LIMIT_MAX_FIELD)?;
    let remaining = required_u32_field(value, SALESFORCE_LIMIT_REMAINING_FIELD)?;

    if remaining > max {
        return Err(SalesforceProviderError::InvalidParameter {
            name: SALESFORCE_DAILY_API_REQUESTS_LIMIT,
            reason: "remaining limit must not exceed max",
        });
    }

    Ok(SalesforceOrgLimit { max, remaining })
}

fn deleted_record_summary_from_json(
    value: &Value,
) -> Result<SalesforceDeletedRecordSummary, SalesforceProviderError> {
    let id = required_str_field(value, SALESFORCE_DELETED_RECORD_ID_FIELD)?;
    validate_salesforce_id(SALESFORCE_DELETED_RECORD_ID_FIELD, id)?;
    let deleted_date = required_str_field(value, SALESFORCE_DELETED_DATE_FIELD)?;
    validate_salesforce_datetime(SALESFORCE_DELETED_DATE_FIELD, deleted_date)?;

    Ok(SalesforceDeletedRecordSummary {
        id: id.to_string(),
        deleted_date: deleted_date.to_string(),
    })
}

fn validate_salesforce_id(name: &'static str, value: &str) -> Result<(), SalesforceProviderError> {
    if matches!(value.len(), 15 | 18) && value.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return Ok(());
    }

    Err(SalesforceProviderError::InvalidParameter {
        name,
        reason: "must be a 15 or 18 character Salesforce id",
    })
}

fn validate_salesforce_datetime(
    name: &'static str,
    value: &str,
) -> Result<(), SalesforceProviderError> {
    let has_basic_shape = value.len() >= 20
        && value.contains('T')
        && (value.ends_with('Z') || value.contains('+') || value.rmatch_indices('-').count() >= 3);
    let has_safe_chars = value
        .chars()
        .all(|ch| ch.is_ascii_digit() || matches!(ch, '-' | ':' | 'T' | 'Z' | '.' | '+'));

    if has_basic_shape && has_safe_chars {
        return Ok(());
    }

    Err(SalesforceProviderError::InvalidParameter {
        name,
        reason: "must be an ISO-8601 Salesforce datetime literal",
    })
}

fn required_u32_field(value: &Value, name: &'static str) -> Result<u32, SalesforceProviderError> {
    let number = value.get(name).and_then(Value::as_u64).ok_or(
        SalesforceProviderError::InvalidParameter {
            name,
            reason: "must be an unsigned integer",
        },
    )?;

    u32::try_from(number).map_err(|_| SalesforceProviderError::InvalidParameter {
        name,
        reason: "must fit in a 32-bit unsigned integer",
    })
}

fn required_str_field<'a>(
    value: &'a Value,
    name: &'static str,
) -> Result<&'a str, SalesforceProviderError> {
    value
        .get(name)
        .and_then(Value::as_str)
        .ok_or(SalesforceProviderError::InvalidParameter {
            name,
            reason: "must be a string",
        })
}

fn required_bool_field(value: &Value, name: &'static str) -> Result<bool, SalesforceProviderError> {
    value
        .get(name)
        .and_then(Value::as_bool)
        .ok_or(SalesforceProviderError::InvalidParameter {
            name,
            reason: "must be a boolean",
        })
}

fn optional_bool_field(
    value: &Value,
    name: &'static str,
) -> Result<Option<bool>, SalesforceProviderError> {
    value
        .get(name)
        .map(|field| {
            field
                .as_bool()
                .ok_or(SalesforceProviderError::InvalidParameter {
                    name,
                    reason: "must be a boolean when present",
                })
        })
        .transpose()
}

fn optional_str_field<'a>(
    value: &'a Value,
    name: &'static str,
) -> Result<Option<&'a str>, SalesforceProviderError> {
    value
        .get(name)
        .map(|field| {
            field
                .as_str()
                .ok_or(SalesforceProviderError::InvalidParameter {
                    name,
                    reason: "must be a string when present",
                })
        })
        .transpose()
}

fn required_array_field<'a>(
    value: &'a Value,
    name: &'static str,
) -> Result<&'a [Value], SalesforceProviderError> {
    value
        .get(name)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or(SalesforceProviderError::InvalidParameter {
            name,
            reason: "must be an array",
        })
}

fn parse_api_usage(value: &str) -> Result<SalesforceApiUsageLimit, SalesforceProviderError> {
    let (used, allowed) =
        value
            .split_once('/')
            .ok_or(SalesforceProviderError::InvalidParameter {
                name: SFORCE_LIMIT_INFO_HEADER,
                reason: "api-usage must be formatted as used/allowed",
            })?;

    Ok(SalesforceApiUsageLimit {
        used: used
            .parse::<u32>()
            .map_err(|_| SalesforceProviderError::InvalidParameter {
                name: SFORCE_LIMIT_INFO_HEADER,
                reason: "api-usage used value must be numeric",
            })?,
        allowed: allowed
            .parse::<u32>()
            .map_err(|_| SalesforceProviderError::InvalidParameter {
                name: SFORCE_LIMIT_INFO_HEADER,
                reason: "api-usage allowed value must be numeric",
            })?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_query_response_page_metadata_without_raw_records() {
        let page = SalesforceQueryResponsePage::from_salesforce_metadata(
            3,
            false,
            Some("/services/data/v67.0/query/01gD0000002HU6KIAW-2000"),
            vec![SalesforceRedactedRecordPayload::new(
                Some("001000000000001AAA"),
                Some("Account"),
                4,
            )],
        )
        .expect("page metadata");

        assert_eq!(page.total_size, 3);
        assert!(!page.done);
        assert!(page.has_next_page());
        assert_eq!(
            page.next_records_url.expect("next url").as_path(),
            "/services/data/v67.0/query/01gD0000002HU6KIAW-2000"
        );
        assert_eq!(
            page.records,
            vec![SalesforceRedactedRecordPayload {
                id: Some("001000000000001AAA".to_string()),
                sobject_type: Some("Account".to_string()),
                field_count: 4,
            }]
        );
    }

    #[test]
    fn parses_query_all_next_records_url() {
        let next_records_url =
            SalesforceNextRecordsUrl::parse("/services/data/v67.0/queryAll/01gabc-4000")
                .expect("queryAll nextRecordsUrl");

        assert_eq!(
            next_records_url.as_path(),
            "/services/data/v67.0/queryAll/01gabc-4000"
        );
        assert!(next_records_url.is_query_all());
    }

    #[test]
    fn query_response_page_exposes_valid_shared_url_continuation() {
        let page = SalesforceQueryResponsePage::from_salesforce_metadata(
            3,
            false,
            Some("/services/data/v67.0/query/01gD0000002HU6KIAW-2000"),
            vec![SalesforceRedactedRecordPayload::new(
                Some("001000000000001AAA"),
                Some("Account"),
                4,
            )],
        )
        .expect("page metadata");

        let continuation = page.next_continuation().expect("next continuation");

        assert_eq!(
            continuation,
            PaginationContinuation::UrlCursor(UrlCursorContinuation::new(
                "/services/data/v67.0/query/01gD0000002HU6KIAW-2000",
            ))
        );
        continuation
            .validate()
            .expect("Salesforce nextRecordsUrl is a valid shared continuation");
    }

    #[test]
    fn query_response_json_redacts_records_and_returns_url_continuation() {
        let page = SalesforceQueryResponsePage::from_json(&json!({
            "totalSize": 2,
            "done": false,
            "nextRecordsUrl": "/services/data/v67.0/query/01gD0000002HU6KIAW-2000",
            "records": [
                {
                    "attributes": {
                        "type": "Account",
                        "url": "/services/data/v67.0/sobjects/Account/001000000000001AAA"
                    },
                    "Id": "001000000000001AAA",
                    "Name": "Acme Medical Center",
                    "AccountNumber": "PHI-should-not-survive"
                }
            ]
        }))
        .expect("query page JSON");

        assert_eq!(page.total_size, 2);
        assert!(page.has_next_page());
        assert_eq!(
            page.next_continuation(),
            Some(PaginationContinuation::UrlCursor(
                UrlCursorContinuation::new("/services/data/v67.0/query/01gD0000002HU6KIAW-2000")
            ))
        );
        assert_eq!(
            page.records,
            vec![SalesforceRedactedRecordPayload {
                id: Some("001000000000001AAA".to_string()),
                sobject_type: Some("Account".to_string()),
                field_count: 4,
            }]
        );

        let debug_payload = format!("{page:?}");
        assert!(!debug_payload.contains("Acme Medical Center"));
        assert!(!debug_payload.contains("PHI-should-not-survive"));
    }

    #[test]
    fn query_response_json_rejects_absolute_next_records_url() {
        let error = SalesforceQueryResponsePage::from_json(&json!({
            "totalSize": 1,
            "done": false,
            "nextRecordsUrl": "https://evil.example/services/data/v67.0/query/01g",
            "records": []
        }))
        .unwrap_err();

        assert_eq!(
            error,
            SalesforceProviderError::InvalidParameter {
                name: SALESFORCE_NEXT_RECORDS_URL_FIELD,
                reason: "must be a Salesforce relative REST path, not an absolute URL"
            }
        );
    }

    #[test]
    fn updated_ids_response_parses_ids_and_latest_date() {
        let summary = SalesforceUpdatedIdsSummary::from_json(&json!({
            "ids": ["001000000000001AAA", "001000000000002AAA"],
            "latestDateCovered": "2026-06-26T00:00:00Z"
        }))
        .expect("updated ids summary");

        assert_eq!(
            summary,
            SalesforceUpdatedIdsSummary {
                ids: vec![
                    "001000000000001AAA".to_string(),
                    "001000000000002AAA".to_string(),
                ],
                latest_date_covered: "2026-06-26T00:00:00Z".to_string(),
            }
        );
    }

    #[test]
    fn updated_ids_response_rejects_malformed_ids_and_dates() {
        let bad_id = SalesforceUpdatedIdsSummary::from_json(&json!({
            "ids": ["001000000000001' OR Name != ''"],
            "latestDateCovered": "2026-06-26T00:00:00Z"
        }))
        .unwrap_err();
        let bad_date = SalesforceUpdatedIdsSummary::from_json(&json!({
            "ids": ["001000000000001AAA"],
            "latestDateCovered": "tomorrow"
        }))
        .unwrap_err();

        assert_eq!(
            bad_id,
            SalesforceProviderError::InvalidParameter {
                name: "id",
                reason: "must be a 15 or 18 character Salesforce id",
            }
        );
        assert_eq!(
            bad_date,
            SalesforceProviderError::InvalidParameter {
                name: SALESFORCE_LATEST_DATE_COVERED_FIELD,
                reason: "must be an ISO-8601 Salesforce datetime literal",
            }
        );
    }

    #[test]
    fn deleted_ids_response_parses_deleted_records_and_coverage_dates() {
        let summary = SalesforceDeletedIdsSummary::from_json(&json!({
            "deletedRecords": [
                {
                    "id": "001000000000001AAA",
                    "deletedDate": "2026-06-25T03:04:05Z"
                }
            ],
            "earliestDateAvailable": "2026-06-01T00:00:00Z",
            "latestDateCovered": "2026-06-26T00:00:00Z"
        }))
        .expect("deleted ids summary");

        assert_eq!(
            summary,
            SalesforceDeletedIdsSummary {
                deleted_records: vec![SalesforceDeletedRecordSummary {
                    id: "001000000000001AAA".to_string(),
                    deleted_date: "2026-06-25T03:04:05Z".to_string(),
                }],
                earliest_date_available: "2026-06-01T00:00:00Z".to_string(),
                latest_date_covered: "2026-06-26T00:00:00Z".to_string(),
            }
        );
    }

    #[test]
    fn deleted_ids_response_rejects_malformed_deleted_records() {
        let bad_id = SalesforceDeletedIdsSummary::from_json(&json!({
            "deletedRecords": [
                {
                    "id": "bad/id",
                    "deletedDate": "2026-06-25T03:04:05Z"
                }
            ],
            "earliestDateAvailable": "2026-06-01T00:00:00Z",
            "latestDateCovered": "2026-06-26T00:00:00Z"
        }))
        .unwrap_err();
        let bad_deleted_date = SalesforceDeletedIdsSummary::from_json(&json!({
            "deletedRecords": [
                {
                    "id": "001000000000001AAA",
                    "deletedDate": "not-a-date"
                }
            ],
            "earliestDateAvailable": "2026-06-01T00:00:00Z",
            "latestDateCovered": "2026-06-26T00:00:00Z"
        }))
        .unwrap_err();

        assert_eq!(
            bad_id,
            SalesforceProviderError::InvalidParameter {
                name: SALESFORCE_DELETED_RECORD_ID_FIELD,
                reason: "must be a 15 or 18 character Salesforce id",
            }
        );
        assert_eq!(
            bad_deleted_date,
            SalesforceProviderError::InvalidParameter {
                name: SALESFORCE_DELETED_DATE_FIELD,
                reason: "must be an ISO-8601 Salesforce datetime literal",
            }
        );
    }

    #[test]
    fn describe_object_summary_accepts_required_account_fields() {
        let summary = SalesforceDescribeObjectSummary::from_json_for_projection(
            &json!({
                "name": "Account",
                "fields": [
                    { "name": "Id", "deprecatedAndHidden": false },
                    { "name": "Name", "deprecatedAndHidden": false },
                    { "name": "AccountNumber", "deprecatedAndHidden": false },
                    { "name": "SystemModstamp", "deprecatedAndHidden": false }
                ]
            }),
            SalesforceObject::Account,
            &[
                SalesforceField::AccountId,
                SalesforceField::AccountName,
                SalesforceField::AccountNumber,
            ],
        )
        .expect("describe summary");

        assert_eq!(
            summary,
            SalesforceDescribeObjectSummary {
                object_name: "Account".to_string(),
                field_count: 4,
                required_field_count: 3,
            }
        );
    }

    #[test]
    fn describe_object_summary_fails_closed_when_account_field_missing() {
        let error = SalesforceDescribeObjectSummary::from_json_for_projection(
            &json!({
                "name": "Account",
                "fields": [
                    { "name": "Id", "deprecatedAndHidden": false },
                    { "name": "Name", "deprecatedAndHidden": false },
                    { "name": "AccountNumber", "deprecatedAndHidden": true }
                ]
            }),
            SalesforceObject::Account,
            &[
                SalesforceField::AccountId,
                SalesforceField::AccountName,
                SalesforceField::AccountNumber,
            ],
        )
        .unwrap_err();

        assert_eq!(
            error,
            SalesforceProviderError::InvalidParameter {
                name: SALESFORCE_DESCRIBE_FIELDS_FIELD,
                reason: "required field is missing or hidden",
            }
        );
    }

    #[test]
    fn fetch_limits_summary_extracts_daily_api_remaining() {
        let summary = SalesforceLimitsSummary::from_json(&json!({
            "DailyApiRequests": {
                "Max": 5000,
                "Remaining": 4982
            },
            "DailyAsyncApexExecutions": {
                "Max": 250000,
                "Remaining": 249999
            }
        }))
        .expect("limits summary");

        let daily_api = summary.daily_api_requests.expect("daily API limit");
        assert_eq!(
            daily_api,
            SalesforceOrgLimit {
                max: 5000,
                remaining: 4982,
            }
        );
        assert_eq!(daily_api.used(), 18);
    }

    #[test]
    fn fetch_limits_summary_rejects_remaining_above_max() {
        let error = SalesforceLimitsSummary::from_json(&json!({
            "DailyApiRequests": {
                "Max": 5000,
                "Remaining": 5001
            }
        }))
        .unwrap_err();

        assert_eq!(
            error,
            SalesforceProviderError::InvalidParameter {
                name: SALESFORCE_DAILY_API_REQUESTS_LIMIT,
                reason: "remaining limit must not exceed max",
            }
        );
    }

    #[test]
    fn rate_limit_signals_do_not_expose_header_values() {
        let summary = SalesforceRateLimitSummary::from_response_parts(
            403,
            Some(REQUEST_LIMIT_EXCEEDED_ERROR_CODE),
            [
                ("Sforce-Limit-Info", "api-usage=100/100"),
                ("Retry-After", "5"),
                ("Authorization", "Bearer raw-token"),
            ],
        );

        assert_eq!(summary.error_class, SalesforceRestErrorClass::RateLimited);
        assert!(summary.retryable);
        assert_eq!(summary.signal.retry_after_ms, Some(5_000));
        assert_eq!(summary.signal.api_usage.as_ref().unwrap().remaining(), 0);

        let debug_summary = format!("{summary:?}");
        assert!(!debug_summary.contains("api-usage=100/100"));
        assert!(!debug_summary.contains("Bearer raw-token"));
    }

    #[test]
    fn rate_limit_summary_honors_retry_after_in_retry_decision() {
        let summary = SalesforceRateLimitSummary::from_response_parts(
            403,
            Some(REQUEST_LIMIT_EXCEEDED_ERROR_CODE),
            [
                ("Retry-After", "5"),
                ("Sforce-Limit-Info", "api-usage=100/100"),
            ],
        );

        assert_eq!(
            summary
                .retry_decision(1, &appfw_saas_core::RetryPolicy::default())
                .expect("retry decision"),
            appfw_saas_core::RetryDecision {
                should_retry: true,
                delay_ms: 5_000,
            }
        );
    }

    #[test]
    fn rate_limit_summary_stops_at_retry_attempt_budget() {
        let summary = SalesforceRateLimitSummary::from_response_parts(
            429,
            None,
            [("Sforce-Limit-Info", "api-usage=99/100")],
        );

        assert_eq!(
            summary
                .retry_decision(3, &appfw_saas_core::RetryPolicy::default())
                .expect("retry decision"),
            appfw_saas_core::RetryDecision {
                should_retry: false,
                delay_ms: 0,
            }
        );
    }

    #[test]
    fn rejects_absolute_next_records_url() {
        let error =
            SalesforceNextRecordsUrl::parse("https://evil.example/services/data/v67.0/query/01g")
                .unwrap_err();

        assert_eq!(
            error,
            SalesforceProviderError::InvalidParameter {
                name: SALESFORCE_NEXT_RECORDS_URL_FIELD,
                reason: "must be a Salesforce relative REST path, not an absolute URL"
            }
        );
    }

    #[test]
    fn rejects_inconsistent_query_page_pagination_metadata() {
        let missing_next =
            SalesforceQueryResponsePage::from_salesforce_metadata(10, false, None, Vec::new())
                .unwrap_err();
        let unexpected_next = SalesforceQueryResponsePage::from_salesforce_metadata(
            10,
            true,
            Some("/services/data/v67.0/query/01gD0000002HU6KIAW-2000"),
            Vec::new(),
        )
        .unwrap_err();

        assert_eq!(
            missing_next,
            SalesforceProviderError::InvalidParameter {
                name: SALESFORCE_NEXT_RECORDS_URL_FIELD,
                reason: "must be present when done is false"
            }
        );
        assert_eq!(
            unexpected_next,
            SalesforceProviderError::InvalidParameter {
                name: SALESFORCE_NEXT_RECORDS_URL_FIELD,
                reason: "must be absent when done is true"
            }
        );
    }

    #[test]
    fn parses_sforce_limit_info_api_usage() {
        let limit_info = SalesforceLimitInfo::parse("api-usage=18/5000").expect("limit header");

        assert_eq!(
            limit_info.api_usage,
            Some(SalesforceApiUsageLimit {
                used: 18,
                allowed: 5000
            })
        );
        assert_eq!(limit_info.api_usage.expect("api usage").remaining(), 4982);
    }

    #[test]
    fn classifies_request_limit_exceeded_as_rate_limited() {
        let error = SalesforceRestError::new(403, Some(REQUEST_LIMIT_EXCEEDED_ERROR_CODE));

        assert_eq!(error.classify(), SalesforceRestErrorClass::RateLimited);
        assert!(error.classify().is_retryable());
    }

    #[test]
    fn classifies_http_429_as_rate_limited_without_error_code() {
        let error = SalesforceRestError::new(429, None);

        assert_eq!(error.classify(), SalesforceRestErrorClass::RateLimited);
        assert!(error.classify().is_retryable());
    }

    #[test]
    fn classifies_documented_rest_statuses() {
        assert_eq!(
            classify_salesforce_rest_error(401, Some("INVALID_SESSION_ID")),
            SalesforceRestErrorClass::Authentication
        );
        assert_eq!(
            classify_salesforce_rest_error(414, None),
            SalesforceRestErrorClass::RequestTooLarge
        );
        assert_eq!(
            classify_salesforce_rest_error(503, None),
            SalesforceRestErrorClass::Transient
        );
        assert_eq!(
            classify_salesforce_rest_error(412, None),
            SalesforceRestErrorClass::ConditionalRequest
        );
    }
}
