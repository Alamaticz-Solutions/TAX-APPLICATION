//! Rate-limit / error classification, redacted payloads, result caps.
//!
//! `parse_retry_after`, `redact`, and their helpers below are product-owned.

use std::sync::LazyLock;
use std::time::Duration;

use regex::Regex;

/// Result payloads are capped so a hostile / huge response cannot pin memory.
pub const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug)]
pub enum GraphError {
    /// 401/403 - token / permission problem. Not retryable.
    Unauthorized { code: String },
    /// 404 - the named resource does not exist for this tenant.
    NotFound,
    /// 429 / 503 - throttled. `retry_after` is honored by the caller.
    RateLimited { retry_after: Option<Duration> },
    /// 5xx other than 503.
    Upstream { status: u16, code: String },
    /// 4xx other than 401/403/404/429.
    BadRequest { status: u16, code: String },
    /// transport / decode failure (message is pre-redacted). Matched
    /// exhaustively below but not yet constructed -- `client.rs`/`writes.rs`
    /// currently map a transport failure to `Unauthorized { code: "transport" }`.
    #[allow(dead_code)]
    Transport { message: String },
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unauthorized { code } => write!(f, "graph unauthorized ({code})"),
            Self::NotFound => write!(f, "graph resource not found"),
            Self::RateLimited { retry_after } => write!(
                f,
                "graph rate limited (retry after {}s)",
                retry_after.map(|d| d.as_secs()).unwrap_or(0)
            ),
            Self::Upstream { status, code } => write!(f, "graph upstream {status} ({code})"),
            Self::BadRequest { status, code } => write!(f, "graph bad request {status} ({code})"),
            Self::Transport { message } => write!(f, "graph transport error: {message}"),
        }
    }
}
impl std::error::Error for GraphError {}

fn error_code(body: &serde_json::Value) -> String {
    body.get("error")
        .and_then(|e| {
            e.get("code").and_then(|c| c.as_str()).or_else(|| {
                e.get("innerError")
                    .and_then(|i| i.get("code"))
                    .and_then(|c| c.as_str())
            })
        })
        .unwrap_or("unknown")
        .to_string()
}

pub fn classify(status: reqwest::StatusCode, body: &serde_json::Value) -> GraphError {
    let s = status.as_u16();
    match s {
        401 | 403 => GraphError::Unauthorized {
            code: error_code(body),
        },
        404 => GraphError::NotFound,
        429 | 503 => GraphError::RateLimited { retry_after: None },
        400 | 402 | 405..=428 | 430..=499 => GraphError::BadRequest {
            status: s,
            code: error_code(body),
        },
        500..=599 => GraphError::Upstream {
            status: s,
            code: error_code(body),
        },
        _ => GraphError::BadRequest {
            status: s,
            code: error_code(body),
        },
    }
}

/// Parses an HTTP `Retry-After` header value, which per RFC 9110 -10.2.3 is
/// either a delta-seconds integer or an HTTP-date. Returns the delay as a
/// `Duration`, floored at zero for a date already in the past.
pub fn retry_after_from(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_retry_after)
}

fn parse_retry_after(value: &str) -> Option<Duration> {
    let value = value.trim();
    if let Ok(seconds) = value.parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }
    // HTTP-date form (IMF-fixdate, e.g. "Sun, 06 Nov 1994 08:49:37 GMT").
    let retry_at = chrono::DateTime::parse_from_rfc2822(value).ok()?;
    let now = chrono::Utc::now();
    let delta = retry_at.with_timezone(&chrono::Utc) - now;
    Some(Duration::from_secs(delta.num_seconds().max(0) as u64))
}

/// Names that must never be echoed back once matched as a JSON object key
/// (case/punctuation-insensitive: `client_secret`, `clientSecret`, and
/// `Client-Secret` all match `secret`).
const SENSITIVE_KEY_FRAGMENTS: &[&str] = &[
    "password",
    "pwd",
    "token",
    "secret",
    "credential",
    "authorization",
    "privatekey",
    "accesskey",
    "accountkey",
    "apikey",
];

fn is_sensitive_key(key: &str) -> bool {
    let normalized: String = key
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();
    SENSITIVE_KEY_FRAGMENTS
        .iter()
        .any(|fragment| normalized.contains(fragment))
}

/// Matches an embedded `Authorization: <scheme> <token>` or bare
/// `Bearer <token>` segment inside free text (e.g. an upstream error message
/// that echoed a request header), so it can be blanked out even when it
/// isn't sitting under a JSON key named "authorization".
// `.+` (rather than `\S+`) redacts through the rest of the line, not just
// the first whitespace-delimited token -- a credential like "Bearer abc.def"
// is two tokens, and `\S+` alone would only blank "Bearer", leaving the
// actual token exposed. `.` does not match `\n` by default, so this still
// stops at a line break rather than eating an entire multi-line payload.
static EMBEDDED_CREDENTIAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(authorization\s*:\s*|\bbearer\s+).+").expect("static regex is valid")
});

fn redact_embedded_credentials(text: &str) -> String {
    EMBEDDED_CREDENTIAL
        .replace_all(text, "$1[REDACTED]")
        .into_owned()
}

/// Redact a JSON value recursively: any object value whose key looks
/// sensitive (`is_sensitive_key`) is replaced outright; every remaining
/// string is additionally scanned for embedded credentials
/// (`redact_embedded_credentials`), since a token can show up inside a
/// message string under an innocuous key too.
fn redact_json_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::String(text) => {
            serde_json::Value::String(redact_embedded_credentials(&text))
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(redact_json_value).collect())
        }
        serde_json::Value::Object(entries) => serde_json::Value::Object(
            entries
                .into_iter()
                .map(|(key, val)| {
                    let val = if is_sensitive_key(&key) {
                        serde_json::Value::String("[REDACTED]".to_string())
                    } else {
                        redact_json_value(val)
                    };
                    (key, val)
                })
                .collect(),
        ),
        other => other,
    }
}

/// Redact a Graph payload before it is logged or returned to a non-Graph
/// caller: first key-level + embedded-credential redaction, then strip the
/// Graph-notification-specific fields (`clientState`, `resourceData`,
/// `@odata.context`) that must never propagate.
pub fn redact(value: serde_json::Value) -> serde_json::Value {
    let mut value = redact_json_value(value);
    fn scrub(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(map) => {
                for k in ["clientState", "@odata.context", "resourceData"] {
                    map.remove(k);
                }
                for child in map.values_mut() {
                    scrub(child);
                }
            }
            serde_json::Value::Array(items) => items.iter_mut().for_each(scrub),
            _ => {}
        }
    }
    scrub(&mut value);
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn retry_after_parses_delta_seconds() {
        assert_eq!(parse_retry_after("120"), Some(Duration::from_secs(120)));
        assert_eq!(parse_retry_after("  7 "), Some(Duration::from_secs(7)));
    }

    #[test]
    fn retry_after_parses_http_date_in_the_future() {
        let future = chrono::Utc::now() + chrono::Duration::seconds(30);
        let header = future.to_rfc2822();
        let parsed = parse_retry_after(&header).expect("valid IMF-fixdate");
        // Allow a couple seconds of slack for wall-clock time passing during the test.
        assert!((28..=31).contains(&parsed.as_secs()), "got {parsed:?}");
    }

    #[test]
    fn retry_after_floors_past_dates_at_zero() {
        let past = chrono::Utc::now() - chrono::Duration::seconds(30);
        assert_eq!(parse_retry_after(&past.to_rfc2822()), Some(Duration::ZERO));
    }

    #[test]
    fn retry_after_rejects_garbage() {
        assert_eq!(parse_retry_after("not-a-duration"), None);
    }

    #[test]
    fn sensitive_keys_match_common_casings_and_separators() {
        assert!(is_sensitive_key("api_key"));
        assert!(is_sensitive_key("clientSecret"));
        assert!(is_sensitive_key("Client-Secret"));
        assert!(is_sensitive_key("Authorization"));
        assert!(!is_sensitive_key("display_name"));
    }

    #[test]
    fn json_redaction_recurses_through_sensitive_keys_and_embedded_credentials() {
        let value = json!({
            "access_token": "secret-token",
            "nested": { "message": "failed Authorization: Bearer abc.def" },
            "items": [{ "client_secret": "shh" }, "plain text"]
        });
        let redacted = redact_json_value(value);
        assert_eq!(redacted["access_token"], "[REDACTED]");
        assert_eq!(redacted["items"][0]["client_secret"], "[REDACTED]");
        assert_eq!(redacted["items"][1], "plain text");
        assert_eq!(
            redacted["nested"]["message"],
            "failed Authorization: [REDACTED]"
        );
    }

    #[test]
    fn redact_strips_graph_notification_fields_after_key_redaction() {
        let value = json!({
            "clientState": "should-be-removed",
            "resourceData": { "id": "1" },
            "@odata.context": "should-be-removed",
            "value": [{ "clientState": "nested-too" }]
        });
        let redacted = redact(value);
        assert!(redacted.get("clientState").is_none());
        assert!(redacted.get("resourceData").is_none());
        assert!(redacted.get("@odata.context").is_none());
        assert!(redacted["value"][0].get("clientState").is_none());
    }
}
