use serde_json::json;
use tracing::warn;

use crate::{
    extension::UserAuth,
    model_metadata::{RuntimeDataType, RuntimeEntityMetadata},
    PolicyAccess,
};

pub const TENANT_ID_FIELD: &str = "tenant_id";

pub fn apply(
    entity: &RuntimeEntityMetadata,
    user: &UserAuth,
    access: PolicyAccess,
) -> PolicyAccess {
    if !access.allow || !is_tenant_scoped(entity) {
        return access;
    }

    if user.tenant_id.trim().is_empty() {
        warn!(
          entity = %entity.pascal_1,
          "denying access because tenant-scoped entity was requested without a tenant id"
        );
        return PolicyAccess::deny();
    }

    access.and_filter(Some(
        json!({ TENANT_ID_FIELD: { "_eq": user.tenant_id.clone() } }),
    ))
}

pub fn is_tenant_scoped(entity: &RuntimeEntityMetadata) -> bool {
    entity.properties.iter().any(|property| {
        property.name == TENANT_ID_FIELD
            && !matches!(
                property.data_type,
                RuntimeDataType::NavToOne
                    | RuntimeDataType::NavToMany
                    | RuntimeDataType::ManyToMany
            )
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::model_metadata::{RuntimeDataType, RuntimePropertyMetadata};

    use super::*;

    fn user(tenant_id: &str) -> UserAuth {
        UserAuth::human(tenant_id, "casey", "UTC", vec![], vec![], "token")
    }

    fn entity(properties: Vec<RuntimePropertyMetadata>) -> RuntimeEntityMetadata {
        RuntimeEntityMetadata {
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
            properties,
        }
    }

    fn property(name: &str, data_type: RuntimeDataType) -> RuntimePropertyMetadata {
        RuntimePropertyMetadata {
            id: name.to_string(),
            name: name.to_string(),
            caption: name.to_string(),
            data_type,
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
            meta: None,
        }
    }

    #[test]
    fn detects_native_tenant_id_field() {
        assert!(!is_tenant_scoped(&entity(vec![property(
            "name",
            RuntimeDataType::String,
        )])));

        assert!(is_tenant_scoped(&entity(vec![property(
            TENANT_ID_FIELD,
            RuntimeDataType::String,
        )])));

        assert!(!is_tenant_scoped(&entity(vec![property(
            TENANT_ID_FIELD,
            RuntimeDataType::NavToOne,
        )])));
    }

    #[test]
    fn appends_tenant_filter_to_existing_policy_filter() {
        let access = apply(
            &entity(vec![property(TENANT_ID_FIELD, RuntimeDataType::String)]),
            &user("tenant-1"),
            PolicyAccess::allow_with_filter(json!({ "billing_state": { "_eq": "CA" } })),
        );

        assert_eq!(
            access.filter,
            Some(json!({
                "_and": [
                    { "billing_state": { "_eq": "CA" } },
                    { TENANT_ID_FIELD: { "_eq": "tenant-1" } }
                ]
            }))
        );
    }

    #[test]
    fn denies_tenant_scoped_access_without_tenant_id() {
        let access = apply(
            &entity(vec![property(TENANT_ID_FIELD, RuntimeDataType::String)]),
            &user(" "),
            PolicyAccess::allow_all(),
        );

        assert!(!access.allow);
        assert_eq!(access.filter, None);
    }
}
