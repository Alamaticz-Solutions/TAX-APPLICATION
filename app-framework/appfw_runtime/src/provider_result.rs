use std::{fmt::Display, time::Instant};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub type RuntimeJsonObj = Map<String, Value>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeJsonQueryResult {
    #[serde(rename = "date_time")]
    pub date_time: String,
    #[serde(rename = "request_duration")]
    pub request_duration: f64,
    #[serde(rename = "skip")]
    pub skip: i32,
    #[serde(rename = "limit")]
    pub limit: i32,
    #[serde(rename = "page_count")]
    pub page_count: i32,
    #[serde(rename = "page_index")]
    pub page_index: i32,
    #[serde(rename = "query_count")]
    pub query_count: i64,
    #[serde(rename = "next_cursor")]
    pub next_cursor: Option<String>,
    #[serde(rename = "previous_cursor")]
    pub previous_cursor: Option<String>,
    #[serde(rename = "items")]
    pub items: Vec<RuntimeJsonObj>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeJsonAggregateResult {
    #[serde(rename = "date_time")]
    pub date_time: String,
    #[serde(rename = "request_duration")]
    pub request_duration: f64,
    #[serde(rename = "skip")]
    pub skip: i32,
    #[serde(rename = "limit")]
    pub limit: i32,
    #[serde(rename = "page_count")]
    pub page_count: i32,
    #[serde(rename = "page_index")]
    pub page_index: i32,
    #[serde(rename = "query_count")]
    pub query_count: i64,
    #[serde(rename = "items")]
    pub items: Vec<Value>,
}

impl RuntimeJsonQueryResult {
    pub fn new(
        skip: i32,
        limit: i32,
        query_count: i64,
        items: Vec<RuntimeJsonObj>,
        request_datetime: impl Display,
        request_start: Instant,
    ) -> Self {
        Self {
            date_time: request_datetime.to_string(),
            request_duration: request_start.elapsed().as_secs_f64(),
            skip,
            limit,
            page_count: 1,
            page_index: page_index(skip, limit),
            query_count,
            next_cursor: None,
            previous_cursor: None,
            items,
        }
    }
}

impl RuntimeJsonAggregateResult {
    pub fn new(
        skip: i32,
        limit: i32,
        query_count: i64,
        items: Vec<Value>,
        request_datetime: impl Display,
        request_start: Instant,
    ) -> Self {
        Self {
            date_time: request_datetime.to_string(),
            request_duration: request_start.elapsed().as_secs_f64(),
            skip,
            limit,
            page_count: 1,
            page_index: page_index(skip, limit),
            query_count,
            items,
        }
    }
}

fn page_index(skip: i32, limit: i32) -> i32 {
    match limit {
        0 => 0,
        _ => skip / limit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn json_query_result_uses_stable_provider_envelope() {
        let started_at = Instant::now();
        let result = RuntimeJsonQueryResult::new(
            20,
            10,
            42,
            vec![json!({ "id": "account-1" })
                .as_object()
                .expect("object")
                .clone()],
            "2026-05-29T00:00:00Z",
            started_at,
        );

        assert_eq!(result.date_time, "2026-05-29T00:00:00Z");
        assert_eq!(result.page_count, 1);
        assert_eq!(result.page_index, 2);
        assert_eq!(result.query_count, 42);
        assert_eq!(result.items[0]["id"], json!("account-1"));
    }

    #[test]
    fn json_aggregate_result_uses_stable_provider_envelope() {
        let result = RuntimeJsonAggregateResult::new(
            0,
            0,
            3,
            vec![json!({ "count": 2 })],
            "2026-05-29T00:00:00Z",
            Instant::now(),
        );

        assert_eq!(result.page_index, 0);
        assert_eq!(result.query_count, 3);
        assert_eq!(result.items, vec![json!({ "count": 2 })]);
    }
}
