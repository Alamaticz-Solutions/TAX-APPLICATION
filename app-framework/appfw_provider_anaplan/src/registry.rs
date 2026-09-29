use std::collections::BTreeMap;

use appfw_saas_core::{ChunkDownloadContinuation, ReadRequestPageContinuation, SaasRequestPlan};

use crate::{
    AnaplanDataSensitivity, AnaplanNamedOperationRequest, AnaplanOperationAvailability,
    AnaplanOperationGate, AnaplanOperationParameter, AnaplanOperationPrimitive,
    AnaplanProviderError, AnaplanRestMethod, AnaplanRestRequestPlan, AnaplanTenantBinding,
};

pub const ANAPLAN_MODEL_GET_STATUS_OPERATION: &str = "anaplan.model.get_status";
pub const ANAPLAN_FILES_LIST_OPERATION: &str = "anaplan.files.list";
pub const ANAPLAN_VIEW_CREATE_READ_REQUEST_OPERATION: &str = "anaplan.view.create_read_request";
pub const ANAPLAN_VIEW_GET_READ_REQUEST_OPERATION: &str = "anaplan.view.get_read_request";
pub const ANAPLAN_VIEW_GET_READ_PAGE_OPERATION: &str = "anaplan.view.get_read_page";
pub const ANAPLAN_FILE_DOWNLOAD_CHUNK_OPERATION: &str = "anaplan.file.download_chunk";

const TENANT_EXPORT_GATES: &[AnaplanOperationGate] = &[
    AnaplanOperationGate::TenantExportAllowList,
    AnaplanOperationGate::LiveCertification,
];

const READ_REQUEST_PARAMETER: &[AnaplanOperationParameter] = &[AnaplanOperationParameter {
    name: "read_request",
    required: true,
    description: "typed read-request continuation returned by an earlier Anaplan read request",
}];
const CHUNK_PARAMETER: &[AnaplanOperationParameter] = &[AnaplanOperationParameter {
    name: "chunk",
    required: true,
    description: "typed chunk-download continuation returned by Anaplan file metadata",
}];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnaplanOperationKind {
    ModelGetStatus,
    FilesList,
    ViewCreateReadRequest,
    ViewGetReadRequest,
    ViewGetReadPage,
    FileDownloadChunk,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisteredAnaplanOperation {
    pub name: &'static str,
    pub kind: AnaplanOperationKind,
    pub primitive: AnaplanOperationPrimitive,
    pub method: AnaplanRestMethod,
    pub fixed_view_id: Option<&'static str>,
    pub fixed_file_id: Option<&'static str>,
    pub parameters: &'static [AnaplanOperationParameter],
    pub availability: AnaplanOperationAvailability,
    pub gates: &'static [AnaplanOperationGate],
    pub sensitivity: AnaplanDataSensitivity,
    pub description: &'static str,
}

impl RegisteredAnaplanOperation {
    pub fn model_get_status() -> Self {
        Self {
            name: ANAPLAN_MODEL_GET_STATUS_OPERATION,
            kind: AnaplanOperationKind::ModelGetStatus,
            primitive: AnaplanOperationPrimitive::GetModelStatus,
            method: AnaplanRestMethod::Get,
            fixed_view_id: None,
            fixed_file_id: None,
            parameters: &[],
            availability: AnaplanOperationAvailability::Executable,
            gates: &[],
            sensitivity: AnaplanDataSensitivity::Operational,
            description: "Fixed server-bound Anaplan model status read.",
        }
    }

    pub fn files_list() -> Self {
        Self {
            name: ANAPLAN_FILES_LIST_OPERATION,
            kind: AnaplanOperationKind::FilesList,
            primitive: AnaplanOperationPrimitive::ListFiles,
            method: AnaplanRestMethod::Get,
            fixed_view_id: None,
            fixed_file_id: None,
            parameters: &[],
            availability: AnaplanOperationAvailability::Executable,
            gates: &[],
            sensitivity: AnaplanDataSensitivity::BusinessConfidential,
            description:
                "List files for the server-bound workspace/model with fixed sort and pagination.",
        }
    }

    pub fn planned_view_create_read_request() -> Self {
        planned_operation(
            ANAPLAN_VIEW_CREATE_READ_REQUEST_OPERATION,
            AnaplanOperationKind::ViewCreateReadRequest,
            AnaplanOperationPrimitive::CreateViewReadRequest,
            AnaplanRestMethod::Post,
            &[],
            "View read-request creation requires a tenant-approved registry-bound view id.",
        )
    }

    pub fn planned_view_get_read_request() -> Self {
        planned_operation(
            ANAPLAN_VIEW_GET_READ_REQUEST_OPERATION,
            AnaplanOperationKind::ViewGetReadRequest,
            AnaplanOperationPrimitive::GetViewReadRequest,
            AnaplanRestMethod::Get,
            READ_REQUEST_PARAMETER,
            "Read-request status requires a tenant-approved registry-bound view id.",
        )
    }

    pub fn planned_view_get_read_page() -> Self {
        planned_operation(
            ANAPLAN_VIEW_GET_READ_PAGE_OPERATION,
            AnaplanOperationKind::ViewGetReadPage,
            AnaplanOperationPrimitive::GetViewReadPage,
            AnaplanRestMethod::Get,
            READ_REQUEST_PARAMETER,
            "Read-request page reads require a tenant-approved registry-bound view id.",
        )
    }

    pub fn planned_file_download_chunk() -> Self {
        planned_operation(
            ANAPLAN_FILE_DOWNLOAD_CHUNK_OPERATION,
            AnaplanOperationKind::FileDownloadChunk,
            AnaplanOperationPrimitive::DownloadFileChunk,
            AnaplanRestMethod::Get,
            CHUNK_PARAMETER,
            "File chunk downloads require a tenant-approved registry-bound file id.",
        )
    }

    pub fn view_create_read_request_for_view(
        name: &'static str,
        view_id: &'static str,
    ) -> Result<Self, AnaplanProviderError> {
        validate_registry_identifier("view_id", view_id)?;
        Ok(Self {
            name,
            kind: AnaplanOperationKind::ViewCreateReadRequest,
            primitive: AnaplanOperationPrimitive::CreateViewReadRequest,
            method: AnaplanRestMethod::Post,
            fixed_view_id: Some(view_id),
            fixed_file_id: None,
            parameters: &[],
            availability: AnaplanOperationAvailability::Executable,
            gates: &[],
            sensitivity: AnaplanDataSensitivity::BusinessConfidential,
            description: "Create a read request for a registry-bound Anaplan view.",
        })
    }

    pub fn view_get_read_request_for_view(
        name: &'static str,
        view_id: &'static str,
    ) -> Result<Self, AnaplanProviderError> {
        validate_registry_identifier("view_id", view_id)?;
        Ok(Self {
            name,
            kind: AnaplanOperationKind::ViewGetReadRequest,
            primitive: AnaplanOperationPrimitive::GetViewReadRequest,
            method: AnaplanRestMethod::Get,
            fixed_view_id: Some(view_id),
            fixed_file_id: None,
            parameters: READ_REQUEST_PARAMETER,
            availability: AnaplanOperationAvailability::Executable,
            gates: &[],
            sensitivity: AnaplanDataSensitivity::BusinessConfidential,
            description: "Fetch read-request metadata for a registry-bound Anaplan view.",
        })
    }

    pub fn view_get_read_page_for_view(
        name: &'static str,
        view_id: &'static str,
    ) -> Result<Self, AnaplanProviderError> {
        validate_registry_identifier("view_id", view_id)?;
        Ok(Self {
            name,
            kind: AnaplanOperationKind::ViewGetReadPage,
            primitive: AnaplanOperationPrimitive::GetViewReadPage,
            method: AnaplanRestMethod::Get,
            fixed_view_id: Some(view_id),
            fixed_file_id: None,
            parameters: READ_REQUEST_PARAMETER,
            availability: AnaplanOperationAvailability::Executable,
            gates: &[],
            sensitivity: AnaplanDataSensitivity::BusinessConfidential,
            description: "Fetch one read-request page for a registry-bound Anaplan view.",
        })
    }

    pub fn file_download_chunk_for_file(
        name: &'static str,
        file_id: &'static str,
    ) -> Result<Self, AnaplanProviderError> {
        validate_registry_identifier("file_id", file_id)?;
        Ok(Self {
            name,
            kind: AnaplanOperationKind::FileDownloadChunk,
            primitive: AnaplanOperationPrimitive::DownloadFileChunk,
            method: AnaplanRestMethod::Get,
            fixed_view_id: None,
            fixed_file_id: Some(file_id),
            parameters: CHUNK_PARAMETER,
            availability: AnaplanOperationAvailability::Executable,
            gates: &[],
            sensitivity: AnaplanDataSensitivity::BusinessConfidential,
            description: "Download one chunk for a registry-bound Anaplan file.",
        })
    }

    pub fn is_executable(&self) -> bool {
        self.availability == AnaplanOperationAvailability::Executable
    }

    pub fn unsupported_reason(&self) -> &'static str {
        self.gates
            .first()
            .map(|gate| gate.reason())
            .unwrap_or("operation is planned but not executable")
    }
}

fn planned_operation(
    name: &'static str,
    kind: AnaplanOperationKind,
    primitive: AnaplanOperationPrimitive,
    method: AnaplanRestMethod,
    parameters: &'static [AnaplanOperationParameter],
    description: &'static str,
) -> RegisteredAnaplanOperation {
    RegisteredAnaplanOperation {
        name,
        kind,
        primitive,
        method,
        fixed_view_id: None,
        fixed_file_id: None,
        parameters,
        availability: AnaplanOperationAvailability::PlannedUnsupported,
        gates: TENANT_EXPORT_GATES,
        sensitivity: AnaplanDataSensitivity::BusinessConfidential,
        description,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnaplanOperationRegistry {
    operations: BTreeMap<&'static str, RegisteredAnaplanOperation>,
}

impl AnaplanOperationRegistry {
    pub fn new(operations: impl IntoIterator<Item = RegisteredAnaplanOperation>) -> Self {
        let operations = operations
            .into_iter()
            .map(|operation| (operation.name, operation))
            .collect();

        Self { operations }
    }

    pub fn default_read_operations() -> Self {
        Self::new([
            RegisteredAnaplanOperation::model_get_status(),
            RegisteredAnaplanOperation::files_list(),
            RegisteredAnaplanOperation::planned_view_create_read_request(),
            RegisteredAnaplanOperation::planned_view_get_read_request(),
            RegisteredAnaplanOperation::planned_view_get_read_page(),
            RegisteredAnaplanOperation::planned_file_download_chunk(),
        ])
    }

    pub fn get(&self, name: &str) -> Option<&RegisteredAnaplanOperation> {
        self.operations.get(name)
    }

    pub fn operation_names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.operations.keys().copied()
    }

    pub fn build_operation_plan(
        &self,
        request: &AnaplanNamedOperationRequest,
        tenant_binding: &AnaplanTenantBinding,
    ) -> Result<AnaplanRestRequestPlan, AnaplanProviderError> {
        let operation = self
            .get(&request.name)
            .ok_or_else(|| AnaplanProviderError::UnknownOperation(request.name.clone()))?;

        if !operation.is_executable() {
            return Err(AnaplanProviderError::UnsupportedOperation {
                name: operation.name.to_string(),
                reason: operation.unsupported_reason(),
            });
        }

        match operation.kind {
            AnaplanOperationKind::ModelGetStatus => {
                build_model_get_status_plan(operation, request, tenant_binding)
            }
            AnaplanOperationKind::FilesList => {
                build_files_list_plan(operation, request, tenant_binding)
            }
            AnaplanOperationKind::ViewCreateReadRequest => {
                build_view_create_read_request_plan(operation, request, tenant_binding)
            }
            AnaplanOperationKind::ViewGetReadRequest => {
                build_view_get_read_request_plan(operation, request, tenant_binding)
            }
            AnaplanOperationKind::ViewGetReadPage => {
                build_view_get_read_page_plan(operation, request, tenant_binding)
            }
            AnaplanOperationKind::FileDownloadChunk => {
                build_file_download_chunk_plan(operation, request, tenant_binding)
            }
        }
    }

    pub fn build_saas_request_plan(
        &self,
        request: &AnaplanNamedOperationRequest,
        tenant_binding: &AnaplanTenantBinding,
    ) -> Result<SaasRequestPlan, AnaplanProviderError> {
        self.build_operation_plan(request, tenant_binding)?
            .to_saas_request_plan()
            .map_err(AnaplanProviderError::InvalidSaasRequestPlan)
    }
}

impl Default for AnaplanOperationRegistry {
    fn default() -> Self {
        Self::default_read_operations()
    }
}

fn build_model_get_status_plan(
    operation: &RegisteredAnaplanOperation,
    request: &AnaplanNamedOperationRequest,
    tenant_binding: &AnaplanTenantBinding,
) -> Result<AnaplanRestRequestPlan, AnaplanProviderError> {
    reject_unexpected_parameters(request)?;

    Ok(AnaplanRestRequestPlan::new(
        operation.name,
        operation.primitive,
        operation.method,
        format!("{}/status", tenant_binding.model_path_prefix()),
        request.limits.timeout_ms,
    )
    .with_caps(crate::operation::anaplan_response_caps(1)))
}

fn build_files_list_plan(
    operation: &RegisteredAnaplanOperation,
    request: &AnaplanNamedOperationRequest,
    tenant_binding: &AnaplanTenantBinding,
) -> Result<AnaplanRestRequestPlan, AnaplanProviderError> {
    reject_unexpected_parameters(request)?;
    let limit = request.limits.effective_max_results()?;

    Ok(AnaplanRestRequestPlan::new(
        operation.name,
        operation.primitive,
        operation.method,
        format!("{}/files", tenant_binding.model_path_prefix()),
        request.limits.timeout_ms,
    )
    .with_query_parameter("sort", "name")
    .with_query_parameter("limit", limit.to_string())
    .with_query_parameter("offset", request.limits.offset.to_string())
    .with_caps(crate::operation::anaplan_response_caps(limit)))
}

fn build_view_create_read_request_plan(
    operation: &RegisteredAnaplanOperation,
    request: &AnaplanNamedOperationRequest,
    tenant_binding: &AnaplanTenantBinding,
) -> Result<AnaplanRestRequestPlan, AnaplanProviderError> {
    reject_unexpected_parameters(request)?;
    let view_id = required_view_id(operation)?;

    Ok(AnaplanRestRequestPlan::new(
        operation.name,
        operation.primitive,
        operation.method,
        format!(
            "{}/views/{view_id}/readRequests/",
            tenant_binding.model_path_prefix()
        ),
        request.limits.timeout_ms,
    )
    .with_caps(crate::operation::anaplan_response_caps(1)))
}

fn build_view_get_read_request_plan(
    operation: &RegisteredAnaplanOperation,
    request: &AnaplanNamedOperationRequest,
    tenant_binding: &AnaplanTenantBinding,
) -> Result<AnaplanRestRequestPlan, AnaplanProviderError> {
    let view_id = required_view_id(operation)?;
    let continuation = required_read_request_continuation(request)?;
    validate_read_request_id(&continuation.request_id)?;

    Ok(AnaplanRestRequestPlan::new(
        operation.name,
        operation.primitive,
        operation.method,
        format!(
            "{}/views/{view_id}/readRequests/{}",
            tenant_binding.model_path_prefix(),
            continuation.request_id
        ),
        request.limits.timeout_ms,
    )
    .with_caps(crate::operation::anaplan_response_caps(1)))
}

fn build_view_get_read_page_plan(
    operation: &RegisteredAnaplanOperation,
    request: &AnaplanNamedOperationRequest,
    tenant_binding: &AnaplanTenantBinding,
) -> Result<AnaplanRestRequestPlan, AnaplanProviderError> {
    let view_id = required_view_id(operation)?;
    let continuation = required_read_request_continuation(request)?;
    validate_read_request_id(&continuation.request_id)?;

    let max_rows = continuation
        .page_size
        .unwrap_or(request.limits.effective_max_results()?);

    Ok(AnaplanRestRequestPlan::new(
        operation.name,
        operation.primitive,
        operation.method,
        format!(
            "{}/views/{view_id}/readRequests/{}/pages/{}",
            tenant_binding.model_path_prefix(),
            continuation.request_id,
            continuation.page
        ),
        request.limits.timeout_ms,
    )
    .with_caps(crate::operation::anaplan_response_caps(max_rows)))
}

fn build_file_download_chunk_plan(
    operation: &RegisteredAnaplanOperation,
    request: &AnaplanNamedOperationRequest,
    tenant_binding: &AnaplanTenantBinding,
) -> Result<AnaplanRestRequestPlan, AnaplanProviderError> {
    let file_id = required_file_id(operation)?;
    let chunk = required_chunk_continuation(request)?;

    if chunk.chunk_set_id != file_id {
        return Err(AnaplanProviderError::InvalidParameter {
            name: "chunk",
            reason: "chunk continuation must belong to the registry-bound file id",
        });
    }

    Ok(AnaplanRestRequestPlan::new(
        operation.name,
        operation.primitive,
        operation.method,
        format!(
            "{}/files/{file_id}/chunks/{}",
            tenant_binding.model_path_prefix(),
            chunk.chunk_id
        ),
        request.limits.timeout_ms,
    )
    .with_caps(crate::operation::anaplan_response_caps(1)))
}

fn required_view_id(
    operation: &RegisteredAnaplanOperation,
) -> Result<&'static str, AnaplanProviderError> {
    operation
        .fixed_view_id
        .ok_or(AnaplanProviderError::UnsupportedOperation {
            name: operation.name.to_string(),
            reason: "operation requires a registry-bound view id",
        })
}

fn required_file_id(
    operation: &RegisteredAnaplanOperation,
) -> Result<&'static str, AnaplanProviderError> {
    operation
        .fixed_file_id
        .ok_or(AnaplanProviderError::UnsupportedOperation {
            name: operation.name.to_string(),
            reason: "operation requires a registry-bound file id",
        })
}

fn reject_unexpected_parameters(
    request: &AnaplanNamedOperationRequest,
) -> Result<(), AnaplanProviderError> {
    if let Some(name) = request.parameters.keys().next() {
        return Err(AnaplanProviderError::InvalidParameter {
            name: leak_static_parameter_name(name),
            reason: "operation does not accept caller parameters",
        });
    }
    Ok(())
}

fn required_read_request_continuation(
    request: &AnaplanNamedOperationRequest,
) -> Result<&ReadRequestPageContinuation, AnaplanProviderError> {
    request
        .parameters
        .get("read_request")
        .ok_or(AnaplanProviderError::MissingParameter("read_request"))
        .and_then(|value| {
            value
                .as_read_request_page()
                .ok_or(AnaplanProviderError::InvalidParameter {
                    name: "read_request",
                    reason: "must be a typed read-request page continuation",
                })
        })
        .and_then(|continuation| {
            continuation
                .validate()
                .map_err(|_| AnaplanProviderError::InvalidParameter {
                    name: "read_request",
                    reason: "must be a valid read-request page continuation",
                })?;
            Ok(continuation)
        })
}

fn required_chunk_continuation(
    request: &AnaplanNamedOperationRequest,
) -> Result<&ChunkDownloadContinuation, AnaplanProviderError> {
    request
        .parameters
        .get("chunk")
        .ok_or(AnaplanProviderError::MissingParameter("chunk"))
        .and_then(|value| {
            value
                .as_chunk_download()
                .ok_or(AnaplanProviderError::InvalidParameter {
                    name: "chunk",
                    reason: "must be a typed chunk-download continuation",
                })
        })
        .and_then(|continuation| {
            continuation
                .validate()
                .map_err(|_| AnaplanProviderError::InvalidParameter {
                    name: "chunk",
                    reason: "must be a valid chunk-download continuation",
                })?;
            Ok(continuation)
        })
}

fn validate_registry_identifier(
    name: &'static str,
    value: &str,
) -> Result<(), AnaplanProviderError> {
    if value.trim() != value || value.is_empty() {
        return Err(AnaplanProviderError::InvalidParameter {
            name,
            reason: "must be a non-empty registry-owned identifier",
        });
    }

    if value
        .chars()
        .any(|character| !(character.is_ascii_alphanumeric() || matches!(character, '-' | '_')))
    {
        return Err(AnaplanProviderError::InvalidParameter {
            name,
            reason: "must contain only ASCII letters, digits, hyphen, or underscore",
        });
    }

    Ok(())
}

fn validate_read_request_id(value: &str) -> Result<(), AnaplanProviderError> {
    validate_registry_identifier("read_request", value)
}

fn leak_static_parameter_name(name: &str) -> &'static str {
    match name {
        "read_request" => "read_request",
        "chunk" => "chunk",
        _ => "parameter",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AnaplanParameterValue, AnaplanQueryLimits};
    use appfw_saas_core::{
        SaasHttpMethod, SaasRequestBody, SaasResponseCaps, SaasTransportProtocol,
    };

    const VIEW_READ_PAGE_OPERATION: &str = "anaplan.view.sales_forecast.get_read_page";
    const VIEW_CREATE_OPERATION: &str = "anaplan.view.sales_forecast.create_read_request";
    const FILE_CHUNK_OPERATION: &str = "anaplan.file.sales_forecast.download_chunk";

    fn tenant() -> AnaplanTenantBinding {
        AnaplanTenantBinding::new(
            "https://api.anaplan.com",
            "https://auth.anaplan.com",
            "workspace123",
            "model456",
        )
        .expect("tenant binding")
    }

    #[test]
    fn registry_exposes_stable_operation_names() {
        let registry = AnaplanOperationRegistry::default();
        let names = registry.operation_names().collect::<Vec<_>>();

        assert_eq!(
            names,
            vec![
                ANAPLAN_FILE_DOWNLOAD_CHUNK_OPERATION,
                ANAPLAN_FILES_LIST_OPERATION,
                ANAPLAN_MODEL_GET_STATUS_OPERATION,
                ANAPLAN_VIEW_CREATE_READ_REQUEST_OPERATION,
                ANAPLAN_VIEW_GET_READ_PAGE_OPERATION,
                ANAPLAN_VIEW_GET_READ_REQUEST_OPERATION,
            ]
        );
    }

    #[test]
    fn model_status_converts_to_server_bound_saas_request_plan() {
        let registry = AnaplanOperationRegistry::default();
        let request = AnaplanNamedOperationRequest::new(ANAPLAN_MODEL_GET_STATUS_OPERATION);

        let plan = registry
            .build_saas_request_plan(&request, &tenant())
            .expect("SaaS request plan");

        assert_eq!(plan.operation_name, ANAPLAN_MODEL_GET_STATUS_OPERATION);
        assert_eq!(plan.protocol, SaasTransportProtocol::RestJson);
        assert_eq!(plan.method, SaasHttpMethod::Get);
        assert_eq!(
            plan.path,
            "/2/0/workspaces/workspace123/models/model456/status"
        );
        assert!(plan.query.is_empty());
        assert_eq!(plan.caps.max_rows, 1);
        assert_eq!(plan.body, SaasRequestBody::Empty);
        plan.validate().expect("shared plan accepts status read");
    }

    #[test]
    fn files_list_uses_fixed_sort_offset_limit_and_caps() {
        let registry = AnaplanOperationRegistry::default();
        let request = AnaplanNamedOperationRequest::new(ANAPLAN_FILES_LIST_OPERATION).with_limits(
            AnaplanQueryLimits {
                max_results: 25,
                timeout_ms: 5_000,
                offset: 50,
            },
        );

        let plan = registry
            .build_saas_request_plan(&request, &tenant())
            .expect("SaaS request plan");

        assert_eq!(plan.operation_name, ANAPLAN_FILES_LIST_OPERATION);
        assert_eq!(
            plan.path,
            "/2/0/workspaces/workspace123/models/model456/files"
        );
        assert_eq!(plan.query.get("sort").map(String::as_str), Some("name"));
        assert_eq!(plan.query.get("limit").map(String::as_str), Some("25"));
        assert_eq!(plan.query.get("offset").map(String::as_str), Some("50"));
        assert_eq!(
            plan.caps,
            SaasResponseCaps {
                max_body_bytes: SaasResponseCaps::DEFAULT_MAX_BODY_BYTES,
                max_rows: 25,
            }
        );
        plan.validate().expect("shared plan accepts file list");
    }

    #[test]
    fn files_list_caps_max_results_at_anaplan_page_limit() {
        let registry = AnaplanOperationRegistry::default();
        let request = AnaplanNamedOperationRequest::new(ANAPLAN_FILES_LIST_OPERATION).with_limits(
            AnaplanQueryLimits {
                max_results: AnaplanQueryLimits::MAX_PAGE_LIMIT + 1,
                timeout_ms: 5_000,
                offset: 0,
            },
        );

        let plan = registry
            .build_saas_request_plan(&request, &tenant())
            .expect("SaaS request plan");

        assert_eq!(plan.query.get("limit").map(String::as_str), Some("1000"));
        assert_eq!(plan.caps.max_rows, AnaplanQueryLimits::MAX_PAGE_LIMIT);
    }

    #[test]
    fn view_read_request_operations_need_registry_bound_view_ids() {
        let registry = AnaplanOperationRegistry::default();
        let request = AnaplanNamedOperationRequest::new(ANAPLAN_VIEW_CREATE_READ_REQUEST_OPERATION);

        match registry
            .build_operation_plan(&request, &tenant())
            .unwrap_err()
        {
            AnaplanProviderError::UnsupportedOperation { name, reason } => {
                assert_eq!(name, ANAPLAN_VIEW_CREATE_READ_REQUEST_OPERATION);
                assert!(reason.contains("tenant-approved"));
            }
            other => panic!("expected unsupported operation, got {other:?}"),
        }
    }

    #[test]
    fn registry_bound_view_read_request_converts_to_saas_post_plan() {
        let registry = AnaplanOperationRegistry::new([
            RegisteredAnaplanOperation::view_create_read_request_for_view(
                VIEW_CREATE_OPERATION,
                "view789",
            )
            .expect("registered view"),
        ]);
        let request = AnaplanNamedOperationRequest::new(VIEW_CREATE_OPERATION);

        let plan = registry
            .build_saas_request_plan(&request, &tenant())
            .expect("SaaS request plan");

        assert_eq!(plan.operation_name, VIEW_CREATE_OPERATION);
        assert_eq!(plan.method, SaasHttpMethod::Post);
        assert_eq!(
            plan.path,
            "/2/0/workspaces/workspace123/models/model456/views/view789/readRequests/"
        );
        assert!(plan.query.is_empty());
        assert_eq!(plan.caps.max_rows, 1);
        plan.validate().expect("shared plan accepts read request");
    }

    #[test]
    fn registry_bound_view_read_page_uses_typed_continuation() {
        let registry = AnaplanOperationRegistry::new([
            RegisteredAnaplanOperation::view_get_read_page_for_view(
                VIEW_READ_PAGE_OPERATION,
                "view789",
            )
            .expect("registered view"),
        ]);
        let request = AnaplanNamedOperationRequest::new(VIEW_READ_PAGE_OPERATION)
            .with_parameter(
                "read_request",
                AnaplanParameterValue::ReadRequestPage(
                    ReadRequestPageContinuation::new("request123", 3).with_page_size(75),
                ),
            )
            .with_limits(AnaplanQueryLimits {
                max_results: 25,
                timeout_ms: 3_000,
                offset: 0,
            });

        let plan = registry
            .build_saas_request_plan(&request, &tenant())
            .expect("SaaS request plan");

        assert_eq!(plan.operation_name, VIEW_READ_PAGE_OPERATION);
        assert_eq!(plan.method, SaasHttpMethod::Get);
        assert_eq!(
            plan.path,
            "/2/0/workspaces/workspace123/models/model456/views/view789/readRequests/request123/pages/3"
        );
        assert_eq!(plan.caps.max_rows, 75);
        plan.validate().expect("shared plan accepts read page");
    }

    #[test]
    fn registry_bound_file_chunk_uses_typed_chunk_continuation() {
        let registry = AnaplanOperationRegistry::new([
            RegisteredAnaplanOperation::file_download_chunk_for_file(
                FILE_CHUNK_OPERATION,
                "file123",
            )
            .expect("registered file"),
        ]);
        let request = AnaplanNamedOperationRequest::new(FILE_CHUNK_OPERATION).with_parameter(
            "chunk",
            AnaplanParameterValue::ChunkDownload(
                ChunkDownloadContinuation::new("file123", 2).with_total_chunks(4),
            ),
        );

        let plan = registry
            .build_saas_request_plan(&request, &tenant())
            .expect("SaaS request plan");

        assert_eq!(plan.operation_name, FILE_CHUNK_OPERATION);
        assert_eq!(plan.method, SaasHttpMethod::Get);
        assert_eq!(
            plan.path,
            "/2/0/workspaces/workspace123/models/model456/files/file123/chunks/2"
        );
        assert_eq!(plan.caps.max_rows, 1);
        plan.validate().expect("shared plan accepts chunk read");
    }

    #[test]
    fn file_chunk_rejects_mismatched_chunk_set_id() {
        let registry = AnaplanOperationRegistry::new([
            RegisteredAnaplanOperation::file_download_chunk_for_file(
                FILE_CHUNK_OPERATION,
                "file123",
            )
            .expect("registered file"),
        ]);
        let request = AnaplanNamedOperationRequest::new(FILE_CHUNK_OPERATION).with_parameter(
            "chunk",
            AnaplanParameterValue::ChunkDownload(ChunkDownloadContinuation::new("other-file", 0)),
        );

        assert_eq!(
            registry
                .build_saas_request_plan(&request, &tenant())
                .unwrap_err(),
            AnaplanProviderError::InvalidParameter {
                name: "chunk",
                reason: "chunk continuation must belong to the registry-bound file id",
            }
        );
    }

    #[test]
    fn unknown_operation_is_rejected() {
        let registry = AnaplanOperationRegistry::default();
        let request = AnaplanNamedOperationRequest::new("anaplan.raw.path");

        assert_eq!(
            registry
                .build_saas_request_plan(&request, &tenant())
                .unwrap_err(),
            AnaplanProviderError::UnknownOperation("anaplan.raw.path".to_string())
        );
    }
}
