use std::{
    env,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context, Result};
use serde_json::{json, Map, Value};
use tokio::time::{sleep, Duration};

use crate::harness::{
    contracts, provider_certification_enabled, should_run_provider_certification, TestContext,
};

static UNIQUE_SUFFIX_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LiveProvider {
    Postgres,
    Mongo,
    Mssql,
    Snowflake,
    Unknown,
}

impl LiveProvider {
    fn from_env() -> Self {
        let raw = env::var("API_TEST_PROVIDER")
            .or_else(|_| env::var("APP_CRM_DATA_SOURCE_NAME"))
            .or_else(|_| env::var("APP_DATA_SOURCE_NAME"))
            .unwrap_or_default()
            .to_ascii_lowercase();
        if raw.contains("mongo") {
            Self::Mongo
        } else if raw.contains("mssql") || raw.contains("sqlserver") {
            Self::Mssql
        } else if raw.contains("snowflake") {
            Self::Snowflake
        } else if raw.contains("pg") || raw.contains("postgres") {
            Self::Postgres
        } else {
            Self::Unknown
        }
    }

    fn supports_relationship_projection(self) -> bool {
        !matches!(self, Self::Unknown)
    }

    fn supports_live_stored_routine(self) -> bool {
        matches!(self, Self::Postgres)
    }
}

#[tokio::test]
async fn provider_scalar_query_pagination_projection_and_error_contract() -> Result<()> {
    if !should_run_provider_certification(
        "provider_scalar_query_pagination_projection_and_error_contract",
    ) {
        return Ok(());
    }

    let mut ctx = TestContext::from_env()?;
    let suffix = unique_suffix()?;
    let alpha_name = format!("Parity Matrix Industry Alpha {suffix}");
    let beta_name = format!("Parity Matrix Industry Beta {suffix}");
    let null_phone_account_name = format!("Parity Matrix Null Phone Account {suffix}");
    let non_null_phone_account_name = format!("Parity Matrix Phone Account {suffix}");
    let activity_subject = format!("Parity Matrix Activity {suffix}");
    let mut created_industries = Vec::new();
    let mut created_accounts = Vec::new();
    let mut created_activities = Vec::new();

    let test_result = async {
        created_industries.push(create_industry(&mut ctx, &alpha_name, 111).await?);
        created_industries.push(create_industry(&mut ctx, &beta_name, 222).await?);
        let industry_id = string_field(&created_industries[0], "id")?.to_string();

        let null_phone_account = create_account_with_profile(
            &mut ctx,
            &null_phone_account_name,
            &industry_id,
            &suffix,
            None,
            125.0,
            25,
            "CA",
            &["priority", "west"],
        )
        .await?;
        let null_phone_account_id = string_field(&null_phone_account, "id")?.to_string();
        created_accounts.push(null_phone_account);

        created_accounts.push(
            create_account_with_profile(
                &mut ctx,
                &non_null_phone_account_name,
                &industry_id,
                &suffix,
                Some("949-555-0142"),
                250.0,
                125,
                "CA",
                &["standard", "east"],
            )
            .await?,
        );

        created_activities.push(
            create_activity(
                &mut ctx,
                &activity_subject,
                &today_utc_date_string()?,
                &null_phone_account_id,
            )
            .await?,
        );

        assert_query_filter_sort_pagination_and_projection(
            &mut ctx,
            &suffix,
            &alpha_name,
            &beta_name,
        )
        .await?;
        assert_account_scalar_filter_edges(
            &mut ctx,
            &suffix,
            &null_phone_account_name,
            &non_null_phone_account_name,
        )
        .await?;
        assert_activity_date_period_filter(&mut ctx, &suffix, &activity_subject).await?;
        assert_pagination_edge_errors(&mut ctx).await?;
        assert_invalid_filter_error_shape(&mut ctx).await
    }
    .await;

    for activity in created_activities.into_iter().rev() {
        let _ = delete_activity(&mut ctx, activity).await;
    }
    for account in created_accounts.into_iter().rev() {
        let _ = delete_account(&mut ctx, account).await;
    }
    for industry in created_industries.into_iter().rev() {
        let _ = delete_industry(&mut ctx, industry).await;
    }

    test_result
}

#[tokio::test]
async fn provider_stale_delete_concurrency_contract() -> Result<()> {
    if !should_run_provider_certification("provider_stale_delete_concurrency_contract") {
        return Ok(());
    }

    let mut ctx = TestContext::from_env()?;
    let suffix = unique_suffix()?;
    let industry_name = format!("Parity Matrix Stale Delete {suffix}");
    let industry = create_industry(&mut ctx, &industry_name, 333).await?;

    let test_result = reject_stale_delete(&mut ctx, industry.clone()).await;
    let _ = delete_industry(&mut ctx, industry).await;

    test_result
}

#[tokio::test]
async fn provider_relationship_projection_contract() -> Result<()> {
    if !should_run_provider_certification("provider_relationship_projection_contract") {
        return Ok(());
    }

    let provider = LiveProvider::from_env();
    if !provider.supports_relationship_projection() {
        eprintln!("skipping relationship projection contract for {provider:?}");
        return Ok(());
    }

    let mut ctx = TestContext::from_env()?;
    let suffix = unique_suffix()?;
    let industry_name = format!("Parity Matrix Relationship Industry {suffix}");
    let account_name = format!("Parity Matrix Relationship Account {suffix}");
    let contact_first_name = format!("Parity{suffix}");
    let mut created_account = None;
    let mut created_contact = None;
    let mut created_industry = None;

    let test_result = async {
        let industry = create_industry(&mut ctx, &industry_name, 444).await?;
        let industry_id = string_field(&industry, "id")?.to_string();
        created_industry = Some(industry);

        let account = create_account(&mut ctx, &account_name, &industry_id, &suffix).await?;
        let account_id = string_field(&account, "id")?.to_string();
        created_account = Some(account);

        let contact =
            create_contact(&mut ctx, &contact_first_name, "Relationship", &account_id).await?;
        require_string(&contact, "account_id", &account_id)?;
        created_contact = Some(contact);

        assert_account_relationship_projections(
            &mut ctx,
            &account_name,
            &industry_name,
            &contact_first_name,
        )
        .await?;
        assert_account_relationship_filters(
            &mut ctx,
            &account_name,
            &industry_name,
            &contact_first_name,
        )
        .await?;
        assert_seeded_many_to_many_projection(&mut ctx).await
    }
    .await;

    if let Some(contact) = created_contact {
        let _ = delete_contact(&mut ctx, contact).await;
    }
    if let Some(account) = created_account {
        let _ = delete_account(&mut ctx, account).await;
    }
    if let Some(industry) = created_industry {
        let _ = delete_industry(&mut ctx, industry).await;
    }

    test_result
}

#[tokio::test]
async fn provider_stored_routine_return_payload_contract() -> Result<()> {
    if !should_run_provider_certification("provider_stored_routine_return_payload_contract") {
        return Ok(());
    }

    let provider = LiveProvider::from_env();
    if !provider.supports_live_stored_routine() {
        eprintln!("skipping live stored routine contract for {provider:?}");
        return Ok(());
    }

    let mut ctx = TestContext::from_env()?;
    let suffix = unique_suffix()?;
    let industry_name = format!("Parity Matrix Routine Industry {suffix}");
    let account_name = format!("Parity Matrix Routine Account {suffix}");
    let mut created_industry = None;
    let mut created_account = None;

    let test_result = async {
        let industry = create_industry(&mut ctx, &industry_name, 446).await?;
        let industry_id = string_field(&industry, "id")?.to_string();
        created_industry = Some(industry);

        let account = create_account(&mut ctx, &account_name, &industry_id, &suffix).await?;
        let account_id = string_field(&account, "id")?.to_string();
        created_account = Some(account);

        let refreshed =
            assert_account_health_stored_routine_returns_and_persists(&mut ctx, &account_id)
                .await?;
        created_account = Some(refreshed);
        Ok(())
    }
    .await;

    if let Some(account) = created_account {
        let _ = delete_account(&mut ctx, account).await;
    }
    if let Some(industry) = created_industry {
        let _ = delete_industry(&mut ctx, industry).await;
    }

    test_result
}

#[tokio::test]
async fn provider_access_filter_policy_cannot_widen_user_filter_contract() -> Result<()> {
    if !should_run_provider_certification(
        "provider_access_filter_policy_cannot_widen_user_filter_contract",
    ) {
        return Ok(());
    }

    if !policy_contract_auth_available("access-filter live contract")? {
        return Ok(());
    }

    let mut ctx = TestContext::from_env()?;
    let suffix = unique_suffix()?;
    let industry_name = format!("Parity Matrix Access Industry {suffix}");
    let ca_account_name = format!("Parity Matrix Access CA Account {suffix}");
    let ny_account_name = format!("Parity Matrix Access NY Account {suffix}");
    let mut created_accounts = Vec::new();
    let mut created_industry = None;

    let test_result = async {
        let industry = create_industry(&mut ctx, &industry_name, 555).await?;
        let industry_id = string_field(&industry, "id")?.to_string();
        created_industry = Some(industry);

        created_accounts.push(
            create_account_with_profile(
                &mut ctx,
                &ca_account_name,
                &industry_id,
                &suffix,
                Some("949-555-0150"),
                150.0,
                50,
                "CA",
                &["access", "west"],
            )
            .await?,
        );
        let ny_account = create_account_with_profile(
            &mut ctx,
            &ny_account_name,
            &industry_id,
            &suffix,
            Some("212-555-0150"),
            175.0,
            60,
            "NY",
            &["access", "east"],
        )
        .await?;
        let ny_account_id = string_field(&ny_account, "id")?.to_string();
        created_accounts.push(ny_account.clone());
        let ny_account_locator = account_locator_as(&mut ctx, "pdsh_admin", &ny_account_id).await?;

        assert_account_filter_names_as(
            &mut ctx,
            "cc_tenant_user",
            "access filter narrows broad user query",
            json!({ "name": { "_contains": suffix } }),
            &[&ca_account_name],
        )
        .await?;
        assert_account_filter_names_as(
            &mut ctx,
            "cc_tenant_user",
            "access filter cannot be widened by conflicting user filter",
            json!({
              "_and": [
                { "name": { "_contains": suffix } },
                { "billingState": { "_eq": "NY" } }
              ]
            }),
            &[],
        )
        .await?;
        assert_account_filter_names_as(
            &mut ctx,
            "cc_tenant_user",
            "access filter wraps disjunction without widening policy scope",
            json!({
              "_or": [
                { "billingState": { "_eq": "NY" } },
                { "name": { "_contains": suffix } }
              ]
            }),
            &[&ca_account_name],
        )
        .await?;
        assert_account_locator_read_absent_or_denied(
            &mut ctx,
            "cc_tenant_user",
            &ny_account_locator,
            "policy-scoped user locator read",
        )
        .await?;
        assert_account_delete_denied_preserves_record(
            &mut ctx,
            "west_sales_rep",
            ny_account.clone(),
            &ny_account_name,
        )
        .await?;
        assert_account_create_denied_for_policy_scoped_user(&mut ctx, &industry_id, &suffix)
            .await?;
        assert_account_update_denied_for_policy_scoped_user(
            &mut ctx,
            "west_sales_rep",
            ny_account,
            &suffix,
        )
        .await
    }
    .await;

    for account in created_accounts.into_iter().rev() {
        let _ = delete_account(&mut ctx, account).await;
    }
    if let Some(industry) = created_industry {
        let _ = delete_industry(&mut ctx, industry).await;
    }

    test_result
}

#[tokio::test]
async fn provider_error_normalization_contract() -> Result<()> {
    if !should_run_provider_certification("provider_error_normalization_contract") {
        return Ok(());
    }

    let mut ctx = TestContext::from_env()?;
    let suffix = unique_suffix()?;
    let policy_auth_available =
        policy_contract_auth_available("denied-access error normalization contract")?;
    let duplicate_industry_id = unique_uuid()?;
    let stale_industry_name = format!("Parity Matrix Error Stale {suffix}");
    let denied_industry_name = format!("Parity Matrix Error Denied Industry {suffix}");
    let mut created_industries = Vec::new();

    let test_result = async {
        let duplicate = create_industry_with_id(
            &mut ctx,
            &duplicate_industry_id,
            &format!("Parity Matrix Duplicate {suffix}"),
            661,
        )
        .await?;
        created_industries.push(duplicate);
        assert_duplicate_key_error_normalized(&mut ctx, &duplicate_industry_id, &suffix).await?;
        assert_foreign_key_error_normalized(&mut ctx, &suffix).await?;
        assert_required_field_error_normalized(&mut ctx, &suffix).await?;

        let stale = create_industry(&mut ctx, &stale_industry_name, 662).await?;
        created_industries.push(stale.clone());
        let stale_response = stale_delete_response(&mut ctx, stale).await?;
        assert_graphql_error_normalized(&stale_response, "stale version", Some("version"))?;

        let denied_industry = create_industry(&mut ctx, &denied_industry_name, 663).await?;
        let denied_industry_id = string_field(&denied_industry, "id")?.to_string();
        created_industries.push(denied_industry);
        if policy_auth_available {
            let denied_response =
                denied_account_create_response(&mut ctx, &denied_industry_id, &suffix).await?;
            assert_denied_account_mutation_response(
                &mut ctx,
                denied_response,
                "denied access",
                Some("access"),
            )
            .await?;
        }

        let invalid_filter_response = invalid_filter_response(&mut ctx).await?;
        assert_graphql_error_normalized(&invalid_filter_response, "invalid filter", Some("filter"))
    }
    .await;

    for industry in created_industries.into_iter().rev() {
        let _ = delete_industry(&mut ctx, industry).await;
    }

    test_result
}

#[tokio::test]
async fn provider_tenant_isolation_contract() -> Result<()> {
    if !should_run_provider_certification("provider_tenant_isolation_contract") {
        return Ok(());
    }

    if !policy_contract_auth_available("tenant isolation live contract")? {
        return Ok(());
    }

    let mut ctx = TestContext::from_env()?;
    let suffix = unique_suffix()?;
    let industry_name = format!("Parity Matrix Tenant Industry {suffix}");
    let local_account_name = format!("Parity Matrix Tenant Local Account {suffix}");
    let other_account_name = format!("Parity Matrix Tenant Other Account {suffix}");
    let mut created_industry = None;
    let mut local_account = None;
    let mut other_account = None;

    let test_result = async {
        let industry = create_industry(&mut ctx, &industry_name, 771).await?;
        let industry_id = string_field(&industry, "id")?.to_string();
        created_industry = Some(industry);

        let local = create_account_with_profile(
            &mut ctx,
            &local_account_name,
            &industry_id,
            &suffix,
            Some("949-555-0171"),
            171.0,
            17,
            "CA",
            &["tenant", "local"],
        )
        .await?;
        let local_account_id = string_field(&local, "id")?.to_string();
        local_account = Some(local);

        let other = create_account_with_profile_as(
            &mut ctx,
            "other_tenant_admin",
            &other_account_name,
            &industry_id,
            &suffix,
            Some("303-555-0172"),
            172.0,
            18,
            "CO",
            &["tenant", "other"],
        )
        .await?;
        let other_account_id = string_field(&other, "id")?.to_string();
        other_account = Some(other);

        assert_account_audit_tenant_visibility(
            &mut ctx,
            "pdsh_admin",
            &local_account_id,
            Some("local"),
        )
        .await?;
        assert_account_audit_tenant_visibility(&mut ctx, "pdsh_admin", &other_account_id, None)
            .await?;
        assert_account_audit_tenant_visibility(
            &mut ctx,
            "other_tenant_admin",
            &other_account_id,
            Some("tenant-2"),
        )
        .await?;
        assert_account_audit_tenant_visibility(
            &mut ctx,
            "other_tenant_admin",
            &local_account_id,
            None,
        )
        .await
    }
    .await;

    if let Some(account) = other_account {
        let _ = delete_account_as(&mut ctx, "other_tenant_admin", account).await;
    }
    if let Some(account) = local_account {
        let _ = delete_account(&mut ctx, account).await;
    }
    if let Some(industry) = created_industry {
        let _ = delete_industry(&mut ctx, industry).await;
    }

    test_result
}

#[tokio::test]
async fn provider_locator_tenant_isolation_contract() -> Result<()> {
    if !should_run_provider_certification("provider_locator_tenant_isolation_contract") {
        return Ok(());
    }

    if !policy_contract_auth_available("locator tenant isolation live contract")? {
        return Ok(());
    }

    let mut ctx = TestContext::from_env()?;
    let suffix = unique_suffix()?;
    let industry_name = format!("Parity Matrix Locator Industry {suffix}");
    let local_account_name = format!("Parity Matrix Locator Local Account {suffix}");
    let other_account_name = format!("Parity Matrix Locator Other Account {suffix}");
    let mut created_industry = None;
    let mut local_account = None;
    let mut other_account = None;

    let test_result = async {
        let industry = create_industry(&mut ctx, &industry_name, 778).await?;
        let industry_id = string_field(&industry, "id")?.to_string();
        created_industry = Some(industry);

        let local = create_account_with_profile(
            &mut ctx,
            &local_account_name,
            &industry_id,
            &suffix,
            Some("949-555-0178"),
            178.0,
            19,
            "CA",
            &["locator", "local"],
        )
        .await?;
        let local_account_id = string_field(&local, "id")?.to_string();
        local_account = Some(local);

        let other = create_account_with_profile_as(
            &mut ctx,
            "other_tenant_admin",
            &other_account_name,
            &industry_id,
            &suffix,
            Some("303-555-0179"),
            179.0,
            20,
            "CO",
            &["locator", "other"],
        )
        .await?;
        let other_account_id = string_field(&other, "id")?.to_string();
        other_account = Some(other);

        let local_locator = account_locator_as(&mut ctx, "pdsh_admin", &local_account_id).await?;
        let other_locator =
            account_locator_as(&mut ctx, "other_tenant_admin", &other_account_id).await?;

        assert_account_locator_read_as(
            &mut ctx,
            "cc_tenant_user",
            &local_locator,
            &local_account_id,
            &local_account_name,
        )
        .await?;
        assert_invalid_account_locator_shape(&mut ctx).await?;
        assert_account_locator_read_absent_or_denied(
            &mut ctx,
            "cc_tenant_user",
            &other_locator,
            "policy user cross-tenant locator read",
        )
        .await?;
        Ok(())
    }
    .await;

    if let Some(account) = other_account {
        let _ = delete_account_as(&mut ctx, "other_tenant_admin", account).await;
    }
    if let Some(account) = local_account {
        let _ = delete_account(&mut ctx, account).await;
    }
    if let Some(industry) = created_industry {
        let _ = delete_industry(&mut ctx, industry).await;
    }

    test_result
}

#[tokio::test]
async fn provider_audit_append_redaction_chain_contract() -> Result<()> {
    if !should_run_provider_certification("provider_audit_append_redaction_chain_contract") {
        return Ok(());
    }

    let mut ctx = TestContext::from_env()?;
    let suffix = unique_suffix()?;
    let industry_name = format!("Parity Matrix Audit Industry {suffix}");
    let account_name = format!("Parity Matrix Audit Account {suffix}");
    let updated_account_name = format!("Parity Matrix Audit Account Updated {suffix}");
    let denied_update_name = format!("Parity Matrix Audit Denied Update {suffix}");
    let denied_update_email = format!("audit-denied-{suffix}@api-test.example.com");
    let cross_tenant_account_name = format!("Parity Matrix Audit Cross Tenant Account {suffix}");
    let cross_tenant_updated_name =
        format!("Parity Matrix Audit Cross Tenant Account Updated {suffix}");
    let cross_tenant_denied_name =
        format!("Parity Matrix Audit Cross Tenant Denied Update {suffix}");
    let cross_tenant_denied_email =
        format!("audit-cross-tenant-denied-{suffix}@api-test.example.com");
    let mut created_account = None;
    let mut cross_tenant_account = None;
    let mut tenant_one_industry = None;
    let mut cross_tenant_industry = None;

    let test_result = async {
        let assert_denied_attempt =
            policy_contract_auth_available("audit denied mutation contract")?;
        let industry =
            create_industry_as(&mut ctx, "tenant_one_crm_ops", &industry_name, 777).await?;
        let industry_id = string_field(&industry, "id")?.to_string();
        tenant_one_industry = Some(industry);

        let account = create_account_with_profile_as(
            &mut ctx,
            "tenant_one_crm_ops",
            &account_name,
            &industry_id,
            &suffix,
            Some("949-555-0177"),
            777.0,
            77,
            "CA",
            &["audit", "redaction"],
        )
        .await?;
        let account_id = string_field(&account, "id")?.to_string();

        let updated = update_account_name_and_email_as(
            &mut ctx,
            "tenant_one_crm_ops",
            account,
            &updated_account_name,
            &format!("audit-updated-{suffix}@api-test.example.com"),
        )
        .await?;
        created_account = Some(updated);

        if assert_denied_attempt {
            let target_chain_before =
                account_audit_items_as(&mut ctx, "tenant_one_crm_ops", &account_id).await?;
            assert_account_audit_events(
                &target_chain_before,
                &account_id,
                &account_name,
                &updated_account_name,
            )?;

            find_account_by_id_as(&mut ctx, "cc_tenant_user", &account_id)
                .await?
                .with_context(|| {
                    format!("cc_tenant_user could not read actor-visible account {account_id}")
                })?;
            let denied_response = denied_account_update_response(
                &mut ctx,
                "cc_tenant_user",
                created_account
                    .clone()
                    .expect("updated account should be captured"),
                &denied_update_name,
                &denied_update_email,
            )
            .await?;
            assert_denied_account_mutation_response(
                &mut ctx,
                denied_response,
                "audited denied update",
                Some("access"),
            )
            .await?;

            assert_account_audit_access_denied(
                &mut ctx,
                "cc_tenant_user",
                &account_id,
                "denied actor audit read",
            )
            .await?;
            let actor_items =
                account_audit_items_as(&mut ctx, "tenant_one_crm_ops", &account_id).await?;
            let update_hash = audit_event_hash(&target_chain_before, "update", "succeeded")?;
            assert_actor_denied_audit_event(
                &actor_items,
                &account_id,
                "tenant-1",
                "cc_tenant_user",
                (&denied_update_name, &denied_update_email),
                3,
                Some(update_hash),
            )?;
            assert_chain_extended_by_denied_event(
                &target_chain_before,
                &actor_items,
                "same-tenant denied update",
            )?;
            assert_audit_items_absent(
                &mut ctx,
                "pdsh_admin",
                &account_id,
                "local tenant audit read",
            )
            .await?;
            assert_audit_items_absent(
                &mut ctx,
                "other_tenant_admin",
                &account_id,
                "tenant-2 audit read",
            )
            .await?;

            let cross_industry = create_industry_as(
                &mut ctx,
                "other_tenant_admin",
                &format!("{industry_name} Cross Tenant"),
                778,
            )
            .await?;
            let cross_industry_id = string_field(&cross_industry, "id")?.to_string();
            cross_tenant_industry = Some(cross_industry);

            let cross_tenant = create_account_with_profile_as(
                &mut ctx,
                "other_tenant_admin",
                &cross_tenant_account_name,
                &cross_industry_id,
                &suffix,
                Some("303-555-0177"),
                778.0,
                78,
                "CO",
                &["audit", "cross-tenant"],
            )
            .await?;
            let cross_tenant_account_id = string_field(&cross_tenant, "id")?.to_string();
            let cross_tenant_updated = update_account_name_and_email_as(
                &mut ctx,
                "other_tenant_admin",
                cross_tenant,
                &cross_tenant_updated_name,
                &format!("audit-cross-tenant-updated-{suffix}@api-test.example.com"),
            )
            .await?;
            cross_tenant_account = Some(cross_tenant_updated.clone());

            let cross_tenant_chain_before =
                account_audit_items_as(&mut ctx, "other_tenant_admin", &cross_tenant_account_id)
                    .await?;
            assert_account_audit_events(
                &cross_tenant_chain_before,
                &cross_tenant_account_id,
                &cross_tenant_account_name,
                &cross_tenant_updated_name,
            )?;

            let denied_response = denied_account_update_response(
                &mut ctx,
                "cc_tenant_user",
                cross_tenant_updated,
                &cross_tenant_denied_name,
                &cross_tenant_denied_email,
            )
            .await?;
            assert_denied_account_mutation_response(
                &mut ctx,
                denied_response,
                "cross-tenant audited denied update",
                Some("access"),
            )
            .await?;

            assert_account_audit_access_denied(
                &mut ctx,
                "cc_tenant_user",
                &cross_tenant_account_id,
                "cross-tenant denied actor audit read",
            )
            .await?;
            let cross_tenant_actor_items =
                account_audit_items_as(&mut ctx, "tenant_one_crm_ops", &cross_tenant_account_id)
                    .await?;
            assert_actor_denied_audit_event(
                &cross_tenant_actor_items,
                &cross_tenant_account_id,
                "tenant-1",
                "cc_tenant_user",
                (&cross_tenant_denied_name, &cross_tenant_denied_email),
                1,
                None,
            )?;
            assert_audit_items_absent(
                &mut ctx,
                "pdsh_admin",
                &cross_tenant_account_id,
                "wrong-tenant target audit read",
            )
            .await?;
            let cross_tenant_chain_after =
                account_audit_items_as(&mut ctx, "other_tenant_admin", &cross_tenant_account_id)
                    .await?;
            assert_target_chain_unchanged(
                &cross_tenant_chain_before,
                &cross_tenant_chain_after,
                "cross-tenant denied update",
            )?;
        }

        Ok(())
    }
    .await;

    if let Some(account) = cross_tenant_account {
        let _ = delete_account_as(&mut ctx, "other_tenant_admin", account).await;
    }
    if let Some(account) = created_account {
        let _ = delete_account_as(&mut ctx, "tenant_one_crm_ops", account).await;
    }
    if let Some(industry) = cross_tenant_industry {
        let _ = delete_industry_as(&mut ctx, "other_tenant_admin", industry).await;
    }
    if let Some(industry) = tenant_one_industry {
        let _ = delete_industry_as(&mut ctx, "tenant_one_crm_ops", industry).await;
    }

    test_result
}

async fn assert_query_filter_sort_pagination_and_projection(
    ctx: &mut TestContext,
    suffix: &str,
    expected_second_page_name: &str,
    expected_first_page_name: &str,
) -> Result<()> {
    let req = r#"
      query($filter: JSON, $sort: JSON, $skip: Int!, $limit: Int!){
        queryIndustries(filter: $filter, sort: $sort, skip: $skip, limit: $limit){
          skip
          limit
          page_index
          query_count
          items { id name sort_order version }
        }
      }
    "#;
    let response = ctx
        .execute_graphql(
            "pdsh_admin",
            "crm",
            req,
            Some(json!({
              "filter": { "name": { "_contains": suffix } },
              "sort": { "sortOrder": "DESC" },
              "skip": 1,
              "limit": 1
            })),
        )
        .await?;
    let result = contracts::operation_object(&response, "queryIndustries")?;
    require_i64(result, "skip", 1)?;
    require_i64(result, "limit", 1)?;
    require_i64(result, "page_index", 1)?;
    require_i64(result, "query_count", 2)?;

    let items = result
        .get("items")
        .and_then(Value::as_array)
        .context("queryIndustries.items was not an array")?;
    if items.len() != 1 {
        bail!("expected one paged item, got {}: {items:?}", items.len());
    }
    let item = items[0]
        .as_object()
        .context("queryIndustries item was not an object")?;
    if item.get("name").and_then(Value::as_str) != Some(expected_second_page_name) {
        bail!("expected second sorted page item {expected_second_page_name}, got {item:?}");
    }

    let first_page = query_first_industry_page(ctx, suffix).await?;
    let first_item = first_page
        .get("items")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(Value::as_object)
        .context("first page did not contain an item")?;
    if first_item.get("name").and_then(Value::as_str) != Some(expected_first_page_name) {
        bail!("expected first sorted page item {expected_first_page_name}, got {first_item:?}");
    }

    Ok(())
}

async fn query_first_industry_page(
    ctx: &mut TestContext,
    suffix: &str,
) -> Result<Map<String, Value>> {
    let req = r#"
      query($filter: JSON, $sort: JSON, $skip: Int!, $limit: Int!){
        queryIndustries(filter: $filter, sort: $sort, skip: $skip, limit: $limit){
          query_count
          items { id name sort_order version }
        }
      }
    "#;
    let response = ctx
        .execute_graphql(
            "pdsh_admin",
            "crm",
            req,
            Some(json!({
              "filter": { "name": { "_contains": suffix } },
              "sort": { "sortOrder": "DESC" },
              "skip": 0,
              "limit": 1
            })),
        )
        .await?;
    Ok(contracts::operation_object(&response, "queryIndustries")?.clone())
}

async fn assert_account_scalar_filter_edges(
    ctx: &mut TestContext,
    suffix: &str,
    null_phone_account_name: &str,
    non_null_phone_account_name: &str,
) -> Result<()> {
    assert_account_filter_names(
        ctx,
        "null equality",
        json!({
          "_and": [
            { "name": { "_contains": suffix } },
            { "phone": { "_eq": null } }
          ]
        }),
        &[null_phone_account_name],
    )
    .await?;

    assert_account_filter_names(
        ctx,
        "null inequality",
        json!({
          "_and": [
            { "name": { "_contains": suffix } },
            { "phone": { "_ne": null } }
          ]
        }),
        &[non_null_phone_account_name],
    )
    .await?;

    assert_account_filter_names(
        ctx,
        "numeric comparisons",
        json!({
          "_and": [
            { "name": { "_contains": suffix } },
            { "numberOfEmployees": { "_gt": 50 } },
            { "annualRevenue": { "_lte": 250.0 } }
          ]
        }),
        &[non_null_phone_account_name],
    )
    .await?;

    assert_account_filter_names(
        ctx,
        "numeric inequality",
        json!({
          "_and": [
            { "name": { "_contains": suffix } },
            { "numberOfEmployees": { "_ne": 25 } }
          ]
        }),
        &[non_null_phone_account_name],
    )
    .await?;

    assert_account_filter_names(
        ctx,
        "string starts and ends",
        json!({
          "_and": [
            { "name": { "_starts": "Parity Matrix" } },
            { "name": { "_ends": suffix } }
          ]
        }),
        &[null_phone_account_name, non_null_phone_account_name],
    )
    .await?;

    assert_account_filter_names(
        ctx,
        "membership",
        json!({ "name": { "_in": [non_null_phone_account_name, format!("missing-{suffix}")] } }),
        &[non_null_phone_account_name],
    )
    .await?;

    assert_account_filter_names(
        ctx,
        "boolean conjunction",
        json!({
          "_and": [
            { "name": { "_contains": suffix } },
            {
              "_or": [
                { "numberOfEmployees": { "_lt": 50 } },
                { "annualRevenue": { "_gte": 250.0 } }
              ]
            }
          ]
        }),
        &[null_phone_account_name, non_null_phone_account_name],
    )
    .await?;

    assert_account_filter_names(
        ctx,
        "array contains scalar",
        json!({
          "_and": [
            { "name": { "_contains": suffix } },
            { "tags": { "_contains": format!("parity-{suffix}-priority") } }
          ]
        }),
        &[null_phone_account_name],
    )
    .await?;

    assert_account_filter_names(
        ctx,
        "array overlaps",
        json!({
          "_and": [
            { "name": { "_contains": suffix } },
            {
              "tags": {
                "_overlaps": [
                  format!("parity-{suffix}-standard"),
                  format!("parity-{suffix}-north")
                ]
              }
            }
          ]
        }),
        &[non_null_phone_account_name],
    )
    .await
}

async fn assert_account_filter_names(
    ctx: &mut TestContext,
    case: &str,
    filter: Value,
    expected_names: &[&str],
) -> Result<()> {
    assert_account_filter_names_as(ctx, "pdsh_admin", case, filter, expected_names).await
}

async fn assert_account_filter_names_as(
    ctx: &mut TestContext,
    auth_token_name: &str,
    case: &str,
    filter: Value,
    expected_names: &[&str],
) -> Result<()> {
    let req = r#"
      query($filter: JSON, $sort: JSON, $skip: Int!, $limit: Int!){
        queryAccounts(filter: $filter, sort: $sort, skip: $skip, limit: $limit){
          query_count
          items { id name phone annual_revenue number_of_employees }
        }
      }
    "#;
    let response = ctx
        .execute_graphql(
            auth_token_name,
            "crm",
            req,
            Some(json!({
              "filter": filter,
              "sort": { "name": "ASC" },
              "skip": 0,
              "limit": 10
            })),
        )
        .await?;
    let result = contracts::operation_object(&response, "queryAccounts")
        .with_context(|| format!("{case} response: {response:?}"))?;
    require_i64(result, "query_count", expected_names.len() as i64)
        .with_context(|| format!("{case} query_count mismatch"))?;

    let items = result
        .get("items")
        .and_then(Value::as_array)
        .with_context(|| format!("{case} items missing"))?;
    let actual_names = items
        .iter()
        .map(|item| {
            item.get("name")
                .and_then(Value::as_str)
                .unwrap_or("<missing>")
                .to_string()
        })
        .collect::<Vec<_>>();
    let expected = expected_names
        .iter()
        .map(|name| (*name).to_string())
        .collect::<Vec<_>>();
    if actual_names != expected {
        bail!("{case} expected account names {expected:?}, got {actual_names:?}");
    }

    Ok(())
}

async fn assert_account_create_denied_for_policy_scoped_user(
    ctx: &mut TestContext,
    industry_id: &str,
    suffix: &str,
) -> Result<()> {
    let response = denied_account_create_response(ctx, industry_id, suffix).await?;
    assert_denied_account_mutation_response(
        ctx,
        response,
        "policy-scoped user create",
        Some("access"),
    )
    .await
}

async fn assert_account_update_denied_for_policy_scoped_user(
    ctx: &mut TestContext,
    auth_token_name: &str,
    input: Map<String, Value>,
    suffix: &str,
) -> Result<()> {
    let response = denied_account_update_response(
        ctx,
        auth_token_name,
        input,
        &format!("Parity Matrix Denied Update {suffix}"),
        &format!("denied-update-{suffix}@api-test.example.com"),
    )
    .await?;
    assert_denied_account_mutation_response(
        ctx,
        response,
        "policy-scoped user update",
        Some("access"),
    )
    .await
}

async fn assert_account_delete_denied_preserves_record(
    ctx: &mut TestContext,
    auth_token_name: &str,
    input: Map<String, Value>,
    expected_name: &str,
) -> Result<()> {
    let account_id = string_field(&input, "id")?.to_string();
    let response = account_delete_response(ctx, auth_token_name, input).await?;
    if response.get("errors").and_then(Value::as_array).is_some() {
        assert_graphql_error_normalized(&response, "policy-scoped user delete", Some("access"))?;
    } else {
        let deleted = contracts::operation_i64(&response, "deleteAccount")
            .with_context(|| format!("policy-scoped user delete response: {response:?}"))?;
        if deleted != 0 {
            bail!(
                "policy-scoped user delete expected an access error or 0 deleted rows, deleted {deleted}: {response:?}"
            );
        }
    }

    let preserved = find_account_by_id_as(ctx, "pdsh_admin", &account_id)
        .await?
        .with_context(|| {
            format!("policy-scoped user delete removed account {account_id}: {response:?}")
        })?;
    if preserved.get("name").and_then(Value::as_str) != Some(expected_name) {
        bail!("preserved account name mismatch after denied delete: {preserved:?}");
    }

    Ok(())
}

async fn denied_account_update_response(
    ctx: &mut TestContext,
    auth_token_name: &str,
    mut input: Map<String, Value>,
    name: &str,
    email: &str,
) -> Result<Value> {
    let req = r#"
      mutation($input: InputAccount!){
        updateAccount(input: $input){
          id,name,website,phone,email,billing_city,billing_state,billing_country,
          annual_revenue,number_of_employees,tags,industry_id,version
        }
      }
    "#;
    input.insert("name".to_string(), json!(name));
    input.insert("email".to_string(), json!(email));
    ctx.execute_graphql(
        auth_token_name,
        "crm",
        req,
        Some(json!({ "input": Value::Object(input) })),
    )
    .await
}

async fn assert_denied_account_mutation_response(
    ctx: &mut TestContext,
    response: Value,
    case: &str,
    expected_fragment: Option<&str>,
) -> Result<()> {
    if let Some(account) = successful_account_mutation(&response) {
        let cleanup = delete_account(ctx, account).await;
        if let Err(err) = cleanup {
            bail!(
                "{case} unexpectedly succeeded and cleanup failed: {err}; response: {response:?}"
            );
        }
    }
    assert_graphql_error_normalized(&response, case, expected_fragment)
}

fn successful_account_mutation(response: &Value) -> Option<Map<String, Value>> {
    response
        .get("data")
        .and_then(Value::as_object)
        .and_then(|data| {
            data.get("createAccount")
                .or_else(|| data.get("updateAccount"))
                .and_then(Value::as_object)
        })
        .cloned()
}

async fn denied_account_create_response(
    ctx: &mut TestContext,
    industry_id: &str,
    suffix: &str,
) -> Result<Value> {
    let req = r#"
      mutation($input: InputAccount!){
        createAccount(input: $input){
          id,name,website,phone,email,billing_city,billing_state,billing_country,
          annual_revenue,number_of_employees,tags,industry_id,version
        }
      }
    "#;
    let response = ctx
        .execute_graphql(
            "cc_tenant_user",
            "crm",
            req,
            Some(json!({
              "input": {
                "name": format!("Parity Matrix Denied Account {suffix}"),
                "website": format!("https://denied-{suffix}.example.com"),
                "email": format!("denied-{suffix}@api-test.example.com"),
                "billing_city": "Irvine",
                "billing_state": "CA",
                "billing_country": "USA",
                "annual_revenue": 10.0,
                "number_of_employees": 1,
                "tags": [format!("parity-{suffix}-denied")],
                "industry_id": industry_id,
                "version": 0
              }
            })),
        )
        .await?;
    Ok(response)
}

async fn account_locator_as(
    ctx: &mut TestContext,
    auth_token_name: &str,
    account_id: &str,
) -> Result<String> {
    let account = find_account_by_id_as(ctx, auth_token_name, account_id)
        .await?
        .with_context(|| format!("{auth_token_name} could not read account {account_id}"))?;
    let locator = string_field(&account, "record_locator")?;
    assert_record_locator_shape(locator)?;
    Ok(locator.to_string())
}

async fn find_account_by_id_as(
    ctx: &mut TestContext,
    auth_token_name: &str,
    account_id: &str,
) -> Result<Option<Map<String, Value>>> {
    let req = r#"
      query($id: String!){
        findAccount(id: $id){
          id
          name
          record_locator
          version
        }
      }
    "#;
    let response = ctx
        .execute_graphql(
            auth_token_name,
            "crm",
            req,
            Some(json!({ "id": account_id })),
        )
        .await?;
    if response.get("errors").and_then(Value::as_array).is_some() {
        bail!("{auth_token_name} findAccount returned errors: {response:?}");
    }
    optional_operation_object(&response, "findAccount", "findAccount")
}

async fn assert_account_locator_read_as(
    ctx: &mut TestContext,
    auth_token_name: &str,
    locator: &str,
    expected_id: &str,
    expected_name: &str,
) -> Result<()> {
    let response = find_account_by_locator_response(ctx, auth_token_name, locator).await?;
    let account = contracts::operation_object(&response, "findAccountByLocator")
        .with_context(|| format!("{auth_token_name} allowed locator read: {response:?}"))?;
    if account.get("id").and_then(Value::as_str) != Some(expected_id)
        || account.get("name").and_then(Value::as_str) != Some(expected_name)
        || account.get("record_locator").and_then(Value::as_str) != Some(locator)
    {
        bail!(
            "{auth_token_name} locator read returned unexpected account, expected id={expected_id} name={expected_name} locator={locator}: {account:?}"
        );
    }
    Ok(())
}

async fn assert_invalid_account_locator_shape(ctx: &mut TestContext) -> Result<()> {
    let response = find_account_by_locator_response(ctx, "pdsh_admin", "account-1").await?;
    assert_graphql_error_normalized(
        &response,
        "invalid account locator shape",
        Some("invalid record locator"),
    )
}

async fn assert_account_locator_read_absent_or_denied(
    ctx: &mut TestContext,
    auth_token_name: &str,
    locator: &str,
    case: &str,
) -> Result<()> {
    let response = find_account_by_locator_response(ctx, auth_token_name, locator).await?;
    if response.get("errors").and_then(Value::as_array).is_some() {
        return assert_graphql_error_normalized(&response, case, Some("access"));
    }

    if let Some(account) = optional_operation_object(&response, "findAccountByLocator", case)? {
        bail!("{case} exposed account by locator {locator}: {account:?}");
    }

    Ok(())
}

async fn find_account_by_locator_response(
    ctx: &mut TestContext,
    auth_token_name: &str,
    locator: &str,
) -> Result<Value> {
    let req = r#"
      query($locator: String!){
        findAccountByLocator(locator: $locator){
          id
          name
          record_locator
          billing_state
          version
        }
      }
    "#;
    ctx.execute_graphql(
        auth_token_name,
        "crm",
        req,
        Some(json!({ "locator": locator })),
    )
    .await
}

async fn assert_account_health_stored_routine_returns_and_persists(
    ctx: &mut TestContext,
    account_id: &str,
) -> Result<Map<String, Value>> {
    let score = 91.25;
    let response = refresh_account_health_stored_procedure_response(ctx, account_id, score).await?;
    let payload = contracts::operation_object(&response, "refreshAccountHealthStoredProcedure")
        .with_context(|| format!("stored routine response: {response:?}"))?;

    require_string(payload, "account_id", account_id)?;
    require_f64(payload, "health_score", score)?;
    if payload
        .get("refreshed_at")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        bail!("stored routine did not return refreshed_at: {payload:?}");
    }
    if payload
        .get("source")
        .and_then(Value::as_str)
        .filter(|value| value.contains("stored-procedure"))
        .is_none()
    {
        bail!("stored routine did not return provider source: {payload:?}");
    }

    let refreshed = find_account_health_snapshot(ctx, account_id).await?;
    require_f64(&refreshed, "health_score", score)?;
    if refreshed
        .get("health_last_refreshed_at")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        bail!("stored routine did not persist health_last_refreshed_at: {refreshed:?}");
    }
    Ok(refreshed)
}

async fn refresh_account_health_stored_procedure_response(
    ctx: &mut TestContext,
    account_id: &str,
    health_score: f64,
) -> Result<Value> {
    let req = r#"
      mutation($accountId: String!, $healthScore: Float!){
        refreshAccountHealthStoredProcedure(accountId: $accountId, healthScore: $healthScore)
      }
    "#;
    ctx.execute_graphql(
        "pdsh_admin",
        "crm",
        req,
        Some(json!({
          "accountId": account_id,
          "healthScore": health_score
        })),
    )
    .await
}

async fn find_account_health_snapshot(
    ctx: &mut TestContext,
    account_id: &str,
) -> Result<Map<String, Value>> {
    let req = r#"
      query($id: String!){
        findAccount(id: $id){
          id
          name
          website
          phone
          email
          billing_city
          billing_state
          billing_country
          annual_revenue
          number_of_employees
          tags
          industry_id
          health_score
          health_last_refreshed_at
          version
        }
      }
    "#;
    let response = ctx
        .execute_graphql("pdsh_admin", "crm", req, Some(json!({ "id": account_id })))
        .await?;
    Ok(contracts::operation_object(&response, "findAccount")
        .with_context(|| format!("findAccount health snapshot response: {response:?}"))?
        .clone())
}

fn optional_operation_object(
    response: &Value,
    operation: &str,
    case: &str,
) -> Result<Option<Map<String, Value>>> {
    let data = response
        .get("data")
        .and_then(Value::as_object)
        .with_context(|| format!("{case} response missing data: {response:?}"))?;
    match data.get(operation) {
        Some(Value::Null) => Ok(None),
        Some(Value::Object(obj)) => Ok(Some(obj.clone())),
        Some(other) => bail!("{case} returned unexpected {operation} payload: {other:?}"),
        None => bail!("{case} response missing {operation}: {response:?}"),
    }
}

fn assert_record_locator_shape(locator: &str) -> Result<()> {
    if locator.len() != 35
        || !locator.starts_with("rl_")
        || !locator[3..].chars().all(|ch| ch.is_ascii_hexdigit())
    {
        bail!("invalid record locator returned by API: {locator:?}");
    }
    Ok(())
}

async fn assert_activity_date_period_filter(
    ctx: &mut TestContext,
    suffix: &str,
    activity_subject: &str,
) -> Result<()> {
    let req = r#"
      query($filter: JSON, $sort: JSON, $skip: Int!, $limit: Int!){
        queryActivities(filter: $filter, sort: $sort, skip: $skip, limit: $limit){
          query_count
          items { id subject activity_date }
        }
      }
    "#;
    let response = ctx
        .execute_graphql(
            "pdsh_admin",
            "crm",
            req,
            Some(json!({
              "filter": {
                "_and": [
                  { "subject": { "_contains": suffix } },
                  { "activityDate": { "_during": "_today" } }
                ]
              },
              "sort": { "subject": "ASC" },
              "skip": 0,
              "limit": 5
            })),
        )
        .await?;
    let result = contracts::operation_object(&response, "queryActivities")
        .with_context(|| format!("date period response: {response:?}"))?;
    require_i64(result, "query_count", 1)?;
    let items = result
        .get("items")
        .and_then(Value::as_array)
        .context("date period items missing")?;
    contracts::require_item(items, "subject", activity_subject)
        .context("date period filter did not return created activity")?;
    Ok(())
}

async fn assert_invalid_filter_error_shape(ctx: &mut TestContext) -> Result<()> {
    let response = invalid_filter_response(ctx).await?;
    if response.get("errors").and_then(Value::as_array).is_none() {
        bail!("invalid filter did not return GraphQL errors: {response:?}");
    }
    Ok(())
}

async fn invalid_filter_response(ctx: &mut TestContext) -> Result<Value> {
    let req = r#"
      query($filter: JSON, $sort: JSON, $skip: Int!, $limit: Int!){
        queryIndustries(filter: $filter, sort: $sort, skip: $skip, limit: $limit){
          query_count
          items { id name }
        }
      }
    "#;
    let response = ctx
        .execute_graphql(
            "pdsh_admin",
            "crm",
            req,
            Some(json!({
              "filter": { "notARealField": { "_eq": "x" } },
              "sort": {},
              "skip": 0,
              "limit": 1
            })),
        )
        .await?;
    Ok(response)
}

async fn assert_pagination_edge_errors(ctx: &mut TestContext) -> Result<()> {
    let req = r#"
      query($filter: JSON, $sort: JSON, $skip: Int!, $limit: Int!){
        queryIndustries(filter: $filter, sort: $sort, skip: $skip, limit: $limit){
          query_count
          items { id name }
        }
      }
    "#;

    for (case, skip, limit) in [("negative skip", -1, 1), ("zero limit", 0, 0)] {
        let response = ctx
            .execute_graphql(
                "pdsh_admin",
                "crm",
                req,
                Some(json!({
                  "filter": {},
                  "sort": {},
                  "skip": skip,
                  "limit": limit
                })),
            )
            .await?;
        if response.get("errors").and_then(Value::as_array).is_none() {
            bail!("{case} did not return GraphQL errors: {response:?}");
        }
    }

    Ok(())
}

async fn assert_account_relationship_projections(
    ctx: &mut TestContext,
    account_name: &str,
    industry_name: &str,
    contact_first_name: &str,
) -> Result<()> {
    let req = r#"
      query($filter: JSON, $sort: JSON, $skip: Int!, $limit: Int!){
        queryAccounts(filter: $filter, sort: $sort, skip: $skip, limit: $limit){
          query_count
          items {
            id
            name
            industry { id name }
            contacts { id first_name last_name }
            version
          }
        }
      }
    "#;
    let variables = Some(json!({
      "filter": { "name": { "_eq": account_name } },
      "sort": { "name": "ASC" },
      "skip": 0,
      "limit": 1
    }));
    let mut last_error = String::new();
    let mut last_response = Value::Null;
    for attempt in 1..=10 {
        let response = ctx
            .execute_graphql("pdsh_admin", "crm", req, variables.clone())
            .await?;
        match assert_account_relationship_projection_response(
            &response,
            industry_name,
            contact_first_name,
        ) {
            Ok(()) => return Ok(()),
            Err(err) => {
                last_error = err.to_string();
                last_response = response;
                if attempt < 10 {
                    sleep(Duration::from_millis(200)).await;
                }
            }
        }
    }

    bail!(
        "account relationship projection did not include created contact after retries: {last_error}; last response: {last_response:?}"
    )
}

fn assert_account_relationship_projection_response(
    response: &Value,
    industry_name: &str,
    contact_first_name: &str,
) -> Result<()> {
    let result = contracts::operation_object(response, "queryAccounts")?;
    require_i64(result, "query_count", 1)?;
    let account = result
        .get("items")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(Value::as_object)
        .context("queryAccounts did not return account")?;
    let industry = account
        .get("industry")
        .and_then(Value::as_object)
        .context("account industry projection was missing")?;
    if industry.get("name").and_then(Value::as_str) != Some(industry_name) {
        bail!("expected projected industry {industry_name}, got {industry:?}");
    }

    let contacts = account
        .get("contacts")
        .and_then(Value::as_array)
        .context("account contacts projection was missing")?;
    let projected_contact = contracts::require_item(contacts, "first_name", contact_first_name)?;
    if projected_contact.get("last_name").and_then(Value::as_str) != Some("Relationship") {
        bail!("expected projected account contact Relationship, got {projected_contact:?}");
    }

    Ok(())
}

async fn assert_seeded_many_to_many_projection(ctx: &mut TestContext) -> Result<()> {
    let req = r#"
      query($id: String!){
        findOpportunity(id: $id){
          id
          name
          contacts { id first_name last_name }
        }
      }
    "#;
    let response = ctx
        .execute_graphql(
            "pdsh_admin",
            "crm",
            req,
            Some(json!({
              "id": "099c0000-0000-4000-8000-100000000001"
            })),
        )
        .await?;
    let opportunity = contracts::operation_object(&response, "findOpportunity")?;
    let contacts = opportunity
        .get("contacts")
        .and_then(Value::as_array)
        .context("seeded opportunity contacts projection was missing")?;
    let contact = contracts::require_item(contacts, "id", "c0c00000-0000-4000-8000-100000000001")?;
    if contact.get("first_name").and_then(Value::as_str) != Some("Sarah") {
        bail!("expected seeded many-to-many contact Sarah, got {contact:?}");
    }

    Ok(())
}

async fn assert_account_relationship_filters(
    ctx: &mut TestContext,
    account_name: &str,
    industry_name: &str,
    contact_first_name: &str,
) -> Result<()> {
    let req = r#"
      query($filter: JSON, $sort: JSON, $skip: Int!, $limit: Int!){
        queryAccounts(filter: $filter, sort: $sort, skip: $skip, limit: $limit){
          query_count
          items { id name }
        }
      }
    "#;

    for (case, filter) in [
        (
            "to-one industry filter",
            json!({ "industry": { "name": { "_eq": industry_name } } }),
        ),
        (
            "to-many contact filter",
            json!({ "contacts": { "firstName": { "_eq": contact_first_name } } }),
        ),
    ] {
        let response = ctx
            .execute_graphql(
                "pdsh_admin",
                "crm",
                req,
                Some(json!({
                  "filter": filter,
                  "sort": { "name": "ASC" },
                  "skip": 0,
                  "limit": 5
                })),
            )
            .await?;
        let result = contracts::operation_object(&response, "queryAccounts")?;
        let items = result
            .get("items")
            .and_then(Value::as_array)
            .with_context(|| format!("{case} items missing"))?;
        contracts::require_item(items, "name", account_name)
            .with_context(|| format!("{case} did not return account {account_name}"))?;
    }

    Ok(())
}

fn assert_account_audit_events(
    items: &[Value],
    account_id: &str,
    created_name: &str,
    updated_name: &str,
) -> Result<()> {
    if items.len() < 2 {
        bail!("expected at least create and update audit events, got {items:?}");
    }

    let create_event = find_audit_event_with_outcome(items, "create", "succeeded")?;
    let update_event = find_audit_event_with_outcome(items, "update", "succeeded")?;

    assert_audit_common(create_event, account_id, "create", "succeeded")?;
    assert_audit_common(update_event, account_id, "update", "succeeded")?;

    if create_event
        .get("after_json")
        .and_then(|value| value.get("name"))
        .and_then(Value::as_str)
        != Some(created_name)
    {
        bail!("create audit event did not capture created name: {create_event:?}");
    }
    if create_event
        .get("after_json")
        .and_then(|value| value.get("email"))
        != Some(&json!({ "_redacted": true }))
    {
        bail!("create audit event did not redact email: {create_event:?}");
    }

    let diff = update_event
        .get("diff_json")
        .and_then(Value::as_object)
        .context("update audit diff_json missing")?;
    if diff
        .get("name")
        .and_then(|value| value.get("before"))
        .and_then(Value::as_str)
        != Some(created_name)
        || diff
            .get("name")
            .and_then(|value| value.get("after"))
            .and_then(Value::as_str)
            != Some(updated_name)
    {
        bail!("update audit event did not capture name diff: {update_event:?}");
    }
    if diff.contains_key("email") {
        bail!("update audit diff exposed redacted email: {update_event:?}");
    }

    let redacted = update_event
        .get("redactions_json")
        .and_then(|value| value.get("redacted_properties"))
        .and_then(Value::as_array)
        .context("update audit redactions_json missing redacted_properties")?;
    if !redacted.iter().any(|value| value.as_str() == Some("email")) {
        bail!("update audit redactions did not include email: {update_event:?}");
    }

    let create_hash = string_field(create_event, "event_hash")?;
    let update_prev = string_field(update_event, "prev_hash")?;
    let update_hash = string_field(update_event, "event_hash")?;
    if update_prev != create_hash {
        bail!("audit chain discontinuity: prev={update_prev}, create_hash={create_hash}");
    }
    if update_hash == create_hash {
        bail!("audit update event hash should differ from create event hash");
    }

    if items
        .iter()
        .any(|item| item.get("outcome").and_then(Value::as_str) == Some("denied"))
    {
        bail!("target record chain exposed a denied attempt: {items:?}");
    }

    Ok(())
}

fn assert_actor_denied_audit_event(
    items: &[Value],
    account_id: &str,
    actor_tenant_id: &str,
    actor_user_name: &str,
    denied_update: (&str, &str),
    expected_cardinality: usize,
    expected_prev_hash: Option<&str>,
) -> Result<()> {
    let (denied_update_name, denied_update_email) = denied_update;
    if items.len() != expected_cardinality {
        bail!("expected exactly {expected_cardinality} actor-tenant audit events, got {items:?}");
    }
    let denied_event = find_audit_event_with_outcome(items, "update", "denied")?;
    assert_audit_common(denied_event, account_id, "update", "denied")?;
    if denied_event.get("tenant_id").and_then(Value::as_str) != Some(actor_tenant_id) {
        bail!("denied audit event tenant mismatch: {denied_event:?}");
    }
    if denied_event.get("actor_user_name").and_then(Value::as_str) != Some(actor_user_name) {
        bail!("denied audit event actor mismatch: {denied_event:?}");
    }
    if denied_event
        .get("after_json")
        .and_then(|value| value.get("name"))
        .and_then(Value::as_str)
        != Some(denied_update_name)
    {
        bail!("denied audit event did not capture attempted name: {denied_event:?}");
    }
    if denied_event
        .get("after_json")
        .and_then(|value| value.get("email"))
        != Some(&json!({ "_redacted": true }))
    {
        bail!("denied audit event did not redact attempted email: {denied_event:?}");
    }
    if Value::Object(denied_event.clone())
        .to_string()
        .contains(denied_update_email)
    {
        bail!("denied audit event leaked attempted email: {denied_event:?}");
    }
    if denied_event
        .get("policy_json")
        .and_then(|value| value.get("decision"))
        .and_then(|value| value.get("reason"))
        .and_then(Value::as_str)
        != Some("policy_denied")
    {
        bail!("denied audit event did not capture policy decision: {denied_event:?}");
    }
    match expected_prev_hash {
        Some(expected) => {
            if denied_event.get("prev_hash").and_then(Value::as_str) != Some(expected) {
                bail!("same-tenant denied event did not continue target chain: {denied_event:?}");
            }
        }
        None => {
            if denied_event
                .get("prev_hash")
                .is_some_and(|value| !value.is_null())
            {
                bail!(
                    "first actor-tenant denied event unexpectedly had a predecessor: {denied_event:?}"
                );
            }
        }
    }
    Ok(())
}

fn audit_event_hash<'a>(items: &'a [Value], action: &str, outcome: &str) -> Result<&'a str> {
    string_field(
        find_audit_event_with_outcome(items, action, outcome)?,
        "event_hash",
    )
}

fn assert_chain_extended_by_denied_event(
    before: &[Value],
    after: &[Value],
    case: &str,
) -> Result<()> {
    if after.len() != before.len() + 1 || !after.starts_with(before) {
        bail!("{case} did not append exactly one event: before={before:?}, after={after:?}");
    }
    Ok(())
}

fn assert_target_chain_unchanged(before: &[Value], after: &[Value], case: &str) -> Result<()> {
    if before != after {
        bail!("{case} changed the target tenant audit chain: before={before:?}, after={after:?}");
    }
    Ok(())
}

async fn assert_audit_items_absent(
    ctx: &mut TestContext,
    auth_token_name: &str,
    account_id: &str,
    case: &str,
) -> Result<()> {
    let items = account_audit_items_as(ctx, auth_token_name, account_id).await?;
    if !items.is_empty() {
        bail!("{case} exposed audit evidence for {account_id}: {items:?}");
    }
    Ok(())
}

async fn assert_account_audit_access_denied(
    ctx: &mut TestContext,
    auth_token_name: &str,
    account_id: &str,
    case: &str,
) -> Result<()> {
    let response = account_audit_response(ctx, auth_token_name, account_id).await?;
    assert_graphql_error_normalized(&response, case, Some("access"))
}

async fn account_audit_items_as(
    ctx: &mut TestContext,
    auth_token_name: &str,
    account_id: &str,
) -> Result<Vec<Value>> {
    let response = account_audit_response(ctx, auth_token_name, account_id).await?;
    let result = contracts::operation_object(&response, "queryAccountsAudit")
        .with_context(|| format!("audit query response: {response:?}"))?;
    result
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .context("audit query items missing")
}

async fn account_audit_response(
    ctx: &mut TestContext,
    auth_token_name: &str,
    account_id: &str,
) -> Result<Value> {
    let req = r#"
      query($filter: JSON, $sort: JSON, $skip: Int!, $limit: Int!){
        queryAccountsAudit(filter: $filter, sort: $sort, skip: $skip, limit: $limit){
          query_count
          items {
            audit_id
            occurred_at
            tenant_id
            actor_user_name
            action
            outcome
            record_id
            before_json
            after_json
            diff_json
            policy_json
            redactions_json
            chain_scope
            prev_hash
            event_hash
          }
        }
      }
    "#;
    ctx.execute_graphql(
        auth_token_name,
        "crm",
        req,
        Some(json!({
          "filter": { "recordId": { "_eq": account_id } },
          "sort": { "occurredAt": "ASC" },
          "skip": 0,
          "limit": 10
        })),
    )
    .await
}

async fn assert_account_audit_tenant_visibility(
    ctx: &mut TestContext,
    auth_token_name: &str,
    account_id: &str,
    expected_tenant_id: Option<&str>,
) -> Result<()> {
    let items = account_audit_items_as(ctx, auth_token_name, account_id).await?;
    match expected_tenant_id {
        Some(expected_tenant_id) => {
            if items.is_empty() {
                bail!(
                    "{auth_token_name} expected tenant-scoped audit rows for {account_id}, got none"
                );
            }
            for item in &items {
                if item.get("tenant_id").and_then(Value::as_str) != Some(expected_tenant_id) {
                    bail!(
                        "{auth_token_name} saw audit row outside tenant {expected_tenant_id}: {item:?}"
                    );
                }
            }
        }
        None => {
            if !items.is_empty() {
                bail!("{auth_token_name} saw cross-tenant audit rows for {account_id}: {items:?}");
            }
        }
    }
    Ok(())
}

fn find_audit_event_with_outcome<'a>(
    items: &'a [Value],
    action: &str,
    outcome: &str,
) -> Result<&'a Map<String, Value>> {
    items
        .iter()
        .filter_map(Value::as_object)
        .find(|item| {
            item.get("action").and_then(Value::as_str) == Some(action)
                && item.get("outcome").and_then(Value::as_str) == Some(outcome)
        })
        .with_context(|| format!("missing {action}/{outcome} audit event: {items:?}"))
}

fn assert_audit_common(
    event: &Map<String, Value>,
    account_id: &str,
    expected_action: &str,
    expected_outcome: &str,
) -> Result<()> {
    if event.get("action").and_then(Value::as_str) != Some(expected_action) {
        bail!("audit event action mismatch: {event:?}");
    }
    if event.get("outcome").and_then(Value::as_str) != Some(expected_outcome) {
        bail!("audit event outcome mismatch: {event:?}");
    }
    if event.get("record_id").and_then(Value::as_str) != Some(account_id) {
        bail!("audit event record_id mismatch: {event:?}");
    }
    let actor = string_field(event, "actor_user_name")?;
    if actor.trim().is_empty() {
        bail!("audit event actor_user_name was empty: {event:?}");
    }
    let chain_scope = string_field(event, "chain_scope")?;
    if !chain_scope.starts_with("crm.Account:") || !chain_scope.ends_with(account_id) {
        bail!("audit chain_scope did not include tenant/record scope: {event:?}");
    }
    if let Some(tenant_id) = event.get("tenant_id").and_then(Value::as_str) {
        if !tenant_id.is_empty() && !chain_scope.contains(tenant_id) {
            bail!("audit chain_scope did not include tenant_id {tenant_id}: {event:?}");
        }
    }
    let event_hash = string_field(event, "event_hash")?;
    if event_hash.len() < 32 {
        bail!("audit event_hash was unexpectedly short: {event:?}");
    }
    Ok(())
}

async fn create_industry(
    ctx: &mut TestContext,
    name: &str,
    sort_order: i64,
) -> Result<Map<String, Value>> {
    create_industry_as(ctx, "pdsh_admin", name, sort_order).await
}

async fn create_industry_as(
    ctx: &mut TestContext,
    auth_token_name: &str,
    name: &str,
    sort_order: i64,
) -> Result<Map<String, Value>> {
    create_industry_with_optional_id_as(ctx, auth_token_name, None, name, sort_order).await
}

async fn create_industry_with_id(
    ctx: &mut TestContext,
    id: &str,
    name: &str,
    sort_order: i64,
) -> Result<Map<String, Value>> {
    create_industry_with_optional_id_as(ctx, "pdsh_admin", Some(id), name, sort_order).await
}

async fn create_industry_with_optional_id_as(
    ctx: &mut TestContext,
    auth_token_name: &str,
    id: Option<&str>,
    name: &str,
    sort_order: i64,
) -> Result<Map<String, Value>> {
    let req = r#"
      mutation($input: InputIndustry!){
        createIndustry(input: $input){
          id,name,description,sort_order,version
        }
      }
    "#;
    let mut input = Map::new();
    if let Some(id) = id {
        input.insert("id".to_string(), json!(id));
    }
    input.insert("name".to_string(), json!(name));
    input.insert(
        "description".to_string(),
        json!("Provider semantic parity matrix"),
    );
    input.insert("sort_order".to_string(), json!(sort_order));
    input.insert("version".to_string(), json!(0));
    let response = ctx
        .execute_graphql(
            auth_token_name,
            "crm",
            req,
            Some(json!({ "input": Value::Object(input) })),
        )
        .await?;
    Ok(contracts::operation_object(&response, "createIndustry")?.clone())
}

async fn assert_duplicate_key_error_normalized(
    ctx: &mut TestContext,
    duplicate_id: &str,
    suffix: &str,
) -> Result<()> {
    let response = create_industry_raw_response(
        ctx,
        json!({
          "id": duplicate_id,
          "name": format!("Parity Matrix Duplicate Again {suffix}"),
          "description": "Provider duplicate-key normalization",
          "sort_order": 664,
          "version": 0
        }),
    )
    .await?;
    assert_graphql_error_normalized(
        &response,
        "duplicate key",
        Some("duplicate key: record already exists"),
    )
}

async fn assert_foreign_key_error_normalized(ctx: &mut TestContext, suffix: &str) -> Result<()> {
    let req = r#"
      mutation($input: InputAccount!){
        createAccount(input: $input){
          id name
        }
      }
    "#;
    let response = ctx
        .execute_graphql(
            "pdsh_admin",
            "crm",
            req,
            Some(json!({
              "input": {
                "name": format!("Parity Matrix Missing FK Account {suffix}"),
                "website": format!("https://missing-fk-{suffix}.example.com"),
                "email": format!("missing-fk-{suffix}@api-test.example.com"),
                "billing_city": "Irvine",
                "billing_state": "CA",
                "billing_country": "USA",
                "annual_revenue": 10.0,
                "number_of_employees": 1,
                "tags": [format!("parity-{suffix}-error")],
                "industry_id": unique_uuid()?,
                "version": 0
              }
            })),
        )
        .await?;
    assert_graphql_error_normalized(
        &response,
        "foreign key",
        Some("foreign key: record references a missing related record"),
    )
}

async fn assert_required_field_error_normalized(ctx: &mut TestContext, suffix: &str) -> Result<()> {
    let response = create_industry_raw_response(
        ctx,
        json!({
          "description": format!("Missing required name {suffix}"),
          "sort_order": 665,
          "version": 0
        }),
    )
    .await?;
    assert_graphql_error_normalized(&response, "required field", Some("name"))
}

async fn create_industry_raw_response(ctx: &mut TestContext, input: Value) -> Result<Value> {
    let req = r#"
      mutation($input: InputIndustry!){
        createIndustry(input: $input){
          id name version
        }
      }
    "#;
    ctx.execute_graphql("pdsh_admin", "crm", req, Some(json!({ "input": input })))
        .await
}

async fn create_account(
    ctx: &mut TestContext,
    name: &str,
    industry_id: &str,
    suffix: &str,
) -> Result<Map<String, Value>> {
    create_account_with_profile(
        ctx,
        name,
        industry_id,
        suffix,
        Some("949-555-0100"),
        42.0,
        7,
        "CA",
        &["relationship", "west"],
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn create_account_with_profile(
    ctx: &mut TestContext,
    name: &str,
    industry_id: &str,
    suffix: &str,
    phone: Option<&str>,
    annual_revenue: f64,
    number_of_employees: i64,
    billing_state: &str,
    tags: &[&str],
) -> Result<Map<String, Value>> {
    create_account_with_profile_as(
        ctx,
        "pdsh_admin",
        name,
        industry_id,
        suffix,
        phone,
        annual_revenue,
        number_of_employees,
        billing_state,
        tags,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn create_account_with_profile_as(
    ctx: &mut TestContext,
    auth_token_name: &str,
    name: &str,
    industry_id: &str,
    suffix: &str,
    phone: Option<&str>,
    annual_revenue: f64,
    number_of_employees: i64,
    billing_state: &str,
    tags: &[&str],
) -> Result<Map<String, Value>> {
    let req = r#"
      mutation($input: InputAccount!){
        createAccount(input: $input){
          id,name,website,phone,email,billing_city,billing_state,billing_country,
          annual_revenue,number_of_employees,tags,industry_id,version
        }
      }
    "#;
    let mut input = Map::new();
    input.insert("name".to_string(), json!(name));
    input.insert(
        "website".to_string(),
        json!(format!("https://parity-{suffix}.example.com")),
    );
    input.insert(
        "email".to_string(),
        json!(format!("parity-{suffix}@api-test.example.com")),
    );
    input.insert(
        "phone".to_string(),
        phone.map_or(Value::Null, |value| json!(value)),
    );
    input.insert("billing_city".to_string(), json!("Irvine"));
    input.insert("billing_state".to_string(), json!(billing_state));
    input.insert("billing_country".to_string(), json!("USA"));
    input.insert("annual_revenue".to_string(), json!(annual_revenue));
    input.insert(
        "number_of_employees".to_string(),
        json!(number_of_employees),
    );
    input.insert(
        "tags".to_string(),
        json!(tags
            .iter()
            .map(|tag| format!("parity-{suffix}-{tag}"))
            .collect::<Vec<_>>()),
    );
    input.insert("industry_id".to_string(), json!(industry_id));
    input.insert("version".to_string(), json!(0));

    let response = ctx
        .execute_graphql(
            auth_token_name,
            "crm",
            req,
            Some(json!({ "input": Value::Object(input) })),
        )
        .await?;
    Ok(contracts::operation_object(&response, "createAccount")?.clone())
}

async fn update_account_name_and_email_as(
    ctx: &mut TestContext,
    auth_token_name: &str,
    mut input: Map<String, Value>,
    name: &str,
    email: &str,
) -> Result<Map<String, Value>> {
    let req = r#"
      mutation($input: InputAccount!){
        updateAccount(input: $input){
          id,name,website,phone,email,billing_city,billing_state,billing_country,
          annual_revenue,number_of_employees,tags,industry_id,version
        }
      }
    "#;
    input.insert("name".to_string(), json!(name));
    input.insert("email".to_string(), json!(email));
    let response = ctx
        .execute_graphql(
            auth_token_name,
            "crm",
            req,
            Some(json!({ "input": Value::Object(input) })),
        )
        .await?;
    Ok(contracts::operation_object(&response, "updateAccount")?.clone())
}

async fn create_contact(
    ctx: &mut TestContext,
    first_name: &str,
    last_name: &str,
    account_id: &str,
) -> Result<Map<String, Value>> {
    let req = r#"
      mutation($input: InputContact!){
        createContact(input: $input){
          id,first_name,last_name,email,account_id,version
        }
      }
    "#;
    let response = ctx
        .execute_graphql(
            "pdsh_admin",
            "crm",
            req,
            Some(json!({
              "input": {
                "first_name": first_name,
                "last_name": last_name,
                "email": format!("{first_name}.{last_name}@api-test.example.com").to_ascii_lowercase(),
                "account_id": account_id,
                "version": 0
              }
            })),
        )
        .await?;
    Ok(contracts::operation_object(&response, "createContact")?.clone())
}

async fn create_activity(
    ctx: &mut TestContext,
    subject: &str,
    activity_date: &str,
    account_id: &str,
) -> Result<Map<String, Value>> {
    let req = r#"
      mutation($input: InputActivity!){
        createActivity(input: $input){
          id,subject,description,activity_date,is_closed,priority,status,type_id,account_id,version
        }
      }
    "#;
    let response = ctx
        .execute_graphql(
            "pdsh_admin",
            "crm",
            req,
            Some(json!({
              "input": {
                "subject": subject,
                "description": "Provider semantic date-period contract",
                "activity_date": activity_date,
                "is_closed": false,
                "priority": "Normal",
                "status": "Open",
                "type_id": "ac71ec00-0000-4000-8000-100000000001",
                "account_id": account_id,
                "version": 0
              }
            })),
        )
        .await?;
    Ok(contracts::operation_object(&response, "createActivity")?.clone())
}

async fn reject_stale_delete(ctx: &mut TestContext, input: Map<String, Value>) -> Result<()> {
    let response = stale_delete_response(ctx, input).await?;
    if response.get("errors").and_then(Value::as_array).is_none() {
        bail!("stale delete did not return GraphQL errors: {response:?}");
    }
    Ok(())
}

async fn stale_delete_response(
    ctx: &mut TestContext,
    mut input: Map<String, Value>,
) -> Result<Value> {
    let req = r#"
      mutation($input: InputIndustry!){
        deleteIndustry(input: $input)
      }
    "#;
    let stale_version = input
        .get("version")
        .and_then(Value::as_i64)
        .unwrap_or_default()
        + 1000;
    input.insert("version".to_string(), json!(stale_version));

    let response = ctx
        .execute_graphql(
            "pdsh_admin",
            "crm",
            req,
            Some(json!({ "input": Value::Object(input) })),
        )
        .await?;
    Ok(response)
}

async fn delete_industry(ctx: &mut TestContext, input: Map<String, Value>) -> Result<()> {
    delete_industry_as(ctx, "pdsh_admin", input).await
}

async fn delete_industry_as(
    ctx: &mut TestContext,
    auth_token_name: &str,
    input: Map<String, Value>,
) -> Result<()> {
    let req = r#"
      mutation($input: InputIndustry!){
        deleteIndustry(input: $input)
      }
    "#;
    let response = ctx
        .execute_graphql(
            auth_token_name,
            "crm",
            req,
            Some(json!({ "input": Value::Object(input) })),
        )
        .await?;
    let deleted = contracts::operation_i64(&response, "deleteIndustry")
        .with_context(|| format!("deleteIndustry response: {response:?}"))?;
    if deleted != 1 {
        bail!("deleteIndustry expected to delete 1 row, deleted {deleted}");
    }
    Ok(())
}

async fn delete_activity(ctx: &mut TestContext, input: Map<String, Value>) -> Result<()> {
    let req = r#"
      mutation($input: InputActivity!){
        deleteActivity(input: $input)
      }
    "#;
    let response = ctx
        .execute_graphql(
            "pdsh_admin",
            "crm",
            req,
            Some(json!({ "input": Value::Object(input) })),
        )
        .await?;
    let deleted = contracts::operation_i64(&response, "deleteActivity")
        .with_context(|| format!("deleteActivity response: {response:?}"))?;
    if deleted != 1 {
        bail!("deleteActivity expected to delete 1 row, deleted {deleted}");
    }
    Ok(())
}

async fn delete_account(ctx: &mut TestContext, input: Map<String, Value>) -> Result<()> {
    delete_account_as(ctx, "pdsh_admin", input).await
}

async fn delete_account_as(
    ctx: &mut TestContext,
    auth_token_name: &str,
    input: Map<String, Value>,
) -> Result<()> {
    let response = account_delete_response(ctx, auth_token_name, input).await?;
    let deleted = contracts::operation_i64(&response, "deleteAccount")
        .with_context(|| format!("deleteAccount response: {response:?}"))?;
    if deleted != 1 {
        bail!("deleteAccount expected to delete 1 row, deleted {deleted}");
    }
    Ok(())
}

async fn account_delete_response(
    ctx: &mut TestContext,
    auth_token_name: &str,
    input: Map<String, Value>,
) -> Result<Value> {
    let req = r#"
      mutation($input: InputAccount!){
        deleteAccount(input: $input)
      }
    "#;
    ctx.execute_graphql(
        auth_token_name,
        "crm",
        req,
        Some(json!({ "input": Value::Object(input) })),
    )
    .await
}

async fn delete_contact(ctx: &mut TestContext, input: Map<String, Value>) -> Result<()> {
    let req = r#"
      mutation($input: InputContact!){
        deleteContact(input: $input)
      }
    "#;
    let response = ctx
        .execute_graphql(
            "pdsh_admin",
            "crm",
            req,
            Some(json!({ "input": Value::Object(input) })),
        )
        .await?;
    let deleted = contracts::operation_i64(&response, "deleteContact")
        .with_context(|| format!("deleteContact response: {response:?}"))?;
    if deleted != 1 {
        bail!("deleteContact expected to delete 1 row, deleted {deleted}");
    }
    Ok(())
}

fn string_field<'a>(obj: &'a Map<String, Value>, field: &str) -> Result<&'a str> {
    obj.get(field)
        .and_then(Value::as_str)
        .with_context(|| format!("expected string field {field}: {obj:?}"))
}

fn require_i64(obj: &Map<String, Value>, field: &str, expected: i64) -> Result<()> {
    let actual = obj
        .get(field)
        .and_then(Value::as_i64)
        .with_context(|| format!("expected integer field {field}: {obj:?}"))?;
    if actual != expected {
        bail!("expected {field} = {expected}, got {actual}: {obj:?}");
    }
    Ok(())
}

fn require_string(obj: &Map<String, Value>, field: &str, expected: &str) -> Result<()> {
    let actual = obj
        .get(field)
        .and_then(Value::as_str)
        .with_context(|| format!("expected string field {field}: {obj:?}"))?;
    if actual != expected {
        bail!("expected {field} = {expected}, got {actual}: {obj:?}");
    }
    Ok(())
}

fn require_f64(obj: &Map<String, Value>, field: &str, expected: f64) -> Result<()> {
    let actual = obj
        .get(field)
        .and_then(Value::as_f64)
        .with_context(|| format!("expected numeric field {field}: {obj:?}"))?;
    if (actual - expected).abs() > f64::EPSILON {
        bail!("expected {field} = {expected}, got {actual}: {obj:?}");
    }
    Ok(())
}

fn assert_graphql_error_normalized(
    response: &Value,
    case: &str,
    expected_fragment: Option<&str>,
) -> Result<()> {
    let errors = response
        .get("errors")
        .and_then(Value::as_array)
        .with_context(|| format!("{case} did not return GraphQL errors: {response:?}"))?;
    if errors.is_empty() {
        bail!("{case} returned an empty GraphQL errors array: {response:?}");
    }
    let messages = errors
        .iter()
        .filter_map(|error| error.get("message").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join(" | ");
    if messages.trim().is_empty() {
        bail!("{case} GraphQL errors did not include messages: {errors:?}");
    }
    if let Some(expected_fragment) = expected_fragment {
        if !messages
            .to_ascii_lowercase()
            .contains(&expected_fragment.to_ascii_lowercase())
        {
            bail!("{case} expected error to mention {expected_fragment:?}, got {messages:?}");
        }
    }

    let provider_leak_markers = [
        "e11000",
        "sqlstate",
        "sql server",
        "postgres",
        "postgresql",
        "mongo server error",
        "duplicate key value violates",
        "violates foreign key",
        "mongoservererror",
        "constraint",
        "tiberius",
        "tokio-postgres",
        "odbc",
        "snowflake error",
        "snowflake sql api",
        "java.sql",
    ];
    let lower = messages.to_ascii_lowercase();
    if let Some(marker) = provider_leak_markers
        .iter()
        .find(|marker| lower.contains(**marker))
    {
        bail!("{case} leaked provider-native marker {marker:?}: {messages:?}");
    }

    Ok(())
}

fn unique_suffix() -> Result<String> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system time before unix epoch")?
        .as_millis();
    let counter = UNIQUE_SUFFIX_COUNTER.fetch_add(1, Ordering::Relaxed);
    Ok(format!("{millis}-{counter}"))
}

fn unique_uuid() -> Result<String> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system time before unix epoch")?
        .as_nanos();
    let counter = UNIQUE_SUFFIX_COUNTER.fetch_add(1, Ordering::Relaxed) as u128;
    let value = nanos ^ (counter << 32);
    Ok(format!(
        "{:08x}-{:04x}-4{:03x}-8{:03x}-{:012x}",
        (value & 0xffff_ffff) as u32,
        ((value >> 32) & 0xffff) as u16,
        ((value >> 48) & 0x0fff) as u16,
        ((value >> 60) & 0x0fff) as u16,
        ((value >> 72) & 0xffff_ffff_ffff) as u64
    ))
}

fn policy_contract_auth_available(contract: &str) -> Result<bool> {
    if auth_mode_has_named_policy_users() {
        return Ok(true);
    }

    if provider_certification_enabled() {
        bail!(
            "{contract} requires API_TEST_AUTH_MODE=local_dev or token_files when API_TEST_PROVIDER_CERTIFICATION=1"
        );
    }

    eprintln!("skipping {contract}; set API_TEST_AUTH_MODE=local_dev or token_files");
    Ok(false)
}

fn auth_mode_has_named_policy_users() -> bool {
    env::var("API_TEST_AUTH_MODE")
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "local_dev" | "local-dev" | "local" | "token_files" | "token-files" | "files"
            )
        })
        .unwrap_or(false)
}

fn today_utc_date_string() -> Result<String> {
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system time before unix epoch")?
        .as_secs()
        / 86_400;
    let (year, month, day) = civil_from_days(days as i64);
    Ok(format!("{year:04}-{month:02}-{day:02}"))
}

fn civil_from_days(days_since_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let month_prime = (5 * doy + 2) / 153;
    let day = doy - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };
    (year as i32, month as u32, day as u32)
}
