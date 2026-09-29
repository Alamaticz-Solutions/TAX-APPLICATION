#![allow(
    clippy::io_other_error,
    clippy::print_literal,
    clippy::ptr_arg,
    clippy::redundant_pattern_matching,
    clippy::too_many_arguments,
    clippy::trim_split_whitespace,
    clippy::unnecessary_map_or,
    clippy::useless_conversion
)]

use data_source::DataSourceType;
use dotenv::dotenv;
use loader::get_data_sources;
use std::{env, io::Result};

mod bulk;
mod console;
mod data_source;
mod doctor;
mod error;
mod loader;
mod migration_options;
mod migrations;
mod mongo;
mod mssql;
mod observability;
mod postgres;
mod snowflake;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let command = match migrations::Command::from_env_args() {
        Ok(command) => command,
        Err(error) => {
            eprintln!("command error: {error}");
            std::process::exit(1);
        }
    };
    if command.json_output() {
        console::set_quiet(true);
        observability::set_silent(true);
    }

    let observability_guard = observability::init_tracing();

    console::app_start("database");
    if let Ok(curr_dir) = env::current_dir() {
        console::cwd(&curr_dir);
    }

    let result = match command {
        migrations::Command::Bootstrap => migrate().await,
        command => migrations::run(command).await,
    };

    if let Err(error) = result {
        console::warn(format!("database error: {error}"));
        observability::write_metrics();
        observability_guard.shutdown();
        std::process::exit(1);
    }

    console::app_done("database");
    observability::write_metrics();
    observability_guard.shutdown();
}

async fn migrate() -> Result<()> {
    console::stage("migrate");

    let data_sources = get_data_sources()?;

    for data_source in data_sources {
        console::item(
            "data source",
            format!(
                "{} ({})",
                &data_source.name,
                data_source.data_source_type.as_str()
            ),
        );

        match data_source.data_source_type {
            DataSourceType::PostgreSQL => {
                let pg = postgres::Postgres::new(&data_source)?;
                pg.migrate().await?;
            }
            DataSourceType::MongoDB => {
                let mongo = mongo::Mongo::new(&data_source).await?;
                mongo.migrate().await?;
            }
            DataSourceType::MsSqlServer => {
                let ms = mssql::MsSql::new(&data_source).await?;
                ms.migrate().await?;
            }
            DataSourceType::FabricSqlAnalytics => {
                console::skip(
                    &env::current_dir().unwrap_or_default(),
                    "FabricSqlAnalytics is a read-only Microsoft Fabric SQL analytics endpoint; database migrate is not supported",
                );
            }
            DataSourceType::Snowflake => {
                let sf = snowflake::Snowflake::new(&data_source)?;
                sf.migrate().await?;
            }
            DataSourceType::Neo4j => {
                console::skip(
                    &env::current_dir().unwrap_or_default(),
                    "Neo4j is a graph read provider; projection/bootstrap is not part of database migrate",
                );
            }
            DataSourceType::ServiceNow
            | DataSourceType::Workday
            | DataSourceType::Icims
            | DataSourceType::Salesforce
            | DataSourceType::Anaplan
            | DataSourceType::OracleFinancials => {
                console::skip(
                    &env::current_dir().unwrap_or_default(),
                    format!(
                        "{} is an external API provider; database migrate is not managed by the database utility",
                        data_source.data_source_type.as_str()
                    ),
                );
            }
        }
    }

    console::done("migrate");
    Ok(())
}
