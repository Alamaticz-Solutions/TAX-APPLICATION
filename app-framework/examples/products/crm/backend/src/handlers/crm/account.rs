//
// Backend crm Account Implementation
//    Product-owned handler extension file.
//    Generated once by app_gen, then preserved.
//
#[allow(unused_imports)]
pub(crate) use super::generated::account::*;

use std::sync::Arc;

use appfw_runtime::record_locator::is_record_locator;
use serde_json::json;

use crate::{
    product_api::{DataAccess, EntityType, HandlerResult, JsonValue, UserAuth},
    schemas::crm::AccountProjection,
    services::account_health,
};

fn require_user(user: Option<UserAuth>) -> HandlerResult<UserAuth> {
    user.ok_or_else(|| anyhow::anyhow!("authentication is required for graph operations"))
}

pub async fn account_health_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    _selections: JsonValue,
    account_id: String,
) -> HandlerResult<JsonValue> {
    account_health::summarize_account_health(user, data_access, entity_type, account_id).await
}

pub async fn refresh_account_health_stored_procedure_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    selections: JsonValue,
    account_id: String,
    health_score: f64,
) -> HandlerResult<JsonValue> {
    let resolved_account_id =
        resolve_account_primary_key(user.clone(), data_access, entity_type, account_id).await?;
    super::generated::account::refresh_account_health_stored_procedure_impl(
        user,
        data_access,
        entity_type,
        selections,
        resolved_account_id,
        health_score,
    )
    .await
}

pub async fn account_relationship_graph_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    _selections: JsonValue,
    account: String,
) -> HandlerResult<JsonValue> {
    let user = require_user(user)?;
    Ok(data_access
        .account_relationship_graph(entity_type, &user, account)
        .await?)
}

pub async fn link_account_referral_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    _selections: JsonValue,
    from_account: String,
    to_account: String,
) -> HandlerResult<JsonValue> {
    let user = require_user(user)?;
    Ok(data_access
        .link_account_referral(entity_type, &user, from_account, to_account)
        .await?)
}

pub async fn unlink_account_referral_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    _selections: JsonValue,
    from_account: String,
    to_account: String,
) -> HandlerResult<JsonValue> {
    let user = require_user(user)?;
    Ok(data_access
        .unlink_account_referral(entity_type, &user, from_account, to_account)
        .await?)
}

async fn resolve_account_primary_key(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    account_ref: String,
) -> HandlerResult<String> {
    let selection = json!({
        "name": "account",
        "selection_set": [{ "name": "id", "selection_set": [] }]
    });
    let account = if is_record_locator(&account_ref) {
        data_access
            .find_item_by_locator::<AccountProjection>(
                entity_type.clone(),
                selection,
                account_ref.clone(),
                user,
            )
            .await?
    } else {
        data_access
            .find_item::<AccountProjection>(
                entity_type.clone(),
                selection,
                account_ref.clone(),
                user,
            )
            .await?
    };
    account
        .and_then(|account| account.id)
        .ok_or_else(|| anyhow::anyhow!("account `{account_ref}` was not found"))
}
