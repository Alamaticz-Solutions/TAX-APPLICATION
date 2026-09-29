//! Runtime-owned admin API contracts and response shaping.

use async_trait::async_trait;
use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post, MethodRouter},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    env,
    future::Future,
    path::{Path, PathBuf},
};
use tower_http::services::ServeDir;
use tracing::warn;

use crate::{
    auth::RuntimeJwtExtractor,
    auth_state::RuntimeAuthState,
    data_access::RuntimeQueryPlanDiagnostic,
    extension::UserAuth,
    model_metadata::RuntimeDataType,
    observability::{redact_diagnostic_text, redact_diagnostic_value, RequestContext},
    provider_capabilities,
    provider_contract_types::{GraphReadCapability, ProviderCapability, SaasReadCapability},
    provider_keys::FrameworkProvider,
    query_filter::{
        runtime_filter_capabilities_for_provider, RuntimeFilterCapabilities,
        RuntimeFilterDataTypeCapability,
    },
    query_ir::RuntimePaginationPolicy,
    AccessAction, PolicyAccess,
};

pub const ADMIN_ROLE: &str = "admin";
pub const ADMIN_TROUBLESHOOTING_ENV_VAR: &str = "APP_ADMIN_TROUBLESHOOTING_ENABLED";
pub const ADMIN_UI_DIST_DIR_ENV_VAR: &str = "APP_ADMIN_UI_DIST_DIR";
pub const ADMIN_MIGRATIONS_DIR_ENV_VAR: &str = "APP_MIGRATIONS_DIR";
pub const ADMIN_INDEX_PATH: &str = "/admin";
pub const ADMIN_INDEX_SLASH_PATH: &str = "/admin/";
pub const ADMIN_MODEL_PATH: &str = "/admin/model";
pub const ADMIN_TROUBLESHOOTING_PATH: &str = "/admin/troubleshooting";
pub const ADMIN_POLICY_EXPLAIN_PATH: &str = "/admin/troubleshooting/policy/explain";
pub const ADMIN_AUDIT_TIMELINE_PATH: &str = "/admin/troubleshooting/audit";
pub const ADMIN_QUERY_DIAGNOSE_PATH: &str = "/admin/troubleshooting/query/diagnose";
pub const ADMIN_ASSETS_PATH: &str = "/admin/assets";
pub const ADMIN_TROUBLESHOOTING_DISABLED_MESSAGE: &str =
    "admin troubleshooting is disabled; set APP_ADMIN_TROUBLESHOOTING_ENABLED=true to enable it";

#[derive(Debug, Serialize)]
pub struct AdminError {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_context: Option<RequestContext>,
}

impl AdminError {
    pub fn with_context(error: impl Into<String>, request_context: &RequestContext) -> Self {
        Self {
            error: redact_diagnostic_text(error.into()),
            request_context: Some(request_context.clone()),
        }
    }

    pub fn troubleshooting_disabled(request_context: &RequestContext) -> Self {
        Self::with_context(ADMIN_TROUBLESHOOTING_DISABLED_MESSAGE, request_context)
    }
}

#[derive(Debug, Deserialize)]
pub struct AdminPolicyExplainRequest {
    pub schema_name: String,
    pub type_name: String,
    pub action: String,
}

#[derive(Debug, Serialize)]
pub struct AdminPolicyExplainResponse {
    pub enabled: bool,
    pub request_context: RequestContext,
    pub policy_key: String,
    pub schema_name: String,
    pub type_name: String,
    pub action: String,
    pub user_name: String,
    pub roles: Vec<String>,
    pub decision: AdminPolicyDecision,
}

#[derive(Debug)]
pub struct AdminPolicyExplainSubject {
    pub policy_key: String,
    pub schema_name: String,
    pub type_name: String,
}

#[derive(Debug)]
pub struct AdminPolicyExplainResult {
    pub subject: AdminPolicyExplainSubject,
    pub decision: AdminPolicyDecision,
}

pub fn admin_policy_explain_result(
    policy_key: impl Into<String>,
    schema_name: impl Into<String>,
    type_name: impl Into<String>,
    decision: AdminPolicyDecision,
) -> AdminPolicyExplainResult {
    AdminPolicyExplainResult {
        subject: AdminPolicyExplainSubject {
            policy_key: policy_key.into(),
            schema_name: schema_name.into(),
            type_name: type_name.into(),
        },
        decision,
    }
}

#[derive(Debug, Deserialize)]
pub struct AdminAuditTimelineRequest {
    pub schema_name: String,
    pub type_name: String,
    pub record_id: String,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct AdminAuditTimelineResponse {
    pub enabled: bool,
    pub request_context: RequestContext,
    pub audited: bool,
    pub schema_name: String,
    pub type_name: String,
    pub record_id: String,
    pub events: Vec<Value>,
    pub current_policy: AdminPolicyDecision,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AdminMigration {
    pub id: String,
    pub name: String,
    pub schema: Option<String>,
    pub data_source: String,
    pub dialect: String,
    pub phase: String,
    pub path: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AdminHealthItem {
    pub status: &'static str,
    pub label: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct AdminProviderCapability {
    pub area_key: &'static str,
    pub area_label: &'static str,
    pub status: &'static str,
    pub reason: Option<&'static str>,
    pub evidence: Vec<AdminCapabilityEvidence>,
}

#[derive(Debug, Serialize)]
pub struct AdminCapabilityEvidence {
    pub kind: &'static str,
    pub contract: &'static str,
}

#[derive(Debug, Serialize)]
pub struct AdminModel<TEntityType, TDataSourceType, TFilterCapabilities> {
    pub backend_version: String,
    pub troubleshooting_enabled: bool,
    pub schemas: Vec<AdminSchema<TDataSourceType, TFilterCapabilities>>,
    pub entity_types: Vec<TEntityType>,
}

#[derive(Debug, Serialize)]
pub struct AdminSchema<TDataSourceType, TFilterCapabilities> {
    pub id: String,
    pub name: String,
    pub description: String,
    pub data_source_name: String,
    pub data_source_type: Option<TDataSourceType>,
    pub filter_capabilities: Option<TFilterCapabilities>,
    pub latest_migration: Option<AdminMigration>,
    pub health: AdminSchemaHealth,
}

#[derive(Debug, Serialize)]
pub struct AdminSchemaHealth {
    pub migration_status: AdminHealthItem,
    pub pending_drift: AdminHealthItem,
    pub connectivity: AdminHealthItem,
    pub entity_count: usize,
    pub table_entity_count: usize,
    pub migration_count: usize,
    pub provider_capabilities: Vec<AdminProviderCapability>,
}

#[derive(Debug)]
pub struct AdminSchemaSummaryInput<'a, TDataSourceType, TFilterCapabilities> {
    pub id: String,
    pub name: String,
    pub description: String,
    pub data_source_name: String,
    pub data_source_type: Option<TDataSourceType>,
    pub filter_capabilities: Option<TFilterCapabilities>,
    pub migration_dialect: Option<&'a str>,
    pub data_access_configured: bool,
    pub entity_count: usize,
    pub table_entity_count: usize,
    pub provider_capabilities: Vec<AdminProviderCapability>,
}

#[derive(Debug, Deserialize)]
struct AdminMigrationManifest {
    migrations: Vec<AdminMigration>,
}

#[derive(Debug)]
pub struct AdminAuditTimelineSubject {
    pub schema_name: String,
    pub type_name: String,
}

#[derive(Debug)]
pub struct AdminAuditTimelineResult {
    pub subject: AdminAuditTimelineSubject,
    pub audited: bool,
    pub events: Vec<Value>,
    pub current_policy: AdminPolicyDecision,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AdminQueryDiagnoseRequest {
    pub schema_name: String,
    pub type_name: String,
    pub filter: Option<Value>,
    pub sort: Option<Value>,
    pub skip: Option<i32>,
    pub limit: Option<i32>,
    pub after: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AdminQueryDiagnoseResponse {
    pub enabled: bool,
    pub request_context: RequestContext,
    pub diagnostic: RuntimeQueryPlanDiagnostic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdminServiceStatus {
    Unauthorized,
    BadRequest,
    Forbidden,
    InternalServerError,
}

#[derive(Debug)]
pub struct AdminServiceError {
    pub status: AdminServiceStatus,
    pub error: AdminError,
}

impl AdminServiceError {
    pub fn unauthorized(error: impl Into<String>, request_context: &RequestContext) -> Self {
        Self {
            status: AdminServiceStatus::Unauthorized,
            error: AdminError::with_context(error, request_context),
        }
    }

    pub fn bad_request(error: impl Into<String>, request_context: &RequestContext) -> Self {
        Self {
            status: AdminServiceStatus::BadRequest,
            error: AdminError::with_context(error, request_context),
        }
    }

    pub fn forbidden(error: impl Into<String>, request_context: &RequestContext) -> Self {
        Self {
            status: AdminServiceStatus::Forbidden,
            error: AdminError::with_context(error, request_context),
        }
    }

    pub fn internal(error: impl Into<String>, request_context: &RequestContext) -> Self {
        Self {
            status: AdminServiceStatus::InternalServerError,
            error: AdminError::with_context(error, request_context),
        }
    }

    pub fn troubleshooting_disabled(request_context: &RequestContext) -> Self {
        Self {
            status: AdminServiceStatus::Forbidden,
            error: AdminError::troubleshooting_disabled(request_context),
        }
    }
}

pub fn admin_dist_dir(product_manifest_dir: impl AsRef<Path>) -> PathBuf {
    env::var_os(ADMIN_UI_DIST_DIR_ENV_VAR)
        .map(PathBuf::from)
        .unwrap_or_else(|| product_manifest_dir.as_ref().join("admin_dist"))
}

pub fn admin_assets_dir(product_manifest_dir: impl AsRef<Path>) -> PathBuf {
    admin_dist_dir(product_manifest_dir).join("assets")
}

pub async fn admin_index_response(product_manifest_dir: impl AsRef<Path>) -> Response {
    let index_path = admin_dist_dir(product_manifest_dir).join("index.html");
    match tokio::fs::read_to_string(&index_path).await {
        Ok(html) => (
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            Html(html),
        )
            .into_response(),
        Err(err) => (
            StatusCode::SERVICE_UNAVAILABLE,
            format!(
                "admin UI bundle not found at {} ({err}). Run `cd admin_ui && npm install && npm run build`.",
                index_path.display()
            ),
        )
            .into_response(),
    }
}

pub fn admin_route_shell<S>(
    product_manifest_dir: impl AsRef<Path>,
    index: MethodRouter<S>,
    index_slash: MethodRouter<S>,
    model: MethodRouter<S>,
    troubleshooting_status: MethodRouter<S>,
    policy_explain: MethodRouter<S>,
    audit_timeline: MethodRouter<S>,
    query_diagnose: MethodRouter<S>,
) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let assets_dir = admin_assets_dir(product_manifest_dir);

    Router::new()
        .route(ADMIN_INDEX_PATH, index)
        .route(ADMIN_INDEX_SLASH_PATH, index_slash)
        .route(ADMIN_MODEL_PATH, model)
        .route(ADMIN_TROUBLESHOOTING_PATH, troubleshooting_status)
        .route(ADMIN_POLICY_EXPLAIN_PATH, policy_explain)
        .route(ADMIN_AUDIT_TIMELINE_PATH, audit_timeline)
        .route(ADMIN_QUERY_DIAGNOSE_PATH, query_diagnose)
        .nest_service(ADMIN_ASSETS_PATH, ServeDir::new(assets_dir))
}

pub fn admin_migrations_manifest_path(product_manifest_dir: impl AsRef<Path>) -> PathBuf {
    env::var_os(ADMIN_MIGRATIONS_DIR_ENV_VAR)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            product_manifest_dir
                .as_ref()
                .join("../database/_pkg/migrations")
        })
        .join("manifest.yaml")
}

pub async fn load_admin_migrations_manifest(
    product_manifest_dir: impl AsRef<Path>,
) -> Vec<AdminMigration> {
    let path = admin_migrations_manifest_path(product_manifest_dir);
    match tokio::fs::read_to_string(&path).await {
        Ok(contents) => match serde_yaml::from_str::<AdminMigrationManifest>(&contents) {
            Ok(manifest) => manifest.migrations,
            Err(err) => {
                warn!(
                    path = %path.display(),
                    error = %err,
                    "unable to parse admin UI migration manifest"
                );
                Vec::new()
            }
        },
        Err(_) => Vec::new(),
    }
}

pub fn admin_schema_migrations(
    schema_name: &str,
    data_source_name: &str,
    dialect: &str,
    migrations: &[AdminMigration],
) -> Vec<AdminMigration> {
    migrations
        .iter()
        .filter(|migration| {
            migration.data_source == data_source_name
                && migration.dialect == dialect
                && migration
                    .schema
                    .as_ref()
                    .is_none_or(|migration_schema| migration_schema == schema_name)
        })
        .cloned()
        .collect()
}

pub fn admin_migration_dialect(provider: FrameworkProvider) -> &'static str {
    match provider {
        FrameworkProvider::Postgres => "postgresql",
        FrameworkProvider::Mongo => "mongodb",
        FrameworkProvider::Mssql => "mssql",
        FrameworkProvider::FabricSqlAnalytics => "fabric_sql_analytics_readonly",
        FrameworkProvider::Snowflake => "snowflake",
        FrameworkProvider::Neo4j => "neo4j",
        FrameworkProvider::ServiceNow
        | FrameworkProvider::Workday
        | FrameworkProvider::Icims
        | FrameworkProvider::Salesforce
        | FrameworkProvider::Anaplan
        | FrameworkProvider::OracleFinancials => "external_api_readonly",
        FrameworkProvider::AiSearch => "ai_search_readonly",
    }
}

pub fn latest_admin_migration(migrations: &[AdminMigration]) -> Option<AdminMigration> {
    migrations
        .iter()
        .max_by(|left, right| left.id.cmp(&right.id).then(left.name.cmp(&right.name)))
        .cloned()
}

pub fn admin_migration_status(migrations: &[AdminMigration]) -> AdminHealthItem {
    match latest_admin_migration(migrations) {
        Some(migration) => AdminHealthItem {
            status: "ok",
            label: "Migration package available".to_string(),
            message: format!(
                "{} migration entr{} found. Latest: {} ({}, {}).",
                migrations.len(),
                if migrations.len() == 1 { "y" } else { "ies" },
                migration.name,
                migration.id,
                migration.phase,
            ),
        },
        None => AdminHealthItem {
            status: "warning",
            label: "No migration metadata".to_string(),
            message: "No migration entry was found for this schema and data source.".to_string(),
        },
    }
}

pub fn admin_generated_drift_status() -> AdminHealthItem {
    AdminHealthItem {
        status: "unknown",
        label: "Not evaluated by server".to_string(),
        message: "Run `scripts/appfw generate --check --json` to detect pending generated drift."
            .to_string(),
    }
}

pub fn admin_connectivity_status(data_access_configured: bool) -> AdminHealthItem {
    if data_access_configured {
        AdminHealthItem {
            status: "ok",
            label: "Ready".to_string(),
            message: "Data access initialized for this schema during route startup.".to_string(),
        }
    } else {
        AdminHealthItem {
            status: "error",
            label: "Not initialized".to_string(),
            message: "No data access client is registered for this schema.".to_string(),
        }
    }
}

pub fn admin_missing_data_access_error(
    schema_name: &str,
    request_context: &RequestContext,
) -> AdminServiceError {
    AdminServiceError::internal(
        format!("data access is not configured for schema '{schema_name}'"),
        request_context,
    )
}

pub fn admin_schema_health(
    migrations: &[AdminMigration],
    data_access_configured: bool,
    entity_count: usize,
    table_entity_count: usize,
    provider_capabilities: Vec<AdminProviderCapability>,
) -> AdminSchemaHealth {
    AdminSchemaHealth {
        migration_status: admin_migration_status(migrations),
        pending_drift: admin_generated_drift_status(),
        connectivity: admin_connectivity_status(data_access_configured),
        entity_count,
        table_entity_count,
        migration_count: migrations.len(),
        provider_capabilities,
    }
}

pub fn admin_schema_summary<TDataSourceType, TFilterCapabilities>(
    input: AdminSchemaSummaryInput<'_, TDataSourceType, TFilterCapabilities>,
    migrations: &[AdminMigration],
) -> AdminSchema<TDataSourceType, TFilterCapabilities> {
    let schema_migrations = input
        .migration_dialect
        .map(|dialect| {
            admin_schema_migrations(&input.name, &input.data_source_name, dialect, migrations)
        })
        .unwrap_or_default();
    let latest_migration = latest_admin_migration(&schema_migrations);
    let health = admin_schema_health(
        &schema_migrations,
        input.data_access_configured,
        input.entity_count,
        input.table_entity_count,
        input.provider_capabilities,
    );

    AdminSchema {
        id: input.id,
        name: input.name,
        description: input.description,
        data_source_name: input.data_source_name,
        data_source_type: input.data_source_type,
        filter_capabilities: input.filter_capabilities,
        latest_migration,
        health,
    }
}

pub fn admin_provider_capabilities(
    capabilities: &[ProviderCapability],
) -> Vec<AdminProviderCapability> {
    capabilities.iter().map(admin_provider_capability).collect()
}

pub fn admin_provider_capabilities_for_provider(
    provider: Option<FrameworkProvider>,
) -> Vec<AdminProviderCapability> {
    provider
        .map(|provider| {
            if provider.is_external_api_provider() {
                let profile = provider_capabilities::saas_read_profile(provider);
                return admin_saas_provider_capabilities(profile.capabilities);
            }
            if provider.is_graph_read_provider() {
                let profile = provider_capabilities::graph_read_profile(provider);
                return admin_graph_provider_capabilities(profile.capabilities);
            }

            let profile = provider_capabilities::semantic_profile(provider);
            admin_provider_capabilities(profile.capabilities)
        })
        .unwrap_or_default()
}

pub fn admin_filter_capabilities_for_provider<TProvider, TDataType>(
    provider: TProvider,
    runtime_provider: FrameworkProvider,
    map_data_type: impl Fn(RuntimeDataType) -> TDataType,
) -> RuntimeFilterCapabilities<TProvider, TDataType> {
    let runtime_capabilities = runtime_filter_capabilities_for_provider(runtime_provider);

    RuntimeFilterCapabilities {
        provider,
        data_types: runtime_capabilities
            .data_types
            .into_iter()
            .map(|capability| RuntimeFilterDataTypeCapability {
                data_type: map_data_type(capability.data_type),
                operators: capability.operators,
            })
            .collect(),
    }
}

pub fn admin_provider_capability(capability: &ProviderCapability) -> AdminProviderCapability {
    AdminProviderCapability {
        area_key: capability.area.key(),
        area_label: capability.area.label(),
        status: capability.status.label(),
        reason: capability.status.reason(),
        evidence: capability
            .evidence
            .iter()
            .map(|evidence| AdminCapabilityEvidence {
                kind: evidence.label(),
                contract: evidence.contract(),
            })
            .collect(),
    }
}

fn admin_graph_provider_capabilities(
    capabilities: &[GraphReadCapability],
) -> Vec<AdminProviderCapability> {
    capabilities
        .iter()
        .map(|capability| AdminProviderCapability {
            area_key: capability.area.key(),
            area_label: capability.area.label(),
            status: capability.status.label(),
            reason: capability.status.reason(),
            evidence: capability
                .evidence
                .iter()
                .map(|evidence| AdminCapabilityEvidence {
                    kind: evidence.label(),
                    contract: evidence.contract(),
                })
                .collect(),
        })
        .collect()
}

fn admin_saas_provider_capabilities(
    capabilities: &[SaasReadCapability],
) -> Vec<AdminProviderCapability> {
    capabilities
        .iter()
        .map(|capability| AdminProviderCapability {
            area_key: capability.area.key(),
            area_label: capability.area.label(),
            status: capability.status.label(),
            reason: capability.status.reason(),
            evidence: capability
                .evidence
                .iter()
                .map(|evidence| AdminCapabilityEvidence {
                    kind: evidence.label(),
                    contract: evidence.contract(),
                })
                .collect(),
        })
        .collect()
}

pub fn admin_service_status_code(status: AdminServiceStatus) -> StatusCode {
    match status {
        AdminServiceStatus::Unauthorized => StatusCode::UNAUTHORIZED,
        AdminServiceStatus::BadRequest => StatusCode::BAD_REQUEST,
        AdminServiceStatus::Forbidden => StatusCode::FORBIDDEN,
        AdminServiceStatus::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

pub fn admin_service_error_response(err: AdminServiceError) -> Response {
    (admin_service_status_code(err.status), Json(err.error)).into_response()
}

pub async fn admin_user_from_headers(
    auth_state: RuntimeAuthState,
    headers: HeaderMap,
    request_context: &RequestContext,
) -> Result<UserAuth, AdminServiceError> {
    let extractor = RuntimeJwtExtractor::new(auth_state, headers, false)
        .await
        .map_err(|err| AdminServiceError::unauthorized(err.to_string(), request_context))?;
    let Some(user) = extractor.user.as_ref().map(|user| (**user).clone()) else {
        return Err(AdminServiceError::forbidden(
            "admin role required",
            request_context,
        ));
    };
    if is_admin_user(&user) {
        Ok(user)
    } else {
        Err(AdminServiceError::forbidden(
            "admin role required",
            request_context,
        ))
    }
}

#[async_trait]
pub trait AdminPolicyExplainProvider: Sync {
    async fn explain_policy_access(
        &self,
        schema_name: &str,
        type_name: &str,
        action: AccessAction,
        user: &UserAuth,
        request_context: &RequestContext,
    ) -> Result<AdminPolicyExplainResult, AdminServiceError>;
}

#[async_trait]
pub trait AdminAuditTimelineProvider: Sync {
    async fn load_audit_timeline(
        &self,
        schema_name: &str,
        type_name: &str,
        record_id: &str,
        limit: i64,
        user: &UserAuth,
        request_context: &RequestContext,
    ) -> Result<AdminAuditTimelineResult, AdminServiceError>;
}

#[async_trait]
pub trait AdminQueryDiagnoseProvider: Sync {
    async fn diagnose_query(
        &self,
        schema_name: &str,
        type_name: &str,
        filter: Option<Value>,
        sort: Option<Value>,
        skip: i32,
        limit: i32,
        after: Option<String>,
        user: UserAuth,
        request_context: &RequestContext,
    ) -> Result<RuntimeQueryPlanDiagnostic, AdminServiceError>;
}

pub trait AdminModelProvider {
    type EntityType;
    type DataSourceType;
    type FilterCapabilities;

    fn admin_schemas(
        &self,
        migrations: &[AdminMigration],
    ) -> Vec<AdminSchema<Self::DataSourceType, Self::FilterCapabilities>>;

    fn admin_entity_types(&self) -> Vec<Self::EntityType>;
}

pub trait AdminRuntimeState: Clone + Send + Sync + 'static {
    type ModelProvider<'a>: AdminModelProvider + Send + Sync + 'a
    where
        Self: 'a;
    type PolicyExplainProvider<'a>: AdminPolicyExplainProvider + Send + 'a
    where
        Self: 'a;
    type AuditTimelineProvider<'a>: AdminAuditTimelineProvider + Send + 'a
    where
        Self: 'a;
    type QueryDiagnoseProvider<'a>: AdminQueryDiagnoseProvider + Send + 'a
    where
        Self: 'a;

    fn product_manifest_dir() -> &'static str;
    fn backend_version() -> &'static str;
    fn auth_state(&self) -> RuntimeAuthState;
    fn troubleshooting_enabled(&self) -> bool;
    fn model_provider(&self) -> Self::ModelProvider<'_>;
    fn policy_explain_provider(&self) -> Self::PolicyExplainProvider<'_>;
    fn audit_timeline_provider(&self) -> Self::AuditTimelineProvider<'_>;
    fn query_diagnose_provider(&self) -> Self::QueryDiagnoseProvider<'_>;
}

pub fn admin_runtime_routes<S>() -> Router<S>
where
    S: AdminRuntimeState,
    for<'a> <S::ModelProvider<'a> as AdminModelProvider>::EntityType: Serialize,
    for<'a> <S::ModelProvider<'a> as AdminModelProvider>::DataSourceType: Serialize,
    for<'a> <S::ModelProvider<'a> as AdminModelProvider>::FilterCapabilities: Serialize,
{
    admin_route_shell(
        S::product_manifest_dir(),
        get(admin_index_endpoint::<S>),
        get(admin_index_endpoint::<S>),
        get(admin_model_endpoint::<S>),
        get(admin_troubleshooting_status_endpoint::<S>),
        post(admin_policy_explain_endpoint::<S>),
        post(admin_audit_timeline_endpoint::<S>),
        post(admin_query_diagnose_endpoint::<S>),
    )
}

async fn admin_index_endpoint<S>() -> Response
where
    S: AdminRuntimeState,
{
    admin_index_response(S::product_manifest_dir()).await
}

async fn admin_model_endpoint<S>(State(state): State<S>, headers: HeaderMap) -> Response
where
    S: AdminRuntimeState,
    for<'a> <S::ModelProvider<'a> as AdminModelProvider>::EntityType: Serialize,
    for<'a> <S::ModelProvider<'a> as AdminModelProvider>::DataSourceType: Serialize,
    for<'a> <S::ModelProvider<'a> as AdminModelProvider>::FilterCapabilities: Serialize,
{
    let auth_state = state.auth_state();
    let troubleshooting_enabled = state.troubleshooting_enabled();
    let provider = state.model_provider();
    admin_model_response(
        auth_state,
        headers,
        &provider,
        S::product_manifest_dir(),
        S::backend_version(),
        troubleshooting_enabled,
    )
    .await
}

async fn admin_troubleshooting_status_endpoint<S>(
    State(state): State<S>,
    headers: HeaderMap,
) -> Response
where
    S: AdminRuntimeState,
{
    admin_troubleshooting_status_response(
        state.auth_state(),
        headers,
        state.troubleshooting_enabled(),
    )
    .await
}

async fn admin_policy_explain_endpoint<S>(
    State(state): State<S>,
    headers: HeaderMap,
    Json(request): Json<AdminPolicyExplainRequest>,
) -> Response
where
    S: AdminRuntimeState,
{
    let auth_state = state.auth_state();
    let troubleshooting_enabled = state.troubleshooting_enabled();
    let provider = state.policy_explain_provider();
    admin_policy_explain_response(
        &provider,
        auth_state,
        headers,
        troubleshooting_enabled,
        request,
    )
    .await
}

async fn admin_audit_timeline_endpoint<S>(
    State(state): State<S>,
    headers: HeaderMap,
    Json(request): Json<AdminAuditTimelineRequest>,
) -> Response
where
    S: AdminRuntimeState,
{
    let auth_state = state.auth_state();
    let troubleshooting_enabled = state.troubleshooting_enabled();
    let provider = state.audit_timeline_provider();
    admin_audit_timeline_response(
        &provider,
        auth_state,
        headers,
        troubleshooting_enabled,
        request,
    )
    .await
}

async fn admin_query_diagnose_endpoint<S>(
    State(state): State<S>,
    headers: HeaderMap,
    Json(request): Json<AdminQueryDiagnoseRequest>,
) -> Response
where
    S: AdminRuntimeState,
{
    let auth_state = state.auth_state();
    let troubleshooting_enabled = state.troubleshooting_enabled();
    let provider = state.query_diagnose_provider();
    admin_query_diagnose_response(
        &provider,
        auth_state,
        headers,
        troubleshooting_enabled,
        request,
    )
    .await
}

pub async fn admin_model<P>(
    provider: &P,
    product_manifest_dir: impl AsRef<Path>,
    backend_version: impl Into<String>,
    troubleshooting_enabled: bool,
) -> AdminModel<P::EntityType, P::DataSourceType, P::FilterCapabilities>
where
    P: AdminModelProvider + ?Sized,
{
    let migrations = load_admin_migrations_manifest(product_manifest_dir).await;

    AdminModel {
        backend_version: backend_version.into(),
        troubleshooting_enabled,
        schemas: provider.admin_schemas(&migrations),
        entity_types: provider.admin_entity_types(),
    }
}

pub async fn admin_model_response<P>(
    auth_state: RuntimeAuthState,
    headers: HeaderMap,
    provider: &P,
    product_manifest_dir: impl AsRef<Path>,
    backend_version: impl Into<String>,
    troubleshooting_enabled: bool,
) -> Response
where
    P: AdminModelProvider + ?Sized,
    P::EntityType: Serialize,
    P::DataSourceType: Serialize,
    P::FilterCapabilities: Serialize,
{
    let request_context = RequestContext::from_headers(&headers);
    match admin_user_from_headers(auth_state, headers, &request_context).await {
        Ok(_) => Json(
            admin_model(
                provider,
                product_manifest_dir,
                backend_version,
                troubleshooting_enabled,
            )
            .await,
        )
        .into_response(),
        Err(err) => admin_service_error_response(err),
    }
}

pub async fn admin_troubleshooting_status_response(
    auth_state: RuntimeAuthState,
    headers: HeaderMap,
    enabled: bool,
) -> Response {
    let request_context = RequestContext::from_headers(&headers);
    match admin_user_from_headers(auth_state, headers, &request_context).await {
        Ok(_) => Json(admin_troubleshooting_status(enabled, request_context)).into_response(),
        Err(err) => admin_service_error_response(err),
    }
}

pub async fn admin_policy_explain_response(
    provider: &(impl AdminPolicyExplainProvider + ?Sized),
    auth_state: RuntimeAuthState,
    headers: HeaderMap,
    troubleshooting_enabled: bool,
    request: AdminPolicyExplainRequest,
) -> Response {
    let request_context = RequestContext::from_headers(&headers);
    let user = match admin_user_from_headers(auth_state, headers, &request_context).await {
        Ok(user) => user,
        Err(err) => return admin_service_error_response(err),
    };

    match explain_admin_policy(
        provider,
        troubleshooting_enabled,
        request_context,
        request,
        &user,
    )
    .await
    {
        Ok(response) => Json(response).into_response(),
        Err(err) => admin_service_error_response(err),
    }
}

pub async fn explain_admin_policy(
    provider: &(impl AdminPolicyExplainProvider + ?Sized),
    troubleshooting_enabled: bool,
    request_context: RequestContext,
    request: AdminPolicyExplainRequest,
    user: &UserAuth,
) -> Result<AdminPolicyExplainResponse, AdminServiceError> {
    if !troubleshooting_enabled {
        return Err(AdminServiceError::troubleshooting_disabled(
            &request_context,
        ));
    }

    let action = parse_admin_access_action(&request.action).ok_or_else(|| {
        AdminServiceError::bad_request(
            "action must be one of read, create, update, delete",
            &request_context,
        )
    })?;

    let result = provider
        .explain_policy_access(
            &request.schema_name,
            &request.type_name,
            action,
            user,
            &request_context,
        )
        .await?;

    Ok(AdminPolicyExplainResponse {
        enabled: true,
        request_context,
        policy_key: result.subject.policy_key,
        schema_name: result.subject.schema_name,
        type_name: result.subject.type_name,
        action: action.to_string(),
        user_name: user.user_name.clone(),
        roles: user.roles.clone(),
        decision: result.decision,
    })
}

pub async fn admin_query_diagnose_response(
    provider: &(impl AdminQueryDiagnoseProvider + ?Sized),
    auth_state: RuntimeAuthState,
    headers: HeaderMap,
    troubleshooting_enabled: bool,
    request: AdminQueryDiagnoseRequest,
) -> Response {
    let request_context = RequestContext::from_headers(&headers);
    let user = match admin_user_from_headers(auth_state, headers, &request_context).await {
        Ok(user) => user,
        Err(err) => return admin_service_error_response(err),
    };

    match admin_query_diagnose(
        provider,
        troubleshooting_enabled,
        request_context,
        request,
        user,
    )
    .await
    {
        Ok(response) => Json(response).into_response(),
        Err(err) => admin_service_error_response(err),
    }
}

pub async fn admin_query_diagnose(
    provider: &(impl AdminQueryDiagnoseProvider + ?Sized),
    troubleshooting_enabled: bool,
    request_context: RequestContext,
    request: AdminQueryDiagnoseRequest,
    user: UserAuth,
) -> Result<AdminQueryDiagnoseResponse, AdminServiceError> {
    if !troubleshooting_enabled {
        return Err(AdminServiceError::troubleshooting_disabled(
            &request_context,
        ));
    }

    let (skip, limit) = RuntimePaginationPolicy::from_env()
        .normalize(request.skip, request.limit)
        .map_err(|err| AdminServiceError::bad_request(err.to_string(), &request_context))?;

    let mut diagnostic = provider
        .diagnose_query(
            &request.schema_name,
            &request.type_name,
            request.filter,
            request.sort,
            skip,
            limit,
            request.after,
            user,
            &request_context,
        )
        .await?;
    diagnostic.provider_diagnostic = redact_diagnostic_value(diagnostic.provider_diagnostic);

    Ok(AdminQueryDiagnoseResponse {
        enabled: true,
        request_context,
        diagnostic,
    })
}

pub async fn admin_audit_timeline_response(
    provider: &(impl AdminAuditTimelineProvider + ?Sized),
    auth_state: RuntimeAuthState,
    headers: HeaderMap,
    troubleshooting_enabled: bool,
    request: AdminAuditTimelineRequest,
) -> Response {
    let request_context = RequestContext::from_headers(&headers);
    let user = match admin_user_from_headers(auth_state, headers, &request_context).await {
        Ok(user) => user,
        Err(err) => return admin_service_error_response(err),
    };

    match admin_audit_timeline(
        provider,
        troubleshooting_enabled,
        request_context,
        request,
        &user,
    )
    .await
    {
        Ok(response) => Json(response).into_response(),
        Err(err) => admin_service_error_response(err),
    }
}

pub async fn admin_audit_timeline(
    provider: &(impl AdminAuditTimelineProvider + ?Sized),
    troubleshooting_enabled: bool,
    request_context: RequestContext,
    request: AdminAuditTimelineRequest,
    user: &UserAuth,
) -> Result<AdminAuditTimelineResponse, AdminServiceError> {
    if !troubleshooting_enabled {
        return Err(AdminServiceError::troubleshooting_disabled(
            &request_context,
        ));
    }

    let result = provider
        .load_audit_timeline(
            &request.schema_name,
            &request.type_name,
            &request.record_id,
            request.limit.unwrap_or(25),
            user,
            &request_context,
        )
        .await?;

    if !result.current_policy.allow {
        return Err(AdminServiceError::forbidden(
            "read access denied for audit timeline",
            &request_context,
        ));
    }

    let audited = result.audited;
    let events = if audited { result.events } else { Vec::new() };
    let message = if audited {
        result.message
    } else {
        result
            .message
            .or_else(|| Some("Entity is not configured with the audited facet.".to_string()))
    };

    Ok(AdminAuditTimelineResponse {
        enabled: true,
        request_context,
        audited,
        schema_name: result.subject.schema_name,
        type_name: result.subject.type_name,
        record_id: request.record_id,
        events,
        current_policy: result.current_policy,
        message,
    })
}

pub async fn admin_audit_timeline_for_subject<F, Fut>(
    subject: AdminAuditTimelineSubject,
    current_policy: AdminPolicyDecision,
    audited: bool,
    message: Option<String>,
    load_events: F,
) -> Result<AdminAuditTimelineResult, AdminServiceError>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<Vec<Value>, AdminServiceError>>,
{
    if !current_policy.allow {
        return Ok(AdminAuditTimelineResult {
            subject,
            audited: false,
            events: Vec::new(),
            current_policy,
            message,
        });
    }

    if !audited {
        return Ok(AdminAuditTimelineResult {
            subject,
            audited: false,
            events: Vec::new(),
            current_policy,
            message,
        });
    }

    let events = load_events().await?;

    Ok(AdminAuditTimelineResult {
        subject,
        audited: true,
        events,
        current_policy,
        message,
    })
}

#[derive(Serialize)]
pub struct AdminTroubleshootingStatus {
    pub enabled: bool,
    pub env_var: &'static str,
    pub request_context: RequestContext,
    pub features: Vec<AdminTroubleshootingFeature>,
}

#[derive(Serialize)]
pub struct AdminTroubleshootingFeature {
    pub id: &'static str,
    pub label: &'static str,
    pub enabled: bool,
    pub path: &'static str,
}

#[derive(Debug, Serialize)]
pub struct AdminPolicyDecision {
    pub allow: bool,
    pub filter: Option<Value>,
}

impl From<PolicyAccess> for AdminPolicyDecision {
    fn from(access: PolicyAccess) -> Self {
        Self {
            allow: access.allow,
            filter: access.filter,
        }
    }
}

pub fn admin_troubleshooting_status(
    enabled: bool,
    request_context: RequestContext,
) -> AdminTroubleshootingStatus {
    AdminTroubleshootingStatus {
        enabled,
        env_var: ADMIN_TROUBLESHOOTING_ENV_VAR,
        request_context,
        features: vec![
            AdminTroubleshootingFeature {
                id: "policy_explain",
                label: "Policy Explainability",
                enabled,
                path: "/admin/troubleshooting/policy/explain",
            },
            AdminTroubleshootingFeature {
                id: "audit_timeline",
                label: "Audit Timeline",
                enabled,
                path: "/admin/troubleshooting/audit",
            },
            AdminTroubleshootingFeature {
                id: "query_diagnose",
                label: "Query Diagnostics",
                enabled,
                path: "/admin/troubleshooting/query/diagnose",
            },
        ],
    }
}

pub fn is_admin_user(user: &UserAuth) -> bool {
    user.roles.iter().any(|role| role == ADMIN_ROLE)
}

pub fn parse_admin_access_action(action: &str) -> Option<AccessAction> {
    match action.to_ascii_lowercase().as_str() {
        "read" => Some(AccessAction::Read),
        "create" => Some(AccessAction::Create),
        "update" => Some(AccessAction::Update),
        "delete" => Some(AccessAction::Delete),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_contract_types::SaasReadArea;
    use crate::{
        data_access::RuntimePaginationDiagnostic,
        provider_contract_types::{CapabilityStatus, CertificationEvidence, ProviderContractArea},
        query_cost::{QueryCost, QueryCostBudget},
    };
    use axum::{
        body::Body,
        http::{Method, Request},
        routing::{get, post},
    };
    use serde_json::json;
    use std::{
        fs,
        sync::Mutex,
        time::{SystemTime, UNIX_EPOCH},
    };
    use tower::ServiceExt;

    static ADMIN_UI_ENV_LOCK: Mutex<()> = Mutex::new(());
    static ADMIN_MIGRATIONS_ENV_LOCK: Mutex<()> = Mutex::new(());

    fn request_context() -> RequestContext {
        RequestContext {
            request_id: "request-123".to_string(),
            correlation_id: "correlation-456".to_string(),
        }
    }

    fn user_with_roles(roles: &[&str]) -> UserAuth {
        UserAuth::human(
            "tenant-1",
            "casey",
            "UTC",
            roles.iter().map(|role| (*role).to_string()).collect(),
            Vec::new(),
            "redacted",
        )
    }

    async fn ok_handler() -> &'static str {
        "ok"
    }

    async fn route_status(router: Router, method: Method, path: &str) -> StatusCode {
        router
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("route should respond")
            .status()
    }

    fn migration(id: &str, name: &str, schema: Option<&str>, data_source: &str) -> AdminMigration {
        AdminMigration {
            id: id.to_string(),
            name: name.to_string(),
            schema: schema.map(str::to_string),
            data_source: data_source.to_string(),
            dialect: "postgresql".to_string(),
            phase: "baseline".to_string(),
            path: format!("{id}_{name}.sql"),
            description: None,
        }
    }

    struct StaticPolicyExplainProvider;

    #[async_trait::async_trait]
    impl AdminPolicyExplainProvider for StaticPolicyExplainProvider {
        async fn explain_policy_access(
            &self,
            schema_name: &str,
            type_name: &str,
            _action: AccessAction,
            _user: &UserAuth,
            _request_context: &RequestContext,
        ) -> Result<AdminPolicyExplainResult, AdminServiceError> {
            Ok(AdminPolicyExplainResult {
                subject: AdminPolicyExplainSubject {
                    policy_key: format!("{schema_name}.account"),
                    schema_name: schema_name.to_string(),
                    type_name: type_name.to_string(),
                },
                decision: AdminPolicyDecision {
                    allow: true,
                    filter: Some(json!({ "owner_id": "user-1" })),
                },
            })
        }
    }

    struct StaticAuditTimelineProvider {
        allow: bool,
        audited: bool,
        events: Vec<Value>,
        message: Option<String>,
    }

    #[async_trait::async_trait]
    impl AdminAuditTimelineProvider for StaticAuditTimelineProvider {
        async fn load_audit_timeline(
            &self,
            schema_name: &str,
            type_name: &str,
            _record_id: &str,
            limit: i64,
            _user: &UserAuth,
            _request_context: &RequestContext,
        ) -> Result<AdminAuditTimelineResult, AdminServiceError> {
            Ok(AdminAuditTimelineResult {
                subject: AdminAuditTimelineSubject {
                    schema_name: schema_name.to_string(),
                    type_name: type_name.to_string(),
                },
                audited: self.audited,
                events: self
                    .events
                    .iter()
                    .take(usize::try_from(limit).unwrap_or_default())
                    .cloned()
                    .collect(),
                current_policy: AdminPolicyDecision {
                    allow: self.allow,
                    filter: None,
                },
                message: self.message.clone(),
            })
        }
    }

    struct StaticQueryDiagnoseProvider;

    #[async_trait::async_trait]
    impl AdminQueryDiagnoseProvider for StaticQueryDiagnoseProvider {
        async fn diagnose_query(
            &self,
            schema_name: &str,
            type_name: &str,
            filter: Option<Value>,
            sort: Option<Value>,
            skip: i32,
            limit: i32,
            after: Option<String>,
            user: UserAuth,
            _request_context: &RequestContext,
        ) -> Result<RuntimeQueryPlanDiagnostic, AdminServiceError> {
            Ok(RuntimeQueryPlanDiagnostic {
                schema_name: schema_name.to_string(),
                type_name: type_name.to_string(),
                provider: "postgres".to_string(),
                data_source: "crm_primary".to_string(),
                pagination: RuntimePaginationDiagnostic {
                    strategy: if after.is_some() { "keyset" } else { "offset" },
                    skip,
                    limit,
                    after_present: after.is_some(),
                },
                access_filter_applied: filter.is_some(),
                cost: QueryCost {
                    score: 1,
                    ..QueryCost::default()
                },
                budget: QueryCostBudget {
                    max_query_cost: 100,
                    max_aggregate_cost: 200,
                    ..QueryCostBudget::default()
                },
                provider_diagnostic: json!({
                    "sort_present": sort.is_some(),
                    "user_name": user.user_name,
                }),
            })
        }
    }

    struct LeakyQueryDiagnoseProvider;

    #[async_trait::async_trait]
    impl AdminQueryDiagnoseProvider for LeakyQueryDiagnoseProvider {
        async fn diagnose_query(
            &self,
            schema_name: &str,
            type_name: &str,
            _filter: Option<Value>,
            _sort: Option<Value>,
            skip: i32,
            limit: i32,
            after: Option<String>,
            _user: UserAuth,
            _request_context: &RequestContext,
        ) -> Result<RuntimeQueryPlanDiagnostic, AdminServiceError> {
            Ok(RuntimeQueryPlanDiagnostic {
                schema_name: schema_name.to_string(),
                type_name: type_name.to_string(),
                provider: "postgres".to_string(),
                data_source: "crm_primary".to_string(),
                pagination: RuntimePaginationDiagnostic {
                    strategy: if after.is_some() { "keyset" } else { "offset" },
                    skip,
                    limit,
                    after_present: after.is_some(),
                },
                access_filter_applied: false,
                cost: QueryCost {
                    score: 1,
                    ..QueryCost::default()
                },
                budget: QueryCostBudget {
                    max_query_cost: 100,
                    max_aggregate_cost: 200,
                    ..QueryCostBudget::default()
                },
                provider_diagnostic: json!({
                    "connection": "server=db;Pwd=short",
                    "nested": {
                        "private_key": "pem",
                        "safe": "query plan available"
                    },
                    "warnings": [
                        "Authorization: Bearer jwt.value"
                    ]
                }),
            })
        }
    }

    struct StaticModelProvider;

    impl AdminModelProvider for StaticModelProvider {
        type EntityType = &'static str;
        type DataSourceType = &'static str;
        type FilterCapabilities = &'static str;

        fn admin_schemas(
            &self,
            migrations: &[AdminMigration],
        ) -> Vec<AdminSchema<Self::DataSourceType, Self::FilterCapabilities>> {
            vec![AdminSchema {
                id: "schema-crm".to_string(),
                name: "crm".to_string(),
                description: "CRM schema".to_string(),
                data_source_name: "crm_primary".to_string(),
                data_source_type: Some("postgresql"),
                filter_capabilities: Some("filters"),
                latest_migration: latest_admin_migration(migrations),
                health: admin_schema_health(migrations, true, 2, 1, Vec::new()),
            }]
        }

        fn admin_entity_types(&self) -> Vec<Self::EntityType> {
            vec!["Account", "Contact"]
        }
    }

    #[derive(Clone)]
    struct StaticAdminState;

    impl AdminRuntimeState for StaticAdminState {
        type ModelProvider<'a>
            = StaticModelProvider
        where
            Self: 'a;
        type PolicyExplainProvider<'a>
            = StaticPolicyExplainProvider
        where
            Self: 'a;
        type AuditTimelineProvider<'a>
            = StaticAuditTimelineProvider
        where
            Self: 'a;
        type QueryDiagnoseProvider<'a>
            = StaticQueryDiagnoseProvider
        where
            Self: 'a;

        fn product_manifest_dir() -> &'static str {
            "/tmp/product-backend"
        }

        fn backend_version() -> &'static str {
            "1.2.3"
        }

        fn auth_state(&self) -> RuntimeAuthState {
            RuntimeAuthState {
                jwt_issuer: "https://tenant.okta.com/oauth2/default".to_string(),
                jwt_audience: "api://tenant".to_string(),
                okta_client_id: "client-id".to_string(),
            }
        }

        fn troubleshooting_enabled(&self) -> bool {
            true
        }

        fn model_provider(&self) -> Self::ModelProvider<'_> {
            StaticModelProvider
        }

        fn policy_explain_provider(&self) -> Self::PolicyExplainProvider<'_> {
            StaticPolicyExplainProvider
        }

        fn audit_timeline_provider(&self) -> Self::AuditTimelineProvider<'_> {
            StaticAuditTimelineProvider {
                allow: true,
                audited: true,
                events: Vec::new(),
                message: None,
            }
        }

        fn query_diagnose_provider(&self) -> Self::QueryDiagnoseProvider<'_> {
            StaticQueryDiagnoseProvider
        }
    }

    #[test]
    fn admin_gate_requires_admin_role() {
        assert!(is_admin_user(&user_with_roles(&["admin"])));
        assert!(!is_admin_user(&user_with_roles(&["sales_rep", "analyst"])));
    }

    #[test]
    fn maps_admin_service_status_to_http_status() {
        assert_eq!(
            admin_service_status_code(AdminServiceStatus::Unauthorized),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            admin_service_status_code(AdminServiceStatus::BadRequest),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            admin_service_status_code(AdminServiceStatus::Forbidden),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            admin_service_status_code(AdminServiceStatus::InternalServerError),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn diagnostic_errors_redact_sensitive_values() {
        let error = AdminError::with_context(
            "policy diagnostic failed with password=hunter2 Authorization: Bearer jwt.value",
            &request_context(),
        );

        assert!(error.error.contains("[REDACTED]"));
        assert!(!error.error.contains("hunter2"));
        assert!(!error.error.contains("jwt.value"));
        assert_eq!(
            error.request_context.as_ref().unwrap().request_id,
            "request-123"
        );
    }

    #[test]
    fn troubleshooting_status_lists_runtime_supported_features() {
        let status = admin_troubleshooting_status(true, request_context());

        assert!(status.enabled);
        assert_eq!(status.env_var, ADMIN_TROUBLESHOOTING_ENV_VAR);
        assert_eq!(status.features.len(), 3);
        assert!(status
            .features
            .iter()
            .any(|feature| feature.id == "query_diagnose" && feature.enabled));
    }

    #[test]
    fn admin_dist_dir_defaults_under_product_manifest_dir() {
        let _guard = ADMIN_UI_ENV_LOCK.lock().expect("admin UI env lock");
        let previous = env::var_os(ADMIN_UI_DIST_DIR_ENV_VAR);
        env::remove_var(ADMIN_UI_DIST_DIR_ENV_VAR);

        let dist_dir = admin_dist_dir("/tmp/product-backend");

        assert_eq!(dist_dir, PathBuf::from("/tmp/product-backend/admin_dist"));
        match previous {
            Some(value) => env::set_var(ADMIN_UI_DIST_DIR_ENV_VAR, value),
            None => env::remove_var(ADMIN_UI_DIST_DIR_ENV_VAR),
        }
    }

    #[test]
    fn admin_assets_dir_uses_dist_dir_override() {
        let _guard = ADMIN_UI_ENV_LOCK.lock().expect("admin UI env lock");
        let previous = env::var_os(ADMIN_UI_DIST_DIR_ENV_VAR);
        env::set_var(ADMIN_UI_DIST_DIR_ENV_VAR, "/tmp/admin-bundle");

        let assets_dir = admin_assets_dir("/tmp/product-backend");

        assert_eq!(assets_dir, PathBuf::from("/tmp/admin-bundle/assets"));
        match previous {
            Some(value) => env::set_var(ADMIN_UI_DIST_DIR_ENV_VAR, value),
            None => env::remove_var(ADMIN_UI_DIST_DIR_ENV_VAR),
        }
    }

    #[tokio::test]
    async fn admin_route_shell_mounts_standard_paths_and_assets() {
        let router = admin_route_shell(
            "/tmp/product-backend",
            get(ok_handler),
            get(ok_handler),
            get(ok_handler),
            get(ok_handler),
            post(ok_handler),
            post(ok_handler),
            post(ok_handler),
        );

        assert_eq!(
            route_status(router.clone(), Method::GET, ADMIN_INDEX_PATH).await,
            StatusCode::OK
        );
        assert_eq!(
            route_status(router.clone(), Method::GET, ADMIN_INDEX_SLASH_PATH).await,
            StatusCode::OK
        );
        assert_eq!(
            route_status(router.clone(), Method::GET, ADMIN_MODEL_PATH).await,
            StatusCode::OK
        );
        assert_eq!(
            route_status(router.clone(), Method::GET, ADMIN_TROUBLESHOOTING_PATH).await,
            StatusCode::OK
        );
        assert_eq!(
            route_status(router.clone(), Method::POST, ADMIN_POLICY_EXPLAIN_PATH).await,
            StatusCode::OK
        );
        assert_eq!(
            route_status(router.clone(), Method::POST, ADMIN_AUDIT_TIMELINE_PATH).await,
            StatusCode::OK
        );
        assert_eq!(
            route_status(router.clone(), Method::POST, ADMIN_QUERY_DIAGNOSE_PATH).await,
            StatusCode::OK
        );
        assert_eq!(
            route_status(router, Method::GET, "/admin/assets/missing.js").await,
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn admin_runtime_routes_mount_standard_handler_shell() {
        let router = admin_runtime_routes::<StaticAdminState>().with_state(StaticAdminState);

        assert_eq!(
            route_status(router.clone(), Method::GET, ADMIN_MODEL_PATH).await,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            route_status(router.clone(), Method::GET, ADMIN_TROUBLESHOOTING_PATH).await,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            route_status(router, Method::GET, "/admin/assets/missing.js").await,
            StatusCode::NOT_FOUND
        );
    }

    #[test]
    fn migrations_manifest_path_defaults_under_product_database_package() {
        let _guard = ADMIN_MIGRATIONS_ENV_LOCK
            .lock()
            .expect("admin migrations env lock");
        let previous = env::var_os(ADMIN_MIGRATIONS_DIR_ENV_VAR);
        env::remove_var(ADMIN_MIGRATIONS_DIR_ENV_VAR);

        let manifest_path = admin_migrations_manifest_path("/tmp/product-backend");

        assert_eq!(
            manifest_path,
            PathBuf::from("/tmp/product-backend/../database/_pkg/migrations/manifest.yaml")
        );
        match previous {
            Some(value) => env::set_var(ADMIN_MIGRATIONS_DIR_ENV_VAR, value),
            None => env::remove_var(ADMIN_MIGRATIONS_DIR_ENV_VAR),
        }
    }

    #[test]
    fn migrations_manifest_path_uses_directory_override() {
        let _guard = ADMIN_MIGRATIONS_ENV_LOCK
            .lock()
            .expect("admin migrations env lock");
        let previous = env::var_os(ADMIN_MIGRATIONS_DIR_ENV_VAR);
        env::set_var(ADMIN_MIGRATIONS_DIR_ENV_VAR, "/tmp/migrations");

        let manifest_path = admin_migrations_manifest_path("/tmp/product-backend");

        assert_eq!(
            manifest_path,
            PathBuf::from("/tmp/migrations/manifest.yaml")
        );
        match previous {
            Some(value) => env::set_var(ADMIN_MIGRATIONS_DIR_ENV_VAR, value),
            None => env::remove_var(ADMIN_MIGRATIONS_DIR_ENV_VAR),
        }
    }

    #[test]
    fn schema_migrations_match_schema_data_source_and_dialect() {
        let migrations = vec![
            migration("001", "all-crm", None, "crm_primary"),
            migration("002", "crm-only", Some("crm"), "crm_primary"),
            migration("003", "other-schema", Some("sales"), "crm_primary"),
            migration("004", "other-ds", Some("crm"), "audit_store"),
        ];

        let matched = admin_schema_migrations("crm", "crm_primary", "postgresql", &migrations);

        assert_eq!(matched.len(), 2);
        assert_eq!(matched[0].id, "001");
        assert_eq!(matched[1].id, "002");
    }

    #[test]
    fn migration_dialect_maps_all_framework_providers() {
        assert_eq!(
            admin_migration_dialect(FrameworkProvider::Postgres),
            "postgresql"
        );
        assert_eq!(admin_migration_dialect(FrameworkProvider::Mongo), "mongodb");
        assert_eq!(admin_migration_dialect(FrameworkProvider::Mssql), "mssql");
        assert_eq!(
            admin_migration_dialect(FrameworkProvider::Snowflake),
            "snowflake"
        );
    }

    #[test]
    fn migration_status_reports_latest_migration() {
        let migrations = vec![
            migration("001", "initial", Some("crm"), "crm_primary"),
            migration("010", "add_accounts", Some("crm"), "crm_primary"),
        ];

        let status = admin_migration_status(&migrations);

        assert_eq!(status.status, "ok");
        assert!(status.message.contains("add_accounts"));
        assert!(status.message.contains("010"));
    }

    #[test]
    fn connectivity_status_reports_missing_data_access() {
        let status = admin_connectivity_status(false);

        assert_eq!(status.status, "error");
        assert_eq!(status.label, "Not initialized");
    }

    #[test]
    fn missing_data_access_error_uses_runtime_service_shape() {
        let err = admin_missing_data_access_error("crm", &request_context());

        assert_eq!(err.status, AdminServiceStatus::InternalServerError);
        assert!(err
            .error
            .error
            .contains("data access is not configured for schema 'crm'"));
        assert_eq!(
            err.error.request_context.as_ref().unwrap().correlation_id,
            "correlation-456"
        );
    }

    #[test]
    fn schema_health_shapes_counts_and_runtime_statuses() {
        let migrations = vec![migration("010", "add_accounts", Some("crm"), "crm_primary")];

        let health = admin_schema_health(&migrations, true, 7, 5, Vec::new());

        assert_eq!(health.entity_count, 7);
        assert_eq!(health.table_entity_count, 5);
        assert_eq!(health.migration_count, 1);
        assert_eq!(health.migration_status.status, "ok");
        assert_eq!(health.pending_drift.status, "unknown");
        assert_eq!(health.connectivity.status, "ok");
        assert!(health.provider_capabilities.is_empty());
    }

    #[test]
    fn schema_summary_filters_migrations_and_shapes_admin_schema() {
        let migrations = vec![
            migration("001", "all-crm", None, "crm_primary"),
            migration("010", "crm-only", Some("crm"), "crm_primary"),
            migration("020", "other-source", Some("crm"), "analytics"),
        ];

        let schema = admin_schema_summary(
            AdminSchemaSummaryInput {
                id: "schema-crm".to_string(),
                name: "crm".to_string(),
                description: "CRM schema".to_string(),
                data_source_name: "crm_primary".to_string(),
                data_source_type: Some("postgres"),
                filter_capabilities: Some("filters"),
                migration_dialect: Some("postgresql"),
                data_access_configured: true,
                entity_count: 4,
                table_entity_count: 3,
                provider_capabilities: Vec::new(),
            },
            &migrations,
        );

        assert_eq!(schema.id, "schema-crm");
        assert_eq!(schema.name, "crm");
        assert_eq!(schema.data_source_type, Some("postgres"));
        assert_eq!(schema.filter_capabilities, Some("filters"));
        assert_eq!(
            schema
                .latest_migration
                .as_ref()
                .map(|migration| migration.id.as_str()),
            Some("010")
        );
        assert_eq!(schema.health.migration_count, 2);
        assert_eq!(schema.health.entity_count, 4);
        assert_eq!(schema.health.table_entity_count, 3);
        assert_eq!(schema.health.connectivity.status, "ok");
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn admin_model_loads_migrations_and_shapes_response() {
        let _guard = ADMIN_MIGRATIONS_ENV_LOCK
            .lock()
            .expect("admin migrations env lock");
        let previous = env::var_os(ADMIN_MIGRATIONS_DIR_ENV_VAR);
        env::remove_var(ADMIN_MIGRATIONS_DIR_ENV_VAR);

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let product_root = env::temp_dir().join(format!("appfw-admin-model-{unique}"));
        let backend_dir = product_root.join("backend");
        let migrations_dir = product_root.join("database/_pkg/migrations");
        fs::create_dir_all(&backend_dir).expect("backend dir should be created");
        fs::create_dir_all(&migrations_dir).expect("migrations dir should be created");
        fs::write(
            migrations_dir.join("manifest.yaml"),
            r#"
migrations:
  - id: "010"
    name: "add_accounts"
    schema: "crm"
    data_source: "crm_primary"
    dialect: "postgresql"
    phase: "baseline"
    path: "010_add_accounts.sql"
"#,
        )
        .expect("migration manifest should be written");

        let model = admin_model(&StaticModelProvider, &backend_dir, "1.2.3", true).await;

        assert_eq!(model.backend_version, "1.2.3");
        assert!(model.troubleshooting_enabled);
        assert_eq!(model.entity_types, vec!["Account", "Contact"]);
        assert_eq!(model.schemas.len(), 1);
        assert_eq!(model.schemas[0].name, "crm");
        assert_eq!(
            model.schemas[0]
                .latest_migration
                .as_ref()
                .map(|migration| migration.id.as_str()),
            Some("010")
        );
        assert_eq!(model.schemas[0].health.migration_count, 1);

        fs::remove_dir_all(&product_root).expect("temp product root should be removed");
        match previous {
            Some(value) => env::set_var(ADMIN_MIGRATIONS_DIR_ENV_VAR, value),
            None => env::remove_var(ADMIN_MIGRATIONS_DIR_ENV_VAR),
        }
    }

    #[test]
    fn provider_capabilities_shape_certification_evidence() {
        let capabilities = [ProviderCapability {
            area: ProviderContractArea::Sorting,
            status: CapabilityStatus::LiveCertified,
            evidence: &[CertificationEvidence::LiveContract(
                "provider_sort_contract",
            )],
        }];

        let shaped = admin_provider_capabilities(&capabilities);

        assert_eq!(shaped.len(), 1);
        assert_eq!(shaped[0].area_key, "sorting");
        assert_eq!(shaped[0].area_label, "sorting");
        assert_eq!(shaped[0].status, "live-certified");
        assert_eq!(shaped[0].reason, None);
        assert_eq!(shaped[0].evidence[0].kind, "live-contract");
        assert_eq!(shaped[0].evidence[0].contract, "provider_sort_contract");
    }

    #[test]
    fn provider_capabilities_for_provider_uses_runtime_profile() {
        let shaped = admin_provider_capabilities_for_provider(Some(FrameworkProvider::Postgres));

        assert!(!shaped.is_empty());
        assert!(shaped
            .iter()
            .any(|capability| capability.area_key == "sorting"));
        let anaplan = admin_provider_capabilities_for_provider(Some(FrameworkProvider::Anaplan));
        assert_eq!(anaplan.len(), SaasReadArea::ALL.len());
        assert!(anaplan.iter().any(|capability| {
            capability.area_key == "named_operation_registry"
                && capability.status == "compiler-contracted"
                && !capability.evidence.is_empty()
        }));
        assert!(anaplan.iter().any(|capability| {
            capability.area_key == "connection_auth"
                && capability.status == "unsupported"
                && capability
                    .reason
                    .is_some_and(|reason| reason.contains("live tenant evidence"))
        }));
        assert!(anaplan.iter().any(|capability| {
            capability.area_key == "delegated_actor_context"
                && capability.status == "unsupported"
                && capability
                    .reason
                    .is_some_and(|reason| reason.contains("per-user token storage"))
        }));
        let oracle =
            admin_provider_capabilities_for_provider(Some(FrameworkProvider::OracleFinancials));
        assert_eq!(oracle.len(), SaasReadArea::ALL.len());
        assert!(oracle.iter().any(|capability| {
            capability.area_key == "named_operation_registry"
                && capability.status == "compiler-contracted"
                && !capability.evidence.is_empty()
        }));
        assert!(oracle.iter().any(|capability| {
            capability.area_key == "connection_auth"
                && capability.status == "unsupported"
                && capability
                    .reason
                    .is_some_and(|reason| reason.contains("live tenant evidence"))
        }));
        let neo4j = admin_provider_capabilities_for_provider(Some(FrameworkProvider::Neo4j));
        assert!(neo4j
            .iter()
            .any(|capability| capability.area_key == "named_query_compilation"));
        assert!(admin_provider_capabilities_for_provider(None).is_empty());
    }

    #[test]
    fn filter_capabilities_for_provider_maps_runtime_data_types() {
        let shaped = admin_filter_capabilities_for_provider(
            "postgres",
            FrameworkProvider::Postgres,
            |data_type| data_type.key().to_string(),
        );

        assert_eq!(shaped.provider, "postgres");
        assert!(shaped
            .data_types
            .iter()
            .any(|capability| capability.data_type == "string"));
    }

    #[test]
    fn parses_admin_access_actions_case_insensitively() {
        assert_eq!(parse_admin_access_action("READ"), Some(AccessAction::Read));
        assert_eq!(parse_admin_access_action("bogus"), None);
    }

    #[test]
    fn policy_explain_result_shapes_subject_and_decision() {
        let result = admin_policy_explain_result(
            "crm.account",
            "crm",
            "Account",
            AdminPolicyDecision {
                allow: true,
                filter: Some(json!({ "owner_id": "user-1" })),
            },
        );

        assert_eq!(result.subject.policy_key, "crm.account");
        assert_eq!(result.subject.schema_name, "crm");
        assert_eq!(result.subject.type_name, "Account");
        assert!(result.decision.allow);
        assert_eq!(
            result.decision.filter,
            Some(json!({ "owner_id": "user-1" }))
        );
    }

    #[tokio::test]
    async fn policy_explain_fails_when_troubleshooting_disabled() {
        let err = explain_admin_policy(
            &StaticPolicyExplainProvider,
            false,
            request_context(),
            AdminPolicyExplainRequest {
                schema_name: "crm".to_string(),
                type_name: "Account".to_string(),
                action: "read".to_string(),
            },
            &user_with_roles(&["admin"]),
        )
        .await
        .expect_err("disabled troubleshooting should fail");

        assert_eq!(err.status, AdminServiceStatus::Forbidden);
        assert!(err
            .error
            .error
            .contains("admin troubleshooting is disabled"));
    }

    #[tokio::test]
    async fn policy_explain_rejects_unknown_actions() {
        let err = explain_admin_policy(
            &StaticPolicyExplainProvider,
            true,
            request_context(),
            AdminPolicyExplainRequest {
                schema_name: "crm".to_string(),
                type_name: "Account".to_string(),
                action: "archive".to_string(),
            },
            &user_with_roles(&["admin"]),
        )
        .await
        .expect_err("unknown action should fail");

        assert_eq!(err.status, AdminServiceStatus::BadRequest);
        assert!(err.error.error.contains("read, create, update, delete"));
    }

    #[tokio::test]
    async fn policy_explain_shapes_success_response() {
        let response = explain_admin_policy(
            &StaticPolicyExplainProvider,
            true,
            request_context(),
            AdminPolicyExplainRequest {
                schema_name: "crm".to_string(),
                type_name: "Account".to_string(),
                action: "READ".to_string(),
            },
            &user_with_roles(&["admin", "analyst"]),
        )
        .await
        .expect("policy explain should succeed");

        assert!(response.enabled);
        assert_eq!(response.policy_key, "crm.account");
        assert_eq!(response.schema_name, "crm");
        assert_eq!(response.type_name, "Account");
        assert_eq!(response.action, "read");
        assert_eq!(response.user_name, "casey");
        assert_eq!(response.roles, vec!["admin", "analyst"]);
        assert!(response.decision.allow);
        assert_eq!(
            response.decision.filter,
            Some(json!({ "owner_id": "user-1" }))
        );
    }

    #[tokio::test]
    async fn audit_timeline_fails_when_troubleshooting_disabled() {
        let err = admin_audit_timeline(
            &StaticAuditTimelineProvider {
                allow: true,
                audited: true,
                events: Vec::new(),
                message: None,
            },
            false,
            request_context(),
            AdminAuditTimelineRequest {
                schema_name: "crm".to_string(),
                type_name: "Account".to_string(),
                record_id: "account-1".to_string(),
                limit: None,
            },
            &user_with_roles(&["admin"]),
        )
        .await
        .expect_err("disabled troubleshooting should fail");

        assert_eq!(err.status, AdminServiceStatus::Forbidden);
        assert!(err
            .error
            .error
            .contains("admin troubleshooting is disabled"));
    }

    #[tokio::test]
    async fn audit_timeline_rejects_read_denied_policy() {
        let err = admin_audit_timeline(
            &StaticAuditTimelineProvider {
                allow: false,
                audited: true,
                events: vec![json!({ "event": "update" })],
                message: None,
            },
            true,
            request_context(),
            AdminAuditTimelineRequest {
                schema_name: "crm".to_string(),
                type_name: "Account".to_string(),
                record_id: "account-1".to_string(),
                limit: None,
            },
            &user_with_roles(&["admin"]),
        )
        .await
        .expect_err("read-denied policy should fail");

        assert_eq!(err.status, AdminServiceStatus::Forbidden);
        assert!(err.error.error.contains("read access denied"));
    }

    #[tokio::test]
    async fn audit_timeline_shapes_not_audited_response() {
        let response = admin_audit_timeline(
            &StaticAuditTimelineProvider {
                allow: true,
                audited: false,
                events: vec![json!({ "event": "ignored" })],
                message: None,
            },
            true,
            request_context(),
            AdminAuditTimelineRequest {
                schema_name: "crm".to_string(),
                type_name: "Account".to_string(),
                record_id: "account-1".to_string(),
                limit: None,
            },
            &user_with_roles(&["admin"]),
        )
        .await
        .expect("not-audited response should succeed");

        assert!(response.enabled);
        assert!(!response.audited);
        assert_eq!(response.schema_name, "crm");
        assert_eq!(response.type_name, "Account");
        assert_eq!(response.record_id, "account-1");
        assert!(response.events.is_empty());
        assert_eq!(
            response.message,
            Some("Entity is not configured with the audited facet.".to_string())
        );
        assert!(response.current_policy.allow);
    }

    #[tokio::test]
    async fn audit_timeline_shapes_audited_response() {
        let response = admin_audit_timeline(
            &StaticAuditTimelineProvider {
                allow: true,
                audited: true,
                events: vec![json!({ "event": "create" }), json!({ "event": "update" })],
                message: None,
            },
            true,
            request_context(),
            AdminAuditTimelineRequest {
                schema_name: "crm".to_string(),
                type_name: "Account".to_string(),
                record_id: "account-1".to_string(),
                limit: Some(1),
            },
            &user_with_roles(&["admin"]),
        )
        .await
        .expect("audited response should succeed");

        assert!(response.enabled);
        assert!(response.audited);
        assert_eq!(response.record_id, "account-1");
        assert_eq!(response.events.len(), 1);
        assert_eq!(response.events[0], json!({ "event": "create" }));
        assert!(response.message.is_none());
    }

    #[tokio::test]
    async fn audit_timeline_subject_shell_skips_loader_for_not_audited_entities() {
        let result = admin_audit_timeline_for_subject(
            AdminAuditTimelineSubject {
                schema_name: "crm".to_string(),
                type_name: "Account".to_string(),
            },
            AdminPolicyDecision {
                allow: true,
                filter: None,
            },
            false,
            Some("not audited".to_string()),
            || async { panic!("loader should not run for not-audited entities") },
        )
        .await
        .expect("not-audited shell should succeed");

        assert!(!result.audited);
        assert!(result.events.is_empty());
        assert_eq!(result.message, Some("not audited".to_string()));
    }

    #[tokio::test]
    async fn audit_timeline_subject_shell_loads_events_when_allowed_and_audited() {
        let result = admin_audit_timeline_for_subject(
            AdminAuditTimelineSubject {
                schema_name: "crm".to_string(),
                type_name: "Account".to_string(),
            },
            AdminPolicyDecision {
                allow: true,
                filter: None,
            },
            true,
            None,
            || async { Ok(vec![json!({ "event": "create" })]) },
        )
        .await
        .expect("audited shell should load events");

        assert!(result.audited);
        assert_eq!(result.events, vec![json!({ "event": "create" })]);
    }

    #[tokio::test]
    async fn query_diagnose_fails_when_troubleshooting_disabled() {
        let err = admin_query_diagnose(
            &StaticQueryDiagnoseProvider,
            false,
            request_context(),
            AdminQueryDiagnoseRequest {
                schema_name: "crm".to_string(),
                type_name: "Account".to_string(),
                filter: None,
                sort: None,
                skip: None,
                limit: None,
                after: None,
            },
            user_with_roles(&["admin"]),
        )
        .await
        .expect_err("disabled troubleshooting should fail");

        assert_eq!(err.status, AdminServiceStatus::Forbidden);
        assert!(err
            .error
            .error
            .contains("admin troubleshooting is disabled"));
    }

    #[tokio::test]
    async fn query_diagnose_rejects_invalid_pagination() {
        let err = admin_query_diagnose(
            &StaticQueryDiagnoseProvider,
            true,
            request_context(),
            AdminQueryDiagnoseRequest {
                schema_name: "crm".to_string(),
                type_name: "Account".to_string(),
                filter: None,
                sort: None,
                skip: Some(-1),
                limit: Some(10),
                after: None,
            },
            user_with_roles(&["admin"]),
        )
        .await
        .expect_err("invalid pagination should fail");

        assert_eq!(err.status, AdminServiceStatus::BadRequest);
        assert!(err.error.error.contains("pagination skip"));
    }

    #[tokio::test]
    async fn query_diagnose_shapes_success_response() {
        let response = admin_query_diagnose(
            &StaticQueryDiagnoseProvider,
            true,
            request_context(),
            AdminQueryDiagnoseRequest {
                schema_name: "crm".to_string(),
                type_name: "Account".to_string(),
                filter: Some(json!({ "name": { "eq": "Acme" } })),
                sort: Some(json!([{ "name": "asc" }])),
                skip: Some(5),
                limit: Some(10),
                after: Some("cursor-1".to_string()),
            },
            user_with_roles(&["admin"]),
        )
        .await
        .expect("query diagnose should succeed");

        assert!(response.enabled);
        assert_eq!(response.diagnostic.schema_name, "crm");
        assert_eq!(response.diagnostic.type_name, "Account");
        assert_eq!(response.diagnostic.pagination.strategy, "keyset");
        assert_eq!(response.diagnostic.pagination.skip, 5);
        assert_eq!(response.diagnostic.pagination.limit, 10);
        assert!(response.diagnostic.pagination.after_present);
        assert!(response.diagnostic.access_filter_applied);
        assert_eq!(
            response.diagnostic.provider_diagnostic,
            json!({ "sort_present": true, "user_name": "casey" })
        );
    }

    #[tokio::test]
    async fn query_diagnose_redacts_provider_diagnostic_payload() {
        let response = admin_query_diagnose(
            &LeakyQueryDiagnoseProvider,
            true,
            request_context(),
            AdminQueryDiagnoseRequest {
                schema_name: "crm".to_string(),
                type_name: "Account".to_string(),
                filter: None,
                sort: None,
                skip: Some(0),
                limit: Some(10),
                after: None,
            },
            user_with_roles(&["admin"]),
        )
        .await
        .expect("query diagnose should succeed");

        let provider_diagnostic = response.diagnostic.provider_diagnostic;
        assert_eq!(
            provider_diagnostic["nested"]["safe"],
            json!("query plan available")
        );
        assert_eq!(
            provider_diagnostic["nested"]["private_key"],
            json!("[REDACTED]")
        );

        let serialized = provider_diagnostic.to_string();
        assert!(serialized.contains("[REDACTED]"));
        assert!(!serialized.contains("short"));
        assert!(!serialized.contains("pem"));
        assert!(!serialized.contains("jwt.value"));
    }
}
