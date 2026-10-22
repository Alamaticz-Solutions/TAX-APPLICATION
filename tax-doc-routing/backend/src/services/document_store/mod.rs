//! `DocumentStore`: the neutral document-storage boundary (design §4.3).
//!
//! Nothing above this trait knows whether bytes live in Box, SharePoint or memory. The
//! Pega-derived workflow (`workflow`) and folder rules (`routing`) are written only against it,
//! so each storage vendor is one implementation behind it: `memory` (dev, tests, demo) and
//! `box_store` (Box, first target). SharePoint can be added later the same way.
//! See `docs/architecture/pega-box-to-sharepoint-implementation-plan.md`.
//!
//! Not wired to any handler or screen yet: nothing in the running app calls this module.

#![allow(dead_code)]

pub mod box_store;
pub mod latency;
pub mod memory;
pub mod routing;
pub mod workflow;

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;

use crate::schemas::system::DataSource;

/// Opaque handle to a folder in the store. Callers never build or parse these.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FolderRef(pub String);

/// Opaque handle to a file in the store.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FileRef(pub String);

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum StoreError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("already exists: {0}")]
    AlreadyExists(String),
    #[error("invalid name: {0}")]
    InvalidName(String),
    /// Storage could not be reached or refused us (network, sign-in, rate limit, server error).
    /// A retry may succeed.
    #[error("storage unavailable: {0}")]
    Unavailable(String),
}

/// The operations the Pega process needs, described without a vendor name.
#[async_trait]
pub trait DocumentStore: Send + Sync {
    /// Get-or-create a child folder. Idempotent, so repeating a step never duplicates folders
    /// (Pega's `CreateBoxInheritFolder` was not: a retry made duplicates).
    async fn ensure_folder(&self, parent: &FolderRef, name: &str) -> Result<FolderRef, StoreError>;
    /// Create a folder that must not exist yet: `AlreadyExists` if the name is taken. Used for a
    /// request's temp folder, which is later deleted, so it must never be a folder somebody else
    /// (for example a Pega case's `TD-2797`) already owns.
    async fn create_folder(&self, parent: &FolderRef, name: &str) -> Result<FolderRef, StoreError>;
    /// Write a file. Fails with `AlreadyExists` rather than overwriting.
    async fn upload_file(&self, parent: &FolderRef, name: &str, bytes: Vec<u8>) -> Result<FileRef, StoreError>;
    /// Write a file, or return the one already there under that name (never overwrites). One
    /// storage call when the name is free, which is what filing needs: it replaces "look for the
    /// file, then upload" (a listing of the folder per file) and makes a repeat harmless.
    async fn upload_or_existing(&self, parent: &FolderRef, name: &str, bytes: Vec<u8>) -> Result<FileRef, StoreError> {
        match self.upload_file(parent, name, bytes).await {
            Err(StoreError::AlreadyExists(_)) => self.find_file(parent, name).await?.ok_or_else(|| StoreError::AlreadyExists(name.to_string())),
            other => other,
        }
    }
    /// The "same-named file already exists" check.
    async fn find_file(&self, parent: &FolderRef, name: &str) -> Result<Option<FileRef>, StoreError>;
    async fn read_file(&self, file: &FileRef) -> Result<Vec<u8>, StoreError>;
    /// Delete a folder and everything under it (used only for the case temp folder).
    async fn delete_folder(&self, folder: &FolderRef) -> Result<(), StoreError>;
    /// Is there a folder with this id? Pega's `CheckClientBoxLocation`: checked before filing, so a
    /// wrong client folder fails the request up front instead of half way through.
    async fn folder_exists(&self, folder: &FolderRef) -> Result<bool, StoreError>;
    /// Turns the folder value kept on a client (a URL or an id, as in Pega) into a handle.
    fn parse_folder(&self, value: &str) -> Result<FolderRef, StoreError> {
        folder_id_from_url(value)
            .map(FolderRef)
            .ok_or_else(|| StoreError::InvalidName(format!("'{}' is not a folder id or URL", value.trim())))
    }
}

/// Picks the store from `STORAGE_PROVIDER`: unset or `memory` (the default, so nothing changes
/// until Box is switched on) or `box`. `box` needs the `BOX_*` settings and fails closed without
/// them. The Box token is kept in the `storage_tokens` table through `sources`' Postgres.
pub fn build_store(sources: &HashMap<String, DataSource>) -> Result<Arc<dyn DocumentStore>, String> {
    let provider = std::env::var("STORAGE_PROVIDER").unwrap_or_default().trim().to_ascii_lowercase();
    match provider.as_str() {
        "" | "memory" => {
            let store: Arc<dyn DocumentStore> = Arc::new(memory::InMemoryDocumentStore::lenient());
            // A performance-testing aid (see latency.rs): wait this long before every storage call.
            let delay_ms = std::env::var("STORAGE_SIMULATED_LATENCY_MS").ok().and_then(|v| v.trim().parse::<u64>().ok()).unwrap_or(0);
            if delay_ms > 0 {
                tracing::warn!(delay_ms, "STORAGE_SIMULATED_LATENCY_MS is set: every storage call is slowed down on purpose");
                Ok(Arc::new(latency::LatencyStore::new(store, std::time::Duration::from_millis(delay_ms))))
            } else {
                Ok(store)
            }
        }
        "box" => {
            let config = box_store::BoxConfig::from_env()?
                .ok_or_else(|| "STORAGE_PROVIDER=box but the BOX_* settings are missing".to_string())?;
            let tokens: Arc<dyn box_store::TokenStore> =
                box_store::PostgresTokenStore::init(sources).map_err(|e| format!("token table unavailable: {e}"))?;
            Ok(Arc::new(box_store::BoxDocumentStore::from_config(config, Some(tokens))?))
        }
        other => Err(format!("unknown STORAGE_PROVIDER '{other}' (use memory or box)")),
    }
}

/// Characters no store accepts in a folder or file name (the union of Box, SharePoint and Windows).
pub(crate) fn validate_name(name: &str) -> Result<(), StoreError> {
    let bad = name.trim().is_empty()
        || name.chars().any(|c| c.is_control() || "/\\:*?\"<>|".contains(c))
        || name.ends_with('.')
        || name.ends_with(' ')
        || name == "."
        || name == "..";
    if bad {
        Err(StoreError::InvalidName(name.to_string()))
    } else {
        Ok(())
    }
}

/// Pega reads a folder id as everything after the last `/` of the stored Box URL
/// (`@whatComesAfterLast(TaxpayerData.InternalFolder, '/')`). Accepts a URL or a bare id.
pub fn folder_id_from_url(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let no_query = trimmed.split(['?', '#']).next().unwrap_or(trimmed).trim_end_matches('/');
    let id = no_query.rsplit('/').next().unwrap_or(no_query);
    if !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) {
        Some(id.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_id_is_taken_after_the_last_slash() {
        assert_eq!(folder_id_from_url("https://app.box.com/folder/123456789"), Some("123456789".into()));
        assert_eq!(folder_id_from_url("https://app.box.com/folder/123456789/"), Some("123456789".into()));
        assert_eq!(folder_id_from_url("https://app.box.com/folder/123456789?s=abc#x"), Some("123456789".into()));
        assert_eq!(folder_id_from_url("84473772615"), Some("84473772615".into()));
        assert_eq!(folder_id_from_url("/Clients/Smith_John"), None);
        assert_eq!(folder_id_from_url(""), None);
    }

    #[test]
    fn names_with_forbidden_characters_are_refused() {
        assert!(validate_name("2025").is_ok());
        assert!(validate_name("6. Released to Client").is_ok());
        assert!(validate_name("Moore, Andrea - Pega AI.pdf").is_ok());
        for bad in ["", " ", "a/b", "a\\b", "a:b", "a*b", "what?", "a|b", "trailing.", "trailing ", ".."] {
            assert!(validate_name(bad).is_err(), "{bad:?}");
        }
    }
}
