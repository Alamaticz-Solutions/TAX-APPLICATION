use std::{env, error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretError {
    Missing { name: String },
    Read { name: String, message: String },
}

impl fmt::Display for SecretError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecretError::Missing { name } => {
                write!(f, "required secret `{name}` is missing or empty")
            }
            SecretError::Read { name, message } => {
                write!(f, "could not read secret `{name}`: {message}")
            }
        }
    }
}

impl Error for SecretError {}

pub trait SecretProvider {
    fn get_secret(&self, name: &str) -> Result<Option<String>, SecretError>;

    fn require_secret(&self, name: &str) -> Result<String, SecretError> {
        self.get_secret(name)?.ok_or_else(|| SecretError::Missing {
            name: name.to_string(),
        })
    }
}

#[derive(Clone, Default)]
pub struct EnvSecretProvider;

impl SecretProvider for EnvSecretProvider {
    fn get_secret(&self, name: &str) -> Result<Option<String>, SecretError> {
        match env::var(name) {
            Ok(value) => Ok(non_empty(value)),
            Err(env::VarError::NotPresent) => Ok(None),
            Err(err) => Err(SecretError::Read {
                name: name.to_string(),
                message: err.to_string(),
            }),
        }
    }
}

fn non_empty(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StaticProvider(Option<String>);

    impl SecretProvider for StaticProvider {
        fn get_secret(&self, _name: &str) -> Result<Option<String>, SecretError> {
            Ok(self.0.clone().and_then(non_empty))
        }
    }

    #[test]
    fn require_secret_returns_value_when_present() {
        let provider = StaticProvider(Some("secret".to_string()));

        assert_eq!(provider.require_secret("TOKEN").unwrap(), "secret");
    }

    #[test]
    fn require_secret_rejects_empty_values() {
        let provider = StaticProvider(Some("   ".to_string()));

        assert_eq!(
            provider.require_secret("TOKEN").unwrap_err(),
            SecretError::Missing {
                name: "TOKEN".to_string()
            }
        );
    }
}
