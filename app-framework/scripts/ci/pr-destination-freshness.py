#!/usr/bin/env python3
"""Fail closed when a long PR pipeline's destination branch has advanced."""

from __future__ import annotations

import datetime as dt
import json
import os
import re
import subprocess
import sys
from pathlib import Path


SCHEMA = "appfw_pr_destination_freshness@1"
REPORT = Path("target/appfw/pr-destination-freshness.json")
DEFAULT_REMOTE_GIT_TIMEOUT_SECONDS = 45
MAX_REMOTE_GIT_TIMEOUT_SECONDS = 300


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z")


def destination_identity_matches(full_sha: str, supplied_id: str) -> bool:
    """Accept Bitbucket's 12-char abbreviation; require exact full IDs."""

    if len(supplied_id) == 12:
        return full_sha.startswith(supplied_id)
    return full_sha == supplied_id


def remote_git_timeout_seconds(root_causes: list[str]) -> int | None:
    raw = os.environ.get(
        "APPFW_PR_REMOTE_GIT_TIMEOUT_SECONDS",
        str(DEFAULT_REMOTE_GIT_TIMEOUT_SECONDS),
    )
    if not re.fullmatch(r"[0-9]+", raw):
        root_causes.append(
            "APPFW_PR_REMOTE_GIT_TIMEOUT_SECONDS must be an integer from 1 through 300"
        )
        return None
    value = int(raw)
    if value < 1 or value > MAX_REMOTE_GIT_TIMEOUT_SECONDS:
        root_causes.append(
            "APPFW_PR_REMOTE_GIT_TIMEOUT_SECONDS must be an integer from 1 through 300"
        )
        return None
    return value


def main() -> int:
    branch = os.environ.get("BITBUCKET_PR_DESTINATION_BRANCH", "")
    supplied_id = os.environ.get("BITBUCKET_PR_DESTINATION_COMMIT", "").lower()
    root_causes: list[str] = []
    remote_sha: str | None = None
    timeout_seconds = remote_git_timeout_seconds(root_causes)
    remote_query_status = "not_run"
    remote_query_returncode: int | None = None

    if not branch:
        root_causes.append("BITBUCKET_PR_DESTINATION_BRANCH is required")
    elif subprocess.run(
        ["git", "check-ref-format", "--branch", branch],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    ).returncode != 0:
        root_causes.append("BITBUCKET_PR_DESTINATION_BRANCH is invalid")

    if not re.fullmatch(r"[0-9a-f]{12}|[0-9a-f]{40}|[0-9a-f]{64}", supplied_id):
        root_causes.append(
            "BITBUCKET_PR_DESTINATION_COMMIT must be a lowercase 12-, 40-, or "
            "64-character hexadecimal object ID"
        )

    if not root_causes and timeout_seconds is not None:
        environment = os.environ.copy()
        environment["GIT_TERMINAL_PROMPT"] = "0"
        try:
            result = subprocess.run(
                ["git", "ls-remote", "--exit-code", "origin", f"refs/heads/{branch}"],
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
                env=environment,
                timeout=timeout_seconds,
            )
        except subprocess.TimeoutExpired:
            remote_query_status = "timed_out"
            root_causes.append(
                f"remote destination query timed out after {timeout_seconds} seconds"
            )
        except OSError:
            remote_query_status = "start_failed"
            root_causes.append("remote destination query could not start")
        else:
            remote_query_returncode = result.returncode
            if result.returncode != 0:
                remote_query_status = "failed"
                root_causes.append(
                    f"remote destination query failed with status {result.returncode}"
                )
            else:
                remote_query_status = "returned"

    if remote_query_status == "returned":
        fields = result.stdout.strip().split()
        expected_ref = f"refs/heads/{branch}"
        if (
            len(fields) != 2
            or fields[1] != expected_ref
            or not re.fullmatch(r"[0-9a-fA-F]{40}|[0-9a-fA-F]{64}", fields[0])
        ):
            remote_query_status = "invalid_response"
            root_causes.append("remote destination branch identity is unavailable or ambiguous")
        else:
            remote_query_status = "ok"
            remote_sha = fields[0].lower()
            if not destination_identity_matches(remote_sha, supplied_id):
                root_causes.append(
                    "destination branch advanced after the pipeline identity was pinned"
                )

    report = {
        "schema": SCHEMA,
        "command": "pr-destination-freshness",
        "ok": not root_causes,
        "generated_at_utc": utc_now(),
        "destination_branch": branch or None,
        "supplied_destination_id": supplied_id or None,
        "remote_destination_sha": remote_sha,
        "remote_query": {
            "status": remote_query_status,
            "returncode": remote_query_returncode,
            "timeout_seconds": timeout_seconds,
            "terminal_prompt_disabled": True,
        },
        "root_causes": root_causes,
    }
    REPORT.parent.mkdir(parents=True, exist_ok=True)
    REPORT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report, sort_keys=True))
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    sys.exit(main())
