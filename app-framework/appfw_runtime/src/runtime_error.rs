use thiserror::Error;

use crate::{ConfigError, DataStoreError, MetadataError, QueryBuildError};

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("not authorized")]
    NotAuthorized,
    #[error("validation error: {0}")]
    Validation(String),
    #[error("access denied")]
    AccessDenied,
    #[error("invalid key or version")]
    InvalidKeyOrVersion,
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Metadata(#[from] MetadataError),
    #[error(transparent)]
    QueryBuild(#[from] QueryBuildError),
    #[error(transparent)]
    DataStore(#[from] DataStoreError),
    #[error("data access error: {0}")]
    DataAccess(String),
    #[error("internal server error")]
    Internal(String),
}

#[derive(Error, Debug)]
pub enum RuntimeAppError {
    #[error("not authorized")]
    NotAuthorized,
    #[error("validation error: {0}")]
    Validation(String),
    #[error("access denied")]
    AccessDenied,
    #[error("invalid key or version")]
    InvalidKeyOrVersion,
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Metadata(#[from] MetadataError),
    #[error(transparent)]
    QueryBuild(#[from] QueryBuildError),
    #[error(transparent)]
    DataStore(#[from] DataStoreError),
    #[error("data access error: {0}")]
    DataAccess(String),
    #[error("internal server error")]
    InternalServerError(#[from] anyhow::Error),
}

impl RuntimeError {
    pub fn category(&self) -> &'static str {
        match self {
            RuntimeError::NotAuthorized => "not_authorized",
            RuntimeError::Validation(_) => "validation",
            RuntimeError::AccessDenied => "access_denied",
            RuntimeError::InvalidKeyOrVersion => "invalid_key_or_version",
            RuntimeError::Config(_) => "config",
            RuntimeError::Metadata(_) => "metadata",
            RuntimeError::QueryBuild(_) => "query_build",
            RuntimeError::DataStore(_) => "data_store",
            RuntimeError::DataAccess(_) => "data_access",
            RuntimeError::Internal(_) => "internal",
        }
    }
}

impl From<RuntimeError> for RuntimeAppError {
    fn from(error: RuntimeError) -> Self {
        match error {
            RuntimeError::NotAuthorized => RuntimeAppError::NotAuthorized,
            RuntimeError::Validation(message) => RuntimeAppError::Validation(message),
            RuntimeError::AccessDenied => RuntimeAppError::AccessDenied,
            RuntimeError::InvalidKeyOrVersion => RuntimeAppError::InvalidKeyOrVersion,
            RuntimeError::Config(error) => RuntimeAppError::Config(error),
            RuntimeError::Metadata(error) => RuntimeAppError::Metadata(error),
            RuntimeError::QueryBuild(error) => RuntimeAppError::QueryBuild(error),
            RuntimeError::DataStore(error) => RuntimeAppError::DataStore(error),
            RuntimeError::DataAccess(message) => RuntimeAppError::DataAccess(message),
            RuntimeError::Internal(message) => {
                RuntimeAppError::InternalServerError(anyhow::anyhow!(message))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_error_categories_are_stable() {
        assert_eq!(RuntimeError::AccessDenied.category(), "access_denied");
        assert_eq!(
            RuntimeError::Validation("bad input".to_string()).category(),
            "validation"
        );
        assert_eq!(
            RuntimeError::DataStore(DataStoreError::DuplicateKey).category(),
            "data_store"
        );
    }

    #[test]
    fn runtime_app_error_maps_runtime_errors() {
        let error = RuntimeAppError::from(RuntimeError::AccessDenied);
        assert!(matches!(error, RuntimeAppError::AccessDenied));

        let error = RuntimeAppError::from(RuntimeError::DataAccess("provider failed".to_string()));
        assert!(
            matches!(error, RuntimeAppError::DataAccess(message) if message == "provider failed")
        );
    }
}
