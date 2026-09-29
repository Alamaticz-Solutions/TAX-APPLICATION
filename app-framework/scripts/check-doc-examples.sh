#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"
report_dir="$repo_root/target/appfw"
mkdir -p "$report_dir"
cargo_target_dir="${CARGO_TARGET_DIR:-$repo_root/target}"
if [[ "$cargo_target_dir" != /* ]]; then
  cargo_target_dir="$repo_root/$cargo_target_dir"
fi
cargo_debug_dir="$cargo_target_dir/debug"

json=0
mode="${APPFW_DOCS_CHECK_MODE:-full}"
progress=0
plan_only=0
focused_subcheck="${APPFW_DOCS_CHECK_SUBCHECK:-}"
base_ref="${APPFW_DOCS_CHECK_BASE:-origin/main}"
enforce_budget="${APPFW_DOCS_CHECK_ENFORCE_BUDGET:-0}"
budget_override_ms="${APPFW_DOCS_CHECK_BUDGET_MS:-}"
fast_budget_ms="${APPFW_DOCS_CHECK_FAST_BUDGET_MS:-15000}"
full_budget_ms="${APPFW_DOCS_CHECK_FULL_BUDGET_MS:-60000}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --json)
      json=1
      ;;
    --full)
      mode="full"
      ;;
    --fast)
      mode="fast"
      ;;
    --changed-only)
      mode="changed-only"
      ;;
    --progress)
      progress=1
      ;;
    --plan)
      plan_only=1
      ;;
    --subcheck)
      shift || true
      if [[ $# -eq 0 ]]; then
        echo "--subcheck requires a docs-check subcheck name" >&2
        exit 2
      fi
      focused_subcheck="$1"
      ;;
    --enforce-budget)
      enforce_budget=1
      ;;
    --budget-ms)
      shift || true
      if [[ $# -eq 0 ]]; then
        echo "--budget-ms requires a millisecond value" >&2
        exit 2
      fi
      budget_override_ms="$1"
      ;;
    --base)
      shift || true
      if [[ $# -eq 0 ]]; then
        echo "--base requires a git ref" >&2
        exit 2
      fi
      base_ref="$1"
      ;;
    --help|-h)
      cat <<'EOF'
Usage: scripts/check-doc-examples.sh [--json] [--full|--fast|--changed-only] [--progress] [--plan] [--subcheck <name>] [--base <ref>] [--enforce-budget] [--budget-ms <ms>]

Runs the agent-facing CLI examples that are documented as safe, read-only, or
local-artifact-only commands. Provider-backed and long-running examples remain
covered by release-check, provider-test, api-test, migrate, serve, and load-test.

Modes:
  --full          Run the complete docs/example/maintainability gate. Default.
  --fast          Run static docs, shell, IA, security env, and design-system checks.
  --changed-only  Select --fast unless the changed surface affects CLI/generator
                  behavior, docs-check itself, agent skills, release CI behavior,
                  or command contracts. When only ordinary Product Increment plan
                  records changed, select the focused product-increment-delivery
                  subcheck instead of the full docs gate.

Use --progress with --json to stream JSONL progress events to stderr while the
final machine-readable report remains on stdout.

Use --plan to report the selected mode without running checks.

Use --subcheck <name> for a focused local loop over one retained docs-check
subcheck group. This is opt-in and does not narrow default, CI, or release
coverage.

Use --enforce-budget to fail when the selected mode exceeds its timing budget.
Set APPFW_DOCS_CHECK_FAST_BUDGET_MS or APPFW_DOCS_CHECK_FULL_BUDGET_MS to tune
the selected-mode budgets. Set APPFW_DOCS_CHECK_BUDGET_MS or pass --budget-ms
to override both selected-mode budgets for a single run.
EOF
      exit 0
      ;;
    *)
      echo "Unknown docs example option: $1" >&2
      exit 2
      ;;
  esac
  shift || true
done

case "$mode" in
  full|fast|changed-only)
    ;;
  *)
    echo "Unknown docs-check mode: $mode" >&2
    exit 2
    ;;
esac

case "$enforce_budget" in
  1|true|TRUE|yes|YES)
    enforce_budget=1
    ;;
  *)
    enforce_budget=0
    ;;
esac

if [[ -n "$budget_override_ms" && ! "$budget_override_ms" =~ ^[0-9]+$ ]]; then
  echo "--budget-ms/APPFW_DOCS_CHECK_BUDGET_MS must be an integer millisecond value" >&2
  exit 2
fi
if [[ -n "$fast_budget_ms" && ! "$fast_budget_ms" =~ ^[0-9]+$ ]]; then
  echo "APPFW_DOCS_CHECK_FAST_BUDGET_MS must be an integer millisecond value" >&2
  exit 2
fi
if [[ -n "$full_budget_ms" && ! "$full_budget_ms" =~ ^[0-9]+$ ]]; then
  echo "APPFW_DOCS_CHECK_FULL_BUDGET_MS must be an integer millisecond value" >&2
  exit 2
fi

json_escape() {
  local value="$1"
  value="${value//\\/\\\\}"
  value="${value//\"/\\\"}"
  value="${value//$'\n'/\\n}"
  value="${value//$'\r'/\\r}"
  value="${value//$'\t'/\\t}"
  printf '"%s"' "$value"
}

docs_check_subcheck_known() {
  case "$1" in
    shell-syntax|lifecycle-evidence|design-system|security-env-doc-parity|maintainability-contract|saas-vendor-doc-parity|release-lite-guard|sra-package-command-examples|product-increment-delivery|\
core-bootstrap-command-examples|core-generation-command-examples|core-generated-drift-command-examples|core-explain-profile-command-examples|\
intake-skills-command-examples|agent-governance-command-examples|agent-governance-core-command-examples|agent-governance-chat-command-examples|agent-governance-write-command-examples|\
agent-governance-mobile-command-examples|agent-governance-runtime-command-examples|agent-governance-release-command-examples|\
packaging-dependency-command-examples|compiled-cli-command-examples)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

docs_check_subcheck_requires_full() {
  case "$1" in
    shell-syntax|lifecycle-evidence|design-system|security-env-doc-parity|maintainability-contract|saas-vendor-doc-parity|release-lite-guard|sra-package-command-examples|product-increment-delivery)
      return 1
      ;;
    core-bootstrap-command-examples|core-generation-command-examples|core-generated-drift-command-examples|core-explain-profile-command-examples|\
intake-skills-command-examples|agent-governance-command-examples|agent-governance-core-command-examples|agent-governance-chat-command-examples|agent-governance-write-command-examples|\
agent-governance-mobile-command-examples|agent-governance-runtime-command-examples|agent-governance-release-command-examples|\
packaging-dependency-command-examples|compiled-cli-command-examples)
      return 0
      ;;
    *)
      return 2
      ;;
  esac
}

print_docs_check_subcheck_focus_json_fields() {
  printf ',"focused_subcheck":'
  if [[ -n "$focused_subcheck" ]]; then
    json_escape "$focused_subcheck"
  else
    printf 'null'
  fi
}

print_docs_check_subcheck_guidance_json_fields() {
  local slowest_name=""
  local slowest_ms=0
  local slowest_item_count=0
  local slowest_detail=""
  local item subcheck elapsed_ms subcheck_ok item_count subcheck_detail
  for item in "${DOCS_CHECK_SUBCHECK_ITEMS[@]}"; do
    IFS='|' read -r subcheck elapsed_ms subcheck_ok item_count subcheck_detail <<<"$item"
    elapsed_ms="${elapsed_ms:-0}"
    if (( elapsed_ms > slowest_ms )); then
      slowest_ms="$elapsed_ms"
      slowest_name="$subcheck"
      slowest_item_count="${item_count:-0}"
      slowest_detail="$subcheck_detail"
    fi
  done

  printf ',"slowest_subcheck":'
  if [[ -n "$slowest_name" ]]; then
    printf '{"name":'
    json_escape "$slowest_name"
    printf ',"elapsed_ms":%s' "$slowest_ms"
    printf ',"item_count":%s' "$slowest_item_count"
    printf ',"detail":'
    json_escape "$slowest_detail"
    printf '}'
  else
    printf 'null'
  fi

  printf ',"focused_rerun_command":'
  if [[ -n "$slowest_name" ]]; then
    json_escape "scripts/appfw framework docs-check --subcheck ${slowest_name} --json"
  else
    printf 'null'
  fi

  printf ',"parallelization_guidance":{'
  printf '"same_worktree_parallel_safe":false'
  printf ',"reason":'
  json_escape "Docs-check examples share target/appfw logs and retained artifacts; run focused subchecks sequentially in one worktree or in isolated worktrees/report roots."
  printf '}'
}

DOCS_CHECK_SUBCHECK_MATCHED=0
docs_check_agent_governance_alias_matches() {
  local name="$1"
  [[ "$focused_subcheck" == "agent-governance-command-examples" ]] || return 1
  case "$name" in
    agent-governance-core-command-examples|agent-governance-write-command-examples|\
agent-governance-mobile-command-examples|agent-governance-runtime-command-examples|agent-governance-release-command-examples)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

should_run_docs_check_subcheck() {
  local name="$1"
  if [[ -z "$focused_subcheck" || "$focused_subcheck" == "$name" ]] \
    || docs_check_agent_governance_alias_matches "$name"; then
    DOCS_CHECK_SUBCHECK_MATCHED=1
    return 0
  fi
  return 1
}

docs_check_existing_change_impact_base_ref() {
  if git -C "$repo_root" rev-parse --verify "${base_ref}^{commit}" >/dev/null 2>&1; then
    printf '%s\n' "$base_ref"
  else
    printf 'HEAD\n'
  fi
}

if [[ -n "$focused_subcheck" ]] && ! docs_check_subcheck_known "$focused_subcheck"; then
  echo "Unknown docs-check subcheck: $focused_subcheck" >&2
  echo "Known subchecks: shell-syntax lifecycle-evidence design-system security-env-doc-parity maintainability-contract saas-vendor-doc-parity release-lite-guard sra-package-command-examples product-increment-delivery core-bootstrap-command-examples core-generation-command-examples core-generated-drift-command-examples core-explain-profile-command-examples intake-skills-command-examples agent-governance-command-examples agent-governance-core-command-examples agent-governance-write-command-examples agent-governance-mobile-command-examples agent-governance-runtime-command-examples agent-governance-release-command-examples packaging-dependency-command-examples compiled-cli-command-examples" >&2
  exit 2
fi

example_json_item() {
  local first="$1"
  local name="$2"
  local ok="$3"
  local exit_code="$4"
  local log_file="$5"
  local command="$6"
  local elapsed_ms="${7:-0}"

  if [[ "$first" != "1" ]]; then
    printf ','
  fi
  printf '{"name":'
  json_escape "$name"
  printf ',"ok":%s,"exit_code":%s,"log":' "$ok" "$exit_code"
  json_escape "$log_file"
  printf ',"command":'
  json_escape "$command"
  printf ',"elapsed_ms":%s' "$elapsed_ms"
  printf '}'
}

ms_now() {
  python3 - <<'PY'
import time
print(int(time.time() * 1000))
PY
}

progress_event() {
  local phase="$1"
  local name="$2"
  local elapsed_ms="${3:-0}"
  local status="${4:-running}"

  if [[ "$json" == "1" && "$progress" == "1" ]]; then
    printf '{"event":"docs-check-%s","name":' "$phase" >&2
    json_escape "$name" >&2
    printf ',"status":' >&2
    json_escape "$status" >&2
    printf ',"elapsed_ms":%s}\n' "$elapsed_ms" >&2
  fi
}

docs_check_started_ms="$(ms_now)"
docs_check_timing_artifact="$report_dir/docs-check-timing.json"
docs_check_timing_artifact_rel="${docs_check_timing_artifact#$repo_root/}"
docs_check_changed_surface_artifact="$report_dir/docs-check-changed-surface.json"
docs_check_changed_surface_artifact_rel="${docs_check_changed_surface_artifact#$repo_root/}"
docs_check_subchecks_artifact="$report_dir/docs-check-subchecks.json"
docs_check_subchecks_artifact_rel="${docs_check_subchecks_artifact#$repo_root/}"
DOCS_CHECK_TOTAL_WALL_MS=0
DOCS_CHECK_BUDGET_MS=0
DOCS_CHECK_BUDGET_OK=true
DOCS_CHECK_AGGREGATE_EXAMPLE_MS=0
DOCS_CHECK_AGGREGATE_PHASE_MS=0
DOCS_CHECK_UNATTRIBUTED_WALL_MS=0
DOCS_CHECK_EXAMPLE_COUNT=0
DOCS_CHECK_PHASE_COUNT=0
DOCS_CHECK_SLOWEST_EXAMPLE_NAME=""
DOCS_CHECK_SLOWEST_EXAMPLE_MS=0
DOCS_CHECK_PHASE_ITEMS=()
DOCS_CHECK_SUBCHECK_ITEMS=()
DOC_EXAMPLE_FAILED=0
DOC_EXAMPLE_ITEMS=()

docs_check_budget_ms() {
  if [[ -n "$budget_override_ms" ]]; then
    printf '%s' "$budget_override_ms"
    return
  fi
  case "$effective_mode" in
    fast)
      printf '%s' "$fast_budget_ms"
      ;;
    full)
      printf '%s' "$full_budget_ms"
      ;;
    *)
      printf '%s' "$full_budget_ms"
      ;;
  esac
}

write_docs_check_timing_artifact() {
  local ok="$1"
  local completed_ms
  completed_ms="$(ms_now)"
  local total_wall_ms=$(( completed_ms - docs_check_started_ms ))
  DOCS_CHECK_TOTAL_WALL_MS="$total_wall_ms"
  DOCS_CHECK_BUDGET_MS="$(docs_check_budget_ms)"
  DOCS_CHECK_BUDGET_OK=true
  if (( total_wall_ms > DOCS_CHECK_BUDGET_MS )); then
    DOCS_CHECK_BUDGET_OK=false
  fi
  local aggregate_example_ms=0
  local aggregate_phase_ms=0
  local example_count=0
  local phase_count=0
  local max_elapsed_ms=0
  local max_elapsed_name=""
  local item name item_ok exit_code log_file command_text elapsed_ms
  local phase phase_ok phase_detail

  for item in "${DOC_EXAMPLE_ITEMS[@]}"; do
    IFS='|' read -r name item_ok exit_code log_file command_text elapsed_ms <<<"$item"
    elapsed_ms="${elapsed_ms:-0}"
    aggregate_example_ms=$(( aggregate_example_ms + elapsed_ms ))
    example_count=$(( example_count + 1 ))
    if (( elapsed_ms > max_elapsed_ms )); then
      max_elapsed_ms="$elapsed_ms"
      max_elapsed_name="$name"
    fi
  done
  for item in "${DOCS_CHECK_PHASE_ITEMS[@]}"; do
    IFS='|' read -r phase elapsed_ms phase_ok phase_detail <<<"$item"
    elapsed_ms="${elapsed_ms:-0}"
    aggregate_phase_ms=$(( aggregate_phase_ms + elapsed_ms ))
    phase_count=$(( phase_count + 1 ))
  done
  local unattributed_wall_ms=$(( total_wall_ms - aggregate_example_ms - aggregate_phase_ms ))
  if (( unattributed_wall_ms < 0 )); then
    unattributed_wall_ms=0
  fi
  DOCS_CHECK_AGGREGATE_EXAMPLE_MS="$aggregate_example_ms"
  DOCS_CHECK_AGGREGATE_PHASE_MS="$aggregate_phase_ms"
  DOCS_CHECK_UNATTRIBUTED_WALL_MS="$unattributed_wall_ms"
  DOCS_CHECK_EXAMPLE_COUNT="$example_count"
  DOCS_CHECK_PHASE_COUNT="$phase_count"
  DOCS_CHECK_SLOWEST_EXAMPLE_NAME="$max_elapsed_name"
  DOCS_CHECK_SLOWEST_EXAMPLE_MS="$max_elapsed_ms"

  {
    printf '{"command":"docs-check-timing","ok":%s' "$ok"
    printf ',"requested_mode":'
    json_escape "$mode"
    printf ',"selected_mode":'
    json_escape "$effective_mode"
    printf ',"mode_reason":'
    json_escape "$mode_reason"
    printf ',"changed_files_count":%s' "$changed_files_count"
    printf ',"changed_surface_artifact":'
    json_escape "$docs_check_changed_surface_artifact_rel"
    printf ',"subchecks_artifact":'
    json_escape "$docs_check_subchecks_artifact_rel"
    print_docs_check_subcheck_focus_json_fields
    printf ',"changed_surface_requires_full":%s' "$changed_surface_requires_full_result"
    printf ',"changed_surface_trigger_count":%s' "$changed_surface_trigger_count"
    printf ',"total_wall_ms":%s' "$total_wall_ms"
    printf ',"budget_ms":%s' "$DOCS_CHECK_BUDGET_MS"
    printf ',"budget_ok":%s' "$DOCS_CHECK_BUDGET_OK"
    printf ',"budget_enforced":'
    if [[ "$enforce_budget" == "1" ]]; then
      printf 'true'
    else
      printf 'false'
    fi
    printf ',"aggregate_example_ms":%s' "$aggregate_example_ms"
    printf ',"aggregate_phase_ms":%s' "$aggregate_phase_ms"
    printf ',"unattributed_wall_ms":%s' "$unattributed_wall_ms"
    printf ',"example_count":%s' "$example_count"
    printf ',"phase_count":%s' "$phase_count"
    printf ',"subcheck_count":%s' "${#DOCS_CHECK_SUBCHECK_ITEMS[@]}"
    printf ',"slowest_example":{"name":'
    json_escape "$max_elapsed_name"
    printf ',"elapsed_ms":%s}' "$max_elapsed_ms"
    printf ',"phases":['
    first=1
    for item in "${DOCS_CHECK_PHASE_ITEMS[@]}"; do
      IFS='|' read -r phase elapsed_ms phase_ok phase_detail <<<"$item"
      if [[ "$first" != "1" ]]; then
        printf ','
      fi
      printf '{"name":'
      json_escape "$phase"
      printf ',"ok":%s' "$phase_ok"
      printf ',"elapsed_ms":%s' "${elapsed_ms:-0}"
      printf ',"detail":'
      json_escape "$phase_detail"
      printf '}'
      first=0
    done
    printf ']'
    printf ',"subchecks":['
    first=1
    for item in "${DOCS_CHECK_SUBCHECK_ITEMS[@]}"; do
      IFS='|' read -r subcheck elapsed_ms subcheck_ok item_count subcheck_detail <<<"$item"
      if [[ "$first" != "1" ]]; then
        printf ','
      fi
      printf '{"name":'
      json_escape "$subcheck"
      printf ',"ok":%s' "$subcheck_ok"
      printf ',"elapsed_ms":%s' "${elapsed_ms:-0}"
      printf ',"item_count":%s' "${item_count:-0}"
      printf ',"detail":'
      json_escape "$subcheck_detail"
      printf '}'
      first=0
    done
    printf ']'
    print_docs_check_subcheck_guidance_json_fields
    printf ',"examples":['
    first=1
    for item in "${DOC_EXAMPLE_ITEMS[@]}"; do
      IFS='|' read -r name item_ok exit_code log_file command_text elapsed_ms <<<"$item"
      example_json_item "$first" "$name" "$item_ok" "$exit_code" "$log_file" "$command_text" "${elapsed_ms:-0}"
      first=0
    done
    printf ']}\n'
  } >"$docs_check_timing_artifact"
}

record_docs_check_phase() {
  local name="$1"
  local elapsed_ms="$2"
  local ok="${3:-true}"
  local detail="${4:-}"
  DOCS_CHECK_PHASE_ITEMS+=("$name|$elapsed_ms|$ok|$detail")
}

doc_example_item_count() {
  set +u
  local count="${#DOC_EXAMPLE_ITEMS[@]}"
  set -u
  printf '%s' "$count"
}

doc_example_failure_count() {
  local count=0
  local item name item_ok exit_code log_file command_text elapsed_ms
  set +u
  for item in "${DOC_EXAMPLE_ITEMS[@]}"; do
    IFS='|' read -r name item_ok exit_code log_file command_text elapsed_ms <<<"$item"
    if [[ "$item_ok" != "true" ]]; then
      count=$(( count + 1 ))
    fi
  done
  set -u
  printf '%s' "$count"
}

record_docs_check_subcheck_span() {
  local name="$1"
  local start_ms="$2"
  local before_item_count="$3"
  local before_failure_count="$4"
  local detail="$5"
  local elapsed_ms=$(( $(ms_now) - start_ms ))
  local after_item_count
  after_item_count="$(doc_example_item_count)"
  local after_failure_count
  after_failure_count="$(doc_example_failure_count)"
  local item_count=$(( after_item_count - before_item_count ))
  local ok=true
  if (( after_failure_count > before_failure_count )); then
    ok=false
  fi
  DOCS_CHECK_SUBCHECK_ITEMS+=("$name|$elapsed_ms|$ok|$item_count|$detail")
}

run_docs_check_subcheck() {
  local name="$1"
  local detail="$2"
  shift 2
  if ! should_run_docs_check_subcheck "$name"; then
    return 0
  fi
  local start_ms before_item_count before_failure_count
  start_ms="$(ms_now)"
  before_item_count="$(doc_example_item_count)"
  before_failure_count="$(doc_example_failure_count)"
  "$@"
  record_docs_check_subcheck_span "$name" "$start_ms" "$before_item_count" "$before_failure_count" "$detail"
}

write_docs_check_subchecks_artifact() {
  local ok="$1"
  {
    printf '{"command":"docs-check-subchecks","ok":%s' "$ok"
    printf ',"requested_mode":'
    json_escape "$mode"
    printf ',"selected_mode":'
    json_escape "$effective_mode"
    print_docs_check_subcheck_focus_json_fields
    printf ',"subcheck_count":%s' "${#DOCS_CHECK_SUBCHECK_ITEMS[@]}"
    printf ',"subchecks":['
    first=1
    local item subcheck elapsed_ms subcheck_ok item_count subcheck_detail
    for item in "${DOCS_CHECK_SUBCHECK_ITEMS[@]}"; do
      IFS='|' read -r subcheck elapsed_ms subcheck_ok item_count subcheck_detail <<<"$item"
      if [[ "$first" != "1" ]]; then
        printf ','
      fi
      printf '{"name":'
      json_escape "$subcheck"
      printf ',"ok":%s' "$subcheck_ok"
      printf ',"elapsed_ms":%s' "${elapsed_ms:-0}"
      printf ',"item_count":%s' "${item_count:-0}"
      printf ',"detail":'
      json_escape "$subcheck_detail"
      printf '}'
      first=0
    done
    printf ']'
    print_docs_check_subcheck_guidance_json_fields
    printf '}\n'
  } >"$docs_check_subchecks_artifact"
}

run_example_with_allowed_statuses() {
  local allowed_statuses="$1"
  shift
  local name="$1"
  shift
  local log_file="$report_dir/docs-example-${name}.log"
  local command_text="$*"
  local start_ms
  start_ms="$(ms_now)"

  if [[ "$json" != "1" ]]; then
    printf 'Checking docs example: %s\n' "$command_text"
  fi
  progress_event "start" "$name"

  set +e
  "$@" >"$log_file" 2>&1
  local status=$?
  set -e
  local elapsed_ms
  elapsed_ms=$(( $(ms_now) - start_ms ))

  local ok=false
  local allowed_status
  for allowed_status in $allowed_statuses; do
    if [[ $status -eq "$allowed_status" ]]; then
      ok=true
      break
    fi
  done

  DOC_EXAMPLE_ITEMS+=("$name|$ok|$status|$log_file|$command_text|$elapsed_ms")
  progress_event "finish" "$name" "$elapsed_ms" "$ok"
  if [[ "$json" != "1" ]]; then
    printf 'Finished docs example: %s (%sms)\n' "$name" "$elapsed_ms"
  fi
  if [[ "$ok" != "true" ]]; then
    DOC_EXAMPLE_FAILED=1
    if [[ "$json" != "1" || "$progress" == "1" ]]; then
      printf 'Docs example failed: %s\n' "$name" >&2
      printf 'Expected exit status in [%s], actual %s\n' "$allowed_statuses" "$status" >&2
      printf 'Log: %s\n' "$log_file" >&2
      cat "$log_file" >&2 || true
    fi
  fi
}

run_example() {
  run_example_with_allowed_statuses "0" "$@"
}

run_log_assertion() {
  local name="$1"
  local log_file="$2"
  local pattern="$3"
  local detail="$4"
  local command_text="assert ${detail} in ${log_file}"
  local start_ms
  start_ms="$(ms_now)"

  if [[ "$json" != "1" ]]; then
    printf 'Checking docs example assertion: %s\n' "$detail"
  fi
  progress_event "start" "$name"

  local status=0
  if ! grep -Fq "$pattern" "$log_file"; then
    status=1
    DOC_EXAMPLE_FAILED=1
    if [[ "$json" != "1" ]]; then
      printf 'Missing expected docs example output: %s\n' "$detail" >&2
    fi
  fi

  local ok=false
  if [[ $status -eq 0 ]]; then
    ok=true
  fi
  local elapsed_ms
  elapsed_ms=$(( $(ms_now) - start_ms ))
  DOC_EXAMPLE_ITEMS+=("$name|$ok|$status|$log_file|$command_text|$elapsed_ms")
  progress_event "finish" "$name" "$elapsed_ms" "$ok"
}

run_json_field_assertion() {
  local name="$1"
  local log_file="$2"
  local field_path="$3"
  local expected_json="$4"
  local detail="$5"
  local command_text="assert JSON ${field_path} == ${expected_json} in ${log_file}"
  local start_ms
  start_ms="$(ms_now)"

  if [[ "$json" != "1" ]]; then
    printf 'Checking docs JSON semantic assertion: %s\n' "$detail"
  fi
  progress_event "start" "$name"

  local status=0
  set +e
  python3 - "$log_file" "$field_path" "$expected_json" <<'PY'
import json
import sys

path, field_path, expected_json = sys.argv[1:4]
with open(path, "r", encoding="utf-8") as handle:
    value = json.load(handle)
for field in field_path.split("."):
    if not isinstance(value, dict) or field not in value:
        raise SystemExit(f"missing JSON field: {field_path}")
    value = value[field]
expected = json.loads(expected_json)
if value != expected:
    raise SystemExit(f"JSON field {field_path} was {value!r}, expected {expected!r}")
PY
  status=$?
  set -e

  local ok=false
  if [[ $status -eq 0 ]]; then
    ok=true
  else
    DOC_EXAMPLE_FAILED=1
    if [[ "$json" != "1" ]]; then
      printf 'Docs JSON semantic assertion failed: %s\n' "$detail" >&2
    fi
  fi
  local elapsed_ms
  elapsed_ms=$(( $(ms_now) - start_ms ))
  DOC_EXAMPLE_ITEMS+=("$name|$ok|$status|$log_file|$command_text|$elapsed_ms")
  progress_event "finish" "$name" "$elapsed_ms" "$ok"
}

run_change_impact_restored_assertion() {
  local name="$1"
  local log_file="$2"
  local detail="$3"
  local command_text="assert restored change-impact contract in ${log_file}"
  local start_ms
  start_ms="$(ms_now)"

  if [[ "$json" != "1" ]]; then
    printf 'Checking docs change-impact assertion: %s\n' "$detail"
  fi
  progress_event "start" "$name"

  local status=0
  set +e
  python3 - "$log_file" <<'PY'
import json
import sys

with open(sys.argv[1], "r", encoding="utf-8") as handle:
    payload = json.load(handle)

for field, expected in {
    "command": "change-impact",
    "scope": "framework",
    "mode": "report-only",
    "artifact": "target/appfw/change-impact.json",
}.items():
    if payload.get(field) != expected:
        raise SystemExit(f"restored change-impact {field} mismatch")
for forbidden in ("error", "git_errors", "git_error_count"):
    if forbidden in payload:
        raise SystemExit(f"restored change-impact retained {forbidden}")
if not payload.get("diff_base_ref") or not payload.get("head_ref") or not payload.get("sha"):
    raise SystemExit("restored change-impact omitted comparison identity")

operational_ok = payload.get("ok")
if operational_ok is True:
    raise SystemExit(0)
if operational_ok is not False:
    raise SystemExit("restored change-impact ok is not boolean")
profile = payload.get("delivery_profile")
projection = profile.get("gate_execution_projection") if isinstance(profile, dict) else None
if not isinstance(projection, dict) or projection.get("ok") is not False:
    raise SystemExit(
        "restored change-impact is false for a reason other than candidate gate projection"
    )
if profile.get("enforce_candidate") is not True:
    raise SystemExit("false gate projection is not bound to candidate enforcement")
PY
  status=$?
  set -e

  local ok=false
  if [[ $status -eq 0 ]]; then
    ok=true
  else
    DOC_EXAMPLE_FAILED=1
    if [[ "$json" != "1" ]]; then
      printf 'Docs change-impact assertion failed: %s\n' "$detail" >&2
    fi
  fi
  local elapsed_ms
  elapsed_ms=$(( $(ms_now) - start_ms ))
  DOC_EXAMPLE_ITEMS+=("$name|$ok|$status|$log_file|$command_text|$elapsed_ms")
  progress_event "finish" "$name" "$elapsed_ms" "$ok"
}

run_log_absence_assertion() {
  local name="$1"
  local log_file="$2"
  local pattern="$3"
  local detail="$4"
  local command_text="assert ${detail} is absent from ${log_file}"
  local start_ms
  start_ms="$(ms_now)"

  if [[ "$json" != "1" ]]; then
    printf 'Checking docs example absence assertion: %s\n' "$detail"
  fi
  progress_event "start" "$name"

  local status=0
  if grep -Fq "$pattern" "$log_file"; then
    status=1
    DOC_EXAMPLE_FAILED=1
    if [[ "$json" != "1" ]]; then
      printf 'Unexpected docs example output: %s\n' "$detail" >&2
    fi
  fi

  local ok=false
  if [[ $status -eq 0 ]]; then
    ok=true
  fi
  local elapsed_ms
  elapsed_ms=$(( $(ms_now) - start_ms ))
  DOC_EXAMPLE_ITEMS+=("$name|$ok|$status|$log_file|$command_text|$elapsed_ms")
  progress_event "finish" "$name" "$elapsed_ms" "$ok"
}

run_wave2_external_plan_artifact_assertion() {
  local name="$1"
  local log_file="$2"
  local detail="$3"
  local command_text="assert ${detail} in ${log_file}"
  local start_ms
  start_ms="$(ms_now)"

  if [[ "$json" != "1" ]]; then
    printf 'Checking docs example assertion: %s\n' "$detail"
  fi
  progress_event "start" "$name"

  set +e
  python3 - "$log_file" <<'PY'
import json
import sys

path = sys.argv[1]
with open(path, "r", encoding="utf-8") as handle:
    data = json.load(handle)

bad = []
bad_commands = []
product_local_tokens = (
    "examples/products/",
    ".appfw/target/appfw",
    ".appfw-mobile",
)
placeholder_tokens = (
    "<real-",
    "PLACEHOLDER",
    "TODO",
)
for item in data.get("external_evidence_plan", []):
    lane = item.get("lane", "<unknown>")
    for artifact in item.get("required_artifacts", []) or []:
        if not isinstance(artifact, str) or not artifact.startswith("target/appfw/"):
            bad.append(f"{lane}: {artifact!r}")
            continue
        if any(token in artifact for token in product_local_tokens):
            bad.append(f"{lane}: product-local artifact {artifact!r}")
    for command in item.get("commands", []) or []:
        if not isinstance(command, str):
            bad_commands.append(f"{lane}: non-string command {command!r}")
            continue
        if any(token in command for token in placeholder_tokens):
            bad_commands.append(f"{lane}: placeholder command {command!r}")

if bad:
    print("External evidence plan contains non-release-staged artifacts:")
    for entry in bad:
        print(f"- {entry}")
if bad_commands:
    print("External evidence plan contains placeholder commands:")
    for entry in bad_commands:
        print(f"- {entry}")
if bad or bad_commands:
    sys.exit(1)
PY
  local status=$?
  set -e
  if [[ $status -ne 0 ]]; then
    DOC_EXAMPLE_FAILED=1
    if [[ "$json" != "1" ]]; then
      printf 'Invalid docs example output: %s\n' "$detail" >&2
    fi
  fi

  local ok=false
  if [[ $status -eq 0 ]]; then
    ok=true
  fi
  local elapsed_ms
  elapsed_ms=$(( $(ms_now) - start_ms ))
  DOC_EXAMPLE_ITEMS+=("$name|$ok|$status|$log_file|$command_text|$elapsed_ms")
  progress_event "finish" "$name" "$elapsed_ms" "$ok"
}

run_security_env_doc_parity() {
  local name="security-env-doc-parity"
  local log_file="$report_dir/docs-example-${name}.log"
  local command_text="assert SecurityConfig env vars are documented in docs/release/deployment-reference.md"
  local start_ms
  start_ms="$(ms_now)"

  if [[ "$json" != "1" ]]; then
    printf 'Checking docs example assertion: %s\n' "$command_text"
  fi
  progress_event "start" "$name"

  set +e
  python3 - "$repo_root" >"$log_file" 2>&1 <<'PY'
import json
import re
import sys
from pathlib import Path

repo_root = Path(sys.argv[1])
security_rs = repo_root / "appfw_runtime/src/security.rs"
deployment_doc = repo_root / "docs/release/deployment-reference.md"

source = security_rs.read_text(encoding="utf-8")
match = re.search(
    r"pub fn from_env\(\) -> Self \{(?P<body>.*?)\n    \}\n\n    pub fn validate_runtime_safety",
    source,
    re.S,
)
if not match:
    raise SystemExit("could not locate SecurityConfig::from_env in appfw_runtime/src/security.rs")

env_vars = sorted(set(re.findall(r'"(APP_[A-Z0-9_]+)"', match.group("body"))))
env_vars.extend(
    name
    for name in [
        "APP_ENABLE_LOCAL_TEST_AUTH",
        "APP_ALLOW_MISSING_POLICIES_IN_LOCAL",
        "APP_BYPASS_POLICIES_IN_LOCAL",
    ]
    if name not in env_vars
)
env_vars = sorted(env_vars)

doc_text = deployment_doc.read_text(encoding="utf-8")
missing = [name for name in env_vars if name not in doc_text]
report = {
    "command": "security-env-doc-parity",
    "ok": not missing,
    "source": security_rs.relative_to(repo_root).as_posix(),
    "document": deployment_doc.relative_to(repo_root).as_posix(),
    "env_vars": env_vars,
    "missing": missing,
}
print(json.dumps(report, indent=2, sort_keys=True))
if missing:
    raise SystemExit(1)
PY
  local status=$?
  set -e

  local ok=false
  if [[ $status -eq 0 ]]; then
    ok=true
  else
    DOC_EXAMPLE_FAILED=1
    if [[ "$json" != "1" ]]; then
      cat "$log_file" >&2 || true
    fi
  fi
  local elapsed_ms
  elapsed_ms=$(( $(ms_now) - start_ms ))
  DOC_EXAMPLE_ITEMS+=("$name|$ok|$status|$log_file|$command_text|$elapsed_ms")
  progress_event "finish" "$name" "$elapsed_ms" "$ok"
}

run_maintainability_contract_check() {
  local name="maintainability-contract"
  local log_file="$report_dir/${name}.json"
  local command_text="assert docs IA, roadmap, deployment, ingress, frontend design system, and golden path contracts are discoverable"
  local start_ms
  start_ms="$(ms_now)"

  if [[ "$json" != "1" ]]; then
    printf 'Checking docs example assertion: %s\n' "$command_text"
  fi
  progress_event "start" "$name"

  set +e
  python3 - "$repo_root" >"$log_file" 2>&1 <<'PY'
import json
import sys
from pathlib import Path

repo_root = Path(sys.argv[1])

required_docs = [
    "README.md",
    "AGENTS.md",
    "agent_skills/README.md",
    "agent_skills/framework-pr-review/SKILL.md",
    "agent_skills/framework-structure-steward/SKILL.md",
    "agent_skills/framework-research-steward/SKILL.md",
    "agent_skills/product-pr-review/SKILL.md",
    "docs/README.md",
    "docs/product/README.md",
    "docs/framework/README.md",
    "docs/start/README.md",
    "docs/start/agent-task-map.md",
    "docs/start/agent-role-cards.md",
    "docs/start/agentic-human-operating-model.md",
    "docs/start/branch-integration-model.md",
    "docs/start/remote-promotion-state-synchronization.md",
    "docs/start/cli-quickstart.md",
    "docs/start/delivery-profiles.md",
    "docs/start/team-harness-replication.md",
    "docs/start/framework-research-steward-harness.md",
    "docs/reference/cli.md",
    "docs/specs/README.md",
    "docs/specs/app-framework-delivery-profiles.md",
    "docs/specs/afs-008-current-task-instruction-routing.md",
    "docs/lifecycle/README.md",
    "docs/architecture/overview.md",
    "docs/architecture/README.md",
    "docs/architecture/adr/0013-docs-information-architecture.md",
    "docs/architecture/concerns/packaging.md",
    "docs/architecture/concerns/runtime-modularity.md",
    "docs/architecture/concerns/north-star-wave-0.md",
    "docs/runtime/saas-connectors.md",
    "docs/architecture/concerns/agentic-development-control-system.md",
    "docs/specs/remote-promotion-state-synchronization.md",
    "docs/strategy/product-development-north-star.md",
    "docs/release/roadmap.md",
    "docs/release/evidence-matrix.md",
    "docs/release/deployment-reference.md",
    "docs/release/live-environment-work-items.md",
    "docs/release/pds-security-baseline-traceability.md",
    "docs/release/versioning-and-compatibility.md",
    "docs/lifecycle/product-golden-path.md",
    "docs/lifecycle/intake-and-discovery.md",
    "docs/lifecycle/legacy-modernization.md",
    "docs/model/README.md",
    "docs/model/model-proposal-to-config.md",
    "docs/frontend/product-frontend.md",
    "docs/frontend/mobile-react-native.md",
    "docs/frontend/pds-health-design-system.md",
    "docs/start/pr-review-agent-harness.md",
    "docs/start/product-pr-review-agent-harness.md",
    "docs/start/pds-sra-package-harness.md",
    "docs/start/agent-handoff-backlog.md",
    "docs/architecture/adr/0008-frontend-design-system.md",
    "docs/architecture/concerns/maintainability.md",
    "docs/start/cli.md",
    "scripts/check-pds-tokens.mjs",
    "scripts/check-pds-components.mjs",
    "appfw_ui/pds_health/README.md",
    "appfw_ui/pds_health/components/README.md",
    "appfw_ui/pds_health/components/package.json",
    "appfw_ui/pds_health/components/src/index.ts",
    "appfw_ui/pds_health/components/src/styles.css",
    "appfw_ui/pds_health/tokens/pdsTokens.css",
    "appfw_ui/pds_health/tokens/pdsTokens.ts",
]

required_tokens = {
    "README.md": [
        "examples/products/crm/frontend",
        "docs/architecture/concerns/maintainability.md",
        "agent_skills/README.md",
        "agentic-first product",
    ],
    "AGENTS.md": [
        "agent_skills/README.md",
        "docs/start/cli-quickstart.md",
        "docs/reference/cli.md",
        "docs/architecture/concerns/maintainability.md",
        "scripts/appfw framework docs-check --json",
        "scripts/appfw product handoff --json",
        "scripts/appfw framework handoff --json",
        "Pre-Push Review Gate",
        "Do not ask the",
        "human for case-by-case permission",
        "framework-pr-review",
        "/framework-pr-review --comprehensive",
        "/product-pr-review --comprehensive",
        "review-brief --auto-depth --json",
        "docs/start/framework-research-steward-harness.md",
        "agent_skills/framework-research-steward/SKILL.md",
        "/framework-research-refresh",
        "/check-pr-review-performance",
        "PR Review Performance Oversight",
        "Recommendation Summary",
        "GO WITH CONDITIONS",
        "severity counts",
        "Conditions captured",
        "Role Adherence Assessment",
        "Alignment Drift Assessment",
        "docs/start/agent-role-cards.md",
        "docs/start/remote-promotion-state-synchronization.md",
        "standing approval authorizes pushing assigned",
        "Branch Naming",
        "Branch prefixes describe the work stream",
        "Do not use assistant/tool prefixes",
    ],
    "agent_skills/README.md": [
        "progressive disclosure",
        "Product Skills",
        "Framework Steward Skills",
        "scripts/appfw product",
        "scripts/appfw framework",
        "scripts/appfw framework instructions",
        "product-bootstrap",
        "product-poc-intake",
        "product-legacy-modernization",
        "target/appfw/product-analysis.json",
        "target/appfw/model-status.json",
        "review_decisions",
        "product-schema-modeling",
        "product-generate-verify",
        "product-local-run-test",
        "product-frontend",
        "product-mobile-react-native",
        "product-agent-harness",
        "product-release-evidence",
        "framework-docs-ia",
        "framework-structure-steward",
        "framework-research-steward",
        "framework-frontend-design-system",
        "framework-provider-certification",
        "canonical docs",
        "framework-pr-review",
        "product-pr-review",
        "pds-sra-package",
        "/framework-pr-review --comprehensive",
        "/framework-research-refresh",
        "/product-pr-review --comprehensive",
        "/pds-sra-package --framework",
        "docs/start/pds-sra-package-harness.md",
        "/check-pr-review-performance",
        "docs/start/pr-review-agent-harness.md",
        "docs/start/product-pr-review-agent-harness.md",
        "docs/start/framework-research-steward-harness.md",
        "docs/start/agent-role-cards.md",
    ],
    "agent_skills/framework-structure-steward/SKILL.md": [
        "Framework Structure Steward",
        "docs IA, skills, CLI contracts",
        "maintainability drift",
        "Product Owner/Strategist",
        "Architect evaluates feasibility and sequencing",
        "Verdict",
        "healthy",
        "needs-maintenance",
        "scripts/appfw framework change-impact --json",
        "scripts/appfw framework docs-check --changed-only --json",
        "should not sneak broad refactors",
    ],
    "agent_skills/framework-research-steward/SKILL.md": [
        "App Framework Research Steward",
        "/framework-research-refresh",
        "Research Verdict",
        "Business Value Implications",
        "Challenge To Current Guidance",
        "Recommended Product Actions",
        "Evidence Quality",
        "Anti-Fad Filter",
        "Strategist/Product Manager",
        "Do not replace the Strategist/Product Manager function or Product Owner",
    ],
    "agent_skills/framework-pr-review/SKILL.md": [
        "Framework PR Review Agent",
        "/framework-pr-review --comprehensive",
        "agent-authored",
        "scripts/appfw framework review-brief --auto-depth --json",
        "human reviewer's first-pass responsibility",
        "Recommendation Summary",
        "Attention Items when final status is not GO",
        "GO WITH CONDITIONS",
        "severity counts",
        "Conditions captured",
        "Role Adherence Assessment",
        "Alignment Drift Assessment",
        "code, skills, CLI, docs",
        "informationally and philosophically aligned",
    ],
    "agent_skills/product-pr-review/SKILL.md": [
        "Product PR Review Agent",
        "/product-pr-review --comprehensive",
        "agent-authored",
        "scripts/appfw product review-brief --auto-depth --json",
        "Recommendation Summary",
        "Attention Items when final status is not GO",
        "GO WITH CONDITIONS",
        "severity counts",
        "Conditions captured",
        "Role Adherence Assessment",
        "Alignment Drift Assessment",
        "product/framework alignment",
        "informationally and philosophically aligned",
    ],
    "agent_skills/pds-sra-package/SKILL.md": [
        "PDS SRA Package",
        "/pds-sra-package --framework",
        "/pds-sra-package --product",
        "target/appfw/sra-package.md",
        "scripts/appfw framework sra-package --all-products --json",
        "scripts/appfw product sra-package --json",
        "Data Flow Diagram",
        "Infrastructure / Logical Architecture Diagram",
        "LogicGate",
        "The SRA package is preparation evidence, not SRA approval",
    ],
    "docs/start/pr-review-agent-harness.md": [
        "Framework PR Review Agent",
        "PR Review Agent Harness",
        "framework-pr-review",
        "/framework-pr-review --comprehensive",
        "/check-pr-review-performance",
        "automatic at the push boundary",
        "scripts/ci/pre-push-review-guard.sh",
        "target/appfw/framework-pr-review.md",
        "newer than the current handoff artifact",
        "auto-depth selection",
        "generic `/Code Review` command is not automatically this harness",
        "Coding Agent Pre-Push Rule",
        "Review Depth Flags",
        "Focused mode is the default",
        "scripts/appfw framework review-brief --auto-depth --json",
        "agent-authored",
        "PR Review Performance Oversight",
        "Performance Verdict",
        "System Tuning Actions",
        "standing push approval",
        "Agent Role Cards",
        "Shared Reviewer Mandate",
        "Standing Authorization To Invoke Review",
        "must not pause to ask whether private repository code may be sent",
        "The Framework PR Review Agent and the human reviewer should be of one mind",
        "human reviewer's first-pass responsibility",
        "Recommendation Summary",
        "GO WITH CONDITIONS",
        "Severity counts",
        "Strategic Significance",
        "Role Adherence Assessment",
        "Alignment Drift Assessment",
        "Independent Code Quality And Architecture Assessment",
        "Shared Reviewer Judgment",
        "Engineering And Architecture Review Lens",
        "Cross-Surface Alignment Drift Review",
        "Cross-surface alignment",
        "code, skills, CLI, docs",
        "counterpart surfaces",
        "informationally and philosophically aligned",
        "Correctness and contract fidelity",
        "Security by design",
        "Business value alignment",
        "Human Approval Brief",
        "docs/architecture/concerns/agentic-development-control-system.md",
        "scripts/appfw framework review-brief --json",
        "target/appfw/change-impact.json",
        "target/appfw/review-brief.json",
    ],
    "docs/start/product-pr-review-agent-harness.md": [
        "Product PR Review Agent Harness",
        "standing-authorized",
        "Do not ask for per-review human permission",
        "/product-pr-review --comprehensive",
        "Focused mode is the default",
        "scripts/appfw product review-brief --auto-depth --json",
        "scripts/appfw product review-brief --comprehensive --json",
        "whole product branch",
        "product/framework boundary drift",
        "Recommendation Summary",
        "GO WITH CONDITIONS",
        "severity counts",
        "Role Adherence Assessment",
        "Alignment Drift Assessment",
        "informationally and philosophically aligned",
    ],
    "docs/start/pds-sra-package-harness.md": [
        "PDS SRA Package Harness",
        "/pds-sra-package --all-products",
        "target/appfw/sra-package.md",
        ".appfw/target/appfw/sra-package.md",
        "scripts/appfw framework sra-package --all-products --json",
        "scripts/appfw product sra-package --json",
        "Framework Perspective",
        "Product App Perspective",
        "Data Flow Diagram",
        "Infrastructure / Logical Architecture Diagram",
        "missing_human_inputs",
        "Do not treat `sra-package.json` or `sra-package.md` as approval",
    ],
    "docs/start/branch-integration-model.md": [
        "Branch Integration Model",
        "Branch Names",
        "Branch prefixes describe the work",
        "feature/<topic>",
        "integrate/<wave-or-family>",
        "Do not use author/tool prefixes",
        "codex/",
        "claude/",
        "agent/",
        "timeout is an escalation/transfer trigger",
        "failure alone",
        "monitoring responsibility disappear",
    ],
    "docs/start/remote-promotion-state-synchronization.md": [
        "Remote Promotion State Synchronization",
        "appfw-promotion-assignment:v1:<train_id>",
        "appfw-promotion-ack:v1:<ack_id>",
        "assignment_status",
        "lease_generation",
        "Bitbucket server time",
        "cannot exceed the configured",
        "configured stale threshold",
        "event_sequence",
        "XO-owned append-only acknowledgement",
        "preserves every valid unacknowledged event",
        "timeout emits a stale/escalation event",
        "pipeline-reported commit SHA",
        "exact merge SHA",
        "Failed, stopped, or expired pipeline runs remain `ACTIVE`",
        "DESTINATION_VERIFIED",
        "STALE",
        "UNKNOWN",
        "preserves dependency and shared-seam holds",
        "Product Owner owns product acceptance criteria",
        "Automation must not",
        "task replacement or machine move",
    ],
    "docs/specs/remote-promotion-state-synchronization.md": [
        "Remote Promotion State Synchronization Spec",
        "Spec depth: full",
        "Bitbucket PR is the default durable carrier",
        "appfw.remote-promotion-assignment.v1",
        "appfw.remote-promotion-event.v1",
        "appfw.remote-promotion-ack.v1",
        "assignment_status",
        "lease_generation",
        "lease_issued_at",
        "locally invented lease is invalid",
        "unacknowledged global sequence regardless of the producing lease generation",
        "pipeline_commit_sha",
        "Pipeline-only trains are not",
        "supported by v1",
        "failed, stopped, or expired pipeline run is run-terminal but not",
        "XO may write acknowledgement markers only",
        "competing lease claims",
        "Effective freshness is independent of cached freshness",
        "DESTINATION_VERIFIED",
        "Product `Accepted`",
        "Role Card Check",
    ],
    "docs/start/README.md": [
        "Agent Task Map",
        "Agent Role Cards",
        "Branch Integration Model",
        "Remote Promotion State Synchronization",
        "Framework Research Steward Harness",
        "Product PR Review Agent Harness",
        "PDS SRA Package Harness",
        "branch naming",
    ],
    "docs/start/agent-role-cards.md": [
        "Agent Role Cards",
        "default coordination topology",
        "Architect-Structure-Product Triage",
        "Structure Steward detects and recommends",
        "Architect evaluates feasibility and sequencing",
        "Product Owner prioritizes",
        "durable adjacent thread",
        "bounded subagent",
        "Remote Push Standing Approval",
        "Independent Review Invocation Standing Authorization",
        "must not ask",
        "case-by-case human permission",
        "GO WITH CONDITIONS",
        "blockers",
        "critical",
        "Standing approval does",
        "Strategist / Product Manager Agent",
        "Product Owner Agent",
        "Workstream Analyst",
        "Architect Agent",
        "Coding Agent",
        "Integration Branch Manager Agent",
        "Framework PR Review Agent",
        "Product PR Review Agent",
        "Framework Structure Steward",
        "App Framework Research Steward",
        "Tech Debt Steward",
        "SRA Package Agent",
        "CAB Package Agent",
        "Automatic Pre-Push Guard",
        "scripts/ci/install-local-git-hooks.sh",
        "pre-push-review-guard.sh",
    ],
    "docs/start/agentic-human-operating-model.md": [
        "Agentic And Human Operating Model",
        "Default Coordination Topology",
        "Structure Steward Triage Flow",
        "Structure Steward detects and recommends",
        "Architect evaluates feasibility and sequencing",
        "Product Owner prioritizes",
        "Do not spawn every role up front",
        "Durable product-owner thread",
        "Durable operating coordinator",
        "adjacent thread",
        "subagent",
        "one owner per branch",
        "Strategist / Product Manager function",
        "Product Owner",
        "Workstream Analyst",
        "Architect Agent",
        "Integration Branch Manager",
        "Framework PR Review Agent",
        "GO WITH CONDITIONS",
        "standing approval",
        "App Framework Research Steward",
        "Tech Debt Steward",
        "Agent Role Cards",
    ],
    "docs/start/team-harness-replication.md": [
        "Team Harness Replication",
        "fresh checkout",
        "Agent Role Cards",
        "default coordination topology",
        "Integration Branch Manager",
        "bounded analysis",
        "framework-research-steward",
        "Slash Command Portability",
        "Review gates",
        "What Must Stay Local",
        "push-capable agents",
        "ProGet",
        "Continuous Replication Checks",
        "appfw_instruction_route@1",
        "scripts/appfw framework instructions",
    ],
    "docs/frontend/pds-health-design-system.md": [
        "PDS Health Enterprise Design System",
        "appfw_ui/pds_health",
        "CRM-neutral",
        "scripts/check-pds-tokens.mjs --json",
        "scripts/check-pds-components.mjs --json",
        "scripts/appfw framework golden-downstream --json",
        "scripts/appfw product frontend-test --json",
    ],
    "docs/frontend/product-frontend.md": [
        "PDS Health Enterprise Design System",
        "appfw_ui/pds_health/tokens/pdsTokens.css",
        "appfw_ui/pds_health/components",
        "mobile-react-native.md",
        "scripts/appfw product handoff --json",
        "CRM frontend as a reference implementation",
    ],
    "docs/frontend/mobile-react-native.md": [
        "React Native Mobile App Contract",
        "React Native + Expo",
        "scripts/appfw product mobile-plan --ui-artifact",
        "product-mobile-react-native",
        "mobile/.appfw-mobile/ownership.json",
        "generate --target mobile-rn",
        "mobile-test --json",
        "mobile-test --run-local --json",
        "store-track-evidence.json",
    ],
    "docs/strategy/product-development-north-star.md": [
        "React Native + Expo is the mobile direction.",
        "PWA remains a",
        "docs/frontend/mobile-react-native.md",
        "scripts/appfw product mobile-plan --ui-artifact",
    ],
    "appfw_ui/pds_health/README.md": [
        "PDS Health Design System",
        "Framework",
        "@appfw/pds-health/tokens",
        "@appfw/pds-health-components",
    ],
    "appfw_ui/pds_health/components/README.md": [
        "PDS Health Component System",
        "@appfw/pds-health-components",
        "scripts/check-pds-components.mjs --json",
        "DataGridShell",
    ],
    "docs/README.md": [
        "Choose Your Path",
        "Product Developer",
        "Framework Steward",
        "scripts/appfw product <command>",
        "scripts/appfw framework <command>",
        "Agent Skills Pack",
        "CLI Quickstart",
        "CLI Reference",
        "Product Delivery Lifecycle",
        "Information Architecture",
        "Canonical Contract Docs",
        "0013 Docs Information Architecture",
        "Archive Rule",
    ],
    "docs/architecture/adr/0013-docs-information-architecture.md": [
        "audience-aware, lifecycle-first",
        "product developer path",
        "framework steward path",
        "Skills are procedural overlays",
        "must declare their audience",
        "do not create root-level redirect stubs",
    ],
    "docs/product/README.md": [
        "scripts/appfw product <command>",
        "Product Lifecycle",
        "product-poc-intake",
        "product-legacy-modernization",
        "product-schema-modeling",
        "product-release-evidence",
        "scripts/appfw product analyze --summary --json",
        "scripts/appfw product propose-model --summary --json",
        "scripts/appfw product model-status --json",
        "scripts/appfw product scaffold-model --dry-run --json",
        ".appfw/model-proposal.yaml",
        "target/appfw/model-proposal.json",
        "target/appfw/model-status.json",
        "target/appfw/model-scaffold.json",
        "review_decisions",
        "Model Proposal To Config",
        "product-handoff",
    ],
    "docs/lifecycle/intake-and-discovery.md": [
        "PoC",
        ".appfw/poc-intake.yaml",
        ".appfw/poc-analysis.yaml",
        ".appfw/model-proposal.yaml",
        "target/appfw/product-analysis.json",
        "target/appfw/model-proposal.json",
        "scripts/appfw product analyze --summary --json",
        "scripts/appfw product propose-model --summary --json",
        "scripts/appfw product model-status --json",
        "scripts/appfw product scaffold-model --dry-run --json",
        "review_decisions",
        "model_clues",
        "Model Proposal To Config",
    ],
    "docs/model/README.md": [
        "Model And Contract Design",
        "Model Proposal To Config",
        "Schema Design",
        "Custom Methods And Routines",
    ],
    "docs/model/model-proposal-to-config.md": [
        ".appfw/model-proposal.yaml",
        "target/appfw/model-proposal.json",
        "target/appfw/model-status.json",
        "target/appfw/model-scaffold.json",
        "review_decisions",
        ".appfw/model",
        "NavToOne",
        "NavToMany",
        "custom method",
        "scripts/appfw product validate --json",
        "scripts/appfw product generate --check --json",
        "Workbook-Backed PoC Example",
    ],
    "docs/framework/README.md": [
        "scripts/appfw framework <command>",
        "Framework Stewardship Lifecycle",
        "framework-docs-ia",
        "scripts/appfw framework intake-proof --json",
        "framework-provider-certification",
    ],
    "docs/start/agent-task-map.md": [
        "scripts/appfw context --json",
        "scripts/appfw framework instructions",
        "appfw_instruction_route@1",
        "scripts/appfw product validate --json",
        "scripts/appfw product new --list-profiles --json",
        "--source-kind legacy",
        "scripts/appfw product analyze --summary --json",
        "scripts/appfw product propose-model --summary --json",
        "scripts/appfw product model-status --json",
        "scripts/appfw product scaffold-model --dry-run --json",
        "scripts/appfw framework intake-proof --json",
        "scripts/appfw framework docs-check --json",
        "scripts/appfw framework provider-test --provider <provider>",
        "scripts/appfw product handoff --json",
        "/framework-pr-review --comprehensive",
        "/framework-research-refresh",
        "/product-pr-review --comprehensive",
        "/check-pr-review-performance",
        "PR Review Performance Oversight",
        "Recommendation Summary",
        "severity counts",
        "Branch Integration Model",
        "branch naming",
    ],
    "docs/start/framework-research-steward-harness.md": [
        "Framework Research Steward Harness",
        "/framework-research-refresh",
        "framework-research-steward",
        "Research Verdict",
        "Business Value Implications",
        "Challenge To Current Guidance",
        "Recommended Product Actions",
        "Evidence Quality",
        "Anti-Fad Filter",
        "Strategist/Product Manager",
        "Product Owner",
        "target/appfw/research-refresh.md",
    ],
    "docs/start/generated-ownership.md": [
        "scripts/appfw product topology --json",
        "scripts/appfw product generate --check --json",
        "scripts/appfw framework generate --check --json",
        "scripts/appfw product manifest --json",
        "scripts/appfw product handoff --json",
    ],
    "docs/lifecycle/legacy-modernization.md": [
        "Legacy Application Modernization",
        "Static Analysis Checklist",
        "Data Discovery Checklist",
        "Stored Procedure Disposition",
        "--source-kind legacy",
        ".appfw/legacy-modernization.yaml",
        ".appfw/legacy-analysis.yaml",
        ".appfw/model-proposal.yaml",
        "target/appfw/product-analysis.json",
        "target/appfw/model-proposal.json",
        "scripts/appfw product analyze --summary --json",
        "scripts/appfw product propose-model --summary --json",
        "scripts/appfw product model-status --json",
        "scripts/appfw product scaffold-model --dry-run --json",
        "review_decisions",
        "model_clues",
        "scripts/appfw product validate --json",
        "scripts/appfw product generate --check --json",
        "scripts/appfw product handoff --json",
    ],
    "docs/release/evidence-matrix.md": [
        "Release Evidence Matrix",
        "Scoring Rubric",
        "Executive Tree",
    ],
    "docs/architecture/concerns/packaging.md": [
        "Packaging And Product Boundary",
        "Product apps own",
        "Framework packages own",
    ],
    "docs/architecture/concerns/runtime-modularity.md": [
        "Runtime Modularity",
        "independently loadable",
        "one governed runtime operation path",
    ],
    "docs/architecture/concerns/agentic-code-provenance.md": [
        "Agentic Code Provenance And AIBOM",
        "scripts/appfw framework aibom-check --json",
        "target/appfw/aibom-check.json",
        "appfw.aibom.release-attestation.v1",
        "Agent-Assisted",
        "AIBOM-Artifact",
        "Human-Review",
        "Evidence",
    ],
    "docs/runtime/saas-freshness-lineage.md": [
        "SaaS Freshness And Lineage",
        "scripts/appfw framework saas-lineage --json",
        "target/appfw/saas-freshness-lineage.json",
        "target/appfw/saas-freshness-lineage-enforced.json",
        "appfw.saas.freshness-lineage.v1",
        "APPFW_SAAS_LINEAGE_EVIDENCE_FILE",
    ],
    "docs/architecture/concerns/north-star-wave-0.md": [
        "North-Star Wave 0 Contract Freeze",
        "Wave 0 edits are serial",
        "SaasReadArea",
        "DelegatedActorContext",
        "TokenStoreIsolation",
        "NamedMutationRegistry",
        "scripts/appfw product generate --target mobile-rn --json",
        "scripts/appfw product mobile-test --json",
        "scripts/appfw framework fork-check --json",
        "scripts/appfw framework governance-check --json",
        "Wave 0 Spec Artifact Contracts",
        "scripts/ci/release-evidence-check.sh",
        "scripts/ci/security-assurance-decision.sh",
    ],
    "docs/architecture/concerns/north-star-wave-0-specs.md": [
        "Wave 0 Spec Artifact Contracts",
        "target/appfw/governed-write-evidence.json",
        "target/appfw/pds-component-check.json",
        "target/appfw/agentic-threat-model-check.json",
        "target/appfw/governance-check.json",
        ".appfw/target/appfw/harness-check.json",
        "target/appfw/provider-graduation.json",
        ".appfw/target/appfw/mobile-test.json",
        "target/appfw/fork-check.json",
        "target/appfw/composition-check.json",
        "ASI01",
        "ASI02",
        "ASI03",
        "governed-action",
        "release-evidence-check.sh",
        "security-assurance-decision.sh",
    ],
    "docs/runtime/saas-connectors.md": [
        "Three Supported Paths",
        "Path A: Live External API Read",
        "Path B: Materialized Projection",
        "Path B Kappa/CDC Mode",
        "Path C: Governed Write-Back",
        "RuntimeSaasRequestExecutor",
        "principal envelope",
        "echo-loop prevention",
        "SaasReadArea",
        "LiveCertified",
    ],
    "docs/release/roadmap.md": [
        "Engineering Assurance Inventory",
        "Release Evidence Inventory",
        "Production Exit Checklist",
        "Highest Priority Work",
    ],
    "docs/strategy/product-development-north-star.md": [
        "Product Development Strategy",
        "North Star Statement",
        "2026 Industry Convergence Review",
        "Framework code",
        "Product code",
        "governed digital workers",
        "Grounding, permissions, and data governance",
        "Agent governance is runtime architecture",
        "Secure-by-design now includes agent-specific threats",
        "Coding-agent productivity depends on small slices",
        "Employee experience platforms are credible product surfaces",
        "contract-backed",
        "evidence-gated",
    ],
    "docs/release/deployment-reference.md": [
        "Deployment Path For Agents",
        "release-check",
        "release-evidence-check",
        "APPFW_RELEASE_REQUIRE_PERFORMANCE_EVIDENCE",
        "APPFW_RELEASE_REQUIRE_LIVE_OPS_EVIDENCE",
    ],
    "docs/architecture/concerns/maintainability.md": [
        "Treat docs like code",
        "Product Developer",
        "Framework Steward",
        "branch naming convention",
        "branch prefixes describe the work stream",
        "product-poc-intake",
        "product-legacy-modernization",
        "scripts/appfw product <command>",
        "scripts/appfw framework <command>",
        "Product Delivery Lifecycle",
        "0013-docs-information-architecture.md",
        "Agent Skills Pack",
        "Maintainability Moves",
        "Framework Structure Steward",
        "Architect-Structure-Product Triage",
        "Structure Steward detects and recommends",
        "Architect evaluates feasibility and sequencing",
        "Product Owner prioritizes",
        "Strategist challenges strategic fit",
        "RuntimeIngress",
        "RuntimeHostPlan",
        "RuntimeOperationDispatcher",
        "appfw_runtime::ingress",
        "Golden Downstream CI",
        "scripts/appfw framework intake-proof --json",
        "target/appfw/product-intake-proof.json",
        "scripts/appfw framework golden-downstream --json",
        "Frontend Scaffold Execution",
        "scripts/appfw framework docs-check --json",
        "scripts/appfw framework cli-test --json",
    ],
    "docs/architecture/concerns/agentic-threat-model.md": [
        "Agentic Threat Model",
        "ASI01",
        "ASI02",
        "ASI03",
        "filesystem sandbox",
        "network sandbox",
        "Human Review Checkpoints",
        "scripts/appfw product handoff --json",
        "scripts/appfw framework handoff --json",
        ".appfw/target/appfw/harness-check.json",
        "principal_type",
        "on_behalf_of",
        "governed writes remain unsupported until G1 evidence",
    ],
    "docs/architecture/overview.md": [
        "Runtime Ingress And Operation Invocation",
        "RuntimeHostPlan",
        "RuntimeOperationDispatcher",
        "appfw_runtime::ingress",
        "trusted ingress",
    ],
    "docs/lifecycle/product-golden-path.md": [
        "Legacy Application Modernization",
        ".appfw/legacy-modernization.yaml",
        "scripts/appfw product analyze --summary --json",
        "scripts/appfw product propose-model --summary --json",
        "scripts/appfw product scaffold-model --dry-run --json",
        "scripts/appfw framework intake-proof --json",
        "target/appfw/product-intake-proof.json",
        "stored-procedure disposition",
        "Prove The Golden Downstream Path",
        "scripts/appfw framework golden-downstream --json",
        "target/appfw/golden-downstream.json",
        "scripts/appfw product frontend-test --json",
        "scripts/appfw product load-test-suite --json",
        "target/appfw/maintainability-contract.json",
    ],
    "docs/start/cli-quickstart.md": [
        "CLI Quickstart",
        "scripts/appfw product <command>",
        "scripts/appfw framework <command>",
        "scripts/appfw context --json",
        "scripts/appfw lifecycle --json",
        "scripts/appfw framework instructions",
        "appfw_instruction_route@1",
        "scripts/appfw product validate --json",
        "scripts/appfw product analyze --summary --json",
        "scripts/appfw product propose-model --summary --json",
        "scripts/appfw product scaffold-model --dry-run --json",
        "scripts/appfw framework intake-proof --json",
        "scripts/appfw framework docs-check --changed-only --json",
        "docs-check --full --json",
        "scripts/appfw framework cli-test --json",
        "scripts/appfw product skills --json",
        "scripts/appfw framework skills --json",
    ],
    "docs/start/cli.md": [
        "CLI Router",
        "CLI Quickstart",
        "CLI Reference",
        "scripts/appfw product <command>",
        "scripts/appfw framework <command>",
    ],
    "docs/reference/cli.md": [
        "App Framework CLI Reference",
        "Audience Namespaces",
        "scripts/appfw product <command>",
        "scripts/appfw framework <command>",
        "Current-Task Instruction Routing",
        "scripts/appfw framework instructions",
        "appfw_instruction_route@1",
        "legacy-modernization",
        "--source-kind legacy",
        ".appfw/legacy-modernization.yaml",
        ".appfw/poc-analysis.yaml",
        ".appfw/legacy-analysis.yaml",
        ".appfw/model-proposal.yaml",
        "target/appfw/product-analysis.json",
        "target/appfw/model-proposal.json",
        "target/appfw/product-intake-proof.json",
        "scripts/appfw product analyze --summary --json",
        "scripts/appfw product propose-model --summary --json",
        "scripts/appfw product scaffold-model --dry-run --json",
        "scripts/appfw framework intake-proof --json",
        "product-legacy-modernization",
        "model_clues",
        "scripts/appfw context --json",
        "scripts/appfw lifecycle --json",
        "Human Display",
        "APPFW_COLOR",
        "NO_COLOR",
        "scripts/appfw skills",
        "dependency-check",
        "dependency-plan",
        "dependency-upgrade",
        "scripts/appfw product harness-check --json",
        "scripts/appfw framework provider-graduation --json",
        "scripts/appfw product mobile-test --plan --json",
        "scripts/appfw framework fork-check --plan --json",
        "scripts/appfw framework composition-check --plan --json",
        "scripts/appfw product compat-verify --plan --json",
        "scripts/appfw framework governance-check --plan --json",
        "scripts/appfw framework saas-lineage --json",
        "target/appfw/saas-freshness-lineage.json",
        "scripts/appfw framework wave2-status --json",
        "target/appfw/wave2-readiness.json",
        "reserved-wave-0-contract",
        "frontend-test",
        "load-test-suite",
        "pds-baseline",
        "provider-performance",
        "release-check",
        "scripts/appfw framework local-live-preflight --plan --json",
        "target/appfw/local-live-release-preflight-plan.json",
        "required_live_inputs",
        "external_release_authority_gates",
        "local_preflight_satisfies",
        "local_preflight_satisfies_release",
        "managed-release-ci",
        "external_api_providers",
        "governed_write_certified",
        "target/appfw/governed-write-evidence.json",
        "docs-check",
        "slowest_subcheck",
        "item_count",
        "detail",
        "focused_rerun_command",
        "parallelization_guidance",
        "generate-check-timing.json",
        "new --list-profiles",
        "intake-proof",
        "golden-downstream",
    ],
    "docs/specs/afs-008-current-task-instruction-routing.md": [
        "AFS-008 Current-Task Instruction Routing",
        "accepted-for-implementation",
        "appfw_instruction_route@1",
        "Class C",
        "comprehensive",
        "generic inheritance",
    ],
    "docs/release/pds-security-baseline-traceability.md": [
        "PDS Security Baseline Traceability",
        "PDS Health IT Security Baseline Standard",
        "LIVE-015",
        "LIVE-016",
        "LIVE-017",
        "LIVE-018",
        "release_ready",
    ],
}

layers = [
    "Start Here",
    "Product Delivery Lifecycle",
    "Reference",
    "Internal Architecture",
    "Scorecard",
    "Archive",
]

missing_docs = [
    path for path in required_docs if not (repo_root / path).is_file()
]
missing_tokens = []
for path, tokens in required_tokens.items():
    doc_path = repo_root / path
    text = doc_path.read_text(encoding="utf-8") if doc_path.is_file() else ""
    for token in tokens:
        if token not in text:
            missing_tokens.append({"document": path, "token": token})

delivery_mode_docs = [
    "docs/specs/app-framework-delivery-profiles.md",
    "docs/specs/README.md",
    "docs/start/delivery-profiles.md",
    "docs/reference/cli.md",
    "docs/start/cli-quickstart.md",
    "agent_skills/README.md",
    "docs/start/team-harness-replication.md",
]
delivery_mode_required_wording = "each worktree's Git directory"
delivery_mode_forbidden_claims = [
    "Git's common directory",
    "Git-common-dir",
    "shared state",
    "shared mutable state",
    "shared local worktree profile",
    "shared by Git worktrees",
    "linked worktrees observe one local mode",
]
delivery_mode_contract_violations = []
for path in delivery_mode_docs:
    doc_path = repo_root / path
    text = doc_path.read_text(encoding="utf-8") if doc_path.is_file() else ""
    if delivery_mode_required_wording not in text:
        delivery_mode_contract_violations.append(
            {
                "document": path,
                "missing": delivery_mode_required_wording,
            }
        )
    lower_text = text.lower()
    for claim in delivery_mode_forbidden_claims:
        if claim.lower() in lower_text:
            delivery_mode_contract_violations.append(
                {
                    "document": path,
                    "forbidden": claim,
                }
            )

delivery_mode_portable_docs = [
    "docs/specs/app-framework-delivery-profiles.md",
    "docs/start/delivery-profiles.md",
    "docs/reference/cli.md",
    "docs/start/cli-quickstart.md",
    "docs/start/team-harness-replication.md",
]
delivery_mode_portable_text = "\n".join(
    (repo_root / path).read_text(encoding="utf-8")
    for path in delivery_mode_portable_docs
).lower()
delivery_mode_portable_claims = [
    "tracked `accelerated` default",
    "side-effect-free",
    "fresh primary",
    "linked worktree",
    "mode set",
    "clean checkout",
    "exact source sha",
    "half-present",
    "malformed",
    "noncanonical",
    "dirty or stale",
    "json stdout",
    "final retained artifact",
    "delivery_profile",
    "pre-push",
]
missing_delivery_mode_portable_claims = [
    claim
    for claim in delivery_mode_portable_claims
    if claim not in delivery_mode_portable_text
]

required_skills = [
    "product-bootstrap",
    "product-poc-intake",
    "product-legacy-modernization",
    "product-schema-modeling",
    "product-generate-verify",
    "product-local-run-test",
    "product-frontend",
    "product-mobile-react-native",
    "product-agent-harness",
    "product-release-evidence",
    "product-pr-review",
    "pds-sra-package",
    "product-upgrade",
    "product-handoff",
    "framework-docs-ia",
    "framework-frontend-design-system",
    "framework-generator",
    "framework-runtime-ingress",
    "framework-provider-certification",
    "framework-release-certification",
    "framework-dependency-maintenance",
    "framework-pr-review",
    "framework-structure-steward",
    "framework-research-steward",
    "framework-handoff",
]
missing_skill_files = []
invalid_skills = []
for skill in required_skills:
    skill_path = repo_root / "agent_skills" / skill / "SKILL.md"
    if not skill_path.is_file():
        missing_skill_files.append(skill_path.relative_to(repo_root).as_posix())
        continue
    text = skill_path.read_text(encoding="utf-8")
    checks = {
        "frontmatter_name": f"name: {skill}" in text,
        "frontmatter_audience": "audience:" in text,
        "frontmatter_phase": "phase:" in text,
        "frontmatter_cli_namespace": "cli_namespace:" in text,
        "frontmatter_artifacts": "artifacts:" in text,
        "frontmatter_description": "description:" in text,
        "procedure": "## Procedure" in text,
        "proof": "## Proof" in text,
        "guardrails": "## Guardrails" in text,
    }
    missing = [name for name, ok in checks.items() if not ok]
    if missing:
        invalid_skills.append(
            {
                "skill": skill,
                "path": skill_path.relative_to(repo_root).as_posix(),
                "missing": missing,
            }
        )

readme = (repo_root / "docs/README.md").read_text(encoding="utf-8")
missing_layers = [layer for layer in layers if layer not in readme]
extra_root_docs = sorted(
    path.relative_to(repo_root).as_posix()
    for path in (repo_root / "docs").glob("*.md")
    if path.name != "README.md"
)

report = {
    "command": "maintainability-contract",
    "ok": not missing_docs
    and not missing_tokens
    and not delivery_mode_contract_violations
    and not missing_delivery_mode_portable_claims
    and not missing_layers
    and not extra_root_docs
    and not missing_skill_files
    and not invalid_skills,
    "source_docs": required_docs,
    "required_tokens": required_tokens,
    "required_skills": required_skills,
    "layers": layers,
    "checks": {
        "docs_exist": not missing_docs,
        "required_tokens_present": not missing_tokens,
        "delivery_mode_docs_are_worktree_local": not delivery_mode_contract_violations,
        "delivery_mode_docs_cover_portable_default": not missing_delivery_mode_portable_claims,
        "readme_layers_present": not missing_layers,
        "docs_root_has_only_readme": not extra_root_docs,
        "agent_skills_present": not missing_skill_files,
        "agent_skills_have_minimal_anatomy": not invalid_skills,
        "roadmap_is_current_scorecard": True,
        "historical_ledger_pruned_from_live_roadmap": True,
        "deployment_path_is_stage_separated": True,
        "RuntimeIngress": "shared runtime ingress contract is documented",
        "RuntimeHostPlan": "independent runtime ingress loading is documented",
        "RuntimeOperationDispatcher": "shared operation dispatch boundary is documented",
        "golden-downstream": "golden downstream profile evidence command is documented",
        "skills": "agent skills are discoverable from scripts/appfw skills --json",
        "product-framework-seam": "docs, skills, and CLI distinguish product development from framework stewardship",
        "pds-design-system": "PDS Health design-system docs, skill, and token source are discoverable",
        "load-test-suite": "performance evidence command is documented",
    },
    "missing_docs": missing_docs,
    "missing_tokens": missing_tokens,
    "delivery_mode_contract_violations": delivery_mode_contract_violations,
    "missing_delivery_mode_portable_claims": missing_delivery_mode_portable_claims,
    "missing_layers": missing_layers,
    "extra_root_docs": extra_root_docs,
    "missing_skill_files": missing_skill_files,
    "invalid_skills": invalid_skills,
}
print(json.dumps(report, indent=2, sort_keys=True))
if not report["ok"]:
    raise SystemExit(1)
PY
  local status=$?
  set -e

  local ok=false
  if [[ $status -eq 0 ]]; then
    ok=true
  else
    DOC_EXAMPLE_FAILED=1
    if [[ "$json" != "1" ]]; then
      cat "$log_file" >&2 || true
    fi
  fi
  local elapsed_ms
  elapsed_ms=$(( $(ms_now) - start_ms ))
  DOC_EXAMPLE_ITEMS+=("$name|$ok|$status|$log_file|$command_text|$elapsed_ms")
  progress_event "finish" "$name" "$elapsed_ms" "$ok"
}

write_lifecycle_evidence_checklist() {
  local checklist_file="$report_dir/lifecycle-evidence-checklist.json"
  local start_ms
  start_ms="$(ms_now)"

  cat >"$checklist_file" <<'JSON'
{
  "schema_version": 1,
  "name": "appfw-lifecycle-evidence-checklist",
  "checked_by": "scripts/appfw framework docs-check --json",
  "source_docs": [
    "docs/lifecycle/application-lifecycle.md",
    "docs/reference/product-workspace-contract.md",
    "docs/start/agent-task-map.md",
    "docs/start/cli-quickstart.md",
    "docs/reference/cli.md"
  ],
  "steps": [
    {
      "step": "create",
      "required_commands": [
        "scripts/appfw product new --list-profiles --json",
        "scripts/appfw product new <target> --from current --profile <profile> --generate --json",
        "scripts/appfw product validate --json",
        "scripts/appfw product handoff --json"
      ],
      "required_artifacts": [
        "profile list JSON",
        "bootstrap JSON",
        "appfw.lock",
        "validation JSON",
        "target/appfw/agent-handoff.json"
      ],
      "docs_check_coverage": [
        "new-list-profiles-json",
        "new-list-profiles-profile-path",
        "new-list-profiles-verification",
        "new-list-profiles-generate-check"
      ],
      "ci_gap": "Golden downstream CI still needs to run appfw new because it writes a new app root."
    },
    {
      "step": "customize",
      "required_commands": [
        "scripts/appfw product explain ownership <path> --json",
        "scripts/appfw product topology --json",
        "scripts/appfw product validate --json",
        "scripts/appfw product boundary-check --json",
        "scripts/appfw product test --fast",
        "scripts/appfw product handoff --json"
      ],
      "required_artifacts": [
        "ownership explanation JSON",
        "topology JSON",
        "validation JSON",
        "boundary-check JSON",
        "focused test result",
        "target/appfw/agent-handoff.json"
      ],
      "docs_check_coverage": [
        "explain-ownership-json",
        "topology-json",
        "validate-json",
        "boundary-check-json",
        "handoff-json"
      ]
    },
    {
      "step": "generate",
      "required_commands": [
        "scripts/appfw product validate --json",
        "scripts/appfw product generate",
        "scripts/appfw product generate --check --json",
        "scripts/appfw product test",
        "scripts/appfw product handoff --json"
      ],
      "required_artifacts": [
        ".appfw/target/appfw/artifacts.json",
        ".appfw/target/appfw/artifact_provenance.json",
        "generated diff review",
        "generate --check JSON",
        "test result",
        "target/appfw/agent-handoff.json"
      ],
      "docs_check_coverage": [
        "validate-json",
        "generate-check-json",
        "handoff-json"
      ]
    },
    {
      "step": "test",
      "required_commands": [
        "scripts/appfw product test --fast",
        "scripts/appfw product test",
        "scripts/appfw product api-test"
      ],
      "required_artifacts": [
        "unit or compile result",
        "generated API scenario result",
        "skipped live-test reason when applicable"
      ],
      "ci_gap": "Risk-specific test jobs and live API tests stay outside docs-check."
    },
    {
      "step": "release",
      "required_commands": [
        "scripts/appfw product migrate plan --json",
        "scripts/appfw product migrate lint --phase all --json",
        "scripts/appfw product migrate drift --json",
        "scripts/appfw product migrate rollback-guide --json",
        "scripts/appfw product handoff --json"
      ],
      "required_artifacts": [
        "migration plan JSON",
        "migration lint JSON",
        "migration drift JSON or skipped reason",
        "rollback guide JSON",
        "security and deployment notes",
        "target/appfw/agent-handoff.json"
      ],
      "ci_gap": "Provider-backed drift belongs in product or release CI with reachable data sources."
    },
    {
      "step": "upgrade",
      "required_commands": [
        "scripts/appfw product upgrade --json",
        "scripts/appfw product validate --json",
        "scripts/appfw product generate",
        "scripts/appfw product generate --check --json",
        "scripts/appfw product test",
        "scripts/appfw product lock --write",
        "scripts/appfw product upgrade --json",
        "scripts/appfw product handoff --json"
      ],
      "required_artifacts": [
        "initial target/appfw/product-upgrade.json",
        "generated drift decision",
        "refreshed appfw.lock",
        "final target/appfw/product-upgrade.json",
        "test result",
        "target/appfw/agent-handoff.json"
      ],
      "ci_gap": "Golden downstream upgrade CI still needs a disposable product checkout because lock refresh writes files."
    },
    {
      "step": "drift",
      "required_commands": [
        "scripts/appfw product generate --check --json",
        "scripts/appfw product migrate drift --json"
      ],
      "required_artifacts": [
        "generated drift JSON",
        "migration drift JSON or skipped reason",
        "owner, provider, reason, and next action"
      ],
      "docs_check_coverage": [
        "generate-check-json"
      ]
    },
    {
      "step": "handoff",
      "required_commands": [
        "scripts/appfw product handoff --json"
      ],
      "required_artifacts": [
        "target/appfw/agent-handoff.json",
        "commands run",
        "skipped checks",
        "remaining risks"
      ],
      "docs_check_coverage": [
        "handoff-json"
      ]
    }
  ]
}
JSON

  local elapsed_ms
  elapsed_ms=$(( $(ms_now) - start_ms ))
  DOC_EXAMPLE_ITEMS+=("lifecycle-evidence-checklist|true|0|$checklist_file|write target/appfw/lifecycle-evidence-checklist.json|$elapsed_ms")
}

DOC_EXAMPLE_FAILED=0
DOC_EXAMPLE_ITEMS=()
effective_mode="$mode"
mode_reason="explicit ${mode}"
changed_files_count=0
changed_surface_files=""
changed_surface_triggers=""
changed_surface_trigger_count=0
changed_surface_requires_full_result=false

json_array_from_lines() {
  local lines="$1"
  local first=1
  printf '['
  while IFS= read -r item; do
    [[ -n "$item" ]] || continue
    if [[ "$first" != "1" ]]; then
      printf ','
    fi
    json_escape "$item"
    first=0
  done <<<"$lines"
  printf ']'
}

json_trigger_array_from_lines() {
  local lines="$1"
  local first=1
  local path rule
  printf '['
  while IFS='|' read -r path rule; do
    [[ -n "$path" ]] || continue
    if [[ "$first" != "1" ]]; then
      printf ','
    fi
    printf '{"path":'
    json_escape "$path"
    printf ',"rule":'
    json_escape "$rule"
    printf '}'
    first=0
  done <<<"$lines"
  printf ']'
}

write_changed_surface_artifact() {
  {
    printf '{"command":"docs-check-changed-surface","ok":true'
    printf ',"requested_mode":'
    json_escape "$mode"
    printf ',"selected_mode":'
    json_escape "$effective_mode"
    printf ',"mode_reason":'
    json_escape "$mode_reason"
    printf ',"base_ref":'
    json_escape "$base_ref"
    printf ',"changed_files_count":%s' "$changed_files_count"
    printf ',"requires_full":%s' "$changed_surface_requires_full_result"
    printf ',"trigger_count":%s' "$changed_surface_trigger_count"
    printf ',"changed_files":'
    json_array_from_lines "$changed_surface_files"
    printf ',"triggers":'
    json_trigger_array_from_lines "$changed_surface_triggers"
    printf '}\n'
  } >"$docs_check_changed_surface_artifact"
}

collect_changed_files() {
  local diff_output=""
  if git -C "$repo_root" rev-parse --verify "$base_ref" >/dev/null 2>&1; then
    diff_output="$(
      {
        git -C "$repo_root" diff --name-only "$base_ref"...HEAD
        git -C "$repo_root" diff --cached --name-only
        git -C "$repo_root" diff --name-only
        git -C "$repo_root" ls-files --others --exclude-standard
      } | sort -u
    )"
  else
    return 1
  fi

  printf '%s\n' "$diff_output" \
    | grep -Ev '(^|/)product_dist/|^target/|^\.git/' \
    | sed '/^[[:space:]]*$/d' \
    || true
}

changed_surface_generic_rule_for_path() {
  local path="$1"
  case "$path" in
    scripts/ci/pre-push-review-guard.sh|scripts/ci/wave3-pr-gates.sh|scripts/ci/pr-fast-framework-check.sh|scripts/ci/local-pre-push-gates.sh)
      return 1
      ;;
    scripts/appfw|scripts/check-doc-examples.sh|scripts/appfw-delivery-mode.py|scripts/check-product-increment-plan.mjs|scripts/check-product-increment-portfolio.mjs|scripts/check-nexus-workstreams.mjs|scripts/product-increment-path-safety.mjs|scripts/ci/*|scripts/git-hooks/*|docs/start/delivery-profiles.json)
      printf 'cli-or-docs-check-behavior\n'
      ;;
    appfw_cli/*|app_gen/src/*|app_gen/_templates/*|app_gen/_config/*)
      printf 'generator-or-cli-contract\n'
      ;;
    appfw_runtime/src/security.rs|appfw_runtime/src/ingress.rs|appfw_runtime/src/lib.rs)
      printf 'runtime-security-or-ingress-contract\n'
      ;;
    docs/reference/cli.md|docs/start/cli-quickstart.md|docs/start/agent-task-map.md)
      printf 'agent-facing-command-contract-doc\n'
      ;;
    docs/architecture/concerns/maintainability.md|docs/architecture/adr/*)
      printf 'architecture-or-docs-ia-contract\n'
      ;;
    agent_skills/*)
      printf 'agent-skill-contract\n'
      ;;
    *)
      return 1
      ;;
  esac
}

collect_changed_surface_triggers() {
  local changed_files classified_files classified_paths path rule
  changed_files="$(cat)"
  if ! classified_files="$(
    printf '%s\n' "$changed_files" |
      node "$repo_root/scripts/product-increment-path-safety.mjs" \
        --classify-docs-surface-batch
  )"; then
    printf 'batch-classifier-failed|batch-classifier-failed\n'
    return 0
  fi

  classified_paths="$(
    printf '%s\n' "$classified_files" |
      sed '/^[[:space:]]*$/d' |
      cut -d'|' -f1
  )"
  if [[ "$classified_paths" != "$changed_files" ]]; then
    printf 'batch-classifier-invalid-output|batch-classifier-invalid-output\n'
    return 0
  fi

  while IFS='|' read -r path rule; do
    [[ -n "$path" ]] || continue
    case "$rule" in
      focused:product-increment-delivery|product-increment-authority-contract)
        ;;
      "")
        if ! rule="$(changed_surface_generic_rule_for_path "$path")"; then
          continue
        fi
        ;;
      *)
        printf 'batch-classifier-invalid-rule|batch-classifier-invalid-rule\n'
        return 0
        ;;
    esac
    printf '%s|%s\n' "$path" "$rule"
  done <<<"$classified_files"
}

run_product_increment_path_safety_tests() {
  run_example \
    "product-increment-path-safety-tests" \
    node "$repo_root/scripts/product-increment-path-safety.test.mjs"
}

select_effective_mode() {
  if [[ "$mode" != "changed-only" ]]; then
    return
  fi

  local changed_files
  if ! changed_files="$(collect_changed_files)"; then
    effective_mode="full"
    mode_reason="changed-only could not inspect ${base_ref}; selected full"
    changed_surface_requires_full_result=true
    changed_surface_triggers="base-ref-unavailable|base-ref-unavailable"
    changed_surface_trigger_count=1
    return
  fi

  changed_surface_files="$changed_files"
  changed_surface_triggers="$(printf '%s\n' "$changed_files" | collect_changed_surface_triggers)"
  changed_surface_trigger_count="$(printf '%s\n' "$changed_surface_triggers" | sed '/^[[:space:]]*$/d' | wc -l | tr -d ' ')"
  changed_files_count="$(printf '%s\n' "$changed_files" | sed '/^[[:space:]]*$/d' | wc -l | tr -d ' ')"
  if [[ "$changed_files_count" == "0" ]]; then
    effective_mode="fast"
    mode_reason="changed-only found no changed files; selected fast"
    return
  fi

  if [[ "$changed_surface_trigger_count" != "0" ]]; then
    local trigger_rules
    trigger_rules="$(printf '%s\n' "$changed_surface_triggers" | sed '/^[[:space:]]*$/d' | cut -d'|' -f2 | sort -u)"
    if [[ "$trigger_rules" == "focused:product-increment-delivery" && "$changed_surface_trigger_count" == "$changed_files_count" && -z "$focused_subcheck" ]]; then
      effective_mode="fast"
      focused_subcheck="product-increment-delivery"
      mode_reason="changed-only found only Product Increment plan records; selected focused product-increment-delivery"
    else
      effective_mode="full"
      changed_surface_requires_full_result=true
      mode_reason="changed-only found CLI/generator/docs-contract changes; selected full"
    fi
  else
    effective_mode="fast"
    mode_reason="changed-only found docs or generated-output-only changes; selected fast"
  fi
}

run_shell_syntax_checks() {
  run_example "shell-appfw" bash -n "$repo_root/scripts/appfw"
  run_example "shell-doc-examples" bash -n "$repo_root/scripts/check-doc-examples.sh"
  run_example "shell-cli-semantic-gates" bash -n "$repo_root/scripts/cli-semantic-gates-test.sh"
  run_example "shell-wave3-pr-gates" bash -n "$repo_root/scripts/ci/wave3-pr-gates.sh"
  run_example "shell-pr-fast-framework-check" bash -n "$repo_root/scripts/ci/pr-fast-framework-check.sh"
  run_example "shell-local-pre-push-gates" bash -n "$repo_root/scripts/ci/local-pre-push-gates.sh"
  run_example "shell-bitbucket-api-smoke" bash -n "$repo_root/scripts/ci/bitbucket-api-smoke.sh"
  run_example "shell-install-node" bash -n "$repo_root/scripts/ci/install-node.sh"
  run_example "shell-prepare-appfw-evidence-root" bash -n "$repo_root/scripts/ci/prepare-appfw-evidence-root.sh"
  run_example "shell-pre-push-review-guard" bash -n "$repo_root/scripts/ci/pre-push-review-guard.sh"
  run_example "shell-install-local-git-hooks" bash -n "$repo_root/scripts/ci/install-local-git-hooks.sh"
  run_example "shell-git-pre-push-hook" bash -n "$repo_root/scripts/git-hooks/pre-push"
}

run_lifecycle_docs_check() {
  write_lifecycle_evidence_checklist
  run_log_assertion "lifecycle-checklist-create" "$report_dir/lifecycle-evidence-checklist.json" '"step": "create"' "lifecycle checklist maps Create"
  run_log_assertion "lifecycle-checklist-generate-check" "$report_dir/lifecycle-evidence-checklist.json" '"scripts/appfw product generate --check --json"' "lifecycle checklist maps generated drift check"
  run_log_assertion "lifecycle-checklist-handoff-artifact" "$report_dir/lifecycle-evidence-checklist.json" '"target/appfw/agent-handoff.json"' "lifecycle checklist records handoff artifact"
}

run_design_system_docs_check() {
  run_example "pds-token-drift-json" node "$repo_root/scripts/check-pds-tokens.mjs" --json
  run_log_assertion "pds-token-drift-ok" "$report_dir/docs-example-pds-token-drift-json.log" '"ok": true' "PDS token drift check reports ok"
  run_example "pds-component-check-json" node "$repo_root/scripts/check-pds-components.mjs" --json
  run_log_assertion "pds-component-check-ok" "$report_dir/docs-example-pds-component-check-json.log" '"ok": true' "PDS component check reports ok"
  run_log_assertion "pds-component-check-governed-action-live-readiness" "$report_dir/docs-example-pds-component-check-json.log" '"governed_action_live_readiness"' "PDS component check retains governed-action live-readiness"
  run_log_assertion "pds-component-check-flow-graph-token-bridge" "$report_dir/docs-example-pds-component-check-json.log" '"flow_graph_token_bridge"' "PDS component check retains FlowGraph token bridge evidence"
  run_log_assertion "pds-component-check-flow-graph-token-bridge-not-live" "$report_dir/docs-example-pds-component-check-json.log" '"adapter_ready": false' "PDS FlowGraph token bridge stays adapter-gated"
  run_log_assertion "pds-component-check-markdown-sanitizer" "$report_dir/docs-example-pds-component-check-json.log" '"conversation_markdown_sanitizer"' "PDS component check retains CH5 markdown sanitizer decision evidence"
  run_log_assertion "pds-component-check-markdown-sanitizer-not-live" "$report_dir/docs-example-pds-component-check-json.log" '"status": "decision-recorded-not-wired"' "PDS markdown sanitizer evidence stays not wired for live chat"
  run_example_with_allowed_statuses "0 1" "pds-component-check-enforce-governed-action-json" node "$repo_root/scripts/check-pds-components.mjs" --json --artifact-path "$report_dir/docs-example-pds-component-check-enforce-governed-action-json-artifact.json" --enforce-governed-action
  run_log_assertion "pds-component-check-enforce-governed-action-live-ready" "$report_dir/docs-example-pds-component-check-enforce-governed-action-json.log" '"live_ready": false' "PDS governed-action enforcement emits parseable JSON while live evidence is missing"
  run_log_assertion "pds-component-check-enforce-governed-action-missing-evidence" "$report_dir/docs-example-pds-component-check-enforce-governed-action-json.log" '"g1-governed-write-live-evidence"' "PDS governed-action enforcement reports missing G1 live evidence"

  local thin_governed_write_evidence="$report_dir/docs-check-thin-governed-write-evidence.json"
  python3 - "$thin_governed_write_evidence" <<'PY'
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
path.parent.mkdir(parents=True, exist_ok=True)
path.write_text(
    json.dumps(
        {
            "command": "provider-test",
            "lane": "G1",
            "ok": True,
            "provider": "servicenow",
            "operation": "named_mutation",
        },
        indent=2,
    )
    + "\n",
    encoding="utf-8",
)
PY
  run_example_with_allowed_statuses "1" "pds-component-check-enforce-governed-action-thin-evidence-json" node "$repo_root/scripts/check-pds-components.mjs" --json --artifact-path "$report_dir/docs-example-pds-component-check-enforce-governed-action-thin-evidence-json-artifact.json" --enforce-governed-action --governed-write-live-evidence "$thin_governed_write_evidence"
  run_log_assertion "pds-component-check-enforce-governed-action-invalid-evidence" "$report_dir/docs-example-pds-component-check-enforce-governed-action-thin-evidence-json.log" '"invalid_evidence": [' "PDS governed-action enforcement rejects schema-thin G1 live evidence"
  run_log_assertion "pds-component-check-enforce-governed-action-delegated-actor-check" "$report_dir/docs-example-pds-component-check-enforce-governed-action-thin-evidence-json.log" '"delegated-actor-object"' "PDS governed-action enforcement validates delegated actor evidence shape"
}

run_maintainability_docs_check() {
  run_maintainability_contract_check
  run_log_assertion "maintainability-contract-ok" "$report_dir/maintainability-contract.json" '"ok": true' "maintainability contract reports ok"
  run_log_assertion "maintainability-contract-delivery-mode-worktree-local" "$report_dir/maintainability-contract.json" '"delivery_mode_docs_are_worktree_local": true' "delivery mode docs require worktree-local Git-directory state"
  run_log_assertion "maintainability-contract-delivery-mode-portable-default" "$report_dir/maintainability-contract.json" '"delivery_mode_docs_cover_portable_default": true' "delivery mode docs cover the side-effect-free tracked default and diagnostic source-state contract"
  run_log_assertion "maintainability-contract-runtime-ingress" "$report_dir/maintainability-contract.json" '"RuntimeIngress"' "maintainability contract covers RuntimeIngress"
  run_log_assertion "maintainability-contract-load-suite" "$report_dir/maintainability-contract.json" '"load-test-suite"' "maintainability contract covers load-test-suite"
  run_log_assertion "maintainability-contract-deployment-path" "$report_dir/maintainability-contract.json" '"Deployment Path For Agents"' "maintainability contract covers deployment path"
}

run_saas_vendor_doc_parity_docs_check() {
  run_example \
    "saas-vendor-doc-parity-json" \
    python3 "$repo_root/scripts/check-saas-vendor-doc-parity.py" "$repo_root"
  run_log_assertion \
    "saas-vendor-doc-parity-ok" \
    "$report_dir/docs-example-saas-vendor-doc-parity-json.log" \
    '"ok": true' \
    "SaaS vendor doc parity reports ok"
  run_log_assertion \
    "saas-vendor-doc-parity-workday" \
    "$report_dir/docs-example-saas-vendor-doc-parity-json.log" \
    '"provider": "workday"' \
    "SaaS vendor doc parity covers Workday"
  run_log_assertion \
    "saas-vendor-doc-parity-anaplan" \
    "$report_dir/docs-example-saas-vendor-doc-parity-json.log" \
    '"provider": "anaplan"' \
    "SaaS vendor doc parity covers Anaplan"
  run_log_assertion \
    "saas-vendor-doc-parity-artifact" \
    "$report_dir/docs-example-saas-vendor-doc-parity-json.log" \
    'target/appfw/saas-vendor-doc-parity.json' \
    "SaaS vendor doc parity writes retained artifact"
}

run_release_lite_guard_case() {
  local name="$1"
  local changed_files="$2"
  local expected_status="$3"
  local evidence_url="${4:-}"
  local approver="${5:-}"
  local reason="${6:-}"
  local log_file="$report_dir/docs-example-${name}.log"
  local command_text="assert release-lite guard case ${name}"
  local start_ms
  start_ms="$(ms_now)"

  if [[ "$json" != "1" ]]; then
    printf 'Checking docs example assertion: %s\n' "$command_text"
  fi
  progress_event "start" "$name"

  set +e
  APPFW_RELEASE_ARTIFACT_DIR="$report_dir/release-lite-${name}" \
    APPFW_RELEASE_LITE_CHANGED_FILES="$changed_files" \
    APPFW_RELEASE_LITE_EVIDENCE_URL="$evidence_url" \
    APPFW_RELEASE_LITE_APPROVER="$approver" \
    APPFW_RELEASE_LITE_REASON="$reason" \
    bash "$repo_root/scripts/ci/release-lite-guard.sh" >"$log_file" 2>&1
  local status=$?
  set -e

  local ok=false
  if [[ $status -eq $expected_status ]]; then
    ok=true
  else
    DOC_EXAMPLE_FAILED=1
    if [[ "$json" != "1" ]]; then
      cat "$log_file" >&2 || true
    fi
  fi
  local elapsed_ms
  elapsed_ms=$(( $(ms_now) - start_ms ))
  DOC_EXAMPLE_ITEMS+=("$name|$ok|$status|$log_file|$command_text|$elapsed_ms")
  progress_event "finish" "$name" "$elapsed_ms" "$ok"
}

run_release_lite_guard_docs_check() {
  run_release_lite_guard_case \
    "release-lite-guard-nonsensitive" \
    "docs/start/cli-quickstart.md" \
    0
  run_log_assertion \
    "release-lite-guard-nonsensitive-artifact" \
    "$report_dir/release-lite-release-lite-guard-nonsensitive/release-lite-guard.json" \
    '"release_lite_required": false' \
    "release-lite guard passes nonsensitive docs-only changes"

  run_release_lite_guard_case \
    "release-lite-guard-sensitive-missing-approval" \
    "scripts/appfw" \
    1
  run_log_assertion \
    "release-lite-guard-sensitive-fails-closed" \
    "$report_dir/release-lite-release-lite-guard-sensitive-missing-approval/release-lite-guard.json" \
    '"ok": false' \
    "release-lite guard fails closed for sensitive changes without approval fields"
  run_log_assertion \
    "release-lite-guard-sensitive-missing-fields" \
    "$report_dir/release-lite-release-lite-guard-sensitive-missing-approval/release-lite-guard.json" \
    '"APPFW_RELEASE_LITE_EVIDENCE_URL"' \
    "release-lite guard names missing secured evidence URL"

  run_release_lite_guard_case \
    "release-lite-guard-sensitive-approved" \
    "scripts/appfw" \
    0 \
    "https://ci.example.invalid/app-framework/release-lite/123" \
    "release-approver@example.invalid" \
    "Scoped provider-backed release-lite evidence reviewed for this PR"
  run_log_assertion \
    "release-lite-guard-sensitive-approved-ok" \
    "$report_dir/release-lite-release-lite-guard-sensitive-approved/release-lite-guard.json" \
    '"ok": true' \
    "release-lite guard passes sensitive changes with secured approval fields"
  run_log_assertion \
    "release-lite-guard-sensitive-approved-present" \
    "$report_dir/release-lite-release-lite-guard-sensitive-approved/release-lite-guard.json" \
    '"present": true' \
    "release-lite guard records approval presence"
}

run_sra_package_docs_check() {
  run_example "framework-sra-package-json" "$repo_root/scripts/appfw" framework sra-package --all-products --json
  run_log_assertion "framework-sra-package-command" "$report_dir/docs-example-framework-sra-package-json.log" '"command": "sra-package"' "framework SRA package JSON exposes command"
  run_log_assertion "framework-sra-package-scope" "$report_dir/docs-example-framework-sra-package-json.log" '"scope": "framework"' "framework SRA package JSON exposes framework scope"
  run_log_assertion "framework-sra-package-slash" "$report_dir/docs-example-framework-sra-package-json.log" '"/pds-sra-package --framework"' "framework SRA package JSON exposes slash command"
  run_log_assertion "framework-sra-package-all-products-crm" "$report_dir/docs-example-framework-sra-package-json.log" '"name": "crm-sample"' "framework SRA package includes CRM product perspective"
  run_log_assertion "framework-sra-package-all-products-nexus" "$report_dir/docs-example-framework-sra-package-json.log" '"name": "pds-nexus"' "framework SRA package includes PDS Nexus product perspective"
  run_log_assertion "framework-sra-package-data-flow" "$report_dir/docs-example-framework-sra-package-json.log" '"id": "data_flow_diagram"' "framework SRA package records Data Flow Diagram requirement"
  run_log_assertion "framework-sra-package-narrative" "$report_dir/docs-example-framework-sra-package-json.log" 'target/appfw/sra-package.md' "framework SRA package writes retained Markdown narrative"
  run_log_assertion "framework-sra-package-narrative-title" "$repo_root/target/appfw/sra-package.md" 'PDS Health Security Risk Assessment Package' "framework SRA package narrative has title"
  run_log_assertion "framework-sra-package-narrative-not-approval" "$repo_root/target/appfw/sra-package.md" 'not SRA approval' "framework SRA package narrative is not approval"
  run_example "product-sra-package-json" "$repo_root/scripts/appfw" product sra-package --json
  run_log_assertion "product-sra-package-command" "$report_dir/docs-example-product-sra-package-json.log" '"command": "sra-package"' "product SRA package JSON exposes command"
  run_log_assertion "product-sra-package-scope" "$report_dir/docs-example-product-sra-package-json.log" '"scope": "product"' "product SRA package JSON exposes product scope"
  run_log_assertion "product-sra-package-ready" "$report_dir/docs-example-product-sra-package-json.log" '"sra_ready": false' "product SRA package stays unready while human inputs are missing"
  run_log_assertion "product-sra-package-infra-diagram" "$report_dir/docs-example-product-sra-package-json.log" '"id": "infrastructure_logical_architecture_diagram"' "product SRA package records infrastructure diagram requirement"
  run_log_assertion "product-sra-package-artifact" "$report_dir/docs-example-product-sra-package-json.log" 'examples/products/crm/.appfw/target/appfw/sra-package.json' "product SRA package writes product-scoped retained artifact"
  run_log_assertion "product-sra-package-narrative-artifact" "$report_dir/docs-example-product-sra-package-json.log" 'examples/products/crm/.appfw/target/appfw/sra-package.md' "product SRA package writes product-scoped retained Markdown narrative"
  run_log_assertion "product-sra-package-narrative-perspectives" "$repo_root/examples/products/crm/.appfw/target/appfw/sra-package.md" 'Product App Perspectives' "product SRA package narrative includes product perspectives"
}

run_product_increment_delivery_docs_check() {
  run_product_increment_path_safety_tests
  run_example \
    "product-increment-assignment-topology-sharding-tests" \
    node "$repo_root/scripts/run-nexus-workstream-tests.test.mjs"
  run_example \
    "product-increment-plan-tests" \
    node "$repo_root/scripts/check-product-increment-plan.test.mjs"
  run_example \
    "product-increment-portfolio-tests" \
    node "$repo_root/scripts/check-product-increment-portfolio.test.mjs"
  if [[ "$effective_mode" == "full" ]]; then
    run_example \
      "product-increment-assignment-topology-tests" \
      node "$repo_root/scripts/run-nexus-workstream-tests.mjs"
  else
    run_example \
      "product-increment-assignment-topology-smoke" \
      node --test \
      --test-name-pattern="admission-smoke" \
      "$repo_root/scripts/check-nexus-workstreams.test.mjs"
  fi
  if [[ "$effective_mode" == "full" ]]; then
    run_example \
      "product-increment-portfolio-json" \
      node "$repo_root/scripts/check-product-increment-portfolio.mjs" \
      --portfolio "$repo_root/docs/specs/product-increment-portfolio.json" \
      --json
    run_log_assertion \
      "product-increment-portfolio-ok" \
      "$report_dir/docs-example-product-increment-portfolio-json.log" \
      '"ok": true' \
      "Product Increment portfolio and referenced plans validate"
    run_log_assertion \
      "product-increment-portfolio-router" \
      "$report_dir/docs-example-product-increment-portfolio-json.log" \
      '"id": "AFS-PI-P1"' \
      "Product Increment portfolio exposes the current router increment"
    run_log_assertion \
      "product-increment-portfolio-relations" \
      "$report_dir/docs-example-product-increment-portfolio-json.log" \
      '"benefits_from"' \
      "Product Increment portfolio exposes cross-increment benefits"
    run_log_assertion \
      "product-increment-portfolio-economics" \
      "$report_dir/docs-example-product-increment-portfolio-json.log" \
      '"tokens_per_accepted_deliverable"' \
      "Product Increment portfolio exposes accepted-throughput economics"
  fi
}

run_fast_docs_check() {
  run_docs_check_subcheck "shell-syntax" "shell syntax for docs-check-owned scripts" run_shell_syntax_checks
  run_docs_check_subcheck "lifecycle-evidence" "lifecycle checklist and retained artifact mapping" run_lifecycle_docs_check
  run_docs_check_subcheck "design-system" "PDS token and component drift evidence" run_design_system_docs_check
  run_docs_check_subcheck "security-env-doc-parity" "security environment variables match the runtime config contract" run_security_env_doc_parity
  run_docs_check_subcheck "maintainability-contract" "docs IA, skills, ingress, deployment, and command discoverability" run_maintainability_docs_check
  run_docs_check_subcheck "saas-vendor-doc-parity" "SaaS provider vendor docs are version-linked to crate metadata constants" run_saas_vendor_doc_parity_docs_check
  run_docs_check_subcheck "release-lite-guard" "PR release-lite guard fail-closed approval contract" run_release_lite_guard_docs_check
  run_docs_check_subcheck "sra-package-command-examples" "PDS SRA package command and retained artifact contract" run_sra_package_docs_check
  run_docs_check_subcheck "product-increment-delivery" "Product Increment portfolio, plans, dependencies, benefits, and delivery-credit contract" run_product_increment_delivery_docs_check
}

docs_check_selector_start_ms="$(ms_now)"
select_effective_mode
write_changed_surface_artifact
record_docs_check_phase \
  "changed-surface-selector" \
  "$(( $(ms_now) - docs_check_selector_start_ms ))" \
  true \
  "select effective docs-check mode and write changed-surface artifact"

if [[ "$json" != "1" ]]; then
  printf 'docs-check mode: requested=%s selected=%s (%s)\n' "$mode" "$effective_mode" "$mode_reason"
elif [[ "$progress" == "1" ]]; then
  printf '{"event":"docs-check-plan","requested_mode":' >&2
  json_escape "$mode" >&2
  printf ',"selected_mode":' >&2
  json_escape "$effective_mode" >&2
  if [[ -n "$focused_subcheck" ]]; then
    printf ',"focused_subcheck":' >&2
    json_escape "$focused_subcheck" >&2
  fi
  printf ',"reason":' >&2
  json_escape "$mode_reason" >&2
  printf ',"changed_files_count":%s}\n' "$changed_files_count" >&2
fi

if [[ -n "$focused_subcheck" && "$effective_mode" == "fast" ]] \
  && docs_check_subcheck_requires_full "$focused_subcheck"; then
  echo "docs-check subcheck '$focused_subcheck' requires --full; selected mode is fast" >&2
  exit 2
fi

if [[ "$plan_only" == "1" ]]; then
  if [[ "$json" == "1" ]]; then
    printf '{"command":"docs-check","ok":true,"plan_only":true,"requested_mode":'
    json_escape "$mode"
    printf ',"selected_mode":'
    json_escape "$effective_mode"
    printf ',"mode_reason":'
    json_escape "$mode_reason"
    printf ',"changed_files_count":%s' "$changed_files_count"
    printf ',"changed_surface_artifact":'
    json_escape "$docs_check_changed_surface_artifact_rel"
    printf ',"subchecks_artifact":'
    json_escape "$docs_check_subchecks_artifact_rel"
    print_docs_check_subcheck_focus_json_fields
    printf ',"changed_surface_requires_full":%s' "$changed_surface_requires_full_result"
    printf ',"changed_surface_trigger_count":%s' "$changed_surface_trigger_count"
    printf ',"budget_ms":%s' "$(docs_check_budget_ms)"
    printf ',"budget_enforced":'
    if [[ "$enforce_budget" == "1" ]]; then
      printf 'true'
    else
      printf 'false'
    fi
    printf '}\n'
  fi
  exit 0
fi

run_fast_only=0
if [[ "$effective_mode" == "fast" ]]; then
  run_fast_only=1
fi
if [[ -n "$focused_subcheck" ]] && ! docs_check_subcheck_requires_full "$focused_subcheck"; then
  run_fast_only=1
fi

if [[ "$run_fast_only" == "1" ]]; then
  run_fast_docs_check
else

# --- Prebuild docs-check binaries once --------------------------------------
# The examples below otherwise shell out to `cargo run` ~13 times across three
# binary targets. Every `cargo run` re-checks the build graph and acquires the
# global `target/` build lock, so under any concurrent compile (rust-analyzer,
# a parallel agent, CI) the examples serialize behind the lock instead of doing
# work (a single warm call has been observed blocked for ~55s). Build the
# binaries a single time here, then invoke them directly.
#
# app_gen ("appfw-codegen") and appfw-cli BOTH define a bin named `appfw` that
# maps to <cargo-target>/debug/appfw, so we build + snapshot each in turn. The
# `rm -f` forces cargo to relink the requested package's bin to that path even
# when its fingerprint is unchanged, so each snapshot is the correct binary.
docs_appfw_appgen="$cargo_debug_dir/docs-check-appfw-appgen"
docs_appfw_cli="$cargo_debug_dir/docs-check-appfw-cli"
docs_check_codegen_bins=(--bin app_gen --bin appfw_introspect --bin appfw)
docs_check_export_app_gen=1
docs_check_snapshot_appgen_appfw=1
docs_check_prebuild_provider_export=0
if [[ "$focused_subcheck" == agent-governance-* ]]; then
  docs_check_codegen_bins=(--bin appfw_introspect)
  docs_check_export_app_gen=0
  docs_check_snapshot_appgen_appfw=0
  docs_check_prebuild_provider_export=1
elif [[ "$focused_subcheck" == "core-generated-drift-command-examples" ]]; then
  docs_check_codegen_bins=(--bin app_gen)
  docs_check_export_app_gen=1
  docs_check_snapshot_appgen_appfw=0
fi

if [[ "$json" != "1" ]]; then
  if [[ "$focused_subcheck" == agent-governance-* ]]; then
    printf 'Prebuilding docs-check binaries once (appfw_introspect and provider_certification_export for focused agent-governance examples)...\n'
  elif [[ "$focused_subcheck" == "core-generated-drift-command-examples" ]]; then
    printf 'Prebuilding docs-check binaries once (app_gen for focused generated-drift example)...\n'
  else
    printf 'Prebuilding docs-check binaries once (app_gen, appfw_introspect, app_gen appfw, appfw-cli appfw)...\n'
  fi
fi

# Both builds run at workspace scope (-p ...) so cargo resolves a single unified
# feature set for shared deps (appfw_runtime). Mixing a `--manifest-path` build
# with a workspace `-p` build makes cargo recompile appfw_runtime each time.
progress_event "start" "prebuild-appfw-codegen"
prebuild_start_ms="$(ms_now)"
if [[ "$docs_check_snapshot_appgen_appfw" == "1" ]]; then
  rm -f "$cargo_debug_dir/appfw"
fi
(cd "$repo_root" && cargo build --locked --quiet -p appfw-codegen "${docs_check_codegen_bins[@]}")
if [[ "$docs_check_snapshot_appgen_appfw" == "1" ]]; then
  cp "$cargo_debug_dir/appfw" "$docs_appfw_appgen"
fi
prebuild_elapsed_ms="$(( $(ms_now) - prebuild_start_ms ))"
prebuild_codegen_command="cargo build --locked --quiet -p appfw-codegen ${docs_check_codegen_bins[*]}"
record_docs_check_phase \
  "prebuild-appfw-codegen" \
  "$prebuild_elapsed_ms" \
  true \
  "$prebuild_codegen_command"
progress_event "finish" "prebuild-appfw-codegen" "$prebuild_elapsed_ms" true

if [[ -z "$focused_subcheck" || "$focused_subcheck" == "compiled-cli-command-examples" || "$focused_subcheck" == agent-governance-* ]]; then
  progress_event "start" "prebuild-appfw-cli"
  prebuild_start_ms="$(ms_now)"
  rm -f "$cargo_debug_dir/appfw"
  (cd "$repo_root" && cargo build --locked --quiet -p appfw-cli --bin appfw --bin provider_certification_export)
  cp "$cargo_debug_dir/appfw" "$docs_appfw_cli"
  prebuild_elapsed_ms="$(( $(ms_now) - prebuild_start_ms ))"
  record_docs_check_phase \
    "prebuild-appfw-cli" \
    "$prebuild_elapsed_ms" \
    true \
    "cargo build --locked --quiet -p appfw-cli --bin appfw --bin provider_certification_export"
  progress_event "finish" "prebuild-appfw-cli" "$prebuild_elapsed_ms" true
fi

if [[ "$docs_check_prebuild_provider_export" == "1" ]]; then
  progress_event "start" "prebuild-provider-certification-export"
  prebuild_start_ms="$(ms_now)"
  (cd "$repo_root" && cargo build --locked --quiet -p appfw-cli --bin provider_certification_export)
  prebuild_elapsed_ms="$(( $(ms_now) - prebuild_start_ms ))"
  record_docs_check_phase \
    "prebuild-provider-certification-export" \
    "$prebuild_elapsed_ms" \
    true \
    "cargo build --locked --quiet -p appfw-cli --bin provider_certification_export"
  progress_event "finish" "prebuild-provider-certification-export" "$prebuild_elapsed_ms" true
fi

# Route every `scripts/appfw <cmd>` example through the prebuilt introspect bin
# (see run_appfw_introspect in scripts/appfw), so none of them pay `cargo run`.
export APPFW_INTROSPECT_BIN="$cargo_debug_dir/appfw_introspect"
export APPFW_CLI_BIN="$docs_appfw_cli"
export APPFW_PROVIDER_CERTIFICATION_EXPORT_BIN="$cargo_debug_dir/provider_certification_export"
# Route validation/generation examples through the prebuilt generator binary.
# This avoids a second cold `cargo run` in `generate --check` split-root proof.
if [[ "$docs_check_export_app_gen" == "1" ]]; then
  export APPFW_APP_GEN_BIN="$cargo_debug_dir/app_gen"
else
  unset APPFW_APP_GEN_BIN
fi
# ----------------------------------------------------------------------------

if should_run_docs_check_subcheck "core-bootstrap-command-examples"; then
  core_bootstrap_examples_start_ms="$(ms_now)"
  core_bootstrap_examples_start_item_count="$(doc_example_item_count)"
  core_bootstrap_examples_start_failure_count="$(doc_example_failure_count)"
  run_example "doctor-json" "$repo_root/scripts/appfw" doctor --json
  run_example "context-json" "$repo_root/scripts/appfw" context --json
  run_log_assertion "context-recommended-namespace" "$report_dir/docs-example-context-json.log" '"recommended_namespace"' "context JSON exposes recommended namespace"
  run_example "lifecycle-json" "$repo_root/scripts/appfw" lifecycle --json
  run_log_assertion "lifecycle-product-namespace" "$report_dir/docs-example-lifecycle-json.log" '"canonical_namespaces": ["product", "framework"]' "lifecycle JSON exposes product/framework namespaces"
  run_log_assertion "lifecycle-product-phase" "$report_dir/docs-example-lifecycle-json.log" '"phase": "intake"' "lifecycle JSON exposes product phases"
  run_log_assertion "lifecycle-legacy-phase" "$report_dir/docs-example-lifecycle-json.log" '"phase": "legacy-modernization"' "lifecycle JSON exposes legacy modernization phase"
  run_log_assertion "lifecycle-framework-phase" "$report_dir/docs-example-lifecycle-json.log" '"phase": "docs-ia"' "lifecycle JSON exposes framework phases"
  run_log_assertion "lifecycle-package-phase" "$report_dir/docs-example-lifecycle-json.log" '"phase": "package-distribution"' "lifecycle JSON exposes package distribution phase"
  record_docs_check_subcheck_span \
    "core-bootstrap-command-examples" \
    "$core_bootstrap_examples_start_ms" \
    "$core_bootstrap_examples_start_item_count" \
    "$core_bootstrap_examples_start_failure_count" \
    "doctor, context, and lifecycle command examples"
fi

if should_run_docs_check_subcheck "core-generation-command-examples"; then
  core_generation_examples_start_ms="$(ms_now)"
  core_generation_examples_start_item_count="$(doc_example_item_count)"
  core_generation_examples_start_failure_count="$(doc_example_failure_count)"
  run_example "validate-json" "$repo_root/scripts/appfw" validate --json
  run_json_field_assertion "validate-semantic-ok" "$report_dir/docs-example-validate-json.log" "ok" "true" "validate JSON reports semantic success"
  run_json_field_assertion "validate-command-name" "$report_dir/docs-example-validate-json.log" "command" '"validate"' "validate JSON names its command"
  run_json_field_assertion "validate-scope" "$report_dir/docs-example-validate-json.log" "scope" '"product-config"' "flat validate names its actual product config scope"
  run_example "product-validate-json" "$repo_root/scripts/appfw" product validate --json
  run_json_field_assertion "product-validate-semantic-ok" "$report_dir/docs-example-product-validate-json.log" "ok" "true" "product validate JSON reports semantic success"
  run_json_field_assertion "product-validate-scope" "$report_dir/docs-example-product-validate-json.log" "scope" '"product-config"' "product validate names its actual scope"
  run_example "topology-json" "$repo_root/scripts/appfw" topology --json
  run_example "boundary-check-json" "$repo_root/scripts/appfw" boundary-check --json
  record_docs_check_subcheck_span \
    "core-generation-command-examples" \
    "$core_generation_examples_start_ms" \
    "$core_generation_examples_start_item_count" \
    "$core_generation_examples_start_failure_count" \
    "validation, topology, and boundary command examples"
fi

if should_run_docs_check_subcheck "core-generated-drift-command-examples"; then
  core_generated_drift_examples_start_ms="$(ms_now)"
  core_generated_drift_examples_start_item_count="$(doc_example_item_count)"
  core_generated_drift_examples_start_failure_count="$(doc_example_failure_count)"
  run_example "generate-check-json" "$repo_root/scripts/appfw" generate --check --json
  record_docs_check_subcheck_span \
    "core-generated-drift-command-examples" \
    "$core_generated_drift_examples_start_ms" \
    "$core_generated_drift_examples_start_item_count" \
    "$core_generated_drift_examples_start_failure_count" \
    "generated drift command example"
fi

if should_run_docs_check_subcheck "core-explain-profile-command-examples"; then
  core_explain_profile_examples_start_ms="$(ms_now)"
  core_explain_profile_examples_start_item_count="$(doc_example_item_count)"
  core_explain_profile_examples_start_failure_count="$(doc_example_failure_count)"
  run_example "explain-ownership-json" "$repo_root/scripts/appfw" explain ownership README.md --json
  run_example "explain-config-json" "$repo_root/scripts/appfw" explain config entity_type.indexes --json
  run_example "explain-provider-json" "$repo_root/scripts/appfw" explain provider pagination --provider postgres --json
  run_example "explain-provider-sdk-json" "$repo_root/scripts/appfw" explain provider-sdk --json
  run_example "new-list-profiles-json" "$repo_root/scripts/appfw" new --list-profiles --json
  run_log_assertion "new-list-profiles-profile-path" "$report_dir/docs-example-new-list-profiles-json.log" '"profile_path"' "new profile JSON exposes profile_path"
  run_log_assertion "new-list-profiles-verification" "$report_dir/docs-example-new-list-profiles-json.log" '"verification"' "new profile JSON exposes verification commands"
  run_log_assertion "new-list-profiles-generate-check" "$report_dir/docs-example-new-list-profiles-json.log" 'scripts/appfw product generate --check --json' "new profile verification includes product generate check"
  run_log_assertion "new-list-starter-modes" "$report_dir/docs-example-new-list-profiles-json.log" '"starter_modes"' "new profile JSON exposes built-in starter modes"
  run_log_assertion "new-list-product-intake-starter" "$report_dir/docs-example-new-list-profiles-json.log" '"name": "product-intake"' "new profile JSON exposes product-intake starter mode"
  run_log_assertion "new-list-product-intake-proof" "$report_dir/docs-example-new-list-profiles-json.log" 'scripts/appfw framework intake-proof --json' "product-intake starter mode records framework intake proof"
  record_docs_check_subcheck_span \
    "core-explain-profile-command-examples" \
    "$core_explain_profile_examples_start_ms" \
    "$core_explain_profile_examples_start_item_count" \
    "$core_explain_profile_examples_start_failure_count" \
    "ownership, config, provider, SDK, and profile command examples"
fi

if should_run_docs_check_subcheck "intake-skills-command-examples"; then
  intake_skills_examples_start_ms="$(ms_now)"
  intake_skills_examples_start_item_count="$(doc_example_item_count)"
  intake_skills_examples_start_failure_count="$(doc_example_failure_count)"
  run_example "intake-proof-json" "$repo_root/scripts/appfw" framework intake-proof --json
  run_log_assertion "intake-proof-command" "$report_dir/docs-example-intake-proof-json.log" '"command": "intake-proof"' "intake proof JSON exposes command"
  run_log_assertion "intake-proof-artifact" "$report_dir/docs-example-intake-proof-json.log" '"artifact": "target/appfw/product-intake-proof.json"' "intake proof JSON records retained artifact"
  run_log_assertion "intake-proof-proposal" "$report_dir/docs-example-intake-proof-json.log" 'target/appfw/product-intake-proof/model-proposal.json' "intake proof JSON records proposal evidence"
  run_log_assertion "intake-proof-status" "$report_dir/docs-example-intake-proof-json.log" 'target/appfw/product-intake-proof/model-status.json' "intake proof JSON records model status evidence"
  run_log_assertion "intake-proof-frontend-residue" "$report_dir/docs-example-intake-proof-json.log" 'target/appfw/product-intake-proof/frontend-residue-check.json' "intake proof JSON records frontend residue evidence"
  run_log_assertion "intake-proof-frontend-scaffold" "$report_dir/docs-example-intake-proof-json.log" 'target/appfw/product-intake-proof/frontend-scaffold-check.json' "intake proof JSON records frontend scaffold package-check evidence"
  run_example "golden-downstream-json" "$repo_root/scripts/appfw" framework golden-downstream --json
  run_json_field_assertion "golden-downstream-semantic-ok" "$report_dir/docs-example-golden-downstream-json.log" "ok" "true" "golden downstream required profile semantics pass"
  run_json_field_assertion "golden-downstream-nonexecuting-mode" "$report_dir/docs-example-golden-downstream-json.log" "execution.full_disposable_ci_status" '"specified-not-run"' "docs-check golden command is explicitly non-executing"
  run_log_assertion "golden-downstream-command" "$report_dir/docs-example-golden-downstream-json.log" '"command": "golden-downstream"' "golden downstream JSON exposes command"
  run_log_assertion "golden-downstream-artifact" "$report_dir/docs-example-golden-downstream-json.log" '"artifact": "target/appfw/golden-downstream.json"' "golden downstream JSON records retained artifact"
  run_log_assertion "golden-downstream-disposable-lane" "$report_dir/docs-example-golden-downstream-json.log" '"disposable_ci_lane"' "golden downstream JSON records disposable CI lane"
  run_example "skills-json" "$repo_root/scripts/appfw" skills --json
  run_log_assertion "skills-command" "$report_dir/docs-example-skills-json.log" '"command": "skills"' "skills JSON exposes command"
  run_log_assertion "skills-audience" "$report_dir/docs-example-skills-json.log" '"audience": "product"' "skills JSON exposes audience"
  run_log_assertion "skills-phase" "$report_dir/docs-example-skills-json.log" '"phase": "bootstrap"' "skills JSON exposes phase"
  run_log_assertion "skills-cli-namespace" "$report_dir/docs-example-skills-json.log" '"cli_namespace": "product"' "skills JSON exposes CLI namespace"
  run_log_assertion "skills-product-bootstrap" "$report_dir/docs-example-skills-json.log" '"name": "product-bootstrap"' "skills JSON lists product-bootstrap"
  run_log_assertion "skills-product-pr-review" "$report_dir/docs-example-skills-json.log" '"name": "product-pr-review"' "skills JSON lists product-pr-review"
  run_log_assertion "skills-framework-docs" "$report_dir/docs-example-skills-json.log" '"name": "framework-docs-ia"' "skills JSON lists framework-docs-ia"
  run_log_assertion "skills-framework-pr-review" "$report_dir/docs-example-skills-json.log" '"name": "framework-pr-review"' "skills JSON lists framework-pr-review"
  run_log_assertion "skills-framework-research-steward" "$report_dir/docs-example-skills-json.log" '"name": "framework-research-steward"' "skills JSON lists framework-research-steward"
  run_log_assertion "skills-release-evidence" "$report_dir/docs-example-skills-json.log" '"name": "product-release-evidence"' "skills JSON lists product-release-evidence"
  run_log_assertion "skills-pds-sra-package" "$report_dir/docs-example-skills-json.log" '"name": "pds-sra-package"' "skills JSON lists pds-sra-package"
  run_example "product-skills-json" "$repo_root/scripts/appfw" product skills --json
  run_log_assertion "product-skills-filter" "$report_dir/docs-example-product-skills-json.log" '"audience": "product"' "product skills JSON filters product audience"
  run_log_assertion "product-skills-product-pr-review" "$report_dir/docs-example-product-skills-json.log" '"name": "product-pr-review"' "product skills JSON lists product-pr-review"
  run_log_assertion "product-skills-pds-sra-package" "$report_dir/docs-example-product-skills-json.log" '"name": "pds-sra-package"' "product skills JSON lists pds-sra-package"
  run_example "framework-skills-json" "$repo_root/scripts/appfw" framework skills --json
  run_log_assertion "framework-skills-filter" "$report_dir/docs-example-framework-skills-json.log" '"audience": "framework"' "framework skills JSON filters framework audience"
  run_log_assertion "framework-skills-research-steward" "$report_dir/docs-example-framework-skills-json.log" '"name": "framework-research-steward"' "framework skills JSON lists framework-research-steward"
  run_log_assertion "framework-skills-pds-sra-package" "$report_dir/docs-example-framework-skills-json.log" '"name": "pds-sra-package"' "framework skills JSON lists pds-sra-package"
  record_docs_check_subcheck_span \
    "intake-skills-command-examples" \
    "$intake_skills_examples_start_ms" \
    "$intake_skills_examples_start_item_count" \
    "$intake_skills_examples_start_failure_count" \
    "product intake, golden downstream, and skill routing command examples"
fi

if should_run_docs_check_subcheck "agent-governance-core-command-examples"; then
  agent_governance_core_examples_start_ms="$(ms_now)"
  agent_governance_core_examples_start_item_count="$(doc_example_item_count)"
  agent_governance_core_examples_start_failure_count="$(doc_example_failure_count)"
  run_example "cli-test-plan-json" "$repo_root/scripts/appfw" framework cli-test --plan --json
  run_json_field_assertion "cli-test-plan-semantic-ok" "$report_dir/docs-example-cli-test-plan-json.log" "ok" "true" "cli-test plan contract is internally valid"
  run_json_field_assertion "cli-test-plan-mode" "$report_dir/docs-example-cli-test-plan-json.log" "mode" '"plan"' "cli-test docs example is explicitly plan-only"
  run_log_assertion "cli-test-plan-command" "$report_dir/docs-example-cli-test-plan-json.log" '"command":"cli-test"' "cli-test plan JSON exposes command"
  run_log_assertion "cli-test-plan-scope" "$report_dir/docs-example-cli-test-plan-json.log" '"scope":"fast CLI contract"' "cli-test plan JSON exposes fast scope"
  run_log_assertion "cli-test-plan-shell-appfw" "$report_dir/docs-example-cli-test-plan-json.log" '"name":"shell-appfw"' "cli-test plan includes the appfw shell check"
  run_log_assertion "cli-test-plan-shell-doc-examples" "$report_dir/docs-example-cli-test-plan-json.log" '"name":"shell-doc-examples"' "cli-test plan includes the docs shell check"
  run_log_assertion "cli-test-plan-semantic-gates" "$report_dir/docs-example-cli-test-plan-json.log" '"name":"semantic-gates"' "cli-test plan includes semantic gates"
  run_log_assertion "cli-test-plan-delivery-mode-controller" "$report_dir/docs-example-cli-test-plan-json.log" '"name":"delivery-mode-controller"' "cli-test plan includes delivery mode controller proof"
  run_log_assertion "cli-test-plan-mobile-readiness-containment" "$report_dir/docs-example-cli-test-plan-json.log" '"name":"mobile-readiness-containment"' "cli-test plan includes fail-closed legacy mobile readiness proof"
  run_log_assertion "cli-test-plan-appfw-cli-unit" "$report_dir/docs-example-cli-test-plan-json.log" '"name":"appfw-cli-unit"' "cli-test plan includes appfw CLI unit tests"
  run_log_assertion "cli-test-plan-appfw-introspect-contract" "$report_dir/docs-example-cli-test-plan-json.log" '"name":"appfw-introspect-cli-contract"' "cli-test plan includes introspection CLI contract tests"
  run_example "delivery-mode-status-json" "$repo_root/scripts/appfw" mode status --json
  run_log_assertion "delivery-mode-status-profile" "$report_dir/docs-example-delivery-mode-status-json.log" '"delivery_profile"' "root delivery mode status exposes a profile"
  run_log_assertion "delivery-mode-status-routing" "$report_dir/docs-example-delivery-mode-status-json.log" '"routing"' "delivery mode status exposes routing"
  run_log_assertion "delivery-mode-status-source-status" "$report_dir/docs-example-delivery-mode-status-json.log" '"source_status"' "delivery mode status describes current, dirty, or stale source binding"
  run_log_assertion "delivery-mode-status-state-origin" "$report_dir/docs-example-delivery-mode-status-json.log" '"state"' "delivery mode status identifies tracked-default or worktree-local state"
  run_example "instruction-route-class-c-json" "$repo_root/scripts/appfw" framework instructions --task framework-docs-ia --role coding-agent --change-class C --json
  run_log_assertion "instruction-route-schema" "$report_dir/docs-example-instruction-route-class-c-json.log" '"schema":"appfw_instruction_route@1"' "instruction route exposes its versioned schema"
  run_log_assertion "instruction-route-role-card" "$report_dir/docs-example-instruction-route-class-c-json.log" '"producer_role_card":{"heading":"Coding Agent","path":"docs/start/agent-role-cards.md"' "instruction route selects one source-linked producer role card"
  run_log_assertion "instruction-route-skill" "$report_dir/docs-example-instruction-route-class-c-json.log" '"skill":{"heading":"Framework Docs IA","path":"agent_skills/framework-docs-ia/SKILL.md"' "instruction route selects one source-linked skill"
  run_log_assertion "instruction-route-review-depth" "$report_dir/docs-example-instruction-route-class-c-json.log" '"review_depth":"comprehensive","role":"framework-pr-review-agent"' "Class C instruction route selects comprehensive independent review"
  run_log_absence_assertion "instruction-route-no-timestamp" "$report_dir/docs-example-instruction-route-class-c-json.log" 'generated_at' "instruction route omits timestamps"
  run_log_absence_assertion "instruction-route-no-cwd" "$report_dir/docs-example-instruction-route-class-c-json.log" 'invocation_cwd' "instruction route omits invocation CWD"
  run_log_absence_assertion "instruction-route-no-framework-root" "$report_dir/docs-example-instruction-route-class-c-json.log" 'framework_root' "instruction route omits framework root"
  run_log_absence_assertion "instruction-route-no-app-root" "$report_dir/docs-example-instruction-route-class-c-json.log" 'app_root' "instruction route omits app root"
  run_example "test-smoke-plan-json" "$repo_root/scripts/appfw" framework test --smoke --plan --json
  run_log_assertion "test-smoke-plan-command" "$report_dir/docs-example-test-smoke-plan-json.log" '"command":"test"' "test smoke plan JSON exposes command"
  run_log_assertion "test-smoke-plan-mode" "$report_dir/docs-example-test-smoke-plan-json.log" '"mode":"smoke"' "test smoke plan JSON records smoke mode"
  run_log_assertion "test-smoke-plan-syntax-checks" "$report_dir/docs-example-test-smoke-plan-json.log" '"javascript checker syntax"' "test smoke plan JSON records checker syntax coverage"
  run_example "harness-check-json" "$repo_root/scripts/appfw" product harness-check --json
  run_log_assertion "harness-check-command" "$report_dir/docs-example-harness-check-json.log" '"command": "harness-check"' "harness-check JSON exposes command"
  run_log_assertion "harness-check-lane" "$report_dir/docs-example-harness-check-json.log" '"lane": "U2"' "harness-check JSON records U2 lane"
  run_log_assertion "harness-check-artifact" "$report_dir/docs-example-harness-check-json.log" '".appfw/target/appfw/harness-check.json"' "harness-check JSON records product report artifact"
  run_example "provider-graduation-json" "$repo_root/scripts/appfw" framework provider-graduation --json
  run_log_assertion "provider-graduation-command" "$report_dir/docs-example-provider-graduation-json.log" '"command":"provider-graduation"' "provider-graduation JSON exposes command"
  run_log_assertion "provider-graduation-lane" "$report_dir/docs-example-provider-graduation-json.log" '"lane":"U4"' "provider-graduation JSON records U4 lane"
  run_log_assertion "provider-graduation-summary" "$report_dir/docs-example-provider-graduation-json.log" '"promotion_violation_count":0' "provider-graduation JSON records promotion violations"
  run_example "framework-review-brief-focused-json" "$repo_root/scripts/appfw" framework review-brief --json
  run_log_assertion "framework-review-brief-focused-command" "$report_dir/docs-example-framework-review-brief-focused-json.log" '"command": "review-brief"' "framework review-brief JSON exposes command"
  run_log_assertion "framework-review-brief-focused-depth" "$report_dir/docs-example-framework-review-brief-focused-json.log" '"review_depth": "focused"' "framework review-brief defaults to focused mode"
  run_log_assertion "framework-review-brief-focused-slash" "$report_dir/docs-example-framework-review-brief-focused-json.log" '"slash_command": "/framework-pr-review"' "framework focused review-brief exposes base slash command"
  run_example "framework-review-brief-comprehensive-json" "$repo_root/scripts/appfw" framework review-brief --comprehensive --json
  run_log_assertion "framework-review-brief-comprehensive-depth" "$report_dir/docs-example-framework-review-brief-comprehensive-json.log" '"review_depth": "comprehensive"' "framework review-brief supports comprehensive mode"
  run_log_assertion "framework-review-brief-comprehensive-slash" "$report_dir/docs-example-framework-review-brief-comprehensive-json.log" '"slash_command": "/framework-pr-review --comprehensive"' "framework comprehensive review-brief exposes comprehensive slash command"
  run_log_assertion "framework-review-brief-comprehensive-instruction" "$report_dir/docs-example-framework-review-brief-comprehensive-json.log" 'Do not limit the review to the latest or current changes.' "framework comprehensive review-brief records depth instruction"
  run_example "framework-review-brief-auto-depth-json" "$repo_root/scripts/appfw" framework review-brief --auto-depth --json
  run_log_assertion "framework-review-brief-auto-requested" "$report_dir/docs-example-framework-review-brief-auto-depth-json.log" '"requested_review_depth": "auto"' "framework review-brief records auto-depth request"
  run_log_assertion "framework-review-brief-auto-required-depth" "$report_dir/docs-example-framework-review-brief-auto-depth-json.log" '"required_review_depth":' "framework review-brief records auto-depth required depth"
  run_log_assertion "framework-review-brief-auto-selected-depth" "$report_dir/docs-example-framework-review-brief-auto-depth-json.log" '"review_depth":' "framework review-brief records auto-depth selected depth"
  run_log_assertion "framework-review-brief-auto-slash" "$report_dir/docs-example-framework-review-brief-auto-depth-json.log" '"slash_command":' "framework review-brief records auto-depth slash command"
  run_log_assertion "framework-review-brief-comprehensive-status-values" "$report_dir/docs-example-framework-review-brief-comprehensive-json.log" 'GO WITH CONDITIONS' "framework review-brief records recommendation status values"
  run_log_assertion "framework-review-brief-comprehensive-severity-buckets" "$report_dir/docs-example-framework-review-brief-comprehensive-json.log" '"should_address"' "framework review-brief records severity count buckets"
  run_log_assertion "framework-review-brief-comprehensive-unsatisfied" "$report_dir/docs-example-framework-review-brief-comprehensive-json.log" '"satisfied": false' "framework review-brief stays unsatisfied before review output and approval"
  run_example_with_allowed_statuses "1" "framework-change-impact-invalid-base-json" "$repo_root/scripts/appfw" framework change-impact --base refs/heads/appfw-docs-check-missing-base --head HEAD --json
  run_log_assertion "framework-change-impact-invalid-base-fails-closed" "$report_dir/docs-example-framework-change-impact-invalid-base-json.log" '"ok": false' "framework change-impact fails closed for invalid base refs"
  run_log_assertion "framework-change-impact-invalid-base-error-count" "$report_dir/docs-example-framework-change-impact-invalid-base-json.log" '"git_error_count": 1' "framework change-impact records git error count for invalid base refs"
  run_log_assertion "framework-change-impact-invalid-base-phase" "$report_dir/docs-example-framework-change-impact-invalid-base-json.log" '"phase": "base-ref"' "framework change-impact records invalid base ref phase"
  change_impact_base_ref="$(docs_check_existing_change_impact_base_ref)"
  run_example "framework-change-impact-json" "$repo_root/scripts/appfw" framework change-impact --base "$change_impact_base_ref" --head HEAD --json
  run_change_impact_restored_assertion "framework-change-impact-restored-contract" "$report_dir/docs-example-framework-change-impact-json.log" "framework change-impact restores Git-valid retained evidence after invalid-ref fixture without requiring its own candidate gate to have already completed"
  run_example "framework-review-handoff-json" "$repo_root/scripts/appfw" framework handoff --json
  framework_blank_review_output="$report_dir/docs-example-framework-pr-review-blank-output.md"
  : >"$framework_blank_review_output"
  run_example "framework-review-brief-blank-output-json" "$repo_root/scripts/appfw" framework review-brief --comprehensive --review-output "$framework_blank_review_output" --approver docs-check --json
  run_log_assertion "framework-review-brief-blank-output-sections" "$report_dir/docs-example-framework-review-brief-blank-output-json.log" '"review_output_sections_satisfied": false' "framework review-brief rejects blank review output"
  run_log_assertion "framework-review-brief-blank-output-unsatisfied" "$report_dir/docs-example-framework-review-brief-blank-output-json.log" '"satisfied": false' "framework review-brief remains unsatisfied with blank review output"
  framework_headings_only_review_output="$report_dir/docs-example-framework-pr-review-headings-only-output.md"
  cat >"$framework_headings_only_review_output" <<'EOF'
Recommendation Summary
Findings
Strategic Significance
Role Adherence Assessment
Alignment Drift Assessment
Independent Code Quality And Architecture Assessment
Shared Reviewer Judgment
Human Approval Brief
Evidence Checked
Open Questions
EOF
  run_example "framework-review-brief-headings-only-output-json" "$repo_root/scripts/appfw" framework review-brief --comprehensive --review-output "$framework_headings_only_review_output" --approver docs-check --json
  run_log_assertion "framework-review-brief-headings-only-output-sections" "$report_dir/docs-example-framework-review-brief-headings-only-output-json.log" '"review_output_sections_satisfied": false' "framework review-brief rejects headings-only review output"
  run_log_assertion "framework-review-brief-headings-only-output-empty" "$report_dir/docs-example-framework-review-brief-headings-only-output-json.log" '"empty_review_sections": [' "framework review-brief records empty review sections"
  framework_bad_recommendation_review_output="$report_dir/docs-example-framework-pr-review-bad-recommendation-output.md"
  cat >"$framework_bad_recommendation_review_output" <<'EOF'
## Recommendation Summary
This review has a body but no explicit final status or numeric severity counts.

## Findings
No actionable findings in this intentionally incomplete fixture.

## Strategic Significance
This fixture proves the parser rejects incomplete recommendation summaries.

## Role Adherence Assessment
The fixture states the producing role remains inside the Framework PR Review Agent card.

## Alignment Drift Assessment
The sections are present, but the recommendation contract is incomplete.

## Independent Code Quality And Architecture Assessment
The content is non-empty so heading-only checks are not enough.

## Shared Reviewer Judgment
This fixture should not satisfy pre-push review evidence.

## Human Approval Brief
The human lacks a clear go/no-go decision and severity count summary.

## Evidence Checked
docs-check generated this negative fixture.

## Open Questions
No blocking questions for this fixture.
EOF
  run_example "framework-review-brief-bad-recommendation-output-json" "$repo_root/scripts/appfw" framework review-brief --comprehensive --review-output "$framework_bad_recommendation_review_output" --approver docs-check --json
  run_log_assertion "framework-review-brief-bad-recommendation-output-sections" "$report_dir/docs-example-framework-review-brief-bad-recommendation-output-json.log" '"review_output_sections_satisfied": false' "framework review-brief rejects incomplete recommendation summary"
  run_log_assertion "framework-review-brief-bad-recommendation-output-summary" "$report_dir/docs-example-framework-review-brief-bad-recommendation-output-json.log" '"recommendation_summary_satisfied": false' "framework review-brief records incomplete recommendation summary"
  framework_uncaptured_conditions_review_output="$report_dir/docs-example-framework-pr-review-uncaptured-conditions-output.md"
  cat >"$framework_uncaptured_conditions_review_output" <<'EOF'
## Recommendation Summary
Final status: GO WITH CONDITIONS.
Severity counts: blockers=0, critical=0, important=0, should_address=0, nice_to_address=0.
Recommended decision: proceed only after the named condition is captured.

## Attention Items
- Condition: the named condition must be captured before standing approval applies.

## Findings
No actionable findings in this docs-check fixture.

## Strategic Significance
This fixture proves conditional approval cannot be treated as standing approval without capture.

## Role Adherence Assessment
The fixture states the producing role remains inside the Framework PR Review Agent card.

## Alignment Drift Assessment
The review sections are present, but the condition-capture contract is incomplete.

## Independent Code Quality And Architecture Assessment
The fixture is intentionally small and exercises the retained review contract.

## Shared Reviewer Judgment
I would not approve standing push from this fixture alone.

## Human Approval Brief
The human should require an explicit captured condition.

## Evidence Checked
docs-check generated this negative review output.

## Open Questions
No blocking questions for this fixture.
EOF
  run_example "framework-review-brief-uncaptured-conditions-output-json" "$repo_root/scripts/appfw" framework review-brief --comprehensive --review-output "$framework_uncaptured_conditions_review_output" --approver docs-check --json
  run_log_assertion "framework-review-brief-uncaptured-conditions-output-sections" "$report_dir/docs-example-framework-review-brief-uncaptured-conditions-output-json.log" '"review_output_sections_satisfied": false' "framework review-brief rejects uncaptured GO WITH CONDITIONS"
  run_log_assertion "framework-review-brief-uncaptured-conditions-output-summary" "$report_dir/docs-example-framework-review-brief-uncaptured-conditions-output-json.log" '"conditions_capture_satisfied": false' "framework review-brief records uncaptured conditions"
  framework_missing_attention_review_output="$report_dir/docs-example-framework-pr-review-missing-attention-output.md"
  cat >"$framework_missing_attention_review_output" <<'EOF'
## Recommendation Summary
Final status: GO WITH CONDITIONS.
Severity counts: blockers=0, critical=0, important=0, should_address=0, nice_to_address=0.
Conditions captured: yes.
Recommended decision: proceed only after the human can see the named condition.

## Findings
No actionable findings in this docs-check fixture.

## Strategic Significance
This fixture proves non-GO reviews must surface conditions where the guard can print them.

## Role Adherence Assessment
The fixture states the producing role remains inside the Framework PR Review Agent card.

## Alignment Drift Assessment
The review sections are present, but the human attention list is missing.

## Independent Code Quality And Architecture Assessment
The fixture is intentionally small and exercises the retained review contract.

## Shared Reviewer Judgment
I would not approve standing push from this fixture alone.

## Human Approval Brief
The human should require attention items before relying on the guard summary.

## Evidence Checked
docs-check generated this negative review output.

## Open Questions
No blocking questions for this fixture.
EOF
  run_example "framework-review-brief-missing-attention-output-json" "$repo_root/scripts/appfw" framework review-brief --comprehensive --review-output "$framework_missing_attention_review_output" --approver docs-check --json
  run_log_assertion "framework-review-brief-missing-attention-output-sections" "$report_dir/docs-example-framework-review-brief-missing-attention-output-json.log" '"review_output_sections_satisfied": false' "framework review-brief rejects non-GO output without attention items"
  run_log_assertion "framework-review-brief-missing-attention-output-attention" "$report_dir/docs-example-framework-review-brief-missing-attention-output-json.log" '"attention_items_satisfied": false' "framework review-brief records missing attention items"
  framework_review_output="$report_dir/docs-example-framework-pr-review-output.md"
  cat >"$framework_review_output" <<'EOF'
## Recommendation Summary
Final status: GO WITH CONDITIONS.
Severity counts: blockers=0, critical=0, important=0, should_address=0, nice_to_address=0.
Conditions captured: yes.
Recommended decision: proceed only after the human confirms the retained evidence matches the branch state.

## Attention Items
- Condition: confirm the retained evidence matches the branch state before push.

## Findings
No actionable findings in this docs-check fixture.

## Strategic Significance
The review fixture demonstrates the required decision summary and evidence shape.

## Role Adherence Assessment
The fixture states the producing role remains inside the Framework PR Review Agent card.

## Alignment Drift Assessment
The review sections, CLI contract, skill guidance, and docs-check assertions agree.

## Independent Code Quality And Architecture Assessment
The fixture is intentionally small and exercises the retained review contract.

## Shared Reviewer Judgment
I would approve this fixture as test evidence for the review-brief parser.

## Human Approval Brief
The human should confirm status, severity counts, and residual risk are present.

## Evidence Checked
docs-check generated this review output and passed it into review-brief.

## Open Questions
No blocking questions for this fixture.
EOF
  python3 - "$repo_root/target/appfw/agent-handoff.json" <<'PY'
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
data = json.loads(path.read_text(encoding="utf-8"))
data.setdefault("git", {})["sha"] = "stale-docs-check-sha"
path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
  run_example "framework-review-brief-stale-handoff-json" "$repo_root/scripts/appfw" framework review-brief --comprehensive --review-output "$framework_review_output" --approver docs-check --json
  run_log_assertion "framework-review-brief-stale-handoff-current" "$report_dir/docs-example-framework-review-brief-stale-handoff-json.log" '"handoff_current": false' "framework review-brief detects stale handoff"
  run_log_assertion "framework-review-brief-stale-handoff-unsatisfied" "$report_dir/docs-example-framework-review-brief-stale-handoff-json.log" '"satisfied": false' "framework review-brief rejects stale handoff"
  run_example "framework-review-handoff-after-stale-json" "$repo_root/scripts/appfw" framework handoff --json
  python3 - "$repo_root/target/appfw/agent-handoff.json" "$framework_review_output" <<'PY'
import datetime
import json
import os
import sys
from pathlib import Path

handoff_path = Path(sys.argv[1])
review_output_path = Path(sys.argv[2])
handoff = json.loads(handoff_path.read_text(encoding="utf-8"))
generated_at = datetime.datetime.fromisoformat(
    handoff["generated_at"].replace("Z", "+00:00")
)
fixture_timestamp = (generated_at + datetime.timedelta(seconds=1)).timestamp()
os.utime(review_output_path, (fixture_timestamp, fixture_timestamp))
PY
  run_example "framework-review-brief-approved-json" "$repo_root/scripts/appfw" framework review-brief --comprehensive --review-output "$framework_review_output" --approver docs-check --json
  run_log_assertion "framework-review-brief-approved-output" "$report_dir/docs-example-framework-review-brief-approved-json.log" '"review_output_present": true' "framework approved review-brief records review output"
  run_log_assertion "framework-review-brief-approved-sections" "$report_dir/docs-example-framework-review-brief-approved-json.log" '"review_output_sections_satisfied": true' "framework approved review-brief validates review sections"
  run_log_assertion "framework-review-brief-approved-empty-sections" "$report_dir/docs-example-framework-review-brief-approved-json.log" '"empty_review_sections": []' "framework approved review-brief validates non-empty review sections"
  run_log_assertion "framework-review-brief-approved-recommendation-summary" "$report_dir/docs-example-framework-review-brief-approved-json.log" '"recommendation_summary_satisfied": true' "framework approved review-brief validates recommendation summary"
  run_log_assertion "framework-review-brief-approved-recommendation-status" "$report_dir/docs-example-framework-review-brief-approved-json.log" '"status": "GO WITH CONDITIONS"' "framework approved review-brief records recommendation status"
  run_log_assertion "framework-review-brief-approved-severity-buckets" "$report_dir/docs-example-framework-review-brief-approved-json.log" '"missing_severity_count_buckets": []' "framework approved review-brief validates severity buckets"
  run_log_assertion "framework-review-brief-approved-conditions-captured" "$report_dir/docs-example-framework-review-brief-approved-json.log" '"conditions_captured": true' "framework approved review-brief validates captured conditions"
  run_log_assertion "framework-review-brief-approved-attention-items" "$report_dir/docs-example-framework-review-brief-approved-json.log" '"attention_items_satisfied": true' "framework approved review-brief validates attention items"
  run_log_assertion "framework-review-brief-approved-handoff" "$report_dir/docs-example-framework-review-brief-approved-json.log" '"handoff_current": true' "framework approved review-brief records current handoff"
  run_log_assertion "framework-review-brief-approved-approver" "$report_dir/docs-example-framework-review-brief-approved-json.log" '"human_approval_recorded": true' "framework approved review-brief records human approval"
  run_log_assertion "framework-review-brief-approved-satisfied" "$report_dir/docs-example-framework-review-brief-approved-json.log" '"satisfied": true' "framework approved review-brief can satisfy pre-push status"
  run_example "product-review-brief-focused-json" "$repo_root/scripts/appfw" product review-brief --json
  run_log_assertion "product-review-brief-focused-depth" "$report_dir/docs-example-product-review-brief-focused-json.log" '"review_depth": "focused"' "product review-brief defaults to focused mode"
  run_log_assertion "product-review-brief-focused-slash" "$report_dir/docs-example-product-review-brief-focused-json.log" '"slash_command": "/product-pr-review"' "product focused review-brief exposes base slash command"
  run_example "product-review-brief-comprehensive-json" "$repo_root/scripts/appfw" product review-brief --comprehensive --json
  run_log_assertion "product-review-brief-comprehensive-depth" "$report_dir/docs-example-product-review-brief-comprehensive-json.log" '"review_depth": "comprehensive"' "product review-brief supports comprehensive mode"
  run_log_assertion "product-review-brief-comprehensive-slash" "$report_dir/docs-example-product-review-brief-comprehensive-json.log" '"slash_command": "/product-pr-review --comprehensive"' "product comprehensive review-brief exposes comprehensive slash command"
  run_example "product-review-brief-auto-depth-json" "$repo_root/scripts/appfw" product review-brief --auto-depth --json
  run_log_assertion "product-review-brief-auto-requested" "$report_dir/docs-example-product-review-brief-auto-depth-json.log" '"requested_review_depth": "auto"' "product review-brief records auto-depth request"
  run_log_assertion "product-review-brief-auto-required-depth" "$report_dir/docs-example-product-review-brief-auto-depth-json.log" '"required_review_depth":' "product review-brief records auto-depth required depth"
  run_log_assertion "product-review-brief-auto-selected-depth" "$report_dir/docs-example-product-review-brief-auto-depth-json.log" '"review_depth":' "product review-brief records auto-depth selected depth"
  run_log_assertion "product-review-brief-auto-slash" "$report_dir/docs-example-product-review-brief-auto-depth-json.log" '"slash_command":' "product review-brief records auto-depth slash command"
  run_log_assertion "product-review-brief-handoff-normalized-paths" "$report_dir/docs-example-product-review-brief-comprehensive-json.log" '"changed_files_for_handoff": [' "product review-brief exposes handoff-normalized changed paths"
  run_log_assertion "product-review-brief-uses-normalized-handoff-surface-comparison" "$repo_root/scripts/appfw" 'surface_paths(handoff_surface_signatures) == changed_files_for_handoff' "product review-brief compares handoff surfaces against normalized paths"
  run_log_assertion "product-review-brief-uses-normalized-handoff-freshness-check" "$repo_root/scripts/appfw" 'changed_paths_newer_than(changed_files_for_handoff, handoff.get("generated_at"))' "product review-brief freshness check uses normalized paths"
  run_example "framework-review-performance-comprehensive-json" "$repo_root/scripts/appfw" framework review-performance --comprehensive --json
  run_log_assertion "framework-review-performance-comprehensive-command" "$report_dir/docs-example-framework-review-performance-comprehensive-json.log" '"command": "review-performance"' "framework review-performance JSON exposes command"
  run_log_assertion "framework-review-performance-comprehensive-depth" "$report_dir/docs-example-framework-review-performance-comprehensive-json.log" '"review_depth": "comprehensive"' "framework review-performance supports comprehensive mode"
  run_example "framework-review-performance-auto-depth-json" "$repo_root/scripts/appfw" framework review-performance --auto-depth --json
  run_log_assertion "framework-review-performance-auto-command" "$report_dir/docs-example-framework-review-performance-auto-depth-json.log" '"command": "review-performance"' "framework review-performance auto-depth JSON exposes command"
  run_log_assertion "framework-review-performance-auto-requested" "$report_dir/docs-example-framework-review-performance-auto-depth-json.log" '"requested_review_depth": "auto"' "framework review-performance records auto-depth request"
  run_log_assertion "framework-review-performance-auto-depth" "$report_dir/docs-example-framework-review-performance-auto-depth-json.log" '"review_depth":' "framework review-performance records auto-depth selected depth"
  run_example "product-review-performance-auto-depth-json" "$repo_root/scripts/appfw" product review-performance --auto-depth --json
  run_log_assertion "product-review-performance-auto-command" "$report_dir/docs-example-product-review-performance-auto-depth-json.log" '"command": "review-performance"' "product review-performance auto-depth JSON exposes command"
  run_log_assertion "product-review-performance-auto-requested" "$report_dir/docs-example-product-review-performance-auto-depth-json.log" '"requested_review_depth": "auto"' "product review-performance records auto-depth request"
  run_log_assertion "product-review-performance-auto-depth" "$report_dir/docs-example-product-review-performance-auto-depth-json.log" '"review_depth":' "product review-performance records auto-depth selected depth"
  record_docs_check_subcheck_span \
    "agent-governance-core-command-examples" \
    "$agent_governance_core_examples_start_ms" \
    "$agent_governance_core_examples_start_item_count" \
    "$agent_governance_core_examples_start_failure_count" \
    "agent CLI, smoke, harness, and connector graduation command examples"
fi

if should_run_docs_check_subcheck "agent-governance-chat-command-examples"; then
  agent_governance_chat_examples_start_ms="$(ms_now)"
  agent_governance_chat_examples_start_item_count="$(doc_example_item_count)"
  agent_governance_chat_examples_start_failure_count="$(doc_example_failure_count)"
  run_example "chat-eval-json" "$repo_root/scripts/appfw" product chat-eval --json
  run_log_assertion "chat-eval-command" "$report_dir/docs-example-chat-eval-json.log" '"command": "chat-eval"' "chat-eval JSON exposes command"
  run_log_assertion "chat-eval-ok" "$report_dir/docs-example-chat-eval-json.log" '"ok": true' "chat-eval JSON reports ok"
  run_log_assertion "chat-eval-release-ready-false" "$report_dir/docs-example-chat-eval-json.log" '"release_ready": false' "chat-eval local fixture cannot claim release readiness"
  run_log_assertion "chat-eval-mode-local-fixture" "$report_dir/docs-example-chat-eval-json.log" '"mode": "local-fixture"' "chat-eval JSON records local-fixture mode"
  run_log_assertion "chat-eval-red-team" "$report_dir/docs-example-chat-eval-json.log" '"leaks_found": 0' "chat-eval red-team records zero leaks"
  run_example "chat-eval-pipeline-plan-json" "$repo_root/scripts/appfw" product chat-eval --json --pipeline-plan
  run_log_assertion "chat-eval-pipeline-plan-pointer" "$report_dir/docs-example-chat-eval-pipeline-plan-json.log" '"pipeline": {' "chat-eval pipeline plan records pipeline pointer"
  run_log_assertion "chat-eval-pipeline-plan-artifact" "$report_dir/docs-example-chat-eval-pipeline-plan-json.log" 'chat-eval-promptfoo-plan.json' "chat-eval pipeline plan records retained promptfoo plan artifact"
  run_log_assertion "chat-eval-pipeline-plan-not-ready" "$report_dir/docs-example-chat-eval-pipeline-plan-json.log" '"release_ready": false' "chat-eval pipeline plan does not claim release readiness"
  run_example_with_allowed_statuses "1" "chat-eval-inject-leak-json" "$repo_root/scripts/appfw" product chat-eval --json --inject-leak
  run_log_assertion "chat-eval-inject-leak-finding" "$report_dir/docs-example-chat-eval-inject-leak-json.log" '"code": "policy_leak"' "chat-eval leak injection fails closed with policy_leak"
  run_log_assertion "chat-eval-inject-leak-red-team" "$report_dir/docs-example-chat-eval-inject-leak-json.log" '"ok": false' "chat-eval leak injection marks red-team not ok"
  record_docs_check_subcheck_span \
    "agent-governance-chat-command-examples" \
    "$agent_governance_chat_examples_start_ms" \
    "$agent_governance_chat_examples_start_item_count" \
    "$agent_governance_chat_examples_start_failure_count" \
    "chat-eval command, posture, and fail-closed red-team examples"
fi

if should_run_docs_check_subcheck "agent-governance-write-command-examples"; then
  agent_governance_write_examples_start_ms="$(ms_now)"
  agent_governance_write_examples_start_item_count="$(doc_example_item_count)"
  agent_governance_write_examples_start_failure_count="$(doc_example_failure_count)"
  run_example "governed-write-check-json" "$repo_root/scripts/appfw" framework governed-write-check --json --enforce
  run_log_assertion "governed-write-check-command" "$report_dir/docs-example-governed-write-check-json.log" '"command": "governed-write-check"' "governed-write-check JSON exposes command"
  run_log_assertion "governed-write-check-lane" "$report_dir/docs-example-governed-write-check-json.log" '"lane": "G1"' "governed-write-check JSON records G1 lane"
  run_log_assertion "governed-write-check-enforced" "$report_dir/docs-example-governed-write-check-json.log" '"enforced": true' "governed-write-check JSON records enforced posture"
  run_log_assertion "governed-write-check-no-certified" "$report_dir/docs-example-governed-write-check-json.log" '"certified_providers": []' "governed-write-check JSON records no certified providers"
  run_example "provider-test-governed-write-plan-json" "$repo_root/scripts/appfw" framework provider-test --provider servicenow --area governed-write --plan --json
  run_log_assertion "provider-test-governed-write-plan-command" "$report_dir/docs-example-provider-test-governed-write-plan-json.log" '"command": "provider-test"' "provider-test governed-write plan exposes command"
  run_log_assertion "provider-test-governed-write-plan-lane" "$report_dir/docs-example-provider-test-governed-write-plan-json.log" '"lane": "G1"' "provider-test governed-write plan records G1 lane"
  run_log_assertion "provider-test-governed-write-plan-area" "$report_dir/docs-example-provider-test-governed-write-plan-json.log" '"area": "governed_write"' "provider-test governed-write plan records governed-write area"
  run_log_assertion "provider-test-governed-write-plan-evidence" "$report_dir/docs-example-provider-test-governed-write-plan-json.log" '"target/appfw/governed-write-evidence.json"' "provider-test governed-write plan preserves live evidence artifact boundary"
  run_log_assertion "provider-test-governed-write-live-inputs" "$report_dir/docs-example-provider-test-governed-write-plan-json.log" '"required_live_inputs"' "provider-test governed-write plan records grouped live inputs"
  run_log_assertion "provider-test-governed-write-evidence-retention" "$report_dir/docs-example-provider-test-governed-write-plan-json.log" 'APPFW_SERVICENOW_GOVERNED_WRITE_EVIDENCE_FILE' "provider-test governed-write plan requires external live evidence file input"
  run_log_assertion "provider-test-governed-write-mutation-name" "$report_dir/docs-example-provider-test-governed-write-plan-json.log" 'APPFW_SERVICENOW_GOVERNED_WRITE_MUTATION_NAME' "provider-test governed-write plan requires a named mutation input"
  run_log_assertion "provider-test-governed-write-policy-scope" "$report_dir/docs-example-provider-test-governed-write-plan-json.log" 'APPFW_SERVICENOW_GOVERNED_WRITE_POLICY_SCOPE' "provider-test governed-write plan requires a policy scope input"
  run_log_assertion "provider-test-governed-write-evidence-template" "$report_dir/docs-example-provider-test-governed-write-plan-json.log" '"evidence_template"' "provider-test governed-write plan emits a live evidence template"
  run_example "provider-test-saas-read-plan-json" "$repo_root/scripts/appfw" framework provider-test --provider salesforce --area saas-read --plan --json
  run_log_assertion "provider-test-saas-read-plan-command" "$report_dir/docs-example-provider-test-saas-read-plan-json.log" '"command": "provider-test"' "provider-test SaaS read plan exposes command"
  run_log_assertion "provider-test-saas-read-plan-lane" "$report_dir/docs-example-provider-test-saas-read-plan-json.log" '"lane": "DP6"' "provider-test SaaS read plan records DP6 lane"
  run_log_assertion "provider-test-saas-read-plan-area" "$report_dir/docs-example-provider-test-saas-read-plan-json.log" '"area": "saas_read"' "provider-test SaaS read plan records saas-read area"
  run_log_assertion "provider-test-saas-read-plan-schema" "$report_dir/docs-example-provider-test-saas-read-plan-json.log" '"schema_version": "appfw.saas_read_provider_test.plan.v1"' "provider-test SaaS read plan records schema version"
  run_log_assertion "provider-test-saas-read-plan-release-ready" "$report_dir/docs-example-provider-test-saas-read-plan-json.log" '"release_ready": false' "provider-test SaaS read plan stays non-release-ready"
  run_log_assertion "provider-test-saas-read-plan-evidence" "$report_dir/docs-example-provider-test-saas-read-plan-json.log" '"target/appfw/saas-read-evidence.json"' "provider-test SaaS read plan preserves live evidence artifact boundary"
  run_log_assertion "provider-test-saas-read-plan-omits-write" "$report_dir/docs-example-provider-test-saas-read-plan-json.log" '"omitted_write_areas"' "provider-test SaaS read plan separates write areas"
  run_log_assertion "provider-test-saas-read-plan-required-check" "$report_dir/docs-example-provider-test-saas-read-plan-json.log" '"query_metrics_and_audit"' "provider-test SaaS read plan includes query metrics/audit check"
  governed_write_isolated_report_dir="$report_dir/docs-example-governed-write-isolated"
  governed_write_isolated_evidence="$report_dir/docs-example-governed-write-live-evidence.json"
  rm -rf "$governed_write_isolated_report_dir"
  mkdir -p "$governed_write_isolated_report_dir"
  python3 - "$governed_write_isolated_evidence" <<'PY'
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
path.write_text(
    json.dumps(
        {
            "audit": {
                "correlation_id": "docs-example-correlation",
                "provider_request_id": "docs-example-redacted-request",
                "sink": "docs-example-audit-sink",
                "source": "http",
            },
            "command": "provider-test",
            "delegated_actor_context": {
                "on_behalf_of": "docs-example-user",
                "principal_type": "user",
                "tenant": "docs-example-tenant",
            },
            "idempotency": {
                "key_source": "docs-example-idempotency-key",
                "replay_rejected": True,
            },
            "lane": "G1",
            "live_provider_call": {
                "executed": True,
                "mutation_name": "servicenow.create_incident",
                "non_production": True,
                "replay_attempted": True,
                "result": "success",
            },
            "mutation_registry": {
                "mcp_enabled": False,
                "name": "servicenow.create_incident",
                "policy_scope": "servicenow.incident.write",
            },
            "ok": True,
            "operation": "named_mutation",
            "policy": {
                "decision": "allow",
                "scope_enforced": True,
            },
            "provider": "servicenow",
            "redaction": {
                "raw_payloads_removed": True,
                "secrets_removed": True,
                "tenant_data_removed": True,
            },
            "release_ready": True,
            "request_binding": {
                "provider_owned_request": True,
                "raw_url_or_query_from_caller": False,
            },
            "token_store_isolation": {
                "partition_key": ["user", "tenant", "provider"],
                "revocation_checked": True,
            },
        },
        indent=2,
        sort_keys=True,
    )
    + "\n",
    encoding="utf-8",
)
PY
  run_example "provider-test-governed-write-live-validation-json" \
    env \
      APPFW_PROVIDER_TEST_REPORT_DIR="$governed_write_isolated_report_dir" \
      APPFW_SERVICENOW_GOVERNED_WRITE_BASE_URL="https://docs-example.service-now.test" \
      APPFW_SERVICENOW_GOVERNED_WRITE_AUTH_MODE="oauth_client_credentials" \
      APPFW_SERVICENOW_GOVERNED_WRITE_TOKEN_REF="docs-example-token-ref" \
      APPFW_SERVICENOW_GOVERNED_WRITE_TEST_TENANT="docs-example-tenant" \
      APPFW_SERVICENOW_GOVERNED_WRITE_TEST_USER="docs-example-user" \
      APPFW_SERVICENOW_GOVERNED_WRITE_MUTATION_NAME="servicenow.create_incident" \
      APPFW_SERVICENOW_GOVERNED_WRITE_POLICY_SCOPE="servicenow.incident.write" \
      APPFW_SERVICENOW_GOVERNED_WRITE_TEST_IDEMPOTENCY_KEY="docs-example-idempotency-key" \
      APPFW_SERVICENOW_GOVERNED_WRITE_AUDIT_SINK="docs-example-audit-sink" \
      APPFW_SERVICENOW_GOVERNED_WRITE_TEST_INGRESS="http" \
      APPFW_SERVICENOW_GOVERNED_WRITE_EVIDENCE_FILE="$governed_write_isolated_evidence" \
      "$repo_root/scripts/appfw" framework provider-test --provider servicenow --area governed-write --json
  run_log_assertion "provider-test-governed-write-live-validation-mode" "$report_dir/docs-example-provider-test-governed-write-live-validation-json.log" '"mode": "live-evidence-validation"' "provider-test governed-write live validation records live-evidence mode"
  run_log_assertion "provider-test-governed-write-live-validation-release-ready" "$report_dir/docs-example-provider-test-governed-write-live-validation-json.log" '"release_ready": true' "provider-test governed-write live validation can retain valid isolated evidence"
  run_log_assertion "provider-test-governed-write-live-validation-retained" "$report_dir/docs-example-provider-test-governed-write-live-validation-json.log" 'docs-example-governed-write-isolated/governed-write-evidence.json' "provider-test governed-write live validation retains evidence in isolated report dir"
  record_docs_check_subcheck_span \
    "agent-governance-write-command-examples" \
    "$agent_governance_write_examples_start_ms" \
    "$agent_governance_write_examples_start_item_count" \
    "$agent_governance_write_examples_start_failure_count" \
    "governed write fail-closed and provider-test planning command examples"
fi

if should_run_docs_check_subcheck "agent-governance-mobile-command-examples"; then
  agent_governance_mobile_examples_start_ms="$(ms_now)"
  agent_governance_mobile_examples_start_item_count="$(doc_example_item_count)"
  agent_governance_mobile_examples_start_failure_count="$(doc_example_failure_count)"
  run_example_with_allowed_statuses "0 1" "mobile-test-json" "$repo_root/scripts/appfw" product mobile-test --json
  run_log_assertion "mobile-test-command" "$report_dir/docs-example-mobile-test-json.log" '"command": "mobile-test"' "mobile-test JSON exposes command"
  run_log_assertion "mobile-test-lane" "$report_dir/docs-example-mobile-test-json.log" '"lane": "U5"' "mobile-test JSON records U5 lane"
  run_log_assertion "mobile-test-artifact" "$report_dir/docs-example-mobile-test-json.log" '".appfw/target/appfw/mobile-test.json"' "mobile-test JSON records product report artifact"
  run_log_assertion "mobile-test-static-readiness" "$report_dir/docs-example-mobile-test-json.log" '"mode": "static-readiness"' "mobile-test JSON records static readiness mode"
  run_log_assertion "mobile-test-readiness-level" "$report_dir/docs-example-mobile-test-json.log" '"readiness_level": "static-scaffold"' "mobile-test JSON distinguishes static scaffold readiness"
  run_json_field_assertion "mobile-test-candidate-ready-contained" "$report_dir/docs-example-mobile-test-json.log" "candidate_ready" "false" "legacy mobile-test cannot claim candidate readiness"
  run_json_field_assertion "mobile-test-release-ready-contained" "$report_dir/docs-example-mobile-test-json.log" "release_ready" "false" "legacy mobile-test cannot claim release readiness"
  run_json_field_assertion "mobile-test-release-authority-none" "$report_dir/docs-example-mobile-test-json.log" "release_authority" '"none"' "legacy mobile-test declares no release authority"
  run_json_field_assertion "mobile-test-readiness-non-authoritative" "$report_dir/docs-example-mobile-test-json.log" "readiness_authority.authoritative" "false" "legacy mobile-test is machine-normatively non-authoritative"
  run_json_field_assertion "mobile-test-containment-status" "$report_dir/docs-example-mobile-test-json.log" "readiness_authority.status" '"contained-non-authoritative"' "legacy mobile-test records stable containment status"
  run_log_assertion "mobile-test-audit-disposition-input" "$report_dir/docs-example-mobile-test-json.log" '"npm_audit_disposition": "mobile/.appfw-mobile/npm-audit-disposition.json"' "mobile-test JSON records runtime audit disposition path"
  run_log_assertion "mobile-test-runtime-audit-report" "$report_dir/docs-example-mobile-test-json.log" '"runtime_audit": {' "mobile-test JSON reports runtime audit posture"
  run_log_assertion "mobile-test-runtime-audit-status" "$report_dir/docs-example-mobile-test-json.log" '"evidence_status": "failed"' "mobile-test JSON reports retained runtime audit evidence status"
  run_log_assertion "mobile-test-runtime-audit-remediation" "$report_dir/docs-example-mobile-test-json.log" '"remediation": {' "mobile-test JSON reports runtime audit remediation posture"
  run_log_assertion "mobile-test-runtime-audit-unavailable" "$report_dir/docs-example-mobile-test-json.log" '"audit_unavailable": {' "mobile-test JSON reports runtime audit unavailable posture when retained"
  run_log_assertion "mobile-test-runtime-audit-major-fix-count" "$report_dir/docs-example-mobile-test-json.log" '"semver_major_fix_count"' "mobile-test JSON reports semver-major remediation count"
  run_log_assertion "mobile-test-runtime-audit-disposition-required" "$report_dir/docs-example-mobile-test-json.log" '"disposition_required": true' "mobile-test JSON reports when runtime audit findings require disposition"
  run_log_assertion "mobile-test-command-evidence-report" "$report_dir/docs-example-mobile-test-json.log" '"command_evidence": {' "mobile-test JSON reports retained command evidence posture"
  run_log_assertion "mobile-test-real-test-evidence" "$report_dir/docs-example-mobile-test-json.log" '"test": {' "mobile-test JSON reports retained unit/integration test evidence"
  run_log_assertion "mobile-test-expo-doctor-network-not-release-grade" "$report_dir/docs-example-mobile-test-json.log" 'expo doctor evidence ignored network errors' "mobile-test JSON rejects network-bypassed Expo Doctor evidence for legacy diagnostic completeness"
  run_log_assertion "mobile-test-device-evidence-report" "$report_dir/docs-example-mobile-test-json.log" '"device_evidence": {' "mobile-test JSON reports device evidence posture"
  run_log_assertion "mobile-test-device-evidence-input" "$report_dir/docs-example-mobile-test-json.log" '"device_evidence": "mobile/.appfw-mobile/device-evidence.json"' "mobile-test JSON records device evidence path"
  run_log_assertion "mobile-test-device-evidence-placeholder" "$report_dir/docs-example-mobile-test-json.log" '"kind": "static-placeholder"' "mobile-test JSON distinguishes placeholder device evidence from release evidence"
  run_log_assertion "mobile-test-device-evidence-release-ready" "$report_dir/docs-example-mobile-test-json.log" '"release_ready": false' "mobile-test JSON does not claim device evidence release readiness from a placeholder"
  run_log_assertion "mobile-test-store-track-input" "$report_dir/docs-example-mobile-test-json.log" '"store_track_evidence": "mobile/.appfw-mobile/store-track-evidence.json"' "mobile-test JSON records store-track evidence path"
  run_log_assertion "mobile-test-store-track-report" "$report_dir/docs-example-mobile-test-json.log" '"store_track_evidence": {' "mobile-test JSON reports store-track evidence posture"
  run_log_assertion "mobile-test-store-track-placeholder" "$report_dir/docs-example-mobile-test-json.log" '"kind": "static-placeholder"' "mobile-test JSON distinguishes placeholder store-track evidence from release evidence"
  run_example "mobile-readiness-containment-regression" python3 "$repo_root/scripts/mobile-readiness-containment.test.py"
  run_log_assertion "mobile-readiness-containment-regression-ok" "$report_dir/docs-example-mobile-readiness-containment-regression.log" 'mobile readiness containment: OK' "legacy mobile readiness remains fail-closed at producer, Wave 2, and strict evidence consumers"

  mobile_unit_test_evidence="$repo_root/examples/products/crm/mobile/.appfw-mobile/test-evidence.json"
  mobile_unit_test_evidence_backup="$report_dir/docs-check-mobile-unit-test-evidence.backup.json"
  mobile_unit_test_evidence_was_present=0
  mobile_unit_test_evidence_restore_armed=1
  restore_docs_check_mobile_unit_test_evidence() {
    if [[ "${mobile_unit_test_evidence_restore_armed:-0}" != "1" ]]; then
      return 0
    fi
    if [[ "$mobile_unit_test_evidence_was_present" == "1" ]]; then
      mv "$mobile_unit_test_evidence_backup" "$mobile_unit_test_evidence"
    else
      rm -f "$mobile_unit_test_evidence"
      rm -f "$mobile_unit_test_evidence_backup"
    fi
    mobile_unit_test_evidence_restore_armed=0
  }
  trap restore_docs_check_mobile_unit_test_evidence EXIT
  if [[ -f "$mobile_unit_test_evidence" ]]; then
    cp "$mobile_unit_test_evidence" "$mobile_unit_test_evidence_backup"
    mobile_unit_test_evidence_was_present=1
  fi
  python3 - "$mobile_unit_test_evidence" <<'PY'
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
path.parent.mkdir(parents=True, exist_ok=True)
path.write_text(
    json.dumps(
        {
            "version": 1,
            "lane": "U5",
            "name": "test",
            "command": ["npm", "run", "test"],
            "cwd": "mobile",
            "generated_at": "2026-07-02T00:00:00Z",
            "exit_code": 0,
            "ok": True,
            "status": "passed",
            "stdout_tail": "No tests found, exiting with code 0",
            "stderr_tail": "",
        },
        indent=2,
        sort_keys=True,
    )
    + "\n",
    encoding="utf-8",
)
PY
  run_example_with_allowed_statuses "0 1" "mobile-test-zero-test-json" "$repo_root/scripts/appfw" product mobile-test --json
  run_log_assertion "mobile-test-zero-test-not-release-grade" "$report_dir/docs-example-mobile-test-zero-test-json.log" 'test evidence reports zero executed tests' "mobile-test JSON rejects zero-test evidence for legacy diagnostic completeness"
  restore_docs_check_mobile_unit_test_evidence
  trap - EXIT
  run_example_with_allowed_statuses "0 1" "mobile-test-json-after-zero-test" "$repo_root/scripts/appfw" product mobile-test --json
  run_log_assertion "mobile-test-after-zero-test-restored" "$report_dir/docs-example-mobile-test-json-after-zero-test.log" '"test": {' "mobile-test test evidence is restored after zero-test rejection"

  mobile_test_evidence="$repo_root/examples/products/crm/.appfw/target/appfw/mobile-test.json"
  mobile_test_evidence_backup="$report_dir/docs-check-mobile-test.backup.json"
  mobile_test_evidence_was_present=0
  mobile_test_evidence_restore_armed=1
  restore_docs_check_mobile_test_evidence() {
    if [[ "${mobile_test_evidence_restore_armed:-0}" != "1" ]]; then
      return 0
    fi
    if [[ "$mobile_test_evidence_was_present" == "1" ]]; then
      mv "$mobile_test_evidence_backup" "$mobile_test_evidence"
    else
      rm -f "$mobile_test_evidence"
      rm -f "$mobile_test_evidence_backup"
    fi
    mobile_test_evidence_restore_armed=0
  }
  trap restore_docs_check_mobile_test_evidence EXIT
  if [[ -f "$mobile_test_evidence" ]]; then
    cp "$mobile_test_evidence" "$mobile_test_evidence_backup"
    mobile_test_evidence_was_present=1
  fi
  python3 - "$mobile_test_evidence" <<'PY'
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
path.parent.mkdir(parents=True, exist_ok=True)
path.write_text(
    json.dumps(
        {
            "command": "mobile-test",
            "lane": "U5",
            "ok": True,
            "release_ready": True,
            "readiness_level": "static-scaffold",
        },
        indent=2,
    )
    + "\n",
    encoding="utf-8",
)
PY
  run_example "wave2-status-mobile-thin-release-json" "$repo_root/scripts/appfw" framework wave2-status --json
  run_log_assertion "wave2-status-mobile-thin-release-schema" "$report_dir/docs-example-wave2-status-mobile-thin-release-json.log" '"schema_ok": false' "wave2-status rejects schema-thin U5 mobile release evidence"
  run_log_assertion "wave2-status-mobile-thin-release-finding" "$report_dir/docs-example-wave2-status-mobile-thin-release-json.log" 'legacy mobile-test is non-authoritative and must keep release_ready false' "wave2-status reports why legacy U5 mobile release evidence is invalid"
  restore_docs_check_mobile_test_evidence
  trap - EXIT
  run_example "mobile-test-json-after-thin-release" "$repo_root/scripts/appfw" product mobile-test --json
  run_log_assertion "mobile-test-after-thin-release-ok" "$report_dir/docs-example-mobile-test-json-after-thin-release.log" '"ok": true' "mobile-test evidence is restored after schema-thin release rejection"

  mobile_device_tooling_evidence="$repo_root/examples/products/crm/mobile/.appfw-mobile/device-tooling-evidence.json"
  mobile_device_tooling_evidence_backup="$report_dir/docs-check-mobile-device-tooling-evidence.backup.json"
  mobile_device_tooling_evidence_was_present=0
  mobile_device_tooling_evidence_restore_armed=0
  restore_docs_check_mobile_device_tooling_evidence() {
    if [[ "${mobile_device_tooling_evidence_restore_armed:-0}" != "1" ]]; then
      if ! rm -f "$mobile_device_tooling_evidence_backup"; then
        echo "Failed to remove incomplete mobile device-tooling evidence backup" >&2
        return 1
      fi
      if [[ -e "$mobile_device_tooling_evidence_backup" || -L "$mobile_device_tooling_evidence_backup" ]]; then
        echo "Incomplete mobile device-tooling evidence backup remains after cleanup" >&2
        return 1
      fi
      return 0
    fi
    if [[ "$mobile_device_tooling_evidence_was_present" == "1" ]]; then
      if [[ ! -f "$mobile_device_tooling_evidence_backup" ]]; then
        echo "Mobile device-tooling evidence backup is missing during docs-check cleanup" >&2
        return 1
      fi
      if ! cp -p "$mobile_device_tooling_evidence_backup" "$mobile_device_tooling_evidence"; then
        echo "Failed to restore mobile device-tooling evidence after docs example" >&2
        return 1
      fi
      if ! cmp -s "$mobile_device_tooling_evidence_backup" "$mobile_device_tooling_evidence"; then
        echo "Restored mobile device-tooling evidence does not match its docs-check snapshot" >&2
        return 1
      fi
    else
      if ! rm -f "$mobile_device_tooling_evidence"; then
        echo "Failed to remove docs-example mobile device-tooling evidence" >&2
        return 1
      fi
      if [[ -e "$mobile_device_tooling_evidence" || -L "$mobile_device_tooling_evidence" ]]; then
        echo "Docs-example mobile device-tooling evidence remains after cleanup" >&2
        return 1
      fi
    fi
    if ! rm -f "$mobile_device_tooling_evidence_backup"; then
      echo "Failed to remove mobile device-tooling evidence backup" >&2
      return 1
    fi
    mobile_device_tooling_evidence_restore_armed=0
  }
  restore_docs_check_mobile_device_tooling_evidence_on_exit() {
    local command_status="$?"
    trap - EXIT
    if ! restore_docs_check_mobile_device_tooling_evidence; then
      exit 1
    fi
    exit "$command_status"
  }
  trap restore_docs_check_mobile_device_tooling_evidence_on_exit EXIT
  if [[ -e "$mobile_device_tooling_evidence" || -L "$mobile_device_tooling_evidence" ]]; then
    mobile_device_tooling_evidence_was_present=1
    cp -p "$mobile_device_tooling_evidence" "$mobile_device_tooling_evidence_backup"
  else
    rm -f "$mobile_device_tooling_evidence_backup"
  fi
  mobile_device_tooling_evidence_restore_armed=1
  run_example "mobile-test-device-preflight-json" "$repo_root/scripts/appfw" product mobile-test --device-preflight --json
  run_log_assertion "mobile-test-device-preflight-command" "$report_dir/docs-example-mobile-test-device-preflight-json.log" '"command": "mobile-test"' "mobile-test device preflight JSON exposes command"
  run_log_assertion "mobile-test-device-preflight-input" "$report_dir/docs-example-mobile-test-device-preflight-json.log" '"device_tooling_evidence": "mobile/.appfw-mobile/device-tooling-evidence.json"' "mobile-test device preflight records tooling evidence path"
  run_log_assertion "mobile-test-device-preflight-requested" "$report_dir/docs-example-mobile-test-device-preflight-json.log" '"device_preflight_requested": true' "mobile-test device preflight records requested mode"
  restore_docs_check_mobile_device_tooling_evidence
  trap - EXIT
  run_example "mobile-test-plan-json" "$repo_root/scripts/appfw" product mobile-test --plan --json
  run_log_assertion "mobile-test-plan-status" "$report_dir/docs-example-mobile-test-plan-json.log" '"roadmap_lane": "U5"' "mobile-test plan records U5 lane"
  run_example "mobile-rn-generate-json" "$repo_root/scripts/appfw" product generate --target mobile-rn --json
  run_log_assertion "mobile-rn-generate-command" "$report_dir/docs-example-mobile-rn-generate-json.log" '"command": "generate"' "mobile-rn generate JSON exposes command"
  run_log_assertion "mobile-rn-generate-target" "$report_dir/docs-example-mobile-rn-generate-json.log" '"target": "mobile-rn"' "mobile-rn generate JSON exposes target"
  run_log_assertion "mobile-rn-generate-status" "$report_dir/docs-example-mobile-rn-generate-json.log" '"generator_status": "all-entity-route-shells-emitted"' "mobile-rn generate JSON records generated route-shell status"
  run_log_assertion "mobile-rn-generate-actions" "$report_dir/docs-example-mobile-rn-generate-json.log" '"write_actions": [' "mobile-rn generate JSON records generated write actions"
  run_example "mobile-rn-generate-check-json" "$repo_root/scripts/appfw" product generate --target mobile-rn --check --json
  run_log_assertion "mobile-rn-generate-check-mode" "$report_dir/docs-example-mobile-rn-generate-check-json.log" '"check": true' "mobile-rn generate check JSON records check mode"
  run_log_assertion "mobile-rn-generate-check-actions" "$report_dir/docs-example-mobile-rn-generate-check-json.log" '"action": "verified"' "mobile-rn generate check JSON verifies generated mobile artifacts"
  record_docs_check_subcheck_span \
    "agent-governance-mobile-command-examples" \
    "$agent_governance_mobile_examples_start_ms" \
    "$agent_governance_mobile_examples_start_item_count" \
    "$agent_governance_mobile_examples_start_failure_count" \
    "mobile readiness, spoof rejection, and React Native scaffold command examples"
fi

if should_run_docs_check_subcheck "agent-governance-runtime-command-examples"; then
  agent_governance_runtime_examples_start_ms="$(ms_now)"
  agent_governance_runtime_examples_start_item_count="$(doc_example_item_count)"
  agent_governance_runtime_examples_start_failure_count="$(doc_example_failure_count)"
  run_example "fork-check-json" "$repo_root/scripts/appfw" framework fork-check --json
  run_log_assertion "fork-check-command" "$report_dir/docs-example-fork-check-json.log" '"command": "fork-check"' "fork-check JSON exposes command"
  run_log_assertion "fork-check-lane" "$report_dir/docs-example-fork-check-json.log" '"lane": "U6"' "fork-check JSON records U6 lane"
  run_log_assertion "fork-check-artifact" "$report_dir/docs-example-fork-check-json.log" '"target/appfw/fork-check.json"' "fork-check JSON records retained artifact"
  run_example "composition-check-json" "$repo_root/scripts/appfw" framework composition-check --json
  run_log_assertion "composition-check-command" "$report_dir/docs-example-composition-check-json.log" '"command": "composition-check"' "composition-check JSON exposes command"
  run_log_assertion "composition-check-lane" "$report_dir/docs-example-composition-check-json.log" '"lane": "U7"' "composition-check JSON records U7 lane"
  run_log_assertion "composition-check-artifact" "$report_dir/docs-example-composition-check-json.log" '"target/appfw/composition-check.json"' "composition-check JSON records retained artifact"
  run_log_assertion "composition-check-enforced-artifact" "$report_dir/docs-example-composition-check-json.log" '"target/appfw/composition-check-enforced.json"' "composition-check JSON records retained enforced artifact path"
  run_log_assertion "composition-check-reqwest-budget" "$report_dir/docs-example-composition-check-json.log" '"reqwest_versions"' "composition-check JSON records reqwest duplicate-version evidence"
  run_example "compat-verify-json" "$repo_root/scripts/appfw" product compat-verify --json
  run_json_field_assertion "compat-verify-semantic-not-ok" "$report_dir/docs-example-compat-verify-json.log" "ok" "false" "compat top-level success reflects false substantive checks"
  run_json_field_assertion "compat-verify-gating-ok" "$report_dir/docs-example-compat-verify-json.log" "gating_ok" "true" "compat findings are explicitly non-gating"
  run_json_field_assertion "compat-verify-gate-not-enforced" "$report_dir/docs-example-compat-verify-json.log" "gate.enforced" "false" "compat report-only posture is explicit"
  run_log_assertion "compat-verify-command" "$report_dir/docs-example-compat-verify-json.log" '"command": "compat-verify"' "compat-verify JSON exposes command"
  run_log_assertion "compat-verify-lane" "$report_dir/docs-example-compat-verify-json.log" '"lane": "P6"' "compat-verify JSON records P6 lane"
  run_log_assertion "compat-verify-artifact" "$report_dir/docs-example-compat-verify-json.log" '".appfw/target/appfw/compat-verify.json"' "compat-verify JSON records retained artifact"
  run_example "chat-eval-plan-json" "$repo_root/scripts/appfw" product chat-eval --plan --json
  run_log_assertion "chat-eval-plan-command" "$report_dir/docs-example-chat-eval-plan-json.log" '"command": "chat-eval"' "chat-eval plan JSON exposes command"
  run_log_assertion "chat-eval-plan-lane" "$report_dir/docs-example-chat-eval-plan-json.log" '"roadmap_lane": "CH6"' "chat-eval plan JSON records CH6 lane"
  run_example "chat-eval-json" "$repo_root/scripts/appfw" product chat-eval --json
  run_log_assertion "chat-eval-command" "$report_dir/docs-example-chat-eval-json.log" '"command": "chat-eval"' "chat-eval JSON exposes command"
  run_log_assertion "chat-eval-lane" "$report_dir/docs-example-chat-eval-json.log" '"lane": "CH"' "chat-eval JSON records chat lane"
  run_log_assertion "chat-eval-release-ready-false" "$report_dir/docs-example-chat-eval-json.log" '"release_ready": false' "chat-eval local fixture evidence is not release ready"
  run_log_assertion "chat-eval-red-team-ok" "$report_dir/docs-example-chat-eval-json.log" '"leaks_found": 0' "chat-eval local fixture red-team reports zero leaks"
  run_log_assertion "chat-eval-staged-artifact" "$report_dir/docs-example-chat-eval-json.log" '"mode": "local-fixture"' "chat-eval records local fixture mode"
  run_example_with_allowed_statuses "1" "chat-eval-inject-leak-json" "$repo_root/scripts/appfw" product chat-eval --json --inject-leak --artifact "$report_dir/docs-example-chat-eval-injected.json"
  run_log_assertion "chat-eval-inject-leak-command" "$report_dir/docs-example-chat-eval-inject-leak-json.log" '"command": "chat-eval"' "chat-eval injected leak JSON exposes command before failing closed"
  run_log_assertion "chat-eval-inject-leak-finding" "$report_dir/docs-example-chat-eval-inject-leak-json.log" '"code": "policy_leak"' "chat-eval injected leak reports policy leak"
  run_log_assertion "chat-eval-inject-leak-count" "$report_dir/docs-example-chat-eval-inject-leak-json.log" '"leaks_found": 1' "chat-eval injected leak increments leak count"
  run_example "provider-test-ai-search-plan-json" "$repo_root/scripts/appfw" framework provider-test --provider ai_search --plan --json
  run_log_assertion "provider-test-ai-search-plan-command" "$report_dir/docs-example-provider-test-ai-search-plan-json.log" '"command": "provider-test"' "provider-test AI search plan exposes command"
  run_log_assertion "provider-test-ai-search-plan-lane" "$report_dir/docs-example-provider-test-ai-search-plan-json.log" '"lane": "CH3+CH7"' "provider-test AI search plan records CH3+CH7 lane"
  run_log_assertion "provider-test-ai-search-plan-provider" "$report_dir/docs-example-provider-test-ai-search-plan-json.log" '"provider": "ai_search"' "provider-test AI search plan records AI search provider"
  run_log_assertion "provider-test-ai-search-plan-query" "$report_dir/docs-example-provider-test-ai-search-plan-json.log" '"ai_search.search"' "provider-test AI search plan records search named query"
  run_log_assertion "provider-test-ai-search-plan-gateway-evidence" "$report_dir/docs-example-provider-test-ai-search-plan-json.log" 'server-side secret_ref custody evidence' "provider-test AI search plan records gateway credential-custody evidence"
  run_log_assertion "provider-test-ai-search-release-ready-false" "$report_dir/docs-example-provider-test-ai-search-plan-json.log" '"release_ready": false' "provider-test AI search plan does not claim release readiness"
  run_example "saas-lineage-json" "$repo_root/scripts/appfw" framework saas-lineage --json
  run_log_assertion "saas-lineage-command" "$report_dir/docs-example-saas-lineage-json.log" '"command": "saas-lineage"' "saas-lineage JSON exposes command"
  run_log_assertion "saas-lineage-lane" "$report_dir/docs-example-saas-lineage-json.log" '"lane": "DP4"' "saas-lineage JSON records DP4 lane"
  run_log_assertion "saas-lineage-artifact" "$report_dir/docs-example-saas-lineage-json.log" '"target/appfw/saas-freshness-lineage.json"' "saas-lineage JSON records retained artifact"
  run_log_assertion "saas-lineage-evidence-schema" "$report_dir/docs-example-saas-lineage-json.log" '"appfw.saas.freshness-lineage.v1"' "saas-lineage JSON records live evidence schema"
  run_example_with_allowed_statuses "0 1" "saas-lineage-enforce-json" "$repo_root/scripts/appfw" framework saas-lineage --enforce --json
  run_log_assertion "saas-lineage-enforce-command" "$report_dir/docs-example-saas-lineage-enforce-json.log" '"command": "saas-lineage"' "saas-lineage enforce JSON exposes command"
  run_log_assertion "saas-lineage-enforce-gate" "$report_dir/docs-example-saas-lineage-enforce-json.log" '"enforced": true' "saas-lineage enforce JSON records enforced gate"
  run_log_assertion "saas-lineage-enforce-artifact" "$report_dir/docs-example-saas-lineage-enforce-json.log" '"target/appfw/saas-freshness-lineage-enforced.json"' "saas-lineage enforce JSON records retained enforced artifact"
  run_log_assertion "saas-lineage-enforce-fail-closed" "$report_dir/docs-example-saas-lineage-enforce-json.log" '"saas-freshness-lineage-not-release-ready"' "saas-lineage enforce fails closed without runtime lineage evidence"
  record_docs_check_subcheck_span \
    "agent-governance-runtime-command-examples" \
    "$agent_governance_runtime_examples_start_ms" \
    "$agent_governance_runtime_examples_start_item_count" \
    "$agent_governance_runtime_examples_start_failure_count" \
    "fork, composition, compatibility, and chat-eval command examples"
fi

if should_run_docs_check_subcheck "agent-governance-release-command-examples"; then
  agent_governance_release_examples_start_ms="$(ms_now)"
  agent_governance_release_examples_start_item_count="$(doc_example_item_count)"
  agent_governance_release_examples_start_failure_count="$(doc_example_failure_count)"
  run_example "governance-check-json" "$repo_root/scripts/appfw" framework governance-check --json
  run_log_assertion "governance-check-command" "$report_dir/docs-example-governance-check-json.log" '"command": "governance-check"' "governance-check JSON exposes command"
  run_log_assertion "governance-check-lane" "$report_dir/docs-example-governance-check-json.log" '"lane": "G4"' "governance-check JSON records G4 lane"
  run_log_assertion "governance-check-artifact" "$report_dir/docs-example-governance-check-json.log" '"target/appfw/governance-check.json"' "governance-check JSON records retained artifact"
  run_log_assertion "governance-check-phi-pipeline" "$report_dir/docs-example-governance-check-json.log" '"phi_pipeline": {' "governance-check JSON records PHI pipeline package"
  run_log_assertion "governance-check-phi-schema" "$report_dir/docs-example-governance-check-json.log" '"appfw.phi-pipeline-governance.v1"' "governance-check JSON records PHI pipeline evidence schema"
  run_log_assertion "governance-check-phi-rag-control" "$report_dir/docs-example-governance-check-json.log" '"rag_curation"' "governance-check JSON records RAG curation control"
  run_log_assertion "governance-check-release-authority-package" "$report_dir/docs-example-governance-check-json.log" '"release_authority_package": {' "governance-check JSON records release authority package checklist"
  run_log_assertion "governance-check-release-authority-required" "$report_dir/docs-example-governance-check-json.log" '"required_for_release": true' "governance-check JSON marks release authority package required"
  run_example_with_allowed_statuses "0 1" "governance-check-enforce-json" "$repo_root/scripts/appfw" framework governance-check --enforce --json
  run_log_assertion "governance-check-enforce-command" "$report_dir/docs-example-governance-check-enforce-json.log" '"command": "governance-check"' "governance-check enforce JSON exposes command"
  run_log_assertion "governance-check-enforce-gate" "$report_dir/docs-example-governance-check-enforce-json.log" '"enforced": true' "governance-check enforce JSON records enforced gate"
  run_log_assertion "governance-check-enforce-artifact" "$report_dir/docs-example-governance-check-enforce-json.log" '"target/appfw/governance-check-enforced.json"' "governance-check enforce JSON records retained enforced artifact"
  run_log_assertion "governance-check-enforce-phi-missing" "$report_dir/docs-example-governance-check-enforce-json.log" '"phi-pipeline-evidence-missing"' "governance-check enforce rejects missing PHI pipeline evidence"
  run_log_assertion "governance-check-enforce-provenance" "$report_dir/docs-example-governance-check-enforce-json.log" '"production-provenance-not-release-ready"' "governance-check enforce rejects local-only production provenance"
  run_log_assertion "governance-check-enforce-signing" "$report_dir/docs-example-governance-check-enforce-json.log" '"artifact-signing-not-release-ready"' "governance-check enforce rejects local-only artifact signing"
  run_log_assertion "governance-check-enforce-release-authority-package" "$report_dir/docs-example-governance-check-enforce-json.log" '"missing_or_not_ready"' "governance-check enforce JSON records not-ready release authority items"
  run_example "aibom-check-json" "$repo_root/scripts/appfw" framework aibom-check --json
  run_log_assertion "aibom-check-command" "$report_dir/docs-example-aibom-check-json.log" '"command": "aibom-check"' "aibom-check JSON exposes command"
  run_log_assertion "aibom-check-lane" "$report_dir/docs-example-aibom-check-json.log" '"lane": "SEC-AIBOM"' "aibom-check JSON records SEC-AIBOM lane"
  run_log_assertion "aibom-check-artifact" "$report_dir/docs-example-aibom-check-json.log" '"target/appfw/aibom-check.json"' "aibom-check JSON records retained artifact"
  run_log_assertion "aibom-check-attestation-schema" "$report_dir/docs-example-aibom-check-json.log" '"appfw.aibom.release-attestation.v1"' "aibom-check JSON records release attestation schema"
  run_example_with_allowed_statuses "0 1" "aibom-check-enforce-json" "$repo_root/scripts/appfw" framework aibom-check --enforce --json
  run_log_assertion "aibom-check-enforce-command" "$report_dir/docs-example-aibom-check-enforce-json.log" '"command": "aibom-check"' "aibom-check enforce JSON exposes command"
  run_log_assertion "aibom-check-enforce-gate" "$report_dir/docs-example-aibom-check-enforce-json.log" '"enforced": true' "aibom-check enforce JSON records enforced gate"
  run_log_assertion "aibom-check-enforce-artifact" "$report_dir/docs-example-aibom-check-enforce-json.log" '"target/appfw/aibom-check-enforced.json"' "aibom-check enforce JSON records retained enforced artifact"
  run_log_assertion "aibom-check-enforce-attestation" "$report_dir/docs-example-aibom-check-enforce-json.log" '"aibom-release-attestation-not-release-ready"' "aibom-check enforce rejects missing release AIBOM attestation"
  run_example "prompt-audit-check-json" "$repo_root/scripts/appfw" framework prompt-audit-check --json
  run_log_assertion "prompt-audit-check-command" "$report_dir/docs-example-prompt-audit-check-json.log" '"command": "prompt-audit-check"' "prompt-audit-check JSON exposes command"
  run_log_assertion "prompt-audit-check-lane" "$report_dir/docs-example-prompt-audit-check-json.log" '"lane": "SEC-PROMPTAUDIT"' "prompt-audit-check JSON records SEC-PROMPTAUDIT lane"
  run_log_assertion "prompt-audit-check-artifact" "$report_dir/docs-example-prompt-audit-check-json.log" '"target/appfw/prompt-audit-check.json"' "prompt-audit-check JSON records retained artifact"
  run_log_assertion "prompt-audit-check-schema" "$report_dir/docs-example-prompt-audit-check-json.log" '"appfw.prompt-audit.release-evidence.v1"' "prompt-audit-check JSON records release evidence schema"
  run_log_assertion "prompt-audit-check-kill-switch" "$report_dir/docs-example-prompt-audit-check-json.log" '"APP_CHAT_KILL_SWITCH_ACTIVE"' "prompt-audit-check JSON records kill-switch env var"
  run_example_with_allowed_statuses "0 1" "prompt-audit-check-enforce-json" "$repo_root/scripts/appfw" framework prompt-audit-check --enforce --json
  run_log_assertion "prompt-audit-check-enforce-command" "$report_dir/docs-example-prompt-audit-check-enforce-json.log" '"command": "prompt-audit-check"' "prompt-audit-check enforce JSON exposes command"
  run_log_assertion "prompt-audit-check-enforce-gate" "$report_dir/docs-example-prompt-audit-check-enforce-json.log" '"enforced": true' "prompt-audit-check enforce JSON records enforced gate"
  run_log_assertion "prompt-audit-check-enforce-artifact" "$report_dir/docs-example-prompt-audit-check-enforce-json.log" '"target/appfw/prompt-audit-check-enforced.json"' "prompt-audit-check enforce JSON records retained enforced artifact"
  run_log_assertion "prompt-audit-check-enforce-fail-closed" "$report_dir/docs-example-prompt-audit-check-enforce-json.log" '"prompt-audit-release-evidence-not-ready"' "prompt-audit-check enforce fails closed without managed prompt audit evidence"
  run_example "ai-gateway-decision-json" "$repo_root/scripts/appfw" framework ai-gateway-decision --json
  run_log_assertion "ai-gateway-decision-command" "$report_dir/docs-example-ai-gateway-decision-json.log" '"command": "ai-gateway-decision"' "ai-gateway-decision JSON exposes command"
  run_log_assertion "ai-gateway-decision-lane" "$report_dir/docs-example-ai-gateway-decision-json.log" '"lane": "CH7"' "ai-gateway-decision JSON records CH7 lane"
  run_log_assertion "ai-gateway-decision-artifact" "$report_dir/docs-example-ai-gateway-decision-json.log" '"target/appfw/ai-gateway-decision.json"' "ai-gateway-decision JSON records retained artifact"
  run_log_assertion "ai-gateway-decision-schema" "$report_dir/docs-example-ai-gateway-decision-json.log" '"appfw.ai-gateway-decision.v1"' "ai-gateway-decision JSON records decision evidence schema"
  run_example_with_allowed_statuses "0 1" "ai-gateway-decision-enforce-json" "$repo_root/scripts/appfw" framework ai-gateway-decision --enforce --json
  run_log_assertion "ai-gateway-decision-enforce-command" "$report_dir/docs-example-ai-gateway-decision-enforce-json.log" '"command": "ai-gateway-decision"' "ai-gateway-decision enforce JSON exposes command"
  run_log_assertion "ai-gateway-decision-enforce-gate" "$report_dir/docs-example-ai-gateway-decision-enforce-json.log" '"enforced": true' "ai-gateway-decision enforce JSON records enforced gate"
  run_log_assertion "ai-gateway-decision-enforce-artifact" "$report_dir/docs-example-ai-gateway-decision-enforce-json.log" '"target/appfw/ai-gateway-decision-enforced.json"' "ai-gateway-decision enforce JSON records retained enforced artifact"
  run_log_assertion "ai-gateway-decision-enforce-missing" "$report_dir/docs-example-ai-gateway-decision-enforce-json.log" '"ai-gateway-decision-evidence-missing"' "ai-gateway-decision enforce rejects missing stakeholder decision evidence"
  run_example "wave2-status-json" "$repo_root/scripts/appfw" framework wave2-status --json
  run_log_assertion "wave2-status-command" "$report_dir/docs-example-wave2-status-json.log" '"command": "wave2-status"' "wave2-status JSON exposes command"
  run_log_assertion "wave2-status-lane" "$report_dir/docs-example-wave2-status-json.log" '"lane": "north-star-wave-2"' "wave2-status JSON records Wave 2 lane"
  run_log_assertion "wave2-status-artifact" "$report_dir/docs-example-wave2-status-json.log" '"target/appfw/wave2-readiness.json"' "wave2-status JSON records retained artifact"
  run_log_assertion "wave2-status-total-lane-count" "$report_dir/docs-example-wave2-status-json.log" '"total_lane_count": 10' "wave2-status JSON distinguishes total lanes from local executable slices"
  run_log_assertion "wave2-status-local-preflight-count" "$report_dir/docs-example-wave2-status-json.log" '"local_preflight_lane_count": 1' "wave2-status JSON reports branch-level local preflight lane count separately"
  run_log_absence_assertion "wave2-status-no-angle-placeholder" "$report_dir/docs-example-wave2-status-json.log" '<real-' "wave2-status external plan avoids angle-bracket placeholder tokens"
  run_wave2_external_plan_artifact_assertion "wave2-status-external-plan-release-artifacts" "$report_dir/docs-example-wave2-status-json.log" "wave2-status external plan requires release-staged artifacts"
  run_log_assertion "wave2-status-g1-live-evidence" "$report_dir/docs-example-wave2-status-json.log" '"live_evidence": {' "wave2-status JSON reports G1 live evidence schema posture"
  run_log_assertion "wave2-status-g1-live-schema" "$report_dir/docs-example-wave2-status-json.log" '"schema_ok": false' "wave2-status JSON does not accept missing G1 live evidence"
  run_log_assertion "wave2-status-mobile-command-evidence" "$report_dir/docs-example-wave2-status-json.log" '"command_evidence":' "wave2-status JSON passes through U5 command evidence posture, even when absent"
  run_log_assertion "wave2-status-mobile-device-evidence" "$report_dir/docs-example-wave2-status-json.log" '"device_evidence":' "wave2-status JSON passes through U5 device evidence posture, even when absent"
  run_log_assertion "wave2-status-mobile-store-track-evidence" "$report_dir/docs-example-wave2-status-json.log" '"store_track_evidence":' "wave2-status JSON passes through U5 store-track evidence posture, even when absent"
  run_log_assertion "wave2-status-chat-eval-lane" "$report_dir/docs-example-wave2-status-json.log" '"code": "CH6"' "wave2-status JSON records the CH6 chat-eval lane"
  run_log_assertion "wave2-status-chat-eval-local-fixture" "$report_dir/docs-example-wave2-status-json.log" '"local_fixture": {' "wave2-status JSON records deterministic chat-eval local fixture posture"
  run_log_assertion "wave2-status-chat-eval-judge-evidence" "$report_dir/docs-example-wave2-status-json.log" '"judge_live_evidence": {' "wave2-status JSON records missing judge/live chat-eval evidence"
  run_log_assertion "wave2-status-chat-eval-required-artifact" "$report_dir/docs-example-wave2-status-json.log" '"target/appfw/wave4/ch6-chat-eval.json"' "wave2-status JSON records staged CH6 release evidence artifact"
  run_log_assertion "wave2-status-chat-eval-managed-evidence" "$report_dir/docs-example-wave2-status-json.log" '"target/appfw/chat-eval-judge-evidence.json"' "wave2-status JSON records managed CH6 judge/live evidence artifact"
  run_log_assertion "wave2-status-release-ready" "$report_dir/docs-example-wave2-status-json.log" '"release_ready": false' "wave2-status does not claim release readiness from local evidence alone"
  run_log_assertion "wave2-status-remaining-gates" "$report_dir/docs-example-wave2-status-json.log" '"remaining_external_gates"' "wave2-status records remaining external gates"
  run_log_assertion "wave2-status-external-evidence-plan" "$report_dir/docs-example-wave2-status-json.log" '"external_evidence_plan": [' "wave2-status records external evidence plan"
  run_log_assertion "wave2-status-external-plan-u2" "$report_dir/docs-example-wave2-status-json.log" '"lane": "U2"' "wave2-status external evidence plan records the G1-dependent harness lane"
  run_log_assertion "wave2-status-external-plan-g2" "$report_dir/docs-example-wave2-status-json.log" '"lane": "G2"' "wave2-status external evidence plan records the G1-dependent governed-action lane"
  run_log_assertion "wave2-status-external-plan-artifacts" "$report_dir/docs-example-wave2-status-json.log" '"required_artifacts": [' "wave2-status external evidence plan records retained artifact requirements"
  run_log_assertion "wave2-status-external-plan-artifact-status" "$report_dir/docs-example-wave2-status-json.log" '"required_artifact_status": [' "wave2-status external evidence plan records required artifact presence status"
  run_log_assertion "wave2-status-external-required-artifacts-summary" "$report_dir/docs-example-wave2-status-json.log" '"external_required_artifacts": {' "wave2-status JSON records external required artifact summary"
  run_log_assertion "wave2-status-external-plan-owner-boundary" "$report_dir/docs-example-wave2-status-json.log" '"owner_boundary":' "wave2-status external evidence plan records owner boundaries"
  run_log_assertion "wave2-status-external-servicenow-command" "$report_dir/docs-example-wave2-status-json.log" 'APPFW_SERVICENOW_GOVERNED_WRITE_EVIDENCE_FILE=' "wave2-status external evidence plan records ServiceNow governed-write evidence hook"
  run_log_assertion "wave2-status-external-release-command" "$report_dir/docs-example-wave2-status-json.log" 'scripts/ci/bitbucket-release-gate.sh' "wave2-status external evidence plan records managed release gate command"
  chat_eval_judge_required_dir="$report_dir/docs-example-chat-eval-judge-required"
  rm -rf "$chat_eval_judge_required_dir"
  mkdir -p "$chat_eval_judge_required_dir"
  run_example_with_allowed_statuses "1" "release-evidence-chat-eval-judge-required-missing" \
    env APPFW_RELEASE_ARTIFACT_DIR="$chat_eval_judge_required_dir" \
      APPFW_REQUIRE_CHAT_EVAL_JUDGE_EVIDENCE=true \
      bash "$repo_root/scripts/ci/release-evidence-check.sh" --strict
  run_log_assertion "release-evidence-chat-eval-judge-required-message" "$report_dir/docs-example-release-evidence-chat-eval-judge-required-missing.log" "APPFW_REQUIRE_CHAT_EVAL_JUDGE_EVIDENCE" "release evidence fails closed when CH6 judge/live evidence is required but missing"
  run_example_with_allowed_statuses "0 1" "wave2-status-strict-json" "$repo_root/scripts/appfw" framework wave2-status --json --strict
  run_log_assertion "wave2-status-strict-command" "$report_dir/docs-example-wave2-status-strict-json.log" '"command": "wave2-status"' "wave2-status strict JSON exposes command before failing closed"
  run_log_assertion "wave2-status-strict-live-gated" "$report_dir/docs-example-wave2-status-strict-json.log" '"status": "live-gated"' "wave2-status strict JSON records live-gated status"
  run_log_assertion "wave2-status-strict-release-ready" "$report_dir/docs-example-wave2-status-strict-json.log" '"release_ready": false' "wave2-status strict fails closed without release-ready evidence"
  wave2_status_bundle_dir="$report_dir/docs-example-wave2-status-report-dir"
  wave2_status_bundle_artifact="target/appfw/docs-example-wave2-status-report-dir/wave2-readiness.json"
  rm -rf "$wave2_status_bundle_dir"
  mkdir -p "$wave2_status_bundle_dir"
  run_example "wave2-status-report-dir-json" env APPFW_WAVE2_REPORT_DIR="$wave2_status_bundle_dir" "$repo_root/scripts/appfw" framework wave2-status --json
  run_log_assertion "wave2-status-report-dir-artifact" "$report_dir/docs-example-wave2-status-report-dir-json.log" "$wave2_status_bundle_artifact" "wave2-status custom report dir records retained artifact path"
  run_log_assertion "wave2-status-report-dir-canonical-fallback" "$report_dir/docs-example-wave2-status-report-dir-json.log" '"target/appfw/governed-write-posture.json"' "wave2-status custom report dir falls back to canonical lane evidence"
  bitbucket_gate_evidence="$repo_root/target/appfw/bitbucket-release-gate.json"
  bitbucket_gate_evidence_backup="$report_dir/docs-check-bitbucket-release-gate.backup.json"
  bitbucket_gate_evidence_was_present=0
  bitbucket_gate_restore_armed=1
  restore_docs_check_bitbucket_gate_evidence() {
    if [[ "${bitbucket_gate_restore_armed:-0}" != "1" ]]; then
      return 0
    fi
    if [[ "$bitbucket_gate_evidence_was_present" == "1" ]]; then
      mv "$bitbucket_gate_evidence_backup" "$bitbucket_gate_evidence"
    else
      rm -f "$bitbucket_gate_evidence"
      rm -f "$bitbucket_gate_evidence_backup"
    fi
    bitbucket_gate_restore_armed=0
  }
  trap restore_docs_check_bitbucket_gate_evidence EXIT
  if [[ -f "$bitbucket_gate_evidence" ]]; then
    cp "$bitbucket_gate_evidence" "$bitbucket_gate_evidence_backup"
    bitbucket_gate_evidence_was_present=1
  fi
  python3 - "$bitbucket_gate_evidence" <<'PY'
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
path.parent.mkdir(parents=True, exist_ok=True)
path.write_text(
    json.dumps(
        {
            "command": "bitbucket-release-gate",
            "focused_evidence": True,
            "ok": True,
            "release_ready": True,
            "release_requirements": {
                "security_assurance_decision_required": True,
                "production_attestations_required": True,
                "ops_certification_required": True,
                "live_ops_evidence_required": True,
                "pds_baseline_evidence_required": True,
                "performance_evidence_required": True,
                "release_identity_required": True,
            },
            "release_blockers": [],
        },
        indent=2,
        sort_keys=True,
    )
    + "\n",
    encoding="utf-8",
)
PY
  run_example "wave2-status-focused-release-json" "$repo_root/scripts/appfw" framework wave2-status --json
  run_log_assertion "wave2-status-focused-release-detected" "$report_dir/docs-example-wave2-status-focused-release-json.log" '"managed_release_gate_focused_evidence": true' "wave2-status detects focused Bitbucket release evidence"
  run_log_assertion "wave2-status-focused-release-not-ready" "$report_dir/docs-example-wave2-status-focused-release-json.log" '"managed_release_gate_ready": false' "wave2-status refuses focused evidence as managed release readiness"
  restore_docs_check_bitbucket_gate_evidence
  trap - EXIT
  record_docs_check_subcheck_span \
    "agent-governance-release-command-examples" \
    "$agent_governance_release_examples_start_ms" \
    "$agent_governance_release_examples_start_item_count" \
    "$agent_governance_release_examples_start_failure_count" \
    "governance enforcement and Wave 2 aggregate command examples"
fi

if should_run_docs_check_subcheck "packaging-dependency-command-examples"; then
  packaging_dependency_examples_start_ms="$(ms_now)"
  packaging_dependency_examples_start_item_count="$(doc_example_item_count)"
  packaging_dependency_examples_start_failure_count="$(doc_example_failure_count)"
  run_example "local-live-preflight-plan-json" "$repo_root/scripts/appfw" framework local-live-preflight --plan --json
  run_log_assertion "local-live-preflight-plan-command" "$report_dir/docs-example-local-live-preflight-plan-json.log" '"command": "local-live-release-preflight"' "local live preflight plan JSON exposes command"
  run_log_assertion "local-live-preflight-plan-mode" "$report_dir/docs-example-local-live-preflight-plan-json.log" '"mode": "plan"' "local live preflight plan JSON records plan mode"
  run_log_assertion "local-live-preflight-plan-artifact" "$report_dir/docs-example-local-live-preflight-plan-json.log" '"target/appfw/local-live-release-preflight-plan.json"' "local live preflight plan records retained artifact"
  run_log_assertion "local-live-preflight-required-inputs" "$report_dir/docs-example-local-live-preflight-plan-json.log" '"required_live_inputs"' "local live preflight plan records required live inputs"
  run_log_assertion "local-live-preflight-external-gates" "$report_dir/docs-example-local-live-preflight-plan-json.log" '"external_release_authority_gates"' "local live preflight plan records external release authority gates"
  run_log_assertion "local-live-preflight-external-gate-authority" "$report_dir/docs-example-local-live-preflight-plan-json.log" '"authority": "managed-release-ci"' "local live preflight plan labels managed release authority gates"
  run_log_assertion "local-live-preflight-local-satisfies-false" "$report_dir/docs-example-local-live-preflight-plan-json.log" '"local_preflight_satisfies": false' "local live preflight plan does not satisfy external release gates"
  run_log_assertion "local-live-preflight-boundary-false" "$report_dir/docs-example-local-live-preflight-plan-json.log" '"local_preflight_satisfies_release": false' "local live preflight boundary refuses release authority"
  run_example "framework-package-plan-json" "$repo_root/scripts/appfw" framework package --plan --json
  run_log_assertion "framework-package-plan-command" "$report_dir/docs-example-framework-package-plan-json.log" '"command":"package"' "framework package plan JSON exposes command"
  run_log_assertion "framework-package-plan-proget" "$report_dir/docs-example-framework-package-plan-json.log" '"proget_posture":"uploadable-artifacts-produced-locally-ci-publishes"' "framework package plan records ProGet posture"
  run_log_assertion "framework-package-plan-binaries" "$report_dir/docs-example-framework-package-plan-json.log" '"app-framework-binaries"' "framework package plan records binaries pack"
  run_log_assertion "framework-package-plan-crates" "$report_dir/docs-example-framework-package-plan-json.log" '"app-framework-crates-publish-plan"' "framework package plan records crate publish plan"
  run_log_assertion "framework-package-plan-docs" "$report_dir/docs-example-framework-package-plan-json.log" '"app-framework-product-docs"' "framework package plan records product docs pack"
  framework_package_version="$(awk -F\" '/^version =/{print $2; exit}' "$repo_root/appfw_cli/Cargo.toml")"
  framework_crates_plan="$report_dir/proget/app-framework-crates-publish-plan-${framework_package_version}.json"
  run_log_assertion "framework-package-plan-saas-core" "$framework_crates_plan" '"appfw-saas-core"' "framework package plan includes SaaS core crate"
  run_log_assertion "framework-package-plan-saas-provider" "$framework_crates_plan" '"appfw-provider-salesforce"' "framework package plan includes SaaS provider crates"
  run_example "dependency-check-json" "$repo_root/scripts/appfw" dependency-check --json --offline
  run_log_assertion "dependency-check-command" "$report_dir/docs-example-dependency-check-json.log" '"command": "dependency-check"' "dependency check JSON exposes command"
  run_log_assertion "dependency-check-artifact" "$report_dir/docs-example-dependency-check-json.log" '"artifact": "target/appfw/dependency-check.json"' "dependency check JSON records retained artifact"
  run_log_assertion "dependency-check-policy-file" "$report_dir/docs-example-dependency-check-json.log" '"dependency_policy_file": "dependency-check.toml"' "dependency check JSON records policy file"
  run_log_assertion "dependency-check-accepted-osv-count" "$report_dir/docs-example-dependency-check-json.log" '"accepted_osv_finding_count": 0' "dependency check JSON records accepted OSV count"
  run_example "dependency-plan-json" "$repo_root/scripts/appfw" dependency-plan --json --offline --package async-graphql
  run_log_assertion "dependency-plan-command" "$report_dir/docs-example-dependency-plan-json.log" '"command": "dependency-plan"' "dependency plan JSON exposes command"
  run_log_assertion "dependency-plan-artifact" "$report_dir/docs-example-dependency-plan-json.log" '"artifact": "target/appfw/dependency-plan-async-graphql.json"' "dependency plan JSON records retained artifact"
  run_log_assertion "dependency-plan-risk" "$report_dir/docs-example-dependency-plan-json.log" '"risk"' "dependency plan JSON records risk"
  run_example "mcp-posture-json" env APP_MCP_ENABLED=false "$repo_root/scripts/appfw" mcp-posture --json
  run_log_assertion "mcp-posture-release-posture" "$report_dir/docs-example-mcp-posture-json.log" '"release_posture":"excluded"' "mcp posture JSON records release posture"
  run_example_with_allowed_statuses "0 1" "pds-baseline-json" "$repo_root/scripts/appfw" framework pds-baseline --json
  run_log_assertion "pds-baseline-command" "$report_dir/docs-example-pds-baseline-json.log" '"command": "pds-baseline"' "PDS baseline JSON exposes command"
  run_log_assertion "pds-baseline-artifact" "$report_dir/docs-example-pds-baseline-json.log" '"traceability_doc": "docs/release/pds-security-baseline-traceability.md"' "PDS baseline JSON records traceability doc"
  run_log_assertion "pds-baseline-decision-summary" "$report_dir/docs-example-pds-baseline-json.log" '"decision_summary": {' "PDS baseline JSON records decision summary"
  run_log_assertion "pds-baseline-remediation" "$report_dir/docs-example-pds-baseline-json.log" '"id": "LIVE-015"' "PDS baseline JSON records LIVE-015 remediation"
  run_example "handoff-json" "$repo_root/scripts/appfw" handoff --json
  run_log_assertion "handoff-no-unknown-changed-surfaces" "$report_dir/docs-example-handoff-json.log" '"unknown_changed": 0' "handoff JSON keeps changed surfaces classified for agent routing"
  record_docs_check_subcheck_span \
    "packaging-dependency-command-examples" \
    "$packaging_dependency_examples_start_ms" \
    "$packaging_dependency_examples_start_item_count" \
    "$packaging_dependency_examples_start_failure_count" \
    "local live preflight, framework package, dependency, MCP posture, PDS baseline, and handoff command examples"
fi

if should_run_docs_check_subcheck "compiled-cli-command-examples"; then
  compiled_cli_examples_start_ms="$(ms_now)"
  compiled_cli_examples_start_item_count="$(doc_example_item_count)"
  compiled_cli_examples_start_failure_count="$(doc_example_failure_count)"
  run_example "appfw-cli-new-list-profiles-json" "$docs_appfw_cli" --repo-root "$repo_root" new --list-profiles --json
  run_log_assertion "appfw-cli-new-list-profiles-profile-path" "$report_dir/docs-example-appfw-cli-new-list-profiles-json.log" '"profile_path"' "packaged CLI profile JSON exposes profile_path"
  run_example "compiled-cli-new-list-profiles-json" "$docs_appfw_appgen" --repo-root "$repo_root" new --list-profiles --json
  record_docs_check_subcheck_span \
    "compiled-cli-command-examples" \
    "$compiled_cli_examples_start_ms" \
    "$compiled_cli_examples_start_item_count" \
    "$compiled_cli_examples_start_failure_count" \
    "prebuilt appfw-cli and app_gen CLI binary command examples"
fi
if [[ -z "$focused_subcheck" ]]; then
  run_fast_docs_check
fi

fi

if [[ -n "$focused_subcheck" && "$DOCS_CHECK_SUBCHECK_MATCHED" != "1" ]]; then
  DOC_EXAMPLE_FAILED=1
  if [[ "$json" != "1" || "$progress" == "1" ]]; then
    printf 'Requested docs-check subcheck was not executed: %s\n' "$focused_subcheck" >&2
  fi
fi

docs_check_ok=false
if [[ "$DOC_EXAMPLE_FAILED" == "0" ]]; then
  docs_check_ok=true
fi
write_docs_check_subchecks_artifact "$docs_check_ok"
write_docs_check_timing_artifact "$docs_check_ok"
if [[ "$enforce_budget" == "1" && "$DOCS_CHECK_BUDGET_OK" != "true" ]]; then
  DOC_EXAMPLE_FAILED=1
  docs_check_ok=false
  write_docs_check_subchecks_artifact "$docs_check_ok"
  write_docs_check_timing_artifact "$docs_check_ok"
  if [[ "$json" != "1" || "$progress" == "1" ]]; then
    printf 'Docs-check timing budget exceeded: %sms > %sms\n' "$DOCS_CHECK_TOTAL_WALL_MS" "$DOCS_CHECK_BUDGET_MS" >&2
  fi
fi

if [[ "$json" == "1" ]]; then
  printf '{"command":"docs-check","ok":'
  printf '%s' "$docs_check_ok"
  printf ',"requested_mode":'
  json_escape "$mode"
  printf ',"selected_mode":'
  json_escape "$effective_mode"
  printf ',"mode_reason":'
  json_escape "$mode_reason"
  printf ',"changed_files_count":%s' "$changed_files_count"
  printf ',"changed_surface_artifact":'
  json_escape "$docs_check_changed_surface_artifact_rel"
  printf ',"changed_surface_requires_full":%s' "$changed_surface_requires_full_result"
  printf ',"changed_surface_trigger_count":%s' "$changed_surface_trigger_count"
  printf ',"timing_artifact":'
  json_escape "$docs_check_timing_artifact_rel"
  printf ',"subchecks_artifact":'
  json_escape "$docs_check_subchecks_artifact_rel"
  print_docs_check_subcheck_focus_json_fields
  printf ',"budget_ms":%s' "$DOCS_CHECK_BUDGET_MS"
  printf ',"budget_ok":%s' "$DOCS_CHECK_BUDGET_OK"
  printf ',"budget_enforced":'
  if [[ "$enforce_budget" == "1" ]]; then
    printf 'true'
  else
    printf 'false'
  fi
  printf ',"total_wall_ms":%s' "$DOCS_CHECK_TOTAL_WALL_MS"
  printf ',"aggregate_phase_ms":%s' "$DOCS_CHECK_AGGREGATE_PHASE_MS"
  printf ',"aggregate_example_ms":%s' "$DOCS_CHECK_AGGREGATE_EXAMPLE_MS"
  printf ',"unattributed_wall_ms":%s' "$DOCS_CHECK_UNATTRIBUTED_WALL_MS"
  printf ',"phase_count":%s' "$DOCS_CHECK_PHASE_COUNT"
  printf ',"subcheck_count":%s' "${#DOCS_CHECK_SUBCHECK_ITEMS[@]}"
  printf ',"example_count":%s' "$DOCS_CHECK_EXAMPLE_COUNT"
  printf ',"slowest_example":{"name":'
  json_escape "$DOCS_CHECK_SLOWEST_EXAMPLE_NAME"
  printf ',"elapsed_ms":%s}' "$DOCS_CHECK_SLOWEST_EXAMPLE_MS"
  print_docs_check_subcheck_guidance_json_fields
  printf ',"examples":['
  first=1
  for item in "${DOC_EXAMPLE_ITEMS[@]}"; do
    IFS='|' read -r name ok exit_code log_file command_text elapsed_ms <<<"$item"
    example_json_item "$first" "$name" "$ok" "$exit_code" "$log_file" "$command_text" "${elapsed_ms:-0}"
    first=0
  done
  printf ']}\n'
fi

exit "$DOC_EXAMPLE_FAILED"
