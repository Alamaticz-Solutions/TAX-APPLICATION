//! A performance-testing aid: wraps a store and waits a fixed time before every call, standing in
//! for the network round trip a real storage service (Box) costs. Switched on only by setting
//! `STORAGE_SIMULATED_LATENCY_MS` with the in-memory store; it is never used otherwise.
//!
//! It also counts the calls it passes on and logs the running total every 100 calls, so a test
//! run can say how many storage requests a batch really needs.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;

use super::{DocumentStore, FileRef, FolderRef, StoreError};

pub struct LatencyStore {
    inner: Arc<dyn DocumentStore>,
    delay: Duration,
    calls: AtomicU64,
}

impl LatencyStore {
    pub fn new(inner: Arc<dyn DocumentStore>, delay: Duration) -> Self {
        Self { inner, delay, calls: AtomicU64::new(0) }
    }

    async fn pause(&self) {
        let n = self.calls.fetch_add(1, Ordering::Relaxed) + 1;
        if n % 100 == 0 {
            tracing::info!(calls = n, "simulated storage: calls so far");
        }
        tokio::time::sleep(self.delay).await;
    }
}

#[async_trait]
impl DocumentStore for LatencyStore {
    async fn ensure_folder(&self, parent: &FolderRef, name: &str) -> Result<FolderRef, StoreError> {
        self.pause().await;
        self.inner.ensure_folder(parent, name).await
    }
    async fn create_folder(&self, parent: &FolderRef, name: &str) -> Result<FolderRef, StoreError> {
        self.pause().await;
        self.inner.create_folder(parent, name).await
    }
    async fn upload_file(&self, parent: &FolderRef, name: &str, bytes: Vec<u8>) -> Result<FileRef, StoreError> {
        self.pause().await;
        self.inner.upload_file(parent, name, bytes).await
    }
    async fn upload_or_existing(&self, parent: &FolderRef, name: &str, bytes: Vec<u8>) -> Result<FileRef, StoreError> {
        self.pause().await;
        self.inner.upload_or_existing(parent, name, bytes).await
    }
    async fn find_file(&self, parent: &FolderRef, name: &str) -> Result<Option<FileRef>, StoreError> {
        self.pause().await;
        self.inner.find_file(parent, name).await
    }
    async fn read_file(&self, file: &FileRef) -> Result<Vec<u8>, StoreError> {
        self.pause().await;
        self.inner.read_file(file).await
    }
    async fn delete_folder(&self, folder: &FolderRef) -> Result<(), StoreError> {
        self.pause().await;
        self.inner.delete_folder(folder).await
    }
    async fn folder_exists(&self, folder: &FolderRef) -> Result<bool, StoreError> {
        self.pause().await;
        self.inner.folder_exists(folder).await
    }
    fn parse_folder(&self, value: &str) -> Result<FolderRef, StoreError> {
        self.inner.parse_folder(value)
    }
}
