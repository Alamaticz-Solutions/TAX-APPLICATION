//! The one place that speaks HTTP to Box. A small trait so the store can be tested without a
//! network, and so tokens never travel anywhere except the `Authorization` header.

use std::sync::Arc;

use async_trait::async_trait;

use super::token_manager::TokenProvider;
use crate::services::document_store::StoreError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoxMethod {
    Get,
    Post,
    Delete,
}

#[derive(Clone, Debug)]
pub enum BoxBody {
    None,
    Json(serde_json::Value),
    /// Box's multipart upload: an `attributes` JSON part, then the file part.
    Upload { attributes: serde_json::Value, file_name: String, bytes: Vec<u8> },
}

#[derive(Clone, Debug)]
pub struct BoxRequest {
    pub method: BoxMethod,
    pub url: String,
    pub query: Vec<(String, String)>,
    pub body: BoxBody,
}

#[derive(Clone, Debug)]
pub struct BoxResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

#[async_trait]
pub trait BoxHttp: Send + Sync {
    async fn send(&self, request: BoxRequest) -> Result<BoxResponse, StoreError>;
}

/// How many Box requests may be in flight at once, however many requests are being filed
/// (`BOX_MAX_CONCURRENCY`, default 8). Box rate limits a service account; a hundred filings that
/// all call it at once would be refused, so they queue here instead.
const DEFAULT_MAX_IN_FLIGHT: usize = 8;
/// A request that gets no answer within this long is given up on (and retried).
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);
/// Tries per request when Box says "slow down" (429) or has a passing server problem.
const MAX_ATTEMPTS: u32 = 4;

fn max_in_flight() -> usize {
    std::env::var("BOX_MAX_CONCURRENCY").ok().and_then(|v| v.trim().parse::<usize>().ok()).filter(|n| *n > 0).unwrap_or(DEFAULT_MAX_IN_FLIGHT)
}

/// How long to wait before try number `attempt` (0-based): Box's `Retry-After` when it gives one,
/// otherwise 0.5 s, 1 s, 2 s, ... capped at 20 s.
fn backoff(attempt: u32, retry_after: Option<u64>) -> std::time::Duration {
    match retry_after {
        Some(secs) => std::time::Duration::from_secs(secs.min(20)),
        None => std::time::Duration::from_millis((500u64 << attempt.min(6)).min(20_000)),
    }
}

fn should_retry(status: u16) -> bool {
    status == 429 || status == 502 || status == 503 || status == 504
}

/// The real client. Adds the bearer token, retries once with a fresh token on a 401, waits its
/// turn behind a limit on concurrent requests, and retries "slow down" and passing server
/// errors with a growing pause. Every Box call the app makes is safe to repeat (a taken name
/// comes back as 409 and is treated as "already there"), so retrying cannot duplicate anything.
pub struct ReqwestBoxHttp {
    client: reqwest::Client,
    tokens: Arc<dyn TokenProvider>,
    in_flight: tokio::sync::Semaphore,
}

impl ReqwestBoxHttp {
    pub fn new(tokens: Arc<dyn TokenProvider>) -> Self {
        let client = reqwest::Client::builder().timeout(REQUEST_TIMEOUT).build().unwrap_or_else(|_| reqwest::Client::new());
        Self { client, tokens, in_flight: tokio::sync::Semaphore::new(max_in_flight()) }
    }

    async fn attempt(&self, request: &BoxRequest, token: &str) -> Result<(u16, Option<u64>, Vec<u8>), StoreError> {
        let builder = match request.method {
            BoxMethod::Get => self.client.get(&request.url),
            BoxMethod::Post => self.client.post(&request.url),
            BoxMethod::Delete => self.client.delete(&request.url),
        };
        let mut builder = builder.bearer_auth(token).query(&request.query);
        builder = match &request.body {
            BoxBody::None => builder,
            BoxBody::Json(value) => builder.json(value),
            BoxBody::Upload { attributes, file_name, bytes } => {
                let file = reqwest::multipart::Part::bytes(bytes.clone())
                    .file_name(file_name.clone())
                    .mime_str("application/pdf")
                    .map_err(|e| StoreError::InvalidName(e.to_string()))?;
                let form = reqwest::multipart::Form::new().text("attributes", attributes.to_string()).part("file", file);
                builder.multipart(form)
            }
        };
        let response = builder
            .send()
            .await
            .map_err(|e| StoreError::Unavailable(format!("Box could not be reached: {}", e.without_url())))?;
        let status = response.status().as_u16();
        let retry_after = response.headers().get("retry-after").and_then(|v| v.to_str().ok()).and_then(|v| v.trim().parse::<u64>().ok());
        let body = response
            .bytes()
            .await
            .map_err(|e| StoreError::Unavailable(format!("Box response could not be read: {}", e.without_url())))?
            .to_vec();
        Ok((status, retry_after, body))
    }
}

#[async_trait]
impl BoxHttp for ReqwestBoxHttp {
    async fn send(&self, request: BoxRequest) -> Result<BoxResponse, StoreError> {
        let _turn = self.in_flight.acquire().await.map_err(|_| StoreError::Unavailable("Box requests are shutting down".into()))?;
        let mut refreshed = false;
        let mut attempt = 0u32;
        loop {
            let token = self.tokens.token().await?;
            match self.attempt(&request, &token).await {
                Ok((401, _, _)) if !refreshed => {
                    refreshed = true;
                    self.tokens.invalidate().await;
                }
                Ok((401, _, _)) => return Err(StoreError::Unavailable("Box refused the sign-in token twice".into())),
                Ok((status, retry_after, body)) => {
                    if should_retry(status) && attempt + 1 < MAX_ATTEMPTS {
                        tokio::time::sleep(backoff(attempt, retry_after)).await;
                        attempt += 1;
                        continue;
                    }
                    return Ok(BoxResponse { status, body });
                }
                // A network error or timeout: try again a few times before giving up.
                Err(e) => {
                    if attempt + 1 < MAX_ATTEMPTS {
                        tokio::time::sleep(backoff(attempt, None)).await;
                        attempt += 1;
                        continue;
                    }
                    return Err(e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_waits_longer_each_time_and_obeys_retry_after() {
        assert_eq!(backoff(0, None), std::time::Duration::from_millis(500));
        assert_eq!(backoff(1, None), std::time::Duration::from_millis(1000));
        assert_eq!(backoff(2, None), std::time::Duration::from_millis(2000));
        assert_eq!(backoff(0, Some(3)), std::time::Duration::from_secs(3));
        assert_eq!(backoff(0, Some(600)), std::time::Duration::from_secs(20));
    }

    #[test]
    fn only_slow_down_and_passing_server_errors_are_retried() {
        for status in [429, 502, 503, 504] {
            assert!(should_retry(status));
        }
        for status in [200, 201, 404, 409, 400, 403, 500] {
            assert!(!should_retry(status));
        }
    }
}
