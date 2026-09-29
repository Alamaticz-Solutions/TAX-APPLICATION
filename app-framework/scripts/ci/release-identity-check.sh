#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
cd "$repo_root"

report_dir="${APPFW_RELEASE_ARTIFACT_DIR:-target/appfw}"
artifact_file="$report_dir/release-identity.json"
mkdir -p "$report_dir"

python3 - "$repo_root" "$report_dir" "$artifact_file" <<'PY'
import datetime as dt
import glob
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

repo_root = Path(sys.argv[1])
report_dir = Path(sys.argv[2])
artifact_file = Path(sys.argv[3])


def utc_now():
    return (
        dt.datetime.now(dt.timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z")
    )


def env_bool(name):
    return str(os.environ.get(name, "")).strip().lower() in {
        "1",
        "true",
        "yes",
        "y",
        "on",
        "required",
        "require",
    }


def run_git(*args):
    try:
        result = subprocess.run(
            ["git", *args],
            cwd=repo_root,
            text=True,
            capture_output=True,
            check=True,
        )
    except Exception:
        return ""
    return result.stdout.strip()


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return "sha256:" + digest.hexdigest()


def parse_timestamp(value):
    if not isinstance(value, str) or not value.strip():
        return None
    try:
        parsed = dt.datetime.fromisoformat(value.strip().replace("Z", "+00:00"))
    except ValueError:
        return None
    if parsed.tzinfo is None:
        return None
    return parsed.astimezone(dt.timezone.utc)


def resolve_path(value):
    if not isinstance(value, str) or not value.strip():
        return None
    candidate = Path(value.strip()).expanduser()
    if candidate.is_absolute():
        return candidate if candidate.exists() else None
    for base in (repo_root, report_dir):
        resolved = base / candidate
        if resolved.exists():
            return resolved
    return None


def root_relative(path):
    try:
        return path.resolve().relative_to(repo_root.resolve()).as_posix()
    except ValueError:
        return path.as_posix()


def path_is_within_directory(path, parent):
    try:
        path.resolve().relative_to(parent.resolve())
        return True
    except ValueError:
        return False


def read_json(path):
    try:
        with path.open("r", encoding="utf-8") as handle:
            return json.load(handle), None
    except Exception as exc:
        return None, str(exc)


def package_license_metadata():
    manifests = [
        "Cargo.toml",
        "app_gen/Cargo.toml",
        "appfw_cli/Cargo.toml",
        "appfw_runtime/Cargo.toml",
        "appfw_ui/pds_health/package.json",
    ]
    results = []
    for manifest in manifests:
        path = repo_root / manifest
        if not path.is_file():
            continue
        if path.suffix == ".json":
            data, error = read_json(path)
            license_value = data.get("license") if isinstance(data, dict) else None
            publish_value = data.get("private") if isinstance(data, dict) else None
            results.append(
                {
                    "path": manifest,
                    "license": license_value if isinstance(license_value, str) else None,
                    "private": publish_value if isinstance(publish_value, bool) else None,
                }
            )
            continue
        text = path.read_text(encoding="utf-8")
        package_match = re.search(r"(?ms)^\[package\]\s*(.*?)(?:^\[|\Z)", text)
        package_text = package_match.group(1) if package_match else text
        license_match = re.search(r'(?m)^\s*license\s*=\s*"([^"]+)"', package_text)
        license_file_match = re.search(r'(?m)^\s*license-file\s*=\s*"([^"]+)"', package_text)
        publish_match = re.search(r"(?m)^\s*publish\s*=\s*(false|true)", package_text)
        results.append(
            {
                "path": manifest,
                "license": license_match.group(1) if license_match else None,
                "license_file": license_file_match.group(1) if license_file_match else None,
                "publish": publish_match.group(1) if publish_match else None,
            }
        )
    return results


def add_check(checks, name, ok, detail, blocking=True, **extra):
    item = {
        "name": name,
        "ok": bool(ok),
        "blocking": bool(blocking),
        "detail": detail,
    }
    item.update(extra)
    checks.append(item)


release_identity_required = env_bool("APPFW_RELEASE_REQUIRE_RELEASE_IDENTITY")
bitbucket_tag = os.environ.get("BITBUCKET_TAG", "").strip()
exact_tags = [
    tag
    for tag in run_git("tag", "--points-at", "HEAD", "--list", "v*").splitlines()
    if tag.strip()
]
git_sha = run_git("rev-parse", "HEAD")
current_tag = bitbucket_tag if re.fullmatch(r"v[0-9][A-Za-z0-9._+-]*", bitbucket_tag) else ""
if not current_tag and exact_tags:
    current_tag = sorted(exact_tags)[0]
if current_tag:
    release_identity_required = True

license_candidates = [
    Path(path)
    for pattern in ("LICENSE", "LICENSE.*", "NOTICE", "NOTICE.*", "COPYING", "COPYING.*")
    for path in glob.glob(str(repo_root / pattern))
]
license_candidates = sorted({path.resolve() for path in license_candidates if path.is_file()})

decision_file = Path(
    os.environ.get(
        "APPFW_RELEASE_IDENTITY_DECISION_FILE",
        str(report_dir / "release-identity-decision.json"),
    )
)
if not decision_file.is_absolute():
    decision_file = repo_root / decision_file
decision_present = decision_file.is_file()
decision, decision_error = read_json(decision_file) if decision_present else (None, None)

checks = []
blockers = []

add_check(
    checks,
    "release-identity-required",
    release_identity_required,
    (
        "release identity evidence is required by release mode"
        if release_identity_required
        else "set APPFW_RELEASE_REQUIRE_RELEASE_IDENTITY=true in production release gates"
    ),
)

tag_ok = bool(current_tag)
add_check(
    checks,
    "v-tag",
    tag_ok,
    (
        f"release tag {current_tag} is attached to HEAD"
        if tag_ok
        else "HEAD is not associated with a v* release tag and BITBUCKET_TAG is not v*"
    ),
    git_sha=git_sha or None,
    bitbucket_tag=bitbucket_tag or None,
    exact_v_tags=exact_tags,
)

license_ok = bool(license_candidates)
add_check(
    checks,
    "license-or-notice-file",
    license_ok,
    (
        "root license/notice file is present"
        if license_ok
        else "root LICENSE*, NOTICE*, or COPYING* file is missing"
    ),
    files=[root_relative(path) for path in license_candidates],
)

package_metadata = package_license_metadata()
missing_license_metadata = [
    item["path"] for item in package_metadata if not (item.get("license") or item.get("license_file"))
]
package_metadata_ok = not missing_license_metadata and bool(package_metadata)
add_check(
    checks,
    "package-license-metadata",
    package_metadata_ok,
    (
        "package manifests carry license metadata"
        if package_metadata_ok
        else "package manifests are missing license metadata"
    ),
    manifests=package_metadata,
    missing_license_metadata=missing_license_metadata,
)

changelog_file = repo_root / "CHANGELOG.md"
changelog_text = changelog_file.read_text(encoding="utf-8") if changelog_file.is_file() else ""
changelog_has_tag = bool(current_tag and re.search(rf"(?m)^##\s+\[?{re.escape(current_tag.lstrip('v'))}\]?", changelog_text))
changelog_unreleased_only = "## Unreleased" in changelog_text and not changelog_has_tag

decision_ok = False
decision_release_authority_ok = False
decision_tag_ok = False
decision_license_ok = False
decision_release_notes_ok = False
decision_distribution_ok = False
decision_artifacts = []

if not decision_present:
    decision_detail = f"release identity decision file is missing: {decision_file.as_posix()}"
elif decision_error:
    decision_detail = f"release identity decision file cannot be parsed: {decision_error}"
elif not isinstance(decision, dict):
    decision_detail = "release identity decision file must contain a JSON object"
else:
    decision_ok = decision.get("ok") is True and decision.get("release_ready") is True
    decision_detail = (
        "release identity decision declares ok:true and release_ready:true"
        if decision_ok
        else "release identity decision must declare ok:true and release_ready:true"
    )
    authority = decision.get("release_authority")
    if not isinstance(authority, dict):
        authority = decision.get("authority") if isinstance(decision.get("authority"), dict) else {}
    owner = authority.get("owner") or authority.get("release_owner") or decision.get("owner")
    approver = authority.get("approver") or authority.get("approved_by") or decision.get("approver")
    approved_at = (
        authority.get("approved_at_utc")
        or authority.get("approved_at")
        or decision.get("approved_at_utc")
        or decision.get("approved_at")
    )
    parsed_approved_at = parse_timestamp(approved_at)
    decision_release_authority_ok = (
        isinstance(owner, str)
        and bool(owner.strip())
        and isinstance(approver, str)
        and bool(approver.strip())
        and parsed_approved_at is not None
        and parsed_approved_at <= dt.datetime.now(dt.timezone.utc) + dt.timedelta(minutes=5)
    )

    release = decision.get("release") if isinstance(decision.get("release"), dict) else decision
    decision_tag = release.get("git_tag") or release.get("tag") or release.get("version")
    decision_tag_ok = isinstance(decision_tag, str) and bool(current_tag) and decision_tag.strip() == current_tag

    license_file_value = release.get("license_file") or release.get("license_path")
    license_file = resolve_path(license_file_value)
    decision_license_ok = license_file is not None and license_file.is_file()

    release_notes_value = release.get("release_notes") or release.get("release_notes_file")
    release_notes = resolve_path(release_notes_value)
    release_notes_url = release.get("release_notes_url")
    decision_release_notes_ok = (
        (release_notes is not None and release_notes.is_file())
        or (isinstance(release_notes_url, str) and release_notes_url.startswith(("https://", "http://")))
        or changelog_has_tag
    )

    distribution_artifacts = release.get("distribution_artifacts")
    if not isinstance(distribution_artifacts, list):
        distribution_artifacts = decision.get("distribution_artifacts")
    if not isinstance(distribution_artifacts, list):
        distribution_artifacts = []
    distribution_ok_items = []
    for index, item in enumerate(distribution_artifacts):
        if not isinstance(item, dict):
            decision_artifacts.append({"index": index, "ok": False, "detail": "artifact entry is not an object"})
            continue
        name = item.get("name")
        url = item.get("url")
        path_value = item.get("path")
        path = resolve_path(path_value)
        item_ok = isinstance(name, str) and bool(name.strip()) and (
            (isinstance(url, str) and url.startswith(("https://", "http://")))
            or (path is not None and path.is_file())
        )
        artifact_record = {
            "index": index,
            "name": name,
            "ok": item_ok,
            "url": url if isinstance(url, str) else None,
            "path": root_relative(path) if path is not None and path.exists() else path_value,
        }
        if path is not None and path.is_file():
            artifact_record["sha256"] = sha256(path)
            artifact_record["bytes"] = path.stat().st_size
        decision_artifacts.append(artifact_record)
        if item_ok:
            distribution_ok_items.append(item)
    decision_distribution_ok = bool(distribution_ok_items)

add_check(checks, "decision-artifact", decision_ok, decision_detail, decision_file=decision_file.as_posix())
decision_retained = (not decision_present) or path_is_within_directory(decision_file, report_dir)
add_check(
    checks,
    "decision-artifact-retained",
    decision_retained,
    (
        f"release identity decision file is retained under {report_dir.as_posix()}"
        if decision_present and decision_retained
        else (
            "release identity decision retention will be checked once the decision file is present"
            if not decision_present
            else f"release identity decision file must be retained under {report_dir.as_posix()}: {decision_file.as_posix()}"
        )
    ),
)
add_check(
    checks,
    "release-authority",
    decision_release_authority_ok,
    (
        "decision artifact includes owner, approver, and valid approval timestamp"
        if decision_release_authority_ok
        else "decision artifact must include owner, approver, and approved_at_utc"
    ),
)
add_check(
    checks,
    "decision-tag-match",
    decision_tag_ok,
    (
        "decision artifact git tag matches the current release tag"
        if decision_tag_ok
        else "decision artifact must name the current v* release tag"
    ),
    current_tag=current_tag or None,
)
add_check(
    checks,
    "decision-license",
    decision_license_ok,
    (
        "decision artifact references a retained license file"
        if decision_license_ok
        else "decision artifact must reference a retained license_file"
    ),
)
add_check(
    checks,
    "release-notes",
    decision_release_notes_ok,
    (
        "release notes are retained or linked for the current release"
        if decision_release_notes_ok
        else "decision artifact must reference retained release notes or the changelog must contain the release tag"
    ),
    changelog=changelog_file.as_posix(),
    changelog_unreleased_only=changelog_unreleased_only,
)
add_check(
    checks,
    "distribution-artifacts",
    decision_distribution_ok,
    (
        "decision artifact records at least one consumable distribution artifact"
        if decision_distribution_ok
        else "decision artifact must record at least one distribution artifact with name and URL or retained path"
    ),
    artifacts=decision_artifacts,
)

for check in checks:
    if check.get("blocking") and check.get("ok") is not True:
        blockers.append(check["detail"])

remediation_work_items = [] if not blockers else [
    {
        "id": "LIVE-004",
        "priority": "P0",
        "owner": "Release management + CI/CD",
        "summary": "Cut the first release candidate through the v* tag lane.",
        "doc": "docs/release/live-environment-work-items.md#p0-release-blockers",
    },
    {
        "id": "LIVE-014",
        "priority": "P2",
        "owner": "Legal + product leadership",
        "summary": "Select and approve the framework license for downstream consumers.",
        "doc": "docs/release/live-environment-work-items.md#p2-follow-up",
    },
]

ok = not blockers
release_ready = ok and release_identity_required
if ok and not release_identity_required:
    blockers.append("release identity evidence was not required; set APPFW_RELEASE_REQUIRE_RELEASE_IDENTITY=true in production release gates")
    release_ready = False
    remediation_work_items = [
        {
            "id": "LIVE-004",
            "priority": "P0",
            "owner": "Release management + CI/CD",
            "summary": "Cut the first release candidate through the v* tag lane.",
            "doc": "docs/release/live-environment-work-items.md#p0-release-blockers",
        },
        {
            "id": "LIVE-014",
            "priority": "P2",
            "owner": "Legal + product leadership",
            "summary": "Select and approve the framework license for downstream consumers.",
            "doc": "docs/release/live-environment-work-items.md#p2-follow-up",
        },
    ]

evidence = {
    "command": "release-identity",
    "ok": ok,
    "release_ready": release_ready,
    "generated_at_utc": utc_now(),
    "release_identity_required": release_identity_required,
    "git": {
        "sha": git_sha or None,
        "bitbucket_tag": bitbucket_tag or None,
        "exact_v_tags": exact_tags,
        "release_tag": current_tag or None,
    },
    "decision_file": decision_file.as_posix(),
    "decision_artifact": {
        "path": decision_file.as_posix(),
        "present": decision_present,
        "ok": decision_ok,
    },
    "license_files": [root_relative(path) for path in license_candidates],
    "package_metadata": package_metadata,
    "checks": checks,
    "release_blockers": blockers,
    "failure_summary": {
        "root_causes": blockers,
    },
    "remediation_work_items": remediation_work_items,
}
if decision_present:
    evidence["decision_artifact"]["sha256"] = sha256(decision_file)
    evidence["decision_artifact"]["bytes"] = decision_file.stat().st_size

artifact_file.write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n", encoding="utf-8")

if release_ready:
    print(f"release-identity: OK; release_ready=true; evidence written to {artifact_file}")
elif ok:
    print(f"release-identity: OK; release_ready=false; evidence written to {artifact_file}")
else:
    print(f"release-identity: FAILED; evidence written to {artifact_file}", file=sys.stderr)
    for blocker in blockers:
        print(f"  - {blocker}", file=sys.stderr)
sys.exit(0 if ok else 1)
PY
