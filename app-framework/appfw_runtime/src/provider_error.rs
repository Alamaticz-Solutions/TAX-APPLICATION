use tracing::error;

use crate::{
    observability::{current_request_context, redact_diagnostic_text},
    provider_keys::FrameworkProvider,
    DataStoreError,
};

pub fn normalize_provider_error(
    provider: FrameworkProvider,
    message: impl AsRef<str>,
) -> DataStoreError {
    let message = message.as_ref();
    log_provider_error(provider, message);
    classify_provider_message(message).unwrap_or(DataStoreError::OperationFailed)
}

pub fn stable_provider_error(
    provider: FrameworkProvider,
    message: impl AsRef<str>,
    kind: DataStoreError,
) -> DataStoreError {
    log_provider_error(provider, message.as_ref());
    kind
}

pub fn provider_error_log_message(message: &str) -> String {
    redact_diagnostic_text(message)
}

pub fn provider_error_label(provider: FrameworkProvider) -> &'static str {
    match provider {
        FrameworkProvider::Postgres => "PostgreSQL",
        FrameworkProvider::Mongo => "MongoDB",
        FrameworkProvider::Mssql => "MS SQL Server",
        FrameworkProvider::FabricSqlAnalytics => "Fabric SQL analytics",
        FrameworkProvider::Snowflake => "Snowflake",
        FrameworkProvider::Neo4j => "Neo4j",
        FrameworkProvider::ServiceNow => "ServiceNow",
        FrameworkProvider::Workday => "Workday",
        FrameworkProvider::Icims => "iCIMS",
        FrameworkProvider::Salesforce => "Salesforce",
        FrameworkProvider::Anaplan => "Anaplan",
        FrameworkProvider::OracleFinancials => "Oracle Financials",
        FrameworkProvider::AiSearch => "AI Search",
    }
}

pub fn classify_postgres_code(code: &str, field: Option<&str>) -> Option<DataStoreError> {
    match code {
        "23505" => Some(DataStoreError::DuplicateKey),
        "23503" => Some(DataStoreError::ForeignKeyViolation),
        "23502" => Some(DataStoreError::MissingRequiredValue {
            field: field.unwrap_or("unknown").to_string(),
        }),
        _ => None,
    }
}

pub fn classify_mssql_code(code: u32) -> Option<DataStoreError> {
    match code {
        // 2601: duplicate index; 2627: unique/primary-key constraint.
        2601 | 2627 => Some(DataStoreError::DuplicateKey),
        // 547: foreign-key/check constraint conflict.
        547 => Some(DataStoreError::ForeignKeyViolation),
        // 515: cannot insert NULL into non-nullable column.
        515 => Some(DataStoreError::MissingRequiredValue {
            field: "unknown".to_string(),
        }),
        _ => None,
    }
}

pub fn classify_mongo_code(code: i32) -> Option<DataStoreError> {
    match code {
        11000 | 11001 | 12582 => Some(DataStoreError::DuplicateKey),
        // Document validation failure. Mongo only tells us the schema rule
        // failed; provider messages stay server-side and callers receive a
        // stable class.
        121 => Some(DataStoreError::MissingRequiredValue {
            field: "unknown".to_string(),
        }),
        _ => None,
    }
}

pub fn classify_provider_message(message: &str) -> Option<DataStoreError> {
    let lower = message.to_ascii_lowercase();

    if is_duplicate_key(&lower) {
        return Some(DataStoreError::DuplicateKey);
    }
    if is_foreign_key_violation(&lower) {
        return Some(DataStoreError::ForeignKeyViolation);
    }
    if is_missing_required_value(&lower) {
        return Some(DataStoreError::MissingRequiredValue {
            field: extract_required_field(message).unwrap_or_else(|| "unknown".to_string()),
        });
    }

    None
}

fn log_provider_error(provider: FrameworkProvider, message: &str) {
    let request_context = current_request_context();
    let request_id = request_context
        .as_ref()
        .map(|context| context.request_id.as_str())
        .unwrap_or("");
    let correlation_id = request_context
        .as_ref()
        .map(|context| context.correlation_id.as_str())
        .unwrap_or("");
    let error = provider_error_log_message(message);

    error!(
        provider = provider_error_label(provider),
        error = %error,
        request_id,
        correlation_id,
        "provider data store error"
    );
}

fn is_duplicate_key(lower: &str) -> bool {
    lower.contains("23505")
        || lower.contains("e11000")
        || lower.contains("duplicate key")
        || lower.contains("unique constraint")
        || lower.contains("unique key")
        || lower.contains("primary key constraint")
        || lower.contains("violation of primary key")
        || lower.contains("violation of unique key")
        || lower.contains("cannot insert duplicate key")
        || lower.contains("duplicate value")
}

fn is_foreign_key_violation(lower: &str) -> bool {
    lower.contains("23503")
        || lower.contains("foreign key")
        || lower.contains("referential constraint")
        || lower.contains("references a missing")
        || lower.contains("parent key not found")
}

fn is_missing_required_value(lower: &str) -> bool {
    lower.contains("23502")
        || lower.contains("not-null")
        || lower.contains("not null")
        || lower.contains("cannot insert the value null")
        || lower.contains("null value in column")
        || lower.contains("field is required")
        || lower.contains("missing required")
}

fn extract_required_field(message: &str) -> Option<String> {
    for (prefix, suffix) in [
        ("column \"", "\""),
        ("column '", "'"),
        ("field \"", "\""),
        ("field '", "'"),
    ] {
        if let Some(rest) = message.split(prefix).nth(1) {
            if let Some(field) = rest.split(suffix).next() {
                let field = field.trim();
                if !field.is_empty() {
                    return Some(field.to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display(message: &str) -> String {
        normalize_provider_error(FrameworkProvider::Postgres, message).to_string()
    }

    #[test]
    fn maps_provider_duplicate_errors_to_stable_message() {
        let message = display(
            "ERROR: duplicate key value violates unique constraint \"account_pkey\" SQLSTATE 23505",
        );
        assert_eq!(message, "duplicate key: record already exists");
        assert!(!message.to_ascii_lowercase().contains("sqlstate"));
    }

    #[test]
    fn maps_provider_foreign_key_errors_to_stable_message() {
        let message = display(
            "The INSERT statement conflicted with the FOREIGN KEY constraint FK_account_industry",
        );
        assert_eq!(
            message,
            "foreign key: record references a missing related record"
        );
    }

    #[test]
    fn maps_provider_required_field_errors_to_stable_message() {
        let message = display("null value in column \"name\" violates not-null constraint");
        assert_eq!(message, "required field: name");
    }

    #[test]
    fn maps_unknown_provider_errors_to_non_leaky_message() {
        let message = display("ODBC Driver 18 for SQL Server: tcp socket reset by peer");
        assert_eq!(message, "data store operation failed");
        assert!(!message.to_ascii_lowercase().contains("odbc"));
    }

    #[test]
    fn provider_error_log_message_redacts_secrets() {
        let message = provider_error_log_message(
            "connection failed password=hunter2 Authorization: Bearer jwt.value token=plain",
        );

        assert!(message.contains("[REDACTED]"));
        assert!(!message.contains("hunter2"));
        assert!(!message.contains("jwt.value"));
        assert!(!message.contains("plain"));
    }

    #[test]
    fn maps_native_postgres_codes_before_message_heuristics() {
        let message = classify_postgres_code("23502", Some("name"))
            .expect("postgres code mapping")
            .to_string();

        assert_eq!(message, "required field: name");
    }

    #[test]
    fn maps_native_mssql_codes_before_message_heuristics() {
        let duplicate = classify_mssql_code(2627)
            .expect("mssql duplicate mapping")
            .to_string();
        let foreign_key = classify_mssql_code(547)
            .expect("mssql fk mapping")
            .to_string();

        assert_eq!(duplicate, "duplicate key: record already exists");
        assert_eq!(
            foreign_key,
            "foreign key: record references a missing related record"
        );
    }

    #[test]
    fn maps_native_mongo_codes_before_message_heuristics() {
        let duplicate = classify_mongo_code(11000)
            .expect("mongo duplicate mapping")
            .to_string();
        let validation = classify_mongo_code(121)
            .expect("mongo validation mapping")
            .to_string();

        assert_eq!(duplicate, "duplicate key: record already exists");
        assert_eq!(validation, "required field: unknown");
    }

    #[test]
    fn maps_snowflake_native_messages_to_stable_errors() {
        let duplicate = normalize_provider_error(
            FrameworkProvider::Snowflake,
            "Snowflake SQL API returned 422: Duplicate value violates unique constraint",
        )
        .to_string();
        let foreign_key = normalize_provider_error(
            FrameworkProvider::Snowflake,
            "Snowflake SQL API returned 422: foreign key constraint failed",
        )
        .to_string();
        let required = normalize_provider_error(
            FrameworkProvider::Snowflake,
            "Snowflake SQL API returned 422: NULL value in column 'name'",
        )
        .to_string();

        assert_eq!(duplicate, "duplicate key: record already exists");
        assert_eq!(
            foreign_key,
            "foreign key: record references a missing related record"
        );
        assert_eq!(required, "required field: name");
    }

    #[test]
    fn provider_labels_are_operator_readable() {
        assert_eq!(
            provider_error_label(FrameworkProvider::Postgres),
            "PostgreSQL"
        );
        assert_eq!(provider_error_label(FrameworkProvider::Mongo), "MongoDB");
        assert_eq!(
            provider_error_label(FrameworkProvider::Mssql),
            "MS SQL Server"
        );
        assert_eq!(
            provider_error_label(FrameworkProvider::Snowflake),
            "Snowflake"
        );
        assert_eq!(provider_error_label(FrameworkProvider::Anaplan), "Anaplan");
        assert_eq!(
            provider_error_label(FrameworkProvider::OracleFinancials),
            "Oracle Financials"
        );
    }
}
