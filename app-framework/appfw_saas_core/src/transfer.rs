use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SaasTransferCaps {
    pub max_chunk_bytes: u64,
    pub max_total_bytes: u64,
    pub max_chunks: u32,
}

impl SaasTransferCaps {
    pub const DEFAULT_MAX_CHUNK_BYTES: u64 = 5_242_880;
    pub const DEFAULT_MAX_TOTAL_BYTES: u64 = 104_857_600;
    pub const DEFAULT_MAX_CHUNKS: u32 = 1_000;

    pub fn validate(self) -> Result<(), String> {
        if self.max_chunk_bytes == 0 {
            return Err("max_chunk_bytes must be greater than 0".to_string());
        }
        if self.max_total_bytes == 0 {
            return Err("max_total_bytes must be greater than 0".to_string());
        }
        if self.max_chunks == 0 {
            return Err("max_chunks must be greater than 0".to_string());
        }
        if self.max_chunk_bytes > self.max_total_bytes {
            return Err("max_chunk_bytes must not exceed max_total_bytes".to_string());
        }
        Ok(())
    }
}

impl Default for SaasTransferCaps {
    fn default() -> Self {
        Self {
            max_chunk_bytes: Self::DEFAULT_MAX_CHUNK_BYTES,
            max_total_bytes: Self::DEFAULT_MAX_TOTAL_BYTES,
            max_chunks: Self::DEFAULT_MAX_CHUNKS,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChunkDownloadContinuation {
    pub chunk_set_id: String,
    pub chunk_id: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_chunks: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum_sha256: Option<String>,
}

impl ChunkDownloadContinuation {
    pub fn new(chunk_set_id: impl Into<String>, chunk_id: u32) -> Self {
        Self {
            chunk_set_id: chunk_set_id.into(),
            chunk_id,
            total_chunks: None,
            total_bytes: None,
            checksum_sha256: None,
        }
    }

    pub fn with_total_chunks(mut self, total_chunks: u32) -> Self {
        self.total_chunks = Some(total_chunks);
        self
    }

    pub fn with_total_bytes(mut self, total_bytes: u64) -> Self {
        self.total_bytes = Some(total_bytes);
        self
    }

    pub fn with_checksum_sha256(mut self, checksum_sha256: impl Into<String>) -> Self {
        self.checksum_sha256 = Some(checksum_sha256.into());
        self
    }

    pub fn next_chunk(&self) -> Self {
        Self {
            chunk_set_id: self.chunk_set_id.clone(),
            chunk_id: self.chunk_id.saturating_add(1),
            total_chunks: self.total_chunks,
            total_bytes: self.total_bytes,
            checksum_sha256: self.checksum_sha256.clone(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_non_empty_single_line(
            "chunk download continuation chunk_set_id",
            &self.chunk_set_id,
        )?;

        if let Some(total_chunks) = self.total_chunks {
            if total_chunks == 0 {
                return Err(
                    "chunk download continuation total_chunks must be greater than 0".to_string(),
                );
            }
            if self.chunk_id >= total_chunks {
                return Err(
                    "chunk download continuation chunk_id must be less than total_chunks"
                        .to_string(),
                );
            }
        }

        if self.total_bytes == Some(0) {
            return Err(
                "chunk download continuation total_bytes must be greater than 0".to_string(),
            );
        }

        if let Some(checksum) = &self.checksum_sha256 {
            validate_non_empty_single_line("chunk download continuation checksum", checksum)?;
        }

        Ok(())
    }
}

fn validate_non_empty_single_line(name: &'static str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{name} must not be empty"));
    }

    if value.contains('\n') || value.contains('\r') || value.chars().any(char::is_control) {
        return Err(format!("{name} must be a single-line value"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn transfer_caps_require_positive_limits() {
        SaasTransferCaps::default()
            .validate()
            .expect("default caps");

        assert!(SaasTransferCaps {
            max_chunk_bytes: 0,
            ..SaasTransferCaps::default()
        }
        .validate()
        .unwrap_err()
        .contains("max_chunk_bytes"));
        assert!(SaasTransferCaps {
            max_total_bytes: 0,
            ..SaasTransferCaps::default()
        }
        .validate()
        .unwrap_err()
        .contains("max_total_bytes"));
        assert!(SaasTransferCaps {
            max_chunks: 0,
            ..SaasTransferCaps::default()
        }
        .validate()
        .unwrap_err()
        .contains("max_chunks"));
        assert!(SaasTransferCaps {
            max_chunk_bytes: 2,
            max_total_bytes: 1,
            max_chunks: 1,
        }
        .validate()
        .unwrap_err()
        .contains("max_total_bytes"));
    }

    #[test]
    fn chunk_download_continuation_advances_without_losing_metadata() {
        let next = ChunkDownloadContinuation::new("import-dump-1", 0)
            .with_total_chunks(3)
            .with_total_bytes(1_024)
            .with_checksum_sha256("abc123")
            .next_chunk();

        assert_eq!(next.chunk_set_id, "import-dump-1");
        assert_eq!(next.chunk_id, 1);
        assert_eq!(next.total_chunks, Some(3));
        assert_eq!(next.total_bytes, Some(1_024));
        assert_eq!(next.checksum_sha256.as_deref(), Some("abc123"));
        next.validate().expect("next chunk is valid");
    }

    #[test]
    fn chunk_download_continuation_serializes_transfer_metadata() {
        let continuation = ChunkDownloadContinuation::new("process-dump-1", 0)
            .with_total_chunks(2)
            .with_total_bytes(2_048)
            .with_checksum_sha256("abc123");

        let serialized = serde_json::to_value(&continuation).expect("serializes");

        assert_eq!(
            serialized,
            json!({
                "chunk_set_id": "process-dump-1",
                "chunk_id": 0,
                "total_chunks": 2,
                "total_bytes": 2048,
                "checksum_sha256": "abc123",
            })
        );

        let round_trip: ChunkDownloadContinuation =
            serde_json::from_value(serialized).expect("deserializes");
        assert_eq!(round_trip, continuation);
    }

    #[test]
    fn chunk_download_continuation_rejects_invalid_metadata() {
        assert!(ChunkDownloadContinuation::new("", 0)
            .validate()
            .unwrap_err()
            .contains("chunk_set_id"));
        assert!(ChunkDownloadContinuation::new("dump-1", 3)
            .with_total_chunks(3)
            .validate()
            .unwrap_err()
            .contains("chunk_id"));
        assert!(ChunkDownloadContinuation::new("dump-1", 0)
            .with_total_chunks(0)
            .validate()
            .unwrap_err()
            .contains("total_chunks"));
        assert!(ChunkDownloadContinuation::new("dump-1", 0)
            .with_total_bytes(0)
            .validate()
            .unwrap_err()
            .contains("total_bytes"));
        assert!(ChunkDownloadContinuation::new("dump-1", 0)
            .with_checksum_sha256("\n")
            .validate()
            .unwrap_err()
            .contains("checksum"));
    }
}
