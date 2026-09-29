#![allow(dead_code)]

use std::sync::Arc;

use anyhow::Result;
pub use appfw_runtime::ProviderPoolStats;
use appfw_runtime::{
    extension::UserAuth, json::JsonObj, model_metadata::RuntimeDataType, PolicyAccess,
    RuntimeAuditEvent, RuntimeAuditQuery, RuntimeJsonAggregateResult, RuntimeJsonObj,
    RuntimeJsonQueryResult, RuntimeProviderAggregatePlan, RuntimeProviderClient,
    RuntimeProviderDataClient, RuntimeProviderIdentity, RuntimeProviderMutationPlan,
    RuntimeProviderPlanInput, RuntimeProviderQueryPlan,
};
use async_trait::async_trait;
use serde_json::Value;

use crate::{
    data::query_ir::{
        AccessFilterAst, AggregateHavingAst, AggregateMetric, AggregateSortAst, FilterAst,
        GroupBySpec, SelectionTree, SortAst,
    },
    routes::app_error::AppError,
    schemas::system::EntityType,
};

pub type DatabaseClientBox = Box<dyn DatabaseClient + Send + Sync>;
pub type JsonQueryResult = RuntimeJsonQueryResult;
pub type JsonAggregateResult = RuntimeJsonAggregateResult;
pub type ProviderMutationPlan = RuntimeProviderMutationPlan<Arc<EntityType>>;
pub type ProviderQueryPlan =
    RuntimeProviderQueryPlan<Arc<EntityType>, SelectionTree, FilterAst, SortAst, AccessFilterAst>;
pub type ProviderAggregatePlan = RuntimeProviderAggregatePlan<
    Arc<EntityType>,
    FilterAst,
    AccessFilterAst,
    GroupBySpec,
    AggregateMetric,
    AggregateHavingAst,
    AggregateSortAst,
>;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderRoutineKind {
    Function,
    Procedure,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderRoutineReturns {
    None,
    One,
    Many,
}

#[derive(Clone, Debug)]
pub struct ProviderRoutineArgument {
    pub name: &'static str,
    pub data_type: RuntimeDataType,
    pub required: bool,
    pub value: Value,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub struct ProviderRoutineName {
    pub schema: Option<&'static str>,
    pub name: &'static str,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct ProviderRoutineCall {
    pub method_name: &'static str,
    pub kind: ProviderRoutineKind,
    pub returns: ProviderRoutineReturns,
    pub data_source_name: Option<&'static str>,
    pub postgres: Option<ProviderRoutineName>,
    pub mssql: Option<ProviderRoutineName>,
    pub snowflake: Option<ProviderRoutineName>,
}

pub struct DatabaseClientRuntimeAdapter<'a> {
    client: &'a (dyn DatabaseClient + Send + Sync),
}

impl<'a> DatabaseClientRuntimeAdapter<'a> {
    pub fn new(client: &'a (dyn DatabaseClient + Send + Sync)) -> Self {
        Self { client }
    }
}

impl dyn DatabaseClient + Send + Sync {
    pub fn as_runtime_provider(&self) -> DatabaseClientRuntimeAdapter<'_> {
        DatabaseClientRuntimeAdapter::new(self)
    }
}

impl RuntimeProviderIdentity for DatabaseClientRuntimeAdapter<'_> {
    fn data_source_name(&self) -> &str {
        self.client.data_source_name()
    }

    fn framework_provider(&self) -> appfw_runtime::provider_keys::FrameworkProvider {
        self.client.framework_provider()
    }

    fn pool_stats(&self) -> ProviderPoolStats {
        self.client.pool_stats()
    }
}

#[async_trait]
impl RuntimeProviderDataClient for DatabaseClientRuntimeAdapter<'_> {
    type Error = AppError;
    type Entity = Arc<EntityType>;
    type QueryPlan = ProviderQueryPlan;
    type AggregatePlan = ProviderAggregatePlan;
    type MutationPlan = ProviderMutationPlan;

    async fn health_check(&self) -> Result<(), Self::Error> {
        self.client.health_check().await
    }

    async fn create_item_plan_json(
        &self,
        input: RuntimeProviderPlanInput<'_, Self::MutationPlan>,
    ) -> Result<RuntimeJsonObj, Self::Error> {
        self.client.create_item_plan_json(input).await
    }

    async fn update_item_plan_json(
        &self,
        input: RuntimeProviderPlanInput<'_, Self::MutationPlan>,
    ) -> Result<RuntimeJsonObj, Self::Error> {
        self.client.update_item_plan_json(input).await
    }

    async fn delete_item_plan_json(
        &self,
        input: RuntimeProviderPlanInput<'_, Self::MutationPlan>,
    ) -> Result<i64, Self::Error> {
        self.client.delete_item_plan_json(input).await
    }

    async fn get_items_plan_json(
        &self,
        input: RuntimeProviderPlanInput<'_, Self::QueryPlan>,
    ) -> Result<Vec<RuntimeJsonObj>, Self::Error> {
        self.client.get_items_plan_json(input).await
    }

    async fn query_items_plan_json(
        &self,
        input: RuntimeProviderPlanInput<'_, Self::QueryPlan>,
    ) -> Result<JsonQueryResult, Self::Error> {
        self.client.query_items_plan_json(input).await
    }

    async fn batch_get_items_plan_json(
        &self,
        input: RuntimeProviderPlanInput<'_, Self::QueryPlan>,
    ) -> Result<Vec<RuntimeJsonObj>, Self::Error> {
        self.client.batch_get_items_plan_json(input).await
    }

    async fn aggregate_items_plan_json(
        &self,
        input: RuntimeProviderPlanInput<'_, Self::AggregatePlan>,
    ) -> Result<JsonAggregateResult, Self::Error> {
        self.client.aggregate_items_plan_json(input).await
    }

    async fn explain_query_plan(
        &self,
        input: RuntimeProviderPlanInput<'_, &Self::QueryPlan>,
    ) -> Result<Value, Self::Error> {
        self.client.explain_query_plan(input).await
    }

    async fn append_audit_event(&self, event: RuntimeAuditEvent) -> Result<(), Self::Error> {
        self.client.append_audit_event(event).await
    }

    async fn query_audit_events(
        &self,
        query: RuntimeAuditQuery,
    ) -> Result<Vec<Value>, Self::Error> {
        self.client.query_audit_events(query).await
    }

    async fn create_item_json(
        &self,
        entity_type: Self::Entity,
        selections: Value,
        input: RuntimeJsonObj,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<RuntimeJsonObj, Self::Error> {
        self.client
            .create_item_json(entity_type, selections, input, user, access)
            .await
    }

    async fn update_item_json(
        &self,
        entity_type: Self::Entity,
        selections: Value,
        input: RuntimeJsonObj,
        user: &UserAuth,
        access: &PolicyAccess,
        read_version: Option<Value>,
    ) -> Result<RuntimeJsonObj, Self::Error> {
        self.client
            .update_item_json(entity_type, selections, input, user, access, read_version)
            .await
    }

    async fn delete_item_json(
        &self,
        entity_type: Self::Entity,
        input: RuntimeJsonObj,
        user: &UserAuth,
        access: &PolicyAccess,
        read_version: Option<Value>,
    ) -> Result<i64, Self::Error> {
        self.client
            .delete_item_json(entity_type, input, user, access, read_version)
            .await
    }

    async fn find_item_json(
        &self,
        entity_type: Self::Entity,
        selections: Value,
        id: String,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<Option<RuntimeJsonObj>, Self::Error> {
        self.client
            .find_item_json(entity_type, selections, id, user, access)
            .await
    }

    async fn get_items_json(
        &self,
        entity_type: Self::Entity,
        selections: Value,
        filter: Option<Value>,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<Vec<RuntimeJsonObj>, Self::Error> {
        self.client
            .get_items_json(entity_type, selections, filter, user, access)
            .await
    }

    async fn query_items_json(
        &self,
        entity_type: Self::Entity,
        selections: Value,
        filter: Option<Value>,
        sort: Option<Value>,
        skip: i32,
        limit: i32,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<JsonQueryResult, Self::Error> {
        self.client
            .query_items_json(
                entity_type,
                selections,
                filter,
                sort,
                skip,
                limit,
                user,
                access,
            )
            .await
    }
}

#[async_trait]
pub trait DatabaseClient: RuntimeProviderClient {
    async fn health_check(&self) -> Result<(), AppError>;

    /// Primary provider contract for create mutations. DataAccess builds a
    /// typed MutationPlan once; providers should compile the plan instead of
    /// reinterpreting GraphQL JSON directly.
    async fn create_item_plan_json(
        &self,
        input: RuntimeProviderPlanInput<'_, ProviderMutationPlan>,
    ) -> Result<JsonObj, AppError> {
        let (plan, user, access) = input.into_parts();
        let mut plan_access = access.clone();
        plan_access.filter = plan.access_filter_json();
        self.create_item_json(
            plan.entity.clone(),
            plan.selection_json(),
            plan.input,
            user,
            &plan_access,
        )
        .await
    }

    /// Primary provider contract for update mutations.
    async fn update_item_plan_json(
        &self,
        input: RuntimeProviderPlanInput<'_, ProviderMutationPlan>,
    ) -> Result<JsonObj, AppError> {
        let (plan, user, access) = input.into_parts();
        let read_version = plan.read_version();
        let mut plan_access = access.clone();
        plan_access.filter = plan.access_filter_json();
        self.update_item_json(
            plan.entity.clone(),
            plan.selection_json(),
            plan.input,
            user,
            &plan_access,
            read_version,
        )
        .await
    }

    /// Primary provider contract for delete mutations.
    async fn delete_item_plan_json(
        &self,
        input: RuntimeProviderPlanInput<'_, ProviderMutationPlan>,
    ) -> Result<i64, AppError> {
        let (plan, user, access) = input.into_parts();
        let read_version = plan.read_version();
        let mut plan_access = access.clone();
        plan_access.filter = plan.access_filter_json();
        self.delete_item_json(
            plan.entity.clone(),
            plan.input,
            user,
            &plan_access,
            read_version,
        )
        .await
    }

    async fn append_audit_event(&self, event: RuntimeAuditEvent) -> Result<(), AppError> {
        Err(AppError::DataAccess(format!(
            "audit append is not implemented for provider backing {}.{}",
            event.schema_name, event.entity_name
        )))
    }

    async fn query_audit_events(&self, query: RuntimeAuditQuery) -> Result<Vec<Value>, AppError> {
        Err(AppError::DataAccess(format!(
            "audit timeline query is not implemented for provider backing {}.{}",
            query.schema_name, query.entity_name
        )))
    }

    #[allow(dead_code)]
    async fn call_provider_routine_json(
        &self,
        routine: &ProviderRoutineCall,
        _arguments: &[ProviderRoutineArgument],
        _user: &UserAuth,
        _access: &PolicyAccess,
    ) -> Result<Value, AppError> {
        Err(AppError::DataAccess(format!(
            "provider routine `{}` is not implemented for data source `{}`",
            routine.method_name,
            self.data_source_name()
        )))
    }

    /// Primary provider contract for collection reads without explicit paging.
    async fn get_items_plan_json(
        &self,
        input: RuntimeProviderPlanInput<'_, ProviderQueryPlan>,
    ) -> Result<Vec<JsonObj>, AppError> {
        let (plan, user, access) = input.into_parts();
        let mut plan_access = access.clone();
        plan_access.filter = plan.access_filter_json();
        self.get_items_json(
            plan.entity_type.clone(),
            plan.selection_json(),
            plan.filter_json(),
            user,
            &plan_access,
        )
        .await
    }

    /// Primary provider contract for paged collection reads. Providers must
    /// compile QueryPlan directly so access filters, pagination semantics, and
    /// relationship intent are validated once at the DataAccess boundary.
    async fn query_items_plan_json(
        &self,
        input: RuntimeProviderPlanInput<'_, ProviderQueryPlan>,
    ) -> Result<JsonQueryResult, AppError>;

    /// Batch-load records in one provider query. This gives custom resolvers and
    /// generated fallback paths a DataLoader-style primitive when the provider
    /// cannot embed a relationship in the original plan.
    ///
    /// Reserved for DataLoader-style relationship loading when a provider plan
    /// cannot avoid an N+1 fallback path.
    #[allow(dead_code)]
    async fn batch_get_items_plan_json(
        &self,
        input: RuntimeProviderPlanInput<'_, ProviderQueryPlan>,
    ) -> Result<Vec<JsonObj>, AppError> {
        Ok(self.query_items_plan_json(input).await?.items)
    }

    /// Primary provider contract for aggregate reads. DataAccess builds the
    /// AggregatePlan once, including user and access filters; providers compile
    /// it into their native aggregate/query dialect.
    async fn aggregate_items_plan_json(
        &self,
        _input: RuntimeProviderPlanInput<'_, ProviderAggregatePlan>,
    ) -> Result<JsonAggregateResult, AppError> {
        Err(AppError::DataAccess(
            "aggregate queries are not implemented for this provider".to_string(),
        ))
    }

    /// Optional provider-native query diagnostic hook.
    ///
    /// Providers should only override this when they can return a safe,
    /// redacted diagnostic payload. The default deliberately avoids raw SQL,
    /// bind values, credentials, and tenant-sensitive filter values.
    async fn explain_query_plan(
        &self,
        _input: RuntimeProviderPlanInput<'_, &ProviderQueryPlan>,
    ) -> Result<Value, AppError> {
        Ok(self.provider_descriptor().unsupported_explain_diagnostic())
    }

    async fn create_item_json(
        &self,
        entity_type: Arc<EntityType>,
        selections: serde_json::Value,
        input: JsonObj,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<JsonObj, AppError>;

    async fn update_item_json(
        &self,
        entity_type: Arc<EntityType>,
        selections: serde_json::Value,
        input: JsonObj,
        user: &UserAuth,
        access: &PolicyAccess,
        read_version: Option<Value>,
    ) -> Result<JsonObj, AppError>;

    async fn delete_item_json(
        &self,
        entity_type: Arc<EntityType>,
        input: JsonObj,
        user: &UserAuth,
        access: &PolicyAccess,
        read_version: Option<Value>,
    ) -> Result<i64, AppError>;

    async fn find_item_json(
        &self,
        entity_type: Arc<EntityType>,
        selections: serde_json::Value,
        id: String,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<Option<JsonObj>, AppError>;

    async fn get_items_json(
        &self,
        entity_type: Arc<EntityType>,
        selections: serde_json::Value,
        filter: Option<serde_json::Value>,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<Vec<JsonObj>, AppError>;

    /// Legacy compatibility bridge for older custom code. New framework paths
    /// should build QueryPlan and call query_items_plan_json.
    async fn query_items_json(
        &self,
        entity_type: Arc<EntityType>,
        selections: serde_json::Value,
        filter: Option<Value>,
        sort: Option<Value>,
        skip: i32,
        limit: i32,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<JsonQueryResult, AppError>;
}
