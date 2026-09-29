//! Shared MS SQL / Fabric SQL authentication mode metadata.
//!
//! This crate intentionally has **no** ODBC dependency so `app_gen`
//! validation and config-contract generation can share the same mode registry
//! as runtime loaders, doctor, and the MS SQL provider.

#![forbid(unsafe_code)]

/// Plain MsSqlServer SQL login / password.
pub const SQL_PASSWORD: &str = "sql_password";
/// Fabric (or Entra) static access-token auth.
pub const ENTRA_ACCESS_TOKEN: &str = "entra_access_token";
/// Fabric Entra service-principal client credentials.
pub const ENTRA_CLIENT_CREDENTIALS: &str = "entra_client_credentials";
/// On-prem AD NTLM (domain Windows login + password via FreeTDS ODBC).
pub const NTLM: &str = "ntlm";

/// Canonical ordered list of all registered auth mode ids (config contract / enum).
pub const ALL_MODE_IDS: &[&str] = &[
    SQL_PASSWORD,
    ENTRA_ACCESS_TOKEN,
    ENTRA_CLIENT_CREDENTIALS,
    NTLM,
];

pub const PROVIDER_MSSQL: &str = "MsSqlServer";
pub const PROVIDER_FABRIC: &str = "FabricSqlAnalytics";

pub const DEFAULT_MSSQL_AUTH_MODE: &str = SQL_PASSWORD;
pub const DEFAULT_FABRIC_AUTH_MODE: &str = ENTRA_CLIENT_CREDENTIALS;

pub const MSSQL_AUTH_MODE_ENV: &str = "MSSQL_AUTH_MODE";
pub const FABRIC_AUTH_MODE_ENV: &str = "FABRIC_SQL_AUTH_MODE";

pub const FABRIC_SQL_ANALYTICS_DEFAULT_TOKEN_SCOPE: &str = "https://database.windows.net/.default";

/// How `doctor` should preflight credentials for a mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoctorPreflight {
    /// Require SQL service-account env vars (password auth).
    SqlServiceAccount,
    /// Static Entra access token secret.
    EntraAccessToken,
    /// Entra client credentials (tenant / client / secret).
    EntraClientCredentials,
}

/// Host shape required for a mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostRequirement {
    Any,
}

/// Whether a secret field must be present when loading environment secrets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldPresence {
    Required,
    Optional,
}

/// Env-key + required/optional for a single secret slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecretField {
    pub env_key: &'static str,
    pub presence: FieldPresence,
}

/// Secret matrix for a `(provider, mode)` pair used by loaders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecretBindings {
    pub auth_mode_env_key: &'static str,
    pub default_mode: &'static str,
    pub service_account_name: SecretField,
    pub service_account_password: SecretField,
    pub entra_tenant_id: Option<SecretField>,
    /// Always optional when present; loaded with `get_secret`.
    pub entra_token_scope: Option<&'static str>,
}

/// Per-mode descriptor registered once for all consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthModeDescriptor {
    pub id: &'static str,
    /// Providers that may use this mode at runtime (loaders / clients).
    pub providers: &'static [&'static str],
    /// Providers that may declare this mode in model `_res.yaml` / config.
    /// Narrower than `providers` when a mode is env-override-only (e.g. Fabric
    /// `entra_access_token`).
    pub model_providers: &'static [&'static str],
    pub doctor: DoctorPreflight,
    pub host_requirement: HostRequirement,
    /// When true and `entra_token_scope` is set, scope must end with `.default`.
    pub requires_default_token_scope: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthValidationError {
    pub code: &'static str,
    pub message: String,
    pub expected: Option<&'static str>,
    pub remediation: Option<&'static str>,
}

impl AuthValidationError {
    pub fn new(
        code: &'static str,
        message: impl Into<String>,
        expected: Option<&'static str>,
        remediation: Option<&'static str>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            expected,
            remediation,
        }
    }
}

impl std::fmt::Display for AuthValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AuthValidationError {}

const SQL_PASSWORD_DESC: AuthModeDescriptor = AuthModeDescriptor {
    id: SQL_PASSWORD,
    providers: &[PROVIDER_MSSQL],
    model_providers: &[PROVIDER_MSSQL],
    doctor: DoctorPreflight::SqlServiceAccount,
    host_requirement: HostRequirement::Any,
    requires_default_token_scope: false,
};

const ENTRA_ACCESS_TOKEN_DESC: AuthModeDescriptor = AuthModeDescriptor {
    id: ENTRA_ACCESS_TOKEN,
    providers: &[PROVIDER_FABRIC],
    // Env-override / loader only — model validation still requires client-credentials.
    model_providers: &[],
    doctor: DoctorPreflight::EntraAccessToken,
    host_requirement: HostRequirement::Any,
    requires_default_token_scope: false,
};

const ENTRA_CLIENT_CREDENTIALS_DESC: AuthModeDescriptor = AuthModeDescriptor {
    id: ENTRA_CLIENT_CREDENTIALS,
    providers: &[PROVIDER_FABRIC],
    model_providers: &[PROVIDER_FABRIC],
    doctor: DoctorPreflight::EntraClientCredentials,
    host_requirement: HostRequirement::Any,
    requires_default_token_scope: true,
};

const NTLM_DESC: AuthModeDescriptor = AuthModeDescriptor {
    id: NTLM,
    providers: &[PROVIDER_MSSQL],
    model_providers: &[PROVIDER_MSSQL],
    doctor: DoctorPreflight::SqlServiceAccount,
    host_requirement: HostRequirement::Any,
    requires_default_token_scope: false,
};

const ALL_MODES: &[AuthModeDescriptor] = &[
    SQL_PASSWORD_DESC,
    ENTRA_ACCESS_TOKEN_DESC,
    ENTRA_CLIENT_CREDENTIALS_DESC,
    NTLM_DESC,
];

/// All registered auth mode descriptors.
pub fn all_modes() -> &'static [AuthModeDescriptor] {
    ALL_MODES
}

/// Look up a mode by id.
pub fn get(id: &str) -> Option<&'static AuthModeDescriptor> {
    ALL_MODES.iter().find(|mode| mode.id == id)
}

/// Modes allowed at runtime for a data_source_type string.
pub fn allowed_for(provider: &str) -> Vec<&'static AuthModeDescriptor> {
    ALL_MODES
        .iter()
        .filter(|mode| mode.providers.contains(&provider))
        .collect()
}

/// Modes that may appear in model / config_contract declarations for a provider.
pub fn model_allowed_for(provider: &str) -> Vec<&'static AuthModeDescriptor> {
    ALL_MODES
        .iter()
        .filter(|mode| mode.model_providers.contains(&provider))
        .collect()
}

/// Default auth mode id for a provider.
pub fn default_mode_for(provider: &str) -> Option<&'static str> {
    match provider {
        PROVIDER_MSSQL => Some(DEFAULT_MSSQL_AUTH_MODE),
        PROVIDER_FABRIC => Some(DEFAULT_FABRIC_AUTH_MODE),
        _ => None,
    }
}

/// Whether `mode` is registered and applicable to `provider`.
pub fn is_allowed(provider: &str, mode: &str) -> bool {
    get(mode)
        .map(|desc| desc.providers.contains(&provider))
        .unwrap_or(false)
}

/// Env key used to override auth_mode for a provider.
pub fn auth_mode_env_key(provider: &str) -> Option<&'static str> {
    match provider {
        PROVIDER_MSSQL => Some(MSSQL_AUTH_MODE_ENV),
        PROVIDER_FABRIC => Some(FABRIC_AUTH_MODE_ENV),
        _ => None,
    }
}

/// Resolve configured mode or provider default.
pub fn resolve_mode(
    provider: &str,
    configured: Option<&str>,
) -> Result<&'static str, AuthValidationError> {
    let mode = match configured.map(str::trim).filter(|s| !s.is_empty()) {
        Some(value) => value,
        None => {
            return default_mode_for(provider).ok_or_else(|| {
                AuthValidationError::new(
                    "unsupported_mssql_provider",
                    format!("unsupported MS SQL provider `{provider}` for auth_mode resolution"),
                    None,
                    None,
                )
            });
        }
    };
    validate_mode_for_provider(provider, mode)?;
    // Return the canonical static id from the registry.
    Ok(get(mode).expect("validated mode").id)
}

fn reject_provider_mode(provider: &str, mode: &str, allowed: &[&str]) -> AuthValidationError {
    let allowed_list = allowed.join(", ");
    if provider == PROVIDER_FABRIC {
        AuthValidationError::new(
            "invalid_fabric_sql_auth_mode",
            "FabricSqlAnalytics supports only service-principal client-credentials authentication.".to_string(),
            Some(ENTRA_CLIENT_CREDENTIALS),
            Some(
                "Use `entra_client_credentials` with FABRIC_TENANT_ID, FABRIC_CLIENT_ID, and FABRIC_CLIENT_SECRET.",
            ),
        )
    } else {
        AuthValidationError::new(
            "invalid_mssql_auth_mode",
            format!("`{mode}` is not supported for `{provider}`; allowed: {allowed_list}"),
            None,
            Some("Choose an auth_mode allowed for this data_source_type."),
        )
    }
}

/// Reject unknown or runtime-inapplicable modes (loaders / clients).
pub fn validate_mode_for_provider(provider: &str, mode: &str) -> Result<(), AuthValidationError> {
    match get(mode) {
        None => Err(AuthValidationError::new(
            "invalid_auth_mode",
            format!("unsupported auth_mode `{mode}`"),
            Some("registered MS SQL / Fabric auth mode"),
            Some("Use a mode from the auth mode registry (sql_password, ntlm, entra_*)."),
        )),
        Some(desc) if !desc.providers.contains(&provider) => {
            let allowed: Vec<&str> = allowed_for(provider).into_iter().map(|m| m.id).collect();
            Err(reject_provider_mode(provider, mode, &allowed))
        }
        Some(_) => Ok(()),
    }
}

/// Reject modes that must not appear in model `_res.yaml` for this provider.
pub fn validate_model_mode_for_provider(
    provider: &str,
    mode: &str,
) -> Result<(), AuthValidationError> {
    match get(mode) {
        None => Err(AuthValidationError::new(
            "invalid_auth_mode",
            format!("unsupported auth_mode `{mode}`"),
            Some("registered MS SQL / Fabric auth mode"),
            Some("Use a mode from the auth mode registry (sql_password, ntlm, entra_*)."),
        )),
        Some(desc) if !desc.model_providers.contains(&provider) => {
            let allowed: Vec<&str> = model_allowed_for(provider)
                .into_iter()
                .map(|m| m.id)
                .collect();
            Err(reject_provider_mode(provider, mode, &allowed))
        }
        Some(_) => Ok(()),
    }
}

/// Validate `db_host` against the mode's host requirement.
///
/// Current modes (`sql_password`, `ntlm`, Entra) accept any host/IP; this remains
/// a registry-aware hook so future modes can add SPN or hostname constraints.
pub fn validate_db_host(mode: &str, _host: &str) -> Result<(), AuthValidationError> {
    match get(mode) {
        None => Err(AuthValidationError::new(
            "invalid_auth_mode",
            format!("unsupported auth_mode `{mode}`"),
            Some("registered MS SQL / Fabric auth mode"),
            Some("Use a mode from the auth mode registry (sql_password, ntlm, entra_*)."),
        )),
        Some(_) => Ok(()),
    }
}

/// Validate optional Entra token scope for modes that require `.default`.
pub fn validate_token_scope(mode: &str, scope: Option<&str>) -> Result<(), AuthValidationError> {
    let Some(desc) = get(mode) else {
        return Ok(());
    };
    if !desc.requires_default_token_scope {
        return Ok(());
    }
    let Some(scope) = scope.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(());
    };
    if scope.ends_with(".default") {
        Ok(())
    } else {
        Err(AuthValidationError::new(
            "invalid_fabric_sql_token_scope",
            "FabricSqlAnalytics Entra client-credentials auth requires an OAuth v2 token scope ending with `.default`.",
            Some("OAuth v2 `.default` scope"),
            Some(
                "Set entra_token_scope to a v2 scope such as `https://database.windows.net/.default`.",
            ),
        ))
    }
}

/// Secret binding matrix for loaders (`database`, product templates).
pub fn secret_bindings(provider: &str, mode: &str) -> Result<SecretBindings, AuthValidationError> {
    validate_mode_for_provider(provider, mode)?;
    match (provider, mode) {
        (PROVIDER_MSSQL, NTLM) => Ok(SecretBindings {
            auth_mode_env_key: MSSQL_AUTH_MODE_ENV,
            default_mode: DEFAULT_MSSQL_AUTH_MODE,
            service_account_name: SecretField {
                env_key: "MSSQL_SERVICE_ACCOUNT_NAME",
                presence: FieldPresence::Required,
            },
            service_account_password: SecretField {
                env_key: "MSSQL_SERVICE_ACCOUNT_PASS",
                presence: FieldPresence::Required,
            },
            entra_tenant_id: None,
            entra_token_scope: None,
        }),
        (PROVIDER_MSSQL, SQL_PASSWORD) => Ok(SecretBindings {
            auth_mode_env_key: MSSQL_AUTH_MODE_ENV,
            default_mode: DEFAULT_MSSQL_AUTH_MODE,
            service_account_name: SecretField {
                env_key: "MSSQL_SERVICE_ACCOUNT_NAME",
                presence: FieldPresence::Required,
            },
            service_account_password: SecretField {
                env_key: "MSSQL_SERVICE_ACCOUNT_PASS",
                presence: FieldPresence::Required,
            },
            entra_tenant_id: None,
            entra_token_scope: None,
        }),
        (PROVIDER_FABRIC, ENTRA_ACCESS_TOKEN) => Ok(SecretBindings {
            auth_mode_env_key: FABRIC_AUTH_MODE_ENV,
            default_mode: DEFAULT_FABRIC_AUTH_MODE,
            service_account_name: SecretField {
                env_key: "FABRIC_CLIENT_ID",
                presence: FieldPresence::Optional,
            },
            service_account_password: SecretField {
                env_key: "FABRIC_ACCESS_TOKEN",
                presence: FieldPresence::Required,
            },
            entra_tenant_id: None,
            entra_token_scope: Some("FABRIC_TOKEN_SCOPE"),
        }),
        (PROVIDER_FABRIC, ENTRA_CLIENT_CREDENTIALS) => Ok(SecretBindings {
            auth_mode_env_key: FABRIC_AUTH_MODE_ENV,
            default_mode: DEFAULT_FABRIC_AUTH_MODE,
            service_account_name: SecretField {
                env_key: "FABRIC_CLIENT_ID",
                presence: FieldPresence::Required,
            },
            service_account_password: SecretField {
                env_key: "FABRIC_CLIENT_SECRET",
                presence: FieldPresence::Required,
            },
            entra_tenant_id: Some(SecretField {
                env_key: "FABRIC_TENANT_ID",
                presence: FieldPresence::Required,
            }),
            entra_token_scope: Some("FABRIC_TOKEN_SCOPE"),
        }),
        _ => Err(AuthValidationError::new(
            "missing_secret_bindings",
            format!("no secret bindings for `{provider}` / `{mode}`"),
            None,
            None,
        )),
    }
}

/// Doctor preflight kind for a resolved mode (falls back to SQL password for MsSql).
pub fn doctor_preflight(provider: &str, mode: Option<&str>) -> Option<DoctorPreflight> {
    let resolved = match mode {
        Some(m) if !m.trim().is_empty() => m,
        _ => default_mode_for(provider)?,
    };
    get(resolved).map(|d| d.doctor)
}

/// Human-readable auth_mode help for the config contract.
pub fn config_contract_auth_mode_docs() -> &'static str {
    "Authentication mode. FabricSqlAnalytics supports only `entra_client_credentials` (loaders may also accept `entra_access_token`). Plain MsSqlServer supports `sql_password` (default) and `ntlm` (on-prem AD domain login via FreeTDS ODBC; see ADR 0018)."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_covers_every_mode_id() {
        assert_eq!(ALL_MODE_IDS.len(), all_modes().len());
        for id in ALL_MODE_IDS {
            let desc = get(id).expect("mode registered");
            assert_eq!(desc.id, *id);
            assert!(!desc.providers.is_empty());
            let _ = desc.doctor;
        }
    }

    #[test]
    fn mssql_allows_password_and_ntlm_rejects_fabric_modes() {
        assert!(is_allowed(PROVIDER_MSSQL, SQL_PASSWORD));
        assert!(is_allowed(PROVIDER_MSSQL, NTLM));
        assert!(!is_allowed(PROVIDER_MSSQL, ENTRA_CLIENT_CREDENTIALS));
        assert!(!is_allowed(PROVIDER_FABRIC, NTLM));
        assert!(is_allowed(PROVIDER_FABRIC, ENTRA_CLIENT_CREDENTIALS));
    }

    #[test]
    fn fabric_rejects_ntlm_and_unknown() {
        let err =
            validate_mode_for_provider(PROVIDER_FABRIC, NTLM).expect_err("fabric rejects ntlm");
        assert_eq!(err.code, "invalid_fabric_sql_auth_mode");
        let err = validate_mode_for_provider(PROVIDER_MSSQL, "nope").expect_err("unknown");
        assert_eq!(err.code, "invalid_auth_mode");
    }

    #[test]
    fn secret_profiles_match_modes() {
        let ntlm = secret_bindings(PROVIDER_MSSQL, NTLM).expect("ntlm");
        assert_eq!(
            ntlm.service_account_password.presence,
            FieldPresence::Required
        );

        let sql = secret_bindings(PROVIDER_MSSQL, SQL_PASSWORD).expect("sql");
        assert_eq!(
            sql.service_account_password.presence,
            FieldPresence::Required
        );

        let entra = secret_bindings(PROVIDER_FABRIC, ENTRA_CLIENT_CREDENTIALS).expect("entra");
        assert!(entra.entra_tenant_id.is_some());
        assert_eq!(
            entra.entra_tenant_id.unwrap().presence,
            FieldPresence::Required
        );
    }

    #[test]
    fn token_scope_default_required_for_client_credentials() {
        validate_token_scope(
            ENTRA_CLIENT_CREDENTIALS,
            Some("https://database.windows.net/.default"),
        )
        .expect("ok");
        let err = validate_token_scope(
            ENTRA_CLIENT_CREDENTIALS,
            Some("https://database.windows.net/"),
        )
        .expect_err("resource");
        assert_eq!(err.code, "invalid_fabric_sql_token_scope");
    }
}
