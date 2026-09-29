use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("failed to load runtime configuration: {0}")]
    Load(String),
    #[error("failed to read configuration file {path}: {message}")]
    Io { path: String, message: String },
    #[error("failed to parse configuration file {path}: {message}")]
    Parse { path: String, message: String },
    #[error("required environment variable is missing: {name}")]
    MissingEnvVar { name: String },
    #[error(
        "environment '{environment_name}' is not configured for data source '{data_source_name}'"
    )]
    MissingConfiguredEnvironment {
        data_source_name: String,
        environment_name: String,
    },
    #[error("invalid configuration path {path}: {message}")]
    InvalidPath { path: String, message: String },
    #[error("failed to load policy {path}: {message}")]
    PolicyLoad { path: String, message: String },
    #[error("data source not found: {0}")]
    MissingDataSource(String),
    #[error("data source environment not found for: {0}")]
    MissingDataSourceEnvironment(String),
    #[error("entity type not found: {schema_name}.{type_name}")]
    MissingEntityType {
        schema_name: String,
        type_name: String,
    },
    #[error("policy evaluation failed for {policy_key}: {message}")]
    PolicyEvaluation { policy_key: String, message: String },
    #[error("invalid policy result for {policy_key}: {message}")]
    InvalidPolicyResult { policy_key: String, message: String },
}
