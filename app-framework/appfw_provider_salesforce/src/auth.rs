use crate::SalesforceProviderError;

pub const SALESFORCE_INSTANCE_URL_ENV: &str = "SALESFORCE_INSTANCE_URL";
pub const SALESFORCE_LOGIN_BASE_URL_ENV: &str = "SALESFORCE_LOGIN_BASE_URL";
pub const SALESFORCE_CLIENT_ID_ENV: &str = "SALESFORCE_CLIENT_ID";
pub const SALESFORCE_CLIENT_SECRET_OR_PRIVATE_KEY_ENV: &str =
    "SALESFORCE_CLIENT_SECRET_OR_PRIVATE_KEY";
pub const SALESFORCE_USERNAME_OR_SUBJECT_ENV: &str = "SALESFORCE_USERNAME_OR_SUBJECT";
pub const SALESFORCE_AUTHORIZATION_HEADER: &str = "Authorization";
pub const SALESFORCE_REDACTED_BEARER_TOKEN: &str = "Bearer <redacted>";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SalesforceAuthFlow {
    JwtBearer,
    ClientCredentials,
}

impl SalesforceAuthFlow {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::JwtBearer => "jwt_bearer",
            Self::ClientCredentials => "client_credentials",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SalesforceAuthEnvVar {
    pub name: &'static str,
    pub required_for: Option<SalesforceAuthFlow>,
    pub secret: bool,
    pub description: &'static str,
}

impl SalesforceAuthEnvVar {
    pub fn is_required_for(self, flow: SalesforceAuthFlow) -> bool {
        self.required_for.is_none() || self.required_for == Some(flow)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SalesforceAuthConfigMetadata {
    pub integration_user: bool,
    pub supported_flows: &'static [SalesforceAuthFlow],
    pub env_vars: &'static [SalesforceAuthEnvVar],
    pub authorization_header: &'static str,
    pub redacted_bearer_value: &'static str,
}

impl SalesforceAuthConfigMetadata {
    pub fn integration_user_oauth() -> Self {
        Self {
            integration_user: true,
            supported_flows: &SALESFORCE_INTEGRATION_USER_AUTH_FLOWS,
            env_vars: &SALESFORCE_INTEGRATION_USER_AUTH_ENV_VARS,
            authorization_header: SALESFORCE_AUTHORIZATION_HEADER,
            redacted_bearer_value: SALESFORCE_REDACTED_BEARER_TOKEN,
        }
    }

    pub fn env_vars_for_flow(
        self,
        flow: SalesforceAuthFlow,
    ) -> impl Iterator<Item = SalesforceAuthEnvVar> {
        self.env_vars
            .iter()
            .copied()
            .filter(move |env_var| env_var.is_required_for(flow))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SalesforceTenantBinding {
    pub instance_url: String,
    pub login_base_url: String,
}

impl SalesforceTenantBinding {
    pub fn new(
        instance_url: impl Into<String>,
        login_base_url: impl Into<String>,
    ) -> Result<Self, SalesforceProviderError> {
        let instance_url = instance_url.into();
        let login_base_url = login_base_url.into();

        validate_salesforce_https_origin(SALESFORCE_INSTANCE_URL_ENV, &instance_url)?;
        validate_salesforce_https_origin(SALESFORCE_LOGIN_BASE_URL_ENV, &login_base_url)?;

        Ok(Self {
            instance_url,
            login_base_url,
        })
    }

    pub fn instance_host(&self) -> &str {
        https_origin_host(&self.instance_url).expect("validated tenant binding")
    }

    pub fn login_host(&self) -> &str {
        https_origin_host(&self.login_base_url).expect("validated tenant binding")
    }
}

pub const SALESFORCE_INTEGRATION_USER_AUTH_FLOWS: [SalesforceAuthFlow; 2] = [
    SalesforceAuthFlow::JwtBearer,
    SalesforceAuthFlow::ClientCredentials,
];

pub const SALESFORCE_INTEGRATION_USER_AUTH_ENV_VARS: [SalesforceAuthEnvVar; 5] = [
    SalesforceAuthEnvVar {
        name: SALESFORCE_INSTANCE_URL_ENV,
        required_for: None,
        secret: false,
        description: "Bound Salesforce org instance URL used for REST API requests.",
    },
    SalesforceAuthEnvVar {
        name: SALESFORCE_LOGIN_BASE_URL_ENV,
        required_for: None,
        secret: false,
        description: "Salesforce OAuth login base URL, such as login or test.",
    },
    SalesforceAuthEnvVar {
        name: SALESFORCE_CLIENT_ID_ENV,
        required_for: None,
        secret: false,
        description: "Connected app OAuth client id.",
    },
    SalesforceAuthEnvVar {
        name: SALESFORCE_CLIENT_SECRET_OR_PRIVATE_KEY_ENV,
        required_for: None,
        secret: true,
        description:
            "Client secret for client credentials or private key material/reference for JWT bearer.",
    },
    SalesforceAuthEnvVar {
        name: SALESFORCE_USERNAME_OR_SUBJECT_ENV,
        required_for: Some(SalesforceAuthFlow::JwtBearer),
        secret: false,
        description: "Integration user username or subject for JWT bearer assertions.",
    },
];

fn validate_salesforce_https_origin(
    name: &'static str,
    value: &str,
) -> Result<(), SalesforceProviderError> {
    if value.trim() != value
        || value.is_empty()
        || value
            .chars()
            .any(|character| character.is_ascii_control() || character.is_ascii_whitespace())
    {
        return Err(SalesforceProviderError::InvalidParameter {
            name,
            reason: "must be a single-line HTTPS origin without whitespace",
        });
    }

    let host = https_origin_host(value).ok_or(SalesforceProviderError::InvalidParameter {
        name,
        reason: "must be an HTTPS origin without path, query, or fragment",
    })?;

    if host.is_empty() || host.contains('@') || host.contains(':') || !host.contains('.') {
        return Err(SalesforceProviderError::InvalidParameter {
            name,
            reason: "must include a DNS host without userinfo or port",
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
    fn integration_user_auth_metadata_lists_vendor_env_vars() {
        let metadata = SalesforceAuthConfigMetadata::integration_user_oauth();
        let names = metadata
            .env_vars
            .iter()
            .map(|env_var| env_var.name)
            .collect::<Vec<_>>();

        assert!(metadata.integration_user);
        assert_eq!(
            metadata.supported_flows,
            [
                SalesforceAuthFlow::JwtBearer,
                SalesforceAuthFlow::ClientCredentials
            ]
        );
        assert_eq!(
            names,
            vec![
                SALESFORCE_INSTANCE_URL_ENV,
                SALESFORCE_LOGIN_BASE_URL_ENV,
                SALESFORCE_CLIENT_ID_ENV,
                SALESFORCE_CLIENT_SECRET_OR_PRIVATE_KEY_ENV,
                SALESFORCE_USERNAME_OR_SUBJECT_ENV,
            ]
        );
        assert_eq!(metadata.authorization_header, "Authorization");
        assert_eq!(metadata.redacted_bearer_value, "Bearer <redacted>");
    }

    #[test]
    fn client_credentials_metadata_does_not_require_jwt_subject() {
        let metadata = SalesforceAuthConfigMetadata::integration_user_oauth();
        let names = metadata
            .env_vars_for_flow(SalesforceAuthFlow::ClientCredentials)
            .map(|env_var| env_var.name)
            .collect::<Vec<_>>();

        assert_eq!(
            names,
            vec![
                SALESFORCE_INSTANCE_URL_ENV,
                SALESFORCE_LOGIN_BASE_URL_ENV,
                SALESFORCE_CLIENT_ID_ENV,
                SALESFORCE_CLIENT_SECRET_OR_PRIVATE_KEY_ENV,
            ]
        );
    }

    #[test]
    fn jwt_metadata_requires_subject_and_marks_secret_material() {
        let metadata = SalesforceAuthConfigMetadata::integration_user_oauth();
        let env_vars = metadata
            .env_vars_for_flow(SalesforceAuthFlow::JwtBearer)
            .collect::<Vec<_>>();

        assert!(env_vars
            .iter()
            .any(|env_var| env_var.name == SALESFORCE_USERNAME_OR_SUBJECT_ENV));
        assert!(env_vars.iter().any(|env_var| {
            env_var.name == SALESFORCE_CLIENT_SECRET_OR_PRIVATE_KEY_ENV && env_var.secret
        }));
    }

    #[test]
    fn tenant_binding_accepts_server_side_https_origins() {
        let binding = SalesforceTenantBinding::new(
            "https://example.my.salesforce.com",
            "https://login.salesforce.com",
        )
        .expect("tenant binding");

        assert_eq!(binding.instance_host(), "example.my.salesforce.com");
        assert_eq!(binding.login_host(), "login.salesforce.com");
    }

    #[test]
    fn tenant_binding_rejects_paths_queries_and_userinfo() {
        assert_eq!(
            SalesforceTenantBinding::new(
                "https://example.my.salesforce.com/services/data",
                "https://login.salesforce.com",
            )
            .unwrap_err(),
            SalesforceProviderError::InvalidParameter {
                name: SALESFORCE_INSTANCE_URL_ENV,
                reason: "must be an HTTPS origin without path, query, or fragment",
            }
        );
        assert_eq!(
            SalesforceTenantBinding::new(
                "https://example.my.salesforce.com",
                "https://login.salesforce.com?tenant=evil",
            )
            .unwrap_err(),
            SalesforceProviderError::InvalidParameter {
                name: SALESFORCE_LOGIN_BASE_URL_ENV,
                reason: "must be an HTTPS origin without path, query, or fragment",
            }
        );
        assert_eq!(
            SalesforceTenantBinding::new(
                "https://user@example.my.salesforce.com",
                "https://login.salesforce.com",
            )
            .unwrap_err(),
            SalesforceProviderError::InvalidParameter {
                name: SALESFORCE_INSTANCE_URL_ENV,
                reason: "must include a DNS host without userinfo or port",
            }
        );
    }

    #[test]
    fn tenant_binding_rejects_non_https_and_header_injection() {
        assert_eq!(
            SalesforceTenantBinding::new(
                "http://example.my.salesforce.com",
                "https://login.salesforce.com",
            )
            .unwrap_err(),
            SalesforceProviderError::InvalidParameter {
                name: SALESFORCE_INSTANCE_URL_ENV,
                reason: "must be an HTTPS origin without path, query, or fragment",
            }
        );
        assert_eq!(
            SalesforceTenantBinding::new(
                "https://example.my.salesforce.com\r\nx-evil: yes",
                "https://login.salesforce.com",
            )
            .unwrap_err(),
            SalesforceProviderError::InvalidParameter {
                name: SALESFORCE_INSTANCE_URL_ENV,
                reason: "must be a single-line HTTPS origin without whitespace",
            }
        );
    }
}
