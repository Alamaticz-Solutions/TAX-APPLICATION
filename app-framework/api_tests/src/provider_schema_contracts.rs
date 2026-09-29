// Live provider certification contracts for the generated CRM API.
//
// These tests intentionally do not name a database provider. They run against
// the backend currently selected by configuration, so Postgres, MongoDB, MS SQL,
// and Snowflake have to satisfy the same generated API behavior.

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use anyhow::{bail, Context, Result};
    use serde_json::{json, Map, Value};

    use crate::harness::{contracts, should_run_provider_certification, TestContext};

    #[tokio::test]
    async fn provider_crud_filter_sort_and_concurrency_contract() -> Result<()> {
        if !should_run_provider_certification("provider_crud_filter_sort_and_concurrency_contract")
        {
            return Ok(());
        }

        let mut ctx = TestContext::from_env()?;
        let suffix = unique_suffix()?;
        let alpha_name = format!("Contract Industry Alpha {suffix}");
        let beta_name = format!("Contract Industry Beta {suffix}");

        let alpha = create_industry(&mut ctx, &alpha_name, 870).await?;
        let beta = create_industry(&mut ctx, &beta_name, 871).await?;

        find_industry(&mut ctx, string_field(&alpha, "id")?, &alpha_name).await?;
        query_industries_by_filter_and_sort(&mut ctx, &suffix, &beta_name, &alpha_name).await?;
        reject_stale_update(&mut ctx, alpha.clone()).await?;

        delete_industry(&mut ctx, alpha).await?;
        delete_industry(&mut ctx, beta).await?;

        Ok(())
    }

    async fn create_industry(
        ctx: &mut TestContext,
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
        let variables = Some(json!({
          "input": {
            "name": name,
            "description": "Generated API provider contract",
            "sort_order": sort_order,
            "version": 0
          }
        }));

        let response = ctx
            .execute_graphql("pdsh_admin", "crm", req, variables)
            .await?;
        let created = contracts::operation_object(&response, "createIndustry")?;
        if created.get("name").and_then(Value::as_str) != Some(name) {
            bail!("created industry name mismatch: {created:?}");
        }
        Ok(created.clone())
    }

    async fn find_industry(ctx: &mut TestContext, id: &str, expected_name: &str) -> Result<()> {
        let req = r#"
          query($id: String!){
            findIndustry(id: $id){
              id,name,description,sort_order,version
            }
          }
        "#;
        let response = ctx
            .execute_graphql("pdsh_admin", "crm", req, Some(json!({ "id": id })))
            .await?;
        let found = contracts::operation_object(&response, "findIndustry")?;
        if found.get("name").and_then(Value::as_str) != Some(expected_name) {
            bail!("findIndustry returned wrong record: {found:?}");
        }
        Ok(())
    }

    async fn query_industries_by_filter_and_sort(
        ctx: &mut TestContext,
        suffix: &str,
        expected_first: &str,
        expected_second: &str,
    ) -> Result<()> {
        let req = r#"
          query($filter: JSON, $sort: JSON, $skip: Int!, $limit: Int!){
            queryIndustries(filter: $filter, sort: $sort, skip: $skip, limit: $limit){
              query_count
              items { id,name,description,sort_order,version }
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
                  "limit": 10
                })),
            )
            .await?;
        let result = contracts::operation_object(&response, "queryIndustries")?;
        let items = result
            .get("items")
            .and_then(Value::as_array)
            .context("queryIndustries.items was not an array")?;

        let first = items
            .first()
            .and_then(Value::as_object)
            .context("queryIndustries returned no items")?;
        let second = items
            .get(1)
            .and_then(Value::as_object)
            .context("queryIndustries returned fewer than two items")?;

        if first.get("name").and_then(Value::as_str) != Some(expected_first) {
            bail!("sort contract expected first item {expected_first}, got {first:?}");
        }
        if second.get("name").and_then(Value::as_str) != Some(expected_second) {
            bail!("sort contract expected second item {expected_second}, got {second:?}");
        }

        contracts::require_item(items, "name", expected_first)?;
        contracts::require_item(items, "name", expected_second)?;

        Ok(())
    }

    async fn reject_stale_update(
        ctx: &mut TestContext,
        mut input: Map<String, Value>,
    ) -> Result<()> {
        let req = r#"
          mutation($input: InputIndustry!){
            updateIndustry(input: $input){
              id,name,description,sort_order,version
            }
          }
        "#;
        let stale_version = input
            .get("version")
            .and_then(Value::as_i64)
            .unwrap_or_default()
            + 1000;
        input.insert(
            "description".to_string(),
            Value::String("stale update should fail".to_string()),
        );
        input.insert("version".to_string(), json!(stale_version));

        let response = ctx
            .execute_graphql(
                "pdsh_admin",
                "crm",
                req,
                Some(json!({ "input": Value::Object(input) })),
            )
            .await?;
        ctx.expect_error(
            "provider_stale_update_contract",
            &response,
            json!({ "message": "invalid key or version", "path": ["updateIndustry"] }),
        )
    }

    async fn delete_industry(ctx: &mut TestContext, input: Map<String, Value>) -> Result<()> {
        let req = r#"
          mutation($input: InputIndustry!){
            deleteIndustry(input: $input)
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
        let deleted = contracts::operation_i64(&response, "deleteIndustry")
            .with_context(|| format!("deleteIndustry response: {response:?}"))?;
        if deleted != 1 {
            bail!("deleteIndustry expected to delete 1 row, deleted {deleted}");
        }
        Ok(())
    }

    fn string_field<'a>(obj: &'a Map<String, Value>, field: &str) -> Result<&'a str> {
        obj.get(field)
            .and_then(Value::as_str)
            .with_context(|| format!("expected string field {field}: {obj:?}"))
    }

    fn unique_suffix() -> Result<String> {
        Ok(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system time before unix epoch")?
            .as_millis()
            .to_string())
    }
}
