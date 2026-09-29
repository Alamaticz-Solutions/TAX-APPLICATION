#![allow(dead_code)]

#[derive(Debug, Clone)]
pub struct BulkRows {
    pub schema: String,
    pub table: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
}

impl BulkRows {
    pub fn new(
        schema: impl Into<String>,
        table: impl Into<String>,
        columns: Vec<String>,
        rows: Vec<Vec<Option<String>>>,
    ) -> Self {
        Self {
            schema: schema.into(),
            table: table.into(),
            columns,
            rows,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct BackfillCheckpoint {
    pub job_name: String,
    pub chunk_key: String,
    pub last_value: Option<String>,
    pub rows_processed: i64,
}
