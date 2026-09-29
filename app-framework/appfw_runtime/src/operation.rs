use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Map, Value};
use thiserror::Error;

use crate::{extension::UserAuth, model_metadata::RuntimeEntityMetadata, RuntimeAppError};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RuntimeOperationRequest {
    pub name: String,
    pub schema_name: Option<String>,
    pub type_name: Option<String>,
    #[serde(default)]
    pub arguments: Map<String, Value>,
    pub fields: Option<Vec<String>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeOperation {
    pub name: &'static str,
    pub schema_name: &'static str,
    pub type_name: &'static str,
    pub kind: &'static str,
    pub returns: &'static str,
    pub args: Vec<RuntimeOperationArg>,
    pub selection_aware: bool,
    pub enabled: bool,
    pub disabled_reason: Option<&'static str>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeOperationArg {
    pub name: &'static str,
    pub arg_type: &'static str,
    pub required: bool,
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum RuntimeOperationResolutionError {
    #[error("operation `{name}` was not found")]
    NotFound { name: String },
    #[error("operation `{name}` is ambiguous; include schema_name and type_name")]
    Ambiguous { name: String },
}

pub trait RuntimeOperationCatalog: Sync {
    fn list_operations(&self, include_disabled: bool) -> Vec<RuntimeOperation>;

    fn resolve_operation(
        &self,
        request: &RuntimeOperationRequest,
    ) -> Result<RuntimeOperation, RuntimeOperationResolutionError> {
        resolve_operation(&self.list_operations(true), request)
    }
}

#[async_trait]
pub trait RuntimeOperationDispatcher: RuntimeOperationCatalog {
    async fn call_operation(
        &self,
        user: UserAuth,
        request: RuntimeOperationRequest,
    ) -> Result<Value, RuntimeAppError>;
}

pub fn selection_json(entity: &RuntimeEntityMetadata, fields: Option<Vec<String>>) -> Value {
    let fields = fields
        .filter(|fields| !fields.is_empty())
        .unwrap_or_else(|| default_field_names(entity));
    let selection_set = fields
        .into_iter()
        .map(|name| json!({ "name": name, "selection_set": [] }))
        .collect::<Vec<_>>();
    json!({
        "name": entity.snake_n,
        "selection_set": selection_set,
    })
}

fn default_field_names(entity: &RuntimeEntityMetadata) -> Vec<String> {
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

impl RuntimeOperation {
    pub fn query(
        name: &'static str,
        schema_name: &'static str,
        type_name: &'static str,
        returns: &'static str,
        args: Vec<RuntimeOperationArg>,
        selection_aware: bool,
    ) -> Self {
        Self {
            name,
            schema_name,
            type_name,
            kind: "query",
            returns,
            args,
            selection_aware,
            enabled: true,
            disabled_reason: None,
        }
    }

    pub fn disabled_mutation(
        name: &'static str,
        schema_name: &'static str,
        type_name: &'static str,
        returns: &'static str,
        args: Vec<RuntimeOperationArg>,
        selection_aware: bool,
    ) -> Self {
        Self {
            name,
            schema_name,
            type_name,
            kind: "mutation",
            returns,
            args,
            selection_aware,
            enabled: false,
            disabled_reason: Some("remote mutation operations require write-safety certification"),
        }
    }
}

impl RuntimeOperationArg {
    pub fn required(name: &'static str, arg_type: &'static str) -> Self {
        Self {
            name,
            arg_type,
            required: true,
        }
    }

    pub fn optional(name: &'static str, arg_type: &'static str) -> Self {
        Self {
            name,
            arg_type,
            required: false,
        }
    }
}

pub fn resolve_operation(
    operations: &[RuntimeOperation],
    request: &RuntimeOperationRequest,
) -> Result<RuntimeOperation, RuntimeOperationResolutionError> {
    let matches = operations
        .iter()
        .filter(|operation| operation.name == request.name)
        .filter(|operation| {
            request
                .schema_name
                .as_deref()
                .map(|schema_name| operation.schema_name == schema_name)
                .unwrap_or(true)
        })
        .filter(|operation| {
            request
                .type_name
                .as_deref()
                .map(|type_name| operation.type_name == type_name)
                .unwrap_or(true)
        })
        .cloned()
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [operation] => Ok(operation.clone()),
        [] => Err(RuntimeOperationResolutionError::NotFound {
            name: request.name.clone(),
        }),
        _ => Err(RuntimeOperationResolutionError::Ambiguous {
            name: request.name.clone(),
        }),
    }
}

pub fn arg<T: DeserializeOwned>(
    arguments: &Map<String, Value>,
    name: &str,
) -> Result<T, RuntimeAppError> {
    let value = arguments.get(name).cloned().unwrap_or(Value::Null);
    serde_json::from_value(value).map_err(|err| {
        RuntimeAppError::Validation(format!("invalid operation argument `{name}`: {err}"))
    })
}

pub fn to_json<T: Serialize>(value: T) -> Result<Value, RuntimeAppError> {
    serde_json::to_value(value).map_err(|err| {
        RuntimeAppError::DataAccess(format!("failed to serialize operation result: {err}"))
    })
}

pub fn handler_error(error: anyhow::Error) -> RuntimeAppError {
    RuntimeAppError::DataAccess(error.to_string())
}

pub fn redact_entity_payload(entity: &RuntimeEntityMetadata, value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|item| redact_entity_payload(entity, item))
                .collect(),
        ),
        Value::Object(mut obj) => {
            for value in obj.values_mut() {
                *value = redact_entity_payload(entity, value.take());
            }
            Value::Object(crate::record_audit::redact_record_for_external_response(
                entity, obj,
            ))
        }
        value => value,
    }
}

pub fn resolve_operation_app_error(
    operations: &[RuntimeOperation],
    request: &RuntimeOperationRequest,
) -> Result<RuntimeOperation, RuntimeAppError> {
    resolve_operation(operations, request)
        .map_err(|err| RuntimeAppError::Validation(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn operation(
        name: &'static str,
        schema_name: &'static str,
        type_name: &'static str,
    ) -> RuntimeOperation {
        RuntimeOperation::query(name, schema_name, type_name, "Value", vec![], false)
    }

    #[test]
    fn decodes_operation_arguments_with_validation_errors() {
        let arguments = Map::from_iter([("limit".to_string(), serde_json::json!(25))]);

        assert_eq!(arg::<i64>(&arguments, "limit").unwrap(), 25);
        assert!(matches!(
            arg::<i64>(&arguments, "missing"),
            Err(RuntimeAppError::Validation(_))
        ));
    }

    #[test]
    fn serializes_and_redacts_operation_payloads() {
        let entity = RuntimeEntityMetadata {
            id: "account".to_string(),
            schema_name: "crm".to_string(),
            schema_id: None,
            pascal_1: "Account".to_string(),
            pascal_n: "Accounts".to_string(),
            snake_1: "account".to_string(),
            snake_n: "accounts".to_string(),
            caption_1: "Account".to_string(),
            caption_n: "Accounts".to_string(),
            is_union: false,
            base_type: None,
            is_table: true,
            facets: vec![],
            meta: None,
            standard_methods: vec![],
            custom_methods: vec![],
            properties: vec![crate::model_metadata::RuntimePropertyMetadata {
                id: "secret".to_string(),
                name: "secret".to_string(),
                caption: "Secret".to_string(),
                data_type: crate::model_metadata::RuntimeDataType::String,
                is_key: false,
                is_caption: false,
                is_required: false,
                is_read_only: false,
                is_concurrency_control: false,
                default_value: None,
                foreign_key: None,
                nav_by_fk: None,
                many_to_many: None,
                nested_entity_type: None,
                enum_type_name: None,
                meta: Some(serde_json::json!({ "sensitive": true })),
            }],
        };
        let value = to_json(serde_json::json!({ "name": "Acme", "secret": "hidden" })).unwrap();

        let redacted = redact_entity_payload(&entity, value);

        assert_eq!(redacted["name"], "Acme");
        assert_eq!(redacted["secret"], serde_json::json!({ "_redacted": true }));
    }

    struct Catalog;

    impl RuntimeOperationCatalog for Catalog {
        fn list_operations(&self, include_disabled: bool) -> Vec<RuntimeOperation> {
            let operations = vec![
                operation("find_record", "crm", "Account"),
                operation("find_record", "sales", "Account"),
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

    #[test]
    fn resolves_unique_operation_by_name() {
        let operations = vec![operation("find_account", "crm", "Account")];
        let request = RuntimeOperationRequest {
            name: "find_account".to_string(),
            schema_name: None,
            type_name: None,
            arguments: Map::new(),
            fields: None,
        };

        let resolved = resolve_operation(&operations, &request).unwrap();

        assert_eq!(resolved.schema_name, "crm");
        assert_eq!(resolved.type_name, "Account");
    }

    #[test]
    fn requires_schema_or_type_for_ambiguous_names() {
        let operations = vec![
            operation("find_record", "crm", "Account"),
            operation("find_record", "sales", "Account"),
        ];
        let request = RuntimeOperationRequest {
            name: "find_record".to_string(),
            schema_name: None,
            type_name: None,
            arguments: Map::new(),
            fields: None,
        };

        assert_eq!(
            resolve_operation(&operations, &request),
            Err(RuntimeOperationResolutionError::Ambiguous {
                name: "find_record".to_string()
            })
        );
    }

    #[test]
    fn catalog_trait_filters_and_resolves_operations() {
        let catalog = Catalog;

        assert_eq!(catalog.list_operations(false).len(), 2);
        assert_eq!(catalog.list_operations(true).len(), 3);

        let request = RuntimeOperationRequest {
            name: "find_record".to_string(),
            schema_name: Some("sales".to_string()),
            type_name: Some("Account".to_string()),
            arguments: Map::new(),
            fields: None,
        };

        let resolved = catalog.resolve_operation(&request).unwrap();

        assert_eq!(resolved.schema_name, "sales");
    }
}
