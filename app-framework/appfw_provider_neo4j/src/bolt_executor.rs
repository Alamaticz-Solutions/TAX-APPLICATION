//! Live Neo4j executor over the Bolt protocol, backed by the `neo4rs` driver.
//!
//! Enabled by the `bolt` cargo feature. Implements the unified [`Neo4jExecutor`]
//! seam (read + governed write) so the provider guardrails run against a real
//! database. Parameters are bound
//! through `neo4rs`'s `BoltType` conversion (never string-concatenated) and rows
//! are returned as `serde_json::Value`.
//!
//! Note: `neo4rs` 0.9 does not surface Bolt result-summary counters, so live
//! mutation stats (`Neo4jWriteStats`) are reported as the affected RETURN-row
//! count rather than precise nodes/relationships-created counters; precise
//! counters await a driver summary API. Single-statement governed writes run in
//! auto-commit, so a dropped (timed-out) future rolls back on the server.

use appfw_runtime::connection_security::ConnectionSecurity;
use appfw_runtime::RuntimeError;
use async_trait::async_trait;
use neo4rs::{query, BoltType, Graph};

use crate::connection::Neo4jConnectionConfig;
use crate::execution::{
    Neo4jExecutionError, Neo4jExecutionOutcome, Neo4jExecutionRequest, Neo4jExecutor,
    Neo4jWriteOutcome, Neo4jWriteStats,
};

/// Live Neo4j executor over the Bolt protocol.
pub struct Neo4jBoltExecutor {
    graph: Graph,
    database: String,
}

impl Neo4jBoltExecutor {
    pub fn database(&self) -> &str {
        &self.database
    }

    /// Connect to Neo4j using the validated connection security policy to pick
    /// the Bolt scheme (TLS for managed environments, plaintext for local_dev).
    pub async fn connect(
        config: &Neo4jConnectionConfig,
        security: &ConnectionSecurity,
    ) -> Result<Self, RuntimeError> {
        let uri = secure_bolt_uri(config, security);
        let user = config.user.clone().unwrap_or_else(|| "neo4j".to_string());
        let password = config.password.clone().unwrap_or_default();
        let graph = Graph::new(&uri, &user, &password).map_err(|error| {
            RuntimeError::DataAccess(format!("neo4j connection failed: {error}"))
        })?;
        Ok(Self {
            graph,
            database: config.database.clone(),
        })
    }
}

#[async_trait]
impl Neo4jExecutor for Neo4jBoltExecutor {
    async fn verify_connectivity(&self) -> Result<(), Neo4jExecutionError> {
        let mut stream = self
            .graph
            .execute(query("RETURN 1 AS ok"))
            .await
            .map_err(connectivity_error)?;
        let _ = stream.next().await.map_err(connectivity_error)?;
        Ok(())
    }

    async fn run_read_query(
        &self,
        request: Neo4jExecutionRequest,
    ) -> Result<Neo4jExecutionOutcome, Neo4jExecutionError> {
        let rows = self.collect_rows(&request).await?;
        Ok(Neo4jExecutionOutcome::new(rows))
    }

    async fn run_write_query(
        &self,
        request: Neo4jExecutionRequest,
    ) -> Result<Neo4jWriteOutcome, Neo4jExecutionError> {
        // A single governed write runs in auto-commit; collecting the RETURN
        // rows drives the statement to completion.
        let rows = self.collect_rows(&request).await?;
        // neo4rs 0.9 does not expose Bolt summary counters; report the affected
        // RETURN-row count as a best-effort relationships-affected signal.
        let stats = Neo4jWriteStats {
            relationships_created: rows.len() as u64,
            ..Default::default()
        };
        Ok(Neo4jWriteOutcome::new(rows, stats))
    }
}

impl Neo4jBoltExecutor {
    async fn collect_rows(
        &self,
        request: &Neo4jExecutionRequest,
    ) -> Result<Vec<serde_json::Value>, Neo4jExecutionError> {
        let bound = build_query(request)?;
        let mut stream = self.graph.execute(bound).await.map_err(query_error)?;
        let mut rows = Vec::new();
        while let Some(row) = stream.next().await.map_err(query_error)? {
            let value = row.to::<serde_json::Value>().map_err(|error| {
                Neo4jExecutionError::Query(format!("row decode failed: {error}"))
            })?;
            rows.push(value);
            if rows.len() >= request.max_results as usize {
                break;
            }
        }
        Ok(rows)
    }
}

/// Build a parameterized `neo4rs` query, binding every value through `BoltType`
/// (never string interpolation).
fn build_query(request: &Neo4jExecutionRequest) -> Result<neo4rs::Query, Neo4jExecutionError> {
    let mut bound = query(&request.cypher);
    for (name, value) in &request.parameters {
        let bolt = BoltType::try_from(value.clone()).map_err(|error| {
            Neo4jExecutionError::Query(format!("invalid parameter `{name}`: {error}"))
        })?;
        bound = bound.param(name, bolt);
    }
    Ok(bound)
}

/// Compose the Bolt URI, upgrading to a TLS scheme when the security policy
/// requires it.
fn secure_bolt_uri(config: &Neo4jConnectionConfig, security: &ConnectionSecurity) -> String {
    let base = config.bolt_uri();
    if !security.requires_tls() {
        return base;
    }
    if let Some(rest) = base.strip_prefix("bolt://") {
        return format!("bolt+s://{rest}");
    }
    if let Some(rest) = base.strip_prefix("neo4j://") {
        return format!("neo4j+s://{rest}");
    }
    base
}

fn connectivity_error(error: neo4rs::Error) -> Neo4jExecutionError {
    Neo4jExecutionError::Connectivity(error.to_string())
}

fn query_error(error: neo4rs::Error) -> Neo4jExecutionError {
    Neo4jExecutionError::Query(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use appfw_runtime::connection_security::{self, Provider};

    #[test]
    fn secure_uri_upgrades_scheme_when_tls_required() {
        let config = Neo4jConnectionConfig::new("graph.internal", "7687", "neo4j", None, None);
        let managed = connection_security::validate(
            Provider::Neo4j,
            "prod",
            "managed",
            "require",
            "graph.internal",
        )
        .expect("managed security");
        assert_eq!(
            secure_bolt_uri(&config, &managed),
            "bolt+s://graph.internal:7687"
        );

        let local = connection_security::validate(
            Provider::Neo4j,
            "compose",
            "local_dev",
            "disabled",
            "neo4j",
        )
        .expect("local security");
        assert_eq!(
            secure_bolt_uri(&config, &local),
            "bolt://graph.internal:7687"
        );
    }

    #[test]
    fn build_query_binds_parameters() {
        let request = Neo4jExecutionRequest {
            database: "neo4j".to_string(),
            operation: "op".to_string(),
            cypher: "MATCH (a {tenant_id: $tenant_id}) RETURN a".to_string(),
            parameters: std::collections::BTreeMap::from([
                ("tenant_id".to_string(), serde_json::json!("180000")),
                ("count".to_string(), serde_json::json!(3)),
            ]),
            access_mode: crate::execution::Neo4jAccessMode::Read,
            max_results: 10,
            timeout_ms: 1_000,
        };
        let bound = build_query(&request).expect("query builds");
        assert!(bound.has_param_key("tenant_id"));
        assert!(bound.has_param_key("count"));
    }
}
