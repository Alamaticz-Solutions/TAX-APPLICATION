//! Runtime-owned Kafka ingress contracts.
//!
//! This module deliberately stops at configuration and invocation context.
//! Concrete broker clients belong behind an optional runtime ingress package or
//! feature once the shared host boundary is stable.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{collections::BTreeMap, path::Path};

use crate::{
    extension::UserAuth,
    operation::{RuntimeOperationDispatcher, RuntimeOperationRequest},
    ConfigError, RuntimeAppError,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeKafkaAuthMechanism {
    SaslScram,
    Mtls,
    OAuthBearer,
    CloudIam,
    LocalNoAuth,
}

impl RuntimeKafkaAuthMechanism {
    pub fn requires_secret_ref(self) -> bool {
        matches!(
            self,
            Self::SaslScram | Self::Mtls | Self::OAuthBearer | Self::CloudIam
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeKafkaAuthConfig {
    pub mechanism: RuntimeKafkaAuthMechanism,
    pub secret_ref: Option<String>,
}

impl RuntimeKafkaAuthConfig {
    pub fn validate(&self, path: &str) -> Result<(), ConfigError> {
        if self.mechanism.requires_secret_ref() && blank(self.secret_ref.as_deref()) {
            return Err(ConfigError::Load(format!(
                "{path}.secret_ref is required for Kafka auth mechanism {:?}",
                self.mechanism
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeKafkaServiceActor {
    pub subject: String,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
}

impl RuntimeKafkaServiceActor {
    pub fn validate(&self, path: &str) -> Result<(), ConfigError> {
        if blank(Some(&self.subject)) {
            return Err(ConfigError::Load(format!("{path}.subject is required")));
        }
        if self.roles.is_empty() && self.scopes.is_empty() {
            return Err(ConfigError::Load(format!(
                "{path} must configure at least one service role or scope"
            )));
        }
        Ok(())
    }

    pub fn to_user_auth(&self, tenant_id: impl Into<String>) -> UserAuth {
        UserAuth::service(
            tenant_id,
            self.subject.clone(),
            self.roles.clone(),
            self.scopes.clone(),
        )
        .with_ingress("kafka")
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum RuntimeKafkaTenantSource {
    MessageField { field: String },
    Fixed { value: String },
    None,
}

impl RuntimeKafkaTenantSource {
    pub fn validate(&self, path: &str) -> Result<(), ConfigError> {
        match self {
            Self::MessageField { field } if blank(Some(field)) => Err(ConfigError::Load(format!(
                "{path}.field is required when tenant source is message_field"
            ))),
            Self::Fixed { value } if blank(Some(value)) => Err(ConfigError::Load(format!(
                "{path}.value is required when tenant source is fixed"
            ))),
            _ => Ok(()),
        }
    }

    pub fn tenant_id(&self, payload: &Value) -> Result<String, RuntimeAppError> {
        match self {
            Self::Fixed { value } => Ok(value.clone()),
            Self::None => Ok(String::new()),
            Self::MessageField { field } => payload
                .get(field)
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(ToString::to_string)
                .ok_or_else(|| {
                    RuntimeAppError::Validation(format!(
                        "Kafka message is missing string tenant field `{field}`"
                    ))
                }),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum RuntimeKafkaArgumentMapping {
    ObjectFields,
    PayloadArgument { name: String },
}

impl Default for RuntimeKafkaArgumentMapping {
    fn default() -> Self {
        Self::ObjectFields
    }
}

impl RuntimeKafkaArgumentMapping {
    fn validate(&self, path: &str) -> Result<(), ConfigError> {
        match self {
            Self::ObjectFields => Ok(()),
            Self::PayloadArgument { name } if blank(Some(name)) => Err(ConfigError::Load(format!(
                "{path}.name is required when argument mapping is payload_argument"
            ))),
            Self::PayloadArgument { .. } => Ok(()),
        }
    }

    fn arguments(&self, payload: Value) -> Result<Map<String, Value>, RuntimeAppError> {
        match self {
            Self::ObjectFields => match payload {
                Value::Object(arguments) => Ok(arguments),
                other => Err(RuntimeAppError::Validation(format!(
                    "Kafka operation payload mapping object_fields requires an object payload, got {}",
                    value_kind(&other)
                ))),
            },
            Self::PayloadArgument { name } => {
                let mut arguments = Map::new();
                arguments.insert(name.clone(), payload);
                Ok(arguments)
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeKafkaOperationBinding {
    pub name: String,
    pub schema_name: Option<String>,
    pub type_name: Option<String>,
    #[serde(default)]
    pub argument_mapping: RuntimeKafkaArgumentMapping,
    pub fields: Option<Vec<String>>,
}

impl RuntimeKafkaOperationBinding {
    pub fn validate(&self) -> Result<(), ConfigError> {
        ensure_present("kafka operation name", &self.name)?;
        if let Some(schema_name) = &self.schema_name {
            ensure_present("kafka operation schema_name", schema_name)?;
        }
        if let Some(type_name) = &self.type_name {
            ensure_present("kafka operation type_name", type_name)?;
        }
        self.argument_mapping
            .validate("kafka operation argument_mapping")?;
        Ok(())
    }

    pub fn request_for_payload(
        &self,
        payload: Value,
    ) -> Result<RuntimeOperationRequest, RuntimeAppError> {
        Ok(RuntimeOperationRequest {
            name: self.name.clone(),
            schema_name: self.schema_name.clone(),
            type_name: self.type_name.clone(),
            arguments: self.argument_mapping.arguments(payload)?,
            fields: self.fields.clone(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeKafkaRetryPolicy {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_retry_max_attempts")]
    pub max_attempts: u32,
    #[serde(default = "default_retry_initial_backoff_ms")]
    pub initial_backoff_ms: u64,
    #[serde(default = "default_retry_max_backoff_ms")]
    pub max_backoff_ms: u64,
}

impl Default for RuntimeKafkaRetryPolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            max_attempts: default_retry_max_attempts(),
            initial_backoff_ms: default_retry_initial_backoff_ms(),
            max_backoff_ms: default_retry_max_backoff_ms(),
        }
    }
}

impl RuntimeKafkaRetryPolicy {
    pub fn validate(&self, path: &str) -> Result<(), ConfigError> {
        if !self.enabled {
            return Ok(());
        }
        if self.max_attempts < 2 {
            return Err(ConfigError::Load(format!(
                "{path}.max_attempts must be at least 2 when retry is enabled"
            )));
        }
        if self.initial_backoff_ms == 0 {
            return Err(ConfigError::Load(format!(
                "{path}.initial_backoff_ms must be greater than 0"
            )));
        }
        if self.max_backoff_ms < self.initial_backoff_ms {
            return Err(ConfigError::Load(format!(
                "{path}.max_backoff_ms must be greater than or equal to initial_backoff_ms"
            )));
        }
        Ok(())
    }

    pub fn requires_idempotency(&self) -> bool {
        self.enabled && self.max_attempts > 1
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeKafkaDeadLetterPolicy {
    #[serde(default)]
    pub enabled: bool,
    pub topic: Option<String>,
}

impl Default for RuntimeKafkaDeadLetterPolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            topic: None,
        }
    }
}

impl RuntimeKafkaDeadLetterPolicy {
    pub fn validate(&self, path: &str, source_topic: &str) -> Result<(), ConfigError> {
        if !self.enabled {
            return Ok(());
        }
        let Some(topic) = self
            .topic
            .as_deref()
            .filter(|topic| !topic.trim().is_empty())
        else {
            return Err(ConfigError::Load(format!(
                "{path}.topic is required when dead-letter handling is enabled"
            )));
        };
        if topic == source_topic {
            return Err(ConfigError::Load(format!(
                "{path}.topic must not match the source topic `{source_topic}`"
            )));
        }
        Ok(())
    }

    pub fn requires_idempotency(&self) -> bool {
        self.enabled
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeKafkaReadinessPolicy {
    pub max_lag_messages: Option<u64>,
    pub max_idle_ms: Option<u64>,
}

impl Default for RuntimeKafkaReadinessPolicy {
    fn default() -> Self {
        Self {
            max_lag_messages: None,
            max_idle_ms: None,
        }
    }
}

impl RuntimeKafkaReadinessPolicy {
    pub fn validate(&self, path: &str) -> Result<(), ConfigError> {
        if matches!(self.max_lag_messages, Some(0)) {
            return Err(ConfigError::Load(format!(
                "{path}.max_lag_messages must be greater than 0 when configured"
            )));
        }
        if matches!(self.max_idle_ms, Some(0)) {
            return Err(ConfigError::Load(format!(
                "{path}.max_idle_ms must be greater than 0 when configured"
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeKafkaConsumerConfig {
    pub name: String,
    pub topic: String,
    pub consumer_group: String,
    pub handler: String,
    pub actor: RuntimeKafkaServiceActor,
    pub tenant: RuntimeKafkaTenantSource,
    pub auth: RuntimeKafkaAuthConfig,
    pub operation: Option<RuntimeKafkaOperationBinding>,
    pub idempotency_key_field: Option<String>,
    #[serde(default)]
    pub retry: RuntimeKafkaRetryPolicy,
    #[serde(default)]
    pub dead_letter: RuntimeKafkaDeadLetterPolicy,
    #[serde(default)]
    pub readiness: RuntimeKafkaReadinessPolicy,
}

impl RuntimeKafkaConsumerConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        ensure_present("kafka consumer name", &self.name)?;
        ensure_present("kafka consumer topic", &self.topic)?;
        ensure_present("kafka consumer group", &self.consumer_group)?;
        ensure_present("kafka consumer handler", &self.handler)?;
        self.actor.validate("kafka consumer actor")?;
        self.tenant.validate("kafka consumer tenant")?;
        self.auth.validate("kafka consumer auth")?;
        if let Some(operation) = &self.operation {
            operation.validate()?;
        }
        self.retry.validate("kafka consumer retry")?;
        self.dead_letter
            .validate("kafka consumer dead_letter", &self.topic)?;
        self.readiness.validate("kafka consumer readiness")?;
        if (self.retry.requires_idempotency() || self.dead_letter.requires_idempotency())
            && blank(self.idempotency_key_field.as_deref())
        {
            return Err(ConfigError::Load(
                "kafka consumer idempotency_key_field is required when retry or dead-letter handling is enabled".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeKafkaIngressConfig {
    pub enabled: bool,
    #[serde(default)]
    pub consumers: Vec<RuntimeKafkaConsumerConfig>,
}

impl RuntimeKafkaIngressConfig {
    pub fn from_yaml_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let contents = std::fs::read_to_string(path).map_err(|err| ConfigError::Io {
            path: path.display().to_string(),
            message: err.to_string(),
        })?;
        let config: Self = serde_yaml::from_str(&contents).map_err(|err| ConfigError::Parse {
            path: path.display().to_string(),
            message: err.to_string(),
        })?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.enabled && self.consumers.is_empty() {
            return Err(ConfigError::Load(
                "Kafka ingress is enabled but no consumers are configured".to_string(),
            ));
        }
        for consumer in &self.consumers {
            consumer.validate()?;
        }
        Ok(())
    }

    pub fn consumer(&self, name: &str) -> Option<&RuntimeKafkaConsumerConfig> {
        self.consumers.iter().find(|consumer| consumer.name == name)
    }

    pub fn consumer_count(&self) -> usize {
        self.consumers.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeKafkaMessageContext {
    pub consumer_name: String,
    pub topic: String,
    pub partition: i32,
    pub offset: i64,
    pub key: Option<String>,
    pub event_id: Option<String>,
    pub correlation_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RuntimeKafkaMessage {
    pub context: RuntimeKafkaMessageContext,
    pub payload: Value,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeKafkaCdcOperation {
    Upsert,
    Delete,
}

impl RuntimeKafkaCdcOperation {
    fn parse(value: &str) -> Result<Self, RuntimeAppError> {
        match value {
            "upsert" => Ok(Self::Upsert),
            "delete" => Ok(Self::Delete),
            other => Err(RuntimeAppError::Validation(format!(
                "CDC event operation `{other}` is unsupported"
            ))),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RuntimeKafkaCdcEventEnvelope {
    pub event_id: String,
    pub tenant_id: String,
    pub source_system: String,
    pub source_schema: String,
    pub source_object: String,
    pub source_key: String,
    pub operation: RuntimeKafkaCdcOperation,
    pub occurred_at: String,
    pub data: Value,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeKafkaCdcOffsetCheckpoint {
    pub consumer_name: String,
    pub topic: String,
    pub partition: i32,
    pub offset: i64,
    pub event_id: String,
    pub source_object: String,
    pub source_key: String,
}

impl RuntimeKafkaCdcEventEnvelope {
    pub fn decode(message: &RuntimeKafkaMessage) -> Result<Self, RuntimeAppError> {
        let payload = message.payload.as_object().ok_or_else(|| {
            RuntimeAppError::Validation(
                "CDC event envelope requires an object message payload".to_string(),
            )
        })?;

        let event_id = payload_string(payload, "event_id")
            .or_else(|| message.context.event_id.clone())
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                RuntimeAppError::Validation("CDC event envelope is missing event_id".to_string())
            })?;
        let operation = RuntimeKafkaCdcOperation::parse(
            payload_string(payload, "operation")
                .as_deref()
                .ok_or_else(|| {
                    RuntimeAppError::Validation(
                        "CDC event envelope is missing operation".to_string(),
                    )
                })?,
        )?;
        let data = payload.get("data").cloned().ok_or_else(|| {
            RuntimeAppError::Validation("CDC event envelope is missing data".to_string())
        })?;

        Ok(Self {
            event_id,
            tenant_id: required_payload_string(payload, "tenant_id")?,
            source_system: required_payload_string(payload, "source_system")?,
            source_schema: required_payload_string(payload, "source_schema")?,
            source_object: required_payload_string(payload, "source_object")?,
            source_key: required_payload_string(payload, "source_key")?,
            operation,
            occurred_at: required_payload_string(payload, "occurred_at")?,
            data,
        })
    }

    pub fn checkpoint_for_message(
        &self,
        message: &RuntimeKafkaMessage,
    ) -> RuntimeKafkaCdcOffsetCheckpoint {
        RuntimeKafkaCdcOffsetCheckpoint {
            consumer_name: message.context.consumer_name.clone(),
            topic: message.context.topic.clone(),
            partition: message.context.partition,
            offset: message.context.offset,
            event_id: self.event_id.clone(),
            source_object: self.source_object.clone(),
            source_key: self.source_key.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RuntimeKafkaDispatchResult {
    pub consumer_name: String,
    pub operation_name: String,
    pub result: Value,
}

pub async fn dispatch_kafka_operation(
    dispatcher: &dyn RuntimeOperationDispatcher,
    consumer: &RuntimeKafkaConsumerConfig,
    message: RuntimeKafkaMessage,
) -> Result<RuntimeKafkaDispatchResult, RuntimeAppError> {
    let operation = consumer.operation.as_ref().ok_or_else(|| {
        RuntimeAppError::Validation(format!(
            "Kafka consumer `{}` does not define an operation binding",
            consumer.name
        ))
    })?;
    let tenant_id = consumer.tenant.tenant_id(&message.payload)?;
    let user = consumer.actor.to_user_auth(tenant_id);
    let request = operation.request_for_payload(message.payload)?;
    let result = dispatcher.call_operation(user, request).await?;

    Ok(RuntimeKafkaDispatchResult {
        consumer_name: consumer.name.clone(),
        operation_name: operation.name.clone(),
        result,
    })
}

#[async_trait]
pub trait RuntimeKafkaMessageSource {
    async fn next_message(
        &mut self,
        consumer: &RuntimeKafkaConsumerConfig,
    ) -> Result<Option<RuntimeKafkaMessage>, RuntimeAppError>;
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeKafkaWorkerShellReport {
    pub enabled: bool,
    pub consumers_checked: usize,
    pub messages_dispatched: usize,
}

pub async fn run_kafka_worker_shell(
    dispatcher: &dyn RuntimeOperationDispatcher,
    ingress: &RuntimeKafkaIngressConfig,
    source: &mut dyn RuntimeKafkaMessageSource,
) -> Result<RuntimeKafkaWorkerShellReport, RuntimeAppError> {
    ingress.validate().map_err(RuntimeAppError::Config)?;
    if !ingress.enabled {
        return Ok(RuntimeKafkaWorkerShellReport {
            enabled: false,
            consumers_checked: 0,
            messages_dispatched: 0,
        });
    }

    let runner =
        RuntimeKafkaIngressRunner::new(ingress, dispatcher).map_err(RuntimeAppError::Config)?;
    let mut messages_dispatched = 0;
    for consumer in &ingress.consumers {
        while let Some(message) = source.next_message(consumer).await? {
            runner.dispatch_message(message).await?;
            messages_dispatched += 1;
        }
    }

    Ok(RuntimeKafkaWorkerShellReport {
        enabled: true,
        consumers_checked: ingress.consumers.len(),
        messages_dispatched,
    })
}

pub struct RuntimeKafkaIngressRunner<'a> {
    ingress: &'a RuntimeKafkaIngressConfig,
    dispatcher: &'a dyn RuntimeOperationDispatcher,
}

impl<'a> RuntimeKafkaIngressRunner<'a> {
    pub fn new(
        ingress: &'a RuntimeKafkaIngressConfig,
        dispatcher: &'a dyn RuntimeOperationDispatcher,
    ) -> Result<Self, ConfigError> {
        ingress.validate()?;
        Ok(Self {
            ingress,
            dispatcher,
        })
    }

    pub async fn dispatch_message(
        &self,
        message: RuntimeKafkaMessage,
    ) -> Result<RuntimeKafkaDispatchResult, RuntimeAppError> {
        if !self.ingress.enabled {
            return Err(RuntimeAppError::Validation(
                "Kafka ingress is not enabled".to_string(),
            ));
        }
        let consumer = self
            .ingress
            .consumer(&message.context.consumer_name)
            .ok_or_else(|| {
                RuntimeAppError::Validation(format!(
                    "Kafka consumer `{}` is not configured",
                    message.context.consumer_name
                ))
            })?;
        if consumer.topic != message.context.topic {
            return Err(RuntimeAppError::Validation(format!(
                "Kafka message topic `{}` does not match configured topic `{}` for consumer `{}`",
                message.context.topic, consumer.topic, consumer.name
            )));
        }
        dispatch_kafka_operation(self.dispatcher, consumer, message).await
    }
}

fn ensure_present(label: &str, value: &str) -> Result<(), ConfigError> {
    if blank(Some(value)) {
        Err(ConfigError::Load(format!("{label} is required")))
    } else {
        Ok(())
    }
}

fn blank(value: Option<&str>) -> bool {
    value.map(str::trim).unwrap_or("").is_empty()
}

fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn payload_string(payload: &Map<String, Value>, field: &str) -> Option<String> {
    payload
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn required_payload_string(
    payload: &Map<String, Value>,
    field: &'static str,
) -> Result<String, RuntimeAppError> {
    payload_string(payload, field).ok_or_else(|| {
        RuntimeAppError::Validation(format!("CDC event envelope is missing {field}"))
    })
}

fn default_retry_max_attempts() -> u32 {
    1
}

fn default_retry_initial_backoff_ms() -> u64 {
    1_000
}

fn default_retry_max_backoff_ms() -> u64 {
    30_000
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::{collections::VecDeque, sync::Mutex};

    use crate::operation::{
        RuntimeOperation, RuntimeOperationCatalog, RuntimeOperationDispatcher,
        RuntimeOperationRequest,
    };

    fn valid_consumer() -> RuntimeKafkaConsumerConfig {
        RuntimeKafkaConsumerConfig {
            name: "crm-events".to_string(),
            topic: "crm.events".to_string(),
            consumer_group: "crm-events-worker".to_string(),
            handler: "services.crm_events".to_string(),
            actor: RuntimeKafkaServiceActor {
                subject: "crm-event-consumer".to_string(),
                roles: vec!["crm.integration_writer".to_string()],
                scopes: vec![],
            },
            tenant: RuntimeKafkaTenantSource::MessageField {
                field: "tenant_id".to_string(),
            },
            auth: RuntimeKafkaAuthConfig {
                mechanism: RuntimeKafkaAuthMechanism::SaslScram,
                secret_ref: Some("kafka/crm-consumer".to_string()),
            },
            operation: Some(RuntimeKafkaOperationBinding {
                name: "create_account".to_string(),
                schema_name: Some("crm".to_string()),
                type_name: Some("Account".to_string()),
                argument_mapping: RuntimeKafkaArgumentMapping::PayloadArgument {
                    name: "input".to_string(),
                },
                fields: None,
            }),
            idempotency_key_field: Some("event_id".to_string()),
            retry: RuntimeKafkaRetryPolicy {
                enabled: true,
                max_attempts: 3,
                initial_backoff_ms: 1_000,
                max_backoff_ms: 30_000,
            },
            dead_letter: RuntimeKafkaDeadLetterPolicy {
                enabled: true,
                topic: Some("crm.events.dlq".to_string()),
            },
            readiness: RuntimeKafkaReadinessPolicy {
                max_lag_messages: Some(1_000),
                max_idle_ms: Some(300_000),
            },
        }
    }

    fn valid_message(payload: Value) -> RuntimeKafkaMessage {
        RuntimeKafkaMessage {
            context: RuntimeKafkaMessageContext {
                consumer_name: "crm-events".to_string(),
                topic: "crm.events".to_string(),
                partition: 1,
                offset: 42,
                key: Some("account-1".to_string()),
                event_id: Some("event-1".to_string()),
                correlation_id: "correlation-1".to_string(),
            },
            payload,
            headers: BTreeMap::new(),
        }
    }

    fn valid_cdc_message() -> RuntimeKafkaMessage {
        valid_message(serde_json::json!({
            "event_id": "evt-123",
            "tenant_id": "tenant-1",
            "source_system": "salesforce",
            "source_schema": "salesforce_crm_v1",
            "source_object": "Account",
            "source_key": "001xx000003DGbYAAW",
            "operation": "upsert",
            "occurred_at": "2026-07-02T12:00:00Z",
            "data": {
                "Id": "001xx000003DGbYAAW",
                "Name": "Acme"
            }
        }))
    }

    #[test]
    fn validates_enabled_kafka_consumers() {
        RuntimeKafkaIngressConfig {
            enabled: true,
            consumers: vec![valid_consumer()],
        }
        .validate()
        .expect("valid consumer config");
    }

    #[test]
    fn loads_ingress_config_from_yaml_file() {
        let path = std::env::temp_dir().join(format!(
            "appfw-kafka-ingress-{}-{}.yaml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::write(
            &path,
            r#"
enabled: true
consumers:
- name: crm-events
  topic: crm.events
  consumer_group: crm-events-worker
  handler: services.crm_events
  actor:
    subject: crm-event-consumer
    roles:
    - crm.integration_writer
    scopes: []
  tenant:
    source: message_field
    field: tenant_id
  auth:
    mechanism: local_no_auth
    secret_ref: null
  operation:
    name: create_account
    schema_name: crm
    type_name: Account
    argument_mapping:
      mode: payload_argument
      name: input
    fields: null
  idempotency_key_field: event_id
  retry:
    enabled: true
    max_attempts: 3
    initial_backoff_ms: 1000
    max_backoff_ms: 30000
  dead_letter:
    enabled: true
    topic: crm.events.dlq
  readiness:
    max_lag_messages: 1000
    max_idle_ms: 300000
"#,
        )
        .expect("write config");

        let config =
            RuntimeKafkaIngressConfig::from_yaml_file(&path).expect("load runtime kafka config");
        std::fs::remove_file(&path).expect("remove config");

        assert!(config.enabled);
        assert_eq!(config.consumer_count(), 1);
        assert_eq!(config.consumers[0].name, "crm-events");
        assert!(config.consumers[0].retry.enabled);
        assert_eq!(
            config.consumers[0].dead_letter.topic.as_deref(),
            Some("crm.events.dlq")
        );
    }

    #[test]
    fn enabled_kafka_ingress_requires_consumers() {
        let err = RuntimeKafkaIngressConfig {
            enabled: true,
            consumers: vec![],
        }
        .validate()
        .expect_err("consumer required");

        assert!(err.to_string().contains("no consumers"));
    }

    #[test]
    fn broker_auth_requires_secret_for_secret_backed_mechanisms() {
        let mut consumer = valid_consumer();
        consumer.auth.secret_ref = None;

        let err = consumer.validate().expect_err("secret ref required");

        assert!(err.to_string().contains("secret_ref"));
    }

    #[test]
    fn retry_and_dead_letter_require_idempotency_key() {
        let mut consumer = valid_consumer();
        consumer.idempotency_key_field = None;

        let err = consumer
            .validate()
            .expect_err("idempotency required for retry and DLQ");

        assert!(err.to_string().contains("idempotency_key_field"));
    }

    #[test]
    fn dead_letter_topic_must_not_match_source_topic() {
        let mut consumer = valid_consumer();
        consumer.dead_letter.topic = Some(consumer.topic.clone());

        let err = consumer
            .validate()
            .expect_err("dead-letter topic must differ");

        assert!(err.to_string().contains("must not match"));
    }

    #[test]
    fn retry_backoff_must_be_ordered() {
        let mut consumer = valid_consumer();
        consumer.retry.max_backoff_ms = 100;

        let err = consumer.validate().expect_err("invalid retry backoff");

        assert!(err.to_string().contains("max_backoff_ms"));
    }

    #[test]
    fn service_actor_requires_governed_role_or_scope() {
        let mut consumer = valid_consumer();
        consumer.actor.roles.clear();

        let err = consumer
            .validate()
            .expect_err("actor role or scope required");

        assert!(err.to_string().contains("role or scope"));
    }

    #[test]
    fn service_actor_maps_to_runtime_user_auth() {
        let actor = valid_consumer().actor;
        let user = actor.to_user_auth("tenant-1");

        assert_eq!(user.user_name, "crm-event-consumer");
        assert_eq!(user.tenant_id, "tenant-1");
        assert_eq!(user.roles, vec!["crm.integration_writer"]);
        assert_eq!(user.principal_type, crate::RuntimePrincipalType::Service);
        assert_eq!(user.ingress.as_deref(), Some("kafka"));
        assert_eq!(user.on_behalf_of, None);
        assert!(user.token.is_empty());
    }

    #[test]
    fn operation_binding_maps_payload_argument() {
        let binding = valid_consumer().operation.expect("operation binding");
        let request = binding
            .request_for_payload(serde_json::json!({ "name": "Acme" }))
            .expect("request");

        assert_eq!(request.name, "create_account");
        assert_eq!(request.schema_name.as_deref(), Some("crm"));
        assert_eq!(request.type_name.as_deref(), Some("Account"));
        assert_eq!(request.arguments["input"]["name"], "Acme");
    }

    #[test]
    fn decodes_cdc_event_envelope_and_offset_checkpoint() {
        let message = valid_cdc_message();

        let envelope = RuntimeKafkaCdcEventEnvelope::decode(&message).expect("cdc envelope");
        let checkpoint = envelope.checkpoint_for_message(&message);

        assert_eq!(envelope.event_id, "evt-123");
        assert_eq!(envelope.tenant_id, "tenant-1");
        assert_eq!(envelope.source_object, "Account");
        assert_eq!(envelope.operation, RuntimeKafkaCdcOperation::Upsert);
        assert_eq!(envelope.data["Name"], "Acme");
        assert_eq!(checkpoint.topic, "crm.events");
        assert_eq!(checkpoint.partition, 1);
        assert_eq!(checkpoint.offset, 42);
        assert_eq!(checkpoint.source_key, "001xx000003DGbYAAW");
    }

    #[test]
    fn cdc_event_envelope_requires_governed_fields() {
        let mut message = valid_cdc_message();
        message.payload["event_id"] = Value::Null;
        message.context.event_id = None;

        let err = RuntimeKafkaCdcEventEnvelope::decode(&message).expect_err("event id is required");

        assert!(err.to_string().contains("event_id"));
    }

    #[test]
    fn object_field_mapping_requires_object_payload() {
        let binding = RuntimeKafkaOperationBinding {
            name: "create_account".to_string(),
            schema_name: Some("crm".to_string()),
            type_name: Some("Account".to_string()),
            argument_mapping: RuntimeKafkaArgumentMapping::ObjectFields,
            fields: None,
        };

        let err = binding
            .request_for_payload(serde_json::json!("bad"))
            .expect_err("object payload required");

        assert!(err.to_string().contains("object_fields"));
    }

    struct Dispatcher {
        calls: Mutex<Vec<(UserAuth, RuntimeOperationRequest)>>,
    }

    impl Dispatcher {
        fn new() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
            }
        }
    }

    impl RuntimeOperationCatalog for Dispatcher {
        fn list_operations(&self, _include_disabled: bool) -> Vec<RuntimeOperation> {
            vec![RuntimeOperation::disabled_mutation(
                "create_account",
                "crm",
                "Account",
                "Account",
                vec![],
                false,
            )]
        }
    }

    struct InMemorySource {
        messages: VecDeque<RuntimeKafkaMessage>,
    }

    impl InMemorySource {
        fn new(messages: impl IntoIterator<Item = RuntimeKafkaMessage>) -> Self {
            Self {
                messages: messages.into_iter().collect(),
            }
        }
    }

    #[async_trait]
    impl RuntimeKafkaMessageSource for InMemorySource {
        async fn next_message(
            &mut self,
            consumer: &RuntimeKafkaConsumerConfig,
        ) -> Result<Option<RuntimeKafkaMessage>, RuntimeAppError> {
            let Some(index) = self
                .messages
                .iter()
                .position(|message| message.context.consumer_name == consumer.name)
            else {
                return Ok(None);
            };
            Ok(self.messages.remove(index))
        }
    }

    #[async_trait]
    impl RuntimeOperationDispatcher for Dispatcher {
        async fn call_operation(
            &self,
            user: UserAuth,
            request: RuntimeOperationRequest,
        ) -> Result<Value, RuntimeAppError> {
            self.calls.lock().expect("calls lock").push((user, request));
            Ok(serde_json::json!({ "ok": true }))
        }
    }

    #[tokio::test]
    async fn dispatches_kafka_message_as_service_actor_operation() {
        let dispatcher = Dispatcher::new();
        let consumer = valid_consumer();
        let message = valid_message(serde_json::json!({
            "tenant_id": "tenant-1",
            "name": "Acme"
        }));

        let result = dispatch_kafka_operation(&dispatcher, &consumer, message)
            .await
            .expect("dispatch");

        assert_eq!(result.consumer_name, "crm-events");
        assert_eq!(result.operation_name, "create_account");
        assert_eq!(result.result["ok"], true);
        let calls = dispatcher.calls.lock().expect("calls lock");
        assert_eq!(calls[0].0.user_name, "crm-event-consumer");
        assert_eq!(calls[0].0.tenant_id, "tenant-1");
        assert_eq!(calls[0].1.arguments["input"]["name"], "Acme");
    }

    #[tokio::test]
    async fn runner_dispatches_configured_consumer_message() {
        let dispatcher = Dispatcher::new();
        let ingress = RuntimeKafkaIngressConfig {
            enabled: true,
            consumers: vec![valid_consumer()],
        };
        let runner = RuntimeKafkaIngressRunner::new(&ingress, &dispatcher).expect("runner");
        let message = valid_message(serde_json::json!({
            "tenant_id": "tenant-1",
            "name": "Acme"
        }));

        let result = runner.dispatch_message(message).await.expect("dispatch");

        assert_eq!(result.consumer_name, "crm-events");
        assert_eq!(result.operation_name, "create_account");
    }

    #[tokio::test]
    async fn runner_rejects_disabled_ingress() {
        let dispatcher = Dispatcher::new();
        let ingress = RuntimeKafkaIngressConfig {
            enabled: false,
            consumers: vec![valid_consumer()],
        };
        let runner = RuntimeKafkaIngressRunner::new(&ingress, &dispatcher).expect("runner");
        let message = valid_message(serde_json::json!({
            "tenant_id": "tenant-1",
            "name": "Acme"
        }));

        let err = runner
            .dispatch_message(message)
            .await
            .expect_err("disabled ingress rejected");

        assert!(err.to_string().contains("not enabled"));
    }

    #[tokio::test]
    async fn runner_rejects_unknown_consumer() {
        let dispatcher = Dispatcher::new();
        let ingress = RuntimeKafkaIngressConfig {
            enabled: true,
            consumers: vec![valid_consumer()],
        };
        let runner = RuntimeKafkaIngressRunner::new(&ingress, &dispatcher).expect("runner");
        let mut message = valid_message(serde_json::json!({
            "tenant_id": "tenant-1",
            "name": "Acme"
        }));
        message.context.consumer_name = "missing-consumer".to_string();

        let err = runner
            .dispatch_message(message)
            .await
            .expect_err("unknown consumer rejected");

        assert!(err.to_string().contains("missing-consumer"));
    }

    #[tokio::test]
    async fn runner_rejects_topic_mismatch() {
        let dispatcher = Dispatcher::new();
        let ingress = RuntimeKafkaIngressConfig {
            enabled: true,
            consumers: vec![valid_consumer()],
        };
        let runner = RuntimeKafkaIngressRunner::new(&ingress, &dispatcher).expect("runner");
        let mut message = valid_message(serde_json::json!({
            "tenant_id": "tenant-1",
            "name": "Acme"
        }));
        message.context.topic = "wrong.topic".to_string();

        let err = runner
            .dispatch_message(message)
            .await
            .expect_err("topic mismatch rejected");

        assert!(err.to_string().contains("wrong.topic"));
    }

    #[tokio::test]
    async fn worker_shell_drains_available_messages_without_broker_dependency() {
        let dispatcher = Dispatcher::new();
        let ingress = RuntimeKafkaIngressConfig {
            enabled: true,
            consumers: vec![valid_consumer()],
        };
        let mut source = InMemorySource::new([
            valid_message(serde_json::json!({
                "tenant_id": "tenant-1",
                "name": "Acme"
            })),
            valid_message(serde_json::json!({
                "tenant_id": "tenant-2",
                "name": "Globex"
            })),
        ]);

        let report = run_kafka_worker_shell(&dispatcher, &ingress, &mut source)
            .await
            .expect("worker shell");

        assert!(report.enabled);
        assert_eq!(report.consumers_checked, 1);
        assert_eq!(report.messages_dispatched, 2);
        let calls = dispatcher.calls.lock().expect("calls lock");
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].0.tenant_id, "tenant-1");
        assert_eq!(calls[1].0.tenant_id, "tenant-2");
    }

    #[tokio::test]
    async fn worker_shell_reports_disabled_ingress_without_polling() {
        let dispatcher = Dispatcher::new();
        let ingress = RuntimeKafkaIngressConfig {
            enabled: false,
            consumers: vec![valid_consumer()],
        };
        let mut source = InMemorySource::new([valid_message(serde_json::json!({
            "tenant_id": "tenant-1",
            "name": "Acme"
        }))]);

        let report = run_kafka_worker_shell(&dispatcher, &ingress, &mut source)
            .await
            .expect("disabled ingress report");

        assert!(!report.enabled);
        assert_eq!(report.consumers_checked, 0);
        assert_eq!(report.messages_dispatched, 0);
        assert!(dispatcher.calls.lock().expect("calls lock").is_empty());
    }
}
