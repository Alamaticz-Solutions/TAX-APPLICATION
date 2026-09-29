use std::{
    collections::BTreeMap,
    env,
    error::Error,
    fmt::Write,
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use opentelemetry::{global, trace::TracerProvider as _, KeyValue};
use opentelemetry_otlp::SpanExporter;
use opentelemetry_sdk::{propagation::TraceContextPropagator, trace::SdkTracerProvider, Resource};
use serde::Serialize;
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

const DEFAULT_TRACING_FILTER: &str = "database=info";

static METRICS: OnceLock<DatabaseMetrics> = OnceLock::new();
static SILENT: AtomicBool = AtomicBool::new(false);

#[derive(Clone)]
pub struct DatabaseMetrics {
    service_name: String,
    environment: String,
    started_at: Instant,
    started_unix_seconds: u64,
    inner: Arc<Mutex<MetricsInner>>,
}

#[derive(Default)]
struct MetricsInner {
    migrations: BTreeMap<MigrationKey, MigrationCounters>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct MigrationKey {
    provider: String,
    data_source: String,
    phase: String,
    status: String,
}

#[derive(Clone, Debug, Default)]
struct MigrationCounters {
    count: u64,
    duration_ms: u128,
}

#[derive(Debug, Serialize)]
pub struct MetricsSnapshot {
    pub service_name: String,
    pub environment: String,
    pub uptime_seconds: u64,
    pub started_unix_seconds: u64,
    pub migrations: Vec<MigrationMetricSnapshot>,
}

#[derive(Debug, Serialize)]
pub struct MigrationMetricSnapshot {
    pub provider: String,
    pub data_source: String,
    pub phase: String,
    pub status: String,
    pub count: u64,
    pub duration_ms: u128,
}

#[derive(Default)]
pub struct ObservabilityGuard {
    tracer_provider: Option<SdkTracerProvider>,
}

impl ObservabilityGuard {
    pub fn shutdown(mut self) {
        self.shutdown_provider();
    }

    fn shutdown_provider(&mut self) {
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

#[derive(Clone, Debug)]
struct ObservabilityConfig {
    service_name: String,
    environment: String,
    json_logs: bool,
    otel_enabled: bool,
}

impl ObservabilityConfig {
    fn from_env() -> Self {
        Self {
            service_name: env::var("OTEL_SERVICE_NAME")
                .or_else(|_| env::var("APP_SERVICE_NAME"))
                .unwrap_or_else(|_| "database".to_string()),
            environment: env::var("ENV_NAME").unwrap_or_else(|_| "local".to_string()),
            json_logs: env_bool("DATABASE_LOG_JSON", env_bool("APP_LOG_JSON", false)),
            otel_enabled: env_bool("DATABASE_OTEL_ENABLED", env_bool("APP_OTEL_ENABLED", false)),
        }
    }

    fn env_filter(&self) -> EnvFilter {
        if env::var("RUST_LOG")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .is_some()
        {
            return EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(DEFAULT_TRACING_FILTER));
        }

        EnvFilter::try_new(default_filter_from_log_level())
            .unwrap_or_else(|_| EnvFilter::new(DEFAULT_TRACING_FILTER))
    }
}

impl DatabaseMetrics {
    fn from_config(config: &ObservabilityConfig) -> Self {
        Self {
            service_name: config.service_name.clone(),
            environment: config.environment.clone(),
            started_at: Instant::now(),
            started_unix_seconds: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .unwrap_or_default(),
            inner: Arc::new(Mutex::new(MetricsInner::default())),
        }
    }

    fn record_migration(
        &self,
        provider: &str,
        data_source: &str,
        phase: &str,
        status: &str,
        duration: Duration,
    ) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        let counters = inner
            .migrations
            .entry(MigrationKey {
                provider: provider.to_string(),
                data_source: data_source.to_string(),
                phase: phase.to_string(),
                status: status.to_string(),
            })
            .or_default();
        counters.count += 1;
        counters.duration_ms += duration.as_millis();
    }

    fn snapshot(&self) -> MetricsSnapshot {
        let inner = self.inner.lock().ok();
        let migrations = inner
            .as_ref()
            .map(|inner| {
                inner
                    .migrations
                    .iter()
                    .map(|(key, counters)| MigrationMetricSnapshot {
                        provider: key.provider.clone(),
                        data_source: key.data_source.clone(),
                        phase: key.phase.clone(),
                        status: key.status.clone(),
                        count: counters.count,
                        duration_ms: counters.duration_ms,
                    })
                    .collect()
            })
            .unwrap_or_default();
        MetricsSnapshot {
            service_name: self.service_name.clone(),
            environment: self.environment.clone(),
            uptime_seconds: self.started_at.elapsed().as_secs(),
            started_unix_seconds: self.started_unix_seconds,
            migrations,
        }
    }

    fn to_prometheus(&self) -> String {
        let snapshot = self.snapshot();
        let service = prometheus_label_value(&snapshot.service_name);
        let environment = prometheus_label_value(&snapshot.environment);
        let mut output = String::new();

        let _ = writeln!(
            output,
            "# HELP app_database_info Database utility identity."
        );
        let _ = writeln!(output, "# TYPE app_database_info gauge");
        let _ = writeln!(
            output,
            "app_database_info{{service=\"{service}\",environment=\"{environment}\"}} 1"
        );
        let _ = writeln!(
            output,
            "# HELP app_database_migrations_total Migration attempts by provider, data source, phase, and status."
        );
        let _ = writeln!(output, "# TYPE app_database_migrations_total counter");
        for migration in &snapshot.migrations {
            let provider = prometheus_label_value(&migration.provider);
            let data_source = prometheus_label_value(&migration.data_source);
            let phase = prometheus_label_value(&migration.phase);
            let status = prometheus_label_value(&migration.status);
            let _ = writeln!(
                output,
                "app_database_migrations_total{{service=\"{service}\",environment=\"{environment}\",provider=\"{provider}\",data_source=\"{data_source}\",phase=\"{phase}\",status=\"{status}\"}} {}",
                migration.count
            );
            let _ = writeln!(
                output,
                "app_database_migration_duration_milliseconds_sum{{service=\"{service}\",environment=\"{environment}\",provider=\"{provider}\",data_source=\"{data_source}\",phase=\"{phase}\",status=\"{status}\"}} {}",
                migration.duration_ms
            );
        }

        output
    }
}

pub fn init_tracing() -> ObservabilityGuard {
    let config = ObservabilityConfig::from_env();
    let _ = METRICS.set(DatabaseMetrics::from_config(&config));

    if config.otel_enabled {
        match init_tracing_with_otel(&config) {
            Ok(guard) => return guard,
            Err(error) => {
                eprintln!("failed to initialize database OpenTelemetry tracing; falling back to local logs: {error}");
            }
        }
    }

    if let Err(error) = init_local_tracing(&config) {
        eprintln!("failed to initialize database tracing subscriber: {error}");
    }
    ObservabilityGuard::default()
}

pub fn migration_span(
    provider: &str,
    data_source: &str,
    migration_id: &str,
    migration_name: &str,
    phase: &str,
) -> Span {
    tracing::info_span!(
        target: "database",
        "database.migration.apply",
        otel_kind = "internal",
        db_system = %provider,
        app_data_source = %data_source,
        app_migration_id = %migration_id,
        app_migration_name = %migration_name,
        app_migration_phase = %phase,
    )
}

pub fn record_migration(
    provider: &str,
    data_source: &str,
    migration_id: &str,
    migration_name: &str,
    phase: &str,
    status: &str,
    duration: Duration,
) {
    if let Some(metrics) = METRICS.get() {
        metrics.record_migration(provider, data_source, phase, status, duration);
    }
    if SILENT.load(Ordering::Relaxed) {
        return;
    }
    let elapsed_ms = duration.as_millis() as u64;
    tracing::info!(
        target: "database",
        db_system = provider,
        app_data_source = data_source,
        app_migration_id = migration_id,
        app_migration_name = migration_name,
        app_migration_phase = phase,
        app_migration_status = status,
        app_duration_ms = elapsed_ms,
        "database migration completed"
    );

    Span::current().set_attribute("app.migration.status", status.to_string());
    Span::current().set_attribute("app.duration_ms", elapsed_ms as i64);
}

pub fn write_metrics() {
    let Some(metrics) = METRICS.get() else {
        return;
    };
    let target_dir = target_dir();
    if fs::create_dir_all(&target_dir).is_err() {
        return;
    }

    let json_path = target_dir.join("database_metrics.json");
    let prom_path = target_dir.join("database_metrics.prom");
    if let Ok(json) = serde_json::to_string_pretty(&metrics.snapshot()) {
        let _ = fs::write(&json_path, format!("{json}\n"));
    }
    let _ = fs::write(&prom_path, metrics.to_prometheus());
}

pub fn emit_console_event(label: &str, message: &str, elapsed_ms: u128) {
    if !SILENT.load(Ordering::Relaxed) && structured_console_enabled() {
        tracing::info!(
            target: "database",
            app_event = label,
            app_elapsed_ms = elapsed_ms as u64,
            message = message,
            "database console event"
        );
    }
}

pub fn set_silent(silent: bool) {
    SILENT.store(silent, Ordering::Relaxed);
}

pub fn structured_console_enabled() -> bool {
    env_bool("DATABASE_LOG_JSON", env_bool("APP_LOG_JSON", false))
        || env_bool("DATABASE_OTEL_ENABLED", env_bool("APP_OTEL_ENABLED", false))
}

fn init_tracing_with_otel(
    config: &ObservabilityConfig,
) -> Result<ObservabilityGuard, Box<dyn Error + Send + Sync>> {
    global::set_text_map_propagator(TraceContextPropagator::new());

    let exporter = SpanExporter::builder().build()?;
    let resource = Resource::builder()
        .with_service_name(config.service_name.clone())
        .with_attribute(KeyValue::new(
            "deployment.environment.name",
            config.environment.clone(),
        ))
        .build();
    let tracer_provider = SdkTracerProvider::builder()
        .with_resource(resource)
        .with_batch_exporter(exporter)
        .build();
    global::set_tracer_provider(tracer_provider.clone());

    let tracer = tracer_provider.tracer("database");
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
    format!("database={level}")
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

fn target_dir() -> PathBuf {
    env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("target/appfw")
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
    fn prometheus_labels_are_escaped() {
        assert_eq!(
            prometheus_label_value("tenant\\\"one\nline"),
            "tenant\\\\\\\"one\\nline"
        );
    }
}
