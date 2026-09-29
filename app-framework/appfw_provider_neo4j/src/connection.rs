use appfw_runtime::{
    connection_security::{self, ConnectionSecurity, Provider},
    RuntimeError,
};
use serde::{Deserialize, Serialize};

/// Connection parameters for a Neo4j graph-read data source.
///
/// `read_only` records the operator's intent that the configured credentials are
/// scoped to read-only roles. The provider always routes with read access mode
/// and rejects write clauses, but the credential scope itself is enforced by the
/// Neo4j server; this flag keeps that expectation explicit in configuration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Neo4jConnectionConfig {
    pub host: String,
    pub port: String,
    pub database: String,
    pub user: Option<String>,
    pub password: Option<String>,
    #[serde(default = "default_read_only")]
    pub read_only: bool,
}

fn default_read_only() -> bool {
    true
}

impl Neo4jConnectionConfig {
    pub fn new(
        host: impl Into<String>,
        port: impl Into<String>,
        database: impl Into<String>,
        user: Option<String>,
        password: Option<String>,
    ) -> Self {
        Self {
            host: host.into(),
            port: port.into(),
            database: database.into(),
            user,
            password,
            read_only: true,
        }
    }

    pub fn bolt_uri(&self) -> String {
        let host = self.host.trim();
        if host.starts_with("bolt://") || host.starts_with("neo4j://") {
            return host.to_string();
        }
        format!("bolt://{}:{}", host, self.port)
    }
}

/// Validate the connection-security policy for a Neo4j data source, mirroring
/// the relational providers' policy gate (managed environments must use TLS,
/// plaintext is local-dev only).
pub fn validate_neo4j_connection_security(
    env_name: &str,
    security_profile: &str,
    tls_mode: &str,
    host: &str,
) -> Result<ConnectionSecurity, RuntimeError> {
    connection_security::validate(Provider::Neo4j, env_name, security_profile, tls_mode, host)
        .map_err(|error| {
            RuntimeError::DataAccess(format!("invalid Neo4j connection security: {error}"))
        })
}

#[cfg(test)]
mod tests {
    use appfw_runtime::connection_security::TlsMode;

    use super::*;

    #[test]
    fn bolt_uri_defaults_to_configured_port() {
        let config = Neo4jConnectionConfig::new("localhost", "7687", "neo4j", None, None);
        assert_eq!(config.bolt_uri(), "bolt://localhost:7687");
        assert!(config.read_only);
    }

    #[test]
    fn bolt_uri_preserves_explicit_scheme() {
        let config =
            Neo4jConnectionConfig::new("neo4j://graph.internal:7687", "7687", "neo4j", None, None);
        assert_eq!(config.bolt_uri(), "neo4j://graph.internal:7687");
    }

    #[test]
    fn local_compose_security_allows_plaintext() {
        let security =
            validate_neo4j_connection_security("compose", "local_dev", "disabled", "neo4j")
                .expect("local compose security");
        assert_eq!(security.tls_mode, TlsMode::Disabled);
        assert!(!security.requires_tls());
    }

    #[test]
    fn managed_environment_requires_tls() {
        let err =
            validate_neo4j_connection_security("prod", "managed", "disabled", "graph.internal")
                .expect_err("managed plaintext should be rejected");
        assert!(matches!(err, RuntimeError::DataAccess(message) if message.contains("Neo4j")));
    }
}
