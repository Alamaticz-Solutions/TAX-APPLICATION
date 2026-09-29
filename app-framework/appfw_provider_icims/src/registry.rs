use std::collections::BTreeMap;

use crate::IcimsProviderError;

pub const ICIMS_DISCOVERY_FETCH_API_CONTRACT_OPERATION: &str = "icims.discovery.fetch_api_contract";
pub const ICIMS_RECRUITING_QUERY_CANDIDATES_INCREMENTAL_OPERATION: &str =
    "icims.recruiting.query_candidates_incremental";
pub const ICIMS_RECRUITING_QUERY_JOBS_INCREMENTAL_OPERATION: &str =
    "icims.recruiting.query_jobs_incremental";
pub const ICIMS_RECRUITING_QUERY_APPLICATIONS_INCREMENTAL_OPERATION: &str =
    "icims.recruiting.query_applications_incremental";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IcimsNamedOperationRequest {
    pub name: String,
}

impl IcimsNamedOperationRequest {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IcimsOperationKind {
    FetchApiContract,
    QueryCandidatesIncremental,
    QueryJobsIncremental,
    QueryApplicationsIncremental,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IcimsOperationGate {
    AuthenticatedDeveloperDocs,
    SandboxOrTenantExport,
    StandardFieldMatrix,
    MarketplaceValidation,
    LiveCertification,
}

impl IcimsOperationGate {
    pub fn reason(self) -> &'static str {
        match self {
            Self::AuthenticatedDeveloperDocs => {
                "requires authenticated iCIMS Developer Community API documentation"
            }
            Self::SandboxOrTenantExport => {
                "requires iCIMS sandbox or tenant export for schemas and samples"
            }
            Self::StandardFieldMatrix => {
                "requires repeatable standard-field matrix and custom-field extension policy"
            }
            Self::MarketplaceValidation => {
                "requires marketplace validation or revalidation evidence when applicable"
            }
            Self::LiveCertification => "requires live connector certification evidence",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisteredIcimsOperation {
    pub name: &'static str,
    pub kind: IcimsOperationKind,
    pub gates: &'static [IcimsOperationGate],
    pub description: &'static str,
}

impl RegisteredIcimsOperation {
    pub fn fetch_api_contract() -> Self {
        planned_operation(
            ICIMS_DISCOVERY_FETCH_API_CONTRACT_OPERATION,
            IcimsOperationKind::FetchApiContract,
            "Authenticated Developer Community contract capture before provider execution.",
        )
    }

    pub fn query_candidates_incremental() -> Self {
        planned_operation(
            ICIMS_RECRUITING_QUERY_CANDIDATES_INCREMENTAL_OPERATION,
            IcimsOperationKind::QueryCandidatesIncremental,
            "Planned candidate/profile incremental read after schema, auth, and pagination evidence exists.",
        )
    }

    pub fn query_jobs_incremental() -> Self {
        planned_operation(
            ICIMS_RECRUITING_QUERY_JOBS_INCREMENTAL_OPERATION,
            IcimsOperationKind::QueryJobsIncremental,
            "Planned job/requisition incremental read after standard field evidence exists.",
        )
    }

    pub fn query_applications_incremental() -> Self {
        planned_operation(
            ICIMS_RECRUITING_QUERY_APPLICATIONS_INCREMENTAL_OPERATION,
            IcimsOperationKind::QueryApplicationsIncremental,
            "Planned application/workflow-status incremental read after schema and pagination evidence exists.",
        )
    }

    pub fn unsupported_reason(&self) -> &'static str {
        self.gates
            .first()
            .map(|gate| gate.reason())
            .unwrap_or("operation is planned but not executable")
    }
}

const ICIMS_DEVELOPER_DOC_GATES: &[IcimsOperationGate] = &[
    IcimsOperationGate::AuthenticatedDeveloperDocs,
    IcimsOperationGate::SandboxOrTenantExport,
    IcimsOperationGate::StandardFieldMatrix,
    IcimsOperationGate::LiveCertification,
];

fn planned_operation(
    name: &'static str,
    kind: IcimsOperationKind,
    description: &'static str,
) -> RegisteredIcimsOperation {
    RegisteredIcimsOperation {
        name,
        kind,
        gates: ICIMS_DEVELOPER_DOC_GATES,
        description,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IcimsOperationRegistry {
    operations: BTreeMap<&'static str, RegisteredIcimsOperation>,
}

impl IcimsOperationRegistry {
    pub fn new(operations: impl IntoIterator<Item = RegisteredIcimsOperation>) -> Self {
        let operations = operations
            .into_iter()
            .map(|operation| (operation.name, operation))
            .collect();

        Self { operations }
    }

    pub fn default_planned_operations() -> Self {
        Self::new([
            RegisteredIcimsOperation::fetch_api_contract(),
            RegisteredIcimsOperation::query_candidates_incremental(),
            RegisteredIcimsOperation::query_jobs_incremental(),
            RegisteredIcimsOperation::query_applications_incremental(),
        ])
    }

    pub fn get(&self, name: &str) -> Option<&RegisteredIcimsOperation> {
        self.operations.get(name)
    }

    pub fn operation_names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.operations.keys().copied()
    }

    pub fn ensure_operation_is_not_executable(
        &self,
        request: &IcimsNamedOperationRequest,
    ) -> Result<(), IcimsProviderError> {
        let operation = self
            .get(&request.name)
            .ok_or_else(|| IcimsProviderError::UnknownOperation(request.name.clone()))?;

        Err(IcimsProviderError::UnsupportedOperation {
            name: operation.name.to_string(),
            reason: operation.unsupported_reason(),
        })
    }
}

impl Default for IcimsOperationRegistry {
    fn default() -> Self {
        Self::default_planned_operations()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_exposes_stable_operation_names() {
        let registry = IcimsOperationRegistry::default();
        let names = registry.operation_names().collect::<Vec<_>>();

        assert_eq!(
            names,
            vec![
                ICIMS_DISCOVERY_FETCH_API_CONTRACT_OPERATION,
                ICIMS_RECRUITING_QUERY_APPLICATIONS_INCREMENTAL_OPERATION,
                ICIMS_RECRUITING_QUERY_CANDIDATES_INCREMENTAL_OPERATION,
                ICIMS_RECRUITING_QUERY_JOBS_INCREMENTAL_OPERATION,
            ]
        );
    }

    #[test]
    fn planned_operations_require_authenticated_developer_docs() {
        let registry = IcimsOperationRegistry::default();

        for name in registry.operation_names() {
            let request = IcimsNamedOperationRequest::new(name);
            let error = registry
                .ensure_operation_is_not_executable(&request)
                .unwrap_err();

            assert!(matches!(
                error,
                IcimsProviderError::UnsupportedOperation { reason, .. }
                    if reason.contains("authenticated iCIMS")
            ));
        }
    }

    #[test]
    fn unknown_raw_recruiting_operation_is_rejected() {
        let registry = IcimsOperationRegistry::default();
        let request = IcimsNamedOperationRequest::new("icims.recruiting.raw_endpoint");

        assert_eq!(
            registry
                .ensure_operation_is_not_executable(&request)
                .unwrap_err(),
            IcimsProviderError::UnknownOperation("icims.recruiting.raw_endpoint".to_string())
        );
    }
}
