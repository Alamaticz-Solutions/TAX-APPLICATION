use std::{collections::BTreeMap, sync::Arc, time::Duration};

use appfw_saas_core::{
    RateLimitSignal, SaasHttpMethod, SaasRequestBody, SaasRequestPlan, SaasResponseCaps,
    SecretString, REDACTED,
};
use async_trait::async_trait;
use reqwest::{header::HeaderMap, Url};
use serde::Serialize;
use serde_json::{json, Value};
use thiserror::Error;

use crate::{provider_keys::FrameworkProvider, UserAuth};

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum RuntimeSaasTransportError {
    #[error("SaaS endpoint provider must be an external API provider")]
    NonExternalApiProvider,
    #[error("SaaS endpoint base URL is invalid: {0}")]
    InvalidEndpoint(String),
    #[error("SaaS request plan is invalid: {0}")]
    InvalidRequestPlan(String),
    #[error("SaaS response exceeds caps: {0}")]
    ResponseCapsExceeded(String),
}

#[derive(Clone, Debug, Error)]
pub enum RuntimeSaasExecutionError {
    #[error(transparent)]
    Transport(#[from] RuntimeSaasTransportError),
    #[error("SaaS request execution failed: {0}")]
    Http(String),
    #[error("SaaS request plan is invalid: {0}")]
    InvalidRequestPlan(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeSaasEndpoint {
    pub provider: FrameworkProvider,
    pub data_source_name: String,
    pub base_url: String,
}

impl RuntimeSaasEndpoint {
    pub fn new(
        provider: FrameworkProvider,
        data_source_name: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Result<Self, RuntimeSaasTransportError> {
        let endpoint = Self {
            provider,
            data_source_name: data_source_name.into(),
            base_url: normalize_base_url(base_url.into())?,
        };
        endpoint.validate()?;
        Ok(endpoint)
    }

    pub fn validate(&self) -> Result<(), RuntimeSaasTransportError> {
        if !self.provider.is_external_api_provider() {
            return Err(RuntimeSaasTransportError::NonExternalApiProvider);
        }
        if self.data_source_name.trim().is_empty()
            || self.data_source_name.chars().any(char::is_whitespace)
        {
            return Err(RuntimeSaasTransportError::InvalidEndpoint(
                "data_source_name must be a non-empty token".to_string(),
            ));
        }
        validate_base_url(&self.base_url).map(|_| ())
    }

    pub fn url_for_plan(&self, plan: &SaasRequestPlan) -> Result<Url, RuntimeSaasTransportError> {
        plan.validate()
            .map_err(RuntimeSaasTransportError::InvalidRequestPlan)?;

        let mut url = Url::parse(&self.base_url)
            .map_err(|err| RuntimeSaasTransportError::InvalidEndpoint(err.to_string()))?;
        url.set_path(plan.path.as_str());
        if !plan.query.is_empty() {
            let mut pairs = url.query_pairs_mut();
            for (name, value) in &plan.query {
                pairs.append_pair(name, value);
            }
        }
        Ok(url)
    }

    pub fn redacted_summary(&self) -> Value {
        json!({
            "provider": self.provider.key(),
            "data_source": self.data_source_name,
            "base_url_origin": base_url_origin(&self.base_url).unwrap_or(REDACTED.to_string()),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RuntimeSaasRequest {
    pub endpoint: RuntimeSaasEndpoint,
    pub plan: SaasRequestPlan,
}

impl RuntimeSaasRequest {
    pub fn new(
        endpoint: RuntimeSaasEndpoint,
        plan: SaasRequestPlan,
    ) -> Result<Self, RuntimeSaasTransportError> {
        endpoint.validate()?;
        plan.validate()
            .map_err(RuntimeSaasTransportError::InvalidRequestPlan)?;
        Ok(Self { endpoint, plan })
    }

    pub fn url(&self) -> Result<Url, RuntimeSaasTransportError> {
        self.endpoint.url_for_plan(&self.plan)
    }

    pub fn timeout_ms(&self) -> u64 {
        self.plan.timeout_ms
    }

    pub fn redacted_summary(&self) -> Value {
        json!({
            "endpoint": self.endpoint.redacted_summary(),
            "request": self.plan.redacted_summary(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeSaasResponseMetadata {
    pub status: u16,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub header_names: Vec<String>,
    pub body_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_rows: Option<u32>,
    pub rate_limit: RateLimitSignal,
}

impl RuntimeSaasResponseMetadata {
    pub fn new(
        status: u16,
        headers: BTreeMap<String, String>,
        body_bytes: u64,
        observed_rows: Option<u32>,
    ) -> Self {
        let rate_limit = RateLimitSignal::from_headers(headers.iter());
        let header_names = headers.keys().cloned().collect();
        Self {
            status,
            header_names,
            body_bytes,
            observed_rows,
            rate_limit,
        }
    }

    pub fn validate_caps(&self, caps: SaasResponseCaps) -> Result<(), RuntimeSaasTransportError> {
        caps.validate()
            .map_err(RuntimeSaasTransportError::ResponseCapsExceeded)?;

        if self.body_bytes > caps.max_body_bytes {
            return Err(RuntimeSaasTransportError::ResponseCapsExceeded(format!(
                "body_bytes {} exceeds max_body_bytes {}",
                self.body_bytes, caps.max_body_bytes
            )));
        }
        if let Some(rows) = self.observed_rows {
            if rows > caps.max_rows {
                return Err(RuntimeSaasTransportError::ResponseCapsExceeded(format!(
                    "observed_rows {} exceeds max_rows {}",
                    rows, caps.max_rows
                )));
            }
        }
        Ok(())
    }

    pub fn redacted_summary(&self) -> Value {
        json!({
            "status": self.status,
            "header_names": &self.header_names,
            "body_bytes": self.body_bytes,
            "observed_rows": self.observed_rows,
            "rate_limited": self.rate_limit.is_limited(),
            "retry_after_ms": self.rate_limit.retry_after_ms,
            "api_usage_remaining": self.rate_limit.api_usage.as_ref().map(|usage| usage.remaining()),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeSaasResponseBody {
    Empty,
    Json(Value),
    Text(String),
    Bytes(Vec<u8>),
}

impl RuntimeSaasResponseBody {
    pub fn byte_len(&self) -> u64 {
        match self {
            Self::Empty => 0,
            Self::Json(value) => serde_json::to_vec(value)
                .map(|bytes| bytes.len() as u64)
                .unwrap_or(u64::MAX),
            Self::Text(value) => value.len() as u64,
            Self::Bytes(value) => value.len() as u64,
        }
    }

    pub fn content_type_hint(&self) -> Option<&'static str> {
        match self {
            Self::Empty => None,
            Self::Json(_) => Some("application/json"),
            Self::Text(_) => Some("text/plain"),
            Self::Bytes(_) => Some("application/octet-stream"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeSaasResponse {
    pub metadata: RuntimeSaasResponseMetadata,
    pub body: RuntimeSaasResponseBody,
}

impl RuntimeSaasResponse {
    pub fn new(
        status: u16,
        headers: BTreeMap<String, String>,
        body: RuntimeSaasResponseBody,
        observed_rows: Option<u32>,
    ) -> Self {
        let body_bytes = body.byte_len();
        Self {
            metadata: RuntimeSaasResponseMetadata::new(status, headers, body_bytes, observed_rows),
            body,
        }
    }

    pub fn validate_caps(&self, caps: SaasResponseCaps) -> Result<(), RuntimeSaasTransportError> {
        self.metadata.validate_caps(caps)
    }
}

#[async_trait]
pub trait RuntimeSaasRequestExecutor: Send + Sync {
    type Error: Send + Sync + 'static;

    async fn execute_saas_request(
        &self,
        request: RuntimeSaasRequest,
        user: &UserAuth,
    ) -> Result<RuntimeSaasResponse, Self::Error>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeSaasHttpRequest {
    pub method: SaasHttpMethod,
    pub url: Url,
    pub headers: BTreeMap<String, String>,
    pub body: RuntimeSaasHttpRequestBody,
    pub timeout_ms: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeSaasHttpRequestBody {
    Empty,
    Json(Value),
    Text {
        content_type: String,
        value: String,
    },
    Bytes {
        content_type: String,
        value: Vec<u8>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeSaasHttpResponse {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub content_type: Option<String>,
    pub body: Vec<u8>,
}

#[async_trait]
pub trait RuntimeSaasHttpTransport: Send + Sync {
    async fn send(
        &self,
        request: RuntimeSaasHttpRequest,
    ) -> Result<RuntimeSaasHttpResponse, RuntimeSaasExecutionError>;
}

#[derive(Clone)]
pub struct ReqwestSaasHttpTransport {
    client: reqwest::Client,
}

impl ReqwestSaasHttpTransport {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

impl Default for ReqwestSaasHttpTransport {
    fn default() -> Self {
        Self::new(reqwest::Client::new())
    }
}

#[async_trait]
impl RuntimeSaasHttpTransport for ReqwestSaasHttpTransport {
    async fn send(
        &self,
        request: RuntimeSaasHttpRequest,
    ) -> Result<RuntimeSaasHttpResponse, RuntimeSaasExecutionError> {
        let mut builder = self
            .client
            .request(reqwest_method(request.method), request.url)
            .timeout(Duration::from_millis(request.timeout_ms));

        for (name, value) in &request.headers {
            builder = builder.header(name, value);
        }

        builder = match request.body {
            RuntimeSaasHttpRequestBody::Empty => builder,
            RuntimeSaasHttpRequestBody::Json(value) => builder.json(&value),
            RuntimeSaasHttpRequestBody::Text {
                content_type,
                value,
            } => builder.header("content-type", content_type).body(value),
            RuntimeSaasHttpRequestBody::Bytes {
                content_type,
                value,
            } => builder.header("content-type", content_type).body(value),
        };

        let response = builder
            .send()
            .await
            .map_err(|err| RuntimeSaasExecutionError::Http(err.to_string()))?;
        let status = response.status().as_u16();
        let headers = headers_to_map(response.headers());
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let body = response
            .bytes()
            .await
            .map_err(|err| RuntimeSaasExecutionError::Http(err.to_string()))?
            .to_vec();

        Ok(RuntimeSaasHttpResponse {
            status,
            headers,
            content_type,
            body,
        })
    }
}

pub type ReqwestRuntimeSaasRequestExecutor =
    RuntimeHttpSaasRequestExecutor<ReqwestSaasHttpTransport>;

#[derive(Clone)]
pub struct RuntimeHttpSaasRequestExecutor<T = ReqwestSaasHttpTransport>
where
    T: RuntimeSaasHttpTransport,
{
    transport: Arc<T>,
    bearer_token: Option<SecretString>,
}

impl RuntimeHttpSaasRequestExecutor<ReqwestSaasHttpTransport> {
    pub fn reqwest() -> Self {
        Self::new(ReqwestSaasHttpTransport::default())
    }
}

impl<T> RuntimeHttpSaasRequestExecutor<T>
where
    T: RuntimeSaasHttpTransport,
{
    pub fn new(transport: T) -> Self {
        Self {
            transport: Arc::new(transport),
            bearer_token: None,
        }
    }

    pub fn with_bearer_token(mut self, token: impl Into<String>) -> Self {
        self.bearer_token = Some(SecretString::new(token));
        self
    }

    fn build_http_request(
        &self,
        request: RuntimeSaasRequest,
    ) -> Result<RuntimeSaasHttpRequest, RuntimeSaasExecutionError> {
        let url = request.url()?;
        let mut headers = request.plan.headers.clone();
        if headers
            .keys()
            .any(|name| name.eq_ignore_ascii_case("authorization"))
        {
            return Err(RuntimeSaasExecutionError::InvalidRequestPlan(
                "SaaS request plans must not include Authorization; credentials are supplied by the runtime executor".to_string(),
            ));
        }
        if let Some(token) = &self.bearer_token {
            headers.insert(
                "authorization".to_string(),
                format!("Bearer {}", token.expose_secret()),
            );
        }
        let timeout_ms = request.timeout_ms();
        let method = request.plan.method;
        let body = request.plan.body;

        Ok(RuntimeSaasHttpRequest {
            method,
            url,
            headers,
            body: runtime_http_body(body)?,
            timeout_ms,
        })
    }
}

#[async_trait]
impl<T> RuntimeSaasRequestExecutor for RuntimeHttpSaasRequestExecutor<T>
where
    T: RuntimeSaasHttpTransport,
{
    type Error = RuntimeSaasExecutionError;

    async fn execute_saas_request(
        &self,
        request: RuntimeSaasRequest,
        _user: &UserAuth,
    ) -> Result<RuntimeSaasResponse, Self::Error> {
        let caps = request.plan.caps;
        let http_request = self.build_http_request(request)?;
        let http_response = self.transport.send(http_request).await?;
        let body = runtime_response_body(&http_response)?;
        let observed_rows = observed_rows(&body);
        let response = RuntimeSaasResponse::new(
            http_response.status,
            http_response.headers,
            body,
            observed_rows,
        );
        response.validate_caps(caps)?;
        Ok(response)
    }
}

fn runtime_http_body(
    body: SaasRequestBody,
) -> Result<RuntimeSaasHttpRequestBody, RuntimeSaasExecutionError> {
    Ok(match body {
        SaasRequestBody::Empty => RuntimeSaasHttpRequestBody::Empty,
        SaasRequestBody::Json(value) => RuntimeSaasHttpRequestBody::Json(value),
        SaasRequestBody::Text {
            content_type,
            value,
        } => RuntimeSaasHttpRequestBody::Text {
            content_type,
            value,
        },
        SaasRequestBody::Bytes { .. } => {
            return Err(RuntimeSaasExecutionError::InvalidRequestPlan(
                "byte request bodies require a materialized payload before execution".to_string(),
            ))
        }
    })
}

fn runtime_response_body(
    response: &RuntimeSaasHttpResponse,
) -> Result<RuntimeSaasResponseBody, RuntimeSaasExecutionError> {
    if response.body.is_empty() {
        return Ok(RuntimeSaasResponseBody::Empty);
    }

    if response
        .content_type
        .as_deref()
        .map(|value| value.to_ascii_lowercase().contains("json"))
        .unwrap_or(false)
    {
        let value = serde_json::from_slice(&response.body).map_err(|err| {
            RuntimeSaasExecutionError::Http(format!("invalid JSON SaaS response body: {err}"))
        })?;
        return Ok(RuntimeSaasResponseBody::Json(value));
    }

    match String::from_utf8(response.body.clone()) {
        Ok(value) => Ok(RuntimeSaasResponseBody::Text(value)),
        Err(_) => Ok(RuntimeSaasResponseBody::Bytes(response.body.clone())),
    }
}

fn observed_rows(body: &RuntimeSaasResponseBody) -> Option<u32> {
    match body {
        RuntimeSaasResponseBody::Json(Value::Array(rows)) => Some(rows.len() as u32),
        RuntimeSaasResponseBody::Json(Value::Object(obj)) => obj
            .get("records")
            .or_else(|| obj.get("items"))
            .and_then(Value::as_array)
            .map(|rows| rows.len() as u32),
        _ => None,
    }
}

fn reqwest_method(method: SaasHttpMethod) -> reqwest::Method {
    match method {
        SaasHttpMethod::Get => reqwest::Method::GET,
        SaasHttpMethod::Post => reqwest::Method::POST,
        SaasHttpMethod::Put => reqwest::Method::PUT,
        SaasHttpMethod::Patch => reqwest::Method::PATCH,
        SaasHttpMethod::Delete => reqwest::Method::DELETE,
    }
}

fn headers_to_map(headers: &HeaderMap) -> BTreeMap<String, String> {
    headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.as_str().to_string(), value.to_string()))
        })
        .collect()
}

fn normalize_base_url(base_url: String) -> Result<String, RuntimeSaasTransportError> {
    let mut url = validate_base_url(&base_url)?;
    url.set_path("");
    url.set_query(None);
    url.set_fragment(None);
    Ok(url.to_string().trim_end_matches('/').to_string())
}

fn validate_base_url(base_url: &str) -> Result<Url, RuntimeSaasTransportError> {
    let url = Url::parse(base_url)
        .map_err(|err| RuntimeSaasTransportError::InvalidEndpoint(err.to_string()))?;

    if url.scheme() != "https" {
        return Err(RuntimeSaasTransportError::InvalidEndpoint(
            "base_url must use https".to_string(),
        ));
    }
    if url.host_str().is_none() {
        return Err(RuntimeSaasTransportError::InvalidEndpoint(
            "base_url must include a host".to_string(),
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(RuntimeSaasTransportError::InvalidEndpoint(
            "base_url must not include credentials".to_string(),
        ));
    }
    if !matches!(url.path(), "" | "/") {
        return Err(RuntimeSaasTransportError::InvalidEndpoint(
            "base_url must not include a path; request plans own paths".to_string(),
        ));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(RuntimeSaasTransportError::InvalidEndpoint(
            "base_url must not include query or fragment".to_string(),
        ));
    }

    Ok(url)
}

fn base_url_origin(base_url: &str) -> Option<String> {
    let url = Url::parse(base_url).ok()?;
    let host = url.host_str()?;
    let port = url
        .port()
        .map(|port| format!(":{port}"))
        .unwrap_or_default();
    Some(format!("{}://{}{}", url.scheme(), host, port))
}

#[cfg(test)]
mod tests {
    use super::*;
    use appfw_saas_core::{SaasHttpMethod, SaasRequestBody, SaasTransportProtocol};
    use std::sync::Mutex;

    fn endpoint() -> RuntimeSaasEndpoint {
        RuntimeSaasEndpoint::new(
            FrameworkProvider::Salesforce,
            "salesforce_primary",
            "https://example.my.salesforce.com/",
        )
        .expect("valid endpoint")
    }

    fn user() -> UserAuth {
        UserAuth::human(
            "tenant-1",
            "casey",
            "UTC",
            vec!["admin".to_string()],
            vec!["appfw:data.read".to_string()],
            "bearer-token",
        )
    }

    #[test]
    fn endpoint_accepts_external_api_origins_only() {
        let endpoint = endpoint();
        assert_eq!(endpoint.provider, FrameworkProvider::Salesforce);
        assert_eq!(endpoint.base_url, "https://example.my.salesforce.com");
        assert_eq!(
            RuntimeSaasEndpoint::new(
                FrameworkProvider::Anaplan,
                "anaplan_primary",
                "https://api.anaplan.com"
            )
            .expect("anaplan endpoint")
            .provider,
            FrameworkProvider::Anaplan
        );
        assert_eq!(
            RuntimeSaasEndpoint::new(
                FrameworkProvider::OracleFinancials,
                "oracle_financials_primary",
                "https://oracle.example.com"
            )
            .expect("oracle endpoint")
            .provider,
            FrameworkProvider::OracleFinancials
        );

        assert_eq!(
            RuntimeSaasEndpoint::new(
                FrameworkProvider::Postgres,
                "pg_primary",
                "https://db.example.com"
            )
            .unwrap_err(),
            RuntimeSaasTransportError::NonExternalApiProvider
        );

        for invalid in [
            "http://example.service-now.com",
            "https://user:pass@example.service-now.com",
            "https://example.service-now.com/api",
            "https://example.service-now.com?token=secret",
        ] {
            assert!(RuntimeSaasEndpoint::new(
                FrameworkProvider::ServiceNow,
                "servicenow_primary",
                invalid
            )
            .is_err());
        }
    }

    #[test]
    fn request_envelope_builds_url_from_server_origin_and_plan_path() {
        let plan = SaasRequestPlan::new(
            "salesforce.health.fetch_limits",
            SaasTransportProtocol::RestJson,
            SaasHttpMethod::Get,
            "/services/data/v67.0/limits",
        )
        .with_query("client", "appfw");

        let request = RuntimeSaasRequest::new(endpoint(), plan).expect("valid request");

        assert_eq!(
            request.url().expect("url").as_str(),
            "https://example.my.salesforce.com/services/data/v67.0/limits?client=appfw"
        );
        assert_eq!(request.timeout_ms(), 2_000);
        assert_eq!(
            request.redacted_summary()["endpoint"]["base_url_origin"],
            "https://example.my.salesforce.com"
        );
    }

    #[test]
    fn request_envelope_rejects_raw_external_plan_url() {
        let plan = SaasRequestPlan::new(
            "salesforce.health.fetch_limits",
            SaasTransportProtocol::RestJson,
            SaasHttpMethod::Get,
            "https://evil.example/services/data/v67.0/limits",
        );

        let error = RuntimeSaasRequest::new(endpoint(), plan).unwrap_err();

        assert!(matches!(
            error,
            RuntimeSaasTransportError::InvalidRequestPlan(_)
        ));
    }

    #[test]
    fn response_metadata_extracts_rate_limit_and_enforces_caps() {
        let response = RuntimeSaasResponse::new(
            200,
            BTreeMap::from([
                (
                    "Authorization".to_string(),
                    "Bearer secret-token".to_string(),
                ),
                ("Retry-After".to_string(), "5".to_string()),
                ("Set-Cookie".to_string(), "session=secret".to_string()),
                (
                    "Sforce-Limit-Info".to_string(),
                    "api-usage=100/100".to_string(),
                ),
            ]),
            RuntimeSaasResponseBody::Json(json!([{ "id": "001" }])),
            Some(1),
        );

        assert!(response.metadata.rate_limit.is_limited());
        assert_eq!(response.metadata.rate_limit.retry_after_ms, Some(5_000));
        assert_eq!(
            response.metadata.header_names,
            vec![
                "Authorization",
                "Retry-After",
                "Set-Cookie",
                "Sforce-Limit-Info"
            ]
        );
        let serialized_metadata =
            serde_json::to_string(&response.metadata).expect("metadata serializes");
        assert!(serialized_metadata.contains("Set-Cookie"));
        assert!(!serialized_metadata.contains("secret-token"));
        assert!(!serialized_metadata.contains("session=secret"));
        assert!(!response
            .metadata
            .redacted_summary()
            .to_string()
            .contains("secret-token"));
        response
            .validate_caps(SaasResponseCaps {
                max_body_bytes: 64,
                max_rows: 1,
            })
            .expect("within caps");

        assert!(response
            .validate_caps(SaasResponseCaps {
                max_body_bytes: 1,
                max_rows: 1,
            })
            .unwrap_err()
            .to_string()
            .contains("body_bytes"));
        assert!(response
            .validate_caps(SaasResponseCaps {
                max_body_bytes: 64,
                max_rows: 0,
            })
            .is_err());
    }

    #[test]
    fn executor_trait_keeps_user_context_but_not_raw_token_in_summary() {
        struct EchoExecutor;

        #[async_trait]
        impl RuntimeSaasRequestExecutor for EchoExecutor {
            type Error = RuntimeSaasTransportError;

            async fn execute_saas_request(
                &self,
                request: RuntimeSaasRequest,
                user: &UserAuth,
            ) -> Result<RuntimeSaasResponse, Self::Error> {
                assert_eq!(user.tenant_id, "tenant-1");
                request.url()?;
                Ok(RuntimeSaasResponse::new(
                    200,
                    BTreeMap::new(),
                    RuntimeSaasResponseBody::Empty,
                    Some(0),
                ))
            }
        }

        let plan = SaasRequestPlan::new(
            "workday.hcm.get_workers_page",
            SaasTransportProtocol::SoapXml,
            SaasHttpMethod::Post,
            "/ccx/service/acme/Human_Resources/v46.1",
        )
        .with_body(SaasRequestBody::Text {
            content_type: "text/xml".to_string(),
            value: "<soapenv:Envelope/>".to_string(),
        });
        let request = RuntimeSaasRequest::new(
            RuntimeSaasEndpoint::new(
                FrameworkProvider::Workday,
                "workday_primary",
                "https://wd2-impl-services1.workday.com",
            )
            .expect("endpoint"),
            plan,
        )
        .expect("request");
        let summary = request.redacted_summary().to_string();
        assert!(!summary.contains("bearer-token"));

        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let response = runtime
            .block_on(EchoExecutor.execute_saas_request(request, &user()))
            .expect("response");
        assert_eq!(response.metadata.status, 200);
    }

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

    #[test]
    fn http_executor_builds_request_injects_bearer_and_parses_json_response() {
        let transport = Arc::new(FakeHttpTransport::with_response(RuntimeSaasHttpResponse {
            status: 200,
            headers: BTreeMap::from([(
                "Sforce-Limit-Info".to_string(),
                "api-usage=10/100".to_string(),
            )]),
            content_type: Some("application/json; charset=utf-8".to_string()),
            body: br#"{"records":[{"id":"001"},{"id":"002"}]}"#.to_vec(),
        }));
        let executor =
            RuntimeHttpSaasRequestExecutor::new(transport.clone()).with_bearer_token("m2m-token");
        let plan = SaasRequestPlan::new(
            "salesforce.account.list",
            SaasTransportProtocol::RestJson,
            SaasHttpMethod::Post,
            "/services/data/v67.0/query",
        )
        .with_query("q", "SELECT Id FROM Account")
        .with_header("x-appfw-test", "true")
        .with_body(SaasRequestBody::Json(json!({ "page": 1 })))
        .with_caps(SaasResponseCaps {
            max_body_bytes: 128,
            max_rows: 2,
        });
        let request = RuntimeSaasRequest::new(endpoint(), plan).expect("request");

        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let response = runtime
            .block_on(executor.execute_saas_request(request, &user()))
            .expect("response");

        let captured = transport.last_request();
        assert_eq!(captured.method, SaasHttpMethod::Post);
        assert_eq!(
            captured.url.as_str(),
            "https://example.my.salesforce.com/services/data/v67.0/query?q=SELECT+Id+FROM+Account"
        );
        assert_eq!(
            captured.headers.get("authorization").map(String::as_str),
            Some("Bearer m2m-token")
        );
        assert_eq!(
            captured.headers.get("x-appfw-test").map(String::as_str),
            Some("true")
        );
        assert!(matches!(
            captured.body,
            RuntimeSaasHttpRequestBody::Json(Value::Object(_))
        ));
        assert_eq!(response.metadata.status, 200);
        assert_eq!(response.metadata.observed_rows, Some(2));
        assert_eq!(
            response.metadata.rate_limit.api_usage.unwrap().remaining(),
            90
        );
        assert!(matches!(response.body, RuntimeSaasResponseBody::Json(_)));
    }

    #[test]
    fn http_executor_rejects_plan_authorization_header() {
        let transport = Arc::new(FakeHttpTransport::default());
        let executor = RuntimeHttpSaasRequestExecutor::new(transport);
        let plan = SaasRequestPlan::new(
            "salesforce.account.list",
            SaasTransportProtocol::RestJson,
            SaasHttpMethod::Get,
            "/services/data/v67.0/query",
        )
        .with_header("Authorization", "Bearer user-supplied");
        let request = RuntimeSaasRequest::new(endpoint(), plan).expect("request");

        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let err = runtime
            .block_on(executor.execute_saas_request(request, &user()))
            .expect_err("authorization header should be rejected");

        assert!(matches!(
            err,
            RuntimeSaasExecutionError::InvalidRequestPlan(message)
                if message.contains("Authorization")
        ));
    }

    #[test]
    fn http_executor_enforces_response_caps() {
        let transport = Arc::new(FakeHttpTransport::with_response(RuntimeSaasHttpResponse {
            status: 200,
            headers: BTreeMap::new(),
            content_type: Some("application/json".to_string()),
            body: br#"[{"id":"001"},{"id":"002"}]"#.to_vec(),
        }));
        let executor = RuntimeHttpSaasRequestExecutor::new(transport);
        let plan = SaasRequestPlan::new(
            "salesforce.account.list",
            SaasTransportProtocol::RestJson,
            SaasHttpMethod::Get,
            "/services/data/v67.0/query",
        )
        .with_caps(SaasResponseCaps {
            max_body_bytes: 128,
            max_rows: 1,
        });
        let request = RuntimeSaasRequest::new(endpoint(), plan).expect("request");

        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let err = runtime
            .block_on(executor.execute_saas_request(request, &user()))
            .expect_err("row cap should reject response");

        assert!(err.to_string().contains("observed_rows"));
    }
}
