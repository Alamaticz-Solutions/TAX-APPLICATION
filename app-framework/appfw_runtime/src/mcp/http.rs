//! Runtime-owned HTTP shell for MCP endpoints.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde_json::Value;

use crate::{
    auth::RuntimeJwtExtractor, auth_state::RuntimeAuthState, extension::UserAuth,
    observability::RequestContext, security::SecurityConfig,
};

use super::{
    access::{origin_allowed, user_can_access_mcp},
    protocol::{error, INVALID_REQUEST},
    server::{handle_payload, McpJsonRpcResponse, McpJsonRpcStatus, McpMethodHandler},
};

pub const MCP_PATH: &str = "/mcp";

pub trait McpRuntimeState: McpMethodHandler + Clone + Send + Sync + 'static {
    fn auth_state(&self) -> RuntimeAuthState;
    fn security_config(&self) -> &SecurityConfig;

    fn local_env(&self) -> bool {
        std::env::var("ENV_NAME")
            .map(|value| value == "local")
            .unwrap_or(false)
    }
}

pub fn mcp_runtime_routes<S>() -> Router<S>
where
    S: McpRuntimeState,
{
    Router::new().route(MCP_PATH, post(mcp_post::<S>))
}

#[tracing::instrument(skip(state, headers, payload), fields(mcp = true))]
async fn mcp_post<S>(
    State(state): State<S>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Response
where
    S: McpRuntimeState,
{
    let request_context = RequestContext::from_headers(&headers);
    if let Err(response) = validate_mcp_origin(
        &headers,
        state.security_config(),
        state.local_env(),
        &request_context,
    ) {
        return response;
    }

    let user = match authenticate_mcp_user(state.auth_state(), headers, &request_context).await {
        Ok(user) => user,
        Err(response) => return response,
    };
    if let Err(response) = authorize_mcp_user(state.security_config(), &user, &request_context) {
        return response;
    }

    mcp_response(
        handle_payload(
            &state,
            &user,
            &request_context,
            payload,
            state.security_config().mcp_max_batch_items,
        )
        .await,
    )
}

pub fn mcp_response(response: McpJsonRpcResponse) -> Response {
    match (response.status, response.body) {
        (McpJsonRpcStatus::Ok, Some(body)) => (StatusCode::OK, Json(body)).into_response(),
        (McpJsonRpcStatus::Accepted, _) => StatusCode::ACCEPTED.into_response(),
        (McpJsonRpcStatus::BadRequest, Some(body)) => {
            (StatusCode::BAD_REQUEST, Json(body)).into_response()
        }
        (McpJsonRpcStatus::Ok | McpJsonRpcStatus::BadRequest, None) => {
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn authenticate_mcp_user(
    auth_state: RuntimeAuthState,
    headers: HeaderMap,
    request_context: &RequestContext,
) -> Result<UserAuth, Response> {
    let extractor = RuntimeJwtExtractor::new(auth_state, headers, false)
        .await
        .map_err(|err| {
            (
                StatusCode::UNAUTHORIZED,
                Json(error(
                    None,
                    INVALID_REQUEST,
                    err.to_string(),
                    request_context,
                )),
            )
                .into_response()
        })?;
    extractor
        .user
        .as_ref()
        .map(|user| (**user).clone().with_ingress("mcp"))
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(error(
                    None,
                    INVALID_REQUEST,
                    "MCP requests require an authenticated user",
                    request_context,
                )),
            )
                .into_response()
        })
}

#[allow(clippy::result_large_err)]
pub fn authorize_mcp_user(
    security: &SecurityConfig,
    user: &UserAuth,
    request_context: &RequestContext,
) -> Result<(), Response> {
    if user_can_access_mcp(security, user) {
        return Ok(());
    }

    Err((
        StatusCode::FORBIDDEN,
        Json(error(
            None,
            INVALID_REQUEST,
            "MCP access requires an allowed role or OAuth scope",
            request_context,
        )),
    )
        .into_response())
}

#[allow(clippy::result_large_err)]
pub fn validate_mcp_origin(
    headers: &HeaderMap,
    security: &SecurityConfig,
    local_env: bool,
    request_context: &RequestContext,
) -> Result<(), Response> {
    let origin = headers.get("origin").and_then(|value| value.to_str().ok());
    if origin_allowed(origin, security, local_env) {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            Json(error(
                None,
                INVALID_REQUEST,
                "MCP origin is not allowed by APP_MCP_ALLOWED_ORIGINS",
                request_context,
            )),
        )
            .into_response())
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use axum::{body::Body, http::Request};
    use serde_json::json;
    use tower::ServiceExt;

    use crate::{
        auth_config::JwtAuthConfig,
        mcp::{access::McpError, protocol::INVALID_PARAMS},
    };

    use super::*;

    #[derive(Clone)]
    struct StaticMcpState {
        security: SecurityConfig,
    }

    #[async_trait]
    impl McpMethodHandler for StaticMcpState {
        async fn handle_method(
            &self,
            _method: &str,
            _params: Option<Value>,
            _user: &UserAuth,
            _request_context: &RequestContext,
        ) -> Result<Value, McpError> {
            Ok(json!({ "ok": true }))
        }
    }

    impl McpRuntimeState for StaticMcpState {
        fn auth_state(&self) -> RuntimeAuthState {
            RuntimeAuthState::from(JwtAuthConfig {
                jwt_issuer: "https://tenant.okta.com/oauth2/default".to_string(),
                jwt_audience: "api://tenant".to_string(),
                okta_client_id: "client-id".to_string(),
            })
        }

        fn security_config(&self) -> &SecurityConfig {
            &self.security
        }
    }

    fn security_config() -> SecurityConfig {
        SecurityConfig {
            admin_ui_enabled: true,
            admin_troubleshooting_enabled: false,
            product_ui_enabled: true,
            #[cfg(feature = "chat")]
            chat_enabled: false,
            #[cfg(feature = "chat")]
            chat_required_roles: vec!["admin".to_string()],
            #[cfg(feature = "chat")]
            chat_required_scopes: Vec::new(),
            #[cfg(feature = "chat")]
            chat_prompt_audit_enabled: false,
            #[cfg(feature = "chat")]
            chat_prompt_audit_siem_enabled: false,
            #[cfg(feature = "chat")]
            chat_prompt_audit_sink: None,
            #[cfg(feature = "chat")]
            chat_prompt_audit_retention_days: 90,
            #[cfg(feature = "chat")]
            chat_kill_switch_active: false,
            mcp_enabled: true,
            mcp_mutations_enabled: false,
            mcp_allowed_origins: vec!["https://agent.example".to_string()],
            mcp_max_result_bytes: 256 * 1024,
            mcp_max_resource_bytes: 256 * 1024,
            mcp_max_batch_items: 20,
            mcp_required_roles: vec!["admin".to_string()],
            mcp_required_scopes: Vec::new(),
            mcp_privileged_tool_roles: vec!["admin".to_string()],
            mcp_privileged_tool_scopes: vec!["appfw:mcp.admin".to_string()],
            graphql_introspection_enabled: false,
            graphql_introspection_required_roles: vec!["admin".to_string()],
            graphql_introspection_required_scopes: vec![
                "developer".to_string(),
                "appfw:developer".to_string(),
                "appfw:graphql.introspection".to_string(),
            ],
            graphql_max_depth: 12,
            graphql_max_complexity: 500,
            request_body_limit_bytes: 1024 * 1024,
            rate_limit_per_second: 100,
            rate_limit_burst: 100,
        }
    }

    fn request_context() -> RequestContext {
        RequestContext {
            request_id: "request-1".to_string(),
            correlation_id: "correlation-1".to_string(),
        }
    }

    fn user_with_roles(roles: &[&str]) -> UserAuth {
        UserAuth::human(
            "tenant-1",
            "casey",
            "UTC",
            roles.iter().map(|role| (*role).to_string()).collect(),
            Vec::new(),
            "redacted",
        )
        .with_ingress("mcp")
    }

    #[test]
    fn mcp_response_maps_json_rpc_status_to_http_status() {
        assert_eq!(
            mcp_response(McpJsonRpcResponse::ok(json!({ "ok": true }))).status(),
            StatusCode::OK
        );
        assert_eq!(
            mcp_response(McpJsonRpcResponse::accepted()).status(),
            StatusCode::ACCEPTED
        );
        assert_eq!(
            mcp_response(McpJsonRpcResponse::bad_request(json!({
                "error": { "code": INVALID_PARAMS }
            })))
            .status(),
            StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn mcp_authorization_requires_allowed_role_or_scope() {
        let security = security_config();
        assert!(
            authorize_mcp_user(&security, &user_with_roles(&["admin"]), &request_context()).is_ok()
        );
        assert_eq!(
            authorize_mcp_user(
                &security,
                &user_with_roles(&["analyst"]),
                &request_context()
            )
            .expect_err("non-admin should be rejected")
            .status(),
            StatusCode::FORBIDDEN
        );
    }

    #[test]
    fn mcp_origin_validation_uses_configured_origins() {
        let security = security_config();
        let mut headers = HeaderMap::new();
        headers.insert("origin", "https://agent.example".parse().expect("origin"));
        assert!(validate_mcp_origin(&headers, &security, false, &request_context()).is_ok());

        headers.insert(
            "origin",
            "https://untrusted.example".parse().expect("origin"),
        );
        assert_eq!(
            validate_mcp_origin(&headers, &security, false, &request_context())
                .expect_err("origin should be rejected")
                .status(),
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn mcp_runtime_routes_require_authentication_before_dispatch() {
        let router = mcp_runtime_routes::<StaticMcpState>().with_state(StaticMcpState {
            security: security_config(),
        });

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(MCP_PATH)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "jsonrpc": "2.0",
                            "id": 1,
                            "method": "initialize"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("route should respond");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
