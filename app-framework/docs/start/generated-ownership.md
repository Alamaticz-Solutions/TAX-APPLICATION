# Generated Ownership

Use this page when deciding where an agent should edit. It is a non-mutating
reference; it does not require running the generator.

For downstream app ownership in the current copied-framework layout, read
[Product Workspace Boundaries](../reference/product-workspace-boundaries.md) first. This page
then gives path-level routing rules.

## Edit Surface Matrix

| Task | Edit First | Verify |
| --- | --- | --- |
| Change app name or topology assertions | `.appfw/manifest.yaml` | `scripts/appfw product topology --json`; `scripts/appfw product validate --json` |
| Add or change an entity, property, enum, seed, test, or relationship | `.appfw/model` | `scripts/appfw product validate --json`; `scripts/appfw product test` |
| Change the config language or validation rules | `app_gen/src/config_contract.rs`, `app_gen/src/validation.rs` | `scripts/appfw framework validate --json` |
| Change PDS design-token values or render metadata | `appfw_ui/pds_health/tokens/tokens.dtcg.json`, then run `node scripts/generate-pds-tokens.mjs`; do not edit `pdsTokens.css` or `pdsTokens.ts` directly | `node scripts/generate-pds-tokens.mjs --check --json`; `node scripts/check-pds-tokens.mjs --json` |
| Change generated Rust/database/test shape for all apps | `app_gen/_templates` or `app_gen/src` | `scripts/appfw framework validate --json`; `scripts/appfw framework generate`; `scripts/appfw framework generate --check --json`; `scripts/appfw framework test` |
| Change local dev infra providers or compose services | `.appfw/manifest.yaml`, `.appfw/model/data_sources/_res.yaml` | `scripts/appfw product topology --json`; `scripts/appfw product generate`; `scripts/appfw product generate --check --json` |
| Implement app-specific custom behavior | `backend/src/handlers/<schema>/<entity>.rs` and product-owned service modules such as `backend/src/services` | `scripts/appfw product validate --json`; `scripts/appfw product test` |
| Change provider query/filter/sort semantics | `backend/src/data/clients/<provider>` plus shared contract tests | `scripts/appfw framework validate --json`; `scripts/appfw framework test` |
| Change the agentic MCP runtime surface | `appfw_runtime/src/mcp` for schema-neutral protocol/operation contracts, `backend/src/mcp` for remaining schema-coupled runtime source, `app_gen/_templates/backend/mcp` for generated operation dispatch, shared `DataAccess`, and route templates when mounting changes | `scripts/appfw framework validate --json`; `scripts/appfw framework generate`; `scripts/appfw framework generate --check --json`; `scripts/appfw framework test` |
| Change access-control behavior | Rego policy config and product `rego_test` fixtures/assertions; framework verifier changes belong in `appfw_test/src/policy` | `scripts/appfw product validate --json`; `scripts/appfw product policy-test`; `scripts/appfw product test` |
| Change downstream app bootstrap defaults | `app_gen/_golden/downstream_apps` profile manifests and overlays | `scripts/appfw product new --list-profiles --json`; `scripts/appfw framework docs-check --json` |
| Convert a mobile mockup or change a React Native mobile app | Product `mobile/`, `docs/frontend/mobile-react-native.md`, `.appfw/target/appfw/mobile-rn-conversion-plan.json`; future generated surfaces under `mobile/src/generated` | `scripts/appfw product mobile-plan --ui-artifact <mockup.html> --json`; `scripts/appfw product validate --json`; `scripts/appfw product generate --check --json`; React Native checks when mobile source exists |
| Change docs only | Relevant Markdown files | stale-reference scan; `scripts/appfw framework validate --json`; `scripts/appfw framework docs-check --json` |

## Ownership Rules

- `.appfw/manifest.yaml` is product-owned topology source. It names the app and
  lists schemas/data sources, but it does not replace `.appfw/model`.
- `.appfw/model` is application source.
- `app_gen/_templates` and `app_gen/src` are generator source.
- `app_gen/_golden/downstream_apps` is framework-owned downstream bootstrap
  template source for `scripts/appfw product new`. Each profile's `profile.json` is the
  machine-readable profile contract; keep its name, template, overlay, and
  verification commands aligned with the registered profile. Profile overlays
  should hold app-facing defaults and onboarding only, not framework runtime,
  provider, security, or generator behavior.
- Most files under `backend/src/schemas`, `backend/src/routes`, generated
  handler module files such as `backend/src/handlers/mod.rs`,
  `backend/src/handlers/<schema>/mod.rs`, and
  `backend/src/handlers/<schema>/generated.rs`, `backend/config/generated`,
  `database/_pkg`, and `api_tests/src/schemas` are generated.
- `appfw_test/src/harness` and the framework-root
  `api_tests/src/provider_contracts.rs`,
  `api_tests/src/provider_semantic_contracts.rs`, and
  `api_tests/src/provider_schema_contracts.rs` are framework-owned API test
  infrastructure. Product API scenario source belongs in
  `.appfw/model/schemas/<schema>/tests`; generated scenario Rust belongs in
  product `api_tests/src/schemas`.
- `appfw_test/src/policy` owns the policy verifier engine and shared Rego
  access input/result contract. Product `rego_test` crates should keep only
  policy fixtures, product-specific assertions, and thin local fixture-path
  glue.
- Framework-owned backend/runtime/provider unit-test scaffolding stays in
  framework crates and reusable harnesses. CRM sample and `appfw product new`
  output should not carry copied `#[cfg(test)]` modules for framework plumbing
  or local provider `test_support` helpers; product teams add focused tests
  around product-owned handlers, services, API scenarios, and policy fixtures.
- `podman-compose.yml` is generated local dev infra. The source of truth is app
  topology plus data-source environment config. Do not hand-trim providers out
  of compose; remove or change the topology/data-source entry and regenerate.
- `backend/src/handlers/<schema>/<entity>.rs` files are product-owned handler
  implementations after creation. They are the visible app extension surface for
  custom method implementations and intentional standard-operation overrides.
  They import generated defaults so normal CRUD/query behavior stays in
  generated output until the product explicitly overrides it locally. Mark
  intentional standard operation overrides with `appfw: override-standard`.
- `backend/src/services` is product-owned. CRM-sample products include the
  module as a first-class extension point. Use it for durable domain workflows,
  cross-entity orchestration, calculations, and external integrations that
  should not live in GraphQL glue.
- `admin_ui` and `backend/src/admin_ui.rs` are framework-owned source for the
  backend-hosted, model-driven admin console. Downstream products should
  consume explicit admin configuration/branding extension points rather than
  carrying admin UI source forks. `backend/admin_dist` is local Vite build
  output and should not be edited by hand. See `admin_ui/README.md` for the
  Vite dev-server and production build flow.
- `backend/product_dist` is product SPA build output and should not be edited
  by hand. Regenerate it from the product `frontend` workspace with
  `npm run build`. In the default one-image topology, the backend serves this
  bundle at `/` while preserving generated API, admin, health, and metrics
  routes.
- `appfw_ui/pds_health` is framework-owned source for the PDS Health design
  system. Product frontends should consume the tokens, component guidance, and
  scaffold assets from this surface instead of copying CRM screens or forking
  admin UI internals.
- `appfw_ui/pds_health/tokens/tokens.dtcg.json` is the canonical PDS
  design-token source. `pdsTokens.css` and `pdsTokens.ts` are generated
  consumption artifacts; regenerate and drift-check them with
  `scripts/generate-pds-tokens.mjs` instead of editing them directly.
  `--bootstrap-from-files` is a review-only migration escape hatch for
  deliberately capturing an inspected CSS/TS state, not the normal authoring
  workflow.
- `frontend/package.json`, `frontend/src/app`, `frontend/src/components`,
  `frontend/src/scaffold`, `frontend/src/lib`, `frontend/src/styles`,
  `frontend/src/design`, and `frontend/scripts` are product-owned frontend
  scaffold surfaces. Keep them aligned with `.appfw-ui/ownership.json`, PDS
  tokens, generated UI contracts, package checks, and
  `scripts/appfw product frontend-test --json`.
- Product `mobile/` source (`mobile/app`, `mobile/src/features`,
  `mobile/src/components`, `mobile/src/lib`, `mobile/src/test`,
  `mobile/app.json`, `mobile/eas.json`, `mobile/package.json`) is
  product-owned React Native source until the mobile generator lands. Keep it
  aligned with `mobile/.appfw-mobile/ownership.json`, generated mobile
  contracts, PDS native tokens, and
  [React Native Mobile App Contract](../frontend/mobile-react-native.md).
- Future generated mobile files under `mobile/src/generated`,
  `mobile/src/design/pdsNativeTokens.ts`, and `.appfw-mobile` metadata must be
  produced by `app_gen` and drift-checked; do not hand-edit generated mobile
  output once the target ships.
- `appfw_runtime/src/mcp` owns schema-neutral MCP protocol and operation
  contracts. `backend/src/mcp` is still framework-owned source for the remaining
  schema-coupled optional MCP endpoint, except
  `backend/src/mcp/generated_operations.rs`, which is generated from
  `app_gen/_templates/backend/mcp` and the entity model. Keep MCP as a transport
  adapter over public handler-level operations, shared `DataAccess`, `QueryIR`,
  policy, redaction, and provider execution.
- `backend/src/data`, provider clients, app config loading, MCP/admin runtime
  wiring, and remaining backend host plumbing are framework runtime surfaces in
  the current layout. Generated route wiring should register only providers
  that back active schemas, and product-shaped backends should not carry
  inactive provider source copies just because the framework certifies them.
  Shared CORS, GraphiQL, security/rate limiting,
  observability, provider pool stats, product identity contracts (`UserAuth`,
  `Claims`), and generated route-shell assembly now live in `appfw-runtime` and
  are re-exported or consumed through thin backend facades during extraction.
  Product code should usually enter through
  `DataAccess`, not direct provider clients.
- Deployment overlays and app environment docs are product-owned when they live
  in a downstream app repo. `podman-compose.yml` is generated from app topology
  and data-source config so local dev services reflect enabled providers rather
  than every provider supported by the framework.
- Schema-level relationship files are the product-owned source of truth for
  navigation per [ADR 0003](adr/0003-relationship-source-of-truth.md). Product
  authors name relationship endpoints and navigation fields there; `app_gen`
  generates the entity navigation properties. Do not hand-author `NavToOne`,
  `NavToMany`, or `ManyToMany` props in entity type files.
- Do not run `scripts/appfw product generate` only to inspect ownership. It writes
  generated artifacts. Use this reference first, then generate only when the task
  requires regenerated output.

## Handler Extension Contract

Handler implementation files are create-once and preserved to protect product
code. Regenerated handler defaults live in
`backend/src/handlers/<schema>/generated.rs`, and product handler files import
the matching generated module:

```rust
#[allow(unused_imports)]
pub(crate) use super::generated::account::*;
```

GraphQL and MCP adapters continue to call
`backend/src/handlers/<schema>/<entity>.rs`. If the product handler defines a
local `<method>_impl` function, that local implementation is used. If it does
not, the imported generated default is used instead. Generated custom defaults
return an explicit “not implemented yet” error.

## Artifact Manifest

When generation has already been run, inspect the generated manifest:

```bash
scripts/appfw product manifest --json
```

If the manifest is missing, do not create it solely for reconnaissance. Decide
from the path rules above, or run generation only when the requested change
requires generated artifacts.

## Drift Check

`scripts/appfw framework generate --check --json` is the right check for generator and
template changes. Treat any drift failure as a diagnostic and report it rather
than using it as a first-run health check.

## Agent Handoff

At handoff, emit the machine-readable changed-surface and drift summary:

```bash
scripts/appfw product handoff --json
```

The report is written to `target/appfw/agent-handoff.json`.
