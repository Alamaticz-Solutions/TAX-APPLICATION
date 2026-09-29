//! Product-owned service layer.
//!
//! Add durable business workflows here and call them from product-owned
//! handlers. Keep framework/runtime access behind `crate::product_api`.

// Dead-code analysis is silenced for the IX services in the plain binary
// build: constructing and mounting the IX runtime (and therefore calling
// this adapter) is reserved for the B_HOST host-wiring leaf, so until then
// the surface is exercised exclusively by the adapter-level proof tests.
#[allow(dead_code)]
pub mod ix;
