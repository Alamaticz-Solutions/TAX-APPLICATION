use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnaplanProviderError {
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
    InvalidResponse(String),
    UnsupportedMutation(&'static str),
}

impl fmt::Display for AnaplanProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownOperation(name) => write!(f, "unknown Anaplan operation: {name}"),
            Self::UnsupportedOperation { name, reason } => {
                write!(f, "Anaplan operation {name} is not executable: {reason}")
            }
            Self::MissingParameter(name) => {
                write!(f, "missing required Anaplan operation parameter: {name}")
            }
            Self::InvalidParameter { name, reason } => {
                write!(f, "invalid Anaplan operation parameter {name}: {reason}")
            }
            Self::InvalidSaasRequestPlan(reason) => {
                write!(f, "invalid Anaplan SaaS request plan: {reason}")
            }
            Self::InvalidResponse(reason) => write!(f, "invalid Anaplan response: {reason}"),
            Self::UnsupportedMutation(reason) => {
                write!(f, "Anaplan mutation is not supported: {reason}")
            }
        }
    }
}

impl std::error::Error for AnaplanProviderError {}
