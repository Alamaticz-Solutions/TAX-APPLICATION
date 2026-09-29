use crate::query::Neo4jGraphQueryLimits;

/// Provider-level guardrail ceilings.
///
/// A named operation may request *tighter* limits than these ceilings, but never
/// looser ones. Effective limits are the per-operation request clamped down to
/// the ceiling, so a misconfigured operation can never raise the blast radius of
/// a graph read above the data-source policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Neo4jProviderLimits {
    pub max_depth: u16,
    pub max_results: u32,
    pub max_timeout_ms: u64,
}

impl Default for Neo4jProviderLimits {
    fn default() -> Self {
        Self {
            max_depth: 5,
            max_results: 1_000,
            max_timeout_ms: 5_000,
        }
    }
}

/// The limits actually applied to one execution, plus whether any per-operation
/// request had to be clamped down to a ceiling (surfaced in audit/metadata).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Neo4jEffectiveLimits {
    pub max_depth: u16,
    pub max_results: u32,
    pub timeout_ms: u64,
    pub depth_clamped: bool,
    pub results_clamped: bool,
    pub timeout_clamped: bool,
}

impl Neo4jProviderLimits {
    pub fn clamp(&self, requested: &Neo4jGraphQueryLimits) -> Neo4jEffectiveLimits {
        let max_depth = requested.max_depth.min(self.max_depth);
        // A timeout of zero would make every query fail instantly; floor it at 1ms.
        let timeout_ms = requested.timeout_ms.min(self.max_timeout_ms).max(1);
        let max_results = requested.max_results.min(self.max_results).max(1);
        Neo4jEffectiveLimits {
            max_depth,
            max_results,
            timeout_ms,
            depth_clamped: requested.max_depth > self.max_depth,
            results_clamped: requested.max_results > self.max_results,
            timeout_clamped: requested.timeout_ms > self.max_timeout_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn requested(max_depth: u16, max_results: u32, timeout_ms: u64) -> Neo4jGraphQueryLimits {
        Neo4jGraphQueryLimits {
            max_depth,
            max_results,
            timeout_ms,
        }
    }

    #[test]
    fn requests_within_ceilings_pass_through() {
        let ceilings = Neo4jProviderLimits::default();
        let effective = ceilings.clamp(&requested(2, 100, 1_000));
        assert_eq!(effective.max_depth, 2);
        assert_eq!(effective.max_results, 100);
        assert_eq!(effective.timeout_ms, 1_000);
        assert!(!effective.depth_clamped);
        assert!(!effective.results_clamped);
        assert!(!effective.timeout_clamped);
    }

    #[test]
    fn requests_above_ceilings_are_clamped_down() {
        let ceilings = Neo4jProviderLimits {
            max_depth: 3,
            max_results: 250,
            max_timeout_ms: 2_000,
        };
        let effective = ceilings.clamp(&requested(10, 10_000, 60_000));
        assert_eq!(effective.max_depth, 3);
        assert_eq!(effective.max_results, 250);
        assert_eq!(effective.timeout_ms, 2_000);
        assert!(effective.depth_clamped);
        assert!(effective.results_clamped);
        assert!(effective.timeout_clamped);
    }

    #[test]
    fn zero_floors_are_raised_to_one() {
        let ceilings = Neo4jProviderLimits::default();
        let effective = ceilings.clamp(&requested(0, 0, 0));
        assert_eq!(effective.max_results, 1);
        assert_eq!(effective.timeout_ms, 1);
    }
}
