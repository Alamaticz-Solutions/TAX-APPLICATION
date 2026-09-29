pub const APP_NAME: &str = "pds-nexus";
pub const PRODUCT_SCHEMA: &str = "nexus";
pub const BACKEND_PROVIDER: &str = "PostgreSQL";

mod services;

#[cfg(feature = "chat")]
mod host_glue {
    use std::sync::Arc;

    use appfw_runtime::host::RuntimeMode;
    use appfw_runtime::ix::{
        ix_transport_routes, IxCancellationSignal, IxCancellationSignalState, IxClientBinding,
        IxClientProfile, IxContractError, IxJwtVerifier, IxOrchestrationDraft, IxPreparedRun,
        IxPrincipalBinding, IxProductContext, IxProductContextResolver, IxProductOrchestrator,
        IxRunRequest, IxRuntimeIntentPolicy, IxRuntimeService, IxStoredRunSealKey,
        IxTransportPolicy,
    };
    use appfw_runtime::observability::MetricsRegistry;
    use appfw_runtime::product_ui::product_ui_routes_if_present;
    use appfw_runtime::routing::{assemble_runtime_router_for_mode, RuntimeRouteSet};
    use appfw_runtime::security::SecurityConfig;
    use async_trait::async_trait;
    use axum::Router;
    use futures_util::{stream, StreamExt};
    use tower_http::cors::CorsLayer;

    use crate::services::ix::audit::NexusIxAuditSink;
    use crate::services::ix::context_policy::NexusIxContextPolicy;
    use crate::services::ix::orchestration::{
        plan_my_work_run, resolve_my_work_context, select_recipe_registration, NexusIxContextResolution,
        NexusIxResolvedContext, NexusIxRunDraft, NexusIxRunPlan, NexusMyWorkItem, NexusWorkRisk,
        NEXUS_MY_WORK_INTENT_KEY,
    };
    use crate::services::ix::repository::NexusIxRunRepository;

    /// Work-queue owner subject that receives the tenant My Work projection.
    pub const MY_WORK_OWNER_SUBJECT: &str = "nexus.user";
    pub const HOST_CLIENT_ID: &str = "nexus-browser";
    pub const HOST_AUDIENCE: &str = "api://nexus-ix";
    pub const HOST_RELEASE_ID: &str = "release-1";
    pub const HOST_ROLE: &str = "ix.user";
    pub const HOST_SCOPE: &str = "ix.run";

    /// Thin newtype: [`IxProductContextResolver`] by delegation to
    /// `services/ix/orchestration.rs` (`resolve_my_work_context`).
    /// Reuses `appfw_runtime::ix::IxProductContextResolver`. Lift: NONE.
    pub struct HostProductContextResolver {
        repository: NexusIxRunRepository,
    }

    /// Thin newtype: [`IxProductOrchestrator`] by delegation to
    /// `services/ix/orchestration.rs` (`plan_my_work_run`).
    /// Reuses `appfw_runtime::ix::IxProductOrchestrator`. Lift: NONE.
    ///
    /// `proof_hold` is proof-only. Production compose leaves it unset.
    pub struct HostProductOrchestrator {
        proof_hold: Option<Arc<tokio::sync::Notify>>,
    }

    #[async_trait]
    impl IxProductContextResolver for HostProductContextResolver {
        async fn resolve(
            &self,
            principal: &IxPrincipalBinding,
            request: &IxRunRequest,
            cancellation: IxCancellationSignal,
        ) -> Result<IxProductContext, IxContractError> {
            if cancellation.state() != IxCancellationSignalState::Active {
                return Err(IxContractError::InvalidLifecycle);
            }
            if select_recipe_registration(&request.intent_key)?.is_none() {
                return Err(IxContractError::UnknownArtifactType);
            }
            let items = load_my_work_items(&self.repository, principal).await?;
            let evaluated_at = chrono::Utc::now().to_rfc3339();
            match resolve_my_work_context(
                &principal.tenant_id,
                &principal.subject,
                &evaluated_at,
                &items,
            ) {
                NexusIxContextResolution::Unavailable { .. } => {
                    Err(IxContractError::UnauthorizedActor)
                }
                NexusIxContextResolution::Available(resolved) => {
                    to_product_context(*resolved)
                }
            }
        }
    }

    #[async_trait]
    impl IxProductOrchestrator for HostProductOrchestrator {
        async fn run(
            &self,
            prepared: IxPreparedRun,
            mut cancellation: IxCancellationSignal,
        ) -> Result<
            futures_util::stream::BoxStream<
                'static,
                Result<IxOrchestrationDraft, IxContractError>,
            >,
            IxContractError,
        > {
            if cancellation.state() != IxCancellationSignalState::Active {
                return Err(IxContractError::InvalidLifecycle);
            }
            if let Some(hold) = &self.proof_hold {
                tokio::select! {
                    _ = hold.notified() => {}
                    _ = cancellation.cancelled() => {}
                }
                if cancellation.state() != IxCancellationSignalState::Active {
                    return Ok(stream::empty().boxed());
                }
            }
            let items = items_from_context_payload(prepared.context().expose_to_product())?;
            let resolution = NexusIxContextResolution::Available(Box::new(NexusIxResolvedContext {
                context_id: prepared.context().context_id().to_string(),
                summary: prepared.context().summary().clone(),
                payload: prepared.context().expose_to_product().clone(),
            }));
            let presentation_id = format!("brief-{}", prepared.run_id());
            let plan = plan_my_work_run(&presentation_id, &resolution, &items)?;
            let drafts = match plan {
                NexusIxRunPlan::Unavailable { .. } => {
                    return Err(IxContractError::UnauthorizedActor);
                }
                NexusIxRunPlan::Drafts(drafts) => drafts
                    .into_iter()
                    .map(to_framework_draft)
                    .collect::<Result<Vec<_>, _>>()?,
            };
            Ok(stream::iter(drafts.into_iter().map(Ok)).boxed())
        }
    }

    fn to_product_context(
        resolved: NexusIxResolvedContext,
    ) -> Result<IxProductContext, IxContractError> {
        IxProductContext::new(resolved.context_id, resolved.summary, resolved.payload)
    }

    fn to_framework_draft(draft: NexusIxRunDraft) -> Result<IxOrchestrationDraft, IxContractError> {
        Ok(match draft {
            NexusIxRunDraft::Phase { phase, reason } => IxOrchestrationDraft::Phase { phase, reason },
            NexusIxRunDraft::Artifact {
                artifact_id,
                artifact_type,
                content_schema_version,
                renderer_key,
                revision,
                status,
                changed_region_ids,
                presentation,
            } => IxOrchestrationDraft::Artifact {
                artifact_id,
                artifact_type,
                content_schema_version,
                renderer_key,
                revision,
                status,
                presentation,
                changed_region_ids,
            },
            NexusIxRunDraft::Completed { message } => IxOrchestrationDraft::Completed { message },
        })
    }

    fn items_from_context_payload(
        payload: &serde_json::Value,
    ) -> Result<Vec<NexusMyWorkItem>, IxContractError> {
        let Some(rows) = payload.get("items").and_then(|value| value.as_array()) else {
            return Ok(Vec::new());
        };
        let mut items = Vec::new();
        for row in rows {
            items.push(NexusMyWorkItem {
                record_locator: row
                    .get("recordLocator")
                    .and_then(|value| value.as_str())
                    .ok_or(IxContractError::InvalidLifecycle)?
                    .to_string(),
                title: row
                    .get("title")
                    .and_then(|value| value.as_str())
                    .unwrap_or("Untitled")
                    .to_string(),
                owner_team: row
                    .get("ownerTeam")
                    .and_then(|value| value.as_str())
                    .unwrap_or("Unknown team")
                    .to_string(),
                phase: row
                    .get("phase")
                    .and_then(|value| value.as_str())
                    .unwrap_or("Unknown")
                    .to_string(),
                risk: match row.get("risk").and_then(|value| value.as_str()) {
                    Some("High risk") | Some("High") => NexusWorkRisk::High,
                    Some("Medium risk") | Some("Medium") => NexusWorkRisk::Medium,
                    _ => NexusWorkRisk::Low,
                },
                days_stalled: row
                    .get("daysStalled")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0) as u32,
                refreshed_at: row
                    .get("refreshedAt")
                    .and_then(|value| value.as_str())
                    .unwrap_or("1970-01-01T00:00:00Z")
                    .to_string(),
            });
        }
        Ok(items)
    }

    async fn load_my_work_items(
        repository: &NexusIxRunRepository,
        principal: &IxPrincipalBinding,
    ) -> Result<Vec<NexusMyWorkItem>, IxContractError> {
        let client = repository
            .pool_for_audit()
            .get()
            .await
            .map_err(|_| IxContractError::AuditUnavailable)?;
        let rows = client
            .query(
                "SELECT record_locator, title, owner_team, phase, risk, days_stalled, refreshed_at \
                 FROM nexus_my_work_queue WHERE tenant = $1 AND subject = $2 \
                 ORDER BY record_locator",
                &[&principal.tenant_id, &principal.subject],
            )
            .await
            .map_err(|_| IxContractError::AuditUnavailable)?;
        let mut items = Vec::new();
        for row in rows {
            let days: i32 = row.get(5);
            let refreshed: chrono::DateTime<chrono::Utc> = row.get(6);
            items.push(NexusMyWorkItem {
                record_locator: row.get(0),
                title: row.get(1),
                owner_team: row.get(2),
                phase: row.get(3),
                risk: match row.get::<_, String>(4).as_str() {
                    "High" | "High risk" => NexusWorkRisk::High,
                    "Medium" | "Medium risk" => NexusWorkRisk::Medium,
                    _ => NexusWorkRisk::Low,
                },
                days_stalled: days.max(0) as u32,
                refreshed_at: refreshed.to_rfc3339(),
            });
        }
        Ok(items)
    }

    /// Ensures the live My Work projection table exists and carries the
    /// owner-scoped rows used by the Host journey. This is disposable-PG
    /// composition, not a durable `ix_contexts` row.
    pub async fn ensure_my_work_projection(
        repository: &NexusIxRunRepository,
    ) -> Result<(), IxContractError> {
        let client = repository
            .pool_for_audit()
            .get()
            .await
            .map_err(|_| IxContractError::AuditUnavailable)?;
        client
            .batch_execute(
                "CREATE TABLE IF NOT EXISTS nexus_my_work_queue (
                    record_locator varchar PRIMARY KEY,
                    tenant varchar NOT NULL,
                    subject varchar NOT NULL,
                    title varchar NOT NULL,
                    owner_team varchar NOT NULL,
                    phase varchar NOT NULL,
                    risk varchar NOT NULL,
                    days_stalled integer NOT NULL,
                    refreshed_at timestamptz NOT NULL
                 );
                 INSERT INTO nexus_my_work_queue
                    (record_locator, tenant, subject, title, owner_team, phase, risk, days_stalled, refreshed_at)
                 VALUES
                    ('rl_11111111111111111111111111111111', 'tenant_a', 'nexus.user',
                     'Credentialing packet', 'Provider onboarding', 'Licensing', 'High', 8,
                     '2026-08-15T00:00:00Z'),
                    ('rl_22222222222222222222222222222222', 'tenant_a', 'nexus.user',
                     'Equipment approval', 'Clinical operations', 'Buildout', 'Medium', 6,
                     '2026-08-15T00:00:00Z')
                 ON CONFLICT (record_locator) DO NOTHING;",
            )
            .await
            .map_err(|_| IxContractError::AuditUnavailable)?;
        Ok(())
    }

    fn host_security() -> SecurityConfig {
        SecurityConfig {
            admin_ui_enabled: false,
            admin_troubleshooting_enabled: false,
            product_ui_enabled: true,
            chat_enabled: true,
            chat_required_roles: vec![HOST_ROLE.to_string()],
            chat_required_scopes: vec![HOST_SCOPE.to_string()],
            chat_prompt_audit_enabled: true,
            chat_prompt_audit_siem_enabled: true,
            chat_prompt_audit_sink: Some("host-local".to_string()),
            chat_prompt_audit_retention_days: 90,
            chat_kill_switch_active: false,
            #[cfg(feature = "mcp")]
            mcp_enabled: false,
            #[cfg(feature = "mcp")]
            mcp_mutations_enabled: false,
            #[cfg(feature = "mcp")]
            mcp_allowed_origins: vec![],
            #[cfg(feature = "mcp")]
            mcp_max_result_bytes: 256 * 1024,
            #[cfg(feature = "mcp")]
            mcp_max_resource_bytes: 256 * 1024,
            #[cfg(feature = "mcp")]
            mcp_max_batch_items: 20,
            #[cfg(feature = "mcp")]
            mcp_required_roles: vec!["admin".to_string()],
            #[cfg(feature = "mcp")]
            mcp_required_scopes: vec![],
            #[cfg(feature = "mcp")]
            mcp_privileged_tool_roles: vec!["admin".to_string()],
            #[cfg(feature = "mcp")]
            mcp_privileged_tool_scopes: vec!["appfw:mcp.admin".to_string()],
            graphql_introspection_enabled: false,
            graphql_introspection_required_roles: vec!["admin".to_string()],
            graphql_introspection_required_scopes: vec!["developer".to_string()],
            graphql_max_depth: 12,
            graphql_max_complexity: 500,
            request_body_limit_bytes: 1024 * 1024,
            rate_limit_per_second: 100,
            rate_limit_burst: 100,
        }
    }

    /// Constructed Host IX surface. Reuses `IxRuntimeService`, `IxJwtVerifier`,
    /// and `ix_transport_routes`. Does not mint NexusIxRuntime / NexusJwtVerifier
    /// / NexusIxTransport. Lift: NONE.
    pub struct HostIxAssembly {
        pub repository: NexusIxRunRepository,
        pub audit_sink: Arc<NexusIxAuditSink>,
        pub service: Arc<IxRuntimeService>,
        pub verifier: Arc<IxJwtVerifier>,
        pub policy: Arc<IxTransportPolicy>,
        pub router: Router,
    }

    pub struct HostIxCompose {
        pub postgres_url: String,
        pub issuer: String,
        pub frontend_origin: String,
        pub frontend_dist: Option<std::path::PathBuf>,
        pub seal_key: IxStoredRunSealKey,
        pub client_profile: IxClientProfile,
        /// Proof-only orchestrator hold. Production Host leaves this `None`.
        pub proof_orchestrator_hold: Option<Arc<tokio::sync::Notify>>,
    }

    pub async fn compose_host_ix(compose: HostIxCompose) -> anyhow::Result<HostIxAssembly> {
        let repository = NexusIxRunRepository::connect_local(&compose.postgres_url, 8)
            .map_err(|error| anyhow::anyhow!("repository pool: {error}"))?;
        // Production composition consumes an already-present My Work
        // projection. Disposable DDL/seed stays in the proof harness.
        let audit_sink = Arc::new(NexusIxAuditSink::new());
        let resolver = HostProductContextResolver {
            repository: repository.clone(),
        };
        let recipe = crate::services::ix::orchestration::my_work_recipe_registration()
            .map_err(|error| anyhow::anyhow!("recipe: {error}"))?;
        let intent_policy = IxRuntimeIntentPolicy::new_for_recipe("My Work", recipe)
            .map_err(|error| anyhow::anyhow!("intent policy: {error}"))?;
        let service = Arc::new(
            IxRuntimeService::new(
                Arc::new(repository.clone()),
                Arc::new(resolver),
                Arc::new(HostProductOrchestrator {
                    proof_hold: compose.proof_orchestrator_hold,
                }),
                Arc::new(NexusIxContextPolicy::new()),
                compose.seal_key,
                [(NEXUS_MY_WORK_INTENT_KEY.to_string(), intent_policy)],
            )
            .map_err(|error| anyhow::anyhow!("runtime service: {error}"))?,
        );
        let allowed_origins = match compose.client_profile {
            IxClientProfile::Browser => vec![compose.frontend_origin],
            IxClientProfile::Native | IxClientProfile::NonBrowser => Vec::new(),
        };
        let policy = Arc::new(
            IxTransportPolicy::new(
                compose.issuer,
                HOST_RELEASE_ID,
                [IxClientBinding::new(
                    HOST_CLIENT_ID,
                    HOST_AUDIENCE,
                    compose.client_profile,
                    allowed_origins,
                )
                .map_err(|error| anyhow::anyhow!("client binding: {error}"))?],
                [HOST_ROLE.to_string()],
                [HOST_SCOPE.to_string()],
            )
            .map_err(|error| anyhow::anyhow!("transport policy: {error}"))?,
        );
        let verifier = Arc::new(
            IxJwtVerifier::new(Arc::clone(&policy))
                .await
                .map_err(|error| anyhow::anyhow!("jwt verifier: {error}"))?,
        );
        let ix_routes = ix_transport_routes(
            Arc::clone(&verifier),
            Arc::clone(&policy),
            Arc::clone(&service),
        )
        .map_err(|error| anyhow::anyhow!("ix routes: {error}"))?;
        let mut routes = RuntimeRouteSet::new().with_ix_chat(ix_routes);
        if let Some(dist) = compose.frontend_dist.as_ref() {
            std::env::set_var(
                appfw_runtime::product_ui::PRODUCT_UI_DIST_DIR_ENV_VAR,
                dist,
            );
            if let Some(ui) = product_ui_routes_if_present(dist, ["/chat", "/nexus-ix"]) {
                routes = routes.with_product_ui(ui);
            }
        }
        let security = host_security();
        let router = assemble_runtime_router_for_mode(
            routes,
            CorsLayer::new(),
            MetricsRegistry::new("pds-nexus-ix-host", "local"),
            &security,
            &RuntimeMode::all(),
        );
        Ok(HostIxAssembly {
            repository,
            audit_sink,
            service,
            verifier,
            policy,
            router,
        })
    }
}

#[cfg(feature = "chat")]
pub use host_glue::{
    compose_host_ix, ensure_my_work_projection, HostIxAssembly, HostIxCompose,
    HostProductContextResolver, HostProductOrchestrator, HOST_AUDIENCE, HOST_CLIENT_ID,
    HOST_RELEASE_ID, HOST_ROLE, HOST_SCOPE, MY_WORK_OWNER_SUBJECT,
};

#[cfg(feature = "chat")]
pub use services::ix::audit::NexusIxAuditSink;
#[cfg(feature = "chat")]
pub use services::ix::repository::NexusIxRunRepository;
