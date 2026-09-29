use appfw_provider_postgres::{
    create_criterion as provider_create_criterion,
    relation_exists_from_source_fk as provider_relation_exists_from_source_fk,
    relation_exists_from_target_fk as provider_relation_exists_from_target_fk, PostgresFilterField,
    PostgresRelationExists, SqlParam,
};
use appfw_runtime::{
    identifier::to_snake_case_lenient as to_snake_case,
    query_filter::{
        conjunction_token as conjunction, filter_token, normalize_filter_input, RuntimeFilterObject,
    },
};
use serde_json::Value;
use std::sync::Arc;

use crate::config::app_config::AppConfig;
use crate::data::query_ir::FilterAst;
use crate::product_api::runtime_data_type;
use crate::routes::app_error::{AppError, MetadataError};
use crate::schemas::system::DataType;
use crate::schemas::system::{EntityType, PropertyType};
use tracing::instrument;

#[instrument(
  skip(app_config, entity_type, input, params),
  fields(entity = %entity_type.pascal_1, alias = %alias)
)]
pub fn try_create_filter(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    input: Option<Value>,
    alias: &String,
    params: &mut Vec<SqlParam>,
) -> Result<Option<String>, AppError> {
    let normalized = normalize_filter_input(input).map_err(AppError::from)?;

    match normalized {
        Some(fltr_obj) => Ok(Some(create_filter(
            app_config,
            entity_type,
            fltr_obj,
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
    alias: &String,
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

// TODO: pass into this fn an alias to qualify the fields
fn create_filter(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    input: RuntimeFilterObject,
    alias: &String,
    params: &mut Vec<SqlParam>,
) -> Result<String, AppError> {
    let mut criteria: Vec<String> = vec![];

    for v in input {
        let attr_name = &v.0;
        let attr_value = &v.1;

        // println!("\n > create_filter: attr_name {:?}", attr_name);
        // println!("\n > create_filter: attr_value {:?}", attr_value);

        match attr_name.as_str() {
            conjunction::AND | conjunction::OR => {
                // println!("\n > create_filter: conjunction {:?} {:?}", attr_name, attr_value);
                let conj = attr_name[1..].to_string();
                if let Some(conj_criteria) = create_conjunction(
                    app_config.clone(),
                    entity_type.clone(),
                    &conj,
                    attr_value,
                    alias,
                    params,
                )? {
                    criteria.push(conj_criteria);
                }
            }
            _ => {
                let prop_name = to_snake_case(attr_name);
                // println!("\n > create_filter: prop_name {:?}", prop_name);
                let prop = AppConfig::get_prop(entity_type.clone(), &prop_name).map_err(|_| {
                    AppError::Validation(format!(
                        "invalid filter field '{}': property not found on {}",
                        attr_name, entity_type.pascal_1
                    ))
                })?;

                if matches!(prop.data_type, DataType::NavToOne | DataType::NavToMany) {
                    let criterion = get_nav_criterion(
                        app_config.clone(),
                        entity_type.clone(),
                        prop.clone(),
                        attr_value,
                        alias,
                        params,
                    )?;

                    if let Some(criterion) = criterion {
                        criteria.push(criterion);
                    }
                    continue;
                }

                if matches!(prop.data_type, DataType::ManyToMany) {
                    return Err(AppError::Validation(format!(
                        "ManyToMany filtering not yet implemented for property '{}'",
                        &prop.name
                    )));
                }

                // println!("\n > create_filter: attr_value {:?}", attr_value);

                let (op, value) = match attr_value {
                    Value::Null => {
                        return Err(AppError::Validation(format!(
                            "filter value for '{}' must not be null",
                            attr_name
                        )))
                    }
                    Value::Bool(_) | Value::Number(_) | Value::String(_) | Value::Array(_) => {
                        (String::from(filter_token::EQUALS), attr_value)
                    }
                    Value::Object(o_val) => {
                        let element = o_val.into_iter().next().ok_or_else(|| {
                            AppError::Validation(format!(
                                "filter operator object for '{}' must not be empty",
                                attr_name
                            ))
                        })?;
                        (element.0.as_str().to_string(), element.1)
                    }
                };

                let criterion = provider_create_criterion(
                    &provider_filter_field(&prop),
                    &op,
                    value,
                    alias,
                    params,
                )
                .map_err(AppError::from)?;

                if let Some(criterion) = criterion {
                    criteria.push(criterion);
                }
            }
        };
    }

    let result = criteria.join(" and ");

    // println!("\n > create_filter: result {:?}", result);

    Ok(result)
}

fn provider_filter_field(prop: &PropertyType) -> PostgresFilterField {
    PostgresFilterField {
        name: prop.name.clone(),
        data_type: runtime_data_type(prop.data_type),
        is_required: prop.is_required,
    }
}

// GRAPHQL:
// {
//   _or: [
//     { name: { _eq: "ACME" } },
//     _and: [
//       { country: { _eq: "Canada" } },
//       { status: { _ne: "Active" } }
//     ]
//   ]
// }
//
// POSTGRES:
// name = 'ACME' OR (country = 'Canada' AND status <> 'Active')

fn create_conjunction(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    conj: &String,
    attr_value: &serde_json::Value,
    alias: &String,
    params: &mut Vec<SqlParam>,
) -> Result<Option<String>, AppError> {
    // println!("\n > create_conjunction: {:?} {:?}", conj, attr_value);
    let mut conj_criteria: Vec<String> = vec![];
    match attr_value {
        Value::Array(array_val) => {
            for item in array_val {
                match item {
                    Value::Object(item_obj) => {
                        let obj = item_obj.to_owned();
                        let criterion = create_filter(
                            app_config.clone(),
                            entity_type.clone(),
                            obj,
                            alias,
                            params,
                        )?;
                        if !criterion.trim().is_empty() {
                            conj_criteria.push(criterion);
                        }
                    }
                    _ => {
                        return Err(AppError::Validation(format!(
                            "filter conjunction '{}' expects object items",
                            conj
                        )))
                    }
                }
            }
        }
        _ => {
            return Err(AppError::Validation(format!(
                "filter conjunction '{}' expects an array",
                conj
            )))
        }
    };
    if conj_criteria.is_empty() {
        return Ok(None);
    }
    let sep = format!(" {} ", conj);
    Ok(Some(format!("({})", conj_criteria.join(&sep))))
}

fn get_nav_criterion(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    prop: Arc<PropertyType>,
    value: &Value,
    alias: &String,
    params: &mut Vec<SqlParam>,
) -> Result<Option<String>, AppError> {
    // println!("\n > get_nav_criterion: prop {:?}", prop.name);
    match value {
        Value::Object(val_obj) => {
            let nav = prop.nav_by_fk_property.clone().ok_or_else(|| {
                MetadataError::MissingNavigation {
                    entity_type: entity_type.pascal_1.clone(),
                    property_name: prop.name.clone(),
                }
            })?;
            let ref_entity_type =
                app_config.get_nav_entity_type(entity_type.clone(), prop.clone())?;
            let nav_alias = format!("{}_nav", prop.name);
            let inner_filter = create_filter(
                app_config.clone(),
                ref_entity_type.clone(),
                val_obj.to_owned(),
                &nav_alias,
                params,
            )?;
            let relation = PostgresRelationExists {
                schema: ref_entity_type.schema_name.clone(),
                table: ref_entity_type.snake_n.clone(),
                nav_alias: nav_alias.clone(),
                source_alias: alias.to_string(),
                source_key: String::new(),
                target_key: String::new(),
                inner_filter,
            };
            if nav.type_name == entity_type.pascal_1 && nav.schema_name == entity_type.schema_name {
                // Current entity_type (alias) has the foreign_key property
                // This means we're navigating FROM the entity that has the FK TO the referenced entity
                // Filter: current_table.fk_field = referenced_table.pk_field
                Ok(Some(provider_relation_exists_from_source_fk(
                    &PostgresRelationExists {
                        source_key: nav.prop_name.to_string(),
                        target_key: app_config.get_primary_key_name(ref_entity_type.clone())?,
                        ..relation
                    },
                )))
            } else {
                // Other entity_type has the foreign_key property to current entity_type (alias)
                Ok(Some(provider_relation_exists_from_target_fk(
                    &PostgresRelationExists {
                        source_key: app_config.get_primary_key_name(entity_type.clone())?,
                        target_key: nav.prop_name.to_string(),
                        ..relation
                    },
                )))
            }
        }
        _ => {
            return Err(AppError::Validation(
                format!("Invalid input value for property '{}'", &prop.name).into(),
            ));
        }
    }
}
