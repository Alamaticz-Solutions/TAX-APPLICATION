use appfw_runtime::{
    model_metadata::RuntimeDataType, provider_time_period, query_filter::filter_token, RuntimeError,
};
use serde_json::{Map, Value};

use crate::param::{param_placeholder, type_param, SqlParam};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MssqlFilterField {
    pub name: String,
    pub data_type: RuntimeDataType,
    pub is_required: bool,
}

pub fn create_equality_clause(
    field: &MssqlFilterField,
    val: Value,
    alias: &str,
    params: &mut Vec<SqlParam>,
) -> Result<String, RuntimeError> {
    let qualified = qualify(alias, &field.name);
    if matches!(val, Value::Null) {
        return Ok(format!("{} IS NULL", qualified));
    }
    let param = type_param(field.data_type, !field.is_required, &field.name, val)?;
    params.push(param);
    Ok(format!(
        "{} = {}",
        qualified,
        param_placeholder(params.len())
    ))
}

pub fn create_operator_clause(
    field: &MssqlFilterField,
    op_obj: &Map<String, Value>,
    alias: &str,
    params: &mut Vec<SqlParam>,
) -> Result<String, RuntimeError> {
    let qualified = qualify(alias, &field.name);
    let mut parts = Vec::new();

    for (op, val) in op_obj {
        let clause = if is_array_type(field.data_type) && is_array_operator(op) {
            array_clause(&qualified, field, op, val, params)?
        } else {
            match op.as_str() {
                filter_token::EQUALS => single(&qualified, "=", field, val.clone(), params)?,
                filter_token::NOT_EQUALS => single(&qualified, "<>", field, val.clone(), params)?,
                filter_token::LESS_THAN => single(&qualified, "<", field, val.clone(), params)?,
                filter_token::LESS_THAN_OR_EQUAL => {
                    single(&qualified, "<=", field, val.clone(), params)?
                }
                filter_token::GREATER_THAN => single(&qualified, ">", field, val.clone(), params)?,
                filter_token::GREATER_THAN_OR_EQUAL => {
                    single(&qualified, ">=", field, val.clone(), params)?
                }
                filter_token::IN => in_list(&qualified, "IN", field, val, params)?,
                filter_token::NOT_IN => in_list(&qualified, "NOT IN", field, val, params)?,
                filter_token::STARTS_WITH => {
                    like(&qualified, field, val.clone(), params, "{}%", false)?
                }
                filter_token::ENDS_WITH => {
                    like(&qualified, field, val.clone(), params, "%{}", false)?
                }
                filter_token::CONTAINS => {
                    like(&qualified, field, val.clone(), params, "%{}%", false)?
                }
                filter_token::NOT_CONTAINS => {
                    like(&qualified, field, val.clone(), params, "%{}%", true)?
                }
                filter_token::BEFORE => {
                    date_period(&qualified, field, val.clone(), params, DatePeriodOp::Before)?
                }
                filter_token::DURING => {
                    date_period(&qualified, field, val.clone(), params, DatePeriodOp::During)?
                }
                filter_token::AFTER => {
                    date_period(&qualified, field, val.clone(), params, DatePeriodOp::After)?
                }
                other => {
                    return Err(RuntimeError::Validation(format!(
                        "filter operator '{}' not yet supported by mssql client",
                        other
                    )))
                }
            }
        };
        parts.push(clause);
    }

    Ok(if parts.len() == 1 {
        parts.remove(0)
    } else {
        format!("({})", parts.join(" AND "))
    })
}

fn is_array_type(data_type: RuntimeDataType) -> bool {
    matches!(
        data_type,
        RuntimeDataType::StringArray
            | RuntimeDataType::EnumArray
            | RuntimeDataType::UuidArray
            | RuntimeDataType::ObjectIdArray
            | RuntimeDataType::Int8Array
            | RuntimeDataType::Int16Array
            | RuntimeDataType::Int32Array
            | RuntimeDataType::Int64Array
    )
}

fn is_array_operator(op: &str) -> bool {
    matches!(
        op,
        filter_token::CONTAINS
            | filter_token::NOT_CONTAINS
            | filter_token::CONTAINED_BY
            | filter_token::NOT_CONTAINED_BY
            | filter_token::OVERLAPS
            | filter_token::NOT_OVERLAPS
    )
}

fn element_type(data_type: RuntimeDataType) -> Result<RuntimeDataType, RuntimeError> {
    match data_type {
        RuntimeDataType::StringArray
        | RuntimeDataType::EnumArray
        | RuntimeDataType::UuidArray
        | RuntimeDataType::ObjectIdArray => Ok(RuntimeDataType::String),
        RuntimeDataType::Int8Array => Ok(RuntimeDataType::Int8),
        RuntimeDataType::Int16Array => Ok(RuntimeDataType::Int16),
        RuntimeDataType::Int32Array => Ok(RuntimeDataType::Int32),
        RuntimeDataType::Int64Array => Ok(RuntimeDataType::Int64),
        _ => Err(RuntimeError::Validation(
            "array operator requires an array property".to_string(),
        )),
    }
}

fn bind_array_item(
    field: &MssqlFilterField,
    val: Value,
    params: &mut Vec<SqlParam>,
) -> Result<String, RuntimeError> {
    let param = type_param(element_type(field.data_type)?, true, &field.name, val)?;
    params.push(param);
    Ok(param_placeholder(params.len()))
}

fn bind_array_items(
    field: &MssqlFilterField,
    val: &Value,
    params: &mut Vec<SqlParam>,
) -> Result<Vec<String>, RuntimeError> {
    let arr = val.as_array().ok_or_else(|| {
        RuntimeError::Validation(format!(
            "array operator on '{}' expects an array",
            field.name
        ))
    })?;
    let mut placeholders = Vec::with_capacity(arr.len());
    for item in arr {
        placeholders.push(bind_array_item(field, item.clone(), params)?);
    }
    Ok(placeholders)
}

fn array_clause(
    qualified: &str,
    field: &MssqlFilterField,
    op: &str,
    val: &Value,
    params: &mut Vec<SqlParam>,
) -> Result<String, RuntimeError> {
    match (op, val) {
        (filter_token::CONTAINS, Value::Array(_)) => {
            let placeholders = bind_array_items(field, val, params)?;
            if placeholders.is_empty() {
                return Ok("1 = 1".to_string());
            }
            let values = placeholders
                .iter()
                .map(|placeholder| format!("({placeholder})"))
                .collect::<Vec<_>>()
                .join(", ");
            Ok(format!(
                "NOT EXISTS (SELECT 1 FROM (VALUES {values}) AS [wanted]([value]) WHERE NOT EXISTS (SELECT 1 FROM OPENJSON({qualified}) AS [j] WHERE [j].[value] = [wanted].[value]))"
            ))
        }
        (filter_token::NOT_CONTAINS, Value::Array(_)) => {
            let contains = array_clause(qualified, field, filter_token::CONTAINS, val, params)?;
            Ok(format!("NOT ({contains})"))
        }
        (filter_token::CONTAINS, _) | (filter_token::NOT_CONTAINS, _) => {
            let placeholder = bind_array_item(field, val.clone(), params)?;
            let exists = format!(
                "EXISTS (SELECT 1 FROM OPENJSON({qualified}) AS [j] WHERE [j].[value] = {placeholder})"
            );
            Ok(if op == filter_token::NOT_CONTAINS {
                format!("NOT ({exists})")
            } else {
                exists
            })
        }
        (filter_token::OVERLAPS, Value::Array(_))
        | (filter_token::NOT_OVERLAPS, Value::Array(_)) => {
            let placeholders = bind_array_items(field, val, params)?;
            if placeholders.is_empty() {
                return Ok((if op == filter_token::OVERLAPS {
                    "1 = 0"
                } else {
                    "1 = 1"
                })
                .to_string());
            }
            let exists = format!(
                "EXISTS (SELECT 1 FROM OPENJSON({qualified}) AS [j] WHERE [j].[value] IN ({}))",
                placeholders.join(", ")
            );
            Ok(if op == filter_token::NOT_OVERLAPS {
                format!("NOT ({exists})")
            } else {
                exists
            })
        }
        (filter_token::CONTAINED_BY, Value::Array(_))
        | (filter_token::NOT_CONTAINED_BY, Value::Array(_)) => {
            let placeholders = bind_array_items(field, val, params)?;
            if placeholders.is_empty() {
                let clause = format!("NOT EXISTS (SELECT 1 FROM OPENJSON({qualified}) AS [j])");
                return Ok(if op == filter_token::NOT_CONTAINED_BY {
                    format!("NOT ({clause})")
                } else {
                    clause
                });
            }
            let clause = format!(
                "NOT EXISTS (SELECT 1 FROM OPENJSON({qualified}) AS [j] WHERE [j].[value] NOT IN ({}))",
                placeholders.join(", ")
            );
            Ok(if op == filter_token::NOT_CONTAINED_BY {
                format!("NOT ({clause})")
            } else {
                clause
            })
        }
        _ => Err(RuntimeError::Validation(format!(
            "array operator '{}' on '{}' has invalid value shape",
            op, field.name
        ))),
    }
}

fn single(
    qualified: &str,
    op: &str,
    field: &MssqlFilterField,
    val: Value,
    params: &mut Vec<SqlParam>,
) -> Result<String, RuntimeError> {
    if matches!(val, Value::Null) {
        return Ok(format!(
            "{} {} NULL",
            qualified,
            if op == "=" { "IS" } else { "IS NOT" }
        ));
    }
    let param = type_param(field.data_type, !field.is_required, &field.name, val)?;
    params.push(param);
    Ok(format!(
        "{} {} {}",
        qualified,
        op,
        param_placeholder(params.len())
    ))
}

fn in_list(
    qualified: &str,
    op: &str,
    field: &MssqlFilterField,
    val: &Value,
    params: &mut Vec<SqlParam>,
) -> Result<String, RuntimeError> {
    let arr = val
        .as_array()
        .ok_or_else(|| RuntimeError::Validation(format!("operator {} expects an array", op)))?;
    if arr.is_empty() {
        return Ok((if op == "IN" { "1 = 0" } else { "1 = 1" }).to_string());
    }
    let mut placeholders = Vec::with_capacity(arr.len());
    for item in arr {
        let param = type_param(
            field.data_type,
            !field.is_required,
            &field.name,
            item.clone(),
        )?;
        params.push(param);
        placeholders.push(param_placeholder(params.len()));
    }
    Ok(format!(
        "{} {} ({})",
        qualified,
        op,
        placeholders.join(", ")
    ))
}

fn like(
    qualified: &str,
    field: &MssqlFilterField,
    val: Value,
    params: &mut Vec<SqlParam>,
    fmt: &str,
    negate: bool,
) -> Result<String, RuntimeError> {
    let value = val.as_str().ok_or_else(|| {
        RuntimeError::Validation(format!(
            "LIKE operator on '{}' expects a string",
            field.name
        ))
    })?;
    let pattern = fmt.replace("{}", value);
    let param = type_param(
        RuntimeDataType::String,
        !field.is_required,
        &field.name,
        Value::String(pattern),
    )?;
    params.push(param);
    Ok(format!(
        "{} {}LIKE {}",
        qualified,
        if negate { "NOT " } else { "" },
        param_placeholder(params.len())
    ))
}

enum DatePeriodOp {
    Before,
    During,
    After,
}

fn date_period(
    qualified: &str,
    field: &MssqlFilterField,
    val: Value,
    params: &mut Vec<SqlParam>,
    op: DatePeriodOp,
) -> Result<String, RuntimeError> {
    if !matches!(
        field.data_type,
        RuntimeDataType::Date | RuntimeDataType::DateTime
    ) {
        return Err(RuntimeError::Validation(format!(
            "Invalid filter operator for property '{}'",
            field.name
        )));
    }
    let period = val
        .as_str()
        .ok_or_else(|| {
            RuntimeError::Validation(format!(
                "time-period operator on '{}' expects a string",
                field.name
            ))
        })?
        .to_string();
    let (start, end) = provider_time_period::get_period(&period, field.data_type)?;

    match op {
        DatePeriodOp::Before => {
            let param = type_param(field.data_type, !field.is_required, &field.name, start)?;
            params.push(param);
            Ok(format!(
                "{} < {}",
                qualified,
                param_placeholder(params.len())
            ))
        }
        DatePeriodOp::During => {
            let start_param = type_param(field.data_type, !field.is_required, &field.name, start)?;
            params.push(start_param);
            let start_ph = param_placeholder(params.len());
            let end_param = type_param(field.data_type, !field.is_required, &field.name, end)?;
            params.push(end_param);
            let end_ph = param_placeholder(params.len());
            Ok(format!("{} BETWEEN {} AND {}", qualified, start_ph, end_ph))
        }
        DatePeriodOp::After => {
            let param = type_param(field.data_type, !field.is_required, &field.name, end)?;
            params.push(param);
            Ok(format!(
                "{} > {}",
                qualified,
                param_placeholder(params.len())
            ))
        }
    }
}

fn qualify(alias: &str, col: &str) -> String {
    if alias.is_empty() {
        format!("[{}]", col)
    } else {
        format!("[{}].[{}]", alias, col)
    }
}

#[cfg(test)]
mod tests {
    use appfw_runtime::model_metadata::RuntimeDataType;
    use serde_json::json;

    use super::*;

    fn compact(input: &str) -> String {
        input.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn field(name: &str, data_type: RuntimeDataType) -> MssqlFilterField {
        MssqlFilterField {
            name: name.to_string(),
            data_type,
            is_required: false,
        }
    }

    #[test]
    fn builds_scalar_and_list_clauses() {
        let mut params = Vec::new();
        let name = field("name", RuntimeDataType::String);

        let clause =
            create_equality_clause(&name, json!("Ada"), "t0", &mut params).expect("equality");
        assert_eq!(clause, "[t0].[name] = @P1");

        let clause = create_operator_clause(
            &name,
            &json!({ "_starts": "A", "_not_in": ["X", "Y"] })
                .as_object()
                .unwrap()
                .clone(),
            "t0",
            &mut params,
        )
        .expect("operators");
        assert!(clause.contains("[t0].[name] LIKE @P2"));
        assert!(clause.contains("[t0].[name] NOT IN (@P3, @P4)"));
    }

    #[test]
    fn builds_record_locator_equality_clause() {
        let mut params = Vec::new();
        let locator = MssqlFilterField {
            name: "record_locator".to_string(),
            data_type: RuntimeDataType::String,
            is_required: true,
        };

        let clause = create_operator_clause(
            &locator,
            &json!({ "_eq": "rl_0123456789abcdef0123456789abcdef" })
                .as_object()
                .unwrap()
                .clone(),
            "t0",
            &mut params,
        )
        .expect("operator clause");

        assert_eq!(clause, "[t0].[record_locator] = @P1");
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn builds_array_and_date_period_clauses() {
        let mut params = Vec::new();
        let tags = field("tags", RuntimeDataType::StringArray);
        let clause = create_operator_clause(
            &tags,
            &json!({ "_contains": ["a", "b"] })
                .as_object()
                .unwrap()
                .clone(),
            "t0",
            &mut params,
        )
        .expect("array");
        assert!(compact(&clause).contains("OPENJSON([t0].[tags])"));

        let created_at = field("created_at", RuntimeDataType::DateTime);
        let clause = create_operator_clause(
            &created_at,
            &json!({ "_during": "_this_yr" })
                .as_object()
                .unwrap()
                .clone(),
            "t0",
            &mut params,
        )
        .expect("period");
        assert!(clause.contains("[t0].[created_at] BETWEEN"));
    }
}
