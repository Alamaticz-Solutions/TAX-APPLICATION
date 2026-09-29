use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::BufReader,
    path::{Path, PathBuf},
};

use appfw_runtime::{
    connection_security::{self, Provider},
    secrets::{EnvSecretProvider, SecretError, SecretProvider},
};
use appfw_mssql_auth::{self, FieldPresence};

use crate::{
    routes::app_error::{AppError, ConfigError},
    schemas::system::{DataSource, DataSourceType, EntityType, Schema},
};
use tracing::{debug, info};

#[tracing::instrument(skip_all)]
pub async fn read_config() -> Result<
    (
        Vec<DataSource>,
        Vec<Schema>,
        Vec<EntityType>,
        HashMap<String, regorus::Engine>,
    ),
    AppError,
> {
    info!("loading backend configuration");

    let curr_dir = env::current_dir()
        .map_err(|e| ConfigError::Load(format!("could not get current dir: {}", e)))?;

    let config_dir = curr_dir.join("config/generated");
    let data_sources = get_data_sources(&config_dir)?;

    let mut schemas: Vec<Schema> = Vec::new();
    let mut entity_types: Vec<EntityType> = Vec::new();
    let mut access_policies: HashMap<String, regorus::Engine> = HashMap::new();

    let schemas_dir = config_dir.join("schemas");
    for dir_entry in fs::read_dir(&schemas_dir).map_err(|e| io_error(&schemas_dir, e))? {
        // println!("    read_config dir_entry -> {:?}", &dir_entry);
        let schema_dir = dir_entry.map_err(|e| io_error(&schemas_dir, e))?.path();
        if schema_dir.is_dir() {
            let schema = get_schema(&schema_dir)?;
            // println!("    schema -> {:?}", &schema);
            schemas.push(schema);

            let schema_entity_types = get_entity_types(&schema_dir)?;
            entity_types.extend(schema_entity_types);

            let schema_access_policies = get_access_policies(&schema_dir)?;
            for (key, policy) in schema_access_policies {
                access_policies.insert(key, policy);
            }
        }
    }
    info!(
        data_sources = data_sources.len(),
        schemas = schemas.len(),
        entity_types = entity_types.len(),
        access_policies = access_policies.len(),
        "loaded backend configuration"
    );

    Ok((data_sources, schemas, entity_types, access_policies))
}

fn get_schema(schema_dir: &PathBuf) -> Result<Schema, AppError> {
    // Open the file
    let path = schema_dir.join("schema.yaml");
    let file = File::open(&path).map_err(|e| io_error(&path, e))?;
    let reader = BufReader::new(file);
    let schema: Schema = serde_yaml::from_reader(reader).map_err(|e| parse_error(&path, e))?;

    Ok(apply_schema_data_source_override(schema))
}

fn get_data_sources(config_dir: &PathBuf) -> Result<Vec<DataSource>, AppError> {
    // Open the file
    let path = config_dir.join("data_sources.yaml");
    let file = File::open(&path).map_err(|e| io_error(&path, e))?;
    let reader = BufReader::new(file);
    let data_sources: Vec<DataSource> =
        serde_yaml::from_reader(reader).map_err(|e| parse_error(&path, e))?;

    let mut updated_data_sources = vec![];
    for data_source in data_sources {
        updated_data_sources.push(add_secrets(data_source)?);
    }

    Ok(updated_data_sources)
}

#[tracing::instrument(
  skip(data_source),
  fields(data_source = %data_source.name, data_source_type = ?data_source.data_source_type)
)]
fn add_secrets(mut data_source: DataSource) -> Result<DataSource, AppError> {
    let secrets = EnvSecretProvider;
    let env_name = secrets
        .require_secret("ENV_NAME")
        .map_err(map_secret_error)?;
    debug!(environment = %env_name, "loading credentials for active data source environment");
    let data_source_type = data_source.data_source_type;

    let se = data_source
        .environments
        .iter_mut()
        .find(|env| env.name == env_name)
        .ok_or_else(|| ConfigError::MissingConfiguredEnvironment {
            data_source_name: data_source.name.clone(),
            environment_name: env_name.clone(),
        })?;

    // TODO: Get from secrets manager.
    match data_source_type {
        DataSourceType::PostgreSQL => {
            se.service_account_name = Some(
                secrets
                    .require_secret("PG_SERVICE_ACCOUNT_NAME")
                    .map_err(map_secret_error)?,
            );
            se.service_account_password = Some(
                secrets
                    .require_secret("PG_SERVICE_ACCOUNT_PASS")
                    .map_err(map_secret_error)?,
            );
        }
        DataSourceType::MongoDB => {
            se.service_account_name = Some(
                secrets
                    .require_secret("MONGO_SERVICE_ACCOUNT_NAME")
                    .map_err(map_secret_error)?,
            );
            se.service_account_password = Some(
                secrets
                    .require_secret("MONGO_SERVICE_ACCOUNT_PASS")
                    .map_err(map_secret_error)?,
            );
        }
        DataSourceType::MsSqlServer => {
            apply_mssql_family_secrets(
                se,
                appfw_mssql_auth::PROVIDER_MSSQL,
                &secrets,
                map_secret_error,
                true,
            )?;
        }
        DataSourceType::FabricSqlAnalytics => {
            apply_mssql_family_secrets(
                se,
                appfw_mssql_auth::PROVIDER_FABRIC,
                &secrets,
                map_secret_error,
                true,
            )?;
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
            se.service_account_name = Some(
                secrets
                    .require_secret("NEO4J_SERVICE_ACCOUNT_NAME")
                    .map_err(map_secret_error)?,
            );
            se.service_account_password = Some(
                secrets
                    .require_secret("NEO4J_SERVICE_ACCOUNT_PASS")
                    .map_err(map_secret_error)?,
            );
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

    debug!(
      environment = %env_name,
      "configured active data source environment"
    );

    Ok(data_source)
}

fn first_secret(secrets: &impl SecretProvider, names: &[&str]) -> Result<Option<String>, AppError> {
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

fn map_secret_error(error: SecretError) -> AppError {
    match error {
        SecretError::Missing { name } => ConfigError::MissingEnvVar { name }.into(),
        SecretError::Read { name, message } => ConfigError::Load(format!(
            "failed to read secret `{name}` from configured secret provider: {message}"
        ))
        .into(),
    }
}

fn apply_mssql_family_secrets(
    se: &mut crate::schemas::system::DataSourceEnvironment,
    provider: &str,
    secrets: &impl SecretProvider,
    map_err: fn(SecretError) -> AppError,
    model_strict: bool,
) -> Result<(), AppError> {
    let auth_mode_key = appfw_mssql_auth::auth_mode_env_key(provider).ok_or_else(|| {
        ConfigError::Load(format!(
            "unsupported MS SQL family provider `{provider}` for secret loading"
        ))
    })?;
    let default_mode = appfw_mssql_auth::default_mode_for(provider).ok_or_else(|| {
        ConfigError::Load(format!(
            "unsupported MS SQL family provider `{provider}` for secret loading"
        ))
    })?;
    let auth_mode = secrets
        .get_secret(auth_mode_key)
        .map_err(map_err)?
        .or_else(|| se.auth_mode.clone())
        .unwrap_or_else(|| default_mode.to_string());
    if model_strict {
        appfw_mssql_auth::validate_model_mode_for_provider(provider, &auth_mode)
            .map_err(|e| ConfigError::Load(e.message))?;
    }
    let mode = appfw_mssql_auth::resolve_mode(provider, Some(auth_mode.as_str()))
        .map_err(|e| ConfigError::Load(e.message))?;
    se.auth_mode = Some(mode.to_string());

    let bindings = appfw_mssql_auth::secret_bindings(provider, mode)
        .map_err(|e| ConfigError::Load(e.message))?;

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

fn validate_environment_security(
    data_source_type: DataSourceType,
    env_name: &str,
    env: &crate::schemas::system::DataSourceEnvironment,
) -> Result<(), AppError> {
    connection_security::validate(
        provider_for(data_source_type),
        env_name,
        &env.security_profile,
        &env.tls_mode,
        &env.db_host,
    )
    .map(|_| ())
    .map_err(|e| ConfigError::Load(format!("invalid data source connection security: {e}")).into())
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

fn get_entity_types(schema_dir: &PathBuf) -> Result<Vec<EntityType>, AppError> {
    // Open the file
    let path = schema_dir.join("entity_types.yaml");
    let file = File::open(&path).map_err(|e| io_error(&path, e))?;
    let reader = BufReader::new(file);
    let entity_types: Vec<EntityType> =
        serde_yaml::from_reader(reader).map_err(|e| parse_error(&path, e))?;
    Ok(entity_types)
}

fn get_access_policies(schema_dir: &PathBuf) -> Result<HashMap<String, regorus::Engine>, AppError> {
    let mut access_policies: HashMap<String, regorus::Engine> = HashMap::new();

    let schema_name = schema_dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| ConfigError::InvalidPath {
            path: display_path(schema_dir),
            message: "schema directory name must be valid UTF-8".to_string(),
        })?
        .to_string();

    for dir_entry in fs::read_dir(schema_dir).map_err(|e| io_error(schema_dir, e))? {
        let entry_path = dir_entry.map_err(|e| io_error(schema_dir, e))?.path();
        if !entry_path.is_dir() {
            let is_rego = entry_path
                .extension()
                .and_then(|extension| extension.to_str())
                .map(|extension| extension.eq_ignore_ascii_case("rego"))
                .unwrap_or(false);
            if is_rego {
                // Get the name of the rego file
                let entity_type_name = entry_path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .ok_or_else(|| ConfigError::InvalidPath {
                        path: display_path(&entry_path),
                        message: "policy file name must be valid UTF-8".to_string(),
                    })?; // Should be in snake_1 form.
                let policy = load_policy(schema_dir, entity_type_name.to_string())?;
                let key = format!("{}.{}", &schema_name, entity_type_name.to_owned());
                info!(policy_key = %key, "loaded access policy");
                access_policies.insert(key, policy);
            }
        }
    }

    Ok(access_policies)
}

fn load_policy(schema_dir: &PathBuf, policy_name: String) -> Result<regorus::Engine, AppError> {
    debug!(policy_name = %policy_name, "loading access policy");

    let rego_file = schema_dir.join(format!("{policy_name}.rego"));

    let mut engine = regorus::Engine::new();
    let _package =
        engine
            .add_policy_from_file(&rego_file)
            .map_err(|e| ConfigError::PolicyLoad {
                path: display_path(&rego_file),
                message: e.to_string(),
            })?;

    // println!("package name {}", _package);

    Ok(engine)
}

fn io_error(path: &Path, error: std::io::Error) -> AppError {
    ConfigError::Io {
        path: display_path(path),
        message: error.to_string(),
    }
    .into()
}

fn parse_error(path: &Path, error: serde_yaml::Error) -> AppError {
    ConfigError::Parse {
        path: display_path(path),
        message: error.to_string(),
    }
    .into()
}

fn display_path(path: &Path) -> String {
    path.display().to_string()
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
