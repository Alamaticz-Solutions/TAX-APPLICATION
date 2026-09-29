use appfw_runtime::RuntimeError;
use bson::Document;
use serde_json::{Map, Value};

pub type JsonObj = Map<String, Value>;

pub fn normalize_sort(input: Option<Value>) -> Result<Option<JsonObj>, RuntimeError> {
    match input {
        None => Ok(None),
        Some(Value::Null) => Ok(None),
        Some(Value::Object(o)) => Ok(if o.is_empty() { None } else { Some(o) }),
        Some(Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            match serde_json::from_str::<Value>(trimmed) {
                Ok(Value::Object(o)) => Ok(if o.is_empty() { None } else { Some(o) }),
                Ok(other) => Err(RuntimeError::Validation(format!(
                    "sort must be a JSON object or a JSON-encoded object string, got {}",
                    value_kind(&other)
                ))),
                Err(e) => Err(RuntimeError::Validation(format!(
                    "sort string is not valid JSON: {}",
                    e
                ))),
            }
        }
        Some(other) => Err(RuntimeError::Validation(format!(
            "sort must be a JSON object or a JSON-encoded object string, got {}",
            value_kind(&other)
        ))),
    }
}

pub fn create_sort_document(entries: impl IntoIterator<Item = (String, Value)>) -> Document {
    let mut bson_sort = Document::new();

    for (field, direction) in entries {
        bson_sort.insert(field, sort_direction(&direction));
    }

    bson_sort
}

pub fn sort_direction(value: &Value) -> i32 {
    match value.as_str().map(str::to_lowercase).as_deref() {
        Some("desc") => -1,
        _ => 1,
    }
}

fn value_kind(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use appfw_runtime::RuntimeError;
    use bson::Bson;
    use serde_json::json;

    use super::*;

    #[test]
    fn normalize_sort_accepts_empty_inputs() {
        for input in [None, Some(Value::Null), Some(json!({})), Some(json!(""))] {
            assert_eq!(normalize_sort(input).unwrap(), None);
        }
    }

    #[test]
    fn normalize_sort_accepts_object_and_json_string_specs() {
        let sort = normalize_sort(Some(json!({ "name": "DESC", "age": "sideways" })))
            .unwrap()
            .expect("object sort");
        assert_eq!(sort["name"], json!("DESC"));
        assert_eq!(sort["age"], json!("sideways"));

        let sort = normalize_sort(Some(json!(r#"{ "createdAt": "asc" }"#)))
            .unwrap()
            .expect("string sort");
        assert_eq!(sort["createdAt"], json!("asc"));
    }

    #[test]
    fn normalize_sort_rejects_invalid_shapes() {
        let err = normalize_sort(Some(json!(["name"]))).unwrap_err();
        assert!(matches!(err, RuntimeError::Validation(_)));

        let err = normalize_sort(Some(json!("{bad json"))).unwrap_err();
        assert!(matches!(err, RuntimeError::Validation(_)));
    }

    #[test]
    fn create_sort_document_maps_direction_values() {
        let sort = create_sort_document([
            ("name".to_string(), json!("DESC")),
            ("age".to_string(), json!("sideways")),
            ("created_at".to_string(), json!("asc")),
        ]);

        assert_eq!(sort.get("name"), Some(&Bson::Int32(-1)));
        assert_eq!(sort.get("age"), Some(&Bson::Int32(1)));
        assert_eq!(sort.get("created_at"), Some(&Bson::Int32(1)));
    }
}
