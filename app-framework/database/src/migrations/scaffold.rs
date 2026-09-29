use std::{
    fs,
    io::{Result, Write},
    path::PathBuf,
};

use chrono::Utc;
use serde::Serialize;

use crate::{
    data_source::{DataSource, DataSourceType},
    loader,
    migrations::{io_error, Phase},
};

#[derive(Debug, Clone)]
pub struct NewMigrationOptions {
    pub schema: String,
    pub phase: Phase,
    pub name: String,
    pub data_source: Option<String>,
    pub dialect: Option<String>,
    pub description: Option<String>,
    pub json: bool,
}

#[derive(Debug, Serialize)]
pub struct NewMigrationReport {
    pub command: &'static str,
    pub ok: bool,
    pub id: String,
    pub schema: String,
    pub phase: Phase,
    pub name: String,
    pub files: Vec<NewMigrationFile>,
    pub manifest: String,
}

#[derive(Debug, Serialize)]
pub struct NewMigrationFile {
    pub data_source: String,
    pub dialect: String,
    pub path: String,
}

pub fn parse_new_options(args: &[String], json: bool) -> Result<NewMigrationOptions> {
    let mut schema = None;
    let mut phase = None;
    let mut name = None;
    let mut data_source = None;
    let mut dialect = None;
    let mut description = None;
    let mut index = 0usize;

    while index < args.len() {
        match args[index].as_str() {
            "--schema" => {
                index += 1;
                schema = Some(required_value(args, index, "--schema")?.to_string());
            }
            "--phase" => {
                index += 1;
                phase = Some(parse_phase(required_value(args, index, "--phase")?)?);
            }
            "--name" => {
                index += 1;
                name = Some(slug(required_value(args, index, "--name")?));
            }
            "--data-source" => {
                index += 1;
                data_source = Some(required_value(args, index, "--data-source")?.to_string());
            }
            "--dialect" => {
                index += 1;
                dialect = Some(required_value(args, index, "--dialect")?.to_string());
            }
            "--description" => {
                index += 1;
                description = Some(required_value(args, index, "--description")?.to_string());
            }
            "--json" => {}
            other => {
                return Err(io_error(format!(
                    "unknown migrate new option `{other}`; use --schema, --phase, --name, --data-source, --dialect, or --description"
                )))
            }
        }
        index += 1;
    }

    Ok(NewMigrationOptions {
        schema: schema.ok_or_else(|| io_error("migrate new requires --schema"))?,
        phase: phase.ok_or_else(|| io_error("migrate new requires --phase"))?,
        name: name.ok_or_else(|| io_error("migrate new requires --name"))?,
        data_source,
        dialect,
        description,
        json,
    })
}

pub fn create_migration(options: &NewMigrationOptions) -> Result<NewMigrationReport> {
    let migrations_root = loader::get_migrations_path()?;
    let manifest_path = migrations_root.join("manifest.yaml");
    let schemas = loader::get_all_schemas()?;
    if !schemas
        .iter()
        .any(|schema| schema.name.eq_ignore_ascii_case(&options.schema))
    {
        return Err(io_error(format!(
            "schema `{}` is not configured",
            options.schema
        )));
    }

    let data_sources = loader::get_configured_data_sources()?;
    let targets = migration_targets(options, &data_sources)?;
    if targets.is_empty() {
        return Err(io_error(
            "no data sources matched the requested migration target",
        ));
    }

    let id = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let migration_name = format!("{}_{}", slug(&options.schema), options.name);
    let mut files = Vec::new();
    let mut manifest_entries = String::new();

    for data_source in targets {
        let dialect = data_source.data_source_type.as_str();
        let relative = format!(
            "{dialect}/{id}__{migration_name}.{}.sql",
            options.phase.as_str()
        );
        let path = migrations_root.join(&relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        if path.exists() {
            return Err(io_error(format!(
                "migration file already exists: {}",
                path.display()
            )));
        }
        fs::write(
            &path,
            template_sql(&options.schema, &migration_name, options.phase, dialect),
        )?;

        manifest_entries.push_str(&manifest_entry(
            &id,
            &migration_name,
            &options.schema,
            &data_source.name,
            dialect,
            options.phase,
            &relative,
            options.description.as_deref(),
        ));
        files.push(NewMigrationFile {
            data_source: data_source.name,
            dialect: dialect.to_string(),
            path: path.display().to_string(),
        });
    }

    append_manifest_entries(&manifest_path, &manifest_entries)?;

    Ok(NewMigrationReport {
        command: "migrate new",
        ok: true,
        id,
        schema: options.schema.clone(),
        phase: options.phase,
        name: migration_name,
        files,
        manifest: manifest_path.display().to_string(),
    })
}

fn migration_targets(
    options: &NewMigrationOptions,
    data_sources: &[DataSource],
) -> Result<Vec<DataSource>> {
    let mut targets = Vec::new();
    for data_source in data_sources {
        if let Some(requested) = &options.data_source {
            if !data_source.name.eq_ignore_ascii_case(requested) {
                continue;
            }
        }
        if let Some(dialect) = &options.dialect {
            if dialect != "all" && data_source.data_source_type.as_str() != dialect {
                continue;
            }
        } else if options.data_source.is_none() {
            let schema = loader::get_all_schemas()?
                .into_iter()
                .find(|schema| schema.name.eq_ignore_ascii_case(&options.schema))
                .ok_or_else(|| {
                    io_error(format!("schema `{}` is not configured", options.schema))
                })?;
            if !data_source
                .name
                .eq_ignore_ascii_case(&schema.data_source_name)
            {
                continue;
            }
        }
        if matches!(
            data_source.data_source_type,
            DataSourceType::PostgreSQL | DataSourceType::MsSqlServer
        ) {
            targets.push(data_source.clone());
        }
    }
    Ok(targets)
}

fn append_manifest_entries(manifest_path: &PathBuf, entries: &str) -> Result<()> {
    if !manifest_path.exists() {
        if let Some(parent) = manifest_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(manifest_path, "version: 1\nmigrations:\n")?;
    }
    let mut file = fs::OpenOptions::new().append(true).open(manifest_path)?;
    file.write_all(entries.as_bytes())?;
    Ok(())
}

fn manifest_entry(
    id: &str,
    name: &str,
    schema: &str,
    data_source: &str,
    dialect: &str,
    phase: Phase,
    path: &str,
    description: Option<&str>,
) -> String {
    format!(
        r#"
  - id: "{id}"
    name: {name}
    schema: {schema}
    data_source: {data_source}
    dialect: {dialect}
    phase: {phase}
    path: {path}
    description: {description}
"#,
        phase = phase.as_str(),
        description = yaml_quote(description.unwrap_or("TODO: describe the migration"))
    )
}

fn yaml_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn template_sql(schema: &str, name: &str, phase: Phase, dialect: &str) -> String {
    let provider_hint = match dialect {
        "postgresql" => "PostgreSQL",
        "mssql" => "SQL Server",
        other => other,
    };
    let phase_guidance = match phase {
        Phase::Expand => {
            "-- Expand guidance: add backward-compatible structures only.\n-- Prefer nullable-first columns, additive tables, and provider-safe indexes.\n"
        }
        Phase::Backfill => {
            "-- Backfill guidance: make every data change resumable, bounded, and idempotent.\n-- Use checkpoints or stable key ranges rather than unbounded UPDATE/DELETE.\n"
        }
        Phase::Contract => {
            "-- Contract guidance: requires app rollback safety window and --confirm-contract.\n-- Add the rollback review marker after rollback and recovery review.\n"
        }
    };

    format!(
        r#"--
-- Forward-only {phase} migration.
-- Schema: {schema}
-- Name: {name}
-- Dialect: {provider_hint}
--
{phase_guidance}-- TODO: write idempotent SQL. Keep expand/backfill/contract sequencing intact.

"#,
        phase = phase.as_str()
    )
}

fn required_value<'a>(args: &'a [String], index: usize, option: &str) -> Result<&'a str> {
    args.get(index)
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| io_error(format!("{option} requires a value")))
}

fn parse_phase(value: &str) -> Result<Phase> {
    match value {
        "expand" => Ok(Phase::Expand),
        "backfill" => Ok(Phase::Backfill),
        "contract" => Ok(Phase::Contract),
        other => Err(io_error(format!(
            "unknown migration phase `{other}`; expected expand, backfill, or contract"
        ))),
    }
}

fn slug(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}
