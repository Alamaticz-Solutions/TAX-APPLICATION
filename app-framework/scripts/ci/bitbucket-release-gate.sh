#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
cd "$repo_root"

product_app_root="${APPFW_APP_ROOT:-$repo_root/examples/products/crm}"
framework_root="${APPFW_FRAMEWORK_ROOT:-$repo_root}"
release_backend_target_dir="${APPFW_RELEASE_BACKEND_TARGET_DIR:-$repo_root/target/ci-crm-backend}"
release_backend_binary="$release_backend_target_dir/debug/backend"
backend_certification_features="${APPFW_CERTIFICATION_BACKEND_FEATURES:-http,provider-postgres,provider-mongo,provider-mssql,provider-snowflake}"
export APPFW_APP_ROOT="$product_app_root"
export APPFW_FRAMEWORK_ROOT="$framework_root"

report_dir="${APPFW_RELEASE_ARTIFACT_DIR:-target/appfw}"
mkdir -p "$report_dir" api_tests/target

log() {
  printf '\n[%s] %s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" "$*"
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

node_runtime_version() {
  if ! command -v node >/dev/null 2>&1; then
    printf '\n'
    return
  fi

  node -p 'process.versions.node' 2>/dev/null || printf '\n'
}

ensure_frontend_node_runtime() {
  local required_version="${APPFW_NODE_VERSION:-22.17.0}"
  local current_version

  if [[ "$(id -u)" != "0" ]]; then
    export APPFW_NODE_INSTALL_DIR="${APPFW_NODE_INSTALL_DIR:-$repo_root/target/appfw-tools/node}"
    mkdir -p "$APPFW_NODE_INSTALL_DIR"
  fi

  log "Installing checksum-verified Node.js ${required_version} for the product frontend build"
  bash "$script_dir/install-node.sh" || return "$?"
  export PATH="${APPFW_NODE_INSTALL_DIR:-/usr/local}/bin:$PATH"

  current_version="$(node_runtime_version)"
  if [[ "$current_version" != "$required_version" ]] || ! command -v npm >/dev/null 2>&1; then
    log "Pinned Node.js installation did not provide exact runtime ${required_version} with npm"
    node --version >&2 || true
    npm --version >&2 || true
    return 1
  fi

  log "Node.js $(node --version) with npm $(npm --version) is ready for product frontend build"
}

require_snowflake_release_prereqs() {
  local token="${LOCALSTACK_AUTH_TOKEN:-}"
  local normalized
  normalized="$(printf '%s' "$token" | tr '[:upper:]' '[:lower:]')"

  if [[ -z "$token" ]]; then
    log "Missing LocalStack Snowflake release prerequisite"
    cat >&2 <<'EOF'
bitbucket-release-gate requires LOCALSTACK_AUTH_TOKEN to start the LocalStack
Snowflake service used by release-grade provider certification.

Configure LOCALSTACK_AUTH_TOKEN as a secured Bitbucket repository or workspace
variable, then rerun the release-check pipeline. The token is an infrastructure
credential for the CI Snowflake emulator; do not print it in logs or commit it
to the repository.
EOF
    return 2
  fi

  case "$normalized" in
    changeme|change-me|change_me|dummy|example|placeholder|test|your-token|your_token)
      log "Invalid placeholder LocalStack Snowflake release prerequisite"
      cat >&2 <<'EOF'
LOCALSTACK_AUTH_TOKEN is set to a placeholder value. Configure the real secured
Bitbucket variable before running the release gate.
EOF
      return 2
      ;;
  esac

  log "LocalStack Snowflake release prerequisite is configured"
}

odbc_driver18_registered() {
  # Prefer ODBCSYSINI/ODBCINSTINI when set so local/user-space Driver 18
  # installs (outside /etc and /opt/microsoft) can satisfy the gate.
  local odbcinst_file="/etc/odbcinst.ini"
  if [[ -n "${ODBCSYSINI:-}" ]]; then
    odbcinst_file="${ODBCSYSINI}/${ODBCINSTINI:-odbcinst.ini}"
  elif [[ -n "${ODBCINSTINI:-}" && "${ODBCINSTINI}" == /* ]]; then
    odbcinst_file="${ODBCINSTINI}"
  fi

  if [[ ! -f "$odbcinst_file" ]] \
    || ! grep -q '\[ODBC Driver 18 for SQL Server\]' "$odbcinst_file"; then
    return 1
  fi

  local driver_path
  driver_path="$(
    awk '
      $0 == "[ODBC Driver 18 for SQL Server]" { in_section = 1; next }
      /^\[/ { in_section = 0 }
      in_section && $1 ~ /^Driver/ {
        sub(/^[^=]*=[[:space:]]*/, "", $0)
        print $0
        exit
      }
    ' "$odbcinst_file"
  )"
  if [[ -z "$driver_path" || ! -f "$driver_path" ]]; then
    return 1
  fi

  if command -v odbcinst >/dev/null 2>&1 \
    && ! odbcinst -q -d 2>/dev/null | grep -q 'ODBC Driver 18 for SQL Server'; then
    return 1
  fi
  return 0
}

install_mssql_odbc_drivers() {
  # Plain MsSqlServer migrate/backends use Microsoft ODBC Driver 18 for
  # sql_password (and FreeTDS for ntlm). The release-gate image is stock
  # rust:bookworm and does not ship these drivers.
  if odbc_driver18_registered; then
    log "Microsoft ODBC Driver 18 already registered"
    return 0
  fi

  if [[ -f /etc/odbcinst.ini ]] \
    && grep -q '\[ODBC Driver 18 for SQL Server\]' /etc/odbcinst.ini \
    && ls /opt/microsoft/msodbcsql18/lib64/libmsodbcsql-18*.so.* >/dev/null 2>&1; then
    log "Microsoft ODBC Driver 18 already registered"
    return 0
  fi

  if [[ "$(id -u)" != "0" ]]; then
    log "Microsoft ODBC Driver 18 is required for MS SQL migrate/backends"
    cat >&2 <<'EOF'
bitbucket-release-gate requires Microsoft ODBC Driver 18. Install it system-wide
or point ODBCSYSINI/ODBCINSTINI at a user-local odbcinst.ini whose Driver=
path exists, then rerun the gate.
EOF
    return 1
  fi

  log "Installing unixODBC and Microsoft ODBC Driver 18 for MS SQL migrate/backends"
  apt-get update
  apt-get install -y --no-install-recommends \
    ca-certificates curl gnupg unixodbc unixodbc-dev \
    tdsodbc freetds-bin freetds-common
  curl -fsSL https://packages.microsoft.com/keys/microsoft.asc \
    | gpg --dearmor -o /usr/share/keyrings/microsoft-prod.gpg
  echo "deb [arch=amd64 signed-by=/usr/share/keyrings/microsoft-prod.gpg] https://packages.microsoft.com/debian/12/prod bookworm main" \
    > /etc/apt/sources.list.d/mssql-release.list
  apt-get update
  ACCEPT_EULA=Y apt-get install -y --no-install-recommends msodbcsql18

  local ms_driver_path
  ms_driver_path="$(ls /opt/microsoft/msodbcsql18/lib64/libmsodbcsql-18*.so.* 2>/dev/null | head -1)"
  if [[ -z "${ms_driver_path}" ]]; then
    log "msodbcsql18 installed but libmsodbcsql-18*.so.* was not found"
    return 1
  fi

  printf '%s\n' \
    '[FreeTDS]' \
    'Description=FreeTDS Driver' \
    'Driver=/usr/lib/x86_64-linux-gnu/odbc/libtdsodbc.so' \
    'Setup=/usr/lib/x86_64-linux-gnu/odbc/libtdsodbc.so' \
    'UsageCount=1' \
    '' \
    '[ODBC Driver 18 for SQL Server]' \
    'Description=Microsoft ODBC Driver 18 for SQL Server' \
    "Driver=${ms_driver_path}" \
    'UsageCount=1' \
    > /etc/odbcinst.ini

  if ! command -v odbcinst >/dev/null 2>&1 || ! odbcinst -q -d | grep -q 'ODBC Driver 18 for SQL Server'; then
    log "ODBC Driver 18 registration failed"
    odbcinst -q -d >&2 || true
    return 1
  fi
  log "Registered ODBC drivers: $(odbcinst -q -d | tr '\n' ' ')"
}

install_ci_prereqs() {
  local missing=0
  for command in curl nc python3 rsync xz; do
    if ! command -v "$command" >/dev/null 2>&1; then
      missing=1
    fi
  done

  if [[ "$missing" == "1" ]]; then
    log "Installing CI prerequisites"
    apt-get update
    apt-get install -y --no-install-recommends \
      ca-certificates \
      curl \
      libssl-dev \
      netcat-openbsd \
      pkg-config \
      python3 \
      python3-dev \
      rsync \
      xz-utils
  fi

  install_mssql_odbc_drivers
  ensure_frontend_node_runtime || return "$?"

  if command -v rustup >/dev/null 2>&1 && ! rustfmt --version >/dev/null 2>&1; then
    log "Installing Rust formatter component"
    rustup component add rustfmt
  fi

  if ! rustfmt --version >/dev/null 2>&1; then
    log "Rust formatter is unavailable after CI prerequisite installation"
    return 1
  fi
}

build_product_frontend() {
  local frontend_dir="$product_app_root/frontend"
  if [[ ! -f "$frontend_dir/package.json" ]]; then
    log "No product frontend package found at $frontend_dir; skipping product frontend build"
    return 0
  fi

  "$repo_root/scripts/ci/build-pds-components-package.sh" || return "$?"

  local package_version
  package_version="$(node -p "require('$repo_root/appfw_ui/pds_health/components/package.json').version")"
  local package_archive="$repo_root/target/appfw/packages/appfw-pds-health-components-$package_version.tgz"
  local lock_file="$frontend_dir/package-lock.json"
  local lock_backup=""
  if [[ -f "$lock_file" ]]; then
    lock_backup="$(mktemp)"
    cp "$lock_file" "$lock_backup"
  fi
  node "$repo_root/scripts/ci/sync-pds-package-lock-integrity.mjs" \
    --lock "$lock_file" \
    --archive "$package_archive" || {
      [[ -n "$lock_backup" ]] && mv "$lock_backup" "$lock_file"
      return 1
    }

  (
    set -e
    cd "$frontend_dir"
    npm_config_cache="$repo_root/target/appfw/npm-cache" \
      npm ci --no-audit --prefer-offline
    npm run build
  )
  local status=$?
  # Restore the pre-sync lockfile so release docs-check/handoff freshness is not
  # dirtied by a transient integrity rewrite used only for npm ci.
  if [[ -n "$lock_backup" ]]; then
    mv "$lock_backup" "$lock_file"
  fi
  return "$status"
}

append_host_aliases() {
  if [[ "$(id -u)" != "0" || ! -w /etc/hosts ]]; then
    log "Skipping /etc/hosts aliases because the build container cannot write them"
    return
  fi

  {
    printf '\n# App Framework provider release gate aliases\n'
    printf '127.0.0.1 postgres mongo mssql snowflake.localhost.localstack.cloud\n'
  } >>/etc/hosts
}

wait_for_tcp() {
  local host="$1"
  local port="$2"
  local label="$3"
  local attempts="${4:-90}"
  local probe_host="$host"

  if [[ "$host" == "snowflake.localhost.localstack.cloud" ]]; then
    if ! python3 - "$host" <<'PY' >/dev/null 2>&1
import socket
import sys

socket.getaddrinfo(sys.argv[1], None)
PY
    then
      log "$label hostname $host does not resolve; probing 127.0.0.1:$port"
      probe_host="127.0.0.1"
    fi
  fi

  for _ in $(seq 1 "$attempts"); do
    if nc -z "$probe_host" "$port" >/dev/null 2>&1; then
      log "$label is accepting TCP connections"
      return 0
    fi
    sleep 2
  done

  log "$label did not become reachable at $host:$port"
  return 1
}

wait_for_http() {
  local url="$1"
  local label="$2"
  local attempts="${3:-90}"

  for _ in $(seq 1 "$attempts"); do
    if curl -kfsS "$url" >/dev/null 2>&1; then
      log "$label is healthy"
      return 0
    fi
    sleep 2
  done

  log "$label did not become healthy at $url"
  return 1
}

wait_for_snowflake() {
  local attempts="${1:-90}"
  local snowflake_host="snowflake.localhost.localstack.cloud"
  local snowflake_url="http://${snowflake_host}:4566/session"
  local curl_args=(-kfsS -X POST -d '{}')

  if ! python3 - "$snowflake_host" <<'PY' >/dev/null 2>&1
import socket
import sys

socket.getaddrinfo(sys.argv[1], None)
PY
  then
    log "LocalStack Snowflake hostname does not resolve; using curl --resolve for control-plane probe"
    curl_args+=(--resolve "${snowflake_host}:4566:127.0.0.1")
  fi

  for _ in $(seq 1 "$attempts"); do
    if curl "${curl_args[@]}" "$snowflake_url" >/dev/null 2>&1; then
      log "LocalStack Snowflake control plane is healthy"
      return 0
    fi
    sleep 2
  done

  log "LocalStack Snowflake control plane did not become healthy"
  return 1
}

run_migration() {
  local provider="$1"
  local data_source="$2"
  local json_file="$report_dir/migrate-${provider}.json"
  local log_file="$report_dir/migrate-${provider}.log"

  log "Migrating $provider through $data_source"
  set +e
  APP_CRM_DATA_SOURCE_NAME="$data_source" scripts/appfw migrate --json >"$json_file" 2>"$log_file"
  local status=$?
  set -e

  if [[ $status -ne 0 ]]; then
    log "Migration failed for $provider"
    cat "$json_file" || true
    cat "$log_file" || true
    # scripts/appfw migrate also retains a shared diagnostic at migrate.log
    if [[ -f "$report_dir/migrate.log" ]]; then
      log "Contents of $report_dir/migrate.log"
      cat "$report_dir/migrate.log" || true
    fi
    return "$status"
  fi

  case "$provider" in
    postgres|mssql)
      local versioned_json_file="$report_dir/migrate-${provider}-versioned-expand.json"
      local versioned_log_file="$report_dir/migrate-${provider}-versioned-expand.log"

      log "Applying $provider versioned expand migrations through $data_source"
      set +e
      APP_CRM_DATA_SOURCE_NAME="$data_source" \
        scripts/appfw migrate apply --phase expand --json >"$versioned_json_file" 2>"$versioned_log_file"
      status=$?
      set -e

      if [[ $status -ne 0 ]]; then
        log "Versioned expand migration failed for $provider"
        cat "$versioned_json_file" || true
        cat "$versioned_log_file" || true
        return "$status"
      fi
      ;;
  esac
}

run_migrations_parallel() {
  # postgres/mongo/mssql/snowflake are independent databases, so migrating
  # them concurrently removes their durations from the critical path instead
  # of summing them. run_migration already writes per-provider json/log files
  # and prints its own failure diagnostics, so this only has to fan out the
  # calls and aggregate exit statuses.
  local providers=(postgres mongo mssql snowflake)
  local data_sources=(pg_primary mongo_primary mssql_primary snowflake_primary)
  local pids=()
  local index

  for index in "${!providers[@]}"; do
    run_migration "${providers[$index]}" "${data_sources[$index]}" &
    pids+=("$!")
  done

  local failed_provider=""
  local failed_status=0
  local status
  for index in "${!pids[@]}"; do
    status=0
    wait "${pids[$index]}" || status=$?
    if [[ "$status" -ne 0 && -z "$failed_provider" ]]; then
      failed_provider="${providers[$index]}"
      failed_status="$status"
    fi
  done

  if [[ -n "$failed_provider" ]]; then
    log "Parallel migration failed for $failed_provider"
    return "$failed_status"
  fi
}

start_backend() {
  local provider="$1"
  local data_source="$2"
  local port="$3"
  local log_file="$report_dir/backend-${provider}.log"
  local pid_file="$report_dir/backend-${provider}.pid"

  log "Starting $provider backend on port $port"
  (
    cd "$product_app_root/backend"
    APP_CRM_DATA_SOURCE_NAME="$data_source" API_PORT="$port" "$release_backend_binary"
  ) >"$log_file" 2>&1 &
  echo "$!" >"$pid_file"

  wait_for_http "http://127.0.0.1:${port}/health/ready" "$provider backend"
}

assert_provider_certification_artifact() {
  local report_file="$report_dir/provider-parity.json"

  if [[ ! -f "$report_file" ]]; then
    log "Missing provider certification artifact: $report_file"
    return 1
  fi

  python3 - "$report_file" <<'PY'
import json
import sys

path = sys.argv[1]
with open(path, "r", encoding="utf-8") as handle:
    report = json.load(handle)

failures = []
if report.get("ok") is not True:
    failures.append("top-level provider parity ok flag is not true")

expected_providers = {"postgres", "mongo", "mssql", "snowflake"}
seen_providers = {
    provider.get("provider")
    for provider in report.get("providers", [])
    if provider.get("provider")
}
missing_providers = sorted(expected_providers - seen_providers)
unexpected_providers = sorted(seen_providers - expected_providers)
if missing_providers:
    failures.append(f"missing provider entries: {', '.join(missing_providers)}")
if unexpected_providers:
    failures.append(f"unexpected provider entries: {', '.join(unexpected_providers)}")

for provider in report.get("providers", []):
    provider_name = provider.get("provider", "<unknown>")
    if provider.get("ok") is not True:
        failures.append(f"{provider_name}: provider ok flag is not true")
    for area in provider.get("areas", []):
        if area.get("status") != "live-certified":
            continue
        result = area.get("live_result")
        if result != "passed":
            failures.append(
                f"{provider_name}.{area.get('area', '<unknown>')}: "
                f"live-certified result is {result!r}"
            )

if failures:
    print("Provider certification gate failed:", file=sys.stderr)
    for failure in failures:
        print(f"  - {failure}", file=sys.stderr)
    sys.exit(1)
PY
}

write_release_gate_failure_evidence() {
  local failed_stage="$1"
  local exit_status="$2"
  local evidence_file="$report_dir/bitbucket-release-gate.json"

  python3 - "$report_dir" "$evidence_file" "$failed_stage" "$exit_status" <<'PY'
import datetime as dt
import json
import os
import sys
from pathlib import Path

report_dir = Path(sys.argv[1])
evidence_file = Path(sys.argv[2])
failed_stage = sys.argv[3]
exit_status = int(sys.argv[4])


def utc_now():
    return (
        dt.datetime.now(dt.timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z")
    )


def load_optional(name):
    path = report_dir / name
    if not path.is_file():
        return None
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except Exception as exc:
        return {"_parse_error": str(exc)}


def env_bool(name):
    return str(os.environ.get(name, "")).strip().lower() in (
        "1",
        "true",
        "yes",
        "y",
        "on",
        "required",
        "require",
    )


release_requirements = {
    "security_assurance_decision_required": env_bool("APPFW_RELEASE_REQUIRE_SECURITY_ASSURANCE_DECISION"),
    "production_attestations_required": env_bool("APPFW_RELEASE_REQUIRE_PRODUCTION_ATTESTATIONS"),
    "ops_certification_required": env_bool("APPFW_RELEASE_REQUIRE_OPS_CERTIFICATION"),
    "live_ops_evidence_required": env_bool("APPFW_RELEASE_REQUIRE_LIVE_OPS_EVIDENCE"),
    "pds_baseline_evidence_required": env_bool("APPFW_RELEASE_REQUIRE_PDS_BASELINE_EVIDENCE"),
    "performance_evidence_required": env_bool("APPFW_RELEASE_REQUIRE_PERFORMANCE_EVIDENCE"),
    "release_identity_required": env_bool("APPFW_RELEASE_REQUIRE_RELEASE_IDENTITY"),
}
focused_evidence = env_bool("APPFW_BITBUCKET_RELEASE_GATE_FOCUSED_EVIDENCE")
strict_requirements_satisfied = all(release_requirements.values())

reports = {
    "release_check": load_optional("release-check.json"),
    "release_evidence_check": load_optional("release-evidence-check.json"),
    "security_assurance_decision": load_optional("security-assurance-decision.json"),
    "security_certification": load_optional("security-certification.json"),
    "ops_certification": load_optional("ops-certification.json"),
    "pds_security_baseline": load_optional("pds-security-baseline.json"),
    "release_identity": load_optional("release-identity.json"),
    "provider_parity": load_optional("provider-parity.json"),
}

release_blockers = [f"{failed_stage} failed with exit status {exit_status}"]


def append_release_blocker(value):
    if isinstance(value, str) and value.strip():
        release_blockers.append(value.strip())


def is_wrapper_self_check(value):
    return isinstance(value, str) and value.strip().startswith("bitbucket release gate ")


def report_blocker_values(report, include_wrapper_self_checks=True):
    if not isinstance(report, dict):
        return []
    failure_summary = report.get("failure_summary")
    if isinstance(failure_summary, dict):
        root_causes = [
            value.strip()
            for value in failure_summary.get("root_causes", [])
            if isinstance(value, str)
            and value.strip()
            and (include_wrapper_self_checks or not is_wrapper_self_check(value))
        ]
        if root_causes:
            return root_causes
    for field in ("release_blockers", "failures"):
        values = [
            value.strip()
            for value in report.get(field, [])
            if isinstance(value, str)
            and value.strip()
            and (include_wrapper_self_checks or not is_wrapper_self_check(value))
        ]
        if values:
            return values
    return []


def append_report_blockers(name, report, include_wrapper_self_checks=True, include_status=True):
    if not isinstance(report, dict):
        return False
    if report.get("_parse_error"):
        append_release_blocker(f"{name} could not be parsed: {report['_parse_error']}")
        return True
    blocker_values = report_blocker_values(
        report,
        include_wrapper_self_checks=include_wrapper_self_checks,
    )
    if blocker_values:
        for value in blocker_values:
            append_release_blocker(value)
        return True
    added_status = False
    if include_status:
        if "ok" in report and report.get("ok") is not True:
            append_release_blocker(f"{name} ok is not true")
            added_status = True
        if "release_ready" in report and report.get("release_ready") is not True:
            append_release_blocker(f"{name} release_ready is not true")
            added_status = True
    return added_status


primary_report = reports.get("release_evidence_check")
if not append_report_blockers(
    "release_evidence_check",
    primary_report,
    include_wrapper_self_checks=False,
    include_status=False,
):
    for name, report in reports.items():
        if name == "release_evidence_check":
            continue
        append_report_blockers(name, report)

deduped_blockers = []
for blocker in release_blockers:
    if blocker not in deduped_blockers:
        deduped_blockers.append(blocker)

evidence = {
    "command": "bitbucket-release-gate",
    "ok": False,
    "focused_evidence": focused_evidence,
    "ci_ready": False,
    "release_authority": "focused-ci-only" if focused_evidence else "strict-managed-release",
    "release_authority_boundary": {
        "focused_evidence_satisfies_release": False,
        "strict_release_gate_required_for_promotion": True,
        "strict_release_gate_satisfied": False,
        "managed_release_requirements_satisfied": strict_requirements_satisfied,
    },
    "release_ready": False,
    "failed_stage": failed_stage,
    "exit_status": exit_status,
    "generated_at_utc": utc_now(),
    "release_requirements": release_requirements,
    "release_blockers": deduped_blockers,
    "failure_summary": {
        "root_causes": deduped_blockers,
    },
    "reports": reports,
}
evidence_file.write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
}

write_focused_release_gate_evidence() {
  local evidence_file="$report_dir/bitbucket-release-gate.json"

  python3 - "$report_dir" "$evidence_file" <<'PY'
import datetime as dt
import json
import os
import pathlib
import sys

report_dir = pathlib.Path(sys.argv[1])
evidence_file = pathlib.Path(sys.argv[2])


def env_bool(name):
    return str(os.environ.get(name, "")).strip().lower() in (
        "1",
        "true",
        "yes",
        "y",
        "on",
        "required",
        "require",
    )


def load_optional(name):
    path = report_dir / name
    if not path.is_file():
        return None
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except Exception as exc:
        return {"_parse_error": str(exc)}


release_check = load_optional("release-check.json")
release_check_ok = (
    isinstance(release_check, dict)
    and release_check.get("ok") is True
    and release_check.get("release_ready") is True
)
release_requirements = {
    "security_assurance_decision_required": env_bool("APPFW_RELEASE_REQUIRE_SECURITY_ASSURANCE_DECISION"),
    "production_attestations_required": env_bool("APPFW_RELEASE_REQUIRE_PRODUCTION_ATTESTATIONS"),
    "ops_certification_required": env_bool("APPFW_RELEASE_REQUIRE_OPS_CERTIFICATION"),
    "live_ops_evidence_required": env_bool("APPFW_RELEASE_REQUIRE_LIVE_OPS_EVIDENCE"),
    "pds_baseline_evidence_required": env_bool("APPFW_RELEASE_REQUIRE_PDS_BASELINE_EVIDENCE"),
    "performance_evidence_required": env_bool("APPFW_RELEASE_REQUIRE_PERFORMANCE_EVIDENCE"),
    "release_identity_required": env_bool("APPFW_RELEASE_REQUIRE_RELEASE_IDENTITY"),
}
strict_requirements_satisfied = all(release_requirements.values())
root_causes = []
if not release_check_ok:
    root_causes.append("focused release-check ok/release_ready is not true")
root_causes.append(
    "focused release gate evidence is CI-readiness only; strict managed release authority is required for promotion"
)

evidence = {
    "command": "bitbucket-release-gate",
    "focused_evidence": True,
    "ci_ready": release_check_ok,
    "release_authority": "focused-ci-only",
    "release_authority_boundary": {
        "focused_evidence_satisfies_release": False,
        "strict_release_gate_required_for_promotion": True,
        "strict_release_gate_satisfied": False,
        "managed_release_requirements_satisfied": strict_requirements_satisfied,
    },
    "ok": release_check_ok,
    "release_ready": False,
    "generated_at_utc": dt.datetime.now(dt.timezone.utc)
    .replace(microsecond=0)
    .isoformat()
    .replace("+00:00", "Z"),
    "release_requirements": release_requirements,
    "release_blockers": root_causes,
    "failure_summary": {
        "root_causes": root_causes,
    },
    "reports": {
        "release_check": release_check,
        "security_certification": load_optional("security-certification.json"),
        "ops_certification": load_optional("ops-certification.json"),
        "pds_security_baseline": load_optional("pds-security-baseline.json"),
        "provider_parity": load_optional("provider-parity.json"),
        "provider_performance": load_optional("provider-performance.json"),
        "load_test": load_optional("load-test.json"),
    },
}
evidence_file.write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n", encoding="utf-8")
if not release_check_ok:
    sys.exit(1)
PY
}

run_stage_or_record_failure() {
  local stage="$1"
  shift

  set +e
  "$@"
  local status=$?
  set -e

  if [[ $status -ne 0 ]]; then
    write_release_gate_failure_evidence "$stage" "$status"
    exit "$status"
  fi
}

write_release_gate_evidence() {
  local evidence_file="$report_dir/bitbucket-release-gate.json"

  python3 - "$report_dir" "$evidence_file" <<'PY'
import datetime as dt
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

report_dir = Path(sys.argv[1])
evidence_file = Path(sys.argv[2])
repo_root = Path.cwd()


def load_json(path):
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def first_non_empty(mapping, keys):
    if not isinstance(mapping, dict):
        return ""
    for key in keys:
        value = mapping.get(key)
        if isinstance(value, str) and value.strip():
            return value.strip()
    return ""


def nested_mapping(mapping, keys):
    if not isinstance(mapping, dict):
        return {}
    for key in keys:
        value = mapping.get(key)
        if isinstance(value, dict):
            return value
    return {}


def baseline_version_from_decision(decision):
    if not isinstance(decision, dict):
        return ""
    baseline = decision.get("baseline")
    if isinstance(baseline, dict) and isinstance(baseline.get("version"), str):
        return baseline.get("version", "").strip()
    if isinstance(decision.get("baseline_version"), str):
        return decision.get("baseline_version", "").strip()
    return ""


def parse_timestamp_with_timezone(value):
    if not isinstance(value, str) or not value.strip():
        return None
    try:
        parsed = dt.datetime.fromisoformat(value.strip().replace("Z", "+00:00"))
    except ValueError:
        return None
    if parsed.tzinfo is None:
        return None
    return parsed.astimezone(dt.timezone.utc)


def resolve_retained_path(path_text):
    if not isinstance(path_text, str) or not path_text.strip():
        return None
    candidate = Path(path_text.strip()).expanduser()
    if candidate.is_absolute():
        return candidate if candidate.is_file() else None
    for base in (repo_root, report_dir):
        resolved = base / candidate
        if resolved.is_file():
            return resolved
    return None


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact(name, path):
    return {
        "name": name,
        "path": path.as_posix(),
        "sha256": sha256(path),
        "bytes": path.stat().st_size,
    }


def artifact_entry(report, name):
    if not isinstance(report, dict):
        return None
    for item in report.get("artifacts", []):
        if isinstance(item, dict) and item.get("name") == name:
            return item
    return None


def artifact_required(report, name):
    item = artifact_entry(report, name)
    return isinstance(item, dict) and item.get("required") is True


def load_json_if_present(path):
    if not path.is_file():
        return None
    return load_json(path)


def discover_tracked_files():
    try:
        result = subprocess.run(
            ["git", "ls-files"],
            text=True,
            capture_output=True,
            check=True,
        )
    except Exception:
        return []
    return [line for line in result.stdout.splitlines() if line]


def frontend_lockfiles():
    return [
        path
        for path in discover_tracked_files()
        if re.search(r"(^|/)(package-lock\.json|pnpm-lock\.yaml|yarn\.lock)$", path)
        and "/node_modules/" not in f"/{path}"
    ]


def env_flag_enabled(value):
    return str(value or "").strip().lower() in {
        "1",
        "true",
        "yes",
        "y",
        "on",
        "required",
        "require",
    }


def release_mode_requires_image_sbom():
    return env_flag_enabled(os.environ.get("APPFW_REQUIRE_DEPLOYABLE_IMAGE_SBOM")) or env_flag_enabled(
        os.environ.get("APPFW_RELEASE_REQUIRE_IMAGE_SBOM")
    )


def env_image_sources():
    sources = []
    for key in (
        "APPFW_SBOM_IMAGE_SOURCE",
        "APPFW_SBOM_IMAGE_SOURCES",
        "APPFW_SBOM_IMAGE_REF",
        "APPFW_SBOM_IMAGE_TAR",
        "APPFW_SBOM_IMAGE_ARTIFACT",
        "APPFW_DEPLOYABLE_IMAGE",
        "APPFW_IMAGE_REF",
        "APPFW_IMAGE_TAR",
        "APPFW_IMAGE_ARTIFACT",
    ):
        value = os.environ.get(key, "")
        if value:
            sources.extend(
                source.strip()
                for source in re.split(r"[\n,]+", value)
                if source.strip()
            )
    return sources


def retained_image_artifacts():
    paths = []
    for pattern in (
        "deployable-image*.tar",
        "*.image.tar",
        "*.oci",
        "*.oci.tar",
        "images/*.tar",
        "images/*.tar.gz",
        "containers/*.tar",
        "containers/*.tar.gz",
    ):
        paths.extend(report_dir.glob(pattern))
    return [path for path in paths if path.is_file()]


def image_sbom_context(supply=None, sbom_manifest=None):
    env_sources = env_image_sources()
    retained_artifacts = retained_image_artifacts()
    required_by_release_mode = release_mode_requires_image_sbom()
    supply_required = artifact_required(supply, "deployable-image-cyclonedx-sbom")
    manifest_required = artifact_required(sbom_manifest, "deployable-image-cyclonedx-sbom")

    reasons = []
    if required_by_release_mode:
        reasons.append(
            "release mode requires deployable image SBOM via "
            "APPFW_REQUIRE_DEPLOYABLE_IMAGE_SBOM or APPFW_RELEASE_REQUIRE_IMAGE_SBOM"
        )
    if env_sources:
        reasons.append(f"deployable image source env var set ({len(env_sources)} source(s))")
    if retained_artifacts:
        reasons.append(
            f"retained deployable image artifact detected ({len(retained_artifacts)} file(s))"
        )
    if supply_required:
        reasons.append("supply-chain evidence marks deployable image SBOM required")
    if manifest_required:
        reasons.append("SBOM manifest marks deployable image SBOM required")

    required = bool(reasons)
    if required:
        detail = (
            "required: "
            + "; ".join(reasons)
            + ". Expected sbom-deployable-image.cdx.json in CycloneDX JSON format; "
            "generate it with scripts/ci/supply-chain-gate.sh and syft."
        )
        applicability = "required"
    else:
        detail = (
            "not-applicable: no APPFW_SBOM_IMAGE_* / APPFW_IMAGE_* source or retained "
            "image tar/OCI artifact was found, and release mode did not require one"
        )
        applicability = "not-applicable"

    return {
        "required": required,
        "applicability": applicability,
        "detail": detail,
        "env_sources": env_sources,
        "retained_artifacts": [path.as_posix() for path in retained_artifacts],
        "required_by_release_mode": required_by_release_mode,
        "supply_required": supply_required,
        "manifest_required": manifest_required,
    }


release_path = report_dir / "release-check.json"
provider_path = report_dir / "provider-parity.json"
mcp_posture_path = report_dir / "release-mcp-posture.json"
ops_certification_path = report_dir / "ops-certification.json"
pds_baseline_path = report_dir / "pds-security-baseline.json"
release_identity_path = report_dir / "release-identity.json"
security_report_paths = {
    "supply-chain-gate": report_dir / "supply-chain-gate.json",
    "phi-log-lint": report_dir / "phi-log-lint.json",
    "secret-scan": report_dir / "secret-scan.json",
    "security-assurance-decision": report_dir / "security-assurance-decision.json",
    "security-certification": report_dir / "security-certification.json",
    "release-evidence-check": report_dir / "release-evidence-check.json",
}
gitleaks_report_path = report_dir / "gitleaks-report.json"
sbom_manifest_path = report_dir / "sbom-manifest.json"
rust_sbom_path = report_dir / "sbom-rust-workspace.cdx.json"
frontend_sbom_path = report_dir / "sbom-frontend-packages.cdx.json"
image_sbom_path = report_dir / "sbom-deployable-image.cdx.json"
supply_report_preview = load_json_if_present(security_report_paths["supply-chain-gate"])
sbom_manifest_preview = load_json_if_present(sbom_manifest_path)
frontend_sbom_required = bool(frontend_lockfiles())
image_context = image_sbom_context(supply_report_preview, sbom_manifest_preview)
image_sbom_required = image_context["required"]
sbom_artifacts = {
    "sbom-manifest": sbom_manifest_path,
    "rust-cyclonedx-sbom": rust_sbom_path,
}
if frontend_sbom_required:
    sbom_artifacts["frontend-cyclonedx-sbom"] = frontend_sbom_path
if image_sbom_required:
    sbom_artifacts["deployable-image-cyclonedx-sbom"] = image_sbom_path
required_artifacts = {
    "release-check": release_path,
    "provider-parity": provider_path,
    "release-mcp-posture": mcp_posture_path,
    "ops-certification": ops_certification_path,
    "pds-security-baseline": pds_baseline_path,
    "release-identity": release_identity_path,
    **security_report_paths,
    "gitleaks-report": gitleaks_report_path,
    **sbom_artifacts,
}
required_artifact_reasons = {}
if image_sbom_required:
    required_artifact_reasons["deployable-image-cyclonedx-sbom"] = image_context["detail"]
missing = [
    (
        name,
        path.as_posix(),
        required_artifact_reasons.get(name),
    )
    for name, path in required_artifacts.items()
    if not path.is_file()
]
if missing:
    print("Missing required release evidence:", file=sys.stderr)
    for name, path, reason in missing:
        if reason:
            print(f"  - {name} ({path}): {reason}", file=sys.stderr)
        else:
            print(f"  - {name} ({path})", file=sys.stderr)
    sys.exit(1)

release_report = load_json(release_path)
provider_report = load_json(provider_path)
mcp_posture_report = load_json(mcp_posture_path)
ops_certification_report = load_json(ops_certification_path)
pds_baseline_report = load_json(pds_baseline_path)
release_identity_report = load_json(release_identity_path)
security_reports = {
    name: load_json(path)
    for name, path in security_report_paths.items()
}
gitleaks_report = load_json(gitleaks_report_path)
if release_report.get("ok") is not True:
    print("release-check report is not ok", file=sys.stderr)
    sys.exit(1)
if provider_report.get("ok") is not True:
    print("provider parity report is not ok", file=sys.stderr)
    sys.exit(1)
if mcp_posture_report.get("ok") is not True:
    print("MCP release posture report is not ok", file=sys.stderr)
    sys.exit(1)
if ops_certification_report.get("ok") is not True:
    print("ops certification report is not ok", file=sys.stderr)
    sys.exit(1)
if pds_baseline_report.get("ok") is not True:
    print("PDS baseline report is not ok", file=sys.stderr)
    sys.exit(1)
if release_identity_report.get("ok") is not True:
    print("release identity report is not ok", file=sys.stderr)
    sys.exit(1)
if release_identity_report.get("release_ready") is not True:
    print("release identity report is not release-ready", file=sys.stderr)
    sys.exit(1)
pds_baseline_required = (
    str(os.environ.get("APPFW_RELEASE_REQUIRE_PDS_BASELINE_EVIDENCE", "")).strip().lower()
    in ("1", "true", "yes", "y", "on", "required", "require")
)
if pds_baseline_required:
    if pds_baseline_report.get("release_ready") is not True:
        print("PDS baseline report is not release-ready", file=sys.stderr)
        sys.exit(1)
    pds_decision_path = resolve_retained_path(pds_baseline_report.get("decision_file"))
    if pds_decision_path is None:
        print("PDS baseline retained decision file is missing", file=sys.stderr)
        sys.exit(1)
    pds_decision = load_json(pds_decision_path)
    if not isinstance(pds_decision, dict):
        print("PDS baseline retained decision is not a JSON object", file=sys.stderr)
        sys.exit(1)
    if pds_decision.get("ok") is not True or pds_decision.get("release_ready") is not True:
        print("PDS baseline retained decision does not declare ok:true and release_ready:true", file=sys.stderr)
        sys.exit(1)
    if baseline_version_from_decision(pds_decision) != "r4.5":
        print("PDS baseline retained decision does not reference baseline r4.5", file=sys.stderr)
        sys.exit(1)
    pds_decision_authority = nested_mapping(pds_decision, ("release_authority", "authority", "approval"))
    pds_decision_owner = first_non_empty(pds_decision_authority, ("owner", "release_owner")) or first_non_empty(
        pds_decision, ("owner", "release_owner")
    )
    pds_decision_approver = first_non_empty(pds_decision_authority, ("approver", "approved_by")) or first_non_empty(
        pds_decision, ("approver", "approved_by")
    )
    pds_decision_approved_at = first_non_empty(
        pds_decision_authority,
        ("approved_at_utc", "approved_at", "approved_on"),
    ) or first_non_empty(pds_decision, ("approved_at_utc", "approved_at", "approved_on"))
    if not pds_decision_owner:
        print("PDS baseline retained decision is missing release authority owner", file=sys.stderr)
        sys.exit(1)
    if not pds_decision_approver:
        print("PDS baseline retained decision is missing release authority approver", file=sys.stderr)
        sys.exit(1)
    parsed_pds_approved_at = parse_timestamp_with_timezone(pds_decision_approved_at)
    if parsed_pds_approved_at is None:
        print("PDS baseline retained decision approval timestamp is invalid", file=sys.stderr)
        sys.exit(1)
    if parsed_pds_approved_at > dt.datetime.now(dt.timezone.utc) + dt.timedelta(minutes=5):
        print("PDS baseline retained decision approval timestamp is in the future", file=sys.stderr)
        sys.exit(1)
    pds_required_ids = {"LIVE-015", "LIVE-016", "LIVE-017", "LIVE-018", "LIVE-019", "LIVE-020", "LIVE-021"}
    pds_decision_summary = pds_baseline_report.get("decision_summary")
    if not isinstance(pds_decision_summary, dict):
        print("PDS baseline decision summary is missing", file=sys.stderr)
        sys.exit(1)
    pds_covered_ids = {
        str(item)
        for item in pds_decision_summary.get("covered_work_items", [])
        if isinstance(item, str)
    }
    pds_satisfied_ids = {
        str(item)
        for item in pds_decision_summary.get("evidence_backed_work_items", [])
        if isinstance(item, str)
    } | {
        str(item)
        for item in pds_decision_summary.get("risk_accepted_work_items", [])
        if isinstance(item, str)
    }
    missing_covered = sorted(pds_required_ids - pds_covered_ids)
    missing_satisfied = sorted(pds_required_ids - pds_satisfied_ids)
    if missing_covered or missing_satisfied:
        print(
            "PDS baseline decision summary is incomplete: "
            f"missing_covered={missing_covered} missing_satisfied={missing_satisfied}",
            file=sys.stderr,
        )
        sys.exit(1)
mcp_enabled_value = str(mcp_posture_report.get("mcp_enabled", "")).strip().lower()
if mcp_enabled_value not in ("false", "0", "no", "off", ""):
    print("MCP must remain disabled for this release posture", file=sys.stderr)
    sys.exit(1)
for name, report in security_reports.items():
    if report.get("ok") is not True:
        print(f"{name} report is not ok", file=sys.stderr)
        sys.exit(1)
if not isinstance(gitleaks_report, list):
    print("gitleaks report is not a JSON array", file=sys.stderr)
    sys.exit(1)

artifacts = [
    artifact(name, path)
    for name, path in required_artifacts.items()
]
optional_artifacts = {
    "release-check-output": report_dir / "release-check-output.json",
    "agent-handoff": report_dir / "agent-handoff.json",
    "boundary-check": report_dir / "boundary-check.json",
}
for name, path in optional_artifacts.items():
    if path.is_file():
        artifacts.append(artifact(name, path))

checks = [
    {
        "name": check.get("name"),
        "ok": check.get("ok"),
        "artifact": check.get("artifact"),
        "log": check.get("log"),
    }
    for check in release_report.get("checks", [])
]

provider_summaries = []
for provider in provider_report.get("providers", []):
    live_areas = [
        area
        for area in provider.get("areas", [])
        if area.get("status") == "live-certified"
    ]
    provider_summaries.append(
        {
            "provider": provider.get("provider"),
            "ok": provider.get("ok"),
            "data_source": provider.get("data_source"),
            "base_url": provider.get("base_url"),
            "live_certified_total": len(live_areas),
            "live_certified_passed": sum(
                1 for area in live_areas if area.get("live_result") == "passed"
            ),
        }
    )

security_gate_summaries = [
    {
        "name": "supply-chain-gate",
        "ok": security_reports["supply-chain-gate"].get("ok"),
        "checks": security_reports["supply-chain-gate"].get("checks", []),
    },
    {
        "name": "phi-log-lint",
        "ok": security_reports["phi-log-lint"].get("ok"),
        "scanned_files": security_reports["phi-log-lint"].get("scanned_files"),
        "findings": security_reports["phi-log-lint"].get("findings"),
        "redaction": security_reports["phi-log-lint"].get("redaction"),
    },
    {
        "name": "secret-scan",
        "ok": security_reports["secret-scan"].get("ok"),
        "findings": security_reports["secret-scan"].get("findings"),
        "history_scan": security_reports["secret-scan"].get("history_scan"),
        "redaction": security_reports["secret-scan"].get("redaction"),
    },
    {
        "name": "security-assurance-decision",
        "ok": security_reports["security-assurance-decision"].get("ok"),
        "release_ready": security_reports["security-assurance-decision"].get("release_ready"),
        "failure_count": security_reports["security-assurance-decision"].get("failure_count"),
        "requirements": security_reports["security-assurance-decision"].get("requirements"),
    },
    {
        "name": "security-certification",
        "ok": security_reports["security-certification"].get("ok"),
        "release_ready": security_reports["security-certification"].get("release_ready"),
        "failure_count": security_reports["security-certification"].get("failure_count"),
    },
    {
        "name": "release-evidence-check",
        "ok": security_reports["release-evidence-check"].get("ok"),
        "release_ready": security_reports["release-evidence-check"].get("release_ready"),
        "failure_count": security_reports["release-evidence-check"].get("failure_count"),
        "release_blocker_count": security_reports["release-evidence-check"].get("release_blocker_count"),
        "redaction": security_reports["release-evidence-check"].get("redaction"),
    },
    {
        "name": "ops-certification",
        "ok": ops_certification_report.get("ok"),
        "release_ready": ops_certification_report.get("release_ready"),
        "live_evidence_required": ops_certification_report.get("live_evidence_required"),
        "failure_count": ops_certification_report.get("failure_count"),
    },
    {
        "name": "pds-security-baseline",
        "ok": pds_baseline_report.get("ok"),
        "release_ready": pds_baseline_report.get("release_ready"),
        "live_evidence_required": pds_baseline_report.get("live_evidence_required"),
        "failure_count": pds_baseline_report.get("failure_count"),
        "baseline": pds_baseline_report.get("baseline"),
        "decision_summary": pds_baseline_report.get("decision_summary"),
    },
    {
        "name": "release-identity",
        "ok": release_identity_report.get("ok"),
        "release_ready": release_identity_report.get("release_ready"),
        "release_identity_required": release_identity_report.get("release_identity_required"),
        "git": release_identity_report.get("git"),
        "decision_artifact": release_identity_report.get("decision_artifact"),
    },
]

ci_keys = [
    "BITBUCKET_BUILD_NUMBER",
    "BITBUCKET_COMMIT",
    "BITBUCKET_BRANCH",
    "BITBUCKET_TAG",
    "BITBUCKET_PIPELINE_UUID",
    "BITBUCKET_STEP_UUID",
    "BITBUCKET_REPO_FULL_NAME",
]
ci = {
    key.lower(): value
    for key in ci_keys
    if (value := os.environ.get(key))
}

def env_bool(name):
    return str(os.environ.get(name, "")).strip().lower() in (
        "1",
        "true",
        "yes",
        "y",
        "on",
        "required",
        "require",
    )


release_requirements = {
    "security_assurance_decision_required": env_bool("APPFW_RELEASE_REQUIRE_SECURITY_ASSURANCE_DECISION"),
    "production_attestations_required": env_bool("APPFW_RELEASE_REQUIRE_PRODUCTION_ATTESTATIONS"),
    "ops_certification_required": env_bool("APPFW_RELEASE_REQUIRE_OPS_CERTIFICATION"),
    "live_ops_evidence_required": env_bool("APPFW_RELEASE_REQUIRE_LIVE_OPS_EVIDENCE"),
    "pds_baseline_evidence_required": env_bool("APPFW_RELEASE_REQUIRE_PDS_BASELINE_EVIDENCE"),
    "performance_evidence_required": env_bool("APPFW_RELEASE_REQUIRE_PERFORMANCE_EVIDENCE"),
    "release_identity_required": env_bool("APPFW_RELEASE_REQUIRE_RELEASE_IDENTITY"),
}
strict_requirements_satisfied = all(release_requirements.values())

release_evidence_report = security_reports["release-evidence-check"]
release_blockers = []


def append_release_blocker(value):
    if isinstance(value, str) and value.strip() and value not in release_blockers:
        release_blockers.append(value)


def is_wrapper_self_check(value):
    return isinstance(value, str) and value.strip().startswith("bitbucket release gate ")


def report_blocker_values(report, include_wrapper_self_checks=True):
    if not isinstance(report, dict):
        return []
    failure_summary = report.get("failure_summary")
    if isinstance(failure_summary, dict):
        root_causes = failure_summary.get("root_causes")
        if isinstance(root_causes, list):
            values = []
            for value in root_causes:
                if (
                    isinstance(value, str)
                    and value.strip()
                    and (include_wrapper_self_checks or not is_wrapper_self_check(value))
                ):
                    values.append(value.strip())
            if values:
                return values
    for field in ("release_blockers", "failures"):
        values = report.get(field)
        if isinstance(values, list):
            fallback_values = []
            for value in values:
                if (
                    isinstance(value, str)
                    and value.strip()
                    and (include_wrapper_self_checks or not is_wrapper_self_check(value))
                ):
                    fallback_values.append(value.strip())
            if fallback_values:
                return fallback_values
    return []


def append_report_blockers(report, include_wrapper_self_checks=True):
    values = report_blocker_values(
        report,
        include_wrapper_self_checks=include_wrapper_self_checks,
    )
    for value in values:
        append_release_blocker(value)
    return bool(values)


reports_for_blockers = [
    ("release-check", release_report),
    ("security-assurance-decision", security_reports["security-assurance-decision"]),
    ("security-certification", security_reports["security-certification"]),
    ("ops-certification", ops_certification_report),
    ("pds-security-baseline", pds_baseline_report),
    ("release-identity", release_identity_report),
]

release_evidence_has_causes = append_report_blockers(
    release_evidence_report,
    include_wrapper_self_checks=False,
)
if release_evidence_has_causes:
    reports_to_summarize = []
else:
    reports_to_summarize = reports_for_blockers

for name, report in reports_to_summarize:
    has_report_causes = append_report_blockers(report)
    if has_report_causes:
        continue
    if report.get("ok") is not True:
        append_release_blocker(f"{name} ok is not true")
    if report.get("release_ready") is not True:
        append_release_blocker(f"{name} release_ready is not true")

for requirement_key, requirement_label in (
    ("security_assurance_decision_required", "security assurance decision"),
    ("production_attestations_required", "production attestations"),
    ("ops_certification_required", "operations certification"),
    ("live_ops_evidence_required", "live operations evidence"),
    ("pds_baseline_evidence_required", "PDS baseline evidence"),
    ("performance_evidence_required", "performance evidence"),
    ("release_identity_required", "release identity evidence"),
):
    if release_requirements.get(requirement_key) is not True:
        append_release_blocker(f"bitbucket release requirement {requirement_label} is not enabled")

release_ready = not release_blockers

evidence = {
    "command": "bitbucket-release-gate",
    "ok": release_ready,
    "focused_evidence": False,
    "ci_ready": release_ready,
    "release_authority": "strict-managed-release",
    "release_authority_boundary": {
        "focused_evidence_satisfies_release": False,
        "strict_release_gate_required_for_promotion": True,
        "strict_release_gate_satisfied": release_ready,
        "managed_release_requirements_satisfied": strict_requirements_satisfied,
    },
    "release_ready": release_ready,
    "release_blockers": release_blockers,
    "failure_summary": {
        "root_causes": release_blockers,
    },
    "generated_at_utc": dt.datetime.now(dt.timezone.utc)
    .replace(microsecond=0)
    .isoformat()
    .replace("+00:00", "Z"),
    "ci": ci,
    "release_requirements": release_requirements,
    "artifacts": artifacts,
    "release_check": {
        "ok": release_report.get("ok"),
        "release_ready": release_report.get("release_ready"),
        "checks": checks,
    },
    "mcp_release_posture": {
        "ok": mcp_posture_report.get("ok"),
        "release_posture": mcp_posture_report.get("release_posture"),
        "mcp_enabled": mcp_posture_report.get("mcp_enabled"),
        "required_setting": mcp_posture_report.get("required_setting"),
    },
    "provider_certification": {
        "ok": provider_report.get("ok"),
        "expected_providers": ["postgres", "mongo", "mssql", "snowflake"],
        "providers": provider_summaries,
    },
    "ops_certification": {
        "ok": ops_certification_report.get("ok"),
        "release_ready": ops_certification_report.get("release_ready"),
        "live_evidence_required": ops_certification_report.get("live_evidence_required"),
        "promtool_required": ops_certification_report.get("promtool_required"),
        "failure_count": ops_certification_report.get("failure_count"),
    },
    "pds_security_baseline": {
        "ok": pds_baseline_report.get("ok"),
        "release_ready": pds_baseline_report.get("release_ready"),
        "live_evidence_required": pds_baseline_report.get("live_evidence_required"),
        "failure_count": pds_baseline_report.get("failure_count"),
        "baseline": pds_baseline_report.get("baseline"),
        "decision_summary": pds_baseline_report.get("decision_summary"),
        "remediation_work_items": pds_baseline_report.get("remediation_work_items"),
    },
    "release_identity": {
        "ok": release_identity_report.get("ok"),
        "release_ready": release_identity_report.get("release_ready"),
        "release_identity_required": release_identity_report.get("release_identity_required"),
        "git": release_identity_report.get("git"),
        "decision_artifact": release_identity_report.get("decision_artifact"),
        "license_files": release_identity_report.get("license_files"),
        "remediation_work_items": release_identity_report.get("remediation_work_items"),
    },
    "security_gates": {
        "ok": True,
        "checks": security_gate_summaries,
        "gitleaks_findings": len(gitleaks_report),
    },
    "security_assurance_decision": {
        "ok": security_reports["security-assurance-decision"].get("ok"),
        "release_ready": security_reports["security-assurance-decision"].get("release_ready"),
        "requirements": security_reports["security-assurance-decision"].get("requirements"),
        "risk_acceptance": security_reports["security-assurance-decision"].get("risk_acceptance"),
        "categories": security_reports["security-assurance-decision"].get("categories"),
    },
    "deployable_image_sbom": {
        "required": image_context["required"],
        "applicability": image_context["applicability"],
        "detail": image_context["detail"],
        "env_sources": image_context["env_sources"],
        "retained_artifacts": image_context["retained_artifacts"],
        "required_by_release_mode": image_context["required_by_release_mode"],
        "supply_required": image_context["supply_required"],
        "manifest_required": image_context["manifest_required"],
        "artifact": image_sbom_path.as_posix(),
        "status": "present"
        if image_sbom_path.is_file()
        else ("missing" if image_context["required"] else "not-applicable"),
    },
}

evidence_file.write_text(
    json.dumps(evidence, indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
)
PY

  log "Wrote Bitbucket release evidence to $evidence_file"
}

assert_release_gate_ready() {
  local evidence_file="$report_dir/bitbucket-release-gate.json"

  python3 - "$evidence_file" <<'PY'
import datetime as dt
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

MAX_EVIDENCE_AGE = dt.timedelta(hours=24)
FUTURE_SKEW = dt.timedelta(minutes=5)

evidence_file = Path(sys.argv[1])
report_dir = evidence_file.parent.resolve(strict=False)
repo_root = Path.cwd().resolve(strict=False)
try:
    evidence = json.loads(evidence_file.read_text(encoding="utf-8"))
except Exception as exc:
    print(f"Bitbucket release evidence could not be parsed: {exc}", file=sys.stderr)
    sys.exit(1)

def parse_timestamp_with_timezone(value):
    if not isinstance(value, str) or not value.strip():
        return None
    try:
        parsed = dt.datetime.fromisoformat(value.strip().replace("Z", "+00:00"))
    except ValueError:
        return None
    if parsed.tzinfo is None:
        return None
    return parsed.astimezone(dt.timezone.utc)

def parse_date(value):
    if not isinstance(value, str) or not value.strip():
        return None
    text = value.strip().replace("Z", "+00:00")
    try:
        return dt.datetime.fromisoformat(text).date()
    except ValueError:
        try:
            return dt.date.fromisoformat(text[:10])
        except ValueError:
            return None

def non_empty_string(value):
    return isinstance(value, str) and bool(value.strip())

def first_non_empty(mapping, keys):
    if not isinstance(mapping, dict):
        return ""
    for key in keys:
        value = mapping.get(key)
        if non_empty_string(value):
            return value.strip()
    return ""

def discover_tracked_files():
    try:
        result = subprocess.run(
            ["git", "ls-files"],
            text=True,
            capture_output=True,
            check=True,
        )
    except Exception:
        return []
    return [line for line in result.stdout.splitlines() if line]

def frontend_lockfiles():
    return [
        path
        for path in discover_tracked_files()
        if re.search(r"(^|/)(package-lock\.json|pnpm-lock\.yaml|yarn\.lock)$", path)
        and "/node_modules/" not in f"/{path}"
    ]

def split_sources(value):
    return [
        source.strip()
        for source in re.split(r"[\n,]+", value or "")
        if source.strip()
    ]

def env_image_sources():
    sources = []
    for key in (
        "APPFW_SBOM_IMAGE_SOURCE",
        "APPFW_SBOM_IMAGE_SOURCES",
        "APPFW_SBOM_IMAGE_REF",
        "APPFW_SBOM_IMAGE_TAR",
        "APPFW_SBOM_IMAGE_ARTIFACT",
        "APPFW_DEPLOYABLE_IMAGE",
        "APPFW_IMAGE_REF",
        "APPFW_IMAGE_TAR",
        "APPFW_IMAGE_ARTIFACT",
    ):
        sources.extend(split_sources(os.environ.get(key, "")))
    return sources

def retained_image_artifacts():
    paths = []
    for pattern in (
        "deployable-image*.tar",
        "*.image.tar",
        "*.oci",
        "*.oci.tar",
        "images/*.tar",
        "images/*.tar.gz",
        "containers/*.tar",
        "containers/*.tar.gz",
    ):
        paths.extend(report_dir.glob(pattern))
    return [path for path in paths if path.is_file()]

def sorted_unique_strings(values):
    if not isinstance(values, list):
        return None
    normalized = []
    for value in values:
        if isinstance(value, str) and value.strip():
            normalized.append(value.strip())
        else:
            return None
    return sorted(dict.fromkeys(normalized))

def require_mapping(evidence, name):
    value = evidence.get(name)
    if not isinstance(value, dict):
        print(f"Bitbucket release evidence is missing {name} summary", file=sys.stderr)
        sys.exit(1)
    return value

def require_true(mapping, name, key):
    if mapping.get(key) is not True:
        print(
            f"Bitbucket release evidence {name}.{key} is not true",
            file=sys.stderr,
        )
        print(f"  actual={mapping.get(key)!r}", file=sys.stderr)
        sys.exit(1)

def resolve_retained_artifact(path_text):
    if not isinstance(path_text, str) or not path_text.strip():
        return None
    candidate = Path(path_text.strip()).expanduser()
    candidates = [candidate] if candidate.is_absolute() else [Path.cwd() / candidate, report_dir / candidate]
    for path in candidates:
        if path.is_file():
            return path.resolve(strict=False)
    return None

def path_is_in_report_dir(path):
    try:
        path.resolve(strict=False).relative_to(report_dir)
    except ValueError:
        return False
    return True

def sha256_digest(path):
    return "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest()

def normalize_sha256(value):
    if not isinstance(value, str) or not value.strip():
        return None
    digest = value.strip().lower()
    if digest.startswith("sha256:"):
        digest = digest.removeprefix("sha256:")
    if len(digest) != 64 or any(char not in "0123456789abcdef" for char in digest):
        return None
    return "sha256:" + digest

def validate_retained_file_artifact(
    artifact,
    label,
    issues,
    require_required=False,
    require_ok=False,
    require_present=True,
    require_report_dir=True,
):
    if not isinstance(artifact, dict):
        issues.append(f"{label}: artifact entry must be an object")
        return None
    resolved_artifact = resolve_retained_artifact(artifact.get("path"))
    if require_required and artifact.get("required") is not True:
        issues.append(f"{label}: required is not true, actual {artifact.get('required')!r}")
    if require_present and artifact.get("present") is not True:
        issues.append(f"{label}: present is not true, actual {artifact.get('present')!r}")
    if require_ok and artifact.get("ok") is not True:
        issues.append(f"{label}: ok is not true, actual {artifact.get('ok')!r}")
    if resolved_artifact is None:
        issues.append(f"{label}: retained file is missing, path={artifact.get('path')!r}")
    elif require_report_dir and not path_is_in_report_dir(resolved_artifact):
        issues.append(f"{label}: retained file is outside report directory: {resolved_artifact.as_posix()}")
    expected_sha = normalize_sha256(artifact.get("sha256"))
    if expected_sha is None:
        issues.append(f"{label}: sha256 is missing or invalid, actual {artifact.get('sha256')!r}")
    elif resolved_artifact is not None:
        actual_sha = sha256_digest(resolved_artifact)
        if actual_sha != expected_sha:
            issues.append(f"{label}: sha256 mismatch, expected {expected_sha}, actual {actual_sha}")
    expected_bytes = artifact.get("bytes")
    if not is_int(expected_bytes) or expected_bytes <= 0:
        issues.append(f"{label}: byte count is invalid, actual {expected_bytes!r}")
    elif resolved_artifact is not None:
        actual_bytes = resolved_artifact.stat().st_size
        if actual_bytes != expected_bytes:
            issues.append(f"{label}: byte count mismatch, expected {expected_bytes!r}, actual {actual_bytes}")
    return resolved_artifact

def load_retained_json(label, path):
    if not path.is_file():
        print(f"Bitbucket release evidence retained {label} artifact is missing", file=sys.stderr)
        print(f"  path={path.as_posix()}", file=sys.stderr)
        sys.exit(1)
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except Exception as exc:
        print(f"Bitbucket release evidence retained {label} artifact is invalid JSON", file=sys.stderr)
        print(f"  path={path.as_posix()}", file=sys.stderr)
        print(f"  error={exc}", file=sys.stderr)
        sys.exit(1)
    if not isinstance(value, dict):
        print(f"Bitbucket release evidence retained {label} artifact is not an object", file=sys.stderr)
        print(f"  path={path.as_posix()}", file=sys.stderr)
        sys.exit(1)
    return value

def load_retained_array(label, path):
    if not path.is_file():
        print(f"Bitbucket release evidence retained {label} artifact is missing", file=sys.stderr)
        print(f"  path={path.as_posix()}", file=sys.stderr)
        sys.exit(1)
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except Exception as exc:
        print(f"Bitbucket release evidence retained {label} artifact is invalid JSON", file=sys.stderr)
        print(f"  path={path.as_posix()}", file=sys.stderr)
        print(f"  error={exc}", file=sys.stderr)
        sys.exit(1)
    if not isinstance(value, list):
        print(f"Bitbucket release evidence retained {label} artifact is not an array", file=sys.stderr)
        print(f"  path={path.as_posix()}", file=sys.stderr)
        sys.exit(1)
    return value

def require_child_command(report, label, expected):
    if report.get("command") != expected:
        print(f"Bitbucket release evidence retained {label} artifact has unexpected command", file=sys.stderr)
        print(f"  expected={expected!r}", file=sys.stderr)
        print(f"  actual={report.get('command')!r}", file=sys.stderr)
        sys.exit(1)

def require_child_true(report, label, key):
    if report.get(key) is not True:
        print(f"Bitbucket release evidence retained {label} artifact {key} is not true", file=sys.stderr)
        print(f"  actual={report.get(key)!r}", file=sys.stderr)
        sys.exit(1)

def is_int(value):
    return type(value) is int

def is_number(value):
    return type(value) in {int, float}

def require_child_timestamp(report, label, latest_allowed):
    generated_at = parse_timestamp_with_timezone(report.get("generated_at_utc"))
    if generated_at is None:
        print(f"Bitbucket release evidence retained {label} artifact generated_at_utc is missing or invalid", file=sys.stderr)
        print(f"  actual={report.get('generated_at_utc')!r}", file=sys.stderr)
        sys.exit(1)
    if generated_at > now + FUTURE_SKEW:
        print(f"Bitbucket release evidence retained {label} artifact generated_at_utc is in the future", file=sys.stderr)
        print(f"  generated_at_utc={generated_at.isoformat().replace('+00:00', 'Z')}", file=sys.stderr)
        sys.exit(1)
    if generated_at < now - MAX_EVIDENCE_AGE:
        print(f"Bitbucket release evidence retained {label} artifact generated_at_utc is stale", file=sys.stderr)
        print(f"  generated_at_utc={generated_at.isoformat().replace('+00:00', 'Z')}", file=sys.stderr)
        sys.exit(1)
    if latest_allowed is not None and generated_at > latest_allowed + FUTURE_SKEW:
        print(f"Bitbucket release evidence retained {label} artifact postdates the ready wrapper", file=sys.stderr)
        print(f"  child={generated_at.isoformat().replace('+00:00', 'Z')}", file=sys.stderr)
        print(f"  wrapper={latest_allowed.isoformat().replace('+00:00', 'Z')}", file=sys.stderr)
        sys.exit(1)

def collect_green_check_entries(report, label):
    checks = report.get("checks")
    if not isinstance(checks, list) or not checks:
        print(f"Bitbucket release evidence retained {label} checks are missing", file=sys.stderr)
        sys.exit(1)
    entries = [
        check for check in checks if isinstance(check, dict)
    ]
    if len(entries) != len(checks):
        print(f"Bitbucket release evidence retained {label} check entries must be objects", file=sys.stderr)
        sys.exit(1)
    names = []
    invalid_names = []
    for index, check in enumerate(entries):
        check_name = check.get("name")
        if isinstance(check_name, str) and check_name.strip():
            names.append(check_name.strip())
        else:
            invalid_names.append(index)
    if invalid_names:
        print(f"Bitbucket release evidence retained {label} check names are invalid", file=sys.stderr)
        print(f"  indexes={invalid_names}", file=sys.stderr)
        sys.exit(1)
    duplicates = sorted(
        check_name
        for check_name in set(names)
        if names.count(check_name) > 1
    )
    if duplicates:
        print(f"Bitbucket release evidence retained {label} check entries are duplicated", file=sys.stderr)
        for check_name in duplicates:
            print(f"  - {check_name}", file=sys.stderr)
        sys.exit(1)
    failing = [
        check.get("name", f"#{index}")
        for index, check in enumerate(entries)
        if check.get("ok") is not True
    ]
    if failing:
        print(f"Bitbucket release evidence retained {label} checks are not green", file=sys.stderr)
        for check_name in failing:
            print(f"  - {check_name}", file=sys.stderr)
        sys.exit(1)
    return {
        name: entry
        for name, entry in zip(names, entries)
    }

def collect_artifact_entries(report, label):
    artifacts = report.get("artifacts")
    if not isinstance(artifacts, list) or not artifacts:
        print(f"Bitbucket release evidence retained {label} artifacts are missing", file=sys.stderr)
        sys.exit(1)
    entries = [
        artifact for artifact in artifacts if isinstance(artifact, dict)
    ]
    if len(entries) != len(artifacts):
        print(f"Bitbucket release evidence retained {label} artifact entries must be objects", file=sys.stderr)
        sys.exit(1)
    return entries

def collect_artifact_entries_by_name(report, label):
    entries = collect_artifact_entries(report, label)
    names = []
    invalid_names = []
    for index, artifact in enumerate(entries):
        artifact_name = artifact.get("name")
        if isinstance(artifact_name, str) and artifact_name.strip():
            names.append(artifact_name.strip())
        else:
            invalid_names.append(index)
    if invalid_names:
        print(f"Bitbucket release evidence retained {label} artifact names are invalid", file=sys.stderr)
        print(f"  indexes={invalid_names}", file=sys.stderr)
        sys.exit(1)
    duplicates = sorted(
        artifact_name
        for artifact_name in set(names)
        if names.count(artifact_name) > 1
    )
    if duplicates:
        print(f"Bitbucket release evidence retained {label} artifact names are duplicated", file=sys.stderr)
        for artifact_name in duplicates:
            print(f"  - {artifact_name}", file=sys.stderr)
        sys.exit(1)
    return {
        name: entry
        for name, entry in zip(names, entries)
    }

def require_named_artifact(report, label, name, expected_path, require_required=True, require_hash=False):
    artifacts_by_name = collect_artifact_entries_by_name(report, label)
    artifact = artifacts_by_name.get(name)
    if not isinstance(artifact, dict):
        print(f"Bitbucket release evidence retained {label} artifact {name} is missing", file=sys.stderr)
        sys.exit(1)
    if require_required and artifact.get("required") is not True:
        print(f"Bitbucket release evidence retained {label} artifact {name} is not required", file=sys.stderr)
        print(f"  actual={artifact.get('required')!r}", file=sys.stderr)
        sys.exit(1)
    if artifact.get("present") is not True:
        print(f"Bitbucket release evidence retained {label} artifact {name} is not present", file=sys.stderr)
        print(f"  actual={artifact.get('present')!r}", file=sys.stderr)
        sys.exit(1)
    resolved = resolve_retained_artifact(artifact.get("path"))
    expected = expected_path.resolve(strict=False)
    if resolved is None or resolved != expected:
        print(f"Bitbucket release evidence retained {label} artifact {name} points at unexpected path", file=sys.stderr)
        print(f"  expected={expected.as_posix()}", file=sys.stderr)
        print(f"  actual={artifact.get('path')!r}", file=sys.stderr)
        sys.exit(1)
    if not path_is_in_report_dir(resolved):
        print(f"Bitbucket release evidence retained {label} artifact {name} is outside report directory", file=sys.stderr)
        print(f"  actual={resolved.as_posix()}", file=sys.stderr)
        sys.exit(1)
    if require_hash:
        artifact_issues = []
        validate_retained_file_artifact(
            artifact,
            f"{label} {name} artifact",
            artifact_issues,
            require_required=require_required,
        )
        if artifact_issues:
            print(f"Bitbucket release evidence retained {label} artifact {name} hash evidence is incomplete", file=sys.stderr)
            for issue in artifact_issues:
                print(f"  - {issue}", file=sys.stderr)
            sys.exit(1)
    return artifact

def require_clean_child_evidence(report, label):
    collect_green_check_entries(report, label)
    if not is_int(report.get("failure_count")) or report.get("failure_count") != 0:
        print(f"Bitbucket release evidence retained {label} failure_count is not zero", file=sys.stderr)
        print(f"  actual={report.get('failure_count')!r}", file=sys.stderr)
        sys.exit(1)
    if report.get("failures") != []:
        print(f"Bitbucket release evidence retained {label} failures are not empty", file=sys.stderr)
        print(f"  actual={report.get('failures')!r}", file=sys.stderr)
        sys.exit(1)
    if report.get("release_blockers") != []:
        print(f"Bitbucket release evidence retained {label} release_blockers are not empty", file=sys.stderr)
        print(f"  actual={report.get('release_blockers')!r}", file=sys.stderr)
        sys.exit(1)
    failure_summary = report.get("failure_summary")
    if isinstance(failure_summary, dict) and failure_summary.get("root_causes") not in (None, []):
        print(f"Bitbucket release evidence retained {label} failure_summary.root_causes are not empty", file=sys.stderr)
        print(f"  actual={failure_summary.get('root_causes')!r}", file=sys.stderr)
        sys.exit(1)

def require_static_security_evidence(gitleaks_findings):
    retained_phi = load_retained_json("PHI log lint", report_dir / "phi-log-lint.json")
    require_child_command(retained_phi, "PHI log lint", "phi-log-lint")
    require_child_timestamp(retained_phi, "PHI log lint", generated_at)
    require_child_true(retained_phi, "PHI log lint", "ok")
    phi_issues = []
    if not is_int(retained_phi.get("scanned_files")) or retained_phi.get("scanned_files") <= 0:
        phi_issues.append(f"scanned_files must be positive, actual {retained_phi.get('scanned_files')!r}")
    if retained_phi.get("findings") != 0:
        phi_issues.append(f"findings must be 0, actual {retained_phi.get('findings')!r}")
    phi_redaction = retained_phi.get("redaction")
    if not isinstance(phi_redaction, dict):
        phi_issues.append("redaction block is missing")
    elif (
        phi_redaction.get("json_source_excerpt_retained") is not False
        or phi_redaction.get("json_finding_values_retained") is not False
    ):
        phi_issues.append(f"redaction flags are unsafe: {phi_redaction!r}")
    if phi_issues:
        print("Bitbucket release evidence retained PHI log lint evidence is incomplete", file=sys.stderr)
        for issue in phi_issues:
            print(f"  - {issue}", file=sys.stderr)
        sys.exit(1)

    retained_supply_chain = load_retained_json("supply-chain gate", report_dir / "supply-chain-gate.json")
    require_child_command(retained_supply_chain, "supply-chain gate", "supply-chain-gate")
    require_child_timestamp(retained_supply_chain, "supply-chain gate", generated_at)
    require_child_true(retained_supply_chain, "supply-chain gate", "ok")
    supply_checks = collect_green_check_entries(retained_supply_chain, "supply-chain gate")
    supply_check_issues = [
        f"{name}: exit_code={entry.get('exit_code')!r}"
        for name, entry in supply_checks.items()
        if not is_int(entry.get("exit_code")) or entry.get("exit_code") != 0
    ]
    if supply_check_issues:
        print("Bitbucket release evidence retained supply-chain gate checks are incomplete", file=sys.stderr)
        for issue in supply_check_issues:
            print(f"  - {issue}", file=sys.stderr)
        sys.exit(1)
    require_named_artifact(
        retained_supply_chain,
        "supply-chain gate",
        "dependency-check",
        report_dir / "dependency-check.json",
        require_hash=True,
    )
    require_named_artifact(
        retained_supply_chain,
        "supply-chain gate",
        "phi-log-lint",
        report_dir / "phi-log-lint.json",
        require_hash=True,
    )
    require_named_artifact(
        retained_supply_chain,
        "supply-chain gate",
        "sbom-manifest",
        report_dir / "sbom-manifest.json",
        require_hash=True,
    )
    require_named_artifact(
        retained_supply_chain,
        "supply-chain gate",
        "rust-cyclonedx-sbom",
        report_dir / "sbom-rust-workspace.cdx.json",
        require_hash=True,
    )
    require_named_artifact(
        retained_supply_chain,
        "supply-chain gate",
        "frontend-cyclonedx-sbom",
        report_dir / "sbom-frontend-packages.cdx.json",
        require_hash=True,
    )

    retained_dependency = load_retained_json("dependency check", report_dir / "dependency-check.json")
    require_child_command(retained_dependency, "dependency check", "dependency-check")
    require_child_timestamp(retained_dependency, "dependency check", generated_at)
    require_child_true(retained_dependency, "dependency check", "ok")
    dependency_summary = retained_dependency.get("summary")
    dependency_osv = retained_dependency.get("osv")
    dependency_issues = []
    if not isinstance(dependency_summary, dict) or not is_int(dependency_summary.get("accepted_osv_finding_count")):
        dependency_issues.append("summary.accepted_osv_finding_count must be recorded")
    if not isinstance(retained_dependency.get("policy"), dict):
        dependency_issues.append("policy block must be recorded")
    if not isinstance(dependency_osv, dict):
        dependency_issues.append("osv block must be recorded")
    elif not isinstance(dependency_osv.get("accepted_findings"), list) or not isinstance(dependency_osv.get("expired_acceptances"), list):
        dependency_issues.append("osv accepted_findings and expired_acceptances must be arrays")
    dependency_checks = retained_dependency.get("checks")
    if not isinstance(dependency_checks, list) or not dependency_checks:
        dependency_issues.append("executed checks must be recorded")
    else:
        dependency_check_names = []
        required_check_seen = False
        for index, check in enumerate(dependency_checks):
            if not isinstance(check, dict):
                dependency_issues.append(f"check #{index} must be an object")
                continue
            check_name = check.get("name")
            if isinstance(check_name, str) and check_name.strip():
                dependency_check_names.append(check_name.strip())
            else:
                dependency_issues.append(f"check #{index} must record a name")
                check_name = f"#{index}"
            required = check.get("required")
            if required is not False:
                required_check_seen = True
                if check.get("ok") is not True:
                    dependency_issues.append(f"required check {check_name!r} must be ok:true")
                if check.get("exit_code") is not None and check.get("exit_code") != 0:
                    dependency_issues.append(
                        f"required check {check_name!r} must exit 0, actual {check.get('exit_code')!r}"
                    )
                if check.get("status") is not None and check.get("status") != "passed":
                    dependency_issues.append(
                        f"required check {check_name!r} must have status 'passed', actual {check.get('status')!r}"
                    )
        duplicate_check_names = sorted(
            name for name in set(dependency_check_names) if dependency_check_names.count(name) > 1
        )
        if duplicate_check_names:
            dependency_issues.append(f"check names are duplicated: {duplicate_check_names}")
        if not required_check_seen:
            dependency_issues.append("at least one dependency-check child check must be required")
    if retained_dependency.get("blocking_findings") != []:
        dependency_issues.append(f"blocking_findings must be empty, actual {retained_dependency.get('blocking_findings')!r}")
    if dependency_issues:
        print("Bitbucket release evidence retained dependency check evidence is incomplete", file=sys.stderr)
        for issue in dependency_issues:
            print(f"  - {issue}", file=sys.stderr)
        sys.exit(1)

    retained_sbom_manifest = load_retained_json("SBOM manifest", report_dir / "sbom-manifest.json")
    require_child_command(retained_sbom_manifest, "SBOM manifest", "generate-sbom-evidence")
    require_child_timestamp(retained_sbom_manifest, "SBOM manifest", generated_at)
    require_child_true(retained_sbom_manifest, "SBOM manifest", "ok")
    supply_artifacts = collect_artifact_entries_by_name(retained_supply_chain, "supply-chain gate")
    supply_image_artifact = supply_artifacts.get("deployable-image-cyclonedx-sbom")
    sbom_artifacts = collect_artifact_entries_by_name(retained_sbom_manifest, "SBOM manifest")
    manifest_image_artifact = sbom_artifacts.get("deployable-image-cyclonedx-sbom")
    image_source_env_keys = (
        "APPFW_SBOM_IMAGE_SOURCE",
        "APPFW_SBOM_IMAGE_SOURCES",
        "APPFW_SBOM_IMAGE_REF",
        "APPFW_SBOM_IMAGE_TAR",
        "APPFW_SBOM_IMAGE_ARTIFACT",
        "APPFW_DEPLOYABLE_IMAGE",
        "APPFW_IMAGE_REF",
        "APPFW_IMAGE_TAR",
        "APPFW_IMAGE_ARTIFACT",
    )
    image_source_env_present = any(bool(os.environ.get(key)) for key in image_source_env_keys)
    image_release_required = str(os.environ.get("APPFW_REQUIRE_DEPLOYABLE_IMAGE_SBOM") or "").strip().lower() in {
        "1",
        "true",
        "yes",
        "y",
        "on",
        "required",
        "require",
    } or str(os.environ.get("APPFW_RELEASE_REQUIRE_IMAGE_SBOM") or "").strip().lower() in {
        "1",
        "true",
        "yes",
        "y",
        "on",
        "required",
        "require",
    }
    retained_image_artifact_present = any(
        path.is_file()
        for pattern in (
            "deployable-image*.tar",
            "*.image.tar",
            "*.oci",
            "*.oci.tar",
            "images/*.tar",
            "images/*.tar.gz",
            "containers/*.tar",
            "containers/*.tar.gz",
        )
        for path in report_dir.glob(pattern)
    )
    image_required = (
        image_release_required
        or image_source_env_present
        or retained_image_artifact_present
        or (
            isinstance(supply_image_artifact, dict)
            and supply_image_artifact.get("required") is True
        )
        or (
            isinstance(manifest_image_artifact, dict)
            and manifest_image_artifact.get("required") is True
        )
    )
    supply_image_required = (
        image_required
        or (
            isinstance(supply_image_artifact, dict)
            and (supply_image_artifact.get("required") is True or supply_image_artifact.get("present") is True)
        )
    )
    if supply_image_required:
        require_named_artifact(
            retained_supply_chain,
            "supply-chain gate",
            "deployable-image-cyclonedx-sbom",
            report_dir / "sbom-deployable-image.cdx.json",
            require_required=image_required,
            require_hash=True,
        )
    sbom_issues = []
    expected_frontend_sources = sorted(frontend_lockfiles())
    expected_image_sources = sorted(
        dict.fromkeys(
            env_image_sources()
            + [path.as_posix() for path in retained_image_artifacts()]
        )
    )
    manifest_inputs = retained_sbom_manifest.get("inputs")
    if not isinstance(manifest_inputs, dict):
        sbom_issues.append(f"inputs must be recorded, actual {manifest_inputs!r}")
    else:
        actual_frontend_inputs = sorted_unique_strings(manifest_inputs.get("frontend_lockfiles"))
        if actual_frontend_inputs != expected_frontend_sources:
            sbom_issues.append(
                "frontend_lockfiles inputs do not match tracked package locks: "
                f"expected={expected_frontend_sources}, actual={actual_frontend_inputs}"
            )
        actual_image_inputs = sorted_unique_strings(manifest_inputs.get("image_sources"))
        if actual_image_inputs != expected_image_sources:
            sbom_issues.append(
                "image_sources inputs do not match deployable image sources: "
                f"expected={expected_image_sources}, actual={actual_image_inputs}"
            )
        if manifest_inputs.get("image_required_by_release_mode") is not image_release_required:
            sbom_issues.append(
                "image_required_by_release_mode does not match release policy: "
                f"expected={image_release_required!r}, "
                f"actual={manifest_inputs.get('image_required_by_release_mode')!r}"
            )
    expected_sbom_artifacts = [
        ("rust-cyclonedx-sbom", report_dir / "sbom-rust-workspace.cdx.json", True),
        ("frontend-cyclonedx-sbom", report_dir / "sbom-frontend-packages.cdx.json", True),
        ("deployable-image-cyclonedx-sbom", report_dir / "sbom-deployable-image.cdx.json", False),
    ]
    for artifact_name, expected_path, required_by_default in expected_sbom_artifacts:
        artifact = sbom_artifacts.get(artifact_name)
        artifact_required = (
            required_by_default
            or (
                isinstance(artifact, dict)
                and (artifact.get("required") is True or artifact.get("present") is True)
            )
        )
        if not artifact_required:
            continue
        if not isinstance(artifact, dict):
            sbom_issues.append(f"{artifact_name}: missing")
            continue
        resolved_artifact = validate_retained_file_artifact(
            artifact,
            f"SBOM manifest {artifact_name} artifact",
            sbom_issues,
            require_required=required_by_default,
        )
        if resolved_artifact != expected_path.resolve(strict=False):
            sbom_issues.append(f"{artifact_name}: path={artifact.get('path')!r}")
        if artifact_name == "rust-cyclonedx-sbom":
            expected_sources = ["Cargo.lock", "cargo metadata --locked --no-deps"]
        elif artifact_name == "frontend-cyclonedx-sbom":
            expected_sources = expected_frontend_sources
        else:
            expected_sources = expected_image_sources
        actual_sources = sorted_unique_strings(artifact.get("sources"))
        expected_sources = sorted_unique_strings(list(expected_sources))
        if actual_sources != expected_sources:
            sbom_issues.append(
                f"{artifact_name}: sources do not match release inputs; "
                f"expected={expected_sources}, actual={actual_sources}"
            )
        if resolved_artifact is not None:
            try:
                retained_bom = json.loads(resolved_artifact.read_text(encoding="utf-8"))
            except Exception as exc:
                sbom_issues.append(f"{artifact_name}: retained SBOM is invalid JSON: {exc}")
            else:
                if not isinstance(retained_bom, dict):
                    sbom_issues.append(f"{artifact_name}: retained SBOM must be a JSON object")
                else:
                    if retained_bom.get("bomFormat") != "CycloneDX":
                        sbom_issues.append(
                            f"{artifact_name}: bomFormat={retained_bom.get('bomFormat')!r}"
                        )
                    bom_spec_version = retained_bom.get("specVersion")
                    if (
                        not isinstance(artifact.get("spec_version"), str)
                        or artifact.get("spec_version") != bom_spec_version
                    ):
                        sbom_issues.append(
                            f"{artifact_name}: spec_version={artifact.get('spec_version')!r} "
                            f"does not match retained SBOM {bom_spec_version!r}"
                        )
                    components = retained_bom.get("components")
                    if not isinstance(components, list):
                        sbom_issues.append(f"{artifact_name}: retained SBOM components must be an array")
                    elif artifact.get("component_count") != len(components):
                        sbom_issues.append(
                            f"{artifact_name}: component_count={artifact.get('component_count')!r} "
                            f"does not match retained SBOM {len(components)}"
                        )
    if sbom_issues:
        print("Bitbucket release evidence retained SBOM manifest evidence is incomplete", file=sys.stderr)
        for issue in sbom_issues:
            print(f"  - {issue}", file=sys.stderr)
        sys.exit(1)

    retained_secret_scan = load_retained_json("secret scan", report_dir / "secret-scan.json")
    require_child_command(retained_secret_scan, "secret scan", "secret-scan")
    require_child_timestamp(retained_secret_scan, "secret scan", generated_at)
    require_child_true(retained_secret_scan, "secret scan", "ok")
    require_named_artifact(
        retained_secret_scan,
        "secret scan",
        "gitleaks-report",
        report_dir / "gitleaks-report.json",
    )
    retained_gitleaks = load_retained_array("gitleaks report", report_dir / "gitleaks-report.json")
    secret_issues = []
    if retained_secret_scan.get("scanner") != "gitleaks":
        secret_issues.append(f"scanner must be gitleaks, actual {retained_secret_scan.get('scanner')!r}")
    for field in ("scanner_version", "requested_scanner_version"):
        if not isinstance(retained_secret_scan.get(field), str) or not retained_secret_scan.get(field).strip():
            secret_issues.append(f"{field} must be recorded")
    if retained_secret_scan.get("history_scan") is not True:
        secret_issues.append(f"history_scan must be true, actual {retained_secret_scan.get('history_scan')!r}")
    if retained_secret_scan.get("gitleaks_exit_code") not in (0, 1):
        secret_issues.append(f"gitleaks_exit_code must be 0 or 1, actual {retained_secret_scan.get('gitleaks_exit_code')!r}")
    if retained_secret_scan.get("total_findings") != len(retained_gitleaks):
        secret_issues.append(
            f"total_findings must match retained gitleaks report, actual {retained_secret_scan.get('total_findings')!r} vs {len(retained_gitleaks)}"
        )
    if retained_secret_scan.get("findings") != 0:
        secret_issues.append(f"unbaselined findings must be 0, actual {retained_secret_scan.get('findings')!r}")
    if retained_secret_scan.get("baselined_findings") != len(retained_gitleaks):
        secret_issues.append(
            f"baselined_findings must match retained gitleaks report, actual {retained_secret_scan.get('baselined_findings')!r} vs {len(retained_gitleaks)}"
        )
    if gitleaks_findings != len(retained_gitleaks):
        secret_issues.append(
            f"wrapper gitleaks_findings must match retained gitleaks report, actual {gitleaks_findings!r} vs {len(retained_gitleaks)}"
        )
    baseline = retained_secret_scan.get("baseline")
    if not isinstance(baseline, dict) or baseline.get("unbaselined_findings") != 0:
        secret_issues.append(f"baseline.unbaselined_findings must be 0, actual {baseline!r}")
    secret_redaction = retained_secret_scan.get("redaction")
    if not isinstance(secret_redaction, dict):
        secret_issues.append("redaction block is missing")
    elif (
        secret_redaction.get("gitleaks_redact_flag") is not True
        or secret_redaction.get("secret_fields_checked") is not True
        or secret_redaction.get("secret_fields_redacted") is not True
    ):
        secret_issues.append(f"redaction flags are unsafe: {secret_redaction!r}")
    gitleaks_fingerprints = []
    unredacted_secret_indexes = []
    for index, finding in enumerate(retained_gitleaks):
        if not isinstance(finding, dict):
            secret_issues.append(f"gitleaks finding #{index} is not an object")
            continue
        fingerprint = finding.get("Fingerprint")
        if isinstance(fingerprint, str) and fingerprint.strip():
            gitleaks_fingerprints.append(fingerprint.strip())
        else:
            secret_issues.append(f"gitleaks finding #{index} is missing Fingerprint")
        if "Secret" in finding and finding.get("Secret") != "REDACTED":
            unredacted_secret_indexes.append(index)
    duplicate_fingerprints = sorted(
        fingerprint
        for fingerprint in set(gitleaks_fingerprints)
        if gitleaks_fingerprints.count(fingerprint) > 1
    )
    if duplicate_fingerprints:
        secret_issues.append(f"gitleaks fingerprints are duplicated: {duplicate_fingerprints}")
    if unredacted_secret_indexes:
        secret_issues.append(f"gitleaks Secret fields are not redacted at indexes {unredacted_secret_indexes}")
    if secret_issues:
        print("Bitbucket release evidence retained secret-scan evidence is incomplete", file=sys.stderr)
        for issue in secret_issues:
            print(f"  - {issue}", file=sys.stderr)
        sys.exit(1)

def require_security_assurance_decision_evidence():
    retained_decision = load_retained_json(
        "security assurance decision",
        report_dir / "security-assurance-decision.json",
    )
    require_child_command(retained_decision, "security assurance decision", "security-assurance-decision")
    require_child_timestamp(retained_decision, "security assurance decision", generated_at)
    require_child_true(retained_decision, "security assurance decision", "ok")
    require_child_true(retained_decision, "security assurance decision", "release_ready")
    require_child_true(
        retained_decision,
        "security assurance decision",
        "production_attestations_required",
    )
    decision_issues = []
    if retained_decision.get("failure_count") != 0:
        decision_issues.append(f"failure_count must be 0, actual {retained_decision.get('failure_count')!r}")
    if retained_decision.get("failures") != []:
        decision_issues.append(f"failures must be empty, actual {retained_decision.get('failures')!r}")
    if retained_decision.get("rejected_artifacts") != []:
        decision_issues.append(
            f"rejected_artifacts must be empty, actual {retained_decision.get('rejected_artifacts')!r}"
        )
    requirements = retained_decision.get("requirements")
    required_categories = {"dast", "sast", "asvs", "release-provenance", "artifact-signing"}
    requirement_names = {
        str(requirement).strip().lower().replace("_", "-")
        for requirement in requirements
        if isinstance(requirement, str)
    } if isinstance(requirements, list) else set()
    missing_requirements = sorted(required_categories - requirement_names)
    if missing_requirements:
        decision_issues.append(f"requirements missing {missing_requirements}")
    categories = retained_decision.get("categories")
    if not isinstance(categories, list) or not categories:
        decision_issues.append("categories must be recorded")
    else:
        category_names = {
            str(category.get("category")).strip().lower().replace("_", "-")
            for category in categories
            if isinstance(category, dict) and isinstance(category.get("category"), str)
        }
        missing_categories = sorted(required_categories - category_names)
        if missing_categories:
            decision_issues.append(f"categories missing {missing_categories}")
        for index, category in enumerate(categories):
            category_name = category.get("category") if isinstance(category, dict) else f"#{index}"
            if not isinstance(category, dict):
                decision_issues.append(f"{category_name}: category entry must be an object")
                continue
            if category.get("ok") is not True:
                decision_issues.append(f"{category_name}: ok must be true")
            disposition = category.get("disposition")
            if disposition not in {"evidence", "risk-accepted"}:
                decision_issues.append(f"{category_name}: disposition {disposition!r} is not release-authoritative")
                continue
            if disposition == "evidence":
                evidence_artifacts = category.get("evidence_artifacts")
                if not isinstance(evidence_artifacts, list) or not evidence_artifacts:
                    decision_issues.append(f"{category_name}: evidence_artifacts must be recorded")
                    continue
                for artifact in evidence_artifacts:
                    validate_retained_file_artifact(
                        artifact,
                        f"{category_name}: artifact {artifact.get('path')!r}" if isinstance(artifact, dict) else f"{category_name}: artifact",
                        decision_issues,
                        require_ok=True,
                    )
            elif disposition == "risk-accepted":
                risk_acceptance = category.get("risk_acceptance")
                if not isinstance(risk_acceptance, dict):
                    decision_issues.append(f"{category_name}: risk_acceptance must be recorded")
                else:
                    owner = first_non_empty(risk_acceptance, ("owner", "risk_owner"))
                    approver = first_non_empty(risk_acceptance, ("approved_by", "approver"))
                    scope = first_non_empty(risk_acceptance, ("release_scope", "scope"))
                    expires_on = first_non_empty(risk_acceptance, ("expires_on", "expires_at"))
                    rationale = first_non_empty(risk_acceptance, ("rationale", "reason"))
                    follow_up = first_non_empty(risk_acceptance, ("follow_up", "remediation_plan"))
                    controls = risk_acceptance.get("compensating_controls")
                    if controls is None:
                        controls = risk_acceptance.get("controls")
                    controls_ok = (
                        non_empty_string(controls)
                        or (
                            isinstance(controls, list)
                            and bool(controls)
                            and all(non_empty_string(control) for control in controls)
                        )
                    )
                    missing_fields = []
                    if not owner:
                        missing_fields.append("owner")
                    if not approver:
                        missing_fields.append("approved_by")
                    if not scope:
                        missing_fields.append("release_scope")
                    if not expires_on:
                        missing_fields.append("expires_on")
                    if not rationale:
                        missing_fields.append("rationale")
                    if not controls_ok:
                        missing_fields.append("compensating_controls")
                    if not follow_up:
                        missing_fields.append("follow_up")
                    if missing_fields:
                        decision_issues.append(f"{category_name}: risk acceptance missing {missing_fields}")
                    if risk_acceptance.get("accepted") is not True and risk_acceptance.get("status") != "accepted":
                        decision_issues.append(f"{category_name}: risk acceptance is not explicitly accepted")
                    expiry_date = parse_date(expires_on)
                    if expiry_date is None:
                        decision_issues.append(f"{category_name}: risk acceptance expires_on is invalid")
                    else:
                        today = now.date()
                        risk_summary = retained_decision.get("risk_acceptance")
                        max_days = 90
                        if isinstance(risk_summary, dict) and is_int(risk_summary.get("max_days")):
                            max_days = max(1, risk_summary.get("max_days"))
                        if expiry_date < today:
                            decision_issues.append(f"{category_name}: risk acceptance expired on {expiry_date.isoformat()}")
                        if expiry_date > today + dt.timedelta(days=max_days):
                            decision_issues.append(
                                f"{category_name}: risk acceptance expires beyond {max_days} days"
                            )
    decision_artifacts = retained_decision.get("artifacts")
    if not isinstance(decision_artifacts, list) or not decision_artifacts:
        decision_issues.append("top-level artifacts must be recorded")
    else:
        for artifact in decision_artifacts:
            artifact_name = artifact.get("name") if isinstance(artifact, dict) else "<unknown>"
            validate_retained_file_artifact(
                artifact,
                artifact_name,
                decision_issues,
                require_required=artifact_name != "security-risk-acceptance",
            )
    if decision_issues:
        print("Bitbucket release evidence retained security-assurance decision is incomplete", file=sys.stderr)
        for issue in decision_issues:
            print(f"  - {issue}", file=sys.stderr)
        sys.exit(1)

def require_mcp_release_posture_evidence(summary):
    retained_mcp = load_retained_json("MCP release posture", report_dir / "release-mcp-posture.json")
    require_child_command(retained_mcp, "MCP release posture", "mcp-release-posture")
    require_child_timestamp(retained_mcp, "MCP release posture", generated_at)
    require_child_true(retained_mcp, "MCP release posture", "ok")
    mcp_issues = []
    if retained_mcp.get("release_posture") != "excluded":
        mcp_issues.append(f"release_posture must be excluded, actual {retained_mcp.get('release_posture')!r}")
    if str(retained_mcp.get("mcp_enabled")).strip().lower() not in {"false", "0", "no", "off", ""}:
        mcp_issues.append(f"mcp_enabled must be false-like, actual {retained_mcp.get('mcp_enabled')!r}")
    if retained_mcp.get("required_setting") != "APP_MCP_ENABLED=false":
        mcp_issues.append(f"required_setting is unexpected, actual {retained_mcp.get('required_setting')!r}")
    for key in ("release_posture", "mcp_enabled", "required_setting"):
        if retained_mcp.get(key) != summary.get(key):
            mcp_issues.append(
                f"{key} does not match wrapper summary: child={retained_mcp.get(key)!r}; wrapper={summary.get(key)!r}"
            )
    if mcp_issues:
        print("Bitbucket release evidence retained MCP release posture is incomplete", file=sys.stderr)
        for issue in mcp_issues:
            print(f"  - {issue}", file=sys.stderr)
        sys.exit(1)

if evidence.get("ok") is True and evidence.get("release_ready") is True:
    if evidence.get("command") != "bitbucket-release-gate":
        print(
            "Bitbucket release evidence command is not bitbucket-release-gate",
            file=sys.stderr,
        )
        sys.exit(1)
    if evidence.get("focused_evidence") is True:
        print("Bitbucket release evidence is focused CI evidence, not release authority", file=sys.stderr)
        sys.exit(1)
    if evidence.get("release_authority") != "strict-managed-release":
        print("Bitbucket release evidence release_authority is not strict-managed-release", file=sys.stderr)
        print(f"  actual={evidence.get('release_authority')!r}", file=sys.stderr)
        sys.exit(1)
    authority_boundary = evidence.get("release_authority_boundary")
    if not isinstance(authority_boundary, dict):
        print("Bitbucket release evidence is missing release_authority_boundary", file=sys.stderr)
        sys.exit(1)
    if authority_boundary.get("focused_evidence_satisfies_release") is not False:
        print("Bitbucket release evidence boundary must reject focused evidence", file=sys.stderr)
        print(f"  actual={authority_boundary.get('focused_evidence_satisfies_release')!r}", file=sys.stderr)
        sys.exit(1)
    if authority_boundary.get("strict_release_gate_required_for_promotion") is not True:
        print("Bitbucket release evidence boundary must require the strict promotion gate", file=sys.stderr)
        print(f"  actual={authority_boundary.get('strict_release_gate_required_for_promotion')!r}", file=sys.stderr)
        sys.exit(1)
    if authority_boundary.get("strict_release_gate_satisfied") is not True:
        print("Bitbucket release evidence boundary does not satisfy the strict promotion gate", file=sys.stderr)
        print(f"  actual={authority_boundary.get('strict_release_gate_satisfied')!r}", file=sys.stderr)
        sys.exit(1)
    if authority_boundary.get("managed_release_requirements_satisfied") is not True:
        print("Bitbucket release evidence boundary does not satisfy managed release requirements", file=sys.stderr)
        print(f"  actual={authority_boundary.get('managed_release_requirements_satisfied')!r}", file=sys.stderr)
        sys.exit(1)
    generated_at = parse_timestamp_with_timezone(evidence.get("generated_at_utc"))
    if generated_at is None:
        print(
            "Bitbucket release evidence generated_at_utc is missing or invalid",
            file=sys.stderr,
        )
        sys.exit(1)
    now = dt.datetime.now(dt.timezone.utc)
    if generated_at > now + FUTURE_SKEW:
        print(
            "Bitbucket release evidence generated_at_utc is in the future",
            file=sys.stderr,
        )
        sys.exit(1)
    if generated_at < now - MAX_EVIDENCE_AGE:
        print(
            "Bitbucket release evidence generated_at_utc is stale",
            file=sys.stderr,
        )
        sys.exit(1)
    blockers = evidence.get("release_blockers")
    if not isinstance(blockers, list):
        print("Bitbucket release evidence is missing release_blockers list", file=sys.stderr)
        sys.exit(1)
    failure_summary = evidence.get("failure_summary")
    if not isinstance(failure_summary, dict):
        print("Bitbucket release evidence is missing failure_summary", file=sys.stderr)
        sys.exit(1)
    root_causes = failure_summary.get("root_causes")
    if not isinstance(root_causes, list):
        print("Bitbucket release evidence is missing failure_summary.root_causes list", file=sys.stderr)
        sys.exit(1)
    invalid_blockers = [
        index
        for index, blocker in enumerate(blockers)
        if not isinstance(blocker, str) or not blocker.strip()
    ]
    if invalid_blockers:
        print(
            "Bitbucket release evidence release_blockers contain non-string or empty entries",
            file=sys.stderr,
        )
        print(f"  indexes={invalid_blockers}", file=sys.stderr)
        sys.exit(1)
    invalid_root_causes = [
        index
        for index, cause in enumerate(root_causes)
        if not isinstance(cause, str) or not cause.strip()
    ]
    if invalid_root_causes:
        print(
            "Bitbucket release evidence failure_summary.root_causes contain non-string or empty entries",
            file=sys.stderr,
        )
        print(f"  indexes={invalid_root_causes}", file=sys.stderr)
        sys.exit(1)
    normalized_blockers = [
        blocker.strip() for blocker in blockers
    ]
    normalized_root_causes = [
        cause.strip() for cause in root_causes
    ]
    if normalized_root_causes != normalized_blockers:
        print("Bitbucket release evidence failure summary does not match release blockers", file=sys.stderr)
        print(f"  root_causes={normalized_root_causes}", file=sys.stderr)
        print(f"  release_blockers={normalized_blockers}", file=sys.stderr)
        sys.exit(1)
    retained_failure_fields = [
        field for field in ("failed_stage", "exit_status") if field in evidence
    ]
    if retained_failure_fields:
        print(
            "Bitbucket release evidence cannot retain failure fields with ready flags",
            file=sys.stderr,
        )
        print(f"  fields={retained_failure_fields}", file=sys.stderr)
        sys.exit(1)
    if blockers:
        print("Bitbucket release gate has retained blockers despite ready flags:", file=sys.stderr)
        for blocker in blockers:
            if isinstance(blocker, str):
                print(f"  - {blocker}", file=sys.stderr)
        sys.exit(1)
    requirements = evidence.get("release_requirements")
    required_keys = {
        "security_assurance_decision_required": "security assurance decision",
        "production_attestations_required": "production attestations",
        "ops_certification_required": "operations certification",
        "live_ops_evidence_required": "live operations evidence",
        "pds_baseline_evidence_required": "PDS baseline evidence",
        "performance_evidence_required": "performance evidence",
        "release_identity_required": "release identity evidence",
    }
    if not isinstance(requirements, dict):
        print("Bitbucket release evidence is missing release_requirements", file=sys.stderr)
        sys.exit(1)
    missing = [
        label
        for key, label in required_keys.items()
        if requirements.get(key) is not True
    ]
    if missing:
        print("Bitbucket release evidence is missing strict requirements:", file=sys.stderr)
        for label in missing:
            print(f"  - {label}", file=sys.stderr)
        sys.exit(1)
    release_check = require_mapping(evidence, "release_check")
    require_true(release_check, "release_check", "ok")
    require_true(release_check, "release_check", "release_ready")
    checks = release_check.get("checks")
    if not isinstance(checks, list) or not checks:
        print("Bitbucket release evidence release_check.checks are missing", file=sys.stderr)
        sys.exit(1)
    check_entries = [
        check for check in checks if isinstance(check, dict)
    ]
    if len(check_entries) != len(checks):
        print("Bitbucket release evidence release_check.checks entries must be objects", file=sys.stderr)
        sys.exit(1)
    check_names = []
    invalid_check_names = []
    for index, check in enumerate(check_entries):
        check_name = check.get("name")
        if isinstance(check_name, str) and check_name.strip():
            check_names.append(check_name.strip())
        else:
            invalid_check_names.append(index)
    if invalid_check_names:
        print("Bitbucket release evidence release_check.check names are invalid", file=sys.stderr)
        print(f"  indexes={invalid_check_names}", file=sys.stderr)
        sys.exit(1)
    duplicate_checks = sorted(
        check_name
        for check_name in set(check_names)
        if check_names.count(check_name) > 1
    )
    if duplicate_checks:
        print("Bitbucket release evidence release_check.check entries are duplicated", file=sys.stderr)
        for check_name in duplicate_checks:
            print(f"  - {check_name}", file=sys.stderr)
        sys.exit(1)
    failed_checks = [
        check.get("name", f"#{index}")
        for index, check in enumerate(check_entries)
        if check.get("ok") is not True
    ]
    if failed_checks:
        print("Bitbucket release evidence release_check has failing checks", file=sys.stderr)
        for check in failed_checks:
            print(f"  - {check}", file=sys.stderr)
        sys.exit(1)
    required_release_checks = {
        "pds-baseline",
        "provider-url-preflight",
        "release-identity",
        "ops-certification",
        "composition-check",
        "provider-test",
        "security-certification",
        "load-test-suite",
        "provider-performance",
    }
    seen_release_checks = set(check_names)
    if not required_release_checks.issubset(seen_release_checks):
        print("Bitbucket release evidence release_check.checks are incomplete", file=sys.stderr)
        print(f"  missing={sorted(required_release_checks - seen_release_checks)}", file=sys.stderr)
        print(f"  actual={sorted(seen_release_checks)}", file=sys.stderr)
        sys.exit(1)
    artifact_required_checks = {
        "pds-baseline",
        "provider-url-preflight",
        "release-identity",
        "ops-certification",
        "composition-check",
        "provider-test",
        "security-certification",
        "load-test-suite",
        "provider-performance",
    }
    missing_check_artifacts = [
        check.get("name", f"#{index}")
        for index, check in enumerate(check_entries)
        if check.get("name") in artifact_required_checks
        and (
            not isinstance(check.get("artifact"), str)
            or not check.get("artifact").strip()
        )
    ]
    if missing_check_artifacts:
        print("Bitbucket release evidence release_check.check artifacts are missing", file=sys.stderr)
        for check_name in missing_check_artifacts:
            print(f"  - {check_name}", file=sys.stderr)
        sys.exit(1)
    missing_retained_check_artifacts = []
    off_tree_check_artifacts = []
    for index, check in enumerate(check_entries):
        check_name_value = check.get("name")
        check_name = check_name_value.strip() if isinstance(check_name_value, str) else f"#{index}"
        if check_name not in artifact_required_checks:
            continue
        artifact_value = check.get("artifact")
        resolved_artifact = resolve_retained_artifact(artifact_value)
        if resolved_artifact is None:
            missing_retained_check_artifacts.append(f"{check_name}: {artifact_value!r}")
        elif not path_is_in_report_dir(resolved_artifact):
            off_tree_check_artifacts.append(f"{check_name}: {resolved_artifact.as_posix()}")
    if missing_retained_check_artifacts:
        print("Bitbucket release evidence release_check.check artifacts do not exist", file=sys.stderr)
        for artifact in missing_retained_check_artifacts:
            print(f"  - {artifact}", file=sys.stderr)
        sys.exit(1)
    if off_tree_check_artifacts:
        print("Bitbucket release evidence release_check.check artifacts are outside the report directory", file=sys.stderr)
        print(f"  report_dir={report_dir.as_posix()}", file=sys.stderr)
        for artifact in off_tree_check_artifacts:
            print(f"  - {artifact}", file=sys.stderr)
        sys.exit(1)
    expected_check_artifacts = {
        "pds-baseline": report_dir / "pds-security-baseline.json",
        "provider-url-preflight": report_dir / "release-check-provider-url-preflight.json",
        "release-identity": report_dir / "release-identity.json",
        "ops-certification": report_dir / "ops-certification.json",
        "composition-check": report_dir / "composition-check.json",
        "provider-test": report_dir / "provider-parity.json",
        "security-certification": report_dir / "security-certification.json",
        "load-test-suite": report_dir / "load-test.json",
        "provider-performance": report_dir / "provider-performance.json",
    }
    mismatched_check_artifacts = []
    for index, check in enumerate(check_entries):
        check_name_value = check.get("name")
        check_name = check_name_value.strip() if isinstance(check_name_value, str) else f"#{index}"
        expected_artifact = expected_check_artifacts.get(check_name)
        if expected_artifact is None:
            continue
        resolved_artifact = resolve_retained_artifact(check.get("artifact"))
        expected_artifact = expected_artifact.resolve(strict=False)
        if resolved_artifact is None or resolved_artifact != expected_artifact:
            mismatched_check_artifacts.append(
                f"{check_name}: expected {expected_artifact.as_posix()}, actual {check.get('artifact')!r}"
            )
    if mismatched_check_artifacts:
        print("Bitbucket release evidence release_check.check artifacts do not match retained evidence", file=sys.stderr)
        for artifact in mismatched_check_artifacts:
            print(f"  - {artifact}", file=sys.stderr)
        sys.exit(1)

    retained_release_check = load_retained_json("release-check", report_dir / "release-check.json")
    require_child_command(retained_release_check, "release-check", "release-check")
    require_child_timestamp(retained_release_check, "release-check", generated_at)
    require_child_true(retained_release_check, "release-check", "ok")
    require_child_true(retained_release_check, "release-check", "release_ready")
    retained_release_check_entries = collect_green_check_entries(retained_release_check, "release-check")
    retained_release_check_names = set(retained_release_check_entries)
    if not required_release_checks.issubset(retained_release_check_names):
        print("Bitbucket release evidence retained release-check checks are incomplete", file=sys.stderr)
        print(f"  missing={sorted(required_release_checks - retained_release_check_names)}", file=sys.stderr)
        print(f"  actual={sorted(retained_release_check_names)}", file=sys.stderr)
        sys.exit(1)
    missing_from_retained_release_check = sorted(seen_release_checks - retained_release_check_names)
    if missing_from_retained_release_check:
        print("Bitbucket release evidence retained release-check does not match wrapper check summary", file=sys.stderr)
        print(f"  missing={missing_from_retained_release_check}", file=sys.stderr)
        sys.exit(1)
    mismatched_retained_release_check_artifacts = []
    for check_name, expected_artifact in expected_check_artifacts.items():
        retained_entry = retained_release_check_entries.get(check_name)
        if not isinstance(retained_entry, dict):
            mismatched_retained_release_check_artifacts.append(
                f"{check_name}: missing retained check entry"
            )
            continue
        retained_artifact = retained_entry.get("artifact")
        resolved_artifact = resolve_retained_artifact(retained_artifact)
        expected_artifact = expected_artifact.resolve(strict=False)
        if resolved_artifact is None or resolved_artifact != expected_artifact:
            mismatched_retained_release_check_artifacts.append(
                f"{check_name}: expected {expected_artifact.as_posix()}, actual {retained_artifact!r}"
            )
    if mismatched_retained_release_check_artifacts:
        print("Bitbucket release evidence retained release-check artifacts do not match retained evidence", file=sys.stderr)
        for artifact in mismatched_retained_release_check_artifacts:
            print(f"  - {artifact}", file=sys.stderr)
        sys.exit(1)

    retained_provider_report = load_retained_json("provider certification", report_dir / "provider-parity.json")
    require_child_command(retained_provider_report, "provider certification", "provider-test")
    require_child_timestamp(retained_provider_report, "provider certification", generated_at)
    require_child_true(retained_provider_report, "provider certification", "ok")
    retained_provider_entries = retained_provider_report.get("providers")
    if not isinstance(retained_provider_entries, list) or not retained_provider_entries:
        print("Bitbucket release evidence retained provider certification providers are missing", file=sys.stderr)
        sys.exit(1)
    retained_provider_names = [
        provider.get("provider")
        for provider in retained_provider_entries
        if isinstance(provider, dict)
        and isinstance(provider.get("provider"), str)
        and provider.get("provider").strip()
    ]
    if set(retained_provider_names) != {"postgres", "mongo", "mssql", "snowflake"}:
        print("Bitbucket release evidence retained provider certification provider set is incomplete", file=sys.stderr)
        print(f"  actual={sorted(set(retained_provider_names))}", file=sys.stderr)
        sys.exit(1)
    retained_failing_providers = [
        provider.get("provider", f"#{index}")
        for index, provider in enumerate(retained_provider_entries)
        if not isinstance(provider, dict) or provider.get("ok") is not True
    ]
    if retained_failing_providers:
        print("Bitbucket release evidence retained provider certification is not green", file=sys.stderr)
        for provider in retained_failing_providers:
            print(f"  - {provider}", file=sys.stderr)
        sys.exit(1)
    incomplete_retained_provider_evidence = []
    for provider_name in sorted({"postgres", "mongo", "mssql", "snowflake"}):
        provider = next(
            (
                entry
                for entry in retained_provider_entries
                if isinstance(entry, dict) and entry.get("provider") == provider_name
            ),
            None,
        )
        if not isinstance(provider, dict):
            incomplete_retained_provider_evidence.append(f"{provider_name}: provider entry missing")
            continue
        live_areas = [
            area
            for area in provider.get("areas", [])
            if isinstance(area, dict) and area.get("status") == "live-certified"
        ]
        failed_live_areas = [
            area.get("area", f"#{index}")
            for index, area in enumerate(live_areas)
            if area.get("ok") is not True
            or area.get("live_result") != "passed"
            or not isinstance(area.get("live_contracts"), list)
            or not area.get("live_contracts")
        ]
        if (
            provider.get("mode") != "full"
            or not isinstance(provider.get("data_source"), str)
            or not provider.get("data_source").strip()
            or not isinstance(provider.get("base_url"), str)
            or not provider.get("base_url").strip()
            or not live_areas
            or failed_live_areas
        ):
            incomplete_retained_provider_evidence.append(
                f"{provider_name}: mode={provider.get('mode')!r}; "
                f"data_source={provider.get('data_source')!r}; "
                f"base_url={provider.get('base_url')!r}; "
                f"live_certified_total={len(live_areas)}; "
                f"failing_live_areas={failed_live_areas}"
            )
    if incomplete_retained_provider_evidence:
        print("Bitbucket release evidence retained provider live certification is incomplete", file=sys.stderr)
        for provider in incomplete_retained_provider_evidence:
            print(f"  - {provider}", file=sys.stderr)
        sys.exit(1)

    retained_security_certification = load_retained_json("security certification", report_dir / "security-certification.json")
    require_child_command(retained_security_certification, "security certification", "security-certification-evidence")
    require_child_timestamp(retained_security_certification, "security certification", generated_at)
    require_child_true(retained_security_certification, "security certification", "ok")
    require_child_true(retained_security_certification, "security certification", "release_ready")
    require_clean_child_evidence(retained_security_certification, "security certification")
    security_artifacts = collect_artifact_entries(retained_security_certification, "security certification")
    security_artifacts_by_name = {
        artifact.get("name"): artifact
        for artifact in security_artifacts
        if isinstance(artifact.get("name"), str)
    }
    required_security_artifacts = {
        "runtime-routing-source",
        "graphql-introspection-runtime-test-log",
        "provider-parity",
        "provider-log:postgres",
        "provider-log:mongo",
        "provider-log:mssql",
        "provider-log:snowflake",
    }
    missing_security_artifacts = sorted(required_security_artifacts - set(security_artifacts_by_name))
    if missing_security_artifacts:
        print("Bitbucket release evidence retained security certification artifacts are incomplete", file=sys.stderr)
        print(f"  missing={missing_security_artifacts}", file=sys.stderr)
        sys.exit(1)
    invalid_security_artifacts = []
    for artifact_name in required_security_artifacts:
        artifact = security_artifacts_by_name.get(artifact_name)
        if not isinstance(artifact, dict):
            continue
        resolved_artifact = validate_retained_file_artifact(
            artifact,
            artifact_name,
            invalid_security_artifacts,
            require_required=True,
            require_report_dir=artifact_name != "runtime-routing-source",
        )
        if resolved_artifact is None:
            continue
        if artifact_name == "provider-parity":
            expected_provider_parity = (report_dir / "provider-parity.json").resolve(strict=False)
            if resolved_artifact != expected_provider_parity:
                invalid_security_artifacts.append(
                    f"{artifact_name}: expected {expected_provider_parity.as_posix()}, actual {artifact.get('path')!r}"
                )
    if invalid_security_artifacts:
        print("Bitbucket release evidence retained security certification artifacts are invalid", file=sys.stderr)
        for artifact in invalid_security_artifacts:
            print(f"  - {artifact}", file=sys.stderr)
        sys.exit(1)
    introspection_auth = retained_security_certification.get("introspection_auth")
    introspection_cases = introspection_auth.get("cases") if isinstance(introspection_auth, dict) else None
    if not isinstance(introspection_cases, list) or not introspection_cases:
        print("Bitbucket release evidence retained security introspection cases are missing", file=sys.stderr)
        sys.exit(1)
    failing_introspection_cases = [
        case.get("case", f"#{index}")
        for index, case in enumerate(introspection_cases)
        if not isinstance(case, dict)
        or case.get("ok") is not True
        or case.get("runtime_result") != "passed"
        or case.get("source_present") is not True
    ]
    if failing_introspection_cases:
        print("Bitbucket release evidence retained security introspection cases are not green", file=sys.stderr)
        for case in failing_introspection_cases:
            print(f"  - {case}", file=sys.stderr)
        sys.exit(1)
    live_security_contracts = retained_security_certification.get("live_security_contracts")
    live_security_providers = live_security_contracts.get("providers") if isinstance(live_security_contracts, dict) else None
    expected_live_security_contracts = [
        contract.get("name")
        for contract in live_security_contracts.get("expected_contracts", [])
        if isinstance(live_security_contracts, dict)
        and isinstance(contract, dict)
        and isinstance(contract.get("name"), str)
        and contract.get("name").strip()
    ] if isinstance(live_security_contracts, dict) else []
    if not expected_live_security_contracts:
        print("Bitbucket release evidence retained security live contract expectations are missing", file=sys.stderr)
        sys.exit(1)
    if not isinstance(live_security_providers, list) or not live_security_providers:
        print("Bitbucket release evidence retained security live provider contracts are missing", file=sys.stderr)
        sys.exit(1)
    live_security_provider_names = [
        provider.get("provider")
        for provider in live_security_providers
        if isinstance(provider, dict)
        and isinstance(provider.get("provider"), str)
        and provider.get("provider").strip()
    ]
    if set(live_security_provider_names) != {"postgres", "mongo", "mssql", "snowflake"}:
        print("Bitbucket release evidence retained security live provider set is incomplete", file=sys.stderr)
        print(f"  actual={sorted(set(live_security_provider_names))}", file=sys.stderr)
        sys.exit(1)
    failing_security_contracts = []
    for provider in live_security_providers:
        provider_name = provider.get("provider") if isinstance(provider, dict) else "<unknown>"
        if not isinstance(provider, dict):
            failing_security_contracts.append(f"{provider_name}: provider entry is not an object")
            continue
        if provider.get("ok") is not True:
            failing_security_contracts.append(f"{provider_name}: provider ok is not true")
        contracts = provider.get("contracts")
        if not isinstance(contracts, list) or not contracts:
            failing_security_contracts.append(f"{provider_name}: contracts are missing")
            continue
        contract_names = [
            contract.get("name")
            for contract in contracts
            if isinstance(contract, dict)
            and isinstance(contract.get("name"), str)
            and contract.get("name").strip()
        ]
        missing_contracts = sorted(set(expected_live_security_contracts) - set(contract_names))
        if missing_contracts:
            failing_security_contracts.append(f"{provider_name}: missing contracts {missing_contracts}")
        for contract in contracts:
            contract_name = contract.get("name") if isinstance(contract, dict) else "<unknown>"
            if (
                not isinstance(contract, dict)
                or contract.get("ok") is not True
                or contract.get("result") != "passed"
            ):
                failing_security_contracts.append(f"{provider_name}: {contract_name} did not pass")
    if failing_security_contracts:
        print("Bitbucket release evidence retained security live contracts are not green", file=sys.stderr)
        for contract in failing_security_contracts:
            print(f"  - {contract}", file=sys.stderr)
        sys.exit(1)

    retained_release_evidence = load_retained_json("release evidence check", report_dir / "release-evidence-check.json")
    require_child_command(retained_release_evidence, "release evidence check", "release-evidence-check")
    require_child_timestamp(retained_release_evidence, "release evidence check", generated_at)
    require_child_true(retained_release_evidence, "release evidence check", "ok")
    require_child_true(retained_release_evidence, "release evidence check", "release_ready")
    require_clean_child_evidence(retained_release_evidence, "release evidence check")
    if retained_release_evidence.get("schema_ok") is not True:
        print("Bitbucket release evidence retained release evidence check schema_ok is not true", file=sys.stderr)
        print(f"  actual={retained_release_evidence.get('schema_ok')!r}", file=sys.stderr)
        sys.exit(1)
    if retained_release_evidence.get("release_blocker_count") != 0:
        print("Bitbucket release evidence retained release evidence check has release blockers", file=sys.stderr)
        print(f"  actual={retained_release_evidence.get('release_blocker_count')!r}", file=sys.stderr)
        sys.exit(1)
    if retained_release_evidence.get("synthetic_fixtures") != []:
        print("Bitbucket release evidence retained release evidence check used synthetic fixtures", file=sys.stderr)
        print(f"  actual={retained_release_evidence.get('synthetic_fixtures')!r}", file=sys.stderr)
        sys.exit(1)

    retained_ops_certification = load_retained_json("ops certification", report_dir / "ops-certification.json")
    require_child_command(retained_ops_certification, "ops certification", "ops-certification")
    require_child_timestamp(retained_ops_certification, "ops certification", generated_at)
    require_child_true(retained_ops_certification, "ops certification", "ok")
    require_child_true(retained_ops_certification, "ops certification", "release_ready")
    require_child_true(retained_ops_certification, "ops certification", "live_evidence_required")
    require_child_true(retained_ops_certification, "ops certification", "static_bundle_verified")
    require_child_true(retained_ops_certification, "ops certification", "promtool_required")
    require_clean_child_evidence(retained_ops_certification, "ops certification")
    ops_artifacts = collect_artifact_entries(retained_ops_certification, "ops certification")
    live_ops_artifacts = [
        artifact
        for artifact in ops_artifacts
        if artifact.get("name") == "live-ops-evidence"
    ]
    if not live_ops_artifacts:
        print("Bitbucket release evidence retained ops certification live evidence artifacts are missing", file=sys.stderr)
        sys.exit(1)
    invalid_live_ops_artifacts = []
    for artifact in live_ops_artifacts:
        validate_retained_file_artifact(
            artifact,
            f"live ops evidence artifact {artifact.get('path')!r}" if isinstance(artifact, dict) else "live ops evidence artifact",
            invalid_live_ops_artifacts,
            require_required=True,
        )
    if invalid_live_ops_artifacts:
        print("Bitbucket release evidence retained ops certification live evidence artifacts are invalid", file=sys.stderr)
        for artifact in invalid_live_ops_artifacts:
            print(f"  - {artifact}", file=sys.stderr)
        sys.exit(1)

    retained_pds_baseline = load_retained_json("PDS security baseline", report_dir / "pds-security-baseline.json")
    require_child_command(retained_pds_baseline, "PDS security baseline", "pds-baseline")
    require_child_timestamp(retained_pds_baseline, "PDS security baseline", generated_at)
    require_child_true(retained_pds_baseline, "PDS security baseline", "ok")
    require_child_true(retained_pds_baseline, "PDS security baseline", "release_ready")
    require_child_true(retained_pds_baseline, "PDS security baseline", "live_evidence_required")
    require_clean_child_evidence(retained_pds_baseline, "PDS security baseline")
    pds_decision_artifact = retained_pds_baseline.get("decision_artifact")
    invalid_pds_evidence_artifacts = []
    validate_retained_file_artifact(
        pds_decision_artifact,
        "PDS baseline decision artifact",
        invalid_pds_evidence_artifacts,
        require_ok=True,
    )
    if invalid_pds_evidence_artifacts:
        print("Bitbucket release evidence retained PDS baseline decision artifact is invalid", file=sys.stderr)
        for artifact in invalid_pds_evidence_artifacts:
            print(f"  - {artifact}", file=sys.stderr)
        sys.exit(1)
    pds_evidence_artifacts = retained_pds_baseline.get("evidence_artifacts")
    if not isinstance(pds_evidence_artifacts, list) or not pds_evidence_artifacts:
        print("Bitbucket release evidence retained PDS baseline evidence artifacts are missing", file=sys.stderr)
        sys.exit(1)
    invalid_pds_evidence_artifacts = []
    for artifact in pds_evidence_artifacts:
        validate_retained_file_artifact(
            artifact,
            f"PDS baseline evidence artifact {artifact.get('path')!r}" if isinstance(artifact, dict) else "PDS baseline evidence artifact",
            invalid_pds_evidence_artifacts,
            require_ok=True,
        )
    if invalid_pds_evidence_artifacts:
        print("Bitbucket release evidence retained PDS baseline evidence artifacts are invalid", file=sys.stderr)
        for artifact in invalid_pds_evidence_artifacts:
            print(f"  - {artifact}", file=sys.stderr)
        sys.exit(1)
    pds_decision_summary = retained_pds_baseline.get("decision_summary")
    required_pds_work_items = {"LIVE-015", "LIVE-016", "LIVE-017", "LIVE-018", "LIVE-019", "LIVE-020", "LIVE-021"}
    covered_pds_work_items = set()
    if isinstance(pds_decision_summary, dict):
        for field in ("evidence_backed_work_items", "risk_accepted_work_items"):
            values = pds_decision_summary.get(field)
            if isinstance(values, list):
                covered_pds_work_items.update(
                    value for value in values if isinstance(value, str)
                )
    missing_pds_work_items = sorted(required_pds_work_items - covered_pds_work_items)
    if missing_pds_work_items:
        print("Bitbucket release evidence retained PDS baseline decision does not cover required work items", file=sys.stderr)
        print(f"  missing={missing_pds_work_items}", file=sys.stderr)
        sys.exit(1)

    retained_load_test = load_retained_json("load test", report_dir / "load-test.json")
    require_child_command(retained_load_test, "load test", "load-test-suite")
    require_child_timestamp(retained_load_test, "load test", generated_at)
    require_child_true(retained_load_test, "load test", "ok")
    load_totals = retained_load_test.get("totals")
    load_scenarios = retained_load_test.get("scenarios")
    load_violations = retained_load_test.get("violations")
    load_test_incomplete = []
    if not is_int(retained_load_test.get("scenario_count")) or retained_load_test.get("scenario_count") <= 0:
        load_test_incomplete.append("scenario_count must be a positive integer")
    for field in ("passed", "failed", "requests_per_scenario", "concurrency"):
        if not is_int(retained_load_test.get(field)):
            load_test_incomplete.append(f"{field} must be an integer")
    if retained_load_test.get("failed") != 0:
        load_test_incomplete.append(f"failed must be 0, actual {retained_load_test.get('failed')!r}")
    if load_violations != []:
        load_test_incomplete.append(f"violations must be empty, actual {load_violations!r}")
    if not isinstance(load_totals, dict):
        load_test_incomplete.append("totals must be recorded")
    else:
        for field in ("requests", "success", "failed"):
            if not is_int(load_totals.get(field)):
                load_test_incomplete.append(f"totals.{field} must be an integer")
        for field in ("error_rate", "max_p95_ms", "max_latency_ms"):
            if not is_number(load_totals.get(field)):
                load_test_incomplete.append(f"totals.{field} must be numeric")
        if not is_int(load_totals.get("requests")) or load_totals.get("requests") <= 0:
            load_test_incomplete.append(f"totals.requests must be positive, actual {load_totals.get('requests')!r}")
        if load_totals.get("failed") != 0:
            load_test_incomplete.append(f"totals.failed must be 0, actual {load_totals.get('failed')!r}")
    if not isinstance(load_scenarios, list) or not load_scenarios:
        load_test_incomplete.append("scenarios must be recorded")
    elif is_int(retained_load_test.get("scenario_count")) and retained_load_test.get("scenario_count") != len(load_scenarios):
        load_test_incomplete.append(
            f"scenario_count must match scenarios length, actual {retained_load_test.get('scenario_count')!r} vs {len(load_scenarios)}"
        )
    if not isinstance(retained_load_test.get("thresholds"), dict):
        load_test_incomplete.append("thresholds must be recorded")
    if isinstance(load_scenarios, list):
        for index, scenario in enumerate(load_scenarios):
            scenario_label = f"scenario #{index}"
            if not isinstance(scenario, dict):
                load_test_incomplete.append(f"{scenario_label} must be an object")
                continue
            scenario_name = scenario.get("name")
            if isinstance(scenario_name, str) and scenario_name.strip():
                scenario_label = scenario_name.strip()
            if scenario.get("ok") is not True:
                load_test_incomplete.append(f"{scenario_label} ok must be true")
            for field in ("requests", "success", "failed"):
                if not is_int(scenario.get(field)):
                    load_test_incomplete.append(f"{scenario_label}.{field} must be an integer")
            if scenario.get("failed") != 0:
                load_test_incomplete.append(f"{scenario_label}.failed must be 0, actual {scenario.get('failed')!r}")
            if scenario.get("violations") != []:
                load_test_incomplete.append(f"{scenario_label}.violations must be empty")
            validate_retained_file_artifact(
                scenario,
                f"{scenario_label} retained scenario artifact",
                load_test_incomplete,
                require_present=False,
            )
    if load_test_incomplete:
        print("Bitbucket release evidence retained load-test-suite evidence is incomplete", file=sys.stderr)
        for issue in load_test_incomplete:
            print(f"  - {issue}", file=sys.stderr)
        sys.exit(1)
    retained_provider_performance = load_retained_json("provider performance", report_dir / "provider-performance.json")
    require_child_command(retained_provider_performance, "provider performance", "provider-performance")
    require_child_timestamp(retained_provider_performance, "provider performance", generated_at)
    require_child_true(retained_provider_performance, "provider performance", "ok")
    provider_performance_incomplete = []
    if retained_provider_performance.get("require_live") is not True:
        provider_performance_incomplete.append(
            f"require_live must be true, actual {retained_provider_performance.get('require_live')!r}"
        )
    performance_artifacts = retained_provider_performance.get("artifacts")
    if not isinstance(performance_artifacts, dict):
        provider_performance_incomplete.append("artifacts must be recorded")
    else:
        provider_parity_entry = performance_artifacts.get("provider_parity")
        provider_parity_artifact = validate_retained_file_artifact(
            provider_parity_entry,
            "provider-performance provider_parity artifact",
            provider_performance_incomplete,
            require_required=True,
            require_ok=True,
        )
        expected_provider_parity = (report_dir / "provider-parity.json").resolve(strict=False)
        if provider_parity_artifact != expected_provider_parity:
            provider_performance_incomplete.append(
                f"provider_parity artifact must match retained provider-parity.json, actual {provider_parity_entry!r}"
            )
        recommendations_entry = performance_artifacts.get("performance_recommendations")
        recommendations_artifact = validate_retained_file_artifact(
            recommendations_entry,
            "provider-performance performance_recommendations artifact",
            provider_performance_incomplete,
            require_required=True,
            require_ok=True,
        )
        expected_recommendations = (report_dir / "performance_recommendations.json").resolve(strict=False)
        if recommendations_artifact != expected_recommendations:
            provider_performance_incomplete.append(
                "performance_recommendations artifact must match retained "
                f"performance_recommendations.json, actual {recommendations_entry!r}"
            )
        query_cost_entry = performance_artifacts.get("query_cost_source")
        query_cost_artifact = validate_retained_file_artifact(
            query_cost_entry,
            "provider-performance query_cost_source artifact",
            provider_performance_incomplete,
            require_required=True,
            require_ok=True,
            require_report_dir=False,
        )
        expected_query_cost = (repo_root / "appfw_runtime/src/query_cost.rs").resolve(strict=False)
        if query_cost_artifact != expected_query_cost:
            provider_performance_incomplete.append(
                f"query_cost_source artifact must match appfw_runtime/src/query_cost.rs, actual {query_cost_entry!r}"
            )
    performance_summary = retained_provider_performance.get("summary")
    if not isinstance(performance_summary, dict):
        provider_performance_incomplete.append("summary must be recorded")
    else:
        for field in (
            "provider_count",
            "provider_ok_count",
            "recommendation_count",
            "generated_recommendation_count",
            "blocking_failure_count",
        ):
            if not is_int(performance_summary.get(field)):
                provider_performance_incomplete.append(f"summary.{field} must be an integer")
        if performance_summary.get("provider_count") != 4:
            provider_performance_incomplete.append(f"summary.provider_count must be 4, actual {performance_summary.get('provider_count')!r}")
        if performance_summary.get("provider_ok_count") != 4:
            provider_performance_incomplete.append(f"summary.provider_ok_count must be 4, actual {performance_summary.get('provider_ok_count')!r}")
        if is_int(performance_summary.get("recommendation_count")) and performance_summary.get("recommendation_count") <= 0:
            provider_performance_incomplete.append(
                f"summary.recommendation_count must be greater than 0, actual {performance_summary.get('recommendation_count')!r}"
            )
        if (
            is_int(performance_summary.get("generated_recommendation_count"))
            and performance_summary.get("generated_recommendation_count") <= 0
        ):
            provider_performance_incomplete.append(
                "summary.generated_recommendation_count must be greater than 0, "
                f"actual {performance_summary.get('generated_recommendation_count')!r}"
            )
        if performance_summary.get("blocking_failure_count") != 0:
            provider_performance_incomplete.append(
                f"summary.blocking_failure_count must be 0, actual {performance_summary.get('blocking_failure_count')!r}"
            )
    if retained_provider_performance.get("blocking_failures") != []:
        provider_performance_incomplete.append(
            f"blocking_failures must be empty, actual {retained_provider_performance.get('blocking_failures')!r}"
        )
    performance_checks = retained_provider_performance.get("checks")
    if not isinstance(performance_checks, list) or not performance_checks:
        provider_performance_incomplete.append("checks must be recorded")
    performance_providers = retained_provider_performance.get("providers")
    if not isinstance(performance_providers, list) or not performance_providers:
        provider_performance_incomplete.append("providers must be recorded")
    else:
        performance_provider_names = [
            provider.get("provider")
            for provider in performance_providers
            if isinstance(provider, dict)
            and isinstance(provider.get("provider"), str)
            and provider.get("provider").strip()
        ]
        if set(performance_provider_names) != {"postgres", "mongo", "mssql", "snowflake"}:
            provider_performance_incomplete.append(
                f"provider set must be postgres, mongo, mssql, snowflake; actual {sorted(set(performance_provider_names))}"
            )
        duplicate_performance_providers = sorted(
            provider_name
            for provider_name in set(performance_provider_names)
            if performance_provider_names.count(provider_name) > 1
        )
        if duplicate_performance_providers:
            provider_performance_incomplete.append(f"duplicate provider entries: {duplicate_performance_providers}")
        for provider in performance_providers:
            provider_name = provider.get("provider") if isinstance(provider, dict) else "<unknown>"
            if not isinstance(provider, dict):
                provider_performance_incomplete.append(f"{provider_name} provider entry must be an object")
                continue
            if provider.get("ok") is not True:
                provider_performance_incomplete.append(f"{provider_name} provider ok must be true")
            if provider.get("status") != "live-certified":
                provider_performance_incomplete.append(
                    f"{provider_name} status must be live-certified, actual {provider.get('status')!r}"
                )
            if not isinstance(provider.get("base_url"), str) or not provider.get("base_url").strip():
                provider_performance_incomplete.append(f"{provider_name} base_url must be recorded")
            provider_checks = provider.get("checks")
            if not isinstance(provider_checks, list) or not provider_checks:
                provider_performance_incomplete.append(f"{provider_name} checks must be recorded")
                continue
            for check in provider_checks:
                check_name = check.get("name") if isinstance(check, dict) else "<unknown>"
                if not isinstance(check, dict):
                    provider_performance_incomplete.append(f"{provider_name} {check_name} check must be an object")
                    continue
                if check.get("ok") is not True:
                    provider_performance_incomplete.append(f"{provider_name} {check_name} check ok must be true")
                areas = check.get("areas")
                if not isinstance(areas, list) or not areas:
                    provider_performance_incomplete.append(f"{provider_name} {check_name} areas must be recorded")
                    continue
                for area in areas:
                    area_name = area.get("area") if isinstance(area, dict) else "<unknown>"
                    if (
                        not isinstance(area, dict)
                        or area.get("ok") is not True
                        or area.get("live_result") != "passed"
                    ):
                        provider_performance_incomplete.append(
                            f"{provider_name} {check_name} {area_name} live result must pass"
                        )
    if provider_performance_incomplete:
        print("Bitbucket release evidence retained provider-performance evidence is incomplete", file=sys.stderr)
        for issue in provider_performance_incomplete:
            print(f"  - {issue}", file=sys.stderr)
        sys.exit(1)

    security_gates = require_mapping(evidence, "security_gates")
    require_true(security_gates, "security_gates", "ok")
    gitleaks_findings = security_gates.get("gitleaks_findings")
    if type(gitleaks_findings) is not int or gitleaks_findings < 0:
        print("Bitbucket release evidence security_gates.gitleaks_findings is invalid", file=sys.stderr)
        print(f"  actual={gitleaks_findings!r}", file=sys.stderr)
        sys.exit(1)
    security_checks = security_gates.get("checks")
    if not isinstance(security_checks, list) or not security_checks:
        print("Bitbucket release evidence security_gates.checks are missing", file=sys.stderr)
        sys.exit(1)
    expected_security_checks = {
        "supply-chain-gate",
        "phi-log-lint",
        "secret-scan",
        "security-assurance-decision",
        "security-certification",
        "release-evidence-check",
        "ops-certification",
        "pds-security-baseline",
    }
    security_check_entries = [
        check for check in security_checks if isinstance(check, dict)
    ]
    if len(security_check_entries) != len(security_checks):
        print("Bitbucket release evidence security gate entries must be objects", file=sys.stderr)
        sys.exit(1)
    security_check_names = []
    invalid_security_check_names = []
    for index, check in enumerate(security_check_entries):
        check_name = check.get("name")
        if isinstance(check_name, str) and check_name.strip():
            security_check_names.append(check_name.strip())
        else:
            invalid_security_check_names.append(index)
    if invalid_security_check_names:
        print("Bitbucket release evidence security gate names are invalid", file=sys.stderr)
        print(f"  indexes={invalid_security_check_names}", file=sys.stderr)
        sys.exit(1)
    duplicate_security_checks = sorted(
        check_name
        for check_name in set(security_check_names)
        if security_check_names.count(check_name) > 1
    )
    if duplicate_security_checks:
        print("Bitbucket release evidence security gate entries are duplicated", file=sys.stderr)
        for check_name in duplicate_security_checks:
            print(f"  - {check_name}", file=sys.stderr)
        sys.exit(1)
    seen_security_checks = set(security_check_names)
    if seen_security_checks != expected_security_checks:
        print("Bitbucket release evidence security gate set is incomplete", file=sys.stderr)
        print(f"  expected={sorted(expected_security_checks)}", file=sys.stderr)
        print(f"  actual={sorted(seen_security_checks)}", file=sys.stderr)
        sys.exit(1)
    failing_security_checks = [
        check.get("name", f"#{index}")
        for index, check in enumerate(security_check_entries)
        if check.get("ok") is not True
    ]
    if failing_security_checks:
        print("Bitbucket release evidence security gates are not green", file=sys.stderr)
        for check_name in failing_security_checks:
            print(f"  - {check_name}", file=sys.stderr)
        sys.exit(1)
    release_ready_security_checks = {
        "security-assurance-decision",
        "security-certification",
        "release-evidence-check",
        "ops-certification",
        "pds-security-baseline",
    }
    not_ready_security_checks = [
        check.get("name", f"#{index}")
        for index, check in enumerate(security_check_entries)
        if check.get("name") in release_ready_security_checks
        and check.get("release_ready") is not True
    ]
    if not_ready_security_checks:
        print("Bitbucket release evidence security gates are not release-ready", file=sys.stderr)
        for check_name in not_ready_security_checks:
            print(f"  - {check_name}", file=sys.stderr)
        sys.exit(1)
    missing_live_security_checks = [
        check.get("name", f"#{index}")
        for index, check in enumerate(security_check_entries)
        if check.get("name") in {"ops-certification", "pds-security-baseline"}
        and check.get("live_evidence_required") is not True
    ]
    if missing_live_security_checks:
        print("Bitbucket release evidence security gates do not require live evidence", file=sys.stderr)
        for check_name in missing_live_security_checks:
            print(f"  - {check_name}", file=sys.stderr)
        sys.exit(1)
    require_static_security_evidence(gitleaks_findings)

    provider_certification = require_mapping(evidence, "provider_certification")
    require_true(provider_certification, "provider_certification", "ok")
    providers = provider_certification.get("providers")
    if not isinstance(providers, list) or not providers:
        print("Bitbucket release evidence provider_certification.providers are missing", file=sys.stderr)
        sys.exit(1)
    expected_providers = {"postgres", "mongo", "mssql", "snowflake"}
    provider_entries = [
        provider for provider in providers if isinstance(provider, dict)
    ]
    if len(provider_entries) != len(providers):
        print("Bitbucket release evidence provider entries must be objects", file=sys.stderr)
        sys.exit(1)
    provider_names = []
    invalid_provider_names = []
    for index, provider in enumerate(provider_entries):
        provider_name = provider.get("provider")
        if isinstance(provider_name, str) and provider_name.strip():
            provider_names.append(provider_name.strip())
        else:
            invalid_provider_names.append(index)
    if invalid_provider_names:
        print("Bitbucket release evidence provider names are invalid", file=sys.stderr)
        print(f"  indexes={invalid_provider_names}", file=sys.stderr)
        sys.exit(1)
    duplicate_providers = sorted(
        provider_name
        for provider_name in set(provider_names)
        if provider_names.count(provider_name) > 1
    )
    if duplicate_providers:
        print("Bitbucket release evidence provider entries are duplicated", file=sys.stderr)
        for provider_name in duplicate_providers:
            print(f"  - {provider_name}", file=sys.stderr)
        sys.exit(1)
    if len(provider_entries) != len(expected_providers):
        print("Bitbucket release evidence provider entry count is invalid", file=sys.stderr)
        print(f"  expected={len(expected_providers)}", file=sys.stderr)
        print(f"  actual={len(provider_entries)}", file=sys.stderr)
        sys.exit(1)
    seen_providers = set(provider_names)
    if seen_providers != expected_providers:
        print("Bitbucket release evidence provider set is incomplete", file=sys.stderr)
        print(f"  expected={sorted(expected_providers)}", file=sys.stderr)
        print(f"  actual={sorted(seen_providers)}", file=sys.stderr)
        sys.exit(1)
    failing_providers = [
        provider.get("provider", f"#{index}")
        for index, provider in enumerate(provider_entries)
        if provider.get("ok") is not True
    ]
    if failing_providers:
        print("Bitbucket release evidence provider certification is not green", file=sys.stderr)
        for provider in failing_providers:
            print(f"  - {provider}", file=sys.stderr)
        sys.exit(1)
    incomplete_provider_evidence = []
    for provider in provider_entries:
        provider_name = provider.get("provider")
        live_certified_total = provider.get("live_certified_total")
        live_certified_passed = provider.get("live_certified_passed")
        if (
            not isinstance(provider.get("data_source"), str)
            or not provider.get("data_source").strip()
            or not isinstance(provider.get("base_url"), str)
            or not provider.get("base_url").strip()
            or type(live_certified_total) is not int
            or live_certified_total <= 0
            or type(live_certified_passed) is not int
            or live_certified_passed != live_certified_total
        ):
            incomplete_provider_evidence.append(
                f"{provider_name}: live_certified_passed={live_certified_passed!r}; "
                f"live_certified_total={live_certified_total!r}; "
                f"data_source={provider.get('data_source')!r}; "
                f"base_url={provider.get('base_url')!r}"
            )
    if incomplete_provider_evidence:
        print("Bitbucket release evidence provider live certification is incomplete", file=sys.stderr)
        for provider in incomplete_provider_evidence:
            print(f"  - {provider}", file=sys.stderr)
        sys.exit(1)

    for name in ("ops_certification", "pds_security_baseline", "security_assurance_decision"):
        summary = require_mapping(evidence, name)
        require_true(summary, name, "ok")
        require_true(summary, name, "release_ready")
    require_true(evidence["ops_certification"], "ops_certification", "live_evidence_required")
    require_true(evidence["pds_security_baseline"], "pds_security_baseline", "live_evidence_required")
    require_security_assurance_decision_evidence()

    mcp_release_posture = require_mapping(evidence, "mcp_release_posture")
    require_true(mcp_release_posture, "mcp_release_posture", "ok")
    require_mcp_release_posture_evidence(mcp_release_posture)
    sys.exit(0)

print("Bitbucket release gate is not release-ready:", file=sys.stderr)
blockers = evidence.get("release_blockers")
if isinstance(blockers, list) and blockers:
    for blocker in blockers:
        if isinstance(blocker, str):
            print(f"  - {blocker}", file=sys.stderr)
else:
    print("  - bitbucket-release-gate ok/release_ready is not true", file=sys.stderr)
sys.exit(1)
PY
}

cleanup() {
  for pid_file in "$report_dir"/backend-*.pid; do
    [[ -f "$pid_file" ]] || continue
    pid="$(cat "$pid_file")"
    if [[ -n "$pid" ]]; then
      kill "$pid" >/dev/null 2>&1 || true
    fi
  done
}
trap cleanup EXIT

if env_flag_enabled "${APPFW_BITBUCKET_RELEASE_GATE_ASSERT_ONLY:-}"; then
  assert_release_gate_ready
  exit $?
fi

export APPFW_RELEASE_REQUIRE_SECURITY_ASSURANCE_DECISION="${APPFW_RELEASE_REQUIRE_SECURITY_ASSURANCE_DECISION:-true}"
export APPFW_RELEASE_REQUIRE_PRODUCTION_ATTESTATIONS="${APPFW_RELEASE_REQUIRE_PRODUCTION_ATTESTATIONS:-true}"
export APPFW_RELEASE_REQUIRE_LIVE_OPS_EVIDENCE="${APPFW_RELEASE_REQUIRE_LIVE_OPS_EVIDENCE:-true}"
export APPFW_RELEASE_REQUIRE_OPS_CERTIFICATION="${APPFW_RELEASE_REQUIRE_OPS_CERTIFICATION:-$APPFW_RELEASE_REQUIRE_LIVE_OPS_EVIDENCE}"
export APPFW_RELEASE_REQUIRE_PERFORMANCE_EVIDENCE="${APPFW_RELEASE_REQUIRE_PERFORMANCE_EVIDENCE:-true}"
export APPFW_RELEASE_REQUIRE_PDS_BASELINE_EVIDENCE="${APPFW_RELEASE_REQUIRE_PDS_BASELINE_EVIDENCE:-true}"
export APPFW_RELEASE_REQUIRE_RELEASE_IDENTITY="${APPFW_RELEASE_REQUIRE_RELEASE_IDENTITY:-true}"

run_stage_or_record_failure "snowflake-release-prereqs" require_snowflake_release_prereqs
run_stage_or_record_failure "install-ci-prereqs" install_ci_prereqs
append_host_aliases

export ENV_NAME="${ENV_NAME:-compose}"
export LOG_LEVEL="${LOG_LEVEL:-info}"
export API_TEST_AUTH_MODE="${API_TEST_AUTH_MODE:-local_dev}"
export APP_ENABLE_LOCAL_TEST_AUTH="${APP_ENABLE_LOCAL_TEST_AUTH:-true}"
export APP_PROVIDER_CERTIFICATION_CI="${APP_PROVIDER_CERTIFICATION_CI:-true}"
export APP_MCP_ENABLED="${APP_MCP_ENABLED:-false}"
export APPFW_MIGRATE_SKIP_SEED="${APPFW_MIGRATE_SKIP_SEED:-false}"
export APP_RATE_LIMIT_PER_SECOND="${APP_RATE_LIMIT_PER_SECOND:-10000}"
export APP_RATE_LIMIT_BURST="${APP_RATE_LIMIT_BURST:-10000}"
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

export SNOWFLAKE_HOST="${SNOWFLAKE_HOST:-http://snowflake.localhost.localstack.cloud:4566}"
export SNOWFLAKE_SERVICE_ACCOUNT_NAME="${SNOWFLAKE_SERVICE_ACCOUNT_NAME:-test}"
export SNOWFLAKE_ACCESS_TOKEN="${SNOWFLAKE_ACCESS_TOKEN:-test}"
export SNOWFLAKE_AUTH_TOKEN_TYPE="${SNOWFLAKE_AUTH_TOKEN_TYPE:-LOCALSTACK_NO_AUTH}"

export NEO4J_SERVICE_ACCOUNT_NAME="${NEO4J_SERVICE_ACCOUNT_NAME:-neo4j}"
export NEO4J_SERVICE_ACCOUNT_PASS="${NEO4J_SERVICE_ACCOUNT_PASS:-appfw-neo4j-local}"

log "Waiting for provider services"
run_stage_or_record_failure "wait-postgres" wait_for_tcp postgres 5432 PostgreSQL
run_stage_or_record_failure "wait-mongo" wait_for_tcp mongo 27017 MongoDB
run_stage_or_record_failure "wait-mssql" wait_for_tcp mssql 1433 "MS SQL Server"
run_stage_or_record_failure "wait-snowflake" wait_for_tcp snowflake.localhost.localstack.cloud 4566 "LocalStack Snowflake SQL API"
run_stage_or_record_failure "wait-snowflake-control-plane" wait_for_snowflake

run_stage_or_record_failure "migrate-providers" run_migrations_parallel

log "Building product frontend bundle"
run_stage_or_record_failure "build-product-frontend" build_product_frontend

log "Building backend once before launching provider instances"
run_stage_or_record_failure "build-backend" \
  env CARGO_TARGET_DIR="$release_backend_target_dir" \
  cargo build --locked --no-default-features --features "$backend_certification_features" --manifest-path "$product_app_root/backend/Cargo.toml" --bin backend

run_stage_or_record_failure "start-postgres-backend" start_backend postgres pg_primary 8081
run_stage_or_record_failure "start-mongo-backend" start_backend mongo mongo_primary 8082
run_stage_or_record_failure "start-mssql-backend" start_backend mssql mssql_primary 8083
run_stage_or_record_failure "start-snowflake-backend" start_backend snowflake snowflake_primary 8084

export API_TEST_BASE_URL_POSTGRES=http://127.0.0.1:8081
export API_TEST_BASE_URL_MONGO=http://127.0.0.1:8082
export API_TEST_BASE_URL_MSSQL=http://127.0.0.1:8083
export API_TEST_BASE_URL_SNOWFLAKE=http://127.0.0.1:8084
export APPFW_LOAD_TEST_URL="${APPFW_LOAD_TEST_URL:-$API_TEST_BASE_URL_POSTGRES/crm}"

if env_flag_enabled "${APPFW_RELEASE_REQUIRE_LIVE_OPS_EVIDENCE:-}"; then
  export APPFW_RELEASE_REQUIRE_OPS_CERTIFICATION=true
  export APPFW_OPS_BASE_URL="${APPFW_OPS_BASE_URL:-$API_TEST_BASE_URL_POSTGRES}"
  log "Collecting live operations evidence from $APPFW_OPS_BASE_URL"
  run_stage_or_record_failure "live-ops-evidence" bash "$script_dir/live-ops-evidence.sh"
fi

log "Running App Framework release gate"
set +e
scripts/appfw release-check --json | tee "$report_dir/release-check-output.json"
release_check_status=${PIPESTATUS[0]}
set -e
if [[ $release_check_status -ne 0 ]]; then
  write_release_gate_failure_evidence "release-check" "$release_check_status"
  exit "$release_check_status"
fi

if env_flag_enabled "${APPFW_BITBUCKET_RELEASE_GATE_FOCUSED_EVIDENCE:-}"; then
  mkdir -p api_tests/target
  run_stage_or_record_failure "copy-provider-parity-artifact" cp "$report_dir/provider-parity.json" api_tests/target/provider-parity.json
  run_stage_or_record_failure "write-focused-release-gate-evidence" write_focused_release_gate_evidence
  log "Focused release gate evidence passed"
  exit 0
fi

run_stage_or_record_failure "provider-certification-artifact" assert_provider_certification_artifact
run_stage_or_record_failure "security-assurance-decision" \
  env APPFW_RELEASE_REQUIRE_SECURITY_ASSURANCE_DECISION="$APPFW_RELEASE_REQUIRE_SECURITY_ASSURANCE_DECISION" \
  APPFW_RELEASE_REQUIRE_PRODUCTION_ATTESTATIONS="$APPFW_RELEASE_REQUIRE_PRODUCTION_ATTESTATIONS" \
  bash "$script_dir/security-assurance-decision.sh"
run_stage_or_record_failure "release-evidence-check" \
  env APPFW_RELEASE_REQUIRE_SECURITY_ASSURANCE_DECISION="$APPFW_RELEASE_REQUIRE_SECURITY_ASSURANCE_DECISION" \
  APPFW_RELEASE_REQUIRE_PRODUCTION_ATTESTATIONS="$APPFW_RELEASE_REQUIRE_PRODUCTION_ATTESTATIONS" \
  APPFW_RELEASE_REQUIRE_OPS_CERTIFICATION="${APPFW_RELEASE_REQUIRE_OPS_CERTIFICATION:-false}" \
  APPFW_RELEASE_REQUIRE_LIVE_OPS_EVIDENCE="${APPFW_RELEASE_REQUIRE_LIVE_OPS_EVIDENCE:-false}" \
  APPFW_RELEASE_REQUIRE_PDS_BASELINE_EVIDENCE="$APPFW_RELEASE_REQUIRE_PDS_BASELINE_EVIDENCE" \
  APPFW_RELEASE_REQUIRE_PERFORMANCE_EVIDENCE="$APPFW_RELEASE_REQUIRE_PERFORMANCE_EVIDENCE" \
  APPFW_RELEASE_REQUIRE_RELEASE_IDENTITY="$APPFW_RELEASE_REQUIRE_RELEASE_IDENTITY" \
  bash "$script_dir/release-evidence-check.sh"
run_stage_or_record_failure "copy-provider-parity-artifact" cp "$report_dir/provider-parity.json" api_tests/target/provider-parity.json
run_stage_or_record_failure "write-release-gate-evidence" write_release_gate_evidence
run_stage_or_record_failure "assert-release-ready" assert_release_gate_ready

log "Release gate passed"
