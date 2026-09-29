# Custom Resolvers And GraphQL Operations

Custom GraphQL behavior is added through generated custom methods and
human-owned handler implementation files. Do not create a separate
GraphQL-specific source tree; the current backend is organized around generated
`routes`, `handlers`, `schemas`, and `data_access`.

## Add A Custom Method

Declare the method in the entity config:

```yaml
- name: Account
  id: 11111111-1111-1111-1111-111111111111
  is_table: true
  standard_methods: [FindById, GetAll, Query, Create, Update, Delete]
  custom_methods:
  - name: account_health
    kind: Query
    args:
    - name: account_id
      arg_type: String
    return_type: serde_json::Value
  props:
  - name: id
    fragment: property-primary-key-uuid
  - name: name
    fragment: property-string-caption
```

Run:

```bash
scripts/appfw validate
scripts/appfw generate
```

The generated schema module wires the GraphQL operation. Generated defaults are
regenerated in:

```text
backend/src/handlers/<schema>/generated.rs
```

The product-owned implementation surface is created once and then preserved in:

```text
backend/src/handlers/<schema>/<entity>.rs
```

## Optimize Generated SQL With Prepared Statements

Prepared statements optimize generated CRUD/query SQL. They are not stored
procedures and they are not database objects created by migrations. Opt in on
the entity type:

```yaml
- name: Account
  is_table: true
  execution:
    prepared_statements: true
```

When enabled, the generated data-access layer asks the provider to use its
prepared/cached execution path for generated read/query SQL where supported.
PostgreSQL uses cached prepared statements for the generated CTE read path. MS
SQL Server and Snowflake continue to use provider-owned bound parameters unless
a reusable prepared-handle lifecycle is certified for those providers.

## Bind A Custom Method To A Provider Routine

For vetted database functions or stored procedures, declare a provider routine
binding on the custom method instead of writing SQL in the handler:

```yaml
custom_methods:
- name: account_health_from_provider
  kind: Query
  args:
  - name: account_id
    arg_type: String
  return_type: serde_json::Value
  provider_routine:
    kind: Function
    returns: One
    data_source: pg_primary
    schema: crm
    name: account_health_score
```

Routine bindings are generation-time metadata. They are not exposed through
runtime entity metadata. Generated defaults call `DataAccess`, which enforces
the same authentication, policy, provider, and parameter-binding path as normal
operations. The provider client owns the final function/procedure invocation.

Rules:

- Use `kind: Function` for query methods.
- Use `kind: Procedure` for mutations/commands; procedures may return `One`,
  `Many`, or `None` when the provider routine supports that result shape.
- Use `returns: One` for one DTO, `Many` for a list, and `None` for side-effect
  routines.
- Keep routine `schema` and `name` as simple ASCII identifiers. SQL text,
  `CALL`, `EXEC`, and provider scripts do not belong in config.
- Use the top-level `schema` and `name` for the portable routine target. Add
  `routines.postgres`, `routines.mssql`, or `routines.snowflake` only when a
  provider needs a different physical name.
- Prefer a non-table entity type for stable DTO return shapes; keep
  `serde_json::Value` for exploratory results.

The CRM example model includes a prepared generated-query fixture and two
provider-routine fixtures on `Account`:

| Model element | Kind | Provider intent | Providers |
| --- | --- | --- | --- |
| `Account.execution.prepared_statements` | Generated read/query SQL | Prepared/cached execution for generated CRUD/query SQL where supported. | PostgreSQL cached prepared statements; MS SQL Server/Snowflake bound parameter paths |
| `account_health_provider_function` | `Query` | Function-backed custom read path through provider routine invocation. | PostgreSQL, MS SQL Server, Snowflake |
| `refresh_account_health_stored_procedure` | `Mutation` | Procedure-backed command path that persists and returns an account health snapshot. | PostgreSQL, MS SQL Server, Snowflake |

Routine methods are intentionally not “SQL in YAML.” The model names a vetted
provider routine. The generated handler passes typed arguments to `DataAccess`,
and the concrete provider client owns function/procedure invocation.

The fixture DDL lives in:

```text
database/_pkg/migrations/postgresql/202606050000__crm_provider_routine_examples.expand.sql
database/_pkg/migrations/mssql/202606050000__crm_provider_routine_examples.expand.sql
database/_pkg/migrations/snowflake/202606050000__crm_provider_routine_examples.expand.sql
```

## CRM Example Scenarios

The CRM config includes a few representative custom methods:

| Entity | Method | Kind | Scenario |
| --- | --- | --- | --- |
| `Account` | `account_health` | `Query` | Roll up account-level relationship, revenue, and activity signals. |
| `Account` | `account_health_provider_function` | `Query` | Provider-owned function fixture. |
| `Account` | `refresh_account_health_stored_procedure` | `Mutation` | Provider-owned stored procedure that persists and returns the account health snapshot row. |
| `Opportunity` | `pipeline_forecast` | `Query` | Produce a pipeline forecast with optional stage and amount filters. |
| `Lead` | `convert_lead` | `Mutation` | Model a business action that converts a lead into downstream records. |
| `Quote` | `reprice_quote` | `Mutation` | Recalculate quote pricing after line-item or tax changes. |

These are declared under `.appfw/model/schemas/crm/entity_types/*.yaml`.
When adding custom methods to an entity whose handler file already exists,
generation adds a default `<method>_impl` to the generated defaults module. The
human-owned handler imports those defaults. To implement product behavior, add a
local `<method>_impl` function with the same signature to the handler file.

## Implement The Stub

Product overrides follow this shape:

```rust
pub async fn account_health_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    selections: Value,
    account_id: String,
) -> Result<serde_json::Value> {
    let _ = (user, data_access, entity_type, selections, account_id);
    Err(anyhow::anyhow!("custom method `account_health` is not implemented yet"))
}
```

Replace the explicit error with application logic. The file is human-owned, so
future generation should not overwrite your implementation. Substantial logic
should move into a service module and leave the handler as a thin adapter.

## Use DataAccess

Use `DataAccess` when custom operations should respect the same provider and
access-control path as generated operations. Keep direct provider calls rare and
well documented.

```rust
let result = data_access
    .find_one(user, entity_type, selections, account_id)
    .await?;
```

Exact helper names evolve with the backend API, so inspect nearby generated
handler implementations before adding custom logic.

## Return Types

Prefer generated schema types or explicit Rust types when the response shape is
stable. Use `serde_json::Value` for exploratory or provider-specific results,
then promote the shape once it becomes durable.

## Verification

```bash
scripts/appfw validate
scripts/appfw test
```

Add generated API scenarios under:

```text
.appfw/model/schemas/<schema>/tests/
```

Then run them against a prepared backend:

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
