use crate::AnaplanProviderError;

pub const ANAPLAN_API_BASE_URL_ENV: &str = "ANAPLAN_API_BASE_URL";
pub const ANAPLAN_AUTH_BASE_URL_ENV: &str = "ANAPLAN_AUTH_BASE_URL";
pub const ANAPLAN_USERNAME_ENV: &str = "ANAPLAN_USERNAME";
pub const ANAPLAN_PASSWORD_OR_CERTIFICATE_ENV: &str = "ANAPLAN_PASSWORD_OR_CERTIFICATE";
pub const ANAPLAN_WORKSPACE_ID_ENV: &str = "ANAPLAN_WORKSPACE_ID";
pub const ANAPLAN_MODEL_ID_ENV: &str = "ANAPLAN_MODEL_ID";
pub const ANAPLAN_AUTHORIZATION_HEADER: &str = "Authorization";
pub const ANAPLAN_REDACTED_AUTH_TOKEN: &str = "AnaplanAuthToken <redacted>";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnaplanAuthFlow {
    Certificate,
    BasicUserPassword,
}

impl AnaplanAuthFlow {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Certificate => "certificate",
            Self::BasicUserPassword => "basic_user_password",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnaplanAuthEnvVar {
    pub name: &'static str,
    pub required_for: Option<AnaplanAuthFlow>,
    pub secret: bool,
    pub description: &'static str,
}

impl AnaplanAuthEnvVar {
    pub fn is_required_for(self, flow: AnaplanAuthFlow) -> bool {
        self.required_for.is_none() || self.required_for == Some(flow)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnaplanAuthConfigMetadata {
    pub integration_user: bool,
    pub preferred_flow: AnaplanAuthFlow,
    pub supported_flows: &'static [AnaplanAuthFlow],
    pub env_vars: &'static [AnaplanAuthEnvVar],
    pub authorization_header: &'static str,
    pub redacted_token_value: &'static str,
}

impl AnaplanAuthConfigMetadata {
    pub fn integration_user_auth() -> Self {
        Self {
            integration_user: true,
            preferred_flow: AnaplanAuthFlow::Certificate,
            supported_flows: &ANAPLAN_INTEGRATION_USER_AUTH_FLOWS,
            env_vars: &ANAPLAN_INTEGRATION_USER_AUTH_ENV_VARS,
            authorization_header: ANAPLAN_AUTHORIZATION_HEADER,
            redacted_token_value: ANAPLAN_REDACTED_AUTH_TOKEN,
        }
    }

    pub fn env_vars_for_flow(
        self,
        flow: AnaplanAuthFlow,
    ) -> impl Iterator<Item = AnaplanAuthEnvVar> {
        self.env_vars
            .iter()
            .copied()
            .filter(move |env_var| env_var.is_required_for(flow))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnaplanTenantBinding {
    pub api_base_url: String,
    pub auth_base_url: String,
    workspace_id: String,
    model_id: String,
}

impl AnaplanTenantBinding {
    pub fn new(
        api_base_url: impl Into<String>,
        auth_base_url: impl Into<String>,
        workspace_id: impl Into<String>,
        model_id: impl Into<String>,
    ) -> Result<Self, AnaplanProviderError> {
        let api_base_url = api_base_url.into();
        let auth_base_url = auth_base_url.into();
        let workspace_id = workspace_id.into();
        let model_id = model_id.into();

        validate_anaplan_https_origin(ANAPLAN_API_BASE_URL_ENV, &api_base_url)?;
        validate_anaplan_https_origin(ANAPLAN_AUTH_BASE_URL_ENV, &auth_base_url)?;
        validate_anaplan_identifier(ANAPLAN_WORKSPACE_ID_ENV, &workspace_id)?;
        validate_anaplan_identifier(ANAPLAN_MODEL_ID_ENV, &model_id)?;

        Ok(Self {
            api_base_url,
            auth_base_url,
            workspace_id,
            model_id,
        })
    }

    pub fn api_host(&self) -> &str {
        https_origin_host(&self.api_base_url).expect("validated Anaplan API origin")
    }

    pub fn auth_host(&self) -> &str {
        https_origin_host(&self.auth_base_url).expect("validated Anaplan auth origin")
    }

    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    pub fn workspace_path_prefix(&self) -> String {
        format!("/2/0/workspaces/{}", self.workspace_id)
    }

    pub fn model_path_prefix(&self) -> String {
        format!("{}/models/{}", self.workspace_path_prefix(), self.model_id)
    }
}

pub const ANAPLAN_INTEGRATION_USER_AUTH_FLOWS: [AnaplanAuthFlow; 2] = [
    AnaplanAuthFlow::Certificate,
    AnaplanAuthFlow::BasicUserPassword,
];

pub const ANAPLAN_INTEGRATION_USER_AUTH_ENV_VARS: [AnaplanAuthEnvVar; 6] = [
    AnaplanAuthEnvVar {
        name: ANAPLAN_API_BASE_URL_ENV,
        required_for: None,
        secret: false,
        description: "Bound Anaplan Integration API origin used for metadata and export requests.",
    },
    AnaplanAuthEnvVar {
        name: ANAPLAN_AUTH_BASE_URL_ENV,
        required_for: None,
        secret: false,
        description: "Bound Anaplan Authentication API origin used for token lifecycle requests.",
    },
    AnaplanAuthEnvVar {
        name: ANAPLAN_USERNAME_ENV,
        required_for: Some(AnaplanAuthFlow::BasicUserPassword),
        secret: false,
        description: "Tenant-approved Anaplan integration user name for exception flows.",
    },
    AnaplanAuthEnvVar {
        name: ANAPLAN_PASSWORD_OR_CERTIFICATE_ENV,
        required_for: None,
        secret: true,
        description: "Certificate material/reference for preferred auth or password for approved exception flows.",
    },
    AnaplanAuthEnvVar {
        name: ANAPLAN_WORKSPACE_ID_ENV,
        required_for: None,
        secret: false,
        description: "Server-bound Anaplan workspace id; callers cannot override this value.",
    },
    AnaplanAuthEnvVar {
        name: ANAPLAN_MODEL_ID_ENV,
        required_for: None,
        secret: false,
        description: "Server-bound Anaplan model id; callers cannot override this value.",
    },
];

fn validate_anaplan_https_origin(
    name: &'static str,
    value: &str,
) -> Result<(), AnaplanProviderError> {
    if value.trim() != value
        || value.is_empty()
        || value
            .chars()
            .any(|character| character.is_ascii_control() || character.is_ascii_whitespace())
    {
        return Err(AnaplanProviderError::InvalidParameter {
            name,
            reason: "must be a single-line HTTPS origin without whitespace",
        });
    }

    let host = https_origin_host(value).ok_or(AnaplanProviderError::InvalidParameter {
        name,
        reason: "must be an HTTPS origin without path, query, or fragment",
    })?;

    if host.is_empty() || host.contains('@') || host.contains(':') || !host.contains('.') {
        return Err(AnaplanProviderError::InvalidParameter {
            name,
            reason: "must include a DNS host without userinfo or port",
        });
    }

    Ok(())
}

fn validate_anaplan_identifier(
    name: &'static str,
    value: &str,
) -> Result<(), AnaplanProviderError> {
    if value.trim() != value || value.is_empty() {
        return Err(AnaplanProviderError::InvalidParameter {
            name,
            reason: "must be a non-empty server-bound identifier",
        });
    }

    if value
        .chars()
        .any(|character| !(character.is_ascii_alphanumeric() || matches!(character, '-' | '_')))
    {
        return Err(AnaplanProviderError::InvalidParameter {
            name,
            reason: "must contain only ASCII letters, digits, hyphen, or underscore",
        });
    }

    Ok(())
}

fn https_origin_host(value: &str) -> Option<&str> {
    let host = value.strip_prefix("https://")?;
    if host.contains('/') || host.contains('?') || host.contains('#') {
        return None;
    }
    Some(host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integration_user_auth_metadata_prefers_certificate_auth() {
        let metadata = AnaplanAuthConfigMetadata::integration_user_auth();
        let names = metadata
            .env_vars
            .iter()
            .map(|env_var| env_var.name)
            .collect::<Vec<_>>();

        assert!(metadata.integration_user);
        assert_eq!(metadata.preferred_flow, AnaplanAuthFlow::Certificate);
        assert_eq!(
            metadata.supported_flows,
            [
                AnaplanAuthFlow::Certificate,
                AnaplanAuthFlow::BasicUserPassword
            ]
        );
        assert_eq!(
            names,
            vec![
                ANAPLAN_API_BASE_URL_ENV,
                ANAPLAN_AUTH_BASE_URL_ENV,
                ANAPLAN_USERNAME_ENV,
                ANAPLAN_PASSWORD_OR_CERTIFICATE_ENV,
                ANAPLAN_WORKSPACE_ID_ENV,
                ANAPLAN_MODEL_ID_ENV,
            ]
        );
        assert_eq!(metadata.authorization_header, "Authorization");
        assert_eq!(metadata.redacted_token_value, "AnaplanAuthToken <redacted>");
    }

    #[test]
    fn certificate_metadata_does_not_require_username() {
        let metadata = AnaplanAuthConfigMetadata::integration_user_auth();
        let names = metadata
            .env_vars_for_flow(AnaplanAuthFlow::Certificate)
            .map(|env_var| env_var.name)
            .collect::<Vec<_>>();

        assert_eq!(
            names,
            vec![
                ANAPLAN_API_BASE_URL_ENV,
                ANAPLAN_AUTH_BASE_URL_ENV,
                ANAPLAN_PASSWORD_OR_CERTIFICATE_ENV,
                ANAPLAN_WORKSPACE_ID_ENV,
                ANAPLAN_MODEL_ID_ENV,
            ]
        );
    }

    #[test]
    fn tenant_binding_accepts_server_side_origins_and_ids() {
        let binding = AnaplanTenantBinding::new(
            "https://api.anaplan.com",
            "https://auth.anaplan.com",
            "8a8196b15b7dbae6015b8694411d13fe",
            "75A40874E6B64FA3AE0743278996850F",
        )
        .expect("tenant binding");

        assert_eq!(binding.api_host(), "api.anaplan.com");
        assert_eq!(binding.auth_host(), "auth.anaplan.com");
        assert_eq!(
            binding.workspace_path_prefix(),
            "/2/0/workspaces/8a8196b15b7dbae6015b8694411d13fe"
        );
        assert_eq!(
            binding.model_path_prefix(),
            "/2/0/workspaces/8a8196b15b7dbae6015b8694411d13fe/models/75A40874E6B64FA3AE0743278996850F"
        );
    }

    #[test]
    fn tenant_binding_rejects_paths_queries_and_untrusted_ids() {
        assert_eq!(
            AnaplanTenantBinding::new(
                "https://api.anaplan.com/2/0",
                "https://auth.anaplan.com",
                "workspace",
                "model",
            )
            .unwrap_err(),
            AnaplanProviderError::InvalidParameter {
                name: ANAPLAN_API_BASE_URL_ENV,
                reason: "must be an HTTPS origin without path, query, or fragment",
            }
        );

        assert_eq!(
            AnaplanTenantBinding::new(
                "https://api.anaplan.com",
                "https://auth.anaplan.com?token=leak",
                "workspace",
                "model",
            )
            .unwrap_err(),
            AnaplanProviderError::InvalidParameter {
                name: ANAPLAN_AUTH_BASE_URL_ENV,
                reason: "must be an HTTPS origin without path, query, or fragment",
            }
        );

        assert_eq!(
            AnaplanTenantBinding::new(
                "https://api.anaplan.com",
                "https://auth.anaplan.com",
                "../workspace",
                "model",
            )
            .unwrap_err(),
            AnaplanProviderError::InvalidParameter {
                name: ANAPLAN_WORKSPACE_ID_ENV,
                reason: "must contain only ASCII letters, digits, hyphen, or underscore",
            }
        );
    }
}
