fn main() -> anyhow::Result<()> {
    let policy_path = rego_test::fixture_policy_path("scheduling/resource.rego");
    let input = rego_test::AccessInput::for_action(
        "scheduling",
        "resource",
        rego_test::AccessAction::Read,
        rego_test::AccessUser::with_roles(
            rego_test::PDS_TENANT_ID,
            "alex",
            &["operations_manager"],
        ),
    );

    let access = rego_test::evaluate_access(policy_path, &input)?;
    println!("{}", serde_json::to_string_pretty(&access)?);

    Ok(())
}
