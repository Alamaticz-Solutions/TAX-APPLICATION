pub(crate) mod database_client;

// Registered for the file-based `bin` module tree (main.rs -> mod data;).
// The `lib` target declares the same module inline in lib.rs.
#[cfg(feature = "provider-postgres")]
pub(crate) mod postgres;
