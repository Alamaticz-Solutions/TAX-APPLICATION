use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::sensitive_metadata::contains_sensitive_metadata;

pub const FABRIC_COMPONENT_SCHEMA_VERSION: &str = "appfw.fabric.component@1";

const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_NAME_BYTES: usize = 160;
const MAX_REFERENCE_BYTES: usize = 512;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FabricComponentKind {
    Application,
    UserInterface,
    Service,
    Database,
    EventTopic,
    Integration,
    ArtificialIntelligenceService,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FabricEnvironment {
    Local,
    Dev,
    Test,
    Stage,
    Production,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FabricClassification {
    Public,
    Internal,
    Confidential,
    Restricted,
    Phi,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FabricLifecycleState {
    Registered,
    Active,
    Deprecated,
    Retired,
}

impl FabricLifecycleState {
    pub(crate) fn can_transition_to(self, next: Self) -> bool {
        self == next
            || matches!(
                (self, next),
                (Self::Registered, Self::Active)
                    | (Self::Registered, Self::Retired)
                    | (Self::Active, Self::Deprecated)
                    | (Self::Active, Self::Retired)
                    | (Self::Deprecated, Self::Retired)
            )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FabricRelationshipKind {
    Contains,
    DependsOn,
    Invokes,
    ReadsFrom,
    WritesTo,
    PublishesTo,
    ConsumesFrom,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FabricProvenanceSource {
    LocalFixture,
    Declared,
    Observed,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FabricProvenance {
    pub source: FabricProvenanceSource,
    pub authority_id: String,
    pub source_ref: String,
    pub sha256: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FabricEvidenceKind {
    Contract,
    Source,
    Test,
    Health,
    Operations,
    Security,
    Approval,
}

impl FabricEvidenceKind {
    pub const ALL: [Self; 7] = [
        Self::Contract,
        Self::Source,
        Self::Test,
        Self::Health,
        Self::Operations,
        Self::Security,
        Self::Approval,
    ];
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FabricEvidencePosture {
    Unknown,
    Declared,
    Verified,
    Stale,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FabricEvidenceReference {
    pub evidence_id: String,
    pub kind: FabricEvidenceKind,
    pub posture: FabricEvidencePosture,
    pub uri: String,
    pub sha256: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FabricHealth {
    Unknown,
    Healthy,
    Degraded,
    Unhealthy,
    NotApplicable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FabricAvailability {
    Unknown,
    Available,
    Unavailable,
    NotApplicable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FabricObservationSource {
    LocalFixture,
    Declared,
    Observed,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FabricHealthPosture {
    pub health: FabricHealth,
    pub availability: FabricAvailability,
    pub source: FabricObservationSource,
    pub evidence_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FabricDependency {
    pub component_id: String,
    pub relationship: FabricRelationshipKind,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FabricComponentRecord {
    pub schema_version: String,
    pub component_id: String,
    pub name: String,
    pub kind: FabricComponentKind,
    pub owner_id: String,
    pub version: String,
    pub revision: u64,
    pub environment: FabricEnvironment,
    pub classification: FabricClassification,
    pub lifecycle: FabricLifecycleState,
    pub provenance: FabricProvenance,
    #[serde(default)]
    pub dependencies: Vec<FabricDependency>,
    pub evidence: Vec<FabricEvidenceReference>,
    pub health: FabricHealthPosture,
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum FabricRegistryError {
    #[error("Fabric record uses an unsupported component schema version")]
    UnsupportedSchemaVersion,
    #[error("Fabric record field `{field}` is invalid: {reason}")]
    InvalidField {
        field: &'static str,
        reason: &'static str,
    },
    #[error("Fabric record field `{field}` contains a prohibited sensitive value shape")]
    SensitiveValue { field: &'static str },
    #[error("Fabric component `{component_id}` repeats revision {revision}")]
    DuplicateRevision { component_id: String, revision: u64 },
    #[error("Fabric component `{component_id}` changes stable identity field `{field}`")]
    IdentityConflict {
        component_id: String,
        field: &'static str,
    },
    #[error("Fabric component `{component_id}` has an invalid lifecycle transition")]
    InvalidLifecycleTransition { component_id: String },
    #[error("Fabric component `{component_id}` repeats dependency `{target_id}`")]
    DuplicateDependency {
        component_id: String,
        target_id: String,
    },
    #[error("Fabric component `{component_id}` repeats evidence `{evidence_id}`")]
    DuplicateEvidence {
        component_id: String,
        evidence_id: String,
    },
    #[error("Fabric relationship from `{component_id}` points to missing `{target_id}`")]
    DanglingRelationship {
        component_id: String,
        target_id: String,
    },
    #[error("Fabric relationship from `{component_id}` to `{target_id}` is invalid for the component kinds")]
    InvalidRelationship {
        component_id: String,
        target_id: String,
    },
    #[error("Fabric topology digest could not be computed")]
    DigestFailure,
}

impl FabricComponentRecord {
    pub(crate) fn validate_and_canonicalize(&mut self) -> Result<(), FabricRegistryError> {
        if self.schema_version != FABRIC_COMPONENT_SCHEMA_VERSION {
            return Err(FabricRegistryError::UnsupportedSchemaVersion);
        }
        validate_identifier(&self.component_id, "component_id")?;
        validate_identifier(&self.owner_id, "owner_id")?;
        validate_safe_text(&self.name, "name", MAX_NAME_BYTES)?;
        validate_safe_reference(&self.version, "version")?;
        if self.revision == 0 {
            return Err(FabricRegistryError::InvalidField {
                field: "revision",
                reason: "must be greater than zero",
            });
        }

        self.provenance.validate()?;
        if self.evidence.is_empty() {
            return Err(FabricRegistryError::InvalidField {
                field: "evidence",
                reason: "at least one evidence reference is required",
            });
        }

        let mut evidence_by_id = BTreeMap::new();
        for evidence in &self.evidence {
            evidence.validate()?;
            if evidence_by_id
                .insert(
                    evidence.evidence_id.clone(),
                    (evidence.kind, evidence.posture),
                )
                .is_some()
            {
                return Err(FabricRegistryError::DuplicateEvidence {
                    component_id: self.component_id.clone(),
                    evidence_id: evidence.evidence_id.clone(),
                });
            }
        }

        self.health.validate(self.lifecycle, &evidence_by_id)?;

        let mut dependencies = BTreeSet::new();
        for dependency in &self.dependencies {
            validate_identifier(&dependency.component_id, "dependencies.component_id")?;
            if dependency.component_id == self.component_id {
                return Err(FabricRegistryError::InvalidRelationship {
                    component_id: self.component_id.clone(),
                    target_id: dependency.component_id.clone(),
                });
            }
            if !dependencies.insert((dependency.relationship, dependency.component_id.clone())) {
                return Err(FabricRegistryError::DuplicateDependency {
                    component_id: self.component_id.clone(),
                    target_id: dependency.component_id.clone(),
                });
            }
        }

        self.dependencies.sort();
        self.evidence.sort();
        Ok(())
    }
}

impl FabricProvenance {
    fn validate(&self) -> Result<(), FabricRegistryError> {
        validate_identifier(&self.authority_id, "provenance.authority_id")?;
        validate_safe_reference(&self.source_ref, "provenance.source_ref")?;
        validate_optional_sha256(self.sha256.as_deref(), "provenance.sha256")?;
        if self.source == FabricProvenanceSource::Observed && self.sha256.is_none() {
            return Err(FabricRegistryError::InvalidField {
                field: "provenance.sha256",
                reason: "observed provenance requires an integrity digest",
            });
        }
        Ok(())
    }
}

impl FabricEvidenceReference {
    fn validate(&self) -> Result<(), FabricRegistryError> {
        validate_identifier(&self.evidence_id, "evidence.evidence_id")?;
        validate_safe_reference(&self.uri, "evidence.uri")?;
        validate_optional_sha256(self.sha256.as_deref(), "evidence.sha256")?;
        if self.posture == FabricEvidencePosture::Verified && self.sha256.is_none() {
            return Err(FabricRegistryError::InvalidField {
                field: "evidence.sha256",
                reason: "verified evidence requires an integrity digest",
            });
        }
        Ok(())
    }
}

impl FabricHealthPosture {
    fn validate(
        &self,
        lifecycle: FabricLifecycleState,
        evidence_by_id: &BTreeMap<String, (FabricEvidenceKind, FabricEvidencePosture)>,
    ) -> Result<(), FabricRegistryError> {
        let health_not_applicable = self.health == FabricHealth::NotApplicable;
        let availability_not_applicable = self.availability == FabricAvailability::NotApplicable;
        if health_not_applicable != availability_not_applicable {
            return Err(FabricRegistryError::InvalidField {
                field: "health",
                reason: "health and availability must agree on not_applicable",
            });
        }
        if lifecycle == FabricLifecycleState::Retired && !health_not_applicable {
            return Err(FabricRegistryError::InvalidField {
                field: "health",
                reason: "retired components must use not_applicable posture",
            });
        }
        if lifecycle != FabricLifecycleState::Retired && health_not_applicable {
            return Err(FabricRegistryError::InvalidField {
                field: "health",
                reason: "non-retired components cannot use not_applicable posture",
            });
        }
        if !health_availability_are_coherent(self.health, self.availability) {
            return Err(FabricRegistryError::InvalidField {
                field: "health",
                reason: "health and availability postures are contradictory",
            });
        }
        if self.source == FabricObservationSource::Observed && self.evidence_id.is_none() {
            return Err(FabricRegistryError::InvalidField {
                field: "health.evidence_id",
                reason: "observed health requires evidence",
            });
        }
        if let Some(evidence_id) = &self.evidence_id {
            validate_identifier(evidence_id, "health.evidence_id")?;
            let Some((kind, posture)) = evidence_by_id.get(evidence_id) else {
                return Err(FabricRegistryError::InvalidField {
                    field: "health.evidence_id",
                    reason: "must reference component evidence",
                });
            };
            if self.source == FabricObservationSource::Observed
                && (*kind != FabricEvidenceKind::Health
                    || *posture != FabricEvidencePosture::Verified)
            {
                return Err(FabricRegistryError::InvalidField {
                    field: "health.evidence_id",
                    reason: "observed health requires verified health evidence",
                });
            }
        }
        Ok(())
    }
}

pub(crate) fn health_availability_are_coherent(
    health: FabricHealth,
    availability: FabricAvailability,
) -> bool {
    matches!(
        (health, availability),
        (FabricHealth::Unknown, FabricAvailability::Unknown)
            | (FabricHealth::Healthy, FabricAvailability::Available)
            | (FabricHealth::Degraded, FabricAvailability::Available)
            | (FabricHealth::Unhealthy, FabricAvailability::Unavailable)
            | (
                FabricHealth::NotApplicable,
                FabricAvailability::NotApplicable
            )
    )
}

pub(crate) fn relationship_allowed(
    source: FabricComponentKind,
    target: FabricComponentKind,
    relationship: FabricRelationshipKind,
) -> bool {
    use FabricComponentKind as Component;
    use FabricRelationshipKind as Relationship;

    match relationship {
        Relationship::Contains => {
            source == Component::Application
                && matches!(
                    target,
                    Component::UserInterface
                        | Component::Service
                        | Component::Database
                        | Component::Integration
                )
        }
        Relationship::DependsOn => true,
        Relationship::Invokes => {
            matches!(
                source,
                Component::UserInterface | Component::Service | Component::Integration
            ) && matches!(
                target,
                Component::Service
                    | Component::Integration
                    | Component::ArtificialIntelligenceService
            )
        }
        Relationship::ReadsFrom => {
            matches!(
                source,
                Component::Service
                    | Component::Integration
                    | Component::ArtificialIntelligenceService
            ) && target == Component::Database
        }
        Relationship::WritesTo => {
            matches!(source, Component::Service | Component::Integration)
                && target == Component::Database
        }
        Relationship::PublishesTo => {
            matches!(
                source,
                Component::Service | Component::Integration | Component::Database
            ) && target == Component::EventTopic
        }
        Relationship::ConsumesFrom => {
            matches!(source, Component::Service | Component::Integration)
                && target == Component::EventTopic
        }
    }
}

pub(crate) fn validate_identifier(
    value: &str,
    field: &'static str,
) -> Result<(), FabricRegistryError> {
    if value.is_empty()
        || value.len() > MAX_IDENTIFIER_BYTES
        || !value
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_lowercase())
        || !value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ".-_".contains(ch))
    {
        return Err(FabricRegistryError::InvalidField {
            field,
            reason: "must be a bounded lowercase stable identifier",
        });
    }
    validate_not_sensitive(value, field)
}

pub(crate) fn validate_request_context_id(
    value: &str,
    field: &'static str,
) -> Result<(), FabricRegistryError> {
    if value.is_empty()
        || value.len() > 256
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || "-._:/@".contains(ch))
    {
        return Err(FabricRegistryError::InvalidField {
            field,
            reason: "must be a bounded, header-compatible request-context identifier",
        });
    }
    validate_not_sensitive(value, field)
}

fn validate_safe_text(
    value: &str,
    field: &'static str,
    max_bytes: usize,
) -> Result<(), FabricRegistryError> {
    if value.trim().is_empty() || value.len() > max_bytes || value.chars().any(char::is_control) {
        return Err(FabricRegistryError::InvalidField {
            field,
            reason: "must be nonempty, bounded, and free of control characters",
        });
    }
    validate_not_sensitive(value, field)
}

fn validate_safe_reference(value: &str, field: &'static str) -> Result<(), FabricRegistryError> {
    validate_safe_text(value, field, MAX_REFERENCE_BYTES)
}

fn validate_optional_sha256(
    value: Option<&str>,
    field: &'static str,
) -> Result<(), FabricRegistryError> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.len() != 64
        || !value
            .chars()
            .all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch))
    {
        return Err(FabricRegistryError::InvalidField {
            field,
            reason: "must be a lowercase SHA-256 digest",
        });
    }
    Ok(())
}

fn validate_not_sensitive(value: &str, field: &'static str) -> Result<(), FabricRegistryError> {
    let trimmed = value.trim();
    let normalized = trimmed.to_ascii_lowercase();
    let forbidden_markers = [
        "authorization:",
        "bearer ",
        "access_token",
        "refresh_token",
        "client_secret",
        "api_key",
        "password=",
        "passwd=",
        "private key",
        "mongodb://",
        "mongodb+srv://",
        "postgres://",
        "kafka://",
    ];
    let looks_like_structured_payload =
        matches!(trimmed.as_bytes().first(), Some(b'{') | Some(b'['))
            && serde_json::from_str::<serde_json::Value>(trimmed).is_ok();
    let looks_like_url_userinfo = trimmed
        .split_once("://")
        .and_then(|(_, remainder)| remainder.split_once('@'))
        .is_some_and(|(userinfo, _)| userinfo.contains(':'));

    if looks_like_structured_payload
        || looks_like_url_userinfo
        || contains_sensitive_metadata(trimmed)
        || forbidden_markers
            .iter()
            .any(|marker| normalized.contains(marker))
    {
        return Err(FabricRegistryError::SensitiveValue { field });
    }
    Ok(())
}
