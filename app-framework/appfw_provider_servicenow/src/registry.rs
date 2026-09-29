use std::collections::BTreeMap;

use crate::ServiceNowProviderError;

pub const SERVICENOW_TABLE_EXPORT_SCHEMA_OPERATION: &str =
    "servicenow.table.export_schema_from_instance";
pub const SERVICENOW_TABLE_QUERY_INCREMENTAL_OPERATION: &str = "servicenow.table.query_incremental";
pub const SERVICENOW_TABLE_FETCH_BY_SYS_IDS_OPERATION: &str = "servicenow.table.fetch_by_sys_ids";
pub const SERVICENOW_TABLE_FETCH_REFERENCE_VALUES_OPERATION: &str =
    "servicenow.table.fetch_reference_values";
pub const SERVICENOW_INCIDENT_CREATE_OPERATION: &str = "servicenow.create_incident";
pub const SERVICENOW_INCIDENT_CREATE_POLICY_SCOPE: &str = "servicenow.incident.write";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceNowNamedOperationRequest {
    pub name: String,
}

impl ServiceNowNamedOperationRequest {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceNowOperationKind {
    ExportSchemaFromInstance,
    QueryIncremental,
    FetchBySysIds,
    FetchReferenceValues,
    CreateIncident,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceNowOperationGate {
    AuthenticatedInstanceOpenApiExport,
    TableDictionaryAndAclExport,
    DomainSeparationEvidence,
    EncodedQueryBuilderCertification,
    DelegatedActorContextEvidence,
    TokenStoreIsolationEvidence,
    NamedMutationRegistryEvidence,
    MutationRequestBindingEvidence,
    IdempotencyReplayEvidence,
    WritePolicyScopeEvidence,
    WriteAuditEvidence,
    LiveCertification,
}

impl ServiceNowOperationGate {
    pub fn reason(self) -> &'static str {
        match self {
            Self::AuthenticatedInstanceOpenApiExport => {
                "requires authenticated ServiceNow REST API Explorer OpenAPI export"
            }
            Self::TableDictionaryAndAclExport => {
                "requires ServiceNow table dictionary and ACL export for selected tables"
            }
            Self::DomainSeparationEvidence => {
                "requires tenant domain-separation evidence when enabled"
            }
            Self::EncodedQueryBuilderCertification => {
                "requires encoded-query builder certification before sysparm_query use"
            }
            Self::DelegatedActorContextEvidence => {
                "requires delegated actor context evidence for the on-behalf-of user and tenant"
            }
            Self::TokenStoreIsolationEvidence => {
                "requires token-store isolation evidence partitioned by user, tenant, and provider"
            }
            Self::NamedMutationRegistryEvidence => {
                "requires named mutation registry evidence with MCP exposure disabled"
            }
            Self::MutationRequestBindingEvidence => {
                "requires mutation request binding evidence for provider-owned request construction"
            }
            Self::IdempotencyReplayEvidence => "requires idempotency and replay rejection evidence",
            Self::WritePolicyScopeEvidence => {
                "requires write policy and scope enforcement evidence"
            }
            Self::WriteAuditEvidence => "requires write audit and retained evidence",
            Self::LiveCertification => "requires live connector certification evidence",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisteredServiceNowOperation {
    pub name: &'static str,
    pub kind: ServiceNowOperationKind,
    pub policy_scope: Option<&'static str>,
    pub gates: &'static [ServiceNowOperationGate],
    pub description: &'static str,
}

impl RegisteredServiceNowOperation {
    pub fn export_schema_from_instance() -> Self {
        planned_operation(
            SERVICENOW_TABLE_EXPORT_SCHEMA_OPERATION,
            ServiceNowOperationKind::ExportSchemaFromInstance,
            "Authenticated instance export capture for Table API schemas and dictionaries.",
        )
    }

    pub fn query_incremental() -> Self {
        planned_operation(
            SERVICENOW_TABLE_QUERY_INCREMENTAL_OPERATION,
            ServiceNowOperationKind::QueryIncremental,
            "Planned incremental table query using provider-owned table, fields, and sys_updated_on/sys_id predicates.",
        )
    }

    pub fn fetch_by_sys_ids() -> Self {
        planned_operation(
            SERVICENOW_TABLE_FETCH_BY_SYS_IDS_OPERATION,
            ServiceNowOperationKind::FetchBySysIds,
            "Planned bounded fetch by provider-owned table and caller-bound sys_id values.",
        )
    }

    pub fn fetch_reference_values() -> Self {
        planned_operation(
            SERVICENOW_TABLE_FETCH_REFERENCE_VALUES_OPERATION,
            ServiceNowOperationKind::FetchReferenceValues,
            "Planned reference value fetch after selected reference fields are exported.",
        )
    }

    pub fn create_incident() -> Self {
        governed_write_operation(
            SERVICENOW_INCIDENT_CREATE_OPERATION,
            ServiceNowOperationKind::CreateIncident,
            "Planned governed write for ServiceNow incident creation. Non-executable until G1 live evidence proves delegated auth, policy, idempotency, and audit.",
        )
    }

    pub fn is_named_read(&self) -> bool {
        matches!(
            self.kind,
            ServiceNowOperationKind::ExportSchemaFromInstance
                | ServiceNowOperationKind::QueryIncremental
                | ServiceNowOperationKind::FetchBySysIds
                | ServiceNowOperationKind::FetchReferenceValues
        )
    }

    pub fn is_named_mutation(&self) -> bool {
        matches!(self.kind, ServiceNowOperationKind::CreateIncident)
    }

    pub fn unsupported_reason(&self) -> &'static str {
        self.gates
            .first()
            .map(|gate| gate.reason())
            .unwrap_or("operation is planned but not executable")
    }
}

const SERVICE_NOW_INSTANCE_EXPORT_GATES: &[ServiceNowOperationGate] = &[
    ServiceNowOperationGate::AuthenticatedInstanceOpenApiExport,
    ServiceNowOperationGate::TableDictionaryAndAclExport,
    ServiceNowOperationGate::LiveCertification,
];
const SERVICE_NOW_GOVERNED_WRITE_GATES: &[ServiceNowOperationGate] = &[
    ServiceNowOperationGate::DelegatedActorContextEvidence,
    ServiceNowOperationGate::TokenStoreIsolationEvidence,
    ServiceNowOperationGate::NamedMutationRegistryEvidence,
    ServiceNowOperationGate::MutationRequestBindingEvidence,
    ServiceNowOperationGate::IdempotencyReplayEvidence,
    ServiceNowOperationGate::WritePolicyScopeEvidence,
    ServiceNowOperationGate::WriteAuditEvidence,
    ServiceNowOperationGate::LiveCertification,
];

fn planned_operation(
    name: &'static str,
    kind: ServiceNowOperationKind,
    description: &'static str,
) -> RegisteredServiceNowOperation {
    RegisteredServiceNowOperation {
        name,
        kind,
        policy_scope: None,
        gates: SERVICE_NOW_INSTANCE_EXPORT_GATES,
        description,
    }
}

fn governed_write_operation(
    name: &'static str,
    kind: ServiceNowOperationKind,
    description: &'static str,
) -> RegisteredServiceNowOperation {
    RegisteredServiceNowOperation {
        name,
        kind,
        policy_scope: Some(SERVICENOW_INCIDENT_CREATE_POLICY_SCOPE),
        gates: SERVICE_NOW_GOVERNED_WRITE_GATES,
        description,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceNowOperationRegistry {
    operations: BTreeMap<&'static str, RegisteredServiceNowOperation>,
}

impl ServiceNowOperationRegistry {
    pub fn new(operations: impl IntoIterator<Item = RegisteredServiceNowOperation>) -> Self {
        let operations = operations
            .into_iter()
            .map(|operation| (operation.name, operation))
            .collect();

        Self { operations }
    }

    pub fn default_planned_operations() -> Self {
        Self::new([
            RegisteredServiceNowOperation::export_schema_from_instance(),
            RegisteredServiceNowOperation::query_incremental(),
            RegisteredServiceNowOperation::fetch_by_sys_ids(),
            RegisteredServiceNowOperation::fetch_reference_values(),
            RegisteredServiceNowOperation::create_incident(),
        ])
    }

    pub fn get(&self, name: &str) -> Option<&RegisteredServiceNowOperation> {
        self.operations.get(name)
    }

    pub fn operation_names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.operations.keys().copied()
    }

    pub fn ensure_operation_is_not_executable(
        &self,
        request: &ServiceNowNamedOperationRequest,
    ) -> Result<(), ServiceNowProviderError> {
        let operation = self
            .get(&request.name)
            .ok_or_else(|| ServiceNowProviderError::UnknownOperation(request.name.clone()))?;

        Err(ServiceNowProviderError::UnsupportedOperation {
            name: operation.name.to_string(),
            reason: operation.unsupported_reason(),
        })
    }

    pub fn ensure_named_read_is_not_executable(
        &self,
        request: &ServiceNowNamedOperationRequest,
    ) -> Result<(), ServiceNowProviderError> {
        let operation = self
            .get(&request.name)
            .ok_or_else(|| ServiceNowProviderError::UnknownOperation(request.name.clone()))?;
        if !operation.is_named_read() {
            return Err(ServiceNowProviderError::UnsupportedOperation {
                name: operation.name.to_string(),
                reason: "operation is a named mutation and must use the governed-write path",
            });
        }

        Err(ServiceNowProviderError::UnsupportedOperation {
            name: operation.name.to_string(),
            reason: operation.unsupported_reason(),
        })
    }

    pub fn ensure_named_mutation_is_not_executable(
        &self,
        request: &ServiceNowNamedOperationRequest,
    ) -> Result<(), ServiceNowProviderError> {
        let operation = self
            .get(&request.name)
            .ok_or_else(|| ServiceNowProviderError::UnknownOperation(request.name.clone()))?;
        if !operation.is_named_mutation() {
            return Err(ServiceNowProviderError::UnsupportedMutation(
                "operation is a named read and must not use the governed-write path",
            ));
        }

        Err(ServiceNowProviderError::UnsupportedOperation {
            name: operation.name.to_string(),
            reason: operation.unsupported_reason(),
        })
    }
}

impl Default for ServiceNowOperationRegistry {
    fn default() -> Self {
        Self::default_planned_operations()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_exposes_stable_operation_names() {
        let registry = ServiceNowOperationRegistry::default();
        let names = registry.operation_names().collect::<Vec<_>>();

        assert_eq!(
            names,
            vec![
                SERVICENOW_INCIDENT_CREATE_OPERATION,
                SERVICENOW_TABLE_EXPORT_SCHEMA_OPERATION,
                SERVICENOW_TABLE_FETCH_BY_SYS_IDS_OPERATION,
                SERVICENOW_TABLE_FETCH_REFERENCE_VALUES_OPERATION,
                SERVICENOW_TABLE_QUERY_INCREMENTAL_OPERATION,
            ]
        );
    }

    #[test]
    fn planned_operations_require_authenticated_instance_export() {
        let registry = ServiceNowOperationRegistry::default();

        for name in [
            SERVICENOW_TABLE_EXPORT_SCHEMA_OPERATION,
            SERVICENOW_TABLE_FETCH_BY_SYS_IDS_OPERATION,
            SERVICENOW_TABLE_FETCH_REFERENCE_VALUES_OPERATION,
            SERVICENOW_TABLE_QUERY_INCREMENTAL_OPERATION,
        ] {
            let request = ServiceNowNamedOperationRequest::new(name);
            let error = registry
                .ensure_operation_is_not_executable(&request)
                .unwrap_err();

            assert!(matches!(
                error,
                ServiceNowProviderError::UnsupportedOperation { reason, .. }
                    if reason.contains("authenticated ServiceNow")
            ));
        }
    }

    #[test]
    fn governed_write_operation_requires_g1_evidence() {
        let registry = ServiceNowOperationRegistry::default();
        let operation = registry
            .get(SERVICENOW_INCIDENT_CREATE_OPERATION)
            .expect("incident create operation should be registered");

        assert!(operation.is_named_mutation());
        assert_eq!(operation.kind, ServiceNowOperationKind::CreateIncident);
        assert_eq!(
            operation.policy_scope,
            Some(SERVICENOW_INCIDENT_CREATE_POLICY_SCOPE)
        );
        assert_eq!(
            operation.gates,
            [
                ServiceNowOperationGate::DelegatedActorContextEvidence,
                ServiceNowOperationGate::TokenStoreIsolationEvidence,
                ServiceNowOperationGate::NamedMutationRegistryEvidence,
                ServiceNowOperationGate::MutationRequestBindingEvidence,
                ServiceNowOperationGate::IdempotencyReplayEvidence,
                ServiceNowOperationGate::WritePolicyScopeEvidence,
                ServiceNowOperationGate::WriteAuditEvidence,
                ServiceNowOperationGate::LiveCertification,
            ]
        );
    }

    #[test]
    fn named_read_and_mutation_paths_do_not_cross() {
        let registry = ServiceNowOperationRegistry::default();
        let read_request =
            ServiceNowNamedOperationRequest::new(SERVICENOW_TABLE_QUERY_INCREMENTAL_OPERATION);
        let write_request =
            ServiceNowNamedOperationRequest::new(SERVICENOW_INCIDENT_CREATE_OPERATION);

        assert!(matches!(
            registry
                .ensure_named_read_is_not_executable(&write_request)
                .unwrap_err(),
            ServiceNowProviderError::UnsupportedOperation { reason, .. }
                if reason.contains("named mutation")
        ));
        assert!(matches!(
            registry
                .ensure_named_mutation_is_not_executable(&read_request)
                .unwrap_err(),
            ServiceNowProviderError::UnsupportedMutation(reason)
                if reason.contains("named read")
        ));
    }

    #[test]
    fn unknown_raw_table_operation_is_rejected() {
        let registry = ServiceNowOperationRegistry::default();
        let request = ServiceNowNamedOperationRequest::new("servicenow.table.raw_sysparm_query");

        assert_eq!(
            registry
                .ensure_operation_is_not_executable(&request)
                .unwrap_err(),
            ServiceNowProviderError::UnknownOperation(
                "servicenow.table.raw_sysparm_query".to_string()
            )
        );
    }
}
