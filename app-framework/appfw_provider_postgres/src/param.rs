use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use postgres_types::{Json, ToSql};
use serde_json::Value;
use uuid::Uuid;

use appfw_runtime::{model_metadata::RuntimeDataType, RuntimeError};

pub type SqlParam = Box<dyn ToSql + Sync + Send>;

pub fn prop_param_ref(data_type: RuntimeDataType, ndx: usize) -> Result<String, RuntimeError> {
    let placeholder = match data_type {
        RuntimeDataType::Json | RuntimeDataType::Object => {
            format!("${}::jsonb", ndx)
        }
        RuntimeDataType::JsonArray | RuntimeDataType::ObjectArray => {
            format!("${}::jsonb[]", ndx)
        }
        RuntimeDataType::Date => format!("${}::date", ndx),
        RuntimeDataType::Time => format!("${}::time", ndx),
        _ => format!("${}", ndx),
    };
    Ok(placeholder)
}

pub fn type_param(
    data_type: RuntimeDataType,
    is_nullable: bool,
    prop_name: impl AsRef<str>,
    input_value: Value,
) -> Result<SqlParam, RuntimeError> {
    let prop_name = prop_name.as_ref();
    let param_value: SqlParam = match (data_type, input_value) {
        (RuntimeDataType::Boolean, Value::Bool(v)) => Box::new(v),
        (RuntimeDataType::Boolean, Value::Null) => {
            Box::new(try_null::<bool>(is_nullable, prop_name)?)
        }

        (RuntimeDataType::String, Value::String(v)) => Box::new(v),
        (RuntimeDataType::String, Value::Null) => {
            Box::new(try_null::<String>(is_nullable, prop_name)?)
        }

        (RuntimeDataType::StringArray, Value::Array(v)) => {
            Box::new(value_vec_to_string_vec(v, prop_name)?)
        }
        (RuntimeDataType::StringArray, Value::Null) => {
            Box::new(try_null::<Vec<String>>(is_nullable, prop_name)?)
        }

        (RuntimeDataType::Uuid, Value::String(v)) => Box::new(string_to_uuid(v, prop_name)?),
        (RuntimeDataType::Uuid, Value::Null) => Box::new(try_null::<Uuid>(is_nullable, prop_name)?),

        (RuntimeDataType::UuidArray, Value::Array(v)) => {
            Box::new(value_vec_to_uuid_vec(v, prop_name)?)
        }
        (RuntimeDataType::UuidArray, Value::Null) => {
            Box::new(try_null::<Vec<Uuid>>(is_nullable, prop_name)?)
        }

        (RuntimeDataType::Int8, Value::Number(v)) => Box::new(number_to_int::<i8>(v, prop_name)?),
        (RuntimeDataType::Int8, Value::Null) => Box::new(try_null::<i8>(is_nullable, prop_name)?),

        (RuntimeDataType::Int16, Value::Number(v)) => Box::new(number_to_int::<i16>(v, prop_name)?),
        (RuntimeDataType::Int16, Value::Null) => Box::new(try_null::<i16>(is_nullable, prop_name)?),

        (RuntimeDataType::Int32, Value::Number(v)) => Box::new(number_to_int::<i32>(v, prop_name)?),
        (RuntimeDataType::Int32, Value::Null) => Box::new(try_null::<i32>(is_nullable, prop_name)?),

        (RuntimeDataType::Int64, Value::Number(v)) => Box::new(number_to_int::<i64>(v, prop_name)?),
        (RuntimeDataType::Int64, Value::String(v)) => {
            Box::new(v.parse::<i64>().map_err(|_| invalid_number(prop_name))?)
        }
        (RuntimeDataType::Int64, Value::Null) => Box::new(try_null::<i64>(is_nullable, prop_name)?),

        (RuntimeDataType::Int8Array, Value::Array(v)) => {
            Box::new(value_vec_to_int_vec::<i8>(v, prop_name)?)
        }
        (RuntimeDataType::Int8Array, Value::Null) => {
            Box::new(try_null::<Vec<i8>>(is_nullable, prop_name)?)
        }

        (RuntimeDataType::Int16Array, Value::Array(v)) => {
            Box::new(value_vec_to_int_vec::<i16>(v, prop_name)?)
        }
        (RuntimeDataType::Int16Array, Value::Null) => {
            Box::new(try_null::<Vec<i16>>(is_nullable, prop_name)?)
        }

        (RuntimeDataType::Int32Array, Value::Array(v)) => {
            Box::new(value_vec_to_int_vec::<i32>(v, prop_name)?)
        }
        (RuntimeDataType::Int32Array, Value::Null) => {
            Box::new(try_null::<Vec<i32>>(is_nullable, prop_name)?)
        }

        (RuntimeDataType::Int64Array, Value::Array(v)) => {
            Box::new(value_vec_to_int_vec::<i64>(v, prop_name)?)
        }
        (RuntimeDataType::Int64Array, Value::Null) => {
            Box::new(try_null::<Vec<i64>>(is_nullable, prop_name)?)
        }

        (RuntimeDataType::Float32, Value::Number(v)) => {
            Box::new(number_to_float::<f32>(v, prop_name)?)
        }
        (RuntimeDataType::Float32, Value::Null) => {
            Box::new(try_null::<f32>(is_nullable, prop_name)?)
        }

        (RuntimeDataType::Float64, Value::Number(v)) => {
            Box::new(number_to_float::<f64>(v, prop_name)?)
        }
        (RuntimeDataType::Float64, Value::Null) => {
            Box::new(try_null::<f64>(is_nullable, prop_name)?)
        }

        (RuntimeDataType::Date, Value::String(v)) => {
            Box::new(NaiveDate::parse_from_str(&v, "%Y-%m-%d").map_err(|e| {
                RuntimeError::Validation(format!("Invalid date format for {}: {}", prop_name, e))
            })?)
        }
        (RuntimeDataType::Date, Value::Null) => {
            Box::new(try_null::<NaiveDate>(is_nullable, prop_name)?)
        }

        (RuntimeDataType::DateTime, Value::String(v)) => {
            let datetime_with_tz = DateTime::parse_from_rfc3339(&v).map_err(|e| {
                RuntimeError::Validation(format!(
                    "Invalid datetime format for {}: {}",
                    prop_name, e
                ))
            })?;
            Box::new(datetime_with_tz.with_timezone(&Utc))
        }
        (RuntimeDataType::DateTime, Value::Null) => {
            Box::new(try_null::<DateTime<Utc>>(is_nullable, prop_name)?)
        }

        (RuntimeDataType::Time, Value::String(v)) => {
            Box::new(NaiveTime::parse_from_str(&v, "%H:%M:%S").map_err(|e| {
                RuntimeError::Validation(format!("Invalid time format for {}: {}", prop_name, e))
            })?)
        }
        (RuntimeDataType::Time, Value::Null) => {
            Box::new(try_null::<NaiveTime>(is_nullable, prop_name)?)
        }

        (RuntimeDataType::Enum, Value::String(v)) => Box::new(v),
        (RuntimeDataType::Enum, Value::Null) => {
            Box::new(try_null::<String>(is_nullable, prop_name)?)
        }

        (RuntimeDataType::EnumArray, Value::Array(v)) => {
            Box::new(value_vec_to_string_vec(v, prop_name)?)
        }
        (RuntimeDataType::EnumArray, Value::Null) => {
            Box::new(try_null::<Vec<String>>(is_nullable, prop_name)?)
        }

        (RuntimeDataType::Object | RuntimeDataType::Json, Value::Object(v)) => {
            Box::new(Json(v.clone()))
        }
        (RuntimeDataType::Object | RuntimeDataType::Json, Value::Null) => {
            Box::new(try_null::<String>(is_nullable, prop_name)?)
        }

        (RuntimeDataType::ObjectArray | RuntimeDataType::JsonArray, Value::Array(v)) => {
            Box::new(value_vec_to_json_vec(v))
        }
        (RuntimeDataType::ObjectArray | RuntimeDataType::JsonArray, Value::Null) => {
            Box::new(try_null::<Vec<Json<Value>>>(is_nullable, prop_name)?)
        }

        (RuntimeDataType::NavToOne | RuntimeDataType::NavToMany, _) => {
            return Err(non_persistable(prop_name));
        }

        _ => {
            return Err(invalid_type_data_combination(prop_name));
        }
    };

    Ok(param_value)
}

fn invalid_number(prop_name: &str) -> RuntimeError {
    RuntimeError::Validation(format!("Invalid input value for property '{}'", prop_name))
}

fn invalid_array_item(prop_name: &str) -> RuntimeError {
    RuntimeError::Validation(format!(
        "Invalid input array item value for property '{}'",
        prop_name
    ))
}

fn non_persistable(prop_name: &str) -> RuntimeError {
    RuntimeError::Validation(format!("Cannot persist property '{}'", prop_name))
}

fn invalid_type_data_combination(prop_name: &str) -> RuntimeError {
    RuntimeError::Validation(format!(
        "Type mismatch or unsupported type for property '{}'",
        prop_name
    ))
}

fn error_from_str(prop_name: &str, message: String) -> RuntimeError {
    RuntimeError::Validation(format!("Error: {}; property '{}'", &message, prop_name))
}

fn try_null<T>(is_nullable: bool, prop_name: &str) -> Result<Option<T>, RuntimeError> {
    if is_nullable {
        Ok(None)
    } else {
        Err(RuntimeError::Validation(format!(
            "Input for '{}' property must not be null",
            prop_name
        )))
    }
}

fn string_to_uuid(v: String, prop_name: &str) -> Result<Uuid, RuntimeError> {
    Uuid::parse_str(&v)
        .map_err(|_| RuntimeError::Validation(format!("Invalid UUID for property '{}'", prop_name)))
}

fn value_to_int<T>(v: serde_json::Value, prop_name: &str) -> Result<T, RuntimeError>
where
    T: TryFrom<i64>,
{
    match v {
        serde_json::Value::Number(n) => number_to_int(n, prop_name),
        _ => Err(invalid_number(prop_name)),
    }
}

fn number_to_int<T>(n: serde_json::Number, prop_name: &str) -> Result<T, RuntimeError>
where
    T: TryFrom<i64>,
{
    if let Some(int) = n.as_i64() {
        T::try_from(int).map_err(|_| invalid_number(prop_name))
    } else {
        Err(invalid_number(prop_name))
    }
}

fn number_to_float<T>(v: serde_json::Number, prop_name: &str) -> Result<T, RuntimeError>
where
    T: FromF64,
{
    if let Some(f) = v.as_f64() {
        T::from_f64(f).map_err(|s| error_from_str(prop_name, s))
    } else {
        Err(invalid_number(prop_name))
    }
}

fn value_vec_to_int_vec<T>(
    v: Vec<serde_json::Value>,
    prop_name: &str,
) -> Result<Vec<T>, RuntimeError>
where
    T: TryFrom<i64>,
{
    v.iter()
        .map(|i| value_to_int(i.clone(), prop_name))
        .collect()
}

fn value_vec_to_string_vec(
    v: Vec<serde_json::Value>,
    prop_name: &str,
) -> Result<Vec<String>, RuntimeError> {
    v.iter()
        .map(|i| match i {
            Value::String(s) => Ok(s.clone()),
            _ => Err(invalid_array_item(prop_name)),
        })
        .collect()
}

fn value_vec_to_uuid_vec(
    v: Vec<serde_json::Value>,
    prop_name: &str,
) -> Result<Vec<Uuid>, RuntimeError> {
    v.iter()
        .map(|i| match i {
            Value::String(s) => string_to_uuid(s.to_string(), prop_name),
            _ => Err(invalid_array_item(prop_name)),
        })
        .collect()
}

fn value_vec_to_json_vec(v: Vec<serde_json::Value>) -> Vec<Json<Value>> {
    v.iter().map(|i| Json(i.clone())).collect()
}

trait FromF64: Sized {
    fn from_f64(n: f64) -> Result<Self, String>;
}

impl FromF64 for f32 {
    fn from_f64(n: f64) -> Result<Self, String> {
        if n.is_finite() && n >= f32::MIN as f64 && n <= f32::MAX as f64 {
            Ok(n as f32)
        } else {
            Err(format!("Value {} is out of range for f32", n))
        }
    }
}

impl FromF64 for f64 {
    fn from_f64(n: f64) -> Result<Self, String> {
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn postgres_placeholders_are_provider_owned() {
        assert_eq!(prop_param_ref(RuntimeDataType::String, 1).unwrap(), "$1");
        assert_eq!(
            prop_param_ref(RuntimeDataType::Json, 2).unwrap(),
            "$2::jsonb"
        );
        assert_eq!(
            prop_param_ref(RuntimeDataType::JsonArray, 3).unwrap(),
            "$3::jsonb[]"
        );
        assert_eq!(
            prop_param_ref(RuntimeDataType::Date, 4).unwrap(),
            "$4::date"
        );
        assert_eq!(
            prop_param_ref(RuntimeDataType::Time, 5).unwrap(),
            "$5::time"
        );
    }

    #[test]
    fn postgres_params_validate_runtime_data_types() {
        assert!(type_param(RuntimeDataType::Boolean, false, "active", json!(true)).is_ok());
        assert!(type_param(
            RuntimeDataType::Uuid,
            false,
            "id",
            json!(Uuid::new_v4().to_string())
        )
        .is_ok());
        assert!(type_param(
            RuntimeDataType::Date,
            false,
            "service_date",
            json!("2026-05-29")
        )
        .is_ok());
        assert!(type_param(
            RuntimeDataType::DateTime,
            false,
            "created_at",
            json!("2026-05-29T15:45:00Z")
        )
        .is_ok());
        assert!(type_param(RuntimeDataType::Json, false, "payload", json!({"ok": true})).is_ok());
        assert!(type_param(RuntimeDataType::JsonArray, true, "tags", Value::Null).is_ok());
        assert!(type_param(RuntimeDataType::ObjectArray, true, "items", Value::Null).is_ok());
    }

    #[test]
    fn postgres_params_reject_invalid_values_with_runtime_errors() {
        assert!(matches!(
            type_param(RuntimeDataType::Int8, false, "age", json!(999)),
            Err(RuntimeError::Validation(_))
        ));
        assert!(matches!(
            type_param(RuntimeDataType::String, false, "name", Value::Null),
            Err(RuntimeError::Validation(_))
        ));
        assert!(matches!(
            type_param(RuntimeDataType::NavToOne, false, "owner", json!("1")),
            Err(RuntimeError::Validation(_))
        ));
    }
}
