use std::collections::BTreeMap;

use crate::AiSearchProviderError;

pub const AI_SEARCH_SEARCH_OPERATION: &str = "ai_search.search";
pub const AI_SEARCH_EMBED_OPERATION: &str = "ai_search.embed";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiSearchNamedOperationRequest {
    pub name: String,
}

impl AiSearchNamedOperationRequest {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiSearchOperationKind {
    Search,
    Embed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiSearchOperationGate {
    EnterpriseContractEvidence,
    GatewayAuthEvidence,
    SharedSaasHttpExecutor,
    LiveCertification,
}

impl AiSearchOperationGate {
    pub fn reason(self) -> &'static str {
        match self {
            Self::EnterpriseContractEvidence => {
                "requires authenticated PDS AI search API spec/export and recorded-live fixtures"
            }
            Self::GatewayAuthEvidence => {
                "requires approved AI gateway selection, server-side secret_ref custody, prompt audit/SIEM retention, egress allowlist, and policy re-resolution evidence"
            }
            Self::SharedSaasHttpExecutor => {
                "requires the shared SaaS HTTP executor before provider calls can execute"
            }
            Self::LiveCertification => "requires live AI search provider certification evidence",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisteredAiSearchOperation {
    pub name: &'static str,
    pub kind: AiSearchOperationKind,
    pub gates: &'static [AiSearchOperationGate],
    pub description: &'static str,
}

impl RegisteredAiSearchOperation {
    pub fn search() -> Self {
        planned_operation(
            AI_SEARCH_SEARCH_OPERATION,
            AiSearchOperationKind::Search,
            "Planned enterprise search named query. Returns pointers for the chat orchestrator to re-resolve; never raw chat text.",
        )
    }

    pub fn embed() -> Self {
        planned_operation(
            AI_SEARCH_EMBED_OPERATION,
            AiSearchOperationKind::Embed,
            "Planned embedding named query for approved search ingestion or query embedding flows.",
        )
    }

    pub fn is_named_query(&self) -> bool {
        matches!(
            self.kind,
            AiSearchOperationKind::Search | AiSearchOperationKind::Embed
        )
    }

    pub fn unsupported_reason(&self) -> &'static str {
        self.gates
            .first()
            .map(|gate| gate.reason())
            .unwrap_or("operation is planned but not executable")
    }
}

const AI_SEARCH_CONTRACT_GATES: &[AiSearchOperationGate] = &[
    AiSearchOperationGate::EnterpriseContractEvidence,
    AiSearchOperationGate::GatewayAuthEvidence,
    AiSearchOperationGate::SharedSaasHttpExecutor,
    AiSearchOperationGate::LiveCertification,
];

fn planned_operation(
    name: &'static str,
    kind: AiSearchOperationKind,
    description: &'static str,
) -> RegisteredAiSearchOperation {
    RegisteredAiSearchOperation {
        name,
        kind,
        gates: AI_SEARCH_CONTRACT_GATES,
        description,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiSearchOperationRegistry {
    operations: BTreeMap<&'static str, RegisteredAiSearchOperation>,
}

impl AiSearchOperationRegistry {
    pub fn new(operations: impl IntoIterator<Item = RegisteredAiSearchOperation>) -> Self {
        let operations = operations
            .into_iter()
            .map(|operation| (operation.name, operation))
            .collect();

        Self { operations }
    }

    pub fn default_planned_operations() -> Self {
        Self::new([
            RegisteredAiSearchOperation::embed(),
            RegisteredAiSearchOperation::search(),
        ])
    }

    pub fn get(&self, name: &str) -> Option<&RegisteredAiSearchOperation> {
        self.operations.get(name)
    }

    pub fn operation_names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.operations.keys().copied()
    }

    pub fn ensure_named_query_is_not_executable(
        &self,
        request: &AiSearchNamedOperationRequest,
    ) -> Result<(), AiSearchProviderError> {
        let operation = self
            .get(&request.name)
            .ok_or_else(|| AiSearchProviderError::UnknownOperation(request.name.clone()))?;
        if !operation.is_named_query() {
            return Err(AiSearchProviderError::UnsupportedOperation {
                name: operation.name.to_string(),
                reason: "operation is not an AI search named query",
            });
        }

        Err(AiSearchProviderError::UnsupportedOperation {
            name: operation.name.to_string(),
            reason: operation.unsupported_reason(),
        })
    }

    pub fn ensure_named_mutation_is_not_executable(
        &self,
        request: &AiSearchNamedOperationRequest,
    ) -> Result<(), AiSearchProviderError> {
        let operation = self
            .get(&request.name)
            .ok_or_else(|| AiSearchProviderError::UnknownOperation(request.name.clone()))?;

        Err(AiSearchProviderError::UnsupportedMutation(match operation.kind {
            AiSearchOperationKind::Search | AiSearchOperationKind::Embed => {
                "AI search operations are named queries and must not use the governed-write path"
            }
        }))
    }
}

impl Default for AiSearchOperationRegistry {
    fn default() -> Self {
        Self::default_planned_operations()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_exposes_stable_operation_names() {
        let registry = AiSearchOperationRegistry::default();
        let names = registry.operation_names().collect::<Vec<_>>();

        assert_eq!(
            names,
            vec![AI_SEARCH_EMBED_OPERATION, AI_SEARCH_SEARCH_OPERATION]
        );
    }

    #[test]
    fn planned_queries_require_enterprise_contract_evidence() {
        let registry = AiSearchOperationRegistry::default();

        for name in [AI_SEARCH_EMBED_OPERATION, AI_SEARCH_SEARCH_OPERATION] {
            let request = AiSearchNamedOperationRequest::new(name);
            let error = registry
                .ensure_named_query_is_not_executable(&request)
                .unwrap_err();

            assert!(matches!(
                error,
                AiSearchProviderError::UnsupportedOperation { reason, .. }
                    if reason.contains("authenticated PDS AI search")
            ));
        }
    }

    #[test]
    fn planned_queries_require_gateway_auth_evidence() {
        let registry = AiSearchOperationRegistry::default();

        for operation in registry.operations.values() {
            assert!(
                operation
                    .gates
                    .contains(&AiSearchOperationGate::GatewayAuthEvidence),
                "{} missing gateway auth evidence gate",
                operation.name
            );
        }
    }

    #[test]
    fn unknown_operations_are_rejected() {
        let registry = AiSearchOperationRegistry::default();
        let request = AiSearchNamedOperationRequest::new("ai_search.raw_query");

        assert_eq!(
            registry.ensure_named_query_is_not_executable(&request),
            Err(AiSearchProviderError::UnknownOperation(
                "ai_search.raw_query".to_string()
            ))
        );
    }

    #[test]
    fn named_queries_cannot_cross_into_mutation_path() {
        let registry = AiSearchOperationRegistry::default();
        let request = AiSearchNamedOperationRequest::new(AI_SEARCH_SEARCH_OPERATION);

        assert!(matches!(
            registry.ensure_named_mutation_is_not_executable(&request),
            Err(AiSearchProviderError::UnsupportedMutation(reason))
                if reason.contains("named queries")
        ));
    }
}
