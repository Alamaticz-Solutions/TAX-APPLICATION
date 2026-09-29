# Authentication Notes

The backend supports local development with auth bypass for generated API tests
and Okta/JWT validation for normal authenticated requests.

Do not commit real JWTs, signing secrets, tenant identifiers, or user tokens to
this repository. Keep local tokens in ignored files such as
`api_tests/test_tokens/*.txt` when token-file testing is required.

## Local API Tests

Generated API tests default to bypass mode:

```text
API_TEST_AUTH_MODE=bypass
```

For token-file mode, see `api_tests/README.md`.

## Local Admin UI Policy Bypass

When `ENV_NAME=local`, the backend creates a synthetic `local-dev` user with
the `admin` role for requests without an `Authorization` header. Access policies
still evaluate normally unless an explicit local-only override is enabled.

For local UI exploration against seeded data, you can bypass Rego policy
evaluation and tenant filters:

```bash
ENV_NAME=local APP_BYPASS_POLICIES_IN_LOCAL=true API_PORT=8080 scripts/appfw serve
```

This setting only works with `ENV_NAME=local`. Do not use it in shared,
staging, or production environments.

## Okta Configuration

Runtime Okta settings are loaded from environment variables. Use placeholder
values in documentation and keep real values in local environment management or
your deployment secret store.

```text
OKTA_AUDIENCE=api://default
OKTA_ISSUER=https://example.okta.com/oauth2/default
OKTA_CLIENT_ID=example-client-id
```

## GraphiQL Headers

When manually testing authenticated requests in GraphiQL, add a locally issued
or provider-issued bearer token:

```json
{
  "Authorization": "Bearer <local-or-provider-token>"
}
```
