use appfw_runtime::{RuntimeAuditEvent, RuntimeAuditQuery, RuntimeError};

use crate::SqlParam;

pub fn previous_hash_statement(event: &RuntimeAuditEvent) -> (String, Vec<SqlParam>) {
    let table = table_ref(&event.schema_name, &event.audit_table_name);
    (
        format!(
            r#"
            SELECT event_hash
            FROM {table}
            WHERE chain_scope = $1
            ORDER BY occurred_at DESC, audit_id DESC
            LIMIT 1
            "#
        ),
        vec![Box::new(event.chain_scope.clone())],
    )
}

pub fn insert_statement(
    event: &RuntimeAuditEvent,
) -> Result<(String, Vec<SqlParam>), RuntimeError> {
    let table = table_ref(&event.schema_name, &event.audit_table_name);
    let actor_roles = serde_json::to_value(&event.actor_roles)
        .map_err(|e| RuntimeError::DataAccess(e.to_string()))?;
    let sql = format!(
        r#"
            INSERT INTO {table} (
                audit_id,
                occurred_at,
                tenant_id,
                actor_user_name,
                actor_roles,
                action,
                outcome,
                schema_name,
                entity_name,
                table_name,
                audit_table_name,
                record_id,
                before_json,
                after_json,
                diff_json,
                policy_json,
                redactions_json,
                chain_scope,
                prev_hash,
                event_hash,
                signature
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21
            )
            "#
    );
    Ok((
        sql,
        vec![
            Box::new(event.audit_id.clone()),
            Box::new(event.occurred_at),
            Box::new(event.tenant_id.clone()),
            Box::new(event.actor_user_name.clone()),
            Box::new(actor_roles),
            Box::new(event.action.clone()),
            Box::new(event.outcome.clone()),
            Box::new(event.schema_name.clone()),
            Box::new(event.entity_name.clone()),
            Box::new(event.table_name.clone()),
            Box::new(event.audit_table_name.clone()),
            Box::new(event.record_id.clone()),
            Box::new(event.before_json.clone()),
            Box::new(event.after_json.clone()),
            Box::new(event.diff_json.clone()),
            Box::new(event.policy_json.clone()),
            Box::new(event.redactions_json.clone()),
            Box::new(event.chain_scope.clone()),
            Box::new(event.prev_hash.clone()),
            Box::new(event.event_hash.clone()),
            Box::new(event.signature.clone()),
        ],
    ))
}

pub fn query_statement(query: &RuntimeAuditQuery) -> (String, Vec<SqlParam>) {
    let table = table_ref(&query.schema_name, &query.audit_table_name);
    (
        format!(
            r#"
            SELECT COALESCE(json_agg(row_to_json(t)), '[]'::json) AS events
            FROM (
                SELECT
                    audit_id,
                    occurred_at,
                    tenant_id,
                    actor_user_name,
                    actor_roles,
                    action,
                    outcome,
                    schema_name,
                    entity_name,
                    table_name,
                    audit_table_name,
                    record_id,
                    before_json,
                    after_json,
                    diff_json,
                    policy_json,
                    redactions_json,
                    chain_scope,
                    prev_hash,
                    event_hash,
                    signature
                FROM {table}
                WHERE tenant_id = $1 AND record_id = $2
                ORDER BY occurred_at DESC, audit_id DESC
                LIMIT $3
            ) t
            "#
        ),
        vec![
            Box::new(query.tenant_id.clone()),
            Box::new(query.record_id.clone()),
            Box::new(query.limit),
        ],
    )
}

fn table_ref(schema: &str, table: &str) -> String {
    format!("{schema}.{table}")
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

        assert!(sql.contains("FROM crm.accounts_audit"));
        assert!(sql.contains("WHERE chain_scope = $1"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn insert_statement_binds_audit_columns() {
        let (sql, params) = insert_statement(&event()).expect("insert statement");

        assert!(sql.contains("INSERT INTO crm.accounts_audit"));
        assert!(sql.contains("$21"));
        assert_eq!(params.len(), 21);
    }

    #[test]
    fn query_statement_binds_tenant_record_id_and_limit() {
        let query = RuntimeAuditQuery::new(
            "crm",
            "Account",
            "accounts_audit",
            "tenant-a",
            "account-1",
            5,
        );

        let (sql, params) = query_statement(&query);

        assert!(sql.contains("WHERE tenant_id = $1 AND record_id = $2"));
        assert!(sql.contains("LIMIT $3"));
        assert_eq!(params.len(), 3);
    }
}
