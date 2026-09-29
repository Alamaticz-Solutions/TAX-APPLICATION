use std::path::PathBuf;

use anyhow::{Context as AnyhowContext, Result};
use serde::Serialize;
use serde_json::{Map, Value};
use tera::Context;

use crate::app_manifest;
use crate::app_workspace::AppWorkspace;
use crate::bootstrap_types::data_source::{DataSource, DataSourceType, Schema};
use crate::utils::constants::ctx_name;
use crate::utils::context::read_yaml;
use crate::utils::{
    artifacts::{self, Artifact, OverwriteMode},
    console, context, engine, files,
};

//
// Generate backend code from the system-level configurations
//

pub fn run(workspace: &AppWorkspace) -> Result<()> {
    console::stage("backend");

    let data_sources_src_file = files::get_data_sources_file(workspace);
    let data_sources = context::read_data_sources(&data_sources_src_file)?;

    let schema_dir_names = get_schema_dir_names(workspace)?;

    let mut schemas: Vec<Schema> = vec![];
    for schema_dir_name in &schema_dir_names {
        console::item("schema", schema_dir_name);

        let schema_src_file = files::get_schema_file(workspace, schema_dir_name);
        let schema = context::read_schema(&schema_src_file)?;
        let data_source = data_sources
            .iter()
            .find(|ds| ds.name == schema.data_source_name)
            .with_context(|| {
                format!(
                    "schema `{}` references missing data source `{}`",
                    schema.name, schema.data_source_name
                )
            })?;

        gen_schema_rs(&workspace, &schema, data_source)?;
        gen_route_rs(&workspace, &schema, data_source)?;

        gen_schema_handler_generated_rs(&workspace, &schema)?;
        gen_schema_handler_impl_rs(&workspace, &schema)?;
        gen_schema_handler_mod_rs(&workspace, &schema, data_source)?;

        schemas.push(schema);
    }

    gen_mod_rs(&workspace, "schemas", &data_sources, &schemas)?;
    gen_mod_rs(&workspace, "routes", &data_sources, &schemas)?;
    gen_mod_rs(&workspace, "handlers", &data_sources, &schemas)?;
    gen_operations_rs(&workspace, &schemas)?;
    gen_kafka_ingress_config(&workspace)?;

    console::done("backend");

    Ok(())
}

#[derive(Debug, Serialize)]
struct GeneratedKafkaIngressConfig {
    enabled: bool,
    consumers: Vec<GeneratedKafkaConsumerConfig>,
}

#[derive(Debug, Serialize)]
struct GeneratedKafkaConsumerConfig {
    name: String,
    topic: String,
    consumer_group: String,
    handler: String,
    actor: GeneratedKafkaActorConfig,
    tenant: GeneratedKafkaTenantConfig,
    auth: GeneratedKafkaAuthConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation: Option<serde_yaml::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    idempotency_key_field: Option<String>,
    retry: GeneratedKafkaRetryPolicy,
    dead_letter: GeneratedKafkaDeadLetterPolicy,
    readiness: GeneratedKafkaReadinessPolicy,
}

#[derive(Debug, Serialize)]
struct GeneratedKafkaActorConfig {
    subject: String,
    roles: Vec<String>,
    scopes: Vec<String>,
}

#[derive(Debug, Serialize)]
struct GeneratedKafkaTenantConfig {
    source: &'static str,
    field: String,
}

#[derive(Debug, Serialize)]
struct GeneratedKafkaAuthConfig {
    mechanism: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    secret_ref: Option<String>,
}

#[derive(Debug, Serialize)]
struct GeneratedKafkaRetryPolicy {
    enabled: bool,
    max_attempts: u32,
    initial_backoff_ms: u64,
    max_backoff_ms: u64,
}

#[derive(Debug, Serialize)]
struct GeneratedKafkaDeadLetterPolicy {
    enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    topic: Option<String>,
}

#[derive(Debug, Serialize)]
struct GeneratedKafkaReadinessPolicy {
    max_lag_messages: u64,
    max_idle_ms: u64,
}

fn gen_kafka_ingress_config(workspace: &AppWorkspace) -> Result<()> {
    let (topology_report, _) = app_manifest::build(workspace)?;
    let kafka_ingress = topology_report
        .topology
        .ingress
        .iter()
        .filter(|module| module.kind == "kafka" && module.enabled)
        .collect::<Vec<_>>();

    if kafka_ingress.is_empty() {
        return Ok(());
    }

    console::step("backend/config/generated/ingress/kafka.yaml");

    let target_path = workspace
        .app_root
        .join("backend/config/generated/ingress/kafka.yaml");
    artifacts::emit(&Artifact::generated_text(
        target_path,
        render_kafka_ingress_config(&kafka_ingress)?,
        OverwriteMode::Always,
    ))
}

fn render_kafka_ingress_config(ingress: &[&app_manifest::TopologyIngressModule]) -> Result<String> {
    let config = GeneratedKafkaIngressConfig {
        enabled: true,
        consumers: ingress
            .iter()
            .map(|module| {
                let topic = required_kafka_field(module, "topic", module.topic.as_deref())?;
                let consumer_group = required_kafka_field(
                    module,
                    "consumer_group",
                    module.consumer_group.as_deref(),
                )?;
                let handler = required_kafka_field(module, "handler", module.handler.as_deref())?;
                let schema = module.schema.as_deref().unwrap_or("app");
                let dead_letter_topic = format!("{topic}.dlq");
                Ok(GeneratedKafkaConsumerConfig {
                    name: module.name.clone(),
                    topic,
                    consumer_group,
                    handler,
                    actor: GeneratedKafkaActorConfig {
                        subject: format!("{}-consumer", module.name),
                        roles: vec![format!("{schema}.integration_writer")],
                        scopes: vec![],
                    },
                    tenant: GeneratedKafkaTenantConfig {
                        source: "message_field",
                        field: "tenant_id".to_string(),
                    },
                    auth: GeneratedKafkaAuthConfig {
                        mechanism: "local_no_auth",
                        secret_ref: None,
                    },
                    operation: None,
                    idempotency_key_field: Some("event_id".to_string()),
                    retry: GeneratedKafkaRetryPolicy {
                        enabled: true,
                        max_attempts: 3,
                        initial_backoff_ms: 1_000,
                        max_backoff_ms: 30_000,
                    },
                    dead_letter: GeneratedKafkaDeadLetterPolicy {
                        enabled: false,
                        topic: Some(dead_letter_topic),
                    },
                    readiness: GeneratedKafkaReadinessPolicy {
                        max_lag_messages: 1_000,
                        max_idle_ms: 300_000,
                    },
                })
            })
            .collect::<Result<Vec<_>>>()?,
    };
    let yaml = serde_yaml::to_string(&config)?;
    Ok(format!(
        "# Human-owned Kafka runtime ingress config.\n# Generated once from enabled .appfw/manifest.yaml kafka topology entries.\n# Keep broker credentials in secrets and replace local_no_auth before production.\n# Review retry, dead_letter, readiness, tenant, idempotency, and operation bindings before enabling workers.\n{yaml}"
    ))
}

fn required_kafka_field(
    module: &app_manifest::TopologyIngressModule,
    field: &str,
    value: Option<&str>,
) -> Result<String> {
    value
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .with_context(|| {
            format!(
                "enabled Kafka ingress `{}` is missing `{field}`",
                module.name
            )
        })
}

fn gen_operations_rs(workspace: &AppWorkspace, schemas: &Vec<Schema>) -> Result<()> {
    let context = create_mcp_operations_context(workspace, schemas)?;
    let templates_dir = &files::get_backend_templates_dir(workspace).join("operations");

    console::step("operations/generated.rs");

    let target_path = workspace
        .app_root
        .join("backend/src/operations/generated.rs");

    emit_template(
        workspace,
        templates_dir,
        &target_path,
        "_operations",
        &context,
        OverwriteMode::Always,
    )?;

    console::step("operations/mod.rs");

    let target_path = workspace.app_root.join("backend/src/operations/mod.rs");

    emit_template(
        workspace,
        templates_dir,
        &target_path,
        "operations",
        &context,
        OverwriteMode::Always,
    )?;

    Ok(())
}

//
// Generate backend/src/schemas/{schema-name}.rs
//

fn gen_schema_rs(
    workspace: &AppWorkspace,
    schema: &Schema,
    data_source: &DataSource,
) -> Result<()> {
    console::step(format!("backend schema: {}", schema.name));

    let context = create_full_context(workspace, schema, data_source)?;
    let templates_dir = &files::get_backend_templates_dir(workspace).join("schemas/schema");
    let target_path = &files::get_backend_src_rs_path(workspace, "schemas", &schema.name);

    emit_template(
        workspace,
        templates_dir,
        target_path,
        "schema",
        &context,
        OverwriteMode::Always,
    )?;

    Ok(())
}

//
// Generate backend/src/routes/{schema-name}.rs
//

fn gen_route_rs(workspace: &AppWorkspace, schema: &Schema, data_source: &DataSource) -> Result<()> {
    console::step(format!("route module: {}", schema.name));

    let context = create_full_context(workspace, schema, data_source)?;
    let templates_dir = &files::get_backend_templates_dir(workspace).join("routes/route");
    let target_path = &files::get_backend_src_rs_path(workspace, "routes", &schema.name);

    emit_template(
        workspace,
        templates_dir,
        target_path,
        "route",
        &context,
        OverwriteMode::Always,
    )?;

    Ok(())
}

//
// Generate backend/src/handlers/{schema-name}/mod.rs
//

fn gen_schema_handler_mod_rs(
    workspace: &AppWorkspace,
    schema: &Schema,
    data_source: &DataSource,
) -> Result<()> {
    console::step(format!("handler module: {}", schema.name));

    let context = create_full_context(workspace, schema, data_source)?;
    let templates_dir = &files::get_backend_templates_dir(workspace).join("handlers/schema/mod");
    let target_path =
        files::get_backend_src_handlers_schema_dir_path(workspace, &schema.name).join("mod.rs");
    emit_template(
        workspace,
        templates_dir,
        &target_path,
        "mod",
        &context,
        OverwriteMode::Always,
    )?;

    Ok(())
}

/// Generate backend/src/handlers/{schema-name}/generated.rs
///
/// Generated defaults are imported by product-owned handler files. Product
/// handlers can override any generated operation by defining a local function
/// with the same name and signature.
fn gen_schema_handler_generated_rs(workspace: &AppWorkspace, schema: &Schema) -> Result<()> {
    console::step(format!("handler generated defaults: {}", schema.name));

    let entity_types_file = files::get_entity_types_file(workspace, &schema.name);
    let context = &mut context::create()?;
    context.insert("schemaName", &schema.name);
    context.insert(
        ctx_name::ENTITY_TYPES,
        &read_entity_types_context(&entity_types_file)?,
    );

    let templates_dir =
        &files::get_backend_templates_dir(workspace).join("handlers/schema/generated");
    let target_path = files::get_backend_src_handlers_schema_dir_path(workspace, &schema.name)
        .join("generated.rs");

    emit_template(
        workspace,
        templates_dir,
        &target_path,
        "generated",
        context,
        OverwriteMode::Always,
    )?;

    Ok(())
}

/// Generate backend/src/handlers/{schema-name}/{entity-type}.rs  
///
/// The generated {entity-type}.rs files are create-once product extension
/// files. They import regenerated defaults and may override individual
/// operations with product-owned functions.
fn gen_schema_handler_impl_rs(workspace: &AppWorkspace, schema: &Schema) -> Result<()> {
    console::step(format!("handler impls: {}", schema.name));

    // Ensure backend/src/handlers/{schema-name} dir exists
    let target_dir_path = &files::get_backend_src_handlers_schema_dir_path(workspace, &schema.name);

    let templates_dir = &files::get_backend_templates_dir(workspace).join("handlers/schema/impl");
    let tera = engine::create(workspace, &templates_dir)?;

    let entity_types_file = files::get_entity_types_file(workspace, &schema.name);
    let res = read_entity_types_context(&entity_types_file)?;
    let items = res
        .as_array()
        .with_context(|| format!("{} must be a YAML array", entity_types_file.display()))?;

    for item in items {
        if !item
            .get("has_generated_handler")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            continue;
        }

        let type_name = item
            .get("snake_1")
            .and_then(Value::as_str)
            .with_context(|| {
                format!(
                    "entity type in {} is missing string `snake_1`",
                    entity_types_file.display()
                )
            })?;
        let target_path = target_dir_path.join(format!("{type_name}.rs"));
        artifacts::emit_human_text(target_path, || {
            console::step(format!("handler impl: {}.{type_name}", schema.name));
            let context = &mut context::create()?;
            context.insert("schemaName", &schema.name);
            context.insert("entityType", item);
            let output = tera.render("impl", &context)?;
            Ok(output)
        })?;
    }

    Ok(())
}

//
// Generate backend/src/{dir}/mod.rs
//

fn gen_mod_rs(
    workspace: &AppWorkspace,
    target_dir_name: &str,
    data_sources: &Vec<DataSource>,
    schemas: &Vec<Schema>,
) -> Result<()> {
    console::step(format!("{target_dir_name}/mod.rs"));

    let context = &mut context::create()?;
    let used_data_sources = used_data_sources(data_sources, schemas);
    let uses_postgres = uses_data_source_type(&used_data_sources, DataSourceType::PostgreSQL);
    let uses_mongo = uses_data_source_type(&used_data_sources, DataSourceType::MongoDB);
    let uses_mssql_server = uses_data_source_type(&used_data_sources, DataSourceType::MsSqlServer);
    let uses_fabric_sql_analytics =
        uses_data_source_type(&used_data_sources, DataSourceType::FabricSqlAnalytics);
    let uses_mssql = uses_mssql_server;
    let uses_snowflake = uses_data_source_type(&used_data_sources, DataSourceType::Snowflake);

    context.insert("targetDirName", &target_dir_name);
    context.insert("data_sources", &data_sources);
    context.insert("schemas", schemas);
    context.insert("usedDataSources", &used_data_sources);
    context.insert("usesPostgres", &uses_postgres);
    context.insert("usesMongo", &uses_mongo);
    context.insert("usesMsSql", &uses_mssql);
    context.insert("usesMsSqlServer", &uses_mssql_server);
    context.insert("usesFabricSqlAnalytics", &uses_fabric_sql_analytics);
    context.insert("usesSnowflake", &uses_snowflake);

    let templates_dir = &files::get_backend_templates_dir(workspace)
        .join(target_dir_name)
        .join("mod");
    let target_path = &files::get_backend_src_rs_path(workspace, target_dir_name, "mod");

    emit_template(
        workspace,
        templates_dir,
        target_path,
        "mod",
        context,
        OverwriteMode::Always,
    )?;

    Ok(())
}

fn create_full_context(
    workspace: &AppWorkspace,
    schema: &Schema,
    data_source: &DataSource,
) -> Result<Context> {
    console::verbose(format!("create full context: {}", schema.name));

    let entity_types_file = files::get_entity_types_file(workspace, &schema.name);
    let enum_types_file = files::get_gql_enum_types_file(workspace, &schema.name);

    let context = &mut create_basic_context(&schema.name)?;
    context.insert("dataSourceType", &data_source.data_source_type);
    context.insert("schema", schema);

    context::load_array_yaml(context, &enum_types_file, ctx_name::GQL_ENUM_TYPES)?;
    context.insert(
        ctx_name::ENTITY_TYPES,
        &read_entity_types_context(&entity_types_file)?,
    );

    Ok(context.to_owned())
}

fn create_mcp_operations_context(
    workspace: &AppWorkspace,
    schemas: &Vec<Schema>,
) -> Result<Context> {
    let context = &mut context::create()?;
    let mut schema_values = Vec::with_capacity(schemas.len());
    for schema in schemas {
        let mut schema_value = serde_json::to_value(schema)?;
        let entity_types_file = files::get_entity_types_file(workspace, &schema.name);
        let entity_types = read_entity_types_context(&entity_types_file)?;
        schema_value["entity_types"] = entity_types;
        schema_values.push(schema_value);
    }
    context.insert("schemas", &schema_values);
    Ok(context.to_owned())
}

fn read_entity_types_context(entity_types_file: &PathBuf) -> Result<Value> {
    let default = serde_json::Value::Null;
    let mut res: Value = read_yaml(entity_types_file, default)?;
    let items = res
        .as_array_mut()
        .with_context(|| format!("{} must be a YAML array", entity_types_file.display()))?;
    for item in items {
        annotate_entity_context(item);
    }
    Ok(res)
}

fn annotate_entity_context(item: &mut Value) {
    let Some(obj) = item.as_object_mut() else {
        return;
    };

    let standard_methods = string_array(obj.get("standard_methods"));
    let mut custom_methods = obj
        .get("custom_methods")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    annotate_custom_method_args(&mut custom_methods);
    let has_standard_methods = !standard_methods.is_empty();
    let has_custom_methods = !custom_methods.is_empty();

    obj.insert(
        "has_standard_methods".to_string(),
        Value::Bool(has_standard_methods),
    );
    obj.insert(
        "has_custom_methods".to_string(),
        Value::Bool(has_custom_methods),
    );
    obj.insert(
        "has_generated_handler".to_string(),
        Value::Bool(has_standard_methods || has_custom_methods),
    );
    obj.insert(
        "query_standard_methods".to_string(),
        Value::Array(
            standard_methods
                .iter()
                .filter(|method| is_query_standard_method(method))
                .map(|method| Value::String(method.clone()))
                .collect(),
        ),
    );
    obj.insert(
        "mutation_standard_methods".to_string(),
        Value::Array(
            standard_methods
                .iter()
                .filter(|method| is_mutation_standard_method(method))
                .map(|method| Value::String(method.clone()))
                .collect(),
        ),
    );
    obj.insert(
        "query_custom_methods".to_string(),
        Value::Array(filter_custom_methods(&custom_methods, "Query")),
    );
    obj.insert(
        "mutation_custom_methods".to_string(),
        Value::Array(filter_custom_methods(&custom_methods, "Mutation")),
    );
    obj.insert(
        "mcp_query_custom_methods".to_string(),
        Value::Array(filter_mcp_custom_methods(&custom_methods, "Query")),
    );
    obj.insert(
        "mcp_mutation_custom_methods".to_string(),
        Value::Array(filter_mcp_custom_methods(&custom_methods, "Mutation")),
    );
}

fn string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn filter_custom_methods(methods: &[Value], kind: &str) -> Vec<Value> {
    methods
        .iter()
        .filter(|method| method_kind(method).as_deref() == Some(kind))
        .cloned()
        .collect()
}

fn filter_mcp_custom_methods(methods: &[Value], kind: &str) -> Vec<Value> {
    methods
        .iter()
        .filter(|method| method_kind(method).as_deref() == Some(kind))
        .filter(|method| method_mcp_enabled(method))
        .cloned()
        .collect()
}

fn annotate_custom_method_args(methods: &mut [Value]) {
    for method in methods {
        let Some(args) = method.get_mut("args").and_then(Value::as_array_mut) else {
            continue;
        };
        for arg in args {
            let Some(obj) = arg.as_object_mut() else {
                continue;
            };
            let arg_type = obj
                .get("arg_type")
                .and_then(Value::as_str)
                .unwrap_or_default();
            obj.insert(
                "is_optional".to_string(),
                Value::Bool(arg_type.starts_with("Option<")),
            );
        }
    }
}

fn method_kind(method: &Value) -> Option<String> {
    let obj: &Map<String, Value> = method.as_object()?;
    obj.get("kind").and_then(Value::as_str).map(str::to_string)
}

fn method_mcp_enabled(method: &Value) -> bool {
    method
        .as_object()
        .and_then(|obj| obj.get("mcp_enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn is_query_standard_method(method: &str) -> bool {
    matches!(method, "FindById" | "GetAll" | "Query")
}

fn is_mutation_standard_method(method: &str) -> bool {
    matches!(method, "Create" | "Update" | "Delete")
}

fn create_basic_context(schema_dir_name: &str) -> Result<Context> {
    console::verbose(format!("create context: {schema_dir_name}"));

    let context = &mut context::create()?;
    context.insert("schemaName", &schema_dir_name);

    Ok(context.to_owned())
}

fn emit_template(
    workspace: &AppWorkspace,
    templates_dir: &PathBuf,
    target_path: &PathBuf,
    template_name: &str,
    context: &Context,
    overwrite: OverwriteMode,
) -> Result<()> {
    artifacts::emit_text(target_path.clone(), overwrite, || {
        let tera = engine::create(workspace, templates_dir)?;
        let output = tera.render(template_name, context)?;
        Ok(output)
    })
}

fn used_data_sources(data_sources: &[DataSource], schemas: &[Schema]) -> Vec<DataSource> {
    let schema_data_source_names = schemas
        .iter()
        .map(|schema| schema.data_source_name.as_str())
        .collect::<std::collections::BTreeSet<_>>();

    // Runtime provider factories should cover schema hosts plus declared
    // runtime database providers. Local/provider certification can switch a
    // generated schema across declared provider data sources, so a provider
    // that is present in the product topology must be registered behind its
    // Cargo feature gate even when it is not the schema's default host.
    //
    // Graph and SaaS providers are intentionally excluded here; they are
    // reached through named operations, sync workers, or product services
    // instead of the generated CRUD data-access registry.
    let mut used = data_sources
        .iter()
        .filter(|data_source| {
            schema_data_source_names.contains(data_source.name.as_str())
                || is_runtime_database_provider(data_source.data_source_type)
        })
        .cloned()
        .collect::<Vec<_>>();
    used.sort_by(|left, right| {
        let left_schema_host = schema_data_source_names.contains(left.name.as_str());
        let right_schema_host = schema_data_source_names.contains(right.name.as_str());
        right_schema_host
            .cmp(&left_schema_host)
            .then_with(|| left.name.cmp(&right.name))
    });
    used
}

fn is_runtime_database_provider(data_source_type: DataSourceType) -> bool {
    matches!(
        data_source_type,
        DataSourceType::PostgreSQL
            | DataSourceType::MongoDB
            | DataSourceType::MsSqlServer
            | DataSourceType::FabricSqlAnalytics
            | DataSourceType::Snowflake
    )
}

fn uses_data_source_type(data_sources: &[DataSource], data_source_type: DataSourceType) -> bool {
    data_sources
        .iter()
        .any(|data_source| data_source.data_source_type == data_source_type)
}

//
// Get vec of schema dir names
//

fn get_schema_dir_names(workspace: &AppWorkspace) -> Result<Vec<String>> {
    files::get_schema_dir_names(workspace).map_err(|err| anyhow::anyhow!("{err}"))
}

#[cfg(test)]
mod kafka_ingress_config_tests {
    use super::*;

    fn data_source(name: &str, data_source_type: DataSourceType) -> DataSource {
        DataSource {
            name: name.to_string(),
            data_source_type,
            is_system_schema_host: None,
            description: None,
            environments: vec![],
        }
    }

    fn schema(name: &str, data_source_name: &str) -> Schema {
        Schema {
            id: format!("{name}-schema"),
            name: name.to_string(),
            description: format!("{name} schema"),
            data_source_name: data_source_name.to_string(),
            meta: None,
        }
    }

    #[test]
    fn used_data_sources_include_declared_runtime_database_providers() {
        let data_sources = vec![
            data_source("pg_primary", DataSourceType::PostgreSQL),
            data_source("mongo_primary", DataSourceType::MongoDB),
            data_source("mssql_primary", DataSourceType::MsSqlServer),
            data_source("snowflake_primary", DataSourceType::Snowflake),
            data_source("neo4j_graph", DataSourceType::Neo4j),
            data_source("servicenow_api", DataSourceType::ServiceNow),
        ];
        let schemas = vec![schema("crm", "pg_primary")];

        let used = used_data_sources(&data_sources, &schemas);
        let names = used
            .iter()
            .map(|data_source| data_source.name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            names,
            vec![
                "pg_primary",
                "mongo_primary",
                "mssql_primary",
                "snowflake_primary"
            ]
        );
        assert!(uses_data_source_type(&used, DataSourceType::MongoDB));
        assert!(uses_data_source_type(&used, DataSourceType::MsSqlServer));
        assert!(uses_data_source_type(&used, DataSourceType::Snowflake));
        assert!(!uses_data_source_type(&used, DataSourceType::Neo4j));
        assert!(!uses_data_source_type(&used, DataSourceType::ServiceNow));
    }

    #[test]
    fn renders_create_once_kafka_config_from_enabled_topology() {
        let module = app_manifest::TopologyIngressModule {
            name: "crm-events".to_string(),
            kind: "kafka".to_string(),
            enabled: true,
            schema: Some("crm".to_string()),
            path: None,
            topic: Some("crm.events".to_string()),
            consumer_group: Some("crm-events-worker".to_string()),
            handler: Some("services.crm_events".to_string()),
            consumers: vec![],
        };

        let yaml = render_kafka_ingress_config(&[&module]).expect("render config");

        assert!(yaml.contains("Human-owned Kafka runtime ingress config"));
        assert!(yaml.contains("enabled: true"));
        assert!(yaml.contains("name: crm-events"));
        assert!(yaml.contains("topic: crm.events"));
        assert!(yaml.contains("consumer_group: crm-events-worker"));
        assert!(yaml.contains("subject: crm-events-consumer"));
        assert!(yaml.contains("crm.integration_writer"));
        assert!(yaml.contains("mechanism: local_no_auth"));
        assert!(yaml.contains("idempotency_key_field: event_id"));
        assert!(yaml.contains("retry:"));
        assert!(yaml.contains("max_attempts: 3"));
        assert!(yaml.contains("dead_letter:"));
        assert!(yaml.contains("topic: crm.events.dlq"));
        assert!(yaml.contains("readiness:"));
    }
}
