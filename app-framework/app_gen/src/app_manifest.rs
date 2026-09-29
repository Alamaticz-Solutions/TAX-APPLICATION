use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    app_workspace::AppWorkspace,
    bootstrap_types::data_source::{DataSource, Schema},
    utils::{artifacts, context, files},
};

pub const TOPOLOGY_REPORT: &str = "app_topology.json";

#[derive(Debug, Clone, Serialize)]
pub struct TopologyIssue {
    pub severity: &'static str,
    pub code: &'static str,
    pub message: String,
    pub file: String,
    pub path: String,
    pub expected: String,
    pub actual: Option<String>,
    pub suggested_fix: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppTopologyReport {
    pub version: u32,
    pub generated_at: String,
    pub manifest_path: String,
    pub manifest_present: bool,
    pub source: &'static str,
    pub app: TopologyApp,
    pub topology: Topology,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopologyApp {
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Topology {
    pub data_sources: Vec<TopologyDataSource>,
    pub schemas: Vec<TopologySchema>,
    pub ingress: Vec<TopologyIngressModule>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopologyDataSource {
    pub name: String,
    pub provider: String,
    pub role: Option<String>,
    pub description: Option<String>,
    pub is_system_schema_host: bool,
    pub schema_names: Vec<String>,
    pub environments: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopologySchema {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_source_name: Option<String>,
    pub provider: Option<String>,
    pub role: Option<String>,
    pub description: String,
    pub is_system_schema: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopologyIngressModule {
    pub name: String,
    pub kind: String,
    pub enabled: bool,
    pub schema: Option<String>,
    pub path: Option<String>,
    pub topic: Option<String>,
    pub consumer_group: Option<String>,
    pub handler: Option<String>,
    pub consumers: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct Manifest {
    version: Option<u32>,
    app: Option<ManifestApp>,
    topology: Option<ManifestTopology>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct ManifestApp {
    name: Option<String>,
    display_name: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct ManifestTopology {
    data_sources: Option<Vec<ManifestDataSource>>,
    schemas: Option<Vec<ManifestSchema>>,
    ingress: Option<Vec<ManifestIngressModule>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct ManifestDataSource {
    name: Option<String>,
    provider: Option<String>,
    role: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct ManifestSchema {
    name: Option<String>,
    data_source_name: Option<String>,
    role: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct ManifestIngressModule {
    name: Option<String>,
    kind: Option<String>,
    enabled: Option<bool>,
    schema: Option<String>,
    path: Option<String>,
    topic: Option<String>,
    consumer_group: Option<String>,
    handler: Option<String>,
    consumers: Option<Vec<String>>,
}

pub fn build(workspace: &AppWorkspace) -> Result<(AppTopologyReport, Vec<TopologyIssue>)> {
    let manifest_path = manifest_path(workspace);
    let data_sources = read_data_sources(workspace)?;
    let schemas = read_schemas(workspace)?;
    let configured_data_sources = data_sources
        .iter()
        .map(|data_source| (data_source.name.clone(), data_source.clone()))
        .collect::<BTreeMap<_, _>>();
    let configured_schemas = schemas
        .iter()
        .map(|schema| (schema.name.clone(), schema.clone()))
        .collect::<BTreeMap<_, _>>();

    let mut issues = vec![];
    let mut manifest = None;
    let manifest_present = manifest_path.exists();

    if manifest_present {
        let contents = fs::read_to_string(&manifest_path)
            .with_context(|| format!("could not read {}", manifest_path.display()))?;
        validate_reserved_manifest_fields(&manifest_path, &contents, &mut issues);
        match serde_yaml::from_str::<Manifest>(&contents) {
            Ok(parsed) => manifest = Some(parsed),
            Err(err) => issues.push(issue(
                "error",
                "app_manifest_parse_error",
                format!("App manifest could not be parsed: {err}."),
                &manifest_path,
                "$",
                "valid YAML matching the app manifest topology contract",
                Some(err.to_string()),
                "Fix .appfw/manifest.yaml syntax and keep schema/entity details in .appfw/model.",
            )),
        }
    }

    if let Some(manifest) = &manifest {
        validate_manifest_contract(
            &manifest_path,
            manifest,
            &configured_data_sources,
            &configured_schemas,
            &mut issues,
        );
    }

    let app = topology_app(&workspace.app_root, manifest.as_ref());
    let data_source_roles = manifest_data_source_roles(manifest.as_ref());
    let schema_roles = manifest_schema_roles(manifest.as_ref());
    let ingress = topology_ingress(manifest.as_ref());
    let topology = build_topology(
        &data_sources,
        &schemas,
        &data_source_roles,
        &schema_roles,
        ingress,
    );
    let report = AppTopologyReport {
        version: 1,
        generated_at: chrono::offset::Utc::now().to_rfc3339(),
        manifest_path: manifest_path.display().to_string(),
        manifest_present,
        source: if manifest_present {
            "app_manifest"
        } else {
            "derived_from_config"
        },
        app,
        topology,
    };

    Ok((report, issues))
}

pub fn write_report(workspace: &AppWorkspace, report: &AppTopologyReport) -> Result<()> {
    let path = report_path(workspace);
    let json = serde_json::to_string_pretty(report)?;
    artifacts::safe_write(&path, format!("{json}\n"))
        .with_context(|| format!("could not write {}", path.display()))?;
    Ok(())
}

pub fn report_path(workspace: &AppWorkspace) -> PathBuf {
    workspace.target_file(TOPOLOGY_REPORT)
}

pub fn manifest_path(workspace: &AppWorkspace) -> PathBuf {
    workspace.app_root.join(".appfw/manifest.yaml")
}

fn read_data_sources(workspace: &AppWorkspace) -> Result<Vec<DataSource>> {
    let data_sources_file = files::get_data_sources_file(workspace);
    context::read_data_sources(&data_sources_file)
}

fn read_schemas(workspace: &AppWorkspace) -> Result<Vec<Schema>> {
    let mut schema_names = files::get_schema_dir_names(workspace)
        .map_err(|err| anyhow::anyhow!("could not list schema directories: {err}"))?;
    schema_names.sort();
    schema_names
        .iter()
        .map(|schema_name| context::read_schema(&files::get_schema_file(workspace, schema_name)))
        .collect()
}

fn validate_reserved_manifest_fields(path: &Path, contents: &str, issues: &mut Vec<TopologyIssue>) {
    let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(contents) else {
        return;
    };
    let Some(obj) = value.as_mapping() else {
        return;
    };
    for reserved in ["primary_schema", "primary_data_source"] {
        if obj.contains_key(&serde_yaml::Value::String(reserved.to_string())) {
            issues.push(issue(
                "error",
                "app_manifest_primary_field",
                format!("App manifest must not use `{reserved}`."),
                path,
                &format!("$.{reserved}"),
                "topology.schemas[] and topology.data_sources[] lists",
                Some(reserved.to_string()),
                "Model every active schema and data source explicitly; do not imply a single primary surface.",
            ));
        }
    }
}

fn validate_manifest_contract(
    path: &Path,
    manifest: &Manifest,
    configured_data_sources: &BTreeMap<String, DataSource>,
    configured_schemas: &BTreeMap<String, Schema>,
    issues: &mut Vec<TopologyIssue>,
) {
    match manifest.version {
        Some(1) => {}
        Some(version) => issues.push(issue(
            "error",
            "app_manifest_version",
            "Unsupported app manifest version.".to_string(),
            path,
            "$.version",
            "version: 1",
            Some(version.to_string()),
            "Set `version: 1` until a new manifest contract is documented.",
        )),
        None => issues.push(issue(
            "error",
            "app_manifest_missing_version",
            "App manifest is missing `version`.".to_string(),
            path,
            "$.version",
            "version: 1",
            None,
            "Add `version: 1` to .appfw/manifest.yaml.",
        )),
    }

    let Some(app) = &manifest.app else {
        issues.push(issue(
            "error",
            "app_manifest_missing_app",
            "App manifest is missing `app`.".to_string(),
            path,
            "$.app",
            "app metadata with at least `name`",
            None,
            "Add `app.name` and optional display/description fields.",
        ));
        return;
    };
    if app.name.as_deref().unwrap_or("").trim().is_empty() {
        issues.push(issue(
            "error",
            "app_manifest_missing_app_name",
            "App manifest is missing `app.name`.".to_string(),
            path,
            "$.app.name",
            "stable app identifier",
            None,
            "Add a durable product app name such as `od-equity-report`.",
        ));
    }

    let Some(topology) = &manifest.topology else {
        issues.push(issue(
            "error",
            "app_manifest_missing_topology",
            "App manifest is missing `topology`.".to_string(),
            path,
            "$.topology",
            "explicit topology.data_sources[] and topology.schemas[] lists",
            None,
            "List every configured schema and data source without redefining entity details.",
        ));
        return;
    };

    validate_manifest_data_sources(path, topology, configured_data_sources, issues);
    validate_manifest_schemas(
        path,
        topology,
        configured_schemas,
        configured_data_sources,
        issues,
    );
    validate_manifest_ingress(path, topology, configured_schemas, issues);
}

fn validate_manifest_data_sources(
    path: &Path,
    topology: &ManifestTopology,
    configured_data_sources: &BTreeMap<String, DataSource>,
    issues: &mut Vec<TopologyIssue>,
) {
    let Some(data_sources) = &topology.data_sources else {
        issues.push(issue(
            "error",
            "app_manifest_missing_data_sources",
            "App manifest topology is missing `data_sources`.".to_string(),
            path,
            "$.topology.data_sources",
            "list of configured data source names",
            None,
            "Add one entry per .appfw/model/data_sources/_res.yaml data source.",
        ));
        return;
    };

    let mut seen = BTreeSet::new();
    for (idx, data_source) in data_sources.iter().enumerate() {
        let entry_path = format!("$.topology.data_sources[{idx}]");
        let Some(name) = non_empty(data_source.name.as_deref()) else {
            issues.push(issue(
                "error",
                "app_manifest_data_source_missing_name",
                "Data source topology entry is missing `name`.".to_string(),
                path,
                &format!("{entry_path}.name"),
                "configured data source name",
                None,
                "Set `name` to a value from .appfw/model/data_sources/_res.yaml.",
            ));
            continue;
        };
        if !seen.insert(name.to_string()) {
            issues.push(issue(
                "error",
                "app_manifest_duplicate_data_source",
                format!("Duplicate data source topology entry `{name}`."),
                path,
                &format!("{entry_path}.name"),
                "unique data source names",
                Some(name.to_string()),
                "Remove the duplicate topology entry.",
            ));
        }
        let Some(configured) = configured_data_sources.get(name) else {
            issues.push(issue(
                "error",
                "app_manifest_unknown_data_source",
                format!("App manifest references unknown data source `{name}`."),
                path,
                &format!("{entry_path}.name"),
                "name from .appfw/model/data_sources/_res.yaml",
                Some(name.to_string()),
                "Add the data source to _config or remove it from the manifest topology.",
            ));
            continue;
        };
        if let Some(provider) = non_empty(data_source.provider.as_deref()) {
            let configured_provider = format!("{:?}", configured.data_source_type);
            if provider != configured_provider {
                issues.push(issue(
                    "error",
                    "app_manifest_provider_mismatch",
                    format!(
                        "Data source `{name}` declares provider `{provider}` but _config uses `{configured_provider}`."
                    ),
                    path,
                    &format!("{entry_path}.provider"),
                    "provider matching .appfw/model/data_sources/_res.yaml",
                    Some(provider.to_string()),
                    "Update the manifest provider or change the data source config.",
                ));
            }
        }
    }

    for name in configured_data_sources.keys() {
        if !seen.contains(name) {
            issues.push(issue(
                "error",
                "app_manifest_missing_configured_data_source",
                format!("Configured data source `{name}` is missing from the app manifest."),
                path,
                "$.topology.data_sources",
                "one manifest entry per configured data source",
                Some(name.clone()),
                "Add the data source to topology.data_sources; the manifest documents topology but does not replace _config.",
            ));
        }
    }
}

fn validate_manifest_schemas(
    path: &Path,
    topology: &ManifestTopology,
    configured_schemas: &BTreeMap<String, Schema>,
    configured_data_sources: &BTreeMap<String, DataSource>,
    issues: &mut Vec<TopologyIssue>,
) {
    let Some(schemas) = &topology.schemas else {
        issues.push(issue(
            "error",
            "app_manifest_missing_schemas",
            "App manifest topology is missing `schemas`.".to_string(),
            path,
            "$.topology.schemas",
            "list of configured schema names",
            None,
            "Add one entry per .appfw/model/schemas/<schema>/_res.yaml schema.",
        ));
        return;
    };

    let manifest_data_sources = topology
        .data_sources
        .as_ref()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| non_empty(item.name.as_deref()).map(str::to_string))
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();

    let mut seen = BTreeSet::new();
    for (idx, schema) in schemas.iter().enumerate() {
        let entry_path = format!("$.topology.schemas[{idx}]");
        let Some(name) = non_empty(schema.name.as_deref()) else {
            issues.push(issue(
                "error",
                "app_manifest_schema_missing_name",
                "Schema topology entry is missing `name`.".to_string(),
                path,
                &format!("{entry_path}.name"),
                "configured schema name",
                None,
                "Set `name` to a value from .appfw/model/schemas.",
            ));
            continue;
        };
        if !seen.insert(name.to_string()) {
            issues.push(issue(
                "error",
                "app_manifest_duplicate_schema",
                format!("Duplicate schema topology entry `{name}`."),
                path,
                &format!("{entry_path}.name"),
                "unique schema names",
                Some(name.to_string()),
                "Remove the duplicate topology entry.",
            ));
        }
        let Some(configured) = configured_schemas.get(name) else {
            issues.push(issue(
                "error",
                "app_manifest_unknown_schema",
                format!("App manifest references unknown schema `{name}`."),
                path,
                &format!("{entry_path}.name"),
                "schema from .appfw/model/schemas",
                Some(name.to_string()),
                "Add the schema config or remove it from the manifest topology.",
            ));
            continue;
        };
        let code_only = is_code_only_system_schema(configured);
        if let Some(data_source_name) = non_empty(schema.data_source_name.as_deref()) {
            if code_only {
                issues.push(issue(
                    "error",
                    "app_manifest_code_only_schema_has_data_source",
                    format!("Code-only schema `{name}` must not declare a manifest data source."),
                    path,
                    &format!("{entry_path}.data_source_name"),
                    "omit data_source_name for code-only schemas",
                    Some(data_source_name.to_string()),
                    "Remove data_source_name from the manifest entry; the schema exists only as generated code/config metadata.",
                ));
                continue;
            }
            if data_source_name != configured.data_source_name {
                issues.push(issue(
                    "error",
                    "app_manifest_schema_data_source_mismatch",
                    format!(
                        "Schema `{name}` declares data source `{data_source_name}` but _config uses `{}`.",
                        configured.data_source_name
                    ),
                    path,
                    &format!("{entry_path}.data_source_name"),
                    "data_source_name matching schema _config",
                    Some(data_source_name.to_string()),
                    "Update the manifest or the schema config; the manifest must not override schema assignment.",
                ));
            }
        }
        if code_only {
            continue;
        }
        if !configured_data_sources.contains_key(&configured.data_source_name) {
            continue;
        }
        if !manifest_data_sources.contains(&configured.data_source_name) {
            issues.push(issue(
                "error",
                "app_manifest_schema_data_source_not_listed",
                format!(
                    "Schema `{name}` uses data source `{}` but that data source is not listed.",
                    configured.data_source_name
                ),
                path,
                &format!("{entry_path}.data_source_name"),
                "schema data source listed in topology.data_sources",
                Some(configured.data_source_name.clone()),
                "Add the data source to topology.data_sources.",
            ));
        }
    }

    for name in configured_schemas.keys() {
        if !seen.contains(name) {
            issues.push(issue(
                "error",
                "app_manifest_missing_configured_schema",
                format!("Configured schema `{name}` is missing from the app manifest."),
                path,
                "$.topology.schemas",
                "one manifest entry per configured schema",
                Some(name.clone()),
                "Add the schema to topology.schemas; keep entities and relationships in .appfw/model.",
            ));
        }
    }
}

fn validate_manifest_ingress(
    path: &Path,
    topology: &ManifestTopology,
    configured_schemas: &BTreeMap<String, Schema>,
    issues: &mut Vec<TopologyIssue>,
) {
    let Some(ingress) = &topology.ingress else {
        return;
    };

    let mut seen = BTreeSet::new();
    for (idx, module) in ingress.iter().enumerate() {
        let entry_path = format!("$.topology.ingress[{idx}]");
        let Some(name) = non_empty(module.name.as_deref()) else {
            issues.push(issue(
                "error",
                "app_manifest_ingress_missing_name",
                "Ingress topology entry is missing `name`.".to_string(),
                path,
                &format!("{entry_path}.name"),
                "unique ingress module name",
                None,
                "Add a stable ingress module name such as `crm-http` or `crm-events`.",
            ));
            continue;
        };
        if !seen.insert(name.to_string()) {
            issues.push(issue(
                "error",
                "app_manifest_duplicate_ingress",
                format!("Duplicate ingress topology entry `{name}`."),
                path,
                &format!("{entry_path}.name"),
                "unique ingress module names",
                Some(name.to_string()),
                "Remove the duplicate topology entry.",
            ));
        }

        let kind = non_empty(module.kind.as_deref());
        match kind {
            Some("http" | "mcp" | "kafka") => {}
            Some(actual) => issues.push(issue(
                "error",
                "app_manifest_ingress_kind",
                format!("Ingress `{name}` uses unsupported kind `{actual}`."),
                path,
                &format!("{entry_path}.kind"),
                "one of http, mcp, kafka",
                Some(actual.to_string()),
                "Use the runtime ingress vocabulary: http, mcp, or kafka.",
            )),
            None => issues.push(issue(
                "error",
                "app_manifest_ingress_missing_kind",
                format!("Ingress `{name}` is missing `kind`."),
                path,
                &format!("{entry_path}.kind"),
                "one of http, mcp, kafka",
                None,
                "Set ingress kind to http, mcp, or kafka.",
            )),
        }

        if let Some(schema) = non_empty(module.schema.as_deref()) {
            if !configured_schemas.contains_key(schema) {
                issues.push(issue(
                    "error",
                    "app_manifest_ingress_unknown_schema",
                    format!("Ingress `{name}` references unknown schema `{schema}`."),
                    path,
                    &format!("{entry_path}.schema"),
                    "schema from .appfw/model/schemas",
                    Some(schema.to_string()),
                    "Add the schema config or point ingress.schema at a configured schema.",
                ));
            }
        }

        if matches!(kind, Some("http" | "mcp")) {
            if let Some(route_path) = non_empty(module.path.as_deref()) {
                if !route_path.starts_with('/') {
                    issues.push(issue(
                        "error",
                        "app_manifest_ingress_path",
                        format!("Ingress `{name}` path must start with `/`."),
                        path,
                        &format!("{entry_path}.path"),
                        "absolute HTTP path",
                        Some(route_path.to_string()),
                        "Use an absolute route path such as `/crm` or `/mcp`.",
                    ));
                }
            }
        }

        if matches!(kind, Some("kafka")) {
            for (field, value) in [
                ("topic", module.topic.as_deref()),
                ("consumer_group", module.consumer_group.as_deref()),
                ("handler", module.handler.as_deref()),
            ] {
                if non_empty(value).is_none() {
                    issues.push(issue(
                        "error",
                        "app_manifest_kafka_ingress_missing_field",
                        format!("Kafka ingress `{name}` is missing `{field}`."),
                        path,
                        &format!("{entry_path}.{field}"),
                        "Kafka topic, consumer group, and handler binding",
                        None,
                        "Declare Kafka topic, consumer_group, and handler in topology.ingress; keep broker credentials in runtime secrets.",
                    ));
                }
            }
        }
    }
}

fn topology_app(repo_root: &Path, manifest: Option<&Manifest>) -> TopologyApp {
    let default_name = repo_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("app")
        .to_string();
    let app = manifest.and_then(|manifest| manifest.app.as_ref());
    TopologyApp {
        name: app
            .and_then(|app| non_empty(app.name.as_deref()).map(str::to_string))
            .unwrap_or(default_name),
        display_name: app
            .and_then(|app| non_empty(app.display_name.as_deref()).map(str::to_string)),
        description: app.and_then(|app| non_empty(app.description.as_deref()).map(str::to_string)),
    }
}

fn manifest_data_source_roles(
    manifest: Option<&Manifest>,
) -> BTreeMap<String, (Option<String>, Option<String>)> {
    manifest
        .and_then(|manifest| manifest.topology.as_ref())
        .and_then(|topology| topology.data_sources.as_ref())
        .map(|data_sources| {
            data_sources
                .iter()
                .filter_map(|data_source| {
                    let name = non_empty(data_source.name.as_deref())?.to_string();
                    Some((
                        name,
                        (
                            non_empty(data_source.role.as_deref()).map(str::to_string),
                            non_empty(data_source.description.as_deref()).map(str::to_string),
                        ),
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn manifest_schema_roles(
    manifest: Option<&Manifest>,
) -> BTreeMap<String, (Option<String>, Option<String>)> {
    manifest
        .and_then(|manifest| manifest.topology.as_ref())
        .and_then(|topology| topology.schemas.as_ref())
        .map(|schemas| {
            schemas
                .iter()
                .filter_map(|schema| {
                    let name = non_empty(schema.name.as_deref())?.to_string();
                    Some((
                        name,
                        (
                            non_empty(schema.role.as_deref()).map(str::to_string),
                            non_empty(schema.description.as_deref()).map(str::to_string),
                        ),
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn topology_ingress(manifest: Option<&Manifest>) -> Vec<TopologyIngressModule> {
    manifest
        .and_then(|manifest| manifest.topology.as_ref())
        .and_then(|topology| topology.ingress.as_ref())
        .map(|ingress| {
            ingress
                .iter()
                .filter_map(|module| {
                    let name = non_empty(module.name.as_deref())?.to_string();
                    Some(TopologyIngressModule {
                        name,
                        kind: non_empty(module.kind.as_deref())
                            .unwrap_or("unknown")
                            .to_string(),
                        enabled: module.enabled.unwrap_or(true),
                        schema: non_empty(module.schema.as_deref()).map(str::to_string),
                        path: non_empty(module.path.as_deref()).map(str::to_string),
                        topic: non_empty(module.topic.as_deref()).map(str::to_string),
                        consumer_group: non_empty(module.consumer_group.as_deref())
                            .map(str::to_string),
                        handler: non_empty(module.handler.as_deref()).map(str::to_string),
                        consumers: module.consumers.clone().unwrap_or_default(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn build_topology(
    data_sources: &[DataSource],
    schemas: &[Schema],
    data_source_roles: &BTreeMap<String, (Option<String>, Option<String>)>,
    schema_roles: &BTreeMap<String, (Option<String>, Option<String>)>,
    ingress: Vec<TopologyIngressModule>,
) -> Topology {
    let mut schema_names_by_data_source: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for schema in schemas {
        if is_code_only_system_schema(schema) {
            continue;
        }
        schema_names_by_data_source
            .entry(schema.data_source_name.clone())
            .or_default()
            .push(schema.name.clone());
    }
    for schema_names in schema_names_by_data_source.values_mut() {
        schema_names.sort();
    }

    let data_source_providers = data_sources
        .iter()
        .map(|data_source| {
            (
                data_source.name.clone(),
                format!("{:?}", data_source.data_source_type),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let data_sources = data_sources
        .iter()
        .map(|data_source| {
            let (role, description_override) = data_source_roles
                .get(&data_source.name)
                .cloned()
                .unwrap_or_default();
            TopologyDataSource {
                name: data_source.name.clone(),
                provider: format!("{:?}", data_source.data_source_type),
                role,
                description: description_override.or_else(|| data_source.description.clone()),
                is_system_schema_host: data_source.is_system_schema_host.unwrap_or(false),
                schema_names: schema_names_by_data_source
                    .get(&data_source.name)
                    .cloned()
                    .unwrap_or_default(),
                environments: data_source
                    .environments
                    .iter()
                    .map(|environment| environment.name.clone())
                    .collect(),
            }
        })
        .collect();

    let schemas = schemas
        .iter()
        .map(|schema| {
            let (role, description_override) =
                schema_roles.get(&schema.name).cloned().unwrap_or_default();
            let code_only = is_code_only_system_schema(schema);
            TopologySchema {
                name: schema.name.clone(),
                data_source_name: (!code_only).then(|| schema.data_source_name.clone()),
                provider: (!code_only)
                    .then(|| {
                        data_source_providers
                            .get(&schema.data_source_name)
                            .map(|provider| provider.to_string())
                    })
                    .flatten(),
                role,
                description: description_override.unwrap_or_else(|| schema.description.clone()),
                is_system_schema: schema.name == "system",
            }
        })
        .collect();

    Topology {
        data_sources,
        schemas,
        ingress,
    }
}

fn is_code_only_system_schema(schema: &Schema) -> bool {
    schema.name == "system"
        && schema
            .meta
            .as_ref()
            .and_then(|meta| meta.pointer("/storage"))
            .and_then(|value| value.as_str())
            == Some("code_only")
}

fn issue(
    severity: &'static str,
    code: &'static str,
    message: String,
    file: &Path,
    path: &str,
    expected: impl Into<String>,
    actual: Option<String>,
    suggested_fix: impl Into<String>,
) -> TopologyIssue {
    TopologyIssue {
        severity,
        code,
        message,
        file: file.display().to_string(),
        path: path.to_string(),
        expected: expected.into(),
        actual,
        suggested_fix: suggested_fix.into(),
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    let value = value?.trim();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

#[allow(dead_code)]
pub fn topology_report_placeholder() -> serde_json::Value {
    json!({
        "version": 1,
        "source": "app_manifest",
        "note": "App topology is emitted to .appfw/target/appfw/app_topology.json during validation."
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest_path() -> PathBuf {
        PathBuf::from(".appfw/manifest.yaml")
    }

    #[test]
    fn validates_optional_ingress_topology() {
        let topology = ManifestTopology {
            ingress: Some(vec![ManifestIngressModule {
                name: Some("crm-events".to_string()),
                kind: Some("kafka".to_string()),
                schema: None,
                topic: Some("crm.events".to_string()),
                consumer_group: Some("crm-events-worker".to_string()),
                handler: Some("services.crm_events".to_string()),
                ..Default::default()
            }]),
            ..Default::default()
        };
        let mut issues = Vec::new();

        validate_manifest_ingress(&manifest_path(), &topology, &BTreeMap::new(), &mut issues);

        assert!(issues.is_empty());
    }

    #[test]
    fn kafka_ingress_requires_topic_group_and_handler() {
        let topology = ManifestTopology {
            ingress: Some(vec![ManifestIngressModule {
                name: Some("crm-events".to_string()),
                kind: Some("kafka".to_string()),
                ..Default::default()
            }]),
            ..Default::default()
        };
        let mut issues = Vec::new();

        validate_manifest_ingress(&manifest_path(), &topology, &BTreeMap::new(), &mut issues);

        let missing_fields = issues
            .iter()
            .filter(|issue| issue.code == "app_manifest_kafka_ingress_missing_field")
            .count();
        assert_eq!(missing_fields, 3);
    }

    #[test]
    fn http_ingress_path_must_be_absolute() {
        let topology = ManifestTopology {
            ingress: Some(vec![ManifestIngressModule {
                name: Some("crm-http".to_string()),
                kind: Some("http".to_string()),
                path: Some("crm".to_string()),
                ..Default::default()
            }]),
            ..Default::default()
        };
        let mut issues = Vec::new();

        validate_manifest_ingress(&manifest_path(), &topology, &BTreeMap::new(), &mut issues);

        assert!(issues
            .iter()
            .any(|issue| issue.code == "app_manifest_ingress_path"));
    }
}
