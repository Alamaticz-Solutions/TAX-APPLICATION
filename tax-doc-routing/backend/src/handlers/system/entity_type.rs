//
// Backend system EntityType Implementation
//    Product-owned handler extension file.
//    Generated once by app_gen, then preserved.
//
#[allow(unused_imports)]
pub(crate) use super::generated::entity_type::*;

use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;
use tracing::debug;

use crate::product_api::{DataAccess, EntityType, UserAuth};

#[allow(unused)]
pub async fn get_schema_types_impl(
    _user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    _entity_type: &Arc<EntityType>,
    _selections: Value,
    schema_name: String,
) -> Result<Vec<EntityType>> {
    debug!(schema = %schema_name, "loading schema entity types");
    let res = data_access
        .app_config
        .get_entity_types(&schema_name)
        .to_vec();
    Ok(res)
}
