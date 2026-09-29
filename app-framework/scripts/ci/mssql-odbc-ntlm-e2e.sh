#!/usr/bin/env bash
# Local MS SQL ODBC certification lab (docker/mssql-ad compose + test-runner):
# - sql_password_odbc_e2e against real Linux SQL Server + Microsoft ODBC Driver 18
# - ntlm_e2e + ntlm_wrong_password_fails_fast against tds-mock (NTLMv2 via ntlm-auth)
# Bitbucket CI uses scripts/ci/mssql-odbc-ntlm-cert-step.sh (pipeline mssql service +
# in-process tds-mock; no custom compose).
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
compose_file="$repo_root/docker/mssql-ad/docker-compose.yml"
evidence_dir="$repo_root/target/appfw"

log() { printf 'mssql-odbc-ntlm-e2e: %s\n' "$*"; }

export DOCKER_CONFIG="${DOCKER_CONFIG:-/tmp/empty-docker-config-mssql-ad}"
mkdir -p "$DOCKER_CONFIG" "$evidence_dir"

ensure_docker_compose() {
  if docker compose version >/dev/null 2>&1; then
    return 0
  fi
  local plugin_dir="$DOCKER_CONFIG/cli-plugins"
  local plugin="$plugin_dir/docker-compose"
  local version="${APPFW_DOCKER_COMPOSE_VERSION:-v2.29.7}"
  mkdir -p "$plugin_dir"
  log "docker compose plugin missing; installing ${version} into ${plugin_dir}"
  curl -fsSL \
    "https://github.com/docker/compose/releases/download/${version}/docker-compose-linux-x86_64" \
    -o "$plugin"
  chmod +x "$plugin"
  docker compose version >/dev/null
}

compose() {
  docker compose -f "$compose_file" "$@"
}

cd "$repo_root/docker/mssql-ad"

if [[ ! -f .env ]]; then
  cp .env.example .env
  log "seeded docker/mssql-ad/.env from .env.example (throwaway lab credentials)"
elif ! grep -q '^MSSQL_SA_PASSWORD=' .env 2>/dev/null; then
  cp .env.example .env
  log "refreshed docker/mssql-ad/.env from .env.example (MSSQL_SA_PASSWORD required)"
fi

ensure_docker_compose

cleanup() {
  compose down -v --remove-orphans >/dev/null 2>&1 || true
}
trap cleanup EXIT

log "Building and starting mssql + tds-mock stack"
compose down -v --remove-orphans >/dev/null 2>&1 || true
compose up -d --build --wait mssql tds-mock

log "Running tds-mock golden self-test"
compose exec -T tds-mock python golden_selftest.py /golden/handshake.json

log "Building test-runner image (Microsoft ODBC Driver 18 + FreeTDS)"
compose build test-runner

log "Running sql_password_odbc_e2e, ntlm_e2e, and ntlm_wrong_password_fails_fast"
compose --profile test run --rm --no-deps test-runner \
  bash -c '
    set -euo pipefail
    export PATH="/usr/local/cargo/bin:/usr/local/rustup/toolchains/1.95.0-x86_64-unknown-linux-gnu/bin:${PATH:-/usr/bin:/bin}"
    export CARGO_HOME="${CARGO_HOME:-/usr/local/cargo}"
    export RUSTUP_HOME="${RUSTUP_HOME:-/usr/local/rustup}"
    command -v cargo >/dev/null
    cargo test -p appfw-provider-mssql --test sql_password_odbc_e2e -- --ignored --nocapture
    cargo test -p appfw-provider-mssql --test ntlm_e2e -- --ignored --nocapture --test-threads=1
  '

log "Retaining tds-mock NTLM handshake evidence"
if compose exec -T tds-mock test -f /evidence/ntlm-handshake.json; then
  compose exec -T tds-mock cat /evidence/ntlm-handshake.json \
    > "$evidence_dir/ntlm-handshake.json"
  log "evidence: $evidence_dir/ntlm-handshake.json"
else
  log "warning: tds-mock evidence file missing (mock may not have completed a successful handshake)"
fi

log "mssql-odbc-ntlm e2e passed"
