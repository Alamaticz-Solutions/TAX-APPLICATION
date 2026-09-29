use std::cmp::Ordering;

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime};
use regex::RegexBuilder;
use serde_json::{Map, Value};

use crate::{
    model_metadata::{RuntimeDataType, RuntimeEntityMetadata, RuntimePropertyMetadata},
    AccessAction, RuntimeError,
};

pub fn validate_record(
    entity: &RuntimeEntityMetadata,
    record: &Map<String, Value>,
    action: AccessAction,
) -> Result<(), RuntimeError> {
    if action != AccessAction::Create && action != AccessAction::Update {
        return Ok(());
    }

    for prop in &entity.properties {
        let value = record.get(&prop.name);
        if prop.is_required && !prop.is_read_only && value.is_none_or(value_is_missing) {
            return Err(property_error(entity, prop, None, "is required"));
        }

        for validator in property_validators(&prop.name, prop.meta.as_ref())? {
            match validator_kind(validator)? {
                ValidatorKind::Uniqueness => {}
                ValidatorKind::Function => {
                    validate_function(entity, prop, validator, value, record)?
                }
                ValidatorKind::StringLength => {
                    if let Some(value) = value.filter(|v| !value_is_missing(v)) {
                        validate_string_length(entity, prop, validator, value)?
                    }
                }
                ValidatorKind::ArrayLength => {
                    if let Some(value) = value.filter(|v| !value_is_missing(v)) {
                        validate_array_length(entity, prop, validator, value)?
                    }
                }
                ValidatorKind::StringPattern => {
                    if let Some(value) = value.filter(|v| !value_is_missing(v)) {
                        validate_string_pattern(entity, prop, validator, value)?
                    }
                }
                ValidatorKind::ValueRange => {
                    if let Some(value) = value.filter(|v| !value_is_missing(v)) {
                        validate_value_range(entity, prop, validator, value, record)?
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn property_validators<'a>(
    property_name: &str,
    meta: Option<&'a Value>,
) -> Result<Vec<&'a Value>, RuntimeError> {
    let Some(meta) = meta else {
        return Ok(Vec::new());
    };
    let Some(validators) = meta.get("validators") else {
        return Ok(Vec::new());
    };
    let validators = validators.as_array().ok_or_else(|| {
        RuntimeError::Validation(format!(
            "{} validators metadata must be an array",
            property_name
        ))
    })?;
    Ok(validators.iter().collect())
}

pub fn is_validator_named(validator: &Value, expected_name: &str) -> bool {
    normalize_validator_name(validator_name(validator).unwrap_or_default())
        == normalize_validator_name(expected_name)
}

pub fn validator_message(validator: &Value, default_message: &str) -> String {
    validator
        .get("message")
        .and_then(Value::as_str)
        .filter(|message| !message.trim().is_empty())
        .unwrap_or(default_message)
        .to_string()
}

pub fn render_uniqueness_filter(
    filter: &str,
    record: &Map<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = serde_json::from_str::<Value>(filter.trim()).map_err(|err| {
        RuntimeError::Validation(format!("uniqueness filter must be valid JSON: {}", err))
    })?;
    render_filter_value(&value, record)
}

pub fn uniqueness_conflict(
    pk_name: &str,
    record: &Map<String, Value>,
    candidates: &[Map<String, Value>],
) -> bool {
    let current_id = record.get(pk_name).and_then(value_to_string);
    candidates.iter().any(|candidate| {
        let candidate_id = candidate.get(pk_name).and_then(value_to_string);
        !matches!(
            (&current_id, &candidate_id),
            (Some(current_id), Some(candidate_id)) if current_id == candidate_id
        )
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ValidatorKind {
    ValueRange,
    ArrayLength,
    StringLength,
    StringPattern,
    Uniqueness,
    Function,
}

#[derive(Debug)]
enum Comparable {
    Number(f64),
    String(String),
    Date(NaiveDate),
    DateTime(NaiveDateTime),
    Time(NaiveTime),
}

impl Comparable {
    fn compare(&self, other: &Self) -> Result<Ordering, RuntimeError> {
        match (self, other) {
            (Self::Number(left), Self::Number(right)) => left.partial_cmp(right).ok_or_else(|| {
                RuntimeError::Validation(
                    "range comparison encountered a non-finite number".to_string(),
                )
            }),
            (Self::String(left), Self::String(right)) => Ok(left.cmp(right)),
            (Self::Date(left), Self::Date(right)) => Ok(left.cmp(right)),
            (Self::DateTime(left), Self::DateTime(right)) => Ok(left.cmp(right)),
            (Self::Time(left), Self::Time(right)) => Ok(left.cmp(right)),
            _ => Err(RuntimeError::Validation(
                "range comparison values have incompatible types".to_string(),
            )),
        }
    }
}

fn validate_string_length(
    entity: &RuntimeEntityMetadata,
    prop: &RuntimePropertyMetadata,
    validator: &Value,
    value: &Value,
) -> Result<(), RuntimeError> {
    let string = value.as_str().ok_or_else(|| {
        property_error(
            entity,
            prop,
            Some(validator),
            "StringLength expects a string value",
        )
    })?;
    let len = string.chars().count();
    if let Some(min) = optional_usize(validator, "min")? {
        if len < min {
            return Err(property_error(
                entity,
                prop,
                Some(validator),
                &format!("must be at least {} characters", min),
            ));
        }
    }
    if let Some(max) = optional_usize(validator, "max")? {
        if len > max {
            return Err(property_error(
                entity,
                prop,
                Some(validator),
                &format!("must be at most {} characters", max),
            ));
        }
    }
    Ok(())
}

fn validate_array_length(
    entity: &RuntimeEntityMetadata,
    prop: &RuntimePropertyMetadata,
    validator: &Value,
    value: &Value,
) -> Result<(), RuntimeError> {
    let array = value.as_array().ok_or_else(|| {
        property_error(
            entity,
            prop,
            Some(validator),
            "ArrayLength expects an array value",
        )
    })?;
    let len = array.len();
    if let Some(min) = optional_usize(validator, "min")? {
        if len < min {
            return Err(property_error(
                entity,
                prop,
                Some(validator),
                &format!("must contain at least {} item(s)", min),
            ));
        }
    }
    if let Some(max) = optional_usize(validator, "max")? {
        if len > max {
            return Err(property_error(
                entity,
                prop,
                Some(validator),
                &format!("must contain at most {} item(s)", max),
            ));
        }
    }
    Ok(())
}

fn validate_string_pattern(
    entity: &RuntimeEntityMetadata,
    prop: &RuntimePropertyMetadata,
    validator: &Value,
    value: &Value,
) -> Result<(), RuntimeError> {
    let string = value.as_str().ok_or_else(|| {
        property_error(
            entity,
            prop,
            Some(validator),
            "StringPattern expects a string value",
        )
    })?;
    let pattern = validator
        .get("pattern")
        .or_else(|| validator.get("expression"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            property_error(
                entity,
                prop,
                Some(validator),
                "StringPattern is missing pattern",
            )
        })?;
    let (pattern, flags) = split_regex_pattern(pattern);
    let regex = build_regex(pattern, flags).map_err(|message| {
        property_error(
            entity,
            prop,
            Some(validator),
            &format!("StringPattern is invalid: {}", message),
        )
    })?;
    if !regex.is_match(string) {
        return Err(property_error(
            entity,
            prop,
            Some(validator),
            "does not match the required pattern",
        ));
    }
    Ok(())
}

fn validate_value_range(
    entity: &RuntimeEntityMetadata,
    prop: &RuntimePropertyMetadata,
    validator: &Value,
    value: &Value,
    record: &Map<String, Value>,
) -> Result<(), RuntimeError> {
    let current = comparable_from_value(prop.data_type, value)?;
    if let Some(min) = resolve_range_bound(validator, "min", "min_source", record)? {
        let min = comparable_from_value(prop.data_type, &min)?;
        if current.compare(&min)? == Ordering::Less {
            return Err(property_error(
                entity,
                prop,
                Some(validator),
                "is below the allowed minimum",
            ));
        }
    }
    if let Some(max) = resolve_range_bound(validator, "max", "max_source", record)? {
        let max = comparable_from_value(prop.data_type, &max)?;
        if current.compare(&max)? == Ordering::Greater {
            return Err(property_error(
                entity,
                prop,
                Some(validator),
                "is above the allowed maximum",
            ));
        }
    }
    Ok(())
}

fn validate_function(
    entity: &RuntimeEntityMetadata,
    prop: &RuntimePropertyMetadata,
    validator: &Value,
    value: Option<&Value>,
    record: &Map<String, Value>,
) -> Result<(), RuntimeError> {
    let func = validator
        .get("func")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            property_error(
                entity,
                prop,
                Some(validator),
                "Function validator is missing func",
            )
        })?;
    let parts = func.split(':').collect::<Vec<_>>();
    match parts.as_slice() {
        ["required_when", field_name, expected] => {
            if record_value_matches(record, field_name, expected)
                && value.is_none_or(value_is_missing)
            {
                return Err(property_error(entity, prop, Some(validator), "is required"));
            }
            Ok(())
        }
        ["forbidden_when", field_name, expected] => {
            if record_value_matches(record, field_name, expected)
                && value.is_some_and(|value| !value_is_missing(value))
            {
                return Err(property_error(
                    entity,
                    prop,
                    Some(validator),
                    "is not allowed",
                ));
            }
            Ok(())
        }
        ["gte_field", field_name] => compare_against_field(
            entity,
            prop,
            validator,
            value,
            record,
            field_name,
            Ordering::Less,
            "must be greater than or equal to the referenced field",
        ),
        ["lte_field", field_name] => compare_against_field(
            entity,
            prop,
            validator,
            value,
            record,
            field_name,
            Ordering::Greater,
            "must be less than or equal to the referenced field",
        ),
        _ => Err(property_error(
            entity,
            prop,
            Some(validator),
            &format!("unsupported business validator function '{}'", func),
        )),
    }
}

fn compare_against_field(
    entity: &RuntimeEntityMetadata,
    prop: &RuntimePropertyMetadata,
    validator: &Value,
    value: Option<&Value>,
    record: &Map<String, Value>,
    field_name: &str,
    invalid_ordering: Ordering,
    default_message: &str,
) -> Result<(), RuntimeError> {
    let Some(value) = value.filter(|value| !value_is_missing(value)) else {
        return Ok(());
    };
    let Some(other) = lookup_record_value(record, field_name) else {
        return Ok(());
    };
    if value_is_missing(other) {
        return Ok(());
    }

    let current = comparable_from_value(prop.data_type, value)?;
    let other = comparable_from_value(prop.data_type, other)?;
    if current.compare(&other)? == invalid_ordering {
        return Err(property_error(
            entity,
            prop,
            Some(validator),
            default_message,
        ));
    }
    Ok(())
}

fn resolve_range_bound(
    validator: &Value,
    value_key: &str,
    source_key: &str,
    record: &Map<String, Value>,
) -> Result<Option<Value>, RuntimeError> {
    let Some(raw_value) = validator.get(value_key) else {
        return Ok(None);
    };
    if raw_value.is_null() {
        return Ok(None);
    }

    match validator_source(validator, source_key)? {
        BoundSource::Literal => Ok(Some(raw_value.clone())),
        BoundSource::FieldReference => {
            let field_name = raw_value.as_str().ok_or_else(|| {
                RuntimeError::Validation(format!(
                    "{} must be a field name when {} is FieldReference",
                    value_key, source_key
                ))
            })?;
            let value = lookup_record_value(record, field_name)
                .filter(|value| !value_is_missing(value))
                .ok_or_else(|| {
                    RuntimeError::Validation(format!(
                        "range bound references missing field '{}'",
                        field_name
                    ))
                })?;
            Ok(Some(value.clone()))
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BoundSource {
    Literal,
    FieldReference,
}

fn validator_source(validator: &Value, source_key: &str) -> Result<BoundSource, RuntimeError> {
    let source = validator
        .get(source_key)
        .and_then(Value::as_str)
        .unwrap_or("Literal");
    match normalize_source_name(source).as_str() {
        "literal" => Ok(BoundSource::Literal),
        "fieldreference" => Ok(BoundSource::FieldReference),
        "expression" | "function" => Err(RuntimeError::Validation(format!(
            "{} range bound sources are not supported at runtime",
            source
        ))),
        _ => Err(RuntimeError::Validation(format!(
            "unsupported range bound source '{}'",
            source
        ))),
    }
}

fn comparable_from_value(
    data_type: RuntimeDataType,
    value: &Value,
) -> Result<Comparable, RuntimeError> {
    match data_type {
        RuntimeDataType::Int8
        | RuntimeDataType::Int16
        | RuntimeDataType::Int32
        | RuntimeDataType::Int64
        | RuntimeDataType::Float32
        | RuntimeDataType::Float64 => Ok(Comparable::Number(number_from_value(value)?)),
        RuntimeDataType::Date => {
            let value = string_from_value(value)?;
            let date = NaiveDate::parse_from_str(&value, "%Y-%m-%d").map_err(|err| {
                RuntimeError::Validation(format!("invalid date value '{}': {}", value, err))
            })?;
            Ok(Comparable::Date(date))
        }
        RuntimeDataType::DateTime => {
            let value = string_from_value(value)?;
            let date_time = parse_datetime(&value)?;
            Ok(Comparable::DateTime(date_time))
        }
        RuntimeDataType::Time => {
            let value = string_from_value(value)?;
            let time = parse_time(&value)?;
            Ok(Comparable::Time(time))
        }
        RuntimeDataType::String | RuntimeDataType::Enum => {
            Ok(Comparable::String(string_from_value(value)?))
        }
        _ => Err(RuntimeError::Validation(format!(
            "ValueRange is unsupported for {:?}",
            data_type
        ))),
    }
}

fn number_from_value(value: &Value) -> Result<f64, RuntimeError> {
    match value {
        Value::Number(number) => number.as_f64().ok_or_else(|| {
            RuntimeError::Validation(format!("number value '{}' cannot be represented", number))
        }),
        Value::String(value) => value.parse::<f64>().map_err(|err| {
            RuntimeError::Validation(format!("invalid numeric value '{}': {}", value, err))
        }),
        _ => Err(RuntimeError::Validation(
            "numeric range validation expects a number".to_string(),
        )),
    }
}

fn string_from_value(value: &Value) -> Result<String, RuntimeError> {
    match value {
        Value::String(value) => Ok(value.clone()),
        Value::Number(value) => Ok(value.to_string()),
        Value::Bool(value) => Ok(value.to_string()),
        _ => Err(RuntimeError::Validation(
            "range validation expects a scalar value".to_string(),
        )),
    }
}

fn parse_datetime(value: &str) -> Result<NaiveDateTime, RuntimeError> {
    DateTime::parse_from_rfc3339(value)
        .map(|date_time| date_time.naive_utc())
        .or_else(|_| NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f"))
        .or_else(|_| NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.f"))
        .map_err(|err| {
            RuntimeError::Validation(format!("invalid date-time value '{}': {}", value, err))
        })
}

fn parse_time(value: &str) -> Result<NaiveTime, RuntimeError> {
    NaiveTime::parse_from_str(value, "%H:%M:%S%.f")
        .or_else(|_| NaiveTime::parse_from_str(value, "%H:%M"))
        .map_err(|err| RuntimeError::Validation(format!("invalid time value '{}': {}", value, err)))
}

fn render_filter_value(value: &Value, record: &Map<String, Value>) -> Result<Value, RuntimeError> {
    match value {
        Value::String(value) => render_filter_string(value, record),
        Value::Array(items) => items
            .iter()
            .map(|item| render_filter_value(item, record))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Value::Object(obj) => {
            let mut rendered = Map::new();
            for (key, value) in obj {
                rendered.insert(key.clone(), render_filter_value(value, record)?);
            }
            Ok(Value::Object(rendered))
        }
        _ => Ok(value.clone()),
    }
}

fn render_filter_string(value: &str, record: &Map<String, Value>) -> Result<Value, RuntimeError> {
    let Some(field_name) = placeholder_field_name(value) else {
        return Ok(Value::String(value.to_string()));
    };
    lookup_record_value(record, field_name)
        .filter(|value| !value_is_missing(value))
        .cloned()
        .ok_or_else(|| {
            RuntimeError::Validation(format!(
                "uniqueness filter references missing field '{}'",
                field_name
            ))
        })
}

fn placeholder_field_name(value: &str) -> Option<&str> {
    // Documented uniqueness filters use Mustache-style `{{field}}`.
    // Prefer that before single-brace `{field}` so `{{job_key}}` resolves to
    // `job_key` instead of the literal `{job_key}`.
    if let Some(field_name) = value.strip_prefix("{{").and_then(|s| s.strip_suffix("}}")) {
        return non_empty(field_name);
    }
    if let Some(field_name) = value.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        return non_empty(field_name);
    }
    if let Some(field_name) = value.strip_prefix('$') {
        return non_empty(field_name);
    }
    if let Some(field_name) = value.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
        return non_empty(field_name);
    }
    None
}

fn non_empty(value: &str) -> Option<&str> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn validator_name(validator: &Value) -> Option<&str> {
    validator.get("name").and_then(Value::as_str)
}

fn normalize_validator_name(name: &str) -> String {
    name.trim()
        .trim_end_matches("Validator")
        .chars()
        .filter(|c| !c.is_ascii_whitespace() && *c != '_' && *c != '-')
        .flat_map(char::to_lowercase)
        .collect()
}

fn normalize_source_name(name: &str) -> String {
    name.chars()
        .filter(|c| !c.is_ascii_whitespace() && *c != '_' && *c != '-')
        .flat_map(char::to_lowercase)
        .collect()
}

fn optional_usize(validator: &Value, key: &str) -> Result<Option<usize>, RuntimeError> {
    match validator.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(number)) => {
            let value = number.as_i64().ok_or_else(|| {
                RuntimeError::Validation(format!("{} must be a non-negative integer", key))
            })?;
            usize_from_i64(key, value).map(Some)
        }
        Some(Value::String(value)) => {
            let value = value.parse::<i64>().map_err(|err| {
                RuntimeError::Validation(format!("{} must be a non-negative integer: {}", key, err))
            })?;
            usize_from_i64(key, value).map(Some)
        }
        Some(_) => Err(RuntimeError::Validation(format!(
            "{} must be a non-negative integer",
            key
        ))),
    }
}

fn usize_from_i64(key: &str, value: i64) -> Result<usize, RuntimeError> {
    if value < 0 {
        return Err(RuntimeError::Validation(format!(
            "{} must be a non-negative integer",
            key
        )));
    }
    Ok(value as usize)
}

fn split_regex_pattern(pattern: &str) -> (&str, &str) {
    if !pattern.starts_with('/') {
        return (pattern, "");
    }

    let mut escaped = false;
    let mut closing_slash = None;
    for (idx, ch) in pattern.char_indices().skip(1) {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '/' => closing_slash = Some(idx),
            _ => {}
        }
    }

    match closing_slash {
        Some(idx) if idx > 0 => (&pattern[1..idx], &pattern[idx + 1..]),
        _ => (pattern, ""),
    }
}

fn build_regex(pattern: &str, flags: &str) -> Result<regex::Regex, String> {
    let mut builder = RegexBuilder::new(pattern);
    for flag in flags.chars() {
        match flag {
            'i' => {
                builder.case_insensitive(true);
            }
            'm' => {
                builder.multi_line(true);
            }
            's' => {
                builder.dot_matches_new_line(true);
            }
            'g' | 'u' => {}
            other => return Err(format!("unsupported regex flag '{}'", other)),
        }
    }
    builder.build().map_err(|err| err.to_string())
}

fn value_is_missing(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(value) => value.trim().is_empty(),
        _ => false,
    }
}

fn lookup_record_value<'a>(record: &'a Map<String, Value>, field_name: &str) -> Option<&'a Value> {
    record
        .get(field_name)
        .or_else(|| record.get(&to_snake_case_lenient(field_name)))
}

fn record_value_matches(record: &Map<String, Value>, field_name: &str, expected: &str) -> bool {
    lookup_record_value(record, field_name).is_some_and(|value| match value {
        Value::String(value) => value == expected,
        Value::Number(value) => value.to_string() == expected,
        Value::Bool(value) => value.to_string() == expected,
        _ => false,
    })
}

fn to_snake_case_lenient(value: &str) -> String {
    if looks_like_snake_case(value) {
        return value.to_string();
    }

    let mut result = String::with_capacity(value.len());
    let chars = value.chars().collect::<Vec<_>>();

    for (index, &ch) in chars.iter().enumerate() {
        if ch == '_' || ch == '-' || ch.is_ascii_whitespace() {
            if !result.ends_with('_') && !result.is_empty() {
                result.push('_');
            }
            continue;
        }

        let prev = index.checked_sub(1).and_then(|idx| chars.get(idx)).copied();
        let next = chars.get(index + 1).copied();
        let starts_word = ch.is_uppercase()
            && index > 0
            && !result.ends_with('_')
            && (prev.is_some_and(|prev| prev.is_lowercase() || prev.is_ascii_digit())
                || next.is_some_and(|next| next.is_lowercase()));
        if starts_word {
            result.push('_');
        }

        for lower in ch.to_lowercase() {
            result.push(lower);
        }
    }

    result.trim_matches('_').to_string()
}

fn looks_like_snake_case(value: &str) -> bool {
    if value.is_empty() || value.starts_with('_') || value.ends_with('_') || value.contains("__") {
        return false;
    }
    value
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn validator_kind(validator: &Value) -> Result<ValidatorKind, RuntimeError> {
    let name = validator_name(validator)
        .ok_or_else(|| RuntimeError::Validation("validator is missing name".to_string()))?;
    match normalize_validator_name(name).as_str() {
        "valuerange" => Ok(ValidatorKind::ValueRange),
        "arraylength" => Ok(ValidatorKind::ArrayLength),
        "stringlength" => Ok(ValidatorKind::StringLength),
        "stringpattern" => Ok(ValidatorKind::StringPattern),
        "uniqueness" => Ok(ValidatorKind::Uniqueness),
        "function" => Ok(ValidatorKind::Function),
        _ => Err(RuntimeError::Validation(format!(
            "unsupported validator '{}'",
            name
        ))),
    }
}

fn property_error(
    entity: &RuntimeEntityMetadata,
    prop: &RuntimePropertyMetadata,
    validator: Option<&Value>,
    default_message: &str,
) -> RuntimeError {
    let message = validator
        .map(|validator| validator_message(validator, default_message))
        .unwrap_or_else(|| default_message.to_string());
    RuntimeError::Validation(format!("{}.{}: {}", entity.pascal_1, prop.name, message))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn entity(properties: Vec<RuntimePropertyMetadata>) -> RuntimeEntityMetadata {
        RuntimeEntityMetadata {
            id: "test-entity".to_string(),
            schema_name: "test".to_string(),
            schema_id: None,
            pascal_1: "Widget".to_string(),
            pascal_n: "Widgets".to_string(),
            snake_1: "widget".to_string(),
            snake_n: "widgets".to_string(),
            caption_1: "Widget".to_string(),
            caption_n: "Widgets".to_string(),
            is_union: false,
            base_type: None,
            is_table: true,
            facets: Vec::new(),
            meta: None,
            standard_methods: Vec::new(),
            custom_methods: Vec::new(),
            properties,
        }
    }

    fn property(
        name: &str,
        data_type: RuntimeDataType,
        validators: Value,
    ) -> RuntimePropertyMetadata {
        RuntimePropertyMetadata {
            id: format!("{}-id", name),
            name: name.to_string(),
            caption: name.to_string(),
            data_type,
            is_key: false,
            is_caption: false,
            is_required: false,
            is_read_only: false,
            is_concurrency_control: false,
            default_value: None,
            foreign_key: None,
            nav_by_fk: None,
            many_to_many: None,
            nested_entity_type: None,
            enum_type_name: None,
            meta: Some(json!({ "validators": validators })),
        }
    }

    fn required_property(name: &str, data_type: RuntimeDataType) -> RuntimePropertyMetadata {
        let mut property = property(name, data_type, json!([]));
        property.is_required = true;
        property
    }

    fn record(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    fn expect_validation_error(result: Result<(), RuntimeError>, text: &str) {
        let err = result.expect_err("expected validation error");
        assert!(
            err.to_string().contains(text),
            "expected '{err}' to contain '{text}'"
        );
    }

    #[test]
    fn validate_record_rejects_missing_required_fields() {
        let entity = entity(vec![required_property("name", RuntimeDataType::String)]);
        let record = record(json!({ "name": "" }));

        expect_validation_error(
            validate_record(&entity, &record, AccessAction::Create),
            "is required",
        );
    }

    #[test]
    fn validate_record_enforces_length_validators() {
        let entity = entity(vec![
            property(
                "name",
                RuntimeDataType::String,
                json!([{ "name": "StringLength", "min": 2, "max": 4, "message": "bad length" }]),
            ),
            property(
                "tags",
                RuntimeDataType::StringArray,
                json!([{ "name": "ArrayLength", "min": 1, "max": 2 }]),
            ),
        ]);

        assert!(validate_record(
            &entity,
            &record(json!({ "name": "Acme", "tags": ["a"] })),
            AccessAction::Update,
        )
        .is_ok());
        expect_validation_error(
            validate_record(
                &entity,
                &record(json!({ "name": "A", "tags": ["a"] })),
                AccessAction::Update,
            ),
            "bad length",
        );
        expect_validation_error(
            validate_record(
                &entity,
                &record(json!({ "name": "Acme", "tags": ["a", "b", "c"] })),
                AccessAction::Update,
            ),
            "at most 2",
        );
    }

    #[test]
    fn validate_record_enforces_string_pattern() {
        let entity = entity(vec![property(
            "email",
            RuntimeDataType::String,
            json!([{ "name": "StringPattern", "expression": "/^[^@]+@[^@]+$/", "message": "email only" }]),
        )]);

        assert!(validate_record(
            &entity,
            &record(json!({ "email": "a@example.com" })),
            AccessAction::Create,
        )
        .is_ok());
        expect_validation_error(
            validate_record(
                &entity,
                &record(json!({ "email": "not-email" })),
                AccessAction::Create,
            ),
            "email only",
        );
    }

    #[test]
    fn validate_record_enforces_value_range_with_field_reference() {
        let entity = entity(vec![property(
            "close_date",
            RuntimeDataType::Date,
            json!([{ "name": "ValueRange", "min": "open_date", "min_source": "FieldReference" }]),
        )]);

        assert!(validate_record(
            &entity,
            &record(json!({ "open_date": "2026-01-01", "close_date": "2026-01-02" })),
            AccessAction::Update,
        )
        .is_ok());
        expect_validation_error(
            validate_record(
                &entity,
                &record(json!({ "open_date": "2026-01-02", "close_date": "2026-01-01" })),
                AccessAction::Update,
            ),
            "below",
        );
    }

    #[test]
    fn validate_record_enforces_supported_function_validators() {
        let entity = entity(vec![
            property(
                "closed_reason",
                RuntimeDataType::String,
                json!([{ "name": "Function", "func": "required_when:status:Closed" }]),
            ),
            property(
                "end_date",
                RuntimeDataType::Date,
                json!([{ "name": "Function", "func": "gte_field:start_date" }]),
            ),
        ]);

        expect_validation_error(
            validate_record(
                &entity,
                &record(
                    json!({ "status": "Closed", "closed_reason": "", "start_date": "2026-01-02", "end_date": "2026-01-03" }),
                ),
                AccessAction::Update,
            ),
            "closed_reason",
        );
        expect_validation_error(
            validate_record(
                &entity,
                &record(
                    json!({ "status": "Open", "start_date": "2026-01-03", "end_date": "2026-01-02" }),
                ),
                AccessAction::Update,
            ),
            "greater than or equal",
        );
    }

    #[test]
    fn property_validators_reads_array_from_metadata() {
        let meta = json!({
            "validators": [
                { "name": "StringLength", "min": 2 },
                { "name": "Uniqueness", "filter": "{}" }
            ]
        });

        let validators = property_validators("name", Some(&meta)).expect("validators");

        assert_eq!(validators.len(), 2);
        assert!(is_validator_named(validators[0], "string_length"));
        assert!(is_validator_named(validators[1], "UniquenessValidator"));
    }

    #[test]
    fn property_validators_rejects_non_array_metadata() {
        let err = property_validators("name", Some(&json!({ "validators": true })))
            .expect_err("non-array metadata should fail");

        assert_eq!(
            err.to_string(),
            "validation error: name validators metadata must be an array"
        );
    }

    #[test]
    fn validator_message_uses_default_for_blank_message() {
        assert_eq!(
            validator_message(&json!({ "message": " " }), "must be unique"),
            "must be unique"
        );
        assert_eq!(
            validator_message(&json!({ "message": "custom" }), "must be unique"),
            "custom"
        );
    }

    #[test]
    fn uniqueness_filter_rendering_preserves_referenced_value_types() {
        let filter = render_uniqueness_filter(
            r#"{ "email": { "_eq": "$email" }, "tenant_id": { "_eq": "${tenantId}" }, "active": { "_eq": true } }"#,
            &record(json!({ "email": "a@example.com", "tenant_id": 42 })),
        )
        .expect("filter should render");

        assert_eq!(filter["email"]["_eq"], json!("a@example.com"));
        assert_eq!(filter["tenant_id"]["_eq"], json!(42));
        assert_eq!(filter["active"]["_eq"], json!(true));
    }

    #[test]
    fn uniqueness_filter_rendering_supports_mustache_placeholders() {
        let filter = render_uniqueness_filter(
            r#"{ "job_key": "{{job_key}}" }"#,
            &record(json!({ "job_key": "amerassist_collections_bad_debt_job" })),
        )
        .expect("mustache uniqueness filter should render");

        assert_eq!(
            filter["job_key"],
            json!("amerassist_collections_bad_debt_job")
        );
    }

    #[test]
    fn placeholder_lookup_preserves_digit_bearing_snake_case_names() {
        let filter = render_uniqueness_filter(
            r#"{ "q1_revenue": { "_eq": "$q1_revenue" }, "tenant_id": { "_eq": "$TenantID" } }"#,
            &record(json!({ "q1_revenue": 100, "tenant_id": "tenant-1" })),
        )
        .expect("filter should render");

        assert_eq!(filter["q1_revenue"]["_eq"], json!(100));
        assert_eq!(filter["tenant_id"]["_eq"], json!("tenant-1"));
    }

    #[test]
    fn uniqueness_conflict_ignores_current_record_on_update() {
        let current = record(json!({ "id": "1", "email": "a@example.com" }));
        let same = record(json!({ "id": "1" }));
        let other = record(json!({ "id": "2" }));

        assert!(!uniqueness_conflict("id", &current, &[same]));
        assert!(uniqueness_conflict("id", &current, &[other]));
    }
}
