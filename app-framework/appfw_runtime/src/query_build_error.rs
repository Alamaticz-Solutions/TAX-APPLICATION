use thiserror::Error;

#[derive(Error, Debug)]
pub enum QueryBuildError {
    #[error("invalid time period: {0}")]
    InvalidTimePeriod(String),
    #[error("time period '{period}' is unsupported for {data_type}")]
    UnsupportedTimePeriod { period: String, data_type: String },
    #[error("invalid date bound: {0}")]
    InvalidDateBound(String),
    #[error("failed to serialize query value: {0}")]
    SerializeValue(String),
}
