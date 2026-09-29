// JSON filter DSL → T-SQL WHERE clause (MVP subset).
//
// Mirrors postgres/filter.rs but covers only the operators the framework
// actually needs for find / get / query against flat columns:
//   - top-level equality           {field: value}
//   - explicit operators           {field: {_eq|_ne|_lt|_lte|_gt|_gte: v}}
//   - membership                   {field: {_in|_not_in: [v...]}}
//   - LIKE patterns                {field: {_starts|_ends|_contains|_not_contains: "..."}}
//   - date periods                 {field: {_before|_during|_after: "_this_yr"}}
//   - JSON array operators         {field: {_contains|_overlaps|_contained_by: [...]}}
//   - boolean conjunctions         {_and: [...]}, {_or: [...]}
//
// Unsupported operators (regex, arbitrary nested-prop traversal, and
// many-to-many filters) return AppError::Validation rather than emit
// silently-wrong SQL. Parity with the postgres surface can grow incrementally.

use std::sync::Arc;

use appfw_provider_mssql::{
    create_equality_clause as provider_create_equality_clause,
    create_operator_clause as provider_create_operator_clause, MssqlFilterField,
};
use appfw_runtime::{
    identifier::to_snake_case_lenient as to_snake_case,
    query_filter::conjunction_token as conjunction,
};
use serde_json::{Map, Value};

use crate::{
    config::app_config::AppConfig,
    data::{clients::mssql::naming, query_ir::FilterAst},
    product_api::runtime_data_type,
    routes::app_error::{AppError, MetadataError},
    schemas::system::{DataType, EntityType, PropertyType},
};

use super::param::SqlParam;

type JsonObj = Map<String, Value>;

pub fn try_create_filter(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    input: Option<Value>,
    alias: &str,
    params: &mut Vec<SqlParam>,
) -> Result<Option<String>, AppError> {
    let normalized: Option<JsonObj> = match input {
        None | Some(Value::Null) => None,
        Some(Value::Object(o)) if o.is_empty() => None,
        Some(Value::Object(o)) => Some(o),
        Some(Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                match serde_json::from_str::<Value>(trimmed) {
                    Ok(Value::Object(o)) if o.is_empty() => None,
                    Ok(Value::Object(o)) => Some(o),
                    Ok(other) => {
                        return Err(AppError::Validation(format!(
                            "filter must be a JSON object or a JSON-encoded object string, got {}",
                            value_kind(&other)
                        )))
                    }
                    Err(e) => {
                        return Err(AppError::Validation(format!(
                            "filter string is not valid JSON: {}",
                            e
                        )))
                    }
                }
            }
        }
        Some(other) => {
            return Err(AppError::Validation(format!(
                "filter must be a JSON object or a JSON-encoded object string, got {}",
                value_kind(&other)
            )))
        }
    };

    match normalized {
        Some(o) => Ok(Some(create_filter(
            app_config,
            entity_type,
            o,
            alias,
            params,
        )?)),
        None => Ok(None),
    }
}

#[allow(dead_code)]
pub fn try_create_filter_ast(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    input: Option<&FilterAst>,
    alias: &str,
    params: &mut Vec<SqlParam>,
) -> Result<Option<String>, AppError> {
    try_create_filter(
        app_config,
        entity_type,
        input.and_then(FilterAst::to_filter_value),
        alias,
        params,
    )
}

fn create_filter(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    input: JsonObj,
    alias: &str,
    params: &mut Vec<SqlParam>,
) -> Result<String, AppError> {
    let mut clauses: Vec<String> = vec![];

    for (key, val) in input {
        match key.as_str() {
            conjunction::AND | conjunction::OR => {
                let conj = if key == conjunction::AND { "AND" } else { "OR" };
                if let Some(c) = create_conjunction(
                    app_config.clone(),
                    entity_type.clone(),
                    conj,
                    &val,
                    alias,
                    params,
                )? {
                    clauses.push(c);
                }
            }
            _ => {
                let prop_name = to_snake_case(&key);
                let prop_arc = AppConfig::try_get_prop(entity_type.clone(), &prop_name)
                    .ok_or_else(|| {
                        AppError::Validation(format!(
                            "invalid filter field '{}': property not found on {}",
                            key, entity_type.pascal_1
                        ))
                    })?;

                let clause = match (&prop_arc.data_type, &val) {
                    (DataType::NavToOne | DataType::NavToMany, Value::Object(nav_obj)) => {
                        create_nav_clause(
                            app_config.clone(),
                            entity_type.clone(),
                            prop_arc,
                            nav_obj,
                            alias,
                            params,
                        )?
                    }
                    (DataType::ManyToMany, _) => {
                        return Err(AppError::Validation(format!(
                            "ManyToMany filtering not yet implemented for property '{}'",
                            prop_arc.name
                        )))
                    }
                    (_, Value::Object(op_obj)) => provider_create_operator_clause(
                        &provider_filter_field(&prop_arc),
                        op_obj,
                        alias,
                        params,
                    )
                    .map_err(AppError::from)?,
                    _ => provider_create_equality_clause(
                        &provider_filter_field(&prop_arc),
                        val.clone(),
                        alias,
                        params,
                    )
                    .map_err(AppError::from)?,
                };
                clauses.push(clause);
            }
        }
    }

    if clauses.is_empty() {
        Ok("1 = 1".to_string())
    } else {
        Ok(clauses.join(" AND "))
    }
}

fn provider_filter_field(prop: &PropertyType) -> MssqlFilterField {
    MssqlFilterField {
        name: naming::property_column(prop),
        data_type: runtime_data_type(prop.data_type),
        is_required: prop.is_required,
    }
}

fn create_conjunction(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    conj: &str,
    val: &Value,
    alias: &str,
    params: &mut Vec<SqlParam>,
) -> Result<Option<String>, AppError> {
    let arr = val.as_array().ok_or_else(|| {
        AppError::Validation(format!(
            "filter conjunction must be an array, got {}",
            value_kind(val)
        ))
    })?;

    let mut parts: Vec<String> = vec![];
    for item in arr {
        if let Value::Object(o) = item {
            let part = create_filter(
                app_config.clone(),
                entity_type.clone(),
                o.clone(),
                alias,
                params,
            )?;
            if !part.trim().is_empty() && part.trim() != "1 = 1" {
                parts.push(part);
            }
        } else {
            return Err(AppError::Validation(format!(
                "filter conjunction items must be objects, got {}",
                value_kind(item)
            )));
        }
    }

    Ok(if parts.is_empty() {
        None
    } else {
        Some(format!("({})", parts.join(&format!(" {} ", conj))))
    })
}

fn create_nav_clause(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    prop: Arc<PropertyType>,
    nav_obj: &JsonObj,
    alias: &str,
    params: &mut Vec<SqlParam>,
) -> Result<String, AppError> {
    let nav = prop
        .nav_by_fk_property
        .clone()
        .ok_or_else(|| MetadataError::MissingNavigation {
            entity_type: entity_type.pascal_1.clone(),
            property_name: prop.name.clone(),
        })?;
    let ref_entity_type = app_config.get_nav_entity_type(entity_type.clone(), prop.clone())?;
    let nav_alias = format!("{}_nav", prop.name);
    let inner_filter = create_filter(
        app_config.clone(),
        ref_entity_type.clone(),
        nav_obj.clone(),
        &nav_alias,
        params,
    )?;

    let join =
        if nav.type_name == entity_type.pascal_1 && nav.schema_name == entity_type.schema_name {
            let ref_pk_name = app_config.get_primary_key_name(ref_entity_type.clone())?;
            let ref_pk_prop = AppConfig::get_prop(ref_entity_type.clone(), &ref_pk_name)?;
            let local_fk_prop = AppConfig::get_prop(entity_type.clone(), &nav.prop_name)?;
            format!(
                "{} = {}",
                qualify(alias, &naming::property_column(&local_fk_prop)),
                qualify(&nav_alias, &naming::property_column(&ref_pk_prop)),
            )
        } else {
            let parent_pk_name = app_config.get_primary_key_name(entity_type.clone())?;
            let parent_pk_prop = AppConfig::get_prop(entity_type.clone(), &parent_pk_name)?;
            let ref_fk_prop = AppConfig::get_prop(ref_entity_type.clone(), &nav.prop_name)?;
            format!(
                "{} = {}",
                qualify(&nav_alias, &naming::property_column(&ref_fk_prop)),
                qualify(alias, &naming::property_column(&parent_pk_prop)),
            )
        };

    Ok(format!(
        "EXISTS (SELECT 1 FROM {} AS {} WHERE {} AND {})",
        table_ref(ref_entity_type.clone()),
        quote_ident(&nav_alias),
        join,
        inner_filter,
    ))
}

fn qualify(alias: &str, col: &str) -> String {
    if alias.is_empty() {
        format!("[{}]", col)
    } else {
        format!("[{}].[{}]", alias, col)
    }
}

fn table_ref(entity_type: Arc<EntityType>) -> String {
    naming::table_ref(&entity_type)
}

fn quote_ident(name: &str) -> String {
    format!("[{}]", name.replace(']', "]]"))
}

fn value_kind(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
