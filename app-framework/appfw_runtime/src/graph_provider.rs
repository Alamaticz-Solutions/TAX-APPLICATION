use std::collections::BTreeMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{extension::UserAuth, provider_keys::FrameworkProvider, PolicyAccess};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeGraphQueryLimits {
    pub max_depth: u16,
    pub max_results: u32,
    pub timeout_ms: u64,
}

impl Default for RuntimeGraphQueryLimits {
    fn default() -> Self {
        Self {
            max_depth: 3,
            max_results: 250,
            timeout_ms: 2_000,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RuntimeGraphNamedQuery {
    pub name: String,
    pub query_text: String,
    pub parameters: BTreeMap<String, Value>,
    pub limits: RuntimeGraphQueryLimits,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RuntimeGraphQueryResult {
    pub rows: Vec<Value>,
    pub metadata: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RuntimeGraphMutationResult {
    pub rows: Vec<Value>,
    /// Aggregate mutation counters (nodes/relationships created/deleted, …).
    pub stats: Value,
    pub metadata: Value,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeGraphOperation {
    HealthCheck,
    ExecuteNamedQuery,
    ExecuteNamedMutation,
}

impl RuntimeGraphOperation {
    pub const ALL: [RuntimeGraphOperation; 3] = [
        RuntimeGraphOperation::HealthCheck,
        RuntimeGraphOperation::ExecuteNamedQuery,
        RuntimeGraphOperation::ExecuteNamedMutation,
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
pub struct RuntimeGraphProviderDescriptor {
    pub provider: FrameworkProvider,
    pub data_source_name: String,
}

impl RuntimeGraphProviderDescriptor {
    pub fn new(provider: FrameworkProvider, data_source_name: impl Into<String>) -> Self {
        Self {
            provider,
            data_source_name: data_source_name.into(),
        }
    }

    pub fn provider_key(&self) -> &'static str {
        self.provider.key()
    }
}

pub trait RuntimeGraphProviderIdentity {
    fn graph_data_source_name(&self) -> &str;

    fn graph_framework_provider(&self) -> FrameworkProvider {
        FrameworkProvider::Neo4j
    }

    fn graph_provider_descriptor(&self) -> RuntimeGraphProviderDescriptor {
        RuntimeGraphProviderDescriptor::new(
            self.graph_framework_provider(),
            self.graph_data_source_name(),
        )
    }
}

/// Graph provider contract: named, parameterized graph reads plus an opt-in
/// governed-write surface.
///
/// Unlike the relational CRUD providers' `RuntimeProviderDataClient`, graph
/// access is read-first: `execute_named_query` is the primary path, while
/// `execute_named_mutation` is governed and gated at the operation/exposure
/// layer (a product chooses whether to register and expose any write
/// operations, e.g. via `mcp_enabled`), not by a separate trait.
#[async_trait]
pub trait RuntimeGraphProvider: Send + Sync + RuntimeGraphProviderIdentity {
    type Error: Send + Sync + 'static;

    async fn health_check(&self) -> Result<(), Self::Error>;

    async fn execute_named_query(
        &self,
        query: RuntimeGraphNamedQuery,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<RuntimeGraphQueryResult, Self::Error>;

    async fn execute_named_mutation(
        &self,
        mutation: RuntimeGraphNamedQuery,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<RuntimeGraphMutationResult, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestGraphProvider;

    impl RuntimeGraphProviderIdentity for TestGraphProvider {
        fn graph_data_source_name(&self) -> &str {
            "graph_read"
        }
    }

    #[test]
    fn graph_provider_descriptor_uses_neo4j_key() {
        let provider = TestGraphProvider;
        let descriptor = provider.graph_provider_descriptor();

        assert_eq!(descriptor.provider, FrameworkProvider::Neo4j);
        assert_eq!(descriptor.provider_key(), "neo4j");
        assert_eq!(descriptor.data_source_name, "graph_read");
    }

    #[test]
    fn graph_operations_have_stable_tokens() {
        let tokens = RuntimeGraphOperation::ALL
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
}
