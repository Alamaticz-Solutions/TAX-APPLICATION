use std::io::{BufReader, Result};
use std::path::PathBuf;
use std::{
    env,
    fs::{self, File},
};

use appfw_mssql_auth::{self, FieldPresence};
use appfw_runtime::{
    connection_security::{self, Provider},
    secrets::{EnvSecretProvider, SecretError, SecretProvider},
};

use crate::{
    data_source::{DataSource, DataSourceType, Schema},
    error,
};

pub fn get_data_source_schemas(data_source_name: &str) -> Result<Vec<Schema>> {
    let mut schemas: Vec<Schema> = Vec::new();
    for schema in get_all_schemas()? {
        if data_source_name.eq_ignore_ascii_case(&schema.data_source_name) {
            schemas.push(schema);
        }
    }
    Ok(schemas)
}

pub fn get_all_schemas() -> Result<Vec<Schema>> {
    let schemas_dir = get_pkg_path()?.join("schemas");
    let mut schemas = Vec::new();
    for dir_entry in fs::read_dir(&schemas_dir).map_err(|e| {
        error::missing_config(format!(
            "could not read schemas directory `{}`: {e}",
            schemas_dir.display()
        ))
    })? {
        let dir_path = dir_entry?.path();
        if dir_path.is_dir() {
            schemas.push(get_schema(&dir_path)?);
        }
    }
    Ok(schemas)
}

pub fn get_schema(schema_dir: &PathBuf) -> Result<Schema> {
    let schema_path = schema_dir.join("schema.yaml");
    let file = File::open(&schema_path).map_err(|e| {
        error::missing_config(format!(
            "could not open schema config `{}`: {e}",
            schema_path.display()
        ))
    })?;
    let reader = BufReader::new(file);
    let schema: Schema = serde_yaml::from_reader(reader).map_err(|e| {
        error::data(format!(
            "could not parse schema config `{}`: {e}",
            schema_path.display()
        ))
    })?;

    Ok(apply_schema_data_source_override(schema))
}

pub fn get_data_sources() -> Result<Vec<DataSource>> {
    let mut updated_data_sources = vec![];
    for data_source in get_configured_data_sources()? {
        if !matches_selected_data_source(&data_source.name) {
            continue;
        }
        updated_data_sources.push(add_secrets(data_source)?);
    }

    Ok(updated_data_sources)
}

fn matches_selected_data_source(data_source_name: &str) -> bool {
    let schema_specific: Vec<String> = env::vars()
        .filter_map(|(key, value)| {
            if key.starts_with("APP_")
                && key.ends_with("_DATA_SOURCE_NAME")
                && key != "APP_DATA_SOURCE_NAME"
            {
                let value = value.trim();
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
            None
        })
        .collect();

    if !schema_specific.is_empty() {
        return schema_specific
            .iter()
            .any(|selected| data_source_name.eq_ignore_ascii_case(selected));
    }

    if let Ok(selected) = env::var("APP_DATA_SOURCE_NAME") {
        let selected = selected.trim();
        return selected.is_empty() || data_source_name.eq_ignore_ascii_case(selected);
    }

    true
}

pub fn get_configured_data_sources() -> Result<Vec<DataSource>> {
    let data_sources_path = get_data_sources_path()?;
    let file = File::open(&data_sources_path).map_err(|e| {
        error::missing_config(format!(
            "could not open data source config `{}`: {e}",
            data_sources_path.display()
        ))
    })?;
    let reader = BufReader::new(file);
    serde_yaml::from_reader(reader).map_err(|e| {
        error::data(format!(
            "could not parse data source config `{}`: {e}",
            data_sources_path.display()
        ))
    })
}

fn add_secrets(mut data_source: DataSource) -> Result<DataSource> {
    let secrets = EnvSecretProvider;
    let env_name = secrets
        .require_secret("ENV_NAME")
        .map_err(map_secret_error)?;
    let data_source_type = data_source.data_source_type;
    let se = selected_environment(&mut data_source, &env_name)?;

    match data_source_type {
        DataSourceType::PostgreSQL => {
            // TODO: Get from secrets manager
            let svc_account_name = secrets
                .require_secret("PG_SERVICE_ACCOUNT_NAME")
                .map_err(map_secret_error)?;
            let svc_account_pass = secrets
                .require_secret("PG_SERVICE_ACCOUNT_PASS")
                .map_err(map_secret_error)?;
            //

            se.service_account_name = Some(svc_account_name);
            se.service_account_password = Some(svc_account_pass);
        }
        DataSourceType::MsSqlServer => {
            apply_mssql_family_secrets(
                se,
                appfw_mssql_auth::PROVIDER_MSSQL,
                &secrets,
                map_secret_error,
            )?;
        }
        DataSourceType::FabricSqlAnalytics => {
            apply_mssql_family_secrets(
                se,
                appfw_mssql_auth::PROVIDER_FABRIC,
                &secrets,
                map_secret_error,
            )?;
        }
        DataSourceType::MongoDB => {
            // TODO: Get from secrets manager
            let svc_account_name = secrets
                .require_secret("MONGO_SERVICE_ACCOUNT_NAME")
                .map_err(map_secret_error)?;
            let svc_account_pass = secrets
                .require_secret("MONGO_SERVICE_ACCOUNT_PASS")
                .map_err(map_secret_error)?;
            //

            se.service_account_name = Some(svc_account_name);
            se.service_account_password = Some(svc_account_pass);
        }
        DataSourceType::Snowflake => {
            se.service_account_name = secrets
                .get_secret("SNOWFLAKE_SERVICE_ACCOUNT_NAME")
                .map_err(map_secret_error)?;
            se.service_account_password = first_secret(
                &secrets,
                &[
                    "SNOWFLAKE_ACCESS_TOKEN",
                    "SNOWFLAKE_OAUTH_TOKEN",
                    "SNOWFLAKE_JWT",
                    "SNOWFLAKE_SERVICE_ACCOUNT_PASS",
                ],
            )?;
        }
        DataSourceType::Neo4j => {
            let svc_account_name = secrets
                .require_secret("NEO4J_SERVICE_ACCOUNT_NAME")
                .map_err(map_secret_error)?;
            let svc_account_pass = secrets
                .require_secret("NEO4J_SERVICE_ACCOUNT_PASS")
                .map_err(map_secret_error)?;

            se.service_account_name = Some(svc_account_name);
            se.service_account_password = Some(svc_account_pass);
        }
        DataSourceType::ServiceNow
        | DataSourceType::Workday
        | DataSourceType::Icims
        | DataSourceType::Salesforce
        | DataSourceType::Anaplan
        | DataSourceType::OracleFinancials => {}
    }

    if !is_external_api_provider(data_source_type) {
        validate_environment_security(data_source_type, &env_name, se)?;
    }

    // Replace environments vec with the active environment only.
    data_source.environments = vec![se.clone()];

    Ok(data_source)
}

fn first_secret(secrets: &impl SecretProvider, names: &[&str]) -> Result<Option<String>> {
    for name in names {
        if let Some(value) = secrets.get_secret(name).map_err(map_secret_error)? {
            return Ok(Some(value));
        }
    }
    Ok(None)
}

fn is_external_api_provider(data_source_type: DataSourceType) -> bool {
    matches!(
        data_source_type,
        DataSourceType::ServiceNow
            | DataSourceType::Workday
            | DataSourceType::Icims
            | DataSourceType::Salesforce
            | DataSourceType::Anaplan
            | DataSourceType::OracleFinancials
    )
}

fn get_data_sources_path() -> Result<PathBuf> {
    Ok(get_pkg_path()?.join("data_sources.yaml"))
}

pub fn get_schema_path(schema: &Schema) -> Result<PathBuf> {
    Ok(get_pkg_path()?.join("schemas").join(schema.name.clone()))
}

pub fn get_pkg_path() -> Result<PathBuf> {
    let curr_dir = env::current_dir()?;
    Ok(curr_dir.join("_pkg"))
}

pub fn get_migrations_path() -> Result<PathBuf> {
    Ok(get_pkg_path()?.join("migrations"))
}

fn apply_schema_data_source_override(mut schema: Schema) -> Schema {
    let schema_key = schema.name.to_ascii_uppercase().replace('-', "_");
    let specific_key = format!("APP_{}_DATA_SOURCE_NAME", schema_key);
    let fallback_key = "APP_DATA_SOURCE_NAME";

    if let Ok(data_source_name) = env::var(&specific_key).or_else(|_| env::var(fallback_key)) {
        if !data_source_name.trim().is_empty() {
            schema.data_source_name = data_source_name;
        }
    }

    schema
}

fn selected_environment<'a>(
    data_source: &'a mut DataSource,
    env_name: &str,
) -> Result<&'a mut crate::data_source::DataSourceEnvironment> {
    let data_source_name = data_source.name.clone();
    data_source
        .environments
        .iter_mut()
        .find(|env| env.name == env_name)
        .ok_or_else(|| {
            error::config(format!(
                "data source `{data_source_name}` has no environment named `{env_name}`"
            ))
        })
}

fn validate_environment_security(
    data_source_type: DataSourceType,
    env_name: &str,
    env: &crate::data_source::DataSourceEnvironment,
) -> Result<()> {
    connection_security::validate(
        provider_for(data_source_type),
        env_name,
        &env.security_profile,
        &env.tls_mode,
        &env.db_host,
    )
    .map(|_| ())
    .map_err(|e| error::config(format!("invalid data source connection security: {e}")))
}

fn provider_for(data_source_type: DataSourceType) -> Provider {
    match data_source_type {
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
            unreachable!("external API providers return before connection-security validation")
        }
    }
}

fn map_secret_error(error: SecretError) -> std::io::Error {
    match error {
        SecretError::Missing { name } => error::missing_env(&name, "load database configuration"),
        SecretError::Read { name, message } => error::config(format!(
            "failed to read secret `{name}` from configured secret provider: {message}"
        )),
    }
}

fn apply_mssql_family_secrets(
    se: &mut crate::data_source::DataSourceEnvironment,
    provider: &str,
    secrets: &impl SecretProvider,
    map_err: fn(SecretError) -> std::io::Error,
) -> Result<()> {
    let auth_mode_key = appfw_mssql_auth::auth_mode_env_key(provider).ok_or_else(|| {
        error::config(format!(
            "unsupported MS SQL family provider `{provider}` for secret loading"
        ))
    })?;
    let default_mode = appfw_mssql_auth::default_mode_for(provider).ok_or_else(|| {
        error::config(format!(
            "unsupported MS SQL family provider `{provider}` for secret loading"
        ))
    })?;
    let auth_mode = secrets
        .get_secret(auth_mode_key)
        .map_err(map_err)?
        .or_else(|| se.auth_mode.clone())
        .unwrap_or_else(|| default_mode.to_string());
    let mode = appfw_mssql_auth::resolve_mode(provider, Some(auth_mode.as_str()))
        .map_err(|e| error::config(e.message))?;
    se.auth_mode = Some(mode.to_string());

    let bindings =
        appfw_mssql_auth::secret_bindings(provider, mode).map_err(|e| error::config(e.message))?;

    if let Some(scope_key) = bindings.entra_token_scope {
        se.entra_token_scope = secrets
            .get_secret(scope_key)
            .map_err(map_err)?
            .or_else(|| se.entra_token_scope.clone());
    }

    if let Some(tenant) = bindings.entra_tenant_id {
        se.entra_tenant_id = match tenant.presence {
            FieldPresence::Required => {
                Some(secrets.require_secret(tenant.env_key).map_err(map_err)?)
            }
            FieldPresence::Optional => secrets.get_secret(tenant.env_key).map_err(map_err)?,
        };
    }

    se.service_account_name = match bindings.service_account_name.presence {
        FieldPresence::Required => Some(
            secrets
                .require_secret(bindings.service_account_name.env_key)
                .map_err(map_err)?,
        ),
        FieldPresence::Optional => secrets
            .get_secret(bindings.service_account_name.env_key)
            .map_err(map_err)?,
    };
    se.service_account_password = match bindings.service_account_password.presence {
        FieldPresence::Required => Some(
            secrets
                .require_secret(bindings.service_account_password.env_key)
                .map_err(map_err)?,
        ),
        FieldPresence::Optional => secrets
            .get_secret(bindings.service_account_password.env_key)
            .map_err(map_err)?,
    };
    Ok(())
}
