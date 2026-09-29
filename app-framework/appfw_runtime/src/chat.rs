//! Feature-gated runtime shell for AI chat and answer streams.
//!
//! This module is intentionally thin: it proves the HTTP/SSE ingress, auth,
//! authorization, prompt-audit, request-context, and runtime-layer seams before
//! any model provider or generated tool execution is attached.

use std::{convert::Infallible, sync::Arc, time::Duration};

use async_trait::async_trait;
use axum::{
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::get,
    Json, Router,
};
use futures_util::stream;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::watch;
use tracing::warn;

use crate::{
    auth::RuntimeJwtExtractor, auth_state::RuntimeAuthState, extension::UserAuth,
    observability::RequestContext, security::SecurityConfig,
};

pub const CHAT_STREAM_PATH: &str = "/chat/stream";
pub const ANSWER_ENVELOPE_VERSION: &str = "answer_envelope@1";
pub const AI_SDK_ANSWER_ENVELOPE_PART_TYPE: &str = "data-answer-envelope";
pub const CHAT_TRANSCRIPT_SCHEMA_VERSION: &str = "chat_transcript_jsonl@1";
pub const AI_METADATA_SOURCE_SYSTEM: &str = "source_system";
pub const AI_METADATA_SOURCE_RECORD_LOCATOR: &str = "source_record_locator";
pub const AI_METADATA_SCORE: &str = "score";
pub const AI_METADATA_FRESHNESS_WATERMARK: &str = "freshness_watermark";
pub const AI_METADATA_MODEL: &str = "model";
pub const AI_METADATA_USAGE: &str = "usage";
pub const AI_METADATA_INDEX_VERSION: &str = "index_version";
pub const AI_METADATA_QUERY_REWRITE: &str = "query_rewrite";

const LAST_EVENT_ID_HEADER_NAME: &str = "last-event-id";
const X_ACCEL_BUFFERING_HEADER_NAME: &str = "x-accel-buffering";
const CHAT_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);

pub trait ChatRuntimeState: Clone + Send + Sync + 'static {
    fn auth_state(&self) -> RuntimeAuthState;
    fn security_config(&self) -> &SecurityConfig;
    fn prompt_audit_sink(&self) -> Option<Arc<dyn ChatPromptAuditSink>> {
        None
    }
    fn chat_stream_cancel_signal(
        &self,
        _request_context: &RequestContext,
        _user: &UserAuth,
    ) -> Option<ChatStreamCancelSignal> {
        None
    }
    fn chat_stream_heartbeat_interval(&self) -> Duration {
        CHAT_HEARTBEAT_INTERVAL
    }
}

#[derive(Clone, Debug)]
pub struct ChatStreamCancelSignal {
    receiver: watch::Receiver<bool>,
}

impl ChatStreamCancelSignal {
    pub fn new(receiver: watch::Receiver<bool>) -> Self {
        Self { receiver }
    }
}

#[async_trait]
pub trait ChatPromptAuditSink: Send + Sync + 'static {
    async fn append_chat_prompt_audit_event(
        &self,
        event: ChatPromptAuditEvent,
    ) -> Result<(), ChatPromptAuditError>;
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ChatPromptAuditEvent {
    pub event_type: &'static str,
    pub request_id: String,
    pub correlation_id: String,
    pub tenant_id: String,
    pub user_name: String,
    pub resume_from: Option<String>,
}

impl ChatPromptAuditEvent {
    fn stream_opened(
        request_context: &RequestContext,
        user: &UserAuth,
        resume_from: Option<String>,
    ) -> Self {
        Self {
            event_type: "chat_stream_opened",
            request_id: request_context.request_id.clone(),
            correlation_id: request_context.correlation_id.clone(),
            tenant_id: user.tenant_id.clone(),
            user_name: user.user_name.clone(),
            resume_from,
        }
    }
}

#[derive(Clone, Debug, thiserror::Error)]
#[error("chat prompt audit sink failed")]
pub struct ChatPromptAuditError;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AnswerEnvelopeV1 {
    pub version: String,
    pub refs: Vec<AnswerEnvelopeRef>,
    pub citations: Vec<AnswerEnvelopeCitation>,
    pub fallback_view: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
}

impl AnswerEnvelopeV1 {
    pub fn new(fallback_view: impl Into<String>) -> Self {
        Self {
            version: ANSWER_ENVELOPE_VERSION.to_string(),
            refs: Vec::new(),
            citations: Vec::new(),
            fallback_view: fallback_view.into(),
            view_hint: None,
            confidence: None,
        }
    }

    pub fn validate(&self) -> Result<(), AnswerEnvelopeValidationError> {
        if self.version != ANSWER_ENVELOPE_VERSION {
            return Err(AnswerEnvelopeValidationError::InvalidVersion);
        }
        if self.fallback_view.trim().is_empty() {
            return Err(AnswerEnvelopeValidationError::MissingFallbackView);
        }
        for reference in &self.refs {
            reference.validate()?;
        }
        for citation in &self.citations {
            citation.validate()?;
        }
        if let Some(confidence) = self.confidence {
            if !(0.0..=1.0).contains(&confidence) {
                return Err(AnswerEnvelopeValidationError::InvalidConfidence);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AiSdkAnswerEnvelopeDataPart {
    #[serde(rename = "type")]
    pub part_type: String,
    pub data: AnswerEnvelopeV1,
}

impl AiSdkAnswerEnvelopeDataPart {
    pub fn new(data: AnswerEnvelopeV1) -> Self {
        Self {
            part_type: AI_SDK_ANSWER_ENVELOPE_PART_TYPE.to_string(),
            data,
        }
    }

    pub fn validate(&self) -> Result<(), ChatStreamContractError> {
        if self.part_type != AI_SDK_ANSWER_ENVELOPE_PART_TYPE {
            return Err(ChatStreamContractError::InvalidAiSdkPartType);
        }
        self.data
            .validate()
            .map_err(ChatStreamContractError::InvalidAnswerEnvelope)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChatTranscriptEventType {
    Message,
    ToolCall,
    ToolResult,
    Card,
    Citation,
    AnswerEnvelope,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ChatTranscriptJsonlEvent {
    pub schema: String,
    #[serde(rename = "type")]
    pub event_type: ChatTranscriptEventType,
    pub payload: Value,
}

impl ChatTranscriptJsonlEvent {
    pub fn new(event_type: ChatTranscriptEventType, payload: Value) -> Self {
        Self {
            schema: CHAT_TRANSCRIPT_SCHEMA_VERSION.to_string(),
            event_type,
            payload,
        }
    }

    pub fn answer_envelope(part: AiSdkAnswerEnvelopeDataPart) -> Result<Self, serde_json::Error> {
        Ok(Self::new(
            ChatTranscriptEventType::AnswerEnvelope,
            serde_json::to_value(part)?,
        ))
    }

    pub fn validate(&self) -> Result<(), ChatStreamContractError> {
        if self.schema != CHAT_TRANSCRIPT_SCHEMA_VERSION {
            return Err(ChatStreamContractError::InvalidTranscriptSchema);
        }
        if self.payload.is_null() {
            return Err(ChatStreamContractError::MissingTranscriptPayload);
        }
        if !self.payload.is_object() {
            return Err(ChatStreamContractError::InvalidTranscriptPayloadShape);
        }
        if self.event_type == ChatTranscriptEventType::AnswerEnvelope {
            let part: AiSdkAnswerEnvelopeDataPart = serde_json::from_value(self.payload.clone())
                .map_err(ChatStreamContractError::InvalidTranscriptPayload)?;
            part.validate()?;
        }
        Ok(())
    }
}

pub fn transcript_events_to_jsonl(
    events: &[ChatTranscriptJsonlEvent],
) -> Result<String, ChatStreamContractError> {
    let mut lines = Vec::with_capacity(events.len());
    for event in events {
        event.validate()?;
        lines.push(
            serde_json::to_string(event).map_err(ChatStreamContractError::SerializeTranscript)?,
        );
    }
    Ok(lines.join("\n"))
}

pub fn transcript_events_from_jsonl(
    jsonl: &str,
) -> Result<Vec<ChatTranscriptJsonlEvent>, ChatStreamContractError> {
    let mut events = Vec::new();
    for (index, line) in jsonl.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let event: ChatTranscriptJsonlEvent = serde_json::from_str(trimmed).map_err(|source| {
            ChatStreamContractError::DeserializeTranscriptLine {
                line: index + 1,
                source,
            }
        })?;
        event.validate()?;
        events.push(event);
    }
    if events.is_empty() {
        return Err(ChatStreamContractError::EmptyTranscript);
    }
    Ok(events)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnswerEnvelopeRef {
    pub kind: String,
    pub record_locator: String,
    pub caption: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_hint: Option<String>,
}

impl AnswerEnvelopeRef {
    pub fn new(
        kind: impl Into<String>,
        record_locator: impl Into<String>,
        caption: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            record_locator: record_locator.into(),
            caption: caption.into(),
            view_hint: None,
        }
    }

    fn validate(&self) -> Result<(), AnswerEnvelopeValidationError> {
        ensure_present("ref.kind", &self.kind)?;
        ensure_present("ref.caption", &self.caption)?;
        ensure_record_locator("ref.record_locator", &self.record_locator)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnswerEnvelopeCitation {
    pub source: String,
    pub record_locator: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freshness_watermark: Option<String>,
}

impl AnswerEnvelopeCitation {
    pub fn new(source: impl Into<String>, record_locator: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            record_locator: record_locator.into(),
            freshness_watermark: None,
        }
    }

    fn validate(&self) -> Result<(), AnswerEnvelopeValidationError> {
        ensure_present("citation.source", &self.source)?;
        ensure_record_locator("citation.record_locator", &self.record_locator)
    }
}

#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
pub enum AnswerEnvelopeValidationError {
    #[error("answer envelope version must be answer_envelope@1")]
    InvalidVersion,
    #[error("answer envelope fallback_view must be present")]
    MissingFallbackView,
    #[error("answer envelope confidence must be between 0 and 1")]
    InvalidConfidence,
    #[error("{field} must be present")]
    MissingValue { field: &'static str },
    #[error("{field} must be an opaque record locator beginning with rl_")]
    InvalidRecordLocator { field: &'static str },
}

#[derive(Debug, thiserror::Error)]
pub enum ChatStreamContractError {
    #[error("AI SDK answer-envelope data part type must be data-answer-envelope")]
    InvalidAiSdkPartType,
    #[error("answer envelope data part is invalid: {0}")]
    InvalidAnswerEnvelope(AnswerEnvelopeValidationError),
    #[error("chat transcript event schema must be chat_transcript_jsonl@1")]
    InvalidTranscriptSchema,
    #[error("chat transcript payload must be present")]
    MissingTranscriptPayload,
    #[error("chat transcript payload must be an object")]
    InvalidTranscriptPayloadShape,
    #[error("chat transcript payload is invalid: {0}")]
    InvalidTranscriptPayload(serde_json::Error),
    #[error("failed to serialize chat transcript event: {0}")]
    SerializeTranscript(serde_json::Error),
    #[error("failed to deserialize chat transcript line {line}: {source}")]
    DeserializeTranscriptLine {
        line: usize,
        source: serde_json::Error,
    },
    #[error("chat transcript JSONL must contain at least one event")]
    EmptyTranscript,
}

fn ensure_present(field: &'static str, value: &str) -> Result<(), AnswerEnvelopeValidationError> {
    if value.trim().is_empty() {
        Err(AnswerEnvelopeValidationError::MissingValue { field })
    } else {
        Ok(())
    }
}

fn ensure_record_locator(
    field: &'static str,
    value: &str,
) -> Result<(), AnswerEnvelopeValidationError> {
    let locator = value.trim();
    if locator
        .strip_prefix("rl_")
        .filter(|suffix| !suffix.is_empty())
        .filter(|suffix| {
            suffix
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | ':' | '-'))
        })
        .is_some()
    {
        Ok(())
    } else {
        Err(AnswerEnvelopeValidationError::InvalidRecordLocator { field })
    }
}

pub fn chat_runtime_routes<S>() -> Router<S>
where
    S: ChatRuntimeState,
{
    Router::new().route(CHAT_STREAM_PATH, get(chat_stream::<S>))
}

#[tracing::instrument(skip(state, headers), fields(chat = true))]
async fn chat_stream<S>(State(state): State<S>, headers: HeaderMap) -> Response
where
    S: ChatRuntimeState,
{
    let request_context = RequestContext::from_headers(&headers);
    let security = state.security_config();

    if !security.chat_enabled {
        return chat_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "chat runtime is disabled",
            &request_context,
        );
    }

    let user =
        match authenticate_chat_user(state.auth_state(), headers.clone(), &request_context).await {
            Ok(user) => user,
            Err(response) => return response,
        };

    chat_stream_for_user(&state, &headers, request_context, user).await
}

async fn chat_stream_for_user<S>(
    state: &S,
    headers: &HeaderMap,
    request_context: RequestContext,
    user: UserAuth,
) -> Response
where
    S: ChatRuntimeState,
{
    let security = state.security_config();
    if !user_can_access_chat(security, &user) {
        return chat_error(
            StatusCode::FORBIDDEN,
            "chat access requires an allowed role or OAuth scope",
            &request_context,
        );
    }

    let resume_from = header_string(&headers, LAST_EVENT_ID_HEADER_NAME);
    let audit_event = ChatPromptAuditEvent::stream_opened(&request_context, &user, resume_from);
    let Some(audit_sink) = state.prompt_audit_sink() else {
        return chat_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "chat prompt audit sink is not installed",
            &request_context,
        );
    };
    if let Err(error) = audit_sink
        .append_chat_prompt_audit_event(audit_event.clone())
        .await
    {
        warn!(
            request_id = %request_context.request_id,
            correlation_id = %request_context.correlation_id,
            error = ?error,
            "failed to append chat prompt audit event"
        );
        return chat_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "chat prompt audit sink is unavailable",
            &request_context,
        );
    }

    chat_sse_response(
        &request_context,
        &user,
        audit_event.resume_from,
        state.chat_stream_cancel_signal(&request_context, &user),
        state.chat_stream_heartbeat_interval(),
    )
}

pub async fn authenticate_chat_user(
    auth_state: RuntimeAuthState,
    headers: HeaderMap,
    request_context: &RequestContext,
) -> Result<UserAuth, Response> {
    let extractor = RuntimeJwtExtractor::new(auth_state, headers, false)
        .await
        .map_err(|_| {
            chat_error(
                StatusCode::UNAUTHORIZED,
                "chat requests require an authenticated user",
                request_context,
            )
        })?;
    extractor
        .user
        .as_ref()
        .map(|user| (**user).clone())
        .ok_or_else(|| {
            chat_error(
                StatusCode::UNAUTHORIZED,
                "chat requests require an authenticated user",
                request_context,
            )
        })
}

pub fn user_can_access_chat(security: &SecurityConfig, user: &UserAuth) -> bool {
    security
        .chat_required_roles
        .iter()
        .any(|required| user.has_role(required))
        || security
            .chat_required_scopes
            .iter()
            .any(|required| user.has_scope(required))
}

fn chat_sse_response(
    request_context: &RequestContext,
    user: &UserAuth,
    resume_from: Option<String>,
    cancel_signal: Option<ChatStreamCancelSignal>,
    heartbeat_interval: Duration,
) -> Response {
    let heartbeat_interval = normalize_heartbeat_interval(heartbeat_interval);
    let ready = json!({
        "status": "ready",
        "stream": "chat",
        "release_ready": false,
        "orchestrator": {
            "status": "disabled",
            "reason": "CH1 transport shell is enabled before live AI/search orchestration"
        },
        "request_id": request_context.request_id,
        "correlation_id": request_context.correlation_id,
        "tenant_id": user.tenant_id,
        "user_name": user.user_name,
        "resume_from": resume_from,
    });
    let heartbeat = json!({
        "status": "heartbeat",
        "stream": "chat",
        "request_id": request_context.request_id,
        "correlation_id": request_context.correlation_id,
    });

    let mut response = Sse::new(chat_sse_events(
        request_context.request_id.clone(),
        ready,
        heartbeat,
        cancel_signal,
        heartbeat_interval,
    ))
    .keep_alive(
        KeepAlive::new()
            .interval(heartbeat_interval)
            .text("heartbeat"),
    )
    .into_response();
    response.headers_mut().insert(
        X_ACCEL_BUFFERING_HEADER_NAME,
        HeaderValue::from_static("no"),
    );
    response
}

#[derive(Clone, Debug)]
struct ChatSseEventState {
    request_id: String,
    ready: Value,
    heartbeat: Value,
    cancel_signal: Option<ChatStreamCancelSignal>,
    heartbeat_interval: Duration,
    next_event: ChatSseNextEvent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChatSseNextEvent {
    Ready,
    ImmediateHeartbeat,
    DelayedHeartbeat(u64),
}

fn chat_sse_events(
    request_id: String,
    ready: Value,
    heartbeat: Value,
    cancel_signal: Option<ChatStreamCancelSignal>,
    heartbeat_interval: Duration,
) -> impl futures_util::Stream<Item = Result<Event, Infallible>> {
    stream::unfold(
        ChatSseEventState {
            request_id,
            ready,
            heartbeat,
            cancel_signal,
            heartbeat_interval,
            next_event: ChatSseNextEvent::Ready,
        },
        |mut state| async move {
            if state
                .cancel_signal
                .as_ref()
                .map(|signal| *signal.receiver.borrow())
                .unwrap_or(false)
            {
                return None;
            }

            match state.next_event {
                ChatSseNextEvent::Ready => {
                    state.next_event = ChatSseNextEvent::ImmediateHeartbeat;
                    Some((
                        Ok(Event::default()
                            .event("ready")
                            .id(format!("{}:ready", state.request_id))
                            .data(state.ready.to_string())),
                        state,
                    ))
                }
                ChatSseNextEvent::ImmediateHeartbeat => {
                    state.next_event = ChatSseNextEvent::DelayedHeartbeat(1);
                    Some((Ok(heartbeat_event(&state, None)), state))
                }
                ChatSseNextEvent::DelayedHeartbeat(sequence) => {
                    if wait_for_heartbeat_or_cancel(&mut state).await {
                        return None;
                    }
                    state.next_event = ChatSseNextEvent::DelayedHeartbeat(sequence + 1);
                    Some((Ok(heartbeat_event(&state, Some(sequence))), state))
                }
            }
        },
    )
}

async fn wait_for_heartbeat_or_cancel(state: &mut ChatSseEventState) -> bool {
    if let Some(signal) = state.cancel_signal.as_mut() {
        tokio::select! {
            _ = tokio::time::sleep(state.heartbeat_interval) => false,
            changed = signal.receiver.changed() => changed.is_err() || *signal.receiver.borrow(),
        }
    } else {
        tokio::time::sleep(state.heartbeat_interval).await;
        false
    }
}

fn heartbeat_event(state: &ChatSseEventState, sequence: Option<u64>) -> Event {
    let event_id = match sequence {
        Some(sequence) => format!("{}:heartbeat:{sequence}", state.request_id),
        None => format!("{}:heartbeat", state.request_id),
    };
    Event::default()
        .event("heartbeat")
        .id(event_id)
        .data(state.heartbeat.to_string())
}

fn normalize_heartbeat_interval(interval: Duration) -> Duration {
    if interval.is_zero() {
        CHAT_HEARTBEAT_INTERVAL
    } else {
        interval
    }
}

fn chat_error(status: StatusCode, message: &str, request_context: &RequestContext) -> Response {
    (
        status,
        Json(json!({
            "errors": [{ "message": message }],
            "extensions": {
                "request_id": request_context.request_id,
                "correlation_id": request_context.correlation_id,
            }
        })),
    )
        .into_response()
}

fn header_string(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use axum::{
        body::HttpBody as _,
        http::{header, HeaderValue},
    };

    use crate::auth_config::JwtAuthConfig;

    use super::*;

    #[derive(Clone)]
    struct StaticChatState {
        security: SecurityConfig,
        audit: Option<Arc<MemoryAuditSink>>,
        cancel: Option<watch::Receiver<bool>>,
        heartbeat_interval: Duration,
    }

    impl StaticChatState {
        fn new(audit: Option<Arc<MemoryAuditSink>>) -> Self {
            Self {
                security: security_config(),
                audit,
                cancel: None,
                heartbeat_interval: Duration::from_millis(10),
            }
        }
    }

    impl ChatRuntimeState for StaticChatState {
        fn auth_state(&self) -> RuntimeAuthState {
            RuntimeAuthState::from(JwtAuthConfig {
                jwt_issuer: "https://tenant.okta.com/oauth2/default".to_string(),
                jwt_audience: "api://tenant".to_string(),
                okta_client_id: "client-id".to_string(),
            })
        }

        fn security_config(&self) -> &SecurityConfig {
            &self.security
        }

        fn prompt_audit_sink(&self) -> Option<Arc<dyn ChatPromptAuditSink>> {
            self.audit
                .as_ref()
                .map(|audit| audit.clone() as Arc<dyn ChatPromptAuditSink>)
        }

        fn chat_stream_cancel_signal(
            &self,
            _request_context: &RequestContext,
            _user: &UserAuth,
        ) -> Option<ChatStreamCancelSignal> {
            self.cancel
                .as_ref()
                .map(|receiver| ChatStreamCancelSignal::new(receiver.clone()))
        }

        fn chat_stream_heartbeat_interval(&self) -> Duration {
            self.heartbeat_interval
        }
    }

    struct MemoryAuditSink {
        events: Mutex<Vec<ChatPromptAuditEvent>>,
    }

    impl MemoryAuditSink {
        fn new() -> Self {
            Self {
                events: Mutex::new(Vec::new()),
            }
        }

        fn events(&self) -> Vec<ChatPromptAuditEvent> {
            self.events.lock().expect("audit lock").clone()
        }
    }

    #[async_trait]
    impl ChatPromptAuditSink for MemoryAuditSink {
        async fn append_chat_prompt_audit_event(
            &self,
            event: ChatPromptAuditEvent,
        ) -> Result<(), ChatPromptAuditError> {
            self.events.lock().expect("audit lock").push(event);
            Ok(())
        }
    }

    fn security_config() -> SecurityConfig {
        SecurityConfig {
            admin_ui_enabled: true,
            admin_troubleshooting_enabled: false,
            product_ui_enabled: true,
            chat_enabled: true,
            chat_required_roles: vec!["admin".to_string()],
            chat_required_scopes: Vec::new(),
            chat_prompt_audit_enabled: true,
            chat_prompt_audit_siem_enabled: true,
            chat_prompt_audit_sink: Some("pds-siem".to_string()),
            chat_prompt_audit_retention_days: 90,
            chat_kill_switch_active: false,
            #[cfg(feature = "mcp")]
            mcp_enabled: false,
            #[cfg(feature = "mcp")]
            mcp_mutations_enabled: false,
            #[cfg(feature = "mcp")]
            mcp_allowed_origins: vec![],
            #[cfg(feature = "mcp")]
            mcp_max_result_bytes: 256 * 1024,
            #[cfg(feature = "mcp")]
            mcp_max_resource_bytes: 256 * 1024,
            #[cfg(feature = "mcp")]
            mcp_max_batch_items: 20,
            #[cfg(feature = "mcp")]
            mcp_required_roles: vec!["admin".to_string()],
            #[cfg(feature = "mcp")]
            mcp_required_scopes: Vec::new(),
            #[cfg(feature = "mcp")]
            mcp_privileged_tool_roles: vec!["admin".to_string()],
            #[cfg(feature = "mcp")]
            mcp_privileged_tool_scopes: vec!["appfw:mcp.admin".to_string()],
            graphql_introspection_enabled: false,
            graphql_introspection_required_roles: vec!["admin".to_string()],
            graphql_introspection_required_scopes: vec![
                "developer".to_string(),
                "appfw:developer".to_string(),
                "appfw:graphql.introspection".to_string(),
            ],
            graphql_max_depth: 12,
            graphql_max_complexity: 500,
            request_body_limit_bytes: 1024 * 1024,
            rate_limit_per_second: 100,
            rate_limit_burst: 100,
        }
    }

    fn request_context() -> RequestContext {
        RequestContext {
            request_id: "request-chat-1".to_string(),
            correlation_id: "correlation-chat-1".to_string(),
        }
    }

    fn user_with_roles(roles: &[&str]) -> UserAuth {
        UserAuth::human(
            "tenant-1",
            "casey",
            "UTC",
            roles.iter().map(|role| (*role).to_string()).collect(),
            Vec::new(),
            "redacted",
        )
    }

    async fn next_sse_chunk(body: &mut axum::body::BoxBody) -> String {
        let bytes = body
            .data()
            .await
            .expect("next SSE chunk")
            .expect("SSE chunk is successful");
        String::from_utf8(bytes.to_vec()).expect("SSE chunk is utf-8")
    }

    #[test]
    fn answer_envelope_validates_canonical_shape() {
        let mut envelope = AnswerEnvelopeV1::new("list");
        envelope.view_hint = Some("flow-graph".to_string());
        envelope.confidence = Some(0.82);
        envelope.refs.push(AnswerEnvelopeRef::new(
            "nexus.Task",
            "rl_0123456789abcdef0123456789abcdef",
            "Stalled permit review",
        ));
        envelope.citations.push(AnswerEnvelopeCitation {
            source: "servicenow.task".to_string(),
            record_locator: "rl_feedfacecafebeef0123456789abcdef".to_string(),
            freshness_watermark: Some("2026-07-03T00:00:00Z".to_string()),
        });

        envelope.validate().expect("valid envelope");
        let json = serde_json::to_value(&envelope).expect("serializes");
        assert_eq!(json["version"], ANSWER_ENVELOPE_VERSION);
        assert_eq!(
            json["refs"][0]["record_locator"],
            "rl_0123456789abcdef0123456789abcdef"
        );
        assert_eq!(
            json["citations"][0]["freshness_watermark"],
            "2026-07-03T00:00:00Z"
        );
    }

    #[test]
    fn answer_envelope_rejects_internal_ids_and_bad_confidence() {
        let mut envelope = AnswerEnvelopeV1::new("card");
        envelope.refs.push(AnswerEnvelopeRef::new(
            "crm.Account",
            "a0c00000-0000-4000-8000-100000000003",
            "Acme",
        ));
        assert_eq!(
            envelope.validate(),
            Err(AnswerEnvelopeValidationError::InvalidRecordLocator {
                field: "ref.record_locator"
            })
        );

        envelope.refs[0].record_locator = "rl_safe_record_locator".to_string();
        envelope.confidence = Some(1.2);
        assert_eq!(
            envelope.validate(),
            Err(AnswerEnvelopeValidationError::InvalidConfidence)
        );
    }

    #[test]
    fn answer_envelope_pins_metadata_keys() {
        assert_eq!(AI_METADATA_SOURCE_SYSTEM, "source_system");
        assert_eq!(AI_METADATA_SOURCE_RECORD_LOCATOR, "source_record_locator");
        assert_eq!(AI_METADATA_SCORE, "score");
        assert_eq!(AI_METADATA_FRESHNESS_WATERMARK, "freshness_watermark");
        assert_eq!(AI_METADATA_MODEL, "model");
        assert_eq!(AI_METADATA_USAGE, "usage");
        assert_eq!(AI_METADATA_INDEX_VERSION, "index_version");
        assert_eq!(AI_METADATA_QUERY_REWRITE, "query_rewrite");
    }

    #[test]
    fn answer_envelope_schema_artifact_pins_runtime_contract() {
        let schema: Value = serde_json::from_str(include_str!(
            "../../app_gen/_config/chat_evals/_schemas/answer-envelope-v1.schema.json"
        ))
        .expect("answer-envelope schema parses");

        assert_eq!(
            schema["properties"]["version"]["const"],
            ANSWER_ENVELOPE_VERSION
        );
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(schema["properties"]["refs"]["items"]["$ref"], "#/$defs/ref");
        assert_eq!(
            schema["$defs"]["ref"]["properties"]["record_locator"]["pattern"],
            "^rl_[A-Za-z0-9_:-]+$"
        );
        assert_eq!(
            schema["$defs"]["citation"]["properties"]["record_locator"]["pattern"],
            "^rl_[A-Za-z0-9_:-]+$"
        );
        assert_eq!(schema["properties"]["confidence"]["minimum"], 0);
        assert_eq!(schema["properties"]["confidence"]["maximum"], 1);
    }

    #[test]
    fn answer_envelope_maps_to_ai_sdk_data_part() {
        let mut envelope = AnswerEnvelopeV1::new("list");
        envelope.refs.push(AnswerEnvelopeRef::new(
            "nexus.Task",
            "rl_0123456789abcdef0123456789abcdef",
            "Stalled permit review",
        ));
        let part = AiSdkAnswerEnvelopeDataPart::new(envelope);

        part.validate().expect("valid data part");
        let json = serde_json::to_value(&part).expect("serializes");
        assert_eq!(json["type"], AI_SDK_ANSWER_ENVELOPE_PART_TYPE);
        assert_eq!(json["data"]["version"], ANSWER_ENVELOPE_VERSION);
        assert_eq!(
            json["data"]["refs"][0]["record_locator"],
            "rl_0123456789abcdef0123456789abcdef"
        );
    }

    #[test]
    fn answer_envelope_rejects_bad_ai_sdk_part_type() {
        let part = AiSdkAnswerEnvelopeDataPart {
            part_type: "text".to_string(),
            data: AnswerEnvelopeV1::new("list"),
        };

        assert!(matches!(
            part.validate(),
            Err(ChatStreamContractError::InvalidAiSdkPartType)
        ));
    }

    #[test]
    fn transcript_jsonl_schema_artifact_pins_data_part_contract() {
        let schema: Value = serde_json::from_str(include_str!(
            "../../app_gen/_config/chat_evals/_schemas/chat-transcript-jsonl-event-v1.schema.json"
        ))
        .expect("chat transcript schema parses");

        assert_eq!(
            schema["properties"]["schema"]["const"],
            CHAT_TRANSCRIPT_SCHEMA_VERSION
        );
        let event_types = schema["properties"]["type"]["enum"]
            .as_array()
            .expect("event type enum");
        assert!(event_types
            .iter()
            .any(|event_type| event_type == "answer_envelope"));
        assert_eq!(
            schema["allOf"][0]["then"]["properties"]["payload"]["properties"]["type"]["const"],
            AI_SDK_ANSWER_ENVELOPE_PART_TYPE
        );
        assert_eq!(
            schema["allOf"][0]["then"]["properties"]["payload"]["properties"]["data"]["$ref"],
            "answer-envelope-v1.schema.json"
        );
    }

    #[test]
    fn transcript_jsonl_round_trips_answer_envelope_data_part() {
        let mut envelope = AnswerEnvelopeV1::new("list");
        envelope.refs.push(AnswerEnvelopeRef::new(
            "nexus.Task",
            "rl_0123456789abcdef0123456789abcdef",
            "Stalled permit review",
        ));
        let answer_event =
            ChatTranscriptJsonlEvent::answer_envelope(AiSdkAnswerEnvelopeDataPart::new(envelope))
                .expect("event serializes");
        let message_event = ChatTranscriptJsonlEvent::new(
            ChatTranscriptEventType::Message,
            json!({"role": "assistant", "content": "Two stalled tasks need review."}),
        );

        let jsonl = transcript_events_to_jsonl(&[message_event.clone(), answer_event.clone()])
            .expect("jsonl serializes");
        assert!(jsonl.contains(CHAT_TRANSCRIPT_SCHEMA_VERSION));
        assert!(jsonl.contains(AI_SDK_ANSWER_ENVELOPE_PART_TYPE));

        let parsed = transcript_events_from_jsonl(&jsonl).expect("jsonl parses");
        assert_eq!(parsed, vec![message_event, answer_event]);
    }

    #[test]
    fn transcript_jsonl_rejects_malformed_line_with_line_number() {
        let jsonl = concat!(
            r#"{"schema":"chat_transcript_jsonl@1","type":"message","payload":{"role":"assistant"}}"#,
            "\n",
            r#"{"schema":"chat_transcript_jsonl@1","type":"answer_envelope","payload":"#,
        );

        assert!(matches!(
            transcript_events_from_jsonl(jsonl),
            Err(ChatStreamContractError::DeserializeTranscriptLine { line: 2, .. })
        ));
    }

    #[test]
    fn transcript_jsonl_rejects_invalid_schema_and_empty_input() {
        let wrong_schema = ChatTranscriptJsonlEvent {
            schema: "chat_transcript_jsonl@2".to_string(),
            event_type: ChatTranscriptEventType::Message,
            payload: json!({"role": "assistant", "content": "hello"}),
        };

        assert!(matches!(
            wrong_schema.validate(),
            Err(ChatStreamContractError::InvalidTranscriptSchema)
        ));
        assert!(matches!(
            transcript_events_from_jsonl("\n\n"),
            Err(ChatStreamContractError::EmptyTranscript)
        ));
    }

    #[test]
    fn transcript_jsonl_rejects_bad_answer_envelope_data_part() {
        let event = ChatTranscriptJsonlEvent::new(
            ChatTranscriptEventType::AnswerEnvelope,
            json!({
                "type": "data-answer-envelope",
                "data": {
                    "version": "answer_envelope@1",
                    "refs": [{ "kind": "nexus.Task", "record_locator": "internal-id", "caption": "Task" }],
                    "citations": [],
                    "fallback_view": "list"
                }
            }),
        );

        assert!(matches!(
            event.validate(),
            Err(ChatStreamContractError::InvalidAnswerEnvelope(
                AnswerEnvelopeValidationError::InvalidRecordLocator {
                    field: "ref.record_locator"
                }
            ))
        ));
    }

    #[test]
    fn transcript_jsonl_rejects_non_object_payload() {
        let event = ChatTranscriptJsonlEvent::new(ChatTranscriptEventType::Message, json!("hello"));

        assert!(matches!(
            event.validate(),
            Err(ChatStreamContractError::InvalidTranscriptPayloadShape)
        ));
    }

    #[tokio::test]
    async fn chat_stream_requires_prompt_audit_before_opening() {
        let state = StaticChatState::new(None);
        let headers = HeaderMap::new();

        let response = chat_stream_for_user(
            &state,
            &headers,
            request_context(),
            user_with_roles(&["admin"]),
        )
        .await;

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn chat_stream_opens_sse_and_appends_prompt_audit_event() {
        let audit = Arc::new(MemoryAuditSink::new());
        let state = StaticChatState::new(Some(audit.clone()));
        let mut headers = HeaderMap::new();
        headers.insert("last-event-id", "previous-event".parse().expect("event id"));

        let response = chat_stream_for_user(
            &state,
            &headers,
            request_context(),
            user_with_roles(&["admin"]),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE),
            Some(&HeaderValue::from_static("text/event-stream"))
        );
        assert_eq!(
            response.headers().get(X_ACCEL_BUFFERING_HEADER_NAME),
            Some(&HeaderValue::from_static("no"))
        );
        let events = audit.events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "chat_stream_opened");
        assert_eq!(events[0].request_id, "request-chat-1");
        assert_eq!(events[0].correlation_id, "correlation-chat-1");
        assert_eq!(events[0].resume_from.as_deref(), Some("previous-event"));

        let mut body = response.into_body();
        let ready = next_sse_chunk(&mut body).await;
        assert!(ready.contains("event:ready") || ready.contains("event: ready"));
        assert!(ready.contains(r#""status":"disabled""#));
        assert!(ready.contains(r#""orchestrator""#));
        assert!(!ready.contains(AI_SDK_ANSWER_ENVELOPE_PART_TYPE));
        assert!(!ready.contains(ANSWER_ENVELOPE_VERSION));
        assert!(ready.contains(r#""resume_from":"previous-event""#));

        let heartbeat = next_sse_chunk(&mut body).await;
        assert!(heartbeat.contains("event:heartbeat") || heartbeat.contains("event: heartbeat"));
        assert!(heartbeat.contains(r#""status":"heartbeat""#));
    }

    #[tokio::test]
    async fn chat_stream_honors_server_side_cancel_signal() {
        let audit = Arc::new(MemoryAuditSink::new());
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let mut state = StaticChatState::new(Some(audit));
        state.cancel = Some(cancel_rx);
        state.heartbeat_interval = Duration::from_millis(25);
        let headers = HeaderMap::new();

        let response = chat_stream_for_user(
            &state,
            &headers,
            request_context(),
            user_with_roles(&["admin"]),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut body = response.into_body();
        let ready = next_sse_chunk(&mut body).await;
        assert!(ready.contains("event:ready") || ready.contains("event: ready"));
        let heartbeat = next_sse_chunk(&mut body).await;
        assert!(heartbeat.contains("event:heartbeat") || heartbeat.contains("event: heartbeat"));

        cancel_tx.send(true).expect("cancel signal sends");
        let closed = tokio::time::timeout(Duration::from_millis(100), body.data())
            .await
            .expect("stream observes cancel without waiting for a long heartbeat interval");
        assert!(closed.is_none(), "chat SSE body should close after cancel");
    }

    #[tokio::test]
    async fn chat_stream_closes_when_cancel_sender_drops() {
        let audit = Arc::new(MemoryAuditSink::new());
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let mut state = StaticChatState::new(Some(audit));
        state.cancel = Some(cancel_rx);
        state.heartbeat_interval = Duration::from_millis(25);
        let headers = HeaderMap::new();

        let response = chat_stream_for_user(
            &state,
            &headers,
            request_context(),
            user_with_roles(&["admin"]),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut body = response.into_body();
        assert!(next_sse_chunk(&mut body)
            .await
            .contains(r#""status":"disabled""#));
        assert!(next_sse_chunk(&mut body)
            .await
            .contains(r#""status":"heartbeat""#));

        drop(cancel_tx);
        let closed = tokio::time::timeout(Duration::from_millis(100), body.data())
            .await
            .expect("stream closes when the cancel manager is dropped");
        assert!(
            closed.is_none(),
            "chat SSE body should close instead of spinning heartbeats"
        );
    }

    #[test]
    fn zero_heartbeat_interval_falls_back_to_default() {
        assert_eq!(
            normalize_heartbeat_interval(Duration::ZERO),
            CHAT_HEARTBEAT_INTERVAL
        );
        assert_eq!(
            normalize_heartbeat_interval(Duration::from_millis(5)),
            Duration::from_millis(5)
        );
    }

    #[tokio::test]
    async fn chat_stream_rejects_unprivileged_users() {
        let audit = Arc::new(MemoryAuditSink::new());
        let state = StaticChatState::new(Some(audit));
        let headers = HeaderMap::new();

        let response = chat_stream_for_user(
            &state,
            &headers,
            request_context(),
            user_with_roles(&["analyst"]),
        )
        .await;

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
