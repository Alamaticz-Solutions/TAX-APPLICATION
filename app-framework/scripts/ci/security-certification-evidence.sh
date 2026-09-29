#!/usr/bin/env bash
#
# security-certification-evidence.sh - retain live security certification JSON.
#
# This command is intentionally narrow. It proves the release-blocking GraphQL
# introspection authorization regressions and verifies that retained provider
# certification logs include the tenant/IDOR/access-denial negative contracts.
#
# Usage:
#   bash scripts/ci/security-certification-evidence.sh
#   bash scripts/ci/security-certification-evidence.sh --json
#
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
cd "$repo_root"

report_dir="${APPFW_RELEASE_ARTIFACT_DIR:-target/appfw}"
evidence_file="${APPFW_SECURITY_CERTIFICATION_OUTPUT:-$report_dir/security-certification.json}"
provider_parity_file="${APPFW_PROVIDER_PARITY_REPORT:-$report_dir/provider-parity.json}"
runtime_log="${APPFW_SECURITY_CERTIFICATION_RUNTIME_LOG:-$report_dir/security-certification-introspection.log}"
mode="${APPFW_SECURITY_CERTIFICATION_MODE:-strict}"
json=0
run_runtime_tests=1

usage() {
  cat <<'USAGE'
Usage:
  bash scripts/ci/security-certification-evidence.sh [--json] [--strict] [--no-runtime-tests]

Options:
  --json              Print the written JSON evidence to stdout.
  --strict            Require retained provider-parity evidence and provider logs.
  --no-runtime-tests  Do not execute focused runtime introspection tests; validate
                      source test coverage only. Not release-ready evidence.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --json)
      json=1
      ;;
    --strict)
      mode="strict"
      ;;
    --no-runtime-tests)
      run_runtime_tests=0
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "security-certification-evidence: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift || true
done

case "$mode" in
  strict)
    ;;
  *)
    echo "security-certification-evidence: unsupported mode: $mode" >&2
    exit 2
    ;;
esac

mkdir -p "$report_dir"

runtime_status=0
if [[ "$run_runtime_tests" == "1" ]]; then
  set +e
  cargo test --locked -p appfw-runtime graphql_introspection -- --show-output \
    >"$runtime_log" 2>&1
  runtime_status=$?
  set -e
else
  runtime_status=99
  : >"$runtime_log"
fi

python3 - "$repo_root" "$report_dir" "$evidence_file" "$provider_parity_file" "$runtime_log" "$runtime_status" "$run_runtime_tests" <<'PY'
import datetime as dt
import hashlib
import json
import re
import sys
from pathlib import Path

repo_root = Path(sys.argv[1])
report_dir = Path(sys.argv[2])
evidence_file = Path(sys.argv[3])
provider_parity_file = Path(sys.argv[4])
runtime_log = Path(sys.argv[5])
runtime_status = int(sys.argv[6])
run_runtime_tests = sys.argv[7] == "1"

checks = []
artifacts = []
failures = []

EXPECTED_PROVIDERS = ["postgres", "mongo", "mssql", "snowflake"]
PROVIDER_PARITY_MAX_AGE_HOURS = 24
PROVIDER_PARITY_MAX_AGE = dt.timedelta(hours=PROVIDER_PARITY_MAX_AGE_HOURS)
INTROSPECTION_TESTS = [
    {
        "case": "no_auth_disabled_or_missing_user",
        "test": "graphql_introspection_rejects_no_auth_when_disabled_or_missing_user",
        "expectation": "missing auth and disabled introspection are rejected",
    },
    {
        "case": "fake_bearer_or_unprivileged_user",
        "test": "graphql_introspection_rejects_fake_bearer_or_unprivileged_user",
        "expectation": "placeholder bearer and non-admin principals are rejected",
    },
    {
        "case": "admin_or_developer_scope",
        "test": "graphql_introspection_allows_admin_or_developer_scope",
        "expectation": "admin role or developer scope can introspect when enabled",
    },
]
SECURITY_CONTRACTS = [
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


def utc_now():
    return (
        dt.datetime.now(dt.timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z")
    )


def rel(path):
    try:
        return path.resolve().relative_to(repo_root.resolve()).as_posix()
    except Exception:
        return path.as_posix()


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact(name, path, required=True):
    path = resolve_path(path)
    entry = {
        "name": name,
        "path": rel(path),
        "required": required,
        "present": path.is_file(),
    }
    if path.is_file():
        entry["sha256"] = sha256(path)
        entry["bytes"] = path.stat().st_size
    artifacts.append(entry)
    return entry


def resolve_path(path):
    path = Path(path)
    if path.is_absolute():
        return path
    return repo_root / path


def path_is_within_directory(path, directory):
    try:
        path.resolve(strict=False).relative_to(directory.resolve(strict=False))
    except ValueError:
        return False
    return True


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


def record(name, ok, artifact_path=None, detail=None):
    item = {"name": name, "ok": bool(ok)}
    if artifact_path is not None:
        item["artifact"] = rel(resolve_path(artifact_path))
    if detail is not None:
        item["detail"] = detail
    checks.append(item)
    if not ok:
        failures.append(name if detail is None else f"{name}: {detail}")


def load_json(path):
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def test_passed(log_text, test_name):
    pattern = rf"test\s+.*{re.escape(test_name)}\s+\.\.\.\s+ok"
    return re.search(pattern, log_text) is not None


def contract_passed(log_text, contract):
    return f"test {contract} ... ok" in log_text


def provider_log_path(provider):
    log = provider.get("log")
    if isinstance(log, str) and log:
        return resolve_path(log)
    provider_name = provider.get("provider", "unknown")
    return repo_root / "api_tests" / "target" / f"provider-test-{provider_name}.log"


def provider_area(provider, area_name):
    for area in provider.get("areas", []):
        if isinstance(area, dict) and area.get("area") == area_name:
            return area
    return None


def append_unique(items, value):
    if value not in items:
        items.append(value)


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


def summarize_release_blockers(
    failures,
    provider_report,
    introspection_cases,
    providers_evidence,
):
    summaries = []

    failed_cases = [
        case.get("case")
        for case in introspection_cases
        if isinstance(case, dict) and case.get("ok") is not True
    ]
    if failed_cases:
        append_unique(
            summaries,
            "GraphQL introspection authorization evidence is not green "
            f"(cases: {compact_values(failed_cases)})",
        )

    if not isinstance(provider_report, dict):
        append_unique(summaries, "provider parity evidence is missing or malformed")
    else:
        parity_detail_parts = []
        providers_value = provider_report.get("providers")
        if provider_report.get("ok") is not True:
            parity_detail_parts.append("ok=false")
        if not isinstance(providers_value, list):
            parity_detail_parts.append("providers block missing")
        else:
            provider_entries = [
                provider for provider in providers_value if isinstance(provider, dict)
            ]
            seen_providers = [
                provider.get("provider")
                for provider in provider_entries
                if isinstance(provider.get("provider"), str)
            ]
            missing_providers = sorted(set(EXPECTED_PROVIDERS) - set(seen_providers))
            unexpected_providers = sorted(set(seen_providers) - set(EXPECTED_PROVIDERS))
            failing_providers = [
                provider.get("provider")
                for provider in provider_entries
                if provider.get("provider") in EXPECTED_PROVIDERS
                and provider.get("ok") is not True
            ]
            if missing_providers:
                parity_detail_parts.append(
                    f"missing providers: {compact_values(missing_providers)}"
                )
            if unexpected_providers:
                parity_detail_parts.append(
                    f"unexpected providers: {compact_values(unexpected_providers)}"
                )
            if failing_providers:
                parity_detail_parts.append(
                    f"failing providers: {compact_values(failing_providers)}"
                )
        if parity_detail_parts:
            append_unique(
                summaries,
                "provider parity is not green "
                f"({'; '.join(parity_detail_parts)})",
            )

    failing_contract_providers = []
    failing_contracts = []
    for provider in providers_evidence:
        if not isinstance(provider, dict):
            continue
        provider_name = provider.get("provider")
        if not isinstance(provider_name, str) or not provider_name:
            provider_name = "<unknown>"
        provider_has_failure = provider.get("ok") is not True
        for contract in provider.get("contracts", []):
            if not isinstance(contract, dict):
                continue
            if contract.get("ok") is not True:
                provider_has_failure = True
                append_unique(failing_contracts, contract_label(contract))
        if provider_has_failure:
            append_unique(failing_contract_providers, provider_name)
    if failing_contract_providers:
        append_unique(
            summaries,
            "provider security contracts are not green "
            f"(providers: {compact_values(failing_contract_providers)}; "
            f"contracts: {compact_values(failing_contracts)})",
        )

    if not summaries and failures:
        append_unique(
            summaries,
            "security certification retained checks failed "
            f"({compact_values(failures)})",
        )

    return summaries


routing_source = repo_root / "appfw_runtime" / "src" / "routing.rs"
artifact("runtime-routing-source", routing_source)
routing_text = routing_source.read_text(encoding="utf-8") if routing_source.is_file() else ""

runtime_artifact = artifact("graphql-introspection-runtime-test-log", runtime_log)
runtime_log_text = runtime_log.read_text(encoding="utf-8", errors="replace") if runtime_log.is_file() else ""
if run_runtime_tests:
    record(
        "graphql introspection runtime test command passes",
        runtime_status == 0,
        runtime_log,
        None if runtime_status == 0 else f"cargo test exited {runtime_status}",
    )
else:
    record(
        "graphql introspection runtime test command passes",
        False,
        runtime_log,
        "--no-runtime-tests is source coverage only and is not release-ready",
    )

introspection_cases = []
for case in INTROSPECTION_TESTS:
    source_present = case["test"] in routing_text
    runtime_passed = run_runtime_tests and test_passed(runtime_log_text, case["test"])
    ok = source_present and runtime_passed
    introspection_cases.append(
        {
            "case": case["case"],
            "test": case["test"],
            "expectation": case["expectation"],
            "source_present": source_present,
            "runtime_result": "passed" if runtime_passed else "not-passed",
            "ok": ok,
        }
    )
    record(
        f"graphql introspection {case['case']}",
        ok,
        runtime_log if run_runtime_tests else routing_source,
        None if ok else "expected source test and focused runtime pass",
    )

provider_parity_artifact = artifact("provider-parity", provider_parity_file)
provider_report = None
if provider_parity_artifact["present"]:
    try:
        provider_report = load_json(resolve_path(provider_parity_file))
        record("provider parity JSON parses", True, provider_parity_file)
    except Exception as exc:
        record("provider parity JSON parses", False, provider_parity_file, str(exc))
else:
    record("provider parity artifact is present", False, provider_parity_file)

providers_evidence = []
if isinstance(provider_report, dict):
    record(
        "provider parity command is stable",
        provider_report.get("command") == "provider-test",
        provider_parity_file,
    )
    record("provider parity ok flag is true", provider_report.get("ok") is True, provider_parity_file)
    now = dt.datetime.now(dt.timezone.utc)
    provider_generated_at = parse_timestamp_with_timezone(provider_report.get("generated_at_utc"))
    record(
        "provider parity generated timestamp is valid",
        provider_generated_at is not None,
        provider_parity_file,
        "generated_at_utc must be ISO-8601 with timezone",
    )
    if provider_generated_at is not None:
        record(
            "provider parity generated timestamp is not in the future",
            provider_generated_at <= now + dt.timedelta(minutes=5),
            provider_parity_file,
            provider_generated_at.isoformat().replace("+00:00", "Z"),
        )
        record(
            "provider parity generated timestamp is recent",
            provider_generated_at >= now - PROVIDER_PARITY_MAX_AGE,
            provider_parity_file,
            (
                f"generated_at_utc {provider_generated_at.isoformat().replace('+00:00', 'Z')} "
                f"is older than {PROVIDER_PARITY_MAX_AGE_HOURS} hours"
            ),
        )
    providers_value = provider_report.get("providers")
    record("provider parity providers is an array", isinstance(providers_value, list), provider_parity_file)
    provider_entries = [provider for provider in providers_value if isinstance(provider, dict)] if isinstance(providers_value, list) else []
    seen_providers = [
        provider.get("provider") for provider in provider_entries if provider.get("provider")
    ]
    duplicate_providers = sorted(
        provider
        for provider in set(seen_providers)
        if isinstance(provider, str) and seen_providers.count(provider) > 1
    )
    missing_providers = sorted(set(EXPECTED_PROVIDERS) - set(seen_providers))
    unexpected_providers = sorted(set(seen_providers) - set(EXPECTED_PROVIDERS))
    record(
        "provider parity includes expected providers",
        not missing_providers and not unexpected_providers,
        provider_parity_file,
        None
        if not missing_providers and not unexpected_providers
        else f"missing {missing_providers}; unexpected {unexpected_providers}; seen {seen_providers}",
    )
    record(
        "provider parity providers name each provider once",
        not duplicate_providers,
        provider_parity_file,
        f"duplicates {duplicate_providers}",
    )

    for provider in provider_entries:
        provider_name = provider.get("provider", "<unknown>")
        record(
            f"{provider_name} provider parity names release-certified provider",
            provider_name in EXPECTED_PROVIDERS,
            provider_parity_file,
            f"actual {provider_name!r}",
        )
        provider_ok = provider.get("ok") is True
        record(f"{provider_name} provider parity ok", provider_ok, provider_parity_file)
        record(
            f"{provider_name} provider parity run is full mode",
            provider.get("mode") == "full",
            provider_parity_file,
            f"actual {provider.get('mode')!r}",
        )

        log_path = provider_log_path(provider)
        log_artifact = artifact(f"provider-log:{provider_name}", log_path)
        log_text = log_path.read_text(encoding="utf-8", errors="replace") if log_path.is_file() else ""
        record(
            f"{provider_name} provider certification log retained",
            log_path.is_file(),
            log_path,
        )
        record(
            f"{provider_name} provider certification log is retained in report directory",
            log_path.is_file() and path_is_within_directory(log_path, report_dir),
            log_path,
            (
                f"actual {log_path.as_posix()}, report_dir {report_dir.resolve(strict=False).as_posix()}"
            ),
        )
        record(
            f"{provider_name} provider certification log is non-empty",
            log_path.is_file() and log_path.stat().st_size > 0,
            log_path,
        )

        contract_results = []
        for contract in SECURITY_CONTRACTS:
            passed = contract_passed(log_text, contract["contract"])
            area_ok = True
            area = None
            if contract["area"] is not None:
                area = provider_area(provider, contract["area"])
                area_ok = (
                    isinstance(area, dict)
                    and area.get("status") == "live-certified"
                    and area.get("live_result") == "passed"
                    and area.get("ok") is True
                )
            ok = passed and area_ok
            contract_results.append(
                {
                    "name": contract["name"],
                    "contract": contract["contract"],
                    "expectation": contract["expectation"],
                    "result": "passed" if passed else "not-passed",
                    "area": contract["area"],
                    "area_result": None
                    if contract["area"] is None
                    else (area.get("live_result") if isinstance(area, dict) else "missing"),
                    "ok": ok,
                }
            )
            record(
                f"{provider_name} {contract['name']} live security contract",
                ok,
                log_path,
                None if ok else "expected provider log pass and live-certified area pass when mapped",
            )

        providers_evidence.append(
            {
                "provider": provider_name,
                "ok": provider_ok and all(item["ok"] for item in contract_results),
                "log": log_artifact["path"],
                "contracts": contract_results,
            }
        )

release_blockers = summarize_release_blockers(
    failures,
    provider_report,
    introspection_cases,
    providers_evidence,
)

evidence = {
    "command": "security-certification-evidence",
    "ok": not failures,
    "release_ready": not failures and run_runtime_tests,
    "generated_at_utc": utc_now(),
    "checks": checks,
    "artifacts": artifacts,
    "introspection_auth": {
        "evidence_type": "focused runtime unit regression",
        "source": rel(routing_source),
        "runtime_log": runtime_artifact["path"],
        "cases": introspection_cases,
    },
    "live_security_contracts": {
        "provider_parity": provider_parity_artifact["path"],
        "expected_providers": EXPECTED_PROVIDERS,
        "expected_contracts": SECURITY_CONTRACTS,
        "providers": providers_evidence,
    },
    "failure_count": len(failures),
    "failures": failures,
    "release_blockers": release_blockers,
    "failure_summary": {"root_causes": release_blockers},
}

evidence_file.write_text(
    json.dumps(evidence, indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
)

if failures:
    print("security-certification-evidence: FAILED", file=sys.stderr)
    for blocker in release_blockers:
        print(f"  - {blocker}", file=sys.stderr)
    print(f"security-certification-evidence: evidence written to {evidence_file}", file=sys.stderr)
    raise SystemExit(1)

print(f"security-certification-evidence: OK; evidence written to {evidence_file}")
PY

if [[ "$json" == "1" ]]; then
  cat "$evidence_file"
fi
