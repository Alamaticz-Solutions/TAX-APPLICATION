#!/usr/bin/env bash
#
# phi-log-lint.sh — best-effort guard against PHI/PII leaking into logs.
#
# Healthcare/PHI context: the framework must never emit Protected Health
# Information, credentials, or other sensitive values into stdout/stderr or the
# structured logging pipeline. This lint scans first-party Rust source for the
# two highest-signal mistakes:
#
#   1. A logging / print macro that INTERPOLATES a sensitive identifier
#      (e.g.  info!("token={}", access_token)  or  println!("{patient_ssn}")).
#   2. Use of the `dbg!` macro, which dumps arbitrary values to stderr and must
#      never reach production code.
#
# It deliberately does NOT ban logging the *word* "token"/"patient" in a static
# message (those are common and safe) — only the interpolation of a value whose
# identifier looks sensitive. This keeps false positives low while catching the
# realistic leak patterns.
#
# WHAT IS SCANNED
#   First-party crates only. Generated output, build artifacts, vendored
#   dependencies, sibling agent worktrees, and example/product trees are
#   excluded (see EXCLUDE_DIRS). Test files are scanned too, since test fixtures
#   are a common place real-looking PHI sneaks in.
#
# ALLOWLIST
#   Specific source lines that are known-safe (e.g. a literal message that
#   merely contains a sensitive word, or an intentional redaction helper) can be
#   suppressed with an inline marker:
#
#       warn!("rotating service token");   // phi-log-lint:allow reason=static-message
#
#   Lines containing the marker `phi-log-lint:allow` are skipped. Always include
#   a `reason=` so the suppression is auditable.
#
# EXIT CODES
#   0  no findings (or all findings allowlisted)
#   1  one or more potential PHI/PII log leaks found
#   2  usage / environment error
#
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
cd "$repo_root"

# --- configuration --------------------------------------------------------

# Logging / output macros whose arguments must not contain PHI values.
LOG_MACROS='println!|eprintln!|print!|eprint!|info!|warn!|error!|debug!|trace!'

# Sensitive identifier fragments. A match is only a finding when one of these
# appears *interpolated* (inside `{...}`) or *dot-accessed* (`.field`) within a
# logging macro invocation — not when it merely appears in a static string.
SENSITIVE='password|passwd|secret|api_key|apikey|access_token|refresh_token|bearer|client_secret|private_key|ssn|social_security|date_of_birth|patient_name|patient_id|medical_record|\bmrn\b|\bphi\b|\bdob\b|credit_card|card_number|cvv'

# Directories that are NOT first-party source under review.
EXCLUDE_DIRS=(
  './target'
  './.git'
  './.claude'        # sibling agent worktrees live here
  './examples'       # generated example/product trees
  './app_gen/target'
)

# Inline suppression marker.
ALLOW_MARKER='phi-log-lint:allow'

report_dir="${APPFW_RELEASE_ARTIFACT_DIR:-target/appfw}"
report_file="$report_dir/phi-log-lint.json"
mkdir -p "$report_dir"

# --- collect candidate source files ---------------------------------------

prune_args=()
for dir in "${EXCLUDE_DIRS[@]}"; do
  prune_args+=(-path "$dir" -prune -o)
done

# Collect candidate files portably (bash 3.2 on macOS has no `mapfile`).
source_files=()
while IFS= read -r src_file; do
  source_files+=("$src_file")
done < <(find . "${prune_args[@]}" -name '*.rs' -type f -print | sort)

findings=0

write_report() {
  # write_report <ok-json-bool> <failure-reason-or-empty>
  local ok="$1"
  local failure_reason="$2"
  local generated_at
  generated_at="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"

  {
    printf '{\n'
    printf '  "command": "phi-log-lint",\n'
    printf '  "ok": %s,\n' "$ok"
    printf '  "generated_at_utc": "%s",\n' "$generated_at"
    if [[ -n "$failure_reason" ]]; then
      printf '  "failure_reason": "%s",\n' "$failure_reason"
    else
      printf '  "failure_reason": null,\n'
    fi
    printf '  "scanned_files": %s,\n' "${#source_files[@]}"
    printf '  "findings": %s,\n' "$findings"
    printf '  "redaction": {\n'
    printf '    "json_source_excerpt_retained": false,\n'
    printf '    "json_finding_values_retained": false\n'
    printf '  },\n'
    printf '  "allow_marker": "%s"\n' "$ALLOW_MARKER"
    printf '}\n'
  } >"$report_file"
}

if [[ "${#source_files[@]}" -eq 0 ]]; then
  write_report false "no_rust_source_files"
  echo "phi-log-lint: no Rust source files found under $repo_root" >&2
  echo "phi-log-lint: evidence written to $report_file" >&2
  exit 2
fi

# --- scan -------------------------------------------------------------------

emit() {
  # emit <kind> <file:line> <text>
  printf '  [%s] %s\n      %s\n' "$1" "$2" "$3" >&2
  findings=$((findings + 1))
}

# report_match <kind> <file> <grep-n-line>
# grep -n on a SINGLE file emits "LINENO:TEXT" (no filename prefix).
report_match() {
  local kind="$1" file="$2" match="$3"
  [[ -z "$match" ]] && return 0
  [[ "$match" == *"$ALLOW_MARKER"* ]] && return 0
  local lineno text
  lineno="${match%%:*}"
  text="${match#*:}"
  text="${text#"${text%%[![:space:]]*}"}"   # left-trim leading whitespace
  emit "$kind" "${file}:${lineno}" "$text"
}

for file in "${source_files[@]}"; do
  # 1) dbg!(...) anywhere in first-party source.
  while IFS= read -r match; do
    report_match "dbg-macro" "$file" "$match"
  done < <(grep -nE '\bdbg!' "$file" || true)

  # 2) A logging macro that exposes a sensitive identifier, via:
  #      - brace interpolation:   info!("{access_token}")  warn!("{self.password}")
  #      - dot-accessed field:    error!("{}", record.ssn)
  #      - bare positional arg:    info!("token={}", access_token)
  while IFS= read -r match; do
    report_match "sensitive-interpolation" "$file" "$match"
  done < <(
    grep -nE "(${LOG_MACROS})" "$file" \
      | grep -iE "(\{[^}]*(${SENSITIVE})[^}]*\}|[.,[:space:](]+(${SENSITIVE})\b)" \
      || true
  )
done

# --- report -----------------------------------------------------------------

if [[ "$findings" -gt 0 ]]; then
  write_report false "potential_phi_log_leaks"
  echo "" >&2
  echo "phi-log-lint: FAILED — ${findings} potential PHI/PII log leak(s) found." >&2
  echo "phi-log-lint: review the lines above. If a line is provably safe, append" >&2
  echo "phi-log-lint: '${ALLOW_MARKER} reason=<why>' to it and re-run." >&2
  echo "phi-log-lint: evidence written to $report_file" >&2
  exit 1
fi

write_report true ""
echo "phi-log-lint: OK — scanned ${#source_files[@]} first-party Rust file(s); no PHI/PII log leaks detected; evidence at $report_file."
exit 0
