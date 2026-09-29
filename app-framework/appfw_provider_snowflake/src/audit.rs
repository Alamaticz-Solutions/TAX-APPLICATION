use appfw_runtime::{
    model_metadata::RuntimeDataType, RuntimeAuditEvent, RuntimeAuditQuery, RuntimeError,
};
use serde_json::Value;

use crate::{SnowflakeStatement, SnowflakeStatementBuilder};

pub fn previous_hash_statement(
    event: &RuntimeAuditEvent,
) -> Result<SnowflakeStatement, RuntimeError> {
    let mut builder = SnowflakeStatementBuilder::new();
    let chain_scope = bind_text(&mut builder, "chain_scope", &event.chain_scope)?;
    let sql = format!(
        r#"
            SELECT "event_hash" AS event_hash
            FROM {schema}.{table}
            WHERE "chain_scope" = {chain_scope}
            ORDER BY "occurred_at" DESC, "audit_id" DESC
            LIMIT 1
            "#,
        schema = quote_ident(&event.schema_name),
        table = quote_ident(&event.audit_table_name),
        chain_scope = chain_scope,
    );
    Ok(builder.finish(sql))
}

pub fn insert_statement(event: &RuntimeAuditEvent) -> Result<SnowflakeStatement, RuntimeError> {
    let actor_roles = serde_json::to_value(&event.actor_roles)
        .map_err(|e| RuntimeError::DataAccess(e.to_string()))?;
    let mut builder = SnowflakeStatementBuilder::new();
    let audit_id = bind_text(&mut builder, "audit_id", &event.audit_id)?;
    let occurred_at = builder.bind_scalar(
        RuntimeDataType::DateTime,
        "occurred_at",
        true,
        Value::String(event.occurred_at.to_rfc3339()),
    )?;
    let tenant_id = bind_maybe_text(&mut builder, "tenant_id", &event.tenant_id)?;
    let actor_user_name = bind_text(&mut builder, "actor_user_name", &event.actor_user_name)?;
    let actor_roles = bind_json(&mut builder, "actor_roles", &actor_roles)?;
    let action = bind_text(&mut builder, "action", &event.action)?;
    let outcome = bind_text(&mut builder, "outcome", &event.outcome)?;
    let schema_name_value = bind_text(&mut builder, "schema_name", &event.schema_name)?;
    let entity_name = bind_text(&mut builder, "entity_name", &event.entity_name)?;
    let table_name_value = bind_text(&mut builder, "table_name", &event.table_name)?;
    let audit_table_name_value =
        bind_text(&mut builder, "audit_table_name", &event.audit_table_name)?;
    let record_id = bind_maybe_text(&mut builder, "record_id", &event.record_id)?;
    let before_json = bind_maybe_json(&mut builder, "before_json", &event.before_json)?;
    let after_json = bind_maybe_json(&mut builder, "after_json", &event.after_json)?;
    let diff_json = bind_json(&mut builder, "diff_json", &event.diff_json)?;
    let policy_json = bind_maybe_json(&mut builder, "policy_json", &event.policy_json)?;
    let redactions_json = bind_json(&mut builder, "redactions_json", &event.redactions_json)?;
    let chain_scope = bind_text(&mut builder, "chain_scope", &event.chain_scope)?;
    let prev_hash = bind_maybe_text(&mut builder, "prev_hash", &event.prev_hash)?;
    let event_hash = bind_text(&mut builder, "event_hash", &event.event_hash)?;
    let signature = bind_maybe_text(&mut builder, "signature", &event.signature)?;
    let sql = format!(
        r#"
            INSERT INTO {schema}.{table} (
                "audit_id",
                "occurred_at",
                "tenant_id",
                "actor_user_name",
                "actor_roles",
                "action",
                "outcome",
                "schema_name",
                "entity_name",
                "table_name",
                "audit_table_name",
                "record_id",
                "before_json",
                "after_json",
                "diff_json",
                "policy_json",
                "redactions_json",
                "chain_scope",
                "prev_hash",
                "event_hash",
                "signature"
            )
            SELECT
                {audit_id},
                {occurred_at},
                {tenant_id},
                {actor_user_name},
                {actor_roles},
                {action},
                {outcome},
                {schema_name_value},
                {entity_name},
                {table_name_value},
                {audit_table_name_value},
                {record_id},
                {before_json},
                {after_json},
                {diff_json},
                {policy_json},
                {redactions_json},
                {chain_scope},
                {prev_hash},
                {event_hash},
                {signature}
            "#,
        schema = quote_ident(&event.schema_name),
        table = quote_ident(&event.audit_table_name),
        audit_id = audit_id,
        occurred_at = occurred_at,
        tenant_id = tenant_id,
        actor_user_name = actor_user_name,
        actor_roles = actor_roles,
        action = action,
        outcome = outcome,
        schema_name_value = schema_name_value,
        entity_name = entity_name,
        table_name_value = table_name_value,
        audit_table_name_value = audit_table_name_value,
        record_id = record_id,
        before_json = before_json,
        after_json = after_json,
        diff_json = diff_json,
        policy_json = policy_json,
        redactions_json = redactions_json,
        chain_scope = chain_scope,
        prev_hash = prev_hash,
        event_hash = event_hash,
        signature = signature,
    );
    Ok(builder.finish(sql))
}

pub fn query_statement(query: &RuntimeAuditQuery) -> Result<SnowflakeStatement, RuntimeError> {
    let mut builder = SnowflakeStatementBuilder::new();
    let tenant_id = builder.bind_scalar(
        RuntimeDataType::String,
        "tenant_id",
        true,
        Value::String(query.tenant_id.clone()),
    )?;
    let record_id = builder.bind_scalar(
        RuntimeDataType::String,
        "record_id",
        true,
        Value::String(query.record_id.clone()),
    )?;
    let sql = format!(
        r#"
            SELECT
                "audit_id" AS audit_id,
                "occurred_at" AS occurred_at,
                "tenant_id" AS tenant_id,
                "actor_user_name" AS actor_user_name,
                "actor_roles" AS actor_roles,
                "action" AS action,
                "outcome" AS outcome,
                "schema_name" AS schema_name,
                "entity_name" AS entity_name,
                "table_name" AS table_name,
                "audit_table_name" AS audit_table_name,
                "record_id" AS record_id,
                "before_json" AS before_json,
                "after_json" AS after_json,
                "diff_json" AS diff_json,
                "policy_json" AS policy_json,
                "redactions_json" AS redactions_json,
                "chain_scope" AS chain_scope,
                "prev_hash" AS prev_hash,
                "event_hash" AS event_hash,
                "signature" AS signature
            FROM {schema}.{table}
            WHERE "tenant_id" = {tenant_id} AND "record_id" = {record_id}
            ORDER BY "occurred_at" DESC, "audit_id" DESC
            LIMIT {limit}
            "#,
        schema = quote_ident(&query.schema_name),
        table = quote_ident(&query.audit_table_name),
        tenant_id = tenant_id,
        record_id = record_id,
        limit = query.limit,
    );
    Ok(builder.finish(sql))
}

fn bind_text(
    builder: &mut SnowflakeStatementBuilder,
    name: &str,
    value: &str,
) -> Result<String, RuntimeError> {
    builder.bind_scalar(
        RuntimeDataType::String,
        name,
        true,
        Value::String(value.to_string()),
    )
}

fn bind_maybe_text(
    builder: &mut SnowflakeStatementBuilder,
    name: &str,
    value: &Option<String>,
) -> Result<String, RuntimeError> {
    match value {
        Some(value) => bind_text(builder, name, value),
        None => Ok("NULL".to_string()),
    }
}

fn bind_json(
    builder: &mut SnowflakeStatementBuilder,
    name: &str,
    value: &Value,
) -> Result<String, RuntimeError> {
    builder.bind_scalar(RuntimeDataType::Json, name, true, value.clone())
}

fn bind_maybe_json(
    builder: &mut SnowflakeStatementBuilder,
    name: &str,
    value: &Option<Value>,
) -> Result<String, RuntimeError> {
    match value {
        Some(value) => bind_json(builder, name, value),
        None => Ok("NULL".to_string()),
    }
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use serde_json::json;

    fn event() -> RuntimeAuditEvent {
        RuntimeAuditEvent {
            audit_id: "audit-1".to_string(),
            occurred_at: DateTime::parse_from_rfc3339("2026-05-30T00:00:00Z")
                .expect("datetime")
                .with_timezone(&Utc),
            tenant_id: Some("tenant-1".to_string()),
            actor_user_name: "user@example.com".to_string(),
            actor_roles: vec!["admin".to_string()],
            action: "update".to_string(),
            outcome: "succeeded".to_string(),
            schema_name: "crm".to_string(),
            entity_name: "Account".to_string(),
            table_name: "accounts".to_string(),
            audit_table_name: "accounts_audit".to_string(),
            record_id: Some("account-1".to_string()),
            before_json: Some(json!({"name": "Old"})),
            after_json: Some(json!({"name": "New"})),
            diff_json: json!({"name": ["Old", "New"]}),
            policy_json: Some(json!({"allow": true})),
            redactions_json: json!([]),
            chain_scope: "crm:Account:tenant-1:account-1".to_string(),
            prev_hash: Some("prev".to_string()),
            event_hash: "hash".to_string(),
            signature: None,
        }
    }

    #[test]
    fn previous_hash_statement_binds_chain_scope() {
        let statement = previous_hash_statement(&event()).expect("previous hash statement");
        let body = statement.body(30, "app", None, None);

        assert!(statement.sql().contains("FROM \"crm\".\"accounts_audit\""));
        assert!(statement.sql().contains("WHERE \"chain_scope\" = ?"));
        assert_eq!(body["bindings"].as_object().map(|b| b.len()), Some(1));
    }

    #[test]
    fn insert_statement_binds_audit_columns() {
        let statement = insert_statement(&event()).expect("insert statement");
        let body = statement.body(30, "app", None, None);

        assert!(statement
            .sql()
            .contains("INSERT INTO \"crm\".\"accounts_audit\""));
        assert!(statement.sql().contains("\"actor_roles\""));
        assert!(statement.sql().contains("PARSE_JSON(?)"));
        assert_eq!(body["bindings"].as_object().map(|b| b.len()), Some(20));
    }

    #[test]
    fn query_statement_binds_tenant_record_id_and_limits() {
        let query = RuntimeAuditQuery::new(
            "crm",
            "Account",
            "accounts_audit",
            "tenant-a",
            "account-1",
            5,
        );

        let statement = query_statement(&query).expect("query statement");
        let body = statement.body(30, "app", None, None);

        assert!(statement
            .sql()
            .contains("WHERE \"tenant_id\" = ? AND \"record_id\" = ?"));
        assert!(statement.sql().contains("LIMIT 5"));
        assert_eq!(body["bindings"].as_object().map(|b| b.len()), Some(2));
    }
}
