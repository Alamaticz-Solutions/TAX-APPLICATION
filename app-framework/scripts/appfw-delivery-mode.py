#!/usr/bin/env python3
"""Local controller for the tracked App Framework delivery profiles."""

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path


SCHEMA = "appfw_delivery_profiles@1"
STATE_SCHEMA = "appfw_delivery_mode_state@1"
LEDGER_SCHEMA = "appfw_deferred_gate_ledger@1"
EXECUTION_SCHEMA = "appfw_gate_execution_evidence@1"
PROJECTION_SCHEMA = "appfw_gate_execution_projection@1"
EXECUTION_STATUSES = {"passed", "failed", "not-run", "missing", "stale"}
GATE_SPECS = (
    (
        "framework-docs-check",
        "scripts/appfw framework docs-check --json",
        "target/appfw/gate-evidence/framework-docs-check.json",
    ),
    (
        "framework-generate-check",
        "scripts/appfw framework generate --check --json",
        "target/appfw/gate-evidence/framework-generate-check.json",
    ),
    (
        "framework-test-fast",
        "scripts/appfw framework test --fast --json",
        "target/appfw/gate-evidence/framework-test-fast.json",
    ),
)


class ModeError(Exception):
    pass


def load_json(path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ModeError(f"cannot read {path.name}: {exc}") from exc


def load_policy(path):
    policy = load_json(path)
    if not isinstance(policy, dict) or set(policy) != {
        "schema", "default_profile", "profiles", "gate_execution_projection"
    }:
        raise ModeError("delivery profile policy must be an exact object")
    profiles = policy.get("profiles")
    if policy.get("schema") != SCHEMA or not isinstance(profiles, dict):
        raise ModeError("delivery profile policy has an invalid schema")
    if policy.get("default_profile") != "accelerated" or set(profiles) != {"accelerated", "candidate"}:
        raise ModeError("delivery profile policy must define accelerated and candidate")
    accelerated = profiles["accelerated"]
    candidate = profiles["candidate"]
    expected_keys = {
        "accelerated": {"accelerated_reason", "deferred_gate_ids", "enforce_candidate", "routing"},
        "candidate": {"candidate_version", "deferred_gate_ids", "enforce_candidate", "routing"},
    }
    for name, profile in profiles.items():
        if not isinstance(profile, dict) or set(profile) != expected_keys[name]:
            raise ModeError(f"delivery profile {name} must be an exact object")
        gate_ids = profile["deferred_gate_ids"]
        if (
            not isinstance(gate_ids, list)
            or not gate_ids
            or not all(isinstance(item, str) and item.strip() for item in gate_ids)
            or len(set(gate_ids)) != len(gate_ids)
        ):
            raise ModeError(f"delivery profile {name} has invalid deferred gate ids")
        if type(profile["enforce_candidate"]) is not bool:
            raise ModeError(f"delivery profile {name} has an invalid enforce_candidate value")
    if accelerated["routing"] != "focused" or accelerated["enforce_candidate"] is not False:
        raise ModeError("accelerated delivery profile must use focused routing without candidate enforcement")
    if candidate["routing"] != "enforce-candidate" or candidate["enforce_candidate"] is not True:
        raise ModeError("candidate delivery profile must use enforce-candidate routing")
    if accelerated["deferred_gate_ids"] != candidate["deferred_gate_ids"]:
        raise ModeError("delivery profiles must define matching deferred gate ids")
    if accelerated["deferred_gate_ids"] != [item[0] for item in GATE_SPECS]:
        raise ModeError("delivery profiles must define the canonical framework gate ids")
    projection = policy["gate_execution_projection"]
    expected_projection = {
        "schema": PROJECTION_SCHEMA,
        "gates": [
            {"gate_id": gate_id, "command": command, "evidence_path": evidence_path}
            for gate_id, command, evidence_path in GATE_SPECS
        ],
    }
    if projection != expected_projection:
        raise ModeError("delivery profile gate execution projection is invalid")
    if not isinstance(accelerated["accelerated_reason"], str) or not accelerated["accelerated_reason"].strip():
        raise ModeError("accelerated delivery profile requires an accelerated reason")
    if not isinstance(candidate["candidate_version"], str) or not candidate["candidate_version"].strip():
        raise ModeError("candidate delivery profile requires a candidate version")
    return policy


def git_dir(repo_root):
    result = subprocess.run(
        ["git", "-C", str(repo_root), "rev-parse", "--git-dir"],
        text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False,
    )
    if result.returncode:
        raise ModeError("Git worktree directory is unavailable")
    path = Path(result.stdout.strip())
    return (repo_root / path).resolve() if not path.is_absolute() else path.resolve()


def package_root():
    configured = os.environ.get("APPFW_FRAMEWORK_ROOT")
    root = Path(configured).resolve() if configured else Path(__file__).resolve().parent.parent
    identity_path = root / "app-framework-package.json"
    identity = load_json(identity_path)
    expected_keys = {
        "version",
        "platform",
        "dependency_source",
        "cargo_registry",
        "cargo_registry_index",
    }
    if (
        not isinstance(identity, dict)
        or set(identity) != expected_keys
        or not all(isinstance(value, str) and value.strip() for value in identity.values())
    ):
        raise ModeError("package identity must be an exact non-empty object")
    return root, identity


def paths(repo_root):
    directory = git_dir(repo_root) / "appfw" / "delivery-mode"
    return directory / "state.json", directory / "deferred-gates.json"


def source_snapshot(repo_root):
    status = subprocess.run(["git", "-C", str(repo_root), "status", "--porcelain=v1"], text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    if status.returncode:
        raise ModeError("delivery mode requires Git source status")
    source = subprocess.run(["git", "-C", str(repo_root), "rev-parse", "HEAD"], text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    source_sha = source.stdout.strip()
    if source.returncode or not re.fullmatch(r"[0-9a-f]{40}", source_sha):
        raise ModeError("delivery mode requires an exact source SHA")
    return source_sha, bool(status.stdout.strip())


def clean_source_sha(repo_root):
    git_dir(repo_root)
    source_sha, dirty = source_snapshot(repo_root)
    if dirty:
        raise ModeError("delivery mode requires a clean checkout")
    return source_sha


def expected_ledger(profile, policy):
    status = "deferred" if profile == "accelerated" else "required"
    return {
        "schema": LEDGER_SCHEMA,
        "profile": profile,
        "gates": [
            {"id": gate_id, "status": status}
            for gate_id in policy["profiles"][profile]["deferred_gate_ids"]
        ],
    }


def expected_state(profile, policy, source_sha):
    state = {"schema": STATE_SCHEMA, "profile": profile, "policy_schema": SCHEMA, "source_sha": source_sha}
    if profile == "accelerated":
        state["accelerated_reason"] = policy["profiles"][profile]["accelerated_reason"]
    else:
        state["candidate_version"] = policy["profiles"][profile]["candidate_version"]
    return state


def current(repo_root, policy, allow_source_drift=False):
    try:
        state_path, ledger_path = paths(repo_root)
    except ModeError as exc:
        if str(exc) != "Git worktree directory is unavailable":
            raise
        _, identity = package_root()
        profile = policy["default_profile"]
        return {
            "state": {
                "profile": profile,
                "accelerated_reason": policy["profiles"][profile]["accelerated_reason"],
            },
            "ledger": expected_ledger(profile, policy),
            "state_origin": "package-no-git-default",
            "package_identity": identity,
        }
    current_sha, dirty = source_snapshot(repo_root)
    if not state_path.exists() and not ledger_path.exists():
        profile = policy["default_profile"]
        return {
            "state": expected_state(profile, policy, current_sha),
            "ledger": expected_ledger(profile, policy),
            "state_origin": "tracked-default",
            "current_sha": current_sha,
            "dirty": dirty,
            "stale": False,
        }
    if not state_path.exists() or not ledger_path.exists():
        raise ModeError("delivery mode state and deferred-gate ledger must exist together")
    state, ledger = load_json(state_path), load_json(ledger_path)
    if not isinstance(state, dict):
        raise ModeError("delivery mode state must be an object")
    if not isinstance(ledger, dict):
        raise ModeError("deferred-gate ledger must be an object")
    profile = state.get("profile")
    source_sha = state.get("source_sha")
    if (
        profile not in policy["profiles"]
        or not isinstance(source_sha, str)
        or not re.fullmatch(r"[0-9a-f]{40}", source_sha)
        or state != expected_state(profile, policy, source_sha)
    ):
        raise ModeError("delivery mode state violates the canonical policy")
    if ledger != expected_ledger(profile, policy):
        raise ModeError("deferred-gate ledger violates the canonical policy")
    stale = source_sha != current_sha
    if not allow_source_drift:
        if dirty:
            raise ModeError("delivery mode requires a clean checkout")
        if stale:
            raise ModeError("delivery mode state violates the canonical policy")
    return {
        "state": state,
        "ledger": ledger,
        "state_origin": "worktree-git-dir",
        "current_sha": current_sha,
        "dirty": dirty,
        "stale": stale,
    }


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            json.dump(value, handle, indent=2, sort_keys=True)
            handle.write("\n")
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def evidence_digest(value):
    core = {
        key: value[key]
        for key in ("schema", "gate_id", "execution_status", "source_sha", "command", "reason")
    }
    serialized = json.dumps(core, sort_keys=True, separators=(",", ":")) + "\n"
    return hashlib.sha256(serialized.encode("utf-8")).hexdigest()


def record_gate(repo_root, gate_id, command, exit_status, expected_source_sha):
    spec = next((item for item in GATE_SPECS if item[0] == gate_id), None)
    if spec is None or command != spec[1]:
        raise ModeError("gate execution record does not match the canonical command")
    if not isinstance(expected_source_sha, str) or not re.fullmatch(
        r"[0-9a-f]{40}", expected_source_sha
    ):
        raise ModeError("gate execution record requires an expected source SHA")
    current_sha, dirty = source_snapshot(repo_root)
    source_changed = dirty or current_sha != expected_source_sha
    status = "passed" if exit_status == 0 and not source_changed else "failed"
    if source_changed:
        reason = "source SHA or cleanliness changed during gate execution"
    else:
        reason = "command exited 0" if status == "passed" else f"command exited {exit_status}"
    record = {
        "schema": EXECUTION_SCHEMA,
        "gate_id": gate_id,
        "execution_status": status,
        "source_sha": expected_source_sha,
        "command": command,
        "reason": reason,
    }
    record["record_sha256"] = evidence_digest(record)
    write_json(repo_root / spec[2], record)


def gate_projection(repo_root, ledger, current_sha):
    dispositions = {gate["id"]: gate["status"] for gate in ledger["gates"]}
    gates = []
    for gate_id, command, relative_path in GATE_SPECS:
        path = repo_root / relative_path
        base = {
            "gate_id": gate_id,
            "policy_disposition": dispositions[gate_id],
            "source_sha": current_sha,
            "command": command,
            "evidence_path": relative_path,
        }
        if not path.is_file():
            gate = {
                **base,
                "execution_status": "missing",
                "evidence_sha256": None,
                "reason": "canonical evidence file is missing",
            }
        else:
            raw = path.read_bytes()
            file_digest = hashlib.sha256(raw).hexdigest()
            try:
                evidence = json.loads(raw)
            except (UnicodeDecodeError, json.JSONDecodeError):
                evidence = None
            expected_keys = {
                "schema", "gate_id", "execution_status", "source_sha",
                "command", "reason", "record_sha256",
            }
            if not isinstance(evidence, dict) or set(evidence) != expected_keys:
                status, reason = "failed", "canonical evidence is malformed"
            elif not all(
                isinstance(evidence[key], str)
                for key in expected_keys
            ):
                status, reason = "failed", "canonical evidence field types are invalid"
            elif evidence["schema"] != EXECUTION_SCHEMA:
                status, reason = "failed", "canonical evidence schema is invalid"
            elif evidence["gate_id"] != gate_id or evidence["command"] != command:
                status, reason = "failed", "canonical gate identity or command does not match"
            elif evidence["record_sha256"] != evidence_digest(evidence):
                status, reason = "failed", "canonical evidence digest does not match"
            elif evidence["source_sha"] != current_sha:
                status, reason = "stale", "canonical evidence source SHA does not match"
            elif evidence["execution_status"] not in EXECUTION_STATUSES:
                status, reason = "failed", "canonical execution status is invalid"
            else:
                status, reason = evidence["execution_status"], evidence["reason"]
            gate = {
                **base,
                "execution_status": status,
                "evidence_sha256": file_digest,
                "reason": reason,
            }
        gates.append(gate)
    ok = all(
        gate["policy_disposition"] != "required" or gate["execution_status"] == "passed"
        for gate in gates
    )
    return {"schema": PROJECTION_SCHEMA, "ok": ok, "gates": gates}


def annotation(repo_root, policy, allow_source_drift=False):
    current_mode = current(repo_root, policy, allow_source_drift=allow_source_drift)
    state = current_mode["state"]
    ledger = current_mode["ledger"]
    profile = state["profile"]
    definition = policy["profiles"][profile]
    if current_mode["state_origin"] == "package-no-git-default":
        return {
            "profile": profile,
            "routing": definition["routing"],
            "enforce_candidate": definition["enforce_candidate"],
            "deferred_gates": ledger["gates"],
            "source_status": "package-no-git",
            "state": current_mode["state_origin"],
            "package_identity": current_mode["package_identity"],
            "accelerated_reason": state["accelerated_reason"],
        }
    dirty = current_mode["dirty"]
    stale = current_mode["stale"]
    if dirty and stale:
        source_status = "dirty-and-stale"
    elif dirty:
        source_status = "dirty"
    elif stale:
        source_status = "stale"
    else:
        source_status = "current"
    result = {
        "profile": profile,
        "routing": definition["routing"],
        "enforce_candidate": definition["enforce_candidate"],
        "deferred_gates": ledger["gates"],
        "source_sha": state["source_sha"],
        "current_sha": current_mode["current_sha"],
        "source_status": source_status,
        "dirty": dirty,
        "stale": stale,
        "state": current_mode["state_origin"],
        "gate_execution_projection": gate_projection(
            repo_root, ledger, current_mode["current_sha"]
        ),
    }
    if profile == "accelerated":
        result["accelerated_reason"] = state["accelerated_reason"]
    else:
        result["candidate_version"] = state["candidate_version"]
    return result


def emit(command, ok, json_mode, **fields):
    payload = {"command": command, "ok": ok, **fields}
    if json_mode:
        print(json.dumps(payload, indent=2, sort_keys=True))
    elif ok:
        delivery = payload.get("delivery_profile", {})
        print(f"Delivery profile: {delivery.get('profile', '')}")
        print(f"Routing: {delivery.get('routing', '')}")
    else:
        print(payload["error"], file=sys.stderr)


def main():
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--policy-path")
    parser.add_argument("--json", action="store_true")
    parser.add_argument(
        "action", choices=("status", "set", "annotation", "record-gate", "source-sha")
    )
    parser.add_argument("profile", nargs="?")
    parser.add_argument("--gate-id")
    parser.add_argument("--command")
    parser.add_argument("--exit-status", type=int)
    parser.add_argument("--expected-source-sha")
    args = parser.parse_args()
    command = "mode"
    try:
        repo_root = Path(args.repo_root).resolve()
        if args.policy_path:
            policy_path = Path(args.policy_path).resolve()
        else:
            try:
                git_dir(repo_root)
            except ModeError as exc:
                if str(exc) != "Git worktree directory is unavailable":
                    raise
                package_root_path, _ = package_root()
                policy_path = package_root_path / "docs/start/delivery-profiles.json"
            else:
                policy_path = repo_root / "docs/start/delivery-profiles.json"
        policy = load_policy(policy_path)
        if args.action == "source-sha":
            if args.profile is not None:
                raise ModeError("source-sha does not accept a profile")
            print(clean_source_sha(repo_root))
            return 0
        if args.action == "record-gate":
            if (
                not args.gate_id
                or not args.command
                or args.exit_status is None
                or not args.expected_source_sha
                or args.profile is not None
            ):
                raise ModeError("record-gate requires gate id, command, and exit status")
            record_gate(
                repo_root, args.gate_id, args.command, args.exit_status,
                args.expected_source_sha,
            )
            return 0
        if args.action == "set":
            if args.profile not in policy["profiles"]:
                raise ModeError("mode set requires accelerated or candidate")
            state = expected_state(args.profile, policy, clean_source_sha(repo_root))
            ledger = expected_ledger(args.profile, policy)
            state_path, ledger_path = paths(repo_root)
            write_json(state_path, state)
            write_json(ledger_path, ledger)
        elif args.profile is not None:
            raise ModeError(f"mode {args.action} does not accept a profile")
        result = annotation(
            repo_root,
            policy,
            allow_source_drift=args.action != "set",
        )
        if args.action == "annotation":
            print(json.dumps(result, sort_keys=True))
        else:
            emit(command, True, args.json, delivery_profile=result)
    except ModeError as exc:
        emit(command, False, args.json, error=str(exc))
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
