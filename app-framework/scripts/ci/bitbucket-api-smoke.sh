#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/ci/bitbucket-api-smoke.sh [--repo] [--pipelines] [--user] [--dry-run]

Verifies the repo's standard Bitbucket REST API auth path without printing
secrets. This helper intentionally treats BITBUCKET_API_TOKEN as an Atlassian
user API token, which must use Basic auth with the Atlassian account email as
the username.

Environment:
  BITBUCKET_API_TOKEN      Required Atlassian user API token.
  BITBUCKET_API_EMAIL      Required Atlassian account email for the API token.
  BITBUCKET_WORKSPACE      Defaults to pacificdental.
  BITBUCKET_REPO_SLUG      Defaults to app-framework.

Notes:
  - Do not use BITBUCKET_API_TOKEN with a Bearer header.
  - Do not use a Bitbucket username or token label as the Basic username.
  - Do not fall back to BITBUCKET_REST_API_TOKEN unless a human confirms its
    token type and auth scheme.
EOF
}

check_repo=0
check_pipelines=0
check_user=0
dry_run=0

if [[ "$#" -eq 0 ]]; then
  check_repo=1
fi

while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --repo)
      check_repo=1
      ;;
    --pipelines)
      check_pipelines=1
      ;;
    --user)
      check_user=1
      ;;
    --dry-run)
      dry_run=1
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

if [[ "$check_repo" == "0" && "$check_pipelines" == "0" && "$check_user" == "0" ]]; then
  check_repo=1
fi

workspace="${BITBUCKET_WORKSPACE:-pacificdental}"
repo_slug="${BITBUCKET_REPO_SLUG:-app-framework}"
email="${BITBUCKET_API_EMAIL:-}"
token="${BITBUCKET_API_TOKEN:-}"

if [[ -z "$email" ]]; then
  echo "BITBUCKET_API_EMAIL is not set." >&2
  echo "Set BITBUCKET_API_EMAIL to the Atlassian account email for the API token." >&2
  echo "Do not rely on git user.email; Git author email may differ from the Atlassian account email." >&2
  exit 2
fi

if [[ -z "$token" ]]; then
  echo "BITBUCKET_API_TOKEN is not set." >&2
  if [[ -n "${BITBUCKET_REST_API_TOKEN:-}" ]]; then
    echo "BITBUCKET_REST_API_TOKEN is present, but this helper does not assume its token type." >&2
    echo "Use BITBUCKET_API_TOKEN for Atlassian user API tokens with Basic auth." >&2
  fi
  exit 2
fi

base_url="https://api.bitbucket.org/2.0"

run_check() {
  local name="$1"
  local url="$2"
  local body
  local http_code
  local curl_status

  echo "Checking ${name}: ${url}"
  if [[ "$dry_run" == "1" ]]; then
    echo "  dry-run: would use Basic auth username ${email} with BITBUCKET_API_TOKEN"
    return 0
  fi

  body="$(mktemp "${TMPDIR:-/tmp}/bitbucket-api-smoke.XXXXXX")"
  curl_status=0
  http_code="$(curl -sS -o "$body" -w "%{http_code}" -u "${email}:${token}" "$url")" || curl_status=$?

  if [[ "$curl_status" -ne 0 ]]; then
    rm -f "$body"
    echo "  failed: curl exited with status ${curl_status}" >&2
    return "$curl_status"
  fi

  case "$http_code" in
    200)
      echo "  ok: HTTP 200"
      rm -f "$body"
      ;;
    401)
      rm -f "$body"
      echo "  failed: HTTP 401 unauthorized" >&2
      echo "  likely causes:" >&2
      echo "  - Basic username is not the Atlassian account email for this token." >&2
      echo "  - BITBUCKET_API_TOKEN is expired, revoked, copied incorrectly, or not a user API token." >&2
      echo "  - The request was previously attempted with Bearer auth or a Bitbucket username/token label." >&2
      return 1
      ;;
    403)
      rm -f "$body"
      echo "  failed: HTTP 403 forbidden" >&2
      echo "  likely cause: token authenticated but lacks the required Bitbucket scope or workspace/repo access." >&2
      return 1
      ;;
    *)
      echo "  failed: HTTP ${http_code}" >&2
      echo "  response body retained at ${body}" >&2
      return 1
      ;;
  esac
}

if [[ "$check_repo" == "1" ]]; then
  run_check "repository read" "${base_url}/repositories/${workspace}/${repo_slug}"
fi

if [[ "$check_pipelines" == "1" ]]; then
  run_check "pipeline read" "${base_url}/repositories/${workspace}/${repo_slug}/pipelines/?pagelen=1"
fi

if [[ "$check_user" == "1" ]]; then
  run_check "current user" "${base_url}/user"
fi
