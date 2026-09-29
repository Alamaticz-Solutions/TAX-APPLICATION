use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context, Result};
use appfw_codegen::schema_route_segment;
use async_graphql::{Request, Variables};
use reqwest::header::AUTHORIZATION;
use serde_json::Value;

use super::{auth::AuthTokenProvider, config::ApiTestConfig};

const REQUEST_ID_HEADER: &str = "x-request-id";
const CORRELATION_ID_HEADER: &str = "x-correlation-id";
static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug)]
pub struct GraphqlClient {
    config: ApiTestConfig,
    auth: AuthTokenProvider,
    http: reqwest::Client,
}

impl GraphqlClient {
    pub fn new(config: ApiTestConfig) -> Result<Self> {
        let auth = AuthTokenProvider::new(&config)?;
        let http = reqwest::Client::builder()
            .no_proxy()
            .build()
            .context("failed to build API test HTTP client")?;

        Ok(Self { config, auth, http })
    }

    pub async fn execute(
        &self,
        auth_token_name: &str,
        schema_name: &str,
        gql_request: &str,
        variables: Option<Value>,
    ) -> Result<Value> {
        let mut request = Request::new(gql_request);
        if let Some(vars) = variables {
            request = request.variables(Variables::from_json(vars));
        }

        let url = format!(
            "{}/{}",
            self.config.base_url,
            schema_route_segment(schema_name)
        );
        let request_ids = RequestIds::new(schema_name);
        let mut builder = self
            .http
            .post(&url)
            .header("timezone", self.config.timezone.as_str())
            .header(REQUEST_ID_HEADER, request_ids.request_id.as_str())
            .header(CORRELATION_ID_HEADER, request_ids.correlation_id.as_str())
            .json(&request);

        if let Some(auth_header) = self.auth.authorization_header(auth_token_name)? {
            builder = builder.header(AUTHORIZATION, auth_header);
        }

        let response = builder.send().await.with_context(|| {
            format!(
                "failed to send GraphQL request to {url} ({})",
                request_ids.describe()
            )
        })?;
        let status = response.status();
        let response_ids = ResponseIds::from_headers(response.headers());
        let body = response.text().await.with_context(|| {
            format!(
                "failed to read GraphQL response from {url} ({}, {})",
                request_ids.describe(),
                response_ids.describe()
            )
        })?;

        if !status.is_success() {
            bail!(
                "GraphQL request to {url} returned {status} ({}, {}): {body}",
                request_ids.describe(),
                response_ids.describe()
            );
        }

        serde_json::from_str(&body).with_context(|| {
            format!(
                "failed to parse GraphQL JSON response from {url} ({}, {}): {body}",
                request_ids.describe(),
                response_ids.describe()
            )
        })
    }
}

#[derive(Clone, Debug)]
struct RequestIds {
    request_id: String,
    correlation_id: String,
}

impl RequestIds {
    fn new(schema_name: &str) -> Self {
        let counter = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or_default();
        let safe_schema = schema_name
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
            .collect::<String>();
        let request_id = format!("api-test-{safe_schema}-{now_ms}-{counter}");
        Self {
            request_id: request_id.clone(),
            correlation_id: request_id,
        }
    }

    fn describe(&self) -> String {
        format!(
            "request_id={}, correlation_id={}",
            self.request_id, self.correlation_id
        )
    }
}

#[derive(Clone, Debug)]
struct ResponseIds {
    request_id: Option<String>,
    correlation_id: Option<String>,
}

impl ResponseIds {
    fn from_headers(headers: &reqwest::header::HeaderMap) -> Self {
        Self {
            request_id: header_value(headers, REQUEST_ID_HEADER),
            correlation_id: header_value(headers, CORRELATION_ID_HEADER),
        }
    }

    fn describe(&self) -> String {
        format!(
            "response_request_id={}, response_correlation_id={}",
            self.request_id.as_deref().unwrap_or("missing"),
            self.correlation_id.as_deref().unwrap_or("missing")
        )
    }
}

fn header_value(headers: &reqwest::header::HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use appfw_codegen::schema_route_segment;

    #[test]
    fn schema_route_segment_snake_to_kebab() {
        assert_eq!(schema_route_segment("nexus_work"), "nexus-work");
        assert_eq!(schema_route_segment("denovo_workflow"), "denovo-workflow");
        assert_eq!(schema_route_segment("nexus_ix"), "nexus-ix");
    }

    #[test]
    fn schema_route_segment_trims_leading_slash() {
        assert_eq!(schema_route_segment("/nexus_work"), "nexus-work");
        assert_eq!(schema_route_segment("/nexus-work"), "nexus-work");
        assert_eq!(schema_route_segment("/crm"), "crm");
    }

    #[test]
    fn schema_route_segment_keeps_existing_kebab() {
        assert_eq!(schema_route_segment("nexus-work"), "nexus-work");
        assert_eq!(schema_route_segment("foo-bar"), "foo-bar");
        assert_eq!(schema_route_segment("crm"), "crm");
    }

    #[test]
    fn schema_route_segment_collapses_repeated_underscore() {
        assert_eq!(schema_route_segment("nexus__work"), "nexus-work");
        assert_eq!(schema_route_segment("nexus___work"), "nexus-work");
    }

    #[test]
    fn schema_route_segment_digit_bearing_matches_inflector() {
        assert_eq!(schema_route_segment("nexus_work2"), "nexus-work-2");
        assert_eq!(schema_route_segment("schema2_name"), "schema-2-name");
        assert_eq!(schema_route_segment("t12m_ebitda"), "t-1-2m-ebitda");
        assert_eq!(schema_route_segment("q1_revenue"), "q-1-revenue");
        assert_eq!(schema_route_segment("nexus_work_v2"), "nexus-work-v-2");
    }

    #[test]
    fn schema_route_segment_mixed_case_matches_inflector() {
        assert_eq!(schema_route_segment("NexusWork"), "nexus-work");
        assert_eq!(schema_route_segment("NEXUS_WORK"), "nexus-work");
        assert_eq!(schema_route_segment("Nexus_Work"), "nexus-work");
        assert_eq!(schema_route_segment("nexusWork"), "nexus-work");
        assert_eq!(schema_route_segment("Foo-Bar"), "foo-bar");
        assert_eq!(schema_route_segment("XmlHTTPRequest"), "xml-http-request");
    }
}
