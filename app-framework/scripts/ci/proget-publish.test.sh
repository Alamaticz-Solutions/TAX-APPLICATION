#!/usr/bin/env bash
#
# Hermetic failure-path coverage for scripts/ci/proget-publish.sh.
# No network request, credential, or publication is permitted in this suite.
#
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"

export APPFW_PROGET_LIBRARY_ONLY=true
# shellcheck source=scripts/ci/proget-publish.sh
source "$script_dir/proget-publish.sh"
unset APPFW_PROGET_LIBRARY_ONLY

test_root="$(mktemp -d "${TMPDIR:-/tmp}/appfw-proget-publish-test.XXXXXX")"
trap 'rm -rf "$test_root"' EXIT

fail() {
  echo "proget-publish.test: $*" >&2
  exit 1
}

# Workspace members and their declared dependencies only. --no-deps avoids
# downloading crates.io; --offline keeps the check hermetic when the CI
# cargo-registry cache is cold. Do not use `cargo metadata --offline` without
# --no-deps: that requires a warm registry and previously mislabeled the
# failure as a missing appfw-test → appfw-codegen dependency.
workspace_metadata_file=""

load_workspace_cargo_metadata() {
  if [[ -n "$workspace_metadata_file" && -s "$workspace_metadata_file" ]]; then
    return 0
  fi
  workspace_metadata_file="$test_root/workspace-metadata.json"
  local err_file="$test_root/workspace-metadata.err"
  if ! cargo metadata \
    --format-version 1 \
    --locked \
    --offline \
    --no-deps \
    --manifest-path "$repo_root/Cargo.toml" \
    >"$workspace_metadata_file" \
    2>"$err_file"; then
    echo "proget-publish.test: cargo metadata --format-version 1 --locked --offline --no-deps failed:" >&2
    cat "$err_file" >&2
    fail "workspace cargo metadata failed (registry/cache or cargo invocation); this is not a missing appfw-test → appfw-codegen dependency"
  fi
}

expect_failure() {
  local label="$1"
  shift
  if "$@"; then
    fail "$label unexpectedly succeeded"
  fi
}

reset_fixture() {
  local name="$1"
  package_dir="$test_root/$name/package"
  report_dir="$test_root/$name/report"
  work_dir="$test_root/$name/work"
  manifest_file="$package_dir/app-framework-proget-manifest.json"
  package_version="1.2.3"
  upack_name="app-framework-toolchain"
  upack_feed="pds-app-framework"
  upack_group="pds/app-framework"
  cargo_registry="pds-app-framework-crates"
  PROGET_SERVER="https://proget.invalid"
  export BITBUCKET_BUILD_NUMBER="123"
  export BITBUCKET_PIPELINE_UUID="{test-pipeline}"
  dry_run=true
  skipped_crates=()
  published_crates=()
  mkdir -p "$package_dir" "$report_dir"
  printf '{"publish_order":[]}\n' >"$package_dir/app-framework-crates-publish-plan-${package_version}.json"
}

write_release_evidence() {
  local command="${1:-bitbucket-release-gate}"
  local commit="${2:-test-commit}"
  release_evidence_file="$report_dir/bitbucket-release-gate.json"
  python3 - "$release_evidence_file" "$command" "$commit" <<'PY'
import json
import sys

with open(sys.argv[1], "w", encoding="utf-8") as handle:
    json.dump(
        {
            "command": sys.argv[2],
            "ok": True,
            "release_ready": True,
            "focused_evidence": False,
            "release_authority": "strict-managed-release",
            "release_authority_boundary": {
                "strict_release_gate_satisfied": True,
                "managed_release_requirements_satisfied": True,
            },
            "release_requirements": {
                "security_assurance_decision_required": True,
                "production_attestations_required": True,
                "ops_certification_required": True,
                "live_ops_evidence_required": True,
                "pds_baseline_evidence_required": True,
                "performance_evidence_required": True,
                "release_identity_required": True,
            },
            "ci": {
                "bitbucket_commit": sys.argv[3],
                "bitbucket_tag": "v1.2.3",
                "bitbucket_build_number": "123",
                "bitbucket_pipeline_uuid": "{test-pipeline}",
            },
        },
        handle,
    )
    handle.write("\n")
PY
}

test_direct_library_bypass_rejected() {
  reset_fixture "library-bypass"
  expect_failure \
    "direct test-only library bypass" \
    env \
    APPFW_PROGET_LIBRARY_ONLY=true \
    APPFW_RELEASE_ARTIFACT_DIR="$report_dir" \
    bash "$script_dir/proget-publish.sh"
}

test_release_evidence_command_rejected() {
  reset_fixture "release-command"
  write_release_evidence "untrusted-command"
  source_commit="test-commit"
  release_tag="v1.2.3"
  expect_failure "untrusted release evidence command" stage_release_evidence
}

test_release_evidence_identity_rejected() {
  reset_fixture "release-identity"
  write_release_evidence "bitbucket-release-gate" "other-commit"
  source_commit="test-commit"
  release_tag="v1.2.3"
  expect_failure "mismatched release evidence identity" stage_release_evidence
}

test_release_evidence_valid() {
  reset_fixture "release-valid"
  write_release_evidence
  source_commit="test-commit"
  release_tag="v1.2.3"
  stage_release_evidence
}

test_immutability_probe_failure() {
  reset_fixture "probe-failure"
  dry_run=false
  proget_curl() {
    return 22
  }
  expect_failure "failed version probe" stage_immutability
}

test_immutability_malformed_response() {
  reset_fixture "probe-malformed"
  dry_run=false
  proget_curl() {
    printf 'not-json'
  }
  expect_failure "malformed version response" stage_immutability
}

test_immutability_existing_version() {
  reset_fixture "probe-existing"
  dry_run=false
  proget_curl() {
    printf '[{"version":"1.2.3"}]'
  }
  expect_failure "existing immutable version" stage_immutability
}

test_immutability_absent_version() {
  reset_fixture "probe-absent"
  dry_run=false
  proget_curl() {
    printf '[]'
  }
  stage_immutability
}

test_manifest_bound_source_archive() {
  reset_fixture "manifest-source"
  local exact_root="$test_root/manifest-source/exact/app-framework-source-1.2.3"
  local stale_root="$test_root/manifest-source/stale/app-framework-source-0.0.0"
  mkdir -p "$exact_root" "$stale_root"
  printf 'exact\n' >"$exact_root/source-marker.txt"
  printf 'stale\n' >"$stale_root/source-marker.txt"
  tar -czf "$package_dir/app-framework-source-1.2.3.tar.gz" -C "$(dirname "$exact_root")" "$(basename "$exact_root")"
  tar -czf "$package_dir/app-framework-source-0.0.0.tar.gz" -C "$(dirname "$stale_root")" "$(basename "$stale_root")"
  python3 - "$manifest_file" "$package_dir/app-framework-source-1.2.3.tar.gz" <<'PY'
import json
import sys

with open(sys.argv[1], "w", encoding="utf-8") as handle:
    json.dump(
        {
            "artifacts": [
                {
                    "name": "app-framework-source",
                    "path": sys.argv[2],
                }
            ]
        },
        handle,
    )
    handle.write("\n")
PY
  stage_publish_crates
  [[ "$(cat "$work_dir/app-framework-source-1.2.3/source-marker.txt")" == "exact" ]] ||
    fail "publisher did not extract the manifest-bound source archive"
  [[ ! -e "$work_dir/app-framework-source-0.0.0" ]] ||
    fail "publisher extracted an unmanifested stale source archive"
}

write_minimal_manifest() {
  local artifact="$package_dir/toolchain.tar.gz"
  printf 'toolchain\n' >"$artifact"
  python3 - "$manifest_file" "$artifact" <<'PY'
import json
import sys

with open(sys.argv[1], "w", encoding="utf-8") as handle:
    json.dump(
        {
            "platform": "test-platform",
            "artifacts": [
                {
                    "name": "app-framework-toolchain",
                    "path": sys.argv[2],
                }
            ],
        },
        handle,
    )
    handle.write("\n")
PY
}

test_upack_requires_sbom() {
  reset_fixture "missing-sbom"
  write_minimal_manifest
  expect_failure "missing mandatory SBOM" stage_publish_upack
}

test_upack_includes_sbom() {
  reset_fixture "present-sbom"
  write_minimal_manifest
  printf '{"bomFormat":"CycloneDX","specVersion":"1.5","components":[]}\n' \
    >"$report_dir/sbom-rust-workspace.cdx.json"
  stage_publish_upack
  python3 - "$upack_file" <<'PY'
import sys
import zipfile

with zipfile.ZipFile(sys.argv[1]) as archive:
    names = set(archive.namelist())
if "package/sbom-rust-workspace.cdx.json" not in names:
    raise SystemExit("SBOM missing from generated upack")
PY
}

test_official_schema_route_converter_is_shared() {
  local converter="$repo_root/app_gen/src/schema_route.rs"
  local client="$repo_root/appfw_test/src/harness/graphql_client.rs"
  local introspect="$repo_root/app_gen/src/bin/appfw_introspect.rs"
  local test_manifest="$repo_root/appfw_test/Cargo.toml"
  local publish="$repo_root/scripts/ci/proget-publish.sh"
  grep -q 'pub fn schema_route_segment' "$converter" \
    || fail "official converter missing from app_gen/src/schema_route.rs"
  grep -q 'is_kebab_case' "$converter" \
    || fail "official converter does not call Inflector is_kebab_case"
  grep -q 'to_kebab_case' "$converter" \
    || fail "official converter does not call Inflector to_kebab_case"
  grep -q 'use appfw_codegen::schema_route_segment' "$client" \
    || fail "appfw-test does not resolve appfw_codegen::schema_route_segment"
  grep -q 'use appfw_codegen::schema_route_segment' "$introspect" \
    || fail "appfw_introspect does not resolve appfw_codegen::schema_route_segment"
  grep -q 'package = "appfw-codegen"' "$test_manifest" \
    || fail "appfw-test Cargo.toml does not depend on appfw-codegen"
  if grep -n '#\[path' "$converter" "$client" "$introspect"; then
    fail "#[path] include must not remain for the schema route converter"
  fi
  grep -q 'appfw_codegen::schema_route_segment' "$publish" \
    || fail "publish script does not name the official crate::function"
  if grep -q "replace('_', '-')" "$converter" "$client" "$introspect" "$publish"; then
    fail "naive underscore replace must not reappear beside the official converter"
  fi
  local packaged packaged_err
  packaged_err="$test_root/cargo-package-list.err"
  if ! packaged="$(cargo package --list -p appfw-codegen --locked --allow-dirty --manifest-path "$repo_root/Cargo.toml" 2>"$packaged_err")"; then
    echo "proget-publish.test: cargo package --list -p appfw-codegen failed:" >&2
    cat "$packaged_err" >&2
    fail "cargo package --list -p appfw-codegen failed (packaging inventory, not a missing dependency)"
  fi
  printf '%s\n' "$packaged" | grep -qx 'src/schema_route.rs' \
    || fail "isolated appfw-codegen package does not include src/schema_route.rs"
  if printf '%s\n' "$packaged" | grep -q 'appfw_test/src/harness/graphql_client.rs'; then
    fail "isolated appfw-codegen package must not include appfw-test graphql_client.rs"
  fi
  load_workspace_cargo_metadata
  python3 - "$workspace_metadata_file" <<'PY' || fail "appfw-test → appfw-codegen workspace-graph check failed (see python error above)"
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    metadata = json.load(handle)
packages = {package["name"]: package for package in metadata["packages"]}
if "appfw-test" not in packages:
    raise SystemExit("appfw-test missing from workspace cargo metadata")
if "appfw-codegen" not in packages:
    raise SystemExit("appfw-codegen missing from workspace cargo metadata")
dep_names = {dep["name"] for dep in packages["appfw-test"]["dependencies"]}
if "appfw-codegen" not in dep_names:
    raise SystemExit("appfw-test metadata dependencies do not include appfw-codegen")
PY
}

test_publish_plan_is_closed_topological_graph() {
  load_workspace_cargo_metadata
  python3 - "$repo_root" "$workspace_metadata_file" <<'PY' || fail "publish plan graph-closure check failed (see python error above)"
import json
import re
import sys
from pathlib import Path

repo_root = Path(sys.argv[1])
appfw = (repo_root / "scripts/appfw").read_text(encoding="utf-8")
match = re.search(
    r"appfw_crate_publish_order\(\) \{\n  cat <<'EOF'\n(.*?)\nEOF\n\}",
    appfw,
    re.S,
)
if not match:
    raise SystemExit("could not parse appfw_crate_publish_order from scripts/appfw")
publish_order = [line.strip() for line in match.group(1).splitlines() if line.strip()]
index = {name: i for i, name in enumerate(publish_order)}
for earlier, later in (("appfw-cli", "appfw-codegen"), ("appfw-codegen", "appfw-test")):
    if earlier not in index or later not in index:
        raise SystemExit(f"publish plan missing {earlier} or {later}")
    if index[earlier] >= index[later]:
        raise SystemExit(
            f"{earlier} must precede {later} in the authoritative publish plan"
        )
for earlier, later in (
    ("appfw-mssql-auth", "appfw-provider-mssql"),
    ("appfw-mssql-auth", "appfw-codegen"),
):
    if earlier not in index or later not in index:
        raise SystemExit(f"publish plan missing {earlier} or {later}")
    if index[earlier] >= index[later]:
        raise SystemExit(
            f"{earlier} must precede {later} in the authoritative publish plan"
        )

docs = (repo_root / "docs/release/proget-distribution.md").read_text(encoding="utf-8")
docs_match = re.search(
    r"```text\n((?:appfw-[^\n]+\n)+)```",
    docs,
)
if not docs_match:
    raise SystemExit("could not parse mirrored crate list from proget-distribution.md")
docs_order = [line.strip() for line in docs_match.group(1).splitlines() if line.strip()]
if docs_order != publish_order:
    raise SystemExit(
        "docs/release/proget-distribution.md crate list does not mirror "
        "appfw_crate_publish_order"
    )

with open(sys.argv[2], encoding="utf-8") as handle:
    metadata = json.load(handle)
workspace_ids = set(metadata["workspace_members"])
workspace_packages = {
    package["name"]: package
    for package in metadata["packages"]
    if package["id"] in workspace_ids
}

def is_publishable(package):
    # cargo metadata: [] means publish = false; None/non-empty is publishable.
    return package.get("publish") != []

# Intentional external baselines must be version-bound. Empty: no exceptions.
PUBLISH_GRAPH_EXTERNAL_BASELINE = {}

missing = []
order_violations = []
for consumer in publish_order:
    package = workspace_packages.get(consumer)
    if package is None:
        raise SystemExit(f"{consumer} is in the publish plan but not a workspace crate")
    if not is_publishable(package):
        raise SystemExit(f"{consumer} is in the publish plan but is not publishable")
    for dep in package["dependencies"]:
        if dep.get("kind") not in (None, "normal"):
            continue
        dep_name = dep["name"]
        dep_package = workspace_packages.get(dep_name)
        if dep_package is None or not is_publishable(dep_package):
            continue
        baseline = PUBLISH_GRAPH_EXTERNAL_BASELINE.get(dep_name)
        if baseline and baseline.get("version") == dep_package.get("version"):
            continue
        if dep_name not in index:
            missing.append(
                f"{dep_name} is a publishable workspace normal dependency of "
                f"{consumer} but is absent from publish_order"
            )
            continue
        if index[dep_name] >= index[consumer]:
            order_violations.append(f"{dep_name} must precede {consumer}")
if missing:
    raise SystemExit("; ".join(missing))
if order_violations:
    raise SystemExit("; ".join(order_violations))
PY
}

test_official_schema_route_converter_is_shared
test_publish_plan_is_closed_topological_graph
test_direct_library_bypass_rejected
test_release_evidence_command_rejected
test_release_evidence_identity_rejected
test_release_evidence_valid
test_immutability_probe_failure
test_immutability_malformed_response
test_immutability_existing_version
test_immutability_absent_version
test_manifest_bound_source_archive
test_upack_requires_sbom
test_upack_includes_sbom

echo "proget-publish.test: 13/13 passed"
