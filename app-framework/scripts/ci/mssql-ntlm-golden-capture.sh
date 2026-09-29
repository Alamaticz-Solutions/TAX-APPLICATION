#!/usr/bin/env bash
# Capture golden NTLM handshake vectors from a real Windows SQL Server via TDSDUMP.
# Requires throwaway domain credentials and FreeTDS/isql in the environment.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
dump_dir="${TMPDIR:-/tmp}/mssql-ntlm-golden-$$"
dump_file="$dump_dir/tdsdump.log"
golden_out="${1:-$repo_root/docker/mssql-ad/tds-mock/golden/handshake.json}"

log() { printf 'mssql-ntlm-golden-capture: %s\n' "$*"; }

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
export TDSDUMP="$dump_file"

mkdir -p "$dump_dir" "$(dirname "$golden_out")"

log "capturing TDSDUMP to $dump_file"
set +e
isql -v \
  -S "${MSSQL_NTLM_HOST},${MSSQL_NTLM_PORT}" \
  -D "${MSSQL_NTLM_DATABASE}" \
  -U "${MSSQL_NTLM_USERNAME}" \
  -P "${MSSQL_NTLM_PASSWORD}" \
  <<< 'SELECT 1' >/dev/null 2>&1
set -e

if [[ ! -s "$dump_file" ]]; then
  log "TDSDUMP empty — ensure FreeTDS isql is installed and TDSDUMP is honored"
  exit 1
fi

log "extracting sanitized golden vector to $golden_out"
python3 "$repo_root/docker/mssql-ad/tds-mock/tools/extract_golden.py" \
  "$dump_file" \
  --output "$golden_out"

log "golden capture complete (credentials stripped from $golden_out)"
