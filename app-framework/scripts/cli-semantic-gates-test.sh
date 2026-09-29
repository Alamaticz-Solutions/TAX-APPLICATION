#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"
fixture_root="$(mktemp -d "${TMPDIR:-/tmp}/appfw-cli-semantic-gates.XXXXXX")"
trap 'rm -rf "$fixture_root"' EXIT

assert_json() {
  local path="$1"
  local expression="$2"
  local detail="$3"

  python3 - "$path" "$expression" "$detail" <<'PY'
import json
import sys

path, expression, detail = sys.argv[1:4]
with open(path, "r", encoding="utf-8") as handle:
    data = json.load(handle)
if not eval(expression, {"__builtins__": {}, "all": all, "any": any, "data": data}, {}):
    raise SystemExit(f"semantic assertion failed: {detail}")
PY
}

instruction_output="$fixture_root/instruction-route.json"
"$repo_root/scripts/appfw" framework instructions \
  --task framework-docs-ia \
  --role coding-agent \
  --change-class C \
  --json >"$instruction_output"
assert_json "$instruction_output" 'data.get("schema") == "appfw_instruction_route@1"' "instruction route exposes the versioned envelope"
assert_json "$instruction_output" 'data.get("instructions", {}).get("producer_role_card", {}).get("heading") == "Coding Agent"' "instruction route selects exactly one producer role card"
assert_json "$instruction_output" 'data.get("instructions", {}).get("skill", {}).get("path") == "agent_skills/framework-docs-ia/SKILL.md"' "instruction route selects the task skill"
assert_json "$instruction_output" 'data.get("independent_reviewer_route", {}).get("review_depth") == "comprehensive"' "Class C route requires comprehensive review"
assert_json "$instruction_output" 'data.get("implementer_route", {}).get("role") != data.get("independent_reviewer_route", {}).get("role")' "implementer and reviewer routes remain distinct"

instruction_json_first="$fixture_root/instruction-route-json-first.json"
"$repo_root/scripts/appfw" framework instructions \
  --json \
  --task framework-docs-ia \
  --role coding-agent \
  --change-class C >"$instruction_json_first"
assert_json "$instruction_json_first" 'data.get("ok") is True' "wrapper preserves valid instruction-route flag order"

mode_status="$fixture_root/mode-status.json"
"$repo_root/scripts/appfw" mode status --json >"$mode_status"
assert_json "$mode_status" 'data.get("ok") is True and data.get("delivery_profile", {}).get("profile") in {"accelerated", "candidate"}' "root mode status returns a delivery profile"
for mode_failure in product_mode missing_mode; do
  mode_output="$fixture_root/$mode_failure.json"
  set +e
  if [[ "$mode_failure" == "product_mode" ]]; then
    "$repo_root/scripts/appfw" product mode status --json >"$mode_output"
  else
    "$repo_root/scripts/appfw" mode --json >"$mode_output"
  fi
  mode_status_code=$?
  set -e
  [[ $mode_status_code -ne 0 ]] || { echo "$mode_failure returned zero" >&2; exit 1; }
  assert_json "$mode_output" 'data.get("ok") is False' "$mode_failure fails parseably"
done

instruction_repeat="$fixture_root/instruction-route-repeat.json"
"$repo_root/scripts/appfw" framework instructions \
  --task framework-docs-ia \
  --role coding-agent \
  --change-class C \
  --json >"$instruction_repeat"
cmp "$instruction_output" "$instruction_repeat"

for failure_case in unknown mismatch ambiguous duplicate_json unsupported_check positional_missing; do
  failure_output="$fixture_root/instruction-route-$failure_case.json"
  case "$failure_case" in
    unknown)
      failure_args=(framework instructions --task unknown --role coding-agent --change-class C --json)
      ;;
    mismatch)
      failure_args=(product instructions --task framework-docs-ia --role coding-agent --change-class C --json)
      ;;
    ambiguous)
      failure_args=(framework instructions --task framework-docs-ia --task framework-generator --role coding-agent --change-class C --json)
      ;;
    duplicate_json)
      failure_args=(framework instructions --task framework-docs-ia --role coding-agent --change-class C --json --json)
      ;;
    unsupported_check)
      failure_args=(framework instructions --task framework-docs-ia --role coding-agent --change-class C --json --check)
      ;;
    positional_missing)
      failure_args=(framework instructions --task --json framework-docs-ia --role coding-agent --change-class C)
      ;;
  esac
  set +e
  "$repo_root/scripts/appfw" "${failure_args[@]}" >"$failure_output"
  failure_status=$?
  set -e
  [[ $failure_status -ne 0 ]] || {
    echo "instruction route $failure_case input returned zero" >&2
    exit 1
  }
  assert_json "$failure_output" 'data.get("ok") is False' "$failure_case instruction route fails with parseable JSON"
  case "$failure_case" in
    duplicate_json)
      assert_json "$failure_output" 'data.get("error", {}).get("code") == "ambiguous_instruction_route_input"' "duplicate --json reaches the typed resolver"
      ;;
    unsupported_check)
      assert_json "$failure_output" 'data.get("error", {}).get("code") == "unknown_instruction_route_option"' "unsupported --check reaches the typed resolver"
      ;;
    positional_missing)
      assert_json "$failure_output" 'data.get("error", {}).get("code") == "missing_instruction_route_input" and data.get("error", {}).get("field") == "task"' "malformed positional task value reaches the typed resolver"
      ;;
  esac
done

missing_root="$fixture_root/missing-manifest"
mkdir -p "$missing_root/report"
missing_output="$fixture_root/compat-missing.json"
set +e
APPFW_APP_ROOT="$missing_root" \
APPFW_REPORT_ROOT="$missing_root/report" \
  "$repo_root/scripts/appfw" product compat-verify --json >"$missing_output"
missing_status=$?
set -e
[[ $missing_status -ne 0 ]] || {
  echo "compat-verify missing required manifest returned zero" >&2
  exit 1
}
assert_json "$missing_output" 'data.get("ok") is False' "missing manifest is not semantic success"
assert_json "$missing_output" 'data.get("gating_ok") is False' "missing manifest fails the enforced gate"

report_only_root="$fixture_root/report-only"
mkdir -p "$report_only_root/.appfw" "$report_only_root/report"
printf 'app:\n  name: semantic-fixture\n' >"$report_only_root/.appfw/manifest.yaml"
report_only_output="$fixture_root/compat-report-only.json"
APPFW_APP_ROOT="$report_only_root" \
APPFW_REPORT_ROOT="$report_only_root/report" \
  "$repo_root/scripts/appfw" product compat-verify --json >"$report_only_output"
assert_json "$report_only_output" 'data.get("ok") is False' "false substantive checks cannot produce top-level success"
assert_json "$report_only_output" 'data.get("gating_ok") is True' "report-only findings remain explicitly non-gating"
assert_json "$report_only_output" 'all(item.get("ok") is True or item.get("report_only") is True for item in data.get("checks", []))' "every false non-gating check is classified"

validate_command=$'  - command: scripts/appfw product validate --json\n    namespace: product'
handoff_command=$'  - command: scripts/appfw product handoff --json\n    namespace: product'
default_report_root=$'  - path: .appfw/target/appfw\n    ownership: product_evidence'
lifecycle_root=$'  - path: target/appfw\n    ownership: lifecycle_evidence'
both_default_roots="${default_report_root}"$'\n'"${lifecycle_root}"
validate_and_handoff_commands="${validate_command}"$'\n'"${handoff_command}"

write_harness_profile() {
  local root="$1"
  local commands="$2"
  local writable_paths="$3"
  local handoff_artifact="$4"

  mkdir -p "$root/.appfw"
  cat >"$root/.appfw/agent-profile.yaml" <<EOF
schema_version: 1
name: semantic-harness-fixture
allowed_commands:
${commands}
writable_paths:
  - path: .appfw/model
    ownership: application_source
EOF
  printf '%s\n' "$writable_paths" >>"$root/.appfw/agent-profile.yaml"
  cat >>"$root/.appfw/agent-profile.yaml" <<EOF
network_policy: disabled
live_service_policy: disabled
sensitive_capabilities:
  mcp: false
  kafka: false
  release: false
  saas_governed_write: false
threat_controls:
  - ASI01
  - ASI02
  - ASI03
review_checkpoints:
  - handoff
EOF
  if [[ -n "$handoff_artifact" ]]; then
    cat >>"$root/.appfw/agent-profile.yaml" <<EOF
handoff:
  required: true
  artifact: $handoff_artifact
EOF
  fi
  printf 'risk_acceptance: []\n' >>"$root/.appfw/agent-profile.yaml"
}

mismatch_root="$fixture_root/handoff-mismatch"
mismatch_output="$fixture_root/handoff-mismatch.json"
write_harness_profile "$mismatch_root" "$validate_and_handoff_commands" "$both_default_roots" ".appfw/target/appfw/agent-handoff.json"
set +e
APPFW_APP_ROOT="$mismatch_root" \
APPFW_FRAMEWORK_ROOT="$repo_root" \
APPFW_REPORT_ROOT="$mismatch_root/.appfw/target/appfw" \
  "$repo_root/scripts/appfw" product harness-check --json >"$mismatch_output"
mismatch_status=$?
set -e
[[ $mismatch_status -ne 0 ]] || {
  echo "harness-check accepted a mismatched lifecycle handoff artifact" >&2
  exit 1
}
assert_json "$mismatch_output" 'data.get("ok") is False' "mismatched handoff contract is not semantic success"
assert_json "$mismatch_output" 'any(item.get("check") == "handoff-contract" for item in data.get("violations", []))' "mismatched handoff artifact fails the handoff contract"

missing_root="$fixture_root/handoff-missing-writable-root"
missing_output="$fixture_root/handoff-missing-writable-root.json"
write_harness_profile "$missing_root" "$validate_and_handoff_commands" "$default_report_root" "target/appfw/agent-handoff.json"
set +e
APPFW_APP_ROOT="$missing_root" \
APPFW_FRAMEWORK_ROOT="$repo_root" \
APPFW_REPORT_ROOT="$missing_root/.appfw/target/appfw" \
  "$repo_root/scripts/appfw" product harness-check --json >"$missing_output"
missing_status=$?
set -e
[[ $missing_status -ne 0 ]] || {
  echo "harness-check accepted lifecycle handoff without writable coverage" >&2
  exit 1
}
assert_json "$missing_output" 'data.get("ok") is False' "missing lifecycle writable root is not semantic success"
assert_json "$missing_output" 'any(item.get("check") == "handoff-contract" for item in data.get("violations", []))' "missing lifecycle writable root fails the handoff contract"

positive_root="$fixture_root/handoff-contract"
positive_output="$fixture_root/handoff-contract.json"
write_harness_profile "$positive_root" "$validate_and_handoff_commands" "$both_default_roots" "target/appfw/agent-handoff.json"
APPFW_APP_ROOT="$positive_root" \
APPFW_FRAMEWORK_ROOT="$repo_root" \
APPFW_REPORT_ROOT="$positive_root/.appfw/target/appfw" \
  "$repo_root/scripts/appfw" product harness-check --json >"$positive_output"
assert_json "$positive_output" 'data.get("ok") is True' "declared command and lifecycle evidence roots pass together"
assert_json "$positive_output" 'all(path in [item.get("path") for item in data.get("profile", {}).get("writable_paths", [])] for path in [".appfw/target/appfw", "target/appfw"])' "both product evidence roots are retained"
assert_json "$positive_output" 'data.get("profile", {}).get("handoff", {}).get("artifact") == "target/appfw/agent-handoff.json"' "handoff artifact matches the lifecycle contract"

handoff_only_root="$fixture_root/handoff-only"
handoff_only_output="$fixture_root/handoff-only.json"
write_harness_profile "$handoff_only_root" "$handoff_command" "$lifecycle_root" "target/appfw/agent-handoff.json"
APPFW_APP_ROOT="$handoff_only_root" \
APPFW_FRAMEWORK_ROOT="$repo_root" \
APPFW_REPORT_ROOT="$handoff_only_root/.appfw/target/appfw" \
  "$repo_root/scripts/appfw" product harness-check --json >"$handoff_only_output"
assert_json "$handoff_only_output" 'data.get("ok") is True' "handoff-only profile does not require an unused command report root"
assert_json "$handoff_only_output" '".appfw/target/appfw" not in [item.get("path") for item in data.get("profile", {}).get("writable_paths", [])]' "handoff-only profile proves command-derived report root coverage"

nondefault_root="$fixture_root/nondefault-report-root"
nondefault_output="$fixture_root/nondefault-report-root.json"
nondefault_report_root=$'  - path: local/evidence/reports\n    ownership: product_evidence'
write_harness_profile "$nondefault_root" "$validate_command" "$nondefault_report_root" ""
APPFW_APP_ROOT="$nondefault_root" \
APPFW_FRAMEWORK_ROOT="$repo_root" \
APPFW_REPORT_ROOT="$nondefault_root/local/evidence/reports" \
  "$repo_root/scripts/appfw" product harness-check --json >"$nondefault_output"
assert_json "$nondefault_output" 'data.get("ok") is True' "nondefault in-product report root is derived from resolved product roots"
assert_json "$nondefault_output" 'data.get("artifact") == "local/evidence/reports/harness-check.json"' "nondefault report root is retained without a hard-coded default"

validate_output="$fixture_root/validate.json"
APPFW_REPORT_ROOT="$fixture_root/validate-report" \
  "$repo_root/scripts/appfw" framework validate --json >"$validate_output"
assert_json "$validate_output" 'data.get("command") == "validate"' "validate JSON names its command"
assert_json "$validate_output" 'data.get("ok") is True and data.get("valid") is True' "validate semantic success matches validation"
assert_json "$validate_output" 'data.get("requested_namespace") == "framework"' "validate records requested namespace"
assert_json "$validate_output" 'data.get("scope") == "framework-default-product-config"' "framework validate names actual scope"

python3 - "$repo_root" <<'PY'
import json
import sys
from pathlib import Path

repo_root = Path(sys.argv[1])
profile = json.loads(
    (repo_root / "app_gen/_golden/downstream_apps/second-consumer/profile.json").read_text(
        encoding="utf-8"
    )
)
required = {
    "scripts/appfw product validate --json",
    "scripts/appfw product harness-check --json",
    "scripts/appfw product generate",
    "scripts/appfw product generate --check --json",
    "scripts/appfw product test --fast",
    "scripts/appfw product handoff --json",
}
declared = set(profile.get("verification", []))
missing = sorted(required - declared)
if missing:
    raise SystemExit(f"second-consumer missing required commands: {missing}")

agent_profile_path = (
    repo_root
    / "app_gen/_golden/downstream_apps/second-consumer/starter/.appfw/agent-profile.yaml"
)
agent_profile = agent_profile_path.read_text(encoding="utf-8")
expected_profile_contract = (
    "  - path: .appfw/target/appfw",
    "  - path: target/appfw",
    "  artifact: target/appfw/agent-handoff.json",
)
missing_profile_contract = [
    value for value in expected_profile_contract if value not in agent_profile
]
if missing_profile_contract:
    raise SystemExit(
        "second-consumer agent profile does not declare the product evidence root: "
        f"{missing_profile_contract}"
    )

wrapper = (
    repo_root
    / "app_gen/_golden/downstream_apps/second-consumer/starter/scripts/appfw"
).read_text(encoding="utf-8")
if 'APPFW_APP_ROOT="$app_root" APPFW_FRAMEWORK_ROOT="$framework_root"' not in wrapper:
    raise SystemExit(
        "second-consumer wrapper does not route product commands through its app root"
    )
if 'APPFW_REPORT_ROOT="$app_root/.appfw/target/appfw"' not in wrapper:
    raise SystemExit(
        "second-consumer wrapper does not route product evidence to its declared root"
    )

pipeline = (repo_root / "bitbucket-pipelines.yml").read_text(encoding="utf-8")
gitignore = (repo_root / ".gitignore").read_text(encoding="utf-8")
preflight_runner = (
    repo_root / "scripts/ci/pr-preflight.py"
).read_text(encoding="utf-8")
fast_check = (
    repo_root / "scripts/ci/pr-fast-framework-check.sh"
).read_text(encoding="utf-8")
release_lite_guard = (
    repo_root / "scripts/ci/release-lite-guard.sh"
).read_text(encoding="utf-8")
destination_freshness = (
    repo_root / "scripts/ci/pr-destination-freshness.py"
).read_text(encoding="utf-8")
if "bash scripts/ci/pr-fast-framework-check.sh --ci-bootstrap" not in pipeline:
    raise SystemExit(
        "pull-request fast framework step must invoke "
        "scripts/ci/pr-fast-framework-check.sh --ci-bootstrap"
    )
if "- step: *fast-framework-check" not in pipeline:
    raise SystemExit("pull-request pipeline does not invoke fast-framework-check")
preflight_anchor_start = pipeline.index("    - step: &pr-preflight")
preflight_anchor_end = pipeline.index("    - step: &fast-framework-check")
preflight_definition = pipeline[preflight_anchor_start:preflight_anchor_end]
for required_preflight_contract in (
    "name: PR preflight",
    "max-time: 5",
    "command -v python3",
    "timeout --kill-after=2s 3s cargo fmt --version",
    "timeout --kill-after=2s 45s rustup component add rustfmt",
    "python3 scripts/ci/pr-preflight.py --deadline-seconds 240",
    "download: false",
    "type: scoped",
    "capture-on: always",
    "target/appfw/pr-preflight-change-impact.json",
    "target/appfw/pr-preflight.json",
    "target/appfw/pr-preflight-progress.jsonl",
    "target/appfw/pr-preflight-manifest.json",
    "target/appfw/pr-preflight-logs/*.log",
    "test-results/pr-preflight.xml",
):
    if required_preflight_contract not in preflight_definition:
        raise SystemExit(
            "PR preflight definition missing contract: "
            f"{required_preflight_contract}"
        )
if "caches:" in preflight_definition:
    raise SystemExit("PR preflight must remain cache-free")
if "target/appfw/**" in preflight_definition:
    raise SystemExit("PR preflight must not publish a broad target/appfw artifact")
if "type: shared" in preflight_definition:
    raise SystemExit("PR preflight evidence must not be shared with downstream steps")
if "if ! cargo fmt --version" in preflight_definition:
    raise SystemExit("PR preflight rustfmt probe must not run without its short timeout")

pr_lane_start = pipeline.index('  pull-requests:\n    "**":')
pr_lane_end = pipeline.index("  branches:", pr_lane_start)
pr_lane = pipeline[pr_lane_start:pr_lane_end]
if pr_lane.count("- step: *release-lite-guard") != 1:
    raise SystemExit("PR lane must retain exactly one opening release-lite guard")
if pr_lane.count("- step: *release-lite-closing-guard") != 1:
    raise SystemExit("PR lane must retain exactly one closing freshness guard")
ordered_pr_contract = (
    "- step: *release-lite-guard",
    "- step: *pr-preflight",
    "- parallel:",
    "fail-fast: true",
    "steps:",
    "- step: *fast-framework-check",
    "- step: *supply-chain-gate",
    "- step: *secret-scan",
)
cursor = -1
for contract in ordered_pr_contract:
    position = pr_lane.find(contract, cursor + 1)
    if position < 0:
        raise SystemExit(f"PR lane missing or reordered contract: {contract}")
    cursor = position
final_guard_position = pr_lane.rfind("- step: *release-lite-closing-guard")
if final_guard_position <= cursor:
    raise SystemExit("final PR release-lite/freshness guard must remain after retained gates")
if pr_lane.count("- step: *pr-preflight") != 1:
    raise SystemExit("PR lane must invoke the preflight exactly once")
for producer in (
    "- step: *fast-framework-check",
    "- step: *supply-chain-gate",
    "- step: *secret-scan",
):
    if pr_lane.count(producer) != 1:
        raise SystemExit(f"PR lane must invoke producer exactly once: {producer}")

release_lite_anchor_start = pipeline.index("    - step: &release-lite-guard")
closing_guard_anchor_start = pipeline.index("    - step: &release-lite-closing-guard")
release_gate_anchor_start = pipeline.index("    - step: &release-gate")
opening_guard_definition = pipeline[
    release_lite_anchor_start:closing_guard_anchor_start
]
closing_guard_definition = pipeline[
    closing_guard_anchor_start:release_gate_anchor_start
]
for name, definition in (
    ("opening release-lite", opening_guard_definition),
    ("closing freshness", closing_guard_definition),
):
    if "max-time: 5" not in definition:
        raise SystemExit(f"{name} guard must retain a short outer backstop")
for source_name, source in (
    ("opening release-lite", release_lite_guard),
    ("closing freshness", destination_freshness),
):
    for contract in (
        "APPFW_PR_REMOTE_GIT_TIMEOUT_SECONDS",
        'environment["GIT_TERMINAL_PROMPT"] = "0"',
        "timeout=timeout_seconds",
    ):
        if contract not in source:
            raise SystemExit(
                f"{source_name} guard missing bounded remote Git contract: {contract}"
            )
if "destination-fetch-failed" not in release_lite_guard:
    raise SystemExit("opening release-lite guard must fail closed on destination fetch failure")
if '"remote_query"' not in destination_freshness:
    raise SystemExit("closing freshness guard must retain structured remote query evidence")

for category in (
    "change_classification",
    "diff_hygiene",
    "conflict_marker",
    "shell_syntax",
    "rust_format",
    "timeout",
    "infrastructure",
    "unknown",
):
    if f'"{category}"' not in preflight_runner:
        raise SystemExit(f"PR preflight runner missing failure category: {category}")
if '"change_impact_is_report_only": True' not in preflight_runner:
    raise SystemExit("PR preflight must declare change-impact report-only")
for remote_transport_contract in (
    "APPFW_PR_REMOTE_GIT_TIMEOUT_SECONDS",
    'environment["GIT_TERMINAL_PROMPT"] = "0"',
    "timeout_seconds=self.remote_git_timeout_seconds",
):
    if remote_transport_contract not in preflight_runner:
        raise SystemExit(
            "PR preflight runner missing bounded remote Git contract: "
            f"{remote_transport_contract}"
        )
if "self.change_impact_snapshot_path" not in preflight_runner:
    raise SystemExit("PR preflight manifest lacks an immutable change-impact snapshot")
for report_contract in (
    '"change_impact_contract_valid": self.change_impact_contract_valid',
    '"change_impact_operational_ok": self.change_impact_operational_ok',
    '"artifact": "target/appfw/change-impact.json"',
    '"command": "change-impact"',
    '"mode": "report-only"',
    '"scope": "framework"',
    '"diff_base_ref": self.merge_base',
    '"head_ref": self.tested_sha',
    '"sha": self.tested_sha',
    'for field in ("buckets", "domains", "sensitive_flags")',
    '"ownership_domains"',
    '"sensitive_surfaces"',
    '"generated_like_paths"',
    '"change_class.reason is not a nonempty string"',
    'summary["non_generated_added_lines"]',
    '+ summary["non_generated_deleted_lines"]',
    'for forbidden in ("error", "git_errors", "git_error_count")',
    'type(projection.get("ok")) is not bool',
    'self.change_impact_operational_ok is False and projection_ok is not False',
    'projection_ok is False and self.change_impact_operational_ok is not False',
):
    if report_contract not in preflight_runner:
        raise SystemExit(
            "PR preflight missing strict report-only transport contract: "
            f"{report_contract}"
        )
if 'payload.get("ok") is not True' in preflight_runner:
    raise SystemExit(
        "PR preflight must not equate candidate operational ok with report-contract validity"
    )
change_impact_runner = preflight_runner.split("    def run_change_impact", 1)[1].split(
    "    def write_change_impact_unavailable", 1
)[0]
if change_impact_runner.index("self.change_impact_contract_valid = True") < (
    change_impact_runner.index("if coherence_error is not None")
):
    raise SystemExit(
        "PR preflight must not mark the change-impact report contract valid before "
        "operational/projection coherence passes"
    )
if '"downstream_gate_selection": "unchanged"' not in preflight_runner:
    raise SystemExit("PR preflight must not route or suppress downstream gates")
for identity_contract in (
    'os.environ.get("BITBUCKET_COMMIT")',
    '"source_sha": self.source_sha',
    '"tested_sha": self.tested_sha',
    '"tested_commit_relation": self.tested_commit_relation',
    '["rev-list", "--parents", "-n", "1", self.tested_sha]',
    '["merge-base", self.destination_sha, self.tested_sha]',
    "MAX_CHANGED_BLOB_BYTES = 32 * 1024 * 1024",
    '["cat-file", "-s", object_id]',
):
    if identity_contract not in preflight_runner:
        raise SystemExit(
            "PR preflight runner missing source-vs-tested identity contract: "
            f"{identity_contract}"
        )
for timeout_guard in (
    "if not result.timed_out and result.stdout_size > MAX_CAPTURE_BYTES",
    "ref_check.timed_out",
    "remote_check.timed_out",
    "fetched.timed_out",
    "merge.timed_out",
):
    if timeout_guard not in preflight_runner:
        raise SystemExit(
            "PR preflight identity command must fail closed on timeout: "
            f"{timeout_guard}"
        )
if 'self.source_sha or ""' in preflight_runner:
    raise SystemExit(
        "PR preflight content checks must use tested_sha, not source_sha"
    )
if "/test-results/pr-preflight.xml" not in gitignore:
    raise SystemExit("local PR preflight JUnit output must not dirty source status")
required_shared_commands = (
    "scripts/appfw framework cli-test --json",
    "scripts/appfw framework golden-downstream --profile second-consumer --execute --json",
    "scripts/appfw docs-check --changed-only --json --progress --enforce-budget",
    "wave3-pr-gates.sh",
    "--reuse-docs-check",
    "scripts/appfw test --fast --json",
)
missing_shared = [
    command for command in required_shared_commands if command not in fast_check
]
if missing_shared:
    raise SystemExit(
        "pr-fast-framework-check.sh missing shared Fast framework commands: "
        f"{missing_shared}"
    )
if "export APPFW_APP_ROOT=" in fast_check:
    raise SystemExit(
        "pr-fast-framework-check.sh must not export APPFW_APP_ROOT for the whole "
        "step; only golden-downstream should override it (CI parity)"
    )
if 'APPFW_APP_ROOT="$golden_app_root"' not in fast_check:
    raise SystemExit(
        "pr-fast-framework-check.sh must set APPFW_APP_ROOT only for "
        "golden-downstream via golden_app_root"
    )
local_pre_push = (
    repo_root / "scripts/ci/local-pre-push-gates.sh"
).read_text(encoding="utf-8")
if "pr-fast-framework-check.sh" not in local_pre_push:
    raise SystemExit(
        "local-pre-push-gates.sh must invoke pr-fast-framework-check.sh"
    )
if "pr-preflight.py" not in local_pre_push:
    raise SystemExit("local-pre-push-gates.sh must invoke pr-preflight.py")
if local_pre_push.index("pr-preflight.py") > local_pre_push.index(
    'log "pr-fast-framework-check'
):
    raise SystemExit("local PR preflight must run before the fast framework check")
PY

handoff_output="$fixture_root/framework-handoff.json"
"$repo_root/scripts/appfw" framework handoff --json >"$handoff_output"
cmp "$handoff_output" "$repo_root/target/appfw/agent-handoff.json"
assert_json "$handoff_output" 'data.get("delivery_profile", {}).get("source_status") in {"current", "dirty", "stale", "dirty-and-stale"}' "delivery mode annotates handoff with explicit source status"

review_brief_output="$fixture_root/framework-review-brief.json"
"$repo_root/scripts/appfw" framework review-brief --comprehensive --json >"$review_brief_output"
cmp "$review_brief_output" "$repo_root/target/appfw/review-brief.json"
assert_json "$review_brief_output" 'data.get("delivery_profile", {}).get("source_status") in {"current", "dirty", "stale", "dirty-and-stale"}' "delivery mode annotates review-brief with explicit source status"
assert_json "$review_brief_output" 'data.get("pre_push_status", {}).get("satisfied") is False' "delivery mode annotation does not weaken review freshness or pre-push satisfaction"

change_impact_output="$fixture_root/framework-change-impact.json"
"$repo_root/scripts/appfw" framework change-impact --base HEAD --head HEAD --json >"$change_impact_output"
cmp "$change_impact_output" "$repo_root/target/appfw/change-impact.json"
assert_json "$change_impact_output" 'data.get("delivery_profile", {}).get("source_status") in {"current", "dirty", "stale", "dirty-and-stale"}' "delivery mode annotates change-impact with explicit source status"

python3 "$repo_root/scripts/ci/feature-check-progress-evidence.test.py"

printf 'CLI semantic gate fixtures passed.\n'
