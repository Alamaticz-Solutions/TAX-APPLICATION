use std::{error::Error, fmt, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    PostgreSql,
    MsSqlServer,
    FabricSqlAnalytics,
    MongoDb,
    Snowflake,
    Neo4j,
}

impl FromStr for Provider {
    type Err = SecurityPolicyError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "PostgreSQL" | "postgresql" | "postgres" => Ok(Self::PostgreSql),
            "MsSqlServer" | "mssql" | "sqlserver" => Ok(Self::MsSqlServer),
            "FabricSqlAnalytics" | "fabric_sql_analytics" | "fabric-sql-analytics" | "fabric" => {
                Ok(Self::FabricSqlAnalytics)
            }
            "MongoDB" | "mongo" | "mongodb" => Ok(Self::MongoDb),
            "Snowflake" | "snowflake" => Ok(Self::Snowflake),
            "Neo4j" | "neo4j" => Ok(Self::Neo4j),
            _ => Err(SecurityPolicyError::UnknownProvider {
                provider: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityProfile {
    LocalDev,
    Managed,
}

impl FromStr for SecurityProfile {
    type Err = SecurityPolicyError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "local_dev" => Ok(Self::LocalDev),
            "managed" => Ok(Self::Managed),
            _ => Err(SecurityPolicyError::InvalidSecurityProfile {
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsMode {
    Disabled,
    Prefer,
    Require,
    VerifyCa,
    VerifyFull,
}

impl FromStr for TlsMode {
    type Err = SecurityPolicyError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "disabled" => Ok(Self::Disabled),
            "prefer" => Ok(Self::Prefer),
            "require" => Ok(Self::Require),
            "verify_ca" => Ok(Self::VerifyCa),
            "verify_full" => Ok(Self::VerifyFull),
            _ => Err(SecurityPolicyError::InvalidTlsMode {
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionSecurity {
    pub provider: Provider,
    pub security_profile: SecurityProfile,
    pub tls_mode: TlsMode,
}

impl ConnectionSecurity {
    pub fn allows_plaintext(&self) -> bool {
        self.security_profile == SecurityProfile::LocalDev && self.tls_mode == TlsMode::Disabled
    }

    pub fn allows_unvalidated_certificate(&self) -> bool {
        self.security_profile == SecurityProfile::LocalDev
            && matches!(self.tls_mode, TlsMode::Disabled | TlsMode::Prefer)
    }

    pub fn requires_tls(&self) -> bool {
        !matches!(self.tls_mode, TlsMode::Disabled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityPolicyError {
    UnknownProvider {
        provider: String,
    },
    InvalidSecurityProfile {
        value: String,
    },
    InvalidTlsMode {
        value: String,
    },
    LocalDevProfileOutsideLocalEnv {
        env_name: String,
    },
    PlaintextOutsideLocalDev {
        env_name: String,
        security_profile: String,
    },
    OpportunisticTlsOutsideLocalDev {
        env_name: String,
        security_profile: String,
    },
    ManagedRequiresTls {
        env_name: String,
    },
    SnowflakeHttpOutsideLocalDev {
        env_name: String,
        host: String,
    },
}

impl fmt::Display for SecurityPolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecurityPolicyError::UnknownProvider { provider } => {
                write!(f, "unknown data source provider `{provider}`")
            }
            SecurityPolicyError::InvalidSecurityProfile { value } => {
                write!(f, "invalid security_profile `{value}`")
            }
            SecurityPolicyError::InvalidTlsMode { value } => write!(f, "invalid tls_mode `{value}`"),
            SecurityPolicyError::LocalDevProfileOutsideLocalEnv { env_name } => write!(
                f,
                "security_profile `local_dev` is only allowed for `local` or `compose` environments, not `{env_name}`"
            ),
            SecurityPolicyError::PlaintextOutsideLocalDev {
                env_name,
                security_profile,
            } => write!(
                f,
                "tls_mode `disabled` is only allowed with security_profile `local_dev` in local environments; got `{security_profile}` in `{env_name}`"
            ),
            SecurityPolicyError::OpportunisticTlsOutsideLocalDev {
                env_name,
                security_profile,
            } => write!(
                f,
                "tls_mode `prefer` is only allowed with security_profile `local_dev`; got `{security_profile}` in `{env_name}`"
            ),
            SecurityPolicyError::ManagedRequiresTls { env_name } => write!(
                f,
                "managed environment `{env_name}` must use tls_mode `require`, `verify_ca`, or `verify_full`"
            ),
            SecurityPolicyError::SnowflakeHttpOutsideLocalDev { env_name, host } => write!(
                f,
                "Snowflake HTTP endpoints are only allowed for local development; `{host}` is not allowed in `{env_name}`"
            ),
        }
    }
}

impl Error for SecurityPolicyError {}

pub fn validate(
    provider: Provider,
    env_name: &str,
    security_profile: &str,
    tls_mode: &str,
    host: &str,
) -> Result<ConnectionSecurity, SecurityPolicyError> {
    let security_profile = SecurityProfile::from_str(security_profile)?;
    let tls_mode = TlsMode::from_str(tls_mode)?;
    let is_local_env = matches!(env_name, "local" | "compose");

    if security_profile == SecurityProfile::LocalDev && !is_local_env {
        return Err(SecurityPolicyError::LocalDevProfileOutsideLocalEnv {
            env_name: env_name.to_string(),
        });
    }

    if security_profile == SecurityProfile::Managed
        && matches!(tls_mode, TlsMode::Disabled | TlsMode::Prefer)
    {
        return Err(SecurityPolicyError::ManagedRequiresTls {
            env_name: env_name.to_string(),
        });
    }

    if tls_mode == TlsMode::Disabled
        && !(security_profile == SecurityProfile::LocalDev && is_local_env)
    {
        return Err(SecurityPolicyError::PlaintextOutsideLocalDev {
            env_name: env_name.to_string(),
            security_profile: security_profile_name(security_profile).to_string(),
        });
    }

    if tls_mode == TlsMode::Prefer && security_profile != SecurityProfile::LocalDev {
        return Err(SecurityPolicyError::OpportunisticTlsOutsideLocalDev {
            env_name: env_name.to_string(),
            security_profile: security_profile_name(security_profile).to_string(),
        });
    }

    if provider == Provider::Snowflake
        && has_plain_http_scheme(host)
        && !(security_profile == SecurityProfile::LocalDev && is_local_env)
    {
        return Err(SecurityPolicyError::SnowflakeHttpOutsideLocalDev {
            env_name: env_name.to_string(),
            host: host.to_string(),
        });
    }

    Ok(ConnectionSecurity {
        provider,
        security_profile,
        tls_mode,
    })
}

fn has_plain_http_scheme(host: &str) -> bool {
    matches!(
        host.trim_start().get(..7),
        Some(prefix) if prefix.eq_ignore_ascii_case("http://")
    )
}

fn security_profile_name(profile: SecurityProfile) -> &'static str {
    match profile {
        SecurityProfile::LocalDev => "local_dev",
        SecurityProfile::Managed => "managed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_dev_allows_plaintext_only_for_local_environments() {
        let security = validate(
            Provider::PostgreSql,
            "local",
            "local_dev",
            "disabled",
            "localhost",
        )
        .unwrap();

        assert!(security.allows_plaintext());
        assert!(security.allows_unvalidated_certificate());
    }

    #[test]
    fn neo4j_provider_alias_is_supported() {
        assert_eq!(Provider::from_str("Neo4j").unwrap(), Provider::Neo4j);
        let security = validate(Provider::Neo4j, "compose", "local_dev", "disabled", "neo4j")
            .expect("local Neo4j compose security");
        assert!(security.allows_plaintext());
    }

    #[test]
    fn local_dev_is_rejected_for_non_local_environment() {
        assert!(matches!(
            validate(
                Provider::PostgreSql,
                "prod",
                "local_dev",
                "disabled",
                "db.example.com"
            ),
            Err(SecurityPolicyError::LocalDevProfileOutsideLocalEnv { .. })
        ));
    }

    #[test]
    fn managed_environment_requires_tls() {
        assert!(matches!(
            validate(
                Provider::MongoDb,
                "prod",
                "managed",
                "disabled",
                "mongo.example.com"
            ),
            Err(SecurityPolicyError::ManagedRequiresTls { .. })
        ));
    }

    #[test]
    fn snowflake_http_is_rejected_outside_local_dev() {
        assert!(matches!(
            validate(
                Provider::Snowflake,
                "prod",
                "managed",
                "verify_full",
                "http://snowflake.example.com"
            ),
            Err(SecurityPolicyError::SnowflakeHttpOutsideLocalDev { .. })
        ));
    }

    #[test]
    fn snowflake_http_rejection_is_case_insensitive() {
        assert!(matches!(
            validate(
                Provider::Snowflake,
                "prod",
                "managed",
                "verify_full",
                "  HTTP://snowflake.example.com"
            ),
            Err(SecurityPolicyError::SnowflakeHttpOutsideLocalDev { .. })
        ));
    }
}
