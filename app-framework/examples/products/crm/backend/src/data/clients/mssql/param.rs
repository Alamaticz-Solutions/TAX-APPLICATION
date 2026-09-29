use serde_json::Value;
use std::sync::Arc;

use crate::{
    product_api::runtime_data_type,
    routes::app_error::AppError,
    schemas::system::{DataType, PropertyType},
};

pub type SqlParam = appfw_provider_mssql::SqlParam;

pub fn param_placeholder(ndx: usize) -> String {
    appfw_provider_mssql::param_placeholder(ndx)
}

pub fn prop_param(prop: Arc<PropertyType>, input_value: Value) -> Result<SqlParam, AppError> {
    type_param(prop.data_type, !prop.is_required, &prop.name, input_value)
}

pub fn type_param(
    data_type: DataType,
    is_nullable: bool,
    prop_name: &str,
    input_value: Value,
) -> Result<SqlParam, AppError> {
    appfw_provider_mssql::type_param(
        runtime_data_type(data_type),
        is_nullable,
        prop_name,
        input_value,
    )
    .map_err(AppError::from)
}
