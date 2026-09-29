#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
default_framework_root="$(cd "$script_dir/.." && pwd)"
default_app_root="$default_framework_root"
if [[ -f "$default_framework_root/examples/products/crm/.appfw/manifest.yaml" ]]; then
  default_app_root="$default_framework_root/examples/products/crm"
fi
app_root="$(cd "${APPFW_APP_ROOT:-$default_app_root}" && pwd)"
framework_root="$(cd "${APPFW_FRAMEWORK_ROOT:-$default_framework_root}" && pwd)"

app_gen_dir="${APPFW_GENERATOR_ROOT:-$framework_root/app_gen}"
config_root="${APPFW_CONFIG_ROOT:-$app_root/.appfw/model}"
templates_root="${APPFW_TEMPLATES_ROOT:-$app_gen_dir/_templates}"
report_root="${APPFW_REPORT_ROOT:-$app_root/.appfw/target/appfw}"
equivalence_target_dir="${APPFW_APPGEN_EQUIV_TARGET_DIR:-$framework_root/target/appfw/appgen-equivalence-target}"
equivalence_log_dir="${APPFW_APPGEN_EQUIV_LOG_DIR:-$framework_root/target/appfw}"
equivalence_jobs="${APPFW_APPGEN_EQUIV_JOBS:-2}"
equivalence_timing_file="${APPFW_APPGEN_EQUIV_TIMING_FILE:-$equivalence_log_dir/generate-check-timing.json}"

tmp_root="$(mktemp -d "${TMPDIR:-/tmp}/appgen-backend-equivalence.XXXXXX")"
work_app="$tmp_root/product"
work_framework="$tmp_root/framework"
log_file="$tmp_root/app_gen.log"
retained_log_file="$equivalence_log_dir/generate-check-app-gen.log"
APPGEN_EQUIV_PHASE_ITEMS=()

json_escape() {
  local value="$1"
  value="${value//\\/\\\\}"
  value="${value//\"/\\\"}"
  value="${value//$'\n'/\\n}"
  value="${value//$'\r'/\\r}"
  value="${value//$'\t'/\\t}"
  printf '"%s"' "$value"
}

ms_now() {
  python3 - <<'PY'
import time
print(int(time.time() * 1000))
PY
}

script_start_ms="$(ms_now)"

record_phase() {
  local name="$1"
  local elapsed_ms="$2"
  local ok="$3"
  local detail="$4"
  APPGEN_EQUIV_PHASE_ITEMS+=("${name}|${elapsed_ms}|${ok}|${detail}")
}

write_timing_artifact() {
  local status="$1"
  set +e
  mkdir -p "$(dirname "$equivalence_timing_file")"

  local total_wall_ms="$(( $(ms_now) - script_start_ms ))"
  local ok="false"
  if [[ "$status" == "0" ]]; then
    ok="true"
  fi

  local slowest_name=""
  local slowest_ms=0
  local item name elapsed_ms phase_ok detail
  for item in "${APPGEN_EQUIV_PHASE_ITEMS[@]}"; do
    IFS='|' read -r name elapsed_ms phase_ok detail <<<"$item"
    elapsed_ms="${elapsed_ms:-0}"
    if (( elapsed_ms > slowest_ms )); then
      slowest_ms="$elapsed_ms"
      slowest_name="$name"
    fi
  done

  {
    printf '{"command":"generate-check"'
    printf ',"ok":%s' "$ok"
    printf ',"total_wall_ms":%s' "$total_wall_ms"
    printf ',"phase_count":%s' "${#APPGEN_EQUIV_PHASE_ITEMS[@]}"
    printf ',"slowest_phase":'
    if [[ -n "$slowest_name" ]]; then
      printf '{"name":'
      json_escape "$slowest_name"
      printf ',"elapsed_ms":%s}' "$slowest_ms"
    else
      printf 'null'
    fi
    printf ',"phases":['
    local first=1
    for item in "${APPGEN_EQUIV_PHASE_ITEMS[@]}"; do
      IFS='|' read -r name elapsed_ms phase_ok detail <<<"$item"
      if [[ "$first" != "1" ]]; then
        printf ','
      fi
      printf '{"name":'
      json_escape "$name"
      printf ',"ok":%s' "$phase_ok"
      printf ',"elapsed_ms":%s' "$elapsed_ms"
      printf ',"detail":'
      json_escape "$detail"
      printf '}'
      first=0
    done
    printf ']}\n'
  } >"$equivalence_timing_file"
}

cleanup() {
  local status=$?
  write_timing_artifact "$status"
  if [[ "${KEEP_APPGEN_EQUIV_TMP:-}" == "1" ]]; then
    echo "Kept temp directory: $tmp_root"
  else
    rm -rf "$tmp_root"
  fi
}
trap cleanup EXIT

rebase_path() {
  local path="$1"
  local source_root="$2"
  local target_root="$3"

  case "$path" in
    "$source_root")
      printf '%s\n' "$target_root"
      ;;
    "$source_root"/*)
      printf '%s/%s\n' "$target_root" "${path#"$source_root"/}"
      ;;
    *)
      printf '%s\n' "$path"
      ;;
  esac
}

copy_root() {
  local source="$1"
  local target="$2"

  rsync -a \
    --exclude .git \
    --exclude .claude \
    --exclude .codex \
    --exclude .cache \
    --exclude .next \
    --exclude .parcel-cache \
    --exclude .qodo \
    --exclude .turbo \
    --exclude .venv \
    --exclude build \
    --exclude dist \
    --exclude node_modules \
    --exclude target \
    --exclude app_gen/target \
    --exclude backend/target \
    --exclude api_tests/target \
    --exclude database/target \
    --exclude vibe-od-equity-report \
    --exclude vibe-offer-letter \
    "$source/" "$target/"
}

copy_if_exists() {
  local source_root="$1"
  local target_root="$2"
  local rel_path="$3"

  if [[ -d "$source_root/$rel_path" ]]; then
    mkdir -p "$target_root/$rel_path"
    copy_root "$source_root/$rel_path" "$target_root/$rel_path"
  elif [[ -f "$source_root/$rel_path" ]]; then
    mkdir -p "$target_root/$(dirname "$rel_path")"
    cp "$source_root/$rel_path" "$target_root/$rel_path"
  fi
}

copy_product_equivalence_root() {
  local source="$1"
  local target="$2"
  local rel_path

  mkdir -p "$target"
  for rel_path in \
    ".appfw" \
    "backend" \
    "database" \
    "api_tests" \
    "podman-compose.yml" \
    "Cargo.toml" \
    "Cargo.lock" \
    "README.md"; do
    copy_if_exists "$source" "$target" "$rel_path"
  done
}

copy_cargo_member_stub() {
  local source_root="$1"
  local target_root="$2"
  local rel_path="$3"
  local child

  if [[ ! -d "$source_root/$rel_path" ]]; then
    return 0
  fi

  for child in Cargo.toml Cargo.lock build.rs src tests benches examples contracts; do
    copy_if_exists "$source_root" "$target_root" "$rel_path/$child"
  done
}

copy_framework_equivalence_root() {
  local source="$1"
  local target="$2"
  local rel_path

  mkdir -p "$target"
  for rel_path in ".cargo" "Cargo.toml" "Cargo.lock" "app_gen" "observability"; do
    copy_if_exists "$source" "$target" "$rel_path"
  done

  for rel_path in \
    "api_tests" \
    "appfw_provider_ai_search" \
    "appfw_cli" \
    "appfw_provider_anaplan" \
    "appfw_provider_icims" \
    "appfw_provider_oracle_financials" \
    "appfw_provider_servicenow" \
    "appfw_saas_core" \
    "appfw_saas_testkit" \
    "appfw_provider_mongo" \
    "appfw_provider_mssql" \
    "appfw_mssql_auth" \
    "appfw_provider_neo4j" \
    "appfw_provider_postgres" \
    "appfw_provider_salesforce" \
    "appfw_provider_snowflake" \
    "appfw_provider_workday" \
    "appfw_runtime" \
    "appfw_test" \
    "database" \
    "rego_test"; do
    copy_cargo_member_stub "$source" "$target" "$rel_path"
  done
}

rebase_runtime_path_dependency() {
  local manifest="$1"
  local runtime_root="$2"
  if [[ ! -f "$manifest" ]]; then
    return 0
  fi

  python3 - "$manifest" "$runtime_root" <<'PY'
import os
import re
import sys
from pathlib import Path

manifest = Path(sys.argv[1])
runtime_root = Path(sys.argv[2])
relative_runtime = os.path.relpath(runtime_root, manifest.parent)
contents = manifest.read_text()
updated = []
in_dependencies = False
rewritten = 0
for line in contents.splitlines(keepends=True):
    section = re.match(r"^\s*\[([^]]+)]\s*$", line.rstrip("\r\n"))
    if section:
        in_dependencies = section.group(1).strip() == "dependencies"
    if in_dependencies and "path" in line and "=" in line:
        key, declaration = line.split("=", 1)
        key = key.strip().strip('"').strip("'")
        is_runtime = key in {"appfw_runtime", "appfw-runtime"} or bool(
            re.search(r'\bpackage\s*=\s*["\']appfw-runtime["\']', declaration)
        )
        if is_runtime:
            line, count = re.subn(
                r'(\bpath\s*=\s*["\'])[^"\']*(["\'])',
                lambda match: f"{match.group(1)}{relative_runtime}{match.group(2)}",
                line,
                count=1,
            )
            if count != 1:
                raise SystemExit(
                    f"could not rebase appfw-runtime path dependency in {manifest}"
                )
            rewritten += 1
    updated.append(line)

if rewritten > 1:
    raise SystemExit(f"multiple appfw-runtime path dependencies found in {manifest}")
if rewritten == 1:
    manifest.write_text("".join(updated))
PY
}

protected_manifest() {
  local root="$1"
  if [[ ! -d "$root/backend/src/handlers" ]]; then
    return
  fi
  local list_file="$tmp_root/protected-manifest-files.txt"
  (
    cd "$root"
    find "backend/src/handlers" -type f -name '*.rs' ! -name 'mod.rs' ! -name 'generated.rs' -print | sort >"$list_file"
    if [[ -s "$list_file" ]]; then
      xargs shasum -a 256 <"$list_file"
    fi
  )
}

framework_manifest() {
  local root="$1"
  local list_file="$tmp_root/framework-manifest-files.txt"
  : >"$list_file"
  (
    cd "$root"
    for rel_path in \
      "backend/src" \
      "database/_pkg" \
      "database/src/data_source.rs" \
      "api_tests/src/schemas" \
      "podman-compose.yml" \
      ".appfw/model"; do
      if [[ -d "$rel_path" ]]; then
        find "$rel_path" -type f -print
      elif [[ -f "$rel_path" ]]; then
        printf '%s\n' "$rel_path"
      fi
    done | sort >"$list_file"
    if [[ -s "$list_file" ]]; then
      xargs shasum -a 256 <"$list_file"
    fi
  )
}

compare_generated_path() {
  local rel_path="$1"

  echo "Comparing $rel_path..."
  if [[ ! -e "$app_root/$rel_path" && ! -e "$work_app/$rel_path" ]]; then
    echo "Skipping $rel_path; it is not part of this product surface."
    return 0
  fi
  if ! diff -ru "$app_root/$rel_path" "$work_app/$rel_path"; then
    echo "Generated output differs for $rel_path." >&2
    return 1
  fi
}

normalize_compose_framework_bindings() {
  local file="$1"
  sed -E \
    -e 's#source: .*appfw_runtime$#source: __APPFW_RUNTIME__#' \
    -e '/source: __APPFW_RUNTIME__/{n;s#target: .*$#target: __APPFW_RUNTIME_TARGET__#;}' \
    -e 's#source: .*observability/#source: __APPFW_OBSERVABILITY__/#' \
    "$file"
}

compare_compose_path() {
  local rel_path="$1"
  local expected="$tmp_root/expected-compose.yml"
  local actual="$tmp_root/actual-compose.yml"

  echo "Comparing $rel_path..."
  normalize_compose_framework_bindings "$app_root/$rel_path" > "$expected"
  normalize_compose_framework_bindings "$work_app/$rel_path" > "$actual"
  if ! diff -u "$expected" "$actual"; then
    echo "Generated output differs for $rel_path." >&2
    return 1
  fi
}

compare_phase_name() {
  local rel_path="$1"
  local name="${rel_path//\//-}"
  name="${name//_/-}"
  printf 'compare-%s\n' "$name"
}

run_generated_compare_phase() {
  local rel_path="$1"
  local phase_name
  phase_name="$(compare_phase_name "$rel_path")"
  local start_ms status elapsed_ms
  start_ms="$(ms_now)"
  set +e
  compare_generated_path "$rel_path"
  status=$?
  set -e
  elapsed_ms="$(( $(ms_now) - start_ms ))"
  if [[ $status -eq 0 ]]; then
    record_phase "$phase_name" "$elapsed_ms" true "compare generated path $rel_path"
  else
    record_phase "$phase_name" "$elapsed_ms" false "compare generated path $rel_path"
  fi
  return "$status"
}

run_compose_compare_phase() {
  local rel_path="$1"
  local phase_name
  phase_name="$(compare_phase_name "$rel_path")"
  local start_ms status elapsed_ms
  start_ms="$(ms_now)"
  set +e
  compare_compose_path "$rel_path"
  status=$?
  set -e
  elapsed_ms="$(( $(ms_now) - start_ms ))"
  if [[ $status -eq 0 ]]; then
    record_phase "$phase_name" "$elapsed_ms" true "compare generated path $rel_path"
  else
    record_phase "$phase_name" "$elapsed_ms" false "compare generated path $rel_path"
  fi
  return "$status"
}

echo "Copying product and framework roots to temp directories..."
copy_start_ms="$(ms_now)"
copy_product_equivalence_root "$app_root" "$work_app"
copy_framework_equivalence_root "$framework_root" "$work_framework"
rebase_runtime_path_dependency \
  "$work_app/backend/Cargo.toml" \
  "$work_framework/appfw_runtime"
record_phase "copy-roots" "$(( $(ms_now) - copy_start_ms ))" true "copy generator-relevant product and framework roots into an isolated equivalence workspace"

# Prove generation is using framework roots for generator source/templates, not
# product-local framework internals that only exist in the copied layout.
prepare_start_ms="$(ms_now)"
rm -rf "$work_app/app_gen/_templates" "$work_app/app_gen/src"

work_app_gen_dir="$(rebase_path "$app_gen_dir" "$framework_root" "$work_framework")"
work_config_root="$(rebase_path "$config_root" "$app_root" "$work_app")"
work_templates_root="$(rebase_path "$templates_root" "$framework_root" "$work_framework")"
work_report_root="$(rebase_path "$report_root" "$app_root" "$work_app")"

protected_manifest "$work_app" > "$tmp_root/protected.before"
framework_manifest "$work_framework" > "$tmp_root/framework.before"
record_phase "prepare-workspace" "$(( $(ms_now) - prepare_start_ms ))" true "remove product-local generator internals and snapshot protected/framework manifests"

echo "Running app_gen with split product/framework roots..."
mkdir -p "$equivalence_target_dir" "$equivalence_log_dir"
: >"$retained_log_file"
echo "Generator log: $retained_log_file"
generate_start_ms="$(ms_now)"
if [[ -n "${APPFW_APP_GEN_BIN:-}" && -x "${APPFW_APP_GEN_BIN}" ]]; then
  if ! (
    cd "$work_app"
    "${APPFW_APP_GEN_BIN}" \
      --app-root "$work_app" \
      --framework-root "$work_framework" \
      --generator-root "$work_app_gen_dir" \
      --config-root "$work_config_root" \
      --templates-root "$work_templates_root" \
      --report-root "$work_report_root" \
      > "$retained_log_file" 2>&1
  ); then
    record_phase "run-app-gen" "$(( $(ms_now) - generate_start_ms ))" false "run app_gen with split product/framework roots"
    cat "$retained_log_file"
    echo "app_gen failed. Temp directory: $tmp_root" >&2
    exit 1
  fi
elif ! (
  cd "$work_app_gen_dir"
  CARGO_TARGET_DIR="$equivalence_target_dir" cargo run --locked -j "$equivalence_jobs" --bin app_gen -- \
  --app-root "$work_app" \
  --framework-root "$work_framework" \
  --generator-root "$work_app_gen_dir" \
  --config-root "$work_config_root" \
  --templates-root "$work_templates_root" \
  --report-root "$work_report_root" \
  > "$retained_log_file" 2>&1
); then
  record_phase "run-app-gen" "$(( $(ms_now) - generate_start_ms ))" false "run app_gen with split product/framework roots"
  cat "$retained_log_file"
  echo "app_gen failed. Temp directory: $tmp_root" >&2
  exit 1
fi
record_phase "run-app-gen" "$(( $(ms_now) - generate_start_ms ))" true "run app_gen with split product/framework roots"
cp "$retained_log_file" "$log_file"
for retained_report in performance_recommendations.json performance_recommendations.md; do
  if [[ -f "$work_report_root/$retained_report" ]]; then
    cp "$work_report_root/$retained_report" "$equivalence_log_dir/$retained_report"
  fi
done

protected_manifest "$work_app" > "$tmp_root/protected.after"
framework_manifest "$work_framework" > "$tmp_root/framework.after"

echo "Checking protected handler impl files..."
protected_start_ms="$(ms_now)"
set +e
diff -u "$tmp_root/protected.before" "$tmp_root/protected.after"
protected_status=$?
set -e
if [[ $protected_status -eq 0 ]]; then
  record_phase "check-protected-handlers" "$(( $(ms_now) - protected_start_ms ))" true "verify create-once handler impl files were preserved"
else
  record_phase "check-protected-handlers" "$(( $(ms_now) - protected_start_ms ))" false "verify create-once handler impl files were preserved"
  echo "app_gen changed existing handler impl files. These files are intended to be create-once." >&2
  exit 1
fi

echo "Checking framework copy was not mutated..."
framework_start_ms="$(ms_now)"
set +e
diff -u "$tmp_root/framework.before" "$tmp_root/framework.after"
framework_status=$?
set -e
if [[ $framework_status -eq 0 ]]; then
  record_phase "check-framework-copy" "$(( $(ms_now) - framework_start_ms ))" true "verify generator did not write into the framework root"
else
  record_phase "check-framework-copy" "$(( $(ms_now) - framework_start_ms ))" false "verify generator did not write into the framework root"
  echo "app_gen wrote generated output into the framework root. It must write to the product root." >&2
  exit 1
fi

if ! run_generated_compare_phase "backend/config/generated"; then
  echo "Inspect generator log at: $log_file" >&2
  exit 1
fi

if ! run_generated_compare_phase "backend/src"; then
  echo "Inspect generator log at: $log_file" >&2
  exit 1
fi

if ! run_generated_compare_phase "database/_pkg"; then
  echo "Inspect generator log at: $log_file" >&2
  exit 1
fi

if ! run_generated_compare_phase "database/src/data_source.rs"; then
  echo "Inspect generator log at: $log_file" >&2
  exit 1
fi

if ! run_generated_compare_phase "api_tests/src/schemas"; then
  echo "Inspect generator log at: $log_file" >&2
  exit 1
fi

if ! run_compose_compare_phase "podman-compose.yml"; then
  echo "Inspect generator log at: $log_file" >&2
  exit 1
fi

echo "app_gen split-root equivalence check passed."
