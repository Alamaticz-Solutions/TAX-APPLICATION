#!/usr/bin/env bash
# Scheduled/manual NTLM live smoke against a real Windows SQL Server target.
# NOT part of the isolated certification gate — requires env-provided credentials.
#
# Launch analogues for product modes c / d (same FreeTDS NTLM path):
#   c (native workstation):  bash scripts/ci/mssql-ntlm-live-smoke.sh
#   d (Linux container):     MSSQL_NTLM_SMOKE_LAUNCH=container bash scripts/ci/mssql-ntlm-live-smoke.sh
#
# Optional: MSSQL_NTLM_SQL overrides the default SELECT 1 AS ok (printed with --nocapture).
# Optional: MSSQL_NTLM_ENV_FILE=/path/to/env to source credentials (KEY=value lines).
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

log() { printf 'mssql-ntlm-live-smoke: %s\n' "$*"; }

if [[ -n "${MSSQL_NTLM_ENV_FILE:-}" ]]; then
  if [[ ! -f "${MSSQL_NTLM_ENV_FILE}" ]]; then
    log "MSSQL_NTLM_ENV_FILE not found: ${MSSQL_NTLM_ENV_FILE}"
    exit 1
  fi
  log "sourcing credentials from ${MSSQL_NTLM_ENV_FILE}"
  set -a
  # shellcheck disable=SC1090
  source "${MSSQL_NTLM_ENV_FILE}"
  set +a
fi

required_vars=(
  MSSQL_NTLM_HOST
  MSSQL_NTLM_USERNAME
  MSSQL_NTLM_PASSWORD
)

for var in "${required_vars[@]}"; do
  if [[ -z "${!var:-}" ]]; then
    log "missing required env: $var"
    exit 1
  fi
done

export MSSQL_NTLM_PORT="${MSSQL_NTLM_PORT:-1433}"
export MSSQL_NTLM_DATABASE="${MSSQL_NTLM_DATABASE:-master}"
export APP_MSSQL_LOGIN_TIMEOUT_SECS="${APP_MSSQL_LOGIN_TIMEOUT_SECS:-30}"
launch="${MSSQL_NTLM_SMOKE_LAUNCH:-native}"
sql_display="${MSSQL_NTLM_SQL:-SELECT 1 AS ok}"

# Docker Desktop DNS often lacks WSL search domains, so short names like
# DC1AGL01D05 fail inside the container. Resolve on the host first.
resolve_host_for_container() {
  local host="$1"
  local ip
  ip="$(getent ahostsv4 "$host" 2>/dev/null | awk '{print $1; exit}')"
  if [[ -n "${ip}" ]]; then
    printf '%s\n' "${ip}"
    return 0
  fi
  # Fall back to FQDN candidates when search domain is missing in Docker DNS.
  local candidate
  for candidate in "${host}.pdsi.corp" "${host^^}.pdsi.corp" "${host,,}.pdsi.corp"; do
    ip="$(getent ahostsv4 "$candidate" 2>/dev/null | awk '{print $1; exit}')"
    if [[ -n "${ip}" ]]; then
      printf '%s\n' "${ip}"
      return 0
    fi
  done
  printf '%s\n' "${host}"
}

log "running ntlm_e2e against ${MSSQL_NTLM_HOST}:${MSSQL_NTLM_PORT} as ${MSSQL_NTLM_USERNAME}"
log "SQL: ${sql_display}"
log "launch: ${launch}"

run_cargo_test() {
  cargo test -p appfw-provider-mssql --test ntlm_e2e ntlm_connect_and_query -- --ignored --nocapture
}

case "${launch}" in
  native)
    cd "$repo_root"
    run_cargo_test
    ;;
  container)
    # Same FreeTDS + unixODBC stack as the hermetic test-runner (mode d packaging).
    # Use docker run --network host (compose run does not accept --network).
    docker_config="${DOCKER_CONFIG:-/tmp/empty-docker-config}"
    mkdir -p "$docker_config"
    image="${MSSQL_NTLM_SMOKE_IMAGE:-appfw-mssql-ntlm-smoke:local}"
    resolved_host="$(resolve_host_for_container "${MSSQL_NTLM_HOST}")"
    if [[ "${resolved_host}" != "${MSSQL_NTLM_HOST}" ]]; then
      log "resolved ${MSSQL_NTLM_HOST} -> ${resolved_host} for container DNS"
      export MSSQL_NTLM_HOST="${resolved_host}"
    fi
    log "building smoke image ${image}"
    DOCKER_CONFIG="$docker_config" docker build \
      -f "$repo_root/docker/mssql-ad/test-runner.Dockerfile" \
      -t "$image" \
      "$repo_root"
    env_args=(
      -e MSSQL_NTLM_HOST
      -e MSSQL_NTLM_PORT
      -e MSSQL_NTLM_DATABASE
      -e MSSQL_NTLM_USERNAME
      -e MSSQL_NTLM_PASSWORD
      -e APP_MSSQL_LOGIN_TIMEOUT_SECS
    )
    if [[ -n "${MSSQL_NTLM_SQL:-}" ]]; then
      env_args+=(-e MSSQL_NTLM_SQL)
    fi
    DOCKER_CONFIG="$docker_config" docker run --rm --network host \
      -v "$repo_root:/src" \
      -w /src \
      "${env_args[@]}" \
      "$image" \
      bash -c 'export PATH="/usr/local/cargo/bin:${PATH:-}"; cargo test -p appfw-provider-mssql --test ntlm_e2e ntlm_connect_and_query -- --ignored --nocapture'
    ;;
  *)
    log "unsupported MSSQL_NTLM_SMOKE_LAUNCH=${launch} (use native or container)"
    exit 1
    ;;
esac

log "mssql-ntlm live smoke passed"
