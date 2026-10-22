use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;

use super::{folder_id_from_url, validate_name, DocumentStore, FileRef, FolderRef, StoreError};

#[derive(Default)]
struct State {
    next: u64,
    /// id -> (parent id, name)
    folders: HashMap<String, (Option<String>, String)>,
    /// id -> (parent id, name, bytes)
    files: HashMap<String, (String, String, Vec<u8>)>,
}

impl State {
    fn id(&mut self, prefix: &str) -> String {
        self.next += 1;
        format!("{prefix}-{}", self.next)
    }

    fn path_of_folder(&self, id: &str) -> String {
        let mut parts = Vec::new();
        let mut cur = Some(id.to_string());
        while let Some(c) = cur {
            match self.folders.get(&c) {
                Some((parent, name)) => {
                    parts.push(name.clone());
                    cur = parent.clone();
                }
                None => break,
            }
        }
        parts.reverse();
        parts.join("/")
    }
}

/// Dev/test/demo store. Names are case-insensitive, like Box and SharePoint.
///
/// `new()` is strict: a parent folder must have been created. `lenient()` is what the running app
/// uses until Box is switched on: any folder id a client record names is treated as an existing
/// root, so the whole upload and filing flow works without a real storage account.
#[derive(Default)]
pub struct InMemoryDocumentStore {
    state: Mutex<State>,
    lenient: bool,
}

impl InMemoryDocumentStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn lenient() -> Self {
        Self { lenient: true, ..Self::default() }
    }

    /// In lenient mode an unknown parent becomes a root folder of that name.
    fn known_or_root(&self, s: &mut State, id: &str) -> bool {
        if s.folders.contains_key(id) {
            return true;
        }
        if self.lenient {
            s.folders.insert(id.to_string(), (None, id.to_string()));
            return true;
        }
        false
    }

    /// A top-level folder, standing in for a client's Internal / Client root.
    pub fn create_root(&self, name: &str) -> FolderRef {
        let mut s = self.state.lock().unwrap();
        let id = s.id("folder");
        s.folders.insert(id.clone(), (None, name.to_string()));
        FolderRef(id)
    }

    /// Every file as `Root/Folder/.../name`, sorted. For assertions.
    pub fn file_paths(&self) -> Vec<String> {
        let s = self.state.lock().unwrap();
        let mut out: Vec<String> = s
            .files
            .values()
            .map(|(parent, name, _)| format!("{}/{}", s.path_of_folder(parent), name))
            .collect();
        out.sort();
        out
    }

    /// Every folder path, sorted. For assertions.
    pub fn folder_paths(&self) -> Vec<String> {
        let s = self.state.lock().unwrap();
        let mut out: Vec<String> = s.folders.keys().map(|id| s.path_of_folder(id)).collect();
        out.sort();
        out
    }
}

#[async_trait]
impl DocumentStore for InMemoryDocumentStore {
    async fn ensure_folder(&self, parent: &FolderRef, name: &str) -> Result<FolderRef, StoreError> {
        validate_name(name)?;
        let mut s = self.state.lock().unwrap();
        if !self.known_or_root(&mut s, &parent.0) {
            return Err(StoreError::NotFound(parent.0.clone()));
        }
        let existing = s
            .folders
            .iter()
            .find(|(_, (p, n))| p.as_deref() == Some(parent.0.as_str()) && n.eq_ignore_ascii_case(name))
            .map(|(id, _)| id.clone());
        if let Some(id) = existing {
            return Ok(FolderRef(id));
        }
        let id = s.id("folder");
        s.folders.insert(id.clone(), (Some(parent.0.clone()), name.to_string()));
        Ok(FolderRef(id))
    }

    async fn create_folder(&self, parent: &FolderRef, name: &str) -> Result<FolderRef, StoreError> {
        validate_name(name)?;
        let mut s = self.state.lock().unwrap();
        if !self.known_or_root(&mut s, &parent.0) {
            return Err(StoreError::NotFound(parent.0.clone()));
        }
        if s.folders.values().any(|(p, n)| p.as_deref() == Some(parent.0.as_str()) && n.eq_ignore_ascii_case(name)) {
            return Err(StoreError::AlreadyExists(name.to_string()));
        }
        let id = s.id("folder");
        s.folders.insert(id.clone(), (Some(parent.0.clone()), name.to_string()));
        Ok(FolderRef(id))
    }

    async fn upload_file(&self, parent: &FolderRef, name: &str, bytes: Vec<u8>) -> Result<FileRef, StoreError> {
        validate_name(name)?;
        let mut s = self.state.lock().unwrap();
        if !self.known_or_root(&mut s, &parent.0) {
            return Err(StoreError::NotFound(parent.0.clone()));
        }
        if s.files.values().any(|(p, n, _)| p == &parent.0 && n.eq_ignore_ascii_case(name)) {
            return Err(StoreError::AlreadyExists(name.to_string()));
        }
        let id = s.id("file");
        s.files.insert(id.clone(), (parent.0.clone(), name.to_string(), bytes));
        Ok(FileRef(id))
    }

    async fn find_file(&self, parent: &FolderRef, name: &str) -> Result<Option<FileRef>, StoreError> {
        let s = self.state.lock().unwrap();
        Ok(s.files
            .iter()
            .find(|(_, (p, n, _))| p == &parent.0 && n.eq_ignore_ascii_case(name))
            .map(|(id, _)| FileRef(id.clone())))
    }

    async fn read_file(&self, file: &FileRef) -> Result<Vec<u8>, StoreError> {
        let s = self.state.lock().unwrap();
        s.files
            .get(&file.0)
            .map(|(_, _, b)| b.clone())
            .ok_or_else(|| StoreError::NotFound(file.0.clone()))
    }

    async fn folder_exists(&self, folder: &FolderRef) -> Result<bool, StoreError> {
        Ok(self.lenient || self.state.lock().unwrap().folders.contains_key(&folder.0))
    }

    fn parse_folder(&self, value: &str) -> Result<FolderRef, StoreError> {
        if self.lenient {
            let v = value.trim();
            return if v.is_empty() {
                Err(StoreError::InvalidName("the client has no folder set".into()))
            } else {
                Ok(FolderRef(folder_id_from_url(v).unwrap_or_else(|| v.to_string())))
            };
        }
        folder_id_from_url(value)
            .map(FolderRef)
            .ok_or_else(|| StoreError::InvalidName(format!("'{}' is not a folder id or URL", value.trim())))
    }

    async fn delete_folder(&self, folder: &FolderRef) -> Result<(), StoreError> {
        let mut s = self.state.lock().unwrap();
        if !s.folders.contains_key(&folder.0) {
            return Err(StoreError::NotFound(folder.0.clone()));
        }
        let mut doomed = vec![folder.0.clone()];
        let mut i = 0;
        while i < doomed.len() {
            let current = doomed[i].clone();
            let children: Vec<String> = s
                .folders
                .iter()
                .filter(|(_, (p, _))| p.as_deref() == Some(current.as_str()))
                .map(|(id, _)| id.clone())
                .collect();
            doomed.extend(children);
            i += 1;
        }
        s.files.retain(|_, (p, _, _)| !doomed.contains(p));
        for id in &doomed {
            s.folders.remove(id);
        }
        Ok(())
    }
}
