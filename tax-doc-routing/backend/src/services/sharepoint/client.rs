//! Executor: a named READ operation + bound params -> classified, redacted
//! result. The ONLY place this module makes an outbound SharePoint (Graph)
//! GET, reachable only through [`super::registry::ReadOperation`]. There is no
//! `get(path)`.

use std::sync::Arc;
use std::time::Duration;

use crate::services::graph::auth::{GraphAuthConfig, GraphToken};

use super::{
    identity::SHAREPOINT_GRAPH_BASE,
    registry::ReadOperation,
    response::{classify, redact, retry_after_from, GraphError, MAX_BODY_BYTES},
};

pub struct SharePointClient {
    token: Arc<GraphToken>,
    http: reqwest::Client,
}

impl SharePointClient {
    /// Takes the shared Graph read credential. The drive id is supplied
    /// per-call by the caller's [`ReadOperation`].
    pub fn from_read_config(cfg: GraphAuthConfig, http: reqwest::Client) -> Self {
        Self {
            token: Arc::new(GraphToken::new(cfg, http.clone())),
            http,
        }
    }

    /// Execute a registered read. Returns the (redacted) JSON body.
    #[tracing::instrument(
        name = "sharepoint.read",
        skip(self, op),
        fields(operation = op.name(), pii_possible = op.pii_possible())
    )]
    pub async fn read(&self, op: ReadOperation) -> anyhow::Result<serde_json::Value> {
        let plan = op
            .plan()
            .map_err(|error| anyhow::anyhow!("sharepoint {} plan rejected: {error:?}", op.name()))?;
        let url = format!("{SHAREPOINT_GRAPH_BASE}{}", plan.path);
        tracing::info!(path = %plan.path, method = %plan.method, "outbound SharePoint (Graph) read");
        let bearer = self.token.bearer().await?;

        let mut req = self
            .http
            .request(plan.method.clone(), &url)
            .bearer_auth(bearer)
            .header(reqwest::header::ACCEPT, plan.accept.join(", "));
        for (k, v) in &plan.query {
            req = req.query(&[(k, v)]);
        }
        for (k, v) in &plan.headers {
            req = req.header(*k, v);
        }

        let resp = req
            .send()
            .await
            .map_err(|_| anyhow::anyhow!("sharepoint {} request failed (transport)", op.name()))?;
        let status = resp.status();
        let retry_after = retry_after_from(resp.headers());
        let bytes = resp
            .bytes()
            .await
            .map_err(|_| anyhow::anyhow!("sharepoint {} body read failed", op.name()))?;

        interpret_response(op.name(), status, retry_after, &bytes)
    }
}

/// Pure response handling, split out so it is unit-testable without a network.
pub(crate) fn interpret_response(
    operation: &str,
    status: reqwest::StatusCode,
    retry_after: Option<Duration>,
    bytes: &[u8],
) -> anyhow::Result<serde_json::Value> {
    if bytes.len() > MAX_BODY_BYTES {
        return Err(anyhow::anyhow!(
            "sharepoint {operation} response exceeded {MAX_BODY_BYTES} bytes"
        ));
    }
    let value: serde_json::Value = serde_json::from_slice(bytes).unwrap_or(serde_json::Value::Null);

    if !status.is_success() {
        let mut err = classify(status, &value);
        if let GraphError::RateLimited { retry_after: ra } = &mut err {
            *ra = retry_after;
        }
        tracing::warn!(status = status.as_u16(), "SharePoint (Graph) read returned an error status");
        return Err(anyhow::Error::new(err));
    }
    Ok(redact(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode;

    #[test]
    fn success_is_redacted() {
        let body = serde_json::json!({
            "id": "i1",
            "@microsoft.graph.downloadUrl": "https://x/y",
            "access_token": "leak"
        });
        let out = interpret_response(
            "get_drive_item",
            StatusCode::OK,
            None,
            body.to_string().as_bytes(),
        )
        .unwrap();
        assert_eq!(out["id"], "i1");
        assert_eq!(out["access_token"], "[REDACTED]");
    }

    #[test]
    fn not_found_and_unauthorized_are_classified() {
        let nf = interpret_response("x", StatusCode::NOT_FOUND, None, b"{}").unwrap_err();
        assert!(matches!(
            nf.downcast_ref::<GraphError>(),
            Some(GraphError::NotFound)
        ));
        let body = br#"{"error":{"code":"accessDenied"}}"#;
        let ua = interpret_response("x", StatusCode::FORBIDDEN, None, body).unwrap_err();
        assert!(matches!(
            ua.downcast_ref::<GraphError>(),
            Some(GraphError::Unauthorized { code }) if code == "accessDenied"
        ));
    }

    #[test]
    fn throttling_carries_retry_after() {
        let err = interpret_response(
            "x",
            StatusCode::TOO_MANY_REQUESTS,
            Some(Duration::from_secs(7)),
            b"{}",
        )
        .unwrap_err();
        assert!(matches!(
            err.downcast_ref::<GraphError>(),
            Some(GraphError::RateLimited { retry_after: Some(d) }) if d.as_secs() == 7
        ));
    }

    #[test]
    fn oversized_bodies_are_rejected() {
        let big = vec![b' '; MAX_BODY_BYTES + 1];
        assert!(interpret_response("x", StatusCode::OK, None, &big).is_err());
    }
}
