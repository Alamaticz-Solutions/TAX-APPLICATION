# Tax Document Routing: Local Setup & Run Guide

## 1. Prerequisites

| Tool | Notes |
|---|---|
| Git | Clone outside OneDrive/Dropbox (the Rust `target/` directory is ~12 GB of churn). |
| Rust (stable, MSVC on Windows) | `rustup default stable`. First build compiles the whole framework: 8-20 min cold. |
| Node 20+ | Frontend (`engines` in `frontend/package.json`). |
| Docker Desktop | Postgres container, and the `rust-appfw` image for the framework CLI on Windows. |

`rust-appfw:latest` is `rust:1` plus `rustfmt` and `rsync`. It is needed only on Windows, because
`tax-doc-routing/scripts/appfw` is a Bash wrapper.

## 2. Layout

```
Tax-Application/
├── app-framework/      vendored framework (do not edit casually)
└── tax-doc-routing/    the application
```

Never nest the product one level deeper and never link `app-framework` inside it: the Cargo path
dependencies (`../../app-framework/...`) and `scripts/appfw` discovery depend on this exact layout.

## 3. Database

The application has its own Postgres container so it can run beside other products:

```bash
cd tax-doc-routing
APP_POSTGRES_HOST_PORT=5434 docker compose -f podman-compose.yml up -d postgres
```

This starts `tax-doc-routing-postgres` (host port **5434**, database `tax_routing`, user `postgres`).

Load the schema. The generated table SQL does not create the schema itself:

```bash
docker exec tax-doc-routing-postgres psql -U postgres -d tax_routing -c "CREATE SCHEMA IF NOT EXISTS tax_routing"
docker exec -i tax-doc-routing-postgres psql -U postgres -d tax_routing -v ON_ERROR_STOP=1 \
  < database/_pkg/schemas/tax_routing/tables.pg.sql
```

(`scripts/appfw product migrate` does the same and also applies versioned migrations; on Windows run
it through the container, see section 6.)

## 4. Backend

```bash
cd tax-doc-routing/backend
cp .env.example .env          # git-ignored; adjust API_PORT if 8080 is taken
cargo run -p backend --bin backend
```

Run it from `backend/`: `config/loader.rs` resolves `config/generated/` relative to the working
directory. Key `.env` values: `ENV_NAME=local`, `API_PORT`, `PG_PORT=5434`,
`PG_SERVICE_ACCOUNT_NAME` / `_PASS` (must match a real Postgres role),
`APP_PRODUCT_UI_ENABLED=true` (required or `/` returns 404), and `BACKEND_WORKER_STACK_MIB=128`
(leave it: without it the read path overflows the default stack).

Endpoints: `/tax-routing` (product GraphQL), `/system` (framework metadata), `/admin/*`,
`/health/live`, `/health/ready`, `/metrics`.

Auth in local mode: no `Authorization` header means admin. To act as a role:

```
Authorization: Bearer appfw-local:user=<name>;tenant=<id>;roles=tax_staff
```

(`tax_staff` or `tax_admin`.)

## 5. Frontend

```bash
cd tax-doc-routing/frontend
npm install
npm run dev                   # http://localhost:5173
```

The dev server proxies `/tax-routing`, `/system` and `/admin` to `VITE_BACKEND_URL` (default
`http://127.0.0.1:8080`). The screens currently run on mock data, so the UI works without the
backend. `npm run test:frontend` is the full gate (scaffold check, PDS ratchet, typecheck, lint,
tests, build).

## 6. The framework CLI on Windows

Run from the **repo root** so both folders are visible:

```bash
MSYS_NO_PATHCONV=1 docker run --rm -v "$PWD":/work -v tax-cargo-registry:/usr/local/cargo/registry \
  -w /work/tax-doc-routing \
  -e APPFW_CARGO_TARGET_DIR=/work/tax-doc-routing/target/appfw-cargo-linux \
  rust-appfw:latest bash scripts/appfw product validate --json
```

Other subcommands: `product generate`, `product generate --check --json` (drift),
`product boundary-check --json`, `product policy-test --json`. The separate
`APPFW_CARGO_TARGET_DIR` keeps Linux build output apart from your Windows `target/`.

## 7. Troubleshooting

| Symptom | Cause / fix |
|---|---|
| `thread 'tokio-rt-worker' has overflowed its stack` | `BACKEND_WORKER_STACK_MIB` missing from the environment; use `backend/.env`. |
| `schema "tax_routing" does not exist` | Create the schema first (section 3). |
| `failed to bind listener ... 10048` | Another process holds the API port; set `API_PORT`. |
| Backend cannot find `config/generated/...` | Run it from `backend/`. |
| `registry index was not found: pds-app-framework-crates` | Run cargo from inside `tax-doc-routing/` so `.cargo/config.toml` (the registry definition) is picked up. |
| `scripts/appfw` "not a valid Win32 application" | Bash wrapper: use the container (section 6). |
| Blank page on first `npm run dev` load | Vite is pre-bundling dependencies; reload after a few seconds. |
