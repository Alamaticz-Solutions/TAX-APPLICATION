use crate::SharePointProviderError;

// SharePoint-specific contract.
pub const SHAREPOINT_SITE_ID_ENV: &str = "SHAREPOINT_SITE_ID";
pub const SHAREPOINT_DRIVE_ID_ENV: &str = "SHAREPOINT_DRIVE_ID";
pub const SHAREPOINT_AUTH_MODE_ENV: &str = "SHAREPOINT_AUTH_MODE";

// Shared Microsoft Graph app registrations (same tenant, same two apps the
// Teams-meeting integration uses in Project Governance). Named once, here, so
// the SharePoint provider never invents a third/fourth credential pair.
pub const SHAREPOINT_TENANT_ID_ENV: &str = "GRAPH_TENANT_ID";
pub const SHAREPOINT_READ_CLIENT_ID_ENV: &str = "GRAPH_CLIENT_ID";
pub const SHAREPOINT_READ_CLIENT_SECRET_ENV: &str = "GRAPH_CLIENT_SECRET";
pub const SHAREPOINT_WRITE_CLIENT_ID_ENV: &str = "GRAPH_WRITE_CLIENT_ID";
pub const SHAREPOINT_WRITE_CLIENT_SECRET_ENV: &str = "GRAPH_WRITE_CLIENT_SECRET";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharePointAuthFlow {
    OAuthClientCredentials,
}

impl SharePointAuthFlow {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OAuthClientCredentials => "oauth_client_credentials",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SharePointAuthEnvVar {
    pub name: &'static str,
    pub required_for: Option<SharePointAuthFlow>,
    pub secret: bool,
    pub description: &'static str,
}

impl SharePointAuthEnvVar {
    pub fn is_required_for(self, flow: SharePointAuthFlow) -> bool {
        self.required_for.is_none() || self.required_for == Some(flow)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SharePointAuthConfigMetadata {
    pub preferred_flow: SharePointAuthFlow,
    pub supported_flows: &'static [SharePointAuthFlow],
    pub env_vars: &'static [SharePointAuthEnvVar],
}

impl SharePointAuthConfigMetadata {
    pub fn tenant_site_auth() -> Self {
        Self {
            preferred_flow: SharePointAuthFlow::OAuthClientCredentials,
            supported_flows: &SHAREPOINT_AUTH_FLOWS,
            env_vars: &SHAREPOINT_AUTH_ENV_VARS,
        }
    }

    pub fn env_vars_for_flow(
        self,
        flow: SharePointAuthFlow,
    ) -> impl Iterator<Item = SharePointAuthEnvVar> {
        self.env_vars
            .iter()
            .copied()
            .filter(move |env_var| env_var.is_required_for(flow))
    }
}

/// Which SharePoint site + drive this data source targets. Nothing here is
/// server-invented: every value round-trips from the admin handoff
/// (`docs/architecture/sharepoint-graph-admin-handoff.md`), never constructed
/// or guessed (decision D3, `sharepoint-integration-plan.md`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharePointTenantBinding {
    site_id: String,
    drive_id: String,
}

impl SharePointTenantBinding {
    pub fn new(
        site_id: impl Into<String>,
        drive_id: impl Into<String>,
    ) -> Result<Self, SharePointProviderError> {
        let site_id = site_id.into();
        let drive_id = drive_id.into();
        validate_single_line_non_empty(
            SHAREPOINT_SITE_ID_ENV,
            &site_id,
            "must be the Graph site id returned by GET /sites/{host}:/sites/{name}",
        )?;
        validate_single_line_non_empty(
            SHAREPOINT_DRIVE_ID_ENV,
            &drive_id,
            "must be the Graph drive id returned by GET /sites/{site-id}/drives",
        )?;
        Ok(Self { site_id, drive_id })
    }

    pub fn site_id(&self) -> &str {
        &self.site_id
    }

    pub fn drive_id(&self) -> &str {
        &self.drive_id
    }
}

pub const SHAREPOINT_AUTH_FLOWS: [SharePointAuthFlow; 1] =
    [SharePointAuthFlow::OAuthClientCredentials];

pub const SHAREPOINT_AUTH_ENV_VARS: [SharePointAuthEnvVar; 8] = [
    SharePointAuthEnvVar {
        name: SHAREPOINT_SITE_ID_ENV,
        required_for: None,
        secret: false,
        description: "Graph site id for the dedicated SharePoint document site.",
    },
    SharePointAuthEnvVar {
        name: SHAREPOINT_DRIVE_ID_ENV,
        required_for: None,
        secret: false,
        description: "Graph drive id for the document library inside that site.",
    },
    SharePointAuthEnvVar {
        name: SHAREPOINT_AUTH_MODE_ENV,
        required_for: None,
        secret: false,
        description: "Tenant-confirmed auth mode; live auth remains certification-gated.",
    },
    SharePointAuthEnvVar {
        name: SHAREPOINT_TENANT_ID_ENV,
        required_for: Some(SharePointAuthFlow::OAuthClientCredentials),
        secret: false,
        description: "Microsoft Entra tenant (directory) id - single tenant.",
    },
    SharePointAuthEnvVar {
        name: SHAREPOINT_READ_CLIENT_ID_ENV,
        required_for: Some(SharePointAuthFlow::OAuthClientCredentials),
        secret: false,
        description: "Read-scoped Entra app registration client id (shared with the Graph integration).",
    },
    SharePointAuthEnvVar {
        name: SHAREPOINT_READ_CLIENT_SECRET_ENV,
        required_for: Some(SharePointAuthFlow::OAuthClientCredentials),
        secret: true,
        description: "Read-scoped Entra app registration client secret value.",
    },
    SharePointAuthEnvVar {
        name: SHAREPOINT_WRITE_CLIENT_ID_ENV,
        required_for: Some(SharePointAuthFlow::OAuthClientCredentials),
        secret: false,
        description: "Write-scoped Entra app registration client id (shared with the Graph integration, decision D4).",
    },
    SharePointAuthEnvVar {
        name: SHAREPOINT_WRITE_CLIENT_SECRET_ENV,
        required_for: Some(SharePointAuthFlow::OAuthClientCredentials),
        secret: true,
        description: "Write-scoped Entra app registration client secret value.",
    },
];

fn validate_single_line_non_empty(
    name: &'static str,
    value: &str,
    reason: &'static str,
) -> Result<(), SharePointProviderError> {
    if value.trim() != value || value.is_empty() || value.chars().any(|c| c.is_ascii_control()) {
        return Err(SharePointProviderError::InvalidParameter { name, reason });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_metadata_prefers_oauth_client_credentials_and_marks_secrets() {
        let metadata = SharePointAuthConfigMetadata::tenant_site_auth();
        let env_vars = metadata
            .env_vars_for_flow(SharePointAuthFlow::OAuthClientCredentials)
            .collect::<Vec<_>>();

        assert_eq!(
            metadata.preferred_flow,
            SharePointAuthFlow::OAuthClientCredentials
        );
        assert!(env_vars
            .iter()
            .any(|e| e.name == SHAREPOINT_WRITE_CLIENT_SECRET_ENV && e.secret));
        assert!(env_vars
            .iter()
            .any(|e| e.name == SHAREPOINT_READ_CLIENT_SECRET_ENV && e.secret));
    }

    #[test]
    fn only_client_secrets_are_marked_secret() {
        for env_var in SHAREPOINT_AUTH_ENV_VARS {
            assert_eq!(env_var.secret, env_var.name.ends_with("CLIENT_SECRET"));
        }
    }

    #[test]
    fn tenant_binding_accepts_a_site_and_drive_id() {
        let binding = SharePointTenantBinding::new("contoso.sharepoint.com,guid,guid", "b!driveid")
            .expect("tenant binding");
        assert_eq!(binding.site_id(), "contoso.sharepoint.com,guid,guid");
        assert_eq!(binding.drive_id(), "b!driveid");
    }

    #[test]
    fn tenant_binding_rejects_blank_or_whitespace_padded_ids() {
        assert!(SharePointTenantBinding::new("", "b!driveid").is_err());
        assert!(SharePointTenantBinding::new(" site ", "b!driveid").is_err());
        assert!(SharePointTenantBinding::new("site", "").is_err());
    }
}
