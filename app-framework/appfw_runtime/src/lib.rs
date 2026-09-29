//! Shared App Framework runtime contracts for generated products and tooling.
//!
//! Product-facing runtime APIs stabilize here. Product code should depend on
//! `appfw-runtime` instead of framework-internal modules.
//!
//! The dormant domain-change foundation is intentionally crate-private:
//!
//! ```compile_fail,E0603
//! use appfw_runtime::domain_change;
//!
//! fn main() {
//!     let _ = domain_change::DOMAIN_CHANGE_SCHEMA_VERSION;
//! }
//! ```

#![allow(clippy::too_many_arguments)]

mod access;
#[cfg(feature = "http")]
pub mod admin;
pub mod archetype;
#[cfg(feature = "http")]
pub mod auth;
pub mod auth_config;
pub mod auth_state;
#[cfg(feature = "chat")]
pub mod chat;
mod config_error;
pub mod connection_security;
#[cfg(feature = "http")]
pub mod cors;
pub mod data_access;
mod datastore_error;
pub mod delegated_auth;
mod domain_change;
pub mod extension;
pub mod external_api_provider;
#[doc(hidden)]
pub mod fabric;
pub mod graph_provider;
#[cfg(feature = "http")]
pub mod graphiql;
pub mod host;
pub mod identifier;
pub mod ingress;
pub mod ix;
pub mod json;
#[cfg(feature = "kafka")]
pub mod kafka;
#[cfg(feature = "mcp")]
pub mod mcp;
mod metadata_error;
pub mod model_metadata;
pub mod observability;
pub mod operation;
pub mod policy_decision;
pub mod principal;
#[cfg(feature = "http")]
pub mod product_ui;
pub mod provider_bridge;
pub mod provider_capabilities;
pub mod provider_certification;
pub mod provider_contract_types;
pub mod provider_error;
pub mod provider_keys;
pub mod provider_plan;
mod provider_pool_stats;
pub mod provider_registry;
pub mod provider_request;
pub mod provider_result;
pub mod provider_time_period;
mod query_build_error;
pub mod query_cost;
pub mod query_filter;
pub mod query_ir;
#[cfg(feature = "http")]
pub mod readiness;
pub mod record_audit;
pub mod record_computed;
pub mod record_locator;
pub mod record_timezone;
pub mod record_validation;
pub mod record_version;
#[cfg(feature = "http")]
pub mod routing;
mod runtime_error;
pub mod saas_oauth;
pub mod saas_transport;
pub mod secrets;
pub mod security;
mod sensitive_metadata;
pub mod sync_worker;
pub mod tenant_isolation;

pub use access::{combine_access_filters, AccessAction, PolicyAccess};
pub use archetype::{
    ActionMode, ArchetypeConformanceCase, ArchetypeContext, ArchetypeContractError,
    ArchetypeEvidenceRef, ArchetypeModel, ArchetypeTelemetry, CanonicalAction, CanonicalEvent,
    CanonicalRead, ConformancePath, EventDisposition, ARCHETYPE_CONTRACT_VERSION,
};
#[cfg(feature = "http")]
pub use auth::RuntimeJwtExtractor;
pub use auth_state::RuntimeAuthState;
#[cfg(feature = "chat")]
pub use chat::{
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
pub use config_error::ConfigError;
pub use datastore_error::DataStoreError;
pub use delegated_auth::{
    InMemoryRuntimeDelegatedTokenStore, InMemoryRuntimeSaasWriteAuditSink,
    InMemoryRuntimeSaasWriteIdempotencyStore, RuntimeDelegatedAuthCodeRequest,
    RuntimeDelegatedAuthError, RuntimeDelegatedOAuthTokenSet, RuntimeDelegatedTokenKey,
    RuntimeDelegatedTokenStore, RuntimeGovernedWriteCertificationMode,
    RuntimeGovernedWriteCertificationReport, RuntimeGovernedWriteCertificationRequest,
    RuntimeGovernedWriteCertificationRunner, RuntimeSaasWriteAuditRecord,
    RuntimeSaasWriteAuditSink, RuntimeSaasWriteIdempotencyKey, RuntimeSaasWriteIdempotencyStore,
    RuntimeSaasWriteReplayDecision, RuntimeSaasWriteReplayRecord, RuntimeSaasWriteReplayState,
    GOVERNED_WRITE_EVIDENCE_FILE, GOVERNED_WRITE_MOCK_FIXTURE_FILE,
};
#[cfg(feature = "http")]
pub use extension::user_from_graphql_context;
pub use extension::{
    data_from_graphql_context, Claims, RuntimeHandlerContext, RuntimePrincipalType, UserAuth,
};
pub use external_api_provider::{
    RuntimeExternalApiMutationResult, RuntimeExternalApiNamedOperation,
    RuntimeExternalApiOperation, RuntimeExternalApiProvider, RuntimeExternalApiProviderDescriptor,
    RuntimeExternalApiProviderIdentity, RuntimeExternalApiQueryLimits,
    RuntimeExternalApiQueryResult,
};
pub use graph_provider::{
    RuntimeGraphMutationResult, RuntimeGraphNamedQuery, RuntimeGraphOperation,
    RuntimeGraphProvider, RuntimeGraphProviderDescriptor, RuntimeGraphProviderIdentity,
    RuntimeGraphQueryLimits, RuntimeGraphQueryResult,
};
#[cfg(feature = "http")]
pub use host::{serve_http_router, RuntimeHttpServerConfig};
pub use host::{
    RuntimeHostError, RuntimeHostPlan, RuntimeIngressDescriptor, RuntimeIngressKind, RuntimeMode,
};
#[cfg(feature = "kafka")]
pub use kafka::{
    dispatch_kafka_operation, run_kafka_worker_shell, RuntimeKafkaArgumentMapping,
    RuntimeKafkaAuthConfig, RuntimeKafkaAuthMechanism, RuntimeKafkaCdcEventEnvelope,
    RuntimeKafkaCdcOffsetCheckpoint, RuntimeKafkaCdcOperation, RuntimeKafkaConsumerConfig,
    RuntimeKafkaDeadLetterPolicy, RuntimeKafkaDispatchResult, RuntimeKafkaIngressConfig,
    RuntimeKafkaIngressRunner, RuntimeKafkaMessage, RuntimeKafkaMessageContext,
    RuntimeKafkaMessageSource, RuntimeKafkaOperationBinding, RuntimeKafkaReadinessPolicy,
    RuntimeKafkaRetryPolicy, RuntimeKafkaServiceActor, RuntimeKafkaTenantSource,
    RuntimeKafkaWorkerShellReport,
};
pub use metadata_error::MetadataError;
pub use operation::{
    RuntimeOperation, RuntimeOperationArg, RuntimeOperationCatalog, RuntimeOperationDispatcher,
    RuntimeOperationRequest, RuntimeOperationResolutionError,
};
pub use policy_decision::{
    AuthoritativeObject, DecisionEffect, DynamicFieldMask, EntitlementAuthorityKind,
    EntitlementProvenance, FieldMaskTreatment, PermissionDecision, PolicyDecisionError,
    POLICY_DECISION_CONTRACT_VERSION,
};
pub use principal::{
    AuthoritativeSubject, BoundedServiceIdentity, PermissionedPrincipal, PrincipalValidationError,
    PRINCIPAL_CONTRACT_VERSION,
};
pub use provider_bridge::{
    RuntimeProviderClient, RuntimeProviderDataClient, RuntimeProviderDescriptor,
    RuntimeProviderIdentity, RuntimeProviderOperation, RuntimeProviderOperationContract,
    RuntimeProviderOperationCounts, RuntimeProviderOperationRequirement,
    RuntimeProviderOperationSurface,
};
pub use provider_plan::{
    RuntimeProviderAggregatePlan, RuntimeProviderMutationKind, RuntimeProviderMutationPlan,
    RuntimeProviderQueryPlan,
};
pub use provider_pool_stats::ProviderPoolStats;
pub use provider_registry::RuntimeProviderRegistry;
pub use provider_request::RuntimeProviderPlanInput;
pub use provider_result::{RuntimeJsonAggregateResult, RuntimeJsonObj, RuntimeJsonQueryResult};
pub use query_build_error::QueryBuildError;
pub use query_cost::{QueryCost, QueryCostBudget};
pub use query_filter::{
    runtime_filter_capabilities_for_data_type, runtime_filter_capabilities_for_provider,
    runtime_filter_data_types, runtime_filter_operator_capability, runtime_filter_operator_support,
    runtime_filter_specs_for_data_type, RuntimeFilterCapabilities, RuntimeFilterDataTypeCapability,
    RuntimeFilterOp, RuntimeFilterOperatorCapability, RuntimeFilterOperatorSpec,
    RuntimeFilterOperatorSupport, RuntimeFilterValueShape, RUNTIME_FILTER_DATA_TYPES,
};
#[cfg(feature = "http")]
pub use readiness::{
    runtime_info_routes, RuntimeHealthCheck, RuntimeReadinessProbe, RuntimeReadinessState,
};
pub use record_audit::{RuntimeAuditEvent, RuntimeAuditQuery};
pub use runtime_error::{RuntimeAppError, RuntimeError};
pub use saas_oauth::{
    ReqwestRuntimeOAuthClientCredentialsTokenExecutor, RuntimeClientCredentialsSaasRequestExecutor,
    RuntimeHttpOAuthClientCredentialsTokenExecutor, RuntimeOAuthClientCredentialsTokenExecutor,
    RuntimeSaasOAuthError,
};
pub use saas_transport::{
    ReqwestRuntimeSaasRequestExecutor, ReqwestSaasHttpTransport, RuntimeHttpSaasRequestExecutor,
    RuntimeSaasEndpoint, RuntimeSaasExecutionError, RuntimeSaasHttpRequest,
    RuntimeSaasHttpRequestBody, RuntimeSaasHttpResponse, RuntimeSaasHttpTransport,
    RuntimeSaasRequest, RuntimeSaasRequestExecutor, RuntimeSaasResponse, RuntimeSaasResponseBody,
    RuntimeSaasResponseMetadata, RuntimeSaasTransportError,
};
pub use sync_worker::{
    run_sync_worker_shell, RuntimeSyncCheckpointState, RuntimeSyncCheckpointStore,
    RuntimeSyncDeadLetterRecord, RuntimeSyncInMemoryCheckpointStore,
    RuntimeSyncInMemoryProjectionStore, RuntimeSyncObject, RuntimeSyncObjectEchoLoopPolicy,
    RuntimeSyncObjectProvenance, RuntimeSyncProjectionStore, RuntimeSyncProjectionUpsertOutcome,
    RuntimeSyncProjectionWrite, RuntimeSyncRecordProvenance, RuntimeSyncStoredProjectionRecord,
    RuntimeSyncWorker, RuntimeSyncWorkerCheckpoint, RuntimeSyncWorkerConfig,
    RuntimeSyncWorkerDeadLetterPolicy, RuntimeSyncWorkerExecutionReport,
    RuntimeSyncWorkerGovernance, RuntimeSyncWorkerObjectExecutionReport,
    RuntimeSyncWorkerReadiness, RuntimeSyncWorkerRetryPolicy, RuntimeSyncWorkerShellReport,
    RuntimeSyncWorkerUpsertPolicy,
};
pub type HandlerResult<T> = anyhow::Result<T>;
pub type JsonValue = serde_json::Value;
