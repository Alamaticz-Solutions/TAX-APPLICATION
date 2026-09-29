#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
cd "$repo_root"

product_app_root="${APPFW_APP_ROOT:-$repo_root/examples/products/crm}"
framework_root="${APPFW_FRAMEWORK_ROOT:-$repo_root}"
report_dir="${APPFW_RELEASE_ARTIFACT_DIR:-$repo_root/target/appfw}"
preflight_dir="${APPFW_LOCAL_LIVE_PREFLIGHT_DIR:-$report_dir/local-live-preflight}"
checks_file="$preflight_dir/checks.tsv"
evidence_file="${APPFW_LOCAL_LIVE_PREFLIGHT_OUTPUT:-$report_dir/local-live-release-preflight.json}"
backend_target_dir="${APPFW_LOCAL_PREFLIGHT_BACKEND_TARGET_DIR:-$repo_root/target/local-live-preflight-backend}"
backend_binary="$backend_target_dir/debug/backend"
backend_certification_features="${APPFW_CERTIFICATION_BACKEND_FEATURES:-http,provider-postgres,provider-mongo,provider-mssql,provider-snowflake}"
compose_file="$product_app_root/podman-compose.yml"
json=0
plan=0

mkdir -p "$report_dir" "$preflight_dir" api_tests/target
: >"$checks_file"

log() {
  printf '[%s] %s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" "$*" >&2
}

env_flag_enabled() {
  case "$(printf '%s' "${1:-}" | tr '[:upper:]' '[:lower:]')" in
    1|true|yes|y|on)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

load_preflight_env_file() {
  local env_file="$1"

  [[ -f "$env_file" ]] || return 0

  while IFS= read -r line || [[ -n "$line" ]]; do
    line="${line%$'\r'}"
    line="${line#"${line%%[![:space:]]*}"}"
    line="${line%"${line##*[![:space:]]}"}"

    [[ -z "$line" || "$line" == \#* ]] && continue
    [[ "$line" == export\ * ]] && line="${line#export }"
    [[ "$line" == *=* ]] || continue

    local key="${line%%=*}"
    local value="${line#*=}"
    key="${key%"${key##*[![:space:]]}"}"
    value="${value#"${value%%[![:space:]]*}"}"
    value="${value%"${value##*[![:space:]]}"}"

    [[ "$key" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || continue
    [[ -z "${!key:-}" ]] || continue

    if [[ "$value" == \"*\" && "$value" == *\" && ${#value} -ge 2 ]]; then
      value="${value:1:${#value}-2}"
    elif [[ "$value" == \'*\' && ${#value} -ge 2 ]]; then
      value="${value:1:${#value}-2}"
    fi

    export "$key=$value"
  done <"$env_file"
}

load_preflight_env() {
  local explicit_env_file="${APPFW_LOCAL_PREFLIGHT_ENV_FILE:-}"

  if [[ -n "$explicit_env_file" ]]; then
    load_preflight_env_file "$explicit_env_file"
    return 0
  fi

  load_preflight_env_file "$repo_root/.env"
  load_preflight_env_file "$product_app_root/.env"
}

rel_path() {
  local path="$1"
  python3 - "$repo_root" "$path" <<'PY'
import os
import sys

root = os.path.abspath(sys.argv[1])
path = sys.argv[2]
if not path:
    print("")
    raise SystemExit(0)
abs_path = os.path.abspath(path)
try:
    print(os.path.relpath(abs_path, root))
except ValueError:
    print(path)
PY
}

load_preflight_env

record_check() {
  local name="$1"
  local ok="$2"
  local status="$3"
  local log_file="${4:-}"
  local artifact="${5:-}"
  local detail="${6:-}"

  detail="${detail//$'\t'/ }"
  detail="${detail//$'\n'/ }"
  printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
    "$name" "$ok" "$status" "$log_file" "$artifact" "$detail" >>"$checks_file"
}

write_plan() {
  python3 - "$repo_root" "$product_app_root" "$report_dir" "$preflight_dir" "$evidence_file" <<'PY'
import json
import os
import sys
from pathlib import Path

repo_root = Path(sys.argv[1])
product_app_root = Path(sys.argv[2])
report_dir = Path(sys.argv[3])
preflight_dir = Path(sys.argv[4])
evidence_file = Path(sys.argv[5])
plan_file = report_dir / "local-live-release-preflight-plan.json"

def rel(path):
    path = Path(path).resolve()
    try:
        return path.relative_to(repo_root).as_posix()
    except ValueError:
        return path.as_posix()

provider_backends = [
    {
        "provider": provider,
        "env": env,
        "required": True,
        "requirement": (
            "distinct HTTP(S) backend base URL with no credentials, query, or "
            "fragment; /health/ready must be reachable"
        ),
        "default": default,
    }
    for provider, env, default in [
        ("postgres", "API_TEST_BASE_URL_POSTGRES", "http://127.0.0.1:8081"),
        ("mongo", "API_TEST_BASE_URL_MONGO", "http://127.0.0.1:8082"),
        ("mssql", "API_TEST_BASE_URL_MSSQL", "http://127.0.0.1:8083"),
        ("snowflake", "API_TEST_BASE_URL_SNOWFLAKE", "http://127.0.0.1:8084"),
    ]
]
provider_services = [
    {"provider": "postgres", "host": "127.0.0.1", "port": 5432, "required": True},
    {"provider": "mongo", "host": "127.0.0.1", "port": 27017, "required": True},
    {"provider": "mssql", "host": "127.0.0.1", "port": 1433, "required": True},
    {
        "provider": "snowflake",
        "host": "snowflake.localhost.localstack.cloud",
        "port": 4566,
        "required": True,
    },
]
external_release_authority_gates = [
    {
        "id": "provider-backed-release-check",
        "authority": "managed-release-ci",
        "required_for_release": True,
        "local_preflight_satisfies": False,
        "evidence": "target/appfw/release-check.json from managed release CI",
    },
    {
        "id": "live-security-certification",
        "authority": "managed-release-ci",
        "required_for_release": True,
        "local_preflight_satisfies": False,
        "evidence": "target/appfw/security-certification.json retained with managed provider logs",
    },
    {
        "id": "strict-release-evidence",
        "authority": "managed-release-ci",
        "required_for_release": True,
        "local_preflight_satisfies": False,
        "evidence": "target/appfw/release-evidence-check.json from strict release evidence validation",
    },
    {
        "id": "release-authority",
        "authority": "release-authority",
        "required_for_release": True,
        "local_preflight_satisfies": False,
        "evidence": "release identity, PDS baseline, provenance/signing, and governance artifacts",
    },
]
release_authority_boundary = {
    "local_preflight_satisfies_release": False,
    "local_preflight_meaning": (
        "branch-level confidence that local provider services, backend instances, "
        "provider-test, and security-certification can run before PR review"
    ),
    "managed_release_gate_required": True,
    "managed_release_gate_meaning": (
        "release-authoritative provider-backed release-check, strict evidence "
        "check, security certification, release identity, PDS baseline, "
        "provenance/signing, and release authority bundle"
    ),
}
report = {
    "command": "local-live-release-preflight",
    "mode": "plan",
    "ok": True,
    "ci_ready": False,
    "release_ready": False,
    "release_authority": "local-preflight-only",
    "product_app_root": rel(product_app_root),
    "report_dir": rel(report_dir),
    "preflight_dir": rel(preflight_dir),
    "artifact": rel(plan_file),
    "live_artifact": rel(evidence_file),
    "required_live_inputs": {
        "provider_backends": provider_backends,
        "provider_services": provider_services,
        "auth_context": {
            "API_TEST_AUTH_MODE": "local_dev",
            "APP_ENABLE_LOCAL_TEST_AUTH": "true",
            "release_note": (
                "Plan mode does not claim managed identity posture. Release "
                "authority must use release-check/security-certification posture."
            ),
        },
    },
    "external_release_authority_gates": external_release_authority_gates,
    "release_authority_boundary": release_authority_boundary,
    "planned_stages": [
        "provider-services",
        "wait-provider-tcp",
        "migrate-providers",
        "build-backend",
        "start-provider-backends",
        "provider-url-preflight",
        "provider-test",
        "security-certification",
    ],
}
plan_file.parent.mkdir(parents=True, exist_ok=True)
plan_file.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
print(json.dumps(report, indent=2, sort_keys=True))
PY
}

write_evidence() {
  local exit_status="$1"
  local generated_at_utc
  generated_at_utc="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"

  python3 - \
    "$repo_root" \
    "$checks_file" \
    "$evidence_file" \
    "$report_dir" \
    "$preflight_dir" \
    "$generated_at_utc" \
    "$exit_status" <<'PY'
import json
import os
import sys
from pathlib import Path

repo_root = Path(sys.argv[1]).resolve()
checks_path = Path(sys.argv[2])
evidence_path = Path(sys.argv[3])
report_dir = Path(sys.argv[4]).resolve()
preflight_dir = Path(sys.argv[5]).resolve()
generated_at_utc = sys.argv[6]
exit_status = int(sys.argv[7])


def rel(value):
    if not value:
        return ""
    path = Path(value).resolve()
    try:
        return path.relative_to(repo_root).as_posix()
    except ValueError:
        return value


checks = []
if checks_path.is_file():
    for line in checks_path.read_text(encoding="utf-8").splitlines():
        parts = line.split("\t", 5)
        if len(parts) != 6:
            continue
        name, ok, status, log_file, artifact, detail = parts
        item = {
            "name": name,
            "ok": ok == "true",
            "exit_code": int(status),
        }
        if log_file:
            item["log"] = rel(log_file)
        if artifact:
            item["artifact"] = rel(artifact)
        if detail:
            item["detail"] = detail
        checks.append(item)

ok = exit_status == 0 and all(check.get("ok") is True for check in checks)
provider_urls = {
    "postgres": os.environ.get("API_TEST_BASE_URL_POSTGRES", ""),
    "mongo": os.environ.get("API_TEST_BASE_URL_MONGO", ""),
    "mssql": os.environ.get("API_TEST_BASE_URL_MSSQL", ""),
    "snowflake": os.environ.get("API_TEST_BASE_URL_SNOWFLAKE", ""),
}
provider_url_contract = [
    {
        "provider": provider,
        "env": env,
        "required": True,
        "requirement": (
            "distinct HTTP(S) backend base URL with no credentials, query, or "
            "fragment; /health/ready must be reachable"
        ),
        "value_present": bool(os.environ.get(env, "").strip()),
    }
    for provider, env in [
        ("postgres", "API_TEST_BASE_URL_POSTGRES"),
        ("mongo", "API_TEST_BASE_URL_MONGO"),
        ("mssql", "API_TEST_BASE_URL_MSSQL"),
        ("snowflake", "API_TEST_BASE_URL_SNOWFLAKE"),
    ]
]
required_live_inputs = {
    "provider_backends": provider_url_contract,
    "provider_services": [
        {"provider": "postgres", "host": "127.0.0.1", "port": 5432, "required": True},
        {"provider": "mongo", "host": "127.0.0.1", "port": 27017, "required": True},
        {"provider": "mssql", "host": "127.0.0.1", "port": 1433, "required": True},
        {
            "provider": "snowflake",
            "host": "snowflake.localhost.localstack.cloud",
            "port": 4566,
            "required": True,
        },
    ],
    "auth_context": {
        "API_TEST_AUTH_MODE": os.environ.get("API_TEST_AUTH_MODE", ""),
        "APP_ENABLE_LOCAL_TEST_AUTH": os.environ.get("APP_ENABLE_LOCAL_TEST_AUTH", ""),
        "release_note": (
            "Local preflight may use local_dev auth; release authority must use "
            "the managed identity posture required by release-check/security-certification."
        ),
    },
}
external_release_authority_gates = [
    {
        "id": "provider-backed-release-check",
        "authority": "managed-release-ci",
        "required_for_release": True,
        "local_preflight_satisfies": False,
        "evidence": "target/appfw/release-check.json from managed release CI",
    },
    {
        "id": "live-security-certification",
        "authority": "managed-release-ci",
        "required_for_release": True,
        "local_preflight_satisfies": False,
        "evidence": "target/appfw/security-certification.json retained with managed provider logs",
    },
    {
        "id": "strict-release-evidence",
        "authority": "managed-release-ci",
        "required_for_release": True,
        "local_preflight_satisfies": False,
        "evidence": "target/appfw/release-evidence-check.json from strict release evidence validation",
    },
    {
        "id": "release-authority",
        "authority": "release-authority",
        "required_for_release": True,
        "local_preflight_satisfies": False,
        "evidence": "release identity, PDS baseline, provenance/signing, and governance artifacts",
    },
]
release_authority_boundary = {
    "local_preflight_satisfies_release": False,
    "local_preflight_meaning": (
        "branch-level confidence that local provider services, backend instances, "
        "provider-test, and security-certification can run before PR review"
    ),
    "managed_release_gate_required": True,
    "managed_release_gate_meaning": (
        "release-authoritative provider-backed release-check, strict evidence "
        "check, security certification, release identity, PDS baseline, "
        "provenance/signing, and release authority bundle"
    ),
}
artifact_candidates = [
    report_dir / "provider-parity.json",
    report_dir / "security-certification.json",
    preflight_dir / "provider-test-output.json",
    preflight_dir / "security-certification-output.json",
    preflight_dir / "provider-url-preflight.json",
]
artifacts = [
    {"name": path.stem, "path": rel(path), "present": path.is_file()}
    for path in artifact_candidates
]
release_blockers = [
    check["name"] for check in checks if check.get("ok") is not True
]
if exit_status != 0 and not release_blockers:
    release_blockers.append(f"local live preflight exited with status {exit_status}")

evidence = {
    "command": "local-live-release-preflight",
    "generated_at_utc": generated_at_utc,
    "ok": ok,
    "ci_ready": ok,
    "release_ready": False,
    "release_authority": "local-preflight-only",
    "release_authority_note": (
        "This local preflight proves four-provider live certification and "
        "security-certification readiness before CI. Production release authority "
        "still comes from the remote release gate and its live/governance evidence."
    ),
    "required_live_inputs": required_live_inputs,
    "external_release_authority_gates": external_release_authority_gates,
    "release_authority_boundary": release_authority_boundary,
    "checks": checks,
    "provider_base_urls": provider_urls,
    "artifacts": artifacts,
    "release_blockers": release_blockers,
    "failure_summary": {"root_causes": release_blockers},
}
evidence_path.parent.mkdir(parents=True, exist_ok=True)
evidence_path.write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
}

finish() {
  local exit_status="$1"
  set +e
  write_evidence "$exit_status"
  local evidence_status=$?
  if [[ "$json" == "1" && -f "$evidence_file" ]]; then
    cat "$evidence_file"
  elif [[ -f "$evidence_file" ]]; then
    if [[ "$exit_status" == "0" ]]; then
      printf 'local-live-release-preflight: OK; evidence written to %s\n' "$(rel_path "$evidence_file")"
    else
      printf 'local-live-release-preflight: FAILED; evidence written to %s\n' "$(rel_path "$evidence_file")" >&2
    fi
  fi
  if [[ $evidence_status -ne 0 && $exit_status -eq 0 ]]; then
    exit_status=$evidence_status
  fi
  exit "$exit_status"
}

fail_stage() {
  local status="$1"
  finish "$status"
}

run_logged_stage() {
  local name="$1"
  shift
  local log_file="$preflight_dir/${name}.log"

  log "Running $name"
  set +e
  "$@" >"$log_file" 2>&1
  local status=$?
  set -e
  if [[ $status -eq 0 ]]; then
    record_check "$name" true "$status" "$log_file" "" ""
  else
    record_check "$name" false "$status" "$log_file" "" "see stage log"
  fi
  return "$status"
}

run_json_stage() {
  local name="$1"
  local artifact="$2"
  shift 2
  local log_file="$preflight_dir/${name}.log"

  log "Running $name"
  set +e
  "$@" >"$artifact" 2>"$log_file"
  local status=$?
  set -e
  if [[ $status -eq 0 ]]; then
    record_check "$name" true "$status" "$log_file" "$artifact" ""
  else
    record_check "$name" false "$status" "$log_file" "$artifact" "see stage log and artifact output"
  fi
  return "$status"
}

compose_up() {
  if [[ ! -f "$compose_file" ]]; then
    echo "missing compose file: $compose_file" >&2
    return 2
  fi

  if provider_services_reachable; then
    echo "provider service ports are already reachable; skipping compose up"
    return 0
  fi

  if [[ -z "${LOCALSTACK_AUTH_TOKEN:-}" ]]; then
    echo "LOCALSTACK_AUTH_TOKEN is required for the LocalStack Snowflake preflight; set it in the environment, APPFW_LOCAL_PREFLIGHT_ENV_FILE, repo .env, or product .env" >&2
    return 2
  fi

  local status=0
  if command -v docker >/dev/null 2>&1 && docker compose version >/dev/null 2>&1; then
    docker compose -f "$compose_file" --profile snowflake up -d postgres mongo mssql snowflake || status=$?
  elif command -v podman >/dev/null 2>&1 && podman compose version >/dev/null 2>&1; then
    podman compose -f "$compose_file" --profile snowflake up -d postgres mongo mssql snowflake || status=$?
  else
    echo "docker compose or podman compose is required for local live preflight" >&2
    return 2
  fi

  if [[ $status -ne 0 ]] && provider_services_reachable; then
    echo "compose up reported status $status, but provider service ports are reachable; continuing"
    return 0
  fi
  return "$status"
}

tcp_probe() {
  local host="$1"
  local port="$2"

  python3 - "$host" "$port" <<'PY' >/dev/null 2>&1
import socket
import sys

host = sys.argv[1]
port = int(sys.argv[2])
hosts = [host]
if host.endswith(".localhost.localstack.cloud"):
    hosts.append("127.0.0.1")

last_error = None
for candidate in hosts:
    try:
        with socket.create_connection((candidate, port), timeout=2):
            raise SystemExit(0)
    except OSError as exc:
        last_error = exc
raise last_error or OSError(f"unable to connect to {host}:{port}")
PY
}

provider_services_reachable() {
  tcp_probe 127.0.0.1 5432 \
    && tcp_probe 127.0.0.1 27017 \
    && tcp_probe 127.0.0.1 1433 \
    && tcp_probe snowflake.localhost.localstack.cloud 4566
}

wait_for_tcp() {
  local host="$1"
  local port="$2"
  local label="$3"
  local attempts="${4:-90}"

  for _ in $(seq 1 "$attempts"); do
    if tcp_probe "$host" "$port"; then
      echo "$label is accepting TCP connections"
      return 0
    fi
    sleep 2
  done

  echo "$label did not become reachable at $host:$port" >&2
  return 1
}

wait_for_http() {
  local url="$1"
  local label="$2"
  local attempts="${3:-90}"

  for _ in $(seq 1 "$attempts"); do
    if curl -kfsS --max-time 5 "$url" >/dev/null 2>&1; then
      echo "$label is healthy"
      return 0
    fi
    sleep 2
  done

  echo "$label did not become healthy at $url" >&2
  return 1
}

wait_for_snowflake_control_plane() {
  local attempts="${1:-90}"
  local snowflake_host="snowflake.localhost.localstack.cloud"
  local snowflake_url="http://${snowflake_host}:4566/session"
  local curl_args=(-kfsS --max-time 5 -X POST -d '{}')

  if ! python3 - "$snowflake_host" <<'PY' >/dev/null 2>&1
import socket
import sys

socket.getaddrinfo(sys.argv[1], None)
PY
  then
    curl_args+=(--resolve "${snowflake_host}:4566:127.0.0.1")
  fi

  for _ in $(seq 1 "$attempts"); do
    if curl "${curl_args[@]}" "$snowflake_url" >/dev/null 2>&1; then
      echo "LocalStack Snowflake SQL API is healthy"
      return 0
    fi
    sleep 2
  done

  echo "LocalStack Snowflake SQL API did not become healthy" >&2
  return 1
}

run_migration() {
  local provider="$1"
  local data_source="$2"
  local artifact="$preflight_dir/migrate-${provider}.json"
  run_json_stage "migrate-${provider}" "$artifact" \
    env APP_CRM_DATA_SOURCE_NAME="$data_source" scripts/appfw migrate --json
}

start_backend() {
  local provider="$1"
  local data_source="$2"
  local port="$3"
  local log_file="$preflight_dir/backend-${provider}.log"
  local pid_file="$preflight_dir/backend-${provider}.pid"

  if curl -kfsS --max-time 2 "http://127.0.0.1:${port}/health/ready" >/dev/null 2>&1; then
    echo "port ${port} already has a ready backend; stop it before running local preflight" >&2
    return 2
  fi

  log "Starting $provider backend on port $port"
  (
    cd "$product_app_root/backend"
    APP_CRM_DATA_SOURCE_NAME="$data_source" API_PORT="$port" "$backend_binary"
  ) >"$log_file" 2>&1 &
  local pid="$!"
  echo "$pid" >"$pid_file"

  wait_for_http "http://127.0.0.1:${port}/health/ready" "$provider backend"
}

cleanup() {
  for pid_file in "$preflight_dir"/backend-*.pid; do
    [[ -f "$pid_file" ]] || continue
    local pid
    pid="$(cat "$pid_file" 2>/dev/null || true)"
    if [[ -n "$pid" ]]; then
      kill "$pid" >/dev/null 2>&1 || true
      wait "$pid" >/dev/null 2>&1 || true
    fi
    rm -f "$pid_file"
  done
}
trap cleanup EXIT

write_provider_url_snapshot() {
  local artifact="$preflight_dir/provider-url-preflight.json"
  python3 - "$artifact" <<'PY'
import datetime as dt
import json
import os
import sys
import urllib.parse
import urllib.request

artifact = sys.argv[1]
providers = [
    ("postgres", "API_TEST_BASE_URL_POSTGRES"),
    ("mongo", "API_TEST_BASE_URL_MONGO"),
    ("mssql", "API_TEST_BASE_URL_MSSQL"),
    ("snowflake", "API_TEST_BASE_URL_SNOWFLAKE"),
]
items = []
identities = {}
ok = True
for provider, env_name in providers:
    base_url = os.environ.get(env_name, "").rstrip("/")
    parsed = urllib.parse.urlparse(base_url)
    item = {
        "provider": provider,
        "env": env_name,
        "base_url": base_url,
        "health_url": f"{base_url}/health/ready" if base_url else "",
        "ok": False,
    }
    if parsed.scheme not in {"http", "https"} or not parsed.hostname:
        item["reason"] = "base URL must be HTTP(S) and include a host"
        ok = False
    elif parsed.username or parsed.password or parsed.query or parsed.fragment:
        item["reason"] = "base URL must not include credentials, query, or fragment"
        ok = False
    else:
        port = parsed.port or (443 if parsed.scheme == "https" else 80)
        identity = (parsed.scheme, parsed.hostname, port)
        identities.setdefault(identity, []).append(provider)
        try:
            with urllib.request.urlopen(item["health_url"], timeout=5) as response:
                item["status"] = response.status
                item["ok"] = 200 <= response.status < 300
                if not item["ok"]:
                    item["reason"] = f"health endpoint returned HTTP {response.status}"
                    ok = False
        except Exception as exc:
            item["reason"] = f"health endpoint failed: {exc}"
            ok = False
    items.append(item)

for identity, providers_for_identity in identities.items():
    if len(providers_for_identity) > 1:
        ok = False
        for item in items:
            if item["provider"] in providers_for_identity:
                item["ok"] = False
                item["reason"] = "provider URL is not distinct"

generated_at_utc = (
    dt.datetime.now(dt.timezone.utc)
    .replace(microsecond=0)
    .isoformat()
    .replace("+00:00", "Z")
)
report = {
    "command": "local-live-provider-url-preflight",
    "generated_at_utc": generated_at_utc,
    "ok": ok,
    "provider_base_urls": items,
}
with open(artifact, "w", encoding="utf-8") as handle:
    json.dump(report, handle, indent=2, sort_keys=True)
    handle.write("\n")
if not ok:
    raise SystemExit(1)
PY
}

assert_provider_parity() {
  local report_file="$report_dir/provider-parity.json"
  python3 - "$report_file" <<'PY'
import json
import sys

path = sys.argv[1]
with open(path, "r", encoding="utf-8") as handle:
    report = json.load(handle)

failures = []
if report.get("ok") is not True:
    failures.append("top-level provider parity ok flag is not true")

expected = {"postgres", "mongo", "mssql", "snowflake"}
providers = report.get("providers", [])
seen = {item.get("provider") for item in providers if isinstance(item, dict)}
if seen != expected:
    failures.append(f"provider set mismatch: expected {sorted(expected)}, got {sorted(seen)}")

for provider in providers:
    name = provider.get("provider", "<unknown>")
    if provider.get("ok") is not True:
        failures.append(f"{name}: provider ok flag is not true")
    if provider.get("mode") != "full":
        failures.append(f"{name}: provider run mode is not full")
    live_total = 0
    for area in provider.get("areas", []):
        if area.get("status") != "live-certified":
            continue
        live_total += 1
        if area.get("live_result") != "passed":
            failures.append(f"{name}.{area.get('area', '<unknown>')}: live result is not passed")
    if live_total == 0:
        failures.append(f"{name}: no live-certified areas were reported")

if failures:
    print("Provider parity evidence is not release-grade:", file=sys.stderr)
    for failure in failures:
        print(f"  - {failure}", file=sys.stderr)
    raise SystemExit(1)
PY
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --json)
      json=1
      ;;
    --plan)
      plan=1
      ;;
    --help|-h)
      cat <<'EOF'
Usage: scripts/ci/local-live-release-preflight.sh [--json] [--plan]

Starts local provider services, runs migrations, launches one local backend per
CRUD provider, and collects provider parity plus security certification evidence.
Use --plan to print the required live-input and release-authority contract
without starting containers or provider backends.
EOF
      exit 0
      ;;
    *)
      echo "local-live-release-preflight: unknown argument: $1" >&2
      exit 2
      ;;
  esac
  shift
done

if [[ "$plan" == "1" ]]; then
  write_plan
  exit 0
fi

export APPFW_APP_ROOT="$product_app_root"
export APPFW_FRAMEWORK_ROOT="$framework_root"
export APPFW_RELEASE_ARTIFACT_DIR="$report_dir"
export ENV_NAME="${APPFW_LOCAL_PREFLIGHT_ENV_NAME:-local}"
export LOG_LEVEL="${LOG_LEVEL:-info}"
export API_TEST_AUTH_MODE="${API_TEST_AUTH_MODE:-local_dev}"
export APP_ENABLE_LOCAL_TEST_AUTH="${APP_ENABLE_LOCAL_TEST_AUTH:-true}"
export APP_PROVIDER_CERTIFICATION_CI="${APP_PROVIDER_CERTIFICATION_CI:-true}"
export APP_MCP_ENABLED="${APP_MCP_ENABLED:-false}"
export APPFW_MIGRATE_SKIP_SEED="${APPFW_MIGRATE_SKIP_SEED:-true}"
export RUST_MIN_STACK="${RUST_MIN_STACK:-16777216}"

export OKTA_AUDIENCE="${OKTA_AUDIENCE:-api://default}"
export OKTA_ISSUER="${OKTA_ISSUER:-https://example.okta.com/oauth2/default}"
export OKTA_CLIENT_ID="${OKTA_CLIENT_ID:-example-client-id}"

export PG_SERVICE_ACCOUNT_NAME="${PG_SERVICE_ACCOUNT_NAME:-postgres}"
export PG_SERVICE_ACCOUNT_PASS="${PG_SERVICE_ACCOUNT_PASS:-postgres}"
export PG_SERVICE_ACCOUNT_PASSWORD="${PG_SERVICE_ACCOUNT_PASSWORD:-$PG_SERVICE_ACCOUNT_PASS}"

export MONGO_SERVICE_ACCOUNT_NAME="${MONGO_SERVICE_ACCOUNT_NAME:-mongo}"
export MONGO_SERVICE_ACCOUNT_PASS="${MONGO_SERVICE_ACCOUNT_PASS:-mongo}"

export MSSQL_SERVICE_ACCOUNT_NAME="${MSSQL_SERVICE_ACCOUNT_NAME:-sa}"
export MSSQL_SERVICE_ACCOUNT_PASS="${MSSQL_SERVICE_ACCOUNT_PASS:-YourStrong!Passw0rd}"

export SNOWFLAKE_HOST="${SNOWFLAKE_HOST:-https://snowflake.localhost.localstack.cloud:4566}"
export SNOWFLAKE_SERVICE_ACCOUNT_NAME="${SNOWFLAKE_SERVICE_ACCOUNT_NAME:-test}"
export SNOWFLAKE_ACCESS_TOKEN="${SNOWFLAKE_ACCESS_TOKEN:-test}"
export SNOWFLAKE_AUTH_TOKEN_TYPE="${SNOWFLAKE_AUTH_TOKEN_TYPE:-LOCALSTACK_NO_AUTH}"

export NEO4J_SERVICE_ACCOUNT_NAME="${NEO4J_SERVICE_ACCOUNT_NAME:-neo4j}"
export NEO4J_SERVICE_ACCOUNT_PASS="${NEO4J_SERVICE_ACCOUNT_PASS:-appfw-neo4j-local}"

export API_TEST_BASE_URL_POSTGRES="${API_TEST_BASE_URL_POSTGRES:-http://127.0.0.1:8081}"
export API_TEST_BASE_URL_MONGO="${API_TEST_BASE_URL_MONGO:-http://127.0.0.1:8082}"
export API_TEST_BASE_URL_MSSQL="${API_TEST_BASE_URL_MSSQL:-http://127.0.0.1:8083}"
export API_TEST_BASE_URL_SNOWFLAKE="${API_TEST_BASE_URL_SNOWFLAKE:-http://127.0.0.1:8084}"

if env_flag_enabled "${APPFW_LOCAL_PREFLIGHT_START_SERVICES:-true}"; then
  run_logged_stage provider-services compose_up || fail_stage $?
else
  record_check provider-services true 0 "" "" "skipped because APPFW_LOCAL_PREFLIGHT_START_SERVICES=false"
fi

run_logged_stage wait-postgres wait_for_tcp 127.0.0.1 5432 PostgreSQL || fail_stage $?
run_logged_stage wait-mongo wait_for_tcp 127.0.0.1 27017 MongoDB || fail_stage $?
run_logged_stage wait-mssql wait_for_tcp 127.0.0.1 1433 "MS SQL Server" || fail_stage $?
run_logged_stage wait-snowflake wait_for_tcp snowflake.localhost.localstack.cloud 4566 "LocalStack Snowflake SQL API" || fail_stage $?
run_logged_stage wait-snowflake-control-plane wait_for_snowflake_control_plane || fail_stage $?

run_migration postgres pg_primary || fail_stage $?
run_migration mongo mongo_primary || fail_stage $?
run_migration mssql mssql_primary || fail_stage $?
run_migration snowflake snowflake_primary || fail_stage $?

run_logged_stage build-backend \
  env CARGO_TARGET_DIR="$backend_target_dir" \
  cargo build --locked --no-default-features --features "$backend_certification_features" --manifest-path "$product_app_root/backend/Cargo.toml" --bin backend || fail_stage $?

run_logged_stage start-postgres-backend start_backend postgres pg_primary 8081 || fail_stage $?
run_logged_stage start-mongo-backend start_backend mongo mongo_primary 8082 || fail_stage $?
run_logged_stage start-mssql-backend start_backend mssql mssql_primary 8083 || fail_stage $?
run_logged_stage start-snowflake-backend start_backend snowflake snowflake_primary 8084 || fail_stage $?

run_logged_stage provider-url-preflight write_provider_url_snapshot || fail_stage $?
record_check provider-url-preflight-artifact true 0 "" "$preflight_dir/provider-url-preflight.json" ""

run_json_stage provider-test "$preflight_dir/provider-test-output.json" \
  env APPFW_PROVIDER_TEST_REPORT_DIR="$report_dir" scripts/appfw framework provider-test --all --json || fail_stage $?
if [[ -f "$report_dir/provider-parity.json" ]]; then
  record_check provider-parity-artifact true 0 "" "$report_dir/provider-parity.json" ""
fi
run_logged_stage provider-certification-artifact assert_provider_parity || fail_stage $?

run_json_stage security-certification "$preflight_dir/security-certification-output.json" \
  scripts/appfw framework security-certification --json || fail_stage $?
if [[ -f "$report_dir/security-certification.json" ]]; then
  record_check security-certification-artifact true 0 "" "$report_dir/security-certification.json" ""
fi

finish 0
