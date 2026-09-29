use bytes::Bytes;
use deadpool_postgres::{Client, Config, Pool, Runtime};
use futures_util::{pin_mut, SinkExt};
use std::{
    env,
    fs::{self},
    io::Result,
    path::PathBuf,
};
use tokio_postgres::NoTls;

use appfw_runtime::connection_security::{self, Provider};

use crate::{
    bulk::{BackfillCheckpoint, BulkRows},
    console,
    data_source::{DataSource, DataSourceEnvironment},
    error,
    loader::{get_data_source_schemas, get_schema_path},
    migration_options,
    migrations::drift::ActualColumn,
};

use super::data_source::Schema;

pub struct Postgres {
    default_pool: Pool,
    db_pool: Pool,
    db_name: String,
    data_source: DataSource,
}

impl Postgres {
    pub fn new(data_source: &DataSource) -> Result<Self> {
        let data_source_env = data_source
            .environments
            .first()
            .ok_or_else(|| {
                error::config(format!(
                    "data source `{}` has no selected environment",
                    data_source.name
                ))
            })?
            .to_owned();
        let default_pool = Self::get_pool(&data_source_env, true)?;
        let db_pool = Self::get_pool(&data_source_env, false)?;
        let db_name = data_source_env.db_name.clone();
        let ds = data_source.clone();
        Ok(Self {
            default_pool,
            db_pool,
            db_name,
            data_source: ds,
        })
    }

    fn get_pool(data_source_env: &DataSourceEnvironment, is_default: bool) -> Result<Pool> {
        ensure_supported_postgres_security(data_source_env)?;

        let mut pg_config = Config::new();
        pg_config.host = Some(data_source_env.db_host.to_string());
        pg_config.port = Some(parse_port(&data_source_env.db_port, "PostgreSQL")?);
        if is_default {
            pg_config.dbname = Some("postgres".to_string());
        } else {
            pg_config.dbname = Some(data_source_env.db_name.to_string());
        }
        pg_config.user = data_source_env.service_account_name.clone();
        pg_config.password = data_source_env.service_account_password.clone();

        pg_config
            .create_pool(Some(Runtime::Tokio1), NoTls)
            .map_err(|e| error::external(format!("could not create PostgreSQL pool: {e}")))
    }

    async fn get_client(&self, is_default: bool) -> Result<Client> {
        let pool = if is_default {
            &self.default_pool
        } else {
            &self.db_pool
        };
        pool.get()
            .await
            .map_err(|e| error::external(format!("could not get PostgreSQL client: {e}")))
    }

    pub async fn migrate(&self) -> Result<()> {
        console::step("Postgres migration");

        self.ensure_db_exists().await?;

        // The system schema lives in-memory in the backend (loaded from YAML config)
        // and is never persisted to a database. Only schemas in `_pkg/schemas/` are
        // migrated, and app_gen does not publish the system schema there.
        let schemas = get_data_source_schemas(&self.data_source.name)?;

        for schema in schemas {
            console::item("schema", &schema.name);
            self.ensure_schema_exists(&schema).await?;
            self.migrate_schema(&schema).await?;
        }

        Ok(())
    }

    pub async fn ensure_ready(&self) -> Result<()> {
        self.ensure_db_exists().await
    }

    pub async fn migration_registry_exists(&self) -> Result<bool> {
        if !self.database_exists().await? {
            return Ok(false);
        }

        let query = "SELECT TRUE WHERE to_regclass('app_meta.schema_migrations') IS NOT NULL";
        Ok(self.db_query(false, query.to_string()).await?.is_some())
    }

    pub async fn exec_sql(&self, sql: &str) -> Result<()> {
        self.db_exec(false, sql.to_string()).await.map(|_| ())
    }

    pub async fn query_json(&self, sql: &str) -> Result<Option<serde_json::Value>> {
        self.db_query(false, sql.to_string()).await
    }

    pub async fn query_value_json(&self, sql: &str) -> Result<Option<serde_json::Value>> {
        let query_res = self
            .get_client(false)
            .await?
            .query_opt(sql, &[])
            .await
            .map_err(|e| error::external(format!("PostgreSQL query failed: {e}")))?;

        Ok(match query_res {
            Some(row) => row.get(0),
            None => None,
        })
    }

    pub async fn schema_columns(&self, schema: &str) -> Result<Vec<ActualColumn>> {
        let rows = self
            .get_client(false)
            .await?
            .query(
                r#"
SELECT
    table_schema AS schema_name,
    table_name,
    column_name,
    data_type
FROM information_schema.columns
WHERE table_schema = $1
ORDER BY table_schema, table_name, ordinal_position
"#,
                &[&schema],
            )
            .await
            .map_err(|e| error::external(format!("PostgreSQL schema inspection failed: {e}")))?;

        Ok(rows
            .into_iter()
            .map(|row| ActualColumn {
                schema: row.get("schema_name"),
                table: row.get("table_name"),
                column: row.get("column_name"),
                data_type: row.get("data_type"),
            })
            .collect())
    }

    #[allow(dead_code)]
    pub async fn copy_insert_rows(&self, bulk: &BulkRows) -> Result<u64> {
        if bulk.is_empty() {
            return Ok(0);
        }
        validate_bulk_rows(bulk)?;

        let columns = bulk
            .columns
            .iter()
            .map(|column| pg_identifier(column))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "COPY {}.{} ({columns}) FROM STDIN WITH (FORMAT csv, NULL '\\N')",
            pg_identifier(&bulk.schema),
            pg_identifier(&bulk.table),
        );

        let client = self.get_client(false).await?;
        let sink = client
            .copy_in(&sql)
            .await
            .map_err(|e| error::external(format!("PostgreSQL COPY setup failed: {e}")))?;
        pin_mut!(sink);

        for row in &bulk.rows {
            let mut line = row
                .iter()
                .map(pg_copy_csv_cell)
                .collect::<Vec<_>>()
                .join(",");
            line.push('\n');
            sink.as_mut()
                .send(Bytes::from(line))
                .await
                .map_err(|e| error::external(format!("PostgreSQL COPY stream failed: {e}")))?;
        }

        sink.as_mut()
            .finish()
            .await
            .map_err(|e| error::external(format!("PostgreSQL COPY finish failed: {e}")))
    }

    #[allow(dead_code)]
    pub async fn ensure_backfill_registry(&self) -> Result<()> {
        self.exec_sql(
            r#"
CREATE SCHEMA IF NOT EXISTS app_meta;

CREATE TABLE IF NOT EXISTS app_meta.backfill_checkpoints (
    job_name text NOT NULL,
    chunk_key text NOT NULL,
    last_value text NULL,
    rows_processed bigint NOT NULL DEFAULT 0,
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (job_name, chunk_key)
);
"#,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn backfill_checkpoint(
        &self,
        job_name: &str,
        chunk_key: &str,
    ) -> Result<Option<BackfillCheckpoint>> {
        self.ensure_backfill_registry().await?;
        let sql = format!(
            r#"
SELECT job_name, chunk_key, last_value, rows_processed
FROM app_meta.backfill_checkpoints
WHERE job_name = '{}' AND chunk_key = '{}'
"#,
            pg_string_literal(job_name),
            pg_string_literal(chunk_key)
        );
        let Some(value) = self.query_json(&sql).await? else {
            return Ok(None);
        };

        Ok(Some(BackfillCheckpoint {
            job_name: value["job_name"].as_str().unwrap_or_default().to_string(),
            chunk_key: value["chunk_key"].as_str().unwrap_or_default().to_string(),
            last_value: value["last_value"].as_str().map(str::to_string),
            rows_processed: value["rows_processed"].as_i64().unwrap_or_default(),
        }))
    }

    #[allow(dead_code)]
    pub async fn record_backfill_checkpoint(&self, checkpoint: &BackfillCheckpoint) -> Result<()> {
        self.ensure_backfill_registry().await?;
        let last_value = checkpoint
            .last_value
            .as_deref()
            .map(|value| format!("'{}'", pg_string_literal(value)))
            .unwrap_or_else(|| "NULL".to_string());
        let sql = format!(
            r#"
INSERT INTO app_meta.backfill_checkpoints
    (job_name, chunk_key, last_value, rows_processed)
VALUES
    ('{job_name}', '{chunk_key}', {last_value}, {rows_processed})
ON CONFLICT (job_name, chunk_key) DO UPDATE SET
    last_value = EXCLUDED.last_value,
    rows_processed = EXCLUDED.rows_processed,
    updated_at = now();
"#,
            job_name = pg_string_literal(&checkpoint.job_name),
            chunk_key = pg_string_literal(&checkpoint.chunk_key),
            rows_processed = checkpoint.rows_processed,
        );
        self.exec_sql(&sql).await
    }

    async fn ensure_db_exists(&self) -> std::result::Result<(), std::io::Error> {
        console::step(format!("ensure database: {}", self.db_name));

        if self.database_exists().await? {
            console::verbose(format!("database exists: {}", self.db_name));
            return Ok(());
        }

        // TODO: move into sql file from app_gen template
        let sql = format!(
            "
CREATE DATABASE {}
WITH
    ENCODING = 'UTF8'
    OWNER = postgres
    CONNECTION LIMIT = 100;
",
            pg_identifier(&self.db_name)
        );
        console::step(format!("create database: {}", self.db_name));
        self.db_exec(true, sql).await?;

        Ok(())
    }

    pub async fn database_exists(&self) -> Result<bool> {
        let query = format!(
            "SELECT TRUE FROM pg_database WHERE datname = '{}'",
            pg_string_literal(&self.db_name)
        );
        Ok(self.db_query(true, query).await?.is_some())
    }

    async fn ensure_schema_exists(
        &self,
        schema: &Schema,
    ) -> std::result::Result<(), std::io::Error> {
        console::step(format!("ensure schema: {}", schema.name));

        let sql = format!(
            "CREATE SCHEMA IF NOT EXISTS {};",
            pg_identifier(&schema.name)
        );
        self.db_exec(false, sql).await?;

        Ok(())
    }

    async fn migrate_schema(&self, schema: &Schema) -> Result<()> {
        let schema_path = get_schema_path(schema)?;

        // DDL (PostgreSQL dialect)
        self.exec_schema_sql(&schema_path, "tables.pg.sql").await?;

        // Logger tables
        if let Ok(_) = self.exec_schema_sql(&schema_path, "logs.sql").await {
            console::verbose(format!("logger schema executed: {}", schema.name));
        }

        // Seed data (optional — only runs if seed.pg.sql was generated for this schema)
        if migration_options::skip_seed() {
            console::skip(
                &schema_path.join("seed.pg.sql"),
                "seed execution skipped by APPFW_MIGRATE_SKIP_SEED",
            );
        } else {
            match self.exec_schema_sql(&schema_path, "seed.pg.sql").await {
                Ok(_) => console::verbose(format!("seed data executed: {}", schema.name)),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => return Err(err),
            }
        }

        Ok(())
    }

    async fn exec_schema_sql(&self, schema_dir: &PathBuf, file_name: &str) -> Result<()> {
        let sql_file = &schema_dir.join(file_name);

        // Check if the file exists
        if !sql_file.exists() {
            console::skip(sql_file, "file not found");
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {:?}", sql_file),
            ));
        }

        console::step_path("execute SQL", sql_file);
        let sql = self.get_file_sql(sql_file)?;

        self.db_exec(false, sql).await?;

        Ok(())
    }

    #[allow(dead_code)]
    async fn update_apps(&self) -> Result<()> {
        console::step("update apps");
        let curr_dir = env::current_dir()?;
        let apps_dir = curr_dir.join("_pkg/app");
        for dir_entry in fs::read_dir(apps_dir)? {
            let dir_path = dir_entry?.path();
            if dir_path.is_file() {
                self.update_app(&dir_path).await?;
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    async fn update_app(&self, app_sql_file: &PathBuf) -> Result<()> {
        console::step_path("update app", app_sql_file);

        let sql = self.get_file_sql(app_sql_file)?;

        self.db_exec(false, sql).await?;

        Ok(())
    }

    fn get_file_sql(&self, sql_file: &PathBuf) -> Result<String> {
        console::verbose_path("read SQL", sql_file);

        fs::read_to_string(sql_file).map_err(|e| {
            error::missing_config(format!(
                "could not read SQL file `{}`: {e}",
                sql_file.display()
            ))
        })
    }
    // /Users/wayne.kempf/projects/app-framework/database/_pkg/schemas/scheduling/tables.sql
    // /Users/wayne.kempf/projects/app-framework/database/_pkg/scheduling/tables.sql

    async fn db_query(&self, is_default: bool, sql: String) -> Result<Option<serde_json::Value>> {
        console::verbose("db_query");
        console::verbose(&sql);

        let outer_sql = format!("select row_to_json(t) from ({}) t", sql);

        let query_res = self
            .get_client(is_default)
            .await?
            .query_opt(&outer_sql, &[])
            .await
            .map_err(|e| error::external(format!("PostgreSQL query failed: {e}")))?;

        let res: Option<serde_json::Value> = match query_res {
            Some(row) => row.get(0),
            None => None,
        };

        Ok(res)
    }

    async fn db_exec(&self, is_default: bool, sql: String) -> Result<String> {
        console::verbose("db_exec");
        console::verbose(&sql);

        self.get_client(is_default)
            .await?
            .batch_execute(&sql)
            .await
            .map_err(|e| error::external(format!("PostgreSQL statement failed: {e}")))?;

        Ok("DONE".to_string())
    }
}

fn pg_string_literal(value: &str) -> String {
    value.replace('\'', "''")
}

fn pg_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

#[allow(dead_code)]
fn validate_bulk_rows(bulk: &BulkRows) -> Result<()> {
    for (idx, row) in bulk.rows.iter().enumerate() {
        if row.len() != bulk.columns.len() {
            return Err(error::data(format!(
                "bulk row {idx} has {} values but {} columns were declared",
                row.len(),
                bulk.columns.len()
            )));
        }
    }
    Ok(())
}

#[allow(dead_code)]
fn pg_copy_csv_cell(value: &Option<String>) -> String {
    let Some(value) = value else {
        return "\\N".to_string();
    };
    let needs_quotes = value == "\\N"
        || value.contains(',')
        || value.contains('"')
        || value.contains('\n')
        || value.contains('\r');
    if !needs_quotes {
        return value.to_string();
    }

    format!("\"{}\"", value.replace('"', "\"\""))
}

fn ensure_supported_postgres_security(env: &DataSourceEnvironment) -> Result<()> {
    let security = connection_security::validate(
        Provider::PostgreSql,
        &env.name,
        &env.security_profile,
        &env.tls_mode,
        &env.db_host,
    )
    .map_err(|e| error::external(format!("invalid PostgreSQL connection security: {e}")))?;

    if security.requires_tls() {
        return Err(error::external(
            "PostgreSQL tls_mode requires a TLS connector; this build currently supports tls_mode `disabled` only for local development",
        ));
    }

    Ok(())
}

fn parse_port(value: &str, provider: &str) -> Result<u16> {
    value.parse::<u16>().map_err(|e| {
        error::config(format!(
            "{provider} port `{value}` is not a valid TCP port: {e}"
        ))
    })
}
