use std::{
    fs,
    io::{Error, ErrorKind, Result},
    path::Path,
};

use serde::Serialize;

use super::{io_error, Migration, Phase};
use crate::console;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize)]
pub struct LintIssue {
    pub severity: Severity,
    pub code: &'static str,
    pub migration_id: String,
    pub migration_name: String,
    pub line: usize,
    pub message: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LintReport {
    pub data_source: String,
    pub checked: usize,
    pub errors: usize,
    pub warnings: usize,
    pub issues: Vec<LintIssue>,
}

impl LintReport {
    pub fn has_errors(&self) -> bool {
        self.errors > 0
    }
}

pub fn lint_migrations(
    migrations_root: &Path,
    data_source: &str,
    migrations: &[Migration],
) -> Result<LintReport> {
    let mut issues = Vec::new();
    for migration in migrations {
        let sql = migration.sql(migrations_root)?;
        issues.extend(lint_sql(migration, &sql));
    }

    let errors = issues
        .iter()
        .filter(|issue| issue.severity == Severity::Error)
        .count();
    let warnings = issues.len().saturating_sub(errors);
    Ok(LintReport {
        data_source: data_source.to_string(),
        checked: migrations.len(),
        errors,
        warnings,
        issues,
    })
}

pub fn write_report(report: &LintReport) -> Result<()> {
    let path = std::env::current_dir()?.join("target/appfw").join(format!(
        "migration_lint_{}.json",
        report_name(&report.data_source)
    ));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        &path,
        serde_json::to_string_pretty(report)
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?
            + "\n",
    )?;
    console::step_path("migration lint report", &path);
    Ok(())
}

pub fn print_report(report: &LintReport) {
    console::step(format!(
        "migration lint: {} checked, {} error(s), {} warning(s)",
        report.checked, report.errors, report.warnings
    ));

    for issue in report.issues.iter().take(20) {
        let severity = match issue.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        console::warn(format!(
            "{severity} {} {}:{} - {}",
            issue.code, issue.migration_id, issue.line, issue.message
        ));
    }
    if report.issues.len() > 20 {
        console::warn(format!(
            "{} additional lint issue(s) written to report",
            report.issues.len() - 20
        ));
    }
}

pub fn fail_on_errors(report: &LintReport) -> Result<()> {
    if report.has_errors() {
        return Err(io_error(format!(
            "migration lint failed with {} error(s); see target/appfw/migration_lint_{}.json",
            report.errors,
            report_name(&report.data_source)
        )));
    }
    Ok(())
}

fn report_name(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn lint_sql(migration: &Migration, sql: &str) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    let lines: Vec<&str> = sql.lines().collect();
    let mut emitted_non_idempotent_insert = false;
    let mut emitted_blocking_index = false;
    let mut emitted_missing_if = false;
    let rollback_reviewed = has_rollback_review_marker(sql);

    for (idx, raw_line) in lines.iter().enumerate() {
        let line_no = idx + 1;
        let line = strip_inline_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }
        let normalized = normalize_sql(line);

        if contains_contract_operation(&normalized) && migration.phase != Phase::Contract {
            issues.push(issue(
                Severity::Error,
                "unsafe_contract_operation",
                migration,
                line_no,
                "destructive schema operation is not in a contract migration",
                "Move DROP/TRUNCATE operations to a contract migration and require --confirm-contract.",
            ));
        }
        if contains_contract_operation(&normalized)
            && migration.phase == Phase::Contract
            && !rollback_reviewed
        {
            issues.push(issue(
                Severity::Warning,
                "missing_rollback_review_marker",
                migration,
                line_no,
                "destructive contract operation lacks an appfw rollback review marker",
                "Add a reviewed `-- appfw: rollback-reviewed <ticket>` marker after app rollback and recovery plans are approved.",
            ));
        }

        if starts_unbounded_update_or_delete(&normalized, &lines, idx) {
            issues.push(issue(
                Severity::Error,
                "unbounded_data_change",
                migration,
                line_no,
                "UPDATE or DELETE does not include a WHERE clause",
                "Constrain data changes by key range or checkpoint predicate.",
            ));
        }

        if !emitted_blocking_index
            && is_blocking_index_create(&normalized, migration.dialect.as_str())
        {
            emitted_blocking_index = true;
            issues.push(issue(
                Severity::Warning,
                "blocking_index_creation",
                migration,
                line_no,
                "index creation may take blocking locks",
                "Use PostgreSQL CREATE INDEX CONCURRENTLY or online/low-priority provider options where supported.",
            ));
        }

        if !emitted_missing_if && is_create_without_guard(&normalized, &lines, idx) {
            emitted_missing_if = true;
            issues.push(issue(
                Severity::Warning,
                "missing_idempotency_guard",
                migration,
                line_no,
                "CREATE statement does not include an obvious IF EXISTS/IF NOT EXISTS guard",
                "Make migration DDL idempotent or wrap it in a catalog existence check.",
            ));
        }

        if is_long_lock_risk(&normalized) {
            issues.push(issue(
                Severity::Warning,
                "long_lock_risk",
                migration,
                line_no,
                "statement may rewrite or lock a large table",
                "Use expand/backfill/contract sequencing, chunked backfills, and nullable-first column changes.",
            ));
        }

        if !emitted_non_idempotent_insert && is_insert_without_guard(&normalized, &lines, idx) {
            emitted_non_idempotent_insert = true;
            issues.push(issue(
                Severity::Warning,
                "non_idempotent_insert",
                migration,
                line_no,
                "INSERT statement does not include an obvious idempotency guard",
                "Use ON CONFLICT/IF NOT EXISTS/MERGE or make the backfill resumable.",
            ));
        }
    }

    issues
}

fn has_rollback_review_marker(sql: &str) -> bool {
    sql.lines().any(|line| {
        let normalized = line.trim_start().to_ascii_lowercase();
        normalized
            .strip_prefix("-- appfw: rollback-reviewed ")
            .is_some_and(|ticket| !ticket.trim().is_empty())
    })
}

fn issue(
    severity: Severity,
    code: &'static str,
    migration: &Migration,
    line: usize,
    message: &str,
    suggestion: &str,
) -> LintIssue {
    LintIssue {
        severity,
        code,
        migration_id: migration.id.clone(),
        migration_name: migration.name.clone(),
        line,
        message: message.to_string(),
        suggestion: suggestion.to_string(),
    }
}

fn normalize_sql(line: &str) -> String {
    line.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn strip_inline_comment(line: &str) -> &str {
    line.split_once("--")
        .map(|(prefix, _)| prefix)
        .unwrap_or(line)
}

fn contains_contract_operation(normalized: &str) -> bool {
    normalized.contains("drop table")
        || normalized.contains("drop column")
        || normalized.contains("drop constraint")
        || normalized.starts_with("truncate ")
        || normalized.contains(" truncate ")
}

fn starts_unbounded_update_or_delete(normalized: &str, lines: &[&str], idx: usize) -> bool {
    let is_update = normalized.starts_with("update ");
    let is_delete = normalized.starts_with("delete from ");
    if !is_update && !is_delete {
        return false;
    }
    if contains_where(normalized) {
        return false;
    }
    !statement_context_contains_where(lines, idx)
}

fn statement_context_contains_where(lines: &[&str], idx: usize) -> bool {
    for raw_line in lines.iter().skip(idx + 1) {
        let line = strip_inline_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }
        let normalized = normalize_sql(line);
        if contains_where(&normalized) {
            return true;
        }
        if normalized.contains(';') {
            return false;
        }
    }
    false
}

fn contains_where(normalized: &str) -> bool {
    normalized == "where" || normalized.starts_with("where ") || normalized.contains(" where ")
}

fn is_blocking_index_create(normalized: &str, dialect: &str) -> bool {
    normalized.starts_with("create index ")
        && !normalized.contains(" concurrently ")
        && !(dialect == "mssql" && normalized.contains(" online = on"))
}

fn is_create_without_guard(normalized: &str, lines: &[&str], idx: usize) -> bool {
    let is_create = normalized.starts_with("create table ")
        || normalized.starts_with("create schema ")
        || normalized.starts_with("create index ");
    if !is_create || normalized.contains(" if not exists ") {
        return false;
    }
    !previous_context_contains(lines, idx, &["if not exists", "object_id", "sys.schemas"])
}

fn is_insert_without_guard(normalized: &str, lines: &[&str], idx: usize) -> bool {
    if !normalized.starts_with("insert into ") {
        return false;
    }
    if normalized.contains(" on conflict ")
        || normalized.contains(" where not exists ")
        || normalized.contains(" if not exists ")
        || normalized.contains(" merge ")
    {
        return false;
    }
    !previous_context_contains(
        lines,
        idx,
        &["if not exists", "where not exists", "on conflict"],
    )
}

fn is_long_lock_risk(normalized: &str) -> bool {
    (normalized.starts_with("alter table ") && normalized.contains(" set not null"))
        || (normalized.starts_with("alter table ") && normalized.contains(" alter column "))
        || (normalized.starts_with("alter table ") && normalized.contains(" add constraint "))
}

fn previous_context_contains(lines: &[&str], idx: usize, needles: &[&str]) -> bool {
    let start = idx.saturating_sub(6);
    lines[start..idx].iter().any(|line| {
        let normalized = normalize_sql(line);
        needles.iter().any(|needle| normalized.contains(needle))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn migration(phase: Phase, dialect: &str) -> Migration {
        Migration {
            id: "1".to_string(),
            name: "test".to_string(),
            schema: Some("crm".to_string()),
            data_source: "pg_primary".to_string(),
            dialect: dialect.to_string(),
            phase,
            path: "test.sql".into(),
            description: None,
        }
    }

    #[test]
    fn linter_blocks_unbounded_update_and_drop_outside_contract() {
        let issues = lint_sql(
            &migration(Phase::Backfill, "postgresql"),
            "UPDATE crm.accounts SET version = 1;\nDROP TABLE crm.accounts;",
        );

        assert!(issues
            .iter()
            .any(|issue| issue.code == "unbounded_data_change"));
        assert!(issues
            .iter()
            .any(|issue| issue.code == "unsafe_contract_operation"));
    }

    #[test]
    fn linter_allows_bounded_update() {
        let issues = lint_sql(
            &migration(Phase::Backfill, "postgresql"),
            "UPDATE crm.accounts SET version = 1 WHERE version IS NULL;",
        );

        assert!(issues.is_empty());
    }

    #[test]
    fn linter_allows_multiline_bounded_update_inside_routine() {
        let issues = lint_sql(
            &migration(Phase::Expand, "postgresql"),
            r#"
CREATE OR REPLACE PROCEDURE crm.refresh_account_health_stored_procedure(
    p_account_id text
)
LANGUAGE plpgsql
AS $$
BEGIN
    UPDATE crm.accounts
    SET health_last_refreshed_at = NOW()
    WHERE id = p_account_id::uuid
    RETURNING id INTO p_account_id;
END;
$$;
"#,
        );

        assert!(
            issues
                .iter()
                .all(|issue| issue.code != "unbounded_data_change"),
            "unexpected unbounded data-change lint issue: {issues:?}"
        );
    }

    #[test]
    fn contract_destructive_operation_requires_review_marker() {
        let issues = lint_sql(
            &migration(Phase::Contract, "postgresql"),
            "DROP TABLE crm.legacy_accounts;",
        );

        assert!(issues
            .iter()
            .any(|issue| issue.code == "missing_rollback_review_marker"));

        let reviewed = lint_sql(
            &migration(Phase::Contract, "postgresql"),
            "-- appfw: rollback-reviewed CHG-123\nDROP TABLE crm.legacy_accounts;",
        );
        assert!(reviewed
            .iter()
            .all(|issue| issue.code != "missing_rollback_review_marker"));

        let guidance_only = lint_sql(
            &migration(Phase::Contract, "postgresql"),
            "-- Add `-- appfw: rollback-reviewed <ticket>` after review.\nDROP TABLE crm.legacy_accounts;",
        );
        assert!(guidance_only
            .iter()
            .any(|issue| issue.code == "missing_rollback_review_marker"));
    }
}
