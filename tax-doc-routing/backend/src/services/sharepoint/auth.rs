//! SharePoint's own env-var contract: which site and which document library,
//! on top of the two Graph app registrations (read + write) the product shares.
//!
//! Decision D4 (`docs/architecture/sharepoint-integration-plan.md`): reuse the
//! same two Entra app registrations as the Graph integration rather than
//! register more. Token acquisition therefore comes from
//! `services::graph::auth` - re-typing the token cache here would only create a
//! second copy to keep in sync.
//!
//! What is specific to this module: which SharePoint site and drive to target.

use std::env;

use appfw_provider_sharepoint::{
    SHAREPOINT_AUTH_MODE_ENV, SHAREPOINT_DRIVE_ID_ENV, SHAREPOINT_SITE_ID_ENV,
};

use crate::services::graph::auth::GraphAuthConfig;

pub const ENV_SITE_ID: &str = SHAREPOINT_SITE_ID_ENV;
pub const ENV_DRIVE_ID: &str = SHAREPOINT_DRIVE_ID_ENV;
pub const ENV_AUTH_MODE: &str = SHAREPOINT_AUTH_MODE_ENV;

/// The only auth mode this module implements.
pub const SUPPORTED_AUTH_MODE: &str = "oauth_client_credentials";

/// Site + drive to target. Never constructed from caller input and never given
/// a placeholder default: the values come from the admin handoff
/// (`docs/architecture/sharepoint-graph-admin-handoff.md`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharePointTarget {
    pub site_id: String,
    pub drive_id: String,
}

impl SharePointTarget {
    /// `None` until the Microsoft 365 admin has handed back real values, or if
    /// `SHAREPOINT_AUTH_MODE` is set to something this module does not
    /// implement (fail closed rather than guess).
    pub fn from_env() -> Option<Self> {
        let site_id = env::var(ENV_SITE_ID).ok().filter(|s| !s.is_empty())?;
        let drive_id = env::var(ENV_DRIVE_ID).ok().filter(|s| !s.is_empty())?;
        if let Some(mode) = env::var(ENV_AUTH_MODE).ok().filter(|s| !s.is_empty()) {
            if mode != SUPPORTED_AUTH_MODE {
                tracing::warn!(
                    configured = %mode,
                    supported = SUPPORTED_AUTH_MODE,
                    "SHAREPOINT_AUTH_MODE is not supported; SharePoint stays unconfigured"
                );
                return None;
            }
        }
        Some(Self { site_id, drive_id })
    }
}

/// Read-scoped credential (shared Graph read app) + target.
pub fn read_config() -> Option<(GraphAuthConfig, SharePointTarget)> {
    Some((GraphAuthConfig::from_env()?, SharePointTarget::from_env()?))
}

/// Write-scoped credential (shared Graph write app) + target.
///
/// Unlike a "fall back to the read app" policy, this fails closed: a missing
/// write registration means SharePoint writes are unconfigured, never that
/// uploads silently run with the read app's identity.
pub fn write_config() -> Option<(GraphAuthConfig, SharePointTarget)> {
    Some((GraphAuthConfig::from_write_env()?, SharePointTarget::from_env()?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::graph::auth::{
        ENV_CLIENT_ID, ENV_CLIENT_SECRET, ENV_LOCK, ENV_TENANT_ID, ENV_WRITE_CLIENT_ID,
        ENV_WRITE_CLIENT_SECRET,
    };

    fn clear() {
        for name in [
            ENV_SITE_ID,
            ENV_DRIVE_ID,
            ENV_AUTH_MODE,
            ENV_TENANT_ID,
            ENV_CLIENT_ID,
            ENV_CLIENT_SECRET,
            ENV_WRITE_CLIENT_ID,
            ENV_WRITE_CLIENT_SECRET,
        ] {
            env::remove_var(name);
        }
    }

    #[test]
    fn env_names_are_the_ones_the_brief_specifies() {
        assert_eq!(ENV_SITE_ID, "SHAREPOINT_SITE_ID");
        assert_eq!(ENV_DRIVE_ID, "SHAREPOINT_DRIVE_ID");
        assert_eq!(ENV_AUTH_MODE, "SHAREPOINT_AUTH_MODE");
    }

    #[test]
    fn target_requires_both_site_and_drive() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear();
        assert!(SharePointTarget::from_env().is_none());

        env::set_var(ENV_SITE_ID, "contoso.sharepoint.com,guid,guid");
        assert!(SharePointTarget::from_env().is_none());

        env::set_var(ENV_DRIVE_ID, "b!driveid");
        let target = SharePointTarget::from_env().expect("both set");
        assert_eq!(target.site_id, "contoso.sharepoint.com,guid,guid");
        assert_eq!(target.drive_id, "b!driveid");
        clear();
    }

    #[test]
    fn unsupported_auth_mode_fails_closed() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear();
        env::set_var(ENV_SITE_ID, "s");
        env::set_var(ENV_DRIVE_ID, "d");
        env::set_var(ENV_AUTH_MODE, "delegated");
        assert!(SharePointTarget::from_env().is_none());
        env::set_var(ENV_AUTH_MODE, SUPPORTED_AUTH_MODE);
        assert!(SharePointTarget::from_env().is_some());
        clear();
    }

    #[test]
    fn write_config_never_falls_back_to_the_read_app() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear();
        env::set_var(ENV_SITE_ID, "s");
        env::set_var(ENV_DRIVE_ID, "d");
        env::set_var(ENV_TENANT_ID, "t");
        env::set_var(ENV_CLIENT_ID, "read-id");
        env::set_var(ENV_CLIENT_SECRET, "read-secret");

        assert!(read_config().is_some());
        assert!(write_config().is_none());

        env::set_var(ENV_WRITE_CLIENT_ID, "write-id");
        env::set_var(ENV_WRITE_CLIENT_SECRET, "write-secret");
        let (cfg, _) = write_config().expect("write configured");
        assert_eq!(cfg.client_id, "write-id");
        clear();
    }
}
