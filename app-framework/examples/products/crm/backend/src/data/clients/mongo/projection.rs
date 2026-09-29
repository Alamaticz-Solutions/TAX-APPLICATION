#![allow(dead_code)]

use bson::Document;
use std::sync::Arc;

use appfw_provider_mongo::MongoProjectionField;

use crate::config::app_config::AppConfig;
use crate::routes::app_error::{AppError, MetadataError};
use crate::schemas::system::{DataType, EntityType};
use tracing::debug;

pub fn create_projection(parent: &serde_json::Value) -> Result<Document, AppError> {
    debug!("creating MongoDB projection");
    appfw_provider_mongo::create_projection(parent).map_err(AppError::from)
}

pub fn create_projection_for_entity(
    entity_type: Arc<EntityType>,
    parent: &serde_json::Value,
) -> Result<Document, AppError> {
    debug!("creating MongoDB typed projection");
    let selection_names = appfw_provider_mongo::selection_names(parent).map_err(AppError::from)?;

    let mut fields = Vec::new();
    for orig in selection_names {
        let prop = AppConfig::try_get_prop(entity_type.clone(), &orig).ok_or_else(|| {
            MetadataError::MissingProperty {
                entity_type: entity_type.pascal_1.clone(),
                property_name: orig,
            }
        })?;
        fields.push(MongoProjectionField::new(
            prop.name.clone(),
            prop.is_key
                && prop.name == appfw_provider_mongo::TYPE_PK_NAME
                && prop.data_type == DataType::ObjectId,
        ));
    }

    Ok(appfw_provider_mongo::create_projection_for_fields(fields))
}
