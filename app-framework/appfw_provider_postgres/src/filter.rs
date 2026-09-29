use appfw_runtime::{
    model_metadata::RuntimeDataType, provider_time_period, query_filter::filter_token, RuntimeError,
};
use serde_json::Value;

use crate::param::{type_param, SqlParam};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostgresFilterField {
    pub name: String,
    pub data_type: RuntimeDataType,
    pub is_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostgresRelationExists {
    pub schema: String,
    pub table: String,
    pub nav_alias: String,
    pub source_alias: String,
    pub source_key: String,
    pub target_key: String,
    pub inner_filter: String,
}

pub fn create_criterion(
    field: &PostgresFilterField,
    op: &str,
    value: &Value,
    alias: &str,
    params: &mut Vec<SqlParam>,
) -> Result<Option<String>, RuntimeError> {
    if value.is_null() {
        return match op {
            filter_token::EQUALS => Ok(Some(format!("{}.{} IS NULL", alias, field.name))),
            filter_token::NOT_EQUALS => Ok(Some(format!("{}.{} IS NOT NULL", alias, field.name))),
            _ => Err(invalid_filter_operator(field, op)),
        };
    }

    match field.data_type {
        RuntimeDataType::Boolean => bool_criterion(field, op, value, alias, params),
        RuntimeDataType::String | RuntimeDataType::Uuid | RuntimeDataType::Enum => {
            string_criterion(field, op, value, alias, params)
        }
        RuntimeDataType::StringArray | RuntimeDataType::UuidArray | RuntimeDataType::EnumArray => {
            string_array_criterion(field, op, value, alias, params)
        }
        RuntimeDataType::Date | RuntimeDataType::DateTime | RuntimeDataType::Time => {
            date_time_criterion(field, op, value, alias, params)
        }
        RuntimeDataType::Int8
        | RuntimeDataType::Int16
        | RuntimeDataType::Int32
        | RuntimeDataType::Int64
        | RuntimeDataType::Float32
        | RuntimeDataType::Float64 => number_criterion(field, op, value, alias, params),
        RuntimeDataType::Int8Array
        | RuntimeDataType::Int16Array
        | RuntimeDataType::Int32Array
        | RuntimeDataType::Int64Array => number_array_criterion(field, op, value, alias, params),
        _ => Err(RuntimeError::Validation(format!(
            "Type mismatch or unsupported type for property '{}'",
            field.name
        ))),
    }
}

pub fn relation_exists_from_source_fk(relation: &PostgresRelationExists) -> String {
    format!(
        "EXISTS (SELECT 1 FROM {schema}.{table} {nav_alias} WHERE {source_alias}.{source_key} = {nav_alias}.{target_key} AND {inner_filter})",
        schema = relation.schema,
        table = relation.table,
        nav_alias = relation.nav_alias,
        source_alias = relation.source_alias,
        source_key = relation.source_key,
        target_key = relation.target_key,
        inner_filter = relation.inner_filter,
    )
}

pub fn relation_exists_from_target_fk(relation: &PostgresRelationExists) -> String {
    format!(
        "EXISTS (SELECT 1 FROM {schema}.{table} {nav_alias} WHERE {nav_alias}.{target_key} = {source_alias}.{source_key} AND {inner_filter})",
        schema = relation.schema,
        table = relation.table,
        nav_alias = relation.nav_alias,
        source_alias = relation.source_alias,
        source_key = relation.source_key,
        target_key = relation.target_key,
        inner_filter = relation.inner_filter,
    )
}

fn invalid_filter_operator(field: &PostgresFilterField, op: &str) -> RuntimeError {
    RuntimeError::Validation(format!(
        "Invalid filter operator '{}' for property '{}'",
        op, field.name
    ))
}

fn invalid_input(field: &PostgresFilterField) -> RuntimeError {
    RuntimeError::Validation(format!("Invalid input value for property '{}'", field.name))
}

fn bool_criterion(
    field: &PostgresFilterField,
    op: &str,
    value: &Value,
    alias: &str,
    params: &mut Vec<SqlParam>,
) -> Result<Option<String>, RuntimeError> {
    match value {
        Value::Bool(_) => {
            let oper = match op {
                filter_token::EQUALS => "=",
                filter_token::NOT_EQUALS => "<>",
                _ => return Err(invalid_filter_operator(field, op)),
            };
            params.push(type_param(
                RuntimeDataType::Boolean,
                true,
                &field.name,
                value.clone(),
            )?);
            Ok(Some(format!(
                "{}.{} {} ${}",
                alias,
                field.name,
                oper,
                params.len()
            )))
        }
        _ => Err(invalid_input(field)),
    }
}

fn string_criterion(
    field: &PostgresFilterField,
    op: &str,
    value: &Value,
    alias: &str,
    params: &mut Vec<SqlParam>,
) -> Result<Option<String>, RuntimeError> {
    let ndx = params.len() + 1;
    match value {
        Value::String(str_value) => {
            let res = match op {
                filter_token::EQUALS => (
                    format!("{}.{} = ${}", alias, field.name, ndx),
                    value.clone(),
                ),
                filter_token::NOT_EQUALS => (
                    format!("{}.{} <> ${}", alias, field.name, ndx),
                    value.clone(),
                ),
                filter_token::REGEX => (
                    format!("{}.{} ~ ${}", alias, field.name, ndx),
                    value.clone(),
                ),
                filter_token::STARTS_WITH => (
                    format!("{}.{} like ${}", alias, field.name, ndx),
                    Value::String(format!("{}%", str_value)),
                ),
                filter_token::CONTAINS => (
                    format!("{}.{} like ${}", alias, field.name, ndx),
                    Value::String(format!("%{}%", str_value)),
                ),
                filter_token::ENDS_WITH => (
                    format!("{}.{} like ${}", alias, field.name, ndx),
                    Value::String(format!("%{}", str_value)),
                ),
                _ => return Err(invalid_input(field)),
            };
            params.push(type_param(field.data_type, true, &field.name, res.1)?);
            Ok(Some(res.0))
        }
        Value::Array(_) => {
            let array_arg_type = match field.data_type {
                RuntimeDataType::Uuid => RuntimeDataType::UuidArray,
                RuntimeDataType::String => RuntimeDataType::StringArray,
                RuntimeDataType::Enum => RuntimeDataType::EnumArray,
                _ => return Err(invalid_input(field)),
            };
            let (oper, quantifier) = match op {
                filter_token::IN => ("=", "ANY"),
                filter_token::NOT_IN => ("<>", "ALL"),
                _ => return Err(invalid_filter_operator(field, op)),
            };
            params.push(type_param(
                array_arg_type,
                true,
                &field.name,
                value.clone(),
            )?);
            Ok(Some(format!(
                "{}.{} {} {}(${})",
                alias,
                field.name,
                oper,
                quantifier,
                params.len()
            )))
        }
        _ => Err(invalid_input(field)),
    }
}

fn string_array_criterion(
    field: &PostgresFilterField,
    op: &str,
    value: &Value,
    alias: &str,
    params: &mut Vec<SqlParam>,
) -> Result<Option<String>, RuntimeError> {
    let ndx = params.len() + 1;
    match value {
        Value::String(_) => {
            let singular_arg_type = match field.data_type {
                RuntimeDataType::UuidArray => RuntimeDataType::Uuid,
                RuntimeDataType::StringArray => RuntimeDataType::String,
                RuntimeDataType::EnumArray => RuntimeDataType::Enum,
                _ => return Err(invalid_input(field)),
            };
            let res = match op {
                filter_token::CONTAINS => format!("${} = ANY({}.{})", ndx, alias, field.name),
                filter_token::NOT_CONTAINS => {
                    format!("NOT (${} = ANY({}.{}))", ndx, alias, field.name)
                }
                _ => return Err(invalid_filter_operator(field, op)),
            };
            params.push(type_param(
                singular_arg_type,
                true,
                &field.name,
                value.clone(),
            )?);
            Ok(Some(res))
        }
        Value::Array(_) => {
            let res = match op {
                filter_token::CONTAINS => format!("{}.{} @> ${}", alias, field.name, ndx),
                filter_token::NOT_CONTAINS => {
                    format!("NOT ({}.{} @> ${})", alias, field.name, ndx)
                }
                filter_token::CONTAINED_BY => format!("{}.{} <@ ${}", alias, field.name, ndx),
                filter_token::NOT_CONTAINED_BY => {
                    format!("NOT ({}.{} <@ ${})", alias, field.name, ndx)
                }
                filter_token::OVERLAPS => format!("{}.{} && ${}", alias, field.name, ndx),
                filter_token::NOT_OVERLAPS => {
                    format!("NOT ({}.{} && ${})", alias, field.name, ndx)
                }
                _ => return Err(invalid_filter_operator(field, op)),
            };
            params.push(type_param(
                field.data_type,
                true,
                &field.name,
                value.clone(),
            )?);
            Ok(Some(res))
        }
        _ => Err(invalid_input(field)),
    }
}

fn date_time_criterion(
    field: &PostgresFilterField,
    op: &str,
    value: &Value,
    alias: &str,
    params: &mut Vec<SqlParam>,
) -> Result<Option<String>, RuntimeError> {
    match value {
        Value::String(period_or_value) => {
            let res = match (field.data_type, op) {
                (RuntimeDataType::Date | RuntimeDataType::DateTime, filter_token::BEFORE) => {
                    let (start, _) =
                        provider_time_period::get_period(period_or_value, field.data_type)?;
                    params.push(type_param(field.data_type, true, &field.name, start)?);
                    format!("{}.{} < ${}", alias, field.name, params.len())
                }
                (RuntimeDataType::Date | RuntimeDataType::DateTime, filter_token::DURING) => {
                    let (start, end) =
                        provider_time_period::get_period(period_or_value, field.data_type)?;
                    params.push(type_param(field.data_type, true, &field.name, start)?);
                    let start_ndx = params.len();
                    params.push(type_param(field.data_type, true, &field.name, end)?);
                    let end_ndx = params.len();
                    format!(
                        "{}.{} between ${} and ${}",
                        alias, field.name, start_ndx, end_ndx
                    )
                }
                (RuntimeDataType::Date | RuntimeDataType::DateTime, filter_token::AFTER) => {
                    let (_, end) =
                        provider_time_period::get_period(period_or_value, field.data_type)?;
                    params.push(type_param(field.data_type, true, &field.name, end)?);
                    format!("{}.{} > ${}", alias, field.name, params.len())
                }
                (
                    RuntimeDataType::Date | RuntimeDataType::DateTime | RuntimeDataType::Time,
                    filter_token::EQUALS,
                ) => {
                    params.push(type_param(
                        field.data_type,
                        true,
                        &field.name,
                        value.clone(),
                    )?);
                    format!("{}.{} = ${}", alias, field.name, params.len())
                }
                (
                    RuntimeDataType::Date | RuntimeDataType::DateTime | RuntimeDataType::Time,
                    filter_token::NOT_EQUALS,
                ) => {
                    params.push(type_param(
                        field.data_type,
                        true,
                        &field.name,
                        value.clone(),
                    )?);
                    format!("{}.{} <> ${}", alias, field.name, params.len())
                }
                (
                    RuntimeDataType::Date | RuntimeDataType::DateTime | RuntimeDataType::Time,
                    filter_token::LESS_THAN,
                ) => {
                    params.push(type_param(
                        field.data_type,
                        true,
                        &field.name,
                        value.clone(),
                    )?);
                    format!("{}.{} < ${}", alias, field.name, params.len())
                }
                (
                    RuntimeDataType::Date | RuntimeDataType::DateTime | RuntimeDataType::Time,
                    filter_token::LESS_THAN_OR_EQUAL,
                ) => {
                    params.push(type_param(
                        field.data_type,
                        true,
                        &field.name,
                        value.clone(),
                    )?);
                    format!("{}.{} <= ${}", alias, field.name, params.len())
                }
                (
                    RuntimeDataType::Date | RuntimeDataType::DateTime | RuntimeDataType::Time,
                    filter_token::GREATER_THAN_OR_EQUAL,
                ) => {
                    params.push(type_param(
                        field.data_type,
                        true,
                        &field.name,
                        value.clone(),
                    )?);
                    format!("{}.{} >= ${}", alias, field.name, params.len())
                }
                (
                    RuntimeDataType::Date | RuntimeDataType::DateTime | RuntimeDataType::Time,
                    filter_token::GREATER_THAN,
                ) => {
                    params.push(type_param(
                        field.data_type,
                        true,
                        &field.name,
                        value.clone(),
                    )?);
                    format!("{}.{} > ${}", alias, field.name, params.len())
                }
                _ => return Err(invalid_filter_operator(field, op)),
            };
            Ok(Some(res))
        }
        _ => Err(invalid_input(field)),
    }
}

fn number_criterion(
    field: &PostgresFilterField,
    op: &str,
    value: &Value,
    alias: &str,
    params: &mut Vec<SqlParam>,
) -> Result<Option<String>, RuntimeError> {
    match value {
        Value::Number(_) => {
            let oper = match op {
                filter_token::EQUALS => "=",
                filter_token::NOT_EQUALS => "<>",
                filter_token::LESS_THAN => "<",
                filter_token::LESS_THAN_OR_EQUAL => "<=",
                filter_token::GREATER_THAN_OR_EQUAL => ">=",
                filter_token::GREATER_THAN => ">",
                _ => return Err(invalid_filter_operator(field, op)),
            };
            params.push(type_param(
                field.data_type,
                true,
                &field.name,
                value.clone(),
            )?);
            Ok(Some(format!(
                "{}.{} {} ${}",
                alias,
                field.name,
                oper,
                params.len()
            )))
        }
        Value::Array(_) => {
            let array_arg_type = match field.data_type {
                RuntimeDataType::Int8 => RuntimeDataType::Int8Array,
                RuntimeDataType::Int16 => RuntimeDataType::Int16Array,
                RuntimeDataType::Int32 => RuntimeDataType::Int32Array,
                RuntimeDataType::Int64 => RuntimeDataType::Int64Array,
                _ => return Err(invalid_input(field)),
            };
            let (oper, quantifier) = match op {
                filter_token::IN => ("=", "ANY"),
                filter_token::NOT_IN => ("<>", "ALL"),
                _ => return Err(invalid_filter_operator(field, op)),
            };
            params.push(type_param(
                array_arg_type,
                true,
                &field.name,
                value.clone(),
            )?);
            Ok(Some(format!(
                "{}.{} {} {}(${})",
                alias,
                field.name,
                oper,
                quantifier,
                params.len()
            )))
        }
        _ => Err(invalid_input(field)),
    }
}

fn number_array_criterion(
    field: &PostgresFilterField,
    op: &str,
    value: &Value,
    alias: &str,
    params: &mut Vec<SqlParam>,
) -> Result<Option<String>, RuntimeError> {
    let ndx = params.len() + 1;
    match value {
        Value::Number(_) => {
            let singular_arg_type = match field.data_type {
                RuntimeDataType::Int8Array => RuntimeDataType::Int8,
                RuntimeDataType::Int16Array => RuntimeDataType::Int16,
                RuntimeDataType::Int32Array => RuntimeDataType::Int32,
                RuntimeDataType::Int64Array => RuntimeDataType::Int64,
                _ => return Err(invalid_input(field)),
            };
            let res = match op {
                filter_token::CONTAINS => format!("${} = ANY({}.{})", ndx, alias, field.name),
                filter_token::NOT_CONTAINS => {
                    format!("NOT (${} = ANY({}.{}))", ndx, alias, field.name)
                }
                _ => return Err(invalid_filter_operator(field, op)),
            };
            params.push(type_param(
                singular_arg_type,
                true,
                &field.name,
                value.clone(),
            )?);
            Ok(Some(res))
        }
        Value::Array(_) => {
            let res = match op {
                filter_token::CONTAINS => format!("{}.{} @> ${}", alias, field.name, ndx),
                filter_token::NOT_CONTAINS => {
                    format!("NOT ({}.{} @> ${})", alias, field.name, ndx)
                }
                filter_token::CONTAINED_BY => format!("{}.{} <@ ${}", alias, field.name, ndx),
                filter_token::NOT_CONTAINED_BY => {
                    format!("NOT ({}.{} <@ ${})", alias, field.name, ndx)
                }
                filter_token::OVERLAPS => format!("{}.{} && ${}", alias, field.name, ndx),
                filter_token::NOT_OVERLAPS => {
                    format!("NOT ({}.{} && ${})", alias, field.name, ndx)
                }
                _ => return Err(invalid_filter_operator(field, op)),
            };
            params.push(type_param(
                field.data_type,
                true,
                &field.name,
                value.clone(),
            )?);
            Ok(Some(res))
        }
        _ => Err(invalid_input(field)),
    }
}

#[cfg(test)]
mod tests {
    use appfw_runtime::model_metadata::RuntimeDataType;
    use serde_json::json;

    use super::*;

    fn field(name: &str, data_type: RuntimeDataType) -> PostgresFilterField {
        PostgresFilterField {
            name: name.to_string(),
            data_type,
            is_required: false,
        }
    }

    #[test]
    fn builds_scalar_and_list_clauses() {
        let mut params = Vec::new();
        let name = field("name", RuntimeDataType::String);

        let clause = create_criterion(&name, "_eq", &json!("Acme"), "t0", &mut params)
            .expect("criterion")
            .expect("clause");
        assert_eq!(clause, "t0.name = $1");

        let clause = create_criterion(
            &name,
            "_not_in",
            &json!(["Acme", "Globex"]),
            "t0",
            &mut params,
        )
        .expect("criterion")
        .expect("clause");
        assert_eq!(clause, "t0.name <> ALL($2)");
    }

    #[test]
    fn builds_record_locator_equality_clause() {
        let mut params = Vec::new();
        let locator = PostgresFilterField {
            name: "record_locator".to_string(),
            data_type: RuntimeDataType::String,
            is_required: true,
        };

        let clause = create_criterion(
            &locator,
            "_eq",
            &json!("rl_0123456789abcdef0123456789abcdef"),
            "t0",
            &mut params,
        )
        .expect("criterion")
        .expect("clause");

        assert_eq!(clause, "t0.record_locator = $1");
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn builds_null_array_and_period_clauses() {
        let mut params = Vec::new();

        let name = field("name", RuntimeDataType::String);
        let clause = create_criterion(&name, "_ne", &Value::Null, "t0", &mut params)
            .expect("criterion")
            .expect("clause");
        assert_eq!(clause, "t0.name IS NOT NULL");
        assert!(params.is_empty());

        let tags = field("tags", RuntimeDataType::StringArray);
        let clause = create_criterion(
            &tags,
            "_contains",
            &json!(["priority", "customer"]),
            "t0",
            &mut params,
        )
        .expect("criterion")
        .expect("clause");
        assert_eq!(clause, "t0.tags @> $1");

        let created_at = field("created_at", RuntimeDataType::DateTime);
        let clause = create_criterion(&created_at, "_during", &json!("_today"), "t0", &mut params)
            .expect("criterion")
            .expect("clause");
        assert_eq!(clause, "t0.created_at between $2 and $3");
    }

    #[test]
    fn renders_relation_exists_predicates() {
        let relation = PostgresRelationExists {
            schema: "crm".to_string(),
            table: "industries".to_string(),
            nav_alias: "industry_nav".to_string(),
            source_alias: "t0".to_string(),
            source_key: "industry_id".to_string(),
            target_key: "id".to_string(),
            inner_filter: "industry_nav.name = $1".to_string(),
        };

        assert_eq!(
            relation_exists_from_source_fk(&relation),
            "EXISTS (SELECT 1 FROM crm.industries industry_nav WHERE t0.industry_id = industry_nav.id AND industry_nav.name = $1)"
        );

        let inverse = PostgresRelationExists {
            schema: "crm".to_string(),
            table: "contacts".to_string(),
            nav_alias: "contacts_nav".to_string(),
            source_alias: "t0".to_string(),
            source_key: "id".to_string(),
            target_key: "account_id".to_string(),
            inner_filter: "contacts_nav.first_name like $1".to_string(),
        };

        assert_eq!(
            relation_exists_from_target_fk(&inverse),
            "EXISTS (SELECT 1 FROM crm.contacts contacts_nav WHERE contacts_nav.account_id = t0.id AND contacts_nav.first_name like $1)"
        );
    }
}
