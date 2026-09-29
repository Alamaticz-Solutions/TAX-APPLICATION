use std::sync::Arc;

use serde_json::Value;

use crate::{
    product_api::runtime_data_type,
    routes::app_error::AppError,
    schemas::system::{DataType, PropertyType},
};

pub type SnowflakeStatement = appfw_provider_snowflake::SnowflakeStatement;

#[derive(Clone, Debug)]
pub struct SnowflakeStatementBuilder {
    inner: appfw_provider_snowflake::SnowflakeStatementBuilder,
}

impl SnowflakeStatementBuilder {
    pub fn new() -> Self {
        Self {
            inner: appfw_provider_snowflake::SnowflakeStatementBuilder::new(),
        }
    }

    pub fn bind_prop(&mut self, prop: Arc<PropertyType>, value: Value) -> Result<String, AppError> {
        self.bind_scalar(prop.data_type, &prop.name, prop.is_required, value)
    }

    pub fn bind_scalar(
        &mut self,
        data_type: DataType,
        prop_name: &str,
        is_required: bool,
        value: Value,
    ) -> Result<String, AppError> {
        self.inner
            .bind_scalar(runtime_data_type(data_type), prop_name, is_required, value)
            .map_err(AppError::from)
    }

    pub fn append(&mut self, other: Self) {
        self.inner.append(other.inner);
    }

    pub(crate) fn inner_mut(&mut self) -> &mut appfw_provider_snowflake::SnowflakeStatementBuilder {
        &mut self.inner
    }

    pub fn finish(self, sql: impl Into<String>) -> SnowflakeStatement {
        self.inner.finish(sql)
    }
}

impl Default for SnowflakeStatementBuilder {
    fn default() -> Self {
        Self::new()
    }
}
