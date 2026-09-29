use async_trait::async_trait;
use serde::Serialize;
use serde_json::{json, Value};

use crate::{
    extension::UserAuth,
    provider_keys::FrameworkProvider,
    provider_request::RuntimeProviderPlanInput,
    provider_result::{RuntimeJsonAggregateResult, RuntimeJsonObj, RuntimeJsonQueryResult},
    record_audit::{RuntimeAuditEvent, RuntimeAuditQuery},
    PolicyAccess, ProviderPoolStats,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeProviderDescriptor {
    pub provider: FrameworkProvider,
    pub data_source_name: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum RuntimeProviderOperation {
    HealthCheck,
    ExplainQueryPlan,
    CreateItem,
    UpdateItem,
    DeleteItem,
    FindItem,
    GetItems,
    QueryItems,
    BatchFindItemsByIds,
    AggregateItems,
    AppendAuditEvent,
    QueryAuditEvents,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeProviderOperationCounts {
    pub query_count: u64,
    pub result_count: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum RuntimeProviderOperationRequirement {
    Required,
    Optional,
    LegacyCompatibility,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum RuntimeProviderOperationSurface {
    RuntimeNative,
    ProductPlanAdapter,
    ProductLegacyAdapter,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeProviderOperationContract {
    pub operation: RuntimeProviderOperation,
    pub requirement: RuntimeProviderOperationRequirement,
    pub surface: RuntimeProviderOperationSurface,
}

pub trait RuntimeProviderIdentity {
    fn data_source_name(&self) -> &str;

    fn framework_provider(&self) -> FrameworkProvider;

    fn provider_operation_contracts(&self) -> &'static [RuntimeProviderOperationContract] {
        RuntimeProviderOperationContract::DATABASE_CLIENT
    }

    fn provider_declares_operation(&self, operation: RuntimeProviderOperation) -> bool {
        self.provider_operation_contracts()
            .iter()
            .any(|contract| contract.operation == operation)
    }

    fn provider_descriptor(&self) -> RuntimeProviderDescriptor {
        RuntimeProviderDescriptor::new(self.framework_provider(), self.data_source_name())
    }

    fn pool_stats(&self) -> ProviderPoolStats {
        self.provider_descriptor().opaque_pool_stats()
    }
}

pub trait RuntimeProviderClient: Send + Sync + RuntimeProviderIdentity {}

impl<T> RuntimeProviderClient for T where T: Send + Sync + RuntimeProviderIdentity {}

#[async_trait]
pub trait RuntimeProviderDataClient: RuntimeProviderClient {
    type Error: Send + Sync + 'static;
    type Entity: Clone + Send + Sync + 'static;
    type QueryPlan: Send + Sync + 'static;
    type AggregatePlan: Send + Sync + 'static;
    type MutationPlan: Send + Sync + 'static;

    async fn health_check(&self) -> Result<(), Self::Error>;

    async fn create_item_plan_json(
        &self,
        input: RuntimeProviderPlanInput<'_, Self::MutationPlan>,
    ) -> Result<RuntimeJsonObj, Self::Error>;

    async fn update_item_plan_json(
        &self,
        input: RuntimeProviderPlanInput<'_, Self::MutationPlan>,
    ) -> Result<RuntimeJsonObj, Self::Error>;

    async fn delete_item_plan_json(
        &self,
        input: RuntimeProviderPlanInput<'_, Self::MutationPlan>,
    ) -> Result<i64, Self::Error>;

    async fn get_items_plan_json(
        &self,
        input: RuntimeProviderPlanInput<'_, Self::QueryPlan>,
    ) -> Result<Vec<RuntimeJsonObj>, Self::Error>;

    async fn query_items_plan_json(
        &self,
        input: RuntimeProviderPlanInput<'_, Self::QueryPlan>,
    ) -> Result<RuntimeJsonQueryResult, Self::Error>;

    async fn batch_get_items_plan_json(
        &self,
        input: RuntimeProviderPlanInput<'_, Self::QueryPlan>,
    ) -> Result<Vec<RuntimeJsonObj>, Self::Error>;

    async fn aggregate_items_plan_json(
        &self,
        input: RuntimeProviderPlanInput<'_, Self::AggregatePlan>,
    ) -> Result<RuntimeJsonAggregateResult, Self::Error>;

    async fn explain_query_plan(
        &self,
        input: RuntimeProviderPlanInput<'_, &Self::QueryPlan>,
    ) -> Result<Value, Self::Error>;

    async fn append_audit_event(&self, event: RuntimeAuditEvent) -> Result<(), Self::Error>;

    async fn query_audit_events(&self, query: RuntimeAuditQuery)
        -> Result<Vec<Value>, Self::Error>;

    async fn create_item_json(
        &self,
        entity_type: Self::Entity,
        selections: Value,
        input: RuntimeJsonObj,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<RuntimeJsonObj, Self::Error>;

    async fn update_item_json(
        &self,
        entity_type: Self::Entity,
        selections: Value,
        input: RuntimeJsonObj,
        user: &UserAuth,
        access: &PolicyAccess,
        read_version: Option<Value>,
    ) -> Result<RuntimeJsonObj, Self::Error>;

    async fn delete_item_json(
        &self,
        entity_type: Self::Entity,
        input: RuntimeJsonObj,
        user: &UserAuth,
        access: &PolicyAccess,
        read_version: Option<Value>,
    ) -> Result<i64, Self::Error>;

    async fn find_item_json(
        &self,
        entity_type: Self::Entity,
        selections: Value,
        id: String,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<Option<RuntimeJsonObj>, Self::Error>;

    async fn get_items_json(
        &self,
        entity_type: Self::Entity,
        selections: Value,
        filter: Option<Value>,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<Vec<RuntimeJsonObj>, Self::Error>;

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
    ) -> Result<RuntimeJsonQueryResult, Self::Error>;
}

impl RuntimeProviderDescriptor {
    pub fn new(provider: FrameworkProvider, data_source_name: impl Into<String>) -> Self {
        Self {
            provider,
            data_source_name: data_source_name.into(),
        }
    }

    pub fn provider_key(&self) -> &'static str {
        self.provider.key()
    }

    pub fn data_source_name(&self) -> &str {
        &self.data_source_name
    }

    pub fn opaque_pool_stats(&self) -> ProviderPoolStats {
        ProviderPoolStats::opaque(self.provider_key(), self.data_source_name())
    }

    pub fn unsupported_explain_diagnostic(&self) -> Value {
        json!({
            "status": "unsupported",
            "message": "safe provider EXPLAIN diagnostics are not implemented for this provider",
            "provider": self.provider_key(),
            "data_source": self.data_source_name(),
        })
    }
}

impl RuntimeProviderOperationRequirement {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Required => "required",
            Self::Optional => "optional",
            Self::LegacyCompatibility => "legacy_compatibility",
        }
    }
}

impl RuntimeProviderOperationSurface {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RuntimeNative => "runtime_native",
            Self::ProductPlanAdapter => "product_plan_adapter",
            Self::ProductLegacyAdapter => "product_legacy_adapter",
        }
    }
}

impl RuntimeProviderOperation {
    pub const ALL: [RuntimeProviderOperation; 12] = [
        RuntimeProviderOperation::HealthCheck,
        RuntimeProviderOperation::ExplainQueryPlan,
        RuntimeProviderOperation::CreateItem,
        RuntimeProviderOperation::UpdateItem,
        RuntimeProviderOperation::DeleteItem,
        RuntimeProviderOperation::FindItem,
        RuntimeProviderOperation::GetItems,
        RuntimeProviderOperation::QueryItems,
        RuntimeProviderOperation::BatchFindItemsByIds,
        RuntimeProviderOperation::AggregateItems,
        RuntimeProviderOperation::AppendAuditEvent,
        RuntimeProviderOperation::QueryAuditEvents,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            RuntimeProviderOperation::HealthCheck => "health_check",
            RuntimeProviderOperation::ExplainQueryPlan => "explain_query_plan",
            RuntimeProviderOperation::CreateItem => "create_item",
            RuntimeProviderOperation::UpdateItem => "update_item",
            RuntimeProviderOperation::DeleteItem => "delete_item",
            RuntimeProviderOperation::FindItem => "find_item",
            RuntimeProviderOperation::GetItems => "get_items",
            RuntimeProviderOperation::QueryItems => "query_items",
            RuntimeProviderOperation::BatchFindItemsByIds => "batch_find_items_by_ids",
            RuntimeProviderOperation::AggregateItems => "aggregate_items",
            RuntimeProviderOperation::AppendAuditEvent => "append_audit_event",
            RuntimeProviderOperation::QueryAuditEvents => "query_audit_events",
        }
    }
}

impl RuntimeProviderOperationContract {
    pub const DATABASE_CLIENT: &'static [RuntimeProviderOperationContract] = &[
        Self::required(
            RuntimeProviderOperation::HealthCheck,
            RuntimeProviderOperationSurface::RuntimeNative,
        ),
        Self::optional(
            RuntimeProviderOperation::ExplainQueryPlan,
            RuntimeProviderOperationSurface::ProductPlanAdapter,
        ),
        Self::required(
            RuntimeProviderOperation::CreateItem,
            RuntimeProviderOperationSurface::ProductPlanAdapter,
        ),
        Self::required(
            RuntimeProviderOperation::UpdateItem,
            RuntimeProviderOperationSurface::ProductPlanAdapter,
        ),
        Self::required(
            RuntimeProviderOperation::DeleteItem,
            RuntimeProviderOperationSurface::ProductPlanAdapter,
        ),
        Self::required(
            RuntimeProviderOperation::FindItem,
            RuntimeProviderOperationSurface::ProductLegacyAdapter,
        ),
        Self::required(
            RuntimeProviderOperation::GetItems,
            RuntimeProviderOperationSurface::ProductPlanAdapter,
        ),
        Self::required(
            RuntimeProviderOperation::QueryItems,
            RuntimeProviderOperationSurface::ProductPlanAdapter,
        ),
        Self::optional(
            RuntimeProviderOperation::BatchFindItemsByIds,
            RuntimeProviderOperationSurface::ProductPlanAdapter,
        ),
        Self::optional(
            RuntimeProviderOperation::AggregateItems,
            RuntimeProviderOperationSurface::ProductPlanAdapter,
        ),
        Self::optional(
            RuntimeProviderOperation::AppendAuditEvent,
            RuntimeProviderOperationSurface::RuntimeNative,
        ),
        Self::optional(
            RuntimeProviderOperation::QueryAuditEvents,
            RuntimeProviderOperationSurface::RuntimeNative,
        ),
    ];

    pub const fn required(
        operation: RuntimeProviderOperation,
        surface: RuntimeProviderOperationSurface,
    ) -> Self {
        Self {
            operation,
            requirement: RuntimeProviderOperationRequirement::Required,
            surface,
        }
    }

    pub const fn optional(
        operation: RuntimeProviderOperation,
        surface: RuntimeProviderOperationSurface,
    ) -> Self {
        Self {
            operation,
            requirement: RuntimeProviderOperationRequirement::Optional,
            surface,
        }
    }

    pub const fn legacy(
        operation: RuntimeProviderOperation,
        surface: RuntimeProviderOperationSurface,
    ) -> Self {
        Self {
            operation,
            requirement: RuntimeProviderOperationRequirement::LegacyCompatibility,
            surface,
        }
    }

    pub fn requirement_key(self) -> &'static str {
        self.requirement.as_str()
    }

    pub fn surface_key(self) -> &'static str {
        self.surface.as_str()
    }
}

impl RuntimeProviderOperationCounts {
    pub fn new(query_count: u64, result_count: u64) -> Self {
        Self {
            query_count,
            result_count,
        }
    }

    pub fn from_result_count(result_count: usize) -> Self {
        let result_count = u64::try_from(result_count).unwrap_or(u64::MAX);
        Self::new(result_count, result_count)
    }

    pub fn from_counts(query_count: i64, result_count: i64) -> Self {
        Self::new(
            non_negative_count(query_count),
            non_negative_count(result_count),
        )
    }

    pub fn from_affected_rows(affected_rows: i64) -> Self {
        let count = non_negative_count(affected_rows);
        Self::new(count, count)
    }
}

fn non_negative_count(value: i64) -> u64 {
    u64::try_from(value).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_descriptor_uses_canonical_provider_keys() {
        let descriptor = RuntimeProviderDescriptor::new(FrameworkProvider::Mssql, "crm_primary");

        assert_eq!(descriptor.provider_key(), "mssql");
        assert_eq!(descriptor.data_source_name(), "crm_primary");
        assert_eq!(
            descriptor.unsupported_explain_diagnostic(),
            json!({
                "status": "unsupported",
                "message": "safe provider EXPLAIN diagnostics are not implemented for this provider",
                "provider": "mssql",
                "data_source": "crm_primary",
            })
        );
        assert_eq!(descriptor.opaque_pool_stats().provider, "mssql");
    }

    #[test]
    fn provider_operations_have_stable_metric_tokens() {
        let tokens: Vec<_> = RuntimeProviderOperation::ALL
            .iter()
            .map(|operation| operation.as_str())
            .collect();

        assert_eq!(
            tokens,
            vec![
                "health_check",
                "explain_query_plan",
                "create_item",
                "update_item",
                "delete_item",
                "find_item",
                "get_items",
                "query_items",
                "batch_find_items_by_ids",
                "aggregate_items",
                "append_audit_event",
                "query_audit_events",
            ]
        );
    }

    #[test]
    fn provider_operation_counts_normalize_signed_provider_counts() {
        assert_eq!(
            RuntimeProviderOperationCounts::from_counts(12, 3),
            RuntimeProviderOperationCounts::new(12, 3)
        );
        assert_eq!(
            RuntimeProviderOperationCounts::from_counts(-1, -2),
            RuntimeProviderOperationCounts::new(0, 0)
        );
        assert_eq!(
            RuntimeProviderOperationCounts::from_result_count(2),
            RuntimeProviderOperationCounts::new(2, 2)
        );
    }

    #[test]
    fn provider_client_exposes_runtime_operation_contracts() {
        struct TestProvider;

        impl RuntimeProviderIdentity for TestProvider {
            fn data_source_name(&self) -> &str {
                "crm_primary"
            }

            fn framework_provider(&self) -> FrameworkProvider {
                FrameworkProvider::Postgres
            }
        }

        let provider = TestProvider;
        let descriptor = provider.provider_descriptor();
        let pool_stats = provider.pool_stats();
        let contracts = provider.provider_operation_contracts();

        assert_eq!(descriptor.provider, FrameworkProvider::Postgres);
        assert_eq!(descriptor.data_source_name(), "crm_primary");
        assert_eq!(pool_stats.provider, "postgres");
        assert_eq!(pool_stats.data_source, "crm_primary");
        assert!(!pool_stats.instrumented);
        assert_eq!(contracts.len(), RuntimeProviderOperation::ALL.len());
        assert_eq!(
            contracts[0],
            RuntimeProviderOperationContract::required(
                RuntimeProviderOperation::HealthCheck,
                RuntimeProviderOperationSurface::RuntimeNative,
            )
        );
        assert_eq!(
            contracts
                .iter()
                .find(|contract| contract.operation == RuntimeProviderOperation::AppendAuditEvent)
                .expect("append audit contract")
                .surface_key(),
            "runtime_native"
        );
        assert_eq!(
            contracts
                .iter()
                .find(|contract| contract.operation == RuntimeProviderOperation::QueryItems)
                .expect("query items contract")
                .surface,
            RuntimeProviderOperationSurface::ProductPlanAdapter
        );
        assert!(provider.provider_declares_operation(RuntimeProviderOperation::BatchFindItemsByIds));
    }

    #[test]
    fn provider_identity_can_override_operation_contracts() {
        struct LimitedProvider;

        const LIMITED_CONTRACTS: &[RuntimeProviderOperationContract] = &[
            RuntimeProviderOperationContract::required(
                RuntimeProviderOperation::HealthCheck,
                RuntimeProviderOperationSurface::RuntimeNative,
            ),
            RuntimeProviderOperationContract::optional(
                RuntimeProviderOperation::ExplainQueryPlan,
                RuntimeProviderOperationSurface::ProductPlanAdapter,
            ),
        ];

        impl RuntimeProviderIdentity for LimitedProvider {
            fn data_source_name(&self) -> &str {
                "crm_primary"
            }

            fn framework_provider(&self) -> FrameworkProvider {
                FrameworkProvider::Mssql
            }

            fn provider_operation_contracts(&self) -> &'static [RuntimeProviderOperationContract] {
                LIMITED_CONTRACTS
            }
        }

        let provider = LimitedProvider;

        assert!(provider.provider_declares_operation(RuntimeProviderOperation::HealthCheck));
        assert!(provider.provider_declares_operation(RuntimeProviderOperation::ExplainQueryPlan));
        assert!(!provider.provider_declares_operation(RuntimeProviderOperation::QueryItems));
    }
}
