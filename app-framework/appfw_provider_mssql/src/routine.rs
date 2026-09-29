use appfw_runtime::RuntimeError;

use crate::param_placeholder;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MssqlStoredProcedureCall {
    schema: Option<String>,
    name: String,
    argument_count: usize,
}

impl MssqlStoredProcedureCall {
    pub fn new(
        schema: Option<&str>,
        name: impl Into<String>,
        argument_count: usize,
    ) -> Result<Self, RuntimeError> {
        let schema = schema.map(ToString::to_string);
        let name = name.into();
        validate_optional_schema(schema.as_deref())?;
        validate_identifier(&name, "MS SQL Server stored procedure name")?;
        Ok(Self {
            schema,
            name,
            argument_count,
        })
    }

    pub fn argument_count(&self) -> usize {
        self.argument_count
    }

    pub fn statement(&self) -> Result<String, RuntimeError> {
        let args = (1..=self.argument_count)
            .map(param_placeholder)
            .collect::<Vec<_>>()
            .join(", ");
        let suffix = if args.is_empty() {
            String::new()
        } else {
            format!(" {args}")
        };
        Ok(format!(
            "EXEC {}{}",
            qualified_procedure_name(self.schema.as_deref(), &self.name)?,
            suffix
        ))
    }
}

fn qualified_procedure_name(schema: Option<&str>, name: &str) -> Result<String, RuntimeError> {
    let name = quote_ident(name, "MS SQL Server stored procedure name")?;
    match schema {
        Some(schema) => Ok(format!(
            "{}.{}",
            quote_ident(schema, "MS SQL Server stored procedure schema")?,
            name
        )),
        None => Ok(name),
    }
}

fn validate_optional_schema(schema: Option<&str>) -> Result<(), RuntimeError> {
    if let Some(schema) = schema {
        validate_identifier(schema, "MS SQL Server stored procedure schema")?;
    }
    Ok(())
}

fn quote_ident(name: &str, label: &str) -> Result<String, RuntimeError> {
    validate_identifier(name, label)?;
    Ok(format!("[{}]", name.replace(']', "]]")))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_procedure_call_uses_provider_placeholders() {
        let call =
            MssqlStoredProcedureCall::new(Some("crm"), "refresh_account_health", 2).expect("call");

        assert_eq!(
            call.statement().expect("statement"),
            "EXEC [crm].[refresh_account_health] @P1, @P2"
        );
    }

    #[test]
    fn stored_procedure_call_supports_zero_arguments() {
        let call =
            MssqlStoredProcedureCall::new(Some("crm"), "refresh_daily_rollups", 0).expect("call");

        assert_eq!(
            call.statement().expect("statement"),
            "EXEC [crm].[refresh_daily_rollups]"
        );
    }

    #[test]
    fn stored_procedure_call_rejects_unsafe_identifiers() {
        assert!(matches!(
            MssqlStoredProcedureCall::new(Some("crm"), "bad;drop", 0),
            Err(RuntimeError::Validation(_))
        ));
        assert!(matches!(
            MssqlStoredProcedureCall::new(Some("bad.schema"), "ok_name", 0),
            Err(RuntimeError::Validation(_))
        ));
    }
}
