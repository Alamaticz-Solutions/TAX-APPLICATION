use std::{
    fs,
    io::{Read, Result},
    path::PathBuf,
};

use appfw_provider_mssql::{
    mssql_odbc_connection_string, odbc::connect_with_connection_string,
    odbc::fetch_rows_as_mssql_rows, MssqlConnectionConfig, MssqlRow,
};
use appfw_runtime::connection_security::{self, Provider};

use crate::{
    bulk::{BackfillCheckpoint, BulkRows},
    console,
    data_source::{DataSource, DataSourceEnvironment, Schema},
    error,
    loader::{get_data_source_schemas, get_schema_path},
    migration_options,
    migrations::drift::ActualColumn,
};

pub struct MsSql {
    default_conn_string: String,
    db_conn_string: String,
    db_name: String,
    data_source: DataSource,
}

impl MsSql {
    pub async fn new(data_source: &DataSource) -> Result<Self> {
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
        let default_conn_string = connection_string(data_source_env.clone(), "master")?;
        let db_conn_string = connection_string(data_source_env.clone(), &data_source_env.db_name)?;
        let db_name = data_source_env.db_name.clone();
        let ds = data_source.clone();
        Ok(Self {
            default_conn_string,
            db_conn_string,
            db_name,
            data_source: ds,
        })
    }

    pub async fn migrate(&self) -> Result<()> {
        console::step("MsSql migration");

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

        self.db_query_scalar(
            false,
            "SELECT 1 WHERE OBJECT_ID(N'[app_meta].[schema_migrations]', N'U') IS NOT NULL"
                .to_string(),
        )
        .await
    }

    pub async fn exec_sql(&self, sql: &str) -> Result<()> {
        for batch in split_go_batches(sql) {
            if !batch.trim().is_empty() {
                self.db_exec(false, batch).await?;
            }
        }
        Ok(())
    }

    pub async fn query_rows(&self, sql: &str) -> Result<Vec<MssqlRow>> {
        self.odbc_query(&self.db_conn_string, sql).await
    }

    pub async fn schema_columns(&self, schema: &str) -> Result<Vec<ActualColumn>> {
        let sql = format!(
            r#"
SELECT
    TABLE_SCHEMA AS schema_name,
    TABLE_NAME AS table_name,
    COLUMN_NAME AS column_name,
    DATA_TYPE AS data_type
FROM INFORMATION_SCHEMA.COLUMNS
WHERE TABLE_SCHEMA = N'{schema}'
ORDER BY TABLE_SCHEMA, TABLE_NAME, ORDINAL_POSITION;
"#,
            schema = tsql_string_literal(schema)
        );
        let rows = self.query_rows(&sql).await?;
        rows.iter()
            .map(|row| {
                Ok(ActualColumn {
                    schema: read_str(row, "schema_name")?.unwrap_or_default(),
                    table: read_str(row, "table_name")?.unwrap_or_default(),
                    column: read_str(row, "column_name")?.unwrap_or_default(),
                    data_type: read_str(row, "data_type")?.unwrap_or_default(),
                })
            })
            .collect()
    }

    #[allow(dead_code)]
    pub async fn bulk_insert_rows(&self, bulk: &BulkRows, batch_size: usize) -> Result<u64> {
        if bulk.is_empty() {
            return Ok(0);
        }
        validate_bulk_rows(bulk)?;

        let batch_size = if batch_size == 0 {
            1000
        } else {
            batch_size.min(1000)
        };
        let table = format!(
            "[{}].[{}]",
            tsql_bracket_identifier(&bulk.schema),
            tsql_bracket_identifier(&bulk.table)
        );
        let columns = bulk
            .columns
            .iter()
            .map(|column| format!("[{}]", tsql_bracket_identifier(column)))
            .collect::<Vec<_>>()
            .join(", ");

        let mut inserted = 0u64;
        for chunk in bulk.rows.chunks(batch_size) {
            let values = chunk
                .iter()
                .map(|row| {
                    let row_values = row.iter().map(tsql_value).collect::<Vec<_>>().join(", ");
                    format!("({row_values})")
                })
                .collect::<Vec<_>>()
                .join(",\n");
            let sql = format!("INSERT INTO {table} ({columns}) VALUES\n{values};");
            self.db_exec(false, sql).await?;
            inserted += chunk.len() as u64;
        }

        Ok(inserted)
    }

    #[allow(dead_code)]
    pub async fn ensure_backfill_registry(&self) -> Result<()> {
        self.exec_sql(
            r#"
IF NOT EXISTS (SELECT 1 FROM sys.schemas WHERE name = N'app_meta')
    EXEC('CREATE SCHEMA [app_meta]');
GO

IF OBJECT_ID(N'[app_meta].[backfill_checkpoints]', N'U') IS NULL
BEGIN
    CREATE TABLE [app_meta].[backfill_checkpoints] (
        [job_name] nvarchar(255) NOT NULL,
        [chunk_key] nvarchar(255) NOT NULL,
        [last_value] nvarchar(1024) NULL,
        [rows_processed] bigint NOT NULL CONSTRAINT [df_backfill_checkpoints_rows_processed] DEFAULT 0,
        [updated_at] datetimeoffset(7) NOT NULL CONSTRAINT [df_backfill_checkpoints_updated_at] DEFAULT SYSDATETIMEOFFSET(),
        CONSTRAINT [pk_backfill_checkpoints] PRIMARY KEY ([job_name], [chunk_key])
    );
END
GO
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
SELECT
    [job_name] AS job_name,
    [chunk_key] AS chunk_key,
    [last_value] AS last_value,
    [rows_processed] AS rows_processed
FROM [app_meta].[backfill_checkpoints]
WHERE [job_name] = N'{job_name}' AND [chunk_key] = N'{chunk_key}';
"#,
            job_name = tsql_string_literal(job_name),
            chunk_key = tsql_string_literal(chunk_key)
        );
        let rows = self.query_rows(&sql).await?;
        let Some(row) = rows.first() else {
            return Ok(None);
        };

        Ok(Some(BackfillCheckpoint {
            job_name: read_str(row, "job_name")?.unwrap_or_default(),
            chunk_key: read_str(row, "chunk_key")?.unwrap_or_default(),
            last_value: read_str(row, "last_value")?,
            rows_processed: read_i64(row, "rows_processed")?.unwrap_or_default(),
        }))
    }

    #[allow(dead_code)]
    pub async fn record_backfill_checkpoint(&self, checkpoint: &BackfillCheckpoint) -> Result<()> {
        self.ensure_backfill_registry().await?;
        let last_value = checkpoint
            .last_value
            .as_deref()
            .map(tsql_value_string)
            .unwrap_or_else(|| "NULL".to_string());
        let sql = format!(
            r#"
IF EXISTS (
    SELECT 1
    FROM [app_meta].[backfill_checkpoints]
    WHERE [job_name] = N'{job_name}' AND [chunk_key] = N'{chunk_key}'
)
BEGIN
    UPDATE [app_meta].[backfill_checkpoints]
    SET [last_value] = {last_value},
        [rows_processed] = {rows_processed},
        [updated_at] = SYSDATETIMEOFFSET()
    WHERE [job_name] = N'{job_name}' AND [chunk_key] = N'{chunk_key}';
END
ELSE
BEGIN
    INSERT INTO [app_meta].[backfill_checkpoints]
        ([job_name], [chunk_key], [last_value], [rows_processed])
    VALUES
        (N'{job_name}', N'{chunk_key}', {last_value}, {rows_processed});
END
"#,
            job_name = tsql_string_literal(&checkpoint.job_name),
            chunk_key = tsql_string_literal(&checkpoint.chunk_key),
            rows_processed = checkpoint.rows_processed,
        );
        self.exec_sql(&sql).await
    }

    async fn ensure_db_exists(&self) -> Result<()> {
        console::step(format!("ensure database: {}", self.db_name));

        if !self.database_exists().await? {
            // SQL Server 2019+ supports UTF-8 via a compatible collation. CREATE DATABASE
            // must run against master and cannot live inside a user transaction.
            let sql = format!(
                "CREATE DATABASE [{}] COLLATE Latin1_General_100_CI_AS_SC_UTF8;",
                tsql_bracket_identifier(&self.db_name)
            );
            console::step(format!("create database: {}", self.db_name));
            self.db_exec(true, sql).await?;
        }

        Ok(())
    }

    pub async fn database_exists(&self) -> Result<bool> {
        let probe = format!(
            "SELECT 1 FROM sys.databases WHERE name = N'{}'",
            tsql_string_literal(&self.db_name)
        );
        self.db_query_scalar(true, probe).await
    }

    async fn ensure_schema_exists(&self, schema: &Schema) -> Result<()> {
        console::step(format!("ensure schema: {}", schema.name));

        // CREATE SCHEMA must be the first statement in a batch, so we wrap it in EXEC.
        let create_schema = format!("CREATE SCHEMA [{}]", tsql_bracket_identifier(&schema.name));
        let sql = format!(
            "IF NOT EXISTS (SELECT 1 FROM sys.schemas WHERE name = N'{name}') EXEC(N'{create_schema}');",
            name = tsql_string_literal(&schema.name),
            create_schema = tsql_string_literal(&create_schema)
        );
        self.db_exec(false, sql).await?;

        Ok(())
    }

    async fn migrate_schema(&self, schema: &Schema) -> Result<()> {
        let schema_path = get_schema_path(schema)?;

        // DDL (T-SQL dialect)
        self.exec_schema_sql(&schema_path, "tables.mssql.sql")
            .await?;

        // Logger tables (optional)
        if self.exec_schema_sql(&schema_path, "logs.sql").await.is_ok() {
            console::verbose(format!("logger schema executed: {}", schema.name));
        }

        // Seed data (optional — only runs if seed.mssql.sql was generated for this schema)
        if migration_options::skip_seed() {
            console::skip(
                &schema_path.join("seed.mssql.sql"),
                "seed execution skipped by APPFW_MIGRATE_SKIP_SEED",
            );
        } else {
            match self.exec_schema_sql(&schema_path, "seed.mssql.sql").await {
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

        // Split on T-SQL `GO` batch separators (must be on its own line, case-insensitive).
        // Empty batches and the trailing trim are ignored.
        for batch in split_go_batches(&sql) {
            if !batch.trim().is_empty() {
                self.db_exec(false, batch).await?;
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

    async fn db_query_scalar(&self, is_default: bool, sql: String) -> Result<bool> {
        console::verbose("db_query_scalar");
        console::verbose(&sql);

        let conn_string = if is_default {
            self.default_conn_string.clone()
        } else {
            self.db_conn_string.clone()
        };
        let rows = self.odbc_query(&conn_string, &sql).await?;
        Ok(!rows.is_empty())
    }

    async fn db_exec(&self, is_default: bool, sql: String) -> Result<String> {
        console::verbose("db_exec");
        console::verbose(&sql);

        let conn_string = if is_default {
            self.default_conn_string.clone()
        } else {
            self.db_conn_string.clone()
        };
        self.odbc_execute(&conn_string, &sql).await?;
        Ok("DONE".to_string())
    }

    async fn odbc_query(&self, conn_string: &str, sql: &str) -> Result<Vec<MssqlRow>> {
        let conn_string = conn_string.to_string();
        let sql = sql.to_string();
        tokio::task::spawn_blocking(move || odbc_query_blocking(&conn_string, &sql))
            .await
            .map_err(|e| error::external(format!("SQL Server ODBC worker failed: {e}")))?
    }

    async fn odbc_execute(&self, conn_string: &str, sql: &str) -> Result<()> {
        let conn_string = conn_string.to_string();
        let sql = sql.to_string();
        tokio::task::spawn_blocking(move || odbc_execute_blocking(&conn_string, &sql))
            .await
            .map_err(|e| error::external(format!("SQL Server ODBC worker failed: {e}")))?
    }
}

fn connection_string(env: DataSourceEnvironment, database: &str) -> Result<String> {
    let config = mssql_connection_config(&env, database)?;
    let security = validate_connection_security(&env)?;
    mssql_odbc_connection_string(&config, &security)
        .map_err(|e| error::external(format!("SQL Server ODBC connection string failed: {e}")))
}

fn mssql_connection_config(
    env: &DataSourceEnvironment,
    database: &str,
) -> Result<MssqlConnectionConfig> {
    MssqlConnectionConfig::from_environment(
        appfw_mssql_auth::PROVIDER_MSSQL,
        env.db_host.clone(),
        env.db_port.clone(),
        database.to_string(),
        env.auth_mode.as_deref(),
        env.service_account_name.clone(),
        env.service_account_password.clone(),
        None,
        None,
    )
    .map_err(|e| error::config(e.to_string()))
}

fn validate_connection_security(
    env: &DataSourceEnvironment,
) -> Result<connection_security::ConnectionSecurity> {
    connection_security::validate(
        Provider::MsSqlServer,
        &env.name,
        &env.security_profile,
        &env.tls_mode,
        &env.db_host,
    )
    .map_err(|e| error::external(format!("invalid SQL Server connection security: {e}")))
}

fn odbc_query_blocking(conn_string: &str, sql: &str) -> Result<Vec<MssqlRow>> {
    let mut conn = connect_with_connection_string(conn_string)
        .map_err(|e| error::external(format!("SQL Server ODBC connect failed: {e}")))?;
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| error::external(format!("SQL Server ODBC prepare failed: {e}")))?;
    stmt.execute()
        .map_err(|e| error::external(format!("SQL Server query failed: {e}")))?;
    fetch_rows_as_mssql_rows(&mut stmt)
        .map_err(|e| error::external(format!("SQL Server result read failed: {e}")))
}

fn odbc_execute_blocking(conn_string: &str, sql: &str) -> Result<()> {
    let mut conn = connect_with_connection_string(conn_string)
        .map_err(|e| error::external(format!("SQL Server ODBC connect failed: {e}")))?;
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| error::external(format!("SQL Server ODBC prepare failed: {e}")))?;
    stmt.execute()
        .map_err(|e| error::external(format!("SQL Server statement failed: {e}")))?;
    Ok(())
}

/// Split a T-SQL script on `GO` batch separators. `GO` must appear on its own
/// line (case-insensitive, optional surrounding whitespace) to act as a separator;
/// anywhere else the word is treated as a normal identifier.
fn split_go_batches(sql: &str) -> Vec<String> {
    let mut batches = Vec::new();
    let mut current = String::new();
    for line in sql.lines() {
        if line.trim().eq_ignore_ascii_case("GO") {
            batches.push(std::mem::take(&mut current));
        } else {
            current.push_str(line);
            current.push('\n');
        }
    }
    if !current.trim().is_empty() {
        batches.push(current);
    }
    batches
}

fn tsql_string_literal(value: &str) -> String {
    value.replace('\'', "''")
}

fn tsql_bracket_identifier(value: &str) -> String {
    value.replace(']', "]]")
}

fn read_str(row: &MssqlRow, column: &str) -> Result<Option<String>> {
    row.try_get::<String, _>(column)
        .map_err(|e| error::external(format!("SQL Server column read failed: {e}")))
}

#[allow(dead_code)]
fn read_i64(row: &MssqlRow, column: &str) -> Result<Option<i64>> {
    row.try_get::<i64, _>(column)
        .map_err(|e| error::external(format!("SQL Server column read failed: {e}")))
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
fn tsql_value(value: &Option<String>) -> String {
    value
        .as_deref()
        .map(tsql_value_string)
        .unwrap_or_else(|| "NULL".to_string())
}

#[allow(dead_code)]
fn tsql_value_string(value: &str) -> String {
    format!("N'{}'", tsql_string_literal(value))
}
