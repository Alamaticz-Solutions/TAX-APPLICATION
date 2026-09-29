#!/usr/bin/env bash
#
# prepare-appfw-evidence-root.sh - start a CI producer step with fresh evidence.
#
# Bitbucket build caches must not restore retained evidence. This helper clears
# only the canonical framework evidence root, then writes a small marker so later
# reviewers can see the step intentionally started from a clean evidence root.
#
# Usage:
#   bash scripts/ci/prepare-appfw-evidence-root.sh
#
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
cd "$repo_root"

report_dir="${APPFW_RELEASE_ARTIFACT_DIR:-target/appfw}"

case "$report_dir" in
  target/appfw | ./target/appfw | "$repo_root/target/appfw")
    ;;
  *)
    cat >&2 <<EOF
prepare-appfw-evidence-root refuses to clear non-canonical evidence dir:
  APPFW_RELEASE_ARTIFACT_DIR=${report_dir}

Use this helper only for CI producer steps that write the framework evidence
root. Aggregating steps that consume prior artifacts must not call it.
EOF
    exit 2
    ;;
esac

rm -rf target/appfw
mkdir -p target/appfw

python3 - <<'PY'
import json
import os
import subprocess
from datetime import datetime, timezone
from pathlib import Path

def git_value(*args):
    try:
        return subprocess.check_output(
            ["git", *args],
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
    except Exception:
        return None

ci_keys = [
    "BITBUCKET_BUILD_NUMBER",
    "BITBUCKET_COMMIT",
    "BITBUCKET_BRANCH",
    "BITBUCKET_TAG",
    "BITBUCKET_PIPELINE_UUID",
    "BITBUCKET_STEP_UUID",
    "BITBUCKET_REPO_FULL_NAME",
]

payload = {
    "command": "prepare-appfw-evidence-root",
    "ok": True,
    "generated_at_utc": datetime.now(timezone.utc).isoformat(),
    "artifact": "target/appfw/ci-evidence-root.json",
    "evidence_root": "target/appfw",
    "cleaned_before_step": True,
    "cache_boundary": {
        "evidence_root_cached": False,
        "cargo_target_dir": os.environ.get("CARGO_TARGET_DIR"),
        "release_tool_dir": os.environ.get("APPFW_RELEASE_TOOL_DIR"),
    },
    "git": {
        "commit": git_value("rev-parse", "HEAD"),
        "branch": git_value("rev-parse", "--abbrev-ref", "HEAD"),
    },
    "bitbucket": {
        key.lower(): value
        for key in ci_keys
        if (value := os.environ.get(key))
    },
}

Path("target/appfw/ci-evidence-root.json").write_text(
    json.dumps(payload, indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
)
PY

echo "Prepared fresh framework evidence root at target/appfw"
