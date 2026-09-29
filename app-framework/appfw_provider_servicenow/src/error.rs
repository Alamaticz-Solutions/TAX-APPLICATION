#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServiceNowProviderError {
    UnknownOperation(String),
    UnsupportedOperation {
        name: String,
        reason: &'static str,
    },
    InvalidParameter {
        name: &'static str,
        reason: &'static str,
    },
    UnsupportedMutation(&'static str),
}
