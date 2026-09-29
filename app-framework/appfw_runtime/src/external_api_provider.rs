use std::collections::BTreeMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{extension::UserAuth, provider_keys::FrameworkProvider, PolicyAccess};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeExternalApiQueryLimits {
    pub max_results: u32,
    pub timeout_ms: u64,
}

impl Default for RuntimeExternalApiQueryLimits {
    fn default() -> Self {
        Self {
            max_results: 250,
            timeout_ms: 2_000,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RuntimeExternalApiNamedOperation {
    pub name: String,
    pub parameters: BTreeMap<String, Value>,
    pub limits: RuntimeExternalApiQueryLimits,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RuntimeExternalApiQueryResult {
    pub rows: Vec<Value>,
    pub metadata: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RuntimeExternalApiMutationResult {
    pub rows: Vec<Value>,
    pub metadata: Value,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeExternalApiOperation {
    HealthCheck,
    ExecuteNamedQuery,
    ExecuteNamedMutation,
}

impl RuntimeExternalApiOperation {
    pub const ALL: [RuntimeExternalApiOperation; 3] = [
        RuntimeExternalApiOperation::HealthCheck,
        RuntimeExternalApiOperation::ExecuteNamedQuery,
        RuntimeExternalApiOperation::ExecuteNamedMutation,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::HealthCheck => "health_check",
            Self::ExecuteNamedQuery => "execute_named_query",
            Self::ExecuteNamedMutation => "execute_named_mutation",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeExternalApiProviderDescriptor {
    pub provider: FrameworkProvider,
    pub data_source_name: String,
}

impl RuntimeExternalApiProviderDescriptor {
    pub fn new(provider: FrameworkProvider, data_source_name: impl Into<String>) -> Self {
        debug_assert!(
            provider.is_external_api_provider(),
            "external API descriptors must use an external API provider"
        );
        Self {
            provider,
            data_source_name: data_source_name.into(),
        }
    }

    pub fn provider_key(&self) -> &'static str {
        self.provider.key()
    }
}

pub trait RuntimeExternalApiProviderIdentity {
    fn external_api_data_source_name(&self) -> &str;

    fn external_api_framework_provider(&self) -> FrameworkProvider;

    fn external_api_provider_descriptor(&self) -> RuntimeExternalApiProviderDescriptor {
        RuntimeExternalApiProviderDescriptor::new(
            self.external_api_framework_provider(),
            self.external_api_data_source_name(),
        )
    }
}

/// External API provider contract: named, parameterized SaaS reads plus an
/// opt-in governed-write surface.
///
/// The operation payload deliberately carries only a stable operation name,
/// bound parameters, and limits. It does not expose raw URLs, HTTP methods,
/// SOQL, `sysparm_query`, SOAP payloads, or arbitrary vendor command text to
/// generated products, MCP, Kafka, GraphQL, or frontend callers.
#[async_trait]
pub trait RuntimeExternalApiProvider: Send + Sync + RuntimeExternalApiProviderIdentity {
    type Error: Send + Sync + 'static;

    async fn health_check(&self) -> Result<(), Self::Error>;

    async fn execute_named_query(
        &self,
        query: RuntimeExternalApiNamedOperation,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<RuntimeExternalApiQueryResult, Self::Error>;

    async fn execute_named_mutation(
        &self,
        mutation: RuntimeExternalApiNamedOperation,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<RuntimeExternalApiMutationResult, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestExternalApiProvider;

    impl RuntimeExternalApiProviderIdentity for TestExternalApiProvider {
        fn external_api_data_source_name(&self) -> &str {
            "salesforce_primary"
        }

        fn external_api_framework_provider(&self) -> FrameworkProvider {
            FrameworkProvider::Salesforce
        }
    }

    #[test]
    fn external_api_provider_descriptor_uses_external_api_key() {
        let provider = TestExternalApiProvider;
        let descriptor = provider.external_api_provider_descriptor();

        assert_eq!(descriptor.provider, FrameworkProvider::Salesforce);
        assert_eq!(descriptor.provider_key(), "salesforce");
        assert_eq!(descriptor.data_source_name, "salesforce_primary");
    }

    #[test]
    fn external_api_operations_have_stable_tokens() {
        let tokens = RuntimeExternalApiOperation::ALL
            .iter()
            .map(|operation| operation.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            tokens,
            vec![
                "health_check",
                "execute_named_query",
                "execute_named_mutation"
            ]
        );
    }

    #[test]
    fn named_operation_payload_has_no_raw_request_surface() {
        let operation = RuntimeExternalApiNamedOperation {
            name: "salesforce.account_by_id".to_string(),
            parameters: BTreeMap::from([("id".to_string(), Value::String("001xx".to_string()))]),
            limits: RuntimeExternalApiQueryLimits::default(),
        };

        assert_eq!(operation.name, "salesforce.account_by_id");
        assert_eq!(operation.parameters.len(), 1);
        assert_eq!(operation.limits.max_results, 250);
    }
}
