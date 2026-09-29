use appfw_runtime::RuntimeError;

use crate::{SnowflakeStatement, SnowflakeStatementBuilder};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnowflakeStoredProcedureCall {
    schema: Option<String>,
    name: String,
    argument_expressions: Vec<String>,
}

impl SnowflakeStoredProcedureCall {
    pub fn new(
        schema: Option<&str>,
        name: impl Into<String>,
        argument_expressions: Vec<String>,
    ) -> Result<Self, RuntimeError> {
        let schema = schema.map(ToString::to_string);
        let name = name.into();
        validate_optional_schema(schema.as_deref())?;
        validate_identifier(&name, "Snowflake stored procedure name")?;
        for expression in &argument_expressions {
            validate_argument_expression(expression)?;
        }
        Ok(Self {
            schema,
            name,
            argument_expressions,
        })
    }

    pub fn argument_count(&self) -> usize {
        self.argument_expressions.len()
    }

    pub fn statement(
        &self,
        builder: SnowflakeStatementBuilder,
    ) -> Result<SnowflakeStatement, RuntimeError> {
        let args = self.argument_expressions.join(", ");
        Ok(builder.finish(format!(
            "CALL {}({args})",
            qualified_procedure_name(self.schema.as_deref(), &self.name)?
        )))
    }
}

fn qualified_procedure_name(schema: Option<&str>, name: &str) -> Result<String, RuntimeError> {
    let name = quote_ident(name, "Snowflake stored procedure name")?;
    match schema {
        Some(schema) => Ok(format!(
            "{}.{}",
            quote_ident(schema, "Snowflake stored procedure schema")?,
            name
        )),
        None => Ok(name),
    }
}

fn validate_optional_schema(schema: Option<&str>) -> Result<(), RuntimeError> {
    if let Some(schema) = schema {
        validate_identifier(schema, "Snowflake stored procedure schema")?;
    }
    Ok(())
}

fn quote_ident(name: &str, label: &str) -> Result<String, RuntimeError> {
    validate_identifier(name, label)?;
    Ok(format!("\"{}\"", name.replace('"', "\"\"")))
}

fn validate_identifier(name: &str, label: &str) -> Result<(), RuntimeError> {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err(RuntimeError::Validation(format!(
            "{label} must not be empty"
        )));
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(RuntimeError::Validation(format!(
            "{label} must start with an ASCII letter or underscore"
        )));
    }
    if !chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
        return Err(RuntimeError::Validation(format!(
            "{label} must contain only ASCII letters, digits, or underscores"
        )));
    }
    Ok(())
}

fn validate_argument_expression(expression: &str) -> Result<(), RuntimeError> {
    match expression {
        "?"
        | "TO_NUMBER(?)"
        | "TO_DOUBLE(?)"
        | "TO_DATE(?)"
        | "TO_TIME(?)"
        | "TO_TIMESTAMP_TZ(?)"
        | "PARSE_JSON(?)" => Ok(()),
        _ => Err(RuntimeError::Validation(format!(
            "Snowflake stored procedure argument expression `{expression}` is not a provider binding expression"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use appfw_runtime::model_metadata::RuntimeDataType;
    use serde_json::json;

    use super::*;

    #[test]
    fn stored_procedure_call_uses_statement_bindings() {
        let mut builder = SnowflakeStatementBuilder::new();
        let account_id = builder
            .bind_scalar(
                RuntimeDataType::String,
                "account_id",
                true,
                json!("account-1"),
            )
            .expect("account id binding");
        let score = builder
            .bind_scalar(RuntimeDataType::Float64, "score", true, json!(98.5))
            .expect("score binding");
        let call = SnowflakeStoredProcedureCall::new(
            Some("crm"),
            "refresh_account_health",
            vec![account_id, score],
        )
        .expect("call");

        let statement = call.statement(builder).expect("statement");
        let body = statement.body(60, "APP_DB", None, None);

        assert_eq!(
            body["statement"],
            "CALL \"crm\".\"refresh_account_health\"(?, TO_DOUBLE(?))"
        );
        assert_eq!(body["bindings"]["1"]["value"], "account-1");
        assert_eq!(body["bindings"]["2"]["value"], "98.5");
    }

    #[test]
    fn stored_procedure_call_rejects_unsafe_inputs() {
        assert!(matches!(
            SnowflakeStoredProcedureCall::new(Some("crm"), "bad;drop", vec![]),
            Err(RuntimeError::Validation(_))
        ));
        assert!(matches!(
            SnowflakeStoredProcedureCall::new(
                Some("crm"),
                "ok_name",
                vec!["CURRENT_USER()".into()]
            ),
            Err(RuntimeError::Validation(_))
        ));
    }
}
