# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this repo is

A monorepo with two top-level folders that always move together in one commit. It deliberately mirrors the layout of the Project Governance monorepo:

| Folder | What | Edit? |
|---|---|---|
| `tax-doc-routing/` | The Tax Document Routing application: a schema-driven Rust/Axum backend plus a React/TypeScript SPA. Product work happens here. | Yes |
| `app-framework/` | Vendored copy of the PDS App Framework. `tax-doc-routing` consumes it as a Cargo path dependency (`../../app-framework/...`). | Rarely: coordinated framework-bug or upstream-bump work only. It has its own `CLAUDE.md` / `AGENTS.md`; follow those when working inside it. |

Do not move or rename either folder: the Cargo path dependency and `scripts/appfw`'s framework discovery both depend on this exact layout. Never nest the product one level deeper (`tax-doc-routing/tax-doc-routing`) and never add an `app-framework` link inside the product folder.

Application domain: the tax department of a physician-management organisation routes tax documents (K-1s, returns, statements) for owner doctors into the correct permanent document-storage folders. A routing record moves through four stages (collect client info, PDF processing, clean temporary folder, resolution) with an exception queue for failures. Storage is Box in the source spec and moves to SharePoint later, so all storage access sits behind a `DocumentStore` boundary.

## Source of truth

- Business requirements: `tax-doc-routing/.appfw/specs/source/TaxDocumentRouting_Platform_Agnostic_Spec_Final_1.docx`.
- Feature specs: `tax-doc-routing/.appfw/specs/` (start at `000-INDEX.md`).
- Architecture and design: `tax-doc-routing/docs/architecture/` (see `tax-document-routing-design.md`).

Decisions already made:
- Two roles as RBAC: `tax_staff` (sees and acts on records assigned to them) and `tax_admin` (sees all, reassigns, cancels/withdraws, maintains reference data). Row scoping is enforced in Rego, never in the UI.
- Storage platform: Box now in the spec, SharePoint later. Do not reference Box or SharePoint outside the `DocumentStore` implementations.

## Generated vs. hand-owned: the core architectural split

The backend is **generated** from an application model. `tax-doc-routing/.appfw/model/**` (YAML: entities, enums, relationships, RBAC policies, seeds) is the source of truth; the framework's `app_gen` generator turns it into the GraphQL schema, HTTP routes, CRUD handler scaffolds, DB DDL, runtime config and the frontend contract.

**Never hand-edit** anything with `generated` in the path, plus:
- `tax-doc-routing/backend/src/{routes,schemas}/**`, `backend/src/handlers/**/generated.rs`, `handlers/**/mod.rs`, `handlers/selections.rs`, `backend/src/operations/generated.rs`
- `backend/config/generated/**` (except: you author the `.rego` policy *bodies* under `.appfw/model/.../rbac/`, not here)
- `database/_pkg/**`, `frontend/src/generated/**`, `podman-compose.yml`, `appfw.lock`
- `tax-doc-routing/.appfw/model/**` is the generator's input; changing it is a coordinated task
- anything under `app-framework/`

**Hand-owned product code:**
- `backend/src/services/**`: routing engine, document store, PDF protection, notifications. All business logic lives here.
- `backend/src/handlers/tax_routing/<entity>.rs`: created once per entity, then hand-owned; custom-method bodies delegate to `services/`.
- `.appfw/model/schemas/tax_routing/rbac/*.rego`: policy bodies.
- `backend/src/platform/**`, `backend/src/config/**`, `backend/src/data/**`, `backend/src/main.rs`: thin wiring over `appfw_runtime`.
- `frontend/src/**` except `frontend/src/generated/`: screens (`features/**`) and the GraphQL client (`lib/appfwClient.ts`). Components come directly from the vendored `@appfw/pds-health-components` package (`frontend/vendor/`); there is no product-local design-system layer.

**Before adding a frontend component or hand-written style**, check `app-framework/appfw_ui/pds_health/reference/catalog.json` for an existing PDS match. Any frontend change: use the `pds-frontend-guard` skill (`.claude/skills/pds-frontend-guard/`).

## Common commands

Run backend commands from `tax-doc-routing/`. `cargo run` must be from `tax-doc-routing/backend/` because `config/loader.rs` resolves `config/generated/` relative to the working directory.

```bash
cd tax-doc-routing
cargo check --workspace --all-targets
cargo test --workspace

cd frontend
npm install
npm run typecheck && npm run test && npm run build
npm run dev
```

`scripts/appfw` is a Bash wrapper and does not run natively on Windows. Run it in the `rust-appfw:latest` container from the **repo root**:

```bash
docker run --rm -v "$PWD":/work -w /work/tax-doc-routing \
  rust-appfw:latest ./scripts/appfw product validate --json
```

## Local runtime

Its own Postgres container (`tax-doc-routing-postgres`, host port 5434, database `tax_routing`) is defined in `tax-doc-routing/podman-compose.yml`. The backend listens on 8080 by default and the frontend dev server on 5173; when running alongside another product set `API_PORT` and Vite's port explicitly.

Auth in local mode (`ENV_NAME=local`): no `Authorization` header means admin. To act as a role: `Authorization: Bearer appfw-local:user=<name>;tenant=<id>;roles=tax_staff` (or `tax_admin`).

The backend must build its Tokio runtime with a large worker stack (`BACKEND_WORKER_STACK_MIB=128`, as Project Governance does); the default stack overflows on deep GraphQL reads.

## Git workflow

`main` is the only integration branch; commit to it only via pull request. See `docs/TEAM_GIT_WORKFLOW.md`. Generated-file merge conflicts: take `main`'s version, do not hand-merge.
