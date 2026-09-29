use anyhow::{Context, Result};
use serde_json::{Map, Value};

#[derive(Debug, Default)]
pub struct ResultStore {
    state: Map<String, Value>,
}

impl ResultStore {
    pub fn set(&mut self, name: &str, data: Value) {
        self.state.insert(name.to_string(), data);
    }

    pub fn value(&self, reference: &str) -> Result<Value> {
        let mut segments = reference.split('.');
        let root = segments
            .next()
            .filter(|segment| !segment.is_empty())
            .with_context(|| "result reference cannot be empty")?;

        let mut current = self
            .state
            .get(root)
            .with_context(|| format!("result `{root}` was not found"))?;

        for segment in segments {
            current = current.get(segment).with_context(|| {
                format!("result reference `{reference}` is missing segment `{segment}`")
            })?;
        }

        Ok(current.clone())
    }

    pub fn map(
        &self,
        reference: &str,
        overwrite: Option<Map<String, Value>>,
    ) -> Result<Map<String, Value>> {
        let value = self.value(reference)?;
        let mut result = value
            .as_object()
            .cloned()
            .with_context(|| format!("result reference `{reference}` is not an object"))?;

        if let Some(overwrite_values) = overwrite {
            for (key, value) in overwrite_values {
                result.insert(key, value);
            }
        }

        Ok(result)
    }
}
