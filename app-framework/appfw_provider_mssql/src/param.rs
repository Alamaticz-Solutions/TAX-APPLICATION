use appfw_runtime::{model_metadata::RuntimeDataType, RuntimeError};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

/// Provider-owned SQL parameter for ODBC binding.
#[derive(Clone, Debug)]
pub enum SqlParam {
    Bit(Option<bool>),
    I16(Option<i16>),
    I32(Option<i32>),
    I64(Option<i64>),
    U8(Option<u8>),
    F32(Option<f32>),
    F64(Option<f64>),
    Guid(Option<Uuid>),
    String(Option<String>),
    Binary(Option<Vec<u8>>),
}

pub fn param_placeholder(ndx: usize) -> String {
    format!("@P{}", ndx)
}

pub fn type_param(
    data_type: RuntimeDataType,
    is_nullable: bool,
    prop_name: impl AsRef<str>,
    input_value: Value,
) -> Result<SqlParam, RuntimeError> {
    let prop_name = prop_name.as_ref();
    let p: SqlParam = match (data_type, input_value) {
        (RuntimeDataType::Boolean, Value::Bool(v)) => SqlParam::Bit(Some(v)),
        (RuntimeDataType::Boolean, Value::Null) => {
            ensure_nullable(is_nullable, prop_name)?;
            SqlParam::Bit(None)
        }

        (
            RuntimeDataType::String
            | RuntimeDataType::Date
            | RuntimeDataType::Time
            | RuntimeDataType::Enum,
            Value::String(v),
        ) => SqlParam::String(Some(v)),
        (
            RuntimeDataType::String
            | RuntimeDataType::Date
            | RuntimeDataType::Time
            | RuntimeDataType::Enum,
            Value::Null,
        ) => {
            ensure_nullable(is_nullable, prop_name)?;
            SqlParam::String(None)
        }

        (
            RuntimeDataType::StringArray
            | RuntimeDataType::EnumArray
            | RuntimeDataType::UuidArray
            | RuntimeDataType::ObjectIdArray
            | RuntimeDataType::Int8Array
            | RuntimeDataType::Int16Array
            | RuntimeDataType::Int32Array
            | RuntimeDataType::Int64Array
            | RuntimeDataType::ObjectArray
            | RuntimeDataType::JsonArray,
            Value::Array(v),
        ) => {
            let serialized = serde_json::to_string(&Value::Array(v))
                .map_err(|e| RuntimeError::DataAccess(e.to_string()))?;
            SqlParam::String(Some(serialized))
        }
        (
            RuntimeDataType::StringArray
            | RuntimeDataType::EnumArray
            | RuntimeDataType::UuidArray
            | RuntimeDataType::ObjectIdArray
            | RuntimeDataType::Int8Array
            | RuntimeDataType::Int16Array
            | RuntimeDataType::Int32Array
            | RuntimeDataType::Int64Array
            | RuntimeDataType::ObjectArray
            | RuntimeDataType::JsonArray,
            Value::Null,
        ) => {
            ensure_nullable(is_nullable, prop_name)?;
            SqlParam::String(None)
        }

        (RuntimeDataType::Uuid, Value::String(v)) => {
            let u = Uuid::parse_str(&v).map_err(|_| {
                RuntimeError::Validation(format!("Invalid UUID for property '{}'", prop_name))
            })?;
            SqlParam::Guid(Some(u))
        }
        (RuntimeDataType::Uuid, Value::Null) => {
            ensure_nullable(is_nullable, prop_name)?;
            SqlParam::Guid(None)
        }

        (RuntimeDataType::ObjectId, Value::String(v)) => SqlParam::String(Some(v)),
        (RuntimeDataType::ObjectId, Value::Null) => {
            ensure_nullable(is_nullable, prop_name)?;
            SqlParam::String(None)
        }

        (RuntimeDataType::Int8 | RuntimeDataType::Int16, Value::Number(n)) => {
            let i = n.as_i64().ok_or_else(|| invalid_number(prop_name))?;
            SqlParam::I16(Some(
                i16::try_from(i).map_err(|_| invalid_number(prop_name))?,
            ))
        }
        (RuntimeDataType::Int8 | RuntimeDataType::Int16, Value::Null) => {
            ensure_nullable(is_nullable, prop_name)?;
            SqlParam::I16(None)
        }

        (RuntimeDataType::Int32, Value::Number(n)) => {
            let i = n.as_i64().ok_or_else(|| invalid_number(prop_name))?;
            SqlParam::I32(Some(
                i32::try_from(i).map_err(|_| invalid_number(prop_name))?,
            ))
        }
        (RuntimeDataType::Int32, Value::Null) => {
            ensure_nullable(is_nullable, prop_name)?;
            SqlParam::I32(None)
        }

        (RuntimeDataType::Int64, Value::Number(n)) => {
            let i = n.as_i64().ok_or_else(|| invalid_number(prop_name))?;
            SqlParam::I64(Some(i))
        }
        (RuntimeDataType::Int64, Value::Null) => {
            ensure_nullable(is_nullable, prop_name)?;
            SqlParam::I64(None)
        }

        (RuntimeDataType::Float32, Value::Number(n)) => {
            let f = n.as_f64().ok_or_else(|| invalid_number(prop_name))?;
            SqlParam::F32(Some(f as f32))
        }
        (RuntimeDataType::Float32, Value::Null) => {
            ensure_nullable(is_nullable, prop_name)?;
            SqlParam::F32(None)
        }

        (RuntimeDataType::Float64, Value::Number(n)) => {
            let f = n.as_f64().ok_or_else(|| invalid_number(prop_name))?;
            SqlParam::F64(Some(f))
        }
        (RuntimeDataType::Float64, Value::Null) => {
            ensure_nullable(is_nullable, prop_name)?;
            SqlParam::F64(None)
        }

        (RuntimeDataType::DateTime, Value::String(v)) => {
            let dt = datetime_to_utc(&v, prop_name)?;
            SqlParam::String(Some(dt.to_rfc3339()))
        }
        (RuntimeDataType::DateTime, Value::Null) => {
            ensure_nullable(is_nullable, prop_name)?;
            SqlParam::String(None)
        }

        (RuntimeDataType::Object | RuntimeDataType::Json, Value::Object(o)) => {
            let serialized = serde_json::to_string(&Value::Object(o))
                .map_err(|e| RuntimeError::DataAccess(e.to_string()))?;
            SqlParam::String(Some(serialized))
        }
        (RuntimeDataType::Object | RuntimeDataType::Json, Value::Null) => {
            ensure_nullable(is_nullable, prop_name)?;
            SqlParam::String(None)
        }

        (
            RuntimeDataType::NavToOne | RuntimeDataType::NavToMany | RuntimeDataType::ManyToMany,
            _,
        ) => return Err(non_persistable(prop_name)),

        _ => return Err(invalid_type_data_combination(prop_name)),
    };
    Ok(p)
}

fn datetime_to_utc(v: &str, prop_name: &str) -> Result<DateTime<Utc>, RuntimeError> {
    match DateTime::parse_from_rfc3339(v) {
        Ok(dt) => Ok(dt.with_timezone(&Utc)),
        Err(rfc3339_error) => {
            let naive = NaiveDateTime::parse_from_str(v, "%Y-%m-%dT%H:%M:%S%.f")
                .or_else(|_| NaiveDateTime::parse_from_str(v, "%Y-%m-%d %H:%M:%S%.f"))
                .or_else(|_| {
                    NaiveDate::parse_from_str(v, "%Y-%m-%d")
                        .map(|d| d.and_hms_opt(0, 0, 0).expect("midnight is a valid time"))
                })
                .map_err(|_| {
                    RuntimeError::Validation(format!(
                        "Invalid datetime format for {}: {}",
                        prop_name, rfc3339_error
                    ))
                })?;
            Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
        }
    }
}

fn ensure_nullable(is_nullable: bool, prop_name: &str) -> Result<(), RuntimeError> {
    if is_nullable {
        Ok(())
    } else {
        Err(RuntimeError::Validation(format!(
            "Input for '{}' property must not be null",
            prop_name
        )))
    }
}

fn invalid_number(prop_name: &str) -> RuntimeError {
    RuntimeError::Validation(format!("Invalid input value for property '{}'", prop_name))
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn mssql_placeholders_are_provider_owned() {
        assert_eq!(param_placeholder(1), "@P1");
        assert_eq!(param_placeholder(12), "@P12");
    }

    #[test]
    fn mssql_params_validate_runtime_data_types() {
        assert!(matches!(
            type_param(RuntimeDataType::Boolean, false, "active", json!(true)).unwrap(),
            SqlParam::Bit(Some(true))
        ));
        assert!(matches!(
            type_param(
                RuntimeDataType::Uuid,
                false,
                "id",
                json!(Uuid::new_v4().to_string())
            )
            .unwrap(),
            SqlParam::Guid(Some(_))
        ));
        assert!(matches!(
            type_param(
                RuntimeDataType::DateTime,
                false,
                "created_at",
                json!("2026-05-29T15:45:00Z")
            )
            .unwrap(),
            SqlParam::String(Some(_))
        ));
        assert!(matches!(
            type_param(RuntimeDataType::Json, false, "payload", json!({"ok": true})).unwrap(),
            SqlParam::String(Some(_))
        ));
    }

    #[test]
    fn mssql_params_reject_invalid_values_with_runtime_errors() {
        assert!(matches!(
            type_param(RuntimeDataType::Int32, false, "age", json!(2147483648_i64)),
            Err(RuntimeError::Validation(_))
        ));
        assert!(matches!(
            type_param(RuntimeDataType::String, false, "name", Value::Null),
            Err(RuntimeError::Validation(_))
        ));
        assert!(matches!(
            type_param(RuntimeDataType::ManyToMany, false, "contacts", json!([])),
            Err(RuntimeError::Validation(_))
        ));
    }

    #[test]
    fn mssql_datetime_accepts_legacy_naive_formats() {
        for value in ["2026-05-29T15:45:00", "2026-05-29 15:45:00", "2026-05-29"] {
            assert!(matches!(
                type_param(RuntimeDataType::DateTime, false, "created_at", json!(value)).unwrap(),
                SqlParam::String(Some(_))
            ));
        }
    }
}
