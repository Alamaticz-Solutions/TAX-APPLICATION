use appfw_runtime::{RuntimeAuditEvent, RuntimeAuditQuery, RuntimeError};
use bson::{doc, Bson, Document};
use serde_json::Value;

pub fn collection_name(schema_name: &str, audit_table_name: &str) -> String {
    format!("{schema_name}.{audit_table_name}")
}

pub fn previous_hash_filter(event: &RuntimeAuditEvent) -> Document {
    doc! { "chain_scope": &event.chain_scope }
}

pub fn query_filter(query: &RuntimeAuditQuery) -> Document {
    doc! {
        "tenant_id": &query.tenant_id,
        "record_id": &query.record_id,
    }
}

pub fn audit_sort() -> Document {
    doc! { "occurred_at": -1, "audit_id": -1 }
}

pub fn query_limit(query: &RuntimeAuditQuery) -> i64 {
    query.limit
}

pub fn event_document(event: &RuntimeAuditEvent) -> Result<Document, RuntimeError> {
    bson::to_document(event).map_err(|e| RuntimeError::DataAccess(e.to_string()))
}

pub fn document_to_json(doc: Document) -> Result<Value, RuntimeError> {
    bson::from_bson::<Value>(Bson::Document(doc))
        .map_err(|e| RuntimeError::DataAccess(e.to_string()))
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
    fn audit_collection_and_filters_are_provider_owned() {
        let event = event();
        let query = RuntimeAuditQuery::new(
            "crm",
            "Account",
            "accounts_audit",
            "tenant-a",
            "account-1",
            5,
        );

        assert_eq!(
            collection_name("crm", "accounts_audit"),
            "crm.accounts_audit"
        );
        assert_eq!(
            previous_hash_filter(&event).get_str("chain_scope").ok(),
            Some("crm:Account:tenant-1:account-1")
        );
        assert_eq!(
            query_filter(&query).get_str("record_id").ok(),
            Some("account-1")
        );
        assert_eq!(
            query_filter(&query).get_str("tenant_id").ok(),
            Some("tenant-a")
        );
        assert_eq!(audit_sort().get_i32("occurred_at").ok(), Some(-1));
        assert_eq!(query_limit(&query), 5);
    }

    #[test]
    fn audit_event_round_trips_to_json() {
        let doc = event_document(&event()).expect("event document");

        assert_eq!(doc.get_str("audit_id").ok(), Some("audit-1"));
        let value = document_to_json(doc).expect("json value");
        assert_eq!(value["audit_id"], json!("audit-1"));
        assert_eq!(value["actor_roles"], json!(["admin"]));
    }
}
