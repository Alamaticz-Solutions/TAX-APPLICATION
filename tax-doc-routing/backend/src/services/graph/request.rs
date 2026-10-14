//! Safe request-plan construction. Path templates and `$select` / `$filter`
//! field lists are FIXED per named operation; callers pass only bound value
//! parameters. Values are parameter-escaped, never string-concatenated into
//! query text such as `$filter=JoinWebUrl eq '-'` or `$search`.

#[derive(Debug, Clone)]
pub struct RequestPlan {
    pub method: reqwest::Method,
    /// Path relative to the Graph base (no host). Placeholders already substituted
    /// with path-segment-escaped values.
    pub path: String,
    /// Query parameters, each value already escaped by `reqwest` at send time.
    pub query: Vec<(String, String)>,
    /// Extra headers required by the operation (e.g. `ConsistencyLevel: eventual`).
    pub headers: Vec<(&'static str, String)>,
    /// Expected response media types, most-preferred first.
    pub accept: Vec<&'static str>,
}

impl RequestPlan {
    pub fn get(path: impl Into<String>) -> Self {
        Self {
            method: reqwest::Method::GET,
            path: path.into(),
            query: Vec::new(),
            headers: Vec::new(),
            accept: vec!["application/json"],
        }
    }

    pub fn post(path: impl Into<String>) -> Self {
        Self {
            method: reqwest::Method::POST,
            path: path.into(),
            query: Vec::new(),
            headers: Vec::new(),
            accept: vec!["application/json"],
        }
    }

    pub fn query(mut self, k: &str, v: impl Into<String>) -> Self {
        self.query.push((k.to_string(), v.into()));
        self
    }

    pub fn header(mut self, k: &'static str, v: impl Into<String>) -> Self {
        self.headers.push((k, v.into()));
        self
    }

    pub fn accept(mut self, media: Vec<&'static str>) -> Self {
        self.accept = media;
        self
    }
}

/// Percent-encode a value for use inside an OData string literal
/// (`'-'`). Single quotes are doubled per OData rules; the whole
/// literal is then handed to `reqwest` as a query-param value.
pub fn odata_string_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// Escape a value destined for a single URL *path segment*.
pub fn path_segment(value: &str) -> String {
    // Graph ids are opaque tokens (`MSo-`, GUIDs, UPNs). Reject anything with a
    // path separator or control char rather than silently truncating a route.
    if value.is_empty()
        || value.contains('/')
        || value.contains('?')
        || value.contains('#')
        || value.chars().any(|c| c.is_control())
    {
        // Substitute a token that will 404 rather than escape the intended route.
        return "__invalid__".to_string();
    }
    value.to_string()
}
