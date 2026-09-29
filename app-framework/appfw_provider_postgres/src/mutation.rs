use appfw_runtime::{model_metadata::RuntimeDataType, RuntimeError};
use serde_json::Value;

use crate::param::{prop_param_ref, type_param, SqlParam};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PostgresMutationEntity {
    pub schema: String,
    pub table: String,
    pub primary_key: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PostgresMutationField {
    pub name: String,
    pub data_type: RuntimeDataType,
    pub is_nullable: bool,
    pub is_key: bool,
    pub is_concurrency_control: bool,
    pub value: Value,
}

pub struct PostgresMutationUpdate {
    pub assignments: Vec<String>,
    pub params: Vec<SqlParam>,
    pub primary_key_constraint: String,
    pub version_constraint: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PostgresJunctionTable {
    pub schema: String,
    pub table: String,
    pub local_key: String,
    pub foreign_key: String,
}

pub fn insert_statement(
    entity: &PostgresMutationEntity,
    fields: &[PostgresMutationField],
) -> Result<(String, Vec<SqlParam>), RuntimeError> {
    let mut columns = Vec::new();
    let mut placeholders = Vec::new();
    let mut params = Vec::new();

    for field in fields {
        // Let PostgreSQL defaults/generators produce the key when callers send
        // a null key value.
        if field.is_key && field.value.is_null() {
            continue;
        }

        params.push(bind_field(field, field.value.clone())?);
        columns.push(field.name.clone());
        placeholders.push(prop_param_ref(field.data_type, params.len())?);
    }

    Ok((
        format!(
            r#"
        INSERT INTO {table} ({columns})
        VALUES ({values})
        ON CONFLICT DO NOTHING
        RETURNING to_jsonb({pk}) as id
        "#,
            table = table_ref(entity),
            columns = columns.join(","),
            values = placeholders.join(","),
            pk = entity.primary_key,
        ),
        params,
    ))
}

pub fn junction_insert_statement(junction: &PostgresJunctionTable) -> String {
    format!(
        r#"
        INSERT INTO {table} ({local_key}, {foreign_key}, created_at, created_by)
        VALUES ($1, $2, NOW(), $3)
        ON CONFLICT ({local_key}, {foreign_key}) DO NOTHING
        "#,
        table = junction_table_ref(junction),
        local_key = junction.local_key,
        foreign_key = junction.foreign_key,
    )
}

pub fn junction_delete_statement(junction: &PostgresJunctionTable) -> String {
    format!(
        r#"DELETE FROM {table} WHERE {local_key} = $1"#,
        table = junction_table_ref(junction),
        local_key = junction.local_key,
    )
}

pub fn junction_related_entity_id(entity_value: &Value) -> Result<i64, String> {
    match entity_value {
        Value::Object(obj) => match obj.get("id") {
            Some(Value::Number(value)) => value
                .as_i64()
                .ok_or_else(|| "Invalid entity ID".to_string()),
            Some(Value::String(value)) => value
                .parse::<i64>()
                .map_err(|_| "Invalid entity ID format".to_string()),
            Some(_) => Err("Entity ID must be a number or string".to_string()),
            None => Err("Related entity must have an 'id' field".to_string()),
        },
        Value::Number(value) => value
            .as_i64()
            .ok_or_else(|| "Invalid entity ID".to_string()),
        Value::String(value) => value
            .parse::<i64>()
            .map_err(|_| "Invalid entity ID format".to_string()),
        _ => Err("Invalid related entity format".to_string()),
    }
}

pub fn update_parts(
    fields: &[PostgresMutationField],
    read_version: Option<Value>,
) -> Result<PostgresMutationUpdate, RuntimeError> {
    let mut assignments = Vec::new();
    let mut params = Vec::new();
    let mut primary_key_constraint = String::new();
    let mut version_constraint = String::new();

    for field in fields {
        if field.is_key {
            params.push(bind_field(field, field.value.clone())?);
            primary_key_constraint = pk_constraint(&field.name, params.len());
            continue;
        }

        if field.is_concurrency_control {
            let read_version = read_version
                .clone()
                .ok_or(RuntimeError::InvalidKeyOrVersion)?;
            params.push(bind_field(field, read_version)?);
            version_constraint = version_constraint_sql(&field.name, params.len());
        }

        params.push(bind_field(field, field.value.clone())?);
        assignments.push(format!(
            "{} = {}",
            field.name,
            prop_param_ref(field.data_type, params.len())?
        ));
    }

    Ok(PostgresMutationUpdate {
        assignments,
        params,
        primary_key_constraint,
        version_constraint,
    })
}

pub fn update_statement(
    entity: &PostgresMutationEntity,
    update: &PostgresMutationUpdate,
    access_constraint: &str,
) -> String {
    format!(
        r#"
        UPDATE {table} AS t0
        SET {assignments}
        WHERE {primary_key_constraint}{version_constraint}{access_constraint}
        RETURNING to_jsonb(t0.id) as id
      "#,
        table = table_ref(entity),
        assignments = update.assignments.join(","),
        primary_key_constraint = update.primary_key_constraint,
        version_constraint = update.version_constraint,
        access_constraint = access_constraint,
    )
}

pub fn delete_statement(
    entity: &PostgresMutationEntity,
    fields: &[PostgresMutationField],
    read_version: Option<Value>,
    access_constraint: &str,
) -> Result<(String, Vec<SqlParam>), RuntimeError> {
    let mut params = Vec::new();
    let mut primary_key_constraint = String::new();
    let mut version_constraint = String::new();

    for field in fields {
        if field.is_key {
            params.push(bind_field(field, field.value.clone())?);
            primary_key_constraint = pk_constraint(&field.name, params.len());
        } else if field.is_concurrency_control {
            let read_version = read_version
                .clone()
                .ok_or(RuntimeError::InvalidKeyOrVersion)?;
            params.push(bind_field(field, read_version)?);
            version_constraint = version_constraint_sql(&field.name, params.len());
        }
    }

    Ok((
        format!(
            r#"
        DELETE FROM {table} AS t0
        WHERE {primary_key_constraint}{version_constraint}{access_constraint}
        RETURNING (to_jsonb(id))
      "#,
            table = table_ref(entity),
            primary_key_constraint = primary_key_constraint,
            version_constraint = version_constraint,
            access_constraint = access_constraint,
        ),
        params,
    ))
}

fn bind_field(field: &PostgresMutationField, value: Value) -> Result<SqlParam, RuntimeError> {
    type_param(field.data_type, field.is_nullable, &field.name, value)
}

fn pk_constraint(prop_name: &str, param_ndx: usize) -> String {
    format!("{} = ${}", prop_name, param_ndx)
}

fn version_constraint_sql(prop_name: &str, param_ndx: usize) -> String {
    format!(" AND {} = ${}", prop_name, param_ndx)
}

fn table_ref(entity: &PostgresMutationEntity) -> String {
    format!("{}.{}", entity.schema, entity.table)
}

fn junction_table_ref(junction: &PostgresJunctionTable) -> String {
    format!("{}.{}", junction.schema, junction.table)
}

#[cfg(test)]
mod tests {
    use super::*;
    use appfw_runtime::model_metadata::RuntimeDataType;
    use serde_json::json;

    fn entity() -> PostgresMutationEntity {
        PostgresMutationEntity {
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
    ) -> PostgresMutationField {
        PostgresMutationField {
            name: name.to_string(),
            data_type,
            is_nullable: false,
            is_key,
            is_concurrency_control,
            value,
        }
    }

    #[test]
    fn insert_skips_null_key_and_returns_json_id() {
        let fields = vec![
            field("id", RuntimeDataType::Uuid, true, false, Value::Null),
            field("name", RuntimeDataType::String, false, false, json!("Acme")),
        ];

        let (sql, params) = insert_statement(&entity(), &fields).expect("insert");

        assert!(sql.contains("INSERT INTO crm.accounts (name)"));
        assert!(sql.contains("VALUES ($1)"));
        assert!(sql.contains("RETURNING to_jsonb(id) as id"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn update_parts_preserve_key_version_and_assignment_order() {
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

        let update = update_parts(&fields, Some(json!(1))).expect("update parts");
        let sql = update_statement(&entity(), &update, " AND (t0.tenant_id = $5)");

        assert_eq!(update.primary_key_constraint, "id = $1");
        assert_eq!(update.version_constraint, " AND version = $2");
        assert_eq!(update.assignments, vec!["version = $3", "name = $4"]);
        assert!(sql.contains("WHERE id = $1 AND version = $2 AND (t0.tenant_id = $5)"));
        assert_eq!(update.params.len(), 4);
    }

    #[test]
    fn delete_requires_read_version_for_concurrency_field() {
        let fields = vec![
            field("id", RuntimeDataType::Int64, true, false, json!(42)),
            field("version", RuntimeDataType::Int64, false, true, json!(2)),
        ];

        match delete_statement(&entity(), &fields, None, "") {
            Err(RuntimeError::InvalidKeyOrVersion) => {}
            Err(error) => panic!("unexpected error: {error}"),
            Ok(_) => panic!("delete should fail"),
        }
    }

    #[test]
    fn delete_statement_includes_access_constraint() {
        let fields = vec![field("id", RuntimeDataType::Int64, true, false, json!(42))];

        let (sql, params) =
            delete_statement(&entity(), &fields, None, " AND (t0.tenant_id = $2)").expect("delete");

        assert!(sql.contains("DELETE FROM crm.accounts AS t0"));
        assert!(sql.contains("WHERE id = $1 AND (t0.tenant_id = $2)"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn renders_junction_insert_and_delete_statements() {
        let junction = PostgresJunctionTable {
            schema: "crm".to_string(),
            table: "account_contact".to_string(),
            local_key: "account_id".to_string(),
            foreign_key: "contact_id".to_string(),
        };

        let insert = junction_insert_statement(&junction);
        let delete = junction_delete_statement(&junction);

        assert!(insert.contains("INSERT INTO crm.account_contact"));
        assert!(insert.contains("(account_id, contact_id, created_at, created_by)"));
        assert!(insert.contains("ON CONFLICT (account_id, contact_id) DO NOTHING"));
        assert_eq!(
            delete,
            "DELETE FROM crm.account_contact WHERE account_id = $1"
        );
    }

    #[test]
    fn extracts_junction_related_entity_ids_from_supported_shapes() {
        assert_eq!(junction_related_entity_id(&json!(42)).expect("number"), 42);
        assert_eq!(
            junction_related_entity_id(&json!("42")).expect("string"),
            42
        );
        assert_eq!(
            junction_related_entity_id(&json!({ "id": 42 })).expect("object number"),
            42
        );
        assert_eq!(
            junction_related_entity_id(&json!({ "id": "42" })).expect("object string"),
            42
        );
    }

    #[test]
    fn rejects_invalid_junction_related_entity_ids() {
        assert_eq!(
            junction_related_entity_id(&json!({ "name": "Acme" })).expect_err("missing id"),
            "Related entity must have an 'id' field"
        );
        assert_eq!(
            junction_related_entity_id(&json!({ "id": true })).expect_err("wrong type"),
            "Entity ID must be a number or string"
        );
        assert_eq!(
            junction_related_entity_id(&json!("not-a-number")).expect_err("bad string"),
            "Invalid entity ID format"
        );
    }
}
