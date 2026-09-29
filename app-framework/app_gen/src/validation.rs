use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use inflector::cases::pascalcase::{is_pascal_case, to_pascal_case};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{
    app_manifest,
    app_workspace::AppWorkspace,
    config_contract,
    sync_descriptor::{parse_sync_descriptor_yaml, validate_sync_descriptor, SyncDescriptorIssue},
    utils::{artifacts, console, files},
};

const DATA_SOURCE_TYPES: &[&str] = &[
    "MsSqlServer",
    "FabricSqlAnalytics",
    "PostgreSQL",
    "MongoDB",
    "Snowflake",
    "Neo4j",
    "ServiceNow",
    "Workday",
    "Icims",
    "Salesforce",
    "Anaplan",
    "OracleFinancials",
];
const SECURITY_PROFILES: &[&str] = &["local_dev", "managed"];
const TLS_MODES: &[&str] = &["disabled", "prefer", "require", "verify_ca", "verify_full"];
const DATA_CLASSIFICATIONS: &[&str] = &[
    "public",
    "internal",
    "confidential",
    "restricted",
    "secret",
    "sensitive",
    "phi",
    "ephi",
];

fn is_external_api_data_source_type(data_source_type: &str) -> bool {
    matches!(
        data_source_type,
        "ServiceNow" | "Workday" | "Icims" | "Salesforce" | "Anaplan" | "OracleFinancials"
    )
}

fn non_crud_data_source_reason(data_source_type: &str) -> Option<&'static str> {
    match data_source_type {
        "Neo4j" => Some(
            "Neo4j is a graph read provider and cannot host generated CRUD entity schemas.",
        ),
        "ServiceNow" | "Workday" | "Icims" | "Salesforce" | "Anaplan" | "OracleFinancials" => Some(
            "External API providers use named SaaS operations and cannot host generated CRUD entity schemas.",
        ),
        _ => None,
    }
}
const REGULATED_DATA_CLASSIFICATIONS: &[&str] = &[
    "confidential",
    "restricted",
    "secret",
    "sensitive",
    "phi",
    "ephi",
];
const DATA_TYPES: &[&str] = &[
    "Uuid",
    "UuidArray",
    "ObjectId",
    "ObjectIdArray",
    "Boolean",
    "String",
    "StringArray",
    "Date",
    "DateTime",
    "Time",
    "Int8",
    "Int8Array",
    "Int16",
    "Int16Array",
    "Int32",
    "Int32Array",
    "Int64",
    "Int64Array",
    "Float32",
    "Float64",
    "Enum",
    "EnumArray",
    "Object",
    "ObjectArray",
    "Json",
    "JsonArray",
    "NavToOne",
    "NavToMany",
    "ManyToMany",
];
const COMPUTED_VALUES: &[&str] = &[
    "Concatenate",
    "Format",
    "Word",
    "Inflection",
    "DateTimeNow",
    "None",
];
const STANDARD_METHODS: &[&str] = &["FindById", "GetAll", "Query", "Create", "Update", "Delete"];
const CUSTOM_METHOD_KINDS: &[&str] = &["Query", "Mutation", "Command"];
const PROVIDER_ROUTINE_KINDS: &[&str] = &["Function", "Procedure"];
const PROVIDER_ROUTINE_RETURNS: &[&str] = &["None", "One", "Many"];
const PROVIDER_ROUTINE_PROVIDERS: &[&str] = &["postgres", "mssql", "snowflake"];
const PROVIDER_ROUTINE_ARG_TYPES: &[&str] = &[
    "String",
    "Option<String>",
    "bool",
    "Option<bool>",
    "i8",
    "Option<i8>",
    "i16",
    "Option<i16>",
    "i32",
    "Option<i32>",
    "i64",
    "Option<i64>",
    "f32",
    "Option<f32>",
    "f64",
    "Option<f64>",
    "JsonValue",
    "Option<JsonValue>",
    "serde_json::Value",
    "Option<serde_json::Value>",
];
const FACETS: &[&str] = &["audited", "concurrency", "soft-deleted"];
const RELATIONSHIP_KINDS: &[&str] = &["OneToOne", "OneToMany", "ManyToMany"];
const RELATIONSHIP_STORAGE_TYPES: &[&str] = &["ForeignKey"];
const TEMPLATE_LINTS: &[TemplateLint] = &[
    TemplateLint {
        pattern: ".unwrap(",
        code: "template_unwrap",
        expected: "fallible generated code with `?` or explicit error handling",
        suggested_fix: "Move the conversion into Rust generator logic, or emit generated code that returns an error instead of panicking.",
    },
    TemplateLint {
        pattern: ".expect(",
        code: "template_expect",
        expected: "fallible generated code with `?` or explicit error handling",
        suggested_fix: "Move the conversion into Rust generator logic, or emit generated code that returns an error instead of panicking.",
    },
    TemplateLint {
        pattern: "panic!",
        code: "template_panic",
        expected: "non-panicking generated code",
        suggested_fix: "Return a typed error from generated code instead of panicking.",
    },
    TemplateLint {
        pattern: "todo!",
        code: "template_todo",
        expected: "generated code that compiles and fails explicitly when extension code is missing",
        suggested_fix: "Emit a clear error result or generate a documented human-owned extension point.",
    },
    TemplateLint {
        pattern: "unimplemented!",
        code: "template_unimplemented",
        expected: "generated code that compiles and fails explicitly when extension code is missing",
        suggested_fix: "Emit a clear error result or generate a documented human-owned extension point.",
    },
    TemplateLint {
        pattern: "set_global",
        code: "template_global_state",
        expected: "local template variables or precomputed Rust context values",
        suggested_fix: "Compute this value in Rust or use a local `{% set ... %}` binding.",
    },
];

struct TemplateLint {
    pattern: &'static str,
    code: &'static str,
    expected: &'static str,
    suggested_fix: &'static str,
}

#[derive(Debug, Serialize)]
struct ValidationReport {
    version: u32,
    valid: bool,
    generated_at: String,
    config_root: String,
    roots: WorkspaceRoots,
    report_path: String,
    summary: ValidationSummary,
    issues: Vec<ValidationIssue>,
}

#[derive(Debug, Serialize)]
struct WorkspaceRoots {
    app_root: String,
    framework_root: String,
    generator_root: String,
    config_root: String,
    templates_root: String,
    report_root: String,
}

impl WorkspaceRoots {
    fn from_workspace(workspace: &AppWorkspace) -> Self {
        Self {
            app_root: workspace.app_root.display().to_string(),
            framework_root: workspace.framework_root.display().to_string(),
            generator_root: workspace.generator_root.display().to_string(),
            config_root: workspace.config_root.display().to_string(),
            templates_root: workspace.templates_root.display().to_string(),
            report_root: workspace.report_root.display().to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
struct ValidationSummary {
    errors: usize,
    warnings: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ValidationIssue {
    severity: &'static str,
    code: &'static str,
    message: String,
    file: String,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    schema_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    entity_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    property_name: Option<String>,
    expected: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    actual: Option<String>,
    suggested_fix: String,
}

#[derive(Debug, Clone)]
struct Location {
    file: PathBuf,
    path: String,
    schema_name: Option<String>,
    entity_name: Option<String>,
    property_name: Option<String>,
}

#[derive(Debug, Clone)]
struct FragmentInfo {
    value: Value,
}

#[derive(Debug, Clone)]
struct DataSourceInfo {
    data_source_type: String,
    classification: Option<String>,
    regulated: bool,
}

#[derive(Debug, Clone)]
struct SchemaInfo {
    name: String,
    data_source_type: Option<String>,
    classification: Option<String>,
    regulated: bool,
    external_read_only: bool,
}

#[derive(Debug, Clone)]
struct EntityInfo {
    schema_name: String,
    name: String,
    is_table: bool,
    is_union: bool,
    base_type: Option<String>,
    loc: Location,
    props: HashMap<String, PropertyInfo>,
    prop_names: Vec<String>,
}

#[derive(Debug, Clone)]
struct PropertyInfo {
    name: String,
    data_type: Option<String>,
    is_key: bool,
    is_required: bool,
    is_read_only: bool,
    is_concurrency_control: bool,
    computed: Option<String>,
    loc: Location,
    foreign_key: Option<RelationRef>,
    nav_by_fk_property: Option<NavRef>,
    many_to_many_property: Option<ManyToManyRef>,
    nested_entity_type: Option<RelationRef>,
    enum_type_name: Option<String>,
}

#[derive(Debug, Clone)]
struct RelationRef {
    schema_name: String,
    type_name: String,
    loc: Location,
}

#[derive(Debug, Clone)]
struct NavRef {
    schema_name: String,
    type_name: String,
    prop_name: String,
    loc: Location,
}

#[derive(Debug, Clone)]
struct ManyToManyRef {
    junction_table: String,
    junction_schema: Option<String>,
    local_key: String,
    foreign_key: String,
    target_schema: String,
    target_type: String,
    loc: Location,
}

#[derive(Debug, Clone)]
struct RelationshipInfo {
    name: String,
    kind: String,
    loc: Location,
    left: Option<RelationshipEndpointInfo>,
    right: Option<RelationshipEndpointInfo>,
    one: Option<RelationshipEndpointInfo>,
    many: Option<RelationshipEndpointInfo>,
    storage: Option<RelationshipStorageInfo>,
    junction: Option<RelationshipJunctionInfo>,
}

#[derive(Debug, Clone)]
struct RelationshipEndpointInfo {
    schema_name: String,
    entity_name: String,
    field_name: String,
    loc: Location,
}

#[derive(Debug, Clone)]
struct RelationshipStorageInfo {
    storage_type: String,
    owner_schema: String,
    owner: String,
    field: String,
    loc: Location,
}

#[derive(Debug, Clone)]
struct RelationshipJunctionInfo {
    schema_name: String,
    entity_name: String,
    left_key: String,
    right_key: String,
}

#[derive(Debug, Clone)]
struct JunctionInfo {
    columns: BTreeSet<String>,
}

struct Validator {
    workspace: AppWorkspace,
    report_path: PathBuf,
    issues: Vec<ValidationIssue>,
    fragments: HashMap<String, FragmentInfo>,
    data_sources: HashMap<String, DataSourceInfo>,
    schemas: HashMap<String, SchemaInfo>,
    schema_dirs: Vec<(String, PathBuf)>,
    enum_types: HashMap<String, Location>,
    enum_values: HashMap<String, HashSet<String>>,
    entity_types: HashMap<String, EntityInfo>,
    entity_ids: HashMap<String, Location>,
    relationships: Vec<RelationshipInfo>,
    relationship_names: HashMap<String, Location>,
    junctions: HashMap<String, JunctionInfo>,
    seed_keys: HashMap<String, Location>,
}

pub fn run(workspace: &AppWorkspace) -> Result<()> {
    console::stage("config validation");

    let mut validator = Validator::new(workspace);
    validator.validate();
    let report = validator.report();
    validator.write_report(&report)?;
    config_contract::emit(workspace)?;

    if !report.valid {
        console::warn(format!(
            "config validation failed: {} error(s), {} warning(s)",
            report.summary.errors, report.summary.warnings
        ));
        bail!(
            "config validation failed with {} error(s); see {}",
            report.summary.errors,
            validator.report_path.display()
        );
    }

    console::step(format!(
        "validation passed: {} warning(s)",
        report.summary.warnings
    ));
    console::done("config validation");
    Ok(())
}

impl Validator {
    fn new(workspace: &AppWorkspace) -> Self {
        Self {
            workspace: workspace.clone(),
            report_path: workspace.target_file("validation.json"),
            issues: vec![],
            fragments: HashMap::new(),
            data_sources: HashMap::new(),
            schemas: HashMap::new(),
            schema_dirs: vec![],
            enum_types: HashMap::new(),
            enum_values: HashMap::new(),
            entity_types: HashMap::new(),
            entity_ids: HashMap::new(),
            relationships: vec![],
            relationship_names: HashMap::new(),
            junctions: HashMap::new(),
            seed_keys: HashMap::new(),
        }
    }

    fn validate(&mut self) {
        self.validate_fragments();
        self.validate_facets();
        self.validate_templates();
        self.validate_data_sources();
        self.validate_schemas();
        self.validate_sync_descriptors();
        self.validate_app_manifest();
        self.validate_relationships();
        self.validate_provider_features();
        self.validate_seed_configs();
        self.validate_test_configs();
    }

    fn report(&self) -> ValidationReport {
        let errors = self
            .issues
            .iter()
            .filter(|issue| issue.severity == "error")
            .count();
        let warnings = self
            .issues
            .iter()
            .filter(|issue| issue.severity == "warning")
            .count();

        ValidationReport {
            version: 1,
            valid: errors == 0,
            generated_at: chrono::offset::Utc::now().to_rfc3339(),
            config_root: self.workspace.config_root.display().to_string(),
            roots: WorkspaceRoots::from_workspace(&self.workspace),
            report_path: self.report_path.display().to_string(),
            summary: ValidationSummary { errors, warnings },
            issues: self.issues.clone(),
        }
    }

    fn validate_templates(&mut self) {
        let templates_dir = self.workspace.templates_root.clone();
        let files = self.template_files(&templates_dir);

        for file in files {
            let loc = Location::root(file.clone());
            let contents = match fs::read_to_string(&file) {
                Ok(contents) => contents,
                Err(err) => {
                    self.error(
                        "template_read_failed",
                        format!("Could not read template: {err}."),
                        &loc,
                        "readable .j2 template",
                        Some(err.to_string()),
                        "Fix template permissions or restore the expected template file.",
                    );
                    continue;
                }
            };

            for (idx, line) in contents.lines().enumerate() {
                let line_no = idx + 1;
                self.lint_template_line(&loc.line(line_no), line);
            }
        }
    }

    fn lint_template_line(&mut self, loc: &Location, line: &str) {
        for lint in TEMPLATE_LINTS {
            if line.contains(lint.pattern) {
                self.error(
                    lint.code,
                    format!("Template contains brittle construct `{}`.", lint.pattern),
                    loc,
                    lint.expected,
                    Some(line.trim().to_string()),
                    lint.suggested_fix,
                );
            }
        }
    }

    fn write_report(&self, report: &ValidationReport) -> Result<()> {
        let json = serde_json::to_string_pretty(report)?;
        artifacts::safe_write(&self.report_path, format!("{json}\n"))
            .with_context(|| format!("could not write {}", self.report_path.display()))?;
        console::write(&self.report_path);
        Ok(())
    }

    fn validate_fragments(&mut self) {
        let fragments_dir = self.workspace.config_root.join("_fragments");
        let files = self.yaml_files(&fragments_dir, true, false);

        for file in files {
            let Some(fragment_name) = file.file_stem().and_then(|name| name.to_str()) else {
                continue;
            };
            let loc = Location::root(file.clone());
            let Some(value) = self.read_yaml_value(&file, &loc) else {
                continue;
            };

            if self.fragments.contains_key(fragment_name) {
                self.error(
                    "duplicate_fragment",
                    format!("Duplicate fragment name `{fragment_name}`."),
                    &loc,
                    "unique fragment file stems under _config/_fragments",
                    Some(fragment_name.to_string()),
                    "Rename one fragment file so every fragment reference is unambiguous.",
                );
            }

            let _ = self.validate_property_shape(&value, &loc, PropertyShapeMode::Fragment);
            self.fragments
                .insert(fragment_name.to_string(), FragmentInfo { value });
        }
    }

    fn validate_facets(&mut self) {
        let facets_dir = self.workspace.config_root.join("_facets");
        let files = self.yaml_files(&facets_dir, false, false);

        for file in files {
            let loc = Location::root(file.clone());
            let Some(value) = self.read_yaml_value(&file, &loc) else {
                continue;
            };

            if let Some(items) = value.as_array() {
                for (idx, item) in items.iter().enumerate() {
                    self.validate_entity_patch(item, &loc.index(idx));
                }
            } else if let Some(obj) = value.as_object() {
                if obj.contains_key("props") {
                    self.validate_entity_patch(&value, &loc);
                } else {
                    let _ = self.validate_property_shape(
                        &value,
                        &loc,
                        PropertyShapeMode::FacetProperty,
                    );
                }
            } else {
                self.error(
                    "invalid_facet_shape",
                    "Facet YAML must be an object or array of objects.".to_string(),
                    &loc,
                    "object property patch, entity patch, or array of entity patches",
                    Some(value_kind(&value).to_string()),
                    "Use a mapping with property fields, or a list of mappings with `props`.",
                );
            }
        }
    }

    fn validate_data_sources(&mut self) {
        let file = self.workspace.config_root.join("data_sources/_res.yaml");
        let loc = Location::root(file.clone());
        let Some(value) = self.read_yaml_value(&file, &loc) else {
            return;
        };
        let Some(items) = self.expect_array(&value, &loc, "data source array") else {
            return;
        };

        let mut system_hosts = vec![];
        for (idx, item) in items.iter().enumerate() {
            let item_loc = loc.index(idx);
            let Some(obj) = self.expect_object(item, &item_loc, "data source object") else {
                continue;
            };

            self.validate_known_fields(
                obj,
                &[
                    "name",
                    "data_source_type",
                    "is_system_schema_host",
                    "description",
                    "meta",
                    "environments",
                ],
                &item_loc,
                "data source fields",
            );

            let name = self.string_field(obj, "name", &item_loc, true);
            let data_source_type =
                self.enum_field(obj, "data_source_type", &item_loc, DATA_SOURCE_TYPES, true);
            let classification = self.validate_data_classification_meta(
                obj.get("meta"),
                &item_loc.field("meta"),
                "data source",
            );

            if let Some(name) = &name {
                if self.data_sources.contains_key(name) {
                    self.error(
                        "duplicate_data_source",
                        format!("Duplicate data source `{name}`."),
                        &item_loc.field("name"),
                        "unique data source names",
                        Some(name.clone()),
                        "Rename one data source or update schemas to point at the intended one.",
                    );
                }
            }

            if let Some(is_system_host) = obj.get("is_system_schema_host") {
                if !is_system_host.is_boolean() {
                    self.error(
                        "invalid_field_type",
                        "`is_system_schema_host` must be a boolean.".to_string(),
                        &item_loc.field("is_system_schema_host"),
                        "boolean",
                        Some(value_kind(is_system_host).to_string()),
                        "Use `true` on exactly one data source, or omit the field.",
                    );
                } else if is_system_host.as_bool() == Some(true) {
                    if let Some(name) = &name {
                        if data_source_type.as_deref() == Some("FabricSqlAnalytics") {
                            self.error(
                                "invalid_system_schema_host",
                                "FabricSqlAnalytics data sources cannot host the generated system schema.".to_string(),
                                &item_loc.field("is_system_schema_host"),
                                "a writable database provider for the system schema host",
                                Some(name.clone()),
                                "Use PostgreSQL, MongoDB, MS SQL Server, or Snowflake for generated system metadata; keep FabricSqlAnalytics as a read-only analytics data source.",
                            );
                        }
                        if data_source_type.as_deref().is_some_and(|provider| {
                            provider == "Neo4j" || is_external_api_data_source_type(provider)
                        }) {
                            self.error(
                                "invalid_system_schema_host",
                                "Non-CRUD providers cannot host the generated system schema."
                                    .to_string(),
                                &item_loc.field("is_system_schema_host"),
                                "a writable database provider for the system schema host",
                                Some(name.clone()),
                                "Use PostgreSQL, MongoDB, MS SQL Server, or Snowflake for generated system metadata; keep graph and external API providers behind named operation surfaces.",
                            );
                        }
                        system_hosts.push(name.clone());
                    }
                }
            }

            let has_managed_environment = self.validate_environments(
                obj.get("environments"),
                &item_loc,
                data_source_type.as_deref(),
            );
            let regulated =
                has_managed_environment || is_regulated_classification(classification.as_deref());

            if regulated {
                self.require_effective_data_classification(
                    "data source",
                    name.as_deref().unwrap_or("<unknown>"),
                    &item_loc.field("meta"),
                    classification.as_deref(),
                    "Add `meta.classification: confidential` or a stricter value such as `phi`/`ephi` for regulated data sources.",
                );
            }

            if let (Some(name), Some(data_source_type)) = (&name, &data_source_type) {
                self.data_sources.insert(
                    name.clone(),
                    DataSourceInfo {
                        data_source_type: data_source_type.clone(),
                        classification,
                        regulated,
                    },
                );
            }
        }

        if system_hosts.len() > 1 {
            self.error(
                "duplicate_system_schema_host",
                "More than one data source is marked as the system schema host.".to_string(),
                &loc,
                "zero or one data source with `is_system_schema_host: true`",
                Some(system_hosts.join(", ")),
                "Keep `is_system_schema_host: true` on only one legacy persisted system metadata source, or omit it for code-only system metadata.",
            );
        }
    }

    fn validate_environments(
        &mut self,
        value: Option<&Value>,
        parent_loc: &Location,
        data_source_type: Option<&str>,
    ) -> bool {
        let env_loc = parent_loc.field("environments");
        let Some(value) = value else {
            self.error(
                "missing_required_field",
                "Data source is missing `environments`.".to_string(),
                &env_loc,
                "non-empty array of environment objects",
                None,
                "Add at least a `local` environment with db_host, db_name, and db_port.",
            );
            return false;
        };
        let Some(items) = self.expect_array(value, &env_loc, "environment array") else {
            return false;
        };
        if items.is_empty() {
            self.error(
                "empty_environments",
                "Data source has no environments.".to_string(),
                &env_loc,
                "non-empty array of environment objects",
                Some("[]".to_string()),
                "Add a `local` environment entry.",
            );
        }

        let mut names = HashSet::new();
        let mut has_managed_environment = false;
        for (idx, item) in items.iter().enumerate() {
            let item_loc = env_loc.index(idx);
            let Some(obj) = self.expect_object(item, &item_loc, "environment object") else {
                continue;
            };
            self.validate_known_fields(
                obj,
                &[
                    "name",
                    "db_host",
                    "db_name",
                    "db_port",
                    "security_profile",
                    "tls_mode",
                    "auth_mode",
                    "entra_tenant_id",
                    "entra_token_scope",
                    "service_account_name",
                    "service_account_password",
                ],
                &item_loc,
                "environment fields",
            );
            let name = self.string_field(obj, "name", &item_loc, true);
            let db_host = self.string_field(obj, "db_host", &item_loc, true);
            self.string_field(obj, "db_name", &item_loc, true);
            self.string_or_number_field(obj, "db_port", &item_loc, true);
            if data_source_type == Some("MongoDB") {
                if let Some(db_host) = db_host.as_deref() {
                    self.validate_mongodb_connection_host(db_host, &item_loc.field("db_host"));
                }
            }
            let security_profile =
                self.enum_field(obj, "security_profile", &item_loc, SECURITY_PROFILES, true);
            let tls_mode = self.enum_field(obj, "tls_mode", &item_loc, TLS_MODES, true);
            let auth_mode = self.enum_field(
                obj,
                "auth_mode",
                &item_loc,
                appfw_mssql_auth::ALL_MODE_IDS,
                false,
            );
            if let (Some(provider), Some(mode)) = (data_source_type, auth_mode.as_deref()) {
                if matches!(provider, "MsSqlServer" | "FabricSqlAnalytics") {
                    if let Err(err) =
                        appfw_mssql_auth::validate_model_mode_for_provider(provider, mode)
                    {
                        self.error(
                            err.code,
                            err.message,
                            &item_loc.field("auth_mode"),
                            err.expected.unwrap_or("allowed auth_mode"),
                            Some(mode.to_string()),
                            err.remediation.unwrap_or(
                                "Choose an auth_mode allowed for this data_source_type.",
                            ),
                        );
                    }
                }
            }
            if let Some(mode) = auth_mode.as_deref() {
                if let Some(host) = db_host.as_deref() {
                    if let Err(err) = appfw_mssql_auth::validate_db_host(mode, host) {
                        self.error(
                            err.code,
                            err.message,
                            &item_loc.field("db_host"),
                            err.expected.unwrap_or("fully-qualified domain name"),
                            Some(host.to_string()),
                            err.remediation.unwrap_or(
                                "Set db_host to a fully-qualified domain name for this auth mode.",
                            ),
                        );
                    }
                }
            }
            self.string_field(obj, "entra_tenant_id", &item_loc, false);
            let entra_token_scope = self.string_field(obj, "entra_token_scope", &item_loc, false);
            if data_source_type == Some("FabricSqlAnalytics") {
                let mode = auth_mode
                    .as_deref()
                    .unwrap_or(appfw_mssql_auth::DEFAULT_FABRIC_AUTH_MODE);
                if let Err(err) =
                    appfw_mssql_auth::validate_token_scope(mode, entra_token_scope.as_deref())
                {
                    self.error(
                        err.code,
                        err.message,
                        &item_loc.field("entra_token_scope"),
                        err.expected.unwrap_or("OAuth v2 `.default` scope"),
                        entra_token_scope.clone(),
                        err.remediation.unwrap_or(
                            "Use `https://database.windows.net/.default` unless Fabric SQL analytics requires a provider-specific scope.",
                        ),
                    );
                }
            }
            self.string_field(obj, "service_account_name", &item_loc, false);
            self.string_field(obj, "service_account_password", &item_loc, false);

            if let (Some(name), Some(security_profile), Some(tls_mode)) =
                (&name, &security_profile, &tls_mode)
            {
                let is_local_env = matches!(name.as_str(), "local" | "compose");
                if security_profile == "managed" && !is_local_env {
                    has_managed_environment = true;
                }
                self.validate_connection_security(name, security_profile, tls_mode, &item_loc);
            }

            if let Some(name) = name {
                if !names.insert(name.clone()) {
                    self.error(
                        "duplicate_environment",
                        format!("Duplicate data source environment `{name}`."),
                        &item_loc.field("name"),
                        "unique environment names per data source",
                        Some(name),
                        "Rename or remove the duplicate environment.",
                    );
                }
            }
        }
        has_managed_environment
    }

    fn validate_connection_security(
        &mut self,
        env_name: &str,
        security_profile: &str,
        tls_mode: &str,
        loc: &Location,
    ) {
        let is_local_env = matches!(env_name, "local" | "compose");

        if security_profile == "local_dev" && !is_local_env {
            self.error(
                "invalid_connection_security",
                "`local_dev` security_profile is only allowed for `local` or `compose` environments."
                    .to_string(),
                &loc.field("security_profile"),
                "`managed` for non-local environments",
                Some(format!(
                    "name: {env_name}, security_profile: {security_profile}"
                )),
                "Use `managed` with a TLS mode for shared, staging, and production environments.",
            );
        }

        if tls_mode == "disabled" && !(security_profile == "local_dev" && is_local_env) {
            self.error(
                "invalid_connection_security",
                "`disabled` tls_mode is only allowed for local development environments."
                    .to_string(),
                &loc.field("tls_mode"),
                "`require`, `verify_ca`, or `verify_full`",
                Some(format!("name: {env_name}, tls_mode: {tls_mode}")),
                "Use TLS for non-local database connections.",
            );
        }

        if tls_mode == "prefer" && security_profile != "local_dev" {
            self.error(
                "invalid_connection_security",
                "`prefer` tls_mode is only allowed for local development environments.".to_string(),
                &loc.field("tls_mode"),
                "`require`, `verify_ca`, or `verify_full`",
                Some(format!(
                    "security_profile: {security_profile}, tls_mode: {tls_mode}"
                )),
                "Use fail-closed TLS modes for managed data sources.",
            );
        }
    }

    fn validate_mongodb_connection_host(&mut self, db_host: &str, loc: &Location) {
        let trimmed = db_host.trim();
        let is_mongo_uri =
            trimmed.starts_with("mongodb://") || trimmed.starts_with("mongodb+srv://");

        if trimmed.contains("://") && !is_mongo_uri {
            self.error(
                "invalid_mongodb_connection_uri",
                "MongoDB data source db_host uses an unsupported URI scheme.".to_string(),
                loc,
                "`mongodb://`, `mongodb+srv://`, or a plain host name",
                Some(trimmed.to_string()),
                "Use MongoDB connection strings only for MongoDB seed-list, Atlas SRV, or replica-set configurations.",
            );
            return;
        }

        if !is_mongo_uri {
            return;
        }

        let Some((scheme, remainder)) = trimmed.split_once("://") else {
            return;
        };
        let authority = remainder
            .split(['/', '?'])
            .next()
            .unwrap_or_default()
            .trim();

        if authority.is_empty() {
            self.error(
                "invalid_mongodb_connection_uri",
                "MongoDB connection URI is missing a host.".to_string(),
                loc,
                "MongoDB host authority",
                Some(trimmed.to_string()),
                "Use `mongodb://host1:27017,host2:27017/?replicaSet=rs0` or `mongodb+srv://cluster.example.net`.",
            );
            return;
        }

        if authority.contains('@') {
            self.error(
                "invalid_mongodb_connection_uri",
                "MongoDB connection URIs in model config must not embed credentials.".to_string(),
                loc,
                "credential-free MongoDB URI",
                Some(trimmed.to_string()),
                "Keep credentials in secret-backed `service_account_name` and `service_account_password` fields.",
            );
        }

        if scheme == "mongodb+srv" {
            if authority.contains(',') {
                self.error(
                    "invalid_mongodb_connection_uri",
                    "MongoDB SRV connection URIs must name one DNS seed record.".to_string(),
                    loc,
                    "single SRV host",
                    Some(trimmed.to_string()),
                    "Use `mongodb+srv://cluster.example.net` for Atlas, or `mongodb://host1,host2` for a seed list.",
                );
            }
            if authority.contains(':') {
                self.error(
                    "invalid_mongodb_connection_uri",
                    "MongoDB SRV connection URIs must not specify an explicit port.".to_string(),
                    loc,
                    "SRV host without a port",
                    Some(trimmed.to_string()),
                    "Remove the port from `mongodb+srv://...`; SRV records carry the target ports.",
                );
            }
        }
    }

    fn validate_schemas(&mut self) {
        let schemas_dir = self.workspace.schemas_root();
        let schema_dirs = self.schema_dirs(&schemas_dir);

        for schema_dir in schema_dirs {
            let dir_name = schema_dir
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("<unknown>")
                .to_string();
            let schema_file = schema_dir.join("_res.yaml");
            let loc = Location::root(schema_file.clone()).with_schema(dir_name.clone());
            let Some(value) = self.read_yaml_value(&schema_file, &loc) else {
                continue;
            };
            let Some(obj) = self.expect_object(&value, &loc, "schema object") else {
                continue;
            };
            self.validate_known_fields(
                obj,
                &["id", "name", "description", "data_source_name", "meta"],
                &loc,
                "schema fields",
            );

            self.uuid_field(obj, "id", &loc, true);
            let name = self
                .string_field(obj, "name", &loc, true)
                .unwrap_or_else(|| dir_name.clone());
            self.string_field(obj, "description", &loc, true);
            let data_source_name = self
                .string_field(obj, "data_source_name", &loc, true)
                .unwrap_or_default();
            let schema_classification = self.validate_data_classification_meta(
                obj.get("meta"),
                &loc.field("meta"),
                "schema",
            );
            self.validate_projection_meta(obj.get("meta"), &loc.field("meta"), "schema");
            let external_read_only = schema_meta_external_read_only(obj.get("meta"));

            if name != dir_name {
                self.error(
                    "schema_name_mismatch",
                    format!("Schema name `{name}` does not match directory `{dir_name}`."),
                    &loc.field("name"),
                    "schema name matching its .appfw/model/schemas/<name> directory",
                    Some(name.clone()),
                    format!("Rename the directory to `{name}` or set `name: {dir_name}`."),
                );
            }

            if self.schemas.contains_key(&name) {
                self.error(
                    "duplicate_schema",
                    format!("Duplicate schema `{name}`."),
                    &loc.field("name"),
                    "unique schema names",
                    Some(name.clone()),
                    "Rename one schema directory and update references.",
                );
            }

            let data_source_info = if data_source_name.is_empty() {
                None
            } else if let Some(data_source_info) = self.data_sources.get(&data_source_name) {
                Some(data_source_info.clone())
            } else {
                self.error(
                    "missing_data_source",
                    format!("Schema `{name}` references unknown data source `{data_source_name}`."),
                    &loc.field("data_source_name"),
                    "name from .appfw/model/data_sources/_res.yaml",
                    Some(data_source_name.clone()),
                    "Add the data source or update `data_source_name` to an existing data source.",
                );
                None
            };
            if let Some(provider) = data_source_info
                .as_ref()
                .map(|info| info.data_source_type.as_str())
            {
                if let Some(reason) = non_crud_data_source_reason(provider) {
                    let code = if provider == "Neo4j" {
                        "schema_uses_graph_read_provider"
                    } else {
                        "schema_uses_external_api_provider"
                    };
                    self.error(
                        code,
                        format!(
                            "Schema `{name}` uses {provider} data source `{data_source_name}`, but {reason}"
                        ),
                        &loc.field("data_source_name"),
                        "CRUD-capable data source such as PostgreSQL, MongoDB, MS SQL Server, or Snowflake",
                        Some(data_source_name.clone()),
                        "Keep entity schemas on a primary data provider or projection schema; access graph and SaaS providers through named operations, sync workers, or product services.",
                    );
                }
            }
            let data_source_type = data_source_info
                .as_ref()
                .map(|info| info.data_source_type.clone());
            let inherited_classification = data_source_info
                .as_ref()
                .and_then(|info| info.classification.clone());
            let schema_effective_classification =
                schema_classification.clone().or(inherited_classification);
            let schema_regulated = data_source_info
                .as_ref()
                .map(|info| info.regulated)
                .unwrap_or(false)
                || is_regulated_classification(schema_classification.as_deref());

            if schema_regulated {
                self.require_effective_data_classification(
                    "schema",
                    &name,
                    &loc.field("meta"),
                    schema_effective_classification.as_deref(),
                    "Add `meta.classification` to the schema or classify its data source.",
                );
            }

            self.schemas.insert(
                name.clone(),
                SchemaInfo {
                    name: name.clone(),
                    data_source_type,
                    classification: schema_effective_classification,
                    regulated: schema_regulated,
                    external_read_only,
                },
            );
            self.schema_dirs.push((name.clone(), schema_dir.clone()));

            self.validate_enum_type_files(&schema_dir, &name);
            self.validate_entity_type_files(&schema_dir, &name);
            self.validate_relationship_files(&schema_dir, &name);
        }
    }

    fn validate_sync_descriptors(&mut self) {
        let sync_dir = files::get_sync_descriptors_dir(&self.workspace);
        let files = self.yaml_files(&sync_dir, false, false);
        let mut descriptor_names: HashMap<String, PathBuf> = HashMap::new();

        for file in files {
            let loc = Location::root(file.clone());
            let contents = match fs::read_to_string(&file) {
                Ok(contents) => contents,
                Err(err) => {
                    self.error(
                        "sync_descriptor_read_failed",
                        format!("Could not read sync descriptor: {err}."),
                        &loc,
                        "readable .appfw/model/sync/*.yaml file",
                        Some(err.to_string()),
                        "Fix file permissions or restore the expected sync descriptor file.",
                    );
                    continue;
                }
            };

            let descriptor = match parse_sync_descriptor_yaml(&contents) {
                Ok(descriptor) => descriptor,
                Err(err) => {
                    self.error(
                        "invalid_sync_descriptor_yaml",
                        format!("Sync descriptor could not be parsed: {err}."),
                        &loc,
                        "YAML matching the sync descriptor contract",
                        Some(err.to_string()),
                        "Fix the descriptor syntax and fields under `.appfw/model/sync`.",
                    );
                    continue;
                }
            };

            for issue in validate_sync_descriptor(&descriptor) {
                self.sync_descriptor_issue(&file, issue);
            }

            let file_stem = file.file_stem().and_then(|stem| stem.to_str());
            if let (Some(file_stem), Some(name)) = (file_stem, descriptor.name.as_deref()) {
                let name = name.trim();
                if !name.is_empty() && name != file_stem {
                    self.error(
                        "sync_descriptor_name_mismatch",
                        format!(
                            "Sync descriptor name `{name}` does not match file stem `{file_stem}`."
                        ),
                        &loc.field("name"),
                        "descriptor name matching .appfw/model/sync/<name>.yaml",
                        Some(name.to_string()),
                        format!("Rename the file to `{name}.yaml` or set `name: {file_stem}`."),
                    );
                }

                if !name.is_empty() {
                    if let Some(first_file) =
                        descriptor_names.insert(name.to_string(), file.clone())
                    {
                        self.error(
                            "duplicate_sync_descriptor",
                            format!("Duplicate sync descriptor `{name}`."),
                            &loc.field("name"),
                            "unique sync descriptor names",
                            Some(format!("{} and {}", first_file.display(), file.display())),
                            "Rename one descriptor so every sync worker contract is unambiguous.",
                        );
                    }
                }
            }

            if let Some(target_schema) = descriptor
                .target_schema
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                let mut target_schema_allows_entity_checks = false;
                match self.schemas.get(target_schema) {
                    Some(schema) if schema.external_read_only => {
                        self.error(
                            "sync_descriptor_target_schema_external_read_only",
                            format!(
                                "Sync descriptor target schema `{target_schema}` is external read-only."
                            ),
                            &loc.field("target_schema"),
                            "app-owned projection schema",
                            Some(target_schema.to_string()),
                            "Point `target_schema` at the local app-owned projection written by the sync worker.",
                        );
                    }
                    Some(_) => {
                        target_schema_allows_entity_checks = true;
                    }
                    None => {
                        self.error(
                            "sync_descriptor_unknown_target_schema",
                            format!(
                                "Sync descriptor references unknown target schema `{target_schema}`."
                            ),
                            &loc.field("target_schema"),
                            "schema name from .appfw/model/schemas",
                            Some(target_schema.to_string()),
                            "Add the projection schema or update `target_schema` to an existing schema.",
                        );
                    }
                }

                if target_schema_allows_entity_checks {
                    for (idx, object) in descriptor.objects.iter().enumerate() {
                        let Some(target_entity) = object
                            .target
                            .as_deref()
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                        else {
                            continue;
                        };
                        if !self
                            .entity_types
                            .contains_key(&entity_key(target_schema, target_entity))
                        {
                            self.error(
                                "sync_descriptor_unknown_target_entity",
                                format!(
                                    "Sync descriptor target entity `{target_entity}` does not exist in schema `{target_schema}`."
                                ),
                                &loc.field("objects").index(idx).field("target"),
                                "entity name from the target projection schema",
                                Some(target_entity.to_string()),
                                "Add the projection entity or update the sync object target.",
                            );
                        }
                    }
                }
            }

            if let (Some(source_schema), Some(target_schema)) = (
                descriptor.source_schema.as_deref(),
                descriptor.target_schema.as_deref(),
            ) {
                let source_schema = source_schema.trim();
                let target_schema = target_schema.trim();
                if !source_schema.is_empty() && source_schema == target_schema {
                    self.error(
                        "sync_descriptor_source_matches_target",
                        "Sync descriptor source and target schemas must be different.".to_string(),
                        &loc.field("target_schema"),
                        "different source_schema and target_schema values",
                        Some(target_schema.to_string()),
                        "Use a SaaS source projection and a separate app-owned target projection schema.",
                    );
                }
            }
        }
    }

    fn sync_descriptor_issue(&mut self, file: &Path, issue: SyncDescriptorIssue) {
        let SyncDescriptorIssue {
            severity,
            code,
            message,
            path,
            expected,
            actual,
            suggested_fix,
        } = issue;
        let loc = Location {
            file: file.to_path_buf(),
            path,
            schema_name: None,
            entity_name: None,
            property_name: None,
        };

        match severity {
            crate::sync_descriptor::SyncIssueSeverity::Error => {
                self.error(code, message, &loc, expected, actual, suggested_fix);
            }
        }
    }

    fn validate_app_manifest(&mut self) {
        match app_manifest::build(&self.workspace) {
            Ok((report, issues)) => {
                if let Err(err) = app_manifest::write_report(&self.workspace, &report) {
                    let loc = Location::root(app_manifest::report_path(&self.workspace));
                    self.error(
                        "app_topology_report_write_failed",
                        format!("Could not write app topology report: {err}."),
                        &loc,
                        "writable .appfw/target/appfw/app_topology.json",
                        Some(err.to_string()),
                        "Fix filesystem permissions or remove the blocking file.",
                    );
                } else {
                    console::write(&app_manifest::report_path(&self.workspace));
                }

                self.validate_ingress_runtime_config_residue(&report);

                for issue in issues {
                    let app_manifest::TopologyIssue {
                        severity,
                        code,
                        message,
                        file,
                        path,
                        expected,
                        actual,
                        suggested_fix,
                    } = issue;
                    let loc = Location {
                        file: PathBuf::from(file),
                        path,
                        schema_name: None,
                        entity_name: None,
                        property_name: None,
                    };
                    if severity == "warning" {
                        self.warn(code, message, &loc, expected, actual, suggested_fix);
                    } else {
                        self.error(code, message, &loc, expected, actual, suggested_fix);
                    }
                }
            }
            Err(err) => {
                let loc = Location::root(app_manifest::manifest_path(&self.workspace));
                self.error(
                    "app_manifest_validation_failed",
                    format!("Could not validate app manifest topology: {err}."),
                    &loc,
                    "readable app topology inputs",
                    Some(err.to_string()),
                    "Fix .appfw/manifest.yaml, data source config, or schema config.",
                );
            }
        }
    }

    fn validate_ingress_runtime_config_residue(
        &mut self,
        report: &app_manifest::AppTopologyReport,
    ) {
        let kafka_config = self
            .workspace
            .app_root
            .join("backend/config/generated/ingress/kafka.yaml");
        let enabled_kafka_ingress = report
            .topology
            .ingress
            .iter()
            .filter(|module| module.kind == "kafka" && module.enabled)
            .count();
        let loc = Location::root(kafka_config.clone());

        if kafka_config.exists() && enabled_kafka_ingress == 0 {
            self.error(
                "stale_kafka_ingress_config",
                "Kafka runtime config exists but app topology has no enabled Kafka ingress."
                    .to_string(),
                &loc,
                "no backend/config/generated/ingress/kafka.yaml unless topology.ingress enables Kafka",
                Some("stale Kafka runtime config file".to_string()),
                "Remove backend/config/generated/ingress/kafka.yaml or enable the matching kind: kafka topology entry.",
            );
        } else if !kafka_config.exists() && enabled_kafka_ingress > 0 {
            self.warn(
                "missing_kafka_ingress_config",
                "App topology enables Kafka ingress but runtime config has not been scaffolded."
                    .to_string(),
                &loc,
                "backend/config/generated/ingress/kafka.yaml for enabled Kafka ingress",
                None,
                "Run scripts/appfw generate to create the generated Kafka runtime config, then review service actor, tenant, auth, and operation bindings.",
            );
        }
    }

    fn validate_enum_type_files(&mut self, schema_dir: &Path, schema_name: &str) {
        let enum_dir = schema_dir.join("gql_enum_types");
        let files = self.yaml_files(&enum_dir, false, true);

        for file in files {
            let loc = Location::root(file.clone()).with_schema(schema_name.to_string());
            let Some(value) = self.read_yaml_value(&file, &loc) else {
                continue;
            };
            let Some(items) = self.expect_array(&value, &loc, "GraphQL enum type array") else {
                continue;
            };

            for (idx, item) in items.iter().enumerate() {
                let item_loc = loc.index(idx);
                let Some(obj) = self.expect_object(item, &item_loc, "GraphQL enum type object")
                else {
                    continue;
                };
                self.validate_known_fields(obj, &["name", "items"], &item_loc, "enum type fields");
                let Some(name) = self.string_field(obj, "name", &item_loc, true) else {
                    continue;
                };
                let key = enum_key(schema_name, &name);
                if self.enum_types.contains_key(&key) {
                    self.error(
                        "duplicate_enum_type",
                        format!("Duplicate enum type `{schema_name}.{name}`."),
                        &item_loc.field("name"),
                        "unique enum type names per schema",
                        Some(name.clone()),
                        "Rename one enum type or merge the duplicate definitions.",
                    );
                }
                self.enum_types.insert(key, item_loc.clone());
                let values = self.validate_enum_items(obj.get("items"), &item_loc, &name);
                self.enum_values
                    .insert(enum_key(schema_name, &name), values);
            }
        }
    }

    fn validate_enum_items(
        &mut self,
        value: Option<&Value>,
        parent_loc: &Location,
        enum_name: &str,
    ) -> HashSet<String> {
        let mut values = HashSet::new();
        let items_loc = parent_loc.field("items");
        let Some(value) = value else {
            self.error(
                "missing_required_field",
                format!("Enum `{enum_name}` is missing `items`."),
                &items_loc,
                "non-empty array of enum item objects",
                None,
                "Add `items` with at least one `{ value: ... }` entry.",
            );
            return values;
        };
        let Some(items) = self.expect_array(value, &items_loc, "enum items array") else {
            return values;
        };
        if items.is_empty() {
            self.error(
                "empty_enum_items",
                format!("Enum `{enum_name}` has no items."),
                &items_loc,
                "non-empty enum items array",
                Some("[]".to_string()),
                "Add at least one enum item.",
            );
        }

        for (idx, item) in items.iter().enumerate() {
            let item_loc = items_loc.index(idx);
            let Some(obj) = self.expect_object(item, &item_loc, "enum item object") else {
                continue;
            };
            self.validate_known_fields(obj, &["value", "caption"], &item_loc, "enum item fields");
            let value = self.string_field(obj, "value", &item_loc, true);
            self.string_field(obj, "caption", &item_loc, false);
            if let Some(value) = value {
                if !values.insert(value.clone()) {
                    self.error(
                        "duplicate_enum_item",
                        format!("Enum `{enum_name}` has duplicate item `{value}`."),
                        &item_loc.field("value"),
                        "unique item values per enum",
                        Some(value),
                        "Remove or rename the duplicate enum item.",
                    );
                }
            }
        }
        values
    }

    fn validate_entity_type_files(&mut self, schema_dir: &Path, schema_name: &str) {
        let entity_dir = schema_dir.join("entity_types");
        let files = self.yaml_files(&entity_dir, false, true);

        for file in files {
            let loc = Location::root(file.clone()).with_schema(schema_name.to_string());
            let Some(value) = self.read_yaml_value(&file, &loc) else {
                continue;
            };
            let Some(items) = self.expect_array(&value, &loc, "entity type array") else {
                continue;
            };

            for (idx, item) in items.iter().enumerate() {
                self.validate_entity_type_item(schema_name, item, &loc.index(idx));
            }
        }
    }

    fn validate_relationship_files(&mut self, schema_dir: &Path, schema_name: &str) {
        let relationship_dir = schema_dir.join("relationships");
        let files = self.yaml_files(&relationship_dir, false, true);

        for file in files {
            let loc = Location::root(file.clone()).with_schema(schema_name.to_string());
            let Some(value) = self.read_yaml_value(&file, &loc) else {
                continue;
            };
            let Some(items) = self.expect_array(&value, &loc, "relationship array") else {
                continue;
            };

            for (idx, item) in items.iter().enumerate() {
                self.validate_relationship_item(schema_name, item, &loc.index(idx));
            }
        }
    }

    fn validate_relationship_item(&mut self, schema_name: &str, item: &Value, loc: &Location) {
        let Some(obj) = self.expect_object(item, loc, "relationship object") else {
            return;
        };
        self.validate_known_fields(
            obj,
            &[
                "name", "kind", "left", "right", "one", "many", "storage", "junction",
            ],
            loc,
            "relationship fields",
        );

        let Some(name) = self.string_field(obj, "name", loc, true) else {
            return;
        };
        let kind = self
            .enum_field(obj, "kind", loc, RELATIONSHIP_KINDS, true)
            .unwrap_or_default();
        let key = format!("{schema_name}.{name}");
        if let Some(first_loc) = self.relationship_names.get(&key).cloned() {
            self.error(
                "duplicate_relationship",
                format!("Duplicate relationship `{key}`."),
                &loc.field("name"),
                "unique relationship names per schema",
                Some(name.clone()),
                format!(
                    "Rename this relationship or merge it with the first definition at {} {}.",
                    first_loc.file.display(),
                    first_loc.path
                ),
            );
        } else {
            self.relationship_names.insert(key, loc.clone());
        }

        let relationship = RelationshipInfo {
            name,
            kind,
            loc: loc.clone(),
            left: self.parse_relationship_endpoint(
                obj.get("left"),
                &loc.field("left"),
                schema_name,
            ),
            right: self.parse_relationship_endpoint(
                obj.get("right"),
                &loc.field("right"),
                schema_name,
            ),
            one: self.parse_relationship_endpoint(obj.get("one"), &loc.field("one"), schema_name),
            many: self.parse_relationship_endpoint(
                obj.get("many"),
                &loc.field("many"),
                schema_name,
            ),
            storage: self.parse_relationship_storage(
                obj.get("storage"),
                &loc.field("storage"),
                schema_name,
            ),
            junction: self.parse_relationship_junction(
                obj.get("junction"),
                &loc.field("junction"),
                schema_name,
            ),
        };
        self.relationships.push(relationship);
    }

    fn validate_entity_type_item(&mut self, schema_name: &str, item: &Value, loc: &Location) {
        let Some(obj) = self.expect_object(item, loc, "entity type object") else {
            return;
        };
        self.validate_known_fields(
            obj,
            &[
                "id",
                "name",
                "caption",
                "pascal_1",
                "pascal_n",
                "snake_1",
                "snake_n",
                "caption_1",
                "caption_n",
                "is_table",
                "is_union",
                "base_type",
                "facets",
                "indexes",
                "constraints",
                "meta",
                "execution",
                "standard_methods",
                "custom_methods",
                "props",
            ],
            loc,
            "entity type fields",
        );

        let raw_name = self.string_field(obj, "name", loc, true);
        let Some(raw_name) = raw_name else {
            return;
        };
        let name = self.effective_entity_name(obj, &raw_name);
        let entity_loc = loc.with_entity(name.clone());
        let (schema_classification, schema_regulated, schema_external_read_only) = self
            .schemas
            .get(schema_name)
            .map(|schema| {
                (
                    schema.classification.clone(),
                    schema.regulated,
                    schema.external_read_only,
                )
            })
            .unwrap_or((None, false, false));
        let entity_classification = self.validate_data_classification_meta(
            obj.get("meta"),
            &entity_loc.field("meta"),
            "entity",
        );
        self.validate_projection_meta(obj.get("meta"), &entity_loc.field("meta"), "entity");
        let entity_effective_classification =
            entity_classification.clone().or(schema_classification);
        let entity_regulated =
            schema_regulated || is_regulated_classification(entity_classification.as_deref());

        if entity_regulated {
            self.require_effective_data_classification(
                "entity",
                &format!("{schema_name}.{name}"),
                &entity_loc.field("meta"),
                entity_effective_classification.as_deref(),
                "Add `meta.classification` to the entity, its schema, or its data source.",
            );
        }

        let id = self.uuid_field(obj, "id", &entity_loc, true);
        if let Some(id) = id {
            if let Some(first_loc) = self.entity_ids.get(&id).cloned() {
                self.error(
                    "duplicate_entity_id",
                    format!("Entity id `{id}` is used more than once."),
                    &entity_loc.field("id"),
                    "globally unique entity UUIDs",
                    Some(id.clone()),
                    format!(
                        "Generate a new UUID for this entity. First use is at {} {}.",
                        first_loc.file.display(),
                        first_loc.path
                    ),
                );
            } else {
                self.entity_ids.insert(id, entity_loc.clone());
            }
        }

        for field in [
            "caption",
            "pascal_1",
            "pascal_n",
            "snake_1",
            "snake_n",
            "caption_1",
            "caption_n",
            "base_type",
        ] {
            self.string_field(obj, field, &entity_loc, false);
        }

        let is_table = self
            .bool_field(obj, "is_table", &entity_loc, false)
            .unwrap_or(false);
        let is_union = self
            .bool_field(obj, "is_union", &entity_loc, false)
            .unwrap_or(false);
        let base_type = obj
            .get("base_type")
            .and_then(Value::as_str)
            .map(str::to_string);
        let facets = self.string_array_field(obj, "facets", &entity_loc, false);
        let indexes = self.string_array_field(obj, "indexes", &entity_loc, false);

        self.validate_facets_list(&facets, &entity_loc);
        self.validate_data_access_execution(obj.get("execution"), &entity_loc);
        self.validate_standard_methods(
            obj.get("standard_methods"),
            &entity_loc,
            is_table,
            schema_external_read_only,
        );
        self.validate_custom_methods(
            obj.get("custom_methods"),
            &entity_loc,
            Some(schema_name),
            schema_external_read_only,
        );

        let mut props = HashMap::new();
        let mut prop_names = vec![];
        self.validate_props(
            obj.get("props"),
            &entity_loc,
            schema_name,
            &name,
            &mut props,
            &mut prop_names,
            entity_effective_classification,
            entity_regulated,
        );

        if facets.iter().any(|facet| facet == "audited") {
            if !is_table {
                self.error(
                    "audited_facet_on_non_table",
                    format!("Audited entity `{schema_name}.{name}` is not a table entity."),
                    &entity_loc.field("facets"),
                    "`audited` only on entities with `is_table: true`",
                    Some("audited".to_string()),
                    "Set `is_table: true` or remove the `audited` facet.",
                );
            }
            if !props.values().any(|prop| prop.is_key) {
                self.error(
                    "audited_entity_missing_key",
                    format!("Audited entity `{schema_name}.{name}` has no key property."),
                    &entity_loc.field("props"),
                    "one property with `is_key: true`",
                    None,
                    "Add a stable primary key property before enabling the `audited` facet.",
                );
            }
        }

        self.validate_indexes(&indexes, &props, &entity_loc);

        let key = entity_key(schema_name, &name);
        if self.entity_types.contains_key(&key) {
            self.error(
                "duplicate_entity_type",
                format!("Duplicate entity type `{schema_name}.{name}`."),
                &entity_loc.field("name"),
                "unique entity names per schema",
                Some(name.clone()),
                "Rename one entity or merge the duplicate definitions.",
            );
        }

        self.entity_types.insert(
            key,
            EntityInfo {
                schema_name: schema_name.to_string(),
                name,
                is_table,
                is_union,
                base_type,
                loc: entity_loc,
                props,
                prop_names,
            },
        );
    }

    fn validate_entity_patch(&mut self, value: &Value, loc: &Location) {
        let Some(obj) = self.expect_object(value, loc, "entity patch object") else {
            return;
        };
        self.validate_known_fields(
            obj,
            &[
                "id",
                "name",
                "caption",
                "pascal_1",
                "pascal_n",
                "snake_1",
                "snake_n",
                "caption_1",
                "caption_n",
                "is_table",
                "is_union",
                "base_type",
                "facets",
                "indexes",
                "constraints",
                "meta",
                "standard_methods",
                "custom_methods",
                "props",
            ],
            loc,
            "entity patch fields",
        );

        self.string_field(obj, "name", loc, false);
        self.bool_field(obj, "is_table", loc, false);
        self.bool_field(obj, "is_union", loc, false);
        self.string_field(obj, "base_type", loc, false);
        self.string_array_field(obj, "facets", loc, false);
        self.string_array_field(obj, "indexes", loc, false);
        self.validate_data_classification_meta(obj.get("meta"), &loc.field("meta"), "entity patch");
        self.validate_standard_methods(obj.get("standard_methods"), loc, true, false);
        self.validate_custom_methods(obj.get("custom_methods"), loc, None, false);

        if let Some(props) = obj.get("props") {
            let props_loc = loc.field("props");
            if let Some(items) = self.expect_array(props, &props_loc, "property array") {
                for (idx, prop) in items.iter().enumerate() {
                    let prop_loc = props_loc.index(idx);
                    let _ = self.validate_property_shape(
                        prop,
                        &prop_loc,
                        PropertyShapeMode::EntityProperty,
                    );
                }
            }
        }
    }

    fn validate_props(
        &mut self,
        value: Option<&Value>,
        entity_loc: &Location,
        schema_name: &str,
        entity_name: &str,
        props: &mut HashMap<String, PropertyInfo>,
        prop_names: &mut Vec<String>,
        inherited_classification: Option<String>,
        regulated: bool,
    ) {
        let props_loc = entity_loc.field("props");
        let Some(value) = value else {
            self.error(
                "missing_required_field",
                format!("Entity `{schema_name}.{entity_name}` is missing `props`."),
                &props_loc,
                "array of property objects",
                None,
                "Add `props: []` if the entity intentionally has no properties.",
            );
            return;
        };
        let Some(items) = self.expect_array(value, &props_loc, "property array") else {
            return;
        };
        if items.is_empty() {
            self.warn(
                "empty_props",
                format!("Entity `{schema_name}.{entity_name}` has no properties."),
                &props_loc,
                "one or more properties",
                Some("[]".to_string()),
                "Add at least a key property unless this type is intentionally empty.",
            );
        }

        for (idx, item) in items.iter().enumerate() {
            let base_loc = props_loc.index(idx);
            let Some(obj) = self.expect_object(item, &base_loc, "property object") else {
                continue;
            };
            let prop_name = self.string_field(obj, "name", &base_loc, true);
            let Some(prop_name) = prop_name else {
                let _ = self.validate_property_shape(
                    item,
                    &base_loc,
                    PropertyShapeMode::EntityProperty,
                );
                continue;
            };
            let prop_loc = base_loc.with_property(prop_name.clone());
            let prop_classification =
                self.validate_property_shape(item, &prop_loc, PropertyShapeMode::EntityProperty);

            if props.contains_key(&prop_name) {
                self.error(
                    "duplicate_property",
                    format!("Entity `{schema_name}.{entity_name}` has duplicate property `{prop_name}`."),
                    &prop_loc.field("name"),
                    "unique property names per entity",
                    Some(prop_name.clone()),
                    "Rename one property or remove the duplicate.",
                );
            }

            let fragment_name = obj
                .get("fragment")
                .and_then(Value::as_str)
                .map(str::to_string);
            let fragment_value = fragment_name
                .as_ref()
                .and_then(|name| self.fragments.get(name))
                .map(|fragment| fragment.value.clone());
            let data_type = fragment_value
                .as_ref()
                .and_then(|fragment| fragment.get("data_type"))
                .and_then(Value::as_str)
                .or_else(|| obj.get("data_type").and_then(Value::as_str))
                .map(str::to_string);
            let is_key = bool_from_json(fragment_value.as_ref(), "is_key")
                || obj.get("is_key").and_then(Value::as_bool).unwrap_or(false);
            let is_required = bool_from_json(fragment_value.as_ref(), "is_required")
                || obj
                    .get("is_required")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
            let is_read_only = bool_from_json(fragment_value.as_ref(), "is_read_only")
                || obj
                    .get("is_read_only")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
            let is_concurrency_control =
                bool_from_json(fragment_value.as_ref(), "is_concurrency_control")
                    || obj
                        .get("is_concurrency_control")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
            let computed = string_from_json(fragment_value.as_ref(), "computed").or_else(|| {
                obj.get("computed")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            });
            let fragment_classification = fragment_value
                .as_ref()
                .and_then(|fragment| read_property_data_classification(fragment.get("meta")));
            let effective_classification = prop_classification
                .clone()
                .or(fragment_classification)
                .or_else(|| inherited_classification.clone());
            let property_regulated =
                regulated || is_regulated_classification(prop_classification.as_deref());
            if property_regulated {
                self.require_effective_data_classification(
                    "property",
                    &format!("{schema_name}.{entity_name}.{prop_name}"),
                    &prop_loc.field("meta"),
                    effective_classification.as_deref(),
                    "Add `meta.audit.classification` to the property, or classify its entity, schema, or data source.",
                );
            }

            let foreign_key = self.parse_relation_ref(
                obj.get("foreign_key"),
                &prop_loc.field("foreign_key"),
                schema_name,
            );
            let nav_by_fk_property = self.parse_nav_ref(
                obj.get("nav_by_fk_property"),
                &prop_loc.field("nav_by_fk_property"),
                schema_name,
                entity_name,
            );
            let many_to_many_property = self.parse_many_to_many_ref(
                obj.get("many_to_many_property"),
                &prop_loc.field("many_to_many_property"),
            );
            let nested_entity_type = self.parse_relation_ref(
                obj.get("nested_entity_type"),
                &prop_loc.field("nested_entity_type"),
                schema_name,
            );
            let enum_type_name = obj
                .get("enum_type_name")
                .and_then(Value::as_str)
                .map(str::to_string);

            props.insert(
                prop_name.clone(),
                PropertyInfo {
                    name: prop_name.clone(),
                    data_type,
                    is_key,
                    is_required,
                    is_read_only,
                    is_concurrency_control,
                    computed,
                    loc: prop_loc,
                    foreign_key,
                    nav_by_fk_property,
                    many_to_many_property,
                    nested_entity_type,
                    enum_type_name,
                },
            );
            prop_names.push(prop_name);
        }
    }

    fn validate_property_shape(
        &mut self,
        value: &Value,
        loc: &Location,
        mode: PropertyShapeMode,
    ) -> Option<String> {
        let Some(obj) = self.expect_object(value, loc, "property object") else {
            return None;
        };
        self.validate_known_fields(
            obj,
            &[
                "id",
                "name",
                "caption",
                "is_key",
                "is_caption",
                "is_required",
                "is_read_only",
                "is_concurrency_control",
                "data_type",
                "computed",
                "default_value",
                "foreign_key",
                "nav_by_fk_property",
                "many_to_many_property",
                "nested_entity_type",
                "enum_type_name",
                "meta",
                "fragment",
                "type_variant",
            ],
            loc,
            "property fields",
        );

        if mode.requires_name() {
            self.string_field(obj, "name", loc, true);
        } else {
            self.string_field(obj, "name", loc, false);
        }
        self.string_field(obj, "id", loc, false);
        self.string_field(obj, "caption", loc, false);
        for field in [
            "is_key",
            "is_caption",
            "is_required",
            "is_read_only",
            "is_concurrency_control",
            "type_variant",
        ] {
            self.bool_field(obj, field, loc, false);
        }
        self.enum_field(obj, "data_type", loc, DATA_TYPES, false);
        self.enum_field(obj, "computed", loc, COMPUTED_VALUES, false);
        self.string_field(obj, "enum_type_name", loc, false);
        let classification = self.validate_property_meta(obj.get("meta"), &loc.field("meta"));

        if let Some(fragment) = self.string_field(obj, "fragment", loc, false) {
            if !self.fragments.contains_key(&fragment) {
                self.error(
                    "missing_fragment",
                    format!("Property references missing fragment `{fragment}`."),
                    &loc.field("fragment"),
                    "fragment file stem from _config/_fragments/*.yaml",
                    Some(fragment.clone()),
                    format!(
                        "Create `_config/_fragments/{fragment}.yaml` or change `fragment` to one of: {}.",
                        self.fragment_suggestions()
                    ),
                );
            }
        }

        self.validate_relation_shape(
            obj.get("foreign_key"),
            &loc.field("foreign_key"),
            "foreign_key",
        );
        self.validate_nav_shape(
            obj.get("nav_by_fk_property"),
            &loc.field("nav_by_fk_property"),
        );
        self.validate_many_to_many_shape(
            obj.get("many_to_many_property"),
            &loc.field("many_to_many_property"),
        );
        self.validate_relation_shape(
            obj.get("nested_entity_type"),
            &loc.field("nested_entity_type"),
            "nested_entity_type",
        );
        classification
    }

    fn validate_property_meta(&mut self, value: Option<&Value>, loc: &Location) -> Option<String> {
        let Some(value) = value else {
            return None;
        };
        if value.is_null() {
            return None;
        }
        let Some(obj) = self.expect_object(value, loc, "property meta object") else {
            return None;
        };
        let classification =
            self.validate_classification_keys(obj, loc, "property metadata classification");
        self.validate_optional_meta_string(
            obj,
            loc,
            "source_field",
            "invalid_source_field_metadata",
            "property metadata `source_field`",
        );

        let Some(audit) = obj.get("audit") else {
            return classification;
        };
        if audit.is_null() {
            return classification;
        }
        let audit_loc = loc.field("audit");
        let Some(audit_obj) = self.expect_object(audit, &audit_loc, "property audit metadata")
        else {
            return classification;
        };
        self.validate_known_fields(
            audit_obj,
            &["redact", "omit", "classification"],
            &audit_loc,
            "property audit metadata fields",
        );
        self.bool_field(audit_obj, "redact", &audit_loc, false);
        self.bool_field(audit_obj, "omit", &audit_loc, false);
        let audit_classification = audit_obj.get("classification").and_then(|value| {
            self.validate_data_classification_value(
                value,
                &audit_loc.field("classification"),
                "property audit classification",
            )
        });
        if let (Some(meta_classification), Some(audit_classification)) =
            (&classification, &audit_classification)
        {
            if meta_classification != audit_classification {
                self.error(
                    "conflicting_data_classification",
                    "Property metadata declares conflicting classification values.".to_string(),
                    &audit_loc.field("classification"),
                    "one consistent classification value",
                    Some(format!(
                        "meta: {meta_classification}, audit: {audit_classification}"
                    )),
                    "Keep only one classification value, or make `meta.classification` and `meta.audit.classification` match.",
                );
            }
        }
        classification.or(audit_classification)
    }

    fn validate_projection_meta(&mut self, value: Option<&Value>, loc: &Location, label: &str) {
        let Some(value) = value else {
            return;
        };
        if value.is_null() {
            return;
        }
        let Some(obj) = value.as_object() else {
            return;
        };
        self.validate_optional_meta_string(
            obj,
            loc,
            "projection_of",
            "invalid_projection_metadata",
            &format!("{label} metadata `projection_of`"),
        );
    }

    fn validate_optional_meta_string(
        &mut self,
        obj: &Map<String, Value>,
        loc: &Location,
        field: &str,
        code: &'static str,
        label: &str,
    ) {
        let Some(value) = obj.get(field) else {
            return;
        };
        match value.as_str().map(str::trim) {
            Some(value) if !value.is_empty() => {}
            Some(value) => self.error(
                code,
                format!("{label} must be a non-empty string."),
                &loc.field(field),
                "a non-empty string",
                Some(value.to_string()),
                format!("Set `meta.{field}` to a stable source name or remove the field."),
            ),
            None => self.error(
                code,
                format!("{label} must be a string."),
                &loc.field(field),
                "a string",
                Some(value.to_string()),
                format!("Set `meta.{field}` to a stable source name or remove the field."),
            ),
        }
    }

    fn validate_data_classification_meta(
        &mut self,
        value: Option<&Value>,
        loc: &Location,
        label: &str,
    ) -> Option<String> {
        let Some(value) = value else {
            return None;
        };
        if value.is_null() {
            return None;
        }
        let Some(obj) = self.expect_object(value, loc, &format!("{label} metadata object")) else {
            return None;
        };
        self.validate_classification_keys(obj, loc, label)
    }

    fn validate_classification_keys(
        &mut self,
        obj: &Map<String, Value>,
        loc: &Location,
        label: &str,
    ) -> Option<String> {
        let mut classification = None;
        for field in ["classification", "data_classification"] {
            let Some(value) = obj.get(field) else {
                continue;
            };
            let next = self.validate_data_classification_value(
                value,
                &loc.field(field),
                &format!("{label} `{field}`"),
            );
            self.merge_data_classification(&mut classification, next, &loc.field(field), label);
        }
        classification
    }

    fn validate_data_classification_value(
        &mut self,
        value: &Value,
        loc: &Location,
        label: &str,
    ) -> Option<String> {
        let Some(classification) = value.as_str() else {
            self.error(
                "invalid_field_type",
                format!("{label} must be a string."),
                loc,
                allowed_values(DATA_CLASSIFICATIONS),
                Some(value_kind(value).to_string()),
                "Set the classification to a supported string such as `confidential` or `phi`.",
            );
            return None;
        };
        if !DATA_CLASSIFICATIONS.contains(&classification) {
            self.error(
                "invalid_data_classification",
                format!("Unsupported data classification `{classification}`."),
                loc,
                allowed_values(DATA_CLASSIFICATIONS),
                Some(classification.to_string()),
                "Use a known classification value; do not leave regulated data as unknown.",
            );
            return None;
        }
        Some(classification.to_string())
    }

    fn merge_data_classification(
        &mut self,
        classification: &mut Option<String>,
        next: Option<String>,
        loc: &Location,
        label: &str,
    ) {
        let Some(next) = next else {
            return;
        };
        if let Some(existing) = classification {
            if existing != &next {
                self.error(
                    "conflicting_data_classification",
                    format!("{label} declares conflicting classification values."),
                    loc,
                    "one consistent classification value",
                    Some(format!("{existing}, {next}")),
                    "Keep a single classification value or make all aliases match.",
                );
            }
        } else {
            *classification = Some(next);
        }
    }

    fn require_effective_data_classification(
        &mut self,
        scope: &str,
        name: &str,
        loc: &Location,
        classification: Option<&str>,
        suggested_fix: impl Into<String>,
    ) {
        if classification.is_some() {
            return;
        }
        self.error(
            "missing_data_classification",
            format!("Regulated {scope} `{name}` has no effective data classification."),
            loc,
            "classification metadata on the data source, schema, entity, or property",
            None,
            suggested_fix,
        );
    }

    fn validate_relation_shape(&mut self, value: Option<&Value>, loc: &Location, label: &str) {
        let Some(value) = value else {
            return;
        };
        if value.is_null() {
            return;
        }
        let Some(obj) = self.expect_object(value, loc, label) else {
            return;
        };
        self.validate_known_fields(obj, &["schema_name", "type_name"], loc, label);
        self.string_field(obj, "schema_name", loc, false);
        self.string_field(obj, "type_name", loc, true);
    }

    fn validate_nav_shape(&mut self, value: Option<&Value>, loc: &Location) {
        let Some(value) = value else {
            return;
        };
        if value.is_null() {
            return;
        }
        let Some(obj) = self.expect_object(value, loc, "nav_by_fk_property") else {
            return;
        };
        self.validate_known_fields(
            obj,
            &[
                "schema_name",
                "type_name",
                "prop_name",
                "filter",
                "resolved",
            ],
            loc,
            "nav_by_fk_property fields",
        );
        self.string_field(obj, "schema_name", loc, false);
        self.string_field(obj, "type_name", loc, false);
        self.string_field(obj, "prop_name", loc, true);
    }

    fn validate_many_to_many_shape(&mut self, value: Option<&Value>, loc: &Location) {
        let Some(value) = value else {
            return;
        };
        if value.is_null() {
            return;
        }
        let Some(obj) = self.expect_object(value, loc, "many_to_many_property") else {
            return;
        };
        self.validate_known_fields(
            obj,
            &[
                "junction_table",
                "junction_schema",
                "local_key",
                "foreign_key",
                "target_schema",
                "target_type",
            ],
            loc,
            "many_to_many_property fields",
        );
        for field in [
            "junction_table",
            "local_key",
            "foreign_key",
            "target_schema",
            "target_type",
        ] {
            self.string_field(obj, field, loc, true);
        }
        self.string_field(obj, "junction_schema", loc, false);
    }

    fn validate_facets_list(&mut self, facets: &[String], entity_loc: &Location) {
        let mut seen = HashSet::new();
        for facet in facets {
            if !FACETS.contains(&facet.as_str()) {
                self.error(
                    "invalid_facet",
                    format!("Unknown facet `{facet}`."),
                    &entity_loc.field("facets"),
                    allowed_values(FACETS),
                    Some(facet.clone()),
                    "Use one of the supported template facets.",
                );
            }
            if !seen.insert(facet.clone()) {
                self.warn(
                    "duplicate_facet",
                    format!("Facet `{facet}` is listed more than once."),
                    &entity_loc.field("facets"),
                    "unique facets per entity",
                    Some(facet.clone()),
                    "Remove the duplicate facet entry.",
                );
            }
        }
    }

    fn validate_data_access_execution(&mut self, value: Option<&Value>, entity_loc: &Location) {
        let Some(value) = value else {
            return;
        };
        if value.is_null() {
            return;
        }
        let loc = entity_loc.field("execution");
        let Some(obj) = self.expect_object(value, &loc, "data access execution object") else {
            return;
        };
        self.validate_known_fields(
            obj,
            &["prepared_statements"],
            &loc,
            "data access execution fields",
        );
        self.bool_field(obj, "prepared_statements", &loc, false);
    }

    fn validate_standard_methods(
        &mut self,
        value: Option<&Value>,
        entity_loc: &Location,
        is_table: bool,
        external_read_only: bool,
    ) {
        let Some(value) = value else {
            return;
        };
        if value.is_null() {
            return;
        }
        let loc = entity_loc.field("standard_methods");
        let Some(items) = self.expect_array(value, &loc, "standard method array") else {
            return;
        };
        if !is_table && !items.is_empty() {
            self.warn(
                "standard_methods_on_non_table",
                "Standard methods are ignored for non-table entity types.".to_string(),
                &loc,
                "`standard_methods` only on entities with `is_table: true`",
                Some(value_preview(value)),
                "Set `is_table: true`, remove `standard_methods`, or model this behavior as a custom method.",
            );
        }

        let mut seen = HashSet::new();
        for (idx, item) in items.iter().enumerate() {
            let item_loc = loc.index(idx);
            let Some(method) = item.as_str() else {
                self.error(
                    "invalid_standard_method_shape",
                    "Standard method entries must be strings.".to_string(),
                    &item_loc,
                    allowed_values(STANDARD_METHODS),
                    Some(value_kind(item).to_string()),
                    "Use method names such as `FindById` or `Query`.",
                );
                continue;
            };
            if !STANDARD_METHODS.contains(&method) {
                self.error(
                    "invalid_standard_method",
                    format!("Unknown standard method `{method}`."),
                    &item_loc,
                    allowed_values(STANDARD_METHODS),
                    Some(method.to_string()),
                    "Use one of the supported standard method names.",
                );
                continue;
            }
            if external_read_only && matches!(method, "Create" | "Update" | "Delete") {
                self.error(
                    "external_read_only_standard_method",
                    format!(
                        "External read-only entity `{}` cannot expose standard method `{method}`.",
                        entity_loc.entity_name.as_deref().unwrap_or("<unknown>")
                    ),
                    &item_loc,
                    "read-only standard methods such as `FindById`, `GetAll`, or `Query`",
                    Some(method.to_string()),
                    "Remove generated mutation methods from entities backed by authoritative external storage.",
                );
            }
            if !seen.insert(method.to_string()) {
                self.warn(
                    "duplicate_standard_method",
                    format!("Standard method `{method}` is listed more than once."),
                    &item_loc,
                    "unique standard methods per entity",
                    Some(method.to_string()),
                    "Remove the duplicate method entry.",
                );
            }
        }
    }

    fn validate_custom_methods(
        &mut self,
        value: Option<&Value>,
        entity_loc: &Location,
        schema_name: Option<&str>,
        external_read_only: bool,
    ) {
        let Some(value) = value else {
            return;
        };
        if value.is_null() {
            return;
        }
        let loc = entity_loc.field("custom_methods");
        let Some(items) = self.expect_array(value, &loc, "custom method array") else {
            return;
        };

        let mut seen = HashSet::new();
        for (idx, item) in items.iter().enumerate() {
            let item_loc = loc.index(idx);
            let Some(obj) = self.expect_object(item, &item_loc, "custom method object") else {
                continue;
            };
            self.validate_known_fields(
                obj,
                &[
                    "name",
                    "kind",
                    "args",
                    "return_type",
                    "mcp_enabled",
                    "provider_routine",
                ],
                &item_loc,
                "custom method fields",
            );
            let name = self.string_field(obj, "name", &item_loc, true);
            let kind = self.enum_field(obj, "kind", &item_loc, CUSTOM_METHOD_KINDS, true);
            if external_read_only && matches!(kind.as_deref(), Some("Mutation" | "Command")) {
                self.error(
                    "external_read_only_custom_method",
                    format!(
                        "External read-only entity `{}` cannot expose `{}` custom method `{}`.",
                        entity_loc.entity_name.as_deref().unwrap_or("<unknown>"),
                        kind.as_deref().unwrap_or("<unknown>"),
                        name.as_deref().unwrap_or("<unknown>")
                    ),
                    &item_loc.field("kind"),
                    "`Query` custom methods only",
                    kind.clone(),
                    "Move write behavior to the authoritative source owner's workflow, or model a separate app-owned schema for writes.",
                );
            }
            self.string_field(obj, "return_type", &item_loc, true);
            self.bool_field(obj, "mcp_enabled", &item_loc, false);
            if let Some(name) = name {
                if !seen.insert(name.clone()) {
                    self.error(
                        "duplicate_custom_method",
                        format!("Custom method `{name}` is listed more than once."),
                        &item_loc.field("name"),
                        "unique custom method names per entity",
                        Some(name),
                        "Rename one method or remove the duplicate.",
                    );
                }
            }
            self.validate_custom_method_args(obj.get("args"), &item_loc);
            self.validate_provider_routine(
                obj.get("provider_routine"),
                &item_loc,
                kind,
                schema_name,
            );
            if obj.get("provider_routine").is_some() {
                self.validate_provider_routine_arg_types(obj.get("args"), &item_loc);
            }
        }
    }

    fn validate_custom_method_args(&mut self, value: Option<&Value>, method_loc: &Location) {
        let loc = method_loc.field("args");
        let Some(value) = value else {
            self.error(
                "missing_required_field",
                "Custom method is missing `args`.".to_string(),
                &loc,
                "array of argument objects, or []",
                None,
                "Add `args: []` when the method takes no arguments.",
            );
            return;
        };
        let Some(items) = self.expect_array(value, &loc, "custom method args array") else {
            return;
        };
        let mut seen = HashSet::new();
        for (idx, item) in items.iter().enumerate() {
            let item_loc = loc.index(idx);
            let Some(obj) = self.expect_object(item, &item_loc, "custom method arg object") else {
                continue;
            };
            self.validate_known_fields(obj, &["name", "arg_type"], &item_loc, "argument fields");
            let name = self.string_field(obj, "name", &item_loc, true);
            self.string_field(obj, "arg_type", &item_loc, true);
            if let Some(name) = name {
                if !seen.insert(name.clone()) {
                    self.error(
                        "duplicate_custom_method_arg",
                        format!("Custom method argument `{name}` is listed more than once."),
                        &item_loc.field("name"),
                        "unique argument names per method",
                        Some(name),
                        "Rename or remove the duplicate argument.",
                    );
                }
            }
        }
    }

    fn validate_provider_routine(
        &mut self,
        value: Option<&Value>,
        method_loc: &Location,
        method_kind: Option<String>,
        schema_name: Option<&str>,
    ) {
        let Some(value) = value else {
            return;
        };
        if value.is_null() {
            return;
        }
        let loc = method_loc.field("provider_routine");
        let Some(obj) = self.expect_object(value, &loc, "provider routine object") else {
            return;
        };
        self.validate_known_fields(
            obj,
            &[
                "kind",
                "returns",
                "data_source",
                "schema",
                "name",
                "routines",
            ],
            &loc,
            "provider routine fields",
        );
        let routine_kind = self.enum_field(obj, "kind", &loc, PROVIDER_ROUTINE_KINDS, true);
        self.enum_field(obj, "returns", &loc, PROVIDER_ROUTINE_RETURNS, true);
        if matches!(method_kind.as_deref(), Some("Query"))
            && matches!(routine_kind.as_deref(), Some("Procedure"))
        {
            self.error(
                "invalid_provider_routine_kind",
                "Query custom methods must bind to provider functions, not procedures."
                    .to_string(),
                &loc.field("kind"),
                "Function for Query methods",
                routine_kind.clone(),
                "Use `kind: Function` for query methods, or change the custom method kind to Mutation/Command.",
            );
        }
        if let Some(data_source) = self.string_field(obj, "data_source", &loc, false) {
            if !self.data_sources.contains_key(&data_source) {
                self.error(
                    "unknown_data_source",
                    format!("Provider routine references unknown data source `{data_source}`."),
                    &loc.field("data_source"),
                    "data source declared in .appfw/model/data_sources/_res.yaml",
                    Some(data_source),
                    "Set `data_source` to an existing data source or omit it to use the entity schema data source.",
                );
            }
        } else if let Some(schema_name) = schema_name {
            if !self.schemas.contains_key(schema_name) {
                self.error(
                    "unknown_schema",
                    format!("Provider routine belongs to unknown schema `{schema_name}`."),
                    &loc,
                    "schema declared in .appfw/model/schemas/<schema>/_res.yaml",
                    Some(schema_name.to_string()),
                    "Declare the schema before validating routine-backed custom methods.",
                );
            }
        }

        let default_schema = self.string_field(obj, "schema", &loc, false);
        let default_name = self.string_field(obj, "name", &loc, false);
        if let Some(schema) = default_schema {
            self.validate_provider_routine_identifier(
                &schema,
                &loc.field("schema"),
                "routine schema",
            );
        }
        if let Some(name) = default_name.as_ref() {
            self.validate_provider_routine_identifier(name, &loc.field("name"), "routine name");
        }

        let mut declared = 0;
        let routines_loc = loc.field("routines");
        if let Some(routines) = obj.get("routines") {
            let Some(routines_obj) =
                self.expect_object(routines, &routines_loc, "provider routine names object")
            else {
                return;
            };
            self.validate_known_fields(
                routines_obj,
                PROVIDER_ROUTINE_PROVIDERS,
                &routines_loc,
                "provider routine provider names",
            );
            for provider in PROVIDER_ROUTINE_PROVIDERS {
                if let Some(routine) = routines_obj.get(*provider) {
                    if routine.is_null() {
                        continue;
                    }
                    declared += 1;
                    self.validate_provider_routine_name(routine, &routines_loc.field(provider));
                }
            }
        }
        if declared == 0 && default_name.is_none() {
            self.error(
                "missing_provider_routine",
                "Provider routine must declare a portable routine `name` or at least one provider-specific routine override."
                    .to_string(),
                &loc,
                "portable name or one of routines.postgres, routines.mssql, routines.snowflake",
                None,
                "Set `name` for the common provider routine target, or use `routines` only when provider names differ.",
            );
        }
    }

    fn validate_provider_routine_name(&mut self, value: &Value, loc: &Location) {
        let Some(obj) = self.expect_object(value, loc, "provider routine name object") else {
            return;
        };
        self.validate_known_fields(
            obj,
            &["schema", "name"],
            loc,
            "provider routine name fields",
        );
        let schema = self.string_field(obj, "schema", loc, false);
        let name = self.string_field(obj, "name", loc, true);
        if let Some(schema) = schema {
            self.validate_provider_routine_identifier(
                &schema,
                &loc.field("schema"),
                "routine schema",
            );
        }
        if let Some(name) = name {
            self.validate_provider_routine_identifier(&name, &loc.field("name"), "routine name");
        }
    }

    fn validate_provider_routine_arg_types(
        &mut self,
        value: Option<&Value>,
        method_loc: &Location,
    ) {
        let Some(value) = value else {
            return;
        };
        let loc = method_loc.field("args");
        let Some(items) = value.as_array() else {
            return;
        };
        for (idx, item) in items.iter().enumerate() {
            let item_loc = loc.index(idx);
            let Some(obj) = item.as_object() else {
                continue;
            };
            let Some(arg_type) = obj.get("arg_type").and_then(Value::as_str) else {
                continue;
            };
            if PROVIDER_ROUTINE_ARG_TYPES.contains(&arg_type) {
                continue;
            }
            self.error(
                "unsupported_provider_routine_arg_type",
                format!(
                    "Provider routine argument type `{arg_type}` is not supported by generated binding."
                ),
                &item_loc.field("arg_type"),
                allowed_values(PROVIDER_ROUTINE_ARG_TYPES),
                Some(arg_type.to_string()),
                "Use a supported scalar/JSON argument type or add provider binding support for this type.",
            );
        }
    }

    fn validate_provider_routine_identifier(&mut self, value: &str, loc: &Location, label: &str) {
        if provider_routine_identifier_is_safe(value) {
            return;
        }
        self.error(
            "unsafe_provider_routine_identifier",
            format!("Provider routine {label} `{value}` is not a safe identifier."),
            loc,
            "ASCII identifier beginning with a letter or underscore",
            Some(value.to_string()),
            "Use a simple routine identifier and keep SQL text out of app_gen config.",
        );
    }

    fn validate_indexes(
        &mut self,
        indexes: &[String],
        props: &HashMap<String, PropertyInfo>,
        entity_loc: &Location,
    ) {
        let mut seen = HashSet::new();
        for index in indexes {
            if !props.contains_key(index) {
                self.error(
                    "missing_index_property",
                    format!("Index references missing property `{index}`."),
                    &entity_loc.field("indexes"),
                    "property name declared in the same entity",
                    Some(index.clone()),
                    "Change the index to an existing property name or add the property.",
                );
            }
            if !seen.insert(index.clone()) {
                self.warn(
                    "duplicate_index",
                    format!("Index `{index}` is listed more than once."),
                    &entity_loc.field("indexes"),
                    "unique indexes per entity",
                    Some(index.clone()),
                    "Remove the duplicate index entry.",
                );
            }
        }
    }

    fn validate_relationships(&mut self) {
        let entities: Vec<EntityInfo> = self.entity_types.values().cloned().collect();

        for entity in &entities {
            if let Some(base_type) = &entity.base_type {
                let key = entity_key(&entity.schema_name, base_type);
                match self.entity_types.get(&key).cloned() {
                    Some(base) if !base.is_union => self.error(
                        "invalid_base_type",
                        format!(
                            "Entity `{}` derives from `{base_type}`, but `{base_type}` is not marked `is_union: true`.",
                            entity.name
                        ),
                        &entity.loc.field("base_type"),
                        "base_type referencing an entity with `is_union: true`",
                        Some(base_type.clone()),
                        "Set `is_union: true` on the base entity or change `base_type`.",
                    ),
                    Some(_) => {}
                    None => self.error(
                        "missing_base_type",
                        format!("Entity `{}` references missing base type `{base_type}`.", entity.name),
                        &entity.loc.field("base_type"),
                        "existing entity type in the same schema",
                        Some(base_type.clone()),
                        "Add the base entity or correct `base_type`.",
                    ),
                }
            }

            for prop_name in &entity.prop_names {
                let Some(prop) = entity.props.get(prop_name).cloned() else {
                    continue;
                };
                self.validate_property_relationships(entity, &prop);
            }
        }

        let relationships = self.relationships.clone();
        for relationship in &relationships {
            self.validate_schema_relationship(relationship);
        }
    }

    fn validate_schema_relationship(&mut self, relationship: &RelationshipInfo) {
        match relationship.kind.as_str() {
            "OneToOne" => {
                let Some(left) = self.require_relationship_endpoint(relationship, "left") else {
                    return;
                };
                let Some(right) = self.require_relationship_endpoint(relationship, "right") else {
                    return;
                };
                let Some(storage) = self.require_relationship_storage(relationship) else {
                    return;
                };
                if storage.storage_type != "ForeignKey" {
                    return;
                }
                if !self.endpoint_matches_owner(left, storage)
                    && !self.endpoint_matches_owner(right, storage)
                {
                    self.error(
                        "relationship_storage_owner_not_endpoint",
                        format!(
                            "OneToOne relationship `{}` storage owner must be one endpoint.",
                            relationship.name
                        ),
                        &storage.loc.field("owner"),
                        "left.entity or right.entity",
                        Some(format!("{}.{}", storage.owner_schema, storage.owner)),
                        "Set storage.owner to one endpoint entity.",
                    );
                    return;
                }
                let target = if self.endpoint_matches_owner(left, storage) {
                    right
                } else {
                    left
                };
                self.validate_relationship_fk(relationship, storage, target);
                self.validate_generated_relationship_field(relationship, left);
                self.validate_generated_relationship_field(relationship, right);
            }
            "OneToMany" => {
                let Some(one) = self.require_relationship_endpoint(relationship, "one") else {
                    return;
                };
                let Some(many) = self.require_relationship_endpoint(relationship, "many") else {
                    return;
                };
                let Some(storage) = self.require_relationship_storage(relationship) else {
                    return;
                };
                if storage.storage_type != "ForeignKey" {
                    return;
                }
                if !self.endpoint_matches_owner(many, storage) {
                    self.error(
                        "relationship_storage_owner_not_many_endpoint",
                        format!(
                            "OneToMany relationship `{}` must store the foreign key on the many endpoint.",
                            relationship.name
                        ),
                        &storage.loc.field("owner"),
                        format!("{}.{}", many.schema_name, many.entity_name),
                        Some(format!("{}.{}", storage.owner_schema, storage.owner)),
                        "Set storage.owner to the entity named by the `many` endpoint.",
                    );
                    return;
                }
                self.validate_relationship_fk(relationship, storage, one);
                self.validate_generated_relationship_field(relationship, one);
                self.validate_generated_relationship_field(relationship, many);
            }
            "ManyToMany" => {
                let Some(left) = self.require_relationship_endpoint(relationship, "left") else {
                    return;
                };
                let Some(right) = self.require_relationship_endpoint(relationship, "right") else {
                    return;
                };
                let Some(junction) = relationship.junction.as_ref() else {
                    self.error(
                        "missing_required_field",
                        format!(
                            "ManyToMany relationship `{}` is missing `junction`.",
                            relationship.name
                        ),
                        &relationship.loc.field("junction"),
                        "junction object",
                        None,
                        "Add junction.entity, junction.left_key, and junction.right_key.",
                    );
                    return;
                };
                self.validate_relationship_endpoint_entity(relationship, left);
                self.validate_relationship_endpoint_entity(relationship, right);
                self.validate_generated_relationship_field(relationship, left);
                self.validate_generated_relationship_field(relationship, right);

                let key = entity_key(&junction.schema_name, &junction.entity_name);
                let mut columns = BTreeSet::new();
                columns.insert("id".to_string());
                columns.insert(junction.left_key.clone());
                columns.insert(junction.right_key.clone());
                self.junctions
                    .entry(key)
                    .and_modify(|existing| existing.columns.extend(columns.iter().cloned()))
                    .or_insert(JunctionInfo { columns });
            }
            _ => {}
        }
    }

    fn require_relationship_endpoint<'a>(
        &mut self,
        relationship: &'a RelationshipInfo,
        name: &str,
    ) -> Option<&'a RelationshipEndpointInfo> {
        let endpoint = match name {
            "left" => relationship.left.as_ref(),
            "right" => relationship.right.as_ref(),
            "one" => relationship.one.as_ref(),
            "many" => relationship.many.as_ref(),
            _ => None,
        };
        if endpoint.is_none() {
            self.error(
                "missing_required_field",
                format!("Relationship `{}` is missing `{name}`.", relationship.name),
                &relationship.loc.field(name),
                "relationship endpoint object",
                None,
                format!("Add `{name}.entity` and `{name}.field`."),
            );
        }
        endpoint
    }

    fn require_relationship_storage<'a>(
        &mut self,
        relationship: &'a RelationshipInfo,
    ) -> Option<&'a RelationshipStorageInfo> {
        if relationship.storage.is_none() {
            self.error(
                "missing_required_field",
                format!("Relationship `{}` is missing `storage`.", relationship.name),
                &relationship.loc.field("storage"),
                "storage object",
                None,
                "Add storage.type, storage.owner, and storage.field.",
            );
        }
        relationship.storage.as_ref()
    }

    fn endpoint_matches_owner(
        &self,
        endpoint: &RelationshipEndpointInfo,
        storage: &RelationshipStorageInfo,
    ) -> bool {
        endpoint.schema_name == storage.owner_schema && endpoint.entity_name == storage.owner
    }

    fn validate_relationship_endpoint_entity(
        &mut self,
        relationship: &RelationshipInfo,
        endpoint: &RelationshipEndpointInfo,
    ) {
        let key = entity_key(&endpoint.schema_name, &endpoint.entity_name);
        if !self.entity_types.contains_key(&key) {
            self.error(
                "missing_relationship_endpoint_entity",
                format!(
                    "Relationship `{}` endpoint references missing entity `{}.{}`.",
                    relationship.name, endpoint.schema_name, endpoint.entity_name
                ),
                &endpoint.loc.field("entity"),
                "existing entity type",
                Some(format!("{}.{}", endpoint.schema_name, endpoint.entity_name)),
                "Add the entity or correct the endpoint.",
            );
        }
    }

    fn validate_generated_relationship_field(
        &mut self,
        relationship: &RelationshipInfo,
        endpoint: &RelationshipEndpointInfo,
    ) {
        let key = entity_key(&endpoint.schema_name, &endpoint.entity_name);
        let Some(entity) = self.entity_types.get(&key).cloned() else {
            return;
        };
        let Some(existing) = entity.props.get(&endpoint.field_name) else {
            return;
        };
        if !matches!(
            existing.data_type.as_deref(),
            Some("NavToOne" | "NavToMany" | "ManyToMany")
        ) {
            self.error(
                "relationship_field_conflicts_with_native_property",
                format!(
                    "Relationship `{}` would generate `{}.{}` but that field is already native.",
                    relationship.name, entity.name, endpoint.field_name
                ),
                &endpoint.loc.field("field"),
                "missing field or existing virtual relationship field",
                Some(endpoint.field_name.clone()),
                "Rename the relationship field or remove the conflicting native property.",
            );
        }
    }

    fn validate_relationship_fk(
        &mut self,
        relationship: &RelationshipInfo,
        storage: &RelationshipStorageInfo,
        target: &RelationshipEndpointInfo,
    ) {
        let owner_key = entity_key(&storage.owner_schema, &storage.owner);
        let Some(owner) = self.entity_types.get(&owner_key).cloned() else {
            self.error(
                "missing_relationship_storage_owner",
                format!(
                    "Relationship `{}` storage owner `{}.{}` does not exist.",
                    relationship.name, storage.owner_schema, storage.owner
                ),
                &storage.loc.field("owner"),
                "existing entity type",
                Some(format!("{}.{}", storage.owner_schema, storage.owner)),
                "Set storage.owner to an existing entity.",
            );
            return;
        };

        let Some(fk_prop) = owner.props.get(&storage.field).cloned() else {
            self.error(
                "missing_relationship_storage_field",
                format!(
                    "Relationship `{}` references missing FK field `{}.{}`.",
                    relationship.name, owner.name, storage.field
                ),
                &storage.loc.field("field"),
                "foreign-key property on storage.owner",
                Some(storage.field.clone()),
                "Add the scalar FK property or correct storage.field.",
            );
            return;
        };
        let Some(fk) = fk_prop.foreign_key.as_ref() else {
            self.error(
                "relationship_storage_field_not_foreign_key",
                format!(
                    "Relationship `{}` storage field `{}.{}` is not a foreign_key.",
                    relationship.name, owner.name, storage.field
                ),
                &storage.loc.field("field"),
                "property with a foreign_key block",
                Some(storage.field.clone()),
                "Move the relationship to a scalar FK property or add foreign_key metadata.",
            );
            return;
        };

        let fk_schema = if fk.schema_name.trim().is_empty() {
            owner.schema_name.clone()
        } else {
            fk.schema_name.clone()
        };
        if fk_schema != target.schema_name || fk.type_name != target.entity_name {
            self.error(
                "relationship_storage_target_mismatch",
                format!(
                    "Relationship `{}` storage field `{}.{}` targets `{}.{}`, expected `{}.{}`.",
                    relationship.name,
                    owner.name,
                    storage.field,
                    fk_schema,
                    fk.type_name,
                    target.schema_name,
                    target.entity_name
                ),
                &storage.loc.field("field"),
                "FK target matching the opposite endpoint",
                Some(format!("{}.{}", fk_schema, fk.type_name)),
                "Point storage.field at the FK that backs this relationship.",
            );
        }
    }

    fn validate_property_relationships(&mut self, entity: &EntityInfo, prop: &PropertyInfo) {
        let Some(data_type) = &prop.data_type else {
            self.error(
                "missing_data_type",
                format!(
                    "Property `{}.{}` has no data type after fragment resolution.",
                    entity.name, prop.name
                ),
                &prop.loc,
                "`data_type` or `fragment` whose YAML includes `data_type`",
                None,
                "Add `data_type: String` or use a fragment such as `property-string`.",
            );
            return;
        };

        if matches!(data_type.as_str(), "Enum" | "EnumArray") {
            match &prop.enum_type_name {
                Some(enum_type_name)
                    if self.enum_exists(&entity.schema_name, enum_type_name)
                        || self.enum_exists("system", enum_type_name) => {}
                Some(enum_type_name) => self.error(
                    "missing_enum_type",
                    format!(
                        "Property `{}.{}` references missing enum `{enum_type_name}`.",
                        entity.name, prop.name
                    ),
                    &prop.loc.field("enum_type_name"),
                    "enum type from the same schema or system schema",
                    Some(enum_type_name.clone()),
                    "Add the enum YAML or correct `enum_type_name`.",
                ),
                None => self.error(
                    "missing_enum_type",
                    format!(
                        "Property `{}.{}` uses `{data_type}` without `enum_type_name`.",
                        entity.name, prop.name
                    ),
                    &prop.loc.field("enum_type_name"),
                    "enum type name",
                    None,
                    "Set `enum_type_name` to the enum backing this property.",
                ),
            }
        } else if prop.enum_type_name.is_some() {
            self.warn(
                "unused_enum_type_name",
                format!(
                    "Property `{}.{}` sets `enum_type_name` but its data type is `{data_type}`.",
                    entity.name, prop.name
                ),
                &prop.loc.field("enum_type_name"),
                "`enum_type_name` only with `Enum` or `EnumArray`",
                prop.enum_type_name.clone(),
                "Remove `enum_type_name` or change `data_type` to `Enum`/`EnumArray`.",
            );
        }

        if matches!(data_type.as_str(), "Object" | "ObjectArray") {
            match &prop.nested_entity_type {
                Some(nested) => self.validate_entity_ref(nested, "missing_nested_entity_type"),
                None => self.error(
                    "missing_nested_entity_type",
                    format!(
                        "Property `{}.{}` uses `{data_type}` without `nested_entity_type`.",
                        entity.name, prop.name
                    ),
                    &prop.loc.field("nested_entity_type"),
                    "nested entity type reference",
                    None,
                    "Set `nested_entity_type.type_name` to the object shape for this property.",
                ),
            }
        }

        if matches!(data_type.as_str(), "NavToOne" | "NavToMany" | "ManyToMany") {
            self.error(
                "relationship_property_requires_schema_relationship",
                format!(
                    "Property `{}.{}` hand-authors generated navigation data type `{data_type}`.",
                    entity.name, prop.name
                ),
                &prop.loc.field("data_type"),
                "schema-level relationship in .appfw/model/schemas/<schema>/relationships/*.yaml; entity files should keep scalar storage fields only",
                Some(data_type.clone()),
                "Define the relationship under .appfw/model/schemas/<schema>/relationships/*.yaml; keep only the scalar FK property on the entity and let app_gen generate the navigation field. Use relationship.kind: ManyToMany for junction-backed to-many navigation.",
            );
            return;
        }

        if let Some(fk) = &prop.foreign_key {
            if matches!(data_type.as_str(), "NavToOne" | "NavToMany" | "ManyToMany") {
                self.error(
                    "invalid_foreign_key_data_type",
                    format!(
                        "Property `{}.{}` has `foreign_key` but uses virtual relationship data type `{data_type}`.",
                        entity.name, prop.name
                    ),
                    &prop.loc.field("data_type"),
                    "scalar key data type such as Uuid, String, or Int64",
                    Some(data_type.clone()),
                    "Move `foreign_key` onto the scalar key property and define the navigation through a schema-level relationship.",
                );
            }
            self.validate_foreign_key(entity, prop, fk);
        }

        match (&prop.nav_by_fk_property, data_type.as_str()) {
            (Some(nav), "NavToOne" | "NavToMany") => self.validate_nav_by_fk(entity, prop, nav),
            (Some(_), _) => self.error(
                "invalid_nav_data_type",
                format!(
                    "Property `{}.{}` declares `nav_by_fk_property` but data type is `{data_type}`.",
                    entity.name, prop.name
                ),
                &prop.loc.field("data_type"),
                "NavToOne or NavToMany",
                Some(data_type.clone()),
                "Change `data_type` to NavToOne/NavToMany or remove `nav_by_fk_property`.",
            ),
            (None, "NavToOne" | "NavToMany") => self.error(
                "missing_nav_by_fk_property",
                format!(
                    "Property `{}.{}` uses `{data_type}` without `nav_by_fk_property`.",
                    entity.name, prop.name
                ),
                &prop.loc.field("nav_by_fk_property"),
                "nav_by_fk_property with prop_name",
                None,
                "Add `nav_by_fk_property.prop_name` pointing at the scalar foreign-key property.",
            ),
            _ => {}
        }

        match (&prop.many_to_many_property, data_type.as_str()) {
            (Some(many_to_many), "ManyToMany") => {
                self.validate_many_to_many(entity, prop, many_to_many)
            }
            (Some(_), _) => self.error(
                "invalid_many_to_many_data_type",
                format!(
                    "Property `{}.{}` declares `many_to_many_property` but data type is `{data_type}`.",
                    entity.name, prop.name
                ),
                &prop.loc.field("data_type"),
                "ManyToMany",
                Some(data_type.clone()),
                "Change `data_type` to ManyToMany or remove `many_to_many_property`.",
            ),
            (None, "ManyToMany") => self.error(
                "missing_many_to_many_property",
                format!(
                    "Property `{}.{}` uses ManyToMany without `many_to_many_property`.",
                    entity.name, prop.name
                ),
                &prop.loc.field("many_to_many_property"),
                "many_to_many_property block",
                None,
                "Add junction_table, local_key, foreign_key, target_schema, and target_type.",
            ),
            _ => {}
        }
    }

    fn validate_foreign_key(&mut self, entity: &EntityInfo, prop: &PropertyInfo, fk: &RelationRef) {
        let key = entity_key(&fk.schema_name, &fk.type_name);
        let Some(target) = self.entity_types.get(&key).cloned() else {
            self.error(
                "missing_foreign_key_target",
                format!(
                    "Foreign key `{}.{}` targets missing entity `{}.{}`.",
                    entity.name, prop.name, fk.schema_name, fk.type_name
                ),
                &fk.loc,
                "existing entity type",
                Some(format!("{}.{}", fk.schema_name, fk.type_name)),
                "Add the target entity or correct `foreign_key.type_name` / `schema_name`.",
            );
            return;
        };

        let key_props: Vec<PropertyInfo> = target
            .props
            .values()
            .filter(|target_prop| target_prop.is_key)
            .cloned()
            .collect();
        if key_props.is_empty() && target.is_table {
            self.error(
                "missing_target_key",
                format!(
                    "Foreign key `{}.{}` targets table `{}` with no key property.",
                    entity.name, prop.name, target.name
                ),
                &fk.loc,
                "target entity with a property where `is_key: true`",
                None,
                "Add a primary-key property to the target entity.",
            );
        } else if key_props.len() == 1 {
            let target_key = &key_props[0];
            if let (Some(source_dt), Some(target_dt)) = (&prop.data_type, &target_key.data_type) {
                if source_dt != target_dt {
                    self.error(
                        "foreign_key_type_mismatch",
                        format!(
                            "Foreign key `{}.{}` uses `{source_dt}` but target key `{}.{}` uses `{target_dt}`.",
                            entity.name, prop.name, target.name, target_key.name
                        ),
                        &prop.loc.field("data_type"),
                        format!("same data type as target key (`{target_dt}`)"),
                        Some(source_dt.clone()),
                        format!("Change `{}` to `data_type: {target_dt}` or target the correct entity.", prop.name),
                    );
                }
            }
        }
    }

    fn validate_nav_by_fk(&mut self, entity: &EntityInfo, prop: &PropertyInfo, nav: &NavRef) {
        let key = entity_key(&nav.schema_name, &nav.type_name);
        let Some(target) = self.entity_types.get(&key).cloned() else {
            self.error(
                "missing_navigation_target",
                format!(
                    "Navigation `{}.{}` targets missing entity `{}.{}`.",
                    entity.name, prop.name, nav.schema_name, nav.type_name
                ),
                &nav.loc,
                "existing entity type",
                Some(format!("{}.{}", nav.schema_name, nav.type_name)),
                "Add the target entity or correct `nav_by_fk_property.type_name` / `schema_name`.",
            );
            return;
        };

        let Some(fk_prop) = target.props.get(&nav.prop_name).cloned() else {
            self.error(
                "missing_navigation_property",
                format!(
                    "Navigation `{}.{}` points at missing property `{}.{}`.",
                    entity.name, prop.name, target.name, nav.prop_name
                ),
                &nav.loc.field("prop_name"),
                "property on the referenced entity",
                Some(nav.prop_name.clone()),
                "Set `prop_name` to the scalar foreign-key property name.",
            );
            return;
        };

        if fk_prop.foreign_key.is_none() {
            self.error(
                "navigation_property_not_foreign_key",
                format!(
                    "Navigation `{}.{}` points at `{}.{}`, but that property has no `foreign_key`.",
                    entity.name, prop.name, target.name, fk_prop.name
                ),
                &nav.loc.field("prop_name"),
                "property with a `foreign_key` block",
                Some(nav.prop_name.clone()),
                "Point at the scalar FK property, or add `foreign_key` to that property.",
            );
        }
    }

    fn validate_many_to_many(
        &mut self,
        entity: &EntityInfo,
        prop: &PropertyInfo,
        many_to_many: &ManyToManyRef,
    ) {
        let target_key = entity_key(&many_to_many.target_schema, &many_to_many.target_type);
        if !self.entity_types.contains_key(&target_key) {
            self.error(
                "missing_many_to_many_target",
                format!(
                    "Many-to-many `{}.{}` targets missing entity `{}.{}`.",
                    entity.name, prop.name, many_to_many.target_schema, many_to_many.target_type
                ),
                &many_to_many.loc.field("target_type"),
                "existing target entity type",
                Some(format!(
                    "{}.{}",
                    many_to_many.target_schema, many_to_many.target_type
                )),
                "Add the target entity or correct `target_schema` / `target_type`.",
            );
        }

        let junction_schema = many_to_many
            .junction_schema
            .clone()
            .unwrap_or_else(|| entity.schema_name.clone());
        let junction_key = entity_key(&junction_schema, &many_to_many.junction_table);
        if let Some(junction_entity) = self.entity_types.get(&junction_key).cloned() {
            for key_name in [&many_to_many.local_key, &many_to_many.foreign_key] {
                if !junction_entity.props.contains_key(key_name) {
                    self.error(
                        "missing_junction_property",
                        format!(
                            "Many-to-many junction `{}` is missing key property `{key_name}`.",
                            many_to_many.junction_table
                        ),
                        &many_to_many.loc,
                        "junction entity with local_key and foreign_key properties",
                        Some(key_name.clone()),
                        "Add the property to the junction entity or correct the key name.",
                    );
                }
            }
        }

        let key = entity_key(&junction_schema, &many_to_many.junction_table);
        let mut columns = BTreeSet::new();
        columns.insert("id".to_string());
        columns.insert(many_to_many.local_key.clone());
        columns.insert(many_to_many.foreign_key.clone());
        self.junctions
            .entry(key)
            .and_modify(|junction| junction.columns.extend(columns.iter().cloned()))
            .or_insert(JunctionInfo { columns });
    }

    fn validate_entity_ref(&mut self, relation: &RelationRef, code: &'static str) {
        let key = entity_key(&relation.schema_name, &relation.type_name);
        if !self.entity_types.contains_key(&key) {
            self.error(
                code,
                format!(
                    "Reference points at missing entity `{}.{}`.",
                    relation.schema_name, relation.type_name
                ),
                &relation.loc,
                "existing entity type",
                Some(format!("{}.{}", relation.schema_name, relation.type_name)),
                "Add the referenced entity or correct the type name.",
            );
        }
    }

    fn validate_provider_features(&mut self) {
        let entities: Vec<EntityInfo> = self.entity_types.values().cloned().collect();
        for entity in &entities {
            let Some(schema) = self.schemas.get(&entity.schema_name).cloned() else {
                continue;
            };
            let Some(provider) = &schema.data_source_type else {
                continue;
            };

            for prop in entity.props.values() {
                let Some(data_type) = &prop.data_type else {
                    continue;
                };
                if provider == "PostgreSQL"
                    && matches!(data_type.as_str(), "ObjectId" | "ObjectIdArray")
                {
                    self.error(
                        "unsupported_provider_feature",
                        format!(
                            "PostgreSQL schema `{}` uses Mongo-style `{data_type}` on `{}.{}`.",
                            schema.name, entity.name, prop.name
                        ),
                        &prop.loc.field("data_type"),
                        "data type supported by PostgreSQL generation",
                        Some(data_type.clone()),
                        "Use `Uuid`/`String`, or move the schema to a MongoDB data source.",
                    );
                }
            }
        }
    }

    fn validate_seed_configs(&mut self) {
        let schema_dirs = self.schema_dirs.clone();
        for (schema_name, schema_dir) in schema_dirs {
            let seeds_dir = schema_dir.join("seeds");
            let files = self.yaml_files(&seeds_dir, false, true);
            if self
                .schemas
                .get(&schema_name)
                .map(|schema| schema.external_read_only)
                .unwrap_or(false)
            {
                for file in files {
                    let loc = Location::root(file.clone()).with_schema(schema_name.clone());
                    self.error(
                        "external_read_only_seed",
                        format!(
                            "Schema `{schema_name}` is configured as external read-only storage but declares seed data."
                        ),
                        &loc,
                        "no seed files for external read-only schemas",
                        Some(file.display().to_string()),
                        "Remove seed files from this schema; external authoritative data must be loaded and governed by its owning platform.",
                    );
                }
                continue;
            }

            for file in files {
                let loc = Location::root(file.clone()).with_schema(schema_name.clone());
                let Some(value) = self.read_yaml_value(&file, &loc) else {
                    continue;
                };
                let Some(items) = self.expect_array(&value, &loc, "seed group array") else {
                    continue;
                };

                for (idx, item) in items.iter().enumerate() {
                    self.validate_seed_group(item, &loc.index(idx), &schema_name);
                }
            }
        }
    }

    fn validate_seed_group(&mut self, item: &Value, loc: &Location, default_schema: &str) {
        let Some(obj) = self.expect_object(item, loc, "seed group object") else {
            return;
        };
        self.validate_known_fields(
            obj,
            &["entity_type", "schema", "columns", "records"],
            loc,
            "seed group fields",
        );
        let entity_type = self.string_field(obj, "entity_type", loc, true);
        let schema_name = self
            .string_field(obj, "schema", loc, false)
            .unwrap_or_else(|| default_schema.to_string());
        let columns = self.string_array_field(obj, "columns", loc, true);
        let records_loc = loc.field("records");
        let records = obj
            .get("records")
            .and_then(|value| self.expect_array(value, &records_loc, "seed records array"));

        let Some(entity_type) = entity_type else {
            return;
        };

        let entity_key = entity_key(&schema_name, &entity_type);
        let entity = self.entity_types.get(&entity_key).cloned();
        let allowed_columns = self.seed_allowed_columns(&schema_name, &entity_type);
        if allowed_columns.is_none() {
            self.error(
                "missing_seed_entity",
                format!("Seed group references missing entity or junction `{schema_name}.{entity_type}`."),
                &loc.field("entity_type"),
                "entity type name or generated many-to-many junction table",
                Some(entity_type.clone()),
                "Add the entity type, add the many-to-many relationship that creates this junction, or correct `entity_type`.",
            );
            return;
        }
        let allowed_columns = allowed_columns.unwrap_or_default();

        let mut seen_columns = HashSet::new();
        for column in &columns {
            if !allowed_columns.contains(column) {
                self.error(
                    "invalid_seed_column",
                    format!(
                        "Seed column `{column}` is not valid for `{schema_name}.{entity_type}`."
                    ),
                    &loc.field("columns"),
                    "stored property or junction column name",
                    Some(column.clone()),
                    "Remove the column or add the matching stored property.",
                );
            }
            if !seen_columns.insert(column.clone()) {
                self.warn(
                    "duplicate_seed_column",
                    format!(
                        "Seed column `{column}` is listed more than once for `{schema_name}.{entity_type}`."
                    ),
                    &loc.field("columns"),
                    "unique seed columns",
                    Some(column.clone()),
                    "Remove the duplicate column from `columns`.",
                );
            }
        }

        let column_set: HashSet<String> = columns.iter().cloned().collect();
        if let Some(entity) = &entity {
            for prop in entity.props.values() {
                if !prop.is_required
                    || prop.is_read_only
                    || !is_seed_native_property(prop)
                    || is_generated_seed_value(prop)
                {
                    continue;
                }
                if !column_set.contains(&prop.name) {
                    self.error(
                        "missing_required_seed_column",
                        format!(
                            "Seed group `{schema_name}.{entity_type}` does not list required property `{}`.",
                            prop.name
                        ),
                        &loc.field("columns"),
                        "all non-generated required native properties",
                        Some(prop.name.clone()),
                        format!("Add `{}` to `columns` and every record, or make it optional.", prop.name),
                    );
                }
            }
        }

        if let Some(records) = records {
            for (idx, record) in records.iter().enumerate() {
                let record_loc = records_loc.index(idx);
                let Some(record_obj) =
                    self.expect_object(record, &record_loc, "seed record object")
                else {
                    continue;
                };
                for key in record_obj.keys() {
                    if !column_set.contains(key) {
                        self.warn(
                            "seed_record_key_not_in_columns",
                            format!(
                                "Seed record key `{key}` is not listed in the group `columns`."
                            ),
                            &record_loc.field(key),
                            "record keys listed in `columns`",
                            Some(key.clone()),
                            "Add the key to `columns` or remove it from the record.",
                        );
                    }
                }
                if let Some(id_value) = record_obj.get("id").and_then(Value::as_str) {
                    let seed_key = format!("{schema_name}.{entity_type}.{id_value}");
                    if let Some(first_loc) = self.seed_keys.get(&seed_key).cloned() {
                        self.error(
                            "duplicate_seed_key",
                            format!("Seed id `{id_value}` is used more than once for `{schema_name}.{entity_type}`."),
                            &record_loc.field("id"),
                            "unique seed id per entity",
                            Some(id_value.to_string()),
                            format!(
                                "Change this id or remove the duplicate. First use is at {} {}.",
                                first_loc.file.display(),
                                first_loc.path
                            ),
                        );
                    } else {
                        self.seed_keys.insert(seed_key, record_loc.field("id"));
                    }
                }
                if let Some(entity) = &entity {
                    for column in &column_set {
                        let Some(prop) = entity.props.get(column) else {
                            continue;
                        };
                        let value = record_obj.get(column);
                        if value.is_none()
                            && prop.is_required
                            && !prop.is_read_only
                            && is_seed_native_property(prop)
                            && !is_generated_seed_value(prop)
                        {
                            self.error(
                                "missing_required_seed_value",
                                format!(
                                    "Seed record for `{schema_name}.{entity_type}` is missing required property `{column}`."
                                ),
                                &record_loc.field(column),
                                "value for required property",
                                None,
                                format!("Add `{column}: <value>` to this seed record."),
                            );
                            continue;
                        }
                        if let Some(value) = value {
                            self.validate_seed_value(
                                &schema_name,
                                &entity_type,
                                prop,
                                value,
                                &record_loc.field(column),
                            );
                        }
                    }
                }
            }
        }
    }

    fn validate_test_configs(&mut self) {
        let schema_dirs = self.schema_dirs.clone();
        for (schema_name, schema_dir) in schema_dirs {
            let tests_dir = schema_dir.join("tests");
            let files = self.yaml_files(&tests_dir, false, true);

            for file in files {
                let loc = Location::root(file.clone()).with_schema(schema_name.clone());
                let Some(value) = self.read_yaml_value(&file, &loc) else {
                    continue;
                };
                let Some(items) = self.expect_array(&value, &loc, "API test array") else {
                    continue;
                };

                for (idx, item) in items.iter().enumerate() {
                    self.validate_test_case(item, &loc.index(idx), &schema_name);
                }
            }
        }
    }

    fn validate_test_case(&mut self, item: &Value, loc: &Location, default_schema: &str) {
        let Some(obj) = self.expect_object(item, loc, "API test object") else {
            return;
        };
        self.string_field(obj, "name", loc, true);
        self.string_field(obj, "description", loc, false);
        self.string_field(obj, "auth_token", loc, false);
        self.string_field(obj, "result_object_name", loc, false);
        if let Some(graphql) = obj.get("graphql") {
            self.validate_test_graphql(graphql, &loc.field("graphql"), default_schema);
        }
        if let Some(expect) = obj.get("expect") {
            if !expect.is_object() {
                self.error(
                    "invalid_test_expect_shape",
                    "`expect` must be an object.".to_string(),
                    &loc.field("expect"),
                    "object with data or error expectations",
                    Some(value_kind(expect).to_string()),
                    "Use `expect.data` or `expect.error`.",
                );
            }
        }
    }

    fn validate_test_graphql(&mut self, value: &Value, loc: &Location, default_schema: &str) {
        let Some(obj) = self.expect_object(value, loc, "graphql test object") else {
            return;
        };
        let schema_name = self
            .string_field(obj, "schema", loc, false)
            .unwrap_or_else(|| default_schema.to_string());
        if !self.schemas.contains_key(&schema_name) {
            self.error(
                "missing_test_schema",
                format!("API test references missing schema `{schema_name}`."),
                &loc.field("schema"),
                "schema declared under .appfw/model/schemas",
                Some(schema_name),
                "Correct `graphql.schema` or add the schema config.",
            );
        }

        if let Some(operation_type) = self.string_field(obj, "type", loc, true) {
            if !matches!(operation_type.as_str(), "query" | "mutation") {
                self.error(
                    "invalid_graphql_operation_type",
                    format!("Unsupported GraphQL operation type `{operation_type}`."),
                    &loc.field("type"),
                    "`query` or `mutation`",
                    Some(operation_type),
                    "Use `type: query` or `type: mutation`.",
                );
            }
        }
        self.string_field(obj, "name", loc, true);
        self.string_field(obj, "select", loc, false);
        if let Some(variables) = obj.get("variables") {
            let var_loc = loc.field("variables");
            if let Some(items) = self.expect_array(variables, &var_loc, "GraphQL variables array") {
                for (idx, variable) in items.iter().enumerate() {
                    let item_loc = var_loc.index(idx);
                    if let Some(var_obj) =
                        self.expect_object(variable, &item_loc, "GraphQL variable object")
                    {
                        self.string_field(var_obj, "name", &item_loc, true);
                        self.string_field(var_obj, "type", &item_loc, false);
                    }
                }
            }
        }
    }

    fn validate_seed_value(
        &mut self,
        schema_name: &str,
        entity_type: &str,
        prop: &PropertyInfo,
        value: &Value,
        loc: &Location,
    ) {
        if value.is_null() {
            if prop.is_required && !is_generated_seed_value(prop) {
                self.error(
                    "null_required_seed_value",
                    format!(
                        "Seed value `{schema_name}.{entity_type}.{}` is null but the property is required.",
                        prop.name
                    ),
                    loc,
                    "non-null value",
                    Some("null".to_string()),
                    "Provide a value or make the property optional.",
                );
            }
            return;
        }

        let Some(data_type) = prop.data_type.as_deref() else {
            return;
        };

        match data_type {
            "Uuid" => {
                let Some(raw) = self.expect_seed_string(value, loc, "UUID string") else {
                    return;
                };
                if uuid::Uuid::parse_str(raw).is_err() {
                    self.error(
                        "invalid_seed_value_type",
                        format!(
                            "Seed value `{schema_name}.{entity_type}.{}` must be a UUID.",
                            prop.name
                        ),
                        loc,
                        "UUID string",
                        Some(raw.to_string()),
                        "Use a canonical UUID such as `00000000-0000-4000-8000-000000000000`.",
                    );
                }
            }
            "ObjectId" => {
                let Some(raw) = self.expect_seed_string(value, loc, "Mongo ObjectId string") else {
                    return;
                };
                if raw.len() != 24 || !raw.chars().all(|ch| ch.is_ascii_hexdigit()) {
                    self.error(
                        "invalid_seed_value_type",
                        format!(
                            "Seed value `{schema_name}.{entity_type}.{}` must be a 24-character ObjectId hex string.",
                            prop.name
                        ),
                        loc,
                        "24-character ObjectId hex string",
                        Some(raw.to_string()),
                        "Use a valid ObjectId string or change the property data type.",
                    );
                }
            }
            "String" => {
                self.expect_seed_string(value, loc, "string");
            }
            "Date" => {
                let Some(raw) = self.expect_seed_string(value, loc, "date string") else {
                    return;
                };
                if chrono::NaiveDate::parse_from_str(raw, "%Y-%m-%d").is_err() {
                    self.error(
                        "invalid_seed_value_type",
                        format!(
                            "Seed value `{schema_name}.{entity_type}.{}` must be an ISO date.",
                            prop.name
                        ),
                        loc,
                        "YYYY-MM-DD date string",
                        Some(raw.to_string()),
                        "Use a date like `2026-01-31`.",
                    );
                }
            }
            "DateTime" => {
                let Some(raw) = self.expect_seed_string(value, loc, "date-time string") else {
                    return;
                };
                if chrono::DateTime::parse_from_rfc3339(raw).is_err() {
                    self.error(
                        "invalid_seed_value_type",
                        format!(
                            "Seed value `{schema_name}.{entity_type}.{}` must be an RFC3339 date-time.",
                            prop.name
                        ),
                        loc,
                        "RFC3339 date-time string",
                        Some(raw.to_string()),
                        "Use a timestamp like `2026-01-31T15:04:05Z`.",
                    );
                }
            }
            "Time" => {
                let Some(raw) = self.expect_seed_string(value, loc, "time string") else {
                    return;
                };
                if chrono::NaiveTime::parse_from_str(raw, "%H:%M:%S%.f").is_err() {
                    self.error(
                        "invalid_seed_value_type",
                        format!(
                            "Seed value `{schema_name}.{entity_type}.{}` must be a time.",
                            prop.name
                        ),
                        loc,
                        "HH:MM:SS time string",
                        Some(raw.to_string()),
                        "Use a time like `13:45:00`.",
                    );
                }
            }
            "Boolean" => {
                if !value.is_boolean() {
                    self.invalid_seed_type(schema_name, entity_type, prop, loc, "boolean", value);
                }
            }
            "Int8" | "Int16" | "Int32" | "Int64" => {
                let Some(number) = value.as_i64() else {
                    self.invalid_seed_type(schema_name, entity_type, prop, loc, "integer", value);
                    return;
                };
                let valid_range = match data_type {
                    "Int8" => i8::MIN as i64..=i8::MAX as i64,
                    "Int16" => i16::MIN as i64..=i16::MAX as i64,
                    "Int32" => i32::MIN as i64..=i32::MAX as i64,
                    _ => i64::MIN..=i64::MAX,
                };
                if !valid_range.contains(&number) {
                    self.error(
                        "invalid_seed_value_type",
                        format!(
                            "Seed value `{schema_name}.{entity_type}.{}` is outside the `{data_type}` range.",
                            prop.name
                        ),
                        loc,
                        format!("{data_type} numeric range"),
                        Some(number.to_string()),
                        format!("Use a value that fits `{data_type}` or change the property data type."),
                    );
                }
            }
            "Float32" | "Float64" => {
                if !value.is_number() {
                    self.invalid_seed_type(schema_name, entity_type, prop, loc, "number", value);
                }
            }
            "Enum" => {
                let Some(raw) = self.expect_seed_string(value, loc, "enum string") else {
                    return;
                };
                self.validate_seed_enum_value(schema_name, prop, raw, loc);
            }
            "Json" => {}
            "Object" => {
                if !value.is_object() {
                    self.invalid_seed_type(schema_name, entity_type, prop, loc, "object", value);
                }
            }
            "UuidArray" | "ObjectIdArray" | "StringArray" | "Int8Array" | "Int16Array"
            | "Int32Array" | "Int64Array" | "Float32Array" | "EnumArray" | "JsonArray"
            | "ObjectArray" => {
                let Some(items) = value.as_array() else {
                    self.invalid_seed_type(schema_name, entity_type, prop, loc, "array", value);
                    return;
                };
                for (idx, item) in items.iter().enumerate() {
                    self.validate_seed_array_item(
                        schema_name,
                        entity_type,
                        prop,
                        data_type,
                        item,
                        &loc.index(idx),
                    );
                }
            }
            _ => {}
        }
    }

    fn validate_seed_array_item(
        &mut self,
        schema_name: &str,
        entity_type: &str,
        prop: &PropertyInfo,
        data_type: &str,
        value: &Value,
        loc: &Location,
    ) {
        if value.is_null() {
            return;
        }
        match data_type {
            "UuidArray" => {
                let Some(raw) = self.expect_seed_string(value, loc, "UUID string") else {
                    return;
                };
                if uuid::Uuid::parse_str(raw).is_err() {
                    self.invalid_seed_type(
                        schema_name,
                        entity_type,
                        prop,
                        loc,
                        "UUID string",
                        value,
                    );
                }
            }
            "ObjectIdArray" => {
                let Some(raw) = self.expect_seed_string(value, loc, "ObjectId string") else {
                    return;
                };
                if raw.len() != 24 || !raw.chars().all(|ch| ch.is_ascii_hexdigit()) {
                    self.invalid_seed_type(
                        schema_name,
                        entity_type,
                        prop,
                        loc,
                        "24-character ObjectId hex string",
                        value,
                    );
                }
            }
            "StringArray" => {
                self.expect_seed_string(value, loc, "string");
            }
            "Int8Array" | "Int16Array" | "Int32Array" | "Int64Array" => {
                if !value.is_i64() {
                    self.invalid_seed_type(schema_name, entity_type, prop, loc, "integer", value);
                }
            }
            "Float32Array" => {
                if !value.is_number() {
                    self.invalid_seed_type(schema_name, entity_type, prop, loc, "number", value);
                }
            }
            "EnumArray" => {
                let Some(raw) = self.expect_seed_string(value, loc, "enum string") else {
                    return;
                };
                self.validate_seed_enum_value(schema_name, prop, raw, loc);
            }
            "ObjectArray" => {
                if !value.is_object() {
                    self.invalid_seed_type(schema_name, entity_type, prop, loc, "object", value);
                }
            }
            "JsonArray" => {}
            _ => {}
        }
    }

    fn validate_seed_enum_value(
        &mut self,
        schema_name: &str,
        prop: &PropertyInfo,
        value: &str,
        loc: &Location,
    ) {
        let Some(enum_type_name) = prop.enum_type_name.as_deref() else {
            return;
        };
        let key = if self
            .enum_values
            .contains_key(&enum_key(schema_name, enum_type_name))
        {
            enum_key(schema_name, enum_type_name)
        } else {
            enum_key("system", enum_type_name)
        };
        let Some(values) = self.enum_values.get(&key) else {
            return;
        };
        if !values.contains(value) {
            let mut allowed = values.iter().cloned().collect::<Vec<_>>();
            allowed.sort();
            self.error(
                "invalid_seed_enum_value",
                format!(
                    "Seed value for `{}` is not a valid `{enum_type_name}` enum item.",
                    prop.name
                ),
                loc,
                format!("one of: {}", allowed.join(", ")),
                Some(value.to_string()),
                "Use an enum item declared in the matching gql_enum_types YAML.",
            );
        }
    }

    fn expect_seed_string<'a>(
        &mut self,
        value: &'a Value,
        loc: &Location,
        expected: &str,
    ) -> Option<&'a str> {
        match value.as_str() {
            Some(value) => Some(value),
            None => {
                self.error(
                    "invalid_seed_value_type",
                    format!("Seed value must be {expected}."),
                    loc,
                    expected,
                    Some(value_kind(value).to_string()),
                    "Use a YAML scalar whose shape matches the property data_type.",
                );
                None
            }
        }
    }

    fn invalid_seed_type(
        &mut self,
        schema_name: &str,
        entity_type: &str,
        prop: &PropertyInfo,
        loc: &Location,
        expected: impl Into<String>,
        value: &Value,
    ) {
        self.error(
            "invalid_seed_value_type",
            format!(
                "Seed value `{schema_name}.{entity_type}.{}` does not match `{}`.",
                prop.name,
                prop.data_type.as_deref().unwrap_or("<unknown>")
            ),
            loc,
            expected,
            Some(value_kind(value).to_string()),
            "Use a YAML value whose shape matches the property data_type.",
        );
    }

    fn seed_allowed_columns(
        &self,
        schema_name: &str,
        entity_type: &str,
    ) -> Option<HashSet<String>> {
        let key = entity_key(schema_name, entity_type);
        if let Some(entity) = self.entity_types.get(&key) {
            return Some(
                entity
                    .props
                    .values()
                    .filter(|prop| {
                        !matches!(
                            prop.data_type.as_deref(),
                            Some("NavToOne" | "NavToMany" | "ManyToMany")
                        )
                    })
                    .map(|prop| prop.name.clone())
                    .collect(),
            );
        }
        if let Some(junction) = self.junctions.get(&key) {
            return Some(junction.columns.iter().cloned().collect());
        }
        None
    }

    fn parse_relation_ref(
        &self,
        value: Option<&Value>,
        loc: &Location,
        default_schema: &str,
    ) -> Option<RelationRef> {
        let value = value?;
        if value.is_null() {
            return None;
        }
        let obj = value.as_object()?;
        let type_name = obj.get("type_name")?.as_str()?.to_string();
        let schema_name = obj
            .get("schema_name")
            .and_then(Value::as_str)
            .unwrap_or(default_schema)
            .to_string();
        Some(RelationRef {
            schema_name,
            type_name,
            loc: loc.clone(),
        })
    }

    fn parse_nav_ref(
        &self,
        value: Option<&Value>,
        loc: &Location,
        default_schema: &str,
        default_type: &str,
    ) -> Option<NavRef> {
        let value = value?;
        if value.is_null() {
            return None;
        }
        let obj = value.as_object()?;
        let prop_name = obj.get("prop_name")?.as_str()?.to_string();
        let schema_name = obj
            .get("schema_name")
            .and_then(Value::as_str)
            .unwrap_or(default_schema)
            .to_string();
        let type_name = obj
            .get("type_name")
            .and_then(Value::as_str)
            .unwrap_or(default_type)
            .to_string();
        Some(NavRef {
            schema_name,
            type_name,
            prop_name,
            loc: loc.clone(),
        })
    }

    fn parse_many_to_many_ref(
        &self,
        value: Option<&Value>,
        loc: &Location,
    ) -> Option<ManyToManyRef> {
        let value = value?;
        if value.is_null() {
            return None;
        }
        let obj = value.as_object()?;
        Some(ManyToManyRef {
            junction_table: obj.get("junction_table")?.as_str()?.to_string(),
            junction_schema: obj
                .get("junction_schema")
                .and_then(Value::as_str)
                .map(str::to_string),
            local_key: obj.get("local_key")?.as_str()?.to_string(),
            foreign_key: obj.get("foreign_key")?.as_str()?.to_string(),
            target_schema: obj.get("target_schema")?.as_str()?.to_string(),
            target_type: obj.get("target_type")?.as_str()?.to_string(),
            loc: loc.clone(),
        })
    }

    fn parse_relationship_endpoint(
        &mut self,
        value: Option<&Value>,
        loc: &Location,
        default_schema: &str,
    ) -> Option<RelationshipEndpointInfo> {
        let value = value?;
        if value.is_null() {
            return None;
        }
        let obj = self.expect_object(value, loc, "relationship endpoint")?;
        self.validate_known_fields(
            obj,
            &["schema", "entity", "field", "caption"],
            loc,
            "relationship endpoint fields",
        );
        let entity_name = self.string_field(obj, "entity", loc, true)?;
        let field_name = self.string_field(obj, "field", loc, true)?;
        let schema_name = self
            .string_field(obj, "schema", loc, false)
            .unwrap_or_else(|| default_schema.to_string());
        self.string_field(obj, "caption", loc, false);

        Some(RelationshipEndpointInfo {
            schema_name,
            entity_name,
            field_name,
            loc: loc.clone(),
        })
    }

    fn parse_relationship_storage(
        &mut self,
        value: Option<&Value>,
        loc: &Location,
        default_schema: &str,
    ) -> Option<RelationshipStorageInfo> {
        let value = value?;
        if value.is_null() {
            return None;
        }
        let obj = self.expect_object(value, loc, "relationship storage")?;
        self.validate_known_fields(
            obj,
            &["type", "owner_schema", "owner", "field"],
            loc,
            "relationship storage fields",
        );
        let storage_type = self.enum_field(obj, "type", loc, RELATIONSHIP_STORAGE_TYPES, true)?;
        let owner = self.string_field(obj, "owner", loc, true)?;
        let field = self.string_field(obj, "field", loc, true)?;
        let owner_schema = self
            .string_field(obj, "owner_schema", loc, false)
            .unwrap_or_else(|| default_schema.to_string());

        Some(RelationshipStorageInfo {
            storage_type,
            owner_schema,
            owner,
            field,
            loc: loc.clone(),
        })
    }

    fn parse_relationship_junction(
        &mut self,
        value: Option<&Value>,
        loc: &Location,
        default_schema: &str,
    ) -> Option<RelationshipJunctionInfo> {
        let value = value?;
        if value.is_null() {
            return None;
        }
        let obj = self.expect_object(value, loc, "relationship junction")?;
        self.validate_known_fields(
            obj,
            &["schema", "entity", "left_key", "right_key"],
            loc,
            "relationship junction fields",
        );
        let entity_name = self.string_field(obj, "entity", loc, true)?;
        let left_key = self.string_field(obj, "left_key", loc, true)?;
        let right_key = self.string_field(obj, "right_key", loc, true)?;
        let schema_name = self
            .string_field(obj, "schema", loc, false)
            .unwrap_or_else(|| default_schema.to_string());

        Some(RelationshipJunctionInfo {
            schema_name,
            entity_name,
            left_key,
            right_key,
        })
    }

    fn read_yaml_value(&mut self, file: &Path, loc: &Location) -> Option<Value> {
        let file_handle = match fs::File::open(file) {
            Ok(file) => file,
            Err(err) => {
                self.error(
                    "missing_yaml_file",
                    format!("Could not open YAML file: {err}."),
                    loc,
                    "readable YAML file",
                    Some(err.to_string()),
                    "Create the file or fix its permissions.",
                );
                return None;
            }
        };

        match serde_yaml::from_reader(file_handle) {
            Ok(value) => Some(value),
            Err(err) => {
                self.error(
                    "invalid_yaml",
                    format!("YAML parse error: {err}."),
                    loc,
                    "valid YAML",
                    Some(err.to_string()),
                    "Fix the YAML syntax at the reported line and column.",
                );
                None
            }
        }
    }

    fn yaml_files(&mut self, dir: &Path, recursive: bool, skip_res: bool) -> Vec<PathBuf> {
        if !dir.exists() {
            return vec![];
        }
        let mut files = vec![];
        if let Err(err) = collect_yaml_files(dir, recursive, skip_res, &mut files) {
            let loc = Location::root(dir.to_path_buf());
            self.error(
                "read_dir_failed",
                format!("Could not read config directory: {err}."),
                &loc,
                "readable directory",
                Some(err.to_string()),
                "Fix directory permissions or restore the expected config directory.",
            );
        }
        files.sort();
        files
    }

    fn template_files(&mut self, dir: &Path) -> Vec<PathBuf> {
        if !dir.exists() {
            return vec![];
        }
        let mut files = vec![];
        if let Err(err) = collect_template_files(dir, &mut files) {
            let loc = Location::root(dir.to_path_buf());
            self.error(
                "read_dir_failed",
                format!("Could not read templates directory: {err}."),
                &loc,
                "readable _templates directory",
                Some(err.to_string()),
                "Fix directory permissions or restore the expected templates directory.",
            );
        }
        files.sort();
        files
    }

    fn schema_dirs(&mut self, schemas_dir: &Path) -> Vec<PathBuf> {
        if !schemas_dir.exists() {
            let loc = Location::root(schemas_dir.to_path_buf());
            self.error(
                "missing_schemas_dir",
                "Schemas directory is missing.".to_string(),
                &loc,
                ".appfw/model/schemas directory",
                None,
                "Restore `.appfw/model/schemas` with at least the system schema.",
            );
            return vec![];
        }

        let mut dirs = vec![];
        match fs::read_dir(schemas_dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_dir() {
                        continue;
                    }
                    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                        continue;
                    };
                    if !name.starts_with('.') {
                        dirs.push(path);
                    }
                }
            }
            Err(err) => {
                let loc = Location::root(schemas_dir.to_path_buf());
                self.error(
                    "read_dir_failed",
                    format!("Could not read schemas directory: {err}."),
                    &loc,
                    "readable .appfw/model/schemas directory",
                    Some(err.to_string()),
                    "Fix directory permissions or restore the schemas directory.",
                );
            }
        }
        dirs.sort();
        dirs
    }

    fn expect_array<'a>(
        &mut self,
        value: &'a Value,
        loc: &Location,
        expected: &str,
    ) -> Option<&'a Vec<Value>> {
        match value.as_array() {
            Some(items) => Some(items),
            None => {
                self.error(
                    "invalid_shape",
                    format!("Expected {expected}."),
                    loc,
                    expected,
                    Some(value_kind(value).to_string()),
                    "Change this YAML node to the expected shape.",
                );
                None
            }
        }
    }

    fn expect_object<'a>(
        &mut self,
        value: &'a Value,
        loc: &Location,
        expected: &str,
    ) -> Option<&'a Map<String, Value>> {
        match value.as_object() {
            Some(obj) => Some(obj),
            None => {
                self.error(
                    "invalid_shape",
                    format!("Expected {expected}."),
                    loc,
                    expected,
                    Some(value_kind(value).to_string()),
                    "Change this YAML node to the expected shape.",
                );
                None
            }
        }
    }

    fn validate_known_fields(
        &mut self,
        obj: &Map<String, Value>,
        allowed: &[&str],
        loc: &Location,
        expected_shape: &str,
    ) {
        for key in obj.keys() {
            if !allowed.contains(&key.as_str()) {
                self.error(
                    "unknown_field",
                    format!("Unknown field `{key}`."),
                    &loc.field(key),
                    expected_shape,
                    Some(key.clone()),
                    format!(
                        "Remove `{key}` or rename it to one of: {}.",
                        allowed.join(", ")
                    ),
                );
            }
        }
    }

    fn string_field(
        &mut self,
        obj: &Map<String, Value>,
        field: &str,
        loc: &Location,
        required: bool,
    ) -> Option<String> {
        match obj.get(field) {
            Some(value) if value.is_null() && !required => None,
            Some(value) => match value.as_str() {
                Some(value) => Some(value.to_string()),
                None => {
                    self.error(
                        "invalid_field_type",
                        format!("Field `{field}` must be a string."),
                        &loc.field(field),
                        "string",
                        Some(value_kind(value).to_string()),
                        format!("Set `{field}` to a quoted or unquoted string value."),
                    );
                    None
                }
            },
            None if required => {
                self.error(
                    "missing_required_field",
                    format!("Missing required field `{field}`."),
                    &loc.field(field),
                    "string",
                    None,
                    format!("Add `{field}: <value>`."),
                );
                None
            }
            None => None,
        }
    }

    fn uuid_field(
        &mut self,
        obj: &Map<String, Value>,
        field: &str,
        loc: &Location,
        required: bool,
    ) -> Option<String> {
        let value = self.string_field(obj, field, loc, required)?;
        if uuid::Uuid::parse_str(&value).is_err() {
            self.error(
                "invalid_uuid",
                format!("Field `{field}` must be a UUID."),
                &loc.field(field),
                "UUID string",
                Some(value.clone()),
                format!("Replace `{field}` with a valid UUID."),
            );
        }
        Some(value)
    }

    fn bool_field(
        &mut self,
        obj: &Map<String, Value>,
        field: &str,
        loc: &Location,
        required: bool,
    ) -> Option<bool> {
        match obj.get(field) {
            Some(value) if value.is_null() && !required => None,
            Some(value) => match value.as_bool() {
                Some(value) => Some(value),
                None => {
                    self.error(
                        "invalid_field_type",
                        format!("Field `{field}` must be a boolean."),
                        &loc.field(field),
                        "boolean",
                        Some(value_kind(value).to_string()),
                        format!("Set `{field}` to `true` or `false`."),
                    );
                    None
                }
            },
            None if required => {
                self.error(
                    "missing_required_field",
                    format!("Missing required field `{field}`."),
                    &loc.field(field),
                    "boolean",
                    None,
                    format!("Add `{field}: true` or `{field}: false`."),
                );
                None
            }
            None => None,
        }
    }

    fn string_or_number_field(
        &mut self,
        obj: &Map<String, Value>,
        field: &str,
        loc: &Location,
        required: bool,
    ) -> Option<String> {
        match obj.get(field) {
            Some(value) if value.is_null() && !required => None,
            Some(value) if value.is_string() => value.as_str().map(str::to_string),
            Some(value) if value.is_number() => Some(value.to_string()),
            Some(value) => {
                self.error(
                    "invalid_field_type",
                    format!("Field `{field}` must be a string or number."),
                    &loc.field(field),
                    "string or number",
                    Some(value_kind(value).to_string()),
                    format!("Set `{field}` to a port number or string."),
                );
                None
            }
            None if required => {
                self.error(
                    "missing_required_field",
                    format!("Missing required field `{field}`."),
                    &loc.field(field),
                    "string or number",
                    None,
                    format!("Add `{field}: <port>`."),
                );
                None
            }
            None => None,
        }
    }

    fn enum_field(
        &mut self,
        obj: &Map<String, Value>,
        field: &str,
        loc: &Location,
        allowed: &'static [&'static str],
        required: bool,
    ) -> Option<String> {
        let value = self.string_field(obj, field, loc, required)?;
        if !allowed.contains(&value.as_str()) {
            self.error(
                "invalid_enum_value",
                format!("Field `{field}` has unsupported value `{value}`."),
                &loc.field(field),
                allowed_values(allowed),
                Some(value.clone()),
                format!("Set `{field}` to one of: {}.", allowed.join(", ")),
            );
        }
        Some(value)
    }

    fn string_array_field(
        &mut self,
        obj: &Map<String, Value>,
        field: &str,
        loc: &Location,
        required: bool,
    ) -> Vec<String> {
        let Some(value) = obj.get(field) else {
            if required {
                self.error(
                    "missing_required_field",
                    format!("Missing required field `{field}`."),
                    &loc.field(field),
                    "array of strings",
                    None,
                    format!("Add `{field}: []` or a list of strings."),
                );
            }
            return vec![];
        };
        if value.is_null() && !required {
            return vec![];
        }
        let field_loc = loc.field(field);
        let Some(items) = self.expect_array(value, &field_loc, "array of strings") else {
            return vec![];
        };
        let mut result = vec![];
        for (idx, item) in items.iter().enumerate() {
            match item.as_str() {
                Some(item) => result.push(item.to_string()),
                None => self.error(
                    "invalid_field_type",
                    format!("Entry in `{field}` must be a string."),
                    &field_loc.index(idx),
                    "string",
                    Some(value_kind(item).to_string()),
                    "Use plain string entries in the list.",
                ),
            }
        }
        result
    }

    fn effective_entity_name(&self, obj: &Map<String, Value>, raw_name: &str) -> String {
        obj.get("pascal_1")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| {
                if is_pascal_case(raw_name) {
                    raw_name.to_string()
                } else {
                    to_pascal_case(raw_name)
                }
            })
    }

    fn enum_exists(&self, schema_name: &str, enum_type_name: &str) -> bool {
        self.enum_types
            .contains_key(&enum_key(schema_name, enum_type_name))
    }

    fn fragment_suggestions(&self) -> String {
        let mut names: Vec<&str> = self.fragments.keys().map(String::as_str).collect();
        names.sort();
        names.into_iter().take(8).collect::<Vec<_>>().join(", ")
    }

    fn error(
        &mut self,
        code: &'static str,
        message: String,
        loc: &Location,
        expected: impl Into<String>,
        actual: Option<String>,
        suggested_fix: impl Into<String>,
    ) {
        self.issue("error", code, message, loc, expected, actual, suggested_fix);
    }

    fn warn(
        &mut self,
        code: &'static str,
        message: String,
        loc: &Location,
        expected: impl Into<String>,
        actual: Option<String>,
        suggested_fix: impl Into<String>,
    ) {
        self.issue(
            "warning",
            code,
            message,
            loc,
            expected,
            actual,
            suggested_fix,
        );
    }

    fn issue(
        &mut self,
        severity: &'static str,
        code: &'static str,
        message: String,
        loc: &Location,
        expected: impl Into<String>,
        actual: Option<String>,
        suggested_fix: impl Into<String>,
    ) {
        self.issues.push(ValidationIssue {
            severity,
            code,
            message,
            file: loc.file.display().to_string(),
            path: loc.path.clone(),
            schema_name: loc.schema_name.clone(),
            entity_name: loc.entity_name.clone(),
            property_name: loc.property_name.clone(),
            expected: expected.into(),
            actual,
            suggested_fix: suggested_fix.into(),
        });
    }
}

impl Location {
    fn root(file: PathBuf) -> Self {
        Self {
            file,
            path: "$".to_string(),
            schema_name: None,
            entity_name: None,
            property_name: None,
        }
    }

    fn field(&self, field: &str) -> Self {
        let mut loc = self.clone();
        loc.path = format!("{}.{}", self.path, field);
        loc
    }

    fn index(&self, idx: usize) -> Self {
        let mut loc = self.clone();
        loc.path = format!("{}[{idx}]", self.path);
        loc
    }

    fn line(&self, line: usize) -> Self {
        let mut loc = self.clone();
        loc.path = format!("{}:line[{line}]", self.path);
        loc
    }

    fn with_schema(&self, schema_name: String) -> Self {
        let mut loc = self.clone();
        loc.schema_name = Some(schema_name);
        loc
    }

    fn with_entity(&self, entity_name: String) -> Self {
        let mut loc = self.clone();
        loc.entity_name = Some(entity_name);
        loc
    }

    fn with_property(&self, property_name: String) -> Self {
        let mut loc = self.clone();
        loc.property_name = Some(property_name);
        loc
    }
}

#[derive(Clone, Copy)]
enum PropertyShapeMode {
    Fragment,
    FacetProperty,
    EntityProperty,
}

impl PropertyShapeMode {
    fn requires_name(self) -> bool {
        matches!(self, Self::FacetProperty | Self::EntityProperty)
    }
}

fn collect_yaml_files(
    dir: &Path,
    recursive: bool,
    skip_res: bool,
    files: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && recursive {
            collect_yaml_files(&path, recursive, skip_res, files)?;
        } else if is_yaml_file(&path, skip_res) {
            files.push(path);
        }
    }
    Ok(())
}

fn collect_template_files(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_template_files(&path, files)?;
        } else if is_template_file(&path) {
            files.push(path);
        }
    }
    Ok(())
}

fn is_yaml_file(path: &Path, skip_res: bool) -> bool {
    if !path.is_file() {
        return false;
    }
    let is_yaml = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("yaml"))
        .unwrap_or(false);
    let is_res = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem == "_res")
        .unwrap_or(false);
    is_yaml && !(skip_res && is_res)
}

fn is_template_file(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.eq_ignore_ascii_case("j2"))
            .unwrap_or(false)
}

fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn value_preview(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Number(_) | Value::Bool(_) | Value::Null => value.to_string(),
        Value::Array(_) => "array".to_string(),
        Value::Object(_) => "object".to_string(),
    }
}

fn allowed_values(values: &[&str]) -> String {
    format!("one of: {}", values.join(", "))
}

fn provider_routine_identifier_is_safe(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn is_regulated_classification(classification: Option<&str>) -> bool {
    classification
        .map(|classification| REGULATED_DATA_CLASSIFICATIONS.contains(&classification))
        .unwrap_or(false)
}

fn entity_key(schema_name: &str, entity_name: &str) -> String {
    format!("{schema_name}.{entity_name}")
}

fn enum_key(schema_name: &str, enum_name: &str) -> String {
    format!("{schema_name}.{enum_name}")
}

fn bool_from_json(value: Option<&Value>, field: &str) -> bool {
    value
        .and_then(|value| value.get(field))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn string_from_json(value: Option<&Value>, field: &str) -> Option<String> {
    value
        .and_then(|value| value.get(field))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn read_property_data_classification(value: Option<&Value>) -> Option<String> {
    let obj = value?.as_object()?;
    for field in ["classification", "data_classification"] {
        if let Some(classification) = obj.get(field).and_then(Value::as_str) {
            if DATA_CLASSIFICATIONS.contains(&classification) {
                return Some(classification.to_string());
            }
        }
    }
    let classification = obj
        .get("audit")
        .and_then(Value::as_object)
        .and_then(|audit| audit.get("classification"))
        .and_then(Value::as_str)?;
    if DATA_CLASSIFICATIONS.contains(&classification) {
        Some(classification.to_string())
    } else {
        None
    }
}

fn is_seed_native_property(prop: &PropertyInfo) -> bool {
    !matches!(
        prop.data_type.as_deref(),
        Some("NavToOne" | "NavToMany" | "ManyToMany" | "Object" | "ObjectArray")
    ) && prop.nav_by_fk_property.is_none()
        && prop.many_to_many_property.is_none()
        && prop.nested_entity_type.is_none()
}

fn is_generated_seed_value(prop: &PropertyInfo) -> bool {
    prop.is_concurrency_control
        || prop
            .computed
            .as_deref()
            .map(|computed| computed != "None")
            .unwrap_or(false)
}

fn schema_meta_external_read_only(value: Option<&Value>) -> bool {
    let Some(meta) = value.and_then(Value::as_object) else {
        return false;
    };
    match meta.get("storage") {
        Some(Value::String(mode)) => mode == "external_read_only",
        Some(Value::Object(storage)) => {
            storage
                .get("external_read_only")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || storage
                    .get("mode")
                    .and_then(Value::as_str)
                    .is_some_and(|mode| mode == "external_read_only")
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, fs, time::SystemTime};

    #[test]
    fn managed_data_source_requires_classification() {
        let (workspace, root) = temp_workspace("managed-ds-classification");
        write_data_sources(
            &workspace,
            r#"
- name: pg_primary
  data_source_type: PostgreSQL
  is_system_schema_host: true
  environments:
  - name: production
    db_host: db.example.com
    db_name: app
    db_port: 5432
    security_profile: managed
    tls_mode: verify_full
"#,
        );

        let mut validator = Validator::new(&workspace);
        validator.validate_data_sources();

        assert!(
            validator
                .issues
                .iter()
                .any(|issue| issue.code == "missing_data_classification"
                    && issue.path == "$[0].meta"),
            "expected missing data classification issue, got {:?}",
            validator.issues
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn managed_schema_inherits_data_source_classification() {
        let (workspace, root) = temp_workspace("schema-classification-inheritance");
        write_data_sources(
            &workspace,
            r#"
- name: pg_primary
  data_source_type: PostgreSQL
  is_system_schema_host: true
  meta:
    classification: confidential
  environments:
  - name: production
    db_host: db.example.com
    db_name: app
    db_port: 5432
    security_profile: managed
    tls_mode: verify_full
"#,
        );
        write_schema(
            &workspace,
            "crm",
            r#"
id: 11111111-1111-4111-8111-111111111111
name: crm
description: CRM schema
data_source_name: pg_primary
"#,
        );
        write_entity(
            &workspace,
            "crm",
            "account.yaml",
            r#"
- id: 22222222-2222-4222-8222-222222222222
  name: Account
  is_table: true
  props:
  - name: id
    data_type: Uuid
    is_key: true
"#,
        );

        let mut validator = Validator::new(&workspace);
        validator.validate_data_sources();
        validator.validate_schemas();

        assert!(
            !validator
                .issues
                .iter()
                .any(|issue| issue.code == "missing_data_classification"),
            "did not expect missing classification when inherited from data source: {:?}",
            validator.issues
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn unknown_data_classification_fails() {
        let (workspace, root) = temp_workspace("unknown-classification");
        write_data_sources(
            &workspace,
            r#"
- name: pg_primary
  data_source_type: PostgreSQL
  is_system_schema_host: true
  meta:
    classification: unknown
  environments:
  - name: local
    db_host: localhost
    db_name: app
    db_port: 5432
    security_profile: local_dev
    tls_mode: disabled
"#,
        );

        let mut validator = Validator::new(&workspace);
        validator.validate_data_sources();

        assert!(
            validator
                .issues
                .iter()
                .any(|issue| issue.code == "invalid_data_classification"
                    && issue.path == "$[0].meta.classification"),
            "expected invalid data classification issue, got {:?}",
            validator.issues
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn fabric_sql_analytics_accepts_entra_auth_metadata() {
        let (workspace, root) = temp_workspace("fabric-sql-analytics-auth");
        write_data_sources(
            &workspace,
            r#"
- name: pg_primary
  data_source_type: PostgreSQL
  is_system_schema_host: true
  environments:
  - name: local
    db_host: localhost
    db_name: app
    db_port: 5432
    security_profile: local_dev
    tls_mode: disabled

- name: lakehouse_reporting
  data_source_type: FabricSqlAnalytics
  description: Microsoft Fabric SQL analytics endpoint for reporting reads
  meta:
    classification: confidential
  environments:
  - name: production
    db_host: fabric-host.example.invalid
    db_name: fabric_database
    db_port: 1433
    security_profile: managed
    tls_mode: verify_full
    auth_mode: entra_client_credentials
    entra_tenant_id: tenant-guid
    entra_token_scope: https://database.windows.net/.default
"#,
        );

        let mut validator = Validator::new(&workspace);
        validator.validate_data_sources();

        assert!(
            validator.issues.is_empty(),
            "expected FabricSqlAnalytics data-source auth metadata to validate, got {:?}",
            validator.issues
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn fabric_sql_analytics_rejects_non_client_credentials_auth_mode() {
        let (workspace, root) = temp_workspace("fabric-sql-analytics-sql-password");
        for auth_mode in ["sql_password", "entra_access_token"] {
            write_data_sources(
                &workspace,
                &format!(
                    r#"
- name: pg_primary
  data_source_type: PostgreSQL
  is_system_schema_host: true
  environments:
  - name: local
    db_host: localhost
    db_name: app
    db_port: 5432
    security_profile: local_dev
    tls_mode: disabled

- name: lakehouse_reporting
  data_source_type: FabricSqlAnalytics
  meta:
    classification: confidential
  environments:
  - name: production
    db_host: fabric-host.example.invalid
    db_name: fabric_database
    db_port: 1433
    security_profile: managed
    tls_mode: verify_full
    auth_mode: {auth_mode}
"#,
                ),
            );

            let mut validator = Validator::new(&workspace);
            validator.validate_data_sources();

            assert!(
                validator
                    .issues
                    .iter()
                    .any(|issue| issue.code == "invalid_fabric_sql_auth_mode"
                        && issue.path == "$[1].environments[0].auth_mode"),
                "expected FabricSqlAnalytics auth issue for {auth_mode}, got {:?}",
                validator.issues
            );
        }

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn mssql_ntlm_allows_ip_db_host() {
        let (workspace, root) = temp_workspace("mssql-ntlm-ip");
        write_data_sources(
            &workspace,
            r#"
- name: mssql_primary
  data_source_type: MsSqlServer
  environments:
  - name: local
    db_host: 10.0.0.5
    db_name: app
    db_port: 1433
    security_profile: local_dev
    tls_mode: disabled
    auth_mode: ntlm
"#,
        );

        let mut validator = Validator::new(&workspace);
        validator.validate_data_sources();

        assert!(
            !validator.issues.iter().any(|issue| {
                issue.path.contains("db_host") && issue.code == "invalid_ntlm_db_host"
            }),
            "ntlm auth_mode should allow IP db_host, got {:?}",
            validator.issues
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn fabric_sql_analytics_rejects_resource_style_token_scope() {
        let (workspace, root) = temp_workspace("fabric-sql-analytics-token-scope");
        write_data_sources(
            &workspace,
            r#"
- name: pg_primary
  data_source_type: PostgreSQL
  is_system_schema_host: true
  environments:
  - name: local
    db_host: localhost
    db_name: app
    db_port: 5432
    security_profile: local_dev
    tls_mode: disabled

- name: fabric_reporting
  data_source_type: FabricSqlAnalytics
  meta:
    classification: confidential
  environments:
  - name: production
    db_host: fabric-host.example.invalid
    db_name: fabric_database
    db_port: 1433
    security_profile: managed
    tls_mode: verify_full
    auth_mode: entra_client_credentials
    entra_token_scope: https://database.windows.net/
"#,
        );

        let mut validator = Validator::new(&workspace);
        validator.validate_data_sources();

        assert!(
            validator
                .issues
                .iter()
                .any(|issue| issue.code == "invalid_fabric_sql_token_scope"
                    && issue.path == "$[1].environments[0].entra_token_scope"),
            "expected FabricSqlAnalytics token-scope issue, got {:?}",
            validator.issues
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn mongodb_accepts_seed_list_connection_uri_without_embedded_credentials() {
        let (workspace, root) = temp_workspace("mongodb-seed-list-uri");
        write_data_sources(
            &workspace,
            r#"
- name: mongo_primary
  data_source_type: MongoDB
  meta:
    classification: confidential
  environments:
  - name: production
    db_host: mongodb://mongo-a.example.net:27017,mongo-b.example.net:27018/admin?replicaSet=rs0
    db_name: app
    db_port: 27017
    security_profile: managed
    tls_mode: verify_full
"#,
        );

        let mut validator = Validator::new(&workspace);
        validator.validate_data_sources();

        assert!(
            validator.issues.is_empty(),
            "expected MongoDB seed-list URI to validate, got {:?}",
            validator.issues
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn mongodb_rejects_connection_uri_with_embedded_credentials() {
        let (workspace, root) = temp_workspace("mongodb-uri-credentials");
        write_data_sources(
            &workspace,
            r#"
- name: mongo_primary
  data_source_type: MongoDB
  meta:
    classification: confidential
  environments:
  - name: production
    db_host: mongodb://user:secret@mongo.example.net/app
    db_name: app
    db_port: 27017
    security_profile: managed
    tls_mode: verify_full
"#,
        );

        let mut validator = Validator::new(&workspace);
        validator.validate_data_sources();

        assert!(
            validator
                .issues
                .iter()
                .any(|issue| issue.code == "invalid_mongodb_connection_uri"
                    && issue.path == "$[0].environments[0].db_host"),
            "expected embedded credential URI issue, got {:?}",
            validator.issues
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn mongodb_rejects_srv_uri_with_port_or_seed_list() {
        let (workspace, root) = temp_workspace("mongodb-srv-shape");
        for db_host in [
            "mongodb+srv://cluster.example.net:27017/app",
            "mongodb+srv://cluster-a.example.net,cluster-b.example.net/app",
        ] {
            write_data_sources(
                &workspace,
                &format!(
                    r#"
- name: mongo_primary
  data_source_type: MongoDB
  meta:
    classification: confidential
  environments:
  - name: production
    db_host: {db_host}
    db_name: app
    db_port: 27017
    security_profile: managed
    tls_mode: verify_full
"#,
                ),
            );

            let mut validator = Validator::new(&workspace);
            validator.validate_data_sources();

            assert!(
                validator
                    .issues
                    .iter()
                    .any(|issue| issue.code == "invalid_mongodb_connection_uri"
                        && issue.path == "$[0].environments[0].db_host"),
                "expected SRV shape issue for {db_host}, got {:?}",
                validator.issues
            );
        }

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn fabric_sql_analytics_cannot_host_system_schema() {
        let (workspace, root) = temp_workspace("fabric-sql-analytics-system-host");
        write_data_sources(
            &workspace,
            r#"
- name: lakehouse_reporting
  data_source_type: FabricSqlAnalytics
  is_system_schema_host: true
  meta:
    classification: confidential
  environments:
  - name: production
    db_host: fabric-host.example.invalid
    db_name: fabric_database
    db_port: 1433
    security_profile: managed
    tls_mode: verify_full
    auth_mode: entra_client_credentials
"#,
        );

        let mut validator = Validator::new(&workspace);
        validator.validate_data_sources();

        assert!(
            validator
                .issues
                .iter()
                .any(|issue| issue.code == "invalid_system_schema_host"
                    && issue.path == "$[0].is_system_schema_host"),
            "expected FabricSqlAnalytics system-host issue, got {:?}",
            validator.issues
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn neo4j_cannot_host_generated_entity_schema() {
        let (workspace, root) = temp_workspace("neo4j-schema-host");
        write_data_sources(
            &workspace,
            r#"
- name: neo4j_graph
  data_source_type: Neo4j
  environments:
  - name: local
    db_host: localhost
    db_name: neo4j
    db_port: 7687
    security_profile: local_dev
    tls_mode: disabled
"#,
        );
        write_schema(
            &workspace,
            "graph",
            r#"
id: 11111111-1111-4111-8111-111111111111
name: graph
description: Graph schema
data_source_name: neo4j_graph
"#,
        );

        let mut validator = Validator::new(&workspace);
        validator.validate_data_sources();
        validator.validate_schemas();

        assert!(
            validator
                .issues
                .iter()
                .any(|issue| issue.code == "schema_uses_graph_read_provider"
                    && issue.path == "$.data_source_name"),
            "expected schema_uses_graph_read_provider issue, got {:?}",
            validator.issues
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn external_api_cannot_host_generated_entity_schema() {
        for provider in [
            "ServiceNow",
            "Workday",
            "Icims",
            "Salesforce",
            "Anaplan",
            "OracleFinancials",
        ] {
            let (workspace, root) = temp_workspace(&format!(
                "external-api-schema-host-{}",
                provider.to_ascii_lowercase()
            ));
            write_data_sources(
                &workspace,
                &format!(
                    r#"
- name: external_primary
  data_source_type: {provider}
  environments:
  - name: local
    db_host: external.example.invalid
    db_name: external
    db_port: 443
    security_profile: local_dev
    tls_mode: disabled
"#
                ),
            );
            write_schema(
                &workspace,
                "external_schema_v1",
                r#"
id: 11111111-1111-4111-8111-111111111112
name: external_schema_v1
description: External API schema
data_source_name: external_primary
"#,
            );

            let mut validator = Validator::new(&workspace);
            validator.validate_data_sources();
            validator.validate_schemas();

            assert!(
                validator
                    .issues
                    .iter()
                    .any(|issue| issue.code == "schema_uses_external_api_provider"
                        && issue.path == "$.data_source_name"),
                "expected schema_uses_external_api_provider issue for {provider}, got {:?}",
                validator.issues
            );

            let _ = fs::remove_dir_all(root);
        }
    }

    #[test]
    fn external_api_cannot_host_system_schema_metadata() {
        for provider in [
            "ServiceNow",
            "Workday",
            "Icims",
            "Salesforce",
            "Anaplan",
            "OracleFinancials",
        ] {
            let (workspace, root) = temp_workspace(&format!(
                "external-api-system-host-{}",
                provider.to_ascii_lowercase()
            ));
            write_data_sources(
                &workspace,
                &format!(
                    r#"
- name: external_primary
  data_source_type: {provider}
  is_system_schema_host: true
  environments:
  - name: local
    db_host: external.example.invalid
    db_name: external
    db_port: 443
    security_profile: local_dev
    tls_mode: disabled
"#
                ),
            );

            let mut validator = Validator::new(&workspace);
            validator.validate_data_sources();

            assert!(
                validator
                    .issues
                    .iter()
                    .any(|issue| issue.code == "invalid_system_schema_host"
                        && issue.path == "$[0].is_system_schema_host"),
                "expected invalid_system_schema_host issue for {provider}, got {:?}",
                validator.issues
            );

            let _ = fs::remove_dir_all(root);
        }
    }

    #[test]
    fn stale_kafka_ingress_config_fails_when_topology_disables_kafka() {
        let (workspace, root) = temp_workspace("stale-kafka-config");
        write_manifest_inputs(&workspace, false);
        let kafka_config = workspace
            .app_root
            .join("backend/config/generated/ingress/kafka.yaml");
        fs::create_dir_all(kafka_config.parent().expect("config parent"))
            .expect("create ingress config dir");
        fs::write(&kafka_config, "enabled: true\nconsumers: []\n").expect("write stale config");

        let mut validator = Validator::new(&workspace);
        validator.validate_app_manifest();

        assert!(
            validator.issues.iter().any(
                |issue| issue.code == "stale_kafka_ingress_config" && issue.severity == "error"
            ),
            "expected stale Kafka config issue, got {:?}",
            validator.issues
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn enabled_kafka_topology_warns_until_runtime_config_is_scaffolded() {
        let (workspace, root) = temp_workspace("missing-kafka-config");
        write_manifest_inputs(&workspace, true);

        let mut validator = Validator::new(&workspace);
        validator.validate_app_manifest();

        assert!(
            validator
                .issues
                .iter()
                .any(|issue| issue.code == "missing_kafka_ingress_config"
                    && issue.severity == "warning"),
            "expected missing Kafka config warning, got {:?}",
            validator.issues
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sync_descriptor_validation_reports_contract_issues() {
        let (workspace, root) = temp_workspace("sync-descriptor-contract");
        write_sync_descriptor(
            &workspace,
            "salesforce_to_pipeline.yaml",
            r#"
name: salesforce_to_pipeline
mode: full_refresh
objects: []
"#,
        );

        let mut validator = Validator::new(&workspace);
        validator.validate_sync_descriptors();

        let codes: HashSet<&str> = validator.issues.iter().map(|issue| issue.code).collect();
        for code in [
            "missing_source_schema",
            "missing_target_schema",
            "invalid_sync_mode",
            "missing_schedule",
            "missing_freshness_slo",
            "missing_sync_objects",
            "missing_sync_governance",
        ] {
            assert!(
                codes.contains(code),
                "expected sync descriptor issue {code}, got {:?}",
                validator.issues
            );
        }

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn valid_sync_descriptor_references_app_owned_target_schema() {
        let (workspace, root) = temp_workspace("valid-sync-descriptor");
        write_data_sources(
            &workspace,
            r#"
- name: pg_primary
  data_source_type: PostgreSQL
  environments:
  - name: local
    db_host: localhost
    db_name: app
    db_port: 5432
    security_profile: local_dev
    tls_mode: disabled
"#,
        );
        write_schema(
            &workspace,
            "pipeline",
            r#"
id: 11111111-1111-4111-8111-111111111111
name: pipeline
description: App-owned SaaS projection schema
data_source_name: pg_primary
"#,
        );
        write_entity(
            &workspace,
            "pipeline",
            "account.yaml",
            r#"
- id: 22222222-2222-4222-8222-222222222222
  name: Account
  is_table: true
  props:
  - name: id
    data_type: String
    is_key: true
"#,
        );
        write_sync_descriptor(
            &workspace,
            "salesforce_to_pipeline.yaml",
            r#"
name: salesforce_to_pipeline
source_schema: salesforce_crm_v1
target_schema: pipeline
mode: incremental_watermark
schedule: "*/5 * * * *"
freshness_slo: PT10M
objects:
- source: Account
  target: Account
  key: Id
  watermark: SystemModstamp
  provenance:
    projection_of: salesforce_crm_v1.Account
    source_key_field: Id
    source_watermark_field: SystemModstamp
    source_origin_field: AppFrameworkOrigin__c
  echo_loop:
    policy: reject_self_origin
    local_origin: appfw:pipeline:salesforce_to_pipeline
    source_origin_field: AppFrameworkOrigin__c
governance:
  classification: internal
  deletion: ignore
  source_drift: warn
"#,
        );

        let mut validator = Validator::new(&workspace);
        validator.validate_data_sources();
        validator.validate_schemas();
        validator.validate_sync_descriptors();

        assert!(
            validator.issues.is_empty(),
            "expected valid sync descriptor, got {:?}",
            validator.issues
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sync_descriptor_rejects_unknown_and_external_read_only_targets() {
        let (workspace, root) = temp_workspace("sync-descriptor-target");
        write_data_sources(
            &workspace,
            r#"
- name: fabric_reporting
  data_source_type: FabricSqlAnalytics
  meta:
    classification: confidential
  environments:
  - name: local
    db_host: fabric.example.invalid
    db_name: lakehouse
    db_port: 1433
    security_profile: local_dev
    tls_mode: disabled
    auth_mode: entra_client_credentials
    entra_token_scope: https://database.windows.net/.default
"#,
        );
        write_schema(
            &workspace,
            "fabric_read_model",
            r#"
id: 11111111-1111-4111-8111-111111111111
name: fabric_read_model
description: External read model
data_source_name: fabric_reporting
meta:
  classification: confidential
  storage:
    mode: external_read_only
"#,
        );
        write_sync_descriptor(
            &workspace,
            "unknown_target.yaml",
            r#"
name: unknown_target
source_schema: salesforce_crm_v1
target_schema: missing_projection
mode: incremental_watermark
schedule: "*/5 * * * *"
freshness_slo: PT10M
objects:
- source: Account
  target: Account
  key: Id
  watermark: SystemModstamp
  provenance:
    projection_of: salesforce_crm_v1.Account
    source_key_field: Id
    source_watermark_field: SystemModstamp
    source_origin_field: AppFrameworkOrigin__c
  echo_loop:
    policy: reject_self_origin
    local_origin: appfw:pipeline:unknown_target
    source_origin_field: AppFrameworkOrigin__c
governance:
  classification: internal
  deletion: ignore
  source_drift: warn
"#,
        );
        write_sync_descriptor(
            &workspace,
            "external_target.yaml",
            r#"
name: external_target
source_schema: salesforce_crm_v1
target_schema: fabric_read_model
mode: incremental_watermark
schedule: "*/5 * * * *"
freshness_slo: PT10M
objects:
- source: Account
  target: Account
  key: Id
  watermark: SystemModstamp
  provenance:
    projection_of: salesforce_crm_v1.Account
    source_key_field: Id
    source_watermark_field: SystemModstamp
    source_origin_field: AppFrameworkOrigin__c
  echo_loop:
    policy: reject_self_origin
    local_origin: appfw:fabric_read_model:external_target
    source_origin_field: AppFrameworkOrigin__c
governance:
  classification: internal
  deletion: ignore
  source_drift: warn
"#,
        );

        let mut validator = Validator::new(&workspace);
        validator.validate_data_sources();
        validator.validate_schemas();
        validator.validate_sync_descriptors();

        assert!(
            validator
                .issues
                .iter()
                .any(|issue| issue.code == "sync_descriptor_unknown_target_schema"),
            "expected unknown target schema issue, got {:?}",
            validator.issues
        );
        assert!(
            validator
                .issues
                .iter()
                .any(|issue| issue.code == "sync_descriptor_target_schema_external_read_only"),
            "expected external read-only target schema issue, got {:?}",
            validator.issues
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sync_descriptor_rejects_name_mismatches_duplicates_and_self_syncs() {
        let (workspace, root) = temp_workspace("sync-descriptor-identity");
        write_data_sources(
            &workspace,
            r#"
- name: pg_primary
  data_source_type: PostgreSQL
  environments:
  - name: local
    db_host: localhost
    db_name: app
    db_port: 5432
    security_profile: local_dev
    tls_mode: disabled
"#,
        );
        write_schema(
            &workspace,
            "pipeline",
            r#"
id: 11111111-1111-4111-8111-111111111111
name: pipeline
description: Projection schema
data_source_name: pg_primary
"#,
        );
        for file_name in ["first.yaml", "second.yaml"] {
            write_sync_descriptor(
                &workspace,
                file_name,
                r#"
name: duplicate_sync
source_schema: pipeline
target_schema: pipeline
mode: incremental_watermark
schedule: "*/5 * * * *"
freshness_slo: PT10M
objects:
- source: Account
  target: Account
  key: Id
  watermark: SystemModstamp
  provenance:
    projection_of: pipeline.Account
    source_key_field: Id
    source_watermark_field: SystemModstamp
    source_origin_field: AppFrameworkOrigin__c
  echo_loop:
    policy: reject_self_origin
    local_origin: appfw:pipeline:duplicate_sync
    source_origin_field: AppFrameworkOrigin__c
governance:
  classification: internal
  deletion: ignore
  source_drift: warn
"#,
            );
        }

        let mut validator = Validator::new(&workspace);
        validator.validate_data_sources();
        validator.validate_schemas();
        validator.validate_sync_descriptors();

        for code in [
            "sync_descriptor_name_mismatch",
            "duplicate_sync_descriptor",
            "sync_descriptor_source_matches_target",
            "sync_descriptor_unknown_target_entity",
        ] {
            assert!(
                validator.issues.iter().any(|issue| issue.code == code),
                "expected sync descriptor issue {code}, got {:?}",
                validator.issues
            );
        }

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn provider_routine_custom_method_validates_safe_binding() {
        let (workspace, root) = temp_workspace("provider-routine-valid");
        write_data_sources(
            &workspace,
            r#"
- name: pg_primary
  data_source_type: PostgreSQL
  is_system_schema_host: true
  environments:
  - name: local
    db_host: localhost
    db_name: app
    db_port: 5432
    security_profile: local_dev
    tls_mode: disabled
"#,
        );
        write_schema(
            &workspace,
            "crm",
            r#"
id: 11111111-1111-4111-8111-111111111111
name: crm
description: CRM schema
data_source_name: pg_primary
"#,
        );
        write_entity(
            &workspace,
            "crm",
            "account.yaml",
            r#"
- id: 22222222-2222-4222-8222-222222222222
  name: Account
  is_table: true
  custom_methods:
  - name: account_health_from_provider
    kind: Query
    args:
    - name: account_id
      arg_type: String
    return_type: serde_json::Value
    provider_routine:
      kind: Function
      returns: One
      data_source: pg_primary
      schema: crm
      name: account_health_score
  props:
  - name: id
    data_type: Uuid
    is_key: true
"#,
        );

        let mut validator = Validator::new(&workspace);
        validator.validate_data_sources();
        validator.validate_schemas();

        assert!(
            validator.issues.is_empty(),
            "expected valid provider routine binding, got {:?}",
            validator.issues
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn external_read_only_schema_rejects_seed_files() {
        let (workspace, root) = temp_workspace("external-read-only-seeds");
        write_data_sources(
            &workspace,
            r#"
- name: fabric_reporting
  data_source_type: FabricSqlAnalytics
  meta:
    classification: confidential
  environments:
  - name: local
    db_host: fabric.example.com
    db_name: fabric_database
    db_port: 1433
    security_profile: managed
    tls_mode: verify_full
    auth_mode: entra_client_credentials
    entra_token_scope: https://database.windows.net/.default
"#,
        );
        write_schema(
            &workspace,
            "finance",
            r#"
id: 11111111-1111-4111-8111-111111111111
name: finance
description: Finance reporting schema
data_source_name: fabric_reporting
meta:
  classification: confidential
  storage:
    mode: external_read_only
"#,
        );
        write_entity(
            &workspace,
            "finance",
            "office.yaml",
            r#"
- id: 22222222-2222-4222-8222-222222222222
  name: Office
  is_table: true
  standard_methods: [FindById, GetAll, Query]
  props:
  - name: id
    data_type: String
    is_key: true
"#,
        );
        write_seed(
            &workspace,
            "finance",
            "office.yaml",
            r#"
- entity_type: Office
  columns: [id]
  records:
  - id: demo
"#,
        );

        let mut validator = Validator::new(&workspace);
        validator.validate_data_sources();
        validator.validate_schemas();
        validator.validate_seed_configs();

        assert!(
            validator.issues.iter().any(|issue| {
                issue.code == "external_read_only_seed" && issue.severity == "error"
            }),
            "expected external read-only seed issue, got {:?}",
            validator.issues
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn external_read_only_schema_rejects_mutation_methods() {
        let (workspace, root) = temp_workspace("external-read-only-mutations");
        write_data_sources(
            &workspace,
            r#"
- name: fabric_reporting
  data_source_type: FabricSqlAnalytics
  meta:
    classification: confidential
  environments:
  - name: local
    db_host: fabric.example.com
    db_name: fabric_database
    db_port: 1433
    security_profile: managed
    tls_mode: verify_full
    auth_mode: entra_client_credentials
    entra_token_scope: https://database.windows.net/.default
"#,
        );
        write_schema(
            &workspace,
            "finance",
            r#"
id: 11111111-1111-4111-8111-111111111111
name: finance
description: Finance reporting schema
data_source_name: fabric_reporting
meta:
  classification: confidential
  storage:
    mode: external_read_only
"#,
        );
        write_entity(
            &workspace,
            "finance",
            "office.yaml",
            r#"
- id: 22222222-2222-4222-8222-222222222222
  name: Office
  is_table: true
  standard_methods: [FindById, Query, Create]
  custom_methods:
  - name: refresh_office_rollup
    kind: Mutation
    args: []
    return_type: serde_json::Value
  props:
  - name: id
    data_type: String
    is_key: true
"#,
        );

        let mut validator = Validator::new(&workspace);
        validator.validate_data_sources();
        validator.validate_schemas();

        assert!(
            validator.issues.iter().any(|issue| {
                issue.code == "external_read_only_standard_method" && issue.severity == "error"
            }),
            "expected external read-only standard method issue, got {:?}",
            validator.issues
        );
        assert!(
            validator.issues.iter().any(|issue| {
                issue.code == "external_read_only_custom_method" && issue.severity == "error"
            }),
            "expected external read-only custom method issue, got {:?}",
            validator.issues
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn provider_routine_custom_method_allows_procedure_return_payload() {
        let (workspace, root) = temp_workspace("provider-routine-procedure-return");
        write_data_sources(
            &workspace,
            r#"
- name: pg_primary
  data_source_type: PostgreSQL
  is_system_schema_host: true
  environments:
  - name: local
    db_host: localhost
    db_name: app
    db_port: 5432
    security_profile: local_dev
    tls_mode: disabled
"#,
        );
        write_schema(
            &workspace,
            "crm",
            r#"
id: 11111111-1111-4111-8111-111111111111
name: crm
description: CRM schema
data_source_name: pg_primary
"#,
        );
        write_entity(
            &workspace,
            "crm",
            "account.yaml",
            r#"
- id: 22222222-2222-4222-8222-222222222222
  name: Account
  is_table: true
  custom_methods:
  - name: refresh_account_health_stored_procedure
    kind: Mutation
    args:
    - name: account_id
      arg_type: String
    - name: health_score
      arg_type: f64
    return_type: serde_json::Value
    provider_routine:
      kind: Procedure
      returns: One
      schema: crm
      name: refresh_account_health_stored_procedure
  props:
  - name: id
    data_type: Uuid
    is_key: true
"#,
        );

        let mut validator = Validator::new(&workspace);
        validator.validate_data_sources();
        validator.validate_schemas();

        assert!(
            validator.issues.is_empty(),
            "expected procedure return payload binding to validate, got {:?}",
            validator.issues
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn provider_routine_custom_method_rejects_unsafe_identifier() {
        let (workspace, root) = temp_workspace("provider-routine-unsafe");
        write_data_sources(
            &workspace,
            r#"
- name: pg_primary
  data_source_type: PostgreSQL
  is_system_schema_host: true
  environments:
  - name: local
    db_host: localhost
    db_name: app
    db_port: 5432
    security_profile: local_dev
    tls_mode: disabled
"#,
        );
        write_schema(
            &workspace,
            "crm",
            r#"
id: 11111111-1111-4111-8111-111111111111
name: crm
description: CRM schema
data_source_name: pg_primary
"#,
        );
        write_entity(
            &workspace,
            "crm",
            "account.yaml",
            r#"
- id: 22222222-2222-4222-8222-222222222222
  name: Account
  is_table: true
  custom_methods:
  - name: account_health_from_provider
    kind: Query
    args:
    - name: account_id
      arg_type: String
    return_type: serde_json::Value
    provider_routine:
      kind: Function
      returns: One
      schema: crm
      name: account_health;drop
  props:
  - name: id
    data_type: Uuid
    is_key: true
"#,
        );

        let mut validator = Validator::new(&workspace);
        validator.validate_data_sources();
        validator.validate_schemas();

        assert!(
            validator
                .issues
                .iter()
                .any(|issue| issue.code == "unsafe_provider_routine_identifier"),
            "expected unsafe provider routine identifier issue, got {:?}",
            validator.issues
        );

        let _ = fs::remove_dir_all(root);
    }

    fn temp_workspace(label: &str) -> (AppWorkspace, PathBuf) {
        let root = temp_root(label);
        let generator_root = root.join("app_gen");
        let config_root = generator_root.join("_config");
        let templates_root = generator_root.join("_templates");
        let report_root = generator_root.join("target/appfw");
        fs::create_dir_all(config_root.join("data_sources")).expect("create data_sources dir");
        fs::create_dir_all(config_root.join("schemas")).expect("create schemas dir");
        fs::create_dir_all(&templates_root).expect("create templates dir");
        fs::create_dir_all(generator_root.join("src")).expect("create generator src dir");
        fs::create_dir_all(&report_root).expect("create report dir");

        (
            AppWorkspace {
                app_root: root.clone(),
                framework_root: root.clone(),
                generator_root,
                config_root,
                templates_root,
                report_root,
            },
            root,
        )
    }

    fn write_data_sources(workspace: &AppWorkspace, contents: &str) {
        fs::write(
            workspace.config_root.join("data_sources/_res.yaml"),
            contents,
        )
        .expect("write data sources");
    }

    fn write_schema(workspace: &AppWorkspace, schema: &str, contents: &str) {
        let schema_dir = workspace.config_root.join("schemas").join(schema);
        fs::create_dir_all(&schema_dir).expect("create schema dir");
        fs::write(schema_dir.join("_res.yaml"), contents).expect("write schema");
    }

    fn write_entity(workspace: &AppWorkspace, schema: &str, file_name: &str, contents: &str) {
        let entity_dir = workspace
            .config_root
            .join("schemas")
            .join(schema)
            .join("entity_types");
        fs::create_dir_all(&entity_dir).expect("create entity dir");
        fs::write(entity_dir.join(file_name), contents).expect("write entity");
    }

    fn write_seed(workspace: &AppWorkspace, schema: &str, file_name: &str, contents: &str) {
        let seed_dir = workspace
            .config_root
            .join("schemas")
            .join(schema)
            .join("seeds");
        fs::create_dir_all(&seed_dir).expect("create seed dir");
        fs::write(seed_dir.join(file_name), contents).expect("write seed");
    }

    fn write_sync_descriptor(workspace: &AppWorkspace, file_name: &str, contents: &str) {
        let sync_dir = workspace.config_root.join("sync");
        fs::create_dir_all(&sync_dir).expect("create sync dir");
        fs::write(sync_dir.join(file_name), contents).expect("write sync descriptor");
    }

    fn write_manifest_inputs(workspace: &AppWorkspace, kafka_enabled: bool) {
        write_data_sources(
            workspace,
            r#"
- name: pg_primary
  data_source_type: PostgreSQL
  is_system_schema_host: true
  environments:
  - name: local
    db_host: localhost
    db_name: app
    db_port: 5432
    security_profile: local_dev
    tls_mode: disabled
"#,
        );
        write_schema(
            workspace,
            "crm",
            r#"
id: 11111111-1111-4111-8111-111111111111
name: crm
description: CRM schema
data_source_name: pg_primary
"#,
        );
        fs::create_dir_all(workspace.app_root.join(".appfw")).expect("create manifest dir");
        fs::write(
            workspace.app_root.join(".appfw/manifest.yaml"),
            format!(
                r#"
version: 1
app:
  name: crm-sample
topology:
  data_sources:
  - name: pg_primary
    provider: PostgreSQL
  schemas:
  - name: crm
    data_source_name: pg_primary
  ingress:
  - name: crm-events
    kind: kafka
    enabled: {kafka_enabled}
    schema: crm
    topic: crm.events
    consumer_group: crm-events-worker
    handler: services.crm_events
"#
            ),
        )
        .expect("write manifest");
    }

    fn temp_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        env::temp_dir().join(format!(
            "appfw-validation-{label}-{}-{nanos}",
            std::process::id()
        ))
    }
}
