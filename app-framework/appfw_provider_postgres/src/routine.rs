use appfw_runtime::RuntimeError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PostgresStoredProcedureCall {
    schema: Option<String>,
    name: String,
    argument_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PostgresFunctionCall {
    schema: Option<String>,
    name: String,
    argument_count: usize,
}

impl PostgresStoredProcedureCall {
    pub fn new(
        schema: Option<&str>,
        name: impl Into<String>,
        argument_count: usize,
    ) -> Result<Self, RuntimeError> {
        let schema = schema.map(ToString::to_string);
        let name = name.into();
        validate_optional_schema(schema.as_deref())?;
        validate_identifier(&name, "PostgreSQL stored procedure name")?;
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
        Ok(format!(
            "CALL {}({})",
            qualified_routine_name(self.schema.as_deref(), &self.name)?,
            positional_placeholders(self.argument_count)
        ))
    }

    pub fn json_result_statement(&self) -> Result<String, RuntimeError> {
        Ok(format!(
            "CALL {}({})",
            qualified_routine_name(self.schema.as_deref(), &self.name)?,
            json_result_placeholders(self.argument_count)
        ))
    }
}

impl PostgresFunctionCall {
    pub fn new(
        schema: Option<&str>,
        name: impl Into<String>,
        argument_count: usize,
    ) -> Result<Self, RuntimeError> {
        let schema = schema.map(ToString::to_string);
        let name = name.into();
        validate_optional_schema(schema.as_deref())?;
        validate_identifier(&name, "PostgreSQL function name")?;
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
        Ok(format!(
            "SELECT * FROM {}({})",
            qualified_routine_name(self.schema.as_deref(), &self.name)?,
            positional_placeholders(self.argument_count)
        ))
    }

    pub fn json_one_statement(&self) -> Result<String, RuntimeError> {
        Ok(format!(
            "SELECT to_jsonb(r) FROM {}({}) AS r LIMIT 1",
            qualified_routine_name(self.schema.as_deref(), &self.name)?,
            positional_placeholders(self.argument_count)
        ))
    }

    pub fn json_many_statement(&self) -> Result<String, RuntimeError> {
        Ok(format!(
            "SELECT COALESCE(jsonb_agg(to_jsonb(r)), '[]'::jsonb) FROM {}({}) AS r",
            qualified_routine_name(self.schema.as_deref(), &self.name)?,
            positional_placeholders(self.argument_count)
        ))
    }
}

fn qualified_routine_name(schema: Option<&str>, name: &str) -> Result<String, RuntimeError> {
    let name = quote_ident(name, "PostgreSQL routine name")?;
    match schema {
        Some(schema) => Ok(format!(
            "{}.{}",
            quote_ident(schema, "PostgreSQL routine schema")?,
            name
        )),
        None => Ok(name),
    }
}

fn positional_placeholders(argument_count: usize) -> String {
    (1..=argument_count)
        .map(|index| format!("${index}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn json_result_placeholders(argument_count: usize) -> String {
    let mut placeholders = (1..=argument_count)
        .map(|index| format!("${index}"))
        .collect::<Vec<_>>();
    placeholders.push("NULL::jsonb".to_string());
    placeholders.join(", ")
}

fn validate_optional_schema(schema: Option<&str>) -> Result<(), RuntimeError> {
    if let Some(schema) = schema {
        validate_identifier(schema, "PostgreSQL routine schema")?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_procedure_call_quotes_identifiers_and_placeholders() {
        let call = PostgresStoredProcedureCall::new(Some("crm"), "refresh_account_health", 2)
            .expect("stored procedure call");

        assert_eq!(
            call.statement().expect("statement"),
            "CALL \"crm\".\"refresh_account_health\"($1, $2)"
        );
        assert_eq!(
            call.json_result_statement().expect("statement"),
            "CALL \"crm\".\"refresh_account_health\"($1, $2, NULL::jsonb)"
        );
    }

    #[test]
    fn function_call_quotes_identifiers_and_placeholders() {
        let call =
            PostgresFunctionCall::new(Some("crm"), "account_health_score", 1).expect("function");

        assert_eq!(
            call.statement().expect("statement"),
            "SELECT * FROM \"crm\".\"account_health_score\"($1)"
        );
    }

    #[test]
    fn function_call_builds_json_result_statements() {
        let call =
            PostgresFunctionCall::new(Some("crm"), "account_health_score", 1).expect("function");

        assert_eq!(
            call.json_one_statement().expect("statement"),
            "SELECT to_jsonb(r) FROM \"crm\".\"account_health_score\"($1) AS r LIMIT 1"
        );
        assert_eq!(
            call.json_many_statement().expect("statement"),
            "SELECT COALESCE(jsonb_agg(to_jsonb(r)), '[]'::jsonb) FROM \"crm\".\"account_health_score\"($1) AS r"
        );
    }

    #[test]
    fn routine_calls_reject_unsafe_identifiers() {
        assert!(matches!(
            PostgresStoredProcedureCall::new(Some("crm"), "bad;drop", 0),
            Err(RuntimeError::Validation(_))
        ));
        assert!(matches!(
            PostgresFunctionCall::new(Some("bad.schema"), "ok_name", 0),
            Err(RuntimeError::Validation(_))
        ));
    }
}
