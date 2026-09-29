use appfw_provider_mssql::{order_by as provider_order_by, MssqlSortField};
use appfw_runtime::query_ir::parse_sort_specs;
use std::sync::Arc;

use serde_json::Value;

use crate::{
    config::app_config::AppConfig,
    data::{clients::mssql::naming, query_ir::SortAst},
    routes::app_error::AppError,
    schemas::system::{DataType, EntityType},
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

#[allow(dead_code)]
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
        .or_else(|| {
            entity_type.props.iter().find(|prop| {
                !matches!(
                    prop.data_type,
                    DataType::NavToOne | DataType::NavToMany | DataType::ManyToMany
                )
            })
        })
        .map(naming::property_column)
        .unwrap_or_else(|| "id".to_string())
}

fn sort_fields(
    entity_type: Arc<EntityType>,
    sort: Option<Value>,
) -> Result<Vec<MssqlSortField>, AppError> {
    parse_sort_specs(sort)
        .map_err(AppError::from)?
        .into_iter()
        .map(|spec| {
            let prop = AppConfig::get_prop(entity_type.clone(), &spec.field)?;
            Ok(MssqlSortField {
                name: naming::property_column(&prop),
                direction: spec.direction,
            })
        })
        .collect()
}
