use crate::{auth_config::JwtAuthConfig, ConfigError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeAuthState {
    pub jwt_issuer: String,
    pub jwt_audience: String,
    pub okta_client_id: String,
}

impl RuntimeAuthState {
    pub fn from_env() -> Result<Self, ConfigError> {
        JwtAuthConfig::from_env()
            .map(Self::from)
            .map_err(ConfigError::from)
    }
}

impl From<JwtAuthConfig> for RuntimeAuthState {
    fn from(auth_config: JwtAuthConfig) -> Self {
        Self {
            jwt_issuer: auth_config.jwt_issuer,
            jwt_audience: auth_config.jwt_audience,
            okta_client_id: auth_config.okta_client_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_runtime_auth_state_from_config() {
        let state = RuntimeAuthState::from(JwtAuthConfig {
            jwt_issuer: "https://tenant.okta.com/oauth2/default".to_string(),
            jwt_audience: "api://tenant".to_string(),
            okta_client_id: "client-id".to_string(),
        });

        assert_eq!(state.jwt_issuer, "https://tenant.okta.com/oauth2/default");
        assert_eq!(state.jwt_audience, "api://tenant");
        assert_eq!(state.okta_client_id, "client-id");
    }
}
