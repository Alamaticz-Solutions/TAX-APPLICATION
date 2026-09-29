use appfw_runtime::{model_metadata::RuntimeDataType, RuntimeError};
use bson::Bson;
use serde_json::Value;

use crate::{
    json_number_to_bson, object_to_objectid, string_to_date_time, string_to_objectid,
    value_vec_to_num_vec, value_vec_to_objectid_vec, value_vec_to_str_vec, MONGO_PK_NAME,
    TYPE_PK_NAME,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MongoRecordProperty {
    pub name: String,
    pub data_type: RuntimeDataType,
    pub is_required: bool,
    pub is_key: bool,
}

impl MongoRecordProperty {
    pub fn new(
        name: impl Into<String>,
        data_type: RuntimeDataType,
        is_required: bool,
        is_key: bool,
    ) -> Self {
        Self {
            name: name.into(),
            data_type,
            is_required,
            is_key,
        }
    }

    pub fn is_object_id_key(&self) -> bool {
        self.is_key && self.name == TYPE_PK_NAME && self.data_type == RuntimeDataType::ObjectId
    }
}

pub fn record_field_name(prop: &MongoRecordProperty) -> String {
    if prop.is_object_id_key() {
        MONGO_PK_NAME.to_string()
    } else {
        prop.name.clone()
    }
}

pub fn value_to_bson(prop: &MongoRecordProperty, value: &Value) -> Result<Bson, RuntimeError> {
    if value.is_null() {
        return if prop.is_required {
            Err(invalid_input(prop))
        } else {
            Ok(Bson::Null)
        };
    }

    match prop.data_type {
        RuntimeDataType::Boolean => bool_value(prop, value),

        RuntimeDataType::ObjectId => object_id_value(prop, value),
        RuntimeDataType::ObjectIdArray => object_id_array_value(prop, value),

        RuntimeDataType::String | RuntimeDataType::Uuid | RuntimeDataType::Enum => {
            string_value(prop, value)
        }

        RuntimeDataType::StringArray | RuntimeDataType::UuidArray | RuntimeDataType::EnumArray => {
            string_array_value(prop, value)
        }

        RuntimeDataType::Date | RuntimeDataType::Time => string_value(prop, value),

        RuntimeDataType::DateTime => date_time_value(prop, value),

        RuntimeDataType::Int8
        | RuntimeDataType::Int16
        | RuntimeDataType::Int32
        | RuntimeDataType::Int64
        | RuntimeDataType::Float32
        | RuntimeDataType::Float64 => number_value(prop, value),

        RuntimeDataType::Int8Array
        | RuntimeDataType::Int16Array
        | RuntimeDataType::Int32Array
        | RuntimeDataType::Int64Array => number_array_value(prop, value),

        RuntimeDataType::Json => json_value(value),
        RuntimeDataType::JsonArray => json_array_value(prop, value),

        RuntimeDataType::Object | RuntimeDataType::ObjectArray => Err(RuntimeError::Validation(
            format!(
                "Object property '{}' with type '{:?}' requires product nested conversion.",
                prop.name, prop.data_type
            ),
        )),

        RuntimeDataType::NavToOne | RuntimeDataType::NavToMany | RuntimeDataType::ManyToMany => {
            Err(RuntimeError::Validation(format!("Navigation property '{}' with type '{:?}' cannot be persisted directly. Navigation properties are resolved at query time.", prop.name, prop.data_type)))
        }
    }
}

fn bool_value(prop: &MongoRecordProperty, value: &Value) -> Result<Bson, RuntimeError> {
    match value {
        Value::Bool(bool_value) => Ok((*bool_value).into()),
        _ => Err(invalid_input(prop)),
    }
}

fn object_id_value(prop: &MongoRecordProperty, value: &Value) -> Result<Bson, RuntimeError> {
    match value {
        Value::String(str_value) => Ok(string_to_objectid(&prop.name, str_value)?.into()),
        Value::Object(obj_value) => Ok(object_to_objectid(&prop.name, obj_value)?.into()),
        _ => Err(RuntimeError::Validation(format!(
            "Invalid input value for objectid property '{}' {:?}",
            prop.name, value
        ))),
    }
}

fn object_id_array_value(prop: &MongoRecordProperty, value: &Value) -> Result<Bson, RuntimeError> {
    match value {
        Value::Array(value_vec) => Ok(value_vec_to_objectid_vec(&prop.name, value_vec)?.into()),
        _ => Err(invalid_input(prop)),
    }
}

fn string_value(prop: &MongoRecordProperty, value: &Value) -> Result<Bson, RuntimeError> {
    match value {
        Value::String(str_value) => Ok(str_value.into()),
        _ => Err(invalid_input(prop)),
    }
}

fn string_array_value(prop: &MongoRecordProperty, value: &Value) -> Result<Bson, RuntimeError> {
    match value {
        Value::Array(value_vec) => Ok(value_vec_to_str_vec(&prop.name, value_vec)?.into()),
        _ => Err(invalid_input(prop)),
    }
}

fn date_time_value(prop: &MongoRecordProperty, value: &Value) -> Result<Bson, RuntimeError> {
    match value {
        Value::String(str_value) => Ok(string_to_date_time(&prop.name, str_value)?.into()),
        _ => Err(invalid_input(prop)),
    }
}

fn number_value(prop: &MongoRecordProperty, value: &Value) -> Result<Bson, RuntimeError> {
    match value {
        Value::Number(num_value) => json_number_to_bson(&prop.name, num_value),
        _ => Err(invalid_input(prop)),
    }
}

fn number_array_value(prop: &MongoRecordProperty, value: &Value) -> Result<Bson, RuntimeError> {
    match value {
        Value::Array(value_vec) => Ok(value_vec_to_num_vec(&prop.name, value_vec)?.into()),
        _ => Err(invalid_input(prop)),
    }
}

fn json_value(value: &Value) -> Result<Bson, RuntimeError> {
    bson::to_bson(value).map_err(|e| RuntimeError::DataAccess(e.to_string()))
}

fn json_array_value(prop: &MongoRecordProperty, value: &Value) -> Result<Bson, RuntimeError> {
    match value {
        Value::Array(value_vec) => value_vec
            .iter()
            .map(json_value)
            .collect::<Result<Vec<Bson>, RuntimeError>>()
            .map(Into::into),
        _ => Err(invalid_input(prop)),
    }
}

fn invalid_input(prop: &MongoRecordProperty) -> RuntimeError {
    RuntimeError::Validation(format!(
        "to_bson::Invalid input value for property '{}'",
        prop.name
    ))
}

#[cfg(test)]
mod tests {
    use appfw_runtime::RuntimeError;
    use bson::Bson;
    use serde_json::{json, Value};

    use super::*;

    fn prop(name: &str, data_type: RuntimeDataType) -> MongoRecordProperty {
        MongoRecordProperty::new(name, data_type, true, false)
    }

    #[test]
    fn record_field_name_maps_object_id_primary_key() {
        let id = MongoRecordProperty::new("id", RuntimeDataType::ObjectId, true, true);
        let uuid = MongoRecordProperty::new("id", RuntimeDataType::Uuid, true, true);

        assert_eq!(record_field_name(&id), "_id");
        assert_eq!(record_field_name(&uuid), "id");
    }

    #[test]
    fn value_to_bson_converts_scalar_values() {
        assert_eq!(
            value_to_bson(&prop("active", RuntimeDataType::Boolean), &json!(true)).unwrap(),
            Bson::Boolean(true)
        );
        assert_eq!(
            value_to_bson(&prop("name", RuntimeDataType::String), &json!("Ada")).unwrap(),
            Bson::String("Ada".to_string())
        );
        assert_eq!(
            value_to_bson(&prop("score", RuntimeDataType::Int32), &json!(42)).unwrap(),
            Bson::Int64(42)
        );
    }

    #[test]
    fn value_to_bson_converts_object_id_and_arrays() {
        let id = "63a57582ba167433e41be35f";
        assert!(matches!(
            value_to_bson(&prop("id", RuntimeDataType::ObjectId), &json!(id)).unwrap(),
            Bson::ObjectId(_)
        ));
        assert!(matches!(
            value_to_bson(
                &prop("ids", RuntimeDataType::ObjectIdArray),
                &json!([id, { "$oid": "63a57582ba167433e41be360" }])
            )
            .unwrap(),
            Bson::Array(_)
        ));
        assert_eq!(
            value_to_bson(
                &prop("tags", RuntimeDataType::StringArray),
                &json!(["a", "b"])
            )
            .unwrap(),
            Bson::Array(vec![
                Bson::String("a".to_string()),
                Bson::String("b".to_string())
            ])
        );
    }

    #[test]
    fn value_to_bson_converts_json_and_datetime() {
        assert!(matches!(
            value_to_bson(
                &prop("created_at", RuntimeDataType::DateTime),
                &json!("2026-05-28T16:00:00Z")
            )
            .unwrap(),
            Bson::DateTime(_)
        ));
        assert!(matches!(
            value_to_bson(
                &prop("payload", RuntimeDataType::Json),
                &json!({"ok": true})
            )
            .unwrap(),
            Bson::Document(_)
        ));
        assert!(matches!(
            value_to_bson(
                &prop("events", RuntimeDataType::JsonArray),
                &json!([{"ok": true}])
            )
            .unwrap(),
            Bson::Array(_)
        ));
    }

    #[test]
    fn value_to_bson_enforces_nullability_and_invalid_shapes() {
        let required = MongoRecordProperty::new("name", RuntimeDataType::String, true, false);
        let optional = MongoRecordProperty::new("name", RuntimeDataType::String, false, false);

        assert!(matches!(
            value_to_bson(&required, &Value::Null),
            Err(RuntimeError::Validation(_))
        ));
        assert_eq!(value_to_bson(&optional, &Value::Null).unwrap(), Bson::Null);
        assert!(matches!(
            value_to_bson(&required, &json!(42)),
            Err(RuntimeError::Validation(_))
        ));
    }

    #[test]
    fn value_to_bson_rejects_navigation_properties() {
        let err = value_to_bson(
            &prop("contacts", RuntimeDataType::ManyToMany),
            &json!(["contact-1"]),
        )
        .unwrap_err();

        assert!(matches!(err, RuntimeError::Validation(_)));
    }
}
