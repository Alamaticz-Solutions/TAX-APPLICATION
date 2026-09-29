use std::{
    io::Result,
    path::Path,
    time::{Duration, Instant},
};

use crate::{console, observability, postgres::Postgres};

use super::{
    io_error, migration_report_item, migration_state, print_migration, should_include, sql_literal,
    Command, DataSourceMigrationReport, Migration, MigrationRecord, MigrationState,
    ProviderRunReport,
};

const LOCK_NAME: &str = "schema_migrations";

pub async fn run(
    pg: &Postgres,
    migrations_root: &Path,
    migrations: &[Migration],
    command: &Command,
) -> Result<ProviderRunReport> {
    let mut report = provider_report(migrations, "postgresql");
    let mut drift_report = None;
    match command {
        Command::Plan { .. } | Command::Lint { .. } | Command::Status { .. } => {
            let registry_exists = pg.migration_registry_exists().await?;
            for migration in migrations.iter().filter(|m| should_include(command, m)) {
                let checksum = migration.checksum(migrations_root)?;
                let record = if registry_exists {
                    record_for(pg, &migration.id).await?
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
            pg.ensure_ready().await?;
            let checked_drift_report =
                super::drift::check_postgres(pg, migrations_root, migrations).await?;
            super::drift::write_report(&checked_drift_report)?;
            super::drift::print_report(&checked_drift_report);
            super::drift::fail_on_errors(&checked_drift_report)?;
            drift_report = Some(checked_drift_report);
            ensure_registry(pg).await?;
            acquire_lock(pg).await?;
            let result = apply_locked(pg, migrations_root, migrations, command, &mut report).await;
            let release_result = release_lock(pg).await;
            result?;
            release_result?;
        }
        Command::Bootstrap
        | Command::Doctor { .. }
        | Command::New(_)
        | Command::RollbackGuide { .. }
        | Command::Drift { .. } => {
            return Err(io_error(
                "bootstrap command is not valid for the PostgreSQL versioned migration runner",
            ))
        }
    }

    Ok(ProviderRunReport {
        data_source: report,
        drift_report,
    })
}

async fn apply_locked(
    pg: &Postgres,
    migrations_root: &Path,
    migrations: &[Migration],
    command: &Command,
    report: &mut DataSourceMigrationReport,
) -> Result<()> {
    for migration in migrations.iter().filter(|m| should_include(command, m)) {
        let checksum = migration.checksum(migrations_root)?;
        let record = record_for(pg, &migration.id).await?;
        match migration_state(record.as_ref(), &checksum) {
            MigrationState::Applied => {
                print_migration(migration, MigrationState::Applied);
                report.migrations.push(migration_report_item(
                    migration,
                    MigrationState::Applied,
                    Some(checksum),
                ));
                observability::record_migration(
                    "postgresql",
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
                    "postgresql",
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
                    "postgresql",
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
                    "postgresql",
                    &migration.data_source,
                    &migration.id,
                    &migration.name,
                    migration.phase.as_str(),
                );
                let _entered = span.enter();
                if let Err(error) = pg.exec_sql(&sql).await {
                    let mut item = migration_report_item(
                        migration,
                        MigrationState::Failed,
                        Some(checksum.clone()),
                    );
                    item.error = Some(error.to_string());
                    item.duration_ms = Some(started.elapsed().as_millis() as i64);
                    report.migrations.push(item);
                    observability::record_migration(
                        "postgresql",
                        &migration.data_source,
                        &migration.id,
                        &migration.name,
                        migration.phase.as_str(),
                        "failed",
                        started.elapsed(),
                    );
                    record_failure(pg, migration, &checksum, &error.to_string()).await?;
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
                    "postgresql",
                    &migration.data_source,
                    &migration.id,
                    &migration.name,
                    migration.phase.as_str(),
                    "applied",
                    started.elapsed(),
                );
                record_success(pg, migration, &checksum, duration_ms).await?;
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

async fn ensure_registry(pg: &Postgres) -> Result<()> {
    pg.exec_sql(
        r#"
CREATE SCHEMA IF NOT EXISTS app_meta;

CREATE TABLE IF NOT EXISTS app_meta.schema_migrations (
    id text PRIMARY KEY,
    name text NOT NULL,
    phase text NOT NULL,
    checksum text NOT NULL,
    applied_at timestamptz NOT NULL DEFAULT now(),
    duration_ms bigint NOT NULL,
    status text NOT NULL,
    error text NULL
);

CREATE TABLE IF NOT EXISTS app_meta.schema_migration_lock (
    lock_name text PRIMARY KEY,
    locked_at timestamptz NOT NULL DEFAULT now()
);
"#,
    )
    .await
}

async fn acquire_lock(pg: &Postgres) -> Result<()> {
    let sql = format!(
        "INSERT INTO app_meta.schema_migration_lock(lock_name) VALUES ('{}') ON CONFLICT DO NOTHING RETURNING to_json(lock_name)",
        sql_literal(LOCK_NAME)
    );
    if pg.query_value_json(&sql).await?.is_some() {
        console::verbose("migration lock acquired");
        Ok(())
    } else {
        Err(io_error(
            "another migration run holds the database migration lock",
        ))
    }
}

async fn release_lock(pg: &Postgres) -> Result<()> {
    let sql = format!(
        "DELETE FROM app_meta.schema_migration_lock WHERE lock_name = '{}'",
        sql_literal(LOCK_NAME)
    );
    pg.exec_sql(&sql).await
}

async fn record_for(pg: &Postgres, id: &str) -> Result<Option<MigrationRecord>> {
    let sql = format!(
        "SELECT id, checksum, status FROM app_meta.schema_migrations WHERE id = '{}'",
        sql_literal(id)
    );
    let Some(value) = pg.query_json(&sql).await? else {
        return Ok(None);
    };

    Ok(Some(MigrationRecord {
        checksum: value["checksum"].as_str().unwrap_or_default().to_string(),
        status: value["status"].as_str().unwrap_or_default().to_string(),
    }))
}

async fn record_success(
    pg: &Postgres,
    migration: &Migration,
    checksum: &str,
    duration_ms: i64,
) -> Result<()> {
    let sql = format!(
        r#"
INSERT INTO app_meta.schema_migrations
    (id, name, phase, checksum, duration_ms, status, error)
VALUES
    ('{id}', '{name}', '{phase}', '{checksum}', {duration_ms}, 'applied', NULL)
ON CONFLICT (id) DO UPDATE SET
    name = EXCLUDED.name,
    phase = EXCLUDED.phase,
    checksum = EXCLUDED.checksum,
    applied_at = now(),
    duration_ms = EXCLUDED.duration_ms,
    status = EXCLUDED.status,
    error = NULL;
"#,
        id = sql_literal(&migration.id),
        name = sql_literal(&migration.name),
        phase = migration.phase.as_str(),
        checksum = sql_literal(checksum),
    );
    pg.exec_sql(&sql).await
}

async fn record_failure(
    pg: &Postgres,
    migration: &Migration,
    checksum: &str,
    error: &str,
) -> Result<()> {
    let sql = format!(
        r#"
INSERT INTO app_meta.schema_migrations
    (id, name, phase, checksum, duration_ms, status, error)
VALUES
    ('{id}', '{name}', '{phase}', '{checksum}', 0, 'failed', '{error}')
ON CONFLICT (id) DO UPDATE SET
    name = EXCLUDED.name,
    phase = EXCLUDED.phase,
    checksum = EXCLUDED.checksum,
    applied_at = now(),
    duration_ms = 0,
    status = EXCLUDED.status,
    error = EXCLUDED.error;
"#,
        id = sql_literal(&migration.id),
        name = sql_literal(&migration.name),
        phase = migration.phase.as_str(),
        checksum = sql_literal(checksum),
        error = sql_literal(error),
    );
    pg.exec_sql(&sql).await
}
