#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OracleFinancialsProviderError {
    UnknownOperation(String),
    UnsupportedOperation {
        name: String,
        reason: &'static str,
    },
    MissingParameter(&'static str),
    InvalidParameter {
        name: &'static str,
        reason: &'static str,
    },
    InvalidSaasRequestPlan(String),
    InvalidResponse(&'static str),
    UnsupportedMutation(&'static str),
}
