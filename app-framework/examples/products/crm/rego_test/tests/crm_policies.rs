use std::path::PathBuf;

use anyhow::{Context, Result};
use rego_test::{evaluate_access, AccessAction, AccessInput, AccessResult, AccessUser};
use serde_json::json;

const GENERATED_CRM_POLICIES: &[&str] = &[
    "account",
    "activity",
    "activity_type",
    "contact",
    "industry",
    "lead",
    "lead_source",
    "lead_status",
    "opportunity",
    "opportunity_stage",
    "pricebook",
    "product",
    "quote",
    "quote_line_item",
    "quote_status",
    "user",
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("rego_test should live below the repository root")
        .to_path_buf()
}

fn crm_policy(entity_type: &str) -> PathBuf {
    repo_root()
        .join("backend/config/generated/schemas/crm")
        .join(format!("{entity_type}.rego"))
}

fn crm_input(entity_type: &str, action: AccessAction, roles: &[&str]) -> AccessInput {
    AccessInput::for_action(
        "crm",
        entity_type,
        action,
        AccessUser::with_roles("local", "casey", roles),
    )
}

fn eval_crm(entity_type: &str, action: AccessAction, roles: &[&str]) -> Result<AccessResult> {
    evaluate_access(
        crm_policy(entity_type),
        &crm_input(entity_type, action, roles),
    )
    .with_context(|| format!("evaluate crm/{entity_type} policy for {action}"))
}

#[test]
fn generated_crm_policies_load_and_deny_unknown_roles() -> Result<()> {
    for entity_type in GENERATED_CRM_POLICIES {
        assert_eq!(
            eval_crm(entity_type, AccessAction::Read, &["unknown_role"])?,
            AccessResult::denied(),
            "{entity_type} should deny unknown roles"
        );
    }

    Ok(())
}

#[test]
fn crm_admin_can_read_every_generated_policy() -> Result<()> {
    for entity_type in GENERATED_CRM_POLICIES {
        assert_eq!(
            eval_crm(entity_type, AccessAction::Read, &["admin"])?,
            AccessResult::allowed(json!({})),
            "{entity_type} admin read should be unrestricted"
        );
    }

    Ok(())
}

#[test]
fn account_policy_expresses_realistic_regional_scopes() -> Result<()> {
    assert_eq!(
        eval_crm("account", AccessAction::Read, &["account_ca_reader"])?,
        AccessResult::allowed(json!({"billing_state": {"_eq": "CA"}}))
    );
    assert_eq!(
        eval_crm("account", AccessAction::Update, &["sales_rep"])?,
        AccessResult::allowed(json!({
            "_and": [
                {"billing_country": {"_eq": "USA"}},
                {"billing_state": {"_in": ["CA", "OR", "WA", "NV", "AZ"]}}
            ]
        }))
    );
    assert_eq!(
        eval_crm("account", AccessAction::Delete, &["sales_rep"])?,
        AccessResult::denied()
    );

    Ok(())
}

#[test]
fn commercial_roles_are_threshold_scoped() -> Result<()> {
    assert_eq!(
        eval_crm("opportunity", AccessAction::Read, &["sales_rep"])?,
        AccessResult::allowed(json!({
            "_and": [
                {"amount": {"_lte": 250000}},
                {"lead_source": {"_ne": "Executive Referral"}}
            ]
        }))
    );
    assert_eq!(
        eval_crm("quote", AccessAction::Update, &["sales_rep"])?,
        AccessResult::allowed(json!({
            "_and": [
                {"discount": {"_lte": 25}},
                {"total_price": {"_lte": 100000}}
            ]
        }))
    );
    assert_eq!(
        eval_crm("quote_line_item", AccessAction::Update, &["sales_rep"])?,
        AccessResult::allowed(json!({"discount": {"_lte": 20}}))
    );

    Ok(())
}

#[test]
fn catalog_and_lookup_policies_are_read_shaped_for_business_roles() -> Result<()> {
    assert_eq!(
        eval_crm("product", AccessAction::Read, &["sales_rep"])?,
        AccessResult::allowed(json!({"is_active": {"_eq": true}}))
    );
    assert_eq!(
        eval_crm("pricebook", AccessAction::Read, &["finance_user"])?,
        AccessResult::allowed(json!({"is_active": {"_eq": true}}))
    );
    assert_eq!(
        eval_crm("industry", AccessAction::Read, &["finance_user"])?,
        AccessResult::allowed(json!({}))
    );
    assert_eq!(
        eval_crm("lead_status", AccessAction::Update, &["support_user"])?,
        AccessResult::denied()
    );

    Ok(())
}
