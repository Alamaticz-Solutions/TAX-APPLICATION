use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManyToManyConfig {
    pub max_related_entities: i32,
    pub enable_pagination: bool,
    pub json_size_warning_kb: i32,
    pub enable_performance_monitoring: bool,
    pub enable_query_plan_logging: bool,
    pub slow_query_threshold_ms: u64,
    pub enable_index_suggestions: bool,
    pub max_relationships_per_entity: i32,
}

impl Default for ManyToManyConfig {
    fn default() -> Self {
        Self {
            max_related_entities: 100,
            enable_pagination: true,
            json_size_warning_kb: 1024,
            enable_performance_monitoring: true,
            enable_query_plan_logging: false,
            slow_query_threshold_ms: 1000,
            enable_index_suggestions: true,
            max_relationships_per_entity: 5,
        }
    }
}

impl ManyToManyConfig {
    pub fn high_performance() -> Self {
        Self {
            max_related_entities: 50,
            enable_pagination: true,
            json_size_warning_kb: 512,
            enable_performance_monitoring: true,
            enable_query_plan_logging: true,
            slow_query_threshold_ms: 500,
            enable_index_suggestions: true,
            max_relationships_per_entity: 3,
        }
    }

    pub fn development() -> Self {
        Self {
            max_related_entities: 20,
            enable_pagination: true,
            json_size_warning_kb: 256,
            enable_performance_monitoring: true,
            enable_query_plan_logging: true,
            slow_query_threshold_ms: 100,
            enable_index_suggestions: true,
            max_relationships_per_entity: 10,
        }
    }

    pub fn unrestricted() -> Self {
        Self {
            max_related_entities: -1,
            enable_pagination: false,
            json_size_warning_kb: -1,
            enable_performance_monitoring: false,
            enable_query_plan_logging: false,
            slow_query_threshold_ms: u64::MAX,
            enable_index_suggestions: false,
            max_relationships_per_entity: -1,
        }
    }

    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        if self.max_related_entities > 500 {
            warnings.push(format!(
                "max_related_entities ({}) is very high, consider pagination",
                self.max_related_entities
            ));
        }

        if self.json_size_warning_kb > 5120 {
            warnings.push(format!(
                "json_size_warning_kb ({}) is very high, large JSON responses may impact performance",
                self.json_size_warning_kb
            ));
        }

        if !self.enable_performance_monitoring && self.enable_query_plan_logging {
            warnings.push(
                "Query plan logging is enabled but performance monitoring is disabled".to_string(),
            );
        }

        if self.max_relationships_per_entity > 10 {
            warnings.push(format!(
                "max_relationships_per_entity ({}) is high, consider entity design review",
                self.max_relationships_per_entity
            ));
        }

        warnings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_keeps_conservative_limits() {
        let config = ManyToManyConfig::default();

        assert_eq!(config.max_related_entities, 100);
        assert!(config.enable_pagination);
        assert!(config.enable_performance_monitoring);
    }

    #[test]
    fn high_performance_profile_tightens_limits_and_logs_plans() {
        let config = ManyToManyConfig::high_performance();

        assert_eq!(config.max_related_entities, 50);
        assert_eq!(config.slow_query_threshold_ms, 500);
        assert!(config.enable_query_plan_logging);
    }

    #[test]
    fn validation_warns_for_expensive_settings() {
        let config = ManyToManyConfig {
            max_related_entities: 1000,
            json_size_warning_kb: 10240,
            ..Default::default()
        };

        let warnings = config.validate();

        assert!(warnings.len() >= 2);
        assert!(warnings.iter().any(|w| w.contains("max_related_entities")));
        assert!(warnings.iter().any(|w| w.contains("json_size_warning_kb")));
    }
}
