# Bitbucket REST Auth Runbook

Use this runbook before an agent or human spends time debugging Bitbucket REST
API `401 Unauthorized` responses. The goal is to prove the authentication shape
once, then stop retrying with mismatched token/header combinations.

## Standard App Framework Path

For this repository, use `BITBUCKET_API_TOKEN` as an Atlassian user API token:

```bash
export BITBUCKET_API_EMAIL="first.last@company.com"
scripts/ci/bitbucket-api-smoke.sh --repo --pipelines
```

For local operator convenience, the ignored `.env` file may include:

```bash
BITBUCKET_API_EMAIL=first.last@company.com
```

Keep `.env` local; do not commit tokens, account-specific secrets, or local
operator configuration.

The helper uses:

```text
Authorization: Basic base64(Atlassian account email:BITBUCKET_API_TOKEN)
```

Do **not** use:

- `Authorization: Bearer $BITBUCKET_API_TOKEN`
- a Bitbucket username as the Basic username
- a token label as the Basic username
- `BITBUCKET_REST_API_TOKEN` as an automatic fallback
- legacy app passwords for new automation

`BITBUCKET_API_EMAIL` is required. Do not rely on `git config user.email`; Git
author email can differ from the Atlassian account email that owns the API
token.

## Why This Matters

Atlassian's Bitbucket Cloud REST documentation distinguishes the schemes:

- API tokens are the long-term replacement for app passwords and are used with
  Basic HTTP authentication. The username is the Atlassian email and the
  password is the API token.
- OAuth access tokens are sent as `Authorization: Bearer ...`.
- App passwords are deprecated and in controlled brownout/removal.

Those token types are easy to confuse because they all look like opaque secret
strings. The failure mode is noisy: a valid token sent with the wrong username
or wrong header scheme still returns `401`.

## REST Is Not Git

Bitbucket uses a different username convention for Git commands than for REST
API calls:

| Use case | Username | Password/token |
| --- | --- | --- |
| REST API | Atlassian account email | `BITBUCKET_API_TOKEN` |
| Git clone/fetch/push | Bitbucket username, or `x-bitbucket-api-token-auth` | `BITBUCKET_API_TOKEN` |

For App Framework Git operations, prefer the repo wrapper so the token stays out
of the remote URL, shell history, process arguments, and logs:

```bash
scripts/ci/bitbucket-git-auth.sh fetch origin
scripts/ci/bitbucket-git-auth.sh push origin docs/harness-throughput-quality
```

The wrapper reads `BITBUCKET_API_TOKEN` from the environment or the ignored
repo-local `.env` file and supplies it to Git through the credential-helper
protocol. It also works as a credential helper for `fetch`, `push`, and
`ls-remote`; it does not persist credentials in `.git/config`.

For a lower-level promptless Git push, use the branch name as the source ref so
the local pre-push review guard can validate the checked-out branch:

```bash
GIT_ASKPASS=/path/to/askpass \
GIT_TERMINAL_PROMPT=0 \
git push https://x-bitbucket-api-token-auth@bitbucket.org/pacificdental/app-framework.git \
  fix/fast-framework-check-npm:fix/fast-framework-check-npm
```

The askpass helper must return `BITBUCKET_API_TOKEN` for the password prompt.
Do not put the token directly in the remote URL or persist it in `.git/config`.

## 401 Decision Tree

When a Bitbucket REST call returns `401`, do this in order:

1. Run the smoke check:

   ```bash
   scripts/ci/bitbucket-api-smoke.sh --repo --pipelines
   ```

2. If the smoke check fails before the request, set the missing local
   environment:

   ```bash
   export BITBUCKET_API_EMAIL="first.last@company.com"
   export BITBUCKET_API_TOKEN="<token from Atlassian API token page>"
   ```

3. If the smoke check returns `401`, regenerate or recopy the
   `BITBUCKET_API_TOKEN` and confirm the Basic username is the Atlassian email
   for the account that owns the token.
4. If the smoke check returns `403`, the token authenticated but lacks the
   required scope or repository/workspace access.
5. If repository read succeeds but pipeline read fails, add the pipeline read
   permission and rerun the smoke check.
6. Do not try a different header scheme unless the token type has been
   explicitly reclassified as OAuth/access-token based.

## Common Scope Needs

Use the smallest scope set that matches the operation:

| Operation | Minimum useful permissions |
| --- | --- |
| Read repository metadata, branches, commits, or files | repository read |
| Read pull requests | repository read and pull-request read |
| Create or update pull requests through REST | repository write and pull-request write |
| Read pipeline status, steps, logs, or artifacts | repository read and pipeline read |
| Trigger or manage pipelines through REST | repository write and pipeline write |

The Bitbucket UI may display modern names such as
`read:repository:bitbucket`, `read:pipeline:bitbucket`, or
`write:repository:bitbucket`. Treat those as the UI labels for the same
least-privilege intent.

## Agent Operating Rule

Before using Bitbucket REST in a fresh thread or on a fresh machine, run:

```bash
scripts/ci/bitbucket-api-smoke.sh --repo --pipelines
```

If it passes, reuse that exact auth pattern for the rest of the thread. If it
fails, stop and report the specific failure. Do not churn through
`BITBUCKET_REST_API_TOKEN`, Bearer headers, Bitbucket usernames, token labels,
or app-password assumptions without a human explicitly changing the token type.

## References

- [Atlassian Bitbucket Cloud API tokens](https://support.atlassian.com/bitbucket-cloud/docs/api-tokens/)
- [Atlassian Bitbucket Cloud REST API authentication](https://developer.atlassian.com/cloud/bitbucket/rest/intro/#authentication)
- [Atlassian Bitbucket Cloud app passwords](https://support.atlassian.com/bitbucket-cloud/docs/using-app-passwords/)
