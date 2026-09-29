use appfw_runtime::{
    connection_security::{ConnectionSecurity, TlsMode},
    ConfigError, RuntimeError,
};

use crate::auth::{
    fetch_entra_access_token, MssqlAuthConfig, FABRIC_SQL_ANALYTICS_DEFAULT_TOKEN_SCOPE,
};
use crate::odbc::{resolve_odbc_driver_name, FREETDS_DRIVER};

pub const MSSQL_POOL_MAX_SIZE: u32 = 10;

const FREETDS_TDS_VERSION: &str = "7.4";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MssqlConnectionConfig {
    pub host: String,
    pub port: String,
    pub database: String,
    pub auth: MssqlAuthConfig,
}

impl MssqlConnectionConfig {
    pub fn new(
        host: impl Into<String>,
        port: impl Into<String>,
        database: impl Into<String>,
        username: Option<String>,
        password: Option<String>,
    ) -> Self {
        Self {
            host: host.into(),
            port: port.into(),
            database: database.into(),
            auth: MssqlAuthConfig::SqlPassword { username, password },
        }
    }

    pub fn new_ntlm(
        host: impl Into<String>,
        port: impl Into<String>,
        database: impl Into<String>,
        username: Option<String>,
        password: Option<String>,
    ) -> Self {
        Self {
            host: host.into(),
            port: port.into(),
            database: database.into(),
            auth: MssqlAuthConfig::Ntlm { username, password },
        }
    }

    pub fn new_entra_access_token(
        host: impl Into<String>,
        port: impl Into<String>,
        database: impl Into<String>,
        access_token: impl Into<String>,
    ) -> Self {
        Self {
            host: host.into(),
            port: port.into(),
            database: database.into(),
            auth: MssqlAuthConfig::EntraAccessToken {
                access_token: access_token.into(),
            },
        }
    }

    pub fn new_entra_client_credentials(
        host: impl Into<String>,
        port: impl Into<String>,
        database: impl Into<String>,
        tenant_id: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        token_scope: Option<String>,
    ) -> Self {
        Self {
            host: host.into(),
            port: port.into(),
            database: database.into(),
            auth: MssqlAuthConfig::EntraClientCredentials {
                tenant_id: tenant_id.into(),
                client_id: client_id.into(),
                client_secret: client_secret.into(),
                token_scope,
            },
        }
    }

    /// Build connection config from loaded data-source environment fields.
    #[allow(clippy::too_many_arguments)]
    pub fn from_environment(
        provider: &str,
        host: impl Into<String>,
        port: impl Into<String>,
        database: impl Into<String>,
        auth_mode: Option<&str>,
        service_account_name: Option<String>,
        service_account_password: Option<String>,
        entra_tenant_id: Option<String>,
        entra_token_scope: Option<String>,
    ) -> Result<Self, RuntimeError> {
        let mode = appfw_mssql_auth::resolve_mode(provider, auth_mode)
            .map_err(|e| RuntimeError::Config(ConfigError::Load(e.message)))?;
        let host = host.into();
        let port = port.into();
        let database = database.into();
        match mode {
            appfw_mssql_auth::SQL_PASSWORD => Ok(Self::new(
                host,
                port,
                database,
                service_account_name,
                service_account_password,
            )),
            appfw_mssql_auth::NTLM => Ok(Self::new_ntlm(
                host,
                port,
                database,
                service_account_name,
                service_account_password,
            )),
            appfw_mssql_auth::ENTRA_ACCESS_TOKEN => {
                let access_token = service_account_password.ok_or_else(|| {
                    RuntimeError::Config(ConfigError::Load(format!(
                        "{provider} auth_mode `{mode}` requires an access token secret"
                    )))
                })?;
                Ok(Self::new_entra_access_token(
                    host,
                    port,
                    database,
                    access_token,
                ))
            }
            appfw_mssql_auth::ENTRA_CLIENT_CREDENTIALS => {
                let tenant_id = entra_tenant_id.ok_or_else(|| {
                    RuntimeError::Config(ConfigError::Load(format!(
                        "{provider} auth_mode `{mode}` requires FABRIC_TENANT_ID"
                    )))
                })?;
                let client_id = service_account_name.ok_or_else(|| {
                    RuntimeError::Config(ConfigError::Load(format!(
                        "{provider} auth_mode `{mode}` requires FABRIC_CLIENT_ID"
                    )))
                })?;
                let client_secret = service_account_password.ok_or_else(|| {
                    RuntimeError::Config(ConfigError::Load(format!(
                        "{provider} auth_mode `{mode}` requires FABRIC_CLIENT_SECRET"
                    )))
                })?;
                Ok(Self::new_entra_client_credentials(
                    host,
                    port,
                    database,
                    tenant_id,
                    client_id,
                    client_secret,
                    entra_token_scope,
                ))
            }
            other => Err(RuntimeError::Config(ConfigError::Load(format!(
                "unsupported auth_mode `{other}` for provider `{provider}`"
            )))),
        }
    }
}

/// Build a DSN-less ODBC connection string for plain MsSqlServer auth modes.
pub fn mssql_odbc_connection_string(
    connection: &MssqlConnectionConfig,
    security: &ConnectionSecurity,
) -> Result<String, RuntimeError> {
    match &connection.auth {
        MssqlAuthConfig::SqlPassword { username, password } => {
            ms_driver18_connection_string(
                connection,
                security,
                username.clone().unwrap_or_default(),
                password.clone().unwrap_or_default(),
            )
        }
        MssqlAuthConfig::Ntlm { username, password } => {
            let username = username.clone().unwrap_or_default();
            let password = password.clone().unwrap_or_default();
            if username.is_empty() || password.is_empty() {
                return Err(RuntimeError::Config(ConfigError::Load(
                    "MS SQL ntlm auth requires domain Windows username and password".to_string(),
                )));
            }
            freetds_ntlm_connection_string(connection, &username, &password)
        }
        MssqlAuthConfig::EntraAccessToken { .. }
        | MssqlAuthConfig::EntraClientCredentials { .. } => Err(RuntimeError::Config(
            ConfigError::Load(
                "plain MsSqlServer ODBC connection strings do not support Entra auth; use FabricSqlAnalytics"
                    .to_string(),
            ),
        )),
    }
}

pub fn mssql_client_config(
    connection: &MssqlConnectionConfig,
    security: &ConnectionSecurity,
) -> Result<String, RuntimeError> {
    mssql_odbc_connection_string(connection, security)
}

pub async fn mssql_client_config_resolving_auth(
    connection: &MssqlConnectionConfig,
    security: &ConnectionSecurity,
) -> Result<String, RuntimeError> {
    let resolved = mssql_connection_config_resolving_auth(connection).await?;
    mssql_odbc_connection_string(&resolved, security)
}

pub async fn mssql_connection_config_resolving_auth(
    connection: &MssqlConnectionConfig,
) -> Result<MssqlConnectionConfig, RuntimeError> {
    Ok(match &connection.auth {
        MssqlAuthConfig::EntraClientCredentials {
            tenant_id,
            client_id,
            client_secret,
            token_scope,
        } => {
            let token = fetch_entra_access_token(
                tenant_id,
                client_id,
                client_secret,
                token_scope
                    .as_deref()
                    .unwrap_or(FABRIC_SQL_ANALYTICS_DEFAULT_TOKEN_SCOPE),
            )
            .await?;
            MssqlConnectionConfig::new_entra_access_token(
                connection.host.clone(),
                connection.port.clone(),
                connection.database.clone(),
                token.into_secret(),
            )
        }
        _ => connection.clone(),
    })
}

fn ms_driver18_connection_string(
    connection: &MssqlConnectionConfig,
    security: &ConnectionSecurity,
    username: String,
    password: String,
) -> Result<String, RuntimeError> {
    let driver = resolve_odbc_driver_name();
    let port = parse_port("SQL Server", &connection.port)?;
    let encrypt = if mssql_encrypt(security) { "yes" } else { "no" };
    let trust = if security.allows_unvalidated_certificate() {
        "yes"
    } else {
        "no"
    };
    Ok(format!(
        "Driver={driver};Server=tcp:{host},{port};Database={database};UID={uid};PWD={pwd};Encrypt={encrypt};TrustServerCertificate={trust};Connection Timeout=30;MARS_Connection=no;",
        driver = odbc_braced(&driver),
        host = connection.host,
        port = port,
        database = odbc_braced(&connection.database),
        uid = odbc_braced(&username),
        pwd = odbc_braced(&password),
        encrypt = encrypt,
        trust = trust,
    ))
}

fn freetds_ntlm_connection_string(
    connection: &MssqlConnectionConfig,
    username: &str,
    password: &str,
) -> Result<String, RuntimeError> {
    let port = parse_port("SQL Server", &connection.port)?;
    Ok(format!(
        "Driver={driver};Server={host};Port={port};Database={database};UID={uid};PWD={pwd};TDS_Version={tds};ClientCharset=UTF-8;UseNTLMv2=Yes;Trusted_Connection=No;Connection Timeout=30;",
        driver = odbc_braced(FREETDS_DRIVER),
        host = connection.host,
        port = port,
        database = odbc_braced(&connection.database),
        uid = odbc_braced(username),
        pwd = odbc_braced(password),
        tds = FREETDS_TDS_VERSION,
    ))
}

fn mssql_encrypt(security: &ConnectionSecurity) -> bool {
    !matches!(security.tls_mode, TlsMode::Disabled)
}

fn odbc_braced(value: &str) -> String {
    format!("{{{}}}", value.replace('}', "}}"))
}

fn parse_port(provider: &str, port: &str) -> Result<u16, RuntimeError> {
    port.parse::<u16>().map_err(|e| {
        RuntimeError::Config(ConfigError::Load(format!(
            "invalid {provider} port '{}': {}",
            port, e
        )))
    })
}

#[cfg(test)]
mod tests {
    use appfw_runtime::connection_security::{
        ConnectionSecurity, Provider, SecurityProfile, TlsMode,
    };

    use super::*;
    use crate::odbc::MS_ODBC_DRIVER_18;

    fn security(tls_mode: TlsMode) -> ConnectionSecurity {
        ConnectionSecurity {
            provider: Provider::MsSqlServer,
            security_profile: SecurityProfile::LocalDev,
            tls_mode,
        }
    }

    #[test]
    fn builds_sql_password_odbc_connection_string() {
        let connection = MssqlConnectionConfig::new(
            "mssql.local",
            "1433",
            "crm",
            Some("user".to_string()),
            Some("secret".to_string()),
        );

        let conn = mssql_odbc_connection_string(&connection, &security(TlsMode::Disabled))
            .expect("connection string");

        assert!(conn.contains(&format!("Driver={{{MS_ODBC_DRIVER_18}}}")));
        assert!(conn.contains("Server=tcp:mssql.local,1433"));
        assert!(conn.contains("UID={user}"));
        assert!(conn.contains("Encrypt=no"));
    }

    #[test]
    fn odbc_braced_escapes_special_password_chars() {
        let connection = MssqlConnectionConfig::new(
            "mssql.local",
            "1433",
            "crm",
            Some("user".to_string()),
            Some("p;w}rd".to_string()),
        );

        let conn = mssql_odbc_connection_string(&connection, &security(TlsMode::Disabled))
            .expect("connection string");

        assert!(conn.contains("PWD={p;w}}rd}"));
        assert!(conn.contains("UID={user}"));
    }

    #[test]
    fn builds_ntlm_freetds_connection_string() {
        let connection = MssqlConnectionConfig::new_ntlm(
            "mssql.appfw.test",
            "1433",
            "master",
            Some("APPFW\\svc-app".to_string()),
            Some("secret".to_string()),
        );

        let conn = mssql_odbc_connection_string(&connection, &security(TlsMode::Prefer))
            .expect("connection string");

        assert!(conn.contains(&format!("Driver={{{FREETDS_DRIVER}}}")));
        assert!(conn.contains("Server=mssql.appfw.test"));
        assert!(conn.contains("UID={APPFW\\svc-app}"));
        assert!(conn.contains("UseNTLMv2=Yes"));
        assert!(conn.contains("Trusted_Connection=No"));
        assert!(conn.contains(&format!("TDS_Version={FREETDS_TDS_VERSION}")));
    }

    #[test]
    fn rejects_invalid_mssql_ports() {
        let connection = MssqlConnectionConfig::new("mssql.local", "bad", "crm", None, None);
        let err = mssql_odbc_connection_string(&connection, &security(TlsMode::Disabled))
            .expect_err("invalid port");

        assert!(
            matches!(err, RuntimeError::Config(ConfigError::Load(message)) if message.contains("invalid SQL Server port"))
        );
    }

    #[test]
    fn entra_client_credentials_require_async_resolution() {
        let connection = MssqlConnectionConfig::new_entra_client_credentials(
            "fabric-host.example.invalid",
            "1433",
            "fabric_database",
            "tenant",
            "client",
            "secret",
            None,
        );

        let err = mssql_odbc_connection_string(&connection, &security(TlsMode::Require))
            .expect_err("entra not for plain mssql");

        assert!(
            matches!(err, RuntimeError::Config(ConfigError::Load(message)) if message.contains("do not support Entra"))
        );
    }
}
