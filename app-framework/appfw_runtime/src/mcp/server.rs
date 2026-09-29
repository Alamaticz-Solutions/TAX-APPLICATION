use async_trait::async_trait;
use serde_json::Value;
use tracing::debug;

use crate::{extension::UserAuth, observability::RequestContext};

use super::{
    access::McpError,
    protocol::{
        error, success, JsonRpcRequest, INVALID_REQUEST, JSONRPC_VERSION, METHOD_NOT_FOUND,
        PARSE_ERROR,
    },
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum McpJsonRpcStatus {
    Ok,
    Accepted,
    BadRequest,
}

#[derive(Debug, Clone, PartialEq)]
pub struct McpJsonRpcResponse {
    pub status: McpJsonRpcStatus,
    pub body: Option<Value>,
}

impl McpJsonRpcResponse {
    pub fn ok(body: Value) -> Self {
        Self {
            status: McpJsonRpcStatus::Ok,
            body: Some(body),
        }
    }

    pub fn accepted() -> Self {
        Self {
            status: McpJsonRpcStatus::Accepted,
            body: None,
        }
    }

    pub fn bad_request(body: Value) -> Self {
        Self {
            status: McpJsonRpcStatus::BadRequest,
            body: Some(body),
        }
    }
}

#[async_trait]
pub trait McpMethodHandler: Sync {
    async fn handle_method(
        &self,
        method: &str,
        params: Option<Value>,
        user: &UserAuth,
        request_context: &RequestContext,
    ) -> Result<Value, McpError>;

    async fn handle_notification(&self, method: &str) {
        debug!(method, "received MCP notification");
    }
}

pub async fn handle_payload<H>(
    handler: &H,
    user: &UserAuth,
    request_context: &RequestContext,
    payload: Value,
    max_batch_items: usize,
) -> McpJsonRpcResponse
where
    H: McpMethodHandler,
{
    match payload {
        Value::Array(items) => {
            handle_batch(handler, user, request_context, items, max_batch_items).await
        }
        Value::Object(_) => match handle_jsonrpc(handler, user, request_context, payload).await {
            Some(response) => McpJsonRpcResponse::ok(response),
            None => McpJsonRpcResponse::accepted(),
        },
        _ => McpJsonRpcResponse::bad_request(error(
            None,
            PARSE_ERROR,
            "MCP request body must be a JSON-RPC object or batch",
            request_context,
        )),
    }
}

async fn handle_batch<H>(
    handler: &H,
    user: &UserAuth,
    request_context: &RequestContext,
    items: Vec<Value>,
    max_batch_items: usize,
) -> McpJsonRpcResponse
where
    H: McpMethodHandler,
{
    if items.is_empty() {
        return McpJsonRpcResponse::bad_request(error(
            None,
            INVALID_REQUEST,
            "JSON-RPC batch must not be empty",
            request_context,
        ));
    }
    if items.len() > max_batch_items {
        return McpJsonRpcResponse::bad_request(error(
            None,
            INVALID_REQUEST,
            format!(
                "JSON-RPC batch contains {} items; APP_MCP_MAX_BATCH_ITEMS is {}",
                items.len(),
                max_batch_items
            ),
            request_context,
        ));
    }

    let mut responses = Vec::new();
    for item in items {
        if let Some(response) = handle_jsonrpc(handler, user, request_context, item).await {
            responses.push(response);
        }
    }

    if responses.is_empty() {
        McpJsonRpcResponse::accepted()
    } else {
        McpJsonRpcResponse::ok(Value::Array(responses))
    }
}

pub async fn handle_jsonrpc<H>(
    handler: &H,
    user: &UserAuth,
    request_context: &RequestContext,
    payload: Value,
) -> Option<Value>
where
    H: McpMethodHandler,
{
    let request = match serde_json::from_value::<JsonRpcRequest>(payload) {
        Ok(request) => request,
        Err(err) => {
            return Some(error(
                None,
                INVALID_REQUEST,
                format!("invalid JSON-RPC request: {err}"),
                request_context,
            ))
        }
    };

    let id = request.id.clone();
    if request.jsonrpc.as_deref() != Some(JSONRPC_VERSION) {
        return Some(error(
            id,
            INVALID_REQUEST,
            "JSON-RPC version must be 2.0",
            request_context,
        ));
    }
    let Some(method) = request.method.as_deref() else {
        return if request.is_notification() {
            None
        } else {
            Some(error(
                id,
                INVALID_REQUEST,
                "missing method",
                request_context,
            ))
        };
    };

    if request.is_notification() {
        handler.handle_notification(method).await;
        return None;
    }

    Some(
        match handler
            .handle_method(method, request.params, user, request_context)
            .await
        {
            Ok(result) => success(id, result),
            Err(err) => error(id, err.code, err.message, request_context),
        },
    )
}

pub fn unsupported_method(method: &str) -> McpError {
    McpError::new(
        METHOD_NOT_FOUND,
        format!("unsupported MCP method: {method}"),
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    struct EchoHandler;

    #[async_trait]
    impl McpMethodHandler for EchoHandler {
        async fn handle_method(
            &self,
            method: &str,
            params: Option<Value>,
            _user: &UserAuth,
            _request_context: &RequestContext,
        ) -> Result<Value, McpError> {
            if method == "echo" {
                Ok(json!({ "params": params }))
            } else {
                Err(unsupported_method(method))
            }
        }
    }

    fn user() -> UserAuth {
        UserAuth::human("", "tester", "UTC", vec![], vec![], "").with_ingress("mcp")
    }

    fn request_context() -> RequestContext {
        RequestContext {
            request_id: "req-1".to_string(),
            correlation_id: "corr-1".to_string(),
        }
    }

    #[tokio::test]
    async fn jsonrpc_loop_wraps_success_result() {
        let result = handle_jsonrpc(
            &EchoHandler,
            &user(),
            &request_context(),
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "echo",
                "params": { "ok": true }
            }),
        )
        .await
        .expect("response");

        assert_eq!(result["id"], json!(1));
        assert_eq!(result["result"]["params"]["ok"], json!(true));
    }

    #[tokio::test]
    async fn jsonrpc_loop_maps_invalid_requests() {
        let result = handle_jsonrpc(&EchoHandler, &user(), &request_context(), json!([]))
            .await
            .expect("response");

        assert_eq!(result["error"]["code"], json!(INVALID_REQUEST));
    }

    #[tokio::test]
    async fn jsonrpc_loop_suppresses_notifications() {
        let result = handle_jsonrpc(
            &EchoHandler,
            &user(),
            &request_context(),
            json!({
                "jsonrpc": "2.0",
                "method": "echo"
            }),
        )
        .await;

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn payload_loop_handles_batch_limits() {
        let result = handle_payload(&EchoHandler, &user(), &request_context(), json!([]), 2).await;

        assert_eq!(result.status, McpJsonRpcStatus::BadRequest);
        assert_eq!(
            result.body.expect("body")["error"]["code"],
            json!(INVALID_REQUEST)
        );
    }

    #[tokio::test]
    async fn payload_loop_accepts_notification_only_batches() {
        let result = handle_payload(
            &EchoHandler,
            &user(),
            &request_context(),
            json!([{ "jsonrpc": "2.0", "method": "echo" }]),
            2,
        )
        .await;

        assert_eq!(result, McpJsonRpcResponse::accepted());
    }
}
