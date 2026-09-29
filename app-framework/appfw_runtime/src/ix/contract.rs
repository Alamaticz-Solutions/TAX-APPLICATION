use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, Write},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Digest;

use crate::{
    extension::{RuntimePrincipalType, UserAuth},
    observability::RequestContext,
};

use super::presentation::validate_pds_ix_presentation;
use super::recipe::{validate_recipe_binding, IxRecipeRegistration};

pub const IX_EVENT_SCHEMA_VERSION: &str = "appfw.ix_event@1";
pub const IX_RUN_REQUEST_SCHEMA_VERSION: &str = "appfw.ix_run_request@1";
pub const IX_REPLAY_SCHEMA_VERSION: &str = "appfw.ix_replay@1";
pub const IX_REPLAY_CHECKPOINT_SCHEMA_VERSION: &str = "appfw.ix_replay_checkpoint@1";
pub const IX_AUDIT_SCHEMA_VERSION: &str = "appfw.ix_audit@1";
pub const PDS_IX_PRESENTATION_SCHEMA_VERSION: &str = "pds.ix.presentation@1";
pub(crate) const IX_SNAPSHOT_SCHEMA_VERSION: &str = "appfw.ix_snapshot@1";
pub(crate) const IX_COMMIT_SCHEMA_VERSION: &str = "appfw.ix_commit@1";
pub(crate) const IX_MAX_PUBLIC_INTEGER: u64 = 9_007_199_254_740_991;
pub(crate) const IX_MAX_COMMITS: usize = 128;
pub(crate) const IX_MAX_EVENTS_PER_COMMIT: usize = 4;
pub(crate) const IX_MAX_REPLAY_BYTES: usize = 4 * 1024 * 1024;
pub(crate) const IX_MAX_PRESENTATION_BYTES: usize = 512 * 1024;
pub(crate) const IX_MAX_ACTIVE_RUNS: usize = 128;
pub(crate) const IX_MAX_ACTIVE_RUNS_PER_OWNER: usize = 4;
pub(crate) const IX_MAX_RETAINED_RUNS: usize = 256;
pub(crate) const IX_MAX_JSON_DEPTH: usize = 32;
pub(crate) const IX_MAX_JSON_NODES: usize = 4096;

const MAX_KEY_BYTES: usize = 256;
const MAX_LABEL_BYTES: usize = 512;
const MAX_DETAIL_BYTES: usize = 4096;
const MAX_QUESTION_BYTES: usize = 4096;
const MAX_CONTEXT_SOURCES: usize = 32;
const MAX_CONTEXT_GAPS: usize = 32;
const MAX_CHANGED_REGIONS: usize = 128;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IxFocusRef {
    pub kind: String,
    pub id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IxRelatedArtifactRef {
    pub artifact_id: String,
    pub artifact_type: String,
    pub content_schema_version: String,
    pub renderer_key: String,
    pub revision: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IxRunRequest {
    pub schema_version: String,
    pub intent_key: String,
    pub focus: IxFocusRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related_artifact: Option<IxRelatedArtifactRef>,
}

impl IxRunRequest {
    pub fn validate(&self) -> Result<(), IxContractError> {
        if self.schema_version != IX_RUN_REQUEST_SCHEMA_VERSION {
            return Err(IxContractError::UnsupportedSchema);
        }
        validate_key("intentKey", &self.intent_key)?;
        validate_key("focus.kind", &self.focus.kind)?;
        validate_key("focus.id", &self.focus.id)?;
        if let Some(question) = self.question.as_deref() {
            validate_text("question", question, MAX_QUESTION_BYTES)?;
        }
        if let Some(artifact) = &self.related_artifact {
            artifact.validate()?;
            if artifact.revision == 0 || artifact.revision >= IX_MAX_PUBLIC_INTEGER {
                return Err(IxContractError::InvalidArtifactRevision);
            }
        }
        Ok(())
    }
}

impl IxRelatedArtifactRef {
    fn validate(&self) -> Result<(), IxContractError> {
        validate_key("relatedArtifact.artifactId", &self.artifact_id)?;
        validate_key("relatedArtifact.artifactType", &self.artifact_type)?;
        validate_key(
            "relatedArtifact.contentSchemaVersion",
            &self.content_schema_version,
        )?;
        validate_key("relatedArtifact.rendererKey", &self.renderer_key)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IxPrincipalBinding {
    pub tenant_id: String,
    pub subject: String,
    pub principal_type: RuntimePrincipalType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_behalf_of: Option<String>,
}

impl From<&UserAuth> for IxPrincipalBinding {
    fn from(user: &UserAuth) -> Self {
        Self {
            tenant_id: user.tenant_id.clone(),
            subject: user.user_name.clone(),
            principal_type: user.principal_type,
            on_behalf_of: user.on_behalf_of.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IxArtifactBinding {
    pub artifact_type: String,
    pub content_schema_version: String,
    pub renderer_key: String,
}

impl IxArtifactBinding {
    pub fn new(
        artifact_type: impl Into<String>,
        content_schema_version: impl Into<String>,
        renderer_key: impl Into<String>,
    ) -> Result<Self, IxContractError> {
        let binding = Self {
            artifact_type: artifact_type.into(),
            content_schema_version: content_schema_version.into(),
            renderer_key: renderer_key.into(),
        };
        binding.validate()?;
        Ok(binding)
    }

    pub(crate) fn validate(&self) -> Result<(), IxContractError> {
        validate_key("artifactBinding.artifactType", &self.artifact_type)?;
        validate_key(
            "artifactBinding.contentSchemaVersion",
            &self.content_schema_version,
        )?;
        validate_key("artifactBinding.rendererKey", &self.renderer_key)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IxDataClassification {
    Public,
    Internal,
    Confidential,
    Restricted,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IxConsentState {
    NotRequired,
    Granted,
    Denied,
    Withdrawn,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "posture", rename_all = "snake_case", deny_unknown_fields)]
pub enum IxEgressPosture {
    LocalOnly,
    Approved {
        policy_id: String,
        decision_id: String,
        destination: String,
    },
}

impl IxEgressPosture {
    fn validate(&self) -> Result<(), IxContractError> {
        match self {
            Self::LocalOnly => Ok(()),
            Self::Approved {
                policy_id,
                decision_id,
                destination,
            } => {
                validate_key("egress.policyId", policy_id)?;
                validate_key("egress.decisionId", decision_id)?;
                validate_key("egress.destination", destination)
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IxExecutionAuthority {
    pub context_revision: u64,
    pub authorization_fingerprint: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IxContextClaims {
    release_id: String,
    session_id: String,
    principal: IxPrincipalBinding,
    roles: BTreeSet<String>,
    scopes: BTreeSet<String>,
}

impl IxContextClaims {
    pub fn from_authenticated_human(
        release_id: impl Into<String>,
        session_id: impl Into<String>,
        principal: &UserAuth,
    ) -> Result<Self, IxContractError> {
        if principal.principal_type != RuntimePrincipalType::User
            || principal.token.trim().is_empty()
        {
            return Err(IxContractError::UnauthorizedActor);
        }
        let claims = Self {
            release_id: release_id.into(),
            session_id: session_id.into(),
            principal: IxPrincipalBinding::from(principal),
            roles: principal.roles.iter().cloned().collect(),
            scopes: principal.scopes.iter().cloned().collect(),
        };
        claims.validate()?;
        Ok(claims)
    }

    #[cfg(feature = "chat")]
    pub(crate) fn from_verified_human_parts(
        release_id: impl Into<String>,
        session_id: impl Into<String>,
        principal: IxPrincipalBinding,
        roles: BTreeSet<String>,
        scopes: BTreeSet<String>,
    ) -> Result<Self, IxContractError> {
        let claims = Self {
            release_id: release_id.into(),
            session_id: session_id.into(),
            principal,
            roles,
            scopes,
        };
        claims.validate()?;
        Ok(claims)
    }

    pub fn release_id(&self) -> &str {
        &self.release_id
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn principal(&self) -> &IxPrincipalBinding {
        &self.principal
    }

    pub fn roles(&self) -> &BTreeSet<String> {
        &self.roles
    }

    pub fn scopes(&self) -> &BTreeSet<String> {
        &self.scopes
    }

    pub(crate) fn validate(&self) -> Result<(), IxContractError> {
        validate_key("contextClaims.releaseId", &self.release_id)?;
        validate_key("contextClaims.sessionId", &self.session_id)?;
        validate_principal_binding("contextClaims.principal", &self.principal)?;
        if self.principal.principal_type != RuntimePrincipalType::User
            || self.roles.len() > 64
            || self.scopes.len() > 128
        {
            return Err(IxContractError::InvalidContextRevision);
        }
        for role in &self.roles {
            validate_key("contextClaims.role", role)?;
        }
        for scope in &self.scopes {
            validate_key("contextClaims.scope", scope)?;
        }
        Ok(())
    }

    pub(crate) fn matches_authenticated_human(&self, principal: &UserAuth) -> bool {
        principal.principal_type == RuntimePrincipalType::User
            && !principal.token.trim().is_empty()
            && self.principal == IxPrincipalBinding::from(principal)
            && self.roles == principal.roles.iter().cloned().collect()
            && self.scopes == principal.scopes.iter().cloned().collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IxContextProposal {
    proposal_id: String,
    base_context_revision: u64,
    claims: IxContextClaims,
    reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    requested_classification: Option<IxDataClassification>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    requested_consent: Option<IxConsentState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    requested_egress_destination: Option<String>,
}

impl IxContextProposal {
    #[allow(clippy::too_many_arguments)]
    pub fn from_authenticated_human(
        proposal_id: impl Into<String>,
        base_context_revision: u64,
        release_id: impl Into<String>,
        session_id: impl Into<String>,
        principal: &UserAuth,
        reason: impl Into<String>,
        requested_classification: Option<IxDataClassification>,
        requested_consent: Option<IxConsentState>,
        requested_egress_destination: Option<String>,
    ) -> Result<Self, IxContractError> {
        let proposal = Self {
            proposal_id: proposal_id.into(),
            base_context_revision,
            claims: IxContextClaims::from_authenticated_human(release_id, session_id, principal)?,
            reason: reason.into(),
            requested_classification,
            requested_consent,
            requested_egress_destination,
        };
        proposal.validate()?;
        Ok(proposal)
    }

    pub fn proposal_id(&self) -> &str {
        &self.proposal_id
    }

    pub(crate) fn base_context_revision(&self) -> u64 {
        self.base_context_revision
    }

    pub fn claims(&self) -> &IxContextClaims {
        &self.claims
    }

    pub fn requested_classification(&self) -> Option<IxDataClassification> {
        self.requested_classification
    }

    pub fn requested_consent(&self) -> Option<IxConsentState> {
        self.requested_consent
    }

    pub fn requested_egress_destination(&self) -> Option<&str> {
        self.requested_egress_destination.as_deref()
    }

    pub(crate) fn validate(&self) -> Result<(), IxContractError> {
        validate_key("contextProposal.proposalId", &self.proposal_id)?;
        if self.base_context_revision == 0 || self.base_context_revision > IX_MAX_PUBLIC_INTEGER {
            return Err(IxContractError::InvalidContextRevision);
        }
        self.claims.validate()?;
        validate_detail("contextProposal.reason", &self.reason)?;
        if let Some(destination) = &self.requested_egress_destination {
            validate_key("contextProposal.requestedEgressDestination", destination)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IxPolicyDecisionProvenance {
    authority: IxPrincipalBinding,
    decision_id: String,
    policy_version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IxPolicyVerdict {
    pub decision_id: String,
    pub policy_version: String,
    pub classification: IxDataClassification,
    pub consent: IxConsentState,
    pub egress: IxEgressPosture,
}

impl IxPolicyVerdict {
    pub(crate) fn validate(&self) -> Result<(), IxContractError> {
        validate_key("policyVerdict.decisionId", &self.decision_id)?;
        validate_key("policyVerdict.policyVersion", &self.policy_version)?;
        self.egress.validate()
    }
}

impl IxPolicyDecisionProvenance {
    pub fn authority(&self) -> &IxPrincipalBinding {
        &self.authority
    }

    pub fn decision_id(&self) -> &str {
        &self.decision_id
    }

    pub fn policy_version(&self) -> &str {
        &self.policy_version
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IxContextRevision {
    revision: u64,
    claims: IxContextClaims,
    classification: IxDataClassification,
    consent: IxConsentState,
    egress: IxEgressPosture,
    policy_decision: IxPolicyDecisionProvenance,
    authorization_fingerprint: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct IxAuthorizationFingerprintMaterial<'a> {
    schema_version: &'static str,
    revision: u64,
    claims: &'a IxContextClaims,
    classification: IxDataClassification,
    consent: IxConsentState,
    egress: &'a IxEgressPosture,
    policy_decision: &'a IxPolicyDecisionProvenance,
}

impl IxContextRevision {
    pub(crate) fn from_policy_verdict(
        revision: u64,
        claims: IxContextClaims,
        policy_authority: IxPrincipalBinding,
        verdict: IxPolicyVerdict,
    ) -> Result<Self, IxContractError> {
        verdict.validate()?;
        let mut context = Self {
            revision,
            claims,
            classification: verdict.classification,
            consent: verdict.consent,
            egress: verdict.egress,
            policy_decision: IxPolicyDecisionProvenance {
                authority: policy_authority,
                decision_id: verdict.decision_id,
                policy_version: verdict.policy_version,
            },
            authorization_fingerprint: String::new(),
        };
        context.validate_fields()?;
        context.authorization_fingerprint = context.derived_fingerprint()?;
        Ok(context)
    }

    pub fn binding(&self) -> IxExecutionAuthority {
        IxExecutionAuthority {
            context_revision: self.revision,
            authorization_fingerprint: self.authorization_fingerprint.clone(),
        }
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn release_id(&self) -> &str {
        self.claims.release_id()
    }

    pub(crate) fn claims(&self) -> &IxContextClaims {
        &self.claims
    }

    pub fn principal(&self) -> &IxPrincipalBinding {
        self.claims.principal()
    }

    pub fn session_id(&self) -> &str {
        self.claims.session_id()
    }

    pub fn roles(&self) -> &BTreeSet<String> {
        self.claims.roles()
    }

    pub fn scopes(&self) -> &BTreeSet<String> {
        self.claims.scopes()
    }

    pub fn classification(&self) -> IxDataClassification {
        self.classification
    }

    pub fn consent(&self) -> IxConsentState {
        self.consent
    }

    pub fn egress(&self) -> &IxEgressPosture {
        &self.egress
    }

    pub fn authorization_fingerprint(&self) -> &str {
        &self.authorization_fingerprint
    }

    pub fn policy_decision(&self) -> &IxPolicyDecisionProvenance {
        &self.policy_decision
    }

    pub(crate) fn policy_authority(&self) -> &IxPrincipalBinding {
        &self.policy_decision.authority
    }

    pub(crate) fn validate(&self) -> Result<(), IxContractError> {
        self.validate_fields()?;
        if self.authorization_fingerprint != self.derived_fingerprint()? {
            return Err(IxContractError::AuthorizationFingerprintMismatch);
        }
        Ok(())
    }

    pub(crate) fn matches_principal_claims(&self, principal: &UserAuth) -> bool {
        self.claims.matches_authenticated_human(principal)
    }

    pub(crate) fn decided_by(&self, authority: &IxPrincipalBinding) -> bool {
        &self.policy_decision.authority == authority
    }

    pub(crate) fn allows_effects(&self) -> bool {
        !matches!(
            self.consent,
            IxConsentState::Denied | IxConsentState::Withdrawn
        )
    }

    pub(crate) fn same_authorization_semantics(&self, other: &Self) -> bool {
        self.claims == other.claims
            && self.classification == other.classification
            && self.consent == other.consent
            && self.egress == other.egress
            && self.policy_decision == other.policy_decision
    }

    fn validate_fields(&self) -> Result<(), IxContractError> {
        if self.revision == 0 || self.revision > IX_MAX_PUBLIC_INTEGER {
            return Err(IxContractError::InvalidContextRevision);
        }
        self.claims.validate()?;
        self.egress.validate()?;
        validate_principal_binding("policyDecision.authority", &self.policy_decision.authority)?;
        if self.policy_decision.authority.principal_type != RuntimePrincipalType::Service {
            return Err(IxContractError::UnauthorizedPolicyAuthority);
        }
        validate_key(
            "policyDecision.decisionId",
            &self.policy_decision.decision_id,
        )?;
        validate_key(
            "policyDecision.policyVersion",
            &self.policy_decision.policy_version,
        )
    }

    fn derived_fingerprint(&self) -> Result<String, IxContractError> {
        let material = IxAuthorizationFingerprintMaterial {
            schema_version: "appfw.ix.authorization_context@1",
            revision: self.revision,
            claims: &self.claims,
            classification: self.classification,
            consent: self.consent,
            egress: &self.egress,
            policy_decision: &self.policy_decision,
        };
        let bytes = serde_json::to_vec(&material)
            .map_err(|_| IxContractError::AuthorizationFingerprintMismatch)?;
        Ok(format!("sha256:{:x}", sha2::Sha256::digest(bytes)))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IxRunPolicy {
    pub(super) intent_label: String,
    pub(super) runner: IxPrincipalBinding,
    pub(super) registered_artifacts: BTreeSet<IxArtifactBinding>,
    pub(super) recipe_registration: Option<IxRecipeRegistration>,
}

impl IxRunPolicy {
    pub fn new(
        intent_label: impl Into<String>,
        runner: &UserAuth,
        registered_artifacts: impl IntoIterator<Item = IxArtifactBinding>,
    ) -> Result<Self, IxContractError> {
        if !matches!(
            runner.principal_type,
            RuntimePrincipalType::Agent | RuntimePrincipalType::Service
        ) || !runner.has_scope("appfw:ix.progress")
        {
            return Err(IxContractError::UnauthorizedActor);
        }
        let intent_label = intent_label.into();
        validate_text("intentLabel", &intent_label, MAX_LABEL_BYTES)?;
        let registered_artifacts = registered_artifacts
            .into_iter()
            .map(|binding| {
                binding.validate()?;
                Ok(binding)
            })
            .collect::<Result<BTreeSet<_>, IxContractError>>()?;
        if registered_artifacts.is_empty() || registered_artifacts.len() > 8 {
            return Err(IxContractError::InvalidArtifactRegistry);
        }
        Ok(Self {
            intent_label,
            runner: IxPrincipalBinding::from(runner),
            registered_artifacts,
            recipe_registration: None,
        })
    }

    pub fn new_for_recipe(
        intent_label: impl Into<String>,
        runner: &UserAuth,
        recipe_registration: IxRecipeRegistration,
    ) -> Result<Self, IxContractError> {
        let artifact_binding = recipe_registration.artifact_binding()?;
        let mut policy = Self::new(intent_label, runner, [artifact_binding])?;
        policy.recipe_registration = Some(recipe_registration);
        Ok(policy)
    }

    pub(crate) fn validate_for_request(
        &self,
        request: &IxRunRequest,
    ) -> Result<(), IxContractError> {
        validate_recipe_binding(
            self.recipe_registration.as_ref(),
            request,
            &self.registered_artifacts,
        )
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IxContextFreshness {
    Current,
    Stale,
    Mixed,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IxSourceSummary {
    pub source_ref: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refreshed_at: Option<String>,
    pub freshness: IxContextFreshness,
    pub inspectable: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IxContextSummary {
    pub focus_label: String,
    pub detail: String,
    pub evaluated_at: String,
    pub freshness: IxContextFreshness,
    #[serde(default)]
    pub sources: Vec<IxSourceSummary>,
    #[serde(default)]
    pub gaps: Vec<String>,
}

impl IxContextSummary {
    pub fn validate(&self) -> Result<(), IxContractError> {
        validate_text("context.focusLabel", &self.focus_label, MAX_LABEL_BYTES)?;
        validate_text("context.detail", &self.detail, MAX_DETAIL_BYTES)?;
        let evaluated_at = chrono::DateTime::parse_from_rfc3339(&self.evaluated_at)
            .map_err(|_| IxContractError::InvalidTimestamp)?;
        if self.sources.len() > MAX_CONTEXT_SOURCES || self.gaps.len() > MAX_CONTEXT_GAPS {
            return Err(IxContractError::ResourceLimitExceeded);
        }
        let mut refs = BTreeSet::new();
        for source in &self.sources {
            validate_key("context.source.sourceRef", &source.source_ref)?;
            validate_text("context.source.label", &source.label, MAX_LABEL_BYTES)?;
            if !refs.insert(source.source_ref.as_str()) {
                return Err(IxContractError::DuplicateSourceReference);
            }
            if source.freshness == IxContextFreshness::Mixed {
                return Err(IxContractError::InvalidContextFreshness);
            }
            match source.refreshed_at.as_deref() {
                Some(value) => {
                    let refreshed_at = chrono::DateTime::parse_from_rfc3339(value)
                        .map_err(|_| IxContractError::InvalidTimestamp)?;
                    if refreshed_at > evaluated_at {
                        return Err(IxContractError::InvalidContextFreshness);
                    }
                }
                None if source.freshness != IxContextFreshness::Unknown => {
                    return Err(IxContractError::InvalidContextFreshness)
                }
                None => {}
            }
        }
        for gap in &self.gaps {
            validate_text("context.gap", gap, MAX_DETAIL_BYTES)?;
        }
        let expected_freshness = match self.sources.as_slice() {
            [] => IxContextFreshness::Unknown,
            [first, rest @ ..]
                if rest
                    .iter()
                    .all(|source| source.freshness == first.freshness) =>
            {
                first.freshness
            }
            _ => IxContextFreshness::Mixed,
        };
        if self.freshness != expected_freshness {
            return Err(IxContractError::InvalidContextFreshness);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IxPhase {
    Acknowledged,
    Understanding,
    Gathering,
    Resolving,
    Interpreting,
    Composing,
    Checking,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IxRunState {
    Pending,
    Running,
    WaitingForUser,
    Cancelling,
    Terminal,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IxTerminalOutcome {
    Completed,
    Partial,
    Cancelled,
    Failed,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IxArtifactStatus {
    Partial,
    Ready,
    Revising,
    Stale,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IxArtifactRevision {
    pub artifact_id: String,
    pub artifact_type: String,
    pub content_schema_version: String,
    pub renderer_key: String,
    pub revision: u64,
    pub status: IxArtifactStatus,
    #[serde(default)]
    pub changed_region_ids: Vec<String>,
    pub presentation: Value,
}

impl IxArtifactRevision {
    pub fn validate(&self) -> Result<(), IxContractError> {
        validate_key("artifact.artifactId", &self.artifact_id)?;
        validate_key("artifact.artifactType", &self.artifact_type)?;
        validate_key(
            "artifact.contentSchemaVersion",
            &self.content_schema_version,
        )?;
        validate_key("artifact.rendererKey", &self.renderer_key)?;
        if self.revision == 0 || self.revision > IX_MAX_PUBLIC_INTEGER {
            return Err(IxContractError::InvalidArtifactRevision);
        }
        if self.changed_region_ids.len() > MAX_CHANGED_REGIONS {
            return Err(IxContractError::ResourceLimitExceeded);
        }
        let mut regions = BTreeSet::new();
        for region in &self.changed_region_ids {
            if !regions.insert(region.as_str()) {
                return Err(IxContractError::DuplicateChangedRegion);
            }
        }
        validate_portable_json(&self.presentation)?;
        validate_serialized_json_size(&self.presentation, IX_MAX_PRESENTATION_BYTES)?;
        validate_pds_ix_presentation(&self.presentation)?;
        let Some(object) = self.presentation.as_object() else {
            return Err(IxContractError::InvalidPresentation);
        };
        if self.content_schema_version != PDS_IX_PRESENTATION_SCHEMA_VERSION {
            return Err(IxContractError::InvalidPresentation);
        }
        let Some(identity) = object.get("identity").and_then(Value::as_object) else {
            return Err(IxContractError::InvalidPresentation);
        };
        if identity.get("schemaVersion").and_then(Value::as_str)
            != Some(self.content_schema_version.as_str())
            || identity.get("presentationId").and_then(Value::as_str)
                != Some(self.artifact_id.as_str())
            || identity.get("revision").and_then(Value::as_u64) != Some(self.revision)
        {
            return Err(IxContractError::InvalidPresentation);
        }
        let response_regions = object
            .get("response")
            .and_then(Value::as_object)
            .and_then(|response| response.get("regions"))
            .and_then(Value::as_array)
            .ok_or(IxContractError::InvalidPresentation)?;
        let mut presentation_region_ids = BTreeSet::new();
        let mut presentation_changed_region_ids = BTreeSet::new();
        for region in response_regions {
            let region = region
                .as_object()
                .ok_or(IxContractError::InvalidPresentation)?;
            let region_id = region
                .get("id")
                .and_then(Value::as_str)
                .ok_or(IxContractError::InvalidPresentation)?;
            if !presentation_region_ids.insert(region_id) {
                return Err(IxContractError::DuplicatePresentationRegion);
            }
            if region.get("changed").and_then(Value::as_bool) == Some(true) {
                presentation_changed_region_ids.insert(region_id);
            }
        }
        if !regions
            .iter()
            .all(|changed_region_id| presentation_region_ids.contains(*changed_region_id))
        {
            return Err(IxContractError::UnknownChangedRegion);
        }
        if regions != presentation_changed_region_ids {
            return Err(IxContractError::ChangedRegionMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IxArtifactState {
    pub artifact_type: String,
    pub content_schema_version: String,
    pub renderer_key: String,
    pub revision: u64,
    pub status: IxArtifactStatus,
    pub changed_region_ids: Vec<String>,
    pub presentation: Value,
}

impl From<&IxArtifactRevision> for IxArtifactState {
    fn from(value: &IxArtifactRevision) -> Self {
        Self {
            artifact_type: value.artifact_type.clone(),
            content_schema_version: value.content_schema_version.clone(),
            renderer_key: value.renderer_key.clone(),
            revision: value.revision,
            status: value.status,
            changed_region_ids: value.changed_region_ids.clone(),
            presentation: value.presentation.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum IxCancellationStage {
    Accepted,
    Stopping,
    Draining,
    Stopped,
    Confirmed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IxCancellationFence {
    pub command_id: String,
    pub accepted_cursor: String,
    pub accepted_revision: u64,
    pub stage: IxCancellationStage,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum IxEventPayload {
    RunAcknowledged {
        intent_label: String,
        focus: IxFocusRef,
    },
    ContextResolved {
        authority: IxExecutionAuthority,
        context: IxContextSummary,
    },
    ContextReconciliationRequested {
        proposal: IxContextProposal,
    },
    ContextReconciled {
        proposal_id: String,
        context_revision: IxContextRevision,
    },
    PhaseChanged {
        authority: IxExecutionAuthority,
        phase: IxPhase,
        reason: String,
    },
    ArtifactRevision {
        authority: IxExecutionAuthority,
        artifact: IxArtifactRevision,
    },
    WaitingForUser {
        authority: IxExecutionAuthority,
        prompt: String,
    },
    UserInputAccepted {
        authority: IxExecutionAuthority,
    },
    CancelRequested {
        command_id: String,
        accepted_cursor: String,
        accepted_revision: u64,
    },
    CancellationProgress {
        command_id: String,
        stage: IxCancellationStage,
    },
    RunFinished {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        authority: Option<IxExecutionAuthority>,
        outcome: IxTerminalOutcome,
        message: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IxEvent {
    pub schema_version: String,
    pub event_id: String,
    pub run_id: String,
    pub sequence: u64,
    pub occurred_at: String,
    pub actor: IxPrincipalBinding,
    pub payload: IxEventPayload,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IxRunSnapshot {
    pub schema_version: String,
    pub run_id: String,
    pub release_id: String,
    pub owner: IxPrincipalBinding,
    pub runner: IxPrincipalBinding,
    pub request: IxRunRequest,
    pub registered_artifacts: BTreeSet<IxArtifactBinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipe_registration: Option<IxRecipeRegistration>,
    pub authorization_context: IxContextRevision,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_context_proposal: Option<IxContextProposal>,
    pub state: IxRunState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<IxPhase>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<IxContextSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_authority: Option<IxExecutionAuthority>,
    #[serde(default)]
    pub artifacts: BTreeMap<String, IxArtifactState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancellation: Option<IxCancellationFence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_outcome: Option<IxTerminalOutcome>,
    pub task_revision: u64,
    pub last_sequence: u64,
    pub cursor: String,
    pub next_safe_move: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IxCommitEnvelope {
    pub schema_version: String,
    pub commit_id: String,
    pub committed_by: String,
    pub run_id: String,
    pub base_cursor: String,
    pub result_cursor: String,
    pub base_revision: u64,
    pub result_revision: u64,
    pub events: Vec<IxEvent>,
    pub next_safe_move: String,
    pub base_snapshot_sha256: String,
    pub result_snapshot_sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IxReplayBatch {
    pub schema_version: String,
    pub base_snapshot: IxRunSnapshot,
    pub envelopes: Vec<IxCommitEnvelope>,
    pub expected_snapshot: IxRunSnapshot,
}

/// Separately retained authority binding for replay verification. A replay
/// batch can prove deterministic consistency, but it is authoritative only
/// when its base matches a checkpoint obtained from the authenticated runtime
/// and retained outside the batch under review. It binds both ends of the
/// range and the canonical complete-envelope chain, not only the base.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IxReplayCheckpoint {
    pub schema_version: String,
    pub run_id: String,
    pub owner: IxPrincipalBinding,
    pub base_revision: u64,
    pub result_revision: u64,
    pub base_cursor: String,
    pub result_cursor: String,
    pub base_snapshot_sha256: String,
    pub result_snapshot_sha256: String,
    pub envelope_count: u64,
    pub envelope_chain_sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IxAuditEventKind {
    RunRequested,
    CancelDisposition,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IxAuditCancelDisposition {
    Accepted,
    AlreadyRequested,
    AlreadyTerminal,
    NotFound,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IxAuditEvent {
    pub schema_version: String,
    pub event_kind: IxAuditEventKind,
    pub request_id: String,
    pub correlation_id: String,
    pub principal: IxPrincipalBinding,
    pub run_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancel_disposition: Option<IxAuditCancelDisposition>,
}

pub trait IxAuditSink: Send + Sync + 'static {
    fn append(&self, event: &IxAuditEvent) -> Result<(), IxContractError>;
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IxCancelDisposition {
    Accepted,
    AlreadyRequested,
    AlreadyTerminal,
    NotFound,
}

impl From<IxCancelDisposition> for IxAuditCancelDisposition {
    fn from(value: IxCancelDisposition) -> Self {
        match value {
            IxCancelDisposition::Accepted => Self::Accepted,
            IxCancelDisposition::AlreadyRequested => Self::AlreadyRequested,
            IxCancelDisposition::AlreadyTerminal => Self::AlreadyTerminal,
            IxCancelDisposition::NotFound => Self::NotFound,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IxStartReceipt {
    pub run_id: String,
    pub cursor: String,
    pub task_revision: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IxCancelReceipt {
    pub disposition: IxCancelDisposition,
    pub audit_retained: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
pub enum IxContractError {
    #[error("IX contract schema is unsupported")]
    UnsupportedSchema,
    #[error("IX actor is not authorized for this operation")]
    UnauthorizedActor,
    #[error("IX policy decision was not issued by the configured policy authority")]
    UnauthorizedPolicyAuthority,
    #[error("IX audit evidence is unavailable")]
    AuditUnavailable,
    #[error("IX run was not found")]
    RunNotFound,
    #[error("IX lifecycle transition is invalid")]
    InvalidLifecycle,
    #[error("IX replay cursor is invalid or not a commit boundary")]
    InvalidReplayCursor,
    #[error("IX snapshot or replay digest does not match")]
    DigestMismatch,
    #[error("IX event sequence is not contiguous")]
    InvalidSequence,
    #[error("IX artifact registry is invalid")]
    InvalidArtifactRegistry,
    #[error("IX recipe registration does not match the canonical PDS registry")]
    InvalidRecipeRegistration,
    #[error("IX artifact type, presentation schema, and renderer tuple is not registered")]
    UnknownArtifactType,
    #[error("IX artifact identity changed")]
    ArtifactIdentityChanged,
    #[error("IX artifact revision is not the exact next revision")]
    InvalidArtifactRevision,
    #[error("IX artifact presentation is not pds.ix.presentation@1")]
    InvalidPresentation,
    #[error("IX changed-region identity is duplicated")]
    DuplicateChangedRegion,
    #[error("IX presentation region identity is duplicated")]
    DuplicatePresentationRegion,
    #[error("IX changed-region identity does not name a presentation region")]
    UnknownChangedRegion,
    #[error("IX changed-region identity conflicts with presentation change state")]
    ChangedRegionMismatch,
    #[error("IX source reference is duplicated")]
    DuplicateSourceReference,
    #[error("IX context freshness is inconsistent")]
    InvalidContextFreshness,
    #[error("IX authorization context revision is invalid")]
    InvalidContextRevision,
    #[error("IX release identity is immutable for the lifetime of a run")]
    ImmutableRunIdentityChanged,
    #[error("IX context reconciliation already has a pending human proposal")]
    ContextProposalAlreadyPending,
    #[error("IX authorization context fingerprint does not match its typed fields")]
    AuthorizationFingerprintMismatch,
    #[error("IX execution authority is stale for the current context revision")]
    StaleAuthorization,
    #[error("IX current consent posture does not authorize an effect")]
    EffectNotAuthorized,
    #[error("IX timestamp is invalid")]
    InvalidTimestamp,
    #[error("IX cancellation command conflicts with an existing command")]
    CancellationConflict,
    #[error("IX cancellation progress is not monotonic")]
    InvalidCancellationProgress,
    #[error("IX resource bound was exceeded")]
    ResourceLimitExceeded,
    #[error("{field} is empty, too large, or not a stable key")]
    InvalidField { field: &'static str },
    #[error("IX JSON value is not portable across web and native channels")]
    NonPortableJson,
}

pub(crate) fn runtime_actor() -> IxPrincipalBinding {
    IxPrincipalBinding {
        tenant_id: "appfw".to_string(),
        subject: "appfw-runtime".to_string(),
        principal_type: RuntimePrincipalType::Service,
        on_behalf_of: None,
    }
}

pub(crate) fn validate_policy_authority(
    authority: &UserAuth,
) -> Result<IxPrincipalBinding, IxContractError> {
    if authority.principal_type != RuntimePrincipalType::Service
        || !authority.has_scope("appfw:ix.authorize")
    {
        return Err(IxContractError::UnauthorizedPolicyAuthority);
    }
    let binding = IxPrincipalBinding::from(authority);
    validate_principal_binding("policyDecision.authority", &binding)?;
    Ok(binding)
}

pub(crate) fn validate_principal_binding(
    prefix: &'static str,
    principal: &IxPrincipalBinding,
) -> Result<(), IxContractError> {
    let tenant_field = match prefix {
        "contextRevision.principal" => "contextRevision.principal.tenantId",
        _ => "principal.tenantId",
    };
    let subject_field = match prefix {
        "contextRevision.principal" => "contextRevision.principal.subject",
        _ => "principal.subject",
    };
    validate_key(tenant_field, &principal.tenant_id)?;
    validate_key(subject_field, &principal.subject)?;
    if let Some(on_behalf_of) = principal.on_behalf_of.as_deref() {
        validate_key("principal.onBehalfOf", on_behalf_of)?;
    }
    Ok(())
}

pub(crate) fn audit_event(
    kind: IxAuditEventKind,
    request_context: &RequestContext,
    principal: &UserAuth,
    run_id: String,
    intent_key: Option<String>,
    cancel_disposition: Option<IxAuditCancelDisposition>,
) -> IxAuditEvent {
    IxAuditEvent {
        schema_version: IX_AUDIT_SCHEMA_VERSION.to_string(),
        event_kind: kind,
        request_id: request_context.request_id.clone(),
        correlation_id: request_context.correlation_id.clone(),
        principal: IxPrincipalBinding::from(principal),
        run_id,
        intent_key,
        cancel_disposition,
    }
}

pub(crate) fn cursor(run_id: &str, revision: u64) -> String {
    format!("ix1.{run_id}.{revision}")
}

pub(crate) fn validate_text(
    field: &'static str,
    value: &str,
    max_bytes: usize,
) -> Result<(), IxContractError> {
    if value.trim().is_empty() || value.len() > max_bytes || value.contains('\0') {
        return Err(IxContractError::InvalidField { field });
    }
    Ok(())
}

pub(crate) fn validate_key(field: &'static str, value: &str) -> Result<(), IxContractError> {
    validate_text(field, value, MAX_KEY_BYTES)?;
    if !value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':' | b'@' | b'/')
    }) {
        return Err(IxContractError::InvalidField { field });
    }
    Ok(())
}

pub(crate) fn validate_detail(field: &'static str, value: &str) -> Result<(), IxContractError> {
    validate_text(field, value, MAX_DETAIL_BYTES)
}

pub(crate) fn validate_portable_json(value: &Value) -> Result<(), IxContractError> {
    validate_json_shape(value, IX_MAX_JSON_DEPTH, IX_MAX_JSON_NODES, true)
}

struct BoundedJsonWriter {
    written: usize,
    limit: usize,
    exceeded: bool,
}

impl Write for BoundedJsonWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let Some(next) = self.written.checked_add(bytes.len()) else {
            self.exceeded = true;
            return Err(io::Error::other("IX JSON serialization limit exceeded"));
        };
        if next > self.limit {
            self.exceeded = true;
            return Err(io::Error::other("IX JSON serialization limit exceeded"));
        }
        self.written = next;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(crate) fn validate_serialized_json_size(
    value: &Value,
    limit: usize,
) -> Result<(), IxContractError> {
    let mut writer = BoundedJsonWriter {
        written: 0,
        limit,
        exceeded: false,
    };
    match serde_json::to_writer(&mut writer, value) {
        Ok(()) => Ok(()),
        Err(_) if writer.exceeded => Err(IxContractError::ResourceLimitExceeded),
        Err(_) => Err(IxContractError::InvalidPresentation),
    }
}

pub(crate) fn validate_json_shape(
    value: &Value,
    max_depth: usize,
    max_nodes: usize,
    validate_numbers: bool,
) -> Result<(), IxContractError> {
    let mut stack = vec![(value, 0usize)];
    let mut node_count = 0usize;
    while let Some((current, depth)) = stack.pop() {
        node_count = node_count
            .checked_add(1)
            .filter(|count| *count <= max_nodes)
            .ok_or(IxContractError::ResourceLimitExceeded)?;
        if depth > max_depth {
            return Err(IxContractError::ResourceLimitExceeded);
        }
        match current {
            Value::Null | Value::Bool(_) | Value::String(_) => {}
            Value::Array(values) => {
                stack.extend(values.iter().map(|value| (value, depth + 1)));
            }
            Value::Object(values) => {
                stack.extend(values.values().map(|value| (value, depth + 1)));
            }
            Value::Number(number) => {
                if !validate_numbers {
                    continue;
                }
                if let Some(value) = number.as_u64() {
                    if value > IX_MAX_PUBLIC_INTEGER {
                        return Err(IxContractError::NonPortableJson);
                    }
                    continue;
                }
                if let Some(value) = number.as_i64() {
                    if value.unsigned_abs() > IX_MAX_PUBLIC_INTEGER {
                        return Err(IxContractError::NonPortableJson);
                    }
                    continue;
                }
                let Some(value) = number.as_f64() else {
                    return Err(IxContractError::NonPortableJson);
                };
                if !value.is_finite()
                    || value.abs() > IX_MAX_PUBLIC_INTEGER as f64
                    || (value == 0.0 && value.is_sign_negative())
                {
                    return Err(IxContractError::NonPortableJson);
                }
            }
        }
    }
    Ok(())
}
