# Tax Document Routing

A tax document routing application: tax staff look up an owner-doctor client, collect the documents
being submitted, and the system files them into the correct permanent storage folders (with
password protection where required), notifies the client, and surfaces failures in an exception
queue. A schema-driven Rust/Axum backend and a React/TypeScript single-page frontend, built on the
PDS App Framework.

## Architecture

The backend is generated and run on the **PDS App Framework** (`appfw_runtime` plus
`appfw-provider-postgres`), consumed as a Cargo path dependency on `../app-framework`, vendored in
the `Tax-Application` monorepo alongside this folder. The application model lives in
`.appfw/model/**`; the framework's `app_gen` generator turns it into the GraphQL schema, routes,
handler scaffolds, database DDL and the frontend contract. Durable business logic is hand-owned and
lives outside the generated surface.

| Path | Contents | Ownership |
|---|---|---|
| `.appfw/model/` | Application model: the `tax_routing` schema (routing records, documents, reference tables, exception tasks), enums, relationships, RBAC. Source of truth. | Product |
| `.appfw/manifest.yaml` | Topology: schemas, data sources, ingress, UI packaging. | Product |
| `.appfw/specs/` | Feature specifications and the source business spec (start at `000-INDEX.md`). | Product |
| `backend/src/services/` | Routing engine, document store boundary, PDF protection, notifications. | Product |
| `backend/src/handlers/tax_routing/<entity>.rs` | Custom method implementations; delegate to services. | Product |
| `backend/src/routes/`, `backend/src/schemas/`, `backend/src/handlers/**/generated.rs`, `backend/src/operations/` | Generated from the model. | Generated: do not hand-edit |
| `backend/src/platform/` | Thin product-side wiring over `appfw_runtime`. | Product |
| `frontend/src/features/**` | Product screens. | Product |
| `frontend/src/generated/` | Generated UI contract. | Generated: do not hand-edit |

## Topology

- Single PostgreSQL data source (`pg_primary`) hosting the `tax_routing` product schema and the
  framework `system` schema, in the `tax_routing` database.
- Frontend: built into the backend image and served at `/` when `APP_PRODUCT_UI_ENABLED=true`.
- MCP server: off. Kafka: off. Single-tenant.
- Document storage is Box in the source spec and moves to SharePoint later; all access goes through
  a `DocumentStore` boundary in `backend/src/services/`.

## Framework dependency

```
Tax-Application/
├── app-framework/         # vendored PDS App Framework
└── tax-doc-routing/       # this application
```

`Cargo.toml` consumes it as a path dependency (`../app-framework`); no separate clone is needed.

## Build and test

```bash
# backend (app-framework/ is vendored one level up: nothing to fetch)
cargo check --workspace --all-targets
cargo test --workspace

# model validation and codegen drift: the framework CLI is a bash wrapper;
# on Windows run it in a Linux container (see the root CLAUDE.md)
scripts/appfw product validate --json
scripts/appfw product generate --check --json
scripts/appfw product boundary-check --json

# frontend
cd frontend && npm install
npm run test:frontend
```

## Run locally

```bash
# 1. database (host port 5434, database tax_routing)
APP_POSTGRES_HOST_PORT=5434 docker compose -f podman-compose.yml up -d postgres

# 2. backend on http://127.0.0.1:8080  (GraphQL at /tax-routing and /system)
cd backend && cp .env.example .env && cargo run -p backend --bin backend

# 3. frontend dev server on http://localhost:5173
cd ../frontend && npm run dev
```

## Documentation

- `docs/architecture/tax-document-routing-design.md`: business flow, backend architecture, database
  and frontend design, open decisions.
- `.appfw/specs/`: feature specs and the source business spec.
- `docs/runbook/new-schema-checklist.md`: generator and build gotchas when adding entities.
- Component READMEs under `backend/`, `frontend/`, `database/`.
- `../docs/LOCAL_DEV_SETUP.md` and `../docs/TEAM_GIT_WORKFLOW.md`: machine setup and the PR process.
