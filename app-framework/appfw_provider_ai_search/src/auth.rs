use crate::AiSearchProviderError;

pub const AI_SEARCH_BASE_URL_ENV: &str = "AI_SEARCH_BASE_URL";
pub const AI_SEARCH_API_KEY_ENV: &str = "AI_SEARCH_API_KEY";
pub const AI_SEARCH_BEARER_TOKEN_ENV: &str = "AI_SEARCH_BEARER_TOKEN";
pub const AI_SEARCH_CLIENT_ID_ENV: &str = "AI_SEARCH_CLIENT_ID";
pub const AI_SEARCH_CLIENT_SECRET_ENV: &str = "AI_SEARCH_CLIENT_SECRET";
pub const AI_SEARCH_TOKEN_URL_ENV: &str = "AI_SEARCH_TOKEN_URL";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiSearchAuthFlow {
    ApiKey,
    BearerToken,
    ClientCredentials,
}

impl AiSearchAuthFlow {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ApiKey => "api_key",
            Self::BearerToken => "bearer_token",
            Self::ClientCredentials => "client_credentials",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AiSearchAuthEnvVar {
    pub name: &'static str,
    pub required_for: Option<AiSearchAuthFlow>,
    pub secret: bool,
    pub description: &'static str,
}

impl AiSearchAuthEnvVar {
    pub fn is_required_for(self, flow: AiSearchAuthFlow) -> bool {
        self.required_for.is_none() || self.required_for == Some(flow)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AiSearchAuthConfigMetadata {
    pub preferred_flow: AiSearchAuthFlow,
    pub supported_flows: &'static [AiSearchAuthFlow],
    pub env_vars: &'static [AiSearchAuthEnvVar],
}

impl AiSearchAuthConfigMetadata {
    pub fn search_service_auth() -> Self {
        Self {
            preferred_flow: AiSearchAuthFlow::ClientCredentials,
            supported_flows: &AI_SEARCH_AUTH_FLOWS,
            env_vars: &AI_SEARCH_AUTH_ENV_VARS,
        }
    }

    pub fn env_vars_for_flow(
        self,
        flow: AiSearchAuthFlow,
    ) -> impl Iterator<Item = AiSearchAuthEnvVar> {
        self.env_vars
            .iter()
            .copied()
            .filter(move |env_var| env_var.is_required_for(flow))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiSearchTenantBinding {
    pub base_url: String,
}

impl AiSearchTenantBinding {
    pub fn new(base_url: impl Into<String>) -> Result<Self, AiSearchProviderError> {
        let base_url = base_url.into();
        validate_https_origin_allowing_port(AI_SEARCH_BASE_URL_ENV, &base_url)?;

        Ok(Self { base_url })
    }

    pub fn host(&self) -> &str {
        https_origin_host(&self.base_url).expect("validated AI search origin")
    }
}

pub const AI_SEARCH_AUTH_FLOWS: [AiSearchAuthFlow; 3] = [
    AiSearchAuthFlow::ApiKey,
    AiSearchAuthFlow::BearerToken,
    AiSearchAuthFlow::ClientCredentials,
];

pub const AI_SEARCH_AUTH_ENV_VARS: [AiSearchAuthEnvVar; 6] = [
    AiSearchAuthEnvVar {
        name: AI_SEARCH_BASE_URL_ENV,
        required_for: None,
        secret: false,
        description: "Bound AI search/generation endpoint origin.",
    },
    AiSearchAuthEnvVar {
        name: AI_SEARCH_API_KEY_ENV,
        required_for: Some(AiSearchAuthFlow::ApiKey),
        secret: true,
        description: "Public-service API key, such as an Anthropic-style x-api-key.",
    },
    AiSearchAuthEnvVar {
        name: AI_SEARCH_BEARER_TOKEN_ENV,
        required_for: Some(AiSearchAuthFlow::BearerToken),
        secret: true,
        description: "Static bearer token for self-hosted serving stacks.",
    },
    AiSearchAuthEnvVar {
        name: AI_SEARCH_CLIENT_ID_ENV,
        required_for: Some(AiSearchAuthFlow::ClientCredentials),
        secret: false,
        description: "OAuth2 client-credentials client id.",
    },
    AiSearchAuthEnvVar {
        name: AI_SEARCH_CLIENT_SECRET_ENV,
        required_for: Some(AiSearchAuthFlow::ClientCredentials),
        secret: true,
        description: "OAuth2 client-credentials secret reference.",
    },
    AiSearchAuthEnvVar {
        name: AI_SEARCH_TOKEN_URL_ENV,
        required_for: Some(AiSearchAuthFlow::ClientCredentials),
        secret: false,
        description: "OAuth2 token endpoint.",
    },
];

fn validate_https_origin_allowing_port(
    name: &'static str,
    value: &str,
) -> Result<(), AiSearchProviderError> {
    if value.trim() != value
        || value.is_empty()
        || value
            .chars()
            .any(|character| character.is_ascii_control() || character.is_ascii_whitespace())
    {
        return Err(AiSearchProviderError::InvalidParameter {
            name,
            reason: "must be a single-line HTTPS origin without whitespace",
        });
    }

    let host = https_origin_host(value).ok_or(AiSearchProviderError::InvalidParameter {
        name,
        reason: "must be an HTTPS origin without path, query, or fragment",
    })?;
    validate_host_with_optional_port(name, host)
}

fn validate_host_with_optional_port(
    name: &'static str,
    host: &str,
) -> Result<(), AiSearchProviderError> {
    if host.is_empty() || host.contains('@') {
        return Err(AiSearchProviderError::InvalidParameter {
            name,
            reason: "must include a DNS host without userinfo",
        });
    }

    let dns_host = if let Some((dns_host, port)) = host.rsplit_once(':') {
        if port.is_empty() || !port.chars().all(|character| character.is_ascii_digit()) {
            return Err(AiSearchProviderError::InvalidParameter {
                name,
                reason: "must use a numeric port when a port is present",
            });
        }
        dns_host
    } else {
        host
    };

    if dns_host.is_empty() || !dns_host.contains('.') || dns_host.contains(':') {
        return Err(AiSearchProviderError::InvalidParameter {
            name,
            reason: "must include a DNS host; explicit numeric ports are allowed",
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
    fn auth_metadata_marks_secret_values_and_flow_requirements() {
        let metadata = AiSearchAuthConfigMetadata::search_service_auth();
        let client_credentials = metadata
            .env_vars_for_flow(AiSearchAuthFlow::ClientCredentials)
            .map(|env_var| (env_var.name, env_var.secret))
            .collect::<Vec<_>>();

        assert_eq!(
            client_credentials,
            vec![
                (AI_SEARCH_BASE_URL_ENV, false),
                (AI_SEARCH_CLIENT_ID_ENV, false),
                (AI_SEARCH_CLIENT_SECRET_ENV, true),
                (AI_SEARCH_TOKEN_URL_ENV, false),
            ]
        );
    }

    #[test]
    fn tenant_binding_accepts_https_origin_with_explicit_port() {
        let binding =
            AiSearchTenantBinding::new("https://search.internal.pds:8443").expect("valid binding");

        assert_eq!(binding.host(), "search.internal.pds:8443");
    }

    #[test]
    fn tenant_binding_rejects_paths_and_unsafe_hosts() {
        for value in [
            "http://search.internal.pds",
            "https://search.internal.pds/path",
            "https://user@search.internal.pds",
            "https://localhost",
            "https://search.internal.pds:notaport",
            "https://search.internal.pds\n",
        ] {
            assert!(AiSearchTenantBinding::new(value).is_err(), "{value}");
        }
    }

    #[test]
    fn search_auth_headers_are_redacted_by_shared_saas_core() {
        assert!(appfw_saas_core::is_sensitive_header_name("x-api-key"));
        assert!(appfw_saas_core::is_sensitive_header_name("api-key"));
        assert!(appfw_saas_core::is_sensitive_header_name("authorization"));
    }
}
