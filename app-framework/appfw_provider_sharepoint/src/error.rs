#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SharePointProviderError {
    UnknownOperation(String),
    UnsupportedOperation {
        name: String,
        reason: &'static str,
    },
    InvalidParameter {
        name: &'static str,
        reason: &'static str,
    },
    MissingParameter(&'static str),
    UnsupportedMutation(&'static str),
}
