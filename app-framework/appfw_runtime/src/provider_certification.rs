use chrono::Utc;
use serde_json::{json, Value};

use crate::{
    provider_capabilities::{
        graph_read_profile, saas_read_profile, semantic_profile, CapabilityStatus,
        CertificationEvidence, GraphReadCapability, ProviderCapability, ProviderContractArea,
        SaasReadCapability, SUPPORTED_PROVIDERS,
    },
    provider_keys::FrameworkProvider,
};

const AI_SEARCH_CONTRACT_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(
        "appfw_provider_ai_search::registry::tests::registry_exposes_stable_operation_names",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_ai_search::registry::tests::planned_queries_require_enterprise_contract_evidence",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_ai_search::metadata::tests::metadata_operation_names_match_registry_constants",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_ai_search::gateway::tests::gateway_contract_prefers_litellm_with_server_secret_custody",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_ai_search::gateway::tests::gateway_contract_requires_prompt_audit_and_policy_re_resolution",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_ai_search::gateway::tests::token_cache_key_is_provider_and_tenant_scoped",
    ),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractResult {
    Passed,
    Failed,
    Listed,
    NotRun,
    NotRequired,
}

impl ContractResult {
    pub fn label(self) -> &'static str {
        match self {
            ContractResult::Passed => "passed",
            ContractResult::Failed => "failed",
            ContractResult::Listed => "listed",
            ContractResult::NotRun => "not-run",
            ContractResult::NotRequired => "not-required",
        }
    }
}

pub fn provider_matrix_json(provider: Option<FrameworkProvider>) -> Value {
    if provider == Some(FrameworkProvider::Neo4j) {
        let profile = graph_read_profile(FrameworkProvider::Neo4j);
        return json!({
            "providers": [
                {
                    "provider": "neo4j",
                    "category": "graph_read",
                    "database_semantic_parity": "not_applicable",
                    "reason": "Neo4j is certified through graph-read contracts and named query guardrails, not generated CRUD provider parity.",
                    "graph_read_areas": profile
                        .capabilities
                        .iter()
                        .map(graph_read_capability_json)
                        .collect::<Vec<_>>(),
                    "live_certification": "pending"
                }
            ]
        });
    }

    if provider
        .map(|provider| provider.is_external_api_provider())
        .unwrap_or(false)
    {
        let provider = provider.expect("checked provider");
        let profile = saas_read_profile(provider);
        return json!({
            "providers": [
                {
                    "provider": provider.key(),
                    "category": "external_api",
                    "database_semantic_parity": "not_applicable",
                    "reason": "External API providers are certified through SaaS named-operation contracts, not generated CRUD provider parity.",
                    "saas_read_areas": profile
                        .capabilities
                        .iter()
                        .map(saas_read_capability_json)
                        .collect::<Vec<_>>(),
                    "live_certification": "pending"
                }
            ]
        });
    }

    if provider == Some(FrameworkProvider::AiSearch) {
        let (compiler_contracts, live_contracts) = split_evidence(AI_SEARCH_CONTRACT_EVIDENCE);
        return json!({
            "providers": [
                {
                    "provider": "ai_search",
                    "category": "ai_search",
                    "database_semantic_parity": "not_applicable",
                    "reason": "AI Search is certified through named-query contract evidence, not generated CRUD provider parity.",
                    "ai_search_areas": [
                        {
                            "area": "contract_evidence",
                            "label": "AI Search Contract Evidence",
                            "status": "unsupported",
                            "reason": "AI search contract evidence is compiler-contracted, but executable calls require authenticated PDS AI search evidence, approved gateway/auth evidence, and the shared SaaS HTTP executor.",
                            "live_result": "not-required",
                            "compiler_contracts": compiler_contracts,
                            "live_contracts": live_contracts
                        }
                    ],
                    "live_certification": "pending"
                }
            ]
        });
    }

    let providers = provider
        .map(|provider| vec![provider])
        .unwrap_or_else(|| SUPPORTED_PROVIDERS.to_vec());

    let profiles: Vec<Value> = providers
        .into_iter()
        .map(|provider| {
            let profile = semantic_profile(provider);
            json!({
                "provider": provider.key(),
                "areas": profile.capabilities.iter().map(capability_json).collect::<Vec<_>>(),
            })
        })
        .collect();

    json!({ "providers": profiles })
}

pub fn provider_graduation_json() -> Value {
    let providers = FrameworkProvider::ALL
        .into_iter()
        .map(provider_graduation_entry)
        .collect::<Vec<_>>();

    let summary = graduation_summary(&providers);
    let ok = summary["promotion_violation_count"] == json!(0);

    json!({
        "command": "provider-graduation",
        "lane": "U4",
        "ok": ok,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "mode": "report-only",
        "summary": summary,
        "providers": providers,
    })
}

fn provider_graduation_entry(provider: FrameworkProvider) -> Value {
    if provider.is_ai_search_provider() {
        const AI_SEARCH_UNSUPPORTED_REASON: &str =
            "AI search contract evidence is compiler-contracted, but executable calls require authenticated PDS AI search evidence, approved gateway/auth evidence, and the shared SaaS HTTP executor.";
        let areas = vec![graduation_area_json(
            "contract_evidence",
            "AI Search Contract Evidence",
            CapabilityStatus::Unsupported(AI_SEARCH_UNSUPPORTED_REASON),
            AI_SEARCH_CONTRACT_EVIDENCE,
        )];
        return graduation_provider_json(provider, "ai_search", "ai_search", areas);
    }

    if provider.is_graph_read_provider() {
        let profile = graph_read_profile(provider);
        let areas = profile
            .capabilities
            .iter()
            .map(|capability| {
                graduation_area_json(
                    capability.area.key(),
                    capability.area.label(),
                    capability.status,
                    capability.evidence,
                )
            })
            .collect::<Vec<_>>();
        return graduation_provider_json(provider, "graph_read", "graph_read", areas);
    }

    if provider.is_external_api_provider() {
        let profile = saas_read_profile(provider);
        let areas = profile
            .capabilities
            .iter()
            .map(|capability| {
                graduation_area_json(
                    capability.area.key(),
                    capability.area.label(),
                    capability.status,
                    capability.evidence,
                )
            })
            .collect::<Vec<_>>();
        return graduation_provider_json(provider, "external_api", "saas_read", areas);
    }

    let profile = semantic_profile(provider);
    let areas = profile
        .capabilities
        .iter()
        .map(|capability| {
            graduation_area_json(
                capability.area.key(),
                capability.area.label(),
                capability.status,
                capability.evidence,
            )
        })
        .collect::<Vec<_>>();
    graduation_provider_json(provider, "database", "semantic_parity", areas)
}

fn graduation_provider_json(
    provider: FrameworkProvider,
    family: &'static str,
    profile: &'static str,
    areas: Vec<Value>,
) -> Value {
    let unsupported_capability_count = areas
        .iter()
        .filter(|area| area["status"] == json!("unsupported"))
        .count();
    let graduated_capability_count = areas
        .iter()
        .filter(|area| area["graduated"] == json!(true))
        .count();
    let promotion_violation_count = areas
        .iter()
        .filter(|area| area["promotion_violation"] == json!(true))
        .count();
    let ungraduated_capability_count = areas.len().saturating_sub(graduated_capability_count);
    let graduated_capabilities = areas
        .iter()
        .filter_map(|area| {
            if area["graduated"] == json!(true) {
                area["area"].as_str()
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    json!({
        "provider": provider.key(),
        "family": family,
        "profile": profile,
        "ok": promotion_violation_count == 0,
        "unsupported_capability_count": unsupported_capability_count,
        "ungraduated_capability_count": ungraduated_capability_count,
        "graduated_capability_count": graduated_capability_count,
        "promotion_violation_count": promotion_violation_count,
        "graduated_capabilities": graduated_capabilities,
        "areas": areas,
    })
}

fn graduation_summary(providers: &[Value]) -> Value {
    let provider_count = providers.len();
    let unsupported_capability_count =
        sum_provider_count(providers, "unsupported_capability_count");
    let ungraduated_capability_count =
        sum_provider_count(providers, "ungraduated_capability_count");
    let graduated_capability_count = sum_provider_count(providers, "graduated_capability_count");
    let promotion_violation_count = sum_provider_count(providers, "promotion_violation_count");

    json!({
        "provider_count": provider_count,
        "unsupported_capability_count": unsupported_capability_count,
        "ungraduated_capability_count": ungraduated_capability_count,
        "graduated_capability_count": graduated_capability_count,
        "promotion_violation_count": promotion_violation_count,
    })
}

fn sum_provider_count(providers: &[Value], field: &str) -> u64 {
    providers
        .iter()
        .map(|provider| provider[field].as_u64().unwrap_or(0))
        .sum()
}

fn graduation_area_json(
    area: &'static str,
    label: &'static str,
    status: CapabilityStatus,
    evidence: &'static [CertificationEvidence],
) -> Value {
    let (compiler_contracts, live_contracts) = split_evidence(evidence);
    let graduated = match status {
        CapabilityStatus::LiveCertified => !live_contracts.is_empty(),
        CapabilityStatus::CompilerContracted => !compiler_contracts.is_empty(),
        CapabilityStatus::Implemented(_)
        | CapabilityStatus::Partial(_)
        | CapabilityStatus::Unsupported(_)
        | CapabilityStatus::EmulatorLimited(_) => false,
    };
    let promotion_violation = match status {
        CapabilityStatus::LiveCertified => live_contracts.is_empty(),
        CapabilityStatus::CompilerContracted => compiler_contracts.is_empty(),
        CapabilityStatus::Implemented(_)
        | CapabilityStatus::Partial(_)
        | CapabilityStatus::Unsupported(_)
        | CapabilityStatus::EmulatorLimited(_) => false,
    };

    let mut value = json!({
        "area": area,
        "label": label,
        "status": status.label(),
        "graduated": graduated,
        "ok": !promotion_violation,
        "promotion_violation": promotion_violation,
        "compiler_contracts": compiler_contracts,
        "live_contracts": live_contracts,
        "required_evidence": required_graduation_evidence(status),
    });

    if let Some(reason) = status.reason() {
        value["reason"] = json!(reason);
    }

    value
}

fn required_graduation_evidence(status: CapabilityStatus) -> Vec<&'static str> {
    match status {
        CapabilityStatus::LiveCertified => vec!["live-contract"],
        CapabilityStatus::CompilerContracted => vec!["compiler-contract"],
        CapabilityStatus::Implemented(_)
        | CapabilityStatus::Partial(_)
        | CapabilityStatus::Unsupported(_)
        | CapabilityStatus::EmulatorLimited(_) => Vec::new(),
    }
}

/// Render a graph-read capability the same way [`capability_json`] renders a
/// CRUD capability, so the graph matrix is data-driven from
/// [`crate::provider_capabilities::graph_read_profile`] rather than hand-rolled.
pub fn graph_read_capability_json(capability: &GraphReadCapability) -> Value {
    let (compiler_contracts, live_contracts) = split_evidence(capability.evidence);
    let live_result = if live_contracts.is_empty() {
        "not-required"
    } else {
        "not-run"
    };

    let mut value = json!({
        "area": capability.area.key(),
        "label": capability.area.label(),
        "status": capability.status.label(),
        "live_result": live_result,
        "compiler_contracts": compiler_contracts,
        "live_contracts": live_contracts,
    });

    if let Some(reason) = capability.status.reason() {
        value["reason"] = json!(reason);
    }

    value
}

/// Render a SaaS/external-API capability the same way [`capability_json`]
/// renders a CRUD capability, so the SaaS matrix is data-driven from
/// [`crate::provider_capabilities::saas_read_profile`].
pub fn saas_read_capability_json(capability: &SaasReadCapability) -> Value {
    let (compiler_contracts, live_contracts) = split_evidence(capability.evidence);
    let live_result = if live_contracts.is_empty() {
        "not-required"
    } else {
        "not-run"
    };

    let mut value = json!({
        "area": capability.area.key(),
        "label": capability.area.label(),
        "status": capability.status.label(),
        "live_result": live_result,
        "compiler_contracts": compiler_contracts,
        "live_contracts": live_contracts,
    });

    if let Some(reason) = capability.status.reason() {
        value["reason"] = json!(reason);
    }

    value
}

pub fn external_api_area_report_json(provider: FrameworkProvider) -> (Vec<Value>, bool) {
    let profile = saas_read_profile(provider);
    let areas = profile
        .capabilities
        .iter()
        .map(saas_read_capability_json)
        .collect();

    (areas, false)
}

pub fn provider_sdk_rules_json() -> Value {
    json!({
        "command": "provider-sdk-rules",
        "ok": true,
        "rule_sets": [
            {
                "name": "provider identity",
                "rules": [
                    "Add the provider enum value to config, backend schema metadata, database loader, and connection-security mapping.",
                    "Expose a stable provider key for CLI, certification, metrics, and logs."
                ]
            },
            {
                "name": "query semantics",
                "rules": [
                    "Compile only from QueryIR, filter AST, selection AST, sort AST, and aggregate plan structures.",
                    "Reject unsupported canonical operators before provider command generation.",
                    "Keep access filters and tenant filters conjoined with user filters."
                ]
            },
            {
                "name": "security and errors",
                "rules": [
                    "Use bound parameters or provider-native structured predicates for every user value.",
                    "Normalize duplicate, foreign-key, required-field, stale-version, denied-access, invalid-filter, and unknown-provider errors.",
                    "Redact provider commands, bind values, credentials, and tenant-sensitive payloads in logs and diagnostics."
                ]
            },
            {
                "name": "observability and operations",
                "rules": [
                    "Emit request/correlation IDs, provider timing metrics, query-count/result-count fields, and slow-query events.",
                    "Implement health/readiness checks and pool stats, or declare an explicit unsupported reason.",
                    "Provider EXPLAIN hooks must default to unsupported until redaction is proven safe."
                ]
            },
            {
                "name": "certification",
                "rules": [
                    "Declare every ProviderContractArea with LiveCertified, CompilerContracted, Implemented, Partial, Unsupported, or EmulatorLimited.",
                    "Back every LiveCertified area with executable live contracts and every CompilerContracted area with compiler tests.",
                    "Run scripts/appfw provider-test --provider <provider> --json and scripts/appfw release-check --json before release."
                ]
            }
        ],
        "required_commands": [
            "scripts/appfw validate --json",
            "scripts/appfw test",
            "scripts/appfw explain provider-sdk --json",
            "scripts/appfw provider-test --provider <provider> --json",
            "scripts/appfw release-check --json"
        ],
        "owned_files": [
            "backend/src/data/clients/<provider>/",
            "appfw_runtime/src/provider_capabilities.rs",
            "backend/src/data/clients/provider_error.rs",
            "api_tests/src/provider_semantic_contracts.rs",
            "database/src/loader.rs",
            "app_gen/src/config_contract.rs"
        ]
    })
}

pub fn capability_json(capability: &ProviderCapability) -> Value {
    let (compiler_contracts, live_contracts) = evidence_contracts(capability);

    let mut value = json!({
        "area": capability.area.key(),
        "label": capability.area.label(),
        "status": capability.status.label(),
        "compiler_contracts": compiler_contracts,
        "live_contracts": live_contracts,
    });

    if let Some(reason) = capability.status.reason() {
        value["reason"] = json!(reason);
    }

    value
}

pub fn area_report_json(provider: FrameworkProvider, log: &str) -> (Vec<Value>, bool) {
    let profile = semantic_profile(provider);
    let mut failed = false;
    let areas = ProviderContractArea::ALL
        .into_iter()
        .map(|area| {
            let capability = profile
                .capabilities
                .iter()
                .find(|capability| capability.area == area)
                .expect("semantic profiles must declare every contract area");
            let (compiler_contracts, live_contracts) = evidence_contracts(capability);
            let live_contract = live_contracts.first().copied();
            let result = live_contract
                .map(|contract| contract_result(log, contract))
                .unwrap_or(ContractResult::NotRequired);
            let ok = area_ok(capability.status, result);
            failed |= !ok;

            let mut value = json!({
                "area": area.key(),
                "status": capability.status.label(),
                "ok": ok,
                "live_result": result.label(),
                "required_live_result": required_live_result(capability.status),
                "compiler_contracts": compiler_contracts,
                "live_contracts": live_contracts,
            });
            if let Some(contract) = live_contract {
                value["live_contract"] = json!(contract);
            }
            if let Some(reason) = capability.status.reason() {
                value["reason"] = json!(reason);
            }
            value
        })
        .collect();

    (areas, failed)
}

fn evidence_contracts(capability: &ProviderCapability) -> (Vec<&'static str>, Vec<&'static str>) {
    split_evidence(capability.evidence)
}

/// Split a slice of evidence into (compiler-contract names, live-contract names).
/// Shared by the CRUD and graph-read capability renderers.
fn split_evidence(evidence: &[CertificationEvidence]) -> (Vec<&'static str>, Vec<&'static str>) {
    let compiler_contracts = evidence
        .iter()
        .filter_map(|evidence| match evidence {
            CertificationEvidence::CompilerContract(contract) => Some(*contract),
            CertificationEvidence::LiveContract(_) => None,
        })
        .collect();
    let live_contracts = evidence
        .iter()
        .filter_map(|evidence| match evidence {
            CertificationEvidence::LiveContract(contract) => Some(*contract),
            CertificationEvidence::CompilerContract(_) => None,
        })
        .collect();

    (compiler_contracts, live_contracts)
}

pub fn contract_result(log: &str, contract: &str) -> ContractResult {
    if log.contains(&format!("test {contract} ... ok")) {
        ContractResult::Passed
    } else if log.contains(&format!("test {contract} ... FAILED")) {
        ContractResult::Failed
    } else if log.contains(&format!("{contract}: test")) {
        ContractResult::Listed
    } else {
        ContractResult::NotRun
    }
}

pub fn area_ok(status: CapabilityStatus, result: ContractResult) -> bool {
    match status {
        CapabilityStatus::LiveCertified => result == ContractResult::Passed,
        CapabilityStatus::EmulatorLimited(_) => result != ContractResult::Failed,
        CapabilityStatus::CompilerContracted
        | CapabilityStatus::Implemented(_)
        | CapabilityStatus::Partial(_)
        | CapabilityStatus::Unsupported(_) => true,
    }
}

pub fn required_live_result(status: CapabilityStatus) -> &'static str {
    match status {
        CapabilityStatus::LiveCertified => ContractResult::Passed.label(),
        CapabilityStatus::EmulatorLimited(_) => "not-failed",
        CapabilityStatus::CompilerContracted
        | CapabilityStatus::Implemented(_)
        | CapabilityStatus::Partial(_)
        | CapabilityStatus::Unsupported(_) => ContractResult::NotRequired.label(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_capabilities::SaasReadArea;

    #[test]
    fn live_certified_requires_executed_pass() {
        assert!(area_ok(
            CapabilityStatus::LiveCertified,
            ContractResult::Passed
        ));
        assert!(!area_ok(
            CapabilityStatus::LiveCertified,
            ContractResult::Listed
        ));
        assert!(!area_ok(
            CapabilityStatus::LiveCertified,
            ContractResult::NotRun
        ));
    }

    #[test]
    fn listed_provider_contracts_fail_the_area_report() {
        let listed_log = "provider_semantic_contracts::provider_scalar_query_pagination_projection_and_error_contract: test\n\
            provider_semantic_contracts::provider_relationship_projection_contract: test\n\
            provider_contracts::aggregate_accounts_contract: test\n\
            provider_semantic_contracts::provider_stale_delete_concurrency_contract: test\n";
        let (areas, failed) = area_report_json(FrameworkProvider::Postgres, listed_log);

        assert!(failed);
        assert!(areas.iter().any(|area| {
            area["area"] == "scalar_filters"
                && area["status"] == "live-certified"
                && area["live_result"] == "listed"
                && area["ok"] == false
        }));
    }

    #[test]
    fn area_report_exports_declared_evidence_for_release_artifacts() {
        let passed_log = "test provider_semantic_contracts::provider_scalar_query_pagination_projection_and_error_contract ... ok";
        let (areas, failed) = area_report_json(FrameworkProvider::Postgres, passed_log);

        assert!(failed);
        let scalar_filters = areas
            .iter()
            .find(|area| area["area"] == "scalar_filters")
            .expect("scalar filters area");

        assert_eq!(scalar_filters["status"], "live-certified");
        assert_eq!(scalar_filters["live_result"], "passed");
        assert_eq!(scalar_filters["required_live_result"], "passed");
        assert_eq!(
            scalar_filters["live_contract"],
            "provider_semantic_contracts::provider_scalar_query_pagination_projection_and_error_contract"
        );
        assert_eq!(
            scalar_filters["compiler_contracts"],
            json!(["data::clients::contract_tests::filter_dsl_conformance_contract"])
        );
        assert_eq!(
            scalar_filters["live_contracts"],
            json!(["provider_semantic_contracts::provider_scalar_query_pagination_projection_and_error_contract"])
        );
    }

    #[test]
    fn not_run_live_provider_contracts_fail_with_explicit_release_expectation() {
        let (areas, failed) = area_report_json(FrameworkProvider::Postgres, "");

        assert!(failed);
        let scalar_filters = areas
            .iter()
            .find(|area| area["area"] == "scalar_filters")
            .expect("scalar filters area");

        assert_eq!(scalar_filters["status"], "live-certified");
        assert_eq!(scalar_filters["ok"], false);
        assert_eq!(scalar_filters["live_result"], "not-run");
        assert_eq!(scalar_filters["required_live_result"], "passed");
        assert_eq!(
            scalar_filters["live_contract"],
            "provider_semantic_contracts::provider_scalar_query_pagination_projection_and_error_contract"
        );
    }

    #[test]
    fn emulator_limited_areas_report_non_failing_live_expectation() {
        let (areas, failed) = area_report_json(FrameworkProvider::Snowflake, "");

        assert!(failed);
        let aggregation = areas
            .iter()
            .find(|area| area["area"] == "aggregation")
            .expect("aggregation area");

        assert_eq!(aggregation["status"], "emulator-limited");
        assert_eq!(aggregation["ok"], true);
        assert_eq!(aggregation["live_result"], "not-run");
        assert_eq!(aggregation["required_live_result"], "not-failed");
    }

    #[test]
    fn neo4j_provider_matrix_reports_graph_read_category() {
        let matrix = provider_matrix_json(Some(FrameworkProvider::Neo4j));

        assert_eq!(matrix["providers"][0]["provider"], "neo4j");
        assert_eq!(matrix["providers"][0]["category"], "graph_read");
        assert_eq!(
            matrix["providers"][0]["database_semantic_parity"],
            "not_applicable"
        );
        assert_eq!(matrix["providers"][0]["live_certification"], "pending");

        let areas = matrix["providers"][0]["graph_read_areas"]
            .as_array()
            .expect("graph_read_areas array");
        // Data-driven from graph_read_profile: 11 areas (9 read + 2 governed-write).
        assert_eq!(areas.len(), 11);
        let names: Vec<&str> = areas
            .iter()
            .map(|area| area["area"].as_str().expect("area name"))
            .collect();
        for expected in [
            "tenant_isolation",
            "traversal_limits",
            "query_metrics_and_audit",
            "projection_freshness",
            "governed_write_enforcement",
            "write_metrics_and_audit",
        ] {
            assert!(names.contains(&expected), "missing graph area {expected}");
        }
        // Each area renders with the same data-driven shape as the CRUD matrix.
        let first = &areas[0];
        assert_eq!(first["status"], "compiler-contracted");
        assert!(first["label"].is_string());
        assert!(!first["compiler_contracts"]
            .as_array()
            .expect("compiler_contracts array")
            .is_empty());
    }

    #[test]
    fn external_api_provider_matrix_reports_saas_category() {
        for provider in FrameworkProvider::EXTERNAL_API {
            let matrix = provider_matrix_json(Some(provider));

            assert_eq!(matrix["providers"][0]["provider"], provider.key());
            assert_eq!(matrix["providers"][0]["category"], "external_api");
            assert_eq!(
                matrix["providers"][0]["database_semantic_parity"],
                "not_applicable"
            );
            assert_eq!(matrix["providers"][0]["live_certification"], "pending");

            let areas = matrix["providers"][0]["saas_read_areas"]
                .as_array()
                .expect("saas_read_areas array");
            assert_eq!(areas.len(), SaasReadArea::ALL.len());
            let names: Vec<&str> = areas
                .iter()
                .map(|area| area["area"].as_str().expect("area name"))
                .collect();
            for expected in [
                "connection_auth",
                "named_operation_registry",
                "request_binding",
                "pagination_cursoring",
                "incremental_watermark",
                "field_redaction",
                "tenant_scoping",
                "freshness_reporting",
                "governed_write_enforcement",
                "delegated_actor_context",
                "token_store_isolation",
                "named_mutation_registry",
                "mutation_request_binding",
                "idempotency_and_replay_protection",
                "write_policy_and_scope_enforcement",
                "write_audit_and_evidence",
            ] {
                assert!(names.contains(&expected), "missing SaaS area {expected}");
            }

            if matches!(
                provider,
                FrameworkProvider::ServiceNow | FrameworkProvider::Icims
            ) {
                assert!(areas.iter().all(|area| area["status"] == "unsupported"));
                assert!(areas.iter().all(|area| area["compiler_contracts"]
                    .as_array()
                    .expect("compiler_contracts array")
                    .is_empty()));
                continue;
            }

            let named_registry = areas
                .iter()
                .find(|area| area["area"] == "named_operation_registry")
                .expect("named registry area");
            assert_eq!(named_registry["status"], "compiler-contracted");
            assert!(!named_registry["compiler_contracts"]
                .as_array()
                .expect("compiler_contracts array")
                .is_empty());
            assert!(named_registry["live_contracts"]
                .as_array()
                .expect("live_contracts array")
                .is_empty());

            let connection_auth = areas
                .iter()
                .find(|area| area["area"] == "connection_auth")
                .expect("connection auth area");
            assert_eq!(connection_auth["status"], "unsupported");
            assert!(connection_auth["reason"]
                .as_str()
                .expect("reason")
                .contains("live tenant evidence"));
            assert!(connection_auth["compiler_contracts"]
                .as_array()
                .expect("compiler_contracts array")
                .is_empty());

            let governed_write = areas
                .iter()
                .find(|area| area["area"] == "governed_write_enforcement")
                .expect("governed write area");
            assert_eq!(governed_write["status"], "unsupported");
            assert!(governed_write["reason"]
                .as_str()
                .expect("reason")
                .contains("writes remain unsupported"));
            assert!(!governed_write["compiler_contracts"]
                .as_array()
                .expect("compiler_contracts array")
                .is_empty());
            assert!(governed_write["live_contracts"]
                .as_array()
                .expect("live_contracts array")
                .is_empty());
        }
    }

    #[test]
    fn provider_graduation_reports_ai_search_as_planned_family() {
        let report = provider_graduation_json();
        let providers = report["providers"].as_array().expect("providers array");
        let ai_search = providers
            .iter()
            .find(|provider| provider["provider"] == "ai_search")
            .expect("ai_search provider graduation entry");

        assert_eq!(report["ok"], true);
        assert_eq!(ai_search["family"], "ai_search");
        assert_eq!(ai_search["profile"], "ai_search");
        assert_eq!(ai_search["ok"], true);
        assert_eq!(ai_search["unsupported_capability_count"], 1);
        assert_eq!(ai_search["graduated_capability_count"], 0);
        assert_eq!(ai_search["promotion_violation_count"], 0);
        assert_eq!(ai_search["areas"][0]["area"], "contract_evidence");
        assert_eq!(ai_search["areas"][0]["status"], "unsupported");
        assert!(ai_search["areas"][0]["reason"]
            .as_str()
            .expect("reason")
            .contains("gateway/auth evidence"));
        assert_eq!(ai_search["areas"][0]["graduated"], false);
        assert!(!ai_search["areas"][0]["compiler_contracts"]
            .as_array()
            .expect("compiler contracts array")
            .is_empty());
    }

    #[test]
    fn provider_matrix_reports_ai_search_as_non_crud_contract_family() {
        let matrix = provider_matrix_json(Some(FrameworkProvider::AiSearch));
        let provider = &matrix["providers"][0];

        assert_eq!(provider["provider"], "ai_search");
        assert_eq!(provider["category"], "ai_search");
        assert_eq!(provider["database_semantic_parity"], "not_applicable");
        assert_eq!(provider["ai_search_areas"][0]["area"], "contract_evidence");
        assert!(provider["ai_search_areas"][0]["reason"]
            .as_str()
            .expect("reason")
            .contains("gateway/auth evidence"));
        assert!(!provider["ai_search_areas"][0]["compiler_contracts"]
            .as_array()
            .expect("compiler contracts array")
            .is_empty());
    }
}
