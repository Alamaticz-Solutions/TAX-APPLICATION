//! Caller-scoped deterministic management projections over the registry.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::{
    FabricAvailability, FabricClassification, FabricComponentKind, FabricEnvironment,
    FabricEvidenceKind, FabricEvidencePosture, FabricEvidenceReference, FabricHealth,
    FabricLifecycleState, FabricObservationSource, FabricProvenance, FabricProvenanceSource,
    FabricRegistry, FabricRegistryError, FabricRelationship, FabricTopologyNode,
};

pub const FABRIC_MANAGEMENT_OVERVIEW_SCHEMA_VERSION: &str = "appfw.fabric.management_overview@1";
pub const FABRIC_COMPONENT_DETAIL_SCHEMA_VERSION: &str = "appfw.fabric.component_detail@1";

const MAX_SCOPE_FILTERS: usize = 1_024;
const MAX_MANAGEMENT_COMPONENTS: usize = 10_000;
const MAX_MANAGEMENT_RELATIONSHIPS: usize = 100_000;
const MAX_MANAGEMENT_EVIDENCE_REFERENCES: usize = 100_000;

/// An already-authorized visibility boundary for App Fabric metadata.
///
/// App Fabric does not make identity or policy decisions here. A caller must
/// compile this scope from its authenticated authorization result. Omitted
/// owner/component filters mean "no additional restriction"; explicitly empty
/// filters are rejected rather than ambiguously granting broad access.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FabricReadScope {
    environments: BTreeSet<FabricEnvironment>,
    classifications: BTreeSet<FabricClassification>,
    owner_ids: Option<BTreeSet<String>>,
    component_ids: Option<BTreeSet<String>>,
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum FabricManagementReadError {
    #[error("Fabric read scope is invalid")]
    InvalidScope,
    #[error("Fabric component is not available in this read scope")]
    NotAvailable,
    #[error("Fabric management projection could not be computed")]
    ProjectionFailure,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FabricManagementOverview {
    pub schema_version: String,
    pub projection_digest: String,
    pub summary: FabricManagementSummary,
    pub components: Vec<FabricManagementComponent>,
    pub relationships: Vec<FabricRelationship>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FabricManagementSummary {
    pub component_count: u64,
    pub relationship_count: u64,
    pub owner_count: u64,
    pub by_kind: BTreeMap<FabricComponentKind, u64>,
    pub by_environment: BTreeMap<FabricEnvironment, u64>,
    pub by_classification: BTreeMap<FabricClassification, u64>,
    pub by_lifecycle: BTreeMap<FabricLifecycleState, u64>,
    pub by_health: BTreeMap<FabricHealth, u64>,
    pub by_availability: BTreeMap<FabricAvailability, u64>,
    pub evidence: FabricEvidenceCoverage,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FabricEvidenceCoverage {
    pub total: u64,
    pub by_kind: BTreeMap<FabricEvidenceKind, u64>,
    pub by_posture: BTreeMap<FabricEvidencePosture, u64>,
    pub missing_kinds: Vec<FabricEvidenceKind>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FabricManagementComponent {
    pub component_id: String,
    pub name: String,
    pub kind: FabricComponentKind,
    pub owner_id: String,
    pub version: String,
    pub revision: u64,
    pub environment: FabricEnvironment,
    pub classification: FabricClassification,
    pub lifecycle: FabricLifecycleState,
    pub provenance_source: FabricProvenanceSource,
    pub health: FabricHealth,
    pub availability: FabricAvailability,
    pub health_source: FabricObservationSource,
    pub health_evidence_id: Option<String>,
    pub evidence: FabricEvidenceCoverage,
    pub incoming_relationship_count: u64,
    pub outgoing_relationship_count: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FabricComponentDetail {
    pub schema_version: String,
    pub projection_digest: String,
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
    pub health: FabricHealth,
    pub availability: FabricAvailability,
    pub health_source: FabricObservationSource,
    pub health_evidence_id: Option<String>,
    pub evidence: Vec<FabricEvidenceReference>,
    pub evidence_coverage: FabricEvidenceCoverage,
    pub incoming_relationships: Vec<FabricRelationship>,
    pub outgoing_relationships: Vec<FabricRelationship>,
}

impl FabricReadScope {
    pub fn new<E, C>(environments: E, classifications: C) -> Result<Self, FabricManagementReadError>
    where
        E: IntoIterator<Item = FabricEnvironment>,
        C: IntoIterator<Item = FabricClassification>,
    {
        let environments = environments.into_iter().collect::<BTreeSet<_>>();
        let classifications = classifications.into_iter().collect::<BTreeSet<_>>();
        if environments.is_empty() || classifications.is_empty() {
            return Err(FabricManagementReadError::InvalidScope);
        }
        Ok(Self {
            environments,
            classifications,
            owner_ids: None,
            component_ids: None,
        })
    }

    pub fn restrict_to_owners<I, S>(
        mut self,
        owner_ids: I,
    ) -> Result<Self, FabricManagementReadError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.owner_ids = Some(narrow_filter(self.owner_ids.take(), owner_ids)?);
        Ok(self)
    }

    pub fn restrict_to_components<I, S>(
        mut self,
        component_ids: I,
    ) -> Result<Self, FabricManagementReadError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.component_ids = Some(narrow_filter(self.component_ids.take(), component_ids)?);
        Ok(self)
    }

    fn allows(&self, component: &FabricTopologyNode) -> bool {
        self.environments.contains(&component.environment)
            && self.classifications.contains(&component.classification)
            && self
                .owner_ids
                .as_ref()
                .is_none_or(|owners| owners.contains(&component.owner_id))
            && self
                .component_ids
                .as_ref()
                .is_none_or(|components| components.contains(&component.component_id))
    }
}

impl FabricRegistry {
    pub fn management_overview(
        &self,
        scope: &FabricReadScope,
    ) -> Result<FabricManagementOverview, FabricManagementReadError> {
        if self.len() > MAX_MANAGEMENT_COMPONENTS {
            return Err(FabricManagementReadError::ProjectionFailure);
        }
        let (relationship_count, evidence_count) = self
            .component_records()
            .try_fold((0_usize, 0_usize), |(relationships, evidence), record| {
                Some((
                    relationships.checked_add(record.dependencies.len())?,
                    evidence.checked_add(record.evidence.len())?,
                ))
            })
            .ok_or(FabricManagementReadError::ProjectionFailure)?;
        validate_projection_size(self.len(), relationship_count, evidence_count)?;
        let topology = self
            .topology()
            .map_err(|_| FabricManagementReadError::ProjectionFailure)?;
        let components = topology
            .components
            .into_iter()
            .filter(|component| scope.allows(component))
            .collect::<Vec<_>>();
        let visible_ids = components
            .iter()
            .map(|component| component.component_id.as_str())
            .collect::<BTreeSet<_>>();
        let relationships = topology
            .relationships
            .into_iter()
            .filter(|relationship| {
                visible_ids.contains(relationship.source_component_id.as_str())
                    && visible_ids.contains(relationship.target_component_id.as_str())
            })
            .collect::<Vec<_>>();
        if relationships.len() > MAX_MANAGEMENT_RELATIONSHIPS {
            return Err(FabricManagementReadError::ProjectionFailure);
        }

        let relationship_counts = relationship_counts(&relationships);
        let cards = components
            .iter()
            .map(|component| management_card(component, &relationship_counts))
            .collect::<Vec<_>>();
        let summary = management_summary(&components, &relationships);
        let projection_digest = overview_digest(&summary, &cards, &relationships)?;

        Ok(FabricManagementOverview {
            schema_version: FABRIC_MANAGEMENT_OVERVIEW_SCHEMA_VERSION.to_string(),
            projection_digest,
            summary,
            components: cards,
            relationships,
        })
    }

    pub fn component_detail(
        &self,
        scope: &FabricReadScope,
        component_id: &str,
    ) -> Result<FabricComponentDetail, FabricManagementReadError> {
        let overview = self.management_overview(scope)?;
        let component = overview
            .components
            .iter()
            .find(|component| component.component_id == component_id)
            .ok_or(FabricManagementReadError::NotAvailable)?;
        let record = self
            .component(component_id)
            .ok_or(FabricManagementReadError::NotAvailable)?;
        let incoming_relationships = overview
            .relationships
            .iter()
            .filter(|relationship| relationship.target_component_id == component_id)
            .cloned()
            .collect::<Vec<_>>();
        let outgoing_relationships = overview
            .relationships
            .iter()
            .filter(|relationship| relationship.source_component_id == component_id)
            .cloned()
            .collect::<Vec<_>>();

        let mut detail = FabricComponentDetail {
            schema_version: FABRIC_COMPONENT_DETAIL_SCHEMA_VERSION.to_string(),
            projection_digest: String::new(),
            component_id: component.component_id.clone(),
            name: component.name.clone(),
            kind: component.kind,
            owner_id: component.owner_id.clone(),
            version: component.version.clone(),
            revision: component.revision,
            environment: component.environment,
            classification: component.classification,
            lifecycle: component.lifecycle,
            provenance: record.provenance.clone(),
            health: component.health,
            availability: component.availability,
            health_source: component.health_source,
            health_evidence_id: component.health_evidence_id.clone(),
            evidence: record.evidence.clone(),
            evidence_coverage: component.evidence.clone(),
            incoming_relationships,
            outgoing_relationships,
        };
        detail.projection_digest = component_detail_digest(&detail)?;
        Ok(detail)
    }
}

fn validate_filter_ids<I, S>(ids: I) -> Result<BTreeSet<String>, FabricManagementReadError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut validated = BTreeSet::new();
    let mut item_count = 0_usize;
    for id in ids {
        item_count = item_count
            .checked_add(1)
            .ok_or(FabricManagementReadError::InvalidScope)?;
        if item_count > MAX_SCOPE_FILTERS {
            return Err(FabricManagementReadError::InvalidScope);
        }
        let id = id.into();
        super::model::validate_identifier(&id, "fabric_read_scope")
            .map_err(|_| FabricManagementReadError::InvalidScope)?;
        validated.insert(id);
    }
    if validated.is_empty() {
        return Err(FabricManagementReadError::InvalidScope);
    }
    Ok(validated)
}

fn narrow_filter<I, S>(
    existing: Option<BTreeSet<String>>,
    requested: I,
) -> Result<BTreeSet<String>, FabricManagementReadError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let requested = validate_filter_ids(requested)?;
    let narrowed = existing.map_or(requested.clone(), |current| {
        current.intersection(&requested).cloned().collect()
    });
    if narrowed.is_empty() {
        return Err(FabricManagementReadError::InvalidScope);
    }
    Ok(narrowed)
}

fn management_card(
    component: &FabricTopologyNode,
    relationship_counts: &BTreeMap<String, (u64, u64)>,
) -> FabricManagementComponent {
    let (incoming_relationship_count, outgoing_relationship_count) = relationship_counts
        .get(&component.component_id)
        .copied()
        .unwrap_or_default();
    FabricManagementComponent {
        component_id: component.component_id.clone(),
        name: component.name.clone(),
        kind: component.kind,
        owner_id: component.owner_id.clone(),
        version: component.version.clone(),
        revision: component.revision,
        environment: component.environment,
        classification: component.classification,
        lifecycle: component.lifecycle,
        provenance_source: component.provenance.source,
        health: component.health.health,
        availability: component.health.availability,
        health_source: component.health.source,
        health_evidence_id: component.health.evidence_id.clone(),
        evidence: evidence_coverage(component.evidence.iter()),
        incoming_relationship_count,
        outgoing_relationship_count,
    }
}

fn relationship_counts(relationships: &[FabricRelationship]) -> BTreeMap<String, (u64, u64)> {
    let mut counts = BTreeMap::<String, (u64, u64)>::new();
    for relationship in relationships {
        counts
            .entry(relationship.source_component_id.clone())
            .or_default()
            .1 += 1;
        counts
            .entry(relationship.target_component_id.clone())
            .or_default()
            .0 += 1;
    }
    counts
}

fn management_summary(
    components: &[FabricTopologyNode],
    relationships: &[FabricRelationship],
) -> FabricManagementSummary {
    let mut owners = BTreeSet::new();
    let mut by_kind = BTreeMap::new();
    let mut by_environment = BTreeMap::new();
    let mut by_classification = BTreeMap::new();
    let mut by_lifecycle = BTreeMap::new();
    let mut by_health = BTreeMap::new();
    let mut by_availability = BTreeMap::new();
    for component in components {
        owners.insert(component.owner_id.as_str());
        increment(&mut by_kind, component.kind);
        increment(&mut by_environment, component.environment);
        increment(&mut by_classification, component.classification);
        increment(&mut by_lifecycle, component.lifecycle);
        increment(&mut by_health, component.health.health);
        increment(&mut by_availability, component.health.availability);
    }

    FabricManagementSummary {
        component_count: components.len() as u64,
        relationship_count: relationships.len() as u64,
        owner_count: owners.len() as u64,
        by_kind,
        by_environment,
        by_classification,
        by_lifecycle,
        by_health,
        by_availability,
        evidence: evidence_coverage(components.iter().flat_map(|component| &component.evidence)),
    }
}

fn evidence_coverage<'a, I>(evidence: I) -> FabricEvidenceCoverage
where
    I: IntoIterator<Item = &'a FabricEvidenceReference>,
{
    let mut total = 0;
    let mut by_kind = BTreeMap::new();
    let mut by_posture = BTreeMap::new();
    for item in evidence {
        total += 1;
        increment(&mut by_kind, item.kind);
        increment(&mut by_posture, item.posture);
    }
    let missing_kinds = FabricEvidenceKind::ALL
        .into_iter()
        .filter(|kind| !by_kind.contains_key(kind))
        .collect();
    FabricEvidenceCoverage {
        total,
        by_kind,
        by_posture,
        missing_kinds,
    }
}

fn increment<K: Ord>(counts: &mut BTreeMap<K, u64>, key: K) {
    *counts.entry(key).or_insert(0) += 1;
}

#[derive(Serialize)]
struct OverviewDigestMaterial<'a> {
    schema_version: &'static str,
    summary: &'a FabricManagementSummary,
    components: &'a [FabricManagementComponent],
    relationships: &'a [FabricRelationship],
}

fn overview_digest(
    summary: &FabricManagementSummary,
    components: &[FabricManagementComponent],
    relationships: &[FabricRelationship],
) -> Result<String, FabricManagementReadError> {
    let material = OverviewDigestMaterial {
        schema_version: FABRIC_MANAGEMENT_OVERVIEW_SCHEMA_VERSION,
        summary,
        components,
        relationships,
    };
    digest(&material)
}

fn component_detail_digest(
    detail: &FabricComponentDetail,
) -> Result<String, FabricManagementReadError> {
    #[derive(Serialize)]
    struct DetailDigestMaterial<'a> {
        schema_version: &'static str,
        component_id: &'a str,
        name: &'a str,
        kind: FabricComponentKind,
        owner_id: &'a str,
        version: &'a str,
        revision: u64,
        environment: FabricEnvironment,
        classification: FabricClassification,
        lifecycle: FabricLifecycleState,
        provenance: &'a FabricProvenance,
        health: FabricHealth,
        availability: FabricAvailability,
        health_source: FabricObservationSource,
        health_evidence_id: &'a Option<String>,
        evidence: &'a [FabricEvidenceReference],
        evidence_coverage: &'a FabricEvidenceCoverage,
        incoming_relationships: &'a [FabricRelationship],
        outgoing_relationships: &'a [FabricRelationship],
    }
    digest(&DetailDigestMaterial {
        schema_version: FABRIC_COMPONENT_DETAIL_SCHEMA_VERSION,
        component_id: &detail.component_id,
        name: &detail.name,
        kind: detail.kind,
        owner_id: &detail.owner_id,
        version: &detail.version,
        revision: detail.revision,
        environment: detail.environment,
        classification: detail.classification,
        lifecycle: detail.lifecycle,
        provenance: &detail.provenance,
        health: detail.health,
        availability: detail.availability,
        health_source: detail.health_source,
        health_evidence_id: &detail.health_evidence_id,
        evidence: &detail.evidence,
        evidence_coverage: &detail.evidence_coverage,
        incoming_relationships: &detail.incoming_relationships,
        outgoing_relationships: &detail.outgoing_relationships,
    })
}

fn digest<T: Serialize>(value: &T) -> Result<String, FabricManagementReadError> {
    let bytes =
        serde_json::to_vec(value).map_err(|_| FabricManagementReadError::ProjectionFailure)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn validate_projection_size(
    component_count: usize,
    relationship_count: usize,
    evidence_count: usize,
) -> Result<(), FabricManagementReadError> {
    if component_count > MAX_MANAGEMENT_COMPONENTS
        || relationship_count > MAX_MANAGEMENT_RELATIONSHIPS
        || evidence_count > MAX_MANAGEMENT_EVIDENCE_REFERENCES
    {
        return Err(FabricManagementReadError::ProjectionFailure);
    }
    Ok(())
}

impl From<FabricRegistryError> for FabricManagementReadError {
    fn from(_: FabricRegistryError) -> Self {
        Self::ProjectionFailure
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn management_projection_limits_are_inclusive_and_fail_closed() {
        assert_eq!(
            validate_projection_size(
                MAX_MANAGEMENT_COMPONENTS,
                MAX_MANAGEMENT_RELATIONSHIPS,
                MAX_MANAGEMENT_EVIDENCE_REFERENCES,
            ),
            Ok(())
        );
        for rejected in [
            (MAX_MANAGEMENT_COMPONENTS + 1, 0, 0),
            (0, MAX_MANAGEMENT_RELATIONSHIPS + 1, 0),
            (0, 0, MAX_MANAGEMENT_EVIDENCE_REFERENCES + 1),
        ] {
            assert_eq!(
                validate_projection_size(rejected.0, rejected.1, rejected.2),
                Err(FabricManagementReadError::ProjectionFailure)
            );
        }
    }
}
