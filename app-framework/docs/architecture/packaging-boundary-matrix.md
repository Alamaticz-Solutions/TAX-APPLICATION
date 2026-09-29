# Packaging Boundary Matrix

This one-page matrix is the Step 0 contract baseline for packaging extraction.
Use it before any refactor that moves code between product and framework
surfaces.

## Intent Buckets

Every change must be classified as exactly one of:

1. Product intent
2. Generated product output
3. Reusable framework behavior

If a change spans multiple buckets, split it into separate commits or PRs so
review ownership stays clear.

## Ownership Matrix

| Surface | Owner | Editable In Product Repo | Packaging Target |
| --- | --- | --- | --- |
| `.appfw/manifest.yaml` topology | Product | Yes | Stays product-owned |
| `.appfw/model` source | Product | Yes | Stays product-owned |
| `backend/src/handlers/<schema>/<entity>.rs` | Product | Yes | Stays product-owned |
| `backend/src/services` | Product | Yes | Stays product-owned |
| Deployment overlays and env docs | Product | Yes | Stays product-owned |
| Generated routes/schemas/default handlers | Generated | No (regenerate instead) | Stays generated in product repo |
| Generated `database/_pkg` | Generated | No (regenerate/migrate workflow) | Stays generated in product repo |
| Generated `api_tests/src/schemas` | Generated | No (regenerate instead) | Stays generated in product repo |
| CLI (`appfw-cli`, `scripts/appfw` contract) | Framework | No | Upstream package surface |
| Generator/templates (`appfw-codegen`) | Framework | No | Upstream package surface |
| Runtime (`DataAccess`, product identity, auth, config, GraphQL/MCP/admin service routing support; CORS/GraphiQL/security/observability/routing host helpers, schema-neutral MCP protocol and operation contracts, provider capability profiles and certification report semantics as extracted) | Framework | No | `appfw-runtime` package surface |
| Provider implementations | Framework | No | `appfw-provider-*` or runtime features; package boundaries now include `appfw-provider-mongo` for MongoDB driver connection option shaping, result formatting, BSON scalar conversion, audit document shaping and persistence, mutation key/filter/update document shaping, projection, sort, filter criterion compilation, relationship lookup and nested projection assembly, query/aggregate pipeline assembly, record-value support, driver handle/collection resolution, health ping, find execution, query/aggregate facet execution, facet result decoding, and native error normalization, `appfw-provider-postgres` for PostgreSQL connection pool config shaping, port validation, parameter binding, audit statement rendering and persistence, query/aggregate sort rendering, aggregate select/group/having rendering, mutation statement rendering, filter criterion rendering support, pool handle/client checkout, health check, generic JSON-row query execution, aggregate row execution, row count/array decoding, and native error normalization, `appfw-provider-mssql` for MS SQL Server client config shaping, encryption/trust-cert mapping, pool construction/handle/client checkout, pool-size support, port validation, parameter binding, audit statement rendering and persistence, query/aggregate sort rendering, aggregate select/group/having rendering, mutation statement rendering, filter criterion rendering support, generic query/execute/batch helpers, JSON-row query execution, aggregate row execution, row count/JSON decoding, health check, and native error normalization, and `appfw-provider-snowflake` for Snowflake SQL API endpoint/session config shaping, token requirement/defaults, statement/binding, HTTP SQL API execution/polling, response row decoding, health check, audit persistence/query execution, native error normalization, query/aggregate sort rendering, aggregate select/group/having rendering, mutation statement rendering, and filter criterion rendering support |
| API scenario harness/provider certification | Framework | No | `appfw-test` package surface |
| Policy verifier engine (`appfw_test/src/policy`) | Framework | No | framework verifier package/command |
| Product policy fixtures (`rego_test`) | Product | Yes | product fixtures/assertions consuming framework verifier |

## Dependency Rules

Product crates may depend on:

- stable framework package APIs (`appfw-cli`, `appfw-codegen`, `appfw-runtime`,
  `appfw-test`, and future runtime/provider packages);
- generated schema and operation surfaces; and
- product-owned handlers/services.

Product crates must not depend on:

- framework internal modules not exported as package API;
- provider client internals (except documented, explicit product exceptions);
- copied framework runtime source as a customization mechanism.

Product-owned handlers/services must enter shared runtime behavior
through `crate::product_api`, generated schema surfaces, or product-owned
services. Direct imports from `crate::data`, `crate::config`, `crate::routes`,
auth handler internals, MCP, observability, admin UI, app state, or provider
certification modules are boundary violations.

`UserAuth` and `Claims` are runtime identity contracts. Generated adapters
package request identity, data access, entity metadata, and selections through
`crate::product_api::HandlerContext`; product-owned handlers/services receive
the explicit handler arguments and auth extractor modules remain framework
implementation details.

## Runtime And Provider Rules

- Runtime behavior is framework-owned; topology activation is product-owned.
- `examples/products/crm/backend` is the canonical sample product backend.
  The repository root does not carry a product backend crate; framework-root
  product workflows default to the CRM sample unless `APPFW_APP_ROOT` selects a
  different product app.
- Global HTTP layers and admin/MCP feature-gated route mounting are runtime
  behavior. Generated product route modules should supply schema routers and
  product data access, not own the framework HTTP shell.
- MCP protocol constants, JSON-RPC envelopes, and reflected operation metadata
  are runtime contracts. Generated product MCP dispatchers may reference those
  contracts, but should not define them locally.
- Runtime extraction must keep GraphQL service, MCP server, and admin UI
  integration in the same framework-owned boundary move.
  Do not treat GraphQL-only extraction as complete.
- Cargo dependencies/features control which providers are compilable.
- Manifest topology controls which configured data sources are active at
  runtime.
- Provider behavior changes require certification evidence before release.

## Step 0 Exit Criteria

Step 0 is complete only when:

1. This matrix is referenced by packaging and workspace contract docs.
2. Dependency rules are documented in `docs/architecture/framework-packaging.md`.
3. Reviewers can classify each packaging PR by intent bucket and owner with no
   ambiguity.
