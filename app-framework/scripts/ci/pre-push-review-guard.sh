#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/ci/pre-push-review-guard.sh [--json]

Checks whether the current framework branch has current handoff evidence and
Framework PR Review Agent output that satisfies the standing pre-push policy.
Review depth is selected automatically: focused for narrow ordinary branches,
comprehensive for broad or sensitive branches.

Environment:
  APPFW_PRE_PUSH_REVIEW_OUTPUT          Review output path. Defaults to target/appfw/framework-pr-review.md.
  APPFW_PRE_PUSH_REVIEW_BYPASS_REASON  Explicit one-push human bypass reason.
EOF
}

json_mode=false
while [[ $# -gt 0 ]]; do
  case "$1" in
    --json)
      json_mode=true
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "unknown pre-push-review-guard option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

artifact_dir="target/appfw"
mkdir -p "$artifact_dir"

report_path="$artifact_dir/pre-push-review.json"
review_brief_path="$artifact_dir/pre-push-review-brief.json"
stderr_path="$artifact_dir/pre-push-review.err"
review_output="${APPFW_PRE_PUSH_REVIEW_OUTPUT:-target/appfw/framework-pr-review.md}"

if [[ -n "${APPFW_PRE_PUSH_REVIEW_BYPASS_REASON:-}" ]]; then
  python3 - "$report_path" "$review_output" "$APPFW_PRE_PUSH_REVIEW_BYPASS_REASON" "$json_mode" <<'PY'
import datetime
import json
import sys
from pathlib import Path

report_path = Path(sys.argv[1])
review_output = sys.argv[2]
review_output_path = Path(review_output)
review_output_abs = str(review_output_path if review_output_path.is_absolute() else Path.cwd() / review_output_path)
review_output_label = review_output_path.name or "framework-pr-review.md"
review_output_link = f"[{review_output_label}]({review_output_abs})"
reason = sys.argv[3]
json_mode = sys.argv[4] == "true"

payload = {
    "command": "pre-push-review",
    "ok": True,
    "bypassed": True,
    "generated_at": datetime.datetime.now(datetime.timezone.utc).isoformat().replace("+00:00", "Z"),
    "review_output": review_output,
    "review_output_abs": review_output_abs,
    "review_output_link": review_output_link,
    "reason": reason,
}
report_path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
if json_mode:
    print(json.dumps(payload, indent=2, sort_keys=True))
else:
    print("pre-push review guard bypassed by explicit human reason.", file=sys.stderr)
    print(f"reason: {reason}", file=sys.stderr)
    print(f"full review: {review_output_link}", file=sys.stderr)
raise SystemExit(0)
PY
  exit 0
fi

set +e
review_stdout="$("$repo_root/scripts/appfw" framework review-brief --auto-depth --review-output "$review_output" --json 2>"$stderr_path")"
review_exit=$?
set -e

if [[ -n "$review_stdout" ]]; then
  printf '%s\n' "$review_stdout" >"$review_brief_path"
else
  printf '{"command":"pre-push-review","ok":false,"detail":"review-brief produced no output"}\n' >"$review_brief_path"
fi

if [[ "$review_exit" -ne 0 ]]; then
  echo "pre-push review guard could not run review-brief." >&2
  if [[ -s "$stderr_path" ]]; then
    cat "$stderr_path" >&2
  fi
  exit "$review_exit"
fi

python3 - "$review_brief_path" "$report_path" "$review_output" "$json_mode" <<'PY'
import json
import sys
from pathlib import Path

review_brief_path = Path(sys.argv[1])
report_path = Path(sys.argv[2])
review_output = sys.argv[3]
review_output_path = Path(review_output)
review_output_abs = str(review_output_path if review_output_path.is_absolute() else Path.cwd() / review_output_path)
review_output_label = review_output_path.name or "framework-pr-review.md"
review_output_link = f"[{review_output_label}]({review_output_abs})"
json_mode = sys.argv[4] == "true"

try:
    report = json.loads(review_brief_path.read_text(encoding="utf-8"))
except Exception as exc:
    print(f"pre-push review guard could not parse {review_brief_path}: {exc}", file=sys.stderr)
    raise SystemExit(2)

pre_push = report.get("pre_push_status") if isinstance(report, dict) else None
if not isinstance(pre_push, dict):
    print("pre-push review guard did not find pre_push_status in review-brief output.", file=sys.stderr)
    raise SystemExit(2)

satisfied = bool(pre_push.get("satisfied"))
reason = str(pre_push.get("reason") or "no reason was reported")
slash_command = str(report.get("slash_command") or "/framework-pr-review")
required_depth = str(pre_push.get("required_review_depth") or report.get("required_review_depth") or "focused")
comprehensive_reasons = pre_push.get("comprehensive_review_reasons") or []
recommendation = pre_push.get("recommendation_summary")
if not isinstance(recommendation, dict):
    recommendation = {}
severity_counts = recommendation.get("severity_counts")
if not isinstance(severity_counts, dict):
    severity_counts = {}
attention_items = pre_push.get("attention_items")
if not isinstance(attention_items, list):
    attention_items = []
attention_items = [str(item) for item in attention_items if str(item).strip()]

severity_buckets = ["blockers", "critical", "important", "should_address", "nice_to_address"]

def count_display(bucket):
    value = severity_counts.get(bucket)
    return str(value) if isinstance(value, int) else "missing"

status = str(recommendation.get("status") or "missing")
conditions_value = recommendation.get("conditions_captured_value")
if conditions_value is None:
    if recommendation.get("conditions_not_applicable"):
        conditions_display = "not_applicable"
    elif recommendation.get("conditions_captured"):
        conditions_display = "yes"
    else:
        conditions_display = "missing"
else:
    conditions_display = str(conditions_value)

review_summary = {
    "status": status,
    "severity_counts": {bucket: severity_counts.get(bucket) for bucket in severity_buckets},
    "conditions_captured": conditions_display,
    "required_review_depth": required_depth,
    "attention_items": attention_items,
    "standing_push_approval_applies": bool(pre_push.get("standing_push_approval_applies")),
    "human_approval_recorded": bool(pre_push.get("human_approval_recorded")),
    "full_review": review_output_abs,
    "full_review_link": review_output_link,
}
summary = {
    "command": "pre-push-review",
    "ok": satisfied,
    "review_brief": str(report_path),
    "review_brief_detail": str(review_brief_path),
    "review_output": review_output,
    "review_output_abs": review_output_abs,
    "review_summary": review_summary,
    "slash_command": slash_command,
    "required_review_depth": required_depth,
    "comprehensive_review_reasons": comprehensive_reasons,
    "reason": reason,
    "remote_name": report.get("remote_name") or None,
    "local_ref": None,
    "remote_ref": None,
    "pre_push_status": pre_push,
}
summary["review_brief"] = str(review_brief_path)
summary["local_ref"] = __import__("os").environ.get("APPFW_PRE_PUSH_LOCAL_REF")
summary["remote_ref"] = __import__("os").environ.get("APPFW_PRE_PUSH_REMOTE_REF")
summary["local_sha"] = __import__("os").environ.get("APPFW_PRE_PUSH_LOCAL_SHA")
summary["remote_sha"] = __import__("os").environ.get("APPFW_PRE_PUSH_REMOTE_SHA")
summary["remote_name"] = __import__("os").environ.get("APPFW_PRE_PUSH_REMOTE_NAME")
report_path.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")

def print_review_summary():
    counts = ", ".join(f"{bucket}={count_display(bucket)}" for bucket in severity_buckets)
    print("Review summary:", file=sys.stderr)
    print(f"- status: {status}", file=sys.stderr)
    print(f"- severity counts: {counts}", file=sys.stderr)
    print(f"- conditions captured: {conditions_display}", file=sys.stderr)
    print(f"- required depth: {required_depth}", file=sys.stderr)
    if attention_items:
        print("- attention items:", file=sys.stderr)
        for item in attention_items[:5]:
            print(f"  - {item}", file=sys.stderr)
        if len(attention_items) > 5:
            print(f"  - plus {len(attention_items) - 5} more in the full review", file=sys.stderr)
    print(f"- full review: {review_output_link}", file=sys.stderr)

if satisfied:
    if json_mode:
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        print(f"pre-push review gate satisfied: {reason}", file=sys.stderr)
        print_review_summary()
    raise SystemExit(0)

if json_mode:
    print(json.dumps(summary, indent=2, sort_keys=True))

print("", file=sys.stderr)
print("Push blocked: Framework PR Review Agent evidence is missing, stale, or not approved.", file=sys.stderr)
print(f"Reason: {reason}", file=sys.stderr)
print_review_summary()
print(f"Review brief artifact: {report_path}", file=sys.stderr)
print(f"Expected review output: {review_output}", file=sys.stderr)
print(f"Open review output: {review_output_abs}", file=sys.stderr)
print("", file=sys.stderr)
print("Before pushing this branch:", file=sys.stderr)
print("1. Run `scripts/appfw framework handoff --json`.", file=sys.stderr)
print(f"2. Run `{slash_command}`.", file=sys.stderr)
print(f"3. Save the structured review output at `{review_output}`.", file=sys.stderr)
print("4. Re-run `scripts/ci/pre-push-review-guard.sh`.", file=sys.stderr)
print("5. Push again.", file=sys.stderr)
if required_depth == "comprehensive" and comprehensive_reasons:
    print("", file=sys.stderr)
    print("Comprehensive review is required because:", file=sys.stderr)
    for item in comprehensive_reasons[:8]:
        print(f"- {item}", file=sys.stderr)
    if len(comprehensive_reasons) > 8:
        print(f"- plus {len(comprehensive_reasons) - 8} more", file=sys.stderr)
print("", file=sys.stderr)
print("A human may explicitly bypass one push with:", file=sys.stderr)
print('APPFW_PRE_PUSH_REVIEW_BYPASS_REASON="reason" git push', file=sys.stderr)
raise SystemExit(1)
PY
