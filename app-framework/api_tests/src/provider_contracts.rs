use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use serde_json::{json, Map, Value};

use crate::harness::{contracts, should_run_provider_certification, TestContext};

#[tokio::test]
async fn aggregate_accounts_contract() -> Result<()> {
    if !should_run_provider_certification("aggregate_accounts_contract") {
        return Ok(());
    }

    let mut ctx = TestContext::from_env()?;
    let suffix = unique_suffix()?;
    let industry_name = format!("Aggregate Contract Industry {suffix}");
    let mut created_accounts = Vec::new();
    let mut created_industry = None;

    let test_result = async {
        let industry = create_industry(&mut ctx, &industry_name).await?;
        let industry_id = string_field(&industry, "id")?.to_string();
        created_industry = Some(industry);

        created_accounts.push(
            create_account(
                &mut ctx,
                &format!("Aggregate Contract Alpha {suffix}"),
                &industry_id,
                "CA",
                "Irvine",
                100.0,
                10,
                &suffix,
            )
            .await?,
        );
        created_accounts.push(
            create_account(
                &mut ctx,
                &format!("Aggregate Contract Beta {suffix}"),
                &industry_id,
                "CA",
                "San Diego",
                300.0,
                15,
                &suffix,
            )
            .await?,
        );
        created_accounts.push(
            create_account(
                &mut ctx,
                &format!("Aggregate Contract Gamma {suffix}"),
                &industry_id,
                "NY",
                "New York",
                50.0,
                5,
                &suffix,
            )
            .await?,
        );

        assert_account_aggregation(&mut ctx, &suffix).await
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

async fn create_industry(ctx: &mut TestContext, name: &str) -> Result<Map<String, Value>> {
    let req = r#"
      mutation($input: InputIndustry!){
        createIndustry(input: $input){
          id,name,description,sort_order,version
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
                "name": name,
                "description": "Generated API aggregate contract",
                "sort_order": 998,
                "version": 0
              }
            })),
        )
        .await?;
    Ok(contracts::operation_object(&response, "createIndustry")?.clone())
}

#[allow(clippy::too_many_arguments)]
async fn create_account(
    ctx: &mut TestContext,
    name: &str,
    industry_id: &str,
    billing_state: &str,
    billing_city: &str,
    annual_revenue: f64,
    employees: i64,
    suffix: &str,
) -> Result<Map<String, Value>> {
    let req = r#"
      mutation($input: InputAccount!){
        createAccount(input: $input){
          id,name,website,phone,email,billing_city,billing_state,billing_country,
          annual_revenue,number_of_employees,industry_id,version
        }
      }
    "#;
    let email_name = name.to_ascii_lowercase().replace([' ', '-'], ".");
    let response = ctx
        .execute_graphql(
            "pdsh_admin",
            "crm",
            req,
            Some(json!({
              "input": {
                "name": name,
                "website": format!("https://aggregate-{suffix}.example.com"),
                "phone": "555-0177",
                "email": format!("{email_name}@api-test.example.com"),
                "billing_city": billing_city,
                "billing_state": billing_state,
                "billing_country": "USA",
                "annual_revenue": annual_revenue,
                "number_of_employees": employees,
                "industry_id": industry_id,
                "version": 0
              }
            })),
        )
        .await?;
    Ok(contracts::operation_object(&response, "createAccount")?.clone())
}

async fn assert_account_aggregation(ctx: &mut TestContext, suffix: &str) -> Result<()> {
    let req = r#"
      query(
        $filter: JSON,
        $groupBy: JSON,
        $metrics: JSON,
        $having: JSON,
        $sort: JSON,
        $skip: Int!,
        $limit: Int!
      ){
        aggregateAccounts(
          filter: $filter,
          groupBy: $groupBy,
          metrics: $metrics,
          having: $having,
          sort: $sort,
          skip: $skip,
          limit: $limit
        ){
          query_count
          page_count
          page_index
          items
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
              "groupBy": [{ "field": "billingState", "alias": "state" }],
              "metrics": [
                { "fn": "count", "alias": "account_count" },
                { "fn": "sum", "field": "annualRevenue", "alias": "total_revenue" },
                { "fn": "avg", "field": "numberOfEmployees", "alias": "avg_employees" },
                { "fn": "min", "field": "annualRevenue", "alias": "min_revenue" },
                { "fn": "max", "field": "annualRevenue", "alias": "max_revenue" },
                { "fn": "count_distinct", "field": "billingCity", "alias": "city_count" }
              ],
              "having": { "total_revenue": { "_gte": 200.0 } },
              "sort": { "total_revenue": "DESC" },
              "skip": 0,
              "limit": 10
            })),
        )
        .await?;

    let aggregate = contracts::operation_object(&response, "aggregateAccounts")?;
    let query_count = aggregate
        .get("query_count")
        .and_then(Value::as_i64)
        .context("aggregateAccounts.query_count was not an integer")?;
    if query_count != 1 {
        bail!("expected one grouped aggregate row after having, got {query_count}: {aggregate:?}");
    }

    let items = aggregate
        .get("items")
        .and_then(Value::as_array)
        .context("aggregateAccounts.items was not an array")?;
    let row = items
        .first()
        .and_then(Value::as_object)
        .context("aggregateAccounts returned no aggregate rows")?;

    require_string(row, "state", "CA")?;
    require_number(row, "account_count", 2.0)?;
    require_number(row, "total_revenue", 400.0)?;
    require_number(row, "avg_employees", 12.5)?;
    require_number(row, "min_revenue", 100.0)?;
    require_number(row, "max_revenue", 300.0)?;
    require_number(row, "city_count", 2.0)?;

    Ok(())
}

async fn delete_account(ctx: &mut TestContext, input: Map<String, Value>) -> Result<()> {
    let req = r#"
      mutation($input: InputAccount!){
        deleteAccount(input: $input)
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
    let deleted = contracts::operation_i64(&response, "deleteAccount")?;
    if deleted != 1 {
        bail!("deleteAccount expected to delete 1 row, deleted {deleted}");
    }
    Ok(())
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
    let deleted = contracts::operation_i64(&response, "deleteIndustry")?;
    if deleted != 1 {
        bail!("deleteIndustry expected to delete 1 row, deleted {deleted}");
    }
    Ok(())
}

fn require_string(row: &Map<String, Value>, field: &str, expected: &str) -> Result<()> {
    let actual = row
        .get(field)
        .and_then(Value::as_str)
        .with_context(|| format!("aggregate field {field} was not a string: {row:?}"))?;
    if actual != expected {
        bail!("aggregate field {field} expected {expected:?}, got {actual:?}");
    }
    Ok(())
}

fn require_number(row: &Map<String, Value>, field: &str, expected: f64) -> Result<()> {
    let actual = numeric_value(row.get(field))
        .with_context(|| format!("aggregate field {field} was not numeric: {row:?}"))?;
    if (actual - expected).abs() > 0.0001 {
        bail!("aggregate field {field} expected {expected}, got {actual}");
    }
    Ok(())
}

fn numeric_value(value: Option<&Value>) -> Option<f64> {
    match value? {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.parse::<f64>().ok(),
        _ => None,
    }
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
        .as_nanos()
        .to_string())
}
