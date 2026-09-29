#!/usr/bin/env bash
#
# pr-fast-framework-check.sh — single source of truth for the Bitbucket
# "Fast framework check" gate sequence.
#
# Bitbucket Pipelines and local pre-push both run this script so the command
# list cannot drift. Do not duplicate these steps in docs or AGENTS.md; point
# here instead.
#
# Usage:
#   bash scripts/ci/pr-fast-framework-check.sh
#   bash scripts/ci/pr-fast-framework-check.sh --ci-bootstrap
#
# --ci-bootstrap installs rustfmt/apt packages/Node as the pipeline image step
# does. Local runs skip that and only ensure Node via install-node.sh when needed.
#
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
cd "$repo_root"

ci_bootstrap=false
for arg in "$@"; do
  case "$arg" in
    --ci-bootstrap) ci_bootstrap=true ;;
    -h|--help)
      sed -n '2,20p' "$0"
      exit 0
      ;;
    *)
      echo "pr-fast-framework-check: unknown argument: $arg" >&2
      exit 2
      ;;
  esac
done

log() {
  printf '\n[%s] %s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" "$*"
}

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target/cargo}"
export APPFW_RELEASE_TOOL_DIR="${APPFW_RELEASE_TOOL_DIR:-target/appfw-tools}"
export APPFW_DOCS_CHECK_FAST_BUDGET_MS="${APPFW_DOCS_CHECK_FAST_BUDGET_MS:-20000}"
export APPFW_DOCS_CHECK_FULL_BUDGET_MS="${APPFW_DOCS_CHECK_FULL_BUDGET_MS:-480000}"

# Match bitbucket-pipelines.yml: only golden-downstream overrides APPFW_APP_ROOT
# to the framework checkout (second-consumer profile). Do not export it for the
# whole step — framework validate/cli-test/wave3 default to examples/products/crm.
golden_app_root="${BITBUCKET_CLONE_DIR:-$repo_root}"

finalize_pr_fast_evidence() {
  local gate_status=$?
  local package_status=0
  trap - EXIT
  set +e
  python3 "$script_dir/package-pr-fast-evidence.py" \
    --repo-root "$repo_root" \
    --gate-exit-status "$gate_status"
  package_status=$?
  set -e

  # A packaging failure must fail an otherwise-green Fast gate. When an
  # assurance gate already failed, retain that original status so evidence
  # packaging never masks the smallest real failure.
  if [[ "$gate_status" -ne 0 ]]; then
    exit "$gate_status"
  fi
  exit "$package_status"
}

# Always retain bounded diagnostics, including when one of the six existing
# gates fails. The finalizer validates the complete green contract only when
# every gate and the focused artifact contract test returned zero.
trap finalize_pr_fast_evidence EXIT

if [[ "$ci_bootstrap" == "true" ]]; then
  log "CI bootstrap: evidence root, rustfmt, apt packages, Node"
  bash "$script_dir/prepare-appfw-evidence-root.sh"
  if command -v rustup >/dev/null 2>&1; then
    rustup component add rustfmt
  fi
  if [[ "$(id -u)" == "0" ]] && command -v apt-get >/dev/null 2>&1; then
    apt-get update
    apt-get install -y --no-install-recommends \
      git rsync diffutils perl ca-certificates curl xz-utils
    rm -rf /var/lib/apt/lists/*
  fi
  bash "$script_dir/install-node.sh"
else
  log "Local run: ensuring evidence root and Node (no apt bootstrap)"
  bash "$script_dir/prepare-appfw-evidence-root.sh"
  # Prefer a repo-local Node prefix when not root so install-node.sh matches CI
  # without needing /usr/local write access.
  if [[ "$(id -u)" != "0" ]]; then
    export APPFW_NODE_INSTALL_DIR="${APPFW_NODE_INSTALL_DIR:-$repo_root/target/appfw-tools/node}"
    mkdir -p "$APPFW_NODE_INSTALL_DIR"
    export PATH="$APPFW_NODE_INSTALL_DIR/bin:$PATH"
  fi
  bash "$script_dir/install-node.sh"
  export PATH="${APPFW_NODE_INSTALL_DIR:-/usr/local}/bin:$PATH"
fi

log "Node $(node --version) / npm $(npm --version)"

log "1/6 framework validate"
scripts/appfw framework validate --json

log "2/6 framework cli-test"
scripts/appfw framework cli-test --json

log "3/6 golden-downstream second-consumer --execute"
APPFW_APP_ROOT="$golden_app_root" \
  scripts/appfw framework golden-downstream --profile second-consumer --execute --json

log "4/6 docs-check --changed-only --enforce-budget"
scripts/appfw docs-check --changed-only --json --progress --enforce-budget

log "5/6 wave3-pr-gates --reuse-docs-check"
bash "$script_dir/wave3-pr-gates.sh" --reuse-docs-check

log "6/6 framework test --fast"
scripts/appfw test --fast --json

log "PR Fast evidence artifact contract"
node --test scripts/ci/pr-fast-evidence-contract.test.mjs

log "pr-fast-framework-check: OK"
