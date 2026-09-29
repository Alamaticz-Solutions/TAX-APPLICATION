use appfw_runtime::{ConfigError, RuntimeError};

pub const SNOWFLAKE_DEFAULT_TOKEN_TYPE: &str = "PROGRAMMATIC_ACCESS_TOKEN";
pub const SNOWFLAKE_DEFAULT_STATEMENT_TIMEOUT_SECS: u64 = 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnowflakeConnectionConfig {
    pub host: String,
    pub port: String,
    pub database: String,
    pub warehouse: Option<String>,
    pub role: Option<String>,
    pub auth_token: Option<String>,
    pub token_type: Option<String>,
    pub statement_timeout_secs: Option<String>,
}

impl SnowflakeConnectionConfig {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        host: impl Into<String>,
        port: impl Into<String>,
        database: impl Into<String>,
        warehouse: Option<String>,
        role: Option<String>,
        auth_token: Option<String>,
        token_type: Option<String>,
        statement_timeout_secs: Option<String>,
    ) -> Self {
        Self {
            host: host.into(),
            port: port.into(),
            database: database.into(),
            warehouse,
            role,
            auth_token,
            token_type,
            statement_timeout_secs,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnowflakeSessionConfig {
    pub endpoint: String,
    pub database: String,
    pub warehouse: Option<String>,
    pub role: Option<String>,
    pub token: String,
    pub token_type: String,
    pub statement_timeout_secs: u64,
}

pub fn snowflake_session_config(
    connection: &SnowflakeConnectionConfig,
) -> Result<SnowflakeSessionConfig, RuntimeError> {
    Ok(SnowflakeSessionConfig {
        endpoint: snowflake_endpoint(&connection.host, &connection.port),
        database: connection.database.clone(),
        warehouse: connection.warehouse.clone(),
        role: connection.role.clone(),
        token: snowflake_token(connection.auth_token.clone())?,
        token_type: connection
            .token_type
            .clone()
            .unwrap_or_else(|| SNOWFLAKE_DEFAULT_TOKEN_TYPE.to_string()),
        statement_timeout_secs: snowflake_statement_timeout_secs(
            connection.statement_timeout_secs.as_deref(),
        ),
    })
}

pub fn snowflake_endpoint(host: &str, port: &str) -> String {
    let host = host.trim().trim_end_matches('/');
    if host.starts_with("http://") || host.starts_with("https://") {
        if endpoint_has_explicit_port(host) || is_default_https_port(port) {
            host.to_string()
        } else {
            format!("{}:{}", host, port.trim())
        }
    } else if endpoint_has_explicit_port(host) || is_default_https_port(port) {
        format!("https://{}", host)
    } else {
        format!("https://{}:{}", host, port.trim())
    }
}

fn snowflake_token(token: Option<String>) -> Result<String, RuntimeError> {
    token.ok_or_else(|| {
        RuntimeError::Config(ConfigError::Load(
            "Snowflake requires a configured auth token secret".to_string(),
        ))
    })
}

fn snowflake_statement_timeout_secs(value: Option<&str>) -> u64 {
    value
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(SNOWFLAKE_DEFAULT_STATEMENT_TIMEOUT_SECS)
}

fn endpoint_has_explicit_port(host: &str) -> bool {
    let host = host
        .strip_prefix("http://")
        .or_else(|| host.strip_prefix("https://"))
        .unwrap_or(host);
    host.rsplit_once(':')
        .map(|(_, port)| !port.is_empty() && port.chars().all(|ch| ch.is_ascii_digit()))
        .unwrap_or(false)
}

fn is_default_https_port(port: &str) -> bool {
    let port = port.trim();
    port.is_empty() || port == "443"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_config_defaults_and_maps_endpoint() {
        let config = SnowflakeConnectionConfig::new(
            "account_identifier.snowflakecomputing.com",
            "443",
            "crm",
            None,
            None,
            Some("token".to_string()),
            None,
            None,
        );

        let session = snowflake_session_config(&config).expect("session config");

        assert_eq!(
            session.endpoint,
            "https://account_identifier.snowflakecomputing.com"
        );
        assert_eq!(session.database, "crm");
        assert_eq!(session.token, "token");
        assert_eq!(session.token_type, SNOWFLAKE_DEFAULT_TOKEN_TYPE);
        assert_eq!(
            session.statement_timeout_secs,
            SNOWFLAKE_DEFAULT_STATEMENT_TIMEOUT_SECS
        );
    }

    #[test]
    fn session_config_preserves_options_and_timeout() {
        let config = SnowflakeConnectionConfig::new(
            "snowflake.localhost.localstack.cloud",
            "4566",
            "crm",
            Some("WH".to_string()),
            Some("ROLE".to_string()),
            Some("token".to_string()),
            Some("CUSTOM_TOKEN".to_string()),
            Some("45".to_string()),
        );

        let session = snowflake_session_config(&config).expect("session config");

        assert_eq!(
            session.endpoint,
            "https://snowflake.localhost.localstack.cloud:4566"
        );
        assert_eq!(session.warehouse.as_deref(), Some("WH"));
        assert_eq!(session.role.as_deref(), Some("ROLE"));
        assert_eq!(session.token_type, "CUSTOM_TOKEN");
        assert_eq!(session.statement_timeout_secs, 45);
    }

    #[test]
    fn session_config_requires_token() {
        let config = SnowflakeConnectionConfig::new(
            "snowflake.local",
            "443",
            "crm",
            None,
            None,
            None,
            None,
            None,
        );

        let err = snowflake_session_config(&config).expect_err("missing token");

        assert!(
            matches!(err, RuntimeError::Config(ConfigError::Load(message)) if message.contains("Snowflake requires a configured auth token secret"))
        );
    }

    #[test]
    fn endpoint_preserves_explicit_scheme_and_port() {
        assert_eq!(
            snowflake_endpoint("http://snowflake.localhost.localstack.cloud:4566/", "443"),
            "http://snowflake.localhost.localstack.cloud:4566"
        );
    }

    #[test]
    fn invalid_statement_timeout_uses_default() {
        let config = SnowflakeConnectionConfig::new(
            "snowflake.local",
            "443",
            "crm",
            None,
            None,
            Some("token".to_string()),
            None,
            Some("bad".to_string()),
        );

        let session = snowflake_session_config(&config).expect("session config");

        assert_eq!(
            session.statement_timeout_secs,
            SNOWFLAKE_DEFAULT_STATEMENT_TIMEOUT_SECS
        );
    }
}
