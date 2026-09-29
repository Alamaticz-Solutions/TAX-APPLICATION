#!/usr/bin/env bash
#
# supply-chain-gate.sh — static supply-chain and code-quality gate.
#
# Runs the checks that previously had NO coverage in the pipeline:
#   * cargo fmt   --all --check                 (formatting)
#   * cargo clippy --workspace --all-targets    (lint, warnings = errors)
#   * cargo audit                               (RUSTSEC vulnerability scan)
#   * cargo deny  check                         (advisories+licenses+bans+sources)
#   * scripts/appfw dependency-check            (retained advisory/upgrade evidence)
#   * CycloneDX SBOM generation                 (Rust, frontend, image when present)
#   * scripts/ci/phi-log-lint.sh                (PHI/PII log-leak guard)
#
# Tooling (cargo-audit, cargo-deny, and npm when frontend lockfiles are present)
# is installed here so the gate is self-contained on the CI image
# (rust:1.x-bookworm). Installs are skipped when the binaries are already
# present (e.g. warm cache / local runs). npm is required by
# scripts/appfw dependency-check --strict, which shells out to `npm audit`
# and `npm outdated` for each tracked npm lockfile — it is not just used for
# the CycloneDX SBOM step below, which parses package-lock.json in Python.
#
# Usage:  bash scripts/ci/supply-chain-gate.sh
#
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
cd "$repo_root"

report_dir="${APPFW_RELEASE_ARTIFACT_DIR:-target/appfw}"
report_file="$report_dir/supply-chain-gate.json"
dependency_report="$report_dir/dependency-check.json"
phi_report="$report_dir/phi-log-lint.json"
sbom_manifest="$report_dir/sbom-manifest.json"
rust_sbom="$report_dir/sbom-rust-workspace.cdx.json"
frontend_sbom="$report_dir/sbom-frontend-packages.cdx.json"
image_sbom="$report_dir/sbom-deployable-image.cdx.json"
mkdir -p "$report_dir"
advisory_cargo_home="${CARGO_HOME:-}"
advisory_cargo_home_source=unset
advisory_cargo_home_writable=false

check_names=()
check_statuses=()

log() {
  printf '\n[%s] %s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" "$*"
}

record_check() {
  # record_check <name> <exit-code>
  check_names+=("$1")
  check_statuses+=("$2")
}

file_present_json_bool() {
  if [[ -f "$1" ]]; then
    printf 'true'
  else
    printf 'false'
  fi
}

file_sha256_json() {
  local path="$1"
  local digest=""
  if [[ ! -f "$path" ]]; then
    printf 'null'
    return 0
  fi
  if command -v sha256sum >/dev/null 2>&1; then
    digest="$(sha256sum "$path" | awk '{print $1}')"
  elif command -v shasum >/dev/null 2>&1; then
    digest="$(shasum -a 256 "$path" | awk '{print $1}')"
  fi
  if [[ -z "$digest" ]]; then
    printf 'null'
  else
    printf '"sha256:%s"' "$digest"
  fi
}

file_bytes_json() {
  local path="$1"
  if [[ -f "$path" ]]; then
    wc -c <"$path" | tr -d '[:space:]'
  else
    printf 'null'
  fi
}

frontend_sbom_required() {
  local lockfiles
  if command -v git >/dev/null 2>&1; then
    lockfiles="$(
      git ls-files \
        | grep -E '(^|/)(package-lock\.json|pnpm-lock\.yaml|yarn\.lock)$' \
        | grep -Ev '(^|/)node_modules/' \
        || true
    )"
    if [[ -n "$lockfiles" ]]; then
      return 0
    fi
  fi

  local found_lockfile
  found_lockfile="$(
    find . \
    \( -path './.git' -o -path './target' -o -path './app_gen/target' -o -path '*/node_modules' \) -prune -o \
    \( -name 'package-lock.json' -o -name 'pnpm-lock.yaml' -o -name 'yarn.lock' \) -type f -print -quit
  )"
  [[ -n "$found_lockfile" ]]
}

env_flag_enabled() {
  case "$(printf '%s' "${1:-}" | tr '[:upper:]' '[:lower:]')" in
    1 | true | yes | y | on | required | require)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

image_sbom_required_by_policy() {
  env_flag_enabled "${APPFW_REQUIRE_DEPLOYABLE_IMAGE_SBOM:-}" \
    || env_flag_enabled "${APPFW_RELEASE_REQUIRE_IMAGE_SBOM:-}"
}

image_source_env_present() {
  [[ -n "${APPFW_SBOM_IMAGE_SOURCE:-}" \
    || -n "${APPFW_SBOM_IMAGE_SOURCES:-}" \
    || -n "${APPFW_SBOM_IMAGE_REF:-}" \
    || -n "${APPFW_SBOM_IMAGE_TAR:-}" \
    || -n "${APPFW_SBOM_IMAGE_ARTIFACT:-}" \
    || -n "${APPFW_DEPLOYABLE_IMAGE:-}" \
    || -n "${APPFW_IMAGE_REF:-}" \
    || -n "${APPFW_IMAGE_TAR:-}" \
    || -n "${APPFW_IMAGE_ARTIFACT:-}" ]]
}

retained_image_artifact_present() {
  local candidate
  for candidate in \
    "$report_dir"/deployable-image*.tar \
    "$report_dir"/*.image.tar \
    "$report_dir"/*.oci \
    "$report_dir"/*.oci.tar \
    "$report_dir"/images/*.tar \
    "$report_dir"/images/*.tar.gz \
    "$report_dir"/containers/*.tar \
    "$report_dir"/containers/*.tar.gz; do
    if [[ -e "$candidate" ]]; then
      return 0
    fi
  done

  return 1
}

image_sbom_required() {
  if image_sbom_required_by_policy || image_source_env_present || retained_image_artifact_present; then
    return 0
  fi

  return 1
}

image_sbom_requirement_reason() {
  if image_sbom_required_by_policy; then
    printf 'release policy requires deployable image SBOM via APPFW_REQUIRE_DEPLOYABLE_IMAGE_SBOM or APPFW_RELEASE_REQUIRE_IMAGE_SBOM'
  elif image_source_env_present; then
    printf 'deployable image source environment variable is set'
  elif retained_image_artifact_present; then
    printf 'retained deployable image artifact was found under the release artifact directory'
  else
    printf 'not-applicable: no deployable image source env var or retained image tar/OCI artifact was found'
  fi
}

path_writable() {
  local path="$1"
  local probe
  mkdir -p "$path" 2>/dev/null || return 1
  probe="$path/.appfw-write-test-$$"
  : >"$probe" 2>/dev/null || return 1
  rm -f "$probe" 2>/dev/null || true
}

configure_dependency_cargo_home() {
  local requested="${APPFW_DEPENDENCY_CHECK_CARGO_HOME:-}"
  local ambient="${CARGO_HOME:-}"
  local default_home=""
  if [[ -n "${HOME:-}" ]]; then
    default_home="$HOME/.cargo"
  fi

  if [[ -n "$requested" ]]; then
    advisory_cargo_home="$requested"
    advisory_cargo_home_source=APPFW_DEPENDENCY_CHECK_CARGO_HOME
  else
    if [[ -n "$ambient" ]] && path_writable "$ambient"; then
      advisory_cargo_home="$ambient"
      advisory_cargo_home_source=CARGO_HOME
    elif [[ -n "$default_home" ]] && path_writable "$default_home"; then
      advisory_cargo_home="$default_home"
      advisory_cargo_home_source=default-home
    else
      advisory_cargo_home="$report_dir/cargo-home"
      advisory_cargo_home_source=repo-local-fallback
    fi
  fi

  if path_writable "$advisory_cargo_home"; then
    advisory_cargo_home_writable=true
  else
    advisory_cargo_home_writable=false
    echo "supply-chain-gate: dependency advisory Cargo home is not writable: $advisory_cargo_home" >&2
    return 2
  fi

  export CARGO_HOME="$advisory_cargo_home"
  export APPFW_DEPENDENCY_CHECK_CARGO_HOME="$advisory_cargo_home"
  export PATH="$CARGO_HOME/bin:$PATH"
  log "Using dependency advisory Cargo home ($advisory_cargo_home_source): $CARGO_HOME"
}

write_evidence() {
  # write_evidence <ok-json-bool>
  local ok="$1"
  local generated_at dependency_present phi_present sbom_manifest_present rust_sbom_present
  local frontend_required frontend_sbom_present image_required image_sbom_present
  local image_status image_applicability image_reason
  generated_at="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  dependency_present="$(file_present_json_bool "$dependency_report")"
  phi_present="$(file_present_json_bool "$phi_report")"
  sbom_manifest_present="$(file_present_json_bool "$sbom_manifest")"
  rust_sbom_present="$(file_present_json_bool "$rust_sbom")"
  frontend_sbom_present="$(file_present_json_bool "$frontend_sbom")"
  image_sbom_present="$(file_present_json_bool "$image_sbom")"
  if frontend_sbom_required; then
    frontend_required=true
  else
    frontend_required=false
  fi
  if image_sbom_required; then
    image_required=true
  else
    image_required=false
  fi
  image_reason="$(image_sbom_requirement_reason)"
  if [[ "$image_sbom_present" == "true" ]]; then
    image_status=present
  elif [[ "$image_required" == "true" ]]; then
    image_status=missing
  else
    image_status=not-applicable
  fi
  if [[ "$image_required" == "true" ]]; then
    image_applicability=required
  else
    image_applicability=not-applicable
  fi

  {
    printf '{\n'
    printf '  "command": "supply-chain-gate",\n'
    printf '  "ok": %s,\n' "$ok"
    printf '  "generated_at_utc": "%s",\n' "$generated_at"
    printf '  "tool_environment": {\n'
    printf '    "cargo_home": "%s",\n' "${CARGO_HOME:-$advisory_cargo_home}"
    printf '    "cargo_home_source": "%s",\n' "$advisory_cargo_home_source"
    printf '    "cargo_home_writable": %s,\n' "$advisory_cargo_home_writable"
    printf '    "dependency_check_cargo_home": "%s"\n' "${APPFW_DEPENDENCY_CHECK_CARGO_HOME:-$advisory_cargo_home}"
    printf '  },\n'
    printf '  "checks": [\n'
    local index
    for index in "${!check_names[@]}"; do
      if [[ "$index" -gt 0 ]]; then
        printf ',\n'
      fi
      printf '    {\n'
      printf '      "name": "%s",\n' "${check_names[$index]}"
      printf '      "exit_code": %s,\n' "${check_statuses[$index]}"
      if [[ "${check_statuses[$index]}" == "0" ]]; then
        printf '      "ok": true\n'
      else
        printf '      "ok": false\n'
      fi
      printf '    }'
    done
    printf '\n'
    printf '  ],\n'
    printf '  "artifacts": [\n'
	    printf '    {\n'
	    printf '      "name": "dependency-check",\n'
	    printf '      "path": "%s",\n' "$dependency_report"
	    printf '      "required": true,\n'
	    printf '      "present": %s,\n' "$dependency_present"
	    printf '      "sha256": %s,\n' "$(file_sha256_json "$dependency_report")"
	    printf '      "bytes": %s\n' "$(file_bytes_json "$dependency_report")"
	    printf '    },\n'
	    printf '    {\n'
	    printf '      "name": "phi-log-lint",\n'
	    printf '      "path": "%s",\n' "$phi_report"
	    printf '      "required": true,\n'
	    printf '      "present": %s,\n' "$phi_present"
	    printf '      "sha256": %s,\n' "$(file_sha256_json "$phi_report")"
	    printf '      "bytes": %s\n' "$(file_bytes_json "$phi_report")"
	    printf '    },\n'
	    printf '    {\n'
	    printf '      "name": "sbom-manifest",\n'
	    printf '      "path": "%s",\n' "$sbom_manifest"
	    printf '      "required": true,\n'
	    printf '      "present": %s,\n' "$sbom_manifest_present"
	    printf '      "sha256": %s,\n' "$(file_sha256_json "$sbom_manifest")"
	    printf '      "bytes": %s\n' "$(file_bytes_json "$sbom_manifest")"
	    printf '    },\n'
	    printf '    {\n'
	    printf '      "name": "rust-cyclonedx-sbom",\n'
	    printf '      "path": "%s",\n' "$rust_sbom"
	    printf '      "required": true,\n'
	    printf '      "present": %s,\n' "$rust_sbom_present"
	    printf '      "sha256": %s,\n' "$(file_sha256_json "$rust_sbom")"
	    printf '      "bytes": %s\n' "$(file_bytes_json "$rust_sbom")"
	    printf '    },\n'
	    printf '    {\n'
	    printf '      "name": "frontend-cyclonedx-sbom",\n'
	    printf '      "path": "%s",\n' "$frontend_sbom"
	    printf '      "required": %s,\n' "$frontend_required"
	    printf '      "present": %s,\n' "$frontend_sbom_present"
	    printf '      "sha256": %s,\n' "$(file_sha256_json "$frontend_sbom")"
	    printf '      "bytes": %s\n' "$(file_bytes_json "$frontend_sbom")"
	    printf '    },\n'
	    printf '    {\n'
	    printf '      "name": "deployable-image-cyclonedx-sbom",\n'
	    printf '      "path": "%s",\n' "$image_sbom"
	    printf '      "required": %s,\n' "$image_required"
	    printf '      "present": %s,\n' "$image_sbom_present"
	    printf '      "status": "%s",\n' "$image_status"
	    printf '      "applicability": "%s",\n' "$image_applicability"
	    printf '      "applicability_reason": "%s",\n' "$image_reason"
	    printf '      "sha256": %s,\n' "$(file_sha256_json "$image_sbom")"
	    printf '      "bytes": %s\n' "$(file_bytes_json "$image_sbom")"
	    printf '    }\n'
    printf '  ]\n'
    printf '}\n'
  } >"$report_file"
}

run_check() {
  # run_check <name> <command> [args...]
  local name="$1"
  shift

  log "$name"
  set +e
  "$@"
  local status=$?
  set -e

  record_check "$name" "$status"
  if [[ "$status" -ne 0 ]]; then
    write_evidence false
    log "Supply-chain evidence written to $report_file"
    exit "$status"
  fi
}

ensure_tool() {
  # ensure_tool <binary> <cargo-install-args...>
  local bin="$1"
  shift
  if command -v "$bin" >/dev/null 2>&1; then
    log "$bin already installed: $("$bin" --version 2>/dev/null | head -1)"
    return 0
  fi
  log "Installing $bin"
  # --locked keeps the installed tool reproducible across runs.
  cargo install --locked "$@"
}

ensure_python3() {
  if command -v python3 >/dev/null 2>&1; then
    return 0
  fi

  if command -v apt-get >/dev/null 2>&1 && [[ "$(id -u)" == "0" ]]; then
    log "Installing python3"
    apt-get update
    apt-get install -y --no-install-recommends python3
    return 0
  fi

  echo "supply-chain-gate: python3 is required to generate SBOM evidence" >&2
  return 2
}

ensure_npm_for_frontend_audit() {
  if ! frontend_sbom_required; then
    return 0
  fi

  if command -v npm >/dev/null 2>&1; then
    log "npm already installed: $(npm --version 2>/dev/null | head -1)"
    return 0
  fi

  if command -v apt-get >/dev/null 2>&1 && [[ "$(id -u)" == "0" ]]; then
    log "Installing npm for frontend dependency audit"
    apt-get update
    apt-get install -y --no-install-recommends npm
  fi

  if command -v npm >/dev/null 2>&1; then
    log "npm installed: $(npm --version 2>/dev/null | head -1)"
    return 0
  fi

  echo "supply-chain-gate: npm is required for strict frontend lockfile audit" >&2
  return 2
}

ensure_syft() {
  if ! image_sbom_required; then
    return 0
  fi

  if command -v syft >/dev/null 2>&1; then
    log "syft already installed: $(syft version 2>/dev/null | head -1)"
    return 0
  fi

  local missing_tooling=0
  for command in curl tar python3; do
    if ! command -v "$command" >/dev/null 2>&1; then
      missing_tooling=1
    fi
  done
  if [[ "$missing_tooling" == "1" && "$(id -u)" == "0" ]]; then
    if command -v apt-get >/dev/null 2>&1; then
      log "Installing syft download prerequisites"
      apt-get update
      apt-get install -y --no-install-recommends ca-certificates curl tar python3
    fi
  fi
  for command in curl tar python3; do
    if ! command -v "$command" >/dev/null 2>&1; then
      echo "supply-chain-gate: $command is required to install syft for deployable image SBOM generation" >&2
      return 2
    fi
  done

  local version os arch archive base_url tmp_dir install_dir
  version="${APPFW_SYFT_VERSION:-1.45.0}"
  version="${version#v}"
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  case "$(uname -m)" in
    x86_64 | amd64)
      arch=amd64
      ;;
    arm64 | aarch64)
      arch=arm64
      ;;
    *)
      echo "supply-chain-gate: unsupported architecture for syft install: $(uname -m)" >&2
      return 2
      ;;
  esac
  case "$os" in
    linux | darwin)
      ;;
    *)
      echo "supply-chain-gate: unsupported OS for syft install: $os" >&2
      return 2
      ;;
  esac

  archive="syft_${version}_${os}_${arch}.tar.gz"
  base_url="https://github.com/anchore/syft/releases/download/v${version}"
  tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/appfw-syft.XXXXXX")"
  log "Installing syft v${version}"
  curl -fsSLo "$tmp_dir/$archive" "$base_url/$archive"
  curl -fsSLo "$tmp_dir/checksums.txt" "$base_url/syft_${version}_checksums.txt"
  python3 - "$tmp_dir/$archive" "$tmp_dir/checksums.txt" "$archive" <<'PY'
import hashlib
import sys
from pathlib import Path

archive_path = Path(sys.argv[1])
checksums_path = Path(sys.argv[2])
archive_name = sys.argv[3]

expected = None
for line in checksums_path.read_text(encoding="utf-8").splitlines():
    parts = line.split()
    if len(parts) >= 2 and parts[1] == archive_name:
        expected = parts[0]
        break

if not expected:
    raise SystemExit(f"checksum entry for {archive_name} was not found")

actual = hashlib.sha256(archive_path.read_bytes()).hexdigest()
if actual != expected:
    raise SystemExit(f"checksum mismatch for {archive_name}: expected {expected}, got {actual}")
PY
  tar -xzf "$tmp_dir/$archive" -C "$tmp_dir" syft

  install_dir="${APPFW_SYFT_INSTALL_DIR:-/usr/local/bin}"
  if [[ ! -d "$install_dir" || ! -w "$install_dir" ]]; then
    install_dir="${TMPDIR:-/tmp}/appfw-supply-chain-tools/bin"
    mkdir -p "$install_dir"
  fi
  cp "$tmp_dir/syft" "$install_dir/syft"
  chmod 0755 "$install_dir/syft"
  rm -rf "$tmp_dir"
  export PATH="$install_dir:$PATH"
  log "syft installed: $(syft version 2>/dev/null | head -1)"
}

generate_sbom_evidence() {
  python3 - "$repo_root" "$report_dir" "$rust_sbom" "$frontend_sbom" "$image_sbom" "$sbom_manifest" <<'PY'
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tomllib
from urllib.parse import quote
import uuid

repo_root = Path(sys.argv[1]).resolve()
report_dir = Path(sys.argv[2]).resolve()
rust_sbom_path = Path(sys.argv[3]).resolve()
frontend_sbom_path = Path(sys.argv[4]).resolve()
image_sbom_path = Path(sys.argv[5]).resolve()
manifest_path = Path(sys.argv[6]).resolve()


def utc_now():
    return (
        dt.datetime.now(dt.timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z")
    )


generated_at = utc_now()


def rel(path):
    path = Path(path).resolve()
    try:
        return path.relative_to(repo_root).as_posix()
    except ValueError:
        return path.as_posix()


def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def file_size(path):
    return Path(path).stat().st_size if Path(path).is_file() else None


def cyclonedx_base(name, component_name):
    return {
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "serialNumber": f"urn:uuid:{uuid.uuid4()}",
        "version": 1,
        "metadata": {
            "timestamp": generated_at,
            "tools": {
                "components": [
                    {
                        "type": "application",
                        "name": "appfw-supply-chain-gate",
                        "version": "1",
                    }
                ]
            },
            "component": {
                "type": "application",
                "name": component_name,
                "bom-ref": name,
            },
        },
        "components": [],
        "dependencies": [],
    }


def write_json(path, value):
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    Path(path).write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def cargo_purl(name, version):
    return f"pkg:cargo/{quote(name, safe='')}@{quote(version, safe='')}"


def npm_purl(name, version):
    return f"pkg:npm/{quote(name, safe='/')}@{quote(version, safe='')}"


def package_path_name(package_path):
    parts = package_path.split("/")
    indexes = [index for index, part in enumerate(parts) if part == "node_modules"]
    if not indexes:
        return None
    start = indexes[-1] + 1
    if start >= len(parts):
        return None
    if parts[start].startswith("@") and start + 1 < len(parts):
        return f"{parts[start]}/{parts[start + 1]}"
    return parts[start]


def dep_package_path(base_path, dependency_name):
    dep_parts = dependency_name.split("/")
    if dependency_name.startswith("@") and len(dep_parts) >= 2:
        dep_rel = f"node_modules/{dep_parts[0]}/{dep_parts[1]}"
    else:
        dep_rel = f"node_modules/{dependency_name}"

    candidates = []
    if base_path:
        parts = base_path.split("/")
        for index in range(len(parts), -1, -1):
            prefix = "/".join(parts[:index])
            candidates.append(f"{prefix}/{dep_rel}" if prefix else dep_rel)
    candidates.append(dep_rel)
    return candidates


def discover_tracked_files():
    try:
        result = subprocess.run(
            ["git", "ls-files"],
            cwd=repo_root,
            text=True,
            capture_output=True,
            check=True,
        )
    except Exception:
        return []
    return [line for line in result.stdout.splitlines() if line]


def discover_frontend_lockfiles():
    tracked = discover_tracked_files()
    lockfiles = [
        repo_root / path
        for path in tracked
        if re.search(r"(^|/)(package-lock\.json|pnpm-lock\.yaml|yarn\.lock)$", path)
        and "/node_modules/" not in f"/{path}"
    ]
    return sorted(dict.fromkeys(lockfiles), key=lambda path: rel(path))


def generate_rust_sbom():
    lock_path = repo_root / "Cargo.lock"
    if not lock_path.is_file():
        raise SystemExit("supply-chain-gate: missing Cargo.lock; cannot generate Rust SBOM")

    metadata = subprocess.run(
        ["cargo", "metadata", "--locked", "--format-version", "1", "--no-deps"],
        cwd=repo_root,
        text=True,
        capture_output=True,
        check=True,
    )
    metadata_json = json.loads(metadata.stdout)
    workspace_member_ids = set(metadata_json.get("workspace_members", []))
    workspace_packages = [
        package
        for package in metadata_json.get("packages", [])
        if package.get("id") in workspace_member_ids
    ]
    workspace_keys = {
        (package.get("name"), package.get("version"))
        for package in workspace_packages
        if package.get("name") and package.get("version")
    }

    with lock_path.open("rb") as handle:
        cargo_lock = tomllib.load(handle)

    bom = cyclonedx_base("app-framework-rust-workspace", "app-framework-rust-workspace")
    refs_by_key = {}
    refs_by_name = {}
    components = []

    for index, package in enumerate(cargo_lock.get("package", [])):
        name = package.get("name")
        version = package.get("version")
        if not name or not version:
            continue
        source = package.get("source")
        checksum = package.get("checksum")
        base_ref = cargo_purl(name, version)
        bom_ref = base_ref
        if bom_ref in refs_by_key.values():
            digest = hashlib.sha1(f"{name}@{version}:{source or index}".encode()).hexdigest()[:10]
            bom_ref = f"{base_ref}#{digest}"
        refs_by_key[(name, version, source)] = bom_ref
        refs_by_name.setdefault(name, []).append(bom_ref)

        component = {
            "type": "library",
            "name": name,
            "version": version,
            "bom-ref": bom_ref,
            "purl": base_ref,
            "properties": [
                {"name": "appfw:ecosystem", "value": "rust"},
                {"name": "appfw:cargo_source", "value": source or "workspace"},
                {
                    "name": "appfw:workspace_member",
                    "value": "true" if (name, version) in workspace_keys else "false",
                },
            ],
        }
        if checksum:
            component["hashes"] = [{"alg": "SHA-256", "content": checksum}]
        components.append(component)

    for package in workspace_packages:
        name = package.get("name")
        version = package.get("version")
        if not name or not version:
            continue
        key = (name, version, None)
        if key in refs_by_key:
            continue
        bom_ref = cargo_purl(name, version)
        refs_by_key[key] = bom_ref
        refs_by_name.setdefault(name, []).append(bom_ref)
        components.append(
            {
                "type": "library",
                "name": name,
                "version": version,
                "bom-ref": bom_ref,
                "purl": bom_ref,
                "properties": [
                    {"name": "appfw:ecosystem", "value": "rust"},
                    {"name": "appfw:cargo_source", "value": "workspace"},
                    {"name": "appfw:workspace_member", "value": "true"},
                    {"name": "appfw:manifest_path", "value": rel(package.get("manifest_path", ""))},
                ],
            }
        )

    dependencies = []
    workspace_refs = sorted(
        ref
        for (name, version, source), ref in refs_by_key.items()
        if (name, version) in workspace_keys
    )
    dependencies.append(
        {
            "ref": "app-framework-rust-workspace",
            "dependsOn": workspace_refs,
        }
    )

    for package in cargo_lock.get("package", []):
        name = package.get("name")
        version = package.get("version")
        source = package.get("source")
        ref = refs_by_key.get((name, version, source))
        if not ref:
            continue
        depends_on = []
        for dependency in package.get("dependencies", []):
            parts = dependency.split()
            if not parts:
                continue
            dep_name = parts[0]
            dep_version = parts[1] if len(parts) > 1 and re.match(r"^\d", parts[1]) else None
            matches = []
            if dep_version:
                matches = [
                    candidate_ref
                    for (candidate_name, candidate_version, _), candidate_ref in refs_by_key.items()
                    if candidate_name == dep_name and candidate_version == dep_version
                ]
            if not matches:
                matches = refs_by_name.get(dep_name, [])
            depends_on.extend(matches)
        dependencies.append({"ref": ref, "dependsOn": sorted(dict.fromkeys(depends_on))})

    bom["components"] = sorted(components, key=lambda component: component["bom-ref"])
    bom["dependencies"] = dependencies
    write_json(rust_sbom_path, bom)
    return len(bom["components"])


def generate_frontend_sbom(lockfiles):
    unsupported = [
        path
        for path in lockfiles
        if path.name not in ("package-lock.json",)
    ]
    if unsupported:
        joined = ", ".join(rel(path) for path in unsupported)
        raise SystemExit(
            "supply-chain-gate: frontend SBOM generation currently requires "
            f"npm package-lock.json files; unsupported lockfile(s): {joined}"
        )

    bom = cyclonedx_base("app-framework-frontend-packages", "app-framework-frontend-packages")
    components = []
    dependencies = []

    for lockfile in lockfiles:
        data = json.loads(lockfile.read_text(encoding="utf-8"))
        packages = data.get("packages")
        if not isinstance(packages, dict):
            raise SystemExit(
                f"supply-chain-gate: {rel(lockfile)} is not an npm lockfile v2/v3 package map"
            )
        root = packages.get("", {})
        project_name = root.get("name") or lockfile.parent.name
        project_version = root.get("version") or "0.0.0"
        project_ref = f"npm-project:{rel(lockfile.parent)}"
        components.append(
            {
                "type": "application",
                "name": project_name,
                "version": project_version,
                "bom-ref": project_ref,
                "properties": [
                    {"name": "appfw:ecosystem", "value": "npm"},
                    {"name": "appfw:lockfile", "value": rel(lockfile)},
                ],
            }
        )

        path_to_ref = {}
        for package_path, package in packages.items():
            if package_path == "":
                continue
            if not isinstance(package, dict):
                continue
            version = package.get("version")
            name = package.get("name") or package_path_name(package_path)
            if not name or not version:
                continue
            bom_ref = f"npm:{rel(lockfile)}:{package_path}@{version}"
            path_to_ref[package_path] = bom_ref
            component = {
                "type": "library",
                "name": name,
                "version": version,
                "bom-ref": bom_ref,
                "purl": npm_purl(name, version),
                "properties": [
                    {"name": "appfw:ecosystem", "value": "npm"},
                    {"name": "appfw:lockfile", "value": rel(lockfile)},
                    {"name": "appfw:package_path", "value": package_path},
                    {"name": "appfw:dev_dependency", "value": "true" if package.get("dev") else "false"},
                ],
            }
            license_value = package.get("license")
            if isinstance(license_value, str) and license_value:
                if re.fullmatch(r"[A-Za-z0-9+.-]+", license_value):
                    component["licenses"] = [{"license": {"id": license_value}}]
                else:
                    component["licenses"] = [{"expression": license_value}]
            components.append(component)

        root_dep_names = []
        for dependency_set in ("dependencies", "devDependencies", "optionalDependencies"):
            values = root.get(dependency_set, {})
            if isinstance(values, dict):
                root_dep_names.extend(values.keys())
        root_dep_refs = []
        for dependency_name in root_dep_names:
            for candidate_path in dep_package_path("", dependency_name):
                if candidate_path in path_to_ref:
                    root_dep_refs.append(path_to_ref[candidate_path])
                    break
        dependencies.append(
            {
                "ref": project_ref,
                "dependsOn": sorted(dict.fromkeys(root_dep_refs)),
            }
        )

        for package_path, package in packages.items():
            ref = path_to_ref.get(package_path)
            if not ref or not isinstance(package, dict):
                continue
            dep_refs = []
            for dependency_set in ("dependencies", "optionalDependencies"):
                values = package.get(dependency_set, {})
                if not isinstance(values, dict):
                    continue
                for dependency_name in values.keys():
                    for candidate_path in dep_package_path(package_path, dependency_name):
                        if candidate_path in path_to_ref:
                            dep_refs.append(path_to_ref[candidate_path])
                            break
            dependencies.append(
                {
                    "ref": ref,
                    "dependsOn": sorted(dict.fromkeys(dep_refs)),
                }
            )

    bom["components"] = sorted(components, key=lambda component: component["bom-ref"])
    bom["dependencies"] = dependencies
    write_json(frontend_sbom_path, bom)
    return len(bom["components"])


def split_sources(value):
    return [
        source.strip()
        for source in re.split(r"[\n,]+", value)
        if source.strip()
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


def image_sbom_required_by_policy():
    return env_flag_enabled(os.environ.get("APPFW_REQUIRE_DEPLOYABLE_IMAGE_SBOM")) or env_flag_enabled(
        os.environ.get("APPFW_RELEASE_REQUIRE_IMAGE_SBOM")
    )


def discover_image_sources():
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
            sources.extend(split_sources(value))

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
        sources.extend(path.as_posix() for path in sorted(report_dir.glob(pattern)))

    return sorted(dict.fromkeys(sources))


def image_sbom_status(required, present):
    if present:
        return "present"
    if required:
        return "missing"
    return "not-applicable"


def image_sbom_applicability_reason(image_sources, required_by_policy):
    if required_by_policy:
        return (
            "release policy requires deployable image SBOM via "
            "APPFW_REQUIRE_DEPLOYABLE_IMAGE_SBOM or APPFW_RELEASE_REQUIRE_IMAGE_SBOM"
        )
    if image_sources:
        return "deployable image source or retained image artifact was found"
    return (
        "not-applicable: no deployable image source env var or retained image "
        "tar/OCI artifact was found"
    )


def generate_image_sbom(image_sources, required_by_policy):
    if not image_sources:
        if required_by_policy:
            raise SystemExit(
                "supply-chain-gate: deployable image SBOM is required by release "
                "policy, but no image source was provided. Set APPFW_SBOM_IMAGE_SOURCE "
                "to the release image ref/tar, or retain exactly one deployable image "
                "tar/OCI artifact under the release artifact directory."
            )
        return None, None
    if len(image_sources) > 1:
        joined = ", ".join(image_sources)
        raise SystemExit(
            "supply-chain-gate: multiple deployable image sources found; "
            f"set APPFW_SBOM_IMAGE_SOURCE to the exact release image: {joined}"
        )
    if not shutil.which("syft"):
        raise SystemExit(
            "supply-chain-gate: deployable image SBOM is required because an image "
            "source exists, but syft is not installed. Install syft in the CI image "
            "or generate this gate in an environment that has syft on PATH."
        )

    source = image_sources[0]
    result = subprocess.run(
        ["syft", source, "-o", "cyclonedx-json"],
        cwd=repo_root,
        text=True,
        capture_output=True,
    )
    if result.returncode != 0:
        raise SystemExit(result.stderr or result.stdout or "supply-chain-gate: syft failed")
    image_bom = json.loads(result.stdout)
    write_json(image_sbom_path, image_bom)
    return len(image_bom.get("components", [])), image_bom.get("specVersion")


frontend_lockfiles = discover_frontend_lockfiles()
image_sources = discover_image_sources()
image_required_by_policy = image_sbom_required_by_policy()
image_required = bool(image_sources) or image_required_by_policy
image_applicability = "required" if image_required else "not-applicable"
image_applicability_reason = image_sbom_applicability_reason(
    image_sources,
    image_required_by_policy,
)

rust_component_count = generate_rust_sbom()
frontend_component_count = None
if frontend_lockfiles:
    frontend_component_count = generate_frontend_sbom(frontend_lockfiles)
elif frontend_sbom_path.exists():
    frontend_sbom_path.unlink()

if image_sbom_path.exists():
    image_sbom_path.unlink()
image_component_count, image_spec_version = generate_image_sbom(
    image_sources,
    image_required_by_policy,
)

artifacts = [
    {
        "name": "rust-cyclonedx-sbom",
        "path": rel(rust_sbom_path),
        "format": "CycloneDX",
        "spec_version": "1.5",
        "ecosystem": "rust",
        "required": True,
        "present": rust_sbom_path.is_file(),
        "sha256": sha256(rust_sbom_path),
        "bytes": file_size(rust_sbom_path),
        "component_count": rust_component_count,
        "sources": [rel(repo_root / "Cargo.lock"), "cargo metadata --locked --no-deps"],
    },
    {
        "name": "frontend-cyclonedx-sbom",
        "path": rel(frontend_sbom_path),
        "format": "CycloneDX",
        "spec_version": "1.5",
        "ecosystem": "npm",
        "required": bool(frontend_lockfiles),
        "present": frontend_sbom_path.is_file(),
        "sha256": sha256(frontend_sbom_path) if frontend_sbom_path.is_file() else None,
        "bytes": file_size(frontend_sbom_path),
        "component_count": frontend_component_count,
        "sources": [rel(path) for path in frontend_lockfiles],
    },
    {
        "name": "deployable-image-cyclonedx-sbom",
        "path": rel(image_sbom_path),
        "format": "CycloneDX",
        "spec_version": image_spec_version,
        "ecosystem": "container-image",
        "required": image_required,
        "present": image_sbom_path.is_file(),
        "status": image_sbom_status(image_required, image_sbom_path.is_file()),
        "applicability": image_applicability,
        "applicability_reason": image_applicability_reason,
        "sha256": sha256(image_sbom_path) if image_sbom_path.is_file() else None,
        "bytes": file_size(image_sbom_path),
        "component_count": image_component_count,
        "sources": image_sources,
        "scanner": "syft" if image_sources else None,
        "required_by_release_mode": image_required_by_policy,
    },
]

manifest = {
    "command": "generate-sbom-evidence",
    "ok": all(not artifact["required"] or artifact["present"] for artifact in artifacts),
    "generated_at_utc": generated_at,
    "artifacts": artifacts,
    "inputs": {
        "frontend_lockfiles": [rel(path) for path in frontend_lockfiles],
        "image_sources": image_sources,
        "image_required_by_release_mode": image_required_by_policy,
        "image_applicability": image_applicability,
    },
}
write_json(manifest_path, manifest)
print(f"SBOM evidence written to {rel(manifest_path)}")
PY
}

# --- SBOM evidence ----------------------------------------------------------

run_check "configure dependency advisory Cargo home" configure_dependency_cargo_home
run_check "ensure python3" ensure_python3
run_check "ensure npm for frontend dependency audit" ensure_npm_for_frontend_audit
run_check "ensure syft for deployable image SBOM" ensure_syft
run_check "generate CycloneDX SBOM evidence" generate_sbom_evidence

# --- formatting -------------------------------------------------------------

rustup component add rustfmt >/dev/null 2>&1 || true
run_check "cargo fmt --all --check" cargo fmt --all --check

# --- lint -------------------------------------------------------------------

rustup component add clippy >/dev/null 2>&1 || true
run_check "cargo clippy --workspace --all-targets --locked -- -D warnings" \
  cargo clippy --workspace --all-targets --locked -- -D warnings

# --- vulnerability scan (RUSTSEC) ------------------------------------------

run_check "install cargo-audit" ensure_tool cargo-audit cargo-audit
run_check "cargo audit" cargo audit

# --- advisories + licenses + bans + sources --------------------------------

run_check "install cargo-deny" ensure_tool cargo-deny cargo-deny
run_check "cargo deny --all-features check advisories licenses bans sources" \
  cargo deny --all-features check advisories licenses bans sources

# --- retained dependency advisory / upgrade evidence ------------------------

run_check "scripts/appfw dependency-check --json --strict" \
  "$repo_root/scripts/appfw" dependency-check --json --strict

# --- PHI / PII log-leak guard ----------------------------------------------

run_check "scripts/ci/phi-log-lint.sh" bash "$script_dir/phi-log-lint.sh"

write_evidence true
log "Supply-chain evidence written to $report_file"
log "Supply-chain & lint gate passed"
