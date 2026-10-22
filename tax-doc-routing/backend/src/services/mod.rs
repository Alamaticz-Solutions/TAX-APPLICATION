//! Product-owned service layer.
//!
//! All Tax Document Routing business logic lives here and is called from the hand-owned
//! handlers in `handlers/tax_routing/<entity>.rs`. Keep framework/runtime access behind
//! `crate::product_api`.
//!
//! Planned modules (see `docs/architecture/tax-document-routing-design.md`):
//! - `routing_engine`   - status transitions, job queue, exception handling
//! - `document_store`   - `DocumentStore` boundary (in-memory now; SharePoint later)
//! - `pdf_protection`   - in-memory PDF encryption, fail-closed
//! - `notifications`    - exception and final-confirmation email
//!
//! Built:
//! - `graph`            - shared Microsoft Graph plumbing (auth, escaping, classification)
//! - `sharepoint`       - SharePoint document-storage provider (separate from `graph`;
//!                        see `docs/architecture/sharepoint-integration-plan.md`)

pub mod authz;
pub mod bulk_requests;
pub mod case_numbers;
pub mod document_routing;
pub mod document_store;
pub mod pdf_protection;
pub mod exception_task_actions;
pub mod form_engine;
pub mod graph;
pub mod routing_record_actions;
pub mod sharepoint;
