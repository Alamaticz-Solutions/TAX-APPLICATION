use appfw_runtime::{model_metadata::RuntimeDataType, RuntimeError, RuntimeJsonObj};
use serde_json::Value;
use tokio::task;

use crate::{
    auth::{EntraTokenProvider, MssqlAuthConfig},
    odbc::{connect_with_access_token, odbc_sql_and_params, resolve_odbc_driver_name},
    MssqlConnectionConfig, SqlParam,
};

#[derive(Clone)]
pub struct FabricOdbcExecutionClient {
    connection_string: String,
    token_provider: EntraTokenProvider,
}

#[derive(Debug)]
pub struct FabricOdbcJsonRows {
    pub total: i64,
    pub items: Vec<RuntimeJsonObj>,
}

#[derive(Debug)]
pub struct FabricOdbcAggregateRows {
    pub total: i64,
    pub items: Vec<Value>,
}

#[derive(Clone, Debug)]
pub struct FabricOdbcColumn {
    pub name: String,
    pub data_type: RuntimeDataType,
}

impl FabricOdbcExecutionClient {
    pub async fn from_connection_config(
        connection: &MssqlConnectionConfig,
    ) -> Result<Self, RuntimeError> {
        let token_provider = match &connection.auth {
            MssqlAuthConfig::EntraClientCredentials { .. } => {
                EntraTokenProvider::from_auth(&connection.auth)?
            }
            MssqlAuthConfig::EntraAccessToken { .. } => {
                return Err(RuntimeError::DataAccess(
                    "FabricSqlAnalytics requires Entra client-credentials auth; static access-token auth is not supported"
                        .to_string(),
                ));
            }
            MssqlAuthConfig::SqlPassword { .. } => {
                return Err(RuntimeError::DataAccess(
                    "FabricSqlAnalytics requires Microsoft Entra authentication; SQL password auth is not supported"
                        .to_string(),
                ));
            }
            MssqlAuthConfig::Ntlm { .. } => {
                return Err(RuntimeError::DataAccess(
                    "FabricSqlAnalytics requires Microsoft Entra authentication; ntlm auth is not supported"
                        .to_string(),
                ));
            }
        };

        Ok(Self {
            connection_string: fabric_connection_string(connection),
            token_provider,
        })
    }

    pub async fn health_check(&self) -> Result<(), RuntimeError> {
        self.run_scalar_query("SELECT 1 AS [ok]", Vec::new())
            .await
            .map(|_| ())
    }

    pub async fn query_json_rows(
        &self,
        sql: &str,
        params: Vec<SqlParam>,
    ) -> Result<Option<FabricOdbcJsonRows>, RuntimeError> {
        let row = self.query_count_and_json(sql, params).await?;
        row.map(json_rows_from_odbc_row).transpose()
    }

    pub async fn aggregate_json_rows(
        &self,
        sql: &str,
        params: Vec<SqlParam>,
    ) -> Result<Option<FabricOdbcAggregateRows>, RuntimeError> {
        let row = self.query_count_and_json(sql, params).await?;
        row.map(aggregate_rows_from_odbc_row).transpose()
    }

    pub async fn query_flat_json_rows(
        &self,
        count_sql: &str,
        count_params: Vec<SqlParam>,
        rows_sql: &str,
        rows_params: Vec<SqlParam>,
        columns: Vec<FabricOdbcColumn>,
    ) -> Result<FabricOdbcJsonRows, RuntimeError> {
        let connection_string = self.connection_string.clone();
        let access_token = self.token_provider.access_token().await?;
        let count_sql = count_sql.to_string();
        let rows_sql = rows_sql.to_string();
        task::spawn_blocking(move || {
            let mut conn = connect_with_access_token(&connection_string, &access_token)?;

            let (count_sql, count_params) = odbc_sql_and_params(count_sql, count_params)?;
            let mut count_stmt = conn.prepare(&count_sql)?;
            count_stmt.bind_params(count_params)?;
            count_stmt.execute()?;
            let total = if count_stmt.fetch()? {
                count_stmt
                    .get_string(1)?
                    .and_then(|value| value.parse::<i64>().ok())
                    .unwrap_or(0)
            } else {
                0
            };
            drop(count_stmt);

            let (rows_sql, rows_params) = odbc_sql_and_params(rows_sql, rows_params)?;
            let mut rows_stmt = conn.prepare(&rows_sql)?;
            rows_stmt.bind_params(rows_params)?;
            rows_stmt.execute()?;

            let mut items = Vec::new();
            while rows_stmt.fetch()? {
                let mut row = RuntimeJsonObj::new();
                for (index, column) in columns.iter().enumerate() {
                    let raw = rows_stmt.get_string((index + 1) as u16)?;
                    row.insert(
                        column.name.clone(),
                        typed_value(raw, column.data_type, &column.name)?,
                    );
                }
                items.push(row);
            }

            Ok(FabricOdbcJsonRows { total, items })
        })
        .await
        .map_err(|err| RuntimeError::DataAccess(format!("Fabric ODBC worker failed: {err}")))?
    }

    async fn run_scalar_query(&self, sql: &str, params: Vec<SqlParam>) -> Result<(), RuntimeError> {
        let connection_string = self.connection_string.clone();
        let access_token = self.token_provider.access_token().await?;
        let sql = sql.to_string();
        task::spawn_blocking(move || {
            let (sql, params) = odbc_sql_and_params(sql, params)?;
            let mut conn = connect_with_access_token(&connection_string, &access_token)?;
            let mut stmt = conn.prepare(&sql)?;
            stmt.bind_params(params)?;
            stmt.execute()?;
            Ok(())
        })
        .await
        .map_err(|err| RuntimeError::DataAccess(format!("Fabric ODBC worker failed: {err}")))?
    }

    async fn query_count_and_json(
        &self,
        sql: &str,
        params: Vec<SqlParam>,
    ) -> Result<Option<(i64, String)>, RuntimeError> {
        let connection_string = self.connection_string.clone();
        let access_token = self.token_provider.access_token().await?;
        let sql = sql.to_string();
        task::spawn_blocking(move || {
            let (sql, params) = odbc_sql_and_params(sql, params)?;
            let mut conn = connect_with_access_token(&connection_string, &access_token)?;
            let mut stmt = conn.prepare(&sql)?;
            stmt.bind_params(params)?;
            stmt.execute()?;
            if !stmt.fetch()? {
                return Ok(None);
            }
            let total = stmt
                .get_string(1)?
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(0);
            let rows_json = stmt.get_string(2)?.unwrap_or_else(|| "[]".to_string());
            Ok(Some((total, rows_json)))
        })
        .await
        .map_err(|err| RuntimeError::DataAccess(format!("Fabric ODBC worker failed: {err}")))?
    }
}

fn json_rows_from_odbc_row(row: (i64, String)) -> Result<FabricOdbcJsonRows, RuntimeError> {
    let (total, rows_json) = row;
    let items = parse_json_objects(&rows_json, "Fabric ODBC JSON rows payload")?;
    Ok(FabricOdbcJsonRows { total, items })
}

fn aggregate_rows_from_odbc_row(
    row: (i64, String),
) -> Result<FabricOdbcAggregateRows, RuntimeError> {
    let (total, rows_json) = row;
    let items = parse_json_values(&rows_json, "Fabric ODBC aggregate JSON rows payload")?;
    Ok(FabricOdbcAggregateRows { total, items })
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

fn typed_value(
    raw: Option<String>,
    data_type: RuntimeDataType,
    column_name: &str,
) -> Result<Value, RuntimeError> {
    let Some(raw) = raw else {
        return Ok(Value::Null);
    };
    let trimmed = raw.trim();
    match data_type {
        RuntimeDataType::Boolean => Ok(Value::Bool(matches!(
            trimmed.to_ascii_lowercase().as_str(),
            "1" | "true"
        ))),
        RuntimeDataType::Int8
        | RuntimeDataType::Int16
        | RuntimeDataType::Int32
        | RuntimeDataType::Int64 => {
            let value = trimmed.parse::<i64>().map_err(|err| {
                RuntimeError::DataAccess(format!(
                    "Fabric ODBC column `{column_name}` returned non-integer value `{raw}`: {err}"
                ))
            })?;
            Ok(Value::Number(value.into()))
        }
        RuntimeDataType::Float32 | RuntimeDataType::Float64 => {
            let value = trimmed.parse::<f64>().map_err(|err| {
                RuntimeError::DataAccess(format!(
                    "Fabric ODBC column `{column_name}` returned non-float value `{raw}`: {err}"
                ))
            })?;
            let number = serde_json::Number::from_f64(value).ok_or_else(|| {
                RuntimeError::DataAccess(format!(
                    "Fabric ODBC column `{column_name}` returned non-finite float `{raw}`"
                ))
            })?;
            Ok(Value::Number(number))
        }
        RuntimeDataType::Json | RuntimeDataType::Object => serde_json::from_str::<Value>(&raw)
            .map_err(|err| {
                RuntimeError::DataAccess(format!(
                    "Fabric ODBC column `{column_name}` returned invalid JSON: {err}"
                ))
            }),
        _ => Ok(Value::String(raw)),
    }
}

fn fabric_connection_string(connection: &MssqlConnectionConfig) -> String {
    let driver = resolve_odbc_driver_name();
    format!(
        "Driver={{{driver}}};Server=tcp:{host},{port};Database={database};Encrypt=yes;TrustServerCertificate=no;Connection Timeout=30;MARS_Connection=no;",
        driver = driver.replace('}', "}}"),
        host = connection.host,
        port = connection.port,
        database = connection.database.replace(';', ""),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::odbc::MS_ODBC_DRIVER_18;

    #[test]
    fn fabric_connection_string_uses_driver_18_defaults() {
        let connection = MssqlConnectionConfig::new_entra_client_credentials(
            "fabric-host.example.invalid",
            "1433",
            "fabric_database",
            "tenant-id",
            "client-id",
            "client-secret",
            None,
        );
        let conn = fabric_connection_string(&connection);
        assert!(conn.contains(&format!("Driver={{{}}}", MS_ODBC_DRIVER_18)));
        assert!(conn.contains("Server=tcp:fabric-host.example.invalid,1433"));
        assert!(conn.contains("Database=fabric_database"));
        assert!(conn.contains("Encrypt=yes"));
        assert!(conn.contains("MARS_Connection=no"));
    }

    #[test]
    fn fabric_client_accepts_client_credentials_without_eager_token_fetch() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let connection = MssqlConnectionConfig::new_entra_client_credentials(
            "fabric-host.example.invalid",
            "1433",
            "fabric_database",
            "tenant-id",
            "client-id",
            "client-secret",
            None,
        );

        let client = runtime.block_on(FabricOdbcExecutionClient::from_connection_config(
            &connection,
        ));

        assert!(client.is_ok());
    }

    #[test]
    fn fabric_client_rejects_static_access_token_auth() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let connection = MssqlConnectionConfig::new_entra_access_token(
            "fabric-host.example.invalid",
            "1433",
            "fabric_database",
            "token",
        );

        let result = runtime.block_on(FabricOdbcExecutionClient::from_connection_config(
            &connection,
        ));
        let err = match result {
            Ok(_) => panic!("Fabric should reject static access-token auth"),
            Err(err) => err,
        };

        assert!(
            matches!(err, RuntimeError::DataAccess(message) if message.contains("client-credentials auth"))
        );
    }
}
