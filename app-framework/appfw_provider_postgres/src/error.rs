use appfw_runtime::{
    provider_error as runtime_provider_error, provider_keys::FrameworkProvider, DataStoreError,
    RuntimeError,
};

pub fn classify_postgres_error(error: &tokio_postgres::Error) -> Option<DataStoreError> {
    error.code().and_then(|code| {
        classify_postgres_error_code(
            code.code(),
            error.as_db_error().and_then(|db_error| db_error.column()),
        )
    })
}

pub fn classify_postgres_error_code(code: &str, field: Option<&str>) -> Option<DataStoreError> {
    runtime_provider_error::classify_postgres_code(code, field)
}

pub fn postgres_runtime_error(error: tokio_postgres::Error) -> RuntimeError {
    let message = error.to_string();
    if let Some(db_error) = error.as_db_error() {
        if let Some(kind) = classify_postgres_error_code(db_error.code().code(), db_error.column())
        {
            return RuntimeError::DataStore(runtime_provider_error::stable_provider_error(
                FrameworkProvider::Postgres,
                message,
                kind,
            ));
        }
    }
    RuntimeError::DataStore(runtime_provider_error::normalize_provider_error(
        FrameworkProvider::Postgres,
        message,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_native_postgres_codes_to_stable_runtime_errors() {
        assert_eq!(
            classify_postgres_error_code("23505", None)
                .expect("duplicate code")
                .to_string(),
            "duplicate key: record already exists"
        );
        assert_eq!(
            classify_postgres_error_code("23503", None)
                .expect("foreign key code")
                .to_string(),
            "foreign key: record references a missing related record"
        );
        assert_eq!(
            classify_postgres_error_code("23502", Some("name"))
                .expect("required field code")
                .to_string(),
            "required field: name"
        );
        assert!(classify_postgres_error_code("00000", None).is_none());
    }
}
