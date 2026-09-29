use serde::Serialize;
use serde_json::{json, Value};

use crate::{extension::UserAuth, observability::redact_diagnostic_text};

use super::protocol::{INTERNAL_ERROR, INVALID_PARAMS};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpError {
    pub code: i64,
    pub message: String,
}

impl McpError {
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(INVALID_PARAMS, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(INTERNAL_ERROR, message)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct McpAccessConfig {
    pub allowed_origins: Vec<String>,
    pub required_roles: Vec<String>,
    pub required_scopes: Vec<String>,
    pub privileged_tool_roles: Vec<String>,
    pub privileged_tool_scopes: Vec<String>,
}

pub trait McpAccessPolicy {
    fn allowed_origins(&self) -> &[String];
    fn required_roles(&self) -> &[String];
    fn required_scopes(&self) -> &[String];
    fn privileged_tool_roles(&self) -> &[String];
    fn privileged_tool_scopes(&self) -> &[String];
}

impl McpAccessPolicy for McpAccessConfig {
    fn allowed_origins(&self) -> &[String] {
        &self.allowed_origins
    }

    fn required_roles(&self) -> &[String] {
        &self.required_roles
    }

    fn required_scopes(&self) -> &[String] {
        &self.required_scopes
    }

    fn privileged_tool_roles(&self) -> &[String] {
        &self.privileged_tool_roles
    }

    fn privileged_tool_scopes(&self) -> &[String] {
        &self.privileged_tool_scopes
    }
}

#[cfg(all(feature = "http", feature = "mcp"))]
impl McpAccessPolicy for crate::security::SecurityConfig {
    fn allowed_origins(&self) -> &[String] {
        &self.mcp_allowed_origins
    }

    fn required_roles(&self) -> &[String] {
        &self.mcp_required_roles
    }

    fn required_scopes(&self) -> &[String] {
        &self.mcp_required_scopes
    }

    fn privileged_tool_roles(&self) -> &[String] {
        &self.mcp_privileged_tool_roles
    }

    fn privileged_tool_scopes(&self) -> &[String] {
        &self.mcp_privileged_tool_scopes
    }
}

pub fn user_can_access_mcp(security: &impl McpAccessPolicy, user: &UserAuth) -> bool {
    principal_matches(user, security.required_roles(), security.required_scopes())
}

pub fn user_can_use_privileged_tool(security: &impl McpAccessPolicy, user: &UserAuth) -> bool {
    principal_matches(
        user,
        security.privileged_tool_roles(),
        security.privileged_tool_scopes(),
    )
}

pub fn principal_matches(user: &UserAuth, roles: &[String], scopes: &[String]) -> bool {
    (roles.is_empty() && scopes.is_empty())
        || roles
            .iter()
            .any(|required| user.roles.iter().any(|role| role == required))
        || scopes
            .iter()
            .any(|required| user.scopes.iter().any(|scope| scope == required))
}

pub fn origin_allowed(
    origin: Option<&str>,
    security: &impl McpAccessPolicy,
    local_env: bool,
) -> bool {
    let Some(origin) = origin else {
        return true;
    };

    security
        .allowed_origins()
        .iter()
        .any(|allowed| allowed == origin)
        || (local_env && is_local_origin(origin))
}

pub fn is_local_origin(origin: &str) -> bool {
    origin.starts_with("http://localhost:")
        || origin.starts_with("http://127.0.0.1:")
        || origin.starts_with("http://[::1]:")
}

pub fn json_resource(uri: &str, value: impl Serialize) -> Result<Value, McpError> {
    let text = serde_json::to_string_pretty(&value)
        .map_err(|err| McpError::new(INTERNAL_ERROR, err.to_string()))?;
    Ok(json!({
        "uri": uri,
        "mimeType": "application/json",
        "text": text,
    }))
}

pub fn bounded_json(value: Value, max_bytes: usize, label: &str) -> Result<Value, McpError> {
    let text = serde_json::to_string(&value)
        .map_err(|err| McpError::new(INTERNAL_ERROR, err.to_string()))?;
    if text.len() > max_bytes {
        return Err(McpError::new(
            INVALID_PARAMS,
            format!("{label} exceeded configured byte limit ({max_bytes})"),
        ));
    }
    Ok(value)
}

pub fn tool_success(value: Value, max_result_bytes: usize) -> Result<Value, McpError> {
    let text = serde_json::to_string_pretty(&value)
        .map_err(|err| McpError::new(INTERNAL_ERROR, err.to_string()))?;
    if text.len() > max_result_bytes {
        return Err(McpError::new(
            INVALID_PARAMS,
            format!(
                "MCP tool result exceeded APP_MCP_MAX_RESULT_BYTES ({max_result_bytes}); narrow fields, filters, or pagination"
            ),
        ));
    }
    Ok(json!({
        "content": [{
            "type": "text",
            "text": text,
        }]
    }))
}

pub fn tool_error(message: impl Into<String>) -> Value {
    json!({
        "isError": true,
        "content": [{
            "type": "text",
            "text": redact_diagnostic_text(message.into()),
        }]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user() -> UserAuth {
        UserAuth::human(
            "",
            "tester",
            "UTC",
            vec!["developer".to_string()],
            vec!["appfw:mcp.read".to_string()],
            "",
        )
        .with_ingress("mcp")
    }

    fn access_config() -> McpAccessConfig {
        McpAccessConfig {
            allowed_origins: vec!["https://agent.example".to_string()],
            required_roles: vec!["admin".to_string()],
            required_scopes: vec!["appfw:mcp.read".to_string()],
            privileged_tool_roles: vec!["admin".to_string()],
            privileged_tool_scopes: vec!["appfw:mcp.admin".to_string()],
        }
    }

    #[test]
    fn access_accepts_role_or_scope() {
        let mut user = user();
        let roles = vec!["admin".to_string()];
        let scopes = vec!["appfw:mcp.read".to_string()];

        assert!(principal_matches(&user, &roles, &scopes));

        user.scopes.clear();
        assert!(!principal_matches(&user, &roles, &scopes));
    }

    #[test]
    fn origin_allows_configured_and_local_loopback() {
        let security = access_config();

        assert!(origin_allowed(
            Some("https://agent.example"),
            &security,
            false
        ));
        assert!(origin_allowed(
            Some("http://localhost:3000"),
            &security,
            true
        ));
        assert!(!origin_allowed(
            Some("https://untrusted.example"),
            &security,
            false
        ));
    }

    #[test]
    fn bounded_json_rejects_oversized_resources() {
        let err = bounded_json(json!({ "payload": "abcdef" }), 5, "test resource")
            .expect_err("resource should exceed limit");

        assert!(err.message.contains("test resource exceeded"));
    }

    #[test]
    fn error_constructors_shape_standard_codes() {
        assert_eq!(McpError::invalid_params("bad").code, INVALID_PARAMS);
        assert_eq!(McpError::internal("failed").code, INTERNAL_ERROR);
    }

    #[test]
    fn tool_success_rejects_oversized_results() {
        let err = tool_success(json!({ "payload": "abcdef" }), 5)
            .expect_err("tool result should exceed limit");

        assert_eq!(err.code, INVALID_PARAMS);
        assert!(err.message.contains("APP_MCP_MAX_RESULT_BYTES"));
    }
}
