# Auth Handler Notes

Authentication behavior should be documented as a runtime contract, not as a
copy of an identity-provider console walkthrough. Provider UI steps change
often; keep this file focused on what the backend needs.

## Runtime Contract

The backend expects JWT validation settings from environment variables:

```text
OKTA_AUDIENCE=api://default
OKTA_ISSUER=https://example.okta.com/oauth2/default
OKTA_CLIENT_ID=example-client-id
```

Use deployment secret management or local ignored environment files for real
values. Do not commit tenant-specific issuer URLs, client IDs, tokens, signing
keys, or user data.

## Local Development

Generated API tests can use auth bypass mode:

```text
API_TEST_AUTH_MODE=bypass
```

Token-file testing is documented in `api_tests/README.md`.

## Related Docs

```text
backend/src/auth.md
backend/README.md
api_tests/README.md
```
