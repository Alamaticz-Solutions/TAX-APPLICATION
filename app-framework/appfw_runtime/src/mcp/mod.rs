//! Model Context Protocol contracts shared by generated products.
//!
//! Runtime-owned MCP code in this module must stay schema-neutral. Product
//! backends provide generated operation dispatch and product data access.

pub mod access;
pub mod builtin;
pub mod catalog;
#[cfg(feature = "http")]
pub mod http;
pub mod operation;
pub mod protocol;
pub mod server;
pub mod service;
pub mod support_tool;
pub mod tool_call;
