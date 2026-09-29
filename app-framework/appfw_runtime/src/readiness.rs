use std::{collections::BTreeSet, env, time::Instant};

use async_trait::async_trait;
use axum::{
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Serialize;

use crate::observability::{redact_diagnostic_text, MetricsRegistry, RequestContext};

#[async_trait]
pub trait RuntimeReadinessProbe: Clone + Send + Sync + 'static {
    async fn run(&self) -> RuntimeHealthCheck;

    fn record_pool_stats(&self, metrics: &MetricsRegistry);
}

#[derive(Clone)]
pub struct RuntimeReadinessState<C> {
    checks: Vec<RuntimeHealthCheck>,
    provider_checks: Vec<C>,
}

impl<C> RuntimeReadinessState<C> {
    pub fn new(checks: Vec<RuntimeHealthCheck>) -> Self {
        Self {
            checks,
            provider_checks: Vec::new(),
        }
    }

    pub fn with_provider_check(mut self, check: C) -> Self {
        self.provider_checks.push(check);
        self
    }
}

impl<C> RuntimeReadinessState<C>
where
    C: RuntimeReadinessProbe,
{
    pub async fn checks(&self) -> Vec<RuntimeHealthCheck> {
        let mut checks = self.checks.clone();
        for provider_check in &self.provider_checks {
            checks.push(provider_check.run().await);
        }
        checks
    }

    pub fn record_provider_pool_stats(&self, metrics: &MetricsRegistry) {
        for provider_check in &self.provider_checks {
            provider_check.record_pool_stats(metrics);
        }
    }

    pub async fn is_ready(&self) -> bool {
        self.checks().await.iter().all(RuntimeHealthCheck::is_pass)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct RuntimeHealthCheck {
    name: String,
    status: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    schema: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration_ms: Option<u64>,
}

impl RuntimeHealthCheck {
    pub fn pass(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: "pass".to_string(),
            message: message.into(),
            provider: None,
            data_source: None,
            schema: None,
            duration_ms: None,
        }
    }

    pub fn fail(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: "fail".to_string(),
            message: redact_diagnostic_text(message.into()),
            provider: None,
            data_source: None,
            schema: None,
            duration_ms: None,
        }
    }

    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    pub fn with_data_source(mut self, data_source: impl Into<String>) -> Self {
        self.data_source = Some(data_source.into());
        self
    }

    pub fn with_schema(mut self, schema: impl Into<String>) -> Self {
        self.schema = Some(schema.into());
        self
    }

    pub fn with_duration_ms(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    pub fn is_pass(&self) -> bool {
        self.status == "pass"
    }
}

#[derive(Clone)]
struct RuntimeHealthState<C> {
    version: String,
    started_at: Instant,
    readiness: RuntimeReadinessState<C>,
}

#[derive(Clone, Debug, Serialize)]
struct RuntimeHealthResponse {
    status: String,
    service: String,
    version: String,
    uptime_seconds: u64,
    request_context: RequestContext,
    summary: RuntimeHealthSummary,
    checks: Vec<RuntimeHealthCheck>,
}

#[derive(Clone, Debug, Serialize)]
struct RuntimeHealthSummary {
    total_checks: usize,
    passing_checks: usize,
    failing_checks: usize,
    data_source_checks: usize,
    failing_data_sources: Vec<String>,
}

pub async fn runtime_info_routes<C>(
    _parent: Router,
    metrics: MetricsRegistry,
    readiness: RuntimeReadinessState<C>,
) -> Router
where
    C: RuntimeReadinessProbe,
{
    let version = load_version();
    let info_version = version.clone();
    let prometheus_metrics = metrics.clone();
    let json_metrics = metrics.clone();
    let health = RuntimeHealthState {
        version,
        started_at: Instant::now(),
        readiness,
    };

    Router::new()
        .route(
            "/info",
            get(move || {
                let version = info_version.clone();
                async move { version }
            }),
        )
        .route(
            "/health",
            get({
                let health = health.clone();
                move |headers: HeaderMap| readiness_probe(health.clone(), headers)
            }),
        )
        .route(
            "/health/ready",
            get({
                let health = health.clone();
                move |headers: HeaderMap| readiness_probe(health.clone(), headers)
            }),
        )
        .route(
            "/readyz",
            get({
                let health = health.clone();
                move |headers: HeaderMap| readiness_probe(health.clone(), headers)
            }),
        )
        .route(
            "/health/live",
            get({
                let health = health.clone();
                move |headers: HeaderMap| liveness_probe(health.clone(), headers)
            }),
        )
        .route(
            "/livez",
            get({
                let health = health.clone();
                move |headers: HeaderMap| liveness_probe(health.clone(), headers)
            }),
        )
        .route(
            "/metrics",
            get({
                let health = health.clone();
                move || {
                    let metrics = prometheus_metrics.clone();
                    let readiness = health.readiness.clone();
                    async move {
                        readiness.record_provider_pool_stats(&metrics);
                        (
                            [(
                                header::CONTENT_TYPE,
                                "text/plain; version=0.0.4; charset=utf-8",
                            )],
                            metrics.to_prometheus(),
                        )
                            .into_response()
                    }
                }
            }),
        )
        .route(
            "/metrics.json",
            get({
                let health = health.clone();
                move || {
                    let metrics = json_metrics.clone();
                    let readiness = health.readiness.clone();
                    async move {
                        readiness.record_provider_pool_stats(&metrics);
                        Json(metrics.snapshot())
                    }
                }
            }),
        )
}

async fn liveness_probe<C>(health: RuntimeHealthState<C>, headers: HeaderMap) -> impl IntoResponse
where
    C: RuntimeReadinessProbe,
{
    let request_context = RequestContext::from_headers(&headers);
    (
        StatusCode::OK,
        Json(health.liveness_response(request_context)),
    )
}

async fn readiness_probe<C>(health: RuntimeHealthState<C>, headers: HeaderMap) -> impl IntoResponse
where
    C: RuntimeReadinessProbe,
{
    let request_context = RequestContext::from_headers(&headers);
    let response = health.readiness_response(request_context).await;
    let status = if response.status == "ready" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status, Json(response))
}

impl<C> RuntimeHealthState<C>
where
    C: RuntimeReadinessProbe,
{
    fn liveness_response(&self, request_context: RequestContext) -> RuntimeHealthResponse {
        let checks = vec![RuntimeHealthCheck::pass("process", "serving HTTP requests")];
        let summary = health_summary(&checks);
        RuntimeHealthResponse {
            status: "live".to_string(),
            service: "backend".to_string(),
            version: self.version.clone(),
            uptime_seconds: self.started_at.elapsed().as_secs(),
            request_context,
            summary,
            checks,
        }
    }

    async fn readiness_response(&self, request_context: RequestContext) -> RuntimeHealthResponse {
        let checks = self.readiness.checks().await;
        let is_ready = checks.iter().all(RuntimeHealthCheck::is_pass);
        let summary = health_summary(&checks);
        RuntimeHealthResponse {
            status: if is_ready {
                "ready".to_string()
            } else {
                "not_ready".to_string()
            },
            service: "backend".to_string(),
            version: self.version.clone(),
            uptime_seconds: self.started_at.elapsed().as_secs(),
            request_context,
            summary,
            checks,
        }
    }
}

fn health_summary(checks: &[RuntimeHealthCheck]) -> RuntimeHealthSummary {
    let total_checks = checks.len();
    let passing_checks = checks.iter().filter(|check| check.is_pass()).count();
    let data_source_checks = checks
        .iter()
        .filter(|check| check.data_source.is_some())
        .count();
    let failing_data_sources = checks
        .iter()
        .filter(|check| !check.is_pass())
        .filter_map(|check| check.data_source.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    RuntimeHealthSummary {
        total_checks,
        passing_checks,
        failing_checks: total_checks.saturating_sub(passing_checks),
        data_source_checks,
        failing_data_sources,
    }
}

fn load_version() -> String {
    match env::var("VERSION") {
        Ok(v) => v.to_string(),
        Err(_) => "Error loading VERSION env variable".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    #[derive(Clone)]
    struct NoopProbe;

    #[async_trait]
    impl RuntimeReadinessProbe for NoopProbe {
        async fn run(&self) -> RuntimeHealthCheck {
            RuntimeHealthCheck::pass("noop", "noop")
        }

        fn record_pool_stats(&self, _metrics: &MetricsRegistry) {}
    }

    async fn status(router: Router, path: &str) -> StatusCode {
        router
            .oneshot(
                Request::builder()
                    .uri(path)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response")
            .status()
    }

    #[tokio::test]
    async fn runtime_info_routes_report_ready_when_checks_pass() {
        let readiness = RuntimeReadinessState::<NoopProbe>::new(vec![RuntimeHealthCheck::pass(
            "configuration",
            "loaded",
        )]);
        let router = runtime_info_routes(
            Router::new(),
            MetricsRegistry::new("test", "test"),
            readiness,
        )
        .await;

        assert_eq!(
            status(router.clone(), "/health/ready").await,
            StatusCode::OK
        );
        assert_eq!(status(router.clone(), "/livez").await, StatusCode::OK);
        assert_eq!(status(router, "/metrics").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn runtime_info_routes_report_unavailable_when_checks_fail() {
        let readiness = RuntimeReadinessState::<NoopProbe>::new(vec![RuntimeHealthCheck::fail(
            "configuration",
            "missing",
        )]);
        let router = runtime_info_routes(
            Router::new(),
            MetricsRegistry::new("test", "test"),
            readiness,
        )
        .await;

        assert_eq!(
            status(router, "/readyz").await,
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[test]
    fn readiness_failures_redact_sensitive_messages() {
        let check = RuntimeHealthCheck::fail(
            "provider:crm_primary",
            "provider readiness probe failed for postgres://svc:secret@db password=hunter2 Authorization: Bearer jwt.value",
        );

        assert!(check.message.contains("[REDACTED]"));
        assert!(!check.message.contains("svc:secret"));
        assert!(!check.message.contains("hunter2"));
        assert!(!check.message.contains("jwt.value"));
    }

    #[tokio::test]
    async fn health_responses_include_request_context() {
        let health = RuntimeHealthState {
            version: "1.2.3".to_string(),
            started_at: Instant::now(),
            readiness: RuntimeReadinessState::<NoopProbe>::new(vec![RuntimeHealthCheck::pass(
                "configuration",
                "loaded",
            )]),
        };
        let request_context = RequestContext {
            request_id: "request-123".to_string(),
            correlation_id: "correlation-456".to_string(),
        };

        let live = health.liveness_response(request_context.clone());
        let ready = health.readiness_response(request_context.clone()).await;

        assert_eq!(live.request_context.request_id, "request-123");
        assert_eq!(live.request_context.correlation_id, "correlation-456");
        assert_eq!(ready.request_context.request_id, "request-123");
        assert_eq!(ready.request_context.correlation_id, "correlation-456");
    }

    #[tokio::test]
    async fn readiness_response_summarizes_check_results_and_data_sources() {
        let health = RuntimeHealthState {
            version: "1.2.3".to_string(),
            started_at: Instant::now(),
            readiness: RuntimeReadinessState::<NoopProbe>::new(vec![
                RuntimeHealthCheck::pass("configuration", "loaded"),
                RuntimeHealthCheck::fail("provider:crm_primary", "connection refused")
                    .with_provider("postgres")
                    .with_data_source("crm_primary"),
                RuntimeHealthCheck::fail("provider:crm_replica", "timeout")
                    .with_provider("postgres")
                    .with_data_source("crm_primary"),
            ]),
        };
        let request_context = RequestContext {
            request_id: "request-123".to_string(),
            correlation_id: "correlation-456".to_string(),
        };

        let ready = health.readiness_response(request_context).await;

        assert_eq!(ready.status, "not_ready");
        assert_eq!(ready.summary.total_checks, 3);
        assert_eq!(ready.summary.passing_checks, 1);
        assert_eq!(ready.summary.failing_checks, 2);
        assert_eq!(ready.summary.data_source_checks, 2);
        assert_eq!(ready.summary.failing_data_sources, vec!["crm_primary"]);
    }
}
