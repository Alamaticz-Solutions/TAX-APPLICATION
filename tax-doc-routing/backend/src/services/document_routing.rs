//! Document upload, submission and filing for a routing record: the Pega process (temp folder,
//! upload, `CreateTargetFolders`, move, clean-up) run against `services::document_store`.
//!
//! Three custom mutations call in here (`upload_document`, `submit_request`, `discard_request`),
//! and `create_impl` calls `create_temp_folder_for`. Submission starts a server-side pipeline
//! that keeps going whether or not the browser stays open, writes its progress to the record, and
//! on a failure opens an exception task and leaves the temp folder in place so a retry can
//! finish the job. Every storage step is safe to repeat.
//!
//! Like `routing_record_actions`, these are custom mutations that bypass the Rego row filter, so
//! the caller is checked against a permission code and the record is always read as the caller
//! (row scoping still applies: staff only reach their own records).

use std::sync::Arc;

use base64::Engine;
use futures::{stream, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    product_api::{entity_type_for_handler, DataAccess, EntityType, HandlerResult, UserAuth},
    schemas::tax_routing::{
        DocumentTypeRouteProjection, ExceptionTaskProjection, InputExceptionTask, InputTaxDocument,
        InputTaxRoutingRecord, RoutingRecordStatus, TaxDocumentProjection, TaxRoutingRecordProjection,
    },
    services::{
        authz,
        document_store::{
            routing::{find_route, DocumentRoute},
            workflow::{prepare_document, place_prepared_cached, upload_to_temp, validate_upload, FolderCache, Prepared, Roots, SubmitDocument, UploadRules, WorkflowError},
            DocumentStore, FileRef, FolderRef, StoreError,
        },
        pdf_protection::ClientPasswordProtector,
        routing_record_actions::{full_record_selection, load_record, to_input},
    },
};

/// Pega hardcodes this folder name; here it is a setting with the same default.
const RELEASED_FOLDER_DEFAULT: &str = "6. Released to Client";
/// The 2 MB upload limit the client confirmed, as a default that can be changed without code.
const MAX_UPLOAD_BYTES_DEFAULT: usize = 2 * 1024 * 1024;

fn field(name: &str) -> Value {
    json!({ "name": name, "selection_set": [] })
}

pub fn upload_rules() -> UploadRules {
    let max_bytes = std::env::var("MAX_UPLOAD_BYTES")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(MAX_UPLOAD_BYTES_DEFAULT);
    UploadRules { max_bytes, ..UploadRules::default() }
}

fn setting(name: &str) -> Option<String> {
    std::env::var(name).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

/// External filing (to the client's own Client folder id) is on by default; set
/// `STORAGE_EXTERNAL_ENABLED=false` to switch it off.
fn external_enabled() -> bool {
    !matches!(setting("STORAGE_EXTERNAL_ENABLED").map(|v| v.to_ascii_lowercase()).as_deref(), Some("false" | "0" | "no" | "off"))
}

fn clean_name_part(raw: &str) -> String {
    let replaced: String = raw.chars().map(|c| if c.is_control() || "/\\:*?\"<>|".contains(c) { '_' } else { c }).collect();
    replaced.trim().trim_end_matches('.').trim().to_string()
}

fn released_folder_name() -> String {
    std::env::var("RELEASED_TO_CLIENT_FOLDER_NAME")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| RELEASED_FOLDER_DEFAULT.to_string())
}

fn storage_error(e: StoreError) -> anyhow::Error {
    anyhow::anyhow!("Document storage: {e}")
}

// ---------------------------------------------------------------------------------------------
// Phase 1: the temp folder, at request creation
// ---------------------------------------------------------------------------------------------

/// Where `TD-1001` temp folders are created: the folder named by `STORAGE_TEMP_PARENT_FOLDER` (a
/// Box folder id or URL) when it is set, otherwise the client's own Internal folder.
fn choose_temp_parent(setting: Option<&str>, client_internal: Option<&str>) -> Option<String> {
    [setting, client_internal]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|v| !v.is_empty())
        .map(str::to_string)
}

/// Creates the request's temporary folder, named with the case number (`TD-1001`), and returns its
/// id. It goes inside the folder set in `STORAGE_TEMP_PARENT_FOLDER`, or the client's Internal
/// folder if that is not set. Safe to repeat: an existing folder is reused.
pub async fn create_temp_folder_for(
    data_access: &Arc<DataAccess>,
    internal_folder: Option<&str>,
    case_number: i32,
) -> HandlerResult<TempFolder> {
    let setting = std::env::var("STORAGE_TEMP_PARENT_FOLDER").ok();
    let value = choose_temp_parent(setting.as_deref(), internal_folder).ok_or_else(|| {
        anyhow::anyhow!("No folder is set for this request's temporary files: set STORAGE_TEMP_PARENT_FOLDER or give the client an Internal folder.")
    })?;
    let value = value.as_str();
    let store = data_access.app_config.document_store().clone();
    let root = store.parse_folder(value).map_err(storage_error)?;
    // Created, never reused: the temp folder is deleted after filing, so it must not be a folder
    // that already existed (Pega's own TD-2797, say) with someone else's files in it.
    match store.create_folder(&root, &format!("TD-{case_number}")).await {
        Ok(temp) => Ok(TempFolder::Created(temp.0)),
        Err(StoreError::AlreadyExists(_)) => Ok(TempFolder::Taken),
        Err(e) => Err(storage_error(e)),
    }
}

/// Removes a temp folder that no record points at (best effort; a failure is only logged).
pub async fn remove_temp_folder(data_access: &Arc<DataAccess>, folder_id: &str) {
    if let Err(e) = data_access.app_config.document_store().delete_folder(&FolderRef(folder_id.to_string())).await {
        tracing::warn!(folder = folder_id, error = %e, "could not remove an unused temp folder");
    }
}

/// Result of making a request's temp folder.
pub enum TempFolder {
    Created(String),
    /// A folder with that name was already there; the caller picks the next case number.
    Taken,
}

fn require_collecting(record: &TaxRoutingRecordProjection) -> HandlerResult<()> {
    match record.status {
        Some(RoutingRecordStatus::collecting_info) => Ok(()),
        _ => Err(anyhow::anyhow!("Documents can only be added or submitted while the request is still being collected.")),
    }
}

async fn authorised_record(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    record_id: &str,
) -> HandlerResult<(UserAuth, TaxRoutingRecordProjection)> {
    let user = user.ok_or_else(|| anyhow::anyhow!("not authenticated"))?;
    let principal = data_access.app_config.authz().resolve(&user.tenant_id, &user.user_name);
    authz::require(&principal, "routing_record.update")?;
    let record = load_record(data_access, entity_type, record_id, Some(user.clone())).await?;
    Ok((user, record))
}

/// The record's temp folder, created now if it is somehow missing.
async fn temp_folder(
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    user: &UserAuth,
    record: &TaxRoutingRecordProjection,
) -> HandlerResult<FolderRef> {
    if let Some(id) = record.temp_folder_id.as_deref().filter(|v| !v.is_empty()) {
        return Ok(FolderRef(id.to_string()));
    }
    let case = record.case_number.ok_or_else(|| anyhow::anyhow!("the request has no case number"))?;
    let id = match create_temp_folder_for(data_access, record.internal_folder.as_deref(), case).await? {
        TempFolder::Created(id) => id,
        TempFolder::Taken => return Err(anyhow::anyhow!("A folder named TD-{case} already exists in storage, so this request has no temporary folder.")),
    };
    let mut input = to_input(record, None, None)?;
    input.temp_folder_id = Some(id.clone());
    data_access
        .update_item::<InputTaxRoutingRecord, TaxRoutingRecordProjection>(entity_type.clone(), full_record_selection(), input, Some(user.clone()))
        .await?;
    Ok(FolderRef(id))
}

// ---------------------------------------------------------------------------------------------
// Phase 1: one uploaded PDF
// ---------------------------------------------------------------------------------------------

pub async fn upload_document(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    record_id: String,
    file_name: String,
    content_base64: String,
) -> HandlerResult<Value> {
    let (user, record) = authorised_record(user, data_access, entity_type, &record_id).await?;
    upload_for_record(&user, data_access, entity_type, &record, file_name, content_base64, false).await
}

/// Stores one PDF in a request's temp folder. `record` is the request as already read by the
/// caller (a bulk upload has just created it, so reading it again per file is wasted work).
/// With `reuse`, a file of that name already in the folder is taken as the upload (a retry after
/// a failure part-way through), instead of being refused as a duplicate.
pub(crate) async fn upload_for_record(
    user: &UserAuth,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    record: &TaxRoutingRecordProjection,
    file_name: String,
    content_base64: String,
    reuse: bool,
) -> HandlerResult<Value> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(content_base64.trim())
        .map_err(|_| anyhow::anyhow!("The file content could not be read."))?;
    upload_bytes_for_record(user, data_access, entity_type, record, file_name, bytes, reuse).await
}

/// `upload_for_record` for a file already in hand (no base64 round trip).
pub(crate) async fn upload_bytes_for_record(
    user: &UserAuth,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    record: &TaxRoutingRecordProjection,
    file_name: String,
    bytes: Vec<u8>,
    reuse: bool,
) -> HandlerResult<Value> {
    let record_id = record.id.clone().unwrap_or_default();
    require_collecting(record)?;
    let size = bytes.len();

    let store = data_access.app_config.document_store().clone();
    let temp = temp_folder(data_access, entity_type, user, record).await?;
    let stored = if reuse {
        match validate_upload(&file_name, size, &upload_rules()) {
            Ok(()) => store.upload_or_existing(&temp, &file_name, bytes).await.map_err(WorkflowError::from),
            Err(e) => Err(e),
        }
    } else {
        upload_to_temp(store.as_ref(), &temp, &file_name, bytes, &upload_rules()).await
    };
    let file = stored
        .map_err(|e| match e {
            WorkflowError::InvalidUpload(why) => anyhow::anyhow!("{file_name}: {why}."),
            WorkflowError::Store(StoreError::AlreadyExists(_)) => {
                anyhow::anyhow!("A file named {file_name} was already uploaded for this request.")
            }
            WorkflowError::Store(StoreError::InvalidName(_)) => {
                anyhow::anyhow!("{file_name}: the file name contains characters that cannot be used.")
            }
            other => anyhow::anyhow!("{file_name}: {other}"),
        })?;

    tracing::info!(record_id = %record_id, file = %file_name, bytes = size, "document uploaded to the temp folder");
    Ok(json!({ "file_id": file.0, "file_name": file_name, "file_size": size, "temp_folder_id": temp.0 }))
}

/// One stored document of a request, for the case page's preview and download. Row scoping is the
/// record load (a staff member only sees their own records); the read permission is checked on top.
pub async fn download_document(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    record_id: String,
    document_id: String,
) -> HandlerResult<Value> {
    let user = user.ok_or_else(|| anyhow::anyhow!("not authenticated"))?;
    let principal = data_access.app_config.authz().resolve(&user.tenant_id, &user.user_name);
    authz::require(&principal, "routing_record.read")?;
    let record = load_record(data_access, entity_type, &record_id, Some(user.clone())).await?;
    let doc = documents_of(data_access, &user, &record_id)
        .await?
        .into_iter()
        .find(|d| d.id.as_deref() == Some(document_id.as_str()))
        .ok_or_else(|| anyhow::anyhow!("That document is not on this request."))?;
    let file_id = doc.storage_file_id.clone().filter(|v| !v.is_empty());
    let Some(file_id) = file_id.filter(|_| doc.is_file_moved.unwrap_or(false) || record_is_collecting(&record)) else {
        return Err(anyhow::anyhow!("The stored copy of this document is not available."));
    };
    let bytes = data_access
        .app_config
        .document_store()
        .read_file(&FileRef(file_id))
        .await
        .map_err(|e| match e {
            StoreError::NotFound(_) => anyhow::anyhow!("The stored copy of this document is no longer available."),
            other => storage_error(other),
        })?;
    let name = doc.document_name.clone().unwrap_or_else(|| "document.pdf".to_string());
    Ok(json!({ "file_name": name, "content_base64": base64::engine::general_purpose::STANDARD.encode(bytes) }))
}

fn record_is_collecting(record: &TaxRoutingRecordProjection) -> bool {
    matches!(record.status, Some(RoutingRecordStatus::collecting_info))
}

// ---------------------------------------------------------------------------------------------
// Submit
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct SubmittedDocument {
    file_id: String,
    file_name: String,
    document_type: String,
    year: i32,
    #[serde(default)]
    file_size: Option<i32>,
    #[serde(default)]
    entity_type: Option<String>,
    #[serde(default)]
    entity_oracle_id: Option<String>,
    #[serde(default)]
    entity_name: Option<String>,
    #[serde(default)]
    password_protected: bool,
    #[serde(default)]
    k1_attachments: Vec<String>,
    #[serde(default)]
    form_8308_attachments: Vec<String>,
}

fn parse_submitted(documents_json: &str) -> HandlerResult<Vec<SubmittedDocument>> {
    let docs: Vec<SubmittedDocument> =
        serde_json::from_str(documents_json).map_err(|e| anyhow::anyhow!("The document list could not be read: {e}"))?;
    if docs.is_empty() {
        return Err(anyhow::anyhow!("Upload at least one document before submitting."));
    }
    let mut seen = std::collections::HashSet::new();
    for d in &docs {
        if d.file_id.trim().is_empty() || d.file_name.trim().is_empty() || d.document_type.trim().is_empty() {
            return Err(anyhow::anyhow!("Every document needs an uploaded file, a name and a document type."));
        }
        if !(1900..=2200).contains(&d.year) {
            return Err(anyhow::anyhow!("{}: choose a valid document year.", d.file_name));
        }
        let key = (d.document_type.to_lowercase(), d.year, d.file_name.to_lowercase());
        if !seen.insert(key) {
            return Err(anyhow::anyhow!("{} is listed twice for the same type and year.", d.file_name));
        }
    }
    Ok(docs)
}

/// The document types set up for filing, with whether each supports password protection. Used by
/// bulk upload to recognise a type from a file name.
pub(crate) async fn document_types(data_access: &Arc<DataAccess>, user: &UserAuth) -> HandlerResult<Vec<(String, bool)>> {
    Ok(load_routes(data_access, user)
        .await?
        .into_iter()
        .map(|r| (r.route.document_type, r.supports_password_protection))
        .collect())
}

#[derive(Clone)]
struct RouteRow {
    id: String,
    route: DocumentRoute,
    supports_password_protection: bool,
}

/// How long a read of the document types is reused. They change only when an administrator edits
/// the routing table, and every request filed (and every bulk upload) needs them.
const ROUTES_TTL: std::time::Duration = std::time::Duration::from_secs(30);

fn routes_cache() -> &'static std::sync::Mutex<std::collections::HashMap<String, (std::time::Instant, Vec<RouteRow>)>> {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, (std::time::Instant, Vec<RouteRow>)>>> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

async fn load_routes(data_access: &Arc<DataAccess>, user: &UserAuth) -> HandlerResult<Vec<RouteRow>> {
    // Read as the caller (row rules apply), so the reuse is per caller.
    let key = format!("{}|{}", user.tenant_id, user.user_name);
    if let Ok(cache) = routes_cache().lock() {
        if let Some((at, rows)) = cache.get(&key) {
            if at.elapsed() < ROUTES_TTL {
                return Ok(rows.clone());
            }
        }
    }
    let rows = read_routes(data_access, user).await?;
    if let Ok(mut cache) = routes_cache().lock() {
        cache.insert(key, (std::time::Instant::now(), rows.clone()));
    }
    Ok(rows)
}

async fn read_routes(data_access: &Arc<DataAccess>, user: &UserAuth) -> HandlerResult<Vec<RouteRow>> {
    let entity_type = entity_type_for_handler(data_access, "tax_routing", "DocumentTypeRoute")?;
    let selection = json!({
        "name": "document_type_route",
        "selection_set": [
            field("id"), field("document_type"), field("internal_folder"), field("external_folder"),
            field("is_external_only"), field("supports_password_protection"),
        ]
    });
    let result = data_access
        .query_items::<DocumentTypeRouteProjection>(entity_type, selection, None, None, 0, 250, None, Some(user.clone()))
        .await?;
    Ok(result
        .items
        .into_iter()
        .filter_map(|r| {
            Some(RouteRow {
                id: r.id?,
                supports_password_protection: r.supports_password_protection.unwrap_or(true),
                route: DocumentRoute {
                    document_type: r.document_type?,
                    internal_folder: r.internal_folder.filter(|v| !v.trim().is_empty()),
                    external_folder: r.external_folder.unwrap_or_default(),
                    is_external_only: r.is_external_only.unwrap_or(false),
                },
            })
        })
        .collect())
}

fn doc_selection() -> Value {
    json!({
        "name": "tax_document",
        "selection_set": [
            field("id"), field("routing_record_id"), field("document_type_route_id"), field("assigned_user_id"),
            field("document_year"), field("document_name"), field("entity_type"), field("entity_oracle_id"),
            field("entity_name"), field("is_password_protected"), field("box_file_url"), field("is_duplicate"),
            field("attachment_file"), field("k1_attachments"), field("form_8308_attachments"),
            field("amended_documents"), field("storage_file_id"), field("file_size"), field("is_file_moved"),
            field("version"),
        ]
    })
}

async fn documents_of(data_access: &Arc<DataAccess>, user: &UserAuth, record_id: &str) -> HandlerResult<Vec<TaxDocumentProjection>> {
    let entity_type = entity_type_for_handler(data_access, "tax_routing", "TaxDocument")?;
    let result = data_access
        .query_items::<TaxDocumentProjection>(
            entity_type,
            doc_selection(),
            Some(json!({ "routing_record_id": { "_eq": record_id } })),
            None,
            0,
            250,
            None,
            Some(user.clone()),
        )
        .await?;
    Ok(result.items)
}

fn doc_input(d: &TaxDocumentProjection, moved: Option<bool>) -> HandlerResult<InputTaxDocument> {
    let need = |v: &Option<String>, what: &str| v.clone().ok_or_else(|| anyhow::anyhow!("document is missing {what}"));
    Ok(InputTaxDocument {
        id: d.id.clone(),
        routing_record_id: need(&d.routing_record_id, "routing_record_id")?,
        document_type_route_id: need(&d.document_type_route_id, "document_type_route_id")?,
        assigned_user_id: need(&d.assigned_user_id, "assigned_user_id")?,
        document_year: d.document_year.ok_or_else(|| anyhow::anyhow!("document is missing its year"))?,
        document_name: need(&d.document_name, "document_name")?,
        entity_type: d.entity_type.clone(),
        entity_oracle_id: d.entity_oracle_id.clone(),
        entity_name: d.entity_name.clone(),
        is_password_protected: d.is_password_protected.unwrap_or(false),
        box_file_url: d.box_file_url.clone(),
        is_duplicate: d.is_duplicate,
        attachment_file: need(&d.attachment_file, "attachment_file")?,
        k1_attachments: d.k1_attachments.clone(),
        form_8308_attachments: d.form_8308_attachments.clone(),
        amended_documents: d.amended_documents.clone(),
        storage_file_id: d.storage_file_id.clone(),
        file_size: d.file_size,
        is_file_moved: moved.unwrap_or(d.is_file_moved.unwrap_or(false)),
        version: d.version,
    })
}

pub async fn submit_request(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    record_id: String,
    documents_json: String,
) -> HandlerResult<Value> {
    let (user, record) = authorised_record(user, data_access, entity_type, &record_id).await?;
    submit_for_record(user, data_access, entity_type, record, documents_json, false).await
}

/// Saves a request's documents and starts filing. `record` is the request as already read by the
/// caller. `fresh` says the request has never had documents saved (a brand new bulk request), so
/// the look for an earlier list to clear away is skipped.
pub(crate) async fn submit_for_record(
    user: UserAuth,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    record: TaxRoutingRecordProjection,
    documents_json: String,
    fresh: bool,
) -> HandlerResult<Value> {
    let record_id = record.id.clone().unwrap_or_default();
    require_collecting(&record)?;
    let submitted = parse_submitted(&documents_json)?;
    if record.temp_folder_id.as_deref().map_or(true, str::is_empty) {
        return Err(anyhow::anyhow!("This request has no temporary folder; its documents were not uploaded."));
    }

    let routes = load_routes(data_access, &user).await?;
    let known: Vec<DocumentRoute> = routes.iter().map(|r| r.route.clone()).collect();
    let assignee = record
        .assigned_user_id
        .clone()
        .ok_or_else(|| anyhow::anyhow!("the request has no assignee"))?;
    let doc_entity = entity_type_for_handler(data_access, "tax_routing", "TaxDocument")?;

    // A resubmission starts from a clean list rather than stacking duplicates.
    if !fresh {
        for old in documents_of(data_access, &user, &record_id).await? {
            let input = doc_input(&old, None)?;
            data_access.delete_item::<InputTaxDocument>(doc_entity.clone(), input, Some(user.clone())).await?;
        }
    }

    // The documents are independent rows, so they are saved together.
    let mut saves = Vec::with_capacity(submitted.len());
    for d in &submitted {
        let route = find_route(&known, &d.document_type).map_err(|_| {
            anyhow::anyhow!("{}: \"{}\" is not a document type that can be filed.", d.file_name, d.document_type)
        })?;
        let row = routes.iter().find(|r| r.route.document_type == route.document_type).expect("found above");
        let input = InputTaxDocument {
            id: None,
            routing_record_id: record_id.clone(),
            document_type_route_id: row.id.clone(),
            assigned_user_id: assignee.clone(),
            document_year: d.year,
            document_name: d.file_name.clone(),
            entity_type: d.entity_type.clone().filter(|v| !v.is_empty()),
            entity_oracle_id: d.entity_oracle_id.clone().filter(|v| !v.is_empty()),
            entity_name: d.entity_name.clone().filter(|v| !v.is_empty()),
            // A type that cannot be protected is never marked protected.
            is_password_protected: d.password_protected && row.supports_password_protection,
            box_file_url: None,
            is_duplicate: Some(false),
            attachment_file: d.file_name.clone(),
            k1_attachments: Some(d.k1_attachments.clone()),
            form_8308_attachments: Some(d.form_8308_attachments.clone()),
            amended_documents: Some(vec![]),
            storage_file_id: Some(d.file_id.clone()),
            file_size: d.file_size,
            is_file_moved: false,
            version: None,
        };
        let (da, et, who) = (data_access.clone(), doc_entity.clone(), user.clone());
        saves.push(async move {
            da.create_item::<InputTaxDocument, TaxDocumentProjection>(et, json!({ "name": "tax_document", "selection_set": [field("id")] }), input, Some(who)).await
        });
    }
    futures::future::try_join_all(saves).await?;

    // Documents are saved: the request is now processing at step 3 ("documents uploaded").
    let mut input = to_input(&record, Some(RoutingRecordStatus::processing), None)?;
    input.progress_step = 3;
    data_access
        .update_item::<InputTaxRoutingRecord, TaxRoutingRecordProjection>(entity_type.clone(), full_record_selection(), input, Some(user.clone()))
        .await?;

    spawn_pipeline(user, data_access.clone(), record_id.clone());
    tracing::info!(record_id = %record_id, documents = submitted.len(), "request submitted; filing started");
    Ok(json!({ "record_id": record_id, "status": "processing", "documents": submitted.len() }))
}

// ---------------------------------------------------------------------------------------------
// Discard
// ---------------------------------------------------------------------------------------------

pub async fn discard_request(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    record_id: String,
) -> HandlerResult<Value> {
    let (user, record) = authorised_record(user, data_access, entity_type, &record_id).await?;
    require_collecting(&record)?;

    if let Some(temp) = record.temp_folder_id.as_deref().filter(|v| !v.is_empty()) {
        match data_access.app_config.document_store().delete_folder(&FolderRef(temp.to_string())).await {
            Ok(()) | Err(StoreError::NotFound(_)) => {}
            Err(e) => return Err(storage_error(e)),
        }
    }
    let mut input = to_input(&record, Some(RoutingRecordStatus::cancelled), None)?;
    input.temp_folder_id = None;
    data_access
        .update_item::<InputTaxRoutingRecord, TaxRoutingRecordProjection>(entity_type.clone(), full_record_selection(), input, Some(user))
        .await?;
    tracing::info!(record_id = %record_id, "unsubmitted request discarded");
    Ok(json!({ "record_id": record_id, "status": "cancelled" }))
}

// ---------------------------------------------------------------------------------------------
// The filing pipeline (Pega: CreateTargetFolders, move, DeleteTempFolder)
// ---------------------------------------------------------------------------------------------

/// Starts (or restarts, after a retry) the filing pipeline in the background.
/// How many requests are filed at the same time (`FILING_CONCURRENCY`, default 8). A bulk upload
/// submits a hundred requests within seconds; filing them all at once would flood the database and
/// Box, so the rest wait their turn here.
fn filing_turns() -> &'static tokio::sync::Semaphore {
    static TURNS: std::sync::OnceLock<tokio::sync::Semaphore> = std::sync::OnceLock::new();
    TURNS.get_or_init(|| {
        let n = std::env::var("FILING_CONCURRENCY").ok().and_then(|v| v.trim().parse::<usize>().ok()).filter(|n| *n > 0).unwrap_or(8);
        tokio::sync::Semaphore::new(n)
    })
}

/// How many of one request's files are read, protected or filed at the same time.
const FILES_AT_ONCE: usize = 4;

pub fn spawn_pipeline(user: UserAuth, data_access: Arc<DataAccess>, record_id: String) {
    tokio::spawn(async move {
        let _turn = filing_turns().acquire().await;
        let pipeline = match Pipeline::new(user, data_access, record_id.clone()) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!(record_id = %record_id, error = %e, "filing pipeline could not start");
                return;
            }
        };
        if let Err(e) = pipeline.run().await {
            tracing::error!(record_id = %record_id, error = %e, "filing pipeline stopped");
        }
    });
}

/// "Last, First - OriginalFileName.pdf": the name written at every destination.
pub fn target_file_name(last: Option<&str>, first: Option<&str>, full: Option<&str>, original: &str) -> String {
    let clean = clean_name_part;
    let part = |v: Option<&str>| v.map(clean).filter(|s| !s.is_empty());
    let who = match (part(last), part(first)) {
        (Some(l), Some(f)) => format!("{l}, {f}"),
        _ => part(full).unwrap_or_else(|| "Client".to_string()),
    };
    format!("{who} - {}", clean(original))
}

struct Pipeline {
    user: UserAuth,
    da: Arc<DataAccess>,
    record_id: String,
    record_et: Arc<EntityType>,
    doc_et: Arc<EntityType>,
    exc_et: Arc<EntityType>,
    store: Arc<dyn DocumentStore>,
    /// The record as last written by this pipeline, so its next progress write needs no read.
    latest: tokio::sync::Mutex<Option<TaxRoutingRecordProjection>>,
    /// When this pipeline run began, for the per-step timings in the log.
    started: std::time::Instant,
}

enum Outcome {
    /// The step failed; an exception task has been opened and the temp folder kept.
    Failed,
    Continue,
}

impl Pipeline {
    fn new(user: UserAuth, da: Arc<DataAccess>, record_id: String) -> HandlerResult<Self> {
        Ok(Self {
            record_et: entity_type_for_handler(&da, "tax_routing", "TaxRoutingRecord")?,
            doc_et: entity_type_for_handler(&da, "tax_routing", "TaxDocument")?,
            exc_et: entity_type_for_handler(&da, "tax_routing", "ExceptionTask")?,
            store: da.app_config.document_store().clone(),
            user,
            da,
            record_id,
            latest: tokio::sync::Mutex::new(None),
            started: std::time::Instant::now(),
        })
    }

    async fn record(&self) -> HandlerResult<TaxRoutingRecordProjection> {
        load_record(&self.da, &self.record_et, &self.record_id, Some(self.user.clone())).await
    }

    /// Saves where the request has got to. Stops quietly if someone cancelled it meanwhile.
    async fn progress(&self, status: RoutingRecordStatus, step: i32, clear_temp: bool) -> HandlerResult<bool> {
        // The record is read fresh before the long storage steps and at the end, which is where a
        // cancel would be noticed; between those the version from the last write is reused. (If
        // someone else changed the record meanwhile, the write is refused by the version check.)
        let fresh = matches!(step, 4 | 9) || (step == 7 && !clear_temp);
        let remembered = if fresh { None } else { self.latest.lock().await.clone() };
        let current = match remembered {
            Some(record) => record,
            None => self.record().await?,
        };
        if matches!(current.status, Some(RoutingRecordStatus::cancelled)) {
            return Ok(false);
        }
        let mut input = to_input(&current, Some(status), None)?;
        input.progress_step = step;
        if clear_temp {
            input.temp_folder_id = None;
        }
        let updated = self
            .da
            .update_item::<InputTaxRoutingRecord, TaxRoutingRecordProjection>(self.record_et.clone(), full_record_selection(), input, Some(self.user.clone()))
            .await?;
        *self.latest.lock().await = Some(updated);
        tracing::debug!(record_id = %self.record_id, step, ms = self.started.elapsed().as_millis() as u64, "filing step reached");
        Ok(true)
    }

    /// Opens an exception task and marks the record as needing attention. The temp folder stays.
    async fn fail(&self, reason: String, step: &str) -> HandlerResult<Outcome> {
        let current = self.record().await?;
        tracing::warn!(record_id = %self.record_id, step, reason = %reason, "filing step failed");
        let exception = InputExceptionTask {
            id: None,
            routing_record_id: self.record_id.clone(),
            failure_reason: reason,
            failed_step: step.to_string(),
            opened_at: chrono::Utc::now(),
            resolved_at: None,
            assigned_user_id: current.assigned_user_id.clone(),
            case_number: current.case_number,
            version: None,
        };
        self.da
            .create_item::<InputExceptionTask, ExceptionTaskProjection>(
                self.exc_et.clone(),
                json!({ "name": "exception_task", "selection_set": [field("id")] }),
                exception,
                Some(self.user.clone()),
            )
            .await?;
        let mut input = to_input(&current, Some(RoutingRecordStatus::exception), None)?;
        input.progress_step = current.progress_step.unwrap_or(0);
        self.da
            .update_item::<InputTaxRoutingRecord, TaxRoutingRecordProjection>(self.record_et.clone(), full_record_selection(), input, Some(self.user.clone()))
            .await?;
        Ok(Outcome::Failed)
    }

    async fn run(&self) -> HandlerResult<()> {
        let record = self.record().await?;
        if matches!(record.status, Some(RoutingRecordStatus::cancelled) | Some(RoutingRecordStatus::resolved)) {
            return Ok(());
        }
        let docs = documents_of(&self.da, &self.user, &self.record_id).await?;

        // Records that predate document storage have nothing to file: just finish their steps.
        if docs.is_empty() && record.temp_folder_id.as_deref().map_or(true, str::is_empty) {
            for step in (record.progress_step.unwrap_or(0) + 1)..=8 {
                if !self.progress(RoutingRecordStatus::processing, step, false).await? {
                    return Ok(());
                }
            }
            self.progress(RoutingRecordStatus::resolved, 9, false).await?;
            return Ok(());
        }

        // Step 4: verify the client's folders (Pega: CheckClientBoxLocation).
        if !self.progress(RoutingRecordStatus::processing, 4, false).await? {
            return Ok(());
        }
        // The roots are the client's own folder ids from the client record (shown on screen 1): the
        // Internal folder id and the Client (External) folder id. The year folder is created directly
        // inside them.
        let internal_root = match self.store.parse_folder(record.internal_folder.as_deref().unwrap_or("")) {
            Ok(f) => f,
            Err(e) => return self.fail(format!("Storage Folder Error: the client's Internal folder id is not usable ({e})."), "verify").await.map(|_| ()),
        };
        match self.store.folder_exists(&internal_root).await {
            Ok(true) => {}
            Ok(false) => return self.fail("Storage Folder Error: the client's Internal folder could not be found in storage. Check the Internal Folder ID in the client record.".into(), "verify").await.map(|_| ()),
            Err(e) => return self.fail(format!("Storage Folder Error: {e}"), "verify").await.map(|_| ()),
        }
        let client_root: Option<FolderRef> = if external_enabled() {
            let folder = match self.store.parse_folder(record.folder_name.as_deref().unwrap_or("")) {
                Ok(f) => f,
                Err(e) => return self.fail(format!("Storage Folder Error: the client's Client (External) folder id is not usable ({e})."), "verify").await.map(|_| ()),
            };
            match self.store.folder_exists(&folder).await {
                Ok(true) => Some(folder),
                Ok(false) => return self.fail("Storage Folder Error: the client's Client (External) folder could not be found in storage. Check the Client Folder ID in the client record.".into(), "verify").await.map(|_| ()),
                Err(e) => return self.fail(format!("Storage Folder Error: {e}"), "verify").await.map(|_| ()),
            }
        } else {
            None
        };
        let temp = match record.temp_folder_id.as_deref().filter(|v| !v.is_empty()) {
            Some(id) => FolderRef(id.to_string()),
            None => return self.fail("Storage Folder Error: the temporary folder is missing.".into(), "verify").await.map(|_| ()),
        };

        // Step 5: read each file and protect the client copy where asked.
        if !self.progress(RoutingRecordStatus::processing, 5, false).await? {
            return Ok(());
        }
        let routes = load_routes(&self.da, &self.user).await?;
        let protector = ClientPasswordProtector::new(record.read_write_password.as_deref().unwrap_or(""));
        let mut sources: Vec<(TaxDocumentProjection, SubmitDocument)> = Vec::new();
        for doc in docs.iter().filter(|d| !d.is_file_moved.unwrap_or(false)) {
            let name = doc.document_name.clone().unwrap_or_default();
            let Some(route) = routes.iter().find(|r| Some(&r.id) == doc.document_type_route_id.as_ref()) else {
                return self.fail(format!("Processing Error: {name} has a document type that is no longer set up for filing."), "processing").await.map(|_| ());
            };
            sources.push((
                doc.clone(),
                SubmitDocument {
                    temp_file: FileRef(doc.storage_file_id.clone().unwrap_or_default()),
                    target_name: target_file_name(record.client_last_name.as_deref(), record.client_first_name.as_deref(), record.client_full_name.as_deref(), &name),
                    year: doc.document_year.unwrap_or_default(),
                    route: route.route.clone(),
                    protect_client_copies: doc.is_password_protected.unwrap_or(false),
                },
            ));
        }
        let read: Vec<(TaxDocumentProjection, Result<Prepared, WorkflowError>)> = stream::iter(sources.into_iter().map(|(doc, source)| {
            let (store, protector) = (self.store.clone(), &protector);
            async move {
                let result = prepare_document(store.as_ref(), protector, &source).await;
                (doc, result)
            }
        }))
        .buffer_unordered(FILES_AT_ONCE)
        .collect()
        .await;
        let mut prepared: Vec<(TaxDocumentProjection, Prepared)> = Vec::new();
        for (doc, result) in read {
            let name = doc.document_name.clone().unwrap_or_default();
            match result {
                Ok(p) => prepared.push((doc, p)),
                Err(WorkflowError::Protection(why)) => {
                    return self.fail(format!("Processing Error: password protection failed for {name}; the file was not filed ({why})."), "processing").await.map(|_| ())
                }
                Err(e) => return self.fail(format!("Processing Error: {name} could not be read from the temporary folder ({e})."), "processing").await.map(|_| ()),
            }
        }

        // Step 6: create the folders and copy each file to its destinations.
        if !self.progress(RoutingRecordStatus::processing, 6, false).await? {
            return Ok(());
        }
        let released = released_folder_name();
        let roots = Roots { internal: &internal_root, external: client_root.as_ref(), released_folder_name: &released };
        // The year and type folders are looked up or made once and shared by every file of the request.
        let folders = FolderCache::default();
        let filed: Vec<Result<(), String>> = stream::iter(0..prepared.len()).map(|index| {
            let (roots, folders, prepared) = (&roots, &folders, &prepared);
            async move {
                let (doc, prep) = &prepared[index];
                let name = doc.document_name.clone().unwrap_or_default();
                let placed = place_prepared_cached(self.store.as_ref(), roots, prep, folders).await.map_err(|e| format!("{name}: {e}"))?;
                // From here the document's stored copy is the filed one (the temp copy is deleted in
                // step 7), so the case page can open it: the Internal quick view, else the External copy.
                let stored = placed.internal_quick_view.or(placed.internal_client_copy).or(placed.external);
                let mut input = doc_input(doc, Some(true)).map_err(|e| format!("{name}: {e}"))?;
                if let Some(f) = stored {
                    input.storage_file_id = Some(f.0);
                }
                self.da
                    .update_item::<InputTaxDocument, TaxDocumentProjection>(self.doc_et.clone(), doc_selection(), input, Some(self.user.clone()))
                    .await
                    .map_err(|e| format!("{name}: {e}"))?;
                Ok(())
            }
        })
        .buffer_unordered(FILES_AT_ONCE)
        .collect()
        .await;
        let failures: Vec<String> = filed.into_iter().filter_map(Result::err).collect();
        if !failures.is_empty() {
            return self
                .fail(format!("File Transfer Failed: {} could not be moved to its folders.", failures.join("; ")), "moving")
                .await
                .map(|_| ());
        }

        // Step 7: delete the temp folder, only now that every document is filed.
        if !self.progress(RoutingRecordStatus::processing, 7, false).await? {
            return Ok(());
        }
        match self.store.delete_folder(&temp).await {
            Ok(()) | Err(StoreError::NotFound(_)) => {}
            Err(e) => return self.fail(format!("Storage Folder Error: the temporary folder could not be deleted ({e})."), "cleanup").await.map(|_| ()),
        }
        if !self.progress(RoutingRecordStatus::processing, 7, true).await? {
            return Ok(());
        }

        // Step 8: the client email. Not built yet, so it is recorded as skipped rather than sent.
        if !self.progress(RoutingRecordStatus::processing, 8, false).await? {
            return Ok(());
        }
        if record.notification_flag.unwrap_or(false) {
            tracing::warn!(record_id = %self.record_id, "client notification requested but the email service is not built yet; nothing was sent");
        }

        // Step 9: done.
        self.progress(RoutingRecordStatus::resolved, 9, false).await?;
        tracing::info!(record_id = %self.record_id, "filing complete");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_filed_name_is_last_first_dash_original() {
        assert_eq!(target_file_name(Some("Moore"), Some("Andrea"), Some("Andrea Moore, DDS INC."), "Pega AI.pdf"), "Moore, Andrea - Pega AI.pdf");
    }

    #[test]
    fn it_falls_back_to_the_full_name_and_cleans_forbidden_characters() {
        assert_eq!(target_file_name(None, Some("Andrea"), Some("Andrea Moore"), "a.pdf"), "Andrea Moore - a.pdf");
        assert_eq!(target_file_name(Some("O/Brien"), Some("Pat"), None, "K1: 2025?.pdf"), "O_Brien, Pat - K1_ 2025_.pdf");
        assert_eq!(target_file_name(None, None, None, "x.pdf"), "Client - x.pdf");
    }

    #[test]
    fn the_submitted_list_is_checked() {
        let one = r#"[{"file_id":"1","file_name":"a.pdf","document_type":"K-1","year":2025}]"#;
        let docs = parse_submitted(one).unwrap();
        assert_eq!(docs.len(), 1);
        assert!(!docs[0].password_protected);

        assert!(parse_submitted("[]").is_err(), "needs at least one document");
        assert!(parse_submitted("not json").is_err());
        assert!(parse_submitted(r#"[{"file_id":"","file_name":"a.pdf","document_type":"K-1","year":2025}]"#).is_err());
        assert!(parse_submitted(r#"[{"file_id":"1","file_name":"a.pdf","document_type":"K-1","year":1800}]"#).is_err());
        let twice = r#"[{"file_id":"1","file_name":"a.pdf","document_type":"K-1","year":2025},{"file_id":"2","file_name":"A.PDF","document_type":"k-1","year":2025}]"#;
        assert!(parse_submitted(twice).is_err(), "the same document twice is refused");
    }

    #[test]
    fn the_temp_folder_goes_in_the_configured_folder_else_the_clients_internal_folder() {
        assert_eq!(choose_temp_parent(Some("84473772615"), Some("111")).as_deref(), Some("84473772615"));
        assert_eq!(choose_temp_parent(Some("  "), Some("111")).as_deref(), Some("111"));
        assert_eq!(choose_temp_parent(None, Some("111")).as_deref(), Some("111"));
        assert_eq!(choose_temp_parent(None, Some(" ")), None);
        assert_eq!(choose_temp_parent(None, None), None);
    }

    #[test]
    fn upload_limit_defaults_to_two_megabytes() {
        assert_eq!(upload_rules().max_bytes, 2 * 1024 * 1024);
    }
}
