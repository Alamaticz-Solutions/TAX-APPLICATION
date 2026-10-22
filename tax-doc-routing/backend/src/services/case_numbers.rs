//! Allocates the human-facing case number (shown as TD-1001, TD-1002, ...) for a new routing
//! record.
//!
//! Why this reads Postgres directly instead of going through `DataAccess`: every `DataAccess`
//! read is row-scoped to the caller, and a Tax Staff user only sees their own records, so
//! "the highest number in use" computed that way would be wrong and would collide with other
//! people's records. Handing out the next number is inherently global, so it uses its own
//! execution client the same way `services::authz` does for the RBAC tables.
//!
//! This only *proposes* a number. The `uq_tax_routing_record_case_number` UNIQUE constraint is
//! what actually guarantees uniqueness; `create_impl` retries with a fresh proposal if two
//! creators race.

use std::{collections::HashMap, sync::Arc};

use appfw_provider_postgres::{
    postgres_execution_client as provider_postgres_execution_client,
    validate_postgres_connection_security as provider_validate_postgres_connection_security,
    PostgresConnectionConfig, PostgresExecutionClient,
};

use crate::{
    product_api::AppError,
    schemas::system::DataSource,
    services::authz::resolve_data_source_env,
};

/// The number given to the very first record in an empty database.
pub const FIRST_CASE_NUMBER: i32 = 1001;

pub struct CaseNumbers {
    execution: PostgresExecutionClient,
    /// The highest number this process has handed out. Two requests created at the same moment (a
    /// bulk upload does this) would both read the same highest number from the table and propose the
    /// same next one; the second would then lose the UNIQUE race, throw away its storage folder and
    /// start over. Handing out numbers in order here stops that.
    last: tokio::sync::Mutex<i32>,
}

impl CaseNumbers {
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

        let execution =
            provider_postgres_execution_client(&connection, &security).map_err(AppError::from)?;

        Ok(Arc::new(Self { execution, last: tokio::sync::Mutex::new(0) }))
    }

    /// One past the highest case number held by *any* record, or `FIRST_CASE_NUMBER` when none
    /// has one yet.
    pub async fn next(&self) -> Result<i32, AppError> {
        let client = self
            .execution
            .client()
            .await
            .map_err(|e| AppError::InternalServerError(anyhow::anyhow!(e)))?;

        let rows = client
            .query(
                &format!(
                    "select coalesce(max(case_number), {}) + 1 as next \
                     from tax_routing.tax_routing_records",
                    FIRST_CASE_NUMBER - 1
                ),
                &[],
            )
            .await
            .map_err(|e| AppError::InternalServerError(anyhow::anyhow!(e)))?;

        let row = rows.first().ok_or_else(|| {
            AppError::InternalServerError(anyhow::anyhow!("case number query returned no row"))
        })?;
        let from_table = row.get::<_, i32>("next");
        let mut last = self.last.lock().await;
        let number = from_table.max(*last + 1);
        *last = number;
        Ok(number)
    }
}
