#!/usr/bin/env bash
#
# live-ops-evidence.sh - collect live observability evidence from a running
# backend and the committed observability bundle.
#
# This writes the default artifacts consumed by `scripts/appfw ops-certification`
# when APPFW_RELEASE_REQUIRE_LIVE_OPS_EVIDENCE=true.
#
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
cd "$repo_root"

report_dir="${APPFW_RELEASE_ARTIFACT_DIR:-target/appfw}"
base_url="${APPFW_OPS_BASE_URL:-${APPFW_SECURITY_ASSURANCE_BASE_URL:-http://127.0.0.1:8080}}"
timeout_secs="${APPFW_OPS_EVIDENCE_TIMEOUT_SECS:-15}"

usage() {
  cat <<'USAGE'
Usage:
  APPFW_OPS_BASE_URL=http://127.0.0.1:8080 \
  APPFW_ALERTMANAGER_URL=http://127.0.0.1:9093 \
    bash scripts/ci/live-ops-evidence.sh

Writes:
  target/appfw/live-otel-evidence.json
  target/appfw/health-ready.json
  target/appfw/metrics.prom
  target/appfw/metrics.json
  target/appfw/prometheus-targets.json
  target/appfw/prometheus-rules.json
  target/appfw/alertmanager-status.json
  target/appfw/grafana-dashboard-provisioning.json
  target/appfw/runbook-drill.json
USAGE
}

case "${1:-}" in
  "")
    ;;
  -h | --help)
    usage
    exit 0
    ;;
  *)
    echo "live-ops-evidence: unknown argument: $1" >&2
    usage >&2
    exit 2
    ;;
esac

mkdir -p "$report_dir"

python3 - "$repo_root" "$report_dir" "$base_url" "$timeout_secs" <<'PY'
import datetime as dt
import hashlib
import json
import os
import re
import sys
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

repo_root = Path(sys.argv[1])
report_dir = Path(sys.argv[2])
base_url = sys.argv[3].rstrip("/")
timeout_secs = float(sys.argv[4])

failures = []


def utc_now():
    return (
        dt.datetime.now(dt.timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z")
    )


def rel(path):
    try:
        return path.resolve().relative_to(repo_root.resolve()).as_posix()
    except Exception:
        return path.as_posix()


def sha256_bytes(value):
    return hashlib.sha256(value).hexdigest()


def sha256_file(path):
    return sha256_bytes(path.read_bytes())


def write_json(name, value):
    path = report_dir / name
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return path


def read_text(path):
    return path.read_text(encoding="utf-8") if path.is_file() else ""


def redact_url(url):
    parsed = urllib.parse.urlsplit(url)
    netloc = parsed.hostname or ""
    if parsed.port:
        netloc = f"{netloc}:{parsed.port}"
    return urllib.parse.urlunsplit((parsed.scheme, netloc, parsed.path, "", ""))


def fetch_url(url, expect_json=False, headers=None):
    request = urllib.request.Request(url, headers=headers or {})
    try:
        with urllib.request.urlopen(request, timeout=timeout_secs) as response:
            body = response.read()
            text = body.decode("utf-8", errors="replace")
            parsed = None
            if expect_json:
                parsed = json.loads(text)
            return {
                "ok": 200 <= int(response.status) < 300,
                "url": redact_url(url),
                "status": int(response.status),
                "headers": {
                    key.lower(): value
                    for key, value in response.headers.items()
                    if key.lower()
                    in {
                        "content-type",
                        "x-request-id",
                        "x-correlation-id",
                        "traceparent",
                        "tracestate",
                    }
                },
                "body": body,
                "text": text,
                "json": parsed,
                "sha256": sha256_bytes(body),
            }
    except Exception as exc:
        return {
            "ok": False,
            "url": redact_url(url),
            "status": None,
            "error": str(exc),
            "body": b"",
            "text": "",
            "json": None,
            "sha256": None,
        }


def fetch(path, expect_json=False, headers=None):
    return fetch_url(f"{base_url}{path}", expect_json=expect_json, headers=headers)


def summarize_alertmanager_status(value):
    if not isinstance(value, dict):
        return {
            "semantic_ok": False,
            "reason": "status response is not a JSON object",
        }

    status = value.get("status")
    data = value.get("data")
    if status == "success" and isinstance(data, dict):
        status_body = data
        api_shape = "wrapped"
    else:
        status_body = value
        api_shape = "direct"

    version_info = status_body.get("versionInfo")
    version = version_info.get("version") if isinstance(version_info, dict) else None
    config_yaml = status_body.get("configYAML")
    config = status_body.get("config")
    config_original = config.get("original") if isinstance(config, dict) else None
    has_config_yaml = any(
        isinstance(item, str) and "route:" in item
        for item in (config_yaml, config_original)
    )
    has_cluster = isinstance(status_body.get("cluster"), dict)
    semantic_ok = bool(version and (has_config_yaml or has_cluster))
    return {
        "semantic_ok": semantic_ok,
        "api_shape": api_shape,
        "status": status,
        "version": version,
        "has_config_yaml": has_config_yaml,
        "has_cluster": has_cluster,
        "reason": None if semantic_ok else "missing Alertmanager versionInfo plus config/cluster status",
    }


def parse_prometheus_targets(text):
    jobs = []
    current = None
    in_targets = False
    for raw_line in text.splitlines():
        line = raw_line.strip()
        match = re.match(r"-\s+job_name:\s+(.+)", line)
        if match:
            current = {
                "job_name": match.group(1).strip().strip('"'),
                "targets": [],
                "metrics_path": "/metrics",
            }
            jobs.append(current)
            in_targets = False
            continue
        if current is None:
            continue
        match = re.match(r"metrics_path:\s+(.+)", line)
        if match:
            current["metrics_path"] = match.group(1).strip().strip('"')
            continue
        if line == "- targets:" or line == "targets:":
            in_targets = True
            continue
        match = re.match(r"-\s+([^#]+)", line)
        if in_targets and match:
            value = match.group(1).strip().strip('"')
            if value and ":" in value:
                current["targets"].append(value)
    return jobs


def summarize_prometheus_metrics(text):
    metric_names = []
    help_lines = 0
    type_lines = 0
    for line in text.splitlines():
        if line.startswith("# HELP "):
            help_lines += 1
            parts = line.split()
            if len(parts) >= 3:
                metric_names.append(parts[2])
        elif line.startswith("# TYPE "):
            type_lines += 1
    expected = [
        "app_http_requests_total",
        "app_http_request_duration_seconds_bucket",
        "app_provider_operation_duration_seconds_bucket",
        "app_provider_operation_duration_seconds_max",
        "app_provider_query_count_total",
        "app_provider_result_count_total",
        "app_provider_pool_pressure",
    ]
    return {
        "line_count": len(text.splitlines()),
        "help_lines": help_lines,
        "type_lines": type_lines,
        "metric_names": sorted(set(metric_names)),
        "expected_metrics": [
            {"name": name, "present": name in text}
            for name in expected
        ],
    }


generated_at = utc_now()
health = fetch("/health/ready", expect_json=True)
metrics_prom = fetch("/metrics")
metrics_json = fetch("/metrics.json", expect_json=True)
trace_probe = fetch(
    "/health/live",
    expect_json=True,
    headers={
        "traceparent": "00-11111111111111111111111111111111-2222222222222222-01",
        "x-request-id": "appfw-live-ops-evidence",
        "x-correlation-id": "appfw-live-ops-evidence",
    },
)

if not health["ok"]:
    failures.append(f"/health/ready did not return a 2xx JSON response from {health['url']}")
if not metrics_prom["ok"]:
    failures.append(f"/metrics did not return a 2xx response from {metrics_prom['url']}")
if not metrics_json["ok"]:
    failures.append(f"/metrics.json did not return a 2xx JSON response from {metrics_json['url']}")

health_path = write_json(
    "health-ready.json",
    {
        "command": "live-ops-evidence",
        "ok": health["ok"],
        "generated_at_utc": generated_at,
        "base_url": base_url,
        "probe": {
            "path": "/health/ready",
            "status": health["status"],
            "headers": health.get("headers", {}),
            "body_sha256": health["sha256"],
        },
        "response": health["json"],
        "error": health.get("error"),
    },
)

metrics_prom_path = report_dir / "metrics.prom"
metrics_prom_path.write_text(metrics_prom["text"], encoding="utf-8")
metrics_summary = summarize_prometheus_metrics(metrics_prom["text"])
if not metrics_prom["text"].strip():
    failures.append("/metrics response was empty")

metrics_json_path = write_json(
    "metrics.json",
    {
        "command": "live-ops-evidence",
        "ok": metrics_json["ok"],
        "generated_at_utc": generated_at,
        "base_url": base_url,
        "probe": {
            "path": "/metrics.json",
            "status": metrics_json["status"],
            "headers": metrics_json.get("headers", {}),
            "body_sha256": metrics_json["sha256"],
        },
        "snapshot": metrics_json["json"],
        "prometheus_summary": metrics_summary,
        "error": metrics_json.get("error"),
    },
)

prometheus_config = repo_root / "observability/prometheus/prometheus.yml"
prometheus_rules = repo_root / "observability/prometheus/alerts.yml"
alertmanager_config = repo_root / "observability/alertmanager/alertmanager.yml"
grafana_dashboard = repo_root / "observability/grafana/dashboards/app-framework-backend.json"
grafana_datasources = repo_root / "observability/grafana/provisioning/datasources/datasources.yml"
grafana_dashboards = repo_root / "observability/grafana/provisioning/dashboards/dashboards.yml"
observability_runbook = repo_root / "observability/README.md"

prometheus_text = read_text(prometheus_config)
prometheus_jobs = parse_prometheus_targets(prometheus_text)
write_json(
    "prometheus-targets.json",
    {
        "command": "live-ops-evidence",
        "ok": prometheus_config.is_file() and any(job["targets"] for job in prometheus_jobs),
        "generated_at_utc": generated_at,
        "source": rel(prometheus_config),
        "source_sha256": sha256_file(prometheus_config) if prometheus_config.is_file() else None,
        "scrape_jobs": prometheus_jobs,
        "release_note": "Targets are the committed Prometheus release configuration; backend reachability is proven separately by health-ready.json and metrics.prom.",
    },
)

rules_text = read_text(prometheus_rules)
alerts = re.findall(r"^\s*-\s+alert:\s+([A-Za-z0-9_:-]+)\s*$", rules_text, re.MULTILINE)
write_json(
    "prometheus-rules.json",
    {
        "command": "live-ops-evidence",
        "ok": prometheus_rules.is_file() and len(alerts) >= 1,
        "generated_at_utc": generated_at,
        "source": rel(prometheus_rules),
        "source_sha256": sha256_file(prometheus_rules) if prometheus_rules.is_file() else None,
        "alerts": alerts,
        "required_alerts": [
            {"name": name, "present": name in alerts}
            for name in [
                "BackendTargetDown",
                "BackendHigh5xxErrorRate",
                "BackendHighRequestLatencyP95",
                "ProviderSlowOperationsP95",
                "ProviderPoolSaturation",
            ]
        ],
    },
)

alertmanager_text = read_text(alertmanager_config)
alertmanager_static_ok = (
    alertmanager_config.is_file()
    and "route:" in alertmanager_text
    and "receivers:" in alertmanager_text
)
alertmanager_url = os.environ.get("APPFW_ALERTMANAGER_URL", "").strip().rstrip("/")
alertmanager_probe = None
alertmanager_live_ok = False
if alertmanager_url:
    ready_probe = fetch_url(f"{alertmanager_url}/-/ready")
    status_probe = fetch_url(f"{alertmanager_url}/api/v2/status", expect_json=True)
    status_summary = summarize_alertmanager_status(status_probe.get("json"))
    alertmanager_live_ok = status_probe.get("ok") is True and status_summary.get("semantic_ok") is True
    alertmanager_probe = {
        "ready": {
            "path": "/-/ready",
            "url": ready_probe.get("url"),
            "status": ready_probe.get("status"),
            "ok": ready_probe.get("ok"),
            "body_sha256": ready_probe.get("sha256"),
            "error": ready_probe.get("error"),
        },
        "status": {
            "path": "/api/v2/status",
            "url": status_probe.get("url"),
            "status": status_probe.get("status"),
            "ok": status_probe.get("ok"),
            "body_sha256": status_probe.get("sha256"),
            "error": status_probe.get("error"),
            "semantic": status_summary,
        },
    }

write_json(
    "alertmanager-status.json",
    {
        "command": "live-ops-evidence",
        "ok": alertmanager_static_ok and (not alertmanager_url or alertmanager_live_ok),
        "generated_at_utc": generated_at,
        "source": rel(alertmanager_config),
        "source_sha256": sha256_file(alertmanager_config) if alertmanager_config.is_file() else None,
        "configured_endpoint": bool(alertmanager_url),
        "live_endpoint": alertmanager_live_ok,
        "status": "live-ok"
        if alertmanager_live_ok
        else ("live-unreachable" if alertmanager_url else "static-config-retained"),
        "probe": alertmanager_probe,
        "secret_values_retained": False,
    },
)

dashboard_ok = False
dashboard_title = None
dashboard_panels = []
if grafana_dashboard.is_file():
    try:
        dashboard_value = json.loads(grafana_dashboard.read_text(encoding="utf-8"))
        dashboard_title = dashboard_value.get("title")
        dashboard_panels = [
            panel.get("title")
            for panel in dashboard_value.get("panels", [])
            if isinstance(panel, dict)
        ]
        dashboard_ok = True
    except Exception:
        dashboard_ok = False

write_json(
    "grafana-dashboard-provisioning.json",
    {
        "command": "live-ops-evidence",
        "ok": dashboard_ok and grafana_datasources.is_file() and grafana_dashboards.is_file(),
        "generated_at_utc": generated_at,
        "dashboard": {
            "path": rel(grafana_dashboard),
            "sha256": sha256_file(grafana_dashboard) if grafana_dashboard.is_file() else None,
            "title": dashboard_title,
            "panels": dashboard_panels,
        },
        "provisioning": [
            {
                "path": rel(path),
                "present": path.is_file(),
                "sha256": sha256_file(path) if path.is_file() else None,
            }
            for path in [grafana_datasources, grafana_dashboards]
        ],
    },
)

runbook_text = read_text(observability_runbook)
runbook_tokens = [
    "ProviderSlowOperationsP95",
    "/health/ready",
    "/metrics",
    "/metrics.json",
    "OpenTelemetry Export Contract",
]
write_json(
    "runbook-drill.json",
    {
        "command": "live-ops-evidence",
        "ok": observability_runbook.is_file() and all(token in runbook_text for token in runbook_tokens),
        "generated_at_utc": generated_at,
        "scope": "local-live-backend",
        "runbook": {
            "path": rel(observability_runbook),
            "sha256": sha256_file(observability_runbook) if observability_runbook.is_file() else None,
        },
        "steps": [
            {
                "name": "readiness-probe",
                "artifact": rel(health_path),
                "passed": health["ok"],
            },
            {
                "name": "prometheus-metrics-probe",
                "artifact": rel(metrics_prom_path),
                "passed": metrics_prom["ok"] and bool(metrics_prom["text"].strip()),
            },
            {
                "name": "structured-metrics-probe",
                "artifact": rel(metrics_json_path),
                "passed": metrics_json["ok"],
            },
            {
                "name": "trace-context-probe",
                "passed": trace_probe["ok"],
                "headers": trace_probe.get("headers", {}),
            },
        ],
        "required_runbook_tokens": [
            {"token": token, "present": token in runbook_text}
            for token in runbook_tokens
        ],
    },
)

otel_enabled = os.environ.get("APP_OTEL_ENABLED", "").strip().lower() in {"1", "true", "yes", "on"}
write_json(
    "live-otel-evidence.json",
    {
        "command": "live-ops-evidence",
        "ok": not failures,
        "generated_at_utc": generated_at,
        "scope": "local-live-backend",
        "base_url": base_url,
        "otel": {
            "app_otel_enabled": otel_enabled,
            "service_name_configured": bool(os.environ.get("OTEL_SERVICE_NAME") or os.environ.get("APP_SERVICE_NAME")),
            "shared_endpoint_configured": bool(os.environ.get("OTEL_EXPORTER_OTLP_ENDPOINT")),
            "traces_endpoint_configured": bool(os.environ.get("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT")),
            "metrics_endpoint_configured": bool(os.environ.get("OTEL_EXPORTER_OTLP_METRICS_ENDPOINT")),
            "protocol_configured": bool(os.environ.get("OTEL_EXPORTER_OTLP_PROTOCOL")),
            "secret_values_retained": False,
        },
        "probes": [
            {"path": "/health/ready", "status": health["status"], "ok": health["ok"]},
            {"path": "/metrics", "status": metrics_prom["status"], "ok": metrics_prom["ok"]},
            {"path": "/metrics.json", "status": metrics_json["status"], "ok": metrics_json["ok"]},
            {"path": "/health/live", "status": trace_probe["status"], "ok": trace_probe["ok"]},
        ],
        "artifacts": [
            {"name": "health-ready", "path": rel(health_path), "sha256": sha256_file(health_path)},
            {"name": "metrics-prometheus", "path": rel(metrics_prom_path), "sha256": sha256_file(metrics_prom_path)},
            {"name": "metrics-json", "path": rel(metrics_json_path), "sha256": sha256_file(metrics_json_path)},
        ],
        "failures": failures,
    },
)

if failures:
    print("live-ops-evidence: FAILED", file=sys.stderr)
    for failure in failures:
        print(f"  - {failure}", file=sys.stderr)
    sys.exit(1)

print(f"live-ops-evidence: OK; evidence written to {report_dir}")
PY
