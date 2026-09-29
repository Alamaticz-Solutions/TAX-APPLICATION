#!/usr/bin/env python3
"""Dependency advisory and upgrade-pressure report for App Framework."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time
from typing import Any
from urllib import error, parse, request

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - repository CI uses Python 3.11+.
    tomllib = None


LEGACY_RECOMMENDATIONS = {
    "async-graphql": {
        "severity": "high",
        "trigger": "duplicate",
        "reason": "runtime/backend manifests still use async-graphql 6 while test harnesses use async-graphql 7",
        "recommendation": "upgrade appfw-runtime, backend templates, and generated GraphQL integration to async-graphql 7",
    },
    "reqwest": {
        "severity": "high",
        "trigger": "direct_legacy",
        "legacy_prefixes": ("^0.11", "0.11"),
        "reason": "direct reqwest 0.11 users coexist with newer reqwest lines and keep legacy TLS transitive debt in the graph",
        "recommendation": "move direct HTTP clients off reqwest 0.11 and align with the current supported reqwest line used by auth/telemetry dependencies",
    },
    "axum": {
        "severity": "medium",
        "trigger": "direct_legacy",
        "legacy_prefixes": ("^0.6", "0.6"),
        "reason": "runtime HTTP ingress is still on axum 0.6 while the wider graph already carries modern http/hyper/tower stacks",
        "recommendation": "plan an axum/tower/tower-http ingress migration with focused runtime route and middleware tests",
    },
    "tower": {
        "severity": "medium",
        "trigger": "direct_legacy",
        "legacy_prefixes": ("^0.4", "0.4"),
        "reason": "tower 0.4 remains a direct runtime/backend dependency while newer tower versions are already pulled transitively",
        "recommendation": "upgrade with axum/tower-http so middleware and service trait versions stay aligned",
    },
    "tower-http": {
        "severity": "medium",
        "trigger": "direct_legacy",
        "legacy_prefixes": ("^0.4", "0.4"),
        "reason": "tower-http 0.4 remains a direct runtime/backend dependency while newer tower-http versions are already pulled transitively",
        "recommendation": "upgrade with axum/tower so CORS, timeout, request-id, trace, and static-file middleware stay coherent",
    },
    "dotenv": {
        "severity": "medium",
        "trigger": "direct",
        "reason": "dotenv is recorded in deny.toml as reviewed unmaintained dependency debt",
        "recommendation": "replace dotenv with dotenvy or a runtime-owned environment loader",
    },
    "json": {
        "severity": "medium",
        "trigger": "direct",
        "reason": "json is recorded in deny.toml as reviewed unmaintained backend compatibility debt",
        "recommendation": "replace json crate usage with serde_json value/object handling",
    },
    "derivative": {
        "severity": "medium",
        "trigger": "direct",
        "reason": "derivative is recorded in deny.toml as reviewed unmaintained derive debt",
        "recommendation": "replace with standard derives, derive_more, derive-where, or local trait implementations",
    },
    "serde_yaml": {
        "severity": "medium",
        "trigger": "locked_deprecated",
        "reason": "the resolved serde_yaml version carries +deprecated metadata",
        "recommendation": "track a YAML parser migration or formal risk acceptance for continued config parsing use",
    },
    "env_logger": {
        "severity": "low",
        "trigger": "direct_legacy",
        "legacy_prefixes": ("^0.10", "0.10"),
        "reason": "env_logger 0.10 is older logging setup debt in codegen",
        "recommendation": "upgrade to the current env_logger line or converge startup logging on tracing-subscriber",
    },
    "Inflector": {
        "severity": "low",
        "trigger": "wildcard",
        "reason": "wildcard requirements hide intentional upgrade boundaries",
        "recommendation": "pin Inflector to an explicit compatible requirement or replace with a maintained naming helper",
    },
}


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def rel(path: str | Path, root: Path) -> str:
    path = Path(path)
    try:
        return path.resolve().relative_to(root.resolve()).as_posix()
    except Exception:
        return path.as_posix()


def truncate(value: str, limit: int = 6000) -> str:
    if len(value) <= limit:
        return value
    return value[:limit] + f"\n... truncated {len(value) - limit} characters ..."


def command_text(args: list[str]) -> str:
    return " ".join(args)


def run_command(
    args: list[str],
    cwd: Path,
    *,
    timeout: int = 180,
    env: dict[str, str] | None = None,
) -> dict[str, Any]:
    started = time.monotonic()
    try:
        result = subprocess.run(
            args,
            cwd=cwd,
            env=env,
            text=True,
            capture_output=True,
            timeout=timeout,
            check=False,
        )
        duration_ms = int((time.monotonic() - started) * 1000)
        return {
            "command": command_text(args),
            "ok": result.returncode == 0,
            "exit_code": result.returncode,
            "duration_ms": duration_ms,
            "stdout": result.stdout,
            "stderr": result.stderr,
        }
    except FileNotFoundError as err:
        return {
            "command": command_text(args),
            "ok": False,
            "exit_code": 127,
            "duration_ms": int((time.monotonic() - started) * 1000),
            "stdout": "",
            "stderr": str(err),
        }
    except subprocess.TimeoutExpired as err:
        return {
            "command": command_text(args),
            "ok": False,
            "exit_code": 124,
            "duration_ms": int((time.monotonic() - started) * 1000),
            "stdout": err.stdout or "",
            "stderr": err.stderr or f"timed out after {timeout}s",
        }


def is_relative_to(path: Path, parent: Path) -> bool:
    try:
        path.resolve().relative_to(parent.resolve())
        return True
    except Exception:
        return False


def ensure_writable_directory(path: Path) -> tuple[bool, str | None]:
    try:
        path.mkdir(parents=True, exist_ok=True)
        probe = path / f".appfw-write-test-{os.getpid()}"
        probe.write_text("ok", encoding="utf-8")
        probe.unlink(missing_ok=True)
        return True, None
    except Exception as err:
        return False, str(err)


def dependency_tool_environment(repo_root: Path, report_dir: Path) -> tuple[dict[str, str], dict[str, Any]]:
    env = os.environ.copy()
    requested = os.environ.get("APPFW_DEPENDENCY_CHECK_CARGO_HOME")
    ambient = os.environ.get("CARGO_HOME")
    attempted: list[dict[str, Any]] = []

    if requested:
        chosen = Path(requested).expanduser()
        source = "APPFW_DEPENDENCY_CHECK_CARGO_HOME"
    else:
        ambient_home = Path(ambient).expanduser() if ambient else Path.home() / ".cargo"
        ambient_ok, ambient_error = ensure_writable_directory(ambient_home)
        attempted.append(
            {
                "source": "CARGO_HOME" if ambient else "default-home",
                "path": ambient_home.as_posix(),
                "writable": ambient_ok,
                "error": ambient_error,
            }
        )
        if ambient_ok:
            chosen = ambient_home
            source = "CARGO_HOME" if ambient else "default-home"
        else:
            chosen = report_dir / "cargo-home"
            source = "repo-local-fallback"

    chosen_ok, chosen_error = ensure_writable_directory(chosen)
    attempted.append(
        {
            "source": source,
            "path": chosen.as_posix(),
            "writable": chosen_ok,
            "error": chosen_error,
        }
    )
    env["CARGO_HOME"] = chosen.as_posix()
    env["APPFW_DEPENDENCY_CHECK_CARGO_HOME"] = chosen.as_posix()

    metadata = {
        "cargo_home": chosen.as_posix(),
        "cargo_home_source": source,
        "cargo_home_writable": chosen_ok,
        "cargo_home_error": chosen_error,
        "repo_local": is_relative_to(chosen, repo_root),
        "attempted_cargo_homes": attempted,
    }
    return env, metadata


def check_record(
    name: str,
    result: dict[str, Any],
    *,
    required: bool,
    status: str | None = None,
    include_output: bool = True,
) -> dict[str, Any]:
    record = {
        "name": name,
        "ok": bool(result.get("ok")),
        "required": required,
        "status": status or ("passed" if result.get("ok") else "failed"),
        "exit_code": result.get("exit_code"),
        "duration_ms": result.get("duration_ms"),
        "command": result.get("command"),
    }
    stdout = result.get("stdout") or ""
    stderr = result.get("stderr") or ""
    if include_output and stdout:
        record["stdout_excerpt"] = truncate(stdout)
    if include_output and stderr:
        record["stderr_excerpt"] = truncate(stderr)
    return record


def load_toml(path: Path) -> dict[str, Any]:
    if tomllib is None:
        raise RuntimeError("Python tomllib is required; use Python 3.11 or newer")
    with path.open("rb") as handle:
        return tomllib.load(handle)


def discover_tracked_files(repo_root: Path) -> list[str]:
    if shutil.which("git"):
        result = run_command(["git", "ls-files"], repo_root, timeout=60)
        if result["ok"]:
            return [line for line in result["stdout"].splitlines() if line]

    files: list[str] = []
    excluded_dirs = {".git", "target", "node_modules"}
    for current_root, dirs, names in os.walk(repo_root):
        dirs[:] = [name for name in dirs if name not in excluded_dirs]
        for name in names:
            files.append(rel(Path(current_root) / name, repo_root))
    return sorted(files)


def parse_version(version: str) -> tuple[int, int, int] | None:
    match = re.match(r"^(\d+)(?:\.(\d+))?(?:\.(\d+))?", version)
    if not match:
        return None
    return (
        int(match.group(1)),
        int(match.group(2) or 0),
        int(match.group(3) or 0),
    )


def compatibility_key(version: str) -> str:
    parsed = parse_version(version)
    if not parsed:
        return version
    major, minor, patch = parsed
    if major == 0 and minor == 0:
        return f"0.0.{patch}"
    if major == 0:
        return f"0.{minor}"
    return str(major)


def requirement_base(req: str) -> str | None:
    cleaned = req.strip()
    if cleaned == "*":
        return None
    cleaned = cleaned.removeprefix("^").removeprefix("=").strip()
    match = re.search(r"\d+(?:\.\d+)?(?:\.\d+)?", cleaned)
    return match.group(0) if match else None


def latest_compatible_with_req(req: str, latest: str) -> bool | None:
    if req.strip() == "*":
        return True
    base = requirement_base(req)
    if not base:
        return None
    return compatibility_key(base) == compatibility_key(latest)


def version_sort_key(version: str) -> tuple[int, int, int, str]:
    parsed = parse_version(version)
    if not parsed:
        return (0, 0, 0, version)
    return (*parsed, version)


def direct_dependency_rows(metadata: dict[str, Any], repo_root: Path) -> list[dict[str, Any]]:
    workspace_members = set(metadata.get("workspace_members", []))
    rows: list[dict[str, Any]] = []
    for package in metadata.get("packages", []):
        if workspace_members and package.get("id") not in workspace_members:
            continue
        manifest_path = Path(package.get("manifest_path", ""))
        for dependency in package.get("dependencies", []):
            if dependency.get("source") is None:
                continue
            rows.append(
                {
                    "workspace_crate": package.get("name"),
                    "manifest": rel(manifest_path, repo_root),
                    "name": dependency.get("name"),
                    "requirement": dependency.get("req"),
                    "rename": dependency.get("rename"),
                    "kind": dependency.get("kind") or "normal",
                    "optional": bool(dependency.get("optional")),
                    "features": dependency.get("features") or [],
                }
            )
    return sorted(
        rows,
        key=lambda row: (
            row["name"] or "",
            row["workspace_crate"] or "",
            row["manifest"] or "",
            row["requirement"] or "",
        ),
    )


def locked_packages(lock_data: dict[str, Any]) -> list[dict[str, Any]]:
    packages = []
    for package in lock_data.get("package", []):
        name = package.get("name")
        version = package.get("version")
        if not name or not version:
            continue
        packages.append(
            {
                "name": name,
                "version": version,
                "source": package.get("source") or "workspace",
                "checksum_present": bool(package.get("checksum")),
            }
        )
    return packages


def duplicate_version_groups(packages: list[dict[str, Any]]) -> list[dict[str, Any]]:
    versions_by_name: dict[str, set[str]] = {}
    for package in packages:
        if package["source"] == "workspace":
            continue
        versions_by_name.setdefault(package["name"], set()).add(package["version"])

    duplicates = []
    for name, versions in sorted(versions_by_name.items()):
        if len(versions) <= 1:
            continue
        sorted_versions = sorted(versions, key=version_sort_key)
        compat_keys = sorted({compatibility_key(version) for version in sorted_versions})
        duplicates.append(
            {
                "name": name,
                "versions": sorted_versions,
                "compatibility_keys": compat_keys,
                "compatibility_line_count": len(compat_keys),
            }
        )
    return duplicates


def direct_requirements_by_name(rows: list[dict[str, Any]]) -> dict[str, list[dict[str, Any]]]:
    by_name: dict[str, list[dict[str, Any]]] = {}
    for row in rows:
        by_name.setdefault(row["name"], []).append(row)
    return by_name


def locked_versions_by_name(packages: list[dict[str, Any]]) -> dict[str, list[str]]:
    by_name: dict[str, set[str]] = {}
    for package in packages:
        if package["source"] == "workspace":
            continue
        by_name.setdefault(package["name"], set()).add(package["version"])
    return {name: sorted(versions, key=version_sort_key) for name, versions in by_name.items()}


def build_upgrade_advice(
    direct_rows: list[dict[str, Any]],
    duplicates: list[dict[str, Any]],
    locked_by_name: dict[str, list[str]],
    latest_versions: dict[str, str],
) -> list[dict[str, Any]]:
    advice: dict[str, dict[str, Any]] = {}
    requirements = direct_requirements_by_name(direct_rows)
    duplicate_names = {item["name"]: item for item in duplicates}

    for name, recommendation in LEGACY_RECOMMENDATIONS.items():
        trigger = recommendation["trigger"]
        affected = requirements.get(name, [])
        locked_versions = locked_by_name.get(name, [])
        include = False
        if trigger == "duplicate":
            include = name in duplicate_names
        elif trigger == "direct":
            include = bool(affected)
        elif trigger == "wildcard":
            include = any(row.get("requirement") == "*" for row in affected)
        elif trigger == "locked_deprecated":
            include = any("+deprecated" in version for version in locked_versions)
        elif trigger == "direct_legacy":
            legacy_prefixes = recommendation.get("legacy_prefixes", ())
            include = any(
                str(row.get("requirement") or "").startswith(legacy_prefixes)
                for row in affected
            )

        if include:
            advice[name] = {
                "package": name,
                "severity": recommendation["severity"],
                "reason": recommendation["reason"],
                "recommendation": recommendation["recommendation"],
                "current_requirements": sorted(
                    {
                        f"{row['workspace_crate']}:{row['requirement']}"
                        for row in affected
                        if row.get("workspace_crate") and row.get("requirement")
                    }
                ),
                "locked_versions": locked_versions,
            }
            if name in duplicate_names:
                advice[name]["duplicate_versions"] = duplicate_names[name]["versions"]
                advice[name]["compatibility_keys"] = duplicate_names[name]["compatibility_keys"]

    for name, rows in requirements.items():
        latest = latest_versions.get(name)
        if not latest:
            continue
        locked_versions = locked_by_name.get(name, [])
        current_latest_locked = locked_versions[-1] if locked_versions else None
        if current_latest_locked == latest:
            continue
        manifest_update_needed = any(
            latest_compatible_with_req(row.get("requirement") or "", latest) is False
            for row in rows
        )
        severity = "medium" if manifest_update_needed else "low"
        existing = advice.get(name)
        if existing:
            existing["latest_version"] = latest
            existing["latest_action"] = "manifest_update" if manifest_update_needed else "lockfile_update"
            continue
        advice[name] = {
            "package": name,
            "severity": severity,
            "reason": "online registry lookup found a newer direct dependency version",
            "recommendation": "update the manifest requirement before refreshing Cargo.lock"
            if manifest_update_needed
            else "refresh Cargo.lock with an intentional cargo update for this package",
            "current_requirements": sorted(
                {
                    f"{row['workspace_crate']}:{row['requirement']}"
                    for row in rows
                    if row.get("workspace_crate") and row.get("requirement")
                }
            ),
            "locked_versions": locked_versions,
            "latest_version": latest,
            "latest_action": "manifest_update" if manifest_update_needed else "lockfile_update",
        }

    severity_rank = {"high": 0, "medium": 1, "low": 2}
    return sorted(advice.values(), key=lambda item: (severity_rank.get(item["severity"], 9), item["package"]))


def parse_cargo_audit(stdout: str) -> dict[str, Any]:
    if not stdout.strip():
        return {"status": "empty"}
    try:
        data = json.loads(stdout)
    except json.JSONDecodeError as err:
        return {"status": "unparsed", "error": str(err)}

    vulnerabilities = data.get("vulnerabilities") or {}
    warnings = data.get("warnings") or {}
    return {
        "status": "parsed",
        "database": data.get("database"),
        "dependency_count": data.get("dependency_count"),
        "vulnerabilities": vulnerabilities,
        "warnings": warnings,
        "vulnerability_count": int(vulnerabilities.get("count") or 0),
        "warning_count": sum(len(value) for value in warnings.values() if isinstance(value, list)),
    }


GHSA_ID_RE = re.compile(r"GHSA-[0-9a-z]+-[0-9a-z]+-[0-9a-z]+", re.IGNORECASE)


def normalize_advisory_id(value: str) -> str:
    return value.strip().upper()


def extract_ghsa_id(value: Any) -> str | None:
    if value is None:
        return None
    match = GHSA_ID_RE.search(str(value))
    return normalize_advisory_id(match.group(0)) if match else None


def npm_audit_leaf_advisories(audit_payload: dict[str, Any]) -> list[dict[str, str]]:
    leaves: list[dict[str, str]] = []
    seen: set[tuple[str, str]] = set()
    vulnerabilities = audit_payload.get("vulnerabilities") or {}
    if not isinstance(vulnerabilities, dict):
        return leaves
    for package_name, vuln in vulnerabilities.items():
        if not isinstance(vuln, dict):
            continue
        for via in vuln.get("via") or []:
            if not isinstance(via, dict):
                continue
            advisory_id = (
                extract_ghsa_id(via.get("url"))
                or extract_ghsa_id(via.get("id"))
                or extract_ghsa_id(via.get("source"))
            )
            if not advisory_id:
                continue
            package = str(via.get("name") or package_name or "").strip()
            key = (advisory_id, package)
            if key in seen:
                continue
            seen.add(key)
            leaves.append({"id": advisory_id, "package": package, "url": str(via.get("url") or "")})
    return leaves


def npm_leaf_is_accepted(
    leaf: dict[str, str],
    acceptances: list[dict[str, Any]],
    lockfile: Path,
    today: dt.date,
) -> bool:
    leaf_id = normalize_advisory_id(leaf.get("id") or "")
    leaf_package = leaf.get("package") or ""
    if not leaf_id:
        return False
    for acceptance in acceptances:
        if not acceptance_is_active(acceptance, today):
            continue
        accepted_ids = {normalize_advisory_id(identifier) for identifier in (acceptance.get("ids") or [])}
        if leaf_id not in accepted_ids:
            continue
        ecosystem = acceptance.get("ecosystem")
        if ecosystem and ecosystem != "npm":
            continue
        package = acceptance.get("package")
        if package and package != leaf_package:
            continue
        version = acceptance.get("version")
        if version:
            locked = locked_npm_versions(lockfile, leaf_package or package)
            if not locked or any(item != version for item in locked):
                continue
        return True
    return False


def parse_npm_audit(stdout: str) -> dict[str, Any]:
    if not stdout.strip():
        return {"status": "empty"}
    try:
        data = json.loads(stdout)
    except json.JSONDecodeError as err:
        return {"status": "unparsed", "error": str(err)}
    metadata = data.get("metadata") or {}
    vulnerabilities = metadata.get("vulnerabilities") or {}
    total = vulnerabilities.get("total")
    if total is None:
        total = sum(value for value in vulnerabilities.values() if isinstance(value, int))
    return {
        "status": "parsed",
        "vulnerabilities": vulnerabilities,
        "vulnerability_count": int(total or 0),
        "audit_report_version": data.get("auditReportVersion"),
        "leaf_advisories": npm_audit_leaf_advisories(data),
    }


def parse_package_lock(lockfile: Path, repo_root: Path) -> dict[str, Any]:
    try:
        data = json.loads(lockfile.read_text(encoding="utf-8"))
    except Exception as err:
        return {
            "path": rel(lockfile, repo_root),
            "status": "unparsed",
            "error": str(err),
        }
    packages = data.get("packages") or {}
    root = packages.get("") if isinstance(packages, dict) else {}
    dependencies: list[dict[str, Any]] = []
    if isinstance(root, dict):
        for key in ("dependencies", "devDependencies", "optionalDependencies"):
            values = root.get(key) or {}
            if isinstance(values, dict):
                dependencies.extend(
                    {"name": name, "requirement": requirement, "kind": key}
                    for name, requirement in sorted(values.items())
                )
    return {
        "path": rel(lockfile, repo_root),
        "status": "parsed",
        "project_name": root.get("name") if isinstance(root, dict) else None,
        "project_version": root.get("version") if isinstance(root, dict) else None,
        "package_count": max(len(packages) - 1, 0) if isinstance(packages, dict) else None,
        "direct_dependencies": dependencies,
    }


def package_lock_components(lockfile: Path) -> list[dict[str, str]]:
    try:
        data = json.loads(lockfile.read_text(encoding="utf-8"))
    except Exception:
        return []
    packages = data.get("packages") or {}
    if not isinstance(packages, dict):
        return []
    components: list[dict[str, str]] = []
    for package_path, package in packages.items():
        if not package_path or not isinstance(package, dict):
            continue
        version = package.get("version")
        name = package.get("name") or npm_name_from_package_path(package_path)
        if name and version:
            components.append({"ecosystem": "npm", "name": name, "version": version})
    return components


def npm_component_key(component: dict[str, str]) -> tuple[str, str, str] | None:
    ecosystem = component.get("ecosystem")
    name = component.get("name")
    version = component.get("version")
    if ecosystem and name and version:
        return (ecosystem, name, version)
    return None


def npm_name_from_package_path(package_path: str) -> str | None:
    parts = package_path.split("/")
    indexes = [index for index, part in enumerate(parts) if part == "node_modules"]
    if not indexes:
        return None
    start = indexes[-1] + 1
    if start >= len(parts):
        return None
    if parts[start].startswith("@") and start + 1 < len(parts):
        return f"{parts[start]}/{parts[start + 1]}"
    return parts[start]


def load_json_file(path: Path) -> dict[str, Any] | None:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        return None
    return value if isinstance(value, dict) else None


def mobile_runtime_audit_release_gate(lockfile: Path, repo_root: Path) -> dict[str, Any] | None:
    """Return release-gate metadata for product mobile runtime audit posture.

    Product mobile scaffolds intentionally retain npm audit evidence while the
    legacy mobile-test command remains non-authoritative. The future source-bound
    mobile candidate checker owns candidate gating; dependency-check should keep
    these findings visible without turning every Prototype scaffold into a
    framework PR blocker.
    """

    if lockfile.name != "package-lock.json":
        return None
    mobile_root = lockfile.parent
    evidence_path = mobile_root / ".appfw-mobile/npm-audit-evidence.json"
    template_path = mobile_root / ".appfw-mobile/npm-audit-disposition.template.json"
    disposition_path = mobile_root / ".appfw-mobile/npm-audit-disposition.json"
    if not evidence_path.is_file() or not template_path.is_file():
        return None

    evidence = load_json_file(evidence_path) or {}
    template = load_json_file(template_path) or {}
    if template.get("template_only") is not True:
        return None

    audit_summary = evidence.get("audit_summary")
    if not isinstance(audit_summary, dict):
        metadata = evidence.get("metadata")
        if isinstance(metadata, dict) and isinstance(metadata.get("vulnerabilities"), dict):
            audit_summary = metadata["vulnerabilities"]
    if not isinstance(audit_summary, dict):
        audit_summary = None

    return {
        "status": "mobile-release-gated",
        "authoritative": False,
        "candidate_gate": "future-source-bound-mobile-candidate-checker",
        "reason": (
            "mobile runtime audit findings are retained by product mobile-test as "
            "non-authoritative diagnostics and must be cleared or formally "
            "dispositioned before a future source-bound mobile candidate checker "
            "can consider the app"
        ),
        "lockfile": rel(lockfile, repo_root),
        "audit_evidence": rel(evidence_path, repo_root),
        "disposition": rel(disposition_path, repo_root),
        "disposition_present": disposition_path.is_file(),
        "disposition_template": rel(template_path, repo_root),
        "disposition_required": not disposition_path.is_file(),
        "audit_summary": audit_summary,
        "mobile_test_command": "scripts/appfw product mobile-test --json",
    }


def http_json(url: str, *, payload: dict[str, Any] | None = None, timeout: int = 30) -> dict[str, Any]:
    curl = shutil.which("curl")
    if curl:
        curl_args = [
            curl,
            "-fsSL",
            "--retry",
            "2",
            "--max-time",
            str(timeout),
            "-H",
            "User-Agent: appfw-dependency-check/1.0",
        ]
        if payload is not None:
            curl_args.extend(
                [
                    "-H",
                    "Content-Type: application/json",
                    "--data-binary",
                    json.dumps(payload),
                ]
            )
        curl_args.append(url)
        result = subprocess.run(
            curl_args,
            text=True,
            capture_output=True,
            timeout=timeout + 5,
            check=False,
        )
        if result.returncode == 0:
            return json.loads(result.stdout)
        raise OSError(result.stderr or result.stdout or f"curl exited with {result.returncode}")

    data = None
    headers = {"User-Agent": "appfw-dependency-check/1.0"}
    if payload is not None:
        data = json.dumps(payload).encode("utf-8")
        headers["Content-Type"] = "application/json"
    req = request.Request(url, data=data, headers=headers)
    with request.urlopen(req, timeout=timeout) as response:
        return json.loads(response.read().decode("utf-8"))


def query_crates_latest(names: list[str]) -> tuple[dict[str, str], list[dict[str, str]]]:
    latest: dict[str, str] = {}
    errors: list[dict[str, str]] = []
    for name in names:
        url = f"https://crates.io/api/v1/crates/{parse.quote(name, safe='')}"
        try:
            data = http_json(url, timeout=20)
            crate = data.get("crate") or {}
            version = crate.get("max_stable_version") or crate.get("newest_version")
            if version:
                latest[name] = version
        except (error.URLError, TimeoutError, json.JSONDecodeError, OSError) as err:
            errors.append({"package": name, "error": str(err)})
    return latest, errors


def query_osv(components: list[dict[str, str]]) -> tuple[list[dict[str, Any]], list[str]]:
    findings: list[dict[str, Any]] = []
    errors: list[str] = []
    seen_components: set[tuple[str, str, str]] = set()
    queries: list[dict[str, Any]] = []
    query_components: list[dict[str, str]] = []
    for component in components:
        key = (component["ecosystem"], component["name"], component["version"])
        if key in seen_components:
            continue
        seen_components.add(key)
        queries.append(
            {
                "package": {
                    "ecosystem": component["ecosystem"],
                    "name": component["name"],
                },
                "version": component["version"],
            }
        )
        query_components.append(component)

    detail_cache: dict[str, dict[str, Any]] = {}
    for start in range(0, len(queries), 100):
        batch = queries[start : start + 100]
        batch_components = query_components[start : start + 100]
        try:
            data = http_json("https://api.osv.dev/v1/querybatch", payload={"queries": batch}, timeout=60)
        except (error.URLError, TimeoutError, json.JSONDecodeError, OSError) as err:
            errors.append(str(err))
            continue
        results = data.get("results") or []
        for component, result in zip(batch_components, results):
            vulns = result.get("vulns") or []
            for vuln in vulns:
                vuln_id = vuln.get("id")
                detail = detail_cache.get(vuln_id or "")
                if vuln_id and detail is None:
                    try:
                        detail = http_json(f"https://api.osv.dev/v1/vulns/{parse.quote(vuln_id, safe='')}", timeout=30)
                    except (error.URLError, TimeoutError, json.JSONDecodeError, OSError):
                        detail = vuln
                    detail_cache[vuln_id] = detail
                detail = detail or vuln
                findings.append(
                    {
                        "ecosystem": component["ecosystem"],
                        "package": component["name"],
                        "version": component["version"],
                        "id": vuln_id,
                        "aliases": detail.get("aliases") or [],
                        "summary": detail.get("summary"),
                        "severity": detail.get("severity") or [],
                        "modified": detail.get("modified") or vuln.get("modified"),
                        "database_specific": detail.get("database_specific") or {},
                    }
                )
    return findings, errors


def dependency_policy_path(repo_root: Path, policy_file: Path | None) -> Path | None:
    if policy_file is not None:
        return policy_file if policy_file.is_absolute() else repo_root / policy_file
    default_path = repo_root / "dependency-check.toml"
    return default_path if default_path.is_file() else None


def load_dependency_policy(repo_root: Path, policy_file: Path | None) -> dict[str, Any]:
    path = dependency_policy_path(repo_root, policy_file)
    policy: dict[str, Any] = {
        "path": rel(path, repo_root) if path else None,
        "status": "not_configured" if path is None else "configured",
        "osv_acceptances": [],
        "errors": [],
    }
    if path is None:
        return policy
    if not path.is_file():
        policy["status"] = "missing"
        policy["errors"].append(f"policy file not found: {rel(path, repo_root)}")
        return policy

    try:
        data = load_toml(path)
    except Exception as err:
        policy["status"] = "unparsed"
        policy["errors"].append(f"policy file parse failed: {err}")
        return policy

    osv_policy = data.get("osv") if isinstance(data, dict) else {}
    raw_acceptances = []
    if isinstance(osv_policy, dict):
        raw_acceptances = osv_policy.get("accepted") or osv_policy.get("ignore") or []
    if not isinstance(raw_acceptances, list):
        policy["errors"].append("osv.accepted must be an array")
        raw_acceptances = []

    for index, item in enumerate(raw_acceptances):
        if not isinstance(item, dict):
            policy["errors"].append(f"osv.accepted[{index}] must be an object")
            continue
        identifiers = []
        if item.get("id"):
            identifiers.append(str(item["id"]))
        for key in ("ids", "aliases"):
            values = item.get(key) or []
            if isinstance(values, str):
                values = [values]
            if isinstance(values, list):
                identifiers.extend(str(value) for value in values if value)
            else:
                policy["errors"].append(f"osv.accepted[{index}].{key} must be a string or array")
        identifiers = sorted({identifier.strip() for identifier in identifiers if identifier.strip()})
        reason = str(item.get("reason") or "").strip()
        until = str(item.get("until") or "").strip()
        if not identifiers:
            policy["errors"].append(f"osv.accepted[{index}] must include id, ids, or aliases")
        if not reason:
            policy["errors"].append(f"osv.accepted[{index}] must include a reason")
        if not until:
            policy["errors"].append(f"osv.accepted[{index}] must include an until date")
        else:
            try:
                dt.date.fromisoformat(until)
            except ValueError:
                policy["errors"].append(f"osv.accepted[{index}].until must be YYYY-MM-DD")
        policy["osv_acceptances"].append(
            {
                "ids": identifiers,
                "package": str(item.get("package") or "").strip() or None,
                "ecosystem": str(item.get("ecosystem") or "").strip() or None,
                "version": str(item.get("version") or "").strip() or None,
                "until": until or None,
                "reason": reason or None,
                "owner": str(item.get("owner") or "").strip() or None,
                "source": rel(path, repo_root),
                "index": index,
            }
        )

    if policy["errors"]:
        policy["status"] = "invalid"
    return policy


def finding_identifiers(finding: dict[str, Any]) -> set[str]:
    return {
        identifier
        for identifier in [finding.get("id") or "", *(finding.get("aliases") or [])]
        if identifier
    }


def acceptance_matches_finding(acceptance: dict[str, Any], finding: dict[str, Any]) -> bool:
    accepted_ids = set(acceptance.get("ids") or [])
    if not accepted_ids.intersection(finding_identifiers(finding)):
        return False
    for key in ("package", "ecosystem", "version"):
        expected = acceptance.get(key)
        if expected and expected != finding.get(key):
            return False
    return True


def acceptance_is_active(acceptance: dict[str, Any], today: dt.date) -> bool:
    until = acceptance.get("until")
    if not until:
        return False
    try:
        return dt.date.fromisoformat(until) >= today
    except ValueError:
        return False


def compact_osv_finding(finding: dict[str, Any]) -> dict[str, Any]:
    return {
        "ecosystem": finding.get("ecosystem"),
        "package": finding.get("package"),
        "version": finding.get("version"),
        "id": finding.get("id"),
        "aliases": finding.get("aliases") or [],
        "summary": finding.get("summary"),
        "severity": finding.get("severity") or [],
        "modified": finding.get("modified"),
    }


def compact_osv_acceptance(acceptance: dict[str, Any]) -> dict[str, Any]:
    return {
        "ids": acceptance.get("ids") or [],
        "package": acceptance.get("package"),
        "ecosystem": acceptance.get("ecosystem"),
        "version": acceptance.get("version"),
        "until": acceptance.get("until"),
        "reason": acceptance.get("reason"),
        "owner": acceptance.get("owner"),
        "source": acceptance.get("source"),
    }


def classify_osv_findings(
    findings: list[dict[str, Any]],
    acceptances: list[dict[str, Any]],
) -> dict[str, Any]:
    today = dt.datetime.now(dt.timezone.utc).date()
    blocking: list[dict[str, Any]] = []
    accepted: list[dict[str, Any]] = []
    expired: list[dict[str, Any]] = []
    matched_acceptance_indexes: set[int] = set()

    for finding in findings:
        if not osv_finding_is_blocking(finding):
            continue
        matches = [
            acceptance
            for acceptance in acceptances
            if acceptance_matches_finding(acceptance, finding)
        ]
        active = next(
            (acceptance for acceptance in matches if acceptance_is_active(acceptance, today)),
            None,
        )
        if active:
            matched_acceptance_indexes.add(int(active.get("index") or 0))
            accepted.append(
                {
                    "finding": compact_osv_finding(finding),
                    "acceptance": compact_osv_acceptance(active),
                }
            )
            continue

        if matches:
            for acceptance in matches:
                matched_acceptance_indexes.add(int(acceptance.get("index") or 0))
                expired.append(
                    {
                        "finding": compact_osv_finding(finding),
                        "acceptance": compact_osv_acceptance(acceptance),
                    }
                )
        blocking.append(finding)

    unused = [
        compact_osv_acceptance(acceptance)
        for acceptance in acceptances
        if int(acceptance.get("index") or 0) not in matched_acceptance_indexes
    ]
    return {
        "blocking_findings": blocking,
        "accepted_findings": accepted,
        "expired_acceptances": expired,
        "unused_acceptances": unused,
    }


def build_osv_components(
    cargo_packages: list[dict[str, Any]],
    npm_lockfiles: list[Path],
) -> list[dict[str, str]]:
    components = [
        {"ecosystem": "crates.io", "name": package["name"], "version": package["version"]}
        for package in cargo_packages
        if package["source"] != "workspace"
    ]
    for lockfile in npm_lockfiles:
        components.extend(package_lock_components(lockfile))
    return components


def compact_run(result: dict[str, Any]) -> dict[str, Any]:
    return {
        "ok": result["ok"],
        "exit_code": result["exit_code"],
        "command": result["command"],
        "stdout_excerpt": truncate(result.get("stdout") or "", 2000),
        "stderr_excerpt": truncate(result.get("stderr") or "", 2000),
    }


def severity_counts(items: list[dict[str, Any]]) -> dict[str, int]:
    counts: dict[str, int] = {}
    for item in items:
        severity = item.get("severity") or "unknown"
        counts[severity] = counts.get(severity, 0) + 1
    return counts


def osv_finding_is_blocking(finding: dict[str, Any]) -> bool:
    database_specific = finding.get("database_specific") or {}
    informational = str(database_specific.get("informational") or "").lower()
    if informational in {"unmaintained", "notice"}:
        return False

    identifiers = [finding.get("id") or "", *(finding.get("aliases") or [])]
    if any(identifier.startswith(("CVE-", "GHSA-")) for identifier in identifiers):
        return True
    if finding.get("severity"):
        return True
    return informational not in {"", "none"}


def artifact_slug(value: str) -> str:
    slug = re.sub(r"[^A-Za-z0-9_.-]+", "-", value.strip())
    return slug.strip("-") or "dependency"


def load_dependency_context(repo_root: Path, package: str, *, online: bool) -> dict[str, Any]:
    tracked_files = discover_tracked_files(repo_root)
    npm_lockfile_paths = sorted(
        path
        for path in tracked_files
        if re.search(r"(^|/)(package-lock\.json|pnpm-lock\.yaml|yarn\.lock)$", path)
        and "/node_modules/" not in f"/{path}"
    )
    npm_lockfiles = [repo_root / path for path in npm_lockfile_paths]

    checks: list[dict[str, Any]] = []
    errors: list[str] = []
    metadata: dict[str, Any] = {}
    metadata_result = run_command(["cargo", "metadata", "--locked", "--format-version", "1", "--no-deps"], repo_root)
    checks.append(check_record("cargo metadata --locked", metadata_result, required=True, include_output=False))
    if metadata_result["ok"]:
        metadata = json.loads(metadata_result["stdout"])
    else:
        errors.append("cargo metadata failed")

    lock_data: dict[str, Any] = {}
    lock_path = repo_root / "Cargo.lock"
    if lock_path.is_file():
        try:
            lock_data = load_toml(lock_path)
        except Exception as err:
            errors.append(f"Cargo.lock parse failed: {err}")
    else:
        errors.append("Cargo.lock is missing")

    direct_rows = direct_dependency_rows(metadata, repo_root) if metadata else []
    cargo_packages = locked_packages(lock_data) if lock_data else []
    duplicates = duplicate_version_groups(cargo_packages)
    locked_by_name = locked_versions_by_name(cargo_packages)
    latest_versions: dict[str, str] = {}
    latest_errors: list[dict[str, str]] = []
    if online:
        latest_versions, latest_errors = query_crates_latest([package])

    npm_reports = [parse_package_lock(lockfile, repo_root) for lockfile in npm_lockfiles]
    return {
        "checks": checks,
        "errors": errors,
        "tracked_files": tracked_files,
        "direct_rows": direct_rows,
        "cargo_packages": cargo_packages,
        "duplicates": duplicates,
        "locked_by_name": locked_by_name,
        "latest_versions": latest_versions,
        "latest_errors": latest_errors,
        "npm_lockfiles": npm_lockfiles,
        "npm_reports": npm_reports,
    }


def cargo_reverse_dependency_reports(repo_root: Path, package: str, versions: list[str]) -> list[dict[str, Any]]:
    reports: list[dict[str, Any]] = []
    specs = [package] if len(versions) <= 1 else [f"{package}@{version}" for version in versions]
    for spec in specs:
        result = run_command(["cargo", "tree", "-i", spec, "--locked"], repo_root, timeout=120)
        reports.append(
            {
                "package_spec": spec,
                "ok": result["ok"],
                "exit_code": result["exit_code"],
                "command": result["command"],
                "tree_excerpt": truncate(result.get("stdout") or result.get("stderr") or "", 6000),
            }
        )
    return reports


def direct_npm_lockfile_matches(npm_reports: list[dict[str, Any]], package: str) -> list[dict[str, Any]]:
    matches: list[dict[str, Any]] = []
    for report in npm_reports:
        direct = [
            dependency
            for dependency in report.get("direct_dependencies") or []
            if dependency.get("name") == package
        ]
        if direct:
            matches.append(
                {
                    "path": report.get("path"),
                    "project_name": report.get("project_name"),
                    "direct_dependencies": direct,
                }
            )
    return matches


def locked_npm_versions(lockfile: Path, package: str) -> list[str]:
    versions = {
        component["version"]
        for component in package_lock_components(lockfile)
        if component["name"] == package
    }
    return sorted(versions, key=version_sort_key)


def npm_latest_version(package: str) -> tuple[str | None, str | None]:
    if not shutil.which("npm"):
        return None, "npm is not installed"
    result = run_command(["npm", "view", package, "version", "--json"], Path.cwd(), timeout=120)
    if not result["ok"]:
        return None, result.get("stderr") or result.get("stdout") or "npm view failed"
    try:
        value = json.loads(result.get("stdout") or "null")
    except json.JSONDecodeError as err:
        return None, f"npm view parse failed: {err}"
    return str(value) if value else None, None


def npm_lockfile_plan(repo_root: Path, lockfiles: list[Path], package: str, *, online: bool) -> dict[str, Any]:
    lockfile_rows: list[dict[str, Any]] = []
    latest: str | None = None
    latest_error: str | None = None
    if online:
        latest, latest_error = npm_latest_version(package)
    for lockfile in lockfiles:
        report = parse_package_lock(lockfile, repo_root)
        direct = [
            dependency
            for dependency in report.get("direct_dependencies") or []
            if dependency.get("name") == package
        ]
        versions = locked_npm_versions(lockfile, package)
        if direct or versions:
            lockfile_rows.append(
                {
                    "path": rel(lockfile, repo_root),
                    "root": rel(lockfile.parent, repo_root),
                    "project_name": report.get("project_name"),
                    "direct_dependencies": direct,
                    "locked_versions": versions,
                    "latest_version": latest,
                }
            )
    return {
        "lockfiles": lockfile_rows,
        "latest_version": latest,
        "latest_error": latest_error,
    }


def rust_upgrade_advice_for_package(
    package: str,
    direct_rows: list[dict[str, Any]],
    duplicates: list[dict[str, Any]],
    locked_by_name: dict[str, list[str]],
    latest_versions: dict[str, str],
) -> list[dict[str, Any]]:
    return [
        item
        for item in build_upgrade_advice(direct_rows, duplicates, locked_by_name, latest_versions)
        if item.get("package") == package
    ]


def infer_dependency_risk(
    package: str,
    target: str | None,
    rust_rows: list[dict[str, Any]],
    locked_versions: list[str],
    duplicate: dict[str, Any] | None,
    advice: list[dict[str, Any]],
) -> dict[str, Any]:
    reasons: list[str] = []
    severity = "low"
    if advice:
        severity = advice[0].get("severity") or severity
        reasons.extend(item.get("reason") for item in advice if item.get("reason"))
    if duplicate and duplicate.get("compatibility_line_count", 0) > 1:
        severity = "medium" if severity == "low" else severity
        reasons.append("multiple incompatible locked versions are present")
    if target and locked_versions:
        target_key = compatibility_key(target)
        current_keys = {compatibility_key(version) for version in locked_versions}
        if target_key not in current_keys:
            severity = "high" if any(row.get("workspace_crate") in {"backend", "appfw-runtime"} for row in rust_rows) else "medium"
            reasons.append("target version crosses the current compatibility line")
    if package in {"async-graphql", "axum", "tower", "tower-http", "reqwest", "jsonwebtoken"}:
        severity = "high"
        reasons.append("package participates in runtime ingress, auth, or transport behavior")
    return {"level": severity, "reasons": sorted(set(reasons))}


def recommended_commands_for_plan(plan: dict[str, Any]) -> list[str]:
    commands: list[str] = []
    rust = plan.get("rust") or {}
    for crate in rust.get("impacted_workspace_crates") or []:
        commands.append(f"cargo check --locked -p {crate}")
    npm = plan.get("npm") or {}
    for lockfile in npm.get("lockfiles") or []:
        root = lockfile.get("root")
        if root:
            commands.append(f"npm --prefix {root} audit --json --package-lock-only")
            commands.append(f"npm --prefix {root} run build --if-present")
    commands.extend(
        [
            "scripts/appfw validate --json",
            "scripts/appfw test --fast",
            "scripts/appfw dependency-check --json --strict",
        ]
    )
    return list(dict.fromkeys(commands))


def build_dependency_plan(args: argparse.Namespace) -> tuple[dict[str, Any], int]:
    repo_root = args.repo_root.resolve()
    report_dir = args.report_dir.resolve()
    report_dir.mkdir(parents=True, exist_ok=True)
    package = args.package
    target = args.target
    online = args.online and not args.offline
    artifact_path = report_dir / f"dependency-plan-{artifact_slug(package)}.json"
    context = load_dependency_context(repo_root, package, online=online)

    direct_rows = [row for row in context["direct_rows"] if row.get("name") == package]
    locked_versions = context["locked_by_name"].get(package, [])
    duplicate = next((item for item in context["duplicates"] if item.get("name") == package), None)
    latest_versions = context["latest_versions"]
    advice = rust_upgrade_advice_for_package(
        package,
        context["direct_rows"],
        context["duplicates"],
        context["locked_by_name"],
        latest_versions,
    )
    reverse_dependencies = cargo_reverse_dependency_reports(repo_root, package, locked_versions) if locked_versions else []
    npm = npm_lockfile_plan(repo_root, context["npm_lockfiles"], package, online=online)
    if target is None:
        target = latest_versions.get(package) or npm.get("latest_version")

    impacted_workspace_crates = sorted({row["workspace_crate"] for row in direct_rows if row.get("workspace_crate")})
    rust = {
        "present": bool(direct_rows or locked_versions),
        "direct_requirements": direct_rows,
        "locked_versions": locked_versions,
        "latest_version": latest_versions.get(package),
        "latest_errors": context["latest_errors"],
        "duplicate_version_group": duplicate,
        "reverse_dependencies": reverse_dependencies,
        "upgrade_advice": advice,
        "impacted_workspace_crates": impacted_workspace_crates,
        "manifest_update_needed": bool(
            target
            and direct_rows
            and any(latest_compatible_with_req(row.get("requirement") or "", target) is False for row in direct_rows)
        ),
    }
    plan = {
        "command": "dependency-plan",
        "ok": not context["errors"] and bool(rust["present"] or npm["lockfiles"]),
        "generated_at_utc": utc_now(),
        "mode": {
            "online": online,
            "offline": not online,
        },
        "repo_root": repo_root.as_posix(),
        "artifact": rel(artifact_path, repo_root),
        "package": package,
        "target_version": target,
        "checks": context["checks"],
        "rust": rust,
        "npm": npm,
        "risk": infer_dependency_risk(package, target, direct_rows, locked_versions, duplicate, advice),
        "recommended_commands": [],
        "plan_steps": [],
        "warnings": [],
        "errors": context["errors"],
    }
    plan["recommended_commands"] = recommended_commands_for_plan(plan)
    if not rust["present"] and not npm["lockfiles"]:
        plan["errors"].append(f"package `{package}` was not found in Cargo.lock or tracked npm lockfiles")
    if rust["present"]:
        plan["plan_steps"].append("Review reverse dependency tree and direct manifest owners.")
        if rust["manifest_update_needed"]:
            plan["plan_steps"].append("Update direct Cargo.toml requirements before refreshing Cargo.lock.")
        else:
            plan["plan_steps"].append("Refresh Cargo.lock with an intentional cargo update for this package.")
    if npm["lockfiles"]:
        plan["plan_steps"].append("Refresh affected npm package-lock roots with package-lock-only install.")
    if not online:
        plan["warnings"].append({"category": "latest-version", "detail": "run with --online to populate latest registry versions"})

    artifact_path.write_text(json.dumps(plan, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return plan, 0 if plan["ok"] else 1


def cargo_manifest_dependency_key(row: dict[str, Any]) -> str:
    return row.get("rename") or row.get("name") or ""


def replace_cargo_dependency_version(text: str, dependency_key: str, target: str) -> tuple[str, bool]:
    escaped = re.escape(dependency_key)
    changed = False
    lines: list[str] = []
    for line in text.splitlines(keepends=True):
        if not re.match(rf"^\s*{escaped}\s*=", line):
            lines.append(line)
            continue
        if re.search(r'version\s*=\s*"[^"]+"', line):
            new_line = re.sub(r'version\s*=\s*"[^"]+"', f'version = "{target}"', line, count=1)
        else:
            new_line = re.sub(r'=\s*"[^"]+"', f'= "{target}"', line, count=1)
        if new_line != line:
            changed = True
        lines.append(new_line)
    return "".join(lines), changed


def update_cargo_manifests(repo_root: Path, direct_rows: list[dict[str, Any]], target: str) -> list[dict[str, Any]]:
    actions: list[dict[str, Any]] = []
    for row in direct_rows:
        manifest = repo_root / row["manifest"]
        dependency_key = cargo_manifest_dependency_key(row)
        original = manifest.read_text(encoding="utf-8")
        updated, changed = replace_cargo_dependency_version(original, dependency_key, target)
        if changed:
            manifest.write_text(updated, encoding="utf-8")
        actions.append(
            {
                "kind": "cargo_manifest_update",
                "path": row["manifest"],
                "dependency_key": dependency_key,
                "from": row.get("requirement"),
                "to": target,
                "changed": changed,
            }
        )
    return actions


def npm_save_flag(kind: str) -> str:
    return {
        "dependencies": "--save-prod",
        "devDependencies": "--save-dev",
        "optionalDependencies": "--save-optional",
    }.get(kind, "--save")


def npm_upgrade_commands(
    repo_root: Path,
    npm_plan: dict[str, Any],
    package: str,
    target: str,
    *,
    dry_run: bool,
    offline: bool,
) -> list[tuple[Path, list[str]]]:
    commands: list[tuple[Path, list[str]]] = []
    for lockfile in npm_plan.get("lockfiles") or []:
        root = repo_root / lockfile["root"]
        direct = lockfile.get("direct_dependencies") or []
        flag = npm_save_flag(direct[0].get("kind") if direct else "dependencies")
        command = [
            "npm",
            "install",
            f"{package}@{target}",
            flag,
            "--package-lock-only",
            "--ignore-scripts",
        ]
        if dry_run:
            command.append("--dry-run")
        if offline:
            command.append("--offline")
        commands.append((root, command))
    return commands


def run_verification_commands(repo_root: Path, commands: list[str]) -> list[dict[str, Any]]:
    verification: list[dict[str, Any]] = []
    for command in commands:
        args = command.split()
        result = run_command(args, repo_root, timeout=600)
        verification.append(check_record(command, result, required=True))
        if not result["ok"]:
            break
    return verification


def build_dependency_upgrade(args: argparse.Namespace) -> tuple[dict[str, Any], int]:
    plan_args = argparse.Namespace(
        repo_root=args.repo_root,
        report_dir=args.report_dir,
        package=args.package,
        target=args.target,
        online=args.online or bool(args.apply),
        offline=args.offline,
    )
    plan, plan_exit = build_dependency_plan(plan_args)
    repo_root = args.repo_root.resolve()
    report_dir = args.report_dir.resolve()
    artifact_path = report_dir / f"dependency-upgrade-{artifact_slug(args.package)}.json"
    target = args.target or plan.get("target_version")
    actions: list[dict[str, Any]] = []
    checks: list[dict[str, Any]] = []
    errors: list[str] = []
    dry_run = not args.apply

    if plan_exit != 0:
        errors.extend(plan.get("errors") or ["dependency plan failed"])
    if not target:
        errors.append("target version is required; pass --target or run with --online so latest can be resolved")

    if target and not errors:
        rust = plan.get("rust") or {}
        if rust.get("present"):
            if args.apply:
                actions.extend(update_cargo_manifests(repo_root, rust.get("direct_requirements") or [], target))
            locked_versions = rust.get("locked_versions") or []
            package_specs = [
                f"{args.package}@{version}"
                for version in locked_versions
                if version != target
            ] or [args.package]
            for package_spec in package_specs:
                update_args = ["cargo", "update", "-p", package_spec, "--precise", target]
                if dry_run or args.check:
                    update_args.append("--dry-run")
                if args.offline:
                    update_args.append("--offline")
                result = run_command(update_args, repo_root, timeout=300)
                checks.append(check_record(f"cargo update {package_spec}", result, required=True))
                if not result["ok"]:
                    errors.append(f"cargo update failed for {package_spec}")

        npm = plan.get("npm") or {}
        for cwd, command in npm_upgrade_commands(
            repo_root,
            npm,
            args.package,
            target,
            dry_run=dry_run or args.check,
            offline=args.offline,
        ):
            result = run_command(command, cwd, timeout=300)
            checks.append(check_record(f"npm install {rel(cwd, repo_root)}", result, required=True))
            if not result["ok"]:
                errors.append(f"npm install failed for {rel(cwd, repo_root)}")

    verification: list[dict[str, Any]] = []
    if args.run_tests and not errors and args.apply:
        verification = run_verification_commands(repo_root, plan.get("recommended_commands") or [])
        if any(not item.get("ok") for item in verification):
            errors.append("one or more verification commands failed")

    report = {
        "command": "dependency-upgrade",
        "ok": not errors,
        "generated_at_utc": utc_now(),
        "mode": {
            "apply": bool(args.apply),
            "check": bool(args.check),
            "dry_run": dry_run,
            "run_tests": bool(args.run_tests),
        },
        "repo_root": repo_root.as_posix(),
        "artifact": rel(artifact_path, repo_root),
        "package": args.package,
        "target_version": target,
        "plan_artifact": plan.get("artifact"),
        "actions": actions,
        "checks": checks,
        "verification": verification,
        "recommended_commands": plan.get("recommended_commands") or [],
        "errors": errors,
        "warnings": plan.get("warnings") or [],
    }
    artifact_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return report, 0 if report["ok"] else 1


def build_report(args: argparse.Namespace) -> tuple[dict[str, Any], int]:
    repo_root = args.repo_root.resolve()
    report_dir = args.report_dir.resolve()
    report_dir.mkdir(parents=True, exist_ok=True)
    artifact_path = report_dir / "dependency-check.json"
    online = args.online or (args.strict and not args.offline)
    if args.offline:
        online = False
    dependency_policy = load_dependency_policy(repo_root, args.policy_file)

    tracked_files = discover_tracked_files(repo_root)
    cargo_manifests = sorted(path for path in tracked_files if path.endswith("Cargo.toml"))
    npm_lockfile_paths = sorted(
        path
        for path in tracked_files
        if re.search(r"(^|/)(package-lock\.json|pnpm-lock\.yaml|yarn\.lock)$", path)
        and "/node_modules/" not in f"/{path}"
    )
    npm_lockfiles = [repo_root / path for path in npm_lockfile_paths]
    mobile_release_gates: dict[Path, dict[str, Any]] = {}
    release_gated_npm_components: set[tuple[str, str, str]] = set()
    normal_npm_components: set[tuple[str, str, str]] = set()
    for lockfile in npm_lockfiles:
        gate = mobile_runtime_audit_release_gate(lockfile, repo_root)
        if gate:
            mobile_release_gates[lockfile] = gate
        for component in package_lock_components(lockfile):
            key = npm_component_key(component)
            if not key:
                continue
            if gate:
                release_gated_npm_components.add(key)
            else:
                normal_npm_components.add(key)

    checks: list[dict[str, Any]] = []
    warnings: list[dict[str, str]] = []
    blocking: list[dict[str, str]] = []
    for err in dependency_policy.get("errors") or []:
        blocking.append({"category": "dependency-policy", "detail": err})

    advisory_tool_env, advisory_tool_environment = dependency_tool_environment(repo_root, report_dir)
    if not advisory_tool_environment["cargo_home_writable"]:
        detail = (
            "dependency advisory Cargo home is not writable: "
            f"{advisory_tool_environment['cargo_home']}"
        )
        warnings.append({"category": "tooling", "detail": detail})
        if args.strict:
            blocking.append({"category": "tooling", "detail": detail})

    metadata_result = run_command(["cargo", "metadata", "--locked", "--format-version", "1", "--no-deps"], repo_root)
    checks.append(check_record("cargo metadata --locked", metadata_result, required=True, include_output=False))
    metadata: dict[str, Any] = {}
    if metadata_result["ok"]:
        metadata = json.loads(metadata_result["stdout"])
    else:
        blocking.append({"category": "rust", "detail": "cargo metadata failed"})

    lock_path = repo_root / "Cargo.lock"
    lock_data: dict[str, Any] = {}
    if lock_path.is_file():
        try:
            lock_data = load_toml(lock_path)
        except Exception as err:
            blocking.append({"category": "rust", "detail": f"Cargo.lock parse failed: {err}"})
    else:
        blocking.append({"category": "rust", "detail": "Cargo.lock is missing"})

    tree_result = run_command(["cargo", "tree", "-d", "--locked"], repo_root, timeout=240)
    checks.append(check_record("cargo tree duplicate versions", tree_result, required=False, include_output=False))
    if not tree_result["ok"] and tree_result["exit_code"] != 0:
        warnings.append({"category": "rust", "detail": "cargo tree duplicate-version scan failed"})

    direct_rows = direct_dependency_rows(metadata, repo_root) if metadata else []
    cargo_packages = locked_packages(lock_data) if lock_data else []
    duplicates = duplicate_version_groups(cargo_packages)
    locked_by_name = locked_versions_by_name(cargo_packages)

    cargo_audit_status: dict[str, Any] = {"status": "not_run"}
    if shutil.which("cargo-audit"):
        audit_args = ["cargo", "audit", "--json"]
        if not online:
            audit_args.extend(["--no-fetch", "--stale"])
        audit_result = run_command(audit_args, repo_root, timeout=240, env=advisory_tool_env)
        cargo_audit_status = parse_cargo_audit(audit_result.get("stdout") or "")
        cargo_audit_status["run"] = compact_run(audit_result)
        required = args.strict or online
        checks.append(check_record("cargo audit", audit_result, required=required))
        if cargo_audit_status.get("vulnerability_count", 0) > 0:
            blocking.append({"category": "rustsec", "detail": "cargo audit found vulnerabilities"})
        elif not audit_result["ok"] and required:
            blocking.append({"category": "rustsec", "detail": "cargo audit did not complete"})
        elif not audit_result["ok"]:
            warnings.append({"category": "rustsec", "detail": "cargo audit unavailable in offline/local mode"})
    else:
        cargo_audit_status = {"status": "missing_tool"}
        warnings.append({"category": "tooling", "detail": "cargo-audit is not installed"})
        if args.strict:
            blocking.append({"category": "tooling", "detail": "cargo-audit is required in strict mode"})

    cargo_deny_status: dict[str, Any] = {"status": "not_run"}
    if shutil.which("cargo-deny"):
        deny_args = ["cargo", "deny", "-f", "json", "--all-features"]
        if not online:
            deny_args.append("--offline")
        deny_args.extend(["check", "advisories", "bans", "sources", "licenses"])
        deny_result = run_command(deny_args, repo_root, timeout=300, env=advisory_tool_env)
        required = args.strict or online
        checks.append(check_record("cargo deny policy", deny_result, required=required))
        cargo_deny_status = {
            "status": "passed" if deny_result["ok"] else "failed",
            "run": compact_run(deny_result),
        }
        if not deny_result["ok"] and required:
            blocking.append({"category": "cargo-deny", "detail": "cargo deny policy check failed"})
        elif not deny_result["ok"]:
            warnings.append({"category": "cargo-deny", "detail": "cargo deny unavailable in offline/local mode"})
    else:
        cargo_deny_status = {"status": "missing_tool"}
        warnings.append({"category": "tooling", "detail": "cargo-deny is not installed"})
        if args.strict:
            blocking.append({"category": "tooling", "detail": "cargo-deny is required in strict mode"})

    npm_reports: list[dict[str, Any]] = []
    for lockfile in npm_lockfiles:
        lock_report = parse_package_lock(lockfile, repo_root)
        mobile_release_gate = mobile_release_gates.get(lockfile)
        if mobile_release_gate:
            lock_report["mobile_release_gate"] = mobile_release_gate
        if online and lockfile.name == "package-lock.json":
            if shutil.which("npm"):
                audit_result = run_command(["npm", "audit", "--json", "--package-lock-only"], lockfile.parent, timeout=240)
                parsed = parse_npm_audit(audit_result.get("stdout") or "")
                lock_report["audit"] = parsed
                lock_report["audit"]["run"] = compact_run(audit_result)
                if mobile_release_gate:
                    lock_report["audit"]["release_gated"] = mobile_release_gate
                checks.append(
                    check_record(
                        f"npm audit {rel(lockfile.parent, repo_root)}",
                        audit_result,
                        required=args.strict and mobile_release_gate is None,
                    )
                )
                if parsed.get("vulnerability_count", 0) > 0:
                    if mobile_release_gate:
                        warnings.append(
                            {
                                "category": "mobile-runtime-audit",
                                "detail": (
                                    "npm audit found vulnerabilities in "
                                    f"{rel(lockfile, repo_root)}; mobile-test retains this surface diagnostically for the future candidate gate"
                                ),
                            }
                        )
                    else:
                        today = dt.datetime.now(dt.timezone.utc).date()
                        leaves = parsed.get("leaf_advisories") or []
                        unaccepted_leaves = [
                            leaf
                            for leaf in leaves
                            if not npm_leaf_is_accepted(
                                leaf,
                                dependency_policy.get("osv_acceptances") or [],
                                lockfile,
                                today,
                            )
                        ]
                        if leaves and not unaccepted_leaves:
                            parsed["accepted_leaf_advisories"] = leaves
                            warnings.append(
                                {
                                    "category": "npm-accepted",
                                    "detail": (
                                        "npm audit vulnerabilities in "
                                        f"{rel(lockfile, repo_root)} are covered by active "
                                        "dependency-check.toml OSV acceptances: "
                                        + ", ".join(sorted({leaf["id"] for leaf in leaves}))
                                    ),
                                }
                            )
                        else:
                            blocking.append({"category": "npm", "detail": f"npm audit found vulnerabilities in {rel(lockfile, repo_root)}"})
                elif not audit_result["ok"] and args.strict:
                    if mobile_release_gate:
                        warnings.append(
                            {
                                "category": "mobile-runtime-audit",
                                "detail": (
                                    "npm audit failed for "
                                    f"{rel(lockfile, repo_root)}; mobile-test retains this surface diagnostically for the future candidate gate"
                                ),
                            }
                        )
                    else:
                        blocking.append({"category": "npm", "detail": f"npm audit failed for {rel(lockfile, repo_root)}"})

                outdated_result = run_command(["npm", "outdated", "--json"], lockfile.parent, timeout=180)
                outdated_payload: dict[str, Any] = {}
                if outdated_result.get("stdout", "").strip():
                    try:
                        outdated_payload = json.loads(outdated_result["stdout"])
                    except json.JSONDecodeError as err:
                        outdated_payload = {"parse_error": str(err)}
                lock_report["outdated"] = {
                    "status": "parsed" if outdated_payload else "none_reported",
                    "package_count": len(outdated_payload) if isinstance(outdated_payload, dict) else 0,
                    "packages": outdated_payload,
                    "run": compact_run(outdated_result),
                }
                outdated_check = dict(outdated_result)
                if outdated_check["exit_code"] in (0, 1):
                    outdated_check["ok"] = True
                checks.append(
                    check_record(
                        f"npm outdated {rel(lockfile.parent, repo_root)}",
                        outdated_check,
                        required=False,
                        status="passed" if outdated_result["exit_code"] in (0, 1) else "failed",
                    )
                )
            else:
                lock_report["audit"] = {"status": "missing_tool"}
                warnings.append({"category": "tooling", "detail": "npm is not installed"})
                if args.strict:
                    blocking.append({"category": "tooling", "detail": "npm is required in strict mode for npm lockfiles"})
        else:
            lock_report["audit"] = {
                "status": "skipped_offline" if not online else "unsupported_lockfile",
                "detail": "run with --online to call npm audit" if not online else "only package-lock.json audit is currently supported",
            }
        npm_reports.append(lock_report)

    latest_versions: dict[str, str] = {}
    latest_errors: list[dict[str, str]] = []
    if online:
        direct_names = sorted({row["name"] for row in direct_rows if row.get("name")})
        latest_versions, latest_errors = query_crates_latest(direct_names)
        for err in latest_errors:
            warnings.append({"category": "crates.io", "detail": f"{err['package']}: {err['error']}"})

    upgrade_advice = build_upgrade_advice(direct_rows, duplicates, locked_by_name, latest_versions)

    osv_report: dict[str, Any] = {
        "status": "skipped_offline",
        "finding_count": 0,
        "blocking_finding_count": 0,
        "accepted_finding_count": 0,
        "accepted_findings": [],
        "expired_acceptances": [],
        "unused_acceptances": [],
    }
    if online:
        components = build_osv_components(cargo_packages, npm_lockfiles)
        findings, osv_errors = query_osv(components)
        osv_classification = classify_osv_findings(
            findings,
            dependency_policy.get("osv_acceptances") or [],
        )
        blocking_osv_findings = []
        release_gated_osv_findings = []
        for finding in osv_classification["blocking_findings"]:
            key = (
                str(finding.get("ecosystem") or ""),
                str(finding.get("package") or ""),
                str(finding.get("version") or ""),
            )
            if (
                key[0] == "npm"
                and key in release_gated_npm_components
                and key not in normal_npm_components
            ):
                release_gated_osv_findings.append(compact_osv_finding(finding))
            else:
                blocking_osv_findings.append(finding)
        osv_report = {
            "status": "queried" if not osv_errors else "partial" if findings else "failed",
            "component_count": len(components),
            "finding_count": len(findings),
            "blocking_finding_count": len(blocking_osv_findings),
            "release_gated_finding_count": len(release_gated_osv_findings),
            "accepted_finding_count": len(osv_classification["accepted_findings"]),
            "findings": findings,
            "release_gated_findings": release_gated_osv_findings,
            "accepted_findings": osv_classification["accepted_findings"],
            "expired_acceptances": osv_classification["expired_acceptances"],
            "unused_acceptances": osv_classification["unused_acceptances"],
            "errors": osv_errors,
        }
        if blocking_osv_findings:
            blocking.append({"category": "osv", "detail": "OSV found vulnerable package versions"})
        if osv_classification["expired_acceptances"]:
            blocking.append({"category": "osv-policy", "detail": "OSV policy has expired accepted findings"})
        if osv_errors:
            for err in osv_errors:
                warnings.append({"category": "osv", "detail": err})
            if args.strict:
                blocking.append({"category": "osv", "detail": "OSV query failed in strict mode"})

    duplicate_compatibility_count = sum(1 for item in duplicates if item["compatibility_line_count"] > 1)
    report = {
        "command": "dependency-check",
        "ok": not blocking,
        "generated_at_utc": utc_now(),
        "mode": {
            "online": online,
            "strict": bool(args.strict),
            "offline": not online,
        },
        "repo_root": repo_root.as_posix(),
        "artifact": rel(artifact_path, repo_root),
        "summary": {
            "cargo_manifest_count": len(cargo_manifests),
            "cargo_locked_package_count": len(cargo_packages),
            "rust_direct_dependency_count": len(direct_rows),
            "duplicate_version_group_count": len(duplicates),
            "duplicate_compatibility_line_count": duplicate_compatibility_count,
            "npm_lockfile_count": len(npm_lockfiles),
            "upgrade_advice_count": len(upgrade_advice),
            "upgrade_advice_by_severity": severity_counts(upgrade_advice),
            "accepted_osv_finding_count": int(osv_report.get("accepted_finding_count") or 0),
            "release_gated_osv_finding_count": int(osv_report.get("release_gated_finding_count") or 0),
            "mobile_release_gated_lockfile_count": len(mobile_release_gates),
            "blocking_finding_count": len(blocking),
        },
        "inputs": {
            "cargo_lock": "Cargo.lock" if lock_path.is_file() else None,
            "cargo_manifests": cargo_manifests,
            "npm_lockfiles": npm_lockfile_paths,
        },
        "policy": {
            "dependency_policy_file": dependency_policy.get("path"),
            "dependency_policy_status": dependency_policy.get("status"),
            "osv_acceptance_count": len(dependency_policy.get("osv_acceptances") or []),
            "blocks_on": [
                "known RustSec vulnerabilities",
                "cargo-deny policy failures in strict or online mode",
                "npm audit vulnerabilities in online mode except product mobile runtime audit findings retained by the non-authoritative mobile-test diagnostic for the future candidate gate",
                "unaccepted OSV vulnerable package matches in online mode",
                "expired or invalid dependency-check policy acceptances",
                "missing advisory tooling in strict mode",
            ],
            "release_gated": [
                "product mobile runtime audit findings retained diagnostically by mobile-test until a future source-bound candidate checker requires a clean audit or approved disposition",
                "OSV npm findings that occur only inside product mobile lockfiles with retained U5 mobile audit evidence",
            ],
            "advisory_only": [
                "duplicate crate compatibility lines",
                "outdated direct dependencies",
                "reviewed unmaintained dependency debt",
                "offline skipped online checks outside strict mode",
            ],
        },
        "tool_environment": {
            "advisory_tools": advisory_tool_environment,
        },
        "checks": checks,
        "rust": {
            "direct_dependencies": direct_rows,
            "duplicate_versions": duplicates,
            "cargo_audit": cargo_audit_status,
            "cargo_deny": cargo_deny_status,
            "latest_versions": {
                "status": "queried" if online else "skipped_offline",
                "packages": latest_versions,
                "errors": latest_errors,
            },
            "upgrade_advice": upgrade_advice,
        },
        "node": {
            "lockfiles": npm_reports,
        },
        "osv": osv_report,
        "warnings": warnings,
        "blocking_findings": blocking,
    }

    artifact_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return report, 0 if report["ok"] else 1


def print_text(report: dict[str, Any]) -> None:
    status = "ok" if report["ok"] else "attention required"
    summary = report["summary"]
    print(f"dependency-check: {status}")
    print(f"  artifact: {report['artifact']}")
    print(f"  mode: {'online' if report['mode']['online'] else 'offline'}")
    print(f"  cargo manifests: {summary['cargo_manifest_count']}")
    print(f"  locked Rust packages: {summary['cargo_locked_package_count']}")
    print(f"  duplicate version groups: {summary['duplicate_version_group_count']}")
    print(f"  upgrade advice: {summary['upgrade_advice_count']}")
    advisory_env = report.get("tool_environment", {}).get("advisory_tools", {})
    if advisory_env:
        print(
            "  advisory cargo home: "
            f"{advisory_env.get('cargo_home')} ({advisory_env.get('cargo_home_source')})"
        )
    if summary.get("accepted_osv_finding_count"):
        print(f"  accepted OSV findings: {summary['accepted_osv_finding_count']}")
    print(f"  npm lockfiles: {summary['npm_lockfile_count']}")
    if report["blocking_findings"]:
        print("  blocking findings:")
        for finding in report["blocking_findings"]:
            print(f"    - {finding['category']}: {finding['detail']}")
    if report["rust"]["upgrade_advice"]:
        print("  top upgrade advice:")
        for item in report["rust"]["upgrade_advice"][:8]:
            print(f"    - [{item['severity']}] {item['package']}: {item['recommendation']}")
    if report["warnings"]:
        print("  warnings:")
        for warning in report["warnings"][:8]:
            print(f"    - {warning['category']}: {warning['detail']}")


def print_plan_text(report: dict[str, Any]) -> None:
    status = "ok" if report["ok"] else "attention required"
    print(f"dependency-plan: {status}")
    print(f"  package: {report['package']}")
    print(f"  artifact: {report['artifact']}")
    print(f"  target: {report.get('target_version') or 'unknown'}")
    print(f"  risk: {report['risk']['level']}")
    rust = report.get("rust") or {}
    npm = report.get("npm") or {}
    if rust.get("present"):
        print(f"  rust locked versions: {', '.join(rust.get('locked_versions') or []) or 'none'}")
        owners = rust.get("impacted_workspace_crates") or []
        if owners:
            print(f"  rust direct owners: {', '.join(owners)}")
    if npm.get("lockfiles"):
        print("  npm lockfiles:")
        for lockfile in npm["lockfiles"]:
            print(f"    - {lockfile['path']}: {', '.join(lockfile.get('locked_versions') or []) or 'direct only'}")
    if report.get("plan_steps"):
        print("  plan steps:")
        for step in report["plan_steps"]:
            print(f"    - {step}")
    if report.get("recommended_commands"):
        print("  recommended verification:")
        for command in report["recommended_commands"][:8]:
            print(f"    - {command}")
    if report.get("errors"):
        print("  errors:")
        for error_message in report["errors"]:
            print(f"    - {error_message}")


def print_upgrade_text(report: dict[str, Any]) -> None:
    status = "ok" if report["ok"] else "attention required"
    mode = "apply" if report["mode"]["apply"] else "dry-run"
    print(f"dependency-upgrade: {status}")
    print(f"  package: {report['package']}")
    print(f"  target: {report.get('target_version') or 'unknown'}")
    print(f"  mode: {mode}")
    print(f"  artifact: {report['artifact']}")
    if report.get("checks"):
        print("  checks:")
        for check in report["checks"]:
            print(f"    - {check['name']}: {'ok' if check['ok'] else 'failed'}")
    if report.get("actions"):
        print("  actions:")
        for action in report["actions"]:
            print(f"    - {action['kind']} {action['path']}: {action.get('from')} -> {action.get('to')}")
    if report.get("errors"):
        print("  errors:")
        for error_message in report["errors"]:
            print(f"    - {error_message}")
    if not report["mode"]["apply"]:
        print("  no files were modified; pass --apply to perform the upgrade")


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Check dependency advisories and upgrade pressure")
    parser.add_argument("--mode", choices=("check", "plan", "upgrade"), default="check")
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--report-dir", type=Path, default=Path("target/appfw"))
    parser.add_argument("--package", help="dependency package name for plan/upgrade modes")
    parser.add_argument("--target", help="target package version for upgrade planning")
    parser.add_argument("--json", action="store_true", help="print JSON report")
    parser.add_argument("--online", action="store_true", help="enable network-backed npm audit, OSV, and registry latest checks")
    parser.add_argument("--offline", action="store_true", help="force local/offline checks")
    parser.add_argument("--strict", action="store_true", help="treat missing advisory tooling and online advisory failures as blocking")
    parser.add_argument("--apply", action="store_true", help="apply dependency-upgrade changes; default is dry-run")
    parser.add_argument("--check", action="store_true", help="run dependency-upgrade manager dry-run checks")
    parser.add_argument("--run-tests", action="store_true", help="after --apply, run recommended verification commands")
    parser.add_argument(
        "--policy-file",
        type=Path,
        default=None,
        help="dependency-check policy file; defaults to dependency-check.toml when present",
    )
    args = parser.parse_args(argv)
    if args.online and args.offline:
        parser.error("--online and --offline cannot be used together")
    if args.mode in {"plan", "upgrade"} and not args.package:
        parser.error(f"--package is required for {args.mode} mode")
    if args.apply and args.check:
        parser.error("--apply cannot be combined with --check")
    if args.run_tests and not args.apply:
        parser.error("--run-tests requires --apply")
    return args


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    if args.mode == "plan":
        report, exit_code = build_dependency_plan(args)
    elif args.mode == "upgrade":
        report, exit_code = build_dependency_upgrade(args)
    else:
        report, exit_code = build_report(args)
    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    elif args.mode == "plan":
        print_plan_text(report)
    elif args.mode == "upgrade":
        print_upgrade_text(report)
    else:
        print_text(report)
    return exit_code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
