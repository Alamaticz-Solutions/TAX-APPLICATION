#!/usr/bin/env python3
"""Build the bounded, hash-bound evidence artifact for the PR Fast gate.

The Fast gate previously uploaded all of ``target/appfw``. That root can also
contain caches, generated workspaces, and package payloads, so this packager
copies only review evidence into a dedicated staging root and writes a manifest
that binds every staged file to the tested Git SHA.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


SCHEMA = "appfw_pr_fast_evidence_manifest@1"
MAX_EVIDENCE_BYTES = 100 * 1024 * 1024
ALLOWED_SUFFIXES = {".json", ".jsonl", ".log", ".md", ".sarif", ".txt", ".xml"}
FRAMEWORK_EVIDENCE_ROOT = Path("target/appfw")
STAGING_ROOT = FRAMEWORK_EVIDENCE_ROOT / "pr-fast-evidence"
MANIFEST_PATH = FRAMEWORK_EVIDENCE_ROOT / "pr-fast-evidence-manifest.json"
PRODUCT_EVIDENCE_ROOT = Path("examples/products/crm/.appfw/target/appfw")
RECURSIVE_EVIDENCE_DIRS = (
    FRAMEWORK_EVIDENCE_ROOT / "feature-check-logs",
    FRAMEWORK_EVIDENCE_ROOT / "wave3-pr-gates",
)
BITBUCKET_IDENTITY_KEYS = (
    "BITBUCKET_BUILD_NUMBER",
    "BITBUCKET_COMMIT",
    "BITBUCKET_PIPELINE_UUID",
    "BITBUCKET_PR_DESTINATION_BRANCH",
    "BITBUCKET_PR_DESTINATION_COMMIT",
    "BITBUCKET_REPO_FULL_NAME",
    "BITBUCKET_STEP_UUID",
)
REQUIRED_SUCCESS_REPORTS = (
    (FRAMEWORK_EVIDENCE_ROOT / "ci-evidence-root.json", "ok"),
    (PRODUCT_EVIDENCE_ROOT / "validation.json", "valid"),
    (FRAMEWORK_EVIDENCE_ROOT / "cli-test.json", "ok"),
    (FRAMEWORK_EVIDENCE_ROOT / "golden-downstream.json", "ok"),
    (FRAMEWORK_EVIDENCE_ROOT / "docs-check-timing.json", "docs_timing"),
    (FRAMEWORK_EVIDENCE_ROOT / "docs-check-changed-surface.json", "json"),
    (FRAMEWORK_EVIDENCE_ROOT / "wave3-pr-gates.json", "ok"),
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).resolve().parents[2],
        help="repository root (defaults to the script's checkout)",
    )
    parser.add_argument(
        "--gate-exit-status",
        type=int,
        required=True,
        help="exit status returned by the complete Fast gate sequence",
    )
    return parser.parse_args()


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def git_head(repo_root: Path) -> str:
    return subprocess.check_output(
        ["git", "rev-parse", "HEAD"],
        cwd=repo_root,
        text=True,
        stderr=subprocess.DEVNULL,
    ).strip()


def git_worktree_changes(repo_root: Path) -> list[str]:
    output = subprocess.check_output(
        ["git", "status", "--porcelain", "--untracked-files=normal"],
        cwd=repo_root,
        text=True,
        stderr=subprocess.DEVNULL,
    )
    return [line for line in output.splitlines() if line.strip()]


def load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def relative_file(repo_root: Path, path: Path) -> Path | None:
    """Return a safe repository-relative regular file, rejecting symlinks."""

    try:
        relative = path.relative_to(repo_root)
    except ValueError:
        return None
    if path.is_symlink() or not path.is_file():
        return None
    resolved = path.resolve()
    try:
        resolved.relative_to(repo_root.resolve())
    except ValueError:
        return None
    return relative


def discover_evidence(repo_root: Path) -> list[Path]:
    """Return the explicit Fast evidence allowlist in deterministic order."""

    discovered: set[Path] = set()
    framework_root = repo_root / FRAMEWORK_EVIDENCE_ROOT

    # Root-level reports/logs are produced by validate, cli-test,
    # golden-downstream, docs-check, feature-check, and their focused helpers.
    if framework_root.is_dir():
        for candidate in framework_root.iterdir():
            if candidate.suffix.lower() not in ALLOWED_SUFFIXES:
                continue
            relative = relative_file(repo_root, candidate)
            if relative is not None and relative != MANIFEST_PATH:
                discovered.add(relative)

    # Wave 3 deliberately captures child stdout/stderr outside the main log;
    # preserve those diagnostics, but no other nested target/appfw directory.
    for relative_dir in RECURSIVE_EVIDENCE_DIRS:
        directory = repo_root / relative_dir
        if not directory.is_dir():
            continue
        for candidate in directory.rglob("*"):
            if candidate.suffix.lower() not in ALLOWED_SUFFIXES:
                continue
            relative = relative_file(repo_root, candidate)
            if relative is not None:
                discovered.add(relative)

    return sorted(discovered, key=lambda item: item.as_posix())


def configured_max_bytes() -> int:
    raw = os.environ.get("APPFW_PR_FAST_EVIDENCE_MAX_BYTES")
    if raw is None or raw.strip() == "":
        return MAX_EVIDENCE_BYTES
    try:
        requested = int(raw)
    except ValueError as exc:
        raise ValueError("APPFW_PR_FAST_EVIDENCE_MAX_BYTES must be an integer") from exc
    if requested <= 0:
        raise ValueError("APPFW_PR_FAST_EVIDENCE_MAX_BYTES must be positive")
    # Test/debug callers may tighten the ceiling, never weaken the repository
    # contract by raising it above 100 MiB.
    return min(requested, MAX_EVIDENCE_BYTES)


def required_success_errors(repo_root: Path, tested_sha: str) -> list[str]:
    errors: list[str] = []
    for relative, contract in REQUIRED_SUCCESS_REPORTS:
        path = repo_root / relative
        if not path.is_file():
            errors.append(f"missing required Fast evidence: {relative.as_posix()}")
            continue
        try:
            payload = load_json(path)
        except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
            errors.append(f"invalid JSON in {relative.as_posix()}: {exc}")
            continue
        if not isinstance(payload, dict):
            errors.append(f"required Fast evidence is not an object: {relative.as_posix()}")
            continue
        if contract == "ok" and payload.get("ok") is not True:
            errors.append(f"required Fast evidence is not green: {relative.as_posix()}")
        elif contract == "valid" and payload.get("valid") is not True:
            errors.append(f"required validation evidence is not valid: {relative.as_posix()}")
        elif contract == "docs_timing":
            if payload.get("ok") is not True:
                errors.append(f"docs-check timing is not green: {relative.as_posix()}")
            if payload.get("budget_ok") is not True:
                errors.append(f"docs-check timing exceeded its budget: {relative.as_posix()}")
            if payload.get("budget_enforced") is not True:
                errors.append(f"docs-check timing was not budget-enforced: {relative.as_posix()}")

    marker_path = repo_root / FRAMEWORK_EVIDENCE_ROOT / "ci-evidence-root.json"
    if marker_path.is_file():
        try:
            marker = load_json(marker_path)
            marker_sha = (marker.get("git") or {}).get("commit")
            if marker_sha != tested_sha:
                errors.append(
                    "ci-evidence-root Git SHA does not match the tested checkout SHA: "
                    f"{marker_sha!r} != {tested_sha!r}"
                )
        except (AttributeError, OSError, UnicodeDecodeError, json.JSONDecodeError):
            # The primary JSON error is already emitted above.
            pass
    return errors


def git_resolve_commit(repo_root: Path, revision: str) -> str | None:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", f"{revision}^{{commit}}"],
        cwd=repo_root,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    if result.returncode != 0:
        return None
    resolved = result.stdout.strip().lower()
    return resolved if re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", resolved) else None


def git_is_ancestor(repo_root: Path, ancestor: str, descendant: str) -> bool:
    return (
        subprocess.run(
            ["git", "merge-base", "--is-ancestor", ancestor, descendant],
            cwd=repo_root,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        ).returncode
        == 0
    )


def destination_identity_matches(full_sha: str, supplied_id: str) -> bool:
    """Accept Bitbucket's 12-char abbreviation; require exact full IDs."""

    if len(supplied_id) == 12:
        return full_sha.startswith(supplied_id)
    return full_sha == supplied_id


def bitbucket_identity(
    repo_root: Path, tested_sha: str
) -> tuple[dict[str, str], str, str | None, str, list[str]]:
    identity = {
        key.lower(): value
        for key in BITBUCKET_IDENTITY_KEYS
        if (value := os.environ.get(key))
    }
    errors: list[str] = []
    bitbucket_sha = os.environ.get("BITBUCKET_COMMIT")
    destination_branch = os.environ.get("BITBUCKET_PR_DESTINATION_BRANCH")
    destination_id = os.environ.get("BITBUCKET_PR_DESTINATION_COMMIT")
    source_sha = tested_sha
    destination_sha: str | None = None
    relation = "local-tested-head"

    if bitbucket_sha:
        if not re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", bitbucket_sha):
            errors.append(
                "BITBUCKET_COMMIT must be a lowercase full 40- or 64-character object ID"
            )
        else:
            resolved_source = git_resolve_commit(repo_root, bitbucket_sha)
            if resolved_source != bitbucket_sha:
                errors.append("BITBUCKET_COMMIT does not resolve to its exact commit")
            else:
                source_sha = resolved_source

    destination_identity_present = bool(destination_branch or destination_id)
    if destination_identity_present:
        if not bitbucket_sha or not destination_branch or not destination_id:
            errors.append(
                "BITBUCKET_COMMIT, BITBUCKET_PR_DESTINATION_BRANCH, and "
                "BITBUCKET_PR_DESTINATION_COMMIT must be present together"
            )
        elif not re.fullmatch(r"[0-9a-f]{12}|[0-9a-f]{40}|[0-9a-f]{64}", destination_id):
            errors.append(
                "BITBUCKET_PR_DESTINATION_COMMIT must be a lowercase 12-, 40-, "
                "or 64-character hexadecimal object ID"
            )
        else:
            destination_sha = git_resolve_commit(repo_root, destination_id)
            if destination_sha is None or not destination_identity_matches(
                destination_sha, destination_id
            ):
                errors.append(
                    "BITBUCKET_PR_DESTINATION_COMMIT does not resolve to its exact commit"
                )

        branch_check = subprocess.run(
            ["git", "check-ref-format", "--branch", destination_branch or ""],
            cwd=repo_root,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        if branch_check.returncode != 0:
            errors.append("BITBUCKET_PR_DESTINATION_BRANCH is not a valid branch name")

        if not errors and destination_sha is not None:
            if tested_sha == source_sha:
                if git_is_ancestor(repo_root, destination_sha, tested_sha):
                    relation = "bitbucket-source-equals-tested-destination-ancestor"
                else:
                    errors.append(
                        "tested HEAD equals BITBUCKET_COMMIT but does not contain "
                        "the exact destination commit"
                    )
            elif tested_sha == destination_sha:
                if git_is_ancestor(repo_root, source_sha, tested_sha):
                    relation = "bitbucket-destination-equals-tested-source-ancestor"
                else:
                    errors.append(
                        "tested HEAD equals the destination but does not contain "
                        "the exact BITBUCKET_COMMIT source"
                    )
            else:
                parent_line = subprocess.run(
                    ["git", "rev-list", "--parents", "-n", "1", tested_sha],
                    cwd=repo_root,
                    text=True,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.DEVNULL,
                    check=False,
                )
                fields = parent_line.stdout.strip().lower().split()
                if (
                    parent_line.returncode == 0
                    and len(fields) == 3
                    and fields[0] == tested_sha
                    and sorted(fields[1:]) == sorted([source_sha, destination_sha])
                ):
                    relation = "bitbucket-tested-merge-of-source-and-destination"
                else:
                    errors.append(
                        "tested HEAD is neither BITBUCKET_COMMIT nor an exact "
                        "two-parent merge of source and destination"
                    )
    elif bitbucket_sha and source_sha != tested_sha:
        errors.append(
            "BITBUCKET_COMMIT does not match tested HEAD outside a complete PR identity: "
            f"{source_sha} != {tested_sha}"
        )

    return identity, source_sha, destination_sha, relation, errors


def stage_evidence(
    repo_root: Path, evidence: list[Path]
) -> tuple[list[dict[str, Any]], int, Path, Path]:
    """Build a complete, unpublished bundle outside Bitbucket's upload glob.

    ``capture-on: always`` can run after any failure. Building directly in the
    published root would therefore expose a partial, unmanifested bundle when
    a copy or hash check fails. Keep the verified copy beside the final root;
    the caller publishes it only after the manifest and hard size ceiling have
    also been finalized.
    """

    staging_root = repo_root / STAGING_ROOT
    temporary_root = staging_root.with_name(f".{staging_root.name}.{os.getpid()}.tmp")
    if staging_root.exists():
        shutil.rmtree(staging_root)
    if temporary_root.exists():
        shutil.rmtree(temporary_root)
    temporary_root.mkdir(parents=True, exist_ok=True)

    artifacts: list[dict[str, Any]] = []
    total_bytes = 0
    try:
        for source_relative in evidence:
            source = repo_root / source_relative
            source_hash = sha256(source)
            source_bytes = source.stat().st_size
            staged_relative = STAGING_ROOT / "files" / source_relative
            staged = temporary_root / "files" / source_relative
            staged.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, staged)
            staged_hash = sha256(staged)
            source_hash_after_copy = sha256(source)
            if (
                staged_hash != source_hash
                or source_hash_after_copy != source_hash
                or staged.stat().st_size != source_bytes
                or source.stat().st_size != source_bytes
            ):
                raise RuntimeError(f"staged evidence changed during copy: {source_relative}")
            artifacts.append(
                {
                    "source_path": source_relative.as_posix(),
                    "artifact_path": staged_relative.as_posix(),
                    "bytes": source_bytes,
                    "sha256": source_hash,
                }
            )
            total_bytes += source_bytes
    except BaseException:
        shutil.rmtree(temporary_root, ignore_errors=True)
        shutil.rmtree(staging_root, ignore_errors=True)
        raise
    return artifacts, total_bytes, temporary_root, staging_root


def stabilize_manifest_size(
    manifest: dict[str, Any],
    total_bytes: int,
    max_bytes: int,
    non_size_root_causes: list[str],
) -> str:
    """Return stable JSON whose byte accounting includes the manifest itself."""

    size_cause_prefix = "Fast evidence artifact is "
    serialized = ""
    for _ in range(12):
        serialized = json.dumps(manifest, indent=2, sort_keys=True) + "\n"
        manifest_bytes = len(serialized.encode("utf-8"))
        artifact_bytes = total_bytes + manifest_bytes
        under_limit = artifact_bytes <= max_bytes
        desired_root_causes = list(non_size_root_causes)
        if not under_limit:
            desired_root_causes.append(
                f"{size_cause_prefix}{artifact_bytes} bytes, above the {max_bytes}-byte ceiling"
            )
        desired_ok = not desired_root_causes
        if all(
            (
                manifest["size"]["manifest_bytes"] == manifest_bytes,
                manifest["size"]["total_uncompressed_bytes"] == artifact_bytes,
                manifest["size"]["under_limit"] == under_limit,
                manifest["root_causes"] == desired_root_causes,
                manifest["ok"] == desired_ok,
            )
        ):
            return serialized
        manifest["size"]["manifest_bytes"] = manifest_bytes
        manifest["size"]["total_uncompressed_bytes"] = artifact_bytes
        manifest["size"]["under_limit"] = under_limit
        manifest["root_causes"] = desired_root_causes
        manifest["ok"] = desired_ok

    manifest["root_causes"].append("manifest byte accounting did not converge")
    manifest["ok"] = False
    return json.dumps(manifest, indent=2, sort_keys=True) + "\n"


def main() -> int:
    args = parse_args()
    repo_root = args.repo_root.resolve()
    report_root = repo_root / FRAMEWORK_EVIDENCE_ROOT
    report_root.mkdir(parents=True, exist_ok=True)
    manifest_path = repo_root / MANIFEST_PATH
    manifest_path.unlink(missing_ok=True)

    root_causes: list[str] = []
    worktree_changes: list[str] | None = None
    try:
        tested_sha = git_head(repo_root)
        worktree_changes = git_worktree_changes(repo_root)
        if worktree_changes:
            root_causes.append(
                "tested worktree differs from Git HEAD; exact-SHA evidence "
                "requires a clean source tree"
            )
    except (OSError, subprocess.CalledProcessError) as exc:
        tested_sha = None
        root_causes.append(f"unable to resolve tested Git SHA: {exc}")

    identity: dict[str, str] = {}
    source_sha: str | None = tested_sha
    destination_sha: str | None = None
    tested_commit_relation = "unavailable"
    if tested_sha is not None:
        (
            identity,
            source_sha,
            destination_sha,
            tested_commit_relation,
            identity_errors,
        ) = bitbucket_identity(repo_root, tested_sha)
        root_causes.extend(identity_errors)

    if args.gate_exit_status != 0:
        root_causes.append(f"Fast gate sequence exited with status {args.gate_exit_status}")
    elif tested_sha is not None:
        root_causes.extend(required_success_errors(repo_root, tested_sha))

    temporary_root: Path | None = None
    staging_root: Path | None = None
    try:
        evidence = discover_evidence(repo_root)
        artifacts, total_bytes, temporary_root, staging_root = stage_evidence(
            repo_root, evidence
        )
    except (OSError, RuntimeError) as exc:
        evidence = []
        artifacts = []
        total_bytes = 0
        root_causes.append(f"unable to stage Fast evidence: {exc}")

    try:
        max_bytes = configured_max_bytes()
    except ValueError as exc:
        max_bytes = MAX_EVIDENCE_BYTES
        root_causes.append(str(exc))

    ok = not root_causes
    manifest = {
        "schema": SCHEMA,
        "command": "package-pr-fast-evidence",
        "ok": ok,
        "generated_at_utc": utc_now(),
        "artifact": MANIFEST_PATH.as_posix(),
        "artifact_name": "pr-fast-evidence",
        "source_sha": source_sha,
        "tested_sha": tested_sha,
        "destination_sha": destination_sha,
        "tested_commit_relation": tested_commit_relation,
        "source_worktree_clean": worktree_changes == [],
        "gate_exit_status": args.gate_exit_status,
        "bitbucket": identity,
        "assurance": {
            "existing_fast_gate_count": 6,
            "existing_fast_gates_preserved": True,
            "required_reports_checked_on_success": [
                relative.as_posix() for relative, _ in REQUIRED_SUCCESS_REPORTS
            ],
        },
        "allowlist": {
            "root_level_evidence_suffixes": sorted(ALLOWED_SUFFIXES),
            "recursive_directories": [path.as_posix() for path in RECURSIVE_EVIDENCE_DIRS],
            # Product validation remains a local Fast success precondition, but
            # product-owned reports were not part of the former target/appfw/**
            # upload and therefore must not enter this strict-subset artifact.
            "product_report_files": [],
            "excluded_classes": [
                "cargo build output",
                "npm cache",
                "generated workspace",
                "package archive",
                "tool installation",
            ],
        },
        "size": {
            "retained_file_bytes": total_bytes,
            "manifest_bytes": 0,
            "total_uncompressed_bytes": total_bytes,
            "maximum_uncompressed_bytes": max_bytes,
            "under_limit": True,
            "compressed_bytes": None,
            "remote_upload_seconds": None,
        },
        "artifacts": artifacts,
        "root_causes": root_causes,
    }

    # The uploaded artifact also contains this manifest. Stabilize its byte
    # count, total, size verdict, and root causes so the hard ceiling covers
    # every uploaded regular file, not only the staged evidence copies.
    non_size_root_causes = list(root_causes)
    serialized = stabilize_manifest_size(
        manifest, total_bytes, max_bytes, non_size_root_causes
    )

    # Publish the small diagnostic manifest first. The evidence directory stays
    # outside the upload glob until both copying and the prospective complete
    # bundle size pass. An over-limit failure therefore uploads only the
    # manifest, never the oversized payload that caused it.
    manifest_path.write_text(serialized, encoding="utf-8")
    if temporary_root is not None and staging_root is not None:
        if manifest["size"]["under_limit"]:
            try:
                os.replace(temporary_root, staging_root)
            except OSError as exc:
                shutil.rmtree(temporary_root, ignore_errors=True)
                shutil.rmtree(staging_root, ignore_errors=True)
                non_size_root_causes.append(f"unable to publish Fast evidence: {exc}")
                serialized = stabilize_manifest_size(
                    manifest, total_bytes, max_bytes, non_size_root_causes
                )
                manifest_path.write_text(serialized, encoding="utf-8")
        else:
            shutil.rmtree(temporary_root, ignore_errors=True)
            shutil.rmtree(staging_root, ignore_errors=True)
    ok = manifest["ok"]

    print(
        json.dumps(
            {
                "command": manifest["command"],
                "ok": ok,
                "artifact": manifest["artifact"],
                "artifact_count": len(artifacts),
                "total_uncompressed_bytes": manifest["size"]["total_uncompressed_bytes"],
                "source_sha": source_sha,
                "root_causes": manifest["root_causes"],
            },
            sort_keys=True,
        )
    )
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
