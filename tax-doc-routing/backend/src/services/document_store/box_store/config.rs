use std::fmt;
use std::path::PathBuf;

pub const DEFAULT_TOKEN_URL: &str = "https://api.box.com/oauth2/token";
pub const DEFAULT_API_BASE: &str = "https://api.box.com/2.0";
pub const DEFAULT_UPLOAD_BASE: &str = "https://upload.box.com/api/2.0";

/// Box app settings, read from the environment (never from the repository or the database).
///
/// The sign-in is Box server-side JWT authentication, the same mechanism Pega's `GenerateToken_Box`
/// uses with the PDSBOX keystore: the app signs a short-lived request with the keystore's private
/// key and trades it for a bearer token.
///
/// | Setting | What |
/// |---|---|
/// | `BOX_CLIENT_ID`, `BOX_CLIENT_SECRET` | the Box app's id and secret |
/// | `BOX_ENTERPRISE_ID` | the enterprise the app acts for |
/// | `BOX_PUBLIC_KEY_ID` | id of the public key registered for the keystore's private key |
/// | `BOX_KEYSTORE_PATH`, `BOX_KEYSTORE_PASSWORD` | the PKCS#12 file and its password |
/// | `BOX_TOKEN_URL`, `BOX_API_BASE_URL`, `BOX_UPLOAD_BASE_URL` | optional overrides |
#[derive(Clone)]
pub struct BoxConfig {
    pub client_id: String,
    pub client_secret: String,
    pub enterprise_id: String,
    pub public_key_id: String,
    pub keystore_path: PathBuf,
    pub keystore_password: String,
    pub token_url: String,
    pub api_base: String,
    pub upload_base: String,
}

// Secrets must never reach a log line, so Debug shows only the non-secret fields.
impl fmt::Debug for BoxConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BoxConfig")
            .field("client_id", &self.client_id)
            .field("client_secret", &"<redacted>")
            .field("enterprise_id", &self.enterprise_id)
            .field("public_key_id", &self.public_key_id)
            .field("keystore_path", &self.keystore_path)
            .field("keystore_password", &"<redacted>")
            .field("token_url", &self.token_url)
            .finish()
    }
}

const REQUIRED: [&str; 6] = [
    "BOX_CLIENT_ID",
    "BOX_CLIENT_SECRET",
    "BOX_ENTERPRISE_ID",
    "BOX_PUBLIC_KEY_ID",
    "BOX_KEYSTORE_PATH",
    "BOX_KEYSTORE_PASSWORD",
];

impl BoxConfig {
    pub fn from_env() -> Result<Option<Self>, String> {
        Self::from_lookup(|k| std::env::var(k).ok())
    }

    /// `Ok(None)` when no Box setting is present (Box is simply not configured). Setting only some
    /// of them is an error that names what is missing: fail closed, never half-configured.
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Option<Self>, String> {
        let value = |k: &str| get(k).map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
        let present: Vec<&str> = REQUIRED.iter().copied().filter(|k| value(k).is_some()).collect();
        if present.is_empty() {
            return Ok(None);
        }
        let missing: Vec<&str> = REQUIRED.iter().copied().filter(|k| value(k).is_none()).collect();
        if !missing.is_empty() {
            return Err(format!("Box is partly configured; missing: {}", missing.join(", ")));
        }
        let need = |k: &str| value(k).expect("checked above");
        Ok(Some(Self {
            client_id: need("BOX_CLIENT_ID"),
            client_secret: need("BOX_CLIENT_SECRET"),
            enterprise_id: need("BOX_ENTERPRISE_ID"),
            public_key_id: need("BOX_PUBLIC_KEY_ID"),
            keystore_path: PathBuf::from(need("BOX_KEYSTORE_PATH")),
            keystore_password: need("BOX_KEYSTORE_PASSWORD"),
            token_url: value("BOX_TOKEN_URL").unwrap_or_else(|| DEFAULT_TOKEN_URL.to_string()),
            api_base: value("BOX_API_BASE_URL").unwrap_or_else(|| DEFAULT_API_BASE.to_string()).trim_end_matches('/').to_string(),
            upload_base: value("BOX_UPLOAD_BASE_URL").unwrap_or_else(|| DEFAULT_UPLOAD_BASE.to_string()).trim_end_matches('/').to_string(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn lookup(map: HashMap<&'static str, &'static str>) -> impl Fn(&str) -> Option<String> {
        move |k| map.get(k).map(|v| v.to_string())
    }

    fn full() -> HashMap<&'static str, &'static str> {
        HashMap::from([
            ("BOX_CLIENT_ID", "id"),
            ("BOX_CLIENT_SECRET", "super-secret"),
            ("BOX_ENTERPRISE_ID", "7233266"),
            ("BOX_PUBLIC_KEY_ID", "cofqpfps"),
            ("BOX_KEYSTORE_PATH", "keys/box.p12"),
            ("BOX_KEYSTORE_PASSWORD", "hunter2"),
        ])
    }

    #[test]
    fn unconfigured_is_none() {
        assert!(BoxConfig::from_lookup(lookup(HashMap::new())).unwrap().is_none());
    }

    #[test]
    fn partly_configured_fails_closed_and_names_what_is_missing() {
        let mut m = full();
        m.remove("BOX_PUBLIC_KEY_ID");
        let err = BoxConfig::from_lookup(lookup(m)).unwrap_err();
        assert!(err.contains("BOX_PUBLIC_KEY_ID") && !err.contains("BOX_CLIENT_ID"));
    }

    #[test]
    fn fully_configured_uses_defaults_for_the_urls() {
        let c = BoxConfig::from_lookup(lookup(full())).unwrap().unwrap();
        assert_eq!(c.enterprise_id, "7233266");
        assert_eq!(c.token_url, DEFAULT_TOKEN_URL);
        assert_eq!(c.api_base, DEFAULT_API_BASE);
    }

    #[test]
    fn debug_output_never_contains_a_secret() {
        let c = BoxConfig::from_lookup(lookup(full())).unwrap().unwrap();
        let shown = format!("{c:?}");
        assert!(!shown.contains("super-secret") && !shown.contains("hunter2"));
        assert!(shown.contains("<redacted>"));
    }
}
