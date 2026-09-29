use bson::{Bson, Document};
use serde_json::{Map, Value};

use crate::{MONGO_PK_NAME, TYPE_PK_NAME};

pub fn bson_vec_to_json_vec(bson_vec: Vec<Bson>) -> Vec<Map<String, Value>> {
    let mut items: Vec<Map<String, Value>> = Vec::new();
    for b in bson_vec {
        let mut project = bson_to_json_obj(b);
        if project.contains_key(MONGO_PK_NAME) {
            if let Some((_, val)) = project.remove_entry(MONGO_PK_NAME) {
                if !project.contains_key(TYPE_PK_NAME) {
                    project.insert(TYPE_PK_NAME.to_owned(), val);
                }
            }
        }
        items.push(project)
    }
    items
}

pub fn bson_to_json_obj(bson: Bson) -> Map<String, Value> {
    bson.as_document()
        .map(|doc| bson_doc_to_json_obj(doc.to_owned()))
        .unwrap_or_default()
}

pub fn bson_doc_to_json_obj(bson_doc: Document) -> Map<String, Value> {
    let result_json = serde_json::to_value(bson_doc).unwrap_or(Value::Null);
    let res = match &result_json {
        Value::Object(o) => Some(o),
        _ => None,
    };
    let Some(res) = res else {
        return Map::new();
    };
    let mut fixed = format_result_obj(res.to_owned());
    if fixed.contains_key(MONGO_PK_NAME) {
        if let Some((_, val)) = fixed.remove_entry(MONGO_PK_NAME) {
            if !fixed.contains_key(TYPE_PK_NAME) {
                fixed.insert(TYPE_PK_NAME.to_owned(), val);
            }
        }
    }
    fixed
}

pub fn format_result_obj(input: Map<String, Value>) -> Map<String, Value> {
    let input_v: Value = input.into();
    let result_v = format_result(input_v);
    result_v.as_object().cloned().unwrap_or_default()
}

pub fn format_result(input: Value) -> Value {
    match input {
        Value::Null => Value::Null,
        Value::Bool(b_val) => Value::Bool(b_val.to_owned()),
        Value::Number(n_val) => Value::Number(n_val.to_owned()),
        Value::String(s_val) => Value::String(s_val.to_owned()),
        Value::Array(a_val) => {
            let mut result: Vec<Value> = Vec::new();
            for array_item in a_val {
                result.push(format_result(array_item.to_owned()));
            }
            result.into()
        }
        Value::Object(o_val) => {
            let mut result: Map<String, Value> = Map::new();
            for key in o_val.keys() {
                match o_val.get(key) {
                    Some(obj_prop_val) => match obj_prop_val {
                        Value::Object(child_obj) => {
                            if child_obj.contains_key("$oid") {
                                if let Some(id) =
                                    child_obj.get("$oid").and_then(|value| value.as_str())
                                {
                                    result.insert(key.to_string(), Value::String(id.to_string()));
                                } else {
                                    result.insert(
                                        key.to_string(),
                                        format_result(obj_prop_val.to_owned()),
                                    );
                                }
                            } else if child_obj.contains_key("$date") {
                                if let Some(date_value) =
                                    child_obj.get("$date").and_then(format_mongo_date)
                                {
                                    result.insert(key.to_string(), Value::String(date_value));
                                } else {
                                    result.insert(
                                        key.to_string(),
                                        format_result(obj_prop_val.to_owned()),
                                    );
                                }
                            } else {
                                let o = format_result(obj_prop_val.to_owned());
                                result.insert(key.to_string(), o);
                            }
                        }
                        _ => {
                            let o = format_result(obj_prop_val.to_owned());
                            result.insert(key.to_string(), o);
                        }
                    },
                    None => {
                        result.insert(key.to_string(), Value::Null);
                    }
                }
            }
            if let Some(id_value) = result.remove(MONGO_PK_NAME) {
                result.entry(TYPE_PK_NAME.to_string()).or_insert(id_value);
            }
            result.into()
        }
    }
}

fn format_mongo_date(value: &Value) -> Option<String> {
    if let Some(value) = value.as_str() {
        return Some(value.to_string());
    }
    let millis = value
        .as_object()
        .and_then(|obj| obj.get("$numberLong"))
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<i64>().ok())?;
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(millis).map(|value| value.to_rfc3339())
}

#[cfg(test)]
mod tests {
    use bson::{doc, oid::ObjectId, Bson};
    use serde_json::json;

    use super::*;

    #[test]
    fn format_result_recursively_maps_object_id_field_to_id() {
        let formatted = format_result(json!({
            "_id": "root-internal",
            "id": "root-domain",
            "contacts": [
                { "_id": "contact-internal", "first_name": "Ada" }
            ],
            "industry": {
                "_id": "industry-internal",
                "name": "Manufacturing"
            }
        }));

        assert_eq!(formatted["id"], json!("root-domain"));
        assert!(formatted.get("_id").is_none());
        assert_eq!(formatted["contacts"][0]["id"], json!("contact-internal"));
        assert!(formatted["contacts"][0].get("_id").is_none());
        assert_eq!(formatted["industry"]["id"], json!("industry-internal"));
        assert!(formatted["industry"].get("_id").is_none());
    }

    #[test]
    fn bson_doc_to_json_obj_maps_extended_json_shapes() {
        let account_id = ObjectId::parse_str("63a57582ba167433e41be35f").unwrap();
        let contact_id = ObjectId::parse_str("63a57582ba167433e41be360").unwrap();
        let doc = doc! {
            "_id": account_id,
            "name": "Acme",
            "contacts": [ { "_id": contact_id, "first_name": "Ada" } ],
            "created_at": bson::DateTime::from_millis(1_779_984_000_000_i64),
        };

        let formatted = bson_doc_to_json_obj(doc);

        assert_eq!(formatted["id"], json!("63a57582ba167433e41be35f"));
        assert!(formatted.get("_id").is_none());
        assert_eq!(
            formatted["contacts"][0]["id"],
            json!("63a57582ba167433e41be360")
        );
        assert_eq!(formatted["created_at"], json!("2026-05-28T16:00:00+00:00"));
    }

    #[test]
    fn bson_vec_to_json_vec_maps_documents() {
        let id = ObjectId::parse_str("63a57582ba167433e41be35f").unwrap();
        let items = bson_vec_to_json_vec(vec![Bson::Document(doc! {
            "_id": id,
            "name": "Acme"
        })]);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["id"], json!("63a57582ba167433e41be35f"));
        assert_eq!(items[0]["name"], json!("Acme"));
    }
}
