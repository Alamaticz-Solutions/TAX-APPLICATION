use anyhow::{bail, Context, Result};
use serde_json::{Map, Value};

pub fn operation_object<'a>(
    response: &'a Value,
    operation_name: &str,
) -> Result<&'a Map<String, Value>> {
    response
        .get("data")
        .and_then(|data| data.get(operation_name))
        .and_then(Value::as_object)
        .with_context(|| format!("response did not contain data.{operation_name} object"))
}

pub fn operation_array<'a>(response: &'a Value, operation_name: &str) -> Result<&'a Vec<Value>> {
    response
        .get("data")
        .and_then(|data| data.get(operation_name))
        .and_then(Value::as_array)
        .with_context(|| format!("response did not contain data.{operation_name} array"))
}

pub fn operation_i64(response: &Value, operation_name: &str) -> Result<i64> {
    response
        .get("data")
        .and_then(|data| data.get(operation_name))
        .and_then(Value::as_i64)
        .with_context(|| format!("response did not contain data.{operation_name} integer"))
}

pub fn require_item<'a>(
    items: &'a [Value],
    field_name: &str,
    expected: &str,
) -> Result<&'a Map<String, Value>> {
    for item in items {
        let Some(obj) = item.as_object() else {
            bail!("query result item was not an object: {item:?}");
        };
        if obj.get(field_name).and_then(Value::as_str) == Some(expected) {
            return Ok(obj);
        }
    }

    bail!("query result did not include item where {field_name} = {expected}")
}
