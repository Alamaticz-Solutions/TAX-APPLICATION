use std::{
    collections::BTreeMap,
    fmt,
    sync::Mutex,
    time::{Duration, SystemTime},
};

use appfw_saas_core::{redact_json_value, SecretString, TokenRefreshDecision, REDACTED};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

use crate::{provider_keys::FrameworkProvider, UserAuth};

pub const GOVERNED_WRITE_EVIDENCE_FILE: &str = "governed-write-evidence.json";
pub const GOVERNED_WRITE_MOCK_FIXTURE_FILE: &str = "governed-write-mock-fixture.json";

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum RuntimeDelegatedAuthError {
    #[error("delegated auth provider must be an external API provider")]
    NonExternalApiProvider,
    #[error("delegated auth field is invalid: {0}")]
    InvalidField(String),
    #[error("delegated token is revoked")]
    TokenRevoked,
    #[error("delegated token is expired")]
    TokenExpired,
    #[error("delegated write replay conflict for idempotency key")]
    ReplayConflict,
    #[error("mock certification output cannot use release evidence artifact name")]
    MockEvidenceArtifactName,
    #[error("live governed-write certification requires managed provider evidence")]
    LiveEvidenceRequired,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct RuntimeDelegatedTokenKey {
    pub provider_key: String,
    pub data_source_name: String,
    pub tenant_id: String,
    pub user_subject: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<String>,
}

impl RuntimeDelegatedTokenKey {
    pub fn new(
        provider: FrameworkProvider,
        data_source_name: impl Into<String>,
        user: &UserAuth,
        scopes: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, RuntimeDelegatedAuthError> {
        if !provider.is_external_api_provider() {
            return Err(RuntimeDelegatedAuthError::NonExternalApiProvider);
        }

        let mut key = Self {
            provider_key: provider.key().to_string(),
            data_source_name: data_source_name.into(),
            tenant_id: user.tenant_id.clone(),
            user_subject: user.user_name.clone(),
            scopes: normalize_string_set(scopes),
        };
        key.validate()?;
        Ok(key)
    }

    pub fn validate(&mut self) -> Result<(), RuntimeDelegatedAuthError> {
        validate_token("provider_key", &self.provider_key)?;
        validate_token("data_source_name", &self.data_source_name)?;
        validate_token("tenant_id", &self.tenant_id)?;
        validate_token("user_subject", &self.user_subject)?;
        self.scopes = normalize_string_set(self.scopes.clone());
        for scope in &self.scopes {
            validate_scope(scope)?;
        }
        let provider = FrameworkProvider::parse_key(&self.provider_key)
            .map_err(RuntimeDelegatedAuthError::InvalidField)?;
        if !provider.is_external_api_provider() {
            return Err(RuntimeDelegatedAuthError::NonExternalApiProvider);
        }
        Ok(())
    }

    pub fn redacted_summary(&self) -> Value {
        json!({
            "provider": self.provider_key,
            "data_source": self.data_source_name,
            "tenant_id": self.tenant_id,
            "user_subject": self.user_subject,
            "scopes": self.scopes,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeDelegatedAuthCodeRequest {
    pub provider_key: String,
    pub data_source_name: String,
    pub tenant_id: String,
    pub user_subject: String,
    pub authorization_url: String,
    pub redirect_uri: String,
    pub state: SecretString,
    pub pkce_challenge: SecretString,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<String>,
}

impl RuntimeDelegatedAuthCodeRequest {
    pub fn new(
        key: &RuntimeDelegatedTokenKey,
        authorization_url: impl Into<String>,
        redirect_uri: impl Into<String>,
        state: impl Into<String>,
        pkce_challenge: impl Into<String>,
    ) -> Result<Self, RuntimeDelegatedAuthError> {
        let request = Self {
            provider_key: key.provider_key.clone(),
            data_source_name: key.data_source_name.clone(),
            tenant_id: key.tenant_id.clone(),
            user_subject: key.user_subject.clone(),
            authorization_url: authorization_url.into(),
            redirect_uri: redirect_uri.into(),
            state: SecretString::new(state),
            pkce_challenge: SecretString::new(pkce_challenge),
            scopes: key.scopes.clone(),
        };
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> Result<(), RuntimeDelegatedAuthError> {
        validate_https_url("authorization_url", &self.authorization_url)?;
        validate_https_url("redirect_uri", &self.redirect_uri)?;
        if self.state.is_empty() {
            return Err(RuntimeDelegatedAuthError::InvalidField(
                "state must not be empty".to_string(),
            ));
        }
        if self.pkce_challenge.is_empty() {
            return Err(RuntimeDelegatedAuthError::InvalidField(
                "pkce_challenge must not be empty".to_string(),
            ));
        }
        Ok(())
    }

    pub fn redacted_summary(&self) -> Value {
        json!({
            "provider": self.provider_key,
            "data_source": self.data_source_name,
            "tenant_id": self.tenant_id,
            "user_subject": self.user_subject,
            "authorization_url": self.authorization_url,
            "redirect_uri": self.redirect_uri,
            "state": REDACTED,
            "pkce_challenge": REDACTED,
            "scopes": self.scopes,
        })
    }
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeDelegatedOAuthTokenSet {
    pub access_token: SecretString,
    pub refresh_token: Option<SecretString>,
    pub token_type: String,
    pub expires_at: SystemTime,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<SystemTime>,
}

impl RuntimeDelegatedOAuthTokenSet {
    pub fn bearer(
        access_token: impl Into<String>,
        refresh_token: Option<impl Into<String>>,
        expires_at: SystemTime,
        scopes: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            access_token: SecretString::new(access_token),
            refresh_token: refresh_token.map(SecretString::new),
            token_type: "Bearer".to_string(),
            expires_at,
            scopes: normalize_string_set(scopes),
            revoked_at: None,
        }
    }

    pub fn refresh_decision(
        &self,
        now: SystemTime,
        refresh_skew: Duration,
    ) -> TokenRefreshDecision {
        TokenRefreshDecision::from_expiration(self.expires_at, now, refresh_skew)
    }

    pub fn is_fresh(&self, now: SystemTime, refresh_skew: Duration) -> bool {
        self.revoked_at.is_none()
            && matches!(
                self.refresh_decision(now, refresh_skew),
                TokenRefreshDecision::Fresh
            )
    }

    pub fn revoked(mut self, revoked_at: SystemTime) -> Self {
        self.revoked_at = Some(revoked_at);
        self
    }
}

impl fmt::Debug for RuntimeDelegatedOAuthTokenSet {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RuntimeDelegatedOAuthTokenSet")
            .field("access_token", &REDACTED)
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| REDACTED),
            )
            .field("token_type", &self.token_type)
            .field("expires_at", &self.expires_at)
            .field("scopes", &self.scopes)
            .field("revoked_at", &self.revoked_at)
            .finish()
    }
}

pub trait RuntimeDelegatedTokenStore: Send + Sync {
    fn put_delegated_token(
        &self,
        key: RuntimeDelegatedTokenKey,
        token: RuntimeDelegatedOAuthTokenSet,
    ) -> Result<Option<RuntimeDelegatedOAuthTokenSet>, RuntimeDelegatedAuthError>;

    fn get_delegated_token(
        &self,
        key: &RuntimeDelegatedTokenKey,
        now: SystemTime,
        refresh_skew: Duration,
    ) -> Result<Option<RuntimeDelegatedOAuthTokenSet>, RuntimeDelegatedAuthError>;

    fn revoke_delegated_token(
        &self,
        key: &RuntimeDelegatedTokenKey,
        revoked_at: SystemTime,
    ) -> Result<bool, RuntimeDelegatedAuthError>;
}

#[derive(Debug, Default)]
pub struct InMemoryRuntimeDelegatedTokenStore {
    entries: Mutex<BTreeMap<RuntimeDelegatedTokenKey, RuntimeDelegatedOAuthTokenSet>>,
}

impl InMemoryRuntimeDelegatedTokenStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries
            .lock()
            .expect("delegated token store mutex poisoned")
            .len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries
            .lock()
            .expect("delegated token store mutex poisoned")
            .is_empty()
    }
}

impl RuntimeDelegatedTokenStore for InMemoryRuntimeDelegatedTokenStore {
    fn put_delegated_token(
        &self,
        mut key: RuntimeDelegatedTokenKey,
        token: RuntimeDelegatedOAuthTokenSet,
    ) -> Result<Option<RuntimeDelegatedOAuthTokenSet>, RuntimeDelegatedAuthError> {
        key.validate()?;
        if token.revoked_at.is_some() {
            return Err(RuntimeDelegatedAuthError::TokenRevoked);
        }
        Ok(self
            .entries
            .lock()
            .expect("delegated token store mutex poisoned")
            .insert(key, token))
    }

    fn get_delegated_token(
        &self,
        key: &RuntimeDelegatedTokenKey,
        now: SystemTime,
        refresh_skew: Duration,
    ) -> Result<Option<RuntimeDelegatedOAuthTokenSet>, RuntimeDelegatedAuthError> {
        let Some(token) = self
            .entries
            .lock()
            .expect("delegated token store mutex poisoned")
            .get(key)
            .cloned()
        else {
            return Ok(None);
        };
        if token.revoked_at.is_some() {
            return Err(RuntimeDelegatedAuthError::TokenRevoked);
        }
        if !token.is_fresh(now, refresh_skew) {
            return Err(RuntimeDelegatedAuthError::TokenExpired);
        }
        Ok(Some(token))
    }

    fn revoke_delegated_token(
        &self,
        key: &RuntimeDelegatedTokenKey,
        revoked_at: SystemTime,
    ) -> Result<bool, RuntimeDelegatedAuthError> {
        let mut entries = self
            .entries
            .lock()
            .expect("delegated token store mutex poisoned");
        let Some(token) = entries.get_mut(key) else {
            return Ok(false);
        };
        token.revoked_at = Some(revoked_at);
        Ok(true)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct RuntimeSaasWriteIdempotencyKey {
    pub provider_key: String,
    pub data_source_name: String,
    pub tenant_id: String,
    pub user_subject: String,
    pub operation_name: String,
    pub idempotency_key: String,
}

impl RuntimeSaasWriteIdempotencyKey {
    pub fn new(
        token_key: &RuntimeDelegatedTokenKey,
        operation_name: impl Into<String>,
        idempotency_key: impl Into<String>,
    ) -> Result<Self, RuntimeDelegatedAuthError> {
        let key = Self {
            provider_key: token_key.provider_key.clone(),
            data_source_name: token_key.data_source_name.clone(),
            tenant_id: token_key.tenant_id.clone(),
            user_subject: token_key.user_subject.clone(),
            operation_name: operation_name.into(),
            idempotency_key: idempotency_key.into(),
        };
        key.validate()?;
        Ok(key)
    }

    pub fn validate(&self) -> Result<(), RuntimeDelegatedAuthError> {
        validate_token("operation_name", &self.operation_name)?;
        validate_token("idempotency_key", &self.idempotency_key)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeSaasWriteReplayState {
    Reserved,
    Succeeded,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeSaasWriteReplayRecord {
    pub request_hash: String,
    pub state: RuntimeSaasWriteReplayState,
    pub first_seen_at: SystemTime,
    pub last_seen_at: SystemTime,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome_hash: Option<String>,
}

impl RuntimeSaasWriteReplayRecord {
    pub fn redacted_summary(&self) -> Value {
        json!({
            "state": self.state,
            "request_hash": self.request_hash,
            "outcome_hash": self.outcome_hash,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeSaasWriteReplayDecision {
    FirstAttempt(RuntimeSaasWriteReplayRecord),
    Replay(RuntimeSaasWriteReplayRecord),
    Conflict(RuntimeSaasWriteReplayRecord),
}

pub trait RuntimeSaasWriteIdempotencyStore: Send + Sync {
    fn reserve_saas_write(
        &self,
        key: RuntimeSaasWriteIdempotencyKey,
        request_hash: impl Into<String>,
        now: SystemTime,
    ) -> Result<RuntimeSaasWriteReplayDecision, RuntimeDelegatedAuthError>;

    fn mark_saas_write_succeeded(
        &self,
        key: &RuntimeSaasWriteIdempotencyKey,
        outcome_hash: impl Into<String>,
        now: SystemTime,
    ) -> Result<Option<RuntimeSaasWriteReplayRecord>, RuntimeDelegatedAuthError>;
}

#[derive(Debug, Default)]
pub struct InMemoryRuntimeSaasWriteIdempotencyStore {
    entries: Mutex<BTreeMap<RuntimeSaasWriteIdempotencyKey, RuntimeSaasWriteReplayRecord>>,
}

impl InMemoryRuntimeSaasWriteIdempotencyStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl RuntimeSaasWriteIdempotencyStore for InMemoryRuntimeSaasWriteIdempotencyStore {
    fn reserve_saas_write(
        &self,
        key: RuntimeSaasWriteIdempotencyKey,
        request_hash: impl Into<String>,
        now: SystemTime,
    ) -> Result<RuntimeSaasWriteReplayDecision, RuntimeDelegatedAuthError> {
        let request_hash = request_hash.into();
        validate_token("request_hash", &request_hash)?;
        let mut entries = self
            .entries
            .lock()
            .expect("SaaS write idempotency store mutex poisoned");
        if let Some(existing) = entries.get_mut(&key) {
            existing.last_seen_at = now;
            if existing.request_hash == request_hash {
                return Ok(RuntimeSaasWriteReplayDecision::Replay(existing.clone()));
            }
            return Ok(RuntimeSaasWriteReplayDecision::Conflict(existing.clone()));
        }

        let record = RuntimeSaasWriteReplayRecord {
            request_hash,
            state: RuntimeSaasWriteReplayState::Reserved,
            first_seen_at: now,
            last_seen_at: now,
            outcome_hash: None,
        };
        entries.insert(key, record.clone());
        Ok(RuntimeSaasWriteReplayDecision::FirstAttempt(record))
    }

    fn mark_saas_write_succeeded(
        &self,
        key: &RuntimeSaasWriteIdempotencyKey,
        outcome_hash: impl Into<String>,
        now: SystemTime,
    ) -> Result<Option<RuntimeSaasWriteReplayRecord>, RuntimeDelegatedAuthError> {
        let outcome_hash = outcome_hash.into();
        validate_token("outcome_hash", &outcome_hash)?;
        let mut entries = self
            .entries
            .lock()
            .expect("SaaS write idempotency store mutex poisoned");
        let Some(existing) = entries.get_mut(key) else {
            return Ok(None);
        };
        existing.state = RuntimeSaasWriteReplayState::Succeeded;
        existing.outcome_hash = Some(outcome_hash);
        existing.last_seen_at = now;
        Ok(Some(existing.clone()))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RuntimeSaasWriteAuditRecord {
    pub provider_key: String,
    pub data_source_name: String,
    pub tenant_id: String,
    pub actor_user: String,
    pub operation_name: String,
    pub request_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    pub delegated_auth: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_behalf_of: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ingress: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<Value>,
    #[serde(default)]
    pub metadata: Value,
}

impl RuntimeSaasWriteAuditRecord {
    pub fn new(
        token_key: &RuntimeDelegatedTokenKey,
        operation_name: impl Into<String>,
        request_id: impl Into<String>,
    ) -> Result<Self, RuntimeDelegatedAuthError> {
        let record = Self {
            provider_key: token_key.provider_key.clone(),
            data_source_name: token_key.data_source_name.clone(),
            tenant_id: token_key.tenant_id.clone(),
            actor_user: token_key.user_subject.clone(),
            operation_name: operation_name.into(),
            request_id: request_id.into(),
            idempotency_key: None,
            delegated_auth: true,
            on_behalf_of: None,
            ingress: None,
            before: None,
            after: None,
            metadata: Value::Null,
        };
        record.validate()?;
        Ok(record)
    }

    pub fn with_idempotency_key(mut self, idempotency_key: impl Into<String>) -> Self {
        self.idempotency_key = Some(idempotency_key.into());
        self
    }

    pub fn with_ingress(mut self, ingress: impl Into<String>) -> Self {
        self.ingress = Some(ingress.into());
        self
    }

    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_before_after(mut self, before: Option<Value>, after: Option<Value>) -> Self {
        self.before = before;
        self.after = after;
        self
    }

    pub fn validate(&self) -> Result<(), RuntimeDelegatedAuthError> {
        validate_token("operation_name", &self.operation_name)?;
        validate_token("request_id", &self.request_id)?;
        Ok(())
    }

    pub fn redacted_summary(&self) -> Value {
        json!({
            "provider": self.provider_key,
            "data_source": self.data_source_name,
            "tenant_id": self.tenant_id,
            "actor_user": self.actor_user,
            "operation_name": self.operation_name,
            "request_id": self.request_id,
            "idempotency_key": self.idempotency_key.as_ref().map(|_| REDACTED),
            "delegated_auth": self.delegated_auth,
            "on_behalf_of": self.on_behalf_of,
            "ingress": self.ingress,
            "before": self.before.clone().map(redact_json_value),
            "after": self.after.clone().map(redact_json_value),
            "metadata": redact_json_value(self.metadata.clone()),
        })
    }
}

pub trait RuntimeSaasWriteAuditSink: Send + Sync {
    fn append_saas_write_audit(
        &self,
        record: RuntimeSaasWriteAuditRecord,
    ) -> Result<(), RuntimeDelegatedAuthError>;
}

#[derive(Debug, Default)]
pub struct InMemoryRuntimeSaasWriteAuditSink {
    records: Mutex<Vec<RuntimeSaasWriteAuditRecord>>,
}

impl InMemoryRuntimeSaasWriteAuditSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn records(&self) -> Vec<RuntimeSaasWriteAuditRecord> {
        self.records
            .lock()
            .expect("SaaS write audit sink mutex poisoned")
            .clone()
    }
}

impl RuntimeSaasWriteAuditSink for InMemoryRuntimeSaasWriteAuditSink {
    fn append_saas_write_audit(
        &self,
        record: RuntimeSaasWriteAuditRecord,
    ) -> Result<(), RuntimeDelegatedAuthError> {
        record.validate()?;
        self.records
            .lock()
            .expect("SaaS write audit sink mutex poisoned")
            .push(record);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeGovernedWriteCertificationMode {
    Plan,
    Mock,
    Live,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeGovernedWriteCertificationRequest {
    pub provider_key: String,
    pub data_source_name: String,
    pub mutation_name: String,
    pub mode: RuntimeGovernedWriteCertificationMode,
    pub output_artifact_name: String,
}

impl RuntimeGovernedWriteCertificationRequest {
    pub fn new(
        provider: FrameworkProvider,
        data_source_name: impl Into<String>,
        mutation_name: impl Into<String>,
        mode: RuntimeGovernedWriteCertificationMode,
    ) -> Result<Self, RuntimeDelegatedAuthError> {
        if !provider.is_external_api_provider() {
            return Err(RuntimeDelegatedAuthError::NonExternalApiProvider);
        }
        let output_artifact_name = match mode {
            RuntimeGovernedWriteCertificationMode::Plan
            | RuntimeGovernedWriteCertificationMode::Mock => GOVERNED_WRITE_MOCK_FIXTURE_FILE,
            RuntimeGovernedWriteCertificationMode::Live => GOVERNED_WRITE_EVIDENCE_FILE,
        };
        let request = Self {
            provider_key: provider.key().to_string(),
            data_source_name: data_source_name.into(),
            mutation_name: mutation_name.into(),
            mode,
            output_artifact_name: output_artifact_name.to_string(),
        };
        request.validate()?;
        Ok(request)
    }

    pub fn with_output_artifact_name(mut self, artifact_name: impl Into<String>) -> Self {
        self.output_artifact_name = artifact_name.into();
        self
    }

    pub fn validate(&self) -> Result<(), RuntimeDelegatedAuthError> {
        validate_token("provider_key", &self.provider_key)?;
        validate_token("data_source_name", &self.data_source_name)?;
        validate_token("mutation_name", &self.mutation_name)?;
        validate_token("output_artifact_name", &self.output_artifact_name)?;
        let provider = FrameworkProvider::parse_key(&self.provider_key)
            .map_err(RuntimeDelegatedAuthError::InvalidField)?;
        if !provider.is_external_api_provider() {
            return Err(RuntimeDelegatedAuthError::NonExternalApiProvider);
        }
        if matches!(
            self.mode,
            RuntimeGovernedWriteCertificationMode::Plan
                | RuntimeGovernedWriteCertificationMode::Mock
        ) && self.output_artifact_name == GOVERNED_WRITE_EVIDENCE_FILE
        {
            return Err(RuntimeDelegatedAuthError::MockEvidenceArtifactName);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeGovernedWriteCertificationReport {
    pub provider_key: String,
    pub data_source_name: String,
    pub mutation_name: String,
    pub mode: RuntimeGovernedWriteCertificationMode,
    pub output_artifact_name: String,
    pub ok: bool,
    pub release_ready: bool,
    pub accepted_as_live_evidence: bool,
    #[serde(default)]
    pub checks: BTreeMap<String, bool>,
}

impl RuntimeGovernedWriteCertificationReport {
    pub fn redacted_summary(&self) -> Value {
        json!({
            "provider": self.provider_key,
            "data_source": self.data_source_name,
            "mutation_name": self.mutation_name,
            "mode": self.mode,
            "output_artifact_name": self.output_artifact_name,
            "ok": self.ok,
            "release_ready": self.release_ready,
            "accepted_as_live_evidence": self.accepted_as_live_evidence,
            "checks": self.checks,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct RuntimeGovernedWriteCertificationRunner;

impl RuntimeGovernedWriteCertificationRunner {
    pub fn run(
        &self,
        request: RuntimeGovernedWriteCertificationRequest,
    ) -> Result<RuntimeGovernedWriteCertificationReport, RuntimeDelegatedAuthError> {
        request.validate()?;
        match request.mode {
            RuntimeGovernedWriteCertificationMode::Live => {
                Err(RuntimeDelegatedAuthError::LiveEvidenceRequired)
            }
            RuntimeGovernedWriteCertificationMode::Plan
            | RuntimeGovernedWriteCertificationMode::Mock => Ok(mock_or_plan_report(request)),
        }
    }
}

fn mock_or_plan_report(
    request: RuntimeGovernedWriteCertificationRequest,
) -> RuntimeGovernedWriteCertificationReport {
    RuntimeGovernedWriteCertificationReport {
        provider_key: request.provider_key,
        data_source_name: request.data_source_name,
        mutation_name: request.mutation_name,
        mode: request.mode,
        output_artifact_name: request.output_artifact_name,
        ok: true,
        release_ready: false,
        accepted_as_live_evidence: false,
        checks: BTreeMap::from([
            ("delegated_token_store_contract".to_string(), true),
            ("idempotency_replay_guard_contract".to_string(), true),
            ("saas_write_audit_contract".to_string(), true),
            ("live_provider_evidence".to_string(), false),
        ]),
    }
}

fn validate_token(name: &str, value: &str) -> Result<(), RuntimeDelegatedAuthError> {
    if value.trim().is_empty() || value.chars().any(char::is_whitespace) {
        return Err(RuntimeDelegatedAuthError::InvalidField(format!(
            "{name} must be a non-empty token"
        )));
    }
    Ok(())
}

fn validate_scope(value: &str) -> Result<(), RuntimeDelegatedAuthError> {
    if value.trim().is_empty() || value.chars().any(char::is_whitespace) {
        return Err(RuntimeDelegatedAuthError::InvalidField(
            "scope values must be non-empty tokens".to_string(),
        ));
    }
    Ok(())
}

fn validate_https_url(name: &str, value: &str) -> Result<(), RuntimeDelegatedAuthError> {
    if !value.starts_with("https://") || value.contains(char::is_whitespace) {
        return Err(RuntimeDelegatedAuthError::InvalidField(format!(
            "{name} must be an https URL"
        )));
    }
    Ok(())
}

fn normalize_string_set<I, S>(values: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut values = values.into_iter().map(Into::into).collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

#[cfg(test)]
mod tests {
    use super::*;
    use appfw_saas_core::redact_text;

    fn time(seconds: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)
    }

    fn user(name: &str, tenant: &str) -> UserAuth {
        UserAuth::human(
            tenant,
            name,
            "UTC",
            vec!["writer".to_string()],
            vec!["servicenow.write".to_string()],
            "user-jwt-should-not-leak",
        )
    }

    fn token_key(name: &str, tenant: &str) -> RuntimeDelegatedTokenKey {
        RuntimeDelegatedTokenKey::new(
            FrameworkProvider::ServiceNow,
            "servicenow_primary",
            &user(name, tenant),
            ["incident.write", "incident.read", "incident.write"],
        )
        .expect("delegated token key")
    }

    #[test]
    fn delegated_token_key_partitions_by_provider_tenant_user_and_scope() {
        let key_a = token_key("casey", "tenant-1");
        let key_b = token_key("morgan", "tenant-1");
        let key_c = token_key("casey", "tenant-2");

        assert_ne!(key_a, key_b);
        assert_ne!(key_a, key_c);
        assert_eq!(key_a.scopes, vec!["incident.read", "incident.write"]);
        assert_eq!(key_a.redacted_summary()["provider"], "servicenow");
    }

    #[test]
    fn delegated_token_store_returns_fresh_tokens_and_rejects_revoked_or_expired() {
        let store = InMemoryRuntimeDelegatedTokenStore::new();
        let key = token_key("casey", "tenant-1");
        let token = RuntimeDelegatedOAuthTokenSet::bearer(
            "access-secret",
            Some("refresh-secret"),
            time(1_600),
            ["incident.write"],
        );

        store
            .put_delegated_token(key.clone(), token)
            .expect("insert token");
        assert!(store
            .get_delegated_token(&key, time(1_000), Duration::from_secs(300))
            .expect("fresh token")
            .is_some());
        assert_eq!(
            store
                .get_delegated_token(&key, time(1_400), Duration::from_secs(300))
                .unwrap_err(),
            RuntimeDelegatedAuthError::TokenExpired
        );

        store
            .revoke_delegated_token(&key, time(1_100))
            .expect("revoke token");
        assert_eq!(
            store
                .get_delegated_token(&key, time(1_100), Duration::from_secs(300))
                .unwrap_err(),
            RuntimeDelegatedAuthError::TokenRevoked
        );
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn delegated_auth_code_request_redacts_state_and_pkce() {
        let key = token_key("casey", "tenant-1");
        let request = RuntimeDelegatedAuthCodeRequest::new(
            &key,
            "https://idp.example.test/oauth2/v1/authorize",
            "https://app.example.test/callback",
            "state-secret",
            "pkce-secret",
        )
        .expect("auth request");

        let serialized = serde_json::to_string(&request).expect("request serializes");
        let summary = request.redacted_summary().to_string();

        assert!(serialized.contains(REDACTED));
        assert!(!serialized.contains("state-secret"));
        assert!(!serialized.contains("pkce-secret"));
        assert!(!summary.contains("state-secret"));
        assert!(!summary.contains("pkce-secret"));
    }

    #[test]
    fn idempotency_store_accepts_exact_replays_and_flags_conflicts() {
        let store = InMemoryRuntimeSaasWriteIdempotencyStore::new();
        let token_key = token_key("casey", "tenant-1");
        let key = RuntimeSaasWriteIdempotencyKey::new(&token_key, "incident.update", "request-123")
            .expect("idempotency key");

        let first = store
            .reserve_saas_write(key.clone(), "hash-a", time(1_000))
            .expect("reserve first");
        assert!(matches!(
            first,
            RuntimeSaasWriteReplayDecision::FirstAttempt(_)
        ));

        let replay = store
            .reserve_saas_write(key.clone(), "hash-a", time(1_001))
            .expect("reserve replay");
        assert!(matches!(replay, RuntimeSaasWriteReplayDecision::Replay(_)));

        let conflict = store
            .reserve_saas_write(key.clone(), "hash-b", time(1_002))
            .expect("reserve conflict");
        assert!(matches!(
            conflict,
            RuntimeSaasWriteReplayDecision::Conflict(_)
        ));

        let succeeded = store
            .mark_saas_write_succeeded(&key, "outcome-a", time(1_003))
            .expect("mark success")
            .expect("existing record");
        assert_eq!(succeeded.state, RuntimeSaasWriteReplayState::Succeeded);
        assert_eq!(succeeded.outcome_hash.as_deref(), Some("outcome-a"));
    }

    #[test]
    fn saas_write_audit_sink_redacts_sensitive_payloads() {
        let sink = InMemoryRuntimeSaasWriteAuditSink::new();
        let key = token_key("casey", "tenant-1");
        let record = RuntimeSaasWriteAuditRecord::new(&key, "incident.update", "request-123")
            .expect("audit record")
            .with_idempotency_key("idem-secret")
            .with_ingress("graphql")
            .with_before_after(
                Some(json!({ "status": "open", "access_token": "old-secret" })),
                Some(json!({ "status": "closed", "authorization": "Bearer new-secret" })),
            )
            .with_metadata(json!({
                "request": "Authorization: Bearer metadata-secret",
                "client_secret": "metadata-client-secret"
            }));

        let summary = record.redacted_summary().to_string();
        assert!(!summary.contains("old-secret"));
        assert!(!summary.contains("new-secret"));
        assert!(!summary.contains("metadata-secret"));
        assert!(!summary.contains("metadata-client-secret"));
        assert!(summary.contains(REDACTED));

        sink.append_saas_write_audit(record)
            .expect("append audit record");
        assert_eq!(sink.records().len(), 1);
    }

    #[test]
    fn governed_write_mock_runner_never_emits_release_evidence() {
        let runner = RuntimeGovernedWriteCertificationRunner;
        let request = RuntimeGovernedWriteCertificationRequest::new(
            FrameworkProvider::ServiceNow,
            "servicenow_primary",
            "incident.update",
            RuntimeGovernedWriteCertificationMode::Mock,
        )
        .expect("mock request");

        let report = runner.run(request).expect("mock report");
        assert!(report.ok);
        assert!(!report.release_ready);
        assert!(!report.accepted_as_live_evidence);
        assert_eq!(
            report.output_artifact_name,
            GOVERNED_WRITE_MOCK_FIXTURE_FILE
        );
        assert_ne!(report.output_artifact_name, GOVERNED_WRITE_EVIDENCE_FILE);
        assert!(!report.checks["live_provider_evidence"]);

        let rejected = RuntimeGovernedWriteCertificationRequest::new(
            FrameworkProvider::ServiceNow,
            "servicenow_primary",
            "incident.update",
            RuntimeGovernedWriteCertificationMode::Mock,
        )
        .expect("mock request")
        .with_output_artifact_name(GOVERNED_WRITE_EVIDENCE_FILE);
        assert_eq!(
            runner.run(rejected).unwrap_err(),
            RuntimeDelegatedAuthError::MockEvidenceArtifactName
        );
    }

    #[test]
    fn governed_write_live_runner_requires_managed_provider_evidence() {
        let runner = RuntimeGovernedWriteCertificationRunner;
        let request = RuntimeGovernedWriteCertificationRequest::new(
            FrameworkProvider::ServiceNow,
            "servicenow_primary",
            "incident.update",
            RuntimeGovernedWriteCertificationMode::Live,
        )
        .expect("live request");

        assert_eq!(
            runner.run(request).unwrap_err(),
            RuntimeDelegatedAuthError::LiveEvidenceRequired
        );
    }

    #[test]
    fn sensitive_debug_and_json_never_leak_tokens() {
        let token = RuntimeDelegatedOAuthTokenSet::bearer(
            "delegated-access-secret",
            Some("delegated-refresh-secret"),
            time(1_600),
            ["incident.write"],
        );

        let debug = format!("{token:?}");
        let serialized = serde_json::to_string(&token).expect("token serializes");
        let text = redact_text("Authorization: Bearer raw-token\nclient_secret=secret");

        assert!(!debug.contains("delegated-access-secret"));
        assert!(!debug.contains("delegated-refresh-secret"));
        assert!(!serialized.contains("delegated-access-secret"));
        assert!(!serialized.contains("delegated-refresh-secret"));
        assert!(!text.contains("raw-token"));
        assert!(!text.contains("client_secret=secret"));
        assert!(text.contains(REDACTED));
    }
}
