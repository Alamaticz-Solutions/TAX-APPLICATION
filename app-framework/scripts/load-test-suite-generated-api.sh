#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"

usage() {
  cat <<'EOF'
load-test-suite-generated-api - generated GraphQL API scenario load suite

Usage:
  scripts/load-test-suite-generated-api.sh [options]

Options:
  --json                 Emit the suite JSON summary
  --url URL              GraphQL endpoint URL
  --requests N           Requests per scenario
  --concurrency N        Requests to run per batch
  --header NAME:VALUE    Extra header; can be repeated
  --only NAME            Run only one scenario from the suite
  --max-p95-ms N         Fail a scenario when p95 latency exceeds N milliseconds
  --max-max-ms N         Fail a scenario when max latency exceeds N milliseconds
  --max-error-rate N     Fail a scenario when failed / total exceeds N
  --output-dir PATH      Directory for per-scenario JSON artifacts
  --summary-output PATH  Also write the suite summary JSON to PATH
  --help                 Show this help

Environment:
  APPFW_LOAD_TEST_URL
  APPFW_LOAD_TEST_REQUESTS
  APPFW_LOAD_TEST_CONCURRENCY
  APPFW_LOAD_TEST_TOKEN
  APPFW_LOAD_TEST_TIMEZONE
  APPFW_LOAD_TEST_MAX_P95_MS
  APPFW_LOAD_TEST_MAX_MAX_MS
  APPFW_LOAD_TEST_MAX_ERROR_RATE
  APPFW_LOAD_TEST_SUITE_OUTPUT_DIR
  APPFW_LOAD_TEST_SUITE_SUMMARY
  APPFW_LOAD_TEST_SUITE_ONLY
EOF
}

json_escape() {
  local value="$1"
  value="${value//\\/\\\\}"
  value="${value//\"/\\\"}"
  value="${value//$'\n'/\\n}"
  value="${value//$'\r'/\\r}"
  value="${value//$'\t'/\\t}"
  printf '"%s"' "$value"
}

json_result() {
  local ok="$1"
  local detail="$2"
  printf '{"command":"load-test-suite","ok":%s,"detail":' "$ok"
  json_escape "$detail"
  printf '}\n'
}

usage_error() {
  local message="$1"
  if [[ "${json:-0}" == "1" ]]; then
    json_result false "$message"
  else
    echo "$message" >&2
  fi
  exit 2
}

json=0
url="${APPFW_LOAD_TEST_URL:-http://127.0.0.1:8080/crm}"
requests="${APPFW_LOAD_TEST_REQUESTS:-100}"
concurrency="${APPFW_LOAD_TEST_CONCURRENCY:-8}"
max_p95_ms="${APPFW_LOAD_TEST_MAX_P95_MS:-}"
max_max_ms="${APPFW_LOAD_TEST_MAX_MAX_MS:-}"
max_error_rate="${APPFW_LOAD_TEST_MAX_ERROR_RATE:-0}"
output_dir="${APPFW_LOAD_TEST_SUITE_OUTPUT_DIR:-$repo_root/target/appfw/load-tests}"
summary_output="${APPFW_LOAD_TEST_SUITE_SUMMARY:-}"
only_scenario="${APPFW_LOAD_TEST_SUITE_ONLY:-}"
headers=("")

while [[ $# -gt 0 ]]; do
  case "$1" in
    --json)
      json=1
      ;;
    --url)
      shift || true
      url="${1:-}"
      ;;
    --requests)
      shift || true
      requests="${1:-}"
      ;;
    --concurrency)
      shift || true
      concurrency="${1:-}"
      ;;
    --header)
      shift || true
      headers+=("${1:-}")
      ;;
    --only)
      shift || true
      only_scenario="${1:-}"
      ;;
    --max-p95-ms)
      shift || true
      max_p95_ms="${1:-}"
      ;;
    --max-max-ms)
      shift || true
      max_max_ms="${1:-}"
      ;;
    --max-error-rate)
      shift || true
      max_error_rate="${1:-}"
      ;;
    --output-dir)
      shift || true
      output_dir="${1:-}"
      ;;
    --summary-output)
      shift || true
      summary_output="${1:-}"
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      usage_error "unknown load-test-suite option: $1"
      ;;
  esac
  shift || true
done

case "$requests" in
  ''|*[!0-9]*)
    usage_error "--requests must be a positive integer"
    ;;
esac
case "$concurrency" in
  ''|*[!0-9]*)
    usage_error "--concurrency must be a positive integer"
    ;;
esac
if [[ "$requests" -le 0 || "$concurrency" -le 0 ]]; then
  usage_error "--requests and --concurrency must be greater than 0"
fi
for numeric_value in "$max_p95_ms" "$max_max_ms" "$max_error_rate"; do
  if [[ -n "$numeric_value" && ! "$numeric_value" =~ ^[0-9]+([.][0-9]+)?$ ]]; then
    usage_error "load-test-suite thresholds must be non-negative numbers"
  fi
done

mkdir -p "$output_dir"

scenario_names=(
  crm-dashboard-account-summary
  crm-accounts-grid
  crm-accounts-search
  crm-account-form-projection
  crm-lookup-selector
  crm-account-activities-relationship
)

scenario_bodies=(
  '{"query":"query LoadDashboardAccounts($filter: JSON, $groupBy: JSON, $metrics: JSON, $having: JSON, $sort: JSON, $skip: Int!, $limit: Int!){ aggregateAccounts(filter:$filter, groupBy:$groupBy, metrics:$metrics, having:$having, sort:$sort, skip:$skip, limit:$limit){ query_count items } }","variables":{"filter":null,"groupBy":null,"metrics":[{"fn":"count","alias":"account_count"},{"fn":"sum","field":"annualRevenue","alias":"total_revenue"},{"fn":"avg","field":"numberOfEmployees","alias":"avg_employees"}],"having":null,"sort":null,"skip":0,"limit":1}}'
  '{"query":"query LoadAccountsGrid($filter: JSON, $sort: JSON, $skip: Int, $limit: Int){ queryAccounts(filter:$filter, sort:$sort, skip:$skip, limit:$limit){ query_count page_count page_index items { id name website phone email annual_revenue number_of_employees } } }","variables":{"filter":null,"sort":{"name":"ASC"},"skip":0,"limit":25}}'
  '{"query":"query LoadAccountsSearch($filter: JSON, $sort: JSON, $skip: Int, $limit: Int){ queryAccounts(filter:$filter, sort:$sort, skip:$skip, limit:$limit){ query_count items { id name billing_city billing_state industry { id name } } } }","variables":{"filter":{"name":{"_contains":"Acme"}},"sort":{"name":"ASC"},"skip":0,"limit":25}}'
  '{"query":"query LoadAccountFormProjection($filter: JSON, $sort: JSON, $skip: Int, $limit: Int){ queryAccounts(filter:$filter, sort:$sort, skip:$skip, limit:$limit){ query_count items { id name website phone email tags billing_street billing_city billing_state billing_postal_code billing_country annual_revenue number_of_employees description version industry { id name } } } }","variables":{"filter":null,"sort":{"name":"ASC"},"skip":0,"limit":1}}'
  '{"query":"query LoadLookupSelector($filter: JSON, $sort: JSON, $skip: Int, $limit: Int){ queryIndustries(filter:$filter, sort:$sort, skip:$skip, limit:$limit){ query_count items { id name description sort_order } } }","variables":{"filter":null,"sort":{"sortOrder":"ASC"},"skip":0,"limit":25}}'
  '{"query":"query LoadAccountActivitiesRelationship($filter: JSON, $sort: JSON, $skip: Int, $limit: Int){ queryActivities(filter:$filter, sort:$sort, skip:$skip, limit:$limit){ query_count items { id subject activity_date due_date is_closed priority status account { id name } } } }","variables":{"filter":{"account":{"name":{"_contains":"Acme"}}},"sort":{"activityDate":"DESC"},"skip":0,"limit":25}}'
)

scenario_files=("")
scenario_logs=("")
failed=0
executed=0
started_epoch="$(date +%s)"

for i in "${!scenario_names[@]}"; do
  scenario="${scenario_names[$i]}"
  body="${scenario_bodies[$i]}"
  if [[ -n "$only_scenario" && "$only_scenario" != "$scenario" ]]; then
    continue
  fi

  executed=$((executed + 1))
  scenario_file="$output_dir/$scenario.json"
  scenario_log="$output_dir/$scenario.log"
  scenario_files+=("$scenario_file")
  scenario_logs+=("$scenario_log")

  cmd=(
    bash "$script_dir/load-test-generated-api.sh"
    --json
    --url "$url"
    --requests "$requests"
    --concurrency "$concurrency"
    --scenario "$scenario"
    --output "$scenario_file"
  )
  if [[ -n "$max_p95_ms" ]]; then
    cmd+=(--max-p95-ms "$max_p95_ms")
  fi
  if [[ -n "$max_max_ms" ]]; then
    cmd+=(--max-max-ms "$max_max_ms")
  fi
  if [[ -n "$max_error_rate" ]]; then
    cmd+=(--max-error-rate "$max_error_rate")
  fi
  for header in "${headers[@]}"; do
    if [[ -n "$header" ]]; then
      cmd+=(--header "$header")
    fi
  done

  set +e
  APPFW_LOAD_TEST_BODY="$body" "${cmd[@]}" >"$scenario_log" 2>&1
  status=$?
  set -e
  if [[ $status -ne 0 ]]; then
    failed=1
  fi
done

if [[ "$executed" -eq 0 ]]; then
  usage_error "no load-test-suite scenarios matched"
fi

ended_epoch="$(date +%s)"
summary_file="$output_dir/summary.json"
set +e
python3 - "$summary_file" "$url" "$requests" "$concurrency" "$started_epoch" "$ended_epoch" "$max_error_rate" "$max_p95_ms" "$max_max_ms" "${scenario_files[@]}" <<'PY'
import datetime as dt
import hashlib
import json
import sys
from pathlib import Path

summary_file = Path(sys.argv[1])
url = sys.argv[2]
requests = int(sys.argv[3])
concurrency = int(sys.argv[4])
started_epoch = int(sys.argv[5])
ended_epoch = int(sys.argv[6])
max_error_rate = sys.argv[7]
max_p95_ms = sys.argv[8]
max_max_ms = sys.argv[9]
scenario_files = [Path(path) for path in sys.argv[10:] if path]

def utc_from_epoch(value):
    return dt.datetime.fromtimestamp(value, dt.timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")

def sha256(path):
    return "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest()

scenarios = []
total_requests = 0
total_success = 0
total_failed = 0
max_p95 = 0.0
max_latency = 0.0
violations = []
for path in scenario_files:
    try:
        data = json.loads(path.read_text())
    except Exception as exc:
        data = {
            "command": "load-test",
            "ok": False,
            "scenario": path.stem,
            "requests": 0,
            "success": 0,
            "failed": 0,
            "error_rate": 1.0,
            "latency_ms": {"min": 0, "avg": 0, "p95": 0, "max": 0},
            "thresholds": {},
            "violations": [f"scenario artifact did not parse: {exc}"],
        }
    scenario_violations = data.get("violations") if isinstance(data.get("violations"), list) else []
    if not data.get("ok", False):
        violations.append(f"{data.get('scenario', path.stem)} failed")
    for item in scenario_violations:
        violations.append(f"{data.get('scenario', path.stem)}: {item}")
    requests_count = int(data.get("requests") or 0)
    success_count = int(data.get("success") or 0)
    failed_count = int(data.get("failed") or 0)
    latency = data.get("latency_ms") if isinstance(data.get("latency_ms"), dict) else {}
    p95 = float(latency.get("p95") or 0)
    maximum = float(latency.get("max") or 0)
    total_requests += requests_count
    total_success += success_count
    total_failed += failed_count
    max_p95 = max(max_p95, p95)
    max_latency = max(max_latency, maximum)
    scenarios.append(
        {
            "name": data.get("scenario", path.stem),
            "ok": bool(data.get("ok")),
            "path": path.as_posix(),
            "sha256": sha256(path),
            "bytes": path.stat().st_size,
            "requests": requests_count,
            "success": success_count,
            "failed": failed_count,
            "error_rate": data.get("error_rate"),
            "latency_ms": latency,
            "violations": scenario_violations,
        }
    )

summary = {
    "command": "load-test-suite",
    "ok": total_failed == 0 and not violations and all(item["ok"] for item in scenarios),
    "generated_at_utc": utc_from_epoch(ended_epoch),
    "url": url,
    "scenario_count": len(scenarios),
    "passed": sum(1 for item in scenarios if item["ok"]),
    "failed": sum(1 for item in scenarios if not item["ok"]),
    "requests_per_scenario": requests,
    "concurrency": concurrency,
    "duration_seconds": max(0, ended_epoch - started_epoch),
    "thresholds": {
        "max_error_rate": float(max_error_rate) if max_error_rate else None,
        "max_p95_ms": float(max_p95_ms) if max_p95_ms else None,
        "max_max_ms": float(max_max_ms) if max_max_ms else None,
    },
    "totals": {
        "requests": total_requests,
        "success": total_success,
        "failed": total_failed,
        "error_rate": (total_failed / total_requests) if total_requests else 1.0,
        "max_p95_ms": max_p95,
        "max_latency_ms": max_latency,
    },
    "violations": violations,
    "scenarios": scenarios,
}
summary_file.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
sys.exit(0 if summary["ok"] else 1)
PY
aggregate_status=$?
set -e

if [[ -n "$summary_output" ]]; then
  mkdir -p "$(dirname "$summary_output")"
  cp "$summary_file" "$summary_output"
fi

if [[ "$json" == "1" ]]; then
  cat "$summary_file"
else
  echo "Load-test suite: $url"
  echo "  scenarios:    $executed"
  echo "  requests:     $requests per scenario"
  echo "  concurrency:  $concurrency"
  echo "  evidence:     $summary_file"
  if [[ "$aggregate_status" -eq 0 ]]; then
    echo "  status:       OK"
  else
    echo "  status:       FAILED"
  fi
fi

exit "$aggregate_status"
