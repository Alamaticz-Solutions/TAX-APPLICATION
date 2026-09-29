// Selection-aware query builder for MS SQL Server.
//
// PostgreSQL builds recursive CTEs and aggregates relationship rows with
// `json_agg`. SQL Server can produce the same response shape with correlated
// subqueries and `FOR JSON PATH`. This module returns one query that yields:
//   - `count`: the total matching root rows, independent of pagination
//   - `rows`: a JSON array containing the requested page and nested selections

use std::sync::Arc;

use appfw_runtime::extension::UserAuth;
use serde_json::Value;

use appfw_runtime::{AccessAction, PolicyAccess};

use crate::{
    config::app_config::AppConfig,
    data::clients::mssql::{filter, naming, param, sort},
    routes::app_error::{AppError, MetadataError},
    schemas::system::{DataType, EntityType, PropertyType},
};

use super::param::SqlParam;

enum WherePart {
    Filter(Option<Value>),
    Raw(String),
}

pub fn build_json_query(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    selections: Value,
    filter_val: Option<Value>,
    sort_val: Option<Value>,
    skip: i32,
    limit: i32,
    user: &UserAuth,
    access: &PolicyAccess,
    params: &mut Vec<SqlParam>,
) -> Result<String, AppError> {
    let root_alias = "t0".to_string();
    let mut alias_counter = 1usize;

    let select_list = build_select_list(
        app_config.clone(),
        entity_type.clone(),
        selections,
        &root_alias,
        user,
        params,
        &mut alias_counter,
    )?;

    let select_list = if select_list.is_empty() {
        let pk_name = app_config.get_primary_key_name(entity_type.clone())?;
        let pk_prop = AppConfig::get_prop(entity_type.clone(), &pk_name)?;
        vec![format!(
            "{}.{} AS {}",
            quote_alias(&root_alias),
            quote_ident(&naming::property_column(&pk_prop)),
            quote_ident(&pk_name)
        )]
    } else {
        select_list
    };

    let where_sql = create_where_clause(
        app_config.clone(),
        entity_type.clone(),
        &root_alias,
        vec![
            WherePart::Filter(filter_val),
            WherePart::Filter(access.filter.clone()),
        ],
        params,
    )?;
    let order_by = sort::create_if_exists(entity_type.clone(), &root_alias, sort_val)?;

    let skip_i64 = skip.max(0) as i64;
    let limit_i64 = limit.max(0) as i64;
    params.push(SqlParam::I64(Some(skip_i64)));
    let skip_ph = param::param_placeholder(params.len());
    params.push(SqlParam::I64(Some(limit_i64)));
    let limit_ph = param::param_placeholder(params.len());

    Ok(format!(
        "SELECT \
       (SELECT COUNT_BIG(*) FROM {table_ref} AS {root_alias} {where_sql}) AS [count], \
       JSON_QUERY(( \
         SELECT {select_list} \
         FROM {table_ref} AS {root_alias} \
         {where_sql} \
         {order_by} \
         OFFSET {skip_ph} ROWS FETCH NEXT {limit_ph} ROWS ONLY \
         FOR JSON PATH \
       )) AS [rows];",
        table_ref = table_ref(entity_type.clone()),
        root_alias = quote_alias(&root_alias),
        where_sql = where_sql,
        select_list = select_list.join(", "),
        order_by = order_by,
        skip_ph = skip_ph,
        limit_ph = limit_ph,
    ))
}

fn build_select_list(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    selection_parent: Value,
    alias: &str,
    user: &UserAuth,
    params: &mut Vec<SqlParam>,
    alias_counter: &mut usize,
) -> Result<Vec<String>, AppError> {
    let selection_set = selection_parent
        .get("selection_set")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            MetadataError::InvalidSelection("selection_set must be an array".to_string())
        })?;

    let mut expressions = Vec::new();
    for selection in selection_set {
        let sel_name = selection
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                MetadataError::InvalidSelection("selection name must be a string".to_string())
            })?
            .to_string();
        let prop = AppConfig::get_prop(entity_type.clone(), &sel_name)?;

        match prop.data_type {
            DataType::NavToOne | DataType::NavToMany => {
                expressions.push(build_nav_projection(
                    app_config.clone(),
                    entity_type.clone(),
                    prop.clone(),
                    selection.clone(),
                    alias,
                    user,
                    params,
                    alias_counter,
                )?);
            }
            DataType::ManyToMany => {
                expressions.push(build_many_to_many_projection(
                    app_config.clone(),
                    entity_type.clone(),
                    prop.clone(),
                    selection.clone(),
                    alias,
                    user,
                    params,
                    alias_counter,
                )?);
            }
            _ => expressions.push(native_projection(prop.clone(), alias)),
        }
    }

    Ok(expressions)
}

fn build_nav_projection(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    prop: Arc<PropertyType>,
    selection: Value,
    parent_alias: &str,
    user: &UserAuth,
    params: &mut Vec<SqlParam>,
    alias_counter: &mut usize,
) -> Result<String, AppError> {
    let nav = prop
        .nav_by_fk_property
        .clone()
        .ok_or_else(|| MetadataError::MissingNavigation {
            entity_type: entity_type.pascal_1.clone(),
            property_name: prop.name.clone(),
        })?;
    let child_entity_type = app_config.get_nav_entity_type(entity_type.clone(), prop.clone())?;
    let child_access =
        app_config.evaluate_user_access(child_entity_type.clone(), AccessAction::Read, user)?;
    if !child_access.allow {
        return Err(AppError::AccessDenied);
    }

    let child_alias = next_alias(alias_counter);
    let child_pk_name = app_config.get_primary_key_name(child_entity_type.clone())?;
    let curr_pk_name = app_config.get_primary_key_name(entity_type.clone())?;
    let child_pk_prop = AppConfig::get_prop(child_entity_type.clone(), &child_pk_name)?;
    let curr_pk_prop = AppConfig::get_prop(entity_type.clone(), &curr_pk_name)?;

    let child_select_list = build_select_list(
        app_config.clone(),
        child_entity_type.clone(),
        selection,
        &child_alias,
        user,
        params,
        alias_counter,
    )?;
    let child_select_list = ensure_select_list(child_select_list, &child_alias, &child_pk_prop);

    let join_clause =
        if nav.type_name == entity_type.pascal_1 && nav.schema_name == entity_type.schema_name {
            let parent_fk_prop = AppConfig::get_prop(entity_type.clone(), &nav.prop_name)?;
            format!(
                "{}.{} = {}.{}",
                quote_alias(&child_alias),
                quote_ident(&naming::property_column(&child_pk_prop)),
                quote_alias(parent_alias),
                quote_ident(&naming::property_column(&parent_fk_prop)),
            )
        } else {
            let child_fk_prop = AppConfig::get_prop(child_entity_type.clone(), &nav.prop_name)?;
            format!(
                "{}.{} = {}.{}",
                quote_alias(&child_alias),
                quote_ident(&naming::property_column(&child_fk_prop)),
                quote_alias(parent_alias),
                quote_ident(&naming::property_column(&curr_pk_prop)),
            )
        };

    let where_sql = create_where_clause(
        app_config.clone(),
        child_entity_type.clone(),
        &child_alias,
        vec![
            WherePart::Raw(join_clause),
            WherePart::Filter(child_access.filter.clone()),
        ],
        params,
    )?;
    let order_by = format!(
        "ORDER BY {}.{} ASC",
        quote_alias(&child_alias),
        quote_ident(&naming::property_column(&child_pk_prop))
    );
    let top_clause = if prop.data_type == DataType::NavToOne {
        "TOP 1 "
    } else {
        ""
    };
    let without_array_wrapper = if prop.data_type == DataType::NavToOne {
        ", WITHOUT_ARRAY_WRAPPER"
    } else {
        ""
    };

    Ok(format!(
    "JSON_QUERY((SELECT {top_clause}{select_list} FROM {table_ref} AS {child_alias} {where_sql} {order_by} FOR JSON PATH{without_array_wrapper})) AS {prop_name}",
    top_clause = top_clause,
    select_list = child_select_list.join(", "),
    table_ref = table_ref(child_entity_type.clone()),
    child_alias = quote_alias(&child_alias),
    where_sql = where_sql,
    order_by = order_by,
    without_array_wrapper = without_array_wrapper,
    prop_name = quote_ident(&prop.name),
  ))
}

fn build_many_to_many_projection(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    prop: Arc<PropertyType>,
    selection: Value,
    parent_alias: &str,
    user: &UserAuth,
    params: &mut Vec<SqlParam>,
    alias_counter: &mut usize,
) -> Result<String, AppError> {
    let many_to_many =
        prop.many_to_many_property
            .clone()
            .ok_or_else(|| MetadataError::MissingManyToMany {
                entity_type: entity_type.pascal_1.clone(),
                property_name: prop.name.clone(),
            })?;
    let target_entity_type = app_config
        .get_entity_type_by_name(&many_to_many.target_schema, &many_to_many.target_type)?;
    let target_access =
        app_config.evaluate_user_access(target_entity_type.clone(), AccessAction::Read, user)?;
    if !target_access.allow {
        return Err(AppError::AccessDenied);
    }

    let junction_schema = many_to_many
        .junction_schema
        .as_ref()
        .unwrap_or(&entity_type.schema_name);
    let junction_table = naming::physical_table_name(&many_to_many.junction_table);
    let target_alias = next_alias(alias_counter);
    let junction_alias = next_alias(alias_counter);
    let curr_pk_name = app_config.get_primary_key_name(entity_type.clone())?;
    let target_pk_name = app_config.get_primary_key_name(target_entity_type.clone())?;
    let curr_pk_prop = AppConfig::get_prop(entity_type.clone(), &curr_pk_name)?;
    let target_pk_prop = AppConfig::get_prop(target_entity_type.clone(), &target_pk_name)?;

    let target_select_list = build_select_list(
        app_config.clone(),
        target_entity_type.clone(),
        selection,
        &target_alias,
        user,
        params,
        alias_counter,
    )?;
    let target_select_list = ensure_select_list(target_select_list, &target_alias, &target_pk_prop);

    let join_clause = format!(
        "{}.{} = {}.{}",
        quote_alias(&junction_alias),
        quote_ident(&many_to_many.local_key),
        quote_alias(parent_alias),
        quote_ident(&naming::property_column(&curr_pk_prop)),
    );
    let where_sql = create_where_clause(
        app_config.clone(),
        target_entity_type.clone(),
        &target_alias,
        vec![
            WherePart::Raw(join_clause),
            WherePart::Filter(target_access.filter.clone()),
        ],
        params,
    )?;
    let order_by = format!(
        "ORDER BY {}.{} ASC",
        quote_alias(&target_alias),
        quote_ident(&naming::property_column(&target_pk_prop))
    );

    Ok(format!(
    "JSON_QUERY((SELECT {select_list} FROM {junction_ref} AS {junction_alias} INNER JOIN {target_ref} AS {target_alias} ON {junction_target_join} {where_sql} {order_by} FOR JSON PATH)) AS {prop_name}",
    select_list = target_select_list.join(", "),
    junction_ref = format!("{}.{}", quote_ident(junction_schema), quote_ident(&junction_table)),
    junction_alias = quote_alias(&junction_alias),
    target_ref = table_ref(target_entity_type.clone()),
    target_alias = quote_alias(&target_alias),
    junction_target_join = format!(
      "{}.{} = {}.{}",
      quote_alias(&junction_alias),
      quote_ident(&many_to_many.foreign_key),
      quote_alias(&target_alias),
      quote_ident(&naming::property_column(&target_pk_prop)),
    ),
    where_sql = where_sql,
    order_by = order_by,
    prop_name = quote_ident(&prop.name),
  ))
}

fn create_where_clause(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    alias: &str,
    filters: Vec<WherePart>,
    params: &mut Vec<SqlParam>,
) -> Result<String, AppError> {
    let mut clauses: Vec<String> = Vec::new();

    for where_part in filters {
        match where_part {
            WherePart::Raw(raw_sql) => {
                clauses.push(raw_sql);
            }
            WherePart::Filter(filter_val) => {
                let maybe_filter = filter::try_create_filter(
                    app_config.clone(),
                    entity_type.clone(),
                    filter_val,
                    alias,
                    params,
                )?;
                if let Some(filter_sql) = maybe_filter {
                    if !filter_sql.trim().is_empty() && filter_sql.trim() != "1 = 1" {
                        clauses.push(filter_sql);
                    }
                }
            }
        }
    }

    Ok(if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    })
}

fn ensure_select_list(
    select_list: Vec<String>,
    alias: &str,
    pk_prop: &PropertyType,
) -> Vec<String> {
    if select_list.is_empty() {
        vec![format!(
            "{}.{} AS {}",
            quote_alias(alias),
            quote_ident(&naming::property_column(pk_prop)),
            quote_ident(&pk_prop.name)
        )]
    } else {
        select_list
    }
}

fn native_projection(prop: Arc<PropertyType>, alias: &str) -> String {
    let qualified = format!(
        "{}.{}",
        quote_alias(alias),
        quote_ident(&naming::property_column(&prop))
    );
    match prop.data_type {
        DataType::Object
        | DataType::ObjectArray
        | DataType::Json
        | DataType::JsonArray
        | DataType::StringArray
        | DataType::UuidArray
        | DataType::ObjectIdArray
        | DataType::Int8Array
        | DataType::Int16Array
        | DataType::Int32Array
        | DataType::Int64Array
        | DataType::EnumArray => json_projection(&qualified, &quote_ident(&prop.name)),
        DataType::Uuid => format!(
            "LOWER(CONVERT(varchar(36), {})) AS {}",
            qualified,
            quote_ident(&prop.name)
        ),
        _ => format!("{} AS {}", qualified, quote_ident(&prop.name)),
    }
}

fn json_projection(qualified: &str, output_alias: &str) -> String {
    format!(
        "JSON_QUERY(CASE WHEN {qualified} IS NULL THEN NULL WHEN ISJSON({qualified}) = 1 THEN {qualified} ELSE NULL END) AS {output_alias}"
    )
}

fn table_ref(entity_type: Arc<EntityType>) -> String {
    naming::table_ref(&entity_type)
}

fn next_alias(alias_counter: &mut usize) -> String {
    let alias = format!("t{}", *alias_counter);
    *alias_counter += 1;
    alias
}

fn quote_alias(alias: &str) -> String {
    quote_ident(alias)
}

fn quote_ident(name: &str) -> String {
    naming::quote_ident(name)
}
