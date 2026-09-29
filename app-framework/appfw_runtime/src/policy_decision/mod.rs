use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::principal::AuthoritativeSubject;

pub const POLICY_DECISION_CONTRACT_VERSION: &str = "policy_decision@1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionEffect {
    Allow,
    Deny,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EntitlementAuthorityKind {
    SourceNative,
    Rebac,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldMaskTreatment {
    Omit,
    Redact,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthoritativeObject {
    pub source: String,
    pub object_type: String,
    pub object_id: String,
    pub object_version: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EntitlementProvenance {
    pub authority_kind: EntitlementAuthorityKind,
    pub authority: String,
    pub entitlement_version: String,
    pub evidence_ref: String,
    pub reconciled_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DynamicFieldMask {
    pub field: String,
    pub treatment: FieldMaskTreatment,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PermissionDecision {
    pub contract_version: String,
    pub decision_id: String,
    pub effect: DecisionEffect,
    pub subject: AuthoritativeSubject,
    pub object: AuthoritativeObject,
    pub tenant_id: String,
    pub row_filter: Value,
    #[serde(default)]
    pub field_masks: Vec<DynamicFieldMask>,
    #[serde(default)]
    pub allowed_actions: Vec<String>,
    #[serde(default)]
    pub obligations: Vec<String>,
    pub reason_code: String,
    pub policy_version: String,
    pub schema_version: String,
    pub decided_at: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub freshness_slo_seconds: u64,
    pub entitlement: EntitlementProvenance,
    pub audit_id: String,
    pub correlation_id: String,
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum PolicyDecisionError {
    #[error("unsupported policy-decision contract version `{0}`")]
    UnsupportedVersion(String),
    #[error("policy-decision contract is invalid: {0}")]
    InvalidContract(&'static str),
    #[error("policy decision has an invalid timestamp sequence")]
    InvalidTimeline,
    #[error("policy decision was revoked")]
    Revoked,
    #[error("policy decision is stale or expired")]
    Stale,
    #[error("entitlement provenance has not been reconciled")]
    Unreconciled,
    #[error("{0} version does not match the consuming contract")]
    VersionMismatch(&'static str),
}

impl PermissionDecision {
    pub fn validate_at(
        &self,
        now: DateTime<Utc>,
        expected_policy_version: &str,
        expected_schema_version: &str,
    ) -> Result<(), PolicyDecisionError> {
        if self.contract_version != POLICY_DECISION_CONTRACT_VERSION {
            return Err(PolicyDecisionError::UnsupportedVersion(
                self.contract_version.clone(),
            ));
        }
        validate_required_fields(self)?;
        if self.policy_version != expected_policy_version {
            return Err(PolicyDecisionError::VersionMismatch("policy"));
        }
        if self.schema_version != expected_schema_version {
            return Err(PolicyDecisionError::VersionMismatch("schema"));
        }
        let reconciled_at = self
            .entitlement
            .reconciled_at
            .ok_or(PolicyDecisionError::Unreconciled)?;
        if self.decided_at > now
            || self.decided_at > self.valid_until
            || reconciled_at > self.decided_at
        {
            return Err(PolicyDecisionError::InvalidTimeline);
        }
        if self.revoked_at.is_some() {
            return Err(PolicyDecisionError::Revoked);
        }
        if self.valid_until <= now
            || now.signed_duration_since(reconciled_at).num_seconds()
                > self.freshness_slo_seconds as i64
        {
            return Err(PolicyDecisionError::Stale);
        }
        Ok(())
    }
}

fn validate_required_fields(decision: &PermissionDecision) -> Result<(), PolicyDecisionError> {
    let required = [
        &decision.decision_id,
        &decision.subject.issuer,
        &decision.subject.subject_id,
        &decision.object.source,
        &decision.object.object_type,
        &decision.object.object_id,
        &decision.object.object_version,
        &decision.tenant_id,
        &decision.reason_code,
        &decision.policy_version,
        &decision.schema_version,
        &decision.entitlement.authority,
        &decision.entitlement.entitlement_version,
        &decision.entitlement.evidence_ref,
        &decision.audit_id,
        &decision.correlation_id,
    ];
    if required.into_iter().any(|value| !valid_token(value)) {
        return Err(PolicyDecisionError::InvalidContract("required_string"));
    }
    if decision.freshness_slo_seconds == 0 {
        return Err(PolicyDecisionError::InvalidContract(
            "freshness_slo_seconds",
        ));
    }
    let Some(filter) = decision.row_filter.as_object() else {
        return Err(PolicyDecisionError::InvalidContract("row_filter"));
    };
    if filter.is_empty() {
        return Err(PolicyDecisionError::InvalidContract("row_filter"));
    }
    validate_unique(
        "field_masks",
        decision.field_masks.iter().map(|mask| &mask.field),
    )?;
    validate_unique("allowed_actions", decision.allowed_actions.iter())?;
    validate_unique("obligations", decision.obligations.iter())?;
    if decision.effect == DecisionEffect::Deny && !decision.allowed_actions.is_empty() {
        return Err(PolicyDecisionError::InvalidContract("deny_allowed_actions"));
    }
    Ok(())
}

fn validate_unique<'a>(
    field: &'static str,
    values: impl Iterator<Item = &'a String>,
) -> Result<(), PolicyDecisionError> {
    let mut seen = HashSet::new();
    for value in values {
        if !valid_token(value) || !seen.insert(value) {
            return Err(PolicyDecisionError::InvalidContract(field));
        }
    }
    Ok(())
}

fn valid_token(value: &str) -> bool {
    !value.trim().is_empty() && !value.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone};
    use serde_json::json;

    use super::*;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 10, 14, 0, 0).unwrap()
    }

    fn decision() -> PermissionDecision {
        PermissionDecision {
            contract_version: POLICY_DECISION_CONTRACT_VERSION.to_string(),
            decision_id: "decision-1".to_string(),
            effect: DecisionEffect::Allow,
            subject: AuthoritativeSubject {
                issuer: "https://identity.example.test".to_string(),
                subject_id: "casey".to_string(),
            },
            object: AuthoritativeObject {
                source: "servicenow".to_string(),
                object_type: "task".to_string(),
                object_id: "task-1".to_string(),
                object_version: "7".to_string(),
            },
            tenant_id: "tenant-a".to_string(),
            row_filter: json!({ "tenant_id": { "_eq": "tenant-a" } }),
            field_masks: vec![DynamicFieldMask {
                field: "private_notes".to_string(),
                treatment: FieldMaskTreatment::Omit,
            }],
            allowed_actions: vec!["task.read".to_string()],
            obligations: vec!["audit_read".to_string()],
            reason_code: "source_acl_and_context_allow".to_string(),
            policy_version: "policy-7".to_string(),
            schema_version: "task-3".to_string(),
            decided_at: now(),
            valid_until: now() + Duration::minutes(5),
            revoked_at: None,
            freshness_slo_seconds: 300,
            entitlement: EntitlementProvenance {
                authority_kind: EntitlementAuthorityKind::SourceNative,
                authority: "servicenow-acl".to_string(),
                entitlement_version: "acl-19".to_string(),
                evidence_ref: "evidence://acl/19".to_string(),
                reconciled_at: Some(now()),
            },
            audit_id: "audit-1".to_string(),
            correlation_id: "correlation-1".to_string(),
        }
    }

    #[test]
    fn accepts_current_source_authoritative_decision() {
        assert_eq!(decision().validate_at(now(), "policy-7", "task-3"), Ok(()));
    }

    fn error(value: &PermissionDecision, at: DateTime<Utc>) -> PolicyDecisionError {
        value.validate_at(at, "policy-7", "task-3").unwrap_err()
    }

    #[test]
    fn rejects_missing_filter_or_unreconciled_entitlement() {
        let mut value = decision();
        value.row_filter = json!({});
        assert_eq!(
            error(&value, now()),
            PolicyDecisionError::InvalidContract("row_filter")
        );
        value.row_filter = json!({ "tenant_id": "tenant-a" });
        value.entitlement.reconciled_at = None;
        assert_eq!(error(&value, now()), PolicyDecisionError::Unreconciled);
    }

    #[test]
    fn rejects_stale_revoked_or_version_mismatched_decision() {
        let mut value = decision();
        assert_eq!(
            value.validate_at(now(), "policy-8", "task-3").unwrap_err(),
            PolicyDecisionError::VersionMismatch("policy")
        );
        value.revoked_at = Some(now());
        assert_eq!(error(&value, now()), PolicyDecisionError::Revoked);
        value.revoked_at = None;
        assert_eq!(
            error(&value, now() + Duration::minutes(6)),
            PolicyDecisionError::Stale
        );
    }

    #[test]
    fn rejects_denial_that_exposes_an_action() {
        let mut value = decision();
        value.effect = DecisionEffect::Deny;
        assert_eq!(
            error(&value, now()),
            PolicyDecisionError::InvalidContract("deny_allowed_actions")
        );
    }
}
