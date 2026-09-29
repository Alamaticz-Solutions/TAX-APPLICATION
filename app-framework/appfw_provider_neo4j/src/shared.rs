//! Helpers shared by the read and write graph providers so both enforce the
//! exact same server-bound-tenant parameter binding, error mapping, and
//! cypher-normalization rules.

use std::collections::{BTreeMap, BTreeSet};

use appfw_runtime::{RuntimeError, UserAuth};
use serde_json::Value;

use crate::execution::Neo4jExecutionError;
use crate::query::Neo4jParameter;

/// Build the bound parameter map for an execution: registered defaults overlaid
/// by caller values for declared names only, with the tenant parameter bound
/// from the authenticated user (never from caller input).
pub(crate) fn merge_bound_parameters(
    op_name: &str,
    tenant_parameter: Option<&str>,
    declared: &BTreeSet<String>,
    defaults: &[Neo4jParameter],
    supplied: &BTreeMap<String, Value>,
    user: &UserAuth,
) -> Result<BTreeMap<String, Value>, RuntimeError> {
    let mut parameters: BTreeMap<String, Value> = defaults
        .iter()
        .map(|parameter| (parameter.name.clone(), parameter.value.clone()))
        .collect();

    for (name, value) in supplied {
        if tenant_parameter == Some(name.as_str()) {
            return Err(RuntimeError::Validation(format!(
                "tenant parameter `${name}` is server-controlled and cannot be supplied by the caller"
            )));
        }
        if !declared.contains(name) {
            return Err(RuntimeError::Validation(format!(
                "graph operation `{op_name}` does not declare parameter `${name}`"
            )));
        }
        parameters.insert(name.clone(), value.clone());
    }

    if let Some(tenant_parameter) = tenant_parameter {
        parameters.insert(
            tenant_parameter.to_string(),
            Value::String(user.tenant_id.clone()),
        );
    }

    Ok(parameters)
}

pub(crate) fn map_execution_error(error: Neo4jExecutionError) -> RuntimeError {
    match error {
        Neo4jExecutionError::Connectivity(message) => {
            RuntimeError::DataAccess(format!("neo4j connectivity error: {message}"))
        }
        Neo4jExecutionError::Query(message) => {
            RuntimeError::DataAccess(format!("neo4j query error: {message}"))
        }
    }
}

pub(crate) fn normalize_ws(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn value_to_locator(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}
