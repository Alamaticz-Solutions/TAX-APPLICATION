//
// Backend tax_routing RequestScreenSubmission Implementation
//    Product-owned handler extension file.
//    Generated once by app_gen, then preserved.
//
#[allow(unused_imports)]
pub(crate) use super::generated::request_screen_submission::*;
#[allow(unused)]
use std::sync::Arc;

#[allow(unused_imports)]
use crate::{
    product_api::{DataAccess, EntityType, HandlerResult, JsonValue, UserAuth},
    schemas::common::AggregateResult,
    schemas::tax_routing::{
        InputRequestScreenSubmission, RequestScreenSubmissionProjection,
        RequestScreenSubmissionQueryResult,
    },
};

#[allow(unused)]
pub async fn save_screen_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    selections: JsonValue,
    routing_record_id: String,
    screen_code: String,
    payload: serde_json::Value,
) -> HandlerResult<serde_json::Value> {
    Err(anyhow::anyhow!(
        "custom method `save_screen` is not implemented yet"
    ))
}
