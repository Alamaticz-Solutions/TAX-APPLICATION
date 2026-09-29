use crate::OracleFinancialsProviderError;

pub const ORACLE_FINANCIALS_BASE_URL_ENV: &str = "ORACLE_FINANCIALS_BASE_URL";
pub const ORACLE_FINANCIALS_AUTH_MODE_ENV: &str = "ORACLE_FINANCIALS_AUTH_MODE";
pub const ORACLE_FINANCIALS_CLIENT_ID_ENV: &str = "ORACLE_FINANCIALS_CLIENT_ID";
pub const ORACLE_FINANCIALS_CLIENT_SECRET_ENV: &str = "ORACLE_FINANCIALS_CLIENT_SECRET";
pub const ORACLE_FINANCIALS_USERNAME_ENV: &str = "ORACLE_FINANCIALS_USERNAME";
pub const ORACLE_FINANCIALS_PASSWORD_ENV: &str = "ORACLE_FINANCIALS_PASSWORD";
pub const ORACLE_FINANCIALS_REST_FRAMEWORK_VERSION_ENV: &str =
    "ORACLE_FINANCIALS_REST_FRAMEWORK_VERSION";
pub const ORACLE_FINANCIALS_METADATA_CONTEXT_ENV: &str = "ORACLE_FINANCIALS_METADATA_CONTEXT";
pub const ORACLE_FINANCIALS_DATA_SECURITY_SCOPE_ENV: &str = "ORACLE_FINANCIALS_DATA_SECURITY_SCOPE";
pub const ORACLE_FINANCIALS_AUTHORIZATION_HEADER: &str = "Authorization";
pub const ORACLE_FINANCIALS_REDACTED_BEARER_TOKEN: &str = "Bearer <redacted>";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OracleFinancialsAuthFlow {
    TenantOauthOrOidc,
    BasicUserPassword,
}

impl OracleFinancialsAuthFlow {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TenantOauthOrOidc => "tenant_oauth_or_oidc",
            Self::BasicUserPassword => "basic_user_password",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OracleFinancialsAuthEnvVar {
    pub name: &'static str,
    pub required_for: Option<OracleFinancialsAuthFlow>,
    pub secret: bool,
    pub description: &'static str,
}

impl OracleFinancialsAuthEnvVar {
    pub fn is_required_for(self, flow: OracleFinancialsAuthFlow) -> bool {
        self.required_for.is_none() || self.required_for == Some(flow)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OracleFinancialsAuthConfigMetadata {
    pub preferred_flow: OracleFinancialsAuthFlow,
    pub supported_flows: &'static [OracleFinancialsAuthFlow],
    pub env_vars: &'static [OracleFinancialsAuthEnvVar],
    pub authorization_header: &'static str,
    pub redacted_token_value: &'static str,
}

impl OracleFinancialsAuthConfigMetadata {
    pub fn tenant_configured_auth() -> Self {
        Self {
            preferred_flow: OracleFinancialsAuthFlow::TenantOauthOrOidc,
            supported_flows: &ORACLE_FINANCIALS_AUTH_FLOWS,
            env_vars: &ORACLE_FINANCIALS_AUTH_ENV_VARS,
            authorization_header: ORACLE_FINANCIALS_AUTHORIZATION_HEADER,
            redacted_token_value: ORACLE_FINANCIALS_REDACTED_BEARER_TOKEN,
        }
    }

    pub fn env_vars_for_flow(
        self,
        flow: OracleFinancialsAuthFlow,
    ) -> impl Iterator<Item = OracleFinancialsAuthEnvVar> {
        self.env_vars
            .iter()
            .copied()
            .filter(move |env_var| env_var.is_required_for(flow))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OracleFinancialsTenantBinding {
    pub base_url: String,
    rest_framework_version: String,
    metadata_context: Option<String>,
    data_security_scope: String,
}

impl OracleFinancialsTenantBinding {
    pub fn new(
        base_url: impl Into<String>,
        rest_framework_version: impl Into<String>,
        data_security_scope: impl Into<String>,
    ) -> Result<Self, OracleFinancialsProviderError> {
        Self::with_metadata_context(
            base_url,
            rest_framework_version,
            None::<String>,
            data_security_scope,
        )
    }

    pub fn with_metadata_context(
        base_url: impl Into<String>,
        rest_framework_version: impl Into<String>,
        metadata_context: Option<impl Into<String>>,
        data_security_scope: impl Into<String>,
    ) -> Result<Self, OracleFinancialsProviderError> {
        let base_url = base_url.into();
        let rest_framework_version = rest_framework_version.into();
        let metadata_context = metadata_context.map(Into::into);
        let data_security_scope = data_security_scope.into();

        validate_oracle_https_origin(ORACLE_FINANCIALS_BASE_URL_ENV, &base_url)?;
        validate_oracle_token(
            ORACLE_FINANCIALS_REST_FRAMEWORK_VERSION_ENV,
            &rest_framework_version,
            "must be a single-line REST framework version token",
        )?;
        if let Some(metadata_context) = &metadata_context {
            validate_single_line_non_empty(
                ORACLE_FINANCIALS_METADATA_CONTEXT_ENV,
                metadata_context,
                "must be a single-line metadata context value",
            )?;
        }
        validate_single_line_non_empty(
            ORACLE_FINANCIALS_DATA_SECURITY_SCOPE_ENV,
            &data_security_scope,
            "must identify a server-bound ledger, business unit, or enterprise security scope",
        )?;

        Ok(Self {
            base_url,
            rest_framework_version,
            metadata_context,
            data_security_scope,
        })
    }

    pub fn host(&self) -> &str {
        https_origin_host(&self.base_url).expect("validated Oracle Financials origin")
    }

    pub fn rest_framework_version(&self) -> &str {
        &self.rest_framework_version
    }

    pub fn metadata_context(&self) -> Option<&str> {
        self.metadata_context.as_deref()
    }

    pub fn data_security_scope(&self) -> &str {
        &self.data_security_scope
    }
}

pub const ORACLE_FINANCIALS_AUTH_FLOWS: [OracleFinancialsAuthFlow; 2] = [
    OracleFinancialsAuthFlow::TenantOauthOrOidc,
    OracleFinancialsAuthFlow::BasicUserPassword,
];

pub const ORACLE_FINANCIALS_AUTH_ENV_VARS: [OracleFinancialsAuthEnvVar; 9] = [
    OracleFinancialsAuthEnvVar {
        name: ORACLE_FINANCIALS_BASE_URL_ENV,
        required_for: None,
        secret: false,
        description: "Tenant-specific Oracle Fusion Cloud Financials HTTPS origin.",
    },
    OracleFinancialsAuthEnvVar {
        name: ORACLE_FINANCIALS_AUTH_MODE_ENV,
        required_for: None,
        secret: false,
        description: "Tenant-confirmed auth mode; live auth remains certification-gated.",
    },
    OracleFinancialsAuthEnvVar {
        name: ORACLE_FINANCIALS_CLIENT_ID_ENV,
        required_for: Some(OracleFinancialsAuthFlow::TenantOauthOrOidc),
        secret: false,
        description: "Tenant-approved OAuth/OIDC client id, if that flow is certified.",
    },
    OracleFinancialsAuthEnvVar {
        name: ORACLE_FINANCIALS_CLIENT_SECRET_ENV,
        required_for: Some(OracleFinancialsAuthFlow::TenantOauthOrOidc),
        secret: true,
        description: "Tenant-approved OAuth/OIDC client secret or secret reference.",
    },
    OracleFinancialsAuthEnvVar {
        name: ORACLE_FINANCIALS_USERNAME_ENV,
        required_for: Some(OracleFinancialsAuthFlow::BasicUserPassword),
        secret: false,
        description: "Tenant-approved Oracle integration user for exception flows.",
    },
    OracleFinancialsAuthEnvVar {
        name: ORACLE_FINANCIALS_PASSWORD_ENV,
        required_for: Some(OracleFinancialsAuthFlow::BasicUserPassword),
        secret: true,
        description: "Tenant-approved Oracle integration password or secret reference.",
    },
    OracleFinancialsAuthEnvVar {
        name: ORACLE_FINANCIALS_REST_FRAMEWORK_VERSION_ENV,
        required_for: None,
        secret: false,
        description: "Server-bound Oracle REST framework version header value.",
    },
    OracleFinancialsAuthEnvVar {
        name: ORACLE_FINANCIALS_METADATA_CONTEXT_ENV,
        required_for: None,
        secret: false,
        description: "Optional server-bound Oracle Metadata-Context header value.",
    },
    OracleFinancialsAuthEnvVar {
        name: ORACLE_FINANCIALS_DATA_SECURITY_SCOPE_ENV,
        required_for: None,
        secret: false,
        description: "Server-bound ledger, business unit, or enterprise data-security scope label.",
    },
];

fn validate_oracle_https_origin(
    name: &'static str,
    value: &str,
) -> Result<(), OracleFinancialsProviderError> {
    if value.trim() != value
        || value.is_empty()
        || value
            .chars()
            .any(|character| character.is_ascii_control() || character.is_ascii_whitespace())
    {
        return Err(OracleFinancialsProviderError::InvalidParameter {
            name,
            reason: "must be a single-line HTTPS origin without whitespace",
        });
    }

    let host = https_origin_host(value).ok_or(OracleFinancialsProviderError::InvalidParameter {
        name,
        reason: "must be an HTTPS origin without path, query, or fragment",
    })?;

    if host.is_empty() || host.contains('@') || host.contains(':') || !host.contains('.') {
        return Err(OracleFinancialsProviderError::InvalidParameter {
            name,
            reason: "must include a DNS host without userinfo or port",
        });
    }

    Ok(())
}

fn validate_oracle_token(
    name: &'static str,
    value: &str,
    reason: &'static str,
) -> Result<(), OracleFinancialsProviderError> {
    validate_single_line_non_empty(name, value, reason)?;

    if value.chars().any(|character| {
        !(character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_'))
    }) {
        return Err(OracleFinancialsProviderError::InvalidParameter { name, reason });
    }

    Ok(())
}

fn validate_single_line_non_empty(
    name: &'static str,
    value: &str,
    reason: &'static str,
) -> Result<(), OracleFinancialsProviderError> {
    if value.trim() != value
        || value.is_empty()
        || value
            .chars()
            .any(|character| character.is_ascii_control() || matches!(character, '\n' | '\r'))
    {
        return Err(OracleFinancialsProviderError::InvalidParameter { name, reason });
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
    fn auth_metadata_prefers_tenant_configured_oauth_and_marks_secrets() {
        let metadata = OracleFinancialsAuthConfigMetadata::tenant_configured_auth();
        let env_vars = metadata
            .env_vars_for_flow(OracleFinancialsAuthFlow::TenantOauthOrOidc)
            .collect::<Vec<_>>();

        assert_eq!(
            metadata.preferred_flow,
            OracleFinancialsAuthFlow::TenantOauthOrOidc
        );
        assert!(env_vars
            .iter()
            .any(|env_var| env_var.name == ORACLE_FINANCIALS_CLIENT_SECRET_ENV && env_var.secret));
        assert!(env_vars
            .iter()
            .any(|env_var| env_var.name == ORACLE_FINANCIALS_BASE_URL_ENV));
    }

    #[test]
    fn tenant_binding_accepts_https_origin_and_scope() {
        let binding = OracleFinancialsTenantBinding::with_metadata_context(
            "https://tenant.fa.us2.oraclecloud.com",
            "26B",
            Some("sandbox"),
            "ledger:vision-operations",
        )
        .expect("tenant binding");

        assert_eq!(binding.host(), "tenant.fa.us2.oraclecloud.com");
        assert_eq!(binding.rest_framework_version(), "26B");
        assert_eq!(binding.metadata_context(), Some("sandbox"));
        assert_eq!(binding.data_security_scope(), "ledger:vision-operations");
    }

    #[test]
    fn tenant_binding_rejects_paths_ports_and_empty_scope() {
        assert!(OracleFinancialsTenantBinding::new(
            "https://tenant.fa.us2.oraclecloud.com/fscmRestApi/resources",
            "26B",
            "ledger"
        )
        .is_err());

        assert!(OracleFinancialsTenantBinding::new(
            "https://tenant.fa.us2.oraclecloud.com:443",
            "26B",
            "ledger"
        )
        .is_err());

        assert!(OracleFinancialsTenantBinding::new(
            "https://tenant.fa.us2.oraclecloud.com",
            "26B",
            ""
        )
        .is_err());
    }
}
