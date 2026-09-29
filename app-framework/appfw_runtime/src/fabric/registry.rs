//! Deterministic, in-memory registry reduction and topology projection.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::model::{
    relationship_allowed, FabricClassification, FabricComponentKind, FabricComponentRecord,
    FabricEnvironment, FabricEvidenceReference, FabricHealthPosture, FabricLifecycleState,
    FabricProvenance, FabricRegistryError, FabricRelationshipKind,
};

pub const FABRIC_TOPOLOGY_SCHEMA_VERSION: &str = "appfw.fabric.topology@1";

#[derive(Clone, Debug, Default)]
pub struct FabricRegistry {
    components: BTreeMap<String, FabricComponentRecord>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FabricTopologyProjection {
    pub schema_version: String,
    pub components: Vec<FabricTopologyNode>,
    pub relationships: Vec<FabricRelationship>,
    pub digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FabricTopologyNode {
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
    pub evidence: Vec<FabricEvidenceReference>,
    pub health: FabricHealthPosture,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FabricRelationship {
    pub source_component_id: String,
    pub target_component_id: String,
    pub kind: FabricRelationshipKind,
}

impl FabricRegistry {
    pub fn load<I>(records: I) -> Result<Self, FabricRegistryError>
    where
        I: IntoIterator<Item = FabricComponentRecord>,
    {
        let mut records = records.into_iter().collect::<Vec<_>>();
        for record in &mut records {
            record.validate_and_canonicalize()?;
        }
        records.sort_by(|left, right| {
            (&left.component_id, left.revision).cmp(&(&right.component_id, right.revision))
        });

        let mut components = BTreeMap::<String, FabricComponentRecord>::new();
        // The sort above makes reduction deterministic even when callers
        // provide revision history in a different order.
        for record in &records {
            if let Some(previous) = components.get(&record.component_id) {
                if previous.revision == record.revision {
                    return Err(FabricRegistryError::DuplicateRevision {
                        component_id: record.component_id.clone(),
                        revision: record.revision,
                    });
                }
                if previous.kind != record.kind {
                    return Err(FabricRegistryError::IdentityConflict {
                        component_id: record.component_id.clone(),
                        field: "kind",
                    });
                }
                if previous.environment != record.environment {
                    return Err(FabricRegistryError::IdentityConflict {
                        component_id: record.component_id.clone(),
                        field: "environment",
                    });
                }
                if !previous.lifecycle.can_transition_to(record.lifecycle) {
                    return Err(FabricRegistryError::InvalidLifecycleTransition {
                        component_id: record.component_id.clone(),
                    });
                }
            }
            components.insert(record.component_id.clone(), record.clone());
        }

        let registry = Self { components };
        registry.validate_relationships(records.iter())?;
        Ok(registry)
    }

    pub fn len(&self) -> usize {
        self.components.len()
    }

    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    pub fn component(&self, component_id: &str) -> Option<&FabricComponentRecord> {
        self.components.get(component_id)
    }

    pub(crate) fn component_records(&self) -> impl Iterator<Item = &FabricComponentRecord> {
        self.components.values()
    }

    pub fn topology(&self) -> Result<FabricTopologyProjection, FabricRegistryError> {
        let components = self
            .components
            .values()
            .map(FabricTopologyNode::from)
            .collect::<Vec<_>>();
        let mut relationships = self
            .components
            .values()
            .flat_map(|component| {
                component
                    .dependencies
                    .iter()
                    .map(move |dependency| FabricRelationship {
                        source_component_id: component.component_id.clone(),
                        target_component_id: dependency.component_id.clone(),
                        kind: dependency.relationship,
                    })
            })
            .collect::<Vec<_>>();
        relationships.sort();

        let digest = topology_digest(&components, &relationships)?;
        Ok(FabricTopologyProjection {
            schema_version: FABRIC_TOPOLOGY_SCHEMA_VERSION.to_string(),
            components,
            relationships,
            digest,
        })
    }

    fn validate_relationships<'a, I>(&self, records: I) -> Result<(), FabricRegistryError>
    where
        I: IntoIterator<Item = &'a FabricComponentRecord>,
    {
        for component in records {
            for dependency in &component.dependencies {
                let Some(target) = self.components.get(&dependency.component_id) else {
                    return Err(FabricRegistryError::DanglingRelationship {
                        component_id: component.component_id.clone(),
                        target_id: dependency.component_id.clone(),
                    });
                };
                if !relationship_allowed(component.kind, target.kind, dependency.relationship) {
                    return Err(FabricRegistryError::InvalidRelationship {
                        component_id: component.component_id.clone(),
                        target_id: target.component_id.clone(),
                    });
                }
                if dependency.relationship == FabricRelationshipKind::Contains
                    && component.owner_id != target.owner_id
                {
                    return Err(FabricRegistryError::InvalidRelationship {
                        component_id: component.component_id.clone(),
                        target_id: target.component_id.clone(),
                    });
                }
            }
        }
        Ok(())
    }
}

impl From<&FabricComponentRecord> for FabricTopologyNode {
    fn from(record: &FabricComponentRecord) -> Self {
        Self {
            component_id: record.component_id.clone(),
            name: record.name.clone(),
            kind: record.kind,
            owner_id: record.owner_id.clone(),
            version: record.version.clone(),
            revision: record.revision,
            environment: record.environment,
            classification: record.classification,
            lifecycle: record.lifecycle,
            provenance: record.provenance.clone(),
            evidence: record.evidence.clone(),
            health: record.health.clone(),
        }
    }
}

#[derive(Serialize)]
struct TopologyDigestMaterial<'a> {
    schema_version: &'static str,
    components: &'a [FabricTopologyNode],
    relationships: &'a [FabricRelationship],
}

fn topology_digest(
    components: &[FabricTopologyNode],
    relationships: &[FabricRelationship],
) -> Result<String, FabricRegistryError> {
    let material = TopologyDigestMaterial {
        schema_version: FABRIC_TOPOLOGY_SCHEMA_VERSION,
        components,
        relationships,
    };
    let bytes = serde_json::to_vec(&material).map_err(|_| FabricRegistryError::DigestFailure)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}
