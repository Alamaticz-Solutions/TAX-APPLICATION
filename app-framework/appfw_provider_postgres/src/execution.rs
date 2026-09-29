use appfw_runtime::{RuntimeAuditEvent, RuntimeAuditQuery, RuntimeError};
use deadpool_postgres::{Client, Pool};
use postgres_types::ToSql;
use serde_json::Value;
use tokio_postgres::Row;

use crate::{
    audit_insert_statement, audit_previous_hash_statement, audit_query_statement,
    postgres_runtime_error, PostgresFunctionCall, PostgresStoredProcedureCall, SqlParam,
};

#[derive(Clone)]
pub struct PostgresExecutionClient {
    pool: Pool,
}

pub struct PostgresJsonRows {
    pub query_count: i64,
    pub rows: Option<Value>,
}

pub struct PostgresAggregateRows {
    pub total: i64,
    pub rows: Vec<Value>,
}

impl PostgresExecutionClient {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &Pool {
        &self.pool
    }

    pub async fn client(&self) -> Result<Client, RuntimeError> {
        self.pool
            .get()
            .await
            .map_err(|e| RuntimeError::DataAccess(e.to_string()))
    }

    pub async fn health_check(&self) -> Result<(), RuntimeError> {
        self.client()
            .await?
            .query_one("SELECT 1", &[])
            .await
            .map_err(postgres_runtime_error)?;
        Ok(())
    }

    pub async fn append_audit_event(&self, event: RuntimeAuditEvent) -> Result<(), RuntimeError> {
        let client = self.client().await?;
        let (prev_sql, prev_params) = audit_previous_hash_statement(&event);
        let prev_hash = client
            .query_opt(&prev_sql, &param_refs(&prev_params))
            .await
            .map_err(postgres_runtime_error)?
            .map(|row| row.get::<_, String>(0));
        let event = event.finalize(prev_hash)?;

        let (sql, params) = audit_insert_statement(&event)?;
        client
            .execute(&sql, &param_refs(&params))
            .await
            .map_err(postgres_runtime_error)?;
        Ok(())
    }

    pub async fn query_audit_events(
        &self,
        query: RuntimeAuditQuery,
    ) -> Result<Vec<Value>, RuntimeError> {
        let (sql, params) = audit_query_statement(&query);
        let row = self
            .client()
            .await?
            .query_one(&sql, &param_refs(&params))
            .await
            .map_err(postgres_runtime_error)?;
        let events: Value = row
            .try_get("events")
            .map_err(|e| RuntimeError::Internal(e.to_string()))?;
        Ok(events.as_array().cloned().unwrap_or_default())
    }

    pub async fn query_json_rows(
        &self,
        sql: &str,
        params: &[SqlParam],
    ) -> Result<Option<PostgresJsonRows>, RuntimeError> {
        let row = self
            .client()
            .await?
            .query_opt(sql, &param_refs(params))
            .await
            .map_err(postgres_runtime_error)?;
        row.map(postgres_json_rows).transpose()
    }

    pub async fn query_json_rows_prepared(
        &self,
        sql: &str,
        params: &[SqlParam],
    ) -> Result<Option<PostgresJsonRows>, RuntimeError> {
        let rows = self.query_prepared(sql, params).await?;
        rows.into_iter().next().map(postgres_json_rows).transpose()
    }

    pub async fn query_prepared(
        &self,
        sql: &str,
        params: &[SqlParam],
    ) -> Result<Vec<Row>, RuntimeError> {
        let client = self.client().await?;
        let statement = client
            .prepare_cached(sql)
            .await
            .map_err(postgres_runtime_error)?;
        client
            .query(&statement, &param_refs(params))
            .await
            .map_err(postgres_runtime_error)
    }

    pub async fn execute_prepared(
        &self,
        sql: &str,
        params: &[SqlParam],
    ) -> Result<u64, RuntimeError> {
        let client = self.client().await?;
        let statement = client
            .prepare_cached(sql)
            .await
            .map_err(postgres_runtime_error)?;
        client
            .execute(&statement, &param_refs(params))
            .await
            .map_err(postgres_runtime_error)
    }

    pub async fn call_stored_procedure(
        &self,
        call: &PostgresStoredProcedureCall,
        params: &[SqlParam],
    ) -> Result<u64, RuntimeError> {
        ensure_argument_count(
            "PostgreSQL stored procedure",
            call.argument_count(),
            params.len(),
        )?;
        self.execute_prepared(&call.statement()?, params).await
    }

    pub async fn query_function(
        &self,
        call: &PostgresFunctionCall,
        params: &[SqlParam],
    ) -> Result<Vec<Row>, RuntimeError> {
        ensure_argument_count("PostgreSQL function", call.argument_count(), params.len())?;
        self.query_prepared(&call.statement()?, params).await
    }

    pub async fn aggregate_json_rows(
        &self,
        sql: &str,
        params: &[SqlParam],
    ) -> Result<PostgresAggregateRows, RuntimeError> {
        let row = self
            .client()
            .await?
            .query_one(sql, &param_refs(params))
            .await
            .map_err(postgres_runtime_error)?;
        let total = row
            .try_get(0)
            .map_err(|e| RuntimeError::Internal(e.to_string()))?;
        let rows: Option<Value> = row
            .try_get(1)
            .map_err(|e| RuntimeError::Internal(e.to_string()))?;
        Ok(PostgresAggregateRows {
            total,
            rows: json_array(rows),
        })
    }
}

fn ensure_argument_count(label: &str, expected: usize, actual: usize) -> Result<(), RuntimeError> {
    if expected == actual {
        return Ok(());
    }
    Err(RuntimeError::Validation(format!(
        "{label} expected {expected} argument(s), got {actual}"
    )))
}

fn param_refs(params: &[SqlParam]) -> Vec<&(dyn ToSql + Sync)> {
    params
        .iter()
        .map(|v| v.as_ref() as &(dyn ToSql + Sync))
        .collect()
}

fn postgres_json_rows(row: Row) -> Result<PostgresJsonRows, RuntimeError> {
    let query_count = row
        .try_get(0)
        .map_err(|e| RuntimeError::Internal(e.to_string()))?;
    let rows = row
        .try_get(1)
        .map_err(|e| RuntimeError::Internal(e.to_string()))?;
    Ok(PostgresJsonRows { query_count, rows })
}

fn json_array(value: Option<Value>) -> Vec<Value> {
    value
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn json_array_returns_array_or_empty_vec() {
        assert_eq!(json_array(Some(json!([{"id": 1}]))), vec![json!({"id": 1})]);
        assert!(json_array(None).is_empty());
        assert!(json_array(Some(json!({"id": 1}))).is_empty());
    }

    #[test]
    fn stored_routine_argument_count_must_match_bound_params() {
        assert!(ensure_argument_count("PostgreSQL stored procedure", 2, 2).is_ok());
        assert!(matches!(
            ensure_argument_count("PostgreSQL stored procedure", 2, 1),
            Err(RuntimeError::Validation(_))
        ));
    }
}
