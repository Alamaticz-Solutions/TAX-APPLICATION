use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Error, ErrorKind, Result},
    path::Path,
};

use serde::Serialize;

use super::{io_error, Migration, Phase};
use crate::{console, mssql::MsSql, postgres::Postgres};

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct ActualColumn {
    pub schema: String,
    pub table: String,
    pub column: String,
    pub data_type: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
struct ExpectedColumn {
    schema: String,
    table: String,
    column: String,
    data_type: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize)]
pub struct DriftIssue {
    pub severity: Severity,
    pub code: &'static str,
    pub schema: String,
    pub table: String,
    pub column: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DriftReport {
    pub provider: String,
    pub data_source: String,
    pub checked_tables: usize,
    pub errors: usize,
    pub warnings: usize,
    pub issues: Vec<DriftIssue>,
}

impl DriftReport {
    pub fn has_errors(&self) -> bool {
        self.errors > 0
    }
}

pub async fn check_postgres(
    pg: &Postgres,
    migrations_root: &Path,
    migrations: &[Migration],
) -> Result<DriftReport> {
    let expected = expected_columns(migrations_root, migrations, "postgresql")?;
    let schemas = expected
        .iter()
        .map(|column| column.schema.clone())
        .collect::<BTreeSet<_>>();
    let mut actual = Vec::new();
    for schema in schemas {
        actual.extend(pg.schema_columns(&schema).await?);
    }
    Ok(compare_expected_actual(
        "postgresql",
        migrations
            .first()
            .map(|migration| migration.data_source.as_str())
            .unwrap_or("unknown"),
        expected,
        actual,
    ))
}

pub async fn check_mssql(
    ms: &MsSql,
    migrations_root: &Path,
    migrations: &[Migration],
) -> Result<DriftReport> {
    let expected = expected_columns(migrations_root, migrations, "mssql")?;
    let schemas = expected
        .iter()
        .map(|column| column.schema.clone())
        .collect::<BTreeSet<_>>();
    let mut actual = Vec::new();
    for schema in schemas {
        actual.extend(ms.schema_columns(&schema).await?);
    }
    Ok(compare_expected_actual(
        "mssql",
        migrations
            .first()
            .map(|migration| migration.data_source.as_str())
            .unwrap_or("unknown"),
        expected,
        actual,
    ))
}

pub fn print_report(report: &DriftReport) {
    console::step(format!(
        "schema drift: {} table(s), {} error(s), {} warning(s)",
        report.checked_tables, report.errors, report.warnings
    ));
    for issue in report.issues.iter().take(20) {
        let severity = match issue.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        console::warn(format!(
            "{severity} {} {}.{}.{} - {}",
            issue.code, issue.schema, issue.table, issue.column, issue.message
        ));
    }
    if report.issues.len() > 20 {
        console::warn(format!(
            "{} additional drift issue(s) written to report",
            report.issues.len() - 20
        ));
    }
}

pub fn write_report(report: &DriftReport) -> Result<()> {
    let path = std::env::current_dir()?
        .join("target/appfw")
        .join(format!("schema_drift_{}.json", report.data_source));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        &path,
        serde_json::to_string_pretty(report)
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?
            + "\n",
    )?;
    console::step_path("schema drift report", &path);
    Ok(())
}

pub fn fail_on_errors(report: &DriftReport) -> Result<()> {
    if report.has_errors() {
        return Err(io_error(format!(
            "schema drift check failed with {} error(s); see target/appfw/schema_drift_{}.json",
            report.errors, report.data_source
        )));
    }
    Ok(())
}

fn expected_columns(
    migrations_root: &Path,
    migrations: &[Migration],
    dialect: &str,
) -> Result<Vec<ExpectedColumn>> {
    let mut expected = BTreeSet::new();
    for migration in migrations
        .iter()
        .filter(|migration| migration.phase == Phase::Expand && migration.dialect == dialect)
    {
        let sql = migration.sql(migrations_root)?;
        let parsed = if dialect == "postgresql" {
            parse_postgres_expected(&sql)
        } else {
            parse_mssql_expected(&sql)
        };
        expected.extend(parsed);
    }
    Ok(expected.into_iter().collect())
}

fn compare_expected_actual(
    provider: &str,
    data_source: &str,
    expected: Vec<ExpectedColumn>,
    actual: Vec<ActualColumn>,
) -> DriftReport {
    let expected_by_key = expected
        .iter()
        .map(|column| {
            (
                key(&column.schema, &column.table, &column.column),
                column.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let actual_by_key = actual
        .iter()
        .map(|column| {
            (
                key(&column.schema, &column.table, &column.column),
                column.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut issues = Vec::new();
    for (column_key, expected_column) in &expected_by_key {
        let Some(actual_column) = actual_by_key.get(column_key) else {
            continue;
        };
        if !types_compatible(&expected_column.data_type, &actual_column.data_type) {
            issues.push(DriftIssue {
                severity: Severity::Error,
                code: "column_type_mismatch",
                schema: expected_column.schema.clone(),
                table: expected_column.table.clone(),
                column: expected_column.column.clone(),
                expected: Some(expected_column.data_type.clone()),
                actual: Some(actual_column.data_type.clone()),
                message: "actual column type differs from generated expectation".to_string(),
            });
        }
    }

    for (column_key, actual_column) in &actual_by_key {
        if !expected_by_key.contains_key(column_key) {
            issues.push(DriftIssue {
                severity: Severity::Warning,
                code: "unexpected_column",
                schema: actual_column.schema.clone(),
                table: actual_column.table.clone(),
                column: actual_column.column.clone(),
                expected: None,
                actual: Some(actual_column.data_type.clone()),
                message: "actual database has a column not present in generated schema".to_string(),
            });
        }
    }

    let errors = issues
        .iter()
        .filter(|issue| issue.severity == Severity::Error)
        .count();
    let warnings = issues.len().saturating_sub(errors);
    let checked_tables = expected
        .iter()
        .map(|column| format!("{}.{}", column.schema, column.table))
        .collect::<BTreeSet<_>>()
        .len();

    DriftReport {
        provider: provider.to_string(),
        data_source: data_source.to_string(),
        checked_tables,
        errors,
        warnings,
        issues,
    }
}

fn parse_postgres_expected(sql: &str) -> Vec<ExpectedColumn> {
    let mut columns = BTreeSet::new();
    let mut current_table: Option<(String, String)> = None;
    for raw_line in sql.lines() {
        let line = raw_line.trim().trim_end_matches(';').trim_end_matches(',');
        let normalized = line.to_ascii_lowercase();

        if let Some(rest) = normalized.strip_prefix("create table ") {
            current_table = parse_postgres_table_name(rest);
            continue;
        }
        if normalized == ")" || normalized == ");" {
            current_table = None;
            continue;
        }
        if normalized.starts_with("alter table ") && normalized.contains(" add column ") {
            if let Some(column) = parse_postgres_alter_column(line) {
                columns.insert(column);
            }
            continue;
        }
        if let Some((schema, table)) = &current_table {
            if let Some((column, data_type)) = parse_postgres_column_line(line) {
                columns.insert(ExpectedColumn {
                    schema: schema.clone(),
                    table: table.clone(),
                    column,
                    data_type,
                });
            }
        }
    }
    columns.into_iter().collect()
}

fn parse_mssql_expected(sql: &str) -> Vec<ExpectedColumn> {
    let mut columns = BTreeSet::new();
    let mut current_table: Option<(String, String)> = None;
    for raw_line in sql.lines() {
        let line = raw_line.trim().trim_end_matches(';').trim_end_matches(',');
        let normalized = line.to_ascii_lowercase();

        if normalized.starts_with("create table ") {
            current_table = parse_mssql_table_name(line);
            continue;
        }
        if normalized == ")" || normalized == "go" || normalized == "end" {
            current_table = None;
            continue;
        }
        if normalized.starts_with("alter table ") && normalized.contains(" add ") {
            if let Some(column) = parse_mssql_alter_column(line) {
                columns.insert(column);
            }
            continue;
        }
        if let Some((schema, table)) = &current_table {
            if let Some((column, data_type)) = parse_mssql_column_line(line) {
                columns.insert(ExpectedColumn {
                    schema: schema.clone(),
                    table: table.clone(),
                    column,
                    data_type,
                });
            }
        }
    }
    columns.into_iter().collect()
}

fn parse_postgres_table_name(rest: &str) -> Option<(String, String)> {
    let token = rest.split_whitespace().next()?;
    let (schema, table) = token.split_once('.')?;
    Some((unquote_ident(schema), unquote_ident(table)))
}

fn parse_postgres_alter_column(line: &str) -> Option<ExpectedColumn> {
    let normalized = line.to_ascii_lowercase();
    let table_start = normalized.strip_prefix("alter table ")?;
    let table_token = table_start.split_whitespace().next()?;
    let (schema, table) = table_token.split_once('.')?;
    let column_marker = "add column if not exists ";
    let column_rest = normalized
        .split_once(column_marker)
        .or_else(|| normalized.split_once("add column "))?
        .1;
    let (column, data_type) = parse_postgres_column_line(column_rest)?;
    Some(ExpectedColumn {
        schema: unquote_ident(schema),
        table: unquote_ident(table),
        column,
        data_type,
    })
}

fn parse_postgres_column_line(line: &str) -> Option<(String, String)> {
    let line = line.trim().trim_start_matches(',').trim();
    if !line.starts_with('"') {
        return None;
    }
    let rest = line.strip_prefix('"')?;
    let (column, after_column) = rest.split_once('"')?;
    let data_type = after_column
        .trim()
        .split_whitespace()
        .take_while(|part| !matches!(*part, "not" | "default" | "primary" | "constraint"))
        .collect::<Vec<_>>()
        .join(" ");
    if data_type.is_empty() {
        None
    } else {
        Some((column.to_string(), normalize_type(&data_type)))
    }
}

fn parse_mssql_table_name(line: &str) -> Option<(String, String)> {
    let open = line.find('[')?;
    let rest = &line[open..];
    let mut parts = bracket_idents(rest);
    Some((parts.next()?, parts.next()?))
}

fn parse_mssql_alter_column(line: &str) -> Option<ExpectedColumn> {
    let mut parts = bracket_idents(line);
    let schema = parts.next()?;
    let table = parts.next()?;
    let column = parts.next()?;
    let after_add = line.to_ascii_lowercase();
    let data_type = after_add.split_once(" add ")?.1;
    let data_type = data_type.split_once(']').map(|(_, rest)| rest)?.trim();
    Some(ExpectedColumn {
        schema,
        table,
        column,
        data_type: normalize_type(data_type),
    })
}

fn parse_mssql_column_line(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    if !line.starts_with('[') {
        return None;
    }
    let column = bracket_idents(line).next()?;
    let data_type = line.split_once(']')?.1.trim();
    Some((column, normalize_type(data_type)))
}

fn bracket_idents(line: &str) -> impl Iterator<Item = String> + '_ {
    line.match_indices('[').filter_map(|(start, _)| {
        let rest = &line[start + 1..];
        let end = rest.find(']')?;
        Some(rest[..end].to_string())
    })
}

fn key(schema: &str, table: &str, column: &str) -> String {
    format!(
        "{}.{}.{}",
        schema.to_ascii_lowercase(),
        table.to_ascii_lowercase(),
        column.to_ascii_lowercase()
    )
}

fn unquote_ident(value: &str) -> String {
    value
        .trim_matches('"')
        .trim_matches('[')
        .trim_matches(']')
        .to_string()
}

fn normalize_type(value: &str) -> String {
    value
        .trim()
        .trim_end_matches(';')
        .trim_end_matches(',')
        .split_whitespace()
        .take_while(|part| {
            !matches!(
                part.to_ascii_lowercase().as_str(),
                "not" | "null" | "default" | "primary" | "constraint" | "collate"
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn types_compatible(expected: &str, actual: &str) -> bool {
    canonical_type(expected) == canonical_type(actual)
}

fn canonical_type(value: &str) -> &str {
    match value {
        "character varying" | "varchar" | "nvarchar" | "nvarchar(450)" => "string",
        "double precision" | "float" => "float64",
        "real" => "float32",
        "integer" | "int" => "int32",
        "smallint" => "int16",
        "bigint" => "int64",
        "uuid" | "uniqueidentifier" => "uuid",
        "boolean" | "bit" => "bool",
        "timestamp" | "timestamp without time zone" => "timestamp",
        "timestamp with time zone" | "timestamptz" | "datetimeoffset" | "datetimeoffset(7)" => {
            "datetime"
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_postgres_create_and_alter_columns() {
        let columns = parse_postgres_expected(
            r#"
CREATE TABLE crm.accounts (
  "id" uuid NOT NULL PRIMARY KEY,
  ,"name" varchar
);
ALTER TABLE crm.accounts ADD COLUMN IF NOT EXISTS "version" bigint;
"#,
        );

        assert!(columns
            .iter()
            .any(|c| c.table == "accounts" && c.column == "id"));
        assert!(columns
            .iter()
            .any(|c| c.table == "accounts" && c.column == "version"));
    }

    #[test]
    fn parses_mssql_create_and_alter_columns() {
        let columns = parse_mssql_expected(
            r#"
CREATE TABLE [crm].[accounts] (
  [id] uniqueidentifier NOT NULL,
  [name] nvarchar(450)
);
ALTER TABLE [crm].[accounts] ADD [version] bigint;
"#,
        );

        assert!(columns
            .iter()
            .any(|c| c.table == "accounts" && c.column == "id"));
        assert!(columns
            .iter()
            .any(|c| c.table == "accounts" && c.column == "version"));
    }

    #[test]
    fn compare_flags_type_mismatch_as_error() {
        let report = compare_expected_actual(
            "postgresql",
            "pg_primary",
            vec![ExpectedColumn {
                schema: "crm".to_string(),
                table: "accounts".to_string(),
                column: "id".to_string(),
                data_type: "uuid".to_string(),
            }],
            vec![ActualColumn {
                schema: "crm".to_string(),
                table: "accounts".to_string(),
                column: "id".to_string(),
                data_type: "integer".to_string(),
            }],
        );

        assert_eq!(report.errors, 1);
    }

    #[test]
    fn compare_allows_provider_timestamp_aliases() {
        let report = compare_expected_actual(
            "postgresql",
            "pg_primary",
            vec![
                ExpectedColumn {
                    schema: "crm".to_string(),
                    table: "accounts".to_string(),
                    column: "created_at".to_string(),
                    data_type: "timestamp".to_string(),
                },
                ExpectedColumn {
                    schema: "crm".to_string(),
                    table: "accounts".to_string(),
                    column: "health_last_refreshed_at".to_string(),
                    data_type: "timestamptz".to_string(),
                },
            ],
            vec![
                ActualColumn {
                    schema: "crm".to_string(),
                    table: "accounts".to_string(),
                    column: "created_at".to_string(),
                    data_type: "timestamp without time zone".to_string(),
                },
                ActualColumn {
                    schema: "crm".to_string(),
                    table: "accounts".to_string(),
                    column: "health_last_refreshed_at".to_string(),
                    data_type: "timestamp with time zone".to_string(),
                },
            ],
        );

        assert_eq!(report.errors, 0);
    }
}
