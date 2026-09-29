use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Serialize;
use toml_edit::{Document, Item};

use crate::{
    app_manifest::{self, AppTopologyReport},
    app_workspace::AppWorkspace,
    bootstrap_types::data_source::{DataSource, DataSourceEnvironment, DataSourceType},
    utils::{
        artifacts::{self, Artifact, OverwriteMode},
        console, context, files,
    },
};

const DEV_INFRA_REPORT: &str = "dev_infra.json";

#[derive(Debug, Clone)]
struct ComposeDataService {
    service_name: String,
    data_source_names: Vec<String>,
    data_source_type: DataSourceType,
    db_name: String,
    db_port: String,
    volume_name: String,
    host_port_env: String,
    profile: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum FrameworkDependencySourceKind {
    Path,
    Git,
    Registry,
}

impl FrameworkDependencySourceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::Git => "git",
            Self::Registry => "registry",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum FrameworkDependencySource {
    Path { declared_path: String },
    Git,
    Registry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FrameworkDependency {
    declaration_key: String,
    effective_package: String,
    source: FrameworkDependencySource,
}

#[derive(Debug, Clone)]
pub(crate) struct ComposeFrameworkAssets {
    manifest_path: PathBuf,
    declaration_key: String,
    effective_package: String,
    source: FrameworkDependencySourceKind,
    runtime_bind_source: Option<String>,
    runtime_bind_target: Option<String>,
    observability_bind_source: Option<String>,
    rationale: String,
}

impl ComposeFrameworkAssets {
    fn runtime_bind(&self) -> Option<(&str, &str)> {
        assert_eq!(
            self.runtime_bind_source.is_some(),
            self.runtime_bind_target.is_some(),
            "runtime bind source and target must be present together"
        );
        self.runtime_bind_source
            .as_deref()
            .zip(self.runtime_bind_target.as_deref())
    }
}

#[derive(Debug, Serialize)]
struct DevInfraReport {
    version: u32,
    generated_at: String,
    app_name: String,
    compose_path: String,
    topology_report_path: String,
    services: Vec<DevInfraServiceReport>,
    ingress_services: Vec<DevInfraIngressServiceReport>,
    skipped_data_sources: Vec<SkippedDataSourceReport>,
    profiles: Vec<String>,
    framework_dependency: DevInfraFrameworkDependencyReport,
}

#[derive(Debug, Serialize)]
struct DevInfraFrameworkDependencyReport {
    manifest_path: String,
    dependency: String,
    package: String,
    source: FrameworkDependencySourceKind,
    runtime_bind_included: bool,
    runtime_bind_target: Option<String>,
    observability_services_included: bool,
    rationale: String,
}

#[derive(Debug, Serialize)]
struct DevInfraServiceReport {
    service_name: String,
    provider: String,
    data_source_names: Vec<String>,
    compose_profile: Option<String>,
}

#[derive(Debug, Serialize)]
struct DevInfraIngressServiceReport {
    service_name: String,
    ingress_kind: String,
    ingress_names: Vec<String>,
    topics: Vec<String>,
    compose_profile: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct SkippedDataSourceReport {
    name: String,
    provider: String,
    reason: String,
}

pub fn run(workspace: &AppWorkspace, framework_assets: &ComposeFrameworkAssets) -> Result<()> {
    console::stage("dev infra");

    let data_sources = context::read_data_sources(&files::get_data_sources_file(workspace))?;
    let (topology_report, issues) = app_manifest::build(workspace)?;
    if issues.iter().any(|issue| issue.severity == "error") {
        anyhow::bail!("cannot generate dev infra while app topology has validation errors");
    }

    let selected = topology_data_sources(&topology_report, &data_sources)?;
    let (services, skipped) = compose_services(selected);
    let compose_path = workspace.app_root.join("podman-compose.yml");
    let kafka_ingress = enabled_kafka_ingress(&topology_report);
    let compose = render_compose(
        &topology_report,
        &services,
        &kafka_ingress,
        framework_assets,
    );

    artifacts::emit(&Artifact::generated_text(
        compose_path.clone(),
        compose,
        OverwriteMode::Always,
    ))?;
    write_report(
        workspace,
        &topology_report,
        &compose_path,
        &services,
        &kafka_ingress,
        &skipped,
        framework_assets,
    )?;

    console::done("dev infra");
    Ok(())
}

pub(crate) fn preflight(workspace: &AppWorkspace) -> Result<ComposeFrameworkAssets> {
    let manifest_path = workspace.app_root.join("backend/Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path).with_context(|| {
        format!(
            "could not read product backend manifest {}",
            manifest_path.display()
        )
    })?;
    let dependency = framework_dependency_from_manifest(&manifest, &manifest_path)?;

    match dependency.source {
        FrameworkDependencySource::Path { declared_path } => {
            let backend_root = manifest_path.parent().with_context(|| {
                format!("product backend manifest has no parent: {}", manifest_path.display())
            })?;
            let runtime_root = validate_runtime_path_dependency(
                &normalize_path(&backend_root.join(&declared_path)),
                &manifest_path,
            )?;
            let framework_asset_root = runtime_root.parent().with_context(|| {
                format!(
                    "appfw_runtime path has no parent Framework asset root: {}",
                    runtime_root.display()
                )
            })?;
            let runtime_bind_target = runtime_container_bind_target(&declared_path)?;
            let observability_root = validate_observability_asset_root(
                &framework_asset_root.join("observability"),
                &manifest_path,
            )?;
            let (observability_bind_source, rationale) =
                if let Some(observability_root) = observability_root {
                    (
                        Some(relative_path_from(&workspace.app_root, &observability_root)),
                        format!(
                            "path dependency declared by the product backend manifest; local Framework runtime bind is included at {runtime_bind_target} and validated observability services are included"
                        ),
                    )
                } else {
                    (
                        None,
                        format!(
                            "path dependency declared by the product backend manifest; local Framework runtime bind is included at {runtime_bind_target}; observability services are omitted because the sibling observability asset root is absent"
                        ),
                    )
                };
            Ok(ComposeFrameworkAssets {
                manifest_path,
                declaration_key: dependency.declaration_key,
                effective_package: dependency.effective_package,
                source: FrameworkDependencySourceKind::Path,
                runtime_bind_source: Some(relative_path_from(
                    &workspace.app_root,
                    &runtime_root,
                )),
                runtime_bind_target: Some(runtime_bind_target),
                observability_bind_source,
                rationale,
            })
        }
        FrameworkDependencySource::Git => Ok(ComposeFrameworkAssets {
            manifest_path,
            declaration_key: dependency.declaration_key,
            effective_package: dependency.effective_package,
            source: FrameworkDependencySourceKind::Git,
            runtime_bind_source: None,
            runtime_bind_target: None,
            observability_bind_source: None,
            rationale: "Git dependency declared by the product backend manifest; checkout-bound Framework runtime and observability binds are omitted".to_string(),
        }),
        FrameworkDependencySource::Registry => Ok(ComposeFrameworkAssets {
            manifest_path,
            declaration_key: dependency.declaration_key,
            effective_package: dependency.effective_package,
            source: FrameworkDependencySourceKind::Registry,
            runtime_bind_source: None,
            runtime_bind_target: None,
            observability_bind_source: None,
            rationale: "registry dependency declared by the product backend manifest; checkout-bound Framework runtime and observability binds are omitted".to_string(),
        }),
    }
}

fn framework_dependency_from_manifest(
    manifest: &str,
    manifest_path: &Path,
) -> Result<FrameworkDependency> {
    let document = manifest.parse::<Document>().with_context(|| {
        format!(
            "could not parse product backend manifest {}",
            manifest_path.display()
        )
    })?;
    reject_non_root_runtime_dependencies(&document, manifest_path)?;

    let dependencies = document
        .get("dependencies")
        .and_then(Item::as_table_like)
        .with_context(|| {
            format!(
                "product backend manifest has no [dependencies] table: {}",
                manifest_path.display()
            )
        })?;

    let mut candidates = vec![];
    for (name, dependency) in dependencies.iter() {
        if effective_runtime_package(name, dependency, "[dependencies]")? {
            candidates.push((name, dependency));
        }
    }
    if candidates.is_empty() {
        anyhow::bail!(
            "product backend manifest must declare package `appfw-runtime` exactly once in root [dependencies]: {}",
            manifest_path.display()
        );
    }
    if candidates.len() != 1 {
        anyhow::bail!(
            "product backend manifest declares multiple appfw-runtime dependencies; source classification is ambiguous: {}",
            manifest_path.display()
        );
    }

    let (dependency_name, dependency) = candidates[0];

    if let Some(version) = dependency.as_str() {
        if version.trim().is_empty() {
            anyhow::bail!(
                "product backend dependency `{dependency_name}` has an empty registry version"
            );
        }
        return Ok(FrameworkDependency {
            declaration_key: dependency_name.to_string(),
            effective_package: "appfw-runtime".to_string(),
            source: FrameworkDependencySource::Registry,
        });
    }

    if dependency.as_table_like().is_none() {
        anyhow::bail!(
            "product backend dependency `{dependency_name}` must be a non-empty version string or detailed dependency table"
        );
    }
    if dependency_field(dependency, "workspace").is_some() {
        let workspace = dependency_field_bool_required(dependency, "workspace", dependency_name)?;
        anyhow::bail!(
            "product backend dependency `{dependency_name}` uses workspace = {workspace}; workspace-inherited App Framework dependencies are unsupported because generation requires an explicit product path, Git, or registry source"
        );
    }

    let path = dependency_nonempty_string(dependency, "path", dependency_name)?;
    let git = dependency_nonempty_string(dependency, "git", dependency_name)?;
    let version = dependency_nonempty_string(dependency, "version", dependency_name)?;
    let registry = dependency_nonempty_string(dependency, "registry", dependency_name)?;
    if path.is_some() && git.is_some() {
        anyhow::bail!(
            "product backend dependency `{dependency_name}` mixes path and Git selectors; Cargo fallback metadata may add a version to one location, but generation requires one active local source"
        );
    }
    if registry.is_some() && version.is_none() {
        anyhow::bail!(
            "product backend dependency `{dependency_name}` declares `registry` without a non-empty `version`"
        );
    }
    if let Some(path) = path {
        return Ok(FrameworkDependency {
            declaration_key: dependency_name.to_string(),
            effective_package: "appfw-runtime".to_string(),
            source: FrameworkDependencySource::Path {
                declared_path: path.to_string(),
            },
        });
    }
    if git.is_some() {
        if registry.is_some() {
            anyhow::bail!(
                "product backend dependency `{dependency_name}` mixes Git and registry selectors; declare Git with an optional fallback version or a registry source"
            );
        }
        return Ok(FrameworkDependency {
            declaration_key: dependency_name.to_string(),
            effective_package: "appfw-runtime".to_string(),
            source: FrameworkDependencySource::Git,
        });
    }
    if version.is_some() {
        return Ok(FrameworkDependency {
            declaration_key: dependency_name.to_string(),
            effective_package: "appfw-runtime".to_string(),
            source: FrameworkDependencySource::Registry,
        });
    }

    anyhow::bail!(
        "product backend dependency `{dependency_name}` does not declare an unambiguous path, Git, or registry source"
    )
}

fn validate_runtime_path_dependency(runtime_root: &Path, manifest_path: &Path) -> Result<PathBuf> {
    let metadata = fs::symlink_metadata(runtime_root).with_context(|| {
        format!(
            "appfw-runtime path dependency from {} does not exist: {}",
            manifest_path.display(),
            runtime_root.display()
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
        anyhow::bail!(
            "appfw-runtime path dependency must be an existing non-symlink directory: {}",
            runtime_root.display()
        );
    }
    let canonical_root = fs::canonicalize(runtime_root).with_context(|| {
        format!(
            "could not canonicalize appfw-runtime path dependency {}",
            runtime_root.display()
        )
    })?;
    let cargo_manifest = canonical_root.join("Cargo.toml");
    let cargo_metadata = fs::symlink_metadata(&cargo_manifest).with_context(|| {
        format!(
            "appfw-runtime path dependency has no Cargo.toml: {}",
            canonical_root.display()
        )
    })?;
    if cargo_metadata.file_type().is_symlink() || !cargo_metadata.file_type().is_file() {
        anyhow::bail!(
            "appfw-runtime path dependency Cargo.toml must be a regular non-symlink file: {}",
            cargo_manifest.display()
        );
    }
    let cargo_contents = fs::read_to_string(&cargo_manifest)
        .with_context(|| format!("could not read {}", cargo_manifest.display()))?;
    let cargo = cargo_contents.parse::<Document>().with_context(|| {
        format!(
            "could not parse appfw-runtime path dependency manifest {}",
            cargo_manifest.display()
        )
    })?;
    let package_name = cargo
        .get("package")
        .and_then(Item::as_table_like)
        .and_then(|package| package.get("name"))
        .and_then(Item::as_str)
        .with_context(|| {
            format!(
                "appfw-runtime path dependency manifest has no string package.name: {}",
                cargo_manifest.display()
            )
        })?;
    if package_name != "appfw-runtime" {
        anyhow::bail!(
            "appfw-runtime path dependency selects package `{package_name}` instead of `appfw-runtime`: {}",
            cargo_manifest.display()
        );
    }
    let library_source = canonical_root.join("src/lib.rs");
    let library_metadata = fs::symlink_metadata(&library_source).with_context(|| {
        format!(
            "appfw-runtime path dependency has no src/lib.rs package layout: {}",
            canonical_root.display()
        )
    })?;
    if library_metadata.file_type().is_symlink() || !library_metadata.file_type().is_file() {
        anyhow::bail!(
            "appfw-runtime path dependency src/lib.rs must be a regular non-symlink file: {}",
            library_source.display()
        );
    }
    Ok(canonical_root)
}

fn runtime_container_bind_target(declared_path: &str) -> Result<String> {
    let declared = Path::new(declared_path);
    if declared.is_absolute() {
        anyhow::bail!(
            "appfw-runtime path dependency must be relative so it can be reproduced inside the backend container: {declared_path}"
        );
    }

    let mut components = vec!["app".to_string()];
    for component in declared.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                components.pop();
            }
            Component::Normal(component) => components.push(
                component
                    .to_str()
                    .with_context(|| {
                        format!("appfw-runtime path dependency is not valid UTF-8: {declared_path}")
                    })?
                    .to_string(),
            ),
            Component::RootDir | Component::Prefix(_) => {
                anyhow::bail!(
                    "appfw-runtime path dependency must be relative so it can be reproduced inside the backend container: {declared_path}"
                );
            }
        }
    }

    if components.is_empty() || components == ["app"] {
        anyhow::bail!(
            "appfw-runtime path dependency resolves to a conflicting backend container mount target: {declared_path}"
        );
    }
    Ok(format!("/{}", components.join("/")))
}

const REQUIRED_OBSERVABILITY_FILES: &[&str] = &[
    "loki/loki-config.yaml",
    "alloy/config.alloy",
    "prometheus/prometheus.yml",
    "prometheus/alerts.yml",
    "alertmanager/alertmanager.yml",
    "grafana/provisioning/alerting/alerting.yml",
    "grafana/provisioning/dashboards/dashboards.yml",
    "grafana/provisioning/datasources/datasources.yml",
    "grafana/provisioning/plugins/apps.yml",
    "grafana/dashboards/app-framework-backend.json",
];

fn validate_observability_asset_root(
    observability_root: &Path,
    manifest_path: &Path,
) -> Result<Option<PathBuf>> {
    let root_metadata = match fs::symlink_metadata(observability_root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "could not inspect sibling observability asset root for {}: {}",
                    manifest_path.display(),
                    observability_root.display()
                )
            })
        }
    };
    if root_metadata.file_type().is_symlink() || !root_metadata.file_type().is_dir() {
        anyhow::bail!(
            "sibling observability asset root must be an existing non-symlink directory when present: {}",
            observability_root.display()
        );
    }

    for relative in REQUIRED_OBSERVABILITY_FILES {
        validate_observability_file(observability_root, Path::new(relative), manifest_path)?;
    }
    Ok(Some(observability_root.to_path_buf()))
}

fn validate_observability_file(
    observability_root: &Path,
    relative: &Path,
    manifest_path: &Path,
) -> Result<()> {
    let components = relative.components().collect::<Vec<_>>();
    let mut current = observability_root.to_path_buf();
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(component) = component else {
            anyhow::bail!(
                "internal observability asset requirement is not a contained relative path: {}",
                relative.display()
            );
        };
        current.push(component);
        let metadata = fs::symlink_metadata(&current).with_context(|| {
            format!(
                "appfw-runtime path dependency from {} has incomplete observability assets; required path is missing: {}",
                manifest_path.display(),
                current.display()
            )
        })?;
        let is_leaf = index + 1 == components.len();
        if metadata.file_type().is_symlink()
            || (is_leaf && !metadata.file_type().is_file())
            || (!is_leaf && !metadata.file_type().is_dir())
        {
            anyhow::bail!(
                "required observability asset must be a regular non-symlink {}: {}",
                if is_leaf { "file" } else { "directory" },
                current.display()
            );
        }
    }
    Ok(())
}

fn reject_non_root_runtime_dependencies(document: &Document, manifest_path: &Path) -> Result<()> {
    if let Some(workspace_dependencies) = document
        .get("workspace")
        .and_then(Item::as_table_like)
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(Item::as_table_like)
    {
        for (name, dependency) in workspace_dependencies.iter() {
            if effective_runtime_package(name, dependency, "[workspace.dependencies]")? {
                anyhow::bail!(
                    "product backend manifest declares `appfw-runtime` in [workspace.dependencies]; generation requires exactly one explicit root [dependencies] declaration: {}",
                    manifest_path.display()
                );
            }
        }
    }

    if let Some(targets) = document.get("target").and_then(Item::as_table_like) {
        for (target_name, target) in targets.iter() {
            let Some(dependencies) = target
                .as_table_like()
                .and_then(|target| target.get("dependencies"))
                .and_then(Item::as_table_like)
            else {
                continue;
            };
            for (name, dependency) in dependencies.iter() {
                if effective_runtime_package(
                    name,
                    dependency,
                    &format!("[target.{target_name}.dependencies]"),
                )? {
                    anyhow::bail!(
                        "product backend manifest declares `appfw-runtime` in target-specific [target.{target_name}.dependencies]; target-specific App Framework runtime dependencies are unsupported because generated compose requires one platform-independent root source: {}",
                        manifest_path.display()
                    );
                }
            }
        }
    }
    Ok(())
}

fn effective_runtime_package(name: &str, dependency: &Item, table: &str) -> Result<bool> {
    let explicit_package =
        match dependency_field(dependency, "package") {
            None => None,
            Some(item) => Some(item.as_str().with_context(|| {
                format!("dependency `{name}` in {table} has non-string `package`")
            })?),
        };
    if explicit_package.is_some_and(|package| package.trim().is_empty()) {
        anyhow::bail!("dependency `{name}` in {table} has an empty `package`");
    }

    if matches!(name, "appfw-runtime" | "appfw_runtime") {
        if let Some(package) = explicit_package {
            if package != "appfw-runtime" {
                anyhow::bail!(
                    "dependency `{name}` in {table} claims the App Framework runtime name but explicitly selects unrelated package `{package}`"
                );
            }
            return Ok(true);
        }
        if name == "appfw-runtime" {
            return Ok(true);
        }
        anyhow::bail!(
            "dependency alias `appfw_runtime` in {table} must explicitly declare `package = \"appfw-runtime\"`"
        );
    }

    Ok(explicit_package == Some("appfw-runtime"))
}

fn dependency_field<'a>(dependency: &'a Item, field: &str) -> Option<&'a Item> {
    dependency
        .as_table_like()
        .and_then(|table| table.get(field))
}

fn dependency_nonempty_string<'a>(
    dependency: &'a Item,
    field: &str,
    dependency_name: &str,
) -> Result<Option<&'a str>> {
    let Some(item) = dependency_field(dependency, field) else {
        return Ok(None);
    };
    let value = item.as_str().with_context(|| {
        format!("product backend dependency `{dependency_name}` has non-string `{field}`")
    })?;
    if value.trim().is_empty() {
        anyhow::bail!(
            "product backend dependency `{dependency_name}` has an empty `{field}` selector"
        );
    }
    Ok(Some(value))
}

fn dependency_field_bool_required(
    dependency: &Item,
    field: &str,
    dependency_name: &str,
) -> Result<bool> {
    dependency_field(dependency, field)
        .and_then(Item::as_bool)
        .with_context(|| {
            format!("product backend dependency `{dependency_name}` has non-boolean `{field}`")
        })
}

fn topology_data_sources(
    topology_report: &AppTopologyReport,
    data_sources: &[DataSource],
) -> Result<Vec<DataSource>> {
    let configured = data_sources
        .iter()
        .map(|data_source| (data_source.name.as_str(), data_source))
        .collect::<BTreeMap<_, _>>();

    topology_report
        .topology
        .data_sources
        .iter()
        .map(|topology_data_source| {
            configured
                .get(topology_data_source.name.as_str())
                .copied()
                .cloned()
                .with_context(|| {
                    format!(
                        "topology references missing data source `{}`",
                        topology_data_source.name
                    )
                })
        })
        .collect()
}

fn compose_services(
    data_sources: Vec<DataSource>,
) -> (Vec<ComposeDataService>, Vec<SkippedDataSourceReport>) {
    let mut grouped = BTreeMap::<(String, String), ComposeDataService>::new();
    let mut skipped = vec![];

    for data_source in data_sources {
        if data_source.data_source_type == DataSourceType::FabricSqlAnalytics {
            skipped.push(SkippedDataSourceReport {
                name: data_source.name,
                provider: data_source.data_source_type.as_str().to_string(),
                reason: "FabricSqlAnalytics is an external read-only Microsoft Fabric SQL analytics endpoint; no local compose service is generated".to_string(),
            });
            continue;
        }
        if is_external_api_provider(data_source.data_source_type) {
            skipped.push(SkippedDataSourceReport {
                name: data_source.name,
                provider: data_source.data_source_type.as_str().to_string(),
                reason: "External API providers are SaaS systems of record; no local compose database service is generated".to_string(),
            });
            continue;
        }

        let Some(environment) = compose_environment(&data_source) else {
            skipped.push(SkippedDataSourceReport {
                name: data_source.name,
                provider: data_source.data_source_type.as_str().to_string(),
                reason: "no compose environment is configured".to_string(),
            });
            continue;
        };

        let service_name = service_name_for(&data_source, environment);
        let key = (
            data_source.data_source_type.as_str().to_string(),
            service_name.clone(),
        );
        let service = grouped.entry(key).or_insert_with(|| {
            let provider = data_source.data_source_type;
            ComposeDataService {
                service_name: service_name.clone(),
                data_source_names: vec![],
                data_source_type: provider,
                db_name: environment.db_name.clone(),
                db_port: environment.db_port.clone(),
                volume_name: format!("{service_name}-data"),
                host_port_env: format!("APP_{}_HOST_PORT", env_token(&service_name)),
                profile: provider_profile(provider),
            }
        });
        service.data_source_names.push(data_source.name);
        service.data_source_names.sort();
    }

    (grouped.into_values().collect(), skipped)
}

fn compose_environment(data_source: &DataSource) -> Option<&DataSourceEnvironment> {
    data_source
        .environments
        .iter()
        .find(|environment| environment.name == "compose")
}

fn service_name_for(data_source: &DataSource, environment: &DataSourceEnvironment) -> String {
    if data_source.data_source_type == DataSourceType::Snowflake {
        return "snowflake".to_string();
    }

    let sanitized = slug(&environment.db_host);
    if sanitized.is_empty() || sanitized == "localhost" {
        provider_service_name(data_source.data_source_type).to_string()
    } else {
        sanitized
    }
}

fn render_compose(
    topology_report: &AppTopologyReport,
    services: &[ComposeDataService],
    kafka_ingress: &[&app_manifest::TopologyIngressModule],
    framework_assets: &ComposeFrameworkAssets,
) -> String {
    let app_slug = slug(&topology_report.app.name);
    let active_providers = active_providers(services);
    let kafka_enabled = !kafka_ingress.is_empty();

    let mut out = String::new();
    out.push_str("version: '3.8'\n\n");
    out.push_str("# Generated by scripts/appfw generate from .appfw/manifest.yaml and .appfw/model/data_sources/_res.yaml.\n");
    out.push_str("# Edit app topology or data-source config, then rerun scripts/appfw generate.\n");
    if framework_assets.source != FrameworkDependencySourceKind::Path {
        out.push_str(&format!(
            "# Framework dependency source: {}; checkout-bound runtime and observability binds are intentionally omitted.\n",
            framework_assets.source.as_str()
        ));
    }
    out.push_str("services:\n");

    for service in services {
        render_data_service(&mut out, service, &app_slug);
    }

    if kafka_enabled {
        render_kafka_broker(&mut out, &app_slug, kafka_ingress);
    }

    render_backend(
        &mut out,
        services,
        &active_providers,
        kafka_enabled,
        &app_slug,
        framework_assets.runtime_bind(),
    );
    if let Some(observability_source) = framework_assets.observability_bind_source.as_deref() {
        render_observability_services(&mut out, &app_slug, observability_source);
    }

    if active_providers.contains(&DataSourceType::Snowflake) {
        render_snowflake_smoke(&mut out);
    }

    render_volumes(&mut out, services);
    render_networks(
        &mut out,
        framework_assets.observability_bind_source.is_some(),
    );
    out
}

fn render_data_service(out: &mut String, service: &ComposeDataService, app_slug: &str) {
    match service.data_source_type {
        DataSourceType::PostgreSQL => render_postgres(out, service, app_slug),
        DataSourceType::MsSqlServer => render_mssql(out, service, app_slug),
        DataSourceType::FabricSqlAnalytics => {}
        DataSourceType::MongoDB => render_mongo(out, service, app_slug),
        DataSourceType::Snowflake => render_snowflake(out, service, app_slug),
        DataSourceType::Neo4j => render_neo4j(out, service, app_slug),
        DataSourceType::ServiceNow
        | DataSourceType::Workday
        | DataSourceType::Icims
        | DataSourceType::Salesforce
        | DataSourceType::Anaplan
        | DataSourceType::OracleFinancials => {}
    }
}

fn render_postgres(out: &mut String, service: &ComposeDataService, app_slug: &str) {
    out.push_str(&format!(
        r#"
  {name}:
    image: postgres:14
    container_name: ${{APP_STACK_NAME:-{app}}}-{name}
    restart: always
    ports:
      - "${{{host_port_env}:-{port}}}:5432"
    environment:
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: postgres
      POSTGRES_DB: {db_name}
    volumes:
      - type: volume
        source: {volume}
        target: /var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 5s
      timeout: 5s
      retries: 5
    networks:
      - app-network
    security_opt:
      - label=disable
"#,
        app = app_slug,
        db_name = service.db_name,
        host_port_env = service.host_port_env,
        name = service.service_name,
        port = service.db_port,
        volume = service.volume_name,
    ));
}

fn render_mssql(out: &mut String, service: &ComposeDataService, app_slug: &str) {
    out.push_str(&format!(
        r#"
  {name}:
    image: mcr.microsoft.com/mssql/server:2022-latest
    container_name: ${{APP_STACK_NAME:-{app}}}-{name}
    restart: always
    ports:
      - "${{{host_port_env}:-{port}}}:1433"
    environment:
      ACCEPT_EULA: "Y"
      MSSQL_SA_PASSWORD: "${{MSSQL_SERVICE_ACCOUNT_PASS:-YourStrong!Passw0rd}}"
      MSSQL_PID: Developer
    volumes:
      - type: volume
        source: {volume}
        target: /var/opt/mssql
    healthcheck:
      test: ["CMD-SHELL", "/opt/mssql-tools18/bin/sqlcmd -S localhost -U sa -P \"$$MSSQL_SA_PASSWORD\" -C -Q 'SELECT 1' || exit 1"]
      interval: 10s
      timeout: 5s
      retries: 10
      start_period: 30s
    networks:
      - app-network
    security_opt:
      - label=disable
"#,
        app = app_slug,
        host_port_env = service.host_port_env,
        name = service.service_name,
        port = service.db_port,
        volume = service.volume_name,
    ));
}

fn render_mongo(out: &mut String, service: &ComposeDataService, app_slug: &str) {
    out.push_str(&format!(
        r#"
  {name}:
    image: mongo:7
    container_name: ${{APP_STACK_NAME:-{app}}}-{name}
    restart: always
    ports:
      - "${{{host_port_env}:-{port}}}:27017"
    environment:
      MONGO_INITDB_ROOT_USERNAME: "${{MONGO_SERVICE_ACCOUNT_NAME:-mongo}}"
      MONGO_INITDB_ROOT_PASSWORD: "${{MONGO_SERVICE_ACCOUNT_PASS:-mongo}}"
    volumes:
      - type: volume
        source: {volume}
        target: /data/db
    healthcheck:
      test: ["CMD", "mongosh", "--quiet", "--eval", "db.runCommand({{ ping: 1 }}).ok"]
      interval: 10s
      timeout: 5s
      retries: 10
      start_period: 10s
    networks:
      - app-network
    security_opt:
      - label=disable
"#,
        app = app_slug,
        host_port_env = service.host_port_env,
        name = service.service_name,
        port = service.db_port,
        volume = service.volume_name,
    ));
}

fn render_snowflake(out: &mut String, service: &ComposeDataService, app_slug: &str) {
    out.push_str(&format!(
        r#"
  {name}:
    image: localstack/snowflake:${{LOCALSTACK_SNOWFLAKE_TAG:-stable}}
    container_name: ${{APP_STACK_NAME:-{app}}}-{name}
    profiles: ["snowflake"]
    ports:
      - "127.0.0.1:4566:4566"
      - "127.0.0.1:4510-4559:4510-4559"
      - "127.0.0.1:443:443"
    environment:
      - LOCALSTACK_AUTH_TOKEN=${{LOCALSTACK_AUTH_TOKEN:-}}
      - SF_DEFAULT_USER=${{SNOWFLAKE_SERVICE_ACCOUNT_NAME:-test}}
      - SF_DEFAULT_PASSWORD=${{SNOWFLAKE_SERVICE_ACCOUNT_PASS:-test}}
      - SF_LOG=${{SF_LOG:-}}
    volumes:
      - type: volume
        source: {volume}
        target: /var/lib/localstack
    networks:
      app-network:
        aliases:
          - snowflake.localhost.localstack.cloud
    healthcheck:
      test: ["CMD-SHELL", "python3 -c \"import urllib.request; urllib.request.urlopen('http://snowflake.localhost.localstack.cloud:4566/session', data=b'{{}}', timeout=2).read()\""]
      interval: 10s
      timeout: 5s
      retries: 12
      start_period: 20s
    security_opt:
      - label=disable
"#,
        app = app_slug,
        name = service.service_name,
        volume = service.volume_name,
    ));
}

fn render_neo4j(out: &mut String, service: &ComposeDataService, app_slug: &str) {
    let http_host_port_env = format!("APP_{}_HTTP_HOST_PORT", env_token(&service.service_name));
    out.push_str(&format!(
        r#"
  {name}:
    image: neo4j:5
    container_name: ${{APP_STACK_NAME:-{app}}}-{name}
    profiles: ["graph"]
    ports:
      - "127.0.0.1:${{{http_host_port_env}:-7474}}:7474"
      - "127.0.0.1:${{{host_port_env}:-{port}}}:7687"
    environment:
      NEO4J_AUTH: "${{NEO4J_SERVICE_ACCOUNT_NAME:-neo4j}}/${{NEO4J_SERVICE_ACCOUNT_PASS:-appfw-neo4j-local}}"
      NEO4J_SERVICE_ACCOUNT_NAME: "${{NEO4J_SERVICE_ACCOUNT_NAME:-neo4j}}"
      NEO4J_SERVICE_ACCOUNT_PASS: "${{NEO4J_SERVICE_ACCOUNT_PASS:-appfw-neo4j-local}}"
      NEO4J_dbms_security_procedures_unrestricted: ""
    volumes:
      - type: volume
        source: {volume}
        target: /data
    healthcheck:
      test: ["CMD-SHELL", "cypher-shell -a bolt://localhost:7687 -u \"$${{NEO4J_SERVICE_ACCOUNT_NAME}}\" -p \"$${{NEO4J_SERVICE_ACCOUNT_PASS}}\" 'RETURN 1' >/dev/null"]
      interval: 10s
      timeout: 5s
      retries: 12
      start_period: 20s
    networks:
      - app-network
    security_opt:
      - label=disable
"#,
        app = app_slug,
        host_port_env = service.host_port_env,
        http_host_port_env = http_host_port_env,
        name = service.service_name,
        port = service.db_port,
        volume = service.volume_name,
    ));
}

fn render_kafka_broker(
    out: &mut String,
    app_slug: &str,
    kafka_ingress: &[&app_manifest::TopologyIngressModule],
) {
    let topics = kafka_topics(kafka_ingress);
    out.push_str(&format!(
        r#"
  broker:
    image: ${{KAFKA_IMAGE:-apache/kafka:latest}}
    hostname: broker
    container_name: ${{APP_STACK_NAME:-{app}}}-broker
    profiles: ["kafka"]
    ports:
      - "127.0.0.1:${{APP_KAFKA_HOST_PORT:-9092}}:9092"
    environment:
      KAFKA_BROKER_ID: 1
      KAFKA_LISTENER_SECURITY_PROTOCOL_MAP: PLAINTEXT:PLAINTEXT,PLAINTEXT_HOST:PLAINTEXT,CONTROLLER:PLAINTEXT
      KAFKA_ADVERTISED_LISTENERS: PLAINTEXT://broker:29092,PLAINTEXT_HOST://localhost:${{APP_KAFKA_HOST_PORT:-9092}}
      KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR: 1
      KAFKA_GROUP_INITIAL_REBALANCE_DELAY_MS: 0
      KAFKA_TRANSACTION_STATE_LOG_MIN_ISR: 1
      KAFKA_TRANSACTION_STATE_LOG_REPLICATION_FACTOR: 1
      KAFKA_PROCESS_ROLES: broker,controller
      KAFKA_NODE_ID: 1
      KAFKA_CONTROLLER_QUORUM_VOTERS: 1@broker:29093
      KAFKA_LISTENERS: PLAINTEXT://broker:29092,CONTROLLER://broker:29093,PLAINTEXT_HOST://0.0.0.0:9092
      KAFKA_INTER_BROKER_LISTENER_NAME: PLAINTEXT
      KAFKA_CONTROLLER_LISTENER_NAMES: CONTROLLER
      KAFKA_LOG_DIRS: /tmp/kraft-combined-logs
      CLUSTER_ID: ${{APP_KAFKA_CLUSTER_ID:-MkU3OEVBNTcwNTJENDM2Qk}}
    healthcheck:
      test: ["CMD-SHELL", "/opt/kafka/bin/kafka-broker-api-versions.sh --bootstrap-server broker:29092 >/dev/null 2>&1"]
      interval: 10s
      timeout: 5s
      retries: 12
      start_period: 20s
    networks:
      - app-network
    security_opt:
      - label=disable

  kafka-smoke:
    image: ${{KAFKA_IMAGE:-apache/kafka:latest}}
    container_name: ${{APP_STACK_NAME:-{app}}}-kafka-smoke
    profiles: ["kafka"]
    depends_on:
      broker:
        condition: service_healthy
    command:
      - sh
      - -c
      - |
          until /opt/kafka/bin/kafka-topics.sh --bootstrap-server broker:29092 --list >/dev/null 2>&1; do
            sleep 2
          done
"#,
        app = app_slug,
    ));
    for topic in topics {
        out.push_str(&format!(
            "          /opt/kafka/bin/kafka-topics.sh --bootstrap-server broker:29092 --create --if-not-exists --topic {}\n",
            shell_single_quote(&topic),
        ));
    }
    out.push_str(
        r#"          /opt/kafka/bin/kafka-topics.sh --bootstrap-server broker:29092 --list
    networks:
      - app-network
    security_opt:
      - label=disable
"#,
    );
}

fn backend_mssql_odbc_bootstrap_shell_lines() -> &'static str {
    r#"apt-get install -y --no-install-recommends ca-certificates curl gnupg unixodbc unixodbc-dev tdsodbc freetds-bin freetds-common &&
             curl -fsSL https://packages.microsoft.com/keys/microsoft.asc | gpg --dearmor -o /usr/share/keyrings/microsoft-prod.gpg &&
             echo "deb [arch=amd64 signed-by=/usr/share/keyrings/microsoft-prod.gpg] https://packages.microsoft.com/debian/12/prod bookworm main" > /etc/apt/sources.list.d/mssql-release.list &&
             apt-get update &&
             ACCEPT_EULA=Y apt-get install -y --no-install-recommends msodbcsql18 &&
             MS_DRIVER_PATH="$(ls /opt/microsoft/msodbcsql18/lib64/libmsodbcsql-18*.so.* 2>/dev/null | head -1)" &&
             printf "%s\n" "[FreeTDS]" "Description=FreeTDS Driver" "Driver=/usr/lib/x86_64-linux-gnu/odbc/libtdsodbc.so" "Setup=/usr/lib/x86_64-linux-gnu/odbc/libtdsodbc.so" "UsageCount=1" "" "[ODBC Driver 18 for SQL Server]" "Description=Microsoft ODBC Driver 18 for SQL Server" "Driver=${MS_DRIVER_PATH}" "UsageCount=1" > /etc/odbcinst.ini &&
             "#
}

fn render_backend(
    out: &mut String,
    services: &[ComposeDataService],
    active_providers: &[DataSourceType],
    kafka_enabled: bool,
    app_slug: &str,
    runtime_bind: Option<(&str, &str)>,
) {
    let mssql_odbc_bootstrap = if active_providers.contains(&DataSourceType::MsSqlServer) {
        backend_mssql_odbc_bootstrap_shell_lines()
    } else {
        ""
    };
    out.push_str(&format!(
        r#"
  backend:
    image: rust:latest
    container_name: ${{APP_STACK_NAME:-{app}}}-backend
    working_dir: /app
    volumes:
      - type: bind
        source: ./backend
        target: /app
"#,
        app = app_slug,
    ));
    if let Some((runtime_source, runtime_target)) = runtime_bind {
        out.push_str(&format!(
            r#"      - type: bind
        source: {runtime_source}
        target: {runtime_target}
"#,
            runtime_source = runtime_source,
            runtime_target = runtime_target,
        ));
    }
    out.push_str(&format!(
        r#"      - type: volume
        source: backend-observability-logs
        target: /var/log/app-framework
    ports:
      - "127.0.0.1:${{API_PORT:-3000}}:${{API_PORT:-3000}}"
    command: >
      sh -c 'apt-get update &&
             apt-get install -y python3-dev python3-pip python3-venv &&
             {mssql_odbc_bootstrap}if [ -f requirements.txt ]; then pip3 install --break-system-packages -r requirements.txt; fi &&
             mkdir -p /var/log/app-framework &&
             rm -f /tmp/backend-log &&
             mkfifo /tmp/backend-log &&
             (tee -a /var/log/app-framework/backend.log < /tmp/backend-log &) &&
             cargo run --bin backend > /tmp/backend-log 2>&1'
"#,
        mssql_odbc_bootstrap = mssql_odbc_bootstrap,
    ));

    let dependencies = services
        .iter()
        .filter(|service| service.profile.is_none())
        .map(|service| service.service_name.as_str())
        .collect::<Vec<_>>();
    if !dependencies.is_empty() {
        out.push_str("    depends_on:\n");
        for dependency in dependencies {
            out.push_str(&format!(
                r#"      {dependency}:
        condition: service_healthy
"#
            ));
        }
    }

    out.push_str(
        r#"    environment:
      - API_HOST=${API_HOST:-0.0.0.0}
      - API_PORT=${API_PORT:-3000}
      - ENV_NAME=${ENV_NAME:-compose}
"#,
    );
    render_backend_provider_env(out, active_providers);
    render_backend_kafka_env(out, kafka_enabled);
    out.push_str(
        r#"      - APP_DATA_SOURCE_NAME=${APP_DATA_SOURCE_NAME:-}
      - APP_ENABLE_LOCAL_TEST_AUTH=${APP_ENABLE_LOCAL_TEST_AUTH:-false}
      - APP_PROVIDER_CERTIFICATION_CI=${APP_PROVIDER_CERTIFICATION_CI:-false}
      - LOG_LEVEL=${LOG_LEVEL:-info}
      - RUST_LOG=${RUST_LOG:-backend=info,tower_http=info}
      - APP_LOG_JSON=${APP_LOG_JSON:-true}
      - OTEL_SERVICE_NAME=${OTEL_SERVICE_NAME:-backend}
      - PYTHONPATH=/app/src/handlers/python
    networks:
      - app-network
    security_opt:
      - label=disable
"#,
    );
}

fn render_backend_provider_env(out: &mut String, active_providers: &[DataSourceType]) {
    if active_providers.contains(&DataSourceType::PostgreSQL) {
        out.push_str(
            r#"      - PG_SERVICE_ACCOUNT_NAME=${PG_SERVICE_ACCOUNT_NAME:-postgres}
      - PG_SERVICE_ACCOUNT_PASS=${PG_SERVICE_ACCOUNT_PASS:-postgres}
      - PG_SERVICE_ACCOUNT_PASSWORD=${PG_SERVICE_ACCOUNT_PASSWORD:-postgres}
"#,
        );
    }
    if active_providers.contains(&DataSourceType::MongoDB) {
        out.push_str(
            r#"      - MONGO_SERVICE_ACCOUNT_NAME=${MONGO_SERVICE_ACCOUNT_NAME:-mongo}
      - MONGO_SERVICE_ACCOUNT_PASS=${MONGO_SERVICE_ACCOUNT_PASS:-mongo}
"#,
        );
    }
    if active_providers.contains(&DataSourceType::MsSqlServer) {
        out.push_str(
            r#"      - MSSQL_SERVICE_ACCOUNT_NAME=${MSSQL_SERVICE_ACCOUNT_NAME:-sa}
      - MSSQL_SERVICE_ACCOUNT_PASS=${MSSQL_SERVICE_ACCOUNT_PASS:-YourStrong!Passw0rd}
"#,
        );
    }
    if active_providers.contains(&DataSourceType::Snowflake) {
        out.push_str(
            r#"      - SNOWFLAKE_HOST=${SNOWFLAKE_HOST:-}
      - SNOWFLAKE_WAREHOUSE=${SNOWFLAKE_WAREHOUSE:-}
      - SNOWFLAKE_ROLE=${SNOWFLAKE_ROLE:-}
      - SNOWFLAKE_AUTH_TOKEN_TYPE=${SNOWFLAKE_AUTH_TOKEN_TYPE:-PROGRAMMATIC_ACCESS_TOKEN}
      - SNOWFLAKE_SERVICE_ACCOUNT_NAME=${SNOWFLAKE_SERVICE_ACCOUNT_NAME:-}
      - SNOWFLAKE_SERVICE_ACCOUNT_PASS=${SNOWFLAKE_SERVICE_ACCOUNT_PASS:-}
      - SNOWFLAKE_ACCESS_TOKEN=${SNOWFLAKE_ACCESS_TOKEN:-}
"#,
        );
    }
    if active_providers.contains(&DataSourceType::Neo4j) {
        out.push_str(
            r#"      - NEO4J_SERVICE_ACCOUNT_NAME=${NEO4J_SERVICE_ACCOUNT_NAME:-neo4j}
      - NEO4J_SERVICE_ACCOUNT_PASS=${NEO4J_SERVICE_ACCOUNT_PASS:-appfw-neo4j-local}
"#,
        );
    }
}

fn render_backend_kafka_env(out: &mut String, kafka_enabled: bool) {
    if kafka_enabled {
        out.push_str(
            r#"      - APPFW_KAFKA_BOOTSTRAP_SERVERS=${APPFW_KAFKA_BOOTSTRAP_SERVERS:-broker:29092}
      - APPFW_KAFKA_SECURITY_PROTOCOL=${APPFW_KAFKA_SECURITY_PROTOCOL:-PLAINTEXT}
"#,
        );
    }
}

fn render_observability_services(out: &mut String, app_slug: &str, observability_source: &str) {
    out.push_str(&format!(
        r#"
  loki:
    image: ${{LOKI_IMAGE:-grafana/loki:3.6.0}}
    container_name: ${{APP_STACK_NAME:-{app}}}-loki
    profiles: ["observability"]
    command: ["-config.file=/etc/loki/loki-config.yaml"]
    volumes:
      - type: bind
        source: {observability_source}/loki/loki-config.yaml
        target: /etc/loki/loki-config.yaml
        read_only: true
    tmpfs:
      - /loki:size=512m,mode=1777
    networks:
      - observability-network
    cap_drop:
      - ALL
    security_opt:
      - no-new-privileges:true
      - label=disable

  alloy:
    image: ${{ALLOY_IMAGE:-grafana/alloy:v1.16.1-boringcrypto}}
    container_name: ${{APP_STACK_NAME:-{app}}}-alloy
    profiles: ["observability"]
    command:
      - run
      - --server.http.listen-addr=0.0.0.0:12345
      - --storage.path=/tmp/alloy-data
      - /etc/alloy/config.alloy
    depends_on:
      - loki
    volumes:
      - type: bind
        source: {observability_source}/alloy/config.alloy
        target: /etc/alloy/config.alloy
        read_only: true
      - type: volume
        source: backend-observability-logs
        target: /var/log/app-framework
        read_only: true
    tmpfs:
      - /tmp:size=128m,mode=1777
    networks:
      - observability-network
    cap_drop:
      - ALL
    security_opt:
      - no-new-privileges:true
      - label=disable

  prometheus:
    image: ${{PROMETHEUS_IMAGE:-prom/prometheus:v3.7.3}}
    container_name: ${{APP_STACK_NAME:-{app}}}-prometheus
    profiles: ["observability"]
    command:
      - --config.file=/etc/prometheus/prometheus.yml
      - --storage.tsdb.path=/prometheus
      - --storage.tsdb.retention.time=${{PROMETHEUS_RETENTION:-24h}}
    depends_on:
      - backend
      - alertmanager
      - alloy
      - loki
    volumes:
      - type: bind
        source: {observability_source}/prometheus/prometheus.yml
        target: /etc/prometheus/prometheus.yml
        read_only: true
      - type: bind
        source: {observability_source}/prometheus/alerts.yml
        target: /etc/prometheus/alerts.yml
        read_only: true
    tmpfs:
      - /prometheus:size=512m,mode=1777
    networks:
      - app-network
      - observability-network
    cap_drop:
      - ALL
    security_opt:
      - no-new-privileges:true
      - label=disable

  alertmanager:
    image: ${{ALERTMANAGER_IMAGE:-prom/alertmanager:v0.29.0}}
    container_name: ${{APP_STACK_NAME:-{app}}}-alertmanager
    profiles: ["observability"]
    command:
      - --config.file=/etc/alertmanager/alertmanager.yml
      - --storage.path=/alertmanager
      - --web.listen-address=0.0.0.0:9093
    ports:
      - "127.0.0.1:${{ALERTMANAGER_PORT:-9093}}:9093"
    volumes:
      - type: bind
        source: {observability_source}/alertmanager/alertmanager.yml
        target: /etc/alertmanager/alertmanager.yml
        read_only: true
    tmpfs:
      - /alertmanager:size=128m,mode=1777
    networks:
      - observability-network
    cap_drop:
      - ALL
    security_opt:
      - no-new-privileges:true
      - label=disable

  grafana:
    image: ${{GRAFANA_IMAGE:-grafana/grafana:12.4.3}}
    container_name: ${{APP_STACK_NAME:-{app}}}-grafana
    profiles: ["observability"]
    depends_on:
      - loki
      - prometheus
    ports:
      - "127.0.0.1:${{GRAFANA_PORT:-3001}}:3000"
    environment:
      - GF_SECURITY_ADMIN_USER=${{GRAFANA_ADMIN_USER:-admin}}
      - GF_SECURITY_ADMIN_PASSWORD=${{GRAFANA_ADMIN_PASSWORD:-appfw-local-observability-change-me}}
      - GF_AUTH_ANONYMOUS_ENABLED=false
      - GF_USERS_ALLOW_SIGN_UP=false
      - GF_USERS_ALLOW_ORG_CREATE=false
      - GF_SECURITY_DISABLE_GRAVATAR=true
      - GF_ANALYTICS_REPORTING_ENABLED=false
      - GF_ANALYTICS_CHECK_FOR_UPDATES=false
      - GF_SNAPSHOTS_EXTERNAL_ENABLED=false
      - GF_LOG_LEVEL=warn
      - GF_SERVER_DOMAIN=localhost
    volumes:
      - type: bind
        source: {observability_source}/grafana/provisioning
        target: /etc/grafana/provisioning
        read_only: true
      - type: bind
        source: {observability_source}/grafana/dashboards
        target: /etc/grafana/dashboards
        read_only: true
    tmpfs:
      - /var/lib/grafana:size=256m,mode=1777
    networks:
      - observability-network
    cap_drop:
      - ALL
    security_opt:
      - no-new-privileges:true
      - label=disable
"#,
        app = app_slug,
        observability_source = observability_source
    ));
}

fn render_snowflake_smoke(out: &mut String) {
    out.push_str(
        r#"
  snowflake-smoke:
    image: curlimages/curl:8.10.1
    profiles: ["snowflake"]
    depends_on:
      snowflake:
        condition: service_healthy
    command: >
      sh -c 'curl -fsS -d "{}" "http://snowflake.localhost.localstack.cloud:4566/session" &&
             curl -fsS "http://snowflake.localhost.localstack.cloud:4566/api/v2/statements?nullable=true"
               -X POST
               -H "Content-Type: application/json"
               -d "{\"statement\":\"SELECT 1\",\"timeout\":60,\"database\":\"test\"}"'
    environment:
      - SNOWFLAKE_ACCESS_TOKEN=${SNOWFLAKE_ACCESS_TOKEN:-test}
      - SNOWFLAKE_AUTH_TOKEN_TYPE=${SNOWFLAKE_AUTH_TOKEN_TYPE:-LOCALSTACK_NO_AUTH}
    networks:
      app-network:
        aliases:
          - snowflake-smoke
    security_opt:
      - label=disable
"#,
    );
}

fn render_volumes(out: &mut String, services: &[ComposeDataService]) {
    out.push_str("\nvolumes:\n");
    for service in services {
        out.push_str(&format!(
            r#"  {volume}:
    driver: local
"#,
            volume = service.volume_name,
        ));
    }
    out.push_str(
        r#"  backend-observability-logs:
    driver: local
    driver_opts:
      type: tmpfs
      device: tmpfs
      o: size=64m,mode=1777
"#,
    );
}

fn render_networks(out: &mut String, observability_included: bool) {
    out.push_str(
        r#"
networks:
  app-network:
    driver: bridge
"#,
    );
    if observability_included {
        out.push_str(
            r#"  observability-network:
    driver: bridge
"#,
        );
    }
}

fn write_report(
    workspace: &AppWorkspace,
    topology_report: &AppTopologyReport,
    compose_path: &std::path::Path,
    services: &[ComposeDataService],
    kafka_ingress: &[&app_manifest::TopologyIngressModule],
    skipped: &[SkippedDataSourceReport],
    framework_assets: &ComposeFrameworkAssets,
) -> Result<()> {
    let mut profiles = services
        .iter()
        .filter_map(|service| service.profile.map(str::to_string))
        .collect::<BTreeSet<_>>();
    if !kafka_ingress.is_empty() {
        profiles.insert("kafka".to_string());
    }
    let profiles = profiles.into_iter().collect::<Vec<_>>();
    let report = DevInfraReport {
        version: 1,
        generated_at: chrono::offset::Utc::now().to_rfc3339(),
        app_name: topology_report.app.name.clone(),
        compose_path: compose_path.display().to_string(),
        topology_report_path: app_manifest::report_path(workspace).display().to_string(),
        services: services
            .iter()
            .map(|service| DevInfraServiceReport {
                service_name: service.service_name.clone(),
                provider: service.data_source_type.as_str().to_string(),
                data_source_names: service.data_source_names.clone(),
                compose_profile: service.profile.map(str::to_string),
            })
            .collect(),
        ingress_services: kafka_ingress_services(kafka_ingress),
        skipped_data_sources: skipped.to_vec(),
        profiles,
        framework_dependency: DevInfraFrameworkDependencyReport {
            manifest_path: framework_assets.manifest_path.display().to_string(),
            dependency: framework_assets.declaration_key.clone(),
            package: framework_assets.effective_package.clone(),
            source: framework_assets.source,
            runtime_bind_included: framework_assets.runtime_bind().is_some(),
            runtime_bind_target: framework_assets.runtime_bind_target.clone(),
            observability_services_included: framework_assets.observability_bind_source.is_some(),
            rationale: framework_assets.rationale.clone(),
        },
    };
    let report_path = workspace.target_file(DEV_INFRA_REPORT);
    artifacts::safe_write(&report_path, serde_json::to_string_pretty(&report)? + "\n")
        .with_context(|| format!("could not write {}", report_path.display()))?;
    console::write(&report_path);
    Ok(())
}

fn enabled_kafka_ingress(
    topology_report: &AppTopologyReport,
) -> Vec<&app_manifest::TopologyIngressModule> {
    topology_report
        .topology
        .ingress
        .iter()
        .filter(|module| module.kind == "kafka" && module.enabled)
        .collect()
}

fn kafka_ingress_services(
    kafka_ingress: &[&app_manifest::TopologyIngressModule],
) -> Vec<DevInfraIngressServiceReport> {
    if kafka_ingress.is_empty() {
        return vec![];
    }

    vec![DevInfraIngressServiceReport {
        service_name: "broker".to_string(),
        ingress_kind: "kafka".to_string(),
        ingress_names: kafka_ingress
            .iter()
            .map(|module| module.name.clone())
            .collect(),
        topics: kafka_topics(kafka_ingress),
        compose_profile: Some("kafka".to_string()),
    }]
}

fn kafka_topics(kafka_ingress: &[&app_manifest::TopologyIngressModule]) -> Vec<String> {
    kafka_ingress
        .iter()
        .filter_map(|module| module.topic.as_deref())
        .filter(|topic| !topic.trim().is_empty())
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn provider_profile(provider: DataSourceType) -> Option<&'static str> {
    match provider {
        DataSourceType::FabricSqlAnalytics => Some("external"),
        DataSourceType::ServiceNow
        | DataSourceType::Workday
        | DataSourceType::Icims
        | DataSourceType::Salesforce
        | DataSourceType::Anaplan
        | DataSourceType::OracleFinancials => Some("external"),
        DataSourceType::Snowflake => Some("snowflake"),
        DataSourceType::Neo4j => Some("graph"),
        _ => None,
    }
}

fn is_external_api_provider(provider: DataSourceType) -> bool {
    matches!(
        provider,
        DataSourceType::ServiceNow
            | DataSourceType::Workday
            | DataSourceType::Icims
            | DataSourceType::Salesforce
            | DataSourceType::Anaplan
            | DataSourceType::OracleFinancials
    )
}

fn active_providers(services: &[ComposeDataService]) -> Vec<DataSourceType> {
    let mut providers = Vec::new();
    for service in services {
        if !providers.contains(&service.data_source_type) {
            providers.push(service.data_source_type);
        }
    }
    providers
}

fn provider_service_name(provider: DataSourceType) -> &'static str {
    match provider {
        DataSourceType::PostgreSQL => "postgres",
        DataSourceType::MongoDB => "mongo",
        DataSourceType::MsSqlServer => "mssql",
        DataSourceType::FabricSqlAnalytics => "fabric-sql-analytics",
        DataSourceType::Snowflake => "snowflake",
        DataSourceType::Neo4j => "neo4j",
        DataSourceType::ServiceNow => "servicenow",
        DataSourceType::Workday => "workday",
        DataSourceType::Icims => "icims",
        DataSourceType::Salesforce => "salesforce",
        DataSourceType::Anaplan => "anaplan",
        DataSourceType::OracleFinancials => "oracle-financials",
    }
}

fn slug(value: &str) -> String {
    let mut out = String::new();
    let mut last_was_dash = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash && !out.is_empty() {
            out.push('-');
            last_was_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

fn env_token(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn relative_path_from(from: &Path, to: &Path) -> String {
    let from = normalize_path(from);
    let to = normalize_path(to);
    let from_components = path_components(&from);
    let to_components = path_components(&to);
    let common_len = from_components
        .iter()
        .zip(to_components.iter())
        .take_while(|(left, right)| left == right)
        .count();

    let mut relative = PathBuf::new();
    for _ in common_len..from_components.len() {
        relative.push("..");
    }
    for component in to_components.iter().skip(common_len) {
        relative.push(component);
    }

    if relative.as_os_str().is_empty() {
        ".".to_string()
    } else {
        let rendered = relative.display().to_string();
        if rendered == ".." || rendered.starts_with("../") {
            rendered
        } else {
            format!("./{rendered}")
        }
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn path_components(path: &Path) -> Vec<String> {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn compose_services_include_only_data_sources_with_compose_environments() {
        let (services, skipped) = compose_services(vec![
            data_source("equity", DataSourceType::MsSqlServer, Some("mssql")),
            data_source("warehouse", DataSourceType::Snowflake, None),
        ]);

        assert_eq!(services.len(), 1);
        assert_eq!(services[0].service_name, "mssql");
        assert_eq!(services[0].data_source_names, vec!["equity"]);
        assert_eq!(skipped.len(), 1);
        assert_eq!(skipped[0].name, "warehouse");
    }

    #[test]
    fn framework_bind_sources_point_from_product_to_framework_root() {
        assert_eq!(
            relative_path_from(
                Path::new("/work/od-equity-report"),
                Path::new("/work/app-framework/appfw_runtime"),
            ),
            "../app-framework/appfw_runtime"
        );
        assert_eq!(
            relative_path_from(
                Path::new("/work/app-framework/examples/products/crm"),
                Path::new("/work/app-framework/appfw_runtime"),
            ),
            "../../../appfw_runtime"
        );
        assert_eq!(
            relative_path_from(
                Path::new("/work/od-equity-report"),
                Path::new("/work/app-framework/observability"),
            ),
            "../app-framework/observability"
        );
        assert_eq!(
            relative_path_from(
                Path::new("/work/app-framework"),
                Path::new("/work/app-framework/appfw_runtime"),
            ),
            "./appfw_runtime"
        );
        assert_eq!(
            relative_path_from(
                Path::new("/work/app-framework/examples/products/crm"),
                Path::new("/work/app-framework/observability"),
            ),
            "../../../observability"
        );
    }

    #[test]
    fn enabled_kafka_ingress_emits_broker_and_smoke_services() {
        let report = topology_report_with_kafka(true);
        let kafka_ingress = enabled_kafka_ingress(&report);
        let compose = render_compose(&report, &[], &kafka_ingress, &path_framework_assets());

        assert!(compose.contains("  broker:\n"));
        assert!(compose.contains("profiles: [\"kafka\"]"));
        assert!(compose.contains("image: ${KAFKA_IMAGE:-apache/kafka:latest}"));
        assert!(compose.contains("PLAINTEXT://broker:29092"));
        assert!(compose.contains("PLAINTEXT_HOST://localhost:${APP_KAFKA_HOST_PORT:-9092}"));
        assert!(compose.contains("  kafka-smoke:\n"));
        assert!(compose.contains("--create --if-not-exists --topic 'crm.events'"));
        assert!(compose.contains(
            "APPFW_KAFKA_BOOTSTRAP_SERVERS=${APPFW_KAFKA_BOOTSTRAP_SERVERS:-broker:29092}"
        ));
    }

    #[test]
    fn neo4j_data_source_renders_graph_profile_service() {
        let (services, skipped) = compose_services(vec![data_source(
            "graph",
            DataSourceType::Neo4j,
            Some("neo4j"),
        )]);
        assert!(skipped.is_empty());
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].profile, Some("graph"));

        let report = topology_report_with_kafka(false);
        let compose = render_compose(&report, &services, &[], &path_framework_assets());

        assert!(compose.contains("image: neo4j:5"));
        assert!(compose.contains("profiles: [\"graph\"]"));
        assert!(compose.contains("NEO4J_SERVICE_ACCOUNT_NAME"));
        assert!(compose.contains("NEO4J_SERVICE_ACCOUNT_PASS"));
    }

    #[test]
    fn disabled_kafka_ingress_leaves_compose_without_kafka_services() {
        let report = topology_report_with_kafka(false);
        let kafka_ingress = enabled_kafka_ingress(&report);
        let compose = render_compose(&report, &[], &kafka_ingress, &path_framework_assets());

        assert!(kafka_ingress.is_empty());
        assert!(!compose.contains("  broker:\n"));
        assert!(!compose.contains("  kafka-smoke:\n"));
        assert!(!compose.contains("APPFW_KAFKA_BOOTSTRAP_SERVERS"));
    }

    #[test]
    fn kafka_topics_are_unique_and_shell_quoted() {
        let report = topology_report_with_kafka(true);
        let kafka_ingress = enabled_kafka_ingress(&report);

        assert_eq!(kafka_topics(&kafka_ingress), vec!["crm.events"]);
        assert_eq!(shell_single_quote("crm.events"), "'crm.events'");
        assert_eq!(shell_single_quote("crm's.events"), "'crm'\\''s.events'");
    }

    #[test]
    fn classifies_path_git_and_registry_runtime_dependencies_from_product_manifest() {
        let manifest_path = Path::new("/work/fabric-console/backend/Cargo.toml");
        let path = framework_dependency_from_manifest(
            r#"
[dependencies]
appfw_runtime = { package = "appfw-runtime", path = "../../app-framework/appfw_runtime", default-features = false }
"#,
            manifest_path,
        )
        .expect("classify path dependency");
        assert_eq!(
            &path.source,
            &FrameworkDependencySource::Path {
                declared_path: "../../app-framework/appfw_runtime".to_string(),
            },
        );
        assert_eq!(path.declaration_key, "appfw_runtime");

        let git = framework_dependency_from_manifest(
            r#"
[dependencies]
appfw_runtime = { package = "appfw-runtime", git = "https://bitbucket.org/pacificdental/app-framework.git", rev = "9ea26157a2bed29424e1052a909303985ba466bd", default-features = false }
"#,
            manifest_path,
        )
        .expect("classify exact Fabric Git dependency");
        assert_eq!(git.source, FrameworkDependencySource::Git);

        let registry = framework_dependency_from_manifest(
            r#"
[dependencies]
runtime = { package = "appfw-runtime", version = "0.2.0", registry = "pds-app-framework-crates", default-features = false }
"#,
            manifest_path,
        )
        .expect("classify aliased registry dependency");
        assert_eq!(&registry.source, &FrameworkDependencySource::Registry);
        assert_eq!(registry.declaration_key, "runtime");

        let shorthand = framework_dependency_from_manifest(
            "[dependencies]\nappfw-runtime = \"0.2.0\"\n",
            manifest_path,
        )
        .expect("classify shorthand registry dependency");
        assert_eq!(&shorthand.source, &FrameworkDependencySource::Registry);
        assert_eq!(shorthand.declaration_key, "appfw-runtime");

        let canonical_git = framework_dependency_from_manifest(
            r#"
[dependencies]
appfw-runtime = { git = "https://example.invalid/app-framework.git", rev = "exact" }
"#,
            manifest_path,
        )
        .expect("classify canonical hyphenated dependency key");
        assert_eq!(&canonical_git.source, &FrameworkDependencySource::Git);
        assert_eq!(canonical_git.declaration_key, "appfw-runtime");

        for manifest in [
            r#"
[dependencies]
appfw-runtime = { path = "../runtime", version = "0.2.0" }
"#,
            r#"
[dependencies]
appfw-runtime = { path = "../runtime", version = "0.2.0", registry = "pds-app-framework-crates" }
"#,
        ] {
            let dual_location = framework_dependency_from_manifest(manifest, manifest_path)
                .expect("classify Cargo path plus publish fallback metadata");
            assert_eq!(
                dual_location.source,
                FrameworkDependencySource::Path {
                    declared_path: "../runtime".to_string(),
                }
            );
        }

        let git_with_version = framework_dependency_from_manifest(
            r#"
[dependencies]
appfw-runtime = { git = "https://example.invalid/runtime.git", rev = "exact", version = "0.2.0" }
"#,
            manifest_path,
        )
        .expect("classify Cargo Git plus publish fallback version");
        assert_eq!(git_with_version.source, FrameworkDependencySource::Git);
    }

    #[test]
    fn ambiguous_or_workspace_only_runtime_dependencies_fail_closed() {
        let manifest_path = Path::new("/work/fabric-console/backend/Cargo.toml");
        let mixed = framework_dependency_from_manifest(
            r#"
[dependencies]
appfw_runtime = { package = "appfw-runtime", path = "../runtime", git = "https://example.invalid/runtime.git" }
"#,
            manifest_path,
        )
        .expect_err("mixed dependency source must fail");
        assert!(mixed.to_string().contains("mixes path and Git"));

        let workspace = framework_dependency_from_manifest(
            "[dependencies]\nappfw_runtime = { package = \"appfw-runtime\", workspace = true }\n",
            manifest_path,
        )
        .expect_err("workspace-only dependency must fail");
        assert!(workspace.to_string().contains("workspace = true"));

        let unspecified = framework_dependency_from_manifest(
            "[dependencies]\nappfw_runtime = { package = \"appfw-runtime\", default-features = false }\n",
            manifest_path,
        )
        .expect_err("unspecified dependency source must fail");
        assert!(unspecified
            .to_string()
            .contains("unambiguous path, Git, or registry"));
    }

    #[test]
    fn rejects_spoofed_target_specific_and_empty_runtime_dependency_shapes() {
        let manifest_path = Path::new("/work/fabric-console/backend/Cargo.toml");
        let spoofed = framework_dependency_from_manifest(
            r#"
[dependencies]
appfw_runtime = { package = "unrelated-runtime", git = "https://example.invalid/runtime.git" }
"#,
            manifest_path,
        )
        .expect_err("spoofed runtime identity must fail");
        assert!(spoofed.to_string().contains("unrelated package"));

        let target_specific = framework_dependency_from_manifest(
            r#"
[target.'cfg(unix)'.dependencies]
appfw-runtime = { git = "https://example.invalid/runtime.git" }
"#,
            manifest_path,
        )
        .expect_err("target-specific runtime dependency must fail");
        assert!(target_specific.to_string().contains("target-specific"));

        for manifest in [
            "[dependencies]\nappfw-runtime = { version = \"\" }\n",
            "[dependencies]\nappfw-runtime = { version = \"1\", registry = \"\" }\n",
            "[dependencies]\nappfw-runtime = { registry = \"private\" }\n",
        ] {
            let error = framework_dependency_from_manifest(manifest, manifest_path)
                .expect_err("empty or incomplete registry selector must fail");
            assert!(
                error.to_string().contains("empty")
                    || error.to_string().contains("without a non-empty `version`")
            );
        }
    }

    #[test]
    fn git_and_registry_compose_omit_checkout_bound_framework_assets() {
        let report = topology_report_with_kafka(true);
        for source in [
            FrameworkDependencySourceKind::Git,
            FrameworkDependencySourceKind::Registry,
        ] {
            let compose = render_compose(
                &report,
                &[],
                &enabled_kafka_ingress(&report),
                &portable_framework_assets(source),
            );

            assert!(compose.contains(&format!(
                "# Framework dependency source: {}; checkout-bound runtime and observability binds are intentionally omitted.",
                source.as_str()
            )));
            assert!(compose.contains("source: ./backend"));
            assert!(compose.contains("source: backend-observability-logs"));
            assert!(!compose.contains("target: /appfw_runtime"));
            assert!(!compose.contains("  loki:\n"));
            assert!(!compose.contains("  alloy:\n"));
            assert!(!compose.contains("  prometheus:\n"));
            assert!(!compose.contains("  alertmanager:\n"));
            assert!(!compose.contains("  grafana:\n"));
            assert!(!compose.contains("observability-network"));
        }
    }

    #[test]
    fn path_compose_keeps_runtime_and_observability_binds() {
        let report = topology_report_with_kafka(false);
        let compose = render_compose(&report, &[], &[], &path_framework_assets());

        assert!(!compose.contains("# Framework dependency source:"));
        assert!(compose.contains("source: ./appfw_runtime\n        target: /appfw_runtime"));
        assert!(compose.contains("source: ./observability/loki/loki-config.yaml"));
        assert!(compose.contains("source: ./observability/prometheus/alerts.yml"));
        assert!(compose.contains("  grafana:\n"));
        assert!(compose.contains("  observability-network:\n"));
    }

    #[test]
    fn runtime_bind_targets_match_cargo_resolution_from_container_app_root() {
        let crm_manifest = include_str!("../../examples/products/crm/backend/Cargo.toml");
        let crm_dependency = framework_dependency_from_manifest(
            crm_manifest,
            Path::new("examples/products/crm/backend/Cargo.toml"),
        )
        .expect("classify the checked-in CRM runtime dependency");
        let FrameworkDependencySource::Path {
            declared_path: crm_path,
        } = crm_dependency.source
        else {
            panic!("checked-in CRM runtime dependency must remain path-based");
        };
        assert_eq!(crm_path, "../../../../appfw_runtime");

        for (declared_path, expected_target) in [
            ("../runtime", "/runtime"),
            (crm_path.as_str(), "/appfw_runtime"),
        ] {
            let emitted_target = runtime_container_bind_target(declared_path)
                .expect("derive runtime bind target from Cargo path");
            let cargo_resolved = normalize_path(&Path::new("/app").join(declared_path));

            assert_eq!(emitted_target, expected_target);
            assert_eq!(
                Path::new(&emitted_target),
                cargo_resolved,
                "emitted bind target must equal the path Cargo resolves from /app"
            );
        }
    }

    #[test]
    fn noncanonical_runtime_path_uses_matching_container_target_and_omits_absent_observability() {
        let root = unique_test_root("runtime-path-container-target");
        let product = root.join("product");
        let backend = product.join("backend");
        let runtime = product.join("runtime");
        fs::create_dir_all(&backend).expect("create backend root");
        fs::create_dir_all(runtime.join("src")).expect("create runtime layout");
        fs::write(
            backend.join("Cargo.toml"),
            "[package]\nname = \"backend\"\nversion = \"0.1.0\"\n\n[dependencies]\nappfw-runtime = { path = \"../runtime\" }\n",
        )
        .expect("write backend manifest");
        fs::write(
            runtime.join("Cargo.toml"),
            "[package]\nname = \"appfw-runtime\"\nversion = \"0.1.1\"\n",
        )
        .expect("write runtime manifest");
        fs::write(runtime.join("src/lib.rs"), "pub fn runtime() {}\n")
            .expect("write runtime source");
        let product = fs::canonicalize(&product).expect("canonicalize product root");
        let workspace = AppWorkspace {
            app_root: product.clone(),
            framework_root: product.clone(),
            generator_root: product.join("app_gen"),
            config_root: product.join(".appfw/model"),
            templates_root: product.join("app_gen/_templates"),
            report_root: product.join(".appfw/target/appfw"),
        };

        let assets = preflight(&workspace).expect("preflight standalone runtime layout");
        assert_eq!(assets.runtime_bind_source.as_deref(), Some("./runtime"));
        assert_eq!(assets.runtime_bind_target.as_deref(), Some("/runtime"));
        assert!(assets.observability_bind_source.is_none());
        assert!(assets
            .rationale
            .contains("observability services are omitted"));

        let report = topology_report_with_kafka(false);
        let compose = render_compose(&report, &[], &[], &assets);
        assert!(compose.contains("source: ./runtime\n        target: /runtime"));
        assert!(!compose.contains("target: /appfw_runtime"));
        assert!(!compose.contains("  loki:\n"));
        assert!(!compose.contains("  observability-network:\n"));

        fs::remove_dir_all(root).expect("remove runtime target fixture");
    }

    #[test]
    fn partial_observability_assets_fail_closed_before_compose_generation() {
        let root = unique_test_root("partial-observability-assets");
        let product = root.join("product");
        let backend = product.join("backend");
        let runtime = product.join("runtime");
        fs::create_dir_all(&backend).expect("create backend root");
        fs::create_dir_all(runtime.join("src")).expect("create runtime layout");
        fs::create_dir_all(product.join("observability/loki"))
            .expect("create partial observability root");
        fs::write(
            backend.join("Cargo.toml"),
            "[package]\nname = \"backend\"\nversion = \"0.1.0\"\n\n[dependencies]\nappfw-runtime = { path = \"../runtime\" }\n",
        )
        .expect("write backend manifest");
        fs::write(
            runtime.join("Cargo.toml"),
            "[package]\nname = \"appfw-runtime\"\nversion = \"0.1.1\"\n",
        )
        .expect("write runtime manifest");
        fs::write(runtime.join("src/lib.rs"), "pub fn runtime() {}\n")
            .expect("write runtime source");
        fs::write(
            product.join("observability/loki/loki-config.yaml"),
            "auth_enabled: false\n",
        )
        .expect("write one observability config");
        let workspace = AppWorkspace {
            app_root: product.clone(),
            framework_root: product.clone(),
            generator_root: product.join("app_gen"),
            config_root: product.join(".appfw/model"),
            templates_root: product.join("app_gen/_templates"),
            report_root: product.join(".appfw/target/appfw"),
        };

        let error = preflight(&workspace)
            .expect_err("partial observability assets must not produce a compose profile");
        assert!(format!("{error:#}").contains("required path is missing"));

        fs::remove_dir_all(root).expect("remove partial observability fixture");
    }

    #[test]
    fn path_dependency_requires_real_appfw_runtime_package_layout() {
        let root = unique_test_root("runtime-path-layout");
        let runtime = root.join("appfw_runtime");
        fs::create_dir_all(runtime.join("src")).expect("create runtime layout");
        fs::write(
            runtime.join("Cargo.toml"),
            "[package]\nname = \"appfw-runtime\"\nversion = \"0.1.1\"\n",
        )
        .expect("write runtime manifest");
        fs::write(runtime.join("src/lib.rs"), "pub fn runtime() {}\n")
            .expect("write runtime source");

        let validated =
            validate_runtime_path_dependency(&runtime, &root.join("product/backend/Cargo.toml"))
                .expect("accept real runtime package");
        assert_eq!(validated, fs::canonicalize(&runtime).unwrap());

        fs::write(
            runtime.join("Cargo.toml"),
            "[package]\nname = \"lookalike-runtime\"\nversion = \"0.1.1\"\n",
        )
        .expect("write spoofed manifest");
        let spoofed =
            validate_runtime_path_dependency(&runtime, &root.join("product/backend/Cargo.toml"))
                .expect_err("reject wrong package identity");
        assert!(spoofed.to_string().contains("lookalike-runtime"));

        fs::remove_dir_all(root).expect("remove runtime layout fixture");
    }

    #[cfg(unix)]
    #[test]
    fn path_dependency_rejects_symlinked_runtime_root() {
        use std::os::unix::fs::symlink;

        let root = unique_test_root("runtime-path-symlink");
        let target = root.join("real-runtime");
        fs::create_dir_all(target.join("src")).expect("create real runtime");
        fs::write(
            target.join("Cargo.toml"),
            "[package]\nname = \"appfw-runtime\"\nversion = \"0.1.1\"\n",
        )
        .expect("write runtime manifest");
        fs::write(target.join("src/lib.rs"), "pub fn runtime() {}\n")
            .expect("write runtime source");
        let link = root.join("runtime-link");
        symlink(&target, &link).expect("create runtime symlink");

        let error =
            validate_runtime_path_dependency(&link, &root.join("product/backend/Cargo.toml"))
                .expect_err("reject symlinked runtime root");
        assert!(error.to_string().contains("non-symlink directory"));

        fs::remove_file(link).expect("remove runtime symlink");
        fs::remove_dir_all(root).expect("remove runtime symlink fixture");
    }

    #[cfg(unix)]
    #[test]
    fn path_dependency_rejects_symlinked_package_manifest_or_library_source() {
        use std::os::unix::fs::symlink;

        let root = unique_test_root("runtime-layout-symlink");
        let runtime = root.join("appfw_runtime");
        let outside = root.join("outside");
        fs::create_dir_all(runtime.join("src")).expect("create runtime layout");
        fs::create_dir_all(&outside).expect("create outside fixture root");
        fs::write(
            outside.join("Cargo.toml"),
            "[package]\nname = \"appfw-runtime\"\nversion = \"0.1.1\"\n",
        )
        .expect("write outside manifest");
        fs::write(outside.join("lib.rs"), "pub fn runtime() {}\n").expect("write outside source");
        symlink(outside.join("Cargo.toml"), runtime.join("Cargo.toml"))
            .expect("create manifest symlink");
        fs::write(runtime.join("src/lib.rs"), "pub fn runtime() {}\n")
            .expect("write local runtime source");

        let manifest_error =
            validate_runtime_path_dependency(&runtime, &root.join("product/backend/Cargo.toml"))
                .expect_err("reject symlinked runtime manifest");
        assert!(manifest_error
            .to_string()
            .contains("Cargo.toml must be a regular non-symlink file"));

        fs::remove_file(runtime.join("Cargo.toml")).expect("remove manifest symlink");
        fs::write(
            runtime.join("Cargo.toml"),
            "[package]\nname = \"appfw-runtime\"\nversion = \"0.1.1\"\n",
        )
        .expect("write local manifest");
        fs::remove_file(runtime.join("src/lib.rs")).expect("remove local runtime source");
        symlink(outside.join("lib.rs"), runtime.join("src/lib.rs")).expect("create source symlink");

        let source_error =
            validate_runtime_path_dependency(&runtime, &root.join("product/backend/Cargo.toml"))
                .expect_err("reject symlinked runtime source");
        assert!(source_error
            .to_string()
            .contains("src/lib.rs must be a regular non-symlink file"));

        fs::remove_dir_all(root).expect("remove runtime layout symlink fixture");
    }

    fn path_framework_assets() -> ComposeFrameworkAssets {
        ComposeFrameworkAssets {
            manifest_path: PathBuf::from("/work/product/backend/Cargo.toml"),
            declaration_key: "appfw_runtime".to_string(),
            effective_package: "appfw-runtime".to_string(),
            source: FrameworkDependencySourceKind::Path,
            runtime_bind_source: Some("./appfw_runtime".to_string()),
            runtime_bind_target: Some("/appfw_runtime".to_string()),
            observability_bind_source: Some("./observability".to_string()),
            rationale: "path test fixture".to_string(),
        }
    }

    fn portable_framework_assets(source: FrameworkDependencySourceKind) -> ComposeFrameworkAssets {
        assert_ne!(source, FrameworkDependencySourceKind::Path);
        ComposeFrameworkAssets {
            manifest_path: PathBuf::from("/work/fabric-console/backend/Cargo.toml"),
            declaration_key: "appfw_runtime".to_string(),
            effective_package: "appfw-runtime".to_string(),
            source,
            runtime_bind_source: None,
            runtime_bind_target: None,
            observability_bind_source: None,
            rationale: "portable test fixture".to_string(),
        }
    }

    fn unique_test_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "appfw-dev-infra-{label}-{}-{nanos}",
            std::process::id()
        ))
    }

    fn topology_report_with_kafka(enabled: bool) -> AppTopologyReport {
        AppTopologyReport {
            version: 1,
            generated_at: "2026-06-05T00:00:00Z".to_string(),
            manifest_path: ".appfw/manifest.yaml".to_string(),
            manifest_present: true,
            source: "manifest",
            app: app_manifest::TopologyApp {
                name: "crm-sample".to_string(),
                display_name: Some("CRM Sample".to_string()),
                description: None,
            },
            topology: app_manifest::Topology {
                data_sources: vec![],
                schemas: vec![],
                ingress: vec![app_manifest::TopologyIngressModule {
                    name: "crm-events".to_string(),
                    kind: "kafka".to_string(),
                    enabled,
                    schema: Some("crm".to_string()),
                    path: None,
                    topic: Some("crm.events".to_string()),
                    consumer_group: Some("crm-events-worker".to_string()),
                    handler: Some("services.crm_events".to_string()),
                    consumers: vec![],
                }],
            },
        }
    }

    fn data_source(name: &str, data_source_type: DataSourceType, host: Option<&str>) -> DataSource {
        let environments = host
            .map(|db_host| {
                vec![DataSourceEnvironment {
                    name: "compose".to_string(),
                    db_host: db_host.to_string(),
                    db_name: "app".to_string(),
                    db_port: "1433".to_string(),
                    security_profile: "local_dev".to_string(),
                    tls_mode: "disabled".to_string(),
                    auth_mode: None,
                    entra_tenant_id: None,
                    entra_token_scope: None,
                    service_account_name: None,
                    service_account_password: None,
                }]
            })
            .unwrap_or_default();
        DataSource {
            name: name.to_string(),
            data_source_type,
            is_system_schema_host: Some(false),
            description: None,
            environments,
        }
    }
}
