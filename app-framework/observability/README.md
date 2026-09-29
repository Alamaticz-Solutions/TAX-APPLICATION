# Secure Local Observability

This profile is a local development mirror for the production observability
shape. It is not a replacement for a managed HIPAA control plane, encrypted
object storage, SSO/RBAC, audit logging, or legal/compliance review.

The local stack uses:

- Grafana for dashboards and log exploration.
- Prometheus for backend `/metrics` and alert-rule evaluation.
- Alertmanager for alert routing and notification.
- Loki for backend logs.
- Alloy for log collection and redaction-in-depth.

## Alerting

Alerting follows a single path: **Prometheus evaluates the rules and
Alertmanager routes them.** Grafana is the visualization layer only and does
not own any alert rules (see
`grafana/provisioning/alerting/alerting.yml`), so there is exactly one
evaluation engine.

- Rules: `prometheus/alerts.yml`, wired into `prometheus/prometheus.yml` via
  `rule_files` and the `alerting.alertmanagers` block.
- Routing/receivers: `alertmanager/alertmanager.yml`, run as the
  `alertmanager` service in the `observability` Compose profile.

Rules cover, at minimum:

| Alert | Condition | Metric |
|-------|-----------|--------|
| `BackendTargetDown` | backend scrape target unreachable (`up == 0`) for 2m | `up{job="app-framework-backend"}` |
| `BackendHigh5xxErrorRate` | >5% of responses are 5xx over 5m | `app_http_requests_total{status=~"5.."}` |
| `BackendHighRequestLatencyP95` | p95 request latency > 1s sustained 10m | `app_http_request_duration_seconds_bucket` |
| `ProviderSlowOperationsP95` | provider operation p95 latency > 1s sustained 10m | `app_provider_operation_duration_seconds_bucket` |
| `ProviderPoolSaturation` | pool pressure > 0.90 for 5m | `app_provider_pool_pressure` |

Metric names match what the backend emits from
`appfw_runtime/src/observability/metrics.rs`. Thresholds are conservative
starting points; tune them per environment.

## Runbook Evidence

The backend exposes two complementary evidence surfaces:

- `/health/ready` for provider readiness, request context, and summarized
  data-source status.
- `/metrics` for Prometheus alerts and Grafana panels. Provider operation
  metrics include `app_provider_operation_duration_seconds`,
  `app_provider_operation_duration_seconds_max`,
  `app_provider_query_count_total`, and
  `app_provider_result_count_total`.
- `/metrics.json` for handoff/debug snapshots. Provider operation snapshots
  include `operation_count`, `query_count`, `result_count`,
  `duration_avg_seconds`, and `duration_max_seconds` by provider, data source,
  entity, and operation.

For `ProviderSlowOperationsP95`, start with the Grafana provider latency,
provider max-duration, query/result-count, and pool-pressure panels. If latency
tracks high query counts, result counts, or pool pressure, reduce fan-out,
tighten pagination/filters, or tune the provider pool. If the slow path is
isolated to a query shape, use approved admin diagnostics or safe EXPLAIN hooks
only after confirming request/correlation IDs and redacted logs do not contain
PHI or secrets.

Machine-checkable proof:

```bash
cargo test -p appfw-runtime observability_bundle_links_provider_latency_alert_dashboard_and_runbook
```

This parses the committed Grafana dashboard and Prometheus alert YAML, then
checks they still reference the provider latency, max-duration, query/result,
and pool-pressure metrics described by this runbook.

## OpenTelemetry Export Contract

OpenTelemetry export is opt-in and uses the standard OTLP environment contract
already consumed by the runtime exporter:

| Variable | Purpose |
|----------|---------|
| `APP_OTEL_ENABLED` | Enables OTLP trace and metric export when set to `true`, `1`, `yes`, or `on`. |
| `OTEL_SERVICE_NAME` | Preferred service name for OTEL resource attributes. |
| `APP_SERVICE_NAME` | Fallback service name when `OTEL_SERVICE_NAME` is absent. |
| `ENV_NAME` | Deployment environment name for OTEL resource attributes. |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | Shared OTLP endpoint when traces and metrics use the same collector. |
| `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT` | Trace-specific OTLP endpoint. |
| `OTEL_EXPORTER_OTLP_METRICS_ENDPOINT` | Metric-specific OTLP endpoint. |
| `OTEL_EXPORTER_OTLP_PROTOCOL` | OTLP protocol; local proof expects `http/protobuf`. |

The runtime resource attributes are `service.name` and
`deployment.environment.name`. Incoming W3C trace context is accepted and
response trace context is injected through `traceparent` and `tracestate`.
Application correlation remains available through `x-request-id` and
`x-correlation-id` response headers, GraphQL extensions, and span/log metadata.

Request correlation fields are intentionally not Loki labels. The runtime emits
`http.request_id` and `app.correlation_id` as span/log metadata, and Alloy keeps
those fields as structured metadata so support can join logs, traces, and
GraphQL errors without increasing label cardinality.

Machine-checkable proof:

```bash
cargo test -p appfw-runtime otel_export_contract_is_reflected_in_alloy_and_runbook
```

**Receivers are per-environment placeholders.** The committed Alertmanager
receiver is a non-delivering local webhook with **no secrets**. Each
environment must replace it with its own paging/notification integration
(PagerDuty, Slack, Opsgenie, email gateway, etc.) supplied through
environment-specific secret injection — never committed to this repo.

## PHI Posture

Treat logs as regulated data. The primary control is source minimization:
application code must not log PHI, request bodies, tenant records, credentials,
tokens, raw SQL parameters, or provider result payloads.

Alloy applies fallback redaction for common accidental patterns such as email
addresses, US phone numbers, SSN-like values, and Luhn-matching identifiers.
That collector redaction is defense in depth only; it is not a guarantee that
all PHI has been removed.

High-cardinality identifiers such as request IDs and correlation IDs are sent as
structured metadata instead of Loki labels. Loki labels should remain bounded
to operational dimensions such as service, environment, and log level.

## Local Controls

- Grafana and Alertmanager are bound to `127.0.0.1` only.
- Loki, Alloy, and Prometheus do not publish host ports.
- Loki runs with multi-tenancy enabled and the local tenant
  `app-framework-local`.
- Observability services run on a separate Compose network from the backend.
- Prometheus is the only observability service attached to the app network.
- Grafana anonymous access and sign-up are disabled.
- Grafana analytics and external snapshots are disabled.
- Prometheus and Loki retention default to 24 hours.
- The shared backend log handoff volume is tmpfs-backed, and Loki, Prometheus,
  Alloy, and Grafana runtime stores use container-local tmpfs so log/query data
  is not persisted by default.
- Alloy does not mount the Docker socket.

Set a unique local Grafana password before starting the profile:

```bash
export GRAFANA_ADMIN_USER=admin
export GRAFANA_ADMIN_PASSWORD="$(openssl rand -base64 32)"
docker compose -f podman-compose.yml --profile observability up -d
```

Open Grafana at:

```text
http://127.0.0.1:3001
```

Stop the local observability containers when finished:

```bash
docker compose -f podman-compose.yml --profile observability stop grafana prometheus alertmanager alloy loki
```

## Production Controls

For environments where logs can contain PHI, use the same telemetry shape but
with platform controls:

- Put Grafana behind SSO, MFA, RBAC, and audit logging.
- Put Loki behind an authenticating reverse proxy or gateway; Loki itself does
  not provide an authentication layer.
- Use Loki tenant isolation with `X-Scope-OrgID`.
- Use TLS or mTLS between collectors, gateways, Loki, Prometheus, and Grafana.
- Use encrypted-at-rest storage with managed keys and explicit retention.
- Route logs by environment and tenant without storing tenant identifiers or PHI
  as indexed Loki labels.
- Scan and pin images by digest in deployment repos.
- Keep admin troubleshooting endpoints disabled unless approved for an
  operational incident.
