use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use appfw_runtime::{
    provider_error as runtime_provider_error, provider_keys::FrameworkProvider, RuntimeAuditEvent,
    RuntimeAuditQuery, RuntimeError, RuntimeJsonObj,
};
use reqwest::StatusCode;
use serde_json::{Map, Value};
use tokio::time::sleep;
use tracing::{debug, warn};

use crate::{
    audit::{
        insert_statement as audit_insert_statement,
        previous_hash_statement as audit_previous_hash_statement,
        query_statement as audit_query_statement,
    },
    SnowflakeSessionConfig, SnowflakeStatement, SnowflakeStatementBuilder,
};

#[derive(Clone)]
pub struct SnowflakeExecutionClient {
    http: reqwest::Client,
    endpoint: String,
    database: String,
    warehouse: Option<String>,
    role: Option<String>,
    token: String,
    token_type: String,
    statement_timeout_secs: u64,
}

impl SnowflakeExecutionClient {
    pub fn from_session(session: SnowflakeSessionConfig) -> Result<Self, RuntimeError> {
        let http = localstack_resolved_client(&session.endpoint)?
            .no_proxy()
            .build()
            .map_err(|e| RuntimeError::DataAccess(e.to_string()))?;

        Ok(Self {
            http,
            endpoint: session.endpoint,
            database: session.database,
            warehouse: session.warehouse,
            role: session.role,
            token: session.token,
            token_type: session.token_type,
            statement_timeout_secs: session.statement_timeout_secs,
        })
    }

    fn uses_localstack_no_auth(&self) -> bool {
        self.token_type.eq_ignore_ascii_case("LOCALSTACK_NO_AUTH")
    }

    fn apply_auth(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if self.uses_localstack_no_auth() {
            return request;
        }

        request
            .bearer_auth(&self.token)
            .header("X-Snowflake-Authorization-Token-Type", &self.token_type)
    }

    fn uses_localstack_endpoint(&self) -> bool {
        self.endpoint.contains("localhost.localstack.cloud")
    }

    pub async fn health_check(&self) -> Result<(), RuntimeError> {
        self.run_query_statement(SnowflakeStatementBuilder::new().finish("SELECT 1"))
            .await?;
        Ok(())
    }

    pub async fn append_audit_event(&self, event: RuntimeAuditEvent) -> Result<(), RuntimeError> {
        let prev_hash = self.previous_audit_hash(&event).await?;
        let event = event.finalize(prev_hash)?;
        self.run_exec_statement(audit_insert_statement(&event)?)
            .await
    }

    pub async fn query_audit_events(
        &self,
        query: RuntimeAuditQuery,
    ) -> Result<Vec<Value>, RuntimeError> {
        let attempts = audit_chain_read_attempts(query.limit, self.uses_localstack_endpoint());
        let mut rows = Vec::new();
        for attempt in 0..attempts {
            rows = self
                .run_query_statement(audit_query_statement(&query)?)
                .await?;
            if !rows.is_empty() || attempt + 1 == attempts {
                break;
            }
            sleep(audit_chain_retry_delay(attempt)).await;
        }
        Ok(rows.into_iter().map(Value::Object).collect())
    }

    async fn previous_audit_hash(
        &self,
        event: &RuntimeAuditEvent,
    ) -> Result<Option<String>, RuntimeError> {
        let attempts = previous_audit_hash_attempts(event, self.uses_localstack_endpoint());
        let mut prev_hash = None;
        for attempt in 0..attempts {
            prev_hash = self
                .run_query_statement(audit_previous_hash_statement(event)?)
                .await?
                .first()
                .and_then(|row| row.get("event_hash").or_else(|| row.get("EVENT_HASH")))
                .and_then(Value::as_str)
                .map(ToString::to_string);
            if prev_hash.is_some() || attempt + 1 == attempts {
                break;
            }
            sleep(audit_chain_retry_delay(attempt)).await;
        }
        Ok(prev_hash)
    }

    pub async fn run_query_statement(
        &self,
        statement: SnowflakeStatement,
    ) -> Result<Vec<RuntimeJsonObj>, RuntimeError> {
        debug!(sql = %statement.sql(), "executing Snowflake query");
        let result = self.execute_statement(statement).await?;
        result_rows(&result)
    }

    pub async fn run_exec_statement(
        &self,
        statement: SnowflakeStatement,
    ) -> Result<(), RuntimeError> {
        debug!(sql = %statement.sql(), "executing Snowflake statement");
        self.execute_statement(statement).await?;
        Ok(())
    }

    pub async fn call_stored_procedure(
        &self,
        statement: SnowflakeStatement,
    ) -> Result<Vec<RuntimeJsonObj>, RuntimeError> {
        debug!(sql = %statement.sql(), "calling Snowflake stored procedure");
        self.run_query_statement(statement).await
    }

    /// Open a transaction on the current Snowflake session. Subsequent
    /// statements issued through this client run within the transaction until
    /// [`commit_transaction`](Self::commit_transaction) or
    /// [`rollback_transaction`](Self::rollback_transaction) is called.
    ///
    /// The Snowflake SQL API multiplexes statements over a single session
    /// (identified by the bearer token), so `BEGIN`/`COMMIT`/`ROLLBACK` scope
    /// the statements that run between them. Multi-statement / relationship
    /// writes must be wrapped so a partial failure does not leave the row
    /// persisted while its junction rows are missing (or vice versa).
    pub async fn begin_transaction(&self) -> Result<(), RuntimeError> {
        debug!("beginning Snowflake transaction");
        self.run_exec_statement(SnowflakeStatementBuilder::new().finish("BEGIN"))
            .await
    }

    pub async fn commit_transaction(&self) -> Result<(), RuntimeError> {
        debug!("committing Snowflake transaction");
        self.run_exec_statement(SnowflakeStatementBuilder::new().finish("COMMIT"))
            .await
    }

    /// Roll back the current transaction. Errors are logged but not surfaced so
    /// the original failure that triggered the rollback is the one returned to
    /// the caller (fail-closed: the caller still sees the operation as failed).
    pub async fn rollback_transaction(&self) {
        debug!("rolling back Snowflake transaction");
        if let Err(err) = self
            .run_exec_statement(SnowflakeStatementBuilder::new().finish("ROLLBACK"))
            .await
        {
            tracing::warn!(error = %err, "failed to roll back Snowflake transaction");
        }
    }

    async fn execute_statement(
        &self,
        statement: SnowflakeStatement,
    ) -> Result<Value, RuntimeError> {
        let url = format!("{}/api/v2/statements?nullable=true", self.endpoint);
        let sql = statement.sql().to_string();
        let body = statement.body(
            self.statement_timeout_secs,
            &self.database,
            self.warehouse.as_deref(),
            self.role.as_deref(),
        );

        let response = self
            .apply_auth(self.http.post(url).json(&body))
            .send()
            .await
            .map_err(|e| RuntimeError::DataAccess(e.to_string()))?;

        match self.handle_response(response).await {
            Ok(value) => Ok(value),
            Err(err) => {
                warn!(error = %err, sql = %sql, "Snowflake statement failed");
                Err(err)
            }
        }
    }

    async fn handle_response(&self, response: reqwest::Response) -> Result<Value, RuntimeError> {
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| RuntimeError::DataAccess(e.to_string()))?;
        let value: Value = serde_json::from_str(&body).map_err(|e| {
            RuntimeError::DataAccess(format!(
                "invalid Snowflake SQL API response: {}; body: {}",
                e, body
            ))
        })?;

        match status {
            StatusCode::OK => match snowflake_response_error(&value) {
                Some(message) => Err(snowflake_error(message)),
                None => Ok(value),
            },
            StatusCode::ACCEPTED => self.poll_statement(value).await,
            _ => Err(snowflake_error(format!(
                "Snowflake SQL API returned {}: {}",
                status,
                snowflake_error_message(&value)
            ))),
        }
    }

    async fn poll_statement(&self, initial: Value) -> Result<Value, RuntimeError> {
        let status_url = initial
            .get("statementStatusUrl")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                RuntimeError::DataAccess(
                    "Snowflake async response did not include statementStatusUrl".to_string(),
                )
            })?;
        let url = if status_url.starts_with("http") {
            status_url.to_string()
        } else {
            format!("{}{}", self.endpoint, status_url)
        };

        for _ in 0..120 {
            sleep(Duration::from_millis(500)).await;
            let response = self
                .apply_auth(self.http.get(&url))
                .send()
                .await
                .map_err(|e| RuntimeError::DataAccess(e.to_string()))?;
            let status = response.status();
            let body = response
                .text()
                .await
                .map_err(|e| RuntimeError::DataAccess(e.to_string()))?;
            let value: Value = serde_json::from_str(&body).map_err(|e| {
                RuntimeError::DataAccess(format!(
                    "invalid Snowflake SQL API response: {}; body: {}",
                    e, body
                ))
            })?;

            match status {
                StatusCode::OK => {
                    return match snowflake_response_error(&value) {
                        Some(message) => Err(snowflake_error(message)),
                        None => Ok(value),
                    }
                }
                StatusCode::ACCEPTED => continue,
                _ => {
                    return Err(snowflake_error(format!(
                        "Snowflake SQL API returned {} while polling: {}",
                        status,
                        snowflake_error_message(&value)
                    )))
                }
            }
        }

        Err(RuntimeError::DataAccess(
            "Snowflake statement polling timed out".to_string(),
        ))
    }
}

fn localstack_resolved_client(endpoint: &str) -> Result<reqwest::ClientBuilder, RuntimeError> {
    let mut builder = reqwest::Client::builder();
    let url = reqwest::Url::parse(endpoint)
        .map_err(|e| RuntimeError::DataAccess(format!("invalid Snowflake endpoint: {e}")))?;
    let Some(host) = url.host_str() else {
        return Ok(builder);
    };
    if host == "snowflake.localhost.localstack.cloud" {
        let port = url.port_or_known_default().unwrap_or(443);
        builder = builder.resolve(host, SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port));
    }
    Ok(builder)
}

pub fn result_rows(result: &Value) -> Result<Vec<RuntimeJsonObj>, RuntimeError> {
    let data = result
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let row_types = match result
        .get("resultSetMetaData")
        .and_then(|m| m.get("rowType"))
        .and_then(Value::as_array)
    {
        Some(row_types) => row_types,
        None if data.is_empty() => return Ok(Vec::new()),
        None => {
            return Err(RuntimeError::DataAccess(
                "Snowflake response missing resultSetMetaData.rowType".to_string(),
            ))
        }
    };
    let names = row_types
        .iter()
        .map(|r| {
            r.get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string()
        })
        .collect::<Vec<_>>();
    let mut rows = Vec::with_capacity(data.len());
    for row in data {
        let cells = row.as_array().ok_or_else(|| {
            RuntimeError::DataAccess("Snowflake data row must be an array".to_string())
        })?;
        let mut obj = Map::new();
        for (idx, name) in names.iter().enumerate() {
            obj.insert(name.clone(), cells.get(idx).cloned().unwrap_or(Value::Null));
        }
        rows.push(obj);
    }
    Ok(rows)
}

fn snowflake_error(message: impl Into<String>) -> RuntimeError {
    RuntimeError::DataStore(runtime_provider_error::normalize_provider_error(
        FrameworkProvider::Snowflake,
        message.into(),
    ))
}

fn snowflake_error_message(value: &Value) -> String {
    value
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_else(|| value.as_str().unwrap_or("unknown Snowflake error"))
        .to_string()
}

fn snowflake_response_error(value: &Value) -> Option<String> {
    let code = value.get("code").and_then(Value::as_str).unwrap_or("");
    let sql_state = value.get("sqlState").and_then(Value::as_str).unwrap_or("");
    let has_error_code = !code.is_empty() && code != "090001";
    let has_error_state = !sql_state.is_empty() && sql_state != "00000";
    if has_error_code || has_error_state {
        Some(snowflake_error_message(value))
    } else {
        None
    }
}

fn audit_chain_read_attempts(limit: i64, localstack_endpoint: bool) -> usize {
    if limit != 1 {
        1
    } else if localstack_endpoint {
        25
    } else {
        6
    }
}

fn previous_audit_hash_attempts(event: &RuntimeAuditEvent, localstack_endpoint: bool) -> usize {
    if event.record_id.is_some() && event.action != "create" {
        if localstack_endpoint {
            25
        } else {
            6
        }
    } else {
        1
    }
}

fn audit_chain_retry_delay(attempt: usize) -> Duration {
    let _ = attempt;
    Duration::from_millis(100)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn result_rows_uses_metadata_names() {
        let result = json!({
          "resultSetMetaData": {
            "rowType": [
              { "name": "id" },
              { "name": "name" }
            ]
          },
          "data": [["123", "Acme"]]
        });

        let rows = result_rows(&result).expect("rows");

        assert_eq!(rows[0].get("id"), Some(&Value::String("123".to_string())));
        assert_eq!(
            rows[0].get("name"),
            Some(&Value::String("Acme".to_string()))
        );
    }

    #[test]
    fn result_rows_rejects_non_array_rows() {
        let result = json!({
          "resultSetMetaData": {
            "rowType": [{ "name": "id" }]
          },
          "data": [{"id": "123"}]
        });

        let err = result_rows(&result).expect_err("invalid rows");

        assert!(matches!(err, RuntimeError::DataAccess(message) if message.contains("data row")));
    }

    #[test]
    fn result_rows_allows_empty_response_without_metadata() {
        let result = json!({
          "data": []
        });

        let rows = result_rows(&result).expect("empty rows");

        assert!(rows.is_empty());
    }

    #[test]
    fn detects_error_payloads_returned_with_success_http_status() {
        let result = json!({
          "code": "000904",
          "message": "invalid identifier",
          "sqlState": "42000"
        });

        assert_eq!(
            snowflake_response_error(&result).as_deref(),
            Some("invalid identifier")
        );

        let success = json!({
          "code": "090001",
          "message": "Statement executed successfully.",
          "sqlState": "00000"
        });
        assert!(snowflake_response_error(&success).is_none());
    }

    #[test]
    fn normalizes_snowflake_errors() {
        let err = snowflake_error("Duplicate value violates unique constraint");

        assert_eq!(err.to_string(), "duplicate key: record already exists");
    }

    #[test]
    fn retries_only_audit_chain_continuation_reads() {
        assert_eq!(audit_chain_read_attempts(1, false), 6);
        assert_eq!(audit_chain_read_attempts(1, true), 25);
        assert_eq!(audit_chain_read_attempts(2, true), 1);

        let mut event = RuntimeAuditEvent {
            audit_id: "audit-1".to_string(),
            occurred_at: chrono::Utc::now(),
            tenant_id: Some("tenant-1".to_string()),
            actor_user_name: "casey".to_string(),
            actor_roles: Vec::new(),
            action: "update".to_string(),
            outcome: "denied".to_string(),
            schema_name: "crm".to_string(),
            entity_name: "Account".to_string(),
            table_name: "accounts".to_string(),
            audit_table_name: "accounts_audit".to_string(),
            record_id: Some("account-1".to_string()),
            before_json: None,
            after_json: None,
            diff_json: serde_json::json!({}),
            policy_json: None,
            redactions_json: serde_json::json!({}),
            chain_scope: "crm.Account:tenant-1:account-1".to_string(),
            prev_hash: None,
            event_hash: String::new(),
            signature: None,
        };
        assert_eq!(previous_audit_hash_attempts(&event, false), 6);
        assert_eq!(previous_audit_hash_attempts(&event, true), 25);

        event.action = "create".to_string();
        assert_eq!(previous_audit_hash_attempts(&event, true), 1);
    }
}
