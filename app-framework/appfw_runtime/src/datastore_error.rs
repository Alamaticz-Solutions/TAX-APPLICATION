use thiserror::Error;

#[derive(Error, Debug)]
pub enum DataStoreError {
    #[error("duplicate key: record already exists")]
    DuplicateKey,
    #[error("foreign key: record references a missing related record")]
    ForeignKeyViolation,
    #[error("required field: {field}")]
    MissingRequiredValue { field: String },
    #[error("data store operation failed")]
    OperationFailed,
}
