# App Framework

App Framework is an enterprise backend generation system for teams building
agentic applications. Application shape lives in `.appfw/model`; `app_gen`
validates that source, generates a Rust backend, emits database packages, and
creates API test scaffolding that can be checked into source control.

The repo is intentionally optimized for both people and coding agents. A single
top-level command surface validates config, regenerates artifacts, checks drift,
and runs the verification path that enterprise developers need before handing
work to CI.

Release adoption is governed by the framework-level
[`CHANGELOG.md`](CHANGELOG.md), [`SECURITY.md`](SECURITY.md),
[`SUPPORT.md`](SUPPORT.md), and
[`docs/release/versioning-and-compatibility.md`](docs/release/versioning-and-compatibility.md).
Live environment tasks that require CI/CD, IaC, managed providers, monitoring,
or release authority are tracked in
[`docs/release/live-environment-work-items.md`](docs/release/live-environment-work-items.md).

## Framework Philosophy

- Config is source code. Application shape starts in `.appfw/model`, and
  validation is the first contract.
- Generated code should be predictable and replaceable. Change repeated
  behavior in `app_gen/src` or `app_gen/_templates`, not by patching generated
  output.
- Human-owned extension points are protected. Durable business behavior belongs
  in handler implementations, services, policies, or other explicit modules
  that survive regeneration.
- Provider semantics are a framework contract. PostgreSQL, MongoDB, MS SQL
  Server, and Snowflake should expose the same API behavior unless a limitation
  is documented.
- Agents and humans use the same workflow. Prefer the root CLI, generated
  contracts, artifact manifests, and reproducible checks over tribal knowledge.

## What This Repo Provides

- Schema-driven Rust backend generation with Axum and async-graphql.
- Multi-provider database package generation for PostgreSQL, MongoDB, MS SQL
  Server, Microsoft Fabric SQL analytics read models, and Snowflake-oriented
  work.
- Generated GraphQL schema, handlers, routes, and integration test modules.
- Config validation before generation, including a generated contract for
  agents and humans.
- Artifact ownership metadata that distinguishes overwrite-safe generated files
  from human-owned extension points.
- QueryIR cost budgets, pagination policy, generated performance
  recommendations, and a lightweight load-test harness for generated APIs.
- Golden downstream app profiles and framework provenance locking for
  app-specific repositories.
- A first-class CRM sample product under `examples/products/crm` that consumes
  the framework through the same split-root contract downstream teams use.
- Agent handoff JSON that summarizes changed surfaces, verification artifacts,
  and generated drift.
- A downstream product workspace contract for ownership, security evidence,
  upgrade review, and the path toward versioned framework packages.
- Optional model-driven MCP endpoint so agents can inspect schemas and query
  records through the same governed runtime path as GraphQL.
- A root workflow wrapper at `scripts/appfw`.

Product frontends are first-class product workspaces, not forks of the admin
console. The CRM sample includes a reference frontend under
`examples/products/crm/frontend`; it demonstrates generated UI contracts,
typed API usage, PDS Health tokens, enterprise grids/forms/dashboards, and
CLI-runnable frontend evidence.

The default production topology is one backend image. Product Vite frontends
build into the product backend package at `backend/product_dist`; when
`APP_PRODUCT_UI_ENABLED=true` and `backend/product_dist/index.html` exists, the
same Rust backend serves the product SPA at `/` while keeping generated API,
admin, health, metrics, readiness, and MCP routes backend-owned.

Framework releases produce ProGet-ready toolchain artifacts with
`scripts/appfw framework package --json`: a packaged `appfw` CLI, prebuilt
generator/introspection/database runner binaries, a binaries-only bundle, a
Cargo crate publish plan for the product-facing framework modules, templates,
golden downstream profiles, and a product-only docs pack. Product developers
can consume those artifacts instead of downloading or building the full
framework checkout locally.

The repo also includes a backend-hosted admin console at `/admin`; it is a
React/Vite, runtime model-driven SPA that fetches entity metadata instead of
generating entity-specific product screens. Build it with
`cd admin_ui && npm install && npm run build`; the backend serves the compiled
bundle from `backend/admin_dist`. Set `APP_ADMIN_UI_ENABLED=false` to disable
that route. See `admin_ui/README.md` for the Vite dev-server and production
build flow.

For agentic tools, the backend can also expose `POST /mcp` when
`APP_MCP_ENABLED=true`. The MCP surface is model-driven and read-only today:
it lists schemas, describes entities, and exposes generated public
handler-level operations as MCP tools through the same
handler/DataAccess/QueryIR/policy/provider runtime path as GraphQL. MCP has its
own role/scope gate, bounded batches/resources/results, and entity audit events
for audited entities. Current release posture excludes MCP unless it is
explicitly certified, so release pipelines should keep `APP_MCP_ENABLED=false`.
See `docs/runtime/mcp.md`.

## Quick Start

Install Rust stable, then run the repository doctor:

```bash
scripts/appfw doctor
scripts/appfw context --json
```

Validate `.appfw/model` without generating backend, database, or API test
artifacts. Validation does emit validation and config-contract reports:

```bash
scripts/appfw product validate
```

Generate backend, database, and API test artifacts:

```bash
scripts/appfw product generate
```

Create a downstream-style product app from the CRM sample contract:

```bash
scripts/appfw product new ../customer-crm --profile crm-sample
```

Run local compile and unit checks:

```bash
scripts/appfw product test
```

Run the generated backend:

```bash
ENV_NAME=local API_PORT=8080 scripts/appfw product serve
```

For local development, the wrapper provides safe defaults for `VERSION` and
`RUST_MIN_STACK`; release/deployment launchers should set their own explicit
runtime values.

Generated GraphQL routes are schema-specific, for example:

```text
http://localhost:8080/crm
http://localhost:8080/system
```

## Agentic Workflow

Agents should use the same command path as developers:

```bash
scripts/appfw context --json
scripts/appfw product validate --json
scripts/appfw product test
scripts/appfw product handoff --json
scripts/appfw product upgrade --json
```

Important generated reports:

```text
.appfw/target/appfw/validation.json
.appfw/target/appfw/app_topology.json
.appfw/target/appfw/config_contract.json
.appfw/target/appfw/config_contract.md
.appfw/target/appfw/dev_infra.json
.appfw/target/appfw/artifacts.json
.appfw/target/appfw/performance_recommendations.json
target/appfw/agent-handoff.json
target/appfw/product-upgrade.json
.appfw/model/_specs/CONFIG_CONTRACT.md
```

`app_gen` resolves app, framework, config, template, and report roots
explicitly through `scripts/appfw`. Downstream products use split roots by
default: the app owns config and generated output, while framework crates,
templates, CLI behavior, and shared assets are resolved from the framework
checkout or package source.

Before changing generated-looking files, agents should check whether the change
belongs in `.appfw/model`, `app_gen/src`, `app_gen/_templates`, or a
human-owned extension point. See `AGENTS.md`, `CLAUDE.md`,
`docs/start/agent-task-map.md`, `docs/start/generated-ownership.md`, and
`docs/architecture/concerns/maintainability.md` for repo-specific operating instructions. For a
small task-specific procedure, start with the repo-native skill pack in
`agent_skills/README.md`.

## Repository Map

```text
app-framework/
|-- appfw_cli/    # appfw-cli package, installable appfw command surface
|-- app_gen/      # appfw-codegen package, config source, templates, validation
|-- appfw_test/   # appfw-test package, reusable API test harness
|-- agent_skills/ # Concise agent procedures that route to canonical docs
|-- backend/      # Generated and human-extended Rust API server
|-- database/     # Generated database package runner
|-- api_tests/    # Generated integration test harness and scenarios
|-- rego_test/    # Policy and Rego verification utilities
|-- examples/     # First-class sample product apps that consume the framework
|-- docs/         # Architecture, workflow, and framework guides
`-- scripts/      # Root automation entrypoints
```

## Config Source

Application definition starts in:

```text
.appfw/model/
```

The canonical config contract is generated from Rust-owned definitions in:

```text
app_gen/src/config_contract.rs
```

The checked-in human-readable contract is:

```text
.appfw/model/_specs/CONFIG_CONTRACT.md
```

Treat `.appfw/model` as source code. Validate it before generation, review
generated diffs, and keep durable business behavior in explicit config,
templates, or human-owned extension points rather than patching generated files
directly.

## Generated Ownership

`app_gen` emits an artifact manifest at:

```text
.appfw/target/appfw/artifacts.json
```

Use it to understand whether a file is:

- `generated`: overwrite-safe and controlled by config/templates.
- `human_owned`: created when missing, then preserved for custom logic.

This boundary is central to maintainability. Generator and template changes
should be checked with:

```bash
scripts/appfw product validate --json
scripts/appfw product generate
scripts/appfw product generate --check --json
scripts/appfw product test
```

Use this as a diagnostic for generator/template work, not as a first-run health
check.

## Runtime Configuration

Runtime crates load `.env` files through `dotenv`. Common local variables are:

```text
ENV_NAME=local
API_HOST=127.0.0.1
API_PORT=8080
RUST_LOG=info
LOG_LEVEL=info
PG_SERVICE_ACCOUNT_NAME=...
PG_SERVICE_ACCOUNT_PASS=...
MONGO_SERVICE_ACCOUNT_NAME=...
MONGO_SERVICE_ACCOUNT_PASS=...
```

Data source connection metadata is generated from:

```text
.appfw/model/data_sources/_res.yaml
backend/config/data_sources.yaml
database/_pkg/data_sources.yaml
```

`ENV_NAME=local` is for processes running on the host and connecting to
Docker/Podman-published ports such as `localhost:5432`. `ENV_NAME=compose` is
for processes running inside generated `podman-compose.yml`, where service DNS
names come from data-source `compose` environments. Each environment declares
`security_profile` and `tls_mode`; local development may use explicit local
transport exceptions, while managed environments must use fail-closed TLS modes.

Do not commit real credentials or tenant-specific secrets.

For a local Grafana/Loki/Alloy/Prometheus stack, use the opt-in
`observability` compose profile. See `observability/README.md` before using it
with sensitive data; logs must be treated as regulated data and should not
contain PHI.

## Documentation

Start here:

- `docs/README.md` for the documentation index.
- `docs/lifecycle/product-golden-path.md` for the end-to-end downstream
  product developer flow.
- `docs/start/agent-task-map.md` for intent-to-edit routing.
- `docs/start/cli-quickstart.md` for the first CLI path and
  `docs/reference/cli.md` for the full `scripts/appfw` command contract.
- `docs/reference/product-workspace-contract.md` for downstream ownership, evidence,
  upgrade, and packaging rules.
- `docs/architecture/framework-packaging.md` for current and target crate/package
  boundaries.
- `docs/reference/codegen-api.md` for the supported `appfw-codegen` Rust API.
- `docs/reference/app-manifest.md` for `.appfw/manifest.yaml` app topology rules.
- `docs/start/generated-ownership.md` for agent edit-surface decisions.
- `docs/lifecycle/application-lifecycle.md` for downstream app creation and framework
  upgrade strategy.
- `docs/runtime/performance-and-scalability.md` for QueryIR budgets, pagination,
  index recommendations, load testing, and safe provider diagnostics.
- `docs/release/deployment-reference.md` for containers, env contract, secrets, CI,
  and observability.
- `docs/release/pds-security-baseline-traceability.md` for PDS production
  security baseline evidence mapping.
- `docs/runtime/provider-sdk.md` for adding a fifth provider safely.
- `docs/frontend/product-frontend.md` for product frontend scaffolding.
- `docs/architecture/concerns/maintainability.md` for docs IA, command contracts, architecture
  intent, and maintainability gates.
- `app_gen/README.md` for generator details.
- `backend/README.md` for backend runtime configuration.
- `database/README.md` for database package execution.
- `api_tests/README.md` for generated API test scenarios.

## Current Maturity

This framework is evolving toward an enterprise-grade, agentic-first product
platform. The highest-value ongoing work is live release evidence, provider
parity, security and operations certification, generated ownership guarantees,
frontend scaffold execution, and the CLI/documentation surface that lets agents
make safe changes quickly.
