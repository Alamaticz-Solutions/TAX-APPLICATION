use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::{
    policy_decision::{
        DecisionEffect, FieldMaskTreatment, PermissionDecision, PolicyDecisionError,
    },
    principal::{PermissionedPrincipal, PrincipalValidationError},
};

pub const ARCHETYPE_CONTRACT_VERSION: &str = "permissioned_archetype@1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArchetypeModel {
    pub contract_version: String,
    pub archetype_id: String,
    pub schema_version: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArchetypeEvidenceRef {
    pub kind: String,
    pub uri: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArchetypeTelemetry {
    pub correlation_id: String,
    pub traceparent: String,
    pub channel: String,
    pub journey_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ArchetypeContext {
    pub model: ArchetypeModel,
    pub principal: PermissionedPrincipal,
    pub policy: PermissionDecision,
    pub telemetry: ArchetypeTelemetry,
    pub evidence: Vec<ArchetypeEvidenceRef>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CanonicalRead {
    pub context: ArchetypeContext,
    pub observed_at: DateTime<Utc>,
    pub payload: Value,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionMode {
    Preview,
    Execute,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CanonicalAction {
    pub context: ArchetypeContext,
    pub operation_id: String,
    pub operation_version: String,
    pub expected_object_version: String,
    pub idempotency_key: String,
    pub mode: ActionMode,
    pub arguments: Value,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CanonicalEvent {
    pub context: ArchetypeContext,
    pub event_id: String,
    pub event_type: String,
    pub event_version: String,
    pub occurred_at: DateTime<Utc>,
    pub observed_at: DateTime<Utc>,
    pub causation_id: Option<String>,
    pub payload_digest: String,
    pub release_identity: String,
    pub disposition: EventDisposition,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventDisposition {
    Observed,
    Authorized,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConformancePath {
    Read,
    Action,
    Event,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArchetypeConformanceCase {
    pub case_id: String,
    pub channel: String,
    pub path: ConformancePath,
    pub expected_effect: DecisionEffect,
    pub expected_reason_code: String,
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum ArchetypeContractError {
    #[error("archetype contract is invalid: {0}")]
    Invalid(&'static str),
    #[error("principal contract rejected the archetype context: {0}")]
    Principal(#[from] PrincipalValidationError),
    #[error("policy decision rejected the archetype context: {0}")]
    Policy(#[from] PolicyDecisionError),
    #[error("policy decision denied this canonical path")]
    Denied,
}

impl ArchetypeContext {
    pub fn validate_at(
        &self,
        now: DateTime<Utc>,
        expected_policy_version: &str,
    ) -> Result<(), ArchetypeContractError> {
        if self.model.contract_version != ARCHETYPE_CONTRACT_VERSION {
            return Err(ArchetypeContractError::Invalid("contract_version"));
        }
        for value in [&self.model.archetype_id, &self.model.schema_version] {
            validate_token(value, "model")?;
        }
        self.principal.validate_at(now)?;
        self.policy
            .validate_at(now, expected_policy_version, &self.model.schema_version)?;
        if self.principal.tenant_id != self.policy.tenant_id {
            return Err(ArchetypeContractError::Invalid("tenant_mismatch"));
        }
        if self.principal.effective_subject() != &self.policy.subject {
            return Err(ArchetypeContractError::Invalid("subject_mismatch"));
        }
        validate_token(&self.telemetry.correlation_id, "correlation_id")?;
        validate_token(&self.telemetry.traceparent, "traceparent")?;
        validate_token(&self.telemetry.channel, "channel")?;
        if self.telemetry.correlation_id != self.policy.correlation_id {
            return Err(ArchetypeContractError::Invalid("correlation_mismatch"));
        }
        if self.evidence.is_empty() {
            return Err(ArchetypeContractError::Invalid("evidence"));
        }
        for evidence in &self.evidence {
            validate_token(&evidence.kind, "evidence.kind")?;
            validate_token(&evidence.uri, "evidence.uri")?;
            validate_token(&evidence.sha256, "evidence.sha256")?;
        }
        Ok(())
    }
}

impl CanonicalRead {
    pub fn validate_at(
        &self,
        now: DateTime<Utc>,
        policy_version: &str,
    ) -> Result<(), ArchetypeContractError> {
        self.context.validate_at(now, policy_version)?;
        if self.context.policy.effect != DecisionEffect::Allow {
            return Err(ArchetypeContractError::Denied);
        }
        if !self.payload.is_object() || self.observed_at > now {
            return Err(ArchetypeContractError::Invalid("read"));
        }
        validate_read_payload(&self.context.policy, &self.payload)?;
        Ok(())
    }
}

impl CanonicalAction {
    pub fn validate_at(
        &self,
        now: DateTime<Utc>,
        policy_version: &str,
    ) -> Result<(), ArchetypeContractError> {
        self.context.validate_at(now, policy_version)?;
        for value in [
            &self.operation_id,
            &self.operation_version,
            &self.idempotency_key,
        ] {
            validate_token(value, "action")?;
        }
        if self.context.policy.effect != DecisionEffect::Allow
            || !self
                .context
                .policy
                .allowed_actions
                .contains(&self.operation_id)
            || !self.context.principal.allows_operation(&self.operation_id)
        {
            return Err(ArchetypeContractError::Denied);
        }
        if self.expected_object_version != self.context.policy.object.object_version
            || !self.arguments.is_object()
        {
            return Err(ArchetypeContractError::Invalid("action"));
        }
        Ok(())
    }
}

impl CanonicalEvent {
    pub fn validate_at(
        &self,
        now: DateTime<Utc>,
        policy_version: &str,
    ) -> Result<(), ArchetypeContractError> {
        self.context.validate_at(now, policy_version)?;
        for value in [
            &self.event_id,
            &self.event_type,
            &self.event_version,
            &self.payload_digest,
            &self.release_identity,
        ] {
            validate_token(value, "event")?;
        }
        if self.context.policy.effect != DecisionEffect::Allow {
            return Err(ArchetypeContractError::Denied);
        }
        if self.occurred_at > self.observed_at || self.observed_at > now {
            return Err(ArchetypeContractError::Invalid("event_timeline"));
        }
        if self.disposition == EventDisposition::Authorized
            && (!self
                .context
                .policy
                .allowed_actions
                .contains(&self.event_type)
                || !self.context.principal.allows_operation(&self.event_type))
        {
            return Err(ArchetypeContractError::Denied);
        }
        Ok(())
    }
}

fn validate_read_payload(
    policy: &PermissionDecision,
    payload: &Value,
) -> Result<(), ArchetypeContractError> {
    let payload = payload
        .as_object()
        .ok_or(ArchetypeContractError::Invalid("read"))?;
    let filter = policy
        .row_filter
        .as_object()
        .ok_or(ArchetypeContractError::Invalid("read.policy_enforcement"))?;
    for (field, predicate) in filter {
        let Some(expected) = predicate
            .as_object()
            .filter(|operator| operator.len() == 1)
            .and_then(|operator| operator.get("_eq"))
        else {
            return Err(ArchetypeContractError::Invalid("read.policy_enforcement"));
        };
        if payload.get(field) != Some(expected) {
            return Err(ArchetypeContractError::Invalid("read.policy_enforcement"));
        }
    }
    for mask in &policy.field_masks {
        let enforced = match mask.treatment {
            FieldMaskTreatment::Omit => !payload.contains_key(&mask.field),
            FieldMaskTreatment::Redact => {
                payload.get(&mask.field) == Some(&Value::String("[REDACTED]".into()))
            }
        };
        if !enforced {
            return Err(ArchetypeContractError::Invalid("read.policy_enforcement"));
        }
    }
    Ok(())
}

fn validate_token(value: &str, field: &'static str) -> Result<(), ArchetypeContractError> {
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        Err(ArchetypeContractError::Invalid(field))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone};
    use serde_json::json;

    use super::*;
    use crate::{
        policy_decision::{
            AuthoritativeObject, DynamicFieldMask, EntitlementAuthorityKind, EntitlementProvenance,
            POLICY_DECISION_CONTRACT_VERSION,
        },
        principal::{AuthoritativeSubject, BoundedServiceIdentity, PRINCIPAL_CONTRACT_VERSION},
        RuntimePrincipalType,
    };

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 10, 16, 0, 0).unwrap()
    }

    fn context() -> ArchetypeContext {
        let subject = AuthoritativeSubject {
            issuer: "https://identity.example.test".into(),
            subject_id: "casey".into(),
        };
        ArchetypeContext {
            model: ArchetypeModel {
                contract_version: ARCHETYPE_CONTRACT_VERSION.into(),
                archetype_id: "work_item".into(),
                schema_version: "work-item-1".into(),
            },
            principal: PermissionedPrincipal {
                contract_version: PRINCIPAL_CONTRACT_VERSION.into(),
                principal_type: RuntimePrincipalType::User,
                subject: subject.clone(),
                tenant_id: "tenant-a".into(),
                roles: vec![],
                scopes: vec!["work.read".into()],
                service_identity: None,
                on_behalf_of: None,
            },
            policy: PermissionDecision {
                contract_version: POLICY_DECISION_CONTRACT_VERSION.into(),
                decision_id: "decision-1".into(),
                effect: DecisionEffect::Allow,
                subject,
                object: AuthoritativeObject {
                    source: "source-system".into(),
                    object_type: "work_item".into(),
                    object_id: "work-1".into(),
                    object_version: "3".into(),
                },
                tenant_id: "tenant-a".into(),
                row_filter: json!({"tenant_id": {"_eq": "tenant-a"}}),
                field_masks: vec![DynamicFieldMask {
                    field: "private_note".into(),
                    treatment: FieldMaskTreatment::Omit,
                }],
                allowed_actions: vec!["work.preview".into()],
                obligations: vec!["audit".into()],
                reason_code: "source_acl_allow".into(),
                policy_version: "policy-1".into(),
                schema_version: "work-item-1".into(),
                decided_at: now(),
                valid_until: now() + Duration::minutes(5),
                revoked_at: None,
                freshness_slo_seconds: 300,
                entitlement: EntitlementProvenance {
                    authority_kind: EntitlementAuthorityKind::SourceNative,
                    authority: "source-acl".into(),
                    entitlement_version: "acl-1".into(),
                    evidence_ref: "evidence://acl/1".into(),
                    reconciled_at: Some(now()),
                },
                audit_id: "audit-1".into(),
                correlation_id: "corr-1".into(),
            },
            telemetry: ArchetypeTelemetry {
                correlation_id: "corr-1".into(),
                traceparent: "00-0123456789abcdef0123456789abcdef-0123456789abcdef-01".into(),
                channel: "neutral-test".into(),
                journey_id: None,
            },
            evidence: vec![ArchetypeEvidenceRef {
                kind: "source_acl".into(),
                uri: "evidence://acl/1".into(),
                sha256: "abc123".into(),
            }],
        }
    }

    fn read(context: ArchetypeContext, payload: Value) -> CanonicalRead {
        CanonicalRead {
            context,
            observed_at: now(),
            payload,
        }
    }

    #[test]
    fn validates_read_action_and_event_against_one_context() {
        let read = read(context(), json!({"id": "work-1", "tenant_id": "tenant-a"}));
        assert_eq!(read.validate_at(now(), "policy-1"), Ok(()));

        let action = CanonicalAction {
            context: context(),
            operation_id: "work.preview".into(),
            operation_version: "1".into(),
            expected_object_version: "3".into(),
            idempotency_key: "idem-1".into(),
            mode: ActionMode::Preview,
            arguments: json!({"note": "preview only"}),
        };
        assert_eq!(action.validate_at(now(), "policy-1"), Ok(()));

        let event = CanonicalEvent {
            context: context(),
            event_id: "event-1".into(),
            event_type: "work.observed".into(),
            event_version: "1".into(),
            occurred_at: now(),
            observed_at: now(),
            causation_id: None,
            payload_digest: "digest-1".into(),
            release_identity: "local-fixture".into(),
            disposition: EventDisposition::Observed,
        };
        assert_eq!(event.validate_at(now(), "policy-1"), Ok(()));
    }

    #[test]
    fn rejects_cross_tenant_subject_and_unallowed_action() {
        let mut mismatched = context();
        mismatched.policy.tenant_id = "tenant-b".into();
        assert_eq!(
            mismatched.validate_at(now(), "policy-1"),
            Err(ArchetypeContractError::Invalid("tenant_mismatch"))
        );

        let action = CanonicalAction {
            context: context(),
            operation_id: "work.execute".into(),
            operation_version: "1".into(),
            expected_object_version: "3".into(),
            idempotency_key: "idem-2".into(),
            mode: ActionMode::Preview,
            arguments: json!({}),
        };
        assert_eq!(
            action.validate_at(now(), "policy-1"),
            Err(ArchetypeContractError::Denied)
        );
    }

    #[test]
    fn rejects_action_outside_bounded_service_operations() {
        let mut service_context = context();
        let service = AuthoritativeSubject {
            issuer: "https://identity.example.test".into(),
            subject_id: "svc-projection".into(),
        };
        service_context.principal.principal_type = RuntimePrincipalType::Service;
        service_context.principal.subject = service.clone();
        service_context.principal.service_identity = Some(BoundedServiceIdentity {
            service: service.clone(),
            purpose: "projection-read".into(),
            allowed_scopes: vec!["work.read".into()],
            allowed_operations: vec!["work.observe".into()],
            valid_until: now() + Duration::minutes(5),
        });
        service_context.policy.subject = service;

        let action = CanonicalAction {
            context: service_context,
            operation_id: "work.preview".into(),
            operation_version: "1".into(),
            expected_object_version: "3".into(),
            idempotency_key: "idem-service-1".into(),
            mode: ActionMode::Preview,
            arguments: json!({}),
        };
        assert_eq!(
            action.validate_at(now(), "policy-1"),
            Err(ArchetypeContractError::Denied)
        );

        let mut event_context = action.context;
        event_context.policy.allowed_actions = vec!["work.observed".into()];
        let event = CanonicalEvent {
            context: event_context,
            event_id: "event-service-1".into(),
            event_type: "work.observed".into(),
            event_version: "1".into(),
            occurred_at: now(),
            observed_at: now(),
            causation_id: None,
            payload_digest: "digest-service-1".into(),
            release_identity: "local-fixture".into(),
            disposition: EventDisposition::Authorized,
        };
        assert_eq!(
            event.validate_at(now(), "policy-1"),
            Err(ArchetypeContractError::Denied)
        );
    }

    #[test]
    fn rejects_read_payload_or_decision_changed_after_enforcement() {
        let changed_payload = read(
            context(),
            json!({"id": "work-1", "tenant_id": "tenant-a", "private_note": "not masked"}),
        );
        assert_eq!(
            changed_payload.validate_at(now(), "policy-1"),
            Err(ArchetypeContractError::Invalid("read.policy_enforcement"))
        );

        let changed_decision = read(context(), json!({"id": "work-1", "tenant_id": "tenant-b"}));
        assert_eq!(
            changed_decision.validate_at(now(), "policy-1"),
            Err(ArchetypeContractError::Invalid("read.policy_enforcement"))
        );

        let mut redacted_context = context();
        redacted_context.policy.field_masks[0].treatment = FieldMaskTreatment::Redact;
        let exposed = read(
            redacted_context.clone(),
            json!({"id": "work-1", "tenant_id": "tenant-a", "private_note": "secret"}),
        );
        assert_eq!(
            exposed.validate_at(now(), "policy-1"),
            Err(ArchetypeContractError::Invalid("read.policy_enforcement"))
        );
        let redacted = read(
            redacted_context,
            json!({"id": "work-1", "tenant_id": "tenant-a", "private_note": "[REDACTED]"}),
        );
        assert_eq!(redacted.validate_at(now(), "policy-1"), Ok(()));

        let mut unsupported_context = context();
        unsupported_context.policy.row_filter = json!({"tenant_id": {"_in": ["tenant-a"]}});
        let unsupported = read(
            unsupported_context,
            json!({"id": "work-1", "tenant_id": "tenant-a"}),
        );
        assert_eq!(
            unsupported.validate_at(now(), "policy-1"),
            Err(ArchetypeContractError::Invalid("read.policy_enforcement"))
        );
    }

    #[test]
    fn distinguishes_observed_events_from_authorized_emission() {
        let base = CanonicalEvent {
            context: context(),
            event_id: "event-1".into(),
            event_type: "work.observed".into(),
            event_version: "1".into(),
            occurred_at: now(),
            observed_at: now(),
            causation_id: None,
            payload_digest: "digest-1".into(),
            release_identity: "local-fixture".into(),
            disposition: EventDisposition::Observed,
        };
        assert_eq!(base.validate_at(now(), "policy-1"), Ok(()));

        let unauthorized = CanonicalEvent {
            disposition: EventDisposition::Authorized,
            ..base.clone()
        };
        assert_eq!(
            unauthorized.validate_at(now(), "policy-1"),
            Err(ArchetypeContractError::Denied)
        );

        let mut authorized = unauthorized;
        authorized.context.policy.allowed_actions = vec!["work.observed".into()];
        assert_eq!(authorized.validate_at(now(), "policy-1"), Ok(()));
    }
}
