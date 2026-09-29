use appfw_runtime::{
    extension::UserAuth,
    record_computed::{self as runtime_computed, RuntimeComputedKind},
    record_timezone as runtime_timezone, record_validation as runtime_validation,
    record_version as runtime_version, AccessAction,
};
use serde_json::{Map, Value};
use std::sync::Arc;

use crate::product_api::{
    runtime_entity_metadata, runtime_property_metadata, RuntimePropertyMetadata,
};
use crate::routes::app_error::AppError;
use crate::schemas::system::{Computed, EntityType, PropertyType};

pub fn evaluate(
    entity_type: Arc<EntityType>,
    mut record: Map<String, Value>,
    action: AccessAction,
    user: &UserAuth,
) -> Result<Map<String, Value>, AppError> {
    // println!("\n rules::evaluate: record {:?}", record);

    for p in &entity_type.props {
        let prop = Arc::new(p.to_owned());

        if action == AccessAction::Create || action == AccessAction::Update {
            if prop.computed != Computed::None {
                // If a compute configuration is defined, this will evaluate it and apply to the result record.
                if let Some(computed_value) = try_compute_property(prop.clone(), &record)? {
                    // println!("\n compute_properties: prop:: {:?} {:?}", &prop.name, res);
                    record.insert(prop.name.to_string(), computed_value);
                }
            }

            // println!("\n compute_properties: prop:: {:?} {:?}", &prop.name, res);
            if prop.is_concurrency_control {
                // If a record version (concurrency control) property is defined, this will apply a new value to the result record.
                if let Some(new_version) = new_record_version(prop.clone(), &record)? {
                    // println!("\n compute_properties: prop:: {:?} {:?}", &prop.name, res);
                    record.insert(prop.name.to_string(), new_version);
                }
            }
        }

        if action != AccessAction::Delete {
            // If a DateTime property is defined, this will adjust timezone and apply to the result record.
            if let Some(adjusted_dt) = try_adjust_timezone(prop.clone(), &record, action, user)? {
                // println!("\n compute_properties: prop:: {:?} {:?}", &prop.name, res);
                record.insert(prop.name.to_string(), adjusted_dt);
            }
        }
    }

    runtime_validation::validate_record(&runtime_entity_metadata(&entity_type), &record, action)
        .map_err(AppError::from)?;

    Ok(record.to_owned())
}

pub fn get_record_version(
    entity_type: Arc<EntityType>,
    input: &Map<String, Value>,
) -> Result<Option<Value>, AppError> {
    let entity = runtime_entity_metadata(&entity_type);
    runtime_version::get_record_version(&entity, input).map_err(AppError::from)
}

fn try_compute_property(
    prop: Arc<PropertyType>,
    record: &Map<String, Value>,
) -> Result<Option<Value>, AppError> {
    runtime_computed::try_compute_property(
        runtime_computed_kind(prop.computed),
        &prop.name,
        prop.meta.as_ref(),
        record,
    )
    .map_err(AppError::from)
}

fn runtime_computed_kind(computed: Computed) -> RuntimeComputedKind {
    match computed {
        Computed::Concatenate => RuntimeComputedKind::Concatenate,
        Computed::Format => RuntimeComputedKind::Format,
        Computed::Word => RuntimeComputedKind::Word,
        Computed::Inflection => RuntimeComputedKind::Inflection,
        Computed::DateTimeNow => RuntimeComputedKind::DateTimeNow,
        Computed::None => RuntimeComputedKind::None,
    }
}

fn new_record_version(
    prop: Arc<PropertyType>,
    record: &Map<String, Value>,
) -> Result<Option<Value>, AppError> {
    let prop = runtime_property_metadata(&prop);
    runtime_version::new_record_version(&prop, record).map_err(AppError::from)
}

fn try_adjust_timezone(
    prop: Arc<PropertyType>,
    record: &Map<String, Value>,
    action: AccessAction,
    user: &UserAuth,
) -> Result<Option<Value>, AppError> {
    let prop: RuntimePropertyMetadata = runtime_property_metadata(&prop);
    runtime_timezone::try_adjust_timezone(&prop, record, action, &user.timezone)
        .map_err(AppError::from)
}
