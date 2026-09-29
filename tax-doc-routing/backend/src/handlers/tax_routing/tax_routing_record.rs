//
// Backend tax_routing TaxRoutingRecord Implementation
//    Product-owned handler extension file.
//    Generated once by app_gen, then preserved.
//
#[allow(unused_imports)]
pub(crate) use super::generated::tax_routing_record::*;
#[allow(unused)]
use std::sync::Arc;

#[allow(unused_imports)]
use crate::{
    product_api::{DataAccess, EntityType, HandlerResult, JsonValue, UserAuth},
    services::routing_record_actions,
};

/// Admin-only close (business spec 9.1). Delegates to services/routing_record_actions, the one
/// place both this and `reassign_record` share their "read current row, replace it" logic.
pub async fn cancel_record_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    _selections: JsonValue,
    record_id: String,
    resolution_status: String,
    comments: String,
) -> HandlerResult<serde_json::Value> {
    routing_record_actions::cancel_record(
        user,
        data_access,
        entity_type,
        record_id,
        resolution_status,
        comments,
    )
    .await
}

/// Admin-only reassignment (business spec 12.3).
pub async fn reassign_record_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    _selections: JsonValue,
    record_id: String,
    new_assigned_user_id: String,
) -> HandlerResult<serde_json::Value> {
    routing_record_actions::reassign_record(
        user,
        data_access,
        entity_type,
        record_id,
        new_assigned_user_id,
    )
    .await
}
