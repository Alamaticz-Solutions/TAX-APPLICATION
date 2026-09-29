//! Stable product extension contracts.
//!
//! Types in this module are safe for product-owned handlers and services to
//! depend on. Keep them provider-neutral and independent from generated schema
//! modules.

use std::{fmt, sync::Arc};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(feature = "http")]
use crate::auth::RuntimeJwtExtractor;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimePrincipalType {
    #[default]
    User,
    Service,
    Agent,
}

#[derive(Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct UserAuth {
    pub tenant_id: String,
    pub user_name: String,
    pub timezone: String,
    #[serde(default)]
    pub principal_type: RuntimePrincipalType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_behalf_of: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ingress: Option<String>,
    pub roles: Vec<String>,
    pub scopes: Vec<String>,
    /// Raw bearer JWT. Never serialized (policy input, responses, logs) and
    /// redacted in `Debug` to prevent credential leakage. Policy evaluation
    /// relies on decoded claims (`tenant_id`, `user_name`, `roles`, `scopes`),
    /// not this value.
    #[serde(skip_serializing, default)]
    pub token: String,
}

impl fmt::Debug for UserAuth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UserAuth")
            .field("tenant_id", &self.tenant_id)
            .field("user_name", &self.user_name)
            .field("timezone", &self.timezone)
            .field("principal_type", &self.principal_type)
            .field("on_behalf_of", &self.on_behalf_of)
            .field("ingress", &self.ingress)
            .field("roles", &self.roles)
            .field("scopes", &self.scopes)
            .field("token", &"<redacted>")
            .finish()
    }
}

impl UserAuth {
    pub fn human(
        tenant_id: impl Into<String>,
        user_name: impl Into<String>,
        timezone: impl Into<String>,
        roles: Vec<String>,
        scopes: Vec<String>,
        token: impl Into<String>,
    ) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            user_name: user_name.into(),
            timezone: timezone.into(),
            principal_type: RuntimePrincipalType::User,
            on_behalf_of: None,
            ingress: Some("http".to_string()),
            roles,
            scopes,
            token: token.into(),
        }
    }

    pub fn service(
        tenant_id: impl Into<String>,
        subject: impl Into<String>,
        roles: Vec<String>,
        scopes: Vec<String>,
    ) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            user_name: subject.into(),
            timezone: "UTC".to_string(),
            principal_type: RuntimePrincipalType::Service,
            on_behalf_of: None,
            ingress: None,
            roles,
            scopes,
            token: String::new(),
        }
    }

    pub fn agent(
        tenant_id: impl Into<String>,
        subject: impl Into<String>,
        roles: Vec<String>,
        scopes: Vec<String>,
    ) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            user_name: subject.into(),
            timezone: "UTC".to_string(),
            principal_type: RuntimePrincipalType::Agent,
            on_behalf_of: None,
            ingress: None,
            roles,
            scopes,
            token: String::new(),
        }
    }

    pub fn with_ingress(mut self, ingress: impl Into<String>) -> Self {
        self.ingress = Some(ingress.into());
        self
    }

    pub fn with_on_behalf_of(mut self, subject: impl Into<String>) -> Self {
        self.on_behalf_of = Some(subject.into());
        self
    }

    pub fn effective_subject(&self) -> &str {
        self.on_behalf_of.as_deref().unwrap_or(&self.user_name)
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|candidate| candidate == role)
    }

    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|candidate| candidate == scope)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct Claims {
    pub iss: String,
    pub aud: String,
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub company: String,
    pub roles: Vec<String>,
    pub clock_id: String,
    pub cid: String,
    pub uid: String,
    pub scopes: Vec<String>,
    pub auth_time: usize,
    pub ver: usize,
    pub jti: String,
}

#[derive(Clone)]
pub struct RuntimeHandlerContext<D, E> {
    pub user: Option<UserAuth>,
    pub data_access: Arc<D>,
    pub entity_type: Arc<E>,
    pub selections: Value,
}

impl<D, E> RuntimeHandlerContext<D, E> {
    pub fn new(
        user: Option<UserAuth>,
        data_access: Arc<D>,
        entity_type: Arc<E>,
        selections: Value,
    ) -> Self {
        Self {
            user,
            data_access,
            entity_type,
            selections,
        }
    }

    pub fn into_handler_parts(self) -> (Option<UserAuth>, Arc<D>, Arc<E>, Value) {
        (
            self.user,
            self.data_access,
            self.entity_type,
            self.selections,
        )
    }
}

#[cfg(feature = "http")]
pub fn user_from_graphql_context(ctx: &async_graphql::Context<'_>) -> Option<UserAuth> {
    ctx.data_opt::<RuntimeJwtExtractor>()
        .and_then(|jwt_extractor| jwt_extractor.user.as_ref().map(|user| (**user).clone()))
}

pub fn data_from_graphql_context<D>(ctx: &async_graphql::Context<'_>) -> Arc<D>
where
    D: Send + Sync + 'static,
{
    ctx.data_unchecked::<Arc<D>>().clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_auth_matches_roles_and_scopes_exactly() {
        let user = UserAuth::human(
            "tenant-1",
            "casey",
            "UTC",
            vec!["admin".to_string(), "analyst".to_string()],
            vec!["appfw:mcp.read".to_string()],
            "token",
        );

        assert!(user.has_role("admin"));
        assert!(!user.has_role("adm"));
        assert!(user.has_scope("appfw:mcp.read"));
        assert!(!user.has_scope("appfw:mcp"));
    }

    #[test]
    fn user_auth_never_leaks_raw_token() {
        const SECRET_JWT: &str = "header.payload.signature-super-secret-jwt";

        let user = UserAuth::human(
            "tenant-1",
            "casey",
            "UTC",
            vec!["admin".to_string()],
            vec!["appfw:mcp.read".to_string()],
            SECRET_JWT,
        );

        // Serialization (policy input, responses, logs) must never contain the raw JWT.
        let serialized = serde_json::to_string(&user).expect("user serializes");
        assert!(
            !serialized.contains(SECRET_JWT),
            "serialized UserAuth leaked the raw JWT: {serialized}"
        );
        assert!(
            !serialized.contains("token"),
            "serialized UserAuth still emits a token field: {serialized}"
        );
        // Decoded claims the Rego policy relies on must still be present.
        assert!(serialized.contains("tenant-1"));
        assert!(serialized.contains("casey"));
        assert!(serialized.contains("admin"));

        // Debug output must redact the token while keeping other fields readable.
        let debug = format!("{user:?}");
        assert!(
            !debug.contains(SECRET_JWT),
            "Debug output leaked the raw JWT: {debug}"
        );
        assert!(
            debug.contains("<redacted>"),
            "Debug should redact token: {debug}"
        );
        assert!(
            debug.contains("casey"),
            "Debug should keep readable fields: {debug}"
        );
    }

    #[test]
    fn user_auth_serializes_principal_envelope_for_policy_input() {
        let service = UserAuth::service(
            "tenant-1",
            "crm-event-consumer",
            vec!["integration_writer".to_string()],
            vec!["crm.account.write".to_string()],
        )
        .with_ingress("kafka")
        .with_on_behalf_of("casey");

        let serialized = serde_json::to_value(&service).expect("service user serializes");
        assert_eq!(serialized["principal_type"], "service");
        assert_eq!(serialized["ingress"], "kafka");
        assert_eq!(serialized["on_behalf_of"], "casey");
        assert_eq!(service.effective_subject(), "casey");
    }

    #[test]
    fn handler_context_round_trips_product_bound_parts() {
        let user = UserAuth::human(
            "tenant-1",
            "casey",
            "UTC",
            vec!["admin".to_string()],
            vec![],
            "token",
        );

        let context = RuntimeHandlerContext::new(
            Some(user.clone()),
            Arc::new("data-access"),
            Arc::new("entity"),
            serde_json::json!({ "selection_set": [] }),
        );
        let (actual_user, data_access, entity, selections) = context.into_handler_parts();

        assert_eq!(actual_user, Some(user));
        assert_eq!(*data_access, "data-access");
        assert_eq!(*entity, "entity");
        assert_eq!(selections, serde_json::json!({ "selection_set": [] }));
    }

    #[cfg(feature = "http")]
    #[tokio::test]
    async fn graphql_context_helpers_extract_user_and_product_data() {
        use async_graphql::{Context, EmptyMutation, EmptySubscription, Object, Schema};

        struct ContextQuery;

        #[Object]
        impl ContextQuery {
            async fn user_name(&self, ctx: &Context<'_>) -> String {
                user_from_graphql_context(ctx)
                    .map(|user| user.user_name)
                    .unwrap_or_default()
            }

            async fn data_value(&self, ctx: &Context<'_>) -> String {
                data_from_graphql_context::<String>(ctx).as_ref().clone()
            }
        }

        let user = UserAuth::human(
            "tenant-1",
            "casey",
            "UTC",
            vec!["admin".to_string()],
            vec![],
            "token",
        );
        let schema = Schema::build(ContextQuery, EmptyMutation, EmptySubscription)
            .data(RuntimeJwtExtractor {
                user: Some(Arc::new(user)),
            })
            .data(Arc::new("product-state".to_string()))
            .finish();

        let response = schema.execute("{ userName dataValue }").await;

        assert!(response.errors.is_empty(), "{:?}", response.errors);
        assert_eq!(
            response.data.into_json().expect("response json"),
            serde_json::json!({ "userName": "casey", "dataValue": "product-state" })
        );
    }
}
