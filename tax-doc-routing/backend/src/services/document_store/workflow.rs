//! The Pega process, phase by phase, against `DocumentStore`:
//!   1. `create_temp_folder` + `upload_to_temp`   (CreateTempBoxFolder, SaveToBoxStorage)
//!   2. + 3. `place_document(s)`                  (CreateTargetFolders, CreateFolderReleasedToClient,
//!                                                 PDFFileProcessing / MoveFile_*)
//!   4. `clean_temp`                              (DeleteTempFolder)
//! Every step is safe to repeat, so a retry re-runs only what failed.

use super::routing::{plan_for, DocumentRoute, RoutingError};
use super::{DocumentStore, FileRef, FolderRef, StoreError};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum WorkflowError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Routing(#[from] RoutingError),
    #[error("upload rejected: {0}")]
    InvalidUpload(String),
    /// Fail closed: a copy that must be protected is never written unprotected.
    #[error("password protection failed: {0}")]
    Protection(String),
    /// The document needs a destination that is not switched on yet.
    #[error("{0}")]
    NotSupported(String),
}

/// Upload validation from Pega (`AllowFileType`, `ValDocName`): PDF only, name <= 256.
#[derive(Clone, Debug)]
pub struct UploadRules {
    pub max_bytes: usize,
    pub max_name_chars: usize,
}

impl Default for UploadRules {
    fn default() -> Self {
        Self { max_bytes: 2 * 1024 * 1024, max_name_chars: 256 }
    }
}

pub fn validate_upload(name: &str, size: usize, rules: &UploadRules) -> Result<(), WorkflowError> {
    let invalid = |m: &str| Err(WorkflowError::InvalidUpload(m.to_string()));
    if name.trim().is_empty() {
        return invalid("file name is empty");
    }
    if name.chars().count() > rules.max_name_chars {
        return invalid("file name is longer than 256 characters");
    }
    if !name.to_lowercase().ends_with(".pdf") {
        return invalid("only PDF files are accepted");
    }
    if size == 0 {
        return invalid("file is empty");
    }
    if size > rules.max_bytes {
        return invalid("file is larger than the allowed size");
    }
    Ok(())
}

/// Phase 1, at case creation: the temp folder is named with the case id (e.g. `TD-1001`)
/// inside the client's Internal root.
pub async fn create_temp_folder(
    store: &dyn DocumentStore,
    internal_root: &FolderRef,
    case_label: &str,
) -> Result<FolderRef, WorkflowError> {
    Ok(store.ensure_folder(internal_root, case_label).await?)
}

/// Phase 1, per uploaded PDF: validate, then save straight into the temp folder.
pub async fn upload_to_temp(
    store: &dyn DocumentStore,
    temp: &FolderRef,
    name: &str,
    bytes: Vec<u8>,
    rules: &UploadRules,
) -> Result<FileRef, WorkflowError> {
    validate_upload(name, bytes.len(), rules)?;
    Ok(store.upload_file(temp, name, bytes).await?)
}

/// Encrypts a PDF with the client's password (the client record's Read Write Password).
/// `services::pdf_protection` will implement this.
pub trait PdfProtector: Send + Sync {
    fn protect(&self, pdf: &[u8]) -> Result<Vec<u8>, String>;
}

pub struct SubmitDocument {
    /// The file in the temp folder.
    pub temp_file: FileRef,
    /// Final name written at every destination (`Last, First - OriginalFileName.pdf`).
    pub target_name: String,
    pub year: i32,
    pub route: DocumentRoute,
    /// "Password Protect Client Copies?" = Yes.
    pub protect_client_copies: bool,
}

/// Where a document ended up. `None` for the internal copies of external-only types.
#[derive(Debug, PartialEq, Eq)]
pub struct Placed {
    /// `Year/<internal type folder>`. Never encrypted.
    pub internal_quick_view: Option<FileRef>,
    /// `Year/6. Released to Client/<type folder>`.
    pub internal_client_copy: Option<FileRef>,
    /// Client root `Year/<type folder>`. `None` while External filing is not switched on.
    pub external: Option<FileRef>,
}

pub struct Roots<'a> {
    /// Client's Internal folder (`TaxpayerData.InternalFolder`).
    pub internal: &'a FolderRef,
    /// Client's Client folder (`TaxpayerData.FolderName`). `None` = External filing is off: only
    /// the Internal copies are made, and a type that is filed only externally cannot be filed yet.
    pub external: Option<&'a FolderRef>,
    /// `6. Released to Client` (configurable; a hardcoded literal in Pega).
    pub released_folder_name: &'a str,
}

/// Folders already looked up or made while filing one request, so its documents share them
/// instead of asking storage for the same year and type folders once per document.
#[derive(Default)]
pub struct FolderCache {
    known: tokio::sync::Mutex<std::collections::HashMap<(String, String), FolderRef>>,
}

async fn ensure_path_cached(store: &dyn DocumentStore, cache: &FolderCache, root: &FolderRef, path: &[String]) -> Result<FolderRef, StoreError> {
    let mut cur = root.clone();
    for name in path {
        let key = (cur.0.clone(), name.to_lowercase());
        let hit = cache.known.lock().await.get(&key).cloned();
        cur = match hit {
            Some(folder) => folder,
            None => {
                let folder = store.ensure_folder(&cur, name).await?;
                cache.known.lock().await.insert(key, folder.clone());
                folder
            }
        };
    }
    Ok(cur)
}

async fn ensure_path(store: &dyn DocumentStore, root: &FolderRef, path: &[String]) -> Result<FolderRef, StoreError> {
    let mut cur = root.clone();
    for name in path {
        cur = store.ensure_folder(&cur, name).await?;
    }
    Ok(cur)
}

/// Writes the file, or returns the existing one when a retry finds it already placed.
async fn put(store: &dyn DocumentStore, folder: &FolderRef, name: &str, bytes: &[u8]) -> Result<FileRef, StoreError> {
    store.upload_or_existing(folder, name, bytes.to_vec()).await
}

/// A document read from the temp folder, with its client copy already protected if required.
/// Nothing has been written to a permanent folder yet.
pub struct Prepared {
    pub original: Vec<u8>,
    pub client_copy: Vec<u8>,
    pub target_name: String,
    pub year: i32,
    pub route: DocumentRoute,
}

/// "Processing documents": read the file and, where the user asked for it, password-protect the
/// client copy. Nothing is written yet, so a protection failure leaves every folder untouched.
pub async fn prepare_document(
    store: &dyn DocumentStore,
    protector: &dyn PdfProtector,
    doc: &SubmitDocument,
) -> Result<Prepared, WorkflowError> {
    let original = store.read_file(&doc.temp_file).await?;
    let client_copy = if doc.protect_client_copies {
        protector.protect(&original).map_err(WorkflowError::Protection)?
    } else {
        original.clone()
    };
    Ok(Prepared { original, client_copy, target_name: doc.target_name.clone(), year: doc.year, route: doc.route.clone() })
}

/// Phases 2 + 3 for one document: protect, then write. Protection runs first so nothing is written
/// if it fails.
pub async fn place_document(
    store: &dyn DocumentStore,
    protector: &dyn PdfProtector,
    roots: &Roots<'_>,
    doc: &SubmitDocument,
) -> Result<Placed, WorkflowError> {
    let prepared = prepare_document(store, protector, doc).await?;
    place_prepared(store, roots, &prepared).await
}

/// "Moving documents": create or reuse the folders, then copy the file to each destination.
pub async fn place_prepared(store: &dyn DocumentStore, roots: &Roots<'_>, doc: &Prepared) -> Result<Placed, WorkflowError> {
    place_prepared_cached(store, roots, doc, &FolderCache::default()).await
}

/// Like `place_prepared`, with a cache shared by all of one request's documents.
pub async fn place_prepared_cached(store: &dyn DocumentStore, roots: &Roots<'_>, doc: &Prepared, cache: &FolderCache) -> Result<Placed, WorkflowError> {
    let plan = plan_for(doc.year, &doc.route, roots.released_folder_name)?;
    let (original, client_copy) = (&doc.original, &doc.client_copy);

    let mut quick_view = None;
    let mut released = None;
    if let Some(path) = &plan.internal {
        let folder = ensure_path_cached(store, cache, roots.internal, path).await?;
        quick_view = Some(put(store, &folder, &doc.target_name, original).await?);
    }
    if let Some(path) = &plan.released {
        let folder = ensure_path_cached(store, cache, roots.internal, path).await?;
        released = Some(put(store, &folder, &doc.target_name, client_copy).await?);
    }
    let external = match roots.external {
        Some(external_root) => {
            let ext_folder = ensure_path_cached(store, cache, external_root, &plan.external).await?;
            Some(put(store, &ext_folder, &doc.target_name, client_copy).await?)
        }
        // A type with no internal side has nowhere to go until External filing is switched on.
        None if plan.internal.is_none() => {
            return Err(WorkflowError::NotSupported(format!(
                "{} is filed only to the client's External folder, which is not switched on yet",
                doc.route.document_type
            )))
        }
        None => None,
    };
    Ok(Placed { internal_quick_view: quick_view, internal_client_copy: released, external })
}

/// One result per document, in order, so a failure on one file doesn't hide the others.
pub async fn place_documents(
    store: &dyn DocumentStore,
    protector: &dyn PdfProtector,
    roots: &Roots<'_>,
    docs: &[SubmitDocument],
) -> Vec<Result<Placed, WorkflowError>> {
    let mut out = Vec::with_capacity(docs.len());
    for d in docs {
        out.push(place_document(store, protector, roots, d).await);
    }
    out
}

/// Phase 4: deletes the temp folder, but only when every document was placed.
pub async fn clean_temp(
    store: &dyn DocumentStore,
    temp: &FolderRef,
    results: &[Result<Placed, WorkflowError>],
) -> Result<bool, WorkflowError> {
    if results.iter().any(|r| r.is_err()) {
        return Ok(false);
    }
    store.delete_folder(temp).await?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::super::memory::InMemoryDocumentStore;
    use super::super::routing::find_route;
    use super::*;

    struct Prefix;
    impl PdfProtector for Prefix {
        fn protect(&self, pdf: &[u8]) -> Result<Vec<u8>, String> {
            Ok([b"ENC:".as_slice(), pdf].concat())
        }
    }
    struct Failing;
    impl PdfProtector for Failing {
        fn protect(&self, _: &[u8]) -> Result<Vec<u8>, String> {
            Err("no password".into())
        }
    }

    fn routes() -> Vec<DocumentRoute> {
        let r = |t: &str, i: Option<&str>, e: &str, x: bool| DocumentRoute {
            document_type: t.into(),
            internal_folder: i.map(String::from),
            external_folder: e.into(),
            is_external_only: x,
        };
        vec![
            r("K-1", Some("4. Tax Documents"), "K-1s", false),
            r("Draft K-1", Some("4. Tax Documents"), "K-1s", false),
            r("Tax Return", Some("7. Tax Returns"), "Tax Returns", false),
            r("Income Statement", Some("Loan Assistance"), "Loan Assistance", false),
            r("Balance Sheet", Some("Loan Assistance"), "Loan Assistance", false),
            r("S-Corp Election", None, "Signature Requests", true),
            r("Form 8308", None, "K-1s", true),
            r("Extensions", None, "Tax Returns", true),
        ]
    }

    struct Fixture {
        store: InMemoryDocumentStore,
        internal: FolderRef,
        client: FolderRef,
    }
    fn fixture() -> Fixture {
        let store = InMemoryDocumentStore::new();
        let internal = store.create_root("Moore, Andrea - Internal");
        let client = store.create_root("Moore, Andrea - Client");
        Fixture { store, internal, client }
    }

    async fn staged(f: &Fixture, temp: &FolderRef, name: &str, ty: &str, year: i32, protect: bool) -> SubmitDocument {
        let file = upload_to_temp(&f.store, temp, name, b"%PDF-1.7 data".to_vec(), &UploadRules::default()).await.unwrap();
        SubmitDocument {
            temp_file: file,
            target_name: format!("Moore, Andrea - {name}"),
            year,
            route: find_route(&routes(), ty).unwrap().clone(),
            protect_client_copies: protect,
        }
    }

    const RELEASED: &str = "6. Released to Client";

    #[tokio::test]
    async fn k1_2025_matches_the_pega_worked_example() {
        let f = fixture();
        let temp = create_temp_folder(&f.store, &f.internal, "TD-1001").await.unwrap();
        let doc = staged(&f, &temp, "K1_2025.pdf", "K-1", 2025, true).await;
        // Before submit: only the temp folder holds the PDF.
        assert_eq!(f.store.file_paths(), vec!["Moore, Andrea - Internal/TD-1001/K1_2025.pdf"]);

        let roots = Roots { internal: &f.internal, external: Some(&f.client), released_folder_name: RELEASED };
        let results = place_documents(&f.store, &Prefix, &roots, std::slice::from_ref(&doc)).await;
        assert!(results[0].is_ok());
        assert!(clean_temp(&f.store, &temp, &results).await.unwrap());

        assert_eq!(
            f.store.file_paths(),
            vec![
                "Moore, Andrea - Client/2025/K-1s/Moore, Andrea - K1_2025.pdf",
                "Moore, Andrea - Internal/2025/4. Tax Documents/Moore, Andrea - K1_2025.pdf",
                "Moore, Andrea - Internal/2025/6. Released to Client/K-1s/Moore, Andrea - K1_2025.pdf",
            ]
        );
        assert!(!f.store.folder_paths().iter().any(|p| p.contains("TD-1001")), "temp folder is deleted");
    }

    #[tokio::test]
    async fn external_only_types_build_only_the_client_side() {
        let f = fixture();
        let temp = create_temp_folder(&f.store, &f.internal, "TD-1002").await.unwrap();
        let doc = staged(&f, &temp, "election.pdf", "S-Corp Election", 2024, false).await;
        let roots = Roots { internal: &f.internal, external: Some(&f.client), released_folder_name: RELEASED };
        let r = place_document(&f.store, &Prefix, &roots, &doc).await.unwrap();
        assert!(r.internal_quick_view.is_none() && r.internal_client_copy.is_none());
        let paths = f.store.file_paths();
        assert!(paths.contains(&"Moore, Andrea - Client/2024/Signature Requests/Moore, Andrea - election.pdf".to_string()));
        assert!(!f.store.folder_paths().iter().any(|p| p == "Moore, Andrea - Internal/2024"));
    }

    #[tokio::test]
    async fn quick_view_is_plain_and_client_copies_are_protected() {
        let f = fixture();
        let temp = create_temp_folder(&f.store, &f.internal, "TD-1003").await.unwrap();
        let doc = staged(&f, &temp, "return.pdf", "Tax Return", 2025, true).await;
        let roots = Roots { internal: &f.internal, external: Some(&f.client), released_folder_name: RELEASED };
        let r = place_document(&f.store, &Prefix, &roots, &doc).await.unwrap();
        assert_eq!(f.store.read_file(&r.internal_quick_view.unwrap()).await.unwrap(), b"%PDF-1.7 data");
        assert_eq!(f.store.read_file(&r.external.unwrap()).await.unwrap(), b"ENC:%PDF-1.7 data");
        assert_eq!(f.store.read_file(&r.internal_client_copy.unwrap()).await.unwrap(), b"ENC:%PDF-1.7 data");
    }

    #[tokio::test]
    async fn with_external_filing_off_only_the_internal_copies_are_made() {
        let f = fixture();
        let temp = create_temp_folder(&f.store, &f.internal, "TD-1006").await.unwrap();
        let doc = staged(&f, &temp, "k1.pdf", "K-1", 2025, false).await;
        let roots = Roots { internal: &f.internal, external: None, released_folder_name: RELEASED };
        let placed = place_document(&f.store, &Prefix, &roots, &doc).await.unwrap();
        assert!(placed.external.is_none());
        let paths = f.store.file_paths();
        assert!(paths.contains(&"Moore, Andrea - Internal/2025/4. Tax Documents/Moore, Andrea - k1.pdf".to_string()));
        assert!(paths.contains(&"Moore, Andrea - Internal/2025/6. Released to Client/K-1s/Moore, Andrea - k1.pdf".to_string()));
        assert!(!paths.iter().any(|p| p.starts_with("Moore, Andrea - Client/")), "nothing is written to the client folder");
    }

    #[tokio::test]
    async fn an_external_only_type_cannot_be_filed_while_external_filing_is_off() {
        let f = fixture();
        let temp = create_temp_folder(&f.store, &f.internal, "TD-1007").await.unwrap();
        let doc = staged(&f, &temp, "election.pdf", "S-Corp Election", 2025, false).await;
        let roots = Roots { internal: &f.internal, external: None, released_folder_name: RELEASED };
        let err = place_document(&f.store, &Prefix, &roots, &doc).await.unwrap_err();
        assert!(matches!(err, WorkflowError::NotSupported(_)));
        assert_eq!(f.store.file_paths(), vec!["Moore, Andrea - Internal/TD-1007/election.pdf"], "nothing was written");
    }

    #[tokio::test]
    async fn protection_failure_writes_nothing_and_keeps_the_temp_folder() {
        let f = fixture();
        let temp = create_temp_folder(&f.store, &f.internal, "TD-1004").await.unwrap();
        let doc = staged(&f, &temp, "k1.pdf", "K-1", 2025, true).await;
        let roots = Roots { internal: &f.internal, external: Some(&f.client), released_folder_name: RELEASED };
        let results = place_documents(&f.store, &Failing, &roots, std::slice::from_ref(&doc)).await;
        assert!(matches!(results[0], Err(WorkflowError::Protection(_))));
        assert!(!clean_temp(&f.store, &temp, &results).await.unwrap());
        assert_eq!(f.store.file_paths(), vec!["Moore, Andrea - Internal/TD-1004/k1.pdf"]);
    }

    #[tokio::test]
    async fn submit_is_idempotent_and_shares_year_folders() {
        let f = fixture();
        let temp = create_temp_folder(&f.store, &f.internal, "TD-1005").await.unwrap();
        let a = staged(&f, &temp, "a.pdf", "K-1", 2025, false).await;
        let b = staged(&f, &temp, "b.pdf", "Draft K-1", 2025, false).await;
        let roots = Roots { internal: &f.internal, external: Some(&f.client), released_folder_name: RELEASED };
        let docs = [a, b];
        let first = place_documents(&f.store, &Prefix, &roots, &docs).await;
        let again = place_documents(&f.store, &Prefix, &roots, &docs).await;
        assert!(first.iter().chain(again.iter()).all(|r| r.is_ok()));
        assert_eq!(first[0], again[0], "a retry returns the files already placed");
        let years = f.store.folder_paths().iter().filter(|p| p.ends_with("/2025")).count();
        assert_eq!(years, 2, "one 2025 folder per root, shared by both documents");
        assert_eq!(f.store.file_paths().len(), 2 + 2 * 3, "temp copies of a and b plus 3 destinations each");
    }

    #[test]
    fn upload_rules_follow_pega() {
        let r = UploadRules::default();
        assert!(validate_upload("a.PDF", 10, &r).is_ok());
        assert!(validate_upload("a.docx", 10, &r).is_err());
        assert!(validate_upload(&format!("{}.pdf", "x".repeat(260)), 10, &r).is_err());
        assert!(validate_upload("a.pdf", 0, &r).is_err());
        assert!(validate_upload("a.pdf", r.max_bytes + 1, &r).is_err());
    }

    #[test]
    fn routing_covers_every_type_and_normalises_keys() {
        let rs = routes();
        for r in &rs {
            assert!(plan_for(2025, r, RELEASED).is_ok(), "{}", r.document_type);
        }
        assert_eq!(find_route(&rs, "k 1").unwrap().document_type, "K-1");
        assert_eq!(find_route(&rs, "scorp election").unwrap().document_type, "S-Corp Election");
        assert!(matches!(find_route(&rs, "Mystery"), Err(RoutingError::UnknownDocumentType(_))));
        assert!(matches!(plan_for(1800, &rs[0], "x"), Err(RoutingError::InvalidYear(_))));
    }
}
