#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

python3 - "$@" <<'PY'
import json
import os
import pathlib
import shutil
import signal
import shlex
import subprocess
import sys
import tempfile
import time


REPO_ROOT = pathlib.Path.cwd()
REPORT_DIR = REPO_ROOT / "target" / "appfw"
DETAIL_DIR = REPORT_DIR / "wave3-pr-gates"
SUMMARY_PATH = REPORT_DIR / "wave3-pr-gates.json"
PROGRESS_PATH = DETAIL_DIR / "progress.jsonl"
DETAIL_DIR.mkdir(parents=True, exist_ok=True)

args = set(sys.argv[1:])
reuse_docs_check = "--reuse-docs-check" in args


def env_int(name: str, default: int, minimum: int = 1) -> int:
    raw = os.environ.get(name)
    if raw is None or raw.strip() == "":
        return default
    try:
        value = int(raw)
    except ValueError:
        return default
    return max(minimum, value)


DEFAULT_GATE_TIMEOUT_SECS = env_int("APPFW_WAVE3_GATE_TIMEOUT_SECS", 1800)
NPM_CI_TIMEOUT_SECS = env_int(
    "APPFW_WAVE3_NPM_CI_TIMEOUT_SECS", DEFAULT_GATE_TIMEOUT_SECS
)
NPM_BUILD_TIMEOUT_SECS = env_int(
    "APPFW_WAVE3_NPM_BUILD_TIMEOUT_SECS", DEFAULT_GATE_TIMEOUT_SECS
)
PROGRESS_INTERVAL_SECS = env_int("APPFW_WAVE3_PROGRESS_INTERVAL_SECS", 30)
PROGRESS_PATH.write_text("", encoding="utf-8")


def rel(path: pathlib.Path) -> str:
    try:
        return str(path.relative_to(REPO_ROOT))
    except ValueError:
        return str(path)


def parse_json_text(text: str):
    stripped = text.strip()
    if not stripped:
        return None
    return json.loads(stripped)


def read_json(path: pathlib.Path):
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def write_summary(summary):
    SUMMARY_PATH.parent.mkdir(parents=True, exist_ok=True)
    with SUMMARY_PATH.open("w", encoding="utf-8") as handle:
        json.dump(summary, handle, indent=2, sort_keys=True)
        handle.write("\n")


def utc_now() -> str:
    return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())


def command_text(command) -> str:
    return " ".join(shlex.quote(part) for part in command)


def emit_progress(event: str, name: str, **fields):
    payload = {
        "event": event,
        "name": name,
        "timestamp": utc_now(),
        **fields,
    }
    line = json.dumps(payload, sort_keys=True)
    with PROGRESS_PATH.open("a", encoding="utf-8") as handle:
        handle.write(line)
        handle.write("\n")
    print(line, flush=True)


def terminate_process_group(proc: subprocess.Popen):
    try:
        os.killpg(proc.pid, signal.SIGTERM)
    except ProcessLookupError:
        return
    try:
        proc.wait(timeout=5)
        return
    except subprocess.TimeoutExpired:
        pass
    try:
        os.killpg(proc.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    proc.wait()


def run_process_gate(
    name,
    command,
    cwd=REPO_ROOT,
    allowed_statuses=(0,),
    timeout_secs=None,
):
    stdout_path = DETAIL_DIR / f"{name}.stdout.log"
    stderr_path = DETAIL_DIR / f"{name}.stderr.log"
    resolved_timeout_secs = timeout_secs or DEFAULT_GATE_TIMEOUT_SECS
    command_display = command_text(command)
    cwd_path = pathlib.Path(cwd)
    start = time.monotonic()

    emit_progress(
        "gate-start",
        name,
        command=command_display,
        cwd=rel(cwd_path),
        stdout_log=rel(stdout_path),
        stderr_log=rel(stderr_path),
        timeout_secs=resolved_timeout_secs,
    )

    timed_out = False
    with stdout_path.open("w", encoding="utf-8") as stdout_handle, stderr_path.open(
        "w", encoding="utf-8"
    ) as stderr_handle:
        proc = subprocess.Popen(
            command,
            cwd=cwd,
            text=True,
            stdout=stdout_handle,
            stderr=stderr_handle,
            start_new_session=True,
        )
        deadline = start + resolved_timeout_secs
        last_heartbeat = start
        try:
            while True:
                now = time.monotonic()
                if now >= deadline:
                    timed_out = True
                    emit_progress(
                        "gate-timeout",
                        name,
                        command=command_display,
                        cwd=rel(cwd_path),
                        elapsed_ms=int((now - start) * 1000),
                        timeout_secs=resolved_timeout_secs,
                        stdout_log=rel(stdout_path),
                        stderr_log=rel(stderr_path),
                    )
                    terminate_process_group(proc)
                    break
                if now - last_heartbeat >= PROGRESS_INTERVAL_SECS:
                    emit_progress(
                        "gate-heartbeat",
                        name,
                        command=command_display,
                        cwd=rel(cwd_path),
                        elapsed_ms=int((now - start) * 1000),
                        timeout_secs=resolved_timeout_secs,
                        stdout_log=rel(stdout_path),
                        stderr_log=rel(stderr_path),
                    )
                    last_heartbeat = now
                wait_secs = min(1, max(0.1, deadline - now))
                try:
                    proc.wait(timeout=wait_secs)
                    break
                except subprocess.TimeoutExpired:
                    continue
        except KeyboardInterrupt:
            emit_progress(
                "gate-interrupted",
                name,
                command=command_display,
                cwd=rel(cwd_path),
                elapsed_ms=int((time.monotonic() - start) * 1000),
                stdout_log=rel(stdout_path),
                stderr_log=rel(stderr_path),
            )
            terminate_process_group(proc)
            raise

    elapsed_ms = int((time.monotonic() - start) * 1000)
    exit_code = proc.returncode if proc.returncode is not None else -1
    status_ok = exit_code in allowed_statuses and not timed_out
    emit_progress(
        "gate-finish",
        name,
        command=command_display,
        cwd=rel(cwd_path),
        elapsed_ms=elapsed_ms,
        exit_code=exit_code,
        ok=status_ok,
        timed_out=timed_out,
        stdout_log=rel(stdout_path),
        stderr_log=rel(stderr_path),
    )
    return {
        "name": name,
        "command": command_display,
        "cwd": rel(cwd_path),
        "exit_code": exit_code,
        "allowed_statuses": list(allowed_statuses),
        "elapsed_ms": elapsed_ms,
        "ok": status_ok,
        "status_ok": status_ok,
        "timed_out": timed_out,
        "timeout_secs": resolved_timeout_secs,
        "stdout_log": rel(stdout_path),
        "stderr_log": rel(stderr_path),
    }


def run_gate(name, command, allowed_statuses=(0,), required_ok=True, timeout_secs=None):
    process_gate = run_process_gate(
        name,
        command,
        cwd=REPO_ROOT,
        allowed_statuses=allowed_statuses,
        timeout_secs=timeout_secs,
    )
    stdout_path = DETAIL_DIR / f"{name}.stdout.log"
    stdout_text = stdout_path.read_text(encoding="utf-8")

    parsed = None
    parse_error = None
    try:
        parsed = parse_json_text(stdout_text)
    except json.JSONDecodeError as exc:
        parse_error = str(exc)

    status_ok = bool(process_gate.get("status_ok"))
    json_ok = parsed is not None and parse_error is None
    payload_ok = True if parsed is None else bool(parsed.get("ok", False))
    ok = status_ok and json_ok and (payload_ok if required_ok else True)

    return {
        **process_gate,
        "elapsed_ms": process_gate.get("elapsed_ms"),
        "ok": ok,
        "status_ok": status_ok,
        "json_ok": json_ok,
        "payload_ok": payload_ok,
        "required_payload_ok": required_ok,
        "json": parsed,
        "parse_error": parse_error,
    }


def summarize_payload(payload):
    if not isinstance(payload, dict):
        return None
    summary = {
        "command": payload.get("command"),
        "ok": payload.get("ok"),
        "artifact": payload.get("artifact"),
        "lane": payload.get("lane"),
        "release_ready": payload.get("release_ready"),
    }
    if payload.get("enforced_artifact"):
        summary["enforced_artifact"] = payload.get("enforced_artifact")
    if payload.get("gate"):
        summary["gate"] = payload.get("gate")
    if payload.get("blocking_violations"):
        summary["blocking_violations"] = payload.get("blocking_violations")
    if payload.get("enforcement_violations"):
        summary["enforcement_violations"] = payload.get("enforcement_violations")
    if payload.get("release_authority_package"):
        release_authority = payload.get("release_authority_package") or {}
        summary["release_authority_package"] = {
            "required_for_release": release_authority.get("required_for_release"),
            "local_evidence_satisfies_release": release_authority.get(
                "local_evidence_satisfies_release"
            ),
            "missing_or_not_ready": release_authority.get("missing_or_not_ready"),
        }
    if payload.get("summary"):
        summary["summary"] = payload.get("summary")
    if payload.get("external_required_artifacts"):
        external = payload.get("external_required_artifacts") or {}
        summary["external_required_artifacts"] = {
            "all_present": external.get("all_present"),
            "missing_count": external.get("missing_count"),
            "release_ready_false_count": external.get("release_ready_false_count"),
            "required_count": external.get("required_count"),
        }
    return {key: value for key, value in summary.items() if value is not None}


def public_gate(gate):
    public = {key: value for key, value in gate.items() if key != "json"}
    payload_summary = summarize_payload(gate.get("json"))
    if payload_summary is not None:
        public["json_summary"] = payload_summary
    return public


def docs_check_gate():
    timing_path = REPORT_DIR / "docs-check-timing.json"
    surface_path = REPORT_DIR / "docs-check-changed-surface.json"
    if reuse_docs_check:
        emit_progress(
            "gate-start",
            "docs-check",
            mode="reuse-docs-check",
            artifact=rel(timing_path),
            changed_surface_artifact=rel(surface_path),
        )
        if not timing_path.exists():
            gate = {
                "name": "docs-check",
                "ok": False,
                "mode": "reuse-docs-check",
                "reason": "missing target/appfw/docs-check-timing.json from the required docs-check step",
                "artifact": rel(timing_path),
            }
            emit_progress("gate-finish", "docs-check", ok=False, mode="reuse-docs-check")
            return gate
        try:
            timing = read_json(timing_path)
            surface = read_json(surface_path) if surface_path.exists() else None
        except json.JSONDecodeError as exc:
            gate = {
                "name": "docs-check",
                "ok": False,
                "mode": "reuse-docs-check",
                "reason": f"docs-check artifact is not valid JSON: {exc}",
                "artifact": rel(timing_path),
            }
            emit_progress("gate-finish", "docs-check", ok=False, mode="reuse-docs-check")
            return gate
        budget_ok = bool(timing.get("budget_ok", False))
        budget_enforced = bool(timing.get("budget_enforced", False))
        payload_ok = bool(timing.get("ok", False))
        ok = budget_ok and payload_ok and budget_enforced
        gate = {
            "name": "docs-check",
            "ok": ok,
            "mode": "reuse-docs-check",
            "artifact": rel(timing_path),
            "changed_surface_artifact": rel(surface_path),
            "selected_mode": timing.get("selected_mode"),
            "requested_mode": timing.get("requested_mode"),
            "budget_ok": timing.get("budget_ok"),
            "budget_enforced": timing.get("budget_enforced"),
            "reason": (
                "reused docs-check artifact must be ok, within budget, and produced with --enforce-budget"
            ),
            "total_wall_ms": timing.get("total_wall_ms"),
            "slowest_subcheck": timing.get("slowest_subcheck"),
            "changed_surface": surface,
        }
        emit_progress("gate-finish", "docs-check", ok=ok, mode="reuse-docs-check")
        return gate

    return run_gate(
        "docs-check",
        [
            "scripts/appfw",
            "framework",
            "docs-check",
            "--changed-only",
            "--json",
            "--progress",
            "--enforce-budget",
        ],
    )


def product_spa_build_gate():
    frontend_dir = REPO_ROOT / "examples" / "products" / "crm" / "frontend"
    dist_index = (
        REPO_ROOT
        / "examples"
        / "products"
        / "crm"
        / "backend"
        / "product_dist"
        / "index.html"
    )
    package_json = frontend_dir / "package.json"
    package_lock = frontend_dir / "package-lock.json"

    if dist_index.exists():
        emit_progress(
            "gate-finish",
            "product-spa-build",
            ok=True,
            mode="already-built",
            artifact=rel(dist_index),
        )
        return {
            "name": "product-spa-build",
            "ok": True,
            "mode": "already-built",
            "artifact": rel(dist_index),
            "reason": "backend/product_dist already exists before composition enforcement",
        }

    if not package_json.exists() or not package_lock.exists():
        emit_progress(
            "gate-finish",
            "product-spa-build",
            ok=False,
            mode="missing-frontend-workspace",
            artifact=rel(dist_index),
        )
        return {
            "name": "product-spa-build",
            "ok": False,
            "mode": "missing-frontend-workspace",
            "artifact": rel(dist_index),
            "reason": "composition enforcement requires backend/product_dist, but the CRM frontend package files are missing",
        }

    package_gate = run_process_gate(
        "pds-components-package",
        ["scripts/ci/build-pds-components-package.sh"],
        cwd=REPO_ROOT,
    )
    install_gate = None
    lock_sync_gate = None
    consumer_gate = None
    build_gate = None
    # Match bitbucket-release-gate.sh: npm pack headers can change the archive
    # digest, so sync the consumer lock integrity before npm ci, then restore
    # the committed lockfile so handoff/docs-check freshness stays clean.
    lock_backup = None
    try:
        if package_gate.get("ok") and package_lock.exists():
            package_version = json.loads(
                (REPO_ROOT / "appfw_ui" / "pds_health" / "components" / "package.json").read_text(
                    encoding="utf-8"
                )
            )["version"]
            package_archive = (
                REPORT_DIR
                / "packages"
                / f"appfw-pds-health-components-{package_version}.tgz"
            )
            fd, backup_path = tempfile.mkstemp(prefix="crm-frontend-lock-")
            os.close(fd)
            lock_backup = pathlib.Path(backup_path)
            shutil.copy2(package_lock, lock_backup)
            lock_sync_gate = run_process_gate(
                "product-spa-lock-integrity",
                [
                    "node",
                    "scripts/ci/sync-pds-package-lock-integrity.mjs",
                    "--lock",
                    str(package_lock),
                    "--archive",
                    str(package_archive),
                ],
                cwd=REPO_ROOT,
            )
            if lock_sync_gate.get("ok"):
                install_gate = run_process_gate(
                    "product-spa-npm-ci",
                    [
                        "npm",
                        "ci",
                        "--no-audit",
                        "--no-fund",
                        "--cache",
                        str(REPORT_DIR / "npm-cache"),
                    ],
                    cwd=frontend_dir,
                    timeout_secs=NPM_CI_TIMEOUT_SECS,
                )
            else:
                install_gate = {
                    "name": "product-spa-npm-ci",
                    "ok": False,
                    "skipped": True,
                    "reason": "package-lock integrity sync failed before npm ci",
                    "lock_sync": lock_sync_gate,
                }
        elif package_gate.get("ok"):
            install_gate = run_process_gate(
                "product-spa-npm-ci",
                [
                    "npm",
                    "ci",
                    "--no-audit",
                    "--no-fund",
                    "--cache",
                    str(REPORT_DIR / "npm-cache"),
                ],
                cwd=frontend_dir,
                timeout_secs=NPM_CI_TIMEOUT_SECS,
            )

        if package_gate.get("ok"):
            consumer_gate = run_process_gate(
                "pds-package-consumer",
                ["node", "scripts/ci/check-pds-package-consumer.mjs", "--json"],
                cwd=REPO_ROOT,
            )
        if install_gate and install_gate.get("ok"):
            build_gate = run_process_gate(
                "product-spa-build",
                ["npm", "run", "build"],
                cwd=frontend_dir,
                timeout_secs=NPM_BUILD_TIMEOUT_SECS,
            )
    finally:
        if lock_backup is not None:
            if lock_backup.exists():
                shutil.copy2(lock_backup, package_lock)
                lock_backup.unlink(missing_ok=True)

    artifact_present = dist_index.exists()
    ok = (
        bool(package_gate.get("ok"))
        and bool(install_gate and install_gate.get("ok"))
        and bool(consumer_gate and consumer_gate.get("ok"))
        and bool(build_gate and build_gate.get("ok"))
        and artifact_present
    )
    return {
        "name": "product-spa-build",
        "ok": ok,
        "mode": "build-backend-product-dist",
        "artifact": rel(dist_index),
        "artifact_present": artifact_present,
        "package": package_gate,
        "lock_sync": lock_sync_gate,
        "consumer": consumer_gate,
        "install": install_gate,
        "build": build_gate,
        "reason": (
            "CRM manifest enables backend-product-dist packaging, so PR composition "
            "enforcement builds the retained product SPA artifact first"
        ),
    }


def governance_release_gate(gate):
    payload = gate.get("json") or {}
    release_authority = payload.get("release_authority_package") or {}
    enforcement_violations = payload.get("enforcement_violations") or []
    expected_release_authority_items = {
        "release-provenance",
        "artifact-signing",
        "release-identity",
        "pds-baseline",
    }
    observed_blockers = {
        str(violation).split(":", 1)[0] for violation in enforcement_violations
    }
    observed_release_authority_items = {
        str(item)
        for item in release_authority.get("missing_or_not_ready", []) or []
    }

    if gate.get("exit_code") == 0:
        return {
            "name": "governance-check-release-posture",
            "ok": bool(payload.get("ok", False)),
            "mode": "release-ready",
            "reason": "governance-check --enforce passed with release-ready evidence",
        }

    missing_expected_items = sorted(
        expected_release_authority_items - observed_release_authority_items
    )
    local_evidence_satisfies_release = release_authority.get(
        "local_evidence_satisfies_release"
    )
    ok = (
        gate.get("json_ok")
        and payload.get("command") == "governance-check"
        and (payload.get("gate") or {}).get("enforced") is True
        and release_authority.get("required_for_release") is True
        and local_evidence_satisfies_release is False
        and not missing_expected_items
    )
    return {
        "name": "governance-check-release-posture",
        "ok": ok,
        "mode": "fail-closed-release-gated",
        "reason": "ordinary PRs must prove governance fails closed until managed release authority/provenance/signing evidence exists",
        "expected_release_authority_items": sorted(expected_release_authority_items),
        "observed_release_authority_items": sorted(observed_release_authority_items),
        "observed_blockers": sorted(observed_blockers),
        "missing_expected_release_authority_items": missing_expected_items,
        "local_evidence_satisfies_release": local_evidence_satisfies_release,
    }


def wave2_report_gate(gate):
    payload = gate.get("json") or {}
    summary = payload.get("summary") or {}
    ok = bool(payload.get("ok", False)) and payload.get("command") == "wave2-status"
    return {
        "name": "wave2-status-report-only",
        "ok": ok,
        "mode": "report-only-until-ci-produces-all-inputs",
        "release_ready": payload.get("release_ready"),
        "status": summary.get("status"),
        "remaining_external_gate_count": summary.get("remaining_external_gate_count"),
        "external_required_artifact_missing_count": summary.get(
            "external_required_artifact_missing_count"
        ),
    }


started = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
gates = []
gates.append(docs_check_gate())
gates.append(
    run_gate(
        "fabric-contracts-check",
        [
            "node",
            "scripts/check-fabric-contracts.mjs",
            "--json",
        ],
    )
)
gates.append(
    run_process_gate(
        "fabric-contracts-test",
        [
            "node",
            "--test",
            "scripts/fabric-contracts/fabric-contracts.test.mjs",
        ],
    )
)
gates.append(
    run_gate(
        "feature-check",
        [
            "scripts/appfw",
            "framework",
            "feature-check",
            "--json",
        ],
    )
)
gates.append(product_spa_build_gate())
gates.append(
    run_gate(
        "composition-check",
        [
            "scripts/appfw",
            "framework",
            "composition-check",
            "--enforce",
            "--json",
        ],
    )
)
gates.append(
    run_gate(
        "fork-check",
        ["scripts/appfw", "framework", "fork-check", "--json"],
    )
)
governance_gate = run_gate(
    "governance-check",
    [
        "scripts/appfw",
        "framework",
        "governance-check",
        "--enforce",
        "--json",
    ],
    allowed_statuses=(0, 1),
    required_ok=False,
)
gates.append(governance_gate)
gates.append(governance_release_gate(governance_gate))
wave2_gate = run_gate(
    "wave2-status",
    ["scripts/appfw", "framework", "wave2-status", "--json"],
)
gates.append(wave2_gate)
gates.append(wave2_report_gate(wave2_gate))

ok = all(bool(gate.get("ok", False)) for gate in gates)
summary = {
    "command": "wave3-pr-gates",
    "ok": ok,
    "artifact": rel(SUMMARY_PATH),
    "progress_log": rel(PROGRESS_PATH),
    "started_at": started,
    "completed_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    "lane": "PM-1",
    "mode": "pull-request-required-local-gates",
    "release_ready_claimed": False,
    "release_ready_note": "Wave 2 release readiness remains report-only until managed CI produces all external inputs.",
    "gates": [public_gate(gate) for gate in gates],
}
write_summary(summary)

print(json.dumps(summary, indent=2, sort_keys=True))
if not ok:
    sys.exit(1)
PY
