//
// Backend crm Lead Implementation
//    Product-owned handler extension file.
//    Generated once by app_gen, then preserved.
//
#[allow(unused_imports)]
pub(crate) use super::generated::lead::*;

use anyhow::Result;
use serde_json::Value;
#[allow(unused)]
use std::sync::Arc;

use crate::product_api::{DataAccess, EntityType, UserAuth};

pub async fn convert_lead_impl(
    _user: Option<UserAuth>,
    _data_access: &Arc<DataAccess>,
    _entity_type: &Arc<EntityType>,
    _selections: Value,
    _lead_id: String,
    _target_account_id: Option<String>,
) -> Result<Value> {
    Err(anyhow::anyhow!(
        "custom method `convert_lead` is not implemented yet"
    ))
}
