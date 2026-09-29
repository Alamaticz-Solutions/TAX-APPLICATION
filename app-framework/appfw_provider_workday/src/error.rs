use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkdayProviderError {
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
    InvalidTemplate(String),
    InvalidResponse(String),
    UnsupportedMutation(&'static str),
}

impl fmt::Display for WorkdayProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownOperation(name) => write!(f, "unknown Workday operation: {name}"),
            Self::UnsupportedOperation { name, reason } => {
                write!(f, "Workday operation {name} is not executable: {reason}")
            }
            Self::MissingParameter(name) => {
                write!(f, "missing required Workday operation parameter: {name}")
            }
            Self::InvalidParameter { name, reason } => {
                write!(f, "invalid Workday operation parameter {name}: {reason}")
            }
            Self::InvalidSaasRequestPlan(reason) => {
                write!(f, "invalid shared SaaS request plan: {reason}")
            }
            Self::InvalidTemplate(reason) => {
                write!(f, "invalid Workday SOAP template: {reason}")
            }
            Self::InvalidResponse(reason) => write!(f, "invalid Workday response: {reason}"),
            Self::UnsupportedMutation(reason) => {
                write!(f, "Workday mutation is not supported: {reason}")
            }
        }
    }
}

impl std::error::Error for WorkdayProviderError {}
