use std::collections::BTreeMap;

use crate::{
    operation::{
        optional_bool_parameter, optional_string_parameter, required_reference_parameter,
        required_string_parameter, WorkdayOperationAvailability, WorkdayOperationGate,
    },
    GetJobProfilesRequestTemplate, GetLocationsRequestTemplate, GetOrganizationsRequestTemplate,
    GetServerTimestampRequestTemplate, GetWorkerEventHistoryRequestTemplate,
    GetWorkersRequestTemplate, WorkdayEventDateRange, WorkdayNamedOperationRequest,
    WorkdayOperationParameter, WorkdayProviderError, WorkdayQueryLimits, WorkdayRequestPlan,
    WorkdayRequestTemplate, WorkdayResponseFilter, WorkdaySoapRequestTemplate,
    WorkdayWorkerTransactionLogCriteria, WorkdayWwsOperation,
};

pub const WORKDAY_LIST_WORKERS_OPERATION: &str = "workday.list_workers";
pub const WORKDAY_GET_WORKER_OPERATION: &str = "workday.get_worker";
pub const WORKDAY_GET_SERVER_TIMESTAMP_OPERATION: &str = "workday.get_server_timestamp";
pub const WORKDAY_GET_WORKDAY_ACCOUNTS_OPERATION: &str = "workday.get_workday_accounts";
pub const WORKDAY_LIST_FORMER_WORKERS_OPERATION: &str = "workday.list_former_workers";
pub const WORKDAY_LIST_WORKER_EVENTS_OPERATION: &str = "workday.list_worker_events";
pub const WORKDAY_LIST_ORGANIZATIONS_OPERATION: &str = "workday.list_organizations";
pub const WORKDAY_LIST_LOCATIONS_OPERATION: &str = "workday.list_locations";
pub const WORKDAY_LIST_JOB_PROFILES_OPERATION: &str = "workday.list_job_profiles";
pub const WORKDAY_HCM_GET_WORKERS_PAGE_OPERATION: &str = "workday.hcm.get_workers_page";
pub const WORKDAY_HCM_GET_WORKER_PROFILE_OPERATION: &str = "workday.hcm.get_worker_profile";
pub const WORKDAY_HCM_GET_ORGANIZATIONS_PAGE_OPERATION: &str = "workday.hcm.get_organizations_page";
pub const WORKDAY_HCM_GET_JOB_PROFILES_PAGE_OPERATION: &str = "workday.hcm.get_job_profiles_page";
pub const WORKDAY_HCM_GET_LOCATIONS_PAGE_OPERATION: &str = "workday.hcm.get_locations_page";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkdayOperationKind {
    ListWorkers,
    GetWorker,
    GetWorkerProfileViaGetWorkers,
    PlannedUnsupported,
    ListWorkerEvents,
    ListOrganizations,
    ListLocations,
    ListJobProfiles,
    GetServerTimestamp,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisteredWorkdayOperation {
    pub name: &'static str,
    pub kind: WorkdayOperationKind,
    pub wws_operation: WorkdayWwsOperation,
    pub parameters: &'static [WorkdayOperationParameter],
    pub pii_heavy: bool,
    pub availability: WorkdayOperationAvailability,
    pub gates: &'static [WorkdayOperationGate],
}

impl RegisteredWorkdayOperation {
    fn with_name(mut self, name: &'static str) -> Self {
        self.name = name;
        self
    }

    pub fn list_workers() -> Self {
        Self {
            name: WORKDAY_LIST_WORKERS_OPERATION,
            kind: WorkdayOperationKind::ListWorkers,
            wws_operation: WorkdayWwsOperation::GetWorkers,
            parameters: &[
                WorkdayOperationParameter {
                    name: "updated_from",
                    required: false,
                    description: "inclusive lower-bound Workday transaction-log datetime",
                },
                WorkdayOperationParameter {
                    name: "updated_through",
                    required: false,
                    description: "inclusive upper-bound Workday transaction-log datetime",
                },
                WorkdayOperationParameter {
                    name: "effective_from",
                    required: false,
                    description: "inclusive lower-bound effective date",
                },
                WorkdayOperationParameter {
                    name: "effective_through",
                    required: false,
                    description: "inclusive upper-bound effective date",
                },
            ],
            pii_heavy: true,
            availability: WorkdayOperationAvailability::Executable,
            gates: &[],
        }
    }

    pub fn hcm_get_workers_page() -> Self {
        Self::list_workers().with_name(WORKDAY_HCM_GET_WORKERS_PAGE_OPERATION)
    }

    pub fn get_worker() -> Self {
        Self {
            name: WORKDAY_GET_WORKER_OPERATION,
            kind: WorkdayOperationKind::GetWorker,
            wws_operation: WorkdayWwsOperation::GetWorkers,
            parameters: &[WorkdayOperationParameter {
                name: "worker_reference",
                required: true,
                description: "typed Workday Worker reference, such as WID or Employee_ID",
            }],
            pii_heavy: true,
            availability: WorkdayOperationAvailability::Executable,
            gates: &[],
        }
    }

    /// Metadata-aligned Workday HCM operation currently bound through the
    /// proven `Get_Workers` by-reference request. Do not swap this to a
    /// distinct `Get_Worker_Profile` SOAP template until tenant-approved WWS
    /// request-shape evidence is available locally.
    pub fn hcm_get_worker_profile() -> Self {
        Self {
            name: WORKDAY_HCM_GET_WORKER_PROFILE_OPERATION,
            kind: WorkdayOperationKind::GetWorkerProfileViaGetWorkers,
            wws_operation: WorkdayWwsOperation::GetWorkers,
            parameters: &[WorkdayOperationParameter {
                name: "worker_reference",
                required: true,
                description: "typed Workday Worker reference, such as WID or Employee_ID",
            }],
            pii_heavy: true,
            availability: WorkdayOperationAvailability::Executable,
            gates: &[],
        }
    }

    pub fn list_worker_events() -> Self {
        Self {
            name: WORKDAY_LIST_WORKER_EVENTS_OPERATION,
            kind: WorkdayOperationKind::ListWorkerEvents,
            wws_operation: WorkdayWwsOperation::GetWorkerEventHistory,
            parameters: &[
                WorkdayOperationParameter {
                    name: "worker_reference",
                    required: true,
                    description: "typed Workday Worker reference, such as WID or Employee_ID",
                },
                WorkdayOperationParameter {
                    name: "from_event_date",
                    required: true,
                    description: "inclusive event-history start date",
                },
                WorkdayOperationParameter {
                    name: "through_event_date",
                    required: true,
                    description: "inclusive event-history end date",
                },
            ],
            pii_heavy: true,
            availability: WorkdayOperationAvailability::Executable,
            gates: &[],
        }
    }

    pub fn list_organizations() -> Self {
        Self {
            name: WORKDAY_LIST_ORGANIZATIONS_OPERATION,
            kind: WorkdayOperationKind::ListOrganizations,
            wws_operation: WorkdayWwsOperation::GetOrganizations,
            parameters: &[WorkdayOperationParameter {
                name: "include_inactive",
                required: false,
                description: "include inactive organizations in the bounded read",
            }],
            pii_heavy: false,
            availability: WorkdayOperationAvailability::Executable,
            gates: &[],
        }
    }

    pub fn hcm_get_organizations_page() -> Self {
        Self::list_organizations().with_name(WORKDAY_HCM_GET_ORGANIZATIONS_PAGE_OPERATION)
    }

    pub fn list_locations() -> Self {
        Self {
            name: WORKDAY_LIST_LOCATIONS_OPERATION,
            kind: WorkdayOperationKind::ListLocations,
            wws_operation: WorkdayWwsOperation::GetLocations,
            parameters: &[WorkdayOperationParameter {
                name: "include_inactive",
                required: false,
                description: "include inactive locations in the bounded read",
            }],
            pii_heavy: false,
            availability: WorkdayOperationAvailability::Executable,
            gates: &[],
        }
    }

    pub fn hcm_get_locations_page() -> Self {
        Self::list_locations().with_name(WORKDAY_HCM_GET_LOCATIONS_PAGE_OPERATION)
    }

    pub fn list_job_profiles() -> Self {
        Self {
            name: WORKDAY_LIST_JOB_PROFILES_OPERATION,
            kind: WorkdayOperationKind::ListJobProfiles,
            wws_operation: WorkdayWwsOperation::GetJobProfiles,
            parameters: &[WorkdayOperationParameter {
                name: "include_inactive",
                required: false,
                description: "include inactive job profiles in the bounded read",
            }],
            pii_heavy: false,
            availability: WorkdayOperationAvailability::Executable,
            gates: &[],
        }
    }

    pub fn hcm_get_job_profiles_page() -> Self {
        Self::list_job_profiles().with_name(WORKDAY_HCM_GET_JOB_PROFILES_PAGE_OPERATION)
    }

    pub fn get_server_timestamp() -> Self {
        Self {
            name: WORKDAY_GET_SERVER_TIMESTAMP_OPERATION,
            kind: WorkdayOperationKind::GetServerTimestamp,
            wws_operation: WorkdayWwsOperation::GetServerTimestamp,
            parameters: &[],
            pii_heavy: false,
            availability: WorkdayOperationAvailability::Executable,
            gates: &[],
        }
    }

    pub fn planned_list_former_workers() -> Self {
        Self {
            name: WORKDAY_LIST_FORMER_WORKERS_OPERATION,
            kind: WorkdayOperationKind::PlannedUnsupported,
            wws_operation: WorkdayWwsOperation::GetFormerWorkers,
            parameters: &[
                WorkdayOperationParameter {
                    name: "updated_from",
                    required: false,
                    description: "former-worker updated-from datetime; request shape pending tenant evidence",
                },
                WorkdayOperationParameter {
                    name: "updated_through",
                    required: false,
                    description: "former-worker updated-through datetime; request shape pending tenant evidence",
                },
                WorkdayOperationParameter {
                    name: "termination_from",
                    required: false,
                    description: "former-worker termination-date lower bound; request shape pending tenant evidence",
                },
                WorkdayOperationParameter {
                    name: "termination_through",
                    required: false,
                    description: "former-worker termination-date upper bound; request shape pending tenant evidence",
                },
            ],
            pii_heavy: true,
            availability: WorkdayOperationAvailability::PlannedUnsupported,
            gates: &[
                WorkdayOperationGate::TenantApprovedWwsRequestShape,
                WorkdayOperationGate::HrPiiGovernanceReview,
                WorkdayOperationGate::LiveCertification,
            ],
        }
    }

    pub fn planned_get_workday_accounts() -> Self {
        Self {
            name: WORKDAY_GET_WORKDAY_ACCOUNTS_OPERATION,
            kind: WorkdayOperationKind::PlannedUnsupported,
            wws_operation: WorkdayWwsOperation::GetWorkdayAccount,
            parameters: &[WorkdayOperationParameter {
                name: "account_reference",
                required: false,
                description:
                    "typed Workday account reference; request shape pending tenant evidence",
            }],
            pii_heavy: true,
            availability: WorkdayOperationAvailability::PlannedUnsupported,
            gates: &[
                WorkdayOperationGate::TenantApprovedWwsRequestShape,
                WorkdayOperationGate::IdentitySecurityGovernanceReview,
                WorkdayOperationGate::LiveCertification,
            ],
        }
    }

    pub fn is_executable(&self) -> bool {
        self.availability == WorkdayOperationAvailability::Executable
    }

    pub fn unsupported_reason(&self) -> &'static str {
        self.gates
            .first()
            .map(|gate| gate.reason())
            .unwrap_or("operation is planned but not executable")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkdayOperationRegistry {
    operations: BTreeMap<&'static str, RegisteredWorkdayOperation>,
}

impl WorkdayOperationRegistry {
    pub fn new(operations: impl IntoIterator<Item = RegisteredWorkdayOperation>) -> Self {
        let operations = operations
            .into_iter()
            .map(|operation| (operation.name, operation))
            .collect();

        Self { operations }
    }

    pub fn default_read_operations() -> Self {
        Self::new([
            RegisteredWorkdayOperation::list_workers(),
            RegisteredWorkdayOperation::get_server_timestamp(),
            RegisteredWorkdayOperation::get_worker(),
            RegisteredWorkdayOperation::planned_get_workday_accounts(),
            RegisteredWorkdayOperation::list_worker_events(),
            RegisteredWorkdayOperation::planned_list_former_workers(),
            RegisteredWorkdayOperation::list_organizations(),
            RegisteredWorkdayOperation::list_locations(),
            RegisteredWorkdayOperation::list_job_profiles(),
            RegisteredWorkdayOperation::hcm_get_workers_page(),
            RegisteredWorkdayOperation::hcm_get_worker_profile(),
            RegisteredWorkdayOperation::hcm_get_organizations_page(),
            RegisteredWorkdayOperation::hcm_get_locations_page(),
            RegisteredWorkdayOperation::hcm_get_job_profiles_page(),
        ])
    }

    pub fn get(&self, name: &str) -> Option<&RegisteredWorkdayOperation> {
        self.operations.get(name)
    }

    pub fn operation_names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.operations.keys().copied()
    }

    pub fn build_request_plan(
        &self,
        request: &WorkdayNamedOperationRequest,
    ) -> Result<WorkdayRequestPlan, WorkdayProviderError> {
        let operation = self
            .get(&request.name)
            .ok_or_else(|| WorkdayProviderError::UnknownOperation(request.name.clone()))?;

        request.limits.validate()?;
        request.response_filter.validate()?;

        if !operation.is_executable() {
            return Err(WorkdayProviderError::UnsupportedOperation {
                name: operation.name.to_string(),
                reason: operation.unsupported_reason(),
            });
        }

        match operation.kind {
            WorkdayOperationKind::ListWorkers => build_list_workers_plan(operation, request),
            WorkdayOperationKind::GetWorker => build_get_worker_plan(operation, request),
            WorkdayOperationKind::GetWorkerProfileViaGetWorkers => {
                build_get_worker_profile_plan(operation, request)
            }
            WorkdayOperationKind::ListWorkerEvents => {
                build_list_worker_events_plan(operation, request)
            }
            WorkdayOperationKind::ListOrganizations => {
                build_list_organizations_plan(operation, request)
            }
            WorkdayOperationKind::ListLocations => build_list_locations_plan(operation, request),
            WorkdayOperationKind::ListJobProfiles => {
                build_list_job_profiles_plan(operation, request)
            }
            WorkdayOperationKind::GetServerTimestamp => {
                build_get_server_timestamp_plan(operation, request)
            }
            WorkdayOperationKind::PlannedUnsupported => {
                Err(WorkdayProviderError::UnsupportedOperation {
                    name: operation.name.to_string(),
                    reason: operation.unsupported_reason(),
                })
            }
        }
    }
}

impl Default for WorkdayOperationRegistry {
    fn default() -> Self {
        Self::default_read_operations()
    }
}

fn build_list_workers_plan(
    operation: &RegisteredWorkdayOperation,
    request: &WorkdayNamedOperationRequest,
) -> Result<WorkdayRequestPlan, WorkdayProviderError> {
    let criteria = WorkdayWorkerTransactionLogCriteria {
        updated_from: optional_string_parameter(request, "updated_from")?.map(str::to_string),
        updated_through: optional_string_parameter(request, "updated_through")?.map(str::to_string),
        effective_from: optional_string_parameter(request, "effective_from")?.map(str::to_string),
        effective_through: optional_string_parameter(request, "effective_through")?
            .map(str::to_string),
    };
    let template = WorkdayRequestTemplate::GetWorkers(GetWorkersRequestTemplate::list(
        criteria,
        request.response_filter.clone(),
    ));

    build_plan(operation, request, template)
}

fn build_get_worker_plan(
    operation: &RegisteredWorkdayOperation,
    request: &WorkdayNamedOperationRequest,
) -> Result<WorkdayRequestPlan, WorkdayProviderError> {
    build_worker_by_reference_plan(operation, request)
}

fn build_get_worker_profile_plan(
    operation: &RegisteredWorkdayOperation,
    request: &WorkdayNamedOperationRequest,
) -> Result<WorkdayRequestPlan, WorkdayProviderError> {
    // Compatibility binding: local evidence confirms Get_Worker_Profile exists,
    // but not enough request-shape detail to emit a distinct template safely.
    build_worker_by_reference_plan(operation, request)
}

fn build_worker_by_reference_plan(
    operation: &RegisteredWorkdayOperation,
    request: &WorkdayNamedOperationRequest,
) -> Result<WorkdayRequestPlan, WorkdayProviderError> {
    let worker_reference = required_reference_parameter(request, "worker_reference")?.clone();
    let template = WorkdayRequestTemplate::GetWorkers(GetWorkersRequestTemplate::by_reference(
        worker_reference,
        request.response_filter.clone(),
    ));

    build_plan(operation, request, template)
}

fn build_list_worker_events_plan(
    operation: &RegisteredWorkdayOperation,
    request: &WorkdayNamedOperationRequest,
) -> Result<WorkdayRequestPlan, WorkdayProviderError> {
    let worker_reference = required_reference_parameter(request, "worker_reference")?.clone();
    let event_date_range = WorkdayEventDateRange::new(
        required_string_parameter(request, "from_event_date")?,
        required_string_parameter(request, "through_event_date")?,
    )?;
    let template =
        WorkdayRequestTemplate::GetWorkerEventHistory(GetWorkerEventHistoryRequestTemplate {
            worker_reference,
            event_date_range,
            response_filter: request.response_filter.clone(),
        });

    build_plan(operation, request, template)
}

fn build_list_organizations_plan(
    operation: &RegisteredWorkdayOperation,
    request: &WorkdayNamedOperationRequest,
) -> Result<WorkdayRequestPlan, WorkdayProviderError> {
    let template = WorkdayRequestTemplate::GetOrganizations(GetOrganizationsRequestTemplate {
        include_inactive: optional_bool_parameter(request, "include_inactive")?.unwrap_or(false),
        response_filter: request.response_filter.clone(),
    });

    build_plan(operation, request, template)
}

fn build_list_locations_plan(
    operation: &RegisteredWorkdayOperation,
    request: &WorkdayNamedOperationRequest,
) -> Result<WorkdayRequestPlan, WorkdayProviderError> {
    let template = WorkdayRequestTemplate::GetLocations(GetLocationsRequestTemplate {
        include_inactive: optional_bool_parameter(request, "include_inactive")?.unwrap_or(false),
        response_filter: request.response_filter.clone(),
    });

    build_plan(operation, request, template)
}

fn build_list_job_profiles_plan(
    operation: &RegisteredWorkdayOperation,
    request: &WorkdayNamedOperationRequest,
) -> Result<WorkdayRequestPlan, WorkdayProviderError> {
    let template = WorkdayRequestTemplate::GetJobProfiles(GetJobProfilesRequestTemplate {
        include_inactive: optional_bool_parameter(request, "include_inactive")?.unwrap_or(false),
        response_filter: request.response_filter.clone(),
    });

    build_plan(operation, request, template)
}

fn build_get_server_timestamp_plan(
    operation: &RegisteredWorkdayOperation,
    request: &WorkdayNamedOperationRequest,
) -> Result<WorkdayRequestPlan, WorkdayProviderError> {
    let template = WorkdayRequestTemplate::GetServerTimestamp(GetServerTimestampRequestTemplate);
    let envelope = template.soap_envelope()?;

    Ok(WorkdayRequestPlan::new(
        operation.name,
        template,
        envelope,
        WorkdayResponseFilter::new(1, 1),
        WorkdayQueryLimits {
            timeout_ms: request.limits.timeout_ms,
            max_pages: 1,
        },
    ))
}

fn build_plan(
    operation: &RegisteredWorkdayOperation,
    request: &WorkdayNamedOperationRequest,
    template: WorkdayRequestTemplate,
) -> Result<WorkdayRequestPlan, WorkdayProviderError> {
    let envelope = template.soap_envelope()?;

    Ok(WorkdayRequestPlan::new(
        operation.name,
        template,
        envelope,
        request.response_filter.clone(),
        request.limits,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        WorkdayObjectReference, WorkdayParameterValue, WorkdayQueryLimits, WorkdayReferenceType,
        WorkdayResponseFilter,
    };
    use appfw_saas_core::{
        SaasHttpMethod, SaasRequestBody, SaasResponseCaps, SaasTransportProtocol,
    };

    const HUMAN_RESOURCES_WWS_PATH: &str = "/ccx/service/acme/Human_Resources/v46.1";

    #[test]
    fn registry_exposes_stable_operation_names() {
        let registry = WorkdayOperationRegistry::default();
        let names = registry.operation_names().collect::<Vec<_>>();

        assert_eq!(
            names,
            vec![
                WORKDAY_GET_SERVER_TIMESTAMP_OPERATION,
                WORKDAY_GET_WORKDAY_ACCOUNTS_OPERATION,
                WORKDAY_GET_WORKER_OPERATION,
                WORKDAY_HCM_GET_JOB_PROFILES_PAGE_OPERATION,
                WORKDAY_HCM_GET_LOCATIONS_PAGE_OPERATION,
                WORKDAY_HCM_GET_ORGANIZATIONS_PAGE_OPERATION,
                WORKDAY_HCM_GET_WORKER_PROFILE_OPERATION,
                WORKDAY_HCM_GET_WORKERS_PAGE_OPERATION,
                WORKDAY_LIST_FORMER_WORKERS_OPERATION,
                WORKDAY_LIST_JOB_PROFILES_OPERATION,
                WORKDAY_LIST_LOCATIONS_OPERATION,
                WORKDAY_LIST_ORGANIZATIONS_OPERATION,
                WORKDAY_LIST_WORKER_EVENTS_OPERATION,
                WORKDAY_LIST_WORKERS_OPERATION,
            ]
        );
    }

    #[test]
    fn executable_operations_have_no_evidence_gates() {
        let registry = WorkdayOperationRegistry::default();

        for operation in registry
            .operation_names()
            .filter_map(|name| registry.get(name))
            .filter(|operation| operation.is_executable())
        {
            assert_eq!(
                operation.availability,
                WorkdayOperationAvailability::Executable
            );
            assert!(operation.gates.is_empty());
        }
    }

    #[test]
    fn planned_former_workers_is_registered_but_not_executable() {
        let registry = WorkdayOperationRegistry::default();
        let operation = registry
            .get(WORKDAY_LIST_FORMER_WORKERS_OPERATION)
            .expect("registered former-workers operation");

        assert_eq!(operation.kind, WorkdayOperationKind::PlannedUnsupported);
        assert_eq!(
            operation.wws_operation,
            WorkdayWwsOperation::GetFormerWorkers
        );
        assert_eq!(
            operation.availability,
            WorkdayOperationAvailability::PlannedUnsupported
        );
        assert_eq!(
            operation.gates,
            &[
                WorkdayOperationGate::TenantApprovedWwsRequestShape,
                WorkdayOperationGate::HrPiiGovernanceReview,
                WorkdayOperationGate::LiveCertification,
            ]
        );

        assert_eq!(
            registry
                .build_request_plan(&WorkdayNamedOperationRequest::new(
                    WORKDAY_LIST_FORMER_WORKERS_OPERATION,
                ))
                .unwrap_err(),
            WorkdayProviderError::UnsupportedOperation {
                name: WORKDAY_LIST_FORMER_WORKERS_OPERATION.to_string(),
                reason: "requires tenant-approved Workday WWS request-shape evidence",
            }
        );
    }

    #[test]
    fn planned_workday_accounts_is_registered_but_not_executable() {
        let registry = WorkdayOperationRegistry::default();
        let operation = registry
            .get(WORKDAY_GET_WORKDAY_ACCOUNTS_OPERATION)
            .expect("registered Workday account operation");

        assert_eq!(operation.kind, WorkdayOperationKind::PlannedUnsupported);
        assert_eq!(
            operation.wws_operation,
            WorkdayWwsOperation::GetWorkdayAccount
        );
        assert_eq!(
            operation.availability,
            WorkdayOperationAvailability::PlannedUnsupported
        );
        assert_eq!(
            operation.gates,
            &[
                WorkdayOperationGate::TenantApprovedWwsRequestShape,
                WorkdayOperationGate::IdentitySecurityGovernanceReview,
                WorkdayOperationGate::LiveCertification,
            ]
        );

        assert_eq!(
            registry
                .build_request_plan(&WorkdayNamedOperationRequest::new(
                    WORKDAY_GET_WORKDAY_ACCOUNTS_OPERATION,
                ))
                .unwrap_err(),
            WorkdayProviderError::UnsupportedOperation {
                name: WORKDAY_GET_WORKDAY_ACCOUNTS_OPERATION.to_string(),
                reason: "requires tenant-approved Workday WWS request-shape evidence",
            }
        );
    }

    fn assert_alias_builds_same_template_as_existing_name<F>(
        existing_name: &'static str,
        alias_name: &'static str,
        configure: F,
    ) where
        F: Fn(WorkdayNamedOperationRequest) -> WorkdayNamedOperationRequest,
    {
        let registry = WorkdayOperationRegistry::default();
        let existing_plan = registry
            .build_request_plan(&configure(WorkdayNamedOperationRequest::new(existing_name)))
            .expect("existing request plan");
        let alias_plan = registry
            .build_request_plan(&configure(WorkdayNamedOperationRequest::new(alias_name)))
            .expect("alias request plan");

        assert_eq!(existing_plan.operation_name.as_str(), existing_name);
        assert_eq!(alias_plan.operation_name.as_str(), alias_name);
        assert_eq!(alias_plan.wws_operation, existing_plan.wws_operation);
        assert_eq!(alias_plan.request_template, existing_plan.request_template);
        assert_eq!(alias_plan.soap_envelope, existing_plan.soap_envelope);
        assert_eq!(alias_plan.response_filter, existing_plan.response_filter);
        assert_eq!(alias_plan.timeout_ms, existing_plan.timeout_ms);
        assert_eq!(alias_plan.max_pages, existing_plan.max_pages);
        assert_eq!(alias_plan.read_only, existing_plan.read_only);
    }

    #[test]
    fn metadata_aliases_build_same_typed_templates_as_existing_names() {
        assert_alias_builds_same_template_as_existing_name(
            WORKDAY_LIST_WORKERS_OPERATION,
            WORKDAY_HCM_GET_WORKERS_PAGE_OPERATION,
            |request| {
                request
                    .with_parameter(
                        "updated_from",
                        WorkdayParameterValue::String("2026-06-25T00:00:00Z".to_string()),
                    )
                    .with_parameter(
                        "updated_through",
                        WorkdayParameterValue::String("2026-06-26T00:00:00Z".to_string()),
                    )
                    .with_response_filter(WorkdayResponseFilter::new(1, 25))
                    .with_limits(WorkdayQueryLimits {
                        timeout_ms: 7_500,
                        max_pages: 3,
                    })
            },
        );

        for (existing_name, alias_name) in [
            (
                WORKDAY_LIST_ORGANIZATIONS_OPERATION,
                WORKDAY_HCM_GET_ORGANIZATIONS_PAGE_OPERATION,
            ),
            (
                WORKDAY_LIST_LOCATIONS_OPERATION,
                WORKDAY_HCM_GET_LOCATIONS_PAGE_OPERATION,
            ),
            (
                WORKDAY_LIST_JOB_PROFILES_OPERATION,
                WORKDAY_HCM_GET_JOB_PROFILES_PAGE_OPERATION,
            ),
        ] {
            assert_alias_builds_same_template_as_existing_name(
                existing_name,
                alias_name,
                |request| {
                    request
                        .with_parameter("include_inactive", WorkdayParameterValue::Bool(true))
                        .with_response_filter(WorkdayResponseFilter::new(1, 25))
                },
            );
        }
    }

    #[test]
    fn hcm_get_worker_profile_is_first_class_compatibility_binding() {
        let registry = WorkdayOperationRegistry::default();
        let operation = registry
            .get(WORKDAY_HCM_GET_WORKER_PROFILE_OPERATION)
            .expect("registered HCM worker profile operation");

        assert_eq!(
            operation.kind,
            WorkdayOperationKind::GetWorkerProfileViaGetWorkers
        );
        assert_eq!(operation.wws_operation, WorkdayWwsOperation::GetWorkers);
        assert!(operation.pii_heavy);

        let reference =
            WorkdayObjectReference::new(WorkdayReferenceType::EmployeeId, "E123").unwrap();
        let request = WorkdayNamedOperationRequest::new(WORKDAY_HCM_GET_WORKER_PROFILE_OPERATION)
            .with_parameter(
                "worker_reference",
                WorkdayParameterValue::Reference(reference),
            )
            .with_response_filter(WorkdayResponseFilter::new(1, 1))
            .with_limits(WorkdayQueryLimits {
                timeout_ms: 2_500,
                max_pages: 1,
            });

        let plan = registry.build_request_plan(&request).expect("request plan");

        assert_eq!(
            plan.operation_name.as_str(),
            WORKDAY_HCM_GET_WORKER_PROFILE_OPERATION
        );
        assert_eq!(plan.wws_operation, WorkdayWwsOperation::GetWorkers);
        assert!(matches!(
            plan.request_template,
            WorkdayRequestTemplate::GetWorkers(_)
        ));
        assert!(plan
            .soap_envelope
            .contains("<bsvc:Get_Workers_Request bsvc:version=\"v46.1\">"));
        assert!(plan.soap_envelope.contains("<bsvc:Request_References>"));
        assert!(plan
            .soap_envelope
            .contains("<bsvc:ID bsvc:type=\"Employee_ID\">E123</bsvc:ID>"));
    }

    #[test]
    fn list_workers_builds_read_only_get_workers_plan() {
        let registry = WorkdayOperationRegistry::default();
        let request = WorkdayNamedOperationRequest::new(WORKDAY_LIST_WORKERS_OPERATION)
            .with_parameter(
                "updated_from",
                WorkdayParameterValue::String("2026-06-25T00:00:00Z".to_string()),
            )
            .with_parameter(
                "updated_through",
                WorkdayParameterValue::String("2026-06-26T00:00:00Z".to_string()),
            )
            .with_response_filter(
                WorkdayResponseFilter::new(1, 25).with_as_of_entry_datetime("2026-06-26T04:00:00Z"),
            )
            .with_limits(WorkdayQueryLimits {
                timeout_ms: 7_500,
                max_pages: 3,
            });

        let plan = registry.build_request_plan(&request).expect("request plan");

        assert_eq!(plan.operation_name, WORKDAY_LIST_WORKERS_OPERATION);
        assert_eq!(plan.wws_operation, WorkdayWwsOperation::GetWorkers);
        assert_eq!(plan.timeout_ms, 7_500);
        assert_eq!(plan.max_pages, 3);
        assert!(plan.read_only);
        assert!(plan
            .soap_envelope
            .contains("<bsvc:Get_Workers_Request bsvc:version=\"v46.1\">"));
        assert!(plan
            .soap_envelope
            .contains("<bsvc:Updated_Through>2026-06-26T00:00:00Z</bsvc:Updated_Through>"));
    }

    #[test]
    fn get_workers_page_converts_to_valid_saas_soap_request_plan() {
        let registry = WorkdayOperationRegistry::default();
        let request = WorkdayNamedOperationRequest::new(WORKDAY_HCM_GET_WORKERS_PAGE_OPERATION)
            .with_response_filter(WorkdayResponseFilter::new(1, 25))
            .with_limits(WorkdayQueryLimits {
                timeout_ms: 7_500,
                max_pages: 3,
            });

        let workday_plan = registry.build_request_plan(&request).expect("Workday plan");
        let saas_plan = workday_plan
            .to_saas_request_plan(HUMAN_RESOURCES_WWS_PATH)
            .expect("SaaS request plan");

        assert_eq!(
            saas_plan.operation_name,
            WORKDAY_HCM_GET_WORKERS_PAGE_OPERATION
        );
        assert_eq!(saas_plan.protocol, SaasTransportProtocol::SoapXml);
        assert_eq!(saas_plan.method, SaasHttpMethod::Post);
        assert_eq!(saas_plan.path, HUMAN_RESOURCES_WWS_PATH);
        assert_eq!(saas_plan.timeout_ms, 7_500);
        assert_eq!(
            saas_plan.caps,
            SaasResponseCaps {
                max_body_bytes: SaasResponseCaps::DEFAULT_MAX_BODY_BYTES * 3,
                max_rows: 75,
            }
        );
        assert!(saas_plan.query.is_empty());
        assert!(saas_plan.headers.is_empty());

        match &saas_plan.body {
            SaasRequestBody::Text {
                content_type,
                value,
            } => {
                assert_eq!(content_type, "text/xml");
                assert!(value.contains("<soapenv:Envelope"));
                assert!(value.contains("<bsvc:Get_Workers_Request bsvc:version=\"v46.1\">"));
            }
            other => panic!("expected SOAP XML text body, got {other:?}"),
        }
        saas_plan.validate().expect("shared plan validation");
    }

    #[test]
    fn metadata_alias_converts_to_valid_saas_soap_request_plan() {
        let registry = WorkdayOperationRegistry::default();
        let reference =
            WorkdayObjectReference::new(WorkdayReferenceType::EmployeeId, "E123").unwrap();
        let request = WorkdayNamedOperationRequest::new(WORKDAY_HCM_GET_WORKER_PROFILE_OPERATION)
            .with_parameter(
                "worker_reference",
                WorkdayParameterValue::Reference(reference),
            )
            .with_response_filter(WorkdayResponseFilter::new(1, 1))
            .with_limits(WorkdayQueryLimits {
                timeout_ms: 2_500,
                max_pages: 1,
            });

        let workday_plan = registry.build_request_plan(&request).expect("Workday plan");
        let saas_plan = workday_plan
            .to_saas_request_plan(HUMAN_RESOURCES_WWS_PATH)
            .expect("SaaS request plan");

        assert_eq!(
            saas_plan.operation_name,
            WORKDAY_HCM_GET_WORKER_PROFILE_OPERATION
        );
        assert_eq!(saas_plan.protocol, SaasTransportProtocol::SoapXml);
        assert_eq!(saas_plan.method, SaasHttpMethod::Post);
        assert_eq!(saas_plan.timeout_ms, 2_500);
        assert_eq!(saas_plan.caps.max_rows, 1);
        match &saas_plan.body {
            SaasRequestBody::Text {
                content_type,
                value,
            } => {
                assert_eq!(content_type, "text/xml");
                assert!(value.contains("<bsvc:Get_Workers_Request bsvc:version=\"v46.1\">"));
                assert!(value.contains("<bsvc:Request_References>"));
            }
            other => panic!("expected SOAP XML text body, got {other:?}"),
        }
        saas_plan.validate().expect("shared plan validation");
    }

    #[test]
    fn absolute_soap_endpoint_is_rejected_by_shared_plan_validation() {
        let registry = WorkdayOperationRegistry::default();
        let request = WorkdayNamedOperationRequest::new(WORKDAY_HCM_GET_WORKERS_PAGE_OPERATION);
        let workday_plan = registry.build_request_plan(&request).expect("Workday plan");

        let err = workday_plan
            .to_saas_request_plan(
                "https://wd2-impl-services1.workday.com/ccx/service/acme/Human_Resources/v46.1",
            )
            .unwrap_err();

        match err {
            WorkdayProviderError::InvalidSaasRequestPlan(reason) => {
                assert!(reason.contains("path"));
            }
            other => panic!("expected shared SaaS request plan validation error, got {other:?}"),
        }
    }

    #[test]
    fn non_human_resources_wws_path_shape_is_rejected() {
        let registry = WorkdayOperationRegistry::default();
        let request = WorkdayNamedOperationRequest::new(WORKDAY_HCM_GET_WORKERS_PAGE_OPERATION);
        let workday_plan = registry.build_request_plan(&request).expect("Workday plan");

        for path in [
            "/ccx/service/acme/Financial_Management/v46.1",
            "/ccx/service/acme/Human_Resources/v45.0",
            "/ccx/service/acme/Human_Resources/v46.1/extra",
            "/foo/Human_Resources/bar",
        ] {
            assert_eq!(
                workday_plan.to_saas_request_plan(path).unwrap_err(),
                WorkdayProviderError::InvalidParameter {
                    name: "human_resources_wws_path",
                    reason: "must be /ccx/service/{tenant}/Human_Resources/v46.1",
                }
            );
        }
    }

    #[test]
    fn human_resources_wws_path_rejects_tenant_punctuation() {
        let registry = WorkdayOperationRegistry::default();
        let request = WorkdayNamedOperationRequest::new(WORKDAY_HCM_GET_WORKERS_PAGE_OPERATION);
        let workday_plan = registry.build_request_plan(&request).expect("Workday plan");

        assert_eq!(
            workday_plan
                .to_saas_request_plan("/ccx/service/acme:443/Human_Resources/v46.1")
                .unwrap_err(),
            WorkdayProviderError::InvalidParameter {
                name: "human_resources_wws_path.tenant",
                reason: "must not contain query, fragment, userinfo, or port punctuation",
            }
        );
    }

    #[test]
    fn get_worker_uses_typed_reference_parameter() {
        let registry = WorkdayOperationRegistry::default();
        let reference =
            WorkdayObjectReference::new(WorkdayReferenceType::EmployeeId, "E123").unwrap();
        let request = WorkdayNamedOperationRequest::new(WORKDAY_GET_WORKER_OPERATION)
            .with_parameter(
                "worker_reference",
                WorkdayParameterValue::Reference(reference),
            )
            .with_response_filter(WorkdayResponseFilter::new(1, 1));

        let plan = registry.build_request_plan(&request).expect("request plan");

        assert_eq!(plan.wws_operation, WorkdayWwsOperation::GetWorkers);
        assert!(plan.soap_envelope.contains("<bsvc:Request_References>"));
        assert!(plan
            .soap_envelope
            .contains("<bsvc:ID bsvc:type=\"Employee_ID\">E123</bsvc:ID>"));
        assert!(plan
            .soap_envelope
            .contains("<bsvc:Page>1</bsvc:Page><bsvc:Count>1</bsvc:Count>"));
    }

    #[test]
    fn list_worker_events_builds_event_history_template() {
        let registry = WorkdayOperationRegistry::default();
        let reference = WorkdayObjectReference::new(WorkdayReferenceType::Wid, "abc123").unwrap();
        let request = WorkdayNamedOperationRequest::new(WORKDAY_LIST_WORKER_EVENTS_OPERATION)
            .with_parameter(
                "worker_reference",
                WorkdayParameterValue::Reference(reference),
            )
            .with_parameter(
                "from_event_date",
                WorkdayParameterValue::String("2026-06-01".to_string()),
            )
            .with_parameter(
                "through_event_date",
                WorkdayParameterValue::String("2026-06-26".to_string()),
            );

        let plan = registry.build_request_plan(&request).expect("request plan");

        assert_eq!(
            plan.wws_operation,
            WorkdayWwsOperation::GetWorkerEventHistory
        );
        assert!(plan
            .soap_envelope
            .contains("<bsvc:Get_Worker_Event_History_Request bsvc:version=\"v46.1\">"));
        assert!(plan
            .soap_envelope
            .contains("<bsvc:From_Date>2026-06-01</bsvc:From_Date>"));
    }

    #[test]
    fn reference_data_operations_build_registered_get_templates() {
        let registry = WorkdayOperationRegistry::default();
        let cases = [
            (
                WORKDAY_LIST_ORGANIZATIONS_OPERATION,
                WorkdayWwsOperation::GetOrganizations,
                "<bsvc:Get_Organizations_Request bsvc:version=\"v46.1\">",
            ),
            (
                WORKDAY_LIST_LOCATIONS_OPERATION,
                WorkdayWwsOperation::GetLocations,
                "<bsvc:Get_Locations_Request bsvc:version=\"v46.1\">",
            ),
            (
                WORKDAY_LIST_JOB_PROFILES_OPERATION,
                WorkdayWwsOperation::GetJobProfiles,
                "<bsvc:Get_Job_Profiles_Request bsvc:version=\"v46.1\">",
            ),
        ];

        for (operation_name, wws_operation, request_element) in cases {
            let request = WorkdayNamedOperationRequest::new(operation_name)
                .with_parameter("include_inactive", WorkdayParameterValue::Bool(true));

            let plan = registry.build_request_plan(&request).expect("request plan");

            assert_eq!(plan.wws_operation, wws_operation);
            assert!(plan.soap_envelope.contains(request_element));
            assert!(plan
                .soap_envelope
                .contains("<bsvc:Include_Inactive>true</bsvc:Include_Inactive>"));
        }
    }

    #[test]
    fn get_server_timestamp_builds_single_row_metadata_request() {
        let registry = WorkdayOperationRegistry::default();
        let request = WorkdayNamedOperationRequest::new(WORKDAY_GET_SERVER_TIMESTAMP_OPERATION)
            .with_response_filter(WorkdayResponseFilter::new(1, 999))
            .with_limits(WorkdayQueryLimits {
                timeout_ms: 1_500,
                max_pages: 99,
            });

        let plan = registry.build_request_plan(&request).expect("request plan");

        assert_eq!(plan.operation_name, WORKDAY_GET_SERVER_TIMESTAMP_OPERATION);
        assert_eq!(plan.wws_operation, WorkdayWwsOperation::GetServerTimestamp);
        assert_eq!(plan.timeout_ms, 1_500);
        assert_eq!(plan.max_pages, 1);
        assert_eq!(plan.response_filter, WorkdayResponseFilter::new(1, 1));
        assert!(!plan.soap_envelope.contains("<bsvc:Response_Filter>"));
        assert!(plan
            .soap_envelope
            .contains("<bsvc:Get_Server_Timestamp_Request bsvc:version=\"v46.1\">"));

        let saas_plan = plan
            .to_saas_request_plan(HUMAN_RESOURCES_WWS_PATH)
            .expect("SaaS request plan");

        assert_eq!(
            saas_plan.operation_name,
            WORKDAY_GET_SERVER_TIMESTAMP_OPERATION
        );
        assert_eq!(
            saas_plan.caps,
            SaasResponseCaps {
                max_body_bytes: SaasResponseCaps::DEFAULT_MAX_BODY_BYTES,
                max_rows: 1,
            }
        );
    }

    #[test]
    fn unknown_operation_is_rejected() {
        let registry = WorkdayOperationRegistry::default();
        let request = WorkdayNamedOperationRequest::new("workday.raw_xml");

        assert_eq!(
            registry.build_request_plan(&request).unwrap_err(),
            WorkdayProviderError::UnknownOperation("workday.raw_xml".to_string())
        );
    }
}
