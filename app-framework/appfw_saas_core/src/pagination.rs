use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PageDirection {
    #[default]
    Forward,
    Backward,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PaginationCursor {
    pub token: String,
    #[serde(default)]
    pub direction: PageDirection,
}

impl PaginationCursor {
    pub fn forward(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            direction: PageDirection::Forward,
        }
    }

    pub fn backward(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            direction: PageDirection::Backward,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.token.trim().is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UrlCursorContinuation {
    pub url: String,
}

impl UrlCursorContinuation {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_non_empty_single_line("url cursor continuation", &self.url)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PageCountContinuation {
    pub page: u32,
    pub count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stable_as_of: Option<String>,
}

impl PageCountContinuation {
    pub fn new(page: u32, count: u32) -> Self {
        Self {
            page,
            count,
            stable_as_of: None,
        }
    }

    pub fn with_stable_as_of(mut self, stable_as_of: impl Into<String>) -> Self {
        self.stable_as_of = Some(stable_as_of.into());
        self
    }

    pub fn next_page(&self) -> Self {
        Self {
            page: self.page.saturating_add(1),
            count: self.count,
            stable_as_of: self.stable_as_of.clone(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.page == 0 {
            return Err("page/count continuation page must be greater than 0".to_string());
        }
        if self.count == 0 {
            return Err("page/count continuation count must be greater than 0".to_string());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OffsetLimitContinuation {
    pub offset: u64,
    pub limit: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_size: Option<u64>,
}

impl OffsetLimitContinuation {
    pub fn new(offset: u64, limit: u32) -> Self {
        Self {
            offset,
            limit,
            total_size: None,
        }
    }

    pub fn with_total_size(mut self, total_size: u64) -> Self {
        self.total_size = Some(total_size);
        self
    }

    pub fn next_page(&self) -> Self {
        Self {
            offset: self.offset.saturating_add(self.limit as u64),
            limit: self.limit,
            total_size: self.total_size,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.limit == 0 {
            return Err("offset/limit continuation limit must be greater than 0".to_string());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AsyncTaskContinuation {
    pub task_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poll_after_ms: Option<u64>,
    #[serde(default)]
    pub attempt: u32,
}

impl AsyncTaskContinuation {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            poll_after_ms: None,
            attempt: 0,
        }
    }

    pub fn with_poll_after_ms(mut self, poll_after_ms: u64) -> Self {
        self.poll_after_ms = Some(poll_after_ms);
        self
    }

    pub fn with_attempt(mut self, attempt: u32) -> Self {
        self.attempt = attempt;
        self
    }

    pub fn next_attempt(&self) -> Self {
        Self {
            task_id: self.task_id.clone(),
            poll_after_ms: self.poll_after_ms,
            attempt: self.attempt.saturating_add(1),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_non_empty_single_line("async task continuation task_id", &self.task_id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReadRequestPageContinuation {
    pub request_id: String,
    pub page: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_size: Option<u32>,
}

impl ReadRequestPageContinuation {
    pub fn new(request_id: impl Into<String>, page: u32) -> Self {
        Self {
            request_id: request_id.into(),
            page,
            page_size: None,
        }
    }

    pub fn with_page_size(mut self, page_size: u32) -> Self {
        self.page_size = Some(page_size);
        self
    }

    pub fn next_page(&self) -> Self {
        Self {
            request_id: self.request_id.clone(),
            page: self.page.saturating_add(1),
            page_size: self.page_size,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_non_empty_single_line("read request continuation request_id", &self.request_id)?;

        if self.page == 0 {
            return Err("read request continuation page must be greater than 0".to_string());
        }

        if self.page_size == Some(0) {
            return Err("read request continuation page_size must be greater than 0".to_string());
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "continuation", rename_all = "snake_case")]
pub enum PaginationContinuation {
    UrlCursor(UrlCursorContinuation),
    PageCount(PageCountContinuation),
    OffsetLimit(OffsetLimitContinuation),
    AsyncTask(AsyncTaskContinuation),
    ReadRequestPage(ReadRequestPageContinuation),
}

impl PaginationContinuation {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::UrlCursor(continuation) => continuation.validate(),
            Self::PageCount(continuation) => continuation.validate(),
            Self::OffsetLimit(continuation) => continuation.validate(),
            Self::AsyncTask(continuation) => continuation.validate(),
            Self::ReadRequestPage(continuation) => continuation.validate(),
        }
    }
}

fn validate_non_empty_single_line(name: &'static str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{name} must not be empty"));
    }

    if value.contains('\n') || value.contains('\r') || value.chars().any(char::is_control) {
        return Err(format!("{name} must be a single-line value"));
    }

    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PageRequest {
    pub limit: u32,
    pub cursor: Option<PaginationCursor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation: Option<PaginationContinuation>,
}

impl PageRequest {
    pub fn first_page(limit: u32) -> Self {
        Self {
            limit,
            cursor: None,
            continuation: None,
        }
    }

    pub fn with_cursor(limit: u32, cursor: PaginationCursor) -> Self {
        Self {
            limit,
            cursor: Some(cursor),
            continuation: None,
        }
    }

    pub fn with_continuation(limit: u32, continuation: PaginationContinuation) -> Self {
        Self {
            limit,
            cursor: None,
            continuation: Some(continuation),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PageResult<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<PaginationCursor>,
    pub previous_cursor: Option<PaginationCursor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_continuation: Option<PaginationContinuation>,
    #[serde(default)]
    pub metadata: Value,
}

impl<T> PageResult<T> {
    pub fn new(items: Vec<T>) -> Self {
        Self {
            items,
            next_cursor: None,
            previous_cursor: None,
            next_continuation: None,
            metadata: Value::Null,
        }
    }

    pub fn with_next_cursor(mut self, cursor: PaginationCursor) -> Self {
        self.next_cursor = Some(cursor);
        self
    }

    pub fn with_previous_cursor(mut self, cursor: PaginationCursor) -> Self {
        self.previous_cursor = Some(cursor);
        self
    }

    pub fn with_next_continuation(mut self, continuation: PaginationContinuation) -> Self {
        self.next_continuation = Some(continuation);
        self
    }

    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = metadata;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn page_request_can_represent_first_and_cursor_pages() {
        let first = PageRequest::first_page(100);
        let next = PageRequest::with_cursor(100, PaginationCursor::forward("next-1"));

        assert_eq!(first.cursor, None);
        assert_eq!(first.continuation, None);
        assert_eq!(next.cursor.unwrap().direction, PageDirection::Forward);
    }

    #[test]
    fn page_request_can_represent_vendor_continuation_shapes() {
        let salesforce = PageRequest::with_continuation(
            2_000,
            PaginationContinuation::UrlCursor(UrlCursorContinuation::new(
                "/services/data/v67.0/query/01g-next",
            )),
        );
        let workday = PageRequest::with_continuation(
            100,
            PaginationContinuation::PageCount(
                PageCountContinuation::new(2, 100).with_stable_as_of("2026-06-26T00:00:00Z"),
            ),
        );
        let oracle = PageRequest::with_continuation(
            500,
            PaginationContinuation::OffsetLimit(OffsetLimitContinuation::new(500, 500)),
        );
        let anaplan = PageRequest::with_continuation(
            1,
            PaginationContinuation::AsyncTask(
                AsyncTaskContinuation::new("task-1").with_poll_after_ms(10_000),
            ),
        );
        let anaplan_read_page = PageRequest::with_continuation(
            1,
            PaginationContinuation::ReadRequestPage(
                ReadRequestPageContinuation::new("read-request-1", 1).with_page_size(25),
            ),
        );

        assert!(salesforce.continuation.unwrap().validate().is_ok());
        assert!(workday.continuation.unwrap().validate().is_ok());
        assert!(oracle.continuation.unwrap().validate().is_ok());
        assert!(anaplan.continuation.unwrap().validate().is_ok());
        assert!(anaplan_read_page.continuation.unwrap().validate().is_ok());
    }

    #[test]
    fn read_request_page_continuation_serializes_with_stable_variant_name() {
        let continuation = PaginationContinuation::ReadRequestPage(
            ReadRequestPageContinuation::new("read-request-1", 2).with_page_size(25),
        );

        let serialized = serde_json::to_value(&continuation).expect("serializes");
        assert_eq!(serialized["kind"], "read_request_page");
        assert_eq!(serialized["continuation"]["request_id"], "read-request-1");
        assert_eq!(serialized["continuation"]["page"], 2);

        let round_trip: PaginationContinuation =
            serde_json::from_value(serialized).expect("deserializes");
        assert_eq!(round_trip, continuation);
    }

    #[test]
    fn page_result_carries_bidirectional_cursors_and_metadata() {
        let page = PageResult::new(vec![json!({ "id": 1 })])
            .with_next_cursor(PaginationCursor::forward("next-1"))
            .with_previous_cursor(PaginationCursor::backward("prev-1"))
            .with_next_continuation(PaginationContinuation::OffsetLimit(
                OffsetLimitContinuation::new(100, 100),
            ))
            .with_metadata(json!({ "source": "unit-test" }));

        assert_eq!(page.items.len(), 1);
        assert_eq!(page.next_cursor.unwrap().token, "next-1");
        assert_eq!(
            page.previous_cursor.unwrap().direction,
            PageDirection::Backward
        );
        assert_eq!(
            page.next_continuation,
            Some(PaginationContinuation::OffsetLimit(
                OffsetLimitContinuation::new(100, 100)
            ))
        );
        assert_eq!(page.metadata["source"], "unit-test");
    }

    #[test]
    fn continuation_helpers_advance_without_losing_stability_context() {
        let workday = PageCountContinuation::new(1, 100)
            .with_stable_as_of("2026-06-26T00:00:00Z")
            .next_page();
        let oracle = OffsetLimitContinuation::new(0, 500)
            .with_total_size(2_000)
            .next_page();
        let anaplan = AsyncTaskContinuation::new("task-1")
            .with_poll_after_ms(5_000)
            .next_attempt();
        let read_request = ReadRequestPageContinuation::new("read-request-1", 1)
            .with_page_size(25)
            .next_page();

        assert_eq!(workday.page, 2);
        assert_eq!(
            workday.stable_as_of.as_deref(),
            Some("2026-06-26T00:00:00Z")
        );
        assert_eq!(oracle.offset, 500);
        assert_eq!(oracle.total_size, Some(2_000));
        assert_eq!(anaplan.attempt, 1);
        assert_eq!(anaplan.poll_after_ms, Some(5_000));
        assert_eq!(read_request.page, 2);
        assert_eq!(read_request.page_size, Some(25));
    }

    #[test]
    fn invalid_continuations_report_clear_errors() {
        assert!(UrlCursorContinuation::new("\n").validate().is_err());
        assert!(PageCountContinuation::new(0, 100)
            .validate()
            .unwrap_err()
            .contains("page"));
        assert!(OffsetLimitContinuation::new(0, 0)
            .validate()
            .unwrap_err()
            .contains("limit"));
        assert!(AsyncTaskContinuation::new("")
            .validate()
            .unwrap_err()
            .contains("task_id"));
        assert!(ReadRequestPageContinuation::new("", 1)
            .validate()
            .unwrap_err()
            .contains("request_id"));
        assert!(ReadRequestPageContinuation::new("read-request-1", 0)
            .validate()
            .unwrap_err()
            .contains("page"));
    }
}
