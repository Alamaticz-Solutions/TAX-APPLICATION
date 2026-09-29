use std::sync::Arc;

use appfw_provider_snowflake::{
    create_equality_clause as provider_create_equality_clause,
    create_operator_clause as provider_create_operator_clause, SnowflakeFilterField,
};
use appfw_runtime::{
    identifier::to_snake_case_lenient as to_snake_case,
    query_filter::conjunction_token as conjunction,
};
use serde_json::{Map, Value};

use crate::{
    config::app_config::AppConfig,
    data::{clients::snowflake::statement::SnowflakeStatementBuilder, query_ir::FilterAst},
    product_api::runtime_data_type,
    routes::app_error::{AppError, MetadataError},
    schemas::system::{DataType, EntityType, PropertyType},
};

type JsonObj = Map<String, Value>;

pub fn try_create_filter(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    filter_val: Option<Value>,
    alias: &str,
    builder: &mut SnowflakeStatementBuilder,
) -> Result<Option<String>, AppError> {
    let Some(filter_val) = filter_val else {
        return Ok(None);
    };
    match filter_val {
        Value::Null => Ok(None),
        Value::Object(obj) if obj.is_empty() => Ok(None),
        Value::Object(obj) => Ok(Some(create_filter(
            app_config,
            entity_type,
            obj,
            alias,
            builder,
        )?)),
        Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            let parsed: Value = serde_json::from_str(trimmed).map_err(|e| {
                AppError::Validation(format!("filter string is not valid JSON: {}", e))
            })?;
            try_create_filter(app_config, entity_type, Some(parsed), alias, builder)
        }
        other => Err(AppError::Validation(format!(
            "filter must be a JSON object or a JSON-encoded object string, got {}",
            value_kind(&other)
        ))),
    }
}

pub fn try_create_filter_ast(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    filter: Option<&FilterAst>,
    alias: &str,
    builder: &mut SnowflakeStatementBuilder,
) -> Result<Option<String>, AppError> {
    try_create_filter(
        app_config,
        entity_type,
        filter.and_then(FilterAst::to_filter_value),
        alias,
        builder,
    )
}

fn create_filter(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    obj: JsonObj,
    alias: &str,
    builder: &mut SnowflakeStatementBuilder,
) -> Result<String, AppError> {
    let mut clauses: Vec<String> = vec![];
    for (key, val) in obj {
        match key.as_str() {
            conjunction::AND | conjunction::OR => clauses.push(create_conjunction(
                app_config.clone(),
                entity_type.clone(),
                key.as_str(),
                val,
                alias,
                builder,
            )?),
            _ => {
                let prop = AppConfig::try_get_prop(entity_type.clone(), &key)
                    .or_else(|| {
                        let snake = to_snake_case(&key);
                        AppConfig::try_get_prop(entity_type.clone(), &snake)
                    })
                    .ok_or_else(|| {
                        AppError::Validation(format!(
                            "invalid filter field '{}': property not found on {}",
                            key, entity_type.pascal_1
                        ))
                    })?;

                let clause = match (&prop.data_type, &val) {
                    (DataType::NavToOne | DataType::NavToMany, Value::Object(nav_obj)) => {
                        create_nav_clause(
                            app_config.clone(),
                            entity_type.clone(),
                            prop.clone(),
                            nav_obj,
                            alias,
                            builder,
                        )?
                    }
                    (DataType::ManyToMany, _) => {
                        return Err(AppError::Validation(format!(
                            "ManyToMany filtering not yet implemented for property '{}'",
                            prop.name
                        )))
                    }
                    (_, Value::Object(op_obj)) => provider_create_operator_clause(
                        &provider_filter_field(&prop),
                        op_obj,
                        alias,
                        builder.inner_mut(),
                    )
                    .map_err(AppError::from)?,
                    _ => provider_create_equality_clause(
                        &provider_filter_field(&prop),
                        val.clone(),
                        alias,
                        builder.inner_mut(),
                    )
                    .map_err(AppError::from)?,
                };
                clauses.push(clause);
            }
        }
    }

    Ok(if clauses.is_empty() {
        "1 = 1".to_string()
    } else {
        clauses
            .into_iter()
            .map(|clause| format!("({})", clause))
            .collect::<Vec<_>>()
            .join(" AND ")
    })
}

fn create_conjunction(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    op: &str,
    val: Value,
    alias: &str,
    builder: &mut SnowflakeStatementBuilder,
) -> Result<String, AppError> {
    let arr = val.as_array().ok_or_else(|| {
        AppError::Validation(format!("{} expects an array of filter objects", op))
    })?;
    let joiner = if op == conjunction::OR {
        " OR "
    } else {
        " AND "
    };
    let mut parts = Vec::new();
    for item in arr {
        let obj = item.as_object().ok_or_else(|| {
            AppError::Validation(format!("{} array entries must be filter objects", op))
        })?;
        let part = create_filter(
            app_config.clone(),
            entity_type.clone(),
            obj.clone(),
            alias,
            builder,
        )?;
        if !part.trim().is_empty() && part.trim() != "1 = 1" {
            parts.push(format!("({})", part));
        }
    }
    if parts.is_empty() {
        return Ok("1 = 1".to_string());
    }
    Ok(parts.join(joiner))
}

fn provider_filter_field(prop: &PropertyType) -> SnowflakeFilterField {
    SnowflakeFilterField {
        name: prop.name.clone(),
        data_type: runtime_data_type(prop.data_type),
        is_required: prop.is_required,
    }
}

fn create_nav_clause(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    prop: Arc<PropertyType>,
    nav_obj: &JsonObj,
    alias: &str,
    builder: &mut SnowflakeStatementBuilder,
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
        builder,
    )?;

    let join =
        if nav.type_name == entity_type.pascal_1 && nav.schema_name == entity_type.schema_name {
            let ref_pk_name = app_config.get_primary_key_name(ref_entity_type.clone())?;
            format!(
                "{} = {}",
                qualify(alias, &nav.prop_name),
                qualify(&nav_alias, &ref_pk_name)
            )
        } else {
            let parent_pk_name = app_config.get_primary_key_name(entity_type.clone())?;
            format!(
                "{} = {}",
                qualify(&nav_alias, &nav.prop_name),
                qualify(alias, &parent_pk_name)
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
        quote_ident(col)
    } else {
        format!("{}.{}", quote_ident(alias), quote_ident(col))
    }
}

fn table_ref(entity_type: Arc<EntityType>) -> String {
    format!(
        "{}.{}",
        quote_ident(&entity_type.schema_name),
        quote_ident(&entity_type.snake_n)
    )
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
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
