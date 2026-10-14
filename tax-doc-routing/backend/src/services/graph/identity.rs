//! Provider identity for Microsoft Graph: every request URL is built from
//! `GRAPH_BASE`. There is deliberately no descriptor struct here - nothing
//! consumes provider identity as a value today.

pub const GRAPH_BASE: &str = "https://graph.microsoft.com/v1.0";
