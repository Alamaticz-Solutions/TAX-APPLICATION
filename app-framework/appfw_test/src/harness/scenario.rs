use anyhow::{bail, Context, Result};
use serde_json::{Map, Value};

use super::{
    assertions, config::ApiTestConfig, graphql_client::GraphqlClient, result_store::ResultStore,
};

#[derive(Debug)]
pub struct TestContext {
    client: GraphqlClient,
    results: ResultStore,
}

impl TestContext {
    pub fn from_env() -> Result<Self> {
        Self::new(ApiTestConfig::from_env()?)
    }

    pub fn new(config: ApiTestConfig) -> Result<Self> {
        Ok(Self {
            client: GraphqlClient::new(config)?,
            results: ResultStore::default(),
        })
    }

    pub async fn execute_graphql(
        &self,
        auth_token_name: &str,
        schema_name: &str,
        gql_request: &str,
        variables: Option<Value>,
    ) -> Result<Value> {
        self.client
            .execute(auth_token_name, schema_name, gql_request, variables)
            .await
    }

    pub fn result_value(&self, reference: &str) -> Result<Value> {
        self.results.value(reference)
    }

    pub fn result_map(
        &self,
        reference: &str,
        overwrite: Option<Map<String, Value>>,
    ) -> Result<Map<String, Value>> {
        self.results.map(reference, overwrite)
    }

    pub fn expect_data(
        &mut self,
        test_name: &str,
        response: &Value,
        operation_name: &str,
        expected: Value,
        result_name: Option<&str>,
    ) -> Result<()> {
        let expected_values = expected
            .as_object()
            .with_context(|| format!("{test_name} expected data must be a JSON object"))?;
        let actual_values = response
            .get("data")
            .and_then(|data| data.get(operation_name))
            .and_then(Value::as_object)
            .with_context(|| {
                format!("{test_name} response did not contain data.{operation_name} object")
            })?;
        let diffs = assertions::partial_eq(actual_values, expected_values);

        if !diffs.is_empty() {
            bail!("{test_name} data assertion failed: {diffs:#?}");
        }

        if let Some(result_name) = result_name {
            self.results
                .set(result_name, Value::Object(actual_values.clone()));
        }

        Ok(())
    }

    pub fn expect_error(&self, test_name: &str, response: &Value, expected: Value) -> Result<()> {
        let expected_values = expected
            .as_object()
            .with_context(|| format!("{test_name} expected error must be a JSON object"))?;
        let errors = response
            .get("errors")
            .and_then(Value::as_array)
            .with_context(|| format!("{test_name} response did not contain an errors array"))?;

        let mut mismatches = Vec::new();
        for error in errors {
            let error_object = error
                .as_object()
                .with_context(|| format!("{test_name} response contained a non-object error"))?;
            let diffs = assertions::partial_eq(error_object, expected_values);
            if diffs.is_empty() {
                return Ok(());
            }
            mismatches.push(diffs);
        }

        bail!(
            "{test_name} error assertion failed; no error matched expected fields: {mismatches:#?}"
        )
    }
}
