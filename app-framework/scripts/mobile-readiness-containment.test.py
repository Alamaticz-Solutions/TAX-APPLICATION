#!/usr/bin/env python3
"""Prove the legacy mobile verifier cannot make a release claim.

The fixture intentionally satisfies every check that historically fed the
`mobile-test` release-ready calculation. The command must preserve that useful
diagnostic signal while keeping candidate/release authority fail-closed.
"""

from __future__ import annotations

import datetime as dt
import copy
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


REPO_ROOT = Path(__file__).resolve().parents[1]
PERFORMANCE_EVIDENCE_REQUIREMENT_ALIASES = (
    "APPFW_REQUIRE_PERFORMANCE_EVIDENCE",
    "APPFW_RELEASE_REQUIRE_PERFORMANCE_EVIDENCE",
)


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def command_evidence(name: str, command: list[str], generated_at: str) -> dict[str, object]:
    return {
        "command": command,
        "cwd": "mobile",
        "exit_code": 0,
        "generated_at": generated_at,
        "lane": "U5",
        "name": name,
        "ok": True,
        "status": "passed",
        "stderr_tail": "",
        "stdout_tail": "1 test suite passed",
        "version": 1,
    }


def require(condition: bool, detail: str) -> None:
    if not condition:
        raise AssertionError(detail)


def reject_duplicate_json_keys(pairs: list[tuple[str, object]]) -> dict[str, object]:
    value: dict[str, object] = {}
    for key, child in pairs:
        if key in value:
            raise ValueError(f"duplicate object key {key!r}")
        value[key] = child
    return value


def strict_json_loads(text: str) -> object:
    return json.loads(text, object_pairs_hook=reject_duplicate_json_keys)


def synthetic_release_environment(environment: dict[str, str]) -> dict[str, str]:
    child_environment = environment.copy()
    for alias in PERFORMANCE_EVIDENCE_REQUIREMENT_ALIASES:
        child_environment[alias] = "false"
    return child_environment


def load_synthetic_release_report(
    path: Path,
    completed: subprocess.CompletedProcess[str],
    label: str,
) -> dict[str, object]:
    require(
        path.is_file(),
        f"{label} must write {path}: stdout={completed.stdout!r}; stderr={completed.stderr!r}",
    )
    report = strict_json_loads(path.read_text(encoding="utf-8"))
    require(isinstance(report, dict), f"{label} report must be a JSON object")
    return report


def require_synthetic_performance_is_not_applicable(report: dict[str, object]) -> None:
    artifacts = report.get("artifacts")
    require(isinstance(artifacts, list), "synthetic release report must list artifacts")
    for artifact_name in ("load-test", "provider-performance"):
        matches = [
            item
            for item in artifacts
            if isinstance(item, dict) and item.get("name") == artifact_name
        ]
        require(len(matches) == 1, f"synthetic release report must list one {artifact_name} artifact")
        artifact = matches[0]
        require(artifact.get("required") is False, f"synthetic {artifact_name} must not be required")
        require(
            artifact.get("required_in_ci") is False,
            f"synthetic {artifact_name} must not be required in CI",
        )
        require(
            artifact.get("status") == "not-applicable"
            and artifact.get("applicability") == "not-applicable",
            f"synthetic {artifact_name} must be not-applicable",
        )


def invalid_contained_readiness_paths(value: object) -> list[str]:
    matches: list[str] = []
    pending: list[tuple[str, object]] = [("$", value)]
    while pending:
        current_path, current = pending.pop()
        if isinstance(current, dict):
            for key, child in current.items():
                child_path = f"{current_path}.{key}"
                if key in {"candidate_ready", "release_ready"} and child is not False:
                    matches.append(child_path)
                pending.append((child_path, child))
        elif isinstance(current, list):
            for index, child in enumerate(current):
                pending.append((f"{current_path}[{index}]", child))
    return sorted(matches)


def require_staged_containment(stage_root: Path) -> None:
    staged_paths = sorted(stage_root.glob("u5-*.json"))
    require(staged_paths, f"expected staged U5 artifacts under {stage_root}")
    for staged_path in staged_paths:
        staged_value = strict_json_loads(staged_path.read_text(encoding="utf-8"))
        claim_paths = invalid_contained_readiness_paths(staged_value)
        require(
            not claim_paths,
            f"{staged_path.name} readiness fields must be absent or exact false: {claim_paths}",
        )


def run_appfw(
    app_root: Path,
    framework_root: Path,
    *arguments: str,
) -> subprocess.CompletedProcess[str]:
    environment = os.environ.copy()
    environment.update(
        {
            "APPFW_APP_ROOT": str(app_root),
            "APPFW_CONFIG_ROOT": str(app_root / ".appfw/model"),
            "APPFW_FRAMEWORK_ROOT": str(framework_root),
            "APPFW_REPORT_ROOT": str(app_root / ".appfw/target/appfw"),
        }
    )
    return subprocess.run(
        [str(REPO_ROOT / "scripts/appfw"), *arguments],
        cwd=REPO_ROOT,
        env=environment,
        check=False,
        capture_output=True,
        text=True,
    )


def main() -> None:
    source_mobile = REPO_ROOT / "examples/products/crm/mobile"
    require(source_mobile.is_dir(), f"missing mobile fixture source: {source_mobile}")

    with tempfile.TemporaryDirectory(prefix="appfw-mobile-readiness-containment-") as temp_dir:
        temp_root = Path(temp_dir)
        app_root = temp_root / "product"
        framework_root = temp_root / "framework"
        mobile_root = app_root / "mobile"
        evidence_root = mobile_root / ".appfw-mobile"
        shutil.copytree(source_mobile, mobile_root)

        generated_at = dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z")
        write_json(
            evidence_root / "typecheck-evidence.json",
            command_evidence("typecheck", ["npm", "run", "typecheck"], generated_at),
        )
        write_json(
            evidence_root / "test-evidence.json",
            command_evidence("test", ["npm", "run", "test"], generated_at),
        )
        write_json(
            evidence_root / "expo-doctor-evidence.json",
            command_evidence("expo-doctor", ["npm", "run", "doctor"], generated_at),
        )

        npm_audit = command_evidence(
            "npm-audit-runtime",
            ["npm", "audit", "--omit=dev", "--json"],
            generated_at,
        )
        npm_audit["audit_summary"] = {
            "critical": 0,
            "high": 0,
            "low": 0,
            "moderate": 0,
            "total": 0,
        }
        write_json(evidence_root / "npm-audit-evidence.json", npm_audit)
        write_json(
            evidence_root / "npm-audit-disposition.json",
            {
                "approved_by": "synthetic-security-authority",
                "audit_evidence": "mobile/.appfw-mobile/npm-audit-evidence.json",
                "current_evidence_summary": copy.deepcopy(npm_audit["audit_summary"]),
                "expires_on": (dt.date.today() + dt.timedelta(days=30)).isoformat(),
                "lane": "U5",
                "name": "npm-audit-runtime-disposition",
                "reason": "Synthetic containment fixture for a structurally valid diagnostic disposition.",
                "release_approved": True,
                "status": "mitigated_by_controls",
                "template_only": False,
                "version": 1,
            },
        )

        write_json(
            evidence_root / "device-evidence.json",
            {
                "app_profile": "preview",
                "command": ["npm", "run", "test:device"],
                "device_target": "synthetic-ios-device-pool",
                "evidence_artifacts": ["mobile/.appfw-mobile/artifacts/ios-smoke.json"],
                "lane": "U5",
                "name": "mobile-device-smoke-evidence",
                "platform": "ios",
                "status": "passed",
                "tested_at": generated_at,
                "version": 1,
            },
        )
        write_json(
            evidence_root / "store-track-evidence.json",
            {
                "app_profile": "preview",
                "approved_by": "synthetic-mobile-release-authority",
                "binary_digest": "sha256:synthetic",
                "build_id": "synthetic-ios-preview-1",
                "evidence_artifacts": ["mobile/.appfw-mobile/artifacts/store-track.json"],
                "lane": "U5",
                "name": "mobile-store-track-evidence",
                "release_track": "testflight",
                "runtime_version": "1.0.0",
                "signing_identity": "synthetic-development-signing-identity",
                "status": "certified",
                "store_metadata": {"status": "ready-for-review"},
                "tested_at": generated_at,
                "version": 1,
            },
        )

        completed = run_appfw(app_root, framework_root, "product", "mobile-test", "--json")
        require(
            completed.returncode == 0,
            f"mobile-test failed unexpectedly ({completed.returncode}): {completed.stderr}\n{completed.stdout}",
        )
        report = json.loads(completed.stdout)

        require(report.get("ok") is True, "legacy diagnostic fixture should remain structurally green")
        require(report.get("release_ready") is False, "legacy verifier must keep release_ready false")
        require(
            report.get("legacy_evidence_satisfied") is True,
            "fixture must exercise the historical all-evidence-satisfied path",
        )
        require(report.get("candidate_ready") is False, "legacy verifier must keep candidate_ready false")
        require(report.get("readiness_level") == "static-scaffold", "legacy green must remain static-scaffold only")

        authority = report.get("readiness_authority")
        require(isinstance(authority, dict), "mobile-test must emit readiness_authority")
        require(authority.get("authoritative") is False, "readiness authority must be explicitly false")
        require(authority.get("status") == "contained-non-authoritative", "containment status must be stable")
        require(authority.get("containment_card") == "M0-05", "containment must trace to M0-05")
        require(authority.get("release_authority") == "none", "legacy verifier must declare no release authority")

        require(report.get("runtime_audit", {}).get("legacy_condition_satisfied") is True, "audit diagnostic must pass")
        require(report.get("device_evidence", {}).get("legacy_condition_satisfied") is True, "device diagnostic must pass")
        require(report.get("store_track_evidence", {}).get("legacy_condition_satisfied") is True, "store diagnostic must pass")
        for name in ("typecheck", "test", "expo_doctor"):
            require(
                report.get("command_evidence", {}).get(name, {}).get("legacy_condition_satisfied") is True,
                f"{name} diagnostic must pass",
            )
        report_claim_paths = invalid_contained_readiness_paths(report)
        require(
            not report_claim_paths,
            f"legacy report must not emit nested candidate/release readiness claims: {report_claim_paths}",
        )

        mobile_artifact = next(
            item for item in report.get("release_artifacts", []) if item.get("name") == "u5-mobile-test"
        )
        require(mobile_artifact.get("release_ready") is False, "staged U5 summary must remain non-release-ready")

        staged_report_path = framework_root / "target/appfw/wave2/u5-mobile-test.json"
        staged_report = json.loads(staged_report_path.read_text(encoding="utf-8"))
        require(staged_report.get("release_ready") is False, "retained staged report must remain fail-closed")
        require(
            staged_report.get("readiness_authority", {}).get("authoritative") is False,
            "retained staged report must preserve non-authoritative status",
        )
        stage_root = framework_root / "target/appfw/wave2"
        require_staged_containment(stage_root)

        baseline_report = copy.deepcopy(report)
        raw_stage_targets = {
            evidence_root / "npm-audit-evidence.json": stage_root / "u5-npm-audit-evidence.json",
            evidence_root / "npm-audit-disposition.json": stage_root / "u5-npm-audit-disposition.json",
            evidence_root / "device-evidence.json": stage_root / "u5-device-evidence.json",
            evidence_root / "store-track-evidence.json": stage_root / "u5-store-track-evidence.json",
        }
        safe_raw_sources = {
            source: json.loads(source.read_text(encoding="utf-8"))
            for source in raw_stage_targets
        }

        def rejected_artifact_for(
            completed_run: subprocess.CompletedProcess[str],
            source: Path,
            target: Path,
        ) -> tuple[dict[str, object], dict[str, object]]:
            require(completed_run.returncode == 1, f"{source.name} rejection must make mobile-test nonzero")
            rejected_report = json.loads(completed_run.stdout)
            require(rejected_report.get("ok") is False, f"{source.name} rejection must fail the report")
            rejected_artifact = next(
                item
                for item in rejected_report.get("release_artifacts", [])
                if item.get("name") == target.stem
            )
            require(rejected_artifact.get("source_present") is True, f"{source.name} source must be reported present")
            require(rejected_artifact.get("staging_allowed") is False, f"{source.name} staging must be denied")
            require(rejected_artifact.get("source_rejected") is True, f"{source.name} rejection must be explicit")
            require(rejected_artifact.get("staged") is False, f"{source.name} target must be reported unstaged")
            require(not target.exists(), f"{source.name} rejection must delete its staged target")
            return rejected_report, rejected_artifact

        def restore_safe_source(source: Path, target: Path) -> None:
            write_json(source, safe_raw_sources[source])
            restored = run_appfw(app_root, framework_root, "product", "mobile-test", "--json")
            require(restored.returncode == 0, f"restored {source.name} should return mobile-test to diagnostic green")
            require(target.is_file(), f"restored {source.name} must be staged again")
            staged_value = strict_json_loads(target.read_text(encoding="utf-8"))
            require(staged_value == safe_raw_sources[source], f"{source.name} must stage its validated canonical value")
            require_staged_containment(stage_root)

        expected_claim_paths = {"$.candidate_ready", "$.containment_probe.release_ready"}
        for source, target in raw_stage_targets.items():
            forged_source = copy.deepcopy(safe_raw_sources[source])
            forged_source["candidate_ready"] = True
            forged_source["containment_probe"] = {"release_ready": True}
            write_json(source, forged_source)

            rejected = run_appfw(app_root, framework_root, "product", "mobile-test", "--json")
            rejected_report, rejected_artifact = rejected_artifact_for(rejected, source, target)
            require(rejected_artifact.get("source_parse_valid") is True, f"{source.name} forgery must still parse strictly")
            actual_claim_paths = set(rejected_artifact.get("forbidden_readiness_claims", []))
            require(
                expected_claim_paths <= actual_claim_paths,
                f"{source.name} must report every forbidden JSON path: {actual_claim_paths}",
            )
            violation_text = "\n".join(rejected_report.get("blocking_violations", []))
            for claim_path in expected_claim_paths:
                require(claim_path in violation_text, f"{source.name} blocking violation must name {claim_path}")
            restore_safe_source(source, target)

        for source, target in raw_stage_targets.items():
            source.write_text('{"candidate_ready": tru\n', encoding="utf-8")
            malformed = run_appfw(app_root, framework_root, "product", "mobile-test", "--json")
            malformed_report, malformed_artifact = rejected_artifact_for(malformed, source, target)
            require(malformed_artifact.get("source_parse_valid") is False, f"{source.name} malformed JSON must fail strict parse")
            parse_error = malformed_artifact.get("source_parse_error")
            require(isinstance(parse_error, str) and "$" in parse_error, f"{source.name} parse error must name JSON root")
            require("JSON parse failed" in parse_error, f"{source.name} parse error must be explicit: {parse_error!r}")
            require(parse_error in "\n".join(malformed_report.get("blocking_violations", [])), f"{source.name} parse error must block")
            restore_safe_source(source, target)

            source.write_text('{"release_ready":true,"release_ready":false}\n', encoding="utf-8")
            duplicate = run_appfw(app_root, framework_root, "product", "mobile-test", "--json")
            _, duplicate_artifact = rejected_artifact_for(duplicate, source, target)
            require(duplicate_artifact.get("source_parse_valid") is False, f"{source.name} duplicate keys must fail strict parse")
            duplicate_error = duplicate_artifact.get("source_parse_error")
            require(
                isinstance(duplicate_error, str) and "duplicate object key 'release_ready'" in duplicate_error,
                f"{source.name} duplicate-key error must be explicit: {duplicate_error!r}",
            )
            restore_safe_source(source, target)

            confused_source = copy.deepcopy(safe_raw_sources[source])
            confused_source["containment_probe"] = {"candidate_ready": 1}
            write_json(source, confused_source)
            confused = run_appfw(app_root, framework_root, "product", "mobile-test", "--json")
            confused_report, confused_artifact = rejected_artifact_for(confused, source, target)
            require(confused_artifact.get("source_parse_valid") is True, f"{source.name} type-confusion input is valid JSON")
            invalid_fields = confused_artifact.get("invalid_readiness_fields", [])
            require(
                any(
                    item.get("path") == "$.containment_probe.candidate_ready"
                    and item.get("observed_type") == "number"
                    and item.get("observed_value") == "1"
                    for item in invalid_fields
                ),
                f"{source.name} must report readiness type/value confusion: {invalid_fields!r}",
            )
            require(
                "$.containment_probe.candidate_ready=1 (number)"
                in "\n".join(confused_report.get("blocking_violations", [])),
                f"{source.name} type-confusion violation must include path, value, and type",
            )
            restore_safe_source(source, target)

        stale_device_target = framework_root / "target/appfw/wave2/u5-device-evidence.json"
        write_json(stale_device_target, {"release_ready": True, "status": "stale-forged"})
        (evidence_root / "device-evidence.json").unlink()
        missing_device = run_appfw(app_root, framework_root, "product", "mobile-test", "--json")
        require(missing_device.returncode == 1, "missing required device evidence must keep mobile-test nonzero")
        require(not stale_device_target.exists(), "missing source evidence must invalidate its stale staged target")

        forged_mobile = copy.deepcopy(baseline_report)
        forged_mobile["candidate_ready"] = True
        forged_mobile["readiness_level"] = "release-ready"
        forged_mobile["release_ready"] = True
        forged_mobile["readiness_authority"] = {
            "authoritative": True,
            "containment_card": "M0-05",
            "release_authority": "self-declared",
            "status": "release-ready",
        }
        write_json(app_root / ".appfw/target/appfw/mobile-test.json", forged_mobile)
        wave2_completed = run_appfw(app_root, framework_root, "framework", "wave2-status", "--json")
        require(
            wave2_completed.returncode == 0,
            f"report-only wave2-status failed unexpectedly: {wave2_completed.stderr}\n{wave2_completed.stdout}",
        )
        wave2_report = json.loads(wave2_completed.stdout)
        u5_lane = next(item for item in wave2_report.get("lanes", []) if item.get("code") == "U5")
        require(u5_lane.get("release_ready") is False, "wave2 must reject forged legacy U5 readiness")
        require(
            u5_lane.get("details", {}).get("readiness_authority", {}).get("authoritative") is False,
            "wave2 must derive U5 non-authority independently of the forged producer",
        )
        u5_findings = u5_lane.get("details", {}).get("release_schema", {}).get("findings", [])
        require(
            any("legacy mobile-test is non-authoritative" in finding for finding in u5_findings),
            f"wave2 must explain legacy containment; findings={u5_findings!r}",
        )

        nested_mobile = copy.deepcopy(baseline_report)
        nested_mobile["containment_probe"] = {
            "candidate_ready": True,
            "release_ready": True,
        }
        write_json(app_root / ".appfw/target/appfw/mobile-test.json", nested_mobile)
        nested_wave2_completed = run_appfw(app_root, framework_root, "framework", "wave2-status", "--json")
        require(
            nested_wave2_completed.returncode == 0,
            f"report-only nested Wave2 check failed unexpectedly: {nested_wave2_completed.stderr}",
        )
        nested_wave2_report = json.loads(nested_wave2_completed.stdout)
        nested_wave2_u5 = next(item for item in nested_wave2_report.get("lanes", []) if item.get("code") == "U5")
        nested_wave2_schema = nested_wave2_u5.get("details", {}).get("release_schema", {})
        require(nested_wave2_u5.get("release_ready") is False, "Wave2 must keep a nested-only U5 forgery false")
        require(nested_wave2_schema.get("schema_ok") is False, "Wave2 must independently reject nested-only readiness")
        nested_wave2_findings = nested_wave2_schema.get("findings", [])
        require(
            any("$.containment_probe.release_ready=true" in finding for finding in nested_wave2_findings),
            f"Wave2 nested-only rejection must name the forbidden path/value: {nested_wave2_findings!r}",
        )

        duplicate_mobile_text = json.dumps(baseline_report, indent=2, sort_keys=True)
        top_level_release_marker = '\n  "release_ready": false,'
        require(top_level_release_marker in duplicate_mobile_text, "mobile fixture must contain top-level release_ready")
        duplicate_mobile_text = duplicate_mobile_text.replace(
            top_level_release_marker,
            '\n  "release_ready": true,\n  "release_ready": false,',
            1,
        )
        (app_root / ".appfw/target/appfw/mobile-test.json").write_text(
            duplicate_mobile_text + "\n",
            encoding="utf-8",
        )
        duplicate_mobile_wave2 = run_appfw(app_root, framework_root, "framework", "wave2-status", "--json")
        require(duplicate_mobile_wave2.returncode == 0, "Wave2 duplicate-producer check remains report-only")
        duplicate_mobile_wave2_report = json.loads(duplicate_mobile_wave2.stdout)
        duplicate_mobile_u5 = next(
            item for item in duplicate_mobile_wave2_report.get("lanes", []) if item.get("code") == "U5"
        )
        duplicate_mobile_artifact = duplicate_mobile_u5.get("artifacts", [])[0]
        require(duplicate_mobile_u5.get("release_ready") is False, "duplicate mobile producer must stay fail-closed")
        require(
            duplicate_mobile_u5.get("details", {}).get("release_schema", {}).get("schema_ok") is False,
            "duplicate mobile producer must fail the Wave2 U5 schema",
        )
        require(
            "duplicate object key 'release_ready'" in duplicate_mobile_artifact.get("parse_error", ""),
            f"Wave2 must expose duplicate mobile producer ambiguity: {duplicate_mobile_artifact!r}",
        )

        forged_wave2 = copy.deepcopy(wave2_report)
        forged_u5 = next(item for item in forged_wave2.get("lanes", []) if item.get("code") == "U5")
        forged_u5["release_ready"] = True
        write_json(framework_root / "target/appfw/wave2-readiness.json", forged_wave2)
        release_evidence_output = framework_root / "target/appfw/release-evidence-containment-test.json"
        outer_true_regression = os.environ.copy()
        outer_true_regression["CI"] = "true"
        outer_true_regression.update({alias: "true" for alias in PERFORMANCE_EVIDENCE_REQUIREMENT_ALIASES})
        release_environment = synthetic_release_environment(outer_true_regression)
        require(
            all(
                outer_true_regression[alias] == "true"
                for alias in PERFORMANCE_EVIDENCE_REQUIREMENT_ALIASES
            ),
            "synthetic release isolation must not mutate the outer performance requirement",
        )
        require(
            all(
                release_environment[alias] == "false"
                for alias in PERFORMANCE_EVIDENCE_REQUIREMENT_ALIASES
            ),
            "synthetic release child must disable both performance requirement aliases",
        )
        require(
            release_environment.get("CI") == "true",
            "synthetic release child must preserve the outer CI environment",
        )
        require(
            {
                key
                for key in outer_true_regression.keys() | release_environment.keys()
                if outer_true_regression.get(key) != release_environment.get(key)
            }
            == set(PERFORMANCE_EVIDENCE_REQUIREMENT_ALIASES),
            "synthetic release isolation must change exactly the two performance requirement aliases",
        )

        release_environment.update(
            {
                "APPFW_RELEASE_ARTIFACT_DIR": str(framework_root / "target/appfw"),
                "APPFW_RELEASE_EVIDENCE_OUTPUT": str(release_evidence_output),
            }
        )
        strict_consumer = subprocess.run(
            ["bash", str(REPO_ROOT / "scripts/ci/release-evidence-check.sh"), "--local-fixture"],
            cwd=REPO_ROOT,
            env=release_environment,
            check=False,
            capture_output=True,
            text=True,
        )
        require(strict_consumer.returncode != 0, "release evidence validation must reject forged U5 readiness")
        strict_report = load_synthetic_release_report(
            release_evidence_output,
            strict_consumer,
            "forged U5 release consumer",
        )
        require_synthetic_performance_is_not_applicable(strict_report)
        containment_checks = [
            item
            for item in strict_report.get("checks", [])
            if item.get("name") == "wave2 U5 legacy mobile lane is non-authoritative"
        ]
        require(containment_checks, "strict evidence must retain the U5 containment check")
        require(containment_checks[-1].get("ok") is False, "forged U5 readiness must fail the containment check")

        nested_forged_wave2 = copy.deepcopy(wave2_report)
        nested_forged_u5 = next(item for item in nested_forged_wave2.get("lanes", []) if item.get("code") == "U5")
        nested_forged_u5["release_ready"] = False
        nested_forged_u5.setdefault("details", {})["containment_probe"] = {
            "candidate_ready": True,
            "release_ready": True,
        }
        write_json(framework_root / "target/appfw/wave2-readiness.json", nested_forged_wave2)
        nested_release_evidence_output = framework_root / "target/appfw/release-evidence-nested-containment-test.json"
        nested_release_environment = release_environment.copy()
        nested_release_environment["APPFW_RELEASE_EVIDENCE_OUTPUT"] = str(nested_release_evidence_output)
        nested_strict_consumer = subprocess.run(
            ["bash", str(REPO_ROOT / "scripts/ci/release-evidence-check.sh"), "--local-fixture"],
            cwd=REPO_ROOT,
            env=nested_release_environment,
            check=False,
            capture_output=True,
            text=True,
        )
        require(nested_strict_consumer.returncode != 0, "strict evidence must reject nested U5 readiness claims")
        nested_strict_report = load_synthetic_release_report(
            nested_release_evidence_output,
            nested_strict_consumer,
            "nested U5 release consumer",
        )
        require_synthetic_performance_is_not_applicable(nested_strict_report)
        nested_authority_checks = [
            item
            for item in nested_strict_report.get("checks", [])
            if item.get("name") == "wave2 U5 legacy mobile lane is non-authoritative"
        ]
        require(nested_authority_checks, "strict evidence must retain the top-level U5 authority check")
        require(
            nested_authority_checks[-1].get("ok") is True,
            "nested forgery fixture must preserve the valid top-level containment envelope",
        )
        nested_claim_checks = [
            item
            for item in nested_strict_report.get("checks", [])
            if item.get("name") == "wave2 U5 legacy mobile lane contains only false readiness fields"
        ]
        require(nested_claim_checks, "strict evidence must retain the nested U5 containment check")
        require(nested_claim_checks[-1].get("ok") is False, "nested U5 readiness claims must fail independently")

        duplicate_wave2_text = json.dumps(wave2_report, indent=2, sort_keys=True)
        u5_marker = '"code": "U5",'
        require(u5_marker in duplicate_wave2_text, "Wave2 fixture must contain a U5 lane marker")
        duplicate_wave2_text = duplicate_wave2_text.replace(
            u5_marker,
            '"code": "U5",\n      "release_ready": true,',
            1,
        )
        (framework_root / "target/appfw/wave2-readiness.json").write_text(
            duplicate_wave2_text + "\n",
            encoding="utf-8",
        )
        duplicate_wave2_output = framework_root / "target/appfw/release-evidence-duplicate-wave2-test.json"
        duplicate_wave2_environment = release_environment.copy()
        duplicate_wave2_environment["APPFW_RELEASE_EVIDENCE_OUTPUT"] = str(duplicate_wave2_output)
        duplicate_wave2_consumer = subprocess.run(
            ["bash", str(REPO_ROOT / "scripts/ci/release-evidence-check.sh"), "--local-fixture"],
            cwd=REPO_ROOT,
            env=duplicate_wave2_environment,
            check=False,
            capture_output=True,
            text=True,
        )
        require(duplicate_wave2_consumer.returncode != 0, "strict evidence must reject duplicate Wave2 keys")
        duplicate_wave2_report = load_synthetic_release_report(
            duplicate_wave2_output,
            duplicate_wave2_consumer,
            "duplicate-key Wave2 release consumer",
        )
        require_synthetic_performance_is_not_applicable(duplicate_wave2_report)
        duplicate_parse_checks = [
            item
            for item in duplicate_wave2_report.get("checks", [])
            if item.get("name") == "wave2 readiness evidence parses as JSON"
        ]
        require(duplicate_parse_checks, "strict evidence must record the duplicate-key parse check")
        require(duplicate_parse_checks[-1].get("ok") is False, "duplicate Wave2 keys must fail strict parsing")
        require(
            "duplicate object key 'release_ready'" in duplicate_parse_checks[-1].get("detail", ""),
            f"duplicate Wave2 failure must identify the key: {duplicate_parse_checks[-1]!r}",
        )

        missing_app_root = temp_root / "missing-product"
        missing_workspace = run_appfw(
            missing_app_root,
            temp_root / "missing-framework",
            "product",
            "mobile-test",
            "--json",
        )
        require(missing_workspace.returncode == 1, "missing workspace compatibility path must remain nonzero")
        missing_report = json.loads(missing_workspace.stdout)
        require(missing_report.get("ok") is False, "missing workspace must remain structurally not ok")
        require(missing_report.get("candidate_ready") is False, "missing workspace candidate readiness must be false")
        require(missing_report.get("release_ready") is False, "missing workspace release readiness must be false")
        require(
            missing_report.get("readiness_authority", {}).get("authoritative") is False,
            "missing workspace must preserve the same fail-closed authority contract",
        )

    print("mobile readiness containment: OK")


if __name__ == "__main__":
    main()
