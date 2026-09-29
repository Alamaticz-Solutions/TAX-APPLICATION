use serde_json::{json, Map, Value};

use crate::{model_metadata::RuntimeModelMetadata, observability::RequestContext};

use super::{
    access::{bounded_json, json_resource, McpError},
    catalog,
    protocol::{INVALID_PARAMS, MCP_PROTOCOL_VERSION, METHOD_NOT_FOUND},
};

pub struct McpBuiltinContext<'a> {
    pub model: &'a RuntimeModelMetadata,
    pub max_resource_bytes: usize,
    pub server_name: &'a str,
    pub server_version: &'a str,
}

pub fn is_builtin_method(method: &str) -> bool {
    matches!(
        method,
        "initialize"
            | "ping"
            | "resources/list"
            | "resources/read"
            | "prompts/list"
            | "prompts/get"
    )
}

pub fn handle_builtin_method(
    context: &McpBuiltinContext<'_>,
    method: &str,
    params: Option<Value>,
    _request_context: &RequestContext,
) -> Option<Result<Value, McpError>> {
    Some(match method {
        "initialize" => Ok(initialize_result(context)),
        "ping" => Ok(json!({})),
        "resources/list" => bounded_json(
            json!({ "resources": catalog::resource_list(context.model) }),
            context.max_resource_bytes,
            "MCP resource list",
        ),
        "resources/read" => read_resource(context, params),
        "prompts/list" => Ok(json!({ "prompts": catalog::prompt_list() })),
        "prompts/get" => get_prompt(params),
        _ => return None,
    })
}

pub fn initialize_result(context: &McpBuiltinContext<'_>) -> Value {
    json!({
        "protocolVersion": MCP_PROTOCOL_VERSION,
        "capabilities": {
            "tools": {},
            "resources": {},
            "prompts": {}
        },
        "serverInfo": {
            "name": context.server_name,
            "version": context.server_version
        },
        "instructions": "Use model-driven App Framework tools and resources. All data access is governed by the backend's auth, policy, tenant isolation, QueryIR, audit, and provider execution path."
    })
}

fn read_resource(
    context: &McpBuiltinContext<'_>,
    params: Option<Value>,
) -> Result<Value, McpError> {
    let params = params_obj(params, "resources/read params")?;
    let uri = required_string(&params, "uri")?;
    let contents = resource_contents(context.model, &uri)?;
    bounded_json(
        json!({ "contents": [contents] }),
        context.max_resource_bytes,
        "MCP resource read",
    )
}

fn resource_contents(model: &RuntimeModelMetadata, uri: &str) -> Result<Value, McpError> {
    if uri == "appfw://schemas" {
        return json_resource(uri, catalog::schema_summaries(model));
    }

    let Some(path) = uri.strip_prefix("appfw://schemas/") else {
        return Err(McpError::new(
            INVALID_PARAMS,
            format!("unknown resource URI: {uri}"),
        ));
    };
    let parts = path.split('/').collect::<Vec<_>>();
    match parts.as_slice() {
        [schema_name] => {
            let schema = model.schema(schema_name).ok_or_else(|| {
                McpError::new(INVALID_PARAMS, format!("unknown schema: {schema_name}"))
            })?;
            json_resource(
                uri,
                json!({
                    "schema": schema,
                    "entities": catalog::entity_summaries(model, schema_name),
                }),
            )
        }
        [schema_name, "entities"] => {
            json_resource(uri, catalog::entity_summaries(model, schema_name))
        }
        [schema_name, "entities", type_name] => {
            let entity = model
                .entity(schema_name, type_name)
                .map_err(|err| McpError::new(INVALID_PARAMS, err.to_string()))?;
            json_resource(uri, catalog::entity_summary(entity))
        }
        _ => Err(McpError::new(
            INVALID_PARAMS,
            format!("unknown resource URI: {uri}"),
        )),
    }
}

fn get_prompt(params: Option<Value>) -> Result<Value, McpError> {
    let params = params_obj(params, "prompts/get params")?;
    let name = required_string(&params, "name")?;
    let arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let prompt = match name.as_str() {
        "appfw_query_entity" => {
            let schema_name = arguments
                .get("schema_name")
                .and_then(Value::as_str)
                .unwrap_or("<schema>");
            let type_name = arguments
                .get("type_name")
                .and_then(Value::as_str)
                .unwrap_or("<entity>");
            format!(
                "Use the App Framework MCP tools to describe {schema_name}.{type_name}, choose explicit fields, then call the reflected query handler tool for that entity with bounded pagination."
            )
        }
        "appfw_design_entity_change" => {
            let schema_name = arguments
                .get("schema_name")
                .and_then(Value::as_str)
                .unwrap_or("<schema>");
            let goal = arguments
                .get("goal")
                .and_then(Value::as_str)
                .unwrap_or("<goal>");
            format!(
                "Plan a safe App Framework config change for schema {schema_name}: {goal}. Start from app_gen/_config, preserve generated ownership boundaries, validate, generate, generate --check, test, and emit handoff JSON."
            )
        }
        "appfw_explain_policy" => {
            "Use appfw_explain_access to inspect the current user's decision, then compare the returned access filter with the entity model and requested operation.".to_string()
        }
        other => {
            return Err(McpError::new(
                METHOD_NOT_FOUND,
                format!("unknown MCP prompt: {other}"),
            ))
        }
    };

    Ok(json!({
        "description": name,
        "messages": [{
            "role": "user",
            "content": {
                "type": "text",
                "text": prompt
            }
        }]
    }))
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

fn required_string(obj: &Map<String, Value>, field: &str) -> Result<String, McpError> {
    obj.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| McpError::new(INVALID_PARAMS, format!("missing string field: {field}")))
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
    use crate::model_metadata::{
        RuntimeDataSourceMetadata, RuntimeDataType, RuntimeEntityMetadata, RuntimePropertyMetadata,
        RuntimeSchemaMetadata,
    };
    use crate::provider_keys::FrameworkProvider;

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

    fn context(model: &RuntimeModelMetadata) -> McpBuiltinContext<'_> {
        McpBuiltinContext {
            model,
            max_resource_bytes: 256 * 1024,
            server_name: "app-framework",
            server_version: env!("CARGO_PKG_VERSION"),
        }
    }

    fn request_context() -> RequestContext {
        RequestContext {
            request_id: "req-1".to_string(),
            correlation_id: "corr-1".to_string(),
        }
    }

    #[test]
    fn initialize_advertises_model_driven_capabilities() {
        let model = model();
        let result = initialize_result(&context(&model));

        assert_eq!(result["protocolVersion"], MCP_PROTOCOL_VERSION);
        assert!(result["capabilities"]["tools"].is_object());
        assert!(result["instructions"]
            .as_str()
            .expect("instructions")
            .contains("QueryIR"));
    }

    #[test]
    fn reads_schema_resources_from_runtime_metadata() {
        let model = model();
        let result = handle_builtin_method(
            &context(&model),
            "resources/read",
            Some(json!({ "uri": "appfw://schemas/crm/entities/Account" })),
            &request_context(),
        )
        .expect("resources/read handled")
        .expect("resource read succeeds");

        let text = result["contents"][0]["text"]
            .as_str()
            .expect("resource text");

        assert!(text.contains("Account"));
    }

    #[test]
    fn unknown_methods_are_not_builtin() {
        let model = model();
        let result =
            handle_builtin_method(&context(&model), "tools/call", None, &request_context());

        assert!(result.is_none());
    }
}
