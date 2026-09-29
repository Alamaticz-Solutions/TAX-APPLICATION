//! Shared primitives for App Framework SaaS connector providers.
//!
//! This crate intentionally avoids HTTP clients and provider-specific behavior.
//! Provider crates can build on these request shapes, cache decisions,
//! pagination contracts, retry policies, and redaction helpers while keeping
//! transport and vendor semantics outside the core library.

pub mod oauth;
pub mod pagination;
pub mod rate_limit;
pub mod redaction;
pub mod request;
pub mod retry;
pub mod transfer;

pub use oauth::{
    ClientCredentialsAuthStyle, M2mAccessToken, M2mTokenCache, M2mTokenCacheKey,
    OAuthClientCredentialsRequest, OAuthClientCredentialsRequestBuilder, OAuthFormRequest,
    SecretString, TokenRefreshDecision,
};
pub use pagination::{
    AsyncTaskContinuation, OffsetLimitContinuation, PageCountContinuation, PageDirection,
    PageRequest, PageResult, PaginationContinuation, PaginationCursor, ReadRequestPageContinuation,
    UrlCursorContinuation,
};
pub use rate_limit::{
    parse_retry_after_seconds, parse_salesforce_limit_info, ApiUsageLimit, RateLimitSignal,
};
pub use redaction::{
    is_sensitive_header_name, is_sensitive_json_key, redact_header_value, redact_headers,
    redact_json_value, redact_text, REDACTED,
};
pub use request::{
    SaasHttpMethod, SaasRequestBody, SaasRequestPlan, SaasResponseCaps, SaasTransportProtocol,
};
pub use retry::{BackoffPolicy, RetryDecision, RetryPolicy};
pub use transfer::{ChunkDownloadContinuation, SaasTransferCaps};
