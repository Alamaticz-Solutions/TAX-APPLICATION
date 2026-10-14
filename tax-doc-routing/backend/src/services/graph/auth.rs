//! Env-var auth contract + client-credentials token acquisition for Microsoft
//! Graph. Credential *delivery* (putting the values in the environment) is a
//! platform precondition, not framework-generated.

use std::env;
use std::fmt;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize, Serializer};
use tokio::sync::RwLock;

/// A string that must never leak into a log line, error message, or
/// serialized payload. `Debug`/`Serialize` always emit `[REDACTED]`; the value
/// is reachable only through `expose_secret()`, called at the one place that
/// must send it (the token request below).
#[derive(Clone, Deserialize)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose_secret(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("\"[REDACTED]\"")
    }
}

impl Serialize for SecretString {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str("[REDACTED]")
    }
}

/// The complete Graph env-var contract. No other Graph env var is read.
pub const ENV_TENANT_ID: &str = "GRAPH_TENANT_ID";
pub const ENV_CLIENT_ID: &str = "GRAPH_CLIENT_ID";
pub const ENV_CLIENT_SECRET: &str = "GRAPH_CLIENT_SECRET";
/// A write-capable app registration distinct from the read one (same tenant,
/// only the client id/secret differ).
pub const ENV_WRITE_CLIENT_ID: &str = "GRAPH_WRITE_CLIENT_ID";
pub const ENV_WRITE_CLIENT_SECRET: &str = "GRAPH_WRITE_CLIENT_SECRET";

pub const SCOPE: &str = "https://graph.microsoft.com/.default";

#[derive(Clone, Debug)]
pub struct GraphAuthConfig {
    pub tenant_id: String,
    pub client_id: String,
    // `SecretString` redacts in Debug/Serialize so the secret cannot leak
    // through `{:?}` on this config or any struct that embeds it.
    client_secret: SecretString,
}

fn non_empty_env(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.is_empty())
}

impl GraphAuthConfig {
    /// Read-scoped credential. `None` when the provider is not configured.
    pub fn from_env() -> Option<Self> {
        Some(Self {
            tenant_id: non_empty_env(ENV_TENANT_ID)?,
            client_id: non_empty_env(ENV_CLIENT_ID)?,
            client_secret: SecretString::new(non_empty_env(ENV_CLIENT_SECRET)?),
        })
    }

    /// Write-scoped credential: same tenant, distinct client id/secret.
    /// `None` when either write variable is unset.
    pub fn from_write_env() -> Option<Self> {
        Some(Self {
            tenant_id: non_empty_env(ENV_TENANT_ID)?,
            client_id: non_empty_env(ENV_WRITE_CLIENT_ID)?,
            client_secret: SecretString::new(non_empty_env(ENV_WRITE_CLIENT_SECRET)?),
        })
    }

    /// Test-only constructor so other modules can build a config without
    /// touching process-global environment variables.
    #[cfg(test)]
    pub fn for_test(tenant_id: &str, client_id: &str, client_secret: &str) -> Self {
        Self {
            tenant_id: tenant_id.to_string(),
            client_id: client_id.to_string(),
            client_secret: SecretString::new(client_secret),
        }
    }

    fn token_url(&self) -> String {
        format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
            self.tenant_id
        )
    }
}

struct Cached {
    token: String,
    fetched_at: Instant,
    ttl: Duration,
}

/// Single-flight cached app-only token.
pub struct GraphToken {
    cfg: GraphAuthConfig,
    http: reqwest::Client,
    cache: RwLock<Option<Cached>>,
}

impl GraphToken {
    pub fn new(cfg: GraphAuthConfig, http: reqwest::Client) -> Self {
        Self {
            cfg,
            http,
            cache: RwLock::new(None),
        }
    }

    pub async fn bearer(&self) -> anyhow::Result<String> {
        if let Some(c) = self.cache.read().await.as_ref() {
            if c.fetched_at.elapsed() + Duration::from_secs(60) < c.ttl {
                return Ok(c.token.clone());
            }
        }
        let mut guard = self.cache.write().await;
        if let Some(c) = guard.as_ref() {
            if c.fetched_at.elapsed() + Duration::from_secs(60) < c.ttl {
                return Ok(c.token.clone());
            }
        }
        // The underlying reqwest error can echo the URL/params - never surface it.
        let resp = self
            .http
            .post(self.cfg.token_url())
            .form(&[
                ("client_id", self.cfg.client_id.as_str()),
                ("client_secret", self.cfg.client_secret.expose_secret()),
                ("scope", SCOPE),
                ("grant_type", "client_credentials"),
            ])
            .send()
            .await
            .map_err(|_| anyhow::anyhow!("Graph token request failed"))?;
        let status = resp.status();
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|_| anyhow::anyhow!("Graph token response was not JSON"))?;
        if !status.is_success() {
            let code = body
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            return Err(anyhow::anyhow!(
                "Graph token endpoint returned {status}: {code}"
            ));
        }
        let token = body
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Graph token response missing access_token"))?
            .to_string();
        let ttl_secs = body
            .get("expires_in")
            .and_then(|v| v.as_u64())
            .unwrap_or(3300);
        *guard = Some(Cached {
            token: token.clone(),
            fetched_at: Instant::now(),
            ttl: Duration::from_secs(ttl_secs),
        });
        Ok(token)
    }
}

/// `env::set_var` is process-global; every test in this crate that touches
/// the Graph/SharePoint environment must hold this one lock.
#[cfg(test)]
pub(crate) static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_string_never_debug_prints_the_value() {
        let secret = SecretString::new("super-secret-value");
        assert_eq!(format!("{secret:?}"), "\"[REDACTED]\"");
        assert_eq!(secret.expose_secret(), "super-secret-value");
    }

    #[test]
    fn secret_string_never_serializes_the_value() {
        let secret = SecretString::new("super-secret-value");
        assert_eq!(serde_json::to_string(&secret).unwrap(), "\"[REDACTED]\"");
    }

    #[test]
    fn config_debug_does_not_leak_the_client_secret() {
        let cfg = GraphAuthConfig::for_test("t", "c", "super-secret-value");
        assert!(!format!("{cfg:?}").contains("super-secret-value"));
    }

    #[test]
    fn read_and_write_contracts_are_independent() {
        let _guard = ENV_LOCK.lock().unwrap();
        for name in [
            ENV_TENANT_ID,
            ENV_CLIENT_ID,
            ENV_CLIENT_SECRET,
            ENV_WRITE_CLIENT_ID,
            ENV_WRITE_CLIENT_SECRET,
        ] {
            env::remove_var(name);
        }
        env::set_var(ENV_TENANT_ID, "tenant");
        env::set_var(ENV_CLIENT_ID, "read-id");
        env::set_var(ENV_CLIENT_SECRET, "read-secret");

        assert_eq!(GraphAuthConfig::from_env().unwrap().client_id, "read-id");
        assert!(GraphAuthConfig::from_write_env().is_none());

        env::set_var(ENV_WRITE_CLIENT_ID, "write-id");
        env::set_var(ENV_WRITE_CLIENT_SECRET, "write-secret");
        let write = GraphAuthConfig::from_write_env().unwrap();
        assert_eq!(write.client_id, "write-id");
        assert_eq!(write.tenant_id, "tenant");

        for name in [
            ENV_TENANT_ID,
            ENV_CLIENT_ID,
            ENV_CLIENT_SECRET,
            ENV_WRITE_CLIENT_ID,
            ENV_WRITE_CLIENT_SECRET,
        ] {
            env::remove_var(name);
        }
    }
}
