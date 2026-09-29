//! Database-driven RBAC resolver (docs/architecture/rbac-design.md).
//!
//! This is the **one** reader of the `app_user` / `user_role` / `role` / `role_permission` /
//! `permission` tables. Everything else — the shared Rego grant rule in
//! `.appfw/model/schemas/tax_routing/rbac/*.rego`, the `cancel_record` / `reassign_record`
//! custom methods, and the `myPermissions` query the frontend reads — consumes this module's
//! output, so they can never disagree about what a user may do.
//!
//! `evaluate_user_access` (in `config/app_config.rs`) runs synchronously deep inside the
//! generic query-building path, so it cannot itself await a database round trip. This cache
//! is the bridge: it is refreshed from Postgres on a timer, and every lookup after that is an
//! in-memory read. A grant edited through the RBAC admin screens takes up to the refresh
//! interval to take effect everywhere; nothing here ever falls back to a role name.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Duration,
};

use appfw_provider_postgres::{
    postgres_execution_client as provider_postgres_execution_client,
    validate_postgres_connection_security as provider_validate_postgres_connection_security,
    PostgresConnectionConfig, PostgresExecutionClient,
};
use serde::Serialize;
use tracing::{debug, error, info, warn};

use crate::{
    product_api::AppError,
    schemas::system::{DataSource, DataSourceEnvironment},
};

/// How often the cache re-reads the RBAC tables from Postgres.
const REFRESH_INTERVAL: Duration = Duration::from_secs(30);

/// One resolved grant: a role, via `role_permission`, holding a `permission`. Produced entirely
/// from rows in the database — never constructed from a role name or permission name in code.
#[derive(Clone, Debug, Serialize)]
pub struct Grant {
    /// `permission.code`, e.g. "routing_record.cancel". What `authz::require` and the frontend's
    /// `usePermission(code)` match on.
    pub code: String,
    /// `permission.resource`: the entity this grant governs a CRUD action on, or `None` for a
    /// capability with no matching entity (checked only via `require`, never by Rego).
    pub resource: Option<String>,
    /// `permission.action`: "create" | "read" | "update" | "delete", or `None`.
    pub action: Option<String>,
    /// `role_permission.scope`: "all" | "own" | "active_only".
    pub scope: String,
    /// `permission.owner_field`: the column an "own"-scope grant filters on.
    pub owner_field: Option<String>,
}

/// The caller's resolved identity and grants. `user_id` is `None` only when the token names a
/// user this cache has never seen (unknown app_user row) — such a caller has no grants either.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Principal {
    pub user_id: Option<String>,
    pub grants: Vec<Grant>,
}

impl Principal {
    /// Whether any of the caller's grants carries this exact permission code. Used by custom
    /// methods (`cancel_record`, `reassign_record`, RBAC administration) for capabilities that
    /// have no row filter of their own.
    pub fn has_permission(&self, code: &str) -> bool {
        self.grants.iter().any(|g| g.code == code)
    }
}

type CacheKey = (String, String); // (tenant_id, user_name)

pub struct AuthzCache {
    execution: PostgresExecutionClient,
    principals: RwLock<HashMap<CacheKey, Principal>>,
}

impl AuthzCache {
    /// Connects to the `pg_primary` data source, loads the RBAC tables once, and spawns a
    /// background task that keeps reloading them for the life of the process. A failed initial
    /// load does not stop the backend from starting — it starts with an empty cache, which
    /// denies everything until the database is reachable, rather than failing closed on start.
    pub async fn init(sources: &HashMap<String, DataSource>) -> Result<Arc<Self>, AppError> {
        let data_source_env = resolve_data_source_env(sources, "pg_primary")?;

        let security = provider_validate_postgres_connection_security(
            &data_source_env.name,
            &data_source_env.security_profile,
            &data_source_env.tls_mode,
            &data_source_env.db_host,
        )
        .map_err(AppError::from)?;

        let connection = PostgresConnectionConfig::new(
            data_source_env.db_host.clone(),
            data_source_env.db_port.clone(),
            data_source_env.db_name.clone(),
            data_source_env.service_account_name.clone(),
            data_source_env.service_account_password.clone(),
        );

        let execution =
            provider_postgres_execution_client(&connection, &security).map_err(AppError::from)?;

        let cache = Arc::new(Self {
            execution,
            principals: RwLock::new(HashMap::new()),
        });

        if let Err(e) = cache.refresh().await {
            warn!(
                error = %e,
                "authz cache: initial refresh failed; starting with no grants (every check denies until the next refresh succeeds)"
            );
        }

        let refresher = cache.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(REFRESH_INTERVAL);
            ticker.tick().await; // the first tick fires immediately; the init() call above already refreshed once
            loop {
                ticker.tick().await;
                if let Err(e) = refresher.refresh().await {
                    error!(error = %e, "authz cache: periodic refresh failed; keeping the previous grants");
                }
            }
        });

        Ok(cache)
    }

    async fn refresh(&self) -> Result<(), AppError> {
        let client = self
            .execution
            .client()
            .await
            .map_err(|e| AppError::InternalServerError(anyhow::anyhow!(e)))?;

        let rows = client
            .query(
                "select au.tenant_id, au.user_name, au.id::text as user_id, \
                        p.code, p.resource, p.action, rp.scope, p.owner_field \
                 from tax_routing.app_users au \
                 join tax_routing.user_roles ur on ur.user_id = au.id \
                 join tax_routing.roles r on r.id = ur.role_id and r.is_active \
                 join tax_routing.role_permissions rp on rp.role_id = r.id \
                 join tax_routing.permissions p on p.id = rp.permission_id \
                 where au.is_active",
                &[],
            )
            .await
            .map_err(|e| AppError::InternalServerError(anyhow::anyhow!(e)))?;

        let mut next: HashMap<CacheKey, Principal> = HashMap::new();
        for row in &rows {
            let tenant_id: String = row.get("tenant_id");
            let user_name: String = row.get("user_name");
            let user_id: String = row.get("user_id");
            let grant = Grant {
                code: row.get("code"),
                resource: row.get("resource"),
                action: row.get("action"),
                scope: row.get("scope"),
                owner_field: row.get("owner_field"),
            };
            next.entry((tenant_id, user_name))
                .or_insert_with(|| Principal {
                    user_id: Some(user_id),
                    grants: Vec::new(),
                })
                .grants
                .push(grant);
        }

        let principal_count = next.len();
        let grant_count = rows.len();
        *self
            .principals
            .write()
            .map_err(|_| AppError::InternalServerError(anyhow::anyhow!("authz cache lock poisoned")))? = next;
        debug!(principal_count, grant_count, "authz cache refreshed");
        Ok(())
    }

    /// Resolve a caller's grants from the in-memory cache. Never touches the database: this is
    /// what `evaluate_user_access` and the custom methods call, and both need it to be
    /// synchronous-fast. An unknown, inactive, or role-less user resolves to no grants, which
    /// every consumer treats as a deny.
    pub fn resolve(&self, tenant_id: &str, user_name: &str) -> Principal {
        self.principals
            .read()
            .map(|map| {
                map.get(&(tenant_id.to_string(), user_name.to_string()))
                    .cloned()
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    }

    /// True as soon as the first successful refresh has populated the cache. Used to log a
    /// clearer warning than "denied" when the RBAC tables have never loaded.
    pub fn is_populated(&self) -> bool {
        self.principals.read().map(|m| !m.is_empty()).unwrap_or(false)
    }
}

fn resolve_data_source_env(
    sources: &HashMap<String, DataSource>,
    data_source_name: &str,
) -> Result<Arc<DataSourceEnvironment>, AppError> {
    let data_source = sources
        .get(data_source_name)
        .ok_or_else(|| {
            AppError::InternalServerError(anyhow::anyhow!(
                "authz cache: data source `{data_source_name}` is not configured"
            ))
        })?
        .to_owned();
    let environment = data_source
        .environments
        .first()
        .ok_or_else(|| {
            AppError::InternalServerError(anyhow::anyhow!(
                "authz cache: data source `{data_source_name}` has no environment configured"
            ))
        })?
        .to_owned();
    Ok(Arc::new(environment))
}

/// Require that a resolved principal holds `permission_code`. Used by custom methods for
/// capabilities Rego cannot express as a row filter (cancel, reassign, RBAC administration).
pub fn require(principal: &Principal, permission_code: &str) -> Result<(), AppError> {
    if principal.has_permission(permission_code) {
        Ok(())
    } else {
        info!(permission = permission_code, "authz: denied (permission not held)");
        Err(AppError::AccessDenied)
    }
}
