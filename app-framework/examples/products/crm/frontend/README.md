# CRM Reference Frontend

The product-owned CRM reference frontend, and the **first reference instance of
the enterprise frontend scaffold** (see `docs/adr/0005`–`0012`). It is not the
backend-hosted admin UI: the admin UI is a runtime-generic operations console;
this is an opinionated PDS Health product experience built on the generated App
Framework APIs.

This is a **buildable** Vite + React + TypeScript app with a model-generated UI
contract. It compiles, renders an auth/tenant-aware app shell, and serves
generic model-driven list/detail/edit scaffolds for CRM entities from
`app_gen` output. Opinionated workflow screens are layered on top of that
contract.

## Quick start

```bash
cd examples/products/crm/frontend
npm install
# run the CRM backend first (serves /crm GraphQL on :8080), then:
npm run dev          # http://localhost:5173, proxies /crm + /system to the backend
npm run appfw:check  # offline scaffold package manifest/ownership check
npm test             # lightweight route identity/client regression tests
npm run test:browser # Playwright E2E and axe accessibility tests
npm run test:frontend
npm run build        # tsc --noEmit && vite build
npm run typecheck    # tsc --noEmit
npm run release:evidence
```

Set `VITE_BACKEND_URL` (see `.env.example`) to target a backend other than the
dev proxy default (`http://127.0.0.1:8080`).

`npm run build` writes the production SPA bundle to
`../backend/product_dist`. The backend serves that bundle at `/` in the default
one-image deployment topology when `APP_PRODUCT_UI_ENABLED=true`. Vite's
`frontend/dist` directory is only a local dev/default Vite convention; the
product deployable bundle belongs beside the backend binary.

## Ownership boundary (ADR 0007)

The scaffold's safety guarantee is an explicit generated/scaffold/human-owned
split, recorded in `.appfw-ui/ownership.json`:

| Area | Path | Rule |
| --- | --- | --- |
| Generated | `src/generated/**`, `.appfw-ui/scaffold-manifest.json` | Overwrite-safe; emitted from the backend model (ADR 0006). Do not hand-edit. |
| Scaffold | `src/app`, `src/components`, `src/scaffold`, `src/lib`, `src/styles`, `src/design`, `scripts/check-scaffold.mjs` | Reusable framework scaffold; updated by scaffold upgrades. |
| Human-owned | `src/features/**` | Product customization; created once, preserved across regeneration. |

Agents and developers customize under `src/features/`; regeneration never
clobbers it.

`scripts/appfw generate` also emits `.appfw-ui/scaffold-manifest.json`, a
deterministic package manifest that names the generated contract, ownership
manifest, package roots, required frontend scripts, and release evidence. Run
`npm run appfw:check` before handoff to verify the scaffold package shape without
requiring a live backend.

## Reference workflows

Opinionated screens (under `src/features/`) that demonstrate real enterprise UX,
not generic CRUD:

| Workflow | Proof point |
| --- | --- |
| Accounts | Relationship navigation, policy-aware views, validation, tenant-aware filters. |
| Pipeline | Sortable/filterable opportunity lists, aggregate summaries, pagination, state handling. |
| Activities | Task-like workflow, date/time handling, ownership, optimistic refresh. |
| Audit | Read-only timeline, request/correlation IDs, policy context, redacted diagnostics. |

These currently ship as thin screens over the generic list; they are built out
as the generated contract and CRUD kit mature.

## Implementation rules

- Consume `src/generated/appfw-ui-contract.ts` and the typed `appfwClient.ts`
  boundary (request/correlation IDs, response timing, validation, policy-denied
  outcomes are preserved there). `scripts/appfw generate` emits the contract
  from the model (ADR 0006); do not hand-edit it.
- Treat `EntityListView` as the reusable scaffold proof: generated entity
  metadata selects list/detail/edit fields, GraphQL operations, pagination
  shape, validation hints, optimistic concurrency fields, and policy-denied
  states.
- Use PDS `--pds-*` tokens through the canonical
  `@appfw/pds-health/tokens/pdsTokens.css` import for brand/theme. The
  `src/design/pdsTokens.css` copy must not be recreated; U6 fork-check rejects
  product-local token copies.
- Keep auth and tenant context explicit (`authContext.ts`, `tenantContext.ts`);
  the dev identity bar uses session storage. Production auth/tenant come from the
  governed Okta/JWT flow (ADR 0010). The UI reflects backend authorization and
  fails closed; it never re-implements it.
- Synthetic data only in source, fixtures, and tests — no PII/PHI/financial
  values (ADR 0010).
- Do not fork or copy `admin_ui`. Reuse patterns and contracts, not the ops shell.

## Production serving

The CRM reference product follows the App Framework default deployment model:
one backend image serves the Rust API, the product SPA, and the optional admin
UI. Build order for a deployable image is:

```bash
cd examples/products/crm/frontend
npm ci
npm run build

cd ../../../../admin_ui
npm ci
npm run build

cd ../examples/products/crm/backend
cargo build --release --locked
```

The resulting backend package contains:

```text
backend/product_dist/  # product SPA
backend/admin_dist/    # admin UI, when built
```

At runtime the product SPA is mounted at `/` and reserves generated API,
admin, health, and metrics paths for the backend.

## Verification

```bash
# From examples/products/crm
scripts/appfw validate --json
scripts/appfw generate --check --json

# From examples/products/crm/frontend
npm run tokens:check
npm run appfw:check
npm test
npm run test:browser
npm run test:frontend
npm run typecheck
npm run build
npm run release:evidence
```

`npm run test:browser` starts the CRM frontend on an isolated Playwright port
and uses a deterministic mocked CRM GraphQL backend, so agents can prove the
dashboard, grid, form, query builder, lookup selector, delete confirmation, and
dark/light mode without running provider containers. `scripts/appfw
frontend-test --json` runs the full frontend evidence bundle from the repo root
and writes `target/appfw/frontend-test.json`.
