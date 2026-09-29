# CRM Product Example

This is a first-class downstream product app used to prove the split-root App
Framework contract. It keeps product-owned configuration, generated artifacts,
and extension code in this product root while consuming framework-owned CLI,
generator, templates, runtime crates, and test harnesses from the parent
`app-framework` checkout.

## Shape

```text
examples/products/crm/
|-- .appfw/manifest.yaml
|-- Cargo.toml
|-- .appfw/model/
|-- backend/
|-- database/
|-- api_tests/
|-- frontend/
|-- rego_test/
|-- podman-compose.yml
`-- scripts/appfw
```

`scripts/appfw` resolves the framework root from `APPFW_FRAMEWORK_ROOT`,
`.appfw/local.env`, an adjacent `../app-framework`, or this repository's parent
framework checkout. `appfw new` writes `.appfw/local.env` for the developer who
bootstrapped a product app, and `.gitignore` keeps that local path out of source
control.

The `database/` directory intentionally contains product database package output
(`_pkg`) but no database Rust crate. `scripts/appfw migrate` executes the
framework-owned database runner against this product package.

`backend/` is the canonical sample product backend. It should keep
product-owned handlers, services, generated schemas/routes/adapters, and thin
app wiring. Framework-owned host helpers such as CORS, GraphiQL, security/rate
limiting, observability, route-shell assembly, admin/MCP mount gating, and
provider pool stats are consumed from `appfw-runtime` through small
compatibility facades while the remaining runtime/provider extraction work
continues upstream.

`frontend/` is the product-owned CRM reference frontend and the first buildable
instance of the enterprise frontend scaffold. It proves typed generated API
usage, PDS Health styling, auth/tenant-aware UX, relationship navigation,
policy-aware screens, scaffold ownership metadata, and release evidence. It must
not fork the framework-owned `admin_ui` operations console.

## Verify

From this directory:

```bash
scripts/appfw validate --json
scripts/appfw topology --json
scripts/appfw generate --check --json
scripts/appfw test
```

For frontend evidence:

```bash
cd frontend
npm run appfw:check
npm test
npm run typecheck
npm run build
npm run release:evidence
```

For local Kafka ingress validation, the CRM manifest enables `crm-events`, so
generated compose includes a `kafka` profile:

```bash
docker compose -f podman-compose.yml --profile kafka up -d broker kafka-smoke
```

The broker advertises `broker:29092` inside compose and
`localhost:${APP_KAFKA_HOST_PORT:-9092}` from the host. `kafka-smoke` creates the
`crm.events` topic and lists broker topics.

The example intentionally does not check in `appfw.lock`: because it lives in
the same Git repository as the framework, the framework commit SHA changes with
the commit that updates the example. `appfw new` writes `appfw.lock` for real
downstream product apps.
