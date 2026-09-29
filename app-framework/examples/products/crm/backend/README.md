# Backend

The backend is a generated and human-extended Rust API server built with Axum
and async-graphql.

Run it from the repository root:

```bash
ENV_NAME=local API_PORT=8080 scripts/appfw serve
```

The wrapper supplies local defaults for `VERSION` and `RUST_MIN_STACK`; set
those variables explicitly when reproducing a packaged runtime environment.

## Routes

GraphQL endpoints are generated per schema:

```text
http://localhost:8080/crm
http://localhost:8080/system
```

The backend-hosted product SPA is served at:

```text
http://localhost:8080/
```

Its React/Vite source lives in `frontend`. The production build writes to
`backend/product_dist`, which is build output and should not be edited by hand.
Rebuild it with:

```bash
cd ../frontend
npm run build
```

The backend-hosted admin console is served at:

```text
http://localhost:8080/admin
```

Its React/Vite source lives in `admin_ui`. The backend serves the compiled
bundle from `backend/admin_dist`, which is build output and should not be edited
by hand. Rebuild it with:

```bash
cd admin_ui
npm run build
```

For frontend iteration, run the backend on port `8080` and use the Vite dev
servers. Product frontend development uses `http://127.0.0.1:5173/`; admin UI
development uses `http://127.0.0.1:5173/admin/` from the admin UI workspace.
Vite proxies `/admin/model`, `/crm`, and `/system` back to the backend.

Operational endpoints are also merged into the Axum router:

```text
GET /info
GET /health
GET /health/ready
GET /readyz
GET /health/live
GET /livez
GET /metrics
GET /metrics.json
```

`/health/live` and `/livez` are process liveness probes. They return `200 OK`
when the HTTP process is serving requests.

`/health`, `/health/ready`, and `/readyz` are readiness probes. They return
`200 OK` when startup prerequisites are ready and `503 Service Unavailable`
when a readiness check fails. The current readiness checks cover configuration
load, auth configuration, and data-access initialization. Database connectivity
is validated during provider/client initialization where that provider supports
it; a future cached provider ping can make readiness reflect post-startup data
source outages without adding per-request database pressure.

## Runtime Configuration

Common local variables:

```text
VERSION=1.0.0
API_PORT=8080
ENV_NAME=local
RUST_LOG=backend=info,tower_http=info
LOG_LEVEL=info
APP_BYPASS_POLICIES_IN_LOCAL=false
PG_SERVICE_ACCOUNT_NAME=...
PG_SERVICE_ACCOUNT_PASS=...
MONGO_SERVICE_ACCOUNT_NAME=...
MONGO_SERVICE_ACCOUNT_PASS=...
OKTA_AUDIENCE=...
OKTA_ISSUER=...
OKTA_CLIENT_ID=...
```

Keep real credentials and tenant values out of git.

Use `ENV_NAME=local` when the backend runs on the host and connects to
Docker/Podman-published database ports. Use `ENV_NAME=compose` when the backend
runs as the `backend` service in `podman-compose.yml`; generated data source
config then uses service DNS names like `postgres`, `mongo`, and `mssql`.

Data source environments include `security_profile` and `tls_mode`. Local
development environments can opt into explicit local transport exceptions;
managed environments must use TLS modes such as `require`, `verify_ca`, or
`verify_full`.

`APP_BYPASS_POLICIES_IN_LOCAL=true` is a local-only admin UI convenience for
exploring seeded data without Rego policy or tenant filters. It is ignored
unless `ENV_NAME=local`.

## Source Layout

```text
backend/src/main.rs
backend/src/routes/
backend/src/handlers/
backend/src/schemas/
backend/src/data/
backend/src/config/
```

Generated handler implementation files under `backend/src/handlers/<schema>/`
are human-owned extension points once created.

## Verification

```bash
scripts/appfw validate
scripts/appfw test
```

Run generated API scenarios only when the backend and required data sources are
prepared:

```bash
scripts/appfw migrate
```

Run the backend in a separate terminal:

```bash
ENV_NAME=local API_PORT=8080 scripts/appfw serve
```

Then run API scenarios:

```bash
scripts/appfw api-test
```
