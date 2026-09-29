#!/usr/bin/env bash
#
# pds-baseline-ready-fixture.sh - local structural preflight for the PDS
# Security Baseline r4.5 live-required gate.
#
# This writes synthetic retained evidence under target/appfw/pds-ready-fixture
# by default, then runs the real pds-baseline command with live evidence
# required. The result proves the retained decision/evidence shape is acceptable
# to the gate. It is not release evidence and must not be promoted.
#
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
cd "$repo_root"

fixture_dir="${APPFW_PDS_BASELINE_FIXTURE_DIR:-target/appfw/pds-ready-fixture}"
decision_file="$fixture_dir/pds-baseline-decision.json"
output_file="$fixture_dir/pds-baseline-ready-fixture-output.json"

ensure_python3() {
  if command -v python3 >/dev/null 2>&1; then
    return 0
  fi

  echo "pds-baseline-ready-fixture: python3 is required" >&2
  exit 2
}

ensure_python3
mkdir -p "$fixture_dir"

python3 - "$fixture_dir" "$decision_file" <<'PY'
import datetime as dt
import json
import sys
from pathlib import Path

fixture_dir = Path(sys.argv[1])
decision_file = Path(sys.argv[2])
evidence_dir = fixture_dir / "pds-baseline"
evidence_dir.mkdir(parents=True, exist_ok=True)

generated_at = (
    dt.datetime.now(dt.timezone.utc)
    .replace(microsecond=0)
    .isoformat()
    .replace("+00:00", "Z")
)

work_items = [
    ("LIVE-015", "release-authority"),
    ("LIVE-016", "iam-okta-mfa-lifecycle"),
    ("LIVE-017", "secrets-management"),
    ("LIVE-018", "api-gateway-waf-tls-scanning"),
    ("LIVE-019", "lower-env-backup-dr"),
    ("LIVE-020", "platform-network-db-hardening"),
    ("LIVE-021", "siem-monitoring-access-review"),
]

decision_work_items = []
for work_item_id, slug in work_items:
    evidence_path = evidence_dir / f"{work_item_id.lower()}-{slug}.json"
    evidence_path.write_text(
        json.dumps(
            {
                "fixture": True,
                "ok": True,
                "release_ready": True,
                "work_item_id": work_item_id,
                "generated_at_utc": generated_at,
                "evidence_type": "local pre-CI ready-shape fixture",
                "note": (
                    "Synthetic structural fixture only; not live PDS evidence "
                    "or release approval."
                ),
            },
            indent=2,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )
    decision_work_items.append(
        {
            "id": work_item_id,
            "status": "evidence",
            "evidence_files": [evidence_path.relative_to(fixture_dir).as_posix()],
            "summary": f"Synthetic retained evidence shape for {work_item_id}",
        }
    )

decision_file.write_text(
    json.dumps(
        {
            "fixture": True,
            "ok": True,
            "release_ready": True,
            "baseline": {
                "version": "r4.5",
            },
            "release_authority": {
                "owner": "local-pre-ci-fixture",
                "approver": "local-pre-ci-fixture",
                "approved_at_utc": generated_at,
            },
            "work_items": decision_work_items,
            "note": (
                "Synthetic structural fixture only; not live PDS evidence "
                "or release approval."
            ),
        },
        indent=2,
        sort_keys=True,
    )
    + "\n",
    encoding="utf-8",
)
PY

if ! APPFW_RELEASE_ARTIFACT_DIR="$fixture_dir" \
  APPFW_RELEASE_REQUIRE_PDS_BASELINE_EVIDENCE=true \
  APPFW_PDS_BASELINE_DECISION_FILE="$decision_file" \
  scripts/appfw framework pds-baseline --json >"$output_file"; then
  cat "$output_file" >&2 || true
  echo "pds-baseline-ready-fixture: FAILED" >&2
  exit 1
fi

python3 - "$fixture_dir/pds-security-baseline.json" <<'PY'
import json
import sys
from pathlib import Path

artifact = Path(sys.argv[1])
report = json.loads(artifact.read_text(encoding="utf-8"))
required_work_items = {
    "LIVE-015",
    "LIVE-016",
    "LIVE-017",
    "LIVE-018",
    "LIVE-019",
    "LIVE-020",
    "LIVE-021",
}
summary = report.get("decision_summary", {})
covered = set(summary.get("covered_work_items", []))
evidence_backed = set(summary.get("evidence_backed_work_items", []))
failed_checks = [
    check.get("name", "<unnamed>")
    for check in report.get("checks", [])
    if isinstance(check, dict) and check.get("ok") is not True
]

problems = []
if report.get("ok") is not True:
    problems.append("ok is not true")
if report.get("release_ready") is not True:
    problems.append("release_ready is not true")
if report.get("live_evidence_required") is not True:
    problems.append("live_evidence_required is not true")
if report.get("failure_count") != 0:
    problems.append(f"failure_count is {report.get('failure_count')!r}")
if report.get("release_blockers"):
    problems.append("release_blockers is not empty")
if covered != required_work_items:
    problems.append("covered_work_items does not cover LIVE-015 through LIVE-021")
if evidence_backed != required_work_items:
    problems.append("evidence_backed_work_items does not cover LIVE-015 through LIVE-021")
if len(report.get("evidence_artifacts", [])) < len(required_work_items):
    problems.append("evidence_artifacts does not retain one artifact per LIVE item")
if failed_checks:
    problems.append("failing checks: " + ", ".join(failed_checks))

if problems:
    print("pds-baseline-ready-fixture: FAILED", file=sys.stderr)
    for problem in problems:
        print(f"  - {problem}", file=sys.stderr)
    sys.exit(1)

print("pds-baseline-ready-fixture: OK")
print(f"  evidence: {artifact.as_posix()}")
print("  note: synthetic structural fixture only; not release evidence")
PY
