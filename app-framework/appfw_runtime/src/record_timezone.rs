use chrono::{DateTime, FixedOffset, Utc};
use serde_json::{Map, Value};

use crate::{
    model_metadata::{RuntimeDataType, RuntimePropertyMetadata},
    AccessAction, RuntimeError,
};

pub fn try_adjust_timezone(
    prop: &RuntimePropertyMetadata,
    record: &Map<String, Value>,
    action: AccessAction,
    user_timezone: &str,
) -> Result<Option<Value>, RuntimeError> {
    if prop.data_type != RuntimeDataType::DateTime {
        return Ok(None);
    }

    let Some(value) = record.get(&prop.name) else {
        return Ok(None);
    };

    match action {
        AccessAction::Create | AccessAction::Update => Ok(Some(to_utc(value, user_timezone)?)),
        AccessAction::Read => Ok(Some(to_client_tz(value, user_timezone)?)),
        _ => Ok(None),
    }
}

pub fn to_client_tz(utc_value: &Value, client_tz: &str) -> Result<Value, RuntimeError> {
    if utc_value.is_null() {
        return Ok(Value::Null);
    }

    let datetime_str = utc_value
        .as_str()
        .ok_or_else(|| RuntimeError::Validation("Expected datetime as a string".to_string()))?;
    let datetime_utc = datetime_str
        .parse::<DateTime<Utc>>()
        .map_err(|err| RuntimeError::Internal(err.to_string()))?;
    let datetime_client = datetime_utc.with_timezone(&string_to_tz(client_tz));

    Ok(Value::String(datetime_client.to_rfc3339()))
}

pub fn to_utc(client_value: &Value, _client_tz: &str) -> Result<Value, RuntimeError> {
    if client_value.is_null() {
        return Ok(Value::Null);
    }

    let datetime_str = client_value.as_str().ok_or_else(|| {
        RuntimeError::Validation(format!(
            "Expected datetime as a string, got: {:?}",
            client_value
        ))
    })?;
    let datetime_client = datetime_str
        .parse::<DateTime<FixedOffset>>()
        .map_err(|err| RuntimeError::Internal(err.to_string()))?;
    let datetime_utc = datetime_client.with_timezone(&Utc);

    Ok(Value::String(datetime_utc.to_rfc3339()))
}

fn string_to_tz(client_tz: &str) -> chrono_tz::Tz {
    client_tz.parse::<chrono_tz::Tz>().unwrap_or(chrono_tz::UCT)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn prop(name: &str, data_type: RuntimeDataType) -> RuntimePropertyMetadata {
        RuntimePropertyMetadata {
            id: format!("prop-{name}"),
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
            meta: None,
        }
    }

    #[test]
    fn to_client_tz_converts_utc_datetime_to_named_timezone() {
        let adjusted = to_client_tz(&json!("2024-01-15T12:00:00Z"), "America/New_York").unwrap();

        assert_eq!(adjusted, json!("2024-01-15T07:00:00-05:00"));
    }

    #[test]
    fn to_client_tz_defaults_invalid_timezone_to_utc() {
        let adjusted = to_client_tz(&json!("2024-01-15T12:00:00Z"), "not-a-timezone").unwrap();

        assert_eq!(adjusted, json!("2024-01-15T12:00:00+00:00"));
    }

    #[test]
    fn to_utc_converts_fixed_offset_datetime_to_utc() {
        let adjusted = to_utc(&json!("2024-01-15T07:00:00-05:00"), "America/New_York").unwrap();

        assert_eq!(adjusted, json!("2024-01-15T12:00:00+00:00"));
    }

    #[test]
    fn timezone_helpers_preserve_null_optional_values() {
        assert_eq!(
            to_client_tz(&Value::Null, "America/Denver").unwrap(),
            Value::Null
        );
        assert_eq!(to_utc(&Value::Null, "America/Denver").unwrap(), Value::Null);
    }

    #[test]
    fn timezone_helpers_reject_non_string_values() {
        assert!(matches!(
            to_client_tz(&json!(123), "UTC"),
            Err(RuntimeError::Validation(_))
        ));
        assert!(matches!(
            to_utc(&json!({ "date": "2024-01-15T12:00:00Z" }), "UTC"),
            Err(RuntimeError::Validation(_))
        ));
    }

    #[test]
    fn try_adjust_timezone_converts_by_access_action_for_datetime_properties() {
        let prop = prop("starts_at", RuntimeDataType::DateTime);
        let mut record = Map::new();
        record.insert("starts_at".to_string(), json!("2024-01-15T12:00:00Z"));

        let read_value =
            try_adjust_timezone(&prop, &record, AccessAction::Read, "America/New_York").unwrap();
        assert_eq!(read_value, Some(json!("2024-01-15T07:00:00-05:00")));

        record.insert("starts_at".to_string(), json!("2024-01-15T07:00:00-05:00"));
        let create_value =
            try_adjust_timezone(&prop, &record, AccessAction::Create, "America/New_York").unwrap();
        assert_eq!(create_value, Some(json!("2024-01-15T12:00:00+00:00")));

        let update_value =
            try_adjust_timezone(&prop, &record, AccessAction::Update, "America/New_York").unwrap();
        assert_eq!(update_value, Some(json!("2024-01-15T12:00:00+00:00")));
    }

    #[test]
    fn try_adjust_timezone_ignores_missing_non_datetime_and_delete_actions() {
        let datetime_prop = prop("starts_at", RuntimeDataType::DateTime);
        let string_prop = prop("name", RuntimeDataType::String);
        let mut record = Map::new();
        record.insert("name".to_string(), json!("Meeting"));
        record.insert("starts_at".to_string(), json!("2024-01-15T12:00:00Z"));

        assert_eq!(
            try_adjust_timezone(
                &string_prop,
                &record,
                AccessAction::Read,
                "America/New_York"
            )
            .unwrap(),
            None
        );
        assert_eq!(
            try_adjust_timezone(
                &datetime_prop,
                &Map::new(),
                AccessAction::Read,
                "America/New_York"
            )
            .unwrap(),
            None
        );
        assert_eq!(
            try_adjust_timezone(
                &datetime_prop,
                &record,
                AccessAction::Delete,
                "America/New_York"
            )
            .unwrap(),
            None
        );
    }
}
