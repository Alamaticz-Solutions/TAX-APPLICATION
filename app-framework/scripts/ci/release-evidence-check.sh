#!/usr/bin/env bash
#
# release-evidence-check.sh - validate retained CI/security evidence JSON.
#
# This is intentionally local and artifact-only: it does not run networked
# scanners or provider services. It verifies that previous CI steps produced
# release/security evidence with the stable fields the release gate relies on.
#
# Usage:
#   bash scripts/ci/release-evidence-check.sh
#   bash scripts/ci/release-evidence-check.sh --local-fixture
#
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
cd "$repo_root"

report_dir="${APPFW_RELEASE_ARTIFACT_DIR:-target/appfw}"
mode="${APPFW_RELEASE_EVIDENCE_MODE:-strict}"

usage() {
  cat <<'USAGE'
Usage:
  bash scripts/ci/release-evidence-check.sh
  bash scripts/ci/release-evidence-check.sh --local-fixture

Modes:
  strict          Validate retained release/security evidence for CI release gates.
  local-fixture   Validate the evidence schema offline with synthetic fallbacks for
                  missing external-tool artifacts. This mode is not release evidence.
USAGE
}

case "${1:-}" in
  "")
    ;;
  --strict)
    mode="strict"
    shift
    ;;
  --local-fixture | --offline-fixture)
    mode="local-fixture"
    shift
    ;;
  -h | --help)
    usage
    exit 0
    ;;
  *)
    echo "release-evidence-check: unknown argument: $1" >&2
    usage >&2
    exit 2
    ;;
esac

if [[ "$#" -ne 0 ]]; then
  echo "release-evidence-check: unexpected extra arguments: $*" >&2
  usage >&2
  exit 2
fi

case "$mode" in
  strict | local-fixture)
    ;;
  *)
    echo "release-evidence-check: unsupported APPFW_RELEASE_EVIDENCE_MODE=$mode" >&2
    exit 2
    ;;
esac

if [[ "$mode" == "local-fixture" ]]; then
  evidence_file="${APPFW_RELEASE_EVIDENCE_OUTPUT:-$report_dir/release-evidence-check-local.json}"
else
  evidence_file="${APPFW_RELEASE_EVIDENCE_OUTPUT:-$report_dir/release-evidence-check.json}"
fi

mkdir -p "$report_dir"

ensure_python3() {
  if command -v python3 >/dev/null 2>&1; then
    return 0
  fi

  if command -v apt-get >/dev/null 2>&1 && [[ "$(id -u)" == "0" ]]; then
    apt-get update
    apt-get install -y --no-install-recommends python3
    return 0
  fi

  echo "release-evidence-check: python3 is required to validate JSON evidence" >&2
  exit 2
}

ensure_python3

python3 - "$report_dir" "$evidence_file" "$mode" <<'PY'
import datetime as dt
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path
from urllib.parse import urlsplit, urlunsplit

report_dir = Path(sys.argv[1])
evidence_file = Path(sys.argv[2])
mode = sys.argv[3]
local_fixture = mode == "local-fixture"
repo_root = Path.cwd()

checks = []
artifacts = []
failures = []
release_blockers = []
synthetic_fixtures = []
PDS_RISK_ACCEPTANCE_MAX_DAYS_LIMIT = 365
RELEASE_CHECK_MAX_AGE_HOURS = 24
RELEASE_CHECK_MAX_AGE = dt.timedelta(hours=RELEASE_CHECK_MAX_AGE_HOURS)
PROVIDER_PREFLIGHT_MAX_AGE_HOURS = 24
PROVIDER_PREFLIGHT_MAX_AGE = dt.timedelta(hours=PROVIDER_PREFLIGHT_MAX_AGE_HOURS)
PROVIDER_PARITY_MAX_AGE_HOURS = 24
PROVIDER_PARITY_MAX_AGE = dt.timedelta(hours=PROVIDER_PARITY_MAX_AGE_HOURS)
SECURITY_CERTIFICATION_MAX_AGE_HOURS = 24
SECURITY_CERTIFICATION_MAX_AGE = dt.timedelta(hours=SECURITY_CERTIFICATION_MAX_AGE_HOURS)
PDS_BASELINE_MAX_AGE_HOURS = 24
PDS_BASELINE_MAX_AGE = dt.timedelta(hours=PDS_BASELINE_MAX_AGE_HOURS)
RELEASE_IDENTITY_MAX_AGE_HOURS = 24
RELEASE_IDENTITY_MAX_AGE = dt.timedelta(hours=RELEASE_IDENTITY_MAX_AGE_HOURS)
SECURITY_ASSURANCE_DECISION_MAX_AGE_HOURS = 24
SECURITY_ASSURANCE_DECISION_MAX_AGE = dt.timedelta(hours=SECURITY_ASSURANCE_DECISION_MAX_AGE_HOURS)
OPS_CERTIFICATION_MAX_AGE_HOURS = 24
OPS_CERTIFICATION_MAX_AGE = dt.timedelta(hours=OPS_CERTIFICATION_MAX_AGE_HOURS)
BITBUCKET_RELEASE_GATE_MAX_AGE_HOURS = 24
BITBUCKET_RELEASE_GATE_MAX_AGE = dt.timedelta(hours=BITBUCKET_RELEASE_GATE_MAX_AGE_HOURS)
AGENT_HANDOFF_MAX_AGE_HOURS = 24
AGENT_HANDOFF_MAX_AGE = dt.timedelta(hours=AGENT_HANDOFF_MAX_AGE_HOURS)
STATIC_EVIDENCE_MAX_AGE_HOURS = 24
STATIC_EVIDENCE_MAX_AGE = dt.timedelta(hours=STATIC_EVIDENCE_MAX_AGE_HOURS)
CHAT_EVAL_MAX_AGE_HOURS = 24
CHAT_EVAL_MAX_AGE = dt.timedelta(hours=CHAT_EVAL_MAX_AGE_HOURS)
EVIDENCE_TIMESTAMP_SKEW = dt.timedelta(minutes=5)


def utc_now():
    return (
        dt.datetime.now(dt.timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z")
    )


def format_failure(name, detail):
    return name if detail is None else f"{name}: {detail}"


def append_unique(items, value):
    if value not in items:
        items.append(value)


def sha256_digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def record(name, ok, artifact=None, detail=None, blocking=True):
    check = {"name": name, "ok": bool(ok)}
    if artifact is not None:
        check["artifact"] = artifact
    if detail is not None:
        check["detail"] = detail
    if not blocking:
        check["blocking"] = False
    checks.append(check)
    if not ok:
        if blocking:
            failures.append(format_failure(name, detail))
        else:
            release_blockers.append(format_failure(name, detail))


def record_artifact(
    name,
    path,
    required=True,
    synthetic_fixture=False,
    required_in_ci=None,
    status=None,
    applicability=None,
    applicability_reason=None,
):
    present = path.is_file()
    if required_in_ci is None:
        required_in_ci = required
    if status is None:
        status = "present" if present else ("missing" if required else "not-applicable")
    artifact = {
        "name": name,
        "path": path.as_posix(),
        "required": required,
        "present": present,
        "status": status,
        "synthetic_fixture": synthetic_fixture,
        "required_in_ci": required_in_ci,
    }
    if applicability is not None:
        artifact["applicability"] = applicability
    if applicability_reason is not None:
        artifact["applicability_reason"] = applicability_reason
    artifacts.append(artifact)
    return present


def retained_fixture_file_artifact(
    name,
    path,
    ok=True,
    work_item_id=None,
    work_item_ids=None,
    required=None,
):
    artifact = {
        "name": name,
        "path": path.as_posix(),
        "present": True,
        "ok": bool(ok),
        "sha256": f"sha256:{sha256_digest(path)}",
        "bytes": path.stat().st_size,
    }
    if work_item_id is not None:
        artifact["work_item_id"] = work_item_id
    if work_item_ids is not None:
        artifact["work_item_ids"] = list(work_item_ids)
    if required is not None:
        artifact["required"] = bool(required)
    return artifact


def write_json_fixture(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return path


def write_synthetic_report_fixture(name, filename):
    path = report_dir / filename
    write_json_fixture(path, synthetic_report(name))
    append_unique(synthetic_fixtures, name)
    return path


def synthetic_report(name):
    generated_at = utc_now()
    if name == "release-check":
        fixture_dir = report_dir / "fixture-artifacts"
        fixture_dir.mkdir(parents=True, exist_ok=True)
        pds_baseline_path = report_dir / "pds-security-baseline.json"
        pds_baseline_log = fixture_dir / "pds-baseline.fixture.log"
        pds_baseline_report = synthetic_report("pds-security-baseline")
        pds_baseline_report["generated_at_utc"] = generated_at
        pds_baseline_path.write_text(
            json.dumps(pds_baseline_report, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        pds_baseline_log.write_text(
            "Synthetic local fixture forces PDS baseline schema validation.\n",
            encoding="utf-8",
        )
        release_identity_path = report_dir / "release-identity.json"
        release_identity_log = fixture_dir / "release-identity.fixture.log"
        release_identity_report = synthetic_report("release-identity")
        release_identity_report["generated_at_utc"] = generated_at
        release_identity_path.write_text(
            json.dumps(release_identity_report, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        release_identity_log.write_text(
            "Synthetic local fixture forces release identity schema validation.\n",
            encoding="utf-8",
        )
        append_unique(synthetic_fixtures, "pds-security-baseline")
        append_unique(synthetic_fixtures, "release-identity")
        return {
            "command": "release-check",
            "ok": False,
            "release_ready": False,
            "generated_at_utc": generated_at,
            "incomplete": False,
            "current_check": "pds-baseline",
            "checks": [
                {
                    "name": "local-fixture:schema-only",
                    "ok": True,
                    "artifact": None,
                },
                {
                    "name": "pds-baseline",
                    "ok": False,
                    "artifact": pds_baseline_path.as_posix(),
                    "log": pds_baseline_log.as_posix(),
                    "detail": "synthetic local fixture forces PDS baseline schema validation",
                },
                {
                    "name": "release-identity",
                    "ok": False,
                    "artifact": release_identity_path.as_posix(),
                    "log": release_identity_log.as_posix(),
                    "detail": "synthetic local fixture forces release identity schema validation",
                }
            ],
            "evidence_mode": {
                "provider_certification": "live-required",
                "security_certification": "live-provider-evidence-required",
                "operations": "static-local-allowed",
                "pds_security_baseline": "traceability-local-allowed",
                "performance": "not-required",
                "production_attestations_required": False,
                "security_assurance": "not-required",
            },
            "provider_scope": {
                "release_certified_crud": ["postgres", "mongo", "mssql", "snowflake"],
                "graph_providers": [
                    {
                        "provider": "neo4j",
                        "workspace_crate": "appfw-provider-neo4j",
                        "release_certified": False,
                        "posture": "graph-read and governed-write foundation; outside CRUD semantic parity",
                        "certification_path": "docs/runtime/graph-read-providers.md#certification-posture",
                    }
                ],
                "external_api_providers": [
                    {
                        "provider": provider,
                        "workspace_crate": workspace_crate,
                        "release_certified": False,
                        "governed_write_certified": False,
                        "posture": (
                            "external API named-read foundation; governed writes disabled "
                            "until G1 delegated-auth evidence exists"
                        ),
                        "certification_path": "docs/runtime/saas-certification.md#governed-write-posture",
                    }
                    for provider, workspace_crate in [
                        ("servicenow", "appfw-provider-servicenow"),
                        ("workday", "appfw-provider-workday"),
                        ("icims", "appfw-provider-icims"),
                        ("salesforce", "appfw-provider-salesforce"),
                        ("anaplan", "appfw-provider-anaplan"),
                        ("oracle_financials", "appfw-provider-oracle-financials"),
                    ]
                ],
            },
            "missing_provider_base_urls": [],
            "invalid_provider_base_urls": [],
            "failure_summary": {
                "root_causes": [
                    "synthetic local release-check fixture is not release evidence",
                ],
            },
            "remediation_work_items": [],
        }
    if name == "phi-log-lint":
        return {
            "command": "phi-log-lint",
            "ok": True,
            "generated_at_utc": generated_at,
            "scanned_files": 0,
            "findings": 0,
            "failure_reason": None,
            "allow_marker": "appfw-allow-phi-log",
            "redaction": {
                "json_source_excerpt_retained": False,
                "json_finding_values_retained": False,
            },
        }
    if name == "supply-chain-gate":
        dependency_path = write_synthetic_report_fixture("dependency-check", "dependency-check.json")
        phi_path = write_synthetic_report_fixture("phi-log-lint", "phi-log-lint.json")
        rust_sbom_path = write_synthetic_report_fixture("rust-cyclonedx-sbom", "sbom-rust-workspace.cdx.json")
        frontend_sbom_path = write_synthetic_report_fixture(
            "frontend-cyclonedx-sbom",
            "sbom-frontend-packages.cdx.json",
        )
        sbom_manifest_path = write_synthetic_report_fixture("sbom-manifest", "sbom-manifest.json")
        return {
            "command": "supply-chain-gate",
            "ok": True,
            "generated_at_utc": generated_at,
            "checks": [
                {
                    "name": "local-fixture:schema-only",
                    "exit_code": 0,
                    "ok": True,
                }
            ],
            "artifacts": [
                retained_fixture_file_artifact("dependency-check", dependency_path, required=True),
                retained_fixture_file_artifact("phi-log-lint", phi_path, required=True),
                retained_fixture_file_artifact("sbom-manifest", sbom_manifest_path, required=True),
                retained_fixture_file_artifact("rust-cyclonedx-sbom", rust_sbom_path, required=True),
                retained_fixture_file_artifact("frontend-cyclonedx-sbom", frontend_sbom_path, required=True),
                {
                    "name": "deployable-image-cyclonedx-sbom",
                    "path": (report_dir / "sbom-deployable-image.cdx.json").as_posix(),
                    "required": False,
                    "present": False,
                    "status": "not-applicable",
                    "applicability": "not-applicable",
                    "applicability_reason": (
                        "not-applicable: no deployable image source env var or retained "
                        "image tar/OCI artifact was found"
                    ),
                }
            ],
        }
    if name == "dependency-check":
        return {
            "command": "dependency-check",
            "ok": True,
            "generated_at_utc": generated_at,
            "mode": {
                "online": True,
                "strict": True,
                "offline": False,
            },
            "summary": {
                "cargo_manifest_count": 1,
                "cargo_locked_package_count": 1,
                "rust_direct_dependency_count": 1,
                "duplicate_version_group_count": 0,
                "duplicate_compatibility_line_count": 0,
                "npm_lockfile_count": 0,
                "upgrade_advice_count": 0,
                "accepted_osv_finding_count": 0,
                "blocking_finding_count": 0,
            },
            "policy": {
                "dependency_policy_file": "dependency-check.toml",
                "dependency_policy_status": "configured",
                "osv_acceptance_count": 0,
            },
            "checks": [
                {
                    "name": "local-fixture:schema-only",
                    "exit_code": 0,
                    "ok": True,
                    "required": True,
                    "status": "passed",
                }
            ],
            "rust": {
                "upgrade_advice": [],
                "duplicate_versions": [],
            },
            "node": {
                "lockfiles": [],
            },
            "osv": {
                "status": "queried",
                "finding_count": 0,
                "blocking_finding_count": 0,
                "accepted_finding_count": 0,
                "accepted_findings": [],
                "expired_acceptances": [],
                "unused_acceptances": [],
            },
            "warnings": [],
            "blocking_findings": [],
        }
    if name == "sbom-manifest":
        rust_sbom_path = report_dir / "sbom-rust-workspace.cdx.json"
        frontend_sbom_path = report_dir / "sbom-frontend-packages.cdx.json"
        frontend_sources = frontend_lockfiles()
        rust_artifact = retained_fixture_file_artifact("rust-cyclonedx-sbom", rust_sbom_path, required=True)
        rust_artifact.update(
            {
                "format": "CycloneDX",
                "spec_version": "1.5",
                "ecosystem": "rust",
                "component_count": 1,
                "sources": ["Cargo.lock", "cargo metadata --locked --no-deps"],
            }
        )
        frontend_artifact = retained_fixture_file_artifact(
            "frontend-cyclonedx-sbom",
            frontend_sbom_path,
            required=True,
        )
        frontend_artifact.update(
            {
                "format": "CycloneDX",
                "spec_version": "1.5",
                "ecosystem": "npm",
                "component_count": 1,
                "sources": frontend_sources,
            }
        )
        return {
            "command": "generate-sbom-evidence",
            "ok": True,
            "generated_at_utc": generated_at,
            "artifacts": [
                rust_artifact,
                frontend_artifact,
                {
                    "name": "deployable-image-cyclonedx-sbom",
                    "path": (report_dir / "sbom-deployable-image.cdx.json").as_posix(),
                    "format": "CycloneDX",
                    "spec_version": None,
                    "ecosystem": "container-image",
                    "required": False,
                    "present": False,
                    "status": "not-applicable",
                    "applicability": "not-applicable",
                    "applicability_reason": (
                        "not-applicable: no deployable image source env var or retained "
                        "image tar/OCI artifact was found"
                    ),
                    "component_count": None,
                    "sources": [],
                    "scanner": None,
                },
            ],
            "inputs": {
                "frontend_lockfiles": frontend_sources,
                "image_sources": [],
                "image_applicability": "not-applicable",
                "image_required_by_release_mode": False,
            },
        }
    if name in (
        "rust-cyclonedx-sbom",
        "frontend-cyclonedx-sbom",
        "deployable-image-cyclonedx-sbom",
    ):
        component_name = name.removesuffix("-cyclonedx-sbom")
        return {
            "bomFormat": "CycloneDX",
            "specVersion": "1.5",
            "serialNumber": "urn:uuid:00000000-0000-0000-0000-000000000000",
            "version": 1,
            "metadata": {
                "timestamp": generated_at,
                "component": {
                    "type": "application",
                    "name": component_name,
                    "bom-ref": component_name,
                },
            },
            "components": [
                {
                    "type": "library",
                    "name": f"local-fixture-{component_name}",
                    "version": "0.0.0",
                    "bom-ref": f"local-fixture-{component_name}",
                }
            ],
        }
    if name == "secret-scan":
        gitleaks_report_path = write_synthetic_report_fixture("gitleaks-report", "gitleaks-report.json")
        return {
            "command": "secret-scan",
            "ok": True,
            "generated_at_utc": generated_at,
            "scanner": "gitleaks",
            "scanner_version": "local-fixture",
            "requested_scanner_version": "local-fixture",
            "gitleaks_exit_code": 0,
            "findings": 0,
            "total_findings": 0,
            "baselined_findings": 0,
            "history_scan": True,
            "baseline": {
                "path": (report_dir / "gitleaks-baseline.txt").as_posix(),
                "present": False,
                "entries": 0,
                "unbaselined_findings": 0,
            },
            "artifacts": [
                retained_fixture_file_artifact("gitleaks-report", gitleaks_report_path, required=True),
                {
                    "name": "gitleaks-baseline",
                    "path": (report_dir / "gitleaks-baseline.txt").as_posix(),
                    "required": False,
                    "present": False,
                }
            ],
            "redaction": {
                "gitleaks_redact_flag": True,
                "secret_fields_checked": True,
                "secret_fields_redacted": True,
            },
        }
    if name == "gitleaks-report":
        return []
    if name == "security-assurance-decision":
        return {
            "command": "security-assurance-decision",
            "ok": True,
            "release_ready": True,
            "generated_at_utc": generated_at,
            "requirements": [
                "dast",
                "sast",
                "asvs",
                "release-provenance",
                "artifact-signing",
            ],
            "categories": [
                {
                    "category": category,
                    "ok": True,
                    "disposition": "local-fixture",
                    "evidence_artifacts": [],
                }
                for category in [
                    "dast",
                    "sast",
                    "asvs",
                    "release-provenance",
                    "artifact-signing",
                ]
            ],
            "risk_acceptance": {
                "present": False,
                "valid": False,
                "detail": "synthetic local fixture; CI requires the real artifact",
            },
            "artifacts": [],
            "failure_count": 0,
            "failures": [],
        }
    if name == "governed-write-evidence":
        return {
            "command": "provider-test",
            "lane": "G1",
            "ok": True,
            "provider": "servicenow",
            "operation": "named_mutation",
            "delegated_actor_context": {
                "principal_type": "user",
                "tenant": "local-fixture-tenant",
                "on_behalf_of": "local-fixture-user",
            },
            "token_store_isolation": {
                "partition_key": ["user", "tenant", "provider"],
                "revocation_checked": True,
            },
            "mutation_registry": {
                "name": "servicenow.local_fixture_mutation",
                "mcp_enabled": False,
                "policy_scope": "servicenow.fixture.write",
            },
            "idempotency": {
                "key_source": "request",
                "replay_rejected": True,
            },
            "audit": {
                "source": "http",
                "correlation_id": "required",
                "provider_request_id": "redacted-or-hash",
            },
        }
    if name == "security-certification":
        expected_contracts = [
            {
                "name": "tenant_isolation",
                "contract": "provider_semantic_contracts::provider_tenant_isolation_contract",
                "expectation": "tenant-scoped reads do not expose cross-tenant records",
                "area": "tenant_isolation",
            },
            {
                "name": "locator_idor_negative",
                "contract": "provider_semantic_contracts::provider_locator_tenant_isolation_contract",
                "expectation": "record locator reads do not become cross-tenant IDOR",
                "area": None,
            },
            {
                "name": "access_filter_denied_mutation",
                "contract": "provider_semantic_contracts::provider_access_filter_policy_cannot_widen_user_filter_contract",
                "expectation": "policy filters cannot be widened and denied mutations preserve records",
                "area": "access_filters",
            },
            {
                "name": "denied_error_normalization",
                "contract": "provider_semantic_contracts::provider_error_normalization_contract",
                "expectation": "denied access and invalid requests normalize to GraphQL errors",
                "area": "error_normalization",
            },
            {
                "name": "audit_denied_redaction_chain",
                "contract": "provider_semantic_contracts::provider_audit_append_redaction_chain_contract",
                "expectation": "denied mutation attempts are audited with redaction and chain continuity",
                "area": "audit",
            },
        ]
        security_fixture_dir = report_dir / "fixture-artifacts" / "security-certification"
        security_fixture_dir.mkdir(parents=True, exist_ok=True)
        provider_logs = {}
        for provider in ["postgres", "mongo", "mssql", "snowflake"]:
            provider_log = security_fixture_dir / f"provider-test-{provider}.fixture.log"
            provider_log.write_text(
                f"local fixture provider security certification log for {provider}\n",
                encoding="utf-8",
            )
            provider_logs[provider] = provider_log
        introspection_log = security_fixture_dir / "security-certification-introspection.fixture.log"
        introspection_log.write_text(
            "local fixture GraphQL introspection runtime checks passed\n",
            encoding="utf-8",
        )
        provider_parity_path = report_dir / "provider-parity.json"
        provider_parity_report = {
            "command": "provider-test",
            "ok": True,
            "generated_at_utc": generated_at,
            "providers": [
                {
                    "provider": provider,
                    "ok": True,
                    "mode": "full",
                    "base_url": f"http://127.0.0.1:{18080 + index}",
                    "log": provider_logs[provider].as_posix(),
                    "areas": [
                        {
                            "area": "tenant_isolation",
                            "status": "live-certified",
                            "ok": True,
                            "live_result": "passed",
                            "required_live_result": "passed",
                            "live_contracts": [
                                "provider_semantic_contracts::provider_tenant_isolation_contract"
                            ],
                        }
                    ],
                }
                for index, provider in enumerate(["postgres", "mongo", "mssql", "snowflake"])
            ],
        }
        provider_parity_path.write_text(
            json.dumps(provider_parity_report, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        routing_source = repo_root / "appfw_runtime" / "src" / "routing.rs"
        return {
            "command": "security-certification-evidence",
            "ok": True,
            "release_ready": True,
            "generated_at_utc": generated_at,
            "checks": [
                {"name": "local-fixture:schema-only", "ok": True},
            ],
            "artifacts": [
                retained_fixture_file_artifact(
                    "runtime-routing-source",
                    routing_source,
                ),
                retained_fixture_file_artifact(
                    "graphql-introspection-runtime-test-log",
                    introspection_log,
                ),
                retained_fixture_file_artifact(
                    "provider-parity",
                    provider_parity_path,
                ),
            ]
            + [
                retained_fixture_file_artifact(
                    f"provider-log:{provider}",
                    provider_logs[provider],
                )
                for provider in ["postgres", "mongo", "mssql", "snowflake"]
            ],
            "introspection_auth": {
                "evidence_type": "focused runtime unit regression",
                "source": "appfw_runtime/src/routing.rs",
                "runtime_log": introspection_log.as_posix(),
                "cases": [
                    {
                        "case": "no_auth_disabled_or_missing_user",
                        "test": "graphql_introspection_rejects_no_auth_when_disabled_or_missing_user",
                        "runtime_result": "passed",
                        "source_present": True,
                        "ok": True,
                    },
                    {
                        "case": "fake_bearer_or_unprivileged_user",
                        "test": "graphql_introspection_rejects_fake_bearer_or_unprivileged_user",
                        "runtime_result": "passed",
                        "source_present": True,
                        "ok": True,
                    },
                    {
                        "case": "admin_or_developer_scope",
                        "test": "graphql_introspection_allows_admin_or_developer_scope",
                        "runtime_result": "passed",
                        "source_present": True,
                        "ok": True,
                    },
                ],
            },
            "live_security_contracts": {
                "provider_parity": provider_parity_path.as_posix(),
                "expected_providers": ["postgres", "mongo", "mssql", "snowflake"],
                "expected_contracts": expected_contracts,
                "providers": [
                    {
                        "provider": provider,
                        "ok": True,
                        "log": provider_logs[provider].as_posix(),
                        "contracts": [
                            {
                                **contract,
                                "result": "passed",
                                "area_result": "passed" if contract["area"] else None,
                                "ok": True,
                            }
                            for contract in expected_contracts
                        ],
                    }
                    for provider in ["postgres", "mongo", "mssql", "snowflake"]
                ],
            },
            "failure_count": 0,
            "failures": [],
            "release_blockers": [],
        }
    if name == "pds-security-baseline":
        pds_fixture_dir = report_dir / "fixture-artifacts" / "pds-baseline"
        pds_fixture_dir.mkdir(parents=True, exist_ok=True)
        pds_work_items = [
            ("LIVE-015", "release-authority"),
            ("LIVE-016", "iam-okta-mfa-lifecycle"),
            ("LIVE-017", "secrets-management"),
            ("LIVE-018", "api-gateway-waf-tls-scanning"),
            ("LIVE-019", "lower-env-backup-dr"),
            ("LIVE-020", "platform-network-db-hardening"),
            ("LIVE-021", "siem-monitoring-access-review"),
        ]
        evidence_files = []
        evidence_artifacts = []
        decision_work_items = []
        for work_item_id, slug in pds_work_items:
            evidence_path = pds_fixture_dir / f"{work_item_id.lower()}-{slug}.fixture.json"
            evidence_payload = {
                "fixture": True,
                "ok": False,
                "release_ready": False,
                "work_item_id": work_item_id,
                "evidence_type": "local schema fixture",
                "notes": [
                    "Synthetic local fixture used only to exercise PDS retained evidence validation.",
                    "This file is not live PDS baseline evidence.",
                ],
            }
            evidence_path.write_text(json.dumps(evidence_payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
            evidence_files.append(evidence_path.as_posix())
            evidence_artifacts.append(
                retained_fixture_file_artifact(
                    f"PDS {work_item_id} evidence",
                    evidence_path,
                    work_item_id=work_item_id,
                    work_item_ids=[work_item_id],
                )
            )
            decision_work_items.append(
                {
                    "id": work_item_id,
                    "status": "evidence",
                    "evidence_files": [evidence_path.as_posix()],
                    "summary": f"Synthetic retained evidence fixture for {work_item_id}",
                }
            )
        decision_path = report_dir / "fixture-artifacts" / "pds-baseline-decision.fixture.json"
        decision_payload = {
            "fixture": True,
            "ok": False,
            "release_ready": False,
            "baseline": {
                "version": "r4.5",
            },
            "release_authority": {
                "owner": "local-fixture",
                "approver": "local-fixture",
                "approved_at_utc": generated_at,
            },
            "work_items": decision_work_items,
            "notes": [
                "Synthetic local fixture used only to exercise retained PDS decision and evidence validation.",
                "This file is not release-authority approval.",
            ],
        }
        decision_path.write_text(json.dumps(decision_payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        return {
            "command": "pds-baseline",
            "ok": True,
            "release_ready": False,
            "generated_at_utc": generated_at,
            "baseline": {
                "name": "PDS Health IT Security Baseline Standard",
                "version": "r4.5",
                "traceability_doc": "docs/release/pds-security-baseline-traceability.md",
            },
            "live_evidence_required": False,
            "decision_file": decision_path.as_posix(),
            "decision_artifact": retained_fixture_file_artifact(
                "PDS baseline decision",
                decision_path,
                ok=False,
            ),
            "decision_summary": {
                "covered_work_items": [work_item_id for work_item_id, _ in pds_work_items],
                "evidence_backed_work_items": [work_item_id for work_item_id, _ in pds_work_items],
                "risk_accepted_work_items": [],
            },
            "risk_acceptance": {
                "max_days": 365,
            },
            "evidence_files": evidence_files,
            "evidence_artifacts": evidence_artifacts,
            "checks": [
                {
                    "name": "local-fixture:pds-baseline-schema",
                    "ok": True,
                    "blocking": False,
                    "detail": "synthetic local fixture; CI requires real PDS baseline evidence",
                },
                {
                    "name": "local-fixture:pds-decision-retained",
                    "ok": True,
                    "blocking": False,
                    "detail": f"synthetic PDS decision fixture retained under {report_dir.as_posix()}",
                },
                {
                    "name": "local-fixture:pds-evidence-retained",
                    "ok": True,
                    "blocking": False,
                    "detail": "synthetic PDS evidence fixtures retained for LIVE-015 through LIVE-021",
                }
            ],
            "failure_count": 0,
            "failures": [],
            "release_blockers": [
                "synthetic local PDS baseline fixture is not release evidence",
            ],
            "failure_summary": {
                "root_causes": [
                    "synthetic local PDS baseline fixture is not release evidence",
                ],
            },
            "remediation_work_items": [
                {
                    "id": "LIVE-015",
                    "priority": "P0",
                    "owner": "Release authority + security",
                    "summary": "Retain PDS Security Baseline r4.5 traceability for production-readiness claims.",
                    "doc": "docs/release/live-environment-work-items.md#p0-release-blockers",
                },
                {
                    "id": "LIVE-016",
                    "priority": "P0",
                    "owner": "IAM + security + platform",
                    "summary": "Prove PDS IAM, Okta, MFA, and user-lifecycle posture.",
                    "doc": "docs/release/live-environment-work-items.md#p0-release-blockers",
                },
                {
                    "id": "LIVE-017",
                    "priority": "P0",
                    "owner": "Security + platform",
                    "summary": "Prove PDS secrets-management posture.",
                    "doc": "docs/release/live-environment-work-items.md#p0-release-blockers",
                },
                {
                    "id": "LIVE-018",
                    "priority": "P0",
                    "owner": "Network + platform + security",
                    "summary": "Prove PDS internet-facing API and gateway posture.",
                    "doc": "docs/release/live-environment-work-items.md#p0-release-blockers",
                },
                {
                    "id": "LIVE-019",
                    "priority": "P1",
                    "owner": "SRE/platform + data owners + security",
                    "summary": "Prove lower-environment data, backup, restore, and DR posture.",
                    "doc": "docs/release/live-environment-work-items.md#p1-enterprise-evidence",
                },
                {
                    "id": "LIVE-020",
                    "priority": "P1",
                    "owner": "Platform + network + database operations",
                    "summary": "Prove host, cloud, network, and database hardening posture.",
                    "doc": "docs/release/live-environment-work-items.md#p1-enterprise-evidence",
                },
                {
                    "id": "LIVE-021",
                    "priority": "P1",
                    "owner": "Security operations + SRE + IAM",
                    "summary": "Prove PDS SIEM, monitoring, and access-review posture.",
                    "doc": "docs/release/live-environment-work-items.md#p1-enterprise-evidence",
                },
            ],
        }
    if name == "release-identity":
        decision_path = report_dir / "fixture-artifacts" / "release-identity-decision.fixture.json"
        decision_path.parent.mkdir(parents=True, exist_ok=True)
        decision_fixture = {
            "fixture": True,
            "ok": False,
            "release_ready": False,
            "release_authority": {
                "owner": "local-fixture",
                "approver": "local-fixture",
                "approved_at_utc": generated_at,
            },
            "release": {
                "git_tag": "v0.0.0-local-fixture",
                "release_notes": "local-fixture-only",
                "distribution_artifacts": [],
            },
            "notes": [
                "Synthetic local fixture used only to exercise retained decision artifact validation.",
                "This file is not release evidence and must not be used as release approval.",
            ],
        }
        decision_path.write_text(json.dumps(decision_fixture, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        decision_sha = f"sha256:{sha256_digest(decision_path)}"
        decision_bytes = decision_path.stat().st_size
        return {
            "command": "release-identity",
            "ok": False,
            "release_ready": False,
            "generated_at_utc": generated_at,
            "release_identity_required": False,
            "git": {
                "sha": git_output(["rev-parse", "HEAD"]),
                "bitbucket_tag": None,
                "exact_v_tags": [],
                "release_tag": None,
            },
            "decision_file": decision_path.as_posix(),
            "decision_artifact": {
                "path": decision_path.as_posix(),
                "present": True,
                "ok": False,
                "sha256": decision_sha,
                "bytes": decision_bytes,
            },
            "license_files": [],
            "package_metadata": [],
            "checks": [
                {
                    "name": "release-identity-required",
                    "ok": False,
                    "blocking": True,
                    "detail": "synthetic local fixture; CI requires real release identity evidence",
                },
                {
                    "name": "v-tag",
                    "ok": False,
                    "blocking": True,
                    "detail": "synthetic local fixture is not associated with a v* release tag",
                },
                {
                    "name": "license-or-notice-file",
                    "ok": False,
                    "blocking": True,
                    "detail": "synthetic local fixture has no license file",
                },
                {
                    "name": "package-license-metadata",
                    "ok": False,
                    "blocking": True,
                    "detail": "synthetic local fixture has no package license metadata",
                },
                {
                    "name": "decision-artifact",
                    "ok": False,
                    "blocking": True,
                    "detail": "synthetic local fixture decision is retained but is not release approval",
                },
                {
                    "name": "decision-artifact-retained",
                    "ok": True,
                    "blocking": True,
                    "detail": f"synthetic local fixture decision is retained under {report_dir.as_posix()}",
                },
                {
                    "name": "release-authority",
                    "ok": False,
                    "blocking": True,
                    "detail": "synthetic local fixture has no release authority approval",
                },
                {
                    "name": "decision-tag-match",
                    "ok": False,
                    "blocking": True,
                    "detail": "synthetic local fixture has no matching release tag",
                },
                {
                    "name": "decision-license",
                    "ok": False,
                    "blocking": True,
                    "detail": "synthetic local fixture has no retained license reference",
                },
                {
                    "name": "release-notes",
                    "ok": False,
                    "blocking": True,
                    "detail": "synthetic local fixture has no release notes",
                },
                {
                    "name": "distribution-artifacts",
                    "ok": False,
                    "blocking": True,
                    "detail": "synthetic local fixture has no distribution artifacts",
                },
            ],
            "release_blockers": [
                "synthetic local release identity fixture is not release evidence",
            ],
            "failure_summary": {
                "root_causes": [
                    "synthetic local release identity fixture is not release evidence",
                ],
            },
            "remediation_work_items": [
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
            ],
        }
    raise ValueError(f"unknown synthetic report: {name}")


def load_required(
    name,
    filename,
    expected_type,
    missing_detail=None,
    applicability=None,
    applicability_reason=None,
):
    path = report_dir / filename
    artifact_path = path.as_posix()
    if not record_artifact(
        name,
        path,
        applicability=applicability,
        applicability_reason=applicability_reason,
    ):
        if local_fixture:
            artifacts[-1]["synthetic_fixture"] = True
            synthetic_fixtures.append(name)
            record(
                f"{name} artifact present",
                False,
                artifact_path,
                missing_detail or "using synthetic local fixture; CI requires the real artifact",
                blocking=False,
            )
            return synthetic_report(name)
        record(f"{name} artifact present", False, artifact_path, missing_detail)
        return None
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except Exception as exc:  # pragma: no cover - exercised by shell usage.
        record(f"{name} parses as JSON", False, artifact_path, str(exc))
        return None
    record(f"{name} parses as JSON", True, artifact_path)
    if not isinstance(value, expected_type):
        record(
            f"{name} has expected JSON type",
            False,
            artifact_path,
            f"expected {expected_type.__name__}",
        )
        return None
    record(f"{name} has expected JSON type", True, artifact_path)
    return value


def require(condition, name, artifact, detail=None, blocking=True):
    record(name, bool(condition), artifact, detail, blocking=blocking)


def first_non_empty(mapping, keys):
    if not isinstance(mapping, dict):
        return ""
    for key in keys:
        value = mapping.get(key)
        if isinstance(value, str) and value.strip():
            return value.strip()
    return ""


def nested_mapping(mapping, keys):
    if not isinstance(mapping, dict):
        return {}
    for key in keys:
        value = mapping.get(key)
        if isinstance(value, dict):
            return value
    return {}


class DuplicateJsonKeyError(ValueError):
    pass


def reject_duplicate_json_keys(pairs):
    value = {}
    for key, child in pairs:
        if key in value:
            raise DuplicateJsonKeyError(f"duplicate object key {key!r}")
        value[key] = child
    return value


def reject_non_json_constant(value):
    raise ValueError(f"non-standard JSON constant {value!r}")


def strict_json_loads(text):
    return json.loads(
        text,
        object_pairs_hook=reject_duplicate_json_keys,
        parse_constant=reject_non_json_constant,
    )


def invalid_contained_readiness_fields(value):
    matches = []
    pending = [("$", value)]
    while pending:
        current_path, current = pending.pop()
        if isinstance(current, dict):
            for key, child in current.items():
                child_path = f"{current_path}.{key}"
                if key in {"candidate_ready", "release_ready"} and child is not False:
                    matches.append(
                        f"{child_path}={json.dumps(child, sort_keys=True)[:160]} "
                        f"({type(child).__name__})"
                    )
                pending.append((child_path, child))
        elif isinstance(current, list):
            for index, child in enumerate(current):
                pending.append((f"{current_path}[{index}]", child))
    return sorted(matches)


def baseline_version_from_decision(decision):
    if not isinstance(decision, dict):
        return ""
    baseline = decision.get("baseline")
    if isinstance(baseline, dict) and isinstance(baseline.get("version"), str):
        return baseline.get("version", "").strip()
    if isinstance(decision.get("baseline_version"), str):
        return decision.get("baseline_version", "").strip()
    return ""


def parse_timestamp_with_timezone(value):
    if not isinstance(value, str) or not value.strip():
        return None
    try:
        parsed = dt.datetime.fromisoformat(value.strip().replace("Z", "+00:00"))
    except ValueError:
        return None
    if parsed.tzinfo is None:
        return None
    return parsed.astimezone(dt.timezone.utc)


def validate_generated_timestamp(
    report_name,
    report,
    artifact_path,
    max_age,
    max_age_hours,
    blocking,
):
    generated_at = parse_timestamp_with_timezone(report.get("generated_at_utc"))
    require(
        generated_at is not None,
        f"{report_name} generated timestamp is valid",
        artifact_path,
        "generated_at_utc must be ISO-8601 with timezone",
        blocking=blocking,
    )
    if generated_at is None:
        return None

    now = dt.datetime.now(dt.timezone.utc)
    require(
        generated_at <= now + EVIDENCE_TIMESTAMP_SKEW,
        f"{report_name} generated timestamp is not in the future",
        artifact_path,
        generated_at.isoformat().replace("+00:00", "Z"),
        blocking=blocking,
    )
    require(
        generated_at >= now - max_age,
        f"{report_name} generated timestamp is recent",
        artifact_path,
        (
            f"generated_at_utc {generated_at.isoformat().replace('+00:00', 'Z')} "
            f"is older than {max_age_hours} hours"
        ),
        blocking=blocking,
    )
    return generated_at


def validate_release_check_artifact_timestamp(
    check_label,
    resolved_artifact,
    check_path,
    release_check_generated_at,
    blocking,
):
    if release_check_generated_at is None:
        return
    try:
        artifact_value = json.loads(resolved_artifact.read_text(encoding="utf-8"))
    except Exception:
        return
    if not isinstance(artifact_value, dict) or "generated_at_utc" not in artifact_value:
        return

    artifact_generated_at = parse_timestamp_with_timezone(artifact_value.get("generated_at_utc"))
    require(
        artifact_generated_at is not None,
        f"release-check check {check_label} artifact timestamp is valid",
        check_path,
        "generated_at_utc must be ISO-8601 with timezone",
        blocking=blocking,
    )
    if artifact_generated_at is None:
        return

    require(
        artifact_generated_at <= release_check_generated_at + EVIDENCE_TIMESTAMP_SKEW,
        f"release-check check {check_label} artifact timestamp does not postdate release-check",
        check_path,
        (
            f"artifact generated_at_utc {artifact_generated_at.isoformat().replace('+00:00', 'Z')}; "
            f"release-check generated_at_utc "
            f"{release_check_generated_at.isoformat().replace('+00:00', 'Z')}"
        ),
        blocking=blocking,
    )


def parse_expiry_date(value):
    if not isinstance(value, str) or not value.strip():
        return None
    text = value.strip()
    try:
        if "T" in text:
            parsed = dt.datetime.fromisoformat(text.replace("Z", "+00:00"))
            if parsed.tzinfo is None:
                parsed = parsed.replace(tzinfo=dt.timezone.utc)
            return parsed.astimezone(dt.timezone.utc).date()
        return dt.date.fromisoformat(text[:10])
    except ValueError:
        return None


def as_list(value):
    if value is None:
        return []
    if isinstance(value, list):
        return value
    return [value]


def resolve_retained_path(path_text):
    if not isinstance(path_text, str) or not path_text.strip():
        return None
    candidate = Path(path_text.strip()).expanduser()
    if candidate.is_absolute():
        return candidate if candidate.is_file() else None
    for base in (repo_root, report_dir):
        resolved = base / candidate
        if resolved.is_file():
            return resolved
    return None


def path_is_within_directory(path, directory):
    try:
        path.resolve(strict=False).relative_to(directory.resolve(strict=False))
    except ValueError:
        return False
    return True


def paths_match(left, right):
    if left is None or right is None:
        return False
    return Path(left).resolve(strict=False) == Path(right).resolve(strict=False)


def resolve_pds_decision_evidence_path(path_text, decision_path):
    if not isinstance(path_text, str) or not path_text.strip():
        return None
    candidate = Path(path_text.strip()).expanduser()
    if candidate.is_absolute():
        return candidate if candidate.is_file() else None
    for base in (report_dir, decision_path.parent, repo_root):
        resolved = base / candidate
        if resolved.is_file():
            return resolved
    return None


def collect_pds_decision_entries(decision):
    if not isinstance(decision, dict):
        return []
    entries = []
    for key in ("work_items", "workItems", "categories"):
        collection = decision.get(key)
        if isinstance(collection, list):
            entries.extend(entry for entry in collection if isinstance(entry, dict))
        elif isinstance(collection, dict):
            for entry_id, entry_value in collection.items():
                if isinstance(entry_value, dict):
                    entry = dict(entry_value)
                    entry.setdefault("id", entry_id)
                    entries.append(entry)
    return entries


def collect_pds_work_item_ids(entry):
    values = []
    for key in ("id", "work_item_id", "workItemId"):
        values.extend(as_list(entry.get(key)))
    for key in ("ids", "work_item_ids", "workItemIds"):
        values.extend(as_list(entry.get(key)))
    return {
        str(value).strip().upper()
        for value in values
        if isinstance(value, str) and value.strip()
    }


def collect_pds_evidence_paths(entry):
    paths = []
    for key in ("evidence_files", "artifact_files", "artifacts", "evidence"):
        for value in as_list(entry.get(key)):
            if isinstance(value, dict):
                path = first_non_empty(value, ("path", "file", "artifact", "uri"))
                if path:
                    paths.append(path)
            elif isinstance(value, str) and value.strip():
                paths.append(value.strip())
    return paths


def validate_pds_risk_acceptance(entry, work_item_id, artifact_path, max_days, blocking):
    risk = nested_mapping(entry, ("risk_acceptance", "riskAcceptance", "exception", "risk"))
    ok = True
    for label, keys in (
        ("owner", ("owner", "risk_owner")),
        ("approver", ("approver", "approved_by")),
        ("scope", ("scope", "affected_scope", "impact_scope")),
        ("expiry", ("expires_on", "expires_at", "expiration_date", "expiry")),
        ("rationale", ("rationale", "justification")),
        ("compensating controls", ("compensating_controls", "compensatingControls")),
        ("follow-up", ("follow_up", "follow_up_work_item", "followUp")),
    ):
        value = first_non_empty(risk, keys)
        require(
            bool(value),
            f"PDS {work_item_id} risk acceptance has {label}",
            artifact_path,
            value or f"PDS baseline decision risk acceptance for {work_item_id} must include {label}",
            blocking=blocking,
        )
        ok = ok and bool(value)

    expiry = first_non_empty(risk, ("expires_on", "expires_at", "expiration_date", "expiry"))
    parsed_expiry = parse_expiry_date(expiry)
    require(
        parsed_expiry is not None,
        f"PDS {work_item_id} risk acceptance expiry is valid",
        artifact_path,
        expiry or f"PDS baseline decision risk acceptance for {work_item_id} must include an ISO expires_on date",
        blocking=blocking,
    )
    if parsed_expiry is not None:
        not_expired = parsed_expiry >= dt.datetime.now(dt.timezone.utc).date()
        require(
            not_expired,
            f"PDS {work_item_id} risk acceptance is unexpired",
            artifact_path,
            f"expires_on {parsed_expiry.isoformat()}"
            if not_expired
            else f"PDS baseline decision risk acceptance for {work_item_id} expired on {parsed_expiry.isoformat()}",
            blocking=blocking,
        )
        within_window = parsed_expiry <= dt.datetime.now(dt.timezone.utc).date() + dt.timedelta(days=max_days)
        require(
            within_window,
            f"PDS {work_item_id} risk acceptance expiry is within max window",
            artifact_path,
            f"expires_on {parsed_expiry.isoformat()}, max_days {max_days}"
            if within_window
            else (
                f"PDS baseline decision risk acceptance for {work_item_id} expires on "
                f"{parsed_expiry.isoformat()}, beyond max_days {max_days}"
            ),
            blocking=blocking,
        )
        ok = ok and not_expired and within_window
    else:
        ok = False
    return ok


def validate_pds_decision_work_items(decision, decision_path, required_work_items, max_risk_days, blocking):
    artifact_path = decision_path.as_posix()
    entries = collect_pds_decision_entries(decision)
    require(
        bool(entries),
        "PDS baseline retained decision maps work items",
        artifact_path,
        f"{len(entries)} work item/category entries"
        if entries
        else "pds-baseline-decision.json must include work_items or categories mapping LIVE-015 through LIVE-021",
        blocking=blocking,
    )

    entries_by_work_item = {}
    for entry in entries:
        for work_item_id in collect_pds_work_item_ids(entry):
            entries_by_work_item.setdefault(work_item_id, []).append(entry)

    covered_ids = set()
    evidence_backed_ids = set()
    risk_accepted_ids = set()
    evidence_path_set = set()
    evidence_statuses = {"evidence", "proven", "satisfied", "complete"}
    risk_statuses = {"risk-accepted", "risk_accepted", "exception"}

    for work_item_id in sorted(required_work_items):
        item_entries = entries_by_work_item.get(work_item_id, [])
        require(
            bool(item_entries),
            f"PDS baseline retained decision covers {work_item_id}",
            artifact_path,
            f"{work_item_id} has {len(item_entries)} decision entr{'y' if len(item_entries) == 1 else 'ies'}"
            if item_entries
            else f"pds-baseline-decision.json must map {work_item_id} to evidence or risk acceptance",
            blocking=blocking,
        )
        if not item_entries:
            continue
        covered_ids.add(work_item_id)
        item_valid = False
        for entry in item_entries:
            status = str(entry.get("status", "")).strip().lower()
            status_allowed = status in evidence_statuses or status in risk_statuses
            require(
                status_allowed,
                f"PDS {work_item_id} decision status is allowed",
                artifact_path,
                f"status {status}"
                if status_allowed
                else f"PDS baseline decision for {work_item_id} must use status evidence or risk-accepted",
                blocking=blocking,
            )
            entry_evidence_paths = collect_pds_evidence_paths(entry)
            if status in evidence_statuses:
                evidence_paths_present = bool(entry_evidence_paths)
                require(
                    evidence_paths_present,
                    f"PDS {work_item_id} decision lists evidence files",
                    artifact_path,
                    f"{len(entry_evidence_paths)} evidence file{'s' if len(entry_evidence_paths) != 1 else ''}"
                    if evidence_paths_present
                    else f"PDS baseline decision for {work_item_id} must list evidence_files",
                    blocking=blocking,
                )
                evidence_ok = evidence_paths_present
                for evidence_path in entry_evidence_paths:
                    resolved = resolve_pds_decision_evidence_path(evidence_path, decision_path)
                    require(
                        resolved is not None,
                        f"PDS {work_item_id} evidence file exists: {Path(evidence_path).name}",
                        artifact_path,
                        resolved.as_posix()
                        if resolved is not None
                        else f"missing PDS baseline decision evidence file for {work_item_id}: {evidence_path}",
                        blocking=blocking,
                    )
                    evidence_retained = resolved is not None and path_is_within_directory(resolved, report_dir)
                    require(
                        evidence_retained,
                        f"PDS {work_item_id} evidence file is retained in report directory",
                        artifact_path,
                        (
                            f"{resolved.as_posix()} is retained under {report_dir.as_posix()}"
                            if evidence_retained
                            else f"PDS baseline decision evidence file for {work_item_id} must be retained under {report_dir.as_posix()}: {evidence_path}"
                        ),
                        blocking=blocking,
                    )
                    evidence_non_empty = resolved is not None and resolved.stat().st_size > 0
                    require(
                        evidence_non_empty,
                        f"PDS {work_item_id} evidence file is non-empty",
                        artifact_path,
                        (
                            f"{resolved.as_posix()} has {resolved.stat().st_size} bytes"
                            if evidence_non_empty
                            else f"PDS baseline decision evidence file for {work_item_id} must be non-empty: {evidence_path}"
                        ),
                        blocking=blocking,
                    )
                    evidence_ok = evidence_ok and evidence_retained and evidence_non_empty
                    if evidence_retained and evidence_non_empty:
                        evidence_path_set.add(resolved.as_posix())
                if evidence_ok:
                    item_valid = True
                    evidence_backed_ids.add(work_item_id)
            elif status in risk_statuses and validate_pds_risk_acceptance(
                entry,
                work_item_id,
                artifact_path,
                max_risk_days,
                blocking,
            ):
                item_valid = True
                risk_accepted_ids.add(work_item_id)
        require(
            item_valid,
            f"PDS baseline retained decision satisfies {work_item_id}",
            artifact_path,
            f"{work_item_id} satisfied"
            if item_valid
            else f"pds-baseline-decision.json must satisfy {work_item_id} with existing evidence files or an unexpired risk acceptance",
            blocking=blocking,
        )

    return {
        "covered_work_items": covered_ids,
        "evidence_backed_work_items": evidence_backed_ids,
        "risk_accepted_work_items": risk_accepted_ids,
        "evidence_paths": evidence_path_set,
    }


def validate_security_assurance_evidence_artifacts(category_name, evidence_artifacts, artifact_path, blocking):
    require(
        isinstance(evidence_artifacts, list) and bool(evidence_artifacts),
        f"security assurance {category_name} records evidence artifacts",
        artifact_path,
        "evidence disposition requires at least one retained evidence artifact",
        blocking=blocking,
    )
    if not isinstance(evidence_artifacts, list):
        return
    for index, evidence_artifact in enumerate(evidence_artifacts):
        evidence_path = f"{artifact_path}#/evidence_artifacts/{index}"
        require(
            isinstance(evidence_artifact, dict),
            f"security assurance {category_name} evidence artifact is object",
            evidence_path,
            blocking=blocking,
        )
        if not isinstance(evidence_artifact, dict):
            continue
        retained_path = evidence_artifact.get("path")
        require(
            isinstance(retained_path, str) and bool(retained_path.strip()),
            f"security assurance {category_name} evidence artifact records path",
            evidence_path,
            blocking=blocking,
        )
        if isinstance(retained_path, str) and retained_path.strip():
            resolved = resolve_retained_path(retained_path)
            require(
                resolved is not None,
                f"security assurance {category_name} evidence artifact exists",
                evidence_path,
                resolved.as_posix()
                if resolved is not None
                else f"missing retained evidence artifact: {retained_path}",
                blocking=blocking,
            )
            if resolved is not None:
                expected_sha = evidence_artifact.get("sha256")
                actual_sha = f"sha256:{sha256_digest(resolved)}"
                require(
                    expected_sha == actual_sha,
                    f"security assurance {category_name} evidence artifact sha256 matches retained file",
                    evidence_path,
                    f"expected {expected_sha or '<missing>'}, actual {actual_sha}",
                    blocking=blocking,
                )
                expected_bytes = evidence_artifact.get("bytes")
                actual_bytes = resolved.stat().st_size
                require(
                    expected_bytes == actual_bytes,
                    f"security assurance {category_name} evidence artifact byte size matches retained file",
                    evidence_path,
                    f"expected {expected_bytes!r}, actual {actual_bytes}",
                    blocking=blocking,
                )
        require(
            evidence_artifact.get("present") is True,
            f"security assurance {category_name} evidence artifact present flag is true",
            evidence_path,
            f"actual {evidence_artifact.get('present')!r}",
            blocking=blocking,
        )
        require(
            evidence_artifact.get("ok") is True,
            f"security assurance {category_name} evidence artifact ok flag is true",
            evidence_path,
            f"actual {evidence_artifact.get('ok')!r}",
            blocking=blocking,
        )
        require(
            isinstance(evidence_artifact.get("sha256"), str)
            and evidence_artifact.get("sha256", "").startswith("sha256:"),
            f"security assurance {category_name} evidence artifact records sha256",
            evidence_path,
            blocking=blocking,
        )


def validate_retained_file_artifact(
    report_name,
    evidence_artifact,
    artifact_path,
    blocking,
    required_parent=None,
    required_parent_label="report directory",
    require_ok_flag=True,
):
    require(
        isinstance(evidence_artifact, dict),
        f"{report_name} retained file artifact is object",
        artifact_path,
        blocking=blocking,
    )
    if not isinstance(evidence_artifact, dict):
        return None
    retained_path = evidence_artifact.get("path")
    require(
        isinstance(retained_path, str) and bool(retained_path.strip()),
        f"{report_name} retained file artifact records path",
        artifact_path,
        blocking=blocking,
    )
    resolved = None
    if isinstance(retained_path, str) and retained_path.strip():
        resolved = resolve_retained_path(retained_path)
        require(
            resolved is not None,
            f"{report_name} retained file artifact exists",
            artifact_path,
            resolved.as_posix()
            if resolved is not None
            else f"missing retained artifact: {retained_path}",
            blocking=blocking,
        )
        if resolved is not None and required_parent is not None:
            retained_under_parent = path_is_within_directory(resolved, required_parent)
            require(
                retained_under_parent,
                f"{report_name} retained file artifact is retained in {required_parent_label}",
                artifact_path,
                (
                    f"{resolved.as_posix()} is retained under {required_parent.as_posix()}"
                    if retained_under_parent
                    else f"{report_name} retained file artifact must be retained under {required_parent.as_posix()}: {resolved.as_posix()}"
                ),
                blocking=blocking,
            )
    require(
        evidence_artifact.get("present") is True,
        f"{report_name} retained file artifact present flag is true",
        artifact_path,
        f"actual {evidence_artifact.get('present')!r}",
        blocking=blocking,
    )
    if require_ok_flag or "ok" in evidence_artifact:
        require(
            evidence_artifact.get("ok") is True,
            f"{report_name} retained file artifact ok flag is true",
            artifact_path,
            f"actual {evidence_artifact.get('ok')!r}",
            blocking=blocking,
        )
    expected_sha = evidence_artifact.get("sha256")
    normalized_expected_sha = None
    if isinstance(expected_sha, str):
        expected_sha_text = expected_sha.strip()
        if expected_sha_text.startswith("sha256:"):
            normalized_expected_sha = "sha256:" + expected_sha_text.removeprefix("sha256:").lower()
        elif re.fullmatch(r"[0-9a-fA-F]{64}", expected_sha_text):
            normalized_expected_sha = f"sha256:{expected_sha_text.lower()}"
    require(
        normalized_expected_sha is not None,
        f"{report_name} retained file artifact records sha256",
        artifact_path,
        f"actual {expected_sha!r}",
        blocking=blocking,
    )
    expected_bytes = evidence_artifact.get("bytes")
    require(
        isinstance(expected_bytes, int) and not isinstance(expected_bytes, bool),
        f"{report_name} retained file artifact records byte size",
        artifact_path,
        f"actual {expected_bytes!r}",
        blocking=blocking,
    )
    require(
        isinstance(expected_bytes, int) and not isinstance(expected_bytes, bool) and expected_bytes > 0,
        f"{report_name} retained file artifact byte size is non-zero",
        artifact_path,
        f"actual {expected_bytes!r}",
        blocking=blocking,
    )
    if resolved is not None:
        actual_sha = f"sha256:{sha256_digest(resolved)}"
        require(
            normalized_expected_sha == actual_sha,
            f"{report_name} retained file artifact sha256 matches retained file",
            artifact_path,
            f"expected {expected_sha or '<missing>'}, actual {actual_sha}",
            blocking=blocking,
        )
        actual_bytes = resolved.stat().st_size
        require(
            expected_bytes == actual_bytes,
            f"{report_name} retained file artifact byte size matches retained file",
            artifact_path,
            f"expected {expected_bytes!r}, actual {actual_bytes}",
            blocking=blocking,
        )
        require(
            actual_bytes > 0,
            f"{report_name} retained file artifact retained file is non-empty",
            artifact_path,
            f"actual {actual_bytes}",
            blocking=blocking,
        )
    return resolved


def validate_security_assurance_risk_acceptance(category_name, risk_acceptance, artifact_path, max_days, blocking):
    require(
        isinstance(risk_acceptance, dict),
        f"security assurance {category_name} risk acceptance is object",
        artifact_path,
        "risk-accepted disposition requires a structured risk_acceptance object",
        blocking=blocking,
    )
    if not isinstance(risk_acceptance, dict):
        return

    ok = True
    for label, keys in (
        ("owner", ("owner", "risk_owner")),
        ("approver", ("approved_by", "approver")),
        ("release scope", ("release_scope", "scope")),
        ("expiry", ("expires_on", "expires_at", "expiration_date", "expiry")),
        ("rationale", ("rationale", "reason")),
        ("follow-up", ("follow_up", "remediation_plan", "followUp")),
    ):
        value = first_non_empty(risk_acceptance, keys)
        require(
            bool(value),
            f"security assurance {category_name} risk acceptance has {label}",
            artifact_path,
            value or f"risk acceptance for {category_name} must include {label}",
            blocking=blocking,
        )
        ok = ok and bool(value)

    controls = risk_acceptance.get("compensating_controls")
    if controls is None:
        controls = risk_acceptance.get("controls")
    controls_ok = (
        isinstance(controls, list)
        and bool(controls)
        and all(isinstance(control, str) and bool(control.strip()) for control in controls)
    )
    require(
        controls_ok,
        f"security assurance {category_name} risk acceptance has compensating controls",
        artifact_path,
        "risk acceptance must list non-empty compensating controls",
        blocking=blocking,
    )
    ok = ok and controls_ok

    expiry = first_non_empty(
        risk_acceptance,
        ("expires_on", "expires_at", "expiration_date", "expiry"),
    )
    parsed_expiry = parse_expiry_date(expiry)
    require(
        parsed_expiry is not None,
        f"security assurance {category_name} risk acceptance expiry is valid",
        artifact_path,
        expiry or f"risk acceptance for {category_name} must include an ISO expires_on date",
        blocking=blocking,
    )
    if parsed_expiry is not None:
        today = dt.datetime.now(dt.timezone.utc).date()
        not_expired = parsed_expiry >= today
        require(
            not_expired,
            f"security assurance {category_name} risk acceptance is unexpired",
            artifact_path,
            f"expires_on {parsed_expiry.isoformat()}"
            if not_expired
            else f"risk acceptance for {category_name} expired on {parsed_expiry.isoformat()}",
            blocking=blocking,
        )
        within_window = parsed_expiry <= today + dt.timedelta(days=max_days)
        require(
            within_window,
            f"security assurance {category_name} risk acceptance expiry is within max window",
            artifact_path,
            f"expires_on {parsed_expiry.isoformat()}, max_days {max_days}",
            blocking=blocking,
        )
        ok = ok and not_expired and within_window

    require(
        ok,
        f"security assurance {category_name} risk acceptance is complete",
        artifact_path,
        blocking=blocking,
    )


def validate_remediation_items(report_name, remediation_items, artifact_path, blocking):
    require(
        isinstance(remediation_items, list),
        f"{report_name} remediation work items are recorded",
        artifact_path,
        blocking=blocking,
    )
    remediation_ids = set()
    if not isinstance(remediation_items, list):
        return remediation_ids
    for index, item in enumerate(remediation_items):
        item_path = f"{artifact_path}#/remediation_work_items/{index}"
        require(
            isinstance(item, dict),
            f"{report_name} remediation work item is object",
            item_path,
            blocking=blocking,
        )
        if not isinstance(item, dict):
            continue
        item_id = item.get("id")
        require(
            isinstance(item_id, str) and bool(item_id.strip()),
            f"{report_name} remediation work item has id",
            item_path,
            blocking=blocking,
        )
        if isinstance(item_id, str) and item_id.strip():
            remediation_ids.add(item_id.strip())
        for field in ("priority", "owner", "summary", "doc"):
            value = item.get(field)
            require(
                isinstance(value, str) and bool(value.strip()),
                f"{report_name} remediation work item {item_id or index} has {field}",
                item_path,
                blocking=blocking,
            )
    return remediation_ids


def validate_release_check_checks(
    release_check,
    artifact_path,
    provider_url_preflight_issues,
    release_check_generated_at,
    blocking,
):
    check_entries = release_check.get("checks")
    require(
        isinstance(check_entries, list) and bool(check_entries),
        "release-check records checks",
        artifact_path,
        blocking=blocking,
    )
    check_names = []
    failed_check_names = []
    provider_preflight_seen = False
    provider_preflight_failed = False
    provider_preflight_log_retained = False
    check_entries_by_name = {}
    if isinstance(check_entries, list):
        seen_names = set()
        for index, entry in enumerate(check_entries):
            entry_path = f"{artifact_path}#/checks/{index}"
            require(
                isinstance(entry, dict),
                "release-check check entry is object",
                entry_path,
                blocking=blocking,
            )
            if not isinstance(entry, dict):
                continue
            name = entry.get("name")
            name_ok = isinstance(name, str) and bool(name.strip())
            label = name.strip() if name_ok else str(index)
            require(
                name_ok,
                "release-check check entry has name",
                entry_path,
                blocking=blocking,
            )
            if name_ok:
                require(
                    label not in seen_names,
                    f"release-check check {label} name is unique",
                    entry_path,
                    blocking=blocking,
                )
                seen_names.add(label)
                check_names.append(label)
                check_entries_by_name[label] = entry

            ok_value = entry.get("ok")
            require(
                isinstance(ok_value, bool),
                f"release-check check {label} ok flag is boolean",
                entry_path,
                f"actual {ok_value!r}",
                blocking=blocking,
            )
            if ok_value is False and name_ok:
                failed_check_names.append(label)

            log_value = entry.get("log")
            if ok_value is False:
                require(
                    isinstance(log_value, str) and bool(log_value.strip()),
                    f"release-check failed check {label} retains log",
                    entry_path,
                    "failed checks must retain a log path",
                    blocking=blocking,
                )
            if log_value is not None:
                log_is_path = isinstance(log_value, str) and bool(log_value.strip())
                require(
                    log_is_path,
                    f"release-check check {label} log path is string",
                    entry_path,
                    blocking=blocking,
                )
                if log_is_path:
                    resolved_log = resolve_retained_path(log_value)
                    require(
                        resolved_log is not None,
                        f"release-check check {label} log exists",
                        entry_path,
                        f"missing retained log: {log_value}",
                        blocking=blocking,
                    )
                    if resolved_log is not None:
                        require(
                            path_is_within_directory(resolved_log, report_dir),
                            f"release-check check {label} log is retained in report directory",
                            entry_path,
                            (
                                f"actual {resolved_log.as_posix()}, "
                                f"report_dir {report_dir.resolve(strict=False).as_posix()}"
                            ),
                            blocking=blocking,
                        )
                    if label == "provider-url-preflight" and resolved_log is not None:
                        provider_preflight_log_retained = True

            if "artifact" in entry and entry.get("artifact") is not None:
                artifact_value = entry.get("artifact")
                artifact_is_path = isinstance(artifact_value, str) and bool(artifact_value.strip())
                require(
                    artifact_is_path,
                    f"release-check check {label} artifact path is string",
                    entry_path,
                    blocking=blocking,
                )
                if artifact_is_path:
                    resolved_artifact = resolve_retained_path(artifact_value)
                    require(
                        resolved_artifact is not None,
                        f"release-check check {label} artifact exists",
                        entry_path,
                        f"missing retained artifact: {artifact_value}",
                        blocking=blocking,
                    )
                    if resolved_artifact is not None:
                        require(
                            path_is_within_directory(resolved_artifact, report_dir),
                            f"release-check check {label} artifact is retained in report directory",
                            entry_path,
                            (
                                f"actual {resolved_artifact.as_posix()}, "
                                f"report_dir {report_dir.resolve(strict=False).as_posix()}"
                            ),
                            blocking=blocking,
                        )
                        validate_release_check_artifact_timestamp(
                            label,
                            resolved_artifact,
                            entry_path,
                            release_check_generated_at,
                            blocking=blocking,
                        )
                        try:
                            artifact_value = json.loads(
                                resolved_artifact.read_text(encoding="utf-8")
                            )
                        except Exception:
                            artifact_value = None
                        if (
                            isinstance(artifact_value, dict)
                            and artifact_value.get("release_ready") is False
                        ):
                            require(
                                ok_value is not True,
                                f"release-check check {label} ok flag does not mask non-ready artifact",
                                entry_path,
                                f"{resolved_artifact.as_posix()} has release_ready:false",
                                blocking=blocking,
                            )

            if label == "provider-url-preflight":
                provider_preflight_seen = True
                provider_preflight_failed = ok_value is False

    require(
        release_check.get("ok") is not True or not failed_check_names,
        "release-check ok flag matches retained check results",
        artifact_path,
        f"failed checks {failed_check_names}",
        blocking=blocking,
    )
    require(
        release_check.get("release_ready") is not True or not failed_check_names,
        "release-check release_ready flag matches retained check results",
        artifact_path,
        f"failed checks {failed_check_names}",
        blocking=blocking,
    )

    current_check = release_check.get("current_check")
    if current_check is None:
        if release_check.get("ok") is not True:
            require(
                False,
                "release-check current_check is recorded when not ok",
                artifact_path,
                blocking=blocking,
            )
    else:
        current_check_ok = isinstance(current_check, str) and bool(current_check.strip())
        require(
            current_check_ok,
            "release-check current_check is a non-empty string",
            artifact_path,
            f"actual {current_check!r}",
            blocking=blocking,
        )
        if current_check_ok:
            current_check_name = current_check.strip()
            require(
                current_check_name in check_names,
                "release-check current_check names a recorded check",
                artifact_path,
                f"current_check {current_check_name}; checks {check_names}",
                blocking=blocking,
            )
            if release_check.get("ok") is not True:
                require(
                    current_check_name in failed_check_names,
                    "release-check current_check identifies a failed check",
                    artifact_path,
                    f"current_check {current_check_name}; failed checks {failed_check_names}",
                    blocking=blocking,
                )
            require(
                release_check.get("ok") is not True,
                "release-check current_check is absent when ok",
                artifact_path,
                f"current_check {current_check_name}",
                blocking=blocking,
            )
            require(
                release_check.get("release_ready") is not True,
                "release-check current_check is absent when release-ready",
                artifact_path,
                f"current_check {current_check_name}",
                blocking=blocking,
            )

    if isinstance(provider_url_preflight_issues, list) and provider_url_preflight_issues:
        require(
            provider_preflight_seen,
            "release-check records provider URL preflight check",
            artifact_path,
            blocking=blocking,
        )
        require(
            provider_preflight_failed,
            "release-check provider URL preflight check failed",
            artifact_path,
            blocking=blocking,
        )
        require(
            provider_preflight_log_retained,
            "release-check provider URL preflight check retains log",
            artifact_path,
            blocking=blocking,
        )

    return check_entries_by_name, set(check_names), set(failed_check_names)


def validate_release_check_ready_artifact(
    check_entries_by_name,
    check_name,
    expected_artifact_path,
    release_check_path,
    blocking,
):
    entry = check_entries_by_name.get(check_name)
    require(
        isinstance(entry, dict),
        f"release-ready release-check includes {check_name} check",
        release_check_path,
        blocking=blocking,
    )
    if not isinstance(entry, dict):
        return {}
    require(
        entry.get("ok") is True,
        f"release-ready release-check {check_name} check passed",
        release_check_path,
        f"actual {entry.get('ok')!r}",
        blocking=blocking,
    )
    artifact_value = entry.get("artifact")
    artifact_is_path = isinstance(artifact_value, str) and bool(artifact_value.strip())
    require(
        artifact_is_path,
        f"release-ready release-check {check_name} retains artifact",
        release_check_path,
        blocking=blocking,
    )
    if not artifact_is_path:
        return {}
    resolved_artifact = resolve_retained_path(artifact_value)
    expected_artifact = Path(expected_artifact_path).resolve()
    require(
        resolved_artifact is not None and resolved_artifact.resolve() == expected_artifact,
        f"release-ready release-check {check_name} artifact matches retained evidence",
        release_check_path,
        f"expected {expected_artifact.as_posix()}, actual {artifact_value}",
        blocking=blocking,
    )


def validate_provider_url_preflight_snapshot(
    check_entries_by_name,
    expected_missing_provider_base_urls,
    expected_invalid_provider_base_urls,
    release_check_path,
    blocking,
):
    entry = check_entries_by_name.get("provider-url-preflight")
    require(
        isinstance(entry, dict),
        "release-check records provider URL preflight check",
        release_check_path,
        blocking=blocking,
    )
    if not isinstance(entry, dict):
        return

    artifact_value = entry.get("artifact")
    artifact_is_path = isinstance(artifact_value, str) and bool(artifact_value.strip())
    require(
        artifact_is_path,
        "release-check provider URL preflight retains snapshot artifact",
        release_check_path,
        blocking=blocking,
    )
    if not artifact_is_path:
        return
    resolved_artifact = resolve_retained_path(artifact_value)
    require(
        resolved_artifact is not None,
        "release-check provider URL preflight snapshot exists",
        release_check_path,
        f"missing retained artifact: {artifact_value}",
        blocking=blocking,
    )
    if resolved_artifact is None:
        return {}
    try:
        resolved_artifact.resolve().relative_to(report_dir.resolve())
        artifact_in_report_dir = True
    except ValueError:
        artifact_in_report_dir = False
    require(
        artifact_in_report_dir,
        "release-check provider URL preflight snapshot is retained in report directory",
        release_check_path,
        f"actual {resolved_artifact.as_posix()}, report_dir {report_dir.resolve().as_posix()}",
        blocking=blocking,
    )

    try:
        snapshot = json.loads(resolved_artifact.read_text(encoding="utf-8"))
        require(
            isinstance(snapshot, dict),
            "release-check provider URL preflight snapshot is object",
            resolved_artifact.as_posix(),
            blocking=blocking,
        )
    except Exception as exc:  # pragma: no cover - exercised by shell usage.
        require(
            False,
            "release-check provider URL preflight snapshot parses as JSON",
            resolved_artifact.as_posix(),
            str(exc),
            blocking=blocking,
        )
        return {}
    if not isinstance(snapshot, dict):
        return {}

    now = dt.datetime.now(dt.timezone.utc)
    snapshot_generated_at = parse_timestamp_with_timezone(snapshot.get("generated_at_utc"))
    require(
        snapshot_generated_at is not None,
        "release-check provider URL preflight snapshot timestamp is valid",
        resolved_artifact.as_posix(),
        "generated_at_utc must be ISO-8601 with timezone",
        blocking=blocking,
    )
    if snapshot_generated_at is not None:
        require(
            snapshot_generated_at <= now + dt.timedelta(minutes=5),
            "release-check provider URL preflight snapshot timestamp is not in the future",
            resolved_artifact.as_posix(),
            snapshot_generated_at.isoformat().replace("+00:00", "Z"),
            blocking=blocking,
        )
        require(
            snapshot_generated_at >= now - PROVIDER_PREFLIGHT_MAX_AGE,
            "release-check provider URL preflight snapshot timestamp is recent",
            resolved_artifact.as_posix(),
            (
                f"generated_at_utc {snapshot_generated_at.isoformat().replace('+00:00', 'Z')} "
                f"is older than {PROVIDER_PREFLIGHT_MAX_AGE_HOURS} hours"
            ),
            blocking=blocking,
        )

    snapshot_missing = snapshot.get("missing_provider_base_urls")
    snapshot_invalid = snapshot.get("invalid_provider_base_urls")
    snapshot_valid = snapshot.get("valid_provider_base_urls")
    require(
        isinstance(snapshot_missing, list),
        "release-check provider URL preflight snapshot records missing provider URLs",
        resolved_artifact.as_posix(),
        blocking=blocking,
    )
    require(
        isinstance(snapshot_invalid, list),
        "release-check provider URL preflight snapshot records invalid provider URLs",
        resolved_artifact.as_posix(),
        blocking=blocking,
    )
    require(
        isinstance(snapshot_valid, list),
        "release-check provider URL preflight snapshot records valid provider URLs",
        resolved_artifact.as_posix(),
        blocking=blocking,
    )
    issue_provider_names = []
    for issue in list(expected_missing_provider_base_urls) + list(expected_invalid_provider_base_urls):
        if not isinstance(issue, dict):
            continue
        provider = issue.get("provider")
        if isinstance(provider, str) and provider.strip() in SECURITY_EXPECTED_PROVIDERS:
            issue_provider_names.append(provider.strip())
    snapshot_valid_base_urls = {}
    if isinstance(snapshot_valid, list):
        valid_url_keys = {}
        valid_provider_names = []
        for index, valid_provider_url in enumerate(snapshot_valid):
            item_path = f"{resolved_artifact.as_posix()}#/valid_provider_base_urls/{index}"
            require(
                isinstance(valid_provider_url, dict),
                "release-check valid provider base URL entry is object",
                item_path,
                blocking=blocking,
            )
            if not isinstance(valid_provider_url, dict):
                continue
            provider = valid_provider_url.get("provider")
            env = valid_provider_url.get("env")
            redacted_url = valid_provider_url.get("redacted_url")
            health_url = valid_provider_url.get("health_url")
            health_status = valid_provider_url.get("health_status")
            health_checked_at = valid_provider_url.get("health_checked_at_utc")
            health_timeout_seconds = valid_provider_url.get("health_timeout_seconds")
            require(
                isinstance(provider, str) and provider.strip() in SECURITY_EXPECTED_PROVIDERS,
                "release-check valid provider base URL entry names release-certified provider",
                item_path,
                f"actual {provider!r}",
                blocking=blocking,
            )
            if isinstance(provider, str) and provider.strip() in SECURITY_EXPECTED_PROVIDERS:
                valid_provider_names.append(provider.strip())
                expected_env = expected_provider_base_url_env(provider)
                require(
                    isinstance(env, str) and env.strip() == expected_env,
                    "release-check valid provider base URL entry records canonical env var",
                    item_path,
                    f"expected {expected_env}, actual {env!r}",
                    blocking=blocking,
                )
            redacted_url_safe = (
                isinstance(redacted_url, str)
                and bool(redacted_url.strip())
                and "@" not in redacted_url
                and "?" not in redacted_url
                and "#" not in redacted_url
            )
            require(
                redacted_url_safe,
                "release-check valid provider base URL entry records safe redacted URL",
                item_path,
                f"actual {redacted_url!r}",
                blocking=blocking,
            )
            if not redacted_url_safe:
                continue
            try:
                parts = urlsplit(redacted_url.strip())
                host = parts.hostname
                port = parts.port
            except ValueError:
                host = None
                port = None
                parts = None
            provider_url_identity_ok = (
                parts is not None
                and parts.scheme.lower() in {"http", "https"}
                and bool(host)
            )
            require(
                provider_url_identity_ok,
                "release-check valid provider base URL entry has parseable HTTP identity",
                item_path,
                f"actual {redacted_url!r}",
                blocking=blocking,
            )
            if provider_url_identity_ok:
                effective_port = port if port is not None else (443 if parts.scheme.lower() == "https" else 80)
                identity_key = (parts.scheme.lower(), host.lower(), effective_port)
                valid_url_keys.setdefault(identity_key, []).append(provider)
                expected_health_url = redacted_url.strip().rstrip("/") + "/health/ready"
                if isinstance(provider, str) and provider.strip() in SECURITY_EXPECTED_PROVIDERS:
                    snapshot_valid_base_urls[provider.strip()] = redacted_url.strip()
            else:
                expected_health_url = ""
            health_url_safe = (
                isinstance(health_url, str)
                and bool(health_url.strip())
                and "@" not in health_url
                and "?" not in health_url
                and "#" not in health_url
            )
            require(
                health_url_safe,
                "release-check valid provider base URL entry records safe health URL",
                item_path,
                f"actual {health_url!r}",
                blocking=blocking,
            )
            require(
                health_url_safe and health_url.strip() == expected_health_url,
                "release-check valid provider base URL health URL matches retained base URL",
                item_path,
                f"expected {expected_health_url or '<unavailable>'}, actual {health_url!r}",
                blocking=blocking,
            )
            require(
                is_int(health_status) and 200 <= health_status < 400,
                "release-check valid provider base URL health status is passing",
                item_path,
                f"actual {health_status!r}",
                blocking=blocking,
            )
            parsed_health_checked_at = parse_timestamp_with_timezone(health_checked_at)
            require(
                parsed_health_checked_at is not None,
                "release-check valid provider base URL health timestamp is valid",
                item_path,
                "health_checked_at_utc must be ISO-8601 with timezone",
                blocking=blocking,
            )
            if parsed_health_checked_at is not None:
                require(
                    parsed_health_checked_at <= now + dt.timedelta(minutes=5),
                    "release-check valid provider base URL health timestamp is not in the future",
                    item_path,
                    parsed_health_checked_at.isoformat().replace("+00:00", "Z"),
                    blocking=blocking,
                )
                require(
                    parsed_health_checked_at >= now - PROVIDER_PREFLIGHT_MAX_AGE,
                    "release-check valid provider base URL health timestamp is recent",
                    item_path,
                    (
                        f"health_checked_at_utc {parsed_health_checked_at.isoformat().replace('+00:00', 'Z')} "
                        f"is older than {PROVIDER_PREFLIGHT_MAX_AGE_HOURS} hours"
                    ),
                    blocking=blocking,
                )
            timeout_ok = (
                isinstance(health_timeout_seconds, (int, float))
                and not isinstance(health_timeout_seconds, bool)
                and health_timeout_seconds > 0
            )
            require(
                timeout_ok,
                "release-check valid provider base URL health timeout is positive",
                item_path,
                f"actual {health_timeout_seconds!r}",
                blocking=blocking,
            )
        duplicate_provider_names = sorted(
            provider
            for provider in set(valid_provider_names)
            if valid_provider_names.count(provider) > 1
        )
        require(
            not duplicate_provider_names,
            "release-check valid provider base URL entries name each provider once",
            resolved_artifact.as_posix(),
                f"duplicates={duplicate_provider_names}",
                blocking=blocking,
            )
        expected_providers = set(SECURITY_EXPECTED_PROVIDERS)
        actual_provider_statuses = set(valid_provider_names) | set(issue_provider_names)
        overlapping_provider_statuses = sorted(set(valid_provider_names) & set(issue_provider_names))
        require(
            not overlapping_provider_statuses,
            "release-check provider URL preflight snapshot assigns one status per provider",
            resolved_artifact.as_posix(),
            f"overlapping providers={overlapping_provider_statuses}",
            blocking=blocking,
        )
        require(
            actual_provider_statuses == expected_providers,
            "release-check provider URL preflight snapshot covers release-certified providers",
            resolved_artifact.as_posix(),
            (
                f"missing={sorted(expected_providers - actual_provider_statuses)}; "
                f"extra={sorted(actual_provider_statuses - expected_providers)}"
            ),
            blocking=blocking,
        )
        for identity_key, providers in valid_url_keys.items():
            duplicate_providers = sorted(str(provider) for provider in providers if isinstance(provider, str))
            require(
                len(duplicate_providers) == 1,
                "release-check valid provider base URLs use distinct host/port identities",
                resolved_artifact.as_posix(),
                f"identity={identity_key}; providers={duplicate_providers}",
                blocking=blocking,
            )
    if isinstance(snapshot_missing, list):
        require(
            snapshot_missing == expected_missing_provider_base_urls,
            "release-check missing provider URLs match preflight snapshot",
            release_check_path,
            f"snapshot={snapshot_missing}; release-check={expected_missing_provider_base_urls}",
            blocking=blocking,
        )
    if isinstance(snapshot_invalid, list):
        require(
            snapshot_invalid == expected_invalid_provider_base_urls,
            "release-check invalid provider URLs match preflight snapshot",
            release_check_path,
            f"snapshot={snapshot_invalid}; release-check={expected_invalid_provider_base_urls}",
            blocking=blocking,
        )
    if isinstance(snapshot.get("ok"), bool):
        expected_ok = not expected_missing_provider_base_urls and not expected_invalid_provider_base_urls
        require(
            snapshot.get("ok") is expected_ok,
            "release-check provider URL preflight snapshot ok matches URL findings",
            resolved_artifact.as_posix(),
            f"expected {expected_ok}, actual {snapshot.get('ok')!r}",
            blocking=blocking,
        )
    else:
        require(
            False,
            "release-check provider URL preflight snapshot ok flag is boolean",
            resolved_artifact.as_posix(),
            f"actual {snapshot.get('ok')!r}",
            blocking=blocking,
        )
    return snapshot_valid_base_urls


def is_int(value):
    return isinstance(value, int) and not isinstance(value, bool)


def is_number(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def has_artifact(report, name):
    return any(
        artifact.get("name") == name
        and artifact.get("required") is True
        and artifact.get("present") is True
        for artifact in report.get("artifacts", [])
        if isinstance(artifact, dict)
    )


def artifact_entry(report, name):
    if not isinstance(report, dict):
        return None
    for artifact in report.get("artifacts", []):
        if isinstance(artifact, dict) and artifact.get("name") == name:
            return artifact
    return None


def artifact_required(report, name):
    artifact = artifact_entry(report, name)
    return isinstance(artifact, dict) and artifact.get("required") is True


def validate_artifact_names_unique(report_name, report, artifact_path, blocking):
    artifacts_value = report.get("artifacts") if isinstance(report, dict) else None
    require(
        isinstance(artifacts_value, list),
        f"{report_name} artifacts is an array",
        artifact_path,
        blocking=blocking,
    )
    if not isinstance(artifacts_value, list):
        return
    artifact_names = []
    for index, artifact in enumerate(artifacts_value):
        item_path = f"{artifact_path}#/artifacts/{index}"
        require(
            isinstance(artifact, dict),
            f"{report_name} artifact entry is object",
            item_path,
            blocking=blocking,
        )
        if not isinstance(artifact, dict):
            continue
        name = artifact.get("name")
        name_ok = isinstance(name, str) and bool(name.strip())
        require(
            name_ok,
            f"{report_name} artifact entry has name",
            item_path,
            blocking=blocking,
        )
        if name_ok:
            artifact_names.append(name.strip())
    duplicate_names = sorted(name for name in set(artifact_names) if artifact_names.count(name) > 1)
    require(
        not duplicate_names,
        f"{report_name} artifact names are unique",
        artifact_path,
        f"duplicates={duplicate_names}",
        blocking=blocking,
    )


def validate_named_artifact_reference(
    report_name,
    report,
    artifact_name,
    expected_path,
    artifact_path,
    blocking,
    require_required=True,
    require_sha=False,
    require_bytes=False,
):
    entry = artifact_entry(report, artifact_name)
    require(
        isinstance(entry, dict),
        f"{report_name} records {artifact_name} artifact",
        artifact_path,
        blocking=blocking,
    )
    if not isinstance(entry, dict):
        return None
    entry_path = f"{artifact_path}#/artifacts/{artifact_name}"
    if require_required:
        require(
            entry.get("required") is True,
            f"{report_name} {artifact_name} artifact is required",
            entry_path,
            f"actual {entry.get('required')!r}",
            blocking=blocking,
        )
    require(
        entry.get("present") is True,
        f"{report_name} {artifact_name} artifact present flag is true",
        entry_path,
        f"actual {entry.get('present')!r}",
        blocking=blocking,
    )
    retained_path = entry.get("path")
    require(
        isinstance(retained_path, str) and bool(retained_path.strip()),
        f"{report_name} {artifact_name} artifact records path",
        entry_path,
        blocking=blocking,
    )
    resolved = None
    if isinstance(retained_path, str) and retained_path.strip():
        resolved = resolve_retained_path(retained_path)
        require(
            resolved is not None,
            f"{report_name} {artifact_name} artifact exists",
            entry_path,
            resolved.as_posix() if resolved is not None else f"missing retained artifact: {retained_path}",
            blocking=blocking,
        )
        if resolved is not None:
            require(
                path_is_within_directory(resolved, report_dir),
                f"{report_name} {artifact_name} artifact is retained in report directory",
                entry_path,
                (
                    f"{resolved.as_posix()} is retained under {report_dir.as_posix()}"
                    if path_is_within_directory(resolved, report_dir)
                    else f"{report_name} {artifact_name} artifact must be retained under {report_dir.as_posix()}: {resolved.as_posix()}"
                ),
                blocking=blocking,
            )
            expected = Path(expected_path).resolve()
            require(
                resolved.resolve() == expected,
                f"{report_name} {artifact_name} artifact matches retained evidence",
                entry_path,
                f"expected {expected.as_posix()}, actual {resolved.resolve().as_posix()}",
                blocking=blocking,
            )
            if require_sha:
                expected_sha = entry.get("sha256")
                normalized_expected_sha = None
                if isinstance(expected_sha, str):
                    expected_sha_text = expected_sha.strip()
                    if expected_sha_text.startswith("sha256:"):
                        normalized_expected_sha = "sha256:" + expected_sha_text.removeprefix("sha256:").lower()
                    elif re.fullmatch(r"[0-9a-fA-F]{64}", expected_sha_text):
                        normalized_expected_sha = f"sha256:{expected_sha_text.lower()}"
                require(
                    normalized_expected_sha is not None,
                    f"{report_name} {artifact_name} artifact records sha256",
                    entry_path,
                    f"actual {expected_sha!r}",
                    blocking=blocking,
                )
                actual_sha = f"sha256:{sha256_digest(resolved)}"
                require(
                    normalized_expected_sha == actual_sha,
                    f"{report_name} {artifact_name} artifact sha256 matches retained file",
                    entry_path,
                    f"expected {expected_sha or '<missing>'}, actual {actual_sha}",
                    blocking=blocking,
                )
            if require_bytes:
                expected_bytes = entry.get("bytes")
                actual_bytes = resolved.stat().st_size
                require(
                    isinstance(expected_bytes, int) and not isinstance(expected_bytes, bool),
                    f"{report_name} {artifact_name} artifact records byte size",
                    entry_path,
                    f"actual {expected_bytes!r}",
                    blocking=blocking,
                )
                require(
                    expected_bytes == actual_bytes,
                    f"{report_name} {artifact_name} artifact byte size matches retained file",
                    entry_path,
                    f"expected {expected_bytes!r}, actual {actual_bytes}",
                    blocking=blocking,
                )
    return resolved


def parse_gitleaks_baseline_entries(path):
    entries = []
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except Exception as exc:
        return entries, str(exc)
    for line in lines:
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        entries.append(stripped.split()[0])
    return entries, None


def duplicate_values(values):
    return sorted(value for value in set(values) if values.count(value) > 1)


def lower_text(value):
    return value.strip().lower() if isinstance(value, str) else ""


def plan_text_contains_any(value, needles):
    text = lower_text(value)
    return [needle for needle in needles if needle in text]


def external_plan_required_artifact_issues(values):
    issues = []
    forbidden_fragments = (
        "focused",
        "focused-ci",
        "focused_evidence",
        "local-fixture",
        "fixture-artifacts",
        ".fixture.",
        "placeholder",
        "example",
        "127.0.0.1",
        "localhost",
        "<",
        ">",
    )
    if not isinstance(values, list):
        return issues
    for index, value in enumerate(values):
        if not isinstance(value, str):
            issues.append(f"#{index}:{value!r} is not a string")
            continue
        normalized = value.strip().replace("\\", "/")
        while normalized.startswith("./"):
            normalized = normalized[2:]
        if normalized.startswith("/") or not normalized.startswith("target/appfw/"):
            issues.append(
                f"#{index}:{value!r} is not release-staged under target/appfw"
            )
        matches = plan_text_contains_any(value, forbidden_fragments)
        if matches:
            issues.append(f"#{index}:{value!r} contains {matches}")
    return issues


def wave2_required_artifact_status_issues(required_artifacts, status_values, summary):
    issues = []
    counts = {
        "required_count": 0,
        "present_count": 0,
        "missing_count": 0,
        "release_ready_false_count": 0,
    }
    normalized_required = []
    if not isinstance(required_artifacts, list):
        issues.append("required_artifacts is not an array")
        return issues, counts
    for index, path in enumerate(required_artifacts):
        if not isinstance(path, str) or not path.strip():
            issues.append(f"required_artifacts/#{index} is not a non-empty string")
            continue
        normalized_required.append(path.strip().replace("\\", "/"))
    required_set = set(normalized_required)
    counts["required_count"] = len(normalized_required)

    if not isinstance(status_values, list):
        issues.append("required_artifact_status is not an array")
        status_values = []
    if len(status_values) != len(normalized_required):
        issues.append(
            f"required_artifact_status length {len(status_values)} does not match required_artifacts length {len(normalized_required)}"
        )

    seen_paths = set()
    for index, status in enumerate(status_values):
        status_path = f"required_artifact_status/#{index}"
        if not isinstance(status, dict):
            issues.append(f"{status_path} is not an object")
            continue
        path = status.get("path")
        normalized_path = path.strip().replace("\\", "/") if isinstance(path, str) else ""
        if not normalized_path:
            issues.append(f"{status_path}.path is not a non-empty string")
        elif normalized_path not in required_set:
            issues.append(f"{status_path}.path {path!r} is not listed in required_artifacts")
        if normalized_path:
            if normalized_path in seen_paths:
                issues.append(f"{status_path}.path {path!r} is duplicated")
            seen_paths.add(normalized_path)

        if status.get("required") is not True:
            issues.append(f"{status_path}.required is not true")
        present = status.get("present")
        if not isinstance(present, bool):
            issues.append(f"{status_path}.present is not boolean")
            present = False
        json_status = status.get("json")
        if not isinstance(json_status, bool):
            issues.append(f"{status_path}.json is not boolean")
            json_status = False
        if present:
            counts["present_count"] += 1
            if json_status is not True:
                issues.append(f"{status_path}.json is not true for a present release artifact")
            artifact_bytes = status.get("bytes")
            if not isinstance(artifact_bytes, int) or artifact_bytes <= 0:
                issues.append(f"{status_path}.bytes is not a positive integer")
            artifact_sha = status.get("sha256")
            if not isinstance(artifact_sha, str) or not artifact_sha.startswith("sha256:"):
                issues.append(f"{status_path}.sha256 is not a sha256: digest")
            command = status.get("command")
            if command is not None:
                if isinstance(command, list):
                    if not all(isinstance(item, str) and item.strip() for item in command):
                        issues.append(f"{status_path}.command array contains non-string entries")
                elif not isinstance(command, str) or not command.strip():
                    issues.append(f"{status_path}.command is not a non-empty string or string array")
            lane = status.get("lane")
            if lane is not None and (not isinstance(lane, str) or not lane.strip()):
                issues.append(f"{status_path}.lane is not a non-empty string")
            ok_value = status.get("ok")
            if ok_value is not None and not isinstance(ok_value, bool):
                issues.append(f"{status_path}.ok is not boolean")
            release_ready = status.get("release_ready")
            if release_ready is not None and not isinstance(release_ready, bool):
                issues.append(f"{status_path}.release_ready is not boolean")
            if release_ready is False:
                counts["release_ready_false_count"] += 1
        else:
            counts["missing_count"] += 1
            if json_status is not False:
                issues.append(f"{status_path}.json is not false for a missing release artifact")

    missing_status_paths = sorted(required_set - seen_paths)
    if missing_status_paths:
        issues.append(f"required_artifact_status missing paths {missing_status_paths}")

    summary_issues = wave2_artifact_summary_issues(summary, counts)
    issues.extend(f"required_artifact_summary.{issue}" for issue in summary_issues)
    return issues, counts


def wave2_artifact_summary_issues(summary, expected_counts):
    issues = []
    if not isinstance(summary, dict):
        return ["is not an object"]
    for field in ("required_count", "present_count", "missing_count", "release_ready_false_count"):
        value = summary.get(field)
        if not isinstance(value, int):
            issues.append(f"{field} is not an integer")
            continue
        expected = expected_counts.get(field)
        if value != expected:
            issues.append(f"{field} expected {expected}, actual {value}")
    all_present = summary.get("all_present")
    if not isinstance(all_present, bool):
        issues.append("all_present is not boolean")
    else:
        expected_all_present = expected_counts.get("missing_count") == 0
        if all_present != expected_all_present:
            issues.append(f"all_present expected {expected_all_present}, actual {all_present}")
    return issues


def external_plan_command_issues(commands, plan_lane):
    issues = []
    forbidden_fragments = (
        "--local-fixture",
        "--offline-fixture",
        "appfw_bitbucket_release_gate_focused_evidence=true",
        "appfw_release_evidence_mode=local-fixture",
        "release-gate-focused",
        "local-fixture",
        "fixture-artifacts",
        "placeholder",
        "<",
        ">",
        "127.0.0.1",
        "localhost",
    )
    if not isinstance(commands, list):
        return issues
    normalized_lane = plan_lane.strip() if isinstance(plan_lane, str) else ""
    for index, command in enumerate(commands):
        matches = plan_text_contains_any(command, forbidden_fragments)
        if matches:
            issues.append(f"#{index}:{command!r} contains {matches}")
            continue
        command_text = lower_text(command)
        if "local-live-preflight" in command_text:
            allowed_preflight_plan = normalized_lane == "P1-P4" and "--plan" in command_text
            if not allowed_preflight_plan:
                issues.append(
                    f"#{index}:{command!r} uses local live preflight outside the P1-P4 plan-only preparation path"
                )
    return issues


def git_tracked_path(path):
    try:
        relative_path = path.resolve(strict=False).relative_to(repo_root.resolve(strict=False))
    except ValueError:
        return False
    try:
        result = subprocess.run(
            ["git", "ls-files", "--error-unmatch", relative_path.as_posix()],
            text=True,
            capture_output=True,
            check=False,
        )
    except Exception:
        return False
    return result.returncode == 0


def validate_secret_scan_reconciliation(secret, gitleaks, secret_path, gitleaks_path, blocking):
    if not isinstance(secret, dict) or not isinstance(gitleaks, list):
        return

    baseline = secret.get("baseline")
    require(
        isinstance(baseline, dict),
        "secret baseline block is present",
        secret_path,
        blocking=blocking,
    )
    if not isinstance(baseline, dict):
        baseline = {}

    baseline_path_text = baseline.get("path")
    baseline_path_valid = isinstance(baseline_path_text, str) and bool(baseline_path_text.strip())
    require(
        baseline_path_valid,
        "secret baseline path is present",
        secret_path,
        blocking=blocking,
    )
    baseline_present = baseline.get("present")
    require(
        isinstance(baseline_present, bool),
        "secret baseline present flag is boolean",
        secret_path,
        f"actual {baseline_present!r}",
        blocking=blocking,
    )
    require(is_int(baseline.get("entries")), "secret baseline entries is an integer", secret_path, blocking=blocking)
    require(
        is_int(baseline.get("unbaselined_findings")),
        "secret baseline unbaselined findings is an integer",
        secret_path,
        blocking=blocking,
    )

    baseline_artifact = artifact_entry(secret, "gitleaks-baseline")
    require(
        isinstance(baseline_artifact, dict),
        "secret records gitleaks-baseline artifact",
        secret_path,
        blocking=blocking,
    )
    if isinstance(baseline_artifact, dict):
        artifact_path = f"{secret_path}#/artifacts/gitleaks-baseline"
        require(
            baseline_artifact.get("required") is False,
            "secret gitleaks-baseline artifact is optional",
            artifact_path,
            f"actual {baseline_artifact.get('required')!r}",
            blocking=blocking,
        )
        artifact_present = baseline_artifact.get("present")
        require(
            isinstance(artifact_present, bool),
            "secret gitleaks-baseline artifact present flag is boolean",
            artifact_path,
            f"actual {artifact_present!r}",
            blocking=blocking,
        )
        require(
            artifact_present == baseline_present,
            "secret gitleaks-baseline artifact present flag matches baseline",
            artifact_path,
            f"artifact {artifact_present!r}, baseline {baseline_present!r}",
            blocking=blocking,
        )
        artifact_baseline_path = baseline_artifact.get("path")
        require(
            isinstance(artifact_baseline_path, str) and bool(artifact_baseline_path.strip()),
            "secret gitleaks-baseline artifact records path",
            artifact_path,
            blocking=blocking,
        )
        if baseline_path_valid and isinstance(artifact_baseline_path, str):
            require(
                artifact_baseline_path.strip() == baseline_path_text.strip(),
                "secret gitleaks-baseline artifact path matches baseline path",
                artifact_path,
                f"artifact {artifact_baseline_path!r}, baseline {baseline_path_text!r}",
                blocking=blocking,
            )

    baseline_entries = []
    baseline_file = resolve_retained_path(baseline_path_text) if baseline_path_valid else None
    require(
        bool(baseline_present) == (baseline_file is not None),
        "secret baseline present flag matches retained baseline file",
        secret_path,
        (
            f"baseline present={baseline_present!r}, "
            f"resolved={baseline_file.as_posix() if baseline_file is not None else '<missing>'}"
        ),
        blocking=blocking,
    )
    if baseline_file is not None:
        require(
            path_is_within_directory(baseline_file, repo_root),
            "secret baseline file is retained under repository root",
            secret_path,
            (
                f"{baseline_file.as_posix()} is retained under {repo_root.as_posix()}"
                if path_is_within_directory(baseline_file, repo_root)
                else f"baseline file must be under {repo_root.as_posix()}: {baseline_file.as_posix()}"
            ),
            blocking=blocking,
        )
        require(
            git_tracked_path(baseline_file),
            "secret baseline file is tracked by git",
            secret_path,
            f"baseline {baseline_file.as_posix()} must be committed release policy",
            blocking=blocking,
        )
        baseline_entries, parse_error = parse_gitleaks_baseline_entries(baseline_file)
        require(
            parse_error is None,
            "secret baseline file is readable",
            baseline_file.as_posix(),
            parse_error,
            blocking=blocking,
        )

    duplicate_baseline_entries = duplicate_values(baseline_entries)
    require(
        not duplicate_baseline_entries,
        "secret baseline entries are unique",
        secret_path,
        f"duplicates={duplicate_baseline_entries}",
        blocking=blocking,
    )
    require(
        baseline.get("entries") == len(baseline_entries),
        "secret baseline entry count matches retained baseline",
        secret_path,
        f"reported {baseline.get('entries')!r}, actual {len(baseline_entries)}",
        blocking=blocking,
    )

    fingerprints = []
    missing_fingerprint_count = 0
    for finding in gitleaks:
        if not isinstance(finding, dict):
            missing_fingerprint_count += 1
            continue
        fingerprint = finding.get("Fingerprint")
        if isinstance(fingerprint, str) and fingerprint.strip():
            fingerprints.append(fingerprint.strip())
        else:
            missing_fingerprint_count += 1

    require(
        missing_fingerprint_count == 0,
        "gitleaks findings have fingerprints",
        gitleaks_path,
        f"{missing_fingerprint_count} finding(s) without Fingerprint",
        blocking=blocking,
    )
    duplicate_fingerprints = duplicate_values(fingerprints)
    require(
        not duplicate_fingerprints,
        "gitleaks finding fingerprints are unique",
        gitleaks_path,
        f"duplicates={duplicate_fingerprints}",
        blocking=blocking,
    )

    allowed_fingerprints = set(baseline_entries)
    total_findings = len(gitleaks)
    baselined_findings = sum(1 for fingerprint in fingerprints if fingerprint in allowed_fingerprints)
    unbaselined_findings = total_findings - baselined_findings

    require(
        secret.get("total_findings") == total_findings,
        "secret total findings match retained gitleaks report",
        secret_path,
        f"reported {secret.get('total_findings')!r}, actual {total_findings}",
        blocking=blocking,
    )
    require(
        secret.get("baselined_findings") == baselined_findings,
        "secret baselined findings match retained gitleaks report",
        secret_path,
        f"reported {secret.get('baselined_findings')!r}, actual {baselined_findings}",
        blocking=blocking,
    )
    require(
        secret.get("findings") == unbaselined_findings,
        "secret unbaselined findings match retained gitleaks report",
        secret_path,
        f"reported {secret.get('findings')!r}, actual {unbaselined_findings}",
        blocking=blocking,
    )
    require(
        baseline.get("unbaselined_findings") == unbaselined_findings,
        "secret baseline unbaselined count matches retained gitleaks report",
        secret_path,
        f"reported {baseline.get('unbaselined_findings')!r}, actual {unbaselined_findings}",
        blocking=blocking,
    )
    require(
        unbaselined_findings == 0,
        "secret gitleaks unbaselined findings count is zero",
        secret_path,
        f"{unbaselined_findings} unbaselined finding(s)",
        blocking=blocking,
    )
    require(
        secret.get("gitleaks_exit_code") in (0, 1),
        "secret gitleaks exit code is expected",
        secret_path,
        f"actual {secret.get('gitleaks_exit_code')!r}",
        blocking=blocking,
    )
    require(
        secret.get("ok") is not True or unbaselined_findings == 0,
        "secret ok flag matches retained gitleaks findings",
        secret_path,
        f"ok {secret.get('ok')!r}, unbaselined {unbaselined_findings}",
        blocking=blocking,
    )


def discover_tracked_files():
    try:
        result = subprocess.run(
            ["git", "ls-files"],
            text=True,
            capture_output=True,
            check=True,
        )
    except Exception:
        return []
    return [line for line in result.stdout.splitlines() if line]


def git_output(args):
    try:
        result = subprocess.run(
            ["git", *args],
            text=True,
            capture_output=True,
            check=True,
        )
    except Exception:
        return None
    return result.stdout.strip()


def frontend_lockfiles():
    return [
        path
        for path in discover_tracked_files()
        if re.search(r"(^|/)(package-lock\.json|pnpm-lock\.yaml|yarn\.lock)$", path)
        and "/node_modules/" not in f"/{path}"
    ]


def env_flag_enabled(value):
    return str(value or "").strip().lower() in {
        "1",
        "true",
        "yes",
        "y",
        "on",
        "required",
        "require",
    }


def release_mode_requires_image_sbom():
    return env_flag_enabled(os.environ.get("APPFW_REQUIRE_DEPLOYABLE_IMAGE_SBOM")) or env_flag_enabled(
        os.environ.get("APPFW_RELEASE_REQUIRE_IMAGE_SBOM")
    )


def release_mode_requires_security_assurance_decision():
    return env_flag_enabled(
        os.environ.get("APPFW_REQUIRE_SECURITY_ASSURANCE_DECISION")
    ) or env_flag_enabled(
        os.environ.get("APPFW_RELEASE_REQUIRE_SECURITY_ASSURANCE_DECISION")
    ) or env_flag_enabled(
        os.environ.get("APPFW_REQUIRE_PRODUCTION_ATTESTATIONS")
    ) or env_flag_enabled(
        os.environ.get("APPFW_RELEASE_REQUIRE_PRODUCTION_ATTESTATIONS")
    )


def release_mode_requires_live_ops_evidence():
    return env_flag_enabled(os.environ.get("APPFW_REQUIRE_LIVE_OPS_EVIDENCE")) or env_flag_enabled(
        os.environ.get("APPFW_RELEASE_REQUIRE_LIVE_OPS_EVIDENCE")
    )


def release_mode_requires_promtool():
    return env_flag_enabled(os.environ.get("APPFW_REQUIRE_PROMTOOL")) or env_flag_enabled(
        os.environ.get("APPFW_RELEASE_REQUIRE_PROMTOOL")
    )


def release_mode_requires_ops_certification():
    return env_flag_enabled(os.environ.get("APPFW_REQUIRE_OPS_CERTIFICATION")) or env_flag_enabled(
        os.environ.get("APPFW_RELEASE_REQUIRE_OPS_CERTIFICATION")
    ) or release_mode_requires_live_ops_evidence()


def release_mode_requires_pds_baseline_evidence():
    return env_flag_enabled(os.environ.get("APPFW_REQUIRE_PDS_BASELINE_EVIDENCE")) or env_flag_enabled(
        os.environ.get("APPFW_RELEASE_REQUIRE_PDS_BASELINE_EVIDENCE")
    ) or env_flag_enabled(os.environ.get("APPFW_RELEASE_REQUIRE_PDS_BASELINE"))


def release_mode_requires_release_identity():
    return env_flag_enabled(os.environ.get("APPFW_REQUIRE_RELEASE_IDENTITY")) or env_flag_enabled(
        os.environ.get("APPFW_RELEASE_REQUIRE_RELEASE_IDENTITY")
    )


def release_mode_requires_performance_evidence():
    return env_flag_enabled(os.environ.get("APPFW_REQUIRE_PERFORMANCE_EVIDENCE")) or env_flag_enabled(
        os.environ.get("APPFW_RELEASE_REQUIRE_PERFORMANCE_EVIDENCE")
    )


def release_mode_requires_chat_eval_judge_evidence():
    return env_flag_enabled(os.environ.get("APPFW_REQUIRE_CHAT_EVAL_JUDGE_EVIDENCE")) or env_flag_enabled(
        os.environ.get("APPFW_RELEASE_REQUIRE_CHAT_EVAL_JUDGE_EVIDENCE")
    )


def release_mode_requires_production_attestations():
    return env_flag_enabled(os.environ.get("APPFW_REQUIRE_PRODUCTION_ATTESTATIONS")) or env_flag_enabled(
        os.environ.get("APPFW_RELEASE_REQUIRE_PRODUCTION_ATTESTATIONS")
    )


def env_image_sources():
    sources = []
    for key in (
        "APPFW_SBOM_IMAGE_SOURCE",
        "APPFW_SBOM_IMAGE_SOURCES",
        "APPFW_SBOM_IMAGE_REF",
        "APPFW_SBOM_IMAGE_TAR",
        "APPFW_SBOM_IMAGE_ARTIFACT",
        "APPFW_DEPLOYABLE_IMAGE",
        "APPFW_IMAGE_REF",
        "APPFW_IMAGE_TAR",
        "APPFW_IMAGE_ARTIFACT",
    ):
        value = os.environ.get(key, "")
        if value:
            sources.extend(
                source.strip()
                for source in re.split(r"[\n,]+", value)
                if source.strip()
            )
    return sources


def retained_image_artifacts():
    paths = []
    for pattern in (
        "deployable-image*.tar",
        "*.image.tar",
        "*.oci",
        "*.oci.tar",
        "images/*.tar",
        "images/*.tar.gz",
        "containers/*.tar",
        "containers/*.tar.gz",
    ):
        paths.extend(report_dir.glob(pattern))
    return [path for path in paths if path.is_file()]


def image_sbom_context(supply=None, sbom_manifest=None):
    env_sources = env_image_sources()
    retained_artifacts = retained_image_artifacts()
    required_by_release_mode = release_mode_requires_image_sbom()
    supply_required = artifact_required(supply, "deployable-image-cyclonedx-sbom")
    manifest_required = artifact_required(sbom_manifest, "deployable-image-cyclonedx-sbom")

    reasons = []
    if required_by_release_mode:
        reasons.append(
            "release mode requires deployable image SBOM via "
            "APPFW_REQUIRE_DEPLOYABLE_IMAGE_SBOM or APPFW_RELEASE_REQUIRE_IMAGE_SBOM"
        )
    if env_sources:
        reasons.append(f"deployable image source env var set ({len(env_sources)} source(s))")
    if retained_artifacts:
        reasons.append(
            f"retained deployable image artifact detected ({len(retained_artifacts)} file(s))"
        )
    if supply_required:
        reasons.append("supply-chain evidence marks deployable image SBOM required")
    if manifest_required:
        reasons.append("SBOM manifest marks deployable image SBOM required")

    required = bool(reasons)
    if required:
        detail = (
            "required: "
            + "; ".join(reasons)
            + ". Expected sbom-deployable-image.cdx.json in CycloneDX JSON format; "
            "generate it with scripts/ci/supply-chain-gate.sh and syft."
        )
        applicability = "required"
    else:
        detail = (
            "not-applicable: no APPFW_SBOM_IMAGE_* / APPFW_IMAGE_* source or retained "
            "image tar/OCI artifact was found, and release mode did not require one"
        )
        applicability = "not-applicable"

    return {
        "required": required,
        "applicability": applicability,
        "detail": detail,
        "env_sources": env_sources,
        "retained_artifacts": [path.as_posix() for path in retained_artifacts],
        "required_by_release_mode": required_by_release_mode,
        "supply_required": supply_required,
        "manifest_required": manifest_required,
    }


def validate_cyclonedx(name, bom, artifact_path, require_components=True):
    require(
        bom.get("bomFormat") == "CycloneDX",
        f"{name} bomFormat is CycloneDX",
        artifact_path,
    )
    require(
        isinstance(bom.get("specVersion"), str),
        f"{name} specVersion is present",
        artifact_path,
    )
    require(
        isinstance(bom.get("version"), int),
        f"{name} version is an integer",
        artifact_path,
    )
    components = bom.get("components")
    require(
        isinstance(components, list),
        f"{name} components is an array",
        artifact_path,
    )
    if require_components and isinstance(components, list):
        require(
            bool(components),
            f"{name} has at least one component",
            artifact_path,
        )


def validate_sbom_manifest_matches_bom(
    name,
    sbom_manifest,
    artifact_name,
    bom,
    artifact_path,
    blocking,
):
    if not isinstance(sbom_manifest, dict) or not isinstance(bom, dict):
        return
    entry = artifact_entry(sbom_manifest, artifact_name)
    if not isinstance(entry, dict):
        return
    entry_path = f"{sbom_manifest_path}#/artifacts/{artifact_name}"
    require(
        entry.get("format") == "CycloneDX",
        f"SBOM manifest {artifact_name} format is CycloneDX",
        entry_path,
        f"actual {entry.get('format')!r}",
        blocking=blocking,
    )
    bom_spec_version = bom.get("specVersion")
    require(
        isinstance(entry.get("spec_version"), str)
        and entry.get("spec_version") == bom_spec_version,
        f"SBOM manifest {artifact_name} spec version matches retained SBOM",
        entry_path,
        f"manifest={entry.get('spec_version')!r}; bom={bom_spec_version!r}",
        blocking=blocking,
    )
    components = bom.get("components")
    if isinstance(components, list):
        expected_count = entry.get("component_count")
        require(
            is_int(expected_count),
            f"SBOM manifest {artifact_name} component_count is an integer",
            entry_path,
            f"actual {expected_count!r}",
            blocking=blocking,
        )
        require(
            expected_count == len(components),
            f"SBOM manifest {artifact_name} component_count matches retained SBOM",
            entry_path,
            f"manifest={expected_count!r}; actual={len(components)}; artifact={artifact_path}",
            blocking=blocking,
        )


def sorted_unique_strings(values):
    if not isinstance(values, list):
        return None
    normalized = []
    for value in values:
        if isinstance(value, str) and value.strip():
            normalized.append(value.strip())
        else:
            return None
    return sorted(dict.fromkeys(normalized))


def validate_sbom_manifest_artifact_sources(
    artifact_name,
    sbom_manifest,
    expected_sources,
    blocking,
):
    if not isinstance(sbom_manifest, dict):
        return
    entry = artifact_entry(sbom_manifest, artifact_name)
    if not isinstance(entry, dict):
        return
    entry_path = f"{sbom_manifest_path}#/artifacts/{artifact_name}"
    actual_sources = sorted_unique_strings(entry.get("sources"))
    expected_normalized = sorted_unique_strings(list(expected_sources))
    require(
        actual_sources is not None,
        f"SBOM manifest {artifact_name} sources are recorded",
        entry_path,
        f"actual {entry.get('sources')!r}",
        blocking=blocking,
    )
    require(
        actual_sources == expected_normalized,
        f"SBOM manifest {artifact_name} sources match release inputs",
        entry_path,
        f"expected={expected_normalized}; actual={actual_sources}",
        blocking=blocking,
    )


def is_redacted_secret(value):
    if value == "":
        return True
    if isinstance(value, str) and value.lower() in ("redacted", "<redacted>"):
        return True
    return isinstance(value, str) and bool(value) and set(value) <= {"*"}


def compact_values(values, limit=6):
    unique_values = []
    for value in values:
        if value is None:
            continue
        text = str(value)
        if not text or text in unique_values:
            continue
        unique_values.append(text)
    if len(unique_values) <= limit:
        return ", ".join(unique_values)
    shown = ", ".join(unique_values[:limit])
    return f"{shown}, +{len(unique_values) - limit} more"


def contract_label(contract):
    if isinstance(contract, dict):
        name = contract.get("name")
        if isinstance(name, str) and name:
            return name
        contract_path = contract.get("contract")
        if isinstance(contract_path, str) and contract_path:
            return contract_path.rsplit("::", 1)[-1]
    return "<unknown>"


def failed_security_checks(security_cert):
    checks_value = security_cert.get("checks") if isinstance(security_cert, dict) else None
    if not isinstance(checks_value, list):
        return []
    return [check for check in checks_value if isinstance(check, dict) and check.get("ok") is not True]


def summarize_security_certification(security_cert):
    if not isinstance(security_cert, dict):
        return []

    summaries = []
    if security_cert.get("ok") is not True or security_cert.get("release_ready") is not True:
        detail_parts = []
        if security_cert.get("ok") is not True:
            detail_parts.append("ok=false")
        if security_cert.get("release_ready") is not True:
            detail_parts.append("release_ready=false")
        failure_count = security_cert.get("failure_count")
        if is_int(failure_count):
            detail_parts.append(f"retained failures={failure_count}")
        detail = f" ({'; '.join(detail_parts)})" if detail_parts else ""
        append_unique(summaries, f"security-certification is not release-ready{detail}")

    checks_value = security_cert.get("checks")
    failed_checks = failed_security_checks(security_cert)
    if not isinstance(checks_value, list):
        append_unique(summaries, "security-certification checks block is missing or malformed")

    parity_provider_failures = []
    parity_failed = False
    expected_provider_shape_failed = False
    missing_log_providers = []
    for check in failed_checks:
        name = check.get("name")
        if not isinstance(name, str):
            continue
        if name == "provider parity ok flag is true":
            parity_failed = True
        elif name == "provider parity includes expected providers":
            expected_provider_shape_failed = True
        elif name.endswith(" provider parity ok"):
            append_unique(parity_provider_failures, name.removesuffix(" provider parity ok"))
        elif name.endswith(" provider certification log retained"):
            append_unique(
                missing_log_providers,
                name.removesuffix(" provider certification log retained"),
            )

    if parity_failed or parity_provider_failures or expected_provider_shape_failed:
        detail_parts = []
        if parity_provider_failures:
            detail_parts.append(
                f"failing providers: {compact_values(parity_provider_failures)}"
            )
        if expected_provider_shape_failed:
            detail_parts.append("expected provider set mismatch")
        detail = f" ({'; '.join(detail_parts)})" if detail_parts else ""
        append_unique(summaries, f"security-certification provider parity is not green{detail}")

    if missing_log_providers:
        append_unique(
            summaries,
            "security-certification retained provider logs are missing "
            f"({compact_values(missing_log_providers)})",
        )

    introspection = security_cert.get("introspection_auth")
    if not isinstance(introspection, dict):
        append_unique(
            summaries,
            "security-certification GraphQL introspection authorization block is missing or malformed",
        )
    elif isinstance(introspection, dict):
        cases = introspection.get("cases")
        if not isinstance(cases, list):
            append_unique(
                summaries,
                "security-certification GraphQL introspection authorization cases are missing or malformed",
            )
        elif isinstance(cases, list):
            failed_cases = []
            for case in cases:
                if not isinstance(case, dict) or case.get("ok") is True:
                    continue
                label = case.get("case") if isinstance(case.get("case"), str) else "<unknown>"
                details = []
                if case.get("source_present") is not True:
                    details.append("source missing")
                runtime_result = case.get("runtime_result")
                if runtime_result != "passed":
                    details.append(f"runtime={runtime_result or 'missing'}")
                if details:
                    label = f"{label} ({', '.join(details)})"
                append_unique(failed_cases, label)
            if failed_cases:
                append_unique(
                    summaries,
                    "security-certification GraphQL introspection authorization is not green "
                    f"(cases: {compact_values(failed_cases)})",
                )

    live_security = security_cert.get("live_security_contracts")
    if not isinstance(live_security, dict):
        append_unique(
            summaries,
            "security-certification live security contracts block is missing or malformed",
        )
    elif isinstance(live_security, dict):
        expected_providers = live_security.get("expected_providers")
        providers = live_security.get("providers")
        if isinstance(expected_providers, list) and isinstance(providers, list):
            seen_providers = [
                provider.get("provider")
                for provider in providers
                if isinstance(provider, dict) and isinstance(provider.get("provider"), str)
            ]
            missing_providers = [
                provider for provider in expected_providers if provider not in seen_providers
            ]
            if missing_providers:
                append_unique(
                    summaries,
                    "security-certification live provider evidence is missing "
                    f"({compact_values(missing_providers)})",
                )
        if not isinstance(providers, list):
            append_unique(
                summaries,
                "security-certification live provider evidence is missing or malformed",
            )
        elif isinstance(providers, list):
            failing_contract_providers = []
            failing_contracts = []
            for provider in providers:
                if not isinstance(provider, dict):
                    continue
                provider_name = provider.get("provider")
                if not isinstance(provider_name, str) or not provider_name:
                    provider_name = "<unknown>"
                contracts = provider.get("contracts")
                if not isinstance(contracts, list):
                    append_unique(failing_contract_providers, provider_name)
                    append_unique(failing_contracts, "contracts block missing")
                    continue
                provider_has_failing_contract = False
                for contract in contracts:
                    if not isinstance(contract, dict):
                        provider_has_failing_contract = True
                        append_unique(failing_contracts, "malformed contract")
                        continue
                    result = contract.get("result")
                    area = contract.get("area")
                    area_result = contract.get("area_result")
                    area_ok = area is None or area_result == "passed"
                    if contract.get("ok") is not True or result != "passed" or not area_ok:
                        provider_has_failing_contract = True
                        append_unique(failing_contracts, contract_label(contract))
                if provider_has_failing_contract:
                    append_unique(failing_contract_providers, provider_name)
            if failing_contract_providers:
                append_unique(
                    summaries,
                    "security-certification provider security contracts are not green "
                    f"(providers: {compact_values(failing_contract_providers)}; "
                    f"contracts: {compact_values(failing_contracts)})",
                )

    if not summaries:
        failed_names = [
            format_failure(check.get("name", "<unnamed>"), check.get("detail"))
            for check in failed_checks
        ]
        if failed_names:
            append_unique(
                summaries,
                "security-certification retained checks failed "
                f"({compact_values(failed_names)})",
            )

    return summaries


def summarize_provider_parity(provider_parity):
    if not isinstance(provider_parity, dict):
        return []

    summaries = []
    if provider_parity.get("ok") is not True:
        append_unique(summaries, "provider-parity is not release-ready (ok=false)")

    providers = provider_parity.get("providers")
    if not isinstance(providers, list):
        append_unique(summaries, "provider-parity providers block is missing or malformed")
        return summaries

    providers_by_name = {
        provider.get("provider"): provider
        for provider in providers
        if isinstance(provider, dict) and isinstance(provider.get("provider"), str)
    }
    missing_providers = [
        provider for provider in SECURITY_EXPECTED_PROVIDERS if provider not in providers_by_name
    ]
    extra_providers = [
        provider for provider in providers_by_name if provider not in SECURITY_EXPECTED_PROVIDERS
    ]
    detail_parts = []
    if missing_providers:
        detail_parts.append(f"missing providers: {compact_values(missing_providers)}")
    if extra_providers:
        detail_parts.append(f"unexpected providers: {compact_values(extra_providers)}")

    failing_providers = []
    non_full_providers = []
    missing_url_providers = []
    missing_log_providers = []
    missing_area_providers = []
    for provider_name in SECURITY_EXPECTED_PROVIDERS:
        provider = providers_by_name.get(provider_name)
        if not isinstance(provider, dict):
            continue
        if provider.get("ok") is not True:
            append_unique(failing_providers, provider_name)
        if provider.get("mode") != "full":
            append_unique(non_full_providers, provider_name)
        if redacted_base_url_from_text(provider.get("base_url")) is None:
            append_unique(missing_url_providers, provider_name)
        if not (isinstance(provider.get("log"), str) and provider.get("log").strip()):
            append_unique(missing_log_providers, provider_name)
        areas = provider.get("areas")
        if not isinstance(areas, list) or not areas:
            append_unique(missing_area_providers, provider_name)

    if failing_providers:
        detail_parts.append(f"failing providers: {compact_values(failing_providers)}")
    if non_full_providers:
        detail_parts.append(f"non-full runs: {compact_values(non_full_providers)}")
    if missing_url_providers:
        detail_parts.append(f"missing safe base URLs: {compact_values(missing_url_providers)}")
    if missing_log_providers:
        detail_parts.append(f"missing logs: {compact_values(missing_log_providers)}")
    if missing_area_providers:
        detail_parts.append(f"missing certified areas: {compact_values(missing_area_providers)}")

    if detail_parts:
        append_unique(
            summaries,
            "provider-parity retained evidence is not green "
            f"({'; '.join(detail_parts)})",
        )

    return summaries


def is_security_certification_failure(failure):
    if not isinstance(failure, str):
        return False
    if failure.startswith(
        (
            "security certification ok flag is true",
            "security certification is release-ready",
            "security certification check ok",
        )
    ):
        return True
    if failure.startswith("introspection case "):
        return True
    if " live security provider ok" in failure:
        return True
    if " live security contract " in failure and (
        failure.endswith(" passed")
        or " passed:" in failure
        or failure.endswith(" ok")
        or " ok:" in failure
    ):
        return True
    parity_provider_suffixes = (
        " provider parity ok",
        " provider parity ok:",
    )
    if any(failure.startswith(f"{provider}{suffix}") for provider in SECURITY_EXPECTED_PROVIDERS for suffix in parity_provider_suffixes):
        return True
    return False


def is_provider_parity_failure(failure):
    if not isinstance(failure, str):
        return False
    if failure.startswith("provider parity "):
        return True
    return any(
        failure.startswith(f"{provider} provider parity ")
        for provider in SECURITY_EXPECTED_PROVIDERS
    )


def summarize_display_failures(failure_items, security_summaries, provider_summaries):
    if not security_summaries and not provider_summaries:
        return list(failure_items)
    filtered = []
    suppressed_security_failures = 0
    suppressed_provider_failures = 0
    for failure in failure_items:
        if security_summaries and is_security_certification_failure(failure):
            suppressed_security_failures += 1
        elif provider_summaries and is_provider_parity_failure(failure):
            suppressed_provider_failures += 1
        else:
            filtered.append(failure)
    if suppressed_provider_failures:
        filtered.extend(provider_summaries)
        filtered.append(
            "provider-parity detailed assertion failures retained in JSON "
            f"({suppressed_provider_failures} release-evidence assertion(s))"
        )
    if suppressed_security_failures == 0:
        return filtered
    filtered.extend(security_summaries)
    filtered.append(
        "security-certification detailed assertion failures retained in JSON "
        f"({suppressed_security_failures} release-evidence assertion(s))"
    )
    return filtered


SECURITY_EXPECTED_PROVIDERS = ["postgres", "mongo", "mssql", "snowflake"]
PERFORMANCE_EXPECTED_PROVIDERS = ["postgres", "mongo", "mssql", "snowflake"]
EXPECTED_FRAMEWORK_PROVIDERS = [
    "postgres",
    "mongo",
    "mssql",
    "fabric_sql_analytics",
    "snowflake",
    "neo4j",
    "servicenow",
    "workday",
    "icims",
    "salesforce",
    "anaplan",
    "oracle_financials",
    "ai_search",
]
EXTERNAL_API_EXPECTED_PROVIDERS = [
    "servicenow",
    "workday",
    "icims",
    "salesforce",
    "anaplan",
    "oracle_financials",
]
EXTERNAL_API_WORKSPACE_CRATES = {
    "servicenow": "appfw-provider-servicenow",
    "workday": "appfw-provider-workday",
    "icims": "appfw-provider-icims",
    "salesforce": "appfw-provider-salesforce",
    "anaplan": "appfw-provider-anaplan",
    "oracle_financials": "appfw-provider-oracle-financials",
}
GOVERNED_WRITE_AREAS = [
    "governed_write_enforcement",
    "delegated_actor_context",
    "token_store_isolation",
    "named_mutation_registry",
    "mutation_request_binding",
    "idempotency_and_replay_protection",
    "write_policy_and_scope_enforcement",
    "write_audit_and_evidence",
]
SECURITY_INTROSPECTION_CASES = [
    "no_auth_disabled_or_missing_user",
    "fake_bearer_or_unprivileged_user",
    "admin_or_developer_scope",
]
SECURITY_EXPECTED_CONTRACTS = [
    "provider_semantic_contracts::provider_tenant_isolation_contract",
    "provider_semantic_contracts::provider_locator_tenant_isolation_contract",
    "provider_semantic_contracts::provider_access_filter_policy_cannot_widen_user_filter_contract",
    "provider_semantic_contracts::provider_error_normalization_contract",
    "provider_semantic_contracts::provider_audit_append_redaction_chain_contract",
]
PROVIDER_AREA_ALLOWED_STATUSES = {
    "live-certified",
    "compiler-contracted",
    "implemented",
    "partial",
    "unsupported",
    "emulator-limited",
}
SECURITY_INTROSPECTION_ROUTING_SOURCE = repo_root / "appfw_runtime" / "src" / "routing.rs"


def expected_provider_base_url_env(provider):
    if not isinstance(provider, str):
        return ""
    return f"API_TEST_BASE_URL_{provider.strip().upper()}"


def redacted_base_url_from_text(value):
    if not isinstance(value, str) or not value.strip():
        return None
    try:
        parts = urlsplit(value.strip())
        host = parts.hostname
        port = parts.port
    except ValueError:
        return None
    if parts.scheme.lower() not in {"http", "https"} or not host:
        return None
    if parts.username or parts.password or parts.query or parts.fragment:
        return None
    netloc_host = host
    if ":" in netloc_host and not netloc_host.startswith("["):
        netloc_host = f"[{netloc_host}]"
    netloc = netloc_host if port is None else f"{netloc_host}:{port}"
    return urlunsplit((parts.scheme.lower(), netloc, parts.path.rstrip("/"), "", ""))


def validate_provider_parity_report(
    provider_parity,
    provider_parity_path,
    expected_preflight_base_urls,
    blocking,
):
    provider_logs_by_name = {}
    generated_at = None
    if not isinstance(expected_preflight_base_urls, dict):
        expected_preflight_base_urls = {}
    require(
        isinstance(provider_parity, dict),
        "provider parity report is object",
        provider_parity_path,
        blocking=blocking,
    )
    if not isinstance(provider_parity, dict):
        return {"generated_at": generated_at, "provider_logs_by_name": provider_logs_by_name}
    require(
        provider_parity.get("command") == "provider-test",
        "provider parity command is stable",
        provider_parity_path,
        blocking=blocking,
    )
    require(
        provider_parity.get("ok") is True,
        "provider parity ok flag is true",
        provider_parity_path,
        blocking=blocking,
    )
    now = dt.datetime.now(dt.timezone.utc)
    generated_at = parse_timestamp_with_timezone(provider_parity.get("generated_at_utc"))
    require(
        generated_at is not None,
        "provider parity generated timestamp is valid",
        provider_parity_path,
        "generated_at_utc must be ISO-8601 with timezone",
        blocking=blocking,
    )
    if generated_at is not None:
        require(
            generated_at <= now + dt.timedelta(minutes=5),
            "provider parity generated timestamp is not in the future",
            provider_parity_path,
            generated_at.isoformat().replace("+00:00", "Z"),
            blocking=blocking,
        )
        require(
            generated_at >= now - PROVIDER_PARITY_MAX_AGE,
            "provider parity generated timestamp is recent",
            provider_parity_path,
            (
                f"generated_at_utc {generated_at.isoformat().replace('+00:00', 'Z')} "
                f"is older than {PROVIDER_PARITY_MAX_AGE_HOURS} hours"
            ),
            blocking=blocking,
        )

    providers = provider_parity.get("providers")
    require(
        isinstance(providers, list) and bool(providers),
        "provider parity providers are recorded",
        provider_parity_path,
        blocking=blocking,
    )
    if not isinstance(providers, list):
        return {"generated_at": generated_at, "provider_logs_by_name": provider_logs_by_name}
    provider_names = []
    for index, provider in enumerate(providers):
        entry_path = f"{provider_parity_path}#/providers/{index}"
        require(
            isinstance(provider, dict),
            "provider parity provider entry is object",
            entry_path,
            blocking=blocking,
        )
        if not isinstance(provider, dict):
            continue
        provider_name = provider.get("provider")
        provider_name_ok = (
            isinstance(provider_name, str)
            and provider_name.strip() in SECURITY_EXPECTED_PROVIDERS
        )
        require(
            provider_name_ok,
            "provider parity provider entry names release-certified provider",
            entry_path,
            f"actual {provider_name!r}",
            blocking=blocking,
        )
        if provider_name_ok:
            provider_names.append(provider_name.strip())
    duplicate_provider_names = sorted(
        provider_name
        for provider_name in set(provider_names)
        if provider_names.count(provider_name) > 1
    )
    require(
        not duplicate_provider_names,
        "provider parity providers name each provider once",
        provider_parity_path,
        f"duplicates={duplicate_provider_names}",
        blocking=blocking,
    )
    providers_by_name = {
        provider.get("provider"): provider
        for provider in providers
        if isinstance(provider, dict) and isinstance(provider.get("provider"), str)
    }
    actual_providers = set(providers_by_name)
    expected_providers = set(SECURITY_EXPECTED_PROVIDERS)
    require(
        actual_providers == expected_providers,
        "provider parity includes release-certified providers",
        provider_parity_path,
        (
            f"missing={sorted(expected_providers - actual_providers)}; "
            f"extra={sorted(actual_providers - expected_providers)}"
        ),
        blocking=blocking,
    )

    for provider_name in SECURITY_EXPECTED_PROVIDERS:
        provider = providers_by_name.get(provider_name)
        provider_path = f"{provider_parity_path}#/providers/{provider_name}"
        require(
            isinstance(provider, dict),
            f"{provider_name} provider parity entry is present",
            provider_path,
            blocking=blocking,
        )
        if not isinstance(provider, dict):
            continue
        require(
            provider.get("ok") is True,
            f"{provider_name} provider parity ok",
            provider_path,
            blocking=blocking,
        )
        require(
            provider.get("mode") == "full",
            f"{provider_name} provider parity run is full mode",
            provider_path,
            f"actual {provider.get('mode')!r}",
            blocking=blocking,
        )
        redacted_base_url = redacted_base_url_from_text(provider.get("base_url"))
        require(
            redacted_base_url is not None,
            f"{provider_name} provider parity base URL is safe HTTP",
            provider_path,
            f"actual {provider.get('base_url')!r}",
            blocking=blocking,
        )
        expected_base_url = expected_preflight_base_urls.get(provider_name)
        if expected_base_url:
            require(
                redacted_base_url == expected_base_url,
                f"{provider_name} provider parity base URL matches provider preflight",
                provider_path,
                f"expected {expected_base_url}, actual {redacted_base_url}",
                blocking=blocking,
            )

        log_value = provider.get("log")
        log_is_path = isinstance(log_value, str) and bool(log_value.strip())
        require(
            log_is_path,
            f"{provider_name} provider parity log path is recorded",
            provider_path,
            blocking=blocking,
        )
        resolved_log = resolve_retained_path(log_value) if log_is_path else None
        require(
            resolved_log is not None,
            f"{provider_name} provider parity log exists",
            provider_path,
            f"missing retained log: {log_value}",
            blocking=blocking,
        )
        if resolved_log is not None:
            provider_logs_by_name[provider_name] = resolved_log
            require(
                path_is_within_directory(resolved_log, report_dir),
                f"{provider_name} provider parity log is retained in report directory",
                provider_path,
                (
                    f"actual {resolved_log.as_posix()}, "
                    f"report_dir {report_dir.resolve(strict=False).as_posix()}"
                ),
                blocking=blocking,
            )
            require(
                resolved_log.stat().st_size > 0,
                f"{provider_name} provider parity log is non-empty",
                provider_path,
                blocking=blocking,
            )

        areas = provider.get("areas")
        require(
            isinstance(areas, list) and bool(areas),
            f"{provider_name} provider parity areas are recorded",
            provider_path,
            blocking=blocking,
        )
        if isinstance(areas, list):
            area_names = [
                area.get("area").strip()
                for area in areas
                if isinstance(area, dict)
                and isinstance(area.get("area"), str)
                and area.get("area").strip()
            ]
            duplicate_area_names = sorted(
                area_name
                for area_name in set(area_names)
                if area_names.count(area_name) > 1
            )
            require(
                not duplicate_area_names,
                f"{provider_name} provider parity areas name each area once",
                provider_path,
                f"duplicates={duplicate_area_names}",
                blocking=blocking,
            )
            for index, area in enumerate(areas):
                area_path = f"{provider_path}/areas/{index}"
                require(
                    isinstance(area, dict),
                    f"{provider_name} provider parity area is object",
                    area_path,
                    blocking=blocking,
                )
                if not isinstance(area, dict):
                    continue
                area_name_value = area.get("area")
                area_name_ok = isinstance(area_name_value, str) and bool(area_name_value.strip())
                area_name = area_name_value.strip() if area_name_ok else "<unknown>"
                require(
                    area_name_ok,
                    f"{provider_name} provider parity area has name",
                    area_path,
                    f"actual {area_name_value!r}",
                    blocking=blocking,
                )
                status_value = area.get("status")
                status_ok = (
                    isinstance(status_value, str)
                    and status_value.strip() in PROVIDER_AREA_ALLOWED_STATUSES
                )
                require(
                    status_ok,
                    f"{provider_name} provider parity {area_name} area status is known",
                    area_path,
                    f"actual {status_value!r}",
                    blocking=blocking,
                )
                require(
                    area.get("ok") is True,
                    f"{provider_name} provider parity {area_name} area ok",
                    area_path,
                    blocking=blocking,
                )
                if area.get("status") == "live-certified" or area.get("required_live_result") == "passed":
                    require(
                        area.get("live_result") == "passed",
                        f"{provider_name} provider parity {area_name} live result passed",
                        area_path,
                        f"actual {area.get('live_result')!r}",
                        blocking=blocking,
                    )
                    require(
                        isinstance(area.get("live_contracts"), list)
                        and bool(area.get("live_contracts")),
                        f"{provider_name} provider parity {area_name} live contracts are recorded",
                        area_path,
                        blocking=blocking,
                    )
    return {"generated_at": generated_at, "provider_logs_by_name": provider_logs_by_name}


def validate_provider_graduation_report(provider_graduation, provider_graduation_path, blocking):
    require(
        isinstance(provider_graduation, dict),
        "provider graduation report is object",
        provider_graduation_path,
        blocking=blocking,
    )
    if not isinstance(provider_graduation, dict):
        return
    require(
        provider_graduation.get("command") == "provider-graduation",
        "provider graduation command is provider-graduation",
        provider_graduation_path,
        f"actual {provider_graduation.get('command')!r}",
        blocking=blocking,
    )
    require(
        provider_graduation.get("lane") == "U4",
        "provider graduation lane is U4",
        provider_graduation_path,
        f"actual {provider_graduation.get('lane')!r}",
        blocking=blocking,
    )
    require(
        provider_graduation.get("mode") == "report-only",
        "provider graduation mode is report-only",
        provider_graduation_path,
        f"actual {provider_graduation.get('mode')!r}",
        blocking=blocking,
    )
    require(
        provider_graduation.get("ok") is True,
        "provider graduation ok flag is true",
        provider_graduation_path,
        blocking=blocking,
    )
    generated_at = parse_timestamp_with_timezone(provider_graduation.get("generated_at_utc"))
    require(
        generated_at is not None,
        "provider graduation generated_at_utc has timezone",
        provider_graduation_path,
        f"actual {provider_graduation.get('generated_at_utc')!r}",
        blocking=blocking,
    )

    providers = provider_graduation.get("providers")
    require(
        isinstance(providers, list),
        "provider graduation providers block is list",
        provider_graduation_path,
        blocking=blocking,
    )
    if not isinstance(providers, list):
        return

    provider_names = []
    calculated = {
        "provider_count": len(providers),
        "unsupported_capability_count": 0,
        "ungraduated_capability_count": 0,
        "graduated_capability_count": 0,
        "promotion_violation_count": 0,
    }
    for index, provider in enumerate(providers):
        provider_path = f"{provider_graduation_path}#/providers/{index}"
        require(
            isinstance(provider, dict),
            "provider graduation provider entry is object",
            provider_path,
            blocking=blocking,
        )
        if not isinstance(provider, dict):
            continue
        name = provider.get("provider")
        require(
            isinstance(name, str) and name in EXPECTED_FRAMEWORK_PROVIDERS,
            "provider graduation entry names framework provider",
            provider_path,
            f"actual {name!r}",
            blocking=blocking,
        )
        if isinstance(name, str):
            provider_names.append(name)
        family = provider.get("family")
        require(
            family in {"database", "graph_read", "external_api", "ai_search"},
            "provider graduation entry records provider family",
            provider_path,
            f"actual {family!r}",
            blocking=blocking,
        )
        profile = provider.get("profile")
        require(
            profile in {"semantic_parity", "graph_read", "saas_read", "ai_search"},
            "provider graduation entry records profile",
            provider_path,
            f"actual {profile!r}",
            blocking=blocking,
        )
        require(
            provider.get("ok") is True,
            "provider graduation provider entry ok flag is true",
            provider_path,
            blocking=blocking,
        )
        for field in [
            "unsupported_capability_count",
            "ungraduated_capability_count",
            "graduated_capability_count",
            "promotion_violation_count",
        ]:
            value = provider.get(field)
            require(
                isinstance(value, int) and value >= 0,
                f"provider graduation {field} is non-negative integer",
                provider_path,
                f"actual {value!r}",
                blocking=blocking,
            )
            if isinstance(value, int) and value >= 0:
                calculated[field] += value
        areas = provider.get("areas")
        require(
            isinstance(areas, list),
            "provider graduation provider areas block is list",
            provider_path,
            blocking=blocking,
        )
        if not isinstance(areas, list):
            continue
        for area_index, area in enumerate(areas):
            area_path = f"{provider_path}/areas/{area_index}"
            require(
                isinstance(area, dict),
                "provider graduation area entry is object",
                area_path,
                blocking=blocking,
            )
            if not isinstance(area, dict):
                continue
            require(
                isinstance(area.get("area"), str) and bool(area.get("area").strip()),
                "provider graduation area has key",
                area_path,
                blocking=blocking,
            )
            require(
                isinstance(area.get("label"), str) and bool(area.get("label").strip()),
                "provider graduation area has label",
                area_path,
                blocking=blocking,
            )
            require(
                area.get("status") in PROVIDER_AREA_ALLOWED_STATUSES,
                "provider graduation area status is known",
                area_path,
                f"actual {area.get('status')!r}",
                blocking=blocking,
            )
            require(
                isinstance(area.get("graduated"), bool),
                "provider graduation area records graduated flag",
                area_path,
                f"actual {area.get('graduated')!r}",
                blocking=blocking,
            )
            require(
                isinstance(area.get("ok"), bool),
                "provider graduation area records ok flag",
                area_path,
                f"actual {area.get('ok')!r}",
                blocking=blocking,
            )
            require(
                isinstance(area.get("promotion_violation"), bool),
                "provider graduation area records promotion_violation flag",
                area_path,
                f"actual {area.get('promotion_violation')!r}",
                blocking=blocking,
            )
            require(
                isinstance(area.get("compiler_contracts"), list),
                "provider graduation area records compiler contracts",
                area_path,
                blocking=blocking,
            )
            require(
                isinstance(area.get("live_contracts"), list),
                "provider graduation area records live contracts",
                area_path,
                blocking=blocking,
            )
            require(
                isinstance(area.get("required_evidence"), list),
                "provider graduation area records required evidence",
                area_path,
                blocking=blocking,
            )

    duplicate_names = duplicate_values(provider_names)
    require(
        not duplicate_names,
        "provider graduation names each provider once",
        provider_graduation_path,
        f"duplicates={duplicate_names}",
        blocking=blocking,
    )
    require(
        provider_names == EXPECTED_FRAMEWORK_PROVIDERS,
        "provider graduation provider list is stable",
        provider_graduation_path,
        f"actual {provider_names!r}",
        blocking=blocking,
    )

    summary = provider_graduation.get("summary")
    require(
        isinstance(summary, dict),
        "provider graduation summary is object",
        provider_graduation_path,
        blocking=blocking,
    )
    if isinstance(summary, dict):
        for field, calculated_value in calculated.items():
            require(
                summary.get(field) == calculated_value,
                f"provider graduation summary {field} matches provider totals",
                provider_graduation_path,
                f"expected {calculated_value}, actual {summary.get(field)!r}",
                blocking=blocking,
            )


def validate_governed_write_evidence(evidence, evidence_path, claimed_providers, blocking):
    require(
        isinstance(evidence, dict),
        "governed-write evidence is object",
        evidence_path,
        blocking=blocking,
    )
    if not isinstance(evidence, dict):
        return set()
    require(
        evidence.get("command") == "provider-test",
        "governed-write evidence command is provider-test",
        evidence_path,
        f"actual {evidence.get('command')!r}",
        blocking=blocking,
    )
    require(
        evidence.get("lane") == "G1",
        "governed-write evidence lane is G1",
        evidence_path,
        f"actual {evidence.get('lane')!r}",
        blocking=blocking,
    )
    require(
        evidence.get("ok") is True,
        "governed-write evidence ok flag is true",
        evidence_path,
        blocking=blocking,
    )
    require(
        evidence.get("release_ready") is True,
        "governed-write evidence release_ready flag is true",
        evidence_path,
        f"actual {evidence.get('release_ready')!r}",
        blocking=blocking,
    )
    provider = evidence.get("provider")
    provider_ok = isinstance(provider, str) and provider.strip() in EXTERNAL_API_EXPECTED_PROVIDERS
    require(
        provider_ok,
        "governed-write evidence names an external API provider",
        evidence_path,
        f"actual {provider!r}",
        blocking=blocking,
    )
    evidence_providers = {provider.strip()} if provider_ok else set()
    if claimed_providers:
        missing = sorted(set(claimed_providers) - evidence_providers)
        require(
            not missing,
            "governed-write evidence covers every certified write provider",
            evidence_path,
            f"missing={missing}",
            blocking=blocking,
        )
    require(
        evidence.get("operation") == "named_mutation",
        "governed-write evidence operation is named_mutation",
        evidence_path,
        f"actual {evidence.get('operation')!r}",
        blocking=blocking,
    )

    delegated_actor = evidence.get("delegated_actor_context")
    delegated_path = f"{evidence_path}#/delegated_actor_context"
    require(
        isinstance(delegated_actor, dict),
        "governed-write evidence records delegated actor context",
        delegated_path,
        blocking=blocking,
    )
    if isinstance(delegated_actor, dict):
        require(
            delegated_actor.get("principal_type") == "user",
            "governed-write delegated actor is a user principal",
            delegated_path,
            f"actual {delegated_actor.get('principal_type')!r}",
            blocking=blocking,
        )
        for field in ["tenant", "on_behalf_of"]:
            require(
                isinstance(delegated_actor.get(field), str) and bool(delegated_actor.get(field).strip()),
                f"governed-write delegated actor records {field}",
                delegated_path,
                blocking=blocking,
            )

    token_store = evidence.get("token_store_isolation")
    token_store_path = f"{evidence_path}#/token_store_isolation"
    require(
        isinstance(token_store, dict),
        "governed-write evidence records token-store isolation",
        token_store_path,
        blocking=blocking,
    )
    if isinstance(token_store, dict):
        partition_key = token_store.get("partition_key")
        require(
            isinstance(partition_key, list)
            and all(value in partition_key for value in ["user", "tenant", "provider"]),
            "governed-write token store is partitioned by user tenant provider",
            token_store_path,
            f"actual {partition_key!r}",
            blocking=blocking,
        )
        require(
            token_store.get("revocation_checked") is True,
            "governed-write token revocation is checked",
            token_store_path,
            blocking=blocking,
        )

    mutation_registry = evidence.get("mutation_registry")
    mutation_registry_path = f"{evidence_path}#/mutation_registry"
    require(
        isinstance(mutation_registry, dict),
        "governed-write evidence records mutation registry",
        mutation_registry_path,
        blocking=blocking,
    )
    if isinstance(mutation_registry, dict):
        name = mutation_registry.get("name")
        policy_scope = mutation_registry.get("policy_scope")
        require(
            isinstance(name, str) and "." in name and bool(name.strip()),
            "governed-write mutation registry uses a named mutation",
            mutation_registry_path,
            f"actual {name!r}",
            blocking=blocking,
        )
        require(
            mutation_registry.get("mcp_enabled") is False,
            "governed-write mutation keeps MCP disabled until certified",
            mutation_registry_path,
            f"actual {mutation_registry.get('mcp_enabled')!r}",
            blocking=blocking,
        )
        require(
            isinstance(policy_scope, str)
            and bool(policy_scope.strip()),
            "governed-write mutation registry records policy scope",
            mutation_registry_path,
            blocking=blocking,
        )
        allowed_mutations = {
            "servicenow": {
                "servicenow.create_incident": "servicenow.incident.write",
            },
        }
        provider_mutations = allowed_mutations.get(provider, {})
        if provider_mutations:
            require(
                name in provider_mutations,
                "governed-write mutation registry uses provider allow-listed mutation",
                mutation_registry_path,
                f"actual {name!r}; allowed={sorted(provider_mutations)}",
                blocking=blocking,
            )
            if name in provider_mutations:
                require(
                    policy_scope == provider_mutations[name],
                    "governed-write mutation registry policy scope matches provider allow-list",
                    mutation_registry_path,
                    f"actual {policy_scope!r}; expected {provider_mutations[name]!r}",
                    blocking=blocking,
                )

    request_binding = evidence.get("request_binding")
    request_binding_path = f"{evidence_path}#/request_binding"
    require(
        isinstance(request_binding, dict),
        "governed-write evidence records request binding",
        request_binding_path,
        blocking=blocking,
    )
    if isinstance(request_binding, dict):
        require(
            request_binding.get("provider_owned_request") is True,
            "governed-write request binding is provider-owned",
            request_binding_path,
            f"actual {request_binding.get('provider_owned_request')!r}",
            blocking=blocking,
        )
        require(
            request_binding.get("raw_url_or_query_from_caller") is False,
            "governed-write request binding rejects caller raw URL/query",
            request_binding_path,
            f"actual {request_binding.get('raw_url_or_query_from_caller')!r}",
            blocking=blocking,
        )

    idempotency = evidence.get("idempotency")
    idempotency_path = f"{evidence_path}#/idempotency"
    require(
        isinstance(idempotency, dict),
        "governed-write evidence records idempotency",
        idempotency_path,
        blocking=blocking,
    )
    if isinstance(idempotency, dict):
        require(
            isinstance(idempotency.get("key_source"), str)
            and bool(idempotency.get("key_source").strip()),
            "governed-write idempotency records key source",
            idempotency_path,
            blocking=blocking,
        )
        require(
            idempotency.get("replay_rejected") is True,
            "governed-write idempotency rejects replay",
            idempotency_path,
            blocking=blocking,
        )

    policy = evidence.get("policy")
    policy_path = f"{evidence_path}#/policy"
    require(
        isinstance(policy, dict),
        "governed-write evidence records policy decision",
        policy_path,
        blocking=blocking,
    )
    if isinstance(policy, dict):
        require(
            policy.get("scope_enforced") is True,
            "governed-write policy enforces scope",
            policy_path,
            f"actual {policy.get('scope_enforced')!r}",
            blocking=blocking,
        )
        require(
            policy.get("decision") == "allow",
            "governed-write policy decision is allow",
            policy_path,
            f"actual {policy.get('decision')!r}",
            blocking=blocking,
        )

    audit = evidence.get("audit")
    audit_path = f"{evidence_path}#/audit"
    require(
        isinstance(audit, dict),
        "governed-write evidence records audit",
        audit_path,
        blocking=blocking,
    )
    if isinstance(audit, dict):
        require(
            audit.get("source") in {"http", "mcp", "kafka"},
            "governed-write audit records ingress source",
            audit_path,
            f"actual {audit.get('source')!r}",
            blocking=blocking,
        )
        for field in ["correlation_id", "provider_request_id", "sink"]:
            require(
                isinstance(audit.get(field), str) and bool(audit.get(field).strip()),
                f"governed-write audit records {field}",
                audit_path,
                blocking=blocking,
            )

    live_provider_call = evidence.get("live_provider_call")
    live_provider_call_path = f"{evidence_path}#/live_provider_call"
    require(
        isinstance(live_provider_call, dict),
        "governed-write evidence records live provider call",
        live_provider_call_path,
        blocking=blocking,
    )
    if isinstance(live_provider_call, dict):
        require(
            live_provider_call.get("executed") is True,
            "governed-write live provider call executed",
            live_provider_call_path,
            f"actual {live_provider_call.get('executed')!r}",
            blocking=blocking,
        )
        require(
            live_provider_call.get("non_production") is True,
            "governed-write live provider call is non-production",
            live_provider_call_path,
            f"actual {live_provider_call.get('non_production')!r}",
            blocking=blocking,
        )
        require(
            live_provider_call.get("result") == "success",
            "governed-write live provider call succeeded",
            live_provider_call_path,
            f"actual {live_provider_call.get('result')!r}",
            blocking=blocking,
        )
        require(
            live_provider_call.get("replay_attempted") is True,
            "governed-write live provider call attempted replay",
            live_provider_call_path,
            f"actual {live_provider_call.get('replay_attempted')!r}",
            blocking=blocking,
        )
        if isinstance(mutation_registry, dict) and isinstance(mutation_registry.get("name"), str):
            require(
                live_provider_call.get("mutation_name") == mutation_registry.get("name"),
                "governed-write live provider call mutation matches registry",
                live_provider_call_path,
                f"actual {live_provider_call.get('mutation_name')!r}; expected {mutation_registry.get('name')!r}",
                blocking=blocking,
            )
    return evidence_providers


def validate_chat_eval_artifact_items(report, report_path, blocking):
    artifact_items = report.get("artifacts")
    require(
        isinstance(artifact_items, list) and bool(artifact_items),
        "chat-eval records sub-artifacts",
        report_path,
        blocking=blocking,
    )
    if not isinstance(artifact_items, list):
        return
    for index, item in enumerate(artifact_items):
        item_path = f"{report_path}#/artifacts/{index}"
        require(isinstance(item, dict), "chat-eval sub-artifact is object", item_path, blocking=blocking)
        if not isinstance(item, dict):
            continue
        rel_path = item.get("path")
        require(
            isinstance(rel_path, str) and bool(rel_path.strip()),
            "chat-eval sub-artifact records path",
            item_path,
            blocking=blocking,
        )
        expected_path = repo_root / rel_path if isinstance(rel_path, str) else None
        present = expected_path.is_file() if expected_path is not None else False
        require(
            item.get("present") is True and present,
            "chat-eval sub-artifact exists",
            item_path,
            rel_path,
            blocking=blocking,
        )
        require(
            isinstance(item.get("sha256"), str) and item["sha256"].startswith("sha256:"),
            "chat-eval sub-artifact records sha256",
            item_path,
            blocking=blocking,
        )
        require(
            isinstance(item.get("bytes"), int) and item["bytes"] >= 0,
            "chat-eval sub-artifact records byte size",
            item_path,
            blocking=blocking,
        )
        if present and isinstance(item.get("sha256"), str):
            actual_sha = f"sha256:{sha256_digest(expected_path)}"
            require(
                item.get("sha256") == actual_sha,
                "chat-eval sub-artifact sha256 matches retained file",
                item_path,
                f"expected {actual_sha}, actual {item.get('sha256')}",
                blocking=blocking,
            )
        if present and isinstance(item.get("bytes"), int):
            actual_size = expected_path.stat().st_size
            require(
                item.get("bytes") == actual_size,
                "chat-eval sub-artifact byte size matches retained file",
                item_path,
                f"expected {actual_size}, actual {item.get('bytes')}",
                blocking=blocking,
            )


def validate_chat_eval_report(report, report_path, blocking):
    require(isinstance(report, dict), "chat-eval evidence is object", report_path, blocking=blocking)
    if not isinstance(report, dict):
        return False

    validate_generated_timestamp(
        "chat-eval",
        report,
        report_path,
        CHAT_EVAL_MAX_AGE,
        CHAT_EVAL_MAX_AGE_HOURS,
        blocking,
    )
    require(
        report.get("command") == "chat-eval",
        "chat-eval command is stable",
        report_path,
        f"actual {report.get('command')!r}",
        blocking=blocking,
    )
    require(
        report.get("lane") == "CH",
        "chat-eval lane is CH",
        report_path,
        f"actual {report.get('lane')!r}",
        blocking=blocking,
    )
    require(
        report.get("ok") is True,
        "chat-eval ok flag is true",
        report_path,
        f"actual {report.get('ok')!r}",
        blocking=blocking,
    )
    require(
        report.get("mode") in {"local-fixture", "pipeline-fixture"},
        "chat-eval posture mode is local fixture",
        report_path,
        f"actual {report.get('mode')!r}",
        blocking=blocking,
    )
    require(
        report.get("release_ready") is False,
        "chat-eval local fixture does not claim release readiness",
        report_path,
        f"actual {report.get('release_ready')!r}",
        blocking=blocking,
    )

    checks_by_name = {
        item.get("name"): item
        for item in (report.get("checks") or [])
        if isinstance(item, dict)
    }
    expected_checks = {
        "chat_transcript_schema_contract",
        "chat_tool_registry_binding_contract",
        "chat_citation_resolvability_contract",
        "chat_tenant_isolation_contract",
        "chat_write_gate_contract",
        "chat_replay_determinism_contract",
    }
    require(
        expected_checks.issubset(checks_by_name.keys()),
        "chat-eval records every deterministic contract",
        report_path,
        f"missing={sorted(expected_checks - set(checks_by_name.keys()))}",
        blocking=blocking,
    )
    for check_name in sorted(expected_checks & set(checks_by_name.keys())):
        require(
            checks_by_name[check_name].get("ok") is True,
            f"chat-eval {check_name} is ok",
            f"{report_path}#/checks/{check_name}",
            blocking=blocking,
        )

    red_team = report.get("red_team")
    red_team_path = f"{report_path}#/red_team"
    require(isinstance(red_team, dict), "chat-eval records red-team summary", red_team_path, blocking=blocking)
    if isinstance(red_team, dict):
        require(
            red_team.get("ok") is True,
            "chat-eval red-team ok flag is true",
            red_team_path,
            f"actual {red_team.get('ok')!r}",
            blocking=blocking,
        )
        require(
            red_team.get("leaks_found") == 0,
            "chat-eval red-team found zero leaks",
            red_team_path,
            f"actual {red_team.get('leaks_found')!r}",
            blocking=blocking,
        )

    judge = report.get("judge")
    judge_path = f"{report_path}#/judge"
    require(isinstance(judge, dict), "chat-eval records judge disposition", judge_path, blocking=blocking)
    if isinstance(judge, dict):
        require(
            judge.get("enabled") is False,
            "chat-eval local fixture has judge disabled",
            judge_path,
            f"actual {judge.get('enabled')!r}",
            blocking=blocking,
        )
        require(
            judge.get("disposition") == "local-fixture",
            "chat-eval local fixture disposition is local-fixture",
            judge_path,
            f"actual {judge.get('disposition')!r}",
            blocking=blocking,
        )

    validate_chat_eval_artifact_items(report, report_path, blocking)
    return True


def validate_chat_eval_judge_evidence(report, report_path, blocking):
    require(isinstance(report, dict), "chat-eval judge evidence is object", report_path, blocking=blocking)
    if not isinstance(report, dict):
        return False
    validate_generated_timestamp(
        "chat-eval judge evidence",
        report,
        report_path,
        CHAT_EVAL_MAX_AGE,
        CHAT_EVAL_MAX_AGE_HOURS,
        blocking,
    )
    require(
        report.get("command") == "chat-eval",
        "chat-eval judge evidence command is stable",
        report_path,
        f"actual {report.get('command')!r}",
        blocking=blocking,
    )
    require(
        report.get("lane") == "CH",
        "chat-eval judge evidence lane is CH",
        report_path,
        f"actual {report.get('lane')!r}",
        blocking=blocking,
    )
    require(
        report.get("ok") is True,
        "chat-eval judge evidence ok flag is true",
        report_path,
        f"actual {report.get('ok')!r}",
        blocking=blocking,
    )
    require(
        report.get("release_ready") is True,
        "chat-eval judge evidence release_ready flag is true",
        report_path,
        f"actual {report.get('release_ready')!r}",
        blocking=blocking,
    )
    judge = report.get("judge")
    judge_path = f"{report_path}#/judge"
    require(isinstance(judge, dict), "chat-eval judge evidence records judge section", judge_path, blocking=blocking)
    if isinstance(judge, dict):
        require(
            judge.get("enabled") is True,
            "chat-eval judge evidence has judge enabled",
            judge_path,
            f"actual {judge.get('enabled')!r}",
            blocking=blocking,
        )
        require(
            judge.get("disposition") in {"evidence", "judge-certified", "recorded-live"},
            "chat-eval judge evidence disposition is release evidence",
            judge_path,
            f"actual {judge.get('disposition')!r}",
            blocking=blocking,
        )
        for field in ("rubric_sha256", "judge_model", "verdict"):
            require(
                isinstance(judge.get(field), str) and bool(judge.get(field).strip()),
                f"chat-eval judge evidence records {field}",
                judge_path,
                blocking=blocking,
            )
    return True


def validate_governed_write_posture(posture, posture_path, blocking):
    require(
        isinstance(posture, dict),
        "governed-write posture is object",
        posture_path,
        blocking=blocking,
    )
    if not isinstance(posture, dict):
        return
    require(
        posture.get("command") == "governed-write-check",
        "governed-write posture command is governed-write-check",
        posture_path,
        f"actual {posture.get('command')!r}",
        blocking=blocking,
    )
    require(
        posture.get("lane") == "G1",
        "governed-write posture lane is G1",
        posture_path,
        f"actual {posture.get('lane')!r}",
        blocking=blocking,
    )
    require(
        posture.get("ok") is True,
        "governed-write posture ok flag is true",
        posture_path,
        blocking=blocking,
    )
    gate = posture.get("gate")
    require(
        isinstance(gate, dict),
        "governed-write posture gate is recorded",
        posture_path,
        blocking=blocking,
    )
    if isinstance(gate, dict):
        require(
            gate.get("enforced") is True,
            "governed-write posture gate is enforced",
            f"{posture_path}#/gate",
            f"actual {gate.get('enforced')!r}",
            blocking=blocking,
        )
        require(
            gate.get("ready_to_enforce") is True,
            "governed-write posture gate is ready to enforce",
            f"{posture_path}#/gate",
            f"actual {gate.get('ready_to_enforce')!r}",
            blocking=blocking,
        )
    provider_graduation = posture.get("provider_graduation")
    require(
        isinstance(provider_graduation, dict),
        "governed-write posture records provider-graduation linkage",
        posture_path,
        blocking=blocking,
    )
    if isinstance(provider_graduation, dict):
        require(
            provider_graduation.get("present") is True,
            "governed-write posture provider-graduation evidence is present",
            f"{posture_path}#/provider_graduation",
            blocking=blocking,
        )
        require(
            provider_graduation.get("ok") is True,
            "governed-write posture provider-graduation evidence is ok",
            f"{posture_path}#/provider_graduation",
            blocking=blocking,
        )
    providers = posture.get("providers")
    require(
        isinstance(providers, list),
        "governed-write posture providers are recorded",
        posture_path,
        blocking=blocking,
    )
    provider_names = []
    if isinstance(providers, list):
        for index, provider in enumerate(providers):
            item_path = f"{posture_path}#/providers/{index}"
            require(
                isinstance(provider, dict),
                "governed-write posture provider entry is object",
                item_path,
                blocking=blocking,
            )
            if not isinstance(provider, dict):
                continue
            provider_name = provider.get("provider")
            known = isinstance(provider_name, str) and provider_name.strip() in EXTERNAL_API_EXPECTED_PROVIDERS
            require(
                known,
                "governed-write posture names known external API provider",
                item_path,
                f"actual {provider_name!r}",
                blocking=blocking,
            )
            if known:
                provider_names.append(provider_name.strip())
            require(
                provider.get("family") == "external_api",
                "governed-write posture provider family is external_api",
                item_path,
                f"actual {provider.get('family')!r}",
                blocking=blocking,
            )
            require(
                provider.get("write_enabled") is False,
                "governed-write posture provider write path is disabled",
                item_path,
                f"actual {provider.get('write_enabled')!r}",
                blocking=blocking,
            )
            require(
                provider.get("mcp_enabled") is False,
                "governed-write posture provider MCP path is disabled",
                item_path,
                f"actual {provider.get('mcp_enabled')!r}",
                blocking=blocking,
            )
            require(
                isinstance(provider.get("governed_write_certified"), bool),
                "governed-write posture provider certification flag is present",
                item_path,
                f"actual {provider.get('governed_write_certified')!r}",
                blocking=blocking,
            )
            gates = provider.get("gates")
            require(
                isinstance(gates, list) and bool(gates),
                "governed-write posture provider gates are recorded",
                item_path,
                blocking=blocking,
            )
    require(
        provider_names == EXTERNAL_API_EXPECTED_PROVIDERS,
        "governed-write posture provider list is stable",
        posture_path,
        f"actual {provider_names!r}",
        blocking=blocking,
    )
    certified_providers = posture.get("certified_providers")
    require(
        isinstance(certified_providers, list),
        "governed-write posture certified provider list is recorded",
        posture_path,
        blocking=blocking,
    )
    blocking_violations = posture.get("blocking_violations")
    require(
        isinstance(blocking_violations, list) and not blocking_violations,
        "governed-write posture has no blocking violations",
        posture_path,
        f"actual {blocking_violations!r}",
        blocking=blocking,
    )


def validate_security_certification_timestamp(
    security_cert,
    security_cert_path,
    provider_parity_generated_at,
    blocking,
):
    if not isinstance(security_cert, dict):
        return None

    now = dt.datetime.now(dt.timezone.utc)
    generated_at = parse_timestamp_with_timezone(security_cert.get("generated_at_utc"))
    require(
        generated_at is not None,
        "security certification generated timestamp is valid",
        security_cert_path,
        "generated_at_utc must be ISO-8601 with timezone",
        blocking=blocking,
    )
    if generated_at is None:
        return None

    require(
        generated_at <= now + EVIDENCE_TIMESTAMP_SKEW,
        "security certification generated timestamp is not in the future",
        security_cert_path,
        generated_at.isoformat().replace("+00:00", "Z"),
        blocking=blocking,
    )
    require(
        generated_at >= now - SECURITY_CERTIFICATION_MAX_AGE,
        "security certification generated timestamp is recent",
        security_cert_path,
        (
            f"generated_at_utc {generated_at.isoformat().replace('+00:00', 'Z')} "
            f"is older than {SECURITY_CERTIFICATION_MAX_AGE_HOURS} hours"
        ),
        blocking=blocking,
    )
    if provider_parity_generated_at is not None:
        require(
            generated_at + EVIDENCE_TIMESTAMP_SKEW >= provider_parity_generated_at,
            "security certification generated timestamp does not predate provider parity",
            security_cert_path,
            (
                "security-certification must be generated after the provider-parity "
                f"evidence it consumes; security={generated_at.isoformat().replace('+00:00', 'Z')}, "
                f"provider_parity={provider_parity_generated_at.isoformat().replace('+00:00', 'Z')}"
            ),
            blocking=blocking,
        )
    return generated_at


def validate_security_certification_artifacts(security_cert, security_cert_path, blocking):
    resolved_artifacts_by_name = {}
    artifacts_value = security_cert.get("artifacts")
    require(
        isinstance(artifacts_value, list) and bool(artifacts_value),
        "security certification retained artifacts are recorded",
        security_cert_path,
        blocking=blocking,
    )
    if not isinstance(artifacts_value, list):
        return resolved_artifacts_by_name
    artifacts_by_name = {
        artifact.get("name"): artifact
        for artifact in artifacts_value
        if isinstance(artifact, dict) and isinstance(artifact.get("name"), str)
    }
    duplicate_names = sorted(
        name
        for name in set(artifacts_by_name)
        if sum(
            1
            for artifact in artifacts_value
            if isinstance(artifact, dict) and artifact.get("name") == name
        )
        > 1
    )
    require(
        not duplicate_names,
        "security certification retained artifact names are unique",
        security_cert_path,
        f"duplicates={duplicate_names}",
        blocking=blocking,
    )

    required_artifacts = [
        ("runtime-routing-source", None),
        ("graphql-introspection-runtime-test-log", report_dir),
        ("provider-parity", report_dir),
    ] + [(f"provider-log:{provider}", report_dir) for provider in SECURITY_EXPECTED_PROVIDERS]
    for artifact_name, required_parent in required_artifacts:
        artifact = artifacts_by_name.get(artifact_name)
        artifact_path = f"{security_cert_path}#/artifacts/{artifact_name}"
        require(
            isinstance(artifact, dict),
            f"security certification retained artifact {artifact_name} is recorded",
            artifact_path,
            blocking=blocking,
        )
        if isinstance(artifact, dict):
            resolved = validate_retained_file_artifact(
                f"security certification {artifact_name}",
                artifact,
                artifact_path,
                blocking=blocking,
                required_parent=required_parent,
                require_ok_flag=False,
            )
            if resolved is not None:
                resolved_artifacts_by_name[artifact_name] = resolved
    return resolved_artifacts_by_name


def validate_security_certification_cross_references(
    security_cert,
    security_cert_path,
    security_cert_artifact_paths,
    provider_parity_file,
    provider_parity_logs_by_provider,
    blocking,
):
    if not isinstance(security_cert, dict):
        return
    if not isinstance(security_cert_artifact_paths, dict):
        security_cert_artifact_paths = {}
    if not isinstance(provider_parity_logs_by_provider, dict):
        provider_parity_logs_by_provider = {}

    introspection = security_cert.get("introspection_auth")
    if isinstance(introspection, dict):
        source_ref = introspection.get("source")
        source_path = resolve_retained_path(source_ref) if isinstance(source_ref, str) else None
        require(
            source_path is not None,
            "security certification introspection source reference exists",
            f"{security_cert_path}#/introspection_auth/source",
            f"missing retained source reference: {source_ref!r}",
            blocking=blocking,
        )
        artifact_source_path = security_cert_artifact_paths.get("runtime-routing-source")
        require(
            paths_match(source_path, artifact_source_path),
            "security certification introspection source reference matches retained artifact",
            f"{security_cert_path}#/introspection_auth/source",
            (
                f"expected {artifact_source_path.as_posix() if artifact_source_path is not None else '<missing>'}, "
                f"actual {source_path.as_posix() if source_path is not None else '<missing>'}"
            ),
            blocking=blocking,
        )
        require(
            paths_match(source_path, SECURITY_INTROSPECTION_ROUTING_SOURCE),
            "security certification introspection source is canonical runtime routing source",
            f"{security_cert_path}#/introspection_auth/source",
            (
                f"expected {SECURITY_INTROSPECTION_ROUTING_SOURCE.as_posix()}, "
                f"actual {source_path.as_posix() if source_path is not None else '<missing>'}"
            ),
            blocking=blocking,
        )

        runtime_log_ref = introspection.get("runtime_log")
        runtime_log_path = (
            resolve_retained_path(runtime_log_ref)
            if isinstance(runtime_log_ref, str)
            else None
        )
        require(
            runtime_log_path is not None,
            "security certification introspection runtime log reference exists",
            f"{security_cert_path}#/introspection_auth/runtime_log",
            f"missing retained runtime log reference: {runtime_log_ref!r}",
            blocking=blocking,
        )
        artifact_runtime_log_path = security_cert_artifact_paths.get(
            "graphql-introspection-runtime-test-log"
        )
        require(
            paths_match(runtime_log_path, artifact_runtime_log_path),
            "security certification introspection runtime log reference matches retained artifact",
            f"{security_cert_path}#/introspection_auth/runtime_log",
            (
                f"expected {artifact_runtime_log_path.as_posix() if artifact_runtime_log_path is not None else '<missing>'}, "
                f"actual {runtime_log_path.as_posix() if runtime_log_path is not None else '<missing>'}"
            ),
            blocking=blocking,
        )

    live_security = security_cert.get("live_security_contracts")
    if not isinstance(live_security, dict):
        return

    canonical_provider_parity = provider_parity_file.resolve(strict=False)
    artifact_provider_parity = security_cert_artifact_paths.get("provider-parity")
    require(
        paths_match(artifact_provider_parity, canonical_provider_parity),
        "security certification provider-parity artifact matches retained provider parity report",
        f"{security_cert_path}#/artifacts/provider-parity",
        (
            f"expected {canonical_provider_parity.as_posix()}, "
            f"actual {artifact_provider_parity.as_posix() if artifact_provider_parity is not None else '<missing>'}"
        ),
        blocking=blocking,
    )

    live_provider_parity_ref = live_security.get("provider_parity")
    live_provider_parity_path = (
        resolve_retained_path(live_provider_parity_ref)
        if isinstance(live_provider_parity_ref, str)
        else None
    )
    require(
        live_provider_parity_path is not None,
        "security certification live security provider parity reference exists",
        f"{security_cert_path}#/live_security_contracts/provider_parity",
        f"missing retained provider parity reference: {live_provider_parity_ref!r}",
        blocking=blocking,
    )
    require(
        paths_match(live_provider_parity_path, canonical_provider_parity),
        "security certification live security provider parity reference matches retained provider parity report",
        f"{security_cert_path}#/live_security_contracts/provider_parity",
        (
            f"expected {canonical_provider_parity.as_posix()}, "
            f"actual {live_provider_parity_path.as_posix() if live_provider_parity_path is not None else '<missing>'}"
        ),
        blocking=blocking,
    )

    providers = live_security.get("providers")
    if not isinstance(providers, list):
        return
    providers_by_name = {
        provider.get("provider"): provider
        for provider in providers
        if isinstance(provider, dict) and isinstance(provider.get("provider"), str)
    }
    for provider_name in SECURITY_EXPECTED_PROVIDERS:
        provider = providers_by_name.get(provider_name)
        provider_path = f"{security_cert_path}#/live_security_contracts/providers/{provider_name}"
        if not isinstance(provider, dict):
            continue

        live_log_ref = provider.get("log")
        live_log_path = resolve_retained_path(live_log_ref) if isinstance(live_log_ref, str) else None
        require(
            live_log_path is not None,
            f"{provider_name} security certification provider log reference exists",
            f"{provider_path}/log",
            f"missing retained provider log reference: {live_log_ref!r}",
            blocking=blocking,
        )

        artifact_log_path = security_cert_artifact_paths.get(f"provider-log:{provider_name}")
        require(
            paths_match(live_log_path, artifact_log_path),
            f"{provider_name} security certification provider log reference matches retained artifact",
            f"{provider_path}/log",
            (
                f"expected {artifact_log_path.as_posix() if artifact_log_path is not None else '<missing>'}, "
                f"actual {live_log_path.as_posix() if live_log_path is not None else '<missing>'}"
            ),
            blocking=blocking,
        )

        parity_log_path = provider_parity_logs_by_provider.get(provider_name)
        require(
            paths_match(live_log_path, parity_log_path),
            f"{provider_name} security certification provider log reference matches provider parity log",
            f"{provider_path}/log",
            (
                f"expected {parity_log_path.as_posix() if parity_log_path is not None else '<missing>'}, "
                f"actual {live_log_path.as_posix() if live_log_path is not None else '<missing>'}"
            ),
            blocking=blocking,
        )


release_check_path = (report_dir / "release-check.json").as_posix()
provider_preflight_valid_base_urls = {}
release_check_generated_at = None
release_check_entries_by_name = {}
governed_write_certified_providers = []
release_check = load_required(
    "release-check",
    "release-check.json",
    dict,
    missing_detail=(
        "top-level release-check evidence is required so final release evidence can "
        "verify release_ready, evidence_mode, provider_scope, and remediation signals"
    ),
)
if release_check is not None:
    require(
        release_check.get("command") == "release-check",
        "release-check command is stable",
        release_check_path,
    )
    release_check_generated_at = parse_timestamp_with_timezone(release_check.get("generated_at_utc"))
    require(
        release_check_generated_at is not None,
        "release-check generated timestamp is valid",
        release_check_path,
        "generated_at_utc must be ISO-8601 with timezone",
        blocking=not local_fixture,
    )
    if release_check_generated_at is not None:
        now = dt.datetime.now(dt.timezone.utc)
        require(
            release_check_generated_at <= now + EVIDENCE_TIMESTAMP_SKEW,
            "release-check generated timestamp is not in the future",
            release_check_path,
            release_check_generated_at.isoformat().replace("+00:00", "Z"),
            blocking=not local_fixture,
        )
        require(
            release_check_generated_at >= now - RELEASE_CHECK_MAX_AGE,
            "release-check generated timestamp is recent",
            release_check_path,
            (
                f"generated_at_utc {release_check_generated_at.isoformat().replace('+00:00', 'Z')} "
                f"is older than {RELEASE_CHECK_MAX_AGE_HOURS} hours"
            ),
            blocking=not local_fixture,
        )
    require(
        release_check.get("ok") is True,
        "release-check ok flag must be true for release promotion",
        release_check_path,
        f"actual {release_check.get('ok')!r}",
        blocking=not local_fixture,
    )
    require(
        isinstance(release_check.get("release_ready"), bool),
        "release-check release_ready flag is present",
        release_check_path,
    )
    evidence_mode = release_check.get("evidence_mode")
    require(
        isinstance(evidence_mode, dict),
        "release-check evidence_mode is recorded",
        release_check_path,
    )
    if isinstance(evidence_mode, dict):
        require(
            evidence_mode.get("provider_certification") == "live-required",
            "release-check provider certification evidence mode is live-required",
            release_check_path,
            f"actual {evidence_mode.get('provider_certification') or '<missing>'}",
        )
        require(
            evidence_mode.get("security_certification") == "live-provider-evidence-required",
            "release-check security certification evidence mode is live-provider-evidence-required",
            release_check_path,
            f"actual {evidence_mode.get('security_certification') or '<missing>'}",
        )
        if release_mode_requires_live_ops_evidence():
            require(
                evidence_mode.get("operations") == "live-required",
                "release-check operations evidence mode is live-required",
                release_check_path,
                f"actual {evidence_mode.get('operations') or '<missing>'}",
                blocking=not local_fixture,
            )
        if release_mode_requires_pds_baseline_evidence():
            require(
                evidence_mode.get("pds_security_baseline") == "live-required",
                "release-check PDS baseline evidence mode is live-required",
                release_check_path,
                f"actual {evidence_mode.get('pds_security_baseline') or '<missing>'}",
                blocking=not local_fixture,
            )
        if release_mode_requires_performance_evidence():
            require(
                evidence_mode.get("performance") == "live-required",
                "release-check performance evidence mode is live-required",
                release_check_path,
                f"actual {evidence_mode.get('performance') or '<missing>'}",
                blocking=not local_fixture,
            )
        if release_mode_requires_production_attestations():
            require(
                evidence_mode.get("production_attestations_required") is True,
                "release-check production attestation evidence mode is required",
                release_check_path,
                f"actual {evidence_mode.get('production_attestations_required')!r}",
                blocking=not local_fixture,
            )
            require(
                evidence_mode.get("security_assurance") == "production-attestations-required",
                "release-check security assurance evidence mode requires production attestations",
                release_check_path,
                f"actual {evidence_mode.get('security_assurance') or '<missing>'}",
                blocking=not local_fixture,
            )
        elif release_mode_requires_security_assurance_decision():
            require(
                evidence_mode.get("security_assurance")
                in {"decision-required", "production-attestations-required"},
                "release-check security assurance evidence mode is decision-required",
                release_check_path,
                f"actual {evidence_mode.get('security_assurance') or '<missing>'}",
                blocking=not local_fixture,
            )
        else:
            if "security_assurance" in evidence_mode:
                require(
                    evidence_mode.get("security_assurance")
                    in {"not-required", "decision-required", "production-attestations-required"},
                    "release-check security assurance evidence mode is valid",
                    release_check_path,
                    f"actual {evidence_mode.get('security_assurance') or '<missing>'}",
                )
            else:
                record(
                    "release-check security assurance evidence mode is stale",
                    False,
                    release_check_path,
                    (
                        "missing evidence_mode.security_assurance; rerun scripts/appfw release-check --json"
                    ),
                    blocking=False,
                )
    missing_provider_base_urls = release_check.get("missing_provider_base_urls")
    require(
        isinstance(missing_provider_base_urls, list),
        "release-check missing provider base URL list is recorded",
        release_check_path,
    )
    missing_provider_details = []
    missing_provider_names = []
    if isinstance(missing_provider_base_urls, list):
        for index, missing_provider_url in enumerate(missing_provider_base_urls):
            item_path = f"{release_check_path}#/missing_provider_base_urls/{index}"
            require(
                isinstance(missing_provider_url, dict),
                "release-check missing provider base URL entry is object",
                item_path,
            )
            if not isinstance(missing_provider_url, dict):
                continue
            provider = missing_provider_url.get("provider")
            env = missing_provider_url.get("env")
            require(
                isinstance(provider, str) and bool(provider.strip()),
                "release-check missing provider base URL entry records provider",
                item_path,
            )
            require(
                isinstance(env, str) and bool(env.strip()),
                "release-check missing provider base URL entry records env var",
                item_path,
            )
            provider_known = isinstance(provider, str) and provider.strip() in SECURITY_EXPECTED_PROVIDERS
            require(
                provider_known,
                "release-check missing provider base URL entry names release-certified provider",
                item_path,
                f"actual {provider!r}",
            )
            if provider_known:
                missing_provider_names.append(provider.strip())
                expected_env = expected_provider_base_url_env(provider)
                require(
                    isinstance(env, str) and env.strip() == expected_env,
                    "release-check missing provider base URL entry records canonical env var",
                    item_path,
                    f"expected {expected_env}, actual {env!r}",
                )
            if isinstance(provider, str) and isinstance(env, str):
                missing_provider_details.append(f"{provider}:{env}")
        duplicate_missing_provider_names = duplicate_values(missing_provider_names)
        require(
            not duplicate_missing_provider_names,
            "release-check missing provider base URL entries name each provider once",
            release_check_path,
            f"duplicates={duplicate_missing_provider_names}",
        )
        require(
            not missing_provider_base_urls,
            "release-check provider URL preflight must have no missing provider URLs",
            release_check_path,
            f"missing {missing_provider_details or missing_provider_base_urls}",
            blocking=not local_fixture,
        )
    invalid_provider_base_urls = release_check.get("invalid_provider_base_urls")
    require(
        isinstance(invalid_provider_base_urls, list),
        "release-check invalid provider base URL list is recorded",
        release_check_path,
    )
    invalid_provider_details = []
    invalid_provider_names = []
    if isinstance(invalid_provider_base_urls, list):
        for index, invalid_provider_url in enumerate(invalid_provider_base_urls):
            item_path = f"{release_check_path}#/invalid_provider_base_urls/{index}"
            require(
                isinstance(invalid_provider_url, dict),
                "release-check invalid provider base URL entry is object",
                item_path,
            )
            if not isinstance(invalid_provider_url, dict):
                continue
            provider = invalid_provider_url.get("provider")
            env = invalid_provider_url.get("env")
            reason = invalid_provider_url.get("reason")
            redacted_url = invalid_provider_url.get("redacted_url")
            require(
                isinstance(provider, str) and bool(provider.strip()),
                "release-check invalid provider base URL entry records provider",
                item_path,
            )
            require(
                isinstance(env, str) and bool(env.strip()),
                "release-check invalid provider base URL entry records env var",
                item_path,
            )
            provider_known = isinstance(provider, str) and provider.strip() in SECURITY_EXPECTED_PROVIDERS
            require(
                provider_known,
                "release-check invalid provider base URL entry names release-certified provider",
                item_path,
                f"actual {provider!r}",
            )
            if provider_known:
                invalid_provider_names.append(provider.strip())
                expected_env = expected_provider_base_url_env(provider)
                require(
                    isinstance(env, str) and env.strip() == expected_env,
                    "release-check invalid provider base URL entry records canonical env var",
                    item_path,
                    f"expected {expected_env}, actual {env!r}",
                )
            require(
                isinstance(reason, str) and bool(reason.strip()),
                "release-check invalid provider base URL entry records reason",
                item_path,
            )
            redacted_url_safe = (
                isinstance(redacted_url, str)
                and bool(redacted_url.strip())
                and "@" not in redacted_url
                and "?" not in redacted_url
                and "#" not in redacted_url
            )
            require(
                redacted_url_safe,
                "release-check invalid provider base URL entry records safe redacted URL",
                item_path,
                f"actual {redacted_url!r}",
            )
            if isinstance(provider, str) and isinstance(env, str):
                invalid_provider_details.append(f"{provider}:{env}")
        duplicate_invalid_provider_names = duplicate_values(invalid_provider_names)
        require(
            not duplicate_invalid_provider_names,
            "release-check invalid provider base URL entries name each provider once",
            release_check_path,
            f"duplicates={duplicate_invalid_provider_names}",
        )
        overlapping_issue_provider_names = sorted(set(missing_provider_names) & set(invalid_provider_names))
        require(
            not overlapping_issue_provider_names,
            "release-check provider URL preflight issues assign one status per provider",
            release_check_path,
            f"overlapping providers={overlapping_issue_provider_names}",
        )
        require(
            not invalid_provider_base_urls,
            "release-check provider URL preflight must have no invalid provider URLs",
            release_check_path,
            f"invalid {invalid_provider_details or invalid_provider_base_urls}",
            blocking=not local_fixture,
        )
    provider_url_preflight_issues = []
    if isinstance(missing_provider_base_urls, list):
        provider_url_preflight_issues.extend(missing_provider_base_urls)
    if isinstance(invalid_provider_base_urls, list):
        provider_url_preflight_issues.extend(invalid_provider_base_urls)
    release_check_entries_by_name, _, _ = validate_release_check_checks(
        release_check,
        release_check_path,
        provider_url_preflight_issues,
        release_check_generated_at,
        blocking=True,
    )
    provider_preflight_valid_base_urls = validate_provider_url_preflight_snapshot(
        release_check_entries_by_name,
        missing_provider_base_urls if isinstance(missing_provider_base_urls, list) else [],
        invalid_provider_base_urls if isinstance(invalid_provider_base_urls, list) else [],
        release_check_path,
        blocking=not local_fixture,
    )
    if release_check.get("release_ready") is True:
        ready_required_checks = [
            "pds-baseline",
            "provider-url-preflight",
            "release-identity",
            "ops-certification",
            "composition-check",
            "provider-graduation",
            "governed-write-check",
            "provider-test",
            "security-certification",
        ]
        if release_mode_requires_performance_evidence():
            ready_required_checks.extend(["load-test-suite", "provider-performance"])
        for check_name in ready_required_checks:
            entry = release_check_entries_by_name.get(check_name)
            require(
                isinstance(entry, dict),
                f"release-ready release-check includes {check_name} check",
                release_check_path,
                blocking=not local_fixture,
            )
            if isinstance(entry, dict):
                require(
                    entry.get("ok") is True,
                    f"release-ready release-check {check_name} check passed",
                    release_check_path,
                    f"actual {entry.get('ok')!r}",
                    blocking=not local_fixture,
                )
        ready_artifacts = {
            "pds-baseline": report_dir / "pds-security-baseline.json",
            "release-identity": report_dir / "release-identity.json",
            "ops-certification": report_dir / "ops-certification.json",
            "composition-check": report_dir / "composition-check.json",
            "provider-graduation": report_dir / "provider-graduation.json",
            "governed-write-check": report_dir / "governed-write-posture.json",
            "provider-test": report_dir / "provider-parity.json",
            "security-certification": report_dir / "security-certification.json",
        }
        if release_mode_requires_performance_evidence():
            ready_artifacts.update(
                {
                    "load-test-suite": report_dir / "load-test.json",
                    "provider-performance": report_dir / "provider-performance.json",
                }
            )
        for check_name, expected_artifact_path in ready_artifacts.items():
            validate_release_check_ready_artifact(
                release_check_entries_by_name,
                check_name,
                expected_artifact_path,
                release_check_path,
                blocking=not local_fixture,
            )
    remediation_ids = validate_remediation_items(
        "release-check",
        release_check.get("remediation_work_items"),
        release_check_path,
        blocking=not local_fixture,
    )
    provider_scope = release_check.get("provider_scope")
    require(
        isinstance(provider_scope, dict),
        "release-check provider_scope is recorded",
        release_check_path,
    )
    if isinstance(provider_scope, dict):
        require(
            provider_scope.get("release_certified_crud") == SECURITY_EXPECTED_PROVIDERS,
            "release-check release-certified CRUD provider scope is stable",
            release_check_path,
        )
        graph_providers = provider_scope.get("graph_providers")
        require(
            isinstance(graph_providers, list),
            "release-check graph provider scope is recorded",
            release_check_path,
        )
        if isinstance(graph_providers, list):
            graph_provider_names = []
            for index, graph_provider in enumerate(graph_providers):
                item_path = f"{release_check_path}#/provider_scope/graph_providers/{index}"
                require(
                    isinstance(graph_provider, dict),
                    "release-check graph provider scope entry is object",
                    item_path,
                )
                if not isinstance(graph_provider, dict):
                    continue
                provider_name = graph_provider.get("provider")
                if isinstance(provider_name, str) and provider_name.strip():
                    graph_provider_names.append(provider_name.strip())
                require(
                    provider_name == "neo4j",
                    "release-check graph provider scope names known graph provider",
                    item_path,
                    f"actual {provider_name!r}",
                )
                require(
                    graph_provider.get("workspace_crate") == "appfw-provider-neo4j",
                    "release-check graph provider scope records workspace crate",
                    item_path,
                    f"actual {graph_provider.get('workspace_crate')!r}",
                )
                require(
                    graph_provider.get("release_certified") is False,
                    "release-check graph provider scope is not release-certified",
                    item_path,
                    f"actual {graph_provider.get('release_certified')!r}",
                    blocking=not local_fixture,
                )
                require(
                    graph_provider.get("certification_path")
                    == "docs/runtime/graph-read-providers.md#certification-posture",
                    "release-check graph provider scope records certification path",
                    item_path,
                    f"actual {graph_provider.get('certification_path')!r}",
                )
                posture = graph_provider.get("posture")
                require(
                    isinstance(posture, str)
                    and "outside CRUD semantic parity" in posture
                    and "foundation" in posture,
                    "release-check graph provider scope records non-CRUD posture",
                    item_path,
                    f"actual {posture!r}",
                )
            duplicate_graph_provider_names = duplicate_values(graph_provider_names)
            require(
                not duplicate_graph_provider_names,
                "release-check graph provider scope names each provider once",
                release_check_path,
                f"duplicates={duplicate_graph_provider_names}",
            )
            require(
                graph_provider_names == ["neo4j"],
                "release-check graph provider scope is stable",
                release_check_path,
                f"actual {graph_provider_names!r}",
            )
        external_api_providers = provider_scope.get("external_api_providers")
        require(
            isinstance(external_api_providers, list),
            "release-check external API provider scope is recorded",
            release_check_path,
        )
        if isinstance(external_api_providers, list):
            external_api_provider_names = []
            for index, external_api_provider in enumerate(external_api_providers):
                item_path = f"{release_check_path}#/provider_scope/external_api_providers/{index}"
                require(
                    isinstance(external_api_provider, dict),
                    "release-check external API provider scope entry is object",
                    item_path,
                )
                if not isinstance(external_api_provider, dict):
                    continue
                provider_name = external_api_provider.get("provider")
                provider_known = (
                    isinstance(provider_name, str)
                    and provider_name.strip() in EXTERNAL_API_EXPECTED_PROVIDERS
                )
                require(
                    provider_known,
                    "release-check external API provider scope names known provider",
                    item_path,
                    f"actual {provider_name!r}",
                )
                if provider_known:
                    provider_name = provider_name.strip()
                    external_api_provider_names.append(provider_name)
                    require(
                        external_api_provider.get("workspace_crate")
                        == EXTERNAL_API_WORKSPACE_CRATES[provider_name],
                        "release-check external API provider scope records workspace crate",
                        item_path,
                        f"actual {external_api_provider.get('workspace_crate')!r}",
                    )
                require(
                    external_api_provider.get("release_certified") is False,
                    "release-check external API provider is outside CRUD release certification",
                    item_path,
                    f"actual {external_api_provider.get('release_certified')!r}",
                    blocking=not local_fixture,
                )
                governed_write_certified = external_api_provider.get("governed_write_certified")
                require(
                    isinstance(governed_write_certified, bool),
                    "release-check external API provider governed-write certification flag is present",
                    item_path,
                    f"actual {governed_write_certified!r}",
                )
                if governed_write_certified is True and provider_known:
                    governed_write_certified_providers.append(provider_name)
                require(
                    external_api_provider.get("certification_path")
                    == "docs/runtime/saas-certification.md#governed-write-posture",
                    "release-check external API provider scope records governed-write certification path",
                    item_path,
                    f"actual {external_api_provider.get('certification_path')!r}",
                )
                posture = external_api_provider.get("posture")
                if governed_write_certified is True:
                    require(
                        isinstance(posture, str)
                        and "governed" in posture
                        and "write" in posture,
                        "release-check external API provider scope records governed-write posture",
                        item_path,
                        f"actual {posture!r}",
                    )
                else:
                    require(
                        isinstance(posture, str)
                        and "governed writes disabled" in posture,
                        "release-check external API provider scope records disabled write posture",
                        item_path,
                        f"actual {posture!r}",
                    )
            duplicate_external_api_provider_names = duplicate_values(external_api_provider_names)
            require(
                not duplicate_external_api_provider_names,
                "release-check external API provider scope names each provider once",
                release_check_path,
                f"duplicates={duplicate_external_api_provider_names}",
            )
            require(
                external_api_provider_names == EXTERNAL_API_EXPECTED_PROVIDERS,
                "release-check external API provider scope is stable",
                release_check_path,
                f"actual {external_api_provider_names!r}",
            )
    failure_summary = release_check.get("failure_summary")
    require(
        isinstance(failure_summary, dict),
        "release-check failure summary is recorded",
        release_check_path,
    )
    release_check_root_causes = []
    if isinstance(failure_summary, dict):
        root_causes = failure_summary.get("root_causes")
        require(
            isinstance(root_causes, list),
            "release-check failure summary root causes are recorded",
            release_check_path,
        )
        if isinstance(root_causes, list):
            for index, value in enumerate(root_causes):
                item_path = f"{release_check_path}#/failure_summary/root_causes/{index}"
                require(
                    isinstance(value, str) and bool(value.strip()),
                    "release-check failure summary root cause is non-empty",
                    item_path,
                )
                if isinstance(value, str) and value.strip():
                    release_check_root_causes.append(value.strip())
    if release_check.get("ok") is not True or release_check.get("release_ready") is not True:
        require(
            bool(release_check_root_causes),
            "release-check failure summary includes root cause when not ready",
            release_check_path,
            blocking=not local_fixture,
        )
    if provider_url_preflight_issues:
        required_provider_remediation = {"LIVE-001", "LIVE-002"}
        provider_root_cause = any(
            (
                "provider" in cause.lower()
                and ("url" in cause.lower() or "base url" in cause.lower())
            )
            or any(detail in cause for detail in missing_provider_details)
            or any(detail in cause for detail in invalid_provider_details)
            for cause in release_check_root_causes
        )
        require(
            provider_root_cause,
            "release-check failure summary explains provider URL preflight blockers",
            release_check_path,
            blocking=not local_fixture,
        )
        require(
            required_provider_remediation.issubset(remediation_ids),
            "release-check remediation covers provider URL preflight blockers",
            release_check_path,
            f"missing {sorted(required_provider_remediation - remediation_ids)}",
            blocking=not local_fixture,
        )
    if any("PDS Security Baseline" in cause for cause in release_check_root_causes):
        required_pds_remediation = {
            "LIVE-015",
            "LIVE-016",
            "LIVE-017",
            "LIVE-018",
            "LIVE-019",
            "LIVE-020",
            "LIVE-021",
        }
        require(
            required_pds_remediation.issubset(remediation_ids),
            "release-check remediation covers PDS baseline blockers",
            release_check_path,
            f"missing {sorted(required_pds_remediation - remediation_ids)}",
            blocking=not local_fixture,
        )
    if any("release identity" in cause.lower() for cause in release_check_root_causes):
        required_release_identity_remediation = {"LIVE-004", "LIVE-014"}
        require(
            required_release_identity_remediation.issubset(remediation_ids),
            "release-check remediation covers release identity blockers",
            release_check_path,
            f"missing {sorted(required_release_identity_remediation - remediation_ids)}",
            blocking=not local_fixture,
        )
    if release_check.get("release_ready") is not True:
        record(
            "release-check retained release blocker",
            False,
            release_check_path,
            "; ".join(release_check_root_causes)
            if release_check_root_causes
            else "release-check release_ready is not true",
            blocking=False,
        )


governed_write_evidence_file = report_dir / "governed-write-evidence.json"
governed_write_evidence_path = governed_write_evidence_file.as_posix()
governed_write_evidence_required = bool(governed_write_certified_providers)
if governed_write_evidence_required or governed_write_evidence_file.is_file():
    governed_write_evidence = load_required(
        "governed-write-evidence",
        "governed-write-evidence.json",
        dict,
        missing_detail=(
            "governed write evidence is required when release-check provider_scope "
            "claims an external API provider has governed_write_certified:true"
        ),
        applicability="required" if governed_write_evidence_required else "optional",
    )
    if governed_write_evidence is not None:
        validate_governed_write_evidence(
            governed_write_evidence,
            governed_write_evidence_path,
            governed_write_certified_providers,
            blocking=not local_fixture,
        )
else:
    record_artifact(
        "governed-write-evidence",
        governed_write_evidence_file,
        required=False,
        required_in_ci=False,
        status="not-applicable",
        applicability="not-applicable",
        applicability_reason=(
            "not required until provider_scope.external_api_providers contains "
            "governed_write_certified:true"
        ),
    )

chat_eval_file = report_dir / "chat-eval.json"
chat_eval_path = chat_eval_file.as_posix()
if chat_eval_file.is_file():
    chat_eval = load_required(
        "chat-eval",
        "chat-eval.json",
        dict,
        applicability="optional",
        applicability_reason=(
            "validated when retained as CH6 deterministic local-fixture posture; "
            "this artifact never satisfies release readiness"
        ),
    )
    if chat_eval is not None:
        validate_chat_eval_report(chat_eval, chat_eval_path, blocking=not local_fixture)
else:
    record_artifact(
        "chat-eval",
        chat_eval_file,
        required=False,
        required_in_ci=False,
        status="not-applicable",
        applicability="not-applicable",
        applicability_reason=(
            "not required until the product chat-eval command is run; local fixture "
            "posture remains non-release-ready even when present"
        ),
    )

chat_eval_judge_file = report_dir / "chat-eval-judge-evidence.json"
chat_eval_judge_path = chat_eval_judge_file.as_posix()
chat_eval_judge_required = release_mode_requires_chat_eval_judge_evidence()
if chat_eval_judge_required or chat_eval_judge_file.is_file():
    chat_eval_judge = load_required(
        "chat-eval-judge-evidence",
        "chat-eval-judge-evidence.json",
        dict,
        missing_detail=(
            "chat-eval judge/live evidence is required by "
            "APPFW_REQUIRE_CHAT_EVAL_JUDGE_EVIDENCE or "
            "APPFW_RELEASE_REQUIRE_CHAT_EVAL_JUDGE_EVIDENCE"
        ),
        applicability="required" if chat_eval_judge_required else "optional",
        applicability_reason=(
            "required by APPFW_REQUIRE_CHAT_EVAL_JUDGE_EVIDENCE or "
            "APPFW_RELEASE_REQUIRE_CHAT_EVAL_JUDGE_EVIDENCE"
            if chat_eval_judge_required
            else "validated when retained as real chat judge/live evidence; this is "
            "the only chat-eval artifact allowed to claim release readiness"
        ),
    )
    if chat_eval_judge is not None:
        validate_chat_eval_judge_evidence(
            chat_eval_judge,
            chat_eval_judge_path,
            blocking=not local_fixture,
        )
else:
    record_artifact(
        "chat-eval-judge-evidence",
        chat_eval_judge_file,
        required=False,
        required_in_ci=chat_eval_judge_required,
        status="not-applicable",
        applicability="not-applicable",
        applicability_reason=(
            "not required until APPFW_REQUIRE_CHAT_EVAL_JUDGE_EVIDENCE or "
            "APPFW_RELEASE_REQUIRE_CHAT_EVAL_JUDGE_EVIDENCE is enabled"
        ),
    )

wave2_readiness_file = report_dir / "wave2-readiness.json"
wave2_readiness_path = wave2_readiness_file.as_posix()
if wave2_readiness_file.is_file():
    record_artifact(
        "wave2-readiness",
        wave2_readiness_file,
        required=False,
        required_in_ci=False,
        status="present",
        applicability="optional",
        applicability_reason=(
            "validated when retained as report-only North-Star Wave 2 readiness evidence"
        ),
    )
    try:
        wave2_readiness = strict_json_loads(wave2_readiness_file.read_text(encoding="utf-8"))
        require(
            isinstance(wave2_readiness, dict),
            "wave2 readiness evidence is object",
            wave2_readiness_path,
        )
    except Exception as exc:  # pragma: no cover - exercised by shell usage.
        wave2_readiness = None
        require(False, "wave2 readiness evidence parses as JSON", wave2_readiness_path, str(exc))
    if isinstance(wave2_readiness, dict):
        require(
            wave2_readiness.get("command") == "wave2-status",
            "wave2 readiness command is stable",
            wave2_readiness_path,
            f"actual {wave2_readiness.get('command')!r}",
        )
        require(
            wave2_readiness.get("lane") == "north-star-wave-2",
            "wave2 readiness lane is stable",
            wave2_readiness_path,
            f"actual {wave2_readiness.get('lane')!r}",
        )
        require(
            wave2_readiness.get("ok") is True,
            "wave2 readiness ok flag is true",
            wave2_readiness_path,
            f"actual {wave2_readiness.get('ok')!r}",
        )
        wave2_release_ready = wave2_readiness.get("release_ready")
        require(
            isinstance(wave2_release_ready, bool),
            "wave2 readiness release_ready flag is boolean",
            wave2_readiness_path,
            f"actual {wave2_release_ready!r}",
        )
        wave2_lanes = wave2_readiness.get("lanes")
        require(
            isinstance(wave2_lanes, list) and bool(wave2_lanes),
            "wave2 readiness records lanes",
            wave2_readiness_path,
        )
        wave2_external_gates = wave2_readiness.get("remaining_external_gates")
        require(
            isinstance(wave2_external_gates, list),
            "wave2 readiness remaining external gates is an array",
            wave2_readiness_path,
            f"actual {type(wave2_external_gates).__name__}",
        )
        wave2_summary = wave2_readiness.get("summary")
        require(
            isinstance(wave2_summary, dict),
            "wave2 readiness summary is recorded",
            wave2_readiness_path,
            f"actual {type(wave2_summary).__name__}",
        )
        if isinstance(wave2_summary, dict):
            expected_status = "release-ready" if wave2_release_ready is True else "live-gated"
            require(
                wave2_summary.get("status") == expected_status,
                "wave2 readiness summary status matches release_ready",
                wave2_readiness_path,
                f"expected {expected_status!r}; actual {wave2_summary.get('status')!r}",
            )
        wave2_external_plan = wave2_readiness.get("external_evidence_plan")
        require(
            isinstance(wave2_external_plan, list),
            "wave2 readiness records external evidence plan",
            wave2_readiness_path,
            f"actual {type(wave2_external_plan).__name__}",
        )
        wave2_not_ready_lanes = []
        wave2_lanes_with_external_gates = []
        wave2_lane_codes = set()
        wave2_duplicate_lane_codes = set()
        if isinstance(wave2_lanes, list):
            for index, lane_report in enumerate(wave2_lanes):
                lane_path = f"{wave2_readiness_path}#/lanes/{index}"
                require(
                    isinstance(lane_report, dict),
                    "wave2 readiness lane entry is object",
                    lane_path,
                )
                if not isinstance(lane_report, dict):
                    continue
                lane_code = lane_report.get("code")
                require(
                    isinstance(lane_code, str) and bool(lane_code.strip()),
                    "wave2 readiness lane entry records code",
                    lane_path,
                    f"actual {lane_code!r}",
                )
                lane_label = lane_code.strip() if isinstance(lane_code, str) and lane_code.strip() else f"lane-{index}"
                if isinstance(lane_code, str) and lane_code.strip():
                    normalized_lane_code = lane_code.strip()
                    if normalized_lane_code in wave2_lane_codes:
                        wave2_duplicate_lane_codes.add(normalized_lane_code)
                    wave2_lane_codes.add(normalized_lane_code)
                lane_release_ready = lane_report.get("release_ready")
                require(
                    isinstance(lane_release_ready, bool),
                    "wave2 readiness lane release_ready flag is boolean",
                    lane_path,
                    f"actual {lane_release_ready!r}",
                )
                if lane_label == "U5":
                    lane_details = lane_report.get("details")
                    lane_authority = (
                        lane_details.get("readiness_authority")
                        if isinstance(lane_details, dict)
                        and isinstance(lane_details.get("readiness_authority"), dict)
                        else {}
                    )
                    require(
                        lane_release_ready is False
                        and lane_authority.get("authoritative") is False
                        and lane_authority.get("status") == "contained-non-authoritative"
                        and lane_authority.get("containment_card") == "M0-05"
                        and lane_authority.get("release_authority") == "none",
                        "wave2 U5 legacy mobile lane is non-authoritative",
                        lane_path,
                        (
                            "legacy mobile-test cannot establish U5 candidate or release readiness; "
                            f"release_ready={lane_release_ready!r}; readiness_authority={lane_authority!r}; "
                            "a future source-bound mobile candidate checker must replace this containment contract"
                        ),
                    )
                    lane_readiness_claim_paths = invalid_contained_readiness_fields(lane_report)
                    require(
                        not lane_readiness_claim_paths,
                        "wave2 U5 legacy mobile lane contains only false readiness fields",
                        lane_path,
                        (
                            "legacy U5 readiness fields must be absent or exact JSON boolean false"
                            if not lane_readiness_claim_paths
                            else "invalid readiness fields found: "
                            + "; ".join(lane_readiness_claim_paths)
                        ),
                    )
                if lane_release_ready is not True:
                    wave2_not_ready_lanes.append(lane_label)
                lane_external_gates = lane_report.get("remaining_external_gates")
                require(
                    isinstance(lane_external_gates, list),
                    "wave2 readiness lane remaining external gates is an array",
                    lane_path,
                    f"actual {type(lane_external_gates).__name__}",
                )
                if isinstance(lane_external_gates, list) and lane_external_gates:
                    wave2_lanes_with_external_gates.append(lane_label)
        wave2_plan_lane_codes = set()
        wave2_duplicate_plan_lane_codes = set()
        wave2_external_artifact_counts = {
            "required_count": 0,
            "present_count": 0,
            "missing_count": 0,
            "release_ready_false_count": 0,
        }
        if isinstance(wave2_external_plan, list):
            for index, plan_entry in enumerate(wave2_external_plan):
                plan_path = f"{wave2_readiness_path}#/external_evidence_plan/{index}"
                require(
                    isinstance(plan_entry, dict),
                    "wave2 external evidence plan entry is object",
                    plan_path,
                )
                if not isinstance(plan_entry, dict):
                    continue
                plan_lane = plan_entry.get("lane")
                require(
                    isinstance(plan_lane, str) and bool(plan_lane.strip()),
                    "wave2 external evidence plan entry records lane",
                    plan_path,
                    f"actual {plan_lane!r}",
                )
                if isinstance(plan_lane, str) and plan_lane.strip():
                    normalized_plan_lane = plan_lane.strip()
                    if normalized_plan_lane in wave2_plan_lane_codes:
                        wave2_duplicate_plan_lane_codes.add(normalized_plan_lane)
                    wave2_plan_lane_codes.add(normalized_plan_lane)
                for field in ("goal", "owner_boundary"):
                    require(
                        isinstance(plan_entry.get(field), str) and bool(plan_entry.get(field).strip()),
                        f"wave2 external evidence plan entry records {field}",
                        plan_path,
                        f"actual {plan_entry.get(field)!r}",
                    )
                for field in ("required_artifacts", "commands"):
                    value = plan_entry.get(field)
                    require(
                        isinstance(value, list) and bool(value) and all(isinstance(item, str) and item.strip() for item in value),
                        f"wave2 external evidence plan entry records {field}",
                        plan_path,
                        f"actual {value!r}",
                    )
                artifact_issues = external_plan_required_artifact_issues(
                    plan_entry.get("required_artifacts")
                )
                require(
                    not artifact_issues,
                    "wave2 external evidence plan required artifacts are release evidence paths",
                    plan_path,
                    f"issues={artifact_issues}",
                    blocking=not local_fixture,
                )
                artifact_status_issues, artifact_status_counts = wave2_required_artifact_status_issues(
                    plan_entry.get("required_artifacts"),
                    plan_entry.get("required_artifact_status"),
                    plan_entry.get("required_artifact_summary"),
                )
                require(
                    not artifact_status_issues,
                    "wave2 external evidence plan required artifact status matches required artifacts",
                    plan_path,
                    f"issues={artifact_status_issues}",
                    blocking=not local_fixture,
                )
                for count_field, count_value in artifact_status_counts.items():
                    wave2_external_artifact_counts[count_field] += count_value
                command_issues = external_plan_command_issues(
                    plan_entry.get("commands"),
                    plan_lane,
                )
                require(
                    not command_issues,
                    "wave2 external evidence plan commands do not use local fixtures or placeholders",
                    plan_path,
                    f"issues={command_issues}",
                    blocking=not local_fixture,
                )
                authoritative_plan_fields = [
                    field
                    for field in (
                        "ok",
                        "ci_ready",
                        "status",
                        "release_ready",
                        "release_authority",
                        "release_authority_boundary",
                        "release_requirements",
                        "release_blockers",
                        "satisfies_release",
                        "local_preflight_satisfies",
                        "focused_evidence",
                        "evidence_mode",
                        "provider_scope",
                        "failure_summary",
                        "artifact",
                        "artifact_path",
                        "artifacts",
                        "evidence_artifact",
                        "generated_at",
                        "generated_at_utc",
                    )
                    if field in plan_entry
                ]
                require(
                    not authoritative_plan_fields,
                    "wave2 external evidence plan entry does not claim release evidence",
                    plan_path,
                    f"unexpected_fields={authoritative_plan_fields}",
                )
                explicit_local_satisfaction_claims = [
                    field
                    for field in (
                        "local_preflight_satisfies_release",
                        "local_fixture_satisfies_release",
                        "local_evidence_satisfies_release",
                        "focused_evidence_satisfies_release",
                    )
                    if plan_entry.get(field) is True
                ]
                require(
                    not explicit_local_satisfaction_claims,
                    "wave2 external evidence plan rejects local/focused satisfaction claims",
                    plan_path,
                    f"unexpected_true_fields={explicit_local_satisfaction_claims}",
                    blocking=not local_fixture,
                )
        if isinstance(wave2_external_plan, list):
            external_required_artifacts = wave2_readiness.get("external_required_artifacts")
            external_required_artifact_issues = wave2_artifact_summary_issues(
                external_required_artifacts,
                wave2_external_artifact_counts,
            )
            require(
                not external_required_artifact_issues,
                "wave2 readiness external required artifact summary matches per-lane artifact status",
                wave2_readiness_path,
                f"issues={external_required_artifact_issues}",
                blocking=not local_fixture,
            )
        if isinstance(wave2_lanes, list):
            expected_lane_codes = {"G1", "U2", "G2", "U5", "U6", "U7", "D6", "G4", "P1-P4"}
            require(
                expected_lane_codes.issubset(wave2_lane_codes),
                "wave2 readiness records expected lane codes",
                wave2_readiness_path,
                f"missing={sorted(expected_lane_codes - wave2_lane_codes)}",
            )
            require(
                wave2_lane_codes.issubset(expected_lane_codes),
                "wave2 readiness lane codes are recognized",
                wave2_readiness_path,
                f"unexpected={sorted(wave2_lane_codes - expected_lane_codes)}",
            )
            require(
                not wave2_duplicate_lane_codes,
                "wave2 readiness lane codes are unique",
                wave2_readiness_path,
                f"duplicates={sorted(wave2_duplicate_lane_codes)}",
            )
            expected_external_plan_codes = {"G1", "U2", "G2", "U5", "G4", "P1-P4"}
            require(
                expected_external_plan_codes.issubset(wave2_plan_lane_codes),
                "wave2 readiness external evidence plan covers live-gated lanes",
                wave2_readiness_path,
                f"missing={sorted(expected_external_plan_codes - wave2_plan_lane_codes)}",
            )
            require(
                wave2_plan_lane_codes.issubset(expected_lane_codes),
                "wave2 readiness external evidence plan lanes are recognized",
                wave2_readiness_path,
                f"unexpected={sorted(wave2_plan_lane_codes - expected_lane_codes)}",
            )
            require(
                not wave2_duplicate_plan_lane_codes,
                "wave2 readiness external evidence plan lanes are unique",
                wave2_readiness_path,
                f"duplicates={sorted(wave2_duplicate_plan_lane_codes)}",
            )
            if wave2_lanes_with_external_gates:
                require(
                    set(wave2_lanes_with_external_gates).issubset(wave2_plan_lane_codes),
                    "wave2 readiness external evidence plan covers lanes with external gates",
                    wave2_readiness_path,
                    f"missing={sorted(set(wave2_lanes_with_external_gates) - wave2_plan_lane_codes)}",
                )
        if wave2_release_ready is True:
            require(
                not wave2_external_gates,
                "release-ready wave2 readiness has no remaining external gates",
                wave2_readiness_path,
                f"remaining_external_gates={wave2_external_gates!r}",
            )
            require(
                not wave2_not_ready_lanes,
                "release-ready wave2 readiness has every lane release-ready",
                wave2_readiness_path,
                f"not_ready_lanes={wave2_not_ready_lanes}",
            )
            require(
                not wave2_lanes_with_external_gates,
                "release-ready wave2 readiness lanes have no external gates",
                wave2_readiness_path,
                f"lanes_with_external_gates={wave2_lanes_with_external_gates}",
            )
        elif isinstance(wave2_release_ready, bool):
            require(
                False,
                "retained wave2 readiness is not release-ready before strict release evidence can pass",
                wave2_readiness_path,
                (
                    "external_evidence_plan is guidance only; "
                    f"remaining_external_gates={wave2_external_gates!r}; "
                    f"not_ready_lanes={wave2_not_ready_lanes}; "
                    f"lanes_with_external_gates={wave2_lanes_with_external_gates}"
                ),
                blocking=not local_fixture,
            )
else:
    record_artifact(
        "wave2-readiness",
        wave2_readiness_file,
        required=False,
        required_in_ci=False,
        status="not-applicable",
        applicability="not-applicable",
        applicability_reason=(
            "Wave 2 readiness is report-only; release-check may retain "
            "wave2-readiness.json but it is not required for this evidence bundle"
        ),
    )


provider_graduation_file = report_dir / "provider-graduation.json"
provider_graduation_path = provider_graduation_file.as_posix()
if provider_graduation_file.is_file():
    provider_graduation = load_required(
        "provider-graduation",
        "provider-graduation.json",
        dict,
        applicability="optional",
    )
    if provider_graduation is not None:
        validate_provider_graduation_report(
            provider_graduation,
            provider_graduation_path,
            blocking=not local_fixture,
        )
else:
    record_artifact(
        "provider-graduation",
        provider_graduation_file,
        required=False,
        required_in_ci=False,
        status="not-applicable",
        applicability="not-applicable",
        applicability_reason=(
            "not required until the U4 connector graduation lane is included in release scope"
        ),
    )

governed_write_posture_file = report_dir / "governed-write-posture.json"
governed_write_posture_path = governed_write_posture_file.as_posix()
if governed_write_posture_file.is_file():
    governed_write_posture = load_required(
        "governed-write-posture",
        "governed-write-posture.json",
        dict,
        applicability="optional",
    )
    if governed_write_posture is not None:
        validate_governed_write_posture(
            governed_write_posture,
            governed_write_posture_path,
            blocking=not local_fixture,
        )
else:
    record_artifact(
        "governed-write-posture",
        governed_write_posture_file,
        required=False,
        required_in_ci=False,
        status="not-applicable",
        applicability="not-applicable",
        applicability_reason=(
            "not required until the G1 governed-write posture lane is included in release scope"
        ),
    )

chat_eval_file = report_dir / "wave4/ch6-chat-eval.json"
chat_eval_path = chat_eval_file.as_posix()
if chat_eval_file.is_file():
    chat_eval = load_required(
        "chat-eval",
        "wave4/ch6-chat-eval.json",
        dict,
        applicability="optional",
    )
    if chat_eval is not None:
        require(
            chat_eval.get("command") == "chat-eval",
            "chat-eval command is stable",
            chat_eval_path,
            f"actual {chat_eval.get('command')!r}",
        )
        require(
            chat_eval.get("lane") == "CH",
            "chat-eval lane is stable",
            chat_eval_path,
            f"actual {chat_eval.get('lane')!r}",
        )
        require(
            chat_eval.get("ok") is True,
            "chat-eval local contracts pass",
            chat_eval_path,
            f"actual {chat_eval.get('ok')!r}",
            blocking=not local_fixture,
        )
        require(
            chat_eval.get("release_ready") is False,
            "chat-eval local fixture does not claim release readiness",
            chat_eval_path,
            f"actual {chat_eval.get('release_ready')!r}",
        )
        require(
            chat_eval.get("mode") == "local-fixture",
            "chat-eval mode is local-fixture",
            chat_eval_path,
            f"actual {chat_eval.get('mode')!r}",
        )
        red_team = chat_eval.get("red_team")
        require(
            isinstance(red_team, dict),
            "chat-eval records red-team summary",
            chat_eval_path,
            f"actual {type(red_team).__name__}",
        )
        if isinstance(red_team, dict):
            require(
                red_team.get("ok") is True,
                "chat-eval red-team summary passes",
                chat_eval_path,
                f"actual {red_team.get('ok')!r}",
                blocking=not local_fixture,
            )
            require(
                red_team.get("leaks_found") == 0,
                "chat-eval red-team leak count is zero",
                chat_eval_path,
                f"actual {red_team.get('leaks_found')!r}",
                blocking=not local_fixture,
            )
        judge = chat_eval.get("judge")
        require(
            isinstance(judge, dict),
            "chat-eval records judge/live evidence posture",
            chat_eval_path,
            f"actual {type(judge).__name__}",
        )
        if isinstance(judge, dict):
            require(
                judge.get("enabled") is False,
                "chat-eval local fixture judge is disabled",
                chat_eval_path,
                f"actual {judge.get('enabled')!r}",
            )
            require(
                judge.get("disposition") == "local-fixture",
                "chat-eval local fixture judge disposition is stable",
                chat_eval_path,
                f"actual {judge.get('disposition')!r}",
            )
else:
    record_artifact(
        "chat-eval",
        chat_eval_file,
        required=False,
        required_in_ci=False,
        status="not-applicable",
        applicability="not-applicable",
        applicability_reason=(
            "not required until the CH6 chat-eval lane is included in release scope"
        ),
    )


chat_eval_file = report_dir / "chat-eval.json"
chat_eval_path = chat_eval_file.as_posix()
if chat_eval_file.is_file():
    chat_eval = load_required(
        "chat-eval",
        "chat-eval.json",
        dict,
        applicability="optional",
    )
    if chat_eval is not None:
        chat_eval_blocking = not local_fixture
        require(
            chat_eval.get("command") == "chat-eval",
            "chat-eval command is stable",
            chat_eval_path,
            f"actual {chat_eval.get('command')!r}",
            blocking=chat_eval_blocking,
        )
        require(
            chat_eval.get("lane") == "CH",
            "chat-eval lane is CH",
            chat_eval_path,
            f"actual {chat_eval.get('lane')!r}",
            blocking=chat_eval_blocking,
        )
        require(
            chat_eval.get("ok") is True,
            "chat-eval ok flag is true",
            chat_eval_path,
            f"actual {chat_eval.get('ok')!r}",
            blocking=chat_eval_blocking,
        )
        require(
            chat_eval.get("release_ready") is False,
            "chat-eval local fixture report does not claim release readiness",
            chat_eval_path,
            f"actual {chat_eval.get('release_ready')!r}",
            blocking=chat_eval_blocking,
        )
        require(
            chat_eval.get("mode") in ("local-fixture", "deterministic-certified", "judge-certified", "recorded-only", "unsupported"),
            "chat-eval disposition is valid",
            chat_eval_path,
            f"actual {chat_eval.get('mode')!r}",
            blocking=chat_eval_blocking,
        )
        validate_generated_timestamp(
            "chat-eval",
            chat_eval,
            chat_eval_path,
            CHAT_EVAL_MAX_AGE,
            CHAT_EVAL_MAX_AGE_HOURS,
            blocking=chat_eval_blocking,
        )
        inputs = chat_eval.get("inputs")
        require(
            isinstance(inputs, dict),
            "chat-eval inputs are recorded",
            chat_eval_path,
            blocking=chat_eval_blocking,
        )
        if isinstance(inputs, dict):
            require(
                isinstance(inputs.get("fixture_dir"), str) and bool(inputs.get("fixture_dir")),
                "chat-eval records fixture directory",
                f"{chat_eval_path}#/inputs",
                blocking=chat_eval_blocking,
            )
            require(
                isinstance(inputs.get("provenance_counts"), dict),
                "chat-eval records provenance counts",
                f"{chat_eval_path}#/inputs",
                blocking=chat_eval_blocking,
            )
        checks_value = chat_eval.get("checks")
        require(
            isinstance(checks_value, list) and bool(checks_value),
            "chat-eval records deterministic checks",
            chat_eval_path,
            blocking=chat_eval_blocking,
        )
        if isinstance(checks_value, list):
            expected_chat_checks = {
                "chat_transcript_schema_contract",
                "chat_tool_registry_binding_contract",
                "chat_citation_resolvability_contract",
                "chat_tenant_isolation_contract",
                "chat_write_gate_contract",
                "chat_replay_determinism_contract",
            }
            actual_chat_checks = {
                check.get("name")
                for check in checks_value
                if isinstance(check, dict) and isinstance(check.get("name"), str)
            }
            require(
                expected_chat_checks.issubset(actual_chat_checks),
                "chat-eval records all deterministic contract checks",
                chat_eval_path,
                f"missing {sorted(expected_chat_checks - actual_chat_checks)}",
                blocking=chat_eval_blocking,
            )
            for index, check in enumerate(checks_value):
                if isinstance(check, dict):
                    require(
                        check.get("ok") is True,
                        "chat-eval deterministic check is ok",
                        f"{chat_eval_path}#/checks/{index}",
                        f"actual {check.get('ok')!r}",
                        blocking=chat_eval_blocking,
                    )
        red_team = chat_eval.get("red_team")
        require(
            isinstance(red_team, dict),
            "chat-eval red-team summary is recorded",
            chat_eval_path,
            blocking=chat_eval_blocking,
        )
        if isinstance(red_team, dict):
            require(
                red_team.get("ok") is True,
                "chat-eval red-team leak check is ok",
                f"{chat_eval_path}#/red_team",
                f"actual {red_team.get('ok')!r}",
                blocking=chat_eval_blocking,
            )
            require(
                red_team.get("leaks_found") == 0,
                "chat-eval red-team finds zero leaks",
                f"{chat_eval_path}#/red_team",
                f"actual {red_team.get('leaks_found')!r}",
                blocking=chat_eval_blocking,
            )
        artifacts_value = chat_eval.get("artifacts")
        require(
            isinstance(artifacts_value, list) and bool(artifacts_value),
            "chat-eval records sha256 sub-artifacts",
            chat_eval_path,
            blocking=chat_eval_blocking,
        )
        if isinstance(artifacts_value, list):
            for index, artifact in enumerate(artifacts_value):
                validate_retained_file_artifact(
                    "chat-eval",
                    artifact,
                    f"{chat_eval_path}#/artifacts/{index}",
                    blocking=chat_eval_blocking,
                    require_ok_flag=False,
                )
        judge = chat_eval.get("judge")
        require(
            isinstance(judge, dict),
            "chat-eval judge posture is recorded",
            chat_eval_path,
            blocking=chat_eval_blocking,
        )
        if isinstance(judge, dict):
            require(
                judge.get("disposition") in ("local-fixture", "deterministic-certified", "judge-certified", "recorded-only", "unsupported"),
                "chat-eval judge disposition is valid",
                f"{chat_eval_path}#/judge",
                f"actual {judge.get('disposition')!r}",
                blocking=chat_eval_blocking,
            )
else:
    record_artifact(
        "chat-eval",
        chat_eval_file,
        required=False,
        required_in_ci=False,
        status="not-applicable",
        applicability="not-applicable",
        applicability_reason=(
            "not required until the CH6 chat-eval lane is included in product release scope"
        ),
    )


release_identity_file = report_dir / "release-identity.json"
release_identity_path = release_identity_file.as_posix()
release_identity_check_entry = (
    release_check_entries_by_name.get("release-identity")
    if isinstance(release_check_entries_by_name, dict)
    else None
)
release_identity_required = (
    release_mode_requires_release_identity()
    or release_identity_file.is_file()
    or isinstance(release_identity_check_entry, dict)
)
if release_identity_required:
    release_identity = load_required(
        "release-identity",
        "release-identity.json",
        dict,
        missing_detail=(
            "release identity evidence is required when release-check retains "
            "the release-identity child check or APPFW_RELEASE_REQUIRE_RELEASE_IDENTITY is set"
        ),
        applicability="required" if release_mode_requires_release_identity() else "optional",
    )
    if release_identity is not None:
        require(
            release_identity.get("command") == "release-identity",
            "release identity command is stable",
            release_identity_path,
            f"actual {release_identity.get('command')!r}",
        )
        require(
            isinstance(release_identity.get("ok"), bool),
            "release identity ok flag is boolean",
            release_identity_path,
            f"actual {release_identity.get('ok')!r}",
        )
        require(
            isinstance(release_identity.get("release_ready"), bool),
            "release identity release_ready flag is boolean",
            release_identity_path,
            f"actual {release_identity.get('release_ready')!r}",
        )
        require(
            isinstance(release_identity.get("release_identity_required"), bool),
            "release identity required flag is boolean",
            release_identity_path,
            f"actual {release_identity.get('release_identity_required')!r}",
        )
        if release_mode_requires_release_identity():
            require(
                release_identity.get("release_identity_required") is True,
                "release identity evidence records required release mode",
                release_identity_path,
                f"actual {release_identity.get('release_identity_required')!r}",
                blocking=not local_fixture,
            )
        release_identity_generated_at = validate_generated_timestamp(
            "release identity",
            release_identity,
            release_identity_path,
            RELEASE_IDENTITY_MAX_AGE,
            RELEASE_IDENTITY_MAX_AGE_HOURS,
            blocking=not local_fixture,
        )
        if release_identity_generated_at is not None and release_check_generated_at is not None:
            require(
                release_identity_generated_at <= release_check_generated_at + EVIDENCE_TIMESTAMP_SKEW,
                "release identity generated timestamp does not postdate release-check",
                release_identity_path,
                (
                    f"release_identity={release_identity_generated_at.isoformat().replace('+00:00', 'Z')}, "
                    f"release_check={release_check_generated_at.isoformat().replace('+00:00', 'Z')}"
                ),
                blocking=not local_fixture,
            )
        git = release_identity.get("git")
        require(
            isinstance(git, dict),
            "release identity git block is recorded",
            release_identity_path,
        )
        if isinstance(git, dict):
            sha = git.get("sha")
            release_tag = git.get("release_tag")
            exact_v_tags = git.get("exact_v_tags")
            require(
                isinstance(sha, str) and bool(re.fullmatch(r"[0-9a-f]{40}", sha)),
                "release identity git sha is recorded",
                release_identity_path,
                f"actual {sha!r}",
                blocking=not local_fixture,
            )
            require(
                isinstance(exact_v_tags, list),
                "release identity exact v-tags list is recorded",
                release_identity_path,
                f"actual {exact_v_tags!r}",
            )
            if isinstance(exact_v_tags, list):
                invalid_tags = [
                    tag
                    for tag in exact_v_tags
                    if not (isinstance(tag, str) and re.fullmatch(r"v[0-9][A-Za-z0-9._+-]*", tag))
                ]
                require(
                    not invalid_tags,
                    "release identity exact v-tags use release tag format",
                    release_identity_path,
                    f"invalid={invalid_tags}",
                )
            if release_tag is not None:
                require(
                    isinstance(release_tag, str)
                    and bool(re.fullmatch(r"v[0-9][A-Za-z0-9._+-]*", release_tag)),
                    "release identity release tag uses v* format when present",
                    release_identity_path,
                    f"actual {release_tag!r}",
                    blocking=not local_fixture,
                )
        require(
            isinstance(release_identity.get("decision_file"), str)
            and bool(release_identity.get("decision_file").strip()),
            "release identity decision file path is recorded",
            release_identity_path,
            f"actual {release_identity.get('decision_file')!r}",
        )
        decision_artifact = release_identity.get("decision_artifact")
        require(
            isinstance(decision_artifact, dict),
            "release identity decision artifact summary is recorded",
            release_identity_path,
        )
        if isinstance(decision_artifact, dict):
            require(
                isinstance(decision_artifact.get("present"), bool),
                "release identity decision artifact present flag is boolean",
                f"{release_identity_path}#/decision_artifact",
                f"actual {decision_artifact.get('present')!r}",
            )
            require(
                isinstance(decision_artifact.get("ok"), bool),
                "release identity decision artifact ok flag is boolean",
                f"{release_identity_path}#/decision_artifact",
                f"actual {decision_artifact.get('ok')!r}",
            )
            if decision_artifact.get("present") is True or release_identity.get("release_ready") is True:
                resolved_decision_artifact = validate_retained_file_artifact(
                    "release identity decision",
                    decision_artifact,
                    f"{release_identity_path}#/decision_artifact",
                    blocking=not local_fixture,
                    required_parent=report_dir,
                    required_parent_label="report directory",
                    require_ok_flag=True,
                )
                decision_file_text = release_identity.get("decision_file")
                if resolved_decision_artifact is not None and isinstance(decision_file_text, str):
                    resolved_decision_file = resolve_retained_path(decision_file_text)
                    if resolved_decision_file is not None:
                        require(
                            resolved_decision_artifact.resolve() == resolved_decision_file.resolve(),
                            "release identity decision artifact path matches decision_file",
                            release_identity_path,
                            (
                                f"decision_file={resolved_decision_file.as_posix()}, "
                                f"artifact={resolved_decision_artifact.as_posix()}"
                            ),
                            blocking=not local_fixture,
                        )
        license_files = release_identity.get("license_files")
        require(
            isinstance(license_files, list),
            "release identity license files list is recorded",
            release_identity_path,
            f"actual {license_files!r}",
        )
        package_metadata = release_identity.get("package_metadata")
        require(
            isinstance(package_metadata, list),
            "release identity package metadata list is recorded",
            release_identity_path,
            f"actual {package_metadata!r}",
        )
        checks_value = release_identity.get("checks")
        require(
            isinstance(checks_value, list) and bool(checks_value),
            "release identity records checks",
            release_identity_path,
            f"actual {checks_value!r}",
        )
        release_identity_check_names = []
        release_identity_checks_by_name = {}
        release_identity_failing_checks = []
        if isinstance(checks_value, list):
            for index, check in enumerate(checks_value):
                check_path = f"{release_identity_path}#/checks/{index}"
                require(
                    isinstance(check, dict),
                    "release identity check entry is object",
                    check_path,
                )
                if not isinstance(check, dict):
                    continue
                check_name = check.get("name")
                check_ok = check.get("ok")
                require(
                    isinstance(check_name, str) and bool(check_name.strip()),
                    "release identity check entry records name",
                    check_path,
                    f"actual {check_name!r}",
                )
                if isinstance(check_name, str) and check_name.strip():
                    normalized_check_name = check_name.strip()
                    release_identity_check_names.append(normalized_check_name)
                    release_identity_checks_by_name[normalized_check_name] = check
                require(
                    isinstance(check_ok, bool),
                    "release identity check entry ok flag is boolean",
                    check_path,
                    f"actual {check_ok!r}",
                )
                if check_ok is False:
                    release_identity_failing_checks.append(
                        check_name.strip()
                        if isinstance(check_name, str) and check_name.strip()
                        else f"checks[{index}]"
                    )
            duplicate_release_identity_checks = duplicate_values(release_identity_check_names)
            require(
                not duplicate_release_identity_checks,
                "release identity checks are unique",
                release_identity_path,
                f"duplicates={duplicate_release_identity_checks}",
            )
            required_release_identity_checks = {
                "release-identity-required",
                "v-tag",
                "license-or-notice-file",
                "package-license-metadata",
                "decision-artifact",
                "decision-artifact-retained",
                "release-authority",
                "decision-tag-match",
                "decision-license",
                "release-notes",
                "distribution-artifacts",
            }
            seen_release_identity_checks = set(release_identity_check_names)
            require(
                required_release_identity_checks.issubset(seen_release_identity_checks),
                "release identity includes required checks",
                release_identity_path,
                f"missing={sorted(required_release_identity_checks - seen_release_identity_checks)}",
                blocking=not local_fixture,
            )
        release_identity_claims_ready = release_identity.get("ok") is True or release_identity.get("release_ready") is True
        if release_identity_claims_ready:
            require(
                bool(license_files),
                "release identity ready claim records license/notice files",
                release_identity_path,
                f"actual {license_files!r}",
                blocking=not local_fixture,
            )
            if isinstance(license_files, list):
                for index, license_file in enumerate(license_files):
                    license_path = f"{release_identity_path}#/license_files/{index}"
                    require(
                        isinstance(license_file, str) and bool(license_file.strip()),
                        "release identity license file entry is non-empty",
                        license_path,
                        f"actual {license_file!r}",
                        blocking=not local_fixture,
                    )
                    resolved_license = resolve_retained_path(license_file)
                    require(
                        resolved_license is not None,
                        "release identity license file exists",
                        license_path,
                        resolved_license.as_posix()
                        if resolved_license is not None
                        else f"missing retained license/notice file: {license_file}",
                        blocking=not local_fixture,
                    )
                    if resolved_license is not None:
                        retained_license_location = path_is_within_directory(
                            resolved_license,
                            repo_root,
                        ) or path_is_within_directory(resolved_license, report_dir)
                        require(
                            retained_license_location,
                            "release identity license file is retained in repo or report directory",
                            license_path,
                            resolved_license.as_posix(),
                            blocking=not local_fixture,
                        )
                        require(
                            resolved_license.stat().st_size > 0,
                            "release identity license file is non-empty",
                            license_path,
                            f"actual {resolved_license.stat().st_size}",
                            blocking=not local_fixture,
                        )
            require(
                bool(package_metadata),
                "release identity ready claim records package metadata",
                release_identity_path,
                f"actual {package_metadata!r}",
                blocking=not local_fixture,
            )
            if isinstance(package_metadata, list):
                for index, metadata in enumerate(package_metadata):
                    metadata_path = f"{release_identity_path}#/package_metadata/{index}"
                    require(
                        isinstance(metadata, dict),
                        "release identity package metadata entry is object",
                        metadata_path,
                        blocking=not local_fixture,
                    )
                    if not isinstance(metadata, dict):
                        continue
                    manifest_path = metadata.get("path")
                    require(
                        isinstance(manifest_path, str) and bool(manifest_path.strip()),
                        "release identity package metadata entry records manifest path",
                        metadata_path,
                        f"actual {manifest_path!r}",
                        blocking=not local_fixture,
                    )
                    license_value = metadata.get("license")
                    license_file_value = metadata.get("license_file")
                    metadata_has_license = (
                        isinstance(license_value, str)
                        and bool(license_value.strip())
                    ) or (
                        isinstance(license_file_value, str)
                        and bool(license_file_value.strip())
                    )
                    require(
                        metadata_has_license,
                        "release identity package metadata entry records license metadata",
                        metadata_path,
                        f"license={license_value!r}, license_file={license_file_value!r}",
                        blocking=not local_fixture,
                    )
            distribution_check = release_identity_checks_by_name.get("distribution-artifacts")
            require(
                isinstance(distribution_check, dict),
                "release identity ready claim records distribution-artifacts check",
                release_identity_path,
                blocking=not local_fixture,
            )
            distribution_artifacts = (
                distribution_check.get("artifacts")
                if isinstance(distribution_check, dict)
                else None
            )
            require(
                isinstance(distribution_artifacts, list) and bool(distribution_artifacts),
                "release identity ready claim records distribution artifacts",
                release_identity_path,
                f"actual {distribution_artifacts!r}",
                blocking=not local_fixture,
            )
            valid_distribution_artifacts = 0
            if isinstance(distribution_artifacts, list):
                for index, artifact in enumerate(distribution_artifacts):
                    artifact_path = f"{release_identity_path}#/checks/distribution-artifacts/artifacts/{index}"
                    require(
                        isinstance(artifact, dict),
                        "release identity distribution artifact entry is object",
                        artifact_path,
                        blocking=not local_fixture,
                    )
                    if not isinstance(artifact, dict):
                        continue
                    name_value = artifact.get("name")
                    require(
                        isinstance(name_value, str) and bool(name_value.strip()),
                        "release identity distribution artifact records name",
                        artifact_path,
                        f"actual {name_value!r}",
                        blocking=not local_fixture,
                    )
                    artifact_ok = artifact.get("ok") is True
                    require(
                        artifact_ok,
                        "release identity distribution artifact ok flag is true",
                        artifact_path,
                        f"actual {artifact.get('ok')!r}",
                        blocking=not local_fixture,
                    )
                    url_value = artifact.get("url")
                    url_ok = isinstance(url_value, str) and url_value.startswith(
                        ("https://", "http://")
                    )
                    path_value = artifact.get("path")
                    path_ok = isinstance(path_value, str) and bool(path_value.strip())
                    resolved_distribution = resolve_retained_path(path_value) if path_ok else None
                    if path_ok:
                        require(
                            resolved_distribution is not None,
                            "release identity distribution artifact retained path exists",
                            artifact_path,
                            resolved_distribution.as_posix()
                            if resolved_distribution is not None
                            else f"missing retained distribution artifact: {path_value}",
                            blocking=not local_fixture,
                        )
                    distribution_has_locator = url_ok or resolved_distribution is not None
                    require(
                        distribution_has_locator,
                        "release identity distribution artifact has URL or retained path",
                        artifact_path,
                        f"url={url_value!r}, path={path_value!r}",
                        blocking=not local_fixture,
                    )
                    if resolved_distribution is not None:
                        require(
                            path_is_within_directory(resolved_distribution, report_dir),
                            "release identity distribution artifact is retained in report directory",
                            artifact_path,
                            (
                                f"{resolved_distribution.as_posix()} is retained under {report_dir.as_posix()}"
                                if path_is_within_directory(resolved_distribution, report_dir)
                                else f"distribution artifact must be retained under {report_dir.as_posix()}: {resolved_distribution.as_posix()}"
                            ),
                            blocking=not local_fixture,
                        )
                        expected_sha = artifact.get("sha256")
                        actual_sha = f"sha256:{sha256_digest(resolved_distribution)}"
                        require(
                            expected_sha == actual_sha,
                            "release identity distribution artifact sha256 matches retained file",
                            artifact_path,
                            f"expected {expected_sha or '<missing>'}, actual {actual_sha}",
                            blocking=not local_fixture,
                        )
                        expected_bytes = artifact.get("bytes")
                        actual_bytes = resolved_distribution.stat().st_size
                        require(
                            expected_bytes == actual_bytes,
                            "release identity distribution artifact byte size matches retained file",
                            artifact_path,
                            f"expected {expected_bytes!r}, actual {actual_bytes}",
                            blocking=not local_fixture,
                        )
                        require(
                            actual_bytes > 0,
                            "release identity distribution artifact retained file is non-empty",
                            artifact_path,
                            f"actual {actual_bytes}",
                            blocking=not local_fixture,
                        )
                    if (
                        artifact_ok
                        and isinstance(name_value, str)
                        and bool(name_value.strip())
                        and distribution_has_locator
                    ):
                        valid_distribution_artifacts += 1
            require(
                valid_distribution_artifacts > 0,
                "release identity ready claim has at least one usable distribution artifact",
                release_identity_path,
                f"valid_distribution_artifacts={valid_distribution_artifacts}",
                blocking=not local_fixture,
            )
            require(
                not release_identity_failing_checks,
                "release identity ok claim has no failing checks",
                release_identity_path,
                f"failing checks={release_identity_failing_checks}",
                blocking=not local_fixture,
            )
        release_blockers_value = release_identity.get("release_blockers")
        require(
            isinstance(release_blockers_value, list),
            "release identity release blockers are recorded",
            release_identity_path,
            f"actual {release_blockers_value!r}",
        )
        release_identity_blockers = []
        if isinstance(release_blockers_value, list):
            for index, blocker in enumerate(release_blockers_value):
                require(
                    isinstance(blocker, str) and bool(blocker.strip()),
                    "release identity release blocker entry is non-empty",
                    f"{release_identity_path}#/release_blockers/{index}",
                    f"actual {blocker!r}",
                )
                if isinstance(blocker, str) and blocker.strip():
                    release_identity_blockers.append(blocker.strip())
            duplicate_release_identity_blockers = duplicate_values(release_identity_blockers)
            require(
                not duplicate_release_identity_blockers,
                "release identity release blockers are unique",
                release_identity_path,
                f"duplicates={duplicate_release_identity_blockers}",
            )
        failure_summary = release_identity.get("failure_summary")
        require(
            isinstance(failure_summary, dict),
            "release identity failure summary is recorded",
            release_identity_path,
        )
        release_identity_root_causes = []
        if isinstance(failure_summary, dict):
            root_causes = failure_summary.get("root_causes")
            require(
                isinstance(root_causes, list),
                "release identity failure summary root causes are recorded",
                release_identity_path,
            )
            if isinstance(root_causes, list):
                release_identity_root_causes = [
                    cause.strip()
                    for cause in root_causes
                    if isinstance(cause, str) and cause.strip()
                ]
                require(
                    len(release_identity_root_causes) == len(root_causes),
                    "release identity failure summary root causes are non-empty strings",
                    release_identity_path,
                    f"actual {root_causes!r}",
                )
        require(
            release_identity_root_causes == release_identity_blockers,
            "release identity failure summary matches release blockers",
            release_identity_path,
            f"root_causes={release_identity_root_causes}; release_blockers={release_identity_blockers}",
            blocking=not local_fixture,
        )
        remediation_ids = validate_remediation_items(
            "release-identity",
            release_identity.get("remediation_work_items"),
            release_identity_path,
            blocking=not local_fixture,
        )
        if release_identity.get("release_ready") is True:
            require(
                release_identity.get("ok") is True,
                "release identity release-ready claim is ok",
                release_identity_path,
                f"actual ok={release_identity.get('ok')!r}",
                blocking=not local_fixture,
            )
            require(
                release_identity.get("release_identity_required") is True,
                "release identity release-ready claim required evidence mode",
                release_identity_path,
                f"actual {release_identity.get('release_identity_required')!r}",
                blocking=not local_fixture,
            )
            require(
                not release_identity_blockers,
                "release identity release-ready claim has no blockers",
                release_identity_path,
                f"release_blockers={release_identity_blockers}",
                blocking=not local_fixture,
            )
        else:
            required_release_identity_remediation = {"LIVE-004", "LIVE-014"}
            require(
                bool(release_identity_blockers),
                "release identity records blockers when not release-ready",
                release_identity_path,
                blocking=not local_fixture,
            )
            require(
                required_release_identity_remediation.issubset(remediation_ids),
                "release identity remediation covers release blockers",
                release_identity_path,
                f"missing={sorted(required_release_identity_remediation - remediation_ids)}",
                blocking=not local_fixture,
            )


bitbucket_gate_file = report_dir / "bitbucket-release-gate.json"
bitbucket_gate_path = bitbucket_gate_file.as_posix()
bitbucket_gate = None
bitbucket_gate_generated_at = None
normalized_bitbucket_blockers = []
if bitbucket_gate_file.is_file():
    record_artifact(
        "bitbucket-release-gate",
        bitbucket_gate_file,
        required=False,
        required_in_ci=False,
        status="present",
        applicability="optional",
        applicability_reason=(
            "validated when retained; bitbucket-release-gate writes this artifact "
            "after release-evidence-check in normal CI execution"
        ),
    )
    try:
        bitbucket_gate = json.loads(bitbucket_gate_file.read_text(encoding="utf-8"))
        require(
            isinstance(bitbucket_gate, dict),
            "bitbucket release gate evidence is object",
            bitbucket_gate_path,
        )
    except Exception as exc:  # pragma: no cover - exercised by shell usage.
        bitbucket_gate = None
        require(
            False,
            "bitbucket release gate evidence parses as JSON",
            bitbucket_gate_path,
            str(exc),
        )
    if isinstance(bitbucket_gate, dict):
        require(
            bitbucket_gate.get("command") == "bitbucket-release-gate",
            "bitbucket release gate command is stable",
            bitbucket_gate_path,
            f"actual {bitbucket_gate.get('command')!r}",
        )
        require(
            isinstance(bitbucket_gate.get("ok"), bool),
            "bitbucket release gate ok flag is boolean",
            bitbucket_gate_path,
            f"actual {bitbucket_gate.get('ok')!r}",
        )
        require(
            isinstance(bitbucket_gate.get("release_ready"), bool),
            "bitbucket release gate release_ready flag is boolean",
            bitbucket_gate_path,
            f"actual {bitbucket_gate.get('release_ready')!r}",
        )
        bitbucket_focused_evidence = bitbucket_gate.get("focused_evidence") is True
        if "focused_evidence" in bitbucket_gate:
            require(
                isinstance(bitbucket_gate.get("focused_evidence"), bool),
                "bitbucket release gate focused_evidence flag is boolean",
                bitbucket_gate_path,
                f"actual {bitbucket_gate.get('focused_evidence')!r}",
            )
        if bitbucket_focused_evidence:
            require(
                isinstance(bitbucket_gate.get("ci_ready"), bool),
                "focused bitbucket release gate ci_ready flag is boolean",
                bitbucket_gate_path,
                f"actual {bitbucket_gate.get('ci_ready')!r}",
            )
            require(
                bitbucket_gate.get("release_ready") is False,
                "focused bitbucket release gate cannot claim release_ready",
                bitbucket_gate_path,
                f"actual {bitbucket_gate.get('release_ready')!r}",
                blocking=not local_fixture,
            )
            require(
                bitbucket_gate.get("release_authority") == "focused-ci-only",
                "focused bitbucket release gate records non-promotable authority",
                bitbucket_gate_path,
                f"actual {bitbucket_gate.get('release_authority')!r}",
                blocking=not local_fixture,
            )
        else:
            require(
                bitbucket_gate.get("ok") == bitbucket_gate.get("release_ready"),
                "bitbucket release gate ok flag matches release_ready",
                bitbucket_gate_path,
                f"ok={bitbucket_gate.get('ok')!r}; release_ready={bitbucket_gate.get('release_ready')!r}",
            )
        bitbucket_authority_boundary = bitbucket_gate.get("release_authority_boundary")
        require(
            isinstance(bitbucket_authority_boundary, dict),
            "bitbucket release gate records release authority boundary",
            bitbucket_gate_path,
            blocking=not local_fixture,
        )
        if isinstance(bitbucket_authority_boundary, dict):
            require(
                bitbucket_authority_boundary.get("focused_evidence_satisfies_release") is False,
                "bitbucket release gate boundary rejects focused evidence as release evidence",
                bitbucket_gate_path,
                f"actual {bitbucket_authority_boundary.get('focused_evidence_satisfies_release')!r}",
                blocking=not local_fixture,
            )
            require(
                bitbucket_authority_boundary.get("strict_release_gate_required_for_promotion") is True,
                "bitbucket release gate boundary requires strict promotion gate",
                bitbucket_gate_path,
                f"actual {bitbucket_authority_boundary.get('strict_release_gate_required_for_promotion')!r}",
                blocking=not local_fixture,
            )
            if bitbucket_focused_evidence:
                require(
                    bitbucket_authority_boundary.get("strict_release_gate_satisfied") is False,
                    "focused bitbucket release gate boundary does not satisfy strict gate",
                    bitbucket_gate_path,
                    f"actual {bitbucket_authority_boundary.get('strict_release_gate_satisfied')!r}",
                    blocking=not local_fixture,
                )
        require(
            isinstance(bitbucket_gate.get("generated_at_utc"), str),
            "bitbucket release gate generated timestamp is present",
            bitbucket_gate_path,
        )
        bitbucket_gate_generated_at = validate_generated_timestamp(
            "bitbucket release gate",
            bitbucket_gate,
            bitbucket_gate_path,
            BITBUCKET_RELEASE_GATE_MAX_AGE,
            BITBUCKET_RELEASE_GATE_MAX_AGE_HOURS,
            blocking=not local_fixture,
        )
        if bitbucket_gate_generated_at is not None and release_check_generated_at is not None:
            require(
                bitbucket_gate_generated_at + EVIDENCE_TIMESTAMP_SKEW >= release_check_generated_at,
                "bitbucket release gate generated timestamp does not predate release-check",
                bitbucket_gate_path,
                (
                    "bitbucket-release-gate must be generated after the release-check "
                    f"evidence it summarizes; bitbucket={bitbucket_gate_generated_at.isoformat().replace('+00:00', 'Z')}, "
                    f"release_check={release_check_generated_at.isoformat().replace('+00:00', 'Z')}"
                ),
                blocking=not local_fixture,
            )
        bitbucket_requirements = bitbucket_gate.get("release_requirements")
        require(
            isinstance(bitbucket_requirements, dict),
            "bitbucket release gate records release requirements",
            bitbucket_gate_path,
            blocking=not local_fixture,
        )
        expected_bitbucket_requirements = {
            "security_assurance_decision_required": "security assurance decision",
            "production_attestations_required": "production attestations",
            "ops_certification_required": "operations certification",
            "live_ops_evidence_required": "live operations evidence",
            "pds_baseline_evidence_required": "PDS baseline evidence",
            "performance_evidence_required": "performance evidence",
            "release_identity_required": "release identity evidence",
        }
        if isinstance(bitbucket_requirements, dict):
            for key, label in expected_bitbucket_requirements.items():
                require(
                    bitbucket_requirements.get(key) is True,
                    f"bitbucket release gate requires {label}",
                    bitbucket_gate_path,
                    f"actual {bitbucket_requirements.get(key)!r}",
                    blocking=not local_fixture,
                )
        bitbucket_release_blockers = bitbucket_gate.get("release_blockers")
        require(
            isinstance(bitbucket_release_blockers, list),
            "bitbucket release gate release_blockers are recorded",
            bitbucket_gate_path,
        )
        if isinstance(bitbucket_release_blockers, list):
            for index, blocker in enumerate(bitbucket_release_blockers):
                require(
                    isinstance(blocker, str) and bool(blocker.strip()),
                    "bitbucket release gate release_blocker entry is non-empty",
                    f"{bitbucket_gate_path}#/release_blockers/{index}",
                    f"actual {blocker!r}",
                )
                if isinstance(blocker, str) and blocker.strip():
                    normalized_bitbucket_blockers.append(blocker.strip())
            duplicate_bitbucket_blockers = duplicate_values(normalized_bitbucket_blockers)
            require(
                not duplicate_bitbucket_blockers,
                "bitbucket release gate release_blockers are unique",
                bitbucket_gate_path,
                f"duplicates={duplicate_bitbucket_blockers}",
            )
        bitbucket_failure_summary = bitbucket_gate.get("failure_summary")
        require(
            isinstance(bitbucket_failure_summary, dict),
            "bitbucket release gate failure summary is recorded",
            bitbucket_gate_path,
        )
        if isinstance(bitbucket_failure_summary, dict):
            root_causes = bitbucket_failure_summary.get("root_causes")
            require(
                isinstance(root_causes, list),
                "bitbucket release gate failure summary root causes are recorded",
                bitbucket_gate_path,
            )
            if isinstance(root_causes, list):
                normalized_root_causes = [
                    value.strip()
                    for value in root_causes
                    if isinstance(value, str) and value.strip()
                ]
                require(
                    normalized_root_causes == normalized_bitbucket_blockers,
                    "bitbucket release gate failure summary matches release blockers",
                    bitbucket_gate_path,
                    f"root_causes={normalized_root_causes}; release_blockers={normalized_bitbucket_blockers}",
                    blocking=not local_fixture,
                )
        if bitbucket_gate.get("release_ready") is True:
            require(
                bitbucket_gate.get("focused_evidence") is not True,
                "bitbucket release gate ready claim is not focused evidence",
                bitbucket_gate_path,
                f"actual {bitbucket_gate.get('focused_evidence')!r}",
                blocking=not local_fixture,
            )
            require(
                bitbucket_gate.get("release_authority") == "strict-managed-release",
                "bitbucket release gate ready claim records strict managed authority",
                bitbucket_gate_path,
                f"actual {bitbucket_gate.get('release_authority')!r}",
                blocking=not local_fixture,
            )
            if isinstance(bitbucket_authority_boundary, dict):
                require(
                    bitbucket_authority_boundary.get("strict_release_gate_satisfied") is True,
                    "bitbucket release gate ready claim satisfies strict authority boundary",
                    bitbucket_gate_path,
                    f"actual {bitbucket_authority_boundary.get('strict_release_gate_satisfied')!r}",
                    blocking=not local_fixture,
                )
                require(
                    bitbucket_authority_boundary.get("managed_release_requirements_satisfied") is True,
                    "bitbucket release gate ready claim satisfies managed release requirements",
                    bitbucket_gate_path,
                    f"actual {bitbucket_authority_boundary.get('managed_release_requirements_satisfied')!r}",
                    blocking=not local_fixture,
                )
            retained_failure_fields = [
                field
                for field in ("failed_stage", "exit_status")
                if field in bitbucket_gate
            ]
            require(
                not retained_failure_fields,
                "bitbucket release gate ready claim has no retained failure fields",
                bitbucket_gate_path,
                f"fields={retained_failure_fields}",
                blocking=not local_fixture,
            )
            require(
                not normalized_bitbucket_blockers,
                "bitbucket release gate ready claim has no retained blockers",
                bitbucket_gate_path,
                f"release_blockers={normalized_bitbucket_blockers}",
                blocking=not local_fixture,
            )
            require(
                isinstance(release_check, dict) and release_check.get("release_ready") is True,
                "bitbucket release gate ready claim matches retained release-check",
                bitbucket_gate_path,
                f"release-check release_ready={release_check.get('release_ready') if isinstance(release_check, dict) else '<missing>'}",
                blocking=not local_fixture,
            )
            bitbucket_release_check = bitbucket_gate.get("release_check")
            require(
                isinstance(bitbucket_release_check, dict),
                "bitbucket release gate release-check summary is recorded",
                bitbucket_gate_path,
                blocking=not local_fixture,
            )
            if isinstance(bitbucket_release_check, dict):
                require(
                    bitbucket_release_check.get("ok") is True,
                    "bitbucket release gate release-check summary is green",
                    bitbucket_gate_path,
                    f"actual {bitbucket_release_check.get('ok')!r}",
                    blocking=not local_fixture,
                )
                require(
                    bitbucket_release_check.get("release_ready") is True,
                    "bitbucket release gate release-check summary is release-ready",
                    bitbucket_gate_path,
                    f"actual {bitbucket_release_check.get('release_ready')!r}",
                    blocking=not local_fixture,
                )
                bitbucket_release_checks = bitbucket_release_check.get("checks")
                require(
                    isinstance(bitbucket_release_checks, list) and bool(bitbucket_release_checks),
                    "bitbucket release gate release-check summary records checks",
                    bitbucket_gate_path,
                    blocking=not local_fixture,
                )
                if isinstance(bitbucket_release_checks, list):
                    bitbucket_release_check_entries = [
                        check
                        for check in bitbucket_release_checks
                        if isinstance(check, dict)
                    ]
                    require(
                        len(bitbucket_release_check_entries) == len(bitbucket_release_checks),
                        "bitbucket release gate release-check summary checks are objects",
                        bitbucket_gate_path,
                        blocking=not local_fixture,
                    )
                    bitbucket_release_check_names = []
                    invalid_bitbucket_release_check_indexes = []
                    for index, check in enumerate(bitbucket_release_check_entries):
                        check_name = check.get("name")
                        if isinstance(check_name, str) and check_name.strip():
                            bitbucket_release_check_names.append(check_name.strip())
                        else:
                            invalid_bitbucket_release_check_indexes.append(index)
                    require(
                        not invalid_bitbucket_release_check_indexes,
                        "bitbucket release gate release-check summary check names are valid",
                        bitbucket_gate_path,
                        f"indexes={invalid_bitbucket_release_check_indexes}",
                        blocking=not local_fixture,
                    )
                    duplicate_bitbucket_release_checks = duplicate_values(
                        bitbucket_release_check_names
                    )
                    require(
                        not duplicate_bitbucket_release_checks,
                        "bitbucket release gate release-check summary checks are unique",
                        bitbucket_gate_path,
                        f"duplicates={duplicate_bitbucket_release_checks}",
                        blocking=not local_fixture,
                    )
                    failing_bitbucket_release_checks = [
                        check.get("name", f"#{index}")
                        for index, check in enumerate(bitbucket_release_check_entries)
                        if check.get("ok") is not True
                    ]
                    require(
                        not failing_bitbucket_release_checks,
                        "bitbucket release gate release-check summary has no failing checks",
                        bitbucket_gate_path,
                        f"checks={failing_bitbucket_release_checks}",
                        blocking=not local_fixture,
                    )
                    required_bitbucket_release_checks = {
                        "pds-baseline",
                        "provider-url-preflight",
                        "release-identity",
                        "ops-certification",
                        "composition-check",
                        "provider-test",
                        "security-certification",
                        "load-test-suite",
                        "provider-performance",
                    }
                    seen_bitbucket_release_checks = set(bitbucket_release_check_names)
                    require(
                        required_bitbucket_release_checks.issubset(seen_bitbucket_release_checks),
                        "bitbucket release gate release-check summary includes release-ready checks",
                        bitbucket_gate_path,
                        (
                            f"missing={sorted(required_bitbucket_release_checks - seen_bitbucket_release_checks)}; "
                            f"actual={sorted(seen_bitbucket_release_checks)}"
                        ),
                        blocking=not local_fixture,
                    )
                    artifact_required_bitbucket_release_checks = {
                        "pds-baseline",
                        "provider-url-preflight",
                        "release-identity",
                        "ops-certification",
                        "composition-check",
                        "provider-test",
                        "security-certification",
                        "load-test-suite",
                        "provider-performance",
                    }
                    missing_bitbucket_release_check_artifacts = [
                        check.get("name", f"#{index}")
                        for index, check in enumerate(bitbucket_release_check_entries)
                        if check.get("name") in artifact_required_bitbucket_release_checks
                        and (
                            not isinstance(check.get("artifact"), str)
                            or not check.get("artifact").strip()
                        )
                    ]
                    require(
                        not missing_bitbucket_release_check_artifacts,
                        "bitbucket release gate release-check summary retains check artifacts",
                        bitbucket_gate_path,
                        f"checks={missing_bitbucket_release_check_artifacts}",
                        blocking=not local_fixture,
                    )
                    missing_retained_bitbucket_release_check_artifacts = []
                    off_tree_bitbucket_release_check_artifacts = []
                    for index, check in enumerate(bitbucket_release_check_entries):
                        check_name_value = check.get("name")
                        check_name = (
                            check_name_value.strip()
                            if isinstance(check_name_value, str)
                            else f"#{index}"
                        )
                        if check_name not in artifact_required_bitbucket_release_checks:
                            continue
                        artifact_value = check.get("artifact")
                        resolved_artifact = resolve_retained_path(artifact_value)
                        if resolved_artifact is None:
                            missing_retained_bitbucket_release_check_artifacts.append(
                                f"{check_name}: {artifact_value!r}"
                            )
                        elif not path_is_within_directory(resolved_artifact, report_dir):
                            off_tree_bitbucket_release_check_artifacts.append(
                                f"{check_name}: {resolved_artifact.as_posix()}"
                            )
                    require(
                        not missing_retained_bitbucket_release_check_artifacts,
                        "bitbucket release gate release-check summary artifact files exist",
                        bitbucket_gate_path,
                        f"checks={missing_retained_bitbucket_release_check_artifacts}",
                        blocking=not local_fixture,
                    )
                    require(
                        not off_tree_bitbucket_release_check_artifacts,
                        "bitbucket release gate release-check summary artifacts are retained in report directory",
                        bitbucket_gate_path,
                        (
                            f"report_dir={report_dir.resolve(strict=False).as_posix()}; "
                            f"checks={off_tree_bitbucket_release_check_artifacts}"
                        ),
                        blocking=not local_fixture,
                    )
                    expected_bitbucket_release_check_artifacts = {
                        "pds-baseline": report_dir / "pds-security-baseline.json",
                        "provider-url-preflight": report_dir / "release-check-provider-url-preflight.json",
                        "release-identity": report_dir / "release-identity.json",
                        "ops-certification": report_dir / "ops-certification.json",
                        "composition-check": report_dir / "composition-check.json",
                        "provider-test": report_dir / "provider-parity.json",
                        "security-certification": report_dir / "security-certification.json",
                        "load-test-suite": report_dir / "load-test.json",
                        "provider-performance": report_dir / "provider-performance.json",
                    }
                    mismatched_bitbucket_release_check_artifacts = []
                    for index, check in enumerate(bitbucket_release_check_entries):
                        check_name_value = check.get("name")
                        check_name = (
                            check_name_value.strip()
                            if isinstance(check_name_value, str)
                            else f"#{index}"
                        )
                        expected_artifact = expected_bitbucket_release_check_artifacts.get(check_name)
                        if expected_artifact is None:
                            continue
                        resolved_artifact = resolve_retained_path(check.get("artifact"))
                        expected_artifact = expected_artifact.resolve(strict=False)
                        if resolved_artifact is None or resolved_artifact.resolve(strict=False) != expected_artifact:
                            mismatched_bitbucket_release_check_artifacts.append(
                                (
                                    f"{check_name}: expected {expected_artifact.as_posix()}, "
                                    f"actual {check.get('artifact')!r}"
                                )
                            )
                    require(
                        not mismatched_bitbucket_release_check_artifacts,
                        "bitbucket release gate release-check summary artifacts match retained evidence",
                        bitbucket_gate_path,
                        f"checks={mismatched_bitbucket_release_check_artifacts}",
                        blocking=not local_fixture,
                    )
            bitbucket_security_gates = bitbucket_gate.get("security_gates")
            require(
                isinstance(bitbucket_security_gates, dict),
                "bitbucket release gate security gate summary is recorded",
                bitbucket_gate_path,
                blocking=not local_fixture,
            )
            if isinstance(bitbucket_security_gates, dict):
                require(
                    bitbucket_security_gates.get("ok") is True,
                    "bitbucket release gate security gate summary is green",
                    bitbucket_gate_path,
                    f"actual {bitbucket_security_gates.get('ok')!r}",
                    blocking=not local_fixture,
                )
                gitleaks_findings = bitbucket_security_gates.get("gitleaks_findings")
                require(
                    type(gitleaks_findings) is int and gitleaks_findings >= 0,
                    "bitbucket release gate security gate gitleaks finding count is valid",
                    bitbucket_gate_path,
                    f"actual {gitleaks_findings!r}",
                    blocking=not local_fixture,
                )
                security_checks = bitbucket_security_gates.get("checks")
                require(
                    isinstance(security_checks, list) and bool(security_checks),
                    "bitbucket release gate security gate checks are recorded",
                    bitbucket_gate_path,
                    blocking=not local_fixture,
                )
                if isinstance(security_checks, list):
                    security_check_entries = [
                        check for check in security_checks if isinstance(check, dict)
                    ]
                    require(
                        len(security_check_entries) == len(security_checks),
                        "bitbucket release gate security gate checks are objects",
                        bitbucket_gate_path,
                        blocking=not local_fixture,
                    )
                    security_check_names = []
                    invalid_security_check_indexes = []
                    for index, check in enumerate(security_check_entries):
                        check_name = check.get("name")
                        if isinstance(check_name, str) and check_name.strip():
                            security_check_names.append(check_name.strip())
                        else:
                            invalid_security_check_indexes.append(index)
                    require(
                        not invalid_security_check_indexes,
                        "bitbucket release gate security gate check names are valid",
                        bitbucket_gate_path,
                        f"indexes={invalid_security_check_indexes}",
                        blocking=not local_fixture,
                    )
                    duplicate_security_checks = duplicate_values(security_check_names)
                    require(
                        not duplicate_security_checks,
                        "bitbucket release gate security gate checks are unique",
                        bitbucket_gate_path,
                        f"duplicates={duplicate_security_checks}",
                        blocking=not local_fixture,
                    )
                    expected_security_checks = {
                        "supply-chain-gate",
                        "phi-log-lint",
                        "secret-scan",
                        "security-assurance-decision",
                        "security-certification",
                        "release-evidence-check",
                        "ops-certification",
                        "pds-security-baseline",
                    }
                    actual_security_checks = set(security_check_names)
                    require(
                        actual_security_checks == expected_security_checks,
                        "bitbucket release gate security gate checks match release gates",
                        bitbucket_gate_path,
                        (
                            f"missing={sorted(expected_security_checks - actual_security_checks)}; "
                            f"extra={sorted(actual_security_checks - expected_security_checks)}"
                        ),
                        blocking=not local_fixture,
                    )
                    failing_security_checks = [
                        check.get("name", f"#{index}")
                        for index, check in enumerate(security_check_entries)
                        if check.get("ok") is not True
                    ]
                    require(
                        not failing_security_checks,
                        "bitbucket release gate security gate checks are green",
                        bitbucket_gate_path,
                        f"checks={failing_security_checks}",
                        blocking=not local_fixture,
                    )
                    release_ready_security_checks = {
                        "security-assurance-decision",
                        "security-certification",
                        "release-evidence-check",
                        "ops-certification",
                        "pds-security-baseline",
                    }
                    not_ready_security_checks = [
                        check.get("name", f"#{index}")
                        for index, check in enumerate(security_check_entries)
                        if check.get("name") in release_ready_security_checks
                        and check.get("release_ready") is not True
                    ]
                    require(
                        not not_ready_security_checks,
                        "bitbucket release gate security gate checks are release-ready",
                        bitbucket_gate_path,
                        f"checks={not_ready_security_checks}",
                        blocking=not local_fixture,
                    )
                    missing_live_security_checks = [
                        check.get("name", f"#{index}")
                        for index, check in enumerate(security_check_entries)
                        if check.get("name") in {"ops-certification", "pds-security-baseline"}
                        and check.get("live_evidence_required") is not True
                    ]
                    require(
                        not missing_live_security_checks,
                        "bitbucket release gate security gate checks require live evidence",
                        bitbucket_gate_path,
                        f"checks={missing_live_security_checks}",
                        blocking=not local_fixture,
                    )
            bitbucket_provider_certification = bitbucket_gate.get("provider_certification")
            require(
                isinstance(bitbucket_provider_certification, dict),
                "bitbucket release gate provider certification summary is recorded",
                bitbucket_gate_path,
                blocking=not local_fixture,
            )
            if isinstance(bitbucket_provider_certification, dict):
                require(
                    bitbucket_provider_certification.get("ok") is True,
                    "bitbucket release gate provider certification summary is green",
                    bitbucket_gate_path,
                    f"actual {bitbucket_provider_certification.get('ok')!r}",
                    blocking=not local_fixture,
                )
                bitbucket_providers = bitbucket_provider_certification.get("providers")
                require(
                    isinstance(bitbucket_providers, list) and bool(bitbucket_providers),
                    "bitbucket release gate provider summaries are recorded",
                    bitbucket_gate_path,
                    blocking=not local_fixture,
                )
                if isinstance(bitbucket_providers, list):
                    provider_entries = [
                        provider
                        for provider in bitbucket_providers
                        if isinstance(provider, dict)
                    ]
                    require(
                        len(provider_entries) == len(bitbucket_providers),
                        "bitbucket release gate provider summaries are objects",
                        bitbucket_gate_path,
                        blocking=not local_fixture,
                    )
                    provider_names = []
                    invalid_provider_indexes = []
                    for index, provider in enumerate(provider_entries):
                        provider_name = provider.get("provider")
                        if isinstance(provider_name, str) and provider_name.strip():
                            provider_names.append(provider_name.strip())
                        else:
                            invalid_provider_indexes.append(index)
                    require(
                        not invalid_provider_indexes,
                        "bitbucket release gate provider summary names are valid",
                        bitbucket_gate_path,
                        f"indexes={invalid_provider_indexes}",
                        blocking=not local_fixture,
                    )
                    duplicate_provider_names = duplicate_values(provider_names)
                    require(
                        not duplicate_provider_names,
                        "bitbucket release gate provider summaries are unique",
                        bitbucket_gate_path,
                        f"duplicates={duplicate_provider_names}",
                        blocking=not local_fixture,
                    )
                    expected_provider_set = set(SECURITY_EXPECTED_PROVIDERS)
                    actual_provider_set = set(provider_names)
                    require(
                        len(provider_entries) == len(SECURITY_EXPECTED_PROVIDERS),
                        "bitbucket release gate provider summary count matches release-certified providers",
                        bitbucket_gate_path,
                        f"expected={len(SECURITY_EXPECTED_PROVIDERS)}; actual={len(provider_entries)}",
                        blocking=not local_fixture,
                    )
                    require(
                        actual_provider_set == expected_provider_set,
                        "bitbucket release gate provider summaries match release-certified providers",
                        bitbucket_gate_path,
                        (
                            f"missing={sorted(expected_provider_set - actual_provider_set)}; "
                            f"extra={sorted(actual_provider_set - expected_provider_set)}"
                        ),
                        blocking=not local_fixture,
                    )
                    failing_provider_summaries = [
                        provider.get("provider", f"#{index}")
                        for index, provider in enumerate(provider_entries)
                        if provider.get("ok") is not True
                    ]
                    require(
                        not failing_provider_summaries,
                        "bitbucket release gate provider summaries are green",
                        bitbucket_gate_path,
                        f"providers={failing_provider_summaries}",
                        blocking=not local_fixture,
                    )
                    incomplete_provider_summaries = []
                    for provider in provider_entries:
                        provider_name = provider.get("provider")
                        live_certified_total = provider.get("live_certified_total")
                        live_certified_passed = provider.get("live_certified_passed")
                        if (
                            not isinstance(provider.get("data_source"), str)
                            or not provider.get("data_source").strip()
                            or not isinstance(provider.get("base_url"), str)
                            or not provider.get("base_url").strip()
                            or type(live_certified_total) is not int
                            or live_certified_total <= 0
                            or type(live_certified_passed) is not int
                            or live_certified_passed != live_certified_total
                        ):
                            incomplete_provider_summaries.append(
                                f"{provider_name}: live_certified_passed={live_certified_passed!r}; "
                                f"live_certified_total={live_certified_total!r}; "
                                f"data_source={provider.get('data_source')!r}; "
                                f"base_url={provider.get('base_url')!r}"
                            )
                    require(
                        not incomplete_provider_summaries,
                        "bitbucket release gate provider live certification summaries are complete",
                        bitbucket_gate_path,
                        "; ".join(incomplete_provider_summaries),
                        blocking=not local_fixture,
                    )
            bitbucket_ready_summaries = {
                "ops_certification": bitbucket_gate.get("ops_certification"),
                "pds_security_baseline": bitbucket_gate.get("pds_security_baseline"),
                "security_assurance_decision": bitbucket_gate.get("security_assurance_decision"),
            }
            for summary_name, summary in bitbucket_ready_summaries.items():
                require(
                    isinstance(summary, dict),
                    f"bitbucket release gate {summary_name} summary is recorded",
                    bitbucket_gate_path,
                    blocking=not local_fixture,
                )
                if isinstance(summary, dict):
                    require(
                        summary.get("ok") is True,
                        f"bitbucket release gate {summary_name} summary is green",
                        bitbucket_gate_path,
                        f"actual {summary.get('ok')!r}",
                        blocking=not local_fixture,
                    )
                    require(
                        summary.get("release_ready") is True,
                        f"bitbucket release gate {summary_name} summary is release-ready",
                        bitbucket_gate_path,
                        f"actual {summary.get('release_ready')!r}",
                        blocking=not local_fixture,
                    )
            for summary_name in ("ops_certification", "pds_security_baseline"):
                summary = bitbucket_ready_summaries.get(summary_name)
                if isinstance(summary, dict):
                    require(
                        summary.get("live_evidence_required") is True,
                        f"bitbucket release gate {summary_name} summary requires live evidence",
                        bitbucket_gate_path,
                        f"actual {summary.get('live_evidence_required')!r}",
                        blocking=not local_fixture,
                    )
            bitbucket_mcp_release_posture = bitbucket_gate.get("mcp_release_posture")
            require(
                isinstance(bitbucket_mcp_release_posture, dict),
                "bitbucket release gate MCP release posture summary is recorded",
                bitbucket_gate_path,
                blocking=not local_fixture,
            )
            if isinstance(bitbucket_mcp_release_posture, dict):
                require(
                    bitbucket_mcp_release_posture.get("ok") is True,
                    "bitbucket release gate MCP release posture summary is green",
                    bitbucket_gate_path,
                    f"actual {bitbucket_mcp_release_posture.get('ok')!r}",
                    blocking=not local_fixture,
                )
        else:
            require(
                bool(normalized_bitbucket_blockers),
                "bitbucket release gate non-ready evidence retains blockers",
                bitbucket_gate_path,
                blocking=not local_fixture,
            )
else:
    record_artifact(
        "bitbucket-release-gate",
        bitbucket_gate_file,
        required=False,
        required_in_ci=False,
        status="not-applicable",
        applicability="not-applicable",
        applicability_reason=(
            "bitbucket-release-gate writes this artifact after release-evidence-check; "
            "the final checker validates it when a retained wrapper artifact is present"
        ),
    )


phi_path = (report_dir / "phi-log-lint.json").as_posix()
phi = load_required("phi-log-lint", "phi-log-lint.json", dict)
if phi is not None:
    require(phi.get("command") == "phi-log-lint", "phi command is stable", phi_path)
    require(
        phi.get("ok") is True,
        "phi ok flag is true",
        phi_path,
        blocking=not local_fixture,
    )
    require(
        isinstance(phi.get("generated_at_utc"), str),
        "phi generated timestamp is present",
        phi_path,
    )
    validate_generated_timestamp(
        "phi",
        phi,
        phi_path,
        STATIC_EVIDENCE_MAX_AGE,
        STATIC_EVIDENCE_MAX_AGE_HOURS,
        blocking=not local_fixture,
    )
    require(is_int(phi.get("scanned_files")), "phi scanned_files is an integer", phi_path)
    require(is_int(phi.get("findings")), "phi findings is an integer", phi_path)
    require(
        phi.get("failure_reason") is None or isinstance(phi.get("failure_reason"), str),
        "phi failure_reason is null or string",
        phi_path,
    )
    require(isinstance(phi.get("allow_marker"), str), "phi allow_marker is present", phi_path)
    phi_redaction = phi.get("redaction")
    require(isinstance(phi_redaction, dict), "phi redaction block is present", phi_path)
    if isinstance(phi_redaction, dict):
        require(
            phi_redaction.get("json_source_excerpt_retained") is False,
            "phi source excerpts are not retained",
            phi_path,
        )
        require(
            phi_redaction.get("json_finding_values_retained") is False,
            "phi finding values are not retained",
            phi_path,
        )

supply_path = (report_dir / "supply-chain-gate.json").as_posix()
supply = load_required("supply-chain-gate", "supply-chain-gate.json", dict)
frontend_sbom_is_required = bool(frontend_lockfiles())
if supply is not None:
    require(
        supply.get("command") == "supply-chain-gate",
        "supply-chain command is stable",
        supply_path,
    )
    require(
        supply.get("ok") is True,
        "supply-chain ok flag is true",
        supply_path,
        blocking=not local_fixture,
    )
    require(
        isinstance(supply.get("generated_at_utc"), str),
        "supply-chain generated timestamp is present",
        supply_path,
    )
    validate_generated_timestamp(
        "supply-chain",
        supply,
        supply_path,
        STATIC_EVIDENCE_MAX_AGE,
        STATIC_EVIDENCE_MAX_AGE_HOURS,
        blocking=not local_fixture,
    )
    validate_artifact_names_unique("supply-chain", supply, supply_path, blocking=not local_fixture)
    supply_checks = supply.get("checks")
    require(isinstance(supply_checks, list), "supply-chain checks is an array", supply_path)
    if isinstance(supply_checks, list):
        require(bool(supply_checks), "supply-chain has at least one check", supply_path)
        for index, check in enumerate(supply_checks):
            check_path = f"{supply_path}#/checks/{index}"
            require(isinstance(check, dict), "supply-chain check is object", check_path)
            if isinstance(check, dict):
                require(isinstance(check.get("name"), str), "supply-chain check name", check_path)
                require(is_int(check.get("exit_code")), "supply-chain check exit_code", check_path)
                require(isinstance(check.get("ok"), bool), "supply-chain check ok", check_path)
    require(
        has_artifact(supply, "dependency-check"),
        "supply-chain records required dependency-check artifact",
        supply_path,
        blocking=not local_fixture,
    )
    validate_named_artifact_reference(
        "supply-chain",
        supply,
        "dependency-check",
        report_dir / "dependency-check.json",
        supply_path,
        blocking=not local_fixture,
        require_sha=True,
        require_bytes=True,
    )
    require(
        has_artifact(supply, "phi-log-lint"),
        "supply-chain records required phi artifact",
        supply_path,
        blocking=not local_fixture,
    )
    validate_named_artifact_reference(
        "supply-chain",
        supply,
        "phi-log-lint",
        report_dir / "phi-log-lint.json",
        supply_path,
        blocking=not local_fixture,
        require_sha=True,
        require_bytes=True,
    )
    require(
        has_artifact(supply, "sbom-manifest"),
        "supply-chain records required SBOM manifest",
        supply_path,
        blocking=not local_fixture,
    )
    validate_named_artifact_reference(
        "supply-chain",
        supply,
        "sbom-manifest",
        report_dir / "sbom-manifest.json",
        supply_path,
        blocking=not local_fixture,
        require_sha=True,
        require_bytes=True,
    )
    require(
        has_artifact(supply, "rust-cyclonedx-sbom"),
        "supply-chain records required Rust CycloneDX SBOM",
        supply_path,
        blocking=not local_fixture,
    )
    validate_named_artifact_reference(
        "supply-chain",
        supply,
        "rust-cyclonedx-sbom",
        report_dir / "sbom-rust-workspace.cdx.json",
        supply_path,
        blocking=not local_fixture,
        require_sha=True,
        require_bytes=True,
    )
    require(
        not frontend_sbom_is_required or has_artifact(supply, "frontend-cyclonedx-sbom"),
        "supply-chain records required frontend CycloneDX SBOM",
        supply_path,
        blocking=not local_fixture,
    )
    if frontend_sbom_is_required:
        validate_named_artifact_reference(
            "supply-chain",
            supply,
            "frontend-cyclonedx-sbom",
            report_dir / "sbom-frontend-packages.cdx.json",
            supply_path,
            blocking=not local_fixture,
            require_sha=True,
            require_bytes=True,
        )

dependency_path = (report_dir / "dependency-check.json").as_posix()
dependency = load_required("dependency-check", "dependency-check.json", dict)
if dependency is not None:
    require(
        dependency.get("command") == "dependency-check",
        "dependency-check command is stable",
        dependency_path,
    )
    require(
        dependency.get("ok") is True,
        "dependency-check ok flag is true",
        dependency_path,
        blocking=not local_fixture,
    )
    require(
        isinstance(dependency.get("generated_at_utc"), str),
        "dependency-check generated timestamp is present",
        dependency_path,
    )
    validate_generated_timestamp(
        "dependency-check",
        dependency,
        dependency_path,
        STATIC_EVIDENCE_MAX_AGE,
        STATIC_EVIDENCE_MAX_AGE_HOURS,
        blocking=not local_fixture,
    )
    require(
        isinstance(dependency.get("summary"), dict),
        "dependency-check summary is object",
        dependency_path,
    )
    summary = dependency.get("summary") if isinstance(dependency.get("summary"), dict) else {}
    require(
        is_int(summary.get("accepted_osv_finding_count")),
        "dependency-check records accepted OSV summary count",
        dependency_path,
    )
    require(
        isinstance(dependency.get("policy"), dict),
        "dependency-check records policy object",
        dependency_path,
    )
    osv = dependency.get("osv")
    require(
        isinstance(osv, dict),
        "dependency-check records OSV object",
        dependency_path,
    )
    if isinstance(osv, dict):
        require(
            isinstance(osv.get("accepted_findings"), list),
            "dependency-check records accepted OSV findings",
            dependency_path,
        )
        require(
            isinstance(osv.get("expired_acceptances"), list),
            "dependency-check records expired OSV acceptances",
            dependency_path,
        )
    require(
        isinstance(dependency.get("checks"), list) and bool(dependency.get("checks")),
        "dependency-check records executed checks",
        dependency_path,
    )
    dependency_checks = dependency.get("checks")
    if isinstance(dependency_checks, list) and dependency_checks:
        check_names = []
        required_check_seen = False
        for index, check in enumerate(dependency_checks):
            check_path = f"{dependency_path}#/checks/{index}"
            require(isinstance(check, dict), "dependency-check check is object", check_path)
            if not isinstance(check, dict):
                continue
            check_name = check.get("name")
            check_name_ok = isinstance(check_name, str) and bool(check_name.strip())
            require(check_name_ok, "dependency-check check records name", check_path)
            if check_name_ok:
                check_names.append(check_name.strip())
            require(
                isinstance(check.get("required"), bool),
                "dependency-check check required flag is boolean",
                check_path,
                f"actual {check.get('required')!r}",
            )
            if check.get("required") is True:
                required_check_seen = True
                require(
                    check.get("ok") is True,
                    "dependency-check required check is green",
                    check_path,
                    f"{check_name or index}: ok={check.get('ok')!r}",
                    blocking=not local_fixture,
                )
                require(
                    check.get("exit_code") == 0,
                    "dependency-check required check exit code is zero",
                    check_path,
                    f"{check_name or index}: exit_code={check.get('exit_code')!r}",
                    blocking=not local_fixture,
                )
                require(
                    check.get("status") == "passed",
                    "dependency-check required check status is passed",
                    check_path,
                    f"{check_name or index}: status={check.get('status')!r}",
                    blocking=not local_fixture,
                )
        duplicate_check_names = duplicate_values(check_names)
        require(
            not duplicate_check_names,
            "dependency-check check names are unique",
            dependency_path,
            f"duplicates={duplicate_check_names}",
        )
        require(
            required_check_seen,
            "dependency-check includes at least one required check",
            dependency_path,
            blocking=not local_fixture,
        )
    require(
        isinstance(dependency.get("blocking_findings"), list),
        "dependency-check records blocking findings",
        dependency_path,
    )
    require(
        dependency.get("blocking_findings") == [],
        "dependency-check blocking findings are empty",
        dependency_path,
        f"actual {dependency.get('blocking_findings')!r}",
        blocking=not local_fixture,
    )

sbom_manifest_path = (report_dir / "sbom-manifest.json").as_posix()
sbom_manifest = load_required("sbom-manifest", "sbom-manifest.json", dict)
image_context = image_sbom_context(supply, sbom_manifest)
image_sbom_is_required = image_context["required"]
if supply is not None:
    require(
        not image_sbom_is_required or has_artifact(supply, "deployable-image-cyclonedx-sbom"),
        "supply-chain records deployable image CycloneDX SBOM when applicable",
        supply_path,
        image_context["detail"],
        blocking=not local_fixture,
    )
    if image_sbom_is_required:
        validate_named_artifact_reference(
            "supply-chain",
            supply,
            "deployable-image-cyclonedx-sbom",
            report_dir / "sbom-deployable-image.cdx.json",
            supply_path,
            blocking=not local_fixture,
            require_sha=True,
            require_bytes=True,
        )

if sbom_manifest is not None:
    require(
        sbom_manifest.get("command") == "generate-sbom-evidence",
        "SBOM manifest command is stable",
        sbom_manifest_path,
    )
    require(
        sbom_manifest.get("ok") is True,
        "SBOM manifest ok flag is true",
        sbom_manifest_path,
        blocking=not local_fixture,
    )
    require(
        isinstance(sbom_manifest.get("generated_at_utc"), str),
        "SBOM manifest generated timestamp is present",
        sbom_manifest_path,
    )
    validate_generated_timestamp(
        "SBOM manifest",
        sbom_manifest,
        sbom_manifest_path,
        STATIC_EVIDENCE_MAX_AGE,
        STATIC_EVIDENCE_MAX_AGE_HOURS,
        blocking=not local_fixture,
    )
    manifest_artifacts = sbom_manifest.get("artifacts")
    require(
        isinstance(manifest_artifacts, list),
        "SBOM manifest artifacts is an array",
        sbom_manifest_path,
    )
    expected_frontend_sources = sorted(frontend_lockfiles())
    expected_image_sources = sorted(
        dict.fromkeys(image_context["env_sources"] + image_context["retained_artifacts"])
    )
    manifest_inputs = sbom_manifest.get("inputs")
    require(
        isinstance(manifest_inputs, dict),
        "SBOM manifest inputs are recorded",
        sbom_manifest_path,
        f"actual {manifest_inputs!r}",
        blocking=not local_fixture,
    )
    if isinstance(manifest_inputs, dict):
        actual_frontend_inputs = sorted_unique_strings(manifest_inputs.get("frontend_lockfiles"))
        require(
            actual_frontend_inputs == expected_frontend_sources,
            "SBOM manifest frontend inputs match tracked package locks",
            sbom_manifest_path,
            f"expected={expected_frontend_sources}; actual={actual_frontend_inputs}",
            blocking=not local_fixture,
        )
        actual_image_inputs = sorted_unique_strings(manifest_inputs.get("image_sources"))
        require(
            actual_image_inputs == expected_image_sources,
            "SBOM manifest image inputs match deployable image sources",
            sbom_manifest_path,
            f"expected={expected_image_sources}; actual={actual_image_inputs}",
            blocking=not local_fixture,
        )
        require(
            manifest_inputs.get("image_required_by_release_mode")
            == image_context["required_by_release_mode"],
            "SBOM manifest image release-policy input matches release mode",
            sbom_manifest_path,
            (
                f"expected={image_context['required_by_release_mode']!r}; "
                f"actual={manifest_inputs.get('image_required_by_release_mode')!r}"
            ),
            blocking=not local_fixture,
        )
    validate_artifact_names_unique("SBOM manifest", sbom_manifest, sbom_manifest_path, blocking=not local_fixture)
    require(
        artifact_required(sbom_manifest, "rust-cyclonedx-sbom"),
        "SBOM manifest requires Rust CycloneDX SBOM",
        sbom_manifest_path,
        blocking=not local_fixture,
    )
    validate_sbom_manifest_artifact_sources(
        "rust-cyclonedx-sbom",
        sbom_manifest,
        ["Cargo.lock", "cargo metadata --locked --no-deps"],
        blocking=not local_fixture,
    )
    validate_named_artifact_reference(
        "SBOM manifest",
        sbom_manifest,
        "rust-cyclonedx-sbom",
        report_dir / "sbom-rust-workspace.cdx.json",
          sbom_manifest_path,
          blocking=not local_fixture,
          require_sha=True,
          require_bytes=True,
      )
    require(
        not frontend_sbom_is_required
        or artifact_required(sbom_manifest, "frontend-cyclonedx-sbom"),
        "SBOM manifest requires frontend CycloneDX SBOM when package locks exist",
        sbom_manifest_path,
        blocking=not local_fixture,
    )
    if frontend_sbom_is_required:
        validate_sbom_manifest_artifact_sources(
            "frontend-cyclonedx-sbom",
            sbom_manifest,
            expected_frontend_sources,
            blocking=not local_fixture,
        )
        validate_named_artifact_reference(
            "SBOM manifest",
            sbom_manifest,
            "frontend-cyclonedx-sbom",
            report_dir / "sbom-frontend-packages.cdx.json",
              sbom_manifest_path,
              blocking=not local_fixture,
              require_sha=True,
              require_bytes=True,
          )
    require(
        not image_sbom_is_required
        or artifact_required(sbom_manifest, "deployable-image-cyclonedx-sbom"),
        "SBOM manifest requires deployable image CycloneDX SBOM when applicable",
        sbom_manifest_path,
        image_context["detail"],
        blocking=not local_fixture,
    )
    if image_sbom_is_required:
        validate_sbom_manifest_artifact_sources(
            "deployable-image-cyclonedx-sbom",
            sbom_manifest,
            expected_image_sources,
            blocking=not local_fixture,
        )
        validate_named_artifact_reference(
            "SBOM manifest",
            sbom_manifest,
            "deployable-image-cyclonedx-sbom",
            report_dir / "sbom-deployable-image.cdx.json",
              sbom_manifest_path,
              blocking=not local_fixture,
              require_sha=True,
              require_bytes=True,
          )

rust_sbom_path = (report_dir / "sbom-rust-workspace.cdx.json").as_posix()
rust_sbom = load_required("rust-cyclonedx-sbom", "sbom-rust-workspace.cdx.json", dict)
if rust_sbom is not None:
    validate_cyclonedx("Rust SBOM", rust_sbom, rust_sbom_path)
    validate_sbom_manifest_matches_bom(
        "Rust SBOM",
        sbom_manifest,
        "rust-cyclonedx-sbom",
        rust_sbom,
        rust_sbom_path,
        blocking=not local_fixture,
    )

if frontend_sbom_is_required:
    frontend_sbom_path = (report_dir / "sbom-frontend-packages.cdx.json").as_posix()
    frontend_sbom = load_required(
        "frontend-cyclonedx-sbom",
        "sbom-frontend-packages.cdx.json",
        dict,
    )
    if frontend_sbom is not None:
        validate_cyclonedx("frontend SBOM", frontend_sbom, frontend_sbom_path)
        validate_sbom_manifest_matches_bom(
            "frontend SBOM",
            sbom_manifest,
            "frontend-cyclonedx-sbom",
            frontend_sbom,
            frontend_sbom_path,
            blocking=not local_fixture,
        )

if image_sbom_is_required:
    image_sbom_path = (report_dir / "sbom-deployable-image.cdx.json").as_posix()
    image_sbom = load_required(
        "deployable-image-cyclonedx-sbom",
        "sbom-deployable-image.cdx.json",
        dict,
        missing_detail=image_context["detail"],
        applicability=image_context["applicability"],
        applicability_reason=image_context["detail"],
    )
    if image_sbom is not None:
        validate_cyclonedx("deployable image SBOM", image_sbom, image_sbom_path)
        validate_sbom_manifest_matches_bom(
            "deployable image SBOM",
            sbom_manifest,
            "deployable-image-cyclonedx-sbom",
            image_sbom,
            image_sbom_path,
            blocking=not local_fixture,
        )
else:
    image_sbom_file = report_dir / "sbom-deployable-image.cdx.json"
    optional_status = "present" if image_sbom_file.is_file() else "not-applicable"
    optional_applicability = "optional" if image_sbom_file.is_file() else "not-applicable"
    record_artifact(
        "deployable-image-cyclonedx-sbom",
        image_sbom_file,
        required=False,
        required_in_ci=False,
        status=optional_status,
        applicability=optional_applicability,
        applicability_reason=image_context["detail"],
    )
    record(
        "deployable image SBOM is not applicable without a deployable image artifact",
        True,
        image_sbom_file.as_posix(),
        image_context["detail"],
    )
    if image_sbom_file.is_file():
        try:
            image_sbom = json.loads(image_sbom_file.read_text(encoding="utf-8"))
        except Exception as exc:
            record(
                "optional deployable image SBOM parses as JSON",
                False,
                image_sbom_file.as_posix(),
                str(exc),
            )
        else:
            validate_cyclonedx(
                "optional deployable image SBOM",
                image_sbom,
                image_sbom_file.as_posix(),
            )
            validate_sbom_manifest_matches_bom(
                "optional deployable image SBOM",
                sbom_manifest,
                "deployable-image-cyclonedx-sbom",
                image_sbom,
                image_sbom_file.as_posix(),
                blocking=not local_fixture,
            )

secret_path = (report_dir / "secret-scan.json").as_posix()
secret = load_required("secret-scan", "secret-scan.json", dict)
if secret is not None:
    require(secret.get("command") == "secret-scan", "secret command is stable", secret_path)
    require(
        secret.get("ok") is True,
        "secret ok flag is true",
        secret_path,
        blocking=not local_fixture,
    )
    require(
        isinstance(secret.get("generated_at_utc"), str),
        "secret generated timestamp is present",
        secret_path,
    )
    validate_generated_timestamp(
        "secret",
        secret,
        secret_path,
        STATIC_EVIDENCE_MAX_AGE,
        STATIC_EVIDENCE_MAX_AGE_HOURS,
        blocking=not local_fixture,
    )
    validate_artifact_names_unique("secret", secret, secret_path, blocking=not local_fixture)
    require(secret.get("scanner") == "gitleaks", "secret scanner is gitleaks", secret_path)
    require(
        isinstance(secret.get("scanner_version"), str),
        "secret scanner_version is present",
        secret_path,
    )
    require(
        isinstance(secret.get("requested_scanner_version"), str),
        "secret requested scanner version is present",
        secret_path,
    )
    require(is_int(secret.get("gitleaks_exit_code")), "secret gitleaks_exit_code", secret_path)
    require(is_int(secret.get("findings")), "secret findings is an integer", secret_path)
    require(is_int(secret.get("total_findings")), "secret total_findings is an integer", secret_path)
    require(
        is_int(secret.get("baselined_findings")),
        "secret baselined_findings is an integer",
        secret_path,
    )
    require(
        secret.get("history_scan") is True,
        "secret history_scan is true",
        secret_path,
        blocking=not local_fixture,
    )
    require(
        has_artifact(secret, "gitleaks-report"),
        "secret records gitleaks artifact",
        secret_path,
        blocking=not local_fixture,
    )
    validate_named_artifact_reference(
        "secret",
        secret,
        "gitleaks-report",
        report_dir / "gitleaks-report.json",
        secret_path,
        blocking=not local_fixture,
    )
    secret_redaction = secret.get("redaction")
    require(isinstance(secret_redaction, dict), "secret redaction block is present", secret_path)
    if isinstance(secret_redaction, dict):
        require(
            secret_redaction.get("gitleaks_redact_flag") is True,
            "secret scan used gitleaks redact flag",
            secret_path,
        )
        require(
            secret_redaction.get("secret_fields_checked") is True,
            "secret fields were checked",
            secret_path,
        )
        require(
            secret_redaction.get("secret_fields_redacted") is True,
            "secret fields are redacted",
            secret_path,
        )

gitleaks_path = (report_dir / "gitleaks-report.json").as_posix()
gitleaks = load_required("gitleaks-report", "gitleaks-report.json", list)
unredacted_secret_count = 0
if gitleaks is not None:
    for finding in gitleaks:
        if not isinstance(finding, dict) or "Secret" not in finding:
            continue
        if not is_redacted_secret(finding.get("Secret")):
            unredacted_secret_count += 1
    require(
        unredacted_secret_count == 0,
        "gitleaks Secret fields are redacted",
        gitleaks_path,
        f"{unredacted_secret_count} unredacted Secret field(s)",
    )
if secret is not None and gitleaks is not None:
    validate_secret_scan_reconciliation(
        secret,
        gitleaks,
        secret_path,
        gitleaks_path,
        blocking=not local_fixture,
    )

security_assurance_decision_required = release_mode_requires_security_assurance_decision()
security_assurance_decision_file = report_dir / "security-assurance-decision.json"
security_assurance_decision_path = security_assurance_decision_file.as_posix()
if security_assurance_decision_required or security_assurance_decision_file.is_file():
    security_assurance_decision = load_required(
        "security-assurance-decision",
        "security-assurance-decision.json",
        dict,
        missing_detail=(
            "security assurance release decision is required by "
            "APPFW_REQUIRE_SECURITY_ASSURANCE_DECISION, "
            "APPFW_RELEASE_REQUIRE_SECURITY_ASSURANCE_DECISION, "
            "APPFW_REQUIRE_PRODUCTION_ATTESTATIONS, or "
            "APPFW_RELEASE_REQUIRE_PRODUCTION_ATTESTATIONS"
        ),
        applicability="required" if security_assurance_decision_required else "optional",
    )
    if security_assurance_decision is not None:
        require(
            security_assurance_decision.get("command") == "security-assurance-decision",
            "security assurance decision command is stable",
            security_assurance_decision_path,
        )
        require(
            security_assurance_decision.get("ok") is True,
            "security assurance decision ok flag is true",
            security_assurance_decision_path,
            blocking=not local_fixture,
        )
        require(
            security_assurance_decision.get("release_ready") is True,
            "security assurance decision is release-ready",
            security_assurance_decision_path,
            blocking=not local_fixture,
        )
        require(
            isinstance(security_assurance_decision.get("generated_at_utc"), str),
            "security assurance decision generated timestamp is present",
            security_assurance_decision_path,
        )
        security_assurance_generated_at = parse_timestamp_with_timezone(
            security_assurance_decision.get("generated_at_utc")
        )
        require(
            security_assurance_generated_at is not None,
            "security assurance decision generated timestamp is valid",
            security_assurance_decision_path,
            "generated_at_utc must be ISO-8601 with timezone",
            blocking=not local_fixture,
        )
        if security_assurance_generated_at is not None:
            now = dt.datetime.now(dt.timezone.utc)
            require(
                security_assurance_generated_at <= now + EVIDENCE_TIMESTAMP_SKEW,
                "security assurance decision generated timestamp is not in the future",
                security_assurance_decision_path,
                security_assurance_generated_at.isoformat().replace("+00:00", "Z"),
                blocking=not local_fixture,
            )
            require(
                security_assurance_generated_at >= now - SECURITY_ASSURANCE_DECISION_MAX_AGE,
                "security assurance decision generated timestamp is recent",
                security_assurance_decision_path,
                (
                    f"generated_at_utc {security_assurance_generated_at.isoformat().replace('+00:00', 'Z')} "
                    f"is older than {SECURITY_ASSURANCE_DECISION_MAX_AGE_HOURS} hours"
                ),
                blocking=not local_fixture,
            )
        if release_mode_requires_production_attestations():
            require(
                security_assurance_decision.get("production_attestations_required") is True,
                "security assurance decision was generated with production attestation requirements",
                security_assurance_decision_path,
                (
                    "rerun scripts/ci/security-assurance-decision.sh with "
                    "APPFW_RELEASE_REQUIRE_PRODUCTION_ATTESTATIONS=true"
                ),
                blocking=not local_fixture,
            )
        categories = security_assurance_decision.get("categories")
        require(
            isinstance(categories, list) and bool(categories),
            "security assurance decision records categories",
            security_assurance_decision_path,
        )
        requirements = security_assurance_decision.get("requirements")
        required_security_assurance_categories = {
            "dast",
            "sast",
            "asvs",
            "release-provenance",
            "artifact-signing",
        }
        if security_assurance_decision_required:
            requirement_names = {
                str(requirement).strip().lower().replace("_", "-")
                for requirement in requirements
                if isinstance(requirement, str)
            } if isinstance(requirements, list) else set()
            category_names = {
                str(category.get("category")).strip().lower().replace("_", "-")
                for category in categories
                if isinstance(category, dict)
            } if isinstance(categories, list) else set()
            require(
                required_security_assurance_categories.issubset(requirement_names),
                "security assurance requirements include regulated release defaults",
                security_assurance_decision_path,
                f"missing {sorted(required_security_assurance_categories - requirement_names)}",
                blocking=not local_fixture,
            )
            require(
                required_security_assurance_categories.issubset(category_names),
                "security assurance categories include regulated release defaults",
                security_assurance_decision_path,
                f"missing {sorted(required_security_assurance_categories - category_names)}",
                blocking=not local_fixture,
            )
        if isinstance(categories, list):
            security_risk_summary = security_assurance_decision.get("risk_acceptance")
            security_risk_max_days = 90
            if isinstance(security_risk_summary, dict) and is_int(security_risk_summary.get("max_days")):
                security_risk_max_days = max(1, security_risk_summary.get("max_days"))
            for index, category in enumerate(categories):
                category_path = f"{security_assurance_decision_path}#/categories/{index}"
                require(
                    isinstance(category, dict),
                    "security assurance category is object",
                    category_path,
                )
                if isinstance(category, dict):
                    require(
                        isinstance(category.get("category"), str),
                        "security assurance category name is present",
                        category_path,
                    )
                    require(
                        category.get("ok") is True,
                        "security assurance category is ok",
                        category_path,
                        blocking=not local_fixture,
                    )
                    require(
                        category.get("disposition") in ("evidence", "risk-accepted", "local-fixture"),
                        "security assurance category disposition is valid",
                        category_path,
                    )
                    category_name = category.get("category")
                    if not isinstance(category_name, str) or not category_name.strip():
                        category_name = f"category-{index}"
                    disposition = category.get("disposition")
                    if disposition == "evidence":
                        validate_security_assurance_evidence_artifacts(
                            category_name,
                            category.get("evidence_artifacts"),
                            category_path,
                            blocking=not local_fixture,
                        )
                    elif disposition == "risk-accepted":
                        validate_security_assurance_risk_acceptance(
                            category_name,
                            category.get("risk_acceptance"),
                            category_path,
                            security_risk_max_days,
                            blocking=not local_fixture,
                        )
                    if not local_fixture:
                        require(
                            category.get("disposition") != "local-fixture",
                            "strict security assurance category is not a local fixture",
                            category_path,
                            blocking=True,
                        )
else:
    record_artifact(
        "security-assurance-decision",
        security_assurance_decision_file,
        required=False,
        required_in_ci=False,
        status="not-applicable",
        applicability="not-applicable",
        applicability_reason=(
            "not required unless APPFW_REQUIRE_SECURITY_ASSURANCE_DECISION or "
            "APPFW_RELEASE_REQUIRE_SECURITY_ASSURANCE_DECISION is enabled, or "
            "production attestations are required, or "
            "the artifact is present"
        ),
    )
    record(
        "security assurance decision is not required for this evidence bundle",
        True,
        security_assurance_decision_path,
        (
            "set APPFW_RELEASE_REQUIRE_SECURITY_ASSURANCE_DECISION=true in "
            "regulated release gates"
        ),
    )

live_ops_evidence_required = release_mode_requires_live_ops_evidence()
promtool_required = release_mode_requires_promtool()
ops_certification_required = release_mode_requires_ops_certification()
ops_certification_file = report_dir / "ops-certification.json"
ops_certification_path = ops_certification_file.as_posix()
if ops_certification_required or ops_certification_file.is_file():
    ops_certification = load_required(
        "ops-certification",
        "ops-certification.json",
        dict,
        missing_detail=(
            "operations certification evidence is required by "
            "APPFW_REQUIRE_OPS_CERTIFICATION, APPFW_RELEASE_REQUIRE_OPS_CERTIFICATION, "
            "APPFW_REQUIRE_LIVE_OPS_EVIDENCE, or APPFW_RELEASE_REQUIRE_LIVE_OPS_EVIDENCE"
        ),
        applicability="required" if ops_certification_required else "optional",
    )
    if ops_certification is not None:
        require(
            ops_certification.get("command") == "ops-certification",
            "ops certification command is stable",
            ops_certification_path,
        )
        require(
            ops_certification.get("ok") is True,
            "ops certification ok flag is true",
            ops_certification_path,
            blocking=not local_fixture,
        )
        require(
            isinstance(ops_certification.get("generated_at_utc"), str),
            "ops certification generated timestamp is present",
            ops_certification_path,
        )
        ops_generated_at = parse_timestamp_with_timezone(ops_certification.get("generated_at_utc"))
        require(
            ops_generated_at is not None,
            "ops certification generated timestamp is valid",
            ops_certification_path,
            "generated_at_utc must be ISO-8601 with timezone",
            blocking=not local_fixture,
        )
        if ops_generated_at is not None:
            now = dt.datetime.now(dt.timezone.utc)
            require(
                ops_generated_at <= now + EVIDENCE_TIMESTAMP_SKEW,
                "ops certification generated timestamp is not in the future",
                ops_certification_path,
                ops_generated_at.isoformat().replace("+00:00", "Z"),
                blocking=not local_fixture,
            )
            require(
                ops_generated_at >= now - OPS_CERTIFICATION_MAX_AGE,
                "ops certification generated timestamp is recent",
                ops_certification_path,
                (
                    f"generated_at_utc {ops_generated_at.isoformat().replace('+00:00', 'Z')} "
                    f"is older than {OPS_CERTIFICATION_MAX_AGE_HOURS} hours"
                ),
                blocking=not local_fixture,
            )
        require(
            isinstance(ops_certification.get("checks"), list)
            and bool(ops_certification.get("checks")),
            "ops certification records checks",
            ops_certification_path,
        )
        ops_checks = ops_certification.get("checks")
        ops_claims_ready = ops_certification.get("ok") is True or ops_certification.get("release_ready") is True
        if isinstance(ops_checks, list):
            failing_ops_checks = []
            for index, check in enumerate(ops_checks):
                check_path = f"{ops_certification_path}#/checks/{index}"
                require(
                    isinstance(check, dict),
                    "ops certification check entry is object",
                    check_path,
                )
                if not isinstance(check, dict):
                    continue
                check_name = check.get("name")
                require(
                    isinstance(check_name, str) and bool(check_name.strip()),
                    "ops certification check entry has name",
                    check_path,
                )
                check_ok = check.get("ok")
                require(
                    isinstance(check_ok, bool),
                    "ops certification check entry ok flag is boolean",
                    check_path,
                    f"actual {check_ok!r}",
                )
                if check_ok is False:
                    failing_ops_checks.append(check_name.strip() if isinstance(check_name, str) else f"check-{index}")
            require(
                not ops_claims_ready or not failing_ops_checks,
                "ops certification ready claim has no failing checks",
                ops_certification_path,
                f"failing checks={failing_ops_checks[:20]}",
                blocking=not local_fixture,
            )
        require(
            isinstance(ops_certification.get("artifacts"), list)
            and bool(ops_certification.get("artifacts")),
            "ops certification records artifacts",
            ops_certification_path,
        )
        ops_artifacts = ops_certification.get("artifacts")
        if isinstance(ops_artifacts, list):
            missing_required_ops_artifacts = []
            retained_live_ops_artifacts = []
            for index, artifact in enumerate(ops_artifacts):
                artifact_path = f"{ops_certification_path}#/artifacts/{index}"
                require(
                    isinstance(artifact, dict),
                    "ops certification artifact entry is object",
                    artifact_path,
                )
                if not isinstance(artifact, dict):
                    continue
                artifact_name = artifact.get("name")
                require(
                    isinstance(artifact_name, str) and bool(artifact_name.strip()),
                    "ops certification artifact entry has name",
                    artifact_path,
                )
                required = artifact.get("required")
                present = artifact.get("present")
                require(
                    isinstance(required, bool),
                    "ops certification artifact required flag is boolean",
                    artifact_path,
                    f"actual {required!r}",
                )
                require(
                    isinstance(present, bool),
                    "ops certification artifact present flag is boolean",
                    artifact_path,
                    f"actual {present!r}",
                )
                if required is True and present is not True:
                    missing_required_ops_artifacts.append(
                        artifact_name.strip() if isinstance(artifact_name, str) else f"artifact-{index}"
                    )
                resolved_artifact = None
                if required is True or present is True:
                    resolved_artifact = validate_retained_file_artifact(
                        "ops certification artifact",
                        artifact,
                        artifact_path,
                        blocking=not local_fixture,
                        require_ok_flag=False,
                    )
                if (
                    isinstance(artifact_name, str)
                    and artifact_name.strip() == "live-ops-evidence"
                    and resolved_artifact is not None
                    and path_is_within_directory(resolved_artifact, report_dir)
                ):
                    retained_live_ops_artifacts.append(resolved_artifact.as_posix())
            require(
                not ops_claims_ready or not missing_required_ops_artifacts,
                "ops certification ready claim has no missing required artifacts",
                ops_certification_path,
                f"missing artifacts={missing_required_ops_artifacts[:20]}",
                blocking=not local_fixture,
            )
            if ops_certification.get("live_evidence_required") is True:
                require(
                    bool(retained_live_ops_artifacts),
                    "ops certification retains live ops evidence artifacts",
                    ops_certification_path,
                    f"retained live ops artifacts={retained_live_ops_artifacts[:20]}",
                    blocking=not local_fixture,
                )
        if ops_certification_required:
            require(
                ops_certification.get("release_ready") is True,
                "ops certification is release-ready",
                ops_certification_path,
                blocking=not local_fixture,
            )
        if live_ops_evidence_required:
            require(
                ops_certification.get("live_evidence_required") is True,
                "ops certification was generated with live ops evidence required",
                ops_certification_path,
                (
                    "rerun scripts/appfw ops-certification --json with "
                    "APPFW_RELEASE_REQUIRE_LIVE_OPS_EVIDENCE=true"
                ),
                blocking=not local_fixture,
            )
        if promtool_required:
            require(
                ops_certification.get("promtool_required") is True,
                "ops certification was generated with promtool required",
                ops_certification_path,
                (
                    "rerun scripts/appfw ops-certification --json with "
                    "APPFW_RELEASE_REQUIRE_PROMTOOL=true"
                ),
                blocking=not local_fixture,
            )
else:
    record_artifact(
        "ops-certification",
        ops_certification_file,
        required=False,
        required_in_ci=False,
        status="not-applicable",
        applicability="not-applicable",
        applicability_reason=(
            "not required unless APPFW_REQUIRE_OPS_CERTIFICATION or "
            "APPFW_RELEASE_REQUIRE_OPS_CERTIFICATION, APPFW_REQUIRE_LIVE_OPS_EVIDENCE, "
            "or APPFW_RELEASE_REQUIRE_LIVE_OPS_EVIDENCE is enabled, or the artifact is present"
        ),
    )
    record(
        "ops certification is not required for this evidence bundle",
        True,
        ops_certification_path,
        (
            "set APPFW_RELEASE_REQUIRE_LIVE_OPS_EVIDENCE=true in release gates that require "
            "live ops proof"
        ),
    )

pds_baseline_required = release_mode_requires_pds_baseline_evidence()
pds_baseline_file = report_dir / "pds-security-baseline.json"
pds_baseline_path = pds_baseline_file.as_posix()
if pds_baseline_required or pds_baseline_file.is_file():
    pds_baseline = load_required(
        "pds-security-baseline",
        "pds-security-baseline.json",
        dict,
        missing_detail=(
            "PDS Security Baseline r4.5 evidence is required by "
            "APPFW_REQUIRE_PDS_BASELINE_EVIDENCE or "
            "APPFW_RELEASE_REQUIRE_PDS_BASELINE_EVIDENCE"
        ),
        applicability="required" if pds_baseline_required else "optional",
    )
    if pds_baseline is not None:
        require(
            pds_baseline.get("command") == "pds-baseline",
            "PDS baseline command is stable",
            pds_baseline_path,
        )
        require(
            pds_baseline.get("ok") is True,
            "PDS baseline ok flag is true",
            pds_baseline_path,
            blocking=not local_fixture,
        )
        require(
            isinstance(pds_baseline.get("generated_at_utc"), str),
            "PDS baseline generated timestamp is present",
            pds_baseline_path,
        )
        now = dt.datetime.now(dt.timezone.utc)
        pds_generated_at = parse_timestamp_with_timezone(pds_baseline.get("generated_at_utc"))
        require(
            pds_generated_at is not None,
            "PDS baseline generated timestamp is valid",
            pds_baseline_path,
            "generated_at_utc must be ISO-8601 with timezone",
        )
        if pds_generated_at is not None:
            require(
                pds_generated_at <= now + EVIDENCE_TIMESTAMP_SKEW,
                "PDS baseline generated timestamp is not in the future",
                pds_baseline_path,
                pds_generated_at.isoformat().replace("+00:00", "Z"),
            )
            require(
                pds_generated_at >= now - PDS_BASELINE_MAX_AGE,
                "PDS baseline generated timestamp is recent",
                pds_baseline_path,
                (
                    f"generated_at_utc {pds_generated_at.isoformat().replace('+00:00', 'Z')} "
                    f"is older than {PDS_BASELINE_MAX_AGE_HOURS} hours"
                ),
            )
        baseline = pds_baseline.get("baseline")
        require(
            isinstance(baseline, dict),
            "PDS baseline metadata is recorded",
            pds_baseline_path,
        )
        if isinstance(baseline, dict):
            require(
                baseline.get("version") == "r4.5",
                "PDS baseline version is r4.5",
                pds_baseline_path,
            )
            require(
                baseline.get("traceability_doc")
                == "docs/release/pds-security-baseline-traceability.md",
                "PDS baseline traceability doc is stable",
                pds_baseline_path,
            )
        require(
            isinstance(pds_baseline.get("checks"), list)
            and bool(pds_baseline.get("checks")),
            "PDS baseline records checks",
            pds_baseline_path,
        )
        pds_checks = pds_baseline.get("checks")
        pds_failing_checks = []
        if isinstance(pds_checks, list):
            for index, pds_check in enumerate(pds_checks):
                check_path = f"{pds_baseline_path}#/checks/{index}"
                require(
                    isinstance(pds_check, dict),
                    "PDS baseline check entry is object",
                    check_path,
                )
                if not isinstance(pds_check, dict):
                    continue
                pds_check_name = pds_check.get("name")
                require(
                    isinstance(pds_check_name, str) and bool(pds_check_name.strip()),
                    "PDS baseline check entry records name",
                    check_path,
                    f"actual {pds_check_name!r}",
                )
                pds_check_ok = pds_check.get("ok")
                require(
                    isinstance(pds_check_ok, bool),
                    "PDS baseline check entry ok flag is boolean",
                    check_path,
                    f"actual {pds_check_ok!r}",
                )
                if pds_check_ok is False:
                    pds_failing_checks.append(
                        pds_check_name.strip() if isinstance(pds_check_name, str) and pds_check_name.strip() else f"checks[{index}]"
                    )
        if pds_baseline.get("ok") is True or pds_baseline.get("release_ready") is True:
            require(
                not pds_failing_checks,
                "PDS baseline ok claim has no failing checks",
                pds_baseline_path,
                f"failing checks={pds_failing_checks}",
                blocking=not local_fixture,
            )
        if pds_baseline.get("release_ready") is True:
            release_blockers = pds_baseline.get("release_blockers")
            failure_summary = pds_baseline.get("failure_summary")
            root_causes = (
                failure_summary.get("root_causes")
                if isinstance(failure_summary, dict)
                else None
            )
            require(
                not release_blockers,
                "PDS baseline release-ready claim has no release blockers",
                pds_baseline_path,
                f"release_blockers={release_blockers!r}",
                blocking=not local_fixture,
            )
            require(
                not root_causes,
                "PDS baseline release-ready claim has no failure-summary root causes",
                pds_baseline_path,
                f"root_causes={root_causes!r}",
                blocking=not local_fixture,
            )
        required_work_items = {"LIVE-015", "LIVE-016", "LIVE-017", "LIVE-018", "LIVE-019", "LIVE-020", "LIVE-021"}
        remediation_items = pds_baseline.get("remediation_work_items")
        remediation_ids = {
            str(item.get("id"))
            for item in remediation_items
            if isinstance(item, dict) and item.get("id") is not None
        } if isinstance(remediation_items, list) else set()
        require(
            required_work_items.issubset(remediation_ids) or pds_baseline.get("release_ready") is True,
            "PDS baseline remediation includes P0 live work items when not release-ready",
            pds_baseline_path,
            f"missing {sorted(required_work_items - remediation_ids)}",
        )
        require(
            isinstance(pds_baseline.get("failure_summary"), dict),
            "PDS baseline failure summary is recorded",
            pds_baseline_path,
        )
        require(
            isinstance(pds_baseline.get("decision_file"), str),
            "PDS baseline decision file path is recorded",
            pds_baseline_path,
        )
        pds_risk_acceptance = pds_baseline.get("risk_acceptance")
        pds_risk_max_days = 365
        require(
            isinstance(pds_risk_acceptance, dict),
            "PDS baseline risk acceptance policy is recorded",
            pds_baseline_path,
            blocking=not local_fixture if pds_baseline_required else True,
        )
        if isinstance(pds_risk_acceptance, dict):
            risk_max_days = pds_risk_acceptance.get("max_days")
            require(
                is_int(risk_max_days) and risk_max_days > 0,
                "PDS baseline risk acceptance max_days is positive",
                pds_baseline_path,
                f"actual {risk_max_days!r}",
                blocking=not local_fixture if pds_baseline_required else True,
            )
            risk_max_days_within_limit = (
                is_int(risk_max_days)
                and risk_max_days > 0
                and risk_max_days <= PDS_RISK_ACCEPTANCE_MAX_DAYS_LIMIT
            )
            require(
                risk_max_days_within_limit,
                "PDS baseline risk acceptance max_days does not exceed enterprise ceiling",
                pds_baseline_path,
                f"actual {risk_max_days!r}; ceiling {PDS_RISK_ACCEPTANCE_MAX_DAYS_LIMIT}",
                blocking=not local_fixture if pds_baseline_required else True,
            )
            if risk_max_days_within_limit:
                pds_risk_max_days = risk_max_days
        pds_decision_path = resolve_retained_path(pds_baseline.get("decision_file"))
        pds_decision = None
        pds_decision_work_item_summary = None
        pds_decision_evidence_paths = set()
        pds_claims_release_ready = pds_baseline.get("release_ready") is True
        pds_decision_required = pds_baseline_required or pds_claims_release_ready
        decision_artifact = pds_baseline.get("decision_artifact")
        decision_artifact_present = (
            isinstance(decision_artifact, dict)
            and decision_artifact.get("present") is True
        )
        pds_decision_metadata_required = (
            pds_decision_required
            or decision_artifact_present
            or pds_decision_path is not None
        )
        pds_decision_artifact_blocking = not local_fixture if pds_decision_metadata_required else False
        pds_decision_release_blocking = not local_fixture if pds_decision_required else False
        if pds_claims_release_ready and isinstance(release_check, dict):
            release_check_evidence_mode = release_check.get("evidence_mode")
            release_check_pds_mode = (
                release_check_evidence_mode.get("pds_security_baseline")
                if isinstance(release_check_evidence_mode, dict)
                else None
            )
            require(
                release_check_pds_mode == "live-required",
                "PDS baseline release-ready claim uses live release-check evidence mode",
                pds_baseline_path,
                f"actual {release_check_pds_mode or '<missing>'}",
                blocking=not local_fixture,
            )
        if pds_decision_metadata_required:
            require(
                pds_decision_path is not None,
                "PDS baseline retained decision file exists",
                pds_baseline_path,
                pds_baseline.get("decision_file") or "PDS baseline decision_file is missing",
                blocking=pds_decision_artifact_blocking,
            )
            require(
                isinstance(decision_artifact, dict),
                "PDS baseline retained decision artifact metadata is recorded",
                pds_baseline_path,
                blocking=pds_decision_artifact_blocking,
            )
            resolved_decision_artifact = validate_retained_file_artifact(
                "PDS baseline decision",
                decision_artifact,
                f"{pds_baseline_path}#/decision_artifact",
                blocking=pds_decision_artifact_blocking,
                required_parent=report_dir,
            )
            if pds_decision_path is not None and resolved_decision_artifact is not None:
                require(
                    pds_decision_path.resolve() == resolved_decision_artifact.resolve(),
                    "PDS baseline decision artifact matches decision_file",
                    pds_baseline_path,
                    (
                        f"decision_file={pds_decision_path.as_posix()} "
                        f"artifact={resolved_decision_artifact.as_posix()}"
                    ),
                    blocking=pds_decision_artifact_blocking,
                )
            if pds_decision_path is not None:
                require(
                    path_is_within_directory(pds_decision_path, report_dir),
                    "PDS baseline retained decision file is retained in report directory",
                    pds_baseline_path,
                    (
                        f"{pds_decision_path.as_posix()} is retained under {report_dir.as_posix()}"
                        if path_is_within_directory(pds_decision_path, report_dir)
                        else f"PDS baseline retained decision file must be retained under {report_dir.as_posix()}: {pds_decision_path.as_posix()}"
                    ),
                    blocking=pds_decision_artifact_blocking,
                )
            if pds_decision_path is not None:
                try:
                    pds_decision = json.loads(pds_decision_path.read_text(encoding="utf-8"))
                    require(
                        True,
                        "PDS baseline retained decision file parses as JSON",
                        pds_decision_path.as_posix(),
                        blocking=pds_decision_artifact_blocking,
                    )
                except Exception as exc:  # pragma: no cover - exercised by shell usage.
                    require(
                        False,
                        "PDS baseline retained decision file parses as JSON",
                        pds_decision_path.as_posix(),
                        str(exc),
                        blocking=pds_decision_artifact_blocking,
                    )
            if pds_decision is not None:
                require(
                    isinstance(pds_decision, dict),
                    "PDS baseline retained decision is a JSON object",
                    pds_decision_path.as_posix(),
                    blocking=pds_decision_artifact_blocking,
                )
                release_ready_decision = (
                    isinstance(pds_decision, dict)
                    and pds_decision.get("ok") is True
                    and pds_decision.get("release_ready") is True
                )
                require(
                    release_ready_decision,
                    "PDS baseline retained decision declares release-ready",
                    pds_decision_path.as_posix(),
                    "decision must declare ok:true and release_ready:true",
                    blocking=pds_decision_release_blocking,
                )
                decision_baseline_version = baseline_version_from_decision(pds_decision)
                require(
                    decision_baseline_version == "r4.5",
                    "PDS baseline retained decision references baseline r4.5",
                    pds_decision_path.as_posix(),
                    f"baseline version {decision_baseline_version or '<missing>'}",
                    blocking=pds_decision_artifact_blocking,
                )
                decision_authority = nested_mapping(pds_decision, ("release_authority", "authority", "approval"))
                decision_owner = first_non_empty(decision_authority, ("owner", "release_owner")) or first_non_empty(
                    pds_decision, ("owner", "release_owner")
                )
                decision_approver = first_non_empty(decision_authority, ("approver", "approved_by")) or first_non_empty(
                    pds_decision, ("approver", "approved_by")
                )
                decision_approved_at = first_non_empty(
                    decision_authority,
                    ("approved_at_utc", "approved_at", "approved_on"),
                ) or first_non_empty(pds_decision, ("approved_at_utc", "approved_at", "approved_on"))
                require(
                    bool(decision_owner),
                    "PDS baseline retained decision has release authority owner",
                    pds_decision_path.as_posix(),
                    blocking=pds_decision_artifact_blocking,
                )
                require(
                    bool(decision_approver),
                    "PDS baseline retained decision has release authority approver",
                    pds_decision_path.as_posix(),
                    blocking=pds_decision_artifact_blocking,
                )
                parsed_decision_approved_at = parse_timestamp_with_timezone(decision_approved_at)
                require(
                    parsed_decision_approved_at is not None,
                    "PDS baseline retained decision approval timestamp is valid",
                    pds_decision_path.as_posix(),
                    "approval timestamp must be ISO-8601 with timezone",
                    blocking=pds_decision_artifact_blocking,
                )
                if parsed_decision_approved_at is not None:
                    require(
                        parsed_decision_approved_at <= dt.datetime.now(dt.timezone.utc) + dt.timedelta(minutes=5),
                        "PDS baseline retained decision approval timestamp is not in the future",
                        pds_decision_path.as_posix(),
                        parsed_decision_approved_at.isoformat().replace("+00:00", "Z"),
                        blocking=pds_decision_artifact_blocking,
                    )
                pds_decision_work_item_summary = validate_pds_decision_work_items(
                    pds_decision,
                    pds_decision_path,
                    required_work_items,
                    pds_risk_max_days,
                    blocking=pds_decision_release_blocking,
                )
                pds_decision_evidence_paths = pds_decision_work_item_summary.get("evidence_paths", set())
        decision_summary = pds_baseline.get("decision_summary")
        require(
            isinstance(decision_summary, dict),
            "PDS baseline decision summary is recorded",
            pds_baseline_path,
        )
        if isinstance(decision_summary, dict):
            covered_ids = {
                str(item)
                for item in decision_summary.get("covered_work_items", [])
                if isinstance(item, str)
            }
            evidence_ids = {
                str(item)
                for item in decision_summary.get("evidence_backed_work_items", [])
                if isinstance(item, str)
            }
            risk_ids = {
                str(item)
                for item in decision_summary.get("risk_accepted_work_items", [])
                if isinstance(item, str)
            }
            satisfied_ids = evidence_ids | risk_ids
            require(
                isinstance(decision_summary.get("covered_work_items"), list),
                "PDS baseline decision summary records covered work items",
                pds_baseline_path,
            )
            require(
                isinstance(decision_summary.get("evidence_backed_work_items"), list),
                "PDS baseline decision summary records evidence-backed work items",
                pds_baseline_path,
            )
            require(
                isinstance(decision_summary.get("risk_accepted_work_items"), list),
                "PDS baseline decision summary records risk-accepted work items",
                pds_baseline_path,
            )
            if pds_decision_required and pds_decision_work_item_summary is not None:
                decision_covered_ids = pds_decision_work_item_summary.get("covered_work_items", set())
                decision_evidence_ids = pds_decision_work_item_summary.get("evidence_backed_work_items", set())
                decision_risk_ids = pds_decision_work_item_summary.get("risk_accepted_work_items", set())
                require(
                    covered_ids == decision_covered_ids,
                    "PDS baseline decision summary matches retained decision coverage",
                    pds_baseline_path,
                    (
                        f"summary={sorted(covered_ids)} "
                        f"decision={sorted(decision_covered_ids)}"
                    ),
                    blocking=not local_fixture,
                )
                require(
                    evidence_ids == decision_evidence_ids,
                    "PDS baseline decision summary matches retained evidence-backed work items",
                    pds_baseline_path,
                    (
                        f"summary={sorted(evidence_ids)} "
                        f"decision={sorted(decision_evidence_ids)}"
                    ),
                    blocking=not local_fixture,
                )
                require(
                    risk_ids == decision_risk_ids,
                    "PDS baseline decision summary matches retained risk-accepted work items",
                    pds_baseline_path,
                    (
                        f"summary={sorted(risk_ids)} "
                        f"decision={sorted(decision_risk_ids)}"
                    ),
                    blocking=not local_fixture,
                )
        else:
            covered_ids = set()
            satisfied_ids = set()
        evidence_artifacts = pds_baseline.get("evidence_artifacts")
        pds_decision_evidence_artifacts_required = (
            pds_decision_required
            or bool(pds_decision_evidence_paths)
        )
        pds_evidence_artifact_blocking = not local_fixture if pds_decision_evidence_artifacts_required else False
        if pds_decision_evidence_artifacts_required:
            require(
                isinstance(evidence_artifacts, list),
                "PDS baseline retained evidence artifacts are recorded",
                pds_baseline_path,
                blocking=pds_evidence_artifact_blocking,
            )
        artifact_paths = set()
        if isinstance(evidence_artifacts, list):
            for index, evidence_artifact in enumerate(evidence_artifacts):
                resolved_artifact = validate_retained_file_artifact(
                    "PDS baseline evidence",
                    evidence_artifact,
                    f"{pds_baseline_path}#/evidence_artifacts/{index}",
                    blocking=pds_evidence_artifact_blocking,
                    required_parent=report_dir,
                )
                if resolved_artifact is not None and path_is_within_directory(resolved_artifact, report_dir):
                    artifact_paths.add(resolved_artifact.as_posix())
        if pds_decision_evidence_artifacts_required and pds_decision_evidence_paths:
            missing_evidence_artifacts = sorted(pds_decision_evidence_paths - artifact_paths)
            require(
                not missing_evidence_artifacts,
                "PDS baseline retained evidence artifacts cover decision evidence files",
                pds_baseline_path,
                f"missing {missing_evidence_artifacts}",
                blocking=pds_evidence_artifact_blocking,
            )
        if pds_decision_required:
            if pds_decision_work_item_summary is not None:
                decision_covered_ids = pds_decision_work_item_summary.get("covered_work_items", set())
                decision_satisfied_ids = pds_decision_work_item_summary.get(
                    "evidence_backed_work_items",
                    set(),
                ) | pds_decision_work_item_summary.get("risk_accepted_work_items", set())
                require(
                    required_work_items.issubset(decision_covered_ids),
                    "PDS baseline retained decision covers live work items",
                    pds_baseline_path,
                    f"missing {sorted(required_work_items - decision_covered_ids)}",
                    blocking=not local_fixture,
                )
                require(
                    required_work_items.issubset(decision_satisfied_ids),
                    "PDS baseline retained decision satisfies live work items",
                    pds_baseline_path,
                    f"missing {sorted(required_work_items - decision_satisfied_ids)}",
                    blocking=not local_fixture,
                )
            require(
                pds_baseline.get("live_evidence_required") is True,
                "PDS baseline live_evidence_required flag is true",
                pds_baseline_path,
                f"actual {pds_baseline.get('live_evidence_required')!r}",
                blocking=not local_fixture,
            )
            require(
                pds_baseline.get("release_ready") is True,
                "PDS baseline release_ready flag is true",
                pds_baseline_path,
                f"actual {pds_baseline.get('release_ready')!r}",
                blocking=not local_fixture,
            )
            require(
                required_work_items.issubset(covered_ids),
                "PDS baseline decision summary covers live work items",
                pds_baseline_path,
                f"missing {sorted(required_work_items - covered_ids)}",
                blocking=not local_fixture,
            )
            require(
                required_work_items.issubset(satisfied_ids),
                "PDS baseline decision summary satisfies live work items",
                pds_baseline_path,
                f"missing {sorted(required_work_items - satisfied_ids)}",
                blocking=not local_fixture,
            )
        elif pds_baseline.get("release_ready") is not True:
            record(
                "PDS baseline is not release-ready",
                False,
                pds_baseline_path,
                "set APPFW_RELEASE_REQUIRE_PDS_BASELINE_EVIDENCE=true in production release gates and retain the baseline decision artifact",
                blocking=False,
            )
else:
    record_artifact(
        "pds-security-baseline",
        pds_baseline_file,
        required=False,
        required_in_ci=False,
        status="not-applicable",
        applicability="not-applicable",
        applicability_reason=(
            "not required unless APPFW_REQUIRE_PDS_BASELINE_EVIDENCE or "
            "APPFW_RELEASE_REQUIRE_PDS_BASELINE_EVIDENCE is enabled, or the artifact is present"
        ),
    )
    record(
        "PDS baseline evidence is not required for this evidence bundle",
        True,
        pds_baseline_path,
        "set APPFW_RELEASE_REQUIRE_PDS_BASELINE_EVIDENCE=true in production release gates",
    )

performance_evidence_required = release_mode_requires_performance_evidence()
performance_evidence_file = report_dir / "load-test.json"
performance_evidence_path = performance_evidence_file.as_posix()
if performance_evidence_required or performance_evidence_file.is_file():
    performance_evidence = load_required(
        "load-test",
        "load-test.json",
        dict,
        missing_detail=(
            "performance evidence is required by APPFW_REQUIRE_PERFORMANCE_EVIDENCE "
            "or APPFW_RELEASE_REQUIRE_PERFORMANCE_EVIDENCE"
        ),
        applicability="required" if performance_evidence_required else "optional",
    )
    if performance_evidence is not None:
        performance_command = performance_evidence.get("command")
        require(
            performance_command in {"load-test", "load-test-suite"},
            "performance evidence command is stable",
            performance_evidence_path,
        )
        if performance_evidence_required:
            require(
                performance_command == "load-test-suite",
                "performance evidence uses load-test-suite for release performance evidence",
                performance_evidence_path,
                (
                    "rerun scripts/appfw load-test-suite --json with "
                    "APPFW_RELEASE_REQUIRE_PERFORMANCE_EVIDENCE=true"
                ),
                blocking=not local_fixture,
            )
        require(
            performance_evidence.get("ok") is True,
            "performance evidence ok flag is true",
            performance_evidence_path,
            blocking=not local_fixture,
        )
        if performance_command == "load-test-suite":
            require(
                is_int(performance_evidence.get("scenario_count"))
                and performance_evidence.get("scenario_count") > 0,
                "load-test-suite scenario_count is a positive integer",
                performance_evidence_path,
            )
            for field in ["passed", "failed", "requests_per_scenario", "concurrency"]:
                require(
                    is_int(performance_evidence.get(field)),
                    f"load-test-suite {field} is an integer",
                    performance_evidence_path,
                )
            totals = performance_evidence.get("totals")
            require(
                isinstance(totals, dict),
                "load-test-suite totals are recorded",
                performance_evidence_path,
            )
            if isinstance(totals, dict):
                for field in ["requests", "success", "failed"]:
                    require(
                        is_int(totals.get(field)),
                        f"load-test-suite totals.{field} is an integer",
                        performance_evidence_path,
                    )
                for field in ["error_rate", "max_p95_ms", "max_latency_ms"]:
                    require(
                        is_number(totals.get(field)),
                        f"load-test-suite totals.{field} is numeric",
                        performance_evidence_path,
                    )
            require(
                isinstance(performance_evidence.get("scenarios"), list)
                and len(performance_evidence.get("scenarios")) > 0,
                "load-test-suite scenarios are recorded",
                performance_evidence_path,
            )
            require(
                isinstance(performance_evidence.get("thresholds"), dict),
                "load-test-suite thresholds are recorded",
                performance_evidence_path,
            )
            require(
                isinstance(performance_evidence.get("violations"), list),
                "load-test-suite violations is an array",
                performance_evidence_path,
            )
            if performance_evidence_required:
                require(
                    performance_evidence.get("failed") == 0,
                    "load-test-suite has zero failed scenarios",
                    performance_evidence_path,
                    blocking=not local_fixture,
                )
                require(
                    performance_evidence.get("violations") == [],
                    "load-test-suite has no threshold violations",
                    performance_evidence_path,
                    blocking=not local_fixture,
                )
        else:
            require(
                isinstance(performance_evidence.get("scenario"), str)
                and bool(performance_evidence.get("scenario")),
                "load-test scenario is present",
                performance_evidence_path,
            )
            for field in ["requests", "concurrency", "success", "failed"]:
                require(
                    is_int(performance_evidence.get(field)),
                    f"load-test {field} is an integer",
                    performance_evidence_path,
                )
            require(
                is_number(performance_evidence.get("error_rate")),
                "load-test error_rate is numeric",
                performance_evidence_path,
            )
            latency = performance_evidence.get("latency_ms")
            require(
                isinstance(latency, dict),
                "load-test latency_ms is an object",
                performance_evidence_path,
            )
            if isinstance(latency, dict):
                for field in ["min", "avg", "p95", "max"]:
                    require(
                        is_number(latency.get(field)),
                        f"load-test latency_ms.{field} is numeric",
                        performance_evidence_path,
                    )
            require(
                isinstance(performance_evidence.get("thresholds"), dict),
                "load-test thresholds are recorded",
                performance_evidence_path,
            )
            require(
                isinstance(performance_evidence.get("violations"), list),
                "load-test violations is an array",
                performance_evidence_path,
            )
            if performance_evidence_required:
                require(
                    performance_evidence.get("failed") == 0,
                    "load-test has zero failed requests",
                    performance_evidence_path,
                    blocking=not local_fixture,
                )
                require(
                    performance_evidence.get("violations") == [],
                    "load-test has no threshold violations",
                    performance_evidence_path,
                    blocking=not local_fixture,
                )
else:
    record_artifact(
        "load-test",
        performance_evidence_file,
        required=False,
        required_in_ci=False,
        status="not-applicable",
        applicability="not-applicable",
        applicability_reason=(
            "not required unless APPFW_REQUIRE_PERFORMANCE_EVIDENCE or "
            "APPFW_RELEASE_REQUIRE_PERFORMANCE_EVIDENCE is enabled, or the artifact is present"
        ),
    )
    record(
        "performance evidence is not required for this evidence bundle",
        True,
        performance_evidence_path,
        "set APPFW_RELEASE_REQUIRE_PERFORMANCE_EVIDENCE=true in release gates that require load proof",
    )

provider_performance_file = report_dir / "provider-performance.json"
provider_performance_path = provider_performance_file.as_posix()
if performance_evidence_required or provider_performance_file.is_file():
    provider_performance = load_required(
        "provider-performance",
        "provider-performance.json",
        dict,
        missing_detail=(
            "provider performance evidence is required when "
            "APPFW_REQUIRE_PERFORMANCE_EVIDENCE or "
            "APPFW_RELEASE_REQUIRE_PERFORMANCE_EVIDENCE is enabled"
        ),
        applicability="required" if performance_evidence_required else "optional",
    )
    if provider_performance is not None:
        require(
            provider_performance.get("command") == "provider-performance",
            "provider-performance command is stable",
            provider_performance_path,
        )
        require(
            provider_performance.get("ok") is True,
            "provider-performance ok flag is true",
            provider_performance_path,
            blocking=not local_fixture,
        )
        require(
            isinstance(provider_performance.get("checks"), list),
            "provider-performance checks are recorded",
            provider_performance_path,
        )
        require(
            isinstance(provider_performance.get("providers"), list),
            "provider-performance providers are recorded",
            provider_performance_path,
        )
        summary = provider_performance.get("summary")
        require(
            isinstance(summary, dict),
            "provider-performance summary is recorded",
            provider_performance_path,
        )
        if isinstance(summary, dict):
            for field in ["provider_count", "provider_ok_count", "blocking_failure_count"]:
                require(
                    is_int(summary.get(field)),
                    f"provider-performance summary.{field} is an integer",
                    provider_performance_path,
                )
            if performance_evidence_required:
                for field in ["recommendation_count", "generated_recommendation_count"]:
                    require(
                        is_int(summary.get(field)),
                        f"provider-performance summary.{field} is an integer",
                        provider_performance_path,
                        blocking=not local_fixture,
                    )
                    require(
                        is_int(summary.get(field)) and summary.get(field) > 0,
                        f"provider-performance summary.{field} is greater than zero",
                        provider_performance_path,
                        f"actual {summary.get(field)!r}",
                        blocking=not local_fixture,
                    )
        if performance_evidence_required:
            performance_artifacts = provider_performance.get("artifacts")
            require(
                isinstance(performance_artifacts, dict),
                "provider-performance artifacts are recorded",
                provider_performance_path,
                blocking=not local_fixture,
            )
            if isinstance(performance_artifacts, dict):
                expected_artifacts = [
                    (
                        "provider_parity",
                        report_dir / "provider-parity.json",
                        report_dir,
                        "report directory",
                    ),
                    (
                        "performance_recommendations",
                        report_dir / "performance_recommendations.json",
                        report_dir,
                        "report directory",
                    ),
                    (
                        "query_cost_source",
                        repo_root / "appfw_runtime/src/query_cost.rs",
                        repo_root,
                        "repository",
                    ),
                ]
                for artifact_name, expected_path, required_parent, parent_label in expected_artifacts:
                    artifact = performance_artifacts.get(artifact_name)
                    require(
                        isinstance(artifact, dict) and artifact.get("required") is True,
                        f"provider-performance {artifact_name} artifact is required",
                        f"{provider_performance_path}#/artifacts/{artifact_name}",
                        f"actual {artifact.get('required')!r}" if isinstance(artifact, dict) else "missing artifact",
                        blocking=not local_fixture,
                    )
                    resolved_artifact = validate_retained_file_artifact(
                        f"provider-performance {artifact_name}",
                        artifact,
                        f"{provider_performance_path}#/artifacts/{artifact_name}",
                        not local_fixture,
                        required_parent=required_parent,
                        required_parent_label=parent_label,
                    )
                    expected = expected_path.resolve(strict=False)
                    require(
                        resolved_artifact is not None
                        and resolved_artifact.resolve(strict=False) == expected,
                        f"provider-performance {artifact_name} artifact matches retained evidence",
                        f"{provider_performance_path}#/artifacts/{artifact_name}",
                        (
                            f"expected {expected.as_posix()}, "
                            f"actual {resolved_artifact.resolve(strict=False).as_posix() if resolved_artifact is not None else '<missing>'}"
                        ),
                        blocking=not local_fixture,
                    )
            providers = provider_performance.get("providers")
            if isinstance(providers, list):
                provider_names = {
                    str(provider.get("provider"))
                    for provider in providers
                    if isinstance(provider, dict) and provider.get("provider") is not None
                }
                missing_providers = sorted(set(PERFORMANCE_EXPECTED_PROVIDERS) - provider_names)
                require(
                    not missing_providers,
                    "provider-performance includes release-certified providers",
                    provider_performance_path,
                    f"missing {missing_providers}",
                    blocking=not local_fixture,
                )
                for provider in providers:
                    name = provider.get("provider") if isinstance(provider, dict) else None
                    require(
                        isinstance(provider, dict) and provider.get("ok") is True,
                        f"provider-performance {name or '<unknown>'} provider is ok",
                        provider_performance_path,
                        blocking=not local_fixture,
                    )
                    if name in PERFORMANCE_EXPECTED_PROVIDERS and isinstance(provider, dict):
                        require(
                            provider.get("status") == "live-certified",
                            f"provider-performance {name} provider is live-certified",
                            provider_performance_path,
                            blocking=not local_fixture,
                        )
                        require(
                            isinstance(provider.get("base_url"), str) and bool(provider.get("base_url").strip()),
                            f"provider-performance {name} provider records live base URL",
                            provider_performance_path,
                            blocking=not local_fixture,
                        )
                        provider_checks = provider.get("checks")
                        require(
                            isinstance(provider_checks, list) and bool(provider_checks),
                            f"provider-performance {name} provider records checks",
                            provider_performance_path,
                            blocking=not local_fixture,
                        )
                        if isinstance(provider_checks, list):
                            for check in provider_checks:
                                check_name = check.get("name") if isinstance(check, dict) else "<unknown>"
                                require(
                                    isinstance(check, dict) and check.get("ok") is True,
                                    f"provider-performance {name} {check_name} check is ok",
                                    provider_performance_path,
                                    blocking=not local_fixture,
                                )
                                areas = check.get("areas") if isinstance(check, dict) else None
                                if isinstance(areas, list):
                                    for area in areas:
                                        area_name = area.get("area") if isinstance(area, dict) else "<unknown>"
                                        require(
                                            isinstance(area, dict)
                                            and area.get("ok") is True
                                            and area.get("live_result") == "passed",
                                            f"provider-performance {name} {area_name} area live result is passing",
                                            provider_performance_path,
                                            blocking=not local_fixture,
                                        )
            require(
                provider_performance.get("blocking_failures") == [],
                "provider-performance has no blocking failures",
                provider_performance_path,
                blocking=not local_fixture,
            )
else:
    record_artifact(
        "provider-performance",
        provider_performance_file,
        required=False,
        required_in_ci=False,
        status="not-applicable",
        applicability="not-applicable",
        applicability_reason=(
            "not required unless APPFW_REQUIRE_PERFORMANCE_EVIDENCE or "
            "APPFW_RELEASE_REQUIRE_PERFORMANCE_EVIDENCE is enabled, or the artifact is present"
        ),
    )
    record(
        "provider performance evidence is not required for this evidence bundle",
        True,
        provider_performance_path,
        "set APPFW_RELEASE_REQUIRE_PERFORMANCE_EVIDENCE=true in release gates that require provider performance proof",
    )

security_cert_path = (report_dir / "security-certification.json").as_posix()
security_cert_artifact_paths = {}
security_cert_generated_at = None
security_cert = load_required("security-certification", "security-certification.json", dict)
if security_cert is not None:
    require(
        security_cert.get("command") == "security-certification-evidence",
        "security certification command is stable",
        security_cert_path,
    )
    require(
        security_cert.get("ok") is True,
        "security certification ok flag is true",
        security_cert_path,
        blocking=not local_fixture,
    )
    require(
        security_cert.get("release_ready") is True,
        "security certification is release-ready",
        security_cert_path,
        blocking=not local_fixture,
    )
    require(
        isinstance(security_cert.get("generated_at_utc"), str),
        "security certification generated timestamp is present",
        security_cert_path,
    )
    security_cert_artifact_paths = validate_security_certification_artifacts(
        security_cert,
        security_cert_path,
        blocking=not local_fixture,
    )
    security_checks = security_cert.get("checks")
    require(
        isinstance(security_checks, list),
        "security certification checks is an array",
        security_cert_path,
    )
    security_check_names = []
    security_failing_checks = []
    if isinstance(security_checks, list):
        require(
            bool(security_checks),
            "security certification has at least one check",
            security_cert_path,
        )
        for index, check in enumerate(security_checks):
            check_path = f"{security_cert_path}#/checks/{index}"
            require(isinstance(check, dict), "security certification check is object", check_path)
            if isinstance(check, dict):
                check_name = check.get("name")
                require(
                    isinstance(check_name, str) and bool(check_name.strip()),
                    "security certification check name",
                    check_path,
                    f"actual {check_name!r}",
                )
                if isinstance(check_name, str) and check_name.strip():
                    security_check_names.append(check_name.strip())
                check_ok = check.get("ok")
                require(
                    check_ok is True,
                    "security certification check ok",
                    check_path,
                    check_name if isinstance(check_name, str) else None,
                    blocking=not local_fixture,
                )
                if check_ok is not True:
                    security_failing_checks.append(
                        check_name.strip() if isinstance(check_name, str) and check_name.strip() else f"checks[{index}]"
                    )
        duplicate_security_check_names = duplicate_values(security_check_names)
        require(
            not duplicate_security_check_names,
            "security certification check names are unique",
            security_cert_path,
            f"duplicates={duplicate_security_check_names}",
        )
    security_failures = security_cert.get("failures")
    security_release_blockers = security_cert.get("release_blockers")
    security_failure_count = security_cert.get("failure_count")
    require(
        is_int(security_failure_count),
        "security certification failure_count is an integer",
        security_cert_path,
        f"actual {security_failure_count!r}",
    )
    require(
        isinstance(security_failures, list),
        "security certification failures are recorded",
        security_cert_path,
    )
    require(
        isinstance(security_release_blockers, list),
        "security certification release_blockers are recorded",
        security_cert_path,
    )
    if isinstance(security_failures, list):
        for index, failure in enumerate(security_failures):
            require(
                isinstance(failure, str) and bool(failure.strip()),
                "security certification failure entry is non-empty",
                f"{security_cert_path}#/failures/{index}",
                f"actual {failure!r}",
            )
        if is_int(security_failure_count):
            require(
                security_failure_count == len(security_failures),
                "security certification failure_count matches retained failures",
                security_cert_path,
                f"failure_count={security_failure_count!r}; failures={len(security_failures)}",
            )
            require(
                security_failure_count == len(security_failing_checks),
                "security certification failure_count matches retained failing checks",
                security_cert_path,
                f"failure_count={security_failure_count!r}; failing_checks={len(security_failing_checks)}",
            )
    if security_cert.get("ok") is True or security_cert.get("release_ready") is True:
        require(
            not security_failing_checks,
            "security certification ready claim has no failing checks",
            security_cert_path,
            f"failing checks={security_failing_checks}",
            blocking=not local_fixture,
        )
        require(
            not security_failures,
            "security certification ready claim has no retained failures",
            security_cert_path,
            f"failures={security_failures!r}",
            blocking=not local_fixture,
        )
        require(
            not security_release_blockers,
            "security certification ready claim has no release blockers",
            security_cert_path,
            f"release_blockers={security_release_blockers!r}",
            blocking=not local_fixture,
        )
        require(
            security_failure_count == 0,
            "security certification ready claim has zero failure_count",
            security_cert_path,
            f"failure_count={security_failure_count!r}",
            blocking=not local_fixture,
        )

    introspection = security_cert.get("introspection_auth")
    require(
        isinstance(introspection, dict),
        "security certification introspection_auth block is present",
        security_cert_path,
    )
    if isinstance(introspection, dict):
        require(
            introspection.get("evidence_type") == "focused runtime unit regression",
            "introspection auth evidence is focused runtime regression",
            security_cert_path,
        )
        cases = introspection.get("cases")
        require(isinstance(cases, list), "introspection auth cases is an array", security_cert_path)
        if isinstance(cases, list):
            cases_by_name = {
                case.get("case"): case
                for case in cases
                if isinstance(case, dict) and isinstance(case.get("case"), str)
            }
            for expected_case in SECURITY_INTROSPECTION_CASES:
                case = cases_by_name.get(expected_case)
                case_path = f"{security_cert_path}#/introspection_auth/cases/{expected_case}"
                require(case is not None, f"introspection case {expected_case} is present", case_path)
                if isinstance(case, dict):
                    require(
                        case.get("source_present") is True,
                        f"introspection case {expected_case} source test is present",
                        case_path,
                    )
                    require(
                        case.get("runtime_result") == "passed",
                        f"introspection case {expected_case} runtime result passed",
                        case_path,
                        blocking=not local_fixture,
                    )
                    require(
                        case.get("ok") is True,
                        f"introspection case {expected_case} ok",
                        case_path,
                        blocking=not local_fixture,
                    )

    live_security = security_cert.get("live_security_contracts")
    require(
        isinstance(live_security, dict),
        "security certification live_security_contracts block is present",
        security_cert_path,
    )
    if isinstance(live_security, dict):
        require(
            live_security.get("expected_providers") == SECURITY_EXPECTED_PROVIDERS,
            "live security expected providers are stable",
            security_cert_path,
        )
        expected_contracts = live_security.get("expected_contracts")
        require(
            isinstance(expected_contracts, list),
            "live security expected_contracts is an array",
            security_cert_path,
        )
        if isinstance(expected_contracts, list):
            expected_names = [
                contract.get("contract")
                for contract in expected_contracts
                if isinstance(contract, dict)
            ]
            duplicate_expected_contracts = sorted(
                contract_name
                for contract_name in set(expected_names)
                if isinstance(contract_name, str)
                and expected_names.count(contract_name) > 1
            )
            require(
                not duplicate_expected_contracts,
                "live security expected contracts name each contract once",
                security_cert_path,
                f"duplicates={duplicate_expected_contracts}",
            )
            for contract in SECURITY_EXPECTED_CONTRACTS:
                require(
                    contract in expected_names,
                    f"live security expected contract {contract} is listed",
                    security_cert_path,
                )

        providers = live_security.get("providers")
        require(isinstance(providers, list), "live security providers is an array", security_cert_path)
        if isinstance(providers, list):
            provider_names = []
            for index, provider in enumerate(providers):
                provider_entry_path = f"{security_cert_path}#/live_security_contracts/providers/{index}"
                require(
                    isinstance(provider, dict),
                    "live security provider entry is object",
                    provider_entry_path,
                )
                if not isinstance(provider, dict):
                    continue
                provider_name = provider.get("provider")
                provider_name_ok = (
                    isinstance(provider_name, str)
                    and provider_name.strip() in SECURITY_EXPECTED_PROVIDERS
                )
                require(
                    provider_name_ok,
                    "live security provider entry names release-certified provider",
                    provider_entry_path,
                    f"actual {provider_name!r}",
                )
                if provider_name_ok:
                    provider_names.append(provider_name.strip())
            duplicate_provider_names = sorted(
                provider_name
                for provider_name in set(provider_names)
                if provider_names.count(provider_name) > 1
            )
            require(
                not duplicate_provider_names,
                "live security providers name each provider once",
                security_cert_path,
                f"duplicates={duplicate_provider_names}",
            )
            providers_by_name = {
                provider.get("provider"): provider
                for provider in providers
                if isinstance(provider, dict) and isinstance(provider.get("provider"), str)
            }
            for expected_provider in SECURITY_EXPECTED_PROVIDERS:
                provider = providers_by_name.get(expected_provider)
                provider_path = f"{security_cert_path}#/live_security_contracts/providers/{expected_provider}"
                require(provider is not None, f"{expected_provider} live security provider is present", provider_path)
                if isinstance(provider, dict):
                    require(
                        provider.get("ok") is True,
                        f"{expected_provider} live security provider ok",
                        provider_path,
                        blocking=not local_fixture,
                    )
                    contracts = provider.get("contracts")
                    require(
                        isinstance(contracts, list),
                        f"{expected_provider} live security contracts is an array",
                        provider_path,
                    )
                    if isinstance(contracts, list):
                        contract_names = [
                            contract.get("contract")
                            for contract in contracts
                            if isinstance(contract, dict)
                        ]
                        duplicate_contract_names = sorted(
                            contract_name
                            for contract_name in set(contract_names)
                            if isinstance(contract_name, str)
                            and contract_names.count(contract_name) > 1
                        )
                        require(
                            not duplicate_contract_names,
                            f"{expected_provider} live security contracts name each contract once",
                            provider_path,
                            f"duplicates={duplicate_contract_names}",
                        )
                        contracts_by_name = {
                            contract.get("contract"): contract
                            for contract in contracts
                            if isinstance(contract, dict)
                            and isinstance(contract.get("contract"), str)
                        }
                        for contract_name in SECURITY_EXPECTED_CONTRACTS:
                            contract = contracts_by_name.get(contract_name)
                            contract_path = f"{provider_path}/contracts/{contract_name}"
                            require(
                                contract is not None,
                                f"{expected_provider} live security contract {contract_name} is present",
                                contract_path,
                            )
                            if isinstance(contract, dict):
                                require(
                                    contract.get("result") == "passed",
                                    f"{expected_provider} live security contract {contract_name} passed",
                                    contract_path,
                                    blocking=not local_fixture,
                                )
                                require(
                                    contract.get("ok") is True,
                                    f"{expected_provider} live security contract {contract_name} ok",
                                    contract_path,
                                    blocking=not local_fixture,
                                )

provider_parity_file = report_dir / "provider-parity.json"
provider_parity_path = provider_parity_file.as_posix()
provider_parity = None
provider_parity_validation = {}
provider_parity_required = (
    not local_fixture
    and (
        (isinstance(release_check, dict) and release_check.get("release_ready") is True)
        or (isinstance(security_cert, dict) and security_cert.get("release_ready") is True)
    )
)
if provider_parity_required or provider_parity_file.is_file():
    provider_parity = load_required(
        "provider-parity",
        "provider-parity.json",
        dict,
        missing_detail=(
            "provider parity evidence is required when release-check or "
            "security-certification claims release_ready:true"
        ),
        applicability="required" if provider_parity_required else "optional",
    )
    if provider_parity is not None:
        provider_parity_validation = validate_provider_parity_report(
            provider_parity,
            provider_parity_path,
            provider_preflight_valid_base_urls,
            blocking=not local_fixture,
        )
else:
    record_artifact(
        "provider-parity",
        provider_parity_file,
        required=False,
        required_in_ci=False,
        status="not-applicable",
        applicability="not-applicable",
        applicability_reason=(
            "not required unless release-check or security-certification is release-ready, "
            "or the artifact is present"
        ),
    )

security_cert_generated_at = validate_security_certification_timestamp(
    security_cert,
    security_cert_path,
    provider_parity_validation.get("generated_at"),
    blocking=not local_fixture,
)

validate_security_certification_cross_references(
    security_cert,
    security_cert_path,
    security_cert_artifact_paths,
    provider_parity_file,
    provider_parity_validation.get("provider_logs_by_name", {}),
    blocking=not local_fixture,
)

agent_handoff_file = report_dir / "agent-handoff.json"
agent_handoff_path = agent_handoff_file.as_posix()
if agent_handoff_file.is_file():
    record_artifact(
        "agent-handoff",
        agent_handoff_file,
        required=False,
        required_in_ci=False,
        status="present",
        applicability="optional",
        applicability_reason="validated when retained as release handoff evidence",
    )
    try:
        agent_handoff = json.loads(agent_handoff_file.read_text(encoding="utf-8"))
        require(
            isinstance(agent_handoff, dict),
            "agent handoff evidence is object",
            agent_handoff_path,
        )
    except Exception as exc:  # pragma: no cover - exercised by shell usage.
        agent_handoff = None
        require(False, "agent handoff evidence parses as JSON", agent_handoff_path, str(exc))
    if isinstance(agent_handoff, dict):
        require(
            agent_handoff.get("command") == "handoff",
            "agent handoff command is stable",
            agent_handoff_path,
            f"actual {agent_handoff.get('command')!r}",
        )
        require(
            agent_handoff.get("ok") is True,
            "agent handoff ok flag is true",
            agent_handoff_path,
            f"actual {agent_handoff.get('ok')!r}",
            blocking=not local_fixture,
        )
        artifact_path_text = agent_handoff.get("artifact_path")
        resolved_handoff_artifact = resolve_retained_path(artifact_path_text)
        require(
            resolved_handoff_artifact is not None and paths_match(resolved_handoff_artifact, agent_handoff_file),
            "agent handoff artifact_path matches retained handoff file",
            agent_handoff_path,
            f"actual {artifact_path_text!r}",
            blocking=not local_fixture,
        )
        require(
            isinstance(agent_handoff.get("generated_at"), str),
            "agent handoff generated timestamp is present",
            agent_handoff_path,
        )
        handoff_generated_at = parse_timestamp_with_timezone(agent_handoff.get("generated_at"))
        require(
            handoff_generated_at is not None,
            "agent handoff generated timestamp is valid",
            agent_handoff_path,
            "generated_at must be ISO-8601 with timezone",
            blocking=not local_fixture,
        )
        if handoff_generated_at is not None:
            now = dt.datetime.now(dt.timezone.utc)
            require(
                handoff_generated_at <= now + EVIDENCE_TIMESTAMP_SKEW,
                "agent handoff generated timestamp is not in the future",
                agent_handoff_path,
                handoff_generated_at.isoformat().replace("+00:00", "Z"),
                blocking=not local_fixture,
            )
            require(
                handoff_generated_at >= now - AGENT_HANDOFF_MAX_AGE,
                "agent handoff generated timestamp is recent",
                agent_handoff_path,
                (
                    f"generated_at {handoff_generated_at.isoformat().replace('+00:00', 'Z')} "
                    f"is older than {AGENT_HANDOFF_MAX_AGE_HOURS} hours"
                ),
                blocking=not local_fixture,
            )
        git = agent_handoff.get("git")
        require(isinstance(git, dict), "agent handoff git block is recorded", agent_handoff_path)
        if isinstance(git, dict):
            current_sha = git_output(["rev-parse", "HEAD"])
            current_branch = git_output(["rev-parse", "--abbrev-ref", "HEAD"])
            handoff_sha = git.get("sha")
            handoff_branch = git.get("branch")
            require(
                isinstance(handoff_sha, str) and bool(re.fullmatch(r"[0-9a-f]{40}", handoff_sha)),
                "agent handoff git sha is recorded",
                agent_handoff_path,
                f"actual {handoff_sha!r}",
                blocking=not local_fixture,
            )
            if current_sha:
                require(
                    handoff_sha == current_sha,
                    "agent handoff git sha matches current HEAD",
                    agent_handoff_path,
                    f"handoff={handoff_sha!r}; current={current_sha!r}",
                    blocking=not local_fixture,
                )
            require(
                isinstance(handoff_branch, str) and bool(handoff_branch.strip()),
                "agent handoff git branch is recorded",
                agent_handoff_path,
                f"actual {handoff_branch!r}",
                blocking=not local_fixture,
            )
            if current_branch:
                require(
                    handoff_branch == current_branch,
                    "agent handoff git branch matches current branch",
                    agent_handoff_path,
                    f"handoff={handoff_branch!r}; current={current_branch!r}",
                    blocking=not local_fixture,
                )
            require(
                isinstance(git.get("dirty"), bool),
                "agent handoff git dirty flag is boolean",
                agent_handoff_path,
                f"actual {git.get('dirty')!r}",
            )
        changed_surfaces = agent_handoff.get("changed_surfaces")
        require(
            isinstance(changed_surfaces, list),
            "agent handoff changed surfaces are recorded",
            agent_handoff_path,
        )
        drift = agent_handoff.get("drift")
        require(isinstance(drift, dict), "agent handoff drift block is recorded", agent_handoff_path)
        if isinstance(drift, dict):
            total_changed = drift.get("total_changed")
            require(
                is_int(total_changed),
                "agent handoff drift total_changed is an integer",
                agent_handoff_path,
                f"actual {total_changed!r}",
            )
            if isinstance(changed_surfaces, list) and is_int(total_changed):
                require(
                    total_changed == len(changed_surfaces),
                    "agent handoff drift total_changed matches changed surfaces",
                    agent_handoff_path,
                    f"total_changed={total_changed!r}; changed_surfaces={len(changed_surfaces)}",
                    blocking=not local_fixture,
                )
        verification = agent_handoff.get("verification")
        require(
            isinstance(verification, list),
            "agent handoff verification entries are recorded",
            agent_handoff_path,
        )
        if isinstance(verification, list):
            verification_by_name = {
                item.get("name"): item
                for item in verification
                if isinstance(item, dict) and isinstance(item.get("name"), str)
            }
            duplicate_verification_names = duplicate_values(
                [
                    item.get("name")
                    for item in verification
                    if isinstance(item, dict) and isinstance(item.get("name"), str)
                ]
            )
            require(
                not duplicate_verification_names,
                "agent handoff verification names are unique",
                agent_handoff_path,
                f"duplicates={duplicate_verification_names}",
            )
            verification_expectations = {
                "release check": release_check.get("ok") if isinstance(release_check, dict) else None,
                "provider certification": provider_parity.get("ok")
                if "provider_parity" in globals() and isinstance(provider_parity, dict)
                else None,
                "ops certification": ops_certification.get("ok")
                if "ops_certification" in globals() and isinstance(ops_certification, dict)
                else None,
            }
            for name, expected_ok in verification_expectations.items():
                item = verification_by_name.get(name)
                require(
                    isinstance(item, dict),
                    f"agent handoff verification includes {name}",
                    agent_handoff_path,
                    blocking=not local_fixture,
                )
                if isinstance(item, dict) and expected_ok is not None:
                    require(
                        item.get("ok") == expected_ok,
                        f"agent handoff verification {name} matches retained evidence",
                        agent_handoff_path,
                        f"handoff={item.get('ok')!r}; retained={expected_ok!r}",
                        blocking=not local_fixture,
                    )
else:
    record_artifact(
        "agent-handoff",
        agent_handoff_file,
        required=False,
        required_in_ci=False,
        status="not-applicable",
        applicability="not-applicable",
        applicability_reason="validated when retained as release handoff evidence",
    )

security_certification_summary = summarize_security_certification(security_cert)
provider_parity_summary = summarize_provider_parity(provider_parity)
assertion_failure_summary = summarize_display_failures(
    failures,
    security_certification_summary,
    provider_parity_summary,
)
release_blocker_summary = summarize_display_failures(
    release_blockers,
    security_certification_summary,
    provider_parity_summary,
)

if isinstance(bitbucket_gate, dict):
    canonical_wrapper_causes = [
        item
        for item in release_blocker_summary + assertion_failure_summary
        if not item.startswith("bitbucket release gate ")
    ]
    if bitbucket_gate.get("release_ready") is True:
        require(
            not canonical_wrapper_causes,
            "bitbucket release gate ready claim matches current release evidence",
            bitbucket_gate_path,
            f"current root causes={canonical_wrapper_causes}",
            blocking=not local_fixture,
        )
        if canonical_wrapper_causes:
            assertion_failure_summary = summarize_display_failures(
                failures,
                security_certification_summary,
                provider_parity_summary,
            )
            release_blocker_summary = summarize_display_failures(
                release_blockers,
                security_certification_summary,
                provider_parity_summary,
            )
    else:
        missing_wrapper_causes = [
            item for item in canonical_wrapper_causes if item not in normalized_bitbucket_blockers
        ]
        require(
            not missing_wrapper_causes,
            "bitbucket release gate includes canonical release evidence root causes",
            bitbucket_gate_path,
            f"missing={missing_wrapper_causes}",
            blocking=not local_fixture,
        )
        if missing_wrapper_causes:
            assertion_failure_summary = summarize_display_failures(
                failures,
                security_certification_summary,
                provider_parity_summary,
            )
            release_blocker_summary = summarize_display_failures(
                release_blockers,
                security_certification_summary,
                provider_parity_summary,
            )

root_causes = []
for item in release_blocker_summary + assertion_failure_summary:
    append_unique(root_causes, item)

redaction = {
    "phi_source_excerpts_retained": (
        phi.get("redaction", {}).get("json_source_excerpt_retained")
        if isinstance(phi, dict)
        else None
    ),
    "phi_finding_values_retained": (
        phi.get("redaction", {}).get("json_finding_values_retained")
        if isinstance(phi, dict)
        else None
    ),
    "gitleaks_secret_fields_redacted": unredacted_secret_count == 0
    if gitleaks is not None
    else None,
}

evidence = {
    "command": "release-evidence-check",
    "mode": mode,
    "ok": not failures,
    "schema_ok": not failures,
    "release_ready": not failures and not release_blockers,
    "generated_at_utc": utc_now(),
    "checks": checks,
    "artifacts": artifacts,
    "deployable_image_sbom": {
        "required": image_context["required"],
        "applicability": image_context["applicability"],
        "detail": image_context["detail"],
        "env_sources": image_context["env_sources"],
        "retained_artifacts": image_context["retained_artifacts"],
        "required_by_release_mode": image_context["required_by_release_mode"],
        "supply_required": image_context["supply_required"],
        "manifest_required": image_context["manifest_required"],
    },
    "synthetic_fixtures": synthetic_fixtures,
    "redaction": redaction,
    "failure_count": len(failures),
    "failures": failures,
    "failure_summary": {
        "root_causes": root_causes,
        "assertion_failures": assertion_failure_summary,
        "release_blockers": release_blocker_summary,
    },
    "release_blocker_count": len(release_blockers),
    "release_blockers": release_blockers,
    "release_blocker_summary": release_blocker_summary,
}

evidence_file.write_text(
    json.dumps(evidence, indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
)

if failures:
    print("release-evidence-check: FAILED", file=sys.stderr)
    for failure in root_causes:
        print(f"  - {failure}", file=sys.stderr)
    print(f"release-evidence-check: evidence written to {evidence_file}", file=sys.stderr)
    sys.exit(1)

if release_blockers:
    if local_fixture:
        print(
            "release-evidence-check: OK for local schema validation; "
            "not release-ready",
            file=sys.stderr,
        )
    else:
        print("release-evidence-check: NOT RELEASE-READY", file=sys.stderr)
    for blocker in release_blocker_summary:
        print(f"  - {blocker}", file=sys.stderr)
    print(f"release-evidence-check: evidence written to {evidence_file}", file=sys.stderr)
    sys.exit(0 if local_fixture else 1)

print(f"release-evidence-check: OK; evidence written to {evidence_file}")
PY
