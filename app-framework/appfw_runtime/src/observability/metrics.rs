//! In-process metrics registry and Prometheus/OpenTelemetry export helpers.
//!
//! The registry keeps a lightweight local snapshot for `/metrics` and
//! `/metrics.json` while also emitting OpenTelemetry instruments when an OTLP
//! provider is configured.

use std::{
    collections::BTreeMap,
    fmt::Write,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[cfg(feature = "http")]
use axum::{
    extract::{MatchedPath, State},
    http::Request,
    middleware::Next,
    response::Response,
};
use opentelemetry::{
    global,
    metrics::{Counter, Histogram},
    KeyValue,
};
use serde::Serialize;

use crate::ProviderPoolStats;

const LATENCY_BUCKETS_SECONDS: [f64; 10] = [
    0.005, 0.010, 0.025, 0.050, 0.100, 0.250, 0.500, 1.000, 2.500, 5.000,
];

// MetricsRegistry is cheap to clone and safe to put in Axum state. The mutex is
// intentionally narrow: counters are updated quickly, then OpenTelemetry export
// happens after releasing the lock.
#[derive(Clone)]
pub struct MetricsRegistry {
    service_name: String,
    environment: String,
    started_at: Instant,
    started_unix_seconds: u64,
    inner: Arc<Mutex<MetricsInner>>,
    otel: Arc<OtelMetrics>,
}

#[derive(Default)]
struct MetricsInner {
    // BTreeMap keeps JSON and Prometheus output deterministic, which makes
    // tests and operator diffs much easier to read.
    requests: BTreeMap<RequestKey, u64>,
    latency: BTreeMap<LatencyKey, LatencyCounters>,
    provider_operations: BTreeMap<ProviderOperationKey, ProviderOperationCounters>,
    provider_pools: BTreeMap<ProviderPoolKey, ProviderPoolStats>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct RequestKey {
    method: String,
    route: String,
    status: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct LatencyKey {
    method: String,
    route: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct ProviderPoolKey {
    provider: String,
    data_source: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct ProviderOperationKey {
    provider: String,
    data_source: String,
    entity: String,
    operation: String,
}

#[derive(Clone, Debug)]
struct LatencyCounters {
    buckets: [u64; LATENCY_BUCKETS_SECONDS.len()],
    count: u64,
    sum_seconds: f64,
    max_seconds: f64,
}

#[derive(Clone, Debug, Default)]
struct ProviderOperationCounters {
    duration: LatencyCounters,
    query_count: u64,
    result_count: u64,
}

#[derive(Clone)]
struct OtelMetrics {
    http_request_counter: Counter<u64>,
    http_request_duration: Histogram<f64>,
    provider_operation_counter: Counter<u64>,
    provider_operation_duration: Histogram<f64>,
    provider_query_count: Counter<u64>,
    provider_result_count: Counter<u64>,
}

impl Default for LatencyCounters {
    fn default() -> Self {
        Self {
            buckets: [0; LATENCY_BUCKETS_SECONDS.len()],
            count: 0,
            sum_seconds: 0.0,
            max_seconds: 0.0,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MetricsSnapshot {
    pub service_name: String,
    pub environment: String,
    pub uptime_seconds: u64,
    pub started_unix_seconds: u64,
    pub requests: Vec<RequestMetricSnapshot>,
    pub latency: Vec<LatencyMetricSnapshot>,
    pub provider_operations: Vec<ProviderOperationMetricSnapshot>,
    pub provider_pools: Vec<ProviderPoolStats>,
}

#[derive(Debug, Serialize)]
pub struct RequestMetricSnapshot {
    pub method: String,
    pub route: String,
    pub status: u16,
    pub count: u64,
}

#[derive(Debug, Serialize)]
pub struct LatencyMetricSnapshot {
    pub method: String,
    pub route: String,
    pub buckets: Vec<LatencyBucketSnapshot>,
    pub count: u64,
    pub sum_seconds: f64,
}

#[derive(Debug, Serialize)]
pub struct LatencyBucketSnapshot {
    pub le_seconds: f64,
    pub count: u64,
}

#[derive(Debug, Serialize)]
pub struct ProviderOperationMetricSnapshot {
    pub provider: String,
    pub data_source: String,
    pub entity: String,
    pub operation: String,
    pub operation_count: u64,
    pub query_count: u64,
    pub result_count: u64,
    pub duration_buckets: Vec<LatencyBucketSnapshot>,
    pub duration_count: u64,
    pub duration_sum_seconds: f64,
    pub duration_avg_seconds: f64,
    pub duration_max_seconds: f64,
}

impl MetricsRegistry {
    pub fn from_env() -> Self {
        let service_name = std::env::var("OTEL_SERVICE_NAME")
            .or_else(|_| std::env::var("APP_SERVICE_NAME"))
            .unwrap_or_else(|_| "backend".to_string());
        let environment = std::env::var("ENV_NAME").unwrap_or_else(|_| "local".to_string());
        Self::new(service_name, environment)
    }

    pub fn new(service_name: impl Into<String>, environment: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            environment: environment.into(),
            started_at: Instant::now(),
            started_unix_seconds: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .unwrap_or_default(),
            inner: Arc::new(Mutex::new(MetricsInner::default())),
            otel: Arc::new(OtelMetrics::new()),
        }
    }

    pub fn record_http_request(&self, method: &str, route: &str, status: u16, duration: Duration) {
        let mut inner = self.inner.lock().expect("metrics mutex poisoned");
        *inner
            .requests
            .entry(RequestKey {
                method: method.to_string(),
                route: route.to_string(),
                status,
            })
            .or_default() += 1;

        let counters = inner
            .latency
            .entry(LatencyKey {
                method: method.to_string(),
                route: route.to_string(),
            })
            .or_default();
        record_latency(counters, duration);

        self.otel
            .record_http_request(method, route, status, duration.as_secs_f64());
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        // Snapshot converts internal counter keys into serializable structs so
        // the public JSON shape is stable even if the storage layout changes.
        let inner = self.inner.lock().expect("metrics mutex poisoned");
        MetricsSnapshot {
            service_name: self.service_name.clone(),
            environment: self.environment.clone(),
            uptime_seconds: self.started_at.elapsed().as_secs(),
            started_unix_seconds: self.started_unix_seconds,
            requests: inner
                .requests
                .iter()
                .map(|(key, count)| RequestMetricSnapshot {
                    method: key.method.clone(),
                    route: key.route.clone(),
                    status: key.status,
                    count: *count,
                })
                .collect(),
            latency: inner
                .latency
                .iter()
                .map(|(key, counters)| LatencyMetricSnapshot {
                    method: key.method.clone(),
                    route: key.route.clone(),
                    buckets: LATENCY_BUCKETS_SECONDS
                        .iter()
                        .enumerate()
                        .map(|(index, bucket)| LatencyBucketSnapshot {
                            le_seconds: *bucket,
                            count: counters.buckets[index],
                        })
                        .collect(),
                    count: counters.count,
                    sum_seconds: counters.sum_seconds,
                })
                .collect(),
            provider_operations: inner
                .provider_operations
                .iter()
                .map(|(key, counters)| ProviderOperationMetricSnapshot {
                    provider: key.provider.clone(),
                    data_source: key.data_source.clone(),
                    entity: key.entity.clone(),
                    operation: key.operation.clone(),
                    operation_count: counters.duration.count,
                    query_count: counters.query_count,
                    result_count: counters.result_count,
                    duration_buckets: latency_bucket_snapshots(&counters.duration),
                    duration_count: counters.duration.count,
                    duration_sum_seconds: counters.duration.sum_seconds,
                    duration_avg_seconds: latency_average_seconds(&counters.duration),
                    duration_max_seconds: counters.duration.max_seconds,
                })
                .collect(),
            provider_pools: inner.provider_pools.values().cloned().collect(),
        }
    }

    pub fn record_provider_operation(
        &self,
        provider: &str,
        data_source: &str,
        entity: &str,
        operation: &str,
        query_count: u64,
        result_count: u64,
        duration: Duration,
    ) {
        let mut inner = self.inner.lock().expect("metrics mutex poisoned");
        let counters = inner
            .provider_operations
            .entry(ProviderOperationKey {
                provider: provider.to_string(),
                data_source: data_source.to_string(),
                entity: entity.to_string(),
                operation: operation.to_string(),
            })
            .or_default();
        record_latency(&mut counters.duration, duration);
        counters.query_count += query_count;
        counters.result_count += result_count;
        drop(inner);

        self.otel.record_provider_operation(
            provider,
            data_source,
            entity,
            operation,
            query_count,
            result_count,
            duration.as_secs_f64(),
        );
    }

    pub fn record_provider_pool_stats(&self, stats: ProviderPoolStats) {
        // Pool pressure is refreshed from readiness checks; replace the prior
        // value for each provider/data-source pair instead of accumulating it.
        let mut inner = self.inner.lock().expect("metrics mutex poisoned");
        inner.provider_pools.insert(
            ProviderPoolKey {
                provider: stats.provider.clone(),
                data_source: stats.data_source.clone(),
            },
            stats,
        );
    }

    pub fn to_prometheus(&self) -> String {
        // Prometheus exposition is rendered from a snapshot, not while holding
        // the live registry lock.
        let snapshot = self.snapshot();
        let service = prometheus_label_value(&snapshot.service_name);
        let environment = prometheus_label_value(&snapshot.environment);
        let mut output = String::new();

        let _ = writeln!(
            output,
            "# HELP app_info Static application identity labels."
        );
        let _ = writeln!(output, "# TYPE app_info gauge");
        let _ = writeln!(
            output,
            "app_info{{service=\"{service}\",environment=\"{environment}\"}} 1"
        );

        let _ = writeln!(
            output,
            "# HELP app_uptime_seconds Seconds since this process started."
        );
        let _ = writeln!(output, "# TYPE app_uptime_seconds gauge");
        let _ = writeln!(
            output,
            "app_uptime_seconds{{service=\"{service}\",environment=\"{environment}\"}} {}",
            snapshot.uptime_seconds
        );

        let _ = writeln!(
            output,
            "# HELP app_http_requests_total Total HTTP responses by method, route, and status."
        );
        let _ = writeln!(output, "# TYPE app_http_requests_total counter");
        for request in &snapshot.requests {
            let method = prometheus_label_value(&request.method);
            let route = prometheus_label_value(&request.route);
            let _ = writeln!(
                output,
                "app_http_requests_total{{service=\"{service}\",environment=\"{environment}\",method=\"{method}\",route=\"{route}\",status=\"{}\"}} {}",
                request.status, request.count
            );
        }

        let _ = writeln!(
            output,
            "# HELP app_http_request_duration_seconds HTTP request latency histogram."
        );
        let _ = writeln!(output, "# TYPE app_http_request_duration_seconds histogram");
        for latency in &snapshot.latency {
            let method = prometheus_label_value(&latency.method);
            let route = prometheus_label_value(&latency.route);
            for bucket in &latency.buckets {
                let _ = writeln!(
                    output,
                    "app_http_request_duration_seconds_bucket{{service=\"{service}\",environment=\"{environment}\",method=\"{method}\",route=\"{route}\",le=\"{:.3}\"}} {}",
                    bucket.le_seconds, bucket.count
                );
            }
            let _ = writeln!(
                output,
                "app_http_request_duration_seconds_bucket{{service=\"{service}\",environment=\"{environment}\",method=\"{method}\",route=\"{route}\",le=\"+Inf\"}} {}",
                latency.count
            );
            let _ = writeln!(
                output,
                "app_http_request_duration_seconds_count{{service=\"{service}\",environment=\"{environment}\",method=\"{method}\",route=\"{route}\"}} {}",
                latency.count
            );
            let _ = writeln!(
                output,
                "app_http_request_duration_seconds_sum{{service=\"{service}\",environment=\"{environment}\",method=\"{method}\",route=\"{route}\"}} {:.6}",
                latency.sum_seconds
            );
        }

        let _ = writeln!(
            output,
            "# HELP app_provider_operations_total Total provider operations by provider, data source, entity, and operation."
        );
        let _ = writeln!(output, "# TYPE app_provider_operations_total counter");
        let _ = writeln!(
            output,
            "# HELP app_provider_query_count_total Total provider query_count values returned by provider operations."
        );
        let _ = writeln!(output, "# TYPE app_provider_query_count_total counter");
        let _ = writeln!(
            output,
            "# HELP app_provider_result_count_total Total result items returned by provider operations."
        );
        let _ = writeln!(output, "# TYPE app_provider_result_count_total counter");
        let _ = writeln!(
            output,
            "# HELP app_provider_operation_duration_seconds Provider operation latency histogram."
        );
        let _ = writeln!(
            output,
            "# TYPE app_provider_operation_duration_seconds histogram"
        );
        let _ = writeln!(
            output,
            "# HELP app_provider_operation_duration_seconds_max Maximum observed provider operation duration in seconds."
        );
        let _ = writeln!(
            output,
            "# TYPE app_provider_operation_duration_seconds_max gauge"
        );
        for operation in &snapshot.provider_operations {
            let provider = prometheus_label_value(&operation.provider);
            let data_source = prometheus_label_value(&operation.data_source);
            let entity = prometheus_label_value(&operation.entity);
            let operation_name = prometheus_label_value(&operation.operation);
            let labels = format!(
                "service=\"{service}\",environment=\"{environment}\",provider=\"{provider}\",data_source=\"{data_source}\",entity=\"{entity}\",operation=\"{operation_name}\""
            );
            let _ = writeln!(
                output,
                "app_provider_operations_total{{{labels}}} {}",
                operation.duration_count
            );
            let _ = writeln!(
                output,
                "app_provider_query_count_total{{{labels}}} {}",
                operation.query_count
            );
            let _ = writeln!(
                output,
                "app_provider_result_count_total{{{labels}}} {}",
                operation.result_count
            );
            for bucket in &operation.duration_buckets {
                let _ = writeln!(
                    output,
                    "app_provider_operation_duration_seconds_bucket{{{labels},le=\"{:.3}\"}} {}",
                    bucket.le_seconds, bucket.count
                );
            }
            let _ = writeln!(
                output,
                "app_provider_operation_duration_seconds_bucket{{{labels},le=\"+Inf\"}} {}",
                operation.duration_count
            );
            let _ = writeln!(
                output,
                "app_provider_operation_duration_seconds_count{{{labels}}} {}",
                operation.duration_count
            );
            let _ = writeln!(
                output,
                "app_provider_operation_duration_seconds_sum{{{labels}}} {:.6}",
                operation.duration_sum_seconds
            );
            let _ = writeln!(
                output,
                "app_provider_operation_duration_seconds_max{{{labels}}} {:.6}",
                operation.duration_max_seconds
            );
        }

        let _ = writeln!(
            output,
            "# HELP app_provider_pool_instrumented Whether provider pool pressure is directly instrumented."
        );
        let _ = writeln!(output, "# TYPE app_provider_pool_instrumented gauge");
        let _ = writeln!(
            output,
            "# HELP app_provider_pool_pressure Provider connection pool pressure ratio."
        );
        let _ = writeln!(output, "# TYPE app_provider_pool_pressure gauge");
        let _ = writeln!(
            output,
            "# HELP app_provider_pool_connections Provider connection pool connection gauges."
        );
        let _ = writeln!(output, "# TYPE app_provider_pool_connections gauge");
        for pool in &snapshot.provider_pools {
            let provider = prometheus_label_value(&pool.provider);
            let data_source = prometheus_label_value(&pool.data_source);
            let instrumented = if pool.instrumented { 1 } else { 0 };
            let _ = writeln!(
                output,
                "app_provider_pool_instrumented{{service=\"{service}\",environment=\"{environment}\",provider=\"{provider}\",data_source=\"{data_source}\"}} {instrumented}"
            );
            if let Some(pressure) = pool.pressure {
                let _ = writeln!(
                    output,
                    "app_provider_pool_pressure{{service=\"{service}\",environment=\"{environment}\",provider=\"{provider}\",data_source=\"{data_source}\"}} {:.6}",
                    pressure
                );
            }
            for (state, value) in [
                ("max", pool.max_size),
                ("size", pool.size),
                ("available", pool.available),
                ("in_use", pool.in_use),
                ("waiting", pool.waiting),
            ] {
                if let Some(value) = value {
                    let _ = writeln!(
                        output,
                        "app_provider_pool_connections{{service=\"{service}\",environment=\"{environment}\",provider=\"{provider}\",data_source=\"{data_source}\",state=\"{state}\"}} {value}"
                    );
                }
            }
        }

        output
    }
}

impl OtelMetrics {
    fn new() -> Self {
        // Instruments are created even when OTLP export is disabled; the global
        // no-op meter keeps call sites simple and avoids feature-gated branches.
        let meter = global::meter("backend");
        Self {
            http_request_counter: meter
                .u64_counter("app.http.requests")
                .with_description("Total HTTP responses by method, route, and status.")
                .build(),
            http_request_duration: meter
                .f64_histogram("app.http.request.duration")
                .with_description("HTTP request latency in seconds.")
                .build(),
            provider_operation_counter: meter
                .u64_counter("app.provider.operations")
                .with_description("Total provider operations.")
                .build(),
            provider_operation_duration: meter
                .f64_histogram("app.provider.operation.duration")
                .with_description("Provider operation latency in seconds.")
                .build(),
            provider_query_count: meter
                .u64_counter("app.provider.query_count")
                .with_description("Total provider query_count values returned by operations.")
                .build(),
            provider_result_count: meter
                .u64_counter("app.provider.result_count")
                .with_description("Total result items returned by provider operations.")
                .build(),
        }
    }

    fn record_http_request(&self, method: &str, route: &str, status: u16, duration_seconds: f64) {
        let attributes = [
            KeyValue::new("http.request.method", method.to_string()),
            KeyValue::new("http.route", route.to_string()),
            KeyValue::new("http.response.status_code", i64::from(status)),
        ];
        self.http_request_counter.add(1, &attributes);
        self.http_request_duration
            .record(duration_seconds, &attributes);
    }

    fn record_provider_operation(
        &self,
        provider: &str,
        data_source: &str,
        entity: &str,
        operation: &str,
        query_count: u64,
        result_count: u64,
        duration_seconds: f64,
    ) {
        let attributes = [
            KeyValue::new("app.provider", provider.to_string()),
            KeyValue::new("app.data_source", data_source.to_string()),
            KeyValue::new("app.entity", entity.to_string()),
            KeyValue::new("app.operation", operation.to_string()),
        ];
        self.provider_operation_counter.add(1, &attributes);
        self.provider_query_count.add(query_count, &attributes);
        self.provider_result_count.add(result_count, &attributes);
        self.provider_operation_duration
            .record(duration_seconds, &attributes);
    }
}

fn record_latency(counters: &mut LatencyCounters, duration: Duration) {
    let seconds = duration.as_secs_f64();
    counters.count += 1;
    counters.sum_seconds += seconds;
    counters.max_seconds = counters.max_seconds.max(seconds);
    for (index, bucket) in LATENCY_BUCKETS_SECONDS.iter().enumerate() {
        if seconds <= *bucket {
            counters.buckets[index] += 1;
        }
    }
}

fn latency_bucket_snapshots(counters: &LatencyCounters) -> Vec<LatencyBucketSnapshot> {
    LATENCY_BUCKETS_SECONDS
        .iter()
        .enumerate()
        .map(|(index, bucket)| LatencyBucketSnapshot {
            le_seconds: *bucket,
            count: counters.buckets[index],
        })
        .collect()
}

fn latency_average_seconds(counters: &LatencyCounters) -> f64 {
    if counters.count == 0 {
        0.0
    } else {
        counters.sum_seconds / counters.count as f64
    }
}

#[cfg(feature = "http")]
pub async fn metrics_hook<B>(
    State(metrics): State<MetricsRegistry>,
    request: Request<B>,
    next: Next<B>,
) -> Response {
    // Capture route and method before the request is moved into the next
    // middleware/service stage.
    let started_at = Instant::now();
    let method = request.method().as_str().to_string();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map(MatchedPath::as_str)
        .unwrap_or_else(|| request.uri().path())
        .to_string();

    let response = next.run(request).await;
    metrics.record_http_request(
        &method,
        &route,
        response.status().as_u16(),
        started_at.elapsed(),
    );
    response
}

fn prometheus_label_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prometheus_output_contains_request_counts_and_latency() {
        let metrics = MetricsRegistry::new("backend-test", "test");
        metrics.record_http_request("GET", "/info", 200, Duration::from_millis(12));
        metrics.record_http_request("POST", "/graphql", 429, Duration::from_millis(250));
        metrics.record_provider_pool_stats(ProviderPoolStats::instrumented(
            "PostgreSQL",
            "pg_primary",
            10,
            8,
            2,
            1,
        ));
        metrics.record_provider_operation(
            "PostgreSQL",
            "pg_primary",
            "Account",
            "query_items",
            42,
            10,
            Duration::from_millis(75),
        );

        let output = metrics.to_prometheus();

        assert!(output.contains("app_info{service=\"backend-test\",environment=\"test\"} 1"));
        assert!(output.contains(
            "app_http_requests_total{service=\"backend-test\",environment=\"test\",method=\"GET\",route=\"/info\",status=\"200\"} 1"
        ));
        assert!(output.contains(
            "app_http_requests_total{service=\"backend-test\",environment=\"test\",method=\"POST\",route=\"/graphql\",status=\"429\"} 1"
        ));
        assert!(output.contains(
            "app_http_request_duration_seconds_bucket{service=\"backend-test\",environment=\"test\",method=\"GET\",route=\"/info\",le=\"0.025\"} 1"
        ));
        assert!(output.contains(
            "app_http_request_duration_seconds_count{service=\"backend-test\",environment=\"test\",method=\"POST\",route=\"/graphql\"} 1"
        ));
        assert!(output.contains(
            "app_provider_pool_pressure{service=\"backend-test\",environment=\"test\",provider=\"PostgreSQL\",data_source=\"pg_primary\"} 0.700000"
        ));
        assert!(output.contains(
            "app_provider_operations_total{service=\"backend-test\",environment=\"test\",provider=\"PostgreSQL\",data_source=\"pg_primary\",entity=\"Account\",operation=\"query_items\"} 1"
        ));
        assert!(output.contains(
            "app_provider_query_count_total{service=\"backend-test\",environment=\"test\",provider=\"PostgreSQL\",data_source=\"pg_primary\",entity=\"Account\",operation=\"query_items\"} 42"
        ));
        assert!(output.contains(
            "app_provider_result_count_total{service=\"backend-test\",environment=\"test\",provider=\"PostgreSQL\",data_source=\"pg_primary\",entity=\"Account\",operation=\"query_items\"} 10"
        ));
        assert!(output.contains(
            "app_provider_operation_duration_seconds_count{service=\"backend-test\",environment=\"test\",provider=\"PostgreSQL\",data_source=\"pg_primary\",entity=\"Account\",operation=\"query_items\"} 1"
        ));
        assert!(output.contains(
            "app_provider_operation_duration_seconds_max{service=\"backend-test\",environment=\"test\",provider=\"PostgreSQL\",data_source=\"pg_primary\",entity=\"Account\",operation=\"query_items\"} 0.075000"
        ));
    }

    #[test]
    fn snapshot_and_prometheus_output_contain_provider_counts_and_max_duration() {
        let metrics = MetricsRegistry::new("backend-test", "test");
        metrics.record_provider_operation(
            "MongoDB",
            "mongo_primary",
            "Lead",
            "aggregate_items",
            12,
            3,
            Duration::from_millis(25),
        );
        metrics.record_provider_operation(
            "MongoDB",
            "mongo_primary",
            "Lead",
            "aggregate_items",
            8,
            4,
            Duration::from_millis(75),
        );

        let snapshot = metrics.snapshot();
        let operation = &snapshot.provider_operations[0];

        assert_eq!(snapshot.provider_operations.len(), 1);
        assert_eq!(operation.provider, "MongoDB");
        assert_eq!(operation.data_source, "mongo_primary");
        assert_eq!(operation.query_count, 20);
        assert_eq!(operation.result_count, 7);
        assert_eq!(operation.operation_count, 2);
        assert_eq!(operation.duration_count, 2);
        assert!((operation.duration_sum_seconds - 0.100).abs() < f64::EPSILON);
        assert!((operation.duration_avg_seconds - 0.050).abs() < f64::EPSILON);
        assert!((operation.duration_max_seconds - 0.075).abs() < f64::EPSILON);

        let output = metrics.to_prometheus();
        assert!(output.contains(
            "app_provider_query_count_total{service=\"backend-test\",environment=\"test\",provider=\"MongoDB\",data_source=\"mongo_primary\",entity=\"Lead\",operation=\"aggregate_items\"} 20"
        ));
        assert!(output.contains(
            "app_provider_result_count_total{service=\"backend-test\",environment=\"test\",provider=\"MongoDB\",data_source=\"mongo_primary\",entity=\"Lead\",operation=\"aggregate_items\"} 7"
        ));
        assert!(output.contains(
            "app_provider_operation_duration_seconds_count{service=\"backend-test\",environment=\"test\",provider=\"MongoDB\",data_source=\"mongo_primary\",entity=\"Lead\",operation=\"aggregate_items\"} 2"
        ));
        assert!(output.contains(
            "app_provider_operation_duration_seconds_sum{service=\"backend-test\",environment=\"test\",provider=\"MongoDB\",data_source=\"mongo_primary\",entity=\"Lead\",operation=\"aggregate_items\"} 0.100000"
        ));
        assert!(output.contains(
            "app_provider_operation_duration_seconds_max{service=\"backend-test\",environment=\"test\",provider=\"MongoDB\",data_source=\"mongo_primary\",entity=\"Lead\",operation=\"aggregate_items\"} 0.075000"
        ));
    }

    #[test]
    fn observability_bundle_links_provider_latency_alert_dashboard_and_runbook() {
        let dashboard: serde_json::Value = serde_json::from_str(include_str!(
            "../../../observability/grafana/dashboards/app-framework-backend.json"
        ))
        .expect("Grafana dashboard JSON parses");
        let dashboard_text = serde_json::to_string(&dashboard).expect("dashboard serializes");

        let alerts: serde_yaml::Value =
            serde_yaml::from_str(include_str!("../../../observability/prometheus/alerts.yml"))
                .expect("Prometheus alerts YAML parses");
        let alerts_text = serde_yaml::to_string(&alerts).expect("alerts serialize");

        let runbook = include_str!("../../../observability/README.md");

        for metric in [
            "app_provider_operation_duration_seconds_bucket",
            "app_provider_operation_duration_seconds_max",
            "app_provider_query_count_total",
            "app_provider_result_count_total",
            "app_provider_pool_pressure",
        ] {
            assert!(
                dashboard_text.contains(metric),
                "Grafana dashboard should reference {metric}"
            );
            assert!(
                runbook.contains(metric),
                "observability runbook should describe {metric}"
            );
        }

        for expected_panel in [
            "Provider Latency p95",
            "Provider Max Duration",
            "Provider Query and Result Rate",
            "Provider Pool Pressure",
            "Backend Logs",
        ] {
            assert!(
                dashboard_text.contains(expected_panel),
                "Grafana dashboard should keep the {expected_panel} panel"
            );
        }

        assert!(
            alerts_text.contains("ProviderSlowOperationsP95"),
            "Prometheus alerts should keep provider slow-operation coverage"
        );
        assert!(
            alerts_text.contains("histogram_quantile")
                && alerts_text.contains("app_provider_operation_duration_seconds_bucket")
                && alerts_text.contains("provider, data_source, entity, operation"),
            "ProviderSlowOperationsP95 should be grouped by provider operation labels"
        );
        assert!(runbook.contains("/metrics"));
        assert!(runbook.contains("/metrics.json"));
        assert!(runbook.contains("request/correlation IDs"));
    }

    #[test]
    fn prometheus_labels_are_escaped() {
        assert_eq!(
            prometheus_label_value("tenant\\\"one\nline"),
            "tenant\\\\\\\"one\\nline"
        );
    }
}
