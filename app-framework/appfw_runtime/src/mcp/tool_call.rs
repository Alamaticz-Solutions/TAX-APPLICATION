//! Runtime MCP `tools/call` execution shell.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use tracing::warn;

use crate::{
    extension::UserAuth,
    model_metadata::RuntimeModelMetadata,
    observability::{redact_diagnostic_text, RequestContext},
    operation::{RuntimeOperation, RuntimeOperationDispatcher, RuntimeOperationRequest},
    AccessAction, PolicyAccess, RuntimeAppError,
};

use super::{
    access::{tool_error, tool_success, McpError},
    catalog,
    protocol::{INVALID_PARAMS, METHOD_NOT_FOUND},
    support_tool::{handle_support_tool, McpSupportToolContext},
};

pub struct McpToolCallContext<'a> {
    pub model: &'a RuntimeModelMetadata,
    pub dispatcher: &'a dyn RuntimeOperationDispatcher,
    pub access_explainer: Option<&'a dyn McpAccessExplainProvider>,
    pub extension_tools: Option<&'a dyn McpToolExtension>,
    pub audit_sink: Option<&'a dyn McpToolAuditSink>,
    pub user: &'a UserAuth,
    pub request_context: &'a RequestContext,
    pub privileged: bool,
    pub max_result_bytes: usize,
}

pub const MCP_EXPLAIN_ACCESS_TOOL: &str = "appfw_explain_access";

#[derive(Debug, Deserialize)]
pub struct McpExplainAccessArgs {
    pub schema_name: String,
    pub type_name: String,
    pub action: String,
}

#[derive(Debug)]
pub struct McpExplainAccessSubject {
    pub schema_name: String,
    pub type_name: String,
}

#[derive(Debug)]
pub struct McpExplainAccessResult {
    pub subject: McpExplainAccessSubject,
    pub decision: McpAccessDecision,
}

#[derive(Debug)]
pub struct McpAccessDecision {
    pub allow: bool,
    pub filter: Option<Value>,
}

impl From<PolicyAccess> for McpAccessDecision {
    fn from(access: PolicyAccess) -> Self {
        Self {
            allow: access.allow,
            filter: access.filter,
        }
    }
}

pub fn mcp_explain_access_result(
    schema_name: impl Into<String>,
    type_name: impl Into<String>,
    decision: McpAccessDecision,
) -> McpExplainAccessResult {
    McpExplainAccessResult {
        subject: McpExplainAccessSubject {
            schema_name: schema_name.into(),
            type_name: type_name.into(),
        },
        decision,
    }
}

#[async_trait]
pub trait McpAccessExplainProvider: Sync {
    async fn explain_access(
        &self,
        schema_name: &str,
        type_name: &str,
        action: AccessAction,
        user: &UserAuth,
    ) -> Result<McpExplainAccessResult, McpError>;
}

#[async_trait]
pub trait McpToolExtension: Sync {
    async fn call_tool(
        &self,
        name: &str,
        arguments: &Value,
        user: &UserAuth,
        privileged: bool,
    ) -> Option<Result<Value, McpError>>;
}

#[async_trait]
pub trait McpToolAuditSink: Sync {
    async fn audit_tool_call(
        &self,
        user: &UserAuth,
        operation: &RuntimeOperation,
        tool_name: &str,
        outcome: McpToolOutcome,
        arguments_hash: &str,
        request_context: &RequestContext,
        error: Option<&str>,
    ) -> Result<(), McpError>;
}

#[async_trait]
pub trait McpGeneratedToolAuditStore: Sync {
    async fn append_mcp_generated_tool_audit_event(
        &self,
        operation: &RuntimeOperation,
        user: &UserAuth,
        outcome: McpToolOutcome,
        metadata_json: Value,
    ) -> Result<(), McpError>;
}

pub struct McpGeneratedToolAuditSink<'a, T>
where
    T: McpGeneratedToolAuditStore + ?Sized,
{
    store: &'a T,
}

impl<'a, T> McpGeneratedToolAuditSink<'a, T>
where
    T: McpGeneratedToolAuditStore + ?Sized,
{
    pub fn new(store: &'a T) -> Self {
        Self { store }
    }
}

#[async_trait]
impl<T> McpToolAuditSink for McpGeneratedToolAuditSink<'_, T>
where
    T: McpGeneratedToolAuditStore + ?Sized,
{
    async fn audit_tool_call(
        &self,
        user: &UserAuth,
        operation: &RuntimeOperation,
        tool_name: &str,
        outcome: McpToolOutcome,
        arguments_hash: &str,
        request_context: &RequestContext,
        error: Option<&str>,
    ) -> Result<(), McpError> {
        self.store
            .append_mcp_generated_tool_audit_event(
                operation,
                user,
                outcome,
                mcp_tool_audit_payload(
                    operation,
                    tool_name,
                    outcome,
                    arguments_hash,
                    request_context,
                    error,
                ),
            )
            .await
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum McpToolOutcome {
    Succeeded,
    Failed,
}

impl McpToolOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }
}

pub fn mcp_tool_audit_payload(
    operation: &RuntimeOperation,
    tool_name: &str,
    outcome: McpToolOutcome,
    arguments_hash: &str,
    request_context: &RequestContext,
    error: Option<&str>,
) -> Value {
    serde_json::json!({
        "tool": {
            "name": tool_name,
            "operation_name": operation.name,
            "schema_name": operation.schema_name,
            "type_name": operation.type_name,
            "kind": operation.kind,
            "outcome": outcome.as_str(),
            "arguments_sha256": arguments_hash,
        },
        "request": {
            "request_id": request_context.request_id,
            "correlation_id": request_context.correlation_id,
        },
        "error": error.map(redact_diagnostic_text),
    })
}

pub async fn handle_tool_call(
    context: &McpToolCallContext<'_>,
    params: Option<Value>,
) -> Result<Value, McpError> {
    let params = params_obj(params, "tools/call params")?;
    let name = required_string(&params, "name")?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| Value::Object(Map::new()));

    let support_context = McpSupportToolContext {
        model: context.model,
        operations: context.dispatcher,
        privileged: context.privileged,
    };
    let mut audit_target: Option<(RuntimeOperation, String, String)> = None;

    let result = if let Some(result) = handle_support_tool(&support_context, &name, &arguments) {
        result
    } else if name == MCP_EXPLAIN_ACCESS_TOOL {
        call_explain_access_tool(context, arguments).await
    } else if let Some(extension_tools) = context.extension_tools {
        match extension_tools
            .call_tool(&name, &arguments, context.user, context.privileged)
            .await
        {
            Some(result) => result,
            None => call_generated_tool(context, &name, arguments, &mut audit_target).await,
        }
    } else {
        call_generated_tool(context, &name, arguments, &mut audit_target).await
    };

    shape_tool_result(context, result, audit_target).await
}

async fn call_explain_access_tool(
    context: &McpToolCallContext<'_>,
    arguments: Value,
) -> Result<Value, McpError> {
    require_privileged_tool(context.privileged)?;
    let Some(access_explainer) = context.access_explainer else {
        return Err(McpError::new(
            INVALID_PARAMS,
            "MCP access explain provider is not configured",
        ));
    };
    let args: McpExplainAccessArgs = decode_args(arguments)?;
    let action = explain_access_action(&args.action)?;
    let result = access_explainer
        .explain_access(&args.schema_name, &args.type_name, action, context.user)
        .await?;

    Ok(serde_json::json!({
        "schema_name": result.subject.schema_name,
        "type_name": result.subject.type_name,
        "action": args.action,
        "user_name": context.user.user_name.clone(),
        "roles": context.user.roles.clone(),
        "decision": {
            "allow": result.decision.allow,
            "filter": result.decision.filter,
        }
    }))
}

async fn call_generated_tool(
    context: &McpToolCallContext<'_>,
    name: &str,
    arguments: Value,
    audit_target: &mut Option<(RuntimeOperation, String, String)>,
) -> Result<Value, McpError> {
    let Some(operation) =
        catalog::resolve_operation_tool(&context.dispatcher.list_operations(false), name)
    else {
        return Err(McpError::new(
            METHOD_NOT_FOUND,
            format!("unknown MCP tool: {name}"),
        ));
    };

    *audit_target = Some((operation.clone(), name.to_string(), hash_value(&arguments)));
    call_reflected_operation(
        context.dispatcher,
        context.user.clone(),
        operation,
        arguments,
    )
    .await
}

async fn call_reflected_operation(
    dispatcher: &dyn RuntimeOperationDispatcher,
    user: UserAuth,
    operation: RuntimeOperation,
    arguments: Value,
) -> Result<Value, McpError> {
    let mut arguments = params_obj(Some(arguments), "handler operation arguments")?;
    let fields = if operation.selection_aware {
        arguments
            .remove("fields")
            .map(serde_json::from_value::<Vec<String>>)
            .transpose()
            .map_err(|err| McpError::new(INVALID_PARAMS, format!("invalid fields: {err}")))?
    } else {
        None
    };

    dispatcher
        .call_operation(
            user,
            RuntimeOperationRequest {
                name: operation.name.to_string(),
                schema_name: Some(operation.schema_name.to_string()),
                type_name: Some(operation.type_name.to_string()),
                arguments,
                fields,
            },
        )
        .await
        .map_err(mcp_error_from_runtime_app_error)
}

fn mcp_error_from_runtime_app_error(err: RuntimeAppError) -> McpError {
    McpError::invalid_params(err.to_string())
}

async fn shape_tool_result(
    context: &McpToolCallContext<'_>,
    result: Result<Value, McpError>,
    audit_target: Option<(RuntimeOperation, String, String)>,
) -> Result<Value, McpError> {
    Ok(match result {
        Ok(value) => match tool_success(value, context.max_result_bytes) {
            Ok(response) => {
                audit_generated_tool(context, audit_target, McpToolOutcome::Succeeded, None)
                    .await?;
                response
            }
            Err(err) => {
                audit_generated_tool(
                    context,
                    audit_target,
                    McpToolOutcome::Failed,
                    Some(err.message.as_str()),
                )
                .await?;
                return Err(err);
            }
        },
        Err(err) => {
            warn!(
                request_id = %context.request_context.request_id,
                correlation_id = %context.request_context.correlation_id,
                error = %redact_diagnostic_text(&err.message),
                "MCP tool failed"
            );
            audit_generated_tool(
                context,
                audit_target,
                McpToolOutcome::Failed,
                Some(err.message.as_str()),
            )
            .await?;
            tool_error(err.message)
        }
    })
}

async fn audit_generated_tool(
    context: &McpToolCallContext<'_>,
    audit_target: Option<(RuntimeOperation, String, String)>,
    outcome: McpToolOutcome,
    error: Option<&str>,
) -> Result<(), McpError> {
    let Some(audit_sink) = context.audit_sink else {
        return Ok(());
    };
    let Some((operation, tool_name, arguments_hash)) = audit_target else {
        return Ok(());
    };

    audit_sink
        .audit_tool_call(
            context.user,
            &operation,
            &tool_name,
            outcome,
            &arguments_hash,
            context.request_context,
            error,
        )
        .await
}

fn params_obj(params: Option<Value>, label: &str) -> Result<Map<String, Value>, McpError> {
    match params.unwrap_or_else(|| Value::Object(Map::new())) {
        Value::Object(obj) => Ok(obj),
        other => Err(McpError::new(
            INVALID_PARAMS,
            format!("{label} must be an object, got {}", value_kind(&other)),
        )),
    }
}

fn decode_args<T: for<'de> Deserialize<'de>>(value: Value) -> Result<T, McpError> {
    serde_json::from_value(value)
        .map_err(|err| McpError::new(INVALID_PARAMS, format!("invalid tool arguments: {err}")))
}

fn explain_access_action(value: &str) -> Result<AccessAction, McpError> {
    match value {
        "read" => Ok(AccessAction::Read),
        "create" => Ok(AccessAction::Create),
        "update" => Ok(AccessAction::Update),
        "delete" => Ok(AccessAction::Delete),
        _ => Err(McpError::new(
            INVALID_PARAMS,
            "action must be one of read, create, update, delete",
        )),
    }
}

fn require_privileged_tool(privileged: bool) -> Result<(), McpError> {
    if privileged {
        Ok(())
    } else {
        Err(McpError::new(
            INVALID_PARAMS,
            "MCP tool requires a privileged role or OAuth scope",
        ))
    }
}

fn required_string(obj: &Map<String, Value>, field: &str) -> Result<String, McpError> {
    obj.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| McpError::new(INVALID_PARAMS, format!("missing string field: {field}")))
}

fn hash_value(value: &Value) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model_metadata::{
            RuntimeDataSourceMetadata, RuntimeDataType, RuntimeEntityMetadata,
            RuntimePropertyMetadata, RuntimeSchemaMetadata,
        },
        operation::{
            RuntimeOperation, RuntimeOperationArg, RuntimeOperationCatalog, RuntimeOperationRequest,
        },
        provider_keys::FrameworkProvider,
    };
    use serde_json::json;
    use std::sync::Mutex;

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
        fn list_operations(&self, include_disabled: bool) -> Vec<RuntimeOperation> {
            let operations = vec![
                RuntimeOperation::query(
                    "query_accounts",
                    "crm",
                    "Account",
                    "AccountQueryResult",
                    vec![RuntimeOperationArg::optional("filter", "serde_json::Value")],
                    true,
                ),
                RuntimeOperation::disabled_mutation(
                    "create_account",
                    "crm",
                    "Account",
                    "Account",
                    vec![],
                    false,
                ),
            ];
            if include_disabled {
                operations
            } else {
                operations
                    .into_iter()
                    .filter(|operation| operation.enabled)
                    .collect()
            }
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

    struct Audit {
        outcomes: Mutex<Vec<McpToolOutcome>>,
    }

    impl Audit {
        fn new() -> Self {
            Self {
                outcomes: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl McpToolAuditSink for Audit {
        async fn audit_tool_call(
            &self,
            _user: &UserAuth,
            _operation: &RuntimeOperation,
            _tool_name: &str,
            outcome: McpToolOutcome,
            _arguments_hash: &str,
            _request_context: &RequestContext,
            _error: Option<&str>,
        ) -> Result<(), McpError> {
            self.outcomes.lock().expect("audit lock").push(outcome);
            Ok(())
        }
    }

    struct GeneratedAuditStore {
        events: Mutex<Vec<(RuntimeOperation, McpToolOutcome, Value)>>,
    }

    impl GeneratedAuditStore {
        fn new() -> Self {
            Self {
                events: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl McpGeneratedToolAuditStore for GeneratedAuditStore {
        async fn append_mcp_generated_tool_audit_event(
            &self,
            operation: &RuntimeOperation,
            _user: &UserAuth,
            outcome: McpToolOutcome,
            metadata_json: Value,
        ) -> Result<(), McpError> {
            self.events.lock().expect("generated audit lock").push((
                operation.clone(),
                outcome,
                metadata_json,
            ));
            Ok(())
        }
    }

    struct AccessExplainer;

    #[async_trait]
    impl McpAccessExplainProvider for AccessExplainer {
        async fn explain_access(
            &self,
            schema_name: &str,
            type_name: &str,
            _action: AccessAction,
            _user: &UserAuth,
        ) -> Result<McpExplainAccessResult, McpError> {
            Ok(mcp_explain_access_result(
                schema_name,
                type_name,
                McpAccessDecision {
                    allow: true,
                    filter: Some(json!({ "owner_id": "tester" })),
                },
            ))
        }
    }

    struct Extension;

    #[async_trait]
    impl McpToolExtension for Extension {
        async fn call_tool(
            &self,
            name: &str,
            _arguments: &Value,
            _user: &UserAuth,
            _privileged: bool,
        ) -> Option<Result<Value, McpError>> {
            match name {
                "custom_tool" => Some(Ok(json!({ "custom": true }))),
                _ => None,
            }
        }
    }

    fn context<'a>(
        model: &'a RuntimeModelMetadata,
        dispatcher: &'a Dispatcher,
        access_explainer: Option<&'a dyn McpAccessExplainProvider>,
        extension_tools: Option<&'a dyn McpToolExtension>,
        audit_sink: Option<&'a dyn McpToolAuditSink>,
        user: &'a UserAuth,
        request_context: &'a RequestContext,
        privileged: bool,
    ) -> McpToolCallContext<'a> {
        McpToolCallContext {
            model,
            dispatcher,
            access_explainer,
            extension_tools,
            audit_sink,
            user,
            request_context,
            privileged,
            max_result_bytes: 256 * 1024,
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

    #[test]
    fn explain_access_result_shapes_subject_and_decision() {
        let result = mcp_explain_access_result(
            "crm",
            "Account",
            McpAccessDecision {
                allow: true,
                filter: Some(json!({ "owner_id": "tester" })),
            },
        );

        assert_eq!(result.subject.schema_name, "crm");
        assert_eq!(result.subject.type_name, "Account");
        assert!(result.decision.allow);
        assert_eq!(
            result.decision.filter,
            Some(json!({ "owner_id": "tester" }))
        );
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

    #[tokio::test]
    async fn support_tools_execute_before_generated_dispatch() {
        let model = model();
        let dispatcher = Dispatcher::new();
        let user = user();
        let request_context = request_context();

        let result = handle_tool_call(
            &context(
                &model,
                &dispatcher,
                None,
                None,
                None,
                &user,
                &request_context,
                false,
            ),
            Some(json!({ "name": "appfw_list_schemas" })),
        )
        .await
        .expect("support tool succeeds");

        let text = result["content"][0]["text"].as_str().expect("text result");
        assert!(text.contains("\"schemas\""));
        assert!(dispatcher.calls.lock().expect("calls lock").is_empty());
    }

    #[tokio::test]
    async fn reflected_generated_tools_dispatch_and_audit_success() {
        let model = model();
        let dispatcher = Dispatcher::new();
        let audit = Audit::new();
        let user = user();
        let request_context = request_context();

        let result = handle_tool_call(
            &context(
                &model,
                &dispatcher,
                None,
                None,
                Some(&audit),
                &user,
                &request_context,
                false,
            ),
            Some(json!({
                "name": "query_accounts",
                "arguments": {
                    "filter": { "id": { "eq": "account-1" } },
                    "fields": ["id"]
                }
            })),
        )
        .await
        .expect("generated operation succeeds");

        let text = result["content"][0]["text"].as_str().expect("text result");
        assert!(text.contains("\"ok\""));

        let calls = dispatcher.calls.lock().expect("calls lock");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "query_accounts");
        assert_eq!(calls[0].fields, Some(vec!["id".to_string()]));
        assert!(!calls[0].arguments.contains_key("fields"));

        assert_eq!(
            audit.outcomes.lock().expect("audit lock").as_slice(),
            &[McpToolOutcome::Succeeded]
        );
    }

    #[tokio::test]
    async fn generated_tool_audit_sink_shapes_metadata_for_product_store() {
        let store = GeneratedAuditStore::new();
        let sink = McpGeneratedToolAuditSink::new(&store);
        let operation = RuntimeOperation::query(
            "query_accounts",
            "crm",
            "Account",
            "AccountQueryResult",
            vec![],
            true,
        );

        sink.audit_tool_call(
            &user(),
            &operation,
            "query_accounts",
            McpToolOutcome::Failed,
            "abc123",
            &request_context(),
            Some("plain failure"),
        )
        .await
        .expect("audit append succeeds");

        let events = store.events.lock().expect("generated audit lock");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0.name, "query_accounts");
        assert_eq!(events[0].1, McpToolOutcome::Failed);
        assert_eq!(events[0].2["tool"]["schema_name"], "crm");
        assert_eq!(events[0].2["tool"]["type_name"], "Account");
        assert_eq!(events[0].2["tool"]["arguments_sha256"], "abc123");
        assert_eq!(events[0].2["request"]["request_id"], "request-1");
        assert_eq!(events[0].2["error"], "plain failure");
    }

    #[tokio::test]
    async fn explain_access_tool_uses_runtime_provider() {
        let model = model();
        let dispatcher = Dispatcher::new();
        let access_explainer = AccessExplainer;
        let user = user();
        let request_context = request_context();

        let result = handle_tool_call(
            &context(
                &model,
                &dispatcher,
                Some(&access_explainer),
                None,
                None,
                &user,
                &request_context,
                true,
            ),
            Some(json!({
                "name": "appfw_explain_access",
                "arguments": {
                    "schema_name": "crm",
                    "type_name": "Account",
                    "action": "read"
                }
            })),
        )
        .await
        .expect("access explain tool succeeds");

        let text = result["content"][0]["text"].as_str().expect("text result");
        assert!(text.contains("\"allow\": true"));
        assert!(text.contains("\"owner_id\""));
        assert!(dispatcher.calls.lock().expect("calls lock").is_empty());
    }

    #[tokio::test]
    async fn explain_access_tool_requires_privileged_user() {
        let model = model();
        let dispatcher = Dispatcher::new();
        let access_explainer = AccessExplainer;
        let user = user();
        let request_context = request_context();

        let result = handle_tool_call(
            &context(
                &model,
                &dispatcher,
                Some(&access_explainer),
                None,
                None,
                &user,
                &request_context,
                false,
            ),
            Some(json!({ "name": "appfw_explain_access" })),
        )
        .await
        .expect("privilege failure is shaped as tool error");

        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .expect("text result")
            .contains("privileged role"));
    }

    #[tokio::test]
    async fn product_extension_tools_are_explicit_callbacks() {
        let model = model();
        let dispatcher = Dispatcher::new();
        let extension = Extension;
        let user = user();
        let request_context = request_context();

        let result = handle_tool_call(
            &context(
                &model,
                &dispatcher,
                None,
                Some(&extension),
                None,
                &user,
                &request_context,
                true,
            ),
            Some(json!({ "name": "custom_tool" })),
        )
        .await
        .expect("extension tool succeeds");

        let text = result["content"][0]["text"].as_str().expect("text result");
        assert!(text.contains("\"custom\""));
        assert!(dispatcher.calls.lock().expect("calls lock").is_empty());
    }

    #[tokio::test]
    async fn unknown_tools_return_tool_error_payload() {
        let model = model();
        let dispatcher = Dispatcher::new();
        let user = user();
        let request_context = request_context();

        let result = handle_tool_call(
            &context(
                &model,
                &dispatcher,
                None,
                None,
                None,
                &user,
                &request_context,
                false,
            ),
            Some(json!({ "name": "missing_tool" })),
        )
        .await
        .expect("unknown tool is shaped as tool error");

        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .expect("text result")
            .contains("unknown MCP tool"));
    }
}
