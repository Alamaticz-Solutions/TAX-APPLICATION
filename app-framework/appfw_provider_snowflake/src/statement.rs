use appfw_runtime::{model_metadata::RuntimeDataType, RuntimeError};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde_json::{json, Map, Value};

#[derive(Clone, Debug)]
pub struct SnowflakeStatement {
    sql: String,
    bindings: Vec<SnowflakeBinding>,
}

#[derive(Clone, Debug)]
struct SnowflakeBinding {
    snowflake_type: &'static str,
    value: Value,
}

impl SnowflakeStatement {
    fn new(sql: impl Into<String>, bindings: Vec<SnowflakeBinding>) -> Self {
        Self {
            sql: sql.into(),
            bindings,
        }
    }

    pub fn sql(&self) -> &str {
        &self.sql
    }

    pub fn body(
        &self,
        timeout_secs: u64,
        database: &str,
        warehouse: Option<&str>,
        role: Option<&str>,
    ) -> Value {
        let mut body = json!({
            "statement": self.sql,
            "timeout": timeout_secs,
            "database": database,
        });
        if let Some(warehouse) = warehouse {
            body["warehouse"] = Value::String(warehouse.to_string());
        }
        if let Some(role) = role {
            body["role"] = Value::String(role.to_string());
        }
        if !self.bindings.is_empty() {
            let mut bindings = Map::new();
            for (idx, binding) in self.bindings.iter().enumerate() {
                bindings.insert(
                    (idx + 1).to_string(),
                    json!({
                        "type": binding.snowflake_type,
                        "value": binding.value,
                    }),
                );
            }
            body["bindings"] = Value::Object(bindings);
        }
        body
    }
}

#[derive(Clone, Debug)]
pub struct SnowflakeStatementBuilder {
    bindings: Vec<SnowflakeBinding>,
}

impl SnowflakeStatementBuilder {
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    pub fn bind_scalar(
        &mut self,
        data_type: RuntimeDataType,
        prop_name: &str,
        is_required: bool,
        value: Value,
    ) -> Result<String, RuntimeError> {
        let placeholder = if value.is_null() {
            "?".to_string()
        } else {
            placeholder_expression(data_type)
        };
        let binding = SnowflakeBinding::from_parts(data_type, prop_name, is_required, value)?;
        self.bindings.push(binding);
        Ok(placeholder)
    }

    pub fn append(&mut self, other: Self) {
        self.bindings.extend(other.bindings);
    }

    pub fn finish(self, sql: impl Into<String>) -> SnowflakeStatement {
        SnowflakeStatement::new(sql, self.bindings)
    }
}

impl Default for SnowflakeStatementBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SnowflakeBinding {
    fn from_parts(
        data_type: RuntimeDataType,
        prop_name: &str,
        is_required: bool,
        value: Value,
    ) -> Result<Self, RuntimeError> {
        if value.is_null() {
            if is_required {
                return Err(RuntimeError::Validation(format!(
                    "Input for '{}' property must not be null",
                    prop_name
                )));
            }
            return Ok(Self {
                snowflake_type: "TEXT",
                value: Value::Null,
            });
        }

        let value = match data_type {
            RuntimeDataType::Boolean => Value::String(
                value
                    .as_bool()
                    .ok_or_else(|| type_mismatch(prop_name))?
                    .to_string(),
            ),
            RuntimeDataType::String
            | RuntimeDataType::Uuid
            | RuntimeDataType::ObjectId
            | RuntimeDataType::Enum => Value::String(
                value
                    .as_str()
                    .ok_or_else(|| type_mismatch(prop_name))?
                    .to_string(),
            ),
            RuntimeDataType::Date | RuntimeDataType::Time => Value::String(
                value
                    .as_str()
                    .ok_or_else(|| type_mismatch(prop_name))?
                    .to_string(),
            ),
            RuntimeDataType::DateTime => {
                let raw = value.as_str().ok_or_else(|| type_mismatch(prop_name))?;
                Value::String(parse_datetime(prop_name, raw)?.to_rfc3339())
            }
            RuntimeDataType::Int8
            | RuntimeDataType::Int16
            | RuntimeDataType::Int32
            | RuntimeDataType::Int64 => Value::String(
                value
                    .as_i64()
                    .ok_or_else(|| type_mismatch(prop_name))?
                    .to_string(),
            ),
            RuntimeDataType::Float32 | RuntimeDataType::Float64 => Value::String(
                value
                    .as_f64()
                    .ok_or_else(|| type_mismatch(prop_name))?
                    .to_string(),
            ),
            RuntimeDataType::StringArray
            | RuntimeDataType::EnumArray
            | RuntimeDataType::UuidArray
            | RuntimeDataType::ObjectIdArray
            | RuntimeDataType::Int8Array
            | RuntimeDataType::Int16Array
            | RuntimeDataType::Int32Array
            | RuntimeDataType::Int64Array
            | RuntimeDataType::ObjectArray
            | RuntimeDataType::JsonArray
            | RuntimeDataType::Object
            | RuntimeDataType::Json => Value::String(
                serde_json::to_string(&value)
                    .map_err(|e| RuntimeError::DataAccess(e.to_string()))?,
            ),
            RuntimeDataType::NavToOne
            | RuntimeDataType::NavToMany
            | RuntimeDataType::ManyToMany => {
                return Err(RuntimeError::Validation(format!(
                    "Cannot persist property '{}'",
                    prop_name
                )))
            }
        };

        Ok(Self {
            snowflake_type: binding_type(data_type),
            value,
        })
    }
}

fn placeholder_expression(data_type: RuntimeDataType) -> String {
    match data_type {
        RuntimeDataType::Int8
        | RuntimeDataType::Int16
        | RuntimeDataType::Int32
        | RuntimeDataType::Int64 => "TO_NUMBER(?)".to_string(),
        RuntimeDataType::Float32 | RuntimeDataType::Float64 => "TO_DOUBLE(?)".to_string(),
        RuntimeDataType::Date => "TO_DATE(?)".to_string(),
        RuntimeDataType::Time => "TO_TIME(?)".to_string(),
        RuntimeDataType::DateTime => "TO_TIMESTAMP_TZ(?)".to_string(),
        RuntimeDataType::StringArray
        | RuntimeDataType::EnumArray
        | RuntimeDataType::UuidArray
        | RuntimeDataType::ObjectIdArray
        | RuntimeDataType::Int8Array
        | RuntimeDataType::Int16Array
        | RuntimeDataType::Int32Array
        | RuntimeDataType::Int64Array
        | RuntimeDataType::ObjectArray
        | RuntimeDataType::JsonArray
        | RuntimeDataType::Object
        | RuntimeDataType::Json => "PARSE_JSON(?)".to_string(),
        _ => "?".to_string(),
    }
}

fn binding_type(data_type: RuntimeDataType) -> &'static str {
    match data_type {
        RuntimeDataType::Boolean => "BOOLEAN",
        RuntimeDataType::Int8
        | RuntimeDataType::Int16
        | RuntimeDataType::Int32
        | RuntimeDataType::Int64 => "FIXED",
        RuntimeDataType::Float32 | RuntimeDataType::Float64 => "REAL",
        RuntimeDataType::Date | RuntimeDataType::Time | RuntimeDataType::DateTime => "TEXT",
        RuntimeDataType::StringArray
        | RuntimeDataType::EnumArray
        | RuntimeDataType::UuidArray
        | RuntimeDataType::ObjectIdArray
        | RuntimeDataType::Int8Array
        | RuntimeDataType::Int16Array
        | RuntimeDataType::Int32Array
        | RuntimeDataType::Int64Array
        | RuntimeDataType::ObjectArray
        | RuntimeDataType::JsonArray
        | RuntimeDataType::Object
        | RuntimeDataType::Json => "TEXT",
        RuntimeDataType::NavToOne | RuntimeDataType::NavToMany | RuntimeDataType::ManyToMany => {
            "TEXT"
        }
        RuntimeDataType::String
        | RuntimeDataType::Uuid
        | RuntimeDataType::ObjectId
        | RuntimeDataType::Enum => "TEXT",
    }
}

fn parse_datetime(prop_name: &str, value: &str) -> Result<DateTime<Utc>, RuntimeError> {
    match DateTime::parse_from_rfc3339(value) {
        Ok(dt) => Ok(dt.with_timezone(&Utc)),
        Err(rfc3339_error) => {
            let naive = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f")
                .or_else(|_| NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.f"))
                .or_else(|_| {
                    NaiveDate::parse_from_str(value, "%Y-%m-%d")
                        .map(|date| date.and_hms_opt(0, 0, 0).expect("midnight is valid"))
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

fn type_mismatch(prop_name: &str) -> RuntimeError {
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
    fn statement_body_includes_snowflake_bindings() {
        let mut builder = SnowflakeStatementBuilder::new();
        let name_expr = builder
            .bind_scalar(
                RuntimeDataType::String,
                "name",
                true,
                Value::String("Acme".to_string()),
            )
            .expect("name binding");
        let age_expr = builder
            .bind_scalar(
                RuntimeDataType::Int64,
                "age",
                true,
                Value::Number(serde_json::Number::from(42)),
            )
            .expect("age binding");
        let statement = builder.finish(format!("SELECT {name_expr}, {age_expr}"));

        let body = statement.body(60, "APP_DB", Some("WH"), Some("ROLE"));

        assert_eq!(body["statement"], "SELECT ?, TO_NUMBER(?)");
        assert_eq!(body["bindings"]["1"]["type"], "TEXT");
        assert_eq!(body["bindings"]["1"]["value"], "Acme");
        assert_eq!(body["bindings"]["2"]["type"], "FIXED");
        assert_eq!(body["bindings"]["2"]["value"], "42");
        assert_eq!(body["warehouse"], "WH");
        assert_eq!(body["role"], "ROLE");
    }

    #[test]
    fn snowflake_bindings_validate_runtime_data_types() {
        let mut builder = SnowflakeStatementBuilder::new();
        assert_eq!(
            builder
                .bind_scalar(
                    RuntimeDataType::Date,
                    "service_date",
                    true,
                    json!("2026-05-29")
                )
                .unwrap(),
            "TO_DATE(?)"
        );
        assert_eq!(
            builder
                .bind_scalar(RuntimeDataType::Json, "payload", true, json!({"ok": true}))
                .unwrap(),
            "PARSE_JSON(?)"
        );
        assert_eq!(
            builder
                .bind_scalar(
                    RuntimeDataType::DateTime,
                    "created_at",
                    true,
                    json!("2026-05-29 15:45:00")
                )
                .unwrap(),
            "TO_TIMESTAMP_TZ(?)"
        );

        let body = builder.finish("SELECT 1").body(60, "APP_DB", None, None);

        assert_eq!(body["bindings"]["1"]["type"], "TEXT");
        assert_eq!(body["bindings"]["2"]["type"], "TEXT");
        assert_eq!(body["bindings"]["3"]["type"], "TEXT");
    }

    #[test]
    fn nullable_null_bindings_use_plain_placeholder() {
        let mut builder = SnowflakeStatementBuilder::new();
        assert_eq!(
            builder
                .bind_scalar(RuntimeDataType::Float64, "score", false, Value::Null)
                .unwrap(),
            "?"
        );
        assert_eq!(
            builder
                .bind_scalar(
                    RuntimeDataType::DateTime,
                    "refreshed_at",
                    false,
                    Value::Null
                )
                .unwrap(),
            "?"
        );

        let body = builder.finish("SELECT 1").body(60, "APP_DB", None, None);
        assert_eq!(body["bindings"]["1"]["type"], "TEXT");
        assert_eq!(body["bindings"]["2"]["type"], "TEXT");
        assert!(body["bindings"]["1"]["value"].is_null());
        assert!(body["bindings"]["2"]["value"].is_null());
    }

    #[test]
    fn snowflake_bindings_reject_invalid_values_with_runtime_errors() {
        let mut builder = SnowflakeStatementBuilder::new();
        assert!(matches!(
            builder.bind_scalar(RuntimeDataType::String, "name", true, Value::Null),
            Err(RuntimeError::Validation(_))
        ));
        assert!(matches!(
            builder.bind_scalar(RuntimeDataType::Int64, "age", true, json!("old")),
            Err(RuntimeError::Validation(_))
        ));
        assert!(matches!(
            builder.bind_scalar(RuntimeDataType::ManyToMany, "contacts", true, json!([])),
            Err(RuntimeError::Validation(_))
        ));
    }
}
