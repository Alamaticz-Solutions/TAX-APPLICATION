// Shared T-SQL query-builder helpers.
//
// Fabric SQL analytics uses these helpers for the framework query/filter/sort
// dialect. Fabric-only products should not compile the SQL Server data-source
// client.

pub(crate) mod cte;
pub(crate) mod filter;
pub(crate) mod naming;
pub(crate) mod param;
pub(crate) mod sort;
