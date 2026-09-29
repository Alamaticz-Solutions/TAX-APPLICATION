# Okta / OIDC authentication — what the framework already supports

Research notes on integrating Okta for frontend login in `tax-doc-routing`. This app currently runs with `APP_ENABLE_LOCAL_TEST_AUTH=true` (local dev only, no real token verification, no login UI). This document captures what the vendored App Framework (`app-framework/`) already provides toward a real Okta-backed login, and what's still missing to turn it on here.

## Current state in this app

- `.env` sets `APP_ENABLE_LOCAL_TEST_AUTH=true`. With no `Authorization` header, every request is treated as `admin`; a request can also send `Authorization: Bearer appfw-local:user=<name>;tenant=<id>;roles=<r1,r2>` to impersonate a role. This is a test-only stand-in, not JWT verification.
- There is no `/login` mutation and no login screen — the SPA's "local session" dialog just stores whatever token string you paste into it.

## What the framework already implements [VERIFIED — read from `app-framework` source/docs]

| Concern | Where it lives | Notes |
|---|---|---|
| JWT signature/issuer/audience/client-id verification | `appfw_runtime/src/auth.rs::verify_token()` | Uses the `okta_jwt_verifier` crate against Okta's JWKS. Runs whenever the app is not in a dev-workstation/local-test-auth mode. |
| Auth config surface | `appfw_runtime/src/auth_config.rs::JwtAuthConfig::from_env()` | Reads `OKTA_ISSUER` (required), `OKTA_CLIENT_ID` (required), `OKTA_AUDIENCE` (optional, defaults to `api://default`). |
| Claim → `UserAuth` mapping | `appfw_runtime/src/extension.rs` | `company` claim → `tenant_id`; `sub` → `user_name`; `groups` → `roles`; `scp` → `scopes`. |
| Reference docs | `app-framework/examples/products/crm/backend/src/auth.md`, `.../handlers/auth/README.md`, `app-framework/docs/release/deployment-reference.md` | The CRM example product documents this end to end; `deployment-reference.md` has a dedicated "Okta/OIDC Login With oauth2-proxy Sidecar" section. |

**Gotcha called out in the framework's own docs:** the Okta authorization server must be configured to emit a `groups` claim on the access token. If it doesn't, authenticated users arrive with empty `roles` and get denied by every Rego policy check — this is an Okta-side authorization-server setting, not something fixable in app code.

## Architecture: the framework is a resource server, not an identity broker

App Framework validates bearer tokens; it does not perform the browser login redirect, PKCE exchange, or session cookie management. The framework's documented topology for real Okta login is an **`oauth2-proxy` sidecar in front of the backend**:

- The sidecar owns `/oauth2/*`, redirects unauthenticated browsers to Okta, redeems the authorization code, and injects `Authorization: Bearer <access_token>` on requests it forwards upstream.
- The backend (this app) only ever sees and validates an already-issued bearer token via `verify_token()` above.
- This requires a Web/confidential Okta application (not a public SPA client), because `oauth2-proxy` needs a client secret to redeem the code.

Consequence for the frontend: under this topology, the SPA does **not** need its own OAuth/PKCE implementation — the proxy handles the redirect dance, and the SPA just rides the session the proxy establishes. No from-scratch frontend auth flow is expected.

## Gap between "supported by the framework" and "working in this app"

The capability above is implemented and documented, but not yet wired up for `tax-doc-routing`:

- No `oauth2-proxy` sidecar exists in this app's local (`podman-compose.yml`) or deployment topology yet.
- No Okta application has been created/configured for this app (issuer, client id, audience, redirect URIs, `groups` claim).
- `OKTA_ISSUER` / `OKTA_CLIENT_ID` / `OKTA_AUDIENCE` are not set anywhere in this app's `.env`/deployment config.
- `APP_ENABLE_LOCAL_TEST_AUTH` would need to be unset (or gated per environment) once real verification is in place, so local dev and any real-auth environment don't collide.

## Suggested next step

Standing up real Okta login (sidecar + Okta app registration + env config + disabling local-test-auth outside dev) is a production topology change, not a code fix — it should go through the normal IT change-request process before implementation, with the sidecar/env changes scoped explicitly (which environments get real auth, who owns the Okta app registration, how local dev continues to work without it).
