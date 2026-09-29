//! Dormant provider-neutral Fabric topology and read-model contracts.
//!
//! This namespace is unsupported and non-stable. Its hidden public visibility
//! exists only so the frozen black-box integration tests can exercise the
//! contract; it is not a downstream compatibility or Product surface.
//! The namespace owns only deterministic in-memory validation and projection.
//! It has no route, persistence adapter, identity provider, approval engine,
//! live probe, dashboard, Product adoption, or release integration. Callers
//! must supply typed, sanitized records; any future adapter requires a new
//! reviewed boundary.

mod management;
mod model;
mod registry;
mod telemetry;

pub use management::{
    FabricComponentDetail, FabricEvidenceCoverage, FabricManagementComponent,
    FabricManagementOverview, FabricManagementReadError, FabricManagementSummary, FabricReadScope,
    FABRIC_COMPONENT_DETAIL_SCHEMA_VERSION, FABRIC_MANAGEMENT_OVERVIEW_SCHEMA_VERSION,
};
pub use model::{
    FabricAvailability, FabricClassification, FabricComponentKind, FabricComponentRecord,
    FabricDependency, FabricEnvironment, FabricEvidenceKind, FabricEvidencePosture,
    FabricEvidenceReference, FabricHealth, FabricHealthPosture, FabricLifecycleState,
    FabricObservationSource, FabricProvenance, FabricProvenanceSource, FabricRegistryError,
    FabricRelationshipKind, FABRIC_COMPONENT_SCHEMA_VERSION,
};
pub use registry::{
    FabricRegistry, FabricRelationship, FabricTopologyNode, FabricTopologyProjection,
    FABRIC_TOPOLOGY_SCHEMA_VERSION,
};
pub use telemetry::{
    FabricTelemetryEvent, FabricTelemetryKind, FabricValidationCode,
    FABRIC_TELEMETRY_SCHEMA_VERSION,
};
