use uuid::Uuid;

use crate::RuntimeError;

pub const RECORD_LOCATOR_FIELD: &str = "record_locator";
pub const RECORD_LOCATOR_PREFIX: &str = "rl_";

const RECORD_LOCATOR_HEX_LEN: usize = 32;

pub fn new_record_locator() -> String {
    format!("{RECORD_LOCATOR_PREFIX}{}", Uuid::new_v4().simple())
}

pub fn is_record_locator(value: &str) -> bool {
    validate_record_locator(value).is_ok()
}

pub fn validate_record_locator(value: &str) -> Result<(), RuntimeError> {
    let Some(token) = value.strip_prefix(RECORD_LOCATOR_PREFIX) else {
        return Err(invalid_record_locator());
    };
    if token.len() != RECORD_LOCATOR_HEX_LEN || !token.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(invalid_record_locator());
    }
    Ok(())
}

fn invalid_record_locator() -> RuntimeError {
    RuntimeError::Validation("invalid record locator".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_locator_is_random_public_identifier() {
        let first = new_record_locator();
        let second = new_record_locator();

        assert!(first.starts_with(RECORD_LOCATOR_PREFIX));
        assert!(second.starts_with(RECORD_LOCATOR_PREFIX));
        assert_ne!(first, second);
        assert!(validate_record_locator(&first).is_ok());
        assert!(validate_record_locator(&second).is_ok());
    }

    #[test]
    fn record_locator_rejects_invalid_shapes() {
        assert!(validate_record_locator("account-1").is_err());
        assert!(validate_record_locator("rl_not-hex").is_err());
        assert!(validate_record_locator("rl_0123456789abcdef").is_err());
        assert!(validate_record_locator("rl_0123456789abcdef0123456789abcdeg").is_err());
    }

    #[test]
    fn is_record_locator_requires_complete_shape() {
        assert!(is_record_locator("rl_0123456789abcdef0123456789ABCDEF"));
        assert!(!is_record_locator("rl_not-hex"));
        assert!(!is_record_locator("account-1"));
    }
}
