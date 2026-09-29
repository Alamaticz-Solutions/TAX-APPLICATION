//
// Backend tax_routing TaxPayerRefData Implementation
//    Product-owned handler extension file.
//    Generated once by app_gen, then preserved.
//
#[allow(unused_imports)]
pub(crate) use super::generated::tax_payer_ref_data::*;
#[allow(unused)]
use std::sync::Arc;

#[allow(unused_imports)]
use crate::{
    product_api::{DataAccess, EntityType, HandlerResult, JsonValue, UserAuth},
    schemas::common::AggregateResult,
    schemas::tax_routing::{
        InputTaxPayerRefData, TaxPayerRefDataProjection, TaxPayerRefDataQueryResult,
    },
};

#[allow(unused)]
pub async fn list_active_taxpayers_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    selections: JsonValue,
    search: Option<String>,
    limit: i32,
    offset: i32,
) -> HandlerResult<serde_json::Value> {
    Err(anyhow::anyhow!(
        "custom method `list_active_taxpayers` is not implemented yet"
    ))
}

#[allow(unused)]
pub async fn update_taxpayer_reference_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    selections: JsonValue,
    taxpayer_id: String,
    external_folder_id: Option<String>,
    internal_folder_path: Option<String>,
    read_write_password: Option<String>,
) -> HandlerResult<serde_json::Value> {
    Err(anyhow::anyhow!(
        "custom method `update_taxpayer_reference` is not implemented yet"
    ))
}
