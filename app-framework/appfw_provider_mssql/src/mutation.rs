use appfw_runtime::{model_metadata::RuntimeDataType, RuntimeError};
use serde_json::Value;

use crate::param::{param_placeholder, type_param, SqlParam};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MssqlMutationEntity {
    pub schema: String,
    pub table: String,
    pub primary_key: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MssqlMutationField {
    pub name: String,
    pub data_type: RuntimeDataType,
    pub is_nullable: bool,
    pub is_key: bool,
    pub is_concurrency_control: bool,
    pub value: Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MssqlJunctionTable {
    pub schema: String,
    pub table: String,
    pub local_key: String,
    pub foreign_key: String,
}

pub fn insert_statement(
    entity: &MssqlMutationEntity,
    fields: &[MssqlMutationField],
) -> Result<(String, Vec<SqlParam>), RuntimeError> {
    let mut columns = Vec::new();
    let mut placeholders = Vec::new();
    let mut params = Vec::new();

    for field in fields {
        // Let IDENTITY / default key generation produce the key when callers
        // send a null key value.
        if field.is_key && field.value.is_null() {
            continue;
        }

        params.push(bind_field(field, field.value.clone())?);
        columns.push(quote_ident(&field.name));
        placeholders.push(param_placeholder(params.len()));
    }

    Ok((
        format!(
            "INSERT INTO {table} ({columns}) OUTPUT INSERTED.{pk} AS [id] VALUES ({values});",
            table = table_ref(entity),
            columns = columns.join(", "),
            pk = quote_ident(&entity.primary_key),
            values = placeholders.join(", "),
        ),
        params,
    ))
}

pub fn update_statement(
    entity: &MssqlMutationEntity,
    fields: &[MssqlMutationField],
    read_version: Option<Value>,
    access_constraint: &str,
) -> Result<(String, Vec<SqlParam>), RuntimeError> {
    let mut assignments = Vec::new();
    let mut params = Vec::new();
    let mut pk_clause = String::new();
    let mut version_clause = String::new();

    for field in fields {
        if field.is_key {
            params.push(bind_field(field, field.value.clone())?);
            pk_clause = format!(
                "{} = {}",
                qualify("t0", &field.name),
                param_placeholder(params.len())
            );
            continue;
        }

        if field.is_concurrency_control {
            let read_version = read_version
                .clone()
                .ok_or(RuntimeError::InvalidKeyOrVersion)?;
            params.push(bind_field(field, read_version)?);
            version_clause = format!(
                " AND {} = {}",
                qualify("t0", &field.name),
                param_placeholder(params.len())
            );
        }

        params.push(bind_field(field, field.value.clone())?);
        assignments.push(format!(
            "{} = {}",
            quote_ident(&field.name),
            param_placeholder(params.len())
        ));
    }

    if pk_clause.is_empty() {
        return Err(RuntimeError::Validation(
            "update requires the primary key".to_string(),
        ));
    }

    Ok((
        format!(
            "UPDATE [t0] SET {sets} OUTPUT INSERTED.{pk} AS [id] FROM {table} AS [t0] WHERE {pk_clause}{version_clause}{access_constraint};",
            table = table_ref(entity),
            sets = assignments.join(", "),
            pk = quote_ident(&entity.primary_key),
            pk_clause = pk_clause,
            version_clause = version_clause,
            access_constraint = access_constraint,
        ),
        params,
    ))
}

pub fn delete_statement(
    entity: &MssqlMutationEntity,
    fields: &[MssqlMutationField],
    read_version: Option<Value>,
    access_constraint: &str,
) -> Result<(String, Vec<SqlParam>), RuntimeError> {
    let mut params = Vec::new();
    let mut pk_clause = String::new();
    let mut version_clause = String::new();

    for field in fields {
        if field.is_key {
            params.push(bind_field(field, field.value.clone())?);
            pk_clause = format!(
                "{} = {}",
                qualify("t0", &field.name),
                param_placeholder(params.len())
            );
        } else if field.is_concurrency_control {
            let read_version = read_version
                .clone()
                .ok_or(RuntimeError::InvalidKeyOrVersion)?;
            params.push(bind_field(field, read_version)?);
            version_clause = format!(
                " AND {} = {}",
                qualify("t0", &field.name),
                param_placeholder(params.len())
            );
        }
    }

    if pk_clause.is_empty() {
        return Err(RuntimeError::Validation(
            "delete requires the primary key".to_string(),
        ));
    }

    Ok((
        format!(
            "DELETE [t0] OUTPUT DELETED.{pk} AS [id] FROM {table} AS [t0] WHERE {pk_clause}{version_clause}{access_constraint};",
            table = table_ref(entity),
            pk = quote_ident(&entity.primary_key),
            pk_clause = pk_clause,
            version_clause = version_clause,
            access_constraint = access_constraint,
        ),
        params,
    ))
}

pub fn junction_insert_statement(junction: &MssqlJunctionTable) -> String {
    format!(
        "INSERT INTO {table} ({local_key}, {foreign_key}, [created_at]) \
         SELECT @P1, @P2, SYSDATETIMEOFFSET() \
         WHERE NOT EXISTS ( \
           SELECT 1 FROM {table} \
           WHERE {local_key} = @P1 AND {foreign_key} = @P2 \
         );",
        table = junction_table_ref(junction),
        local_key = quote_ident(&junction.local_key),
        foreign_key = quote_ident(&junction.foreign_key),
    )
}

pub fn junction_delete_statement(junction: &MssqlJunctionTable) -> String {
    format!(
        "DELETE FROM {table} WHERE {local_key} = @P1;",
        table = junction_table_ref(junction),
        local_key = quote_ident(&junction.local_key),
    )
}

pub fn junction_related_entity_id(entity_value: &Value) -> Result<Value, RuntimeError> {
    match entity_value {
        Value::Object(obj) => obj.get("id").cloned().ok_or_else(|| {
            RuntimeError::Validation("Related entity must have an 'id' field".to_string())
        }),
        Value::Number(_) | Value::String(_) => Ok(entity_value.clone()),
        _ => Err(RuntimeError::Validation(
            "Invalid related entity format".to_string(),
        )),
    }
}

fn bind_field(field: &MssqlMutationField, value: Value) -> Result<SqlParam, RuntimeError> {
    type_param(field.data_type, field.is_nullable, &field.name, value)
}

fn table_ref(entity: &MssqlMutationEntity) -> String {
    format!(
        "{}.{}",
        quote_ident(&entity.schema),
        quote_ident(&entity.table)
    )
}

fn junction_table_ref(junction: &MssqlJunctionTable) -> String {
    format!(
        "{}.{}",
        quote_ident(&junction.schema),
        quote_ident(&junction.table)
    )
}

fn quote_ident(name: &str) -> String {
    format!("[{}]", name.replace(']', "]]"))
}

fn qualify(alias: &str, name: &str) -> String {
    format!("{}.{}", quote_ident(alias), quote_ident(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use appfw_runtime::model_metadata::RuntimeDataType;
    use serde_json::json;

    fn entity() -> MssqlMutationEntity {
        MssqlMutationEntity {
            schema: "crm".to_string(),
            table: "accounts".to_string(),
            primary_key: "id".to_string(),
        }
    }

    fn field(
        name: &str,
        data_type: RuntimeDataType,
        is_key: bool,
        is_concurrency_control: bool,
        value: Value,
    ) -> MssqlMutationField {
        MssqlMutationField {
            name: name.to_string(),
            data_type,
            is_nullable: false,
            is_key,
            is_concurrency_control,
            value,
        }
    }

    #[test]
    fn insert_skips_null_key_and_returns_output_id() {
        let fields = vec![
            field("id", RuntimeDataType::Uuid, true, false, Value::Null),
            field("name", RuntimeDataType::String, false, false, json!("Acme")),
        ];

        let (sql, params) = insert_statement(&entity(), &fields).expect("insert");

        assert_eq!(
            sql,
            "INSERT INTO [crm].[accounts] ([name]) OUTPUT INSERTED.[id] AS [id] VALUES (@P1);"
        );
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn update_builds_key_version_and_assignment_params_in_order() {
        let fields = vec![
            field(
                "id",
                RuntimeDataType::Uuid,
                true,
                false,
                json!("00000000-0000-0000-0000-000000000001"),
            ),
            field("version", RuntimeDataType::Int64, false, true, json!(2)),
            field("name", RuntimeDataType::String, false, false, json!("Acme")),
        ];

        let (sql, params) =
            update_statement(&entity(), &fields, Some(json!(1)), "").expect("update");

        assert_eq!(
            sql,
            "UPDATE [t0] SET [version] = @P3, [name] = @P4 OUTPUT INSERTED.[id] AS [id] FROM [crm].[accounts] AS [t0] WHERE [t0].[id] = @P1 AND [t0].[version] = @P2;"
        );
        assert_eq!(params.len(), 4);
    }

    #[test]
    fn update_includes_access_constraint() {
        let fields = vec![
            field(
                "id",
                RuntimeDataType::Uuid,
                true,
                false,
                json!("00000000-0000-0000-0000-000000000001"),
            ),
            field("name", RuntimeDataType::String, false, false, json!("Acme")),
        ];

        let (sql, params) =
            update_statement(&entity(), &fields, None, " AND ([t0].[tenant_id] = @P3)")
                .expect("update");

        assert_eq!(
            sql,
            "UPDATE [t0] SET [name] = @P2 OUTPUT INSERTED.[id] AS [id] FROM [crm].[accounts] AS [t0] WHERE [t0].[id] = @P1 AND ([t0].[tenant_id] = @P3);"
        );
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn delete_requires_read_version_for_concurrency_field() {
        let fields = vec![
            field("id", RuntimeDataType::Int64, true, false, json!(42)),
            field("version", RuntimeDataType::Int64, false, true, json!(2)),
        ];

        let error = delete_statement(&entity(), &fields, None, "").expect_err("delete should fail");

        assert!(matches!(error, RuntimeError::InvalidKeyOrVersion));
    }

    #[test]
    fn delete_includes_access_constraint() {
        let fields = vec![field("id", RuntimeDataType::Int64, true, false, json!(42))];

        let (sql, params) =
            delete_statement(&entity(), &fields, None, " AND ([t0].[tenant_id] = @P2)")
                .expect("delete");

        assert_eq!(
            sql,
            "DELETE [t0] OUTPUT DELETED.[id] AS [id] FROM [crm].[accounts] AS [t0] WHERE [t0].[id] = @P1 AND ([t0].[tenant_id] = @P2);"
        );
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn renders_junction_insert_and_delete_statements() {
        let junction = MssqlJunctionTable {
            schema: "crm".to_string(),
            table: "account_contacts".to_string(),
            local_key: "account_id".to_string(),
            foreign_key: "contact_id".to_string(),
        };

        assert_eq!(
            junction_insert_statement(&junction),
            "INSERT INTO [crm].[account_contacts] ([account_id], [contact_id], [created_at]) \
         SELECT @P1, @P2, SYSDATETIMEOFFSET() \
         WHERE NOT EXISTS ( \
           SELECT 1 FROM [crm].[account_contacts] \
           WHERE [account_id] = @P1 AND [contact_id] = @P2 \
         );"
        );
        assert_eq!(
            junction_delete_statement(&junction),
            "DELETE FROM [crm].[account_contacts] WHERE [account_id] = @P1;"
        );
    }

    #[test]
    fn extracts_related_entity_id_from_object_or_scalar() {
        assert_eq!(
            junction_related_entity_id(&json!({"id": "contact-1"})).expect("object id"),
            json!("contact-1")
        );
        assert_eq!(
            junction_related_entity_id(&json!(42)).expect("numeric id"),
            json!(42)
        );

        let error =
            junction_related_entity_id(&json!({"name": "missing id"})).expect_err("missing id");
        assert!(matches!(error, RuntimeError::Validation(_)));
    }
}
