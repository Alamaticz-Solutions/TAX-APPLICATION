use std::{collections::HashSet, sync::Arc};

#[cfg(feature = "chat")]
use std::collections::BTreeSet;

#[cfg(feature = "chat")]
use axum::http::header;
use axum::http::HeaderMap;
use okta_jwt_verifier::Verifier;
#[cfg(feature = "chat")]
use serde::Deserialize;
use serde_json::{Map, Value};
use tracing::{debug, warn};

use crate::{
    extension::{Claims, UserAuth},
    security::{is_dev_workstation_env, SecurityConfig},
    RuntimeAuthState, RuntimeError,
};

#[cfg(feature = "chat")]
use crate::ix::{IxTransportError, IxTransportPolicy, IxVerifiedHuman, IX_CALLER_PROFILE_HEADER};

const LOCAL_AUTH_PREFIX: &str = "appfw-local:";

#[cfg(feature = "chat")]
#[derive(Deserialize)]
#[serde(untagged)]
enum IxAudienceClaim {
    One(String),
    Many(Vec<String>),
}

#[cfg(feature = "chat")]
impl IxAudienceClaim {
    fn singleton(self) -> Option<String> {
        match self {
            Self::One(value) if !value.is_empty() => Some(value),
            Self::Many(mut values) if values.len() == 1 && !values[0].is_empty() => values.pop(),
            _ => None,
        }
    }
}

#[cfg(feature = "chat")]
#[derive(Default, Deserialize)]
#[serde(untagged)]
enum IxScopeClaim {
    #[default]
    Missing,
    Text(String),
    Many(Vec<String>),
}

#[cfg(feature = "chat")]
impl IxScopeClaim {
    fn into_values(self) -> Option<Vec<String>> {
        match self {
            Self::Missing => Some(Vec::new()),
            Self::Text(value)
                if !value.is_empty()
                    && value.trim() == value
                    && !value.contains("  ")
                    && value
                        .bytes()
                        .all(|byte| !byte.is_ascii_whitespace() || byte == b' ') =>
            {
                Some(value.split(' ').map(ToString::to_string).collect())
            }
            Self::Text(_) => None,
            Self::Many(values) => Some(values),
        }
    }
}

#[cfg(feature = "chat")]
#[derive(Deserialize)]
struct IxStrictJwtClaims {
    iss: String,
    sub: String,
    exp: u64,
    #[serde(default)]
    nbf: Option<u64>,
    aud: IxAudienceClaim,
    #[serde(default)]
    cid: Option<String>,
    #[serde(default)]
    azp: Option<String>,
    #[serde(default)]
    tenant: Option<String>,
    #[serde(default)]
    company: Option<String>,
    roles: Vec<String>,
    #[serde(default)]
    scp: IxScopeClaim,
    #[serde(rename = "principalType")]
    principal_type: String,
    #[serde(rename = "releaseId")]
    release_id: String,
    #[serde(rename = "sessionId", alias = "session_id", alias = "sid")]
    session_id: String,
}

/// Token-free intermediate owned by the verifier. Its private fields can only
/// enter the sealed IX principal constructor.
#[cfg(feature = "chat")]
pub(crate) struct IxVerifiedClaims {
    tenant_id: String,
    subject: String,
    roles: BTreeSet<String>,
    scopes: BTreeSet<String>,
    client_id: String,
    audience: String,
    release_id: String,
    session_id: String,
}

#[cfg(feature = "chat")]
impl IxVerifiedClaims {
    pub(crate) fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    pub(crate) fn subject(&self) -> &str {
        &self.subject
    }

    pub(crate) fn roles(&self) -> &BTreeSet<String> {
        &self.roles
    }

    pub(crate) fn scopes(&self) -> &BTreeSet<String> {
        &self.scopes
    }

    pub(crate) fn client_id(&self) -> &str {
        &self.client_id
    }

    pub(crate) fn audience(&self) -> &str {
        &self.audience
    }

    pub(crate) fn release_id(&self) -> &str {
        &self.release_id
    }

    pub(crate) fn session_id(&self) -> &str {
        &self.session_id
    }
}

/// Cached production RS256 verifier for the IX-only transport. It never uses
/// the legacy local-auth fallback and never retains a bearer token.
#[cfg(feature = "chat")]
pub struct IxJwtVerifier {
    verifier: Verifier,
    policy: Arc<IxTransportPolicy>,
}

#[cfg(feature = "chat")]
impl IxJwtVerifier {
    pub async fn new(policy: Arc<IxTransportPolicy>) -> Result<Self, IxTransportError> {
        let verifier = Verifier::new(policy.issuer())
            .await
            .map_err(|_| IxTransportError::Unavailable)?
            .audience(policy.audiences())
            .leeway(0)
            .validate_nbf(true);
        Ok(Self { verifier, policy })
    }

    pub(crate) fn policy(&self) -> &Arc<IxTransportPolicy> {
        &self.policy
    }

    pub async fn verify(&self, headers: &HeaderMap) -> Result<IxVerifiedHuman, IxTransportError> {
        if headers.contains_key(header::COOKIE)
            || headers.contains_key(IX_CALLER_PROFILE_HEADER)
            || headers.get_all(header::AUTHORIZATION).iter().count() != 1
        {
            return Err(IxTransportError::Unauthorized);
        }
        let authorization = headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or(IxTransportError::Unauthorized)?;
        if authorization.trim() != authorization {
            return Err(IxTransportError::Unauthorized);
        }
        let token = authorization
            .strip_prefix("Bearer ")
            .filter(|value| {
                !value.is_empty()
                    && !value.chars().any(char::is_whitespace)
                    && value.matches('.').count() == 2
            })
            .ok_or(IxTransportError::Unauthorized)?;
        let token_data = self
            .verifier
            .verify::<IxStrictJwtClaims>(token)
            .await
            .map_err(|_| IxTransportError::Unauthorized)?;
        let claims = token_data.claims;
        if claims.iss != self.policy.issuer()
            || claims.exp == 0
            || claims.nbf == Some(0)
            || claims.principal_type != "human"
        {
            return Err(IxTransportError::Unauthorized);
        }
        let audience = claims
            .aud
            .singleton()
            .ok_or(IxTransportError::Unauthorized)?;
        let client_id = match (claims.cid, claims.azp) {
            (Some(cid), Some(azp)) if cid == azp && canonical_ix_claim(&cid, 256) => cid,
            (Some(cid), None) | (None, Some(cid)) if canonical_ix_claim(&cid, 256) => cid,
            _ => return Err(IxTransportError::Unauthorized),
        };
        let tenant_id = match (claims.tenant, claims.company) {
            (Some(tenant), Some(company))
                if tenant == company && canonical_ix_claim(&tenant, 256) =>
            {
                tenant
            }
            (Some(tenant), None) | (None, Some(tenant)) if canonical_ix_claim(&tenant, 256) => {
                tenant
            }
            _ => return Err(IxTransportError::Unauthorized),
        };
        let role_count = claims.roles.len();
        let roles = claims.roles.into_iter().collect::<BTreeSet<_>>();
        let scope_values = claims
            .scp
            .into_values()
            .ok_or(IxTransportError::Unauthorized)?;
        let scopes = scope_values.iter().cloned().collect::<BTreeSet<_>>();
        if roles.is_empty()
            || roles.len() > 64
            || scopes.len() > 128
            || roles.len() != role_count
            || scopes.len() != scope_values.len()
            || roles
                .iter()
                .chain(scopes.iter())
                .any(|value| !canonical_ix_claim(value, 128))
            || !canonical_ix_claim(&audience, 256)
            || !canonical_ix_claim(&claims.sub, 256)
            || !canonical_ix_claim(&claims.release_id, 128)
            || !canonical_ix_claim(&claims.session_id, 256)
        {
            return Err(IxTransportError::Unauthorized);
        }
        let profile = self.policy.authorize_claims(
            &client_id,
            &audience,
            &roles,
            &scopes,
            &claims.release_id,
        )?;
        IxVerifiedHuman::from_verified_claims(
            IxVerifiedClaims {
                tenant_id,
                subject: claims.sub,
                roles,
                scopes,
                client_id,
                audience,
                release_id: claims.release_id,
                session_id: claims.session_id,
            },
            profile,
        )
    }
}

#[cfg(feature = "chat")]
fn canonical_ix_claim(value: &str, maximum_bytes: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum_bytes
        && value.trim() == value
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'.' | b'_' | b':' | b'/' | b'@' | b'-' | b'+')
        })
}

#[derive(Debug, Clone)]
pub struct RuntimeJwtExtractor {
    pub user: Option<Arc<UserAuth>>,
}

impl RuntimeJwtExtractor {
    #[tracing::instrument(skip(auth_state, headers), fields(is_introspection = is_introspection))]
    pub async fn new(
        auth_state: RuntimeAuthState,
        headers: HeaderMap,
        is_introspection: bool,
    ) -> Result<Self, RuntimeError> {
        let jwt_token = headers
            .get("Authorization")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_string();

        let is_local_dev = is_dev_workstation_env();
        let allow_ci_local_test_auth = SecurityConfig::local_test_auth_enabled();
        debug!(
            is_local_dev,
            allow_ci_local_test_auth,
            has_authorization_header = !jwt_token.is_empty(),
            "extracting JWT"
        );

        if jwt_token.is_empty() && !is_local_dev {
            return Err(RuntimeError::NotAuthorized);
        }

        let validated_timezone = validated_timezone(&headers);

        let user = if is_local_dev {
            Some(Arc::new(if jwt_token.trim().is_empty() {
                local_admin_user(jwt_token.clone(), validated_timezone.clone())
            } else if let Some(local_user) =
                local_test_user_from_authorization(&jwt_token, validated_timezone.clone())?
            {
                local_user
            } else {
                local_admin_user(jwt_token.clone(), validated_timezone.clone())
            }))
        } else if allow_ci_local_test_auth {
            if let Some(local_user) =
                local_test_user_from_authorization(&jwt_token, validated_timezone.clone())?
            {
                warn!("using explicit local test auth token outside local environment");
                Some(Arc::new(local_user))
            } else {
                verify_bearer_token(&jwt_token, &auth_state, validated_timezone).await?
            }
        } else {
            verify_bearer_token(&jwt_token, &auth_state, validated_timezone).await?
        };

        Ok(Self { user })
    }
}

pub async fn verify_token(
    token: &str,
    auth_state: &RuntimeAuthState,
) -> Result<Claims, RuntimeError> {
    let mut aud = HashSet::new();
    aud.insert(auth_state.jwt_audience.clone());

    let verifier = Verifier::new(&auth_state.jwt_issuer)
        .await
        .map_err(|err| {
            warn!(error = ?err, "failed to create Okta JWT verifier");
            RuntimeError::NotAuthorized
        })?
        .client_id(&auth_state.okta_client_id)
        .audience(aud);

    let token_claims = verifier.verify::<Value>(token).await.map_err(|err| {
        warn!(error = ?err, "failed to verify Okta JWT");
        RuntimeError::NotAuthorized
    })?;

    let claims_obj = token_claims
        .claims
        .as_object()
        .ok_or(RuntimeError::NotAuthorized)?;

    Ok(Claims {
        iss: get_string(claims_obj, "iss"),
        aud: auth_state.jwt_audience.clone(),
        sub: get_string(claims_obj, "sub"),
        company: get_string(claims_obj, "company"),
        iat: get_number(claims_obj, "iat"),
        exp: get_number(claims_obj, "exp"),
        roles: extract_roles(claims_obj),
        clock_id: "default".to_owned(),
        cid: auth_state.okta_client_id.clone(),
        uid: get_string(claims_obj, "uid"),
        scopes: extract_scopes(claims_obj),
        auth_time: get_number(claims_obj, "auth_time"),
        ver: get_number(claims_obj, "ver"),
        jti: get_string(claims_obj, "jti"),
    })
}

async fn verify_bearer_token(
    authorization: &str,
    auth_state: &RuntimeAuthState,
    timezone: String,
) -> Result<Option<Arc<UserAuth>>, RuntimeError> {
    let token = authorization.trim_start_matches("Bearer ");
    if token.is_empty() {
        return Ok(None);
    }

    verify_token(token, auth_state).await.map(|claims| {
        Some(Arc::new(UserAuth::human(
            claims.company,
            claims.sub,
            timezone,
            claims.roles,
            claims.scopes,
            token.to_string(),
        )))
    })
}

fn validated_timezone(headers: &HeaderMap) -> String {
    headers
        .get("x-timezone")
        .or_else(|| headers.get("timezone"))
        .and_then(|h| h.to_str().ok())
        .and_then(|tz| tz.parse::<chrono_tz::Tz>().map(|tz| tz.to_string()).ok())
        .unwrap_or(chrono_tz::UTC.to_string())
}

fn local_admin_user(token: String, timezone: String) -> UserAuth {
    UserAuth::human(
        "local",
        "local-dev",
        timezone,
        vec!["admin".to_string()],
        vec!["appfw:mcp".to_string(), "appfw:mcp.admin".to_string()],
        token,
    )
}

fn local_test_user_from_authorization(
    authorization: &str,
    timezone: String,
) -> Result<Option<UserAuth>, RuntimeError> {
    let authorization = authorization.trim();
    let Some(token) = authorization.strip_prefix("Bearer ") else {
        return Ok(None);
    };
    let token = token.trim();
    let Some(payload) = token.strip_prefix(LOCAL_AUTH_PREFIX) else {
        return Ok(None);
    };

    let mut tenant_id = None;
    let mut user_name = None;
    let mut roles = None;
    let mut scopes = Vec::new();
    for part in payload.split(';').filter(|part| !part.trim().is_empty()) {
        let Some((key, value)) = part.split_once('=') else {
            return Err(RuntimeError::NotAuthorized);
        };
        match key.trim() {
            "tenant" => tenant_id = Some(value.trim().to_string()),
            "user" => user_name = Some(value.trim().to_string()),
            "roles" => {
                let parsed = value
                    .split(',')
                    .map(str::trim)
                    .filter(|role| !role.is_empty())
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();
                roles = Some(parsed);
            }
            "scopes" | "scp" => {
                scopes = value
                    .split(',')
                    .map(str::trim)
                    .filter(|scope| !scope.is_empty())
                    .map(ToString::to_string)
                    .collect();
            }
            _ => return Err(RuntimeError::NotAuthorized),
        }
    }

    let roles = roles.filter(|roles| !roles.is_empty());
    Ok(Some(UserAuth::human(
        tenant_id
            .filter(|value| !value.is_empty())
            .ok_or(RuntimeError::NotAuthorized)?,
        user_name
            .filter(|value| !value.is_empty())
            .ok_or(RuntimeError::NotAuthorized)?,
        timezone,
        roles.ok_or(RuntimeError::NotAuthorized)?,
        scopes,
        token.to_string(),
    )))
}

fn get_string(claims: &Map<String, Value>, key: &str) -> String {
    claims
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn get_number(claims: &Map<String, Value>, key: &str) -> usize {
    claims.get(key).and_then(|v| v.as_u64()).unwrap_or(0) as usize
}

fn extract_roles(claims: &Map<String, Value>) -> Vec<String> {
    claims
        .get("groups")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

fn extract_scopes(claims: &Map<String, Value>) -> Vec<String> {
    claims
        .get("scp")
        .and_then(|v| v.as_str())
        .map(|s| s.split(' ').map(String::from).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "chat")]
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test_fixtures/ix_rs256.rs"
    ));

    #[test]
    fn local_test_auth_header_builds_non_admin_user() {
        let user = local_test_user_from_authorization(
            "Bearer appfw-local:user=casey;tenant=tenant-1;roles=account_ca_reader,analyst",
            "UTC".to_string(),
        )
        .expect("local auth header should parse")
        .expect("local auth header should produce a user");

        assert_eq!(user.user_name, "casey");
        assert_eq!(user.tenant_id, "tenant-1");
        assert_eq!(user.roles, vec!["account_ca_reader", "analyst"]);
        assert_eq!(user.timezone, "UTC");
        assert_eq!(user.principal_type, crate::RuntimePrincipalType::User);
        assert_eq!(user.ingress.as_deref(), Some("http"));
        assert_eq!(user.on_behalf_of, None);
    }

    #[test]
    fn local_test_auth_header_builds_scoped_user() {
        let user = local_test_user_from_authorization(
            "Bearer appfw-local:user=casey;tenant=tenant-1;roles=analyst;scopes=appfw:mcp.read,appfw:mcp.admin",
            "UTC".to_string(),
        )
        .expect("local auth header should parse")
        .expect("local auth header should produce a user");

        assert_eq!(user.scopes, vec!["appfw:mcp.read", "appfw:mcp.admin"]);
    }

    #[test]
    fn local_test_auth_header_accepts_scp_alias() {
        let user = local_test_user_from_authorization(
            "Bearer appfw-local:user=casey;tenant=tenant-1;roles=analyst;scp=appfw:mcp.read",
            "UTC".to_string(),
        )
        .expect("local auth header should parse")
        .expect("local auth header should produce a user");

        assert_eq!(user.scopes, vec!["appfw:mcp.read"]);
    }

    #[test]
    fn local_test_auth_requires_bearer_scheme() {
        let user = local_test_user_from_authorization(
            "appfw-local:user=casey;tenant=tenant-1;roles=analyst",
            "UTC".to_string(),
        )
        .expect("non-bearer authorization should not be malformed");

        assert!(user.is_none());
    }

    #[test]
    fn local_test_auth_ignores_non_local_bearer_tokens() {
        let user =
            local_test_user_from_authorization("Bearer external.jwt.token", "UTC".to_string())
                .expect("external bearer token should pass through to normal verification");

        assert!(user.is_none());
    }

    #[test]
    fn local_test_auth_rejects_missing_roles() {
        assert!(local_test_user_from_authorization(
            "Bearer appfw-local:user=casey;tenant=tenant-1;roles=",
            "UTC".to_string(),
        )
        .is_err());
    }

    #[test]
    fn local_test_auth_rejects_missing_tenant() {
        assert!(local_test_user_from_authorization(
            "Bearer appfw-local:user=casey;roles=analyst",
            "UTC".to_string(),
        )
        .is_err());
    }

    #[test]
    fn local_test_auth_rejects_unknown_claim_keys() {
        assert!(local_test_user_from_authorization(
            "Bearer appfw-local:user=casey;tenant=tenant-1;roles=analyst;scope=admin",
            "UTC".to_string(),
        )
        .is_err());
    }

    #[test]
    fn local_test_auth_rejects_malformed_claim_segments() {
        assert!(local_test_user_from_authorization(
            "Bearer appfw-local:user=casey;tenant-1;roles=analyst",
            "UTC".to_string(),
        )
        .is_err());
    }

    #[test]
    fn timezone_header_is_validated() {
        let mut headers = HeaderMap::new();
        headers.insert("x-timezone", "America/Los_Angeles".parse().unwrap());

        assert_eq!(validated_timezone(&headers), "America/Los_Angeles");
    }

    #[test]
    fn invalid_timezone_defaults_to_utc() {
        let mut headers = HeaderMap::new();
        headers.insert("x-timezone", "bad-zone".parse().unwrap());

        assert_eq!(validated_timezone(&headers), "UTC");
    }

    #[cfg(feature = "chat")]
    async fn generated_ix_verifier() -> (
        IxJwtVerifier,
        jsonwebtoken::EncodingKey,
        String,
        serde_json::Value,
    ) {
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::TcpListener,
        };

        let encoding_key = ix_rs256_test_encoding_key();
        let jwks = ix_rs256_test_jwks("ix-runtime-generated").to_string();
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("loopback JWKS listener");
        let issuer = format!("http://{}", listener.local_addr().expect("JWKS address"));
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("JWKS request");
            let mut request = [0_u8; 4096];
            let _ = socket.read(&mut request).await.expect("read JWKS request");
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                jwks.len(),
                jwks
            );
            socket
                .write_all(response.as_bytes())
                .await
                .expect("write JWKS response");
        });
        let policy = Arc::new(
            IxTransportPolicy::new(
                issuer.clone(),
                "release-1",
                [crate::ix::IxClientBinding::new(
                    "nexus-browser",
                    "api://nexus-ix",
                    crate::ix::IxClientProfile::Browser,
                    ["https://nexus.example.com".to_string()],
                )
                .expect("browser binding")],
                ["ix.user".to_string()],
                ["ix.run".to_string()],
            )
            .expect("IX transport policy"),
        );
        let verifier = IxJwtVerifier::new(policy)
            .await
            .expect("generated-key verifier");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("current epoch")
            .as_secs();
        let claims = serde_json::json!({
            "iss": issuer,
            "sub": "human-1",
            "exp": now + 3600,
            "nbf": now.saturating_sub(1),
            "aud": "api://nexus-ix",
            "cid": "nexus-browser",
            "tenant": "tenant-1",
            "roles": ["ix.user"],
            "scp": "ix.run",
            "principalType": "human",
            "releaseId": "release-1",
            "sessionId": "session-1"
        });
        (verifier, encoding_key, issuer, claims)
    }

    #[cfg(feature = "chat")]
    fn sign_ix_claims(key: &jsonwebtoken::EncodingKey, claims: &serde_json::Value) -> String {
        let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
        header.kid = Some("ix-runtime-generated".to_string());
        jsonwebtoken::encode(&header, claims, key).expect("ephemeral JWT")
    }

    #[cfg(feature = "chat")]
    fn bearer_headers(token: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            format!("Bearer {token}").parse().expect("Bearer header"),
        );
        headers
    }

    #[cfg(feature = "chat")]
    #[tokio::test]
    async fn ix_production_verifier_matrix() {
        let (verifier, key, issuer, base) = generated_ix_verifier().await;

        for claims in [
            base.clone(),
            {
                let mut value = base.clone();
                value.as_object_mut().expect("claims").remove("cid");
                value["azp"] = serde_json::json!("nexus-browser");
                value
            },
            {
                let mut value = base.clone();
                value["azp"] = serde_json::json!("nexus-browser");
                value
            },
        ] {
            let token = sign_ix_claims(&key, &claims);
            let human = verifier
                .verify(&bearer_headers(&token))
                .await
                .expect("valid generated JWT");
            assert_eq!(human.principal().tenant_id, "tenant-1");
            assert_eq!(human.principal().subject, "human-1");
            assert!(!format!("{human:?}").contains(&token));
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("current epoch")
            .as_secs();
        let mut invalid = Vec::new();
        for (field, value) in [
            ("iss", serde_json::json!("https://wrong.example.com")),
            ("exp", serde_json::json!(now.saturating_sub(1))),
            ("nbf", serde_json::json!(now + 3600)),
            ("aud", serde_json::json!("api://wrong")),
            ("aud", serde_json::json!(["api://nexus-ix", "api://extra"])),
            ("cid", serde_json::json!("NEXUS-BROWSER")),
            ("sub", serde_json::json!("")),
            ("tenant", serde_json::json!(" tenant-1")),
            ("principalType", serde_json::json!("service")),
            ("roles", serde_json::json!([])),
            ("roles", serde_json::json!(["ix.user", "ix.user"])),
            ("scp", serde_json::json!(" ix.run")),
            ("scp", serde_json::json!("ix.run  extra")),
            ("releaseId", serde_json::json!("release-2")),
            ("sessionId", serde_json::json!("")),
        ] {
            let mut claims = base.clone();
            claims[field] = value;
            invalid.push(claims);
        }
        for field in [
            "aud",
            "cid",
            "sub",
            "tenant",
            "roles",
            "principalType",
            "releaseId",
            "sessionId",
        ] {
            let mut claims = base.clone();
            claims.as_object_mut().expect("claims").remove(field);
            invalid.push(claims);
        }
        let mut mismatched_client = base.clone();
        mismatched_client["azp"] = serde_json::json!("different-client");
        invalid.push(mismatched_client);
        let mut mismatched_tenant = base.clone();
        mismatched_tenant["company"] = serde_json::json!("tenant-2");
        invalid.push(mismatched_tenant);
        let mut wrong_type = base.clone();
        wrong_type["cid"] = serde_json::json!(["nexus-browser"]);
        invalid.push(wrong_type);

        for claims in invalid {
            let token = sign_ix_claims(&key, &claims);
            assert!(
                matches!(
                    verifier.verify(&bearer_headers(&token)).await,
                    Err(IxTransportError::Unauthorized | IxTransportError::Forbidden)
                ),
                "invalid generated claim set was accepted: {claims}"
            );
        }

        let token = sign_ix_claims(&key, &base);
        let mut duplicate = bearer_headers(&token);
        duplicate.append(
            header::AUTHORIZATION,
            format!("Bearer {token}").parse().expect("duplicate Bearer"),
        );
        assert_eq!(
            verifier.verify(&duplicate).await.unwrap_err(),
            IxTransportError::Unauthorized
        );

        for (name, value) in [
            (header::COOKIE, "session=forbidden"),
            (
                header::HeaderName::from_static(IX_CALLER_PROFILE_HEADER),
                "ix-browser@1",
            ),
        ] {
            let mut headers = bearer_headers(&token);
            headers.insert(name, value.parse().expect("header"));
            assert_eq!(
                verifier.verify(&headers).await.unwrap_err(),
                IxTransportError::Unauthorized
            );
        }

        let mut tampered = token.clone();
        tampered.push('x');
        assert_eq!(
            verifier
                .verify(&bearer_headers(&tampered))
                .await
                .unwrap_err(),
            IxTransportError::Unauthorized
        );
        assert_eq!(
            verifier
                .verify(&bearer_headers(
                    "appfw-local:user=casey;tenant=tenant-1;roles=ix.user",
                ))
                .await
                .unwrap_err(),
            IxTransportError::Unauthorized
        );
        assert_eq!(base["iss"], issuer);
    }
}
