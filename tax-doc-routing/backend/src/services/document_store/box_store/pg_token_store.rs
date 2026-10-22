//! `TokenStore` over the `tax_routing.storage_tokens` table.
//!
//! Raw SQL on its own execution client, the same way `services::case_numbers` and
//! `services::authz` reach Postgres: the table is deliberately not reachable through the generated
//! API (no role holds a permission for it), so `DataAccess` could not read it anyway.

use std::{collections::HashMap, sync::Arc};

use appfw_provider_postgres::{
    postgres_execution_client as provider_postgres_execution_client,
    validate_postgres_connection_security as provider_validate_postgres_connection_security,
    PostgresConnectionConfig, PostgresExecutionClient,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::token_manager::{StoredToken, TokenStore};
use crate::{
    product_api::AppError,
    schemas::system::DataSource,
    services::{authz::resolve_data_source_env, document_store::StoreError},
};

pub struct PostgresTokenStore {
    execution: PostgresExecutionClient,
}

fn unavailable(e: impl std::fmt::Display) -> StoreError {
    // Never include the token in an error; only the database's own message.
    StoreError::Unavailable(format!("token table: {e}"))
}

impl PostgresTokenStore {
    pub fn init(sources: &HashMap<String, DataSource>) -> Result<Arc<Self>, AppError> {
        let data_source_env = resolve_data_source_env(sources, "pg_primary")?;
        let security = provider_validate_postgres_connection_security(
            &data_source_env.name,
            &data_source_env.security_profile,
            &data_source_env.tls_mode,
            &data_source_env.db_host,
        )
        .map_err(AppError::from)?;
        let connection = PostgresConnectionConfig::new(
            data_source_env.db_host.clone(),
            data_source_env.db_port.clone(),
            data_source_env.db_name.clone(),
            data_source_env.service_account_name.clone(),
            data_source_env.service_account_password.clone(),
        );
        let execution = provider_postgres_execution_client(&connection, &security).map_err(AppError::from)?;
        Ok(Arc::new(Self { execution }))
    }
}

#[async_trait]
impl TokenStore for PostgresTokenStore {
    async fn load(&self, provider: &str) -> Result<Option<StoredToken>, StoreError> {
        let client = self.execution.client().await.map_err(unavailable)?;
        let rows = client
            .query(
                "select access_token, issued_at, expires_at from tax_routing.storage_tokens where provider = $1",
                &[&provider],
            )
            .await
            .map_err(unavailable)?;
        Ok(rows.first().map(|row| StoredToken {
            access_token: row.get::<_, String>("access_token"),
            issued_at: row.get::<_, DateTime<Utc>>("issued_at"),
            expires_at: row.get::<_, DateTime<Utc>>("expires_at"),
        }))
    }

    /// One row per provider: a new token replaces the old one (`uq_storage_token_provider`).
    async fn save(&self, provider: &str, token: &StoredToken) -> Result<(), StoreError> {
        let client = self.execution.client().await.map_err(unavailable)?;
        client
            .execute(
                "insert into tax_routing.storage_tokens (id, provider, access_token, issued_at, expires_at) \
                 values ($1, $2, $3, $4, $5) \
                 on conflict (provider) do update set \
                   access_token = excluded.access_token, \
                   issued_at = excluded.issued_at, \
                   expires_at = excluded.expires_at",
                &[&uuid::Uuid::new_v4(), &provider, &token.access_token, &token.issued_at, &token.expires_at],
            )
            .await
            .map_err(unavailable)?;
        Ok(())
    }

    async fn clear(&self, provider: &str) -> Result<(), StoreError> {
        let client = self.execution.client().await.map_err(unavailable)?;
        client
            .execute("delete from tax_routing.storage_tokens where provider = $1", &[&provider])
            .await
            .map_err(unavailable)?;
        Ok(())
    }
}
