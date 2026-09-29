use appfw_provider_snowflake::{order_by as provider_order_by, SnowflakeSortField};
use appfw_runtime::query_ir::parse_sort_specs;
use std::sync::Arc;

use serde_json::Value;

use crate::{
    config::app_config::AppConfig, data::query_ir::SortAst, routes::app_error::AppError,
    schemas::system::EntityType,
};

pub fn create_if_exists(
    entity_type: Arc<EntityType>,
    alias: &str,
    sort: Option<Value>,
) -> Result<String, AppError> {
    let fields = sort_fields(entity_type.clone(), sort)?;
    Ok(provider_order_by(
        alias,
        &primary_key_name(&entity_type),
        &fields,
    ))
}

pub fn create_if_exists_ast(
    entity_type: Arc<EntityType>,
    alias: &str,
    sort: &SortAst,
) -> Result<String, AppError> {
    create_if_exists(entity_type, alias, sort.to_sort_value())
}

fn primary_key_name(entity_type: &EntityType) -> String {
    entity_type
        .props
        .iter()
        .find(|prop| prop.is_key)
        .map(|prop| prop.name.clone())
        .unwrap_or_else(|| "id".to_string())
}

fn sort_fields(
    entity_type: Arc<EntityType>,
    sort: Option<Value>,
) -> Result<Vec<SnowflakeSortField>, AppError> {
    parse_sort_specs(sort)
        .map_err(AppError::from)?
        .into_iter()
        .map(|spec| {
            let prop = AppConfig::get_prop(entity_type.clone(), &spec.field)?;
            Ok(SnowflakeSortField {
                name: prop.name.clone(),
                direction: spec.direction,
            })
        })
        .collect()
}
