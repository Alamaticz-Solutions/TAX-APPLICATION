use bson::Document;
use serde_json::Value;
use std::sync::Arc;

use crate::config::app_config::AppConfig;
use crate::data::query_ir::SortAst;
use crate::routes::app_error::AppError;
use crate::schemas::system::EntityType;

pub fn create_if_exists(
    entity_type: Arc<EntityType>,
    sort: Option<Value>,
) -> Result<Document, AppError> {
    let sort_obj = match appfw_provider_mongo::normalize_sort(sort).map_err(AppError::from)? {
        Some(o) => o,
        None => return Ok(Document::new()),
    };

    let mut entries = Vec::new();
    for item in sort_obj {
        // TODO: handle sorting on nested/referenced entity type properties
        let prop = AppConfig::get_prop(entity_type.clone(), &item.0)?;
        entries.push((prop.name.clone(), item.1));
    }

    Ok(appfw_provider_mongo::create_sort_document(entries))
}

pub fn create_if_exists_ast(
    entity_type: Arc<EntityType>,
    sort: &SortAst,
) -> Result<Document, AppError> {
    create_if_exists(entity_type, sort.to_sort_value())
}
