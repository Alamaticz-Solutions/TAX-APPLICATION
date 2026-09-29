use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, SystemTime},
};

use appfw_saas_core::{
    M2mAccessToken, M2mTokenCache, M2mTokenCacheKey, OAuthClientCredentialsRequest,
};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::Url;
use serde::Deserialize;
use thiserror::Error;

use crate::saas_transport::{
    ReqwestSaasHttpTransport, RuntimeHttpSaasRequestExecutor, RuntimeSaasExecutionError,
    RuntimeSaasHttpRequest, RuntimeSaasHttpRequestBody, RuntimeSaasHttpResponse,
    RuntimeSaasHttpTransport, RuntimeSaasRequest, RuntimeSaasRequestExecutor, RuntimeSaasResponse,
};
use crate::UserAuth;

#[derive(Clone, Debug, Error)]
pub enum RuntimeSaasOAuthError {
    #[error("OAuth token endpoint is invalid: {0}")]
    InvalidEndpoint(String),
    #[error("OAuth token request is invalid: {0}")]
    InvalidRequest(String),
    #[error("OAuth token endpoint returned HTTP {status}: {body}")]
    HttpStatus { status: u16, body: String },
    #[error("OAuth token response is invalid: {0}")]
    InvalidResponse(String),
    #[error(transparent)]
    Transport(#[from] RuntimeSaasExecutionError),
}

#[async_trait]
pub trait RuntimeOAuthClientCredentialsTokenExecutor: Send + Sync {
    type Error: Send + Sync + 'static;

    async fn execute_client_credentials_token_request(
        &self,
        request: OAuthClientCredentialsRequest,
        now: SystemTime,
    ) -> Result<M2mAccessToken, Self::Error>;
}

#[derive(Clone)]
pub struct RuntimeHttpOAuthClientCredentialsTokenExecutor<T>
where
    T: RuntimeSaasHttpTransport,
{
    transport: Arc<T>,
    timeout_ms: u64,
}

pub type ReqwestRuntimeOAuthClientCredentialsTokenExecutor =
    RuntimeHttpOAuthClientCredentialsTokenExecutor<ReqwestSaasHttpTransport>;

#[derive(Clone)]
pub struct RuntimeClientCredentialsSaasRequestExecutor<TokenExecutor, Transport>
where
    TokenExecutor: RuntimeOAuthClientCredentialsTokenExecutor<Error = RuntimeSaasOAuthError>,
    Transport: Clone + RuntimeSaasHttpTransport,
{
    token_executor: Arc<TokenExecutor>,
    transport: Transport,
    token_request: OAuthClientCredentialsRequest,
    token_cache: Arc<M2mTokenCache>,
    refresh_skew: Duration,
}

impl<TokenExecutor, Transport> RuntimeClientCredentialsSaasRequestExecutor<TokenExecutor, Transport>
where
    TokenExecutor: RuntimeOAuthClientCredentialsTokenExecutor<Error = RuntimeSaasOAuthError>,
    Transport: Clone + RuntimeSaasHttpTransport,
{
    pub fn new(
        token_executor: TokenExecutor,
        transport: Transport,
        token_request: OAuthClientCredentialsRequest,
    ) -> Self {
        Self {
            token_executor: Arc::new(token_executor),
            transport,
            token_request,
            token_cache: Arc::new(M2mTokenCache::new()),
            refresh_skew: Duration::from_secs(60),
        }
    }

    pub fn with_token_cache(mut self, token_cache: Arc<M2mTokenCache>) -> Self {
        self.token_cache = token_cache;
        self
    }

    pub fn with_refresh_skew(mut self, refresh_skew: Duration) -> Self {
        self.refresh_skew = refresh_skew;
        self
    }

    pub fn token_cache(&self) -> Arc<M2mTokenCache> {
        self.token_cache.clone()
    }

    async fn bearer_token_for_request(
        &self,
        request: &RuntimeSaasRequest,
        tenant_key: Option<&str>,
    ) -> Result<M2mAccessToken, RuntimeSaasOAuthError> {
        let now = SystemTime::now();
        let cache_key = token_cache_key_for_request(request, &self.token_request, tenant_key);
        if let Some(token) = self
            .token_cache
            .get_fresh(&cache_key, now, self.refresh_skew)
        {
            return Ok(token);
        }

        let token = self
            .token_executor
            .execute_client_credentials_token_request(self.token_request.clone(), now)
            .await?;
        self.token_cache.insert(cache_key, token.clone());
        Ok(token)
    }
}

#[async_trait]
impl<TokenExecutor, Transport> RuntimeSaasRequestExecutor
    for RuntimeClientCredentialsSaasRequestExecutor<TokenExecutor, Transport>
where
    TokenExecutor: RuntimeOAuthClientCredentialsTokenExecutor<Error = RuntimeSaasOAuthError>,
    Transport: Clone + RuntimeSaasHttpTransport,
{
    type Error = RuntimeSaasOAuthError;

    async fn execute_saas_request(
        &self,
        request: RuntimeSaasRequest,
        user: &UserAuth,
    ) -> Result<RuntimeSaasResponse, Self::Error> {
        if request
            .plan
            .headers
            .keys()
            .any(|name| name.eq_ignore_ascii_case("authorization"))
        {
            return Err(RuntimeSaasOAuthError::Transport(
                RuntimeSaasExecutionError::InvalidRequestPlan(
                    "SaaS request plans must not include Authorization; credentials are supplied by the runtime executor".to_string(),
                ),
            ));
        }

        let token = self
            .bearer_token_for_request(&request, Some(user.tenant_id.as_str()))
            .await?;
        RuntimeHttpSaasRequestExecutor::new(self.transport.clone())
            .with_bearer_token(token.access_token.expose_secret())
            .execute_saas_request(request, user)
            .await
            .map_err(RuntimeSaasOAuthError::Transport)
    }
}

fn token_cache_key_for_request(
    request: &RuntimeSaasRequest,
    token_request: &OAuthClientCredentialsRequest,
    tenant_key: Option<&str>,
) -> M2mTokenCacheKey {
    let key = M2mTokenCacheKey::new(
        request.endpoint.provider.key(),
        request.endpoint.data_source_name.clone(),
        token_request.client_id.clone(),
    )
    .with_scopes(token_request.scopes.clone());
    match tenant_key.filter(|value| !value.trim().is_empty()) {
        Some(tenant_key) => key.with_tenant_key(tenant_key),
        None => key,
    }
}

impl RuntimeHttpOAuthClientCredentialsTokenExecutor<ReqwestSaasHttpTransport> {
    pub fn reqwest() -> Self {
        Self::new(ReqwestSaasHttpTransport::default())
    }
}

impl<T> RuntimeHttpOAuthClientCredentialsTokenExecutor<T>
where
    T: RuntimeSaasHttpTransport,
{
    pub fn new(transport: T) -> Self {
        Self {
            transport: Arc::new(transport),
            timeout_ms: 5_000,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout_ms = timeout.as_millis().try_into().unwrap_or(u64::MAX);
        self
    }

    fn build_http_request(
        &self,
        request: OAuthClientCredentialsRequest,
    ) -> Result<RuntimeSaasHttpRequest, RuntimeSaasOAuthError> {
        let form_request = request.to_form_request();
        validate_token_endpoint(&form_request.url)?;
        validate_form_headers(&form_request.headers)?;
        let url = Url::parse(&form_request.url)
            .map_err(|err| RuntimeSaasOAuthError::InvalidEndpoint(err.to_string()))?;

        let mut headers = form_request.headers.clone();
        if let Some((client_id, secret)) = form_request.basic_auth {
            headers.insert(
                "authorization".to_string(),
                format!(
                    "Basic {}",
                    STANDARD.encode(format!("{client_id}:{}", secret.expose_secret()))
                ),
            );
        }

        Ok(RuntimeSaasHttpRequest {
            method: appfw_saas_core::SaasHttpMethod::Post,
            url,
            headers,
            body: RuntimeSaasHttpRequestBody::Text {
                content_type: "application/x-www-form-urlencoded".to_string(),
                value: encode_form_fields(&form_request.form_fields),
            },
            timeout_ms: self.timeout_ms,
        })
    }
}

#[async_trait]
impl<T> RuntimeOAuthClientCredentialsTokenExecutor
    for RuntimeHttpOAuthClientCredentialsTokenExecutor<T>
where
    T: RuntimeSaasHttpTransport,
{
    type Error = RuntimeSaasOAuthError;

    async fn execute_client_credentials_token_request(
        &self,
        request: OAuthClientCredentialsRequest,
        now: SystemTime,
    ) -> Result<M2mAccessToken, Self::Error> {
        let scopes = request.scopes.clone();
        let http_request = self.build_http_request(request)?;
        let response = self.transport.send(http_request).await?;
        parse_token_response(response, scopes, now)
    }
}

#[derive(Debug, Deserialize)]
struct TokenEndpointResponse {
    access_token: Option<String>,
    #[serde(default)]
    token_type: Option<String>,
    #[serde(default)]
    expires_in: Option<u64>,
    #[serde(default)]
    scope: Option<String>,
}

fn parse_token_response(
    response: RuntimeSaasHttpResponse,
    requested_scopes: Vec<String>,
    now: SystemTime,
) -> Result<M2mAccessToken, RuntimeSaasOAuthError> {
    if !(200..300).contains(&response.status) {
        return Err(RuntimeSaasOAuthError::HttpStatus {
            status: response.status,
            body: response_body_preview(&response.body),
        });
    }

    let token_response: TokenEndpointResponse = serde_json::from_slice(&response.body)
        .map_err(|err| RuntimeSaasOAuthError::InvalidResponse(err.to_string()))?;
    let access_token = token_response
        .access_token
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            RuntimeSaasOAuthError::InvalidResponse("missing non-empty access_token".to_string())
        })?;
    let token_type = token_response
        .token_type
        .unwrap_or_else(|| "Bearer".to_string());
    if !token_type.eq_ignore_ascii_case("bearer") {
        return Err(RuntimeSaasOAuthError::InvalidResponse(format!(
            "unsupported token_type {token_type}"
        )));
    }
    let expires_in = token_response.expires_in.unwrap_or(3_600);
    let scopes = token_response
        .scope
        .map(|value| value.split_whitespace().map(str::to_string).collect())
        .unwrap_or(requested_scopes);

    Ok(M2mAccessToken::bearer(
        access_token,
        now + Duration::from_secs(expires_in),
        scopes,
    ))
}

fn validate_token_endpoint(url: &str) -> Result<(), RuntimeSaasOAuthError> {
    let url =
        Url::parse(url).map_err(|err| RuntimeSaasOAuthError::InvalidEndpoint(err.to_string()))?;
    if url.scheme() != "https" {
        return Err(RuntimeSaasOAuthError::InvalidEndpoint(
            "token_url must use https".to_string(),
        ));
    }
    if url.host_str().is_none() {
        return Err(RuntimeSaasOAuthError::InvalidEndpoint(
            "token_url must include a host".to_string(),
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(RuntimeSaasOAuthError::InvalidEndpoint(
            "token_url must not include credentials".to_string(),
        ));
    }
    Ok(())
}

fn validate_form_headers(headers: &BTreeMap<String, String>) -> Result<(), RuntimeSaasOAuthError> {
    if headers
        .keys()
        .any(|name| name.eq_ignore_ascii_case("authorization"))
    {
        return Err(RuntimeSaasOAuthError::InvalidRequest(
            "OAuth form request headers must not include Authorization; credentials are supplied by the runtime executor".to_string(),
        ));
    }
    Ok(())
}

fn encode_form_fields(fields: &BTreeMap<String, String>) -> String {
    fields
        .iter()
        .map(|(key, value)| format!("{}={}", form_encode(key), form_encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

fn form_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn response_body_preview(body: &[u8]) -> String {
    String::from_utf8_lossy(&body[..body.len().min(256)]).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{provider_keys::FrameworkProvider, RuntimeSaasEndpoint};
    use appfw_saas_core::{
        ClientCredentialsAuthStyle, SaasHttpMethod, SaasRequestPlan, SaasTransportProtocol,
    };
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeHttpTransport {
        last_request: Mutex<Option<RuntimeSaasHttpRequest>>,
        response: Mutex<Option<RuntimeSaasHttpResponse>>,
    }

    impl FakeHttpTransport {
        fn with_response(response: RuntimeSaasHttpResponse) -> Self {
            Self {
                last_request: Mutex::new(None),
                response: Mutex::new(Some(response)),
            }
        }

        fn last_request(&self) -> RuntimeSaasHttpRequest {
            self.last_request
                .lock()
                .expect("request mutex")
                .clone()
                .expect("request captured")
        }
    }

    #[derive(Default)]
    struct FakeTokenExecutor {
        calls: Mutex<Vec<OAuthClientCredentialsRequest>>,
        tokens: Mutex<Vec<M2mAccessToken>>,
    }

    impl FakeTokenExecutor {
        fn with_tokens(tokens: Vec<M2mAccessToken>) -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                tokens: Mutex::new(tokens),
            }
        }

        fn call_count(&self) -> usize {
            self.calls.lock().expect("calls mutex").len()
        }
    }

    #[async_trait]
    impl RuntimeOAuthClientCredentialsTokenExecutor for FakeTokenExecutor {
        type Error = RuntimeSaasOAuthError;

        async fn execute_client_credentials_token_request(
            &self,
            request: OAuthClientCredentialsRequest,
            _now: SystemTime,
        ) -> Result<M2mAccessToken, Self::Error> {
            self.calls.lock().expect("calls mutex").push(request);
            self.tokens
                .lock()
                .expect("tokens mutex")
                .pop()
                .ok_or_else(|| {
                    RuntimeSaasOAuthError::InvalidResponse("missing fake token".to_string())
                })
        }
    }

    #[async_trait]
    impl RuntimeSaasHttpTransport for Arc<FakeHttpTransport> {
        async fn send(
            &self,
            request: RuntimeSaasHttpRequest,
        ) -> Result<RuntimeSaasHttpResponse, RuntimeSaasExecutionError> {
            *self.last_request.lock().expect("request mutex") = Some(request);
            self.response
                .lock()
                .expect("response mutex")
                .take()
                .ok_or_else(|| RuntimeSaasExecutionError::Http("missing fake response".to_string()))
        }
    }

    fn now() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_000)
    }

    fn service_user() -> UserAuth {
        service_user_for_tenant("tenant-a")
    }

    fn service_user_for_tenant(tenant_id: &str) -> UserAuth {
        UserAuth::service(
            tenant_id,
            "saas-sync-worker",
            vec!["integration_writer".to_string()],
            vec!["servicenow.task.read".to_string()],
        )
        .with_ingress("sync_worker")
    }

    fn token_request() -> OAuthClientCredentialsRequest {
        OAuthClientCredentialsRequest::builder(
            "https://issuer.example.test/oauth/token",
            "client-a",
            "secret-a",
        )
        .scopes(["task.read"])
        .audience("https://api.example.test")
        .build()
    }

    fn saas_request() -> RuntimeSaasRequest {
        RuntimeSaasRequest::new(
            RuntimeSaasEndpoint::new(
                FrameworkProvider::ServiceNow,
                "servicenow_primary",
                "https://instance.service-now.test",
            )
            .expect("endpoint"),
            SaasRequestPlan::new(
                "servicenow.task.list",
                SaasTransportProtocol::RestJson,
                SaasHttpMethod::Get,
                "/api/now/table/task",
            ),
        )
        .expect("request")
    }

    #[test]
    fn client_credentials_request_executor_injects_runtime_bearer_token() {
        let transport = Arc::new(FakeHttpTransport::with_response(RuntimeSaasHttpResponse {
            status: 200,
            headers: BTreeMap::new(),
            content_type: Some("application/json".to_string()),
            body: br#"{"records":[{"number":"TASK001"}]}"#.to_vec(),
        }));
        let token_executor = FakeTokenExecutor::with_tokens(vec![M2mAccessToken::bearer(
            "runtime-access-token",
            SystemTime::now() + Duration::from_secs(3_600),
            ["task.read"],
        )]);
        let executor = RuntimeClientCredentialsSaasRequestExecutor::new(
            token_executor,
            transport.clone(),
            token_request(),
        );
        let runtime = tokio::runtime::Runtime::new().expect("runtime");

        let response = runtime
            .block_on(executor.execute_saas_request(saas_request(), &service_user()))
            .expect("saas response");

        let captured = transport.last_request();
        assert_eq!(
            captured.headers.get("authorization").map(String::as_str),
            Some("Bearer runtime-access-token")
        );
        assert_eq!(response.metadata.status, 200);
        assert_eq!(response.metadata.observed_rows, Some(1));
    }

    #[test]
    fn client_credentials_request_executor_reuses_fresh_cached_token() {
        let token_executor = FakeTokenExecutor::with_tokens(vec![M2mAccessToken::bearer(
            "cached-token",
            SystemTime::now() + Duration::from_secs(3_600),
            ["task.read"],
        )]);
        let executor = RuntimeClientCredentialsSaasRequestExecutor::new(
            token_executor,
            Arc::new(FakeHttpTransport::default()),
            token_request(),
        );
        let request = saas_request();
        let runtime = tokio::runtime::Runtime::new().expect("runtime");

        let first = runtime
            .block_on(executor.bearer_token_for_request(&request, Some("tenant-a")))
            .expect("first token");
        let second = runtime
            .block_on(executor.bearer_token_for_request(&request, Some("tenant-a")))
            .expect("cached token");

        assert_eq!(first, second);
        assert_eq!(executor.token_executor.call_count(), 1);
    }

    #[test]
    fn client_credentials_request_executor_refreshes_expiring_token() {
        let token_executor = FakeTokenExecutor::with_tokens(vec![
            M2mAccessToken::bearer(
                "refreshed-token",
                SystemTime::now() + Duration::from_secs(3_600),
                ["task.read"],
            ),
            M2mAccessToken::bearer(
                "expiring-token",
                SystemTime::now() + Duration::from_secs(1),
                ["task.read"],
            ),
        ]);
        let executor = RuntimeClientCredentialsSaasRequestExecutor::new(
            token_executor,
            Arc::new(FakeHttpTransport::default()),
            token_request(),
        )
        .with_refresh_skew(Duration::from_secs(60));
        let request = saas_request();
        let runtime = tokio::runtime::Runtime::new().expect("runtime");

        let first = runtime
            .block_on(executor.bearer_token_for_request(&request, Some("tenant-a")))
            .expect("first token");
        let second = runtime
            .block_on(executor.bearer_token_for_request(&request, Some("tenant-a")))
            .expect("refreshed token");

        assert_eq!(first.access_token.expose_secret(), "expiring-token");
        assert_eq!(second.access_token.expose_secret(), "refreshed-token");
        assert_eq!(executor.token_executor.call_count(), 2);
    }

    #[test]
    fn client_credentials_request_executor_partitions_cached_tokens_by_tenant() {
        let token_executor = FakeTokenExecutor::with_tokens(vec![
            M2mAccessToken::bearer(
                "tenant-b-token",
                SystemTime::now() + Duration::from_secs(3_600),
                ["task.read"],
            ),
            M2mAccessToken::bearer(
                "tenant-a-token",
                SystemTime::now() + Duration::from_secs(3_600),
                ["task.read"],
            ),
        ]);
        let transport = Arc::new(FakeHttpTransport::with_response(RuntimeSaasHttpResponse {
            status: 200,
            headers: BTreeMap::new(),
            content_type: Some("application/json".to_string()),
            body: br#"{"records":[]}"#.to_vec(),
        }));
        let executor = RuntimeClientCredentialsSaasRequestExecutor::new(
            token_executor,
            transport.clone(),
            token_request(),
        );
        let runtime = tokio::runtime::Runtime::new().expect("runtime");

        runtime
            .block_on(
                executor.execute_saas_request(saas_request(), &service_user_for_tenant("tenant-a")),
            )
            .expect("tenant a response");
        let tenant_a_request = transport.last_request();
        *transport.response.lock().expect("response mutex") = Some(RuntimeSaasHttpResponse {
            status: 200,
            headers: BTreeMap::new(),
            content_type: Some("application/json".to_string()),
            body: br#"{"records":[]}"#.to_vec(),
        });
        runtime
            .block_on(
                executor.execute_saas_request(saas_request(), &service_user_for_tenant("tenant-b")),
            )
            .expect("tenant b response");
        let tenant_b_request = transport.last_request();

        assert_eq!(
            tenant_a_request
                .headers
                .get("authorization")
                .map(String::as_str),
            Some("Bearer tenant-a-token")
        );
        assert_eq!(
            tenant_b_request
                .headers
                .get("authorization")
                .map(String::as_str),
            Some("Bearer tenant-b-token")
        );
        assert_eq!(executor.token_executor.call_count(), 2);
    }

    #[test]
    fn client_credentials_request_executor_rejects_plan_authorization_before_token_fetch() {
        let token_executor = FakeTokenExecutor::with_tokens(vec![M2mAccessToken::bearer(
            "unused-token",
            SystemTime::now() + Duration::from_secs(3_600),
            ["task.read"],
        )]);
        let transport = Arc::new(FakeHttpTransport::with_response(RuntimeSaasHttpResponse {
            status: 200,
            headers: BTreeMap::new(),
            content_type: Some("application/json".to_string()),
            body: br#"{"result":[]}"#.to_vec(),
        }));
        let executor = RuntimeClientCredentialsSaasRequestExecutor::new(
            token_executor,
            transport,
            token_request(),
        );
        let request = RuntimeSaasRequest::new(
            RuntimeSaasEndpoint::new(
                FrameworkProvider::ServiceNow,
                "servicenow_primary",
                "https://instance.service-now.test",
            )
            .expect("endpoint"),
            SaasRequestPlan::new(
                "servicenow.task.list",
                SaasTransportProtocol::RestJson,
                SaasHttpMethod::Get,
                "/api/now/table/task",
            )
            .with_header("Authorization", "Bearer caller-token"),
        )
        .expect("request");
        let runtime = tokio::runtime::Runtime::new().expect("runtime");

        let err = runtime
            .block_on(executor.execute_saas_request(request, &service_user()))
            .expect_err("authorization header rejected");

        assert!(matches!(
            err,
            RuntimeSaasOAuthError::Transport(RuntimeSaasExecutionError::InvalidRequestPlan(_))
        ));
        assert_eq!(executor.token_executor.call_count(), 0);
    }

    #[test]
    fn token_executor_posts_client_secret_form_and_parses_bearer_token() {
        let transport = Arc::new(FakeHttpTransport::with_response(RuntimeSaasHttpResponse {
            status: 200,
            headers: BTreeMap::new(),
            content_type: Some("application/json".to_string()),
            body: br#"{"access_token":"runtime-token","token_type":"Bearer","expires_in":900,"scope":"read write"}"#.to_vec(),
        }));
        let executor = RuntimeHttpOAuthClientCredentialsTokenExecutor::new(transport.clone());
        let request = OAuthClientCredentialsRequest::builder(
            "https://issuer.example.test/oauth/token",
            "client-a",
            "secret-a",
        )
        .scopes(["read", "write"])
        .audience("https://api.example.test")
        .build();

        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let token = runtime
            .block_on(executor.execute_client_credentials_token_request(request, now()))
            .expect("token");

        let captured = transport.last_request();
        assert_eq!(captured.method, appfw_saas_core::SaasHttpMethod::Post);
        assert_eq!(
            captured.url.as_str(),
            "https://issuer.example.test/oauth/token"
        );
        assert_eq!(
            captured.headers.get("content-type").map(String::as_str),
            Some("application/x-www-form-urlencoded")
        );
        assert!(!captured.headers.contains_key("authorization"));
        assert!(matches!(
            captured.body,
            RuntimeSaasHttpRequestBody::Text { ref value, .. }
                if value.contains("grant_type=client_credentials")
                    && value.contains("client_id=client-a")
                    && value.contains("client_secret=secret-a")
                    && value.contains("scope=read+write")
                    && value.contains("audience=https%3A%2F%2Fapi.example.test")
        ));
        assert_eq!(token.expires_at, now() + Duration::from_secs(900));
        assert_eq!(token.scopes, vec!["read", "write"]);
        assert!(!format!("{token:?}").contains("runtime-token"));
    }

    #[test]
    fn token_executor_uses_basic_auth_without_secret_in_form() {
        let transport = Arc::new(FakeHttpTransport::with_response(RuntimeSaasHttpResponse {
            status: 200,
            headers: BTreeMap::new(),
            content_type: Some("application/json".to_string()),
            body: br#"{"access_token":"basic-token","expires_in":3600}"#.to_vec(),
        }));
        let executor = RuntimeHttpOAuthClientCredentialsTokenExecutor::new(transport.clone());
        let request = OAuthClientCredentialsRequest::builder(
            "https://issuer.example.test/oauth/token",
            "client-a",
            "secret-a",
        )
        .auth_style(ClientCredentialsAuthStyle::ClientSecretBasic)
        .resource("https://resource.example.test")
        .build();

        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        runtime
            .block_on(executor.execute_client_credentials_token_request(request, now()))
            .expect("token");

        let captured = transport.last_request();
        assert_eq!(
            captured.headers.get("authorization").map(String::as_str),
            Some("Basic Y2xpZW50LWE6c2VjcmV0LWE=")
        );
        assert!(matches!(
            captured.body,
            RuntimeSaasHttpRequestBody::Text { ref value, .. }
                if !value.contains("client_secret")
                    && !value.contains("client_id")
                    && value.contains("resource=https%3A%2F%2Fresource.example.test")
        ));
    }

    #[test]
    fn token_executor_rejects_non_https_endpoint_and_header_supplied_auth() {
        let transport = Arc::new(FakeHttpTransport::default());
        let executor = RuntimeHttpOAuthClientCredentialsTokenExecutor::new(transport);
        let non_https = OAuthClientCredentialsRequest::builder(
            "http://issuer.example.test/oauth/token",
            "client-a",
            "secret-a",
        )
        .build();
        let runtime = tokio::runtime::Runtime::new().expect("runtime");

        let err = runtime
            .block_on(executor.execute_client_credentials_token_request(non_https, now()))
            .expect_err("http endpoint rejected");
        assert!(matches!(err, RuntimeSaasOAuthError::InvalidEndpoint(_)));

        let transport = Arc::new(FakeHttpTransport::default());
        let executor = RuntimeHttpOAuthClientCredentialsTokenExecutor::new(transport);
        let header_auth = OAuthClientCredentialsRequest::builder(
            "https://issuer.example.test/oauth/token",
            "client-a",
            "secret-a",
        )
        .header("Authorization", "Bearer caller")
        .build();

        let err = runtime
            .block_on(executor.execute_client_credentials_token_request(header_auth, now()))
            .expect_err("caller authorization header rejected");
        assert!(matches!(err, RuntimeSaasOAuthError::InvalidRequest(_)));
    }

    #[test]
    fn token_executor_rejects_error_status_and_invalid_token_shape() {
        let transport = Arc::new(FakeHttpTransport::with_response(RuntimeSaasHttpResponse {
            status: 401,
            headers: BTreeMap::new(),
            content_type: Some("application/json".to_string()),
            body: br#"{"error":"invalid_client"}"#.to_vec(),
        }));
        let executor = RuntimeHttpOAuthClientCredentialsTokenExecutor::new(transport);
        let request = OAuthClientCredentialsRequest::builder(
            "https://issuer.example.test/oauth/token",
            "client-a",
            "secret-a",
        )
        .build();
        let runtime = tokio::runtime::Runtime::new().expect("runtime");

        let err = runtime
            .block_on(executor.execute_client_credentials_token_request(request, now()))
            .expect_err("error status rejected");
        assert!(matches!(
            err,
            RuntimeSaasOAuthError::HttpStatus { status: 401, .. }
        ));

        let transport = Arc::new(FakeHttpTransport::with_response(RuntimeSaasHttpResponse {
            status: 200,
            headers: BTreeMap::new(),
            content_type: Some("application/json".to_string()),
            body: br#"{"token_type":"Bearer"}"#.to_vec(),
        }));
        let executor = RuntimeHttpOAuthClientCredentialsTokenExecutor::new(transport);
        let request = OAuthClientCredentialsRequest::builder(
            "https://issuer.example.test/oauth/token",
            "client-a",
            "secret-a",
        )
        .build();

        let err = runtime
            .block_on(executor.execute_client_credentials_token_request(request, now()))
            .expect_err("missing token rejected");
        assert!(matches!(err, RuntimeSaasOAuthError::InvalidResponse(_)));
    }
}
