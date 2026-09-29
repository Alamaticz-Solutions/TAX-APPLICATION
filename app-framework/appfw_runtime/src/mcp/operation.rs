//! MCP compatibility aliases for the transport-neutral runtime operation API.
//!
//! Product operation dispatch is not owned by MCP. These aliases keep existing
//! MCP adapter code readable while the generated dispatcher moves to
//! `appfw_runtime::operation`.

pub use crate::operation::{
    arg, handler_error, redact_entity_payload, resolve_operation, resolve_operation_app_error,
    to_json, RuntimeOperation as McpOperation, RuntimeOperationArg as McpOperationArg,
    RuntimeOperationCatalog as McpOperationCatalog,
    RuntimeOperationDispatcher as McpOperationDispatcher,
    RuntimeOperationRequest as CallOperationRequest,
    RuntimeOperationResolutionError as McpOperationResolutionError,
};
