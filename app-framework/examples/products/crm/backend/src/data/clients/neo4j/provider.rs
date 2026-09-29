//! Live graph provider wiring for the CRM `neo4j_graph` data source.
//!
//! Lazily builds and caches a single `Neo4jGraphProvider` (over a live
//! `Neo4jBoltExecutor`) from `AppConfig`, registers the product-owned named read
//! queries and governed write mutations, evaluates policy access, and exposes
//! `read_account_graph` / `write_account_link` helpers for the Account
//! custom-method handlers.

use std::collections::BTreeMap;
use std::sync::Arc;

use appfw_provider_neo4j::{
    validate_neo4j_connection_security, Neo4jBoltExecutor, Neo4jConnectionConfig,
    Neo4jGraphProvider,
};
use appfw_runtime::graph_provider::{RuntimeGraphNamedQuery, RuntimeGraphProvider};
use appfw_runtime::{AccessAction, RuntimeGraphQueryLimits};
use serde_json::{json, Value};
use tokio::sync::OnceCell;

use crate::config::app_config::AppConfig;
use crate::product_api::{EntityType, UserAuth};
use crate::routes::app_error::AppError;

use super::queries;

/// The CRM graph data source name (declared in `.appfw/model/data_sources`).
const GRAPH_DATA_SOURCE: &str = "neo4j_graph";

type GraphProvider = Neo4jGraphProvider<Neo4jBoltExecutor>;

/// Process-wide, lazily-initialized graph provider (a single Bolt connection
/// pool serving both reads and governed writes). Rebuilt only if initialization
/// previously failed.
static PROVIDER: OnceCell<GraphProvider> = OnceCell::const_new();

async fn provider(app_config: &Arc<AppConfig>) -> Result<&'static GraphProvider, AppError> {
    PROVIDER
        .get_or_try_init(|| build_provider(app_config.clone()))
        .await
}

async fn build_provider(app_config: Arc<AppConfig>) -> Result<GraphProvider, AppError> {
    let env = app_config.get_data_source_env(&GRAPH_DATA_SOURCE.to_string())?;
    let security = validate_neo4j_connection_security(
        &env.name,
        &env.security_profile,
        &env.tls_mode,
        &env.db_host,
    )?;
    let config = Neo4jConnectionConfig::new(
        env.db_host.clone(),
        env.db_port.clone(),
        env.db_name.clone(),
        env.service_account_name.clone(),
        env.service_account_password.clone(),
    );

    Ok(Neo4jGraphProvider::new(
        GRAPH_DATA_SOURCE,
        env.db_name.clone(),
        Neo4jBoltExecutor::connect(&config, &security).await?,
    )
    .register_queries(queries::read_queries())
    .register_mutations(queries::write_mutations()))
}

/// Execute a registered graph read named query for `account` (a record locator),
/// returning `{ rows, metadata }`. Tenant scoping is bound server-side from the
/// authenticated user; policy access is evaluated against the Account entity.
pub(crate) async fn read_account_graph(
    app_config: &Arc<AppConfig>,
    entity_type: &Arc<EntityType>,
    user: &UserAuth,
    operation: &str,
    account: String,
) -> Result<Value, AppError> {
    let provider = provider(app_config).await?;
    let access = app_config.evaluate_user_access(entity_type.clone(), AccessAction::Read, user)?;
    let query = named_query(
        operation,
        BTreeMap::from([("account".to_string(), json!(account))]),
    );
    let result = provider.execute_named_query(query, user, &access).await?;
    Ok(json!({ "rows": result.rows, "metadata": result.metadata }))
}

/// Execute a registered governed graph write (link/unlink) between two account
/// record locators, returning `{ rows, stats, metadata }`.
pub(crate) async fn write_account_link(
    app_config: &Arc<AppConfig>,
    entity_type: &Arc<EntityType>,
    user: &UserAuth,
    operation: &str,
    from_account: String,
    to_account: String,
) -> Result<Value, AppError> {
    let provider = provider(app_config).await?;
    let access =
        app_config.evaluate_user_access(entity_type.clone(), AccessAction::Update, user)?;
    let mutation = named_query(
        operation,
        BTreeMap::from([
            ("from_account".to_string(), json!(from_account)),
            ("to_account".to_string(), json!(to_account)),
        ]),
    );
    let result = provider
        .execute_named_mutation(mutation, user, &access)
        .await?;
    Ok(json!({ "rows": result.rows, "stats": result.stats, "metadata": result.metadata }))
}

fn named_query(operation: &str, parameters: BTreeMap<String, Value>) -> RuntimeGraphNamedQuery {
    RuntimeGraphNamedQuery {
        name: operation.to_string(),
        query_text: String::new(),
        parameters,
        // The registered definition's limits are authoritative; these are ignored.
        limits: RuntimeGraphQueryLimits::default(),
    }
}
