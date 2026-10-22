//! Box server-side JWT sign-in (Pega's `GenerateToken_Box`, run against the PDSBOX key store).
//!
//! 1. Read the private key from the PKCS#12 key store.
//! 2. Sign a short-lived RS256 assertion naming the app, the enterprise and the public key id.
//! 3. POST it to the token URL with the client id and secret; Box returns a bearer token and how
//!    long it lives.
//!
//! This file only signs in. Saving the token, reusing it until it expires and renewing it is
//! `token_manager`.

use async_trait::async_trait;
use base64::Engine;
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};

use super::config::BoxConfig;
use super::token_manager::{MintedToken, TokenMinter};
use crate::services::document_store::StoreError;

const GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:jwt-bearer";
/// Box accepts an assertion lifetime of at most 60 seconds.
const ASSERTION_TTL_SECS: u64 = 45;
/// Used only if Box leaves `expires_in` out of the response.
const FALLBACK_LIFETIME_SECS: i64 = 3600;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct AssertionClaims {
    pub iss: String,
    pub sub: String,
    pub box_sub_type: &'static str,
    pub aud: String,
    pub jti: String,
    pub exp: u64,
}

/// The claims Box requires for an enterprise-scoped app token.
pub fn build_claims(config: &BoxConfig, now_unix: u64, jti: String) -> AssertionClaims {
    AssertionClaims {
        iss: config.client_id.clone(),
        sub: config.enterprise_id.clone(),
        box_sub_type: "enterprise",
        aud: config.token_url.clone(),
        jti,
        exp: now_unix + ASSERTION_TTL_SECS,
    }
}

/// Reads the private key from the PKCS#12 file and turns it into a signing key.
pub fn load_signing_key(config: &BoxConfig) -> Result<EncodingKey, String> {
    let bytes = std::fs::read(&config.keystore_path)
        .map_err(|e| format!("could not read the Box key store at {}: {e}", config.keystore_path.display()))?;
    let keystore = p12_keystore::KeyStore::from_pkcs12(&bytes, &config.keystore_password, p12_keystore::Pkcs12ImportPolicy::Relaxed)
        .map_err(|e| format!("could not open the Box key store (wrong password or unsupported format): {e}"))?;
    let (_alias, chain) = keystore
        .private_key_chain()
        .ok_or_else(|| "the Box key store holds no private key".to_string())?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(chain.key().as_der());
    let mut pem = String::from("-----BEGIN PRIVATE KEY-----\n");
    for line in b64.as_bytes().chunks(64) {
        pem.push_str(std::str::from_utf8(line).expect("base64 is ascii"));
        pem.push('\n');
    }
    pem.push_str("-----END PRIVATE KEY-----\n");
    EncodingKey::from_rsa_pem(pem.as_bytes()).map_err(|e| format!("the Box key store key is not a usable RSA key: {e}"))
}

pub fn sign_assertion(key: &EncodingKey, config: &BoxConfig, claims: &AssertionClaims) -> Result<String, String> {
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(config.public_key_id.clone());
    jsonwebtoken::encode(&header, claims, key).map_err(|e| format!("could not sign the Box assertion: {e}"))
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    expires_in: Option<i64>,
}

/// Signs in with Box and returns a fresh token with its expiry.
pub struct JwtMinter {
    config: BoxConfig,
    key: EncodingKey,
    http: reqwest::Client,
}

impl JwtMinter {
    pub fn new(config: BoxConfig) -> Result<Self, String> {
        let key = load_signing_key(&config)?;
        Ok(Self { config, key, http: reqwest::Client::new() })
    }
}

#[async_trait]
impl TokenMinter for JwtMinter {
    async fn mint(&self) -> Result<MintedToken, StoreError> {
        let now = Utc::now();
        let claims = build_claims(&self.config, now.timestamp().max(0) as u64, uuid::Uuid::new_v4().to_string());
        let assertion = sign_assertion(&self.key, &self.config, &claims).map_err(StoreError::Unavailable)?;
        let response = self
            .http
            .post(&self.config.token_url)
            .form(&[
                ("grant_type", GRANT_TYPE),
                ("client_id", self.config.client_id.as_str()),
                ("client_secret", self.config.client_secret.as_str()),
                ("assertion", assertion.as_str()),
            ])
            .send()
            .await
            .map_err(|e| StoreError::Unavailable(format!("Box sign-in could not be reached: {}", e.without_url())))?;
        let status = response.status();
        if !status.is_success() {
            // Box's error body names the reason (for example an unknown key id) and holds no secret.
            let reason = response.text().await.unwrap_or_default();
            return Err(StoreError::Unavailable(format!(
                "Box sign-in was refused (HTTP {}): {}",
                status.as_u16(),
                reason.chars().take(300).collect::<String>()
            )));
        }
        let body: TokenResponse = response
            .json()
            .await
            .map_err(|_| StoreError::Unavailable("Box sign-in returned an unreadable response".into()))?;
        let lifetime = body.expires_in.filter(|s| *s > 0).unwrap_or(FALLBACK_LIFETIME_SECS);
        Ok(MintedToken { access_token: body.access_token, expires_at: now + Duration::seconds(lifetime) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn config() -> BoxConfig {
        BoxConfig {
            client_id: "app-id".into(),
            client_secret: "secret".into(),
            enterprise_id: "7233266".into(),
            public_key_id: "cofqpfps".into(),
            keystore_path: PathBuf::from("missing.p12"),
            keystore_password: "pw".into(),
            token_url: "https://api.box.com/oauth2/token".into(),
            api_base: "https://api.box.com/2.0".into(),
            upload_base: "https://upload.box.com/api/2.0".into(),
        }
    }

    #[test]
    fn claims_name_the_app_the_enterprise_and_the_token_url() {
        let c = build_claims(&config(), 1_000, "jti-1".into());
        assert_eq!(c.iss, "app-id");
        assert_eq!(c.sub, "7233266");
        assert_eq!(c.box_sub_type, "enterprise");
        assert_eq!(c.aud, "https://api.box.com/oauth2/token");
        assert_eq!(c.jti, "jti-1");
        assert_eq!(c.exp, 1_045, "short-lived: at most 60 seconds");
    }

    #[test]
    fn a_missing_key_store_is_a_clear_error() {
        let err = load_signing_key(&config()).err().expect("no such file");
        assert!(err.contains("missing.p12"));
    }

    /// Opens the real key store and signs a real assertion without calling Box. Run it yourself
    /// (the password stays in your terminal):
    ///   set the BOX_* settings, then
    ///   cargo test box_keystore_signs -- --ignored --nocapture
    #[test]
    #[ignore = "needs the real key store and BOX_* settings"]
    fn box_keystore_signs_an_assertion() {
        let config = BoxConfig::from_env().expect("settings").expect("BOX_* settings are required");
        let key = load_signing_key(&config).expect("key store opens");
        let claims = build_claims(&config, 1_700_000_000, "live-check".into());
        let jwt = sign_assertion(&key, &config, &claims).expect("signs");
        assert_eq!(jwt.split('.').count(), 3);
        println!("key store opened and an assertion was signed (not sent)");
    }
}
