pub(crate) mod database_client;

// Registered for the file-based `bin` module tree (main.rs -> mod data;).
// The `lib` target declares the same module inline in lib.rs.
#[cfg(feature = "provider-mongo")]
pub(crate) mod mongo;
#[cfg(feature = "provider-mssql")]
pub(crate) mod mssql;
#[cfg(feature = "provider-neo4j")]
pub(crate) mod neo4j;
#[cfg(feature = "provider-postgres")]
pub(crate) mod postgres;
#[cfg(feature = "provider-snowflake")]
pub(crate) mod snowflake;
