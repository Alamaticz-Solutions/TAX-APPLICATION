use appfw_runtime::{model_metadata::RuntimeDataType, RuntimeError};
use serde_json::{Map, Value};

use crate::SnowflakeStatementBuilder;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnowflakeMutationEntity {
    pub schema: String,
    pub table: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SnowflakeMutationField {
    pub name: String,
    pub data_type: RuntimeDataType,
    pub is_required: bool,
    pub is_key: bool,
    pub is_concurrency_control: bool,
    pub value: Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnowflakeInsertParts {
    pub fields: Vec<String>,
    pub values: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SnowflakeMutationUpdate {
    pub sets: Vec<String>,
    pub where_parts: Vec<String>,
    pub id_value: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SnowflakeMutationDelete {
    pub where_parts: Vec<String>,
    pub id_value: Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnowflakeJunctionTable {
    pub schema: String,
    pub table: String,
    pub local_key: String,
    pub foreign_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnowflakeJunctionInsertValues {
    pub new_id: String,
    pub entity_id: String,
    pub related_id: String,
    pub entity_id_exists: String,
    pub related_id_exists: String,
}

pub fn insert_parts(
    fields: &[SnowflakeMutationField],
    builder: &mut SnowflakeStatementBuilder,
) -> Result<SnowflakeInsertParts, RuntimeError> {
    let mut field_names = Vec::new();
    let mut values = Vec::new();
    for field in fields {
        field_names.push(quote_ident(&field.name));
        values.push(bind_field(builder, field, field.value.clone())?);
    }

    Ok(SnowflakeInsertParts {
        fields: field_names,
        values,
    })
}

pub fn insert_statement(entity: &SnowflakeMutationEntity, parts: &SnowflakeInsertParts) -> String {
    format!(
        "INSERT INTO {table} ({fields}) VALUES ({values})",
        table = table_ref(entity),
        fields = parts.fields.join(", "),
        values = parts.values.join(", "),
    )
}

pub fn update_parts(
    fields: &[SnowflakeMutationField],
    read_version: Option<Value>,
    set_builder: &mut SnowflakeStatementBuilder,
    where_builder: &mut SnowflakeStatementBuilder,
) -> Result<SnowflakeMutationUpdate, RuntimeError> {
    let mut sets = Vec::new();
    let mut where_parts = Vec::new();
    let mut id_value = Value::Null;

    for field in fields {
        if field.is_key {
            id_value = field.value.clone();
            let placeholder = bind_field(where_builder, field, field.value.clone())?;
            where_parts.push(format!("{} = {}", quote_ident(&field.name), placeholder));
            continue;
        }

        if field.is_concurrency_control {
            let old_version = read_version
                .clone()
                .ok_or(RuntimeError::InvalidKeyOrVersion)?;
            let placeholder = bind_field(where_builder, field, old_version)?;
            where_parts.push(format!("{} = {}", quote_ident(&field.name), placeholder));
        }

        let placeholder = bind_field(set_builder, field, field.value.clone())?;
        sets.push(format!("{} = {}", quote_ident(&field.name), placeholder));
    }

    Ok(SnowflakeMutationUpdate {
        sets,
        where_parts,
        id_value,
    })
}

pub fn update_statement(
    entity: &SnowflakeMutationEntity,
    sets: &[String],
    where_clause: &str,
) -> String {
    format!(
        "UPDATE {table} SET {sets} WHERE {where_clause}",
        table = table_ref(entity),
        sets = sets.join(", "),
        where_clause = where_clause,
    )
}

pub fn delete_parts(
    fields: &[SnowflakeMutationField],
    read_version: Option<Value>,
    where_builder: &mut SnowflakeStatementBuilder,
) -> Result<SnowflakeMutationDelete, RuntimeError> {
    let mut where_parts = Vec::new();
    let mut id_value = Value::Null;

    for field in fields {
        if field.is_key {
            id_value = field.value.clone();
            let placeholder = bind_field(where_builder, field, field.value.clone())?;
            where_parts.push(format!("{} = {}", quote_ident(&field.name), placeholder));
        } else if field.is_concurrency_control {
            let old_version = read_version
                .clone()
                .ok_or(RuntimeError::InvalidKeyOrVersion)?;
            let placeholder = bind_field(where_builder, field, old_version)?;
            where_parts.push(format!("{} = {}", quote_ident(&field.name), placeholder));
        }
    }

    Ok(SnowflakeMutationDelete {
        where_parts,
        id_value,
    })
}

pub fn delete_statement(entity: &SnowflakeMutationEntity, where_clause: &str) -> String {
    format!(
        "DELETE FROM {table} WHERE {where_clause}",
        table = table_ref(entity),
        where_clause = where_clause,
    )
}

pub fn count_statement(entity: &SnowflakeMutationEntity, where_clause: &str) -> String {
    format!(
        "SELECT COUNT(*) AS \"count\" FROM {table} WHERE {where_clause}",
        table = table_ref(entity),
        where_clause = where_clause,
    )
}

pub fn junction_insert_statement(
    junction: &SnowflakeJunctionTable,
    values: &SnowflakeJunctionInsertValues,
) -> String {
    format!(
        "INSERT INTO {junction_table} ({id_col}, {local_key}, {foreign_key}, {created_at}) SELECT {new_id}, {entity_id}, {related_id}, CURRENT_TIMESTAMP() WHERE NOT EXISTS (SELECT 1 FROM {junction_table} WHERE {local_key} = {entity_id_exists} AND {foreign_key} = {related_id_exists})",
        junction_table = junction_table_ref(junction),
        id_col = quote_ident("id"),
        local_key = quote_ident(&junction.local_key),
        foreign_key = quote_ident(&junction.foreign_key),
        created_at = quote_ident("created_at"),
        new_id = values.new_id,
        entity_id = values.entity_id,
        related_id = values.related_id,
        entity_id_exists = values.entity_id_exists,
        related_id_exists = values.related_id_exists,
    )
}

pub fn junction_delete_statement(
    junction: &SnowflakeJunctionTable,
    entity_id_placeholder: &str,
) -> String {
    format!(
        "DELETE FROM {junction_table} WHERE {local_key} = {entity_id}",
        junction_table = junction_table_ref(junction),
        local_key = quote_ident(&junction.local_key),
        entity_id = entity_id_placeholder,
    )
}

pub fn junction_related_entity_id(
    relationship_name: &str,
    target_pk: &str,
    related: &Map<String, Value>,
) -> Result<Value, RuntimeError> {
    related.get(target_pk).cloned().ok_or_else(|| {
        RuntimeError::Validation(format!(
            "related '{}' record is missing '{}'",
            relationship_name, target_pk
        ))
    })
}

fn bind_field(
    builder: &mut SnowflakeStatementBuilder,
    field: &SnowflakeMutationField,
    value: Value,
) -> Result<String, RuntimeError> {
    builder.bind_scalar(field.data_type, &field.name, field.is_required, value)
}

fn table_ref(entity: &SnowflakeMutationEntity) -> String {
    format!(
        "{}.{}",
        quote_ident(&entity.schema),
        quote_ident(&entity.table)
    )
}

fn junction_table_ref(junction: &SnowflakeJunctionTable) -> String {
    format!(
        "{}.{}",
        quote_ident(&junction.schema),
        quote_ident(&junction.table)
    )
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use appfw_runtime::model_metadata::RuntimeDataType;
    use serde_json::json;

    fn entity() -> SnowflakeMutationEntity {
        SnowflakeMutationEntity {
            schema: "crm".to_string(),
            table: "accounts".to_string(),
        }
    }

    fn field(
        name: &str,
        data_type: RuntimeDataType,
        is_key: bool,
        is_concurrency_control: bool,
        value: Value,
    ) -> SnowflakeMutationField {
        SnowflakeMutationField {
            name: name.to_string(),
            data_type,
            is_required: true,
            is_key,
            is_concurrency_control,
            value,
        }
    }

    #[test]
    fn insert_binds_fields_and_renders_statement() {
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
        let mut builder = SnowflakeStatementBuilder::new();

        let parts = insert_parts(&fields, &mut builder).expect("insert parts");
        let sql = insert_statement(&entity(), &parts);

        assert_eq!(
            sql,
            "INSERT INTO \"crm\".\"accounts\" (\"id\", \"name\") VALUES (?, ?)"
        );
        let statement = builder.finish(sql);
        let body = statement.body(30, "app", None, None);
        assert_eq!(body["bindings"].as_object().map(|b| b.len()), Some(2));
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
        let mut set_builder = SnowflakeStatementBuilder::new();
        let mut where_builder = SnowflakeStatementBuilder::new();

        let update = update_parts(
            &fields,
            Some(json!(1)),
            &mut set_builder,
            &mut where_builder,
        )
        .expect("update parts");
        let sql = update_statement(
            &entity(),
            &update.sets,
            &format!("{} AND \"tenant_id\" = ?", update.where_parts.join(" AND ")),
        );

        assert_eq!(
            update.where_parts,
            vec!["\"id\" = ?", "\"version\" = TO_NUMBER(?)"]
        );
        assert_eq!(
            update.sets,
            vec!["\"version\" = TO_NUMBER(?)", "\"name\" = ?"]
        );
        assert_eq!(
            sql,
            "UPDATE \"crm\".\"accounts\" SET \"version\" = TO_NUMBER(?), \"name\" = ? WHERE \"id\" = ? AND \"version\" = TO_NUMBER(?) AND \"tenant_id\" = ?"
        );
    }

    #[test]
    fn delete_requires_read_version_for_concurrency_field() {
        let fields = vec![
            field("id", RuntimeDataType::Int64, true, false, json!(42)),
            field("version", RuntimeDataType::Int64, false, true, json!(2)),
        ];
        let mut builder = SnowflakeStatementBuilder::new();

        match delete_parts(&fields, None, &mut builder) {
            Err(RuntimeError::InvalidKeyOrVersion) => {}
            Err(error) => panic!("unexpected error: {error}"),
            Ok(_) => panic!("delete should fail"),
        }
    }

    #[test]
    fn renders_junction_insert_and_delete_statements() {
        let junction = SnowflakeJunctionTable {
            schema: "crm".to_string(),
            table: "account_contacts".to_string(),
            local_key: "account_id".to_string(),
            foreign_key: "contact_id".to_string(),
        };
        let values = SnowflakeJunctionInsertValues {
            new_id: "?".to_string(),
            entity_id: "?".to_string(),
            related_id: "?".to_string(),
            entity_id_exists: "?".to_string(),
            related_id_exists: "?".to_string(),
        };

        assert_eq!(
            junction_insert_statement(&junction, &values),
            "INSERT INTO \"crm\".\"account_contacts\" (\"id\", \"account_id\", \"contact_id\", \"created_at\") SELECT ?, ?, ?, CURRENT_TIMESTAMP() WHERE NOT EXISTS (SELECT 1 FROM \"crm\".\"account_contacts\" WHERE \"account_id\" = ? AND \"contact_id\" = ?)"
        );
        assert_eq!(
            junction_delete_statement(&junction, "?"),
            "DELETE FROM \"crm\".\"account_contacts\" WHERE \"account_id\" = ?"
        );
    }

    #[test]
    fn extracts_related_entity_id_by_target_primary_key() {
        let related = json!({"contact_id": "contact-1"});
        let related = related.as_object().expect("object");

        assert_eq!(
            junction_related_entity_id("contacts", "contact_id", related).expect("related id"),
            json!("contact-1")
        );

        let error = junction_related_entity_id("contacts", "id", related).expect_err("missing id");
        assert!(matches!(error, RuntimeError::Validation(_)));
    }
}
