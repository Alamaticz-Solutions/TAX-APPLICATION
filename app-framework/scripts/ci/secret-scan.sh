#!/usr/bin/env bash
#
# secret-scan.sh — committed-secret scanner (gitleaks).
#
# Healthcare/PHI context: credentials, tokens, and connection strings must
# never be committed. This wraps gitleaks to scan the working tree and the
# repository history reachable from the current branch.
#
# gitleaks is installed on demand (pinned version) so the gate is self-contained
# on the CI image. If gitleaks is already on PATH it is reused.
#
# Usage:  bash scripts/ci/secret-scan.sh
#
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
cd "$repo_root"

GITLEAKS_VERSION="${GITLEAKS_VERSION:-8.21.2}"

log() {
  printf '\n[%s] %s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" "$*"
}

install_gitleaks() {
  if command -v gitleaks >/dev/null 2>&1; then
    log "gitleaks already installed: $(gitleaks version 2>/dev/null || true)"
    return 0
  fi

  local arch os tool_dir url tmp
  tool_dir="${APPFW_RELEASE_TOOL_DIR:-target/appfw-tools}"
  if [[ -x "$tool_dir/gitleaks" ]]; then
    PATH="$tool_dir:$PATH"
    export PATH
    log "gitleaks already installed: $(gitleaks version 2>/dev/null || true)"
    return 0
  fi

  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  case "$(uname -m)" in
    x86_64 | amd64) arch="x64" ;;
    aarch64 | arm64) arch="arm64" ;;
    *)
      echo "secret-scan: unsupported architecture $(uname -m)" >&2
      exit 2
      ;;
  esac

  url="https://github.com/gitleaks/gitleaks/releases/download/v${GITLEAKS_VERSION}/gitleaks_${GITLEAKS_VERSION}_${os}_${arch}.tar.gz"
  tmp="$(mktemp -d)"
  log "Installing gitleaks ${GITLEAKS_VERSION} from ${url}"
  # GitHub release downloads flake in CI (connection resets / 5xx). Retry hard
  # before failing the secret-scan gate on infrastructure noise.
  local attempt max_attempts=6
  for ((attempt = 1; attempt <= max_attempts; attempt++)); do
    if curl -fsSL \
      --connect-timeout 30 \
      --retry 8 \
      --retry-delay 3 \
      --retry-all-errors \
      --retry-max-time 180 \
      "$url" -o "$tmp/gitleaks.tar.gz"; then
      break
    fi
    if ((attempt == max_attempts)); then
      echo "secret-scan: failed to download gitleaks after ${max_attempts} attempts" >&2
      rm -rf "$tmp"
      exit 2
    fi
    log "gitleaks download failed (attempt ${attempt}/${max_attempts}); retrying in $((attempt * 5))s"
    sleep $((attempt * 5))
  done
  tar -xzf "$tmp/gitleaks.tar.gz" -C "$tmp" gitleaks
  mkdir -p "$tool_dir"
  install -m 0755 "$tmp/gitleaks" "$tool_dir/gitleaks"
  rm -rf "$tmp"
  PATH="$tool_dir:$PATH"
  export PATH
  log "gitleaks installed: $(gitleaks version 2>/dev/null || true)"
}

install_gitleaks

report_dir="${APPFW_RELEASE_ARTIFACT_DIR:-target/appfw}"
mkdir -p "$report_dir"
report_file="$report_dir/gitleaks-report.json"
evidence_file="$report_dir/secret-scan.json"
baseline_file="${APPFW_GITLEAKS_BASELINE:-scripts/ci/gitleaks-baseline.txt}"

count_report_findings() {
  if [[ ! -f "$report_file" ]]; then
    printf '0'
    return
  fi
  grep -c '"RuleID"' "$report_file" 2>/dev/null || true
}

summarize_baseline() {
  python3 - "$report_file" "$baseline_file" <<'PY'
import json
import sys
from pathlib import Path

report_path = Path(sys.argv[1])
baseline_path = Path(sys.argv[2])

allowed = set()
if baseline_path.is_file():
    for line in baseline_path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        allowed.add(line.split()[0])

if report_path.is_file():
    findings = json.loads(report_path.read_text(encoding="utf-8"))
else:
    findings = []

total = len(findings)
baselined = sum(1 for finding in findings if finding.get("Fingerprint") in allowed)
unbaselined = total - baselined
print(total, baselined, unbaselined, len(allowed))
PY
}

assert_report_redacted() {
  if [[ ! -f "$report_file" ]]; then
    return 0
  fi

  python3 - "$report_file" <<'PY'
import json
import sys
from pathlib import Path

findings = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
for finding in findings:
    if not isinstance(finding, dict) or "Secret" not in finding:
        continue
    secret = finding.get("Secret")
    if secret in (None, "", "REDACTED", "<redacted>"):
        continue
    if isinstance(secret, str) and secret and set(secret) == {"*"}:
        continue
    sys.exit(1)
PY
}

write_evidence() {
  # write_evidence <ok-json-bool> <gitleaks-exit-code> <redaction-ok-json-bool>
  local ok="$1"
  local scan_status="$2"
  local redaction_ok="$3"
  local baseline_entries baseline_present baselined_findings generated_at report_present scanner_version total_findings unbaselined_findings
  generated_at="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  read -r total_findings baselined_findings unbaselined_findings baseline_entries < <(summarize_baseline)
  scanner_version="$(gitleaks version 2>/dev/null || printf '%s' "$GITLEAKS_VERSION")"
  if [[ -f "$report_file" ]]; then
    report_present=true
  else
    report_present=false
  fi
  if [[ -f "$baseline_file" ]]; then
    baseline_present=true
  else
    baseline_present=false
  fi

  {
    printf '{\n'
    printf '  "command": "secret-scan",\n'
    printf '  "ok": %s,\n' "$ok"
    printf '  "generated_at_utc": "%s",\n' "$generated_at"
    printf '  "scanner": "gitleaks",\n'
    printf '  "scanner_version": "%s",\n' "$scanner_version"
    printf '  "requested_scanner_version": "%s",\n' "$GITLEAKS_VERSION"
    printf '  "gitleaks_exit_code": %s,\n' "$scan_status"
    printf '  "findings": %s,\n' "$unbaselined_findings"
    printf '  "total_findings": %s,\n' "$total_findings"
    printf '  "baselined_findings": %s,\n' "$baselined_findings"
    printf '  "history_scan": true,\n'
    printf '  "baseline": {\n'
    printf '    "path": "%s",\n' "$baseline_file"
    printf '    "present": %s,\n' "$baseline_present"
    printf '    "entries": %s,\n' "$baseline_entries"
    printf '    "unbaselined_findings": %s\n' "$unbaselined_findings"
    printf '  },\n'
    printf '  "artifacts": [\n'
    printf '    {\n'
    printf '      "name": "gitleaks-report",\n'
    printf '      "path": "%s",\n' "$report_file"
    printf '      "required": true,\n'
    printf '      "present": %s\n' "$report_present"
    printf '    },\n'
    printf '    {\n'
    printf '      "name": "gitleaks-baseline",\n'
    printf '      "path": "%s",\n' "$baseline_file"
    printf '      "required": false,\n'
    printf '      "present": %s\n' "$baseline_present"
    printf '    }\n'
    printf '  ],\n'
    printf '  "redaction": {\n'
    printf '    "gitleaks_redact_flag": true,\n'
    printf '    "secret_fields_checked": true,\n'
    printf '    "secret_fields_redacted": %s\n' "$redaction_ok"
    printf '  }\n'
    printf '}\n'
  } >"$evidence_file"
}

# `detect` scans git history; `--no-git` would scan only the working tree.
# We scan history so rotated-but-committed secrets are still caught.
log "Running gitleaks detect"
set +e
gitleaks detect \
  --source "$repo_root" \
  --log-opts="--full-history HEAD" \
  --redact \
  --report-format json \
  --report-path "$report_file" \
  --exit-code 1 \
  --verbose
scan_status=$?
set -e

if [[ ! -f "$report_file" ]]; then
  printf '[]\n' >"$report_file"
fi

redaction_ok=true
if ! assert_report_redacted; then
  echo "secret-scan: gitleaks report contains unredacted Secret fields" >&2
  redaction_ok=false
fi

read -r total_findings baselined_findings unbaselined_findings baseline_entries < <(summarize_baseline)
if [[ "$unbaselined_findings" -eq 0 && "$redaction_ok" == "true" && "$scan_status" -le 1 ]]; then
  write_evidence true "$scan_status" "$redaction_ok"
  log "Secret scan passed; report at $report_file; evidence at $evidence_file; baselined findings: $baselined_findings; unbaselined findings: $unbaselined_findings"
  exit 0
fi

write_evidence false "$scan_status" "$redaction_ok"
log "Secret scan failed; report at $report_file; evidence at $evidence_file"

if [[ "$redaction_ok" != "true" ]]; then
  exit 1
fi
exit "$scan_status"
