use std::{future::Future, time::Instant};

use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::{
    extension::UserAuth,
    model_metadata::RuntimeEntityMetadata,
    provider_bridge::{RuntimeProviderDataClient, RuntimeProviderOperation},
    provider_request::RuntimeProviderPlanInput,
    provider_result::{RuntimeJsonAggregateResult, RuntimeJsonObj, RuntimeJsonQueryResult},
    query_cost::{QueryCost, QueryCostBudget},
    query_ir::{
        encode_keyset_cursor, RuntimePagination, RuntimePaginationStrategy, RuntimeSortDirection,
    },
    record_audit, record_validation, AccessAction, DataStoreError, PolicyAccess, RuntimeError,
};

pub use crate::provider_bridge::RuntimeProviderOperationCounts;

#[derive(Clone, Debug, Serialize)]
pub struct RuntimeQueryPlanDiagnostic {
    pub schema_name: String,
    pub type_name: String,
    pub provider: String,
    pub data_source: String,
    pub pagination: RuntimePaginationDiagnostic,
    pub access_filter_applied: bool,
    pub cost: QueryCost,
    pub budget: QueryCostBudget,
    pub provider_diagnostic: Value,
}

#[derive(Clone, Debug, Serialize)]
pub struct RuntimePaginationDiagnostic {
    pub strategy: &'static str,
    pub skip: i32,
    pub limit: i32,
    pub after_present: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeQueryCursors {
    pub previous_cursor: Option<String>,
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeReadSort {
    pub field: String,
    pub direction: RuntimeSortDirection,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeQueryItemsFinalization {
    pub previous_cursor: Option<String>,
    pub next_cursor: Option<String>,
    pub items: Vec<Map<String, Value>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeMutationKind {
    Create,
    Update,
    Delete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeDeleteAuditOutcome {
    Mutation,
    NotApplied,
}

impl RuntimeMutationKind {
    pub fn provider_operation(self) -> RuntimeProviderOperation {
        match self {
            Self::Create => RuntimeProviderOperation::CreateItem,
            Self::Update => RuntimeProviderOperation::UpdateItem,
            Self::Delete => RuntimeProviderOperation::DeleteItem,
        }
    }

    pub fn provider_operation_counts(self, affected_rows: i64) -> RuntimeProviderOperationCounts {
        match self {
            Self::Create | Self::Update => RuntimeProviderOperationCounts::new(1, 1),
            Self::Delete => RuntimeProviderOperationCounts::from_affected_rows(affected_rows),
        }
    }
}

pub fn pagination_diagnostic(pagination: &RuntimePagination) -> RuntimePaginationDiagnostic {
    RuntimePaginationDiagnostic {
        strategy: match &pagination.strategy {
            RuntimePaginationStrategy::Offset => "offset",
            RuntimePaginationStrategy::Keyset { .. } => "keyset",
        },
        skip: pagination.skip,
        limit: pagination.limit,
        after_present: pagination.cursor_after().is_some(),
    }
}

pub async fn provider_health_check<C>(provider: &C) -> Result<(), C::Error>
where
    C: RuntimeProviderDataClient + ?Sized,
    C::Error: From<RuntimeError>,
{
    ensure_provider_operation(provider, RuntimeProviderOperation::HealthCheck)?;
    provider.health_check().await
}

pub async fn provider_create_item_plan_json<C>(
    provider: &C,
    input: RuntimeProviderPlanInput<'_, C::MutationPlan>,
) -> Result<RuntimeJsonObj, C::Error>
where
    C: RuntimeProviderDataClient + ?Sized,
    C::Error: From<RuntimeError>,
{
    ensure_provider_operation(provider, RuntimeProviderOperation::CreateItem)?;
    provider.create_item_plan_json(input).await
}

pub async fn provider_update_item_plan_json<C>(
    provider: &C,
    input: RuntimeProviderPlanInput<'_, C::MutationPlan>,
) -> Result<RuntimeJsonObj, C::Error>
where
    C: RuntimeProviderDataClient + ?Sized,
    C::Error: From<RuntimeError>,
{
    ensure_provider_operation(provider, RuntimeProviderOperation::UpdateItem)?;
    provider.update_item_plan_json(input).await
}

pub async fn provider_delete_item_plan_json<C>(
    provider: &C,
    input: RuntimeProviderPlanInput<'_, C::MutationPlan>,
) -> Result<i64, C::Error>
where
    C: RuntimeProviderDataClient + ?Sized,
    C::Error: From<RuntimeError>,
{
    ensure_provider_operation(provider, RuntimeProviderOperation::DeleteItem)?;
    provider.delete_item_plan_json(input).await
}

pub async fn provider_find_item_json<C>(
    provider: &C,
    entity_type: C::Entity,
    selections: Value,
    id: String,
    user: &UserAuth,
    access: &PolicyAccess,
) -> Result<Option<RuntimeJsonObj>, C::Error>
where
    C: RuntimeProviderDataClient + ?Sized,
    C::Error: From<RuntimeError>,
{
    ensure_provider_operation(provider, RuntimeProviderOperation::FindItem)?;
    provider
        .find_item_json(entity_type, selections, id, user, access)
        .await
}

pub async fn provider_get_items_plan_json<C>(
    provider: &C,
    input: RuntimeProviderPlanInput<'_, C::QueryPlan>,
) -> Result<Vec<RuntimeJsonObj>, C::Error>
where
    C: RuntimeProviderDataClient + ?Sized,
    C::Error: From<RuntimeError>,
{
    ensure_provider_operation(provider, RuntimeProviderOperation::GetItems)?;
    provider.get_items_plan_json(input).await
}

pub async fn provider_query_items_plan_json<C>(
    provider: &C,
    input: RuntimeProviderPlanInput<'_, C::QueryPlan>,
) -> Result<RuntimeJsonQueryResult, C::Error>
where
    C: RuntimeProviderDataClient + ?Sized,
    C::Error: From<RuntimeError>,
{
    ensure_provider_operation(provider, RuntimeProviderOperation::QueryItems)?;
    provider.query_items_plan_json(input).await
}

pub async fn provider_batch_get_items_plan_json<C>(
    provider: &C,
    input: RuntimeProviderPlanInput<'_, C::QueryPlan>,
) -> Result<Vec<RuntimeJsonObj>, C::Error>
where
    C: RuntimeProviderDataClient + ?Sized,
    C::Error: From<RuntimeError>,
{
    ensure_provider_operation(provider, RuntimeProviderOperation::BatchFindItemsByIds)?;
    provider.batch_get_items_plan_json(input).await
}

pub async fn provider_aggregate_items_plan_json<C>(
    provider: &C,
    input: RuntimeProviderPlanInput<'_, C::AggregatePlan>,
) -> Result<RuntimeJsonAggregateResult, C::Error>
where
    C: RuntimeProviderDataClient + ?Sized,
    C::Error: From<RuntimeError>,
{
    ensure_provider_operation(provider, RuntimeProviderOperation::AggregateItems)?;
    provider.aggregate_items_plan_json(input).await
}

pub async fn provider_explain_query_plan<C>(
    provider: &C,
    input: RuntimeProviderPlanInput<'_, &C::QueryPlan>,
) -> Result<Value, C::Error>
where
    C: RuntimeProviderDataClient + ?Sized,
    C::Error: From<RuntimeError>,
{
    ensure_provider_operation(provider, RuntimeProviderOperation::ExplainQueryPlan)?;
    provider.explain_query_plan(input).await
}

pub async fn provider_append_audit_event<C>(
    provider: &C,
    event: record_audit::RuntimeAuditEvent,
) -> Result<(), C::Error>
where
    C: RuntimeProviderDataClient + ?Sized,
    C::Error: From<RuntimeError>,
{
    ensure_provider_operation(provider, RuntimeProviderOperation::AppendAuditEvent)?;
    provider.append_audit_event(event).await
}

pub async fn provider_query_audit_events<C>(
    provider: &C,
    query: record_audit::RuntimeAuditQuery,
) -> Result<Vec<Value>, C::Error>
where
    C: RuntimeProviderDataClient + ?Sized,
    C::Error: From<RuntimeError>,
{
    ensure_provider_operation(provider, RuntimeProviderOperation::QueryAuditEvents)?;
    provider.query_audit_events(query).await
}

pub async fn execute_audit_events_read<C>(
    provider: &C,
    entity_type: C::Entity,
    selections: Value,
    record_id: String,
    user: &UserAuth,
    access: &PolicyAccess,
    query: record_audit::RuntimeAuditQuery,
) -> Result<Vec<Value>, C::Error>
where
    C: RuntimeProviderDataClient + ?Sized,
    C::Error: From<RuntimeError>,
{
    if query.tenant_id.trim().is_empty() || query.tenant_id != user.tenant_id {
        return Err(RuntimeError::DataAccess(
            "audit query tenant does not match authenticated tenant".to_string(),
        )
        .into());
    }
    if !access.allow {
        return Ok(Vec::new());
    }

    let visible =
        provider_find_item_json(provider, entity_type, selections, record_id, user, access).await?;
    if visible.is_none() {
        return Ok(Vec::new());
    }

    provider_query_audit_events(provider, query).await
}

pub async fn execute_find_item_read<C>(
    provider: &C,
    entity_type: C::Entity,
    selections: Value,
    id: String,
    user: &UserAuth,
    access: &PolicyAccess,
    trace: impl FnOnce(RuntimeProviderOperation, Instant, RuntimeProviderOperationCounts),
    evaluate: impl FnMut(RuntimeJsonObj) -> Result<RuntimeJsonObj, C::Error>,
) -> Result<Option<RuntimeJsonObj>, C::Error>
where
    C: RuntimeProviderDataClient + ?Sized,
    C::Error: From<RuntimeError>,
{
    execute_optional_read(
        RuntimeProviderOperation::FindItem,
        || provider_find_item_json(provider, entity_type, selections, id, user, access),
        trace,
        evaluate,
    )
    .await
}

pub async fn execute_get_items_plan_read<C>(
    provider: &C,
    plan: C::QueryPlan,
    user: &UserAuth,
    access: &PolicyAccess,
    trace: impl FnOnce(RuntimeProviderOperation, Instant, RuntimeProviderOperationCounts),
    evaluate: impl FnMut(RuntimeJsonObj) -> Result<RuntimeJsonObj, C::Error>,
) -> Result<Vec<RuntimeJsonObj>, C::Error>
where
    C: RuntimeProviderDataClient + ?Sized,
    C::Error: From<RuntimeError>,
{
    execute_list_read(
        RuntimeProviderOperation::GetItems,
        || {
            provider_get_items_plan_json(
                provider,
                RuntimeProviderPlanInput::new(plan, user, access),
            )
        },
        trace,
        evaluate,
    )
    .await
}

pub async fn execute_batch_get_items_plan_read<C>(
    provider: &C,
    plan: C::QueryPlan,
    user: &UserAuth,
    access: &PolicyAccess,
    trace: impl FnOnce(RuntimeProviderOperation, Instant, RuntimeProviderOperationCounts),
    evaluate: impl FnMut(RuntimeJsonObj) -> Result<RuntimeJsonObj, C::Error>,
) -> Result<Vec<RuntimeJsonObj>, C::Error>
where
    C: RuntimeProviderDataClient + ?Sized,
    C::Error: From<RuntimeError>,
{
    execute_list_read(
        RuntimeProviderOperation::BatchFindItemsByIds,
        || {
            provider_batch_get_items_plan_json(
                provider,
                RuntimeProviderPlanInput::new(plan, user, access),
            )
        },
        trace,
        evaluate,
    )
    .await
}

pub async fn execute_query_items_plan_read<C>(
    provider: &C,
    plan: C::QueryPlan,
    pagination: &RuntimePagination,
    sort: Option<&RuntimeReadSort>,
    user: &UserAuth,
    access: &PolicyAccess,
    trace: impl FnOnce(RuntimeProviderOperation, Instant, RuntimeProviderOperationCounts),
    evaluate: impl FnMut(RuntimeJsonObj) -> Result<RuntimeJsonObj, C::Error>,
) -> Result<RuntimeJsonQueryResult, C::Error>
where
    C: RuntimeProviderDataClient + ?Sized,
    C::Error: From<RuntimeError>,
{
    execute_query_page_read(
        pagination,
        sort,
        || {
            provider_query_items_plan_json(
                provider,
                RuntimeProviderPlanInput::new(plan, user, access),
            )
        },
        |result| result.query_count,
        |result| std::mem::take(&mut result.items),
        |result, finalized| {
            result.items = finalized.items;
            result.previous_cursor = finalized.previous_cursor;
            result.next_cursor = finalized.next_cursor;
        },
        trace,
        evaluate,
    )
    .await
}

pub async fn execute_aggregate_items_plan_read<C>(
    provider: &C,
    plan: C::AggregatePlan,
    user: &UserAuth,
    access: &PolicyAccess,
    trace: impl FnOnce(RuntimeProviderOperation, Instant, RuntimeProviderOperationCounts),
) -> Result<RuntimeJsonAggregateResult, C::Error>
where
    C: RuntimeProviderDataClient + ?Sized,
    C::Error: From<RuntimeError>,
{
    execute_aggregate_read(
        || {
            provider_aggregate_items_plan_json(
                provider,
                RuntimeProviderPlanInput::new(plan, user, access),
            )
        },
        |result| result.query_count,
        |result| result.items.len() as i64,
        trace,
    )
    .await
}

pub async fn execute_create_item_plan_mutation<C>(
    provider: &C,
    plan: C::MutationPlan,
    user: &UserAuth,
    access: &PolicyAccess,
    trace: impl FnOnce(RuntimeProviderOperation, Instant, RuntimeProviderOperationCounts),
) -> Result<RuntimeJsonObj, C::Error>
where
    C: RuntimeProviderDataClient + ?Sized,
    C::Error: From<RuntimeError>,
{
    execute_mutation(
        RuntimeMutationKind::Create,
        || {
            provider_create_item_plan_json(
                provider,
                RuntimeProviderPlanInput::new(plan, user, access),
            )
        },
        |_result| 1,
        trace,
    )
    .await
}

pub async fn execute_update_item_plan_mutation<C>(
    provider: &C,
    plan: C::MutationPlan,
    user: &UserAuth,
    access: &PolicyAccess,
    trace: impl FnOnce(RuntimeProviderOperation, Instant, RuntimeProviderOperationCounts),
) -> Result<RuntimeJsonObj, C::Error>
where
    C: RuntimeProviderDataClient + ?Sized,
    C::Error: From<RuntimeError>,
{
    execute_mutation(
        RuntimeMutationKind::Update,
        || {
            provider_update_item_plan_json(
                provider,
                RuntimeProviderPlanInput::new(plan, user, access),
            )
        },
        |_result| 1,
        trace,
    )
    .await
}

pub async fn execute_delete_item_plan_mutation<C>(
    provider: &C,
    plan: C::MutationPlan,
    user: &UserAuth,
    access: &PolicyAccess,
    trace: impl FnOnce(RuntimeProviderOperation, Instant, RuntimeProviderOperationCounts),
) -> Result<i64, C::Error>
where
    C: RuntimeProviderDataClient + ?Sized,
    C::Error: From<RuntimeError>,
{
    execute_mutation(
        RuntimeMutationKind::Delete,
        || {
            provider_delete_item_plan_json(
                provider,
                RuntimeProviderPlanInput::new(plan, user, access),
            )
        },
        |deleted_count| *deleted_count,
        trace,
    )
    .await
}

pub async fn validate_primary_key_available<C>(
    provider: &C,
    entity_type: C::Entity,
    selections: Value,
    record_id: Option<String>,
    user: &UserAuth,
    access: &PolicyAccess,
) -> Result<(), C::Error>
where
    C: RuntimeProviderDataClient + ?Sized,
    C::Error: From<RuntimeError>,
{
    let Some(record_id) = record_id else {
        return Ok(());
    };
    let existing =
        provider_find_item_json(provider, entity_type, selections, record_id, user, access).await?;
    if existing.is_some() {
        return Err(RuntimeError::DataStore(DataStoreError::DuplicateKey).into());
    }
    Ok(())
}

pub async fn validate_foreign_key_exists<C>(
    provider: &C,
    entity_type: C::Entity,
    selections: Value,
    record_id: Option<String>,
    user: &UserAuth,
    access: &PolicyAccess,
) -> Result<(), C::Error>
where
    C: RuntimeProviderDataClient + ?Sized,
    C::Error: From<RuntimeError>,
{
    let Some(record_id) = record_id else {
        return Ok(());
    };
    let existing =
        provider_find_item_json(provider, entity_type, selections, record_id, user, access).await?;
    if existing.is_none() {
        return Err(RuntimeError::DataStore(DataStoreError::ForeignKeyViolation).into());
    }
    Ok(())
}

pub async fn validate_unique_record<C>(
    provider: &C,
    plan: C::QueryPlan,
    user: &UserAuth,
    access: &PolicyAccess,
    primary_key_name: &str,
    record: &Map<String, Value>,
    entity_name: &str,
    property_name: &str,
    message: &str,
) -> Result<(), C::Error>
where
    C: RuntimeProviderDataClient + ?Sized,
    C::Error: From<RuntimeError>,
{
    let candidates =
        provider_query_items_plan_json(provider, RuntimeProviderPlanInput::new(plan, user, access))
            .await?;
    if record_validation::uniqueness_conflict(primary_key_name, record, &candidates.items) {
        return Err(RuntimeError::Validation(format!(
            "{}.{}: {}",
            entity_name, property_name, message
        ))
        .into());
    }
    Ok(())
}

fn ensure_provider_operation<C>(
    provider: &C,
    operation: RuntimeProviderOperation,
) -> Result<(), C::Error>
where
    C: RuntimeProviderDataClient + ?Sized,
    C::Error: From<RuntimeError>,
{
    if provider.provider_declares_operation(operation) {
        return Ok(());
    }

    let descriptor = provider.provider_descriptor();
    Err(RuntimeError::DataAccess(format!(
        "provider '{}' for data source '{}' does not declare operation '{}'",
        descriptor.provider_key(),
        descriptor.data_source_name(),
        operation.as_str()
    ))
    .into())
}

pub fn batch_limit_for_ids(ids_len: usize) -> Result<i32, RuntimeError> {
    i32::try_from(ids_len).map_err(|_| {
        RuntimeError::Validation("batch load size exceeds supported query limit".to_string())
    })
}

pub fn primary_key_in_filter(
    primary_key_name: &str,
    ids: Vec<Value>,
) -> Result<Value, RuntimeError> {
    let primary_key_name = primary_key_name.trim();
    if primary_key_name.is_empty() {
        return Err(RuntimeError::Metadata(
            crate::MetadataError::MissingPrimaryKey {
                entity_type: "batch read".to_string(),
            },
        ));
    }
    Ok(json!({ primary_key_name: { "_in": ids } }))
}

pub fn keyset_cursors_from_items(
    pagination: &RuntimePagination,
    sort_field: &str,
    sort_direction: RuntimeSortDirection,
    items: &[Map<String, Value>],
) -> Result<RuntimeQueryCursors, RuntimeError> {
    let previous_cursor = pagination.cursor_after();
    let next_cursor = next_keyset_cursor_from_items(pagination, sort_field, sort_direction, items)?;
    Ok(RuntimeQueryCursors {
        previous_cursor,
        next_cursor,
    })
}

pub fn evaluate_optional_read_record<E>(
    record: Option<Map<String, Value>>,
    mut evaluate: impl FnMut(Map<String, Value>) -> Result<Map<String, Value>, E>,
) -> Result<Option<Map<String, Value>>, E> {
    record.map(&mut evaluate).transpose()
}

pub fn evaluate_read_records<E>(
    items: Vec<Map<String, Value>>,
    mut evaluate: impl FnMut(Map<String, Value>) -> Result<Map<String, Value>, E>,
) -> Result<Vec<Map<String, Value>>, E> {
    items.into_iter().map(&mut evaluate).collect()
}

pub async fn execute_optional_read<E, Fut>(
    operation: RuntimeProviderOperation,
    provider_call: impl FnOnce() -> Fut,
    trace: impl FnOnce(RuntimeProviderOperation, Instant, RuntimeProviderOperationCounts),
    evaluate: impl FnMut(Map<String, Value>) -> Result<Map<String, Value>, E>,
) -> Result<Option<Map<String, Value>>, E>
where
    Fut: Future<Output = Result<Option<Map<String, Value>>, E>>,
{
    let started_at = Instant::now();
    let record = provider_call().await?;
    let result_count = if record.is_some() { 1 } else { 0 };
    trace(
        operation,
        started_at,
        RuntimeProviderOperationCounts::from_counts(result_count, result_count),
    );
    evaluate_optional_read_record(record, evaluate)
}

pub async fn execute_list_read<E, Fut>(
    operation: RuntimeProviderOperation,
    provider_call: impl FnOnce() -> Fut,
    trace: impl FnOnce(RuntimeProviderOperation, Instant, RuntimeProviderOperationCounts),
    evaluate: impl FnMut(Map<String, Value>) -> Result<Map<String, Value>, E>,
) -> Result<Vec<Map<String, Value>>, E>
where
    Fut: Future<Output = Result<Vec<Map<String, Value>>, E>>,
{
    let started_at = Instant::now();
    let items = provider_call().await?;
    let counts = RuntimeProviderOperationCounts::from_result_count(items.len());
    trace(operation, started_at, counts);
    evaluate_read_records(items, evaluate)
}

pub fn finalize_query_items_page<E>(
    pagination: &RuntimePagination,
    sort: Option<&RuntimeReadSort>,
    items: Vec<Map<String, Value>>,
    evaluate: impl FnMut(Map<String, Value>) -> Result<Map<String, Value>, E>,
) -> Result<RuntimeQueryItemsFinalization, E>
where
    E: From<RuntimeError>,
{
    let cursors = match sort {
        Some(sort) => keyset_cursors_from_items(pagination, &sort.field, sort.direction, &items)
            .map_err(E::from)?,
        None => RuntimeQueryCursors {
            previous_cursor: pagination.cursor_after(),
            next_cursor: None,
        },
    };
    let items = evaluate_read_records(items, evaluate)?;
    Ok(RuntimeQueryItemsFinalization {
        previous_cursor: cursors.previous_cursor,
        next_cursor: cursors.next_cursor,
        items,
    })
}

pub async fn execute_query_page_read<R, E, Fut>(
    pagination: &RuntimePagination,
    sort: Option<&RuntimeReadSort>,
    provider_call: impl FnOnce() -> Fut,
    query_count: impl FnOnce(&R) -> i64,
    take_items: impl FnOnce(&mut R) -> Vec<Map<String, Value>>,
    apply_finalization: impl FnOnce(&mut R, RuntimeQueryItemsFinalization),
    trace: impl FnOnce(RuntimeProviderOperation, Instant, RuntimeProviderOperationCounts),
    evaluate: impl FnMut(Map<String, Value>) -> Result<Map<String, Value>, E>,
) -> Result<R, E>
where
    Fut: Future<Output = Result<R, E>>,
    E: From<RuntimeError>,
{
    let started_at = Instant::now();
    let mut result = provider_call().await?;
    let query_count = query_count(&result);
    let items = take_items(&mut result);
    trace(
        RuntimeProviderOperation::QueryItems,
        started_at,
        RuntimeProviderOperationCounts::from_counts(query_count, items.len() as i64),
    );
    let finalized = finalize_query_items_page(pagination, sort, items, evaluate)?;
    apply_finalization(&mut result, finalized);
    Ok(result)
}

pub async fn execute_aggregate_read<R, E, Fut>(
    provider_call: impl FnOnce() -> Fut,
    query_count: impl FnOnce(&R) -> i64,
    result_count: impl FnOnce(&R) -> i64,
    trace: impl FnOnce(RuntimeProviderOperation, Instant, RuntimeProviderOperationCounts),
) -> Result<R, E>
where
    Fut: Future<Output = Result<R, E>>,
{
    let started_at = Instant::now();
    let result = provider_call().await?;
    trace(
        RuntimeProviderOperation::AggregateItems,
        started_at,
        RuntimeProviderOperationCounts::from_counts(query_count(&result), result_count(&result)),
    );
    Ok(result)
}

pub async fn execute_mutation<R, E, Fut>(
    kind: RuntimeMutationKind,
    provider_call: impl FnOnce() -> Fut,
    affected_rows: impl FnOnce(&R) -> i64,
    trace: impl FnOnce(RuntimeProviderOperation, Instant, RuntimeProviderOperationCounts),
) -> Result<R, E>
where
    Fut: Future<Output = Result<R, E>>,
{
    let started_at = Instant::now();
    let result = provider_call().await?;
    trace(
        kind.provider_operation(),
        started_at,
        kind.provider_operation_counts(affected_rows(&result)),
    );
    Ok(result)
}

pub async fn append_audit_mutation<E, Fut>(
    entity: &RuntimeEntityMetadata,
    action: AccessAction,
    user: &UserAuth,
    record_id: Option<String>,
    before_json: Option<Value>,
    after_json: Option<Value>,
    access: &PolicyAccess,
    append_event: impl FnOnce(record_audit::RuntimeAuditEvent) -> Fut,
) -> Result<(), E>
where
    Fut: Future<Output = Result<(), E>>,
{
    if !record_audit::is_audited(entity) {
        return Ok(());
    }

    let event = record_audit::RuntimeAuditEvent::entity_mutation(
        entity,
        action,
        user,
        record_id,
        before_json,
        after_json,
        access,
    );
    append_event(event).await
}

pub async fn append_audit_attempt<E, Fut>(
    entity: &RuntimeEntityMetadata,
    action: AccessAction,
    user: Option<&UserAuth>,
    outcome: &str,
    record_id: Option<String>,
    before_json: Option<Value>,
    after_json: Option<Value>,
    policy_json: Option<Value>,
    append_event: impl FnOnce(record_audit::RuntimeAuditEvent) -> Fut,
) -> Result<(), E>
where
    Fut: Future<Output = Result<(), E>>,
{
    if !record_audit::is_audited(entity) {
        return Ok(());
    }

    let event = record_audit::RuntimeAuditEvent::entity_attempt(
        entity,
        action,
        user,
        outcome,
        record_id,
        before_json,
        after_json,
        policy_json,
    );
    append_event(event).await
}

pub async fn append_audit_attempt_on_record_chain<E, AppendFut>(
    entity: &RuntimeEntityMetadata,
    action: AccessAction,
    user: Option<&UserAuth>,
    outcome: &str,
    record_id: Option<String>,
    before_json: Option<Value>,
    after_json: Option<Value>,
    policy_json: Option<Value>,
    append_event: impl FnOnce(record_audit::RuntimeAuditEvent) -> AppendFut,
) -> Result<(), E>
where
    AppendFut: Future<Output = Result<(), E>>,
{
    if !record_audit::is_audited(entity) {
        return Ok(());
    }

    let event = record_audit::RuntimeAuditEvent::entity_attempt(
        entity,
        action,
        user,
        outcome,
        record_id,
        before_json,
        after_json,
        policy_json,
    );

    append_event(event).await
}

pub async fn append_missing_user_audit_attempt<E, Fut>(
    entity: &RuntimeEntityMetadata,
    action: AccessAction,
    record_id: Option<String>,
    error: impl ToString,
    append_event: impl FnOnce(record_audit::RuntimeAuditEvent) -> Fut,
) -> Result<(), E>
where
    Fut: Future<Output = Result<(), E>>,
{
    append_audit_attempt(
        entity,
        action,
        None,
        "denied",
        record_id,
        None,
        None,
        Some(record_audit::policy_error_json("missing_user", error)),
        append_event,
    )
    .await
}

pub async fn append_policy_denied_audit_attempt_on_record_chain<E, AppendFut>(
    entity: &RuntimeEntityMetadata,
    action: AccessAction,
    user: &UserAuth,
    record_id: Option<String>,
    before_json: Option<Value>,
    attempted_json: Option<Value>,
    access: &PolicyAccess,
    append_event: impl FnOnce(record_audit::RuntimeAuditEvent) -> AppendFut,
) -> Result<(), E>
where
    AppendFut: Future<Output = Result<(), E>>,
{
    append_audit_attempt_on_record_chain(
        entity,
        action,
        Some(user),
        "denied",
        record_id,
        before_json,
        attempted_json,
        Some(record_audit::policy_decision_json(access, "policy_denied")),
        append_event,
    )
    .await
}

pub async fn append_policy_error_audit_attempt<E, Fut>(
    entity: &RuntimeEntityMetadata,
    action: AccessAction,
    user: &UserAuth,
    record_id: Option<String>,
    attempted_json: Option<Value>,
    error: impl ToString,
    append_event: impl FnOnce(record_audit::RuntimeAuditEvent) -> Fut,
) -> Result<(), E>
where
    Fut: Future<Output = Result<(), E>>,
{
    append_audit_attempt(
        entity,
        action,
        Some(user),
        "failed",
        record_id,
        None,
        attempted_json,
        Some(record_audit::policy_error_json(
            "policy_evaluation_failed",
            error,
        )),
        append_event,
    )
    .await
}

pub async fn append_operation_failed_audit_attempt<E, Fut>(
    entity: &RuntimeEntityMetadata,
    action: AccessAction,
    user: &UserAuth,
    record_id: Option<String>,
    before_json: Option<Value>,
    after_json: Option<Value>,
    error: impl ToString,
    append_event: impl FnOnce(record_audit::RuntimeAuditEvent) -> Fut,
) -> Result<(), E>
where
    Fut: Future<Output = Result<(), E>>,
{
    append_audit_attempt(
        entity,
        action,
        Some(user),
        "failed",
        record_id,
        before_json,
        after_json,
        Some(record_audit::operation_error_json("mutation_failed", error)),
        append_event,
    )
    .await
}

pub async fn append_operation_not_applied_audit_attempt<E, Fut>(
    entity: &RuntimeEntityMetadata,
    action: AccessAction,
    user: &UserAuth,
    record_id: Option<String>,
    before_json: Option<Value>,
    append_event: impl FnOnce(record_audit::RuntimeAuditEvent) -> Fut,
) -> Result<(), E>
where
    Fut: Future<Output = Result<(), E>>,
{
    append_audit_attempt(
        entity,
        action,
        Some(user),
        "not_applied",
        record_id,
        before_json,
        None,
        Some(record_audit::operation_decision_json("no_rows_affected")),
        append_event,
    )
    .await
}

#[cfg(feature = "mcp")]
pub async fn append_mcp_tool_audit_event<E, Fut>(
    entity: &RuntimeEntityMetadata,
    user: &UserAuth,
    outcome: &str,
    metadata_json: Value,
    append_event: impl FnOnce(record_audit::RuntimeAuditEvent) -> Fut,
) -> Result<(), E>
where
    Fut: Future<Output = Result<(), E>>,
{
    if !record_audit::is_audited(entity) {
        return Ok(());
    }

    let event = record_audit::RuntimeAuditEvent::entity_event(
        entity,
        "mcp_tool",
        Some(user),
        outcome,
        None,
        None,
        None,
        Some(metadata_json),
    );
    append_event(event).await
}

pub fn next_keyset_cursor_from_items(
    pagination: &RuntimePagination,
    sort_field: &str,
    sort_direction: RuntimeSortDirection,
    items: &[Map<String, Value>],
) -> Result<Option<String>, RuntimeError> {
    if !pagination.is_keyset() {
        return Ok(None);
    }
    let page_limit = usize::try_from(pagination.limit).unwrap_or(usize::MAX);
    if items.len() < page_limit {
        return Ok(None);
    }
    let sort_field = sort_field.trim();
    if sort_field.is_empty() {
        return Ok(None);
    }
    let Some(value) = items.last().and_then(|item| item.get(sort_field)).cloned() else {
        return Ok(None);
    };
    encode_keyset_cursor(sort_field, sort_direction, value).map(Some)
}

pub fn should_check_filtered_update_denial(
    access_filter: Option<&Value>,
    record_id: Option<&str>,
    access_visible_before: Option<&Map<String, Value>>,
) -> bool {
    access_filter.is_some()
        && record_id.and_then(non_empty).is_some()
        && access_visible_before.is_none()
}

pub fn mutation_record_id(
    entity: &RuntimeEntityMetadata,
    kind: RuntimeMutationKind,
    input: Option<&Map<String, Value>>,
    provider_result: Option<&Map<String, Value>>,
    before: Option<&Map<String, Value>>,
) -> Option<String> {
    match kind {
        RuntimeMutationKind::Create => provider_result
            .and_then(|record| record_audit::record_id(entity, record))
            .or_else(|| input.and_then(|record| record_audit::record_id(entity, record))),
        RuntimeMutationKind::Update => input
            .and_then(|record| record_audit::record_id(entity, record))
            .or_else(|| provider_result.and_then(|record| record_audit::record_id(entity, record)))
            .or_else(|| before.and_then(|record| record_audit::record_id(entity, record))),
        RuntimeMutationKind::Delete => input
            .and_then(|record| record_audit::record_id(entity, record))
            .or_else(|| before.and_then(|record| record_audit::record_id(entity, record))),
    }
}

pub fn delete_audit_outcome(
    audit_enabled: bool,
    deleted_count: i64,
) -> Option<RuntimeDeleteAuditOutcome> {
    if !audit_enabled {
        None
    } else if deleted_count > 0 {
        Some(RuntimeDeleteAuditOutcome::Mutation)
    } else {
        Some(RuntimeDeleteAuditOutcome::NotApplied)
    }
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::model_metadata::{RuntimeDataType, RuntimePropertyMetadata};

    #[test]
    fn batch_helpers_are_runtime_owned() {
        let filter =
            primary_key_in_filter("id", vec![json!("one"), json!("two")]).expect("batch id filter");

        assert_eq!(batch_limit_for_ids(2).expect("batch limit"), 2);
        assert_eq!(filter, json!({ "id": { "_in": ["one", "two"] } }));
        assert!(batch_limit_for_ids((i32::MAX as usize) + 1).is_err());
        assert!(primary_key_in_filter("", vec![json!("one")]).is_err());
    }

    #[test]
    fn pagination_diagnostic_is_runtime_owned() {
        let pagination =
            RuntimePagination::keyset(Some("cursor".to_string()), 25).expect("keyset pagination");
        let diagnostic = pagination_diagnostic(&pagination);

        assert_eq!(diagnostic.strategy, "keyset");
        assert_eq!(diagnostic.skip, 0);
        assert_eq!(diagnostic.limit, 25);
        assert!(diagnostic.after_present);
    }

    #[test]
    fn keyset_cursor_finalization_is_runtime_owned() {
        let pagination =
            RuntimePagination::keyset(Some("previous".to_string()), 2).expect("keyset pagination");
        let items = vec![
            json!({ "id": "account-1" })
                .as_object()
                .expect("object")
                .clone(),
            json!({ "id": "account-2" })
                .as_object()
                .expect("object")
                .clone(),
        ];

        let cursors =
            keyset_cursors_from_items(&pagination, "id", RuntimeSortDirection::Asc, &items)
                .expect("query cursors");

        assert_eq!(cursors.previous_cursor, Some("previous".to_string()));
        // Cursors are now opaque (base64url + HMAC), so the raw id is not present in the string;
        // decode and verify the signed payload instead.
        let next = cursors.next_cursor.as_deref().expect("next cursor");
        let decoded = crate::query_ir::decode_keyset_cursor(next).expect("decode next cursor");
        assert_eq!(decoded.value, json!("account-2"));
    }

    #[test]
    fn read_record_evaluation_is_runtime_orchestrated() {
        let items = vec![
            json!({ "id": "account-1" })
                .as_object()
                .expect("object")
                .clone(),
            json!({ "id": "account-2" })
                .as_object()
                .expect("object")
                .clone(),
        ];

        let evaluated = evaluate_read_records(items, |mut item| {
            item.insert("evaluated".to_string(), json!(true));
            Ok::<_, RuntimeError>(item)
        })
        .expect("evaluated records");

        assert_eq!(evaluated.len(), 2);
        assert_eq!(evaluated[0]["evaluated"], json!(true));
        assert!(evaluate_optional_read_record::<RuntimeError>(None, Ok)
            .unwrap()
            .is_none());
    }

    #[test]
    fn query_page_finalization_applies_cursors_before_evaluating_records() {
        let pagination =
            RuntimePagination::keyset(Some("previous".to_string()), 2).expect("keyset pagination");
        let items = vec![
            json!({ "id": "account-1" })
                .as_object()
                .expect("object")
                .clone(),
            json!({ "id": "account-2" })
                .as_object()
                .expect("object")
                .clone(),
        ];

        let finalized = finalize_query_items_page(
            &pagination,
            Some(&RuntimeReadSort {
                field: "id".to_string(),
                direction: RuntimeSortDirection::Asc,
            }),
            items,
            |mut item| {
                item.insert("evaluated".to_string(), json!(true));
                Ok::<_, RuntimeError>(item)
            },
        )
        .expect("finalized query page");

        assert_eq!(finalized.previous_cursor, Some("previous".to_string()));
        // Opaque cursor: decode to verify the signed payload carries the last row's sort value.
        let next = finalized.next_cursor.as_deref().expect("next cursor");
        let decoded = crate::query_ir::decode_keyset_cursor(next).expect("decode next cursor");
        assert_eq!(decoded.value, json!("account-2"));
        assert_eq!(finalized.items[0]["evaluated"], json!(true));
    }

    #[tokio::test]
    async fn read_provider_execution_traces_and_evaluates_records() {
        let mut traced = None;

        let evaluated = execute_list_read(
            RuntimeProviderOperation::GetItems,
            || async {
                Ok::<_, RuntimeError>(vec![json!({ "id": "account-1" })
                    .as_object()
                    .expect("object")
                    .clone()])
            },
            |operation, _started_at, counts| traced = Some((operation, counts)),
            |mut item| {
                item.insert("evaluated".to_string(), json!(true));
                Ok::<_, RuntimeError>(item)
            },
        )
        .await
        .expect("executed list read");

        assert_eq!(evaluated[0]["evaluated"], json!(true));
        assert_eq!(
            traced,
            Some((
                RuntimeProviderOperation::GetItems,
                RuntimeProviderOperationCounts::new(1, 1)
            ))
        );
    }

    #[tokio::test]
    async fn query_page_provider_execution_traces_and_finalizes_page() {
        #[derive(Debug)]
        struct QueryPage {
            query_count: i64,
            previous_cursor: Option<String>,
            next_cursor: Option<String>,
            items: Vec<Map<String, Value>>,
        }

        let pagination =
            RuntimePagination::keyset(Some("previous".to_string()), 1).expect("keyset pagination");
        let sort = RuntimeReadSort {
            field: "id".to_string(),
            direction: RuntimeSortDirection::Asc,
        };
        let mut traced = None;

        let page = execute_query_page_read(
            &pagination,
            Some(&sort),
            || async {
                Ok::<_, RuntimeError>(QueryPage {
                    query_count: 7,
                    previous_cursor: None,
                    next_cursor: None,
                    items: vec![json!({ "id": "account-1" })
                        .as_object()
                        .expect("object")
                        .clone()],
                })
            },
            |page| page.query_count,
            |page| std::mem::take(&mut page.items),
            |page, finalized| {
                page.items = finalized.items;
                page.previous_cursor = finalized.previous_cursor;
                page.next_cursor = finalized.next_cursor;
            },
            |operation, _started_at, counts| traced = Some((operation, counts)),
            |mut item| {
                item.insert("evaluated".to_string(), json!(true));
                Ok::<_, RuntimeError>(item)
            },
        )
        .await
        .expect("executed query page read");

        assert_eq!(page.previous_cursor, Some("previous".to_string()));
        // Opaque cursor: decode to verify the signed payload carries the last row's sort value.
        let next = page.next_cursor.as_deref().expect("next cursor");
        let decoded = crate::query_ir::decode_keyset_cursor(next).expect("decode next cursor");
        assert_eq!(decoded.value, json!("account-1"));
        assert_eq!(page.items[0]["evaluated"], json!(true));
        assert_eq!(
            traced,
            Some((
                RuntimeProviderOperation::QueryItems,
                RuntimeProviderOperationCounts::new(7, 1)
            ))
        );
    }

    #[tokio::test]
    async fn aggregate_provider_execution_traces_counts() {
        #[derive(Debug)]
        struct AggregatePage {
            query_count: i64,
            items: Vec<Value>,
        }

        let mut traced = None;
        let page = execute_aggregate_read(
            || async {
                Ok::<_, RuntimeError>(AggregatePage {
                    query_count: 3,
                    items: vec![json!({ "count": 2 }), json!({ "count": 4 })],
                })
            },
            |page| page.query_count,
            |page| page.items.len() as i64,
            |operation, _started_at, counts| traced = Some((operation, counts)),
        )
        .await
        .expect("executed aggregate read");

        assert_eq!(page.items.len(), 2);
        assert_eq!(
            traced,
            Some((
                RuntimeProviderOperation::AggregateItems,
                RuntimeProviderOperationCounts::new(3, 2)
            ))
        );
    }

    #[tokio::test]
    async fn mutation_provider_execution_traces_counts() {
        let mut create_traced = None;
        let created = execute_mutation(
            RuntimeMutationKind::Create,
            || async {
                Ok::<_, RuntimeError>(
                    json!({ "id": "account-1" })
                        .as_object()
                        .expect("object")
                        .clone(),
                )
            },
            |_record| 1,
            |operation, _started_at, counts| create_traced = Some((operation, counts)),
        )
        .await
        .expect("executed create mutation");

        assert_eq!(created["id"], json!("account-1"));
        assert_eq!(
            create_traced,
            Some((
                RuntimeProviderOperation::CreateItem,
                RuntimeProviderOperationCounts::new(1, 1)
            ))
        );

        let mut delete_traced = None;
        let deleted = execute_mutation(
            RuntimeMutationKind::Delete,
            || async { Ok::<_, RuntimeError>(3_i64) },
            |deleted| *deleted,
            |operation, _started_at, counts| delete_traced = Some((operation, counts)),
        )
        .await
        .expect("executed delete mutation");

        assert_eq!(deleted, 3);
        assert_eq!(
            delete_traced,
            Some((
                RuntimeProviderOperation::DeleteItem,
                RuntimeProviderOperationCounts::new(3, 3)
            ))
        );
    }

    #[tokio::test]
    async fn audit_append_helpers_are_runtime_owned() {
        let mut entity = account_entity();
        entity.facets.push("audited".to_string());
        let user = UserAuth::human(
            "tenant-1",
            "alex",
            "UTC",
            vec!["admin".to_string()],
            Vec::new(),
            "do-not-store",
        );

        let mut mutation_event = None;
        append_audit_mutation(
            &entity,
            AccessAction::Update,
            &user,
            Some("account-1".to_string()),
            Some(json!({ "id": "account-1", "name": "Old" })),
            Some(json!({ "id": "account-1", "name": "New" })),
            &PolicyAccess::allow_all(),
            |event| {
                mutation_event = Some(event);
                async { Ok::<_, RuntimeError>(()) }
            },
        )
        .await
        .expect("appended mutation event");

        let mutation_event = mutation_event.expect("mutation event");
        assert_eq!(mutation_event.outcome, "succeeded");
        assert_eq!(mutation_event.chain_scope, "crm.Account:tenant-1:account-1");

        let mut attempt_event = None;
        append_audit_attempt_on_record_chain(
            &entity,
            AccessAction::Update,
            Some(&user),
            "denied",
            Some("account-1".to_string()),
            Some(json!({ "id": "account-1", "name": "Old" })),
            Some(json!({ "id": "account-1", "name": "New" })),
            Some(record_audit::operation_decision_json("policy_denied")),
            |event| {
                attempt_event = Some(event);
                async { Ok::<_, RuntimeError>(()) }
            },
        )
        .await
        .expect("appended chained attempt event");

        let attempt_event = attempt_event.expect("attempt event");
        assert_eq!(attempt_event.outcome, "denied");
        assert_eq!(attempt_event.tenant_id.as_deref(), Some("tenant-1"));
        assert_eq!(attempt_event.chain_scope, "crm.Account:tenant-1:account-1");

        let mut denied_event = None;
        append_policy_denied_audit_attempt_on_record_chain(
            &entity,
            AccessAction::Update,
            &user,
            Some("account-1".to_string()),
            Some(json!({ "id": "account-1", "name": "Old" })),
            Some(json!({ "id": "account-1", "name": "Blocked" })),
            &PolicyAccess {
                allow: false,
                filter: Some(json!({ "tenant_id": { "_eq": "tenant-1" } })),
            },
            |event| {
                denied_event = Some(event);
                async { Ok::<_, RuntimeError>(()) }
            },
        )
        .await
        .expect("appended policy denied attempt event");

        let denied_event = denied_event.expect("denied event");
        assert_eq!(denied_event.outcome, "denied");
        assert_eq!(
            denied_event.policy_json.expect("policy json")["decision"]["reason"],
            json!("policy_denied")
        );

        let mut operation_failed_event = None;
        append_operation_failed_audit_attempt(
            &entity,
            AccessAction::Update,
            &user,
            Some("account-1".to_string()),
            Some(json!({ "id": "account-1", "name": "Old" })),
            Some(json!({ "id": "account-1", "name": "New" })),
            "stale record version",
            |event| {
                operation_failed_event = Some(event);
                async { Ok::<_, RuntimeError>(()) }
            },
        )
        .await
        .expect("appended failed mutation attempt event");

        let operation_failed_event = operation_failed_event.expect("failed mutation attempt event");
        assert_eq!(operation_failed_event.outcome, "failed");
        assert_eq!(
            operation_failed_event.policy_json.expect("policy json")["decision"]["reason"],
            json!("mutation_failed")
        );

        let mut not_applied_event = None;
        append_operation_not_applied_audit_attempt(
            &entity,
            AccessAction::Delete,
            &user,
            Some("account-1".to_string()),
            Some(json!({ "id": "account-1", "name": "Old" })),
            |event| {
                not_applied_event = Some(event);
                async { Ok::<_, RuntimeError>(()) }
            },
        )
        .await
        .expect("appended not-applied mutation attempt event");

        let not_applied_event = not_applied_event.expect("not-applied event");
        assert_eq!(not_applied_event.outcome, "not_applied");
        assert_eq!(
            not_applied_event.policy_json.expect("policy json")["decision"]["reason"],
            json!("no_rows_affected")
        );

        #[cfg(feature = "mcp")]
        {
            let mut mcp_event = None;
            append_mcp_tool_audit_event(
                &entity,
                &user,
                "succeeded",
                json!({
                    "tool": {
                        "name": "query_accounts",
                        "arguments_sha256": "sha256:abc"
                    }
                }),
                |event| {
                    mcp_event = Some(event);
                    async { Ok::<_, RuntimeError>(()) }
                },
            )
            .await
            .expect("appended MCP tool audit event");

            let mcp_event = mcp_event.expect("MCP event");
            assert_eq!(mcp_event.action, "mcp_tool");
            assert_eq!(mcp_event.outcome, "succeeded");
            assert_eq!(mcp_event.tenant_id.as_deref(), Some("tenant-1"));
            assert_eq!(
                mcp_event.policy_json.expect("MCP metadata")["tool"]["name"],
                json!("query_accounts")
            );
        }

        let mut missing_user_event = None;
        append_missing_user_audit_attempt(
            &entity,
            AccessAction::Read,
            Some("account-1".to_string()),
            "not authorized",
            |event| {
                missing_user_event = Some(event);
                async { Ok::<_, RuntimeError>(()) }
            },
        )
        .await
        .expect("appended missing user attempt event");

        let missing_user_event = missing_user_event.expect("missing user event");
        assert_eq!(missing_user_event.outcome, "denied");
        assert_eq!(
            missing_user_event.policy_json.expect("policy json")["decision"]["reason"],
            json!("missing_user")
        );

        let mut policy_error_event = None;
        append_policy_error_audit_attempt(
            &entity,
            AccessAction::Read,
            &user,
            Some("account-1".to_string()),
            Some(json!({ "id": "account-1" })),
            "rego evaluation failed",
            |event| {
                policy_error_event = Some(event);
                async { Ok::<_, RuntimeError>(()) }
            },
        )
        .await
        .expect("appended policy error attempt event");

        let policy_error_event = policy_error_event.expect("policy error event");
        assert_eq!(policy_error_event.outcome, "failed");
        assert_eq!(
            policy_error_event.policy_json.expect("policy json")["decision"]["reason"],
            json!("policy_evaluation_failed")
        );

        let mut append_called = false;
        append_audit_attempt(
            &account_entity(),
            AccessAction::Create,
            Some(&user),
            "failed",
            Some("account-2".to_string()),
            None,
            Some(json!({ "id": "account-2" })),
            Some(record_audit::operation_decision_json("mutation_failed")),
            |_event| {
                append_called = true;
                async { Ok::<_, RuntimeError>(()) }
            },
        )
        .await
        .expect("skipped non-audited append");
        assert!(!append_called);
    }

    #[test]
    fn mutation_helpers_are_runtime_owned() {
        let entity = account_entity();
        let input = json!({ "id": "input-id" })
            .as_object()
            .expect("object")
            .clone();
        let provider_result = json!({ "id": "provider-id" })
            .as_object()
            .expect("object")
            .clone();
        let before = json!({ "id": "before-id" })
            .as_object()
            .expect("object")
            .clone();

        assert_eq!(
            mutation_record_id(
                &entity,
                RuntimeMutationKind::Create,
                Some(&input),
                Some(&provider_result),
                None,
            ),
            Some("provider-id".to_string())
        );
        assert_eq!(
            mutation_record_id(
                &entity,
                RuntimeMutationKind::Update,
                Some(&input),
                Some(&provider_result),
                Some(&before),
            ),
            Some("input-id".to_string())
        );
        assert_eq!(
            mutation_record_id(
                &entity,
                RuntimeMutationKind::Delete,
                Some(&input),
                None,
                Some(&before),
            ),
            Some("input-id".to_string())
        );
        assert!(should_check_filtered_update_denial(
            Some(&json!({ "tenant_id": "tenant-1" })),
            Some("input-id"),
            None,
        ));
        assert!(!should_check_filtered_update_denial(
            None,
            Some("input-id"),
            None,
        ));
        assert_eq!(
            delete_audit_outcome(true, 1),
            Some(RuntimeDeleteAuditOutcome::Mutation)
        );
        assert_eq!(
            delete_audit_outcome(true, 0),
            Some(RuntimeDeleteAuditOutcome::NotApplied)
        );
        assert_eq!(delete_audit_outcome(false, 0), None);
        assert_eq!(
            RuntimeMutationKind::Delete.provider_operation_counts(3),
            RuntimeProviderOperationCounts {
                query_count: 3,
                result_count: 3,
            }
        );
    }

    fn account_entity() -> RuntimeEntityMetadata {
        RuntimeEntityMetadata {
            id: "account-id".to_string(),
            schema_name: "crm".to_string(),
            schema_id: None,
            pascal_1: "Account".to_string(),
            pascal_n: "Accounts".to_string(),
            snake_1: "account".to_string(),
            snake_n: "accounts".to_string(),
            caption_1: "Account".to_string(),
            caption_n: "Accounts".to_string(),
            is_union: false,
            base_type: None,
            is_table: true,
            facets: Vec::new(),
            meta: None,
            standard_methods: Vec::new(),
            custom_methods: Vec::new(),
            properties: vec![RuntimePropertyMetadata {
                id: "id-id".to_string(),
                name: "id".to_string(),
                caption: "Id".to_string(),
                data_type: RuntimeDataType::String,
                is_key: true,
                is_caption: false,
                is_required: true,
                is_read_only: false,
                is_concurrency_control: false,
                default_value: None,
                foreign_key: None,
                nav_by_fk: None,
                many_to_many: None,
                nested_entity_type: None,
                enum_type_name: None,
                meta: None,
            }],
        }
    }
}
