use std::{collections::HashSet, env};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::Sha256;

use crate::{
    model_metadata::{RuntimeDataType, RuntimePropertyMetadata},
    query_filter::{conjunction_token, filter_token},
    RuntimeError, RuntimeFilterOp,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimePagination {
    pub skip: i32,
    pub limit: i32,
    pub strategy: RuntimePaginationStrategy,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimePaginationPolicy {
    pub default_page_size: i32,
    pub max_page_size: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimePaginationStrategy {
    Offset,
    Keyset { after: Option<String> },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeSortDirection {
    Asc,
    Desc,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RuntimeKeysetCursor {
    pub field: String,
    pub direction: String,
    pub value: Value,
    /// Primary-key tiebreaker carried alongside the sort value. Required whenever the sort
    /// field is not itself the primary key (i.e. non-unique columns such as `created_at`) so
    /// that keyset pagination cannot skip or duplicate rows that share the same sort value.
    /// `None` when the sort field IS the primary key (already unique).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tiebreaker: Option<RuntimeKeysetTiebreaker>,
}

/// Primary-key tiebreaker component of a keyset cursor.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RuntimeKeysetTiebreaker {
    pub field: String,
    pub value: Value,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeAggregateFunction {
    Count,
    CountDistinct,
    Sum,
    Avg,
    Min,
    Max,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeAggregateFieldDescriptor {
    pub name: String,
    pub data_type: RuntimeDataType,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeScalarFilterPredicate {
    pub op: RuntimeFilterOp,
    pub value: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeSortSpecInput {
    pub field: String,
    pub direction: RuntimeSortDirection,
}

impl RuntimeAggregateFieldDescriptor {
    pub fn new(name: impl Into<String>, data_type: RuntimeDataType) -> Self {
        Self {
            name: name.into(),
            data_type,
        }
    }
}

impl From<&RuntimePropertyMetadata> for RuntimeAggregateFieldDescriptor {
    fn from(property: &RuntimePropertyMetadata) -> Self {
        Self::new(property.name.clone(), property.data_type)
    }
}

pub fn normalize_object_input(
    input: Option<Value>,
    label: &str,
) -> Result<Option<Map<String, Value>>, RuntimeError> {
    match input {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Object(obj)) => Ok(if obj.is_empty() { None } else { Some(obj) }),
        Some(Value::String(value)) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            match serde_json::from_str::<Value>(trimmed) {
                Ok(Value::Object(obj)) => Ok(if obj.is_empty() { None } else { Some(obj) }),
                Ok(other) => Err(RuntimeError::Validation(format!(
                    "{} must be a JSON object or a JSON-encoded object string, got {}",
                    label,
                    value_kind(&other)
                ))),
                Err(error) => Err(RuntimeError::Validation(format!(
                    "{} string is not valid JSON: {}",
                    label, error
                ))),
            }
        }
        Some(other) => Err(RuntimeError::Validation(format!(
            "{} must be a JSON object or a JSON-encoded object string, got {}",
            label,
            value_kind(&other)
        ))),
    }
}

pub fn normalize_array_input(
    input: Option<Value>,
    label: &str,
) -> Result<Option<Vec<Value>>, RuntimeError> {
    match input {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(items)) => Ok(Some(items)),
        Some(Value::String(value)) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            if trimmed.starts_with('[') {
                match serde_json::from_str::<Value>(trimmed) {
                    Ok(Value::Array(items)) => Ok(Some(items)),
                    Ok(other) => Err(RuntimeError::Validation(format!(
                        "{} must be a JSON array or a JSON-encoded array string, got {}",
                        label,
                        value_kind(&other)
                    ))),
                    Err(error) => Err(RuntimeError::Validation(format!(
                        "{} string is not valid JSON: {}",
                        label, error
                    ))),
                }
            } else {
                Ok(Some(vec![Value::String(trimmed.to_string())]))
            }
        }
        Some(other) => Err(RuntimeError::Validation(format!(
            "{} must be a JSON array or a JSON-encoded array string, got {}",
            label,
            value_kind(&other)
        ))),
    }
}

pub fn take_string(
    obj: &mut Map<String, Value>,
    key: &str,
    label: &str,
) -> Result<String, RuntimeError> {
    obj.remove(key)
        .ok_or_else(|| RuntimeError::Validation(format!("{} requires '{}'", label, key)))
        .and_then(|value| expect_string(value, &format!("{}.{}", label, key)))
}

pub fn take_string_any(
    obj: &mut Map<String, Value>,
    keys: &[&str],
    label: &str,
) -> Result<String, RuntimeError> {
    for key in keys {
        if let Some(value) = obj.remove(*key) {
            return expect_string(value, label);
        }
    }
    Err(RuntimeError::Validation(format!(
        "{} requires one of: {}",
        label,
        keys.join(", ")
    )))
}

pub fn expect_string(value: Value, label: &str) -> Result<String, RuntimeError> {
    value
        .as_str()
        .map(ToString::to_string)
        .ok_or_else(|| RuntimeError::Validation(format!("{} must be a string", label)))
}

pub fn normalize_filter_conjunction_items(
    key: &str,
    value: Value,
) -> Result<Vec<Map<String, Value>>, RuntimeError> {
    let Value::Array(items) = value else {
        return Err(RuntimeError::Validation(format!(
            "filter conjunction '{}' expects an array",
            key
        )));
    };

    let mut normalized = Vec::with_capacity(items.len());
    for item in items {
        let Value::Object(obj) = item else {
            return Err(RuntimeError::Validation(format!(
                "filter conjunction '{}' expects object items",
                key
            )));
        };
        normalized.push(obj);
    }
    Ok(normalized)
}

pub fn expect_relationship_filter_object(
    relationship_kind: &str,
    field_name: &str,
    value: Value,
) -> Result<Map<String, Value>, RuntimeError> {
    match value {
        Value::Object(obj) => Ok(obj),
        _ => Err(RuntimeError::Validation(format!(
            "{} filter '{}' expects an object",
            relationship_kind, field_name
        ))),
    }
}

pub fn scalar_filter_predicates(
    field_name: &str,
    value: Value,
) -> Result<Vec<RuntimeScalarFilterPredicate>, RuntimeError> {
    match value {
        Value::Object(obj) => {
            if obj.len() == 1 && obj.contains_key("$oid") {
                return Ok(vec![RuntimeScalarFilterPredicate {
                    op: RuntimeFilterOp::Eq,
                    value: Value::Object(obj),
                }]);
            }
            if obj.is_empty() {
                return Err(RuntimeError::Validation(format!(
                    "filter operator object for '{}' must not be empty",
                    field_name
                )));
            }

            obj.into_iter()
                .map(|(op_name, value)| {
                    Ok(RuntimeScalarFilterPredicate {
                        op: RuntimeFilterOp::from_token(&op_name)?,
                        value,
                    })
                })
                .collect()
        }
        value => Ok(vec![RuntimeScalarFilterPredicate {
            op: RuntimeFilterOp::Eq,
            value,
        }]),
    }
}

pub fn parse_sort_specs(input: Option<Value>) -> Result<Vec<RuntimeSortSpecInput>, RuntimeError> {
    let Some(obj) = normalize_object_input(input, "sort")? else {
        return Ok(Vec::new());
    };

    Ok(obj
        .into_iter()
        .map(|(field, value)| RuntimeSortSpecInput {
            field,
            direction: RuntimeSortDirection::from_value(&value),
        })
        .collect())
}

impl RuntimePagination {
    pub fn new(skip: i32, limit: i32) -> Result<Self, RuntimeError> {
        RuntimePaginationPolicy::from_env().validate(skip, limit)?;
        Ok(Self {
            skip,
            limit,
            strategy: RuntimePaginationStrategy::Offset,
        })
    }

    pub fn keyset(after: Option<String>, limit: i32) -> Result<Self, RuntimeError> {
        RuntimePaginationPolicy::from_env().validate(0, limit)?;
        Ok(Self {
            skip: 0,
            limit,
            strategy: RuntimePaginationStrategy::Keyset { after },
        })
    }

    pub fn is_keyset(&self) -> bool {
        matches!(self.strategy, RuntimePaginationStrategy::Keyset { .. })
    }

    pub fn cursor_after(&self) -> Option<String> {
        match &self.strategy {
            RuntimePaginationStrategy::Offset => None,
            RuntimePaginationStrategy::Keyset { after } => after.clone(),
        }
    }
}

impl RuntimePaginationPolicy {
    pub const DEFAULT_PAGE_SIZE: i32 = 50;
    pub const DEFAULT_MAX_PAGE_SIZE: i32 = 250;

    pub fn from_env() -> Self {
        let max_page_size = env_i32("APP_QUERY_MAX_PAGE_SIZE", Self::DEFAULT_MAX_PAGE_SIZE, 1);
        let default_page_size =
            env_i32("APP_QUERY_DEFAULT_PAGE_SIZE", Self::DEFAULT_PAGE_SIZE, 1).min(max_page_size);

        Self {
            default_page_size,
            max_page_size,
        }
    }

    pub fn normalize(
        &self,
        skip: Option<i32>,
        limit: Option<i32>,
    ) -> Result<(i32, i32), RuntimeError> {
        let skip = skip.unwrap_or(0);
        let limit = limit.unwrap_or(self.default_page_size);
        self.validate(skip, limit)?;
        Ok((skip, limit))
    }

    pub fn validate(&self, skip: i32, limit: i32) -> Result<(), RuntimeError> {
        if skip < 0 {
            return Err(RuntimeError::Validation(
                "pagination skip must be greater than or equal to 0".to_string(),
            ));
        }
        if limit <= 0 {
            return Err(RuntimeError::Validation(
                "pagination limit must be greater than 0".to_string(),
            ));
        }
        if limit > self.max_page_size {
            return Err(RuntimeError::Validation(format!(
                "pagination limit {} exceeds maximum page size {}",
                limit, self.max_page_size
            )));
        }
        Ok(())
    }
}

impl RuntimeSortDirection {
    pub fn from_value(value: &Value) -> Self {
        match value.as_str().map(|value| value.to_ascii_lowercase()) {
            Some(value) if value == "desc" => Self::Desc,
            _ => Self::Asc,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        }
    }
}

impl RuntimeAggregateFunction {
    pub fn from_name(value: &str) -> Result<Self, RuntimeError> {
        match value.to_ascii_lowercase().as_str() {
            "count" => Ok(Self::Count),
            "count_distinct" | "countdistinct" | "count-distinct" => Ok(Self::CountDistinct),
            "sum" => Ok(Self::Sum),
            "avg" | "average" => Ok(Self::Avg),
            "min" => Ok(Self::Min),
            "max" => Ok(Self::Max),
            other => Err(RuntimeError::Validation(format!(
                "unknown aggregate function '{}'",
                other
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Count => "count",
            Self::CountDistinct => "count_distinct",
            Self::Sum => "sum",
            Self::Avg => "avg",
            Self::Min => "min",
            Self::Max => "max",
        }
    }
}

pub fn ensure_keyset_sort(
    primary_key_name: &str,
    sort: Option<Value>,
) -> Result<Value, RuntimeError> {
    let Some(sort) = sort else {
        if primary_key_name.trim().is_empty() {
            return Err(RuntimeError::Validation(
                "keyset pagination requires a primary key sort field".to_string(),
            ));
        }
        return Ok(json!({ primary_key_name: RuntimeSortDirection::Asc.as_str() }));
    };

    let obj = normalize_sort_object(sort)?;
    if obj.len() != 1 {
        return Err(RuntimeError::Validation(
            "keyset pagination currently supports exactly one sort field".to_string(),
        ));
    }

    // Keyset pagination is only correct when the ORDER BY is over a unique key. A single
    // non-unique sort column (e.g. `created_at`) can skip or duplicate rows across pages when
    // multiple rows share the same value. Append the primary key as a deterministic tiebreaker
    // so the effective sort is `(sort_field, pk)`, which is unique. When the caller already
    // sorts by the primary key the sort is unique and no tiebreaker is needed.
    let (sort_field, _) = obj
        .iter()
        .next()
        .expect("normalized sort object has exactly one entry");
    if sort_field == primary_key_name || primary_key_name.trim().is_empty() {
        return Ok(Value::Object(obj));
    }

    let mut sort_with_tiebreaker = obj;
    // Match the tiebreaker direction to the primary sort direction for index-friendly ordering.
    let tiebreaker_direction = sort_with_tiebreaker
        .values()
        .next()
        .map(RuntimeSortDirection::from_value)
        .unwrap_or(RuntimeSortDirection::Asc);
    sort_with_tiebreaker.insert(
        primary_key_name.to_string(),
        Value::String(tiebreaker_direction.as_str().to_string()),
    );
    Ok(Value::Object(sort_with_tiebreaker))
}

pub fn apply_keyset_cursor_filter(
    filter: Option<Value>,
    sort: &Value,
    after: Option<&str>,
) -> Result<Option<Value>, RuntimeError> {
    let Some(after) = after else {
        return Ok(filter);
    };
    let cursor = decode_keyset_cursor(after)?;
    let sort_obj = sort.as_object().ok_or_else(|| {
        RuntimeError::Validation("keyset pagination requires an object sort".to_string())
    })?;
    let (field, direction_value) = sort_obj.iter().next().ok_or_else(|| {
        RuntimeError::Validation("keyset pagination requires a sort field".to_string())
    })?;
    let direction = RuntimeSortDirection::from_value(direction_value);
    if cursor.field != *field || cursor.direction != direction.as_str() {
        return Err(RuntimeError::Validation(
            "keyset cursor does not match the requested sort".to_string(),
        ));
    }

    let strict_op = match direction {
        RuntimeSortDirection::Asc => filter_token::GREATER_THAN,
        RuntimeSortDirection::Desc => filter_token::LESS_THAN,
    };

    let cursor_filter = match &cursor.tiebreaker {
        // Sort field is unique (it is the primary key): a strict comparison is sufficient.
        None => json!({ field: { strict_op: cursor.value } }),
        // Sort field is non-unique: build the standard keyset predicate so rows sharing the
        // sort value are not skipped or duplicated across pages:
        //   (sort > v) OR (sort = v AND pk > pk_v)
        Some(tiebreaker) => json!({
            conjunction_token::OR: [
                { field: { strict_op: cursor.value.clone() } },
                {
                    conjunction_token::AND: [
                        { field: { filter_token::EQUALS: cursor.value.clone() } },
                        { tiebreaker.field.clone(): { strict_op: tiebreaker.value.clone() } }
                    ]
                }
            ]
        }),
    };

    Ok(match filter {
        None | Some(Value::Null) => Some(cursor_filter),
        Some(existing) => Some(json!({ conjunction_token::AND: [existing, cursor_filter] })),
    })
}

pub fn encode_keyset_cursor(
    field: &str,
    direction: RuntimeSortDirection,
    value: Value,
) -> Result<String, RuntimeError> {
    encode_keyset_cursor_struct(&RuntimeKeysetCursor {
        field: field.to_string(),
        direction: direction.as_str().to_string(),
        value,
        tiebreaker: None,
    })
}

/// Encode a keyset cursor that carries a primary-key tiebreaker. Use this whenever the sort
/// field is not the primary key so that pagination over non-unique columns stays correct.
pub fn encode_keyset_cursor_with_tiebreaker(
    field: &str,
    direction: RuntimeSortDirection,
    value: Value,
    tiebreaker_field: &str,
    tiebreaker_value: Value,
) -> Result<String, RuntimeError> {
    encode_keyset_cursor_struct(&RuntimeKeysetCursor {
        field: field.to_string(),
        direction: direction.as_str().to_string(),
        value,
        tiebreaker: Some(RuntimeKeysetTiebreaker {
            field: tiebreaker_field.to_string(),
            value: tiebreaker_value,
        }),
    })
}

fn encode_keyset_cursor_struct(cursor: &RuntimeKeysetCursor) -> Result<String, RuntimeError> {
    let payload = serde_json::to_vec(cursor)
        .map_err(|error| RuntimeError::DataAccess(format!("failed to encode cursor: {error}")))?;
    let key = cursor_signing_key()?;
    let signature = cursor_hmac(&key, &payload);

    // Opaque, tamper-evident wire format: base64url(payload) "." base64url(hmac_sha256).
    let payload_b64 = URL_SAFE_NO_PAD.encode(&payload);
    let signature_b64 = URL_SAFE_NO_PAD.encode(signature);
    Ok(format!("{payload_b64}.{signature_b64}"))
}

pub fn decode_keyset_cursor(value: &str) -> Result<RuntimeKeysetCursor, RuntimeError> {
    let (payload_b64, signature_b64) = value
        .split_once('.')
        .ok_or_else(|| RuntimeError::Validation("invalid keyset cursor".to_string()))?;

    let payload = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|_| RuntimeError::Validation("invalid keyset cursor".to_string()))?;
    let signature = URL_SAFE_NO_PAD
        .decode(signature_b64)
        .map_err(|_| RuntimeError::Validation("invalid keyset cursor".to_string()))?;

    // Verify the signature before trusting the payload. Reject tampered/forged cursors.
    let key = cursor_signing_key()?;
    let expected = cursor_hmac(&key, &payload);
    if !constant_time_eq(&expected, &signature) {
        return Err(RuntimeError::Validation(
            "invalid keyset cursor".to_string(),
        ));
    }

    let cursor: RuntimeKeysetCursor = serde_json::from_slice(&payload)
        .map_err(|_| RuntimeError::Validation("invalid keyset cursor".to_string()))?;
    if cursor.direction != RuntimeSortDirection::Asc.as_str()
        && cursor.direction != RuntimeSortDirection::Desc.as_str()
    {
        return Err(RuntimeError::Validation(
            "invalid keyset cursor".to_string(),
        ));
    }
    Ok(cursor)
}

/// Dev-only default signing key. Only used when ENV_NAME is `local` or unset (developer
/// workstations / tests); managed environments MUST set APP_CURSOR_SIGNING_KEY.
const DEV_CURSOR_SIGNING_KEY: &str = "appfw-local-dev-cursor-signing-key-do-not-use-in-prod";
const CURSOR_SIGNING_KEY_ENV: &str = "APP_CURSOR_SIGNING_KEY";

fn cursor_signing_key() -> Result<Vec<u8>, RuntimeError> {
    let configured = env::var(CURSOR_SIGNING_KEY_ENV).ok();
    let env_name = env::var("ENV_NAME").ok();
    resolve_cursor_signing_key(configured.as_deref(), env_name.as_deref())
}

/// Pure resolution of the cursor signing key from the two relevant env values. Kept side-effect
/// free so it can be unit tested without mutating process-global environment variables.
///
/// A "managed" environment is any deployment whose ENV_NAME is set to something other than
/// `local`. When ENV_NAME is unset (developer workstations / unit tests) we treat the context as
/// local and permit the dev default key. Any real deployment (compose/staging/prod) sets
/// ENV_NAME, which forces an explicit signing key — fail-closed for healthcare/PHI contexts.
fn resolve_cursor_signing_key(
    configured_key: Option<&str>,
    env_name: Option<&str>,
) -> Result<Vec<u8>, RuntimeError> {
    if let Some(key) = configured_key {
        if !key.trim().is_empty() {
            return Ok(key.as_bytes().to_vec());
        }
    }

    let managed = match env_name {
        Some(value) => !value.trim().eq_ignore_ascii_case("local"),
        None => false,
    };
    if managed {
        Err(RuntimeError::Validation(format!(
            "{CURSOR_SIGNING_KEY_ENV} must be set in managed environments"
        )))
    } else {
        Ok(DEV_CURSOR_SIGNING_KEY.as_bytes().to_vec())
    }
}

fn cursor_hmac(key: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts keys of any length");
    mac.update(payload);
    mac.finalize().into_bytes().to_vec()
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

pub fn validate_aggregate_output_alias(alias: &str) -> Result<(), RuntimeError> {
    let mut chars = alias.chars();
    let Some(first) = chars.next() else {
        return Err(RuntimeError::Validation(
            "aggregate output alias must not be empty".to_string(),
        ));
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return Err(RuntimeError::Validation(format!(
            "aggregate output alias '{}' must start with a letter or underscore",
            alias
        )));
    }
    if !chars.all(|c| c == '_' || c.is_ascii_alphanumeric()) {
        return Err(RuntimeError::Validation(format!(
            "aggregate output alias '{}' may contain only letters, numbers, and underscores",
            alias
        )));
    }
    Ok(())
}

pub fn validate_unique_aggregate_aliases<I, S>(aliases: I) -> Result<(), RuntimeError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut seen = HashSet::new();
    for alias in aliases {
        let alias = alias.as_ref();
        if !seen.insert(alias.to_string()) {
            return Err(RuntimeError::Validation(format!(
                "duplicate aggregate output alias '{}'",
                alias
            )));
        }
    }
    Ok(())
}

pub fn aggregate_alias_exists<I, S>(aliases: I, alias: &str) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    aliases
        .into_iter()
        .any(|candidate| candidate.as_ref() == alias)
}

pub fn ensure_aggregate_groupable(
    field: &RuntimeAggregateFieldDescriptor,
) -> Result<(), RuntimeError> {
    if field.data_type.is_scalar_aggregate() {
        Ok(())
    } else {
        Err(RuntimeError::Validation(format!(
            "property '{}' with type {:?} cannot be used in group_by",
            field.name, field.data_type
        )))
    }
}

pub fn validate_aggregate_metric_field(
    function: RuntimeAggregateFunction,
    field: Option<&RuntimeAggregateFieldDescriptor>,
) -> Result<(), RuntimeError> {
    match function {
        RuntimeAggregateFunction::Count => {
            if let Some(field) = field {
                ensure_scalar_metric(field)?;
            }
        }
        RuntimeAggregateFunction::CountDistinct => {
            let field = field.ok_or_else(|| {
                RuntimeError::Validation("count_distinct metrics require a field".to_string())
            })?;
            ensure_scalar_metric(field)?;
        }
        RuntimeAggregateFunction::Sum | RuntimeAggregateFunction::Avg => {
            let field = field.ok_or_else(|| {
                RuntimeError::Validation(format!(
                    "{} metrics require a numeric field",
                    function.as_str()
                ))
            })?;
            if !field.data_type.is_numeric() {
                return Err(RuntimeError::Validation(format!(
                    "{} metric field '{}' must be numeric",
                    function.as_str(),
                    field.name
                )));
            }
        }
        RuntimeAggregateFunction::Min | RuntimeAggregateFunction::Max => {
            let field = field.ok_or_else(|| {
                RuntimeError::Validation(format!("{} metrics require a field", function.as_str()))
            })?;
            ensure_scalar_metric(field)?;
        }
    }
    Ok(())
}

pub fn default_aggregate_metric_alias(
    function: RuntimeAggregateFunction,
    field_name: Option<&str>,
) -> String {
    match (function, field_name) {
        (RuntimeAggregateFunction::Count, None) => "count".to_string(),
        (function, Some(field_name)) => format!("{}_{}", function.as_str(), field_name),
        (function, None) => function.as_str().to_string(),
    }
}

pub fn ensure_aggregate_having_op(op: RuntimeFilterOp) -> Result<(), RuntimeError> {
    if matches!(
        op,
        RuntimeFilterOp::Eq
            | RuntimeFilterOp::Ne
            | RuntimeFilterOp::Lt
            | RuntimeFilterOp::Lte
            | RuntimeFilterOp::Gt
            | RuntimeFilterOp::Gte
            | RuntimeFilterOp::In
            | RuntimeFilterOp::NotIn
    ) {
        Ok(())
    } else {
        Err(RuntimeError::Validation(format!(
            "operator '{}' is not supported in aggregate having",
            op.as_filter_token()
        )))
    }
}

fn ensure_scalar_metric(field: &RuntimeAggregateFieldDescriptor) -> Result<(), RuntimeError> {
    if field.data_type.is_scalar_aggregate() {
        Ok(())
    } else {
        Err(RuntimeError::Validation(format!(
            "property '{}' with type {:?} cannot be used in aggregate metrics",
            field.name, field.data_type
        )))
    }
}

fn normalize_sort_object(value: Value) -> Result<serde_json::Map<String, Value>, RuntimeError> {
    match value {
        Value::Object(obj) if !obj.is_empty() => Ok(obj),
        Value::Object(_) => Err(RuntimeError::Validation(
            "keyset pagination requires a sort".to_string(),
        )),
        Value::String(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Err(RuntimeError::Validation(
                    "keyset pagination requires a sort".to_string(),
                ));
            }
            match serde_json::from_str::<Value>(trimmed) {
                Ok(Value::Object(obj)) if !obj.is_empty() => Ok(obj),
                Ok(Value::Object(_)) => Err(RuntimeError::Validation(
                    "keyset pagination requires a sort".to_string(),
                )),
                Ok(other) => Err(RuntimeError::Validation(format!(
                    "keyset sort must be a JSON object or a JSON-encoded object string, got {}",
                    value_kind(&other)
                ))),
                Err(error) => Err(RuntimeError::Validation(format!(
                    "keyset sort string is not valid JSON: {error}"
                ))),
            }
        }
        other => Err(RuntimeError::Validation(format!(
            "keyset sort must be a JSON object or a JSON-encoded object string, got {}",
            value_kind(&other)
        ))),
    }
}

pub fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn env_i32(name: &str, default: i32, min: i32) -> i32 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<i32>().ok())
        .filter(|value| *value >= min)
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Cursor encode/decode in these tests relies on the dev-default signing key, which applies
    // whenever ENV_NAME is unset (the case for the test binary). The managed-env gate is tested
    // via the side-effect-free `resolve_cursor_signing_key` so no test mutates global env vars,
    // keeping cursor signatures stable across cargo's parallel test threads.

    #[test]
    fn pagination_policy_applies_defaults_and_maximums() {
        let policy = RuntimePaginationPolicy {
            default_page_size: 25,
            max_page_size: 100,
        };

        assert_eq!(policy.normalize(None, None).expect("defaults"), (0, 25));
        assert_eq!(
            policy
                .normalize(Some(5), Some(50))
                .expect("explicit pagination"),
            (5, 50)
        );
        assert!(policy.normalize(Some(-1), Some(10)).is_err());
        assert!(policy.normalize(Some(0), Some(101)).is_err());
    }

    #[test]
    fn pagination_strategy_tracks_offset_and_keyset_modes() {
        let offset = RuntimePagination {
            skip: 5,
            limit: 25,
            strategy: RuntimePaginationStrategy::Offset,
        };
        assert!(!offset.is_keyset());
        assert_eq!(offset.cursor_after(), None);

        let keyset = RuntimePagination {
            skip: 0,
            limit: 25,
            strategy: RuntimePaginationStrategy::Keyset {
                after: Some("cursor".to_string()),
            },
        };
        assert!(keyset.is_keyset());
        assert_eq!(keyset.cursor_after(), Some("cursor".to_string()));
    }

    #[test]
    fn keyset_sort_defaults_to_primary_key() {
        assert_eq!(
            ensure_keyset_sort("id", None).expect("default sort"),
            json!({ "id": "asc" })
        );
    }

    #[test]
    fn keyset_sort_requires_exactly_one_field() {
        assert!(ensure_keyset_sort("id", Some(json!({}))).is_err());
        assert!(ensure_keyset_sort("id", Some(json!({ "id": "asc", "name": "asc" }))).is_err());
    }

    #[test]
    fn sort_specs_parse_fields_and_directions() {
        let specs = parse_sort_specs(Some(json!({
            "created_at": "desc",
            "name": "asc"
        })))
        .expect("sort specs");

        assert_eq!(
            specs,
            vec![
                RuntimeSortSpecInput {
                    field: "created_at".to_string(),
                    direction: RuntimeSortDirection::Desc,
                },
                RuntimeSortSpecInput {
                    field: "name".to_string(),
                    direction: RuntimeSortDirection::Asc,
                }
            ]
        );

        assert!(parse_sort_specs(Some(json!(["name"]))).is_err());
    }

    #[test]
    fn keyset_cursor_filter_merges_existing_filter() {
        let cursor = encode_keyset_cursor("id", RuntimeSortDirection::Asc, json!("account-2"))
            .expect("cursor");
        let filter = apply_keyset_cursor_filter(
            Some(json!({ "name": { "_starts": "A" } })),
            &json!({ "id": "asc" }),
            Some(&cursor),
        )
        .expect("cursor filter");

        assert_eq!(
            filter,
            Some(json!({
                "_and": [
                    { "name": { "_starts": "A" } },
                    { "id": { "_gt": "account-2" } }
                ]
            }))
        );
    }

    #[test]
    fn keyset_cursor_filter_uses_less_than_for_desc_sort() {
        let cursor = encode_keyset_cursor("created_at", RuntimeSortDirection::Desc, json!(100))
            .expect("cursor");
        let filter =
            apply_keyset_cursor_filter(None, &json!({ "created_at": "desc" }), Some(&cursor))
                .expect("cursor filter");

        assert_eq!(filter, Some(json!({ "created_at": { "_lt": 100 } })));
    }

    #[test]
    fn keyset_cursor_must_match_requested_sort() {
        let cursor = encode_keyset_cursor("id", RuntimeSortDirection::Asc, json!("account-2"))
            .expect("cursor");
        let err = apply_keyset_cursor_filter(None, &json!({ "name": "asc" }), Some(&cursor))
            .expect_err("cursor should not match sort");

        assert!(err.to_string().contains("cursor does not match"));
    }

    #[test]
    fn keyset_cursor_decode_rejects_invalid_payloads() {
        assert!(decode_keyset_cursor("not-json").is_err());
        assert!(
            decode_keyset_cursor(r#"{"field":"id","direction":"sideways","value":1}"#).is_err()
        );
    }

    #[test]
    fn keyset_sort_appends_primary_key_tiebreaker_for_non_unique_field() {
        // A non-unique sort column (created_at) must gain the primary key as a tiebreaker so
        // keyset pagination cannot skip or duplicate rows that share a created_at value.
        let sort = ensure_keyset_sort("id", Some(json!({ "created_at": "desc" })))
            .expect("sort with tiebreaker");
        let obj = sort.as_object().expect("object sort");
        let keys: Vec<&String> = obj.keys().collect();
        assert_eq!(keys, vec![&"created_at".to_string(), &"id".to_string()]);
        assert_eq!(obj["created_at"], json!("desc"));
        // Tiebreaker direction matches the primary sort direction for index-friendly ordering.
        assert_eq!(obj["id"], json!("desc"));
    }

    #[test]
    fn keyset_sort_leaves_primary_key_sort_untouched() {
        // When the sort field already IS the primary key, the sort is unique; no tiebreaker.
        let sort = ensure_keyset_sort("id", Some(json!({ "id": "asc" }))).expect("pk sort");
        assert_eq!(sort, json!({ "id": "asc" }));
    }

    #[test]
    fn keyset_cursor_filter_uses_compound_predicate_with_tiebreaker() {
        // (created_at < v) OR (created_at = v AND id < pk_v) for a desc sort.
        let cursor = encode_keyset_cursor_with_tiebreaker(
            "created_at",
            RuntimeSortDirection::Desc,
            json!("2026-01-01"),
            "id",
            json!("account-9"),
        )
        .expect("cursor with tiebreaker");

        let filter = apply_keyset_cursor_filter(
            None,
            &json!({ "created_at": "desc", "id": "desc" }),
            Some(&cursor),
        )
        .expect("cursor filter");

        assert_eq!(
            filter,
            Some(json!({
                "_or": [
                    { "created_at": { "_lt": "2026-01-01" } },
                    {
                        "_and": [
                            { "created_at": { "_eq": "2026-01-01" } },
                            { "id": { "_lt": "account-9" } }
                        ]
                    }
                ]
            }))
        );
    }

    #[test]
    fn keyset_cursor_sign_verify_roundtrips() {
        let cursor = encode_keyset_cursor("id", RuntimeSortDirection::Asc, json!("account-7"))
            .expect("encode");
        // Opaque wire format: payload "." signature, not raw JSON.
        assert!(cursor.contains('.'));
        assert!(!cursor.contains("\"field\""));

        let decoded = decode_keyset_cursor(&cursor).expect("decode");
        assert_eq!(decoded.field, "id");
        assert_eq!(decoded.direction, "asc");
        assert_eq!(decoded.value, json!("account-7"));
        assert_eq!(decoded.tiebreaker, None);
    }

    #[test]
    fn keyset_cursor_with_tiebreaker_roundtrips() {
        let cursor = encode_keyset_cursor_with_tiebreaker(
            "created_at",
            RuntimeSortDirection::Desc,
            json!("2026-01-01"),
            "id",
            json!("account-9"),
        )
        .expect("encode");
        let decoded = decode_keyset_cursor(&cursor).expect("decode");
        let tiebreaker = decoded.tiebreaker.expect("tiebreaker present");
        assert_eq!(tiebreaker.field, "id");
        assert_eq!(tiebreaker.value, json!("account-9"));
    }

    #[test]
    fn keyset_cursor_rejects_tampered_payload() {
        let cursor = encode_keyset_cursor("id", RuntimeSortDirection::Asc, json!("account-7"))
            .expect("encode");

        // Flip a character in the base64 payload portion (before the '.'): signature no longer
        // matches and the cursor must be rejected.
        let (payload, signature) = cursor.split_once('.').expect("dot-delimited cursor");
        let mut tampered_payload: Vec<char> = payload.chars().collect();
        let last = tampered_payload.len() - 1;
        tampered_payload[last] = if tampered_payload[last] == 'A' {
            'B'
        } else {
            'A'
        };
        let tampered: String = tampered_payload.into_iter().collect();
        let forged = format!("{tampered}.{signature}");

        assert!(decode_keyset_cursor(&forged).is_err());
    }

    #[test]
    fn keyset_cursor_rejects_tampered_signature() {
        let cursor = encode_keyset_cursor("id", RuntimeSortDirection::Asc, json!("account-7"))
            .expect("encode");
        let (payload, _signature) = cursor.split_once('.').expect("dot-delimited cursor");
        // Re-sign the payload with a different key: signature is well-formed but invalid.
        let forged_sig = URL_SAFE_NO_PAD.encode(cursor_hmac(b"attacker-key", payload.as_bytes()));
        let forged = format!("{payload}.{forged_sig}");
        assert!(decode_keyset_cursor(&forged).is_err());
    }

    #[test]
    fn cursor_signing_key_gate_is_fail_closed_in_managed_envs() {
        // Side-effect-free resolution: no process-global env mutation, so this is race-free.
        // Managed env (ENV_NAME != local) with no configured key is rejected.
        assert!(resolve_cursor_signing_key(None, Some("compose")).is_err());
        assert!(resolve_cursor_signing_key(Some("   "), Some("prod")).is_err());

        // Managed env with an explicit key uses that key.
        assert_eq!(
            resolve_cursor_signing_key(Some("managed-secret"), Some("prod")).expect("explicit key"),
            b"managed-secret".to_vec()
        );

        // local / unset ENV_NAME falls back to the dev default key.
        assert_eq!(
            resolve_cursor_signing_key(None, Some("local")).expect("dev default"),
            DEV_CURSOR_SIGNING_KEY.as_bytes().to_vec()
        );
        assert_eq!(
            resolve_cursor_signing_key(None, None).expect("dev default"),
            DEV_CURSOR_SIGNING_KEY.as_bytes().to_vec()
        );
    }

    #[test]
    fn aggregate_function_names_are_runtime_owned() {
        assert_eq!(
            RuntimeAggregateFunction::from_name("COUNT").expect("count"),
            RuntimeAggregateFunction::Count
        );
        assert_eq!(
            RuntimeAggregateFunction::from_name("count-distinct").expect("count distinct"),
            RuntimeAggregateFunction::CountDistinct
        );
        assert_eq!(RuntimeAggregateFunction::Avg.as_str(), "avg");
        assert!(RuntimeAggregateFunction::from_name("median").is_err());
    }

    #[test]
    fn aggregate_alias_validation_is_runtime_owned() {
        validate_aggregate_output_alias("total_age").expect("valid alias");
        validate_unique_aggregate_aliases(["is_active", "total_age"]).expect("unique aliases");
        assert!(validate_aggregate_output_alias("1bad").is_err());
        assert!(validate_unique_aggregate_aliases(["count", "count"]).is_err());
        assert!(aggregate_alias_exists(["count", "total_age"], "total_age"));
    }

    #[test]
    fn aggregate_metric_validation_uses_runtime_field_descriptors() {
        let age = RuntimeAggregateFieldDescriptor::new("age", RuntimeDataType::Int32);
        let name = RuntimeAggregateFieldDescriptor::new("name", RuntimeDataType::String);
        let contacts = RuntimeAggregateFieldDescriptor::new("contacts", RuntimeDataType::NavToMany);

        validate_aggregate_metric_field(RuntimeAggregateFunction::Sum, Some(&age))
            .expect("sum numeric");
        validate_aggregate_metric_field(RuntimeAggregateFunction::CountDistinct, Some(&name))
            .expect("count distinct scalar");
        ensure_aggregate_groupable(&name).expect("string groupable");

        assert!(
            validate_aggregate_metric_field(RuntimeAggregateFunction::Sum, Some(&name)).is_err()
        );
        assert!(
            validate_aggregate_metric_field(RuntimeAggregateFunction::CountDistinct, None).is_err()
        );
        assert!(ensure_aggregate_groupable(&contacts).is_err());
        assert_eq!(
            default_aggregate_metric_alias(RuntimeAggregateFunction::Sum, Some("age")),
            "sum_age"
        );
    }

    #[test]
    fn aggregate_having_operator_policy_is_runtime_owned() {
        ensure_aggregate_having_op(RuntimeFilterOp::Gte).expect("gte supported");
        assert!(ensure_aggregate_having_op(RuntimeFilterOp::Contains).is_err());
    }

    #[test]
    fn parser_input_normalization_is_runtime_owned() {
        assert_eq!(
            normalize_object_input(Some(json!(r#"{"name":"Acme"}"#)), "filter")
                .expect("object string")
                .expect("object")["name"],
            json!("Acme")
        );
        assert_eq!(
            normalize_array_input(Some(json!("name")), "group_by")
                .expect("array")
                .expect("array"),
            vec![json!("name")]
        );
        assert!(normalize_object_input(Some(json!([1])), "filter").is_err());
        assert!(normalize_array_input(Some(json!({ "name": "Acme" })), "group_by").is_err());
    }

    #[test]
    fn parser_string_extraction_is_runtime_owned() {
        let mut obj = json!({
            "field": "age",
            "function": "sum"
        })
        .as_object()
        .cloned()
        .expect("object");

        assert_eq!(
            take_string(&mut obj, "field", "metric").expect("field"),
            "age"
        );
        assert_eq!(
            take_string_any(&mut obj, &["fn", "function"], "metric function").expect("function"),
            "sum"
        );
        assert!(take_string_any(&mut obj, &["fn", "function"], "metric function").is_err());
        assert!(expect_string(json!(1), "metric alias").is_err());
    }

    #[test]
    fn filter_conjunction_and_relationship_shape_checks_are_runtime_owned() {
        let items = normalize_filter_conjunction_items(
            "_and",
            json!([
                { "name": { "_eq": "Acme" } },
                { "age": { "_gt": 10 } }
            ]),
        )
        .expect("conjunction items");
        assert_eq!(items.len(), 2);

        let relation =
            expect_relationship_filter_object("navigation", "industry", json!({ "name": "Tech" }))
                .expect("relationship filter");
        assert_eq!(relation["name"], json!("Tech"));

        assert!(normalize_filter_conjunction_items("_and", json!({})).is_err());
        assert!(
            expect_relationship_filter_object("navigation", "industry", json!("Tech")).is_err()
        );
    }

    #[test]
    fn scalar_filter_predicate_parsing_is_runtime_owned() {
        let predicates =
            scalar_filter_predicates("age", json!({ "_gt": 10, "_lt": 20 })).expect("predicates");
        assert_eq!(predicates.len(), 2);
        assert!(predicates
            .iter()
            .any(|predicate| predicate.op == RuntimeFilterOp::Gt));
        assert!(predicates
            .iter()
            .any(|predicate| predicate.op == RuntimeFilterOp::Lt));

        let default_eq = scalar_filter_predicates("name", json!("Acme")).expect("default eq");
        assert_eq!(default_eq[0].op, RuntimeFilterOp::Eq);
        assert_eq!(default_eq[0].value, json!("Acme"));

        let object_id = scalar_filter_predicates("id", json!({ "$oid": "abc" })).expect("oid");
        assert_eq!(object_id[0].op, RuntimeFilterOp::Eq);
        assert!(scalar_filter_predicates("age", json!({})).is_err());
        assert!(scalar_filter_predicates("age", json!({ "_bogus": 1 })).is_err());
    }
}
