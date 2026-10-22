//
// Backend tax_routing TaxRoutingRecord Implementation
//    Product-owned handler extension file.
//    Generated once by app_gen, then preserved.
//
#[allow(unused_imports)]
pub(crate) use super::generated::tax_routing_record::*;
#[allow(unused)]
use std::sync::Arc;

use serde_json::json;

#[allow(unused_imports)]
use crate::{
    product_api::{AppError, DataAccess, EntityType, HandlerResult, JsonValue, UserAuth},
    schemas::tax_routing::{InputTaxRoutingRecord, TaxRoutingRecordProjection},
    services::{bulk_requests, document_routing, routing_record_actions},
};

/// How many times `create_impl` will propose a fresh case number after losing a race for one.
const MAX_CASE_NUMBER_ATTEMPTS: usize = 5;
/// How many numbers may be skipped because a folder with that name already exists in storage (a run
/// of Pega's own TD-27xx folders, say) before giving up.
const MAX_TAKEN_FOLDER_SKIPS: usize = 200;

/// True when `err` is the database refusing a duplicate `case_number` (the
/// `uq_tax_routing_record_case_number` UNIQUE constraint), i.e. another creator took the number
/// between our read and our insert. Anything else is a real failure and must not be retried.
fn is_case_number_clash(err: &AppError) -> bool {
    let message = err.to_string().to_lowercase();
    message.contains("uq_tax_routing_record_case_number")
        || (message.contains("case_number")
            && (message.contains("duplicate key") || message.contains("unique")))
}

/// Creates a routing record and gives it the next case number (TD-1001, TD-1002, ...).
///
/// The number is always assigned here, on the server: whatever the caller sent for
/// `case_number` is replaced, so a client can neither pick nor skip one. If two records are
/// created at the same instant and both propose the same number, the UNIQUE constraint rejects
/// the second insert and it is retried with a fresh proposal.
pub(crate) async fn create_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    selections: JsonValue,
    input: InputTaxRoutingRecord,
) -> HandlerResult<TaxRoutingRecordProjection> {
    let mut last_clash: Option<AppError> = None;
    // The lowest number still worth trying: raised when a number's storage folder already exists
    // (for example one of Pega's own TD-27xx folders) so the next attempt moves past it.
    let mut at_least = 0;
    for _ in 0..(MAX_CASE_NUMBER_ATTEMPTS + MAX_TAKEN_FOLDER_SKIPS) {
        let mut attempt = input.clone();
        let case_number = data_access
            .app_config
            .case_numbers()
            .next()
            .await?
            .max(at_least);
        attempt.case_number = Some(case_number);
        // The request's temporary storage folder (TD-1001), made before the record so a storage
        // problem stops the request from being created half-way. It is always a NEW folder: if the
        // name is taken, move on to the next number rather than adopt a folder that is not ours.
        match document_routing::create_temp_folder_for(
            data_access,
            attempt.internal_folder.as_deref(),
            case_number,
        )
        .await?
        {
            document_routing::TempFolder::Created(id) => attempt.temp_folder_id = Some(id),
            document_routing::TempFolder::Taken => {
                at_least = case_number + 1;
                continue;
            }
        }
        let created_folder = attempt.temp_folder_id.clone();
        // Steps 1 and 2 of 9 are done: the record exists and its temporary folder has been created.
        attempt.progress_step = attempt.progress_step.max(2);
        match data_access
            .create_item::<InputTaxRoutingRecord, TaxRoutingRecordProjection>(
                entity_type.clone(),
                selections.clone(),
                attempt,
                user.clone(),
            )
            .await
        {
            Ok(created) => return Ok(created),
            Err(e) => {
                // The record was not saved, so the folder made for it would be left empty and unowned.
                if let Some(id) = created_folder {
                    document_routing::remove_temp_folder(data_access, &id).await;
                }
                if is_case_number_clash(&e) {
                    last_clash = Some(e);
                } else {
                    return Err(e.into());
                }
            }
        }
    }
    Err(anyhow::anyhow!(
        "could not allocate a unique case number after {MAX_CASE_NUMBER_ATTEMPTS} attempts{}",
        last_clash.map(|e| format!(": {e}")).unwrap_or_default()
    ))
}

/// Updates a routing record without ever changing its case number: the stored value replaces
/// whatever the caller sent, so the frontend's progress writes (and any other client) cannot
/// alter or clear it.
pub(crate) async fn update_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    selections: JsonValue,
    mut input: InputTaxRoutingRecord,
) -> HandlerResult<TaxRoutingRecordProjection> {
    if let Some(id) = input.id.clone() {
        let stored = data_access
            .find_item::<TaxRoutingRecordProjection>(
                entity_type.clone(),
                json!({
                    "name": "tax_routing_record",
                    "selection_set": [
                        { "name": "id", "selection_set": [] },
                        { "name": "case_number", "selection_set": [] },
                        { "name": "temp_folder_id", "selection_set": [] }
                    ]
                }),
                id,
                user.clone(),
            )
            .await?;
        let stored = stored.map(|record| (record.case_number, record.temp_folder_id));
        input.case_number = stored.as_ref().and_then(|(case_number, _)| *case_number);
        input.temp_folder_id = stored.and_then(|(_, temp)| temp);
    }
    let updated = data_access
        .update_item::<InputTaxRoutingRecord, TaxRoutingRecordProjection>(
            entity_type.clone(),
            selections,
            input,
            user,
        )
        .await?;
    Ok(updated)
}

/// Admin-only close (business spec 9.1). Delegates to services/routing_record_actions, the one
/// place both this and `reassign_record` share their "read current row, replace it" logic.
pub async fn cancel_record_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    _selections: JsonValue,
    record_id: String,
    resolution_status: String,
    comments: String,
) -> HandlerResult<serde_json::Value> {
    routing_record_actions::cancel_record(
        user,
        data_access,
        entity_type,
        record_id,
        resolution_status,
        comments,
    )
    .await
}

/// Admin-only reassignment (business spec 12.3).
pub async fn reassign_record_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    _selections: JsonValue,
    record_id: String,
    new_assigned_user_id: String,
) -> HandlerResult<serde_json::Value> {
    routing_record_actions::reassign_record(
        user,
        data_access,
        entity_type,
        record_id,
        new_assigned_user_id,
    )
    .await
}

/// Uploads one PDF into the request's temporary storage folder (services/document_routing).
pub async fn upload_document_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    _selections: JsonValue,
    record_id: String,
    file_name: String,
    content_base64: String,
) -> HandlerResult<serde_json::Value> {
    document_routing::upload_document(
        user,
        data_access,
        entity_type,
        record_id,
        file_name,
        content_base64,
    )
    .await
}

/// Saves the uploaded documents and starts filing them (services/document_routing).
pub async fn submit_request_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    _selections: JsonValue,
    record_id: String,
    documents_json: String,
) -> HandlerResult<serde_json::Value> {
    document_routing::submit_request(user, data_access, entity_type, record_id, documents_json)
        .await
}

/// Reads one stored document for the case page's preview and download (services/document_routing).
pub async fn download_document_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    _selections: JsonValue,
    record_id: String,
    document_id: String,
) -> HandlerResult<serde_json::Value> {
    document_routing::download_document(user, data_access, entity_type, record_id, document_id)
        .await
}

/// Bulk request creation (services/bulk_requests): check an Excel file and a ZIP, change the
/// preview, then create one request per client.
pub async fn bulk_preview_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    _entity_type: &Arc<EntityType>,
    _selections: JsonValue,
    file_name: String,
    excel_base64: String,
    zip_base64: String,
) -> HandlerResult<serde_json::Value> {
    bulk_requests::preview(user, data_access, file_name, excel_base64, zip_base64).await
}

#[allow(clippy::too_many_arguments)]
pub async fn bulk_update_document_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    _entity_type: &Arc<EntityType>,
    _selections: JsonValue,
    batch_id: String,
    client_key: String,
    file_name: String,
    document_type: String,
    year: i32,
    entity_kind: String,
    entity_name: String,
    entity_number: String,
    password_protected: bool,
) -> HandlerResult<serde_json::Value> {
    let edit = bulk_requests::DocumentEdit {
        document_type,
        year,
        entity_type: entity_kind,
        entity_name,
        entity_number,
        password_protected,
    };
    bulk_requests::update_document(user, data_access, batch_id, client_key, file_name, edit).await
}

#[allow(clippy::too_many_arguments)]
pub async fn bulk_update_client_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    _entity_type: &Arc<EntityType>,
    _selections: JsonValue,
    batch_id: String,
    client_key: String,
    first_name: String,
    last_name: String,
    full_name: String,
    personal_email: String,
    pds_email: String,
    additional_email: String,
    office_location: String,
    internal_folder: String,
    client_folder: String,
    password: String,
    notify: bool,
) -> HandlerResult<serde_json::Value> {
    let details = bulk_requests::ClientEdit {
        first_name,
        last_name,
        full_name,
        personal_email,
        pds_email,
        additional_email,
        office_location,
        internal_folder,
        client_folder,
        password,
        notify,
    };
    bulk_requests::update_client(user, data_access, batch_id, client_key, details).await
}

pub async fn bulk_get_document_impl(
    user: Option<UserAuth>,
    _data_access: &Arc<DataAccess>,
    _entity_type: &Arc<EntityType>,
    _selections: JsonValue,
    batch_id: String,
    client_key: String,
    file_name: String,
) -> HandlerResult<serde_json::Value> {
    bulk_requests::get_document(user, batch_id, client_key, file_name)
}

pub async fn bulk_remove_impl(
    user: Option<UserAuth>,
    _data_access: &Arc<DataAccess>,
    _entity_type: &Arc<EntityType>,
    _selections: JsonValue,
    batch_id: String,
    client_key: String,
    file_name: String,
) -> HandlerResult<serde_json::Value> {
    bulk_requests::remove(user, batch_id, client_key, file_name)
}

pub async fn bulk_confirm_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    _selections: JsonValue,
    batch_id: String,
) -> HandlerResult<serde_json::Value> {
    bulk_requests::confirm(user, data_access, entity_type, batch_id)
}

pub async fn bulk_status_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    _selections: JsonValue,
    batch_id: String,
) -> HandlerResult<serde_json::Value> {
    bulk_requests::status(user, data_access, entity_type, batch_id).await
}

pub async fn bulk_retry_failed_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    _selections: JsonValue,
    batch_id: String,
) -> HandlerResult<serde_json::Value> {
    bulk_requests::retry_failed(user, data_access, entity_type, batch_id)
}

pub async fn bulk_cancel_impl(
    user: Option<UserAuth>,
    _data_access: &Arc<DataAccess>,
    _entity_type: &Arc<EntityType>,
    _selections: JsonValue,
    batch_id: String,
) -> HandlerResult<serde_json::Value> {
    bulk_requests::cancel(user, batch_id)
}

/// Abandons a request that was never submitted and removes its temporary folder.
pub async fn discard_request_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    _selections: JsonValue,
    record_id: String,
) -> HandlerResult<serde_json::Value> {
    document_routing::discard_request(user, data_access, entity_type, record_id).await
}
