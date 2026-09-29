use appfw_runtime::{model_metadata::RuntimeDataType, RuntimeError};
use bson::{doc, oid::ObjectId, Document};
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::TYPE_PK_NAME;

pub fn is_empty_client_primary_key(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(value) => {
            let trimmed = value.trim();
            trimmed.is_empty() || trimmed == "00000000-0000-0000-0000-000000000000"
        }
        _ => false,
    }
}

pub fn create_primary_key_value(
    existing: Option<&Value>,
    data_type: RuntimeDataType,
    property_name: &str,
) -> Result<Value, RuntimeError> {
    if let Some(value) = existing {
        if !is_empty_client_primary_key(value) {
            return Ok(value.clone());
        }
    }
    new_primary_key_value(data_type, property_name)
}

pub fn new_primary_key_value(
    data_type: RuntimeDataType,
    property_name: &str,
) -> Result<Value, RuntimeError> {
    match data_type {
        RuntimeDataType::Uuid | RuntimeDataType::String => {
            Ok(Value::String(Uuid::new_v4().to_string()))
        }
        RuntimeDataType::ObjectId => Ok(Value::String(ObjectId::new().to_hex())),
        _ => Err(RuntimeError::Validation(format!(
            "MongoDB create requires an explicit primary key for non-string key '{}'",
            property_name
        ))),
    }
}

pub fn id_filter(id: Value, read_version: Option<Value>) -> Map<String, Value> {
    let mut filter = Map::new();
    filter.insert(TYPE_PK_NAME.to_string(), id);
    if let Some(version) = read_version {
        filter.insert("version".to_string(), version);
    }
    filter
}

pub fn set_update_document(record: Document) -> Document {
    doc! { "$set": record }
}

pub fn set_update_pipeline(record: Document) -> Vec<Document> {
    vec![set_update_document(record)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use bson::Bson;
    use serde_json::json;

    #[test]
    fn empty_client_primary_key_matches_create_semantics() {
        assert!(is_empty_client_primary_key(&Value::Null));
        assert!(is_empty_client_primary_key(&json!("")));
        assert!(is_empty_client_primary_key(&json!("  ")));
        assert!(is_empty_client_primary_key(&json!(
            "00000000-0000-0000-0000-000000000000"
        )));
        assert!(!is_empty_client_primary_key(&json!("abc")));
        assert!(!is_empty_client_primary_key(&json!(42)));
    }

    #[test]
    fn primary_key_generation_keeps_existing_or_generates_supported_keys() {
        assert_eq!(
            create_primary_key_value(Some(&json!("client-id")), RuntimeDataType::Uuid, "id")
                .expect("client id"),
            json!("client-id")
        );

        let uuid_value =
            create_primary_key_value(Some(&json!("")), RuntimeDataType::Uuid, "id").expect("uuid");
        assert!(uuid_value
            .as_str()
            .is_some_and(|value| Uuid::parse_str(value).is_ok()));

        let object_id_value =
            new_primary_key_value(RuntimeDataType::ObjectId, "id").expect("object id");
        assert!(object_id_value
            .as_str()
            .is_some_and(|value| ObjectId::parse_str(value).is_ok()));

        assert!(new_primary_key_value(RuntimeDataType::Int64, "tenant_id").is_err());
    }

    #[test]
    fn mutation_filters_and_update_shapes_are_provider_owned() {
        let filter = id_filter(json!("abc"), Some(json!(3)));
        assert_eq!(filter.get(TYPE_PK_NAME), Some(&json!("abc")));
        assert_eq!(filter.get("version"), Some(&json!(3)));

        let update = set_update_document(doc! { "name": "Acme" });
        assert!(matches!(update.get("$set"), Some(Bson::Document(_))));

        let pipeline = set_update_pipeline(doc! { "name": "Acme" });
        assert_eq!(pipeline.len(), 1);
        assert!(pipeline[0].contains_key("$set"));
    }
}
