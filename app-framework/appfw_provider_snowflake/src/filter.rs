use appfw_runtime::{
    model_metadata::RuntimeDataType, provider_time_period, query_filter::filter_token, RuntimeError,
};
use serde_json::{Map, Value};

use crate::statement::SnowflakeStatementBuilder;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnowflakeFilterField {
    pub name: String,
    pub data_type: RuntimeDataType,
    pub is_required: bool,
}

pub fn create_equality_clause(
    field: &SnowflakeFilterField,
    val: Value,
    alias: &str,
    builder: &mut SnowflakeStatementBuilder,
) -> Result<String, RuntimeError> {
    let qualified = qualify(alias, &field.name);
    if val.is_null() {
        return Ok(format!("{} IS NULL", qualified));
    }
    let value = builder.bind_scalar(field.data_type, &field.name, field.is_required, val)?;
    Ok(format!("{} = {}", qualified, value))
}

pub fn create_operator_clause(
    field: &SnowflakeFilterField,
    op_obj: &Map<String, Value>,
    alias: &str,
    builder: &mut SnowflakeStatementBuilder,
) -> Result<String, RuntimeError> {
    let qualified = qualify(alias, &field.name);
    let mut clauses = Vec::new();

    for (op, val) in op_obj {
        let clause = if is_array_type(field.data_type) && is_array_operator(op) {
            array_clause(&qualified, field, op, val, builder)?
        } else {
            match op.as_str() {
                filter_token::EQUALS => create_equality_clause(field, val.clone(), alias, builder)?,
                filter_token::NOT_EQUALS => {
                    if val.is_null() {
                        format!("{} IS NOT NULL", qualified)
                    } else {
                        single(&qualified, "<>", field, val.clone(), builder)?
                    }
                }
                filter_token::GREATER_THAN => single(&qualified, ">", field, val.clone(), builder)?,
                filter_token::GREATER_THAN_OR_EQUAL => {
                    single(&qualified, ">=", field, val.clone(), builder)?
                }
                filter_token::LESS_THAN => single(&qualified, "<", field, val.clone(), builder)?,
                filter_token::LESS_THAN_OR_EQUAL => {
                    single(&qualified, "<=", field, val.clone(), builder)?
                }
                filter_token::IN => in_list(&qualified, "IN", field, val, builder)?,
                filter_token::NOT_IN => in_list(&qualified, "NOT IN", field, val, builder)?,
                filter_token::STARTS_WITH => {
                    like(&qualified, field, val.clone(), "{}%", false, builder)?
                }
                filter_token::ENDS_WITH => {
                    like(&qualified, field, val.clone(), "%{}", false, builder)?
                }
                filter_token::CONTAINS => {
                    like(&qualified, field, val.clone(), "%{}%", false, builder)?
                }
                filter_token::NOT_CONTAINS => {
                    like(&qualified, field, val.clone(), "%{}%", true, builder)?
                }
                filter_token::BEFORE => date_period(
                    &qualified,
                    field,
                    val.clone(),
                    DatePeriodOp::Before,
                    builder,
                )?,
                filter_token::DURING => date_period(
                    &qualified,
                    field,
                    val.clone(),
                    DatePeriodOp::During,
                    builder,
                )?,
                filter_token::AFTER => {
                    date_period(&qualified, field, val.clone(), DatePeriodOp::After, builder)?
                }
                other => {
                    return Err(RuntimeError::Validation(format!(
                        "filter operator '{}' not yet supported by snowflake client",
                        other
                    )))
                }
            }
        };
        clauses.push(clause);
    }

    Ok(if clauses.len() == 1 {
        clauses.remove(0)
    } else {
        clauses
            .into_iter()
            .map(|clause| format!("({})", clause))
            .collect::<Vec<_>>()
            .join(" AND ")
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
    field: &SnowflakeFilterField,
    val: Value,
    builder: &mut SnowflakeStatementBuilder,
) -> Result<String, RuntimeError> {
    let placeholder =
        builder.bind_scalar(element_type(field.data_type)?, &field.name, false, val)?;
    Ok(format!("TO_VARIANT({placeholder})"))
}

fn bind_array_items(
    field: &SnowflakeFilterField,
    val: &Value,
    builder: &mut SnowflakeStatementBuilder,
) -> Result<Vec<String>, RuntimeError> {
    let arr = val.as_array().ok_or_else(|| {
        RuntimeError::Validation(format!(
            "array operator on '{}' expects an array",
            field.name
        ))
    })?;
    let mut placeholders = Vec::with_capacity(arr.len());
    for item in arr {
        placeholders.push(bind_array_item(field, item.clone(), builder)?);
    }
    Ok(placeholders)
}

fn array_contains_expr(item: &str, qualified: &str) -> String {
    format!("COALESCE(ARRAY_CONTAINS({item}, {qualified}), FALSE)")
}

fn array_clause(
    qualified: &str,
    field: &SnowflakeFilterField,
    op: &str,
    val: &Value,
    builder: &mut SnowflakeStatementBuilder,
) -> Result<String, RuntimeError> {
    match (op, val) {
        (filter_token::CONTAINS, Value::Array(_))
        | (filter_token::NOT_CONTAINS, Value::Array(_)) => {
            let items = bind_array_items(field, val, builder)?;
            if items.is_empty() {
                return Ok((if op == filter_token::CONTAINS {
                    "1 = 1"
                } else {
                    "1 = 0"
                })
                .to_string());
            }
            let clause = items
                .iter()
                .map(|item| array_contains_expr(item, qualified))
                .collect::<Vec<_>>()
                .join(" AND ");
            Ok(if op == filter_token::NOT_CONTAINS {
                format!("NOT ({clause})")
            } else {
                clause
            })
        }
        (filter_token::CONTAINS, _) | (filter_token::NOT_CONTAINS, _) => {
            let item = bind_array_item(field, val.clone(), builder)?;
            let clause = array_contains_expr(&item, qualified);
            Ok(if op == filter_token::NOT_CONTAINS {
                format!("NOT ({clause})")
            } else {
                clause
            })
        }
        (filter_token::OVERLAPS, Value::Array(_))
        | (filter_token::NOT_OVERLAPS, Value::Array(_)) => {
            let items = bind_array_items(field, val, builder)?;
            if items.is_empty() {
                return Ok((if op == filter_token::OVERLAPS {
                    "1 = 0"
                } else {
                    "1 = 1"
                })
                .to_string());
            }
            let clause = items
                .iter()
                .map(|item| array_contains_expr(item, qualified))
                .collect::<Vec<_>>()
                .join(" OR ");
            Ok(if op == filter_token::NOT_OVERLAPS {
                format!("NOT ({clause})")
            } else {
                clause
            })
        }
        (filter_token::CONTAINED_BY, Value::Array(_))
        | (filter_token::NOT_CONTAINED_BY, Value::Array(_)) => {
            let items = bind_array_items(field, val, builder)?;
            if items.is_empty() {
                let clause =
                    format!("NOT EXISTS (SELECT 1 FROM TABLE(FLATTEN(input => {qualified})) AS f)");
                return Ok(if op == filter_token::NOT_CONTAINED_BY {
                    format!("NOT ({clause})")
                } else {
                    clause
                });
            }
            let clause = format!(
                "NOT EXISTS (SELECT 1 FROM TABLE(FLATTEN(input => {qualified})) AS f WHERE f.value NOT IN ({}))",
                items.join(", ")
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
    field: &SnowflakeFilterField,
    val: Value,
    builder: &mut SnowflakeStatementBuilder,
) -> Result<String, RuntimeError> {
    let value = builder.bind_scalar(field.data_type, &field.name, field.is_required, val)?;
    Ok(format!("{} {} {}", qualified, op, value))
}

fn in_list(
    qualified: &str,
    op: &str,
    field: &SnowflakeFilterField,
    val: &Value,
    builder: &mut SnowflakeStatementBuilder,
) -> Result<String, RuntimeError> {
    let arr = val.as_array().ok_or_else(|| {
        RuntimeError::Validation(format!("{} expects an array for '{}'", op, field.name))
    })?;
    if arr.is_empty() {
        return Ok(if op == "IN" {
            "1 = 0".to_string()
        } else {
            "1 = 1".to_string()
        });
    }
    let mut values = Vec::with_capacity(arr.len());
    for item in arr {
        values.push(builder.bind_scalar(
            field.data_type,
            &field.name,
            field.is_required,
            item.clone(),
        )?);
    }
    Ok(format!("{} {} ({})", qualified, op, values.join(", ")))
}

fn like(
    qualified: &str,
    field: &SnowflakeFilterField,
    val: Value,
    fmt: &str,
    negate: bool,
    builder: &mut SnowflakeStatementBuilder,
) -> Result<String, RuntimeError> {
    if !matches!(
        field.data_type,
        RuntimeDataType::String
            | RuntimeDataType::Enum
            | RuntimeDataType::ObjectId
            | RuntimeDataType::Uuid
    ) {
        return Err(RuntimeError::Validation(format!(
            "LIKE operator is invalid for '{}'",
            field.name
        )));
    }
    let raw = val.as_str().ok_or_else(|| {
        RuntimeError::Validation(format!(
            "LIKE operator on '{}' expects a string",
            field.name
        ))
    })?;
    let pattern = fmt.replace("{}", raw);
    let placeholder = builder.bind_scalar(
        field.data_type,
        &field.name,
        field.is_required,
        Value::String(pattern),
    )?;
    Ok(format!(
        "{} {}LIKE {}",
        qualified,
        if negate { "NOT " } else { "" },
        placeholder
    ))
}

enum DatePeriodOp {
    Before,
    During,
    After,
}

fn date_period(
    qualified: &str,
    field: &SnowflakeFilterField,
    val: Value,
    op: DatePeriodOp,
    builder: &mut SnowflakeStatementBuilder,
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
            let start =
                builder.bind_scalar(field.data_type, &field.name, field.is_required, start)?;
            Ok(format!("{} < {}", qualified, start))
        }
        DatePeriodOp::During => {
            let start =
                builder.bind_scalar(field.data_type, &field.name, field.is_required, start)?;
            let end = builder.bind_scalar(field.data_type, &field.name, field.is_required, end)?;
            Ok(format!("{} BETWEEN {} AND {}", qualified, start, end))
        }
        DatePeriodOp::After => {
            let end = builder.bind_scalar(field.data_type, &field.name, field.is_required, end)?;
            Ok(format!("{} > {}", qualified, end))
        }
    }
}

fn qualify(alias: &str, col: &str) -> String {
    if alias.is_empty() {
        quote_ident(col)
    } else {
        format!("{}.{}", quote_ident(alias), quote_ident(col))
    }
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use appfw_runtime::model_metadata::RuntimeDataType;
    use serde_json::json;

    use super::*;

    fn field(name: &str, data_type: RuntimeDataType) -> SnowflakeFilterField {
        SnowflakeFilterField {
            name: name.to_string(),
            data_type,
            is_required: false,
        }
    }

    #[test]
    fn builds_scalar_and_list_clauses() {
        let mut builder = SnowflakeStatementBuilder::new();
        let name = field("name", RuntimeDataType::String);

        let clause =
            create_equality_clause(&name, json!("Acme"), "t0", &mut builder).expect("equality");
        assert_eq!(clause, "\"t0\".\"name\" = ?");

        let clause = create_operator_clause(
            &name,
            &json!({ "_starts": "A", "_not_in": ["X", "Y"] })
                .as_object()
                .unwrap()
                .clone(),
            "t0",
            &mut builder,
        )
        .expect("operators");
        assert!(clause.contains("\"t0\".\"name\" LIKE ?"));
        assert!(clause.contains("\"t0\".\"name\" NOT IN (?, ?)"));
    }

    #[test]
    fn builds_record_locator_equality_clause() {
        let mut builder = SnowflakeStatementBuilder::new();
        let locator = SnowflakeFilterField {
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
            &mut builder,
        )
        .expect("operator clause");

        assert_eq!(clause, "\"t0\".\"record_locator\" = ?");
        let statement = builder.finish("SELECT 1");
        assert_eq!(
            statement.body(1, "TEST_DB", None, None)["bindings"]
                .as_object()
                .map(|bindings| bindings.len()),
            Some(1)
        );
    }

    #[test]
    fn builds_array_and_date_period_clauses() {
        let mut builder = SnowflakeStatementBuilder::new();
        let tags = field("tags", RuntimeDataType::StringArray);
        let clause = create_operator_clause(
            &tags,
            &json!({ "_contains": ["priority", "customer"] })
                .as_object()
                .unwrap()
                .clone(),
            "t0",
            &mut builder,
        )
        .expect("array");
        assert!(clause.contains("ARRAY_CONTAINS(TO_VARIANT(?), \"t0\".\"tags\")"));

        let created_at = field("created_at", RuntimeDataType::DateTime);
        let clause = create_operator_clause(
            &created_at,
            &json!({ "_during": "_today" }).as_object().unwrap().clone(),
            "t0",
            &mut builder,
        )
        .expect("period");
        assert_eq!(
            clause,
            "\"t0\".\"created_at\" BETWEEN TO_TIMESTAMP_TZ(?) AND TO_TIMESTAMP_TZ(?)"
        );
    }
}
