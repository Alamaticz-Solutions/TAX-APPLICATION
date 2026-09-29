//! Live Neo4j graph client for the CRM product (read + governed write),
//! consumed by the Account custom-method handlers.

pub(crate) mod provider;
pub(crate) mod queries;

pub(crate) use provider::{read_account_graph, write_account_link};
pub(crate) use queries::{
    ACCOUNT_RELATIONSHIP_GRAPH, LINK_ACCOUNT_REFERRAL, UNLINK_ACCOUNT_REFERRAL,
};
