use appfw_runtime::{
    admin::{
        admin_audit_timeline_for_subject, admin_filter_capabilities_for_provider,
        admin_migration_dialect, admin_missing_data_access_error, admin_policy_explain_result,
        admin_provider_capabilities_for_provider, admin_runtime_routes, admin_schema_summary,
        AdminAuditTimelineProvider, AdminAuditTimelineResult, AdminAuditTimelineSubject,
        AdminMigration, AdminModelProvider, AdminPolicyDecision as PolicyDecision,
        AdminPolicyExplainProvider, AdminPolicyExplainResult, AdminQueryDiagnoseProvider,
        AdminRuntimeState, AdminSchema, AdminSchemaSummaryInput, AdminServiceError,
    },
    data_access::RuntimeQueryPlanDiagnostic,
    extension::UserAuth,
    observability::RequestContext,
    record_audit as runtime_audit,
    security::SecurityConfig,
    AccessAction, RuntimeAuthState, RuntimeFilterCapabilities,
};
use async_trait::async_trait;
use axum::Router;
use serde_json::Value;
use std::{collections::HashMap, env, sync::Arc};

use crate::{
    config::app_config::AppConfig,
    data::data_access::DataAccess,
    product_api::{product_data_type, runtime_entity_metadata, runtime_provider},
    schemas::system::{DataSourceType, EntityType},
};

#[derive(Clone)]
struct AdminState {
    app_config: Arc<AppConfig>,
    app_state: RuntimeAuthState,
    data_access_by_schema: HashMap<String, Arc<DataAccess>>,
    troubleshooting_enabled: bool,
}

type ProductAdminSchema = AdminSchema<DataSourceType, FilterCapabilities>;
type FilterCapabilities =
    RuntimeFilterCapabilities<DataSourceType, crate::schemas::system::DataType>;

pub fn get_routes(
    app_config: Arc<AppConfig>,
    app_state: RuntimeAuthState,
    security: SecurityConfig,
    data_access_by_schema: HashMap<String, Arc<DataAccess>>,
) -> Router {
    admin_runtime_routes::<AdminState>().with_state(AdminState {
        app_config,
        app_state,
        data_access_by_schema,
        troubleshooting_enabled: security.admin_troubleshooting_enabled,
    })
}

struct ProductAdminModelProvider<'a> {
    state: &'a AdminState,
}

impl AdminRuntimeState for AdminState {
    type ModelProvider<'a>
        = ProductAdminModelProvider<'a>
    where
        Self: 'a;
    type PolicyExplainProvider<'a>
        = ProductAdminPolicyExplainProvider<'a>
    where
        Self: 'a;
    type AuditTimelineProvider<'a>
        = ProductAdminAuditTimelineProvider<'a>
    where
        Self: 'a;
    type QueryDiagnoseProvider<'a>
        = ProductAdminQueryDiagnoseProvider<'a>
    where
        Self: 'a;

    fn product_manifest_dir() -> &'static str {
        env!("CARGO_MANIFEST_DIR")
    }

    fn backend_version() -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    fn auth_state(&self) -> RuntimeAuthState {
        self.app_state.clone()
    }

    fn troubleshooting_enabled(&self) -> bool {
        self.troubleshooting_enabled
    }

    fn model_provider(&self) -> Self::ModelProvider<'_> {
        ProductAdminModelProvider { state: self }
    }

    fn policy_explain_provider(&self) -> Self::PolicyExplainProvider<'_> {
        ProductAdminPolicyExplainProvider { state: self }
    }

    fn audit_timeline_provider(&self) -> Self::AuditTimelineProvider<'_> {
        ProductAdminAuditTimelineProvider { state: self }
    }

    fn query_diagnose_provider(&self) -> Self::QueryDiagnoseProvider<'_> {
        ProductAdminQueryDiagnoseProvider { state: self }
    }
}

impl AdminModelProvider for ProductAdminModelProvider<'_> {
    type EntityType = EntityType;
    type DataSourceType = DataSourceType;
    type FilterCapabilities = FilterCapabilities;

    fn admin_schemas(&self, migrations: &[AdminMigration]) -> Vec<ProductAdminSchema> {
        admin_schemas(
            &self.state.app_config,
            migrations,
            &self.state.data_access_by_schema,
        )
    }

    fn admin_entity_types(&self) -> Vec<Self::EntityType> {
        self.state.app_config.get_all_entity_types()
    }
}

struct ProductAdminPolicyExplainProvider<'a> {
    state: &'a AdminState,
}

#[async_trait]
impl AdminPolicyExplainProvider for ProductAdminPolicyExplainProvider<'_> {
    async fn explain_policy_access(
        &self,
        schema_name: &str,
        type_name: &str,
        action: AccessAction,
        user: &UserAuth,
        request_context: &RequestContext,
    ) -> Result<AdminPolicyExplainResult, AdminServiceError> {
        let schema_name = schema_name.to_string();
        let type_name = type_name.to_string();
        let entity_type = self
            .state
            .app_config
            .get_entity_type(&schema_name, &type_name)
            .map_err(|err| AdminServiceError::bad_request(err.to_string(), request_context))?;
        let access = self
            .state
            .app_config
            .evaluate_user_access(entity_type.clone(), action, user)
            .map_err(|err| AdminServiceError::internal(err.to_string(), request_context))?;

        Ok(admin_policy_explain_result(
            format!("{}.{}", entity_type.schema_name, entity_type.snake_1),
            entity_type.schema_name.clone(),
            entity_type.pascal_1.clone(),
            access.into(),
        ))
    }
}

struct ProductAdminAuditTimelineProvider<'a> {
    state: &'a AdminState,
}

#[async_trait]
impl AdminAuditTimelineProvider for ProductAdminAuditTimelineProvider<'_> {
    async fn load_audit_timeline(
        &self,
        schema_name: &str,
        type_name: &str,
        record_id: &str,
        limit: i64,
        user: &UserAuth,
        request_context: &RequestContext,
    ) -> Result<AdminAuditTimelineResult, AdminServiceError> {
        let schema_name = schema_name.to_string();
        let type_name = type_name.to_string();
        let entity_type = self
            .state
            .app_config
            .get_entity_type(&schema_name, &type_name)
            .map_err(|err| AdminServiceError::bad_request(err.to_string(), request_context))?;
        let current_access = self
            .state
            .app_config
            .evaluate_user_access(entity_type.clone(), AccessAction::Read, user)
            .map_err(|err| AdminServiceError::internal(err.to_string(), request_context))?;
        let current_policy: PolicyDecision = current_access.clone().into();

        let subject = AdminAuditTimelineSubject {
            schema_name: entity_type.schema_name.clone(),
            type_name: entity_type.pascal_1.clone(),
        };

        admin_audit_timeline_for_subject(
            subject,
            current_policy,
            runtime_audit::is_audited(&runtime_entity_metadata(&entity_type)),
            None,
            || async {
                let Some(data_access) = self
                    .state
                    .data_access_by_schema
                    .get(&entity_type.schema_name)
                else {
                    return Err(admin_missing_data_access_error(
                        &entity_type.schema_name,
                        request_context,
                    ));
                };

                data_access
                    .query_audit_events(
                        entity_type.clone(),
                        record_id,
                        limit,
                        user,
                        &current_access,
                    )
                    .await
                    .map_err(|err| AdminServiceError::internal(err.to_string(), request_context))
            },
        )
        .await
    }
}

struct ProductAdminQueryDiagnoseProvider<'a> {
    state: &'a AdminState,
}

#[async_trait]
impl AdminQueryDiagnoseProvider for ProductAdminQueryDiagnoseProvider<'_> {
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
    ) -> Result<RuntimeQueryPlanDiagnostic, AdminServiceError> {
        let schema_name = schema_name.to_string();
        let type_name = type_name.to_string();
        let entity_type = self
            .state
            .app_config
            .get_entity_type(&schema_name, &type_name)
            .map_err(|err| AdminServiceError::bad_request(err.to_string(), request_context))?;

        let Some(data_access) = self
            .state
            .data_access_by_schema
            .get(&entity_type.schema_name)
        else {
            return Err(admin_missing_data_access_error(
                &entity_type.schema_name,
                request_context,
            ));
        };

        data_access
            .diagnose_query(entity_type, filter, sort, skip, limit, after, user)
            .await
            .map_err(|err| AdminServiceError::bad_request(err.to_string(), request_context))
    }
}

fn admin_schemas(
    app_config: &AppConfig,
    migrations: &[AdminMigration],
    data_access_by_schema: &HashMap<String, Arc<DataAccess>>,
) -> Vec<ProductAdminSchema> {
    let entity_types = app_config.get_all_entity_types();

    app_config
        .get_schemas()
        .into_iter()
        .map(|schema| {
            let data_source_type = app_config
                .get_data_source_type(&schema.data_source_name)
                .ok();
            let entity_count = entity_types
                .iter()
                .filter(|entity| entity.schema_name == schema.name && !entity.is_union)
                .count();
            let table_entity_count = entity_types
                .iter()
                .filter(|entity| {
                    entity.schema_name == schema.name && entity.is_table && !entity.is_union
                })
                .count();
            let framework_provider = data_source_type.map(runtime_provider);
            let filter_capabilities =
                data_source_type
                    .zip(framework_provider)
                    .map(|(provider, framework_provider)| {
                        admin_filter_capabilities_for_provider(
                            provider,
                            framework_provider,
                            product_data_type,
                        )
                    });
            let migration_dialect = framework_provider.map(admin_migration_dialect);
            let data_access_configured = data_access_by_schema.contains_key(&schema.name);
            let provider_capabilities =
                admin_provider_capabilities_for_provider(framework_provider);

            admin_schema_summary(
                AdminSchemaSummaryInput {
                    id: schema.id,
                    name: schema.name,
                    description: schema.description,
                    data_source_name: schema.data_source_name,
                    data_source_type,
                    filter_capabilities,
                    migration_dialect,
                    data_access_configured,
                    entity_count,
                    table_entity_count,
                    provider_capabilities,
                },
                migrations,
            )
        })
        .collect()
}
