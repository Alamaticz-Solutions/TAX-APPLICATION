use appfw_provider_mongo::MongoRecordProperty;
use bson::{doc, Bson, Document};
use serde_json::{Map, Value};
use std::sync::Arc;

use crate::config::app_config::AppConfig;
use crate::product_api::runtime_data_type;
use crate::routes::app_error::{AppError, MetadataError};
use crate::schemas::system::DataType;
use crate::schemas::system::{EntityType, PropertyType};
use tracing::debug;

type JsonObj = Map<String, serde_json::Value>;

pub const TYPE_PK_NAME: &str = "id";

// TODO: pass into this fn an alias to qualify the fields
pub fn to_bson(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    input: JsonObj,
) -> Result<Document, AppError> {
    let mut record = doc! {};

    for v in input {
        let prop_name = &v.0;
        let prop_value = &v.1;

        debug!(property = %prop_name, "converting property to BSON");

        let prop = AppConfig::try_get_prop(entity_type.clone(), prop_name).ok_or_else(|| {
            MetadataError::MissingProperty {
                entity_type: entity_type.pascal_1.clone(),
                property_name: prop_name.clone(),
            }
        })?;

        let value = get_value(
            app_config.clone(),
            entity_type.clone(),
            prop.clone(),
            prop_value,
        )?;

        debug!(property = %prop_name, "converted property to BSON");

        let bson_prop_name = appfw_provider_mongo::record_field_name(&record_property(&prop));

        record.insert(bson_prop_name, value);
    }

    Ok(record)
}

fn get_value(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    prop: Arc<PropertyType>,
    value: &Value,
) -> Result<Bson, AppError> {
    if !value.is_null() {
        match prop.data_type {
            DataType::Object => {
                return get_object_value(
                    app_config.clone(),
                    entity_type.clone(),
                    prop.clone(),
                    value,
                )
            }
            DataType::ObjectArray => {
                return get_object_array_value(
                    app_config.clone(),
                    entity_type.clone(),
                    prop.clone(),
                    value,
                )
            }
            _ => {}
        }
    }

    appfw_provider_mongo::value_to_bson(&record_property(&prop), value).map_err(AppError::from)
}

fn get_object_value(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    prop: Arc<PropertyType>,
    value: &Value,
) -> Result<Bson, AppError> {
    match value {
        Value::Object(obj_value) => {
            let nested_entity_type = app_config.get_nav_entity_type(entity_type.clone(), prop)?;
            let res = to_bson(
                app_config.clone(),
                nested_entity_type.clone(),
                obj_value.clone(),
            )?;
            Ok(res.into())
        }
        _ => {
            return Err(AppError::Validation(
                format!("to_bson::Invalid input value for property '{}'", &prop.name).into(),
            ));
        }
    }
}

fn get_object_array_value(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    prop: Arc<PropertyType>,
    value: &Value,
) -> Result<Bson, AppError> {
    match value {
        Value::Array(value_vec) => {
            let nested_entity_type =
                app_config.get_nav_entity_type(entity_type.clone(), prop.clone())?;
            let res: Vec<Bson> = value_vec
                .iter() // Use `iter()` instead of `into_iter()` for `&Vec<Value>`
                .map(|v| match v {
                    Value::Object(obj) => {
                        to_bson(app_config.clone(), nested_entity_type.clone(), obj.clone())
                            .map(|doc| doc.into())
                    }
                    _ => {
                        return Err(AppError::Validation(
                            format!("to_bson::Invalid input value for property '{}'", &prop.name)
                                .into(),
                        ));
                    }
                })
                .collect::<Result<Vec<Bson>, AppError>>()?;
            Ok(res.into())
        }
        _ => {
            return Err(AppError::Validation(
                format!("to_bson::Invalid input value for property '{}'", &prop.name).into(),
            ));
        }
    }
}

fn record_property(prop: &PropertyType) -> MongoRecordProperty {
    MongoRecordProperty::new(
        prop.name.clone(),
        runtime_data_type(prop.data_type),
        prop.is_required,
        prop.is_key,
    )
}
