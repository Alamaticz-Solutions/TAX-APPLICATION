#!/usr/bin/env bash
#
# local-pre-push-gates.sh — substantive local subset of the Bitbucket PR lane.
#
# Runs the locally reproducible gate bodies from pull-requests "**" in order:
#   release-lite-guard
#   pr-preflight                  (resolved local base; no gate routing)
#   local workstation prep (does NOT change Bitbucket CI)
#   pr-fast-framework-check   (shared with CI Fast framework check)
#   supply-chain-gate
#   secret-scan + release-evidence-check --local-fixture
#   release-lite-guard (again)
#
# This is the agent/developer pre-commit/pre-push entrypoint. Do not maintain a
# second copy of these commands in AGENTS.md or docs/release/README.md — link
# here. Do not put local-only prep into pr-fast-framework-check.sh — that script
# is the Bitbucket Fast framework SSOT and must stay identical to CI.
# Destination freshness, Bitbucket source/tested/destination checkout identity,
# artifact upload behavior, and producer overlap remain remote-only proof. Main
# focused release-gate / v* strict authority remain separate
# (local-live-preflight / managed CI). This script proves local branch
# confidence; it does not independently establish PR merge readiness.
#
# Usage:
#   bash scripts/ci/local-pre-push-gates.sh
#   bash scripts/ci/local-pre-push-gates.sh --base origin/main
#   bash scripts/ci/local-pre-push-gates.sh --skip-release-lite
#
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
cd "$repo_root"

skip_release_lite=false
preflight_base="${APPFW_PR_PREFLIGHT_BASE:-origin/main}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-release-lite) skip_release_lite=true ;;
    --base)
      [[ $# -ge 2 && -n "$2" ]] || {
        echo "local-pre-push-gates: --base requires a ref or SHA" >&2
        exit 2
      }
      preflight_base="$2"
      shift
      ;;
    --base=*)
      preflight_base="${1#--base=}"
      [[ -n "$preflight_base" ]] || {
        echo "local-pre-push-gates: --base requires a ref or SHA" >&2
        exit 2
      }
      ;;
    -h|--help)
      sed -n '2,28p' "$0"
      exit 0
      ;;
    *)
      echo "local-pre-push-gates: unknown argument: $1" >&2
      exit 2
      ;;
  esac
  shift
done

log() {
  printf '\n[%s] %s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" "$*"
}

# Local-only: Bitbucket clones start without CRM product_dist, so wave3 always
# runs pack → lock-integrity sync → npm ci → SPA build. A dirty workstation can
# keep examples/products/crm/backend/product_dist and skip that path
# (already-built), which is a false green vs CI. Clear it here only.
clear_local_crm_product_dist() {
  local product_dist="$repo_root/examples/products/crm/backend/product_dist"
  if [[ -e "$product_dist" ]]; then
    log "local prep: removing $product_dist so wave3 SPA install/build matches CI"
    rm -rf "$product_dist"
  else
    log "local prep: CRM product_dist already absent (CI-like SPA path)"
  fi
}

run_release_lite() {
  local label="$1"
  if [[ "$skip_release_lite" == "true" ]]; then
    log "Skipping release-lite-guard ($label)"
    return 0
  fi
  log "release-lite-guard ($label)"
  bash "$script_dir/release-lite-guard.sh"
}

run_release_lite "before PR preflight"

log "pr-preflight (report-only change classification; base=$preflight_base)"
python3 "$script_dir/pr-preflight.py" --base "$preflight_base" --deadline-seconds 240

clear_local_crm_product_dist

log "pr-fast-framework-check (CI Fast framework check parity)"
bash "$script_dir/pr-fast-framework-check.sh"

log "supply-chain-gate"
bash "$script_dir/supply-chain-gate.sh"

log "secret-scan"
bash "$script_dir/secret-scan.sh"

log "release-evidence-check --local-fixture"
APPFW_RELEASE_ARTIFACT_DIR=target/appfw/local-fixture \
  bash "$script_dir/release-evidence-check.sh" --local-fixture

run_release_lite "after PR evidence"

log "local-pre-push-gates: OK — substantive local PR-gate subset complete"
log "Covered locally: release-lite, pr-preflight, pr-fast-framework-check, supply-chain,"
log "  secret-scan, release-evidence-check --local-fixture."
log "Remote-only proof still required: Bitbucket checkout identity, destination freshness,"
log "  artifact upload/capture timing, fail-fast cancellation, and producer overlap."
log "Intentionally NOT covered here (do not add as silent PR checks):"
log "  main focused release-gate (bitbucket-release-gate focused / local-live-preflight),"
log "  v* strict release-gate, publish-proget, custom mssql-odbc-ntlm-e2e."
log "Next: handoff + review-brief + PR review before push;"
log "  main focused: scripts/appfw framework local-live-preflight --json;"
log "  v*/ProGet: managed strict tag lane only."
