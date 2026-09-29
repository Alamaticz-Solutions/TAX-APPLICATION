#!/usr/bin/env python3
"""Focused lifecycle and invariant proof for appfw-delivery-mode.py."""

import json
import hashlib
import importlib.util
import os
import shutil
import subprocess
import sys
import tempfile
from unittest import mock
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
WRAPPER = ROOT / "scripts/appfw"
CONTROLLER = ROOT / "scripts/appfw-delivery-mode.py"
POLICY = ROOT / "docs/start/delivery-profiles.json"
LEDGER_SCHEMA = ROOT / "docs/start/gate-evidence.schema.json"
DOCS_CHECK = ROOT / "scripts/check-doc-examples.sh"
PRODUCT_INCREMENT_PATH_SAFETY = ROOT / "scripts/product-increment-path-safety.mjs"
GATE_SPECS = (
    ("framework-docs-check", "scripts/appfw framework docs-check --json", "framework-docs-check.json"),
    ("framework-generate-check", "scripts/appfw framework generate --check --json", "framework-generate-check.json"),
    ("framework-test-fast", "scripts/appfw framework test --fast --json", "framework-test-fast.json"),
)


def controller_result(repo, *args, policy=POLICY, env=None):
    command_env = os.environ.copy()
    if env:
        command_env.update(env)
    return subprocess.run(
        [sys.executable, str(CONTROLLER), "--repo-root", str(repo), "--policy-path", str(policy), "--json", *args],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=command_env,
    )


def run(repo, *args, expect=0, policy=POLICY):
    result = controller_result(repo, *args, policy=policy)
    if result.returncode != expect:
        raise AssertionError(f"{' '.join(args)} returned {result.returncode}: {result.stderr} {result.stdout}")
    return json.loads(result.stdout)


def annotation(repo, expect=0):
    result = subprocess.run(
        [
            sys.executable,
            str(CONTROLLER),
            "--repo-root",
            str(repo),
            "--policy-path",
            str(POLICY),
            "annotation",
        ],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.returncode != expect:
        raise AssertionError(
            f"annotation returned {result.returncode}: {result.stderr} {result.stdout}"
        )
    return json.loads(result.stdout)


def assert_invalid_policy(repo, directory, name, policy):
    policy_path = Path(directory) / f"{name}.json"
    policy_path.write_text(json.dumps(policy), encoding="utf-8")
    result = controller_result(repo, "status", policy=policy_path)
    assert result.returncode == 2, f"{name} returned {result.returncode}: {result.stderr} {result.stdout}"
    assert result.stderr == "", f"{name} emitted a traceback: {result.stderr}"
    payload = json.loads(result.stdout)
    assert payload["command"] == "mode" and payload["ok"] is False
    assert isinstance(payload.get("error"), str) and payload["error"]


def policy_copy():
    return json.loads(POLICY.read_text(encoding="utf-8"))


def git(repo, *args):
    subprocess.run(["git", "-C", str(repo), *args], check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)


def assert_ledger_matches_schema(ledger, profile, status):
    schema = json.loads(LEDGER_SCHEMA.read_text(encoding="utf-8"))
    assert schema["additionalProperties"] is False
    assert set(schema["properties"]) == {"schema", "profile", "gates"}
    gate_schema = schema["properties"]["gates"]["items"]
    assert gate_schema["type"] == "object" and gate_schema["additionalProperties"] is False
    assert set(gate_schema["properties"]) == {"id", "status"}
    assert gate_schema["properties"]["status"]["enum"] == ["deferred", "required"]
    assert set(ledger) == set(schema["required"])
    assert ledger["schema"] == schema["properties"]["schema"]["const"]
    assert ledger["profile"] == profile
    for gate in ledger["gates"]:
        assert set(gate) == set(gate_schema["required"])
        assert isinstance(gate["id"], str) and gate["id"]
        assert gate["status"] == status


def git_dir(repo):
    directory = Path(subprocess.check_output(["git", "-C", str(repo), "rev-parse", "--git-dir"], text=True).strip())
    return directory if directory.is_absolute() else (repo / directory).resolve()


def delivery_evidence(repo):
    directory = git_dir(repo) / "appfw/delivery-mode"
    evidence = {}
    for name in ("state.json", "deferred-gates.json"):
        path = directory / name
        evidence[name] = path.read_bytes() if path.exists() else None
    return evidence


def gate_evidence(repo):
    directory = repo / "target/appfw/gate-evidence"
    return {
        filename: (directory / filename).read_bytes()
        if (directory / filename).exists()
        else None
        for _, _, filename in GATE_SPECS
    }


def root_mode(repo, profile):
    result = subprocess.run(
        [str(repo / "scripts/appfw"), "mode", "set", profile, "--json"],
        cwd=repo,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return json.loads(result.stdout)


def root_ledger(repo):
    return json.loads((git_dir(repo) / "appfw/delivery-mode/deferred-gates.json").read_text(encoding="utf-8"))


def root_wrapper_fixture(temporary):
    repo = Path(temporary) / "root-wrapper"
    (repo / "scripts").mkdir(parents=True)
    (repo / "docs/start").mkdir(parents=True)
    shutil.copy2(WRAPPER, repo / "scripts/appfw")
    shutil.copy2(CONTROLLER, repo / "scripts/appfw-delivery-mode.py")
    shutil.copy2(POLICY, repo / "docs/start/delivery-profiles.json")
    (repo / ".gitignore").write_text("target/\n", encoding="utf-8")
    (repo / "Cargo.lock").write_text("# fixture\n", encoding="utf-8")
    introspect = repo / "scripts/fake-introspect"
    introspect.write_text(
        """#!/usr/bin/env python3
import json
import sys
from pathlib import Path
args = sys.argv[1:]
report_root = Path(args[args.index("--report-root") + 1])
path = report_root / "agent-handoff.json"
path.parent.mkdir(parents=True, exist_ok=True)
payload = {"artifact_path": str(path), "command": "handoff", "ok": True}
path.write_text(json.dumps(payload) + "\\n", encoding="utf-8")
print(json.dumps(payload))
""",
        encoding="utf-8",
    )
    introspect.chmod(0o755)
    downstream = repo / "scripts/check-doc-examples.sh"
    downstream.write_text(
        "#!/usr/bin/env bash\n"
        "printf '%s\\n' downstream-ran >\"${APPFW_DOWNSTREAM_MARKER:?}\"\n"
        "exit 0\n",
        encoding="utf-8",
    )
    downstream.chmod(0o755)
    git(repo, "init")
    git(repo, "config", "user.email", "tests@appfw.local")
    git(repo, "config", "user.name", "AppFW Tests")
    git(
        repo, "add", ".gitignore", "Cargo.lock", "scripts/appfw", "scripts/appfw-delivery-mode.py",
        "scripts/fake-introspect", "scripts/check-doc-examples.sh",
        "docs/start/delivery-profiles.json"
    )
    git(repo, "commit", "-m", "root wrapper fixture")
    return repo


def root_handoff(repo):
    result = subprocess.run(
        [str(repo / "scripts/appfw"), "framework", "handoff", "--json"],
        cwd=repo,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env={**os.environ, "APPFW_INTROSPECT_BIN": str(repo / "scripts/fake-introspect")},
    )
    assert result.returncode == 0, result.stderr
    return json.loads(result.stdout)


def changed_only_selection_fixture(temporary, changed_path, additional_changed_paths=()):
    repo = Path(tempfile.mkdtemp(prefix="changed-only-", dir=temporary))
    (repo / "scripts").mkdir(parents=True)
    target = repo / changed_path
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(DOCS_CHECK, repo / "scripts/check-doc-examples.sh")
    shutil.copy2(
        PRODUCT_INCREMENT_PATH_SAFETY,
        repo / "scripts/product-increment-path-safety.mjs",
    )
    target.write_text("baseline\n", encoding="utf-8")
    git(repo, "init")
    git(repo, "config", "user.email", "tests@appfw.local")
    git(repo, "config", "user.name", "AppFW Tests")
    git(
        repo,
        "add",
        "scripts/check-doc-examples.sh",
        "scripts/product-increment-path-safety.mjs",
        changed_path,
    )
    git(repo, "commit", "-m", "changed-only fixture")
    target.write_text("changed\n", encoding="utf-8")
    for additional_path in additional_changed_paths:
        additional_target = repo / additional_path
        additional_target.parent.mkdir(parents=True, exist_ok=True)
        additional_target.write_text("changed\\n", encoding="utf-8")
    result = subprocess.run(
        [
            "bash",
            str(repo / "scripts/check-doc-examples.sh"),
            "--changed-only",
            "--base",
            "HEAD",
            "--plan",
            "--json",
        ],
        cwd=repo,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    assert result.returncode == 0, result.stderr
    return json.loads(result.stdout)


def gate_record(gate_id, command, source_sha, status, reason=None):
    record = {
        "schema": "appfw_gate_execution_evidence@1",
        "gate_id": gate_id,
        "execution_status": status,
        "source_sha": source_sha,
        "command": command,
        "reason": reason or f"fixture {status}",
    }
    core = json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n"
    record["record_sha256"] = hashlib.sha256(core.encode("utf-8")).hexdigest()
    return record


def refresh_record_digest(record):
    core = {
        key: record[key]
        for key in ("schema", "gate_id", "execution_status", "source_sha", "command", "reason")
    }
    serialized = json.dumps(core, sort_keys=True, separators=(",", ":")) + "\n"
    record["record_sha256"] = hashlib.sha256(serialized.encode("utf-8")).hexdigest()
    return record


def write_gate_records(repo, statuses, source_sha=None):
    source_sha = source_sha or subprocess.check_output(
        ["git", "-C", str(repo), "rev-parse", "HEAD"], text=True
    ).strip()
    directory = repo / "target/appfw/gate-evidence"
    directory.mkdir(parents=True, exist_ok=True)
    for (gate_id, command, filename), status in zip(GATE_SPECS, statuses):
        (directory / filename).write_text(
            json.dumps(gate_record(gate_id, command, source_sha, status)) + "\n",
            encoding="utf-8",
        )


def projection(repo):
    return annotation(repo)["gate_execution_projection"]


def main():
    original_evidence = delivery_evidence(ROOT)
    with tempfile.TemporaryDirectory() as temporary:
        repo = Path(temporary) / "repo"
        repo.mkdir()
        git(repo, "init")
        git(repo, "config", "user.email", "tests@appfw.local")
        git(repo, "config", "user.name", "AppFW Tests")
        (repo / "README").write_text("fixture\n", encoding="utf-8")
        (repo / ".gitignore").write_text("target/\n", encoding="utf-8")
        git(repo, "add", "README", ".gitignore")
        git(repo, "commit", "-m", "fixture")

        for authority_path in (
            "scripts/check-product-increment-plan.mjs",
            "scripts/check-product-increment-portfolio.mjs",
            "scripts/check-nexus-workstreams.mjs",
            "scripts/product-increment-path-safety.mjs",
            "scripts/appfw-delivery-mode.py",
            "docs/start/delivery-profiles.json",
        ):
            selection = changed_only_selection_fixture(temporary, authority_path)
            assert selection["selected_mode"] == "full", authority_path
            assert selection["changed_surface_requires_full"] is True, authority_path
        ordinary_plan_selection = changed_only_selection_fixture(
            temporary, "docs/specs/afs-transparent-routing.product-increment.json"
        )
        assert ordinary_plan_selection["selected_mode"] == "fast"
        assert ordinary_plan_selection["changed_surface_requires_full"] is False
        assert ordinary_plan_selection["focused_subcheck"] == "product-increment-delivery"
        mixed_plan_selection = changed_only_selection_fixture(
            temporary,
            "docs/specs/afs-transparent-routing.product-increment.json",
            ("docs/start/ordinary-guide.md",),
        )
        assert mixed_plan_selection["selected_mode"] == "full"
        assert mixed_plan_selection["changed_surface_requires_full"] is True

        initial_evidence = delivery_evidence(repo)
        initial = run(repo, "status")
        assert initial["delivery_profile"]["profile"] == "accelerated"
        assert initial["delivery_profile"]["state"] == "tracked-default"
        assert initial["delivery_profile"]["source_status"] == "current"
        assert delivery_evidence(repo) == initial_evidence

        second = Path(temporary) / "second"
        git(repo, "worktree", "add", "--detach", str(second), "HEAD")
        second_evidence = delivery_evidence(second)
        second_initial = run(second, "status")
        assert second_initial["delivery_profile"]["profile"] == "accelerated"
        assert second_initial["delivery_profile"]["state"] == "tracked-default"
        assert delivery_evidence(second) == second_evidence

        accelerated = run(repo, "set", "accelerated")
        assert accelerated["delivery_profile"]["routing"] == "focused"
        assert accelerated["delivery_profile"]["accelerated_reason"] == "focused-delivery-profile"
        assert accelerated["delivery_profile"]["source_sha"] == subprocess.check_output(["git", "-C", str(repo), "rev-parse", "HEAD"], text=True).strip()
        candidate = run(repo, "set", "candidate")
        assert candidate["delivery_profile"]["enforce_candidate"] is True
        assert candidate["delivery_profile"]["routing"] == "enforce-candidate"
        assert candidate["delivery_profile"]["candidate_version"] == "1"
        assert run(second, "status")["delivery_profile"]["profile"] == "accelerated"
        assert delivery_evidence(second) == second_evidence
        second_mode = run(second, "set", "candidate")
        assert second_mode["delivery_profile"]["state"] == "worktree-git-dir"
        assert run(repo, "status")["delivery_profile"]["profile"] == "candidate"

        assert_invalid_policy(repo, temporary, "non-object-policy", [])

        missing = policy_copy()
        del missing["profiles"]["accelerated"]["enforce_candidate"]
        assert_invalid_policy(repo, temporary, "missing-field", missing)

        mistyped = policy_copy()
        mistyped["profiles"]["accelerated"]["enforce_candidate"] = "no"
        assert_invalid_policy(repo, temporary, "mistyped-field", mistyped)

        mistyped_default = policy_copy()
        mistyped_default["default_profile"] = []
        assert_invalid_policy(repo, temporary, "mistyped-default", mistyped_default)

        nonaccelerated_default = policy_copy()
        nonaccelerated_default["default_profile"] = "candidate"
        assert_invalid_policy(
            repo,
            temporary,
            "nonaccelerated-default",
            nonaccelerated_default,
        )

        duplicate = policy_copy()
        duplicate["profiles"]["accelerated"]["deferred_gate_ids"].append("framework-docs-check")
        assert_invalid_policy(repo, temporary, "duplicate-gate", duplicate)

        empty_gate = policy_copy()
        empty_gate["profiles"]["candidate"]["deferred_gate_ids"][0] = ""
        assert_invalid_policy(repo, temporary, "empty-gate", empty_gate)

        altered = policy_copy()
        altered["profiles"]["candidate"]["deferred_gate_ids"][0] = "altered-gate"
        assert_invalid_policy(repo, temporary, "altered-gate-set", altered)

        altered_both = policy_copy()
        altered_both["profiles"]["accelerated"]["deferred_gate_ids"][0] = "altered-gate"
        altered_both["profiles"]["candidate"]["deferred_gate_ids"][0] = "altered-gate"
        assert_invalid_policy(repo, temporary, "altered-matching-gate-set", altered_both)

        altered_routing = policy_copy()
        altered_routing["profiles"]["candidate"]["routing"] = "focused"
        assert_invalid_policy(repo, temporary, "altered-routing", altered_routing)

        extra = policy_copy()
        extra["profiles"]["accelerated"]["unexpected"] = True
        assert_invalid_policy(repo, temporary, "extra-field", extra)

        (repo / "README").write_text("dirty\n", encoding="utf-8")
        dirty = run(repo, "status")
        assert dirty["delivery_profile"]["source_status"] == "dirty"
        dirty_annotation = annotation(repo)
        assert dirty_annotation["dirty"] is True
        assert dirty_annotation["stale"] is False
        assert dirty_annotation["source_status"] == "dirty"
        dirty_set = run(repo, "set", "candidate", expect=2)
        assert dirty_set["ok"] is False and "clean checkout" in dirty_set["error"]
        git(repo, "checkout", "--", "README")
        git(repo, "commit", "--allow-empty", "-m", "advance source")
        stale = run(repo, "status")
        assert stale["delivery_profile"]["source_status"] == "stale"
        stale_annotation = annotation(repo)
        assert stale_annotation["dirty"] is False
        assert stale_annotation["stale"] is True
        assert stale_annotation["source_status"] == "stale"
        (repo / "README").write_text("dirty and stale\n", encoding="utf-8")
        dirty_stale_annotation = annotation(repo)
        assert dirty_stale_annotation["dirty"] is True
        assert dirty_stale_annotation["stale"] is True
        assert dirty_stale_annotation["source_status"] == "dirty-and-stale"
        git(repo, "checkout", "--", "README")
        run(repo, "set", "candidate")

        state = git_dir(repo) / "appfw/delivery-mode/state.json"
        ledger = git_dir(repo) / "appfw/delivery-mode/deferred-gates.json"
        ledger.unlink()
        half_present = run(repo, "status", expect=2)
        assert half_present["ok"] is False and "exist together" in half_present["error"]
        run(repo, "set", "candidate")
        state.unlink()
        half_present = run(repo, "status", expect=2)
        assert half_present["ok"] is False and "exist together" in half_present["error"]
        run(repo, "set", "candidate")

        ledger.write_text('{"broken":true}\n', encoding="utf-8")
        invalid = run(repo, "status", expect=2)
        assert invalid["ok"] is False and "ledger" in invalid["error"]
        run(repo, "set", "candidate")

        state.write_text("[]\n", encoding="utf-8")
        invalid_state = controller_result(repo, "status")
        assert invalid_state.returncode == 2 and invalid_state.stderr == ""
        assert json.loads(invalid_state.stdout)["ok"] is False
        run(repo, "set", "candidate")
        ledger.write_text("[]\n", encoding="utf-8")
        invalid_ledger = controller_result(repo, "status")
        assert invalid_ledger.returncode == 2 and invalid_ledger.stderr == ""
        assert json.loads(invalid_ledger.stdout)["ok"] is False
        run(repo, "set", "candidate")

        canonical_state = json.loads(state.read_text(encoding="utf-8"))
        canonical_state["source_sha"] = "not-a-sha"
        state.write_text(json.dumps(canonical_state), encoding="utf-8")
        invalid_source = run(repo, "status", expect=2)
        assert invalid_source["ok"] is False and "canonical policy" in invalid_source["error"]
        run(repo, "set", "candidate")

        noncanonical_state = json.loads(state.read_text(encoding="utf-8"))
        noncanonical_state["unexpected"] = True
        state.write_text(json.dumps(noncanonical_state), encoding="utf-8")
        noncanonical = run(repo, "status", expect=2)
        assert noncanonical["ok"] is False and "canonical policy" in noncanonical["error"]
        run(repo, "set", "candidate")

        projection_evidence = delivery_evidence(repo)
        projection_payload = annotation(repo)
        assert projection_payload["profile"] == "candidate"
        assert len(projection_payload["deferred_gates"]) == 3
        assert projection_payload["source_status"] == "current"
        assert delivery_evidence(repo) == projection_evidence

        missing_projection = projection_payload["gate_execution_projection"]
        assert missing_projection["schema"] == "appfw_gate_execution_projection@1"
        assert missing_projection["ok"] is False
        assert [gate["execution_status"] for gate in missing_projection["gates"]] == [
            "missing", "missing", "missing"
        ]

        for execution_status in ("passed", "failed", "not-run"):
            write_gate_records(repo, [execution_status] * 3)
            status_projection = projection(repo)
            assert [gate["execution_status"] for gate in status_projection["gates"]] == [
                execution_status
            ] * 3
            assert status_projection["ok"] is (execution_status == "passed")
            assert all(gate["evidence_sha256"] for gate in status_projection["gates"])

        old_sha = "0" * 40
        write_gate_records(repo, ["passed"] * 3, source_sha=old_sha)
        assert [gate["execution_status"] for gate in projection(repo)["gates"]] == [
            "stale", "stale", "stale"
        ]

        write_gate_records(repo, ["passed", "failed", "not-run"])
        mixed = projection(repo)
        assert [gate["execution_status"] for gate in mixed["gates"]] == [
            "passed", "failed", "not-run"
        ]
        assert mixed["ok"] is False
        run(repo, "set", "accelerated")
        deferred = projection(repo)
        assert deferred["ok"] is True
        assert [gate["policy_disposition"] for gate in deferred["gates"]] == [
            "deferred", "deferred", "deferred"
        ]
        assert [gate["execution_status"] for gate in deferred["gates"]] == [
            "passed", "failed", "not-run"
        ]
        run(repo, "set", "candidate")

        evidence_dir = repo / "target/appfw/gate-evidence"
        docs_evidence = evidence_dir / "framework-docs-check.json"
        wrong_command = gate_record(
            GATE_SPECS[0][0], "wrong command", subprocess.check_output(
                ["git", "-C", str(repo), "rev-parse", "HEAD"], text=True
            ).strip(), "passed"
        )
        docs_evidence.write_text(json.dumps(wrong_command) + "\n", encoding="utf-8")
        assert projection(repo)["gates"][0]["execution_status"] == "failed"

        wrong_digest = gate_record(
            GATE_SPECS[0][0], GATE_SPECS[0][1], subprocess.check_output(
                ["git", "-C", str(repo), "rev-parse", "HEAD"], text=True
            ).strip(), "passed"
        )
        wrong_digest["record_sha256"] = "f" * 64
        docs_evidence.write_text(json.dumps(wrong_digest) + "\n", encoding="utf-8")
        assert projection(repo)["gates"][0]["execution_status"] == "failed"
        docs_evidence.write_text("{malformed\n", encoding="utf-8")
        assert projection(repo)["gates"][0]["execution_status"] == "failed"
        for field, value in (
            ("execution_status", ["passed"]),
            ("reason", 7),
            ("source_sha", {"sha": "bad"}),
            ("command", False),
        ):
            malformed_typed = gate_record(
                GATE_SPECS[0][0], GATE_SPECS[0][1], subprocess.check_output(
                    ["git", "-C", str(repo), "rev-parse", "HEAD"], text=True
                ).strip(), "passed"
            )
            malformed_typed[field] = value
            refresh_record_digest(malformed_typed)
            docs_evidence.write_text(
                json.dumps(malformed_typed) + "\n", encoding="utf-8"
            )
            assert projection(repo)["gates"][0]["execution_status"] == "failed"

        current_source = subprocess.check_output(
            ["git", "-C", str(repo), "rev-parse", "HEAD"], text=True
        ).strip()
        record_result = controller_result(
            repo,
            "record-gate",
            "--gate-id", GATE_SPECS[0][0],
            "--command", GATE_SPECS[0][1],
            "--exit-status", "0",
            "--expected-source-sha", current_source,
        )
        assert record_result.returncode == 0, record_result.stdout
        assert projection(repo)["gates"][0]["execution_status"] == "passed"
        before_move = current_source
        git(repo, "commit", "--allow-empty", "-m", "move during gate")
        moved_result = controller_result(
            repo,
            "record-gate",
            "--gate-id", GATE_SPECS[0][0],
            "--command", GATE_SPECS[0][1],
            "--exit-status", "0",
            "--expected-source-sha", before_move,
        )
        assert moved_result.returncode == 0, moved_result.stdout
        moved_record = json.loads(docs_evidence.read_text(encoding="utf-8"))
        assert moved_record["execution_status"] == "failed"
        assert moved_record["source_sha"] == before_move
        assert "changed during gate" in moved_record["reason"]

        module_spec = importlib.util.spec_from_file_location("delivery_mode", CONTROLLER)
        delivery_mode = importlib.util.module_from_spec(module_spec)
        module_spec.loader.exec_module(delivery_mode)
        atomic_path = Path(temporary) / "atomic.json"
        atomic_path.write_text('{"old":true}\n', encoding="utf-8")
        with mock.patch.object(delivery_mode.os, "replace", side_effect=OSError("interrupt")):
            try:
                delivery_mode.write_json(atomic_path, {"new": True})
            except OSError:
                pass
            else:
                raise AssertionError("atomic interruption did not fail")
        assert atomic_path.read_text(encoding="utf-8") == '{"old":true}\n'
        assert not list(atomic_path.parent.glob(f".{atomic_path.name}.*"))

        wrapper_repo = root_wrapper_fixture(temporary)
        wrapper_evidence = delivery_evidence(wrapper_repo)
        downstream_marker = Path(temporary) / "downstream-marker"
        for gate_id, command, filename in GATE_SPECS:
            exact_result = subprocess.run(
                command.split(),
                cwd=wrapper_repo,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                env={**os.environ, "APPFW_DOWNSTREAM_MARKER": str(downstream_marker)},
            )
            retained_path = wrapper_repo / "target/appfw/gate-evidence" / filename
            assert retained_path.is_file(), (
                f"{command} did not retain evidence: "
                f"{exact_result.returncode} {exact_result.stdout} {exact_result.stderr}"
            )
            retained = json.loads(retained_path.read_text(encoding="utf-8"))
            assert retained["gate_id"] == gate_id
            assert retained["command"] == command
            assert retained["execution_status"] == (
                "passed" if exact_result.returncode == 0 else "failed"
            )
            assert retained["source_sha"] == subprocess.check_output(
                ["git", "-C", str(wrapper_repo), "rev-parse", "HEAD"], text=True
            ).strip()
            if gate_id == "framework-docs-check":
                assert downstream_marker.read_text(encoding="utf-8") == "downstream-ran\n"

        wrapper_default = subprocess.run(
            [str(wrapper_repo / "scripts/appfw"), "mode", "status", "--json"],
            cwd=wrapper_repo,
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        wrapper_default_payload = json.loads(wrapper_default.stdout)
        assert wrapper_default_payload["delivery_profile"]["profile"] == "accelerated"
        assert wrapper_default_payload["delivery_profile"]["state"] == "tracked-default"
        assert delivery_evidence(wrapper_repo) == wrapper_evidence

        accelerated_gate_evidence = gate_evidence(wrapper_repo)
        downstream_marker.unlink(missing_ok=True)
        (wrapper_repo / "dirty-source").write_text("implementation in progress\n", encoding="utf-8")
        accelerated_dirty = subprocess.run(
            GATE_SPECS[0][1].split(),
            cwd=wrapper_repo,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env={**os.environ, "APPFW_DOWNSTREAM_MARKER": str(downstream_marker)},
        )
        assert accelerated_dirty.returncode == 0, accelerated_dirty.stderr
        assert downstream_marker.read_text(encoding="utf-8") == "downstream-ran\n"
        assert gate_evidence(wrapper_repo) == accelerated_gate_evidence
        (wrapper_repo / "dirty-source").unlink()

        accelerated_root = root_mode(wrapper_repo, "accelerated")
        assert accelerated_root["delivery_profile"]["profile"] == "accelerated"
        assert_ledger_matches_schema(root_ledger(wrapper_repo), "accelerated", "deferred")
        candidate_root = root_mode(wrapper_repo, "candidate")
        assert candidate_root["delivery_profile"]["profile"] == "candidate"
        assert_ledger_matches_schema(root_ledger(wrapper_repo), "candidate", "required")

        candidate_gate_evidence = gate_evidence(wrapper_repo)
        downstream_marker.unlink(missing_ok=True)
        (wrapper_repo / "dirty-source").write_text("candidate mutation\n", encoding="utf-8")
        for candidate_command in (
            "scripts/appfw framework docs-check --full --json",
            "scripts/appfw framework docs-check --subcheck shell-syntax --json",
        ):
            candidate_dirty = subprocess.run(
                candidate_command.split(),
                cwd=wrapper_repo,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                env={**os.environ, "APPFW_DOWNSTREAM_MARKER": str(downstream_marker)},
            )
            assert candidate_dirty.returncode != 0, candidate_command
            assert "clean checkout" in candidate_dirty.stderr
            assert not downstream_marker.exists(), candidate_command
            assert gate_evidence(wrapper_repo) == candidate_gate_evidence, candidate_command
        (wrapper_repo / "dirty-source").unlink()

        root_mode(wrapper_repo, "accelerated")
        downstream_marker.unlink(missing_ok=True)
        (wrapper_repo / "dirty-source").write_text("accelerated mutation\n", encoding="utf-8")
        accelerated_option_evidence = gate_evidence(wrapper_repo)
        for accelerated_command in (
            "scripts/appfw framework docs-check --fast --json",
            "scripts/appfw framework docs-check --subcheck shell-syntax --json",
        ):
            accelerated_dirty = subprocess.run(
                accelerated_command.split(),
                cwd=wrapper_repo,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                env={**os.environ, "APPFW_DOWNSTREAM_MARKER": str(downstream_marker)},
            )
            assert accelerated_dirty.returncode == 0, accelerated_dirty.stderr
            assert downstream_marker.read_text(encoding="utf-8") == "downstream-ran\n"
            assert gate_evidence(wrapper_repo) == accelerated_option_evidence, accelerated_command
            downstream_marker.unlink()
        (wrapper_repo / "dirty-source").unlink()
        root_mode(wrapper_repo, "candidate")

        candidate_missing_handoff = root_handoff(wrapper_repo)
        assert candidate_missing_handoff["ok"] is False
        assert candidate_missing_handoff["delivery_profile"]["gate_execution_projection"]["ok"] is False
        write_gate_records(wrapper_repo, ["passed"] * 3)
        candidate_passed_handoff = root_handoff(wrapper_repo)
        assert candidate_passed_handoff["ok"] is True
        assert candidate_passed_handoff["delivery_profile"]["gate_execution_projection"]["ok"] is True
        root_mode(wrapper_repo, "accelerated")
        write_gate_records(wrapper_repo, ["failed", "missing", "stale"])
        accelerated_handoff = root_handoff(wrapper_repo)
        assert accelerated_handoff["ok"] is True
        assert accelerated_handoff["delivery_profile"]["gate_execution_projection"]["ok"] is True
        assert [gate["execution_status"] for gate in accelerated_handoff["delivery_profile"]["gate_execution_projection"]["gates"]] == [
            "failed", "missing", "stale"
        ]

        package_root = Path(temporary) / "package"
        (package_root / "scripts").mkdir(parents=True)
        (package_root / "docs/start").mkdir(parents=True)
        shutil.copy2(CONTROLLER, package_root / "scripts/appfw-delivery-mode.py")
        shutil.copy2(POLICY, package_root / "docs/start/delivery-profiles.json")
        package_identity = {
            "version": "0.1.5",
            "platform": "linux-x64",
            "dependency_source": "registry",
            "cargo_registry": "pds-app-framework-crates",
            "cargo_registry_index": "https://packages.example.invalid/index",
        }
        (package_root / "app-framework-package.json").write_text(
            json.dumps(package_identity), encoding="utf-8"
        )
        no_git_repo = Path(temporary) / "no-git-consumer"
        no_git_repo.mkdir()
        no_git_evidence = sorted(no_git_repo.rglob("*"))
        no_git = controller_result(
            no_git_repo,
            "status",
            env={"APPFW_FRAMEWORK_ROOT": str(package_root)},
        )
        assert no_git.returncode == 0, no_git.stdout
        no_git_payload = json.loads(no_git.stdout)["delivery_profile"]
        assert no_git_payload["state"] == "package-no-git-default"
        assert no_git_payload["source_status"] == "package-no-git"
        assert no_git_payload["profile"] == "accelerated"
        assert no_git_payload["package_identity"] == package_identity
        assert "source_sha" not in no_git_payload
        assert "current_sha" not in no_git_payload
        assert "dirty" not in no_git_payload
        assert "stale" not in no_git_payload
        assert sorted(no_git_repo.rglob("*")) == no_git_evidence
        no_git_set = controller_result(
            no_git_repo,
            "set",
            "candidate",
            env={"APPFW_FRAMEWORK_ROOT": str(package_root)},
        )
        assert no_git_set.returncode == 2
        assert "Git worktree directory is unavailable" in json.loads(no_git_set.stdout)["error"]
        assert sorted(no_git_repo.rglob("*")) == no_git_evidence

        malformed_identity = package_root / "app-framework-package.json"
        malformed_identity.write_text("{}\n", encoding="utf-8")
        malformed = controller_result(
            no_git_repo,
            "status",
            env={"APPFW_FRAMEWORK_ROOT": str(package_root)},
        )
        assert malformed.returncode == 2
        assert "package identity" in json.loads(malformed.stdout)["error"]

    assert delivery_evidence(ROOT) == original_evidence
    print("appfw delivery mode tests: ok")


if __name__ == "__main__":
    main()
