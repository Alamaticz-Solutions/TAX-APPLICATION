//! Product-owned [`IxContextPolicy`] for the Nexus IX vertical.
//!
//! The policy is deterministic and local-proof scoped: it never grants an
//! egress destination, never exceeds the `internal` classification, and
//! decides context reconciliation only for same-principal proposals. The
//! Framework remains the authority that stamps the resulting context
//! revision, fingerprint, and provenance; this policy only issues verdicts.

use appfw_runtime::ix::{
    IxContextClaims, IxContextProposal, IxContextRevision, IxContractError, IxConsentState,
    IxDataClassification, IxEgressPosture, IxPolicyVerdict,
};

pub const NEXUS_IX_CONTEXT_POLICY_VERSION: &str = "nexus.ix.context-policy@1";

/// Deterministic local context policy for Nexus IX runs.
#[derive(Clone, Copy, Debug, Default)]
pub struct NexusIxContextPolicy;

impl NexusIxContextPolicy {
    pub fn new() -> Self {
        Self
    }

    fn verdict(
        decision_id: String,
        classification: IxDataClassification,
        consent: IxConsentState,
    ) -> IxPolicyVerdict {
        IxPolicyVerdict {
            decision_id,
            policy_version: NEXUS_IX_CONTEXT_POLICY_VERSION.to_string(),
            classification,
            consent,
            // Local proof only: this policy never authorizes egress.
            egress: IxEgressPosture::LocalOnly,
        }
    }
}

impl appfw_runtime::ix::IxContextPolicy for NexusIxContextPolicy {
    fn decide_initial(&self, claims: &IxContextClaims) -> Result<IxPolicyVerdict, IxContractError> {
        Ok(Self::verdict(
            format!("nexus.ix.decision.initial.{}", claims.session_id()),
            IxDataClassification::Internal,
            IxConsentState::NotRequired,
        ))
    }

    fn decide_reconciliation(
        &self,
        current: &IxContextRevision,
        proposal: &IxContextProposal,
    ) -> Result<IxPolicyVerdict, IxContractError> {
        if proposal.claims().principal() != current.principal() {
            return Err(IxContractError::UnauthorizedActor);
        }
        if proposal.requested_egress_destination().is_some() {
            // No egress authority exists in the local vertical; a destination
            // request cannot be granted and must not silently degrade.
            return Err(IxContractError::EffectNotAuthorized);
        }
        let classification = match proposal.requested_classification() {
            None => current.classification(),
            Some(requested @ (IxDataClassification::Public | IxDataClassification::Internal)) => {
                requested
            }
            Some(IxDataClassification::Confidential | IxDataClassification::Restricted) => {
                return Err(IxContractError::EffectNotAuthorized);
            }
        };
        let consent = proposal.requested_consent().unwrap_or(current.consent());
        Ok(Self::verdict(
            format!("nexus.ix.decision.reconcile.{}", proposal.proposal_id()),
            classification,
            consent,
        ))
    }
}

#[cfg(test)]
mod tests {
    use appfw_runtime::extension::{RuntimePrincipalType, UserAuth};
    use appfw_runtime::ix::IxContextPolicy as _;

    use super::*;

    fn owner_auth() -> UserAuth {
        UserAuth {
            tenant_id: "tenant_a".to_string(),
            user_name: "nexus.user".to_string(),
            timezone: "UTC".to_string(),
            principal_type: RuntimePrincipalType::User,
            on_behalf_of: None,
            ingress: None,
            roles: vec!["nexus-operations".to_string()],
            scopes: vec!["nexus:my-work.read".to_string()],
            token: "synthetic-local-proof".to_string(),
        }
    }

    fn claims() -> IxContextClaims {
        IxContextClaims::from_authenticated_human(
            "nexus-ix-local-release@1",
            "nexus-session-0001",
            &owner_auth(),
        )
        .expect("claims are key-valid")
    }

    #[test]
    fn initial_verdict_is_internal_local_only_and_key_valid() {
        let verdict = NexusIxContextPolicy::new()
            .decide_initial(&claims())
            .expect("initial verdict");
        assert_eq!(verdict.policy_version, NEXUS_IX_CONTEXT_POLICY_VERSION);
        assert_eq!(verdict.classification, IxDataClassification::Internal);
        assert_eq!(verdict.consent, IxConsentState::NotRequired);
        assert_eq!(verdict.egress, IxEgressPosture::LocalOnly);
        assert_eq!(
            verdict.decision_id,
            "nexus.ix.decision.initial.nexus-session-0001"
        );
    }

    #[test]
    fn reconciliation_bounds_classification_and_refuses_egress() {
        let policy = NexusIxContextPolicy::new();
        let auth = owner_auth();
        let current = current_revision(&policy);

        let widen = proposal(&auth, Some(IxDataClassification::Public), None, None);
        let verdict = policy
            .decide_reconciliation(&current, &widen)
            .expect("public classification is grantable");
        assert_eq!(verdict.classification, IxDataClassification::Public);
        assert_eq!(verdict.egress, IxEgressPosture::LocalOnly);

        let escalate = proposal(&auth, Some(IxDataClassification::Restricted), None, None);
        assert_eq!(
            policy.decide_reconciliation(&current, &escalate),
            Err(IxContractError::EffectNotAuthorized)
        );

        let egress = proposal(&auth, None, None, Some("https-example".to_string()));
        assert_eq!(
            policy.decide_reconciliation(&current, &egress),
            Err(IxContractError::EffectNotAuthorized)
        );
    }

    #[test]
    fn reconciliation_rejects_foreign_principals() {
        let policy = NexusIxContextPolicy::new();
        let current = current_revision(&policy);
        let mut other = owner_auth();
        other.user_name = "different.user".to_string();
        let foreign = proposal(&other, None, None, None);
        assert_eq!(
            policy.decide_reconciliation(&current, &foreign),
            Err(IxContractError::UnauthorizedActor)
        );
    }

    fn current_revision(policy: &NexusIxContextPolicy) -> IxContextRevision {
        // The Framework stamps context revisions; tests obtain one by running
        // the real foreground runtime start path in proof_fixtures. Here a
        // minimal revision is harvested through the same public seam.
        super::super::proof_fixtures::harvested_initial_context_revision(policy)
    }

    fn proposal(
        auth: &UserAuth,
        classification: Option<IxDataClassification>,
        consent: Option<appfw_runtime::ix::IxConsentState>,
        egress: Option<String>,
    ) -> IxContextProposal {
        IxContextProposal::from_authenticated_human(
            "nexus-proposal-0001",
            1,
            "nexus-ix-local-release@1",
            "nexus-session-0001",
            auth,
            "Adjust the authorized context for continued local work.",
            classification,
            consent,
            egress,
        )
        .expect("proposal is key-valid")
    }
}
