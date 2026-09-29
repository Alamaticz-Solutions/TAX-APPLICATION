use appfw_runtime::{
    model_metadata::RuntimeDataType, provider_time_period, query_filter::filter_token as operand,
    RuntimeError,
};
use bson::{doc, Bson, Document};
use serde_json::Value;

use crate::{
    json_number_to_bson, object_to_objectid, string_to_date_time, string_to_objectid,
    value_vec_to_num_vec, value_vec_to_objectid_vec, value_vec_to_str_vec, MONGO_PK_NAME,
    TYPE_PK_NAME,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MongoFilterProperty {
    pub name: String,
    pub data_type: RuntimeDataType,
    pub is_key: bool,
}

pub fn filter_field_name(prop: &MongoFilterProperty) -> String {
    if prop.is_key && prop.name == TYPE_PK_NAME && prop.data_type == RuntimeDataType::ObjectId {
        MONGO_PK_NAME.to_string()
    } else {
        prop.name.clone()
    }
}

pub fn create_criterion(
    prop: &MongoFilterProperty,
    op: &str,
    value: &Value,
) -> Result<Document, RuntimeError> {
    if value.is_null() {
        let prop_name = filter_field_name(prop);
        return match op {
            operand::EQUALS => Ok(doc! { prop_name: Bson::Null }),
            operand::NOT_EQUALS => Ok(doc! { prop_name: { "$ne": Bson::Null, "$exists": true } }),
            _ => Err(filter_op_value_err(prop, op, value)),
        };
    }

    match prop.data_type {
        RuntimeDataType::Boolean => bool_criterion(prop, op, value),
        RuntimeDataType::ObjectId => objectid_criterion(prop, op, value),
        RuntimeDataType::ObjectIdArray => objectid_array_criterion(prop, op, value),
        RuntimeDataType::String | RuntimeDataType::Uuid | RuntimeDataType::Enum => {
            string_criterion(prop, op, value)
        }
        RuntimeDataType::StringArray | RuntimeDataType::UuidArray | RuntimeDataType::EnumArray => {
            string_array_criterion(prop, op, value)
        }
        RuntimeDataType::Date | RuntimeDataType::DateTime | RuntimeDataType::Time => {
            date_time_criterion(prop, op, value)
        }
        RuntimeDataType::Int8
        | RuntimeDataType::Int16
        | RuntimeDataType::Int32
        | RuntimeDataType::Int64
        | RuntimeDataType::Float32
        | RuntimeDataType::Float64 => number_criterion(prop, op, value),
        RuntimeDataType::Int8Array
        | RuntimeDataType::Int16Array
        | RuntimeDataType::Int32Array
        | RuntimeDataType::Int64Array => number_array_criterion(prop, op, value),
        RuntimeDataType::NavToOne | RuntimeDataType::NavToMany | RuntimeDataType::ManyToMany => {
            Err(RuntimeError::Validation(format!(
                "Navigation property '{}' with type '{:?}' cannot be filtered directly in MongoDB. Use aggregation pipelines with $lookup for relationship filtering.",
                prop.name, prop.data_type
            )))
        }
        RuntimeDataType::Object => object_criterion(prop, op, value),
        RuntimeDataType::ObjectArray => object_array_criterion(prop, op, value),
        RuntimeDataType::Json => json_criterion(prop, op, value),
        RuntimeDataType::JsonArray => json_array_criterion(prop, op, value),
    }
}

pub fn qualify_criterion_fields(
    input: Document,
    base_field_names: &[String],
    qualified_field_name: &str,
) -> Document {
    let mut result = Document::new();
    for (key, value) in input {
        let key = if base_field_names.iter().any(|field| field == &key) {
            qualified_field_name.to_string()
        } else {
            key
        };
        result.insert(
            key,
            qualify_criterion_value(value, base_field_names, qualified_field_name),
        );
    }
    result
}

fn qualify_criterion_value(
    input: Bson,
    base_field_names: &[String],
    qualified_field_name: &str,
) -> Bson {
    match input {
        Bson::Document(doc) => Bson::Document(qualify_criterion_fields(
            doc,
            base_field_names,
            qualified_field_name,
        )),
        Bson::Array(items) => Bson::Array(
            items
                .into_iter()
                .map(|item| qualify_criterion_value(item, base_field_names, qualified_field_name))
                .collect(),
        ),
        other => other,
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

fn bool_criterion(
    prop: &MongoFilterProperty,
    op: &str,
    value: &Value,
) -> Result<Document, RuntimeError> {
    let prop_name = filter_field_name(prop);
    match value {
        Value::Bool(bool_value) => match op {
            operand::EQUALS => Ok(doc! { prop_name: bool_value }),
            operand::NOT_EQUALS => Ok(doc! { prop_name: { "$ne": bool_value } }),
            _ => Err(filter_op_value_err(prop, op, value)),
        },
        _ => Err(filter_op_value_err(prop, op, value)),
    }
}

fn objectid_criterion(
    prop: &MongoFilterProperty,
    op: &str,
    value: &Value,
) -> Result<Document, RuntimeError> {
    let prop_name = filter_field_name(prop);
    match value {
        Value::String(str_value) => {
            let oid_value = string_to_objectid(&prop.name, str_value)?;
            match op {
                operand::EQUALS => Ok(doc! { prop_name: oid_value }),
                operand::NOT_EQUALS => Ok(doc! { prop_name: { "$ne": oid_value } }),
                _ => Err(filter_op_value_err(prop, op, value)),
            }
        }
        Value::Object(obj_value) => {
            let oid_value = object_to_objectid(&prop.name, obj_value)?;
            match op {
                operand::EQUALS => Ok(doc! { prop_name: oid_value }),
                operand::NOT_EQUALS => Ok(doc! { prop_name: { "$ne": oid_value } }),
                _ => Err(filter_op_value_err(prop, op, value)),
            }
        }
        Value::Array(value_vec) => match op {
            operand::IN => {
                let oid_vec = value_vec_to_objectid_vec(&prop.name, value_vec)?;
                Ok(doc! { prop_name: { "$in": oid_vec } })
            }
            operand::NOT_IN => {
                let oid_vec = value_vec_to_objectid_vec(&prop.name, value_vec)?;
                Ok(doc! { prop_name: { "$nin": oid_vec } })
            }
            _ => Err(filter_op_value_err(prop, op, value)),
        },
        _ => Err(filter_op_value_err(prop, op, value)),
    }
}

fn objectid_array_criterion(
    prop: &MongoFilterProperty,
    op: &str,
    value: &Value,
) -> Result<Document, RuntimeError> {
    match value {
        Value::String(str_value) => {
            let oid_value = string_to_objectid(&prop.name, str_value)?;
            array_scalar_criterion(
                &filter_field_name(prop),
                op,
                vec![Bson::ObjectId(oid_value)],
            )
            .ok_or_else(|| filter_op_value_err(prop, op, value))
        }
        Value::Array(value_vec) => {
            let oid_vec = value_vec_to_objectid_vec(&prop.name, value_vec)?
                .into_iter()
                .map(Bson::ObjectId)
                .collect();
            array_list_criterion(&filter_field_name(prop), op, oid_vec)
                .ok_or_else(|| filter_op_value_err(prop, op, value))
        }
        _ => Err(filter_op_value_err(prop, op, value)),
    }
}

fn string_criterion(
    prop: &MongoFilterProperty,
    op: &str,
    value: &Value,
) -> Result<Document, RuntimeError> {
    let prop_name = filter_field_name(prop);
    match value {
        Value::String(str_value) => match op {
            operand::EQUALS => Ok(doc! { prop_name: str_value }),
            operand::NOT_EQUALS => Ok(doc! { prop_name: { "$ne": str_value } }),
            operand::REGEX => Ok(doc! { prop_name: { "$regex": str_value, "$options": "i" } }),
            operand::STARTS_WITH => {
                Ok(doc! { prop_name: { "$regex": format!("^{}", str_value), "$options": "i" } })
            }
            operand::CONTAINS => Ok(doc! { prop_name: { "$regex": str_value, "$options": "i" } }),
            operand::ENDS_WITH => {
                Ok(doc! { prop_name: { "$regex": format!("{}$", str_value), "$options": "i" } })
            }
            _ => Err(filter_op_value_err(prop, op, value)),
        },
        Value::Array(value_vec) => match op {
            operand::IN => {
                let str_vec = value_vec_to_str_vec(&prop.name, value_vec)?;
                Ok(doc! { prop_name: { "$in": str_vec } })
            }
            operand::NOT_IN => {
                let str_vec = value_vec_to_str_vec(&prop.name, value_vec)?;
                Ok(doc! { prop_name: { "$nin": str_vec } })
            }
            _ => Err(filter_op_value_err(prop, op, value)),
        },
        _ => Err(filter_op_value_err(prop, op, value)),
    }
}

fn string_array_criterion(
    prop: &MongoFilterProperty,
    op: &str,
    value: &Value,
) -> Result<Document, RuntimeError> {
    match value {
        Value::String(str_value) => array_scalar_criterion(
            &filter_field_name(prop),
            op,
            vec![Bson::String(str_value.clone())],
        )
        .ok_or_else(|| filter_op_value_err(prop, op, value)),
        Value::Array(value_vec) => {
            let str_vec = value_vec_to_str_vec(&prop.name, value_vec)?
                .into_iter()
                .map(Bson::String)
                .collect();
            array_list_criterion(&filter_field_name(prop), op, str_vec)
                .ok_or_else(|| filter_op_value_err(prop, op, value))
        }
        _ => Err(filter_op_value_err(prop, op, value)),
    }
}

fn date_time_criterion(
    prop: &MongoFilterProperty,
    op: &str,
    value: &Value,
) -> Result<Document, RuntimeError> {
    let prop_name = filter_field_name(prop);
    match value {
        Value::String(str_value) => match (prop.data_type, op) {
            (RuntimeDataType::Date, operand::BEFORE) => {
                let (start, _) = provider_time_period::get_period(str_value, prop.data_type)?;
                Ok(doc! { prop_name: { "$lt": period_bound_string(&start)? } })
            }
            (RuntimeDataType::Date, operand::DURING) => {
                let (start, end) = provider_time_period::get_period(str_value, prop.data_type)?;
                Ok(
                    doc! { prop_name: { "$gte": period_bound_string(&start)?, "$lt": period_bound_string(&end)? } },
                )
            }
            (RuntimeDataType::Date, operand::AFTER) => {
                let (_, end) = provider_time_period::get_period(str_value, prop.data_type)?;
                Ok(doc! { prop_name: { "$gt": period_bound_string(&end)? } })
            }
            (RuntimeDataType::DateTime, operand::BEFORE) => {
                let (start, _) = provider_time_period::get_period(str_value, prop.data_type)?;
                Ok(
                    doc! { prop_name: { "$lt": string_to_date_time(&prop.name, start.to_string())? } },
                )
            }
            (RuntimeDataType::DateTime, operand::DURING) => {
                let (start, end) = provider_time_period::get_period(str_value, prop.data_type)?;
                Ok(doc! { prop_name: {
                    "$gte": string_to_date_time(&prop.name, start.to_string())?,
                    "$lt": string_to_date_time(&prop.name, end.to_string())?
                } })
            }
            (RuntimeDataType::DateTime, operand::AFTER) => {
                let (_, end) = provider_time_period::get_period(str_value, prop.data_type)?;
                Ok(doc! { prop_name: { "$gt": string_to_date_time(&prop.name, end.to_string())? } })
            }
            (RuntimeDataType::Date | RuntimeDataType::Time, operand::EQUALS) => {
                Ok(doc! { prop_name: { "$eq": str_value.to_string() } })
            }
            (RuntimeDataType::Date | RuntimeDataType::Time, operand::NOT_EQUALS) => {
                Ok(doc! { prop_name: { "$ne": str_value.to_string() } })
            }
            (RuntimeDataType::Date | RuntimeDataType::Time, operand::LESS_THAN) => {
                Ok(doc! { prop_name: { "$lt": str_value.to_string() } })
            }
            (RuntimeDataType::Date | RuntimeDataType::Time, operand::LESS_THAN_OR_EQUAL) => {
                Ok(doc! { prop_name: { "$lte": str_value.to_string() } })
            }
            (RuntimeDataType::Date | RuntimeDataType::Time, operand::GREATER_THAN_OR_EQUAL) => {
                Ok(doc! { prop_name: { "$gte": str_value.to_string() } })
            }
            (RuntimeDataType::Date | RuntimeDataType::Time, operand::GREATER_THAN) => {
                Ok(doc! { prop_name: { "$gt": str_value.to_string() } })
            }
            (RuntimeDataType::DateTime, operand::EQUALS) => {
                Ok(doc! { prop_name: { "$eq": string_to_date_time(&prop.name, str_value)? } })
            }
            (RuntimeDataType::DateTime, operand::NOT_EQUALS) => {
                Ok(doc! { prop_name: { "$ne": string_to_date_time(&prop.name, str_value)? } })
            }
            (RuntimeDataType::DateTime, operand::LESS_THAN) => {
                Ok(doc! { prop_name: { "$lt": string_to_date_time(&prop.name, str_value)? } })
            }
            (RuntimeDataType::DateTime, operand::LESS_THAN_OR_EQUAL) => {
                Ok(doc! { prop_name: { "$lte": string_to_date_time(&prop.name, str_value)? } })
            }
            (RuntimeDataType::DateTime, operand::GREATER_THAN_OR_EQUAL) => {
                Ok(doc! { prop_name: { "$gte": string_to_date_time(&prop.name, str_value)? } })
            }
            (RuntimeDataType::DateTime, operand::GREATER_THAN) => {
                Ok(doc! { prop_name: { "$gt": string_to_date_time(&prop.name, str_value)? } })
            }
            _ => Err(filter_op_value_err(prop, op, value)),
        },
        _ => Err(filter_op_value_err(prop, op, value)),
    }
}

fn number_criterion(
    prop: &MongoFilterProperty,
    op: &str,
    value: &Value,
) -> Result<Document, RuntimeError> {
    let prop_name = filter_field_name(prop);
    match value {
        Value::Number(num_value) => {
            let bson_num = json_number_to_bson(&prop.name, num_value)?;
            match op {
                operand::EQUALS => Ok(doc! { prop_name: { "$eq": bson_num } }),
                operand::NOT_EQUALS => Ok(doc! { prop_name: { "$ne": bson_num } }),
                operand::LESS_THAN => Ok(doc! { prop_name: { "$lt": bson_num } }),
                operand::LESS_THAN_OR_EQUAL => Ok(doc! { prop_name: { "$lte": bson_num } }),
                operand::GREATER_THAN_OR_EQUAL => Ok(doc! { prop_name: { "$gte": bson_num } }),
                operand::GREATER_THAN => Ok(doc! { prop_name: { "$gt": bson_num } }),
                _ => Err(filter_op_value_err(prop, op, value)),
            }
        }
        Value::Array(value_vec) => {
            let num_vec = value_vec_to_num_vec(&prop.name, value_vec)?;
            array_list_criterion(&prop_name, op, num_vec)
                .ok_or_else(|| filter_op_value_err(prop, op, value))
        }
        _ => Err(filter_op_value_err(prop, op, value)),
    }
}

fn number_array_criterion(
    prop: &MongoFilterProperty,
    op: &str,
    value: &Value,
) -> Result<Document, RuntimeError> {
    let prop_name = filter_field_name(prop);
    match value {
        Value::Number(num_value) => {
            let bson_num = json_number_to_bson(&prop.name, num_value)?;
            match op {
                operand::EQUALS => Ok(doc! { prop_name: { "$eq": vec![bson_num] } }),
                operand::NOT_EQUALS => {
                    Ok(doc! { "$not": { prop_name: { "$ne": vec![bson_num] } } })
                }
                operand::CONTAINS => Ok(doc! { prop_name: { "$in": bson_num } }),
                operand::NOT_CONTAINS => Ok(doc! { "$not": { prop_name: { "$in": bson_num } } }),
                _ => Err(filter_op_value_err(prop, op, value)),
            }
        }
        Value::Array(value_vec) => {
            let num_vec = value_vec_to_num_vec(&prop.name, value_vec)?;
            array_list_criterion(&prop_name, op, num_vec)
                .ok_or_else(|| filter_op_value_err(prop, op, value))
        }
        _ => Err(filter_op_value_err(prop, op, value)),
    }
}

fn object_criterion(
    prop: &MongoFilterProperty,
    op: &str,
    value: &Value,
) -> Result<Document, RuntimeError> {
    match op {
        "eq" => match value.as_object() {
            Some(obj_value) => {
                let bson_doc = bson::to_document(obj_value).map_err(|e| {
                    RuntimeError::Validation(format!("Failed to convert object to BSON: {}", e))
                })?;
                Ok(doc! { prop.name.clone(): bson_doc })
            }
            None => Err(filter_op_value_err(prop, op, value)),
        },
        "ne" => match value.as_object() {
            Some(obj_value) => {
                let bson_doc = bson::to_document(obj_value).map_err(|e| {
                    RuntimeError::Validation(format!("Failed to convert object to BSON: {}", e))
                })?;
                Ok(doc! { prop.name.clone(): { "$ne": bson_doc } })
            }
            None => Err(filter_op_value_err(prop, op, value)),
        },
        _ => Err(filter_op_value_err(prop, op, value)),
    }
}

fn object_array_criterion(
    prop: &MongoFilterProperty,
    op: &str,
    value: &Value,
) -> Result<Document, RuntimeError> {
    match op {
        "has" => match value.as_object() {
            Some(obj_value) => {
                let bson_doc = bson::to_document(obj_value).map_err(|e| {
                    RuntimeError::Validation(format!("Failed to convert object to BSON: {}", e))
                })?;
                Ok(doc! { prop.name.clone(): { "$elemMatch": bson_doc } })
            }
            None => Err(filter_op_value_err(prop, op, value)),
        },
        _ => Err(filter_op_value_err(prop, op, value)),
    }
}

fn json_criterion(
    prop: &MongoFilterProperty,
    op: &str,
    value: &Value,
) -> Result<Document, RuntimeError> {
    match op {
        "eq" => {
            let bson_value = bson::to_bson(value).map_err(|e| {
                RuntimeError::Validation(format!("Failed to convert JSON to BSON: {}", e))
            })?;
            Ok(doc! { prop.name.clone(): bson_value })
        }
        "ne" => {
            let bson_value = bson::to_bson(value).map_err(|e| {
                RuntimeError::Validation(format!("Failed to convert JSON to BSON: {}", e))
            })?;
            Ok(doc! { prop.name.clone(): { "$ne": bson_value } })
        }
        _ => Err(filter_op_value_err(prop, op, value)),
    }
}

fn json_array_criterion(
    prop: &MongoFilterProperty,
    op: &str,
    value: &Value,
) -> Result<Document, RuntimeError> {
    match op {
        "has" => {
            let bson_value = bson::to_bson(value).map_err(|e| {
                RuntimeError::Validation(format!("Failed to convert JSON to BSON: {}", e))
            })?;
            Ok(doc! { prop.name.clone(): { "$elemMatch": { "$eq": bson_value } } })
        }
        _ => Err(filter_op_value_err(prop, op, value)),
    }
}

fn period_bound_string(value: &Value) -> Result<String, RuntimeError> {
    value.as_str().map(ToString::to_string).ok_or_else(|| {
        RuntimeError::Validation(format!(
            "date period bound must be a string, got {}",
            value_kind(value)
        ))
    })
}

fn array_scalar_criterion(field_name: &str, op: &str, items: Vec<Bson>) -> Option<Document> {
    let field_name = field_name.to_string();
    Some(match op {
        operand::EQUALS => doc! { field_name: { "$eq": items } },
        operand::NOT_EQUALS => doc! { "$not": { field_name: { "$ne": items } } },
        operand::CONTAINS => doc! { field_name: { "$in": items } },
        operand::NOT_CONTAINS => doc! { "$not": { field_name: { "$in": items } } },
        _ => return None,
    })
}

fn array_list_criterion(field_name: &str, op: &str, items: Vec<Bson>) -> Option<Document> {
    let field_name = field_name.to_string();
    Some(match op {
        operand::EQUALS => doc! { field_name: { "$eq": items } },
        operand::NOT_EQUALS => doc! { "$not": { field_name: { "$ne": items } } },
        operand::CONTAINS => doc! { field_name: { "$all": items } },
        operand::NOT_CONTAINS => doc! { field_name: { "$not": { "$all": items } } },
        operand::CONTAINED_BY => {
            doc! { "$not": { field_name: { "$elemMatch": { "$nin": items } } } }
        }
        operand::NOT_CONTAINED_BY => doc! { field_name: { "$elemMatch": { "$nin": items } } },
        operand::OVERLAPS => doc! { field_name: { "$elemMatch": { "$in": items } } },
        operand::NOT_OVERLAPS => {
            doc! { "$not": { field_name: { "$elemMatch": { "$in": items } } } }
        }
        _ => return None,
    })
}

fn filter_op_value_err(prop: &MongoFilterProperty, op: &str, value: &Value) -> RuntimeError {
    RuntimeError::Validation(format!(
        "filter::Invalid input value for property '{}', op: {:?}, value: {:?}",
        prop.name, op, value
    ))
}

#[cfg(test)]
mod tests {
    use bson::{oid::ObjectId, Bson};
    use serde_json::json;

    use super::*;

    fn prop(name: &str, data_type: RuntimeDataType) -> MongoFilterProperty {
        MongoFilterProperty {
            name: name.to_string(),
            data_type,
            is_key: name == TYPE_PK_NAME,
        }
    }

    #[test]
    fn maps_object_id_key_to_mongo_id_field() {
        let oid = ObjectId::parse_str("6730f3c8acb1a69db7a14e73").unwrap();
        let filter = create_criterion(
            &prop("id", RuntimeDataType::ObjectId),
            operand::EQUALS,
            &json!(oid.to_hex()),
        )
        .expect("criterion");

        assert_eq!(filter.get_object_id("_id").ok(), Some(oid));
        assert!(!filter.contains_key("id"));
    }

    #[test]
    fn builds_null_string_and_numeric_criteria() {
        let null_filter = create_criterion(
            &prop("name", RuntimeDataType::String),
            operand::NOT_EQUALS,
            &Value::Null,
        )
        .expect("null criterion");
        assert_eq!(
            null_filter
                .get_document("name")
                .unwrap()
                .get_bool("$exists"),
            Ok(true)
        );

        let string_filter = create_criterion(
            &prop("name", RuntimeDataType::String),
            operand::STARTS_WITH,
            &json!("Ac"),
        )
        .expect("string criterion");
        assert_eq!(
            string_filter
                .get_document("name")
                .unwrap()
                .get_str("$regex"),
            Ok("^Ac")
        );

        let number_filter = create_criterion(
            &prop("age", RuntimeDataType::Int32),
            operand::GREATER_THAN_OR_EQUAL,
            &json!(18),
        )
        .expect("number criterion");
        assert_eq!(
            number_filter.get_document("age").unwrap().get("$gte"),
            Some(&Bson::Int64(18))
        );
    }

    #[test]
    fn builds_record_locator_equality_criterion() {
        let filter = create_criterion(
            &prop("record_locator", RuntimeDataType::String),
            operand::EQUALS,
            &json!("rl_0123456789abcdef0123456789abcdef"),
        )
        .expect("locator criterion");

        assert_eq!(
            filter.get_str("record_locator"),
            Ok("rl_0123456789abcdef0123456789abcdef")
        );
    }

    #[test]
    fn builds_array_and_date_period_criteria() {
        let tags_filter = create_criterion(
            &prop("tags", RuntimeDataType::StringArray),
            operand::CONTAINS,
            &json!(["priority", "customer"]),
        )
        .expect("array criterion");
        assert_eq!(
            tags_filter.get_document("tags").unwrap().get_array("$all"),
            Ok(&vec![
                Bson::String("priority".to_string()),
                Bson::String("customer".to_string())
            ])
        );

        let date_filter = create_criterion(
            &prop("created_on", RuntimeDataType::Date),
            operand::DURING,
            &json!("_today"),
        )
        .expect("date criterion");
        let date_doc = date_filter.get_document("created_on").unwrap();
        assert!(date_doc.contains_key("$gte"));
        assert!(date_doc.contains_key("$lt"));
    }

    #[test]
    fn qualifies_nested_relation_criterion_fields() {
        let criterion = doc! { "_id": { "$ne": Bson::Null, "$exists": true } };
        let qualified = qualify_criterion_fields(
            criterion,
            &["_id".to_string(), "id".to_string()],
            "owner._id",
        );

        assert!(qualified.contains_key("owner._id"));
        assert!(!qualified.contains_key("_id"));
    }
}
