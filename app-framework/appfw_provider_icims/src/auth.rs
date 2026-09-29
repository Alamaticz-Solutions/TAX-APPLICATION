use crate::IcimsProviderError;

pub const ICIMS_BASE_URL_ENV: &str = "ICIMS_BASE_URL";
pub const ICIMS_AUTH_MODE_ENV: &str = "ICIMS_AUTH_MODE";
pub const ICIMS_CREDENTIAL_REFERENCE_ENV: &str = "ICIMS_CREDENTIAL_REFERENCE";
pub const ICIMS_CUSTOMER_ID_ENV: &str = "ICIMS_CUSTOMER_ID";
pub const ICIMS_INTEGRATION_NAME_ENV: &str = "ICIMS_INTEGRATION_NAME";
pub const ICIMS_MARKETPLACE_VALIDATION_ENV: &str = "ICIMS_MARKETPLACE_VALIDATION";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IcimsAuthFlow {
    AccessGatedUnknown,
}

impl IcimsAuthFlow {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AccessGatedUnknown => "access_gated_unknown",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IcimsAuthEnvVar {
    pub name: &'static str,
    pub required_for: Option<IcimsAuthFlow>,
    pub secret: bool,
    pub description: &'static str,
}

impl IcimsAuthEnvVar {
    pub fn is_required_for(self, flow: IcimsAuthFlow) -> bool {
        self.required_for.is_none() || self.required_for == Some(flow)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IcimsAuthConfigMetadata {
    pub preferred_flow: IcimsAuthFlow,
    pub supported_flows: &'static [IcimsAuthFlow],
    pub env_vars: &'static [IcimsAuthEnvVar],
}

impl IcimsAuthConfigMetadata {
    pub fn authenticated_docs_required() -> Self {
        Self {
            preferred_flow: IcimsAuthFlow::AccessGatedUnknown,
            supported_flows: &ICIMS_AUTH_FLOWS,
            env_vars: &ICIMS_AUTH_ENV_VARS,
        }
    }

    pub fn env_vars_for_flow(self, flow: IcimsAuthFlow) -> impl Iterator<Item = IcimsAuthEnvVar> {
        self.env_vars
            .iter()
            .copied()
            .filter(move |env_var| env_var.is_required_for(flow))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IcimsTenantBinding {
    pub base_url: String,
    customer_id: String,
    integration_name: String,
    marketplace_validation: Option<String>,
}

impl IcimsTenantBinding {
    pub fn new(
        base_url: impl Into<String>,
        customer_id: impl Into<String>,
        integration_name: impl Into<String>,
    ) -> Result<Self, IcimsProviderError> {
        Self::with_marketplace_validation(base_url, customer_id, integration_name, None::<String>)
    }

    pub fn with_marketplace_validation(
        base_url: impl Into<String>,
        customer_id: impl Into<String>,
        integration_name: impl Into<String>,
        marketplace_validation: Option<impl Into<String>>,
    ) -> Result<Self, IcimsProviderError> {
        let base_url = base_url.into();
        let customer_id = customer_id.into();
        let integration_name = integration_name.into();
        let marketplace_validation = marketplace_validation.map(Into::into);

        validate_https_origin(ICIMS_BASE_URL_ENV, &base_url)?;
        validate_token(
            ICIMS_CUSTOMER_ID_ENV,
            &customer_id,
            "must identify the server-bound iCIMS customer or sandbox",
        )?;
        validate_single_line_non_empty(
            ICIMS_INTEGRATION_NAME_ENV,
            &integration_name,
            "must identify the server-bound iCIMS integration name",
        )?;
        if let Some(marketplace_validation) = &marketplace_validation {
            validate_single_line_non_empty(
                ICIMS_MARKETPLACE_VALIDATION_ENV,
                marketplace_validation,
                "must identify marketplace validation or revalidation evidence",
            )?;
        }

        Ok(Self {
            base_url,
            customer_id,
            integration_name,
            marketplace_validation,
        })
    }

    pub fn host(&self) -> &str {
        https_origin_host(&self.base_url).expect("validated iCIMS origin")
    }

    pub fn customer_id(&self) -> &str {
        &self.customer_id
    }

    pub fn integration_name(&self) -> &str {
        &self.integration_name
    }

    pub fn marketplace_validation(&self) -> Option<&str> {
        self.marketplace_validation.as_deref()
    }
}

pub const ICIMS_AUTH_FLOWS: [IcimsAuthFlow; 1] = [IcimsAuthFlow::AccessGatedUnknown];

pub const ICIMS_AUTH_ENV_VARS: [IcimsAuthEnvVar; 6] = [
    IcimsAuthEnvVar {
        name: ICIMS_BASE_URL_ENV,
        required_for: None,
        secret: false,
        description: "Tenant-specific iCIMS HTTPS origin.",
    },
    IcimsAuthEnvVar {
        name: ICIMS_AUTH_MODE_ENV,
        required_for: None,
        secret: false,
        description:
            "Access-gated auth mode from authenticated Developer Community or tenant docs.",
    },
    IcimsAuthEnvVar {
        name: ICIMS_CREDENTIAL_REFERENCE_ENV,
        required_for: Some(IcimsAuthFlow::AccessGatedUnknown),
        secret: true,
        description:
            "Opaque credential reference only after authenticated docs confirm the credential type.",
    },
    IcimsAuthEnvVar {
        name: ICIMS_CUSTOMER_ID_ENV,
        required_for: None,
        secret: false,
        description: "Server-bound iCIMS customer or sandbox identifier.",
    },
    IcimsAuthEnvVar {
        name: ICIMS_INTEGRATION_NAME_ENV,
        required_for: None,
        secret: false,
        description: "Server-bound marketplace/integration name.",
    },
    IcimsAuthEnvVar {
        name: ICIMS_MARKETPLACE_VALIDATION_ENV,
        required_for: None,
        secret: false,
        description: "Optional marketplace validation or revalidation evidence reference.",
    },
];

fn validate_https_origin(name: &'static str, value: &str) -> Result<(), IcimsProviderError> {
    if value.trim() != value
        || value.is_empty()
        || value
            .chars()
            .any(|character| character.is_ascii_control() || character.is_ascii_whitespace())
    {
        return Err(IcimsProviderError::InvalidParameter {
            name,
            reason: "must be a single-line HTTPS origin without whitespace",
        });
    }

    let host = https_origin_host(value).ok_or(IcimsProviderError::InvalidParameter {
        name,
        reason: "must be an HTTPS origin without path, query, or fragment",
    })?;

    if host.is_empty() || host.contains('@') || host.contains(':') || !host.contains('.') {
        return Err(IcimsProviderError::InvalidParameter {
            name,
            reason: "must include a DNS host without userinfo or port",
        });
    }

    Ok(())
}

fn validate_token(
    name: &'static str,
    value: &str,
    reason: &'static str,
) -> Result<(), IcimsProviderError> {
    validate_single_line_non_empty(name, value, reason)?;

    if value
        .chars()
        .any(|character| !(character.is_ascii_alphanumeric() || matches!(character, '-' | '_')))
    {
        return Err(IcimsProviderError::InvalidParameter { name, reason });
    }

    Ok(())
}

fn validate_single_line_non_empty(
    name: &'static str,
    value: &str,
    reason: &'static str,
) -> Result<(), IcimsProviderError> {
    if value.trim() != value
        || value.is_empty()
        || value
            .chars()
            .any(|character| character.is_ascii_control() || matches!(character, '\n' | '\r'))
    {
        return Err(IcimsProviderError::InvalidParameter { name, reason });
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
    fn auth_metadata_requires_authenticated_docs_before_auth_claims() {
        let metadata = IcimsAuthConfigMetadata::authenticated_docs_required();
        let env_vars = metadata
            .env_vars_for_flow(IcimsAuthFlow::AccessGatedUnknown)
            .collect::<Vec<_>>();

        assert_eq!(metadata.preferred_flow, IcimsAuthFlow::AccessGatedUnknown);
        assert!(env_vars
            .iter()
            .any(|env_var| env_var.name == ICIMS_CUSTOMER_ID_ENV));
        assert!(env_vars
            .iter()
            .any(|env_var| env_var.name == ICIMS_MARKETPLACE_VALIDATION_ENV));
        assert!(env_vars
            .iter()
            .any(|env_var| env_var.name == ICIMS_CREDENTIAL_REFERENCE_ENV && env_var.secret));
    }

    #[test]
    fn tenant_binding_accepts_https_origin_customer_and_marketplace_reference() {
        let binding = IcimsTenantBinding::with_marketplace_validation(
            "https://api.icims.example.com",
            "customer_123",
            "appfw recruiting integration",
            Some("marketplace-validation-2026"),
        )
        .expect("tenant binding");

        assert_eq!(binding.host(), "api.icims.example.com");
        assert_eq!(binding.customer_id(), "customer_123");
        assert_eq!(binding.integration_name(), "appfw recruiting integration");
        assert_eq!(
            binding.marketplace_validation(),
            Some("marketplace-validation-2026")
        );
    }

    #[test]
    fn tenant_binding_rejects_paths_queries_and_untrusted_customer_ids() {
        assert!(IcimsTenantBinding::new(
            "https://api.icims.example.com/rest/candidates",
            "customer_123",
            "integration"
        )
        .is_err());
        assert!(IcimsTenantBinding::new(
            "https://api.icims.example.com?candidate=1",
            "customer_123",
            "integration"
        )
        .is_err());
        assert!(IcimsTenantBinding::new(
            "https://api.icims.example.com:443",
            "customer_123",
            "integration"
        )
        .is_err());
        assert!(IcimsTenantBinding::new(
            "https://api.icims.example.com",
            "../customer",
            "integration"
        )
        .is_err());
    }
}
