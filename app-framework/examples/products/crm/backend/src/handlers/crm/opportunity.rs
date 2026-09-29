//
// Backend crm Opportunity Implementation
//    Product-owned handler extension file.
//    Generated once by app_gen, then preserved.
//
#[allow(unused_imports)]
pub(crate) use super::generated::opportunity::*;

use anyhow::Result;
use serde_json::Value;
#[allow(unused)]
use std::sync::Arc;

use crate::product_api::{DataAccess, EntityType, UserAuth};

pub async fn pipeline_forecast_impl(
    _user: Option<UserAuth>,
    _data_access: &Arc<DataAccess>,
    _entity_type: &Arc<EntityType>,
    _selections: Value,
    _stage_id: Option<String>,
    _minimum_amount: Option<f64>,
) -> Result<Value> {
    Err(anyhow::anyhow!(
        "custom method `pipeline_forecast` is not implemented yet"
    ))
}
