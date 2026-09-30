//! `cancel_record` and `reassign_record`: the two admin-only actions on `TaxRoutingRecord` that a
//! Rego row filter cannot express, because a filter restricts which rows a query returns, not
//! which columns an update may touch (business spec 9.1, BR-18). Both are gated by
//! `services::authz::require`, checking a permission code resolved from the database — never a
//! role literal. See docs/architecture/rbac-design.md section 6.

use std::sync::Arc;
use serde_json::json;

use crate::{
    product_api::{DataAccess, EntityType, HandlerResult, UserAuth},
    schemas::tax_routing::{InputTaxRoutingRecord, RoutingRecordStatus, TaxRoutingRecordProjection},
    services::authz,
};

fn field(name: &str) -> serde_json::Value {
    json!({ "name": name, "selection_set": [] })
}

/// Every scalar column on TaxRoutingRecord: `update_item` takes a fully-populated
/// `InputTaxRoutingRecord`, so both actions read the current row first and carry every
/// untouched field forward unchanged.
fn full_record_selection() -> serde_json::Value {
    json!({
        "name": "tax_routing_record",
        "selection_set": [
            field("id"), field("case_number"), field("status"), field("created_at"), field("updated_at"),
            field("assigned_user_id"), field("client_first_name"), field("client_last_name"),
            field("client_full_name"), field("office_location"), field("pds_email"),
            field("personal_email"), field("additional_email"), field("notification_flag"),
            field("internal_folder"), field("folder_name"), field("read_write_password"),
            field("client_reference_id"), field("progress_step"), field("version"),
        ]
    })
}

async fn load_record(
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    record_id: &str,
    user: Option<UserAuth>,
) -> HandlerResult<TaxRoutingRecordProjection> {
    data_access
        .find_item::<TaxRoutingRecordProjection>(
            entity_type.clone(),
            full_record_selection(),
            record_id.to_string(),
            user,
        )
        .await?
        .ok_or_else(|| anyhow::anyhow!("routing record {record_id} was not found"))
}

fn require_field<T>(value: Option<T>, field_name: &str) -> HandlerResult<T> {
    value.ok_or_else(|| anyhow::anyhow!("routing record is missing required field {field_name}"))
}

/// Builds the full replacement input, carrying every current value forward except whatever the
/// caller supplies as an override.
fn to_input(
    current: &TaxRoutingRecordProjection,
    status_override: Option<RoutingRecordStatus>,
    assigned_user_id_override: Option<String>,
) -> HandlerResult<InputTaxRoutingRecord> {
    Ok(InputTaxRoutingRecord {
        id: current.id.clone(),
        case_number: current.case_number,
        status: status_override.unwrap_or(require_field(current.status.clone(), "status")?),
        created_at: require_field(current.created_at, "created_at")?,
        updated_at: chrono::Utc::now(),
        assigned_user_id: assigned_user_id_override
            .unwrap_or(require_field(current.assigned_user_id.clone(), "assigned_user_id")?),
        client_first_name: current.client_first_name.clone(),
        client_last_name: current.client_last_name.clone(),
        client_full_name: current.client_full_name.clone(),
        office_location: current.office_location.clone(),
        pds_email: current.pds_email.clone(),
        personal_email: current.personal_email.clone(),
        additional_email: current.additional_email.clone(),
        notification_flag: current.notification_flag.unwrap_or(false),
        internal_folder: current.internal_folder.clone(),
        folder_name: current.folder_name.clone(),
        read_write_password: require_field(current.read_write_password.clone(), "read_write_password")?,
        client_reference_id: require_field(current.client_reference_id.clone(), "client_reference_id")?,
        progress_step: current.progress_step.unwrap_or(0),
        version: current.version,
    })
}

/// Permanently closes a record (business spec 9.1). Irreversible: there is no reopen path.
///
/// `resolution_status` and `comments` are accepted and returned in the response, but this pass
/// does not yet persist them — `TaxRoutingRecord` has no `resolution_status` / `comments` columns
/// yet (see docs/architecture/rbac-design.md and the design doc's model-work backlog). Until
/// those columns exist, treat this as closing the record with the reason logged, not archived.
pub async fn cancel_record(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    record_id: String,
    resolution_status: String,
    comments: String,
) -> HandlerResult<serde_json::Value> {
    let user = user.ok_or_else(|| anyhow::anyhow!("not authenticated"))?;
    let principal = data_access.app_config.authz().resolve(&user.tenant_id, &user.user_name);
    authz::require(&principal, "routing_record.cancel")?;

    let current = load_record(data_access, entity_type, &record_id, Some(user.clone())).await?;
    let input = to_input(&current, Some(RoutingRecordStatus::cancelled), None)?;

    tracing::info!(
        record_id = %record_id,
        actor = %user.user_name,
        resolution_status = %resolution_status,
        comments = %comments,
        "routing record cancelled"
    );

    let updated = data_access
        .update_item::<InputTaxRoutingRecord, TaxRoutingRecordProjection>(
            entity_type.clone(),
            full_record_selection(),
            input,
            Some(user),
        )
        .await?;

    Ok(json!({
        "id": updated.id,
        "status": updated.status,
        "resolution_status": resolution_status,
        "comments": comments,
    }))
}

/// Changes who a record is assigned to (business spec 12.3: reassignment is an admin action).
///
/// Denormalised copies of the owner on `TaxDocument` / `ExceptionTask` rows are not cascaded by
/// this pass — that is the routing service's job once it exists (see the comment on
/// `TaxDocument.assigned_user_id` in the model). Until it does, a document's own scope can go
/// stale after a reassignment; this is a known follow-up, not something this change hides.
pub async fn reassign_record(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    record_id: String,
    new_assigned_user_id: String,
) -> HandlerResult<serde_json::Value> {
    let user = user.ok_or_else(|| anyhow::anyhow!("not authenticated"))?;
    let principal = data_access.app_config.authz().resolve(&user.tenant_id, &user.user_name);
    authz::require(&principal, "routing_record.reassign")?;

    let current = load_record(data_access, entity_type, &record_id, Some(user.clone())).await?;
    let input = to_input(&current, None, Some(new_assigned_user_id.clone()))?;

    tracing::info!(
        record_id = %record_id,
        actor = %user.user_name,
        new_assigned_user_id = %new_assigned_user_id,
        "routing record reassigned"
    );

    let updated = data_access
        .update_item::<InputTaxRoutingRecord, TaxRoutingRecordProjection>(
            entity_type.clone(),
            full_record_selection(),
            input,
            Some(user),
        )
        .await?;

    Ok(json!({
        "id": updated.id,
        "assigned_user_id": updated.assigned_user_id,
    }))
}
