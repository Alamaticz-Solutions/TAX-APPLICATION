use inflector::cases::tablecase::{is_table_case, to_table_case};
use serde_json::Value;

use crate::schemas::system::{EntityType, PropertyType};

pub(crate) fn physical_table_name(name: &str) -> String {
    if is_table_case(name) {
        name.to_string()
    } else {
        to_table_case(name)
    }
}

pub(crate) fn entity_schema(entity_type: &EntityType) -> String {
    storage_string(&entity_type.meta, "/storage/physical_schema")
        .unwrap_or_else(|| entity_type.schema_name.clone())
}

pub(crate) fn entity_table(entity_type: &EntityType) -> String {
    storage_string(&entity_type.meta, "/storage/physical_name")
        .unwrap_or_else(|| entity_type.snake_n.clone())
}

pub(crate) fn property_column(prop: &PropertyType) -> String {
    storage_string(&prop.meta, "/storage/physical_name").unwrap_or_else(|| prop.name.clone())
}

pub(crate) fn table_ref(entity_type: &EntityType) -> String {
    format!(
        "{}.{}",
        quote_ident(&entity_schema(entity_type)),
        quote_ident(&entity_table(entity_type))
    )
}

pub(crate) fn quote_ident(name: &str) -> String {
    format!("[{}]", name.replace(']', "]]"))
}

fn storage_string(meta: &Option<Value>, pointer: &str) -> Option<String> {
    meta.as_ref()?
        .pointer(pointer)?
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn entity(snake_n: &str, meta: Option<Value>) -> EntityType {
        EntityType {
            id: "test.AccountAudit".to_string(),
            schema_name: "crm".to_string(),
            schema_id: None,
            pascal_1: "AccountAudit".to_string(),
            pascal_n: "AccountAudits".to_string(),
            snake_1: "account_audit".to_string(),
            snake_n: snake_n.to_string(),
            caption_1: "Account Audit".to_string(),
            caption_n: "Account Audits".to_string(),
            is_union: false,
            base_type: None,
            is_table: true,
            facets: None,
            meta,
            execution: None,
            standard_methods: None,
            custom_methods: None,
            props: Vec::new(),
        }
    }

    #[test]
    fn entity_table_uses_generated_snake_name_without_reinflecting() {
        assert_eq!(entity_table(&entity("accounts_audit", None)), "accounts_audit");
    }

    #[test]
    fn entity_table_honors_storage_override() {
        let meta = json!({"storage": {"physical_name": "account_audit_events"}});

        assert_eq!(
            entity_table(&entity("accounts_audit", Some(meta))),
            "account_audit_events"
        );
    }
}
