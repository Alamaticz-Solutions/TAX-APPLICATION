use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use appfw_runtime::{ConfigError, RuntimeError};
use serde::Deserialize;
use tokio::sync::Mutex;

pub use appfw_mssql_auth::FABRIC_SQL_ANALYTICS_DEFAULT_TOKEN_SCOPE;
pub const ENTRA_TOKEN_REFRESH_SKEW: Duration = Duration::from_secs(5 * 60);
const DEFAULT_ENTRA_TOKEN_EXPIRES_IN: u64 = 60 * 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MssqlAuthConfig {
    SqlPassword {
        username: Option<String>,
        password: Option<String>,
    },
    EntraAccessToken {
        access_token: String,
    },
    EntraClientCredentials {
        tenant_id: String,
        client_id: String,
        client_secret: String,
        token_scope: Option<String>,
    },
    /// On-prem Active Directory NTLM (domain Windows login + password via FreeTDS ODBC).
    Ntlm {
        username: Option<String>,
        password: Option<String>,
    },
}

#[derive(Clone)]
pub struct EntraTokenProvider {
    source: EntraTokenSource,
    cached: Arc<Mutex<Option<EntraAccessToken>>>,
    refresh_skew: Duration,
}

#[derive(Clone)]
enum EntraTokenSource {
    StaticAccessToken(String),
    ClientCredentials {
        tenant_id: String,
        client_id: String,
        client_secret: String,
        token_scope: String,
    },
}

#[derive(Clone)]
pub struct EntraAccessToken {
    access_token: String,
    expires_at: Instant,
}

impl EntraAccessToken {
    fn new(access_token: impl Into<String>, expires_at: Instant) -> Self {
        Self {
            access_token: access_token.into(),
            expires_at,
        }
    }

    pub fn secret(&self) -> &str {
        &self.access_token
    }

    pub fn into_secret(self) -> String {
        self.access_token
    }

    pub fn expires_at(&self) -> Instant {
        self.expires_at
    }

    pub fn expires_within(&self, duration: Duration) -> bool {
        self.expires_at <= Instant::now() + duration
    }
}

impl EntraTokenProvider {
    pub fn from_auth(auth: &MssqlAuthConfig) -> Result<Self, RuntimeError> {
        match auth {
            MssqlAuthConfig::EntraAccessToken { access_token } => {
                Ok(Self::static_access_token(access_token.clone()))
            }
            MssqlAuthConfig::EntraClientCredentials {
                tenant_id,
                client_id,
                client_secret,
                token_scope,
            } => Ok(Self::client_credentials(
                tenant_id.clone(),
                client_id.clone(),
                client_secret.clone(),
                token_scope
                    .clone()
                    .unwrap_or_else(|| FABRIC_SQL_ANALYTICS_DEFAULT_TOKEN_SCOPE.to_string()),
            )),
            MssqlAuthConfig::SqlPassword { .. } | MssqlAuthConfig::Ntlm { .. } => {
                Err(RuntimeError::DataAccess(
                "Microsoft Entra token provider requires Entra access-token or client-credentials auth"
                    .to_string(),
                ))
            }
        }
    }

    pub fn static_access_token(access_token: impl Into<String>) -> Self {
        Self {
            source: EntraTokenSource::StaticAccessToken(access_token.into()),
            cached: Arc::new(Mutex::new(None)),
            refresh_skew: ENTRA_TOKEN_REFRESH_SKEW,
        }
    }

    pub fn client_credentials(
        tenant_id: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        token_scope: impl Into<String>,
    ) -> Self {
        Self {
            source: EntraTokenSource::ClientCredentials {
                tenant_id: tenant_id.into(),
                client_id: client_id.into(),
                client_secret: client_secret.into(),
                token_scope: token_scope.into(),
            },
            cached: Arc::new(Mutex::new(None)),
            refresh_skew: ENTRA_TOKEN_REFRESH_SKEW,
        }
    }

    pub async fn access_token(&self) -> Result<String, RuntimeError> {
        match &self.source {
            EntraTokenSource::StaticAccessToken(access_token) => Ok(access_token.clone()),
            EntraTokenSource::ClientCredentials {
                tenant_id,
                client_id,
                client_secret,
                token_scope,
            } => {
                let mut cached = self.cached.lock().await;
                if let Some(token) = cached.as_ref() {
                    if !token.expires_within(self.refresh_skew) {
                        return Ok(token.secret().to_string());
                    }
                }

                let token =
                    fetch_entra_access_token(tenant_id, client_id, client_secret, token_scope)
                        .await?;
                let access_token = token.secret().to_string();
                *cached = Some(token);
                Ok(access_token)
            }
        }
    }
}

pub(crate) async fn fetch_entra_access_token(
    tenant_id: &str,
    client_id: &str,
    client_secret: &str,
    token_scope: &str,
) -> Result<EntraAccessToken, RuntimeError> {
    let http_client = reqwest::Client::builder().no_proxy().build().map_err(|e| {
        RuntimeError::DataAccess(format!("failed to configure Entra token HTTP client: {e}"))
    })?;
    fetch_entra_access_token_with_client(
        &http_client,
        tenant_id,
        client_id,
        client_secret,
        token_scope,
    )
    .await
}

async fn fetch_entra_access_token_with_client(
    http_client: &reqwest::Client,
    tenant_id: &str,
    client_id: &str,
    client_secret: &str,
    token_scope: &str,
) -> Result<EntraAccessToken, RuntimeError> {
    let token_scope = validated_entra_token_scope(token_scope)?;
    let token_url = format!("https://login.microsoftonline.com/{tenant_id}/oauth2/v2.0/token");
    let response = http_client
        .post(token_url)
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("grant_type", "client_credentials"),
            ("scope", token_scope),
        ])
        .send()
        .await
        .map_err(|e| RuntimeError::DataAccess(format!("failed to request Entra token: {e}")))?;

    let status = response.status();
    if !status.is_success() {
        return Err(RuntimeError::DataAccess(format!(
            "Entra token request failed with HTTP status {status}"
        )));
    }

    let received_at = Instant::now();
    let token = response
        .json::<TokenResponse>()
        .await
        .map_err(|e| RuntimeError::DataAccess(format!("invalid Entra token response: {e}")))?;
    let expires_in = token
        .expires_in
        .as_ref()
        .and_then(TokenExpiresIn::seconds)
        .unwrap_or(DEFAULT_ENTRA_TOKEN_EXPIRES_IN);
    Ok(EntraAccessToken::new(
        token.access_token,
        received_at + Duration::from_secs(expires_in),
    ))
}

fn validated_entra_token_scope(token_scope: &str) -> Result<&str, RuntimeError> {
    let token_value = token_scope.trim();
    if token_value.ends_with(".default") {
        Ok(token_value)
    } else {
        Err(RuntimeError::Config(ConfigError::Load(
            "FabricSqlAnalytics Entra client-credentials auth requires a v2 token scope ending with `.default`, for example `https://database.windows.net/.default`"
                .to_string(),
        )))
    }
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    expires_in: Option<TokenExpiresIn>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum TokenExpiresIn {
    Number(u64),
    String(String),
}

impl TokenExpiresIn {
    fn seconds(&self) -> Option<u64> {
        match self {
            TokenExpiresIn::Number(seconds) => Some(*seconds),
            TokenExpiresIn::String(seconds) => seconds.parse::<u64>().ok(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entra_token_scope_requires_v2_default_scope() {
        assert_eq!(
            validated_entra_token_scope(" https://database.windows.net/.default ").expect("scope"),
            "https://database.windows.net/.default"
        );

        let err = validated_entra_token_scope("https://database.windows.net/")
            .expect_err("resource is not a v2 scope");
        assert!(
            matches!(err, RuntimeError::Config(ConfigError::Load(message)) if message.contains("requires a v2 token scope"))
        );
    }

    #[test]
    fn token_expiry_accepts_numeric_and_string_values() {
        let numeric: TokenResponse =
            serde_json::from_str(r#"{"access_token":"token","expires_in":3599}"#)
                .expect("numeric expires_in");
        assert_eq!(
            numeric
                .expires_in
                .as_ref()
                .and_then(TokenExpiresIn::seconds),
            Some(3599)
        );

        let string: TokenResponse =
            serde_json::from_str(r#"{"access_token":"token","expires_in":"3598"}"#)
                .expect("string expires_in");
        assert_eq!(
            string.expires_in.as_ref().and_then(TokenExpiresIn::seconds),
            Some(3598)
        );
    }

    #[test]
    fn token_reports_expiring_within_refresh_skew() {
        let token = EntraAccessToken::new("token", Instant::now() + Duration::from_secs(60));
        assert!(token.expires_within(ENTRA_TOKEN_REFRESH_SKEW));

        let token = EntraAccessToken::new("token", Instant::now() + Duration::from_secs(600));
        assert!(!token.expires_within(ENTRA_TOKEN_REFRESH_SKEW));
    }
}
