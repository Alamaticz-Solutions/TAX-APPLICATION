//! Closed, payload-free telemetry vocabulary for dormant Fabric events.

use serde::{Deserialize, Deserializer, Serialize};

use crate::observability::RequestContext;

use super::model::{
    health_availability_are_coherent, validate_identifier, validate_request_context_id,
    FabricAvailability, FabricEvidencePosture, FabricHealth, FabricRegistryError,
};

pub const FABRIC_TELEMETRY_SCHEMA_VERSION: &str = "appfw.fabric.telemetry@1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FabricTelemetryKind {
    #[serde(rename = "app.fabric.registry.loaded")]
    RegistryLoaded,
    #[serde(rename = "app.fabric.topology.projected")]
    TopologyProjected,
    #[serde(rename = "app.fabric.validation.failed")]
    ValidationFailed,
    #[serde(rename = "app.fabric.component.health.changed")]
    ComponentHealthChanged,
    #[serde(rename = "app.fabric.evidence.posture.changed")]
    EvidencePostureChanged,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FabricValidationCode {
    UnsupportedSchemaVersion,
    InvalidField,
    SensitiveValue,
    DuplicateRevision,
    IdentityConflict,
    InvalidLifecycleTransition,
    DuplicateDependency,
    DuplicateEvidence,
    DanglingRelationship,
    InvalidRelationship,
    DigestFailure,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FabricTelemetryEvent {
    schema_version: String,
    event: FabricTelemetryKind,
    request_id: String,
    correlation_id: String,
    component_id: Option<String>,
    validation_code: Option<FabricValidationCode>,
    health: Option<FabricHealth>,
    availability: Option<FabricAvailability>,
    evidence_posture: Option<FabricEvidencePosture>,
}

impl FabricTelemetryEvent {
    pub fn registry_loaded(request_context: &RequestContext) -> Result<Self, FabricRegistryError> {
        Self::new(FabricTelemetryKind::RegistryLoaded, request_context)?.validated()
    }

    pub fn topology_projected(
        request_context: &RequestContext,
    ) -> Result<Self, FabricRegistryError> {
        Self::new(FabricTelemetryKind::TopologyProjected, request_context)?.validated()
    }

    pub fn validation_failed(
        request_context: &RequestContext,
        error: &FabricRegistryError,
    ) -> Result<Self, FabricRegistryError> {
        let mut event = Self::new(FabricTelemetryKind::ValidationFailed, request_context)?;
        event.validation_code = Some(FabricValidationCode::from(error));
        event.validated()
    }

    pub fn component_health_changed(
        request_context: &RequestContext,
        component_id: impl Into<String>,
        health: FabricHealth,
        availability: FabricAvailability,
    ) -> Result<Self, FabricRegistryError> {
        let mut event = Self::new(FabricTelemetryKind::ComponentHealthChanged, request_context)?;
        event.set_component_id(component_id.into())?;
        event.health = Some(health);
        event.availability = Some(availability);
        event.validated()
    }

    pub fn evidence_posture_changed(
        request_context: &RequestContext,
        component_id: impl Into<String>,
        posture: FabricEvidencePosture,
    ) -> Result<Self, FabricRegistryError> {
        let mut event = Self::new(FabricTelemetryKind::EvidencePostureChanged, request_context)?;
        event.set_component_id(component_id.into())?;
        event.evidence_posture = Some(posture);
        event.validated()
    }

    fn new(
        event: FabricTelemetryKind,
        request_context: &RequestContext,
    ) -> Result<Self, FabricRegistryError> {
        validate_request_context_id(&request_context.request_id, "telemetry.request_id")?;
        validate_request_context_id(&request_context.correlation_id, "telemetry.correlation_id")?;
        Ok(Self {
            schema_version: FABRIC_TELEMETRY_SCHEMA_VERSION.to_string(),
            event,
            request_id: request_context.request_id.clone(),
            correlation_id: request_context.correlation_id.clone(),
            component_id: None,
            validation_code: None,
            health: None,
            availability: None,
            evidence_posture: None,
        })
    }

    fn set_component_id(&mut self, component_id: String) -> Result<(), FabricRegistryError> {
        validate_identifier(&component_id, "telemetry.component_id")?;
        self.component_id = Some(component_id);
        Ok(())
    }

    pub fn schema_version(&self) -> &str {
        &self.schema_version
    }

    pub fn event(&self) -> FabricTelemetryKind {
        self.event
    }

    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    pub fn correlation_id(&self) -> &str {
        &self.correlation_id
    }

    pub fn component_id(&self) -> Option<&str> {
        self.component_id.as_deref()
    }

    pub fn validation_code(&self) -> Option<FabricValidationCode> {
        self.validation_code
    }

    pub fn health(&self) -> Option<FabricHealth> {
        self.health
    }

    pub fn availability(&self) -> Option<FabricAvailability> {
        self.availability
    }

    pub fn evidence_posture(&self) -> Option<FabricEvidencePosture> {
        self.evidence_posture
    }

    fn validate(&self) -> Result<(), FabricRegistryError> {
        if self.schema_version != FABRIC_TELEMETRY_SCHEMA_VERSION {
            return Err(FabricRegistryError::UnsupportedSchemaVersion);
        }
        validate_request_context_id(&self.request_id, "telemetry.request_id")?;
        validate_request_context_id(&self.correlation_id, "telemetry.correlation_id")?;
        if let Some(component_id) = &self.component_id {
            validate_identifier(component_id, "telemetry.component_id")?;
        }

        let shape_is_valid = match self.event {
            FabricTelemetryKind::RegistryLoaded | FabricTelemetryKind::TopologyProjected => {
                self.component_id.is_none()
                    && self.validation_code.is_none()
                    && self.health.is_none()
                    && self.availability.is_none()
                    && self.evidence_posture.is_none()
            }
            FabricTelemetryKind::ValidationFailed => {
                self.component_id.is_none()
                    && self.validation_code.is_some()
                    && self.health.is_none()
                    && self.availability.is_none()
                    && self.evidence_posture.is_none()
            }
            FabricTelemetryKind::ComponentHealthChanged => {
                self.component_id.is_some()
                    && self.validation_code.is_none()
                    && self
                        .health
                        .zip(self.availability)
                        .is_some_and(|(health, availability)| {
                            health_availability_are_coherent(health, availability)
                        })
                    && self.evidence_posture.is_none()
            }
            FabricTelemetryKind::EvidencePostureChanged => {
                self.component_id.is_some()
                    && self.validation_code.is_none()
                    && self.health.is_none()
                    && self.availability.is_none()
                    && self.evidence_posture.is_some()
            }
        };
        if !shape_is_valid {
            return Err(FabricRegistryError::InvalidField {
                field: "telemetry",
                reason: "event fields do not match the closed telemetry vocabulary",
            });
        }
        Ok(())
    }

    fn validated(self) -> Result<Self, FabricRegistryError> {
        self.validate()?;
        Ok(self)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FabricTelemetryWire {
    schema_version: String,
    event: FabricTelemetryKind,
    request_id: String,
    correlation_id: String,
    component_id: Option<String>,
    validation_code: Option<FabricValidationCode>,
    health: Option<FabricHealth>,
    availability: Option<FabricAvailability>,
    evidence_posture: Option<FabricEvidencePosture>,
}

impl<'de> Deserialize<'de> for FabricTelemetryEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = FabricTelemetryWire::deserialize(deserializer)?;
        let event = Self {
            schema_version: wire.schema_version,
            event: wire.event,
            request_id: wire.request_id,
            correlation_id: wire.correlation_id,
            component_id: wire.component_id,
            validation_code: wire.validation_code,
            health: wire.health,
            availability: wire.availability,
            evidence_posture: wire.evidence_posture,
        };
        event.validate().map_err(serde::de::Error::custom)?;
        Ok(event)
    }
}

impl From<&FabricRegistryError> for FabricValidationCode {
    fn from(error: &FabricRegistryError) -> Self {
        match error {
            FabricRegistryError::UnsupportedSchemaVersion => Self::UnsupportedSchemaVersion,
            FabricRegistryError::InvalidField { .. } => Self::InvalidField,
            FabricRegistryError::SensitiveValue { .. } => Self::SensitiveValue,
            FabricRegistryError::DuplicateRevision { .. } => Self::DuplicateRevision,
            FabricRegistryError::IdentityConflict { .. } => Self::IdentityConflict,
            FabricRegistryError::InvalidLifecycleTransition { .. } => {
                Self::InvalidLifecycleTransition
            }
            FabricRegistryError::DuplicateDependency { .. } => Self::DuplicateDependency,
            FabricRegistryError::DuplicateEvidence { .. } => Self::DuplicateEvidence,
            FabricRegistryError::DanglingRelationship { .. } => Self::DanglingRelationship,
            FabricRegistryError::InvalidRelationship { .. } => Self::InvalidRelationship,
            FabricRegistryError::DigestFailure => Self::DigestFailure,
        }
    }
}
