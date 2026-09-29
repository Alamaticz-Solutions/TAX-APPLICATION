# MS SQL Server Client Notes

**Status:** historical implementation plan. Verify current behavior in
`backend/src/data/clients/mssql/` and the shared provider contract tests before
using this file as guidance.
**Driver:** [`tiberius`](https://docs.rs/tiberius) (pure Rust, async).
**Trait surface:** `crate::data::clients::database_client::DatabaseClient`.

## Scope
Full CRUD parity with the Postgres client:
- create / update / delete with optimistic concurrency
- find / get / query with filter + sort + pagination
- nested-navigation projections (NavToOne, NavToMany)
- DDL codegen for the dialect (in `app_gen/_templates/database/`)

## File-by-file work

| File | Mirrors | LOC est. | TODOs |
|---|---|---|---|
| [mssql/mssql_client.rs](src/data/clients/mssql/mssql_client.rs) | `postgres_client.rs` (1002 LOC) | ~900 | Pool init, all 6 trait methods, transaction handling |
| [mssql/filter.rs](src/data/clients/mssql/filter.rs) | `postgres/filter.rs` (674 LOC) | ~600 | JSON DSL to T-SQL WHERE; identifier `[brackets]`, BIT booleans, JSON_VALUE for nested |
| [mssql/sort.rs](src/data/clients/mssql/sort.rs) | `postgres/sort.rs` (79 LOC) | ~80 | ORDER BY emission, default `[id] ASC` so OFFSET FETCH is legal |
| [mssql/param.rs](src/data/clients/mssql/param.rs) | `postgres/param_field.rs` (286 LOC) | ~250 | DataType to `tiberius::ColumnData` mapping |
| [mssql/record.rs](src/data/clients/mssql/record.rs) | `mongo/record.rs` (306 LOC) | ~280 | Row to JsonObj; rowversion to base64 read_version |
| [mssql/cte.rs](src/data/clients/mssql/cte.rs) | `postgres/cte.rs` (449 LOC) | ~400 | Nested nav via `CROSS APPLY (... FOR JSON PATH)` |

**Total runtime LOC:** ~2,500.

Plus:
- `Cargo.toml`: add `tiberius = "0.12"`, `bb8 = "0.8"`, `bb8-tiberius = "0.16"`, `tokio-util = { version = "0.7", features = ["compat"] }`.
- `app_gen/_templates/database/mssql.j2`: T-SQL DDL template (CREATE TABLE, CREATE INDEX, identity columns, rowversion for concurrency).
- `app_gen/src/database.rs`: dispatch on `data_source_type` to pick template.
- `routes/mod.rs`: dispatch on `data_source_type` when constructing the client (currently always builds `PostgresClient`).
- Integration test: spin up `mcr.microsoft.com/mssql/server:2022-latest` in docker-compose, run the same CRUD suite Postgres uses.

## Dialect cheatsheet (T-SQL vs Postgres)

| Concept | Postgres | T-SQL |
|---|---|---|
| Identifier quoting | `"col"` | `[col]` |
| Boolean | `BOOLEAN` (true/false) | `BIT` (1/0) |
| Auto-PK | `BIGSERIAL` / `IDENTITY` | `BIGINT IDENTITY(1,1)` |
| Now | `now()` | `SYSUTCDATETIME()` |
| String concat | `||` | `+` or `CONCAT()` |
| Pagination | `LIMIT n OFFSET m` | `OFFSET m ROWS FETCH NEXT n ROWS ONLY` (requires ORDER BY) |
| Upsert | `INSERT ... ON CONFLICT` | `MERGE` or `IF EXISTS UPDATE ELSE INSERT` |
| Returning | `RETURNING *` | `OUTPUT INSERTED.* / DELETED.*` |
| JSON | `JSONB` + `->>`, `->` | `NVARCHAR(MAX)` + `JSON_VALUE`, `JSON_QUERY` |
| Array agg | `array_agg`, `json_agg` | `STRING_AGG`, `FOR JSON PATH` |
| Concurrency token | `xmin` or explicit version col | `rowversion` (binary 8) |
| Param binding | `$1, $2, ...` | `@p0, @p1, ...` (named) |

## Connection-string env vars

```
MSSQL_SERVICE_ACCOUNT_NAME=...
MSSQL_SERVICE_ACCOUNT_PASS=...
# host / port / db come from data_sources.yaml
```

## Out of scope (v1)
- Always Encrypted / TDE column encryption surface
- `geography` / `geometry` types
- Linked-server / cross-DB joins
- Replication / readable secondaries (use a separate `data_source_name` per replica)
