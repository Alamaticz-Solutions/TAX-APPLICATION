use std::{env, error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthConfigError {
    MissingEnvVar { name: String },
}

impl fmt::Display for AuthConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthConfigError::MissingEnvVar { name } => {
                write!(f, "required environment variable is missing: {name}")
            }
        }
    }
}

impl Error for AuthConfigError {}

impl From<AuthConfigError> for crate::ConfigError {
    fn from(error: AuthConfigError) -> Self {
        match error {
            AuthConfigError::MissingEnvVar { name } => crate::ConfigError::MissingEnvVar { name },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JwtAuthConfig {
    pub jwt_issuer: String,
    pub jwt_audience: String,
    pub okta_client_id: String,
}

impl JwtAuthConfig {
    pub fn from_env() -> Result<Self, AuthConfigError> {
        Ok(Self {
            jwt_issuer: require_env("OKTA_ISSUER")?,
            jwt_audience: env::var("OKTA_AUDIENCE").unwrap_or_else(|_| "api://default".to_string()),
            okta_client_id: require_env("OKTA_CLIENT_ID")?,
        })
    }
}

fn require_env(name: &str) -> Result<String, AuthConfigError> {
    env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AuthConfigError::MissingEnvVar {
            name: name.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use std::{env, sync::Mutex};

    use super::*;

    const AUTH_ENV: [&str; 3] = ["OKTA_ISSUER", "OKTA_AUDIENCE", "OKTA_CLIENT_ID"];
    static AUTH_ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvSnapshot {
        values: Vec<(&'static str, Option<String>)>,
    }

    impl EnvSnapshot {
        fn capture(keys: &[&'static str]) -> Self {
            let values = keys.iter().map(|key| (*key, env::var(key).ok())).collect();
            for key in keys {
                env::remove_var(key);
            }
            Self { values }
        }
    }

    impl Drop for EnvSnapshot {
        fn drop(&mut self) {
            for (key, value) in &self.values {
                match value {
                    Some(value) => env::set_var(key, value),
                    None => env::remove_var(key),
                }
            }
        }
    }

    #[test]
    fn loads_okta_values_from_environment() {
        let _env = AUTH_ENV_LOCK.lock().expect("auth env lock");
        let _snapshot = EnvSnapshot::capture(&AUTH_ENV);
        env::set_var("OKTA_ISSUER", "https://tenant.okta.com/oauth2/default");
        env::set_var("OKTA_AUDIENCE", "api://tenant");
        env::set_var("OKTA_CLIENT_ID", "client-id");

        let config = JwtAuthConfig::from_env().expect("auth config");

        assert_eq!(config.jwt_issuer, "https://tenant.okta.com/oauth2/default");
        assert_eq!(config.jwt_audience, "api://tenant");
        assert_eq!(config.okta_client_id, "client-id");
    }

    #[test]
    fn defaults_audience_when_not_configured() {
        let _env = AUTH_ENV_LOCK.lock().expect("auth env lock");
        let _snapshot = EnvSnapshot::capture(&AUTH_ENV);
        env::set_var("OKTA_ISSUER", "issuer");
        env::set_var("OKTA_CLIENT_ID", "client-id");

        let config = JwtAuthConfig::from_env().expect("auth config");

        assert_eq!(config.jwt_audience, "api://default");
    }

    #[test]
    fn missing_required_env_var_returns_stable_error() {
        let _env = AUTH_ENV_LOCK.lock().expect("auth env lock");
        let _snapshot = EnvSnapshot::capture(&AUTH_ENV);
        env::set_var("OKTA_CLIENT_ID", "client-id");

        let error = JwtAuthConfig::from_env().unwrap_err();

        assert_eq!(
            error,
            AuthConfigError::MissingEnvVar {
                name: "OKTA_ISSUER".to_string()
            }
        );
    }

    #[test]
    fn empty_required_env_var_returns_stable_error() {
        let _env = AUTH_ENV_LOCK.lock().expect("auth env lock");
        let _snapshot = EnvSnapshot::capture(&AUTH_ENV);
        env::set_var("OKTA_ISSUER", "   ");
        env::set_var("OKTA_CLIENT_ID", "client-id");

        let error = JwtAuthConfig::from_env().unwrap_err();

        assert_eq!(
            error,
            AuthConfigError::MissingEnvVar {
                name: "OKTA_ISSUER".to_string()
            }
        );
    }

    #[test]
    fn auth_config_error_maps_to_runtime_config_error() {
        let error = crate::ConfigError::from(AuthConfigError::MissingEnvVar {
            name: "OKTA_ISSUER".to_string(),
        });

        assert!(matches!(
            error,
            crate::ConfigError::MissingEnvVar { name } if name == "OKTA_ISSUER"
        ));
    }
}
