#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
Usage:
  scripts/ci/bitbucket-git-auth.sh fetch origin
  scripts/ci/bitbucket-git-auth.sh push origin <branch>

This script also acts as a Git credential helper when Git invokes it with
`get`, `store`, or `erase`. It reads BITBUCKET_API_TOKEN from the environment
or from the ignored repo-local .env file. It never prints the token except to
Git's credential-helper stdout protocol.
USAGE
}

repo_root() {
  git rev-parse --show-toplevel 2>/dev/null || pwd
}

load_env() {
  if [[ -n "${BITBUCKET_API_TOKEN:-}" ]]; then
    return 0
  fi

  local env_file="${APPFW_BITBUCKET_ENV_FILE:-}"
  if [[ -z "$env_file" ]]; then
    env_file="$(repo_root)/.env"
  fi

  if [[ -f "$env_file" ]]; then
    set -a
    # shellcheck disable=SC1090
    source "$env_file"
    set +a
  fi

  [[ -n "${BITBUCKET_API_TOKEN:-}" ]]
}

credential_helper() {
  local op="${1:-}"
  if [[ "$op" != "get" ]]; then
    exit 0
  fi

  local protocol=""
  local host=""
  local username=""

  local line
  while IFS= read -r line; do
    [[ -z "$line" ]] && break
    case "$line" in
      protocol=*) protocol="${line#protocol=}" ;;
      host=*) host="${line#host=}" ;;
      username=*) username="${line#username=}" ;;
    esac
  done

  [[ "$protocol" == "https" ]] || exit 0
  [[ "$host" == "bitbucket.org" ]] || exit 0
  load_env || exit 0

  if [[ -z "$username" ]]; then
    printf 'username=%s\n' "${BITBUCKET_GIT_USERNAME:-x-bitbucket-api-token-auth}"
  fi
  printf 'password=%s\n' "$BITBUCKET_API_TOKEN"
}

run_git() {
  if ! load_env; then
    echo "BITBUCKET_API_TOKEN is required in the environment or ignored .env file." >&2
    exit 2
  fi

  local script_path
  script_path="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$(basename "${BASH_SOURCE[0]}")"
  export APPFW_BITBUCKET_ENV_FILE="${APPFW_BITBUCKET_ENV_FILE:-$(repo_root)/.env}"

  git -c credential.helper= -c "credential.helper=$script_path" "$@"
}

case "${1:-}" in
  get | store | erase)
    credential_helper "$@"
    ;;
  fetch | push | ls-remote)
    run_git "$@"
    ;;
  -h | --help | "")
    usage
    ;;
  *)
    usage
    exit 2
    ;;
esac
