#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
runner="$script_dir/pr-preflight.py"
pipeline="$script_dir/../../bitbucket-pipelines.yml"
fixture_root="$(mktemp -d "${TMPDIR:-/tmp}/appfw-pr-preflight.XXXXXX")"
trap 'rm -rf "$fixture_root"' EXIT

fail() {
  echo "pr-preflight fixture failure: $*" >&2
  exit 1
}

grep -q 'timeout --kill-after=2s 3s cargo fmt --version' "$pipeline" ||
  fail "Bitbucket rustfmt probe lacks its TERM-to-KILL timeout bound"
grep -q 'timeout --kill-after=2s 45s rustup component add rustfmt' "$pipeline" ||
  fail "Bitbucket rustfmt bootstrap lacks its TERM-to-KILL timeout bound"
if grep -q 'if ! cargo fmt --version' "$pipeline"; then
  fail "Bitbucket rustfmt probe still has an unbounded invocation"
fi

# Bitbucket's observed 12-character destination value is a consistency prefix.
# Full SHA-1 and SHA-256 identities remain exact, including across algorithms.
python3 - "$runner" <<'PY'
import importlib.util
import sys
from pathlib import Path

runner_path = Path(sys.argv[1])
spec = importlib.util.spec_from_file_location("appfw_pr_preflight_identity_contract", runner_path)
if spec is None or spec.loader is None:
    raise SystemExit("could not load preflight runner identity contract")
module = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = module
spec.loader.exec_module(module)
matches = module.destination_identity_matches

sha1 = "a" * 40
sha256 = "b" * 64
mixed = "c" * 64
cases = [
    (sha1, sha1[:12], True, "12-character SHA-1 prefix"),
    (sha256, sha256[:12], True, "12-character SHA-256 prefix"),
    (sha1, sha1, True, "40-character exact identity"),
    (sha256, sha256, True, "64-character exact identity"),
    (mixed, mixed[:40], False, "40-character prefix of 64-character identity"),
    (sha1, sha1 + ("d" * 24), False, "64-character value against 40-character identity"),
]
for remote, supplied, expected, label in cases:
    if matches(remote, supplied) is not expected:
        raise SystemExit(f"destination identity contract failed: {label}")


class FakeProcess:
    pid = 12345
    returncode = None

    def __init__(self):
        self.wait_called = False

    def wait(self, timeout=None):
        self.wait_called = True
        raise module.PreflightFailure(
            category="timeout",
            check="injected-after-spawn",
            message="injected post-spawn deadline",
            pipeline_exit_code=124,
        )

    def poll(self):
        return self.returncode


created_handles = []
real_temporary_file = module.tempfile.TemporaryFile


def tracked_temporary_file(*args, **kwargs):
    handle = real_temporary_file(*args, **kwargs)
    created_handles.append(handle)
    return handle


fake_process = FakeProcess()
terminated = []
preflight = module.Preflight(Path.cwd(), 20, "fixture-base")
preflight.git_capture = lambda _argv: module.CommandResult(
    argv=["git", "cat-file", "-s", "fixture"],
    returncode=0,
    stdout=b"4\n",
    stderr=b"",
    stdout_size=2,
    stderr_size=0,
    output_truncated=False,
    timed_out=False,
    startup_error=None,
    duration_ms=0,
)
preflight.remaining_seconds = lambda: 1.0
preflight.terminate_process_group = lambda process, grace_seconds=2.0: (
    terminated.append((process, grace_seconds)),
    setattr(process, "returncode", -9),
)
module.tempfile.TemporaryFile = tracked_temporary_file
module.subprocess.Popen = lambda *_args, **_kwargs: fake_process
try:
    preflight.materialize_blob("fixture")
except module.PreflightFailure as error:
    if error.check != "injected-after-spawn":
        raise SystemExit(f"unexpected injected cleanup failure: {error.check}")
else:
    raise SystemExit("injected post-spawn deadline did not propagate")
if not fake_process.wait_called or len(terminated) != 1:
    raise SystemExit("post-spawn deadline did not terminate and reap the child")
if len(created_handles) != 2 or not all(handle.closed for handle in created_handles):
    raise SystemExit("post-spawn deadline leaked a materialization descriptor")


class SuccessfulProcess:
    pid = 12346

    def __init__(self, stdout):
        self.stdout = stdout
        self.returncode = None

    def wait(self, timeout=None):
        self.stdout.write(b"data")
        self.returncode = 0
        return 0

    def poll(self):
        return self.returncode


created_handles.clear()
terminated.clear()
successful_processes = []


def successful_popen(*_args, **kwargs):
    process = SuccessfulProcess(kwargs["stdout"])
    successful_processes.append(process)
    return process


def injected_fstat_failure(_fd):
    raise OSError("injected post-wait fstat failure")


module.subprocess.Popen = successful_popen
module.os.fstat = injected_fstat_failure
try:
    preflight.materialize_blob("fixture")
except OSError as error:
    if "injected post-wait fstat failure" not in str(error):
        raise
else:
    raise SystemExit("injected post-wait fstat failure did not propagate")
if len(successful_processes) != 1 or successful_processes[0].returncode != 0:
    raise SystemExit("post-wait cleanup fixture did not complete its child")
if terminated:
    raise SystemExit("post-wait cleanup tried to terminate an exited child")
if len(created_handles) != 2 or not all(handle.closed for handle in created_handles):
    raise SystemExit("post-wait fstat failure leaked a materialization descriptor")
PY

new_case() {
  local name="$1"
  case_root="$fixture_root/$name"
  mkdir -p "$case_root/scripts" "$case_root/bin" "$case_root/src" "$case_root/docs"
  git -C "$case_root" init -q -b main
  git -C "$case_root" config user.name "AppFW Preflight Fixture"
  git -C "$case_root" config user.email "preflight-fixture@example.invalid"

  cat >"$case_root/scripts/appfw" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
[[ "${1:-}" == "framework" && "${2:-}" == "change-impact" ]] || exit 64
[[ "${3:-}" == "--base" && "${5:-}" == "--head" && "${7:-}" == "--json" ]] || exit 64
base_ref="$4"
head_ref="$6"
operational_ok=true
delivery_annotation=""
extra_annotation=""
case "${APPFW_TEST_CHANGE_IMPACT_MODE:-ok}" in
  ok) ;;
  fail)
    printf '{"ok":false,"schema":"fixture-change-impact@1"}\n'
    exit 17
    ;;
  annotated-false)
    operational_ok=false
    delivery_annotation=',"delivery_profile":{"gate_execution_projection":{"ok":false}}'
    ;;
  arbitrary-false)
    operational_ok=false
    ;;
  projection-false-top-true)
    delivery_annotation=',"delivery_profile":{"gate_execution_projection":{"ok":false}}'
    ;;
  projection-nonboolean)
    operational_ok=false
    delivery_annotation=',"delivery_profile":{"gate_execution_projection":{"ok":0}}'
    ;;
  git-error-count)
    extra_annotation=',"git_error_count":1'
    ;;
  identity-mismatch)
    head_ref=0000000000000000000000000000000000000000
    ;;
  malformed)
    printf '{"artifact":"target/appfw/change-impact.json","ok":true,"command":"change-impact","mode":"report-only","scope":"framework","base_ref":"%s","diff_base_ref":"%s","head_ref":"%s","sha":"%s","changed_files":"not-a-list","buckets":[],"ownership_domains":"docs","sensitive_surfaces":[],"generated_like_paths":[],"summary":{},"recommended_verification":[],"change_class":{"class":"Z","reason":"","requires_human_review":"false","requires_integration_branch":false}}\n' \
      "$base_ref" "$base_ref" "$head_ref" "$head_ref"
    exit 0
    ;;
  sleep)
    sleep 3
    ;;
  term-resistant)
    marker="${APPFW_TEST_TERM_MARKER:?term-resistant marker path is required}"
    (
      trap '' TERM
      while :; do sleep 1; done
    ) &
    descendant=$!
    printf '%s\n' "$descendant" >"$marker"
    trap 'exit 0' TERM
    wait "$descendant"
    ;;
  rogue)
    mkdir -p target/appfw
    printf 'unexpected\n' >target/appfw/pr-preflight-rogue.txt
    ;;
  *) exit 65 ;;
esac
printf '{"artifact":"target/appfw/change-impact.json","ok":%s,"command":"change-impact","mode":"report-only","scope":"framework","base_ref":"%s","diff_base_ref":"%s","head_ref":"%s","sha":"%s","changed_files":[{"path":"docs/README.md","added":2,"deleted":1,"status":"tracked","uncommitted":false,"generated_like":false,"buckets":["docs"],"domains":["docs"],"sensitive_flags":[]}],"buckets":{"docs":1},"ownership_domains":["docs"],"sensitive_surfaces":[],"generated_like_paths":[],"summary":{"changed_file_count":1,"untracked_file_count":0,"non_generated_added_lines":2,"non_generated_deleted_lines":1,"non_generated_line_delta":3,"ownership_domain_count":1,"sensitive_surface_count":0,"generated_like_file_count":0},"recommended_verification":["git diff --check"],"change_class":{"class":"A","reason":"documentation-only fixture","requires_human_review":false,"requires_integration_branch":false}%s%s}\n' \
  "$operational_ok" "$base_ref" "$base_ref" "$head_ref" "$head_ref" "$delivery_annotation" "$extra_annotation"
SH
  chmod +x "$case_root/scripts/appfw"

  cat >"$case_root/bin/cargo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "fmt" && "${2:-}" == "--version" ]]; then
  if [[ "${APPFW_TEST_CARGO_MODE:-ok}" == "infrastructure" ]]; then
    echo "rustfmt component unavailable" >&2
    exit 127
  fi
  echo "rustfmt fixture 1.0"
  exit 0
fi
if [[ "${1:-}" == "fmt" && "${2:-}" == "--all" && "${3:-}" == "--check" ]]; then
  if [[ "${APPFW_TEST_CARGO_MODE:-ok}" == "format-fail" ]]; then
    echo "Diff in fixture Rust source" >&2
    exit 23
  fi
  exit 0
fi
exit 66
SH
  chmod +x "$case_root/bin/cargo"

  printf '[package]\nname = "preflight-fixture"\nversion = "0.1.0"\nedition = "2021"\n' \
    >"$case_root/Cargo.toml"
  printf 'pub fn fixture() {}\n' >"$case_root/src/lib.rs"
  printf '# Fixture\n' >"$case_root/docs/README.md"
  git -C "$case_root" add .
  git -C "$case_root" commit -q -m base
  base_sha="$(git -C "$case_root" rev-parse HEAD)"
}

commit_case() {
  local message="${1:-change}"
  git -C "$case_root" add -A
  git -C "$case_root" commit -q -m "$message"
}

install_git_wrapper() {
  local real_git
  real_git="$(command -v git)"
  cat >"$case_root/bin/git" <<SH
#!/usr/bin/env bash
set -euo pipefail
if [[ "\${1:-}" == "cat-file" && "\${2:-}" == "blob" && "\${APPFW_TEST_GIT_MODE:-}" == "blob-fail" ]]; then
  echo "fixture git cat-file failure" >&2
  exit 86
fi
if [[ "\${1:-}" == "ls-remote" || "\${1:-}" == "fetch" ]]; then
  [[ "\${GIT_TERMINAL_PROMPT:-}" == "0" ]] || {
    echo "remote Git fixture observed an interactive prompt policy" >&2
    exit 88
  }
  if [[ -n "\${APPFW_TEST_REMOTE_GIT_MARKER:-}" ]]; then
    printf '%s\n' "\${1}" >>"\${APPFW_TEST_REMOTE_GIT_MARKER}"
  fi
  if [[ "\${APPFW_TEST_REMOTE_GIT_TARGET:-}" == "\${1}" ]]; then
    case "\${APPFW_TEST_REMOTE_GIT_MODE:-success}" in
      hang)
        sleep 30
        ;;
      nonzero)
        echo "fixture remote Git \${1} failure" >&2
        exit 87
        ;;
      success) ;;
      *) exit 89 ;;
    esac
  fi
fi
exec "$real_git" "\$@"
SH
  chmod +x "$case_root/bin/git"
}

run_local() {
  local root="$1"
  local deadline="${2:-20}"
  shift 2 || true
  set +e
  (
    cd "$root"
    env -u BITBUCKET_COMMIT \
      -u BITBUCKET_PR_DESTINATION_BRANCH \
      -u BITBUCKET_PR_DESTINATION_COMMIT \
      PATH="$root/bin:$PATH" "$@" python3 "$runner" \
        --base "$base_sha" --deadline-seconds "$deadline"
  ) >"$root/runner.stdout" 2>"$root/runner.stderr"
  run_status=$?
  set -e
}

run_without_base() {
  local root="$1"
  local deadline="${2:-20}"
  shift 2 || true
  set +e
  (
    cd "$root"
    env -u BITBUCKET_COMMIT \
      -u BITBUCKET_PR_DESTINATION_BRANCH \
      -u BITBUCKET_PR_DESTINATION_COMMIT \
      PATH="$root/bin:$PATH" "$@" python3 "$runner" \
      --deadline-seconds "$deadline"
  ) >"$root/runner.stdout" 2>"$root/runner.stderr"
  run_status=$?
  set -e
}

run_deadline_before_command() {
  local root="$1"
  set +e
  (
    cd "$root"
    PATH="$root/bin:$PATH" python3 - "$runner" "$root" "$base_sha" <<'PY'
import importlib.util
import sys
import time
from pathlib import Path

runner_path = Path(sys.argv[1])
repo_root = Path(sys.argv[2])
base_sha = sys.argv[3]
module_name = "appfw_pr_preflight_deadline_fixture"
spec = importlib.util.spec_from_file_location(module_name, runner_path)
if spec is None or spec.loader is None:
    raise SystemExit("could not load preflight runner fixture")
module = importlib.util.module_from_spec(spec)
sys.modules[module_name] = module
spec.loader.exec_module(module)


class DeadlineBeforeCommandPreflight(module.Preflight):
    def resolve_identity(self) -> None:
        check = "identity"
        self.emit(check, "started")
        self.log_path(check)
        self.deadline = time.monotonic() - 1
        self.run_command(["git", "rev-parse", "--verify", "HEAD^{commit}"])


raise SystemExit(DeadlineBeforeCommandPreflight(repo_root, 20, base_sha).execute())
PY
  ) >"$root/runner.stdout" 2>"$root/runner.stderr"
  run_status=$?
  set -e
}

assert_evidence() {
  local root="$1"
  local expected_ok="$2"
  local expected_category="$3"
  local expected_child="${4:-any}"
  python3 - "$root" "$expected_ok" "$expected_category" "$expected_child" <<'PY'
import hashlib
import json
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

root = Path(sys.argv[1])
expected_ok = sys.argv[2] == "true"
expected_category = None if sys.argv[3] == "none" else sys.argv[3]
expected_child = sys.argv[4]
report = root / "target/appfw"
required = {
    "target/appfw/pr-preflight-change-impact.json",
    "target/appfw/pr-preflight.json",
    "target/appfw/pr-preflight-progress.jsonl",
    "target/appfw/pr-preflight-manifest.json",
    "test-results/pr-preflight.xml",
}
for relative in required:
    path = root / relative
    if not path.is_file():
        raise SystemExit(f"missing required evidence: {relative}")
canonical_change_impact = root / "target/appfw/change-impact.json"
if not canonical_change_impact.is_file():
    raise SystemExit("missing compatibility change-impact report")

result = json.loads((report / "pr-preflight.json").read_text(encoding="utf-8"))
if result.get("schema") != "appfw_pr_preflight@1":
    raise SystemExit("unexpected result schema")
if result.get("ok") is not expected_ok:
    raise SystemExit(f"unexpected ok value: {result.get('ok')}")
if result.get("category") != expected_category:
    raise SystemExit(f"unexpected category: {result.get('category')}")
if result.get("change_impact_is_report_only") is not True:
    raise SystemExit("change impact is not explicitly report-only")
if result.get("downstream_gate_selection") != "unchanged":
    raise SystemExit("preflight claims to select downstream gates")
if expected_child != "any" and result.get("child_exit_code") != int(expected_child):
    raise SystemExit(f"unexpected child exit code: {result.get('child_exit_code')}")

progress_lines = [
    json.loads(line)
    for line in (report / "pr-preflight-progress.jsonl").read_text(
        encoding="utf-8"
    ).splitlines()
    if line
]
if not progress_lines:
    raise SystemExit("progress JSONL is empty")
if [item.get("sequence") for item in progress_lines] != list(
    range(1, len(progress_lines) + 1)
):
    raise SystemExit("progress sequence is not contiguous")

manifest = json.loads(
    (report / "pr-preflight-manifest.json").read_text(encoding="utf-8")
)
if manifest.get("schema") != "appfw_pr_preflight_manifest@1":
    raise SystemExit("unexpected manifest schema")
if manifest.get("manifest_self_hash") != "excluded-self-referential-manifest":
    raise SystemExit("manifest self-hash exclusion is not explicit")
for identity_field in (
    "source_sha",
    "tested_sha",
    "tested_commit_relation",
    "destination_sha",
    "merge_base",
    "change_impact_contract_valid",
    "change_impact_operational_ok",
    "remote_git",
):
    if manifest.get(identity_field) != result.get(identity_field):
        raise SystemExit(
            f"manifest/result identity mismatch for {identity_field}: "
            f"{manifest.get(identity_field)} != {result.get(identity_field)}"
        )
entries = manifest.get("artifacts", [])
paths = [entry.get("path") for entry in entries]
if len(paths) != len(set(paths)):
    raise SystemExit("manifest contains duplicate paths")
if "target/appfw/pr-preflight-manifest.json" in paths:
    raise SystemExit("self-referential manifest was incorrectly hashed")
if not required.difference({"target/appfw/pr-preflight-manifest.json"}).issubset(paths):
    raise SystemExit("manifest does not include every non-self required artifact")
if not any(path.startswith("target/appfw/pr-preflight-logs/") for path in paths):
    raise SystemExit("manifest contains no bounded check logs")
for entry in entries:
    path = root / entry["path"]
    payload = path.read_bytes()
    if len(payload) != entry.get("bytes"):
        raise SystemExit(f"manifest size mismatch: {entry['path']}")
    if hashlib.sha256(payload).hexdigest() != entry.get("sha256"):
        raise SystemExit(f"manifest SHA-256 mismatch: {entry['path']}")
    if entry["path"].startswith("target/appfw/pr-preflight-logs/") and len(payload) > 65536:
        raise SystemExit(f"unbounded preflight log: {entry['path']}")
junit_root = ET.parse(root / "test-results/pr-preflight.xml").getroot()
terminal_nodes = junit_root.findall(".//failure") + junit_root.findall(".//error")
if expected_ok and terminal_nodes:
    raise SystemExit("passing preflight JUnit contains a terminal failure")
if not expected_ok and not terminal_nodes:
    raise SystemExit("failing preflight JUnit omits the terminal failure")
if not expected_ok and not any(
    record.get("status") == "failed"
    and record.get("name") == result.get("failed_check")
    for record in result.get("checks", [])
):
    raise SystemExit("failing result omits the terminal failed-check record")
PY
}

assert_remote_transport_evidence() {
  local root="$1"
  local expected_timeout="$2"
  local expected_message="$3"
  python3 - "$root" "$expected_timeout" "$expected_message" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
expected_timeout = int(sys.argv[2])
expected_message = sys.argv[3]
result = json.loads(
    (root / "target/appfw/pr-preflight.json").read_text(encoding="utf-8")
)
remote_git = result.get("remote_git", {})
if remote_git.get("timeout_seconds") != expected_timeout:
    raise SystemExit("remote Git result omitted its exact operation timeout")
if remote_git.get("terminal_prompt_disabled") is not True:
    raise SystemExit("remote Git result omitted its noninteractive policy")
if expected_message not in (result.get("message") or ""):
    raise SystemExit("remote Git result omitted its bounded failure diagnostic")
PY
}

expect_status() {
  local expected="$1"
  [[ "$run_status" -eq "$expected" ]] || {
    cat "$case_root/runner.stdout" >&2 || true
    cat "$case_root/runner.stderr" >&2 || true
    fail "expected status $expected, got $run_status for $case_root"
  }
}

expect_nonzero() {
  [[ "$run_status" -ne 0 ]] || fail "expected nonzero status for $case_root"
}

# Clean docs-only success.
new_case clean
printf 'bounded preflight\n' >>"$case_root/docs/README.md"
commit_case
run_local "$case_root" 20 env
expect_status 0
assert_evidence "$case_root" true none
cmp "$case_root/target/appfw/change-impact.json" \
  "$case_root/target/appfw/pr-preflight-change-impact.json" ||
  fail "preflight snapshot does not preserve the exact change-impact payload"
manifest_hash_before="$(shasum -a 256 "$case_root/target/appfw/pr-preflight-manifest.json" | awk '{print $1}')"
snapshot_hash_before="$(shasum -a 256 "$case_root/target/appfw/pr-preflight-change-impact.json" | awk '{print $1}')"
# Normal handoff/review commands refresh the canonical report after preflight.
# That mutable compatibility write must not invalidate the run-owned snapshot
# or any hash in the retained preflight manifest.
printf '{"artifact":"target/appfw/change-impact.json","later_refresh":true}\n' \
  >"$case_root/target/appfw/change-impact.json"
assert_evidence "$case_root" true none
[[ "$(shasum -a 256 "$case_root/target/appfw/pr-preflight-manifest.json" | awk '{print $1}')" == "$manifest_hash_before" ]] ||
  fail "later change-impact refresh mutated the preflight manifest"
[[ "$(shasum -a 256 "$case_root/target/appfw/pr-preflight-change-impact.json" | awk '{print $1}')" == "$snapshot_hash_before" ]] ||
  fail "later change-impact refresh mutated the preflight-owned snapshot"

# Whitespace is reported by git diff --check.
new_case whitespace
printf 'bad whitespace \n' >"$case_root/docs/whitespace.txt"
commit_case
run_local "$case_root" 20 env
expect_nonzero
assert_evidence "$case_root" false diff_hygiene

# Anchored unresolved conflict markers retain their own category.
new_case conflict
printf '<<<<<<< HEAD\nleft\n=======\nright\n>>>>>>> branch\n' >"$case_root/docs/conflict.txt"
commit_case
run_local "$case_root" 20 env
expect_nonzero
assert_evidence "$case_root" false conflict_marker

# An illustrative lone marker line is not a coherent unresolved block.
new_case illustrative-marker
printf 'An example follows:\n=======\nEnd example.\n' >"$case_root/docs/illustrative-marker.txt"
commit_case
run_local "$case_root" 20 env
expect_status 0
assert_evidence "$case_root" true none

# A rooted block cannot escape merely because its end marker is missing.
new_case conflict-missing-end
printf '<<<<<<< HEAD\nleft\n=======\nright\n' >"$case_root/docs/conflict-missing-end.txt"
commit_case
run_local "$case_root" 20 env
expect_nonzero
assert_evidence "$case_root" false conflict_marker

# A rooted block that jumps directly to its end is malformed, not illustrative.
new_case conflict-missing-middle
printf '<<<<<<< HEAD\nleft\n>>>>>>> branch\n' >"$case_root/docs/conflict-missing-middle.txt"
commit_case
run_local "$case_root" 20 env
expect_nonzero
assert_evidence "$case_root" false conflict_marker

# Nested starts are an invalid rooted transition and must fail closed.
new_case conflict-malformed-transition
printf '<<<<<<< HEAD\nleft\n<<<<<<< nested\nright\n' >"$case_root/docs/conflict-malformed-transition.txt"
commit_case
run_local "$case_root" 20 env
expect_nonzero
assert_evidence "$case_root" false conflict_marker

# bash -n failure preserves its child exit code in evidence.
new_case shell
printf '#!/usr/bin/env bash\nif then\n  echo broken\nfi\n' >"$case_root/scripts/broken.sh"
commit_case
run_local "$case_root" 20 env
expect_nonzero
assert_evidence "$case_root" false shell_syntax 2

# A large valid shell blob remains complete after Python samples it for binary
# and shebang detection and then gives the same descriptor to `bash -n`.
new_case large-valid-shell
python3 - "$case_root/scripts/large-valid.sh" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
with path.open("w", encoding="utf-8") as handle:
    handle.write("#!/usr/bin/env bash\n")
    handle.write('payload="\n')
    for index in range(8_000):
        handle.write(f"large shell syntax fixture line {index:05d}\n")
    handle.write('"\n')
    handle.write(': "$payload"\n')
PY
commit_case large-valid-shell
run_local "$case_root" 20 env
expect_status 0
assert_evidence "$case_root" true none

# Git type changes remain in the marker and changed-shell scan surfaces.
new_case type-change-conflict
ln -s original-target "$case_root/type-change-conflict.txt"
git -C "$case_root" add type-change-conflict.txt
git -C "$case_root" commit -q -m type-change-conflict-base
base_sha="$(git -C "$case_root" rev-parse HEAD)"
rm "$case_root/type-change-conflict.txt"
printf '<<<<<<< HEAD\nleft\n=======\nright\n>>>>>>> branch\n' >"$case_root/type-change-conflict.txt"
commit_case type-change-conflict
git -C "$case_root" diff --name-status --diff-filter=T "$base_sha" HEAD | \
  grep -q $'^T[[:space:]]type-change-conflict.txt$' || fail "conflict fixture is not a Git type change"
run_local "$case_root" 20 env
expect_nonzero
assert_evidence "$case_root" false conflict_marker

new_case type-change-shell
ln -s original-target "$case_root/type-change-shell.sh"
git -C "$case_root" add type-change-shell.sh
git -C "$case_root" commit -q -m type-change-shell-base
base_sha="$(git -C "$case_root" rev-parse HEAD)"
rm "$case_root/type-change-shell.sh"
printf '#!/bin/bash\nif then\n  echo broken\nfi\n' >"$case_root/type-change-shell.sh"
commit_case type-change-shell
git -C "$case_root" diff --name-status --diff-filter=T "$base_sha" HEAD | \
  grep -q $'^T[[:space:]]type-change-shell.sh$' || fail "shell fixture is not a Git type change"
run_local "$case_root" 20 env
expect_nonzero
assert_evidence "$case_root" false shell_syntax 2

# A helper failure after scan start still produces the scan log, terminal
# record, failing JUnit, and a complete hash-verified manifest.
new_case mid-scan-blob-failure
printf 'blob failure fixture\n' >"$case_root/docs/blob-failure.txt"
commit_case
install_git_wrapper
run_local "$case_root" 20 env APPFW_TEST_GIT_MODE=blob-fail
expect_status 2
assert_evidence "$case_root" false infrastructure 86
python3 - "$case_root" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
result = json.loads(
    (root / "target/appfw/pr-preflight.json").read_text(encoding="utf-8")
)
if result.get("failed_check") != "conflict_marker":
    raise SystemExit("mid-scan helper failure was not attributed to the started scan")
log_path = root / "target/appfw/pr-preflight-logs/conflict-marker.log"
if "terminal_helper=git_blob" not in log_path.read_text(encoding="utf-8"):
    raise SystemExit("mid-scan helper failure log omits terminal helper evidence")
PY

# Oversized changed text is rejected before materialization; it is never
# silently truncated or skipped, and terminal evidence remains complete.
new_case oversized-changed-blob
python3 - "$case_root/oversized.sh" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
with path.open("wb") as handle:
    chunk = b"x" * (1024 * 1024)
    for _ in range(32):
        handle.write(chunk)
    handle.write(b"x")
PY
commit_case oversized-changed-blob
run_local "$case_root" 20 env
expect_status 2
assert_evidence "$case_root" false infrastructure
python3 - "$case_root" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
result = json.loads(
    (root / "target/appfw/pr-preflight.json").read_text(encoding="utf-8")
)
if result.get("failed_check") != "conflict_marker":
    raise SystemExit("oversized blob failure was not attributed to the active scan")
log = (root / "target/appfw/pr-preflight-logs/conflict-marker.log").read_text(
    encoding="utf-8"
)
if "inspection cap is 33554432 bytes" not in log:
    raise SystemExit("oversized blob evidence omitted the exact inspection cap")
PY

# Rust formatting failure preserves the fixture child status.
new_case rustfmt
printf 'pub fn new_item() {}\n' >>"$case_root/src/lib.rs"
commit_case
run_local "$case_root" 20 env APPFW_TEST_CARGO_MODE=format-fail
expect_status 23
assert_evidence "$case_root" false rust_format 23

# Change-impact remains report-only for routing, but its execution is required.
new_case change-impact
printf 'classification fixture\n' >>"$case_root/docs/README.md"
commit_case
run_local "$case_root" 20 env APPFW_TEST_CHANGE_IMPACT_MODE=fail
expect_status 17
assert_evidence "$case_root" false change_classification 17

# Candidate delivery annotation may make the operational profile false while
# the exact report-only classification remains coherent and usable telemetry.
new_case change-impact-annotated-false
printf 'annotated classification fixture\n' >>"$case_root/docs/README.md"
commit_case
run_local "$case_root" 20 env APPFW_TEST_CHANGE_IMPACT_MODE=annotated-false
expect_status 0
assert_evidence "$case_root" true none
python3 - "$case_root" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
payload = json.loads((root / "target/appfw/change-impact.json").read_text())
result = json.loads((root / "target/appfw/pr-preflight.json").read_text())
if payload.get("ok") is not False:
    raise SystemExit("annotated false payload was rewritten")
projection = payload.get("delivery_profile", {}).get("gate_execution_projection", {})
if projection.get("ok") is not False:
    raise SystemExit("annotated false fixture lacks its candidate gate projection")
if result.get("change_impact_contract_valid") is not True:
    raise SystemExit("annotated false report contract was not retained as valid")
if result.get("change_impact_operational_ok") is not False:
    raise SystemExit("annotated false operational status was not retained")
if result.get("downstream_gate_selection") != "unchanged":
    raise SystemExit("annotated false classification changed assurance routing")
PY

new_case change-impact-malformed
printf 'malformed classification fixture\n' >>"$case_root/docs/README.md"
commit_case
run_local "$case_root" 20 env APPFW_TEST_CHANGE_IMPACT_MODE=malformed
expect_status 1
assert_evidence "$case_root" false change_classification 0
python3 - "$case_root" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
payload = json.loads((root / "target/appfw/change-impact.json").read_text())
result = json.loads((root / "target/appfw/pr-preflight.json").read_text())
if payload.get("changed_files") != "not-a-list":
    raise SystemExit("malformed child payload was not preserved")
if result.get("change_impact_contract_valid") is not False:
    raise SystemExit("malformed report contract was marked valid")
if result.get("change_impact_operational_ok") is not None:
    raise SystemExit("malformed classification invented operational status")
PY

new_case change-impact-identity-mismatch
printf 'identity mismatch classification fixture\n' >>"$case_root/docs/README.md"
commit_case
run_local "$case_root" 20 env APPFW_TEST_CHANGE_IMPACT_MODE=identity-mismatch
expect_status 1
assert_evidence "$case_root" false change_classification 0
python3 - "$case_root" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
payload = json.loads((root / "target/appfw/change-impact.json").read_text())
result = json.loads((root / "target/appfw/pr-preflight.json").read_text())
if payload.get("head_ref") != "0" * 40 or payload.get("sha") != "0" * 40:
    raise SystemExit("identity-mismatch child payload was not preserved")
if result.get("change_impact_contract_valid") is not False:
    raise SystemExit("identity mismatch report contract was marked valid")
PY

new_case change-impact-arbitrary-false
printf 'arbitrary false classification fixture\n' >>"$case_root/docs/README.md"
commit_case
run_local "$case_root" 20 env APPFW_TEST_CHANGE_IMPACT_MODE=arbitrary-false
expect_status 1
assert_evidence "$case_root" false change_classification 0
python3 - "$case_root" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
payload = json.loads((root / "target/appfw/change-impact.json").read_text())
result = json.loads((root / "target/appfw/pr-preflight.json").read_text())
if payload.get("ok") is not False or "delivery_profile" in payload:
    raise SystemExit("arbitrary-false child payload was not preserved")
if result.get("change_impact_contract_valid") is not False:
    raise SystemExit("arbitrary false report contract was marked valid")
if result.get("change_impact_operational_ok") is not False:
    raise SystemExit("arbitrary false operational status was not recorded")
PY

new_case change-impact-incoherent-projection
printf 'incoherent projection classification fixture\n' >>"$case_root/docs/README.md"
commit_case
run_local "$case_root" 20 env APPFW_TEST_CHANGE_IMPACT_MODE=projection-false-top-true
expect_status 1
assert_evidence "$case_root" false change_classification 0
python3 - "$case_root" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
payload = json.loads((root / "target/appfw/change-impact.json").read_text())
result = json.loads((root / "target/appfw/pr-preflight.json").read_text())
projection = payload.get("delivery_profile", {}).get("gate_execution_projection", {})
if payload.get("ok") is not True or projection.get("ok") is not False:
    raise SystemExit("incoherent projection child payload was not preserved")
if result.get("change_impact_contract_valid") is not False:
    raise SystemExit("incoherent projection report contract was marked valid")
if result.get("change_impact_operational_ok") is not True:
    raise SystemExit("incoherent projection operational status was not recorded")
PY

new_case change-impact-nonboolean-projection
printf 'nonboolean projection contract fixture\n' >>"$case_root/docs/README.md"
commit_case
run_local "$case_root" 20 env APPFW_TEST_CHANGE_IMPACT_MODE=projection-nonboolean
expect_status 1
assert_evidence "$case_root" false change_classification 0
python3 - "$case_root" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
payload = json.loads((root / "target/appfw/change-impact.json").read_text())
result = json.loads((root / "target/appfw/pr-preflight.json").read_text())
projection = payload.get("delivery_profile", {}).get("gate_execution_projection", {})
if type(projection.get("ok")) is not int or projection.get("ok") != 0:
    raise SystemExit("nonboolean projection child payload was not preserved")
if result.get("change_impact_contract_valid") is not False:
    raise SystemExit("nonboolean projection report contract was marked valid")
if result.get("change_impact_operational_ok") is not None:
    raise SystemExit("invalid projection contract invented operational status")
PY

new_case change-impact-git-error-count
printf 'git error surface contract fixture\n' >>"$case_root/docs/README.md"
commit_case
run_local "$case_root" 20 env APPFW_TEST_CHANGE_IMPACT_MODE=git-error-count
expect_status 1
assert_evidence "$case_root" false change_classification 0
python3 - "$case_root" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
payload = json.loads((root / "target/appfw/change-impact.json").read_text())
result = json.loads((root / "target/appfw/pr-preflight.json").read_text())
if payload.get("git_error_count") != 1:
    raise SystemExit("git-error-count child payload was not preserved")
if result.get("change_impact_contract_valid") is not False:
    raise SystemExit("git error surface report contract was marked valid")
if result.get("change_impact_operational_ok") is not None:
    raise SystemExit("git error surface invented operational status")
PY

# Local convenience never invents a base when --base is omitted.
new_case missing-identity
printf 'identity fixture\n' >>"$case_root/docs/README.md"
commit_case
run_without_base "$case_root" 20 env
expect_status 2
assert_evidence "$case_root" false infrastructure

# A resolvable but unrelated commit cannot become an invented comparison range.
new_case missing-merge-base
printf 'merge base fixture\n' >>"$case_root/docs/README.md"
commit_case
tree_sha="$(git -C "$case_root" write-tree)"
unrelated_sha="$(printf 'unrelated\n' | git -C "$case_root" commit-tree "$tree_sha")"
base_sha="$unrelated_sha"
run_local "$case_root" 20 env
expect_nonzero
assert_evidence "$case_root" false infrastructure

# Bitbucket mode fetches the named destination and rejects a stale trigger SHA.
new_case missing-source-identity
printf 'missing source identity fixture\n' >>"$case_root/docs/README.md"
commit_case
run_without_base "$case_root" 20 env \
  BITBUCKET_PR_DESTINATION_BRANCH=main \
  BITBUCKET_PR_DESTINATION_COMMIT="$base_sha"
expect_status 2
assert_evidence "$case_root" false infrastructure

new_case invalid-source-identity
printf 'invalid source identity fixture\n' >>"$case_root/docs/README.md"
commit_case
run_without_base "$case_root" 20 env \
  BITBUCKET_COMMIT=not-a-full-object-id \
  BITBUCKET_PR_DESTINATION_BRANCH=main \
  BITBUCKET_PR_DESTINATION_COMMIT="$base_sha"
expect_status 2
assert_evidence "$case_root" false infrastructure

new_case unresolved-source-identity
printf 'unresolved source identity fixture\n' >>"$case_root/docs/README.md"
commit_case
run_without_base "$case_root" 20 env \
  BITBUCKET_COMMIT=0000000000000000000000000000000000000000 \
  BITBUCKET_PR_DESTINATION_BRANCH=main \
  BITBUCKET_PR_DESTINATION_COMMIT="$base_sha"
expect_status 2
assert_evidence "$case_root" false infrastructure

new_case exact-destination
printf 'exact destination fixture\n' >>"$case_root/docs/README.md"
commit_case
source_sha="$(git -C "$case_root" rev-parse HEAD)"
git -C "$case_root" init -q --bare "$case_root/remote.git"
git -C "$case_root" remote add origin "$case_root/remote.git"
git -C "$case_root" push -q origin "$base_sha:refs/heads/main"
install_git_wrapper
remote_marker="$case_root/remote-git-operations.log"
run_without_base "$case_root" 20 env \
  APPFW_TEST_REMOTE_GIT_MODE=success \
  APPFW_TEST_REMOTE_GIT_MARKER="$remote_marker" \
  BITBUCKET_COMMIT="$source_sha" \
  BITBUCKET_PR_DESTINATION_BRANCH=main \
  BITBUCKET_PR_DESTINATION_COMMIT="${base_sha:0:12}"
expect_status 0
assert_evidence "$case_root" true none
python3 - "$case_root" "$source_sha" "$base_sha" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
result = json.loads(
    (root / "target/appfw/pr-preflight.json").read_text(encoding="utf-8")
)
source = sys.argv[2]
destination = sys.argv[3]
if result.get("mode") != "bitbucket-pr":
    raise SystemExit("exact destination fixture did not exercise Bitbucket mode")
if result.get("source_sha") != source or result.get("tested_sha") != source:
    raise SystemExit("Bitbucket direct-source identity binding is incorrect")
if result.get("destination_sha") != destination or result.get("merge_base") != destination:
    raise SystemExit("Bitbucket destination/tested merge-base binding is incorrect")
if result.get("tested_commit_relation") != "bitbucket-source-equals-tested-destination-ancestor":
    raise SystemExit("Bitbucket direct-source relation is incorrect")
identity_log = (root / "target/appfw/pr-preflight-logs/identity.log").read_text(
    encoding="utf-8"
)
for value in (source, destination, result["tested_commit_relation"]):
    if value not in identity_log:
        raise SystemExit(f"identity log omitted provenance: {value}")
progress = [
    json.loads(line)
    for line in (root / "target/appfw/pr-preflight-progress.jsonl").read_text(
        encoding="utf-8"
    ).splitlines()
]
if not any(
    event.get("check") == "identity"
    and event.get("status") == "passed"
    and result["tested_commit_relation"] in (event.get("detail") or "")
    for event in progress
):
    raise SystemExit("identity progress omitted the tested relation")
remote_git = result.get("remote_git", {})
if remote_git.get("timeout_seconds") != 45:
    raise SystemExit("Bitbucket preflight did not retain the default remote Git bound")
if remote_git.get("terminal_prompt_disabled") is not True:
    raise SystemExit("Bitbucket preflight did not retain noninteractive Git evidence")
PY
[[ "$(grep -c '^ls-remote$' "$remote_marker")" -eq 1 ]] ||
  fail "successful remote Git fixture did not exercise exactly one ls-remote"
[[ "$(grep -c '^fetch$' "$remote_marker")" -eq 1 ]] ||
  fail "successful remote Git fixture did not exercise exactly one fetch"

# Both preflight-owned network operations inherit the shared noninteractive
# per-operation timeout and fail closed on transport errors. The runner-wide
# deadline and Bitbucket's five-minute step backstop remain outer bounds.
for remote_target in ls-remote fetch; do
  new_case "remote-${remote_target}-nonzero"
  printf 'remote nonzero fixture\n' >>"$case_root/docs/README.md"
  commit_case
  source_sha="$(git -C "$case_root" rev-parse HEAD)"
  git -C "$case_root" init -q --bare "$case_root/remote.git"
  git -C "$case_root" remote add origin "$case_root/remote.git"
  git -C "$case_root" push -q origin "$base_sha:refs/heads/main"
  install_git_wrapper
  run_without_base "$case_root" 20 env \
    APPFW_PR_REMOTE_GIT_TIMEOUT_SECONDS=2 \
    APPFW_TEST_REMOTE_GIT_MODE=nonzero \
    APPFW_TEST_REMOTE_GIT_TARGET="$remote_target" \
    BITBUCKET_COMMIT="$source_sha" \
    BITBUCKET_PR_DESTINATION_BRANCH=main \
    BITBUCKET_PR_DESTINATION_COMMIT="$base_sha"
  expect_status 87
  assert_evidence "$case_root" false infrastructure 87
  if [[ "$remote_target" == "ls-remote" ]]; then
    remote_failure_message="remote destination branch head is unavailable"
  else
    remote_failure_message="bounded destination fetch failed"
  fi
  assert_remote_transport_evidence "$case_root" 2 "$remote_failure_message"

  new_case "remote-${remote_target}-hang"
  printf 'remote hang fixture\n' >>"$case_root/docs/README.md"
  commit_case
  source_sha="$(git -C "$case_root" rev-parse HEAD)"
  git -C "$case_root" init -q --bare "$case_root/remote.git"
  git -C "$case_root" remote add origin "$case_root/remote.git"
  git -C "$case_root" push -q origin "$base_sha:refs/heads/main"
  install_git_wrapper
  remote_started="$SECONDS"
  run_without_base "$case_root" 20 env \
    APPFW_PR_REMOTE_GIT_TIMEOUT_SECONDS=1 \
    APPFW_TEST_REMOTE_GIT_MODE=hang \
    APPFW_TEST_REMOTE_GIT_TARGET="$remote_target" \
    BITBUCKET_COMMIT="$source_sha" \
    BITBUCKET_PR_DESTINATION_BRANCH=main \
    BITBUCKET_PR_DESTINATION_COMMIT="$base_sha"
  remote_elapsed=$((SECONDS - remote_started))
  expect_status 124
  assert_evidence "$case_root" false timeout
  assert_remote_transport_evidence "$case_root" 1 "timed out after 1 seconds"
  (( remote_elapsed < 8 )) ||
    fail "remote Git $remote_target hang escaped its per-operation bound"
done

# Destination IDs shorter than Bitbucket's observed 12-character value are
# rejected before the named remote branch can establish canonical identity.
new_case short-destination-identity
printf 'short destination identity fixture\n' >>"$case_root/docs/README.md"
commit_case
source_sha="$(git -C "$case_root" rev-parse HEAD)"
git -C "$case_root" init -q --bare "$case_root/remote.git"
git -C "$case_root" remote add origin "$case_root/remote.git"
git -C "$case_root" push -q origin "$base_sha:refs/heads/main"
run_without_base "$case_root" 20 env \
  BITBUCKET_COMMIT="$source_sha" \
  BITBUCKET_PR_DESTINATION_BRANCH=main \
  BITBUCKET_PR_DESTINATION_COMMIT="${base_sha:0:11}"
expect_status 2
assert_evidence "$case_root" false infrastructure

new_case unsupported-destination-identity-length
printf 'unsupported destination identity length fixture\n' >>"$case_root/docs/README.md"
commit_case
source_sha="$(git -C "$case_root" rev-parse HEAD)"
run_without_base "$case_root" 20 env \
  BITBUCKET_COMMIT="$source_sha" \
  BITBUCKET_PR_DESTINATION_BRANCH=main \
  BITBUCKET_PR_DESTINATION_COMMIT="${base_sha:0:13}"
expect_status 2
assert_evidence "$case_root" false infrastructure

new_case nonhex-destination-identity
printf 'nonhex destination identity fixture\n' >>"$case_root/docs/README.md"
commit_case
source_sha="$(git -C "$case_root" rev-parse HEAD)"
run_without_base "$case_root" 20 env \
  BITBUCKET_COMMIT="$source_sha" \
  BITBUCKET_PR_DESTINATION_BRANCH=main \
  BITBUCKET_PR_DESTINATION_COMMIT=zzzzzzzzzzzz
expect_status 2
assert_evidence "$case_root" false infrastructure

new_case stale-destination
printf 'stale destination fixture\n' >>"$case_root/docs/README.md"
commit_case
source_sha="$(git -C "$case_root" rev-parse HEAD)"
git -C "$case_root" init -q --bare "$case_root/remote.git"
git -C "$case_root" remote add origin "$case_root/remote.git"
git -C "$case_root" push -q origin "$base_sha:refs/heads/main"
run_without_base "$case_root" 20 env \
  BITBUCKET_COMMIT="$source_sha" \
  BITBUCKET_PR_DESTINATION_BRANCH=main \
  BITBUCKET_PR_DESTINATION_COMMIT=000000000000
expect_status 2
assert_evidence "$case_root" false infrastructure

# A tested descendant not equal to the source and not an exact provider merge
# is rejected even when every commit resolves.
new_case mismatched-tested-relation
printf 'bound source fixture\n' >>"$case_root/docs/README.md"
commit_case source
source_sha="$(git -C "$case_root" rev-parse HEAD)"
printf 'unbound tested descendant\n' >>"$case_root/docs/README.md"
commit_case unbound-tested
git -C "$case_root" init -q --bare "$case_root/remote.git"
git -C "$case_root" remote add origin "$case_root/remote.git"
git -C "$case_root" push -q origin "$base_sha:refs/heads/main"
run_without_base "$case_root" 20 env \
  BITBUCKET_COMMIT="$source_sha" \
  BITBUCKET_PR_DESTINATION_BRANCH=main \
  BITBUCKET_PR_DESTINATION_COMMIT="$base_sha"
expect_status 2
assert_evidence "$case_root" false infrastructure

# When Bitbucket fast-forwards an empty/behind source to the destination, the
# tested destination is accepted only when it directly contains the source.
new_case tested-destination-fast-forward
source_sha="$base_sha"
printf 'destination fast-forward fixture\n' >>"$case_root/docs/README.md"
commit_case destination
destination_sha="$(git -C "$case_root" rev-parse HEAD)"
git -C "$case_root" init -q --bare "$case_root/remote.git"
git -C "$case_root" remote add origin "$case_root/remote.git"
git -C "$case_root" push -q origin "$destination_sha:refs/heads/main"
run_without_base "$case_root" 20 env \
  BITBUCKET_COMMIT="$source_sha" \
  BITBUCKET_PR_DESTINATION_BRANCH=main \
  BITBUCKET_PR_DESTINATION_COMMIT="$destination_sha"
expect_status 0
assert_evidence "$case_root" true none
python3 - "$case_root" "$source_sha" "$destination_sha" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
result = json.loads(
    (root / "target/appfw/pr-preflight.json").read_text(encoding="utf-8")
)
if result.get("source_sha") != sys.argv[2]:
    raise SystemExit("fast-forward fixture source identity mismatch")
if result.get("tested_sha") != sys.argv[3] or result.get("destination_sha") != sys.argv[3]:
    raise SystemExit("fast-forward fixture tested/destination identity mismatch")
if result.get("merge_base") != sys.argv[3]:
    raise SystemExit("fast-forward fixture merge base must be the tested destination")
if result.get("tested_commit_relation") != "bitbucket-destination-equals-tested-source-ancestor":
    raise SystemExit("fast-forward fixture relation mismatch")
PY

# A real behind-destination provider merge has the source and exact fetched
# destination as its two direct parents; checks run against that tested merge.
new_case synthetic-tested-merge
git -C "$case_root" switch -q -c feature
printf 'source-side fixture\n' >>"$case_root/docs/README.md"
commit_case source
source_sha="$(git -C "$case_root" rev-parse HEAD)"
git -C "$case_root" switch -q -c destination "$base_sha"
printf 'destination-side fixture\n' >"$case_root/docs/destination.txt"
commit_case destination
destination_sha="$(git -C "$case_root" rev-parse HEAD)"
git -C "$case_root" switch -q feature
git -C "$case_root" merge -q --no-ff destination -m 'provider synthetic merge'
tested_sha="$(git -C "$case_root" rev-parse HEAD)"
git -C "$case_root" init -q --bare "$case_root/remote.git"
git -C "$case_root" remote add origin "$case_root/remote.git"
git -C "$case_root" push -q origin "$destination_sha:refs/heads/main"
run_without_base "$case_root" 20 env \
  BITBUCKET_COMMIT="$source_sha" \
  BITBUCKET_PR_DESTINATION_BRANCH=main \
  BITBUCKET_PR_DESTINATION_COMMIT="$destination_sha"
expect_status 0
assert_evidence "$case_root" true none
python3 - "$case_root" "$source_sha" "$tested_sha" "$destination_sha" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
source, tested, destination = sys.argv[2:5]
result = json.loads(
    (root / "target/appfw/pr-preflight.json").read_text(encoding="utf-8")
)
if result.get("source_sha") != source or result.get("tested_sha") != tested:
    raise SystemExit("synthetic merge source/tested identity mismatch")
if result.get("destination_sha") != destination:
    raise SystemExit("synthetic merge destination identity mismatch")
if result.get("merge_base") != destination:
    raise SystemExit("synthetic merge effective comparison must start at destination")
if result.get("tested_commit_relation") != "bitbucket-tested-merge-of-source-and-destination":
    raise SystemExit("synthetic merge relation mismatch")
command_log = (root / "target/appfw/pr-preflight-logs/change-classification.log").read_text(
    encoding="utf-8"
)
if destination not in command_log or tested not in command_log:
    raise SystemExit("change-impact did not use destination-to-tested identity")
if source in command_log and source != tested:
    raise SystemExit("change-impact incorrectly used source as the tested head")
PY

# Runner deadline and tool availability have distinct stable categories.
new_case timeout
printf 'timeout fixture\n' >>"$case_root/docs/README.md"
commit_case
term_marker="$case_root/term-resistant.pid"
run_local "$case_root" 1 env \
  APPFW_TEST_CHANGE_IMPACT_MODE=term-resistant \
  APPFW_TEST_TERM_MARKER="$term_marker"
expect_status 124
assert_evidence "$case_root" false timeout
[[ -s "$term_marker" ]] || fail "TERM-resistant descendant PID was not retained"
term_pid="$(cat "$term_marker")"
for _ in {1..30}; do
  if ! kill -0 "$term_pid" 2>/dev/null; then
    term_pid=""
    break
  fi
  sleep 0.1
done
[[ -z "$term_pid" ]] || fail "TERM-resistant descendant survived preflight timeout cleanup"

# A deadline expiring after log registration but before command launch still
# produces a complete manifest plus terminal result and JUnit evidence.
new_case pre-command-deadline
printf 'deadline-before-command fixture\n' >>"$case_root/docs/README.md"
commit_case
run_deadline_before_command "$case_root"
expect_status 124
assert_evidence "$case_root" false timeout
python3 - "$case_root" <<'PY'
import json
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

root = Path(sys.argv[1])
result = json.loads(
    (root / "target/appfw/pr-preflight.json").read_text(encoding="utf-8")
)
if result.get("failed_check") != "deadline":
    raise SystemExit("pre-command deadline did not retain its terminal check")
if not any(
    record.get("name") == "deadline" and record.get("status") == "failed"
    for record in result.get("checks", [])
):
    raise SystemExit("pre-command deadline omitted its failed result record")
started_log = root / "target/appfw/pr-preflight-logs/identity.log"
if "status=started" not in started_log.read_text(encoding="utf-8"):
    raise SystemExit("registered check log was not materialized before the deadline")
junit = ET.parse(root / "test-results/pr-preflight.xml").getroot()
if junit.find("./testcase[@name='deadline']/error") is None:
    raise SystemExit("pre-command deadline omitted its JUnit terminal error")
PY

new_case infrastructure
printf 'infrastructure fixture\n' >>"$case_root/docs/README.md"
commit_case
run_local "$case_root" 20 env APPFW_TEST_CARGO_MODE=infrastructure
expect_nonzero
assert_evidence "$case_root" false infrastructure 127

# The producer rejects extra files in its owned evidence namespace.
new_case unexpected-artifact
printf 'allowlist fixture\n' >>"$case_root/docs/README.md"
commit_case
run_local "$case_root" 20 env APPFW_TEST_CHANGE_IMPACT_MODE=rogue
expect_status 2
assert_evidence "$case_root" false infrastructure

# Git-object inspection handles rename/delete, binary, symlink, spaces,
# newlines, and pathspec-looking names without following the worktree symlink.
new_case unusual-paths
printf '#!/usr/bin/env bash\necho old\n' >"$case_root/rename old.sh"
printf '#!/usr/bin/env bash\necho delete\n' >"$case_root/delete.sh"
git -C "$case_root" add .
git -C "$case_root" commit -q -m unusual-base
base_sha="$(git -C "$case_root" rev-parse HEAD)"
git -C "$case_root" mv "rename old.sh" "rename new.sh"
git -C "$case_root" rm -q delete.sh
ln -s /definitely/not/followed "$case_root/unsafe symlink.sh"
printf '#!/bin/sh\necho sh\n' >"$case_root/extensionless-sh"
printf '#!/bin/bash\necho bash\n' >"$case_root/extensionless-bash"
printf '#!/usr/bin/env bash\necho env\n' >"$case_root/extensionless-env"
chmod +x "$case_root/extensionless-sh" "$case_root/extensionless-bash" "$case_root/extensionless-env"
python3 - "$case_root" <<'PY'
import sys
from pathlib import Path

root = Path(sys.argv[1])
(root / "binary.dat").write_bytes(b"binary\x00<<<<<<< HEAD\n")
(root / "line\nbreak.txt").write_text("ordinary text\n", encoding="utf-8")
(root / ":(glob)literal.txt").write_text("literal pathspec-looking name\n", encoding="utf-8")
PY
commit_case unusual-change
run_local "$case_root" 20 env
expect_status 0
assert_evidence "$case_root" true none
python3 - "$case_root" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
marker_log = (root / "target/appfw/pr-preflight-logs/conflict-marker.log").read_bytes()
if b"scanned_path=line\\nbreak.txt" not in marker_log:
    raise SystemExit("newline-bearing path is not rendered with a literal escape")
if b"scanned_path=line\nbreak.txt" in marker_log:
    raise SystemExit("newline-bearing path injected a physical log line")
shell_log = (root / "target/appfw/pr-preflight-logs/shell-syntax.log").read_text(
    encoding="utf-8"
)
for expected in (
    "path=extensionless-sh",
    "path=extensionless-bash",
    "path=extensionless-env",
    "path=rename new.sh",
):
    if expected not in shell_log:
        raise SystemExit(f"missing shell shebang/path fixture evidence: {expected}")
if "skipped_symlink_blobs=1" not in shell_log:
    raise SystemExit("shell symlink was not explicitly skipped")
for path in (
    root / "target/appfw/pr-preflight.json",
    root / "target/appfw/pr-preflight-progress.jsonl",
    root / "target/appfw/pr-preflight-manifest.json",
):
    if path.suffix == ".jsonl":
        for line in path.read_text(encoding="utf-8").splitlines():
            json.loads(line)
    else:
        json.loads(path.read_text(encoding="utf-8"))
PY

printf 'PR preflight fixture tests passed.\n'
