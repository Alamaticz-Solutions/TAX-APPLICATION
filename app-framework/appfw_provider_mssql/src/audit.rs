use appfw_runtime::{RuntimeAuditEvent, RuntimeAuditQuery, RuntimeError};

use crate::SqlParam;

pub fn previous_hash_statement(event: &RuntimeAuditEvent) -> (String, Vec<SqlParam>) {
    (
        format!(
            r#"
            SELECT TOP 1 [event_hash]
            FROM [{schema}].[{table}]
            WHERE [chain_scope] = @P1
            ORDER BY [occurred_at] DESC, [audit_id] DESC
            "#,
            schema = event.schema_name,
            table = event.audit_table_name
        ),
        vec![text(event.chain_scope.clone())],
    )
}

pub fn insert_statement(
    event: &RuntimeAuditEvent,
) -> Result<(String, Vec<SqlParam>), RuntimeError> {
    let sql = format!(
        r#"
            INSERT INTO [{schema}].[{table}] (
                [audit_id],
                [occurred_at],
                [tenant_id],
                [actor_user_name],
                [actor_roles],
                [action],
                [outcome],
                [schema_name],
                [entity_name],
                [table_name],
                [audit_table_name],
                [record_id],
                [before_json],
                [after_json],
                [diff_json],
                [policy_json],
                [redactions_json],
                [chain_scope],
                [prev_hash],
                [event_hash],
                [signature]
            )
            VALUES (
                @P1, @P2, @P3, @P4, @P5, @P6, @P7, @P8, @P9, @P10,
                @P11, @P12, @P13, @P14, @P15, @P16, @P17, @P18, @P19, @P20, @P21
            )
            "#,
        schema = event.schema_name,
        table = event.audit_table_name
    );

    Ok((
        sql,
        vec![
            text(event.audit_id.clone()),
            text(event.occurred_at.to_rfc3339()),
            maybe_text(event.tenant_id.clone()),
            text(event.actor_user_name.clone()),
            json_text(
                &serde_json::to_value(&event.actor_roles)
                    .map_err(|e| RuntimeError::DataAccess(e.to_string()))?,
            )?,
            text(event.action.clone()),
            text(event.outcome.clone()),
            text(event.schema_name.clone()),
            text(event.entity_name.clone()),
            text(event.table_name.clone()),
            text(event.audit_table_name.clone()),
            maybe_text(event.record_id.clone()),
            maybe_json_text(&event.before_json)?,
            maybe_json_text(&event.after_json)?,
            json_text(&event.diff_json)?,
            maybe_json_text(&event.policy_json)?,
            json_text(&event.redactions_json)?,
            text(event.chain_scope.clone()),
            maybe_text(event.prev_hash.clone()),
            text(event.event_hash.clone()),
            maybe_text(event.signature.clone()),
        ],
    ))
}

pub fn query_statement(query: &RuntimeAuditQuery) -> (String, Vec<SqlParam>) {
    (
        format!(
            r#"
            SELECT COALESCE((
                SELECT TOP ({limit})
                    [audit_id],
                    [occurred_at],
                    [tenant_id],
                    [actor_user_name],
                    JSON_QUERY([actor_roles]) AS [actor_roles],
                    [action],
                    [outcome],
                    [schema_name],
                    [entity_name],
                    [table_name],
                    [audit_table_name],
                    [record_id],
                    JSON_QUERY([before_json]) AS [before_json],
                    JSON_QUERY([after_json]) AS [after_json],
                    JSON_QUERY([diff_json]) AS [diff_json],
                    JSON_QUERY([policy_json]) AS [policy_json],
                    JSON_QUERY([redactions_json]) AS [redactions_json],
                    [chain_scope],
                    [prev_hash],
                    [event_hash],
                    [signature]
                FROM [{schema}].[{table}]
                WHERE [tenant_id] = @P1 AND [record_id] = @P2
                ORDER BY [occurred_at] DESC, [audit_id] DESC
                FOR JSON PATH
            ), '[]') AS [events]
            "#,
            schema = query.schema_name,
            table = query.audit_table_name,
            limit = query.limit,
        ),
        vec![text(query.tenant_id.clone()), text(query.record_id.clone())],
    )
}

fn text(value: impl Into<String>) -> SqlParam {
    SqlParam::String(Some(value.into()))
}

fn maybe_text(value: Option<String>) -> SqlParam {
    SqlParam::String(value)
}

fn json_text(value: &serde_json::Value) -> Result<SqlParam, RuntimeError> {
    Ok(text(
        serde_json::to_string(value).map_err(|e| RuntimeError::DataAccess(e.to_string()))?,
    ))
}

fn maybe_json_text(value: &Option<serde_json::Value>) -> Result<SqlParam, RuntimeError> {
    value
        .as_ref()
        .map(json_text)
        .transpose()
        .map(|value| value.unwrap_or(SqlParam::String(None)))
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
        let (sql, params) = previous_hash_statement(&event());

        assert!(sql.contains("FROM [crm].[accounts_audit]"));
        assert!(sql.contains("WHERE [chain_scope] = @P1"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn insert_statement_binds_audit_columns() {
        let (sql, params) = insert_statement(&event()).expect("insert statement");

        assert!(sql.contains("INSERT INTO [crm].[accounts_audit]"));
        assert!(sql.contains("@P21"));
        assert_eq!(params.len(), 21);
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

        let (sql, params) = query_statement(&query);

        assert!(sql.contains("WHERE [tenant_id] = @P1 AND [record_id] = @P2"));
        assert!(sql.contains("SELECT TOP (5)"));
        assert_eq!(params.len(), 2);
    }
}
