//! Model Context Protocol surface for App Framework backends.
//!
//! MCP is intentionally a transport adapter. Tool execution goes through the
//! public handler-level operations and then the same DataAccess, QueryIR,
//! policy, audit, provider, and observability path as generated GraphQL
//! resolvers.

use appfw_runtime::{
    extension::UserAuth,
    mcp::access::{user_can_use_privileged_tool, McpError},
    mcp::http::{mcp_runtime_routes, McpRuntimeState},
    mcp::server::McpMethodHandler,
    mcp::service::{handle_method as runtime_handle_method, McpRuntimeServiceContext},
    mcp::tool_call::{
        mcp_explain_access_result, McpAccessDecision, McpAccessExplainProvider,
        McpExplainAccessResult, McpGeneratedToolAuditSink, McpGeneratedToolAuditStore,
        McpToolOutcome,
    },
    observability::RequestContext,
    operation::RuntimeOperation,
    security::SecurityConfig,
    AccessAction, RuntimeAuthState,
};
use async_trait::async_trait;
use axum::Router;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};

use crate::{
    config::app_config::AppConfig, data::data_access::DataAccess,
    operations::generated as generated_operations, product_api::runtime_model_metadata,
    routes::app_error::AppError, schemas::system::EntityType,
};

#[derive(Clone)]
struct McpState {
    app_config: Arc<AppConfig>,
    app_state: RuntimeAuthState,
    data_access_by_schema: HashMap<String, Arc<DataAccess>>,
    security: SecurityConfig,
}

pub(crate) fn get_routes(
    app_config: Arc<AppConfig>,
    app_state: RuntimeAuthState,
    security: SecurityConfig,
    data_access_by_schema: HashMap<String, Arc<DataAccess>>,
) -> Router {
    mcp_runtime_routes::<McpState>().with_state(McpState {
        app_config,
        app_state,
        data_access_by_schema,
        security,
    })
}

impl McpRuntimeState for McpState {
    fn auth_state(&self) -> RuntimeAuthState {
        self.app_state.clone()
    }

    fn security_config(&self) -> &SecurityConfig {
        &self.security
    }
}

#[async_trait]
impl McpMethodHandler for McpState {
    async fn handle_method(
        &self,
        method: &str,
        params: Option<Value>,
        user: &UserAuth,
        request_context: &RequestContext,
    ) -> Result<Value, McpError> {
        let runtime_model = runtime_model_metadata(&self.app_config);
        let dispatcher = generated_operations::GeneratedOperationDispatcher::new(
            self.app_config.clone(),
            &self.data_access_by_schema,
        );
        let access_explainer = ProductMcpAccessExplainer { state: self };
        let audit_sink = McpGeneratedToolAuditSink::new(self);
        let context = McpRuntimeServiceContext {
            model: &runtime_model,
            dispatcher: &dispatcher,
            access_explainer: Some(&access_explainer),
            extension_tools: None,
            audit_sink: Some(&audit_sink),
            server_name: "app-framework",
            server_version: env!("CARGO_PKG_VERSION"),
            max_resource_bytes: self.security.mcp_max_resource_bytes,
            max_result_bytes: self.security.mcp_max_result_bytes,
            privileged: user_can_use_privileged_tool(&self.security, user),
        };

        runtime_handle_method(&context, method, params, user, request_context).await
    }
}

struct ProductMcpAccessExplainer<'a> {
    state: &'a McpState,
}

#[async_trait]
impl McpAccessExplainProvider for ProductMcpAccessExplainer<'_> {
    async fn explain_access(
        &self,
        schema_name: &str,
        type_name: &str,
        action: AccessAction,
        user: &UserAuth,
    ) -> Result<McpExplainAccessResult, McpError> {
        let entity_type = entity_type(&self.state.app_config, schema_name, type_name)?;
        let access = self
            .state
            .app_config
            .evaluate_user_access(entity_type.clone(), action, user)
            .map_err(mcp_error_from_app_error)?;

        Ok(mcp_explain_access_result(
            entity_type.schema_name.clone(),
            entity_type.pascal_1.clone(),
            McpAccessDecision::from(access),
        ))
    }
}

#[async_trait]
impl McpGeneratedToolAuditStore for McpState {
    async fn append_mcp_generated_tool_audit_event(
        &self,
        operation: &RuntimeOperation,
        user: &UserAuth,
        outcome: McpToolOutcome,
        metadata_json: Value,
    ) -> Result<(), McpError> {
        let entity_type =
            entity_type(&self.app_config, operation.schema_name, operation.type_name)?;
        let data_access = self
            .data_access_by_schema
            .get(operation.schema_name)
            .cloned()
            .ok_or_else(|| {
                McpError::invalid_params(format!("unknown schema: {}", operation.schema_name))
            })?;
        data_access
            .append_mcp_tool_audit_event(entity_type, user, outcome.as_str(), metadata_json)
            .await
            .map_err(mcp_error_from_app_error)
    }
}

fn entity_type(
    app_config: &AppConfig,
    schema_name: &str,
    type_name: &str,
) -> Result<Arc<EntityType>, McpError> {
    app_config
        .get_entity_type(&schema_name.to_string(), &type_name.to_string())
        .map_err(mcp_error_from_app_error)
}

fn mcp_error_from_app_error(err: AppError) -> McpError {
    McpError::invalid_params(err.to_string())
}
