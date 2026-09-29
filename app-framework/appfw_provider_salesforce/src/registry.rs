use std::collections::BTreeMap;

use appfw_saas_core::SaasRequestPlan;

use crate::{
    response::{SalesforceNextRecordsUrl, SALESFORCE_NEXT_RECORDS_URL_FIELD},
    SalesforceDataSensitivity, SalesforceField, SalesforceFilter, SalesforceLiteral,
    SalesforceNamedOperationRequest, SalesforceObject, SalesforceOperationAvailability,
    SalesforceOperationGate, SalesforceOperationParameter, SalesforceOperationPlan,
    SalesforceOperationPrimitive, SalesforceOperator, SalesforceParameterValue,
    SalesforceProviderError, SalesforceQueryPlan, SalesforceRestMethod, SalesforceRestRequestPlan,
    SalesforceSoqlQuery, SalesforceSort, SalesforceWatermarkRequest, SALESFORCE_REST_BASE_PATH,
};

pub const ACCOUNT_DESCRIBE_OBJECT_OPERATION: &str = "salesforce.account.describe_object";
pub const ACCOUNT_BY_ID_OPERATION: &str = "salesforce.account_by_id";
pub const ACCOUNT_FETCH_BY_IDS_OPERATION: &str = "salesforce.account.fetch_by_ids";
pub const ACCOUNT_GET_UPDATED_IDS_OPERATION: &str = "salesforce.account.get_updated_ids";
pub const ACCOUNT_GET_DELETED_IDS_OPERATION: &str = "salesforce.account.get_deleted_ids";
pub const SALESFORCE_CONTINUE_QUERY_PAGE_OPERATION: &str = "salesforce.query.continue_page";
pub const UPDATED_ACCOUNTS_OPERATION: &str = "salesforce.updated_accounts";
pub const SALESFORCE_HEALTH_DESCRIBE_OBJECT_OPERATION: &str = "salesforce.health.describe_object";
pub const SALESFORCE_HEALTH_FETCH_LIMITS_OPERATION: &str = "salesforce.health.fetch_limits";
pub const SALESFORCE_HEALTH_GET_UPDATED_IDS_OPERATION: &str = "salesforce.health.get_updated_ids";
pub const SALESFORCE_HEALTH_GET_DELETED_IDS_OPERATION: &str = "salesforce.health.get_deleted_ids";
pub const SALESFORCE_HEALTH_FETCH_BY_IDS_OPERATION: &str = "salesforce.health.fetch_by_ids";
pub const SALESFORCE_HEALTH_QUERY_OBJECT_INCREMENTAL_OPERATION: &str =
    "salesforce.health.query_object_incremental";
pub const HEALTH_CLOUD_CARE_PROGRAMS_UPDATED_SINCE_OPERATION: &str =
    "salesforce.health.care_programs_updated_since";
pub const HEALTH_CLOUD_CARE_PROGRAM_ENROLLEES_BY_PROGRAM_OPERATION: &str =
    "salesforce.health.care_program_enrollees_by_program";
pub const HEALTH_CLOUD_COVERAGE_BENEFIT_ITEMS_BY_MEMBER_OPERATION: &str =
    "salesforce.health.coverage_benefit_items_by_member";
pub const HEALTH_CLOUD_CLINICAL_ENCOUNTERS_UPDATED_SINCE_OPERATION: &str =
    "salesforce.health.clinical_encounters_updated_since";

const LIVE_HEALTH_CLOUD_GATES: &[SalesforceOperationGate] = &[
    SalesforceOperationGate::LiveOrgDescribe,
    SalesforceOperationGate::GovernanceReview,
    SalesforceOperationGate::LiveCertification,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SalesforceOperationKind {
    AccountById,
    AccountDescribeObject,
    AccountFetchByIds,
    AccountGetUpdatedIds,
    AccountGetDeletedIds,
    ContinueQueryPage,
    FetchLimits,
    PlannedUnsupported,
    UpdatedAccounts,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisteredSalesforceOperation {
    pub name: &'static str,
    pub kind: SalesforceOperationKind,
    pub primitive: SalesforceOperationPrimitive,
    pub object: Option<SalesforceObject>,
    pub selected_fields: &'static [SalesforceField],
    pub parameters: &'static [SalesforceOperationParameter],
    pub watermark_field: Option<SalesforceField>,
    pub availability: SalesforceOperationAvailability,
    pub gates: &'static [SalesforceOperationGate],
    pub sensitivity: SalesforceDataSensitivity,
    pub description: &'static str,
}

impl RegisteredSalesforceOperation {
    pub fn account_by_id() -> Self {
        Self {
            name: ACCOUNT_BY_ID_OPERATION,
            kind: SalesforceOperationKind::AccountById,
            primitive: SalesforceOperationPrimitive::FetchByIds,
            object: Some(SalesforceObject::Account),
            selected_fields: &[
                SalesforceField::AccountId,
                SalesforceField::AccountName,
                SalesforceField::AccountNumber,
                SalesforceField::AccountSystemModstamp,
            ],
            parameters: &[SalesforceOperationParameter {
                name: "id",
                required: true,
                description: "15 or 18 character Salesforce Account id",
            }],
            watermark_field: None,
            availability: SalesforceOperationAvailability::Executable,
            gates: &[],
            sensitivity: SalesforceDataSensitivity::BusinessConfidential,
            description:
                "Fixed Account projection by id; callers cannot choose SOQL, object, or fields.",
        }
    }

    pub fn account_describe_object() -> Self {
        Self {
            name: ACCOUNT_DESCRIBE_OBJECT_OPERATION,
            kind: SalesforceOperationKind::AccountDescribeObject,
            primitive: SalesforceOperationPrimitive::DescribeObject,
            object: Some(SalesforceObject::Account),
            selected_fields: &[],
            parameters: &[],
            watermark_field: None,
            availability: SalesforceOperationAvailability::Executable,
            gates: &[],
            sensitivity: SalesforceDataSensitivity::Operational,
            description: "Fixed Account describe request used for drift checks.",
        }
    }

    pub fn account_fetch_by_ids() -> Self {
        Self {
            name: ACCOUNT_FETCH_BY_IDS_OPERATION,
            kind: SalesforceOperationKind::AccountFetchByIds,
            primitive: SalesforceOperationPrimitive::FetchByIds,
            object: Some(SalesforceObject::Account),
            selected_fields: &[
                SalesforceField::AccountId,
                SalesforceField::AccountName,
                SalesforceField::AccountNumber,
                SalesforceField::AccountSystemModstamp,
                SalesforceField::AccountLastModifiedDate,
                SalesforceField::AccountIsDeleted,
            ],
            parameters: &[SalesforceOperationParameter {
                name: "ids",
                required: true,
                description: "bounded list of 15 or 18 character Salesforce Account ids",
            }],
            watermark_field: None,
            availability: SalesforceOperationAvailability::Executable,
            gates: &[],
            sensitivity: SalesforceDataSensitivity::BusinessConfidential,
            description: "Fixed Account projection for a bounded list of ids.",
        }
    }

    pub fn account_get_updated_ids() -> Self {
        Self {
            name: ACCOUNT_GET_UPDATED_IDS_OPERATION,
            kind: SalesforceOperationKind::AccountGetUpdatedIds,
            primitive: SalesforceOperationPrimitive::GetUpdatedIds,
            object: Some(SalesforceObject::Account),
            selected_fields: &[],
            parameters: &[
                SalesforceOperationParameter {
                    name: "start",
                    required: true,
                    description: "inclusive lower-bound UTC datetime for Salesforce getUpdated",
                },
                SalesforceOperationParameter {
                    name: "end",
                    required: true,
                    description: "exclusive upper-bound UTC datetime for Salesforce getUpdated",
                },
            ],
            watermark_field: Some(SalesforceField::AccountSystemModstamp),
            availability: SalesforceOperationAvailability::Executable,
            gates: &[],
            sensitivity: SalesforceDataSensitivity::BusinessConfidential,
            description: "Fixed Account getUpdated ID-only read; callers cannot choose object, path, or fields.",
        }
    }

    pub fn account_get_deleted_ids() -> Self {
        Self {
            name: ACCOUNT_GET_DELETED_IDS_OPERATION,
            kind: SalesforceOperationKind::AccountGetDeletedIds,
            primitive: SalesforceOperationPrimitive::GetDeletedIds,
            object: Some(SalesforceObject::Account),
            selected_fields: &[],
            parameters: &[
                SalesforceOperationParameter {
                    name: "start",
                    required: true,
                    description: "inclusive lower-bound UTC datetime for Salesforce getDeleted",
                },
                SalesforceOperationParameter {
                    name: "end",
                    required: true,
                    description: "exclusive upper-bound UTC datetime for Salesforce getDeleted",
                },
            ],
            watermark_field: Some(SalesforceField::AccountLastModifiedDate),
            availability: SalesforceOperationAvailability::Executable,
            gates: &[],
            sensitivity: SalesforceDataSensitivity::BusinessConfidential,
            description: "Fixed Account getDeleted ID-only read; callers cannot choose object, path, or fields.",
        }
    }

    pub fn continue_query_page() -> Self {
        Self {
            name: SALESFORCE_CONTINUE_QUERY_PAGE_OPERATION,
            kind: SalesforceOperationKind::ContinueQueryPage,
            primitive: SalesforceOperationPrimitive::ContinueQueryPage,
            object: None,
            selected_fields: &[],
            parameters: &[SalesforceOperationParameter {
                name: SALESFORCE_NEXT_RECORDS_URL_FIELD,
                required: true,
                description: "Salesforce returned relative nextRecordsUrl path for a query page.",
            }],
            watermark_field: None,
            availability: SalesforceOperationAvailability::Executable,
            gates: &[],
            sensitivity: SalesforceDataSensitivity::BusinessConfidential,
            description: "Continue a Salesforce query/queryAll page from a validated returned nextRecordsUrl.",
        }
    }

    pub fn updated_accounts() -> Self {
        Self {
            name: UPDATED_ACCOUNTS_OPERATION,
            kind: SalesforceOperationKind::UpdatedAccounts,
            primitive: SalesforceOperationPrimitive::QueryObjectIncremental,
            object: Some(SalesforceObject::Account),
            selected_fields: &[
                SalesforceField::AccountId,
                SalesforceField::AccountName,
                SalesforceField::AccountNumber,
                SalesforceField::AccountSystemModstamp,
                SalesforceField::AccountLastModifiedDate,
                SalesforceField::AccountIsDeleted,
            ],
            parameters: &[
                SalesforceOperationParameter {
                    name: "since",
                    required: true,
                    description: "exclusive lower-bound ISO-8601 SystemModstamp watermark",
                },
                SalesforceOperationParameter {
                    name: "cursor",
                    required: false,
                    description: "opaque continuation token returned by the runtime executor",
                },
            ],
            watermark_field: Some(SalesforceField::AccountSystemModstamp),
            availability: SalesforceOperationAvailability::Executable,
            gates: &[],
            sensitivity: SalesforceDataSensitivity::BusinessConfidential,
            description: "Fixed Account incremental query using SystemModstamp and Id ordering.",
        }
    }

    pub fn fetch_limits() -> Self {
        Self {
            name: SALESFORCE_HEALTH_FETCH_LIMITS_OPERATION,
            kind: SalesforceOperationKind::FetchLimits,
            primitive: SalesforceOperationPrimitive::FetchLimits,
            object: None,
            selected_fields: &[],
            parameters: &[],
            watermark_field: None,
            availability: SalesforceOperationAvailability::Executable,
            gates: &[],
            sensitivity: SalesforceDataSensitivity::Operational,
            description: "GET Salesforce REST org limits for smoke tests and rate visibility.",
        }
    }

    pub fn planned_health_describe_object() -> Self {
        planned_health_primitive(
            SALESFORCE_HEALTH_DESCRIBE_OBJECT_OPERATION,
            SalesforceOperationPrimitive::DescribeObject,
            None,
            "Generic Health Cloud describe remains gated until fixed object projections are certified from org describe.",
        )
    }

    pub fn planned_health_get_updated_ids() -> Self {
        planned_health_primitive(
            SALESFORCE_HEALTH_GET_UPDATED_IDS_OPERATION,
            SalesforceOperationPrimitive::GetUpdatedIds,
            None,
            "Health Cloud getUpdated id sync needs object support from authenticated org describe.",
        )
    }

    pub fn planned_health_get_deleted_ids() -> Self {
        planned_health_primitive(
            SALESFORCE_HEALTH_GET_DELETED_IDS_OPERATION,
            SalesforceOperationPrimitive::GetDeletedIds,
            None,
            "Health Cloud getDeleted id sync needs object support from authenticated org describe.",
        )
    }

    pub fn planned_health_fetch_by_ids() -> Self {
        planned_health_primitive(
            SALESFORCE_HEALTH_FETCH_BY_IDS_OPERATION,
            SalesforceOperationPrimitive::FetchByIds,
            None,
            "Generic Health Cloud fetch-by-ids needs fixed object and field projections from org describe.",
        )
    }

    pub fn planned_health_query_object_incremental() -> Self {
        planned_health_primitive(
            SALESFORCE_HEALTH_QUERY_OBJECT_INCREMENTAL_OPERATION,
            SalesforceOperationPrimitive::QueryObjectIncremental,
            None,
            "Generic Health Cloud incremental query would expose object/field choice without certified projections.",
        )
    }

    pub fn planned_health_care_programs_updated_since() -> Self {
        planned_health_read_candidate(
            HEALTH_CLOUD_CARE_PROGRAMS_UPDATED_SINCE_OPERATION,
            SalesforceObject::CareProgram,
            "CareProgram updated-since projection candidate from the local Health Cloud evidence.",
        )
    }

    pub fn planned_health_care_program_enrollees_by_program() -> Self {
        planned_health_read_candidate(
            HEALTH_CLOUD_CARE_PROGRAM_ENROLLEES_BY_PROGRAM_OPERATION,
            SalesforceObject::CareProgramEnrollee,
            "CareProgramEnrollee by-program projection candidate from the local Health Cloud evidence.",
        )
    }

    pub fn planned_health_coverage_benefit_items_by_member() -> Self {
        planned_health_read_candidate(
            HEALTH_CLOUD_COVERAGE_BENEFIT_ITEMS_BY_MEMBER_OPERATION,
            SalesforceObject::CoverageBenefitItem,
            "CoverageBenefitItem by-member projection candidate from the local Health Cloud evidence.",
        )
    }

    pub fn planned_health_clinical_encounters_updated_since() -> Self {
        planned_health_read_candidate(
            HEALTH_CLOUD_CLINICAL_ENCOUNTERS_UPDATED_SINCE_OPERATION,
            SalesforceObject::ClinicalEncounter,
            "ClinicalEncounter updated-since projection candidate from the local Health Cloud evidence.",
        )
    }

    pub fn is_executable(&self) -> bool {
        self.availability == SalesforceOperationAvailability::Executable
    }

    pub fn unsupported_reason(&self) -> &'static str {
        self.gates
            .first()
            .map(|gate| gate.reason())
            .unwrap_or("operation is planned but not executable")
    }
}

fn planned_health_primitive(
    name: &'static str,
    primitive: SalesforceOperationPrimitive,
    object: Option<SalesforceObject>,
    description: &'static str,
) -> RegisteredSalesforceOperation {
    RegisteredSalesforceOperation {
        name,
        kind: SalesforceOperationKind::PlannedUnsupported,
        primitive,
        object,
        selected_fields: &[],
        parameters: &[],
        watermark_field: None,
        availability: SalesforceOperationAvailability::PlannedUnsupported,
        gates: LIVE_HEALTH_CLOUD_GATES,
        sensitivity: SalesforceDataSensitivity::PhiPii,
        description,
    }
}

fn planned_health_read_candidate(
    name: &'static str,
    object: SalesforceObject,
    description: &'static str,
) -> RegisteredSalesforceOperation {
    RegisteredSalesforceOperation {
        name,
        kind: SalesforceOperationKind::PlannedUnsupported,
        primitive: SalesforceOperationPrimitive::QueryObjectIncremental,
        object: Some(object),
        selected_fields: &[],
        parameters: &[],
        watermark_field: None,
        availability: SalesforceOperationAvailability::PlannedUnsupported,
        gates: LIVE_HEALTH_CLOUD_GATES,
        sensitivity: SalesforceDataSensitivity::PhiPii,
        description,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SalesforceOperationRegistry {
    operations: BTreeMap<&'static str, RegisteredSalesforceOperation>,
}

impl SalesforceOperationRegistry {
    pub fn new(operations: impl IntoIterator<Item = RegisteredSalesforceOperation>) -> Self {
        let operations = operations
            .into_iter()
            .map(|operation| (operation.name, operation))
            .collect();

        Self { operations }
    }

    pub fn default_read_operations() -> Self {
        Self::new([
            RegisteredSalesforceOperation::account_describe_object(),
            RegisteredSalesforceOperation::account_by_id(),
            RegisteredSalesforceOperation::account_fetch_by_ids(),
            RegisteredSalesforceOperation::account_get_deleted_ids(),
            RegisteredSalesforceOperation::account_get_updated_ids(),
            RegisteredSalesforceOperation::continue_query_page(),
            RegisteredSalesforceOperation::updated_accounts(),
            RegisteredSalesforceOperation::fetch_limits(),
            RegisteredSalesforceOperation::planned_health_describe_object(),
            RegisteredSalesforceOperation::planned_health_get_updated_ids(),
            RegisteredSalesforceOperation::planned_health_get_deleted_ids(),
            RegisteredSalesforceOperation::planned_health_fetch_by_ids(),
            RegisteredSalesforceOperation::planned_health_query_object_incremental(),
            RegisteredSalesforceOperation::planned_health_care_programs_updated_since(),
            RegisteredSalesforceOperation::planned_health_care_program_enrollees_by_program(),
            RegisteredSalesforceOperation::planned_health_coverage_benefit_items_by_member(),
            RegisteredSalesforceOperation::planned_health_clinical_encounters_updated_since(),
        ])
    }

    pub fn get(&self, name: &str) -> Option<&RegisteredSalesforceOperation> {
        self.operations.get(name)
    }

    pub fn operation_names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.operations.keys().copied()
    }

    pub fn build_query_plan(
        &self,
        request: &SalesforceNamedOperationRequest,
    ) -> Result<SalesforceQueryPlan, SalesforceProviderError> {
        match self.build_operation_plan(request)? {
            SalesforceOperationPlan::Query(plan) => Ok(plan),
            SalesforceOperationPlan::Rest(_) => {
                Err(SalesforceProviderError::UnsupportedOperation {
                    name: request.name.clone(),
                    reason: "operation plans a REST request, not a SOQL query",
                })
            }
        }
    }

    pub fn build_operation_plan(
        &self,
        request: &SalesforceNamedOperationRequest,
    ) -> Result<SalesforceOperationPlan, SalesforceProviderError> {
        let operation = self
            .get(&request.name)
            .ok_or_else(|| SalesforceProviderError::UnknownOperation(request.name.clone()))?;

        if !operation.is_executable() {
            return Err(SalesforceProviderError::UnsupportedOperation {
                name: operation.name.to_string(),
                reason: operation.unsupported_reason(),
            });
        }

        match operation.kind {
            SalesforceOperationKind::AccountById => {
                build_account_by_id_plan(operation, request).map(SalesforceOperationPlan::Query)
            }
            SalesforceOperationKind::AccountDescribeObject => {
                build_describe_object_plan(operation, request).map(SalesforceOperationPlan::Rest)
            }
            SalesforceOperationKind::AccountFetchByIds => {
                build_fetch_by_ids_plan(operation, request).map(SalesforceOperationPlan::Query)
            }
            SalesforceOperationKind::AccountGetUpdatedIds => {
                build_account_change_ids_plan(operation, request, "updated")
                    .map(SalesforceOperationPlan::Rest)
            }
            SalesforceOperationKind::AccountGetDeletedIds => {
                build_account_change_ids_plan(operation, request, "deleted")
                    .map(SalesforceOperationPlan::Rest)
            }
            SalesforceOperationKind::ContinueQueryPage => {
                build_continue_query_page_plan(operation, request)
                    .map(SalesforceOperationPlan::Rest)
            }
            SalesforceOperationKind::FetchLimits => {
                build_fetch_limits_plan(operation, request).map(SalesforceOperationPlan::Rest)
            }
            SalesforceOperationKind::PlannedUnsupported => {
                Err(SalesforceProviderError::UnsupportedOperation {
                    name: operation.name.to_string(),
                    reason: operation.unsupported_reason(),
                })
            }
            SalesforceOperationKind::UpdatedAccounts => {
                build_updated_accounts_plan(operation, request).map(SalesforceOperationPlan::Query)
            }
        }
    }

    pub fn build_saas_request_plan(
        &self,
        request: &SalesforceNamedOperationRequest,
    ) -> Result<SaasRequestPlan, SalesforceProviderError> {
        self.build_operation_plan(request)?
            .to_saas_request_plan_with_limits(request.limits)
            .map_err(SalesforceProviderError::InvalidSaasRequestPlan)
    }
}

impl Default for SalesforceOperationRegistry {
    fn default() -> Self {
        Self::default_read_operations()
    }
}

fn build_account_by_id_plan(
    operation: &RegisteredSalesforceOperation,
    request: &SalesforceNamedOperationRequest,
) -> Result<SalesforceQueryPlan, SalesforceProviderError> {
    let id = required_string_parameter(request, "id")?;
    let object = required_operation_object(operation)?;
    let soql = SalesforceSoqlQuery::builder(object)
        .select(operation.selected_fields.iter().copied())
        .filter(SalesforceFilter::new(
            SalesforceField::AccountId,
            SalesforceOperator::Eq,
            SalesforceLiteral::Id(id.to_string()),
        )?)
        .limit(1)
        .to_soql()?;

    Ok(SalesforceQueryPlan::new(
        operation.name,
        soql,
        request.limits.timeout_ms,
        None,
    ))
    .map(|plan| plan.with_max_rows(1))
}

fn build_describe_object_plan(
    operation: &RegisteredSalesforceOperation,
    request: &SalesforceNamedOperationRequest,
) -> Result<SalesforceRestRequestPlan, SalesforceProviderError> {
    let object = required_operation_object(operation)?;
    let path = format!(
        "{}/sobjects/{}/describe",
        SALESFORCE_REST_BASE_PATH,
        object.api_name()
    );

    Ok(SalesforceRestRequestPlan::new(
        operation.name,
        operation.primitive,
        SalesforceRestMethod::Get,
        path,
        request.limits.timeout_ms,
    )
    .with_object(object))
}

fn build_fetch_limits_plan(
    operation: &RegisteredSalesforceOperation,
    request: &SalesforceNamedOperationRequest,
) -> Result<SalesforceRestRequestPlan, SalesforceProviderError> {
    Ok(SalesforceRestRequestPlan::new(
        operation.name,
        operation.primitive,
        SalesforceRestMethod::Get,
        format!("{SALESFORCE_REST_BASE_PATH}/limits"),
        request.limits.timeout_ms,
    ))
}

fn build_fetch_by_ids_plan(
    operation: &RegisteredSalesforceOperation,
    request: &SalesforceNamedOperationRequest,
) -> Result<SalesforceQueryPlan, SalesforceProviderError> {
    let ids = required_string_list_parameter(request, "ids")?;
    let max_results = request.limits.effective_max_results()? as usize;

    if ids.len() > max_results {
        return Err(SalesforceProviderError::InvalidParameter {
            name: "ids",
            reason: "must not exceed limits.max_results",
        });
    }

    let object = required_operation_object(operation)?;
    let soql = SalesforceSoqlQuery::builder(object)
        .select(operation.selected_fields.iter().copied())
        .filter(SalesforceFilter::new(
            SalesforceField::AccountId,
            SalesforceOperator::In,
            SalesforceLiteral::IdSet(ids.to_vec()),
        )?)
        .order_by(SalesforceSort::ascending(SalesforceField::AccountId))
        .limit(ids.len() as u32)
        .to_soql()?;

    Ok(SalesforceQueryPlan::new(
        operation.name,
        soql,
        request.limits.timeout_ms,
        None,
    ))
    .map(|plan| plan.with_max_rows(ids.len() as u32))
}

fn build_account_change_ids_plan(
    operation: &RegisteredSalesforceOperation,
    request: &SalesforceNamedOperationRequest,
    endpoint: &'static str,
) -> Result<SalesforceRestRequestPlan, SalesforceProviderError> {
    let start = required_utc_datetime_parameter(request, "start")?;
    let end = required_utc_datetime_parameter(request, "end")?;
    if start > end {
        return Err(SalesforceProviderError::InvalidParameter {
            name: "end",
            reason: "must not sort before start",
        });
    }

    let object = required_operation_object(operation)?;
    let path = format!(
        "{}/sobjects/{}/{}/",
        SALESFORCE_REST_BASE_PATH,
        object.api_name(),
        endpoint
    );

    Ok(SalesforceRestRequestPlan::new(
        operation.name,
        operation.primitive,
        SalesforceRestMethod::Get,
        path,
        request.limits.timeout_ms,
    )
    .with_object(object)
    .with_query_parameter("start", start)
    .with_query_parameter("end", end))
}

fn build_continue_query_page_plan(
    operation: &RegisteredSalesforceOperation,
    request: &SalesforceNamedOperationRequest,
) -> Result<SalesforceRestRequestPlan, SalesforceProviderError> {
    let next_records_url = SalesforceNextRecordsUrl::parse(required_string_parameter(
        request,
        SALESFORCE_NEXT_RECORDS_URL_FIELD,
    )?)?;

    Ok(SalesforceRestRequestPlan::continue_query_page(
        operation.name,
        &next_records_url,
        request.limits.timeout_ms,
    ))
}

fn build_updated_accounts_plan(
    operation: &RegisteredSalesforceOperation,
    request: &SalesforceNamedOperationRequest,
) -> Result<SalesforceQueryPlan, SalesforceProviderError> {
    let since = required_string_parameter(request, "since")?;
    let max_results = request.limits.effective_max_results()?;
    let watermark_field = operation
        .watermark_field
        .ok_or(SalesforceProviderError::InvalidSoql(
            "updated account operation must define a watermark field".to_string(),
        ))?;

    let mut watermark =
        SalesforceWatermarkRequest::exclusive_after(watermark_field, since, max_results);

    if let Some(cursor) = optional_string_parameter(request, "cursor")? {
        watermark = watermark.with_cursor(cursor);
    }

    let object = required_operation_object(operation)?;
    let soql = SalesforceSoqlQuery::builder(object)
        .select(operation.selected_fields.iter().copied())
        .filter(SalesforceFilter::new(
            watermark_field,
            SalesforceOperator::Gt,
            SalesforceLiteral::DateTime(since.to_string()),
        )?)
        .order_by(SalesforceSort::ascending(watermark_field))
        .order_by(SalesforceSort::ascending(SalesforceField::AccountId))
        .limit(max_results)
        .to_soql()?;

    Ok(SalesforceQueryPlan::new(
        operation.name,
        soql,
        request.limits.timeout_ms,
        Some(watermark),
    ))
    .map(|plan| plan.with_max_rows(max_results))
}

fn required_operation_object(
    operation: &RegisteredSalesforceOperation,
) -> Result<SalesforceObject, SalesforceProviderError> {
    operation.object.ok_or(SalesforceProviderError::InvalidSoql(
        "operation must define a fixed Salesforce object".to_string(),
    ))
}

fn required_string_parameter<'a>(
    request: &'a SalesforceNamedOperationRequest,
    name: &'static str,
) -> Result<&'a str, SalesforceProviderError> {
    request
        .parameters
        .get(name)
        .ok_or(SalesforceProviderError::MissingParameter(name))
        .and_then(|value| {
            value
                .as_str()
                .ok_or(SalesforceProviderError::InvalidParameter {
                    name,
                    reason: "must be a string",
                })
        })
}

fn optional_string_parameter<'a>(
    request: &'a SalesforceNamedOperationRequest,
    name: &'static str,
) -> Result<Option<&'a str>, SalesforceProviderError> {
    match request.parameters.get(name) {
        Some(SalesforceParameterValue::String(value)) => Ok(Some(value.as_str())),
        Some(_) => Err(SalesforceProviderError::InvalidParameter {
            name,
            reason: "must be a string",
        }),
        None => Ok(None),
    }
}

fn required_string_list_parameter<'a>(
    request: &'a SalesforceNamedOperationRequest,
    name: &'static str,
) -> Result<&'a [String], SalesforceProviderError> {
    request
        .parameters
        .get(name)
        .ok_or(SalesforceProviderError::MissingParameter(name))
        .and_then(|value| {
            value
                .as_string_list()
                .ok_or(SalesforceProviderError::InvalidParameter {
                    name,
                    reason: "must be a list of strings",
                })
        })
}

fn required_utc_datetime_parameter<'a>(
    request: &'a SalesforceNamedOperationRequest,
    name: &'static str,
) -> Result<&'a str, SalesforceProviderError> {
    let value = required_string_parameter(request, name)?;
    validate_utc_datetime_parameter(name, value)?;
    Ok(value)
}

fn validate_utc_datetime_parameter(
    name: &'static str,
    value: &str,
) -> Result<(), SalesforceProviderError> {
    let bytes = value.as_bytes();
    let has_utc_shape = bytes.len() == 20
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'T'
        && bytes[13] == b':'
        && bytes[16] == b':'
        && bytes[19] == b'Z'
        && bytes.iter().enumerate().all(|(index, byte)| {
            matches!(index, 4 | 7 | 10 | 13 | 16 | 19) || byte.is_ascii_digit()
        });

    if has_utc_shape {
        return Ok(());
    }

    Err(SalesforceProviderError::InvalidParameter {
        name,
        reason: "must be a UTC ISO-8601 datetime literal in YYYY-MM-DDTHH:MM:SSZ form",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SalesforceQueryLimits, SalesforceWatermarkValue};
    use appfw_saas_core::{
        SaasHttpMethod, SaasRequestBody, SaasResponseCaps, SaasTransportProtocol,
    };

    #[test]
    fn registry_exposes_stable_operation_names() {
        let registry = SalesforceOperationRegistry::default();
        let names = registry.operation_names().collect::<Vec<_>>();

        assert_eq!(
            names,
            vec![
                ACCOUNT_DESCRIBE_OBJECT_OPERATION,
                ACCOUNT_FETCH_BY_IDS_OPERATION,
                ACCOUNT_GET_DELETED_IDS_OPERATION,
                ACCOUNT_GET_UPDATED_IDS_OPERATION,
                ACCOUNT_BY_ID_OPERATION,
                HEALTH_CLOUD_CARE_PROGRAM_ENROLLEES_BY_PROGRAM_OPERATION,
                HEALTH_CLOUD_CARE_PROGRAMS_UPDATED_SINCE_OPERATION,
                HEALTH_CLOUD_CLINICAL_ENCOUNTERS_UPDATED_SINCE_OPERATION,
                HEALTH_CLOUD_COVERAGE_BENEFIT_ITEMS_BY_MEMBER_OPERATION,
                SALESFORCE_HEALTH_DESCRIBE_OBJECT_OPERATION,
                SALESFORCE_HEALTH_FETCH_BY_IDS_OPERATION,
                SALESFORCE_HEALTH_FETCH_LIMITS_OPERATION,
                SALESFORCE_HEALTH_GET_DELETED_IDS_OPERATION,
                SALESFORCE_HEALTH_GET_UPDATED_IDS_OPERATION,
                SALESFORCE_HEALTH_QUERY_OBJECT_INCREMENTAL_OPERATION,
                SALESFORCE_CONTINUE_QUERY_PAGE_OPERATION,
                UPDATED_ACCOUNTS_OPERATION,
            ]
        );
    }

    #[test]
    fn account_by_id_operation_builds_bounded_query() {
        let registry = SalesforceOperationRegistry::default();
        let request = SalesforceNamedOperationRequest::new(ACCOUNT_BY_ID_OPERATION).with_parameter(
            "id",
            SalesforceParameterValue::String("001000000000001AAA".to_string()),
        );

        let plan = registry.build_query_plan(&request).expect("query plan");

        assert_eq!(plan.operation_name, ACCOUNT_BY_ID_OPERATION);
        assert_eq!(
            plan.soql,
            "SELECT Id, Name, AccountNumber, SystemModstamp FROM Account WHERE Id = '001000000000001AAA' LIMIT 1"
        );
        assert_eq!(plan.timeout_ms, SalesforceQueryLimits::DEFAULT_TIMEOUT_MS);
        assert_eq!(plan.watermark, None);
    }

    #[test]
    fn account_by_id_operation_converts_to_saas_query_request_plan() {
        let registry = SalesforceOperationRegistry::default();
        let request = SalesforceNamedOperationRequest::new(ACCOUNT_BY_ID_OPERATION).with_parameter(
            "id",
            SalesforceParameterValue::String("001000000000001AAA".to_string()),
        );

        let saas_plan = registry
            .build_saas_request_plan(&request)
            .expect("SaaS request plan");

        assert_eq!(saas_plan.operation_name, ACCOUNT_BY_ID_OPERATION);
        assert_eq!(saas_plan.protocol, SaasTransportProtocol::RestJson);
        assert_eq!(saas_plan.method, SaasHttpMethod::Get);
        assert_eq!(saas_plan.path, "/services/data/v67.0/query");
        assert_eq!(
            saas_plan.query.get("q").map(String::as_str),
            Some(
                "SELECT Id, Name, AccountNumber, SystemModstamp FROM Account WHERE Id = '001000000000001AAA' LIMIT 1"
            )
        );
        assert!(saas_plan.headers.is_empty());
        assert_eq!(saas_plan.body, SaasRequestBody::Empty);
        assert_eq!(
            saas_plan.timeout_ms,
            SalesforceQueryLimits::DEFAULT_TIMEOUT_MS
        );
        assert_eq!(
            saas_plan.caps,
            SaasResponseCaps {
                max_body_bytes: SaasResponseCaps::DEFAULT_MAX_BODY_BYTES,
                max_rows: 1,
            }
        );
        saas_plan
            .validate()
            .expect("shared plan accepts account query");
    }

    #[test]
    fn updated_accounts_operation_builds_watermark_query() {
        let registry = SalesforceOperationRegistry::default();
        let request = SalesforceNamedOperationRequest::new(UPDATED_ACCOUNTS_OPERATION)
            .with_parameter(
                "since",
                SalesforceParameterValue::String("2026-06-26T00:00:00Z".to_string()),
            )
            .with_parameter(
                "cursor",
                SalesforceParameterValue::String("next-page".to_string()),
            )
            .with_limits(SalesforceQueryLimits {
                max_results: 50,
                timeout_ms: 5_000,
            });

        let plan = registry.build_query_plan(&request).expect("query plan");

        assert_eq!(plan.operation_name, UPDATED_ACCOUNTS_OPERATION);
        assert_eq!(
            plan.soql,
            "SELECT Id, Name, AccountNumber, SystemModstamp, LastModifiedDate, IsDeleted FROM Account WHERE SystemModstamp > 2026-06-26T00:00:00Z ORDER BY SystemModstamp ASC, Id ASC LIMIT 50"
        );
        assert_eq!(plan.timeout_ms, 5_000);

        let watermark = plan.watermark.expect("watermark");
        assert_eq!(watermark.field, SalesforceField::AccountSystemModstamp);
        assert_eq!(
            watermark.value,
            SalesforceWatermarkValue::DateTime("2026-06-26T00:00:00Z".to_string())
        );
        assert!(!watermark.inclusive);
        assert_eq!(watermark.max_results, 50);
        assert_eq!(watermark.cursor.as_deref(), Some("next-page"));
    }

    #[test]
    fn updated_accounts_operation_converts_to_saas_query_request_plan() {
        let registry = SalesforceOperationRegistry::default();
        let request = SalesforceNamedOperationRequest::new(UPDATED_ACCOUNTS_OPERATION)
            .with_parameter(
                "since",
                SalesforceParameterValue::String("2026-06-26T00:00:00Z".to_string()),
            )
            .with_limits(SalesforceQueryLimits {
                max_results: 50,
                timeout_ms: 5_000,
            });

        let saas_plan = registry
            .build_saas_request_plan(&request)
            .expect("SaaS request plan");

        assert_eq!(saas_plan.operation_name, UPDATED_ACCOUNTS_OPERATION);
        assert_eq!(saas_plan.protocol, SaasTransportProtocol::RestJson);
        assert_eq!(saas_plan.method, SaasHttpMethod::Get);
        assert_eq!(saas_plan.path, "/services/data/v67.0/query");
        assert_eq!(
            saas_plan.query.get("q").map(String::as_str),
            Some(
                "SELECT Id, Name, AccountNumber, SystemModstamp, LastModifiedDate, IsDeleted FROM Account WHERE SystemModstamp > 2026-06-26T00:00:00Z ORDER BY SystemModstamp ASC, Id ASC LIMIT 50"
            )
        );
        assert_eq!(saas_plan.timeout_ms, 5_000);
        assert_eq!(
            saas_plan.caps,
            SaasResponseCaps {
                max_body_bytes: SaasResponseCaps::DEFAULT_MAX_BODY_BYTES,
                max_rows: 50,
            }
        );
        assert_eq!(saas_plan.body, SaasRequestBody::Empty);
        saas_plan
            .validate()
            .expect("shared plan accepts incremental query");
    }

    #[test]
    fn updated_accounts_saas_plan_caps_max_results_at_salesforce_soql_limit() {
        let registry = SalesforceOperationRegistry::default();
        let request = SalesforceNamedOperationRequest::new(UPDATED_ACCOUNTS_OPERATION)
            .with_parameter(
                "since",
                SalesforceParameterValue::String("2026-06-26T00:00:00Z".to_string()),
            )
            .with_limits(SalesforceQueryLimits {
                max_results: SalesforceQueryLimits::SOQL_MAX_LIMIT + 1,
                timeout_ms: 5_000,
            });

        let saas_plan = registry
            .build_saas_request_plan(&request)
            .expect("SaaS request plan");

        assert_eq!(
            saas_plan.caps.max_rows,
            SalesforceQueryLimits::SOQL_MAX_LIMIT
        );
        assert!(saas_plan
            .query
            .get("q")
            .expect("SOQL query")
            .ends_with("LIMIT 2000"));
    }

    #[test]
    fn zero_timeout_surfaces_as_invalid_saas_request_plan() {
        let registry = SalesforceOperationRegistry::default();
        let request = SalesforceNamedOperationRequest::new(UPDATED_ACCOUNTS_OPERATION)
            .with_parameter(
                "since",
                SalesforceParameterValue::String("2026-06-26T00:00:00Z".to_string()),
            )
            .with_limits(SalesforceQueryLimits {
                max_results: 50,
                timeout_ms: 0,
            });

        match registry.build_saas_request_plan(&request).unwrap_err() {
            SalesforceProviderError::InvalidSaasRequestPlan(reason) => {
                assert!(reason.contains("timeout_ms"));
            }
            other => panic!("expected shared SaaS request plan validation error, got {other:?}"),
        }
    }

    #[test]
    fn account_describe_operation_builds_fixed_rest_plan() {
        let registry = SalesforceOperationRegistry::default();
        let request = SalesforceNamedOperationRequest::new(ACCOUNT_DESCRIBE_OBJECT_OPERATION);

        let plan = registry
            .build_operation_plan(&request)
            .expect("operation plan");

        assert_eq!(
            plan,
            SalesforceOperationPlan::Rest(
                SalesforceRestRequestPlan::new(
                    ACCOUNT_DESCRIBE_OBJECT_OPERATION,
                    SalesforceOperationPrimitive::DescribeObject,
                    SalesforceRestMethod::Get,
                    "/services/data/v67.0/sobjects/Account/describe",
                    SalesforceQueryLimits::DEFAULT_TIMEOUT_MS,
                )
                .with_object(SalesforceObject::Account)
            )
        );
    }

    #[test]
    fn fetch_limits_operation_builds_fixed_rest_plan() {
        let registry = SalesforceOperationRegistry::default();
        let request =
            SalesforceNamedOperationRequest::new(SALESFORCE_HEALTH_FETCH_LIMITS_OPERATION)
                .with_limits(SalesforceQueryLimits {
                    max_results: 1,
                    timeout_ms: 750,
                });

        let plan = registry
            .build_operation_plan(&request)
            .expect("operation plan");

        assert_eq!(
            plan,
            SalesforceOperationPlan::Rest(SalesforceRestRequestPlan::new(
                SALESFORCE_HEALTH_FETCH_LIMITS_OPERATION,
                SalesforceOperationPrimitive::FetchLimits,
                SalesforceRestMethod::Get,
                "/services/data/v67.0/limits",
                750,
            ))
        );
    }

    #[test]
    fn continue_query_page_builds_validated_next_records_rest_plan() {
        let registry = SalesforceOperationRegistry::default();
        let request =
            SalesforceNamedOperationRequest::new(SALESFORCE_CONTINUE_QUERY_PAGE_OPERATION)
                .with_parameter(
                    SALESFORCE_NEXT_RECORDS_URL_FIELD,
                    SalesforceParameterValue::String(
                        "/services/data/v67.0/query/01gD0000002HU6KIAW-2000".to_string(),
                    ),
                )
                .with_limits(SalesforceQueryLimits {
                    max_results: 250,
                    timeout_ms: 1_250,
                });

        let plan = registry
            .build_operation_plan(&request)
            .expect("operation plan");

        assert_eq!(
            plan,
            SalesforceOperationPlan::Rest(SalesforceRestRequestPlan::continue_query_page(
                SALESFORCE_CONTINUE_QUERY_PAGE_OPERATION,
                &SalesforceNextRecordsUrl::parse(
                    "/services/data/v67.0/query/01gD0000002HU6KIAW-2000"
                )
                .expect("next records url"),
                1_250,
            ))
        );
    }

    #[test]
    fn continue_query_page_rejects_absolute_urls() {
        let registry = SalesforceOperationRegistry::default();
        let request =
            SalesforceNamedOperationRequest::new(SALESFORCE_CONTINUE_QUERY_PAGE_OPERATION)
                .with_parameter(
                    SALESFORCE_NEXT_RECORDS_URL_FIELD,
                    SalesforceParameterValue::String(
                        "https://evil.example/services/data/v67.0/query/01g".to_string(),
                    ),
                );

        assert_eq!(
            registry.build_operation_plan(&request).unwrap_err(),
            SalesforceProviderError::InvalidParameter {
                name: SALESFORCE_NEXT_RECORDS_URL_FIELD,
                reason: "must be a Salesforce relative REST path, not an absolute URL"
            }
        );
    }

    #[test]
    fn account_describe_rest_plan_converts_to_saas_request_plan() {
        let registry = SalesforceOperationRegistry::default();
        let request = SalesforceNamedOperationRequest::new(ACCOUNT_DESCRIBE_OBJECT_OPERATION);
        let plan = registry
            .build_operation_plan(&request)
            .expect("operation plan");
        let SalesforceOperationPlan::Rest(rest_plan) = plan else {
            panic!("expected REST plan");
        };

        let saas_plan = rest_plan
            .to_saas_request_plan_with_limits(request.limits)
            .expect("valid SaaS plan");

        assert_eq!(saas_plan.operation_name, ACCOUNT_DESCRIBE_OBJECT_OPERATION);
        assert_eq!(saas_plan.protocol, SaasTransportProtocol::RestJson);
        assert_eq!(saas_plan.method, SaasHttpMethod::Get);
        assert_eq!(
            saas_plan.path,
            "/services/data/v67.0/sobjects/Account/describe"
        );
        assert!(saas_plan.query.is_empty());
        assert!(saas_plan.headers.is_empty());
        assert_eq!(saas_plan.body, SaasRequestBody::Empty);
        assert_eq!(
            saas_plan.timeout_ms,
            SalesforceQueryLimits::DEFAULT_TIMEOUT_MS
        );
        assert_eq!(
            saas_plan.caps,
            SaasResponseCaps {
                max_body_bytes: SaasResponseCaps::DEFAULT_MAX_BODY_BYTES,
                max_rows: 1,
            }
        );
        saas_plan.validate().expect("shared plan accepts describe");
    }

    #[test]
    fn fetch_limits_rest_plan_converts_to_saas_request_plan() {
        let registry = SalesforceOperationRegistry::default();
        let request =
            SalesforceNamedOperationRequest::new(SALESFORCE_HEALTH_FETCH_LIMITS_OPERATION)
                .with_limits(SalesforceQueryLimits {
                    max_results: 25,
                    timeout_ms: 750,
                });
        let plan = registry
            .build_operation_plan(&request)
            .expect("operation plan");
        let SalesforceOperationPlan::Rest(rest_plan) = plan else {
            panic!("expected REST plan");
        };

        let saas_plan = rest_plan
            .to_saas_request_plan_with_limits(request.limits)
            .expect("valid SaaS plan");

        assert_eq!(
            saas_plan.operation_name,
            SALESFORCE_HEALTH_FETCH_LIMITS_OPERATION
        );
        assert_eq!(saas_plan.protocol, SaasTransportProtocol::RestJson);
        assert_eq!(saas_plan.method, SaasHttpMethod::Get);
        assert_eq!(saas_plan.path, "/services/data/v67.0/limits");
        assert!(saas_plan.query.is_empty());
        assert!(saas_plan.headers.is_empty());
        assert_eq!(saas_plan.body, SaasRequestBody::Empty);
        assert_eq!(saas_plan.timeout_ms, 750);
        assert_eq!(
            saas_plan.caps,
            SaasResponseCaps {
                max_body_bytes: SaasResponseCaps::DEFAULT_MAX_BODY_BYTES,
                max_rows: 1,
            }
        );
        saas_plan
            .validate()
            .expect("shared plan accepts fetch limits");
    }

    #[test]
    fn continue_page_rest_plan_converts_to_saas_request_plan() {
        let registry = SalesforceOperationRegistry::default();
        let request =
            SalesforceNamedOperationRequest::new(SALESFORCE_CONTINUE_QUERY_PAGE_OPERATION)
                .with_parameter(
                    SALESFORCE_NEXT_RECORDS_URL_FIELD,
                    SalesforceParameterValue::String(
                        "/services/data/v67.0/query/01gD0000002HU6KIAW-2000".to_string(),
                    ),
                )
                .with_limits(SalesforceQueryLimits {
                    max_results: 25,
                    timeout_ms: 1_250,
                });
        let plan = registry
            .build_operation_plan(&request)
            .expect("operation plan");
        let SalesforceOperationPlan::Rest(rest_plan) = plan else {
            panic!("expected REST plan");
        };

        let saas_plan = rest_plan
            .to_saas_request_plan_with_limits(request.limits)
            .expect("valid SaaS plan");

        assert_eq!(
            saas_plan.operation_name,
            SALESFORCE_CONTINUE_QUERY_PAGE_OPERATION
        );
        assert_eq!(saas_plan.protocol, SaasTransportProtocol::RestJson);
        assert_eq!(saas_plan.method, SaasHttpMethod::Get);
        assert_eq!(
            saas_plan.path,
            "/services/data/v67.0/query/01gD0000002HU6KIAW-2000"
        );
        assert!(saas_plan.query.is_empty());
        assert!(saas_plan.headers.is_empty());
        assert_eq!(saas_plan.body, SaasRequestBody::Empty);
        assert_eq!(saas_plan.timeout_ms, 1_250);
        assert_eq!(
            saas_plan.caps,
            SaasResponseCaps {
                max_body_bytes: SaasResponseCaps::DEFAULT_MAX_BODY_BYTES,
                max_rows: 25,
            }
        );
        saas_plan
            .validate()
            .expect("shared plan accepts continue page");
    }

    #[test]
    fn account_fetch_by_ids_builds_bounded_id_set_query() {
        let registry = SalesforceOperationRegistry::default();
        let request = SalesforceNamedOperationRequest::new(ACCOUNT_FETCH_BY_IDS_OPERATION)
            .with_parameter(
                "ids",
                SalesforceParameterValue::StringList(vec![
                    "001000000000001AAA".to_string(),
                    "001000000000002AAA".to_string(),
                ]),
            )
            .with_limits(SalesforceQueryLimits {
                max_results: 2,
                timeout_ms: 2_500,
            });

        let plan = registry.build_query_plan(&request).expect("query plan");

        assert_eq!(plan.operation_name, ACCOUNT_FETCH_BY_IDS_OPERATION);
        assert_eq!(
            plan.soql,
            "SELECT Id, Name, AccountNumber, SystemModstamp, LastModifiedDate, IsDeleted FROM Account WHERE Id IN ('001000000000001AAA', '001000000000002AAA') ORDER BY Id ASC LIMIT 2"
        );
        assert_eq!(plan.timeout_ms, 2_500);
    }

    #[test]
    fn account_fetch_by_ids_converts_to_saas_query_request_plan() {
        let registry = SalesforceOperationRegistry::default();
        let request = SalesforceNamedOperationRequest::new(ACCOUNT_FETCH_BY_IDS_OPERATION)
            .with_parameter(
                "ids",
                SalesforceParameterValue::StringList(vec![
                    "001000000000001AAA".to_string(),
                    "001000000000002AAA".to_string(),
                ]),
            )
            .with_limits(SalesforceQueryLimits {
                max_results: 25,
                timeout_ms: 2_500,
            });

        let saas_plan = registry
            .build_saas_request_plan(&request)
            .expect("SaaS request plan");

        assert_eq!(saas_plan.operation_name, ACCOUNT_FETCH_BY_IDS_OPERATION);
        assert_eq!(saas_plan.protocol, SaasTransportProtocol::RestJson);
        assert_eq!(saas_plan.method, SaasHttpMethod::Get);
        assert_eq!(saas_plan.path, "/services/data/v67.0/query");
        assert_eq!(
            saas_plan.query.get("q").map(String::as_str),
            Some(
                "SELECT Id, Name, AccountNumber, SystemModstamp, LastModifiedDate, IsDeleted FROM Account WHERE Id IN ('001000000000001AAA', '001000000000002AAA') ORDER BY Id ASC LIMIT 2"
            )
        );
        assert_eq!(saas_plan.timeout_ms, 2_500);
        assert_eq!(
            saas_plan.caps,
            SaasResponseCaps {
                max_body_bytes: SaasResponseCaps::DEFAULT_MAX_BODY_BYTES,
                max_rows: 2,
            }
        );
        saas_plan
            .validate()
            .expect("shared plan accepts bounded id query");
    }

    #[test]
    fn account_fetch_by_ids_rejects_unbounded_id_lists() {
        let registry = SalesforceOperationRegistry::default();
        let request = SalesforceNamedOperationRequest::new(ACCOUNT_FETCH_BY_IDS_OPERATION)
            .with_parameter(
                "ids",
                SalesforceParameterValue::StringList(vec![
                    "001000000000001AAA".to_string(),
                    "001000000000002AAA".to_string(),
                ]),
            )
            .with_limits(SalesforceQueryLimits {
                max_results: 1,
                timeout_ms: 2_500,
            });

        assert_eq!(
            registry.build_query_plan(&request).unwrap_err(),
            SalesforceProviderError::InvalidParameter {
                name: "ids",
                reason: "must not exceed limits.max_results"
            }
        );
    }

    #[test]
    fn account_get_updated_ids_converts_to_saas_rest_plan() {
        let registry = SalesforceOperationRegistry::default();
        let request = SalesforceNamedOperationRequest::new(ACCOUNT_GET_UPDATED_IDS_OPERATION)
            .with_parameter(
                "start",
                SalesforceParameterValue::String("2026-06-25T00:00:00Z".to_string()),
            )
            .with_parameter(
                "end",
                SalesforceParameterValue::String("2026-06-26T00:00:00Z".to_string()),
            )
            .with_limits(SalesforceQueryLimits {
                max_results: 500,
                timeout_ms: 2_500,
            });

        let saas_plan = registry
            .build_saas_request_plan(&request)
            .expect("SaaS request plan");

        assert_eq!(saas_plan.operation_name, ACCOUNT_GET_UPDATED_IDS_OPERATION);
        assert_eq!(saas_plan.protocol, SaasTransportProtocol::RestJson);
        assert_eq!(saas_plan.method, SaasHttpMethod::Get);
        assert_eq!(
            saas_plan.path,
            "/services/data/v67.0/sobjects/Account/updated/"
        );
        assert_eq!(
            saas_plan.query.get("start").map(String::as_str),
            Some("2026-06-25T00:00:00Z")
        );
        assert_eq!(
            saas_plan.query.get("end").map(String::as_str),
            Some("2026-06-26T00:00:00Z")
        );
        assert_eq!(saas_plan.timeout_ms, 2_500);
        assert_eq!(
            saas_plan.caps,
            SaasResponseCaps {
                max_body_bytes: SaasResponseCaps::DEFAULT_MAX_BODY_BYTES,
                max_rows: 500,
            }
        );
        assert_eq!(saas_plan.body, SaasRequestBody::Empty);
        saas_plan
            .validate()
            .expect("shared plan accepts getUpdated");
    }

    #[test]
    fn account_get_deleted_ids_converts_to_saas_rest_plan() {
        let registry = SalesforceOperationRegistry::default();
        let request = SalesforceNamedOperationRequest::new(ACCOUNT_GET_DELETED_IDS_OPERATION)
            .with_parameter(
                "start",
                SalesforceParameterValue::String("2026-06-25T00:00:00Z".to_string()),
            )
            .with_parameter(
                "end",
                SalesforceParameterValue::String("2026-06-26T00:00:00Z".to_string()),
            )
            .with_limits(SalesforceQueryLimits {
                max_results: SalesforceQueryLimits::SOQL_MAX_LIMIT + 1,
                timeout_ms: 2_500,
            });

        let saas_plan = registry
            .build_saas_request_plan(&request)
            .expect("SaaS request plan");

        assert_eq!(saas_plan.operation_name, ACCOUNT_GET_DELETED_IDS_OPERATION);
        assert_eq!(
            saas_plan.path,
            "/services/data/v67.0/sobjects/Account/deleted/"
        );
        assert_eq!(
            saas_plan.caps.max_rows,
            SalesforceQueryLimits::SOQL_MAX_LIMIT
        );
        assert_eq!(saas_plan.body, SaasRequestBody::Empty);
        saas_plan
            .validate()
            .expect("shared plan accepts getDeleted");
    }

    #[test]
    fn account_change_id_operations_reject_invalid_windows() {
        let registry = SalesforceOperationRegistry::default();
        let malformed = SalesforceNamedOperationRequest::new(ACCOUNT_GET_UPDATED_IDS_OPERATION)
            .with_parameter(
                "start",
                SalesforceParameterValue::String("2026-06-25T00:00:00-07:00".to_string()),
            )
            .with_parameter(
                "end",
                SalesforceParameterValue::String("2026-06-26T00:00:00Z".to_string()),
            );
        let reversed = SalesforceNamedOperationRequest::new(ACCOUNT_GET_DELETED_IDS_OPERATION)
            .with_parameter(
                "start",
                SalesforceParameterValue::String("2026-06-27T00:00:00Z".to_string()),
            )
            .with_parameter(
                "end",
                SalesforceParameterValue::String("2026-06-26T00:00:00Z".to_string()),
            );

        assert_eq!(
            registry.build_operation_plan(&malformed).unwrap_err(),
            SalesforceProviderError::InvalidParameter {
                name: "start",
                reason: "must be a UTC ISO-8601 datetime literal in YYYY-MM-DDTHH:MM:SSZ form",
            }
        );
        assert_eq!(
            registry.build_operation_plan(&reversed).unwrap_err(),
            SalesforceProviderError::InvalidParameter {
                name: "end",
                reason: "must not sort before start",
            }
        );
    }

    #[test]
    fn generic_health_cloud_primitives_are_registered_but_not_executable() {
        let registry = SalesforceOperationRegistry::default();
        let operation = registry
            .get(SALESFORCE_HEALTH_GET_UPDATED_IDS_OPERATION)
            .expect("registered operation");

        assert_eq!(
            operation.primitive,
            SalesforceOperationPrimitive::GetUpdatedIds
        );
        assert_eq!(
            operation.availability,
            SalesforceOperationAvailability::PlannedUnsupported
        );
        assert_eq!(operation.object, None);

        let request =
            SalesforceNamedOperationRequest::new(SALESFORCE_HEALTH_GET_UPDATED_IDS_OPERATION);

        assert_eq!(
            registry.build_operation_plan(&request).unwrap_err(),
            SalesforceProviderError::UnsupportedOperation {
                name: SALESFORCE_HEALTH_GET_UPDATED_IDS_OPERATION.to_string(),
                reason: "requires authenticated Salesforce org describe evidence"
            }
        );
    }

    #[test]
    fn fixed_health_cloud_candidates_are_metadata_only_until_describe() {
        let registry = SalesforceOperationRegistry::default();
        let operation = registry
            .get(HEALTH_CLOUD_CARE_PROGRAMS_UPDATED_SINCE_OPERATION)
            .expect("registered operation");

        assert_eq!(operation.object, Some(SalesforceObject::CareProgram));
        assert_eq!(operation.sensitivity, SalesforceDataSensitivity::PhiPii);
        assert!(!operation.is_executable());

        let request = SalesforceNamedOperationRequest::new(
            HEALTH_CLOUD_CARE_PROGRAMS_UPDATED_SINCE_OPERATION,
        );

        assert_eq!(
            registry.build_operation_plan(&request).unwrap_err(),
            SalesforceProviderError::UnsupportedOperation {
                name: HEALTH_CLOUD_CARE_PROGRAMS_UPDATED_SINCE_OPERATION.to_string(),
                reason: "requires authenticated Salesforce org describe evidence"
            }
        );
    }

    #[test]
    fn unknown_operation_is_rejected() {
        let registry = SalesforceOperationRegistry::default();
        let request = SalesforceNamedOperationRequest::new("salesforce.raw_soql");

        assert_eq!(
            registry.build_query_plan(&request).unwrap_err(),
            SalesforceProviderError::UnknownOperation("salesforce.raw_soql".to_string())
        );
    }
}
