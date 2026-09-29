// MS SQL Server client (ODBC).
//
// Mirrors the structure of `postgres/` and `mongo/`:
//   - mssql_client.rs   - DatabaseClient trait impl + connection pool
//   - filter.rs         - translate the framework's JSON filter DSL to T-SQL
//   - sort.rs           - JSON sort spec to T-SQL ORDER BY
//   - param.rs          - typed parameter binding helpers
//   - cte.rs            - nested nav queries via CROSS APPLY / JSON aggregation
//
// Historical implementation notes live in docs/archive/MSSQL_CLIENT_PLAN.md;
// verify current behavior in the module code and provider contract tests.

pub(crate) mod cte;
pub(crate) mod filter;
pub(crate) mod mssql_client;
pub(crate) mod naming;
pub(crate) mod param;
pub(crate) mod sort;
