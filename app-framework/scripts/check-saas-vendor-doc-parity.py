#!/usr/bin/env python3
"""Validate SaaS vendor docs are transcribed from provider metadata constants."""

from __future__ import annotations

import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class ProviderDocContract:
    key: str
    crate_dir: str
    doc_path: str
    metadata_path: str
    lib_path: str
    required_consts: tuple[str, ...]
    metadata_struct: str
    metadata_function: str


PROVIDERS: tuple[ProviderDocContract, ...] = (
    ProviderDocContract(
        key="salesforce",
        crate_dir="appfw_provider_salesforce",
        doc_path="appfw_provider_salesforce/docs/vendor-contract.md",
        metadata_path="appfw_provider_salesforce/src/metadata.rs",
        lib_path="appfw_provider_salesforce/src/lib.rs",
        required_consts=(
            "SALESFORCE_RELEASE_PIN",
            "SALESFORCE_API_VERSION_PIN",
            "SALESFORCE_DOC_VERSION_PIN",
            "SALESFORCE_VENDOR_KEY",
            "SALESFORCE_API_SPEC_PACKAGE_ID",
            "SALESFORCE_INFORMATION_PACKAGE_VERSION",
        ),
        metadata_struct="SalesforceApiSnapshotMetadata",
        metadata_function="salesforce_api_snapshot_metadata",
    ),
    ProviderDocContract(
        key="workday",
        crate_dir="appfw_provider_workday",
        doc_path="appfw_provider_workday/docs/vendor-contract.md",
        metadata_path="appfw_provider_workday/src/metadata.rs",
        lib_path="appfw_provider_workday/src/lib.rs",
        required_consts=(
            "WORKDAY_RELEASE_PIN",
            "WORKDAY_API_VERSION_PIN",
            "WORKDAY_VENDOR_KEY",
            "WORKDAY_API_SPEC_PACKAGE_ID",
            "WORKDAY_INFORMATION_PACKAGE_VERSION",
            "WORKDAY_WWS_SUMMARY_PATH",
        ),
        metadata_struct="WorkdayApiSnapshotMetadata",
        metadata_function="workday_api_snapshot_metadata",
    ),
    ProviderDocContract(
        key="anaplan",
        crate_dir="appfw_provider_anaplan",
        doc_path="appfw_provider_anaplan/docs/vendor-contract.md",
        metadata_path="appfw_provider_anaplan/src/metadata.rs",
        lib_path="appfw_provider_anaplan/src/lib.rs",
        required_consts=(
            "ANAPLAN_RELEASE_PIN",
            "ANAPLAN_API_VERSION_PIN",
            "ANAPLAN_VENDOR_KEY",
            "ANAPLAN_API_SPEC_PACKAGE_ID",
            "ANAPLAN_INFORMATION_PACKAGE_VERSION",
            "ANAPLAN_INTEGRATION_API_SUMMARY_PATH",
        ),
        metadata_struct="AnaplanApiSnapshotMetadata",
        metadata_function="anaplan_api_snapshot_metadata",
    ),
    ProviderDocContract(
        key="oracle_financials",
        crate_dir="appfw_provider_oracle_financials",
        doc_path="appfw_provider_oracle_financials/docs/vendor-contract.md",
        metadata_path="appfw_provider_oracle_financials/src/metadata.rs",
        lib_path="appfw_provider_oracle_financials/src/lib.rs",
        required_consts=(
            "ORACLE_FINANCIALS_VENDOR_KEY",
            "ORACLE_FINANCIALS_API_SPEC_PACKAGE_ID",
            "ORACLE_FINANCIALS_INFORMATION_PACKAGE_VERSION",
            "ORACLE_FINANCIALS_OPENAPI_SUMMARY_PATH",
        ),
        metadata_struct="OracleFinancialsApiSnapshotMetadata",
        metadata_function="oracle_financials_api_snapshot_metadata",
    ),
    ProviderDocContract(
        key="servicenow",
        crate_dir="appfw_provider_servicenow",
        doc_path="appfw_provider_servicenow/docs/vendor-contract.md",
        metadata_path="appfw_provider_servicenow/src/metadata.rs",
        lib_path="appfw_provider_servicenow/src/lib.rs",
        required_consts=(
            "SERVICENOW_VENDOR_KEY",
            "SERVICENOW_API_SPEC_PACKAGE_ID",
            "SERVICENOW_INFORMATION_PACKAGE_VERSION",
            "SERVICENOW_PUBLIC_DOCS_SUMMARY_PATH",
        ),
        metadata_struct="ServiceNowApiSnapshotMetadata",
        metadata_function="servicenow_api_snapshot_metadata",
    ),
    ProviderDocContract(
        key="icims",
        crate_dir="appfw_provider_icims",
        doc_path="appfw_provider_icims/docs/vendor-contract.md",
        metadata_path="appfw_provider_icims/src/metadata.rs",
        lib_path="appfw_provider_icims/src/lib.rs",
        required_consts=(
            "ICIMS_VENDOR_KEY",
            "ICIMS_API_SPEC_PACKAGE_ID",
            "ICIMS_INFORMATION_PACKAGE_VERSION",
            "ICIMS_PUBLIC_DOCS_SUMMARY_PATH",
        ),
        metadata_struct="IcimsApiSnapshotMetadata",
        metadata_function="icims_api_snapshot_metadata",
    ),
)

CONST_RE = re.compile(
    r"pub\s+const\s+([A-Z0-9_]+)\s*:\s*&str\s*=\s*\"((?:[^\"\\]|\\.)*)\"\s*;",
    re.DOTALL,
)


def read_text(path: Path) -> str | None:
    try:
        return path.read_text(encoding="utf-8")
    except FileNotFoundError:
        return None


def rust_string_value(value: str) -> str:
    return bytes(value, "utf-8").decode("unicode_escape")


def extract_string_consts(metadata_text: str) -> dict[str, str]:
    return {name: rust_string_value(value) for name, value in CONST_RE.findall(metadata_text)}


def validate_provider(repo_root: Path, contract: ProviderDocContract) -> dict[str, object]:
    doc_abs = repo_root / contract.doc_path
    metadata_abs = repo_root / contract.metadata_path
    lib_abs = repo_root / contract.lib_path

    doc_text = read_text(doc_abs)
    metadata_text = read_text(metadata_abs)
    lib_text = read_text(lib_abs)

    missing_files = [
        path
        for path, text in (
            (contract.doc_path, doc_text),
            (contract.metadata_path, metadata_text),
            (contract.lib_path, lib_text),
        )
        if text is None
    ]

    consts: dict[str, str] = extract_string_consts(metadata_text or "")
    missing_consts = [
        const for const in contract.required_consts if const not in consts
    ]
    missing_doc_values = [
        {"const": const, "value": consts[const]}
        for const in contract.required_consts
        if const in consts and doc_text is not None and consts[const] not in doc_text
    ]

    metadata_tokens = []
    if metadata_text is not None:
        for token in (
            contract.metadata_struct,
            contract.metadata_function,
            "source_refs",
        ):
            if token not in metadata_text:
                metadata_tokens.append(token)

    lib_tokens = []
    if lib_text is not None:
        for token in ("pub mod metadata;", contract.metadata_struct, contract.metadata_function):
            if token not in lib_text:
                lib_tokens.append(token)

    doc_tokens = []
    if doc_text is not None:
        for token in (
            "appfw:vendor-contract-template v1",
            "saas-vendor-doc-parity",
            "Version Link",
        ):
            if token not in doc_text:
                doc_tokens.append(token)

    ok = not (
        missing_files
        or missing_consts
        or missing_doc_values
        or metadata_tokens
        or lib_tokens
        or doc_tokens
    )

    return {
        "provider": contract.key,
        "ok": ok,
        "doc": contract.doc_path,
        "metadata": contract.metadata_path,
        "required_consts": list(contract.required_consts),
        "missing_files": missing_files,
        "missing_consts": missing_consts,
        "missing_doc_values": missing_doc_values,
        "missing_metadata_tokens": metadata_tokens,
        "missing_lib_tokens": lib_tokens,
        "missing_doc_tokens": doc_tokens,
    }


def main() -> int:
    repo_root = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else Path.cwd()
    artifact = repo_root / "target/appfw/saas-vendor-doc-parity.json"
    artifact.parent.mkdir(parents=True, exist_ok=True)

    provider_results = [validate_provider(repo_root, provider) for provider in PROVIDERS]
    ok = all(result["ok"] for result in provider_results)
    report = {
        "command": "saas-vendor-doc-parity",
        "ok": ok,
        "schema_version": "appfw.saas_vendor_doc_parity.v1",
        "provider_count": len(provider_results),
        "providers": provider_results,
        "artifact": artifact.relative_to(repo_root).as_posix(),
    }

    artifact.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
