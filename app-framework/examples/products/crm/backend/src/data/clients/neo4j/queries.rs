//! Product-owned, vetted named graph operations for the CRM `neo4j_graph`
//! data source. These are the only Cypher statements the graph providers will
//! execute; every value is bound as a parameter and `$tenant_id` is bound by the
//! provider from the authenticated user.

use appfw_provider_neo4j::{
    Neo4jGraphMutation, Neo4jGraphQuery, Neo4jGraphQueryLimits, Neo4jParameter,
};
use serde_json::Value;

pub(crate) const ACCOUNT_RELATIONSHIP_GRAPH: &str = "account_relationship_graph";
pub(crate) const LINK_ACCOUNT_REFERRAL: &str = "link_account_referral";
pub(crate) const UNLINK_ACCOUNT_REFERRAL: &str = "unlink_account_referral";

const READ_LIMITS: Neo4jGraphQueryLimits = Neo4jGraphQueryLimits {
    max_depth: 2,
    max_results: 200,
    timeout_ms: 4_000,
};

const WRITE_LIMITS: Neo4jGraphQueryLimits = Neo4jGraphQueryLimits {
    max_depth: 1,
    max_results: 25,
    timeout_ms: 4_000,
};

/// Read named queries registered on the graph read provider.
pub(crate) fn read_queries() -> Vec<Neo4jGraphQuery> {
    vec![Neo4jGraphQuery::new(
        ACCOUNT_RELATIONSHIP_GRAPH,
        "MATCH (a:Account {tenant_id: $tenant_id, record_locator: $account}) \
         OPTIONAL MATCH (a)-[:REFERRED|PARENT_OF]->(related:Account {tenant_id: $tenant_id}) \
         OPTIONAL MATCH (contact:Contact {tenant_id: $tenant_id})-[:WORKS_AT]->(a) \
         RETURN a{.record_locator, .name} AS account, \
                collect(DISTINCT related{.record_locator, .name}) AS related, \
                collect(DISTINCT contact{.record_locator, .name}) AS contacts",
        vec![Neo4jParameter::new("account", Value::Null)],
        READ_LIMITS,
    )
    .expect("account_relationship_graph is a valid read-only query")
    .with_start_node_parameter("account")]
}

/// Governed relationship-write mutations registered on the graph write provider.
pub(crate) fn write_mutations() -> Vec<Neo4jGraphMutation> {
    vec![
        Neo4jGraphMutation::new(
            LINK_ACCOUNT_REFERRAL,
            "MATCH (a:Account {tenant_id: $tenant_id, record_locator: $from_account}), \
             (b:Account {tenant_id: $tenant_id, record_locator: $to_account}) \
             MERGE (a)-[r:REFERRED {tenant_id: $tenant_id}]->(b) \
             RETURN a.record_locator AS from, b.record_locator AS to",
            vec![
                Neo4jParameter::new("from_account", Value::Null),
                Neo4jParameter::new("to_account", Value::Null),
            ],
            WRITE_LIMITS,
        )
        .expect("link_account_referral is a valid governed write")
        .with_start_node_parameter("from_account"),
        Neo4jGraphMutation::new(
            UNLINK_ACCOUNT_REFERRAL,
            "MATCH (a:Account {tenant_id: $tenant_id, record_locator: $from_account}) \
             -[r:REFERRED]->(b:Account {tenant_id: $tenant_id, record_locator: $to_account}) \
             DELETE r \
             RETURN a.record_locator AS from, b.record_locator AS to",
            vec![
                Neo4jParameter::new("from_account", Value::Null),
                Neo4jParameter::new("to_account", Value::Null),
            ],
            WRITE_LIMITS,
        )
        .expect("unlink_account_referral is a valid governed write")
        .with_start_node_parameter("from_account"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_graph_operations_construct() {
        assert_eq!(read_queries().len(), 1);
        assert_eq!(write_mutations().len(), 2);
    }
}
