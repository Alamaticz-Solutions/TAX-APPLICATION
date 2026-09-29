//! IX protocol policy and Framework-owned service semantics.
//!
//! HTTP/Axum/CORS/SSE framing lives in `routing`. This module binds verified,
//! token-free humans to the single durable lifecycle reducer; Product seams
//! can supply context, drafts, and persistence but never canonical events.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    net::{Ipv4Addr, Ipv6Addr},
    sync::{Arc, Mutex},
    time::Duration,
};

use futures_util::{stream, stream::BoxStream, StreamExt};
use serde::Serialize;
use tokio::{
    sync::{broadcast, watch},
    time::Instant,
};
use uuid::Uuid;

use crate::{
    auth::IxVerifiedClaims, extension::RuntimePrincipalType, observability::RequestContext,
};

use super::{
    contract::validate_key,
    lifecycle::{
        prepare_durable_append, prepare_durable_cancel, prepare_durable_cancel_confirmation,
        prepare_durable_start, IxDurableCancelPreparation, IxStoredRunSealKey, IxStoredRunSealer,
    },
    IxArtifactBinding, IxArtifactRevision, IxAuditProjection, IxAuditProjectionState,
    IxCancelDisposition, IxCancellationSignal, IxCancellationSignalState, IxCommitEnvelope,
    IxContextClaims, IxContextPolicy, IxContractError, IxEventPayload, IxOrchestrationDraft,
    IxPreparedRun, IxPrincipalBinding, IxProductContextResolver, IxProductOrchestrator,
    IxRecipeRegistration, IxReplayBatch, IxReplayCheckpoint, IxReplayVerifier,
    IxRepositoryCommitReceipt, IxRunPolicy, IxRunRepository, IxRunRequest, IxRunState,
    IxTerminalOutcome,
};

use super::contract::{IX_MAX_ACTIVE_RUNS, IX_MAX_ACTIVE_RUNS_PER_OWNER};

pub const IX_STREAM_PATH: &str = "/chat/stream";
pub const IX_CANCEL_PATH: &str = "/chat/runs/:run_id/cancel";
pub const IX_CANCEL_REQUEST_SCHEMA_VERSION: &str = "appfw.ix_cancel@1";
pub const IX_CALLER_PROFILE_HEADER: &str = "x-appfw-ix-client-profile";
pub(crate) const LAST_EVENT_ID_HEADER: &str = "last-event-id";
const IX_MAX_RUN_DURATION: Duration = Duration::from_secs(120);
const IX_FRAMEWORK_POLICY_SUBJECT: &str = "appfw-ix-policy-authority";
const IX_FRAMEWORK_RUNNER_SUBJECT: &str = "appfw-ix-orchestrator";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IxClientProfile {
    Browser,
    Native,
    NonBrowser,
}

#[derive(Clone, Debug)]
pub struct IxClientBinding {
    client_id: String,
    audience: String,
    profile: IxClientProfile,
    allowed_origins: BTreeSet<String>,
}

impl IxClientBinding {
    pub fn new(
        client_id: impl Into<String>,
        audience: impl Into<String>,
        profile: IxClientProfile,
        allowed_origins: impl IntoIterator<Item = String>,
    ) -> Result<Self, IxTransportError> {
        let client_id = client_id.into();
        let audience = audience.into();
        validate_policy_key(&client_id)?;
        validate_policy_key(&audience)?;
        let allowed_origins = allowed_origins
            .into_iter()
            .map(|origin| canonical_origin(&origin))
            .collect::<Result<BTreeSet<_>, _>>()?;
        if profile == IxClientProfile::Browser && allowed_origins.is_empty() {
            return Err(IxTransportError::InvalidPolicy);
        }
        if profile != IxClientProfile::Browser && !allowed_origins.is_empty() {
            return Err(IxTransportError::InvalidPolicy);
        }
        Ok(Self {
            client_id,
            audience,
            profile,
            allowed_origins,
        })
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    pub fn audience(&self) -> &str {
        &self.audience
    }

    pub fn profile(&self) -> IxClientProfile {
        self.profile
    }
}

#[derive(Clone, Debug)]
pub struct IxTransportPolicy {
    issuer: String,
    bindings: BTreeMap<(String, String), IxClientBinding>,
    required_roles: BTreeSet<String>,
    required_scopes: BTreeSet<String>,
    release_id: String,
}

impl IxTransportPolicy {
    pub fn new(
        issuer: impl Into<String>,
        release_id: impl Into<String>,
        bindings: impl IntoIterator<Item = IxClientBinding>,
        required_roles: impl IntoIterator<Item = String>,
        required_scopes: impl IntoIterator<Item = String>,
    ) -> Result<Self, IxTransportError> {
        let issuer = issuer.into();
        let release_id = release_id.into();
        if !(issuer.starts_with("https://") || issuer.starts_with("http://127.0.0.1:"))
            || issuer.ends_with('/')
        {
            return Err(IxTransportError::InvalidPolicy);
        }
        validate_policy_key(&release_id)?;
        let mut indexed = BTreeMap::new();
        for binding in bindings {
            let key = (binding.client_id.clone(), binding.audience.clone());
            if indexed.insert(key, binding).is_some() {
                return Err(IxTransportError::InvalidPolicy);
            }
        }
        if indexed.is_empty() {
            return Err(IxTransportError::InvalidPolicy);
        }
        let required_roles = required_roles.into_iter().collect::<BTreeSet<_>>();
        let required_scopes = required_scopes.into_iter().collect::<BTreeSet<_>>();
        if required_roles.is_empty() || required_scopes.is_empty() {
            return Err(IxTransportError::InvalidPolicy);
        }
        for value in required_roles.iter().chain(required_scopes.iter()) {
            validate_policy_key(value)?;
        }
        Ok(Self {
            issuer,
            bindings: indexed,
            required_roles,
            required_scopes,
            release_id,
        })
    }

    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    pub fn release_id(&self) -> &str {
        &self.release_id
    }

    pub(crate) fn audiences(&self) -> HashSet<String> {
        self.bindings
            .values()
            .map(|binding| binding.audience.clone())
            .collect()
    }

    pub(crate) fn authorize_claims(
        &self,
        client_id: &str,
        audience: &str,
        roles: &BTreeSet<String>,
        scopes: &BTreeSet<String>,
        release_id: &str,
    ) -> Result<IxClientProfile, IxTransportError> {
        let binding = self
            .bindings
            .get(&(client_id.to_string(), audience.to_string()))
            .ok_or(IxTransportError::Forbidden)?;
        if release_id != self.release_id
            || !self.required_roles.iter().all(|role| roles.contains(role))
            || !self
                .required_scopes
                .iter()
                .all(|scope| scopes.contains(scope))
        {
            return Err(IxTransportError::Forbidden);
        }
        Ok(binding.profile)
    }

    pub(crate) fn authorize_origin(
        &self,
        origins: &[&str],
        human: &IxVerifiedHuman,
    ) -> Result<(), IxTransportError> {
        let binding = self
            .bindings
            .get(&(human.client_id.clone(), human.audience.clone()))
            .ok_or(IxTransportError::Forbidden)?;
        match binding.profile {
            IxClientProfile::Browser => {
                if origins.len() != 1 {
                    return Err(IxTransportError::Forbidden);
                }
                let origin = origins[0];
                let canonical =
                    canonical_origin(origin).map_err(|_| IxTransportError::Forbidden)?;
                if origin != canonical || !binding.allowed_origins.contains(&canonical) {
                    return Err(IxTransportError::Forbidden);
                }
            }
            IxClientProfile::Native | IxClientProfile::NonBrowser => {
                if !origins.is_empty() {
                    return Err(IxTransportError::Forbidden);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn allowed_origins(&self) -> impl Iterator<Item = &str> {
        self.bindings
            .values()
            .flat_map(|binding| binding.allowed_origins.iter().map(String::as_str))
    }
}

/// Sealed, token-free authenticated principal. It has no public constructor and
/// deliberately implements neither `Serialize` nor `Deserialize`.
#[derive(Clone, Debug)]
pub struct IxVerifiedHuman {
    principal: IxPrincipalBinding,
    roles: BTreeSet<String>,
    scopes: BTreeSet<String>,
    client_id: String,
    audience: String,
    profile: IxClientProfile,
    release_id: String,
    session_id: String,
}

impl IxVerifiedHuman {
    pub(crate) fn from_verified_claims(
        claims: IxVerifiedClaims,
        profile: IxClientProfile,
    ) -> Result<Self, IxTransportError> {
        let principal = IxPrincipalBinding {
            tenant_id: claims.tenant_id().to_string(),
            subject: claims.subject().to_string(),
            principal_type: crate::extension::RuntimePrincipalType::User,
            on_behalf_of: None,
        };
        Ok(Self {
            principal,
            roles: claims.roles().clone(),
            scopes: claims.scopes().clone(),
            client_id: claims.client_id().to_string(),
            audience: claims.audience().to_string(),
            profile,
            release_id: claims.release_id().to_string(),
            session_id: claims.session_id().to_string(),
        })
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

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    pub fn audience(&self) -> &str {
        &self.audience
    }

    pub fn profile(&self) -> IxClientProfile {
        self.profile
    }

    pub fn release_id(&self) -> &str {
        &self.release_id
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub(crate) fn context_claims(&self) -> Result<IxContextClaims, IxTransportError> {
        IxContextClaims::from_verified_human_parts(
            self.release_id.clone(),
            self.session_id.clone(),
            self.principal.clone(),
            self.roles.clone(),
            self.scopes.clone(),
        )
        .map_err(IxTransportError::from)
    }
}

#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
pub enum IxTransportError {
    #[error("IX transport policy is invalid")]
    InvalidPolicy,
    #[error("IX request is not authenticated")]
    Unauthorized,
    #[error("IX request is not authorized")]
    Forbidden,
    #[error("IX request is invalid")]
    InvalidRequest,
    #[error("IX run was not found")]
    NotFound,
    #[error("IX request conflicts with retained state")]
    Conflict,
    #[error("IX service is unavailable")]
    Unavailable,
    #[error("IX stream failed")]
    StreamFailed,
}

impl From<IxContractError> for IxTransportError {
    fn from(value: IxContractError) -> Self {
        match value {
            IxContractError::UnauthorizedActor | IxContractError::UnauthorizedPolicyAuthority => {
                Self::Forbidden
            }
            IxContractError::RunNotFound => Self::NotFound,
            IxContractError::CancellationConflict | IxContractError::StaleAuthorization => {
                Self::Conflict
            }
            IxContractError::AuditUnavailable => Self::Unavailable,
            _ => Self::InvalidRequest,
        }
    }
}

pub type IxCanonicalEnvelopeStream = BoxStream<'static, Result<IxCommitEnvelope, IxContractError>>;

pub struct IxTransportStream {
    initial: Vec<IxCommitEnvelope>,
    live: IxCanonicalEnvelopeStream,
}

impl IxTransportStream {
    pub(crate) fn from_canonical_replay(
        replay: IxReplayBatch,
        checkpoint: IxReplayCheckpoint,
        live: IxCanonicalEnvelopeStream,
    ) -> Result<Self, IxContractError> {
        IxReplayVerifier::verify(&replay, &checkpoint)?;
        let last_revision = replay.expected_snapshot.task_revision;
        let seen = replay
            .envelopes
            .iter()
            .map(|envelope| envelope.commit_id.clone())
            .collect::<HashSet<_>>();
        let dedup = Arc::new(Mutex::new((last_revision, seen)));
        let live = live
            .filter_map(move |item| {
                let dedup = Arc::clone(&dedup);
                async move {
                    match item {
                        Ok(envelope) => {
                            let mut state = dedup
                                .lock()
                                .unwrap_or_else(|poisoned| poisoned.into_inner());
                            if envelope.result_revision <= state.0
                                || !state.1.insert(envelope.commit_id.clone())
                            {
                                None
                            } else {
                                state.0 = envelope.result_revision;
                                Some(Ok(envelope))
                            }
                        }
                        Err(error) => Some(Err(error)),
                    }
                }
            })
            .boxed();
        Ok(Self {
            initial: replay.envelopes,
            live,
        })
    }

    pub(crate) fn into_envelopes(self) -> IxCanonicalEnvelopeStream {
        stream::iter(self.initial.into_iter().map(Ok))
            .chain(self.live)
            .boxed()
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IxTransportCancelReceipt {
    disposition: IxCancelDisposition,
    run_id: String,
    command_id: String,
    cursor: Option<String>,
    audit_retained: bool,
    audit_projected: IxAuditProjectedPosture,
    #[serde(skip_serializing_if = "Option::is_none")]
    audit_projection_cursor: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IxAuditProjectedPosture {
    Pending,
    Confirmed,
    Unknown,
}

impl IxTransportCancelReceipt {
    pub fn disposition(&self) -> IxCancelDisposition {
        self.disposition
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn command_id(&self) -> &str {
        &self.command_id
    }

    pub fn cursor(&self) -> Option<&str> {
        self.cursor.as_deref()
    }

    pub fn audit_retained(&self) -> bool {
        self.audit_retained
    }

    pub fn audit_projected(&self) -> IxAuditProjectedPosture {
        self.audit_projected
    }

    pub fn audit_projection_cursor(&self) -> Option<&str> {
        self.audit_projection_cursor.as_deref()
    }
}

/// Product-selectable intent and artifact policy with no actor-identity seam.
///
/// The Framework derives the policy authority and orchestration actor from the
/// verified tenant when a run is admitted. Product composition can select the
/// bounded intent label, artifact contract, and optional recipe, but cannot
/// inject either canonical actor into a runtime run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IxRuntimeIntentPolicy {
    intent_label: String,
    registered_artifacts: BTreeSet<IxArtifactBinding>,
    recipe_registration: Option<IxRecipeRegistration>,
}

impl IxRuntimeIntentPolicy {
    pub fn new(
        intent_label: impl Into<String>,
        registered_artifacts: impl IntoIterator<Item = IxArtifactBinding>,
    ) -> Result<Self, IxContractError> {
        let intent_label = intent_label.into();
        super::contract::validate_text("runtimeIntentPolicy.intentLabel", &intent_label, 512)?;
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
            registered_artifacts,
            recipe_registration: None,
        })
    }

    pub fn new_for_recipe(
        intent_label: impl Into<String>,
        recipe_registration: IxRecipeRegistration,
    ) -> Result<Self, IxContractError> {
        let artifact_binding = recipe_registration.artifact_binding()?;
        let mut policy = Self::new(intent_label, [artifact_binding])?;
        policy.recipe_registration = Some(recipe_registration);
        Ok(policy)
    }

    fn materialize(&self, tenant_id: &str) -> Result<IxRunPolicy, IxContractError> {
        validate_key("runtimeIntentPolicy.tenantId", tenant_id)?;
        Ok(IxRunPolicy {
            intent_label: self.intent_label.clone(),
            runner: framework_principal(
                tenant_id,
                IX_FRAMEWORK_RUNNER_SUBJECT,
                RuntimePrincipalType::Agent,
            ),
            registered_artifacts: self.registered_artifacts.clone(),
            recipe_registration: self.recipe_registration.clone(),
        })
    }
}

#[derive(Clone)]
struct IxLiveRun {
    owner: IxPrincipalBinding,
    cancellation: watch::Sender<bool>,
    envelopes: broadcast::Sender<IxCommitEnvelope>,
}

/// Framework-owned IX authority. Products provide bounded context, draft
/// orchestration, and durable storage, but cannot replace this service or
/// author canonical envelopes, replay, cancellation receipts, or audit facts.
///
/// The service owns the private stored-run sealer built from the mandatory
/// composition seal key; every record loaded from the repository is verified
/// against that sealer before replay, append, cancellation, owner acceptance,
/// or receipt handling.
#[derive(Clone)]
pub struct IxRuntimeService {
    repository: Arc<dyn IxRunRepository>,
    context_resolver: Arc<dyn IxProductContextResolver>,
    orchestrator: Arc<dyn IxProductOrchestrator>,
    context_policy: Arc<dyn IxContextPolicy>,
    sealer: Arc<IxStoredRunSealer>,
    run_policies: Arc<BTreeMap<String, IxRuntimeIntentPolicy>>,
    live: Arc<Mutex<HashMap<String, IxLiveRun>>>,
    execution_deadline: Duration,
}

impl IxRuntimeService {
    pub fn new(
        repository: Arc<dyn IxRunRepository>,
        context_resolver: Arc<dyn IxProductContextResolver>,
        orchestrator: Arc<dyn IxProductOrchestrator>,
        context_policy: Arc<dyn IxContextPolicy>,
        stored_run_seal_key: IxStoredRunSealKey,
        run_policies: impl IntoIterator<Item = (String, IxRuntimeIntentPolicy)>,
    ) -> Result<Self, IxTransportError> {
        let mut indexed = BTreeMap::new();
        for (intent_key, policy) in run_policies {
            validate_key("runtimeService.intentKey", &intent_key)
                .map_err(|_| IxTransportError::InvalidPolicy)?;
            if indexed.insert(intent_key, policy).is_some() {
                return Err(IxTransportError::InvalidPolicy);
            }
        }
        if indexed.is_empty() {
            return Err(IxTransportError::InvalidPolicy);
        }
        Ok(Self {
            repository,
            context_resolver,
            orchestrator,
            context_policy,
            sealer: Arc::new(IxStoredRunSealer::from_key(stored_run_seal_key)),
            run_policies: Arc::new(indexed),
            live: Arc::new(Mutex::new(HashMap::new())),
            execution_deadline: IX_MAX_RUN_DURATION,
        })
    }

    /// Selects a shorter foreground deadline without allowing composition to
    /// exceed the Framework's fixed 120-second ceiling.
    pub fn with_execution_deadline(
        mut self,
        execution_deadline: Duration,
    ) -> Result<Self, IxTransportError> {
        if execution_deadline.is_zero() || execution_deadline > IX_MAX_RUN_DURATION {
            return Err(IxTransportError::InvalidPolicy);
        }
        self.execution_deadline = execution_deadline;
        Ok(self)
    }

    pub(crate) async fn start(
        self: &Arc<Self>,
        request_context: &RequestContext,
        human: &IxVerifiedHuman,
        request: IxRunRequest,
    ) -> Result<IxTransportStream, IxTransportError> {
        request.validate().map_err(IxTransportError::from)?;
        let policy = self
            .run_policies
            .get(&request.intent_key)
            .ok_or(IxTransportError::Forbidden)?
            .materialize(&human.principal().tenant_id)
            .map_err(IxTransportError::from)?;
        let policy_authority = framework_principal(
            &human.principal().tenant_id,
            IX_FRAMEWORK_POLICY_SUBJECT,
            RuntimePrincipalType::Service,
        );
        let transaction = prepare_durable_start(
            request_context,
            human.principal().clone(),
            human.context_claims()?,
            &request,
            &policy,
            self.context_policy.as_ref(),
            &policy_authority,
            &self.sealer,
        )
        .map_err(IxTransportError::from)?;
        let stored = transaction.stored_run().clone();
        let final_envelope = stored
            .envelopes()
            .last()
            .cloned()
            .ok_or(IxTransportError::Unavailable)?;
        let (cancellation, cancellation_receiver) = watch::channel(false);
        let (sender, receiver) = broadcast::channel(128);
        self.reserve_live(
            stored.snapshot().run_id.clone(),
            human.principal().clone(),
            cancellation,
            sender,
        )?;
        let receipt = self
            .repository
            .create_run(transaction)
            .await
            .map_err(|_| IxTransportError::Unavailable)
            .and_then(|receipt| verify_repository_receipt(&receipt, &stored, &final_envelope));
        if let Err(error) = receipt {
            self.lock_live().remove(&stored.snapshot().run_id);
            return Err(error);
        }

        let service = Arc::clone(self);
        let owner = human.principal().clone();
        let run_id = stored.snapshot().run_id.clone();
        tokio::spawn(async move {
            service
                .run_product(owner, run_id, request, cancellation_receiver)
                .await;
        });

        let (replay, checkpoint) = stored
            .replay_after(human.principal(), None)
            .map_err(IxTransportError::from)?;
        IxTransportStream::from_canonical_replay(replay, checkpoint, broadcast_stream(receiver))
            .map_err(IxTransportError::from)
    }

    pub(crate) async fn replay(
        &self,
        human: &IxVerifiedHuman,
        cursor: &IxReplayCursor,
    ) -> Result<IxTransportStream, IxTransportError> {
        self.replay_from_repository(human, cursor.run_id(), Some(cursor.as_str()))
            .await
    }

    pub(crate) async fn cancel(
        &self,
        human: &IxVerifiedHuman,
        run_id: &str,
        command_id: &str,
    ) -> Result<IxTransportCancelReceipt, IxTransportError> {
        for _ in 0..3 {
            let stored = self.load_owned_run(human.principal(), run_id).await?;
            match prepare_durable_cancel(&stored, human.principal(), command_id, &self.sealer)
                .map_err(IxTransportError::from)?
            {
                IxDurableCancelPreparation::Retry(accepted) => {
                    self.signal_cancel(human.principal(), run_id);
                    self.project_cancel(accepted.audit_projection().clone());
                    return Ok(cancel_receipt(
                        IxCancelDisposition::Accepted,
                        run_id,
                        command_id,
                        Some(accepted.cursor().to_string()),
                        true,
                    ));
                }
                IxDurableCancelPreparation::AlreadyTerminal { cursor } => {
                    return Ok(cancel_receipt(
                        IxCancelDisposition::AlreadyTerminal,
                        run_id,
                        command_id,
                        Some(cursor),
                        false,
                    ));
                }
                IxDurableCancelPreparation::Accepted {
                    transaction,
                    accepted,
                } => {
                    let transaction = *transaction;
                    let next = transaction.stored_run().clone();
                    let envelope = transaction.envelope().clone();
                    match self.repository.compare_and_append(transaction).await {
                        Ok(receipt) => {
                            verify_repository_receipt(&receipt, &next, &envelope)?;
                            self.signal_cancel(human.principal(), run_id);
                            self.publish_live(run_id, envelope);
                            self.project_cancel(accepted.audit_projection().clone());
                            return Ok(cancel_receipt(
                                IxCancelDisposition::Accepted,
                                run_id,
                                command_id,
                                Some(accepted.cursor().to_string()),
                                true,
                            ));
                        }
                        Err(_) => continue,
                    }
                }
            }
        }
        Err(IxTransportError::Unavailable)
    }

    async fn replay_from_repository(
        &self,
        human: &IxVerifiedHuman,
        run_id: &str,
        after_cursor: Option<&str>,
    ) -> Result<IxTransportStream, IxTransportError> {
        let live = self
            .lock_live()
            .get(run_id)
            .filter(|live| live.owner == *human.principal())
            .map(|live| live.envelopes.subscribe());
        let stored = self.load_owned_run(human.principal(), run_id).await?;
        let (replay, checkpoint) = stored
            .replay_after(human.principal(), after_cursor)
            .map_err(IxTransportError::from)?;
        IxTransportStream::from_canonical_replay(
            replay,
            checkpoint,
            live.map(broadcast_stream)
                .unwrap_or_else(|| stream::empty().boxed()),
        )
        .map_err(IxTransportError::from)
    }

    fn reserve_live(
        &self,
        run_id: String,
        owner: IxPrincipalBinding,
        cancellation: watch::Sender<bool>,
        envelopes: broadcast::Sender<IxCommitEnvelope>,
    ) -> Result<(), IxTransportError> {
        let mut live = self.lock_live();
        let owner_active = live.values().filter(|run| run.owner == owner).count();
        if live.len() >= IX_MAX_ACTIVE_RUNS
            || owner_active >= IX_MAX_ACTIVE_RUNS_PER_OWNER
            || live.contains_key(&run_id)
        {
            return Err(IxTransportError::Conflict);
        }
        live.insert(
            run_id,
            IxLiveRun {
                owner,
                cancellation,
                envelopes,
            },
        );
        Ok(())
    }

    async fn run_product(
        self: Arc<Self>,
        owner: IxPrincipalBinding,
        run_id: String,
        request: IxRunRequest,
        cancellation_receiver: watch::Receiver<bool>,
    ) {
        let deadline = Instant::now() + self.execution_deadline;
        let mut cancellation = IxCancellationSignal::from_receiver(cancellation_receiver.clone());
        let context = tokio::select! {
            biased;
            state = cancellation.cancelled() => {
                self.finish_from_signal(&owner, &run_id, state).await;
                self.lock_live().remove(&run_id);
                return;
            }
            _ = tokio::time::sleep_until(deadline) => {
                let _ = self.commit_orchestration_failure(
                    &owner,
                    &run_id,
                    "The foreground run exceeded its bounded execution window.",
                ).await;
                self.lock_live().remove(&run_id);
                return;
            }
            result = self.context_resolver.resolve(
                &owner,
                &request,
                IxCancellationSignal::from_receiver(cancellation_receiver.clone()),
            ) => result,
        };
        let context = match context {
            Ok(context) => context,
            Err(_) => {
                let _ = self
                    .commit_orchestration_failure(
                        &owner,
                        &run_id,
                        "Product context resolution failed.",
                    )
                    .await;
                self.lock_live().remove(&run_id);
                return;
            }
        };
        match self
            .commit_resolved_context(&owner, &run_id, context.summary().clone())
            .await
        {
            Ok(true) => {}
            Ok(false) => {
                self.lock_live().remove(&run_id);
                return;
            }
            Err(_) => {
                let _ = self
                    .commit_orchestration_failure(
                        &owner,
                        &run_id,
                        "Product context could not be retained.",
                    )
                    .await;
                self.lock_live().remove(&run_id);
                return;
            }
        }

        let prepared = match IxPreparedRun::new(run_id.clone(), owner.clone(), request, context) {
            Ok(prepared) => prepared,
            Err(_) => {
                let _ = self
                    .commit_orchestration_failure(
                        &owner,
                        &run_id,
                        "Product orchestration input was invalid.",
                    )
                    .await;
                self.lock_live().remove(&run_id);
                return;
            }
        };
        let mut cancellation = IxCancellationSignal::from_receiver(cancellation_receiver.clone());
        let drafts = tokio::select! {
            biased;
            state = cancellation.cancelled() => {
                self.finish_from_signal(&owner, &run_id, state).await;
                self.lock_live().remove(&run_id);
                return;
            }
            _ = tokio::time::sleep_until(deadline) => {
                let _ = self.commit_orchestration_failure(
                    &owner,
                    &run_id,
                    "The foreground run exceeded its bounded execution window.",
                ).await;
                self.lock_live().remove(&run_id);
                return;
            }
            result = self.orchestrator.run(
                prepared,
                IxCancellationSignal::from_receiver(cancellation_receiver.clone()),
            ) => result,
        };
        let drafts = match drafts {
            Ok(drafts) => drafts,
            Err(_) => {
                let _ = self
                    .commit_orchestration_failure(
                        &owner,
                        &run_id,
                        "Product orchestration could not start.",
                    )
                    .await;
                self.lock_live().remove(&run_id);
                return;
            }
        };
        self.consume_drafts(
            &owner,
            &run_id,
            drafts,
            IxCancellationSignal::from_receiver(cancellation_receiver),
            deadline,
        )
        .await;
        self.lock_live().remove(&run_id);
    }

    async fn consume_drafts(
        &self,
        owner: &IxPrincipalBinding,
        run_id: &str,
        mut drafts: BoxStream<'static, Result<IxOrchestrationDraft, IxContractError>>,
        mut cancellation: IxCancellationSignal,
        deadline: Instant,
    ) {
        loop {
            let next = tokio::select! {
                biased;
                state = cancellation.cancelled() => {
                    self.finish_from_signal(owner, run_id, state).await;
                    return;
                }
                _ = tokio::time::sleep_until(deadline) => {
                    let _ = self.commit_orchestration_failure(
                        owner,
                        run_id,
                        "The foreground run exceeded its bounded execution window.",
                    ).await;
                    return;
                }
                next = drafts.next() => next,
            };
            let Some(draft) = next else {
                let _ = self
                    .commit_orchestration_failure(
                        owner,
                        run_id,
                        "Product orchestration ended without a terminal outcome.",
                    )
                    .await;
                return;
            };
            let Ok(draft) = draft.and_then(|draft| {
                draft.validate()?;
                Ok(draft)
            }) else {
                let _ = self
                    .commit_orchestration_failure(owner, run_id, "Product orchestration failed.")
                    .await;
                return;
            };
            match self.commit_draft(owner, run_id, draft).await {
                Ok(true) => return,
                Ok(false) => {}
                Err(_) => {
                    let _ = self
                        .commit_orchestration_failure(
                            owner,
                            run_id,
                            "Product output could not be retained within runtime limits.",
                        )
                        .await;
                    return;
                }
            }
        }
    }

    async fn finish_from_signal(
        &self,
        owner: &IxPrincipalBinding,
        run_id: &str,
        state: IxCancellationSignalState,
    ) {
        match state {
            IxCancellationSignalState::Requested => {
                let _ = self.confirm_cancelled(owner, run_id).await;
            }
            IxCancellationSignalState::Active | IxCancellationSignalState::ControllerClosed => {
                let _ = self
                    .commit_orchestration_failure(
                        owner,
                        run_id,
                        "The foreground cancellation controller became unavailable.",
                    )
                    .await;
            }
        }
    }

    async fn commit_resolved_context(
        &self,
        owner: &IxPrincipalBinding,
        run_id: &str,
        context: super::IxContextSummary,
    ) -> Result<bool, IxTransportError> {
        let stored = self.load_owned_run(owner, run_id).await?;
        if stored.snapshot().state == IxRunState::Cancelling {
            self.confirm_cancelled(owner, run_id).await?;
            return Ok(false);
        }
        if stored.snapshot().state == IxRunState::Terminal {
            return Ok(false);
        }
        let transaction = prepare_durable_append(
            &stored,
            stored.snapshot().runner.clone(),
            IxEventPayload::ContextResolved {
                authority: stored.snapshot().authorization_context.binding(),
                context,
            },
            "Begin the first meaningful phase of work.",
            false,
            &self.sealer,
        )
        .map_err(IxTransportError::from)?;
        let next = transaction.stored_run().clone();
        let envelope = transaction.envelope().clone();
        match self.repository.compare_and_append(transaction).await {
            Ok(receipt) => {
                verify_repository_receipt(&receipt, &next, &envelope)?;
                self.publish_live(run_id, envelope);
                Ok(true)
            }
            Err(_) => {
                let refreshed = self.load_owned_run(owner, run_id).await?;
                if refreshed.snapshot().state == IxRunState::Cancelling {
                    self.confirm_cancelled(owner, run_id).await?;
                    Ok(false)
                } else if refreshed.snapshot().state == IxRunState::Terminal {
                    Ok(false)
                } else {
                    Err(IxTransportError::Unavailable)
                }
            }
        }
    }

    async fn confirm_cancelled(
        &self,
        owner: &IxPrincipalBinding,
        run_id: &str,
    ) -> Result<(), IxTransportError> {
        for _ in 0..3 {
            let stored = self.load_owned_run(owner, run_id).await?;
            if stored.snapshot().state == IxRunState::Terminal {
                return Ok(());
            }
            if stored.snapshot().state != IxRunState::Cancelling {
                return Err(IxTransportError::Conflict);
            }
            let transaction = prepare_durable_cancel_confirmation(&stored, &self.sealer)
                .map_err(IxTransportError::from)?;
            let next = transaction.stored_run().clone();
            let envelope = transaction.envelope().clone();
            match self.repository.compare_and_append(transaction).await {
                Ok(receipt) => {
                    verify_repository_receipt(&receipt, &next, &envelope)?;
                    self.publish_live(run_id, envelope);
                    if next.snapshot().state == IxRunState::Terminal {
                        return Ok(());
                    }
                }
                Err(_) => continue,
            }
        }
        Err(IxTransportError::Unavailable)
    }

    async fn load_owned_run(
        &self,
        owner: &IxPrincipalBinding,
        run_id: &str,
    ) -> Result<super::IxStoredRun, IxTransportError> {
        let stored = self
            .repository
            .load_run(owner, run_id)
            .await
            .map_err(|_| IxTransportError::Unavailable)?
            .ok_or(IxTransportError::NotFound)?;
        stored
            .validate()
            .map_err(|_| IxTransportError::Unavailable)?;
        // Repository bytes are untrusted until the Framework sealer
        // authenticates them; failure stays non-oracular.
        self.sealer
            .verify(&stored)
            .map_err(|_| IxTransportError::Unavailable)?;
        if stored.snapshot().run_id != run_id || stored.snapshot().owner != *owner {
            return Err(IxTransportError::NotFound);
        }
        Ok(stored)
    }

    fn publish_live(&self, run_id: &str, envelope: IxCommitEnvelope) {
        if let Some(live) = self.lock_live().get(run_id) {
            let _ = live.envelopes.send(envelope);
        }
    }

    fn signal_cancel(&self, owner: &IxPrincipalBinding, run_id: &str) {
        if let Some(live) = self
            .lock_live()
            .get(run_id)
            .filter(|live| live.owner == *owner)
            .cloned()
        {
            live.cancellation.send_replace(true);
        }
    }

    fn project_cancel(&self, projection: IxAuditProjection) {
        let repository = Arc::clone(&self.repository);
        tokio::spawn(async move {
            let _ = repository
                .mark_audit_projection(&projection, IxAuditProjectionState::Projected)
                .await;
        });
    }

    async fn commit_draft(
        &self,
        owner: &IxPrincipalBinding,
        run_id: &str,
        draft: IxOrchestrationDraft,
    ) -> Result<bool, IxTransportError> {
        let stored = self.load_owned_run(owner, run_id).await?;
        if stored.snapshot().state == IxRunState::Cancelling {
            self.confirm_cancelled(owner, run_id).await?;
            return Ok(true);
        }
        if stored.snapshot().state == IxRunState::Terminal {
            return Ok(true);
        }
        let authority = stored.snapshot().authorization_context.binding();
        let runner = stored.snapshot().runner.clone();
        let (actor, payload, next_safe_move, terminal) = match draft {
            IxOrchestrationDraft::Phase { phase, reason } => (
                runner.clone(),
                IxEventPayload::PhaseChanged {
                    authority,
                    phase,
                    reason,
                },
                "Continue the named work or publish a useful artifact revision.",
                false,
            ),
            IxOrchestrationDraft::Artifact {
                artifact_id,
                artifact_type,
                content_schema_version,
                renderer_key,
                revision,
                status,
                presentation,
                changed_region_ids,
            } => (
                runner,
                IxEventPayload::ArtifactRevision {
                    authority,
                    artifact: IxArtifactRevision {
                        artifact_id,
                        artifact_type,
                        content_schema_version,
                        renderer_key,
                        revision,
                        status,
                        changed_region_ids,
                        presentation,
                    },
                },
                "Inspect, continue, redirect, or stop the progressively useful work.",
                false,
            ),
            IxOrchestrationDraft::WaitingForUser { prompt } => (
                runner,
                IxEventPayload::WaitingForUser { authority, prompt },
                "Answer, redirect, or stop this run.",
                false,
            ),
            IxOrchestrationDraft::Completed { message } => (
                super::contract::runtime_actor(),
                IxEventPayload::RunFinished {
                    authority: Some(authority),
                    outcome: IxTerminalOutcome::Completed,
                    message,
                },
                "Inspect the final work and its supporting context.",
                true,
            ),
            IxOrchestrationDraft::Partial { message } => (
                super::contract::runtime_actor(),
                IxEventPayload::RunFinished {
                    authority: Some(authority),
                    outcome: IxTerminalOutcome::Partial,
                    message,
                },
                "Inspect the final work and its supporting context.",
                true,
            ),
        };
        let transaction = prepare_durable_append(
            &stored,
            actor,
            payload,
            next_safe_move,
            terminal,
            &self.sealer,
        )
        .map_err(IxTransportError::from)?;
        let next = transaction.stored_run().clone();
        let envelope = transaction.envelope().clone();
        match self.repository.compare_and_append(transaction).await {
            Ok(receipt) => {
                verify_repository_receipt(&receipt, &next, &envelope)?;
                self.publish_live(run_id, envelope);
                Ok(terminal)
            }
            Err(_) => {
                let refreshed = self.load_owned_run(owner, run_id).await?;
                if refreshed.snapshot().state == IxRunState::Cancelling {
                    self.confirm_cancelled(owner, run_id).await?;
                    Ok(true)
                } else if refreshed.snapshot().state == IxRunState::Terminal {
                    Ok(true)
                } else {
                    Err(IxTransportError::Unavailable)
                }
            }
        }
    }

    async fn commit_orchestration_failure(
        &self,
        owner: &IxPrincipalBinding,
        run_id: &str,
        message: &str,
    ) -> Result<(), IxTransportError> {
        for _ in 0..3 {
            let stored = self.load_owned_run(owner, run_id).await?;
            if stored.snapshot().state == IxRunState::Cancelling {
                return self.confirm_cancelled(owner, run_id).await;
            }
            if stored.snapshot().state == IxRunState::Terminal {
                return Ok(());
            }
            let transaction = prepare_durable_append(
                &stored,
                super::contract::runtime_actor(),
                IxEventPayload::RunFinished {
                    authority: None,
                    outcome: IxTerminalOutcome::Failed,
                    message: message.to_string(),
                },
                "Inspect the failure and begin a new authorized run if appropriate.",
                true,
                &self.sealer,
            )
            .map_err(IxTransportError::from)?;
            let next = transaction.stored_run().clone();
            let envelope = transaction.envelope().clone();
            match self.repository.compare_and_append(transaction).await {
                Ok(receipt) => {
                    verify_repository_receipt(&receipt, &next, &envelope)?;
                    self.publish_live(run_id, envelope);
                    return Ok(());
                }
                Err(_) => continue,
            }
        }
        Err(IxTransportError::Unavailable)
    }

    fn lock_live(&self) -> std::sync::MutexGuard<'_, HashMap<String, IxLiveRun>> {
        self.live
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn broadcast_stream(receiver: broadcast::Receiver<IxCommitEnvelope>) -> IxCanonicalEnvelopeStream {
    stream::unfold(Some(receiver), |state| async move {
        let mut receiver = state?;
        match receiver.recv().await {
            Ok(envelope) => Some((Ok(envelope), Some(receiver))),
            Err(broadcast::error::RecvError::Lagged(_)) => {
                Some((Err(IxContractError::ResourceLimitExceeded), None))
            }
            Err(broadcast::error::RecvError::Closed) => None,
        }
    })
    .boxed()
}

fn verify_repository_receipt(
    receipt: &IxRepositoryCommitReceipt,
    stored: &super::IxStoredRun,
    envelope: &IxCommitEnvelope,
) -> Result<(), IxTransportError> {
    if receipt.run_id() != stored.snapshot().run_id
        || receipt.cursor() != stored.snapshot().cursor
        || receipt.revision() != stored.repository_version()
        || receipt.commit_id() != envelope.commit_id
        || receipt.deduplicated()
    {
        return Err(IxTransportError::Unavailable);
    }
    Ok(())
}

fn cancel_receipt(
    disposition: IxCancelDisposition,
    run_id: &str,
    command_id: &str,
    cursor: Option<String>,
    audit_retained: bool,
) -> IxTransportCancelReceipt {
    let audit_projected = if audit_retained {
        IxAuditProjectedPosture::Pending
    } else {
        IxAuditProjectedPosture::Unknown
    };
    IxTransportCancelReceipt {
        disposition,
        run_id: run_id.to_string(),
        command_id: command_id.to_string(),
        cursor,
        audit_retained,
        // Projection is asynchronous and at-least-once. Keep the immutable
        // accepted receipt at pending so retries remain byte-identical.
        audit_projected,
        audit_projection_cursor: None,
    }
}

fn framework_principal(
    tenant_id: &str,
    subject: &str,
    principal_type: RuntimePrincipalType,
) -> IxPrincipalBinding {
    IxPrincipalBinding {
        tenant_id: tenant_id.to_string(),
        subject: subject.to_string(),
        principal_type,
        on_behalf_of: None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IxReplayCursor {
    raw: String,
    run_id: String,
    revision: u64,
}

impl IxReplayCursor {
    pub fn parse(value: &str) -> Result<Self, IxTransportError> {
        let tail = value
            .strip_prefix("ix1.")
            .ok_or(IxTransportError::InvalidRequest)?;
        let (run_id, revision_text) = tail
            .rsplit_once('.')
            .ok_or(IxTransportError::InvalidRequest)?;
        let uuid = Uuid::parse_str(run_id).map_err(|_| IxTransportError::InvalidRequest)?;
        let revision = revision_text
            .parse::<u64>()
            .map_err(|_| IxTransportError::InvalidRequest)?;
        if uuid.to_string() != run_id || revision == 0 || revision.to_string() != revision_text {
            return Err(IxTransportError::InvalidRequest);
        }
        let canonical = format!("ix1.{run_id}.{revision}");
        if canonical != value {
            return Err(IxTransportError::InvalidRequest);
        }
        Ok(Self {
            raw: canonical,
            run_id: run_id.to_string(),
            revision,
        })
    }

    pub fn as_str(&self) -> &str {
        &self.raw
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }
}

fn canonical_origin(value: &str) -> Result<String, IxTransportError> {
    if !value.is_ascii() || value.trim() != value {
        return Err(IxTransportError::InvalidPolicy);
    }
    let (scheme, authority) = if let Some(authority) = value.strip_prefix("https://") {
        ("https", authority)
    } else if let Some(authority) = value.strip_prefix("http://") {
        ("http", authority)
    } else {
        return Err(IxTransportError::InvalidPolicy);
    };
    if authority.is_empty()
        || authority.contains(['/', '?', '#', '@'])
        || authority.ends_with('.')
        || authority.chars().any(char::is_uppercase)
    {
        return Err(IxTransportError::InvalidPolicy);
    }

    let (host, port, ipv6) = if let Some(bracketed) = authority.strip_prefix('[') {
        let close = bracketed.find(']').ok_or(IxTransportError::InvalidPolicy)?;
        let host = &bracketed[..close];
        let suffix = &bracketed[close + 1..];
        let port = if suffix.is_empty() {
            None
        } else {
            Some(
                suffix
                    .strip_prefix(':')
                    .ok_or(IxTransportError::InvalidPolicy)?,
            )
        };
        (host, port, true)
    } else {
        if authority.contains('[') || authority.contains(']') {
            return Err(IxTransportError::InvalidPolicy);
        }
        match authority.rsplit_once(':') {
            Some((host, port)) if !host.contains(':') => (host, Some(port), false),
            Some(_) => return Err(IxTransportError::InvalidPolicy),
            None => (authority, None, false),
        }
    };
    if host.is_empty() {
        return Err(IxTransportError::InvalidPolicy);
    }

    let loopback = if ipv6 {
        let address = host
            .parse::<Ipv6Addr>()
            .map_err(|_| IxTransportError::InvalidPolicy)?;
        if address.to_string() != host {
            return Err(IxTransportError::InvalidPolicy);
        }
        address == Ipv6Addr::LOCALHOST
    } else if let Ok(address) = host.parse::<Ipv4Addr>() {
        if address.to_string() != host {
            return Err(IxTransportError::InvalidPolicy);
        }
        address == Ipv4Addr::LOCALHOST
    } else {
        if host != "localhost" && !valid_ascii_dns_name(host) {
            return Err(IxTransportError::InvalidPolicy);
        }
        host == "localhost"
    };

    let port = port
        .map(|text| {
            let parsed = text
                .parse::<u16>()
                .map_err(|_| IxTransportError::InvalidPolicy)?;
            if parsed == 0 || parsed.to_string() != text {
                return Err(IxTransportError::InvalidPolicy);
            }
            Ok(parsed)
        })
        .transpose()?;
    if (scheme == "https" && port == Some(443))
        || (scheme == "http" && (port == Some(80) || port.is_none()))
        || (scheme == "http" && !loopback)
    {
        return Err(IxTransportError::InvalidPolicy);
    }
    Ok(value.to_string())
}

fn valid_ascii_dns_name(host: &str) -> bool {
    host.len() <= 253
        && host.contains('.')
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
}

fn validate_policy_key(value: &str) -> Result<(), IxTransportError> {
    validate_transport_key(value).map_err(|_| IxTransportError::InvalidPolicy)
}

fn validate_transport_key(value: &str) -> Result<(), IxTransportError> {
    if value.is_empty()
        || value.len() > 256
        || value.trim() != value
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'.' | b'_' | b':' | b'/' | b'@' | b'-' | b'+')
        })
    {
        Err(IxTransportError::InvalidRequest)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verified_human(profile: IxClientProfile) -> IxVerifiedHuman {
        IxVerifiedHuman {
            principal: IxPrincipalBinding {
                tenant_id: "tenant-1".to_string(),
                subject: "human-1".to_string(),
                principal_type: RuntimePrincipalType::User,
                on_behalf_of: None,
            },
            roles: BTreeSet::from(["ix.user".to_string()]),
            scopes: BTreeSet::from(["ix.run".to_string()]),
            client_id: match profile {
                IxClientProfile::Browser => "nexus-browser",
                IxClientProfile::Native => "nexus-native",
                IxClientProfile::NonBrowser => "nexus-service",
            }
            .to_string(),
            audience: "api://nexus-ix".to_string(),
            profile,
            release_id: "release-1".to_string(),
            session_id: "session-1".to_string(),
        }
    }

    fn policy(binding: IxClientBinding) -> IxTransportPolicy {
        IxTransportPolicy::new(
            "https://issuer.example.com",
            "release-1",
            [binding],
            ["ix.user".to_string()],
            ["ix.run".to_string()],
        )
        .expect("IX transport policy")
    }

    #[test]
    fn origin_policy_accepts_only_exact_canonical_supported_origins() {
        for origin in [
            "https://nexus.example.com",
            "https://nexus.example.com:8443",
            "https://127.0.0.1",
            "https://[2001:db8::1]",
            "http://localhost:5173",
            "http://127.0.0.1:5173",
            "http://[::1]:5173",
        ] {
            IxClientBinding::new(
                "nexus-browser",
                "api://nexus-ix",
                IxClientProfile::Browser,
                [origin.to_string()],
            )
            .unwrap_or_else(|error| panic!("canonical origin {origin} was rejected: {error}"));
        }

        for origin in [
            "*",
            "null",
            "HTTPS://nexus.example.com",
            "https://NEXUS.example.com",
            "https://nexus.example.com/",
            "https://nexus.example.com.",
            "https://nexus.example.com:443",
            "https://user@nexus.example.com",
            "https://nexus.example.com/path",
            "https://nexus.example.com?query",
            "https://nexus.example.com#fragment",
            "https://nexüs.example.com",
            "http://nexus.example.com:8080",
            "http://localhost",
            "http://127.0.0.1:080",
            "http://[0:0:0:0:0:0:0:1]:5173",
        ] {
            assert!(
                IxClientBinding::new(
                    "nexus-browser",
                    "api://nexus-ix",
                    IxClientProfile::Browser,
                    [origin.to_string()],
                )
                .is_err(),
                "unsupported or noncanonical origin was accepted: {origin}"
            );
        }
    }

    #[test]
    fn request_origin_is_profile_bound_and_byte_exact() {
        let browser = verified_human(IxClientProfile::Browser);
        let browser_policy = policy(
            IxClientBinding::new(
                "nexus-browser",
                "api://nexus-ix",
                IxClientProfile::Browser,
                ["https://nexus.example.com".to_string()],
            )
            .expect("browser binding"),
        );
        assert!(browser_policy
            .authorize_origin(&["https://nexus.example.com"], &browser)
            .is_ok());
        for origins in [
            Vec::<&str>::new(),
            vec!["https://NEXUS.example.com"],
            vec!["https://nexus.example.com/"],
            vec!["https://nexus.example.com", "https://nexus.example.com"],
        ] {
            assert_eq!(
                browser_policy.authorize_origin(&origins, &browser),
                Err(IxTransportError::Forbidden)
            );
        }

        let native = verified_human(IxClientProfile::Native);
        let native_policy = policy(
            IxClientBinding::new(
                "nexus-native",
                "api://nexus-ix",
                IxClientProfile::Native,
                Vec::<String>::new(),
            )
            .expect("native binding"),
        );
        assert!(native_policy.authorize_origin(&[], &native).is_ok());
        assert_eq!(
            native_policy.authorize_origin(&["https://nexus.example.com"], &native),
            Err(IxTransportError::Forbidden)
        );
    }

    #[test]
    fn replay_cursor_requires_canonical_run_and_revision() {
        let run_id = "00000000-0000-4000-8000-000000000001";
        let cursor =
            IxReplayCursor::parse(&format!("ix1.{run_id}.42")).expect("canonical replay cursor");
        assert_eq!(cursor.run_id(), run_id);
        assert_eq!(cursor.revision(), 42);
        for invalid in [
            format!("ix1.{run_id}.0"),
            format!("ix1.{run_id}.042"),
            format!("IX1.{run_id}.42"),
            "ix1.00000000-0000-4000-8000-00000000000A.42".to_string(),
            format!("ix1.{run_id}.42.extra"),
        ] {
            assert_eq!(
                IxReplayCursor::parse(&invalid),
                Err(IxTransportError::InvalidRequest),
                "noncanonical replay cursor was accepted: {invalid}"
            );
        }
    }
}
