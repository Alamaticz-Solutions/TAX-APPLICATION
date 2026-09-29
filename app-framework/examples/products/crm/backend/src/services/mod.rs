//! Product-owned service layer.
//!
//! Add durable business workflows here and call them from product-owned
//! handlers. Keep framework/runtime access behind `crate::product_api`.

pub(crate) mod account_health;
