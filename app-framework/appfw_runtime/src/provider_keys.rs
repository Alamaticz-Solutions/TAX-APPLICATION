use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FrameworkProvider {
    Postgres,
    Mongo,
    Mssql,
    FabricSqlAnalytics,
    Snowflake,
    Neo4j,
    ServiceNow,
    Workday,
    Icims,
    Salesforce,
    Anaplan,
    OracleFinancials,
    AiSearch,
}

impl FrameworkProvider {
    pub const DATABASE: [FrameworkProvider; 5] = [
        FrameworkProvider::Postgres,
        FrameworkProvider::Mongo,
        FrameworkProvider::Mssql,
        FrameworkProvider::FabricSqlAnalytics,
        FrameworkProvider::Snowflake,
    ];
    pub const GRAPH_READ: [FrameworkProvider; 1] = [FrameworkProvider::Neo4j];
    pub const EXTERNAL_API: [FrameworkProvider; 6] = [
        FrameworkProvider::ServiceNow,
        FrameworkProvider::Workday,
        FrameworkProvider::Icims,
        FrameworkProvider::Salesforce,
        FrameworkProvider::Anaplan,
        FrameworkProvider::OracleFinancials,
    ];
    pub const AI_SEARCH: [FrameworkProvider; 1] = [FrameworkProvider::AiSearch];
    pub const ALL: [FrameworkProvider; 13] = [
        FrameworkProvider::Postgres,
        FrameworkProvider::Mongo,
        FrameworkProvider::Mssql,
        FrameworkProvider::FabricSqlAnalytics,
        FrameworkProvider::Snowflake,
        FrameworkProvider::Neo4j,
        FrameworkProvider::ServiceNow,
        FrameworkProvider::Workday,
        FrameworkProvider::Icims,
        FrameworkProvider::Salesforce,
        FrameworkProvider::Anaplan,
        FrameworkProvider::OracleFinancials,
        FrameworkProvider::AiSearch,
    ];

    pub fn key(self) -> &'static str {
        match self {
            FrameworkProvider::Postgres => "postgres",
            FrameworkProvider::Mongo => "mongo",
            FrameworkProvider::Mssql => "mssql",
            FrameworkProvider::FabricSqlAnalytics => "fabric_sql_analytics",
            FrameworkProvider::Snowflake => "snowflake",
            FrameworkProvider::Neo4j => "neo4j",
            FrameworkProvider::ServiceNow => "servicenow",
            FrameworkProvider::Workday => "workday",
            FrameworkProvider::Icims => "icims",
            FrameworkProvider::Salesforce => "salesforce",
            FrameworkProvider::Anaplan => "anaplan",
            FrameworkProvider::OracleFinancials => "oracle_financials",
            FrameworkProvider::AiSearch => "ai_search",
        }
    }

    pub fn is_database_provider(self) -> bool {
        Self::DATABASE.contains(&self)
    }

    pub fn is_graph_read_provider(self) -> bool {
        Self::GRAPH_READ.contains(&self)
    }

    pub fn is_external_api_provider(self) -> bool {
        Self::EXTERNAL_API.contains(&self)
    }

    pub fn is_ai_search_provider(self) -> bool {
        Self::AI_SEARCH.contains(&self)
    }

    pub fn parse_key(provider: &str) -> Result<Self, String> {
        match provider {
            "postgres" => Ok(FrameworkProvider::Postgres),
            "mongo" => Ok(FrameworkProvider::Mongo),
            "mssql" => Ok(FrameworkProvider::Mssql),
            "fabric_sql_analytics" | "fabric-sql-analytics" | "fabric" => {
                Ok(FrameworkProvider::FabricSqlAnalytics)
            }
            "snowflake" => Ok(FrameworkProvider::Snowflake),
            "neo4j" => Ok(FrameworkProvider::Neo4j),
            "servicenow" | "service_now" | "service-now" => Ok(FrameworkProvider::ServiceNow),
            "workday" => Ok(FrameworkProvider::Workday),
            "icims" | "i_cims" | "i-cims" => Ok(FrameworkProvider::Icims),
            "salesforce" | "sales_force" | "sales-force" => Ok(FrameworkProvider::Salesforce),
            "anaplan" | "anaplan_integration" | "anaplan-integration" => {
                Ok(FrameworkProvider::Anaplan)
            }
            "oracle_financials"
            | "oracle-financials"
            | "oracle_fusion_financials"
            | "oracle-fusion-financials"
            | "oracle_fusion_financials_26b"
            | "oracle-fusion-financials-26b" => Ok(FrameworkProvider::OracleFinancials),
            "ai_search" | "ai-search" | "ai" | "vector_search" | "vector-search" => {
                Ok(FrameworkProvider::AiSearch)
            }
            _ => Err(format!("unknown provider: {provider}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_key_round_trip_for_all_providers() {
        for provider in FrameworkProvider::ALL {
            let parsed = FrameworkProvider::parse_key(provider.key()).expect("provider key");
            assert_eq!(parsed, provider);
        }
    }

    #[test]
    fn parse_unknown_provider_returns_error() {
        let error = FrameworkProvider::parse_key("unknown_vendor").unwrap_err();
        assert_eq!(error, "unknown provider: unknown_vendor");

        let error = FrameworkProvider::parse_key("oracle").unwrap_err();
        assert_eq!(error, "unknown provider: oracle");
    }

    #[test]
    fn parse_fabric_sql_analytics_aliases() {
        assert_eq!(
            FrameworkProvider::parse_key("fabric").expect("fabric alias"),
            FrameworkProvider::FabricSqlAnalytics
        );
        assert_eq!(
            FrameworkProvider::parse_key("fabric-sql-analytics")
                .expect("fabric sql analytics alias"),
            FrameworkProvider::FabricSqlAnalytics
        );
    }

    #[test]
    fn external_api_providers_are_not_database_or_graph_read() {
        assert_eq!(
            FrameworkProvider::EXTERNAL_API,
            [
                FrameworkProvider::ServiceNow,
                FrameworkProvider::Workday,
                FrameworkProvider::Icims,
                FrameworkProvider::Salesforce,
                FrameworkProvider::Anaplan,
                FrameworkProvider::OracleFinancials
            ]
        );

        for provider in FrameworkProvider::EXTERNAL_API {
            assert!(provider.is_external_api_provider());
            assert!(!provider.is_database_provider());
            assert!(!provider.is_graph_read_provider());
            assert!(!provider.is_ai_search_provider());
        }
    }

    #[test]
    fn ai_search_provider_is_its_own_family() {
        assert_eq!(FrameworkProvider::AI_SEARCH, [FrameworkProvider::AiSearch]);
        assert!(FrameworkProvider::AiSearch.is_ai_search_provider());
        assert!(!FrameworkProvider::AiSearch.is_database_provider());
        assert!(!FrameworkProvider::AiSearch.is_graph_read_provider());
        assert!(!FrameworkProvider::AiSearch.is_external_api_provider());
    }

    #[test]
    fn provider_families_are_disjoint_and_cover_all_providers() {
        let mut grouped = Vec::new();
        grouped.extend(FrameworkProvider::DATABASE);
        grouped.extend(FrameworkProvider::GRAPH_READ);
        grouped.extend(FrameworkProvider::EXTERNAL_API);
        grouped.extend(FrameworkProvider::AI_SEARCH);

        assert_eq!(
            grouped.len(),
            FrameworkProvider::ALL.len(),
            "provider family arrays must cover every framework provider exactly once"
        );

        for provider in FrameworkProvider::ALL {
            assert!(
                grouped.contains(&provider),
                "{} is missing from the provider family arrays",
                provider.key()
            );
            assert_eq!(
                grouped
                    .iter()
                    .filter(|candidate| **candidate == provider)
                    .count(),
                1,
                "{} is declared in more than one provider family",
                provider.key()
            );
        }
    }

    #[test]
    fn provider_keys_are_unique_and_stable() {
        let mut keys = std::collections::HashSet::new();
        for provider in FrameworkProvider::ALL {
            assert!(
                keys.insert(provider.key()),
                "duplicate provider key {}",
                provider.key()
            );
            assert_eq!(
                FrameworkProvider::parse_key(provider.key()).expect("provider key round trip"),
                provider
            );
        }
    }

    #[test]
    fn parse_external_api_aliases() {
        assert_eq!(
            FrameworkProvider::parse_key("service-now").expect("servicenow alias"),
            FrameworkProvider::ServiceNow
        );
        assert_eq!(
            FrameworkProvider::parse_key("sales-force").expect("salesforce alias"),
            FrameworkProvider::Salesforce
        );
        assert_eq!(
            FrameworkProvider::parse_key("i-cims").expect("icims alias"),
            FrameworkProvider::Icims
        );
        assert_eq!(
            FrameworkProvider::parse_key("anaplan-integration").expect("anaplan alias"),
            FrameworkProvider::Anaplan
        );
        assert_eq!(
            FrameworkProvider::parse_key("oracle-fusion-financials").expect("oracle alias"),
            FrameworkProvider::OracleFinancials
        );
        assert_eq!(
            FrameworkProvider::parse_key("oracle-fusion-financials-26b").expect("oracle alias"),
            FrameworkProvider::OracleFinancials
        );
        assert_eq!(
            FrameworkProvider::parse_key("vector-search").expect("ai search alias"),
            FrameworkProvider::AiSearch
        );
    }
}
