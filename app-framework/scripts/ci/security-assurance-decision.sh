#!/usr/bin/env bash
#
# security-assurance-decision.sh - formal release decision for security
# assurance evidence that is outside the normal SCA/secret/SBOM gates.
#
# The gate is intentionally evidence-or-risk-acceptance based. A regulated
# production release should provide real artifacts for DAST, formal SAST,
# ASVS traceability, release provenance, and artifact signing. Lower-risk lanes
# may provide a structured, time-boxed risk acceptance instead.
#
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
cd "$repo_root"

report_dir="${APPFW_RELEASE_ARTIFACT_DIR:-target/appfw}"
decision_file="$report_dir/security-assurance-decision.json"
risk_acceptance_file="${APPFW_SECURITY_RISK_ACCEPTANCE:-$report_dir/security-risk-acceptance.json}"
requirements="${APPFW_SECURITY_ASSURANCE_REQUIREMENTS:-dast,sast,asvs,release-provenance,artifact-signing}"
mkdir -p "$report_dir"

ensure_python3() {
  if command -v python3 >/dev/null 2>&1; then
    return 0
  fi

  echo "security-assurance-decision: python3 is required to validate release evidence" >&2
  exit 2
}

ensure_python3

python3 - "$report_dir" "$decision_file" "$risk_acceptance_file" "$requirements" <<'PY'
import datetime as dt
import json
import os
import sys
from pathlib import Path

report_dir = Path(sys.argv[1])
decision_file = Path(sys.argv[2])
risk_acceptance_path = Path(sys.argv[3])
raw_requirements = sys.argv[4]
production_attestations_required = (
    str(os.environ.get("APPFW_REQUIRE_PRODUCTION_ATTESTATIONS", "")).strip().lower()
    in {"1", "true", "yes", "on"}
    or str(os.environ.get("APPFW_RELEASE_REQUIRE_PRODUCTION_ATTESTATIONS", "")).strip().lower()
    in {"1", "true", "yes", "on"}
)
try:
    risk_acceptance_max_days = int(os.environ.get("APPFW_SECURITY_RISK_ACCEPTANCE_MAX_DAYS", "90"))
except ValueError:
    raise SystemExit("APPFW_SECURITY_RISK_ACCEPTANCE_MAX_DAYS must be an integer")
if risk_acceptance_max_days <= 0:
    raise SystemExit("APPFW_SECURITY_RISK_ACCEPTANCE_MAX_DAYS must be greater than zero")

DEFAULT_EVIDENCE = {
    "dast": [
        "dast-evidence.json",
        "dast-report.json",
        "zap-report.json",
    ],
    "sast": [
        "sast-evidence.json",
        "sast.sarif",
        "codeql.sarif",
        "semgrep.sarif",
    ],
    "asvs": [
        "asvs-traceability.json",
    ],
    "release-provenance": [
        "release-provenance.intoto.jsonl",
        "release-provenance.json",
        "slsa-provenance.intoto.jsonl",
    ],
    "artifact-signing": [
        "artifact-signing.json",
        "cosign-signing.json",
        "signing-evidence.json",
    ],
}

REGULATED_RELEASE_REQUIREMENTS = list(DEFAULT_EVIDENCE.keys())


def utc_now():
    return (
        dt.datetime.now(dt.timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z")
    )


def normalize_category(value):
    return str(value or "").strip().lower().replace("_", "-")


def non_empty_string(value):
    return isinstance(value, str) and bool(value.strip())


def parse_requirements(value):
    parsed = [
        normalize_category(item)
        for item in value.replace("\n", ",").split(",")
        if normalize_category(item)
    ]
    return parsed or list(DEFAULT_EVIDENCE.keys())


def parse_date(value):
    if not isinstance(value, str) or not value.strip():
        return None
    text = value.strip().replace("Z", "+00:00")
    try:
        return dt.datetime.fromisoformat(text).date()
    except ValueError:
        try:
            return dt.date.fromisoformat(text[:10])
        except ValueError:
            return None


def sha256(path):
    import hashlib

    return hashlib.sha256(path.read_bytes()).hexdigest()


def candidate_paths(category):
    env_name = "APPFW_SECURITY_ASSURANCE_" + category.upper().replace("-", "_") + "_ARTIFACTS"
    configured = [
        item.strip()
        for item in os.environ.get(env_name, "").replace("\n", ",").split(",")
        if item.strip()
    ]
    defaults = DEFAULT_EVIDENCE.get(category, [])
    paths = []
    for item in [*configured, *defaults]:
        path = Path(item)
        if not path.is_absolute():
            path = report_dir / path
        paths.append(path)
    return paths


def validate_json_file(path):
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except Exception as exc:
        return False, f"invalid JSON: {exc}"
    if isinstance(value, dict) and value.get("ok") is False:
        return False, "artifact ok flag is false"
    return True, None


def validate_jsonl_file(path):
    lines = [line for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]
    if not lines:
        return False, "JSONL artifact is empty"
    for index, line in enumerate(lines, start=1):
        try:
            json.loads(line)
        except Exception as exc:
            return False, f"invalid JSONL line {index}: {exc}"
    return True, None


def validate_sarif_file(path):
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except Exception as exc:
        return False, f"invalid SARIF JSON: {exc}"
    if not isinstance(value, dict) or "runs" not in value:
        return False, "SARIF artifact must contain runs"
    return True, None


def is_local_attestation_artifact(category, value):
    if category not in {"release-provenance", "artifact-signing"}:
        return False, None
    if not isinstance(value, dict):
        return False, None

    command = str(value.get("command") or "")
    scope = str(value.get("scope") or "")
    builder = value.get("builder") if isinstance(value.get("builder"), dict) else {}
    builder_id = str(builder.get("id") or "")
    marker_fields = [command, scope, builder_id]
    if any("local" in field.lower() for field in marker_fields):
        return True, (
            f"{category} is local evidence; production attestations require "
            "enterprise signing/provenance or a structured risk acceptance"
        )
    if value.get("enterprise_replacement"):
        return True, (
            f"{category} declares itself replaceable by enterprise evidence; "
            "production attestations require that replacement artifact"
        )
    notes = value.get("notes")
    if isinstance(notes, list) and any("local" in str(note).lower() for note in notes):
        return True, (
            f"{category} notes local provenance; production attestations require "
            "enterprise signing/provenance or a structured risk acceptance"
        )
    return False, None


def evidence_artifact(path, category):
    suffix = "".join(path.suffixes)
    parsed_json = None
    if suffix.endswith(".jsonl"):
        ok, detail = validate_jsonl_file(path)
    elif path.suffix == ".sarif":
        ok, detail = validate_sarif_file(path)
    elif path.suffix == ".json":
        ok, detail = validate_json_file(path)
        if ok:
            parsed_json = json.loads(path.read_text(encoding="utf-8"))
            if production_attestations_required:
                is_local, local_detail = is_local_attestation_artifact(category, parsed_json)
                if is_local:
                    ok = False
                    detail = local_detail
    else:
        ok, detail = True, None
    artifact = {
        "path": path.as_posix(),
        "present": True,
        "ok": ok,
        "bytes": path.stat().st_size,
        "sha256": f"sha256:{sha256(path)}",
    }
    if detail:
        artifact["detail"] = detail
    return artifact


def load_risk_acceptance():
    if not risk_acceptance_path.is_file():
        return None, {
            "path": risk_acceptance_path.as_posix(),
            "present": False,
            "valid": False,
            "detail": "risk acceptance artifact is absent",
        }
    try:
        value = json.loads(risk_acceptance_path.read_text(encoding="utf-8"))
    except Exception as exc:
        return None, {
            "path": risk_acceptance_path.as_posix(),
            "present": True,
            "valid": False,
            "detail": f"invalid JSON: {exc}",
        }

    owner = value.get("owner") or value.get("risk_owner")
    approver = value.get("approved_by") or value.get("approver")
    expires_on = value.get("expires_on") or value.get("expires_at")
    release_scope = value.get("release_scope") or value.get("scope")
    expiry_date = parse_date(expires_on)
    today = dt.datetime.now(dt.timezone.utc).date()
    max_expiry_date = today + dt.timedelta(days=risk_acceptance_max_days)
    items = value.get("items")
    if items is None:
        items = value.get("risk_acceptances")
    if not isinstance(items, list):
        items = []
    item_categories = [
        normalize_category(item.get("category") or item.get("control"))
        for item in items
        if isinstance(item, dict)
    ]
    unknown_categories = sorted(
        {
            category
            for category in item_categories
            if category and category not in DEFAULT_EVIDENCE
        }
    )
    top_valid = (
        value.get("accepted") is True
        or value.get("ok") is True
        or value.get("status") == "accepted"
    )
    valid = bool(
        top_valid
        and non_empty_string(owner)
        and non_empty_string(approver)
        and non_empty_string(release_scope)
        and expiry_date
        and expiry_date >= today
        and expiry_date <= max_expiry_date
        and items
        and not unknown_categories
    )
    detail = None
    if not top_valid:
        detail = "top-level accepted/ok/status field does not approve the acceptance"
    elif not non_empty_string(owner):
        detail = "owner or risk_owner must be a non-empty string"
    elif not non_empty_string(approver):
        detail = "approved_by or approver must be a non-empty string"
    elif not non_empty_string(release_scope):
        detail = "release_scope or scope must be a non-empty string"
    elif not expiry_date:
        detail = "expires_on or expires_at must be an ISO date"
    elif expiry_date < today:
        detail = "risk acceptance is expired"
    elif expiry_date > max_expiry_date:
        detail = f"risk acceptance expires more than {risk_acceptance_max_days} days from now"
    elif not items:
        detail = "risk acceptance requires at least one accepted item"
    elif unknown_categories:
        detail = f"risk acceptance contains unknown categories: {unknown_categories}"

    return value, {
        "path": risk_acceptance_path.as_posix(),
        "present": True,
        "valid": valid,
        "owner": owner,
        "approved_by": approver,
        "release_scope": release_scope,
        "expires_on": expires_on,
        "max_days": risk_acceptance_max_days,
        "detail": detail,
    }


def acceptance_items(value):
    if not isinstance(value, dict):
        return []
    items = value.get("items")
    if items is None:
        items = value.get("risk_acceptances")
    return items if isinstance(items, list) else []


def accepted_item_for(category, risk_acceptance, risk_summary):
    if not risk_summary.get("valid"):
        return None, risk_summary.get("detail") or "risk acceptance is invalid"
    for item in acceptance_items(risk_acceptance):
        if not isinstance(item, dict):
            continue
        item_category = normalize_category(item.get("category") or item.get("control"))
        if item_category != category:
            continue
        accepted = item.get("accepted") is True or item.get("status") == "accepted"
        rationale = item.get("rationale") or item.get("reason")
        controls = item.get("compensating_controls") or item.get("controls")
        follow_up = item.get("follow_up") or item.get("remediation_plan")
        if not accepted:
            return None, f"{category} risk acceptance item is not accepted"
        if not isinstance(rationale, str) or len(rationale.strip()) < 10:
            return None, f"{category} risk acceptance requires a rationale"
        if not isinstance(controls, list) or not controls or not all(non_empty_string(control) for control in controls):
            return None, f"{category} risk acceptance requires compensating controls"
        if not isinstance(follow_up, str) or len(follow_up.strip()) < 10:
            return None, f"{category} risk acceptance requires follow_up or remediation_plan"
        return {
            "category": category,
            "accepted": True,
            "owner": risk_summary.get("owner"),
            "approved_by": risk_summary.get("approved_by"),
            "release_scope": risk_summary.get("release_scope"),
            "expires_on": risk_summary.get("expires_on"),
            "rationale": rationale,
            "compensating_controls": controls,
            "follow_up": follow_up,
        }, None
    return None, f"{category} has no matching risk acceptance item"


requirements = parse_requirements(raw_requirements)
if production_attestations_required:
    for required_category in REGULATED_RELEASE_REQUIREMENTS:
        if required_category not in requirements:
            requirements.append(required_category)
risk_acceptance, risk_summary = load_risk_acceptance()
categories = []
failures = []
artifacts = []
rejected_artifacts = []

for category in requirements:
    candidates = candidate_paths(category)
    present = [evidence_artifact(path, category) for path in candidates if path.is_file()]
    valid_evidence = [artifact for artifact in present if artifact["ok"]]
    rejected_artifacts.extend(
        {
            "name": f"{category}-evidence",
            "path": artifact["path"],
            "present": True,
            "required": True,
            "sha256": artifact["sha256"],
            "bytes": artifact["bytes"],
            "reason": artifact.get("detail") or "artifact did not satisfy release evidence checks",
        }
        for artifact in present
        if not artifact["ok"]
    )
    artifacts.extend(
        {
            "name": f"{category}-evidence",
            "path": artifact["path"],
            "present": True,
            "required": True,
            "sha256": artifact["sha256"],
            "bytes": artifact["bytes"],
        }
        for artifact in valid_evidence
    )
    if valid_evidence:
        categories.append(
            {
                "category": category,
                "ok": True,
                "disposition": "evidence",
                "evidence_artifacts": valid_evidence,
                "candidate_paths": [path.as_posix() for path in candidates],
            }
        )
        continue

    accepted, detail = accepted_item_for(category, risk_acceptance, risk_summary)
    if accepted:
        categories.append(
            {
                "category": category,
                "ok": True,
                "disposition": "risk-accepted",
                "risk_acceptance": accepted,
                "candidate_paths": [path.as_posix() for path in candidates],
            }
        )
        continue

    failure = (
        f"{category} requires a valid evidence artifact or a structured, "
        f"time-boxed risk acceptance ({detail})"
    )
    failures.append(failure)
    categories.append(
        {
            "category": category,
            "ok": False,
            "disposition": "missing",
            "candidate_paths": [path.as_posix() for path in candidates],
            "present_artifacts": present,
            "detail": failure,
        }
    )

if risk_summary.get("present"):
    artifacts.append(
        {
            "name": "security-risk-acceptance",
            "path": risk_summary["path"],
            "present": True,
            "required": False,
            "sha256": f"sha256:{sha256(risk_acceptance_path)}",
            "bytes": risk_acceptance_path.stat().st_size,
        }
    )

evidence = {
    "command": "security-assurance-decision",
    "ok": not failures,
    "release_ready": not failures,
    "generated_at_utc": utc_now(),
    "requirements": requirements,
    "production_attestations_required": production_attestations_required,
    "categories": categories,
    "risk_acceptance": risk_summary,
    "artifacts": artifacts,
    "rejected_artifacts": rejected_artifacts,
    "failure_count": len(failures),
    "failures": failures,
}

decision_file.write_text(
    json.dumps(evidence, indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
)

if failures:
    print("security-assurance-decision: FAILED", file=sys.stderr)
    for failure in failures:
        print(f"  - {failure}", file=sys.stderr)
    print(f"security-assurance-decision: evidence written to {decision_file}", file=sys.stderr)
    sys.exit(1)

print(f"security-assurance-decision: OK; evidence written to {decision_file}")
PY
