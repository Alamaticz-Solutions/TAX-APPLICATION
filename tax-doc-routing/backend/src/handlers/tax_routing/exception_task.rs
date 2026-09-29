//
// Backend tax_routing ExceptionTask Implementation
//    Product-owned handler extension file.
//    Generated once by app_gen, then preserved.
//
#[allow(unused_imports)]
pub(crate) use super::generated::exception_task::*;
#[allow(unused)]
use std::sync::Arc;

#[allow(unused_imports)]
use crate::{
    product_api::{DataAccess, EntityType, HandlerResult, JsonValue, UserAuth},
    schemas::common::AggregateResult,
    schemas::tax_routing::{ExceptionTaskProjection, ExceptionTaskQueryResult, InputExceptionTask},
    services::exception_task_actions,
};

/// Resolves the exception and resumes the parent record's processing (business spec 5.3).
/// Delegates to services/exception_task_actions, which enforces the "own" scope manually since
/// this is a custom mutation, not a row-filtered query (see routing_record_actions.rs for the
/// same pattern on cancel_record/reassign_record).
pub async fn retry_failed_step_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    _selections: JsonValue,
    exception_task_id: String,
) -> HandlerResult<serde_json::Value> {
    exception_task_actions::retry_failed_step(user, data_access, entity_type, exception_task_id)
        .await
}
