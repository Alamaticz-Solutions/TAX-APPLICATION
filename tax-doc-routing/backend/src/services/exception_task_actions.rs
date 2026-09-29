//! `retry_failed_step`: resolves an `ExceptionTask` and resumes the parent `TaxRoutingRecord`'s
//! processing (business spec 5.3). `exception_task.update` is granted at scope "own" to
//! TAX_STAFF, but a custom mutation has no Rego row filter of its own (same reasoning as
//! `routing_record_actions.rs`), so the "own" check is done here by hand against the caller's
//! own `app_user` id.

use std::sync::Arc;
use serde_json::json;

use crate::{
    product_api::{entity_type_for_handler, DataAccess, EntityType, HandlerResult, UserAuth},
    schemas::tax_routing::{
        ExceptionTaskProjection, InputExceptionTask, InputTaxRoutingRecord, RoutingRecordStatus,
        TaxRoutingRecordProjection,
    },
    services::authz,
};

fn field(name: &str) -> serde_json::Value {
    json!({ "name": name, "selection_set": [] })
}

fn exception_selection() -> serde_json::Value {
    json!({
        "name": "exception_task",
        "selection_set": [
            field("id"), field("routing_record_id"), field("failure_reason"), field("failed_step"),
            field("opened_at"), field("resolved_at"), field("assigned_user_id"), field("version"),
        ]
    })
}

fn record_selection() -> serde_json::Value {
    json!({
        "name": "tax_routing_record",
        "selection_set": [
            field("id"), field("status"), field("created_at"), field("updated_at"),
            field("assigned_user_id"), field("client_first_name"), field("client_last_name"),
            field("client_full_name"), field("office_location"), field("pds_email"),
            field("personal_email"), field("additional_email"), field("notification_flag"),
            field("internal_folder"), field("folder_name"), field("read_write_password"),
            field("client_reference_id"), field("progress_step"), field("version"),
        ]
    })
}

fn require_field<T>(value: Option<T>, field_name: &str) -> HandlerResult<T> {
    value.ok_or_else(|| anyhow::anyhow!("record is missing required field {field_name}"))
}

pub async fn retry_failed_step(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    exception_entity_type: &Arc<EntityType>,
    exception_task_id: String,
) -> HandlerResult<serde_json::Value> {
    let user = user.ok_or_else(|| anyhow::anyhow!("not authenticated"))?;
    let principal = data_access.app_config.authz().resolve(&user.tenant_id, &user.user_name);
    authz::require(&principal, "exception_task.update")?;

    let exception = data_access
        .find_item::<ExceptionTaskProjection>(
            exception_entity_type.clone(),
            exception_selection(),
            exception_task_id.clone(),
            Some(user.clone()),
        )
        .await?
        .ok_or_else(|| anyhow::anyhow!("exception task {exception_task_id} was not found"))?;

    // "own"-scope enforcement: a custom mutation bypasses the Rego row filter, so this mirrors
    // what the filter would have done for a plain update (docs/architecture/rbac-design.md §6).
    let scope_is_own = principal
        .grants
        .iter()
        .any(|g| g.code == "exception_task.update" && g.scope == "own");
    if scope_is_own && exception.assigned_user_id.as_deref() != principal.user_id.as_deref() {
        return Err(anyhow::anyhow!("this exception is not assigned to you").into());
    }

    if exception.resolved_at.is_some() {
        return Err(anyhow::anyhow!("this exception was already resolved").into());
    }

    let resolved_input = InputExceptionTask {
        id: exception.id.clone(),
        routing_record_id: require_field(exception.routing_record_id.clone(), "routing_record_id")?,
        failure_reason: require_field(exception.failure_reason.clone(), "failure_reason")?,
        failed_step: require_field(exception.failed_step.clone(), "failed_step")?,
        opened_at: require_field(exception.opened_at, "opened_at")?,
        resolved_at: Some(chrono::Utc::now()),
        assigned_user_id: exception.assigned_user_id.clone(),
        version: exception.version,
    };
    data_access
        .update_item::<InputExceptionTask, ExceptionTaskProjection>(
            exception_entity_type.clone(),
            exception_selection(),
            resolved_input,
            Some(user.clone()),
        )
        .await?;

    let record_entity_type = entity_type_for_handler(data_access, "tax_routing", "TaxRoutingRecord")?;
    let record_id = require_field(exception.routing_record_id.clone(), "routing_record_id")?;
    let record = data_access
        .find_item::<TaxRoutingRecordProjection>(
            record_entity_type.clone(),
            record_selection(),
            record_id.clone(),
            Some(user.clone()),
        )
        .await?
        .ok_or_else(|| anyhow::anyhow!("routing record {record_id} was not found"))?;

    let record_input = InputTaxRoutingRecord {
        id: record.id.clone(),
        status: RoutingRecordStatus::processing,
        created_at: require_field(record.created_at, "created_at")?,
        updated_at: chrono::Utc::now(),
        assigned_user_id: require_field(record.assigned_user_id.clone(), "assigned_user_id")?,
        client_first_name: record.client_first_name.clone(),
        client_last_name: record.client_last_name.clone(),
        client_full_name: record.client_full_name.clone(),
        office_location: record.office_location.clone(),
        pds_email: record.pds_email.clone(),
        personal_email: record.personal_email.clone(),
        additional_email: record.additional_email.clone(),
        notification_flag: record.notification_flag.unwrap_or(false),
        internal_folder: record.internal_folder.clone(),
        folder_name: record.folder_name.clone(),
        read_write_password: require_field(record.read_write_password.clone(), "read_write_password")?,
        client_reference_id: require_field(record.client_reference_id.clone(), "client_reference_id")?,
        progress_step: record.progress_step.unwrap_or(0),
        version: record.version,
    };
    let updated = data_access
        .update_item::<InputTaxRoutingRecord, TaxRoutingRecordProjection>(
            record_entity_type,
            record_selection(),
            record_input,
            Some(user),
        )
        .await?;

    tracing::info!(
        exception_task_id = %exception_task_id,
        record_id = %record_id,
        "exception retried; record resumed processing"
    );

    Ok(json!({
        "exception_task_id": exception_task_id,
        "routing_record_id": updated.id,
        "status": updated.status,
    }))
}
