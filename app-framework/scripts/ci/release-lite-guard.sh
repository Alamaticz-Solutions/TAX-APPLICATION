#!/usr/bin/env bash
#
# release-lite-guard.sh - PR guard for provider/runtime/security-sensitive changes.
#
# This guard is intentionally artifact-only and does not start provider services.
# It makes pull requests fail closed when they touch surfaces that require
# provider/security release evidence, unless an approved manual release-check or
# release-lite run is referenced by environment variables.
#
# Usage:
#   bash scripts/ci/release-lite-guard.sh
#
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
cd "$repo_root"

report_dir="${APPFW_RELEASE_ARTIFACT_DIR:-target/appfw}"
artifact_file="$report_dir/release-lite-guard.json"
mkdir -p "$report_dir"

changed_files_file="$(mktemp "${TMPDIR:-/tmp}/appfw-release-lite-files.XXXXXX")"
trap 'rm -f "$changed_files_file"' EXIT

base_ref="${APPFW_RELEASE_LITE_BASE_REF:-}"
comparison_mode="unknown"
comparison_detail=""
comparison_error=""
comparison_ready=true
remote_git_timeout_seconds="${APPFW_PR_REMOTE_GIT_TIMEOUT_SECONDS:-45}"

write_changed_files_from_env() {
  python3 - "$changed_files_file" "${APPFW_RELEASE_LITE_CHANGED_FILES:-}" <<'PY'
import re
import sys

path = sys.argv[1]
raw = sys.argv[2]
items = [item.strip() for item in re.split(r"[\n,]", raw) if item.strip()]
with open(path, "w", encoding="utf-8") as handle:
    for item in sorted(dict.fromkeys(items)):
        handle.write(item + "\n")
PY
}

resolve_changed_files() {
  if [[ -n "${APPFW_RELEASE_LITE_CHANGED_FILES:-}" ]]; then
    write_changed_files_from_env
    comparison_mode="env"
    comparison_detail="APPFW_RELEASE_LITE_CHANGED_FILES"
    return 0
  fi

  local destination="${BITBUCKET_PR_DESTINATION_BRANCH:-}"
  if [[ -n "$destination" ]]; then
    local remote_ref="refs/remotes/origin/$destination"
    comparison_detail="$remote_ref"

    if ! git check-ref-format --branch "$destination" >/dev/null 2>&1; then
      comparison_mode="destination-fetch-failed"
      comparison_error="BITBUCKET_PR_DESTINATION_BRANCH is invalid"
      return 1
    fi
    if ! [[ "$remote_git_timeout_seconds" =~ ^[0-9]+$ ]] ||
      (( remote_git_timeout_seconds < 1 || remote_git_timeout_seconds > 300 )); then
      comparison_mode="destination-fetch-failed"
      comparison_error="APPFW_PR_REMOTE_GIT_TIMEOUT_SECONDS must be an integer from 1 through 300"
      return 1
    fi

    local fetch_message fetch_status
    set +e
    fetch_message="$(python3 - "$destination" "$remote_git_timeout_seconds" <<'PY'
import os
import subprocess
import sys

destination = sys.argv[1]
timeout_seconds = int(sys.argv[2])
environment = os.environ.copy()
environment["GIT_TERMINAL_PROMPT"] = "0"
try:
    result = subprocess.run(
        [
            "git",
            "fetch",
            "--no-tags",
            "origin",
            f"+refs/heads/{destination}:refs/remotes/origin/{destination}",
        ],
        check=False,
        env=environment,
        stderr=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        timeout=timeout_seconds,
    )
except subprocess.TimeoutExpired:
    print(f"destination fetch timed out after {timeout_seconds} seconds")
    raise SystemExit(124)
except OSError:
    print("destination fetch could not start")
    raise SystemExit(125)

if result.returncode != 0:
    print(f"destination fetch failed with status {result.returncode}")
    raise SystemExit(1)
PY
)"
    fetch_status=$?
    set -e
    if (( fetch_status != 0 )); then
      comparison_mode="destination-fetch-failed"
      comparison_error="${fetch_message:-destination fetch failed without a diagnostic}"
      return 1
    fi

    local merge_base
    if ! git rev-parse --verify "$remote_ref^{commit}" >/dev/null 2>&1; then
      comparison_mode="destination-fetch-failed"
      comparison_error="fetched destination ref is not a commit"
      return 1
    fi
    merge_base="$(git merge-base HEAD "$remote_ref" 2>/dev/null || true)"
    if [[ -z "$merge_base" ]]; then
      comparison_mode="destination-fetch-failed"
      comparison_error="fetched destination has no merge base with HEAD"
      return 1
    fi
    git diff --name-only "$merge_base" HEAD >"$changed_files_file"
    comparison_mode="merge-base"
    return 0
  fi

  local candidate merge_base
  local candidates=()
  if [[ -n "$base_ref" ]]; then
    candidates+=("$base_ref")
  fi
  if [[ -n "$destination" ]]; then
    candidates+=("refs/remotes/origin/$destination" "origin/$destination" "$destination")
  fi
  candidates+=("refs/remotes/origin/main" "origin/main" "main")

  for candidate in "${candidates[@]}"; do
    if git rev-parse --verify "$candidate^{commit}" >/dev/null 2>&1; then
      merge_base="$(git merge-base HEAD "$candidate" 2>/dev/null || true)"
      if [[ -n "$merge_base" ]]; then
        git diff --name-only "$merge_base" HEAD >"$changed_files_file"
        comparison_mode="merge-base"
        comparison_detail="$candidate"
        return 0
      fi
    fi
  done

  if git rev-parse --verify HEAD~1 >/dev/null 2>&1; then
    git diff --name-only HEAD~1 HEAD >"$changed_files_file"
    comparison_mode="previous-commit"
    comparison_detail="HEAD~1"
    return 0
  fi

  git diff --name-only >"$changed_files_file"
  comparison_mode="working-tree"
  comparison_detail="git diff --name-only"
}

if ! resolve_changed_files; then
  comparison_ready=false
fi

python3 - "$artifact_file" "$changed_files_file" "$comparison_mode" "$comparison_detail" "$comparison_ready" "$comparison_error" <<'PY'
import datetime as dt
import fnmatch
import json
import os
import sys
from pathlib import Path

artifact_file = Path(sys.argv[1])
changed_files_file = Path(sys.argv[2])
comparison_mode = sys.argv[3]
comparison_detail = sys.argv[4]
comparison_ready = sys.argv[5] == "true"
comparison_error = sys.argv[6]


def utc_now():
    return (
        dt.datetime.now(dt.timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z")
    )


changed_files = [
    line.strip()
    for line in changed_files_file.read_text(encoding="utf-8").splitlines()
    if line.strip()
]

sensitive_patterns = [
    ("bitbucket-pipelines.yml", "release pipeline contract"),
    ("scripts/appfw", "framework CLI/release command contract"),
    ("scripts/ci/**", "release/security CI evidence and control surface"),
    ("appfw_runtime/**", "shared runtime/security/provider execution"),
    ("appfw_provider_*/**", "provider implementation or certification"),
    ("appfw_test/**", "shared provider/security test harness"),
    ("api_tests/**", "provider/security certification scenarios"),
    ("app_gen/src/**", "generator/runtime output semantics"),
    ("app_gen/_templates/backend/**", "generated backend runtime/security behavior"),
    ("database/**", "migration/provider runtime behavior"),
    ("rego_test/**", "policy verifier behavior"),
    ("examples/products/crm/backend/Cargo.toml", "reference backend provider dependencies"),
    ("examples/products/crm/backend/src/config/**", "reference backend provider/security config"),
    ("examples/products/crm/backend/src/data/**", "reference backend provider data access"),
    ("examples/products/crm/backend/src/routes/**", "reference backend route/security surface"),
    ("examples/products/crm/backend/src/product_api.rs", "reference backend public operation surface"),
    ("Cargo.toml", "workspace dependency/release surface"),
    ("Cargo.lock", "workspace dependency/release surface"),
    ("deny.toml", "supply-chain policy"),
]

sensitive_files = []
for path in changed_files:
    normalized = path.lstrip("./")
    for pattern, reason in sensitive_patterns:
        if fnmatch.fnmatch(normalized, pattern):
            sensitive_files.append(
                {
                    "path": normalized,
                    "pattern": pattern,
                    "reason": reason,
                }
            )
            break

evidence_url = os.environ.get("APPFW_RELEASE_LITE_EVIDENCE_URL", "").strip()
approver = os.environ.get("APPFW_RELEASE_LITE_APPROVER", "").strip()
approval_reason = os.environ.get("APPFW_RELEASE_LITE_REASON", "").strip()
approved_at = os.environ.get("APPFW_RELEASE_LITE_APPROVED_AT_UTC", "").strip()
approval_present = bool(evidence_url and approver and approval_reason)
release_lite_required = bool(sensitive_files)
ok = comparison_ready and (not release_lite_required or approval_present)

missing_approval_fields = []
if release_lite_required and not approval_present:
    if not evidence_url:
        missing_approval_fields.append("APPFW_RELEASE_LITE_EVIDENCE_URL")
    if not approver:
        missing_approval_fields.append("APPFW_RELEASE_LITE_APPROVER")
    if not approval_reason:
        missing_approval_fields.append("APPFW_RELEASE_LITE_REASON")

root_causes = []
if not comparison_ready:
    root_causes.append(
        comparison_error
        or "exact PR destination comparison could not be established"
    )
if not ok:
    if release_lite_required and not approval_present:
        root_causes.append(
            "provider/runtime/security-sensitive PR changes require a referenced "
            "manual release-check or release-lite evidence run before merge"
        )
        root_causes.append(
            "missing approval fields: " + ", ".join(missing_approval_fields)
        )

evidence = {
    "command": "release-lite-guard",
    "ok": ok,
    "generated_at_utc": utc_now(),
    "comparison": {
        "ok": comparison_ready,
        "mode": comparison_mode,
        "detail": comparison_detail,
        "error": comparison_error or None,
    },
    "changed_file_count": len(changed_files),
    "changed_files": changed_files,
    "release_lite_required": release_lite_required,
    "sensitive_files": sensitive_files,
    "approval": {
        "present": approval_present,
        "evidence_url": evidence_url or None,
        "approver": approver or None,
        "reason": approval_reason or None,
        "approved_at_utc": approved_at or None,
        "missing_fields": missing_approval_fields,
    },
    "failure_summary": {
        "root_causes": root_causes,
    },
    "remediation": (
        "Restore bounded access to the exact PR destination branch, then rerun "
        "the guard without using a stale local fallback."
        if not comparison_ready
        else
        "Run the Bitbucket custom release-check pipeline or an approved "
        "provider-backed release-lite lane, then rerun this PR pipeline with "
        "APPFW_RELEASE_LITE_EVIDENCE_URL, APPFW_RELEASE_LITE_APPROVER, and "
        "APPFW_RELEASE_LITE_REASON populated."
        if release_lite_required and not approval_present
        else None
    ),
}

artifact_file.write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n", encoding="utf-8")

if not ok:
    print("release-lite-guard: FAILED", file=sys.stderr)
    for cause in root_causes:
        print(f"  - {cause}", file=sys.stderr)
    print(f"release-lite-guard: evidence written to {artifact_file}", file=sys.stderr)
    sys.exit(1)

if release_lite_required:
    print(
        "release-lite-guard: OK - sensitive changes have referenced release-lite evidence; "
        f"evidence written to {artifact_file}"
    )
else:
    print(
        "release-lite-guard: OK - no provider/runtime/security-sensitive changes detected; "
        f"evidence written to {artifact_file}"
    )
PY
