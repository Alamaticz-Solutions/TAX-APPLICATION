use std::collections::HashMap;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::{
    app_workspace::AppWorkspace,
    bootstrap_types::{
        data_source::{DataSource, Schema},
        entity_type::{
            CustomMethod, CustomMethodKind, DataAccessExecution, DataType, EntityType,
            PropertyType, ProviderRoutineBinding, ProviderRoutineName, StandardMethod,
        },
    },
    relationship_config::{self, RelationshipConfig},
    utils::{artifacts, console, context, files},
};

#[derive(Debug, Serialize)]
pub struct GeneratorIr {
    pub version: u32,
    pub generated_at: String,
    pub schemas: Vec<NormalizedSchema>,
}

#[derive(Debug, Serialize)]
pub struct NormalizedSchema {
    pub name: String,
    pub(crate) data_source_name: String,
    pub(crate) data_source_type: String,
    pub is_system_schema: bool,
    pub(crate) relationships: Vec<NormalizedRelationship>,
    pub(crate) entities: Vec<NormalizedEntity>,
}

#[derive(Debug, Serialize)]
pub(crate) struct NormalizedRelationship {
    pub(crate) name: String,
    pub(crate) kind: String,
    pub(crate) left: Option<NormalizedRelationshipEndpoint>,
    pub(crate) right: Option<NormalizedRelationshipEndpoint>,
    pub(crate) one: Option<NormalizedRelationshipEndpoint>,
    pub(crate) many: Option<NormalizedRelationshipEndpoint>,
    pub(crate) storage: Option<NormalizedRelationshipStorage>,
    pub(crate) junction: Option<NormalizedRelationshipJunction>,
}

#[derive(Debug, Serialize)]
pub(crate) struct NormalizedRelationshipEndpoint {
    pub(crate) schema_name: Option<String>,
    pub(crate) entity_name: String,
    pub(crate) field_name: String,
    pub(crate) caption: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct NormalizedRelationshipStorage {
    pub(crate) storage_type: String,
    pub(crate) owner_schema: Option<String>,
    pub(crate) owner: String,
    pub(crate) field: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct NormalizedRelationshipJunction {
    pub(crate) schema_name: Option<String>,
    pub(crate) entity_name: String,
    pub(crate) left_key: String,
    pub(crate) right_key: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct NormalizedEntity {
    pub(crate) schema_name: String,
    pub(crate) name: String,
    pub(crate) table_name: String,
    pub(crate) caption_singular: String,
    pub(crate) caption_plural: String,
    pub(crate) snake_singular: String,
    pub(crate) is_table: bool,
    pub(crate) is_union: bool,
    pub(crate) base_type: Option<String>,
    pub(crate) facets: Vec<String>,
    pub(crate) execution: NormalizedDataAccessExecution,
    pub(crate) has_standard_methods: bool,
    pub(crate) has_custom_methods: bool,
    pub(crate) has_generated_handler: bool,
    pub(crate) standard_methods: Vec<String>,
    pub(crate) query_standard_methods: Vec<String>,
    pub(crate) mutation_standard_methods: Vec<String>,
    pub(crate) custom_methods: Vec<NormalizedCustomMethod>,
    pub(crate) query_custom_methods: Vec<String>,
    pub(crate) mutation_custom_methods: Vec<String>,
    pub(crate) primary_key: Option<String>,
    pub(crate) audit_table: Option<String>,
    pub(crate) native_properties: Vec<NormalizedProperty>,
    pub(crate) relationship_properties: Vec<NormalizedProperty>,
}

#[derive(Debug, Serialize)]
pub(crate) struct NormalizedDataAccessExecution {
    pub(crate) prepared_statements: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct NormalizedCustomMethod {
    pub(crate) name: String,
    pub(crate) kind: String,
    pub(crate) return_type: String,
    pub(crate) mcp_enabled: bool,
    pub(crate) provider_routine: Option<NormalizedProviderRoutine>,
}

#[derive(Debug, Serialize)]
pub(crate) struct NormalizedProviderRoutine {
    pub(crate) kind: String,
    pub(crate) returns: String,
    pub(crate) data_source: Option<String>,
    pub(crate) postgres: Option<NormalizedProviderRoutineName>,
    pub(crate) mssql: Option<NormalizedProviderRoutineName>,
    pub(crate) snowflake: Option<NormalizedProviderRoutineName>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct NormalizedProviderRoutineName {
    pub(crate) schema: Option<String>,
    pub(crate) name: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct NormalizedProperty {
    pub(crate) name: String,
    pub(crate) caption: String,
    pub(crate) data_type: String,
    pub(crate) is_key: bool,
    pub(crate) is_caption: bool,
    pub(crate) is_required: bool,
    pub(crate) is_read_only: bool,
    pub(crate) is_concurrency_control: bool,
    pub(crate) enum_type_name: Option<String>,
    pub(crate) relation: Option<NormalizedRelation>,
}

#[derive(Debug, Serialize)]
pub(crate) struct NormalizedRelation {
    pub(crate) kind: &'static str,
    pub(crate) schema_name: Option<String>,
    pub(crate) type_name: Option<String>,
    pub(crate) prop_name: Option<String>,
    pub(crate) resolved_schema_name: Option<String>,
    pub(crate) resolved_type_name: Option<String>,
    pub(crate) junction_schema: Option<String>,
    pub(crate) junction_table: Option<String>,
    pub(crate) local_key: Option<String>,
    pub(crate) foreign_key: Option<String>,
}

pub fn build(workspace: &AppWorkspace, entity_types: &[EntityType]) -> Result<GeneratorIr> {
    let schemas = load_schemas(workspace)?;
    let data_sources = context::read_data_sources(&files::get_data_sources_file(workspace))?;
    let data_source_by_name = data_sources
        .into_iter()
        .map(|data_source| (data_source.name.clone(), data_source))
        .collect::<HashMap<_, _>>();

    Ok(GeneratorIr {
        version: 1,
        generated_at: chrono::offset::Utc::now().to_rfc3339(),
        schemas: schemas
            .into_iter()
            .map(|schema| normalize_schema(workspace, schema, &data_source_by_name, entity_types))
            .collect::<Result<Vec<_>>>()?,
    })
}

pub fn emit(workspace: &AppWorkspace, ir: &GeneratorIr) -> Result<()> {
    let target_dir = &workspace.report_root;
    let json = serde_json::to_string_pretty(ir)? + "\n";
    for file_name in ["generator_ir.json", "normalized_config.json"] {
        let target_file = target_dir.join(file_name);
        artifacts::safe_write(&target_file, &json)
            .with_context(|| format!("could not write {}", target_file.display()))?;
        console::write(&target_file);
    }
    Ok(())
}

fn load_schemas(workspace: &AppWorkspace) -> Result<Vec<Schema>> {
    let mut schemas = vec![];
    for schema_dir_name in
        files::get_schema_dir_names(workspace).map_err(|err| anyhow::anyhow!("{err}"))?
    {
        let schema_file = files::get_schema_file(workspace, &schema_dir_name);
        schemas.push(
            context::read_schema(&schema_file)
                .with_context(|| format!("read schema config {}", schema_file.display()))?,
        );
    }
    schemas.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(schemas)
}

fn normalize_schema(
    workspace: &AppWorkspace,
    schema: Schema,
    data_sources: &HashMap<String, DataSource>,
    entity_types: &[EntityType],
) -> Result<NormalizedSchema> {
    let data_source = data_sources
        .get(&schema.data_source_name)
        .with_context(|| {
            format!(
                "schema `{}` references missing data source `{}`",
                schema.name, schema.data_source_name
            )
        })?;
    let mut entities = entity_types
        .iter()
        .filter(|entity_type| entity_type.schema_name == schema.name)
        .map(normalize_entity)
        .collect::<Vec<_>>();
    entities.sort_by(|left, right| left.name.cmp(&right.name));

    Ok(NormalizedSchema {
        name: schema.name.clone(),
        data_source_name: schema.data_source_name,
        data_source_type: format!("{:?}", data_source.data_source_type),
        is_system_schema: schema.name == "system",
        relationships: relationship_config::read_schema_relationships(workspace, &schema.name)?
            .into_iter()
            .map(normalize_relationship)
            .collect(),
        entities,
    })
}

fn normalize_relationship(relationship: RelationshipConfig) -> NormalizedRelationship {
    NormalizedRelationship {
        name: relationship.name,
        kind: format!("{:?}", relationship.kind),
        left: relationship.left.map(normalize_relationship_endpoint),
        right: relationship.right.map(normalize_relationship_endpoint),
        one: relationship.one.map(normalize_relationship_endpoint),
        many: relationship.many.map(normalize_relationship_endpoint),
        storage: relationship
            .storage
            .map(|storage| NormalizedRelationshipStorage {
                storage_type: format!("{:?}", storage.storage_type),
                owner_schema: storage.owner_schema,
                owner: storage.owner,
                field: storage.field,
            }),
        junction: relationship
            .junction
            .map(|junction| NormalizedRelationshipJunction {
                schema_name: junction.schema,
                entity_name: junction.entity,
                left_key: junction.left_key,
                right_key: junction.right_key,
            }),
    }
}

fn normalize_relationship_endpoint(
    endpoint: relationship_config::RelationshipEndpoint,
) -> NormalizedRelationshipEndpoint {
    NormalizedRelationshipEndpoint {
        schema_name: endpoint.schema,
        entity_name: endpoint.entity,
        field_name: endpoint.field,
        caption: endpoint.caption,
    }
}

fn normalize_entity(entity_type: &EntityType) -> NormalizedEntity {
    let facets = entity_type.facets.clone().unwrap_or_default();
    let standard_methods = entity_type.standard_methods.clone().unwrap_or_default();
    let custom_methods = entity_type.custom_methods.clone().unwrap_or_default();
    let standard_method_names = standard_methods
        .iter()
        .map(standard_method_name)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let query_standard_methods = standard_methods
        .iter()
        .filter(|method| standard_method_kind(**method) == HandlerKind::Query)
        .map(standard_method_name)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mutation_standard_methods = standard_methods
        .iter()
        .filter(|method| standard_method_kind(**method) == HandlerKind::Mutation)
        .map(standard_method_name)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let query_custom_methods = custom_methods
        .iter()
        .filter(|method| custom_method_kind(method.kind) == HandlerKind::Query)
        .map(|method| method.name.clone())
        .collect::<Vec<_>>();
    let mutation_custom_methods = custom_methods
        .iter()
        .filter(|method| custom_method_kind(method.kind) == HandlerKind::Mutation)
        .map(|method| method.name.clone())
        .collect::<Vec<_>>();
    let normalized_custom_methods = custom_methods
        .iter()
        .map(normalize_custom_method)
        .collect::<Vec<_>>();
    let (native_properties, relationship_properties): (Vec<_>, Vec<_>) = entity_type
        .props
        .iter()
        .map(normalize_property)
        .partition(|prop| prop.relation.is_none());

    NormalizedEntity {
        schema_name: entity_type.schema_name.clone(),
        name: entity_type.pascal_1.clone(),
        table_name: entity_type.snake_n.clone(),
        caption_singular: entity_type.caption_1.clone(),
        caption_plural: entity_type.caption_n.clone(),
        snake_singular: entity_type.snake_1.clone(),
        is_table: entity_type.is_table,
        is_union: entity_type.is_union,
        base_type: entity_type.base_type.clone(),
        execution: normalize_data_access_execution(entity_type.execution.as_ref()),
        has_standard_methods: !standard_methods.is_empty(),
        has_custom_methods: !custom_methods.is_empty(),
        has_generated_handler: !standard_methods.is_empty() || !custom_methods.is_empty(),
        primary_key: entity_type
            .props
            .iter()
            .find(|prop| prop.is_key)
            .map(|prop| prop.name.clone()),
        audit_table: facets
            .iter()
            .any(|facet| facet == "audited")
            .then(|| format!("{}_audit", entity_type.snake_n)),
        facets,
        standard_methods: standard_method_names,
        query_standard_methods,
        mutation_standard_methods,
        custom_methods: normalized_custom_methods,
        query_custom_methods,
        mutation_custom_methods,
        native_properties,
        relationship_properties,
    }
}

fn normalize_data_access_execution(
    execution: Option<&DataAccessExecution>,
) -> NormalizedDataAccessExecution {
    NormalizedDataAccessExecution {
        prepared_statements: execution
            .and_then(|execution| execution.prepared_statements)
            .unwrap_or(false),
    }
}

fn normalize_custom_method(method: &CustomMethod) -> NormalizedCustomMethod {
    NormalizedCustomMethod {
        name: method.name.clone(),
        kind: custom_method_kind(method.kind).label().to_string(),
        return_type: method.return_type.clone(),
        mcp_enabled: method.mcp_enabled.unwrap_or(false),
        provider_routine: method
            .provider_routine
            .as_ref()
            .map(normalize_provider_routine),
    }
}

fn normalize_provider_routine(routine: &ProviderRoutineBinding) -> NormalizedProviderRoutine {
    let default = routine
        .name
        .as_ref()
        .map(|name| NormalizedProviderRoutineName {
            schema: routine.schema.clone(),
            name: name.clone(),
        });
    let postgres = routine
        .routines
        .as_ref()
        .and_then(|routines| routines.postgres.as_ref())
        .map(normalize_provider_routine_name)
        .or_else(|| default.clone());
    let mssql = routine
        .routines
        .as_ref()
        .and_then(|routines| routines.mssql.as_ref())
        .map(normalize_provider_routine_name)
        .or_else(|| default.clone());
    let snowflake = routine
        .routines
        .as_ref()
        .and_then(|routines| routines.snowflake.as_ref())
        .map(normalize_provider_routine_name)
        .or_else(|| default.clone());

    NormalizedProviderRoutine {
        kind: format!("{:?}", routine.kind),
        returns: format!("{:?}", routine.returns),
        data_source: routine.data_source.clone(),
        postgres,
        mssql,
        snowflake,
    }
}

fn normalize_provider_routine_name(routine: &ProviderRoutineName) -> NormalizedProviderRoutineName {
    NormalizedProviderRoutineName {
        schema: routine.schema.clone(),
        name: routine.name.clone(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HandlerKind {
    Query,
    Mutation,
    Command,
}

impl HandlerKind {
    fn label(self) -> &'static str {
        match self {
            HandlerKind::Query => "Query",
            HandlerKind::Mutation => "Mutation",
            HandlerKind::Command => "Command",
        }
    }
}

fn standard_method_kind(method: StandardMethod) -> HandlerKind {
    match method {
        StandardMethod::FindById | StandardMethod::GetAll | StandardMethod::Query => {
            HandlerKind::Query
        }
        StandardMethod::Create | StandardMethod::Update | StandardMethod::Delete => {
            HandlerKind::Mutation
        }
    }
}

fn custom_method_kind(kind: CustomMethodKind) -> HandlerKind {
    match kind {
        CustomMethodKind::Query => HandlerKind::Query,
        CustomMethodKind::Mutation => HandlerKind::Mutation,
        CustomMethodKind::Command => HandlerKind::Command,
    }
}

fn normalize_property(prop: &PropertyType) -> NormalizedProperty {
    NormalizedProperty {
        name: prop.name.clone(),
        caption: prop.caption.clone(),
        data_type: format!("{:?}", prop.data_type),
        is_key: prop.is_key,
        is_caption: prop.is_caption,
        is_required: prop.is_required,
        is_read_only: prop.is_read_only,
        is_concurrency_control: prop.is_concurrency_control,
        enum_type_name: prop.enum_type_name.clone(),
        relation: normalize_relation(prop),
    }
}

fn normalize_relation(prop: &PropertyType) -> Option<NormalizedRelation> {
    if let Some(foreign_key) = &prop.foreign_key {
        return Some(NormalizedRelation {
            kind: "foreign_key",
            schema_name: Some(foreign_key.schema_name.clone()),
            type_name: Some(foreign_key.type_name.clone()),
            prop_name: None,
            resolved_schema_name: None,
            resolved_type_name: None,
            junction_schema: None,
            junction_table: None,
            local_key: None,
            foreign_key: None,
        });
    }

    if let Some(nav) = &prop.nav_by_fk_property {
        return Some(NormalizedRelation {
            kind: match prop.data_type {
                DataType::NavToOne => "nav_to_one",
                DataType::NavToMany => "nav_to_many",
                _ => "navigation",
            },
            schema_name: Some(nav.schema_name.clone()),
            type_name: Some(nav.type_name.clone()),
            prop_name: Some(nav.prop_name.clone()),
            resolved_schema_name: nav
                .resolved
                .as_ref()
                .map(|resolved| resolved.schema_name.clone()),
            resolved_type_name: nav
                .resolved
                .as_ref()
                .map(|resolved| resolved.type_name.clone()),
            junction_schema: None,
            junction_table: None,
            local_key: None,
            foreign_key: None,
        });
    }

    if let Some(many_to_many) = &prop.many_to_many_property {
        return Some(NormalizedRelation {
            kind: "many_to_many",
            schema_name: Some(many_to_many.target_schema.clone()),
            type_name: Some(many_to_many.target_type.clone()),
            prop_name: None,
            resolved_schema_name: Some(many_to_many.target_schema.clone()),
            resolved_type_name: Some(many_to_many.target_type.clone()),
            junction_schema: many_to_many.junction_schema.clone(),
            junction_table: Some(many_to_many.junction_table.clone()),
            local_key: Some(many_to_many.local_key.clone()),
            foreign_key: Some(many_to_many.foreign_key.clone()),
        });
    }

    if let Some(nested) = &prop.nested_entity_type {
        return Some(NormalizedRelation {
            kind: "nested_entity",
            schema_name: Some(nested.schema_name.clone()),
            type_name: Some(nested.type_name.clone()),
            prop_name: None,
            resolved_schema_name: Some(nested.schema_name.clone()),
            resolved_type_name: Some(nested.type_name.clone()),
            junction_schema: None,
            junction_table: None,
            local_key: None,
            foreign_key: None,
        });
    }

    None
}

fn standard_method_name(method: &StandardMethod) -> &'static str {
    match method {
        StandardMethod::FindById => "FindById",
        StandardMethod::GetAll => "GetAll",
        StandardMethod::Query => "Query",
        StandardMethod::Create => "Create",
        StandardMethod::Update => "Update",
        StandardMethod::Delete => "Delete",
    }
}
