use std::{
    collections::BTreeMap,
    fmt,
    sync::Mutex,
    time::{Duration, SystemTime},
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose_secret(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("\"[REDACTED]\"")
    }
}

impl Serialize for SecretString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str("[REDACTED]")
    }
}

impl<'de> Deserialize<'de> for SecretString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer).map(Self)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct M2mTokenCacheKey {
    pub provider_key: String,
    pub data_source_name: String,
    pub tenant_key: Option<String>,
    pub client_id: String,
    #[serde(default)]
    pub scopes: Vec<String>,
}

impl M2mTokenCacheKey {
    pub fn new(
        provider_key: impl Into<String>,
        data_source_name: impl Into<String>,
        client_id: impl Into<String>,
    ) -> Self {
        Self {
            provider_key: provider_key.into(),
            data_source_name: data_source_name.into(),
            tenant_key: None,
            client_id: client_id.into(),
            scopes: Vec::new(),
        }
    }

    pub fn with_tenant_key(mut self, tenant_key: impl Into<String>) -> Self {
        self.tenant_key = Some(tenant_key.into());
        self
    }

    pub fn with_scopes<I, S>(mut self, scopes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.scopes = scopes.into_iter().map(Into::into).collect();
        self.scopes.sort();
        self.scopes.dedup();
        self
    }
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct M2mAccessToken {
    pub access_token: SecretString,
    pub token_type: String,
    pub expires_at: SystemTime,
    #[serde(default)]
    pub scopes: Vec<String>,
}

impl M2mAccessToken {
    pub fn bearer(
        access_token: impl Into<String>,
        expires_at: SystemTime,
        scopes: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            access_token: SecretString::new(access_token),
            token_type: "Bearer".to_string(),
            expires_at,
            scopes: scopes.into_iter().map(Into::into).collect(),
        }
    }

    pub fn refresh_decision(
        &self,
        now: SystemTime,
        refresh_skew: Duration,
    ) -> TokenRefreshDecision {
        TokenRefreshDecision::from_expiration(self.expires_at, now, refresh_skew)
    }

    pub fn is_fresh(&self, now: SystemTime, refresh_skew: Duration) -> bool {
        matches!(
            self.refresh_decision(now, refresh_skew),
            TokenRefreshDecision::Fresh
        )
    }
}

impl fmt::Debug for M2mAccessToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M2mAccessToken")
            .field("access_token", &"[REDACTED]")
            .field("token_type", &self.token_type)
            .field("expires_at", &self.expires_at)
            .field("scopes", &self.scopes)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokenRefreshDecision {
    Fresh,
    RefreshWithinSkew,
    Expired,
}

impl TokenRefreshDecision {
    pub fn from_expiration(
        expires_at: SystemTime,
        now: SystemTime,
        refresh_skew: Duration,
    ) -> Self {
        if now >= expires_at {
            return Self::Expired;
        }

        match expires_at.duration_since(now) {
            Ok(time_to_live) if time_to_live > refresh_skew => Self::Fresh,
            Ok(_) => Self::RefreshWithinSkew,
            Err(_) => Self::Expired,
        }
    }

    pub fn should_refresh(self) -> bool {
        !matches!(self, Self::Fresh)
    }
}

#[derive(Debug, Default)]
pub struct M2mTokenCache {
    entries: Mutex<BTreeMap<M2mTokenCacheKey, M2mAccessToken>>,
}

impl M2mTokenCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, key: M2mTokenCacheKey, token: M2mAccessToken) -> Option<M2mAccessToken> {
        self.entries
            .lock()
            .expect("m2m token cache mutex poisoned")
            .insert(key, token)
    }

    pub fn get(&self, key: &M2mTokenCacheKey) -> Option<M2mAccessToken> {
        self.entries
            .lock()
            .expect("m2m token cache mutex poisoned")
            .get(key)
            .cloned()
    }

    pub fn get_fresh(
        &self,
        key: &M2mTokenCacheKey,
        now: SystemTime,
        refresh_skew: Duration,
    ) -> Option<M2mAccessToken> {
        self.get(key)
            .filter(|token| token.is_fresh(now, refresh_skew))
    }

    pub fn refresh_decision(
        &self,
        key: &M2mTokenCacheKey,
        now: SystemTime,
        refresh_skew: Duration,
    ) -> Option<TokenRefreshDecision> {
        self.get(key)
            .map(|token| token.refresh_decision(now, refresh_skew))
    }

    pub fn remove(&self, key: &M2mTokenCacheKey) -> Option<M2mAccessToken> {
        self.entries
            .lock()
            .expect("m2m token cache mutex poisoned")
            .remove(key)
    }

    pub fn clear(&self) {
        self.entries
            .lock()
            .expect("m2m token cache mutex poisoned")
            .clear();
    }

    pub fn len(&self) -> usize {
        self.entries
            .lock()
            .expect("m2m token cache mutex poisoned")
            .len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientCredentialsAuthStyle {
    ClientSecretPost,
    ClientSecretBasic,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OAuthClientCredentialsRequest {
    pub token_url: String,
    pub client_id: String,
    pub client_secret: SecretString,
    #[serde(default)]
    pub scopes: Vec<String>,
    pub audience: Option<String>,
    pub resource: Option<String>,
    #[serde(default)]
    pub extra_form_fields: BTreeMap<String, String>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    pub auth_style: ClientCredentialsAuthStyle,
}

impl OAuthClientCredentialsRequest {
    pub fn builder(
        token_url: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
    ) -> OAuthClientCredentialsRequestBuilder {
        OAuthClientCredentialsRequestBuilder::new(token_url, client_id, client_secret)
    }

    pub fn to_form_request(&self) -> OAuthFormRequest {
        let mut form_fields = self.extra_form_fields.clone();
        form_fields.insert("grant_type".to_string(), "client_credentials".to_string());

        if !self.scopes.is_empty() {
            form_fields.insert("scope".to_string(), self.scopes.join(" "));
        }
        if let Some(audience) = &self.audience {
            form_fields.insert("audience".to_string(), audience.clone());
        }
        if let Some(resource) = &self.resource {
            form_fields.insert("resource".to_string(), resource.clone());
        }

        if self.auth_style == ClientCredentialsAuthStyle::ClientSecretPost {
            form_fields.insert("client_id".to_string(), self.client_id.clone());
            form_fields.insert(
                "client_secret".to_string(),
                self.client_secret.expose_secret().to_string(),
            );
        }

        let mut headers = self.headers.clone();
        headers
            .entry("accept".to_string())
            .or_insert_with(|| "application/json".to_string());
        headers
            .entry("content-type".to_string())
            .or_insert_with(|| "application/x-www-form-urlencoded".to_string());

        OAuthFormRequest {
            method: "POST".to_string(),
            url: self.token_url.clone(),
            headers,
            form_fields,
            basic_auth: (self.auth_style == ClientCredentialsAuthStyle::ClientSecretBasic)
                .then(|| (self.client_id.clone(), self.client_secret.clone())),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct OAuthFormRequest {
    pub method: String,
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub form_fields: BTreeMap<String, String>,
    pub basic_auth: Option<(String, SecretString)>,
}

impl fmt::Debug for OAuthFormRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OAuthFormRequest")
            .field("method", &self.method)
            .field("url", &self.url)
            .field("headers", &self.headers)
            .field("form_fields", &redacted_form_fields(&self.form_fields))
            .field(
                "basic_auth",
                &self.basic_auth.as_ref().map(|(id, _)| (id, "[REDACTED]")),
            )
            .finish()
    }
}

fn redacted_form_fields(fields: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    fields
        .iter()
        .map(|(key, value)| {
            let value = if key.eq_ignore_ascii_case("client_secret") {
                "[REDACTED]".to_string()
            } else {
                value.clone()
            };
            (key.clone(), value)
        })
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OAuthClientCredentialsRequestBuilder {
    request: OAuthClientCredentialsRequest,
}

impl OAuthClientCredentialsRequestBuilder {
    pub fn new(
        token_url: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
    ) -> Self {
        Self {
            request: OAuthClientCredentialsRequest {
                token_url: token_url.into(),
                client_id: client_id.into(),
                client_secret: SecretString::new(client_secret),
                scopes: Vec::new(),
                audience: None,
                resource: None,
                extra_form_fields: BTreeMap::new(),
                headers: BTreeMap::new(),
                auth_style: ClientCredentialsAuthStyle::ClientSecretPost,
            },
        }
    }

    pub fn auth_style(mut self, auth_style: ClientCredentialsAuthStyle) -> Self {
        self.request.auth_style = auth_style;
        self
    }

    pub fn scopes<I, S>(mut self, scopes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.request.scopes = scopes.into_iter().map(Into::into).collect();
        self
    }

    pub fn audience(mut self, audience: impl Into<String>) -> Self {
        self.request.audience = Some(audience.into());
        self
    }

    pub fn resource(mut self, resource: impl Into<String>) -> Self {
        self.request.resource = Some(resource.into());
        self
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.request.headers.insert(name.into(), value.into());
        self
    }

    pub fn extra_form_field(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.request
            .extra_form_fields
            .insert(name.into(), value.into());
        self
    }

    pub fn build(self) -> OAuthClientCredentialsRequest {
        self.request
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time(seconds: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)
    }

    #[test]
    fn token_refresh_decision_respects_skew() {
        let now = time(1_000);

        assert_eq!(
            TokenRefreshDecision::from_expiration(time(1_301), now, Duration::from_secs(300)),
            TokenRefreshDecision::Fresh
        );
        assert_eq!(
            TokenRefreshDecision::from_expiration(time(1_300), now, Duration::from_secs(300)),
            TokenRefreshDecision::RefreshWithinSkew
        );
        assert_eq!(
            TokenRefreshDecision::from_expiration(time(999), now, Duration::from_secs(300)),
            TokenRefreshDecision::Expired
        );
    }

    #[test]
    fn token_cache_returns_only_fresh_tokens() {
        let cache = M2mTokenCache::new();
        let key = M2mTokenCacheKey::new("generic", "primary", "client")
            .with_tenant_key("tenant-a")
            .with_scopes(["write", "read", "read"]);
        cache.insert(
            key.clone(),
            M2mAccessToken::bearer("secret-token", time(1_600), ["read", "write"]),
        );

        assert_eq!(key.scopes, vec!["read", "write"]);
        assert!(cache
            .get_fresh(&key, time(1_000), Duration::from_secs(300))
            .is_some());
        assert!(cache
            .get_fresh(&key, time(1_400), Duration::from_secs(300))
            .is_none());
        assert_eq!(
            cache.refresh_decision(&key, time(1_400), Duration::from_secs(300)),
            Some(TokenRefreshDecision::RefreshWithinSkew)
        );
    }

    #[test]
    fn token_debug_output_redacts_secret() {
        let token = M2mAccessToken::bearer("very-secret-token", time(1_600), ["read"]);
        let debug = format!("{token:?}");

        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("very-secret-token"));
    }

    #[test]
    fn token_serialization_redacts_secret() {
        let token = M2mAccessToken::bearer("very-secret-token", time(1_600), ["read"]);
        let serialized = serde_json::to_string(&token).expect("token should serialize");

        assert!(serialized.contains("[REDACTED]"));
        assert!(!serialized.contains("very-secret-token"));
    }

    #[test]
    fn client_credentials_post_request_builds_form_shape() {
        let request = OAuthClientCredentialsRequest::builder(
            "https://issuer.example.test/oauth/token",
            "client-a",
            "secret-a",
        )
        .scopes(["read", "write"])
        .audience("https://api.example.test")
        .extra_form_field("vendor_hint", "default")
        .build();

        let form = request.to_form_request();

        assert_eq!(form.method, "POST");
        assert_eq!(form.form_fields["grant_type"], "client_credentials");
        assert_eq!(form.form_fields["client_id"], "client-a");
        assert_eq!(form.form_fields["client_secret"], "secret-a");
        assert_eq!(form.form_fields["scope"], "read write");
        assert_eq!(form.form_fields["audience"], "https://api.example.test");
        assert!(form.basic_auth.is_none());
    }

    #[test]
    fn client_credentials_basic_request_keeps_secret_out_of_form() {
        let request = OAuthClientCredentialsRequest::builder(
            "https://issuer.example.test/oauth/token",
            "client-a",
            "secret-a",
        )
        .auth_style(ClientCredentialsAuthStyle::ClientSecretBasic)
        .resource("https://resource.example.test")
        .build();

        let form = request.to_form_request();

        assert!(!form.form_fields.contains_key("client_secret"));
        assert!(!form.form_fields.contains_key("client_id"));
        assert_eq!(
            form.form_fields["resource"],
            "https://resource.example.test"
        );
        assert_eq!(
            form.basic_auth
                .as_ref()
                .map(|(client_id, _)| client_id.as_str()),
            Some("client-a")
        );

        let debug = format!("{form:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("secret-a"));
    }
}
