use std::collections::BTreeMap;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

/// Bolt access mode. The graph-read provider only ever issues `Read` (cluster
/// routing can send those to a read replica, reinforcing the read-only
/// credential posture). The governed write provider issues `Write`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Neo4jAccessMode {
    Read,
    Write,
}

/// A fully-resolved, parameterized graph read ready for the driver. The cypher
/// is the vetted text from a registered named query and every value is bound;
/// no user input is ever concatenated into the statement.
#[derive(Clone, Debug, PartialEq)]
pub struct Neo4jExecutionRequest {
    pub database: String,
    pub operation: String,
    pub cypher: String,
    pub parameters: BTreeMap<String, Value>,
    pub access_mode: Neo4jAccessMode,
    pub max_results: u32,
    pub timeout_ms: u64,
}

/// Result of a successful graph read.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Neo4jExecutionOutcome {
    pub rows: Vec<Value>,
    /// When the graph is a projection/read model, the timestamp the projection
    /// was last refreshed. Surfaced so projection freshness is observable.
    pub graph_freshness: Option<DateTime<Utc>>,
}

impl Neo4jExecutionOutcome {
    pub fn new(rows: Vec<Value>) -> Self {
        Self {
            rows,
            graph_freshness: None,
        }
    }

    pub fn with_freshness(mut self, freshness: DateTime<Utc>) -> Self {
        self.graph_freshness = Some(freshness);
        self
    }
}

/// Mutation counters returned by a governed write. These are non-classified
/// aggregate counts and are safe to log and audit.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct Neo4jWriteStats {
    pub nodes_created: u64,
    pub nodes_deleted: u64,
    pub relationships_created: u64,
    pub relationships_deleted: u64,
    pub properties_set: u64,
}

impl Neo4jWriteStats {
    /// Total graph elements affected (nodes + relationships created/deleted).
    pub fn affected(&self) -> u64 {
        self.nodes_created
            + self.nodes_deleted
            + self.relationships_created
            + self.relationships_deleted
    }
}

/// Result of a successful governed graph write.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Neo4jWriteOutcome {
    pub rows: Vec<Value>,
    pub stats: Neo4jWriteStats,
    pub graph_freshness: Option<DateTime<Utc>>,
}

impl Neo4jWriteOutcome {
    pub fn new(rows: Vec<Value>, stats: Neo4jWriteStats) -> Self {
        Self {
            rows,
            stats,
            graph_freshness: None,
        }
    }

    pub fn with_freshness(mut self, freshness: DateTime<Utc>) -> Self {
        self.graph_freshness = Some(freshness);
        self
    }
}

#[derive(Debug, Error)]
pub enum Neo4jExecutionError {
    #[error("neo4j connectivity error: {0}")]
    Connectivity(String),
    #[error("neo4j query error: {0}")]
    Query(String),
}

/// Driver seam for graph reads and governed writes. The provider depends only on
/// this trait, so the guardrail/audit/limit logic is exercised with an in-memory
/// executor in unit tests and a Bolt-driver executor in a live deployment without
/// changing the provider.
#[async_trait]
pub trait Neo4jExecutor: Send + Sync {
    /// Readiness/health probe: confirm the data source is reachable and the
    /// credentials authenticate.
    async fn verify_connectivity(&self) -> Result<(), Neo4jExecutionError>;

    /// Run a single read-only, parameterized query.
    async fn run_read_query(
        &self,
        request: Neo4jExecutionRequest,
    ) -> Result<Neo4jExecutionOutcome, Neo4jExecutionError>;

    /// Run a single governed, parameterized write inside a transaction and
    /// return the affected-element counters.
    async fn run_write_query(
        &self,
        request: Neo4jExecutionRequest,
    ) -> Result<Neo4jWriteOutcome, Neo4jExecutionError>;
}

/// In-memory executor that serves canned rows per operation. Used by unit and
/// contract tests (and local fixtures) to exercise the full provider path
/// without a live Neo4j instance.
#[derive(Default)]
pub struct FixtureGraphExecutor {
    responses: BTreeMap<String, Vec<Value>>,
    write_responses: BTreeMap<String, (Vec<Value>, Neo4jWriteStats)>,
    freshness: Option<DateTime<Utc>>,
    delay: Duration,
    connectivity_error: Option<String>,
    query_error: Option<String>,
}

impl FixtureGraphExecutor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register the rows returned for `operation`.
    pub fn with_rows(mut self, operation: impl Into<String>, rows: Vec<Value>) -> Self {
        self.responses.insert(operation.into(), rows);
        self
    }

    /// Register the rows + mutation stats returned for a write `operation`.
    pub fn with_write_result(
        mut self,
        operation: impl Into<String>,
        rows: Vec<Value>,
        stats: Neo4jWriteStats,
    ) -> Self {
        self.write_responses.insert(operation.into(), (rows, stats));
        self
    }

    pub fn with_freshness(mut self, freshness: DateTime<Utc>) -> Self {
        self.freshness = Some(freshness);
        self
    }

    /// Simulate query latency so timeout handling can be exercised. Pair with
    /// `tokio::test(start_paused = true)` for deterministic timing.
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    pub fn failing_connectivity(mut self, message: impl Into<String>) -> Self {
        self.connectivity_error = Some(message.into());
        self
    }

    pub fn failing_query(mut self, message: impl Into<String>) -> Self {
        self.query_error = Some(message.into());
        self
    }
}

#[async_trait]
impl Neo4jExecutor for FixtureGraphExecutor {
    async fn verify_connectivity(&self) -> Result<(), Neo4jExecutionError> {
        match &self.connectivity_error {
            Some(message) => Err(Neo4jExecutionError::Connectivity(message.clone())),
            None => Ok(()),
        }
    }

    async fn run_read_query(
        &self,
        request: Neo4jExecutionRequest,
    ) -> Result<Neo4jExecutionOutcome, Neo4jExecutionError> {
        if !self.delay.is_zero() {
            tokio::time::sleep(self.delay).await;
        }
        if let Some(message) = &self.query_error {
            return Err(Neo4jExecutionError::Query(message.clone()));
        }
        let rows = self
            .responses
            .get(&request.operation)
            .cloned()
            .unwrap_or_default();
        let mut outcome = Neo4jExecutionOutcome::new(rows);
        outcome.graph_freshness = self.freshness;
        Ok(outcome)
    }

    async fn run_write_query(
        &self,
        request: Neo4jExecutionRequest,
    ) -> Result<Neo4jWriteOutcome, Neo4jExecutionError> {
        if !self.delay.is_zero() {
            tokio::time::sleep(self.delay).await;
        }
        if let Some(message) = &self.query_error {
            return Err(Neo4jExecutionError::Query(message.clone()));
        }
        let (rows, stats) = self
            .write_responses
            .get(&request.operation)
            .cloned()
            .unwrap_or_default();
        let mut outcome = Neo4jWriteOutcome::new(rows, stats);
        outcome.graph_freshness = self.freshness;
        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[tokio::test]
    async fn fixture_executor_returns_registered_rows() {
        let executor =
            FixtureGraphExecutor::new().with_rows("op", vec![json!({"id": 1}), json!({"id": 2})]);
        let request = Neo4jExecutionRequest {
            database: "neo4j".to_string(),
            operation: "op".to_string(),
            cypher: "MATCH (a) RETURN a".to_string(),
            parameters: BTreeMap::new(),
            access_mode: Neo4jAccessMode::Read,
            max_results: 10,
            timeout_ms: 1_000,
        };
        let outcome = executor.run_read_query(request).await.expect("rows");
        assert_eq!(outcome.rows.len(), 2);
    }

    #[tokio::test]
    async fn fixture_executor_can_fail_connectivity() {
        let executor = FixtureGraphExecutor::new().failing_connectivity("down");
        assert!(matches!(
            executor.verify_connectivity().await,
            Err(Neo4jExecutionError::Connectivity(_))
        ));
    }

    #[tokio::test]
    async fn fixture_executor_returns_write_stats() {
        let stats = Neo4jWriteStats {
            relationships_created: 1,
            ..Default::default()
        };
        let executor = FixtureGraphExecutor::new().with_write_result(
            "link",
            vec![json!({"linked": true})],
            stats,
        );
        let request = Neo4jExecutionRequest {
            database: "neo4j".to_string(),
            operation: "link".to_string(),
            cypher: "MATCH (a),(b) MERGE (a)-[r:REFERRED]->(b) RETURN r".to_string(),
            parameters: BTreeMap::new(),
            access_mode: Neo4jAccessMode::Write,
            max_results: 10,
            timeout_ms: 1_000,
        };
        let outcome = executor.run_write_query(request).await.expect("write");
        assert_eq!(outcome.rows.len(), 1);
        assert_eq!(outcome.stats.relationships_created, 1);
        assert_eq!(outcome.stats.affected(), 1);
    }

    #[tokio::test]
    async fn fixture_write_can_fail_query() {
        let executor = FixtureGraphExecutor::new().failing_query("boom");
        let request = Neo4jExecutionRequest {
            database: "neo4j".to_string(),
            operation: "link".to_string(),
            cypher: "MATCH (a),(b) MERGE (a)-[r:REFERRED]->(b) RETURN r".to_string(),
            parameters: BTreeMap::new(),
            access_mode: Neo4jAccessMode::Write,
            max_results: 10,
            timeout_ms: 1_000,
        };
        assert!(matches!(
            executor.run_write_query(request).await,
            Err(Neo4jExecutionError::Query(_))
        ));
    }
}
