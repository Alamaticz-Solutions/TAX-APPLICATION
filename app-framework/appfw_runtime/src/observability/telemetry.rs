//! Tracing, correlation IDs, OpenTelemetry setup, and diagnostic redaction.
//!
//! This module is deliberately request-centric: every incoming request gets a
//! stable request ID and correlation ID, and those IDs are propagated through
//! spans, response headers, GraphQL extensions, and sanitized error messages.

#[cfg(feature = "http")]
use std::borrow::Cow;
use std::{env, error::Error, sync::OnceLock, time::Duration};

#[cfg(feature = "http")]
use async_graphql::{ErrorExtensionValues, ServerError, Value as GraphqlValue};
#[cfg(feature = "http")]
use axum::{
    extract::MatchedPath,
    http::{HeaderMap, HeaderName, HeaderValue, Request},
    middleware::Next,
    response::Response,
};
#[cfg(feature = "http")]
use opentelemetry::propagation::{Extractor, Injector};
use opentelemetry::{global, trace::TracerProvider as _, KeyValue};
use opentelemetry_otlp::{MetricExporter, SpanExporter};
use opentelemetry_sdk::{
    metrics::SdkMeterProvider, propagation::TraceContextPropagator, trace::SdkTracerProvider,
    Resource,
};
use regex::Regex;
use serde::Serialize;
use serde_json::{Map as JsonMap, Value as JsonValue};
#[cfg(feature = "http")]
use tracing::Span;
#[cfg(feature = "http")]
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
#[cfg(feature = "http")]
use uuid::Uuid;

pub const REQUEST_ID_HEADER_NAME: &str = "x-request-id";
pub const CORRELATION_ID_HEADER_NAME: &str = "x-correlation-id";

const DEFAULT_TRACING_FILTER: &str = concat!(
    "backend=info,",
    "appfw_runtime=info,",
    "appfw_provider_postgres=info,",
    "appfw_provider_mongo=info,",
    "appfw_provider_mssql=info,",
    "appfw_provider_snowflake=info,",
    "tower_http=info"
);
const OTEL_EXPORT_ENV_VARS: [&str; 8] = [
    "APP_OTEL_ENABLED",
    "OTEL_SERVICE_NAME",
    "APP_SERVICE_NAME",
    "ENV_NAME",
    "OTEL_EXPORTER_OTLP_ENDPOINT",
    "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT",
    "OTEL_EXPORTER_OTLP_METRICS_ENDPOINT",
    "OTEL_EXPORTER_OTLP_PROTOCOL",
];
const OTEL_RESOURCE_ATTRIBUTES: [&str; 2] = ["service.name", "deployment.environment.name"];
const OTEL_CORRELATION_ATTRIBUTES: [&str; 2] = ["http.request_id", "app.correlation_id"];
const OTEL_PROPAGATION_HEADERS: [&str; 4] = [
    "traceparent",
    "tracestate",
    REQUEST_ID_HEADER_NAME,
    CORRELATION_ID_HEADER_NAME,
];

// Request context is stored task-locally so async code below route handlers can
// attach IDs to provider logs and diagnostics without plumbing parameters
// through every intermediate API.
tokio::task_local! {
    static CURRENT_REQUEST_CONTEXT: RequestContext;
}

#[derive(Clone, Debug, Serialize)]
pub struct RequestContext {
    pub request_id: String,
    pub correlation_id: String,
}

impl RequestContext {
    pub fn new(request_id: impl Into<String>, correlation_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            correlation_id: correlation_id.into(),
        }
    }

    #[cfg(feature = "http")]
    pub fn from_headers(headers: &HeaderMap) -> Self {
        let request_id = header_value(headers, REQUEST_ID_HEADER_NAME)
            .or_else(|| header_value(headers, CORRELATION_ID_HEADER_NAME))
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let correlation_id =
            header_value(headers, CORRELATION_ID_HEADER_NAME).unwrap_or_else(|| request_id.clone());

        Self {
            request_id,
            correlation_id,
        }
    }
}

pub fn current_request_context() -> Option<RequestContext> {
    CURRENT_REQUEST_CONTEXT.try_with(Clone::clone).ok()
}

#[derive(Clone, Debug)]
pub struct OtelExportConfig {
    pub enabled: bool,
    pub service_name: String,
    pub environment: String,
    pub endpoint: Option<String>,
    pub traces_endpoint: Option<String>,
    pub metrics_endpoint: Option<String>,
    pub protocol: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct OtelExportContract {
    pub env_vars: Vec<&'static str>,
    pub resource_attributes: Vec<&'static str>,
    pub correlation_attributes: Vec<&'static str>,
    pub propagation_headers: Vec<&'static str>,
}

pub fn otel_export_contract() -> OtelExportContract {
    OtelExportContract {
        env_vars: OTEL_EXPORT_ENV_VARS.to_vec(),
        resource_attributes: OTEL_RESOURCE_ATTRIBUTES.to_vec(),
        correlation_attributes: OTEL_CORRELATION_ATTRIBUTES.to_vec(),
        propagation_headers: OTEL_PROPAGATION_HEADERS.to_vec(),
    }
}

impl OtelExportConfig {
    pub fn from_env() -> Self {
        Self::from_lookup(|name| env::var(name).ok())
    }

    fn from_lookup<F>(mut lookup: F) -> Self
    where
        F: FnMut(&str) -> Option<String>,
    {
        let enabled = env_bool_from_lookup(&mut lookup, "APP_OTEL_ENABLED", false);
        let service_name = env_value_from_lookup(&mut lookup, "OTEL_SERVICE_NAME")
            .or_else(|| env_value_from_lookup(&mut lookup, "APP_SERVICE_NAME"))
            .unwrap_or_else(|| "backend".to_string());
        let environment =
            env_value_from_lookup(&mut lookup, "ENV_NAME").unwrap_or_else(|| "local".to_string());

        Self {
            enabled,
            service_name,
            environment,
            endpoint: env_value_from_lookup(&mut lookup, "OTEL_EXPORTER_OTLP_ENDPOINT"),
            traces_endpoint: env_value_from_lookup(
                &mut lookup,
                "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT",
            ),
            metrics_endpoint: env_value_from_lookup(
                &mut lookup,
                "OTEL_EXPORTER_OTLP_METRICS_ENDPOINT",
            ),
            protocol: env_value_from_lookup(&mut lookup, "OTEL_EXPORTER_OTLP_PROTOCOL")
                .unwrap_or_else(|| "http/protobuf".to_string()),
        }
    }
}

#[derive(Clone, Debug)]
struct ObservabilityConfig {
    service_name: String,
    environment: String,
    json_logs: bool,
    otel_enabled: bool,
}

#[derive(Default)]
pub struct ObservabilityGuard {
    tracer_provider: Option<SdkTracerProvider>,
    meter_provider: Option<SdkMeterProvider>,
}

impl ObservabilityGuard {
    pub fn shutdown(mut self) {
        self.shutdown_provider();
    }

    fn shutdown_provider(&mut self) {
        if let Some(provider) = self.meter_provider.take() {
            if let Err(error) = provider.shutdown_with_timeout(Duration::from_secs(5)) {
                eprintln!("failed to shutdown OpenTelemetry meter provider: {error}");
            }
        }
        if let Some(provider) = self.tracer_provider.take() {
            if let Err(error) = provider.shutdown_with_timeout(Duration::from_secs(5)) {
                eprintln!("failed to shutdown OpenTelemetry tracer provider: {error}");
            }
        }
    }
}

impl Drop for ObservabilityGuard {
    fn drop(&mut self) {
        self.shutdown_provider();
    }
}

pub fn init_tracing() -> ObservabilityGuard {
    let config = ObservabilityConfig::from_env();

    // OpenTelemetry is opt-in. Local logs remain the fallback so a broken OTLP
    // collector never prevents the backend from starting in development.
    if config.otel_enabled {
        match init_tracing_with_otel(&config) {
            Ok(guard) => return guard,
            Err(error) => {
                eprintln!("failed to initialize OpenTelemetry tracing; falling back to local logs: {error}");
            }
        }
    }

    if let Err(error) = init_local_tracing(&config) {
        eprintln!("failed to initialize tracing subscriber: {error}");
    }
    ObservabilityGuard::default()
}

#[cfg(feature = "http")]
pub fn http_make_span<B>(request: &Request<B>) -> Span {
    let request_context = RequestContext::from_headers(request.headers());
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map(MatchedPath::as_str)
        .unwrap_or_else(|| request.uri().path());
    let user_agent = header_value(request.headers(), "user-agent").unwrap_or_default();

    tracing::info_span!(
        "http.request",
        "otel.kind" = "server",
        "http.request.method" = %request.method(),
        "http.route" = %route,
        "url.path" = %request.uri().path(),
        "http.request_id" = %request_context.request_id,
        "app.correlation_id" = %request_context.correlation_id,
        "user_agent.original" = %user_agent,
    )
}

#[cfg(feature = "http")]
pub async fn trace_context_hook<B>(mut request: Request<B>, next: Next<B>) -> Response {
    // Accept upstream W3C trace context, then publish the request context for
    // downstream async work and response annotations.
    let parent_context = global::get_text_map_propagator(|propagator| {
        propagator.extract(&HeaderExtractor(request.headers()))
    });
    let _ = Span::current().set_parent(parent_context);
    let request_context = RequestContext::from_headers(request.headers());
    request.extensions_mut().insert(request_context.clone());

    let mut response = CURRENT_REQUEST_CONTEXT
        .scope(request_context.clone(), next.run(request))
        .await;
    let current_context = Span::current().context();
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(
            &current_context,
            &mut HeaderInjector(response.headers_mut()),
        );
    });
    insert_request_context_headers(response.headers_mut(), &request_context);
    response
}

#[cfg(feature = "http")]
pub fn annotate_graphql_response(
    mut response: async_graphql::Response,
    request_context: &RequestContext,
) -> async_graphql::Response {
    // GraphQL clients often inspect extensions rather than HTTP headers, so the
    // same request context is attached to both successful and error responses.
    for error in &mut response.errors {
        annotate_graphql_error(error, request_context);
    }
    response.extensions.insert(
        "request_id".to_string(),
        GraphqlValue::from(request_context.request_id.clone()),
    );
    response.extensions.insert(
        "correlation_id".to_string(),
        GraphqlValue::from(request_context.correlation_id.clone()),
    );
    insert_request_context_headers(&mut response.http_headers, request_context);
    response
}

#[cfg(feature = "http")]
pub fn graphql_error_with_context(
    message: impl Into<String>,
    request_context: &RequestContext,
) -> ServerError {
    let mut error = ServerError::new(redact_diagnostic_text(message.into()), None);
    annotate_graphql_error(&mut error, request_context);
    error
}

pub fn redact_diagnostic_text(message: impl AsRef<str>) -> String {
    // Redaction runs before diagnostic text leaves the backend. Keep this list
    // conservative: false positives are less risky than leaking credentials.
    let mut redacted = message.as_ref().to_string();
    for redactor in [
        authorization_redactor(),
        bearer_redactor(),
        url_credentials_redactor(),
        quoted_key_value_redactor(),
        key_value_redactor(),
    ] {
        redacted = redactor
            .replace_all(&redacted, |captures: &regex::Captures<'_>| {
                let prefix = captures.get(1).map(|value| value.as_str()).unwrap_or("");
                format!("{prefix}[REDACTED]")
            })
            .into_owned();
    }
    redacted
}

pub fn redact_diagnostic_value(value: JsonValue) -> JsonValue {
    match value {
        JsonValue::String(message) => JsonValue::String(redact_diagnostic_text(message)),
        JsonValue::Array(items) => {
            JsonValue::Array(items.into_iter().map(redact_diagnostic_value).collect())
        }
        JsonValue::Object(entries) => JsonValue::Object(redact_diagnostic_object(entries)),
        other => other,
    }
}

fn redact_diagnostic_object(entries: JsonMap<String, JsonValue>) -> JsonMap<String, JsonValue> {
    entries
        .into_iter()
        .map(|(key, value)| {
            let value = if diagnostic_key_looks_sensitive(&key) {
                JsonValue::String("[REDACTED]".to_string())
            } else {
                redact_diagnostic_value(value)
            };
            (key, value)
        })
        .collect()
}

fn diagnostic_key_looks_sensitive(key: &str) -> bool {
    let normalized: String = key
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();

    normalized.contains("password")
        || normalized == "pwd"
        || normalized.contains("passwd")
        || normalized.contains("token")
        || normalized.contains("secret")
        || normalized.contains("credential")
        || normalized.contains("authorization")
        || normalized.contains("privatekey")
        || normalized.contains("accesskey")
        || normalized.contains("accountkey")
        || normalized.contains("apikey")
}

#[cfg(feature = "http")]
fn annotate_graphql_error(error: &mut ServerError, request_context: &RequestContext) {
    let redacted_message = redact_diagnostic_text(&error.message);
    let public_error = public_graphql_error(&redacted_message);
    if public_error.message.as_ref() != redacted_message {
        tracing::warn!(
            request_id = %request_context.request_id,
            correlation_id = %request_context.correlation_id,
            error = %redacted_message,
            "suppressed GraphQL diagnostic in client response"
        );
    }
    let error_category = public_error.category;
    error.message = public_error.message.into_owned();
    let extensions = error
        .extensions
        .get_or_insert_with(ErrorExtensionValues::default);
    extensions.set("error_category", GraphqlValue::from(error_category));
    extensions.set(
        "request_id",
        GraphqlValue::from(request_context.request_id.clone()),
    );
    extensions.set(
        "correlation_id",
        GraphqlValue::from(request_context.correlation_id.clone()),
    );
}

#[cfg(feature = "http")]
struct PublicGraphqlError<'a> {
    message: Cow<'a, str>,
    category: &'static str,
}

#[cfg(feature = "http")]
fn public_graphql_error(message: &str) -> PublicGraphqlError<'_> {
    let normalized = message.trim().to_ascii_lowercase();
    if normalized.starts_with("data access error:") {
        return PublicGraphqlError {
            message: Cow::Borrowed("data access error"),
            category: "data_access",
        };
    }
    if normalized.starts_with("not authorized") {
        return PublicGraphqlError {
            message: Cow::Borrowed("not authorized"),
            category: "not_authorized",
        };
    }
    if normalized.starts_with("access denied") {
        return PublicGraphqlError {
            message: Cow::Borrowed("access denied"),
            category: "access_denied",
        };
    }
    if normalized.starts_with("internal server error") {
        return PublicGraphqlError {
            message: Cow::Borrowed("internal server error"),
            category: "internal",
        };
    }
    PublicGraphqlError {
        message: Cow::Borrowed(message),
        category: "graphql",
    }
}

#[cfg(feature = "http")]
fn insert_request_context_headers(headers: &mut HeaderMap, request_context: &RequestContext) {
    insert_header(headers, REQUEST_ID_HEADER_NAME, &request_context.request_id);
    insert_header(
        headers,
        CORRELATION_ID_HEADER_NAME,
        &request_context.correlation_id,
    );
}

#[cfg(feature = "http")]
fn insert_header(headers: &mut HeaderMap, name: &str, value: &str) {
    let Ok(name) = HeaderName::from_bytes(name.as_bytes()) else {
        return;
    };
    let Ok(value) = HeaderValue::from_str(value) else {
        return;
    };
    headers.insert(name, value);
}

impl ObservabilityConfig {
    fn from_env() -> Self {
        let otel = OtelExportConfig::from_env();
        Self {
            service_name: otel.service_name,
            environment: otel.environment,
            json_logs: env_bool("APP_LOG_JSON", false),
            otel_enabled: otel.enabled,
        }
    }

    fn env_filter(&self) -> EnvFilter {
        EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new(default_filter_from_log_level()))
            .unwrap_or_else(|_| EnvFilter::new(DEFAULT_TRACING_FILTER))
    }
}

fn init_tracing_with_otel(
    config: &ObservabilityConfig,
) -> Result<ObservabilityGuard, Box<dyn Error + Send + Sync>> {
    global::set_text_map_propagator(TraceContextPropagator::new());

    // Resource attributes identify the service consistently across spans and
    // metrics, regardless of whether logs are rendered as JSON or compact text.
    let resource = Resource::builder()
        .with_service_name(config.service_name.clone())
        .with_attribute(KeyValue::new(
            "deployment.environment.name",
            config.environment.clone(),
        ))
        .build();
    let exporter = SpanExporter::builder().build()?;
    let tracer_provider = SdkTracerProvider::builder()
        .with_resource(resource.clone())
        .with_batch_exporter(exporter)
        .build();
    global::set_tracer_provider(tracer_provider.clone());

    let metric_exporter = MetricExporter::builder().build()?;
    let meter_provider = SdkMeterProvider::builder()
        .with_resource(resource)
        .with_periodic_exporter(metric_exporter)
        .build();
    global::set_meter_provider(meter_provider.clone());

    let tracer = tracer_provider.tracer("backend");
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let env_filter = config.env_filter();

    if config.json_logs {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(telemetry_layer)
            .with(fmt::layer().json().with_target(true))
            .try_init()?;
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(telemetry_layer)
            .with(fmt::layer().compact().with_target(true))
            .try_init()?;
    }

    Ok(ObservabilityGuard {
        tracer_provider: Some(tracer_provider),
        meter_provider: Some(meter_provider),
    })
}

fn init_local_tracing(config: &ObservabilityConfig) -> Result<(), Box<dyn Error + Send + Sync>> {
    let env_filter = config.env_filter();

    if config.json_logs {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().json().with_target(true))
            .try_init()?;
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().compact().with_target(true))
            .try_init()?;
    }

    Ok(())
}

fn default_filter_from_log_level() -> String {
    let level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    [
        "backend",
        "appfw_runtime",
        "appfw_provider_postgres",
        "appfw_provider_mongo",
        "appfw_provider_mssql",
        "appfw_provider_snowflake",
        "tower_http",
    ]
    .into_iter()
    .map(|target| format!("{target}={level}"))
    .collect::<Vec<_>>()
    .join(",")
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

fn env_bool_from_lookup<F>(lookup: &mut F, name: &str, default: bool) -> bool
where
    F: FnMut(&str) -> Option<String>,
{
    lookup(name)
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}

fn env_value_from_lookup<F>(lookup: &mut F, name: &str) -> Option<String>
where
    F: FnMut(&str) -> Option<String>,
{
    lookup(name)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
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

fn authorization_redactor() -> &'static Regex {
    // Regex compilation is cached because redaction is on the error hot path.
    static REDACTOR: OnceLock<Regex> = OnceLock::new();
    REDACTOR.get_or_init(|| {
        Regex::new(r#"(?i)\b(authorization\s*[:=]\s*)(bearer\s+)?[^,\s;}]+"#)
            .expect("authorization redactor regex")
    })
}

fn bearer_redactor() -> &'static Regex {
    static REDACTOR: OnceLock<Regex> = OnceLock::new();
    REDACTOR.get_or_init(|| {
        Regex::new(r#"(?i)\b(bearer\s+)[A-Za-z0-9._~+/=-]+"#).expect("bearer redactor regex")
    })
}

fn url_credentials_redactor() -> &'static Regex {
    static REDACTOR: OnceLock<Regex> = OnceLock::new();
    REDACTOR
        .get_or_init(|| Regex::new(r#"(://)[^:/?#\s]+:[^@/?#\s]+@"#).expect("url redactor regex"))
}

fn quoted_key_value_redactor() -> &'static Regex {
    static REDACTOR: OnceLock<Regex> = OnceLock::new();
    REDACTOR.get_or_init(|| {
        Regex::new(
            r#"(?i)(["']?(?:api[_-]?key|access[_-]?token|refresh[_-]?token|auth[_-]?token|client[_-]?secret|credential|passwd|password|pwd|private[_-]?key|shared[_-]?access[_-]?key|account[_-]?key|secret|token)["']?\s*:\s*)("[^"]*"|'[^']*'|[^,\s}]+)"#,
        )
        .expect("quoted key/value redactor regex")
    })
}

fn key_value_redactor() -> &'static Regex {
    static REDACTOR: OnceLock<Regex> = OnceLock::new();
    REDACTOR.get_or_init(|| {
        Regex::new(
            r#"(?i)\b((?:api[_-]?key|access[_-]?token|refresh[_-]?token|auth[_-]?token|client[_-]?secret|credential|passwd|password|pwd|private[_-]?key|shared[_-]?access[_-]?key|account[_-]?key|secret|token)\s*[:=]\s*)[^,\s;}]+"#,
        )
        .expect("key/value redactor regex")
    })
}

#[cfg(feature = "http")]
struct HeaderExtractor<'a>(&'a HeaderMap);

#[cfg(feature = "http")]
impl Extractor for HeaderExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|value| value.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(HeaderName::as_str).collect()
    }
}

#[cfg(feature = "http")]
struct HeaderInjector<'a>(&'a mut HeaderMap);

#[cfg(feature = "http")]
impl Injector for HeaderInjector<'_> {
    fn set(&mut self, key: &str, value: String) {
        let Ok(name) = HeaderName::from_bytes(key.as_bytes()) else {
            return;
        };
        let Ok(value) = HeaderValue::from_str(&value) else {
            return;
        };
        self.0.insert(name, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[test]
    fn observability_config_uses_safe_local_defaults() {
        let _lock = env_lock();
        env::remove_var("APP_OTEL_ENABLED");
        env::remove_var("APP_LOG_JSON");
        env::remove_var("APP_SERVICE_NAME");
        env::remove_var("OTEL_SERVICE_NAME");
        env::remove_var("ENV_NAME");

        let config = ObservabilityConfig::from_env();

        assert_eq!(config.service_name, "backend");
        assert_eq!(config.environment, "local");
        assert!(!config.json_logs);
        assert!(!config.otel_enabled);
    }

    #[test]
    fn log_level_fallback_includes_runtime_and_provider_targets() {
        let _lock = env_lock();
        let prior = env::var("LOG_LEVEL").ok();
        env::set_var("LOG_LEVEL", "warn");

        let filter = default_filter_from_log_level();

        assert!(filter.contains("backend=warn"));
        assert!(filter.contains("appfw_runtime=warn"));
        assert!(filter.contains("appfw_provider_postgres=warn"));
        assert!(filter.contains("appfw_provider_mssql=warn"));
        assert!(filter.contains("appfw_provider_snowflake=warn"));
        assert!(filter.contains("tower_http=warn"));

        if let Some(value) = prior {
            env::set_var("LOG_LEVEL", value);
        } else {
            env::remove_var("LOG_LEVEL");
        }
    }

    #[test]
    fn otel_export_config_parses_standard_env_without_network() {
        let values = std::collections::BTreeMap::from([
            ("APP_OTEL_ENABLED", "true"),
            ("APP_SERVICE_NAME", "fallback-service"),
            ("OTEL_SERVICE_NAME", "crm-api"),
            ("ENV_NAME", "staging"),
            ("OTEL_EXPORTER_OTLP_ENDPOINT", "https://otel.example.test"),
            (
                "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT",
                "https://otel.example.test/v1/traces",
            ),
            (
                "OTEL_EXPORTER_OTLP_METRICS_ENDPOINT",
                "https://otel.example.test/v1/metrics",
            ),
            ("OTEL_EXPORTER_OTLP_PROTOCOL", "http/protobuf"),
        ]);

        let config =
            OtelExportConfig::from_lookup(|name| values.get(name).map(|value| value.to_string()));

        assert!(config.enabled);
        assert_eq!(config.service_name, "crm-api");
        assert_eq!(config.environment, "staging");
        assert_eq!(
            config.endpoint.as_deref(),
            Some("https://otel.example.test")
        );
        assert_eq!(
            config.traces_endpoint.as_deref(),
            Some("https://otel.example.test/v1/traces")
        );
        assert_eq!(
            config.metrics_endpoint.as_deref(),
            Some("https://otel.example.test/v1/metrics")
        );
        assert_eq!(config.protocol, "http/protobuf");
    }

    #[test]
    fn otel_export_contract_is_reflected_in_alloy_and_runbook() {
        let contract = otel_export_contract();
        let contract_json = serde_json::to_string(&contract).expect("OTEL contract serializes");
        let alloy = include_str!("../../../observability/alloy/config.alloy");
        let runbook = include_str!("../../../observability/README.md");

        for env_var in &contract.env_vars {
            assert!(
                runbook.contains(env_var),
                "observability runbook should document {env_var}"
            );
        }
        for attribute in &contract.correlation_attributes {
            assert!(
                contract_json.contains(attribute),
                "contract should expose {attribute}"
            );
            assert!(
                alloy.contains(attribute),
                "Alloy config should extract {attribute} from structured logs"
            );
            assert!(
                runbook.contains(attribute),
                "observability runbook should describe {attribute}"
            );
        }
        for header in &contract.propagation_headers {
            assert!(
                runbook.contains(header),
                "observability runbook should document propagation header {header}"
            );
        }
    }

    #[cfg(feature = "http")]
    #[test]
    fn http_span_prefers_request_id_then_correlation_id() {
        let request = Request::builder()
            .uri("/info")
            .header(REQUEST_ID_HEADER_NAME, "req-1")
            .header(CORRELATION_ID_HEADER_NAME, "corr-1")
            .body(())
            .expect("request");
        let _span = http_make_span(&request);

        let request = Request::builder()
            .uri("/info")
            .header(CORRELATION_ID_HEADER_NAME, "corr-1")
            .body(())
            .expect("request");
        let _span = http_make_span(&request);
    }

    #[cfg(feature = "http")]
    #[test]
    fn request_context_defaults_correlation_to_request_id() {
        let mut headers = HeaderMap::new();
        headers.insert(
            REQUEST_ID_HEADER_NAME,
            HeaderValue::from_static("request-123"),
        );

        let request_context = RequestContext::from_headers(&headers);

        assert_eq!(request_context.request_id, "request-123");
        assert_eq!(request_context.correlation_id, "request-123");
    }

    #[cfg(feature = "http")]
    #[test]
    fn graphql_errors_get_request_context_extensions() {
        let request_context = RequestContext {
            request_id: "request-123".to_string(),
            correlation_id: "correlation-456".to_string(),
        };
        let response = async_graphql::Response::from_errors(vec![ServerError::new("boom", None)]);

        let response = annotate_graphql_response(response, &request_context);
        let extensions = response.errors[0]
            .extensions
            .as_ref()
            .expect("error extensions");

        assert_eq!(
            extensions.get("request_id"),
            Some(&GraphqlValue::from("request-123"))
        );
        assert_eq!(
            extensions.get("correlation_id"),
            Some(&GraphqlValue::from("correlation-456"))
        );
    }

    #[cfg(feature = "http")]
    #[test]
    fn graphql_error_messages_are_redacted_before_response() {
        let request_context = RequestContext {
            request_id: "request-123".to_string(),
            correlation_id: "correlation-456".to_string(),
        };
        let response = async_graphql::Response::from_errors(vec![ServerError::new(
            "database failed with password=hunter2 Authorization: Bearer abc.def",
            None,
        )]);

        let response = annotate_graphql_response(response, &request_context);
        let message = &response.errors[0].message;

        assert!(message.contains("[REDACTED]"));
        assert!(!message.contains("hunter2"));
        assert!(!message.contains("abc.def"));
    }

    #[cfg(feature = "http")]
    #[test]
    fn graphql_data_access_errors_hide_provider_diagnostics() {
        let request_context = RequestContext {
            request_id: "request-123".to_string(),
            correlation_id: "correlation-456".to_string(),
        };
        let response = async_graphql::Response::from_errors(vec![ServerError::new(
            "data access error: Fabric ODBC failed in SQLDriverConnect: State: 28000, Native error: 18456",
            None,
        )]);

        let response = annotate_graphql_response(response, &request_context);
        let error = &response.errors[0];
        let extensions = error.extensions.as_ref().expect("error extensions");

        assert_eq!(error.message, "data access error");
        assert!(!error.message.contains("SQLDriverConnect"));
        assert!(!error.message.contains("18456"));
        assert_eq!(
            extensions.get("error_category"),
            Some(&GraphqlValue::from("data_access"))
        );
        assert_eq!(
            extensions.get("request_id"),
            Some(&GraphqlValue::from("request-123"))
        );
    }

    #[test]
    fn diagnostic_redaction_covers_common_secret_shapes() {
        let message = redact_diagnostic_text(
            r#"postgres://svc:secret@db password=hunter2 token=abc Authorization: Bearer jwt.value {"client_secret":"top"}"#,
        );

        assert!(message.contains("[REDACTED]"));
        assert!(!message.contains("svc:secret"));
        assert!(!message.contains("hunter2"));
        assert!(!message.contains("abc"));
        assert!(!message.contains("jwt.value"));
        assert!(!message.contains("top"));
    }

    #[test]
    fn diagnostic_redaction_covers_provider_secret_aliases() {
        let message = redact_diagnostic_text(
            r#"Pwd=short; PrivateKey=snowflake-private; SharedAccessKey=azure-key {"account_key":"storage-key","private_key":"pem"}"#,
        );

        assert!(message.contains("[REDACTED]"));
        assert!(!message.contains("short"));
        assert!(!message.contains("snowflake-private"));
        assert!(!message.contains("azure-key"));
        assert!(!message.contains("storage-key"));
        assert!(!message.contains("pem"));
    }

    #[test]
    fn diagnostic_value_redaction_recurses_through_structured_payloads() {
        let diagnostic = serde_json::json!({
            "connection": "postgres://svc:secret@db",
            "nested": {
                "private_key": "pem",
                "safe": "query plan available"
            },
            "warnings": [
                "Authorization: Bearer jwt.value"
            ]
        });

        let redacted = redact_diagnostic_value(diagnostic);

        assert_eq!(
            redacted["nested"]["safe"],
            serde_json::json!("query plan available")
        );
        assert_eq!(
            redacted["nested"]["private_key"],
            serde_json::json!("[REDACTED]")
        );

        let serialized = redacted.to_string();
        assert!(serialized.contains("[REDACTED]"));
        assert!(!serialized.contains("svc:secret"));
        assert!(!serialized.contains("pem"));
        assert!(!serialized.contains("jwt.value"));
    }

    #[cfg(feature = "http")]
    #[test]
    fn header_injector_ignores_invalid_values() {
        let mut headers = HeaderMap::new();
        let mut injector = HeaderInjector(&mut headers);

        injector.set("traceparent", "valid".to_string());
        injector.set("bad header", "ignored".to_string());
        injector.set("tracestate", "bad\nvalue".to_string());

        assert_eq!(headers.get("traceparent").unwrap(), "valid");
        assert!(!headers.contains_key("bad header"));
        assert!(!headers.contains_key("tracestate"));
    }
}
