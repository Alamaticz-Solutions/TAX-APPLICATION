use std::{
    io::Result,
    path::Path,
    time::{Duration, Instant},
};

use appfw_provider_mssql::MssqlRow;

use crate::{console, mssql::MsSql, observability};

use super::{
    io_error, migration_report_item, migration_state, print_migration, should_include, sql_literal,
    Command, DataSourceMigrationReport, Migration, MigrationRecord, MigrationState,
    ProviderRunReport,
};

const LOCK_NAME: &str = "schema_migrations";

pub async fn run(
    ms: &MsSql,
    migrations_root: &Path,
    migrations: &[Migration],
    command: &Command,
) -> Result<ProviderRunReport> {
    let mut report = provider_report(migrations, "mssql");
    let mut drift_report = None;
    match command {
        Command::Plan { .. } | Command::Lint { .. } | Command::Status { .. } => {
            let registry_exists = ms.migration_registry_exists().await?;
            for migration in migrations.iter().filter(|m| should_include(command, m)) {
                let checksum = migration.checksum(migrations_root)?;
                let record = if registry_exists {
                    record_for(ms, &migration.id).await?
                } else {
                    None
                };
                let state = migration_state(record.as_ref(), &checksum);
                print_migration(migration, state);
                report
                    .migrations
                    .push(migration_report_item(migration, state, Some(checksum)));
            }
        }
        Command::Apply(_) => {
            ms.ensure_ready().await?;
            let checked_drift_report =
                super::drift::check_mssql(ms, migrations_root, migrations).await?;
            super::drift::write_report(&checked_drift_report)?;
            super::drift::print_report(&checked_drift_report);
            super::drift::fail_on_errors(&checked_drift_report)?;
            drift_report = Some(checked_drift_report);
            ensure_registry(ms).await?;
            acquire_lock(ms).await?;
            let result = apply_locked(ms, migrations_root, migrations, command, &mut report).await;
            let release_result = release_lock(ms).await;
            result?;
            release_result?;
        }
        Command::Bootstrap
        | Command::Doctor { .. }
        | Command::New(_)
        | Command::RollbackGuide { .. }
        | Command::Drift { .. } => {
            return Err(io_error(
                "bootstrap command is not valid for the SQL Server versioned migration runner",
            ))
        }
    }

    Ok(ProviderRunReport {
        data_source: report,
        drift_report,
    })
}

async fn apply_locked(
    ms: &MsSql,
    migrations_root: &Path,
    migrations: &[Migration],
    command: &Command,
    report: &mut DataSourceMigrationReport,
) -> Result<()> {
    for migration in migrations.iter().filter(|m| should_include(command, m)) {
        let checksum = migration.checksum(migrations_root)?;
        let record = record_for(ms, &migration.id).await?;
        match migration_state(record.as_ref(), &checksum) {
            MigrationState::Applied => {
                print_migration(migration, MigrationState::Applied);
                report.migrations.push(migration_report_item(
                    migration,
                    MigrationState::Applied,
                    Some(checksum),
                ));
                observability::record_migration(
                    "mssql",
                    &migration.data_source,
                    &migration.id,
                    &migration.name,
                    migration.phase.as_str(),
                    "already_applied",
                    Duration::ZERO,
                );
            }
            MigrationState::ChecksumMismatch => {
                let mut item = migration_report_item(
                    migration,
                    MigrationState::ChecksumMismatch,
                    Some(checksum),
                );
                item.error =
                    Some("migration was already applied with a different checksum".to_string());
                report.migrations.push(item);
                observability::record_migration(
                    "mssql",
                    &migration.data_source,
                    &migration.id,
                    &migration.name,
                    migration.phase.as_str(),
                    "checksum_mismatch",
                    Duration::ZERO,
                );
                return Err(io_error(format!(
                    "migration {} was already applied with a different checksum",
                    migration.id
                )));
            }
            MigrationState::Failed => {
                let mut item =
                    migration_report_item(migration, MigrationState::Failed, Some(checksum));
                item.error = Some(
                    "migration previously failed; repair manually before retrying".to_string(),
                );
                report.migrations.push(item);
                observability::record_migration(
                    "mssql",
                    &migration.data_source,
                    &migration.id,
                    &migration.name,
                    migration.phase.as_str(),
                    "previously_failed",
                    Duration::ZERO,
                );
                return Err(io_error(format!(
                    "migration {} previously failed; repair manually before retrying",
                    migration.id
                )));
            }
            MigrationState::Pending => {
                print_migration(migration, MigrationState::Pending);
                let sql = migration.sql(migrations_root)?;
                let started = Instant::now();
                let span = observability::migration_span(
                    "mssql",
                    &migration.data_source,
                    &migration.id,
                    &migration.name,
                    migration.phase.as_str(),
                );
                let _entered = span.enter();
                if let Err(error) = ms.exec_sql(&sql).await {
                    let mut item = migration_report_item(
                        migration,
                        MigrationState::Failed,
                        Some(checksum.clone()),
                    );
                    item.error = Some(error.to_string());
                    item.duration_ms = Some(started.elapsed().as_millis() as i64);
                    report.migrations.push(item);
                    observability::record_migration(
                        "mssql",
                        &migration.data_source,
                        &migration.id,
                        &migration.name,
                        migration.phase.as_str(),
                        "failed",
                        started.elapsed(),
                    );
                    record_failure(ms, migration, &checksum, &error.to_string()).await?;
                    return Err(error);
                }
                let duration_ms = started.elapsed().as_millis() as i64;
                let mut item = migration_report_item(
                    migration,
                    MigrationState::Applied,
                    Some(checksum.clone()),
                );
                item.duration_ms = Some(duration_ms);
                report.migrations.push(item);
                observability::record_migration(
                    "mssql",
                    &migration.data_source,
                    &migration.id,
                    &migration.name,
                    migration.phase.as_str(),
                    "applied",
                    started.elapsed(),
                );
                record_success(ms, migration, &checksum, duration_ms).await?;
            }
        }
    }

    Ok(())
}

fn provider_report(migrations: &[Migration], dialect: &str) -> DataSourceMigrationReport {
    DataSourceMigrationReport::new(
        migrations
            .first()
            .map(|migration| migration.data_source.as_str())
            .unwrap_or("unknown"),
        dialect,
    )
}

async fn ensure_registry(ms: &MsSql) -> Result<()> {
    ms.exec_sql(
        r#"
IF NOT EXISTS (SELECT 1 FROM sys.schemas WHERE name = N'app_meta')
    EXEC('CREATE SCHEMA [app_meta]');
GO

IF OBJECT_ID(N'[app_meta].[schema_migrations]', N'U') IS NULL
BEGIN
    CREATE TABLE [app_meta].[schema_migrations] (
        [id] nvarchar(64) NOT NULL CONSTRAINT [pk_schema_migrations] PRIMARY KEY,
        [name] nvarchar(255) NOT NULL,
        [phase] nvarchar(32) NOT NULL,
        [checksum] nvarchar(64) NOT NULL,
        [applied_at] datetimeoffset(7) NOT NULL CONSTRAINT [df_schema_migrations_applied_at] DEFAULT SYSDATETIMEOFFSET(),
        [duration_ms] bigint NOT NULL,
        [status] nvarchar(32) NOT NULL,
        [error] nvarchar(max) NULL
    );
END
GO

IF OBJECT_ID(N'[app_meta].[schema_migration_lock]', N'U') IS NULL
BEGIN
    CREATE TABLE [app_meta].[schema_migration_lock] (
        [lock_name] nvarchar(128) NOT NULL CONSTRAINT [pk_schema_migration_lock] PRIMARY KEY,
        [locked_at] datetimeoffset(7) NOT NULL CONSTRAINT [df_schema_migration_lock_locked_at] DEFAULT SYSDATETIMEOFFSET()
    );
END
GO
"#,
    )
    .await
}

async fn acquire_lock(ms: &MsSql) -> Result<()> {
    let sql = format!(
        r#"
SET NOCOUNT ON;
BEGIN TRAN;

IF NOT EXISTS (
    SELECT 1
    FROM [app_meta].[schema_migration_lock] WITH (UPDLOCK, HOLDLOCK)
    WHERE [lock_name] = N'{lock_name}'
)
BEGIN
    INSERT INTO [app_meta].[schema_migration_lock] ([lock_name]) VALUES (N'{lock_name}');
    SELECT CAST(1 AS int) AS acquired;
END
ELSE
BEGIN
    SELECT CAST(0 AS int) AS acquired;
END

COMMIT TRAN;
"#,
        lock_name = sql_literal(LOCK_NAME)
    );
    let rows = ms.query_rows(&sql).await?;
    if first_i32(&rows, "acquired")? == Some(1) {
        console::verbose("migration lock acquired");
        Ok(())
    } else {
        Err(io_error(
            "another migration run holds the database migration lock",
        ))
    }
}

async fn release_lock(ms: &MsSql) -> Result<()> {
    let sql = format!(
        "DELETE FROM [app_meta].[schema_migration_lock] WHERE [lock_name] = N'{}';",
        sql_literal(LOCK_NAME)
    );
    ms.exec_sql(&sql).await
}

async fn record_for(ms: &MsSql, id: &str) -> Result<Option<MigrationRecord>> {
    let sql = format!(
        "SELECT [id], [checksum], [status] FROM [app_meta].[schema_migrations] WHERE [id] = N'{}';",
        sql_literal(id)
    );
    let rows = ms.query_rows(&sql).await?;
    let Some(row) = rows.first() else {
        return Ok(None);
    };

    Ok(Some(MigrationRecord {
        checksum: read_str(row, "checksum")?.unwrap_or_default(),
        status: read_str(row, "status")?.unwrap_or_default(),
    }))
}

async fn record_success(
    ms: &MsSql,
    migration: &Migration,
    checksum: &str,
    duration_ms: i64,
) -> Result<()> {
    let sql = format!(
        r#"
IF EXISTS (SELECT 1 FROM [app_meta].[schema_migrations] WHERE [id] = N'{id}')
BEGIN
    UPDATE [app_meta].[schema_migrations]
    SET [name] = N'{name}',
        [phase] = N'{phase}',
        [checksum] = N'{checksum}',
        [applied_at] = SYSDATETIMEOFFSET(),
        [duration_ms] = {duration_ms},
        [status] = N'applied',
        [error] = NULL
    WHERE [id] = N'{id}';
END
ELSE
BEGIN
    INSERT INTO [app_meta].[schema_migrations]
        ([id], [name], [phase], [checksum], [duration_ms], [status], [error])
    VALUES
        (N'{id}', N'{name}', N'{phase}', N'{checksum}', {duration_ms}, N'applied', NULL);
END
"#,
        id = sql_literal(&migration.id),
        name = sql_literal(&migration.name),
        phase = migration.phase.as_str(),
        checksum = sql_literal(checksum),
    );
    ms.exec_sql(&sql).await
}

async fn record_failure(
    ms: &MsSql,
    migration: &Migration,
    checksum: &str,
    error: &str,
) -> Result<()> {
    let sql = format!(
        r#"
IF EXISTS (SELECT 1 FROM [app_meta].[schema_migrations] WHERE [id] = N'{id}')
BEGIN
    UPDATE [app_meta].[schema_migrations]
    SET [name] = N'{name}',
        [phase] = N'{phase}',
        [checksum] = N'{checksum}',
        [applied_at] = SYSDATETIMEOFFSET(),
        [duration_ms] = 0,
        [status] = N'failed',
        [error] = N'{error}'
    WHERE [id] = N'{id}';
END
ELSE
BEGIN
    INSERT INTO [app_meta].[schema_migrations]
        ([id], [name], [phase], [checksum], [duration_ms], [status], [error])
    VALUES
        (N'{id}', N'{name}', N'{phase}', N'{checksum}', 0, N'failed', N'{error}');
END
"#,
        id = sql_literal(&migration.id),
        name = sql_literal(&migration.name),
        phase = migration.phase.as_str(),
        checksum = sql_literal(checksum),
        error = sql_literal(error),
    );
    ms.exec_sql(&sql).await
}

fn first_i32(rows: &[MssqlRow], column: &str) -> Result<Option<i32>> {
    let Some(row) = rows.first() else {
        return Ok(None);
    };
    row.try_get::<i32, _>(column)
        .map_err(|e| io_error(e.to_string()))
}

fn read_str(row: &MssqlRow, column: &str) -> Result<Option<String>> {
    row.try_get::<String, _>(column)
        .map_err(|e| io_error(e.to_string()))
}
