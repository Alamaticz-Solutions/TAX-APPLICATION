use std::sync::Arc;

use appfw_runtime::{
    connection_security::{self, ConnectionSecurity, Provider, TlsMode},
    ConfigError, RuntimeError,
};
use deadpool_postgres::{Config, PoolConfig, Runtime, Timeouts};
use rustls::{
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    client::WebPkiServerVerifier,
    crypto::CryptoProvider,
    ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme,
};
use rustls_pki_types::{CertificateDer, ServerName, UnixTime};
use tokio_postgres::NoTls;
use tokio_postgres_rustls::MakeRustlsConnect;

use crate::execution::PostgresExecutionClient;

/// Environment variable for supplying an additional PEM-encoded CA certificate
/// (or bundle) to trust on top of the platform's native root store.
const PG_TLS_CA_FILE_ENV: &str = "PG_TLS_CA_FILE";
/// Environment variable overriding the deadpool maximum pool size.
const PG_POOL_MAX_SIZE_ENV: &str = "PG_POOL_MAX_SIZE";
/// Environment variable overriding the deadpool wait timeout (seconds).
const PG_POOL_WAIT_TIMEOUT_ENV: &str = "PG_POOL_WAIT_TIMEOUT_SECS";
/// Environment variable overriding the deadpool create timeout (seconds).
const PG_POOL_CREATE_TIMEOUT_ENV: &str = "PG_POOL_CREATE_TIMEOUT_SECS";
/// Environment variable overriding the deadpool recycle timeout (seconds).
const PG_POOL_RECYCLE_TIMEOUT_ENV: &str = "PG_POOL_RECYCLE_TIMEOUT_SECS";

const DEFAULT_POOL_MAX_SIZE: usize = 16;
const DEFAULT_WAIT_TIMEOUT_SECS: u64 = 30;
const DEFAULT_CREATE_TIMEOUT_SECS: u64 = 30;
const DEFAULT_RECYCLE_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostgresConnectionConfig {
    pub host: String,
    pub port: String,
    pub database: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl PostgresConnectionConfig {
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
            username,
            password,
        }
    }
}

pub fn postgres_pool_config(connection: &PostgresConnectionConfig) -> Result<Config, RuntimeError> {
    let mut config = Config::new();
    config.host = Some(connection.host.clone());
    config.port = Some(parse_port("PostgreSQL", &connection.port)?);
    config.dbname = Some(connection.database.clone());
    config.user = connection.username.clone();
    config.password = connection.password.clone();
    config.pool = Some(build_pool_settings()?);
    Ok(config)
}

/// Build the deadpool sizing / timeout settings from environment overrides,
/// falling back to bounded defaults so connection acquisition can never wait
/// indefinitely (deadpool's own defaults are unbounded).
fn build_pool_settings() -> Result<PoolConfig, RuntimeError> {
    let max_size = parse_env_usize(PG_POOL_MAX_SIZE_ENV, DEFAULT_POOL_MAX_SIZE)?;
    if max_size == 0 {
        return Err(RuntimeError::Config(ConfigError::Load(format!(
            "{PG_POOL_MAX_SIZE_ENV} must be greater than zero"
        ))));
    }

    let mut pool = PoolConfig::new(max_size);
    pool.timeouts = Timeouts {
        wait: Some(parse_env_timeout(
            PG_POOL_WAIT_TIMEOUT_ENV,
            DEFAULT_WAIT_TIMEOUT_SECS,
        )?),
        create: Some(parse_env_timeout(
            PG_POOL_CREATE_TIMEOUT_ENV,
            DEFAULT_CREATE_TIMEOUT_SECS,
        )?),
        recycle: Some(parse_env_timeout(
            PG_POOL_RECYCLE_TIMEOUT_ENV,
            DEFAULT_RECYCLE_TIMEOUT_SECS,
        )?),
    };
    Ok(pool)
}

fn parse_env_usize(var: &str, default: usize) -> Result<usize, RuntimeError> {
    match std::env::var(var) {
        Ok(raw) => raw.trim().parse::<usize>().map_err(|e| {
            RuntimeError::Config(ConfigError::Load(format!(
                "invalid {var} '{}': {}",
                raw.trim(),
                e
            )))
        }),
        Err(std::env::VarError::NotPresent) => Ok(default),
        Err(std::env::VarError::NotUnicode(_)) => Err(RuntimeError::Config(ConfigError::Load(
            format!("{var} contains non-unicode data"),
        ))),
    }
}

fn parse_env_timeout(var: &str, default_secs: u64) -> Result<std::time::Duration, RuntimeError> {
    let secs = match std::env::var(var) {
        Ok(raw) => raw.trim().parse::<u64>().map_err(|e| {
            RuntimeError::Config(ConfigError::Load(format!(
                "invalid {var} '{}': {}",
                raw.trim(),
                e
            )))
        })?,
        Err(std::env::VarError::NotPresent) => default_secs,
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err(RuntimeError::Config(ConfigError::Load(format!(
                "{var} contains non-unicode data"
            ))))
        }
    };
    Ok(std::time::Duration::from_secs(secs))
}

pub fn validate_postgres_connection_security(
    env_name: &str,
    security_profile: &str,
    tls_mode: &str,
    host: &str,
) -> Result<ConnectionSecurity, RuntimeError> {
    connection_security::validate(
        Provider::PostgreSql,
        env_name,
        security_profile,
        tls_mode,
        host,
    )
    .map_err(|error| {
        RuntimeError::DataAccess(format!("invalid PostgreSQL connection security: {error}"))
    })
}

pub fn postgres_execution_client(
    connection: &PostgresConnectionConfig,
    security: &ConnectionSecurity,
) -> Result<PostgresExecutionClient, RuntimeError> {
    let config = postgres_pool_config(connection)?;

    let pool = if security.requires_tls() {
        let connector = build_rustls_connector(security)?;
        config
            .create_pool(Some(Runtime::Tokio1), connector)
            .map_err(|error| RuntimeError::DataAccess(error.to_string()))?
    } else {
        // tls_mode `disabled`: only reachable for local_dev in a local
        // environment (enforced by `connection_security::validate`). Plaintext
        // is intentional here and nowhere else.
        config
            .create_pool(Some(Runtime::Tokio1), NoTls)
            .map_err(|error| RuntimeError::DataAccess(error.to_string()))?
    };

    Ok(PostgresExecutionClient::new(pool))
}

/// Build a rustls-backed Postgres TLS connector that honors the validated
/// [`ConnectionSecurity`] policy. Fails closed: any misconfiguration (missing
/// roots, unreadable CA file, crypto provider failure) returns an error rather
/// than silently downgrading the connection.
fn build_rustls_connector(
    security: &ConnectionSecurity,
) -> Result<MakeRustlsConnect, RuntimeError> {
    let provider = ensure_crypto_provider();
    let client_config = build_client_config(security, provider)?;
    Ok(MakeRustlsConnect::new(client_config))
}

/// Install (idempotently) and return the process-wide ring crypto provider.
fn ensure_crypto_provider() -> Arc<CryptoProvider> {
    if let Some(provider) = CryptoProvider::get_default() {
        return provider.clone();
    }
    // Ignore the result: a concurrent caller may have installed first, in which
    // case `get_default()` below still returns a valid provider.
    let _ = rustls::crypto::ring::default_provider().install_default();
    CryptoProvider::get_default()
        .cloned()
        .unwrap_or_else(|| Arc::new(rustls::crypto::ring::default_provider()))
}

fn build_client_config(
    security: &ConnectionSecurity,
    provider: Arc<CryptoProvider>,
) -> Result<ClientConfig, RuntimeError> {
    let builder = ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .map_err(|error| {
            RuntimeError::DataAccess(format!("failed to configure PostgreSQL TLS: {error}"))
        })?;

    match security.tls_mode {
        TlsMode::Disabled => {
            // Unreachable: `requires_tls()` is false for `disabled`, so the
            // caller never asks for a TLS connector. Fail closed regardless.
            Err(RuntimeError::DataAccess(
                "PostgreSQL TLS connector requested for tls_mode `disabled`".to_string(),
            ))
        }
        TlsMode::Prefer => {
            // `prefer` is only permitted for local_dev (enforced by
            // `connection_security::validate`). Opportunistic TLS without
            // certificate validation is acceptable only for local development.
            if !security.allows_unvalidated_certificate() {
                return Err(RuntimeError::DataAccess(
                    "PostgreSQL tls_mode `prefer` is only permitted for local development"
                        .to_string(),
                ));
            }
            let verifier = Arc::new(NoVerificationVerifier {
                provider: provider.clone(),
            });
            Ok(builder
                .dangerous()
                .with_custom_certificate_verifier(verifier)
                .with_no_client_auth())
        }
        TlsMode::Require | TlsMode::VerifyCa => {
            // Encrypt and verify the certificate chain against trusted roots,
            // but do not enforce the server hostname. This matches libpq's
            // `require`/`verify-ca` semantics.
            let roots = load_root_store(security)?;
            let webpki =
                WebPkiServerVerifier::builder_with_provider(Arc::new(roots), provider.clone())
                    .build()
                    .map_err(|error| {
                        RuntimeError::DataAccess(format!(
                            "failed to build PostgreSQL TLS verifier: {error}"
                        ))
                    })?;
            let verifier = Arc::new(SkipHostnameVerifier { inner: webpki });
            Ok(builder
                .dangerous()
                .with_custom_certificate_verifier(verifier)
                .with_no_client_auth())
        }
        TlsMode::VerifyFull => {
            // Full validation: certificate chain against trusted roots AND the
            // server hostname (rustls' default `WebPkiServerVerifier`).
            let roots = load_root_store(security)?;
            Ok(builder.with_root_certificates(roots).with_no_client_auth())
        }
    }
}

/// Build the trusted root certificate store. Prefers the platform native trust
/// store, falls back to the bundled webpki roots, and additionally trusts a
/// caller-supplied CA via `PG_TLS_CA_FILE` when present. Fails closed if no
/// roots can be assembled.
fn load_root_store(_security: &ConnectionSecurity) -> Result<RootCertStore, RuntimeError> {
    let mut roots = RootCertStore::empty();

    let native = rustls_native_certs::load_native_certs();
    for cert in native.certs {
        // Ignore individual malformed certs in the native store; we validate
        // that at least one usable root exists below.
        let _ = roots.add(cert);
    }

    if roots.is_empty() {
        // Native store unavailable (errors recorded in `native.errors`): fall
        // back to the compiled-in Mozilla root set so prod TLS still works.
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    }

    if let Some(extra) = load_custom_ca()? {
        let mut added = 0usize;
        for cert in extra {
            roots.add(cert).map_err(|error| {
                RuntimeError::DataAccess(format!(
                    "failed to add CA from {PG_TLS_CA_FILE_ENV}: {error}"
                ))
            })?;
            added += 1;
        }
        if added == 0 {
            return Err(RuntimeError::DataAccess(format!(
                "{PG_TLS_CA_FILE_ENV} contained no usable certificates"
            )));
        }
    }

    if roots.is_empty() {
        return Err(RuntimeError::DataAccess(
            "no trusted root certificates available for PostgreSQL TLS".to_string(),
        ));
    }

    Ok(roots)
}

/// Read and parse the optional custom CA bundle referenced by `PG_TLS_CA_FILE`.
fn load_custom_ca() -> Result<Option<Vec<CertificateDer<'static>>>, RuntimeError> {
    let path = match std::env::var(PG_TLS_CA_FILE_ENV) {
        Ok(path) if !path.trim().is_empty() => path,
        Ok(_) => return Ok(None),
        Err(std::env::VarError::NotPresent) => return Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err(RuntimeError::DataAccess(format!(
                "{PG_TLS_CA_FILE_ENV} contains non-unicode data"
            )))
        }
    };

    let pem = std::fs::read(&path).map_err(|error| {
        RuntimeError::DataAccess(format!(
            "failed to read {PG_TLS_CA_FILE_ENV} '{path}': {error}"
        ))
    })?;

    let certs = rustls_pki_types::pem::PemObject::pem_slice_iter(&pem[..])
        .collect::<Result<Vec<CertificateDer<'static>>, _>>()
        .map_err(|error| {
            RuntimeError::DataAccess(format!(
                "failed to parse CA certificates from {PG_TLS_CA_FILE_ENV} '{path}': {error}"
            ))
        })?;

    Ok(Some(certs))
}

/// Verifier that performs full chain validation against trusted roots but does
/// not enforce the server hostname (`require` / `verify-ca` semantics).
#[derive(Debug)]
struct SkipHostnameVerifier {
    inner: Arc<WebPkiServerVerifier>,
}

impl ServerCertVerifier for SkipHostnameVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        // Delegate to webpki for full chain/expiry/signature validation. Only a
        // hostname mismatch (`NotValidForName`) is tolerated, matching libpq
        // `require`/`verify-ca` semantics (encrypt + trust the CA, but do not
        // assert the server identity). Every other failure propagates.
        match self.inner.verify_server_cert(
            end_entity,
            intermediates,
            server_name,
            ocsp_response,
            now,
        ) {
            Ok(verified) => Ok(verified),
            Err(rustls::Error::InvalidCertificate(rustls::CertificateError::NotValidForName)) => {
                Ok(ServerCertVerified::assertion())
            }
            Err(other) => Err(other),
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.inner.supported_verify_schemes()
    }
}

/// Verifier that accepts any certificate. Only ever instantiated for
/// `tls_mode = prefer` under the `local_dev` profile.
#[derive(Debug)]
struct NoVerificationVerifier {
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for NoVerificationVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
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
    use super::*;

    use std::sync::{Mutex, MutexGuard, OnceLock};

    /// Serializes every test that reads or writes process-global environment
    /// variables, since `std::env` is shared across the test harness threads.
    fn env_lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Clears all framework-controlled env vars so a test starts from defaults.
    fn clear_pg_env() {
        for var in [
            PG_TLS_CA_FILE_ENV,
            PG_POOL_MAX_SIZE_ENV,
            PG_POOL_WAIT_TIMEOUT_ENV,
            PG_POOL_CREATE_TIMEOUT_ENV,
            PG_POOL_RECYCLE_TIMEOUT_ENV,
        ] {
            std::env::remove_var(var);
        }
    }

    struct EnvGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }

    #[test]
    fn builds_postgres_pool_config() {
        let _lock = env_lock();
        clear_pg_env();

        let connection = PostgresConnectionConfig::new(
            "postgres.local",
            "5432",
            "crm",
            Some("user".to_string()),
            Some("secret".to_string()),
        );

        let config = postgres_pool_config(&connection).expect("pool config");

        assert_eq!(config.host.as_deref(), Some("postgres.local"));
        assert_eq!(config.port, Some(5432));
        assert_eq!(config.dbname.as_deref(), Some("crm"));
        assert_eq!(config.user.as_deref(), Some("user"));
        assert_eq!(config.password.as_deref(), Some("secret"));

        let pool = config.pool.expect("pool settings present");
        assert_eq!(pool.max_size, DEFAULT_POOL_MAX_SIZE);
        assert_eq!(
            pool.timeouts.wait,
            Some(std::time::Duration::from_secs(DEFAULT_WAIT_TIMEOUT_SECS))
        );
        assert_eq!(
            pool.timeouts.create,
            Some(std::time::Duration::from_secs(DEFAULT_CREATE_TIMEOUT_SECS))
        );
        assert_eq!(
            pool.timeouts.recycle,
            Some(std::time::Duration::from_secs(DEFAULT_RECYCLE_TIMEOUT_SECS))
        );
    }

    #[test]
    fn rejects_invalid_postgres_ports() {
        let _lock = env_lock();
        clear_pg_env();
        let connection = PostgresConnectionConfig::new("postgres.local", "bad", "crm", None, None);
        let err = postgres_pool_config(&connection).expect_err("invalid port");

        assert!(
            matches!(err, RuntimeError::Config(ConfigError::Load(message)) if message.contains("invalid PostgreSQL port"))
        );
    }

    #[test]
    fn validates_postgres_connection_security() {
        let security =
            validate_postgres_connection_security("local", "local_dev", "disabled", "localhost")
                .expect("security");

        assert!(!security.requires_tls());
    }

    #[test]
    fn pool_size_is_env_overridable() {
        let _lock = env_lock();
        clear_pg_env();
        let _guard = EnvGuard::set(PG_POOL_MAX_SIZE_ENV, "4");
        let connection = PostgresConnectionConfig::new("postgres.local", "5432", "crm", None, None);
        let config = postgres_pool_config(&connection).expect("pool config");
        assert_eq!(config.pool.expect("pool").max_size, 4);
    }

    #[test]
    fn pool_timeouts_are_env_overridable() {
        let _lock = env_lock();
        clear_pg_env();
        let _guard = EnvGuard::set(PG_POOL_WAIT_TIMEOUT_ENV, "5");
        let connection = PostgresConnectionConfig::new("postgres.local", "5432", "crm", None, None);
        let config = postgres_pool_config(&connection).expect("pool config");
        assert_eq!(
            config.pool.expect("pool").timeouts.wait,
            Some(std::time::Duration::from_secs(5))
        );
    }

    #[test]
    fn rejects_zero_pool_size() {
        let _lock = env_lock();
        clear_pg_env();
        let _guard = EnvGuard::set(PG_POOL_MAX_SIZE_ENV, "0");
        let connection = PostgresConnectionConfig::new("postgres.local", "5432", "crm", None, None);
        let err = postgres_pool_config(&connection).expect_err("zero pool size");
        assert!(
            matches!(err, RuntimeError::Config(ConfigError::Load(message)) if message.contains("greater than zero"))
        );
    }

    #[test]
    fn rejects_non_numeric_pool_size() {
        let _lock = env_lock();
        clear_pg_env();
        let _guard = EnvGuard::set(PG_POOL_MAX_SIZE_ENV, "lots");
        let connection = PostgresConnectionConfig::new("postgres.local", "5432", "crm", None, None);
        let err = postgres_pool_config(&connection).expect_err("bad pool size");
        assert!(
            matches!(err, RuntimeError::Config(ConfigError::Load(message)) if message.contains(PG_POOL_MAX_SIZE_ENV))
        );
    }

    #[test]
    fn disabled_tls_does_not_require_tls() {
        let security =
            validate_postgres_connection_security("local", "local_dev", "disabled", "localhost")
                .expect("security");
        assert_eq!(security.tls_mode, TlsMode::Disabled);
        assert!(!security.requires_tls());
    }

    #[test]
    fn require_tls_selects_tls_connector() {
        let _lock = env_lock();
        clear_pg_env();
        let security =
            validate_postgres_connection_security("prod", "managed", "require", "postgres.local")
                .expect("security");
        assert_eq!(security.tls_mode, TlsMode::Require);
        assert!(security.requires_tls());
        // Building the rustls connector must succeed (real root store, fail-closed).
        let connector = build_rustls_connector(&security);
        assert!(connector.is_ok(), "require should build a TLS connector");
    }

    #[test]
    fn verify_full_selects_tls_connector() {
        let _lock = env_lock();
        clear_pg_env();
        let security = validate_postgres_connection_security(
            "prod",
            "managed",
            "verify_full",
            "postgres.local",
        )
        .expect("security");
        assert_eq!(security.tls_mode, TlsMode::VerifyFull);
        assert!(security.requires_tls());
        let connector = build_rustls_connector(&security);
        assert!(
            connector.is_ok(),
            "verify_full should build a TLS connector"
        );
    }

    #[test]
    fn verify_ca_selects_tls_connector() {
        let _lock = env_lock();
        clear_pg_env();
        let security =
            validate_postgres_connection_security("prod", "managed", "verify_ca", "postgres.local")
                .expect("security");
        assert_eq!(security.tls_mode, TlsMode::VerifyCa);
        let connector = build_rustls_connector(&security);
        assert!(connector.is_ok(), "verify_ca should build a TLS connector");
    }

    #[test]
    fn prefer_tls_is_local_dev_only_and_builds_connector() {
        let _lock = env_lock();
        clear_pg_env();
        let security =
            validate_postgres_connection_security("local", "local_dev", "prefer", "localhost")
                .expect("security");
        assert_eq!(security.tls_mode, TlsMode::Prefer);
        assert!(security.allows_unvalidated_certificate());
        let connector = build_rustls_connector(&security);
        assert!(connector.is_ok(), "prefer should build a TLS connector");
    }

    #[test]
    fn missing_custom_ca_file_fails_closed() {
        let _lock = env_lock();
        clear_pg_env();
        let _guard = EnvGuard::set(PG_TLS_CA_FILE_ENV, "/nonexistent/path/to/ca.pem");
        let security =
            validate_postgres_connection_security("prod", "managed", "require", "postgres.local")
                .expect("security");
        match build_rustls_connector(&security) {
            Ok(_) => panic!("missing CA file must fail closed"),
            Err(RuntimeError::DataAccess(message)) => {
                assert!(message.contains(PG_TLS_CA_FILE_ENV));
            }
            Err(other) => panic!("unexpected error: {other:?}"),
        }
    }
}
