use appfw_provider_mongo::{
    create_criterion as provider_create_criterion, filter_field_name, qualify_criterion_fields,
    value_kind, MongoFilterProperty,
};
use appfw_runtime::{
    identifier::to_snake_case_lenient as to_snake_case,
    query_filter::{conjunction_token as conjunction, filter_token as operand},
};
use bson::{doc, Bson, Document};
use serde_json::{Map, Value};
use std::sync::Arc;

use crate::config::app_config::AppConfig;
use crate::data::query_ir::FilterAst;
use crate::product_api::runtime_data_type;
use crate::routes::app_error::AppError;
use crate::schemas::system::{EntityType, PropertyType};
use tracing::instrument;

pub const OID_ATTR: &str = "$oid";
type JsonObj = Map<String, serde_json::Value>;

fn filter_property(prop: &PropertyType) -> MongoFilterProperty {
    MongoFilterProperty {
        name: prop.name.clone(),
        data_type: runtime_data_type(prop.data_type),
        is_key: prop.is_key,
    }
}

#[instrument(
  skip(app_config, entity_type, input),
  fields(entity = %entity_type.pascal_1)
)]
pub fn try_create_filter(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    input: Option<Value>,
) -> Result<Document, AppError> {
    // Accept the filter as either a JSON object or a JSON-encoded string that parses to an object.
    // Any other shape is a client mistake and must surface, not silently fall through to "no filter".
    let normalized: Option<JsonObj> = match input {
        None => None,
        Some(Value::Null) => None,
        Some(Value::Object(o)) => {
            if o.is_empty() {
                None
            } else {
                Some(o)
            }
        }
        Some(Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                match serde_json::from_str::<Value>(trimmed) {
                    Ok(Value::Object(o)) => {
                        if o.is_empty() {
                            None
                        } else {
                            Some(o)
                        }
                    }
                    Ok(other) => {
                        return Err(AppError::Validation(format!(
                            "filter must be a JSON object or a JSON-encoded object string, got {}",
                            value_kind(&other)
                        )))
                    }
                    Err(e) => {
                        return Err(AppError::Validation(format!(
                            "filter string is not valid JSON: {}",
                            e
                        )))
                    }
                }
            }
        }
        Some(other) => {
            return Err(AppError::Validation(format!(
                "filter must be a JSON object or a JSON-encoded object string, got {}",
                value_kind(&other)
            )))
        }
    };

    match normalized {
        Some(fltr_obj) => Ok(create_filter(app_config, entity_type, fltr_obj)?),
        None => Ok(doc! {}),
    }
}

pub fn try_create_filter_ast(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    input: Option<&FilterAst>,
) -> Result<Document, AppError> {
    match input {
        Some(filter) => create_filter_ast(app_config, entity_type, filter, None),
        None => Ok(doc! {}),
    }
}

fn create_filter_ast(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    input: &FilterAst,
    path_prefix: Option<&str>,
) -> Result<Document, AppError> {
    match input {
        FilterAst::All => Ok(doc! {}),
        FilterAst::And(items) | FilterAst::Or(items) => {
            let mut bson_items = Vec::new();
            for item in items {
                let item_doc =
                    create_filter_ast(app_config.clone(), entity_type.clone(), item, path_prefix)?;
                if !item_doc.is_empty() {
                    bson_items.push(Bson::Document(item_doc));
                }
            }
            if bson_items.is_empty() {
                Ok(doc! {})
            } else if bson_items.len() == 1 {
                Ok(bson_items
                    .remove(0)
                    .as_document()
                    .cloned()
                    .unwrap_or_default())
            } else {
                let conj = if matches!(input, FilterAst::And(_)) {
                    "$and"
                } else {
                    "$or"
                };
                Ok(doc! { conj: bson_items })
            }
        }
        FilterAst::Field(predicate) => {
            let op = predicate.op.as_filter_token().to_string();
            let filter_prop = filter_property(&predicate.prop);
            let storage_field_name = filter_field_name(&filter_prop);
            let criterion = get_criterion(
                app_config,
                entity_type,
                predicate.prop.clone(),
                &op,
                &predicate.value,
            )?;
            Ok(match path_prefix {
                Some(prefix) => qualify_criterion_fields(
                    criterion,
                    &[storage_field_name.clone(), predicate.prop.name.clone()],
                    &format!("{prefix}.{storage_field_name}"),
                ),
                None => criterion,
            })
        }
        FilterAst::Relation(relation) => {
            let prefix = path_prefix
                .map(|base| format!("{base}.{}", relation.prop.name))
                .unwrap_or_else(|| relation.prop.name.clone());
            create_filter_ast(
                app_config,
                relation.target_entity_type.clone(),
                &relation.filter,
                Some(&prefix),
            )
        }
    }
}

// Note: MongoDB doesn't require field aliases like SQL, so no alias parameter needed
fn create_filter(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    input: JsonObj,
) -> Result<Document, AppError> {
    let mut filter = doc! {};

    for v in input {
        let attr_name = &v.0;
        let attr_value = &v.1;

        match attr_name.as_str() {
            conjunction::AND | conjunction::OR => {
                let conj = format!("${}", attr_name[1..].to_string());
                let items = conjunction_vec(app_config.clone(), entity_type.clone(), attr_value)?;

                filter.insert(conj, items);
            }

            _ => {
                let prop_name = to_snake_case(attr_name);
                // println!("\n > create_filter: prop_name {:?}", prop_name);
                let prop =
                    AppConfig::try_get_prop(entity_type.clone(), &prop_name).ok_or_else(|| {
                        AppError::Validation(format!(
                            "invalid filter field '{}': property not found on {}",
                            attr_name, entity_type.pascal_1
                        ))
                    })?;

                let (op, value) = match attr_value {
                    Value::Null => {
                        return Err(AppError::Validation(format!(
                            "filter value for '{}' must not be null",
                            attr_name
                        )))
                    }
                    Value::Bool(_) | Value::Number(_) | Value::String(_) | Value::Array(_) => {
                        (String::from(operand::EQUALS), attr_value)
                    }
                    Value::Object(o_val) => {
                        let element = o_val.into_iter().next().ok_or_else(|| {
                            AppError::Validation(format!(
                                "filter operator object for '{}' must not be empty",
                                attr_name
                            ))
                        })?;
                        if element.0.as_str().eq(OID_ATTR) {
                            // { "id": Object {"$oid": String("6730f3c8acb1a69db7a14e73")} }
                            // $oid is a special attribute, unpack it to not confuse as an operator
                            (String::from(operand::EQUALS), element.1)
                        } else {
                            (element.0.as_str().to_string(), element.1)
                        }
                    }
                };

                let criterion =
                    get_criterion(app_config.clone(), entity_type.clone(), prop, &op, value)?;

                for (key, value) in criterion {
                    filter.insert(key, value);
                }
            }
        };
    }

    Ok(filter)
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

// "$and": [
//   { "$eq": [{ "$hour": "$timestamp" }, 13] },
//   { "$eq": [{ "$minute": "$timestamp" }, 45] }
// ]
fn conjunction_vec(
    app_config: Arc<AppConfig>,
    entity_type: Arc<EntityType>,
    attr_value: &serde_json::Value,
) -> Result<Vec<Bson>, AppError> {
    // println!("\n > create_conjunction: {:?} {:?}", conj, attr_value);
    let mut conjunction_items: Vec<Bson> = vec![];

    match attr_value {
        Value::Array(array_val) => {
            for item in array_val {
                match item {
                    Value::Object(item_obj) => {
                        let obj = item_obj.to_owned();
                        let criterion =
                            create_filter(app_config.clone(), entity_type.clone(), obj)?;
                        conjunction_items.push(criterion.into());
                    }
                    _ => {
                        return Err(AppError::Validation(
                            "filter conjunction expects object items".to_string(),
                        ))
                    }
                }
            }
        }
        _ => {
            return Err(AppError::Validation(
                format!("Error creating conjunction; unsupported conjunction value").into(),
            ));
        }
    };

    Ok(conjunction_items)
}

// GRAPHQL:
// { name: "ACME" }
// { name: { _eq: "ACME" } }
//
// POSTGRES:
// name = 'ACME'

// GRAPHQL:
// { name: "ACME", status: "Active" }
// { name: { _eq: "ACME" }, status: { _eq: "Active" } }
//
// POSTGRES:
// name = 'ACME' AND status = 'Active'

// GRAPHQL:
// { name: "ACME", status: { _eq: "ACTIVE" }, user: { name: { _eq: "admin" } } }
//
// POSTGRES: ...

fn get_criterion(
    _app_config: Arc<AppConfig>,
    _entity_type: Arc<EntityType>,
    prop: Arc<PropertyType>,
    op: &String,
    value: &Value,
) -> Result<Document, AppError> {
    provider_create_criterion(&filter_property(&prop), op, value).map_err(AppError::from)
}
