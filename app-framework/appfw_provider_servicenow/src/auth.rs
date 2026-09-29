use crate::ServiceNowProviderError;

pub const SERVICENOW_BASE_URL_ENV: &str = "SERVICENOW_BASE_URL";
pub const SERVICENOW_AUTH_MODE_ENV: &str = "SERVICENOW_AUTH_MODE";
pub const SERVICENOW_CLIENT_ID_ENV: &str = "SERVICENOW_CLIENT_ID";
pub const SERVICENOW_CLIENT_SECRET_ENV: &str = "SERVICENOW_CLIENT_SECRET";
pub const SERVICENOW_USERNAME_ENV: &str = "SERVICENOW_USERNAME";
pub const SERVICENOW_PASSWORD_ENV: &str = "SERVICENOW_PASSWORD";
pub const SERVICENOW_INTEGRATION_PRINCIPAL_ENV: &str = "SERVICENOW_INTEGRATION_PRINCIPAL";
pub const SERVICENOW_DOMAIN_SCOPE_ENV: &str = "SERVICENOW_DOMAIN_SCOPE";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceNowAuthFlow {
    OAuthClientCredentials,
    BasicUserPassword,
    DelegatedOAuthLater,
}

impl ServiceNowAuthFlow {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OAuthClientCredentials => "oauth_client_credentials",
            Self::BasicUserPassword => "basic_user_password",
            Self::DelegatedOAuthLater => "delegated_oauth_later",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServiceNowAuthEnvVar {
    pub name: &'static str,
    pub required_for: Option<ServiceNowAuthFlow>,
    pub secret: bool,
    pub description: &'static str,
}

impl ServiceNowAuthEnvVar {
    pub fn is_required_for(self, flow: ServiceNowAuthFlow) -> bool {
        self.required_for.is_none() || self.required_for == Some(flow)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServiceNowAuthConfigMetadata {
    pub preferred_flow: ServiceNowAuthFlow,
    pub supported_flows: &'static [ServiceNowAuthFlow],
    pub env_vars: &'static [ServiceNowAuthEnvVar],
}

impl ServiceNowAuthConfigMetadata {
    pub fn tenant_instance_auth() -> Self {
        Self {
            preferred_flow: ServiceNowAuthFlow::OAuthClientCredentials,
            supported_flows: &SERVICENOW_AUTH_FLOWS,
            env_vars: &SERVICENOW_AUTH_ENV_VARS,
        }
    }

    pub fn env_vars_for_flow(
        self,
        flow: ServiceNowAuthFlow,
    ) -> impl Iterator<Item = ServiceNowAuthEnvVar> {
        self.env_vars
            .iter()
            .copied()
            .filter(move |env_var| env_var.is_required_for(flow))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceNowTenantBinding {
    pub base_url: String,
    integration_principal: String,
    domain_scope: Option<String>,
}

impl ServiceNowTenantBinding {
    pub fn new(
        base_url: impl Into<String>,
        integration_principal: impl Into<String>,
    ) -> Result<Self, ServiceNowProviderError> {
        Self::with_domain_scope(base_url, integration_principal, None::<String>)
    }

    pub fn with_domain_scope(
        base_url: impl Into<String>,
        integration_principal: impl Into<String>,
        domain_scope: Option<impl Into<String>>,
    ) -> Result<Self, ServiceNowProviderError> {
        let base_url = base_url.into();
        let integration_principal = integration_principal.into();
        let domain_scope = domain_scope.map(Into::into);

        validate_https_origin(SERVICENOW_BASE_URL_ENV, &base_url)?;
        validate_single_line_non_empty(
            SERVICENOW_INTEGRATION_PRINCIPAL_ENV,
            &integration_principal,
            "must identify the server-bound ServiceNow integration principal",
        )?;
        if let Some(domain_scope) = &domain_scope {
            validate_single_line_non_empty(
                SERVICENOW_DOMAIN_SCOPE_ENV,
                domain_scope,
                "must identify a server-bound ServiceNow domain separation scope",
            )?;
        }

        Ok(Self {
            base_url,
            integration_principal,
            domain_scope,
        })
    }

    pub fn host(&self) -> &str {
        https_origin_host(&self.base_url).expect("validated ServiceNow origin")
    }

    pub fn integration_principal(&self) -> &str {
        &self.integration_principal
    }

    pub fn domain_scope(&self) -> Option<&str> {
        self.domain_scope.as_deref()
    }
}

pub const SERVICENOW_AUTH_FLOWS: [ServiceNowAuthFlow; 3] = [
    ServiceNowAuthFlow::OAuthClientCredentials,
    ServiceNowAuthFlow::BasicUserPassword,
    ServiceNowAuthFlow::DelegatedOAuthLater,
];

pub const SERVICENOW_AUTH_ENV_VARS: [ServiceNowAuthEnvVar; 8] = [
    ServiceNowAuthEnvVar {
        name: SERVICENOW_BASE_URL_ENV,
        required_for: None,
        secret: false,
        description: "Tenant-specific ServiceNow HTTPS origin.",
    },
    ServiceNowAuthEnvVar {
        name: SERVICENOW_AUTH_MODE_ENV,
        required_for: None,
        secret: false,
        description: "Tenant-confirmed auth mode; live auth remains certification-gated.",
    },
    ServiceNowAuthEnvVar {
        name: SERVICENOW_CLIENT_ID_ENV,
        required_for: Some(ServiceNowAuthFlow::OAuthClientCredentials),
        secret: false,
        description: "Tenant-approved OAuth client id, once authenticated evidence exists.",
    },
    ServiceNowAuthEnvVar {
        name: SERVICENOW_CLIENT_SECRET_ENV,
        required_for: Some(ServiceNowAuthFlow::OAuthClientCredentials),
        secret: true,
        description: "Tenant-approved OAuth client secret or secret reference.",
    },
    ServiceNowAuthEnvVar {
        name: SERVICENOW_USERNAME_ENV,
        required_for: Some(ServiceNowAuthFlow::BasicUserPassword),
        secret: false,
        description: "Temporary live-smoke integration username, if basic auth is approved.",
    },
    ServiceNowAuthEnvVar {
        name: SERVICENOW_PASSWORD_ENV,
        required_for: Some(ServiceNowAuthFlow::BasicUserPassword),
        secret: true,
        description: "Temporary live-smoke integration password or secret reference.",
    },
    ServiceNowAuthEnvVar {
        name: SERVICENOW_INTEGRATION_PRINCIPAL_ENV,
        required_for: None,
        secret: false,
        description: "Server-bound integration principal whose ACLs/roles are certified.",
    },
    ServiceNowAuthEnvVar {
        name: SERVICENOW_DOMAIN_SCOPE_ENV,
        required_for: None,
        secret: false,
        description: "Optional server-bound ServiceNow domain separation scope.",
    },
];

fn validate_https_origin(name: &'static str, value: &str) -> Result<(), ServiceNowProviderError> {
    if value.trim() != value
        || value.is_empty()
        || value
            .chars()
            .any(|character| character.is_ascii_control() || character.is_ascii_whitespace())
    {
        return Err(ServiceNowProviderError::InvalidParameter {
            name,
            reason: "must be a single-line HTTPS origin without whitespace",
        });
    }

    let host = https_origin_host(value).ok_or(ServiceNowProviderError::InvalidParameter {
        name,
        reason: "must be an HTTPS origin without path, query, or fragment",
    })?;

    if host.is_empty() || host.contains('@') || host.contains(':') || !host.contains('.') {
        return Err(ServiceNowProviderError::InvalidParameter {
            name,
            reason: "must include a DNS host without userinfo or port",
        });
    }

    Ok(())
}

fn validate_single_line_non_empty(
    name: &'static str,
    value: &str,
    reason: &'static str,
) -> Result<(), ServiceNowProviderError> {
    if value.trim() != value
        || value.is_empty()
        || value
            .chars()
            .any(|character| character.is_ascii_control() || matches!(character, '\n' | '\r'))
    {
        return Err(ServiceNowProviderError::InvalidParameter { name, reason });
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
    fn auth_metadata_prefers_oauth_client_credentials_and_marks_secrets() {
        let metadata = ServiceNowAuthConfigMetadata::tenant_instance_auth();
        let env_vars = metadata
            .env_vars_for_flow(ServiceNowAuthFlow::OAuthClientCredentials)
            .collect::<Vec<_>>();

        assert_eq!(
            metadata.preferred_flow,
            ServiceNowAuthFlow::OAuthClientCredentials
        );
        assert!(env_vars
            .iter()
            .any(|env_var| env_var.name == SERVICENOW_CLIENT_SECRET_ENV && env_var.secret));
        assert!(env_vars
            .iter()
            .any(|env_var| env_var.name == SERVICENOW_INTEGRATION_PRINCIPAL_ENV));
    }

    #[test]
    fn tenant_binding_accepts_https_origin_principal_and_domain_scope() {
        let binding = ServiceNowTenantBinding::with_domain_scope(
            "https://example.service-now.com",
            "appfw-integration-user",
            Some("global"),
        )
        .expect("tenant binding");

        assert_eq!(binding.host(), "example.service-now.com");
        assert_eq!(binding.integration_principal(), "appfw-integration-user");
        assert_eq!(binding.domain_scope(), Some("global"));
    }

    #[test]
    fn tenant_binding_rejects_paths_queries_and_untrusted_instance_shapes() {
        assert!(ServiceNowTenantBinding::new(
            "https://example.service-now.com/api/now/table/incident",
            "principal"
        )
        .is_err());
        assert!(ServiceNowTenantBinding::new(
            "https://example.service-now.com?sysparm_query=active=true",
            "principal"
        )
        .is_err());
        assert!(
            ServiceNowTenantBinding::new("https://example.service-now.com:443", "principal")
                .is_err()
        );
        assert!(ServiceNowTenantBinding::new("https://example.service-now.com", "").is_err());
    }
}
