use std::{
    env, fs,
    io::{Read, Result},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    time::Duration,
};

use reqwest::StatusCode;
use serde_json::{json, Value};
use tokio::time::sleep;

use appfw_runtime::connection_security::{self, Provider};

use crate::{
    console,
    data_source::{DataSource, DataSourceEnvironment, Schema},
    error,
    loader::{get_data_source_schemas, get_schema_path},
    migration_options,
};

pub struct Snowflake {
    http: reqwest::Client,
    endpoint: String,
    database: String,
    warehouse: Option<String>,
    role: Option<String>,
    token: Option<String>,
    token_type: String,
    data_source: DataSource,
}

impl Snowflake {
    pub fn new(data_source: &DataSource) -> Result<Self> {
        let env = data_source
            .environments
            .first()
            .ok_or_else(|| {
                error::config(format!(
                    "data source `{}` has no selected environment",
                    data_source.name
                ))
            })?
            .to_owned();
        let endpoint = endpoint(&env)?;
        let http = localstack_resolved_client(&endpoint)?
            .no_proxy()
            .build()
            .map_err(|e| error::external(format!("could not create Snowflake HTTP client: {e}")))?;
        Ok(Self {
            http,
            endpoint,
            database: env.db_name.clone(),
            warehouse: optional_env("SNOWFLAKE_WAREHOUSE"),
            role: optional_env("SNOWFLAKE_ROLE"),
            token: env.service_account_password.clone(),
            token_type: env::var("SNOWFLAKE_AUTH_TOKEN_TYPE")
                .unwrap_or_else(|_| "PROGRAMMATIC_ACCESS_TOKEN".to_string()),
            data_source: data_source.clone(),
        })
    }

    pub async fn migrate(&self) -> Result<()> {
        console::step("Snowflake migration");

        let schemas = get_data_source_schemas(&self.data_source.name)?;
        if schemas.is_empty() {
            console::skip(
                &PathBuf::from(&self.data_source.name),
                "no schemas use this data source",
            );
            return Ok(());
        }
        self.ensure_token()?;

        self.exec_without_database(format!(
            "CREATE DATABASE IF NOT EXISTS {}",
            quote_ident(&self.database)
        ))
        .await?;
        for schema in schemas {
            console::item("schema", &schema.name);
            self.ensure_schema_exists(&schema).await?;
            self.migrate_schema(&schema).await?;
        }

        Ok(())
    }

    async fn ensure_schema_exists(&self, schema: &Schema) -> Result<()> {
        self.exec(format!(
            "CREATE SCHEMA IF NOT EXISTS {}",
            quote_ident(&schema.name)
        ))
        .await
    }

    async fn migrate_schema(&self, schema: &Schema) -> Result<()> {
        let schema_path = get_schema_path(schema)?;
        self.exec_schema_sql(&schema_path, "tables.snowflake.sql")
            .await?;
        if migration_options::skip_seed() {
            console::skip(
                &schema_path.join("seed.snowflake.sql"),
                "seed execution skipped by APPFW_MIGRATE_SKIP_SEED",
            );
        } else {
            match self
                .exec_schema_sql(&schema_path, "seed.snowflake.sql")
                .await
            {
                Ok(_) => console::verbose(format!("seed data executed: {}", schema.name)),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => return Err(err),
            }
        }
        Ok(())
    }

    async fn exec_schema_sql(&self, schema_dir: &PathBuf, file_name: &str) -> Result<()> {
        let sql_file = schema_dir.join(file_name);
        if !sql_file.exists() {
            console::skip(&sql_file, "file not found");
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {:?}", sql_file),
            ));
        }

        console::step_path("execute SQL", &sql_file);
        let sql = self.get_file_sql(&sql_file)?;
        for stmt in split_sql_statements(&sql) {
            if !stmt.trim().is_empty() {
                self.exec(stmt).await?;
            }
        }
        Ok(())
    }

    fn get_file_sql(&self, sql_file: &PathBuf) -> Result<String> {
        console::verbose_path("read SQL", sql_file);
        let mut f = fs::File::open(sql_file)?;
        let mut sql = String::new();
        f.read_to_string(&mut sql)?;
        Ok(sql)
    }

    async fn exec_without_database(&self, sql: String) -> Result<()> {
        self.exec_with_database(sql, false).await
    }

    async fn exec(&self, sql: String) -> Result<()> {
        self.exec_with_database(sql, true).await
    }

    async fn exec_with_database(&self, sql: String, include_database: bool) -> Result<()> {
        console::verbose("snowflake exec");
        console::verbose(&sql);
        let token = self.ensure_token()?;
        let mut body = json!({
          "statement": sql,
          "timeout": 60,
        });
        if include_database {
            body["database"] = Value::String(self.database.clone());
        }
        if let Some(warehouse) = &self.warehouse {
            body["warehouse"] = Value::String(warehouse.clone());
        }
        if let Some(role) = &self.role {
            body["role"] = Value::String(role.clone());
        }

        let response = self
            .apply_auth(
                self.http
                    .post(format!("{}/api/v2/statements?nullable=true", self.endpoint))
                    .json(&body),
                token,
            )
            .send()
            .await
            .map_err(io_other)?;

        self.handle_response(response).await.map(|_| ())
    }

    async fn handle_response(&self, response: reqwest::Response) -> Result<Value> {
        let status = response.status();
        let body = response.text().await.map_err(io_other)?;
        let value: Value = serde_json::from_str(&body).map_err(|e| {
            io_other(format!(
                "invalid Snowflake SQL API response: {}; body: {}",
                e, body
            ))
        })?;
        match status {
            StatusCode::OK => Ok(value),
            StatusCode::ACCEPTED => self.poll(value).await,
            _ => Err(io_other(format!(
                "Snowflake SQL API returned {}: {}",
                status,
                error_message(&value)
            ))),
        }
    }

    async fn poll(&self, initial: Value) -> Result<Value> {
        let status_url = initial
            .get("statementStatusUrl")
            .and_then(Value::as_str)
            .ok_or_else(|| io_other("Snowflake async response missing statementStatusUrl"))?;
        let url = if status_url.starts_with("http") {
            status_url.to_string()
        } else {
            format!("{}{}", self.endpoint, status_url)
        };
        let token = self.ensure_token()?;

        for _ in 0..120 {
            sleep(Duration::from_millis(500)).await;
            let response = self
                .apply_auth(self.http.get(&url), token)
                .send()
                .await
                .map_err(io_other)?;
            let status = response.status();
            let body = response.text().await.map_err(io_other)?;
            let value: Value = serde_json::from_str(&body).map_err(|e| {
                io_other(format!(
                    "invalid Snowflake SQL API response: {}; body: {}",
                    e, body
                ))
            })?;
            match status {
                StatusCode::OK => return Ok(value),
                StatusCode::ACCEPTED => continue,
                _ => {
                    return Err(io_other(format!(
                        "Snowflake SQL API returned {} while polling: {}",
                        status,
                        error_message(&value)
                    )))
                }
            }
        }
        Err(io_other("Snowflake statement polling timed out"))
    }

    fn ensure_token(&self) -> Result<&str> {
        self.token.as_deref().filter(|v| !v.trim().is_empty()).ok_or_else(|| {
      io_other("Snowflake migration requires SNOWFLAKE_ACCESS_TOKEN, SNOWFLAKE_OAUTH_TOKEN, SNOWFLAKE_JWT, or SNOWFLAKE_SERVICE_ACCOUNT_PASS")
    })
    }

    fn uses_localstack_no_auth(&self) -> bool {
        self.token_type.eq_ignore_ascii_case("LOCALSTACK_NO_AUTH")
    }

    fn apply_auth(&self, request: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
        if self.uses_localstack_no_auth() {
            return request;
        }

        request
            .bearer_auth(token)
            .header("X-Snowflake-Authorization-Token-Type", &self.token_type)
    }
}

fn localstack_resolved_client(endpoint: &str) -> Result<reqwest::ClientBuilder> {
    let mut builder = reqwest::Client::builder();
    let url = reqwest::Url::parse(endpoint)
        .map_err(|e| error::external(format!("invalid Snowflake endpoint: {e}")))?;
    let Some(host) = url.host_str() else {
        return Ok(builder);
    };
    if host == "snowflake.localhost.localstack.cloud" {
        let port = url.port_or_known_default().unwrap_or(443);
        builder = builder.resolve(host, SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port));
    }
    Ok(builder)
}

fn endpoint(env: &DataSourceEnvironment) -> Result<String> {
    let host = optional_env("SNOWFLAKE_HOST").unwrap_or_else(|| env.db_host.clone());
    connection_security::validate(
        Provider::Snowflake,
        &env.name,
        &env.security_profile,
        &env.tls_mode,
        &host,
    )
    .map_err(|e| error::external(format!("invalid Snowflake connection security: {e}")))?;
    Ok(snowflake_endpoint(&host, &env.db_port))
}

fn snowflake_endpoint(host: &str, port: &str) -> String {
    let host = host.trim().trim_end_matches('/');
    if host.starts_with("http://") || host.starts_with("https://") {
        if endpoint_has_explicit_port(host) || is_default_https_port(port) {
            host.to_string()
        } else {
            format!("{}:{}", host, port.trim())
        }
    } else if endpoint_has_explicit_port(host) || is_default_https_port(port) {
        format!("https://{}", host)
    } else {
        format!("https://{}:{}", host, port.trim())
    }
}

fn endpoint_has_explicit_port(host: &str) -> bool {
    let host = host
        .strip_prefix("http://")
        .or_else(|| host.strip_prefix("https://"))
        .unwrap_or(host);
    host.rsplit_once(':')
        .map(|(_, port)| !port.is_empty() && port.chars().all(|ch| ch.is_ascii_digit()))
        .unwrap_or(false)
}

fn is_default_https_port(port: &str) -> bool {
    let port = port.trim();
    port.is_empty() || port == "443"
}

fn optional_env(name: &str) -> Option<String> {
    env::var(name).ok().filter(|v| !v.trim().is_empty())
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn split_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut chars = sql.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\'' {
            current.push(ch);
            if in_single && chars.peek() == Some(&'\'') {
                if let Some(escaped_quote) = chars.next() {
                    current.push(escaped_quote);
                }
            } else {
                in_single = !in_single;
            }
        } else if ch == ';' && !in_single {
            statements.push(std::mem::take(&mut current));
        } else {
            current.push(ch);
        }
    }
    if !current.trim().is_empty() {
        statements.push(current);
    }
    statements
}

fn error_message(value: &Value) -> String {
    value
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("unknown Snowflake error")
        .to_string()
}

fn io_other<E: ToString>(error: E) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snowflake_endpoint_uses_https_default_for_hosted_accounts() {
        assert_eq!(
            snowflake_endpoint("account_identifier.snowflakecomputing.com", "443"),
            "https://account_identifier.snowflakecomputing.com"
        );
    }

    #[test]
    fn snowflake_endpoint_preserves_explicit_scheme_and_port() {
        assert_eq!(
            snowflake_endpoint("http://snowflake.localhost.localstack.cloud:4566/", "443"),
            "http://snowflake.localhost.localstack.cloud:4566"
        );
    }

    #[test]
    fn snowflake_endpoint_adds_configured_non_default_port() {
        assert_eq!(
            snowflake_endpoint("snowflake.localhost.localstack.cloud", "4566"),
            "https://snowflake.localhost.localstack.cloud:4566"
        );
    }
}
