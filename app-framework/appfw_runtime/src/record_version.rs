use serde_json::{Map, Value};

use crate::{
    model_metadata::{RuntimeDataType, RuntimeEntityMetadata, RuntimePropertyMetadata},
    RuntimeError,
};

pub fn get_record_version(
    entity: &RuntimeEntityMetadata,
    input: &Map<String, Value>,
) -> Result<Option<Value>, RuntimeError> {
    Ok(entity
        .properties
        .iter()
        .find(|prop| prop.is_concurrency_control)
        .and_then(|prop| input.get(&prop.name).cloned()))
}

pub fn new_record_version(
    prop: &RuntimePropertyMetadata,
    record: &Map<String, Value>,
) -> Result<Option<Value>, RuntimeError> {
    match prop.data_type {
        RuntimeDataType::Int64 => {
            // Monotonic counters stay inside JavaScript's Number.MAX_SAFE_INTEGER so
            // browser admin clients can round-trip optimistic-concurrency versions.
            // Nanosecond timestamps previously overflowed JSON number precision and
            // surfaced as "invalid key or version" on the next Save.
            const JS_MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;
            let current = record
                .get(&prop.name)
                .and_then(|value| match value {
                    Value::Number(number) => number.as_i64(),
                    Value::String(text) => text.parse::<i64>().ok(),
                    _ => None,
                })
                .filter(|value| *value >= 0 && *value <= JS_MAX_SAFE_INTEGER)
                .unwrap_or(0);
            Ok(Some(Value::from(current.saturating_add(1))))
        }
        _ => Err(RuntimeError::Validation(format!(
            "unsupported type for concurrency control '{}'",
            prop.name
        ))),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn record(value: Value) -> Map<String, Value> {
        value.as_object().expect("record object").clone()
    }

    fn entity_with_version_prop(prop: RuntimePropertyMetadata) -> RuntimeEntityMetadata {
        RuntimeEntityMetadata {
            id: "entity-1".to_string(),
            schema_name: "crm".to_string(),
            schema_id: None,
            pascal_1: "Account".to_string(),
            pascal_n: "Accounts".to_string(),
            snake_1: "account".to_string(),
            snake_n: "accounts".to_string(),
            caption_1: "Account".to_string(),
            caption_n: "Accounts".to_string(),
            is_union: false,
            base_type: None,
            is_table: true,
            facets: Vec::new(),
            meta: None,
            standard_methods: Vec::new(),
            custom_methods: Vec::new(),
            properties: vec![property("id", RuntimeDataType::Uuid), prop],
        }
    }

    fn property(name: &str, data_type: RuntimeDataType) -> RuntimePropertyMetadata {
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

    fn version_prop(name: &str, data_type: RuntimeDataType) -> RuntimePropertyMetadata {
        let mut prop = property(name, data_type);
        prop.is_concurrency_control = true;
        prop
    }

    #[test]
    fn get_record_version_returns_configured_concurrency_value() {
        let entity = entity_with_version_prop(version_prop("version", RuntimeDataType::Int64));
        let input = record(json!({
            "id": "00000000-0000-4000-8000-000000000001",
            "version": 42
        }));

        let version = get_record_version(&entity, &input).expect("version lookup should succeed");

        assert_eq!(version, Some(json!(42)));
    }

    #[test]
    fn get_record_version_returns_none_when_entity_has_no_concurrency_prop() {
        let mut entity = entity_with_version_prop(property("version", RuntimeDataType::Int64));
        for prop in &mut entity.properties {
            prop.is_concurrency_control = false;
        }
        let input = record(json!({ "version": 42 }));

        let version = get_record_version(&entity, &input).expect("version lookup should succeed");

        assert!(version.is_none());
    }

    #[test]
    fn get_record_version_uses_configured_property_name() {
        let entity = entity_with_version_prop(version_prop("row_version", RuntimeDataType::Int64));
        let input = record(json!({
            "version": 12,
            "row_version": 99
        }));

        let version = get_record_version(&entity, &input).expect("version lookup should succeed");

        assert_eq!(version, Some(json!(99)));
    }

    #[test]
    fn new_record_version_increments_int64_counter() {
        let prop = version_prop("version", RuntimeDataType::Int64);

        let first = new_record_version(&prop, &record(json!({})))
            .expect("new version should be generated")
            .expect("version value");
        assert_eq!(first, json!(1));

        let second = new_record_version(&prop, &record(json!({ "version": 7 })))
            .expect("new version should be generated")
            .expect("version value");
        assert_eq!(second, json!(8));
    }

    #[test]
    fn new_record_version_resets_javascript_unsafe_counters() {
        let prop = version_prop("version", RuntimeDataType::Int64);
        let unsafe_nano = 1_755_051_234_567_890_123_i64;
        let next = new_record_version(&prop, &record(json!({ "version": unsafe_nano })))
            .expect("new version should be generated")
            .expect("version value");
        assert_eq!(next, json!(1));
    }

    #[test]
    fn new_record_version_rejects_unsupported_concurrency_type() {
        let prop = version_prop("version", RuntimeDataType::String);

        let err = new_record_version(&prop, &record(json!({})))
            .expect_err("unsupported version type should fail");

        assert!(matches!(err, RuntimeError::Validation(_)));
        assert!(err
            .to_string()
            .contains("unsupported type for concurrency control 'version'"));
    }
}
