use std::time::Duration;

use appfw_runtime::{connection_security::ConnectionSecurity, ConfigError, RuntimeError};
use mongodb::options::{
    ClientOptions, ConnectionString, Credential, HostInfo, ServerAddress, Tls, TlsOptions,
};

pub const DEFAULT_APP_NAME: &str = "app_framework";
pub const DEFAULT_AUTH_SOURCE: &str = "admin";
pub const DEFAULT_MAX_POOL_SIZE: u32 = 50;
pub const DEFAULT_MIN_POOL_SIZE: u32 = 5;
pub const DEFAULT_MAX_IDLE_TIME: Duration = Duration::from_secs(30);
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
pub const DEFAULT_SERVER_SELECTION_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MongoConnectionConfig {
    pub host: String,
    pub port: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub auth_source: String,
    pub app_name: String,
    pub max_pool_size: u32,
    pub min_pool_size: u32,
    pub max_idle_time: Duration,
    pub connect_timeout: Duration,
    pub server_selection_timeout: Duration,
}

impl MongoConnectionConfig {
    pub fn new(
        host: impl Into<String>,
        port: impl Into<String>,
        username: Option<String>,
        password: Option<String>,
    ) -> Self {
        Self {
            host: host.into(),
            port: port.into(),
            username,
            password,
            auth_source: DEFAULT_AUTH_SOURCE.to_string(),
            app_name: DEFAULT_APP_NAME.to_string(),
            max_pool_size: DEFAULT_MAX_POOL_SIZE,
            min_pool_size: DEFAULT_MIN_POOL_SIZE,
            max_idle_time: DEFAULT_MAX_IDLE_TIME,
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            server_selection_timeout: DEFAULT_SERVER_SELECTION_TIMEOUT,
        }
    }
}

pub async fn mongo_client_options(
    config: &MongoConnectionConfig,
    security: &ConnectionSecurity,
) -> Result<ClientOptions, RuntimeError> {
    if is_mongo_connection_uri(&config.host) {
        return mongo_client_options_from_uri(config, security).await;
    }

    mongo_client_options_from_host_port(config, security)
}

async fn mongo_client_options_from_uri(
    config: &MongoConnectionConfig,
    security: &ConnectionSecurity,
) -> Result<ClientOptions, RuntimeError> {
    let parsed = parse_connection_string(&config.host)?;
    reject_embedded_credentials(&parsed)?;

    let mut options = ClientOptions::parse(&config.host)
        .await
        .map_err(|e| config_error(format!("invalid MongoDB connection URI: {e}")))?;

    apply_framework_options(&mut options, config, security);
    Ok(options)
}

fn mongo_client_options_from_host_port(
    config: &MongoConnectionConfig,
    security: &ConnectionSecurity,
) -> Result<ClientOptions, RuntimeError> {
    let addresses = vec![ServerAddress::Tcp {
        host: config.host.clone(),
        port: Some(parse_port(&config.port)?),
    }];

    let mut options = ClientOptions::builder()
        .hosts(addresses)
        .max_pool_size(Some(config.max_pool_size))
        .min_pool_size(Some(config.min_pool_size))
        .max_idle_time(Some(config.max_idle_time))
        .connect_timeout(Some(config.connect_timeout))
        .server_selection_timeout(Some(config.server_selection_timeout))
        .tls(Some(mongo_tls_options(security)))
        .app_name(config.app_name.clone())
        .build();
    options.credential = config_credential(config);

    Ok(options)
}

fn apply_framework_options(
    options: &mut ClientOptions,
    config: &MongoConnectionConfig,
    security: &ConnectionSecurity,
) {
    options.max_pool_size = Some(config.max_pool_size);
    options.min_pool_size = Some(config.min_pool_size);
    options.max_idle_time = Some(config.max_idle_time);
    options.connect_timeout = Some(config.connect_timeout);
    options.server_selection_timeout = Some(config.server_selection_timeout);
    options.app_name = Some(config.app_name.clone());
    options.credential = config_credential(config);
    if security.requires_tls() || options.tls.is_none() {
        options.tls = Some(mongo_tls_options(security));
    }
}

fn config_credential(config: &MongoConnectionConfig) -> Option<Credential> {
    if config.username.is_none() && config.password.is_none() {
        return None;
    }

    Some(
        Credential::builder()
            .username(config.username.clone())
            .password(config.password.clone())
            .source(config.auth_source.clone())
            .build(),
    )
}

pub fn mongo_connection_summary(host: &str) -> Result<MongoConnectionSummary, RuntimeError> {
    if !is_mongo_connection_uri(host) {
        return Ok(MongoConnectionSummary {
            mode: MongoConnectionMode::HostPort,
            hosts: vec![host.to_string()],
            replica_set: None,
        });
    }

    let parsed = parse_connection_string(host)?;
    reject_embedded_credentials(&parsed)?;
    let (mode, hosts) = match parsed.host_info {
        HostInfo::HostIdentifiers(addresses) => (
            MongoConnectionMode::ConnectionString,
            addresses
                .into_iter()
                .map(|address| match address {
                    ServerAddress::Tcp { host, port } => {
                        port.map(|port| format!("{host}:{port}")).unwrap_or(host)
                    }
                    #[cfg(unix)]
                    ServerAddress::Unix { path } => path.display().to_string(),
                    _ => format!("{address:?}"),
                })
                .collect(),
        ),
        HostInfo::DnsRecord(hostname) => (MongoConnectionMode::Srv, vec![hostname]),
        _ => {
            return Err(config_error(
                "unsupported MongoDB connection URI host shape".to_string(),
            ))
        }
    };

    Ok(MongoConnectionSummary {
        mode,
        hosts,
        replica_set: parsed.replica_set,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MongoConnectionSummary {
    pub mode: MongoConnectionMode,
    pub hosts: Vec<String>,
    pub replica_set: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MongoConnectionMode {
    HostPort,
    ConnectionString,
    Srv,
}

fn parse_connection_string(host: &str) -> Result<ConnectionString, RuntimeError> {
    ConnectionString::try_from(host)
        .map_err(|e| config_error(format!("invalid MongoDB connection URI: {e}")))
}

fn reject_embedded_credentials(parsed: &ConnectionString) -> Result<(), RuntimeError> {
    if parsed
        .credential
        .as_ref()
        .and_then(|credential| credential.username.as_ref())
        .is_some()
    {
        return Err(config_error(
            "MongoDB connection URIs must not embed credentials; use service_account_name and service_account_password secret-backed fields.".to_string(),
        ));
    }
    Ok(())
}

fn is_mongo_connection_uri(host: &str) -> bool {
    host.starts_with("mongodb://") || host.starts_with("mongodb+srv://")
}

pub fn mongo_tls_options(security: &ConnectionSecurity) -> Tls {
    if !security.requires_tls() {
        return Tls::Disabled;
    }

    let tls_options = if security.allows_unvalidated_certificate() {
        TlsOptions::builder()
            .allow_invalid_certificates(true)
            .build()
    } else {
        TlsOptions::default()
    };

    Tls::Enabled(tls_options)
}

fn parse_port(port: &str) -> Result<u16, RuntimeError> {
    port.parse::<u16>()
        .map_err(|e| config_error(format!("invalid MongoDB port '{}': {}", port, e)))
}

fn config_error(message: String) -> RuntimeError {
    RuntimeError::Config(ConfigError::Load(message))
}

#[cfg(test)]
mod tests {
    use appfw_runtime::connection_security::{
        ConnectionSecurity, Provider, SecurityProfile, TlsMode,
    };
    use mongodb::options::{ServerAddress, Tls};

    use super::*;

    fn security(tls_mode: TlsMode) -> ConnectionSecurity {
        ConnectionSecurity {
            provider: Provider::MongoDb,
            security_profile: SecurityProfile::LocalDev,
            tls_mode,
        }
    }

    #[test]
    fn builds_mongo_client_options_from_provider_config() {
        let config = MongoConnectionConfig::new(
            "mongo.local",
            "27017",
            Some("user".to_string()),
            Some("secret".to_string()),
        );

        let options = futures::executor::block_on(mongo_client_options(
            &config,
            &security(TlsMode::Disabled),
        ))
        .expect("client options");

        assert_eq!(
            options.hosts,
            vec![ServerAddress::Tcp {
                host: "mongo.local".to_string(),
                port: Some(27017),
            }]
        );
        assert_eq!(options.max_pool_size, Some(DEFAULT_MAX_POOL_SIZE));
        assert_eq!(options.min_pool_size, Some(DEFAULT_MIN_POOL_SIZE));
        assert_eq!(options.app_name.as_deref(), Some(DEFAULT_APP_NAME));
        assert!(matches!(options.tls, Some(Tls::Disabled)));
    }

    #[test]
    fn maps_tls_security_to_mongo_tls_options() {
        assert!(matches!(
            mongo_tls_options(&security(TlsMode::Disabled)),
            Tls::Disabled
        ));
        assert!(matches!(
            mongo_tls_options(&security(TlsMode::Require)),
            Tls::Enabled(_)
        ));
    }

    #[test]
    fn rejects_invalid_mongo_ports() {
        let config = MongoConnectionConfig::new("mongo.local", "not-a-port", None, None);
        let err = futures::executor::block_on(mongo_client_options(
            &config,
            &security(TlsMode::Disabled),
        ))
        .expect_err("invalid port");

        assert!(
            matches!(err, RuntimeError::Config(ConfigError::Load(message)) if message.contains("invalid MongoDB port"))
        );
    }

    #[test]
    fn summarizes_seed_list_connection_uri() {
        let summary = mongo_connection_summary(
            "mongodb://mongo-a.example.net:27017,mongo-b.example.net:27018/admin?replicaSet=rs0",
        )
        .expect("connection summary");

        assert_eq!(summary.mode, MongoConnectionMode::ConnectionString);
        assert_eq!(
            summary.hosts,
            vec![
                "mongo-a.example.net:27017".to_string(),
                "mongo-b.example.net:27018".to_string(),
            ]
        );
        assert_eq!(summary.replica_set.as_deref(), Some("rs0"));
    }

    #[test]
    fn summarizes_srv_connection_uri_without_dns_lookup() {
        let summary =
            mongo_connection_summary("mongodb+srv://cluster.example.net/app?retryWrites=true")
                .expect("connection summary");

        assert_eq!(summary.mode, MongoConnectionMode::Srv);
        assert_eq!(summary.hosts, vec!["cluster.example.net".to_string()]);
    }

    #[test]
    fn rejects_credentials_embedded_in_connection_uri() {
        let err = mongo_connection_summary("mongodb://user:secret@mongo.example.net/app")
            .expect_err("embedded credentials should be rejected");

        assert!(
            matches!(err, RuntimeError::Config(ConfigError::Load(message)) if message.contains("must not embed credentials"))
        );
    }

    #[test]
    fn builds_options_from_seed_list_uri_and_secret_backed_credentials() {
        let config = MongoConnectionConfig::new(
            "mongodb://mongo-a.example.net:27017,mongo-b.example.net:27018/admin?replicaSet=rs0",
            "27017",
            Some("service-user".to_string()),
            Some("secret".to_string()),
        );

        let options =
            futures::executor::block_on(mongo_client_options(&config, &security(TlsMode::Require)))
                .expect("client options");

        assert_eq!(options.hosts.len(), 2);
        assert_eq!(options.repl_set_name.as_deref(), Some("rs0"));
        assert_eq!(
            options
                .credential
                .as_ref()
                .and_then(|credential| credential.username.as_deref()),
            Some("service-user")
        );
        assert!(matches!(options.tls, Some(Tls::Enabled(_))));
    }
}
