//! Shared SaaS provider certification testkit contracts.
//!
//! This crate intentionally stays transport-free. Runtime/provider crates own
//! live execution; the testkit owns stable report shapes and fail-closed plan
//! semantics for SaaS read certification.

use serde::{Deserialize, Serialize};

pub const SAAS_READ_PLAN_SCHEMA_VERSION: &str = "appfw.saas_read_provider_test.plan.v1";
pub const SAAS_READ_PLAN_ARTIFACT: &str = "target/appfw/saas-read-provider-test.json";
pub const SAAS_READ_EVIDENCE_ARTIFACT: &str = "target/appfw/saas-read-evidence.json";

pub const SAAS_READ_AREAS: &[&str] = &[
    "connection_auth",
    "named_operation_registry",
    "request_binding",
    "pagination_cursoring",
    "rate_limit_backoff",
    "incremental_watermark",
    "field_redaction",
    "tenant_scoping",
    "schema_version_pinning",
    "result_and_timeout_caps",
    "query_metrics_and_audit",
    "freshness_reporting",
];

pub const SAAS_WRITE_AREAS: &[&str] = &[
    "governed_write_enforcement",
    "delegated_actor_context",
    "token_store_isolation",
    "named_mutation_registry",
    "mutation_request_binding",
    "idempotency_and_replay_protection",
    "write_policy_and_scope_enforcement",
    "write_audit_and_evidence",
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaasReadAreaEvidence {
    pub area: String,
    pub label: String,
    pub status: String,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub compiler_contracts: Vec<String>,
    #[serde(default)]
    pub live_contracts: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaasReadProviderTestPlan {
    pub command: String,
    pub lane: String,
    pub schema_version: String,
    pub area: String,
    pub provider: String,
    pub provider_family: String,
    pub mode: String,
    pub plan_only: bool,
    pub ok: bool,
    pub release_ready: bool,
    pub artifact: String,
    pub evidence_artifact: String,
    pub required_checks: Vec<String>,
    pub areas: Vec<SaasReadAreaEvidence>,
}

impl SaasReadProviderTestPlan {
    pub fn validate_shape(&self) -> Result<(), String> {
        if self.command != "provider-test" {
            return Err("command must be provider-test".to_string());
        }
        if self.lane != "DP6" {
            return Err("lane must be DP6".to_string());
        }
        if self.schema_version != SAAS_READ_PLAN_SCHEMA_VERSION {
            return Err(format!(
                "schema_version must be {SAAS_READ_PLAN_SCHEMA_VERSION}"
            ));
        }
        if self.area != "saas_read" {
            return Err("area must be saas_read".to_string());
        }
        if self.provider_family != "external_api" {
            return Err("provider_family must be external_api".to_string());
        }
        if self.mode != "plan" {
            return Err("mode must be plan".to_string());
        }
        if !self.plan_only {
            return Err("plan_only must be true".to_string());
        }
        if !self.ok {
            return Err("ok must be true for plan mode".to_string());
        }
        if self.release_ready {
            return Err("saas-read plan mode must not be release_ready".to_string());
        }
        if self.artifact != SAAS_READ_PLAN_ARTIFACT {
            return Err(format!("artifact must be {SAAS_READ_PLAN_ARTIFACT}"));
        }
        if self.evidence_artifact != SAAS_READ_EVIDENCE_ARTIFACT {
            return Err(format!(
                "evidence_artifact must be {SAAS_READ_EVIDENCE_ARTIFACT}"
            ));
        }
        if self.required_checks != SAAS_READ_AREAS {
            return Err("required_checks must match the SaaS read area set".to_string());
        }

        let mut seen = std::collections::HashSet::new();
        for area in &self.areas {
            if !seen.insert(area.area.as_str()) {
                return Err(format!("duplicate SaaS read area `{}`", area.area));
            }
            if SAAS_WRITE_AREAS.contains(&area.area.as_str()) {
                return Err(format!(
                    "SaaS read plan must not include write area `{}`",
                    area.area
                ));
            }
        }

        for expected in SAAS_READ_AREAS {
            if !seen.contains(expected) {
                return Err(format!("missing SaaS read area `{expected}`"));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_plan() -> SaasReadProviderTestPlan {
        SaasReadProviderTestPlan {
            command: "provider-test".to_string(),
            lane: "DP6".to_string(),
            schema_version: SAAS_READ_PLAN_SCHEMA_VERSION.to_string(),
            area: "saas_read".to_string(),
            provider: "salesforce".to_string(),
            provider_family: "external_api".to_string(),
            mode: "plan".to_string(),
            plan_only: true,
            ok: true,
            release_ready: false,
            artifact: SAAS_READ_PLAN_ARTIFACT.to_string(),
            evidence_artifact: SAAS_READ_EVIDENCE_ARTIFACT.to_string(),
            required_checks: SAAS_READ_AREAS
                .iter()
                .map(|area| area.to_string())
                .collect(),
            areas: SAAS_READ_AREAS
                .iter()
                .map(|area| SaasReadAreaEvidence {
                    area: (*area).to_string(),
                    label: area.replace('_', " "),
                    status: "compiler-contracted".to_string(),
                    reason: None,
                    compiler_contracts: Vec::new(),
                    live_contracts: Vec::new(),
                })
                .collect(),
        }
    }

    #[test]
    fn valid_plan_shape_passes() {
        valid_plan().validate_shape().expect("valid SaaS read plan");
    }

    #[test]
    fn write_areas_are_not_saas_read_checks() {
        let mut plan = valid_plan();
        plan.areas.push(SaasReadAreaEvidence {
            area: "named_mutation_registry".to_string(),
            label: "named mutation registry".to_string(),
            status: "unsupported".to_string(),
            reason: Some("write area".to_string()),
            compiler_contracts: Vec::new(),
            live_contracts: Vec::new(),
        });

        let err = plan.validate_shape().unwrap_err();
        assert!(err.contains("must not include write area"));
    }

    #[test]
    fn release_ready_is_for_live_evidence_not_plan_mode() {
        let mut plan = valid_plan();
        plan.release_ready = true;

        let err = plan.validate_shape().unwrap_err();
        assert!(err.contains("must not be release_ready"));
    }
}
