use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    app_workspace::AppWorkspace,
    utils::{artifacts, console, files},
};

pub const SYNC_DESCRIPTOR_REPORT: &str = "sync_descriptors.json";

const DATA_CLASSIFICATIONS: &[&str] = &[
    "public",
    "internal",
    "confidential",
    "restricted",
    "secret",
    "sensitive",
    "phi",
    "ephi",
];
const REGULATED_DATA_CLASSIFICATIONS: &[&str] = &[
    "confidential",
    "restricted",
    "secret",
    "sensitive",
    "phi",
    "ephi",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SyncDescriptor {
    pub name: Option<String>,
    pub source_schema: Option<String>,
    pub target_schema: Option<String>,
    pub mode: SyncMode,
    pub schedule: Option<String>,
    pub freshness_slo: Option<String>,
    pub objects: Vec<SyncObjectMap>,
    pub governance: Option<SyncGovernance>,
}

impl Default for SyncDescriptor {
    fn default() -> Self {
        Self {
            name: None,
            source_schema: None,
            target_schema: None,
            mode: SyncMode::Missing,
            schedule: None,
            freshness_slo: None,
            objects: vec![],
            governance: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SyncObjectMap {
    pub source: Option<String>,
    pub target: Option<String>,
    pub key: Option<String>,
    pub watermark: Option<String>,
    pub provenance: Option<SyncObjectProvenance>,
    pub echo_loop: Option<SyncObjectEchoLoop>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SyncObjectProvenance {
    pub projection_of: Option<String>,
    pub source_key_field: Option<String>,
    pub source_watermark_field: Option<String>,
    pub source_origin_field: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SyncObjectEchoLoop {
    pub policy: Option<String>,
    pub local_origin: Option<String>,
    pub source_origin_field: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SyncGovernance {
    pub classification: Option<String>,
    pub retention: Option<String>,
    pub deletion: SyncDeletionPolicy,
    pub source_drift: SyncSourceDriftPolicy,
}

impl Default for SyncGovernance {
    fn default() -> Self {
        Self {
            classification: None,
            retention: None,
            deletion: SyncDeletionPolicy::Missing,
            source_drift: SyncSourceDriftPolicy::Missing,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SyncMode {
    IncrementalWatermark,
    CdcEvent,
    #[default]
    Missing,
    Unknown(String),
}

impl Serialize for SyncMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match self {
            Self::IncrementalWatermark => "incremental_watermark",
            Self::CdcEvent => "cdc_event",
            Self::Missing => "missing",
            Self::Unknown(value) => value.as_str(),
        })
    }
}

impl<'de> Deserialize<'de> for SyncMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(match value.as_str() {
            "incremental_watermark" => Self::IncrementalWatermark,
            "cdc_event" => Self::CdcEvent,
            _ => Self::Unknown(value),
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SyncDeletionPolicy {
    Tombstone,
    HardDelete,
    Ignore,
    #[default]
    Missing,
    Unknown(String),
}

impl Serialize for SyncDeletionPolicy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match self {
            Self::Tombstone => "tombstone",
            Self::HardDelete => "hard_delete",
            Self::Ignore => "ignore",
            Self::Missing => "missing",
            Self::Unknown(value) => value.as_str(),
        })
    }
}

impl<'de> Deserialize<'de> for SyncDeletionPolicy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(match value.as_str() {
            "tombstone" => Self::Tombstone,
            "hard_delete" => Self::HardDelete,
            "ignore" => Self::Ignore,
            _ => Self::Unknown(value),
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SyncSourceDriftPolicy {
    FailClosed,
    Warn,
    Ignore,
    #[default]
    Missing,
    Unknown(String),
}

impl Serialize for SyncSourceDriftPolicy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match self {
            Self::FailClosed => "fail_closed",
            Self::Warn => "warn",
            Self::Ignore => "ignore",
            Self::Missing => "missing",
            Self::Unknown(value) => value.as_str(),
        })
    }
}

impl<'de> Deserialize<'de> for SyncSourceDriftPolicy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(match value.as_str() {
            "fail_closed" => Self::FailClosed,
            "warn" => Self::Warn,
            "ignore" => Self::Ignore,
            _ => Self::Unknown(value),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyncDescriptorIssue {
    pub severity: SyncIssueSeverity,
    pub code: &'static str,
    pub message: String,
    pub path: String,
    pub expected: String,
    pub actual: Option<String>,
    pub suggested_fix: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncIssueSeverity {
    Error,
}

pub fn parse_sync_descriptor_yaml(contents: &str) -> Result<SyncDescriptor, serde_yaml::Error> {
    serde_yaml::from_str(contents)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyncDescriptorSet {
    pub version: u32,
    pub generated_at: String,
    pub descriptor_count: usize,
    pub descriptors: Vec<NormalizedSyncDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NormalizedSyncDescriptor {
    pub file: String,
    pub name: String,
    pub source_schema: String,
    pub target_schema: String,
    pub mode: String,
    pub schedule: String,
    pub freshness_slo: String,
    pub objects: Vec<NormalizedSyncObjectMap>,
    pub governance: NormalizedSyncGovernance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NormalizedSyncObjectMap {
    pub source: String,
    pub target: String,
    pub key: String,
    pub watermark: String,
    pub provenance: NormalizedSyncObjectProvenance,
    pub echo_loop: NormalizedSyncObjectEchoLoop,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NormalizedSyncObjectProvenance {
    pub projection_of: String,
    pub source_key_field: String,
    pub source_watermark_field: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_origin_field: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NormalizedSyncObjectEchoLoop {
    pub policy: String,
    pub local_origin: String,
    pub source_origin_field: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NormalizedSyncGovernance {
    pub classification: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention: Option<String>,
    pub deletion: String,
    pub source_drift: String,
}

pub fn emit_sync_descriptor_report(workspace: &AppWorkspace) -> Result<SyncDescriptorSet> {
    let descriptor_set = load_sync_descriptor_set(&files::get_sync_descriptors_dir(workspace))?;
    let report_path = workspace.target_file(SYNC_DESCRIPTOR_REPORT);
    artifacts::safe_write(
        &report_path,
        serde_json::to_string_pretty(&descriptor_set)? + "\n",
    )
    .with_context(|| format!("could not write {}", report_path.display()))?;
    console::write(&report_path);
    Ok(descriptor_set)
}

pub fn load_sync_descriptor_set(sync_dir: &Path) -> Result<SyncDescriptorSet> {
    let mut descriptors = Vec::new();
    for file in sync_descriptor_files(sync_dir)? {
        let contents = fs::read_to_string(&file)
            .with_context(|| format!("read sync descriptor {}", file.display()))?;
        let descriptor = parse_sync_descriptor_yaml(&contents)
            .with_context(|| format!("parse sync descriptor {}", file.display()))?;
        let issues = validate_sync_descriptor(&descriptor);
        if !issues.is_empty() {
            anyhow::bail!(
                "sync descriptor {} has {} validation issue(s)",
                file.display(),
                issues.len()
            );
        }
        descriptors.push(normalize_descriptor(file, descriptor)?);
    }
    descriptors.sort_by(|left, right| left.name.cmp(&right.name).then(left.file.cmp(&right.file)));

    Ok(SyncDescriptorSet {
        version: 1,
        generated_at: chrono::offset::Utc::now().to_rfc3339(),
        descriptor_count: descriptors.len(),
        descriptors,
    })
}

fn sync_descriptor_files(sync_dir: &Path) -> Result<Vec<PathBuf>> {
    if !sync_dir.exists() {
        return Ok(vec![]);
    }

    let mut files = Vec::new();
    for entry in
        fs::read_dir(sync_dir).with_context(|| format!("read sync dir {}", sync_dir.display()))?
    {
        let path = entry?.path();
        if path.is_file()
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("yaml"))
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn normalize_descriptor(
    file: PathBuf,
    descriptor: SyncDescriptor,
) -> Result<NormalizedSyncDescriptor> {
    Ok(NormalizedSyncDescriptor {
        file: file.display().to_string(),
        name: required(descriptor.name, "name")?,
        source_schema: required(descriptor.source_schema, "source_schema")?,
        target_schema: required(descriptor.target_schema, "target_schema")?,
        mode: descriptor.mode.as_str().to_string(),
        schedule: required(descriptor.schedule, "schedule")?,
        freshness_slo: required(descriptor.freshness_slo, "freshness_slo")?,
        objects: descriptor
            .objects
            .into_iter()
            .map(normalize_object_map)
            .collect::<Result<Vec<_>>>()?,
        governance: normalize_governance(
            descriptor
                .governance
                .ok_or_else(|| anyhow::anyhow!("missing sync descriptor governance"))?,
        )?,
    })
}

fn normalize_object_map(object: SyncObjectMap) -> Result<NormalizedSyncObjectMap> {
    Ok(NormalizedSyncObjectMap {
        source: required(object.source, "objects[].source")?,
        target: required(object.target, "objects[].target")?,
        key: required(object.key, "objects[].key")?,
        watermark: required(object.watermark, "objects[].watermark")?,
        provenance: normalize_object_provenance(
            object
                .provenance
                .ok_or_else(|| anyhow::anyhow!("missing objects[].provenance"))?,
        )?,
        echo_loop: normalize_object_echo_loop(
            object
                .echo_loop
                .ok_or_else(|| anyhow::anyhow!("missing objects[].echo_loop"))?,
        )?,
    })
}

fn normalize_object_provenance(
    provenance: SyncObjectProvenance,
) -> Result<NormalizedSyncObjectProvenance> {
    Ok(NormalizedSyncObjectProvenance {
        projection_of: required(
            provenance.projection_of,
            "objects[].provenance.projection_of",
        )?,
        source_key_field: required(
            provenance.source_key_field,
            "objects[].provenance.source_key_field",
        )?,
        source_watermark_field: required(
            provenance.source_watermark_field,
            "objects[].provenance.source_watermark_field",
        )?,
        source_origin_field: optional_trimmed(provenance.source_origin_field),
    })
}

fn normalize_object_echo_loop(
    echo_loop: SyncObjectEchoLoop,
) -> Result<NormalizedSyncObjectEchoLoop> {
    Ok(NormalizedSyncObjectEchoLoop {
        policy: required(echo_loop.policy, "objects[].echo_loop.policy")?,
        local_origin: required(echo_loop.local_origin, "objects[].echo_loop.local_origin")?,
        source_origin_field: required(
            echo_loop.source_origin_field,
            "objects[].echo_loop.source_origin_field",
        )?,
    })
}

fn normalize_governance(governance: SyncGovernance) -> Result<NormalizedSyncGovernance> {
    Ok(NormalizedSyncGovernance {
        classification: required(governance.classification, "governance.classification")?,
        retention: governance.retention,
        deletion: governance.deletion.as_str().to_string(),
        source_drift: governance.source_drift.as_str().to_string(),
    })
}

fn required(value: Option<String>, field: &'static str) -> Result<String> {
    let Some(value) = value else {
        anyhow::bail!("missing required sync descriptor field `{field}`");
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        anyhow::bail!("empty required sync descriptor field `{field}`");
    }
    Ok(trimmed.to_string())
}

fn optional_trimmed(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn validate_sync_descriptor(descriptor: &SyncDescriptor) -> Vec<SyncDescriptorIssue> {
    let mut issues = vec![];

    require_non_empty(
        descriptor.name.as_deref(),
        "$.name",
        "missing_sync_name",
        "Sync descriptor is missing `name`.",
        "a non-empty stable sync descriptor name",
        "Add `name`, matching the sync descriptor file stem when possible.",
        &mut issues,
    );
    require_non_empty(
        descriptor.source_schema.as_deref(),
        "$.source_schema",
        "missing_source_schema",
        "Sync descriptor is missing `source_schema`.",
        "the source schema name under .appfw/model/schemas",
        "Set `source_schema` to the external SaaS projection schema.",
        &mut issues,
    );
    require_non_empty(
        descriptor.target_schema.as_deref(),
        "$.target_schema",
        "missing_target_schema",
        "Sync descriptor is missing `target_schema`.",
        "the target schema name under .appfw/model/schemas",
        "Set `target_schema` to the app-owned projection schema.",
        &mut issues,
    );
    validate_mode(&descriptor.mode, &mut issues);
    require_non_empty(
        descriptor.schedule.as_deref(),
        "$.schedule",
        "missing_schedule",
        "Sync descriptor is missing `schedule`.",
        "a non-empty schedule expression",
        "Add `schedule`, such as a cron expression owned by the sync worker.",
        &mut issues,
    );
    require_non_empty(
        descriptor.freshness_slo.as_deref(),
        "$.freshness_slo",
        "missing_freshness_slo",
        "Sync descriptor is missing `freshness_slo`.",
        "an ISO-8601 duration such as `PT10M`",
        "Add `freshness_slo` so generated operations can report sync lag.",
        &mut issues,
    );

    if descriptor.objects.is_empty() {
        issues.push(issue(
            "missing_sync_objects",
            "Sync descriptor must map at least one source object.",
            "$.objects",
            "one or more sync object mappings",
            None,
            "Add `objects` entries with source, target, key, and watermark.",
        ));
    }
    for (idx, object) in descriptor.objects.iter().enumerate() {
        validate_object_map(object, idx, &mut issues);
    }

    match &descriptor.governance {
        Some(governance) => validate_governance(governance, &mut issues),
        None => issues.push(issue(
            "missing_sync_governance",
            "Sync descriptor is missing governance metadata.",
            "$.governance",
            "classification, retention, deletion, and source_drift governance",
            None,
            "Add `governance` so sync workers can enforce classification, retention, deletion, and drift policy.",
        )),
    }

    issues
}

impl SyncMode {
    pub fn as_str(&self) -> &str {
        match self {
            Self::IncrementalWatermark => "incremental_watermark",
            Self::CdcEvent => "cdc_event",
            Self::Missing => "missing",
            Self::Unknown(value) => value.as_str(),
        }
    }
}

impl SyncDeletionPolicy {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Tombstone => "tombstone",
            Self::HardDelete => "hard_delete",
            Self::Ignore => "ignore",
            Self::Missing => "missing",
            Self::Unknown(value) => value.as_str(),
        }
    }
}

impl SyncSourceDriftPolicy {
    pub fn as_str(&self) -> &str {
        match self {
            Self::FailClosed => "fail_closed",
            Self::Warn => "warn",
            Self::Ignore => "ignore",
            Self::Missing => "missing",
            Self::Unknown(value) => value.as_str(),
        }
    }
}

fn validate_mode(mode: &SyncMode, issues: &mut Vec<SyncDescriptorIssue>) {
    match mode {
        SyncMode::IncrementalWatermark | SyncMode::CdcEvent => {}
        SyncMode::Missing => issues.push(issue(
            "missing_sync_mode",
            "Sync descriptor is missing `mode`.",
            "$.mode",
            allowed_values(&["incremental_watermark", "cdc_event"]),
            None,
            "Set `mode` to `incremental_watermark` for polling or `cdc_event` for Kafka-backed event projections.",
        )),
        SyncMode::Unknown(value) => issues.push(issue(
            "invalid_sync_mode",
            format!("Unsupported sync mode `{value}`."),
            "$.mode",
            allowed_values(&["incremental_watermark", "cdc_event"]),
            Some(value.clone()),
            "Use a supported sync mode; the model-level skeleton supports `incremental_watermark` and fail-closed `cdc_event` plans.",
        )),
    }
}

fn validate_object_map(object: &SyncObjectMap, idx: usize, issues: &mut Vec<SyncDescriptorIssue>) {
    let base = format!("$.objects[{idx}]");
    require_non_empty(
        object.source.as_deref(),
        &format!("{base}.source"),
        "missing_sync_object_source",
        "Sync object mapping is missing `source`.",
        "the source SaaS object name",
        "Set `source` to the external object read by the sync worker.",
        issues,
    );
    require_non_empty(
        object.target.as_deref(),
        &format!("{base}.target"),
        "missing_sync_object_target",
        "Sync object mapping is missing `target`.",
        "the target app entity name",
        "Set `target` to the projection entity written by the sync worker.",
        issues,
    );
    require_non_empty(
        object.key.as_deref(),
        &format!("{base}.key"),
        "missing_sync_object_key",
        "Sync object mapping is missing `key`.",
        "the stable source object key field",
        "Set `key` so generated sync code can upsert target records.",
        issues,
    );
    require_non_empty(
        object.watermark.as_deref(),
        &format!("{base}.watermark"),
        "missing_sync_object_watermark",
        "Sync object mapping is missing `watermark`.",
        "the source field used for incremental watermark polling",
        "Set `watermark` for incremental sync progress tracking.",
        issues,
    );
    match &object.provenance {
        Some(provenance) => validate_object_provenance(object, provenance, &base, issues),
        None => issues.push(issue(
            "missing_sync_object_provenance",
            "Sync object mapping is missing per-record provenance metadata.",
            format!("{base}.provenance"),
            "projection_of, source_key_field, source_watermark_field, and optional source_origin_field",
            None,
            "Add `provenance` so generated workers can stamp source lineage on each projected record.",
        )),
    }
    match &object.echo_loop {
        Some(echo_loop) => validate_object_echo_loop(echo_loop, &base, issues),
        None => issues.push(issue(
            "missing_sync_object_echo_loop",
            "Sync object mapping is missing echo-loop policy metadata.",
            format!("{base}.echo_loop"),
            "policy, local_origin, and source_origin_field",
            None,
            "Add `echo_loop` so generated workers can reject records emitted by this app before re-materializing them.",
        )),
    }
}

fn validate_object_provenance(
    object: &SyncObjectMap,
    provenance: &SyncObjectProvenance,
    base: &str,
    issues: &mut Vec<SyncDescriptorIssue>,
) {
    require_non_empty(
        provenance.projection_of.as_deref(),
        &format!("{base}.provenance.projection_of"),
        "missing_sync_projection_of",
        "Sync object provenance is missing `projection_of`.",
        "a stable source projection such as salesforce_crm_v1.Account",
        "Set `provenance.projection_of` to the source schema/object being materialized.",
        issues,
    );
    require_non_empty(
        provenance.source_key_field.as_deref(),
        &format!("{base}.provenance.source_key_field"),
        "missing_sync_source_key_field",
        "Sync object provenance is missing `source_key_field`.",
        "the same source field named by `key`",
        "Set `provenance.source_key_field` so per-record provenance can be tied to the idempotent upsert key.",
        issues,
    );
    require_non_empty(
        provenance.source_watermark_field.as_deref(),
        &format!("{base}.provenance.source_watermark_field"),
        "missing_sync_source_watermark_field",
        "Sync object provenance is missing `source_watermark_field`.",
        "the same source field named by `watermark`",
        "Set `provenance.source_watermark_field` so lineage records carry the incremental watermark basis.",
        issues,
    );
    if let (Some(key), Some(source_key)) = (
        object.key.as_deref().map(str::trim),
        provenance.source_key_field.as_deref().map(str::trim),
    ) {
        if !key.is_empty() && !source_key.is_empty() && key != source_key {
            issues.push(issue(
                "sync_source_key_field_mismatch",
                "Sync object provenance source key must match the object key.",
                format!("{base}.provenance.source_key_field"),
                format!("`{key}`"),
                Some(source_key.to_string()),
                "Keep `provenance.source_key_field` equal to `key` so upsert and lineage use one stable identifier.",
            ));
        }
    }
    if let (Some(watermark), Some(source_watermark)) = (
        object.watermark.as_deref().map(str::trim),
        provenance.source_watermark_field.as_deref().map(str::trim),
    ) {
        if !watermark.is_empty() && !source_watermark.is_empty() && watermark != source_watermark {
            issues.push(issue(
                "sync_source_watermark_field_mismatch",
                "Sync object provenance source watermark must match the object watermark.",
                format!("{base}.provenance.source_watermark_field"),
                format!("`{watermark}`"),
                Some(source_watermark.to_string()),
                "Keep `provenance.source_watermark_field` equal to `watermark` so freshness and lineage use one basis.",
            ));
        }
    }
}

fn validate_object_echo_loop(
    echo_loop: &SyncObjectEchoLoop,
    base: &str,
    issues: &mut Vec<SyncDescriptorIssue>,
) {
    require_non_empty(
        echo_loop.policy.as_deref(),
        &format!("{base}.echo_loop.policy"),
        "missing_sync_echo_loop_policy",
        "Sync object echo-loop policy is missing `policy`.",
        "reject_self_origin",
        "Set `echo_loop.policy: reject_self_origin` for materialized SaaS projections.",
        issues,
    );
    if let Some(policy) = echo_loop.policy.as_deref().map(str::trim) {
        if !policy.is_empty() && policy != "reject_self_origin" {
            issues.push(issue(
                "invalid_sync_echo_loop_policy",
                format!("Unsupported sync echo-loop policy `{policy}`."),
                format!("{base}.echo_loop.policy"),
                "reject_self_origin",
                Some(policy.to_string()),
                "Use `reject_self_origin`; ignoring self-origin records must be an explicit future contract.",
            ));
        }
    }
    require_non_empty(
        echo_loop.local_origin.as_deref(),
        &format!("{base}.echo_loop.local_origin"),
        "missing_sync_echo_loop_local_origin",
        "Sync object echo-loop policy is missing `local_origin`.",
        "a stable app origin marker such as appfw:pipeline:salesforce_to_pipeline",
        "Set `echo_loop.local_origin` to the value this app writes back to the SaaS origin marker.",
        issues,
    );
    require_non_empty(
        echo_loop.source_origin_field.as_deref(),
        &format!("{base}.echo_loop.source_origin_field"),
        "missing_sync_echo_loop_source_origin_field",
        "Sync object echo-loop policy is missing `source_origin_field`.",
        "the source field carrying the origin marker",
        "Set `echo_loop.source_origin_field` so generated workers can reject records emitted by this app.",
        issues,
    );
}

fn validate_governance(governance: &SyncGovernance, issues: &mut Vec<SyncDescriptorIssue>) {
    let classification = match governance.classification.as_deref() {
        Some(value) if value.trim().is_empty() => {
            issues.push(issue(
                "missing_data_classification",
                "Sync governance classification is empty.",
                "$.governance.classification",
                allowed_values(DATA_CLASSIFICATIONS),
                Some(value.to_string()),
                "Set `governance.classification` to a known data classification.",
            ));
            None
        }
        Some(value) if DATA_CLASSIFICATIONS.contains(&value) => Some(value),
        Some(value) => {
            issues.push(issue(
                "invalid_data_classification",
                format!("Unsupported sync governance classification `{value}`."),
                "$.governance.classification",
                allowed_values(DATA_CLASSIFICATIONS),
                Some(value.to_string()),
                "Use a known classification value; do not leave regulated sync data as unknown.",
            ));
            None
        }
        None => {
            issues.push(issue(
                "missing_data_classification",
                "Sync governance is missing `classification`.",
                "$.governance.classification",
                allowed_values(DATA_CLASSIFICATIONS),
                None,
                "Set `governance.classification` so sync data handling is explicit.",
            ));
            None
        }
    };

    let regulated = classification
        .map(|value| REGULATED_DATA_CLASSIFICATIONS.contains(&value))
        .unwrap_or(false);
    if regulated {
        require_non_empty(
            governance.retention.as_deref(),
            "$.governance.retention",
            "missing_retention_policy",
            "Regulated sync governance must declare `retention`.",
            "an ISO-8601 duration such as `P2Y`",
            "Add `governance.retention` for regulated sync data.",
            issues,
        );
        if matches!(governance.deletion, SyncDeletionPolicy::Missing) {
            issues.push(issue(
                "missing_deletion_policy",
                "Regulated sync governance must declare `deletion`.",
                "$.governance.deletion",
                allowed_values(&["tombstone", "hard_delete", "ignore"]),
                None,
                "Add `governance.deletion` for regulated sync data.",
            ));
        }
    }

    match &governance.deletion {
        SyncDeletionPolicy::Tombstone
        | SyncDeletionPolicy::HardDelete
        | SyncDeletionPolicy::Ignore => {}
        SyncDeletionPolicy::Missing => {
            if !regulated {
                issues.push(issue(
                    "missing_deletion_policy",
                    "Sync governance is missing `deletion`.",
                    "$.governance.deletion",
                    allowed_values(&["tombstone", "hard_delete", "ignore"]),
                    None,
                    "Set `governance.deletion` so source deletes have an explicit policy.",
                ));
            }
        }
        SyncDeletionPolicy::Unknown(value) => issues.push(issue(
            "invalid_deletion_policy",
            format!("Unsupported sync deletion policy `{value}`."),
            "$.governance.deletion",
            allowed_values(&["tombstone", "hard_delete", "ignore"]),
            Some(value.clone()),
            "Use a supported deletion policy.",
        )),
    }

    match &governance.source_drift {
        SyncSourceDriftPolicy::FailClosed
        | SyncSourceDriftPolicy::Warn
        | SyncSourceDriftPolicy::Ignore => {}
        SyncSourceDriftPolicy::Missing => issues.push(issue(
            "missing_source_drift_policy",
            "Sync governance is missing `source_drift`.",
            "$.governance.source_drift",
            allowed_values(&["fail_closed", "warn", "ignore"]),
            None,
            "Set `governance.source_drift` so schema drift behavior is explicit.",
        )),
        SyncSourceDriftPolicy::Unknown(value) => issues.push(issue(
            "invalid_source_drift_policy",
            format!("Unsupported sync source drift policy `{value}`."),
            "$.governance.source_drift",
            allowed_values(&["fail_closed", "warn", "ignore"]),
            Some(value.clone()),
            "Use a supported source drift policy.",
        )),
    }
}

fn require_non_empty(
    value: Option<&str>,
    path: &str,
    code: &'static str,
    message: &str,
    expected: impl Into<String>,
    suggested_fix: impl Into<String>,
    issues: &mut Vec<SyncDescriptorIssue>,
) {
    match value {
        Some(value) if !value.trim().is_empty() => {}
        Some(value) => issues.push(issue(
            code,
            message,
            path,
            expected,
            Some(value.to_string()),
            suggested_fix,
        )),
        None => issues.push(issue(code, message, path, expected, None, suggested_fix)),
    }
}

fn issue(
    code: &'static str,
    message: impl Into<String>,
    path: impl Into<String>,
    expected: impl Into<String>,
    actual: Option<String>,
    suggested_fix: impl Into<String>,
) -> SyncDescriptorIssue {
    SyncDescriptorIssue {
        severity: SyncIssueSeverity::Error,
        code,
        message: message.into(),
        path: path.into(),
        expected: expected.into(),
        actual,
        suggested_fix: suggested_fix.into(),
    }
}

fn allowed_values(values: &[&str]) -> String {
    format!("one of: {}", values.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, time::SystemTime};

    fn parse(contents: &str) -> SyncDescriptor {
        parse_sync_descriptor_yaml(contents).expect("descriptor should parse")
    }

    fn issue_codes(issues: &[SyncDescriptorIssue]) -> Vec<&'static str> {
        issues.iter().map(|issue| issue.code).collect()
    }

    fn temp_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("system time before epoch")
            .as_nanos();
        env::temp_dir().join(format!("appfw-sync-descriptor-{label}-{nanos}"))
    }

    const VALID_DESCRIPTOR: &str = r#"
name: salesforce_to_pipeline
source_schema: salesforce_crm_v1
target_schema: pipeline
mode: incremental_watermark
schedule: "*/5 * * * *"
freshness_slo: PT10M
objects:
- source: Opportunity
  target: Opportunity
  key: Id
  watermark: SystemModstamp
  provenance:
    projection_of: salesforce_crm_v1.Opportunity
    source_key_field: Id
    source_watermark_field: SystemModstamp
    source_origin_field: AppFrameworkOrigin__c
  echo_loop:
    policy: reject_self_origin
    local_origin: appfw:pipeline:salesforce_to_pipeline
    source_origin_field: AppFrameworkOrigin__c
- source: Account
  target: Account
  key: Id
  watermark: SystemModstamp
  provenance:
    projection_of: salesforce_crm_v1.Account
    source_key_field: Id
    source_watermark_field: SystemModstamp
    source_origin_field: AppFrameworkOrigin__c
  echo_loop:
    policy: reject_self_origin
    local_origin: appfw:pipeline:salesforce_to_pipeline
    source_origin_field: AppFrameworkOrigin__c
governance:
  classification: confidential
  retention: P2Y
  deletion: tombstone
  source_drift: fail_closed
"#;

    #[test]
    fn valid_descriptor_has_no_issues() {
        let descriptor = parse(VALID_DESCRIPTOR);

        assert_eq!(validate_sync_descriptor(&descriptor), vec![]);
    }

    #[test]
    fn missing_objects_is_reported() {
        let descriptor = parse(
            r#"
name: salesforce_to_pipeline
source_schema: salesforce_crm_v1
target_schema: pipeline
mode: incremental_watermark
schedule: "*/5 * * * *"
freshness_slo: PT10M
governance:
  classification: internal
  deletion: ignore
  source_drift: warn
"#,
        );

        let issues = validate_sync_descriptor(&descriptor);

        assert_eq!(issue_codes(&issues), vec!["missing_sync_objects"]);
        assert_eq!(issues[0].path, "$.objects");
    }

    #[test]
    fn invalid_mode_is_reported_by_validator() {
        let descriptor = parse(
            r#"
name: salesforce_to_pipeline
source_schema: salesforce_crm_v1
target_schema: pipeline
mode: full_refresh
schedule: "*/5 * * * *"
freshness_slo: PT10M
objects:
- source: Opportunity
  target: Opportunity
  key: Id
  watermark: SystemModstamp
  provenance:
    projection_of: salesforce_crm_v1.Opportunity
    source_key_field: Id
    source_watermark_field: SystemModstamp
    source_origin_field: AppFrameworkOrigin__c
  echo_loop:
    policy: reject_self_origin
    local_origin: appfw:pipeline:salesforce_to_pipeline
    source_origin_field: AppFrameworkOrigin__c
governance:
  classification: internal
  deletion: ignore
  source_drift: warn
"#,
        );

        let issues = validate_sync_descriptor(&descriptor);

        assert_eq!(issue_codes(&issues), vec!["invalid_sync_mode"]);
        assert_eq!(issues[0].actual.as_deref(), Some("full_refresh"));
    }

    #[test]
    fn regulated_governance_requires_retention_and_deletion() {
        let descriptor = parse(
            r#"
name: salesforce_to_pipeline
source_schema: salesforce_crm_v1
target_schema: pipeline
mode: incremental_watermark
schedule: "*/5 * * * *"
freshness_slo: PT10M
objects:
- source: Opportunity
  target: Opportunity
  key: Id
  watermark: SystemModstamp
  provenance:
    projection_of: salesforce_crm_v1.Opportunity
    source_key_field: Id
    source_watermark_field: SystemModstamp
    source_origin_field: AppFrameworkOrigin__c
  echo_loop:
    policy: reject_self_origin
    local_origin: appfw:pipeline:salesforce_to_pipeline
    source_origin_field: AppFrameworkOrigin__c
governance:
  classification: confidential
  source_drift: fail_closed
"#,
        );

        let issues = validate_sync_descriptor(&descriptor);

        assert_eq!(
            issue_codes(&issues),
            vec!["missing_retention_policy", "missing_deletion_policy"]
        );
    }

    #[test]
    fn load_sync_descriptor_set_normalizes_valid_descriptors() {
        let root = temp_root("load");
        fs::create_dir_all(&root).expect("create temp sync dir");
        fs::write(root.join("salesforce_to_pipeline.yaml"), VALID_DESCRIPTOR)
            .expect("write sync descriptor");

        let descriptor_set =
            load_sync_descriptor_set(&root).expect("sync descriptor set should load");

        assert_eq!(descriptor_set.descriptor_count, 1);
        let descriptor = &descriptor_set.descriptors[0];
        assert_eq!(descriptor.name, "salesforce_to_pipeline");
        assert_eq!(descriptor.source_schema, "salesforce_crm_v1");
        assert_eq!(descriptor.target_schema, "pipeline");
        assert_eq!(descriptor.mode, "incremental_watermark");
        assert_eq!(descriptor.objects.len(), 2);
        assert_eq!(
            descriptor.objects[0].provenance.projection_of,
            "salesforce_crm_v1.Opportunity"
        );
        assert_eq!(descriptor.objects[0].echo_loop.policy, "reject_self_origin");
        assert_eq!(descriptor.governance.classification, "confidential");
        assert_eq!(descriptor.governance.deletion, "tombstone");
        assert_eq!(descriptor.governance.source_drift, "fail_closed");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cdc_event_descriptor_normalizes_as_supported_mode() {
        let descriptor = parse(
            r#"
name: salesforce_cdc_to_pipeline
source_schema: salesforce_crm_v1
target_schema: pipeline
mode: cdc_event
schedule: continuous
freshness_slo: PT2M
objects:
- source: AccountChangeEvent
  target: Account
  key: ChangeEventHeader.recordIds[0]
  watermark: ChangeEventHeader.replayId
  provenance:
    projection_of: salesforce_crm_v1.Account
    source_key_field: ChangeEventHeader.recordIds[0]
    source_watermark_field: ChangeEventHeader.replayId
    source_origin_field: AppFrameworkOrigin__c
  echo_loop:
    policy: reject_self_origin
    local_origin: appfw:pipeline:salesforce_cdc_to_pipeline
    source_origin_field: AppFrameworkOrigin__c
governance:
  classification: confidential
  retention: P2Y
  deletion: tombstone
  source_drift: fail_closed
"#,
        );

        assert_eq!(validate_sync_descriptor(&descriptor), vec![]);
        assert_eq!(descriptor.mode.as_str(), "cdc_event");
    }

    #[test]
    fn absent_sync_descriptor_dir_loads_empty_set() {
        let root = temp_root("absent");

        let descriptor_set =
            load_sync_descriptor_set(&root).expect("absent sync dir should load as empty");

        assert_eq!(descriptor_set.descriptor_count, 0);
        assert!(descriptor_set.descriptors.is_empty());
    }
}
