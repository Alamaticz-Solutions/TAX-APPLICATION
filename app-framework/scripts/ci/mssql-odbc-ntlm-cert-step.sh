#!/usr/bin/env bash
# Bitbucket pipeline MS SQL ODBC certification (no docker/mssql-ad compose):
# - sql_password_odbc_e2e against pipeline mssql service + Microsoft ODBC Driver 18
# - ntlm_e2e + ntlm_wrong_password_fails_fast against in-process tds-mock on 127.0.0.1:1434
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
evidence_dir="$repo_root/target/appfw"
tds_mock_dir="$repo_root/docker/mssql-ad/tds-mock"
golden_path="$repo_root/docker/mssql-ad/tds-mock/golden/handshake.json"

log() { printf 'mssql-odbc-ntlm-cert-step: %s\n' "$*"; }

wait_for_tcp() {
  local host="$1"
  local port="$2"
  local label="$3"
  local attempts="${4:-60}"
  local delay="${5:-2}"
  local attempt=1
  while [[ "$attempt" -le "$attempts" ]]; do
    if (echo >/dev/tcp/"$host"/"$port") >/dev/null 2>&1; then
      log "$label ready at ${host}:${port}"
      return 0
    fi
    sleep "$delay"
    attempt=$((attempt + 1))
  done
  log "timeout waiting for $label at ${host}:${port}"
  return 1
}

install_odbc_drivers() {
  log "Installing unixODBC, FreeTDS, and Microsoft ODBC Driver 18"
  apt-get update
  apt-get install -y --no-install-recommends \
    ca-certificates curl gnupg unixodbc unixodbc-dev \
    tdsodbc freetds-bin freetds-dev freetds-common python3-pip
  curl -fsSL https://packages.microsoft.com/keys/microsoft.asc \
    | gpg --dearmor -o /usr/share/keyrings/microsoft-prod.gpg
  echo "deb [arch=amd64 signed-by=/usr/share/keyrings/microsoft-prod.gpg] https://packages.microsoft.com/debian/12/prod bookworm main" \
    > /etc/apt/sources.list.d/mssql-release.list
  apt-get update
  ACCEPT_EULA=Y apt-get install -y --no-install-recommends msodbcsql18
  rm -rf /var/lib/apt/lists/*

  local ms_driver_path
  ms_driver_path="$(ls /opt/microsoft/msodbcsql18/lib64/libmsodbcsql-18*.so.* 2>/dev/null | head -1)"
  printf '%s\n' \
    '[FreeTDS]' \
    'Description=FreeTDS Driver' \
    'Driver=/usr/lib/x86_64-linux-gnu/odbc/libtdsodbc.so' \
    'Setup=/usr/lib/x86_64-linux-gnu/odbc/libtdsodbc.so' \
    'UsageCount=1' \
    '' \
    '[ODBC Driver 18 for SQL Server]' \
    "Description=Microsoft ODBC Driver 18 for SQL Server" \
    "Driver=${ms_driver_path}" \
    'UsageCount=1' \
    > /etc/odbcinst.ini
}

configure_openssl_legacy_md4() {
  if grep -q 'legacy_sect' /etc/ssl/openssl.cnf 2>/dev/null; then
    return 0
  fi
  log "Enabling OpenSSL legacy provider for ntlm-auth MD4"
  printf '\nopenssl_conf = openssl_init\n\n[openssl_init]\nproviders = provider_sect\n\n[provider_sect]\ndefault = default_sect\nlegacy = legacy_sect\n\n[default_sect]\nactivate = 1\n\n[legacy_sect]\nactivate = 1\n' \
    >> /etc/ssl/openssl.cnf
  export OPENSSL_CONF=/etc/ssl/openssl.cnf
}

tds_mock_pid=""

cleanup() {
  if [[ -n "$tds_mock_pid" ]]; then
    kill "$tds_mock_pid" >/dev/null 2>&1 || true
    wait "$tds_mock_pid" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

cd "$repo_root"
mkdir -p "$evidence_dir"

if [[ "${SKIP_ODBC_DRIVER_INSTALL:-0}" != "1" ]]; then
  install_odbc_drivers
fi

configure_openssl_legacy_md4

log "Installing tds-mock Python dependencies"
pip3 install --break-system-packages -r "$tds_mock_dir/requirements.txt"

log "Running tds-mock golden self-test"
cd "$tds_mock_dir"
python3 golden_selftest.py "$golden_path"

log "Starting in-process tds-mock on 127.0.0.1:1434"
export TDS_MOCK_HOST=127.0.0.1
export TDS_MOCK_PORT=1434
export TDS_MOCK_EVIDENCE="$evidence_dir/ntlm-handshake.json"
python3 server.py &
tds_mock_pid=$!
wait_for_tcp 127.0.0.1 1434 "tds-mock"

cd "$repo_root"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target/cargo}"

export MSSQL_ODBC_HOST="${MSSQL_ODBC_HOST:-mssql}"
export MSSQL_ODBC_PORT="${MSSQL_ODBC_PORT:-1433}"
export MSSQL_ODBC_DATABASE="${MSSQL_ODBC_DATABASE:-master}"
export MSSQL_ODBC_USERNAME="${MSSQL_ODBC_USERNAME:-sa}"
export MSSQL_ODBC_PASSWORD="${MSSQL_ODBC_PASSWORD:-YourStrong!Passw0rd}"

export MSSQL_NTLM_HOST="${MSSQL_NTLM_HOST:-127.0.0.1}"
export MSSQL_NTLM_PORT="${MSSQL_NTLM_PORT:-1434}"
export MSSQL_NTLM_DATABASE="${MSSQL_NTLM_DATABASE:-master}"
export MSSQL_NTLM_USERNAME="${MSSQL_NTLM_USERNAME:-APPFW\\svc-app}"
export MSSQL_NTLM_PASSWORD="${MSSQL_NTLM_PASSWORD:-AppSvc!Passw0rd}"

log "Waiting for pipeline mssql service at ${MSSQL_ODBC_HOST}:${MSSQL_ODBC_PORT}"
wait_for_tcp "$MSSQL_ODBC_HOST" "$MSSQL_ODBC_PORT" "mssql"

log "Running sql_password_odbc_e2e against ${MSSQL_ODBC_HOST}:${MSSQL_ODBC_PORT}"
cargo test -p appfw-provider-mssql --test sql_password_odbc_e2e -- --ignored --nocapture

log "Running ntlm_e2e and ntlm_wrong_password_fails_fast against ${MSSQL_NTLM_HOST}:${MSSQL_NTLM_PORT}"
cargo test -p appfw-provider-mssql --test ntlm_e2e -- --ignored --nocapture --test-threads=1

if [[ -f "$evidence_dir/ntlm-handshake.json" ]]; then
  log "evidence: $evidence_dir/ntlm-handshake.json"
else
  log "warning: tds-mock evidence file missing"
fi

log "mssql-odbc-ntlm certification step passed"
