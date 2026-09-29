use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::redact_headers;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SaasTransportProtocol {
    RestJson,
    SoapXml,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SaasHttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl SaasHttpMethod {
    pub fn permits_body(self) -> bool {
        matches!(self, Self::Post | Self::Put | Self::Patch)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "body", rename_all = "snake_case")]
pub enum SaasRequestBody {
    #[default]
    Empty,
    Json(Value),
    Text {
        content_type: String,
        value: String,
    },
    Bytes {
        content_type: String,
        byte_len: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sha256: Option<String>,
    },
}

impl SaasRequestBody {
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    pub fn content_type(&self) -> Option<&str> {
        match self {
            Self::Empty => None,
            Self::Json(_) => Some("application/json"),
            Self::Text { content_type, .. } | Self::Bytes { content_type, .. } => {
                Some(content_type.as_str())
            }
        }
    }

    fn validate(&self) -> Result<(), String> {
        match self {
            Self::Empty | Self::Json(_) => Ok(()),
            Self::Text {
                content_type,
                value: _,
            }
            | Self::Bytes {
                content_type,
                byte_len: _,
                sha256: _,
            } => validate_token("body.content_type", content_type),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SaasResponseCaps {
    pub max_body_bytes: u64,
    pub max_rows: u32,
}

impl SaasResponseCaps {
    pub const DEFAULT_MAX_BODY_BYTES: u64 = 1_048_576;
    pub const DEFAULT_MAX_ROWS: u32 = 250;

    pub fn validate(self) -> Result<(), String> {
        if self.max_body_bytes == 0 {
            return Err("max_body_bytes must be greater than 0".to_string());
        }
        if self.max_rows == 0 {
            return Err("max_rows must be greater than 0".to_string());
        }
        Ok(())
    }
}

impl Default for SaasResponseCaps {
    fn default() -> Self {
        Self {
            max_body_bytes: Self::DEFAULT_MAX_BODY_BYTES,
            max_rows: Self::DEFAULT_MAX_ROWS,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SaasRequestPlan {
    pub operation_name: String,
    pub protocol: SaasTransportProtocol,
    pub method: SaasHttpMethod,
    pub path: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub query: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "SaasRequestBody::is_empty")]
    pub body: SaasRequestBody,
    pub timeout_ms: u64,
    pub caps: SaasResponseCaps,
}

impl SaasRequestPlan {
    pub fn new(
        operation_name: impl Into<String>,
        protocol: SaasTransportProtocol,
        method: SaasHttpMethod,
        path: impl Into<String>,
    ) -> Self {
        Self {
            operation_name: operation_name.into(),
            protocol,
            method,
            path: path.into(),
            query: BTreeMap::new(),
            headers: BTreeMap::new(),
            body: SaasRequestBody::Empty,
            timeout_ms: 2_000,
            caps: SaasResponseCaps::default(),
        }
    }

    pub fn with_query(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.query.insert(name.into(), value.into());
        self
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    pub fn with_body(mut self, body: SaasRequestBody) -> Self {
        self.body = body;
        self
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn with_caps(mut self, caps: SaasResponseCaps) -> Self {
        self.caps = caps;
        self
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_token("operation_name", &self.operation_name)?;
        validate_relative_path(&self.path)?;

        if self.timeout_ms == 0 {
            return Err("timeout_ms must be greater than 0".to_string());
        }

        for (name, value) in &self.query {
            validate_token("query name", name)?;
            validate_single_line("query value", value)?;
        }

        for (name, value) in &self.headers {
            validate_token("header name", name)?;
            validate_single_line("header value", value)?;
        }

        if !self.method.permits_body() && !self.body.is_empty() {
            return Err("GET/DELETE request plans must not carry a body".to_string());
        }

        self.body.validate()?;
        self.caps.validate()
    }

    pub fn redacted_headers(&self) -> BTreeMap<String, String> {
        redact_headers(self.headers.clone())
    }

    pub fn redacted_summary(&self) -> Value {
        json!({
            "operation_name": self.operation_name,
            "protocol": self.protocol,
            "method": self.method,
            "path": "[REDACTED_PATH]",
            "query_keys": self.query.keys().collect::<Vec<_>>(),
            "headers": self.redacted_headers(),
            "has_body": !self.body.is_empty(),
            "body_content_type": self.body.content_type(),
            "timeout_ms": self.timeout_ms,
            "caps": self.caps,
        })
    }
}

fn validate_relative_path(path: &str) -> Result<(), String> {
    validate_single_line("path", path)?;

    if !path.starts_with('/') {
        return Err("path must be a relative absolute-path starting with /".to_string());
    }
    if path.starts_with("//") || path.contains("://") {
        return Err("path must not include a scheme, host, or protocol-relative URL".to_string());
    }
    if path.split('/').any(|segment| matches!(segment, "." | "..")) {
        return Err("path must not contain dot segments".to_string());
    }

    Ok(())
}

fn validate_token(name: &'static str, value: &str) -> Result<(), String> {
    validate_single_line(name, value)?;
    if value.trim().is_empty() {
        return Err(format!("{name} must not be empty"));
    }
    if value.chars().any(char::is_whitespace) {
        return Err(format!("{name} must not contain whitespace"));
    }
    Ok(())
}

fn validate_single_line(name: &'static str, value: &str) -> Result<(), String> {
    if value.contains('\n') || value.contains('\r') || value.chars().any(char::is_control) {
        return Err(format!("{name} must be a single-line value"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rest_request_plan_accepts_relative_paths_and_caps() {
        let plan = SaasRequestPlan::new(
            "salesforce.health.fetch_limits",
            SaasTransportProtocol::RestJson,
            SaasHttpMethod::Get,
            "/services/data/v67.0/limits",
        )
        .with_header("Authorization", "Bearer secret-token")
        .with_header("Accept", "application/json")
        .with_timeout_ms(5_000)
        .with_caps(SaasResponseCaps {
            max_body_bytes: 65_536,
            max_rows: 1,
        });

        plan.validate().expect("valid request plan");

        assert_eq!(plan.redacted_headers()["Authorization"], "[REDACTED]");
        assert_eq!(plan.redacted_summary()["path"], json!("[REDACTED_PATH]"));
        assert_eq!(plan.redacted_summary()["query_keys"], json!([]));
        assert_eq!(plan.redacted_summary()["has_body"], false);
    }

    #[test]
    fn anaplan_style_read_and_chunk_paths_stay_relative_and_redacted() {
        let read_page = SaasRequestPlan::new(
            "anaplan.view.get_read_page",
            SaasTransportProtocol::RestJson,
            SaasHttpMethod::Get,
            "/2/0/workspaces/ws-1/models/model-1/views/view-1/readRequests/request-1/pages/1",
        );
        let chunk = SaasRequestPlan::new(
            "anaplan.import.download_dump_chunk",
            SaasTransportProtocol::RestJson,
            SaasHttpMethod::Get,
            "/2/0/workspaces/ws-1/models/model-1/imports/import-1/tasks/task-1/dump/chunks/0",
        );

        read_page.validate().expect("read page path");
        chunk.validate().expect("chunk path");
        assert_eq!(
            read_page.redacted_summary()["path"],
            json!("[REDACTED_PATH]")
        );
        assert_eq!(chunk.redacted_summary()["path"], json!("[REDACTED_PATH]"));
        assert!(!read_page.redacted_summary().to_string().contains("ws-1"));
        assert!(!chunk.redacted_summary().to_string().contains("task-1"));
    }

    #[test]
    fn request_plan_rejects_absolute_anaplan_urls() {
        let plan = SaasRequestPlan::new(
            "anaplan.view.get_read_page",
            SaasTransportProtocol::RestJson,
            SaasHttpMethod::Get,
            "https://api.anaplan.com/2/0/workspaces/ws-1/models/model-1",
        );

        assert!(plan
            .validate()
            .unwrap_err()
            .contains("relative absolute-path"));
    }

    #[test]
    fn soap_request_plan_accepts_typed_xml_body() {
        let plan = SaasRequestPlan::new(
            "workday.hcm.get_workers_page",
            SaasTransportProtocol::SoapXml,
            SaasHttpMethod::Post,
            "/ccx/service/example/Human_Resources/v46.1",
        )
        .with_body(SaasRequestBody::Text {
            content_type: "text/xml".to_string(),
            value: "<soapenv:Envelope/>".to_string(),
        });

        assert_eq!(plan.body.content_type(), Some("text/xml"));
        plan.validate().expect("valid SOAP request plan");
    }

    #[test]
    fn request_plan_rejects_raw_external_urls_and_dot_segments() {
        let absolute = SaasRequestPlan::new(
            "salesforce.health.fetch_limits",
            SaasTransportProtocol::RestJson,
            SaasHttpMethod::Get,
            "https://example.my.salesforce.com/services/data/v67.0/limits",
        );
        let dot_segment = SaasRequestPlan::new(
            "salesforce.health.fetch_limits",
            SaasTransportProtocol::RestJson,
            SaasHttpMethod::Get,
            "/services/../limits",
        );

        assert!(absolute.validate().unwrap_err().contains("path"));
        assert!(dot_segment.validate().unwrap_err().contains("dot segments"));
    }

    #[test]
    fn request_plan_rejects_get_body_and_header_injection() {
        let with_body = SaasRequestPlan::new(
            "salesforce.health.fetch_limits",
            SaasTransportProtocol::RestJson,
            SaasHttpMethod::Get,
            "/services/data/v67.0/limits",
        )
        .with_body(SaasRequestBody::Json(json!({ "bad": true })));
        let with_bad_header = SaasRequestPlan::new(
            "salesforce.health.fetch_limits",
            SaasTransportProtocol::RestJson,
            SaasHttpMethod::Get,
            "/services/data/v67.0/limits",
        )
        .with_header("x-test", "ok\r\nx-evil: yes");

        assert!(with_body.validate().unwrap_err().contains("body"));
        assert!(with_bad_header
            .validate()
            .unwrap_err()
            .contains("single-line"));
    }
}
