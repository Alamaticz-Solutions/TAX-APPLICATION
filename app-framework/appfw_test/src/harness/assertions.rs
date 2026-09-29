use std::collections::BTreeMap;

use serde_json::{Map, Value};

pub type AssertionDiffs = BTreeMap<String, String>;

pub fn partial_eq(
    actual_values: &Map<String, Value>,
    expected_values: &Map<String, Value>,
) -> AssertionDiffs {
    let mut diffs = BTreeMap::new();

    for (key, expected_value) in expected_values {
        match actual_values.get(key) {
            Some(actual_value) if actual_value == expected_value => {}
            Some(actual_value) => {
                diffs.insert(
                    key.clone(),
                    format!("actual: {actual_value:?}, expected: {expected_value:?}"),
                );
            }
            None => {
                diffs.insert(
                    key.clone(),
                    format!("actual: (missing), expected: {expected_value:?}"),
                );
            }
        }
    }

    diffs
}
