//! Runtime MCP method dispatch shell.

use serde_json::{json, Value};

use crate::{
    extension::UserAuth, model_metadata::RuntimeModelMetadata, observability::RequestContext,
    operation::RuntimeOperationDispatcher,
};

use super::{
    access::McpError,
    builtin::{handle_builtin_method, McpBuiltinContext},
    catalog,
    server::unsupported_method,
    tool_call::{
        handle_tool_call, McpAccessExplainProvider, McpToolAuditSink, McpToolCallContext,
        McpToolExtension,
    },
};

pub struct McpRuntimeServiceContext<'a> {
    pub model: &'a RuntimeModelMetadata,
    pub dispatcher: &'a dyn RuntimeOperationDispatcher,
    pub access_explainer: Option<&'a dyn McpAccessExplainProvider>,
    pub extension_tools: Option<&'a dyn McpToolExtension>,
    pub audit_sink: Option<&'a dyn McpToolAuditSink>,
    pub server_name: &'a str,
    pub server_version: &'a str,
    pub max_resource_bytes: usize,
    pub max_result_bytes: usize,
    pub privileged: bool,
}

pub async fn handle_method(
    context: &McpRuntimeServiceContext<'_>,
    method: &str,
    params: Option<Value>,
    user: &UserAuth,
    request_context: &RequestContext,
) -> Result<Value, McpError> {
    let builtin_context = McpBuiltinContext {
        model: context.model,
        max_resource_bytes: context.max_resource_bytes,
        server_name: context.server_name,
        server_version: context.server_version,
    };
    if let Some(result) =
        handle_builtin_method(&builtin_context, method, params.clone(), request_context)
    {
        return result;
    }

    match method {
        "tools/list" => Ok(json!({ "tools": tool_list(context) })),
        "tools/call" => {
            let tool_context = McpToolCallContext {
                model: context.model,
                dispatcher: context.dispatcher,
                access_explainer: context.access_explainer,
                extension_tools: context.extension_tools,
                audit_sink: context.audit_sink,
                user,
                request_context,
                privileged: context.privileged,
                max_result_bytes: context.max_result_bytes,
            };
            handle_tool_call(&tool_context, params).await
        }
        _ => Err(unsupported_method(method)),
    }
}

pub fn tool_list(context: &McpRuntimeServiceContext<'_>) -> Vec<Value> {
    let mut tools = catalog::support_tool_list();
    tools.extend(catalog::operation_tool_list(
        &context.dispatcher.list_operations(false),
    ));
    tools
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::Mutex;

    use crate::{
        model_metadata::{
            RuntimeDataSourceMetadata, RuntimeDataType, RuntimeEntityMetadata,
            RuntimePropertyMetadata, RuntimeSchemaMetadata,
        },
        operation::{
            RuntimeOperation, RuntimeOperationArg, RuntimeOperationCatalog, RuntimeOperationRequest,
        },
        provider_keys::FrameworkProvider,
        RuntimeAppError,
    };

    use super::*;

    struct Dispatcher {
        calls: Mutex<Vec<RuntimeOperationRequest>>,
    }

    impl Dispatcher {
        fn new() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
            }
        }
    }

    impl RuntimeOperationCatalog for Dispatcher {
        fn list_operations(&self, _include_disabled: bool) -> Vec<RuntimeOperation> {
            vec![RuntimeOperation::query(
                "query_accounts",
                "crm",
                "Account",
                "AccountQueryResult",
                vec![RuntimeOperationArg::optional("filter", "serde_json::Value")],
                true,
            )]
        }
    }

    #[async_trait]
    impl RuntimeOperationDispatcher for Dispatcher {
        async fn call_operation(
            &self,
            _user: UserAuth,
            request: RuntimeOperationRequest,
        ) -> Result<Value, RuntimeAppError> {
            self.calls.lock().expect("calls lock").push(request);
            Ok(json!({ "ok": true }))
        }
    }

    fn user() -> UserAuth {
        UserAuth::human("", "tester", "UTC", Vec::new(), Vec::new(), "").with_ingress("mcp")
    }

    fn request_context() -> RequestContext {
        RequestContext {
            request_id: "request-1".to_string(),
            correlation_id: "correlation-1".to_string(),
        }
    }

    fn model() -> RuntimeModelMetadata {
        RuntimeModelMetadata::new(
            vec![RuntimeDataSourceMetadata {
                name: "crm_db".to_string(),
                description: None,
                provider: FrameworkProvider::Postgres,
                is_system_schema_host: false,
                environments: Vec::new(),
            }],
            vec![RuntimeSchemaMetadata {
                id: "schema-crm".to_string(),
                name: "crm".to_string(),
                description: "CRM".to_string(),
                data_source_name: "crm_db".to_string(),
            }],
            vec![RuntimeEntityMetadata {
                id: "entity-account".to_string(),
                schema_name: "crm".to_string(),
                schema_id: Some("schema-crm".to_string()),
                pascal_1: "Account".to_string(),
                pascal_n: "Accounts".to_string(),
                snake_1: "account".to_string(),
                snake_n: "accounts".to_string(),
                caption_1: "Account".to_string(),
                caption_n: "Accounts".to_string(),
                is_union: false,
                base_type: None,
                is_table: true,
                facets: Vec::new(),
                meta: None,
                standard_methods: Vec::new(),
                custom_methods: Vec::new(),
                properties: vec![RuntimePropertyMetadata {
                    id: "prop-id".to_string(),
                    name: "id".to_string(),
                    caption: "id".to_string(),
                    data_type: RuntimeDataType::Uuid,
                    is_key: true,
                    is_caption: false,
                    is_required: true,
                    is_read_only: false,
                    is_concurrency_control: false,
                    default_value: None,
                    foreign_key: None,
                    nav_by_fk: None,
                    many_to_many: None,
                    nested_entity_type: None,
                    enum_type_name: None,
                    meta: None,
                }],
            }],
        )
    }

    fn context<'a>(
        model: &'a RuntimeModelMetadata,
        dispatcher: &'a Dispatcher,
    ) -> McpRuntimeServiceContext<'a> {
        McpRuntimeServiceContext {
            model,
            dispatcher,
            access_explainer: None,
            extension_tools: None,
            audit_sink: None,
            server_name: "app-framework",
            server_version: env!("CARGO_PKG_VERSION"),
            max_resource_bytes: 256 * 1024,
            max_result_bytes: 256 * 1024,
            privileged: false,
        }
    }

    #[tokio::test]
    async fn service_handles_builtin_methods() {
        let model = model();
        let dispatcher = Dispatcher::new();
        let result = handle_method(
            &context(&model, &dispatcher),
            "initialize",
            None,
            &user(),
            &request_context(),
        )
        .await
        .expect("initialize succeeds");

        assert_eq!(result["serverInfo"]["name"], json!("app-framework"));
        assert!(dispatcher.calls.lock().expect("calls lock").is_empty());
    }

    #[tokio::test]
    async fn service_lists_support_and_generated_tools() {
        let model = model();
        let dispatcher = Dispatcher::new();
        let result = handle_method(
            &context(&model, &dispatcher),
            "tools/list",
            None,
            &user(),
            &request_context(),
        )
        .await
        .expect("tools/list succeeds");

        let tool_names = result["tools"]
            .as_array()
            .expect("tools")
            .iter()
            .filter_map(|tool| tool.get("name").and_then(Value::as_str))
            .collect::<Vec<_>>();
        assert!(tool_names.contains(&"appfw_list_schemas"));
        assert!(tool_names.contains(&"query_accounts"));
    }

    #[tokio::test]
    async fn service_dispatches_generated_tool_calls() {
        let model = model();
        let dispatcher = Dispatcher::new();
        let result = handle_method(
            &context(&model, &dispatcher),
            "tools/call",
            Some(json!({
                "name": "query_accounts",
                "arguments": {
                    "filter": { "id": { "eq": "account-1" } },
                    "fields": ["id"]
                }
            })),
            &user(),
            &request_context(),
        )
        .await
        .expect("tools/call succeeds");

        let text = result["content"][0]["text"].as_str().expect("text result");
        assert!(text.contains("\"ok\""));
        assert_eq!(
            dispatcher.calls.lock().expect("calls lock")[0].fields,
            Some(vec!["id".to_string()])
        );
    }
}
