use std::{env, sync::Arc, time::Duration};

use appfw_runtime::{
    provider_error as runtime_provider_error, provider_keys::FrameworkProvider, RuntimeAuditEvent,
    RuntimeAuditQuery, RuntimeError, RuntimeJsonObj,
};
use bb8::{ManageConnection, Pool, PooledConnection};
use serde_json::Value;
use tokio::task;

use crate::row::MssqlRow;
use crate::{
    audit_insert_statement, audit_previous_hash_statement, audit_query_statement,
    connection::mssql_odbc_connection_string,
    odbc::{
        connect_with_connection_string, fetch_rows_as_mssql_rows, login_timeout_secs,
        odbc_sql_and_params, OdbcConnection,
    },
    MssqlStoredProcedureCall, SqlParam, MSSQL_POOL_MAX_SIZE,
};

pub use crate::row::MssqlRow as MssqlRowExport;

pub type MssqlPool = Pool<MssqlConnectionManager>;
pub type MssqlConnection<'a> = PooledConnection<'a, MssqlConnectionManager>;

const POOL_CONNECTION_TIMEOUT_SLACK_MS: u64 = 5_000;
const POOL_CONNECTION_TIMEOUT_ENV: &str = "APP_MSSQL_POOL_CONNECTION_TIMEOUT_MS";

#[derive(Clone)]
pub struct MssqlExecutionClient {
    pool: MssqlPool,
    connection_timeout_ms: u64,
}

pub struct MssqlJsonRows {
    pub total: i64,
    pub items: Vec<RuntimeJsonObj>,
}

pub struct MssqlAggregateRows {
    pub total: i64,
    pub items: Vec<Value>,
}

#[derive(Clone)]
pub(crate) struct MssqlSessionInner {
    connection_string: String,
    conn: Arc<std::sync::Mutex<Option<OdbcConnection>>>,
}

impl MssqlSessionInner {
    fn with_conn<F, T>(&self, f: F) -> Result<T, RuntimeError>
    where
        F: FnOnce(&mut OdbcConnection) -> Result<T, RuntimeError>,
    {
        let mut guard = self
            .conn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if guard.is_none() {
            *guard = Some(connect_with_connection_string(&self.connection_string)?);
        }
        let conn = guard.as_mut().expect("odbc connection initialized");
        f(conn)
    }

    /// Drop any open transaction left on a pooled session.
    ///
    /// Multi-statement CRUD paths use BEGIN/COMMIT on one checkout. If a path
    /// returns the connection without rolling back (error 266, canceled work,
    /// or a failed ROLLBACK), the next borrower can hang on locks. Resetting
    /// on checkout keeps the pool fail-closed.
    fn rollback_open_transactions(&self) -> Result<(), RuntimeError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare("IF @@TRANCOUNT > 0 ROLLBACK TRANSACTION;")?;
            stmt.execute()?;
            // Consume any DONE_IN_PROC / informational results so the
            // connection is idle before the next statement.
            while stmt.more_results()? {}
            Ok(())
        })
    }

    fn discard_connection(&self) {
        let mut guard = self
            .conn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *guard = None;
    }
}

#[derive(Clone)]
pub struct ManagedOdbcSession(pub(crate) Arc<MssqlSessionInner>);

#[derive(Clone)]
pub struct MssqlConnectionManager {
    connection_string: String,
}

impl MssqlConnectionManager {
    pub fn new(connection_string: String) -> Self {
        Self { connection_string }
    }
}

impl ManageConnection for MssqlConnectionManager {
    type Connection = ManagedOdbcSession;
    type Error = RuntimeError;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let connection_string = self.connection_string.clone();
        task::spawn_blocking(move || {
            let session = Arc::new(MssqlSessionInner {
                connection_string,
                conn: Arc::new(std::sync::Mutex::new(None)),
            });
            session.with_conn(|_| Ok(()))?;
            Ok(ManagedOdbcSession(session))
        })
        .await
        .map_err(|e| RuntimeError::DataAccess(format!("ODBC pool worker failed: {e}")))?
    }

    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        let session = conn.0.clone();
        task::spawn_blocking(move || {
            session.rollback_open_transactions()?;
            session.with_conn(|_| Ok(()))
        })
        .await
        .map_err(|e| RuntimeError::DataAccess(format!("ODBC pool worker failed: {e}")))?
    }

    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        false
    }
}

impl MssqlExecutionClient {
    pub async fn from_connection_config(
        connection: &crate::connection::MssqlConnectionConfig,
        security: &appfw_runtime::connection_security::ConnectionSecurity,
    ) -> Result<Self, RuntimeError> {
        let resolved =
            crate::connection::mssql_connection_config_resolving_auth(connection).await?;
        let connection_string = mssql_odbc_connection_string(&resolved, security)?;
        Self::from_connection_string(connection_string).await
    }

    pub async fn from_connection_string(connection_string: String) -> Result<Self, RuntimeError> {
        let mgr = MssqlConnectionManager::new(connection_string);
        let connection_timeout_ms = pool_connection_timeout_ms();
        let pool = Pool::builder()
            .max_size(MSSQL_POOL_MAX_SIZE)
            .min_idle(1)
            .connection_timeout(Duration::from_millis(connection_timeout_ms))
            .retry_connection(false)
            .build(mgr)
            .await
            .map_err(|e| {
                RuntimeError::DataAccess(format!(
                    "MS SQL-compatible pool initialization failed after {connection_timeout_ms}ms: {e}"
                ))
            })?;
        Ok(Self {
            pool,
            connection_timeout_ms,
        })
    }

    pub fn pool(&self) -> &MssqlPool {
        &self.pool
    }

    pub async fn connection(&self) -> Result<MssqlConnection<'_>, RuntimeError> {
        let mut conn = self.pool.get().await.map_err(|e| {
            RuntimeError::DataAccess(format!(
                "MS SQL-compatible connection checkout failed after {}ms: {e}",
                self.connection_timeout_ms
            ))
        })?;
        if let Err(error) = Self::rollback_open_transactions_on_conn(&mut conn).await {
            // Connection is unsafe to reuse; drop the underlying ODBC handle so
            // the next checkout opens a fresh session.
            conn.0.discard_connection();
            return Err(error);
        }
        Ok(conn)
    }

    pub async fn rollback_open_transactions_on_conn(
        conn: &mut MssqlConnection<'_>,
    ) -> Result<(), RuntimeError> {
        let session = conn.0.clone();
        task::spawn_blocking(move || session.rollback_open_transactions())
            .await
            .map_err(|e| RuntimeError::DataAccess(format!("ODBC worker failed: {e}")))?
    }

    pub async fn run_query(
        &self,
        sql: &str,
        params: Vec<SqlParam>,
    ) -> Result<Vec<MssqlRow>, RuntimeError> {
        let mut conn = self.connection().await?;
        let result = Self::run_query_on_conn(&mut conn, sql, params).await;
        if result.is_err() {
            let _ = Self::rollback_open_transactions_on_conn(&mut conn).await;
        }
        result
    }

    pub async fn run_parameterized_execute(
        &self,
        sql: &str,
        params: Vec<SqlParam>,
    ) -> Result<u64, RuntimeError> {
        let mut conn = self.connection().await?;
        let result = Self::run_execute_on_conn(&mut conn, sql, params).await;
        if result.is_err() {
            let _ = Self::rollback_open_transactions_on_conn(&mut conn).await;
        }
        result
    }

    pub async fn query_stored_procedure(
        &self,
        call: &MssqlStoredProcedureCall,
        params: Vec<SqlParam>,
    ) -> Result<Vec<MssqlRow>, RuntimeError> {
        ensure_argument_count(
            "MS SQL Server stored procedure",
            call.argument_count(),
            params.len(),
        )?;
        self.run_query(&call.statement()?, params).await
    }

    pub async fn execute_stored_procedure(
        &self,
        call: &MssqlStoredProcedureCall,
        params: Vec<SqlParam>,
    ) -> Result<u64, RuntimeError> {
        ensure_argument_count(
            "MS SQL Server stored procedure",
            call.argument_count(),
            params.len(),
        )?;
        self.run_parameterized_execute(&call.statement()?, params)
            .await
    }

    pub async fn run_query_on_conn(
        conn: &mut MssqlConnection<'_>,
        sql: &str,
        params: Vec<SqlParam>,
    ) -> Result<Vec<MssqlRow>, RuntimeError> {
        let session = conn.0.clone();
        let sql = sql.to_string();
        task::spawn_blocking(move || run_query_blocking(&session, sql, params))
            .await
            .map_err(|e| RuntimeError::DataAccess(format!("ODBC worker failed: {e}")))?
    }

    pub async fn run_execute_on_conn(
        conn: &mut MssqlConnection<'_>,
        sql: &str,
        params: Vec<SqlParam>,
    ) -> Result<u64, RuntimeError> {
        let session = conn.0.clone();
        let sql = sql.to_string();
        task::spawn_blocking(move || run_execute_blocking(&session, sql, params))
            .await
            .map_err(|e| RuntimeError::DataAccess(format!("ODBC worker failed: {e}")))?
    }

    pub async fn run_batch_on_conn(
        conn: &mut MssqlConnection<'_>,
        sql: &str,
    ) -> Result<(), RuntimeError> {
        let session = conn.0.clone();
        let sql = sql.to_string();
        task::spawn_blocking(move || {
            session.with_conn(|conn| {
                let mut stmt = conn.prepare(&sql)?;
                stmt.execute()?;
                while stmt.more_results()? {}
                Ok(())
            })
        })
        .await
        .map_err(|e| RuntimeError::DataAccess(format!("ODBC worker failed: {e}")))?
    }

    pub async fn health_check(&self) -> Result<(), RuntimeError> {
        self.run_query("SELECT 1 AS [ok]", Vec::new()).await?;
        Ok(())
    }

    pub async fn append_audit_event(&self, event: RuntimeAuditEvent) -> Result<(), RuntimeError> {
        let (prev_sql, prev_params) = audit_previous_hash_statement(&event);
        let prev_rows = self.run_query(&prev_sql, prev_params).await?;
        let prev_hash = prev_rows
            .first()
            .and_then(|row| row.try_get::<String, _>("event_hash").ok().flatten());
        let event = event.finalize(prev_hash)?;

        let (sql, params) = audit_insert_statement(&event)?;
        self.run_query(&sql, params).await?;
        Ok(())
    }

    pub async fn query_audit_events(
        &self,
        query: RuntimeAuditQuery,
    ) -> Result<Vec<Value>, RuntimeError> {
        let (sql, params) = audit_query_statement(&query);
        let rows = self.run_query(&sql, params).await?;
        let events_json = rows
            .first()
            .and_then(|row| row.try_get::<String, _>("events").ok().flatten())
            .unwrap_or_else(|| "[]".to_string());
        parse_json_values(&events_json, "MSSQL audit timeline JSON payload")
    }

    pub async fn query_json_rows(
        &self,
        sql: &str,
        params: Vec<SqlParam>,
    ) -> Result<Option<MssqlJsonRows>, RuntimeError> {
        let rows = self.run_query(sql, params).await?;
        rows.into_iter().next().map(json_rows_from_row).transpose()
    }

    pub async fn aggregate_json_rows(
        &self,
        sql: &str,
        params: Vec<SqlParam>,
    ) -> Result<Option<MssqlAggregateRows>, RuntimeError> {
        let rows = self.run_query(sql, params).await?;
        rows.into_iter()
            .next()
            .map(aggregate_rows_from_row)
            .transpose()
    }
}

fn run_query_blocking(
    session: &MssqlSessionInner,
    sql: String,
    params: Vec<SqlParam>,
) -> Result<Vec<MssqlRow>, RuntimeError> {
    session.with_conn(|conn| {
        let (sql, params) = odbc_sql_and_params(sql, params)?;
        let mut stmt = if params.is_empty() {
            conn.execute_direct(&sql)?
        } else {
            let mut stmt = conn.prepare(&sql)?;
            stmt.bind_params(params)?;
            stmt.execute()?;
            stmt
        };
        fetch_rows_as_mssql_rows(&mut stmt).map_err(map_odbc_error)
    })
}

fn run_execute_blocking(
    session: &MssqlSessionInner,
    sql: String,
    params: Vec<SqlParam>,
) -> Result<u64, RuntimeError> {
    session.with_conn(|conn| {
        let (sql, params) = odbc_sql_and_params(sql, params)?;
        let mut stmt = conn.prepare(&sql)?;
        stmt.bind_params(params)?;
        stmt.execute()?;
        stmt.rows_affected().map_err(map_odbc_error)
    })
}

fn map_odbc_error(error: RuntimeError) -> RuntimeError {
    let message = error.to_string();
    RuntimeError::DataStore(runtime_provider_error::normalize_provider_error(
        FrameworkProvider::Mssql,
        message,
    ))
}

fn pool_connection_timeout_ms() -> u64 {
    let default_ms = (login_timeout_secs() as u64 * 1000) + POOL_CONNECTION_TIMEOUT_SLACK_MS;
    env::var(POOL_CONNECTION_TIMEOUT_ENV)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default_ms)
}

fn json_rows_from_row(row: MssqlRow) -> Result<MssqlJsonRows, RuntimeError> {
    let total = row
        .try_get::<i64, _>("count")
        .map_err(|e| RuntimeError::Internal(e.to_string()))?
        .unwrap_or(0);
    let rows_json = row
        .try_get::<String, _>("rows")
        .map_err(|e| RuntimeError::Internal(e.to_string()))?
        .unwrap_or_else(|| "[]".to_string());
    let items = parse_json_objects(&rows_json, "MSSQL JSON rows payload")?;
    Ok(MssqlJsonRows { total, items })
}

fn aggregate_rows_from_row(row: MssqlRow) -> Result<MssqlAggregateRows, RuntimeError> {
    let total = row
        .try_get::<i64, _>("count")
        .map_err(|e| RuntimeError::Internal(e.to_string()))?
        .unwrap_or(0);
    let rows_json = row
        .try_get::<String, _>("rows")
        .map_err(|e| RuntimeError::Internal(e.to_string()))?
        .unwrap_or_else(|| "[]".to_string());
    let items = parse_json_values(&rows_json, "MSSQL aggregate JSON rows payload")?;
    Ok(MssqlAggregateRows { total, items })
}

fn parse_json_values(payload: &str, label: &str) -> Result<Vec<Value>, RuntimeError> {
    serde_json::from_str::<Vec<Value>>(payload)
        .map_err(|e| RuntimeError::DataAccess(format!("invalid {label}: {e}")))
}

fn parse_json_objects(payload: &str, label: &str) -> Result<Vec<RuntimeJsonObj>, RuntimeError> {
    parse_json_values(payload, label)?
        .into_iter()
        .map(|value| match value {
            Value::Object(obj) => Ok(obj),
            other => Err(RuntimeError::Internal(format!(
                "{label} item must be an object, got {other:?}"
            ))),
        })
        .collect()
}

fn ensure_argument_count(label: &str, expected: usize, actual: usize) -> Result<(), RuntimeError> {
    if expected == actual {
        return Ok(());
    }
    Err(RuntimeError::Validation(format!(
        "{label} expected {expected} argument(s), got {actual}"
    )))
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, MutexGuard, OnceLock};

    use serde_json::{json, Map};

    use super::*;
    use crate::odbc::{DEFAULT_LOGIN_TIMEOUT_SECS, LOGIN_TIMEOUT_ENV};

    fn env_lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[test]
    fn parses_json_objects() {
        let expected: Vec<Map<String, Value>> = vec![
            json!({"id": 1}).as_object().unwrap().clone(),
            json!({"id": 2}).as_object().unwrap().clone(),
        ];

        let items = parse_json_objects(r#"[{"id":1},{"id":2}]"#, "rows").expect("objects");

        assert_eq!(items, expected);
    }

    #[test]
    fn rejects_non_object_json_rows() {
        let error = parse_json_objects(r#"[1]"#, "rows").expect_err("invalid object row");

        assert!(
            matches!(error, RuntimeError::Internal(message) if message.contains("rows item must be an object"))
        );
    }

    #[test]
    fn pool_connection_timeout_uses_default_and_env_override() {
        let _lock = env_lock();
        let prior = env::var(POOL_CONNECTION_TIMEOUT_ENV).ok();
        let prior_login = env::var(LOGIN_TIMEOUT_ENV).ok();
        env::remove_var(POOL_CONNECTION_TIMEOUT_ENV);
        env::remove_var(LOGIN_TIMEOUT_ENV);
        assert_eq!(
            pool_connection_timeout_ms(),
            (DEFAULT_LOGIN_TIMEOUT_SECS as u64 * 1000) + POOL_CONNECTION_TIMEOUT_SLACK_MS
        );

        env::set_var(POOL_CONNECTION_TIMEOUT_ENV, "45000");
        assert_eq!(pool_connection_timeout_ms(), 45_000);

        env::set_var(POOL_CONNECTION_TIMEOUT_ENV, "0");
        assert_eq!(
            pool_connection_timeout_ms(),
            (DEFAULT_LOGIN_TIMEOUT_SECS as u64 * 1000) + POOL_CONNECTION_TIMEOUT_SLACK_MS
        );

        if let Some(value) = prior {
            env::set_var(POOL_CONNECTION_TIMEOUT_ENV, value);
        } else {
            env::remove_var(POOL_CONNECTION_TIMEOUT_ENV);
        }
        if let Some(value) = prior_login {
            env::set_var(LOGIN_TIMEOUT_ENV, value);
        } else {
            env::remove_var(LOGIN_TIMEOUT_ENV);
        }
    }

    #[test]
    fn stored_procedure_argument_count_must_match_bound_params() {
        assert!(ensure_argument_count("MS SQL Server stored procedure", 2, 2).is_ok());
        assert!(matches!(
            ensure_argument_count("MS SQL Server stored procedure", 2, 1),
            Err(RuntimeError::Validation(_))
        ));
    }
}
