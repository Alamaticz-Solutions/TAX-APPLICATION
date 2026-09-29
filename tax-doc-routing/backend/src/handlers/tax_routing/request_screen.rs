//
// Backend tax_routing RequestScreen Implementation
//    Product-owned handler extension file.
//    Generated once by app_gen, then preserved.
//
#[allow(unused_imports)]
pub(crate) use super::generated::request_screen::*;
#[allow(unused)]
use std::sync::Arc;

#[allow(unused_imports)]
use crate::{
    product_api::{DataAccess, EntityType, HandlerResult, JsonValue, UserAuth},
    services::form_engine,
};

/// The dynamic form engine's condition evaluator (docs/architecture/dynamic-form-engine-design.md).
/// Delegates to services/form_engine, the one place a Condition/ConditionClause is evaluated.
pub async fn resolve_screen_fields_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    _entity_type: &Arc<EntityType>,
    _selections: JsonValue,
    screen_code: String,
    document_type_code: String,
) -> HandlerResult<serde_json::Value> {
    form_engine::resolve_screen_fields(user, data_access, screen_code, document_type_code).await
}
