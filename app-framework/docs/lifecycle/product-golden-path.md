# Product Developer Golden Path

This is the end-to-end product path for developers and coding agents building a
downstream application with App Framework. Use it as the first narrative, then
open the linked reference docs when a step needs detail.

The short version:

```text
create product repo
  -> model schemas and data sources
  -> generate backend/database/API artifacts
  -> add product handlers and services
  -> run backend, admin UI, and product frontend
  -> verify, hand off, and release certify
```

## 1. Start From The Framework Checkout

From the framework repository root:

```bash
scripts/appfw doctor
scripts/appfw context --json
scripts/appfw product validate --json
scripts/appfw product new --list-profiles --json
```

Use:

- [Agent Skills Pack](../../agent_skills/README.md) for concise procedures when
  an agent needs workflow guidance without loading the full docs catalog.
- [CLI Quickstart](../start/cli-quickstart.md) for the first command path and
  [CLI Reference](../reference/cli.md) for the full command contract.
- [Agent Task Map](../start/agent-task-map.md) for intent-to-edit routing.
- [PoC To Enterprise Product Intake](intake-and-discovery.md) when the
  starting point is a citizen-developed workbook, static HTML app, dashboard,
  or other proof of concept.
- [Legacy Application Modernization](legacy-modernization.md) when the starting
  point is a real legacy application, codebase, data source, stored procedure
  estate, integration set, or production workflow that needs re-architecture.
- [Product Workspace Contract](../reference/product-workspace-contract.md) for downstream
  ownership and evidence expectations.
- [Generated Ownership](../start/generated-ownership.md) before editing generated-looking
  files.

## 2. Create The Product Codebase

Choose the starting path deliberately.

Use the CRM reference profile when you want a complete sample app that shows the
framework's generated backend, data package, API tests, frontend scaffold, and
release evidence:

```bash
scripts/appfw product new ../customer-crm --from current --profile crm-sample --generate --json
cd ../customer-crm
scripts/appfw doctor
scripts/appfw product validate --json
scripts/appfw product handoff --json
```

The default profile is `crm-sample`. Profile source lives under the framework
repo at:

```text
app_gen/_golden/downstream_apps
```

The CRM product template lives at:

```text
examples/products/crm
```

For a citizen-developer PoC conversion, do not treat the CRM schema as the
starting data model. Treat CRM as a reference implementation. Start from the PoC
intake guide and run `scripts/appfw product new <target> --profile product-intake`.
That command asks for app identity, schema, backend provider, whether MCP is
needed, whether Kafka is needed, and whether the product needs no UI, a
scaffold, or an enterprise UI. It records those answers in
`.appfw/poc-intake.yaml` and `.appfw/manifest.yaml`, creates product-named
backend, database, API-test, policy-test, compose, optional frontend, and docs
surfaces, and avoids CRM residue in the target repo. Inspect the
workbook/HTML/static data, then create the product-owned schema under
`.appfw/model` before generation. The target product should be shaped by the
PoC business artifacts, not by renaming CRM entities.

After creating a PoC intake shell, run:

```bash
scripts/appfw product analyze --summary --json
scripts/appfw product propose-model --summary --json
scripts/appfw product model-status --json
scripts/appfw product scaffold-model --dry-run --json
```

Review `target/appfw/product-analysis.json`, `.appfw/poc-analysis.yaml`, and
`.appfw/model-proposal.yaml` before writing final `.appfw/model`. The model
proposal is review-only; it does not write product schema source.
`model-status` writes `target/appfw/model-status.json` so the handoff can name
the current phase, blockers, and next command. `scaffold-model --dry-run`
previews starter source files only after signed model review; the real modeling
work remains with the agent/developer using source evidence.

For a legacy modernization, also start from a clean product-intake shell, but
pass `--source-kind legacy` so the scaffold writes
`.appfw/legacy-modernization.yaml`. Do not skip discovery. Use the legacy
modernization guide to inventory code, routes, jobs, auth, integrations,
database objects, data classifications, and stored procedures. Create the
product model only after the modernization slice, target architecture,
migration posture, and stored-procedure disposition are clear. The target
product should preserve business behavior through generated contracts, product
services, custom methods/DTOs, provider routines only where justified, and live
migration/API/frontend evidence where the slice requires it.

After creating a legacy intake shell, run `scripts/appfw product analyze --summary --json`,
`scripts/appfw product propose-model --summary --json`, and
`scripts/appfw product model-status --json`; review
`target/appfw/product-analysis.json`, `.appfw/legacy-analysis.yaml`, and
`.appfw/model-proposal.yaml` before completing the modernization inventory and
model source.

`appfw product new` copies the selected product template, applies the selected
profile overlay, writes product Cargo manifests that consume the approved
App Framework ProGet registry packages, validates the new app, and writes
`appfw.lock`. Keep the bootstrap JSON, selected profile name, selected
framework source or packaged release, generated artifact choice, dependency
registry posture, and `appfw.lock` in the first product PR evidence. For PoC
conversions, also keep the intake file, reviewed source-artifact list, inferred
entity model, and UI conversion notes.
For legacy modernizations, keep the static-analysis inventory, read-only data
discovery evidence, capability map, stored-procedure disposition, migration
plan, and deferred parity/security/performance risks.

## 3. Know What The Product Owns

Product developers normally edit:

```text
.appfw/manifest.yaml
.appfw/model
backend/src/handlers/<schema>/<entity>.rs
backend/src/services
.appfw/model/schemas/<schema>/tests
database/_pkg/migrations created through scripts/appfw product migrate new
frontend/
deployment overlays and product environment docs
```

Do not hand-edit generated routes, generated schema modules, generated handler
defaults, generated API test Rust, generated baseline database package files, or
`podman-compose.yml`. Change model/config/topology and regenerate instead.

When unsure:

```bash
scripts/appfw product explain ownership <path> --json
scripts/appfw product topology --json
```

## 4. Choose The Data Source

Use three source files for data-source intent:

```text
.appfw/manifest.yaml
.appfw/model/data_sources/_res.yaml
.appfw/model/schemas/<schema>/_res.yaml
```

The manifest names active topology: app identity, data sources, providers, and
schema-to-data-source bindings. It must not hold credentials or entity model
details.

The data-source config declares provider type and per-environment connection
metadata such as `local`, `compose`, host, database, TLS mode, and service
account variable names.

The schema `_res.yaml` binds that schema to one configured `data_source_name`.
Before modeling entities, decide the storage ownership posture for each schema:

- `app_owned`: the generated product owns physical tables, migrations, seeds,
  and write paths for that schema. This is the default posture for new product
  data.
- `external_read_only`: an existing database, warehouse, Fabric endpoint, or
  reporting view set is authoritative. The product owns the App Framework model,
  handlers, services, access policy, and UI, but must not generate migrations,
  seeds, or mutation paths for those physical objects.
- `routine_backed`: the product calls approved provider routines through typed
  custom-method config while the underlying database objects remain provider- or
  platform-owned.
- `code_only`: the schema is generated/runtime metadata only and must not create
  or modify physical database objects.

Use schema metadata for the posture that affects generation:

```yaml
meta:
  storage:
    mode: external_read_only
    physical_schema: dbo
```

For external authoritative sources, keep credentials out of config, classify the
data source and schema, record whether local data is sanitized/non-production,
and verify generation does not emit DDL or seed writes for the schema.

### MS SQL NTLM to remote Windows SQL (modes c and d)

Add a managed `dev` (or `tst`/`stg`) environment beside local `compose` — same
fields as `sql_password`, with `auth_mode: ntlm`:

```yaml
- name: mssql_primary
  data_source_type: MsSqlServer
  environments:
  - name: compose
    db_host: mssql
    db_port: 1433
    db_name: master
    auth_mode: sql_password
    security_profile: local_dev
    tls_mode: disabled
  - name: dev
    db_host: win-sql.dev.example.com
    db_port: 1433
    db_name: appdb
    auth_mode: ntlm
    service_account_name: DEVDOMAIN\svc-app
    security_profile: managed
    tls_mode: require
```

Daily development (native backend, no Docker): `ENV_NAME=dev scripts/appfw product serve`.
Pre-commit container verification (same config, deployment-like packaging):
`ENV_NAME=dev podman-compose up backend`. Generated compose passes
`${ENV_NAME:-compose}` to the backend service.

For a connection-only NTLM smoke against the same Windows SQL host (optional
`MSSQL_NTLM_SQL` query override), use `scripts/ci/mssql-ntlm-live-smoke.sh`
(native) or `MSSQL_NTLM_SMOKE_LAUNCH=container` (FreeTDS test-runner image).

Example flow for changing provider topology:

```bash
scripts/appfw product topology --json
scripts/appfw product validate --json
scripts/appfw product generate
scripts/appfw product generate --check --json
```

Regeneration updates generated local infrastructure such as `podman-compose.yml`.
Do not trim provider services from compose by hand.

For local or certification runs, prefer schema-specific data-source overrides
when only one product schema should move:

```bash
APP_CRM_DATA_SOURCE_NAME=pg_primary ENV_NAME=local API_PORT=8080 scripts/appfw product serve
```

Use `APP_DATA_SOURCE_NAME=<data-source>` only for deliberate whole-app smoke
runs where every schema should use the same data source. Release certification
should use schema-specific overrides so framework/system schemas stay on their
intended host.

## 5. Create The Application Data Model

Application shape starts in:

```text
.appfw/model
```

The usual model files are:

```text
.appfw/model/data_sources/_res.yaml
.appfw/model/schemas/<schema>/_res.yaml
.appfw/model/schemas/<schema>/entity_types/*.yaml
.appfw/model/schemas/<schema>/relationships/*.yaml
.appfw/model/schemas/<schema>/gql_enum_types/*.yaml
.appfw/model/schemas/<schema>/seeds/*.yaml
.appfw/model/schemas/<schema>/tests/*.yaml
```

Add or change entities, properties, relationships, enums, seeds, and generated
API scenarios there. Validate before generation:

```bash
scripts/appfw product validate --json
```

Use [Schema Design](../model/schema-design.md) and the generated config contract:

```text
.appfw/model/_specs/CONFIG_CONTRACT.md
.appfw/target/appfw/config_contract.json
```

## 6. Generate The Backend

When model or topology changes should update generated output:

```bash
scripts/appfw product validate --json
scripts/appfw product generate
git diff
scripts/appfw product generate --check --json
scripts/appfw product test
```

Generation writes backend, database package, API-test, frontend scaffold
contract, ownership, and local infrastructure artifacts for the product shape.
Review generated diffs; do not hide drift diagnostics.

## 7. Add Custom Methods And Product Services

Declare app-specific operations in entity config:

```yaml
custom_methods:
- name: account_health
  kind: Query
  mcp_enabled: true
  args:
  - name: account_id
    arg_type: String
  return_type: serde_json::Value
```

Then regenerate:

```bash
scripts/appfw product validate --json
scripts/appfw product generate
```

Generated defaults land in:

```text
backend/src/handlers/<schema>/generated.rs
```

Product-owned overrides live in:

```text
backend/src/handlers/<schema>/<entity>.rs
```

Keep handlers thin. Put durable domain workflows, calculations, cross-entity
logic, and external integration code in:

```text
backend/src/services
```

Verify product extension boundaries:

```bash
scripts/appfw product boundary-check --json
scripts/appfw product test --fast
```

Use [Custom Resolvers](../model/custom-methods-and-routines.md), [Service Layer](../model/service-layer.md),
and [Product Extension API](../reference/product-extension-api.md) for signatures and imports.

## 8. Run And Test Locally

Start the generated local provider services your app uses. For the default CRM
PostgreSQL path:

```bash
docker compose -f podman-compose.yml up -d postgres
```

Podman users can use the equivalent `podman compose` command. For other active
providers, start the matching generated service name such as `mongo`, `mssql`, or
the Snowflake profile documented in [Getting Started](../start/getting-started.md).
When topology enables Kafka ingress, generated compose adds a local single-node
KRaft broker and topic smoke check behind the `kafka` profile:

```bash
docker compose -f podman-compose.yml --profile kafka up -d broker kafka-smoke
```

Containerized workers use `broker:29092`; host-side tools use
`localhost:${APP_KAFKA_HOST_PORT:-9092}`.

Prepare the database package:

```bash
scripts/appfw product migrate
```

Run the backend:

```bash
ENV_NAME=local API_PORT=8080 scripts/appfw product serve
```

The wrapper sets local defaults for `VERSION` and `RUST_MIN_STACK`; product
developers may override them explicitly when reproducing packaged runtime
settings.

Generated GraphQL routes are schema-specific:

```text
http://127.0.0.1:8080/crm
http://127.0.0.1:8080/system
```

Run generated product API scenarios when the backend and required data sources
are running:

```bash
scripts/appfw product api-test
```

Run the normal inner loop when live services are unavailable:

```bash
scripts/appfw product validate --json
scripts/appfw product test --fast
scripts/appfw product handoff --json
```

## 9. Build The Product SPA For The Backend Image

The default production topology is one backend image. If the product has a
frontend, build the Vite SPA into the backend package before building the image:

```bash
cd frontend
npm install
npm run build
```

Product starters write the deployable bundle to:

```text
backend/product_dist
```

At runtime the backend serves this bundle at `/` when
`APP_PRODUCT_UI_ENABLED=true`. Generated API, admin, health, readiness, metrics,
and MCP paths remain backend-owned and are not masked by the SPA fallback.

## 10. Run The Admin UI

The admin UI is the framework-owned, model-driven operations console. It is not
the product frontend.

For the backend-hosted production-style route, build the admin bundle from the
framework checkout:

```bash
cd admin_ui
npm install
npm run build
```

From the framework checkout, repository-root `scripts/appfw` serves the CRM
sample backend by default, so package the built admin bundle into the product
backend or point the product backend at the framework bundle.

In a downstream product app, either package/copy the built bundle into that
product's `backend/admin_dist`, or point the product backend at the framework
bundle:

```bash
APP_ADMIN_UI_DIST_DIR=/absolute/path/to/app-framework/admin_ui/dist \
APP_ADMIN_UI_ENABLED=true \
ENV_NAME=local \
API_PORT=8080 \
scripts/appfw product serve
```

Open:

```text
http://127.0.0.1:8080/admin
```

For admin UI frontend iteration, run the backend on port `8080`, then from the
framework checkout:

```bash
cd admin_ui
npm run dev
```

Open:

```text
http://127.0.0.1:5173/admin/
```

The Vite dev server proxies `/admin/model`, `/crm`, and `/system` to the backend.

## 10. Build The Product Frontend

Product frontends live in the product `frontend/` workspace. Do not fork
`admin_ui` into product screens.

Use the CRM frontend as the reference architecture:

```text
examples/products/crm/frontend
```

It demonstrates:

- `src/generated/appfw-ui-contract.ts` as the replaceable generated UI contract.
- `.appfw-ui/ownership.json` for generated, scaffold, and human-owned frontend
  roots.
- `src/app`, `src/components`, `src/scaffold`, `src/lib`, `src/styles`, and
  `src/design` as reusable scaffold surfaces.
- `src/features/**` as human-owned product workflow code.
- `src/lib/appfwClient.ts`, `authContext.ts`, and `tenantContext.ts` as the typed
  API/auth/tenant boundary.
- PDS `--pds-*` design tokens.

Run the CRM frontend:

```bash
cd examples/products/crm/frontend
npm install
npm run dev
```

Then keep frontend evidence with:

```bash
npm run appfw:check
npm test
npm run typecheck
npm run build
npm run release:evidence
```

For a new product frontend, keep the same ownership model, replace CRM feature
screens with product workflows, and retain frontend command output in
`scripts/appfw product handoff --json`.

Use [Frontend Starter Contract](../frontend/product-frontend.md) before claiming a
prototype is product-ready.

Scaffold status: the CRM frontend is the canonical reference implementation and
the selected profile advertises its scaffold evidence through
`scripts/appfw product new --list-profiles --json`. The next maintainability milestone is
for `appfw product new --generate` to create a downstream product frontend that already
has the same ownership manifest, generated-contract placeholder, package
checks, and `frontend-test --json` evidence path. Until that is automated,
product teams should copy the scaffold shape, not the framework-owned admin UI
or one-off screenshots.

## 11. Enable The MCP Server Locally

MCP is optional and disabled by default. Enable it only in approved local or
non-release environments:

```bash
APP_MCP_ENABLED=true ENV_NAME=local API_PORT=8080 scripts/appfw product serve
```

The endpoint is:

```text
POST /mcp
```

App-defined custom methods appear in MCP operation discovery only when their
config sets:

```yaml
mcp_enabled: true
```

MCP requires authenticated users plus the configured role or scope gate outside
local development. Keep `APP_MCP_ENABLED=false` in release pipelines unless a
dedicated certified MCP release lane exists. See [Agentic MCP Server](../runtime/mcp.md).

## 12. Verify And Hand Off

For config-only or product handler/service changes:

```bash
scripts/appfw product validate --json
scripts/appfw product boundary-check --json
scripts/appfw product test --fast
scripts/appfw product handoff --json
```

For generated artifact cascades:

```bash
scripts/appfw product validate --json
scripts/appfw product generate
scripts/appfw product generate --check --json
scripts/appfw product test
scripts/appfw product handoff --json
```

For product frontend work, add the product frontend checks:

```bash
npm run appfw:check
npm run typecheck
npm run test
npm run build
```

If a live backend, data source, provider, or frontend command is unavailable,
record the skipped command, concrete blocker, and risk in:

```text
target/appfw/agent-handoff.json
```

## 13. Release Certify

Application release candidates should retain the risk-appropriate evidence:

```bash
scripts/appfw product validate --json
scripts/appfw product test
scripts/appfw product migrate plan --json
scripts/appfw product migrate lint --phase all --json
scripts/appfw product migrate drift --json
scripts/appfw product migrate rollback-guide --json
scripts/appfw product handoff --json
```

When the backend and data sources are running, add:

```bash
scripts/appfw product api-test
```

Framework/provider release work uses the stricter release gate:

```bash
scripts/appfw framework provider-test --all --json
scripts/appfw framework release-check --json
```

Release-grade all-provider certification requires separate provider-backed
backend URLs:

```text
API_TEST_BASE_URL_POSTGRES
API_TEST_BASE_URL_MONGO
API_TEST_BASE_URL_MSSQL
API_TEST_BASE_URL_SNOWFLAKE
```

Use [Release Gate, Bitbucket, and ArgoCD](../release/release-gate-ci-cd.md),
[Provider Certification](../runtime/provider-certification.md), and
[Deployment Reference](../release/deployment-reference.md) for regulated promotion.

## 14. Prove The Golden Downstream Path

For framework changes that affect product creation, generated ownership,
frontend scaffolding, release evidence, or upgrade behavior, add disposable
downstream proof when practical:

```bash
scripts/appfw framework intake-proof --json
scripts/appfw framework golden-downstream --json
scripts/appfw framework golden-downstream --profile crm-sample --execute --json
```

The executable command creates the disposable app and records the create-to-handoff
steps in `target/appfw/golden-downstream.json`. To debug the same lifecycle
manually, run the equivalent downstream commands:

```bash
tmp_app="$(mktemp -d)/customer-crm"
scripts/appfw product new "$tmp_app" --from current --profile crm-sample --generate --json
cd "$tmp_app"
scripts/appfw product validate --json
scripts/appfw product generate --check --json
scripts/appfw product test --fast --json
scripts/appfw product handoff --json
```

If the product includes a frontend scaffold and dependencies are available,
retain:

```bash
scripts/appfw product frontend-test --json
```

If live provider services are available, retain:

```bash
scripts/appfw product api-test
scripts/appfw product load-test-suite --json
scripts/appfw framework provider-performance --json --all
```

When this disposable downstream proof is skipped, the handoff must say whether
the blocker was write-heavy setup, missing package dependencies, missing live
providers, or release-environment ownership.
`scripts/appfw framework intake-proof --json` retains
`target/appfw/product-intake-proof.json` and
`target/appfw/product-intake-proof/model-status.json` plus
`target/appfw/product-intake-proof/frontend-residue-check.json` and
`target/appfw/product-intake-proof/frontend-scaffold-check.json`,
`scripts/appfw framework golden-downstream --json` retains
`target/appfw/golden-downstream.json`, and `scripts/appfw framework docs-check --json`
retains `target/appfw/lifecycle-evidence-checklist.json` plus
`target/appfw/maintainability-contract.json` as the low-cost contract. When CI
owns the write-heavy proof, run
`scripts/appfw framework golden-downstream --profile crm-sample --execute --json`
so the same artifact contains per-step disposable downstream execution evidence.

## 15. Keep The Product Repo Documented

Every product repo should have enough documentation for a new human or agent to
reproduce the product safely:

```text
../README.md
.appfw/manifest.yaml
appfw.lock
backend/README.md or runtime notes
frontend/README.md when a product UI exists
deployment/environment notes
release evidence and skipped-check notes
```

The product README should name:

- product purpose, owners, risk/data classification, and production scope.
- active schemas and data sources.
- local data-source setup and required secret variables.
- backend run command and GraphQL routes.
- admin UI run path.
- product frontend run/build/test commands.
- custom method/service locations.
- MCP posture for local and release environments.
- verification, handoff, release, and rollback commands.

When docs or agent-facing examples change, run:

```bash
scripts/appfw framework docs-check --json
scripts/appfw product handoff --json
```
