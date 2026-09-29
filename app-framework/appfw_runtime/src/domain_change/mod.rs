//! Provider-neutral, crate-internal domain-change and selective-refresh contracts.
//!
//! This dormant foundation intentionally contains no broker, database, HTTP,
//! product, provider SDK, durability, authentication, or release integration.

#![cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "dormant internal domain-change foundation; remove with first reviewed non-test consumer or public-adapter reslice"
    )
)]

mod coalesce;
mod model;
mod store;

#[cfg(test)]
mod tests;
