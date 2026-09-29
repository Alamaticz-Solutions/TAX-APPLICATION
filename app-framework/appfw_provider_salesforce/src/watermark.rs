use crate::SalesforceField;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SalesforceWatermarkValue {
    DateTime(String),
}

impl SalesforceWatermarkValue {
    pub fn as_str(&self) -> &str {
        match self {
            Self::DateTime(value) => value.as_str(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SalesforceWatermarkRequest {
    pub field: SalesforceField,
    pub value: SalesforceWatermarkValue,
    pub inclusive: bool,
    pub max_results: u32,
    pub cursor: Option<String>,
}

impl SalesforceWatermarkRequest {
    pub fn exclusive_after(
        field: SalesforceField,
        value: impl Into<String>,
        max_results: u32,
    ) -> Self {
        Self {
            field,
            value: SalesforceWatermarkValue::DateTime(value.into()),
            inclusive: false,
            max_results,
            cursor: None,
        }
    }

    pub fn with_cursor(mut self, cursor: impl Into<String>) -> Self {
        self.cursor = Some(cursor.into());
        self
    }
}
