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

pub mod authz;
pub mod exception_task_actions;
pub mod form_engine;
pub mod routing_record_actions;
