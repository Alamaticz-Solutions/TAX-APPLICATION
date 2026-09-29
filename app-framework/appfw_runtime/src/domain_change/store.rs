use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use thiserror::Error;
use tokio::sync::RwLock;

use super::model::{validate_consumer_id, DomainChangeEnvelope, DomainChangeError};

const MAX_BATCH_SIZE: usize = 1_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StoredDomainChange {
    sequence: u64,
    semantic_digest: String,
    envelope: DomainChangeEnvelope,
}

impl StoredDomainChange {
    pub(crate) fn try_new(
        sequence: u64,
        claimed_digest: impl Into<String>,
        mut envelope: DomainChangeEnvelope,
    ) -> Result<Self, DomainChangeStoreError> {
        if sequence == 0 {
            return Err(DomainChangeStoreError::StoredSequenceZero);
        }
        envelope.validate_and_canonicalize()?;
        let claimed_digest = claimed_digest.into();
        let semantic_digest = envelope.semantic_digest()?;
        if !is_lowercase_sha256(&claimed_digest) || claimed_digest != semantic_digest {
            return Err(DomainChangeStoreError::StoredDigestMismatch { sequence });
        }
        Ok(Self {
            sequence,
            semantic_digest,
            envelope,
        })
    }

    pub(crate) fn sequence(&self) -> u64 {
        self.sequence
    }

    pub(crate) fn semantic_digest(&self) -> &str {
        &self.semantic_digest
    }

    pub(crate) fn envelope(&self) -> &DomainChangeEnvelope {
        &self.envelope
    }

    pub(crate) fn validate(&self) -> Result<(), DomainChangeStoreError> {
        Self::try_new(
            self.sequence,
            self.semantic_digest.clone(),
            self.envelope.clone(),
        )
        .map(|_| ())
    }

    #[cfg(test)]
    pub(super) fn unchecked(
        sequence: u64,
        semantic_digest: impl Into<String>,
        envelope: DomainChangeEnvelope,
    ) -> Self {
        Self {
            sequence,
            semantic_digest: semantic_digest.into(),
            envelope,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum DomainChangeAppendOutcome {
    Inserted(StoredDomainChange),
    Duplicate(StoredDomainChange),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DomainChangeBatch {
    after_sequence: u64,
    next_sequence: u64,
    changes: Vec<StoredDomainChange>,
}

impl DomainChangeBatch {
    pub(crate) fn try_new(
        after_sequence: u64,
        mut changes: Vec<StoredDomainChange>,
    ) -> Result<Self, DomainChangeStoreError> {
        changes.sort_by_key(StoredDomainChange::sequence);
        let mut previous = after_sequence;
        for change in &changes {
            change.validate()?;
            let expected = previous
                .checked_add(1)
                .ok_or(DomainChangeStoreError::StoredSequenceOverflow { previous })?;
            if change.sequence != expected {
                if change.sequence == previous {
                    return Err(DomainChangeStoreError::StoredSequenceDuplicate {
                        sequence: change.sequence,
                    });
                }
                return Err(DomainChangeStoreError::StoredSequenceNonContiguous {
                    expected,
                    actual: change.sequence,
                });
            }
            previous = change.sequence;
        }
        Ok(Self {
            after_sequence,
            next_sequence: previous,
            changes,
        })
    }

    pub(crate) fn validate(&self) -> Result<(), DomainChangeStoreError> {
        let rebuilt = Self::try_new(self.after_sequence, self.changes.clone())?;
        if rebuilt.next_sequence != self.next_sequence {
            return Err(DomainChangeStoreError::StoredSequenceNonContiguous {
                expected: rebuilt.next_sequence,
                actual: self.next_sequence,
            });
        }
        Ok(())
    }

    pub(crate) fn after_sequence(&self) -> u64 {
        self.after_sequence
    }

    pub(crate) fn next_sequence(&self) -> u64 {
        self.next_sequence
    }

    pub(crate) fn changes(&self) -> &[StoredDomainChange] {
        &self.changes
    }

    #[cfg(test)]
    pub(super) fn unchecked(
        after_sequence: u64,
        next_sequence: u64,
        changes: Vec<StoredDomainChange>,
    ) -> Self {
        Self {
            after_sequence,
            next_sequence,
            changes,
        }
    }
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub(crate) enum DomainChangeAdapterErrorCode {
    #[error("adapter unavailable")]
    Unavailable,
    #[error("adapter timeout")]
    Timeout,
    #[error("adapter transaction failed")]
    TransactionFailed,
    #[error("adapter data is corrupt")]
    CorruptData,
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub(crate) enum DomainChangeStoreError {
    #[error(transparent)]
    InvalidEvent(#[from] DomainChangeError),
    #[error("domain-change event identity was reused with different content")]
    EventConflict,
    #[error("domain-change transport position is already occupied")]
    TransportPositionConflict,
    #[error("domain-change batch limit must be between one and 1000")]
    InvalidBatchLimit,
    #[error("domain-change cursor {sequence} is beyond the latest sequence {latest}")]
    CursorOutOfRange { sequence: u64, latest: u64 },
    #[error("domain-change consumer cursor cannot move backward")]
    CursorRegression,
    #[error("stored domain-change sequence must be greater than zero")]
    StoredSequenceZero,
    #[error("stored domain-change sequence {sequence} is repeated")]
    StoredSequenceDuplicate { sequence: u64 },
    #[error("stored domain-change sequence is not contiguous: expected {expected}, got {actual}")]
    StoredSequenceNonContiguous { expected: u64, actual: u64 },
    #[error("stored domain-change sequence overflow after {previous}")]
    StoredSequenceOverflow { previous: u64 },
    #[error("stored domain-change digest does not match sequence {sequence}")]
    StoredDigestMismatch { sequence: u64 },
    #[error("domain-change store adapter failed: {code}")]
    Adapter { code: DomainChangeAdapterErrorCode },
}

#[async_trait]
pub(crate) trait DomainChangeStore: Send + Sync {
    async fn append(
        &self,
        envelope: DomainChangeEnvelope,
    ) -> Result<DomainChangeAppendOutcome, DomainChangeStoreError>;

    async fn read_after(
        &self,
        after_sequence: u64,
        limit: usize,
    ) -> Result<DomainChangeBatch, DomainChangeStoreError>;

    async fn consumer_cursor(&self, consumer_id: &str) -> Result<u64, DomainChangeStoreError>;

    async fn advance_consumer_cursor(
        &self,
        consumer_id: &str,
        sequence: u64,
    ) -> Result<(), DomainChangeStoreError>;
}

/// Deterministic local reference store. It is deliberately non-durable.
#[derive(Clone, Default)]
pub(crate) struct InMemoryDomainChangeStore {
    state: Arc<RwLock<InMemoryState>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InMemoryDomainChangeState {
    pub(crate) event_count: usize,
    pub(crate) latest_sequence: u64,
    pub(crate) consumer_cursors: BTreeMap<String, u64>,
}

#[derive(Default)]
struct InMemoryState {
    changes: Vec<StoredDomainChange>,
    event_sequences: BTreeMap<(String, String, String, String), u64>,
    transport_events: BTreeMap<(String, u32, u64), u64>,
    consumer_cursors: BTreeMap<String, u64>,
}

impl InMemoryDomainChangeStore {
    pub(crate) async fn snapshot(&self) -> InMemoryDomainChangeState {
        let state = self.state.read().await;
        InMemoryDomainChangeState {
            event_count: state.changes.len(),
            latest_sequence: state.changes.len() as u64,
            consumer_cursors: state.consumer_cursors.clone(),
        }
    }
}

#[async_trait]
impl DomainChangeStore for InMemoryDomainChangeStore {
    async fn append(
        &self,
        mut envelope: DomainChangeEnvelope,
    ) -> Result<DomainChangeAppendOutcome, DomainChangeStoreError> {
        envelope.validate_and_canonicalize()?;
        let digest = envelope.semantic_digest()?;
        let event_key = (
            envelope.tenant_id().to_string(),
            envelope.source().source_id().to_string(),
            envelope.source().projection_id().to_string(),
            envelope.event_id().to_string(),
        );
        let transport_key = (
            envelope.transport().stream_id().to_string(),
            envelope.transport().partition(),
            envelope.transport().offset(),
        );

        let mut state = self.state.write().await;
        if let Some(sequence) = state.event_sequences.get(&event_key) {
            let stored = state
                .changes
                .get((*sequence - 1) as usize)
                .expect("validated event index follows its stored sequence");
            return if stored.semantic_digest == digest {
                Ok(DomainChangeAppendOutcome::Duplicate(stored.clone()))
            } else {
                Err(DomainChangeStoreError::EventConflict)
            };
        }
        if state.transport_events.contains_key(&transport_key) {
            return Err(DomainChangeStoreError::TransportPositionConflict);
        }
        let sequence = (state.changes.len() as u64).checked_add(1).ok_or(
            DomainChangeStoreError::StoredSequenceOverflow {
                previous: state.changes.len() as u64,
            },
        )?;
        let stored = StoredDomainChange::try_new(sequence, digest, envelope)?;
        state.event_sequences.insert(event_key, sequence);
        state.transport_events.insert(transport_key, sequence);
        state.changes.push(stored.clone());
        Ok(DomainChangeAppendOutcome::Inserted(stored))
    }

    async fn read_after(
        &self,
        after_sequence: u64,
        limit: usize,
    ) -> Result<DomainChangeBatch, DomainChangeStoreError> {
        if limit == 0 || limit > MAX_BATCH_SIZE {
            return Err(DomainChangeStoreError::InvalidBatchLimit);
        }
        let state = self.state.read().await;
        let latest = state.changes.len() as u64;
        if after_sequence > latest {
            return Err(DomainChangeStoreError::CursorOutOfRange {
                sequence: after_sequence,
                latest,
            });
        }
        let changes = state
            .changes
            .iter()
            .skip(after_sequence as usize)
            .take(limit)
            .cloned()
            .collect();
        DomainChangeBatch::try_new(after_sequence, changes)
    }

    async fn consumer_cursor(&self, consumer_id: &str) -> Result<u64, DomainChangeStoreError> {
        validate_consumer_id(consumer_id)?;
        let state = self.state.read().await;
        Ok(state
            .consumer_cursors
            .get(consumer_id)
            .copied()
            .unwrap_or(0))
    }

    async fn advance_consumer_cursor(
        &self,
        consumer_id: &str,
        sequence: u64,
    ) -> Result<(), DomainChangeStoreError> {
        validate_consumer_id(consumer_id)?;
        let mut state = self.state.write().await;
        let latest = state.changes.len() as u64;
        if sequence > latest {
            return Err(DomainChangeStoreError::CursorOutOfRange { sequence, latest });
        }
        let current = state
            .consumer_cursors
            .get(consumer_id)
            .copied()
            .unwrap_or(0);
        if sequence < current {
            return Err(DomainChangeStoreError::CursorRegression);
        }
        state
            .consumer_cursors
            .insert(consumer_id.to_string(), sequence);
        Ok(())
    }
}

fn is_lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
