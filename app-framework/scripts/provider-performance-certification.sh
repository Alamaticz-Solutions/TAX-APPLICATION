#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"
app_root="${APPFW_APP_ROOT:-$repo_root/examples/products/crm}"
if [[ ! -d "$app_root" ]]; then
  app_root="$repo_root"
fi

usage() {
  cat <<'EOF'
provider-performance-certification - validate provider performance evidence

Usage:
  scripts/provider-performance-certification.sh [options]

Options:
  --json                            Emit JSON evidence
  --provider NAME                   Provider to include; can be repeated
  --all                             Include postgres, mongo, mssql, and snowflake
  --provider-parity PATH            Provider parity artifact to consume
  --performance-recommendations PATH Performance recommendation artifact to consume
  --output PATH                     Write JSON evidence to PATH
  --require-live                    Require retained live provider parity areas
  --help                            Show this help

Environment:
  APPFW_PROVIDER_PERFORMANCE_OUTPUT
  APPFW_PROVIDER_PARITY_REPORT
  APPFW_PERFORMANCE_RECOMMENDATIONS_REPORT
  APPFW_PROVIDER_PERF_REQUIRE_LIVE
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
  printf '{"command":"provider-performance","ok":%s,"detail":' "$ok"
  json_escape "$detail"
  printf '}\n'
}

json=0
providers=("")
all=0
provider_parity="${APPFW_PROVIDER_PARITY_REPORT:-$repo_root/target/appfw/provider-parity.json}"
performance_recommendations="${APPFW_PERFORMANCE_RECOMMENDATIONS_REPORT:-$app_root/.appfw/target/appfw/performance_recommendations.json}"
output="${APPFW_PROVIDER_PERFORMANCE_OUTPUT:-$repo_root/target/appfw/provider-performance.json}"
require_live="${APPFW_PROVIDER_PERF_REQUIRE_LIVE:-false}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --json)
      json=1
      ;;
    --provider)
      shift || true
      providers+=("${1:-}")
      ;;
    --all)
      all=1
      ;;
    --provider-parity)
      shift || true
      provider_parity="${1:-}"
      ;;
    --performance-recommendations)
      shift || true
      performance_recommendations="${1:-}"
      ;;
    --output)
      shift || true
      output="${1:-}"
      ;;
    --require-live)
      require_live=true
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      if [[ "$json" == "1" ]]; then
        json_result false "unknown provider-performance option: $1"
      else
        echo "unknown provider-performance option: $1" >&2
      fi
      exit 2
      ;;
  esac
  shift || true
done

if [[ "$all" == "1" ]]; then
  providers=(postgres mongo mssql snowflake)
else
  filtered=("")
  for provider in "${providers[@]}"; do
    if [[ -n "$provider" ]]; then
      filtered+=("$provider")
    fi
  done
  providers=("${filtered[@]}")
  if [[ ${#providers[@]} -eq 1 && -z "${providers[0]}" ]]; then
    providers=(postgres mongo mssql snowflake)
  fi
fi

mkdir -p "$(dirname "$output")"

set +e
python3 - "$repo_root" "$output" "$provider_parity" "$performance_recommendations" "$require_live" "${providers[@]}" <<'PY'
import datetime as dt
import hashlib
import json
import os
import sys
from pathlib import Path

repo_root = Path(sys.argv[1])
output = Path(sys.argv[2])
provider_parity_path = Path(sys.argv[3])
performance_recommendations_path = Path(sys.argv[4])
require_live = sys.argv[5].lower() in {"1", "true", "yes", "y", "on"}
providers = [value for value in sys.argv[6:] if value]
if not providers:
    providers = ["postgres", "mongo", "mssql", "snowflake"]

checks = []
blocking_failures = []

def utc_now():
    return (
        dt.datetime.now(dt.timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z")
    )

def record(name, ok, detail=None, artifact=None, blocking=True):
    item = {"name": name, "ok": bool(ok), "blocking": bool(blocking)}
    if detail is not None:
        item["detail"] = detail
    if artifact is not None:
        item["artifact"] = str(artifact)
    checks.append(item)
    if blocking and not ok:
        blocking_failures.append(name if detail is None else f"{name}: {detail}")

def load_json(path, name, blocking=True):
    if not path.is_file():
        record(f"{name} artifact is present", False, f"missing {path}", path, blocking=blocking)
        return None
    try:
        data = json.loads(path.read_text())
    except Exception as exc:
        record(f"{name} artifact parses", False, str(exc), path, blocking=blocking)
        return None
    record(f"{name} artifact parses", True, artifact=path, blocking=blocking)
    return data

def file_artifact(name, path, required=True):
    present = path.is_file()
    artifact = {
        "name": name,
        "path": path.as_posix(),
        "required": bool(required),
        "present": present,
        "ok": present,
    }
    if present:
        data = path.read_bytes()
        artifact["sha256"] = "sha256:" + hashlib.sha256(data).hexdigest()
        artifact["bytes"] = len(data)
    return artifact

expected_budget_env = [
    "APP_QUERY_COST_MAX",
    "APP_AGGREGATE_QUERY_COST_MAX",
    "APP_QUERY_RELATIONSHIP_DEPTH_MAX",
    "APP_QUERY_MANY_TO_MANY_EXPANSION_MAX",
    "APP_QUERY_FILTER_PREDICATES_MAX",
    "APP_QUERY_SORT_SPECS_MAX",
    "APP_AGGREGATE_OUTPUTS_MAX",
    "APP_QUERY_SELECTED_FIELDS_MAX",
    "APP_QUERY_OFFSET_ROWS_MAX",
]
query_cost_source = repo_root / "appfw_runtime/src/query_cost.rs"
source_text = query_cost_source.read_text() if query_cost_source.is_file() else ""
missing_budget_env = [name for name in expected_budget_env if name not in source_text]
record(
    "query budget env caps are implemented",
    not missing_budget_env,
    None if not missing_budget_env else f"missing {', '.join(missing_budget_env)}",
    query_cost_source,
)

recommendations = load_json(performance_recommendations_path, "performance recommendations", blocking=True)
recommendation_count = 0
generated_recommendations = 0
if isinstance(recommendations, dict):
    schemas = recommendations.get("schemas")
    if isinstance(schemas, list):
        for schema in schemas:
            recommendation_count += int(schema.get("recommendation_count") or 0)
            for recommendation in schema.get("recommendations") or []:
                if isinstance(recommendation, dict) and recommendation.get("status") == "generated":
                    generated_recommendations += 1
    record(
        "performance recommendations include generated access paths",
        recommendation_count > 0 and generated_recommendations > 0,
        f"recommendations={recommendation_count}, generated={generated_recommendations}",
        performance_recommendations_path,
    )

provider_parity = load_json(provider_parity_path, "provider parity", blocking=require_live)
provider_entries = {}
if isinstance(provider_parity, dict):
    record(
        "provider parity ok flag is true",
        provider_parity.get("ok") is True,
        artifact=provider_parity_path,
        blocking=require_live,
    )
    for item in provider_parity.get("providers") or []:
        if isinstance(item, dict) and item.get("provider"):
            provider_entries[item["provider"]] = item

contract_groups = [
    {
        "name": "filter-and-sort",
        "areas": ["scalar_filters", "sorting"],
        "reason": "filters and sorts must stay provider-normalized and indexable",
    },
    {
        "name": "pagination",
        "areas": ["pagination"],
        "reason": "page windows must be bounded and provider-certified",
    },
    {
        "name": "native-projection",
        "areas": ["native_projection"],
        "reason": "selected-field projections must avoid over-fetching",
    },
    {
        "name": "relationship-projection",
        "areas": ["relationship_projection"],
        "reason": "relationship projections must avoid N+1-style behavior",
    },
    {
        "name": "many-to-many-fanout",
        "areas": ["many_to_many_projection"],
        "reason": "junction fanout must be bounded by provider contracts",
    },
    {
        "name": "aggregate-and-count",
        "areas": ["aggregation", "aggregate_filters"],
        "reason": "aggregate/count paths must be provider-certified",
    },
    {
        "name": "access-filter",
        "areas": ["access_filters"],
        "reason": "policy filters must not widen or de-optimize provider plans",
    },
]

provider_reports = []
for provider in providers:
    entry = provider_entries.get(provider)
    provider_checks = []
    if entry is None:
        record(
            f"{provider} provider parity entry is present",
            False,
            "provider-performance requires provider-test evidence for live certification",
            provider_parity_path,
            blocking=require_live,
        )
        provider_reports.append(
            {
                "provider": provider,
                "ok": not require_live,
                "status": "static-only",
                "checks": [],
                "detail": "provider parity entry missing",
            }
        )
        continue

    areas = {
        area.get("area"): area
        for area in entry.get("areas") or []
        if isinstance(area, dict) and area.get("area")
    }
    provider_ok = bool(entry.get("ok"))
    for group in contract_groups:
        area_results = []
        group_ok = True
        for area_name in group["areas"]:
            area = areas.get(area_name)
            area_ok = bool(area and area.get("ok") is True)
            live_ok = bool(
                area
                and area.get("ok") is True
                and area.get("status") in {"live-certified", "emulator-limited"}
                and area.get("live_result") == "passed"
                and area.get("required_live_result") in {"passed", "not-failed"}
            )
            ok = live_ok if require_live else area_ok
            group_ok = group_ok and ok
            area_results.append(
                {
                    "area": area_name,
                    "ok": ok,
                    "status": area.get("status") if area else "missing",
                    "live_result": area.get("live_result") if area else None,
                    "required_live_result": area.get("required_live_result") if area else None,
                    "live_contracts": area.get("live_contracts", []) if area else [],
                }
            )
        provider_checks.append(
            {
                "name": group["name"],
                "ok": group_ok,
                "reason": group["reason"],
                "areas": area_results,
            }
        )
        provider_ok = provider_ok and group_ok
        record(
            f"{provider} provider performance {group['name']}",
            group_ok,
            group["reason"],
            provider_parity_path,
            blocking=require_live,
        )
    provider_reports.append(
        {
            "provider": provider,
            "ok": provider_ok,
            "status": "live-certified" if provider_ok and require_live else "certified-from-retained-parity" if provider_ok else "incomplete",
            "base_url": entry.get("base_url"),
            "data_source": entry.get("data_source"),
            "provider_parity_ok": bool(entry.get("ok")),
            "checks": provider_checks,
        }
    )

ok = not blocking_failures
report = {
    "command": "provider-performance",
    "ok": ok,
    "generated_at_utc": utc_now(),
    "require_live": require_live,
    "artifacts": {
        "provider_parity": file_artifact("provider_parity", provider_parity_path),
        "performance_recommendations": file_artifact(
            "performance_recommendations",
            performance_recommendations_path,
        ),
        "query_cost_source": file_artifact("query_cost_source", query_cost_source),
    },
    "summary": {
        "provider_count": len(provider_reports),
        "provider_ok_count": sum(1 for item in provider_reports if item["ok"]),
        "recommendation_count": recommendation_count,
        "generated_recommendation_count": generated_recommendations,
        "blocking_failure_count": len(blocking_failures),
    },
    "checks": checks,
    "providers": provider_reports,
    "blocking_failures": blocking_failures,
}
output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
sys.exit(0 if ok else 1)
PY
status=$?
set -e

if [[ "$json" == "1" ]]; then
  cat "$output"
else
  if [[ "$status" -eq 0 ]]; then
    printf 'Provider performance certification: OK\n'
  else
    printf 'Provider performance certification: FAILED\n' >&2
  fi
  printf 'Evidence: %s\n' "$output"
fi

exit "$status"
