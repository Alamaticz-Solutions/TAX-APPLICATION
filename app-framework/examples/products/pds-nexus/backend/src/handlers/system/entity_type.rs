//
// Backend system EntityType Implementation
//    Product-owned handler extension file.
//    Generated once by app_gen, then preserved.
//
#[allow(unused_imports)]
pub(crate) use super::generated::entity_type::*;
#[allow(unused)]
use std::sync::Arc;

#[allow(unused_imports)]
use crate::{
    product_api::{DataAccess, EntityType, HandlerResult, JsonValue, UserAuth},
    schemas::common::AggregateResult,
    schemas::system::{EntityTypeProjection, InputEntityType},
};

#[allow(unused)]
pub async fn get_schema_types_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    selections: JsonValue,
    schema_name: String,
) -> HandlerResult<Vec<EntityType>> {
    Err(anyhow::anyhow!(
        "custom method `get_schema_types` is not implemented yet"
    ))
}
