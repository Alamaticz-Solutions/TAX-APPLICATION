//! Neo4j graph provider for App Framework.
//!
//! This crate implements the [`RuntimeGraphProvider`] contract: a registry of
//! vetted, parameterized named read queries (guarded by policy access, tenant
//! scoping, read-only validation, traversal-depth and result/timeout caps, a
//! redacted audit trail, and observable projection freshness) plus an opt-in
//! registry of governed write mutations (relationship-scoped, tenant-bound).
//! Graph access is read-first; writes are gated at the operation/exposure layer
//! rather than by a separate trait. The driver is injected through the single
//! [`Neo4jExecutor`] seam, so the full guardrail path is exercised without a
//! live database; the `bolt` feature provides a live `neo4rs`-backed executor.

pub mod audit;
#[cfg(feature = "bolt")]
pub mod bolt_executor;
pub mod connection;
pub mod execution;
pub mod limits;
pub mod provider;
pub mod query;
pub mod shared;

pub use audit::{
    CapturingGraphAuditSink, Neo4jGraphAuditSink, Neo4jGraphMutationAudit, Neo4jGraphReadAudit,
    Neo4jGraphReadOutcome, NoopGraphAuditSink, TracingGraphAuditSink,
};
#[cfg(feature = "bolt")]
pub use bolt_executor::Neo4jBoltExecutor;
pub use connection::{validate_neo4j_connection_security, Neo4jConnectionConfig};
pub use execution::{
    FixtureGraphExecutor, Neo4jAccessMode, Neo4jExecutionError, Neo4jExecutionOutcome,
    Neo4jExecutionRequest, Neo4jExecutor, Neo4jWriteOutcome, Neo4jWriteStats,
};
pub use limits::{Neo4jEffectiveLimits, Neo4jProviderLimits};
pub use provider::Neo4jGraphProvider;
pub use query::{
    cypher_references_parameter, validate_bounded_traversal, validate_governed_write_cypher,
    validate_read_only_cypher, validate_traversal_depth, Neo4jGraphMutation, Neo4jGraphQuery,
    Neo4jGraphQueryError, Neo4jGraphQueryLimits, Neo4jParameter, DEFAULT_TENANT_PARAMETER,
};
