//! Provider identity for the SharePoint client. Same base URL as
//! `services::graph::identity::GRAPH_BASE` - SharePoint document libraries are
//! reached through the Microsoft Graph surface (`/sites`, `/drives`), not a
//! distinct SharePoint REST endpoint. Kept as its own constant so this module
//! has no compile-time dependency on `services::graph::identity`.

pub const SHAREPOINT_GRAPH_BASE: &str = "https://graph.microsoft.com/v1.0";

#[cfg(test)]
mod tests {
    #[test]
    fn base_matches_the_shared_graph_base() {
        assert_eq!(
            super::SHAREPOINT_GRAPH_BASE,
            crate::services::graph::identity::GRAPH_BASE
        );
    }
}
