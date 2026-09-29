use appfw_runtime::{
    extension::UserAuth, identifier::to_snake_case_lenient as to_snake_case,
    record_locator::RECORD_LOCATOR_FIELD, security::SecurityConfig, tenant_isolation, AccessAction,
    PolicyAccess,
};
use serde_json::json;
use std::{collections::HashMap, sync::Arc};
use tracing::{debug, warn};

use super::loader;
use crate::{
    product_api::runtime_entity_metadata,
    routes::app_error::{AppError, ConfigError, MetadataError},
    schemas::system::{
        Computed, DataSource, DataSourceEnvironment, DataType, EntityType, PropertyType, Schema,
    },
    services::authz::AuthzCache,
};

#[derive(Clone)]
pub struct AppConfig {
    sources: Arc<HashMap<String, DataSource>>,
    _schemas: Arc<HashMap<String, Schema>>, // TODO: use the schema object to provide metadata to graphiql UI
    types: Arc<HashMap<String, EntityType>>,
    access: Arc<HashMap<String, regorus::Engine>>,
    /// Database-driven RBAC (docs/architecture/rbac-design.md): the one reader of the
    /// app_user/role/permission tables. `evaluate_user_access` below consumes its output to
    /// build the Rego input; nothing else in this struct knows a role name.
    authz: Arc<AuthzCache>,
}

impl AppConfig {
    pub async fn init() -> Result<Self, AppError> {
        let (source_configs, schema_configs, entity_types, access_policies) =
            loader::read_config().await?;

        let sources = source_configs
            .iter()
            .map(|source_config| (source_config.name.clone(), source_config.clone()))
            .collect();

        let schemas = schema_configs
            .iter()
            .map(|schema| (schema.name.clone(), schema.clone()))
            .collect();
        debug!(schema_count = schema_configs.len(), "initialized schemas");

        let types = entity_types
            .iter()
            .map(|entity_type| {
                (
                    AppConfig::type_key(&entity_type.schema_name, &entity_type.pascal_1),
                    entity_type.clone(),
                )
            })
            .collect();

        let authz = AuthzCache::init(&sources).await?;

        Ok(AppConfig {
            sources: Arc::new(sources),
            _schemas: Arc::new(schemas),
            types: Arc::new(types),
            access: Arc::new(access_policies),
            authz,
        })
    }

    pub fn authz(&self) -> &Arc<AuthzCache> {
        &self.authz
    }

    #[tracing::instrument(
    skip(self, entity_type, user),
    fields(entity = %entity_type.pascal_1, action = %action.to_string(), has_user = user.is_some())
  )]
    #[allow(dead_code)]
    pub fn check_access(
        &self,
        entity_type: Arc<EntityType>,
        action: AccessAction,
        user: Option<UserAuth>,
    ) -> Result<PolicyAccess, AppError> {
        match user {
            Some(user) => {
                let access = self.evaluate_user_access(entity_type.clone(), action, &user)?;
                if access.allow {
                    Ok(access)
                } else {
                    Err(AppError::AccessDenied)
                }
            }
            None => {
                return Err(AppError::NotAuthorized);
            }
        }
    }

    #[tracing::instrument(
    skip(self, entity_type, user),
    fields(
      schema = %entity_type.schema_name,
      entity = %entity_type.pascal_1,
      action = %action.to_string(),
      user_name = %user.user_name
    )
  )]
    pub fn evaluate_user_access(
        &self,
        entity_type: Arc<EntityType>,
        action: AccessAction,
        user: &UserAuth,
    ) -> Result<PolicyAccess, AppError> {
        let mut user_json =
            serde_json::to_value(user.clone()).map_err(|e| ConfigError::PolicyEvaluation {
                policy_key: format!("{}.{}", entity_type.schema_name, entity_type.snake_1),
                message: e.to_string(),
            })?;
        let schema_name = &entity_type.schema_name;
        let entity_type_name = &entity_type.snake_1;

        // Database-driven RBAC (docs/architecture/rbac-design.md): add the caller's resolved
        // `id` and `grants` to the policy input. Every Rego policy in this product reads only
        // `input.user.grants` — never a role name — so this is the one place a role, looked up
        // from the RBAC tables via `self.authz`, is translated into what the policies consume.
        let principal = self.authz.resolve(&user.tenant_id, &user.user_name);
        if let Some(user_obj) = user_json.as_object_mut() {
            user_obj.insert(
                "id".to_string(),
                json!(principal.user_id.clone().unwrap_or_default()),
            );
            user_obj.insert("grants".to_string(), json!(principal.grants));
        }

        let input = json!({
          "schema_name": schema_name,
          "entity_type": entity_type_name,
          "action": action.to_string(),
          "user": user_json
        });
        // Where "user" is something like {
        //   "tenant_id": "180000",
        //   "user_name": "alex",
        //   "roles": ["operations_manager"],   <- legacy token roles; no policy reads this any more
        //   "id": "5c1c...",                    <- this user's app_user.id
        //   "grants": [ {"code": "routing_record.read", "resource": "tax_routing_record",
        //                "action": "read", "scope": "own", "owner_field": "assigned_user_id"}, ... ]
        // }

        let policy_key = format!("{}.{}", schema_name, entity_type_name);

        if SecurityConfig::bypass_policies_in_local() {
            warn!(
              policy_key = %policy_key,
              "allowing access because local development policy bypass is enabled"
            );

            return Ok(PolicyAccess::allow_all());
        }

        let engine = self.access.get(&policy_key);

        if engine.is_none() && SecurityConfig::allow_missing_policies_in_local() {
            warn!(
              policy_key = %policy_key,
              "allowing access without policy because local development override is enabled"
            );

            Ok(tenant_isolation::apply(
                &runtime_entity_metadata(&entity_type),
                user,
                PolicyAccess::allow_all(),
            ))
        } else if engine.is_none() {
            warn!(
              policy_key = %policy_key,
              "denying access because no policy was found"
            );

            Ok(PolicyAccess::deny())
        } else {
            let mut engine = engine
                .ok_or_else(|| ConfigError::PolicyEvaluation {
                    policy_key: policy_key.clone(),
                    message: "policy engine disappeared during lookup".to_string(),
                })?
                .to_owned();

            // println!("test_entity_type_access input: {:?}", input);
            engine.set_input(regorus::Value::from(input.clone()));

            let rule_path = format!("data.{}.access", policy_key);
            // println!("\nrule_path = {}\n", rule_path);

            let res = engine
                .eval_rule(rule_path)
                .map_err(|e| ConfigError::PolicyEvaluation {
                    policy_key: policy_key.clone(),
                    message: e.to_string(),
                })?;
            let access =
                serde_json::to_value(res).map_err(|e| ConfigError::InvalidPolicyResult {
                    policy_key: policy_key.clone(),
                    message: e.to_string(),
                })?;
            let access = PolicyAccess::from_policy_result(&policy_key, access)?;
            debug!(
              policy_key = %policy_key,
              allow = access.allow,
              has_filter = access.filter.is_some(),
              "evaluated access policy"
            );
            Ok(tenant_isolation::apply(
                &runtime_entity_metadata(&entity_type),
                user,
                access,
            ))
        }
    }

    pub fn get_nav_entity_type(
        &self,
        entity_type: Arc<EntityType>,
        prop: Arc<PropertyType>,
    ) -> Result<Arc<EntityType>, AppError> {
        let nav =
            prop.nav_by_fk_property
                .clone()
                .ok_or_else(|| MetadataError::MissingNavigation {
                    entity_type: entity_type.pascal_1.clone(),
                    property_name: prop.name.clone(),
                })?;

        // If entity_type has the foreign_key property, return type_name from foreign_key
        // Otherwise, return the type_name from nav_by_fk_property.

        if nav.type_name == entity_type.pascal_1 && nav.schema_name == entity_type.schema_name {
            self.get_entity_type(&nav.resolved.schema_name, &nav.resolved.type_name)
        } else {
            self.get_entity_type(&nav.schema_name, &nav.type_name)
        }
    }

    pub fn get_primary_key_name(&self, entity_type: Arc<EntityType>) -> Result<String, AppError> {
        let prop = entity_type
            .props
            .iter()
            .find(|p| p.is_key)
            .ok_or_else(|| MetadataError::MissingPrimaryKey {
                entity_type: entity_type.pascal_1.clone(),
            })?
            .to_owned();
        Ok(prop.name)
    }

    pub fn get_data_source_env(
        &self,
        data_source_name: &String,
    ) -> Result<Arc<DataSourceEnvironment>, AppError> {
        debug!(data_source = %data_source_name, "resolving data source environment");
        let data_source = self
            .sources
            .get(data_source_name)
            .ok_or_else(|| ConfigError::MissingDataSource(data_source_name.clone()))?
            .to_owned();
        // Loader detects current environment from .env file and updates array
        // to include current env settings as the only item in the vector.
        let environment = data_source
            .environments
            .first()
            .ok_or_else(|| ConfigError::MissingDataSourceEnvironment(data_source_name.clone()))?
            .to_owned();
        Ok(Arc::new(environment))
    }

    pub fn get_data_source_type(
        &self,
        data_source_name: &String,
    ) -> Result<crate::schemas::system::DataSourceType, AppError> {
        let data_source = self
            .sources
            .get(data_source_name)
            .ok_or_else(|| ConfigError::MissingDataSource(data_source_name.clone()))?;
        Ok(data_source.data_source_type)
    }

    pub fn get_schema_data_source_name(&self, schema_name: &str) -> Result<String, AppError> {
        self._schemas
            .get(schema_name)
            .map(|schema| schema.data_source_name.clone())
            .ok_or_else(|| ConfigError::Load(format!("schema not found: {}", schema_name)).into())
    }

    pub fn get_data_sources(&self) -> Vec<DataSource> {
        let mut data_sources = self.sources.values().cloned().collect::<Vec<_>>();
        data_sources.sort_by(|left, right| left.name.cmp(&right.name));
        data_sources
    }

    pub fn get_schemas(&self) -> Vec<Schema> {
        let mut schemas = self._schemas.values().cloned().collect::<Vec<_>>();
        schemas.sort_by(|left, right| left.name.cmp(&right.name));
        schemas
    }

    pub fn get_all_entity_types(&self) -> Vec<EntityType> {
        let mut entity_types = self.types.values().cloned().collect::<Vec<_>>();
        entity_types.sort_by(|left, right| {
            left.schema_name
                .cmp(&right.schema_name)
                .then(left.pascal_1.cmp(&right.pascal_1))
        });
        entity_types
    }

    pub fn get_entity_types(&self, schema_name: &String) -> Arc<Vec<EntityType>> {
        let res = self
            .types
            .values()
            .filter(|&et| et.schema_name.eq(schema_name))
            .map(|et| et.to_owned())
            .collect();
        Arc::new(res)
    }

    pub fn get_entity_type(
        &self,
        schema_name: &String,
        type_name: &String,
    ) -> Result<Arc<EntityType>, AppError> {
        let type_key = Self::type_key(schema_name, type_name);
        let entity_type = self
            .types
            .get(&type_key)
            .ok_or_else(|| ConfigError::MissingEntityType {
                schema_name: schema_name.clone(),
                type_name: type_name.clone(),
            })?
            .to_owned();
        Ok(Arc::new(entity_type))
    }

    pub fn get_entity_type_by_name(
        &self,
        schema_name: &String,
        type_name: &String,
    ) -> Result<Arc<EntityType>, AppError> {
        self.get_entity_type(schema_name, type_name)
    }

    pub fn type_key(schema_name: &String, type_pascal_1: &String) -> String {
        format!("{}.{}", schema_name, type_pascal_1).to_string()
    }

    pub fn get_prop(
        entity_type: Arc<EntityType>,
        input_name: &String,
    ) -> Result<Arc<PropertyType>, AppError> {
        let prop_name = to_snake_case(input_name);
        match entity_type.props.iter().find(|p| p.name.eq(&prop_name)) {
            Some(prop) => Ok(Arc::new(prop.clone())),
            None if prop_name == RECORD_LOCATOR_FIELD && entity_type.is_table => {
                Ok(Arc::new(native_record_locator_prop(&entity_type)))
            }
            None => Err(MetadataError::MissingProperty {
                entity_type: entity_type.pascal_1.clone(),
                property_name: input_name.clone(),
            }
            .into()),
        }
    }

    pub fn try_get_prop(
        entity_type: Arc<EntityType>,
        input_name: &String,
    ) -> Option<Arc<PropertyType>> {
        // println!("\n > app_config try_get_prop: entity_type {:?} input_name {:?}", entity_type.pascal_1, input_name);
        let prop_name = to_snake_case(input_name);
        match entity_type.props.iter().find(|p| p.name.eq(&prop_name)) {
            Some(prop) => Some(Arc::new(prop.to_owned())),
            None if prop_name == RECORD_LOCATOR_FIELD && entity_type.is_table => {
                Some(Arc::new(native_record_locator_prop(&entity_type)))
            }
            None => None,
        }
    }

    pub fn is_native_prop(prop: Arc<PropertyType>) -> bool {
        match prop.data_type {
            DataType::NavToOne | DataType::NavToMany | DataType::ManyToMany => false,
            _ => true, // Otherwise, persistable
        }
    }

    // pub fn has_facet(entity_type: Arc<EntityType>, facet: &String) -> bool
    // {
    //   match &entity_type.facets {
    //     Some(facets) => {
    //       if facets.contains(facet) {
    //         true
    //       } else {
    //         false
    //       }
    //     }
    //     None => {
    //       false
    //     }
    //   }
    // }
}

fn native_record_locator_prop(entity_type: &EntityType) -> PropertyType {
    PropertyType {
        id: format!("{}:{}", entity_type.id, RECORD_LOCATOR_FIELD),
        name: RECORD_LOCATOR_FIELD.to_string(),
        caption: "Record Locator".to_string(),
        is_key: false,
        is_caption: false,
        is_required: false,
        is_read_only: true,
        is_concurrency_control: false,
        data_type: DataType::String,
        computed: Computed::None,
        default_value: None,
        foreign_key: None,
        nav_by_fk_property: None,
        many_to_many_property: None,
        nested_entity_type: None,
        enum_type_name: None,
        meta: Some(json!({
            "native": true,
            "system": true,
            "route_identity": true
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_entity(is_table: bool) -> Arc<EntityType> {
        Arc::new(EntityType {
            id: "tax_routing.Record".to_string(),
            schema_name: "tax_routing".to_string(),
            schema_id: None,
            pascal_1: "Record".to_string(),
            pascal_n: "Records".to_string(),
            snake_1: "record".to_string(),
            snake_n: "records".to_string(),
            caption_1: "Record".to_string(),
            caption_n: "Records".to_string(),
            is_union: false,
            base_type: None,
            is_table,
            facets: None,
            meta: None,
            execution: None,
            standard_methods: None,
            custom_methods: None,
            props: vec![],
        })
    }

    #[test]
    fn table_entities_resolve_native_record_locator_property() {
        let prop = AppConfig::get_prop(test_entity(true), &RECORD_LOCATOR_FIELD.to_string())
            .expect("record locator property");

        assert_eq!(prop.name, RECORD_LOCATOR_FIELD);
        assert_eq!(prop.data_type, DataType::String);
        assert!(prop.is_read_only);
        assert!(!prop.is_key);
    }

    #[test]
    fn non_table_entities_do_not_resolve_native_record_locator_property() {
        assert!(
            AppConfig::try_get_prop(test_entity(false), &RECORD_LOCATOR_FIELD.to_string())
                .is_none()
        );
    }
}
