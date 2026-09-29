#!/usr/bin/env bash
#
# ops-certification-ready-fixture.sh - local structural preflight for the
# live-required operations certification gate.
#
# This writes synthetic retained live-ops evidence under
# target/appfw/ops-ready-fixture by default, then runs the real
# ops-certification command with live evidence required. The result proves the
# retained live-ops evidence shape is acceptable to the gate. It is not release
# evidence and must not be promoted.
#
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
cd "$repo_root"

fixture_dir="${APPFW_OPS_CERTIFICATION_FIXTURE_DIR:-target/appfw/ops-ready-fixture}"
output_file="$fixture_dir/ops-certification-ready-fixture-output.json"

ensure_python3() {
  if command -v python3 >/dev/null 2>&1; then
    return 0
  fi

  echo "ops-certification-ready-fixture: python3 is required" >&2
  exit 2
}

ensure_python3
mkdir -p "$fixture_dir"

python3 - "$fixture_dir" <<'PY'
import datetime as dt
import json
import sys
from pathlib import Path

fixture_dir = Path(sys.argv[1])
generated_at = (
    dt.datetime.now(dt.timezone.utc)
    .replace(microsecond=0)
    .isoformat()
    .replace("+00:00", "Z")
)


def write_json(name, value):
    path = fixture_dir / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return path


common_note = (
    "Synthetic structural fixture only; not live operations evidence "
    "or release approval."
)

write_json(
    "live-otel-evidence.json",
    {
        "fixture": True,
        "ok": True,
        "generated_at_utc": generated_at,
        "note": common_note,
        "otel": {
            "app_otel_enabled": True,
            "service_name_configured": True,
            "shared_endpoint_configured": True,
            "traces_endpoint_configured": True,
            "metrics_endpoint_configured": True,
            "protocol_configured": True,
        },
    },
)
write_json(
    "health-ready.json",
    {
        "fixture": True,
        "ok": True,
        "generated_at_utc": generated_at,
        "status": "ready",
        "note": common_note,
    },
)
(fixture_dir / "metrics.prom").write_text(
    "# HELP app_http_requests_total Synthetic request counter\n"
    "# TYPE app_http_requests_total counter\n"
    "app_http_requests_total{method=\"GET\",route=\"/health/ready\",status=\"200\"} 1\n",
    encoding="utf-8",
)
write_json(
    "metrics.json",
    {
        "fixture": True,
        "ok": True,
        "generated_at_utc": generated_at,
        "metrics_path": "/metrics",
        "note": common_note,
    },
)
write_json(
    "prometheus-targets.json",
    {
        "fixture": True,
        "ok": True,
        "generated_at_utc": generated_at,
        "targets": [
            {
                "health": "up",
                "job": "app-framework",
                "scrape_url": "http://127.0.0.1:8080/metrics",
            }
        ],
        "note": common_note,
    },
)
write_json(
    "prometheus-rules.json",
    {
        "fixture": True,
        "ok": True,
        "generated_at_utc": generated_at,
        "rules": [
            {
                "alert": "ProviderSlowOperationsP95",
                "state": "inactive",
            }
        ],
        "note": common_note,
    },
)
write_json(
    "alertmanager-status.json",
    {
        "fixture": True,
        "ok": True,
        "generated_at_utc": generated_at,
        "live_endpoint": True,
        "status": "success",
        "versionInfo": {
            "version": "local-fixture",
        },
        "cluster": {
            "status": "ready",
        },
        "note": common_note,
    },
)
write_json(
    "grafana-dashboard-provisioning.json",
    {
        "fixture": True,
        "ok": True,
        "generated_at_utc": generated_at,
        "dashboards": [
            {
                "name": "app-framework-backend",
                "provisioned": True,
            }
        ],
        "note": common_note,
    },
)
write_json(
    "runbook-drill.json",
    {
        "fixture": True,
        "ok": True,
        "generated_at_utc": generated_at,
        "steps": [
            {
                "name": "confirm-alert-routing",
                "passed": True,
            }
        ],
        "note": common_note,
    },
)
PY

if ! APPFW_RELEASE_ARTIFACT_DIR="$fixture_dir" \
  APPFW_RELEASE_REQUIRE_OPS_CERTIFICATION=true \
  APPFW_RELEASE_REQUIRE_LIVE_OPS_EVIDENCE=true \
  scripts/appfw framework ops-certification --json >"$output_file"; then
  cat "$output_file" >&2 || true
  echo "ops-certification-ready-fixture: FAILED" >&2
  exit 1
fi

python3 - "$fixture_dir/ops-certification.json" <<'PY'
import json
import sys
from pathlib import Path

artifact = Path(sys.argv[1])
report = json.loads(artifact.read_text(encoding="utf-8"))
required_live_artifacts = {
    "live-otel-evidence.json",
    "health-ready.json",
    "metrics.prom",
    "metrics.json",
    "prometheus-targets.json",
    "prometheus-rules.json",
    "alertmanager-status.json",
    "grafana-dashboard-provisioning.json",
    "runbook-drill.json",
}
retained_live_artifacts = {
    Path(item.get("path", "")).name
    for item in report.get("artifacts", [])
    if isinstance(item, dict)
    and item.get("name") == "live-ops-evidence"
    and item.get("present") is True
    and item.get("required") is True
    and isinstance(item.get("sha256"), str)
    and isinstance(item.get("bytes"), int)
    and item.get("bytes", 0) > 0
}
failed_checks = [
    check.get("name", "<unnamed>")
    for check in report.get("checks", [])
    if isinstance(check, dict) and check.get("ok") is not True
]

problems = []
if report.get("ok") is not True:
    problems.append("ok is not true")
if report.get("release_ready") is not True:
    problems.append("release_ready is not true")
if report.get("live_evidence_required") is not True:
    problems.append("live_evidence_required is not true")
if report.get("failure_count") != 0:
    problems.append(f"failure_count is {report.get('failure_count')!r}")
if report.get("release_blockers"):
    problems.append("release_blockers is not empty")
missing = sorted(required_live_artifacts - retained_live_artifacts)
if missing:
    problems.append("missing retained live artifacts: " + ", ".join(missing))
if failed_checks:
    problems.append("failing checks: " + ", ".join(failed_checks[:20]))

if problems:
    print("ops-certification-ready-fixture: FAILED", file=sys.stderr)
    for problem in problems:
        print(f"  - {problem}", file=sys.stderr)
    sys.exit(1)

print("ops-certification-ready-fixture: OK")
print(f"  evidence: {artifact.as_posix()}")
print("  note: synthetic structural fixture only; not release evidence")
PY
