//
// Backend tax_routing AppUser Implementation
//    Product-owned handler extension file.
//    Generated once by app_gen, then preserved.
//
#[allow(unused_imports)]
pub(crate) use super::generated::app_user::*;
#[allow(unused)]
use std::sync::Arc;

#[allow(unused_imports)]
use crate::{
    product_api::{DataAccess, EntityType, HandlerResult, JsonValue, UserAuth},
    schemas::common::AggregateResult,
    schemas::tax_routing::{AppUserProjection, AppUserQueryResult, InputAppUser},
};

/// The caller's own resolved grants (docs/architecture/rbac-design.md section 8). A user can
/// only ever read their own permissions this way: identity comes from the authenticated token,
/// never from an argument, so there is nothing to authorize beyond being signed in.
pub async fn my_permissions_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    _entity_type: &Arc<EntityType>,
    _selections: JsonValue,
) -> HandlerResult<serde_json::Value> {
    let user = user.ok_or_else(|| anyhow::anyhow!("not authenticated"))?;
    let principal = data_access
        .app_config
        .authz()
        .resolve(&user.tenant_id, &user.user_name);
    Ok(serde_json::to_value(principal)?)
}
