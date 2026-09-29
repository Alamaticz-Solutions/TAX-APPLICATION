use std::collections::BTreeMap;

use serde_json::{Map, Value};

pub const REDACTED: &str = "[REDACTED]";

const SENSITIVE_HEADER_NAMES: &[&str] = &[
    "authorization",
    "proxy-authorization",
    "cookie",
    "set-cookie",
    "x-api-key",
    "api-key",
    "x-auth-token",
    "x-access-token",
];

pub fn is_sensitive_header_name(name: &str) -> bool {
    let normalized = name.trim().to_ascii_lowercase();
    SENSITIVE_HEADER_NAMES.contains(&normalized.as_str())
        || normalized.contains("token")
        || normalized.contains("secret")
        || normalized.contains("credential")
}

pub fn redact_header_value(name: &str, value: &str) -> String {
    if is_sensitive_header_name(name) {
        REDACTED.to_string()
    } else {
        redact_text(value)
    }
}

pub fn redact_headers<I, K, V>(headers: I) -> BTreeMap<String, String>
where
    I: IntoIterator<Item = (K, V)>,
    K: Into<String>,
    V: Into<String>,
{
    headers
        .into_iter()
        .map(|(name, value)| {
            let name = name.into();
            let value = value.into();
            let redacted = redact_header_value(&name, &value);
            (name, redacted)
        })
        .collect()
}

pub fn redact_json_value(value: Value) -> Value {
    match value {
        Value::String(text) => Value::String(redact_text(&text)),
        Value::Array(items) => Value::Array(items.into_iter().map(redact_json_value).collect()),
        Value::Object(entries) => Value::Object(redact_json_object(entries)),
        other => other,
    }
}

fn redact_json_object(entries: Map<String, Value>) -> Map<String, Value> {
    entries
        .into_iter()
        .map(|(key, value)| {
            let value = if is_sensitive_json_key(&key) {
                Value::String(REDACTED.to_string())
            } else {
                redact_json_value(value)
            };
            (key, value)
        })
        .collect()
}

pub fn is_sensitive_json_key(key: &str) -> bool {
    let normalized: String = key
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();

    normalized.contains("password")
        || normalized == "pwd"
        || normalized.contains("passwd")
        || normalized.contains("token")
        || normalized.contains("secret")
        || normalized.contains("credential")
        || normalized.contains("authorization")
        || normalized.contains("privatekey")
        || normalized.contains("accesskey")
        || normalized.contains("accountkey")
        || normalized.contains("apikey")
}

pub fn redact_text(value: &str) -> String {
    let mut redacted = redact_authorization_segments(value);
    redacted = redact_after_case_insensitive_prefix(&redacted, "bearer ");
    redacted = redact_key_value_tokens(&redacted);
    redacted
}

fn redact_authorization_segments(value: &str) -> String {
    let lower = value.to_ascii_lowercase();
    let prefix = "authorization:";
    let mut output = String::with_capacity(value.len());
    let mut cursor = 0;

    while let Some(relative_start) = lower[cursor..].find(prefix) {
        let start = cursor + relative_start;
        let mut secret_start = start + prefix.len();
        while value[secret_start..].starts_with(char::is_whitespace) {
            secret_start += value[secret_start..]
                .chars()
                .next()
                .expect("checked starts_with")
                .len_utf8();
        }
        let secret_end = value[secret_start..]
            .find('\n')
            .map(|offset| secret_start + offset)
            .unwrap_or(value.len());

        output.push_str(&value[cursor..start + prefix.len()]);
        output.push(' ');
        output.push_str(REDACTED);
        cursor = secret_end;
    }

    output.push_str(&value[cursor..]);
    output
}

fn redact_after_case_insensitive_prefix(value: &str, prefix: &str) -> String {
    let lower = value.to_ascii_lowercase();
    let prefix = prefix.to_ascii_lowercase();
    let mut output = String::with_capacity(value.len());
    let mut cursor = 0;

    while let Some(relative_start) = lower[cursor..].find(&prefix) {
        let start = cursor + relative_start;
        let secret_start = start + prefix.len();
        let secret_end = value[secret_start..]
            .find(char::is_whitespace)
            .map(|offset| secret_start + offset)
            .unwrap_or(value.len());

        output.push_str(&value[cursor..secret_start]);
        output.push_str(REDACTED);
        cursor = secret_end;
    }

    output.push_str(&value[cursor..]);
    output
}

fn redact_key_value_tokens(value: &str) -> String {
    value
        .split_whitespace()
        .map(|token| {
            let Some(separator) = token.find('=') else {
                return token.to_string();
            };

            let (key, suffix) = token.split_at(separator);
            if is_sensitive_json_key(key) {
                let trailing = suffix
                    .chars()
                    .rev()
                    .take_while(|character| {
                        matches!(character, ',' | ';' | ')' | ']' | '}' | '"' | '\'')
                    })
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect::<String>();
                format!("{key}={REDACTED}{trailing}")
            } else {
                token.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn headers_redact_known_sensitive_names() {
        let headers = redact_headers([
            ("Authorization", "Bearer raw-token"),
            ("x-api-key", "raw-api-key"),
            ("content-type", "application/json"),
        ]);

        assert_eq!(headers["Authorization"], REDACTED);
        assert_eq!(headers["x-api-key"], REDACTED);
        assert_eq!(headers["content-type"], "application/json");
    }

    #[test]
    fn json_redaction_recurses_through_sensitive_keys_and_text() {
        let value = json!({
            "access_token": "secret-token",
            "nested": {
                "message": "failed Authorization: Bearer abc.def"
            },
            "items": [
                { "client_secret": "client-secret" },
                "password=hunter2"
            ]
        });

        let redacted = redact_json_value(value);

        assert_eq!(redacted["access_token"], REDACTED);
        assert_eq!(redacted["items"][0]["client_secret"], REDACTED);
        assert_eq!(redacted["items"][1], format!("password={REDACTED}"));
        assert_eq!(
            redacted["nested"]["message"],
            format!("failed Authorization: {REDACTED}")
        );
    }

    #[test]
    fn sensitive_json_key_normalization_handles_common_variants() {
        assert!(is_sensitive_json_key("api_key"));
        assert!(is_sensitive_json_key("private-key"));
        assert!(is_sensitive_json_key("clientSecret"));
        assert!(!is_sensitive_json_key("display_name"));
    }
}
