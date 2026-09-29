use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    record_locator::validate_record_locator, sensitive_metadata::contains_sensitive_metadata,
};

pub(crate) const DOMAIN_CHANGE_SCHEMA_VERSION: &str = "appfw.domain_change@1";
pub(crate) const SELECTIVE_REFRESH_SCHEMA_VERSION: &str = "appfw.selective_refresh@1";

const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_CONTEXT_BYTES: usize = 256;
const MAX_NOTICE_VALUES: usize = 64;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DomainChangeKind {
    Upserted,
    Deleted,
    Invalidated,
    PermissionChanged,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DomainTargetKind {
    Record,
    Collection,
    View,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) struct DomainChangeSource {
    source_id: String,
    projection_id: String,
    schema_version: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) struct DomainTransportPosition {
    stream_id: String,
    partition: u32,
    offset: u64,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) struct DomainProjectionWatermark {
    revision: u64,
    observed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) struct DomainInvalidationTarget {
    kind: DomainTargetKind,
    locator: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct DomainChangeEnvelope {
    schema_version: String,
    event_id: String,
    tenant_id: String,
    correlation_id: String,
    occurred_at: DateTime<Utc>,
    source: DomainChangeSource,
    transport: DomainTransportPosition,
    change_kind: DomainChangeKind,
    projection: DomainProjectionWatermark,
    targets: Vec<DomainInvalidationTarget>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) struct SelectiveRefreshTarget {
    target: DomainInvalidationTarget,
    reason: DomainChangeKind,
    revision: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct SelectiveRefreshNotice {
    schema_version: String,
    tenant_id: String,
    source: DomainChangeSource,
    watermark: DomainProjectionWatermark,
    targets: Vec<SelectiveRefreshTarget>,
    cause_event_ids: Vec<String>,
    correlation_ids: Vec<String>,
    authorized_refetch_required: bool,
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub(crate) enum DomainChangeError {
    #[error("domain-change schema version is unsupported")]
    UnsupportedSchemaVersion,
    #[error("domain-change field `{field}` is invalid: {reason}")]
    InvalidField {
        field: &'static str,
        reason: &'static str,
    },
    #[error("domain-change field `{field}` contains a prohibited sensitive value shape")]
    SensitiveValue { field: &'static str },
    #[error("domain-change target is repeated")]
    DuplicateTarget,
    #[error("domain-change semantic digest could not be computed")]
    DigestFailure,
}

impl DomainChangeSource {
    pub(crate) fn try_new(
        source_id: impl Into<String>,
        projection_id: impl Into<String>,
        schema_version: impl Into<String>,
    ) -> Result<Self, DomainChangeError> {
        let value = Self {
            source_id: source_id.into(),
            projection_id: projection_id.into(),
            schema_version: schema_version.into(),
        };
        value.validate()?;
        Ok(value)
    }

    pub(crate) fn source_id(&self) -> &str {
        &self.source_id
    }

    pub(crate) fn projection_id(&self) -> &str {
        &self.projection_id
    }

    pub(crate) fn schema_version(&self) -> &str {
        &self.schema_version
    }

    fn validate(&self) -> Result<(), DomainChangeError> {
        validate_token(
            &self.source_id,
            "source.source_id",
            MAX_IDENTIFIER_BYTES,
            false,
        )?;
        validate_token(
            &self.projection_id,
            "source.projection_id",
            MAX_IDENTIFIER_BYTES,
            false,
        )?;
        validate_token(
            &self.schema_version,
            "source.schema_version",
            MAX_IDENTIFIER_BYTES,
            false,
        )
    }
}

impl DomainTransportPosition {
    pub(crate) fn try_new(
        stream_id: impl Into<String>,
        partition: u32,
        offset: u64,
    ) -> Result<Self, DomainChangeError> {
        let value = Self {
            stream_id: stream_id.into(),
            partition,
            offset,
        };
        value.validate()?;
        Ok(value)
    }

    pub(crate) fn stream_id(&self) -> &str {
        &self.stream_id
    }

    pub(crate) fn partition(&self) -> u32 {
        self.partition
    }

    pub(crate) fn offset(&self) -> u64 {
        self.offset
    }

    fn validate(&self) -> Result<(), DomainChangeError> {
        validate_token(
            &self.stream_id,
            "transport.stream_id",
            MAX_IDENTIFIER_BYTES,
            false,
        )
    }
}

impl DomainProjectionWatermark {
    pub(crate) fn try_new(
        revision: u64,
        observed_at: DateTime<Utc>,
    ) -> Result<Self, DomainChangeError> {
        if revision == 0 {
            return Err(DomainChangeError::InvalidField {
                field: "projection.revision",
                reason: "must be greater than zero",
            });
        }
        Ok(Self {
            revision,
            observed_at,
        })
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    pub(crate) fn observed_at(&self) -> DateTime<Utc> {
        self.observed_at
    }

    fn validate(&self) -> Result<(), DomainChangeError> {
        Self::try_new(self.revision, self.observed_at).map(|_| ())
    }
}

impl DomainInvalidationTarget {
    pub(crate) fn try_new(
        kind: DomainTargetKind,
        locator: impl Into<String>,
    ) -> Result<Self, DomainChangeError> {
        let value = Self {
            kind,
            locator: locator.into(),
        };
        value.validate()?;
        Ok(value)
    }

    pub(crate) fn kind(&self) -> DomainTargetKind {
        self.kind
    }

    pub(crate) fn locator(&self) -> &str {
        &self.locator
    }

    fn validate(&self) -> Result<(), DomainChangeError> {
        if contains_sensitive_metadata(&self.locator) {
            return Err(DomainChangeError::SensitiveValue {
                field: "targets.locator",
            });
        }
        match self.kind {
            DomainTargetKind::Record => validate_record_locator(&self.locator).map_err(|_| {
                DomainChangeError::InvalidField {
                    field: "targets.locator",
                    reason: "record targets require an opaque App Framework record locator",
                }
            }),
            DomainTargetKind::Collection | DomainTargetKind::View => validate_token(
                &self.locator,
                "targets.locator",
                MAX_IDENTIFIER_BYTES,
                false,
            ),
        }
    }
}

impl DomainChangeEnvelope {
    pub(crate) fn try_new(
        event_id: impl Into<String>,
        tenant_id: impl Into<String>,
        correlation_id: impl Into<String>,
        occurred_at: DateTime<Utc>,
        source: DomainChangeSource,
        transport: DomainTransportPosition,
        change_kind: DomainChangeKind,
        projection: DomainProjectionWatermark,
        targets: Vec<DomainInvalidationTarget>,
    ) -> Result<Self, DomainChangeError> {
        let mut value = Self {
            schema_version: DOMAIN_CHANGE_SCHEMA_VERSION.to_string(),
            event_id: event_id.into(),
            tenant_id: tenant_id.into(),
            correlation_id: correlation_id.into(),
            occurred_at,
            source,
            transport,
            change_kind,
            projection,
            targets,
        };
        value.validate_and_canonicalize()?;
        Ok(value)
    }

    pub(crate) fn event_id(&self) -> &str {
        &self.event_id
    }
    pub(crate) fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
    pub(crate) fn correlation_id(&self) -> &str {
        &self.correlation_id
    }
    pub(crate) fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }
    pub(crate) fn source(&self) -> &DomainChangeSource {
        &self.source
    }
    pub(crate) fn transport(&self) -> &DomainTransportPosition {
        &self.transport
    }
    pub(crate) fn change_kind(&self) -> DomainChangeKind {
        self.change_kind
    }
    pub(crate) fn projection(&self) -> &DomainProjectionWatermark {
        &self.projection
    }
    pub(crate) fn targets(&self) -> &[DomainInvalidationTarget] {
        &self.targets
    }

    pub(crate) fn validate_and_canonicalize(&mut self) -> Result<(), DomainChangeError> {
        if self.schema_version != DOMAIN_CHANGE_SCHEMA_VERSION {
            return Err(DomainChangeError::UnsupportedSchemaVersion);
        }
        validate_token(&self.event_id, "event_id", MAX_IDENTIFIER_BYTES, false)?;
        validate_token(&self.tenant_id, "tenant_id", MAX_IDENTIFIER_BYTES, false)?;
        validate_token(
            &self.correlation_id,
            "correlation_id",
            MAX_CONTEXT_BYTES,
            true,
        )?;
        self.source.validate()?;
        self.transport.validate()?;
        self.projection.validate()?;
        if self.targets.is_empty() || self.targets.len() > MAX_NOTICE_VALUES {
            return Err(DomainChangeError::InvalidField {
                field: "targets",
                reason: "must contain between one and 64 targets",
            });
        }
        let mut seen = BTreeSet::new();
        for target in &self.targets {
            target.validate()?;
            if !seen.insert((target.kind, target.locator.clone())) {
                return Err(DomainChangeError::DuplicateTarget);
            }
        }
        self.targets.sort();
        Ok(())
    }

    pub(crate) fn semantic_digest(&self) -> Result<String, DomainChangeError> {
        semantic_digest(self)
    }
}

impl SelectiveRefreshTarget {
    pub(crate) fn try_new(
        target: DomainInvalidationTarget,
        reason: DomainChangeKind,
        revision: u64,
    ) -> Result<Self, DomainChangeError> {
        target.validate()?;
        if revision == 0 {
            return Err(DomainChangeError::InvalidField {
                field: "targets.revision",
                reason: "must be greater than zero",
            });
        }
        Ok(Self {
            target,
            reason,
            revision,
        })
    }

    pub(crate) fn target(&self) -> &DomainInvalidationTarget {
        &self.target
    }
    pub(crate) fn reason(&self) -> DomainChangeKind {
        self.reason
    }
    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }
}

impl SelectiveRefreshNotice {
    pub(crate) fn try_new(
        tenant_id: impl Into<String>,
        source: DomainChangeSource,
        watermark: DomainProjectionWatermark,
        targets: Vec<SelectiveRefreshTarget>,
        cause_event_ids: Vec<String>,
        correlation_ids: Vec<String>,
    ) -> Result<Self, DomainChangeError> {
        let mut value = Self {
            schema_version: SELECTIVE_REFRESH_SCHEMA_VERSION.to_string(),
            tenant_id: tenant_id.into(),
            source,
            watermark,
            targets,
            cause_event_ids,
            correlation_ids,
            authorized_refetch_required: true,
        };
        value.validate_and_canonicalize()?;
        Ok(value)
    }

    pub(crate) fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
    pub(crate) fn source(&self) -> &DomainChangeSource {
        &self.source
    }
    pub(crate) fn watermark(&self) -> &DomainProjectionWatermark {
        &self.watermark
    }
    pub(crate) fn targets(&self) -> &[SelectiveRefreshTarget] {
        &self.targets
    }
    pub(crate) fn cause_event_ids(&self) -> &[String] {
        &self.cause_event_ids
    }
    pub(crate) fn correlation_ids(&self) -> &[String] {
        &self.correlation_ids
    }
    pub(crate) fn authorized_refetch_required(&self) -> bool {
        self.authorized_refetch_required
    }

    pub(crate) fn validate_and_canonicalize(&mut self) -> Result<(), DomainChangeError> {
        if self.schema_version != SELECTIVE_REFRESH_SCHEMA_VERSION {
            return Err(DomainChangeError::UnsupportedSchemaVersion);
        }
        validate_token(&self.tenant_id, "tenant_id", MAX_IDENTIFIER_BYTES, false)?;
        self.source.validate()?;
        self.watermark.validate()?;
        if !self.authorized_refetch_required {
            return Err(DomainChangeError::InvalidField {
                field: "authorized_refetch_required",
                reason: "selective refresh always requires an authorized re-fetch",
            });
        }
        if self.targets.is_empty() || self.targets.len() > MAX_NOTICE_VALUES {
            return Err(DomainChangeError::InvalidField {
                field: "targets",
                reason: "must contain between one and 64 targets",
            });
        }
        let mut seen = BTreeSet::new();
        for target in &self.targets {
            target.target.validate()?;
            if target.revision == 0 || target.revision > self.watermark.revision {
                return Err(DomainChangeError::InvalidField {
                    field: "targets.revision",
                    reason: "must be positive and not exceed the notice watermark",
                });
            }
            if !seen.insert((target.target.kind, target.target.locator.clone())) {
                return Err(DomainChangeError::DuplicateTarget);
            }
        }
        self.targets.sort();
        validate_unique_tokens(&mut self.cause_event_ids, "cause_event_ids")?;
        validate_unique_tokens(&mut self.correlation_ids, "correlation_ids")?;
        Ok(())
    }

    pub(crate) fn semantic_digest(&self) -> Result<String, DomainChangeError> {
        semantic_digest(self)
    }
}

fn semantic_digest<T>(value: &T) -> Result<String, DomainChangeError>
where
    T: Clone + Serialize + Canonicalize,
{
    let mut canonical = value.clone();
    canonical.canonicalize()?;
    let bytes = serde_json::to_vec(&canonical).map_err(|_| DomainChangeError::DigestFailure)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

trait Canonicalize {
    fn canonicalize(&mut self) -> Result<(), DomainChangeError>;
}

impl Canonicalize for DomainChangeEnvelope {
    fn canonicalize(&mut self) -> Result<(), DomainChangeError> {
        self.validate_and_canonicalize()
    }
}

impl Canonicalize for SelectiveRefreshNotice {
    fn canonicalize(&mut self) -> Result<(), DomainChangeError> {
        self.validate_and_canonicalize()
    }
}

pub(crate) fn validate_consumer_id(value: &str) -> Result<(), DomainChangeError> {
    validate_token(value, "consumer_id", MAX_IDENTIFIER_BYTES, false)
}

fn validate_token(
    value: &str,
    field: &'static str,
    max_bytes: usize,
    context_compatible: bool,
) -> Result<(), DomainChangeError> {
    if value.is_empty() || value.len() > max_bytes {
        return Err(DomainChangeError::InvalidField {
            field,
            reason: "must be non-empty and within the byte limit",
        });
    }
    if contains_sensitive_metadata(value) {
        return Err(DomainChangeError::SensitiveValue { field });
    }
    let valid = if context_compatible {
        value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && byte != b'"' && byte != b'\\')
    } else {
        value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'_' | b'-' | b':' | b'@')
        })
    };
    if !valid {
        return Err(DomainChangeError::InvalidField {
            field,
            reason: "contains unsupported characters",
        });
    }
    Ok(())
}

fn validate_unique_tokens(
    values: &mut [String],
    field: &'static str,
) -> Result<(), DomainChangeError> {
    if values.is_empty() || values.len() > MAX_NOTICE_VALUES {
        return Err(DomainChangeError::InvalidField {
            field,
            reason: "must contain between one and 64 values",
        });
    }
    for value in values.iter() {
        validate_token(value, field, MAX_CONTEXT_BYTES, true)?;
    }
    values.sort();
    if values.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(DomainChangeError::InvalidField {
            field,
            reason: "must not contain duplicate values",
        });
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSource {
    source_id: String,
    projection_id: String,
    schema_version: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTransport {
    stream_id: String,
    partition: u32,
    offset: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawWatermark {
    revision: u64,
    observed_at: DateTime<Utc>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTarget {
    kind: DomainTargetKind,
    locator: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDomainChangeEnvelope {
    schema_version: String,
    event_id: String,
    tenant_id: String,
    correlation_id: String,
    occurred_at: DateTime<Utc>,
    source: RawSource,
    transport: RawTransport,
    change_kind: DomainChangeKind,
    projection: RawWatermark,
    targets: Vec<RawTarget>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRefreshTarget {
    target: RawTarget,
    reason: DomainChangeKind,
    revision: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSelectiveRefreshNotice {
    schema_version: String,
    tenant_id: String,
    source: RawSource,
    watermark: RawWatermark,
    targets: Vec<RawRefreshTarget>,
    cause_event_ids: Vec<String>,
    correlation_ids: Vec<String>,
    authorized_refetch_required: bool,
}

impl<'de> Deserialize<'de> for DomainChangeEnvelope {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawDomainChangeEnvelope::deserialize(deserializer)?;
        let mut value = Self {
            schema_version: raw.schema_version,
            event_id: raw.event_id,
            tenant_id: raw.tenant_id,
            correlation_id: raw.correlation_id,
            occurred_at: raw.occurred_at,
            source: raw.source.try_into().map_err(D::Error::custom)?,
            transport: raw.transport.try_into().map_err(D::Error::custom)?,
            change_kind: raw.change_kind,
            projection: raw.projection.try_into().map_err(D::Error::custom)?,
            targets: raw
                .targets
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()
                .map_err(D::Error::custom)?,
        };
        value
            .validate_and_canonicalize()
            .map_err(D::Error::custom)?;
        Ok(value)
    }
}

impl<'de> Deserialize<'de> for SelectiveRefreshNotice {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawSelectiveRefreshNotice::deserialize(deserializer)?;
        let mut value = Self {
            schema_version: raw.schema_version,
            tenant_id: raw.tenant_id,
            source: raw.source.try_into().map_err(D::Error::custom)?,
            watermark: raw.watermark.try_into().map_err(D::Error::custom)?,
            targets: raw
                .targets
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()
                .map_err(D::Error::custom)?,
            cause_event_ids: raw.cause_event_ids,
            correlation_ids: raw.correlation_ids,
            authorized_refetch_required: raw.authorized_refetch_required,
        };
        value
            .validate_and_canonicalize()
            .map_err(D::Error::custom)?;
        Ok(value)
    }
}

impl TryFrom<RawSource> for DomainChangeSource {
    type Error = DomainChangeError;
    fn try_from(raw: RawSource) -> Result<Self, Self::Error> {
        Self::try_new(raw.source_id, raw.projection_id, raw.schema_version)
    }
}

impl TryFrom<RawTransport> for DomainTransportPosition {
    type Error = DomainChangeError;
    fn try_from(raw: RawTransport) -> Result<Self, Self::Error> {
        Self::try_new(raw.stream_id, raw.partition, raw.offset)
    }
}

impl TryFrom<RawWatermark> for DomainProjectionWatermark {
    type Error = DomainChangeError;
    fn try_from(raw: RawWatermark) -> Result<Self, Self::Error> {
        Self::try_new(raw.revision, raw.observed_at)
    }
}

impl TryFrom<RawTarget> for DomainInvalidationTarget {
    type Error = DomainChangeError;
    fn try_from(raw: RawTarget) -> Result<Self, Self::Error> {
        Self::try_new(raw.kind, raw.locator)
    }
}

impl TryFrom<RawRefreshTarget> for SelectiveRefreshTarget {
    type Error = DomainChangeError;
    fn try_from(raw: RawRefreshTarget) -> Result<Self, Self::Error> {
        Self::try_new(raw.target.try_into()?, raw.reason, raw.revision)
    }
}

#[cfg(test)]
impl DomainChangeEnvelope {
    pub(super) fn unchecked_with_event_id(mut self, event_id: String) -> Self {
        self.event_id = event_id;
        self
    }
}
