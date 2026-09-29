use std::collections::BTreeMap;

use appfw_saas_core::{OffsetLimitContinuation, SaasRequestPlan};

use crate::{
    oracle_financials_response_caps, OracleFinancialsDataSensitivity,
    OracleFinancialsNamedOperationRequest, OracleFinancialsOperationAvailability,
    OracleFinancialsOperationGate, OracleFinancialsOperationParameter,
    OracleFinancialsOperationPrimitive, OracleFinancialsParameterValue,
    OracleFinancialsProviderError, OracleFinancialsQueryLimits, OracleFinancialsRestMethod,
    OracleFinancialsRestRequestPlan, OracleFinancialsTenantBinding, ORACLE_FINANCIALS_LIMIT_QUERY,
    ORACLE_FINANCIALS_LINKS_QUERY, ORACLE_FINANCIALS_OFFSET_QUERY,
    ORACLE_FINANCIALS_ONLY_DATA_QUERY, ORACLE_FINANCIALS_REST_RESOURCES_BASE_PATH,
    ORACLE_FINANCIALS_TOTAL_RESULTS_QUERY,
};

pub const ORACLE_FINANCIALS_OPENAPI_DESCRIBE_RESOURCE_OPERATION: &str =
    "oracle.financials.openapi.describe_resource";
pub const ORACLE_FINANCIALS_COLLECTION_QUERY_OPERATION: &str = "oracle.financials.collection_query";
pub const ORACLE_FINANCIALS_FETCH_BY_ID_OPERATION: &str = "oracle.financials.fetch_by_id";
pub const ORACLE_FINANCIALS_LIST_LOV_OPERATION: &str = "oracle.financials.list_lov";
pub const ORACLE_FINANCIALS_SUBMIT_ERP_INTEGRATION_OPERATION: &str =
    "oracle.financials.submit_erpintegration";
pub const ORACLE_FINANCIALS_ACCOUNTING_PERIOD_STATUS_LOV_LIST_OPERATION: &str =
    "oracle.financials.accounting_period_status_lov.list";
pub const ORACLE_FINANCIALS_ACCOUNTING_PERIOD_STATUS_LOV_GET_OPERATION: &str =
    "oracle.financials.accounting_period_status_lov.get";

const TENANT_ALLOWLIST_GATES: &[OracleFinancialsOperationGate] = &[
    OracleFinancialsOperationGate::TenantOperationAllowList,
    OracleFinancialsOperationGate::LiveCertification,
];
const LIVE_WRITE_GATES: &[OracleFinancialsOperationGate] = &[
    OracleFinancialsOperationGate::GovernedWriteReview,
    OracleFinancialsOperationGate::TenantAuthCertification,
    OracleFinancialsOperationGate::LiveCertification,
];

const PAGE_PARAMETER: &[OracleFinancialsOperationParameter] =
    &[OracleFinancialsOperationParameter {
        name: "page",
        required: false,
        description:
            "typed offset/limit continuation returned by an earlier Oracle collection read",
    }];
const ID_PARAMETER: &[OracleFinancialsOperationParameter] = &[OracleFinancialsOperationParameter {
    name: "id",
    required: true,
    description: "single path id for a registry-bound Oracle resource",
}];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OracleFinancialsOperationKind {
    OpenApiDescribeResource,
    CollectionQuery,
    FetchById,
    ListLov,
    SubmitErpIntegration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisteredOracleFinancialsOperation {
    pub name: &'static str,
    pub kind: OracleFinancialsOperationKind,
    pub primitive: OracleFinancialsOperationPrimitive,
    pub method: OracleFinancialsRestMethod,
    pub fixed_collection_path: Option<&'static str>,
    pub path_id_parameter: Option<&'static str>,
    pub parameters: &'static [OracleFinancialsOperationParameter],
    pub availability: OracleFinancialsOperationAvailability,
    pub gates: &'static [OracleFinancialsOperationGate],
    pub sensitivity: OracleFinancialsDataSensitivity,
    pub description: &'static str,
}

impl RegisteredOracleFinancialsOperation {
    pub fn planned_openapi_describe_resource() -> Self {
        planned_operation(
            ORACLE_FINANCIALS_OPENAPI_DESCRIBE_RESOURCE_OPERATION,
            OracleFinancialsOperationKind::OpenApiDescribeResource,
            OracleFinancialsOperationPrimitive::OpenApiDescribeResource,
            OracleFinancialsRestMethod::Get,
            &[],
            "Resource describe planning requires a selected Oracle OpenAPI operation allow-list.",
        )
    }

    pub fn planned_collection_query() -> Self {
        planned_operation(
            ORACLE_FINANCIALS_COLLECTION_QUERY_OPERATION,
            OracleFinancialsOperationKind::CollectionQuery,
            OracleFinancialsOperationPrimitive::CollectionQuery,
            OracleFinancialsRestMethod::Get,
            PAGE_PARAMETER,
            "Collection query planning requires a selected Oracle OpenAPI operation allow-list.",
        )
    }

    pub fn planned_fetch_by_id() -> Self {
        planned_operation(
            ORACLE_FINANCIALS_FETCH_BY_ID_OPERATION,
            OracleFinancialsOperationKind::FetchById,
            OracleFinancialsOperationPrimitive::FetchById,
            OracleFinancialsRestMethod::Get,
            ID_PARAMETER,
            "Fetch-by-id planning requires a selected Oracle OpenAPI operation allow-list.",
        )
    }

    pub fn planned_list_lov() -> Self {
        planned_operation(
            ORACLE_FINANCIALS_LIST_LOV_OPERATION,
            OracleFinancialsOperationKind::ListLov,
            OracleFinancialsOperationPrimitive::ListLov,
            OracleFinancialsRestMethod::Get,
            PAGE_PARAMETER,
            "LOV reads require a selected Oracle OpenAPI operation allow-list.",
        )
    }

    pub fn planned_submit_erp_integration() -> Self {
        RegisteredOracleFinancialsOperation {
            name: ORACLE_FINANCIALS_SUBMIT_ERP_INTEGRATION_OPERATION,
            kind: OracleFinancialsOperationKind::SubmitErpIntegration,
            primitive: OracleFinancialsOperationPrimitive::SubmitErpIntegration,
            method: OracleFinancialsRestMethod::Post,
            fixed_collection_path: Some("/erpintegrations"),
            path_id_parameter: None,
            parameters: &[],
            availability: OracleFinancialsOperationAvailability::PlannedUnsupported,
            gates: LIVE_WRITE_GATES,
            sensitivity: OracleFinancialsDataSensitivity::FinancialConfidential,
            description: "ERP integration submission remains dry-run metadata until live write certification.",
        }
    }

    pub fn accounting_period_status_lov_list() -> Self {
        Self {
            name: ORACLE_FINANCIALS_ACCOUNTING_PERIOD_STATUS_LOV_LIST_OPERATION,
            kind: OracleFinancialsOperationKind::ListLov,
            primitive: OracleFinancialsOperationPrimitive::ListLov,
            method: OracleFinancialsRestMethod::Get,
            fixed_collection_path: Some("/accountingPeriodStatusLOV"),
            path_id_parameter: None,
            parameters: PAGE_PARAMETER,
            availability: OracleFinancialsOperationAvailability::Executable,
            gates: &[],
            sensitivity: OracleFinancialsDataSensitivity::Operational,
            description: "Fixed accounting period status LOV collection read from the Oracle 26B OpenAPI summary.",
        }
    }

    pub fn accounting_period_status_lov_get() -> Self {
        Self {
            name: ORACLE_FINANCIALS_ACCOUNTING_PERIOD_STATUS_LOV_GET_OPERATION,
            kind: OracleFinancialsOperationKind::FetchById,
            primitive: OracleFinancialsOperationPrimitive::FetchById,
            method: OracleFinancialsRestMethod::Get,
            fixed_collection_path: Some("/accountingPeriodStatusLOV"),
            path_id_parameter: Some("accountingPeriodStatusLOVUniqID"),
            parameters: ID_PARAMETER,
            availability: OracleFinancialsOperationAvailability::Executable,
            gates: &[],
            sensitivity: OracleFinancialsDataSensitivity::Operational,
            description: "Fixed accounting period status LOV fetch by id from the Oracle 26B OpenAPI summary.",
        }
    }

    pub fn collection_query_for_resource(
        name: &'static str,
        collection_path: &'static str,
    ) -> Result<Self, OracleFinancialsProviderError> {
        validate_registry_collection_path(collection_path)?;
        Ok(Self {
            name,
            kind: OracleFinancialsOperationKind::CollectionQuery,
            primitive: OracleFinancialsOperationPrimitive::CollectionQuery,
            method: OracleFinancialsRestMethod::Get,
            fixed_collection_path: Some(collection_path),
            path_id_parameter: None,
            parameters: PAGE_PARAMETER,
            availability: OracleFinancialsOperationAvailability::Executable,
            gates: &[],
            sensitivity: OracleFinancialsDataSensitivity::FinancialConfidential,
            description: "Registry-bound Oracle collection read; callers cannot choose path, q, finder, fields, expand, or orderBy.",
        })
    }

    pub fn fetch_by_id_for_resource(
        name: &'static str,
        collection_path: &'static str,
        path_id_parameter: &'static str,
    ) -> Result<Self, OracleFinancialsProviderError> {
        validate_registry_collection_path(collection_path)?;
        validate_registry_identifier("path_id_parameter", path_id_parameter)?;
        Ok(Self {
            name,
            kind: OracleFinancialsOperationKind::FetchById,
            primitive: OracleFinancialsOperationPrimitive::FetchById,
            method: OracleFinancialsRestMethod::Get,
            fixed_collection_path: Some(collection_path),
            path_id_parameter: Some(path_id_parameter),
            parameters: ID_PARAMETER,
            availability: OracleFinancialsOperationAvailability::Executable,
            gates: &[],
            sensitivity: OracleFinancialsDataSensitivity::FinancialConfidential,
            description:
                "Registry-bound Oracle fetch by id; callers cannot choose path or projection.",
        })
    }

    pub fn is_executable(&self) -> bool {
        self.availability == OracleFinancialsOperationAvailability::Executable
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
    kind: OracleFinancialsOperationKind,
    primitive: OracleFinancialsOperationPrimitive,
    method: OracleFinancialsRestMethod,
    parameters: &'static [OracleFinancialsOperationParameter],
    description: &'static str,
) -> RegisteredOracleFinancialsOperation {
    RegisteredOracleFinancialsOperation {
        name,
        kind,
        primitive,
        method,
        fixed_collection_path: None,
        path_id_parameter: None,
        parameters,
        availability: OracleFinancialsOperationAvailability::PlannedUnsupported,
        gates: TENANT_ALLOWLIST_GATES,
        sensitivity: OracleFinancialsDataSensitivity::FinancialConfidential,
        description,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OracleFinancialsOperationRegistry {
    operations: BTreeMap<&'static str, RegisteredOracleFinancialsOperation>,
}

impl OracleFinancialsOperationRegistry {
    pub fn new(operations: impl IntoIterator<Item = RegisteredOracleFinancialsOperation>) -> Self {
        let operations = operations
            .into_iter()
            .map(|operation| (operation.name, operation))
            .collect();

        Self { operations }
    }

    pub fn default_read_operations() -> Self {
        Self::new([
            RegisteredOracleFinancialsOperation::planned_openapi_describe_resource(),
            RegisteredOracleFinancialsOperation::planned_collection_query(),
            RegisteredOracleFinancialsOperation::planned_fetch_by_id(),
            RegisteredOracleFinancialsOperation::planned_list_lov(),
            RegisteredOracleFinancialsOperation::planned_submit_erp_integration(),
            RegisteredOracleFinancialsOperation::accounting_period_status_lov_list(),
            RegisteredOracleFinancialsOperation::accounting_period_status_lov_get(),
        ])
    }

    pub fn get(&self, name: &str) -> Option<&RegisteredOracleFinancialsOperation> {
        self.operations.get(name)
    }

    pub fn operation_names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.operations.keys().copied()
    }

    pub fn build_operation_plan(
        &self,
        request: &OracleFinancialsNamedOperationRequest,
        tenant_binding: &OracleFinancialsTenantBinding,
    ) -> Result<OracleFinancialsRestRequestPlan, OracleFinancialsProviderError> {
        let operation = self
            .get(&request.name)
            .ok_or_else(|| OracleFinancialsProviderError::UnknownOperation(request.name.clone()))?;

        if !operation.is_executable() {
            return Err(OracleFinancialsProviderError::UnsupportedOperation {
                name: operation.name.to_string(),
                reason: operation.unsupported_reason(),
            });
        }

        match operation.kind {
            OracleFinancialsOperationKind::CollectionQuery
            | OracleFinancialsOperationKind::ListLov => {
                build_collection_plan(operation, request, tenant_binding)
            }
            OracleFinancialsOperationKind::FetchById => {
                build_fetch_by_id_plan(operation, request, tenant_binding)
            }
            OracleFinancialsOperationKind::OpenApiDescribeResource
            | OracleFinancialsOperationKind::SubmitErpIntegration => {
                Err(OracleFinancialsProviderError::UnsupportedOperation {
                    name: operation.name.to_string(),
                    reason: operation.unsupported_reason(),
                })
            }
        }
    }

    pub fn build_saas_request_plan(
        &self,
        request: &OracleFinancialsNamedOperationRequest,
        tenant_binding: &OracleFinancialsTenantBinding,
    ) -> Result<SaasRequestPlan, OracleFinancialsProviderError> {
        self.build_operation_plan(request, tenant_binding)?
            .to_saas_request_plan()
            .map_err(OracleFinancialsProviderError::InvalidSaasRequestPlan)
    }
}

impl Default for OracleFinancialsOperationRegistry {
    fn default() -> Self {
        Self::default_read_operations()
    }
}

fn build_collection_plan(
    operation: &RegisteredOracleFinancialsOperation,
    request: &OracleFinancialsNamedOperationRequest,
    tenant_binding: &OracleFinancialsTenantBinding,
) -> Result<OracleFinancialsRestRequestPlan, OracleFinancialsProviderError> {
    let collection_path = required_collection_path(operation)?;
    let page = page_from_request(request)?;

    Ok(OracleFinancialsRestRequestPlan::new(
        operation.name,
        operation.primitive,
        operation.method,
        format!("{ORACLE_FINANCIALS_REST_RESOURCES_BASE_PATH}{collection_path}"),
        request.limits.timeout_ms,
    )
    .with_oracle_headers(tenant_binding)
    .with_query_parameter(ORACLE_FINANCIALS_ONLY_DATA_QUERY, "true")
    .with_query_parameter(ORACLE_FINANCIALS_LINKS_QUERY, "false")
    .with_query_parameter(ORACLE_FINANCIALS_TOTAL_RESULTS_QUERY, "true")
    .with_query_parameter(ORACLE_FINANCIALS_LIMIT_QUERY, page.limit.to_string())
    .with_query_parameter(ORACLE_FINANCIALS_OFFSET_QUERY, page.offset.to_string())
    .with_caps(oracle_financials_response_caps(page.limit)))
}

fn build_fetch_by_id_plan(
    operation: &RegisteredOracleFinancialsOperation,
    request: &OracleFinancialsNamedOperationRequest,
    tenant_binding: &OracleFinancialsTenantBinding,
) -> Result<OracleFinancialsRestRequestPlan, OracleFinancialsProviderError> {
    let collection_path = required_collection_path(operation)?;
    let id_parameter =
        operation
            .path_id_parameter
            .ok_or(OracleFinancialsProviderError::UnsupportedOperation {
                name: operation.name.to_string(),
                reason: "operation requires a registry-bound path id parameter",
            })?;
    let id = required_id_parameter(request)?;
    validate_path_segment("id", id)?;
    validate_registry_identifier("path_id_parameter", id_parameter)?;

    Ok(OracleFinancialsRestRequestPlan::new(
        operation.name,
        operation.primitive,
        operation.method,
        format!("{ORACLE_FINANCIALS_REST_RESOURCES_BASE_PATH}{collection_path}/{id}"),
        request.limits.timeout_ms,
    )
    .with_oracle_headers(tenant_binding)
    .with_query_parameter(ORACLE_FINANCIALS_ONLY_DATA_QUERY, "true")
    .with_query_parameter(ORACLE_FINANCIALS_LINKS_QUERY, "false")
    .with_caps(oracle_financials_response_caps(1)))
}

fn required_collection_path(
    operation: &RegisteredOracleFinancialsOperation,
) -> Result<&'static str, OracleFinancialsProviderError> {
    operation
        .fixed_collection_path
        .ok_or(OracleFinancialsProviderError::UnsupportedOperation {
            name: operation.name.to_string(),
            reason: "operation requires a registry-bound Oracle collection path",
        })
}

fn page_from_request(
    request: &OracleFinancialsNamedOperationRequest,
) -> Result<OffsetLimitContinuation, OracleFinancialsProviderError> {
    if let Some(value) = request.parameters.get("page") {
        reject_unexpected_parameters_except(request, "page")?;
        let page =
            value
                .as_offset_page()
                .ok_or(OracleFinancialsProviderError::InvalidParameter {
                    name: "page",
                    reason: "must be a typed offset/limit continuation",
                })?;
        page.validate()
            .map_err(OracleFinancialsProviderError::InvalidSaasRequestPlan)?;

        return Ok(OffsetLimitContinuation::new(
            page.offset,
            page.limit.min(OracleFinancialsQueryLimits::MAX_PAGE_LIMIT),
        ));
    }

    reject_unexpected_parameters_except(request, "")?;
    request.limits.default_page()
}

fn required_id_parameter(
    request: &OracleFinancialsNamedOperationRequest,
) -> Result<&str, OracleFinancialsProviderError> {
    reject_unexpected_parameters_except(request, "id")?;
    request
        .parameters
        .get("id")
        .ok_or(OracleFinancialsProviderError::MissingParameter("id"))
        .and_then(|value| match value {
            OracleFinancialsParameterValue::String(id) => Ok(id.as_str()),
            OracleFinancialsParameterValue::OffsetPage(_) => {
                Err(OracleFinancialsProviderError::InvalidParameter {
                    name: "id",
                    reason: "must be a single path id string",
                })
            }
        })
}

fn reject_unexpected_parameters_except(
    request: &OracleFinancialsNamedOperationRequest,
    allowed: &str,
) -> Result<(), OracleFinancialsProviderError> {
    if request
        .parameters
        .keys()
        .any(|name| allowed.is_empty() || name != allowed)
    {
        return Err(OracleFinancialsProviderError::InvalidParameter {
            name: "parameters",
            reason: "operation accepts only its typed allowlisted parameters",
        });
    }

    Ok(())
}

fn validate_registry_collection_path(value: &str) -> Result<(), OracleFinancialsProviderError> {
    if !value.starts_with('/') || value.starts_with("//") || value.contains("://") {
        return Err(OracleFinancialsProviderError::InvalidParameter {
            name: "collection_path",
            reason: "must be an OpenAPI relative collection path starting with /",
        });
    }

    if value.split('/').any(|segment| {
        matches!(segment, "." | "..") || segment.contains('{') || segment.contains('}')
    }) {
        return Err(OracleFinancialsProviderError::InvalidParameter {
            name: "collection_path",
            reason: "must not contain dot segments or caller-bound path templates",
        });
    }

    if value.chars().any(|character| {
        !(character.is_ascii_alphanumeric() || matches!(character, '/' | '-' | '_'))
    }) {
        return Err(OracleFinancialsProviderError::InvalidParameter {
            name: "collection_path",
            reason: "must contain only ASCII path characters used by the OpenAPI resource catalog",
        });
    }

    Ok(())
}

fn validate_registry_identifier(
    name: &'static str,
    value: &str,
) -> Result<(), OracleFinancialsProviderError> {
    if value.trim() != value || value.is_empty() {
        return Err(OracleFinancialsProviderError::InvalidParameter {
            name,
            reason: "must be a non-empty registry-bound identifier",
        });
    }

    if value
        .chars()
        .any(|character| !(character.is_ascii_alphanumeric() || matches!(character, '_' | '-')))
    {
        return Err(OracleFinancialsProviderError::InvalidParameter {
            name,
            reason: "must contain only ASCII letters, digits, hyphen, or underscore",
        });
    }

    Ok(())
}

fn validate_path_segment(
    name: &'static str,
    value: &str,
) -> Result<(), OracleFinancialsProviderError> {
    if value.trim() != value || value.is_empty() {
        return Err(OracleFinancialsProviderError::InvalidParameter {
            name,
            reason: "must be a non-empty path segment",
        });
    }

    if value.chars().any(|character| {
        character.is_ascii_control()
            || character.is_ascii_whitespace()
            || matches!(character, '/' | '?' | '#' | '%' | '&')
    }) || matches!(value, "." | "..")
    {
        return Err(OracleFinancialsProviderError::InvalidParameter {
            name,
            reason: "must be a pre-encoded single path segment without separators",
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use appfw_saas_core::SaasHttpMethod;

    fn tenant() -> OracleFinancialsTenantBinding {
        OracleFinancialsTenantBinding::with_metadata_context(
            "https://tenant.fa.us2.oraclecloud.com",
            "26B",
            Some("sandbox"),
            "ledger:vision-operations",
        )
        .expect("tenant binding")
    }

    #[test]
    fn registry_exposes_stable_operation_names() {
        let registry = OracleFinancialsOperationRegistry::default();
        let names = registry.operation_names().collect::<Vec<_>>();

        assert_eq!(
            names,
            vec![
                ORACLE_FINANCIALS_ACCOUNTING_PERIOD_STATUS_LOV_GET_OPERATION,
                ORACLE_FINANCIALS_ACCOUNTING_PERIOD_STATUS_LOV_LIST_OPERATION,
                ORACLE_FINANCIALS_COLLECTION_QUERY_OPERATION,
                ORACLE_FINANCIALS_FETCH_BY_ID_OPERATION,
                ORACLE_FINANCIALS_LIST_LOV_OPERATION,
                ORACLE_FINANCIALS_OPENAPI_DESCRIBE_RESOURCE_OPERATION,
                ORACLE_FINANCIALS_SUBMIT_ERP_INTEGRATION_OPERATION,
            ]
        );
    }

    #[test]
    fn broad_openapi_operations_are_planned_until_allowlisted() {
        let registry = OracleFinancialsOperationRegistry::default();
        let request = OracleFinancialsNamedOperationRequest::new(
            ORACLE_FINANCIALS_COLLECTION_QUERY_OPERATION,
        );

        let error = registry
            .build_operation_plan(&request, &tenant())
            .unwrap_err();

        assert!(matches!(
            error,
            OracleFinancialsProviderError::UnsupportedOperation { reason, .. }
                if reason.contains("allow-list")
        ));
    }

    #[test]
    fn submit_erp_integration_remains_unsupported_write_planning() {
        let registry = OracleFinancialsOperationRegistry::default();
        let request = OracleFinancialsNamedOperationRequest::new(
            ORACLE_FINANCIALS_SUBMIT_ERP_INTEGRATION_OPERATION,
        );

        let error = registry
            .build_operation_plan(&request, &tenant())
            .unwrap_err();

        assert!(matches!(
            error,
            OracleFinancialsProviderError::UnsupportedOperation { reason, .. }
                if reason.contains("governed write")
        ));
    }

    #[test]
    fn accounting_period_status_lov_list_converts_to_fixed_saas_request_plan() {
        let registry = OracleFinancialsOperationRegistry::default();
        let request = OracleFinancialsNamedOperationRequest::new(
            ORACLE_FINANCIALS_ACCOUNTING_PERIOD_STATUS_LOV_LIST_OPERATION,
        );

        let plan = registry
            .build_saas_request_plan(&request, &tenant())
            .expect("SaaS request plan");

        assert_eq!(
            plan.operation_name,
            ORACLE_FINANCIALS_ACCOUNTING_PERIOD_STATUS_LOV_LIST_OPERATION
        );
        assert_eq!(plan.method, SaasHttpMethod::Get);
        assert_eq!(
            plan.path,
            "/fscmRestApi/resources/11.13.18.05/accountingPeriodStatusLOV"
        );
        assert_eq!(
            plan.query.keys().collect::<Vec<_>>(),
            vec!["limit", "links", "offset", "onlyData", "totalResults"]
        );
        assert_eq!(plan.query.get("onlyData").map(String::as_str), Some("true"));
        assert_eq!(plan.query.get("links").map(String::as_str), Some("false"));
        assert_eq!(plan.query.get("limit").map(String::as_str), Some("100"));
        assert!(!plan.query.contains_key("q"));
        assert!(!plan.query.contains_key("finder"));
        assert!(!plan.query.contains_key("orderBy"));
        assert_eq!(
            plan.headers
                .get(crate::ORACLE_FINANCIALS_REST_FRAMEWORK_VERSION_HEADER)
                .map(String::as_str),
            Some("26B")
        );
        assert_eq!(
            plan.headers
                .get(crate::ORACLE_FINANCIALS_METADATA_CONTEXT_HEADER)
                .map(String::as_str),
            Some("sandbox")
        );
        assert_eq!(plan.caps.max_rows, 100);
    }

    #[test]
    fn accounting_period_status_lov_list_accepts_typed_offset_continuation() {
        let registry = OracleFinancialsOperationRegistry::default();
        let request = OracleFinancialsNamedOperationRequest::new(
            ORACLE_FINANCIALS_ACCOUNTING_PERIOD_STATUS_LOV_LIST_OPERATION,
        )
        .with_parameter(
            "page",
            OracleFinancialsParameterValue::OffsetPage(OffsetLimitContinuation::new(200, 25)),
        );

        let plan = registry
            .build_saas_request_plan(&request, &tenant())
            .expect("SaaS request plan");

        assert_eq!(plan.query.get("offset").map(String::as_str), Some("200"));
        assert_eq!(plan.query.get("limit").map(String::as_str), Some("25"));
        assert_eq!(plan.caps.max_rows, 25);
    }

    #[test]
    fn accounting_period_status_lov_list_caps_page_limit() {
        let registry = OracleFinancialsOperationRegistry::default();
        let request = OracleFinancialsNamedOperationRequest::new(
            ORACLE_FINANCIALS_ACCOUNTING_PERIOD_STATUS_LOV_LIST_OPERATION,
        )
        .with_limits(OracleFinancialsQueryLimits {
            max_results: 5_000,
            timeout_ms: 4_000,
            offset: 10,
        });

        let plan = registry
            .build_saas_request_plan(&request, &tenant())
            .expect("SaaS request plan");

        assert_eq!(plan.query.get("limit").map(String::as_str), Some("500"));
        assert_eq!(plan.query.get("offset").map(String::as_str), Some("10"));
        assert_eq!(plan.caps.max_rows, 500);
    }

    #[test]
    fn accounting_period_status_lov_get_requires_single_id_path_segment() {
        let registry = OracleFinancialsOperationRegistry::default();
        let request = OracleFinancialsNamedOperationRequest::new(
            ORACLE_FINANCIALS_ACCOUNTING_PERIOD_STATUS_LOV_GET_OPERATION,
        )
        .with_parameter(
            "id",
            OracleFinancialsParameterValue::String("OPEN".to_string()),
        );

        let plan = registry
            .build_saas_request_plan(&request, &tenant())
            .expect("SaaS request plan");

        assert_eq!(
            plan.path,
            "/fscmRestApi/resources/11.13.18.05/accountingPeriodStatusLOV/OPEN"
        );
        assert_eq!(
            plan.query.keys().collect::<Vec<_>>(),
            vec!["links", "onlyData"]
        );
        assert_eq!(plan.caps.max_rows, 1);
    }

    #[test]
    fn accounting_period_status_lov_get_rejects_raw_path_injection() {
        let registry = OracleFinancialsOperationRegistry::default();
        let request = OracleFinancialsNamedOperationRequest::new(
            ORACLE_FINANCIALS_ACCOUNTING_PERIOD_STATUS_LOV_GET_OPERATION,
        )
        .with_parameter(
            "id",
            OracleFinancialsParameterValue::String("../payablesInvoices".to_string()),
        );

        let error = registry
            .build_saas_request_plan(&request, &tenant())
            .unwrap_err();

        assert!(matches!(
            error,
            OracleFinancialsProviderError::InvalidParameter { name: "id", .. }
        ));
    }

    #[test]
    fn registry_bound_custom_collection_can_be_added_without_exposing_raw_paths() {
        let registry = OracleFinancialsOperationRegistry::new([
            RegisteredOracleFinancialsOperation::collection_query_for_resource(
                "oracle.financials.payables_distribution_sets.list",
                "/payablesDistributionSets",
            )
            .expect("registered operation"),
        ]);
        let request = OracleFinancialsNamedOperationRequest::new(
            "oracle.financials.payables_distribution_sets.list",
        );

        let plan = registry
            .build_saas_request_plan(&request, &tenant())
            .expect("SaaS request plan");

        assert_eq!(
            plan.path,
            "/fscmRestApi/resources/11.13.18.05/payablesDistributionSets"
        );
    }

    #[test]
    fn registry_rejects_unbound_resource_templates() {
        let error = RegisteredOracleFinancialsOperation::collection_query_for_resource(
            "oracle.financials.bad.list",
            "/payablesInvoices/{InvoiceId}",
        )
        .unwrap_err();

        assert!(matches!(
            error,
            OracleFinancialsProviderError::InvalidParameter {
                name: "collection_path",
                ..
            }
        ));
    }

    #[test]
    fn unknown_operation_is_rejected() {
        let registry = OracleFinancialsOperationRegistry::default();
        let request = OracleFinancialsNamedOperationRequest::new("oracle.financials.raw_url");

        assert_eq!(
            registry
                .build_saas_request_plan(&request, &tenant())
                .unwrap_err(),
            OracleFinancialsProviderError::UnknownOperation(
                "oracle.financials.raw_url".to_string()
            )
        );
    }
}
