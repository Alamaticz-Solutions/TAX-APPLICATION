use axum::http::{header, HeaderName, HeaderValue, Method};
use std::{env, str::FromStr};
use tower_http::cors::CorsLayer;
use tracing::{error, warn};

const LOCAL_DEFAULT_ORIGIN: &str = "http://localhost:3000";

pub fn get() -> CorsLayer {
    let cors = CorsLayer::new()
        .allow_headers(vec![
            header::ACCEPT,
            header::ACCEPT_LANGUAGE,
            header::ACCEPT_ENCODING,
            header::AUTHORIZATION,
            header::CONTENT_LANGUAGE,
            header::ACCESS_CONTROL_ALLOW_METHODS,
            header::ACCESS_CONTROL_REQUEST_HEADERS,
            header::CONTENT_TYPE,
            header::REFERER,
            HeaderName::from_str("timezone").unwrap(),
        ])
        .allow_methods(vec![
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::HEAD,
            Method::OPTIONS,
            Method::PATCH,
        ])
        .allow_credentials(true)
        .allow_origin(resolve_allowed_origins());
    // Credentials are always enabled, so the origin list must remain an explicit
    // allow-list. Never combine `tower_http::cors::Any` (wildcard) with credentials.

    cors
}

/// Resolve the CORS allow-list from `APP_CORS_ALLOWED_ORIGINS` (comma separated).
///
/// Fail-closed: in managed (non-`local`) environments an empty/unset list yields
/// an empty allow-list so no cross-origin credentialed request is ever honored.
/// The localhost dev origin is only applied when `ENV_NAME=local`.
fn resolve_allowed_origins() -> Vec<HeaderValue> {
    let configured = parse_origins(env::var("APP_CORS_ALLOWED_ORIGINS").ok().as_deref());
    if !configured.is_empty() {
        return configured;
    }

    if is_local_env() {
        return parse_origins(Some(LOCAL_DEFAULT_ORIGIN));
    }

    error!(
        "APP_CORS_ALLOWED_ORIGINS is not set in a managed environment; CORS allow-list is empty (fail-closed)"
    );
    Vec::new()
}

fn is_local_env() -> bool {
    env::var("ENV_NAME").map(|v| v == "local").unwrap_or(false)
}

fn parse_origins(value: Option<&str>) -> Vec<HeaderValue> {
    value
        .into_iter()
        .flat_map(|raw| raw.split(','))
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
        .filter_map(|origin| match origin.parse::<HeaderValue>() {
            Ok(value) => Some(value),
            Err(error) => {
                warn!(origin = %origin, error = %error, "ignoring invalid CORS origin");
                None
            }
        })
        .collect()
}

// Access-Control-Allow-Headers
//  https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Access-Control-Allow-Headers
//
//  The Access-Control-Allow-Headers response header is used in response to a preflight request which includes the
//  Access-Control-Request-Headers to indicate which HTTP headers can be used during the actual request.
//
//  This header is required if the request has an Access-Control-Request-Headers header.

// Access-Control-Allow-Credentials
//  https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Access-Control-Allow-Credentials
//
//  The Access-Control-Allow-Credentials response header tells browsers whether to expose the response to the
//  frontend JavaScript code when the request's credentials mode (Request.credentials) is include.
//
//  When a request's credentials mode (Request.credentials) is include, browsers will only expose the response
//  to the frontend JavaScript code if the Access-Control-Allow-Credentials value is true.
//
//  Credentials are cookies, authorization headers, or TLS client certificates.
//
//  When used as part of a response to a preflight request, this indicates whether or not the actual request
//  can be made using credentials. Note that simple GET requests are not preflighted. So, if a request is made
//  for a resource with credentials, and if this header is not returned with the resource, the response is ignored
//  by the browser and not returned to the web content.
//
//  The Access-Control-Allow-Credentials header works in conjunction with the XMLHttpRequest.withCredentials
//  property or with the credentials option in the Request() constructor of the Fetch API. For a CORS request with
//  credentials, for browsers to expose the response to the frontend JavaScript code,
//  both the server (using the Access-Control-Allow-Credentials header)
//  and the client (by setting the credentials mode for the XHR, Fetch, or Ajax request)
//  must indicate that they're opting into including credentials.

// Access-Control-Allow-Methods
//  https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Access-Control-Allow-Methods
//
//  The Access-Control-Allow-Methods response header specifies one or more methods allowed when accessing a resource in
//  response to a preflight request.

// Access-Control-Allow-Origin
//  https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Access-Control-Allow-Origin
//
//  The Access-Control-Allow-Origin response header indicates whether the response can be shared with requesting code from
//  the given origin.
//
//  *
//    For requests without credentials, the literal value "*" can be specified as a wildcard; the value tells browsers to allow
//    requesting code from any origin to access the resource. Attempting to use the wildcard with credentials results in an error.
//
//  <origin>
//    Specifies an origin. Only a single origin can be specified. If the server supports clients from multiple origins, it must
//    return the origin for the specific client making the request.
//
//  null
//    Specifies the origin "null".
