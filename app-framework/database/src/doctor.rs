use std::{env, io::Result, time::Duration};

use appfw_mssql_auth::DoctorPreflight;
use appfw_runtime::connection_security::{self, Provider};
use serde::Serialize;
use tokio::{net::TcpStream, time::timeout};

use crate::{
    data_source::{DataSource, DataSourceEnvironment, DataSourceType, Schema},
    loader,
};

#[derive(Debug, Serialize)]
pub struct DoctorReport {
    pub command: &'static str,
    pub ok: bool,
    pub environment: Option<String>,
    pub checks: Vec<DoctorCheck>,
    pub data_sources: Vec<DataSourceDoctor>,
}

#[derive(Debug, Serialize)]
pub struct DataSourceDoctor {
    pub name: String,
    pub provider: String,
    pub environment: Option<String>,
    pub host: Option<String>,
    pub port: Option<String>,
    pub database: Option<String>,
    pub schemas: Vec<String>,
    pub checks: Vec<DoctorCheck>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorCheck {
    pub name: String,
    pub status: CheckStatus,
    pub required: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Ok,
    Warning,
    Error,
    Skipped,
}

impl DoctorReport {
    fn new(environment: Option<String>) -> Self {
        Self {
            command: "doctor",
            ok: true,
            environment,
            checks: Vec::new(),
            data_sources: Vec::new(),
        }
    }

    fn add_check(
        &mut self,
        name: impl Into<String>,
        status: CheckStatus,
        required: bool,
        detail: impl Into<String>,
    ) {
        if required && status == CheckStatus::Error {
            self.ok = false;
        }
        self.checks.push(DoctorCheck {
            name: name.into(),
            status,
            required,
            detail: detail.into(),
        });
    }

    fn add_data_source(&mut self, data_source: DataSourceDoctor) {
        if data_source
            .checks
            .iter()
            .any(|check| check.required && check.status == CheckStatus::Error)
        {
            self.ok = false;
        }
        self.data_sources.push(data_source);
    }
}

impl DataSourceDoctor {
    fn new(
        data_source: &DataSource,
        env: Option<&DataSourceEnvironment>,
        schemas: Vec<String>,
    ) -> Self {
        Self {
            name: data_source.name.clone(),
            provider: data_source.data_source_type.as_str().to_string(),
            environment: env.map(|env| env.name.clone()),
            host: env.map(|env| env.db_host.clone()),
            port: env.map(|env| env.db_port.clone()),
            database: env.map(|env| env.db_name.clone()),
            schemas,
            checks: Vec::new(),
        }
    }

    fn add_check(
        &mut self,
        name: impl Into<String>,
        status: CheckStatus,
        required: bool,
        detail: impl Into<String>,
    ) {
        self.checks.push(DoctorCheck {
            name: name.into(),
            status,
            required,
            detail: detail.into(),
        });
    }
}

pub async fn run() -> Result<DoctorReport> {
    let environment = env::var("ENV_NAME")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let mut report = DoctorReport::new(environment.clone());

    match &environment {
        Some(value) => report.add_check(
            "ENV_NAME",
            CheckStatus::Ok,
            true,
            format!("selected environment `{value}`"),
        ),
        None => report.add_check(
            "ENV_NAME",
            CheckStatus::Error,
            true,
            "ENV_NAME is required to select data source environments",
        ),
    }

    let data_sources = loader::get_configured_data_sources()?;
    let schemas = loader::get_all_schemas().unwrap_or_default();
    for data_source in data_sources {
        let assigned_schemas = assigned_schemas(&schemas, &data_source.name);
        let env_config = environment.as_deref().and_then(|env_name| {
            data_source
                .environments
                .iter()
                .find(|env| env.name == env_name)
        });
        let mut data_source_report =
            DataSourceDoctor::new(&data_source, env_config, assigned_schemas.clone());
        let readiness_required = !assigned_schemas.is_empty();

        if assigned_schemas.is_empty() {
            data_source_report.add_check(
                "schema_assignment",
                CheckStatus::Warning,
                false,
                "no schemas are assigned to this data source in the selected config",
            );
        } else {
            data_source_report.add_check(
                "schema_assignment",
                CheckStatus::Ok,
                true,
                format!("assigned schemas: {}", assigned_schemas.join(", ")),
            );
        }

        let Some(env_config) = env_config else {
            data_source_report.add_check(
                "environment",
                if readiness_required {
                    CheckStatus::Error
                } else {
                    CheckStatus::Warning
                },
                readiness_required,
                "selected ENV_NAME is not configured for this data source",
            );
            report.add_data_source(data_source_report);
            continue;
        };

        if is_external_api_provider(data_source.data_source_type) {
            add_external_api_skip_checks(&mut data_source_report, readiness_required);
            report.add_data_source(data_source_report);
            continue;
        }

        add_tls_check(
            &mut data_source_report,
            data_source.data_source_type,
            env_config,
        );
        let credentials_ok = add_credential_checks(
            &mut data_source_report,
            data_source.data_source_type,
            env_config,
            readiness_required,
        );
        add_reachability_check(&mut data_source_report, env_config, readiness_required).await;

        if readiness_required && credentials_ok {
            let selected = selected_data_source(&data_source, env_config)?;
            add_database_existence_check(&mut data_source_report, &selected).await;
        } else {
            data_source_report.add_check(
                "database_exists",
                CheckStatus::Skipped,
                readiness_required,
                "database existence check requires assigned schemas and complete credentials",
            );
        }

        report.add_data_source(data_source_report);
    }

    Ok(report)
}

fn is_external_api_provider(provider: DataSourceType) -> bool {
    matches!(
        provider,
        DataSourceType::ServiceNow
            | DataSourceType::Workday
            | DataSourceType::Icims
            | DataSourceType::Salesforce
            | DataSourceType::Anaplan
            | DataSourceType::OracleFinancials
    )
}

fn add_external_api_skip_checks(report: &mut DataSourceDoctor, required: bool) {
    report.add_check(
        "tls_mode",
        CheckStatus::Skipped,
        required,
        "external API transport security is certified through SaaS connector contracts",
    );
    report.add_check(
        "credentials",
        CheckStatus::Skipped,
        required,
        "external API credentials are loaded by SaaS connector runtime contracts",
    );
    report.add_check(
        "provider_reachability",
        CheckStatus::Skipped,
        required,
        "external API reachability is checked by provider-specific SaaS smoke tests",
    );
    report.add_check(
        "database_exists",
        CheckStatus::Skipped,
        required,
        "external API providers do not expose framework-managed databases",
    );
}

pub fn print(report: &DoctorReport) {
    println!("Database readiness doctor");
    println!(
        "  environment: {}",
        report.environment.as_deref().unwrap_or("<missing>")
    );
    for check in &report.checks {
        print_check("  ", check);
    }
    for data_source in &report.data_sources {
        println!(
            "\n  data source: {} ({})",
            data_source.name, data_source.provider
        );
        if let Some(database) = &data_source.database {
            println!(
                "    target: {}:{} / {}",
                data_source.host.as_deref().unwrap_or("<unknown>"),
                data_source.port.as_deref().unwrap_or("<unknown>"),
                database
            );
        }
        if !data_source.schemas.is_empty() {
            println!("    schemas: {}", data_source.schemas.join(", "));
        }
        for check in &data_source.checks {
            print_check("    ", check);
        }
    }
}

fn print_check(indent: &str, check: &DoctorCheck) {
    println!(
        "{}{:8} {:24} {}",
        indent,
        status_label(check.status),
        check.name,
        check.detail
    );
}

fn status_label(status: CheckStatus) -> &'static str {
    match status {
        CheckStatus::Ok => "ok",
        CheckStatus::Warning => "warning",
        CheckStatus::Error => "error",
        CheckStatus::Skipped => "skipped",
    }
}

fn assigned_schemas(schemas: &[Schema], data_source_name: &str) -> Vec<String> {
    schemas
        .iter()
        .filter(|schema| {
            schema
                .data_source_name
                .eq_ignore_ascii_case(data_source_name)
        })
        .map(|schema| schema.name.clone())
        .collect()
}

fn add_tls_check(
    report: &mut DataSourceDoctor,
    provider: DataSourceType,
    env: &DataSourceEnvironment,
) {
    match connection_security::validate(
        provider_for(provider),
        &env.name,
        &env.security_profile,
        &env.tls_mode,
        &env.db_host,
    ) {
        Ok(_) => report.add_check(
            "tls_mode",
            CheckStatus::Ok,
            true,
            format!(
                "security_profile={}, tls_mode={}",
                env.security_profile, env.tls_mode
            ),
        ),
        Err(error) => report.add_check("tls_mode", CheckStatus::Error, true, error.to_string()),
    }
}

fn add_credential_checks(
    report: &mut DataSourceDoctor,
    provider: DataSourceType,
    env: &DataSourceEnvironment,
    required: bool,
) -> bool {
    let provider_key = match provider {
        DataSourceType::MsSqlServer => Some(appfw_mssql_auth::PROVIDER_MSSQL),
        DataSourceType::FabricSqlAnalytics => Some(appfw_mssql_auth::PROVIDER_FABRIC),
        _ => None,
    };
    if let Some(provider_key) = provider_key {
        if let Some(preflight) =
            appfw_mssql_auth::doctor_preflight(provider_key, env.auth_mode.as_deref())
        {
            return match preflight {
                DoctorPreflight::SqlServiceAccount
                | DoctorPreflight::EntraAccessToken
                | DoctorPreflight::EntraClientCredentials => {
                    add_env_credential_group_checks(report, provider, required)
                }
            };
        }
    }

    add_env_credential_group_checks(report, provider, required)
}

fn add_env_credential_group_checks(
    report: &mut DataSourceDoctor,
    provider: DataSourceType,
    required: bool,
) -> bool {
    let groups = credential_groups(provider);
    let mut ok = true;
    for group in groups {
        let present = group
            .iter()
            .filter(|name| {
                env::var(name)
                    .ok()
                    .filter(|value| !value.trim().is_empty())
                    .is_some()
            })
            .map(|name| (*name).to_string())
            .collect::<Vec<_>>();
        if present.is_empty() {
            ok = false;
            report.add_check(
                "credentials",
                if required {
                    CheckStatus::Error
                } else {
                    CheckStatus::Warning
                },
                required,
                if required {
                    format!("missing one of: {}", group.join(", "))
                } else {
                    format!(
                        "not required until schemas are assigned; missing one of: {}",
                        group.join(", ")
                    )
                },
            );
        } else {
            report.add_check(
                "credentials",
                CheckStatus::Ok,
                required,
                format!("present: {}", present.join(", ")),
            );
        }
    }
    ok
}

async fn add_reachability_check(
    report: &mut DataSourceDoctor,
    env: &DataSourceEnvironment,
    required: bool,
) {
    let host = connection_host(&env.db_host);
    let address = format!("{}:{}", host, env.db_port);
    match timeout(Duration::from_secs(2), TcpStream::connect(&address)).await {
        Ok(Ok(_)) => report.add_check(
            "provider_reachability",
            CheckStatus::Ok,
            required,
            format!("tcp connect succeeded: {address}"),
        ),
        Ok(Err(error)) => report.add_check(
            "provider_reachability",
            CheckStatus::Error,
            required,
            format!("tcp connect failed for {address}: {error}"),
        ),
        Err(_) => report.add_check(
            "provider_reachability",
            CheckStatus::Error,
            required,
            format!("tcp connect timed out for {address}"),
        ),
    }
}

async fn add_database_existence_check(report: &mut DataSourceDoctor, data_source: &DataSource) {
    let result = match data_source.data_source_type {
        DataSourceType::PostgreSQL => match crate::postgres::Postgres::new(data_source) {
            Ok(pg) => pg.database_exists().await,
            Err(error) => Err(error),
        },
        DataSourceType::MsSqlServer => match crate::mssql::MsSql::new(data_source).await {
            Ok(ms) => ms.database_exists().await,
            Err(error) => Err(error),
        },
        DataSourceType::FabricSqlAnalytics => {
            report.add_check(
                "database_exists",
                CheckStatus::Skipped,
                false,
                "FabricSqlAnalytics is an external read-only SQL analytics endpoint; database bootstrap/existence is not managed by the database utility",
            );
            return;
        }
        DataSourceType::MongoDB => match crate::mongo::Mongo::new(data_source).await {
            Ok(mongo) => mongo.database_exists().await,
            Err(error) => Err(error),
        },
        DataSourceType::Snowflake => {
            report.add_check(
                "database_exists",
                CheckStatus::Skipped,
                true,
                "Snowflake database existence check is not implemented in local doctor",
            );
            return;
        }
        DataSourceType::Neo4j => {
            report.add_check(
                "database_exists",
                CheckStatus::Skipped,
                false,
                "Neo4j graph read model existence checks are handled by graph projection certification",
            );
            return;
        }
        DataSourceType::ServiceNow
        | DataSourceType::Workday
        | DataSourceType::Icims
        | DataSourceType::Salesforce
        | DataSourceType::Anaplan
        | DataSourceType::OracleFinancials => {
            report.add_check(
                "database_exists",
                CheckStatus::Skipped,
                false,
                "External API provider existence checks are handled by SaaS connector certification",
            );
            return;
        }
    };

    match result {
        Ok(true) => report.add_check("database_exists", CheckStatus::Ok, true, "database exists"),
        Ok(false) => report.add_check(
            "database_exists",
            CheckStatus::Error,
            true,
            "database does not exist; run database migration/bootstrap first",
        ),
        Err(error) => report.add_check(
            "database_exists",
            CheckStatus::Error,
            true,
            format!("database existence check failed: {error}"),
        ),
    }
}

fn selected_data_source(
    data_source: &DataSource,
    env_config: &DataSourceEnvironment,
) -> Result<DataSource> {
    let mut selected = data_source.clone();
    let mut selected_env = env_config.clone();
    match data_source.data_source_type {
        DataSourceType::PostgreSQL => {
            selected_env.service_account_name = env::var("PG_SERVICE_ACCOUNT_NAME").ok();
            selected_env.service_account_password = env::var("PG_SERVICE_ACCOUNT_PASS").ok();
        }
        DataSourceType::MsSqlServer => {
            selected_env.auth_mode = env::var("MSSQL_AUTH_MODE")
                .ok()
                .or_else(|| selected_env.auth_mode.clone())
                .or_else(|| Some("sql_password".to_string()));
            selected_env.service_account_name = env::var("MSSQL_SERVICE_ACCOUNT_NAME").ok();
            selected_env.service_account_password = env::var("MSSQL_SERVICE_ACCOUNT_PASS").ok();
        }
        DataSourceType::FabricSqlAnalytics => {
            selected_env.auth_mode = env::var("FABRIC_SQL_AUTH_MODE")
                .ok()
                .or_else(|| selected_env.auth_mode.clone())
                .or_else(|| Some("entra_client_credentials".to_string()));
            selected_env.entra_tenant_id = env::var("FABRIC_TENANT_ID")
                .ok()
                .or_else(|| selected_env.entra_tenant_id.clone());
            selected_env.entra_token_scope = env::var("FABRIC_TOKEN_SCOPE")
                .ok()
                .or_else(|| selected_env.entra_token_scope.clone());
            selected_env.service_account_name = env::var("FABRIC_CLIENT_ID").ok();
            selected_env.service_account_password =
                first_env(&["FABRIC_CLIENT_SECRET", "FABRIC_ACCESS_TOKEN"]);
        }
        DataSourceType::MongoDB => {
            selected_env.service_account_name = env::var("MONGO_SERVICE_ACCOUNT_NAME").ok();
            selected_env.service_account_password = env::var("MONGO_SERVICE_ACCOUNT_PASS").ok();
        }
        DataSourceType::Snowflake => {
            selected_env.service_account_name = env::var("SNOWFLAKE_SERVICE_ACCOUNT_NAME").ok();
            selected_env.service_account_password = first_env(&[
                "SNOWFLAKE_ACCESS_TOKEN",
                "SNOWFLAKE_OAUTH_TOKEN",
                "SNOWFLAKE_JWT",
                "SNOWFLAKE_SERVICE_ACCOUNT_PASS",
            ]);
        }
        DataSourceType::Neo4j => {
            selected_env.service_account_name = env::var("NEO4J_SERVICE_ACCOUNT_NAME").ok();
            selected_env.service_account_password = env::var("NEO4J_SERVICE_ACCOUNT_PASS").ok();
        }
        DataSourceType::ServiceNow
        | DataSourceType::Workday
        | DataSourceType::Icims
        | DataSourceType::Salesforce
        | DataSourceType::Anaplan
        | DataSourceType::OracleFinancials => {}
    }
    selected.environments = vec![selected_env];
    Ok(selected)
}

fn first_env(names: &[&str]) -> Option<String> {
    names
        .iter()
        .find_map(|name| env::var(name).ok().filter(|value| !value.trim().is_empty()))
}

fn credential_groups(provider: DataSourceType) -> Vec<Vec<&'static str>> {
    match provider {
        DataSourceType::PostgreSQL => vec![
            vec!["PG_SERVICE_ACCOUNT_NAME"],
            vec!["PG_SERVICE_ACCOUNT_PASS"],
        ],
        DataSourceType::MsSqlServer => vec![
            vec!["MSSQL_SERVICE_ACCOUNT_NAME"],
            vec!["MSSQL_SERVICE_ACCOUNT_PASS"],
        ],
        DataSourceType::FabricSqlAnalytics => vec![
            vec!["FABRIC_TENANT_ID"],
            vec!["FABRIC_CLIENT_ID"],
            vec!["FABRIC_CLIENT_SECRET", "FABRIC_ACCESS_TOKEN"],
        ],
        DataSourceType::MongoDB => vec![
            vec!["MONGO_SERVICE_ACCOUNT_NAME"],
            vec!["MONGO_SERVICE_ACCOUNT_PASS"],
        ],
        DataSourceType::Snowflake => vec![vec![
            "SNOWFLAKE_ACCESS_TOKEN",
            "SNOWFLAKE_OAUTH_TOKEN",
            "SNOWFLAKE_JWT",
            "SNOWFLAKE_SERVICE_ACCOUNT_PASS",
        ]],
        DataSourceType::Neo4j => vec![
            vec!["NEO4J_SERVICE_ACCOUNT_NAME"],
            vec!["NEO4J_SERVICE_ACCOUNT_PASS"],
        ],
        DataSourceType::ServiceNow
        | DataSourceType::Workday
        | DataSourceType::Icims
        | DataSourceType::Salesforce
        | DataSourceType::Anaplan
        | DataSourceType::OracleFinancials => Vec::new(),
    }
}

fn provider_for(provider: DataSourceType) -> Provider {
    match provider {
        DataSourceType::PostgreSQL => Provider::PostgreSql,
        DataSourceType::MongoDB => Provider::MongoDb,
        DataSourceType::MsSqlServer => Provider::MsSqlServer,
        DataSourceType::FabricSqlAnalytics => Provider::FabricSqlAnalytics,
        DataSourceType::Snowflake => Provider::Snowflake,
        DataSourceType::Neo4j => Provider::Neo4j,
        DataSourceType::ServiceNow
        | DataSourceType::Workday
        | DataSourceType::Icims
        | DataSourceType::Salesforce
        | DataSourceType::Anaplan
        | DataSourceType::OracleFinancials => {
            unreachable!("external API providers are skipped before connection-security validation")
        }
    }
}

fn connection_host(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or(value)
        .rsplit_once(':')
        .map(|(host, _)| host)
        .unwrap_or(value)
        .to_string()
}
