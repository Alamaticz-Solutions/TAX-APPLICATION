use thiserror::Error;

#[derive(Error, Debug)]
pub enum MetadataError {
    #[error("entity type not found: {schema_name}.{type_name}")]
    MissingEntityType {
        schema_name: String,
        type_name: String,
    },
    #[error("property not found: {entity_type}.{property_name}")]
    MissingProperty {
        entity_type: String,
        property_name: String,
    },
    #[error("primary key property not found for entity type: {entity_type}")]
    MissingPrimaryKey { entity_type: String },
    #[error("navigation metadata missing for property: {entity_type}.{property_name}")]
    MissingNavigation {
        entity_type: String,
        property_name: String,
    },
    #[error("many-to-many metadata missing for property: {entity_type}.{property_name}")]
    MissingManyToMany {
        entity_type: String,
        property_name: String,
    },
    #[error("invalid selection shape: {0}")]
    InvalidSelection(String),
    #[error("invalid computed metadata for property {property_name}: {message}")]
    InvalidComputedMetadata {
        property_name: String,
        message: String,
    },
}
