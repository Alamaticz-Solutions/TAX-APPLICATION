use async_graphql::{InputObject, SimpleObject};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type JsonQueryResult = appfw_runtime::RuntimeJsonQueryResult;
pub type JsonAggregateResult = appfw_runtime::RuntimeJsonAggregateResult;

#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[graphql(rename_fields = "snake_case")]
pub struct QueryResult<T: std::marker::Send + std::marker::Sync + async_graphql::OutputType> {
    pub date_time: String,
    pub request_duration: f64,
    pub skip: i32,
    pub limit: i32,
    pub page_count: i32,
    pub page_index: i32,
    pub query_count: i64,
    pub next_cursor: Option<String>,
    pub previous_cursor: Option<String>,
    pub items: Vec<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[graphql(rename_fields = "snake_case")]
pub struct AggregateResult {
    pub date_time: String,
    pub request_duration: f64,
    pub skip: i32,
    pub limit: i32,
    pub page_count: i32,
    pub page_index: i32,
    pub query_count: i64,
    pub items: Vec<Value>,
}

// Auth schema DTOs are public GraphQL/API contract types. Unit-test builds may
// not construct them directly even though downstream callers can.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[graphql(rename_fields = "snake_case")]
pub struct Login {
    pub email: String,
    pub password: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, InputObject)]
#[graphql(rename_fields = "snake_case")]
pub struct InputLogin {
    pub email: String,
    pub password: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[graphql(rename_fields = "snake_case")]
pub struct LoginResultProjection {
    pub error: Option<String>,
    pub user_id: Option<String>,
    pub user_name: Option<String>,
    pub user_email: Option<String>,
    pub full_name: Option<String>,
    pub auth_token: Option<String>,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}
