use std::collections::HashMap;
use std::fmt;

use chrono::{DateTime, FixedOffset};
use uuid::Uuid;

/// ODBC-backed row with name-based column access.
#[derive(Clone, Debug, Default)]
pub struct MssqlRow {
    columns: HashMap<String, Option<String>>,
}

#[derive(Debug)]
pub struct MssqlRowError {
    message: String,
}

impl fmt::Display for MssqlRowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for MssqlRowError {}

impl MssqlRow {
    pub fn from_columns(columns: HashMap<String, Option<String>>) -> Self {
        Self { columns }
    }

    pub fn insert(&mut self, column: impl Into<String>, value: Option<String>) {
        self.columns.insert(column.into(), value);
    }

    pub fn try_get<T, S>(&self, column: S) -> Result<Option<T>, MssqlRowError>
    where
        S: AsRef<str>,
        T: TryFromRowValue,
    {
        let key = column.as_ref();
        let raw = self
            .columns
            .get(key)
            .or_else(|| self.columns.get(&key.to_ascii_lowercase()))
            .or_else(|| {
                self.columns
                    .iter()
                    .find(|(name, _)| name.eq_ignore_ascii_case(key))
                    .map(|(_, value)| value)
            });

        match raw {
            None => Err(MssqlRowError {
                message: format!("column `{key}` not found in row"),
            }),
            Some(None) => Ok(None),
            Some(Some(value)) => T::try_from_row_value(value.as_str(), key).map(Some),
        }
    }
}

pub trait TryFromRowValue: Sized {
    fn try_from_row_value(value: &str, column: &str) -> Result<Self, MssqlRowError>;
}

impl TryFromRowValue for String {
    fn try_from_row_value(value: &str, _column: &str) -> Result<Self, MssqlRowError> {
        Ok(value.to_string())
    }
}

impl TryFromRowValue for i16 {
    fn try_from_row_value(value: &str, column: &str) -> Result<Self, MssqlRowError> {
        value
            .trim()
            .parse::<i16>()
            .map_err(|e| parse_err(column, value, e))
    }
}

impl TryFromRowValue for i32 {
    fn try_from_row_value(value: &str, column: &str) -> Result<Self, MssqlRowError> {
        value
            .trim()
            .parse::<i32>()
            .map_err(|e| parse_err(column, value, e))
    }
}

impl TryFromRowValue for i64 {
    fn try_from_row_value(value: &str, column: &str) -> Result<Self, MssqlRowError> {
        value
            .trim()
            .parse::<i64>()
            .map_err(|e| parse_err(column, value, e))
    }
}

impl TryFromRowValue for f64 {
    fn try_from_row_value(value: &str, column: &str) -> Result<Self, MssqlRowError> {
        value
            .trim()
            .parse::<f64>()
            .map_err(|e| parse_err(column, value, e))
    }
}

impl TryFromRowValue for Uuid {
    fn try_from_row_value(value: &str, column: &str) -> Result<Self, MssqlRowError> {
        Uuid::parse_str(value.trim()).map_err(|e| parse_err(column, value, e))
    }
}

impl TryFromRowValue for DateTime<FixedOffset> {
    fn try_from_row_value(value: &str, column: &str) -> Result<Self, MssqlRowError> {
        DateTime::parse_from_rfc3339(value.trim())
            .or_else(|_| DateTime::parse_from_str(value.trim(), "%Y-%m-%d %H:%M:%S%.f"))
            .map_err(|e| parse_err(column, value, e))
    }
}

fn parse_err(column: &str, value: &str, err: impl fmt::Display) -> MssqlRowError {
    MssqlRowError {
        message: format!("column `{column}` value `{value}`: {err}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_reads_typed_columns() {
        let mut row = MssqlRow::default();
        row.insert("id", Some("42".to_string()));
        row.insert("name", Some("alice".to_string()));

        assert_eq!(row.try_get::<i32, _>("id").unwrap(), Some(42));
        assert_eq!(
            row.try_get::<String, _>("name").unwrap(),
            Some("alice".to_string())
        );
    }
}
