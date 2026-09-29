#!/usr/bin/env python3
"""Deterministic contract tests for additive feature-check progress evidence."""

from __future__ import annotations

import json
import os
import shutil
import stat
import subprocess
import tempfile
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
REPORT_PATH = REPO_ROOT / "target/appfw/feature-check.json"
LOG_DIR = REPO_ROOT / "target/appfw/feature-check-logs"
PRODUCT_MANIFEST = REPO_ROOT / "examples/products/crm/backend/Cargo.toml"

EXPECTED_NAMES = [
    "runtime core without ingress defaults",
    "runtime core excludes HTTP ingress dependency graph",
    "runtime http ingress feature",
    "runtime mcp ingress feature",
    "runtime kafka ingress feature",
    "runtime mcp excludes HTTP ingress dependency graph",
    "runtime kafka excludes HTTP ingress dependency graph",
    "product backend core without ingress defaults",
    "product backend core excludes HTTP ingress dependency graph",
    "product backend http feature",
    "product backend mcp feature",
    "product backend kafka feature",
    "product backend mcp excludes HTTP ingress dependency graph",
    "product backend kafka excludes HTTP ingress dependency graph",
    "product backend provider-postgres provider feature",
    "product backend provider-mongo provider feature",
    "product backend provider-mssql provider feature",
    "product backend provider-snowflake provider feature",
    "product backend provider-neo4j provider feature",
]

EXPECTED_COMMANDS = [
    "check --locked -p appfw-runtime --no-default-features",
    "tree --locked -p appfw-runtime --no-default-features --prefix none",
    "check --locked -p appfw-runtime --no-default-features --features http",
    "check --locked -p appfw-runtime --no-default-features --features mcp",
    "check --locked -p appfw-runtime --no-default-features --features kafka",
    "tree --locked -p appfw-runtime --no-default-features --features mcp --prefix none",
    "tree --locked -p appfw-runtime --no-default-features --features kafka --prefix none",
    f"check --locked --manifest-path {PRODUCT_MANIFEST} --no-default-features",
    f"tree --locked --manifest-path {PRODUCT_MANIFEST} --no-default-features --prefix none",
    f"check --locked --manifest-path {PRODUCT_MANIFEST} --no-default-features --features http",
    f"check --locked --manifest-path {PRODUCT_MANIFEST} --no-default-features --features mcp",
    f"check --locked --manifest-path {PRODUCT_MANIFEST} --no-default-features --features kafka",
    f"tree --locked --manifest-path {PRODUCT_MANIFEST} --no-default-features --features mcp --prefix none",
    f"tree --locked --manifest-path {PRODUCT_MANIFEST} --no-default-features --features kafka --prefix none",
    f"check --locked --manifest-path {PRODUCT_MANIFEST} --no-default-features --features provider-postgres",
    f"check --locked --manifest-path {PRODUCT_MANIFEST} --no-default-features --features provider-mongo",
    f"check --locked --manifest-path {PRODUCT_MANIFEST} --no-default-features --features provider-mssql",
    f"check --locked --manifest-path {PRODUCT_MANIFEST} --no-default-features --features provider-snowflake",
    f"check --locked --manifest-path {PRODUCT_MANIFEST} --no-default-features --features provider-neo4j",
]


def require(condition: bool, detail: str) -> None:
    if not condition:
        raise AssertionError(detail)


def write_fake_cargo(path: Path) -> None:
    path.write_text(
        """#!/usr/bin/env bash
set -euo pipefail
printf '%s\\n' "$*" >>"$APPFW_FEATURE_CHECK_FIXTURE_LOG"
mode="${APPFW_FEATURE_CHECK_FIXTURE_MODE:-success}"
runtime_core='check --locked -p appfw-runtime --no-default-features'
runtime_tree='tree --locked -p appfw-runtime --no-default-features --prefix none'
if [[ "$mode" == "command-failure" && "$*" == "$runtime_core" ]]; then
  printf 'deterministic compile failure\\n' >&2
  exit 23
fi
if [[ "$mode" == "tree-command-failure" && "$*" == "$runtime_tree" ]]; then
  printf 'deterministic cargo tree failure\\n' >&2
  exit 29
fi
if [[ "$mode" == "tree-match" && "$*" == "$runtime_tree" ]]; then
  printf 'axum v0.7.0\\n'
fi
exit 0
""",
        encoding="utf-8",
    )
    path.chmod(path.stat().st_mode | stat.S_IXUSR)


def write_fake_python(path: Path) -> None:
    path.write_text(
        """#!/usr/bin/env bash
set -euo pipefail
monotonic_call='import time; print(time.monotonic_ns() // 1_000_000)'
if [[ "$#" -eq 2 && "$1" == "-c" && "$2" == "$monotonic_call" ]]; then
  state_file="$APPFW_FEATURE_CHECK_CLOCK_STATE"
  call_count=0
  if [[ -f "$state_file" ]]; then
    read -r call_count <"$state_file"
  fi
  call_count=$((call_count + 1))
  printf '%s\\n' "$call_count" >"$state_file"
  sequence="${APPFW_FEATURE_CHECK_CLOCK_SEQUENCE:-stable}"
  if [[ "$sequence" == "malformed" ]]; then
    printf 'not-a-clock-sample\\n'
    exit 0
  fi
  if [[ "$sequence" == "leading-zero" ]]; then
    printf '0%s\\n' "$((7 + call_count))"
    exit 0
  fi
  if [[ "$sequence" == "oversized" ]]; then
    printf '9223372036854775808\\n'
    exit 0
  fi
  if [[ "$sequence" == "always-fail" \
    || ( "$sequence" == "start-sample-failure" && "$call_count" -eq 2 ) \
    || ( "$sequence" == "finish-sample-failure" && "$call_count" -eq 3 ) \
    || ( "$sequence" == "failure-then-success" && "$call_count" -eq 1 ) ]]; then
    printf 'forced-monotonic-failure:%s:%s\\n' "$sequence" "$call_count" >>"$APPFW_FEATURE_CHECK_CLOCK_FALLBACK_LOG"
    exit 17
  fi
fi
exec "$APPFW_FEATURE_CHECK_REAL_PYTHON" "$@"
""",
        encoding="utf-8",
    )
    path.chmod(path.stat().st_mode | stat.S_IXUSR)


def write_fake_date(path: Path) -> None:
    path.write_text(
        """#!/usr/bin/env bash
set -euo pipefail
if [[ "$#" -eq 1 && "$1" == "+%s" ]]; then
  case "${APPFW_FEATURE_CHECK_DATE_SEQUENCE:-stable}" in
    fail)
      exit 19
      ;;
    malformed)
      printf 'not-epoch-seconds\\n'
      exit 0
      ;;
    leading-zero)
      printf '0000000008\\n'
      exit 0
      ;;
    oversized)
      printf '9223372036854775808\\n'
      exit 0
      ;;
  esac
fi
exec "$APPFW_FEATURE_CHECK_REAL_DATE" "$@"
""",
        encoding="utf-8",
    )
    path.chmod(path.stat().st_mode | stat.S_IXUSR)


def parse_progress(stderr: str) -> list[dict[str, object]]:
    events: list[dict[str, object]] = []
    for line in stderr.splitlines():
        if not line.startswith("{"):
            continue
        payload = json.loads(line)
        if str(payload.get("event", "")).startswith("feature-check-"):
            events.append(payload)
    return events


def run_fixture(
    fake_bin: Path,
    fixture_root: Path,
    mode: str,
    *,
    real_python: Path,
    clock_sequence: str = "stable",
    date_sequence: str = "stable",
) -> dict[str, object]:
    fixture_id = f"{mode}-clock-{clock_sequence}-date-{date_sequence}"
    command_log = fixture_root / f"{fixture_id}-commands.log"
    clock_fallback_log = fixture_root / f"{fixture_id}-clock-fallback.log"
    clock_state = fixture_root / f"{fixture_id}-clock-state"
    real_date = shutil.which("date")
    require(real_date is not None, "date is required to run this fixture")
    env = os.environ.copy()
    env["PATH"] = f"{fake_bin.parent}{os.pathsep}{env['PATH']}"
    env["APPFW_FEATURE_CHECK_FIXTURE_MODE"] = mode
    env["APPFW_FEATURE_CHECK_FIXTURE_LOG"] = str(command_log)
    env["APPFW_FEATURE_CHECK_REAL_PYTHON"] = str(real_python)
    env["APPFW_FEATURE_CHECK_REAL_DATE"] = real_date
    env["APPFW_FEATURE_CHECK_CLOCK_SEQUENCE"] = clock_sequence
    env["APPFW_FEATURE_CHECK_DATE_SEQUENCE"] = date_sequence
    env["APPFW_FEATURE_CHECK_CLOCK_FALLBACK_LOG"] = str(clock_fallback_log)
    env["APPFW_FEATURE_CHECK_CLOCK_STATE"] = str(clock_state)
    completed = subprocess.run(
        [str(REPO_ROOT / "scripts/appfw"), "framework", "feature-check", "--json"],
        cwd=REPO_ROOT,
        env=env,
        text=True,
        capture_output=True,
        check=False,
    )
    expected_status = 0 if mode == "success" else 1
    require(
        completed.returncode == expected_status,
        f"{mode}: aggregate exit {completed.returncode}, expected {expected_status}",
    )
    payload = json.loads(completed.stdout)
    retained = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    require(payload["command"] == "feature-check", f"{mode}: wrong command")
    require(payload["ok"] is (mode == "success"), f"{mode}: wrong aggregate ok")
    require(
        [item["name"] for item in payload["checks"]] == EXPECTED_NAMES,
        f"{mode}: check name/count/order drift",
    )
    require(
        [item["name"] for item in retained["checks"]] == EXPECTED_NAMES,
        f"{mode}: retained check name/count/order drift",
    )
    require(
        isinstance(payload.get("elapsed_ms"), int) and payload["elapsed_ms"] >= 0,
        f"{mode}: aggregate elapsed_ms missing or invalid",
    )
    require(
        isinstance(payload.get("timing_ok"), bool),
        f"{mode}: aggregate timing_ok missing or invalid",
    )
    require(
        payload.get("timing_clock") in {"monotonic", "epoch-seconds", "unavailable"},
        f"{mode}: aggregate timing_clock missing or invalid",
    )

    commands = command_log.read_text(encoding="utf-8").splitlines()
    require(commands == EXPECTED_COMMANDS, f"{mode}: command matrix/order drift")

    events = parse_progress(completed.stderr)
    require(len(events) == len(EXPECTED_NAMES) * 2, f"{mode}: progress event count")
    for index, name in enumerate(EXPECTED_NAMES):
        start = events[index * 2]
        finish = events[index * 2 + 1]
        require(
            start == {"event": "feature-check-start", "name": name},
            f"{mode}: start event drift for {name}",
        )
        require(finish.get("event") == "feature-check-finish", f"{mode}: finish event")
        require(finish.get("name") == name, f"{mode}: finish order for {name}")
        require(
            isinstance(finish.get("elapsed_ms"), int) and finish["elapsed_ms"] >= 0,
            f"{mode}: finish elapsed for {name}",
        )
        require(isinstance(finish.get("exit_code"), int), f"{mode}: finish exit code")
        require(isinstance(finish.get("ok"), bool), f"{mode}: finish ok")
        require(
            isinstance(finish.get("timing_ok"), bool),
            f"{mode}: finish timing_ok",
        )

    for item in payload["checks"]:
        require(
            isinstance(item.get("elapsed_ms"), int) and item["elapsed_ms"] >= 0,
            f"{mode}: check elapsed for {item['name']}",
        )
        require(isinstance(item.get("exit_code"), int), f"{mode}: check exit code")
        require(isinstance(item.get("timing_ok"), bool), f"{mode}: check timing_ok")

    return {
        "payload": payload,
        "events": events,
        "clock_fallback_log": clock_fallback_log,
        "clock_state": clock_state,
    }


def assert_failure(
    result: dict[str, object], *, index: int, exit_code: int, detail_token: str
) -> None:
    payload = result["payload"]
    events = result["events"]
    assert isinstance(payload, dict)
    assert isinstance(events, list)
    item = payload["checks"][index]
    finish = events[index * 2 + 1]
    require(item["ok"] is False, f"failure item {index} did not fail")
    require(item["exit_code"] == exit_code, f"failure item {index} lost exit code")
    require(detail_token in item.get("detail", ""), f"failure item {index} lost detail")
    require(finish["ok"] is False, f"failure finish {index} did not fail")
    require(finish["exit_code"] == exit_code, f"failure finish {index} exit code")
    require(
        all(other["ok"] is True for position, other in enumerate(payload["checks"]) if position != index),
        f"failure fixture {index} changed another check",
    )


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="appfw-feature-check-progress-") as tmp:
        fixture_root = Path(tmp)
        fake_bin = fixture_root / "bin/cargo"
        fake_bin.parent.mkdir(parents=True)
        write_fake_cargo(fake_bin)
        fake_python = fake_bin.parent / "python3"
        real_python = shutil.which("python3")
        require(real_python is not None, "python3 is required to run this fixture")
        write_fake_python(fake_python)
        write_fake_date(fake_bin.parent / "date")
        real_python_path = Path(real_python)

        report_backup = REPORT_PATH.read_bytes() if REPORT_PATH.exists() else None
        log_backup = fixture_root / "feature-check-logs-backup"
        if LOG_DIR.exists():
            shutil.copytree(LOG_DIR, log_backup)

        try:
            success = run_fixture(
                fake_bin, fixture_root, "success", real_python=real_python_path
            )
            require(
                all(item["exit_code"] == 0 for item in success["payload"]["checks"]),
                "success fixture did not retain zero exit codes",
            )
            require(
                success["payload"]["timing_clock"] == "monotonic"
                and success["payload"]["timing_ok"] is True
                and all(item["timing_ok"] is True for item in success["payload"]["checks"]),
                "success fixture did not retain valid monotonic timing evidence",
            )

            clock_fallback = run_fixture(
                fake_bin,
                fixture_root,
                "success",
                real_python=real_python_path,
                clock_sequence="always-fail",
            )
            clock_fallback_log = clock_fallback["clock_fallback_log"]
            assert isinstance(clock_fallback_log, Path)
            require(
                clock_fallback_log.exists()
                and clock_fallback_log.read_text(encoding="utf-8").splitlines(),
                "clock fallback fixture did not force the monotonic helper to fail",
            )
            require(
                clock_fallback["payload"]["elapsed_ms"] % 1000 == 0
                and all(
                    item["elapsed_ms"] % 1000 == 0
                    for item in clock_fallback["payload"]["checks"]
                ),
                "clock fallback fixture did not retain seconds-based millisecond evidence",
            )
            require(
                clock_fallback["payload"]["timing_clock"] == "epoch-seconds"
                and clock_fallback["payload"]["timing_ok"] is True
                and all(
                    item["timing_ok"] is True
                    for item in clock_fallback["payload"]["checks"]
                ),
                "clock fallback fixture did not mark epoch timing as valid",
            )

            for sequence in ("start-sample-failure", "finish-sample-failure"):
                transition = run_fixture(
                    fake_bin,
                    fixture_root,
                    "success",
                    real_python=real_python_path,
                    clock_sequence=sequence,
                )
                transition_log = transition["clock_fallback_log"]
                assert isinstance(transition_log, Path)
                require(
                    transition_log.exists(),
                    f"{sequence} fixture did not force one clock failure",
                )
                require(
                    transition["payload"]["timing_clock"] == "monotonic"
                    and transition["payload"]["timing_ok"] is True,
                    f"{sequence} fixture lost valid aggregate timing",
                )
                require(
                    transition["payload"]["checks"][0]["elapsed_ms"] == 0
                    and transition["payload"]["checks"][0]["timing_ok"] is False
                    and all(
                        item["timing_ok"] is True
                        for item in transition["payload"]["checks"][1:]
                    ),
                    f"{sequence} fixture did not isolate degraded item timing",
                )
                require(
                    transition["events"][1]["timing_ok"] is False
                    and all(
                        event["timing_ok"] is True
                        for event in transition["events"][3::2]
                    ),
                    f"{sequence} progress events did not expose timing recovery",
                )

            failure_then_success = run_fixture(
                fake_bin,
                fixture_root,
                "success",
                real_python=real_python_path,
                clock_sequence="failure-then-success",
            )
            transition_state = failure_then_success["clock_state"]
            assert isinstance(transition_state, Path)
            require(
                transition_state.read_text(encoding="utf-8").strip() == "1",
                "failure-then-success fixture retried Python after selecting epoch fallback",
            )
            require(
                failure_then_success["payload"]["elapsed_ms"] % 1000 == 0
                and all(
                    item["elapsed_ms"] % 1000 == 0
                    for item in failure_then_success["payload"]["checks"]
                ),
                "failure-then-success fixture switched away from epoch fallback",
            )
            require(
                failure_then_success["payload"]["timing_clock"] == "epoch-seconds"
                and failure_then_success["payload"]["timing_ok"] is True
                and all(
                    item["timing_ok"] is True
                    for item in failure_then_success["payload"]["checks"]
                ),
                "failure-then-success fixture did not retain epoch timing validity",
            )

            malformed_python = run_fixture(
                fake_bin,
                fixture_root,
                "success",
                real_python=real_python_path,
                clock_sequence="malformed",
            )
            require(
                malformed_python["payload"]["timing_clock"] == "epoch-seconds"
                and malformed_python["payload"]["timing_ok"] is True,
                "malformed Python clock output was not rejected in favor of epoch timing",
            )

            leading_zero_python = run_fixture(
                fake_bin,
                fixture_root,
                "success",
                real_python=real_python_path,
                clock_sequence="leading-zero",
            )
            require(
                leading_zero_python["payload"]["timing_clock"] == "monotonic"
                and leading_zero_python["payload"]["timing_ok"] is True
                and all(
                    item["timing_ok"] is True
                    for item in leading_zero_python["payload"]["checks"]
                ),
                "leading-zero monotonic samples were not canonicalized as decimal",
            )

            leading_zero_date = run_fixture(
                fake_bin,
                fixture_root,
                "success",
                real_python=real_python_path,
                clock_sequence="always-fail",
                date_sequence="leading-zero",
            )
            require(
                leading_zero_date["payload"]["timing_clock"] == "epoch-seconds"
                and leading_zero_date["payload"]["timing_ok"] is True
                and leading_zero_date["payload"]["ok"] is True,
                "leading-zero epoch samples were not canonicalized as decimal",
            )

            oversized_python = run_fixture(
                fake_bin,
                fixture_root,
                "success",
                real_python=real_python_path,
                clock_sequence="oversized",
            )
            require(
                oversized_python["payload"]["timing_clock"] == "epoch-seconds"
                and oversized_python["payload"]["timing_ok"] is True
                and oversized_python["payload"]["ok"] is True,
                "oversized monotonic sample was not rejected in favor of epoch timing",
            )

            oversized_date = run_fixture(
                fake_bin,
                fixture_root,
                "success",
                real_python=real_python_path,
                clock_sequence="always-fail",
                date_sequence="oversized",
            )
            require(
                oversized_date["payload"]["timing_clock"] == "unavailable"
                and oversized_date["payload"]["timing_ok"] is False
                and oversized_date["payload"]["ok"] is True,
                "oversized epoch sample did not degrade timing without failing compile evidence",
            )

            for date_sequence in ("fail", "malformed"):
                unavailable = run_fixture(
                    fake_bin,
                    fixture_root,
                    "success",
                    real_python=real_python_path,
                    clock_sequence="always-fail",
                    date_sequence=date_sequence,
                )
                require(
                    unavailable["payload"]["ok"] is True
                    and unavailable["payload"]["timing_clock"] == "unavailable"
                    and unavailable["payload"]["timing_ok"] is False
                    and unavailable["payload"]["elapsed_ms"] == 0
                    and all(
                        item["timing_ok"] is False and item["elapsed_ms"] == 0
                        for item in unavailable["payload"]["checks"]
                    ),
                    f"{date_sequence} date fixture did not separate compile success from unavailable timing",
                )

            command_failure = run_fixture(
                fake_bin,
                fixture_root,
                "command-failure",
                real_python=real_python_path,
            )
            assert_failure(
                command_failure,
                index=0,
                exit_code=23,
                detail_token="feature-check-logs/runtime-core-without-ingress-defaults.log",
            )

            tree_match = run_fixture(
                fake_bin, fixture_root, "tree-match", real_python=real_python_path
            )
            assert_failure(
                tree_match,
                index=1,
                exit_code=1,
                detail_token="excluded ingress packages were present",
            )

            tree_command_failure = run_fixture(
                fake_bin,
                fixture_root,
                "tree-command-failure",
                real_python=real_python_path,
            )
            assert_failure(
                tree_command_failure,
                index=1,
                exit_code=29,
                detail_token="cargo tree failed",
            )
        finally:
            if REPORT_PATH.exists():
                REPORT_PATH.unlink()
            if report_backup is not None:
                REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
                REPORT_PATH.write_bytes(report_backup)
            if LOG_DIR.exists():
                shutil.rmtree(LOG_DIR)
            if log_backup.exists():
                shutil.copytree(log_backup, LOG_DIR)

    print("Feature-check progress evidence fixtures passed.")


if __name__ == "__main__":
    main()
