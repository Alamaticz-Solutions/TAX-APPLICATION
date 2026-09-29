//! Observability boundary for the generated backend.
//!
//! Keep tracing, request correlation, GraphQL error annotation, and in-process
//! metrics behind this module so routes and data access code can record useful
//! signals without knowing which exporter is enabled.

pub mod metrics;
pub mod telemetry;

#[cfg(feature = "http")]
pub use metrics::metrics_hook;
pub use metrics::MetricsRegistry;
#[cfg(feature = "http")]
pub use telemetry::{
    annotate_graphql_response, graphql_error_with_context, http_make_span, trace_context_hook,
    REQUEST_ID_HEADER_NAME,
};
pub use telemetry::{
    current_request_context, init_tracing, redact_diagnostic_text, redact_diagnostic_value,
    RequestContext,
};
