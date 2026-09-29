use anyhow::Result;
use rego_test::{
    evaluate_access, evaluate_access_rule, fixture_policy_path, AccessAction, AccessInput,
    AccessResult, AccessUser, PDS_TENANT_ID,
};
use serde_json::json;

const RESOURCE_RULE: &str = "data.scheduling.resource.access";

fn resource_policy() -> std::path::PathBuf {
    fixture_policy_path("scheduling/resource.rego")
}

fn resource_input(action: AccessAction, tenant_id: &str, roles: &[&str]) -> AccessInput {
    AccessInput::for_action(
        "scheduling",
        "resource",
        action,
        AccessUser::with_roles(tenant_id, "alex", roles),
    )
}

fn eval_resource(input: &AccessInput) -> Result<AccessResult> {
    evaluate_access_rule(resource_policy(), RESOURCE_RULE, input)
}

#[test]
fn pds_read_role_allows_all_rows() -> Result<()> {
    let input = resource_input(AccessAction::Read, PDS_TENANT_ID, &["operations_manager"]);

    assert_eq!(eval_resource(&input)?, AccessResult::allowed(json!({})));
    Ok(())
}

#[test]
fn community_connect_read_role_is_tenant_scoped() -> Result<()> {
    let input = resource_input(AccessAction::Read, "180123", &["dental_assistant"]);

    assert_eq!(
        eval_resource(&input)?,
        AccessResult::allowed(json!({"tenant_id": "180123"}))
    );
    Ok(())
}

#[test]
fn unsupported_read_role_denies_access() -> Result<()> {
    let input = resource_input(AccessAction::Read, PDS_TENANT_ID, &["office_cleaner"]);

    assert_eq!(eval_resource(&input)?, AccessResult::denied());
    Ok(())
}

#[test]
fn admin_can_use_every_action_without_tenant_scope() -> Result<()> {
    for action in AccessAction::ALL {
        let input = resource_input(action, "180123", &["admin"]);

        assert_eq!(
            eval_resource(&input)?,
            AccessResult::allowed(json!({})),
            "admin should be allowed to {action}"
        );
    }

    Ok(())
}

#[test]
fn mutating_actions_require_mutation_role() -> Result<()> {
    for action in [
        AccessAction::Create,
        AccessAction::Update,
        AccessAction::Delete,
    ] {
        let allowed_input = resource_input(action, "180123", &["operations_manager"]);
        let denied_input = resource_input(action, "180123", &["dental_assistant"]);

        assert_eq!(
            eval_resource(&allowed_input)?,
            AccessResult::allowed(json!({"tenant_id": "180123"})),
            "operations_manager should be allowed to {action}"
        );
        assert_eq!(
            eval_resource(&denied_input)?,
            AccessResult::denied(),
            "dental_assistant should not be allowed to {action}"
        );
    }

    Ok(())
}

#[test]
fn wrong_schema_or_entity_fails_closed_inside_policy() -> Result<()> {
    for (schema_name, entity_type) in [("crm", "resource"), ("scheduling", "office")] {
        let input = AccessInput::for_action(
            schema_name,
            entity_type,
            AccessAction::Read,
            AccessUser::with_roles(PDS_TENANT_ID, "alex", &["operations_manager"]),
        );

        assert_eq!(eval_resource(&input)?, AccessResult::denied());
    }

    Ok(())
}

#[test]
fn app_style_rule_path_matches_policy_package() -> Result<()> {
    let input = resource_input(AccessAction::Read, PDS_TENANT_ID, &["operations_manager"]);

    assert_eq!(
        evaluate_access(resource_policy(), &input)?,
        AccessResult::allowed(json!({}))
    );
    Ok(())
}
