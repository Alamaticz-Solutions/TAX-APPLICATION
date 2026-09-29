use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{provider_keys::FrameworkProvider, MetadataError};

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeModelMetadata {
    pub data_sources: Vec<RuntimeDataSourceMetadata>,
    pub schemas: Vec<RuntimeSchemaMetadata>,
    pub entities: Vec<RuntimeEntityMetadata>,
    data_source_by_name: HashMap<String, usize>,
    schema_by_name: HashMap<String, usize>,
    entity_by_key: HashMap<String, usize>,
}

impl RuntimeModelMetadata {
    pub fn new(
        data_sources: Vec<RuntimeDataSourceMetadata>,
        schemas: Vec<RuntimeSchemaMetadata>,
        entities: Vec<RuntimeEntityMetadata>,
    ) -> Self {
        let data_source_by_name = data_sources
            .iter()
            .enumerate()
            .map(|(index, data_source)| (data_source.name.clone(), index))
            .collect();
        let schema_by_name = schemas
            .iter()
            .enumerate()
            .map(|(index, schema)| (schema.name.clone(), index))
            .collect();
        let entity_by_key = entities
            .iter()
            .enumerate()
            .map(|(index, entity)| (entity.key(), index))
            .collect();

        Self {
            data_sources,
            schemas,
            entities,
            data_source_by_name,
            schema_by_name,
            entity_by_key,
        }
    }

    pub fn data_source(&self, name: &str) -> Option<&RuntimeDataSourceMetadata> {
        self.data_source_by_name
            .get(name)
            .map(|index| &self.data_sources[*index])
    }

    pub fn schema(&self, name: &str) -> Option<&RuntimeSchemaMetadata> {
        self.schema_by_name
            .get(name)
            .map(|index| &self.schemas[*index])
    }

    pub fn entity(
        &self,
        schema_name: &str,
        type_name: &str,
    ) -> Result<&RuntimeEntityMetadata, MetadataError> {
        let key = entity_key(schema_name, type_name);
        self.entity_by_key
            .get(&key)
            .map(|index| &self.entities[*index])
            .ok_or_else(|| MetadataError::MissingEntityType {
                schema_name: schema_name.to_string(),
                type_name: type_name.to_string(),
            })
    }

    pub fn entities_for_schema(&self, schema_name: &str) -> Vec<&RuntimeEntityMetadata> {
        self.entities
            .iter()
            .filter(|entity| entity.schema_name == schema_name)
            .collect()
    }

    pub fn property<'a>(
        &self,
        entity: &'a RuntimeEntityMetadata,
        property_name: &str,
    ) -> Result<&'a RuntimePropertyMetadata, MetadataError> {
        entity
            .property(property_name)
            .ok_or_else(|| MetadataError::MissingProperty {
                entity_type: entity.pascal_1.clone(),
                property_name: property_name.to_string(),
            })
    }

    pub fn primary_key_name<'a>(
        &self,
        entity: &'a RuntimeEntityMetadata,
    ) -> Result<&'a str, MetadataError> {
        entity
            .primary_key()
            .map(|property| property.name.as_str())
            .ok_or_else(|| MetadataError::MissingPrimaryKey {
                entity_type: entity.pascal_1.clone(),
            })
    }

    pub fn relationship_target_ref(
        &self,
        entity: &RuntimeEntityMetadata,
        property: &RuntimePropertyMetadata,
    ) -> Result<RuntimeEntityRef, MetadataError> {
        match property.data_type {
            RuntimeDataType::NavToOne | RuntimeDataType::NavToMany => {
                let nav = property.nav_by_fk.as_ref().ok_or_else(|| {
                    MetadataError::MissingNavigation {
                        entity_type: entity.pascal_1.clone(),
                        property_name: property.name.clone(),
                    }
                })?;

                if nav.type_name == entity.pascal_1 && nav.schema_name == entity.schema_name {
                    Ok(nav.resolved.clone())
                } else {
                    Ok(RuntimeEntityRef {
                        schema_name: nav.schema_name.clone(),
                        type_name: nav.type_name.clone(),
                    })
                }
            }
            RuntimeDataType::ManyToMany => {
                let many_to_many = property.many_to_many.as_ref().ok_or_else(|| {
                    MetadataError::MissingManyToMany {
                        entity_type: entity.pascal_1.clone(),
                        property_name: property.name.clone(),
                    }
                })?;
                Ok(RuntimeEntityRef {
                    schema_name: many_to_many.target_schema.clone(),
                    type_name: many_to_many.target_type.clone(),
                })
            }
            _ => Err(MetadataError::MissingNavigation {
                entity_type: entity.pascal_1.clone(),
                property_name: property.name.clone(),
            }),
        }
    }

    pub fn relationship_target(
        &self,
        entity: &RuntimeEntityMetadata,
        property: &RuntimePropertyMetadata,
    ) -> Result<&RuntimeEntityMetadata, MetadataError> {
        let target = self.relationship_target_ref(entity, property)?;
        self.entity(&target.schema_name, &target.type_name)
    }
}

pub fn entity_key(schema_name: &str, type_name: &str) -> String {
    format!("{}.{}", schema_name, type_name)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeDataSourceMetadata {
    pub name: String,
    pub description: Option<String>,
    pub provider: FrameworkProvider,
    pub is_system_schema_host: bool,
    pub environments: Vec<RuntimeDataSourceEnvironment>,
}

impl RuntimeDataSourceMetadata {
    pub fn provider_key(&self) -> &'static str {
        self.provider.key()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeDataSourceEnvironment {
    pub name: String,
    pub db_host: String,
    pub db_name: String,
    pub db_port: String,
    pub security_profile: String,
    pub tls_mode: String,
    pub service_account_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSchemaMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub data_source_name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RuntimeEntityMetadata {
    pub id: String,
    pub schema_name: String,
    pub schema_id: Option<String>,
    pub pascal_1: String,
    pub pascal_n: String,
    pub snake_1: String,
    pub snake_n: String,
    pub caption_1: String,
    pub caption_n: String,
    pub is_union: bool,
    pub base_type: Option<String>,
    pub is_table: bool,
    pub facets: Vec<String>,
    pub meta: Option<Value>,
    pub standard_methods: Vec<String>,
    pub custom_methods: Vec<RuntimeCustomMethodMetadata>,
    pub properties: Vec<RuntimePropertyMetadata>,
}

impl RuntimeEntityMetadata {
    pub fn key(&self) -> String {
        format!("{}.{}", self.schema_name, self.pascal_1)
    }

    pub fn property(&self, name: &str) -> Option<&RuntimePropertyMetadata> {
        self.properties
            .iter()
            .find(|property| property.name == name)
    }

    pub fn primary_key(&self) -> Option<&RuntimePropertyMetadata> {
        self.properties.iter().find(|property| property.is_key)
    }

    pub fn is_audited(&self) -> bool {
        self.facets.iter().any(|facet| facet == "audited")
    }

    pub fn is_soft_deleted(&self) -> bool {
        self.facets.iter().any(|facet| facet == "soft_delete")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeCustomMethodMetadata {
    pub name: String,
    pub kind: String,
    pub args: Vec<RuntimeMethodArgMetadata>,
    pub return_type: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeMethodArgMetadata {
    pub name: String,
    pub arg_type: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RuntimePropertyMetadata {
    pub id: String,
    pub name: String,
    pub caption: String,
    pub data_type: RuntimeDataType,
    pub is_key: bool,
    pub is_caption: bool,
    pub is_required: bool,
    pub is_read_only: bool,
    pub is_concurrency_control: bool,
    pub default_value: Option<Value>,
    pub foreign_key: Option<RuntimeEntityRef>,
    pub nav_by_fk: Option<RuntimeNavByForeignKey>,
    pub many_to_many: Option<RuntimeManyToMany>,
    pub nested_entity_type: Option<RuntimeEntityRef>,
    pub enum_type_name: Option<String>,
    pub meta: Option<Value>,
}

impl RuntimePropertyMetadata {
    pub fn is_relationship(&self) -> bool {
        matches!(
            self.data_type,
            RuntimeDataType::NavToOne | RuntimeDataType::NavToMany | RuntimeDataType::ManyToMany
        )
    }

    pub fn is_native_storage(&self) -> bool {
        !self.is_relationship()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RuntimeNavByForeignKey {
    pub schema_name: String,
    pub type_name: String,
    pub prop_name: String,
    pub filter: Option<Value>,
    pub resolved: RuntimeEntityRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeManyToMany {
    pub junction_table: String,
    pub junction_schema: Option<String>,
    pub local_key: String,
    pub foreign_key: String,
    pub target_schema: String,
    pub target_type: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeEntityRef {
    pub schema_name: String,
    pub type_name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuntimeDataType {
    Uuid,
    UuidArray,
    ObjectId,
    ObjectIdArray,
    Boolean,
    String,
    StringArray,
    Date,
    DateTime,
    Time,
    Int8,
    Int8Array,
    Int16,
    Int16Array,
    Int32,
    Int32Array,
    Int64,
    Int64Array,
    Float32,
    Float64,
    Enum,
    EnumArray,
    Object,
    ObjectArray,
    Json,
    JsonArray,
    NavToOne,
    NavToMany,
    ManyToMany,
}

impl RuntimeDataType {
    pub fn key(self) -> &'static str {
        match self {
            RuntimeDataType::Uuid => "uuid",
            RuntimeDataType::UuidArray => "uuid_array",
            RuntimeDataType::ObjectId => "object_id",
            RuntimeDataType::ObjectIdArray => "object_id_array",
            RuntimeDataType::Boolean => "boolean",
            RuntimeDataType::String => "string",
            RuntimeDataType::StringArray => "string_array",
            RuntimeDataType::Date => "date",
            RuntimeDataType::DateTime => "date_time",
            RuntimeDataType::Time => "time",
            RuntimeDataType::Int8 => "int8",
            RuntimeDataType::Int8Array => "int8_array",
            RuntimeDataType::Int16 => "int16",
            RuntimeDataType::Int16Array => "int16_array",
            RuntimeDataType::Int32 => "int32",
            RuntimeDataType::Int32Array => "int32_array",
            RuntimeDataType::Int64 => "int64",
            RuntimeDataType::Int64Array => "int64_array",
            RuntimeDataType::Float32 => "float32",
            RuntimeDataType::Float64 => "float64",
            RuntimeDataType::Enum => "enum",
            RuntimeDataType::EnumArray => "enum_array",
            RuntimeDataType::Object => "object",
            RuntimeDataType::ObjectArray => "object_array",
            RuntimeDataType::Json => "json",
            RuntimeDataType::JsonArray => "json_array",
            RuntimeDataType::NavToOne => "nav_to_one",
            RuntimeDataType::NavToMany => "nav_to_many",
            RuntimeDataType::ManyToMany => "many_to_many",
        }
    }

    pub fn is_array(self) -> bool {
        matches!(
            self,
            RuntimeDataType::UuidArray
                | RuntimeDataType::ObjectIdArray
                | RuntimeDataType::StringArray
                | RuntimeDataType::Int8Array
                | RuntimeDataType::Int16Array
                | RuntimeDataType::Int32Array
                | RuntimeDataType::Int64Array
                | RuntimeDataType::EnumArray
                | RuntimeDataType::ObjectArray
                | RuntimeDataType::JsonArray
        )
    }

    pub fn is_relationship(self) -> bool {
        matches!(
            self,
            RuntimeDataType::NavToOne | RuntimeDataType::NavToMany | RuntimeDataType::ManyToMany
        )
    }

    pub fn is_many_to_many(self) -> bool {
        self == RuntimeDataType::ManyToMany
    }

    pub fn is_numeric(self) -> bool {
        matches!(
            self,
            RuntimeDataType::Int8
                | RuntimeDataType::Int16
                | RuntimeDataType::Int32
                | RuntimeDataType::Int64
                | RuntimeDataType::Float32
                | RuntimeDataType::Float64
        )
    }

    pub fn is_scalar_aggregate(self) -> bool {
        matches!(
            self,
            RuntimeDataType::Boolean
                | RuntimeDataType::String
                | RuntimeDataType::Uuid
                | RuntimeDataType::ObjectId
                | RuntimeDataType::Enum
                | RuntimeDataType::Date
                | RuntimeDataType::Time
                | RuntimeDataType::DateTime
                | RuntimeDataType::Int8
                | RuntimeDataType::Int16
                | RuntimeDataType::Int32
                | RuntimeDataType::Int64
                | RuntimeDataType::Float32
                | RuntimeDataType::Float64
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime_property(name: &str, data_type: RuntimeDataType) -> RuntimePropertyMetadata {
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

    fn runtime_entity(
        schema_name: &str,
        type_name: &str,
        properties: Vec<RuntimePropertyMetadata>,
    ) -> RuntimeEntityMetadata {
        RuntimeEntityMetadata {
            id: type_name.to_ascii_lowercase(),
            schema_name: schema_name.to_string(),
            schema_id: None,
            pascal_1: type_name.to_string(),
            pascal_n: format!("{type_name}s"),
            snake_1: type_name.to_ascii_lowercase(),
            snake_n: format!("{}s", type_name.to_ascii_lowercase()),
            caption_1: type_name.to_string(),
            caption_n: format!("{type_name}s"),
            is_union: false,
            base_type: None,
            is_table: true,
            facets: Vec::new(),
            meta: None,
            standard_methods: Vec::new(),
            custom_methods: Vec::new(),
            properties,
        }
    }

    #[test]
    fn entity_metadata_exposes_lookup_helpers() {
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
            facets: vec!["audited".to_string()],
            meta: None,
            standard_methods: vec!["FindById".to_string()],
            custom_methods: Vec::new(),
            properties: vec![
                RuntimePropertyMetadata {
                    id: "account_id".to_string(),
                    name: "account_id".to_string(),
                    caption: "Account ID".to_string(),
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
                },
                RuntimePropertyMetadata {
                    id: "contacts".to_string(),
                    name: "contacts".to_string(),
                    caption: "Contacts".to_string(),
                    data_type: RuntimeDataType::NavToMany,
                    is_key: false,
                    is_caption: false,
                    is_required: false,
                    is_read_only: true,
                    is_concurrency_control: false,
                    default_value: None,
                    foreign_key: None,
                    nav_by_fk: None,
                    many_to_many: None,
                    nested_entity_type: None,
                    enum_type_name: None,
                    meta: None,
                },
            ],
        };

        assert_eq!(entity.key(), "crm.Account");
        assert_eq!(
            entity.primary_key().map(|property| property.name.as_str()),
            Some("account_id")
        );
        assert!(entity.is_audited());
        assert!(entity.property("contacts").unwrap().is_relationship());
        assert!(!entity.property("account_id").unwrap().is_relationship());
    }

    #[test]
    fn model_metadata_resolves_relationship_targets() {
        let mut contact_id = runtime_property("id", RuntimeDataType::Uuid);
        contact_id.is_key = true;

        let mut contacts = runtime_property("contacts", RuntimeDataType::NavToMany);
        contacts.nav_by_fk = Some(RuntimeNavByForeignKey {
            schema_name: "crm".to_string(),
            type_name: "Account".to_string(),
            prop_name: "account_id".to_string(),
            filter: None,
            resolved: RuntimeEntityRef {
                schema_name: "crm".to_string(),
                type_name: "Contact".to_string(),
            },
        });

        let mut opportunities = runtime_property("opportunities", RuntimeDataType::ManyToMany);
        opportunities.many_to_many = Some(RuntimeManyToMany {
            junction_table: "account_opportunity".to_string(),
            junction_schema: None,
            local_key: "account_id".to_string(),
            foreign_key: "opportunity_id".to_string(),
            target_schema: "crm".to_string(),
            target_type: "Opportunity".to_string(),
        });

        let account = runtime_entity(
            "crm",
            "Account",
            vec![
                runtime_property("id", RuntimeDataType::Uuid),
                contacts,
                opportunities,
            ],
        );
        let contact = runtime_entity("crm", "Contact", vec![contact_id]);
        let opportunity = runtime_entity(
            "crm",
            "Opportunity",
            vec![runtime_property("id", RuntimeDataType::Uuid)],
        );
        let model =
            RuntimeModelMetadata::new(Vec::new(), Vec::new(), vec![account, contact, opportunity]);

        let account = model.entity("crm", "Account").expect("account");
        let contacts = account.property("contacts").expect("contacts");
        let contact_target = model
            .relationship_target(account, contacts)
            .expect("contacts target");
        assert_eq!(contact_target.pascal_1, "Contact");

        let opportunities = account.property("opportunities").expect("opportunities");
        let opportunity_target = model
            .relationship_target(account, opportunities)
            .expect("opportunities target");
        assert_eq!(opportunity_target.pascal_1, "Opportunity");
    }

    #[test]
    fn data_type_helpers_cover_storage_shape() {
        assert_eq!(RuntimeDataType::Uuid.key(), "uuid");
        assert!(RuntimeDataType::StringArray.is_array());
        assert!(!RuntimeDataType::ManyToMany.is_array());
        assert!(RuntimeDataType::ManyToMany.is_relationship());
        assert!(RuntimeDataType::ManyToMany.is_many_to_many());
        assert!(RuntimeDataType::Int64.is_numeric());
        assert!(RuntimeDataType::DateTime.is_scalar_aggregate());
        assert!(!RuntimeDataType::NavToOne.is_scalar_aggregate());
    }
}
