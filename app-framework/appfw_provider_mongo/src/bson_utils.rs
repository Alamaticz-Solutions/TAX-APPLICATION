use appfw_runtime::RuntimeError;
use bson::{oid::ObjectId, Bson, DateTime};
use serde_json::{Map, Value};

pub fn string_to_objectid(
    prop_name: &str,
    str_value: impl AsRef<str>,
) -> Result<ObjectId, RuntimeError> {
    let object_id = ObjectId::parse_str(str_value.as_ref()).map_err(|e| {
        RuntimeError::DataAccess(format!(
            "ObjectId parse error for property {}: {}",
            prop_name, e
        ))
    })?;
    Ok(object_id)
}

pub fn object_to_objectid(
    prop_name: &str,
    obj_value: &Map<String, Value>,
) -> Result<ObjectId, RuntimeError> {
    match obj_value.get("$oid") {
        None => Err(RuntimeError::Validation(format!(
            "Invalid input value for objectid property '{}' {:?}",
            prop_name, obj_value
        ))),
        Some(oid_value) => {
            let oid_value = oid_value.as_str().ok_or_else(|| {
                RuntimeError::Validation(format!(
                    "Invalid $oid value for objectid property '{}'",
                    prop_name
                ))
            })?;
            string_to_objectid(prop_name, oid_value)
        }
    }
}

pub fn json_number_to_bson(
    prop_name: &str,
    num: &serde_json::Number,
) -> Result<Bson, RuntimeError> {
    if let Some(n) = num.as_i64() {
        Ok(Bson::Int64(n))
    } else if let Some(n) = num.as_u64() {
        if n <= i64::MAX as u64 {
            Ok(Bson::Int64(n as i64))
        } else {
            Ok(Bson::Double(n as f64))
        }
    } else if let Some(n) = num.as_f64() {
        Ok(Bson::Double(n))
    } else {
        Err(RuntimeError::DataAccess(format!(
            "Value cannot be converted to Bson number for property {}",
            prop_name
        )))
    }
}

pub fn value_vec_to_num_vec(
    prop_name: &str,
    value_vec: &[Value],
) -> Result<Vec<Bson>, RuntimeError> {
    value_vec
        .iter()
        .map(|v| {
            let number = v.as_number().ok_or_else(|| {
                RuntimeError::Validation(format!(
                    "filter::Invalid input value for numeric property '{}'",
                    prop_name
                ))
            })?;
            json_number_to_bson(prop_name, number)
        })
        .collect::<Result<Vec<Bson>, RuntimeError>>()
}

pub fn value_vec_to_objectid_vec(
    prop_name: &str,
    value_vec: &[Value],
) -> Result<Vec<ObjectId>, RuntimeError> {
    value_vec
        .iter()
        .map(|v| match v {
            Value::String(str_value) => string_to_objectid(prop_name, str_value),
            Value::Object(obj_value) => object_to_objectid(prop_name, obj_value),
            _ => Err(RuntimeError::Validation(format!(
                "filter::Invalid input value for property '{}'",
                prop_name
            ))),
        })
        .collect::<Result<Vec<ObjectId>, RuntimeError>>()
}

pub fn value_vec_to_str_vec(
    prop_name: &str,
    value_vec: &[Value],
) -> Result<Vec<String>, RuntimeError> {
    value_vec
        .iter()
        .map(|v| match v {
            Value::String(str_value) => Ok(str_value.clone()),
            _ => Err(RuntimeError::Validation(format!(
                "filter::Invalid input value for property '{}'",
                prop_name
            ))),
        })
        .collect::<Result<Vec<String>, RuntimeError>>()
}

pub fn string_to_date_time(
    prop_name: &str,
    date_time_str: impl AsRef<str>,
) -> Result<DateTime, RuntimeError> {
    let date_time_str = date_time_str.as_ref().trim_matches('"').to_string();
    let chrono_date = if let Ok(value) = date_time_str.parse::<chrono::DateTime<chrono::Utc>>() {
        value
    } else if let Ok(value) = chrono::NaiveDate::parse_from_str(&date_time_str, "%Y-%m-%d") {
        value
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| {
                RuntimeError::DataAccess(format!(
                    "Date/time parse error for property {}: invalid date bound {}",
                    prop_name, date_time_str
                ))
            })?
            .and_utc()
    } else if let Ok(value) =
        chrono::NaiveDateTime::parse_from_str(&date_time_str, "%Y-%m-%dT%H:%M:%S")
    {
        value.and_utc()
    } else {
        return Err(RuntimeError::DataAccess(format!(
            "Date/time parse error for property {}: input is not RFC3339, date-only, or naive ISO datetime: {}",
            prop_name, date_time_str
        )));
    };

    Ok(DateTime::from_chrono(chrono_date))
}

#[cfg(test)]
mod tests {
    use appfw_runtime::RuntimeError;
    use bson::Bson;
    use serde_json::json;

    use super::*;

    #[test]
    fn parses_object_id_from_string_and_extended_json() {
        let id = "63a57582ba167433e41be35f";

        assert_eq!(string_to_objectid("id", id).unwrap().to_hex(), id);

        let object = json!({ "$oid": id }).as_object().unwrap().clone();
        assert_eq!(object_to_objectid("id", &object).unwrap().to_hex(), id);
    }

    #[test]
    fn rejects_invalid_object_id_values() {
        let err = string_to_objectid("id", "not-object-id").unwrap_err();
        assert!(matches!(err, RuntimeError::DataAccess(_)));

        let object = json!({ "$oid": 42 }).as_object().unwrap().clone();
        let err = object_to_objectid("id", &object).unwrap_err();
        assert!(matches!(err, RuntimeError::Validation(_)));
    }

    #[test]
    fn converts_json_numbers_to_bson_number_shapes() {
        assert_eq!(
            json_number_to_bson("amount", json!(42).as_number().unwrap()).unwrap(),
            Bson::Int64(42)
        );
        assert_eq!(
            json_number_to_bson("amount", json!(1.25).as_number().unwrap()).unwrap(),
            Bson::Double(1.25)
        );
    }

    #[test]
    fn converts_value_arrays_for_filter_inputs() {
        let ids = value_vec_to_objectid_vec(
            "id",
            &[
                json!("63a57582ba167433e41be35f"),
                json!({ "$oid": "63a57582ba167433e41be360" }),
            ],
        )
        .unwrap();
        assert_eq!(ids[0].to_hex(), "63a57582ba167433e41be35f");
        assert_eq!(ids[1].to_hex(), "63a57582ba167433e41be360");

        assert_eq!(
            value_vec_to_str_vec("name", &[json!("Ada"), json!("Grace")]).unwrap(),
            vec!["Ada".to_string(), "Grace".to_string()]
        );

        assert_eq!(
            value_vec_to_num_vec("score", &[json!(1), json!(2.5)]).unwrap(),
            vec![Bson::Int64(1), Bson::Double(2.5)]
        );
    }

    #[test]
    fn parses_supported_date_time_inputs() {
        assert_eq!(
            string_to_date_time("created_at", "2026-05-28T16:00:00Z")
                .unwrap()
                .timestamp_millis(),
            1_779_984_000_000_i64
        );
        assert_eq!(
            string_to_date_time("created_at", "2026-05-28")
                .unwrap()
                .timestamp_millis(),
            1_779_926_400_000_i64
        );
        assert_eq!(
            string_to_date_time("created_at", "\"2026-05-28T16:00:00\"")
                .unwrap()
                .timestamp_millis(),
            1_779_984_000_000_i64
        );
    }
}
