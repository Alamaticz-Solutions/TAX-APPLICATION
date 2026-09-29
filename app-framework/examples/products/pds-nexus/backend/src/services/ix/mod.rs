//! Product-owned Nexus Intelligent Experience (IX) services.
//!
//! B_PRODUCT scope (chat-independent, decision-0002 / packet 000004 Option A):
//! this module implements the complete un-gated Framework IX persistence and
//! policy seams — [`appfw_runtime::ix::IxRunRepository`],
//! [`appfw_runtime::ix::IxContextPolicy`], and
//! [`appfw_runtime::ix::IxAuditSink`] — plus providerless context-to-recipe
//! orchestration logic as plain service functions.
//!
//! Authority boundaries (unchanged by this module):
//! - The Framework IX lifecycle is the only author of canonical run facts:
//!   snapshots, commit envelopes, cursors, cancellation fences, replay
//!   checkpoints, and audit facts. This module persists and projects those
//!   values; it never mints a second lifecycle, verifier, or context
//!   authority.
//! - Stored-run JSON decoding performs the Framework's own shape, bounds,
//!   replay, and cancellation-structure checks
//!   ([`appfw_runtime::ix::IxStoredRun::from_json_slice`]); it never implies
//!   HMAC seal acceptance. Seal authentication is performed exclusively by the
//!   chat-gated `IxRuntimeService` and is a named B_HOST claim
//!   (DEFERRED_TO_B_HOST).
//! - Trait-dispatch integration through runtime-constructed
//!   `IxRunCreateTransaction`/`IxRunCommitTransaction` values is likewise a
//!   named B_HOST claim: those transactions are only constructible by the
//!   chat-gated runtime. The trait implementations here delegate 1:1 to the
//!   public persistence operations proven at adapter level against real
//!   PostgreSQL.

pub mod audit;
pub mod context_policy;
pub mod orchestration;
pub mod repository;

pub use audit::NexusIxAuditSink;
pub use context_policy::NexusIxContextPolicy;
pub use repository::NexusIxRunRepository;

#[cfg(test)]
mod proof_fixtures;

#[cfg(test)]
mod pg_proof;
