//! Shared Microsoft Graph plumbing: the pinned base URL, the app-only
//! client-credentials token source, request-path escaping and response
//! classification / redaction.
//!
//! This module deliberately contains NO feature code (no Teams meetings, no
//! directory reads, no writes). It exists so each Graph-backed provider
//! (`services::sharepoint` today) re-exports generic escaping and
//! classification from one place instead of copying them, while keeping its
//! own registry, write gate and ledger. In Project Governance the same module
//! also hosts the Teams-meeting client; that code is intentionally not ported
//! here because Tax Document Routing has no meeting feature.
//!
//!   identity   the pinned Graph API base URL
//!   auth       env-var contract + client-credentials token acquisition
//!   request    safe request-plan builder and path/OData escaping
//!   response   error classification, Retry-After parsing, redaction, caps

// Only `services::sharepoint` consumes this, and it has no caller yet.
#![allow(dead_code)]

pub mod auth;
pub mod identity;
pub mod request;
pub mod response;
