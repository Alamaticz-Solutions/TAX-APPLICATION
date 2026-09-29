#!/usr/bin/env bash
#
# proget-publish.sh — evidence-gated ProGet publish lane.
#
# Publishes the artifacts produced by `scripts/appfw framework package` to the
# approved ProGet feeds after a strict release gate has retained
# release_ready:true evidence:
#   * framework Cargo crates -> ProGet Cargo feed, in dependency order
#   * toolchain/binaries/docs/source tarballs + manifest + SBOM -> ProGet
#     universal (upack) feed as one versioned, immutable package
#
# Gates (each stage is recorded in target/appfw/proget-publish.json):
#   credentials       PROGET_API_KEY / PROGET_SERVER present, not placeholders
#   release-evidence  retained bitbucket-release-gate.json is ok + release_ready;
#                     stable versions require the strict (non-focused) lane
#   package           scripts/appfw framework package --json (release profile)
#   identity          BITBUCKET_TAG == manifest version == appfw_cli version,
#                     packaged commit == BITBUCKET_COMMIT
#   versions          every plan crate resolves in cargo metadata; the tag
#                     anchors to appfw-cli; per-crate versions are recorded
#   integrity         artifact SHA-256s match the ProGet manifest
#   immutability      already-published crates are skipped with a warning;
#                     an existing upack version is a hard failure (no overwrite)
#   publish-crates    cargo publish --locked in plan order, waiting for each
#                     version to appear in the sparse index before the next
#   publish-upack     build .upack with provenance metadata and upload
#
# Environment contract:
#   PROGET_API_KEY               (required) publish-scoped ProGet API key
#   PROGET_SERVER                (required) base URL, e.g. https://proget.example.com
#   APPFW_PACKAGE_PROFILE        debug|release (default release: distributed
#                                binaries are always optimized builds)
#   APPFW_PROGET_UPACK_FEED      universal feed name (default pds-app-framework)
#   APPFW_PROGET_UPACK_GROUP     upack group (default pds/app-framework)
#   APPFW_PROGET_UPACK_NAME      upack name (default app-framework-toolchain)
#   APPFW_CARGO_REGISTRY         cargo registry name (default pds-app-framework-crates)
#   APPFW_PROGET_DRY_RUN         true = run every gate and build the upack, but
#                                skip all network publishes (default false)
#   APPFW_PROGET_INDEX_TIMEOUT   seconds to wait for a published crate to appear
#                                in the sparse index (default 120)
#   APPFW_RELEASE_ARTIFACT_DIR   evidence root (default target/appfw)
#
# This step consumes the release-gate evidence downloaded from the prior
# pipeline step; it must NOT call prepare-appfw-evidence-root.sh.
#
# Official GraphQL schema HTTP route converter (do not fork in this script):
#   appfw_codegen::schema_route_segment in app_gen/src/schema_route.rs
#   resolved by appfw_introspect and appfw-test via Cargo (no #[path]).
#   Inflector 0.11 kebab after leading-slash trim (nexus_work → nexus-work).
#
# Usage: bash scripts/ci/proget-publish.sh
#
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
cd "$repo_root"

report_dir="${APPFW_RELEASE_ARTIFACT_DIR:-target/appfw}"
package_dir="${APPFW_PACKAGE_DIR:-$repo_root/target/appfw/proget}"
evidence_file="$report_dir/proget-publish.json"
work_dir="$repo_root/target/proget-publish-work"
mkdir -p "$report_dir"

build_profile="${APPFW_PACKAGE_PROFILE:-release}"
upack_feed="${APPFW_PROGET_UPACK_FEED:-pds-app-framework}"
upack_group="${APPFW_PROGET_UPACK_GROUP:-pds/app-framework}"
upack_name="${APPFW_PROGET_UPACK_NAME:-app-framework-toolchain}"
cargo_registry="${APPFW_CARGO_REGISTRY:-pds-app-framework-crates}"
index_timeout="${APPFW_PROGET_INDEX_TIMEOUT:-120}"
dry_run=false
case "$(printf '%s' "${APPFW_PROGET_DRY_RUN:-}" | tr '[:upper:]' '[:lower:]')" in
  1 | true | yes | y | on)
    dry_run=true
    ;;
esac

manifest_file="$package_dir/app-framework-proget-manifest.json"
release_evidence_file="$report_dir/bitbucket-release-gate.json"
crate_versions_file="$report_dir/proget-publish-crate-versions.txt"

package_version=""
release_tag="${BITBUCKET_TAG:-}"
source_commit="${BITBUCKET_COMMIT:-}"
registry_index_url=""
upack_file=""
upack_uploaded=false

stage_names=()
stage_statuses=()
published_crates=()
skipped_crates=()

log() {
  printf '\n[%s] %s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" "$*"
}

write_evidence() {
  # write_evidence <ok-bool> <failed-stage-or-empty>
  local ok="$1"
  local failed_stage="${2:-}"
  local stages="" index

  for index in "${!stage_names[@]}"; do
    stages+="${stage_names[$index]}=${stage_statuses[$index]}"$'\n'
  done

  EVIDENCE_OK="$ok" \
  EVIDENCE_FAILED_STAGE="$failed_stage" \
  EVIDENCE_DRY_RUN="$dry_run" \
  EVIDENCE_VERSION="${package_version:-}" \
  EVIDENCE_TAG="${release_tag:-}" \
  EVIDENCE_COMMIT="${source_commit:-}" \
  EVIDENCE_BUILD_PROFILE="$build_profile" \
  EVIDENCE_STAGES="$stages" \
  EVIDENCE_REGISTRY="$cargo_registry" \
  EVIDENCE_PUBLISHED="$(printf '%s\n' ${published_crates[@]+"${published_crates[@]}"})" \
  EVIDENCE_SKIPPED="$(printf '%s\n' ${skipped_crates[@]+"${skipped_crates[@]}"})" \
  EVIDENCE_UPACK_FEED="$upack_feed" \
  EVIDENCE_UPACK_GROUP="$upack_group" \
  EVIDENCE_UPACK_NAME="$upack_name" \
  EVIDENCE_UPACK_FILE="${upack_file:-}" \
  EVIDENCE_UPACK_UPLOADED="$upack_uploaded" \
  EVIDENCE_MANIFEST="$manifest_file" \
  python3 - "$evidence_file" <<'PY'
import datetime as dt
import json
import os
import sys


def env_lines(name):
    return [line for line in os.environ.get(name, "").splitlines() if line]


evidence = {
    "command": "proget-publish",
    "ok": os.environ["EVIDENCE_OK"] == "true",
    "dry_run": os.environ["EVIDENCE_DRY_RUN"] == "true",
    "generated_at_utc": dt.datetime.now(dt.timezone.utc)
    .replace(microsecond=0)
    .isoformat()
    .replace("+00:00", "Z"),
    "version": os.environ["EVIDENCE_VERSION"],
    "tag": os.environ["EVIDENCE_TAG"],
    "commit": os.environ["EVIDENCE_COMMIT"],
    "build_profile": os.environ["EVIDENCE_BUILD_PROFILE"],
    "stages": [
        {"name": line.rsplit("=", 1)[0], "ok": line.rsplit("=", 1)[1] == "true"}
        for line in env_lines("EVIDENCE_STAGES")
    ],
    "crates": {
        "registry": os.environ["EVIDENCE_REGISTRY"],
        "published": env_lines("EVIDENCE_PUBLISHED"),
        "skipped_existing": env_lines("EVIDENCE_SKIPPED"),
    },
    "upack": {
        "feed": os.environ["EVIDENCE_UPACK_FEED"],
        "group": os.environ["EVIDENCE_UPACK_GROUP"],
        "name": os.environ["EVIDENCE_UPACK_NAME"],
        "version": os.environ["EVIDENCE_VERSION"],
        "file": os.environ["EVIDENCE_UPACK_FILE"],
        "uploaded": os.environ["EVIDENCE_UPACK_UPLOADED"] == "true",
    },
    "manifest": os.environ["EVIDENCE_MANIFEST"],
}
if os.environ.get("EVIDENCE_FAILED_STAGE"):
    evidence["failed_stage"] = os.environ["EVIDENCE_FAILED_STAGE"]
with open(sys.argv[1], "w", encoding="utf-8") as handle:
    json.dump(evidence, handle, indent=2, sort_keys=True)
    handle.write("\n")
PY
}

run_stage() {
  # run_stage <name> <fn...> — record the stage; on failure write evidence and exit.
  local stage="$1"
  shift

  log "Stage: $stage"
  set +e
  "$@"
  local status=$?
  set -e

  stage_names+=("$stage")
  if [[ $status -eq 0 ]]; then
    stage_statuses+=(true)
  else
    stage_statuses+=(false)
    write_evidence false "$stage"
    log "Stage failed: $stage (exit $status); evidence at $evidence_file"
    exit "$status"
  fi
}

proget_curl() {
  # proget_curl <curl-args...> — authenticated ProGet API call.
  curl -fsS -H "X-ApiKey: ${PROGET_API_KEY:-}" "$@"
}

stage_credentials() {
  local missing=0
  for command in curl git python3 tar; do
    if ! command -v "$command" >/dev/null 2>&1; then
      missing=1
    fi
  done
  if [[ "$missing" == "1" ]]; then
    if command -v apt-get >/dev/null 2>&1 && [[ "$(id -u)" == "0" ]]; then
      log "Installing publish prerequisites"
      apt-get update || return 2
      apt-get install -y --no-install-recommends ca-certificates curl git python3 tar || return 2
    fi
    for command in curl git python3 tar; do
      if ! command -v "$command" >/dev/null 2>&1; then
        echo "proget-publish: $command is required" >&2
        return 2
      fi
    done
  fi

  if [[ "$dry_run" == "true" ]]; then
    log "Dry run: skipping ProGet credential requirement"
    return 0
  fi

  local key="${PROGET_API_KEY:-}"
  local server="${PROGET_SERVER:-}"
  if [[ -z "$key" || -z "$server" ]]; then
    cat >&2 <<'EOF'
proget-publish requires PROGET_API_KEY and PROGET_SERVER in the environment.
Map them from the secured Bitbucket workspace/deployment variables, e.g.:
  export PROGET_SERVER="$ProgetServer"
  export PROGET_API_KEY="$ProgetApiKey"
The API key must be publish-scoped for the Cargo and universal feeds; do not
print it in logs or commit it to the repository.
EOF
    return 2
  fi
  case "$(printf '%s' "$key" | tr '[:upper:]' '[:lower:]')" in
    changeme | change-me | change_me | dummy | example | placeholder | test | your-key | your_key)
      echo "proget-publish: PROGET_API_KEY is set to a placeholder value" >&2
      return 2
      ;;
  esac
  case "$server" in
    http://* | https://*)
      ;;
    *)
      echo "proget-publish: PROGET_SERVER must be a base URL, got: $server" >&2
      return 2
      ;;
  esac
}

stage_release_evidence() {
  if [[ ! -f "$release_evidence_file" ]]; then
    cat >&2 <<EOF
proget-publish: missing release gate evidence: $release_evidence_file
This step must run after the release gate step in the same pipeline so the
retained bitbucket-release-gate.json artifact is downloaded before publishing.
EOF
    return 2
  fi

  python3 - "$release_evidence_file" "${source_commit:-}" "${release_tag:-}" "$dry_run" <<'PY'
import json
import os
import sys

with open(sys.argv[1], "r", encoding="utf-8") as handle:
    evidence = json.load(handle)

expected_commit = sys.argv[2]
expected_tag = sys.argv[3]
dry_run = sys.argv[4] == "true"
failures = []
if evidence.get("command") != "bitbucket-release-gate":
    failures.append("release gate evidence command is not bitbucket-release-gate")
if evidence.get("ok") is not True:
    failures.append("release gate evidence ok is not true")
if evidence.get("release_ready") is not True:
    failures.append("release gate evidence release_ready is not true")
if evidence.get("focused_evidence") is not False:
    failures.append("release gate evidence is not the strict release lane")
if evidence.get("release_authority") != "strict-managed-release":
    failures.append("release gate evidence lacks strict-managed-release authority")

boundary = evidence.get("release_authority_boundary")
if not isinstance(boundary, dict):
    failures.append("release gate evidence authority boundary is missing")
else:
    if boundary.get("strict_release_gate_satisfied") is not True:
        failures.append("strict release gate is not satisfied")
    if boundary.get("managed_release_requirements_satisfied") is not True:
        failures.append("managed release requirements are not satisfied")

requirements = evidence.get("release_requirements")
if not isinstance(requirements, dict) or not requirements:
    failures.append("release gate requirements are missing")
elif any(value is not True for value in requirements.values()):
    failures.append("not all release gate requirements are enabled")

ci = evidence.get("ci")
if not isinstance(ci, dict):
    failures.append("release gate CI identity is missing")
    ci = {}

expected_ci = {
    "bitbucket_commit": expected_commit,
    "bitbucket_tag": expected_tag,
    "bitbucket_build_number": os.environ.get("BITBUCKET_BUILD_NUMBER", ""),
    "bitbucket_pipeline_uuid": os.environ.get("BITBUCKET_PIPELINE_UUID", ""),
}
if not dry_run:
    for key, value in expected_ci.items():
        if not value:
            failures.append(f"current pipeline identity {key} is missing")
for key, expected in expected_ci.items():
    if expected and ci.get(key) != expected:
        failures.append(
            f"release gate CI identity {key} does not match current pipeline"
        )

if failures:
    print("proget-publish: release gate evidence rejects publishing:", file=sys.stderr)
    for failure in failures:
        print(f"  - {failure}", file=sys.stderr)
    sys.exit(1)
PY
}

stage_package() {
  log "Building framework package (profile: $build_profile)"
  # Clear any stale manifest (e.g. from a --plan run) so a failed package
  # build cannot masquerade as a fresh one. run_stage disables errexit, so
  # every critical command here carries an explicit failure return.
  rm -f "$manifest_file"
  APPFW_PACKAGE_PROFILE="$build_profile" "$repo_root/scripts/appfw" framework package --json >"$report_dir/proget-publish-package.log" || return $?

  if [[ ! -f "$manifest_file" ]]; then
    echo "proget-publish: package manifest missing: $manifest_file" >&2
    return 2
  fi

  package_version="$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['version'])" "$manifest_file")" || return 2
  registry_index_url="$(python3 -c "
import json, sys
plan = json.load(open(sys.argv[1]))
print(plan['registry_index'].removeprefix('sparse+'))
" "$package_dir/app-framework-crates-publish-plan-${package_version}.json")" || return 2
  log "Packaged version $package_version (registry index: $registry_index_url)"
}

stage_identity() {
  local manifest_profile packaged_commit cli_version
  manifest_profile="$(python3 -c "import json,sys; print(json.load(open(sys.argv[1])).get('build_profile',''))" "$manifest_file")"
  cli_version="$(sed -n 's/^[[:space:]]*version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' appfw_cli/Cargo.toml | head -1)"
  packaged_commit="$(cat "$package_dir/COMMIT_FULL.txt" 2>/dev/null || true)"

  if [[ "$manifest_profile" != "$build_profile" ]]; then
    echo "proget-publish: manifest build_profile '$manifest_profile' != requested '$build_profile'" >&2
    return 2
  fi
  if [[ "$package_version" != "$cli_version" ]]; then
    echo "proget-publish: manifest version '$package_version' != appfw_cli/Cargo.toml '$cli_version'" >&2
    return 2
  fi

  if [[ -z "$release_tag" ]]; then
    if [[ "$dry_run" == "true" ]]; then
      log "Dry run: no BITBUCKET_TAG; skipping tag identity check"
    else
      echo "proget-publish: BITBUCKET_TAG is required; publish only runs on the v* tag lane" >&2
      return 2
    fi
  else
    if [[ ! "$release_tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?$ ]]; then
      echo "proget-publish: tag '$release_tag' is not a vMAJOR.MINOR.PATCH[-prerelease] tag" >&2
      return 2
    fi
    if [[ "${release_tag#v}" != "$package_version" ]]; then
      echo "proget-publish: tag '$release_tag' does not match packaged version '$package_version'" >&2
      return 2
    fi
  fi

  if [[ -n "$source_commit" && -n "$packaged_commit" && "$source_commit" != "$packaged_commit" ]]; then
    echo "proget-publish: packaged commit '$packaged_commit' != pipeline commit '$source_commit'" >&2
    return 2
  fi
  if [[ -z "$source_commit" ]]; then
    source_commit="$packaged_commit"
  fi

  # Stable versions may only publish from the strict release lane; -rc.N and
  # other prerelease versions may publish from focused-evidence runs.
  if [[ "$package_version" != *-* ]]; then
    python3 - "$release_evidence_file" <<'PY'
import json
import sys

with open(sys.argv[1], "r", encoding="utf-8") as handle:
    evidence = json.load(handle)

if evidence.get("focused_evidence") is True:
    print(
        "proget-publish: stable versions require the strict v* tag release gate; "
        "focused evidence may only publish prerelease (-rc.N) versions",
        file=sys.stderr,
    )
    sys.exit(1)
PY
  fi
}

stage_versions() {
  # The toolchain/tag version tracks appfw-cli (framework package's version
  # source); other crates carry their own versions (e.g. appfw-runtime 0.1.1
  # under toolchain 0.1.6). Record every plan crate's version for the publish
  # and immutability stages, and anchor the release tag to appfw-cli.
  python3 - "$package_dir/app-framework-crates-publish-plan-${package_version}.json" "$package_version" "$crate_versions_file" <<'PY' || return 2
import json
import subprocess
import sys

plan_path, package_version, versions_path = sys.argv[1], sys.argv[2], sys.argv[3]
with open(plan_path, "r", encoding="utf-8") as handle:
    plan = json.load(handle)

metadata = json.loads(
    subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout
)
versions = {package["name"]: package["version"] for package in metadata["packages"]}

failures = []
for crate in plan["publish_order"]:
    if crate not in versions:
        failures.append(f"{crate}: not found in cargo metadata")

cli_version = versions.get("appfw-cli")
if cli_version != package_version:
    failures.append(
        f"appfw-cli version {cli_version} != packaged toolchain version {package_version}"
    )

if failures:
    print("proget-publish: crate version check failed:", file=sys.stderr)
    for failure in failures:
        print(f"  - {failure}", file=sys.stderr)
    sys.exit(1)

with open(versions_path, "w", encoding="utf-8") as handle:
    for crate in plan["publish_order"]:
        handle.write(f"{crate}={versions[crate]}\n")

print(f"Recorded versions for {len(plan['publish_order'])} publishable crates.")
PY
}

crate_version() {
  # crate_version <crate> — version recorded by stage_versions.
  sed -n "s/^$1=//p" "$crate_versions_file" | head -1
}

stage_integrity() {
  python3 - "$manifest_file" <<'PY'
import hashlib
import json
import sys
from pathlib import Path

with open(sys.argv[1], "r", encoding="utf-8") as handle:
    manifest = json.load(handle)

failures = []
for artifact in manifest.get("artifacts", []):
    path = Path(artifact["path"])
    expected = artifact.get("sha256", "")
    if not path.is_file():
        failures.append(f"{artifact['name']}: missing file {path}")
        continue
    actual = hashlib.sha256(path.read_bytes()).hexdigest()
    if actual != expected:
        failures.append(f"{artifact['name']}: sha256 {actual} != manifest {expected}")

if failures:
    print("proget-publish: artifact integrity check failed:", file=sys.stderr)
    for failure in failures:
        print(f"  - {failure}", file=sys.stderr)
    sys.exit(1)

print(f"Verified {len(manifest.get('artifacts', []))} artifact hashes against the manifest.")
PY
}

crate_index_path() {
  # crate_index_path <crate> — cargo sparse index relative path.
  local crate="$1"
  case "${#crate}" in
    1)
      printf '1/%s' "$crate"
      ;;
    2)
      printf '2/%s' "$crate"
      ;;
    3)
      printf '3/%s/%s' "${crate:0:1}" "$crate"
      ;;
    *)
      printf '%s/%s/%s' "${crate:0:2}" "${crate:2:2}" "$crate"
      ;;
  esac
}

crate_version_in_index() {
  # crate_version_in_index <crate> <version> — 0 when the version is published.
  local crate="$1"
  local version="$2"
  local index_body
  index_body="$(proget_curl "${registry_index_url%/}/$(crate_index_path "$crate")" 2>/dev/null)" || return 1
  printf '%s' "$index_body" | grep -qF "\"vers\":\"${version}\""
}

stage_immutability() {
  if [[ "$dry_run" == "true" ]]; then
    log "Dry run: skipping remote immutability probes"
    return 0
  fi

  local crate crate_ver
  while IFS= read -r crate; do
    [[ -z "$crate" ]] && continue
    crate_ver="$(crate_version "$crate")"
    if crate_version_in_index "$crate" "$crate_ver"; then
      log "WARNING: $crate@$crate_ver already exists in $cargo_registry; it will be skipped, not overwritten"
      skipped_crates+=("$crate@$crate_ver")
    fi
  done < <(python3 -c "
import json, sys
plan = json.load(open(sys.argv[1]))
print('\n'.join(plan['publish_order']))
" "$package_dir/app-framework-crates-publish-plan-${package_version}.json")

  local versions_response version_state
  if ! versions_response="$(
    proget_curl "$PROGET_SERVER/upack/$upack_feed/versions?group=$upack_group&name=$upack_name&version=$package_version"
  )"; then
    echo "proget-publish: unable to verify whether $upack_name@$package_version already exists; refusing to publish" >&2
    return 2
  fi
  if ! version_state="$(python3 -c "
import json, sys
try:
    data = json.loads(sys.argv[1])
except (json.JSONDecodeError, TypeError) as error:
    print(f'invalid ProGet versions response: {error}', file=sys.stderr)
    sys.exit(2)
if not isinstance(data, (list, dict)):
    print(f'invalid ProGet versions response type: {type(data).__name__}', file=sys.stderr)
    sys.exit(2)
print('present' if bool(data) else 'absent')
" "$versions_response")"; then
    echo "proget-publish: malformed ProGet version response; refusing to publish" >&2
    return 2
  fi
  if [[ "$version_state" == "present" ]]; then
    echo "proget-publish: $upack_name@$package_version already exists in feed $upack_feed; refusing to overwrite a published release" >&2
    return 2
  fi
}

wait_for_crate_in_index() {
  local crate="$1"
  local version="$2"
  local waited=0
  while (( waited < index_timeout )); do
    if crate_version_in_index "$crate" "$version"; then
      return 0
    fi
    sleep 5
    waited=$((waited + 5))
  done
  echo "proget-publish: $crate@$version did not appear in the index within ${index_timeout}s" >&2
  return 1
}

stage_publish_crates() {
  local source_tar source_name source_root
  if ! source_tar="$(python3 - "$manifest_file" "$package_dir" <<'PY'
import json
import sys
from pathlib import Path

manifest_path = Path(sys.argv[1])
package_root = Path(sys.argv[2]).resolve()
with manifest_path.open("r", encoding="utf-8") as handle:
    manifest = json.load(handle)

matches = [
    artifact
    for artifact in manifest.get("artifacts", [])
    if artifact.get("name") == "app-framework-source"
]
if len(matches) != 1:
    print(
        f"proget-publish: manifest must name exactly one app-framework-source artifact, found {len(matches)}",
        file=sys.stderr,
    )
    sys.exit(2)

source_path = Path(matches[0].get("path", "")).resolve()
try:
    source_path.relative_to(package_root)
except ValueError:
    print(
        f"proget-publish: manifest source archive escapes package directory: {source_path}",
        file=sys.stderr,
    )
    sys.exit(2)
if not source_path.is_file() or not source_path.name.startswith("app-framework-source-") or not source_path.name.endswith(".tar.gz"):
    print(
        f"proget-publish: manifest source archive is missing or invalid: {source_path}",
        file=sys.stderr,
    )
    sys.exit(2)
print(source_path)
PY
  )"; then
    return 2
  fi
  source_name="$(basename "$source_tar" .tar.gz)"

  rm -rf "$work_dir"
  mkdir -p "$work_dir"
  tar -xzf "$source_tar" -C "$work_dir" || return 2
  source_root="$work_dir/$source_name"
  if [[ ! -d "$source_root" ]]; then
    echo "proget-publish: verified source archive did not contain expected root $source_name" >&2
    return 2
  fi

  local token_env
  token_env="CARGO_REGISTRIES_$(printf '%s' "$cargo_registry" | tr '[:lower:]-' '[:upper:]_')_TOKEN"
  export "$token_env=${PROGET_API_KEY:-}"

  local crate crate_ver skipped
  while IFS= read -r crate; do
    [[ -z "$crate" ]] && continue
    crate_ver="$(crate_version "$crate")"
    skipped=0
    for existing in ${skipped_crates[@]+"${skipped_crates[@]}"}; do
      if [[ "$existing" == "$crate@$crate_ver" ]]; then
        skipped=1
        break
      fi
    done
    if [[ "$skipped" == "1" ]]; then
      log "Skipping already-published crate $crate@$crate_ver"
      continue
    fi
    if [[ "$dry_run" == "true" ]]; then
      log "Dry run: would publish $crate@$crate_ver to $cargo_registry"
      continue
    fi
    log "Publishing $crate@$crate_ver to $cargo_registry"
    (cd "$source_root" && cargo publish --locked --registry "$cargo_registry" -p "$crate") || return $?
    wait_for_crate_in_index "$crate" "$crate_ver" || return $?
    published_crates+=("$crate@$crate_ver")
  done < <(python3 -c "
import json, sys
plan = json.load(open(sys.argv[1]))
print('\n'.join(plan['publish_order']))
" "$package_dir/app-framework-crates-publish-plan-${package_version}.json")
}

stage_publish_upack() {
  upack_file="$package_dir/${upack_name}-${package_version}.upack"

  python3 - "$package_dir" "$manifest_file" "$upack_file" "$upack_group" "$upack_name" "$package_version" \
    "${source_commit:-unknown}" "${BITBUCKET_BUILD_NUMBER:-}" "${BITBUCKET_PIPELINE_UUID:-}" "$build_profile" "$report_dir" <<'PY' || return 2
import json
import sys
import zipfile
from pathlib import Path

(
    package_dir,
    manifest_path,
    upack_path,
    group,
    name,
    version,
    commit,
    build_number,
    pipeline_uuid,
    build_profile,
    report_dir,
) = sys.argv[1:12]

package_dir = Path(package_dir)
with open(manifest_path, "r", encoding="utf-8") as handle:
    manifest = json.load(handle)

platform = manifest.get("platform", "unknown")
files = [Path(artifact["path"]) for artifact in manifest.get("artifacts", [])]
files.append(Path(manifest_path))
sbom = Path(report_dir) / "sbom-rust-workspace.cdx.json"
if not sbom.is_file():
    print(f"proget-publish: required Rust SBOM not found at {sbom}", file=sys.stderr)
    sys.exit(2)
files.append(sbom)

upack_json = {
    "group": group,
    "name": name,
    "version": version,
    "title": "App Framework toolchain",
    "description": "App Framework ProGet toolchain bundle: CLI/toolchain, binaries, product docs, source archive, crates publish plan, manifest, and SBOM.",
    "_commit": commit,
    "_buildNumber": build_number,
    "_pipelineUuid": pipeline_uuid,
    "_platform": platform,
    "_buildProfile": build_profile,
}

seen = set()
with zipfile.ZipFile(upack_path, "w") as archive:
    archive.writestr("upack.json", json.dumps(upack_json, indent=2))
    for path in files:
        if not path.is_file() or path.name in seen:
            continue
        seen.add(path.name)
        compression = zipfile.ZIP_STORED if path.suffix == ".gz" else zipfile.ZIP_DEFLATED
        archive.write(path, f"package/{path.name}", compress_type=compression)

print(f"Built {upack_path} with {len(seen)} artifacts.")
PY

  if [[ "$dry_run" == "true" ]]; then
    log "Dry run: would upload $upack_file to $upack_feed"
    return 0
  fi

  log "Uploading $(basename "$upack_file") to feed $upack_feed"
  proget_curl --upload-file "$upack_file" "$PROGET_SERVER/upack/$upack_feed/upload" || return 2
  upack_uploaded=true
}

if [[ "${APPFW_PROGET_LIBRARY_ONLY:-false}" == "true" ]]; then
  if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    echo "proget-publish: APPFW_PROGET_LIBRARY_ONLY is test-only and cannot bypass direct execution" >&2
    exit 2
  fi
  return 0
fi

run_stage "credentials" stage_credentials
run_stage "release-evidence" stage_release_evidence
run_stage "package" stage_package
run_stage "identity" stage_identity
run_stage "versions" stage_versions
run_stage "integrity" stage_integrity
run_stage "immutability" stage_immutability
run_stage "publish-crates" stage_publish_crates
run_stage "publish-upack" stage_publish_upack

write_evidence true ""
log "ProGet publish evidence written to $evidence_file"
if [[ "$dry_run" == "true" ]]; then
  log "Dry run complete: no artifacts were published"
else
  log "ProGet publish complete for version $package_version"
fi
