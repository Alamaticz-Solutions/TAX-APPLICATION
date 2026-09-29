#!/usr/bin/env python3
"""Bounded, exact-SHA pull-request preflight for App Framework.

The runner intentionally provides an early deterministic signal only.  Change
impact is retained as report-only telemetry and never selects or suppresses a
later pipeline gate.
"""

from __future__ import annotations

import argparse
import codecs
import datetime as dt
import hashlib
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import xml.etree.ElementTree as ET
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import BinaryIO, Iterable, Sequence


RESULT_SCHEMA = "appfw_pr_preflight@1"
PROGRESS_SCHEMA = "appfw_pr_preflight_progress@1"
MANIFEST_SCHEMA = "appfw_pr_preflight_manifest@1"
CHANGE_IMPACT_UNAVAILABLE_SCHEMA = "appfw_pr_preflight_change_impact_unavailable@1"
MAX_LOG_BYTES = 64 * 1024
MAX_CAPTURE_BYTES = 8 * 1024 * 1024
DEFAULT_REMOTE_GIT_TIMEOUT_SECONDS = 45
MAX_REMOTE_GIT_TIMEOUT_SECONDS = 300
# The largest tracked blob at the implementation base is 20,267,463 bytes.
# Keep inspection bounded while allowing the current generated provider seeds.
MAX_CHANGED_BLOB_BYTES = 32 * 1024 * 1024
FETCH_DEPTH = 256
OWNED_REPORT_NAMES = {
    "pr-preflight.json",
    "pr-preflight-change-impact.json",
    "pr-preflight-progress.jsonl",
    "pr-preflight-manifest.json",
    "pr-preflight-logs",
}
FAILURE_CATEGORIES = {
    "change_classification",
    "diff_hygiene",
    "conflict_marker",
    "shell_syntax",
    "rust_format",
    "timeout",
    "infrastructure",
    "unknown",
}
CONFLICT_START = re.compile(br"^<{7}(?: .*)?$")
CONFLICT_BASE = re.compile(br"^\|{7}(?: .*)?$")
CONFLICT_MIDDLE = re.compile(br"^={7}$")
CONFLICT_END = re.compile(br"^>{7}(?: .*)?$")


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z")


def display_path(value: str) -> str:
    """Return a stable printable form without losing unusual path bytes."""

    escaped, _ = codecs.escape_encode(os.fsencode(value))
    return escaped.decode("ascii")


def destination_identity_matches(remote_sha: str, supplied_id: str) -> bool:
    """Match only Bitbucket's 12-char prefix form; full object IDs stay exact."""

    if len(supplied_id) == 12:
        return remote_sha.startswith(supplied_id)
    return remote_sha == supplied_id


def atomic_write_bytes(path: Path, payload: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.tmp-{os.getpid()}")
    temporary.write_bytes(payload)
    os.replace(temporary, path)


def write_json(path: Path, payload: object) -> None:
    atomic_write_bytes(
        path,
        (json.dumps(payload, indent=2, sort_keys=True, ensure_ascii=True) + "\n").encode(
            "utf-8"
        ),
    )


def bounded_bytes(handle: BinaryIO, limit: int = MAX_LOG_BYTES) -> tuple[bytes, bool, int]:
    handle.flush()
    handle.seek(0, os.SEEK_END)
    size = handle.tell()
    handle.seek(0)
    if size <= limit:
        return handle.read(), False, size
    first_size = limit // 2
    last_size = limit - first_size
    first = handle.read(first_size)
    handle.seek(-last_size, os.SEEK_END)
    last = handle.read(last_size)
    marker = f"\n... {size - limit} bytes omitted ...\n".encode("ascii")
    return first + marker + last, True, size


@dataclass
class CommandResult:
    argv: list[str]
    returncode: int | None
    stdout: bytes
    stderr: bytes
    stdout_size: int
    stderr_size: int
    output_truncated: bool
    timed_out: bool
    startup_error: str | None
    duration_ms: int


@dataclass
class CheckRecord:
    name: str
    status: str
    duration_ms: int
    category: str | None = None
    child_exit_code: int | None = None
    log: str | None = None
    detail: str | None = None


class PreflightFailure(Exception):
    def __init__(
        self,
        *,
        category: str,
        check: str,
        message: str,
        child_exit_code: int | None = None,
        pipeline_exit_code: int = 1,
    ) -> None:
        super().__init__(message)
        if category not in FAILURE_CATEGORIES:
            raise ValueError(f"unknown preflight failure category: {category}")
        self.category = category
        self.check = check
        self.message = message
        self.child_exit_code = child_exit_code
        self.pipeline_exit_code = pipeline_exit_code


class Preflight:
    def __init__(self, repo_root: Path, deadline_seconds: int, base: str | None) -> None:
        self.repo_root = repo_root
        self.deadline_seconds = deadline_seconds
        self.base = base
        self.started_at = utc_now()
        self.started_monotonic = time.monotonic()
        self.deadline = self.started_monotonic + deadline_seconds
        self.report_root = repo_root / "target/appfw"
        self.logs_root = self.report_root / "pr-preflight-logs"
        self.result_path = self.report_root / "pr-preflight.json"
        self.progress_path = self.report_root / "pr-preflight-progress.jsonl"
        self.manifest_path = self.report_root / "pr-preflight-manifest.json"
        self.change_impact_path = self.report_root / "change-impact.json"
        self.change_impact_snapshot_path = (
            self.report_root / "pr-preflight-change-impact.json"
        )
        self.junit_path = repo_root / "test-results/pr-preflight.xml"
        self.records: list[CheckRecord] = []
        self.log_paths: set[Path] = set()
        self.sequence = 0
        self.mode = "unknown"
        self.source_sha: str | None = None
        self.tested_sha: str | None = None
        self.tested_commit_relation: str | None = None
        self.destination_sha: str | None = None
        self.destination_branch: str | None = None
        self.merge_base: str | None = None
        self.change_impact_contract_valid = False
        self.change_impact_operational_ok: bool | None = None
        self.remote_git_timeout_seconds: int | None = None

    def prepare_evidence(self) -> None:
        # Clear only files owned by this producer.  Never clear target/appfw as
        # a whole; later PR gates own their evidence independently.
        for path in (
            self.result_path,
            self.progress_path,
            self.manifest_path,
            self.change_impact_path,
            self.change_impact_snapshot_path,
            self.junit_path,
        ):
            if path.is_file() or path.is_symlink():
                path.unlink()
        if self.logs_root.exists():
            if self.logs_root.is_dir() and not self.logs_root.is_symlink():
                shutil.rmtree(self.logs_root)
            else:
                self.logs_root.unlink()
        self.logs_root.mkdir(parents=True, exist_ok=True)
        self.junit_path.parent.mkdir(parents=True, exist_ok=True)
        self.emit("preflight", "started", detail=f"deadline_seconds={self.deadline_seconds}")

    def elapsed_ms(self) -> int:
        return int((time.monotonic() - self.started_monotonic) * 1000)

    def remaining_seconds(self) -> float:
        remaining = self.deadline - time.monotonic()
        if remaining <= 0:
            raise PreflightFailure(
                category="timeout",
                check="deadline",
                message=f"preflight exceeded its {self.deadline_seconds}-second deadline",
                pipeline_exit_code=124,
            )
        return remaining

    def emit(
        self,
        check: str,
        status: str,
        *,
        category: str | None = None,
        child_exit_code: int | None = None,
        detail: str | None = None,
    ) -> None:
        self.sequence += 1
        event = {
            "schema": PROGRESS_SCHEMA,
            "sequence": self.sequence,
            "timestamp_utc": utc_now(),
            "elapsed_ms": self.elapsed_ms(),
            "check": check,
            "status": status,
            "category": category,
            "child_exit_code": child_exit_code,
            "detail": detail,
        }
        self.progress_path.parent.mkdir(parents=True, exist_ok=True)
        with self.progress_path.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(event, sort_keys=True, ensure_ascii=True) + "\n")

    def record(
        self,
        name: str,
        status: str,
        duration_ms: int,
        *,
        category: str | None = None,
        child_exit_code: int | None = None,
        log_path: Path | None = None,
        detail: str | None = None,
    ) -> None:
        relative_log = None
        if log_path is not None:
            relative_log = log_path.relative_to(self.repo_root).as_posix()
        self.records.append(
            CheckRecord(
                name=name,
                status=status,
                duration_ms=duration_ms,
                category=category,
                child_exit_code=child_exit_code,
                log=relative_log,
                detail=detail,
            )
        )

    def log_path(self, name: str) -> Path:
        if not re.fullmatch(r"[a-z0-9-]+", name):
            raise ValueError(f"unsafe preflight log name: {name}")
        path = self.logs_root / f"{name}.log"
        # Registration means the check has started. Materialize bounded
        # evidence before exposing the path so a deadline between registration
        # and the next command cannot leave a manifest entry without a file.
        self.write_log(
            path,
            [
                "status=started",
                f"registered_at_utc={utc_now()}",
                f"elapsed_ms={self.elapsed_ms()}",
            ],
        )
        self.log_paths.add(path)
        return path

    def write_log(self, path: Path, lines: Iterable[str]) -> None:
        payload = "\n".join(lines).rstrip() + "\n"
        encoded = payload.encode("utf-8", "backslashreplace")
        if len(encoded) > MAX_LOG_BYTES:
            encoded = encoded[: MAX_LOG_BYTES - 32] + b"\n... log truncated ...\n"
        atomic_write_bytes(path, encoded)

    @staticmethod
    def process_group_exists(process_group: int) -> bool:
        try:
            os.killpg(process_group, 0)
            return True
        except (ProcessLookupError, PermissionError):
            # macOS can report EPERM for a just-terminated process group after
            # its direct child exits. In either case the runner cannot address
            # that group further; reap the direct child below.
            return False

    def terminate_process_group(
        self, process: subprocess.Popen[bytes], grace_seconds: float = 2.0
    ) -> None:
        """Terminate every child in the command's dedicated process group.

        The direct child may handle SIGTERM and exit while a descendant keeps
        running. Probe the process group, wait the bounded grace period, and
        issue SIGKILL to the group regardless of the direct child's state.
        """

        process_group = process.pid
        try:
            os.killpg(process_group, signal.SIGTERM)
        except (ProcessLookupError, PermissionError):
            pass
        grace_deadline = time.monotonic() + grace_seconds
        while time.monotonic() < grace_deadline:
            if not self.process_group_exists(process_group):
                break
            time.sleep(0.05)
        try:
            os.killpg(process_group, signal.SIGKILL)
        except (ProcessLookupError, PermissionError):
            pass
        if process.poll() is None:
            try:
                process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                try:
                    os.kill(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                process.wait()

    def run_command(
        self,
        argv: Sequence[str],
        *,
        log_path: Path | None = None,
        stdin: BinaryIO | None = None,
        retain_full_stdout: bool = False,
        timeout_seconds: float | None = None,
        environment: dict[str, str] | None = None,
    ) -> CommandResult:
        started = time.monotonic()
        timeout = self.remaining_seconds()
        if timeout_seconds is not None:
            if timeout_seconds <= 0:
                raise ValueError("command timeout must be positive")
            timeout = min(timeout, timeout_seconds)
        with tempfile.TemporaryFile() as stdout_handle, tempfile.TemporaryFile() as stderr_handle:
            try:
                process = subprocess.Popen(
                    list(argv),
                    cwd=self.repo_root,
                    stdin=stdin,
                    stdout=stdout_handle,
                    stderr=stderr_handle,
                    start_new_session=True,
                    env=environment,
                )
            except OSError as error:
                result = CommandResult(
                    argv=list(argv),
                    returncode=None,
                    stdout=b"",
                    stderr=b"",
                    stdout_size=0,
                    stderr_size=0,
                    output_truncated=False,
                    timed_out=False,
                    startup_error=f"{type(error).__name__}: {error}",
                    duration_ms=int((time.monotonic() - started) * 1000),
                )
                if log_path is not None:
                    self.write_command_log(log_path, result)
                return result

            timed_out = False
            try:
                process.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                timed_out = True
                self.terminate_process_group(process)

            stdout_handle.flush()
            stderr_handle.flush()
            stdout_handle.seek(0, os.SEEK_END)
            stdout_size = stdout_handle.tell()
            stderr_handle.seek(0, os.SEEK_END)
            stderr_size = stderr_handle.tell()
            if retain_full_stdout and stdout_size <= MAX_CAPTURE_BYTES:
                stdout_handle.seek(0)
                stdout = stdout_handle.read()
                stdout_truncated = False
            else:
                stdout, stdout_truncated, _ = bounded_bytes(stdout_handle)
            stderr, stderr_truncated, _ = bounded_bytes(stderr_handle)
            result = CommandResult(
                argv=list(argv),
                returncode=process.returncode,
                stdout=stdout,
                stderr=stderr,
                stdout_size=stdout_size,
                stderr_size=stderr_size,
                output_truncated=stdout_truncated or stderr_truncated,
                timed_out=timed_out,
                startup_error=None,
                duration_ms=int((time.monotonic() - started) * 1000),
            )
            if log_path is not None:
                self.write_command_log(log_path, result)
            return result

    def write_command_log(self, path: Path, result: CommandResult) -> None:
        lines = [
            f"command={json.dumps(result.argv, ensure_ascii=True)}",
            f"duration_ms={result.duration_ms}",
            f"return_code={result.returncode}",
            f"timed_out={str(result.timed_out).lower()}",
            f"startup_error={result.startup_error or ''}",
            f"stdout_size={result.stdout_size}",
            f"stderr_size={result.stderr_size}",
            f"output_truncated={str(result.output_truncated).lower()}",
            "stdout:",
            result.stdout.decode("utf-8", "backslashreplace"),
            "stderr:",
            result.stderr.decode("utf-8", "backslashreplace"),
        ]
        self.write_log(path, lines)

    def command_failure(
        self,
        result: CommandResult,
        *,
        check: str,
        category: str,
        message: str,
        timeout_message: str | None = None,
    ) -> PreflightFailure:
        if result.timed_out:
            return PreflightFailure(
                category="timeout",
                check=check,
                message=(
                    timeout_message
                    or f"{check} exceeded the remaining preflight deadline"
                ),
                child_exit_code=result.returncode,
                pipeline_exit_code=124,
            )
        if result.startup_error is not None:
            return PreflightFailure(
                category="infrastructure",
                check=check,
                message=f"{check} could not start its required tool",
                pipeline_exit_code=2,
            )
        exit_code = result.returncode
        pipeline_exit = exit_code if exit_code is not None and 1 <= exit_code <= 125 else 1
        return PreflightFailure(
            category=category,
            check=check,
            message=message,
            child_exit_code=exit_code,
            pipeline_exit_code=pipeline_exit,
        )

    def git_capture(
        self,
        argv: Sequence[str],
        *,
        timeout_seconds: float | None = None,
        environment: dict[str, str] | None = None,
    ) -> CommandResult:
        result = self.run_command(
            ["git", *argv],
            retain_full_stdout=True,
            timeout_seconds=timeout_seconds,
            environment=environment,
        )
        # Timeout/124 is the primary failure when a TERM-handling Git child
        # exits cleanly after its deadline, even if it also filled the bounded
        # metadata capture. Let the direct caller classify that result.
        if not result.timed_out and result.stdout_size > MAX_CAPTURE_BYTES:
            raise PreflightFailure(
                category="infrastructure",
                check="git",
                message="Git metadata output exceeded the bounded capture limit",
                pipeline_exit_code=2,
            )
        return result

    def configure_remote_git_transport(self) -> None:
        raw_timeout = os.environ.get(
            "APPFW_PR_REMOTE_GIT_TIMEOUT_SECONDS",
            str(DEFAULT_REMOTE_GIT_TIMEOUT_SECONDS),
        )
        if not re.fullmatch(r"[0-9]+", raw_timeout):
            raise PreflightFailure(
                category="infrastructure",
                check="identity",
                message=(
                    "APPFW_PR_REMOTE_GIT_TIMEOUT_SECONDS must be an integer "
                    "from 1 through 300"
                ),
                pipeline_exit_code=2,
            )
        timeout_seconds = int(raw_timeout)
        if timeout_seconds < 1 or timeout_seconds > MAX_REMOTE_GIT_TIMEOUT_SECONDS:
            raise PreflightFailure(
                category="infrastructure",
                check="identity",
                message=(
                    "APPFW_PR_REMOTE_GIT_TIMEOUT_SECONDS must be an integer "
                    "from 1 through 300"
                ),
                pipeline_exit_code=2,
            )
        self.remote_git_timeout_seconds = timeout_seconds

    def remote_git_capture(self, argv: Sequence[str]) -> CommandResult:
        if self.remote_git_timeout_seconds is None:
            raise PreflightFailure(
                category="infrastructure",
                check="identity",
                message="remote Git transport was not configured",
                pipeline_exit_code=2,
            )
        environment = os.environ.copy()
        environment["GIT_TERMINAL_PROMPT"] = "0"
        return self.git_capture(
            argv,
            timeout_seconds=self.remote_git_timeout_seconds,
            environment=environment,
        )

    def resolve_identity(self) -> None:
        started = time.monotonic()
        check = "identity"
        self.emit(check, "started")
        log_path = self.log_path(check)
        tested = self.git_capture(["rev-parse", "--verify", "HEAD^{commit}"])
        if tested.returncode != 0 or tested.startup_error or tested.timed_out:
            failure = self.command_failure(
                tested,
                check=check,
                category="infrastructure",
                message="HEAD does not resolve to a commit",
            )
            self.write_log(log_path, ["tested_commit=unavailable"])
            self.record(
                check,
                "failed",
                int((time.monotonic() - started) * 1000),
                category=failure.category,
                child_exit_code=failure.child_exit_code,
                log_path=log_path,
                detail=failure.message,
            )
            raise failure
        self.tested_sha = tested.stdout.decode("ascii", "strict").strip().lower()

        bb_branch = os.environ.get("BITBUCKET_PR_DESTINATION_BRANCH")
        bb_destination_commit = os.environ.get("BITBUCKET_PR_DESTINATION_COMMIT")
        bb_source_commit = os.environ.get("BITBUCKET_COMMIT")
        bitbucket_identity_present = any(
            value is not None
            for value in (bb_branch, bb_destination_commit, bb_source_commit)
        )
        if self.base is not None and bitbucket_identity_present:
            failure = PreflightFailure(
                category="infrastructure",
                check=check,
                message="explicit local base is forbidden when Bitbucket PR identity is present",
                pipeline_exit_code=2,
            )
            self.write_log(log_path, [f"tested_sha={self.tested_sha}", "mode=invalid-mixed"])
            self.record(
                check,
                "failed",
                int((time.monotonic() - started) * 1000),
                category=failure.category,
                log_path=log_path,
                detail=failure.message,
            )
            raise failure

        if self.base is not None:
            self.mode = "local-explicit-base"
            self.source_sha = self.tested_sha
            self.tested_commit_relation = "local-source-equals-tested"
            base_result = self.git_capture(
                ["rev-parse", "--verify", f"{self.base}^{{commit}}"]
            )
            if (
                base_result.returncode != 0
                or base_result.startup_error
                or base_result.timed_out
            ):
                failure = self.command_failure(
                    base_result,
                    check=check,
                    category="infrastructure",
                    message="explicit local base does not resolve to a commit",
                )
                self.write_log(
                    log_path,
                    [
                        f"source_sha={self.source_sha}",
                        f"tested_sha={self.tested_sha}",
                        f"tested_commit_relation={self.tested_commit_relation}",
                        "mode=local-explicit-base",
                        "base=unresolved",
                    ],
                )
                self.record(
                    check,
                    "failed",
                    int((time.monotonic() - started) * 1000),
                    category=failure.category,
                    child_exit_code=failure.child_exit_code,
                    log_path=log_path,
                    detail=failure.message,
                )
                raise failure
            self.destination_sha = base_result.stdout.decode("ascii", "strict").strip().lower()
            self.destination_branch = self.base
        else:
            self.mode = "bitbucket-pr"
            if not bb_branch or not bb_destination_commit or not bb_source_commit:
                failure = PreflightFailure(
                    category="infrastructure",
                    check=check,
                    message=(
                        "BITBUCKET_COMMIT, BITBUCKET_PR_DESTINATION_BRANCH, and "
                        "BITBUCKET_PR_DESTINATION_COMMIT are required; local runs "
                        "must pass --base"
                    ),
                    pipeline_exit_code=2,
                )
                self.write_log(
                    log_path,
                    [f"tested_sha={self.tested_sha}", "mode=identity-missing"],
                )
                self.record(
                    check,
                    "failed",
                    int((time.monotonic() - started) * 1000),
                    category=failure.category,
                    log_path=log_path,
                    detail=failure.message,
                )
                raise failure
            if not re.fullmatch(
                r"[0-9a-fA-F]{40}|[0-9a-fA-F]{64}", bb_source_commit
            ):
                failure = PreflightFailure(
                    category="infrastructure",
                    check=check,
                    message="BITBUCKET_COMMIT is not a full hexadecimal object ID",
                    pipeline_exit_code=2,
                )
                self.write_log(
                    log_path,
                    [
                        f"tested_sha={self.tested_sha}",
                        "mode=bitbucket-pr",
                        "source_commit=invalid",
                    ],
                )
                self.record(
                    check,
                    "failed",
                    int((time.monotonic() - started) * 1000),
                    category=failure.category,
                    log_path=log_path,
                    detail=failure.message,
                )
                raise failure
            source_result = self.git_capture(
                ["rev-parse", "--verify", f"{bb_source_commit}^{{commit}}"]
            )
            if (
                source_result.returncode != 0
                or source_result.startup_error
                or source_result.timed_out
            ):
                if source_result.timed_out or source_result.startup_error:
                    failure = self.command_failure(
                        source_result,
                        check=check,
                        category="infrastructure",
                        message="BITBUCKET_COMMIT does not resolve to a commit",
                    )
                else:
                    failure = PreflightFailure(
                        category="infrastructure",
                        check=check,
                        message="BITBUCKET_COMMIT does not resolve to a commit",
                        child_exit_code=source_result.returncode,
                        pipeline_exit_code=2,
                    )
                self.write_log(
                    log_path,
                    [
                        f"tested_sha={self.tested_sha}",
                        "mode=bitbucket-pr",
                        f"source_commit={bb_source_commit.lower()}",
                        "source_resolution=unavailable",
                    ],
                )
                self.record(
                    check,
                    "failed",
                    int((time.monotonic() - started) * 1000),
                    category=failure.category,
                    child_exit_code=failure.child_exit_code,
                    log_path=log_path,
                    detail=failure.message,
                )
                raise failure
            self.source_sha = (
                source_result.stdout.decode("ascii", "strict").strip().lower()
            )
            if self.source_sha != bb_source_commit.lower():
                failure = PreflightFailure(
                    category="infrastructure",
                    check=check,
                    message="BITBUCKET_COMMIT resolved to an unexpected commit",
                    pipeline_exit_code=2,
                )
                self.write_log(
                    log_path,
                    [
                        f"tested_sha={self.tested_sha}",
                        "mode=bitbucket-pr",
                        f"expected_source_sha={bb_source_commit.lower()}",
                        f"resolved_source_sha={self.source_sha}",
                    ],
                )
                self.record(
                    check,
                    "failed",
                    int((time.monotonic() - started) * 1000),
                    category=failure.category,
                    log_path=log_path,
                    detail=failure.message,
                )
                raise failure
            if not re.fullmatch(
                r"(?:[0-9a-fA-F]{12}|[0-9a-fA-F]{40}|[0-9a-fA-F]{64})",
                bb_destination_commit,
            ):
                failure = PreflightFailure(
                    category="infrastructure",
                    check=check,
                    message=(
                        "Bitbucket destination commit is not a 12, 40, or 64 "
                        "character hexadecimal object ID"
                    ),
                    pipeline_exit_code=2,
                )
                self.write_log(
                    log_path,
                    [
                        f"source_sha={self.source_sha}",
                        f"tested_sha={self.tested_sha}",
                        "mode=bitbucket-pr",
                        "destination_commit=invalid",
                    ],
                )
                self.record(
                    check,
                    "failed",
                    int((time.monotonic() - started) * 1000),
                    category=failure.category,
                    log_path=log_path,
                    detail=failure.message,
                )
                raise failure
            ref_check = self.git_capture(["check-ref-format", "--branch", bb_branch])
            if (
                ref_check.returncode != 0
                or ref_check.startup_error
                or ref_check.timed_out
            ):
                failure = self.command_failure(
                    ref_check,
                    check=check,
                    category="infrastructure",
                    message="Bitbucket destination branch is not a valid branch name",
                )
                self.write_log(
                    log_path,
                    [
                        f"source_sha={self.source_sha}",
                        f"tested_sha={self.tested_sha}",
                        "mode=bitbucket-pr",
                        "destination_branch=invalid",
                    ],
                )
                self.record(
                    check,
                    "failed",
                    int((time.monotonic() - started) * 1000),
                    category=failure.category,
                    child_exit_code=failure.child_exit_code,
                    log_path=log_path,
                )
                raise failure

            try:
                self.configure_remote_git_transport()
            except PreflightFailure as failure:
                self.write_log(
                    log_path,
                    [
                        f"source_sha={self.source_sha}",
                        f"tested_sha={self.tested_sha}",
                        "mode=bitbucket-pr",
                        "remote_git_timeout_seconds=invalid",
                        "remote_git_terminal_prompt_disabled=false",
                    ],
                )
                self.record(
                    check,
                    "failed",
                    int((time.monotonic() - started) * 1000),
                    category=failure.category,
                    log_path=log_path,
                    detail=failure.message,
                )
                raise

            remote_check = self.git_capture(["remote", "get-url", "origin"])
            if (
                remote_check.returncode != 0
                or remote_check.startup_error
                or remote_check.timed_out
            ):
                failure = self.command_failure(
                    remote_check,
                    check=check,
                    category="infrastructure",
                    message="origin remote is unavailable for destination identity resolution",
                )
                self.write_log(
                    log_path,
                    [
                        f"source_sha={self.source_sha}",
                        f"tested_sha={self.tested_sha}",
                        "mode=bitbucket-pr",
                        "origin=unavailable",
                    ],
                )
                self.record(
                    check,
                    "failed",
                    int((time.monotonic() - started) * 1000),
                    category=failure.category,
                    child_exit_code=failure.child_exit_code,
                    log_path=log_path,
                )
                raise failure

            remote_destination = self.remote_git_capture(
                ["ls-remote", "--exit-code", "origin", f"refs/heads/{bb_branch}"]
            )
            if (
                remote_destination.returncode != 0
                or remote_destination.startup_error
                or remote_destination.timed_out
            ):
                failure = self.command_failure(
                    remote_destination,
                    check=check,
                    category="infrastructure",
                    message="remote destination branch head is unavailable",
                    timeout_message=(
                        "remote destination query timed out after "
                        f"{self.remote_git_timeout_seconds} seconds"
                    ),
                )
                self.write_log(
                    log_path,
                    [
                        f"source_sha={self.source_sha}",
                        f"tested_sha={self.tested_sha}",
                        "mode=bitbucket-pr",
                        f"remote_git_timeout_seconds={self.remote_git_timeout_seconds}",
                        "remote_git_terminal_prompt_disabled=true",
                        "remote_destination=unavailable",
                    ],
                )
                self.record(
                    check,
                    "failed",
                    int((time.monotonic() - started) * 1000),
                    category=failure.category,
                    child_exit_code=failure.child_exit_code,
                    log_path=log_path,
                    detail=failure.message,
                )
                raise failure
            remote_lines = remote_destination.stdout.decode(
                "ascii", "strict"
            ).splitlines()
            remote_fields = remote_lines[0].split() if len(remote_lines) == 1 else []
            expected_remote_ref = f"refs/heads/{bb_branch}"
            if (
                len(remote_fields) != 2
                or remote_fields[1] != expected_remote_ref
                or not re.fullmatch(
                    r"[0-9a-fA-F]{40}|[0-9a-fA-F]{64}", remote_fields[0]
                )
            ):
                failure = PreflightFailure(
                    category="infrastructure",
                    check=check,
                    message="remote destination branch returned ambiguous identity",
                    pipeline_exit_code=2,
                )
                self.write_log(
                    log_path,
                    [
                        f"source_sha={self.source_sha}",
                        f"tested_sha={self.tested_sha}",
                        "mode=bitbucket-pr",
                        "remote_destination=ambiguous",
                    ],
                )
                self.record(
                    check,
                    "failed",
                    int((time.monotonic() - started) * 1000),
                    category=failure.category,
                    log_path=log_path,
                    detail=failure.message,
                )
                raise failure
            remote_destination_sha = remote_fields[0].lower()
            supplied_destination_id = bb_destination_commit.lower()
            if not destination_identity_matches(
                remote_destination_sha, supplied_destination_id
            ):
                failure = PreflightFailure(
                    category="infrastructure",
                    check=check,
                    message=(
                        "remote destination branch does not match pipeline "
                        "destination identity"
                    ),
                    pipeline_exit_code=2,
                )
                self.write_log(
                    log_path,
                    [
                        f"source_sha={self.source_sha}",
                        f"tested_sha={self.tested_sha}",
                        "mode=bitbucket-pr",
                        f"destination_branch={display_path(bb_branch)}",
                        f"supplied_destination_id={supplied_destination_id}",
                        f"remote_destination_sha={remote_destination_sha}",
                    ],
                )
                self.record(
                    check,
                    "failed",
                    int((time.monotonic() - started) * 1000),
                    category=failure.category,
                    log_path=log_path,
                    detail=failure.message,
                )
                raise failure

            destination_ref = "refs/appfw-pr-preflight/destination"
            fetch = self.remote_git_capture(
                [
                    "fetch",
                    "--no-tags",
                    f"--depth={FETCH_DEPTH}",
                    "--force",
                    "origin",
                    f"refs/heads/{bb_branch}:{destination_ref}",
                ]
            )
            if fetch.returncode != 0 or fetch.startup_error or fetch.timed_out:
                failure = self.command_failure(
                    fetch,
                    check=check,
                    category="infrastructure",
                    message="bounded destination fetch failed",
                    timeout_message=(
                        "destination fetch timed out after "
                        f"{self.remote_git_timeout_seconds} seconds"
                    ),
                )
                self.write_log(
                    log_path,
                    [
                        f"source_sha={self.source_sha}",
                        f"tested_sha={self.tested_sha}",
                        "mode=bitbucket-pr",
                        f"remote_git_timeout_seconds={self.remote_git_timeout_seconds}",
                        "remote_git_terminal_prompt_disabled=true",
                        "destination_fetch=failed",
                    ],
                )
                self.record(
                    check,
                    "failed",
                    int((time.monotonic() - started) * 1000),
                    category=failure.category,
                    child_exit_code=failure.child_exit_code,
                    log_path=log_path,
                )
                raise failure
            fetched = self.git_capture(["rev-parse", "--verify", f"{destination_ref}^{{commit}}"])
            if (
                fetched.returncode != 0
                or fetched.startup_error
                or fetched.timed_out
            ):
                failure = self.command_failure(
                    fetched,
                    check=check,
                    category="infrastructure",
                    message="fetched destination ref does not resolve to a commit",
                )
                self.write_log(
                    log_path,
                    [
                        f"source_sha={self.source_sha}",
                        f"tested_sha={self.tested_sha}",
                        "mode=bitbucket-pr",
                        "destination_ref=unresolved",
                    ],
                )
                self.record(
                    check,
                    "failed",
                    int((time.monotonic() - started) * 1000),
                    category=failure.category,
                    child_exit_code=failure.child_exit_code,
                    log_path=log_path,
                )
                raise failure
            fetched_sha = fetched.stdout.decode("ascii", "strict").strip().lower()
            if fetched_sha != remote_destination_sha:
                failure = PreflightFailure(
                    category="infrastructure",
                    check=check,
                    message="destination branch advanced or no longer matches pipeline identity",
                    pipeline_exit_code=2,
                )
                self.write_log(
                    log_path,
                    [
                        f"source_sha={self.source_sha}",
                        f"tested_sha={self.tested_sha}",
                        "mode=bitbucket-pr",
                        f"destination_branch={display_path(bb_branch)}",
                        f"expected_destination_sha={remote_destination_sha}",
                        f"fetched_destination_sha={fetched_sha}",
                    ],
                )
                self.record(
                    check,
                    "failed",
                    int((time.monotonic() - started) * 1000),
                    category=failure.category,
                    log_path=log_path,
                    detail=failure.message,
                )
                raise failure
            self.destination_sha = remote_destination_sha
            self.destination_branch = bb_branch

            if self.tested_sha == self.source_sha:
                ancestor = self.git_capture(
                    ["merge-base", "--is-ancestor", self.destination_sha, self.tested_sha]
                )
                if (
                    ancestor.returncode != 0
                    or ancestor.startup_error
                    or ancestor.timed_out
                ):
                    relation_message = (
                        "tested HEAD equals BITBUCKET_COMMIT but does not contain "
                        "the exact destination commit"
                    )
                    if ancestor.timed_out or ancestor.startup_error:
                        failure = self.command_failure(
                            ancestor,
                            check=check,
                            category="infrastructure",
                            message=relation_message,
                        )
                    else:
                        failure = PreflightFailure(
                            category="infrastructure",
                            check=check,
                            message=relation_message,
                            child_exit_code=ancestor.returncode,
                            pipeline_exit_code=2,
                        )
                    self.write_log(
                        log_path,
                        [
                            f"mode={self.mode}",
                            f"source_sha={self.source_sha}",
                            f"tested_sha={self.tested_sha}",
                            f"destination_sha={self.destination_sha}",
                            "tested_commit_relation=invalid-source-head",
                        ],
                    )
                    self.record(
                        check,
                        "failed",
                        int((time.monotonic() - started) * 1000),
                        category=failure.category,
                        child_exit_code=failure.child_exit_code,
                        log_path=log_path,
                        detail=failure.message,
                    )
                    raise failure
                self.tested_commit_relation = (
                    "bitbucket-source-equals-tested-destination-ancestor"
                )
            elif self.tested_sha == self.destination_sha:
                ancestor = self.git_capture(
                    ["merge-base", "--is-ancestor", self.source_sha, self.tested_sha]
                )
                if (
                    ancestor.returncode != 0
                    or ancestor.startup_error
                    or ancestor.timed_out
                ):
                    relation_message = (
                        "tested HEAD equals the destination but does not contain "
                        "the exact BITBUCKET_COMMIT source"
                    )
                    if ancestor.timed_out or ancestor.startup_error:
                        failure = self.command_failure(
                            ancestor,
                            check=check,
                            category="infrastructure",
                            message=relation_message,
                        )
                    else:
                        failure = PreflightFailure(
                            category="infrastructure",
                            check=check,
                            message=relation_message,
                            child_exit_code=ancestor.returncode,
                            pipeline_exit_code=2,
                        )
                    self.write_log(
                        log_path,
                        [
                            f"mode={self.mode}",
                            f"source_sha={self.source_sha}",
                            f"tested_sha={self.tested_sha}",
                            f"destination_sha={self.destination_sha}",
                            "tested_commit_relation=invalid-destination-head",
                        ],
                    )
                    self.record(
                        check,
                        "failed",
                        int((time.monotonic() - started) * 1000),
                        category=failure.category,
                        child_exit_code=failure.child_exit_code,
                        log_path=log_path,
                        detail=failure.message,
                    )
                    raise failure
                self.tested_commit_relation = (
                    "bitbucket-destination-equals-tested-source-ancestor"
                )
            else:
                parent_result = self.git_capture(
                    ["rev-list", "--parents", "-n", "1", self.tested_sha]
                )
                if (
                    parent_result.returncode != 0
                    or parent_result.startup_error
                    or parent_result.timed_out
                ):
                    if parent_result.timed_out or parent_result.startup_error:
                        failure = self.command_failure(
                            parent_result,
                            check=check,
                            category="infrastructure",
                            message="tested commit parents could not be resolved",
                        )
                    else:
                        failure = PreflightFailure(
                            category="infrastructure",
                            check=check,
                            message="tested commit parents could not be resolved",
                            child_exit_code=parent_result.returncode,
                            pipeline_exit_code=2,
                        )
                    self.write_log(
                        log_path,
                        [
                            f"mode={self.mode}",
                            f"source_sha={self.source_sha}",
                            f"tested_sha={self.tested_sha}",
                            f"destination_sha={self.destination_sha}",
                            "tested_commit_relation=unavailable",
                        ],
                    )
                    self.record(
                        check,
                        "failed",
                        int((time.monotonic() - started) * 1000),
                        category=failure.category,
                        child_exit_code=failure.child_exit_code,
                        log_path=log_path,
                        detail=failure.message,
                    )
                    raise failure
                parent_fields: list[str] = []
                try:
                    parent_fields = parent_result.stdout.decode(
                        "ascii", "strict"
                    ).strip().lower().split()
                except UnicodeDecodeError:
                    parent_fields = []
                direct_parents = parent_fields[1:] if parent_fields else []
                expected_parents = [self.source_sha, self.destination_sha]
                relation_valid = (
                    len(parent_fields) == 3
                    and parent_fields[0] == self.tested_sha
                    and sorted(direct_parents) == sorted(expected_parents)
                )
                if not relation_valid:
                    failure = PreflightFailure(
                        category="infrastructure",
                        check=check,
                        message=(
                            "tested HEAD is neither BITBUCKET_COMMIT nor an exact "
                            "two-parent merge of source and destination"
                        ),
                        child_exit_code=parent_result.returncode,
                        pipeline_exit_code=2,
                    )
                    self.write_log(
                        log_path,
                        [
                            f"mode={self.mode}",
                            f"source_sha={self.source_sha}",
                            f"tested_sha={self.tested_sha}",
                            f"destination_sha={self.destination_sha}",
                            "tested_commit_relation=invalid",
                            "tested_direct_parents=" + ",".join(direct_parents),
                        ],
                    )
                    self.record(
                        check,
                        "failed",
                        int((time.monotonic() - started) * 1000),
                        category=failure.category,
                        child_exit_code=failure.child_exit_code,
                        log_path=log_path,
                        detail=failure.message,
                    )
                    raise failure
                self.tested_commit_relation = (
                    "bitbucket-tested-merge-of-source-and-destination"
                )

        merge = self.git_capture(
            ["merge-base", self.destination_sha, self.tested_sha]
        )
        if (
            merge.returncode != 0
            or merge.startup_error
            or merge.timed_out
            or not merge.stdout.strip()
        ):
            failure = self.command_failure(
                merge,
                check=check,
                category="infrastructure",
                message="tested commit and destination have no merge base in bounded history",
            )
            self.write_log(
                log_path,
                [
                    f"source_sha={self.source_sha}",
                    f"tested_sha={self.tested_sha}",
                    f"destination_sha={self.destination_sha}",
                    f"tested_commit_relation={self.tested_commit_relation}",
                    f"mode={self.mode}",
                    "merge_base=unavailable",
                ],
            )
            self.record(
                check,
                "failed",
                int((time.monotonic() - started) * 1000),
                category=failure.category,
                child_exit_code=failure.child_exit_code,
                log_path=log_path,
            )
            raise failure
        self.merge_base = merge.stdout.decode("ascii", "strict").strip().lower()
        self.write_log(
            log_path,
            [
                f"mode={self.mode}",
                f"source_sha={self.source_sha}",
                f"tested_sha={self.tested_sha}",
                f"tested_commit_relation={self.tested_commit_relation}",
                f"destination_branch={display_path(self.destination_branch or '')}",
                f"destination_sha={self.destination_sha}",
                f"merge_base={self.merge_base}",
                f"fetch_depth={FETCH_DEPTH if self.mode == 'bitbucket-pr' else 'not_applicable'}",
                "remote_git_timeout_seconds="
                + (
                    str(self.remote_git_timeout_seconds)
                    if self.remote_git_timeout_seconds is not None
                    else "not_applicable"
                ),
                "remote_git_terminal_prompt_disabled="
                + str(self.remote_git_timeout_seconds is not None).lower(),
            ],
        )
        duration = int((time.monotonic() - started) * 1000)
        self.record(check, "passed", duration, log_path=log_path)
        self.emit(
            check,
            "passed",
            detail=(
                f"source_sha={self.source_sha};tested_sha={self.tested_sha};"
                f"tested_commit_relation={self.tested_commit_relation}"
            ),
        )

    def change_impact_contract_errors(self, payload: dict[str, object]) -> list[str]:
        errors: list[str] = []
        exact_values = {
            "artifact": "target/appfw/change-impact.json",
            "command": "change-impact",
            "mode": "report-only",
            "scope": "framework",
            "base_ref": self.merge_base,
            "diff_base_ref": self.merge_base,
            "head_ref": self.tested_sha,
            "sha": self.tested_sha,
        }
        for field, expected in exact_values.items():
            if payload.get(field) != expected:
                errors.append(f"{field} does not match exact preflight identity")

        if type(payload.get("ok")) is not bool:
            errors.append("ok is not a boolean")
        for forbidden in ("error", "git_errors", "git_error_count"):
            if forbidden in payload:
                errors.append(f"{forbidden} surface is present")

        if "delivery_profile" in payload:
            delivery_profile = payload.get("delivery_profile")
            if not isinstance(delivery_profile, dict):
                errors.append("delivery_profile is not an object")
            elif "gate_execution_projection" in delivery_profile:
                projection = delivery_profile.get("gate_execution_projection")
                if not isinstance(projection, dict):
                    errors.append(
                        "delivery_profile.gate_execution_projection is not an object"
                    )
                elif type(projection.get("ok")) is not bool:
                    errors.append(
                        "delivery_profile.gate_execution_projection.ok is not a boolean"
                    )

        changed_files = payload.get("changed_files")
        changed_files_valid = isinstance(changed_files, list)
        if not changed_files_valid:
            errors.append("changed_files is not a list")
        else:
            for index, entry in enumerate(changed_files):
                prefix = f"changed_files[{index}]"
                if not isinstance(entry, dict):
                    errors.append(f"{prefix} is not an object")
                    changed_files_valid = False
                    continue
                path = entry.get("path")
                if not isinstance(path, str) or not path:
                    errors.append(f"{prefix}.path is not a nonempty string")
                    changed_files_valid = False
                for field in ("added", "deleted"):
                    value = entry.get(field)
                    if type(value) is not int or value < 0:
                        errors.append(
                            f"{prefix}.{field} is not a nonnegative integer"
                        )
                        changed_files_valid = False
                status = entry.get("status")
                if not isinstance(status, str) or not status:
                    errors.append(f"{prefix}.status is not a nonempty string")
                    changed_files_valid = False
                for field in ("uncommitted", "generated_like"):
                    if type(entry.get(field)) is not bool:
                        errors.append(f"{prefix}.{field} is not a boolean")
                        changed_files_valid = False
                for field in ("buckets", "domains", "sensitive_flags"):
                    values = entry.get(field)
                    if not isinstance(values, list) or not all(
                        isinstance(value, str) for value in values
                    ):
                        errors.append(f"{prefix}.{field} is not a list of strings")
                        changed_files_valid = False

        buckets = payload.get("buckets")
        if not isinstance(buckets, dict) or not all(
            isinstance(name, str)
            and bool(name)
            and type(count) is int
            and count >= 0
            for name, count in buckets.items()
        ):
            errors.append("buckets is not a string-to-nonnegative-integer object")

        typed_path_lists: dict[str, list[object]] = {}
        for field in (
            "ownership_domains",
            "sensitive_surfaces",
            "generated_like_paths",
        ):
            values = payload.get(field)
            if not isinstance(values, list) or not all(
                isinstance(value, str) and bool(value) for value in values
            ):
                errors.append(f"{field} is not a list of nonempty strings")
            else:
                typed_path_lists[field] = values

        summary = payload.get("summary")
        nonnegative_summary_fields = (
            "changed_file_count",
            "untracked_file_count",
            "non_generated_added_lines",
            "non_generated_deleted_lines",
            "ownership_domain_count",
            "sensitive_surface_count",
            "generated_like_file_count",
        )
        if not isinstance(summary, dict):
            errors.append("summary is not an object")
        else:
            for field in nonnegative_summary_fields:
                value = summary.get(field)
                if type(value) is not int or value < 0:
                    errors.append(f"summary.{field} is not a nonnegative integer")
            line_delta = summary.get("non_generated_line_delta")
            if type(line_delta) is not int:
                errors.append("summary.non_generated_line_delta is not an integer")
            elif (
                type(summary.get("non_generated_added_lines")) is int
                and type(summary.get("non_generated_deleted_lines")) is int
                and line_delta
                != summary["non_generated_added_lines"]
                + summary["non_generated_deleted_lines"]
            ):
                errors.append(
                    "summary.non_generated_line_delta does not match added plus deleted"
                )
            if changed_files_valid and summary.get("changed_file_count") != len(
                changed_files
            ):
                errors.append("summary.changed_file_count does not match changed_files")
            summary_list_counts = {
                "ownership_domain_count": "ownership_domains",
                "sensitive_surface_count": "sensitive_surfaces",
                "generated_like_file_count": "generated_like_paths",
            }
            for count_field, list_field in summary_list_counts.items():
                if (
                    list_field in typed_path_lists
                    and summary.get(count_field) != len(typed_path_lists[list_field])
                ):
                    errors.append(f"summary.{count_field} does not match {list_field}")

        recommended = payload.get("recommended_verification")
        if not isinstance(recommended, list) or not all(
            isinstance(command, str) and bool(command) for command in recommended
        ):
            errors.append("recommended_verification is not a list of nonempty strings")

        change_class = payload.get("change_class")
        if not isinstance(change_class, dict):
            errors.append("change_class is not an object")
        else:
            if change_class.get("class") not in {"A", "B", "C", "D"}:
                errors.append("change_class.class is not A-D")
            reason = change_class.get("reason")
            if not isinstance(reason, str) or not reason.strip():
                errors.append("change_class.reason is not a nonempty string")
            for field in (
                "requires_human_review",
                "requires_integration_branch",
            ):
                if type(change_class.get(field)) is not bool:
                    errors.append(f"change_class.{field} is not a boolean")
        return errors

    def run_change_impact(self) -> None:
        started = time.monotonic()
        check = "change_classification"
        self.emit(check, "started")
        log_path = self.log_path("change-classification")
        appfw = self.repo_root / "scripts/appfw"
        result = self.run_command(
            [
                str(appfw),
                "framework",
                "change-impact",
                "--base",
                self.merge_base or "",
                "--head",
                self.tested_sha or "",
                "--json",
            ],
            log_path=log_path,
            retain_full_stdout=True,
        )
        if result.returncode != 0 or result.startup_error or result.timed_out:
            failure = self.command_failure(
                result,
                check=check,
                category="change_classification",
                message="report-only change-impact classification did not complete",
            )
            self.write_change_impact_unavailable(failure.message)
            self.record(
                check,
                "failed",
                result.duration_ms,
                category=failure.category,
                child_exit_code=failure.child_exit_code,
                log_path=log_path,
                detail=failure.message,
            )
            raise failure
        if result.stdout_size > MAX_CAPTURE_BYTES:
            failure = PreflightFailure(
                category="change_classification",
                check=check,
                message="change-impact JSON exceeded the bounded capture limit",
                pipeline_exit_code=1,
            )
            self.write_change_impact_unavailable(failure.message)
            self.record(
                check,
                "failed",
                result.duration_ms,
                category=failure.category,
                log_path=log_path,
                detail=failure.message,
            )
            raise failure
        try:
            payload = json.loads(result.stdout.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError):
            payload = None
        if not isinstance(payload, dict):
            failure = PreflightFailure(
                category="change_classification",
                check=check,
                message="change-impact did not produce a parseable JSON object",
                child_exit_code=result.returncode,
                pipeline_exit_code=1,
            )
            self.write_change_impact_unavailable(failure.message)
            self.record(
                check,
                "failed",
                result.duration_ms,
                category=failure.category,
                child_exit_code=failure.child_exit_code,
                log_path=log_path,
                detail=failure.message,
            )
            raise failure
        # Preserve the exact child payload, including delivery-profile
        # annotation. The preflight must not rewrite operational candidate
        # projection into a successful assurance claim. Keep the command-owned
        # canonical report for compatibility, but bind the preflight manifest
        # to its own immutable snapshot so later handoff/review commands may
        # refresh change-impact.json without invalidating this run's evidence.
        atomic_write_bytes(self.change_impact_path, result.stdout)
        atomic_write_bytes(self.change_impact_snapshot_path, result.stdout)
        validation_errors = self.change_impact_contract_errors(payload)
        if validation_errors:
            failure = PreflightFailure(
                category="change_classification",
                check=check,
                message=(
                    "change-impact report contract is invalid: "
                    + "; ".join(validation_errors)
                ),
                child_exit_code=result.returncode,
                pipeline_exit_code=1,
            )
            self.record(
                check,
                "failed",
                result.duration_ms,
                category=failure.category,
                child_exit_code=failure.child_exit_code,
                log_path=log_path,
                detail=failure.message,
            )
            raise failure

        self.change_impact_operational_ok = payload["ok"]
        delivery_profile = payload.get("delivery_profile")
        projection = (
            delivery_profile.get("gate_execution_projection")
            if isinstance(delivery_profile, dict)
            else None
        )
        projection_ok = projection.get("ok") if isinstance(projection, dict) else None
        coherence_error: str | None = None
        if self.change_impact_operational_ok is False and projection_ok is not False:
            coherence_error = (
                "change-impact operational ok=false is not explained by "
                "delivery_profile.gate_execution_projection.ok=false"
            )
        elif projection_ok is False and self.change_impact_operational_ok is not False:
            coherence_error = (
                "change-impact delivery profile gate projection ok=false is "
                "incoherent with top-level ok"
            )
        if coherence_error is not None:
            failure = PreflightFailure(
                category="change_classification",
                check=check,
                message=coherence_error,
                child_exit_code=result.returncode,
                pipeline_exit_code=1,
            )
            self.record(
                check,
                "failed",
                result.duration_ms,
                category=failure.category,
                child_exit_code=failure.child_exit_code,
                log_path=log_path,
                detail=failure.message,
            )
            raise failure

        self.change_impact_contract_valid = True
        detail = (
            "report_only=true;contract_valid=true;"
            f"operational_ok={str(self.change_impact_operational_ok).lower()}"
        )
        self.record(
            check,
            "passed",
            result.duration_ms,
            log_path=log_path,
            detail=detail,
        )
        self.emit(check, "passed", detail=detail)

    def write_change_impact_unavailable(self, reason: str) -> None:
        payload = {
            "schema": CHANGE_IMPACT_UNAVAILABLE_SCHEMA,
            "ok": False,
            "report_only": True,
            "skipped": True,
            "reason": reason,
            "source_sha": self.source_sha,
            "tested_sha": self.tested_sha,
            "tested_commit_relation": self.tested_commit_relation,
            "destination_sha": self.destination_sha,
            "merge_base": self.merge_base,
        }
        write_json(self.change_impact_path, payload)
        write_json(self.change_impact_snapshot_path, payload)

    def run_diff_hygiene(self) -> None:
        started = time.monotonic()
        check = "diff_hygiene"
        self.emit(check, "started")
        log_path = self.log_path("diff-hygiene")
        result = self.run_command(
            ["git", "diff", "--check", self.merge_base or "", self.tested_sha or "", "--"],
            log_path=log_path,
        )
        if result.returncode != 0 or result.startup_error or result.timed_out:
            combined = (result.stdout + b"\n" + result.stderr).lower()
            diagnostic_lines = [line for line in combined.splitlines() if line.strip()]
            conflict_only = bool(diagnostic_lines) and all(
                b"leftover conflict marker" in line for line in diagnostic_lines
            )
            if conflict_only and not result.timed_out and result.startup_error is None:
                # Git intentionally treats any isolated marker-looking line as
                # a conflict warning.  The next check requires a coherent
                # start/middle/end block, so illustrative lone lines do not
                # become false positives while git diff --check still runs.
                self.record(
                    check,
                    "passed",
                    result.duration_ms,
                    log_path=log_path,
                    detail="conflict-only diagnostics deferred to coherent marker scan",
                )
                self.emit(check, "passed", detail="conflict-only diagnostics deferred")
                return
            category = "diff_hygiene"
            failure = self.command_failure(
                result,
                check=check,
                category=category,
                message="changed content failed git diff --check",
            )
            self.record(
                check,
                "failed",
                result.duration_ms,
                category=failure.category,
                child_exit_code=failure.child_exit_code,
                log_path=log_path,
                detail=failure.message,
            )
            raise failure
        self.record(check, "passed", result.duration_ms, log_path=log_path)
        self.emit(check, "passed")

    def changed_paths(self) -> list[str]:
        result = self.git_capture(
            [
                "diff",
                "--name-only",
                "-z",
                "--diff-filter=ACMRT",
                "--find-renames",
                self.merge_base or "",
                self.tested_sha or "",
                "--",
            ]
        )
        if result.returncode != 0 or result.startup_error or result.timed_out:
            raise self.command_failure(
                result,
                check="changed_paths",
                category="infrastructure",
                message="changed path discovery failed",
            )
        if result.output_truncated:
            raise PreflightFailure(
                category="infrastructure",
                check="changed_paths",
                message="changed path list exceeded the bounded capture limit",
                pipeline_exit_code=2,
            )
        return [os.fsdecode(item) for item in result.stdout.split(b"\0") if item]

    def tree_blob(self, path: str) -> tuple[str, str] | None:
        result = self.git_capture(
            ["--literal-pathspecs", "ls-tree", "-z", self.tested_sha or "", "--", path]
        )
        if result.returncode != 0 or result.startup_error or result.timed_out:
            raise self.command_failure(
                result,
                check="git_blob",
                category="infrastructure",
                message="changed blob metadata could not be resolved",
            )
        entries = [item for item in result.stdout.split(b"\0") if item]
        if not entries:
            return None
        if len(entries) != 1 or b"\t" not in entries[0]:
            raise PreflightFailure(
                category="infrastructure",
                check="git_blob",
                message="changed blob metadata was ambiguous",
                pipeline_exit_code=2,
            )
        metadata, _ = entries[0].split(b"\t", 1)
        fields = metadata.split()
        if len(fields) != 3:
            raise PreflightFailure(
                category="infrastructure",
                check="git_blob",
                message="changed blob metadata was malformed",
                pipeline_exit_code=2,
            )
        mode, object_type, object_id = (field.decode("ascii") for field in fields)
        if object_type != "blob":
            return None
        return mode, object_id

    def materialize_blob(self, object_id: str) -> BinaryIO:
        size_result = self.git_capture(["cat-file", "-s", object_id])
        if (
            size_result.returncode != 0
            or size_result.startup_error
            or size_result.timed_out
        ):
            raise self.command_failure(
                size_result,
                check="git_blob",
                category="infrastructure",
                message="changed Git blob size could not be resolved",
            )
        try:
            blob_size = int(size_result.stdout.decode("ascii", "strict").strip())
        except (UnicodeDecodeError, ValueError) as error:
            raise PreflightFailure(
                category="infrastructure",
                check="git_blob",
                message="changed Git blob size was malformed",
                pipeline_exit_code=2,
            ) from error
        if blob_size < 0:
            raise PreflightFailure(
                category="infrastructure",
                check="git_blob",
                message="changed Git blob size was negative",
                pipeline_exit_code=2,
            )
        if blob_size > MAX_CHANGED_BLOB_BYTES:
            raise PreflightFailure(
                category="infrastructure",
                check="git_blob",
                message=(
                    f"changed Git blob is {blob_size} bytes; inspection cap is "
                    f"{MAX_CHANGED_BLOB_BYTES} bytes"
                ),
                pipeline_exit_code=2,
            )
        materialization_timeout = self.remaining_seconds()
        # Git writes through a child process directly to this descriptor.  An
        # unbuffered handle keeps the descriptor offset authoritative when the
        # same blob is sampled in Python and then passed as subprocess stdin.
        handle = tempfile.TemporaryFile(buffering=0)
        stderr_handle = tempfile.TemporaryFile()
        process: subprocess.Popen[bytes] | None = None
        retain_handle = False
        started = time.monotonic()
        try:
            try:
                process = subprocess.Popen(
                    ["git", "cat-file", "blob", object_id],
                    cwd=self.repo_root,
                    stdout=handle,
                    stderr=stderr_handle,
                    start_new_session=True,
                )
            except OSError as error:
                raise PreflightFailure(
                    category="infrastructure",
                    check="git_blob",
                    message=f"git cat-file could not start: {type(error).__name__}",
                    pipeline_exit_code=2,
                ) from error
            try:
                process.wait(timeout=materialization_timeout)
            except subprocess.TimeoutExpired as error:
                self.terminate_process_group(process, grace_seconds=0.25)
                raise PreflightFailure(
                    category="timeout",
                    check="git_blob",
                    message="git blob materialization exceeded the preflight deadline",
                    child_exit_code=process.returncode,
                    pipeline_exit_code=124,
                ) from error
            if process.returncode != 0:
                raise PreflightFailure(
                    category="infrastructure",
                    check="git_blob",
                    message="changed Git blob could not be materialized",
                    child_exit_code=process.returncode,
                    pipeline_exit_code=2,
                )
            actual_blob_size = os.fstat(handle.fileno()).st_size
            if actual_blob_size != blob_size:
                raise PreflightFailure(
                    category="infrastructure",
                    check="git_blob",
                    message="changed Git blob materialization size did not match metadata",
                    pipeline_exit_code=2,
                )
            handle.seek(0)
            _ = started
            retain_handle = True
            return handle
        finally:
            try:
                try:
                    if process is not None and process.poll() is None:
                        self.terminate_process_group(process, grace_seconds=0.25)
                except BaseException:
                    retain_handle = False
                    raise
            finally:
                try:
                    stderr_handle.close()
                except BaseException:
                    handle.close()
                    raise
                finally:
                    if not retain_handle:
                        handle.close()

    @staticmethod
    def is_binary(handle: BinaryIO) -> bool:
        handle.seek(0)
        sample = handle.read(8192)
        handle.seek(0)
        return b"\0" in sample

    def finalize_started_check_failure(
        self,
        *,
        check: str,
        started: float,
        log_path: Path,
        failure: PreflightFailure,
    ) -> PreflightFailure:
        """Guarantee a started scan has a bounded log and terminal record."""

        normalized = (
            failure
            if failure.check == check
            else PreflightFailure(
                category=failure.category,
                check=check,
                message=f"{check} helper {failure.check} failed: {failure.message}",
                child_exit_code=failure.child_exit_code,
                pipeline_exit_code=failure.pipeline_exit_code,
            )
        )
        terminal_lines = [
            f"terminal_status=failed",
            f"terminal_category={normalized.category}",
            f"terminal_helper={failure.check}",
            f"terminal_child_exit_code={failure.child_exit_code}",
            f"terminal_message={normalized.message}",
        ]
        if log_path.is_file():
            existing = log_path.read_bytes()[:MAX_LOG_BYTES].decode(
                "utf-8", "backslashreplace"
            )
            self.write_log(log_path, [existing, *terminal_lines])
        else:
            self.write_log(log_path, terminal_lines)
        if not any(
            record.name == check and record.status == "failed" for record in self.records
        ):
            self.record(
                check,
                "failed",
                int((time.monotonic() - started) * 1000),
                category=normalized.category,
                child_exit_code=normalized.child_exit_code,
                log_path=log_path,
                detail=normalized.message,
            )
        return normalized

    def run_conflict_marker_scan(self) -> None:
        started = time.monotonic()
        check = "conflict_marker"
        self.emit(check, "started")
        log_path = self.log_path("conflict-marker")
        try:
            self.run_conflict_marker_scan_body(started, check, log_path)
        except PreflightFailure as failure:
            raise self.finalize_started_check_failure(
                check=check,
                started=started,
                log_path=log_path,
                failure=failure,
            ) from failure
        except Exception as error:
            failure = PreflightFailure(
                category="unknown",
                check=check,
                message=f"conflict marker scan failed unexpectedly: {type(error).__name__}",
                pipeline_exit_code=2,
            )
            raise self.finalize_started_check_failure(
                check=check,
                started=started,
                log_path=log_path,
                failure=failure,
            ) from error

    def run_conflict_marker_scan_body(
        self, started: float, check: str, log_path: Path
    ) -> None:
        findings: list[str] = []
        scanned_paths: list[str] = []
        scanned = 0
        skipped_binary = 0
        skipped_symlink = 0
        for path in self.changed_paths():
            self.remaining_seconds()
            metadata = self.tree_blob(path)
            if metadata is None:
                continue
            mode, object_id = metadata
            if mode == "120000":
                skipped_symlink += 1
                continue
            with self.materialize_blob(object_id) as blob:
                if self.is_binary(blob):
                    skipped_binary += 1
                    continue
                scanned += 1
                scanned_paths.append(display_path(path))
                marker_state = "outside"
                marker_start_line: int | None = None
                last_line_number = 0
                for line_number, line in enumerate(blob, start=1):
                    last_line_number = line_number
                    self.remaining_seconds()
                    candidate = line.rstrip(b"\r\n")
                    is_start = CONFLICT_START.fullmatch(candidate) is not None
                    is_base = CONFLICT_BASE.fullmatch(candidate) is not None
                    is_middle = CONFLICT_MIDDLE.fullmatch(candidate) is not None
                    is_end = CONFLICT_END.fullmatch(candidate) is not None

                    if marker_state == "outside":
                        if is_start:
                            marker_state = "left"
                            marker_start_line = line_number
                        # Isolated marker-looking lines remain illustrative text.
                        # Once a start line roots a block, however, every later
                        # marker transition must be coherent and complete.
                        continue

                    if is_start:
                        findings.append(
                            f"{display_path(path)}:{marker_start_line or line_number}-{line_number}:nested-start"
                        )
                        marker_state = "outside"
                        marker_start_line = None
                    elif is_base:
                        if marker_state == "left":
                            marker_state = "base"
                        else:
                            findings.append(
                                f"{display_path(path)}:{marker_start_line or line_number}-{line_number}:invalid-base"
                            )
                            marker_state = "outside"
                            marker_start_line = None
                    elif is_middle:
                        if marker_state in {"left", "base"}:
                            marker_state = "right"
                        else:
                            findings.append(
                                f"{display_path(path)}:{marker_start_line or line_number}-{line_number}:invalid-middle"
                            )
                            marker_state = "outside"
                            marker_start_line = None
                    elif is_end:
                        if marker_state == "right":
                            findings.append(
                                f"{display_path(path)}:{marker_start_line or line_number}-{line_number}:complete"
                            )
                        else:
                            findings.append(
                                f"{display_path(path)}:{marker_start_line or line_number}-{line_number}:invalid-end"
                            )
                        marker_state = "outside"
                        marker_start_line = None

                if marker_state != "outside":
                    findings.append(
                        f"{display_path(path)}:{marker_start_line or last_line_number}-{last_line_number}:incomplete-{marker_state}"
                    )
        lines = [
            f"scanned_text_blobs={scanned}",
            f"skipped_binary_blobs={skipped_binary}",
            f"skipped_symlink_blobs={skipped_symlink}",
            f"finding_count={len(findings)}",
            *[f"scanned_path={path}" for path in scanned_paths],
            *[f"marker={finding}" for finding in findings],
        ]
        self.write_log(log_path, lines)
        duration = int((time.monotonic() - started) * 1000)
        if findings:
            failure = PreflightFailure(
                category="conflict_marker",
                check=check,
                message=(
                    "changed text blob contains an unresolved, incomplete, or "
                    "malformed conflict-marker block"
                ),
                pipeline_exit_code=1,
            )
            self.record(
                check,
                "failed",
                duration,
                category=failure.category,
                log_path=log_path,
                detail=failure.message,
            )
            raise failure
        self.record(check, "passed", duration, log_path=log_path)
        self.emit(check, "passed")

    @staticmethod
    def looks_like_shell(path: str, handle: BinaryIO) -> bool:
        if path.endswith(".sh"):
            return True
        handle.seek(0)
        first_line = handle.readline(512)
        handle.seek(0)
        return bool(
            re.match(
                br"^#!\s*(?:/usr/bin/env(?:\s+-S)?\s+|/(?:usr/)?bin/)?(?:ba|da|k|z)?sh(?:\s|$)",
                first_line,
            )
        )

    def run_shell_syntax(self) -> None:
        started = time.monotonic()
        check = "shell_syntax"
        self.emit(check, "started")
        log_path = self.log_path("shell-syntax")
        try:
            self.run_shell_syntax_body(started, check, log_path)
        except PreflightFailure as failure:
            raise self.finalize_started_check_failure(
                check=check,
                started=started,
                log_path=log_path,
                failure=failure,
            ) from failure
        except Exception as error:
            failure = PreflightFailure(
                category="unknown",
                check=check,
                message=f"shell syntax scan failed unexpectedly: {type(error).__name__}",
                pipeline_exit_code=2,
            )
            raise self.finalize_started_check_failure(
                check=check,
                started=started,
                log_path=log_path,
                failure=failure,
            ) from error

    def run_shell_syntax_body(
        self, started: float, check: str, log_path: Path
    ) -> None:
        lines: list[str] = []
        checked = 0
        skipped_symlink = 0
        for path in self.changed_paths():
            self.remaining_seconds()
            metadata = self.tree_blob(path)
            if metadata is None:
                continue
            mode, object_id = metadata
            if mode == "120000":
                skipped_symlink += 1
                continue
            with self.materialize_blob(object_id) as blob:
                if self.is_binary(blob) or not self.looks_like_shell(path, blob):
                    continue
                checked += 1
                blob.seek(0)
                result = self.run_command(["bash", "-n"], stdin=blob)
                lines.extend(
                    [
                        f"path={display_path(path)}",
                        f"return_code={result.returncode}",
                        f"timed_out={str(result.timed_out).lower()}",
                    ]
                )
                if result.stderr:
                    lines.append(
                        "stderr="
                        + result.stderr.decode("utf-8", "backslashreplace").replace("\n", "\\n")
                    )
                if result.returncode != 0 or result.startup_error or result.timed_out:
                    self.write_log(
                        log_path,
                        [f"checked_shell_blobs={checked}", f"skipped_symlink_blobs={skipped_symlink}", *lines],
                    )
                    failure = self.command_failure(
                        result,
                        check=check,
                        category="shell_syntax",
                        message=f"changed shell blob failed bash -n: {display_path(path)}",
                    )
                    self.record(
                        check,
                        "failed",
                        int((time.monotonic() - started) * 1000),
                        category=failure.category,
                        child_exit_code=failure.child_exit_code,
                        log_path=log_path,
                        detail=failure.message,
                    )
                    raise failure
        self.write_log(
            log_path,
            [f"checked_shell_blobs={checked}", f"skipped_symlink_blobs={skipped_symlink}", *lines],
        )
        duration = int((time.monotonic() - started) * 1000)
        self.record(check, "passed", duration, log_path=log_path)
        self.emit(check, "passed")

    def run_rust_format(self) -> None:
        tool_started = time.monotonic()
        tool_check = "rust_format_tool"
        self.emit(tool_check, "started")
        tool_log = self.log_path("rust-format-tool")
        if shutil.which("cargo") is None:
            self.write_log(tool_log, ["cargo=unavailable"])
            failure = PreflightFailure(
                category="infrastructure",
                check=tool_check,
                message="cargo is unavailable for rustfmt",
                pipeline_exit_code=2,
            )
            self.record(
                tool_check,
                "failed",
                int((time.monotonic() - tool_started) * 1000),
                category=failure.category,
                log_path=tool_log,
                detail=failure.message,
            )
            raise failure
        tool = self.run_command(["cargo", "fmt", "--version"], log_path=tool_log)
        if tool.returncode != 0 or tool.startup_error or tool.timed_out:
            failure = self.command_failure(
                tool,
                check=tool_check,
                category="infrastructure",
                message="rustfmt is unavailable or could not start",
            )
            self.record(
                tool_check,
                "failed",
                tool.duration_ms,
                category=failure.category,
                child_exit_code=failure.child_exit_code,
                log_path=tool_log,
                detail=failure.message,
            )
            raise failure
        self.record(tool_check, "passed", tool.duration_ms, log_path=tool_log)
        self.emit(tool_check, "passed")

        check = "rust_format"
        self.emit(check, "started")
        log_path = self.log_path("rust-format")
        result = self.run_command(
            ["cargo", "fmt", "--all", "--check"],
            log_path=log_path,
        )
        if result.returncode != 0 or result.startup_error or result.timed_out:
            category = "infrastructure" if result.startup_error else "rust_format"
            failure = self.command_failure(
                result,
                check=check,
                category=category,
                message="cargo fmt --all --check failed",
            )
            self.record(
                check,
                "failed",
                result.duration_ms,
                category=failure.category,
                child_exit_code=failure.child_exit_code,
                log_path=log_path,
                detail=failure.message,
            )
            raise failure
        self.record(check, "passed", result.duration_ms, log_path=log_path)
        self.emit(check, "passed")

    def owned_unexpected_paths(self) -> list[str]:
        unexpected: list[str] = []
        if self.report_root.exists():
            for path in self.report_root.iterdir():
                if path.name.startswith("pr-preflight") and path.name not in OWNED_REPORT_NAMES:
                    unexpected.append(path.relative_to(self.repo_root).as_posix())
        if self.logs_root.exists():
            for path in self.logs_root.rglob("*"):
                if path.is_dir():
                    unexpected.append(path.relative_to(self.repo_root).as_posix())
                elif path not in self.log_paths or path.suffix != ".log":
                    unexpected.append(path.relative_to(self.repo_root).as_posix())
        if self.junit_path.parent.exists():
            for path in self.junit_path.parent.iterdir():
                if path.name.startswith("pr-preflight") and path != self.junit_path:
                    unexpected.append(path.relative_to(self.repo_root).as_posix())
        return sorted(set(unexpected))

    def result_payload(self, failure: PreflightFailure | None) -> dict[str, object]:
        return {
            "schema": RESULT_SCHEMA,
            "ok": failure is None,
            "status": "passed" if failure is None else "failed",
            "category": None if failure is None else failure.category,
            "failed_check": None if failure is None else failure.check,
            "message": None if failure is None else failure.message,
            "child_exit_code": None if failure is None else failure.child_exit_code,
            "pipeline_exit_code": 0 if failure is None else failure.pipeline_exit_code,
            "mode": self.mode,
            "source_sha": self.source_sha,
            "tested_sha": self.tested_sha,
            "tested_commit_relation": self.tested_commit_relation,
            "destination_branch": self.destination_branch,
            "destination_sha": self.destination_sha,
            "merge_base": self.merge_base,
            "deadline_seconds": self.deadline_seconds,
            "started_at_utc": self.started_at,
            "finished_at_utc": utc_now(),
            "duration_ms": self.elapsed_ms(),
            "change_impact_is_report_only": True,
            "change_impact_contract_valid": self.change_impact_contract_valid,
            "change_impact_operational_ok": self.change_impact_operational_ok,
            "remote_git": {
                "timeout_seconds": self.remote_git_timeout_seconds,
                "terminal_prompt_disabled": self.remote_git_timeout_seconds is not None,
            },
            "downstream_gate_selection": "unchanged",
            "checks": [asdict(record) for record in self.records],
        }

    def write_junit(self, failure: PreflightFailure | None) -> None:
        records = list(self.records)
        if failure is not None and not any(
            record.status == "failed" and record.name == failure.check
            for record in records
        ):
            records.append(
                CheckRecord(
                    name=failure.check,
                    status="failed",
                    duration_ms=0,
                    category=failure.category,
                    child_exit_code=failure.child_exit_code,
                    detail=failure.message,
                )
            )
        if not records:
            records.append(
                CheckRecord(
                    name="preflight",
                    status="passed",
                    duration_ms=self.elapsed_ms(),
                )
            )
        suite = ET.Element(
            "testsuite",
            {
                "name": "appfw-pr-preflight",
                "tests": str(len(records)),
                "failures": str(
                    sum(
                        1
                        for record in records
                        if record.status == "failed"
                        and record.category not in {"infrastructure", "timeout", "unknown"}
                    )
                ),
                "errors": str(
                    sum(
                        1
                        for record in records
                        if record.status == "failed"
                        and record.category in {"infrastructure", "timeout", "unknown"}
                    )
                ),
                "time": f"{self.elapsed_ms() / 1000:.3f}",
                "timestamp": self.started_at,
            },
        )
        for record in records:
            case = ET.SubElement(
                suite,
                "testcase",
                {
                    "classname": "appfw.pr_preflight",
                    "name": record.name,
                    "time": f"{record.duration_ms / 1000:.3f}",
                },
            )
            if record.status == "failed":
                node_name = (
                    "error"
                    if record.category in {"infrastructure", "timeout", "unknown"}
                    else "failure"
                )
                node = ET.SubElement(
                    case,
                    node_name,
                    {
                        "type": record.category or "unknown",
                        "message": record.detail or "preflight check failed",
                    },
                )
                node.text = (
                    f"category={record.category}; child_exit_code={record.child_exit_code}; "
                    f"log={record.log}"
                )
        tree = ET.ElementTree(suite)
        ET.indent(tree, space="  ")
        with tempfile.TemporaryFile() as handle:
            tree.write(handle, encoding="utf-8", xml_declaration=True)
            handle.seek(0)
            atomic_write_bytes(self.junit_path, handle.read() + b"\n")

    def ensure_terminal_failure_record(self, failure: PreflightFailure) -> None:
        """Make the terminal failure part of result and JUnit evidence."""

        if any(
            record.name == failure.check and record.status == "failed"
            for record in self.records
        ):
            return
        self.record(
            failure.check,
            "failed",
            0,
            category=failure.category,
            child_exit_code=failure.child_exit_code,
            detail=failure.message,
        )

    def write_manifest(self, failure: PreflightFailure | None) -> None:
        artifact_paths = [
            self.change_impact_snapshot_path,
            self.result_path,
            self.progress_path,
            self.junit_path,
            *sorted(self.log_paths),
        ]
        artifacts: list[dict[str, object]] = []
        for path in artifact_paths:
            if not path.is_file() or path.is_symlink():
                raise PreflightFailure(
                    category="infrastructure",
                    check="artifact_manifest",
                    message=f"required preflight artifact is unavailable: {path.name}",
                    pipeline_exit_code=2,
                )
            content = path.read_bytes()
            artifacts.append(
                {
                    "path": path.relative_to(self.repo_root).as_posix(),
                    "bytes": len(content),
                    "sha256": hashlib.sha256(content).hexdigest(),
                }
            )
        write_json(
            self.manifest_path,
            {
                "schema": MANIFEST_SCHEMA,
                "ok": failure is None,
                "source_sha": self.source_sha,
                "tested_sha": self.tested_sha,
                "tested_commit_relation": self.tested_commit_relation,
                "destination_sha": self.destination_sha,
                "merge_base": self.merge_base,
                "change_impact_contract_valid": self.change_impact_contract_valid,
                "change_impact_operational_ok": self.change_impact_operational_ok,
                "remote_git": {
                    "timeout_seconds": self.remote_git_timeout_seconds,
                    "terminal_prompt_disabled": self.remote_git_timeout_seconds
                    is not None,
                },
                "manifest_path": self.manifest_path.relative_to(self.repo_root).as_posix(),
                "manifest_self_hash": "excluded-self-referential-manifest",
                "artifacts": artifacts,
            },
        )

    def finalize(self, failure: PreflightFailure | None) -> tuple[PreflightFailure | None, dict[str, object]]:
        if not self.change_impact_snapshot_path.is_file():
            self.write_change_impact_unavailable(
                failure.message if failure is not None else "change-impact did not execute"
            )
        if failure is not None:
            self.ensure_terminal_failure_record(failure)
            self.emit(
                failure.check,
                "failed",
                category=failure.category,
                child_exit_code=failure.child_exit_code,
                detail=failure.message,
            )
        else:
            self.emit("preflight", "passed")

        payload = self.result_payload(failure)
        write_json(self.result_path, payload)
        self.write_junit(failure)

        unexpected = self.owned_unexpected_paths()
        if unexpected:
            failure = PreflightFailure(
                category="infrastructure",
                check="artifact_allowlist",
                message="preflight producer created unexpected owned evidence paths: "
                + ", ".join(unexpected),
                pipeline_exit_code=2,
            )
            allowlist_log = self.log_path("artifact-allowlist")
            self.write_log(allowlist_log, [f"unexpected={path}" for path in unexpected])
            self.record(
                "artifact_allowlist",
                "failed",
                0,
                category=failure.category,
                log_path=allowlist_log,
                detail=failure.message,
            )
            self.emit(
                failure.check,
                "failed",
                category=failure.category,
                detail=failure.message,
            )
            payload = self.result_payload(failure)
            write_json(self.result_path, payload)
            self.write_junit(failure)

        self.write_manifest(failure)
        return failure, payload

    def execute(self) -> int:
        self.prepare_evidence()
        failure: PreflightFailure | None = None
        try:
            self.resolve_identity()
            self.run_change_impact()
            self.run_diff_hygiene()
            self.run_conflict_marker_scan()
            self.run_shell_syntax()
            self.run_rust_format()
        except PreflightFailure as error:
            failure = error
        except Exception as error:  # Defensive finalization for unforeseen runner faults.
            failure = PreflightFailure(
                category="unknown",
                check="runner",
                message=f"unexpected preflight runner failure: {type(error).__name__}",
                pipeline_exit_code=2,
            )
            self.record(
                "runner",
                "failed",
                0,
                category=failure.category,
                detail=failure.message,
            )
        try:
            failure, payload = self.finalize(failure)
        except PreflightFailure as error:
            failure = error
            self.ensure_terminal_failure_record(failure)
            payload = self.result_payload(failure)
            write_json(self.result_path, payload)
            self.write_junit(failure)
            try:
                self.write_manifest(failure)
            except Exception:
                pass
        print(json.dumps(payload, sort_keys=True, ensure_ascii=True))
        return 0 if failure is None else failure.pipeline_exit_code


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Run the exact-SHA App Framework PR preflight. Bitbucket mode uses "
            "destination variables; local mode requires --base."
        )
    )
    parser.add_argument(
        "--base",
        help="Explicit base ref or SHA for local execution (forbidden in Bitbucket PR mode).",
    )
    parser.add_argument(
        "--deadline-seconds",
        type=int,
        default=240,
        help="Runner-wide deadline in seconds (default: 240; maximum: 240).",
    )
    args = parser.parse_args(argv)
    if not 1 <= args.deadline_seconds <= 240:
        parser.error("--deadline-seconds must be between 1 and 240")
    return args


def find_repo_root() -> Path:
    try:
        result = subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            timeout=10,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise SystemExit(f"pr-preflight: unable to resolve Git repository root: {error}")
    return Path(os.fsdecode(result.stdout.rstrip(b"\n"))).resolve()


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    runner = Preflight(find_repo_root(), args.deadline_seconds, args.base)
    return runner.execute()


if __name__ == "__main__":
    raise SystemExit(main())
