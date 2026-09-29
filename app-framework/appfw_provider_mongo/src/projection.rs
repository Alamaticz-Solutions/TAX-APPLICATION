use appfw_runtime::{MetadataError, RuntimeError};
use bson::Document;
use serde_json::Value;

use crate::{MONGO_PK_NAME, TYPE_PK_NAME};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MongoProjectionField {
    pub name: String,
    pub is_object_id_key: bool,
}

impl MongoProjectionField {
    pub fn new(name: impl Into<String>, is_object_id_key: bool) -> Self {
        Self {
            name: name.into(),
            is_object_id_key,
        }
    }
}

pub fn create_projection(parent: &Value) -> Result<Document, RuntimeError> {
    let mut result = Document::new();
    for field_name in selection_names(parent)? {
        result.insert(raw_selection_field_name(&field_name), 1);
    }
    Ok(result)
}

pub fn create_projection_for_fields(
    fields: impl IntoIterator<Item = MongoProjectionField>,
) -> Document {
    let mut result = Document::new();
    result.insert(MONGO_PK_NAME, 0);

    for field in fields {
        result.insert(mongo_field_name(&field.name, field.is_object_id_key), 1);
    }

    result
}

pub fn selection_names(parent: &Value) -> Result<Vec<String>, RuntimeError> {
    let selection_set = parent
        .get("selection_set")
        .and_then(|sel_set| sel_set.as_array())
        .ok_or_else(|| {
            RuntimeError::Metadata(MetadataError::InvalidSelection(
                "selection_set must be an array".to_string(),
            ))
        })?;

    selection_set
        .iter()
        .map(|child| {
            child
                .get("name")
                .and_then(|name| name.as_str())
                .map(str::to_string)
                .ok_or_else(|| {
                    RuntimeError::Metadata(MetadataError::InvalidSelection(
                        "selection name must be a string".to_string(),
                    ))
                })
        })
        .collect()
}

pub fn raw_selection_field_name(field_name: &str) -> String {
    if field_name == TYPE_PK_NAME {
        MONGO_PK_NAME.to_string()
    } else {
        field_name.to_string()
    }
}

pub fn mongo_field_name(field_name: &str, is_object_id_key: bool) -> String {
    if is_object_id_key && field_name == TYPE_PK_NAME {
        MONGO_PK_NAME.to_string()
    } else {
        field_name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use appfw_runtime::RuntimeError;
    use bson::Bson;
    use serde_json::json;

    use super::*;

    #[test]
    fn raw_projection_maps_id_to_object_id_and_keeps_selected_fields() {
        let projection = create_projection(&json!({
          "name": "accounts",
          "selection_set": [
            { "name": "id", "selection_set": [] },
            { "name": "name", "selection_set": [] },
            { "name": "createdAt", "selection_set": [] }
          ]
        }))
        .expect("projection should build");

        assert_eq!(projection.get("_id"), Some(&Bson::Int32(1)));
        assert_eq!(projection.get("name"), Some(&Bson::Int32(1)));
        assert_eq!(projection.get("createdAt"), Some(&Bson::Int32(1)));
        assert!(!projection.contains_key("id"));
        assert!(!projection.contains_key("age"));
    }

    #[test]
    fn typed_projection_maps_object_id_key_and_suppresses_default_id() {
        let projection = create_projection_for_fields([
            MongoProjectionField::new("id", true),
            MongoProjectionField::new("name", false),
        ]);

        assert_eq!(projection.get("_id"), Some(&Bson::Int32(1)));
        assert_eq!(projection.get("name"), Some(&Bson::Int32(1)));
        assert!(!projection.contains_key("id"));
    }

    #[test]
    fn typed_projection_keeps_non_object_id_id_fields() {
        let projection = create_projection_for_fields([
            MongoProjectionField::new("id", false),
            MongoProjectionField::new("name", false),
        ]);

        assert_eq!(projection.get("_id"), Some(&Bson::Int32(0)));
        assert_eq!(projection.get("id"), Some(&Bson::Int32(1)));
        assert_eq!(projection.get("name"), Some(&Bson::Int32(1)));
    }

    #[test]
    fn projection_rejects_invalid_selection_shapes() {
        let err = create_projection(&json!({ "selection_set": "bad" })).unwrap_err();
        assert!(matches!(err, RuntimeError::Metadata(_)));

        let err = create_projection(&json!({
          "selection_set": [
            { "selection_set": [] }
          ]
        }))
        .unwrap_err();
        assert!(matches!(err, RuntimeError::Metadata(_)));
    }
}
