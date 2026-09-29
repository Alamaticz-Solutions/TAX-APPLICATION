# Framework Packaging

This page tracks the crate and package boundaries App Framework is moving
toward. The goal is a versioned upstream framework that downstream product apps
consume through `Cargo.toml`, while product repos keep product config,
generated artifacts, handlers, services, tests, topology, and deployment
overlays.

For the Step 0 contract baseline, use the
[Packaging Boundary Matrix](packaging-boundary-matrix.md).
For durable packaging guidance, use the
[Packaging Concern](concerns/packaging.md). Historical extraction ledgers live
in `docs/archive/`.

## Current Packages

| Package | Path | Current Role | Product Contract |
| --- | --- | --- | --- |
| `appfw-cli` | `appfw_cli` | Packaged `appfw` command-line interface, runnable with `cargo run --locked -p appfw-cli -- ...` and installable as `appfw`. | Product teams use this as the stable command surface instead of depending on repo-local shell paths. |
| `appfw-runtime` | `appfw_runtime` | Shared product-facing runtime contracts, including provider-neutral extension types, product identity contracts, generic runtime handler context, schema-neutral GraphQL context extraction helpers, schema-neutral model metadata descriptors, runtime error categories, product-facing runtime app error, query cost budget/scoring contracts, config/auth primitives, CORS, GraphiQL shell, security/rate limiting, observability helpers, HTTP route/layer assembly, runtime info/readiness route shell, runtime GraphQL schema route shell, tenant-isolation policy helper, admin/MCP mount gating, admin UI route path and asset mount shell, admin runtime state contract and standard handler shell, admin UI bundle path/index response helpers, admin auth/error response helpers, admin migration manifest DTO/path/loading helpers, admin model/schema response DTOs, admin model provider/service orchestration, admin endpoint HTTP auth/error/JSON response shell, admin schema summary input and assembly helpers, admin schema-health helper contracts and summary construction, admin migration dialect helper, admin provider capability response helpers, runtime admin DTO/redaction/status helpers, runtime admin policy-explain result helper and request/response orchestration, audit-timeline subject/result shell, audit-timeline request/response orchestration, and query-diagnose request/response orchestration, schema-neutral MCP protocol/operation/access contracts, bounded MCP response helpers, runtime MCP HTTP route/auth/origin/response shell, runtime MCP payload/batch/JSON-RPC loop and envelope handling, runtime MCP catalog/resource/prompt/tool schema rendering, MCP operation argument/serialization/redaction helpers, MCP generated-operation catalog/dispatcher traits, runtime MCP built-in methods, schema-neutral MCP support-tool execution, runtime MCP method dispatch and tool-list assembly, runtime MCP `tools/call` shell orchestration, MCP access-explain provider contract/response shaping, generated-tool audit payload shaping, generated-tool audit sink/store adapter contracts, and framework runtime surfaces as they are extracted. | Downstream apps depend on this package for stable runtime primitives and extension contracts. |
| `appfw-provider-mongo` | `appfw_provider_mongo` | Concrete MongoDB provider package boundary. It currently owns Mongo driver connection option shaping, BSON result formatting, extended JSON normalization, BSON scalar conversion helpers, Mongo audit document/filter shaping and persistence, mutation key/filter/update document shaping, Mongo projection/sort/filter/pipeline/lookup document helpers, Mongo record-value conversion helpers, Mongo `_id` to product `id` mapping, driver handle/collection resolution, health ping, find execution, query/aggregate facet execution, facet result decoding, and native error normalization. | Product backends depend on this package for MongoDB connection option/formatting/conversion/audit/mutation/query-document/filter/pipeline/lookup/record-value/execution behavior while keeping generated schema, metadata planning, and thin provider-client adapters local. |
| `appfw-provider-postgres` | `appfw_provider_postgres` | First concrete provider package boundary. It currently owns PostgreSQL connection pool config shaping, port validation, parameter placeholder, binding conversion, audit statement rendering and persistence, query/aggregate `ORDER BY` rendering, aggregate select/group/having rendering, mutation statement rendering, scalar/list/array/date-period filter criterion rendering over runtime data types, many-to-many CTE tuning config, pool handle/client checkout, health check, generic JSON-row query execution, aggregate row execution, row count/array decoding, and native error normalization. | Product backends keep generated schema adapters and depend on this package for PostgreSQL provider-owned behavior. |
| `appfw-provider-mssql` | `appfw_provider_mssql` | Concrete MS SQL Server provider package boundary. It currently owns MS SQL Server ODBC connection-string shaping (Microsoft ODBC Driver 18 for `sql_password`, FreeTDS for `ntlm`), encryption/trust-cert mapping, ODBC pool construction/handle/client checkout, pool-size support, port validation, T-SQL parameter placeholders, provider-owned `SqlParam` binding conversion, shared `odbc-api` execution helpers, audit statement rendering and persistence, query/aggregate `ORDER BY` rendering, aggregate select/group/having rendering, mutation statement rendering, scalar/list/array/date-period filter criterion rendering over runtime data types, generic query/execute/batch helpers, JSON-row query execution, aggregate row execution, row count/JSON decoding, health check, Fabric Entra ODBC client, and native error normalization. | Product backends keep generated schema adapters and depend on this package for MS SQL Server provider-owned behavior. |
| `appfw-provider-snowflake` | `appfw_provider_snowflake` | Concrete Snowflake provider package boundary. It currently owns SQL API endpoint/session config shaping, token requirement/defaults, statement bodies, parameter placeholder expressions, binding type/value conversion, Snowflake REST binding payloads, HTTP SQL API client/session execution, async polling, response row decoding, health check, audit persistence, audit query execution, provider error normalization, query/aggregate `ORDER BY` rendering, aggregate select/group/having rendering, mutation statement rendering, and scalar/list/array/date-period filter criterion rendering over runtime data types. | Product backends keep generated schema adapters and depend on this package for Snowflake provider-owned behavior. |
| `appfw-codegen` | `app_gen` | Stable codegen API plus compatibility binaries: `app_gen`, `appfw`, and `appfw_introspect`. | Product apps call the crate-root API or run the CLI/generator, but should not vendor generator internals as product code once packaging is complete. |
| `appfw-test` | `appfw_test` | Reusable API scenario, provider-certification, and policy-verifier harness. | Product apps consume the harness from generated/product API tests and policy tests instead of carrying harness internals as product code. |
| CRM sample backend | `examples/products/crm/backend` | First-class sample product backend and generated-output comparison target for framework-root workflows. | Downstream apps own their generated API crate and depend on framework runtime/provider packages; the repository root no longer carries a product backend crate. |
| `database` | `database` | Framework-owned database package runner and migration tooling, executed with the product `database/` directory as cwd. | Downstream apps own `database/_pkg` package output and migration content while consuming framework migration tooling. |
| `api_tests` | `api_tests` | Generated product scenarios in downstream products; provider certification contracts in the framework-root validation fixture. | Product scenarios stay downstream; reusable harness code lives in `appfw-test`, and provider certification stays upstream. |
| `rego_test` | `rego_test` | Policy verification fixture crate. | Product policy tests own fixtures and assertions while consuming the framework-owned verifier engine from `appfw-test` through `scripts/appfw policy-test`. |

## Compatibility Binaries

`appfw-cli` is the stable packaged CLI:

```bash
cargo run --locked -p appfw-cli -- validate --json
cargo install appfw-cli
```

`appfw-codegen` keeps the existing binary names so current scripts and
downstream checkouts continue to work:

```text
app_gen          validates config and writes generated artifacts
appfw           compiled compatibility CLI for settled workflow commands
appfw_introspect emits ownership, handoff, lock, and upgrade reports
```

The root `scripts/appfw` wrapper remains the human and agent command surface for
now. It runs these binaries with explicit app/framework/config/template/report
roots.

Framework release pipelines create a ProGet-ready distribution with:

```bash
scripts/appfw framework package --json
```

The distribution contains prebuilt `bin/appfw`, `bin/app_gen`,
`bin/appfw_introspect`, and `bin/database`, an ordered Cargo publish plan for
product-facing framework modules, plus the compatibility wrapper and its helper
scripts, templates, config seed facets/fragments, golden downstream profiles,
runtime/generator source needed for hash/provenance checks, and a product-only
docs pack. The packaged CLI and
wrapper auto-discover the sibling binaries under `bin/`, so downstream product
agents can validate, generate, migrate, and hand off without building framework
crates locally. Generated product `Cargo.toml` files should consume the
published framework crates from the approved Cargo registry.

CI publishes the Cargo crates in the order specified by the publish plan, then
publishes the generated tarballs and
`target/appfw/proget/app-framework-proget-manifest.json` to ProGet. See
[ProGet Distribution](../release/proget-distribution.md).

## Stable Codegen API

Downstream automation should depend on the crate-root API documented in
[Codegen API](../reference/codegen-api.md):

```rust
use appfw_codegen::{Codegen, CodegenRoots};

fn main() -> anyhow::Result<()> {
    let roots = CodegenRoots::from_env_or_current_dir()?;
    Codegen::new(roots).validate_only().run()?;
    Ok(())
}
```

The public API is intentionally smaller than the generator implementation.
Modules under `app_gen/src` remain framework internals unless re-exported from
the `appfw-codegen` crate root.

`appfw-codegen` must consume product roots explicitly: product config, reports,
and generated outputs live under the product app root, while generator source
and templates live under the framework root. The deterministic generation check
runs in a temporary split product/framework layout to catch copied-layout
assumptions before a framework release.

## Target Dependency Shape

The packaged downstream shape should eventually look like:

```toml
[dependencies]
appfw-runtime = "0.2"
appfw-provider-mssql = "0.2"

[build-dependencies]
appfw-codegen = "0.2"

[dev-dependencies]
appfw-test = "0.2"
```

Provider implementations should become framework-owned packages or tightly
feature-gated provider modules behind stable runtime contracts. Product
topology decides which data sources are active; Cargo dependencies and features
decide which provider implementations are available. The first concrete
package boundaries are `appfw-provider-mongo`, `appfw-provider-postgres`,
`appfw-provider-mssql`, and `appfw-provider-snowflake`, which own MongoDB
driver connection option shaping, result formatting, BSON scalar conversion,
audit document shaping and persistence, mutation key/filter/update document
shaping, projection, sort, filter criterion compilation, relationship lookup
and nested projection assembly, query/aggregate pipeline assembly, record-value
support, driver handle/collection resolution, health ping, find execution,
query/aggregate facet execution, facet result decoding, and native error
normalization; PostgreSQL connection pool config shaping, port validation,
parameter binding, audit statement rendering and persistence,
query/aggregate sort rendering, aggregate select/group/having rendering,
mutation statement rendering, filter criterion rendering, pool handle/client
checkout, health check, generic JSON-row query execution, aggregate row
execution, row count/array decoding, and native error normalization; MS SQL Server client config shaping,
encryption/trust-cert mapping, pool construction/handle/client checkout,
pool-size support, port validation, parameter binding, audit statement
rendering and persistence, query/aggregate sort rendering, aggregate
select/group/having rendering, mutation statement rendering, filter criterion
rendering, generic query/execute/batch helpers, JSON-row query execution,
aggregate row execution, row count/JSON decoding, health check, and native
error normalization; and Snowflake SQL API endpoint/session config shaping,
token requirement/defaults, statement/binding, HTTP SQL API execution/polling,
response row decoding, health check, audit persistence/query execution,
native error normalization, query/aggregate sort rendering, aggregate
select/group/having rendering, mutation statement rendering, and filter
criterion rendering support while
CRM sample and downstream product backends retain thin generated-schema adapters. The database runner already
follows this shape in split-root products: `scripts/appfw migrate` runs the
framework crate against the product `database/_pkg` surface.

## Dependency Contract Rules

Use these rules for packaging work and downstream product consumption:

- Product code may import stable framework package APIs, generated schema
  surfaces, and product-owned handlers/services.
- Product code must not import framework-internal modules that are not exported
  as package APIs.
- Product code should enter shared data behavior through framework runtime
  abstractions (for example `DataAccess`) instead of provider client internals.
- Product route/bootstrap adapters should register provider factories through
  `appfw-runtime::RuntimeProviderRegistry` rather than owning provider factory
  lookup semantics directly.
- Product code should use identity contracts exposed through
  `crate::product_api`/`appfw-runtime` (`UserAuth`, `Claims`) instead of auth
  handler implementation modules.
- GraphQL and MCP server runtime wiring are framework-owned runtime behavior.
  Downstream products should consume stable contracts for these services instead
  of carrying copied service wiring internals. Global HTTP layering and
  admin/MCP feature-gated route mounting belong in `appfw-runtime`.
  Schema-neutral MCP protocol constants, JSON-RPC envelopes, reflected
  operation metadata, method dispatch, tool-list assembly,
  `appfw_explain_access` request/response orchestration, and generated-tool
  audit payload shaping also belong in `appfw-runtime`; product backends keep
  only generated operation dispatch, product policy lookup, DataAccess audit
  persistence, and product-specific redaction while that boundary is being
  extracted.
- `examples/products/crm/backend` is the first-class sample product backend
  and the default product app root for repository-root `scripts/appfw`
  workflows. The repository-root `backend` crate has been retired so framework
  checkout commands no longer double as a product app.
- Admin UI implementation (`admin_ui` and admin-shell runtime wiring)
  remains framework-owned model-driven behavior; downstream products consume
  configuration/branding extension points instead of carrying UI source forks.
  Admin bundle path resolution and index response shaping are runtime-owned.
  Policy-explain, audit-timeline, and query-diagnose response/request shaping
  are runtime-owned; product backends provide only generated metadata, policy,
  and DataAccess adapters until the fuller admin service boundary lands.
- Provider implementations are framework-owned package surfaces and must not be
  treated as product customization files.
- Cargo dependencies/features select provider implementations available at
  compile time; manifest topology selects which configured data sources are
  active at runtime.
- Packaging PRs should classify changed files as product intent, generated
  output, or reusable framework behavior before review.

## Extraction Order

1. Keep `appfw-runtime` small and stable. Move provider-neutral runtime
   contracts into it until product/runtime dependencies are untangled. Use the
   runtime ownership inventory to choose the next contract and avoid template
   slimming before the runtime boundary exists.
2. Keep `appfw-codegen` as the generator package and expose reusable library
   entry points while preserving the existing binary names.
3. Continue extracting framework-owned command runners, using the database
   runner pattern: framework crate plus product cwd/package surface.
4. Split provider implementations behind feature-gated runtime/provider
   packages after provider certification remains green.
5. Move reusable API test harness and provider certification into an upstream
   `appfw-test` package. Leave product-generated scenario code downstream.
6. Convert downstream apps from copied framework source to dependency-based
   consumption with explicit framework versions or approved git tags.

Each step should preserve current command behavior and prove compatibility with:

```bash
scripts/appfw validate --json
scripts/appfw generate --check --json
scripts/appfw test
scripts/appfw docs-check --json
```

Live `api-test`, `provider-test`, and `release-check` remain required before a
packaging release that changes provider behavior.
