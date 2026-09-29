#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
base_ref=${APPFW_PACKAGE_TEST_BASE_REF:-a5da9fa20c6f1adc95dd226b82944aa201ccfa68}
work_root=${APPFW_PACKAGE_TEST_ROOT:-"${TMPDIR:-/tmp}/appfw-package-first-second-consumer"}
report_path=${APPFW_PACKAGE_TEST_REPORT:-"$repo_root/target/appfw/package-first-second-consumer.json"}
phase=${1:-all}
expected_before_version=0.1.5
expected_after_version=0.1.6
profile=second-consumer
consumer_root="$work_root/consumer"
evidence_root="$work_root/evidence"
logs_root="$work_root/logs"
completion_root="$work_root/completions"
attempt_root="$work_root/attempts"
harness_owner="$work_root/.package-first-second-consumer-owner"

die() {
  printf 'package-first second-consumer proof failed: %s\n' "$*" >&2
  exit 1
}

trap 'exit 143' TERM

for command in git python3 rg shasum tar; do
  command -v "$command" >/dev/null || die "$command is required"
done

case "$phase" in
  before-package | after-package | consumer-report | all) ;;
  *) die "unknown phase '$phase'; expected before-package, after-package, consumer-report, or all" ;;
esac

head_ref=$(git -C "$repo_root" rev-parse "${APPFW_PACKAGE_TEST_HEAD_REF:-HEAD}^{commit}")
actual_head=$(git -C "$repo_root" rev-parse HEAD)
base_ref=$(git -C "$repo_root" rev-parse "$base_ref^{commit}")
test "$actual_head" = "$head_ref" ||
  die "source checkout is $actual_head, expected exact committed source $head_ref"
test -z "$(git -C "$repo_root" status --short --untracked-files=no)" ||
  die "tracked source must be committed before package identities can be proved"
test "$head_ref" != "$base_ref" || die "HEAD must contain the package behavior upgrade"

initialize_work_root() {
  test ! -e "$work_root" || die "before-package requires a new work root: $work_root"
  mkdir -p \
    "$work_root/sources" \
    "$work_root/packages" \
    "$completion_root" \
    "$attempt_root" \
    "$evidence_root" \
    "$logs_root" \
    "$(dirname "$report_path")"
  printf '%s\n%s\n' "$head_ref" "$base_ref" >"$harness_owner"
}

require_work_root() {
  test -f "$harness_owner" || die "work root is not owned by this harness: $work_root"
  test "$(sed -n '1p' "$harness_owner")" = "$head_ref" ||
    die "work root belongs to a different head identity"
  test "$(sed -n '2p' "$harness_owner")" = "$base_ref" ||
    die "work root belongs to a different base identity"
}

materialize_source() {
  local label=$1
  local ref=$2
  local destination=$3
  test "$destination" = "$work_root/sources/$label" ||
    die "refusing unexpected disposable source target: $destination"
  test ! -e "$destination" || die "$label disposable source target already exists"
  test ! -e "$work_root/sources/$label.owner" ||
    die "$label disposable source ownership marker already exists"
  printf '%s\n' "$ref" >"$work_root/sources/$label.owner"
  mkdir -p "$destination"
  git -C "$repo_root" archive "$ref" | tar -x -C "$destination"
  test ! -e "$destination/.git" || die "source snapshot unexpectedly contains .git: $destination"
}

dispose_source() {
  local label=$1
  local expected_ref=$2
  local destination="$work_root/sources/$label"
  local owner="$work_root/sources/$label.owner"
  test "$destination" = "$work_root/sources/$label" ||
    die "refusing unexpected disposable source cleanup target: $destination"
  test -d "$destination" && test ! -L "$destination" ||
    die "$label disposable source target is missing or unsafe"
  test -f "$owner" && test "$(cat "$owner")" = "$expected_ref" ||
    die "$label disposable source ownership marker is missing or mismatched"
  rm -rf "$destination"
  test ! -e "$destination" || die "$label disposable source target was not removed"
}

dispose_incomplete_consumer_phase() {
  local destination
  for destination in \
    "$consumer_root" \
    "$work_root/consumer-cargo-home" \
    "$work_root/consumer-cargo-target" \
    "$work_root/consumer-packages"; do
    test -e "$destination" || continue
    test -d "$destination" && test ! -L "$destination" ||
      die "refusing unsafe incomplete consumer-phase cleanup target: $destination"
    rm -rf "$destination"
    test ! -e "$destination" ||
      die "incomplete consumer-phase cleanup did not remove $destination"
  done
}

sha256_file() {
  shasum -a 256 "$1" | awk '{print $1}'
}

write_package_completion() {
  local label=$1
  local source_ref=$2
  local expected_version=$3
  local elapsed_seconds=$4
  local completion="$completion_root/$label-package.complete.json"
  test ! -e "$completion" || die "$label package completion already exists"
  python3 - "$completion" "$label" "$source_ref" "$head_ref" "$base_ref" \
    "$expected_version" "$elapsed_seconds" "$work_root" <<'PY'
import hashlib
import json
import os
import pathlib
import sys

(
    completion_path,
    label,
    source_ref,
    harness_head,
    harness_base,
    expected_version,
    elapsed_seconds,
    work_root,
) = sys.argv[1:]
root = pathlib.Path(work_root)
evidence = root / "evidence"
archive = root / "packages" / f"{label}.tar.gz"
files = {
    "archive": archive,
    "package_sha256": evidence / f"{label}-package.sha256",
    "package_build_manifest": evidence / f"{label}-package-build-manifest.json",
    "package_identity": evidence / f"{label}-package-identity.json",
    "delivery_profiles_sha256": evidence / f"{label}-delivery-profiles.sha256",
    "package_first": evidence / f"{label}-package-first.json",
    "compatibility": evidence / f"{label}-compatibility.json",
}
if label == "after":
    files.update(
        {
            "mismatch_compatibility": evidence / "mismatch-compatibility.json",
            "mismatch_stderr": evidence / "mismatch-compatibility.stderr",
            "mismatch_exit_code": evidence / "mismatch-compatibility.exit-code",
            "same_version_compatibility": evidence / "same-version-different-content-compatibility.json",
        }
    )
for name, path in files.items():
    if not path.is_file():
        raise SystemExit(f"{label} package completion is missing {name}: {path}")
identity = json.loads(files["package_identity"].read_text(encoding="utf-8"))
if identity.get("version") != expected_version:
    raise SystemExit(
        f"{label} package identity is {identity.get('version')}, expected {expected_version}"
    )
hashes = {
    name: {
        "path": str(path),
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
    }
    for name, path in files.items()
}
if files["package_sha256"].read_text(encoding="utf-8").strip() != hashes["archive"]["sha256"]:
    raise SystemExit(f"{label} retained package checksum does not match the archive")
payload = {
    "schema_version": 1,
    "phase": f"{label}-package",
    "status": "complete",
    "source_ref": source_ref,
    "harness_head": harness_head,
    "harness_base": harness_base,
    "package_version": expected_version,
    "package_sha256": hashes["archive"]["sha256"],
    "elapsed_seconds": int(elapsed_seconds),
    "artifacts": hashes,
}
path = pathlib.Path(completion_path)
temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
temporary.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
os.replace(temporary, path)
PY
}

verify_package_completion() {
  local label=$1
  local expected_ref=$2
  local expected_version=$3
  local completion="$completion_root/$label-package.complete.json"
  python3 - "$completion" "$label" "$expected_ref" "$head_ref" "$base_ref" \
    "$expected_version" "$work_root" <<'PY'
import hashlib
import json
import pathlib
import sys

completion_path, label, expected_ref, harness_head, harness_base, expected_version, work_root = sys.argv[1:]
path = pathlib.Path(completion_path)
if not path.is_file():
    raise SystemExit(f"{label} package completion is missing: {path}")
data = json.loads(path.read_text(encoding="utf-8"))
expected = {
    "phase": f"{label}-package",
    "status": "complete",
    "source_ref": expected_ref,
    "harness_head": harness_head,
    "harness_base": harness_base,
    "package_version": expected_version,
}
for field, value in expected.items():
    if data.get(field) != value:
        raise SystemExit(
            f"{label} package completion {field} is {data.get(field)!r}, expected {value!r}"
        )
artifacts = data.get("artifacts")
if not isinstance(artifacts, dict) or not artifacts:
    raise SystemExit(f"{label} package completion has no retained artifact hashes")
for name, record in artifacts.items():
    if not isinstance(record, dict) or set(record) != {"path", "sha256"}:
        raise SystemExit(f"{label} package completion artifact {name} is malformed")
    artifact = pathlib.Path(record["path"])
    try:
        artifact.relative_to(pathlib.Path(work_root))
    except ValueError:
        raise SystemExit(f"{label} package completion artifact escaped work root: {artifact}")
    if not artifact.is_file():
        raise SystemExit(f"{label} package completion artifact is missing: {artifact}")
    actual = hashlib.sha256(artifact.read_bytes()).hexdigest()
    if actual != record["sha256"]:
        raise SystemExit(f"{label} package completion artifact hash mismatch: {artifact}")
if data.get("package_sha256") != artifacts.get("archive", {}).get("sha256"):
    raise SystemExit(f"{label} package completion archive identity is mismatched")
checksum_path = pathlib.Path(artifacts["package_sha256"]["path"])
if checksum_path.read_text(encoding="utf-8").strip() != data.get("package_sha256"):
    raise SystemExit(f"{label} package completion checksum content is mismatched")
if (pathlib.Path(work_root) / "sources" / label).exists():
    raise SystemExit(f"{label} disposable source target still exists")
PY
}

build_package() {
  local label=$1
  local source_root=$2
  local log="$logs_root/${label}-package.log"
  (
    cd "$source_root"
    scripts/appfw framework package --json
  ) >"$log" 2>&1 || {
    cat "$log" >&2
    die "$label package build failed; retain $work_root and correct package source, not the consumer"
  }

  local manifest="$source_root/target/appfw/proget/app-framework-proget-manifest.json"
  local archive
  archive=$(python3 - "$manifest" <<'PY'
import json
import pathlib
import sys

manifest = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
matches = [item["path"] for item in manifest["artifacts"] if item["name"] == "app-framework-toolchain"]
if len(matches) != 1:
    raise SystemExit("package manifest must contain exactly one app-framework-toolchain artifact")
print(matches[0])
PY
  )
  test -f "$archive" || die "$label package archive was not produced: $archive"
  cp "$manifest" "$evidence_root/${label}-package-build-manifest.json"
  cp "$archive" "$work_root/packages/${label}.tar.gz"
  shasum -a 256 "$work_root/packages/${label}.tar.gz" | awk '{print $1}' \
    >"$evidence_root/${label}-package.sha256"
}

install_package() {
  local label=$1
  local install_root=${2:-"$work_root/packages/$label"}
  test ! -e "$install_root" || die "$label package install target already exists: $install_root"
  mkdir -p "$install_root"
  tar -xzf "$work_root/packages/${label}.tar.gz" -C "$install_root" --strip-components=1
  test ! -e "$install_root/.git" || die "$label package contains forbidden .git metadata"
  test -x "$install_root/bin/appfw" || die "$label package does not contain executable bin/appfw"
  test -f "$install_root/docs/start/delivery-profiles.json" ||
    die "$label package omits the shipped handoff delivery-profile dependency"
  local delivery_profiles_sha
  delivery_profiles_sha=$(sha256_file "$install_root/docs/start/delivery-profiles.json")
  if test -f "$evidence_root/${label}-delivery-profiles.sha256"; then
    test "$(cat "$evidence_root/${label}-delivery-profiles.sha256")" = "$delivery_profiles_sha" ||
      die "$label delivery-profile dependency differs across immutable package extraction"
  else
    printf '%s\n' "$delivery_profiles_sha" >"$evidence_root/${label}-delivery-profiles.sha256"
  fi
  if test -f "$evidence_root/${label}-package-identity.json"; then
    cmp -s "$install_root/app-framework-package.json" \
      "$evidence_root/${label}-package-identity.json" ||
      die "$label installed package identity differs from retained evidence"
  else
    cp "$install_root/app-framework-package.json" \
      "$evidence_root/${label}-package-identity.json"
  fi
  printf '%s\n' "$install_root"
}

package_version() {
  python3 - "$1/app-framework-package.json" <<'PY'
import json
import pathlib
import sys

print(json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))["version"])
PY
}

inspect_package() {
  local label=$1
  local package_root=$2
  "$package_root/bin/appfw" lifecycle package-first \
    --package-root "$package_root" \
    --consumer "$profile" \
    >"$evidence_root/${label}-package-first.json"
}

check_compatibility() {
  local label=$1
  local package_root=$2
  local expected=$3
  "$package_root/bin/appfw" lifecycle compatibility \
    --package-root "$package_root" \
    --expected-version "$expected" \
    >"$evidence_root/${label}-compatibility.json"
}

check_mismatch_diagnostic() {
  local package_root=$1
  local expected=$2
  local status
  set +e
  "$package_root/bin/appfw" lifecycle compatibility \
    --package-root "$package_root" \
    --expected-version "$expected" \
    >"$evidence_root/mismatch-compatibility.json" \
    2>"$evidence_root/mismatch-compatibility.stderr"
  status=$?
  set -e
  test "$status" -ne 0 || die "compatibility mismatch unexpectedly passed"
  printf '%s\n' "$status" >"$evidence_root/mismatch-compatibility.exit-code"
  python3 - "$evidence_root/mismatch-compatibility.json" \
    "$evidence_root/mismatch-compatibility.stderr" <<'PY'
import json
import pathlib
import sys

report = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
stderr = pathlib.Path(sys.argv[2]).read_text(encoding="utf-8")
if report.get("compatible") is not False:
    raise SystemExit("mismatch report did not retain compatible:false")
if not report.get("failure_guidance"):
    raise SystemExit("mismatch report omitted failure guidance")
if "package version is incompatible; see failure_guidance" not in stderr:
    raise SystemExit("mismatch stderr did not route the diagnostic to failure guidance")
PY
}

check_same_version_distinct_content() {
  local package_root=$1
  local first="$work_root/packages/same-version-a"
  local second="$work_root/packages/same-version-b"
  test ! -e "$first" && test ! -e "$second" || die "same-version proof roots already exist"
  cp -R "$package_root" "$first"
  cp -R "$package_root" "$second"
  printf 'independently distinct same-version package content\n' >"$second/README.md"
  "$first/bin/appfw" lifecycle compatibility --package-root "$first" --expected-version "$expected_after_version" >"$evidence_root/same-version-a-compatibility.json"
  "$second/bin/appfw" lifecycle compatibility --package-root "$second" --expected-version "$expected_after_version" >"$evidence_root/same-version-b-compatibility.json"
  python3 - "$first" "$second" "$evidence_root/same-version-different-content-compatibility.json" <<'PY'
import hashlib, json, pathlib, sys
first, second, output = map(pathlib.Path, sys.argv[1:])
def digest(root):
    h = hashlib.sha256()
    for path in sorted(root.rglob('*')):
        if path.is_file():
            h.update(str(path.relative_to(root)).encode())
            h.update(path.read_bytes())
    return h.hexdigest()
left, right = digest(first), digest(second)
if left == right:
    raise SystemExit('same-version fixture content hashes are not distinct')
reports = [json.loads((root / 'app-framework-package.json').read_text()) for root in (first, second)]
payload = {"compatible": True, "compatibility_scope": "package-version-only", "package_versions": [item['version'] for item in reports], "package_content_sha256": [left, right], "archive_identity_equivalent": False}
pathlib.Path(output).write_text(json.dumps(payload, indent=2) + '\n')
PY
}

create_consumer() {
  local package_root=$1
  local phase_started=$SECONDS
  (
    cd "$work_root"
    "$package_root/bin/appfw" --app-root "$consumer_root" --framework-root "$package_root" product new \
      "$consumer_root" --from current --profile "$profile" --json
  ) >"$logs_root/create.log" 2>&1 || {
    cat "$logs_root/create.log" >&2
    die "package-first consumer create failed"
  }
  printf '%s\n' "$((SECONDS - phase_started))" >"$evidence_root/create-elapsed-seconds"
  test ! -e "$consumer_root/.git" || die "external consumer unexpectedly contains .git"
}

run_product_phase() {
  local label=$1
  local package_root=$2
  local phase_started=$SECONDS
  local log="$logs_root/${label}-product.log"
  (
    cd "$consumer_root"
    export APPFW_FRAMEWORK_ROOT="$package_root"
    export CARGO_HOME="$work_root/consumer-cargo-home"
    export CARGO_TARGET_DIR="$work_root/consumer-cargo-target"
    scripts/appfw product validate --json
    scripts/appfw product generate
    scripts/appfw product generate --check --json
    scripts/appfw product test --fast
    scripts/appfw product lock --write --json
    scripts/appfw product compat-verify --json
    scripts/appfw product handoff --json
  ) >"$log" 2>&1 || {
    cat "$log" >&2
    die "$label product lifecycle failed with zero retries and no manual repair"
  }

  test -f "$consumer_root/appfw.lock" || die "$label product lifecycle did not retain appfw.lock"
  test -f "$consumer_root/.appfw/target/appfw/generator_ir.json" ||
    die "$label product lifecycle did not retain generator_ir.json"
  test -f "$consumer_root/.appfw/target/appfw/artifacts.json" ||
    die "$label product lifecycle did not retain artifacts.json"
  test -f "$consumer_root/backend/src/schemas/consumer.rs" ||
    die "$label product lifecycle did not generate the consumer schema"
  cp "$consumer_root/appfw.lock" "$evidence_root/${label}-appfw.lock"
  cp "$consumer_root/.appfw/target/appfw/generator_ir.json" \
    "$evidence_root/${label}-generator-ir.json"
  cp "$consumer_root/.appfw/target/appfw/artifacts.json" \
    "$evidence_root/${label}-artifacts.json"
  cp "$consumer_root/backend/src/schemas/consumer.rs" \
    "$evidence_root/${label}-consumer-schema.rs"
  python3 - "$consumer_root/target/appfw/agent-handoff.json" "$package_root/app-framework-package.json" <<'PY'
import json
import pathlib
import sys

handoff = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
identity = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding="utf-8"))
delivery = handoff.get("delivery_profile")
if not isinstance(delivery, dict):
    raise SystemExit("package-only handoff omitted delivery profile")
if delivery.get("state") != "package-no-git-default":
    raise SystemExit("package-only handoff did not retain package no-Git state")
if delivery.get("source_status") != "package-no-git":
    raise SystemExit("package-only handoff did not retain package no-Git provenance")
if delivery.get("package_identity") != identity:
    raise SystemExit("package-only handoff did not retain package identity")
for unavailable in ("source_sha", "current_sha", "dirty", "stale"):
    if unavailable in delivery:
        raise SystemExit(f"package-only handoff claimed unavailable Git fact {unavailable}")
PY
  printf '%s\n' "$((SECONDS - phase_started))" >"$evidence_root/${label}-elapsed-seconds"
}

run_before_baseline_diagnostic() {
  local package_root=$1
  run_product_phase before_baseline_diagnostic "$package_root"
}

record_upgrade() {
  local label=$1
  local command_package=$2
  local from_package=$3
  local to_package=$4
  "$command_package/bin/appfw" lifecycle upgrade \
    --from-package-root "$from_package" \
    --to-package-root "$to_package" \
    --consumer-root "$consumer_root" \
    >"$evidence_root/${label}-lifecycle-upgrade.json"
  cp "$consumer_root/.appfw/package-first-upgrade.json" \
    "$evidence_root/${label}-consumer-upgrade-state.json"
}

verify_lifecycle_model_transition() {
  local label=$1
  local package_root=$2
  local state="$evidence_root/${label}-consumer-upgrade-state.json"
  local source="$package_root/app_gen/_golden/downstream_apps/$profile/starter/.appfw/model/schemas/consumer/entity_types/work_item.yaml"
  local target="$consumer_root/.appfw/model/schemas/consumer/entity_types/work_item.yaml"
  test -f "$state" || die "$label lifecycle state is missing"
  test -f "$source" || die "$label package WorkItem source is missing"
  test -f "$target" || die "$label consumer WorkItem target is missing"
  cmp -s "$source" "$target" || die "$label lifecycle did not apply the package WorkItem source exactly"
  local source_sha
  source_sha=$(sha256_file "$source")
  python3 - "$state" "$source_sha" <<'PY'
import json
import pathlib
import sys

state = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
expected_sha = sys.argv[2]
transition = state.get("model_transition")
expected = {
    "source": "app_gen/_golden/downstream_apps/second-consumer/starter/.appfw/model/schemas/consumer/entity_types/work_item.yaml",
    "target": ".appfw/model/schemas/consumer/entity_types/work_item.yaml",
    "applied_sha256": expected_sha,
}
if transition != expected:
    raise SystemExit(f"lifecycle model transition is invalid: {transition!r}")
PY
}

run_before_package_phase() {
  initialize_work_root
  local phase_started=$SECONDS
  materialize_source before "$base_ref" "$work_root/sources/before"
  build_package before "$work_root/sources/before"
  local before_package
  local before_version
  before_package=$(install_package before)
  before_version=$(package_version "$before_package")
  test "$before_version" = "$expected_before_version" ||
    die "before package identity is $before_version, expected $expected_before_version"
  inspect_package before "$before_package"
  check_compatibility before "$before_package" "$expected_before_version"
  dispose_source before "$base_ref"
  write_package_completion before "$base_ref" "$expected_before_version" \
    "$((SECONDS - phase_started))"
  verify_package_completion before "$base_ref" "$expected_before_version"
}

run_after_package_phase() {
  require_work_root
  verify_package_completion before "$base_ref" "$expected_before_version"
  test ! -e "$completion_root/after-package.complete.json" ||
    die "after package completion already exists"
  test ! -e "$work_root/sources/after" ||
    die "after disposable source target already exists"
  test ! -e "$work_root/packages/after.tar.gz" ||
    die "after package archive already exists"
  local phase_started=$SECONDS
  materialize_source after "$head_ref" "$work_root/sources/after"
  build_package after "$work_root/sources/after"
  local after_package
  local after_version
  after_package=$(install_package after)
  after_version=$(package_version "$after_package")
  test "$after_version" = "$expected_after_version" ||
    die "after package identity is $after_version, expected $expected_after_version"
  test "$(cat "$evidence_root/before-package.sha256")" != \
    "$(cat "$evidence_root/after-package.sha256")" || die "package checksums are equal"
  inspect_package after "$after_package"
  check_compatibility after "$after_package" "$expected_after_version"
  check_mismatch_diagnostic "$after_package" "$expected_before_version"
  check_same_version_distinct_content "$after_package"
  dispose_source after "$head_ref"
  write_package_completion after "$head_ref" "$expected_after_version" \
    "$((SECONDS - phase_started))"
  verify_package_completion after "$head_ref" "$expected_after_version"
}

run_consumer_report_phase() {
  require_work_root
  verify_package_completion before "$base_ref" "$expected_before_version"
  verify_package_completion after "$head_ref" "$expected_after_version"
  test ! -e "$report_path" || die "consumer report already exists: $report_path"
  test -z "$(find "$attempt_root" -maxdepth 1 -type f -name '*.json' -print -quit)" ||
    die "strict one-attempt consumer proof refuses an existing attempt marker"
  test ! -e "$consumer_root" || die "consumer workspace already exists: $consumer_root"
  test ! -e "$work_root/consumer-cargo-home" ||
    die "consumer Cargo home already exists; cache reuse is unverified"
  test ! -e "$work_root/consumer-cargo-target" ||
    die "consumer Cargo target already exists; cache reuse is unverified"
  test ! -e "$work_root/consumer-packages" ||
    die "consumer package roots already exist; package reuse is unverified"
  test ! -e "$work_root/sources/before" && test ! -e "$work_root/sources/after" ||
    die "disposable package source targets were not removed"
  python3 - "$attempt_root/0001-consumer-report.json" "$head_ref" "$base_ref" "$phase" \
    "$consumer_root" "$work_root/consumer-cargo-home" "$work_root/consumer-cargo-target" "$work_root/consumer-packages" <<'PY'
import datetime
import json
import pathlib
import sys

marker, head, base, phase, *paths = sys.argv[1:]
path = pathlib.Path(marker)
if path.exists():
    raise SystemExit("attempt marker already exists")
payload = {
    "schema_version": 1,
    "sequence": 1,
    "phase": phase,
    "head_sha": head,
    "base_sha": base,
    "started_at": datetime.datetime.now(datetime.UTC).isoformat(),
    "preexisting_paths": {value: pathlib.Path(value).exists() for value in paths},
    "cleanup_performed": False,
}
path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY
  mkdir -p \
    "$work_root/consumer-cargo-home" \
    "$work_root/consumer-cargo-target" \
    "$(dirname "$report_path")"

  local phase_started=$SECONDS
  local before_package
  local after_package
  local before_version
  local after_version
  before_package=$(install_package before "$work_root/consumer-packages/before")
  after_package=$(install_package after "$work_root/consumer-packages/after")
  before_version=$(package_version "$before_package")
  after_version=$(package_version "$after_package")
  test "$before_version" = "$expected_before_version" ||
    die "before package identity is $before_version, expected $expected_before_version"
  test "$after_version" = "$expected_after_version" ||
    die "after package identity is $after_version, expected $expected_after_version"
  test "$(sha256_file "$work_root/packages/before.tar.gz")" != \
    "$(sha256_file "$work_root/packages/after.tar.gz")" || die "package checksums are equal"

  create_consumer "$before_package"
  run_before_baseline_diagnostic "$before_package"

  record_upgrade forward "$after_package" "$before_package" "$after_package"
  verify_lifecycle_model_transition forward "$after_package"
  run_product_phase forward_successor_handoff "$after_package"

  diff -u "$evidence_root/before_baseline_diagnostic-appfw.lock" "$evidence_root/forward_successor_handoff-appfw.lock" \
    >"$evidence_root/appfw-lock-upgrade.diff" || true
  diff -u "$evidence_root/before_baseline_diagnostic-generator-ir.json" "$evidence_root/forward_successor_handoff-generator-ir.json" \
    >"$evidence_root/generated-contract-upgrade.diff" || true
  diff -u "$evidence_root/before_baseline_diagnostic-consumer-schema.rs" "$evidence_root/forward_successor_handoff-consumer-schema.rs" \
    >"$evidence_root/generated-schema-upgrade.diff" || true

  record_upgrade rollback "$after_package" "$after_package" "$before_package"
  record_upgrade recovery "$after_package" "$before_package" "$after_package"
  verify_lifecycle_model_transition recovery "$after_package"
  run_product_phase recovery_successor_handoff "$after_package"

  diff -u "$evidence_root/forward_successor_handoff-appfw.lock" "$evidence_root/recovery_successor_handoff-appfw.lock" \
    >"$evidence_root/recovery-appfw-lock.raw.diff" || true
  diff -u "$evidence_root/forward_successor_handoff-artifacts.json" "$evidence_root/recovery_successor_handoff-artifacts.json" \
    >"$evidence_root/recovery-artifacts.raw.diff" || true

  local package_elapsed
  package_elapsed=$(python3 - \
    "$completion_root/before-package.complete.json" \
    "$completion_root/after-package.complete.json" <<'PY'
import json
import pathlib
import sys

print(
    sum(
        int(json.loads(pathlib.Path(path).read_text(encoding="utf-8"))["elapsed_seconds"])
        for path in sys.argv[1:]
    )
)
PY
  )
  printf '%s\n' "$((package_elapsed + SECONDS - phase_started))" \
    >"$evidence_root/total-elapsed-seconds"

  python3 - "$report_path" "$base_ref" "$head_ref" "$work_root" "$attempt_root" \
  "$expected_before_version" "$expected_after_version" <<'PY'
import hashlib
import json
import os
import pathlib
import sys
import tomllib

report_path, before_source, after_source, work_root, attempt_root, before_version, after_version = sys.argv[1:]
root = pathlib.Path(work_root)
evidence = root / "evidence"
consumer = root / "consumer"
attempts = sorted(pathlib.Path(attempt_root).glob("*.json"))
if len(attempts) != 1:
    raise SystemExit("strict consumer proof requires exactly one retained attempt marker")
attempt = json.loads(attempts[0].read_text(encoding="utf-8"))
if attempt.get("sequence") != 1 or attempt.get("head_sha") != after_source or attempt.get("base_sha") != before_source:
    raise SystemExit("consumer attempt marker does not bind the exact proof identities")
if any(attempt.get("preexisting_paths", {}).values()) or attempt.get("cleanup_performed") is not False:
    raise SystemExit("strict consumer proof cannot claim a fresh one-attempt run")

def read(name):
    return (evidence / name).read_text(encoding="utf-8").strip()

def load_json(name):
    return json.loads((evidence / name).read_text(encoding="utf-8"))

def load_lock(name):
    return tomllib.loads((evidence / name).read_text(encoding="utf-8"))

def work_item_property(name, property_name):
    data = load_json(name)
    for schema in data["schemas"]:
        if schema["name"] != "consumer":
            continue
        for entity in schema["entities"]:
            if entity["name"] == "WorkItem":
                return next(
                    (item for item in entity["native_properties"] if item["name"] == property_name),
                    None,
                )
    raise SystemExit(f"WorkItem was not found in {name}")

properties = {
    label: work_item_property(f"{label}-generator-ir.json", "is_complete")
    for label in ("before_baseline_diagnostic", "forward_successor_handoff", "recovery_successor_handoff")
}
if any(property is None for property in properties.values()):
    raise SystemExit("WorkItem.is_complete is missing from package containment proof")
if any(
    property["data_type"] != "Boolean" or property["is_required"] is not True
    for property in properties.values()
):
    raise SystemExit("WorkItem.is_complete changed during package containment proof")

baseline_lock = load_lock("before_baseline_diagnostic-appfw.lock")
forward_lock = load_lock("forward_successor_handoff-appfw.lock")
recovery_lock = load_lock("recovery_successor_handoff-appfw.lock")
normalized_lock_fields = {"last_generated_at", "artifact_manifest_hash"}
stable_forward = {key: value for key, value in forward_lock.items() if key not in normalized_lock_fields}
stable_recovery = {key: value for key, value in recovery_lock.items() if key not in normalized_lock_fields}
if stable_forward != stable_recovery:
    raise SystemExit("recovery appfw.lock did not return to the successor package contract")
raw_lock_diff = (evidence / "recovery-appfw-lock.raw.diff").read_text(encoding="utf-8")
for line in raw_lock_diff.splitlines():
    if line.startswith(("---", "+++", "@@")) or not line.startswith(("-", "+")):
        continue
    changed_field = line[1:].split("=", 1)[0].strip()
    if changed_field not in normalized_lock_fields:
        raise SystemExit(f"recovery appfw.lock raw diff changed unauthorized field: {changed_field}")

forward_artifacts = load_json("forward_successor_handoff-artifacts.json")
recovery_artifacts = load_json("recovery_successor_handoff-artifacts.json")
if not isinstance(forward_artifacts, list) or not isinstance(recovery_artifacts, list):
    raise SystemExit("artifacts manifests must be JSON arrays")

def normalize_artifacts(records):
    normalized = []
    for record in records:
        if not isinstance(record, dict):
            raise SystemExit("artifact manifest contains a non-object record")
        for required in ("path", "ownership", "overwrite", "content_sha256"):
            if required not in record:
                raise SystemExit(f"artifact record is missing required field: {required}")
        normalized.append({key: value for key, value in record.items() if key != "action"})
    return sorted(normalized, key=lambda record: json.dumps(record, sort_keys=True))

if normalize_artifacts(forward_artifacts) != normalize_artifacts(recovery_artifacts):
    raise SystemExit("recovery artifact manifest changed beyond per-record action")

forward = load_json("forward-lifecycle-upgrade.json")
forward_state = load_json("forward-consumer-upgrade-state.json")
recovery = load_json("recovery-lifecycle-upgrade.json")
recovery_state = load_json("recovery-consumer-upgrade-state.json")
if (forward.get("from_version"), forward.get("to_version"), forward.get("upgraded")) != (
    before_version,
    after_version,
    True,
):
    raise SystemExit("forward lifecycle upgrade record is invalid")
if (forward_state.get("from_version"), forward_state.get("to_version")) != (
    before_version,
    after_version,
):
    raise SystemExit("consumer upgrade state is invalid")
if (recovery.get("from_version"), recovery.get("to_version"), recovery.get("upgraded")) != (
    before_version,
    after_version,
    True,
):
    raise SystemExit("recovery lifecycle upgrade record is invalid")
if (
    recovery_state.get("from_version"),
    recovery_state.get("to_version"),
    recovery_state.get("source"),
) != (before_version, after_version, "package-first lifecycle"):
    raise SystemExit("recovery consumer upgrade state is invalid")

for label, expected in (("before", before_version), ("after", after_version)):
    compatibility = load_json(f"{label}-compatibility.json")
    if compatibility.get("actual_version") != expected or compatibility.get("compatible") is not True:
        raise SystemExit(f"{label} compatibility evidence is invalid")
mismatch = load_json("mismatch-compatibility.json")
if mismatch.get("compatible") is not False or not mismatch.get("failure_guidance"):
    raise SystemExit("mismatch compatibility evidence is not diagnostic")

before_sha = read("before-package.sha256")
after_sha = read("after-package.sha256")
if before_sha == after_sha:
    raise SystemExit("package identities have equal checksums")
delivery_profile_hashes = {
    label: read(f"{label}-delivery-profiles.sha256")
    for label in ("before", "after")
}
if any(len(value) != 64 or any(character not in "0123456789abcdef" for character in value)
       for value in delivery_profile_hashes.values()):
    raise SystemExit("delivery-profile dependency hash is invalid")
if any(path.name == ".git" for path in consumer.rglob(".git")):
    raise SystemExit("external consumer contains forbidden Git metadata")
if any("is_complete" not in read(f"{label}-consumer-schema.rs")
       for label in ("before_baseline_diagnostic", "forward_successor_handoff", "recovery_successor_handoff")):
    raise SystemExit("generated Rust schema changed during package containment proof")

def completion_identity_rows(root_path, status, purpose, supersession_reason, report_path=None):
    rows = []
    for label in ("before", "after"):
        completion_path = root_path / "completions" / f"{label}-package.complete.json"
        if not completion_path.is_file():
            raise SystemExit(f"retained completion is missing: {completion_path}")
        completion = json.loads(completion_path.read_text(encoding="utf-8"))
        source_sha = completion.get("source_ref")
        archive_sha256 = completion.get("package_sha256")
        package_version = completion.get("package_version")
        archive = completion.get("artifacts", {}).get("archive", {})
        archive_path = pathlib.Path(archive.get("path", ""))
        if not isinstance(source_sha, str) or len(source_sha) != 40:
            raise SystemExit(f"retained completion has an invalid source SHA: {completion_path}")
        if not isinstance(archive_sha256, str) or len(archive_sha256) != 64:
            raise SystemExit(f"retained completion has an invalid archive SHA-256: {completion_path}")
        if not isinstance(package_version, str) or not package_version:
            raise SystemExit(f"retained completion has no package version: {completion_path}")
        if not archive_path.is_file() or hashlib.sha256(archive_path.read_bytes()).hexdigest() != archive_sha256:
            raise SystemExit(f"retained completion archive does not rehash: {completion_path}")
        row = {
            "source_sha": source_sha,
            "archive_sha256": archive_sha256,
            "package_version": package_version,
            "purpose": purpose,
            "status": status,
            "supersession_reason": supersession_reason,
            "completion_path": str(completion_path),
        }
        if report_path is not None:
            row["report_path"] = str(report_path)
        rows.append(row)
    return rows

historical_roots = [
    pathlib.Path("/private/tmp/afs-017-lifecycle-model-proof"),
    pathlib.Path("/private/tmp/afs-017-rebased-package-proof"),
    pathlib.Path("/private/tmp/afs-017-no-go-correction-proof"),
    pathlib.Path("/private/tmp/afs-017-retained-roots-413b9ce9"),
    pathlib.Path("/private/tmp/afs-017-source-enum-550f76929"),
]
historical_reports = [
    None,
    pathlib.Path("/private/tmp/afs-017-rebased-package-proof-report.json"),
    pathlib.Path("/private/tmp/afs-017-no-go-correction-proof-report.json"),
    pathlib.Path("/private/tmp/afs-017-retained-roots-413b9ce9-report.json"),
    pathlib.Path("/private/tmp/afs-017-source-enum-550f76929-report.json"),
]
for aggregate_root in sorted(pathlib.Path("/private/tmp").glob("afs-017-aggregate-*")):
    if aggregate_root != root:
        historical_roots.append(aggregate_root)
        report = aggregate_root / "report.json"
        historical_reports.append(report if report.is_file() else None)
if len(historical_roots) != len(historical_reports):
    raise SystemExit("historical package evidence inventory is inconsistent")
identity_reconciliation = []
for historical_root, historical_report in zip(historical_roots, historical_reports):
    if historical_report is not None and not historical_report.is_file():
        raise SystemExit(f"retained historical report is missing: {historical_report}")
    identity_reconciliation.extend(
        completion_identity_rows(
            historical_root,
            "superseded",
            "historical package identity proof",
            "superseded by a later retained correction proof",
            historical_report,
        )
    )
identity_reconciliation.extend(
    completion_identity_rows(
        root,
        "candidate",
        "exact-head package identity proof",
        "current exact-head correction proof pending promotion",
        pathlib.Path(report_path),
    )
)
seen_identity_pairs = set()
for row in identity_reconciliation:
    pair = (row["source_sha"], row["archive_sha256"])
    if pair in seen_identity_pairs:
        raise SystemExit(f"identity reconciliation duplicated an observed package build: {pair}")
    seen_identity_pairs.add(pair)
candidate_rows = [row for row in identity_reconciliation if row["status"] == "candidate"]
if len(candidate_rows) != 2:
    raise SystemExit("only the final exact-head package pair may be candidate")
if {row["source_sha"] for row in candidate_rows} != {before_source, after_source}:
    raise SystemExit("candidate package pair does not bind the exact base and head sources")

report = {
    "schema_version": 1,
    "phase": "consumer-report",
    "status": "complete",
    "ok": True,
    "claim": "local-only same-workspace package self-containment upgrade and recovery proof",
    "governing_decision": "target/appfw/afs-007-option-a-execution-and-afs-017-containment-po-decision.md",
    "nonclaims": [
        "publication",
        "ProGet or Nexus availability",
        "remote CI",
        "live provider behavior",
        "release or production readiness",
        "the prior 0.1.5 NO-GO proof used a harness-owned model copy; it is not a no-Git handoff failure",
    ],
    "same_consumer_workspace": True,
    "consumer_root": str(consumer),
    "consumer_has_framework_checkout": False,
    "consumer_has_git_metadata": False,
    "consumer_used_preexisting_cache": False,
    "consumer_cache_roots": {
        "cargo_home": str(root / "consumer-cargo-home"),
        "cargo_target": str(root / "consumer-cargo-target"),
    },
    "consumer_proof_history": {
        "attempt_markers": [str(path) for path in attempts],
        "attempt_count": len(attempts),
        "strict_one_attempt": len(attempts) == 1,
        "cleanup_performed": attempt["cleanup_performed"],
        "preexisting_paths": attempt["preexisting_paths"],
    },
    "elapsed_seconds": {
        "total": int(read("total-elapsed-seconds")),
        "create": int(read("create-elapsed-seconds")),
        "before_baseline_diagnostic": int(read("before_baseline_diagnostic-elapsed-seconds")),
        "forward_successor_handoff": int(read("forward_successor_handoff-elapsed-seconds")),
        "recovery_successor_handoff": int(read("recovery_successor_handoff-elapsed-seconds")),
    },
    "before_baseline_diagnostic": {
        "source_identity": before_source,
        "package_version": before_version,
        "package_sha256": before_sha,
        "appfw_lock_sha256": hashlib.sha256((evidence / "before_baseline_diagnostic-appfw.lock").read_bytes()).hexdigest(),
        "raw_log": str(root / "logs" / "before_baseline_diagnostic-product.log"),
        "observed_status": "success",
        "acceptance_credit": False,
        "prior_no_go_evidence": {
            "status": "NO-GO",
            "reason": "the earlier 0.1.5 proof harness copied the consumer WorkItem model",
            "diagnostic_only": True,
        },
    },
    "forward_successor_handoff": {
        "source_identity": after_source,
        "package_version": after_version,
        "package_sha256": after_sha,
        "appfw_lock_sha256": hashlib.sha256((evidence / "forward_successor_handoff-appfw.lock").read_bytes()).hexdigest(),
        "raw_log": str(root / "logs" / "forward_successor_handoff-product.log"),
    },
    "compatibility": {
        "after_expected_pass": True,
        "mismatch_diagnostic_fail": True,
        "mismatch_exit_code": int(read("mismatch-compatibility.exit-code")),
        "failure_guidance": mismatch["failure_guidance"],
    },
    "package_self_containment": {
        "required_dependency": "docs/start/delivery-profiles.json",
        "authority": "carried inside each immutable toolchain archive",
        "before_sha256": delivery_profile_hashes["before"],
        "after_sha256": delivery_profile_hashes["after"],
        "consumer_repair": False,
        "checkout_injection": False,
        "handoff_weakened": False,
    },
    "identity_reconciliation": identity_reconciliation,
    "upgrade": {
        "lifecycle_record": str(evidence / "forward-lifecycle-upgrade.json"),
        "consumer_state": str(evidence / "forward-consumer-upgrade-state.json"),
        "package_fixture_applied_by_harness_without_human_edit": False,
        "lifecycle_automatically_applies_consumer_model": True,
    },
    "preserved_generated_contract": {
        "entity": "WorkItem",
        "property": "is_complete",
        "baseline_forward_recovery": {"data_type": "Boolean", "required": True, "generated_rust_type": "bool"},
        "generated_contract_diff": str(evidence / "generated-contract-upgrade.diff"),
        "generated_schema_diff": str(evidence / "generated-schema-upgrade.diff"),
        "appfw_lock_diff": str(evidence / "appfw-lock-upgrade.diff"),
    },
    "recovery": {
        "from_version": before_version,
        "to_version": after_version,
        "lifecycle_record": str(evidence / "recovery-lifecycle-upgrade.json"),
        "consumer_state": str(evidence / "recovery-consumer-upgrade-state.json"),
        "successor_handoff": {
            "package_version": after_version,
            "raw_log": str(root / "logs" / "recovery_successor_handoff-product.log"),
        },
        "generated_contract_restored": True,
        "stable_appfw_lock_restored": True,
        "normalized_lock_fields": sorted(normalized_lock_fields),
        "raw_lock_diff": str(evidence / "recovery-appfw-lock.raw.diff"),
        "artifact_manifest_restored": True,
        "raw_artifact_diff": str(evidence / "recovery-artifacts.raw.diff"),
    },
    "failure_guidance": (
        "Retain the work root and logs; do not repair the consumer manually. "
        "Retain this proof root and its evidence; correct package source or lifecycle orchestration, then use a new unique proof root for a new attempt."
    ),
}
path = pathlib.Path(report_path)
reconciliation_path = path.with_name("afs-017-package-identity-reconciliation.md")
canonical_rows = json.dumps(identity_reconciliation, sort_keys=True, separators=(",", ":")).encode("utf-8")
table_digest = hashlib.sha256(canonical_rows).hexdigest()
identity_row_template = (
    "| {source_sha} | {archive_sha256} | {package_version} | {purpose} | {status} | "
    "{supersession_reason} | {completion_path} | {report_path} |"
)

def render_identity_row(row):
    values = dict(row)
    values.setdefault("report_path", "")
    return identity_row_template.format(**values)

rows_with_report = [row for row in identity_reconciliation if "report_path" in row]
rows_without_report = [row for row in identity_reconciliation if "report_path" not in row]
if not rows_with_report or not rows_without_report:
    raise SystemExit("identity reconciliation must retain rows with and without report references")
if rows_with_report[0]["report_path"] not in render_identity_row(rows_with_report[0]):
    raise SystemExit("identity row renderer did not retain a report reference")
if not render_identity_row(rows_without_report[0]).endswith(" |"):
    raise SystemExit("identity row renderer did not leave an absent report reference empty")

lines = [
    "# AFS-017 Package Identity Reconciliation",
    "",
    "This is a mechanical rendering of `identity_reconciliation` in the terminal package proof.",
    f"Source report: `{path}`",
    f"Canonical row digest: `sha256:{table_digest}`",
    "",
    "| Source SHA | Archive SHA-256 | Version | Purpose | Status | Supersession reason | Completion | Report |",
    "| --- | --- | --- | --- | --- | --- | --- | --- |",
]
for row in identity_reconciliation:
    lines.append(render_identity_row(row))
markdown_bytes = ("\n".join(lines) + "\n").encode("utf-8")
report_bytes = (json.dumps(report, indent=2) + "\n").encode("utf-8")
markdown_temporary = reconciliation_path.with_name(f".{reconciliation_path.name}.{os.getpid()}.tmp")
report_temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
markdown_temporary.write_bytes(markdown_bytes)
report_temporary.write_bytes(report_bytes)
os.replace(markdown_temporary, reconciliation_path)
os.replace(report_temporary, path)
if path.read_bytes() != report_bytes or reconciliation_path.read_bytes() != markdown_bytes:
    raise SystemExit("terminal package evidence did not round-trip byte-for-byte")
if hashlib.sha256(json.dumps(report["identity_reconciliation"], sort_keys=True, separators=(",", ":")).encode("utf-8")).hexdigest() != table_digest:
    raise SystemExit("terminal package evidence canonical row digest changed")
if len({(row["source_sha"], row["archive_sha256"]) for row in identity_reconciliation}) != len(identity_reconciliation):
    raise SystemExit("terminal package evidence identities are not unique")
if len([row for row in identity_reconciliation if row["status"] == "candidate"]) != 2:
    raise SystemExit("terminal package evidence must retain exactly two candidate rows")
print(json.dumps(report, separators=(",", ":")))
PY
}

case "$phase" in
  before-package)
    run_before_package_phase
    ;;
  after-package)
    run_after_package_phase
    ;;
  consumer-report)
    run_consumer_report_phase
    ;;
  all)
    run_before_package_phase
    run_after_package_phase
    run_consumer_report_phase
    ;;
esac
