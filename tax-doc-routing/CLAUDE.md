# CLAUDE.md

This file provides guidance to Claude Code when working with code in this
repository.

## What this repo is

`tax-doc-routing` — the Tax Document Routing product app, bootstrapped from
the App Framework's `crm-sample` profile and consuming the vendored
`../app-framework/` checkout as a Cargo path dependency. Unrelated to the
sibling `project-governance/` product folder at this same repo root; each
has its own `CLAUDE.md`.

## Before adding a new schema or entity

Read `docs/runbook/new-schema-checklist.md` first. It documents six real
generator/build gotchas hit while adding the `tax_routing` schema on top of
the `crm-sample` bootstrap — module wiring in `backend/src/lib.rs`,
`arg_type` vs `data_type` in custom methods, unused provider data sources
left over from bootstrap, kebab-case GraphQL route paths, and app-name
propagation into `podman-compose.yml`. Apply items 1, 2, 3, and 6 proactively
when generating `.appfw/model` entity files — don't wait for the build to
fail on them.

## Specs

`docs/specs/` holds the sequenced spec list for this product, derived from
the original Pega tax-document-routing platform-agnostic spec. Spec #1
(Data Model) is the foundation for every entity in
`.appfw/model/schemas/tax_routing/` — read it before modifying that schema's
entities.

## Normal verification loop

```bash
scripts/appfw product validate --json
scripts/appfw product generate
cd backend && cargo build && cd ..
scripts/appfw product test
```

Migrate and serve require Postgres env vars (no `.env` file exists yet in
this repo — see `docs/runbook/new-schema-checklist.md` item 4 for why
Mongo/MSSQL/Snowflake/Neo4j vars are NOT needed despite being demanded by
`crm-sample`'s original bootstrap):

```bash
ENV_NAME=local \
PG_HOST=127.0.0.1 \
PG_PORT=5432 \
PG_SERVICE_ACCOUNT_NAME=postgres \
PG_SERVICE_ACCOUNT_PASS=postgres \
PG_SERVICE_ACCOUNT_PASSWORD=postgres \
PG_DATABASE=app_framework \
scripts/appfw product migrate
```

Serving additionally requires Okta env vars (placeholder values are fine for
local dev — see `examples/products/crm/backend/src/handlers/auth/README.md`
in `app-framework/` for the real runtime contract):

```bash
API_PORT=8080 \
OKTA_AUDIENCE=api://default \
OKTA_ISSUER=https://example.okta.com/oauth2/default \
OKTA_CLIENT_ID=example-client-id \
scripts/appfw product serve
```

GraphQL route: `/tax-routing` (kebab-case — NOT `/tax_routing`; see runbook
item 5 for why).

## Local auth — no real Okta tenant needed for PoC work

`OKTA_AUDIENCE`/`OKTA_ISSUER`/`OKTA_CLIENT_ID` above are placeholder values
that only satisfy a startup config-loading check — they are never actually
verified against a real Okta tenant as long as `ENV_NAME=local`
(`is_dev_workstation_env()` in `appfw_runtime/src/security.rs:327-328` is
literally `ENV_NAME == "local"`).

With `ENV_NAME=local`:

- **No `Authorization` header at all** → you are automatically an admin
  user. No JWT, no Okta round-trip.
- **To test as a specific role** (e.g. `tax_staff`/`tax_admin` once spec #2's
  RBAC policies exist), send:
  ```
  Authorization: Bearer appfw-local:user=<name>;tenant=<id>;roles=tax_staff
  ```
  This is a first-class runtime test mechanism (`appfw_runtime/src/auth.rs`,
  covered by its own unit tests), not a hack.

This bypass is guarded and cannot leak into a real deployment — outside
`ENV_NAME=local`/`compose`+CI it's rejected
(`docs/architecture/concerns/threat-model.md:93-94` in `app-framework/`).
It means the entire frontend and backend, including role-gated behavior,
can be built and tested through PoC without ever setting up a real Okta
tenant — but swapping in real Okta before any non-PoC deployment is a real,
tracked step, not something this bypass makes optional forever.
