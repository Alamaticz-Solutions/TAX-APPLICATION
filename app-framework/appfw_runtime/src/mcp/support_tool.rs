//! Schema-neutral MCP support tool execution.

use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::{
    model_metadata::RuntimeModelMetadata,
    operation::{
        resolve_operation, RuntimeOperation, RuntimeOperationCatalog, RuntimeOperationRequest,
    },
};

use super::{access::McpError, catalog, protocol::INVALID_PARAMS};

pub struct McpSupportToolContext<'a> {
    pub model: &'a RuntimeModelMetadata,
    pub operations: &'a dyn RuntimeOperationCatalog,
    pub privileged: bool,
}

#[derive(Debug, Deserialize)]
struct EntityArgs {
    schema_name: String,
    type_name: String,
}

#[derive(Debug, Deserialize)]
struct ListOperationsArgs {
    schema_name: Option<String>,
    type_name: Option<String>,
    include_disabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct DescribeOperationArgs {
    name: String,
    schema_name: Option<String>,
    type_name: Option<String>,
    include_disabled: Option<bool>,
}

pub fn handle_support_tool(
    context: &McpSupportToolContext<'_>,
    name: &str,
    arguments: &Value,
) -> Option<Result<Value, McpError>> {
    Some(match name {
        "appfw_list_schemas" => Ok(json!({
            "schemas": catalog::schema_summaries(context.model),
        })),
        "appfw_list_operations" => list_operations(context, arguments.clone()),
        "appfw_describe_operation" => describe_operation(context, arguments.clone()),
        "appfw_describe_entity" => describe_entity(context, arguments.clone()),
        _ => return None,
    })
}

fn list_operations(
    context: &McpSupportToolContext<'_>,
    arguments: Value,
) -> Result<Value, McpError> {
    let args: ListOperationsArgs = decode_args(arguments)?;
    let include_disabled = args.include_disabled.unwrap_or(false);
    require_privileged_if_needed(context, include_disabled)?;

    Ok(json!({
        "operations": filter_operations(
            context.operations.list_operations(include_disabled),
            args.schema_name.as_deref(),
            args.type_name.as_deref(),
        )
    }))
}

fn describe_operation(
    context: &McpSupportToolContext<'_>,
    arguments: Value,
) -> Result<Value, McpError> {
    let args: DescribeOperationArgs = decode_args(arguments)?;
    let include_disabled = args.include_disabled.unwrap_or(false);
    require_privileged_if_needed(context, include_disabled)?;

    let request = RuntimeOperationRequest {
        name: args.name,
        schema_name: args.schema_name,
        type_name: args.type_name,
        arguments: Map::new(),
        fields: None,
    };
    let operation = resolve_operation(
        &context.operations.list_operations(include_disabled),
        &request,
    )
    .map_err(|err| McpError::new(INVALID_PARAMS, err.to_string()))?;

    Ok(json!(operation))
}

fn describe_entity(
    context: &McpSupportToolContext<'_>,
    arguments: Value,
) -> Result<Value, McpError> {
    let args: EntityArgs = decode_args(arguments)?;
    let entity = context
        .model
        .entity(&args.schema_name, &args.type_name)
        .map_err(|err| McpError::new(INVALID_PARAMS, err.to_string()))?;
    Ok(json!(catalog::entity_summary(entity)))
}

fn require_privileged_if_needed(
    context: &McpSupportToolContext<'_>,
    include_disabled: bool,
) -> Result<(), McpError> {
    if !include_disabled || context.privileged {
        return Ok(());
    }

    Err(McpError::new(
        INVALID_PARAMS,
        "MCP tool requires a privileged role or OAuth scope",
    ))
}

fn filter_operations(
    operations: Vec<RuntimeOperation>,
    schema_name: Option<&str>,
    type_name: Option<&str>,
) -> Vec<RuntimeOperation> {
    operations
        .into_iter()
        .filter(|operation| {
            schema_name
                .map(|schema_name| operation.schema_name == schema_name)
                .unwrap_or(true)
        })
        .filter(|operation| {
            type_name
                .map(|type_name| operation.type_name == type_name)
                .unwrap_or(true)
        })
        .collect()
}

fn decode_args<T: for<'de> Deserialize<'de>>(value: Value) -> Result<T, McpError> {
    serde_json::from_value(value)
        .map_err(|err| McpError::new(INVALID_PARAMS, format!("invalid tool arguments: {err}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model_metadata::{
            RuntimeDataSourceMetadata, RuntimeDataType, RuntimeEntityMetadata,
            RuntimePropertyMetadata, RuntimeSchemaMetadata,
        },
        operation::RuntimeOperationArg,
        provider_keys::FrameworkProvider,
    };

    struct Catalog;

    impl RuntimeOperationCatalog for Catalog {
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

    fn context(model: &RuntimeModelMetadata, privileged: bool) -> McpSupportToolContext<'_> {
        McpSupportToolContext {
            model,
            operations: &Catalog,
            privileged,
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

    #[test]
    fn lists_schema_summaries_from_runtime_model() {
        let model = model();
        let result = handle_support_tool(&context(&model, false), "appfw_list_schemas", &json!({}))
            .expect("support tool handled")
            .expect("tool succeeds");

        assert_eq!(result["schemas"][0]["name"], "crm");
    }

    #[test]
    fn filters_operations_and_requires_privilege_for_disabled_operations() {
        let model = model();
        let err = handle_support_tool(
            &context(&model, false),
            "appfw_list_operations",
            &json!({ "include_disabled": true }),
        )
        .expect("support tool handled")
        .expect_err("disabled operations require privilege");

        assert_eq!(err.code, INVALID_PARAMS);

        let result = handle_support_tool(
            &context(&model, true),
            "appfw_list_operations",
            &json!({ "include_disabled": true, "type_name": "Account" }),
        )
        .expect("support tool handled")
        .expect("tool succeeds");

        assert_eq!(
            result["operations"].as_array().expect("operations").len(),
            2
        );
    }

    #[test]
    fn describes_entity_from_runtime_model() {
        let model = model();
        let result = handle_support_tool(
            &context(&model, false),
            "appfw_describe_entity",
            &json!({ "schema_name": "crm", "type_name": "Account" }),
        )
        .expect("support tool handled")
        .expect("tool succeeds");

        assert_eq!(result["type_name"], "Account");
    }
}
