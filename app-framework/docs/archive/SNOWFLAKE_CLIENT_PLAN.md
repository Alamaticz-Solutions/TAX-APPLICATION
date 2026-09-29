# Snowflake Client Notes

**Status:** historical implementation plan. Verify current behavior in
`backend/src/data/clients/snowflake/` and the shared provider contract tests
before using this file as guidance.
**Driver decision from the original plan:** REST API + JWT.
**Trait surface:** `crate::data::clients::database_client::DatabaseClient`.

Snowflake is typically read-heavy in app contexts (analytics, BI, reporting).
Recommend implementing **read-only first** (find / get / query) and deferring
writes until a concrete use case justifies them.

## Driver options

| Approach | Pros | Cons |
|---|---|---|
| REST API + `reqwest` + `jsonwebtoken` | No ODBC dependency, pure Rust, controllable, works in any environment | Need to implement type coercion + statement-status polling for async |
| `arrow-odbc` over Snowflake ODBC | Fast columnar transfers, mature driver | Requires ODBC + Snowflake driver installed on every host |
| `snowflake-api` crate | Native Rust, ergonomic | Alpha, sparse coverage, instability risk |

**Recommendation:** REST + JWT. Predictable footprint, no system-level deps,
matches enterprise OAuth/key-pair auth patterns.

## Dialect cheatsheet (Snowflake vs Postgres)

| Concept | Postgres | Snowflake |
|---|---|---|
| Identifier quoting | `"col"` (case-preserving) | `"col"` (case-preserving); unquoted gets UPPERCASED |
| Boolean | `BOOLEAN` | `BOOLEAN` |
| Auto-PK | `BIGSERIAL` | `IDENTITY(1,1)` or sequences |
| Now | `now()` | `CURRENT_TIMESTAMP()` |
| Pagination | `LIMIT n OFFSET m` | `LIMIT n OFFSET m` (same) |
| JSON | `JSONB` | `VARIANT`, `OBJECT`, `ARRAY` |
| Array agg | `array_agg`, `json_agg` | `ARRAY_AGG`, `OBJECT_CONSTRUCT` |
| Returning | `RETURNING *` | not supported; re-SELECT after write |
| Concurrency token | `xmin` | `HASH(*)` over the row, or explicit `version` column |

## Scope (read-only v1)

| File | Purpose |
|---|---|
| `snowflake_client.rs` | `DatabaseClient` impl, REST/JWT client, statement polling |
| `query_builder.rs` | JSON DSL to Snowflake SQL (filter + sort + pagination) |
| `record.rs` | Result set to JsonObj; VARIANT/ARRAY/OBJECT mapping |
| `auth.rs` | RSA key-pair JWT generation per Snowflake docs |

## Deferred (phase 2)

- Writes (`create_item_json`, `update_item_json`, `delete_item_json`)
- DDL codegen (`app_gen/_templates/database/snowflake.j2`)
- Optimistic concurrency control

## Env vars

```
SNOWFLAKE_SERVICE_ACCOUNT_NAME=...
SNOWFLAKE_SERVICE_ACCOUNT_PASS=...
# For key-pair auth (preferred):
SNOWFLAKE_PRIVATE_KEY_PATH=/etc/secrets/sf_rsa.p8
SNOWFLAKE_PRIVATE_KEY_PASSPHRASE=...
# Host/account from data_sources.yaml
```
