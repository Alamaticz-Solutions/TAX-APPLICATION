use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PDS_TENANT_ID: &str = "180000";

/// Application actions as serialized into Rego policy input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessAction {
    Create,
    Read,
    Update,
    Delete,
}

impl AccessAction {
    pub const ALL: [AccessAction; 4] = [
        AccessAction::Create,
        AccessAction::Read,
        AccessAction::Update,
        AccessAction::Delete,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            AccessAction::Create => "create",
            AccessAction::Read => "read",
            AccessAction::Update => "update",
            AccessAction::Delete => "delete",
        }
    }
}

impl std::fmt::Display for AccessAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// User document shape consumed by generated Rego access policies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AccessUser {
    pub tenant_id: String,
    pub username: String,
    pub principal_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_behalf_of: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ingress: Option<String>,
    pub roles: Vec<String>,
}

impl AccessUser {
    pub fn new(
        tenant_id: impl Into<String>,
        username: impl Into<String>,
        roles: Vec<String>,
    ) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            username: username.into(),
            principal_type: "user".to_string(),
            on_behalf_of: None,
            ingress: Some("http".to_string()),
            roles,
        }
    }

    pub fn with_roles(
        tenant_id: impl Into<String>,
        username: impl Into<String>,
        roles: &[&str],
    ) -> Self {
        Self::new(
            tenant_id,
            username,
            roles.iter().map(|role| role.to_string()).collect(),
        )
    }

    pub fn service(
        tenant_id: impl Into<String>,
        subject: impl Into<String>,
        roles: &[&str],
    ) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            username: subject.into(),
            principal_type: "service".to_string(),
            on_behalf_of: None,
            ingress: Some("kafka".to_string()),
            roles: roles.iter().map(|role| role.to_string()).collect(),
        }
    }

    pub fn with_ingress(mut self, ingress: impl Into<String>) -> Self {
        self.ingress = Some(ingress.into());
        self
    }

    pub fn with_on_behalf_of(mut self, subject: impl Into<String>) -> Self {
        self.on_behalf_of = Some(subject.into());
        self
    }
}

/// Input contract passed to `data.<schema>.<entity>.access`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AccessInput {
    pub schema_name: String,
    pub entity_type: String,
    pub action: String,
    pub user: AccessUser,
}

impl AccessInput {
    pub fn new(
        schema_name: impl Into<String>,
        entity_type: impl Into<String>,
        action: impl Into<String>,
        user: AccessUser,
    ) -> Self {
        Self {
            schema_name: schema_name.into(),
            entity_type: entity_type.into(),
            action: action.into(),
            user,
        }
    }

    pub fn for_action(
        schema_name: impl Into<String>,
        entity_type: impl Into<String>,
        action: AccessAction,
        user: AccessUser,
    ) -> Self {
        Self::new(schema_name, entity_type, action.as_str(), user)
    }

    pub fn rule_path(&self) -> String {
        format!("data.{}.{}.access", self.schema_name, self.entity_type)
    }

    fn to_json(&self) -> Value {
        serde_json::to_value(self).expect("AccessInput should always serialize")
    }
}

/// Result object returned from Rego access policies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccessResult {
    pub allow: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<Value>,
}

impl AccessResult {
    pub fn allowed(filter: Value) -> Self {
        Self {
            allow: true,
            filter: Some(filter),
        }
    }

    pub fn denied() -> Self {
        Self {
            allow: false,
            filter: None,
        }
    }
}

pub fn policy_fixture_path(
    crate_manifest_dir: impl AsRef<Path>,
    policy_path: impl AsRef<Path>,
) -> PathBuf {
    crate_manifest_dir
        .as_ref()
        .join("fixtures/policies")
        .join(policy_path)
}

pub fn load_policy(policy_path: impl AsRef<Path>) -> Result<regorus::Engine> {
    let policy_path = policy_path.as_ref();
    let mut engine = regorus::Engine::new();
    engine
        .add_policy_from_file(policy_path)
        .map_err(|err| anyhow!("failed to load policy {}: {err}", policy_path.display()))?;
    Ok(engine)
}

pub fn evaluate_access(policy_path: impl AsRef<Path>, input: &AccessInput) -> Result<AccessResult> {
    evaluate_access_rule(policy_path, input.rule_path(), input)
}

pub fn evaluate_access_rule(
    policy_path: impl AsRef<Path>,
    rule_path: impl Into<String>,
    input: &AccessInput,
) -> Result<AccessResult> {
    let mut engine = load_policy(policy_path)?;
    engine.set_input(regorus::Value::from(input.to_json()));

    let rule_path = rule_path.into();
    let result = engine
        .eval_rule(rule_path.clone())
        .map_err(|err| anyhow!("failed to evaluate {rule_path}: {err}"))?;
    let value = serde_json::to_value(result)
        .with_context(|| format!("failed to serialize result from {rule_path}"))?;

    serde_json::from_value(value.clone())
        .with_context(|| format!("{rule_path} returned an invalid access result: {value}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{
        fs,
        sync::atomic::{AtomicUsize, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static NEXT_POLICY_ID: AtomicUsize = AtomicUsize::new(0);

    const POLICY: &str = r#"
package crm.account
import rego.v1

default access = {"allow": false}

access := {"allow": true, "filter": {"tenant_id": input.user.tenant_id}} if {
    input.schema_name == "crm"
    input.entity_type == "account"
    input.action == "read"
    input.user.principal_type == "user"
    input.user.ingress == "http"
    input.user.roles[_] == "reader"
}
"#;

    fn write_policy(contents: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let id = NEXT_POLICY_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "appfw-policy-test-{}-{nanos}-{id}.rego",
            std::process::id()
        ));
        fs::write(&path, contents).expect("write test policy");
        path
    }

    #[test]
    fn access_action_tokens_are_stable() {
        assert_eq!(AccessAction::Create.as_str(), "create");
        assert_eq!(AccessAction::Read.as_str(), "read");
        assert_eq!(AccessAction::Update.as_str(), "update");
        assert_eq!(AccessAction::Delete.as_str(), "delete");
        assert_eq!(
            AccessAction::ALL.map(AccessAction::as_str),
            ["create", "read", "update", "delete"]
        );
    }

    #[test]
    fn access_input_rule_path_matches_backend_contract() {
        let input = AccessInput::for_action(
            "crm",
            "account",
            AccessAction::Read,
            AccessUser::with_roles("180123", "casey", &["reader"]),
        );

        assert_eq!(input.rule_path(), "data.crm.account.access");
        let json = input.to_json();
        assert_eq!(json["user"]["principal_type"], "user");
        assert_eq!(json["user"]["ingress"], "http");
    }

    #[test]
    fn access_user_can_model_service_or_on_behalf_of_principals() {
        let service = AccessUser::service("180123", "crm-event-consumer", &["writer"]);
        assert_eq!(service.principal_type, "service");
        assert_eq!(service.ingress.as_deref(), Some("kafka"));

        let delegated = AccessUser::with_roles("180123", "ai-agent", &["writer"])
            .with_ingress("mcp")
            .with_on_behalf_of("casey");
        assert_eq!(delegated.principal_type, "user");
        assert_eq!(delegated.ingress.as_deref(), Some("mcp"));
        assert_eq!(delegated.on_behalf_of.as_deref(), Some("casey"));
    }

    #[test]
    fn evaluates_access_policy_result_contract() {
        let path = write_policy(POLICY);
        let allowed_input = AccessInput::for_action(
            "crm",
            "account",
            AccessAction::Read,
            AccessUser::with_roles("180123", "casey", &["reader"]),
        );
        let denied_input = AccessInput::for_action(
            "crm",
            "account",
            AccessAction::Read,
            AccessUser::with_roles("180123", "casey", &["unknown"]),
        );

        assert_eq!(
            evaluate_access(&path, &allowed_input).expect("reader access should evaluate"),
            AccessResult::allowed(json!({"tenant_id": "180123"}))
        );
        assert_eq!(
            evaluate_access(&path, &denied_input).expect("unknown role access should evaluate"),
            AccessResult::denied()
        );

        fs::remove_file(path).expect("remove test policy");
    }

    #[test]
    fn invalid_access_result_fails_closed() {
        let path = write_policy(
            r#"
package crm.account
import rego.v1

access := {"filter": {}}
"#,
        );
        let input = AccessInput::for_action(
            "crm",
            "account",
            AccessAction::Read,
            AccessUser::with_roles("180123", "casey", &["reader"]),
        );

        let error = evaluate_access(&path, &input).expect_err("missing allow should fail");
        assert!(
            error
                .to_string()
                .contains("returned an invalid access result"),
            "unexpected error: {error:#}"
        );

        fs::remove_file(path).expect("remove test policy");
    }
}
