//! Box implementation of `DocumentStore`: JWT sign-in (`auth`), the HTTP layer (`http`), the five
//! store operations (`store`) the settings (`config`) and the token lifecycle: sign in (`auth`), keep the token in the
//! `storage_tokens` table and renew it when it expires (`token_manager`, `pg_token_store`).
//! Mirrors what Pega's `GenerateToken_Box` and the `*BoxFolder*` / `SaveToBoxStorage` activities
//! do. Off unless `STORAGE_PROVIDER=box`.

pub mod auth;
pub mod config;
pub mod http;
pub mod pg_token_store;
pub mod store;
pub mod token_manager;

pub use config::BoxConfig;
pub use pg_token_store::PostgresTokenStore;
pub use store::BoxDocumentStore;
pub use token_manager::TokenStore;
