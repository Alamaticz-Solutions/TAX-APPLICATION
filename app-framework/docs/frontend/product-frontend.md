# Frontend Starter Contract

App Framework frontends should be model-driven products, not handcrafted
appendages beside the generated backend. This contract gives agents a small,
repeatable path for building a product UI while the generated frontend package
is still taking shape.

The backend-hosted admin UI remains the framework-owned operations console.
Product frontends are product-owned user experiences that consume generated
API/model contracts, the PDS Health design system, and the same auth, tenant,
validation, policy, and release-evidence expectations as the backend.
React Native mobile apps are a sibling product experience target, not a web
layout variant; use [React Native Mobile App Contract](mobile-react-native.md)
when mobile-native navigation, secure storage, push, offline, or app-store
distribution is in scope.

Use [PDS Health Enterprise Design System](pds-health-design-system.md) for
framework-owned brand, token, component, and scaffold maturity guidance. Use
the CRM frontend as a reference implementation and framework E2E fixture only,
not as the base UI for new products.

## Source Inputs

Use these sources before inventing UI data shapes:

| Source | Purpose |
| --- | --- |
| `.appfw/model` | Product model, relationships, custom methods, facets, seeds, and API scenarios. |
| `/admin/model` | Runtime model summary used by the admin UI for entity, schema, provider, filter, and health metadata. |
| Generated GraphQL schema | Operation names, input types, result shapes, pagination, filters, sorts, aggregates, and validation responses. |
| `admin_ui/src/types.ts` | Current checked TypeScript shape for the admin model endpoint. |
| `appfw_ui/pds_health/tokens/tokens.dtcg.json` | Canonical framework source for PDS design-token values and render metadata. |
| `appfw_ui/pds_health/tokens/pdsTokens.css` | Generated stylesheet exposing stable `--pds-*` design tokens to consumers. |
| `appfw_ui/pds_health/tokens/pdsTokens.ts` | Generated typed token view and CSS variable handles for React code. |
| `appfw_ui/pds_health/components` | Framework-owned PDS component source for reusable actions, command access, fields, shell, overlays, feedback, and data surfaces. |
| `@appfw/pds-health-components` package catalog exports | `pdsComponentCatalog`, `pdsComponentFamilies`, and `pdsAgentDecisionGuide` for code-level component and recipe discovery. |
| `appfw_ui/pds_health/reference/catalog.json` | Agent-readable PDS component family manifest, source API map, agent decision guide, maturity levels, evidence requirements, readiness contract, entrypoints, and do/avoid guidance. |
| `appfw_ui/pds_health/reference/index.html` | Static product-neutral PDS Component Catalog fallback for component source ownership, agent decision recipes, states, density, themes, accessibility notes, readiness ledger, and usage examples. |
| `appfw_ui/pds_health/catalog-app` | Interactive PDS Component Catalog for searchable live examples, light/dark review, and copy-ready imports/usage snippets. It consumes the canonical manifest and real component source; do not copy product fixture nouns into it. |
| `target/appfw/pds-component-check.json#component_api_inventory` | Retained evidence mapping each PDS component to its family, source file, and props type after `scripts/check-pds-components.mjs --json` runs. |
| `target/appfw/pds-component-check.json#agent_decision_guide` | Retained evidence mapping common product workflow intents to starter PDS component sets. |
| `@appfw/pds-health/tokens/pdsTokens.css` | Canonical token stylesheet import for admin and product frontends. Product-local token copies are rejected by U6 fork-check; new product UI code should consume the package alias. |

Do not infer product UI contracts from screenshots, seed data, or copied sample
queries when generated model/API contracts are available.

## Starter Shape

A downstream product frontend should converge on this logical layout:

```text
frontend/
|-- src/
|   |-- app/
|   |-- components/
|   |-- features/
|   |   `-- <workflow>/
|   |-- generated/
|   |   |-- appfw-ui-contract.ts
|   |   `-- graphql/
|   |-- lib/
|   |   |-- appfwClient.ts
|   |   |-- authContext.ts
|   |   `-- tenantContext.ts
|   |-- styles/
|   |   `-- index.css
|   |-- design/
|   |   `-- pdsTokens.css
|   `-- test/
|-- .appfw-ui/
|   |-- ownership.json
|   `-- scaffold-manifest.json
|-- scripts/
|   `-- check-scaffold.mjs
|-- package.json
|-- tsconfig.json
`-- ../README.md
```

The generated directory is replaceable. Product workflow code under
`features/` is human-owned and should depend on exported contract types instead
of duplicating raw GraphQL response shapes.

## Enterprise Scaffold Path

Use this path when creating or hardening a downstream product frontend:

| Stage | Agent Action | Evidence |
| --- | --- | --- |
| Discover | Read `.appfw/model`, generated GraphQL schema, `/admin/model`, and the product workflow brief. | Screen-to-entity/workflow map and named generated operations. |
| Scaffold | Create product-owned `frontend/` files for app shell, command palette, typed API client, auth context, tenant context, generated contract imports, PDS token import, and first workflow routes. | Diff limited to product frontend or generated contract scaffold. |
| Package | Emit `.appfw-ui/scaffold-manifest.json`, expose `npm run appfw:check`, and document required frontend release evidence. | Offline package check passes and names contract, ownership, roots, and scripts. |
| Wire Data | Replace mock data with typed `AppfwResult<T>` calls and explicit validation, policy-denied, auth, provider, and unknown error categories. | Typed client tests and component states for loading, empty, validation, denied, and unexpected errors. |
| Harden | Add role and tenant route guards, request/correlation ID propagation, destructive-action confirmations, pagination/sorting/filtering, responsive layouts, keyboard/focus behavior, and accessible labels. | Typecheck/lint/tests, accessibility smoke evidence, screenshots or review notes when useful. |
| Release | Run backend validation and drift checks plus product frontend build/tests before handoff. | `scripts/appfw product handoff --json` and retained frontend command output. |

The first screen should be usable product workflow, not a marketing page. A new
product starter should open on generated product entities, work queues, or
workflow shells derived from the product model. CRM-specific screens such as
accounts, contacts, pipeline, or activities belong only in the CRM reference
frontend.

## Production Serving

The default deployment topology is one backend image. The backend serves the
Rust API routes, the product SPA when present, and the admin UI when enabled.
Product frontends should build their Vite bundle into the backend package at:

```text
backend/product_dist
```

At runtime, `appfw-runtime` mounts that bundle at `/` when
`APP_PRODUCT_UI_ENABLED=true` and `backend/product_dist/index.html` exists.
Static Vite assets are served under `/assets/*`; unmatched browser routes fall
back to `index.html`. The fallback reserves backend-owned paths such as
`/admin`, `/mcp`, health/readiness/metrics routes, and generated schema API
prefixes so a missing API route does not silently become a frontend page.

Declare the product UI serving posture in `.appfw/manifest.yaml` so generated
checks and release evidence know what the product image is supposed to serve:

```yaml
ui:
  product_spa:
    enabled: true
    packaging: backend-product-dist
  admin_ui:
    enabled: false
    packaging: disabled
```

Use `ui.admin_ui.enabled:true` with `packaging: backend-admin-dist` only when
the product image intentionally embeds the framework admin UI and the release
bundle includes `backend/admin_dist/index.html`.

Use a separate nginx/static frontend only for a product that explicitly needs
an independent UI release cadence, CDN/edge caching, or separate frontend
scaling. That topology must be named in deployment docs and release evidence.

Use the shared PDS `CommandPalette` for workspace search, generated route
discovery, and frequent actions. Product code should provide product-owned
command labels and routes, while the component supplies the keyboard shortcut,
search field, popover behavior, and token-backed styling.

## Frontend Ownership Boundaries

Product teams should edit:

- `frontend/src/app`, `frontend/src/features`, `frontend/src/components`,
  `frontend/src/lib`, `frontend/src/styles`, and product frontend tests.
- `frontend/src/generated` only when it is generated or explicitly scaffolded as
  a replaceable contract boundary.
- `frontend/.appfw-ui/scaffold-manifest.json` when emitted by `app_gen`; it is
  deterministic package metadata for downstream check scripts and CI evidence.
- product README, package scripts, and deployment configuration for the product
  frontend.

Product teams should not edit:

- backend-hosted `admin_ui` source to create product screens;
- generated backend route, schema, handler-default, provider, QueryIR, auth, or
  data-access internals from frontend work;
- untyped copied GraphQL snippets inside components when generated contracts or
  typed client functions are available;
- mock-only data paths after a screen is claimed as product-ready; or
- secrets, bearer tokens, tenant data, PHI/ePHI, private keys, local `.env`
  files, or connection strings.

## UI Contract Shape

The eventual generated UI contract should be a TypeScript-first collection
with these stable families:

| Family | Required Content |
| --- | --- |
| `entities` | Entity names, captions, primary keys, caption fields, table/read-only/audited flags, facets, and display metadata. |
| `fields` | Scalar, enum, relationship, array, object, required, read-only, computed, default, validation, and formatting hints. |
| `relationships` | To-one, to-many, many-to-many, target entity, local/foreign keys, junction metadata, and navigation labels. |
| `operations` | Standard methods, custom methods, GraphQL operation names, variables, returns, selection presets, and disabled reasons. |
| `filters` | Provider-supported operators by data type, conjunction support, sortable fields, and unsupported reasons. |
| `pagination` | Default page size, maximum page size, cursor/keyset hints, count behavior, and aggregate availability. |
| `auth` | Required roles/scopes when exposed, tenant context requirements, policy-denied response shape, and local-dev auth hints. |
| `errors` | Validation error shape, policy-denied shape, provider-normalized error categories, request IDs, and correlation IDs. |
| `design` | PDS token import path, semantic token aliases, density defaults, and accessibility baseline. |

Until that generator exists, agents should hand-author narrow types near the
feature being built and keep the names aligned with this table so the code can
be replaced by generated exports later.

## Typed API Pattern

Frontend API calls should be routed through a small typed client. The pattern
is the same whether the transport is GraphQL today or a generated operation
client later:

```ts
export type AppfwRequestContext = {
  schemaName: string;
  operationName: string;
  requestId?: string;
  correlationId?: string;
};

export type AppfwResult<TData> = {
  data: TData;
  requestId: string;
  correlationId: string;
  responseMs: number;
};

export type AppfwError = {
  message: string;
  category: "validation" | "policy_denied" | "auth" | "provider" | "unknown";
  requestId?: string;
  correlationId?: string;
  validation?: Record<string, string[]>;
};
```

Feature code should receive typed `AppfwResult<TData>` values and render
loading, empty, validation, policy-denied, and unexpected-error states
explicitly. It should not inspect untyped `errors[0].message` strings inside
components.

## Prototype To Enterprise Checklist

Before a frontend prototype becomes a product app, complete this checklist:

- Inventory screens, workflows, roles, tenants, entities, relationships,
  custom methods, external dependencies, and optimistic update paths.
- Map every screen to `.appfw/model`, generated operations, service-backed
  custom methods, or a documented product integration boundary.
- Replace mock data and local-only state with typed generated API calls.
- Add auth context, tenant context, request/correlation IDs, and policy-denied
  UX for every protected action.
- Route protected screens through role-aware and tenant-aware guards; show
  denied states instead of hiding failed authorization behind generic errors.
- Add validation summaries, field-level errors, loading states, empty states,
  retry states, and destructive-action confirmations.
- Add pagination, sorting, filtering, relationship navigation, stale data
  handling, optimistic-concurrency conflict UX, and retry/backoff behavior where
  workflows need them.
- Use PDS Health `--pds-*` tokens or typed token exports from
  `appfw_ui/pds_health` before introducing product-local aliases.
- Read `appfw_ui/pds_health/reference/catalog.json` before adding a
  product-local component; if an exported PDS family covers the need, consume
  it first and keep product-specific behavior in product code. Treat
  `release-gated` families as required starter candidates unless the product
  documents a local exception.
- Use `target/appfw/pds-component-check.json#component_api_inventory` after
  running `scripts/check-pds-components.mjs --json` when you need to locate a
  component's source file or props type without scraping the package manually.
- Use `target/appfw/pds-component-check.json#agent_decision_guide` for common
  workflow starts such as generated entity workspaces, generated forms,
  governed actions, dashboards, workspace navigation, and service feedback.
- In frontend or generator code, prefer importing `pdsComponentCatalog`,
  `pdsComponentFamilies`, or `pdsAgentDecisionGuide` from
  `@appfw/pds-health-components` over duplicating local component maps.
- Use shared PDS generated-form controls for typed text, date, time, datetime,
  lookup, multi-select, file evidence, switch, checkbox, hint, and validation
  states before adding product-local controls.
- Use shared PDS overlay controls for modal dialogs, drawers, popovers,
  tooltips, and destructive confirmations so focus, escape, return-focus, and
  screen-reader behavior remain consistent.
- Use shared PDS analytics controls for KPI tiles, metric trend states, chart
  shells, and legends before adding product-local dashboard chrome. Keep
  product-specific chart calculations and chart renderer choices in product
  code.
- Add accessibility checks for keyboard navigation, focus states, labels,
  color contrast, and responsive text fit.
- Add performance budgets for large lists, expensive aggregates, repeated
  requests, and client-side filtering that should move to generated API filters.
- Add frontend tests for workflow logic, API error handling, permission states,
  pagination/sorting/filtering, and critical empty/error states.
- Remove prototype-only routes, fixtures, local bypasses, console logging of
  sensitive payloads, and undocumented feature flags before release.
- Capture release evidence with backend validation, generated drift checks,
  frontend build/tests, and handoff JSON.

## Verification

For a frontend-only docs or starter change:

```bash
scripts/check-pds-components.mjs --json
scripts/appfw framework validate --json
scripts/appfw framework docs-check --json
```

For admin UI source changes:

```bash
cd admin_ui
npm run build
```

For a downstream product frontend, add the product's frontend build/test command
to its release evidence before `scripts/appfw product handoff --json`.

Minimum product frontend evidence should include:

```bash
scripts/appfw product validate --json
scripts/appfw product generate --check --json
scripts/appfw product test --fast
```

and the product frontend's equivalent of:

```bash
npm run appfw:check
npm run typecheck
npm run lint
npm run test
npm run build
```

For products created from an `appfw product new` profile,
`scripts/appfw product new --list-profiles --json` must advertise the reusable
frontend scaffold evidence when the profile includes one. The
`frontend_scaffold` block names the frontend root,
`.appfw-ui/scaffold-manifest.json`, ownership manifest, generated UI contract,
offline package check, and release evidence paths so a new product can carry
the same checks without copying CRM-specific instructions by hand.
For `product-intake` frontends, `npm run appfw:check` writes
`frontend/target/appfw/frontend-scaffold-check.json` and fails when required
scaffold files are missing, PDS component/style aliases are removed, or CRM
sample terms appear in product-facing frontend files. The scaffold manifest's
`design_system` block records the PDS Health source, package import, and shared
style import, plus the starter example families that must be present:
overlay, analytics, data-grid, and generated-form. The generated starter is a
CRM-neutral product shell, not a CRM copy; product teams replace the placeholder
workflow content after their model and generated UI contract exist. The
framework-side disposable proof retains the same class of evidence at
`target/appfw/product-intake-proof/frontend-residue-check.json` and the
product-local package check artifact at
`target/appfw/product-intake-proof/frontend-scaffold-check.json`.

If the product does not expose all four frontend commands yet, the handoff must
state which command is missing and what evidence was used instead. Product
frontend release notes should also name the generated contract source,
auth/tenant assumptions, skipped live checks, and any accessibility or
screenshot evidence retained for review.
