//! SharePoint document-storage provider - a separate module from
//! `services::graph` by deliberate choice. The wire target is still Microsoft
//! Graph (same tenant, same two app registrations), but this module owns its
//! own registry, write gate, ledger correlation and honesty tiers, so a change
//! to any other Graph-backed feature can never affect document storage and
//! vice versa.
//!
//! Module shape mirrors `appfw_provider_salesforce` (file layout) and
//! `appfw_provider_servicenow` (evidence-gated registry), and the network-free
//! request plans are built by the framework crate `appfw-provider-sharepoint`
//! so the allow-list, path rules and the 2 MB limit have exactly one source:
//!
//!   identity          the pinned Graph API base URL
//!   auth              SharePoint's own env-var contract (site id, drive id,
//!                     auth mode); token acquisition is NOT duplicated -
//!                     `services::graph::auth` supplies it
//!   registry          allow-listed named READ operations over one drive
//!   request/response  re-exported from `services::graph` (generic escaping,
//!                     classification and redaction - not SharePoint-specific)
//!   client            executor: named READ operation + bound params ->
//!                     classified result
//!   writes            this module's own G1-style governed-write gate - NOT
//!                     shared with any other module
//!   vendor_contract   honest per-operation status tiers
//!
//! Nothing here is `live_certified`. No real site id / drive id / role grant
//! exists yet; see `docs/architecture/sharepoint-integration-plan.md`.

// No handler calls this module yet (custom methods are not built); everything
// is exercised by unit tests only. Remove once the document service lands.
#![allow(dead_code, unused_imports)]

pub mod auth;
pub mod client;
pub mod identity;
pub mod registry;
pub mod vendor_contract;
pub mod writes;

mod request {
    //! Re-exported from `services::graph::request`: path/query escaping is
    //! generic to any Graph call. No SharePoint-specific escaping lives here.
    pub use crate::services::graph::request::*;
}

mod response {
    //! Re-exported from `services::graph::response`: HTTP status
    //! classification and redaction are Graph-wide.
    pub use crate::services::graph::response::*;
}

pub use client::SharePointClient;
pub use registry::ReadOperation;
pub use writes::{WriteContext, WriteOperation};
