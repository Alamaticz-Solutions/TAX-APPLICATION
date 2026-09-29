#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
cd "$repo_root"

chmod +x scripts/git-hooks/pre-push scripts/ci/pre-push-review-guard.sh
previous_hooks_path="$(git config --get core.hooksPath || true)"
if [[ -n "$previous_hooks_path" && "$previous_hooks_path" != "scripts/git-hooks" ]]; then
  echo "Replacing existing core.hooksPath=$previous_hooks_path"
fi
git config core.hooksPath scripts/git-hooks

echo "Installed repo-owned Git hooks via core.hooksPath=scripts/git-hooks"
echo "The Framework PR Review Agent guard now runs only at git pre-push for branch updates."
