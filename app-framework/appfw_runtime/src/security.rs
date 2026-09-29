use std::env;

#[cfg(feature = "http")]
use std::{num::NonZeroU32, sync::Arc};

#[cfg(feature = "http")]
use axum::{
    extract::State,
    http::{header, HeaderMap, Request, StatusCode},
    middleware::Next,
    response::Response,
};
#[cfg(feature = "http")]
use governor::{DefaultKeyedRateLimiter, Quota, RateLimiter};
#[cfg(feature = "http")]
use sha2::{Digest, Sha256};

use crate::ConfigError;

#[derive(Clone, Debug)]
pub struct SecurityConfig {
    pub admin_ui_enabled: bool,
    pub admin_troubleshooting_enabled: bool,
    pub product_ui_enabled: bool,
    #[cfg(feature = "chat")]
    pub chat_enabled: bool,
    #[cfg(feature = "chat")]
    pub chat_required_roles: Vec<String>,
    #[cfg(feature = "chat")]
    pub chat_required_scopes: Vec<String>,
    #[cfg(feature = "chat")]
    pub chat_prompt_audit_enabled: bool,
    #[cfg(feature = "chat")]
    pub chat_prompt_audit_siem_enabled: bool,
    #[cfg(feature = "chat")]
    pub chat_prompt_audit_sink: Option<String>,
    #[cfg(feature = "chat")]
    pub chat_prompt_audit_retention_days: usize,
    #[cfg(feature = "chat")]
    pub chat_kill_switch_active: bool,
    #[cfg(feature = "mcp")]
    pub mcp_enabled: bool,
    #[cfg(feature = "mcp")]
    pub mcp_mutations_enabled: bool,
    #[cfg(feature = "mcp")]
    pub mcp_allowed_origins: Vec<String>,
    #[cfg(feature = "mcp")]
    pub mcp_max_result_bytes: usize,
    #[cfg(feature = "mcp")]
    pub mcp_max_resource_bytes: usize,
    #[cfg(feature = "mcp")]
    pub mcp_max_batch_items: usize,
    #[cfg(feature = "mcp")]
    pub mcp_required_roles: Vec<String>,
    #[cfg(feature = "mcp")]
    pub mcp_required_scopes: Vec<String>,
    #[cfg(feature = "mcp")]
    pub mcp_privileged_tool_roles: Vec<String>,
    #[cfg(feature = "mcp")]
    pub mcp_privileged_tool_scopes: Vec<String>,
    pub graphql_introspection_enabled: bool,
    pub graphql_introspection_required_roles: Vec<String>,
    pub graphql_introspection_required_scopes: Vec<String>,
    pub graphql_max_depth: usize,
    pub graphql_max_complexity: usize,
    pub request_body_limit_bytes: usize,
    pub rate_limit_per_second: u64,
    pub rate_limit_burst: u64,
}

impl SecurityConfig {
    pub fn from_env() -> Self {
        Self {
            admin_ui_enabled: env_bool("APP_ADMIN_UI_ENABLED", true),
            admin_troubleshooting_enabled: env_bool("APP_ADMIN_TROUBLESHOOTING_ENABLED", false),
            product_ui_enabled: env_bool("APP_PRODUCT_UI_ENABLED", true),
            #[cfg(feature = "chat")]
            chat_enabled: env_bool("APP_CHAT_ENABLED", false),
            #[cfg(feature = "chat")]
            chat_required_roles: env_csv_or("APP_CHAT_REQUIRED_ROLES", &["admin"]),
            #[cfg(feature = "chat")]
            chat_required_scopes: env_csv("APP_CHAT_REQUIRED_SCOPES"),
            #[cfg(feature = "chat")]
            chat_prompt_audit_enabled: env_bool("APP_CHAT_PROMPT_AUDIT_ENABLED", false),
            #[cfg(feature = "chat")]
            chat_prompt_audit_siem_enabled: env_bool("APP_CHAT_PROMPT_AUDIT_SIEM_ENABLED", false),
            #[cfg(feature = "chat")]
            chat_prompt_audit_sink: env_string("APP_CHAT_PROMPT_AUDIT_SINK"),
            #[cfg(feature = "chat")]
            chat_prompt_audit_retention_days: env_usize("APP_CHAT_PROMPT_AUDIT_RETENTION_DAYS", 90),
            #[cfg(feature = "chat")]
            chat_kill_switch_active: env_bool("APP_CHAT_KILL_SWITCH_ACTIVE", false),
            #[cfg(feature = "mcp")]
            mcp_enabled: env_bool("APP_MCP_ENABLED", false),
            #[cfg(feature = "mcp")]
            mcp_mutations_enabled: env_bool("APP_MCP_MUTATIONS_ENABLED", false),
            #[cfg(feature = "mcp")]
            mcp_allowed_origins: env_csv("APP_MCP_ALLOWED_ORIGINS"),
            #[cfg(feature = "mcp")]
            mcp_max_result_bytes: env_usize("APP_MCP_MAX_RESULT_BYTES", 256 * 1024),
            #[cfg(feature = "mcp")]
            mcp_max_resource_bytes: env_usize("APP_MCP_MAX_RESOURCE_BYTES", 256 * 1024),
            #[cfg(feature = "mcp")]
            mcp_max_batch_items: env_usize("APP_MCP_MAX_BATCH_ITEMS", 20),
            #[cfg(feature = "mcp")]
            mcp_required_roles: env_csv_or("APP_MCP_REQUIRED_ROLES", &["admin"]),
            #[cfg(feature = "mcp")]
            mcp_required_scopes: env_csv("APP_MCP_REQUIRED_SCOPES"),
            #[cfg(feature = "mcp")]
            mcp_privileged_tool_roles: env_csv_or("APP_MCP_PRIVILEGED_TOOL_ROLES", &["admin"]),
            #[cfg(feature = "mcp")]
            mcp_privileged_tool_scopes: env_csv_or(
                "APP_MCP_PRIVILEGED_TOOL_SCOPES",
                &["appfw:mcp.admin"],
            ),
            graphql_introspection_enabled: env_bool(
                "APP_GRAPHQL_INTROSPECTION_ENABLED",
                is_dev_workstation_env(),
            ),
            graphql_introspection_required_roles: env_csv_or(
                "APP_GRAPHQL_INTROSPECTION_REQUIRED_ROLES",
                &["admin"],
            ),
            graphql_introspection_required_scopes: env_csv_or(
                "APP_GRAPHQL_INTROSPECTION_REQUIRED_SCOPES",
                &[
                    "developer",
                    "appfw:developer",
                    "appfw:graphql.introspection",
                ],
            ),
            graphql_max_depth: env_usize("APP_GRAPHQL_MAX_DEPTH", 12),
            graphql_max_complexity: env_usize("APP_GRAPHQL_MAX_COMPLEXITY", 500),
            request_body_limit_bytes: env_usize("APP_REQUEST_BODY_LIMIT_BYTES", 1024 * 1024),
            rate_limit_per_second: env_u64("APP_RATE_LIMIT_PER_SECOND", 100),
            rate_limit_burst: env_u64("APP_RATE_LIMIT_BURST", 100),
        }
    }

    pub fn validate_runtime_safety(&self) -> Result<(), ConfigError> {
        ensure_local_only_flag("APP_ALLOW_MISSING_POLICIES_IN_LOCAL")?;
        ensure_local_only_flag("APP_BYPASS_POLICIES_IN_LOCAL")?;

        #[cfg(feature = "chat")]
        {
            if self.chat_enabled {
                ensure_chat_gate_configured(
                    "Chat access",
                    &self.chat_required_roles,
                    &self.chat_required_scopes,
                )?;
                if !self.chat_prompt_audit_enabled {
                    return Err(ConfigError::Load(
                        "APP_CHAT_PROMPT_AUDIT_ENABLED=true is required when APP_CHAT_ENABLED=true"
                            .to_string(),
                    ));
                }
                if !self.chat_prompt_audit_siem_enabled {
                    return Err(ConfigError::Load(
                        "APP_CHAT_PROMPT_AUDIT_SIEM_ENABLED=true is required when APP_CHAT_ENABLED=true"
                            .to_string(),
                    ));
                }
                if self.chat_prompt_audit_sink.is_none() {
                    return Err(ConfigError::Load(
                        "APP_CHAT_PROMPT_AUDIT_SINK must be configured when APP_CHAT_ENABLED=true"
                            .to_string(),
                    ));
                }
                if self.chat_prompt_audit_retention_days < 90 {
                    return Err(ConfigError::Load(
                        "APP_CHAT_PROMPT_AUDIT_RETENTION_DAYS must be at least 90 when APP_CHAT_ENABLED=true"
                            .to_string(),
                    ));
                }
                if self.chat_kill_switch_active {
                    return Err(ConfigError::Load(
                        "APP_CHAT_KILL_SWITCH_ACTIVE is set; chat runtime is disabled".to_string(),
                    ));
                }
            }
        }

        #[cfg(feature = "mcp")]
        {
            if self.mcp_mutations_enabled {
                return Err(ConfigError::Load(
                    "APP_MCP_MUTATIONS_ENABLED is not supported until MCP write-safety certification is implemented"
                        .to_string(),
                ));
            }

            if self.mcp_enabled {
                ensure_mcp_gate_configured(
                    "MCP access",
                    &self.mcp_required_roles,
                    &self.mcp_required_scopes,
                )?;
                ensure_mcp_gate_configured(
                    "MCP privileged tools",
                    &self.mcp_privileged_tool_roles,
                    &self.mcp_privileged_tool_scopes,
                )?;
            }
        }

        if self.graphql_introspection_enabled {
            ensure_security_gate_configured(
                "GraphQL introspection",
                &self.graphql_introspection_required_roles,
                &self.graphql_introspection_required_scopes,
            )?;
        }

        if env_bool("APP_ENABLE_LOCAL_TEST_AUTH", false) && !Self::local_test_auth_enabled() {
            return Err(ConfigError::Load(
                "APP_ENABLE_LOCAL_TEST_AUTH is only allowed for ENV_NAME=local or compose provider certification CI"
                    .to_string(),
            ));
        }

        Ok(())
    }

    pub fn allow_missing_policies_in_local() -> bool {
        is_dev_workstation_env() && env_bool("APP_ALLOW_MISSING_POLICIES_IN_LOCAL", false)
    }

    pub fn bypass_policies_in_local() -> bool {
        is_dev_workstation_env() && env_bool("APP_BYPASS_POLICIES_IN_LOCAL", false)
    }

    pub fn local_test_auth_enabled() -> bool {
        env_bool("APP_ENABLE_LOCAL_TEST_AUTH", false)
            && (is_dev_workstation_env() || is_provider_certification_ci_env())
    }
}

#[derive(Clone)]
#[cfg(feature = "http")]
pub struct RateLimiterState {
    limiter: Arc<DefaultKeyedRateLimiter<String>>,
}

#[cfg(feature = "http")]
impl RateLimiterState {
    pub fn new(limit_per_second: u64, burst: u64) -> Self {
        let per_second = non_zero_u32(limit_per_second);
        let burst = non_zero_u32(burst.max(limit_per_second));
        let quota = Quota::per_second(per_second).allow_burst(burst);
        let limiter: DefaultKeyedRateLimiter<String> = RateLimiter::keyed(quota);
        Self {
            limiter: Arc::new(limiter),
        }
    }

    fn allow_key(&self, key: &str) -> bool {
        self.limiter.check_key(&key.to_string()).is_ok()
    }
}

#[cfg(feature = "http")]
pub async fn rate_limit_hook<B>(
    State(state): State<RateLimiterState>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let key = rate_limit_key(&request);
    if state.allow_key(&key) {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::TOO_MANY_REQUESTS)
    }
}

#[cfg(feature = "http")]
fn rate_limit_key<B>(request: &Request<B>) -> String {
    let headers = request.headers();
    if let Some(tenant_id) = header_value(headers, "x-tenant-id") {
        return format!("tenant:{}", normalize_key_part(&tenant_id));
    }
    if let Some(auth) = header_value(headers, header::AUTHORIZATION.as_str()) {
        return format!("auth:{}", stable_hash(&auth));
    }
    if let Some(forwarded_for) = header_value(headers, "x-forwarded-for") {
        let client_ip = forwarded_for
            .split(',')
            .next()
            .unwrap_or(forwarded_for.as_str())
            .trim();
        if !client_ip.is_empty() {
            return format!("ip:{}", normalize_key_part(client_ip));
        }
    }
    if let Some(real_ip) = header_value(headers, "x-real-ip") {
        return format!("ip:{}", normalize_key_part(&real_ip));
    }
    format!("anonymous:{}", request.uri().path())
}

#[cfg(feature = "http")]
fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

#[cfg(feature = "http")]
fn stable_hash(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

#[cfg(feature = "http")]
fn normalize_key_part(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

#[cfg(feature = "http")]
fn non_zero_u32(value: u64) -> NonZeroU32 {
    let value = value.clamp(1, u32::MAX as u64) as u32;
    NonZeroU32::new(value).expect("value is clamped to be non-zero")
}

pub fn is_dev_workstation_env() -> bool {
    env::var("ENV_NAME").map(|v| v == "local").unwrap_or(false)
}

fn is_provider_certification_ci_env() -> bool {
    env::var("ENV_NAME")
        .map(|v| v == "compose")
        .unwrap_or(false)
        && env_bool("APP_PROVIDER_CERTIFICATION_CI", false)
}

fn ensure_local_only_flag(name: &str) -> Result<(), ConfigError> {
    if env_bool(name, false) && !is_dev_workstation_env() {
        Err(ConfigError::Load(format!(
            "{name} is only allowed when ENV_NAME=local"
        )))
    } else {
        Ok(())
    }
}

#[cfg(feature = "chat")]
fn ensure_chat_gate_configured(
    label: &str,
    roles: &[String],
    scopes: &[String],
) -> Result<(), ConfigError> {
    ensure_security_gate_configured(label, roles, scopes).map_err(|_| {
        ConfigError::Load(format!(
            "{label} must configure at least one required role or scope when APP_CHAT_ENABLED=true"
        ))
    })
}

#[cfg(feature = "mcp")]
fn ensure_mcp_gate_configured(
    label: &str,
    roles: &[String],
    scopes: &[String],
) -> Result<(), ConfigError> {
    ensure_security_gate_configured(label, roles, scopes).map_err(|_| {
        ConfigError::Load(format!(
            "{label} must configure at least one required role or scope when APP_MCP_ENABLED=true"
        ))
    })
}

fn ensure_security_gate_configured(
    label: &str,
    roles: &[String],
    scopes: &[String],
) -> Result<(), ConfigError> {
    if roles.is_empty() && scopes.is_empty() {
        Err(ConfigError::Load(format!(
            "{label} must configure at least one required role or scope"
        )))
    } else {
        Ok(())
    }
}

fn env_usize(name: &str, default: usize) -> usize {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn env_u64(name: &str, default: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

#[cfg(feature = "chat")]
fn env_string(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn env_bool(name: &str, default: bool) -> bool {
    env::var(name)
        .ok()
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}

#[cfg(any(feature = "chat", feature = "mcp"))]
fn env_csv(name: &str) -> Vec<String> {
    env::var(name)
        .ok()
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn env_csv_or(name: &str, default: &[&str]) -> Vec<String> {
    match env::var(name) {
        Ok(value) => value
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .collect(),
        Err(_) => default.iter().map(|value| (*value).to_string()).collect(),
    }
}

#[cfg(all(test, feature = "http"))]
mod tests {
    use super::*;
    use async_graphql::{EmptyMutation, EmptySubscription, Object, Schema};
    use axum::{
        body::Body,
        extract::DefaultBodyLimit,
        middleware,
        routing::{get, post},
        Router,
    };
    use tower::ServiceExt;

    static TEST_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    const SECURITY_ENV: [&str; 34] = [
        "ENV_NAME",
        "APP_ALLOW_MISSING_POLICIES_IN_LOCAL",
        "APP_BYPASS_POLICIES_IN_LOCAL",
        "APP_GRAPHQL_INTROSPECTION_ENABLED",
        "APP_GRAPHQL_INTROSPECTION_REQUIRED_ROLES",
        "APP_GRAPHQL_INTROSPECTION_REQUIRED_SCOPES",
        "APP_GRAPHQL_MAX_DEPTH",
        "APP_GRAPHQL_MAX_COMPLEXITY",
        "APP_REQUEST_BODY_LIMIT_BYTES",
        "APP_RATE_LIMIT_PER_SECOND",
        "APP_RATE_LIMIT_BURST",
        "APP_ADMIN_UI_ENABLED",
        "APP_ADMIN_TROUBLESHOOTING_ENABLED",
        "APP_PRODUCT_UI_ENABLED",
        "APP_CHAT_ENABLED",
        "APP_CHAT_REQUIRED_ROLES",
        "APP_CHAT_REQUIRED_SCOPES",
        "APP_CHAT_PROMPT_AUDIT_ENABLED",
        "APP_CHAT_PROMPT_AUDIT_SIEM_ENABLED",
        "APP_CHAT_PROMPT_AUDIT_SINK",
        "APP_CHAT_PROMPT_AUDIT_RETENTION_DAYS",
        "APP_CHAT_KILL_SWITCH_ACTIVE",
        "APP_MCP_ENABLED",
        "APP_MCP_MUTATIONS_ENABLED",
        "APP_MCP_ALLOWED_ORIGINS",
        "APP_MCP_MAX_RESULT_BYTES",
        "APP_MCP_MAX_RESOURCE_BYTES",
        "APP_MCP_MAX_BATCH_ITEMS",
        "APP_MCP_REQUIRED_ROLES",
        "APP_MCP_REQUIRED_SCOPES",
        "APP_MCP_PRIVILEGED_TOOL_ROLES",
        "APP_MCP_PRIVILEGED_TOOL_SCOPES",
        "APP_ENABLE_LOCAL_TEST_AUTH",
        "APP_PROVIDER_CERTIFICATION_CI",
    ];

    struct EnvSnapshot {
        values: Vec<(&'static str, Option<String>)>,
    }

    impl EnvSnapshot {
        fn capture(names: &[&'static str]) -> Self {
            Self {
                values: names
                    .iter()
                    .map(|name| (*name, env::var(name).ok()))
                    .collect(),
            }
        }
    }

    impl Drop for EnvSnapshot {
        fn drop(&mut self) {
            for (name, value) in &self.values {
                match value {
                    Some(value) => env::set_var(name, value),
                    None => env::remove_var(name),
                }
            }
        }
    }

    #[test]
    fn security_config_uses_secure_defaults() {
        let _env = TEST_ENV_LOCK.lock().expect("security env lock");
        let _snapshot = EnvSnapshot::capture(&SECURITY_ENV);
        for name in SECURITY_ENV {
            env::remove_var(name);
        }

        let config = SecurityConfig::from_env();

        assert!(config.admin_ui_enabled);
        assert!(!config.admin_troubleshooting_enabled);
        assert!(config.product_ui_enabled);
        #[cfg(feature = "chat")]
        {
            assert!(!config.chat_enabled);
            assert_eq!(config.chat_required_roles, vec!["admin"]);
            assert!(config.chat_required_scopes.is_empty());
            assert!(!config.chat_prompt_audit_enabled);
            assert!(!config.chat_prompt_audit_siem_enabled);
            assert!(config.chat_prompt_audit_sink.is_none());
            assert_eq!(config.chat_prompt_audit_retention_days, 90);
            assert!(!config.chat_kill_switch_active);
        }
        #[cfg(feature = "mcp")]
        {
            assert!(!config.mcp_enabled);
            assert!(!config.mcp_mutations_enabled);
            assert!(config.mcp_allowed_origins.is_empty());
            assert_eq!(config.mcp_max_result_bytes, 256 * 1024);
            assert_eq!(config.mcp_max_resource_bytes, 256 * 1024);
            assert_eq!(config.mcp_max_batch_items, 20);
            assert_eq!(config.mcp_required_roles, vec!["admin"]);
            assert!(config.mcp_required_scopes.is_empty());
            assert_eq!(config.mcp_privileged_tool_roles, vec!["admin"]);
            assert_eq!(config.mcp_privileged_tool_scopes, vec!["appfw:mcp.admin"]);
        }
        assert!(!config.graphql_introspection_enabled);
        assert_eq!(config.graphql_introspection_required_roles, vec!["admin"]);
        assert_eq!(
            config.graphql_introspection_required_scopes,
            vec![
                "developer",
                "appfw:developer",
                "appfw:graphql.introspection"
            ]
        );
        assert_eq!(config.graphql_max_depth, 12);
        assert_eq!(config.graphql_max_complexity, 500);
        assert_eq!(config.request_body_limit_bytes, 1024 * 1024);
        assert_eq!(config.rate_limit_per_second, 100);
        assert_eq!(config.rate_limit_burst, 100);
    }

    #[test]
    fn graphql_introspection_defaults_on_only_in_local_env() {
        let _env = TEST_ENV_LOCK.lock().expect("security env lock");
        let _snapshot = EnvSnapshot::capture(&SECURITY_ENV);
        for name in SECURITY_ENV {
            env::remove_var(name);
        }
        env::set_var("ENV_NAME", "local");

        let config = SecurityConfig::from_env();

        assert!(config.graphql_introspection_enabled);
    }

    #[test]
    fn graphql_introspection_gate_env_overrides_are_parsed() {
        let _env = TEST_ENV_LOCK.lock().expect("security env lock");
        let _snapshot = EnvSnapshot::capture(&SECURITY_ENV);
        for name in SECURITY_ENV {
            env::remove_var(name);
        }
        env::set_var("APP_GRAPHQL_INTROSPECTION_ENABLED", "true");
        env::set_var("APP_GRAPHQL_INTROSPECTION_REQUIRED_ROLES", "security_admin");
        env::set_var(
            "APP_GRAPHQL_INTROSPECTION_REQUIRED_SCOPES",
            "appfw:graphql.introspection",
        );

        let config = SecurityConfig::from_env();

        assert!(config.graphql_introspection_enabled);
        assert_eq!(
            config.graphql_introspection_required_roles,
            vec!["security_admin"]
        );
        assert_eq!(
            config.graphql_introspection_required_scopes,
            vec!["appfw:graphql.introspection"]
        );
    }

    #[test]
    fn graphql_introspection_enabled_requires_role_or_scope_gate() {
        let _env = TEST_ENV_LOCK.lock().expect("security env lock");
        let _snapshot = EnvSnapshot::capture(&SECURITY_ENV);
        for name in SECURITY_ENV {
            env::remove_var(name);
        }
        env::set_var("APP_GRAPHQL_INTROSPECTION_ENABLED", "true");
        env::set_var("APP_GRAPHQL_INTROSPECTION_REQUIRED_ROLES", " ");
        env::set_var("APP_GRAPHQL_INTROSPECTION_REQUIRED_SCOPES", "");

        let err = SecurityConfig::from_env()
            .validate_runtime_safety()
            .expect_err("empty introspection gate should fail validation");

        assert!(err
            .to_string()
            .contains("GraphQL introspection must configure"));
    }

    #[test]
    fn admin_ui_can_be_disabled_from_env() {
        let _env = TEST_ENV_LOCK.lock().expect("security env lock");
        let _snapshot = EnvSnapshot::capture(&["APP_ADMIN_UI_ENABLED"]);
        env::set_var("APP_ADMIN_UI_ENABLED", "false");

        let config = SecurityConfig::from_env();

        assert!(!config.admin_ui_enabled);
    }

    #[test]
    fn product_ui_can_be_disabled_from_env() {
        let _env = TEST_ENV_LOCK.lock().expect("security env lock");
        let _snapshot = EnvSnapshot::capture(&["APP_PRODUCT_UI_ENABLED"]);
        env::set_var("APP_PRODUCT_UI_ENABLED", "false");

        let config = SecurityConfig::from_env();

        assert!(!config.product_ui_enabled);
    }

    #[cfg(feature = "chat")]
    #[test]
    fn chat_rbac_env_overrides_are_parsed() {
        let _env = TEST_ENV_LOCK.lock().expect("security env lock");
        let _snapshot = EnvSnapshot::capture(&SECURITY_ENV);
        for name in SECURITY_ENV {
            env::remove_var(name);
        }
        env::set_var("APP_CHAT_REQUIRED_ROLES", "developer,platform_admin");
        env::set_var("APP_CHAT_REQUIRED_SCOPES", "appfw:chat.read");
        env::set_var("APP_CHAT_PROMPT_AUDIT_ENABLED", "true");
        env::set_var("APP_CHAT_PROMPT_AUDIT_SIEM_ENABLED", "true");
        env::set_var("APP_CHAT_PROMPT_AUDIT_SINK", "pds-siem");
        env::set_var("APP_CHAT_PROMPT_AUDIT_RETENTION_DAYS", "180");

        let config = SecurityConfig::from_env();

        assert_eq!(
            config.chat_required_roles,
            vec!["developer", "platform_admin"]
        );
        assert_eq!(config.chat_required_scopes, vec!["appfw:chat.read"]);
        assert!(config.chat_prompt_audit_enabled);
        assert!(config.chat_prompt_audit_siem_enabled);
        assert_eq!(config.chat_prompt_audit_sink.as_deref(), Some("pds-siem"));
        assert_eq!(config.chat_prompt_audit_retention_days, 180);
    }

    #[cfg(feature = "chat")]
    #[test]
    fn chat_enabled_requires_access_role_or_scope_gate() {
        let _env = TEST_ENV_LOCK.lock().expect("security env lock");
        let _snapshot = EnvSnapshot::capture(&SECURITY_ENV);
        for name in SECURITY_ENV {
            env::remove_var(name);
        }
        env::set_var("APP_CHAT_ENABLED", "true");
        env::set_var("APP_CHAT_REQUIRED_ROLES", "   ");
        env::set_var("APP_CHAT_PROMPT_AUDIT_ENABLED", "true");
        env::set_var("APP_CHAT_PROMPT_AUDIT_SIEM_ENABLED", "true");
        env::set_var("APP_CHAT_PROMPT_AUDIT_SINK", "pds-siem");

        let err = SecurityConfig::from_env()
            .validate_runtime_safety()
            .expect_err("empty chat access gate should fail validation");

        assert!(err.to_string().contains("Chat access must configure"));
    }

    #[cfg(feature = "chat")]
    #[test]
    fn chat_enabled_requires_prompt_audit_and_siem() {
        let _env = TEST_ENV_LOCK.lock().expect("security env lock");
        let _snapshot = EnvSnapshot::capture(&SECURITY_ENV);
        for name in SECURITY_ENV {
            env::remove_var(name);
        }
        env::set_var("APP_CHAT_ENABLED", "true");

        let err = SecurityConfig::from_env()
            .validate_runtime_safety()
            .expect_err("chat must require prompt audit");
        assert!(err
            .to_string()
            .contains("APP_CHAT_PROMPT_AUDIT_ENABLED=true"));

        env::set_var("APP_CHAT_PROMPT_AUDIT_ENABLED", "true");
        let err = SecurityConfig::from_env()
            .validate_runtime_safety()
            .expect_err("chat must require SIEM export posture");
        assert!(err
            .to_string()
            .contains("APP_CHAT_PROMPT_AUDIT_SIEM_ENABLED=true"));
    }

    #[cfg(feature = "chat")]
    #[test]
    fn chat_enabled_requires_prompt_audit_sink() {
        let _env = TEST_ENV_LOCK.lock().expect("security env lock");
        let _snapshot = EnvSnapshot::capture(&SECURITY_ENV);
        for name in SECURITY_ENV {
            env::remove_var(name);
        }
        env::set_var("APP_CHAT_ENABLED", "true");
        env::set_var("APP_CHAT_PROMPT_AUDIT_ENABLED", "true");
        env::set_var("APP_CHAT_PROMPT_AUDIT_SIEM_ENABLED", "true");
        env::set_var("APP_CHAT_PROMPT_AUDIT_SINK", " ");

        let err = SecurityConfig::from_env()
            .validate_runtime_safety()
            .expect_err("chat must require prompt audit sink");

        assert!(err
            .to_string()
            .contains("APP_CHAT_PROMPT_AUDIT_SINK must be configured"));
    }

    #[cfg(feature = "chat")]
    #[test]
    fn chat_enabled_enforces_prompt_retention_floor() {
        let _env = TEST_ENV_LOCK.lock().expect("security env lock");
        let _snapshot = EnvSnapshot::capture(&SECURITY_ENV);
        for name in SECURITY_ENV {
            env::remove_var(name);
        }
        env::set_var("APP_CHAT_ENABLED", "true");
        env::set_var("APP_CHAT_PROMPT_AUDIT_ENABLED", "true");
        env::set_var("APP_CHAT_PROMPT_AUDIT_SIEM_ENABLED", "true");
        env::set_var("APP_CHAT_PROMPT_AUDIT_SINK", "pds-siem");
        env::set_var("APP_CHAT_PROMPT_AUDIT_RETENTION_DAYS", "30");

        let err = SecurityConfig::from_env()
            .validate_runtime_safety()
            .expect_err("chat must enforce prompt retention floor");

        assert!(err
            .to_string()
            .contains("APP_CHAT_PROMPT_AUDIT_RETENTION_DAYS must be at least 90"));
    }

    #[cfg(feature = "chat")]
    #[test]
    fn chat_enabled_allows_scope_only_access_gate_with_audit_controls() {
        let _env = TEST_ENV_LOCK.lock().expect("security env lock");
        let _snapshot = EnvSnapshot::capture(&SECURITY_ENV);
        for name in SECURITY_ENV {
            env::remove_var(name);
        }
        env::set_var("APP_CHAT_ENABLED", "true");
        env::set_var("APP_CHAT_REQUIRED_ROLES", " ");
        env::set_var("APP_CHAT_REQUIRED_SCOPES", "appfw:chat.read");
        env::set_var("APP_CHAT_PROMPT_AUDIT_ENABLED", "true");
        env::set_var("APP_CHAT_PROMPT_AUDIT_SIEM_ENABLED", "true");
        env::set_var("APP_CHAT_PROMPT_AUDIT_SINK", "pds-siem");
        env::set_var("APP_CHAT_PROMPT_AUDIT_RETENTION_DAYS", "180");

        SecurityConfig::from_env()
            .validate_runtime_safety()
            .expect("scope-only chat access gate should validate with audit controls");
    }

    #[cfg(feature = "chat")]
    #[test]
    fn chat_kill_switch_blocks_enabled_chat_runtime() {
        let _env = TEST_ENV_LOCK.lock().expect("security env lock");
        let _snapshot = EnvSnapshot::capture(&SECURITY_ENV);
        for name in SECURITY_ENV {
            env::remove_var(name);
        }
        env::set_var("APP_CHAT_ENABLED", "true");
        env::set_var("APP_CHAT_PROMPT_AUDIT_ENABLED", "true");
        env::set_var("APP_CHAT_PROMPT_AUDIT_SIEM_ENABLED", "true");
        env::set_var("APP_CHAT_PROMPT_AUDIT_SINK", "pds-siem");
        env::set_var("APP_CHAT_KILL_SWITCH_ACTIVE", "true");

        let err = SecurityConfig::from_env()
            .validate_runtime_safety()
            .expect_err("chat kill switch should disable enabled chat runtime");

        assert!(err
            .to_string()
            .contains("APP_CHAT_KILL_SWITCH_ACTIVE is set"));
    }

    #[cfg(feature = "mcp")]
    #[test]
    fn mcp_mutations_are_rejected_until_certified() {
        let _env = TEST_ENV_LOCK.lock().expect("security env lock");
        let _snapshot = EnvSnapshot::capture(&SECURITY_ENV);
        for name in SECURITY_ENV {
            env::remove_var(name);
        }
        env::set_var("APP_MCP_MUTATIONS_ENABLED", "true");

        let err = SecurityConfig::from_env()
            .validate_runtime_safety()
            .expect_err("mutations should be blocked");

        assert!(err.to_string().contains("write-safety certification"));
    }

    #[cfg(feature = "mcp")]
    #[test]
    fn mcp_rbac_env_overrides_are_parsed() {
        let _env = TEST_ENV_LOCK.lock().expect("security env lock");
        let _snapshot = EnvSnapshot::capture(&SECURITY_ENV);
        for name in SECURITY_ENV {
            env::remove_var(name);
        }
        env::set_var("APP_MCP_REQUIRED_ROLES", "developer,platform_admin");
        env::set_var("APP_MCP_REQUIRED_SCOPES", "appfw:mcp.read");
        env::set_var("APP_MCP_PRIVILEGED_TOOL_ROLES", "security_admin");
        env::set_var(
            "APP_MCP_PRIVILEGED_TOOL_SCOPES",
            "appfw:mcp.admin,appfw:mcp.policy",
        );
        env::set_var("APP_MCP_MAX_RESOURCE_BYTES", "4096");
        env::set_var("APP_MCP_MAX_BATCH_ITEMS", "7");

        let config = SecurityConfig::from_env();

        assert_eq!(
            config.mcp_required_roles,
            vec!["developer", "platform_admin"]
        );
        assert_eq!(config.mcp_required_scopes, vec!["appfw:mcp.read"]);
        assert_eq!(config.mcp_privileged_tool_roles, vec!["security_admin"]);
        assert_eq!(
            config.mcp_privileged_tool_scopes,
            vec!["appfw:mcp.admin", "appfw:mcp.policy"]
        );
        assert_eq!(config.mcp_max_resource_bytes, 4096);
        assert_eq!(config.mcp_max_batch_items, 7);
    }

    #[cfg(feature = "mcp")]
    #[test]
    fn mcp_enabled_requires_access_role_or_scope_gate() {
        let _env = TEST_ENV_LOCK.lock().expect("security env lock");
        let _snapshot = EnvSnapshot::capture(&SECURITY_ENV);
        for name in SECURITY_ENV {
            env::remove_var(name);
        }
        env::set_var("APP_MCP_ENABLED", "true");
        env::set_var("APP_MCP_REQUIRED_ROLES", "   ");

        let err = SecurityConfig::from_env()
            .validate_runtime_safety()
            .expect_err("empty MCP access gate should fail validation");

        assert!(err.to_string().contains("MCP access must configure"));
    }

    #[cfg(feature = "mcp")]
    #[test]
    fn mcp_enabled_requires_privileged_tool_role_or_scope_gate() {
        let _env = TEST_ENV_LOCK.lock().expect("security env lock");
        let _snapshot = EnvSnapshot::capture(&SECURITY_ENV);
        for name in SECURITY_ENV {
            env::remove_var(name);
        }
        env::set_var("APP_MCP_ENABLED", "true");
        env::set_var("APP_MCP_PRIVILEGED_TOOL_ROLES", " ");
        env::set_var("APP_MCP_PRIVILEGED_TOOL_SCOPES", "");

        let err = SecurityConfig::from_env()
            .validate_runtime_safety()
            .expect_err("empty MCP privileged tool gate should fail validation");

        assert!(err
            .to_string()
            .contains("MCP privileged tools must configure"));
    }

    #[cfg(feature = "mcp")]
    #[test]
    fn mcp_enabled_allows_scope_only_access_gate() {
        let _env = TEST_ENV_LOCK.lock().expect("security env lock");
        let _snapshot = EnvSnapshot::capture(&SECURITY_ENV);
        for name in SECURITY_ENV {
            env::remove_var(name);
        }
        env::set_var("APP_MCP_ENABLED", "true");
        env::set_var("APP_MCP_REQUIRED_ROLES", " ");
        env::set_var("APP_MCP_REQUIRED_SCOPES", "appfw:mcp.read");

        SecurityConfig::from_env()
            .validate_runtime_safety()
            .expect("scope-only MCP access gate should validate");
    }

    #[test]
    fn admin_troubleshooting_must_be_enabled_explicitly() {
        let _env = TEST_ENV_LOCK.lock().expect("security env lock");
        let _snapshot = EnvSnapshot::capture(&["APP_ADMIN_TROUBLESHOOTING_ENABLED"]);
        env::set_var("APP_ADMIN_TROUBLESHOOTING_ENABLED", "true");

        let config = SecurityConfig::from_env();

        assert!(config.admin_troubleshooting_enabled);
    }

    #[test]
    fn request_limit_and_graphql_guard_env_overrides_are_parsed() {
        let _env = TEST_ENV_LOCK.lock().expect("security env lock");
        let _snapshot = EnvSnapshot::capture(&[
            "APP_GRAPHQL_MAX_DEPTH",
            "APP_GRAPHQL_MAX_COMPLEXITY",
            "APP_REQUEST_BODY_LIMIT_BYTES",
        ]);
        env::set_var("APP_GRAPHQL_MAX_DEPTH", "5");
        env::set_var("APP_GRAPHQL_MAX_COMPLEXITY", "42");
        env::set_var("APP_REQUEST_BODY_LIMIT_BYTES", "4096");

        let config = SecurityConfig::from_env();

        assert_eq!(config.graphql_max_depth, 5);
        assert_eq!(config.graphql_max_complexity, 42);
        assert_eq!(config.request_body_limit_bytes, 4096);
    }

    #[test]
    fn invalid_request_limit_values_fall_back_to_secure_defaults() {
        let _env = TEST_ENV_LOCK.lock().expect("security env lock");
        let _snapshot = EnvSnapshot::capture(&[
            "APP_GRAPHQL_MAX_DEPTH",
            "APP_GRAPHQL_MAX_COMPLEXITY",
            "APP_REQUEST_BODY_LIMIT_BYTES",
            "APP_RATE_LIMIT_PER_SECOND",
            "APP_RATE_LIMIT_BURST",
        ]);
        env::set_var("APP_GRAPHQL_MAX_DEPTH", "0");
        env::set_var("APP_GRAPHQL_MAX_COMPLEXITY", "not-a-number");
        env::set_var("APP_REQUEST_BODY_LIMIT_BYTES", "0");
        env::set_var("APP_RATE_LIMIT_PER_SECOND", "0");
        env::set_var("APP_RATE_LIMIT_BURST", "not-a-number");

        let config = SecurityConfig::from_env();

        assert_eq!(config.graphql_max_depth, 12);
        assert_eq!(config.graphql_max_complexity, 500);
        assert_eq!(config.request_body_limit_bytes, 1024 * 1024);
        assert_eq!(config.rate_limit_per_second, 100);
        assert_eq!(config.rate_limit_burst, 100);
    }

    #[test]
    fn unsafe_local_policy_bypass_fails_validation_outside_local_env() {
        let _env = TEST_ENV_LOCK.lock().expect("security env lock");
        let _snapshot = EnvSnapshot::capture(&SECURITY_ENV);
        for name in SECURITY_ENV {
            env::remove_var(name);
        }
        env::set_var("ENV_NAME", "compose");
        env::set_var("APP_BYPASS_POLICIES_IN_LOCAL", "true");

        let err = SecurityConfig::from_env()
            .validate_runtime_safety()
            .expect_err("policy bypass should fail outside local");

        assert!(err
            .to_string()
            .contains("APP_BYPASS_POLICIES_IN_LOCAL is only allowed"));
    }

    #[test]
    fn local_test_auth_fails_validation_outside_local_or_certification_ci() {
        let _env = TEST_ENV_LOCK.lock().expect("security env lock");
        let _snapshot = EnvSnapshot::capture(&SECURITY_ENV);
        for name in SECURITY_ENV {
            env::remove_var(name);
        }
        env::set_var("ENV_NAME", "compose");
        env::set_var("APP_ENABLE_LOCAL_TEST_AUTH", "true");

        let err = SecurityConfig::from_env()
            .validate_runtime_safety()
            .expect_err("local test auth should require certification CI outside local");

        assert!(err
            .to_string()
            .contains("APP_ENABLE_LOCAL_TEST_AUTH is only allowed"));
        assert!(!SecurityConfig::local_test_auth_enabled());
    }

    #[test]
    fn local_test_auth_is_allowed_for_compose_provider_certification_ci() {
        let _env = TEST_ENV_LOCK.lock().expect("security env lock");
        let _snapshot = EnvSnapshot::capture(&SECURITY_ENV);
        for name in SECURITY_ENV {
            env::remove_var(name);
        }
        env::set_var("ENV_NAME", "compose");
        env::set_var("APP_ENABLE_LOCAL_TEST_AUTH", "true");
        env::set_var("APP_PROVIDER_CERTIFICATION_CI", "true");

        SecurityConfig::from_env()
            .validate_runtime_safety()
            .expect("provider certification CI should allow explicit local test auth");
        assert!(SecurityConfig::local_test_auth_enabled());
    }

    #[test]
    fn provider_certification_ci_does_not_enable_local_test_auth_in_prod() {
        let _env = TEST_ENV_LOCK.lock().expect("security env lock");
        let _snapshot = EnvSnapshot::capture(&SECURITY_ENV);
        for name in SECURITY_ENV {
            env::remove_var(name);
        }
        env::set_var("ENV_NAME", "prod");
        env::set_var("APP_ENABLE_LOCAL_TEST_AUTH", "true");
        env::set_var("APP_PROVIDER_CERTIFICATION_CI", "true");

        let err = SecurityConfig::from_env()
            .validate_runtime_safety()
            .expect_err("provider certification CI flag must not allow local test auth in prod");

        assert!(err
            .to_string()
            .contains("APP_ENABLE_LOCAL_TEST_AUTH is only allowed"));
        assert!(!SecurityConfig::local_test_auth_enabled());
    }

    #[test]
    fn governor_limiter_is_keyed() {
        let limiter = RateLimiterState::new(1, 1);

        assert!(limiter.allow_key("tenant:a"));
        assert!(!limiter.allow_key("tenant:a"));
        assert!(limiter.allow_key("tenant:b"));
    }

    #[test]
    fn rate_limit_key_prefers_stable_identity_sources() {
        let request = Request::builder()
            .uri("/crm")
            .header("authorization", "Bearer abc")
            .body(())
            .expect("request");
        let key = rate_limit_key(&request);
        assert!(key.starts_with("auth:"));
        assert!(!key.contains("abc"));

        let request = Request::builder()
            .uri("/crm")
            .header("x-tenant-id", "Tenant-1")
            .header("authorization", "Bearer abc")
            .body(())
            .expect("request");
        assert_eq!(rate_limit_key(&request), "tenant:tenant-1");
    }

    #[tokio::test]
    async fn request_body_limit_rejects_oversized_graphql_payloads() {
        async fn consume_body(_body: String) -> &'static str {
            "ok"
        }

        let app = Router::new()
            .route("/crm", post(consume_body))
            .layer(DefaultBodyLimit::max(8));

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/crm")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"query":"{ value }"}"#))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn rate_limit_layer_rejects_repeated_requests_for_same_key() {
        async fn ok() -> &'static str {
            "ok"
        }

        let app = Router::new()
            .route("/crm", get(ok))
            .layer(middleware::from_fn_with_state(
                RateLimiterState::new(1, 1),
                rate_limit_hook,
            ));

        let first = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/crm")
                    .header("authorization", "Bearer rate-limit-test")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("first response");
        let second = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/crm")
                    .header("authorization", "Bearer rate-limit-test")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("second response");

        assert_eq!(first.status(), StatusCode::OK);
        assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    struct SecurityQuery;

    #[Object]
    impl SecurityQuery {
        async fn value(&self) -> i32 {
            1
        }

        async fn nested(&self) -> SecurityNested {
            SecurityNested
        }
    }

    struct SecurityNested;

    #[Object]
    impl SecurityNested {
        async fn value(&self) -> i32 {
            1
        }

        async fn nested(&self) -> SecurityNested {
            SecurityNested
        }
    }

    #[tokio::test]
    async fn graphql_depth_limit_rejects_deep_queries() {
        let schema = Schema::build(SecurityQuery, EmptyMutation, EmptySubscription)
            .limit_depth(1)
            .finish();

        let response = schema.execute("{ nested { value } }").await;

        assert!(!response.errors.is_empty());
    }

    #[tokio::test]
    async fn graphql_complexity_limit_rejects_expensive_queries() {
        let schema = Schema::build(SecurityQuery, EmptyMutation, EmptySubscription)
            .limit_complexity(1)
            .finish();

        let response = schema.execute("{ a: value b: value }").await;

        assert!(!response.errors.is_empty());
    }
}
