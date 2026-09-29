use std::{
    collections::{hash_map::DefaultHasher, BTreeSet},
    fs,
    hash::{Hash, Hasher},
};

use crate::utils::context;
use anyhow::{anyhow, Context, Result};
use serde_json::json;

use super::files;
use super::{artifacts, console};
use crate::app_workspace::AppWorkspace;
use crate::bootstrap_types::entity_type::{
    Computed, DataType, EntityType, ForeignKey, ManyToManyProperty, NavByFkProperty, PropertyType,
};
use crate::relationship_config::{
    RelationshipConfig, RelationshipEndpoint, RelationshipKind, RelationshipStorage,
    RelationshipStorageType,
};

pub fn run(workspace: &AppWorkspace) -> Result<Vec<EntityType>> {
    let mut all_types = load_all_types(workspace)?;
    let schema_relationships =
        crate::relationship_config::read_all_schema_relationships(workspace)?;

    for schema_relationships in &schema_relationships {
        apply_schema_relationships(
            &mut all_types,
            &schema_relationships.schema_name,
            &schema_relationships.relationships,
        )?;
    }

    let all_types_ref = &all_types.clone(); // Create an immutable reference to all_types

    // Validate relationships before processing
    validate_relationships(&all_types)?;

    for entity_type in all_types.iter_mut() {
        console::verbose(format!(
            "relationships: {}.{}",
            entity_type.schema_name, entity_type.pascal_1
        ));
        for prop in entity_type.props.iter_mut() {
            console::verbose(format!("property: {}", prop.name));
            if let Some(nav) = prop.nav_by_fk_property.as_mut() {
                console::verbose(format!("nav_by_fk_property: {:?}", nav));
                let other = all_types_ref
                    .iter()
                    .find(|t| t.pascal_1 == nav.type_name && t.schema_name == nav.schema_name)
                    .with_context(|| {
                        format!(
                            "navigation target not found: {}.{}",
                            nav.schema_name, nav.type_name
                        )
                    })?;

                // Resolution rule: the resolved type is the entity on the OTHER side of the FK.
                //   - Outgoing NavToOne (this entity owns the FK; nav.type_name == self): resolved = FK target on self
                //   - Incoming NavToOne (other entity owns FK pointing back; nav.type_name == other): resolved = other
                //   - NavToMany is always incoming: resolved = nav.type_name
                let is_incoming =
                    nav.type_name != entity_type.pascal_1 || prop.data_type == DataType::NavToMany;
                if is_incoming {
                    let foreign_key = ForeignKey {
                        schema_name: nav.schema_name.clone(),
                        type_name: nav.type_name.clone(),
                    };
                    nav.resolved = Some(foreign_key);
                } else {
                    let fk_prop = other
                        .props
                        .iter()
                        .find(|p| p.name == nav.prop_name)
                        .with_context(|| {
                            format!(
                                "navigation property not found: {}.{}.{}",
                                other.schema_name, other.pascal_1, nav.prop_name
                            )
                        })?;
                    let foreign_key = fk_prop.foreign_key.clone().with_context(|| {
                        format!(
                            "navigation property has no foreign_key: {}.{}.{}",
                            other.schema_name, other.pascal_1, fk_prop.name
                        )
                    })?;
                    nav.resolved = Some(foreign_key);
                }
            }
        }
    }

    persist(workspace, all_types)
}

fn apply_schema_relationships(
    all_types: &mut Vec<EntityType>,
    default_schema: &str,
    relationships: &[RelationshipConfig],
) -> Result<()> {
    for relationship in relationships {
        console::verbose(format!(
            "schema relationship: {default_schema}.{}",
            relationship.name
        ));
        match relationship.kind {
            RelationshipKind::OneToMany => {
                let one = require_endpoint(
                    relationship.one.as_ref(),
                    &relationship.name,
                    "one",
                    default_schema,
                )?;
                let many = require_endpoint(
                    relationship.many.as_ref(),
                    &relationship.name,
                    "many",
                    default_schema,
                )?;
                let storage = require_storage(relationship)?;
                if storage.storage_type != RelationshipStorageType::ForeignKey {
                    return Err(anyhow!(
                        "relationship `{}` has unsupported storage type",
                        relationship.name
                    ));
                }
                let owner_schema = storage
                    .owner_schema
                    .as_deref()
                    .unwrap_or(many.schema.as_str());
                if owner_schema != many.schema || storage.owner != many.entity {
                    return Err(anyhow!(
                        "OneToMany relationship `{}` must store the foreign key on the `many` endpoint {}.{}",
                        relationship.name,
                        many.schema,
                        many.entity
                    ));
                }
                ensure_fk_targets(all_types, &relationship.name, &many, &storage.field, &one)?;

                upsert_relationship_property(
                    all_types,
                    &one.schema,
                    &one.entity,
                    nav_property(
                        default_schema,
                        relationship,
                        &one,
                        DataType::NavToMany,
                        Some(NavByFkProperty {
                            schema_name: many.schema.clone(),
                            type_name: many.entity.clone(),
                            prop_name: storage.field.clone(),
                            filter: None,
                            resolved: None,
                        }),
                        None,
                        "Many",
                        "InverseForeignKey",
                    ),
                )?;
                upsert_relationship_property(
                    all_types,
                    &many.schema,
                    &many.entity,
                    nav_property(
                        default_schema,
                        relationship,
                        &many,
                        DataType::NavToOne,
                        Some(NavByFkProperty {
                            schema_name: many.schema.clone(),
                            type_name: many.entity.clone(),
                            prop_name: storage.field.clone(),
                            filter: None,
                            resolved: None,
                        }),
                        None,
                        "One",
                        "DirectForeignKey",
                    ),
                )?;
            }
            RelationshipKind::OneToOne => {
                let left = require_endpoint(
                    relationship.left.as_ref(),
                    &relationship.name,
                    "left",
                    default_schema,
                )?;
                let right = require_endpoint(
                    relationship.right.as_ref(),
                    &relationship.name,
                    "right",
                    default_schema,
                )?;
                let storage = require_storage(relationship)?;
                if storage.storage_type != RelationshipStorageType::ForeignKey {
                    return Err(anyhow!(
                        "relationship `{}` has unsupported storage type",
                        relationship.name
                    ));
                }
                let owner_schema = storage.owner_schema.as_deref().unwrap_or(default_schema);
                let owner = if owner_schema == left.schema && storage.owner == left.entity {
                    left.clone()
                } else if owner_schema == right.schema && storage.owner == right.entity {
                    right.clone()
                } else {
                    return Err(anyhow!(
                        "OneToOne relationship `{}` storage.owner must be one endpoint",
                        relationship.name
                    ));
                };
                let other = if owner.schema == left.schema && owner.entity == left.entity {
                    right.clone()
                } else {
                    left.clone()
                };
                ensure_fk_targets(
                    all_types,
                    &relationship.name,
                    &owner,
                    &storage.field,
                    &other,
                )?;

                for endpoint in [left, right] {
                    let storage_path =
                        if endpoint.schema == owner.schema && endpoint.entity == owner.entity {
                            "DirectForeignKey"
                        } else {
                            "InverseForeignKey"
                        };
                    upsert_relationship_property(
                        all_types,
                        &endpoint.schema,
                        &endpoint.entity,
                        nav_property(
                            default_schema,
                            relationship,
                            &endpoint,
                            DataType::NavToOne,
                            Some(NavByFkProperty {
                                schema_name: owner.schema.clone(),
                                type_name: owner.entity.clone(),
                                prop_name: storage.field.clone(),
                                filter: None,
                                resolved: None,
                            }),
                            None,
                            "One",
                            storage_path,
                        ),
                    )?;
                }
            }
            RelationshipKind::ManyToMany => {
                let left = require_endpoint(
                    relationship.left.as_ref(),
                    &relationship.name,
                    "left",
                    default_schema,
                )?;
                let right = require_endpoint(
                    relationship.right.as_ref(),
                    &relationship.name,
                    "right",
                    default_schema,
                )?;
                let junction = relationship.junction.as_ref().ok_or_else(|| {
                    anyhow!(
                        "ManyToMany relationship `{}` is missing `junction`",
                        relationship.name
                    )
                })?;
                ensure_entity_exists(all_types, &left.schema, &left.entity)?;
                ensure_entity_exists(all_types, &right.schema, &right.entity)?;

                let junction_schema = junction
                    .schema
                    .clone()
                    .unwrap_or_else(|| default_schema.to_string());
                upsert_relationship_property(
                    all_types,
                    &left.schema,
                    &left.entity,
                    nav_property(
                        default_schema,
                        relationship,
                        &left,
                        DataType::ManyToMany,
                        None,
                        Some(ManyToManyProperty {
                            junction_table: junction.entity.clone(),
                            junction_schema: Some(junction_schema.clone()),
                            local_key: junction.left_key.clone(),
                            foreign_key: junction.right_key.clone(),
                            target_schema: right.schema.clone(),
                            target_type: right.entity.clone(),
                        }),
                        "Many",
                        "Junction",
                    ),
                )?;
                upsert_relationship_property(
                    all_types,
                    &right.schema,
                    &right.entity,
                    nav_property(
                        default_schema,
                        relationship,
                        &right,
                        DataType::ManyToMany,
                        None,
                        Some(ManyToManyProperty {
                            junction_table: junction.entity.clone(),
                            junction_schema: Some(junction_schema),
                            local_key: junction.right_key.clone(),
                            foreign_key: junction.left_key.clone(),
                            target_schema: left.schema.clone(),
                            target_type: left.entity.clone(),
                        }),
                        "Many",
                        "Junction",
                    ),
                )?;
            }
        }
    }

    Ok(())
}

#[derive(Clone)]
struct ResolvedEndpoint {
    schema: String,
    entity: String,
    field: String,
    caption: Option<String>,
}

fn require_endpoint(
    endpoint: Option<&RelationshipEndpoint>,
    relationship_name: &str,
    label: &str,
    default_schema: &str,
) -> Result<ResolvedEndpoint> {
    let endpoint = endpoint.ok_or_else(|| {
        anyhow!("relationship `{relationship_name}` is missing `{label}` endpoint")
    })?;
    Ok(ResolvedEndpoint {
        schema: endpoint
            .schema
            .clone()
            .unwrap_or_else(|| default_schema.to_string()),
        entity: endpoint.entity.clone(),
        field: endpoint.field.clone(),
        caption: endpoint.caption.clone(),
    })
}

fn require_storage(relationship: &RelationshipConfig) -> Result<&RelationshipStorage> {
    relationship.storage.as_ref().ok_or_else(|| {
        anyhow!(
            "{:?} relationship `{}` is missing `storage`",
            relationship.kind,
            relationship.name
        )
    })
}

fn ensure_entity_exists(
    all_types: &[EntityType],
    schema_name: &str,
    entity_name: &str,
) -> Result<()> {
    if all_types
        .iter()
        .any(|entity| entity.schema_name == schema_name && entity.pascal_1 == entity_name)
    {
        Ok(())
    } else {
        Err(anyhow!(
            "relationship references missing entity {schema_name}.{entity_name}"
        ))
    }
}

fn ensure_fk_targets(
    all_types: &[EntityType],
    relationship_name: &str,
    owner: &ResolvedEndpoint,
    fk_field: &str,
    target: &ResolvedEndpoint,
) -> Result<()> {
    let owner_entity = all_types
        .iter()
        .find(|entity| entity.schema_name == owner.schema && entity.pascal_1 == owner.entity)
        .ok_or_else(|| {
            anyhow!(
                "relationship `{relationship_name}` references missing FK owner {}.{}",
                owner.schema,
                owner.entity
            )
        })?;
    let fk_prop = owner_entity
        .props
        .iter()
        .find(|prop| prop.name == fk_field)
        .ok_or_else(|| {
            anyhow!(
                "relationship `{relationship_name}` references missing FK field {}.{}.{}",
                owner.schema,
                owner.entity,
                fk_field
            )
        })?;
    let fk = fk_prop.foreign_key.as_ref().ok_or_else(|| {
        anyhow!(
            "relationship `{relationship_name}` storage field {}.{}.{} is not a foreign_key property",
            owner.schema,
            owner.entity,
            fk_field
        )
    })?;
    let fk_schema = if fk.schema_name.trim().is_empty() {
        owner.schema.as_str()
    } else {
        fk.schema_name.as_str()
    };
    if fk_schema != target.schema || fk.type_name != target.entity {
        return Err(anyhow!(
            "relationship `{relationship_name}` FK {}.{}.{} targets {}.{}, expected {}.{}",
            owner.schema,
            owner.entity,
            fk_field,
            fk_schema,
            fk.type_name,
            target.schema,
            target.entity
        ));
    }
    Ok(())
}

fn nav_property(
    default_schema: &str,
    relationship: &RelationshipConfig,
    endpoint: &ResolvedEndpoint,
    data_type: DataType,
    nav_by_fk_property: Option<NavByFkProperty>,
    many_to_many_property: Option<ManyToManyProperty>,
    cardinality: &str,
    storage_path: &str,
) -> PropertyType {
    PropertyType {
        id: stable_property_id(&endpoint.schema, &endpoint.entity, &endpoint.field),
        name: endpoint.field.clone(),
        caption: endpoint
            .caption
            .clone()
            .unwrap_or_else(|| caption_from_name(&endpoint.field)),
        is_key: false,
        is_caption: false,
        is_required: false,
        is_read_only: false,
        is_concurrency_control: false,
        data_type,
        computed: Computed::None,
        default_value: None,
        foreign_key: None,
        nav_by_fk_property,
        many_to_many_property,
        nested_entity_type: None,
        enum_type_name: None,
        meta: Some(json!({
            "relationship": {
                "name": relationship.name,
                "kind": format!("{:?}", relationship.kind),
                "source_schema": default_schema,
                "cardinality": cardinality,
                "storage_path": storage_path
            }
        })),
    }
}

fn upsert_relationship_property(
    all_types: &mut [EntityType],
    schema_name: &str,
    entity_name: &str,
    mut generated: PropertyType,
) -> Result<()> {
    let entity = all_types
        .iter_mut()
        .find(|entity| entity.schema_name == schema_name && entity.pascal_1 == entity_name)
        .ok_or_else(|| {
            anyhow!("relationship target entity not found: {schema_name}.{entity_name}")
        })?;

    if let Some(existing) = entity
        .props
        .iter_mut()
        .find(|prop| prop.name == generated.name)
    {
        if !matches!(
            existing.data_type,
            DataType::NavToOne | DataType::NavToMany | DataType::ManyToMany
        ) {
            return Err(anyhow!(
                "relationship field {}.{}.{} conflicts with native property",
                schema_name,
                entity_name,
                generated.name
            ));
        }

        generated.id = existing.id.clone();
        if !existing.caption.trim().is_empty() {
            generated.caption = existing.caption.clone();
        }
        *existing = generated;
    } else {
        entity.props.push(generated);
    }

    Ok(())
}

fn stable_property_id(schema_name: &str, entity_name: &str, field_name: &str) -> String {
    let mut hasher = DefaultHasher::new();
    format!("{schema_name}.{entity_name}.{field_name}").hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn caption_from_name(name: &str) -> String {
    let mut words = vec![];
    for part in name.split('_').filter(|part| !part.is_empty()) {
        let mut chars = part.chars();
        let Some(first) = chars.next() else {
            continue;
        };
        words.push(format!("{}{}", first.to_ascii_uppercase(), chars.as_str()));
    }
    words.join(" ")
}

pub fn load_all_types(workspace: &AppWorkspace) -> Result<Vec<EntityType>> {
    let mut all_types: Vec<EntityType> = vec![];

    let schemas_dir = workspace.schemas_root();
    for dir_entry in fs::read_dir(&schemas_dir)
        .with_context(|| format!("could not read {}", schemas_dir.display()))?
    {
        let dir_path = dir_entry
            .with_context(|| format!("could not read entry in {}", schemas_dir.display()))?
            .path();
        if dir_path.is_dir() {
            let dir_name = dir_path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| anyhow!("invalid schema directory name {}", dir_path.display()))?
                .to_string();
            let entity_types_file = &files::get_entity_types_file(workspace, &dir_name);
            let mut schema_types = context::read_types(entity_types_file)?;

            all_types.append(&mut schema_types);
        }
    }
    Ok(all_types)
}

fn persist(workspace: &AppWorkspace, all_types: Vec<EntityType>) -> Result<Vec<EntityType>> {
    let schemas_dir = workspace.schemas_root();
    for dir_entry in fs::read_dir(&schemas_dir)
        .with_context(|| format!("could not read {}", schemas_dir.display()))?
    {
        let dir_path = dir_entry
            .with_context(|| format!("could not read entry in {}", schemas_dir.display()))?
            .path();
        if dir_path.is_dir() {
            let dir_name = dir_path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| anyhow!("invalid schema directory name {}", dir_path.display()))?
                .to_string();

            let schema_types: Vec<EntityType> = all_types
                .iter()
                .filter(|t| t.schema_name == dir_name)
                .cloned()
                .collect();

            // Serialize the updated types back to YAML
            let updated_yaml = serde_yaml::to_string(&schema_types)
                .context("could not serialize updated schema types")?;

            let entity_types_file = &files::get_entity_types_file(workspace, &dir_name);
            artifacts::safe_write(entity_types_file, updated_yaml)?;
        }
    }
    Ok(all_types)
}

/// Validate relationships for common issues
fn validate_relationships(all_types: &Vec<EntityType>) -> Result<()> {
    console::step("validate relationships");

    // Check for missing foreign key targets
    validate_foreign_key_targets(all_types)?;

    // Check for circular references in NavToMany relationships
    validate_circular_references(all_types)?;

    console::step("relationship validation passed");
    Ok(())
}

/// Validate that all foreign key targets exist
fn validate_foreign_key_targets(all_types: &Vec<EntityType>) -> Result<()> {
    for entity_type in all_types {
        for prop in &entity_type.props {
            if let Some(fk) = &prop.foreign_key {
                let target_exists = all_types
                    .iter()
                    .any(|t| t.pascal_1 == fk.type_name && t.schema_name == fk.schema_name);

                if !target_exists {
                    return Err(anyhow!(
                        "Foreign key target not found: {}.{} -> {}.{}",
                        entity_type.schema_name,
                        entity_type.pascal_1,
                        fk.schema_name,
                        fk.type_name
                    ));
                }
            }

            // Also validate nav_by_fk_property targets
            if let Some(nav) = &prop.nav_by_fk_property {
                let nav_target_exists = all_types
                    .iter()
                    .any(|t| t.pascal_1 == nav.type_name && t.schema_name == nav.schema_name);

                if !nav_target_exists {
                    return Err(anyhow!(
                        "Navigation target not found: {}.{}.{} -> {}.{}",
                        entity_type.schema_name,
                        entity_type.pascal_1,
                        prop.name,
                        nav.schema_name,
                        nav.type_name
                    ));
                }

                // Validate that the target has the specified property
                if let Some(target_type) = all_types
                    .iter()
                    .find(|t| t.pascal_1 == nav.type_name && t.schema_name == nav.schema_name)
                {
                    let prop_exists = target_type.props.iter().any(|p| p.name == nav.prop_name);
                    if !prop_exists {
                        return Err(anyhow!(
                            "Navigation property not found: {}.{}.{} -> {}.{}.{}",
                            entity_type.schema_name,
                            entity_type.pascal_1,
                            prop.name,
                            nav.schema_name,
                            nav.type_name,
                            nav.prop_name
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

/// Warn only on collection-to-collection navigation cycles.
fn validate_circular_references(all_types: &Vec<EntityType>) -> Result<()> {
    let mut warned_pairs = BTreeSet::new();

    for entity_type in all_types {
        for prop in &entity_type.props {
            if prop.data_type != DataType::NavToMany {
                continue;
            }

            if let Some(nav) = &prop.nav_by_fk_property {
                if let Some(target_type) = all_types
                    .iter()
                    .find(|t| t.pascal_1 == nav.type_name && t.schema_name == nav.schema_name)
                {
                    for reverse_prop in target_type
                        .props
                        .iter()
                        .filter(|p| p.data_type == DataType::NavToMany)
                    {
                        let Some(reverse_nav) = &reverse_prop.nav_by_fk_property else {
                            continue;
                        };

                        if reverse_nav.type_name != entity_type.pascal_1
                            || reverse_nav.schema_name != entity_type.schema_name
                        {
                            continue;
                        }

                        let left = format!(
                            "{}.{}.{}",
                            entity_type.schema_name, entity_type.pascal_1, prop.name
                        );
                        let right = format!(
                            "{}.{}.{}",
                            target_type.schema_name, target_type.pascal_1, reverse_prop.name
                        );
                        let pair_key = if left <= right {
                            (left.clone(), right.clone())
                        } else {
                            (right.clone(), left.clone())
                        };

                        if warned_pairs.insert(pair_key) {
                            console::warn(format!(
                                "potential circular NavToMany reference: {left} <-> {right}",
                            ));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relationship_config::{
        RelationshipEndpoint, RelationshipJunction, RelationshipStorage,
    };

    #[test]
    fn schema_one_to_many_generates_directional_nav_fields() {
        let mut entity_types = vec![
            entity("crm", "Account", vec![id_prop()]),
            entity(
                "crm",
                "Contact",
                vec![id_prop(), fk_prop("account_id", "Account")],
            ),
        ];
        let relationship = RelationshipConfig {
            name: "account_contacts".to_string(),
            kind: RelationshipKind::OneToMany,
            left: None,
            right: None,
            one: Some(endpoint("Account", "contacts")),
            many: Some(endpoint("Contact", "account")),
            storage: Some(RelationshipStorage {
                storage_type: RelationshipStorageType::ForeignKey,
                owner_schema: None,
                owner: "Contact".to_string(),
                field: "account_id".to_string(),
            }),
            junction: None,
        };

        apply_schema_relationships(&mut entity_types, "crm", &[relationship])
            .expect("relationship should apply");

        let account = find_entity(&entity_types, "Account");
        let contacts = find_prop(account, "contacts");
        assert_eq!(contacts.data_type, DataType::NavToMany);
        let contacts_nav = contacts.nav_by_fk_property.as_ref().expect("contacts nav");
        assert_eq!(contacts_nav.type_name, "Contact");
        assert_eq!(contacts_nav.prop_name, "account_id");

        let contact = find_entity(&entity_types, "Contact");
        let account_nav = find_prop(contact, "account");
        assert_eq!(account_nav.data_type, DataType::NavToOne);
        assert_eq!(
            account_nav
                .meta
                .as_ref()
                .and_then(|meta| meta.pointer("/relationship/storage_path"))
                .and_then(serde_json::Value::as_str),
            Some("DirectForeignKey")
        );
    }

    #[test]
    fn schema_many_to_many_generates_junction_backed_many_fields_on_both_sides() {
        let mut entity_types = vec![
            entity("crm", "Contact", vec![id_prop()]),
            entity("crm", "Opportunity", vec![id_prop()]),
        ];
        let relationship = RelationshipConfig {
            name: "opportunity_contacts".to_string(),
            kind: RelationshipKind::ManyToMany,
            left: Some(endpoint("Opportunity", "contacts")),
            right: Some(endpoint("Contact", "opportunities")),
            one: None,
            many: None,
            storage: None,
            junction: Some(RelationshipJunction {
                schema: Some("crm".to_string()),
                entity: "OpportunityContact".to_string(),
                left_key: "opportunity_id".to_string(),
                right_key: "contact_id".to_string(),
            }),
        };

        apply_schema_relationships(&mut entity_types, "crm", &[relationship])
            .expect("relationship should apply");

        let opportunity = find_entity(&entity_types, "Opportunity");
        let contacts = find_prop(opportunity, "contacts");
        assert_eq!(contacts.data_type, DataType::ManyToMany);
        let contacts_m2m = contacts
            .many_to_many_property
            .as_ref()
            .expect("contacts many-to-many");
        assert_eq!(contacts_m2m.local_key, "opportunity_id");
        assert_eq!(contacts_m2m.foreign_key, "contact_id");
        assert_eq!(contacts_m2m.target_type, "Contact");

        let contact = find_entity(&entity_types, "Contact");
        let opportunities = find_prop(contact, "opportunities");
        let opportunities_m2m = opportunities
            .many_to_many_property
            .as_ref()
            .expect("opportunities many-to-many");
        assert_eq!(opportunities_m2m.local_key, "contact_id");
        assert_eq!(opportunities_m2m.foreign_key, "opportunity_id");
        assert_eq!(opportunities_m2m.target_type, "Opportunity");
    }

    fn entity(schema_name: &str, name: &str, props: Vec<PropertyType>) -> EntityType {
        EntityType {
            id: format!("{schema_name}-{name}"),
            schema_name: schema_name.to_string(),
            schema_id: None,
            pascal_1: name.to_string(),
            pascal_n: format!("{name}s"),
            snake_1: name.to_ascii_lowercase(),
            snake_n: format!("{}s", name.to_ascii_lowercase()),
            caption_1: name.to_string(),
            caption_n: format!("{name}s"),
            is_union: false,
            base_type: None,
            is_table: true,
            facets: None,
            indexes: None,
            constraints: None,
            meta: None,
            execution: None,
            standard_methods: None,
            custom_methods: None,
            props,
        }
    }

    fn endpoint(entity: &str, field: &str) -> RelationshipEndpoint {
        RelationshipEndpoint {
            schema: None,
            entity: entity.to_string(),
            field: field.to_string(),
            caption: None,
        }
    }

    fn id_prop() -> PropertyType {
        scalar_prop("id", DataType::Uuid, true)
    }

    fn fk_prop(name: &str, target: &str) -> PropertyType {
        let mut prop = scalar_prop(name, DataType::Uuid, false);
        prop.foreign_key = Some(ForeignKey {
            schema_name: "crm".to_string(),
            type_name: target.to_string(),
        });
        prop
    }

    fn scalar_prop(name: &str, data_type: DataType, is_key: bool) -> PropertyType {
        PropertyType {
            id: name.to_string(),
            name: name.to_string(),
            caption: name.to_string(),
            is_key,
            is_caption: false,
            is_required: false,
            is_read_only: false,
            is_concurrency_control: false,
            data_type,
            computed: Computed::None,
            default_value: None,
            foreign_key: None,
            nav_by_fk_property: None,
            many_to_many_property: None,
            nested_entity_type: None,
            enum_type_name: None,
            meta: None,
        }
    }

    fn find_entity<'a>(entity_types: &'a [EntityType], name: &str) -> &'a EntityType {
        entity_types
            .iter()
            .find(|entity| entity.pascal_1 == name)
            .expect("entity exists")
    }

    fn find_prop<'a>(entity: &'a EntityType, name: &str) -> &'a PropertyType {
        entity
            .props
            .iter()
            .find(|prop| prop.name == name)
            .expect("property exists")
    }
}
