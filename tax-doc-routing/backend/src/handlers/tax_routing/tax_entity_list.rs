//
// Backend tax_routing TaxEntityList Implementation
//    Product-owned handler extension file.
//    Generated once by app_gen, then preserved.
//
#[allow(unused_imports)]
pub(crate) use super::generated::tax_entity_list::*;
#[allow(unused)]
use std::sync::Arc;

#[allow(unused_imports)]
use crate::{
    product_api::{DataAccess, EntityType, HandlerResult, JsonValue, UserAuth},
    schemas::common::AggregateResult,
    schemas::tax_routing::{InputTaxEntityList, TaxEntityListProjection, TaxEntityListQueryResult},
};

#[allow(unused)]
pub async fn list_active_entities_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    selections: JsonValue,
    limit: i32,
    offset: i32,
) -> HandlerResult<serde_json::Value> {
    Err(anyhow::anyhow!(
        "custom method `list_active_entities` is not implemented yet"
    ))
}

#[allow(unused)]
pub async fn find_entity_by_oracle_id_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    selections: JsonValue,
    oracle_id: String,
) -> HandlerResult<serde_json::Value> {
    Err(anyhow::anyhow!(
        "custom method `find_entity_by_oracle_id` is not implemented yet"
    ))
}
