use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SalesforceProviderError {
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
    InvalidSoql(String),
    InvalidSaasRequestPlan(String),
    UnsupportedMutation(&'static str),
}

impl fmt::Display for SalesforceProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownOperation(name) => write!(f, "unknown Salesforce operation: {name}"),
            Self::UnsupportedOperation { name, reason } => {
                write!(f, "Salesforce operation {name} is not executable: {reason}")
            }
            Self::MissingParameter(name) => {
                write!(f, "missing required Salesforce operation parameter: {name}")
            }
            Self::InvalidParameter { name, reason } => {
                write!(f, "invalid Salesforce operation parameter {name}: {reason}")
            }
            Self::InvalidSoql(reason) => write!(f, "invalid Salesforce SOQL fragment: {reason}"),
            Self::InvalidSaasRequestPlan(reason) => {
                write!(f, "invalid Salesforce SaaS request plan: {reason}")
            }
            Self::UnsupportedMutation(reason) => {
                write!(f, "Salesforce mutation is not supported: {reason}")
            }
        }
    }
}

impl std::error::Error for SalesforceProviderError {}
