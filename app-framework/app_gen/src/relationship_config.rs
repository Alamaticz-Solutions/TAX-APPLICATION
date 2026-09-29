use std::fs;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::app_workspace::AppWorkspace;
use crate::utils::files;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipKind {
    OneToOne,
    OneToMany,
    ManyToMany,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipStorageType {
    ForeignKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipEndpoint {
    pub schema: Option<String>,
    pub entity: String,
    pub field: String,
    pub caption: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipStorage {
    #[serde(rename = "type")]
    pub storage_type: RelationshipStorageType,
    pub owner_schema: Option<String>,
    pub owner: String,
    pub field: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipJunction {
    pub schema: Option<String>,
    pub entity: String,
    pub left_key: String,
    pub right_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipConfig {
    pub name: String,
    pub kind: RelationshipKind,
    pub left: Option<RelationshipEndpoint>,
    pub right: Option<RelationshipEndpoint>,
    pub one: Option<RelationshipEndpoint>,
    pub many: Option<RelationshipEndpoint>,
    pub storage: Option<RelationshipStorage>,
    pub junction: Option<RelationshipJunction>,
}

#[derive(Debug, Clone)]
pub struct SchemaRelationships {
    pub schema_name: String,
    pub relationships: Vec<RelationshipConfig>,
}

pub fn read_schema_relationships(
    workspace: &AppWorkspace,
    schema_name: &str,
) -> Result<Vec<RelationshipConfig>> {
    let file = files::get_relationships_file(workspace, schema_name);
    if !file.exists() {
        return Ok(vec![]);
    }
    let handle =
        fs::File::open(&file).with_context(|| format!("could not open {}", file.display()))?;
    let relationships = serde_yaml::from_reader(handle)
        .with_context(|| format!("could not parse relationships YAML {}", file.display()))?;
    Ok(relationships)
}

pub fn read_all_schema_relationships(workspace: &AppWorkspace) -> Result<Vec<SchemaRelationships>> {
    let mut result = vec![];
    for schema_name in
        files::get_schema_dir_names(workspace).map_err(|err| anyhow::anyhow!("{err}"))?
    {
        result.push(SchemaRelationships {
            relationships: read_schema_relationships(workspace, &schema_name)?,
            schema_name,
        });
    }
    Ok(result)
}
