//! Runtime ingress contracts for HTTP, MCP, and future worker transports.
//!
//! This module is the forward-facing namespace for transport/ingress shells.
//! The existing `routing` module remains as a compatibility surface while
//! product templates migrate toward ingress-oriented naming.

#[cfg(feature = "chat")]
pub use crate::chat::{
    chat_runtime_routes, transcript_events_from_jsonl, transcript_events_to_jsonl,
    AiSdkAnswerEnvelopeDataPart, AnswerEnvelopeCitation, AnswerEnvelopeRef, AnswerEnvelopeV1,
    AnswerEnvelopeValidationError, ChatPromptAuditError, ChatPromptAuditEvent, ChatPromptAuditSink,
    ChatRuntimeState, ChatStreamCancelSignal, ChatStreamContractError, ChatTranscriptEventType,
    ChatTranscriptJsonlEvent, AI_METADATA_FRESHNESS_WATERMARK, AI_METADATA_INDEX_VERSION,
    AI_METADATA_MODEL, AI_METADATA_QUERY_REWRITE, AI_METADATA_SCORE,
    AI_METADATA_SOURCE_RECORD_LOCATOR, AI_METADATA_SOURCE_SYSTEM, AI_METADATA_USAGE,
    AI_SDK_ANSWER_ENVELOPE_PART_TYPE, ANSWER_ENVELOPE_VERSION, CHAT_STREAM_PATH,
    CHAT_TRANSCRIPT_SCHEMA_VERSION,
};
#[cfg(feature = "http")]
pub use crate::host::{serve_http_router, RuntimeHttpServerConfig};
pub use crate::host::{
    RuntimeHostError, RuntimeHostPlan, RuntimeIngressDescriptor, RuntimeIngressKind, RuntimeMode,
};
#[cfg(feature = "chat")]
pub use crate::ix::{
    ix_transport_routes, IxClientBinding, IxClientProfile, IxJwtVerifier, IxRuntimeService,
    IxTransportPolicy,
};
#[cfg(feature = "kafka")]
pub use crate::kafka::{
    run_kafka_worker_shell, RuntimeKafkaAuthConfig, RuntimeKafkaAuthMechanism,
    RuntimeKafkaConsumerConfig, RuntimeKafkaDeadLetterPolicy, RuntimeKafkaIngressConfig,
    RuntimeKafkaIngressRunner, RuntimeKafkaMessageContext, RuntimeKafkaMessageSource,
    RuntimeKafkaReadinessPolicy, RuntimeKafkaRetryPolicy, RuntimeKafkaServiceActor,
    RuntimeKafkaTenantSource, RuntimeKafkaWorkerShellReport,
};
#[cfg(feature = "chat")]
pub use crate::routing::IxRouteGroup;
#[cfg(feature = "http")]
pub use crate::routing::{
    apply_runtime_layers, assemble_runtime_router, assemble_runtime_router_for_mode,
    runtime_graphql_schema_routes, RuntimeRouteSet,
};
