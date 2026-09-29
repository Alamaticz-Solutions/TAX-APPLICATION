//
// Backend crm Quote Implementation
//    Product-owned handler extension file.
//    Generated once by app_gen, then preserved.
//
#[allow(unused_imports)]
pub(crate) use super::generated::quote::*;

use anyhow::Result;
use serde_json::Value;
#[allow(unused)]
use std::sync::Arc;

use crate::product_api::{DataAccess, EntityType, UserAuth};

pub async fn reprice_quote_impl(
    _user: Option<UserAuth>,
    _data_access: &Arc<DataAccess>,
    _entity_type: &Arc<EntityType>,
    _selections: Value,
    _quote_id: String,
    _include_tax: bool,
) -> Result<Value> {
    Err(anyhow::anyhow!(
        "custom method `reprice_quote` is not implemented yet"
    ))
}
