# Logging And Observability

The backend uses `tracing` and `tracing-subscriber`. There is no separate
database logger or multi-backend app logger abstraction.

## Runtime Configuration

The backend initializes tracing in `backend/src/main.rs`.

Useful local variables:

```text
RUST_LOG=backend=info,appfw_runtime=info,appfw_provider_postgres=info,appfw_provider_mongo=info,appfw_provider_mssql=info,appfw_provider_snowflake=info,tower_http=info
LOG_LEVEL=info
APP_LOG_JSON=true
APP_OTEL_ENABLED=false
OTEL_SERVICE_NAME=backend
OTEL_EXPORTER_OTLP_ENDPOINT=http://collector:4318
OTEL_METRIC_EXPORT_INTERVAL=60000
```

`RUST_LOG` takes precedence through `EnvFilter`. `LOG_LEVEL` is used as a simple
fallback for the backend, shared runtime, release-certified provider crates, and
tower HTTP targets so provider error normalization and request traces land in CI
artifacts without a custom filter.

Set `APP_OTEL_ENABLED=true` to export OpenTelemetry traces and metrics through
the OTLP exporter. Standard OTLP environment variables can split trace and
metric destinations when needed, for example `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT`
and `OTEL_EXPORTER_OTLP_METRICS_ENDPOINT`.

For local Grafana/Loki/Alloy/Prometheus, use:

```bash
export GRAFANA_ADMIN_PASSWORD="$(openssl rand -base64 32)"
docker compose -f podman-compose.yml --profile observability up -d
```

The local profile is intentionally conservative: Grafana binds to `127.0.0.1`,
Loki and Prometheus do not publish host ports, Loki uses a tenant header, and
Alloy tails only the backend log volume rather than mounting the Docker socket.
See `observability/README.md` before using the profile with sensitive data.

## Logging Rules

- Use `tracing::{debug, info, warn, error}`.
- Use structured fields for IDs, counts, provider names, and durations.
- Never log credentials, tokens, secrets, or raw connection strings.
- Never log PHI, request bodies, tenant records, raw SQL parameters, or provider
  result payloads.
- Redact sensitive config as soon as it is loaded.
- Prefer spans for high-value operations.

## Instrumentation

Use `#[tracing::instrument]` on paths that are expensive, security-sensitive, or
hard to debug:

```rust
#[tracing::instrument(skip(data_access, selections), fields(schema = %schema_name, entity = %entity_name))]
pub async fn run_custom_query(
    schema_name: &str,
    entity_name: &str,
    data_access: &DataAccess,
    selections: serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    // ...
}
```

Good candidates:

- Access checks.
- Query/filter compilation.
- Provider execution.
- Custom business workflows.
- External service calls.

## Error Events

Prefer structured errors:

```rust
tracing::error!(
    error = %err,
    data_source = %data_source_name,
    "provider query failed"
);
```

Avoid interpolating large request bodies or raw SQL parameter values unless they
are known safe.

## Request Correlation

The backend treats `x-request-id` and `x-correlation-id` as first-class request
context. If a caller sends `x-request-id`, the backend reuses it; otherwise the
request-id middleware creates one. If `x-correlation-id` is absent, the backend
uses the request ID as the correlation ID.

Request context is attached to:

- HTTP spans as `http.request_id` and `app.correlation_id`.
- HTTP response headers.
- GraphQL response extensions and GraphQL error extensions.
- Admin troubleshooting responses.
- API-test failure output.

This keeps correlation usable from logs, traces, admin diagnostics, and failing
contract tests without passing string IDs through provider and resolver APIs.

## Health And Readiness

`/health/live` and `/livez` are process-only liveness checks. `/health`,
`/health/ready`, and `/readyz` are readiness checks and include structured
provider probes for each schema data source initialized by the backend router.

Provider readiness checks include the schema name, data source name, provider
type, status, message, and probe duration. Liveness should stay cheap and should
not depend on database availability.

## Query Performance

Provider operations record structured slow-query events with provider, data source,
entity, operation, duration, `query_count`, `result_count`, request ID, and
correlation ID. The threshold defaults to 500 ms and can be tuned with:

```text
APP_SLOW_QUERY_THRESHOLD_MS=500
```

Query planning also applies a provider-neutral cost budget before provider
compilation. The budget accounts for filter shape, sort shape, pagination,
relationship depth, many-to-many expansion, and aggregation shape.

```text
APP_QUERY_COST_MAX=1000
APP_AGGREGATE_QUERY_COST_MAX=1500
APP_QUERY_RELATIONSHIP_DEPTH_MAX=4
APP_QUERY_MANY_TO_MANY_EXPANSION_MAX=4
APP_QUERY_FILTER_PREDICATES_MAX=25
APP_QUERY_SORT_SPECS_MAX=3
APP_AGGREGATE_OUTPUTS_MAX=12
APP_QUERY_SELECTED_FIELDS_MAX=80
APP_QUERY_OFFSET_ROWS_MAX=5000
```

Generated list queries support offset pagination through `skip` and `limit`.
They also accept an optional opaque `after` cursor for keyset-style pagination.
When `after` is omitted, the query plan can emit `next_cursor` from the last
item in the result set. Keep cursors opaque to callers.

Generated list and aggregate methods apply defaults when `skip` or `limit` is
omitted:

```text
APP_QUERY_DEFAULT_PAGE_SIZE=50
APP_QUERY_MAX_PAGE_SIZE=250
```

The max page size is enforced in QueryIR, so direct backend paths and generated
GraphQL paths share the same runtime policy.

Admin troubleshooting exposes a safe query diagnostic endpoint at
`/admin/troubleshooting/query/diagnose` when
`APP_ADMIN_TROUBLESHOOTING_ENABLED=true`. Diagnostics include QueryIR cost,
active budget, provider identity, pagination policy, and provider diagnostic
status. Providers must only add EXPLAIN output when it can be redacted safely.

## Provider Pool Metrics

`/metrics` and `/metrics.json` refresh provider pool gauges from readiness
state before emitting metrics.

Provider operations expose both Prometheus-style local metrics and OTLP metrics:

```text
app_provider_operations_total
app_provider_query_count_total
app_provider_result_count_total
app_provider_operation_duration_seconds
```

In OTLP, the same signals are emitted as:

```text
app.provider.operations
app.provider.query_count
app.provider.result_count
app.provider.operation.duration
```

Instrumented provider pools expose:

```text
app_provider_pool_instrumented
app_provider_pool_pressure
app_provider_pool_connections{state="max|size|available|in_use|waiting"}
```

Some drivers do not expose pool internals; those providers report opaque pool
stats with `app_provider_pool_instrumented` set to `0`.

## Verification

Run:

```bash
scripts/appfw test
```

For observability changes that affect startup, also run the backend locally:

```bash
ENV_NAME=local API_PORT=8080 scripts/appfw serve
```
