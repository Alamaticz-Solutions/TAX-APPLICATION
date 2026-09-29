use appfw_runtime::{
    provider_error as runtime_provider_error, provider_keys::FrameworkProvider, DataStoreError,
    RuntimeError,
};

pub fn mongo_runtime_error(error: mongodb::error::Error) -> RuntimeError {
    let message = error.to_string();
    if let Some(kind) = classify_mongo_error(&error) {
        RuntimeError::DataStore(runtime_provider_error::stable_provider_error(
            FrameworkProvider::Mongo,
            message,
            kind,
        ))
    } else {
        RuntimeError::DataStore(runtime_provider_error::normalize_provider_error(
            FrameworkProvider::Mongo,
            message,
        ))
    }
}

pub fn classify_mongo_error(error: &mongodb::error::Error) -> Option<DataStoreError> {
    use mongodb::error::{ErrorKind, WriteFailure};

    let code = match error.kind.as_ref() {
        ErrorKind::Command(command) => Some(command.code),
        ErrorKind::InsertMany(insert_many) => insert_many
            .write_errors
            .as_ref()
            .and_then(|errors| errors.first())
            .map(|error| error.code),
        ErrorKind::Write(WriteFailure::WriteError(write_error)) => Some(write_error.code),
        ErrorKind::Write(WriteFailure::WriteConcernError(write_error)) => Some(write_error.code),
        _ => None,
    };

    code.and_then(classify_mongo_error_code)
}

pub fn classify_mongo_error_code(code: i32) -> Option<DataStoreError> {
    runtime_provider_error::classify_mongo_code(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_native_mongo_codes_to_stable_runtime_errors() {
        assert_eq!(
            classify_mongo_error_code(11000)
                .expect("duplicate code")
                .to_string(),
            "duplicate key: record already exists"
        );
        assert_eq!(
            classify_mongo_error_code(121)
                .expect("validation code")
                .to_string(),
            "required field: unknown"
        );
        assert!(classify_mongo_error_code(42).is_none());
    }
}
