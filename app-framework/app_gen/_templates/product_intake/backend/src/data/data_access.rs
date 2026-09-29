use std::{env, sync::Arc, time::Instant};

use anyhow::Result;
use appfw_runtime::data_access as runtime_data_access;
use appfw_runtime::extension::UserAuth;
use appfw_runtime::json as json_utils;
use appfw_runtime::observability::{current_request_context, MetricsRegistry};
use appfw_runtime::record_audit as runtime_audit;
use appfw_runtime::record_locator::{validate_record_locator, RECORD_LOCATOR_FIELD};
use appfw_runtime::record_validation as runtime_validation;
use appfw_runtime::{
    AccessAction, PolicyAccess, RuntimeProviderDescriptor, RuntimeProviderIdentity,
    RuntimeProviderOperation, RuntimeProviderOperationCounts, RuntimeProviderPlanInput,
};
use serde_json::{json, Value};
use tracing::debug;

use crate::config::app_config::AppConfig;
use crate::data::query_ir::PaginationPolicy;
use crate::data::query_ir::{cost_for_query, AggregatePlan, MutationPlan, QueryPlan};
use crate::data::rules;
use crate::product_api::runtime_entity_metadata;
use crate::routes::app_error::AppError;
use crate::schemas::common::{AggregateResult, JsonAggregateResult, JsonQueryResult, QueryResult};
use crate::schemas::system::{EntityType, PropertyType};

use super::clients::database_client::{
    DatabaseClientBox, DatabaseClientRuntimeAdapter, ProviderRoutineArgument, ProviderRoutineCall,
};

pub struct DataAccess {
    pub app_config: Arc<AppConfig>,
    client: DatabaseClientBox,
    metrics: MetricsRegistry,
}

pub(crate) type QueryPlanDiagnostic = runtime_data_access::RuntimeQueryPlanDiagnostic;

impl DataAccess {
    pub fn init(
        app_config: Arc<AppConfig>,
        client: DatabaseClientBox,
        metrics: MetricsRegistry,
    ) -> DataAccess {
        Self {
            app_config: app_config.clone(),
            client,
            metrics,
        }
    }

    pub async fn query_audit_events(
        &self,
        entity_type: Arc<EntityType>,
        record_id: &str,
        limit: i64,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<Vec<Value>, AppError> {
        let pk_name = self.app_config.get_primary_key_name(entity_type.clone())?;
        let selections = serde_json::json!({
            "name": entity_type.snake_n,
            "selection_set": [
                { "name": pk_name, "selection_set": [] }
            ]
        });
        let query = runtime_audit::audit_query(
            &runtime_entity_metadata(&entity_type),
            &user.tenant_id,
            record_id,
            limit,
        );
        let provider = self.runtime_provider();
        runtime_data_access::execute_audit_events_read(
            &provider,
            entity_type,
            selections,
            record_id.to_string(),
            user,
            access,
            query,
        )
        .await
    }

    pub fn provider_descriptor(&self) -> RuntimeProviderDescriptor {
        self.runtime_provider().provider_descriptor()
    }

    pub async fn health_check(&self) -> Result<(), AppError> {
        let provider = self.runtime_provider();
        runtime_data_access::provider_health_check(&provider).await
    }

    pub fn pool_stats(&self) -> appfw_runtime::ProviderPoolStats {
        self.runtime_provider().pool_stats()
    }

    /// Read the governed Neo4j relationship graph for an account record locator.
    /// Tenant scoping and policy access are enforced by the graph provider.
    #[cfg(feature = "provider-neo4j")]
    pub async fn account_relationship_graph(
        &self,
        entity_type: &Arc<EntityType>,
        user: &UserAuth,
        account: String,
    ) -> Result<Value, AppError> {
        super::clients::neo4j::read_account_graph(
            &self.app_config,
            entity_type,
            user,
            super::clients::neo4j::ACCOUNT_RELATIONSHIP_GRAPH,
            account,
        )
        .await
    }

    /// Report graph operations as unsupported when the backend build excludes Neo4j.
    #[cfg(not(feature = "provider-neo4j"))]
    pub async fn account_relationship_graph(
        &self,
        _entity_type: &Arc<EntityType>,
        _user: &UserAuth,
        _account: String,
    ) -> Result<Value, AppError> {
        Err(AppError::DataAccess(
            "account_relationship_graph requires backend feature `provider-neo4j`".to_string(),
        ))
    }

    /// Create a governed `REFERRED` relationship between two account locators.
    #[cfg(feature = "provider-neo4j")]
    pub async fn link_account_referral(
        &self,
        entity_type: &Arc<EntityType>,
        user: &UserAuth,
        from_account: String,
        to_account: String,
    ) -> Result<Value, AppError> {
        super::clients::neo4j::write_account_link(
            &self.app_config,
            entity_type,
            user,
            super::clients::neo4j::LINK_ACCOUNT_REFERRAL,
            from_account,
            to_account,
        )
        .await
    }

    /// Report graph writes as unsupported when the backend build excludes Neo4j.
    #[cfg(not(feature = "provider-neo4j"))]
    pub async fn link_account_referral(
        &self,
        _entity_type: &Arc<EntityType>,
        _user: &UserAuth,
        _from_account: String,
        _to_account: String,
    ) -> Result<Value, AppError> {
        Err(AppError::DataAccess(
            "link_account_referral requires backend feature `provider-neo4j`".to_string(),
        ))
    }

    /// Remove a governed `REFERRED` relationship between two account locators.
    #[cfg(feature = "provider-neo4j")]
    pub async fn unlink_account_referral(
        &self,
        entity_type: &Arc<EntityType>,
        user: &UserAuth,
        from_account: String,
        to_account: String,
    ) -> Result<Value, AppError> {
        super::clients::neo4j::write_account_link(
            &self.app_config,
            entity_type,
            user,
            super::clients::neo4j::UNLINK_ACCOUNT_REFERRAL,
            from_account,
            to_account,
        )
        .await
    }

    /// Report graph writes as unsupported when the backend build excludes Neo4j.
    #[cfg(not(feature = "provider-neo4j"))]
    pub async fn unlink_account_referral(
        &self,
        _entity_type: &Arc<EntityType>,
        _user: &UserAuth,
        _from_account: String,
        _to_account: String,
    ) -> Result<Value, AppError> {
        Err(AppError::DataAccess(
            "unlink_account_referral requires backend feature `provider-neo4j`".to_string(),
        ))
    }

    #[allow(dead_code)]
    #[tracing::instrument(skip(self, entity_type, _selections, routine, arguments, user), fields(entity = %entity_type.pascal_1, method = routine.method_name, action = %action))]
    pub async fn call_provider_routine<R>(
        &self,
        entity_type: Arc<EntityType>,
        _selections: Value,
        routine: ProviderRoutineCall,
        arguments: Vec<ProviderRoutineArgument>,
        user: Option<UserAuth>,
        action: AccessAction,
    ) -> Result<R, AppError>
    where
        R: std::marker::Send
            + std::marker::Sync
            + async_graphql::OutputType
            + for<'de> serde::Deserialize<'de>
            + std::fmt::Debug,
    {
        let user = self
            .require_user(entity_type.clone(), action, user, None)
            .await?;
        let access = self
            .authorize_entity_access(entity_type.clone(), action, &user, None, None)
            .await?;

        if let Some(expected_data_source) = routine.data_source_name {
            let actual_data_source = self.provider_descriptor().data_source_name().to_string();
            if expected_data_source != actual_data_source {
                return Err(AppError::DataAccess(format!(
                    "provider routine `{}` expects data source `{}`, but runtime is using `{}`",
                    routine.method_name, expected_data_source, actual_data_source
                )));
            }
        }

        let value = self
            .client
            .call_provider_routine_json(&routine, &arguments, &user, &access)
            .await?;
        serde_json::from_value::<R>(value).map_err(|err| {
            AppError::DataAccess(format!(
                "provider routine `{}` returned a payload incompatible with the declared return type: {err}",
                routine.method_name
            ))
        })
    }

    fn runtime_provider(&self) -> DatabaseClientRuntimeAdapter<'_> {
        self.client.as_ref().as_runtime_provider()
    }

    #[tracing::instrument(
        skip(self, entity_type, filter, sort, user),
        fields(entity = %entity_type.pascal_1, skip = skip, limit = limit, after_present = after.is_some())
    )]
    pub(crate) async fn diagnose_query(
        &self,
        entity_type: Arc<EntityType>,
        filter: Option<Value>,
        sort: Option<Value>,
        skip: i32,
        limit: i32,
        after: Option<String>,
        user: UserAuth,
    ) -> Result<QueryPlanDiagnostic, AppError> {
        let access = self
            .authorize_entity_access(entity_type.clone(), AccessAction::Read, &user, None, None)
            .await?;
        let pk_name = self.app_config.get_primary_key_name(entity_type.clone())?;
        let selections = json!({
            "name": entity_type.snake_n,
            "selection_set": [
                { "name": pk_name, "selection_set": [] }
            ]
        });

        let plan = if after.is_some() {
            QueryPlan::new_keyset(
                self.app_config.clone(),
                entity_type.clone(),
                selections,
                filter,
                sort,
                after,
                limit,
                &access,
            )?
        } else {
            QueryPlan::new(
                self.app_config.clone(),
                entity_type.clone(),
                selections,
                filter,
                sort,
                skip,
                limit,
                &access,
            )?
        };
        let budget = appfw_runtime::QueryCostBudget::from_env();
        let cost = cost_for_query(&plan);
        let pagination = runtime_data_access::pagination_diagnostic(&plan.pagination);
        let provider_descriptor = self.provider_descriptor();
        let provider_plan = plan.clone().into_runtime_provider_plan();
        let provider = self.runtime_provider();
        let provider_diagnostic = runtime_data_access::provider_explain_query_plan(
            &provider,
            RuntimeProviderPlanInput::new(&provider_plan, &user, &access),
        )
        .await?;

        Ok(runtime_data_access::RuntimeQueryPlanDiagnostic {
            schema_name: entity_type.schema_name.clone(),
            type_name: entity_type.pascal_1.clone(),
            provider: provider_descriptor.provider_key().to_string(),
            data_source: provider_descriptor.data_source_name().to_string(),
            pagination,
            access_filter_applied: plan.access_filter.is_some(),
            cost,
            budget,
            provider_diagnostic,
        })
    }

    #[tracing::instrument(skip(self, entity_type, selections, record, user), fields(entity = %entity_type.pascal_1))]
    pub async fn create_item<T, R>(
        &self,
        entity_type: Arc<EntityType>,
        selections: Value,
        record: T,
        user: Option<UserAuth>,
    ) -> Result<R, AppError>
    where
        T: std::marker::Send
            + std::marker::Sync
            + async_graphql::InputType
            + for<'de> serde::Serialize
            + std::fmt::Debug,
        R: std::marker::Send
            + std::marker::Sync
            + async_graphql::OutputType
            + for<'de> serde::Deserialize<'de>
            + std::fmt::Debug,
    {
        debug!("creating item");
        let input = json_utils::t_to_json_obj(record);
        let input_record_id =
            runtime_audit::record_id(&runtime_entity_metadata(&entity_type), &input);
        let user = self
            .require_user(
                entity_type.clone(),
                AccessAction::Create,
                user,
                input_record_id.clone(),
            )
            .await?;
        let access = self
            .authorize_entity_access(
                entity_type.clone(),
                AccessAction::Create,
                &user,
                input_record_id.clone(),
                Some(Value::Object(input.clone())),
            )
            .await?;

        let audit_enabled = runtime_audit::is_audited(&runtime_entity_metadata(&entity_type));

        // Apply all configured rules. If applicable, sets initial version on record to persist.
        let evaluated_input =
            rules::evaluate(entity_type.clone(), input, AccessAction::Create, &user)?;
        self.validate_primary_key_available(
            entity_type.clone(),
            &evaluated_input,
            &user,
            &access,
        )
        .await?;
        self.validate_foreign_keys(entity_type.clone(), &evaluated_input, &user)
            .await?;
        self.validate_uniqueness(entity_type.clone(), &evaluated_input, &user, &access)
            .await?;

        // Call data source client to handle the persist operation.
        let plan = MutationPlan::create(
            self.app_config.clone(),
            entity_type.clone(),
            selections,
            evaluated_input.clone(),
            &access,
        )?
        .into_runtime_provider_plan();

        let provider = self.runtime_provider();
        let client_res = match runtime_data_access::execute_create_item_plan_mutation(
            &provider,
            plan,
            &user,
            &access,
            |operation, started_at, counts| {
                self.trace_provider_operation(entity_type.as_ref(), operation, started_at, counts)
            },
        )
        .await
        {
            Ok(client_res) => client_res,
            Err(err) => {
                self.append_operation_failed_audit_attempt(
                    entity_type.clone(),
                    AccessAction::Create,
                    &user,
                    input_record_id,
                    None,
                    Some(Value::Object(evaluated_input)),
                    &err,
                )
                .await?;
                return Err(err);
            }
        };

        if audit_enabled {
            let record_id = runtime_data_access::mutation_record_id(
                &runtime_entity_metadata(&entity_type),
                runtime_data_access::RuntimeMutationKind::Create,
                Some(&evaluated_input),
                Some(&client_res),
                None,
            );
            self.append_audit_mutation(
                entity_type.clone(),
                AccessAction::Create,
                &user,
                record_id,
                None,
                Some(Value::Object(client_res.clone())),
                &access,
            )
            .await?;
        }

        let res = json_utils::json_obj_to_t::<R>(client_res);

        debug!("created item");

        Ok(res)
    }

    #[tracing::instrument(skip(self, entity_type, selections, record, user), fields(entity = %entity_type.pascal_1))]
    pub async fn update_item<T, R>(
        &self,
        entity_type: Arc<EntityType>,
        selections: Value,
        record: T,
        user: Option<UserAuth>,
    ) -> Result<R, AppError>
    where
        T: std::marker::Send
            + std::marker::Sync
            + async_graphql::InputType
            + for<'de> serde::Serialize
            + std::fmt::Debug,
        R: std::marker::Send
            + std::marker::Sync
            + async_graphql::OutputType
            + for<'de> serde::Deserialize<'de>
            + std::fmt::Debug,
    {
        debug!("updating item");
        let input = json_utils::t_to_json_obj(record);
        let input_record_id =
            runtime_audit::record_id(&runtime_entity_metadata(&entity_type), &input);
        let user = self
            .require_user(
                entity_type.clone(),
                AccessAction::Update,
                user,
                input_record_id.clone(),
            )
            .await?;
        let access = self
            .authorize_entity_access(
                entity_type.clone(),
                AccessAction::Update,
                &user,
                input_record_id.clone(),
                Some(Value::Object(input.clone())),
            )
            .await?;

        let audit_enabled = runtime_audit::is_audited(&runtime_entity_metadata(&entity_type));
        let access_visible_before = self
            .record_before_with_access(entity_type.clone(), &input, &user, &access)
            .await?;
        if runtime_data_access::should_check_filtered_update_denial(
            access.filter.as_ref(),
            input_record_id.as_deref(),
            access_visible_before.as_ref(),
        ) {
            self.append_policy_denied_audit_attempt_on_record_chain(
                entity_type.clone(),
                AccessAction::Update,
                &user,
                input_record_id,
                None,
                Some(Value::Object(input.clone())),
                &access,
            )
            .await?;
            return Err(AppError::AccessDenied);
        }
        let audit_before = if audit_enabled {
            access_visible_before
        } else {
            None
        };

        // Record version: Capture to pass to client operation
        let read_version = rules::get_record_version(entity_type.clone(), &input)?;

        // Apply all configured rules. If applicable, updates version on record to persist.
        let evaluated_input = rules::evaluate(
            entity_type.clone(),
            input.clone(),
            AccessAction::Update,
            &user,
        )?;
        self.validate_foreign_keys(entity_type.clone(), &evaluated_input, &user)
            .await?;
        self.validate_uniqueness(entity_type.clone(), &evaluated_input, &user, &access)
            .await?;

        // Call data source client to handle the persist operation.
        let plan = MutationPlan::update(
            self.app_config.clone(),
            entity_type.clone(),
            selections,
            evaluated_input.clone(),
            read_version,
            &access,
        )?
        .into_runtime_provider_plan();

        let provider = self.runtime_provider();
        let client_res = match runtime_data_access::execute_update_item_plan_mutation(
            &provider,
            plan,
            &user,
            &access,
            |operation, started_at, counts| {
                self.trace_provider_operation(entity_type.as_ref(), operation, started_at, counts)
            },
        )
        .await
        {
            Ok(client_res) => client_res,
            Err(err) => {
                if matches!(err, AppError::AccessDenied) {
                    self.append_policy_denied_audit_attempt_on_record_chain(
                        entity_type.clone(),
                        AccessAction::Update,
                        &user,
                        runtime_audit::record_id(&runtime_entity_metadata(&entity_type), &input),
                        audit_before.clone().map(Value::Object),
                        Some(Value::Object(evaluated_input)),
                        &access,
                    )
                    .await?;
                    return Err(err);
                }
                self.append_operation_failed_audit_attempt(
                    entity_type.clone(),
                    AccessAction::Update,
                    &user,
                    input_record_id,
                    audit_before.clone().map(Value::Object),
                    Some(Value::Object(evaluated_input)),
                    &err,
                )
                .await?;
                return Err(err);
            }
        };

        if audit_enabled {
            let record_id = runtime_data_access::mutation_record_id(
                &runtime_entity_metadata(&entity_type),
                runtime_data_access::RuntimeMutationKind::Update,
                Some(&input),
                Some(&client_res),
                audit_before.as_ref(),
            );
            self.append_audit_mutation(
                entity_type.clone(),
                AccessAction::Update,
                &user,
                record_id,
                audit_before.map(Value::Object),
                Some(Value::Object(client_res.clone())),
                &access,
            )
            .await?;
        }

        let res = json_utils::json_obj_to_t::<R>(client_res);

        debug!("updated item");

        Ok(res)
    }

    #[tracing::instrument(skip(self, entity_type, record, user), fields(entity = %entity_type.pascal_1))]
    pub async fn delete_item<T>(
        &self,
        entity_type: Arc<EntityType>,
        record: T,
        user: Option<UserAuth>,
    ) -> Result<i64, AppError>
    where
        T: std::marker::Send
            + std::marker::Sync
            + async_graphql::InputType
            + for<'de> serde::Serialize
            + std::fmt::Debug,
    {
        debug!("deleting item");
        let input = json_utils::t_to_json_obj(record);
        let input_record_id =
            runtime_audit::record_id(&runtime_entity_metadata(&entity_type), &input);
        let user = self
            .require_user(
                entity_type.clone(),
                AccessAction::Delete,
                user,
                input_record_id.clone(),
            )
            .await?;
        let access = self
            .authorize_entity_access(
                entity_type.clone(),
                AccessAction::Delete,
                &user,
                input_record_id.clone(),
                Some(Value::Object(input.clone())),
            )
            .await?;

        let audit_enabled = runtime_audit::is_audited(&runtime_entity_metadata(&entity_type));
        let audit_before = if audit_enabled {
            self.audit_before(entity_type.clone(), &input, &user, &access)
                .await?
        } else {
            None
        };

        // Record version: Capture to pass to client operation
        let read_version = rules::get_record_version(entity_type.clone(), &input)?;

        // Call data source client to handle the persist operation.
        let plan = MutationPlan::delete(
            self.app_config.clone(),
            entity_type.clone(),
            input.clone(),
            read_version,
            &access,
        )?
        .into_runtime_provider_plan();

        let provider = self.runtime_provider();
        let client_res = match runtime_data_access::execute_delete_item_plan_mutation(
            &provider,
            plan,
            &user,
            &access,
            |operation, started_at, counts| {
                self.trace_provider_operation(entity_type.as_ref(), operation, started_at, counts)
            },
        )
        .await
        {
            Ok(client_res) => client_res,
            Err(err) => {
                self.append_operation_failed_audit_attempt(
                    entity_type.clone(),
                    AccessAction::Delete,
                    &user,
                    input_record_id.clone(),
                    audit_before.clone().map(Value::Object),
                    None,
                    &err,
                )
                .await?;
                return Err(err);
            }
        };

        match runtime_data_access::delete_audit_outcome(audit_enabled, client_res) {
            Some(runtime_data_access::RuntimeDeleteAuditOutcome::Mutation) => {
                let record_id = runtime_data_access::mutation_record_id(
                    &runtime_entity_metadata(&entity_type),
                    runtime_data_access::RuntimeMutationKind::Delete,
                    Some(&input),
                    None,
                    audit_before.as_ref(),
                );
                self.append_audit_mutation(
                    entity_type.clone(),
                    AccessAction::Delete,
                    &user,
                    record_id,
                    audit_before.map(Value::Object),
                    None,
                    &access,
                )
                .await?;
            }
            Some(runtime_data_access::RuntimeDeleteAuditOutcome::NotApplied) => {
                let record_id = runtime_data_access::mutation_record_id(
                    &runtime_entity_metadata(&entity_type),
                    runtime_data_access::RuntimeMutationKind::Delete,
                    Some(&input),
                    None,
                    audit_before.as_ref(),
                );
                self.append_operation_not_applied_audit_attempt(
                    entity_type.clone(),
                    AccessAction::Delete,
                    &user,
                    record_id,
                    audit_before.map(Value::Object),
                )
                .await?;
            }
            None => {}
        }

        debug!(deleted_count = client_res, "deleted item");

        Ok(client_res)
    }

    #[tracing::instrument(skip(self, entity_type, selections, id, user), fields(entity = %entity_type.pascal_1, has_id = !id.is_empty()))]
    pub(crate) async fn find_item_json_value(
        &self,
        entity_type: Arc<EntityType>,
        selections: Value,
        id: String,
        user: Option<UserAuth>,
    ) -> Result<Option<json_utils::JsonObj>, AppError> {
        debug!("finding item");
        let user = self
            .require_user(
                entity_type.clone(),
                AccessAction::Read,
                user,
                Some(id.clone()),
            )
            .await?;
        let access = self
            .authorize_entity_access(
                entity_type.clone(),
                AccessAction::Read,
                &user,
                Some(id.clone()),
                None,
            )
            .await?;

        let provider = self.runtime_provider();
        runtime_data_access::execute_find_item_read(
            &provider,
            entity_type.clone(),
            selections,
            id,
            &user,
            &access,
            |operation, started_at, counts| {
                self.trace_provider_operation(entity_type.as_ref(), operation, started_at, counts)
            },
            |record| rules::evaluate(entity_type.clone(), record, AccessAction::Read, &user),
        )
        .await
    }

    #[tracing::instrument(skip(self, entity_type, selections, id, user), fields(entity = %entity_type.pascal_1, has_id = !id.is_empty()))]
    pub async fn find_item<T>(
        &self,
        entity_type: Arc<EntityType>,
        selections: Value,
        id: String,
        user: Option<UserAuth>,
    ) -> Result<Option<T>, AppError>
    where
        T: std::marker::Send
            + std::marker::Sync
            + async_graphql::OutputType
            + for<'de> serde::Deserialize<'de>
            + std::fmt::Debug,
    {
        Ok(self
            .find_item_json_value(entity_type, selections, id, user)
            .await?
            .map(json_utils::json_obj_to_t::<T>))
    }

    pub async fn find_item_by_locator<T>(
        &self,
        entity_type: Arc<EntityType>,
        selections: Value,
        locator: String,
        user: Option<UserAuth>,
    ) -> Result<Option<T>, AppError>
    where
        T: std::marker::Send
            + std::marker::Sync
            + async_graphql::OutputType
            + for<'de> serde::Deserialize<'de>
            + std::fmt::Debug,
    {
        validate_record_locator(&locator)?;
        let filter = json!({ RECORD_LOCATOR_FIELD: { "_eq": locator } });
        let result = self
            .query_items::<T>(
                entity_type,
                selections,
                Some(filter),
                None,
                0,
                1,
                None,
                user,
            )
            .await?;
        Ok(result.items.into_iter().next())
    }

    #[tracing::instrument(skip(self, entity_type, selections, filter, user), fields(entity = %entity_type.pascal_1))]
    pub(crate) async fn get_items_json_value(
        &self,
        entity_type: Arc<EntityType>,
        selections: Value,
        filter: Option<serde_json::Value>,
        user: Option<UserAuth>,
    ) -> Result<Vec<json_utils::JsonObj>, AppError> {
        debug!("getting items");
        let user = self
            .require_user(entity_type.clone(), AccessAction::Read, user, None)
            .await?;
        let access = self
            .authorize_entity_access(entity_type.clone(), AccessAction::Read, &user, None, None)
            .await?;

        let policy = PaginationPolicy::from_env();
        let plan = QueryPlan::new(
            self.app_config.clone(),
            entity_type.clone(),
            selections,
            filter,
            None,
            0,
            policy.max_page_size,
            &access,
        )?;

        let plan = plan.into_runtime_provider_plan();
        let provider = self.runtime_provider();
        let res_vec = runtime_data_access::execute_get_items_plan_read(
            &provider,
            plan,
            &user,
            &access,
            |operation, started_at, counts| {
                self.trace_provider_operation(entity_type.as_ref(), operation, started_at, counts)
            },
            |record| rules::evaluate(entity_type.clone(), record, AccessAction::Read, &user),
        )
        .await?;

        debug!(item_count = res_vec.len(), "got items");
        Ok(res_vec)
    }

    #[tracing::instrument(skip(self, entity_type, selections, filter, user), fields(entity = %entity_type.pascal_1))]
    pub async fn get_items<T>(
        &self,
        entity_type: Arc<EntityType>,
        selections: Value,
        filter: Option<serde_json::Value>,
        user: Option<UserAuth>,
    ) -> Result<Vec<T>, AppError>
    where
        T: std::marker::Send
            + std::marker::Sync
            + async_graphql::OutputType
            + for<'de> serde::Deserialize<'de>
            + std::fmt::Debug,
    {
        Ok(self
            .get_items_json_value(entity_type, selections, filter, user)
            .await?
            .into_iter()
            .map(json_utils::json_obj_to_t::<T>)
            .collect())
    }

    #[tracing::instrument(
    skip(self, entity_type, selections, filter, sort, user),
    fields(entity = %entity_type.pascal_1, skip = skip, limit = limit)
  )]
    pub(crate) async fn query_items_json_value(
        &self,
        entity_type: Arc<EntityType>,
        selections: Value,
        filter: Option<Value>,
        sort: Option<Value>,
        skip: i32,
        limit: i32,
        after: Option<String>,
        user: Option<UserAuth>,
    ) -> Result<JsonQueryResult, AppError> {
        debug!("querying items");
        let user = self
            .require_user(entity_type.clone(), AccessAction::Read, user, None)
            .await?;
        let access = self
            .authorize_entity_access(entity_type.clone(), AccessAction::Read, &user, None, None)
            .await?;

        let plan = if after.is_some() {
            QueryPlan::new_keyset(
                self.app_config.clone(),
                entity_type.clone(),
                selections,
                filter,
                sort,
                after,
                limit,
                &access,
            )?
        } else {
            QueryPlan::new(
                self.app_config.clone(),
                entity_type.clone(),
                selections,
                filter,
                sort,
                skip,
                limit,
                &access,
            )?
        };

        let plan = plan.into_runtime_provider_plan();
        let pagination = plan.pagination.clone();
        let runtime_sort =
            plan.sort
                .specs
                .first()
                .map(|spec| runtime_data_access::RuntimeReadSort {
                    field: spec.prop.name.clone(),
                    direction: spec.direction,
                });

        // println!("\n Database: query_items res:: {:?}", res);
        // println!("\n\n");

        let provider = self.runtime_provider();
        let res = runtime_data_access::execute_query_items_plan_read(
            &provider,
            plan,
            &pagination,
            runtime_sort.as_ref(),
            &user,
            &access,
            |operation, started_at, counts| {
                self.trace_provider_operation(entity_type.as_ref(), operation, started_at, counts)
            },
            |record| rules::evaluate(entity_type.clone(), record, AccessAction::Read, &user),
        )
        .await?;

        debug!(
            query_count = res.query_count,
            item_count = res.items.len(),
            "queried items"
        );
        Ok(res)
    }

    #[tracing::instrument(
    skip(self, entity_type, selections, filter, sort, user),
    fields(entity = %entity_type.pascal_1, skip = skip, limit = limit)
  )]
    pub async fn query_items<T>(
        &self,
        entity_type: Arc<EntityType>,
        selections: Value,
        filter: Option<Value>,
        sort: Option<Value>,
        skip: i32,
        limit: i32,
        after: Option<String>,
        user: Option<UserAuth>,
    ) -> Result<QueryResult<T>, AppError>
    where
        T: std::marker::Send
            + std::marker::Sync
            + async_graphql::OutputType
            + for<'de> serde::Deserialize<'de>
            + std::fmt::Debug,
    {
        let res = self
            .query_items_json_value(
                entity_type,
                selections,
                filter,
                sort,
                skip,
                limit,
                after,
                user,
            )
            .await?;

        Ok(QueryResult {
            date_time: res.date_time,
            request_duration: res.request_duration,
            skip: res.skip,
            limit: res.limit,
            page_count: res.page_count,
            page_index: res.page_index,
            query_count: res.query_count,
            next_cursor: res.next_cursor,
            previous_cursor: res.previous_cursor,
            items: res
                .items
                .into_iter()
                .map(json_utils::json_obj_to_t::<T>)
                .collect(),
        })
    }

    #[tracing::instrument(
        skip(self, entity_type, selections, ids, user),
        fields(entity = %entity_type.pascal_1, requested_count = ids.len())
    )]
    // Reserved for DataLoader-style relationship loading when a provider plan
    // cannot embed a relationship without creating an N+1 query pattern.
    #[allow(dead_code)]
    pub async fn batch_find_items_by_ids<T>(
        &self,
        entity_type: Arc<EntityType>,
        selections: Value,
        ids: Vec<Value>,
        user: Option<UserAuth>,
    ) -> Result<Vec<T>, AppError>
    where
        T: std::marker::Send
            + std::marker::Sync
            + async_graphql::OutputType
            + for<'de> serde::Deserialize<'de>
            + std::fmt::Debug,
    {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let limit = runtime_data_access::batch_limit_for_ids(ids.len()).map_err(AppError::from)?;

        let user = self
            .require_user(entity_type.clone(), AccessAction::Read, user, None)
            .await?;
        let access = self
            .authorize_entity_access(entity_type.clone(), AccessAction::Read, &user, None, None)
            .await?;
        let pk_name = self.app_config.get_primary_key_name(entity_type.clone())?;
        let filter =
            runtime_data_access::primary_key_in_filter(&pk_name, ids).map_err(AppError::from)?;
        let plan = QueryPlan::new(
            self.app_config.clone(),
            entity_type.clone(),
            selections,
            Some(filter),
            None,
            0,
            limit,
            &access,
        )?;

        let plan = plan.into_runtime_provider_plan();
        let provider = self.runtime_provider();
        let res_vec = runtime_data_access::execute_batch_get_items_plan_read(
            &provider,
            plan,
            &user,
            &access,
            |operation, started_at, counts| {
                self.trace_provider_operation(entity_type.as_ref(), operation, started_at, counts)
            },
            |record| rules::evaluate(entity_type.clone(), record, AccessAction::Read, &user),
        )
        .await?
        .into_iter()
        .map(json_utils::json_obj_to_t::<T>)
        .collect();
        Ok(res_vec)
    }

    #[tracing::instrument(
    skip(self, entity_type, filter, group_by, metrics, having, sort, user),
    fields(entity = %entity_type.pascal_1, skip = skip, limit = limit)
  )]
    pub(crate) async fn aggregate_items_json_value(
        &self,
        entity_type: Arc<EntityType>,
        filter: Option<Value>,
        group_by: Option<Value>,
        metrics: Option<Value>,
        having: Option<Value>,
        sort: Option<Value>,
        skip: i32,
        limit: i32,
        user: Option<UserAuth>,
    ) -> Result<JsonAggregateResult, AppError> {
        debug!("aggregating items");
        let user = self
            .require_user(entity_type.clone(), AccessAction::Read, user, None)
            .await?;
        let access = self
            .authorize_entity_access(entity_type.clone(), AccessAction::Read, &user, None, None)
            .await?;

        let plan = AggregatePlan::new(
            self.app_config.clone(),
            entity_type.clone(),
            filter,
            group_by,
            metrics,
            having,
            sort,
            skip,
            limit,
            &access,
        )?;

        let plan = plan.into_runtime_provider_plan();
        let provider = self.runtime_provider();
        let res = runtime_data_access::execute_aggregate_items_plan_read(
            &provider,
            plan,
            &user,
            &access,
            |operation, started_at, counts| {
                self.trace_provider_operation(entity_type.as_ref(), operation, started_at, counts)
            },
        )
        .await?;

        debug!(
            query_count = res.query_count,
            item_count = res.items.len(),
            "aggregated items"
        );

        Ok(res)
    }

    #[tracing::instrument(
    skip(self, entity_type, filter, group_by, metrics, having, sort, user),
    fields(entity = %entity_type.pascal_1, skip = skip, limit = limit)
  )]
    pub async fn aggregate_items(
        &self,
        entity_type: Arc<EntityType>,
        filter: Option<Value>,
        group_by: Option<Value>,
        metrics: Option<Value>,
        having: Option<Value>,
        sort: Option<Value>,
        skip: i32,
        limit: i32,
        user: Option<UserAuth>,
    ) -> Result<AggregateResult, AppError> {
        let res = self
            .aggregate_items_json_value(
                entity_type,
                filter,
                group_by,
                metrics,
                having,
                sort,
                skip,
                limit,
                user,
            )
            .await?;

        Ok(AggregateResult {
            date_time: res.date_time,
            request_duration: res.request_duration,
            skip: res.skip,
            limit: res.limit,
            page_count: res.page_count,
            page_index: res.page_index,
            query_count: res.query_count,
            items: res.items,
        })
    }

    #[cfg(feature = "mcp")]
    pub(crate) async fn append_mcp_tool_audit_event(
        &self,
        entity_type: Arc<EntityType>,
        user: &UserAuth,
        outcome: &str,
        metadata_json: Value,
    ) -> Result<(), AppError> {
        let entity = runtime_entity_metadata(&entity_type);
        let provider = self.runtime_provider();
        runtime_data_access::append_mcp_tool_audit_event(
            &entity,
            user,
            outcome,
            metadata_json,
            |event| runtime_data_access::provider_append_audit_event(&provider, event),
        )
        .await
    }

    async fn require_user(
        &self,
        entity_type: Arc<EntityType>,
        action: AccessAction,
        user: Option<UserAuth>,
        record_id: Option<String>,
    ) -> Result<UserAuth, AppError> {
        match user {
            Some(user) => Ok(user),
            None => {
                let err = AppError::NotAuthorized;
                self.append_missing_user_audit_attempt(entity_type, action, record_id, &err)
                    .await?;
                Err(err)
            }
        }
    }

    fn trace_provider_operation(
        &self,
        entity_type: &EntityType,
        operation: RuntimeProviderOperation,
        started_at: Instant,
        counts: RuntimeProviderOperationCounts,
    ) {
        let duration = started_at.elapsed();
        let duration_ms = duration.as_millis() as u64;
        let threshold_ms = slow_query_threshold_ms();
        let provider_descriptor = self.provider_descriptor();
        let provider = provider_descriptor.provider_key().to_string();
        let data_source = provider_descriptor.data_source_name().to_string();
        let operation = operation.as_str();
        let query_count = counts.query_count;
        let result_count = counts.result_count;
        let request_context = current_request_context();
        let request_id = request_context
            .as_ref()
            .map(|context| context.request_id.as_str())
            .unwrap_or("");
        let correlation_id = request_context
            .as_ref()
            .map(|context| context.correlation_id.as_str())
            .unwrap_or("");

        self.metrics.record_provider_operation(
            &provider,
            &data_source,
            &entity_type.pascal_1,
            operation,
            query_count,
            result_count,
            duration,
        );

        if duration_ms >= threshold_ms {
            tracing::warn!(
                provider = %provider,
                data_source = %data_source,
                entity = %entity_type.pascal_1,
                operation,
                duration_ms,
                query_count,
                result_count,
                request_id,
                correlation_id,
                "slow provider query"
            );
        } else {
            tracing::debug!(
                provider = %provider,
                data_source = %data_source,
                entity = %entity_type.pascal_1,
                operation,
                duration_ms,
                query_count,
                result_count,
                request_id,
                correlation_id,
                "provider query completed"
            );
        }
    }

    async fn authorize_entity_access(
        &self,
        entity_type: Arc<EntityType>,
        action: AccessAction,
        user: &UserAuth,
        record_id: Option<String>,
        attempted_json: Option<Value>,
    ) -> Result<PolicyAccess, AppError> {
        match self
            .app_config
            .evaluate_user_access(entity_type.clone(), action, user)
        {
            Ok(access) if access.allow => Ok(access),
            Ok(access) => {
                self.append_policy_denied_audit_attempt_on_record_chain(
                    entity_type,
                    action,
                    user,
                    record_id,
                    None,
                    attempted_json,
                    &access,
                )
                .await?;
                Err(AppError::AccessDenied)
            }
            Err(err) => {
                self.append_policy_error_audit_attempt(
                    entity_type,
                    action,
                    user,
                    record_id,
                    attempted_json,
                    &err,
                )
                .await?;
                Err(err)
            }
        }
    }

    async fn append_audit_mutation(
        &self,
        entity_type: Arc<EntityType>,
        action: AccessAction,
        user: &UserAuth,
        record_id: Option<String>,
        before_json: Option<Value>,
        after_json: Option<Value>,
        access: &PolicyAccess,
    ) -> Result<(), AppError> {
        let entity = runtime_entity_metadata(&entity_type);
        let provider = self.runtime_provider();
        runtime_data_access::append_audit_mutation(
            &entity,
            action,
            user,
            record_id,
            before_json,
            after_json,
            access,
            |event| runtime_data_access::provider_append_audit_event(&provider, event),
        )
        .await
    }

    async fn append_missing_user_audit_attempt(
        &self,
        entity_type: Arc<EntityType>,
        action: AccessAction,
        record_id: Option<String>,
        error: &AppError,
    ) -> Result<(), AppError> {
        let entity = runtime_entity_metadata(&entity_type);
        let provider = self.runtime_provider();
        runtime_data_access::append_missing_user_audit_attempt(
            &entity,
            action,
            record_id,
            error,
            |event| runtime_data_access::provider_append_audit_event(&provider, event),
        )
        .await
    }

    async fn append_policy_denied_audit_attempt_on_record_chain(
        &self,
        entity_type: Arc<EntityType>,
        action: AccessAction,
        user: &UserAuth,
        record_id: Option<String>,
        before_json: Option<Value>,
        attempted_json: Option<Value>,
        access: &PolicyAccess,
    ) -> Result<(), AppError> {
        let entity = runtime_entity_metadata(&entity_type);
        let provider = self.runtime_provider();
        runtime_data_access::append_policy_denied_audit_attempt_on_record_chain(
            &entity,
            action,
            user,
            record_id,
            before_json,
            attempted_json,
            access,
            |event| runtime_data_access::provider_append_audit_event(&provider, event),
        )
        .await
    }

    async fn append_policy_error_audit_attempt(
        &self,
        entity_type: Arc<EntityType>,
        action: AccessAction,
        user: &UserAuth,
        record_id: Option<String>,
        attempted_json: Option<Value>,
        error: &AppError,
    ) -> Result<(), AppError> {
        let entity = runtime_entity_metadata(&entity_type);
        let provider = self.runtime_provider();
        runtime_data_access::append_policy_error_audit_attempt(
            &entity,
            action,
            user,
            record_id,
            attempted_json,
            error,
            |event| runtime_data_access::provider_append_audit_event(&provider, event),
        )
        .await
    }

    async fn append_operation_failed_audit_attempt(
        &self,
        entity_type: Arc<EntityType>,
        action: AccessAction,
        user: &UserAuth,
        record_id: Option<String>,
        before_json: Option<Value>,
        after_json: Option<Value>,
        error: &AppError,
    ) -> Result<(), AppError> {
        let entity = runtime_entity_metadata(&entity_type);
        let provider = self.runtime_provider();
        runtime_data_access::append_operation_failed_audit_attempt(
            &entity,
            action,
            user,
            record_id,
            before_json,
            after_json,
            error,
            |event| runtime_data_access::provider_append_audit_event(&provider, event),
        )
        .await
    }

    async fn append_operation_not_applied_audit_attempt(
        &self,
        entity_type: Arc<EntityType>,
        action: AccessAction,
        user: &UserAuth,
        record_id: Option<String>,
        before_json: Option<Value>,
    ) -> Result<(), AppError> {
        let entity = runtime_entity_metadata(&entity_type);
        let provider = self.runtime_provider();
        runtime_data_access::append_operation_not_applied_audit_attempt(
            &entity,
            action,
            user,
            record_id,
            before_json,
            |event| runtime_data_access::provider_append_audit_event(&provider, event),
        )
        .await
    }

    async fn audit_before(
        &self,
        entity_type: Arc<EntityType>,
        input: &json_utils::JsonObj,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<Option<json_utils::JsonObj>, AppError> {
        self.record_before_with_access(entity_type, input, user, access)
            .await
    }

    async fn record_before_with_access(
        &self,
        entity_type: Arc<EntityType>,
        input: &json_utils::JsonObj,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<Option<json_utils::JsonObj>, AppError> {
        let pk_name = self.app_config.get_primary_key_name(entity_type.clone())?;
        let Some(id) = input.get(&pk_name).and_then(runtime_audit::value_to_string) else {
            return Ok(None);
        };
        let provider = self.runtime_provider();
        runtime_data_access::provider_find_item_json(
            &provider,
            entity_type.clone(),
            runtime_audit::audit_selection(&runtime_entity_metadata(&entity_type)),
            id,
            user,
            access,
        )
        .await
    }

    async fn validate_uniqueness(
        &self,
        entity_type: Arc<EntityType>,
        record: &json_utils::JsonObj,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<(), AppError> {
        let pk_name = self.app_config.get_primary_key_name(entity_type.clone())?;

        for prop in &entity_type.props {
            for validator in runtime_validation::property_validators(&prop.name, prop.meta.as_ref())
                .map_err(AppError::from)?
            {
                if !runtime_validation::is_validator_named(validator, "Uniqueness") {
                    continue;
                }

                let filter = validator
                    .get("filter")
                    .and_then(Value::as_str)
                    .ok_or_else(|| invalid_uniqueness(prop, "missing filter"))?;
                let filter = runtime_validation::render_uniqueness_filter(filter, record)
                    .map_err(AppError::from)?;
                let selections = serde_json::json!({
                    "name": entity_type.snake_n,
                    "selection_set": [
                        { "name": pk_name, "selection_set": [] }
                    ]
                });
                let plan = QueryPlan::new(
                    self.app_config.clone(),
                    entity_type.clone(),
                    selections,
                    Some(filter),
                    None,
                    0,
                    2,
                    access,
                )?;
                let plan = plan.into_runtime_provider_plan();
                let provider = self.runtime_provider();
                runtime_data_access::validate_unique_record(
                    &provider,
                    plan,
                    user,
                    access,
                    &pk_name,
                    record,
                    &entity_type.pascal_1,
                    &prop.name,
                    &runtime_validation::validator_message(validator, "must be unique"),
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn validate_primary_key_available(
        &self,
        entity_type: Arc<EntityType>,
        record: &json_utils::JsonObj,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<(), AppError> {
        let pk_name = self.app_config.get_primary_key_name(entity_type.clone())?;
        let Some(id) = record
            .get(&pk_name)
            .and_then(runtime_audit::value_to_string)
        else {
            return Ok(());
        };
        let provider = self.runtime_provider();
        runtime_data_access::validate_primary_key_available(
            &provider,
            entity_type.clone(),
            runtime_audit::audit_selection(&runtime_entity_metadata(&entity_type)),
            Some(id),
            user,
            access,
        )
        .await
    }

    async fn validate_foreign_keys(
        &self,
        entity_type: Arc<EntityType>,
        record: &json_utils::JsonObj,
        user: &UserAuth,
    ) -> Result<(), AppError> {
        for prop in &entity_type.props {
            let Some(foreign_key) = &prop.foreign_key else {
                continue;
            };
            let Some(id) = record
                .get(&prop.name)
                .and_then(runtime_audit::value_to_string)
            else {
                continue;
            };
            let target_schema = if foreign_key.schema_name.trim().is_empty() {
                entity_type.schema_name.clone()
            } else {
                foreign_key.schema_name.clone()
            };
            let target = self
                .app_config
                .get_entity_type(&target_schema, &foreign_key.type_name)?;
            let target_access = self.app_config.evaluate_user_access(
                target.clone(),
                AccessAction::Read,
                user,
            )?;
            let provider = self.runtime_provider();
            runtime_data_access::validate_foreign_key_exists(
                &provider,
                target.clone(),
                runtime_audit::audit_selection(&runtime_entity_metadata(&target)),
                Some(id),
                user,
                &target_access,
            )
            .await?;
        }
        Ok(())
    }
}

fn invalid_uniqueness(prop: &PropertyType, message: &str) -> AppError {
    AppError::Validation(format!(
        "{} uniqueness validator is invalid: {}",
        prop.name, message
    ))
}

fn slow_query_threshold_ms() -> u64 {
    env::var("APP_SLOW_QUERY_THRESHOLD_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(500)
}

// Relationships:

// EntityType
// - name: Resource
//   schema_name: scheduling
//   props:
//
//   - name: id
//     fragment: property-primary-key-uuid
//
//   - name: state_id
//     data_type: Uuid
//     foreign_key:
//       schema_name: scheduling
//       type_name: State
//
//   - name: state
//     data_type: NavToOne
//     nav_by_fk_property:
//       schema_name: scheduling
//       type_name: Resource
//       prop_name: state_id
//       resolved:
//         schema_name: scheduling
//         type_name: State
//
//   - name: primary_resource_office
//     data_type: NavToOne
//     nav_by_fk_property:
//       schema_name: scheduling
//       type_name: ResourceOffice
//       prop_name: resource_id
//       filter: { is_primary: true }
//       resolved:
//         schema_name: scheduling
//         type_name: Resource

// EntityType
// - name: ResourceOffice
//   schema_name: scheduling
//   props:
//
//   - name: id
//     fragment: property-primary-key-uuid
//
//   - name: is_primary
//     fragment: property-boolean-toggle
//
//   - name: resource_id
//     data_type: Uuid
//     foreign_key:
//       schema_name: scheduling
//       type_name: Resource
//

// - name: State
//   schema_name: scheduling
//   props:
//   - name: id
//     fragment: property-primary-key-uuid
//   - name: label
//     fragment: property-string-caption

// Selections:

// selections_json: Object {
//   "name": String("queryUsers"),
//   "selection_set": Array [
//     Object {
//       "name": String("dateTime"),
//       "selection_set": Array []
//     },
//     Object {
//       "name": String("requestDuration"),
//       "selection_set": Array []
//     },
//     Object {
//       "name": String("skip"),
//       "selection_set": Array []
//     },
//     Object {
//       "name": String("limit"),
//       "selection_set": Array []
//     },
//     Object {
//       "name": String("pageIndex"),
//       "selection_set": Array []
//     },
//     Object {
//       "name": String("pageCount"),
//       "selection_set": Array []
//     },
//     Object {
//       "name": String("queryCount"),
//       "selection_set": Array []
//     },
//     Object {
//       "name": String("items"),
//       "selection_set": Array [
//         Object {
//           "name": String("id"),
//           "selection_set": Array []
//         },
//         Object {
//           "name": String("userName"),
//           "selection_set": Array []
//         },
//         Object {
//           "name": String("email"),
//           "selection_set": Array []
//         },
//         Object {
//           "name": String("mobile"),
//           "selection_set": Array []
//         }
//       ]
//     }
//   ]
// }

// audit_recs AS
// (
//   select a.*
//   from sys.users_audit a
//   order by a.id
// ),
// root_cte AS
// (
//   select {select_fields},
//         json_agg(audit_recs) as audit_recs
//   from {schema_name}.{table} {root_alias}
//         left join audit_recs on audit_recs.record_id = t0.id
//   {filter}
//   group by {root_alias}.id
//   {order_by}
// )
