use appfw_runtime::identifier::to_snake_case_lenient as to_snake_case;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::{
    config::app_config::AppConfig,
    routes::app_error::{AppError, MetadataError},
    schemas::system::{DataType, EntityType},
};

#[allow(unused)]
pub fn get_query_selections(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    parent: async_graphql::SelectionField,
) -> Result<Value, AppError> {
    let parent = match parent.selection_set().find(|field| field.name().eq("items")) {
        Some(items) => items,
        None => parent,
    };

    get_entity_selections(app_config, entity_type, parent)
}

pub fn get_entity_selections(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    parent: async_graphql::SelectionField,
) -> Result<Value, AppError> {
    let mut selections: Vec<Value> = Vec::new();

    for child in parent.selection_set() {
        let prop = AppConfig::get_prop(entity_type.clone(), &child.name().to_string())?;

        match prop.data_type {
            DataType::NavToOne | DataType::NavToMany => {
                if child.selection_set().count() > 0 {
                    let nav = prop.nav_by_fk_property.clone().ok_or_else(|| {
                        MetadataError::MissingNavigation {
                            entity_type: entity_type.pascal_1.clone(),
                            property_name: prop.name.clone(),
                        }
                    })?;
                    let ref_entity_type =
                        app_config.get_nav_entity_type(entity_type.clone(), prop.clone())?;
                    let child_selections =
                        get_entity_selections(app_config.clone(), ref_entity_type, child)?;

                    if nav.type_name == entity_type.pascal_1
                        && nav.schema_name == entity_type.schema_name
                    {
                        let fk_selected =
                            parent.selection_set().any(|field| field.name().eq(&nav.prop_name));
                        if !fk_selected {
                            selections.push(json!({"name": &nav.prop_name, "selection_set": [] }));
                        }
                        selections.push(child_selections);
                    } else {
                        let mut actual_selections = child_selections.clone();
                        let selection_set = child_selections
                            .get("selection_set")
                            .and_then(|value| value.as_array())
                            .ok_or_else(|| {
                                MetadataError::InvalidSelection(
                                    "selection_set must be an array".to_string(),
                                )
                            })?
                            .to_owned();
                        let fk_selected = selection_set.iter().any(|selection| {
                            selection
                                .get("name")
                                .and_then(|value| value.as_str())
                                .map(|name| name.eq(&nav.prop_name))
                                .unwrap_or(false)
                        });
                        if !fk_selected {
                            let mut selections_with_fk = selection_set;
                            selections_with_fk.push(json!({ "name": &nav.prop_name, "selection_set": [] }));
                            actual_selections = json!({"name": prop.name, "selection_set": selections_with_fk });
                        }
                        selections.push(actual_selections);
                    }
                }
            }
            DataType::ManyToMany => {
                if child.selection_set().count() > 0 {
                    let many_to_many = prop.many_to_many_property.clone().ok_or_else(|| {
                        MetadataError::MissingManyToMany {
                            entity_type: entity_type.pascal_1.clone(),
                            property_name: prop.name.clone(),
                        }
                    })?;
                    let ref_entity_type = app_config.get_entity_type_by_name(
                        &many_to_many.target_schema,
                        &many_to_many.target_type,
                    )?;
                    let child_selections =
                        get_entity_selections(app_config.clone(), ref_entity_type, child)?;
                    selections.push(child_selections);
                }
            }
            _ => selections.push(json!({ "name": to_snake_case(child.name()), "selection_set": [] })),
        }
    }

    Ok(json!({ "name": parent.name(), "selection_set": selections }))
}
