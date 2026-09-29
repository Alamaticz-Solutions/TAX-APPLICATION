#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AiSearchProviderError {
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
