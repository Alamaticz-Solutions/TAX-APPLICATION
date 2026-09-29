use std::collections::HashMap;

use serde::Serialize;
use serde_json::{json, Value};

use crate::{
    model_metadata::{
        RuntimeDataType, RuntimeEntityMetadata, RuntimeModelMetadata, RuntimePropertyMetadata,
        RuntimeSchemaMetadata,
    },
    query_ir::RuntimePaginationPolicy,
};

use crate::operation::RuntimeOperation;

#[derive(Clone, Debug, Serialize)]
pub struct McpSchemaSummary {
    pub name: String,
    pub description: String,
    pub data_source_name: String,
    pub entity_count: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct McpEntitySummary {
    pub schema_name: String,
    pub type_name: String,
    pub collection_name: String,
    pub caption: String,
    pub is_table: bool,
    pub facets: Vec<String>,
    pub primary_key: Option<String>,
    pub default_fields: Vec<String>,
    pub properties: Vec<McpPropertySummary>,
}

#[derive(Clone, Debug, Serialize)]
pub struct McpPropertySummary {
    pub name: String,
    pub caption: String,
    pub data_type: String,
    pub required: bool,
    pub read_only: bool,
    pub key: bool,
    pub relation: Option<McpRelationSummary>,
}

#[derive(Clone, Debug, Serialize)]
pub struct McpRelationSummary {
    pub kind: String,
    pub target_schema: String,
    pub target_type: String,
}

pub fn schema_summaries(model: &RuntimeModelMetadata) -> Vec<McpSchemaSummary> {
    model
        .schemas
        .iter()
        .map(|schema| schema_summary(model, schema))
        .collect()
}

pub fn entity_summaries(model: &RuntimeModelMetadata, schema_name: &str) -> Vec<McpEntitySummary> {
    model
        .entities_for_schema(schema_name)
        .iter()
        .map(|entity| entity_summary(entity))
        .collect()
}

pub fn entity_summary(entity: &RuntimeEntityMetadata) -> McpEntitySummary {
    McpEntitySummary {
        schema_name: entity.schema_name.clone(),
        type_name: entity.pascal_1.clone(),
        collection_name: entity.snake_n.clone(),
        caption: entity.caption_1.clone(),
        is_table: entity.is_table,
        facets: entity.facets.clone(),
        primary_key: entity.primary_key().map(|property| property.name.clone()),
        default_fields: default_field_names(entity),
        properties: entity.properties.iter().map(property_summary).collect(),
    }
}

pub fn default_field_names(entity: &RuntimeEntityMetadata) -> Vec<String> {
    let mut fields = entity
        .properties
        .iter()
        .filter(|property| property.is_native_storage())
        .filter(|property| property.is_key || property.is_caption)
        .map(|property| property.name.clone())
        .collect::<Vec<_>>();
    if fields.is_empty() {
        fields = entity
            .properties
            .iter()
            .filter(|property| property.is_native_storage())
            .take(8)
            .map(|property| property.name.clone())
            .collect();
    }
    fields
}

pub fn selection_json(entity: &RuntimeEntityMetadata, fields: Option<Vec<String>>) -> Value {
    crate::operation::selection_json(entity, fields)
}

pub fn resource_list(model: &RuntimeModelMetadata) -> Vec<Value> {
    let mut resources = vec![json!({
        "uri": "appfw://schemas",
        "name": "App Framework schemas",
        "description": "Model-driven schema catalog for this backend.",
        "mimeType": "application/json",
    })];

    for schema in &model.schemas {
        resources.push(json!({
            "uri": format!("appfw://schemas/{}", schema.name),
            "name": format!("Schema {}", schema.name),
            "description": schema.description,
            "mimeType": "application/json",
        }));
        resources.push(json!({
            "uri": format!("appfw://schemas/{}/entities", schema.name),
            "name": format!("Entities in {}", schema.name),
            "description": "Entity model summaries for this schema.",
            "mimeType": "application/json",
        }));
        for entity in model.entities_for_schema(&schema.name) {
            resources.push(json!({
                "uri": format!("appfw://schemas/{}/entities/{}", schema.name, entity.pascal_1),
                "name": format!("{}.{}", schema.name, entity.pascal_1),
                "description": entity.caption_1,
                "mimeType": "application/json",
            }));
        }
    }

    resources
}

pub fn operation_tool_list(operations: &[RuntimeOperation]) -> Vec<Value> {
    let operation_names = operation_name_counts(operations);
    operations
        .iter()
        .filter(|operation| operation.enabled)
        .map(|operation| operation_tool(operation, &operation_names))
        .collect()
}

pub fn operation_tool_name(
    operation: &RuntimeOperation,
    operation_names: &HashMap<&'static str, usize>,
) -> String {
    if operation_names.get(operation.name).copied().unwrap_or(0) <= 1 {
        operation.name.to_string()
    } else {
        format!(
            "appfw_{}_{}_{}",
            tool_name_part(operation.schema_name),
            tool_name_part(operation.type_name),
            operation.name
        )
    }
}

pub fn operation_tool_names(operations: &[RuntimeOperation]) -> Vec<(String, RuntimeOperation)> {
    let operation_names = operation_name_counts(operations);
    operations
        .iter()
        .filter(|operation| operation.enabled)
        .map(|operation| {
            (
                operation_tool_name(operation, &operation_names),
                operation.clone(),
            )
        })
        .collect()
}

pub fn resolve_operation_tool(
    operations: &[RuntimeOperation],
    tool_name: &str,
) -> Option<RuntimeOperation> {
    operation_tool_names(operations)
        .into_iter()
        .find(|(name, _)| name == tool_name)
        .map(|(_, operation)| operation)
}

pub fn support_tool_list() -> Vec<Value> {
    vec![
        json!({
            "name": "appfw_list_schemas",
            "description": "List schemas and entity counts from the App Framework model.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }
        }),
        json!({
            "name": "appfw_list_operations",
            "description": "List public handler operations reflected as MCP tools.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "schema_name": { "type": "string" },
                    "type_name": { "type": "string" },
                    "include_disabled": { "type": "boolean" }
                },
                "additionalProperties": false
            }
        }),
        json!({
            "name": "appfw_describe_operation",
            "description": "Describe one reflected public handler operation.",
            "inputSchema": {
                "type": "object",
                "required": ["name"],
                "properties": {
                    "name": { "type": "string" },
                    "schema_name": { "type": "string" },
                    "type_name": { "type": "string" },
                    "include_disabled": { "type": "boolean" }
                },
                "additionalProperties": false
            }
        }),
        json!({
            "name": "appfw_describe_entity",
            "description": "Describe one model-driven entity, including fields, relationships, facets, and defaults.",
            "inputSchema": {
                "type": "object",
                "required": ["schema_name", "type_name"],
                "properties": {
                    "schema_name": { "type": "string" },
                    "type_name": { "type": "string" }
                },
                "additionalProperties": false
            }
        }),
        json!({
            "name": "appfw_explain_access",
            "description": "Explain the current user's access decision and access filter for one entity/action.",
            "inputSchema": {
                "type": "object",
                "required": ["schema_name", "type_name", "action"],
                "properties": {
                    "schema_name": { "type": "string" },
                    "type_name": { "type": "string" },
                    "action": { "type": "string", "enum": ["read", "create", "update", "delete"] }
                },
                "additionalProperties": false
            }
        }),
    ]
}

pub fn prompt_list() -> Vec<Value> {
    vec![
        json!({
            "name": "appfw_query_entity",
            "description": "Help an agent query a model-driven entity safely.",
            "arguments": [
                { "name": "schema_name", "description": "Schema name, such as crm.", "required": true },
                { "name": "type_name", "description": "Entity type name, such as Account.", "required": true }
            ]
        }),
        json!({
            "name": "appfw_design_entity_change",
            "description": "Guide an agent through a safe model/config change.",
            "arguments": [
                { "name": "schema_name", "description": "Schema being changed.", "required": true },
                { "name": "goal", "description": "Desired business change.", "required": true }
            ]
        }),
        json!({
            "name": "appfw_explain_policy",
            "description": "Help an agent understand access policy behavior for one entity/action.",
            "arguments": [
                { "name": "schema_name", "description": "Schema name.", "required": true },
                { "name": "type_name", "description": "Entity type name.", "required": true },
                { "name": "action", "description": "read, create, update, or delete.", "required": true }
            ]
        }),
    ]
}

fn schema_summary(
    model: &RuntimeModelMetadata,
    schema: &RuntimeSchemaMetadata,
) -> McpSchemaSummary {
    McpSchemaSummary {
        name: schema.name.clone(),
        description: schema.description.clone(),
        data_source_name: schema.data_source_name.clone(),
        entity_count: model.entities_for_schema(&schema.name).len(),
    }
}

fn property_summary(property: &RuntimePropertyMetadata) -> McpPropertySummary {
    McpPropertySummary {
        name: property.name.clone(),
        caption: property.caption.clone(),
        data_type: format!("{:?}", property.data_type),
        required: property.is_required,
        read_only: property.is_read_only,
        key: property.is_key,
        relation: relation_summary(property),
    }
}

fn relation_summary(property: &RuntimePropertyMetadata) -> Option<McpRelationSummary> {
    match property.data_type {
        RuntimeDataType::NavToOne | RuntimeDataType::NavToMany => {
            let nav = property.nav_by_fk.as_ref()?;
            Some(McpRelationSummary {
                kind: format!("{:?}", property.data_type),
                target_schema: nav.resolved.schema_name.clone(),
                target_type: nav.resolved.type_name.clone(),
            })
        }
        RuntimeDataType::ManyToMany => {
            let many = property.many_to_many.as_ref()?;
            Some(McpRelationSummary {
                kind: "ManyToMany".to_string(),
                target_schema: many.target_schema.clone(),
                target_type: many.target_type.clone(),
            })
        }
        _ => None,
    }
}

fn operation_tool(
    operation: &RuntimeOperation,
    operation_names: &HashMap<&'static str, usize>,
) -> Value {
    json!({
        "name": operation_tool_name(operation, operation_names),
        "description": format!(
            "Call handler operation `{}` for {}.{} through the generated dispatcher, DataAccess, QueryIR, policy, tenant isolation, redaction, audit, and provider execution path.",
            operation.name,
            operation.schema_name,
            operation.type_name,
        ),
        "inputSchema": operation_input_schema(operation),
        "annotations": {
            "operation_name": operation.name,
            "schema_name": operation.schema_name,
            "type_name": operation.type_name,
            "kind": operation.kind,
            "returns": operation.returns,
            "selection_aware": operation.selection_aware,
        }
    })
}

fn operation_input_schema(operation: &RuntimeOperation) -> Value {
    let mut required = Vec::new();
    let mut properties = serde_json::Map::new();

    for arg in &operation.args {
        if arg.required {
            required.push(Value::String(arg.name.to_string()));
        }
        properties.insert(arg.name.to_string(), arg_schema(arg.name, arg.arg_type));
    }

    if operation.selection_aware {
        properties.insert("fields".to_string(), field_list_schema());
    }

    json!({
        "type": "object",
        "required": required,
        "properties": properties,
        "additionalProperties": false
    })
}

fn arg_schema(name: &str, arg_type: &str) -> Value {
    let bare_type = arg_type
        .strip_prefix("Option<")
        .and_then(|value| value.strip_suffix('>'))
        .unwrap_or(arg_type);
    match name {
        "filter" | "sort" | "having" => json!({ "type": "object" }),
        "group_by" => json!({ "type": "array", "items": { "type": ["string", "object"] } }),
        "metrics" => json!({ "type": "array", "items": { "type": "object" } }),
        "skip" => json!({ "type": "integer", "minimum": 0 }),
        "limit" => {
            json!({ "type": "integer", "minimum": 1, "maximum": RuntimePaginationPolicy::DEFAULT_MAX_PAGE_SIZE })
        }
        "after" => json!({ "type": "string" }),
        _ => match bare_type {
            "String" | "Uuid" => json!({ "type": "string" }),
            "bool" | "Boolean" => json!({ "type": "boolean" }),
            "f32" | "f64" | "Float" | "Float32" | "Float64" => json!({ "type": "number" }),
            "i16" | "i32" | "i64" | "u16" | "u32" | "u64" | "usize" | "Int" | "Int32" | "Int64" => {
                json!({ "type": "integer" })
            }
            "serde_json::Value" | "Value" => {
                json!({ "type": ["object", "array", "string", "number", "boolean", "null"] })
            }
            _ if bare_type.starts_with("Input") => json!({ "type": "object" }),
            _ => json!({ "type": ["object", "array", "string", "number", "boolean", "null"] }),
        },
    }
}

fn operation_name_counts(operations: &[RuntimeOperation]) -> HashMap<&'static str, usize> {
    let mut counts = HashMap::new();
    for operation in operations.iter().filter(|operation| operation.enabled) {
        *counts.entry(operation.name).or_insert(0) += 1;
    }
    counts
}

fn tool_name_part(value: &str) -> String {
    let mut out = String::new();
    for (index, ch) in value.chars().enumerate() {
        if ch.is_ascii_uppercase() && index > 0 {
            out.push('_');
        }
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

fn field_list_schema() -> Value {
    json!({
        "type": "array",
        "items": { "type": "string" },
        "description": "Native field names to return. Omit for model defaults."
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_metadata::{
        RuntimeDataSourceMetadata, RuntimeEntityRef, RuntimeNavByForeignKey,
        RuntimePropertyMetadata, RuntimeSchemaMetadata,
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
                facets: vec!["audited".to_string()],
                meta: None,
                standard_methods: Vec::new(),
                custom_methods: Vec::new(),
                properties: vec![
                    property("id", RuntimeDataType::Uuid, true, true, None),
                    property("name", RuntimeDataType::String, false, true, None),
                    property(
                        "industry",
                        RuntimeDataType::NavToOne,
                        false,
                        false,
                        Some(RuntimeEntityRef {
                            schema_name: "crm".to_string(),
                            type_name: "Industry".to_string(),
                        }),
                    ),
                ],
            }],
        )
    }

    fn property(
        name: &str,
        data_type: RuntimeDataType,
        is_key: bool,
        is_caption: bool,
        relation: Option<RuntimeEntityRef>,
    ) -> RuntimePropertyMetadata {
        RuntimePropertyMetadata {
            id: format!("prop-{name}"),
            name: name.to_string(),
            caption: name.to_string(),
            data_type,
            is_key,
            is_caption,
            is_required: is_key,
            is_read_only: false,
            is_concurrency_control: false,
            default_value: None,
            foreign_key: None,
            nav_by_fk: relation.map(|resolved| RuntimeNavByForeignKey {
                schema_name: resolved.schema_name.clone(),
                type_name: resolved.type_name.clone(),
                prop_name: "industry_id".to_string(),
                filter: None,
                resolved,
            }),
            many_to_many: None,
            nested_entity_type: None,
            enum_type_name: None,
            meta: None,
        }
    }

    #[test]
    fn catalog_is_derived_from_runtime_metadata() {
        let model = model();
        let account = model.entity("crm", "Account").expect("account exists");
        let summary = entity_summary(account);

        assert_eq!(summary.type_name, "Account");
        assert_eq!(summary.primary_key.as_deref(), Some("id"));
        assert!(summary.properties.iter().any(|prop| prop.name == "name"));
        assert!(summary.properties.iter().any(|prop| prop
            .relation
            .as_ref()
            .is_some_and(|rel| rel.target_type == "Industry")));
    }

    #[test]
    fn default_selection_uses_runtime_model_fields() {
        let model = model();
        let account = model.entity("crm", "Account").expect("account exists");

        let selection = selection_json(account, None);

        assert_eq!(selection["name"], "accounts");
        assert!(selection["selection_set"]
            .as_array()
            .expect("selection array")
            .iter()
            .any(|field| field["name"] == "id"));
    }

    #[test]
    fn duplicate_operation_names_receive_stable_tool_prefixes() {
        let operations = vec![
            RuntimeOperation::query("find_record", "crm", "Account", "Account", vec![], true),
            RuntimeOperation::query("find_record", "sales", "Account", "Account", vec![], true),
        ];

        let names = operation_tool_names(&operations)
            .into_iter()
            .map(|(name, _)| name)
            .collect::<Vec<_>>();

        assert_eq!(
            names,
            vec![
                "appfw_crm_account_find_record".to_string(),
                "appfw_sales_account_find_record".to_string()
            ]
        );
    }
}
