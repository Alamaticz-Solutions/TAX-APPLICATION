use std::env;

pub const APPFW_MIGRATE_SKIP_SEED: &str = "APPFW_MIGRATE_SKIP_SEED";

pub fn skip_seed() -> bool {
    env::var(APPFW_MIGRATE_SKIP_SEED)
        .ok()
        .map(|value| matches_truthy(value.trim()))
        .unwrap_or(false)
}

fn matches_truthy(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "y" | "on"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truthy_values_enable_seed_skip() {
        for value in ["1", "true", "TRUE", "yes", "y", "on"] {
            assert!(matches_truthy(value));
        }
    }

    #[test]
    fn other_values_do_not_enable_seed_skip() {
        for value in ["", "0", "false", "no", "off", "skip"] {
            assert!(!matches_truthy(value));
        }
    }
}
