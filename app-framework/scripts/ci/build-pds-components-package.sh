#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
component_root="$repo_root/appfw_ui/pds_health/components"
package_dir="$repo_root/target/appfw/packages"
npm_cache_dir="$repo_root/target/appfw/npm-cache"
package_version="$(node -p "require('$component_root/package.json').version")"
package_name="appfw-pds-health-components-$package_version.tgz"

mkdir -p "$package_dir" "$npm_cache_dir"
export npm_config_cache="$npm_cache_dir"
if [[ ! -d "$component_root/node_modules" ]]; then
  npm ci --prefix "$component_root" --no-audit --prefer-offline
fi
# Isolated npm ci does not install optional peers. The packed manifest must keep
# @appfw/pds-ix-presentation-contract as an optional peer (no file: dependency).
# Link the in-repo workspace package so tsc can resolve it.
contract_root="$repo_root/appfw_ui/pds_health/ix-presentation-contract"
if [[ ! -f "$contract_root/package.json" ]]; then
  echo "missing workspace package: $contract_root" >&2
  exit 2
fi
mkdir -p "$component_root/node_modules/@appfw"
ln -sfn "$contract_root" "$component_root/node_modules/@appfw/pds-ix-presentation-contract"
npm --prefix "$component_root" run build
npm --prefix "$component_root" run check:package
rm -f "$package_dir/$package_name"
npm pack "$component_root" --pack-destination "$package_dir" --cache "$npm_cache_dir" --json

test -f "$package_dir/$package_name"
node "$component_root/scripts/check-package.mjs" --archive "$package_dir/$package_name"
shasum -a 256 "$package_dir/$package_name"
