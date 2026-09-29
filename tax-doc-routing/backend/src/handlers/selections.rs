use appfw_runtime::identifier::to_snake_case_lenient as to_snake_case;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::{
    config::app_config::AppConfig,
    routes::app_error::{AppError, MetadataError},
    schemas::system::{DataType, EntityType},
};

// Purpose: translate async_graphql selection set into json

#[allow(unused)]
pub fn get_query_selections(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    parent: async_graphql::SelectionField,
) -> Result<Value, AppError> {
    // println!("\n > get_query_selections parent {:?}", parent);
    let parent = match parent.selection_set().find(|p| p.name().eq("items")) {
        Some(items) => items,
        None => parent,
    };

    get_entity_selections(app_config.clone(), entity_type.clone(), parent)
}

pub fn get_entity_selections(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    parent: async_graphql::SelectionField,
) -> Result<Value, AppError> {
    // println!("\n > get_entity_selections entity_type {:?} parent {:?}", entity_type_name, parent);
    let mut selections: Vec<Value> = Vec::new();

    for child in parent.selection_set() {
        // println!("\n > db get_entity_selections child :: {:?}", &child.name().to_string());
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

                    // TODO: detect infinite recursion here!
                    let child_selections =
                        get_entity_selections(app_config.clone(), ref_entity_type.clone(), child)?;

                    //  Ensure FK field is selected
                    if nav.type_name == entity_type.pascal_1
                        && nav.schema_name == entity_type.schema_name
                    {
                        // If fk property is on current entity (current level)...
                        let fk_selected =
                            parent.selection_set().any(|p| p.name().eq(&nav.prop_name));
                        if !fk_selected {
                            selections.push(json!({"name": &nav.prop_name, "selection_set": [] }));
                        }
                        selections.push(child_selections);
                    } else {
                        // If fk property is on a another entity (child level)...
                        let mut actual_selections = child_selections.clone();
                        let sel_set = child_selections
                            .get("selection_set")
                            .and_then(|v| v.as_array())
                            .ok_or_else(|| {
                                MetadataError::InvalidSelection(
                                    "selection_set must be an array".to_string(),
                                )
                            })?
                            .to_owned();
                        let fk_selected = sel_set.iter().any(|p| {
                            p.get("name")
                                .and_then(|v| v.as_str())
                                .map(|name| name.eq(&nav.prop_name))
                                .unwrap_or(false)
                        });
                        if !fk_selected {
                            let mut x = sel_set;
                            x.push(json!({ "name": &nav.prop_name, "selection_set": [] }));
                            actual_selections = json!({"name": prop.name, "selection_set": x });
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

                    // Circular reference detection implemented in CTE generation
                    let child_selections =
                        get_entity_selections(app_config.clone(), ref_entity_type.clone(), child)?;

                    selections.push(child_selections);
                }
            }

            _ => {
                selections
                    .push(json!({ "name": to_snake_case(child.name()), "selection_set": [] }));
            }
        }
    }

    let result = json!({ "name": parent.name(), "selection_set": selections });
    Ok(result)
}
