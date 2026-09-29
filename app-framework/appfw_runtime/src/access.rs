use std::fmt;

use serde_json::Value;

use crate::ConfigError;

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessAction {
    Create,
    Read,
    Update,
    Delete,
}

impl AccessAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccessAction::Create => "create",
            AccessAction::Read => "read",
            AccessAction::Update => "update",
            AccessAction::Delete => "delete",
        }
    }
}

impl fmt::Display for AccessAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug)]
pub struct PolicyAccess {
    pub allow: bool,
    pub filter: Option<Value>,
    // pub fields: Option<Vec<String>,
}

impl PolicyAccess {
    pub fn allow_all() -> Self {
        Self {
            allow: true,
            filter: None,
        }
    }

    pub fn allow_with_filter(filter: Value) -> Self {
        Self {
            allow: true,
            filter: Some(filter),
        }
    }

    pub fn deny() -> Self {
        Self {
            allow: false,
            filter: None,
        }
    }

    pub fn and_filter(mut self, required: Option<Value>) -> Self {
        if self.allow {
            self.filter = combine_access_filters(self.filter, required);
        }
        self
    }

    pub fn from_policy_result(policy_key: &str, result: Value) -> Result<Self, ConfigError> {
        let access = result
            .as_object()
            .ok_or_else(|| ConfigError::InvalidPolicyResult {
                policy_key: policy_key.to_string(),
                message: "expected policy result object".to_string(),
            })?;

        let allow = access
            .get("allow")
            .and_then(|v| v.as_bool())
            .ok_or_else(|| ConfigError::InvalidPolicyResult {
                policy_key: policy_key.to_string(),
                message: "expected boolean `allow` field".to_string(),
            })?;
        let filter = match access.get("filter") {
            None | Some(Value::Null) => None,
            Some(filter @ Value::Object(_)) => Some(filter.clone()),
            Some(_) => {
                return Err(ConfigError::InvalidPolicyResult {
                    policy_key: policy_key.to_string(),
                    message: "expected object `filter` field when present".to_string(),
                });
            }
        };

        Ok(Self { allow, filter })
    }
}

pub fn combine_access_filters(existing: Option<Value>, required: Option<Value>) -> Option<Value> {
    match (existing, required) {
        (None, None) => None,
        (Some(filter), None) | (None, Some(filter)) => Some(filter),
        (Some(existing), Some(required)) => {
            Some(serde_json::json!({ "_and": [existing, required] }))
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn access_action_exposes_stable_policy_token() {
        assert_eq!(AccessAction::Create.as_str(), "create");
        assert_eq!(AccessAction::Read.to_string(), "read");
        assert_eq!(AccessAction::Update.as_str(), "update");
        assert_eq!(AccessAction::Delete.as_str(), "delete");
    }

    #[test]
    fn constructors_make_policy_decisions_explicit() {
        assert!(PolicyAccess::allow_all().allow);
        assert_eq!(PolicyAccess::allow_all().filter, None);

        let filtered = PolicyAccess::allow_with_filter(json!({ "tenant_id": { "_eq": "t1" } }));
        assert!(filtered.allow);
        assert_eq!(
            filtered.filter,
            Some(json!({ "tenant_id": { "_eq": "t1" } }))
        );

        assert!(!PolicyAccess::deny().allow);
        assert_eq!(PolicyAccess::deny().filter, None);
    }

    #[test]
    fn and_filter_preserves_deny_without_filter() {
        let access = PolicyAccess::deny().and_filter(Some(json!({ "tenant_id": "t1" })));

        assert!(!access.allow);
        assert!(access.filter.is_none());
    }

    #[test]
    fn and_filter_conjoins_required_filter() {
        let access = PolicyAccess::allow_with_filter(json!({ "billing_state": { "_eq": "CA" } }))
            .and_filter(Some(json!({ "tenant_id": { "_eq": "t1" } })));

        assert_eq!(
            access.filter,
            Some(json!({
                "_and": [
                    { "billing_state": { "_eq": "CA" } },
                    { "tenant_id": { "_eq": "t1" } }
                ]
            }))
        );
    }

    #[test]
    fn policy_result_requires_object() {
        let err = PolicyAccess::from_policy_result("crm.account", json!(true)).unwrap_err();

        assert!(matches!(
            err,
            ConfigError::InvalidPolicyResult { policy_key, message }
                if policy_key == "crm.account" && message == "expected policy result object"
        ));
    }

    #[test]
    fn policy_result_requires_boolean_allow() {
        let err = PolicyAccess::from_policy_result(
            "crm.account",
            json!({ "allow": "yes", "filter": {} }),
        )
        .unwrap_err();

        assert!(matches!(
            err,
            ConfigError::InvalidPolicyResult { policy_key, message }
                if policy_key == "crm.account" && message == "expected boolean `allow` field"
        ));
    }

    #[test]
    fn policy_result_rejects_non_object_filter() {
        let err =
            PolicyAccess::from_policy_result("crm.account", json!({ "allow": true, "filter": [] }))
                .unwrap_err();

        assert!(matches!(
            err,
            ConfigError::InvalidPolicyResult { policy_key, message }
                if policy_key == "crm.account"
                    && message == "expected object `filter` field when present"
        ));
    }

    #[test]
    fn policy_result_preserves_filter_value() {
        let access = PolicyAccess::from_policy_result(
            "crm.account",
            json!({ "allow": true, "filter": { "billing_state": { "_eq": "CA" } } }),
        )
        .expect("policy result should parse");

        assert!(access.allow);
        assert_eq!(
            access.filter,
            Some(json!({ "billing_state": { "_eq": "CA" } }))
        );
    }

    #[test]
    fn policy_result_treats_null_filter_as_absent() {
        let access = PolicyAccess::from_policy_result(
            "crm.account",
            json!({ "allow": true, "filter": null }),
        )
        .expect("policy result should parse");

        assert!(access.allow);
        assert_eq!(access.filter, None);
    }
}
