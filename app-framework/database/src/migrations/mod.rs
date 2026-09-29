use std::{
    env, fs,
    io::{Error, ErrorKind, Result},
    path::{Path, PathBuf},
};

use serde::Serialize;
use serde_derive::Deserialize;
use sha2::{Digest, Sha256};

use crate::{
    console,
    data_source::{DataSource, DataSourceType},
    loader,
};

pub mod drift;
mod lint;
mod mssql;
mod postgres;
mod scaffold;

#[derive(Debug, Clone)]
pub enum Command {
    Bootstrap,
    Doctor {
        json: bool,
    },
    New(scaffold::NewMigrationOptions),
    Plan {
        selection: PhaseSelection,
        json: bool,
    },
    Lint {
        selection: PhaseSelection,
        json: bool,
    },
    RollbackGuide {
        json: bool,
    },
    Drift {
        json: bool,
    },
    Status {
        json: bool,
    },
    Apply(ApplyOptions),
}

impl Command {
    pub fn from_env_args() -> Result<Self> {
        let args: Vec<String> = env::args().skip(1).collect();
        parse_args(&args)
    }

    pub fn json_output(&self) -> bool {
        match self {
            Self::Doctor { json }
            | Self::Plan { json, .. }
            | Self::Lint { json, .. }
            | Self::RollbackGuide { json }
            | Self::Drift { json }
            | Self::Status { json } => *json,
            Self::Apply(options) => options.json,
            Self::New(options) => options.json,
            Self::Bootstrap => false,
        }
    }

    fn command_name(&self) -> &'static str {
        match self {
            Self::Bootstrap => "bootstrap",
            Self::Doctor { .. } => "doctor",
            Self::New(_) => "new",
            Self::Plan { .. } => "plan",
            Self::Lint { .. } => "lint",
            Self::RollbackGuide { .. } => "rollback-guide",
            Self::Drift { .. } => "drift",
            Self::Status { .. } => "status",
            Self::Apply(_) => "apply",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApplyOptions {
    pub selection: PhaseSelection,
    pub confirm_contract: bool,
    pub json: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseSelection {
    Safe,
    One(Phase),
    All,
}

impl PhaseSelection {
    fn includes(self, phase: Phase) -> bool {
        match self {
            Self::Safe => phase != Phase::Contract,
            Self::One(selected) => selected == phase,
            Self::All => true,
        }
    }

    fn includes_contract(self) -> bool {
        match self {
            Self::Safe => false,
            Self::One(phase) => phase == Phase::Contract,
            Self::All => true,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Safe => "safe phases (expand, backfill)",
            Self::One(Phase::Expand) => "expand",
            Self::One(Phase::Backfill) => "backfill",
            Self::One(Phase::Contract) => "contract",
            Self::All => "all phases",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Expand,
    Backfill,
    Contract,
}

impl Phase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Expand => "expand",
            Self::Backfill => "backfill",
            Self::Contract => "contract",
        }
    }
}

#[derive(Debug, Deserialize)]
struct Manifest {
    migrations: Vec<Migration>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Migration {
    pub id: String,
    pub name: String,
    pub schema: Option<String>,
    pub data_source: String,
    pub dialect: String,
    pub phase: Phase,
    pub path: PathBuf,
    pub description: Option<String>,
}

impl Migration {
    pub fn file_path(&self, migrations_root: &Path) -> PathBuf {
        migrations_root.join(&self.path)
    }

    pub fn checksum(&self, migrations_root: &Path) -> Result<String> {
        let bytes = fs::read(self.file_path(migrations_root))?;
        let digest = Sha256::digest(&bytes);
        Ok(hex::encode(digest))
    }

    pub fn sql(&self, migrations_root: &Path) -> Result<String> {
        fs::read_to_string(self.file_path(migrations_root))
    }
}

#[derive(Debug, Clone)]
pub struct MigrationRecord {
    pub checksum: String,
    pub status: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationState {
    Pending,
    Applied,
    Failed,
    ChecksumMismatch,
}

impl MigrationState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Applied => "applied",
            Self::Failed => "failed",
            Self::ChecksumMismatch => "checksum-mismatch",
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MigrationRunReport {
    pub command: String,
    pub ok: bool,
    pub selection: Option<String>,
    pub data_sources: Vec<DataSourceMigrationReport>,
    pub lint_reports: Vec<lint::LintReport>,
    pub drift_reports: Vec<drift::DriftReport>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RollbackGuidanceReport {
    pub command: &'static str,
    pub ok: bool,
    pub policy: &'static str,
    pub phases: Vec<RollbackPhaseGuidance>,
    pub required_checks: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct RollbackPhaseGuidance {
    pub phase: &'static str,
    pub rollback_strategy: &'static str,
    pub operator_guidance: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct DataSourceMigrationReport {
    pub data_source: String,
    pub dialect: String,
    pub skipped: Option<String>,
    pub migrations: Vec<MigrationReportItem>,
}

pub struct ProviderRunReport {
    pub data_source: DataSourceMigrationReport,
    pub drift_report: Option<drift::DriftReport>,
}

#[derive(Debug, Serialize)]
pub struct MigrationReportItem {
    pub id: String,
    pub name: String,
    pub schema: Option<String>,
    pub phase: Phase,
    pub status: String,
    pub checksum: Option<String>,
    pub duration_ms: Option<i64>,
    pub description: Option<String>,
    pub error: Option<String>,
}

impl MigrationRunReport {
    fn new(command: &Command) -> Self {
        Self {
            command: command.command_name().to_string(),
            ok: true,
            selection: selection_label(command),
            data_sources: Vec::new(),
            lint_reports: Vec::new(),
            drift_reports: Vec::new(),
            error: None,
        }
    }

    fn fail(&mut self, error: &str) {
        self.ok = false;
        self.error = Some(error.to_string());
    }
}

impl DataSourceMigrationReport {
    pub fn new(data_source: impl Into<String>, dialect: impl Into<String>) -> Self {
        Self {
            data_source: data_source.into(),
            dialect: dialect.into(),
            skipped: None,
            migrations: Vec::new(),
        }
    }

    pub fn skipped(
        data_source: impl Into<String>,
        dialect: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            data_source: data_source.into(),
            dialect: dialect.into(),
            skipped: Some(reason.into()),
            migrations: Vec::new(),
        }
    }
}

pub fn migration_report_item(
    migration: &Migration,
    state: MigrationState,
    checksum: Option<String>,
) -> MigrationReportItem {
    MigrationReportItem {
        id: migration.id.clone(),
        name: migration.name.clone(),
        schema: migration.schema.clone(),
        phase: migration.phase,
        status: state.label().to_string(),
        checksum,
        duration_ms: None,
        description: migration.description.clone(),
        error: None,
    }
}

pub async fn run(command: Command) -> Result<()> {
    if let Command::New(options) = &command {
        let report = scaffold::create_migration(options)?;
        if options.json {
            print_json(&report)?;
        } else {
            console::stage("migration scaffold");
            console::item(
                "migration",
                format!("{} [{}]", report.name, report.phase.as_str()),
            );
            for file in &report.files {
                console::step_path("created", Path::new(&file.path));
            }
            console::step_path("manifest", Path::new(&report.manifest));
        }
        return Ok(());
    }

    if let Command::Doctor { json } = command {
        let report = crate::doctor::run().await?;
        if json {
            print_json(&report)?;
        } else {
            crate::doctor::print(&report);
        }
        if report.ok {
            return Ok(());
        }
        return Err(io_error("database doctor reported readiness failures"));
    }

    if let Command::RollbackGuide { json } = command {
        let report = rollback_guidance_report();
        if json {
            print_json(&report)?;
        } else {
            print_rollback_guidance(&report);
        }
        return Ok(());
    }

    let mut report = MigrationRunReport::new(&command);
    console::stage("versioned migrations");

    let data_sources = if matches!(command, Command::Lint { .. }) {
        loader::get_configured_data_sources()?
    } else {
        loader::get_data_sources()?
    };
    let migrations_root = loader::get_migrations_path()?;
    let manifest = load_manifest(&migrations_root)?;

    match &command {
        Command::Plan { selection, .. } => console::step(format!("plan: {}", selection.label())),
        Command::Lint { selection, .. } => console::step(format!("lint: {}", selection.label())),
        Command::Drift { .. } => console::step("drift"),
        Command::Status { .. } => console::step("status"),
        Command::Apply(options) => {
            if options.selection.includes_contract() && !options.confirm_contract {
                let error =
                    "contract migrations require --confirm-contract; rollback is app-first and DB-forward";
                report.fail(error);
                finish_json_report(&command, &report)?;
                return Err(io_error(error));
            }
            console::step(format!("apply: {}", options.selection.label()));
        }
        Command::Bootstrap
        | Command::Doctor { .. }
        | Command::New(_)
        | Command::RollbackGuide { .. } => {
            return Err(io_error(
                "bootstrap command is not valid for the versioned migration runner",
            ))
        }
    }

    for data_source in data_sources {
        let dialect = data_source.data_source_type.as_str();
        let schemas = loader::get_data_source_schemas(&data_source.name)?;
        let selected = selected_migrations(&manifest, &data_source, dialect, &schemas);

        if selected.is_empty() {
            let reason = if matches!(command, Command::Drift { .. })
                && matches!(
                    data_source.data_source_type,
                    DataSourceType::MongoDB
                        | DataSourceType::FabricSqlAnalytics
                        | DataSourceType::Snowflake
                        | DataSourceType::Neo4j
                        | DataSourceType::ServiceNow
                        | DataSourceType::Workday
                        | DataSourceType::Icims
                        | DataSourceType::Salesforce
                        | DataSourceType::Anaplan
                        | DataSourceType::OracleFinancials
                ) {
                "schema migration drift introspection is not implemented for this provider"
                    .to_string()
            } else if matches!(command, Command::Drift { .. }) {
                format!("no {dialect} migrations are available for schema drift comparison")
            } else {
                format!("no {dialect} migrations")
            };
            console::skip(&PathBuf::from(&data_source.name), reason.clone());
            report.data_sources.push(DataSourceMigrationReport::skipped(
                &data_source.name,
                dialect,
                reason,
            ));
            continue;
        }

        console::item("data source", format!("{} ({dialect})", &data_source.name));

        let selected_for_command = selected
            .iter()
            .filter(|migration| should_include(&command, migration))
            .cloned()
            .collect::<Vec<_>>();
        let lint_report =
            lint::lint_migrations(&migrations_root, &data_source.name, &selected_for_command)?;
        if matches!(
            command,
            Command::Plan { .. } | Command::Lint { .. } | Command::Apply(_)
        ) {
            lint::write_report(&lint_report)?;
            lint::print_report(&lint_report);
        }
        report.lint_reports.push(lint_report.clone());
        if matches!(command, Command::Apply(_)) {
            if let Err(error) = lint::fail_on_errors(&lint_report) {
                report.fail(&error.to_string());
                finish_json_report(&command, &report)?;
                return Err(error);
            }
        }

        if matches!(command, Command::Lint { .. }) {
            continue;
        }

        if matches!(command, Command::Drift { .. }) {
            match data_source.data_source_type {
                DataSourceType::PostgreSQL => {
                    let pg = crate::postgres::Postgres::new(&data_source)?;
                    let drift_report =
                        drift::check_postgres(&pg, &migrations_root, &selected).await?;
                    drift::write_report(&drift_report)?;
                    drift::print_report(&drift_report);
                    if let Err(error) = drift::fail_on_errors(&drift_report) {
                        report.drift_reports.push(drift_report);
                        report.fail(&error.to_string());
                        finish_json_report(&command, &report)?;
                        return Err(error);
                    }
                    report.drift_reports.push(drift_report);
                    report
                        .data_sources
                        .push(DataSourceMigrationReport::new(&data_source.name, dialect));
                }
                DataSourceType::MsSqlServer => {
                    let ms = crate::mssql::MsSql::new(&data_source).await?;
                    let drift_report = drift::check_mssql(&ms, &migrations_root, &selected).await?;
                    drift::write_report(&drift_report)?;
                    drift::print_report(&drift_report);
                    if let Err(error) = drift::fail_on_errors(&drift_report) {
                        report.drift_reports.push(drift_report);
                        report.fail(&error.to_string());
                        finish_json_report(&command, &report)?;
                        return Err(error);
                    }
                    report.drift_reports.push(drift_report);
                    report
                        .data_sources
                        .push(DataSourceMigrationReport::new(&data_source.name, dialect));
                }
                DataSourceType::MongoDB
                | DataSourceType::FabricSqlAnalytics
                | DataSourceType::Snowflake
                | DataSourceType::Neo4j
                | DataSourceType::ServiceNow
                | DataSourceType::Workday
                | DataSourceType::Icims
                | DataSourceType::Salesforce
                | DataSourceType::Anaplan
                | DataSourceType::OracleFinancials => {
                    console::skip(
                        &PathBuf::from(&data_source.name),
                        "schema migration drift introspection is not implemented for this provider",
                    );
                    report.data_sources.push(DataSourceMigrationReport::skipped(
                        &data_source.name,
                        dialect,
                        "schema migration drift introspection is not implemented for this provider",
                    ));
                }
            }
            continue;
        }

        match data_source.data_source_type {
            DataSourceType::PostgreSQL => {
                let pg = crate::postgres::Postgres::new(&data_source)?;
                match postgres::run(&pg, &migrations_root, &selected, &command).await {
                    Ok(provider_report) => {
                        report
                            .drift_reports
                            .extend(provider_report.drift_report.into_iter());
                        report.data_sources.push(provider_report.data_source);
                    }
                    Err(error) => {
                        report.fail(&error.to_string());
                        finish_json_report(&command, &report)?;
                        return Err(error);
                    }
                }
            }
            DataSourceType::MsSqlServer => {
                let ms = crate::mssql::MsSql::new(&data_source).await?;
                match mssql::run(&ms, &migrations_root, &selected, &command).await {
                    Ok(provider_report) => {
                        report
                            .drift_reports
                            .extend(provider_report.drift_report.into_iter());
                        report.data_sources.push(provider_report.data_source);
                    }
                    Err(error) => {
                        report.fail(&error.to_string());
                        finish_json_report(&command, &report)?;
                        return Err(error);
                    }
                }
            }
            DataSourceType::MongoDB
            | DataSourceType::FabricSqlAnalytics
            | DataSourceType::Snowflake
            | DataSourceType::Neo4j
            | DataSourceType::ServiceNow
            | DataSourceType::Workday
            | DataSourceType::Icims
            | DataSourceType::Salesforce
            | DataSourceType::Anaplan
            | DataSourceType::OracleFinancials => {
                console::skip(
                    &PathBuf::from(&data_source.name),
                    "versioned migration runner is not implemented for this data source type",
                );
                report.data_sources.push(DataSourceMigrationReport::skipped(
                    &data_source.name,
                    dialect,
                    "versioned migration runner is not implemented for this data source type",
                ));
            }
        }
    }

    console::done("versioned migrations");
    finish_json_report(&command, &report)?;
    Ok(())
}

pub fn migration_state(record: Option<&MigrationRecord>, checksum: &str) -> MigrationState {
    match record {
        None => MigrationState::Pending,
        Some(record) if record.checksum != checksum => MigrationState::ChecksumMismatch,
        Some(record) if record.status == "applied" => MigrationState::Applied,
        Some(_) => MigrationState::Failed,
    }
}

pub fn should_include(command: &Command, migration: &Migration) -> bool {
    match command {
        Command::Plan { selection, .. } => selection.includes(migration.phase),
        Command::Lint { selection, .. } => selection.includes(migration.phase),
        Command::Apply(options) => options.selection.includes(migration.phase),
        Command::Drift { .. } | Command::Status { .. } => true,
        Command::Bootstrap
        | Command::Doctor { .. }
        | Command::New(_)
        | Command::RollbackGuide { .. } => false,
    }
}

pub fn print_migration(migration: &Migration, state: MigrationState) {
    let description = migration
        .description
        .as_deref()
        .map(|value| format!(" - {value}"))
        .unwrap_or_default();
    console::step(format!(
        "{} {} [{}] {}{}",
        migration.id,
        migration.name,
        migration.phase.as_str(),
        state.label(),
        description
    ));
}

pub fn sql_literal(value: &str) -> String {
    value.replace('\'', "''")
}

pub fn io_error(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::Other, message.into())
}

fn parse_args(args: &[String]) -> Result<Command> {
    let (args, json) = split_json_flag(args);
    if args.is_empty() {
        return Ok(Command::Bootstrap);
    }

    match args[0].as_str() {
        "bootstrap" | "reconcile" => Ok(Command::Bootstrap),
        "doctor" => Ok(Command::Doctor { json }),
        "migrate" | "migrations" => parse_migrate_args(&args[1..], json),
        other => Err(io_error(format!(
            "unknown database command `{other}`; use `doctor`, `migrate new`, `migrate plan`, `migrate lint`, `migrate rollback-guide`, `migrate drift`, `migrate status`, `migrate apply`, or no args for bootstrap"
        ))),
    }
}

fn parse_migrate_args(args: &[String], json: bool) -> Result<Command> {
    if args.is_empty() {
        return Ok(Command::Apply(ApplyOptions {
            selection: PhaseSelection::Safe,
            confirm_contract: false,
            json,
        }));
    }

    match args[0].as_str() {
        "doctor" => Ok(Command::Doctor { json }),
        "new" => Ok(Command::New(scaffold::parse_new_options(&args[1..], json)?)),
        "plan" => Ok(Command::Plan {
            selection: parse_phase_selection(&args[1..])?.0,
            json,
        }),
        "lint" => Ok(Command::Lint {
            selection: parse_phase_selection(&args[1..])?.0,
            json,
        }),
        "rollback-guide" | "rollback-guidance" | "rollback" => Ok(Command::RollbackGuide { json }),
        "drift" => Ok(Command::Drift { json }),
        "status" => Ok(Command::Status { json }),
        "apply" => {
            let (selection, confirm_contract) = parse_phase_selection(&args[1..])?;
            Ok(Command::Apply(ApplyOptions {
                selection,
                confirm_contract,
                json,
            }))
        }
        other => Err(io_error(format!(
            "unknown migrate command `{other}`; use new, doctor, plan, lint, rollback-guide, drift, status, or apply"
        ))),
    }
}

fn parse_phase_selection(args: &[String]) -> Result<(PhaseSelection, bool)> {
    let mut selection = PhaseSelection::Safe;
    let mut confirm_contract = false;
    let mut index = 0usize;

    while index < args.len() {
        match args[index].as_str() {
            "--phase" => {
                index += 1;
                let phase = args.get(index).ok_or_else(|| {
                    io_error("--phase requires expand, backfill, contract, safe, or all")
                })?;
                selection = match phase.as_str() {
                    "safe" => PhaseSelection::Safe,
                    "expand" => PhaseSelection::One(Phase::Expand),
                    "backfill" => PhaseSelection::One(Phase::Backfill),
                    "contract" => PhaseSelection::One(Phase::Contract),
                    "all" => PhaseSelection::All,
                    other => {
                        return Err(io_error(format!(
                            "unknown migration phase `{other}`; expected expand, backfill, contract, safe, or all"
                        )))
                    }
                };
            }
            "--confirm-contract" => confirm_contract = true,
            "--json" => {}
            other => return Err(io_error(format!("unknown migration option `{other}`"))),
        }
        index += 1;
    }

    Ok((selection, confirm_contract))
}

fn load_manifest(migrations_root: &Path) -> Result<Manifest> {
    let manifest_path = migrations_root.join("manifest.yaml");
    let file = fs::File::open(&manifest_path)?;
    let reader = std::io::BufReader::new(file);
    serde_yaml::from_reader(reader)
        .map_err(|e| io_error(format!("could not read {:?}: {}", manifest_path, e)))
}

fn selected_migrations(
    manifest: &Manifest,
    data_source: &DataSource,
    dialect: &str,
    schemas: &[crate::data_source::Schema],
) -> Vec<Migration> {
    let mut migrations: Vec<Migration> = manifest
        .migrations
        .iter()
        .filter(|migration| {
            migration.data_source == data_source.name
                && migration.dialect == dialect
                && migration.schema.as_ref().map_or(true, |schema_name| {
                    schemas.iter().any(|schema| schema.name == *schema_name)
                })
        })
        .cloned()
        .collect();
    migrations.sort_by(|a, b| a.id.cmp(&b.id).then_with(|| a.name.cmp(&b.name)));
    migrations
}

fn split_json_flag(args: &[String]) -> (Vec<String>, bool) {
    let mut json = false;
    let mut filtered = Vec::new();
    for arg in args {
        if arg == "--json" {
            json = true;
        } else {
            filtered.push(arg.clone());
        }
    }
    (filtered, json)
}

fn selection_label(command: &Command) -> Option<String> {
    match command {
        Command::Plan { selection, .. } | Command::Lint { selection, .. } => {
            Some(selection.label().to_string())
        }
        Command::Apply(options) => Some(options.selection.label().to_string()),
        _ => None,
    }
}

fn rollback_guidance_report() -> RollbackGuidanceReport {
    RollbackGuidanceReport {
        command: "migrate rollback-guide",
        ok: true,
        policy: "Migrations are forward-only. Runtime rollback is app-first and database-forward.",
        phases: vec![
            RollbackPhaseGuidance {
                phase: "expand",
                rollback_strategy: "Leave added structures in place; roll back application code first.",
                operator_guidance: vec![
                    "Expanded nullable columns, tables, and indexes must remain backward-compatible.",
                    "Disable new application reads/writes before considering cleanup.",
                ],
            },
            RollbackPhaseGuidance {
                phase: "backfill",
                rollback_strategy: "Pause or resume by checkpoint; do not reverse data in place.",
                operator_guidance: vec![
                    "Backfills must be idempotent and chunked by stable keys.",
                    "Repair partial state with a new forward migration or rerun the resumable job.",
                ],
            },
            RollbackPhaseGuidance {
                phase: "contract",
                rollback_strategy: "No app rollback should depend on contracted structures.",
                operator_guidance: vec![
                    "Requires --confirm-contract and an approved safety window.",
                    "Destructive SQL should include an appfw: rollback-reviewed marker after review.",
                    "Use backups, snapshots, or provider-native recovery for emergency data restore.",
                ],
            },
        ],
        required_checks: vec![
            "scripts/appfw migrate plan --json",
            "scripts/appfw migrate lint --phase all --json",
            "scripts/appfw migrate drift --json",
            "scripts/appfw migrate apply --phase contract --confirm-contract --json",
        ],
    }
}

fn print_rollback_guidance(report: &RollbackGuidanceReport) {
    console::stage("migration rollback guidance");
    console::step(report.policy);
    for phase in &report.phases {
        console::item("phase", phase.phase);
        console::step(phase.rollback_strategy);
        for guidance in &phase.operator_guidance {
            console::step(*guidance);
        }
    }
    console::done("migration rollback guidance");
}

fn finish_json_report(command: &Command, report: &MigrationRunReport) -> Result<()> {
    if command.json_output() {
        print_json(report)?;
    }
    Ok(())
}

fn print_json(value: &impl Serialize) -> Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .map_err(|e| io_error(format!("could not serialize JSON report: {e}")))?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_drift_command_with_json_flag() {
        let args = vec!["drift".to_string()];
        let command = parse_migrate_args(&args, true).expect("drift command");
        assert!(matches!(command, Command::Drift { json: true }));
        assert_eq!(command.command_name(), "drift");
        assert!(command.json_output());
    }

    #[test]
    fn drift_command_includes_all_migrations_for_schema_introspection() {
        let command = Command::Drift { json: false };
        for phase in [Phase::Expand, Phase::Backfill, Phase::Contract] {
            let migration = Migration {
                id: "202605230000".to_string(),
                name: "contract".to_string(),
                schema: Some("crm".to_string()),
                data_source: "pg_primary".to_string(),
                dialect: "postgresql".to_string(),
                phase,
                path: PathBuf::from("postgresql/test.sql"),
                description: None,
            };
            assert!(should_include(&command, &migration));
        }
    }
}
