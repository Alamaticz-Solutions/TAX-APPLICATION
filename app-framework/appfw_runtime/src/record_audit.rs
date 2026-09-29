use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use crate::{
    extension::UserAuth,
    model_metadata::{RuntimeEntityMetadata, RuntimePropertyMetadata},
    AccessAction, PolicyAccess, RuntimeError,
};

const ANONYMOUS_ACTOR: &str = "anonymous";
const OUTCOME_SUCCEEDED: &str = "succeeded";
const MIN_AUDIT_QUERY_LIMIT: i64 = 1;
const MAX_AUDIT_QUERY_LIMIT: i64 = 100;

pub struct RedactedAuditPayload {
    pub before_json: Option<Value>,
    pub after_json: Option<Value>,
    pub redactions_json: Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeAuditQuery {
    pub schema_name: String,
    pub entity_name: String,
    pub audit_table_name: String,
    pub tenant_id: String,
    pub record_id: String,
    pub limit: i64,
}

impl RuntimeAuditQuery {
    pub fn new(
        schema_name: impl Into<String>,
        entity_name: impl Into<String>,
        audit_table_name: impl Into<String>,
        tenant_id: impl Into<String>,
        record_id: impl Into<String>,
        limit: i64,
    ) -> Self {
        Self {
            schema_name: schema_name.into(),
            entity_name: entity_name.into(),
            audit_table_name: audit_table_name.into(),
            tenant_id: tenant_id.into(),
            record_id: record_id.into(),
            limit: audit_query_limit(limit),
        }
    }

    pub fn for_entity(
        entity: &RuntimeEntityMetadata,
        tenant_id: impl Into<String>,
        record_id: impl Into<String>,
        limit: i64,
    ) -> Self {
        audit_query(entity, tenant_id, record_id, limit)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeAuditEvent {
    pub audit_id: String,
    pub occurred_at: DateTime<Utc>,
    pub tenant_id: Option<String>,
    pub actor_user_name: String,
    pub actor_roles: Vec<String>,
    pub action: String,
    pub outcome: String,
    pub schema_name: String,
    pub entity_name: String,
    pub table_name: String,
    pub audit_table_name: String,
    pub record_id: Option<String>,
    pub before_json: Option<Value>,
    pub after_json: Option<Value>,
    pub diff_json: Value,
    pub policy_json: Option<Value>,
    pub redactions_json: Value,
    pub chain_scope: String,
    pub prev_hash: Option<String>,
    pub event_hash: String,
    pub signature: Option<String>,
}

impl RuntimeAuditEvent {
    pub fn entity_mutation(
        entity: &RuntimeEntityMetadata,
        action: AccessAction,
        user: &UserAuth,
        record_id: Option<String>,
        before_json: Option<Value>,
        after_json: Option<Value>,
        access: &PolicyAccess,
    ) -> Self {
        Self::entity_attempt(
            entity,
            action,
            Some(user),
            OUTCOME_SUCCEEDED,
            record_id,
            before_json,
            after_json,
            Some(policy_json(access)),
        )
    }

    pub fn entity_attempt(
        entity: &RuntimeEntityMetadata,
        action: AccessAction,
        user: Option<&UserAuth>,
        outcome: &str,
        record_id: Option<String>,
        before_json: Option<Value>,
        after_json: Option<Value>,
        policy_json: Option<Value>,
    ) -> Self {
        Self::entity_event(
            entity,
            action.to_string(),
            user,
            outcome,
            record_id,
            before_json,
            after_json,
            policy_json,
        )
    }

    pub fn entity_event(
        entity: &RuntimeEntityMetadata,
        action: impl Into<String>,
        user: Option<&UserAuth>,
        outcome: &str,
        record_id: Option<String>,
        before_json: Option<Value>,
        after_json: Option<Value>,
        policy_json: Option<Value>,
    ) -> Self {
        let table_name = entity.snake_n.clone();
        let audit_table_name = audit_table_name(entity);
        let tenant_id =
            user.and_then(|user| non_empty(user.tenant_id.as_str()).map(str::to_string));
        let actor_user_name = user
            .map(|user| user.user_name.clone())
            .unwrap_or_else(|| ANONYMOUS_ACTOR.to_string());
        let actor_roles = user.map(|user| user.roles.clone()).unwrap_or_default();
        let redacted = redact_payloads(entity, before_json, after_json);
        let chain_scope = chain_scope(entity, tenant_id.as_deref(), record_id.as_deref());

        Self {
            audit_id: uuid::Uuid::new_v4().to_string(),
            occurred_at: Utc::now(),
            tenant_id,
            actor_user_name,
            actor_roles,
            action: action.into(),
            outcome: outcome.to_string(),
            schema_name: entity.schema_name.clone(),
            entity_name: entity.pascal_1.clone(),
            table_name,
            audit_table_name,
            record_id,
            diff_json: diff(redacted.before_json.as_ref(), redacted.after_json.as_ref()),
            before_json: redacted.before_json,
            after_json: redacted.after_json,
            policy_json,
            redactions_json: redacted.redactions_json,
            chain_scope,
            prev_hash: None,
            event_hash: String::new(),
            signature: None,
        }
    }

    pub fn finalize(mut self, prev_hash: Option<String>) -> Result<Self, RuntimeError> {
        self.prev_hash = prev_hash;
        self.event_hash = self.compute_hash()?;
        Ok(self)
    }

    pub fn continue_record_chain(self, last_event: Option<&Value>) -> Self {
        continue_record_chain(self, last_event)
    }

    fn compute_hash(&self) -> Result<String, RuntimeError> {
        let mut payload = self.clone();
        payload.event_hash.clear();
        payload.signature = None;
        compute_hash(&payload)
    }
}

pub fn is_audited(entity: &RuntimeEntityMetadata) -> bool {
    entity.facets.iter().any(|facet| facet == "audited")
}

pub fn audit_table_name(entity: &RuntimeEntityMetadata) -> String {
    format!("{}_audit", entity.snake_n)
}

pub fn audit_query(
    entity: &RuntimeEntityMetadata,
    tenant_id: impl Into<String>,
    record_id: impl Into<String>,
    limit: i64,
) -> RuntimeAuditQuery {
    RuntimeAuditQuery::new(
        entity.schema_name.clone(),
        entity.pascal_1.clone(),
        audit_table_name(entity),
        tenant_id,
        record_id,
        limit,
    )
}

pub fn audit_query_limit(limit: i64) -> i64 {
    limit.clamp(MIN_AUDIT_QUERY_LIMIT, MAX_AUDIT_QUERY_LIMIT)
}

pub fn continue_record_chain(
    event: RuntimeAuditEvent,
    last_event: Option<&Value>,
) -> RuntimeAuditEvent {
    if let Some(last_event) = last_event.and_then(Value::as_object) {
        let same_tenant = audit_str(last_event, "tenant_id") == event.tenant_id.as_deref();
        let same_chain = audit_str(last_event, "chain_scope") == Some(event.chain_scope.as_str());
        if !same_tenant || !same_chain {
            return event;
        }
    }
    event
}

fn audit_str<'a>(event: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    event
        .get(key)
        .or_else(|| event.get(&key.to_ascii_uppercase()))
        .and_then(Value::as_str)
}

pub fn audit_selection(entity: &RuntimeEntityMetadata) -> Value {
    let selection_set = entity
        .properties
        .iter()
        .filter(|prop| prop.is_native_storage())
        .map(|prop| json!({ "name": prop.name, "selection_set": [] }))
        .collect::<Vec<_>>();

    json!({
        "name": entity.snake_n,
        "selection_set": selection_set
    })
}

pub fn redact_record_for_external_response(
    entity: &RuntimeEntityMetadata,
    record: Map<String, Value>,
) -> Map<String, Value> {
    let mut redacted_properties = BTreeSet::new();
    match redact_record_value(entity, Value::Object(record), &mut redacted_properties) {
        Value::Object(record) => record,
        _ => Map::new(),
    }
}

pub fn record_id(entity: &RuntimeEntityMetadata, record: &Map<String, Value>) -> Option<String> {
    entity
        .primary_key()
        .and_then(|prop| record.get(&prop.name))
        .and_then(value_to_string)
}

pub fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

pub fn policy_json(access: &PolicyAccess) -> Value {
    json!({
        "allow": access.allow,
        "filter": access.filter,
    })
}

pub fn policy_decision_json(access: &PolicyAccess, reason: &str) -> Value {
    json!({
        "allow": access.allow,
        "filter": access.filter,
        "decision": {
            "reason": reason,
        },
    })
}

pub fn policy_error_json(reason: &str, error: impl ToString) -> Value {
    json!({
        "allow": false,
        "filter": null,
        "decision": {
            "reason": reason,
            "error": error.to_string(),
        },
    })
}

pub fn operation_error_json(reason: &str, error: impl ToString) -> Value {
    json!({
        "decision": {
            "reason": reason,
            "error": error.to_string(),
        },
    })
}

pub fn operation_decision_json(reason: &str) -> Value {
    json!({
        "decision": {
            "reason": reason,
        },
    })
}

pub fn chain_scope(
    entity: &RuntimeEntityMetadata,
    tenant_id: Option<&str>,
    record_id: Option<&str>,
) -> String {
    let mut scope = format!("{}.{}", entity.schema_name, entity.pascal_1);
    if let Some(tenant_id) = tenant_id.and_then(non_empty) {
        scope.push(':');
        scope.push_str(tenant_id);
    }
    if let Some(record_id) = record_id.and_then(non_empty) {
        scope.push(':');
        scope.push_str(record_id);
    }
    scope
}

pub fn redact_payloads(
    entity: &RuntimeEntityMetadata,
    before_json: Option<Value>,
    after_json: Option<Value>,
) -> RedactedAuditPayload {
    let mut redacted_properties = BTreeSet::new();
    let before_json =
        before_json.map(|value| redact_record_value(entity, value, &mut redacted_properties));
    let after_json =
        after_json.map(|value| redact_record_value(entity, value, &mut redacted_properties));

    RedactedAuditPayload {
        before_json,
        after_json,
        redactions_json: json!({
            "omitted_actor_fields": ["token"],
            "redacted_properties": redacted_properties.into_iter().collect::<Vec<_>>(),
            "strategy": "replace_value",
        }),
    }
}

pub fn diff(before: Option<&Value>, after: Option<&Value>) -> Value {
    let before_obj = before.and_then(Value::as_object);
    let after_obj = after.and_then(Value::as_object);
    let mut result = Map::new();

    let mut keys = before_obj
        .into_iter()
        .flat_map(|obj| obj.keys().cloned())
        .collect::<Vec<_>>();
    keys.extend(after_obj.into_iter().flat_map(|obj| obj.keys().cloned()));
    keys.sort();
    keys.dedup();

    for key in keys {
        let before_value = before
            .and_then(Value::as_object)
            .and_then(|obj| obj.get(&key))
            .cloned()
            .unwrap_or(Value::Null);
        let after_value = after
            .and_then(Value::as_object)
            .and_then(|obj| obj.get(&key))
            .cloned()
            .unwrap_or(Value::Null);
        if before_value != after_value {
            result.insert(
                key,
                json!({
                    "before": before_value,
                    "after": after_value,
                }),
            );
        }
    }

    Value::Object(result)
}

pub fn compute_hash<T>(payload: &T) -> Result<String, RuntimeError>
where
    T: Serialize,
{
    let payload = canonicalize_value(
        serde_json::to_value(payload).map_err(|err| RuntimeError::DataAccess(err.to_string()))?,
    );
    let bytes =
        serde_json::to_vec(&payload).map_err(|err| RuntimeError::DataAccess(err.to_string()))?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn redact_record_value(
    entity: &RuntimeEntityMetadata,
    value: Value,
    redacted_properties: &mut BTreeSet<String>,
) -> Value {
    let Value::Object(mut obj) = value else {
        return value;
    };

    for prop in &entity.properties {
        if obj.contains_key(&prop.name) && should_redact_property(prop) {
            obj.insert(prop.name.clone(), json!({ "_redacted": true }));
            redacted_properties.insert(prop.name.clone());
        }
    }

    Value::Object(obj)
}

fn should_redact_property(prop: &RuntimePropertyMetadata) -> bool {
    meta_requests_redaction(prop.meta.as_ref()) || name_looks_sensitive(&prop.name)
}

fn meta_requests_redaction(meta: Option<&Value>) -> bool {
    let Some(meta) = meta else {
        return false;
    };

    bool_at(meta, &["audit", "redact"])
        || bool_at(meta, &["audit", "omit"])
        || bool_at(meta, &["sensitive"])
        || bool_at(meta, &["pii"])
        || bool_at(meta, &["secret"])
        || string_at(meta, &["audit", "classification"])
            .map(|classification| {
                matches!(
                    classification,
                    "confidential" | "restricted" | "secret" | "sensitive"
                )
            })
            .unwrap_or(false)
}

fn bool_at(value: &Value, path: &[&str]) -> bool {
    path.iter()
        .try_fold(value, |curr, part| curr.get(*part))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn string_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    path.iter()
        .try_fold(value, |curr, part| curr.get(*part))
        .and_then(Value::as_str)
}

fn name_looks_sensitive(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    [
        "access_key",
        "api_key",
        "credential",
        "password",
        "private_key",
        "refresh_token",
        "secret",
        "social_security",
        "ssn",
        "token",
    ]
    .iter()
    .any(|marker| name.contains(marker))
}

fn canonicalize_value(value: Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.into_iter().map(canonicalize_value).collect()),
        Value::Object(map) => {
            let mut entries = map.into_iter().collect::<Vec<_>>();
            entries.sort_by(|(left, _), (right, _)| left.cmp(right));
            Value::Object(
                entries
                    .into_iter()
                    .map(|(key, value)| (key, canonicalize_value(value)))
                    .collect(),
            )
        }
        value => value,
    }
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::model_metadata::{RuntimeDataType, RuntimeEntityMetadata, RuntimePropertyMetadata};

    fn entity() -> RuntimeEntityMetadata {
        RuntimeEntityMetadata {
            id: "entity-1".to_string(),
            schema_name: "crm".to_string(),
            schema_id: None,
            pascal_1: "Account".to_string(),
            pascal_n: "Accounts".to_string(),
            snake_1: "account".to_string(),
            snake_n: "accounts".to_string(),
            caption_1: "Account".to_string(),
            caption_n: "Accounts".to_string(),
            is_union: false,
            base_type: None,
            is_table: true,
            facets: vec!["audited".to_string()],
            meta: None,
            standard_methods: Vec::new(),
            custom_methods: Vec::new(),
            properties: vec![
                prop("id", true, None),
                prop("name", false, None),
                prop("api_token", false, None),
                prop(
                    "email",
                    false,
                    Some(json!({
                        "audit": {
                            "redact": true
                        }
                    })),
                ),
            ],
        }
    }

    fn prop(name: &str, is_key: bool, meta: Option<Value>) -> RuntimePropertyMetadata {
        RuntimePropertyMetadata {
            id: format!("prop-{name}"),
            name: name.to_string(),
            caption: name.to_string(),
            data_type: RuntimeDataType::String,
            is_key,
            is_caption: false,
            is_required: false,
            is_read_only: false,
            is_concurrency_control: false,
            default_value: None,
            foreign_key: None,
            nav_by_fk: None,
            many_to_many: None,
            nested_entity_type: None,
            enum_type_name: None,
            meta,
        }
    }

    #[test]
    fn redaction_replaces_sensitive_properties_before_diffing() {
        let entity = entity();
        let redacted = redact_payloads(
            &entity,
            Some(json!({
                "id": "account-1",
                "name": "Old",
                "api_token": "old-token",
                "email": "old@example.com"
            })),
            Some(json!({
                "id": "account-1",
                "name": "New",
                "api_token": "new-token",
                "email": "new@example.com"
            })),
        );

        assert_eq!(
            redacted.before_json.as_ref().unwrap()["api_token"],
            json!({ "_redacted": true })
        );
        assert_eq!(
            redacted.after_json.as_ref().unwrap()["email"],
            json!({ "_redacted": true })
        );
        let diff = diff(redacted.before_json.as_ref(), redacted.after_json.as_ref());
        assert_eq!(diff["name"]["before"], json!("Old"));
        assert!(diff.get("api_token").is_none());
        assert_eq!(
            redacted.redactions_json["redacted_properties"][0],
            "api_token"
        );
        assert_eq!(redacted.redactions_json["redacted_properties"][1], "email");
    }

    #[test]
    fn audit_helpers_are_metadata_driven() {
        let entity = entity();
        let record = json!({ "id": "account-1", "name": "Acme" })
            .as_object()
            .expect("object")
            .clone();

        assert!(is_audited(&entity));
        assert_eq!(record_id(&entity, &record), Some("account-1".to_string()));
        assert_eq!(
            chain_scope(&entity, Some("tenant-1"), Some("account-1")),
            "crm.Account:tenant-1:account-1"
        );
        assert_eq!(
            audit_selection(&entity),
            json!({
                "name": "accounts",
                "selection_set": [
                    { "name": "id", "selection_set": [] },
                    { "name": "name", "selection_set": [] },
                    { "name": "api_token", "selection_set": [] },
                    { "name": "email", "selection_set": [] }
                ]
            })
        );
    }

    #[test]
    fn policy_and_operation_evidence_json_is_stable() {
        let access = PolicyAccess::allow_with_filter(json!({ "tenant_id": { "_eq": "tenant-1" } }));

        assert_eq!(
            policy_decision_json(&access, "policy_denied"),
            json!({
                "allow": true,
                "filter": { "tenant_id": { "_eq": "tenant-1" } },
                "decision": { "reason": "policy_denied" }
            })
        );
        assert_eq!(
            operation_error_json("mutation_failed", "boom"),
            json!({ "decision": { "reason": "mutation_failed", "error": "boom" } })
        );
    }

    #[test]
    fn runtime_audit_event_builds_and_finalizes_stable_payload() {
        let entity = entity();
        let user = UserAuth::human(
            "tenant-1",
            "alex",
            "UTC",
            vec!["admin".to_string()],
            Vec::new(),
            "do-not-store",
        );
        let access = PolicyAccess::allow_all();

        let event = RuntimeAuditEvent::entity_mutation(
            &entity,
            AccessAction::Update,
            &user,
            Some("account-1".to_string()),
            Some(json!({ "id": "account-1", "api_token": "old" })),
            Some(json!({ "id": "account-1", "api_token": "new", "name": "Acme" })),
            &access,
        );

        assert_eq!(event.schema_name, "crm");
        assert_eq!(event.entity_name, "Account");
        assert_eq!(event.table_name, "accounts");
        assert_eq!(event.audit_table_name, "accounts_audit");
        assert_eq!(event.chain_scope, "crm.Account:tenant-1:account-1");
        assert_eq!(
            event.after_json.as_ref().unwrap()["api_token"],
            json!({ "_redacted": true })
        );

        let finalized = event
            .finalize(Some("previous-hash".to_string()))
            .expect("finalized event");
        assert_eq!(finalized.prev_hash.as_deref(), Some("previous-hash"));
        assert!(!finalized.event_hash.is_empty());
    }

    #[test]
    fn runtime_audit_query_uses_entity_topology_and_normalizes_limit() {
        let query = audit_query(&entity(), "tenant-1", "account-1", 250);

        assert_eq!(query.schema_name, "crm");
        assert_eq!(query.entity_name, "Account");
        assert_eq!(query.audit_table_name, "accounts_audit");
        assert_eq!(query.tenant_id, "tenant-1");
        assert_eq!(query.record_id, "account-1");
        assert_eq!(query.limit, 100);

        assert_eq!(
            RuntimeAuditQuery::for_entity(&entity(), "tenant-1", "account-2", 0).limit,
            1
        );
    }

    #[test]
    fn runtime_audit_event_can_continue_existing_record_chain() {
        let user = UserAuth::human(
            "tenant-new",
            "alex",
            "UTC",
            vec!["admin".to_string()],
            Vec::new(),
            "do-not-store",
        );
        let event = RuntimeAuditEvent::entity_mutation(
            &entity(),
            AccessAction::Update,
            &user,
            Some("account-1".to_string()),
            None,
            Some(json!({ "id": "account-1" })),
            &PolicyAccess::allow_all(),
        );
        let last_event = json!({
            "tenant_id": "tenant-b-sentinel",
            "chain_scope": "crm.Account:tenant-b-sentinel:account-1"
        });

        let continued = event.continue_record_chain(Some(&last_event));

        assert_eq!(continued.tenant_id.as_deref(), Some("tenant-new"));
        assert_eq!(continued.chain_scope, "crm.Account:tenant-new:account-1");

        let uppercase_last_event = json!({
            "TENANT_ID": "tenant-b-sentinel",
            "CHAIN_SCOPE": "crm.Account:tenant-b-sentinel:account-1"
        });
        let continued = RuntimeAuditEvent::entity_mutation(
            &entity(),
            AccessAction::Update,
            &user,
            Some("account-1".to_string()),
            None,
            Some(json!({ "id": "account-1" })),
            &PolicyAccess::allow_all(),
        )
        .continue_record_chain(Some(&uppercase_last_event));

        assert_eq!(continued.tenant_id.as_deref(), Some("tenant-new"));
        assert_eq!(continued.chain_scope, "crm.Account:tenant-new:account-1");
    }
}
