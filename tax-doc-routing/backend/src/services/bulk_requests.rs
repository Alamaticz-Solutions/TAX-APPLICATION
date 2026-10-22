//! Bulk request creation: one Excel file of client details plus one ZIP holding a folder per
//! Client Key becomes one request per client.
//!
//! The flow is `preview` (reads and checks, creates nothing), optional corrections
//! (`update_document`, `remove`), then `confirm`, which creates every ready client's request one
//! after another in the background by calling the same code a single request uses
//! (`create_impl`, `upload_document`, `submit_request`). Filing in Box is therefore exactly the
//! existing pipeline. A failure for one client never stops the others.
//!
//! Batch state is held in memory (staged files can be large and are only needed until the batch
//! finishes). A restart loses an unfinished batch's preview and progress screen, but every request
//! already created stays in the database like any other.

use std::collections::{BTreeMap, HashMap};
use std::io::{Cursor, Read};
use std::sync::{Arc, Mutex, OnceLock};

use base64::Engine;
use calamine::{Data, Reader, Xlsx};
use chrono::{DateTime, Duration, Utc};
use futures::{stream, StreamExt};
use serde::Serialize;
use serde_json::{json, Value};

use crate::{
    handlers::tax_routing::tax_routing_record::create_impl,
    product_api::{entity_type_for_handler, DataAccess, EntityType, HandlerResult, UserAuth},
    schemas::tax_routing::{FieldOptionListProjection, FieldOptionValueProjection, InputTaxPayerRefData, InputTaxRoutingRecord, RoutingRecordStatus, TaxEntityListProjection, TaxPayerRefDataProjection, TaxRoutingRecordProjection},
    services::{
        authz, document_routing,
        routing_record_actions::{full_record_selection, load_record},
    },
};

const MAX_EXCEL_BYTES: usize = 5 * 1024 * 1024;
const MAX_ZIP_BYTES: usize = 400 * 1024 * 1024;
const MAX_ZIP_ENTRIES: usize = 5000;
const MAX_ZIP_UNPACKED: u64 = 512 * 1024 * 1024;
const MAX_CLIENTS: usize = 500;
const KEEP_BATCHES_FOR: Duration = Duration::hours(24);

// ---------------------------------------------------------------------------------------------
// Batch state
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Serialize)]
struct Doc {
    file_name: String,
    size: usize,
    document_type: String,
    year: Option<i32>,
    entity_name: String,
    entity_type: String,
    entity_number: String,
    password_protected: bool,
    /// `filename` (read from the file name), `manual` (changed in the preview) or `none`.
    source: &'static str,
    /// `ready` or `error`.
    status: &'static str,
    errors: Vec<String>,
    #[serde(skip)]
    bytes: Arc<Vec<u8>>,
}

#[derive(Clone, Serialize, Default)]
struct ClientDetails {
    first_name: String,
    last_name: String,
    full_name: String,
    personal_email: String,
    pds_email: String,
    additional_email: String,
    office_location: String,
    internal_folder: String,
    client_folder: String,
    password_on_file: bool,
    notify: bool,
}

#[derive(Clone, Serialize)]
struct Item {
    client_key: String,
    client: ClientDetails,
    #[serde(skip)]
    password: String,
    /// `existing` (found in the client directory; its values are used) or `new` (created from the file).
    source: &'static str,
    #[serde(skip)]
    existing_id: Option<String>,
    /// Where the file disagrees with the directory (the directory wins).
    differences: Vec<String>,
    /// Everything that blocks this client (the two lists below, joined for the screen).
    errors: Vec<String>,
    /// Problems with the client's place in the files: no ZIP folder, no Excel row, inactive, and so on.
    #[serde(skip)]
    structural: Vec<String>,
    /// Problems with the client's details: a missing field, or a folder id that cannot be used.
    #[serde(skip)]
    client_errs: Vec<String>,
    docs: Vec<Doc>,
    /// `ready`, `error`, `queued`, `processing`, `created`, `failed` or `skipped`.
    status: &'static str,
    record_id: Option<String>,
    case_number: Option<i32>,
    message: Option<String>,
}

impl Item {
    fn recompute(&mut self) {
        self.errors = self.structural.iter().chain(self.client_errs.iter()).cloned().collect();
        if matches!(self.status, "ready" | "error") {
            let clean = self.errors.is_empty() && !self.docs.is_empty() && self.docs.iter().all(|d| d.status == "ready");
            self.status = if clean { "ready" } else { "error" };
        }
    }
}

#[derive(Clone, Serialize)]
struct Batch {
    id: String,
    #[serde(skip)]
    owner: String,
    file_name: String,
    created_at: DateTime<Utc>,
    /// `checking`, `check_failed`, `staged`, `processing`, `completed`, `completed_with_errors` or `cancelled`.
    status: &'static str,
    /// Why the files could not be checked (`check_failed`).
    error: Option<String>,
    /// What the checking is doing now, and how far it has got.
    progress_label: String,
    progress_done: usize,
    progress_total: usize,
    /// Things worth telling the user that belong to no client (ignored files and the like).
    notes: Vec<String>,
    items: Vec<Item>,
}

fn store() -> &'static Mutex<HashMap<String, Batch>> {
    static STORE: OnceLock<Mutex<HashMap<String, Batch>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn with_batch<T>(user: &UserAuth, batch_id: &str, f: impl FnOnce(&mut Batch) -> HandlerResult<T>) -> HandlerResult<T> {
    let mut all = store().lock().map_err(|_| anyhow::anyhow!("bulk state is unavailable"))?;
    let batch = all
        .get_mut(batch_id)
        .filter(|b| b.owner == user.user_name)
        .ok_or_else(|| anyhow::anyhow!("That bulk upload was not found. It may have expired; upload the files again."))?;
    f(batch)
}

fn require_user(user: Option<UserAuth>) -> HandlerResult<UserAuth> {
    user.ok_or_else(|| anyhow::anyhow!("not authenticated"))
}

fn require_create(data_access: &Arc<DataAccess>, user: &UserAuth) -> HandlerResult<crate::services::authz::Principal> {
    let principal = data_access.app_config.authz().resolve(&user.tenant_id, &user.user_name);
    authz::require(&principal, "routing_record.create")?;
    Ok(principal)
}

// ---------------------------------------------------------------------------------------------
// Excel
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Default, Clone, PartialEq)]
struct ClientRow {
    key: String,
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
}

fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(|c| c.to_lowercase()).collect()
}

fn cell_text(c: &Data) -> String {
    match c {
        Data::String(s) => s.trim().to_string(),
        Data::Int(i) => i.to_string(),
        // Box folder ids are long digit strings that Excel stores as numbers.
        Data::Float(f) if f.fract() == 0.0 && f.abs() < 1e15 => format!("{}", *f as i64),
        Data::Float(f) => f.to_string(),
        Data::Bool(b) => b.to_string(),
        Data::Empty => String::new(),
        other => other.to_string().trim().to_string(),
    }
}

fn truthy(s: &str) -> bool {
    matches!(s.trim().to_ascii_lowercase().as_str(), "yes" | "y" | "true" | "1")
}

/// Turns the sheet's text rows (first row = headers) into client rows.
fn rows_from_text(rows: &[Vec<String>]) -> HandlerResult<Vec<ClientRow>> {
    let header = rows.first().ok_or_else(|| anyhow::anyhow!("The Excel file is empty."))?;
    let col = |names: &[&str]| header.iter().position(|h| names.contains(&norm(h).as_str()));
    let key_col = col(&["clientkey", "key"]).ok_or_else(|| anyhow::anyhow!("The Excel file needs a \"Client Key\" column."))?;
    let c_first = col(&["firstname"]);
    let c_last = col(&["lastname"]);
    let c_full = col(&["clientname", "fullname", "name"]);
    let c_personal = col(&["personalemail", "clientpersonalemail"]);
    let c_pds = col(&["pdsemail", "clientpdsemail"]);
    let c_extra = col(&["additionalemail"]);
    let c_office = col(&["officelocation", "office"]);
    let c_internal = col(&["internalfolderid", "internalfolder"]);
    let c_client = col(&["clientfolderid", "clientfolder", "externalfolderid", "externalfolder"]);
    let c_pass = col(&["readwritepassword", "password"]);
    let c_notify = col(&["sendnotification", "notifyclient", "notification"]);

    let get = |row: &Vec<String>, c: Option<usize>| c.and_then(|i| row.get(i)).map(|s| s.trim().to_string()).unwrap_or_default();
    let mut out: Vec<ClientRow> = Vec::new();
    for row in rows.iter().skip(1) {
        let key = get(row, Some(key_col));
        if key.is_empty() && row.iter().all(|c| c.trim().is_empty()) {
            continue;
        }
        let first = get(row, c_first);
        let last = get(row, c_last);
        let mut full = get(row, c_full);
        if full.is_empty() {
            full = format!("{first} {last}").trim().to_string();
        }
        out.push(ClientRow {
            key,
            first_name: first,
            last_name: last,
            full_name: full,
            personal_email: get(row, c_personal),
            pds_email: get(row, c_pds),
            additional_email: get(row, c_extra),
            office_location: get(row, c_office),
            internal_folder: get(row, c_internal),
            client_folder: get(row, c_client),
            password: get(row, c_pass),
            notify: truthy(&get(row, c_notify)),
        });
    }
    Ok(out)
}

fn read_excel(bytes: &[u8]) -> HandlerResult<Vec<ClientRow>> {
    let mut book: Xlsx<_> = calamine::open_workbook_from_rs(Cursor::new(bytes.to_vec()))
        .map_err(|_| anyhow::anyhow!("The Excel file could not be read. Use a .xlsx file."))?;
    let range = book
        .worksheet_range_at(0)
        .ok_or_else(|| anyhow::anyhow!("The Excel file has no sheets."))?
        .map_err(|_| anyhow::anyhow!("The first sheet of the Excel file could not be read."))?;
    let rows: Vec<Vec<String>> = range.rows().map(|r| r.iter().map(cell_text).collect()).collect();
    rows_from_text(&rows)
}

// ---------------------------------------------------------------------------------------------
// ZIP
// ---------------------------------------------------------------------------------------------

struct ZipFile {
    name: String,
    bytes: Arc<Vec<u8>>,
    problem: Option<String>,
}

#[derive(Default)]
struct ZipFolder {
    key: String,
    files: Vec<ZipFile>,
}

#[derive(Default)]
struct ZipScan {
    /// Keyed by the lower-cased folder name.
    folders: BTreeMap<String, ZipFolder>,
    notes: Vec<String>,
}

fn read_zip(bytes: &[u8], max_file: usize) -> HandlerResult<ZipScan> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| anyhow::anyhow!("The ZIP file could not be read."))?;
    if archive.len() > MAX_ZIP_ENTRIES {
        return Err(anyhow::anyhow!("The ZIP holds more than {MAX_ZIP_ENTRIES} entries."));
    }
    let mut unpacked: u64 = 0;
    for i in 0..archive.len() {
        unpacked = unpacked.saturating_add(archive.by_index_raw(i).map(|f| f.size()).unwrap_or(0));
    }
    if unpacked > MAX_ZIP_UNPACKED {
        return Err(anyhow::anyhow!("The ZIP is too large once unpacked."));
    }
    let mut scan = ZipScan::default();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|_| anyhow::anyhow!("The ZIP file could not be read."))?;
        if entry.is_dir() {
            continue;
        }
        // Reject anything that tries to leave its folder.
        let path = entry
            .enclosed_name()
            .ok_or_else(|| anyhow::anyhow!("The ZIP contains an unsafe path ({}).", entry.name()))?;
        let parts: Vec<String> = path.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
        let file_name = parts.last().cloned().unwrap_or_default();
        if parts.first().map_or(false, |p| p == "__MACOSX") || file_name.starts_with('.') || file_name.eq_ignore_ascii_case("Thumbs.db") {
            continue;
        }
        match parts.len() {
            0 => continue,
            1 => {
                scan.notes.push(format!("\"{file_name}\" is at the top level of the ZIP, not inside a client folder, so it was ignored."));
            }
            n => {
                let key = parts[0].clone();
                let folder = scan.folders.entry(key.to_lowercase()).or_insert_with(|| ZipFolder { key: key.clone(), files: Vec::new() });
                let display = parts[1..].join("/");
                if n > 2 {
                    folder.files.push(ZipFile { name: display, bytes: Arc::new(Vec::new()), problem: Some("Subfolders inside a client folder are not supported. Move the file up one level.".into()) });
                } else if entry.size() as usize > max_file {
                    folder.files.push(ZipFile { name: display, bytes: Arc::new(Vec::new()), problem: Some(format!("The file is larger than {} MB.", max_file / (1024 * 1024))) });
                } else {
                    let mut buf = Vec::with_capacity(entry.size() as usize);
                    entry.by_ref().take(max_file as u64 + 1).read_to_end(&mut buf).map_err(|_| anyhow::anyhow!("\"{display}\" could not be read from the ZIP."))?;
                    folder.files.push(ZipFile { name: display, bytes: Arc::new(buf), problem: None });
                }
            }
        }
    }
    Ok(scan)
}

// ---------------------------------------------------------------------------------------------
// Classifying a file from its name
// ---------------------------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
struct Classified {
    document_type: String,
    year: i32,
    entity_hint: String,
}

/// Reads `<Type>_<Year>[_<Entity>].pdf`: the type is matched (ignoring case, spaces and hyphens)
/// against the configured document types; the longest matching type wins.
/// Is this a year the business accepts? The configured `document_year` list when there is one (the
/// same list the New Request screens offer); without a list, any four-digit year.
fn year_allowed(year: i32, years: &[i32]) -> bool {
    if years.is_empty() {
        (1000..=9999).contains(&year)
    } else {
        years.contains(&year)
    }
}

/// The years set up in the `document_year` option list (administrators maintain it; nothing is built in).
async fn configured_years(data_access: &Arc<DataAccess>, user: &UserAuth) -> HandlerResult<Vec<i32>> {
    let f = |n: &str| json!({ "name": n, "selection_set": [] });
    let list_et = entity_type_for_handler(data_access, "tax_routing", "FieldOptionList")?;
    let lists = data_access
        .query_items::<FieldOptionListProjection>(
            list_et,
            json!({ "name": "field_option_list", "selection_set": [f("id"), f("name")] }),
            Some(json!({ "name": { "_eq": "document_year" } })),
            None,
            0,
            5,
            None,
            Some(user.clone()),
        )
        .await?;
    let Some(list_id) = lists.items.into_iter().find_map(|l| l.id) else { return Ok(Vec::new()) };
    let value_et = entity_type_for_handler(data_access, "tax_routing", "FieldOptionValue")?;
    let values = data_access
        .query_items::<FieldOptionValueProjection>(
            value_et,
            json!({ "name": "field_option_value", "selection_set": [f("code"), f("is_active")] }),
            Some(json!({ "option_list_id": { "_eq": list_id } })),
            None,
            0,
            250,
            None,
            Some(user.clone()),
        )
        .await?;
    Ok(values.items.into_iter().filter(|v| v.is_active.unwrap_or(true)).filter_map(|v| v.code?.trim().parse::<i32>().ok()).collect())
}

fn classify_name(file_name: &str, types: &[(String, bool)], years: &[i32]) -> Option<Classified> {
    let stem = file_name.rsplit_once('.').map(|(s, _)| s).unwrap_or(file_name);
    let parts: Vec<&str> = stem.split('_').collect();
    let mut best: Option<(usize, &str)> = None;
    for take in 1..parts.len() {
        let candidate = norm(&parts[..take].join(" "));
        if let Some((t, _)) = types.iter().find(|(t, _)| norm(t) == candidate) {
            best = Some((take, t.as_str()));
        }
    }
    let (take, doc_type) = best?;
    let year: i32 = parts.get(take)?.trim().parse().ok().filter(|y| year_allowed(*y, years))?;
    let entity_hint = parts[take + 1..].join(" ").replace('-', " ").trim().to_string();
    Some(Classified { document_type: doc_type.to_string(), year, entity_hint })
}

type EntityRow = (String, String, String); // (type, name, number)

/// Matches a name from a file name ("Oakwood Medical Partners") to the entity directory.
fn find_entity<'a>(hint: &str, entities: &'a [EntityRow]) -> Option<&'a EntityRow> {
    let h = norm(hint);
    if h.is_empty() {
        return None;
    }
    if let Some(exact) = entities.iter().find(|(_, name, _)| norm(name) == h) {
        return Some(exact);
    }
    // "Oakwood Medical Partners" for "Oakwood Medical Partners LLC": accept a unique prefix.
    let mut starts = entities.iter().filter(|(_, name, _)| h.len() >= 4 && norm(name).starts_with(&h));
    match (starts.next(), starts.next()) {
        (Some(only), None) => Some(only),
        _ => None,
    }
}

fn build_doc(file: &ZipFile, types: &[(String, bool)], entities: &[EntityRow], years: &[i32], client_has_password: bool) -> Doc {
    let mut doc = Doc {
        file_name: file.name.clone(),
        size: file.bytes.len(),
        document_type: String::new(),
        year: None,
        entity_name: String::new(),
        entity_type: String::new(),
        entity_number: String::new(),
        password_protected: false,
        source: "none",
        status: "error",
        errors: Vec::new(),
        bytes: file.bytes.clone(),
    };
    if let Some(problem) = &file.problem {
        doc.errors.push(problem.clone());
        return doc;
    }
    if !file.name.to_ascii_lowercase().ends_with(".pdf") {
        doc.errors.push("Only PDF files are accepted.".into());
        return doc;
    }
    if !file.bytes.starts_with(b"%PDF") {
        doc.errors.push("The file is not a valid PDF.".into());
        return doc;
    }
    match classify_name(&file.name, types, years) {
        Some(c) => {
            let supports = types.iter().find(|(t, _)| *t == c.document_type).map_or(false, |(_, s)| *s);
            doc.document_type = c.document_type;
            doc.year = Some(c.year);
            match find_entity(&c.entity_hint, entities) {
                Some((t, n, num)) => {
                    doc.entity_type = t.clone();
                    doc.entity_name = n.clone();
                    doc.entity_number = num.clone();
                }
                None => doc.entity_name = c.entity_hint,
            }
            doc.password_protected = client_has_password && supports;
            doc.source = "filename";
            doc.status = "ready";
        }
        None => doc
            .errors
            .push("The document type and year could not be read from the file name. Name it like K-1_2025_EntityName.pdf, or set them here.".into()),
    }
    doc
}

// ---------------------------------------------------------------------------------------------
// Client lookup
// ---------------------------------------------------------------------------------------------

fn client_selection() -> Value {
    let f = |n: &str| json!({ "name": n, "selection_set": [] });
    json!({
        "name": "tax_payer_ref_data",
        "selection_set": [
            f("id"), f("client_key"), f("first_name"), f("last_name"), f("full_name"), f("personal_email"), f("pds_email"),
            f("office_location"), f("folder_name"), f("internal_folder_path"), f("is_active"), f("read_write_password"), f("version"),
        ]
    })
}

async fn find_client(data_access: &Arc<DataAccess>, user: &UserAuth, key: &str) -> HandlerResult<Option<TaxPayerRefDataProjection>> {
    let et = entity_type_for_handler(data_access, "tax_routing", "TaxPayerRefData")?;
    let found = data_access
        .query_items::<TaxPayerRefDataProjection>(et, client_selection(), Some(json!({ "client_key": { "_eq": key } })), None, 0, 2, None, Some(user.clone()))
        .await?;
    Ok(found.items.into_iter().next())
}

// ---------------------------------------------------------------------------------------------
// Preview
// ---------------------------------------------------------------------------------------------

fn decode(label: &str, b64: &str, max: usize) -> HandlerResult<Vec<u8>> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|_| anyhow::anyhow!("The {label} could not be read."))?;
    if bytes.is_empty() {
        return Err(anyhow::anyhow!("The {label} is empty."));
    }
    if bytes.len() > max {
        return Err(anyhow::anyhow!("The {label} is too large."));
    }
    Ok(bytes)
}

fn new_item(key: &str) -> Item {
    Item {
        client_key: key.to_string(),
        client: ClientDetails::default(),
        password: String::new(),
        source: "new",
        existing_id: None,
        differences: Vec::new(),
        errors: Vec::new(),
        structural: Vec::new(),
        client_errs: Vec::new(),
        docs: Vec::new(),
        status: "error",
        record_id: None,
        case_number: None,
        message: None,
    }
}

/// What a look at a folder id found.
#[derive(Clone)]
enum FolderState {
    Ok,
    Unusable,
    Missing,
    Failed(String),
}

async fn check_folder(data_access: &Arc<DataAccess>, raw: &str) -> FolderState {
    let store = data_access.app_config.document_store().clone();
    match store.parse_folder(raw) {
        Err(_) => FolderState::Unusable,
        Ok(folder) => match store.folder_exists(&folder).await {
            Ok(true) => FolderState::Ok,
            Ok(false) => FolderState::Missing,
            Err(e) => FolderState::Failed(e.to_string()),
        },
    }
}

fn folder_message(what: &str, state: &FolderState) -> Option<String> {
    match state {
        FolderState::Ok => None,
        FolderState::Unusable => Some(format!("The {what} is not a usable storage folder id.")),
        FolderState::Missing => Some(format!("The {what} was not found in document storage.")),
        FolderState::Failed(e) => Some(format!("The {what} could not be checked: {e}")),
    }
}

/// How many folder checks run at the same time while a bulk upload is being checked.
const CHECKS_AT_ONCE: usize = 8;

async fn entity_directory(data_access: &Arc<DataAccess>, user: &UserAuth) -> HandlerResult<Vec<EntityRow>> {
    const PAGE: i32 = 250;
    let et = entity_type_for_handler(data_access, "tax_routing", "TaxEntityList")?;
    let f = |n: &str| json!({ "name": n, "selection_set": [] });
    let selection = json!({ "name": "tax_entity_list", "selection_set": [f("entity_type"), f("entity_name"), f("oracle_id"), f("is_active")] });
    let mut out: Vec<EntityRow> = Vec::new();
    let mut skip = 0;
    loop {
        let page = data_access
            .query_items::<TaxEntityListProjection>(et.clone(), selection.clone(), None, None, skip, PAGE, None, Some(user.clone()))
            .await?;
        let got = page.items.len() as i32;
        out.extend(
            page.items
                .into_iter()
                .filter(|e| e.is_active.unwrap_or(true))
                .filter_map(|e| Some((e.entity_type?, e.entity_name?, e.oracle_id.unwrap_or_default()))),
        );
        if got < PAGE {
            return Ok(out);
        }
        skip += PAGE;
    }
}

/// The client directory rows for all these keys, in as few queries as possible (not one per client).
async fn find_clients(data_access: &Arc<DataAccess>, user: &UserAuth, keys: &[String]) -> HandlerResult<HashMap<String, TaxPayerRefDataProjection>> {
    const CHUNK: usize = 200;
    let et = entity_type_for_handler(data_access, "tax_routing", "TaxPayerRefData")?;
    let mut found = HashMap::new();
    for chunk in keys.chunks(CHUNK) {
        let page = data_access
            .query_items::<TaxPayerRefDataProjection>(et.clone(), client_selection(), Some(json!({ "client_key": { "_in": chunk } })), None, 0, 250, None, Some(user.clone()))
            .await?;
        for client in page.items {
            if let Some(key) = client.client_key.clone() {
                found.insert(key.to_lowercase(), client);
            }
        }
    }
    Ok(found)
}

fn required_field_problems(c: &ClientDetails) -> Vec<String> {
    let mut out = Vec::new();
    for (label, value) in [("First Name", &c.first_name), ("Last Name", &c.last_name), ("Internal Folder ID", &c.internal_folder), ("Client Folder ID", &c.client_folder)] {
        if value.trim().is_empty() {
            out.push(format!("{label} is missing."));
        }
    }
    out
}

/// Checks a client's details: the fields a request needs, and that both folders can be used.
async fn client_problems(data_access: &Arc<DataAccess>, item: &Item) -> Vec<String> {
    let c = &item.client;
    let mut out = required_field_problems(c);
    if out.is_empty() {
        let (internal, client) = tokio::join!(check_folder(data_access, &c.internal_folder), check_folder(data_access, &c.client_folder));
        out.extend(folder_message("client's Internal folder", &internal));
        out.extend(folder_message("client's Client folder", &client));
    }
    out
}

/// The file's value when it has one, otherwise the directory's.
fn pick(file: &str, directory: &str) -> String {
    if file.trim().is_empty() {
        directory.trim().to_string()
    } else {
        file.trim().to_string()
    }
}

fn note_difference(label: &str, file: &str, directory: &str, out: &mut Vec<String>) {
    if !file.trim().is_empty() && !directory.trim().is_empty() && !file.trim().eq_ignore_ascii_case(directory.trim()) {
        out.push(format!("{label}: the file has \"{}\", the client directory has \"{}\". The file value is used.", file.trim(), directory.trim()));
    }
}

fn max_clients() -> usize {
    std::env::var("BULK_MAX_CLIENTS").ok().and_then(|v| v.trim().parse::<usize>().ok()).filter(|n| *n > 0).unwrap_or(MAX_CLIENTS)
}

fn set_progress(user: &UserAuth, batch_id: &str, label: &str, done: usize, total: usize) {
    let _ = with_batch(user, batch_id, |b| {
        b.progress_label = label.to_string();
        b.progress_done = done;
        b.progress_total = total;
        Ok(())
    });
}

/// Starts checking the two files and returns at once. The checking (reading the files, finding each
/// client, looking at every client's folders in storage) can take a while for a large upload, so it
/// runs in the background; the screen shows its progress by asking `status`.
pub async fn preview(user: Option<UserAuth>, data_access: &Arc<DataAccess>, file_name: String, excel_base64: String, zip_base64: String) -> HandlerResult<Value> {
    let user = require_user(user)?;
    let principal = require_create(data_access, &user)?;
    let may_add_clients = principal.has_permission("tax_payer_ref_data.create");
    let excel = decode("Excel file", &excel_base64, MAX_EXCEL_BYTES)?;
    let zip_bytes = decode("ZIP file", &zip_base64, MAX_ZIP_BYTES)?;

    let batch = Batch {
        id: uuid::Uuid::new_v4().to_string(),
        owner: user.user_name.clone(),
        file_name,
        created_at: Utc::now(),
        status: "checking",
        notes: Vec::new(),
        error: None,
        progress_label: "Reading the files".to_string(),
        progress_done: 0,
        progress_total: 0,
        items: Vec::new(),
    };
    let body = serde_json::to_value(&batch)?;
    let id = batch.id.clone();
    {
        let mut all = store().lock().map_err(|_| anyhow::anyhow!("bulk state is unavailable"))?;
        let cutoff = Utc::now() - KEEP_BATCHES_FOR;
        all.retain(|_, b| b.created_at > cutoff);
        all.insert(batch.id.clone(), batch);
    }
    let (da, u) = (data_access.clone(), user.clone());
    tokio::spawn(async move {
        let started = std::time::Instant::now();
        let outcome = build_items(&da, &u, &id, may_add_clients, excel, zip_bytes).await;
        let _ = with_batch(&u, &id, |b| {
            match outcome {
                Ok((items, notes)) => {
                    tracing::info!(batch = %id, clients = items.len(), ms = started.elapsed().as_millis() as u64, "bulk files checked");
                    b.items = items;
                    b.notes = notes;
                    b.status = "staged";
                }
                Err(e) => {
                    b.error = Some(e.to_string());
                    b.status = "check_failed";
                }
            }
            Ok(())
        });
    });
    Ok(body)
}

async fn build_items(
    data_access: &Arc<DataAccess>,
    user: &UserAuth,
    batch_id: &str,
    may_add_clients: bool,
    excel: Vec<u8>,
    zip_bytes: Vec<u8>,
) -> HandlerResult<(Vec<Item>, Vec<String>)> {
    // Reading the two files is CPU work: keep it off the threads that serve requests.
    let max_file = document_routing::upload_rules().max_bytes;
    let (rows, mut scan) = tokio::task::spawn_blocking(move || -> HandlerResult<(Vec<ClientRow>, ZipScan)> { Ok((read_excel(&excel)?, read_zip(&zip_bytes, max_file)?)) })
        .await
        .map_err(|_| anyhow::anyhow!("The files could not be read."))??;
    if rows.is_empty() {
        return Err(anyhow::anyhow!("The Excel file has no client rows."));
    }
    let limit = max_clients();
    if rows.len() > limit {
        return Err(anyhow::anyhow!("The Excel file has more than {limit} clients. Split it into smaller batches."));
    }

    set_progress(user, batch_id, "Finding the clients", 0, 0);
    let keys: Vec<String> = rows.iter().map(|r| r.key.trim().to_string()).filter(|k| !k.is_empty()).collect();
    // The lookups do not depend on each other: the types, the entities and all the clients at once.
    let (types, entities, directory, years) = tokio::join!(
        document_routing::document_types(data_access, user),
        entity_directory(data_access, user),
        find_clients(data_access, user, &keys),
        configured_years(data_access, user)
    );
    let types = types?;
    let years = years.unwrap_or_default();
    let directory = directory?;
    let entities = match entities {
        Ok(list) => list,
        Err(e) => {
            tracing::warn!(error = %e, "bulk preview could not read the entity directory; entity names will not be matched");
            Vec::new()
        }
    };

    let mut items: Vec<Item> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for row in &rows {
        let mut item = new_item(&row.key);
        if row.key.is_empty() {
            item.structural.push("This row has no Client Key.".into());
            item.recompute();
            items.push(item);
            continue;
        }
        if !seen.insert(row.key.to_lowercase()) {
            item.structural.push("The Client Key appears more than once in the Excel file.".into());
            item.recompute();
            items.push(item);
            continue;
        }
        // The details are the Excel file's, as given; a blank cell falls back to the client directory.
        let dir = directory.get(&row.key.to_lowercase());
        let s = |v: &Option<String>| v.clone().unwrap_or_default();
        let (d_first, d_last, d_full) = (dir.map(|c| s(&c.first_name)).unwrap_or_default(), dir.map(|c| s(&c.last_name)).unwrap_or_default(), dir.map(|c| s(&c.full_name)).unwrap_or_default());
        let (d_personal, d_pds, d_office) = (dir.map(|c| s(&c.personal_email)).unwrap_or_default(), dir.map(|c| s(&c.pds_email)).unwrap_or_default(), dir.map(|c| s(&c.office_location)).unwrap_or_default());
        let (d_internal, d_client, d_pass) = (dir.map(|c| s(&c.internal_folder_path)).unwrap_or_default(), dir.map(|c| s(&c.folder_name)).unwrap_or_default(), dir.map(|c| s(&c.read_write_password)).unwrap_or_default());
        if let Some(c) = dir {
            item.source = "existing";
            item.existing_id = c.id.clone();
            if !c.is_active.unwrap_or(true) {
                item.structural.push("This client is inactive.".into());
            }
            note_difference("First name", &row.first_name, &d_first, &mut item.differences);
            note_difference("Last name", &row.last_name, &d_last, &mut item.differences);
            note_difference("Personal email", &row.personal_email, &d_personal, &mut item.differences);
            note_difference("PDS email", &row.pds_email, &d_pds, &mut item.differences);
            note_difference("Internal folder id", &row.internal_folder, &d_internal, &mut item.differences);
            note_difference("Client folder id", &row.client_folder, &d_client, &mut item.differences);
        } else {
            item.source = "new";
            if !may_add_clients {
                item.structural.push("This client is not in the client directory, and only a Tax Admin can add new clients. Ask an admin to add it, or have an admin run this upload.".into());
            }
        }
        item.password = pick(&row.password, &d_pass);
        item.client = ClientDetails {
            first_name: pick(&row.first_name, &d_first),
            last_name: pick(&row.last_name, &d_last),
            full_name: pick(&row.full_name, &d_full),
            personal_email: pick(&row.personal_email, &d_personal),
            pds_email: pick(&row.pds_email, &d_pds),
            additional_email: row.additional_email.clone(),
            office_location: pick(&row.office_location, &d_office),
            internal_folder: pick(&row.internal_folder, &d_internal),
            client_folder: pick(&row.client_folder, &d_client),
            password_on_file: !item.password.is_empty(),
            notify: row.notify,
        };
        if item.client.full_name.is_empty() {
            item.client.full_name = format!("{} {}", item.client.first_name, item.client.last_name).trim().to_string();
        }
        item.client_errs = required_field_problems(&item.client);

        match scan.folders.remove(&row.key.to_lowercase()) {
            Some(folder) => {
                let has_pw = item.client.password_on_file;
                item.docs = folder.files.iter().map(|f| build_doc(f, &types, &entities, &years, has_pw)).collect();
                if item.docs.is_empty() {
                    item.structural.push("The ZIP folder for this client has no files.".into());
                }
            }
            None => item.structural.push(format!("There is no folder named \"{}\" in the ZIP.", row.key)),
        }
        items.push(item);
    }
    for (_, folder) in scan.folders {
        let mut item = new_item(&folder.key);
        item.structural.push("This ZIP folder has no matching row in the Excel file.".into());
        item.docs = folder.files.iter().map(|f| build_doc(f, &types, &entities, &years, false)).collect();
        items.push(item);
    }

    // Every distinct folder is looked at once (clients often share folders), several at a time.
    let mut wanted: Vec<String> = Vec::new();
    for item in &items {
        if item.client_errs.is_empty() && item.structural.iter().all(|e| !e.starts_with("There is no folder") && !e.starts_with("This row")) && !item.client.internal_folder.is_empty() {
            for raw in [&item.client.internal_folder, &item.client.client_folder] {
                if !raw.is_empty() && !wanted.contains(raw) {
                    wanted.push(raw.clone());
                }
            }
        }
    }
    let total = wanted.len();
    set_progress(user, batch_id, "Checking the clients' folders", 0, total);
    let mut states: HashMap<String, FolderState> = HashMap::new();
    let mut checks = stream::iter(wanted.into_iter().map(|raw| {
        let da = data_access.clone();
        async move {
            let state = check_folder(&da, &raw).await;
            (raw, state)
        }
    }))
    .buffer_unordered(CHECKS_AT_ONCE);
    while let Some((raw, state)) = checks.next().await {
        states.insert(raw, state);
        set_progress(user, batch_id, "Checking the clients' folders", states.len(), total);
    }
    for item in &mut items {
        if item.client_errs.is_empty() {
            if let Some(state) = states.get(&item.client.internal_folder) {
                item.client_errs.extend(folder_message("client's Internal folder", state));
            }
            if let Some(state) = states.get(&item.client.client_folder) {
                item.client_errs.extend(folder_message("client's Client folder", state));
            }
        }
        item.recompute();
    }
    Ok((items, scan.notes))
}

// ---------------------------------------------------------------------------------------------
// Corrections
// ---------------------------------------------------------------------------------------------

pub struct ClientEdit {
    pub first_name: String,
    pub last_name: String,
    pub full_name: String,
    pub personal_email: String,
    pub pds_email: String,
    pub additional_email: String,
    pub office_location: String,
    pub internal_folder: String,
    pub client_folder: String,
    pub password: String,
    pub notify: bool,
}

pub struct DocumentEdit {
    pub document_type: String,
    pub year: i32,
    pub entity_type: String,
    pub entity_name: String,
    pub entity_number: String,
    pub password_protected: bool,
}

/// Changes the client's details for this upload (the directory record itself is not changed). An
/// empty `password` keeps the current one.
pub async fn update_client(user: Option<UserAuth>, data_access: &Arc<DataAccess>, batch_id: String, client_key: String, details: ClientEdit) -> HandlerResult<Value> {
    let user = require_user(user)?;
    require_create(data_access, &user)?;
    let mut item = with_batch(&user, &batch_id, |batch| {
        require_staged(batch)?;
        batch.items.iter().find(|i| i.client_key == client_key).cloned().ok_or_else(|| anyhow::anyhow!("That client is not in this bulk upload."))
    })?;
    let c = &mut item.client;
    c.first_name = details.first_name.trim().to_string();
    c.last_name = details.last_name.trim().to_string();
    c.full_name = if details.full_name.trim().is_empty() { format!("{} {}", c.first_name, c.last_name).trim().to_string() } else { details.full_name.trim().to_string() };
    c.personal_email = details.personal_email.trim().to_string();
    c.pds_email = details.pds_email.trim().to_string();
    c.additional_email = details.additional_email.trim().to_string();
    c.office_location = details.office_location.trim().to_string();
    c.internal_folder = details.internal_folder.trim().to_string();
    c.client_folder = details.client_folder.trim().to_string();
    c.notify = details.notify;
    if !details.password.is_empty() {
        item.password = details.password;
    }
    item.client.password_on_file = !item.password.is_empty();
    if !item.client.password_on_file {
        for d in &mut item.docs {
            d.password_protected = false;
        }
    }
    item.client_errs = client_problems(data_access, &item).await;
    with_batch(&user, &batch_id, |batch| {
        require_staged(batch)?;
        if let Some(slot) = batch.items.iter_mut().find(|i| i.client_key == client_key) {
            slot.client = item.client.clone();
            slot.password = item.password.clone();
            slot.client_errs = item.client_errs.clone();
            slot.docs = item.docs.clone();
            slot.recompute();
        }
        Ok(serde_json::to_value(&*batch)?)
    })
}

pub async fn update_document(user: Option<UserAuth>, data_access: &Arc<DataAccess>, batch_id: String, client_key: String, file_name: String, edit: DocumentEdit) -> HandlerResult<Value> {
    let user = require_user(user)?;
    require_create(data_access, &user)?;
    let types = document_routing::document_types(data_access, &user).await?;
    let (canonical, supports) = types
        .iter()
        .find(|(t, _)| norm(t) == norm(&edit.document_type))
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("\"{}\" is not a document type that can be filed.", edit.document_type))?;
    let years = configured_years(data_access, &user).await.unwrap_or_default();
    if !year_allowed(edit.year, &years) {
        return Err(anyhow::anyhow!("{} is not one of the document years that can be filed.", edit.year));
    }
    with_batch(&user, &batch_id, |batch| {
        require_staged(batch)?;
        let item = batch
            .items
            .iter_mut()
            .find(|i| i.client_key == client_key)
            .ok_or_else(|| anyhow::anyhow!("That client is not in this bulk upload."))?;
        let can_protect = item.client.password_on_file;
        let doc = item.docs.iter_mut().find(|d| d.file_name == file_name).ok_or_else(|| anyhow::anyhow!("That file is not in this bulk upload."))?;
        let readable = doc.bytes.starts_with(b"%PDF") && doc.file_name.to_ascii_lowercase().ends_with(".pdf") && doc.errors.iter().all(|e| e.contains("document type"));
        if !readable {
            return Err(anyhow::anyhow!("This file cannot be filed; remove it instead."));
        }
        doc.document_type = canonical;
        doc.year = Some(edit.year);
        doc.entity_type = edit.entity_type.trim().to_string();
        doc.entity_name = edit.entity_name.trim().to_string();
        doc.entity_number = edit.entity_number.trim().to_string();
        doc.password_protected = edit.password_protected && supports && can_protect;
        doc.source = "manual";
        doc.status = "ready";
        doc.errors.clear();
        item.recompute();
        Ok(serde_json::to_value(&*batch)?)
    })
}

/// One staged file, for the preview and download buttons.
pub fn get_document(user: Option<UserAuth>, batch_id: String, client_key: String, file_name: String) -> HandlerResult<Value> {
    let user = require_user(user)?;
    with_batch(&user, &batch_id, |batch| {
        let doc = batch
            .items
            .iter()
            .find(|i| i.client_key == client_key)
            .and_then(|i| i.docs.iter().find(|d| d.file_name == file_name))
            .ok_or_else(|| anyhow::anyhow!("That file is not in this bulk upload."))?;
        if doc.bytes.is_empty() {
            return Err(anyhow::anyhow!("This file is no longer held here. Open the request to see the filed copy."));
        }
        Ok(json!({ "file_name": doc.file_name, "content_base64": base64::engine::general_purpose::STANDARD.encode(doc.bytes.as_slice()) }))
    })
}

pub fn remove(user: Option<UserAuth>, batch_id: String, client_key: String, file_name: String) -> HandlerResult<Value> {
    let user = require_user(user)?;
    with_batch(&user, &batch_id, |batch| {
        require_staged(batch)?;
        if file_name.is_empty() {
            batch.items.retain(|i| i.client_key != client_key);
        } else if let Some(item) = batch.items.iter_mut().find(|i| i.client_key == client_key) {
            item.docs.retain(|d| d.file_name != file_name);
            item.recompute();
        }
        Ok(serde_json::to_value(&*batch)?)
    })
}

pub fn cancel(user: Option<UserAuth>, batch_id: String) -> HandlerResult<Value> {
    let user = require_user(user)?;
    with_batch(&user, &batch_id, |batch| {
        require_staged(batch)?;
        batch.status = "cancelled";
        for item in &mut batch.items {
            for d in &mut item.docs {
                d.bytes = Arc::new(Vec::new());
            }
        }
        Ok(json!({ "id": batch.id, "status": batch.status }))
    })
}

fn require_staged(batch: &Batch) -> HandlerResult<()> {
    if batch.status == "staged" {
        Ok(())
    } else {
        Err(anyhow::anyhow!("This bulk upload has already been confirmed or cancelled."))
    }
}

// ---------------------------------------------------------------------------------------------
// Confirm: create each ready client's request
// ---------------------------------------------------------------------------------------------

/// How many clients' requests are created at the same time (`BULK_CONCURRENCY`, default 6). Each
/// one makes its storage folder and database rows and uploads its files, which mostly wait on the
/// network, so several together finish far sooner than one after another. The limit keeps a large
/// batch from flooding the database and Box; filing itself has its own limit (`FILING_CONCURRENCY`).
fn bulk_concurrency() -> usize {
    std::env::var("BULK_CONCURRENCY").ok().and_then(|v| v.trim().parse::<usize>().ok()).filter(|n| *n > 0).unwrap_or(6)
}

/// How many of one client's files are uploaded at the same time.
const UPLOADS_AT_ONCE: usize = 4;

pub fn confirm(user: Option<UserAuth>, data_access: &Arc<DataAccess>, entity_type: &Arc<EntityType>, batch_id: String) -> HandlerResult<Value> {
    let user = require_user(user)?;
    let principal = require_create(data_access, &user)?;
    // Uploading and submitting a request are what the New Request screens need `update` for.
    authz::require(&principal, "routing_record.update")?;
    let assigned = principal.user_id.clone().ok_or_else(|| anyhow::anyhow!("Your user account is not set up for this application."))?;
    let (ready, body) = with_batch(&user, &batch_id, |batch| {
        require_staged(batch)?;
        let mut ready = 0;
        for item in &mut batch.items {
            if item.status == "ready" {
                item.status = "queued";
                ready += 1;
            } else {
                item.status = "skipped";
            }
        }
        if ready == 0 {
            // Nothing to create: leave the batch as it was so the user can correct it.
            for item in &mut batch.items {
                item.status = "error";
                item.recompute();
            }
            return Err(anyhow::anyhow!("There is nothing ready to create. Fix or remove the clients with errors first."));
        }
        batch.status = "processing";
        Ok((ready, serde_json::to_value(&*batch)?))
    })?;
    tracing::info!(batch = %batch_id, clients = ready, "bulk creation started");
    let (da, et, u) = (data_access.clone(), entity_type.clone(), user.clone());
    tokio::spawn(async move { run_batch(u, da, et, batch_id, assigned).await });
    Ok(body)
}

/// Runs the failed clients again. Nothing is created twice: a client whose request already exists
/// carries on with that request (files already in its temp folder are reused, not uploaded again),
/// and a client that was never created gets its request now.
pub fn retry_failed(user: Option<UserAuth>, data_access: &Arc<DataAccess>, entity_type: &Arc<EntityType>, batch_id: String) -> HandlerResult<Value> {
    let user = require_user(user)?;
    let principal = require_create(data_access, &user)?;
    authz::require(&principal, "routing_record.update")?;
    let assigned = principal.user_id.clone().ok_or_else(|| anyhow::anyhow!("Your user account is not set up for this application."))?;
    let body = with_batch(&user, &batch_id, |batch| {
        if batch.status != "completed_with_errors" {
            return Err(anyhow::anyhow!("There is nothing to retry in this bulk upload."));
        }
        let mut again = 0;
        for item in &mut batch.items {
            if item.status == "failed" {
                item.status = "queued";
                item.message = None;
                again += 1;
            }
        }
        if again == 0 {
            return Err(anyhow::anyhow!("There is nothing to retry in this bulk upload."));
        }
        batch.status = "processing";
        Ok(serde_json::to_value(&*batch)?)
    })?;
    let (da, et, u) = (data_access.clone(), entity_type.clone(), user.clone());
    tokio::spawn(async move { run_batch(u, da, et, batch_id, assigned).await });
    Ok(body)
}

fn update_item(user: &UserAuth, batch_id: &str, key: &str, f: impl FnOnce(&mut Item)) {
    let _ = with_batch(user, batch_id, |batch| {
        if let Some(item) = batch.items.iter_mut().find(|i| i.client_key == key) {
            f(item);
        }
        Ok(())
    });
}

async fn run_batch(user: UserAuth, da: Arc<DataAccess>, et: Arc<EntityType>, batch_id: String, assigned: String) {
    let queued: Vec<Item> = with_batch(&user, &batch_id, |b| Ok(b.items.iter().filter(|i| i.status == "queued").cloned().collect())).unwrap_or_default();
    let started = std::time::Instant::now();
    let turns = Arc::new(tokio::sync::Semaphore::new(bulk_concurrency()));
    let mut running = tokio::task::JoinSet::new();
    for item in queued {
        let Ok(turn) = turns.clone().acquire_owned().await else { break };
        let (user, da, et, batch_id, assigned) = (user.clone(), da.clone(), et.clone(), batch_id.clone(), assigned.clone());
        running.spawn(async move {
            let key = item.client_key.clone();
            update_item(&user, &batch_id, &key, |i| i.status = "processing");
            let one = std::time::Instant::now();
            let result = create_one(&user, &da, &et, &assigned, &item).await;
            tracing::info!(batch = %batch_id, client = %key, ms = one.elapsed().as_millis() as u64, ok = result.is_ok(), "bulk item created");
            match result {
                Ok((record_id, case_number)) => update_item(&user, &batch_id, &key, |i| {
                    i.status = "created";
                    i.record_id = Some(record_id);
                    i.case_number = Some(case_number);
                    for d in &mut i.docs {
                        d.bytes = Arc::new(Vec::new());
                    }
                }),
                Err((message, record)) => {
                    tracing::warn!(batch = %batch_id, client = %key, error = %message, "bulk item failed");
                    update_item(&user, &batch_id, &key, |i| {
                        i.status = "failed";
                        i.message = Some(message);
                        if let Some((id, n)) = record {
                            i.record_id = Some(id);
                            i.case_number = Some(n);
                        }
                    });
                }
            }
            drop(turn);
        });
    }
    while running.join_next().await.is_some() {}
    let _ = with_batch(&user, &batch_id, |b| {
        let failed = b.items.iter().any(|i| i.status == "failed");
        b.status = if failed { "completed_with_errors" } else { "completed" };
        Ok(())
    });
    tracing::info!(batch = %batch_id, ms = started.elapsed().as_millis() as u64, "bulk creation finished");
}

type ItemFailure = (String, Option<(String, i32)>);

/// Creates one client's request the same way the New Request screens do. If the client already
/// has a request from an earlier try (`item.record_id`) that is still collecting documents, it
/// carries on with that one instead of making another.
async fn create_one(user: &UserAuth, da: &Arc<DataAccess>, et: &Arc<EntityType>, assigned: &str, item: &Item) -> Result<(String, i32), ItemFailure> {
    let fail = |m: String| -> ItemFailure { (m, None) };
    let t_start = std::time::Instant::now();

    // 1. A request left over from an earlier try, if it can still be used.
    let mut resumed: Option<TaxRoutingRecordProjection> = None;
    if let Some(id) = &item.record_id {
        if let Ok(existing) = load_record(da, et, id, Some(user.clone())).await {
            match existing.status {
                Some(RoutingRecordStatus::collecting_info) => resumed = Some(existing),
                // Already submitted by an earlier try: nothing is left to do for it.
                Some(RoutingRecordStatus::cancelled) | None => {}
                _ => return Ok((id.clone(), existing.case_number.unwrap_or_default())),
            }
        }
    }
    let resuming = resumed.is_some();

    let record: TaxRoutingRecordProjection = match resumed {
        Some(record) => record,
        None => {
            // 2. The client record: the directory's, or a new one made from the file.
            let client_id = match &item.existing_id {
                Some(id) => id.clone(),
                None => match find_clients(da, user, std::slice::from_ref(&item.client_key)).await {
                    // Made by an earlier try that failed later on: use it, do not add it twice.
                    Ok(found) if found.contains_key(&item.client_key.to_lowercase()) => found[&item.client_key.to_lowercase()].id.clone().unwrap_or_default(),
                    _ => {
                        let client_et = entity_type_for_handler(da, "tax_routing", "TaxPayerRefData").map_err(|e| fail(e.to_string()))?;
                        let c = &item.client;
                        let input = InputTaxPayerRefData {
                            id: None,
                            client_key: Some(item.client_key.clone()),
                            first_name: c.first_name.clone(),
                            last_name: c.last_name.clone(),
                            full_name: c.full_name.clone(),
                            personal_email: c.personal_email.clone(),
                            pds_email: c.pds_email.clone(),
                            office_location: c.office_location.clone(),
                            folder_name: c.client_folder.clone(),
                            external_folder_id: item.client_key.clone(),
                            internal_folder_path: c.internal_folder.clone(),
                            is_active: true,
                            read_write_password: item.password.clone(),
                            entity_type: None,
                            version: None,
                        };
                        let created = da
                            .create_item::<InputTaxPayerRefData, TaxPayerRefDataProjection>(client_et, client_selection(), input, Some(user.clone()))
                            .await
                            .map_err(|e| fail(format!("The client could not be added to the client directory: {e}")))?;
                        created.id.ok_or_else(|| fail("The new client has no id.".into()))?
                    }
                },
            };

            // 3. The request (case number and temporary folder are made here).
            let c = &item.client;
            let now = Utc::now();
            let input = InputTaxRoutingRecord {
                id: None,
                status: RoutingRecordStatus::collecting_info,
                created_at: now,
                updated_at: now,
                assigned_user_id: assigned.to_string(),
                client_first_name: Some(c.first_name.clone()),
                client_last_name: Some(c.last_name.clone()),
                client_full_name: Some(c.full_name.clone()),
                office_location: Some(c.office_location.clone()),
                pds_email: Some(c.pds_email.clone()),
                personal_email: Some(c.personal_email.clone()),
                additional_email: if c.additional_email.is_empty() { None } else { Some(c.additional_email.clone()) },
                notification_flag: c.notify,
                internal_folder: Some(c.internal_folder.clone()),
                folder_name: Some(c.client_folder.clone()),
                read_write_password: item.password.clone(),
                client_reference_id: client_id,
                progress_step: 0,
                case_number: None,
                temp_folder_id: None,
                version: None,
            };
            create_impl(Some(user.clone()), da, et, full_record_selection(), input).await.map_err(|e| fail(format!("The request could not be created: {e}")))?
        }
    };
    let record_id = record.id.clone().ok_or_else(|| fail("The new request has no id.".into()))?;
    let case_number = record.case_number.unwrap_or_default();
    let made = Some((record_id.clone(), case_number));
    let abandon = |m: String| -> ItemFailure { (m, made.clone()) };
    let t_created = t_start.elapsed().as_millis() as u64;

    // 4. The documents (a few at a time), then submit, which starts the filing pipeline. A failure
    //    here leaves the request as it is, so a retry carries on from this point.
    let uploads: Vec<Result<Value, String>> = stream::iter(0..item.docs.len())
        .map(|index| {
            let (record, d) = (&record, &item.docs[index]);
            async move {
            let uploaded = document_routing::upload_bytes_for_record(user, da, et, record, d.file_name.clone(), d.bytes.as_ref().clone(), resuming)
                .await
                .map_err(|e| format!("{} could not be uploaded: {e}", d.file_name))?;
            Ok(json!({
                "file_id": uploaded["file_id"],
                "file_name": d.file_name,
                "document_type": d.document_type,
                "year": d.year,
                "file_size": uploaded["file_size"],
                "entity_name": if d.entity_name.is_empty() { Value::Null } else { json!(d.entity_name) },
                "entity_type": if d.entity_type.is_empty() { Value::Null } else { json!(d.entity_type) },
                "entity_oracle_id": if d.entity_number.is_empty() { Value::Null } else { json!(d.entity_number) },
                "password_protected": d.password_protected,
            }))
        }
    })
    .buffered(UPLOADS_AT_ONCE)
    .collect()
    .await;
    let mut submitted: Vec<Value> = Vec::with_capacity(uploads.len());
    for upload in uploads {
        submitted.push(upload.map_err(&abandon)?);
    }
    let t_uploaded = t_start.elapsed().as_millis() as u64;
    document_routing::submit_for_record(user.clone(), da, et, record, Value::Array(submitted).to_string(), !resuming)
        .await
        .map_err(|e| abandon(format!("The request could not be submitted: {e}")))?;
    tracing::info!(client = %item.client_key, create_ms = t_created, upload_ms = t_uploaded - t_created, submit_ms = t_start.elapsed().as_millis() as u64 - t_uploaded, resumed = resuming, "bulk item timings");
    Ok((record_id, case_number))
}

// ---------------------------------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------------------------------

pub async fn status(user: Option<UserAuth>, data_access: &Arc<DataAccess>, entity_type: &Arc<EntityType>, batch_id: String) -> HandlerResult<Value> {
    let user = require_user(user)?;
    let mut body = with_batch(&user, &batch_id, |batch| Ok(serde_json::to_value(&*batch)?))?;
    // How far each created request has got in filing: one query for all of them, not one each.
    let ids: Vec<String> = body["items"].as_array().map(|items| items.iter().filter_map(|i| i["record_id"].as_str().map(str::to_string)).collect()).unwrap_or_default();
    if !ids.is_empty() {
        let f = |n: &str| json!({ "name": n, "selection_set": [] });
        let selection = json!({ "name": "tax_routing_record", "selection_set": [f("id"), f("status"), f("progress_step")] });
        let mut progress: HashMap<String, (String, i32)> = HashMap::new();
        for chunk in ids.chunks(200) {
            if let Ok(page) = data_access
                .query_items::<TaxRoutingRecordProjection>(entity_type.clone(), selection.clone(), Some(json!({ "id": { "_in": chunk } })), None, 0, 250, None, Some(user.clone()))
                .await
            {
                for r in page.items {
                    if let Some(id) = r.id {
                        progress.insert(id, (r.status.map(|s| format!("{s:?}")).unwrap_or_default(), r.progress_step.unwrap_or(0)));
                    }
                }
            }
        }
        if let Some(items) = body.get_mut("items").and_then(Value::as_array_mut) {
            for item in items {
                if let Some((status, step)) = item["record_id"].as_str().and_then(|id| progress.get(id)) {
                    item["record_status"] = json!(status);
                    item["progress_step"] = json!(step);
                }
            }
        }
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn types() -> Vec<(String, bool)> {
        ["K-1", "Tax Return", "Balance Sheet", "S-Corp", "S-Corp Election", "Extensions"].iter().map(|t| (t.to_string(), true)).collect()
    }

    #[test]
    fn a_file_name_gives_type_year_and_entity() {
        let c = classify_name("K-1_2025_Oakwood-Medical.pdf", &types(), &[2024, 2025]).unwrap();
        assert_eq!((c.document_type.as_str(), c.year, c.entity_hint.as_str()), ("K-1", 2025, "Oakwood Medical"));
        let c = classify_name("Tax-Return_2024.pdf", &types(), &[2024, 2025]).unwrap();
        assert_eq!((c.document_type.as_str(), c.year, c.entity_hint.as_str()), ("Tax Return", 2024, ""));
    }

    #[test]
    fn an_entity_hint_matches_the_directory_by_name_or_unique_prefix() {
        let e: Vec<EntityRow> = vec![
            ("Partnership".into(), "Oakwood Medical Partners LLC".into(), "001".into()),
            ("Partnership".into(), "Riverside Surgical Associates LP".into(), "002".into()),
            ("S-Corp".into(), "Riverside Surgical Holdings Inc.".into(), "003".into()),
        ];
        assert_eq!(find_entity("Oakwood Medical Partners", &e).map(|x| x.2.as_str()), Some("001"));
        assert_eq!(find_entity("Riverside Surgical Associates LP", &e).map(|x| x.2.as_str()), Some("002"));
        assert!(find_entity("Riverside Surgical", &e).is_none());
        assert!(find_entity("", &e).is_none());
    }

    #[test]
    fn the_longest_matching_type_wins() {
        let c = classify_name("S-Corp-Election_2025.pdf", &types(), &[]).unwrap();
        assert_eq!(c.document_type, "S-Corp Election");
        let c = classify_name("S-Corp_2025.pdf", &types(), &[]).unwrap();
        assert_eq!(c.document_type, "S-Corp");
    }

    #[test]
    fn unreadable_names_are_not_guessed() {
        assert!(classify_name("scan0001.pdf", &types(), &[2025]).is_none());
        assert!(classify_name("K-1_notayear.pdf", &types(), &[2025]).is_none());
        assert!(classify_name("K-1_1500.pdf", &types(), &[2025]).is_none());
        assert!(classify_name("Wedding_2025.pdf", &types(), &[2025]).is_none());
    }

    #[test]
    fn the_year_must_be_one_the_business_has_set_up() {
        // The configured list decides, not a range written into the code.
        assert!(classify_name("K-1_2019_X.pdf", &types(), &[2024, 2025]).is_none());
        assert!(classify_name("K-1_2019_X.pdf", &types(), &[2019]).is_some());
        // With no list at all, any four-digit year is read as a year.
        assert!(classify_name("K-1_2019_X.pdf", &types(), &[]).is_some());
    }

    #[test]
    fn excel_rows_map_by_flexible_headers() {
        let rows = vec![
            vec!["Client Key", "Client Name", "First Name", "Last Name", "Internal Folder ID", "Client Folder ID", "Send Notification"].into_iter().map(String::from).collect(),
            vec!["CLI-1", "Moore, Andrea", "Andrea", "Moore", "111", "222", "Yes"].into_iter().map(String::from).collect(),
            vec!["", "", "", "", "", "", ""].into_iter().map(String::from).collect(),
        ];
        let out = rows_from_text(&rows).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!((out[0].key.as_str(), out[0].internal_folder.as_str(), out[0].client_folder.as_str(), out[0].notify), ("CLI-1", "111", "222", true));
    }

    #[test]
    fn excel_without_a_client_key_column_is_refused() {
        let rows = vec![vec!["Client Name".to_string()]];
        assert!(rows_from_text(&rows).is_err());
    }

    #[test]
    fn numeric_cells_keep_long_folder_ids() {
        assert_eq!(cell_text(&Data::Float(423190537329.0)), "423190537329");
        assert_eq!(cell_text(&Data::Int(5)), "5");
    }

    fn zip_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
        use std::io::Write;
        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let opts = zip::write::SimpleFileOptions::default();
        for (name, bytes) in entries {
            w.start_file(*name, opts).unwrap();
            w.write_all(bytes).unwrap();
        }
        w.finish().unwrap().into_inner()
    }

    #[test]
    fn the_zip_is_grouped_by_top_level_folder() {
        let z = zip_of(&[("CLI-1/a.pdf", b"%PDF-1"), ("CLI-1/b.pdf", b"%PDF-2"), ("cli-2/c.pdf", b"%PDF-3"), ("loose.pdf", b"%PDF-4"), ("__MACOSX/x", b"1"), ("CLI-1/.DS_Store", b"1")]);
        let scan = read_zip(&z, 1024).unwrap();
        assert_eq!(scan.folders.len(), 2);
        assert_eq!(scan.folders["cli-1"].files.len(), 2);
        assert_eq!(scan.notes.len(), 1);
    }

    #[test]
    fn nested_and_oversized_files_are_flagged_not_read() {
        let z = zip_of(&[("CLI-1/deep/a.pdf", b"%PDF-1"), ("CLI-1/big.pdf", &[b'x'; 2000])]);
        let scan = read_zip(&z, 1000).unwrap();
        let files = &scan.folders["cli-1"].files;
        assert!(files.iter().all(|f| f.problem.is_some()));
    }

    #[test]
    fn a_non_pdf_and_an_unreadable_name_become_errors_a_good_one_is_ready() {
        let t = types();
        let ok = ZipFile { name: "K-1_2025_Oak.pdf".into(), bytes: Arc::new(b"%PDF-1.4".to_vec()), problem: None };
        let bad_name = ZipFile { name: "scan.pdf".into(), bytes: Arc::new(b"%PDF-1.4".to_vec()), problem: None };
        let not_pdf = ZipFile { name: "K-1_2025.pdf".into(), bytes: Arc::new(b"hello".to_vec()), problem: None };
        assert_eq!(build_doc(&ok, &t, &[], &[2025], true).status, "ready");
        assert!(build_doc(&ok, &t, &[], &[2025], true).password_protected);
        assert!(!build_doc(&ok, &t, &[], &[2025], false).password_protected);
        assert_eq!(build_doc(&bad_name, &t, &[], &[2025], true).status, "error");
        assert_eq!(build_doc(&not_pdf, &t, &[], &[2025], true).status, "error");
    }
}
