//! `DocumentStore` over the Box API: the five operations Pega's Box activities perform
//! (`CreateBoxInheritFolder`, `SaveToBoxStorage`, the duplicate check in `PDFFileProcessing`,
//! `GetFileContent`, `DeleteTempFolder`).

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use super::auth::JwtMinter;
use super::config::BoxConfig;
use super::http::{BoxBody, BoxHttp, BoxMethod, BoxRequest, BoxResponse, ReqwestBoxHttp};
use super::token_manager::{TokenManager, TokenStore};
use crate::services::document_store::{validate_name, DocumentStore, FileRef, FolderRef, StoreError};

/// Box returns at most this many items per page.
const PAGE_SIZE: usize = 1000;

pub struct BoxDocumentStore {
    http: Arc<dyn BoxHttp>,
    api_base: String,
    upload_base: String,
}

impl BoxDocumentStore {
    pub fn new(http: Arc<dyn BoxHttp>, api_base: &str, upload_base: &str) -> Self {
        Self { http, api_base: api_base.trim_end_matches('/').to_string(), upload_base: upload_base.trim_end_matches('/').to_string() }
    }

    /// `token_store` is the table the token is kept in between calls and restarts; `None` keeps it
    /// in memory only.
    pub fn from_config(config: BoxConfig, token_store: Option<Arc<dyn TokenStore>>) -> Result<Self, String> {
        let (api_base, upload_base) = (config.api_base.clone(), config.upload_base.clone());
        let minter = Arc::new(JwtMinter::new(config)?);
        let tokens = Arc::new(TokenManager::new("box", minter, token_store));
        Ok(Self::new(Arc::new(ReqwestBoxHttp::new(tokens)), &api_base, &upload_base))
    }

    fn request(&self, method: BoxMethod, url: String, query: Vec<(String, String)>, body: BoxBody) -> BoxRequest {
        BoxRequest { method, url, query, body }
    }
}

fn json_of(response: &BoxResponse) -> Value {
    serde_json::from_slice(&response.body).unwrap_or(Value::Null)
}

/// Box's `code` / `message` explain a refusal and hold nothing sensitive.
fn reason(response: &BoxResponse) -> String {
    let v = json_of(response);
    let code = v["code"].as_str().unwrap_or("");
    let message = v["message"].as_str().unwrap_or("");
    format!("{code} {message}").trim().chars().take(200).collect()
}

/// The status mapping shared by every call. `what` names the thing asked about, for messages.
fn failure(response: &BoxResponse, what: &str) -> StoreError {
    match response.status {
        404 => StoreError::NotFound(what.to_string()),
        409 => StoreError::AlreadyExists(what.to_string()),
        401 | 403 => StoreError::Unavailable(format!("Box refused access to {what} (HTTP {}): {}", response.status, reason(response))),
        429 => StoreError::Unavailable("Box is rate limiting requests; try again shortly".into()),
        s if s >= 500 => StoreError::Unavailable(format!("Box had a server error (HTTP {s})")),
        s => StoreError::Unavailable(format!("Box rejected the request for {what} (HTTP {s}): {}", reason(response))),
    }
}

fn id_of(value: &Value) -> Option<String> {
    value["id"].as_str().map(str::to_string)
}

#[async_trait]
impl DocumentStore for BoxDocumentStore {
    async fn ensure_folder(&self, parent: &FolderRef, name: &str) -> Result<FolderRef, StoreError> {
        validate_name(name)?;
        let response = self
            .http
            .send(self.request(
                BoxMethod::Post,
                format!("{}/folders", self.api_base),
                vec![],
                BoxBody::Json(json!({ "name": name, "parent": { "id": parent.0 } })),
            ))
            .await?;
        match response.status {
            200 | 201 => id_of(&json_of(&response))
                .map(FolderRef)
                .ok_or_else(|| StoreError::Unavailable("Box did not return the new folder id".into())),
            // The folder already exists: Box names it in the conflict, so a repeat is a no-op.
            409 => {
                let conflicts = &json_of(&response)["context_info"]["conflicts"];
                let existing = match conflicts {
                    Value::Array(items) => items.first(),
                    Value::Object(_) => Some(conflicts),
                    _ => None,
                };
                match existing {
                    Some(item) if item["type"].as_str().unwrap_or("folder") == "folder" => id_of(item)
                        .map(FolderRef)
                        .ok_or_else(|| StoreError::Unavailable("Box reported an existing folder without an id".into())),
                    _ => Err(StoreError::AlreadyExists(name.to_string())),
                }
            }
            _ => Err(failure(&response, &format!("folder {}", parent.0))),
        }
    }

    async fn create_folder(&self, parent: &FolderRef, name: &str) -> Result<FolderRef, StoreError> {
        validate_name(name)?;
        let response = self
            .http
            .send(self.request(
                BoxMethod::Post,
                format!("{}/folders", self.api_base),
                vec![],
                BoxBody::Json(json!({ "name": name, "parent": { "id": parent.0 } })),
            ))
            .await?;
        match response.status {
            200 | 201 => id_of(&json_of(&response))
                .map(FolderRef)
                .ok_or_else(|| StoreError::Unavailable("Box did not return the new folder id".into())),
            409 => Err(StoreError::AlreadyExists(name.to_string())),
            404 => Err(StoreError::NotFound(format!("folder {}", parent.0))),
            _ => Err(failure(&response, &format!("folder {}", parent.0))),
        }
    }

    async fn upload_file(&self, parent: &FolderRef, name: &str, bytes: Vec<u8>) -> Result<FileRef, StoreError> {
        validate_name(name)?;
        let response = self
            .http
            .send(self.request(
                BoxMethod::Post,
                format!("{}/files/content", self.upload_base),
                vec![],
                BoxBody::Upload { attributes: json!({ "name": name, "parent": { "id": parent.0 } }), file_name: name.to_string(), bytes },
            ))
            .await?;
        match response.status {
            200 | 201 => json_of(&response)["entries"]
                .get(0)
                .and_then(id_of)
                .map(FileRef)
                .ok_or_else(|| StoreError::Unavailable("Box did not return the new file id".into())),
            409 => Err(StoreError::AlreadyExists(name.to_string())),
            404 => Err(StoreError::NotFound(format!("folder {}", parent.0))),
            _ => Err(failure(&response, name)),
        }
    }

    async fn upload_or_existing(&self, parent: &FolderRef, name: &str, bytes: Vec<u8>) -> Result<FileRef, StoreError> {
        validate_name(name)?;
        let response = self
            .http
            .send(self.request(
                BoxMethod::Post,
                format!("{}/files/content", self.upload_base),
                vec![],
                BoxBody::Upload { attributes: json!({ "name": name, "parent": { "id": parent.0 } }), file_name: name.to_string(), bytes },
            ))
            .await?;
        match response.status {
            200 | 201 => json_of(&response)["entries"]
                .get(0)
                .and_then(id_of)
                .map(FileRef)
                .ok_or_else(|| StoreError::Unavailable("Box did not return the new file id".into())),
            // The name is taken: Box names the file that holds it in the conflict, so no folder listing is needed.
            409 => {
                let conflicts = &json_of(&response)["context_info"]["conflicts"];
                let existing = match conflicts {
                    Value::Array(items) => items.first(),
                    Value::Object(_) => Some(conflicts),
                    _ => None,
                };
                match existing.and_then(id_of) {
                    Some(id) => Ok(FileRef(id)),
                    None => self.find_file(parent, name).await?.ok_or_else(|| StoreError::AlreadyExists(name.to_string())),
                }
            }
            404 => Err(StoreError::NotFound(format!("folder {}", parent.0))),
            _ => Err(failure(&response, name)),
        }
    }

    async fn find_file(&self, parent: &FolderRef, name: &str) -> Result<Option<FileRef>, StoreError> {
        let mut offset = 0usize;
        loop {
            let response = self
                .http
                .send(self.request(
                    BoxMethod::Get,
                    format!("{}/folders/{}/items", self.api_base, parent.0),
                    vec![
                        ("fields".into(), "id,name,type".into()),
                        ("limit".into(), PAGE_SIZE.to_string()),
                        ("offset".into(), offset.to_string()),
                    ],
                    BoxBody::None,
                ))
                .await?;
            if response.status != 200 {
                return Err(failure(&response, &format!("folder {}", parent.0)));
            }
            let body = json_of(&response);
            let entries = body["entries"].as_array().cloned().unwrap_or_default();
            if let Some(hit) = entries
                .iter()
                .find(|e| e["type"] == "file" && e["name"].as_str().is_some_and(|n| n.eq_ignore_ascii_case(name)))
            {
                return Ok(id_of(hit).map(FileRef));
            }
            let total = body["total_count"].as_u64().unwrap_or(0) as usize;
            offset += entries.len();
            if entries.is_empty() || offset >= total {
                return Ok(None);
            }
        }
    }

    async fn read_file(&self, file: &FileRef) -> Result<Vec<u8>, StoreError> {
        let response = self
            .http
            .send(self.request(BoxMethod::Get, format!("{}/files/{}/content", self.api_base, file.0), vec![], BoxBody::None))
            .await?;
        match response.status {
            200 => Ok(response.body),
            _ => Err(failure(&response, &format!("file {}", file.0))),
        }
    }

    async fn folder_exists(&self, folder: &FolderRef) -> Result<bool, StoreError> {
        let response = self
            .http
            .send(self.request(
                BoxMethod::Get,
                format!("{}/folders/{}", self.api_base, folder.0),
                vec![("fields".into(), "id".into())],
                BoxBody::None,
            ))
            .await?;
        match response.status {
            200 => Ok(true),
            404 => Ok(false),
            _ => Err(failure(&response, &format!("folder {}", folder.0))),
        }
    }

    async fn delete_folder(&self, folder: &FolderRef) -> Result<(), StoreError> {
        let response = self
            .http
            .send(self.request(
                BoxMethod::Delete,
                format!("{}/folders/{}", self.api_base, folder.0),
                vec![("recursive".into(), "true".into())],
                BoxBody::None,
            ))
            .await?;
        match response.status {
            200 | 204 => Ok(()),
            _ => Err(failure(&response, &format!("folder {}", folder.0))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use super::*;
    use crate::services::document_store::workflow::{
        clean_temp, create_temp_folder, place_documents, upload_to_temp, PdfProtector, Roots, SubmitDocument, UploadRules,
    };
    use crate::services::document_store::routing::{find_route, DocumentRoute};

    /// A small stand-in for Box that follows the documented behaviour of the five calls we make:
    /// 201 on create, 409 with the existing item in `context_info.conflicts`, paged item listing,
    /// recursive delete.
    #[derive(Default)]
    struct FakeState {
        next: u64,
        folders: HashMap<String, (Option<String>, String)>,
        files: HashMap<String, (String, String, Vec<u8>)>,
        calls: Vec<String>,
    }

    struct FakeBox(Mutex<FakeState>);

    impl FakeBox {
        fn new(root_ids: &[&str]) -> Self {
            let mut s = FakeState::default();
            for id in root_ids {
                s.folders.insert((*id).to_string(), (None, format!("root-{id}")));
            }
            Self(Mutex::new(s))
        }
        fn resp(status: u16, body: Value) -> BoxResponse {
            BoxResponse { status, body: serde_json::to_vec(&body).unwrap() }
        }
        fn paths(&self) -> Vec<String> {
            let s = self.0.lock().unwrap();
            let path_of = |id: &str| {
                let mut parts = vec![];
                let mut cur = Some(id.to_string());
                while let Some(c) = cur {
                    let Some((p, n)) = s.folders.get(&c) else { break };
                    parts.push(n.clone());
                    cur = p.clone();
                }
                parts.reverse();
                parts.join("/")
            };
            let mut out: Vec<String> = s.files.values().map(|(p, n, _)| format!("{}/{}", path_of(p), n)).collect();
            out.sort();
            out
        }
        fn calls(&self) -> Vec<String> {
            self.0.lock().unwrap().calls.clone()
        }
    }

    #[async_trait]
    impl BoxHttp for FakeBox {
        async fn send(&self, req: BoxRequest) -> Result<BoxResponse, StoreError> {
            let mut s = self.0.lock().unwrap();
            let path = req.url.split("/2.0").last().unwrap_or("").to_string();
            let path = path.trim_start_matches("/api").to_string();
            s.calls.push(format!("{:?} {}", req.method, path));
            match (req.method, path.as_str()) {
                (BoxMethod::Post, "/folders") => {
                    let BoxBody::Json(b) = &req.body else { panic!("json expected") };
                    let (name, parent) = (b["name"].as_str().unwrap().to_string(), b["parent"]["id"].as_str().unwrap().to_string());
                    if !s.folders.contains_key(&parent) {
                        return Ok(Self::resp(404, json!({"code":"not_found"})));
                    }
                    if let Some((id, _)) = s.folders.iter().find(|(_, (p, n))| p.as_deref() == Some(&parent) && n.eq_ignore_ascii_case(&name)) {
                        return Ok(Self::resp(409, json!({"code":"item_name_in_use","context_info":{"conflicts":[{"type":"folder","id":id}]}})));
                    }
                    s.next += 1;
                    let id = format!("{}", 1000 + s.next);
                    s.folders.insert(id.clone(), (Some(parent), name));
                    Ok(Self::resp(201, json!({"type":"folder","id":id})))
                }
                (BoxMethod::Post, "/files/content") => {
                    let BoxBody::Upload { attributes, bytes, .. } = req.body else { panic!("upload expected") };
                    let (name, parent) = (attributes["name"].as_str().unwrap().to_string(), attributes["parent"]["id"].as_str().unwrap().to_string());
                    if !s.folders.contains_key(&parent) {
                        return Ok(Self::resp(404, json!({"code":"not_found"})));
                    }
                    if let Some((existing, _)) = s.files.iter().find(|(_, (p, n, _))| p == &parent && n.eq_ignore_ascii_case(&name)) {
                        // Box names the file that holds the name in the conflict.
                        return Ok(Self::resp(409, json!({"code":"item_name_in_use","context_info":{"conflicts":{"type":"file","id":existing}}})));
                    }
                    s.next += 1;
                    let id = format!("{}", 5000 + s.next);
                    s.files.insert(id.clone(), (parent, name, bytes));
                    Ok(Self::resp(201, json!({"total_count":1,"entries":[{"type":"file","id":id}]})))
                }
                (BoxMethod::Get, p) if p.starts_with("/folders/") && !p.ends_with("/items") => {
                    let id = p.trim_start_matches("/folders/");
                    if s.folders.contains_key(id) {
                        Ok(Self::resp(200, json!({"type":"folder","id":id})))
                    } else {
                        Ok(Self::resp(404, json!({"code":"not_found"})))
                    }
                }
                (BoxMethod::Get, p) if p.starts_with("/folders/") && p.ends_with("/items") => {
                    let id = p.trim_start_matches("/folders/").trim_end_matches("/items").to_string();
                    if !s.folders.contains_key(&id) {
                        return Ok(Self::resp(404, json!({"code":"not_found"})));
                    }
                    let q = |k: &str| req.query.iter().find(|(a, _)| a == k).map(|(_, v)| v.parse::<usize>().unwrap_or(0)).unwrap_or(0);
                    let (limit, offset) = (q("limit"), q("offset"));
                    let mut all: Vec<Value> = s.files.iter().filter(|(_, (p, _, _))| p == &id).map(|(fid, (_, n, _))| json!({"type":"file","id":fid,"name":n})).collect();
                    all.extend(s.folders.iter().filter(|(_, (p, _))| p.as_deref() == Some(&id)).map(|(fid, (_, n))| json!({"type":"folder","id":fid,"name":n})));
                    all.sort_by_key(|e| e["id"].as_str().unwrap().to_string());
                    let page: Vec<Value> = all.iter().skip(offset).take(limit.max(1)).cloned().collect();
                    Ok(Self::resp(200, json!({"total_count": all.len(), "entries": page})))
                }
                (BoxMethod::Get, p) if p.starts_with("/files/") && p.ends_with("/content") => {
                    let id = p.trim_start_matches("/files/").trim_end_matches("/content");
                    match s.files.get(id) {
                        Some((_, _, b)) => Ok(BoxResponse { status: 200, body: b.clone() }),
                        None => Ok(Self::resp(404, json!({"code":"not_found"}))),
                    }
                }
                (BoxMethod::Delete, p) if p.starts_with("/folders/") => {
                    let id = p.trim_start_matches("/folders/").to_string();
                    if !s.folders.contains_key(&id) {
                        return Ok(Self::resp(404, json!({"code":"not_found"})));
                    }
                    let mut doomed = vec![id];
                    let mut i = 0;
                    while i < doomed.len() {
                        let cur = doomed[i].clone();
                        let kids: Vec<String> = s.folders.iter().filter(|(_, (p, _))| p.as_deref() == Some(&cur)).map(|(k, _)| k.clone()).collect();
                        doomed.extend(kids);
                        i += 1;
                    }
                    s.files.retain(|_, (p, _, _)| !doomed.contains(p));
                    for d in &doomed {
                        s.folders.remove(d);
                    }
                    Ok(BoxResponse { status: 204, body: vec![] })
                }
                other => panic!("unexpected Box call {other:?}"),
            }
        }
    }

    fn store(fake: Arc<FakeBox>) -> BoxDocumentStore {
        BoxDocumentStore::new(fake, "https://api.box.com/2.0", "https://upload.box.com/api/2.0")
    }

    struct Prefix;
    impl PdfProtector for Prefix {
        fn protect(&self, pdf: &[u8]) -> Result<Vec<u8>, String> {
            Ok([b"ENC:".as_slice(), pdf].concat())
        }
    }

    #[tokio::test]
    async fn ensure_folder_is_get_or_create_through_the_409_conflict() {
        let fake = Arc::new(FakeBox::new(&["1"]));
        let s = store(fake.clone());
        let a = s.ensure_folder(&FolderRef("1".into()), "2025").await.unwrap();
        let b = s.ensure_folder(&FolderRef("1".into()), "2025").await.unwrap();
        assert_eq!(a, b, "the second call returns the folder Box says already exists");
        assert!(matches!(s.ensure_folder(&FolderRef("999".into()), "x").await, Err(StoreError::NotFound(_))));
        assert!(matches!(s.ensure_folder(&FolderRef("1".into()), "bad/name").await, Err(StoreError::InvalidName(_))));
    }

    #[tokio::test]
    async fn upload_or_existing_is_one_call_and_returns_the_file_already_there() {
        let fake = Arc::new(FakeBox::new(&["1"]));
        let s = store(fake.clone());
        let root = FolderRef("1".into());
        let first = s.upload_or_existing(&root, "a.pdf", b"one".to_vec()).await.unwrap();
        let calls_before = fake.calls().len();
        let again = s.upload_or_existing(&root, "A.PDF", b"two".to_vec()).await.unwrap();
        assert_eq!(first, again, "a repeat finds the same file instead of failing or duplicating");
        assert_eq!(fake.calls().len(), calls_before + 1, "the repeat is a single upload attempt, not a folder listing plus an upload");
        assert_eq!(fake.paths().len(), 1);
    }

    #[tokio::test]
    async fn documents_of_one_request_share_their_year_and_type_folders() {
        use crate::services::document_store::workflow::{place_prepared, place_prepared_cached, FolderCache, Prepared};
        let route = DocumentRoute { document_type: "K-1".into(), internal_folder: Some("4. Tax Documents".into()), external_folder: "K-1s".into(), is_external_only: false };
        let doc = |name: &str| Prepared { original: b"o".to_vec(), client_copy: b"c".to_vec(), target_name: name.into(), year: 2025, route: route.clone() };
        let folder_posts = |fake: &FakeBox| fake.calls().iter().filter(|c| c.as_str() == "Post /folders").count();

        // One at a time with no sharing: every document asks Box for every folder again.
        let plain = Arc::new(FakeBox::new(&["1", "2"]));
        let s = store(plain.clone());
        let (internal, client) = (FolderRef("1".into()), FolderRef("2".into()));
        let roots = Roots { internal: &internal, external: Some(&client), released_folder_name: "6. Released to Client" };
        place_prepared(&s, &roots, &doc("a.pdf")).await.unwrap();
        place_prepared(&s, &roots, &doc("b.pdf")).await.unwrap();

        // With a shared cache the second document finds all of them already known.
        let shared = Arc::new(FakeBox::new(&["1", "2"]));
        let s2 = store(shared.clone());
        let cache = FolderCache::default();
        place_prepared_cached(&s2, &roots, &doc("a.pdf"), &cache).await.unwrap();
        place_prepared_cached(&s2, &roots, &doc("b.pdf"), &cache).await.unwrap();

        assert_eq!(plain.paths(), shared.paths(), "the files end up in exactly the same places");
        assert!(folder_posts(&shared) < folder_posts(&plain), "{} folder calls with the cache, {} without", folder_posts(&shared), folder_posts(&plain));
        assert_eq!(folder_posts(&shared) * 2, folder_posts(&plain), "the second document adds no folder calls");
    }

    #[tokio::test]
    async fn create_folder_refuses_a_name_that_is_already_taken() {
        let fake = Arc::new(FakeBox::new(&["1"]));
        let s = store(fake);
        let root = FolderRef("1".into());
        // A folder that was there before us (like a Pega case's TD-2797) must never be adopted.
        let theirs = s.ensure_folder(&root, "TD-2797").await.unwrap();
        assert!(matches!(s.create_folder(&root, "TD-2797").await, Err(StoreError::AlreadyExists(_))));
        let ours = s.create_folder(&root, "TD-1001").await.unwrap();
        assert_ne!(ours, theirs);
        assert!(matches!(s.create_folder(&FolderRef("999".into()), "x").await, Err(StoreError::NotFound(_))));
    }

    #[tokio::test]
    async fn upload_find_read_and_refuse_to_overwrite() {
        let fake = Arc::new(FakeBox::new(&["1"]));
        let s = store(fake);
        let root = FolderRef("1".into());
        let file = s.upload_file(&root, "K1.pdf", b"%PDF".to_vec()).await.unwrap();
        assert_eq!(s.find_file(&root, "k1.PDF").await.unwrap(), Some(file.clone()), "names match case-insensitively");
        assert_eq!(s.find_file(&root, "other.pdf").await.unwrap(), None);
        assert_eq!(s.read_file(&file).await.unwrap(), b"%PDF");
        assert!(matches!(s.upload_file(&root, "K1.pdf", b"x".to_vec()).await, Err(StoreError::AlreadyExists(_))));
        assert!(matches!(s.read_file(&FileRef("nope".into())).await, Err(StoreError::NotFound(_))));
    }

    #[tokio::test]
    async fn find_file_pages_through_a_large_folder() {
        let fake = Arc::new(FakeBox::new(&["1"]));
        let s = store(fake);
        let root = FolderRef("1".into());
        // 1,005 folders precede the file when ordered by id, so it sits on the second page.
        for i in 0..1005 {
            s.ensure_folder(&root, &format!("f{i}")).await.unwrap();
        }
        let file = s.upload_file(&root, "late.pdf", b"x".to_vec()).await.unwrap();
        assert_eq!(s.find_file(&root, "late.pdf").await.unwrap(), Some(file));
    }

    #[tokio::test]
    async fn folder_exists_tells_a_real_folder_from_a_wrong_id() {
        let fake = Arc::new(FakeBox::new(&["1"]));
        let s = store(fake);
        assert!(s.folder_exists(&FolderRef("1".into())).await.unwrap());
        assert!(!s.folder_exists(&FolderRef("999".into())).await.unwrap());
    }

    #[tokio::test]
    async fn delete_folder_removes_everything_under_it() {
        let fake = Arc::new(FakeBox::new(&["1"]));
        let s = store(fake.clone());
        let temp = s.ensure_folder(&FolderRef("1".into()), "TD-1001").await.unwrap();
        s.upload_file(&temp, "a.pdf", b"x".to_vec()).await.unwrap();
        s.delete_folder(&temp).await.unwrap();
        assert!(fake.paths().is_empty());
        assert!(matches!(s.delete_folder(&temp).await, Err(StoreError::NotFound(_))));
    }

    #[tokio::test]
    async fn the_pega_k1_2025_flow_runs_unchanged_on_box() {
        let fake = Arc::new(FakeBox::new(&["84473772615", "20", "30"]));
        let s = store(fake.clone());
        let (internal_root, client_internal, client_external) = (FolderRef("84473772615".into()), FolderRef("20".into()), FolderRef("30".into()));
        let routes = vec![DocumentRoute { document_type: "K-1".into(), internal_folder: Some("4. Tax Documents".into()), external_folder: "K-1s".into(), is_external_only: false }];

        // Phase 1: temp folder named with the case id, inside the INTERNAL root; PDF goes in it.
        let temp = create_temp_folder(&s, &internal_root, "TD-1001").await.unwrap();
        let file = upload_to_temp(&s, &temp, "Pega AI.pdf", b"%PDF data".to_vec(), &UploadRules::default()).await.unwrap();

        // Phases 2-4: folders, copies under the final name, temp folder deleted.
        let docs = [SubmitDocument { temp_file: file, target_name: "Moore, Andrea - Pega AI.pdf".into(), year: 2025, route: find_route(&routes, "K-1").unwrap().clone(), protect_client_copies: true }];
        let roots = Roots { internal: &client_internal, external: Some(&client_external), released_folder_name: "6. Released to Client" };
        let results = place_documents(&s, &Prefix, &roots, &docs).await;
        assert!(results[0].is_ok());
        assert!(clean_temp(&s, &temp, &results).await.unwrap());

        assert_eq!(
            fake.paths(),
            vec![
                "root-20/2025/4. Tax Documents/Moore, Andrea - Pega AI.pdf",
                "root-20/2025/6. Released to Client/K-1s/Moore, Andrea - Pega AI.pdf",
                "root-30/2025/K-1s/Moore, Andrea - Pega AI.pdf",
            ]
        );
        assert!(fake.calls().iter().any(|c| c.starts_with("Delete /folders/")), "the temp folder was deleted");
    }

    /// Talks to the REAL Box. Creates a throwaway folder under a TEST folder, uploads, reads,
    /// refuses a duplicate, then deletes it. Run it yourself (secrets stay in your terminal):
    ///   set BOX_* settings and BOX_TEST_PARENT_FOLDER_ID, then
    ///   cargo test box_live -- --ignored --nocapture
    #[tokio::test]
    #[ignore = "calls the real Box API; needs BOX_* settings and BOX_TEST_PARENT_FOLDER_ID"]
    async fn box_live_round_trip() {
        let config = BoxConfig::from_env().expect("settings").expect("BOX_* settings are required");
        let parent = std::env::var("BOX_TEST_PARENT_FOLDER_ID").expect("BOX_TEST_PARENT_FOLDER_ID");
        let parent = FolderRef(crate::services::document_store::folder_id_from_url(&parent).expect("a numeric folder id or Box folder URL"));
        let s = BoxDocumentStore::from_config(config, None).expect("Box store");

        let name = format!("tax-app-live-check-{}", uuid::Uuid::new_v4());
        let folder = s.ensure_folder(&parent, &name).await.expect("create folder");
        assert_eq!(s.ensure_folder(&parent, &name).await.expect("repeat is a no-op"), folder);
        let file = s.upload_file(&folder, "check.pdf", b"%PDF-live-check".to_vec()).await.expect("upload");
        assert_eq!(s.find_file(&folder, "CHECK.pdf").await.expect("find"), Some(file.clone()));
        assert_eq!(s.read_file(&file).await.expect("read"), b"%PDF-live-check");
        assert!(matches!(s.upload_file(&folder, "check.pdf", b"x".to_vec()).await, Err(StoreError::AlreadyExists(_))));
        s.delete_folder(&folder).await.expect("delete the throwaway folder");
        println!("Box round trip OK (created, repeated, uploaded, found, read, refused duplicate, deleted)");
    }
}
