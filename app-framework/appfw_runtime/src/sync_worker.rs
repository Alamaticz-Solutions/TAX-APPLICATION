//! Runtime-owned SaaS sync worker configuration contracts.
//!
//! This module deliberately stops at config loading and a disabled worker shell.
//! Provider calls, checkpoint storage, and projection writes are later
//! certification gates.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ConfigError, RuntimeAppError};

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
const DELETION_POLICIES: &[&str] = &["tombstone", "hard_delete", "ignore"];
const SOURCE_DRIFT_POLICIES: &[&str] = &["fail_closed", "warn", "ignore"];
const POISON_RECORD_POLICIES: &[&str] = &["record_and_continue"];
const SYNC_WORKER_MODES: &[&str] = &["incremental_watermark", "cdc_event"];
const CHECKPOINT_WATERMARK_MODES: &[&str] = &["incremental_watermark", "cdc_offset"];
const ECHO_LOOP_POLICIES: &[&str] = &["reject_self_origin"];

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeSyncWorkerConfig {
    pub version: u32,
    pub enabled: bool,
    #[serde(default)]
    pub activation_gates: Vec<String>,
    pub worker_count: usize,
    #[serde(default)]
    pub workers: Vec<RuntimeSyncWorker>,
}

impl RuntimeSyncWorkerConfig {
    pub fn from_yaml_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let contents = std::fs::read_to_string(path).map_err(|err| ConfigError::Io {
            path: path.display().to_string(),
            message: err.to_string(),
        })?;
        let config: Self = serde_yaml::from_str(&contents).map_err(|err| ConfigError::Parse {
            path: path.display().to_string(),
            message: err.to_string(),
        })?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.version != 1 {
            return Err(ConfigError::Load(format!(
                "sync worker config version must be 1, got {}",
                self.version
            )));
        }
        if self.worker_count != self.workers.len() {
            return Err(ConfigError::Load(format!(
                "sync worker config worker_count {} does not match workers length {}",
                self.worker_count,
                self.workers.len()
            )));
        }
        for gate in &self.activation_gates {
            ensure_present("sync worker activation gate", gate)?;
        }
        if self.enabled {
            let gate_summary = if self.activation_gates.is_empty() {
                "runtime worker code and provider certification are present".to_string()
            } else {
                self.activation_gates.join(", ")
            };
            return Err(ConfigError::Load(format!(
                "sync worker execution is not enabled until activation gates pass: {gate_summary}"
            )));
        }

        let mut names = BTreeSet::new();
        for worker in &self.workers {
            worker.validate()?;
            if !names.insert(worker.name.as_str()) {
                return Err(ConfigError::Load(format!(
                    "sync worker `{}` is configured more than once",
                    worker.name
                )));
            }
        }
        Ok(())
    }

    pub fn worker(&self, name: &str) -> Option<&RuntimeSyncWorker> {
        self.workers.iter().find(|worker| worker.name == name)
    }

    pub fn planned_worker_count(&self) -> usize {
        self.workers.len()
    }

    pub fn planned_object_count(&self) -> usize {
        self.workers.iter().map(|worker| worker.objects.len()).sum()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeSyncWorker {
    pub name: String,
    pub source_schema: String,
    pub target_schema: String,
    pub mode: String,
    pub schedule: String,
    pub freshness_slo: String,
    pub checkpoint: RuntimeSyncWorkerCheckpoint,
    pub readiness: RuntimeSyncWorkerReadiness,
    pub retry: RuntimeSyncWorkerRetryPolicy,
    pub dead_letter: RuntimeSyncWorkerDeadLetterPolicy,
    pub governance: RuntimeSyncWorkerGovernance,
    #[serde(default)]
    pub objects: Vec<RuntimeSyncObject>,
}

impl RuntimeSyncWorker {
    pub fn validate(&self) -> Result<(), ConfigError> {
        ensure_present("sync worker name", &self.name)?;
        ensure_present("sync worker source_schema", &self.source_schema)?;
        ensure_present("sync worker target_schema", &self.target_schema)?;
        ensure_present("sync worker mode", &self.mode)?;
        ensure_allowed("sync worker mode", &self.mode, SYNC_WORKER_MODES)?;
        ensure_present("sync worker schedule", &self.schedule)?;
        ensure_present("sync worker freshness_slo", &self.freshness_slo)?;
        if self.objects.is_empty() {
            return Err(ConfigError::Load(format!(
                "sync worker `{}` must include at least one object map",
                self.name
            )));
        }
        self.checkpoint
            .validate(&self.name, expected_checkpoint_mode(&self.mode))?;
        self.readiness.validate(&self.freshness_slo)?;
        self.retry.validate("sync worker retry")?;
        self.dead_letter.validate("sync worker dead_letter")?;
        self.governance.validate("sync worker governance")?;
        for object in &self.objects {
            object.validate(&self.name)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeSyncObject {
    pub source: String,
    pub target: String,
    pub key: String,
    pub watermark: String,
    pub provenance: RuntimeSyncObjectProvenance,
    pub echo_loop: RuntimeSyncObjectEchoLoopPolicy,
    pub upsert: RuntimeSyncWorkerUpsertPolicy,
}

impl RuntimeSyncObject {
    pub fn validate(&self, worker_name: &str) -> Result<(), ConfigError> {
        ensure_present("sync object source", &self.source)?;
        ensure_present("sync object target", &self.target)?;
        ensure_present("sync object key", &self.key)?;
        ensure_present("sync object watermark", &self.watermark)?;
        self.provenance.validate(self)?;
        self.echo_loop.validate()?;
        if self
            .provenance
            .source_origin_field
            .as_deref()
            .is_some_and(|field| field != self.echo_loop.source_origin_field)
        {
            return Err(ConfigError::Load(format!(
                "sync worker `{worker_name}` object `{}` provenance source_origin_field must match echo_loop source_origin_field",
                self.source
            )));
        }
        self.upsert.validate()?;
        if self.upsert.stable_source_key != self.key {
            return Err(ConfigError::Load(format!(
                "sync worker `{worker_name}` object `{}` upsert stable_source_key must match key `{}`",
                self.source, self.key
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeSyncObjectProvenance {
    pub projection_of: String,
    pub source_key_field: String,
    pub source_watermark_field: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_origin_field: Option<String>,
}

impl RuntimeSyncObjectProvenance {
    pub fn validate(&self, object: &RuntimeSyncObject) -> Result<(), ConfigError> {
        ensure_present("sync object provenance projection_of", &self.projection_of)?;
        ensure_present(
            "sync object provenance source_key_field",
            &self.source_key_field,
        )?;
        ensure_present(
            "sync object provenance source_watermark_field",
            &self.source_watermark_field,
        )?;
        if self.source_key_field != object.key {
            return Err(ConfigError::Load(format!(
                "sync object `{}` provenance source_key_field must match key `{}`",
                object.source, object.key
            )));
        }
        if self.source_watermark_field != object.watermark {
            return Err(ConfigError::Load(format!(
                "sync object `{}` provenance source_watermark_field must match watermark `{}`",
                object.source, object.watermark
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeSyncObjectEchoLoopPolicy {
    pub policy: String,
    pub local_origin: String,
    pub source_origin_field: String,
}

impl RuntimeSyncObjectEchoLoopPolicy {
    pub fn validate(&self) -> Result<(), ConfigError> {
        ensure_present("sync object echo_loop policy", &self.policy)?;
        ensure_allowed(
            "sync object echo_loop policy",
            &self.policy,
            ECHO_LOOP_POLICIES,
        )?;
        ensure_present("sync object echo_loop local_origin", &self.local_origin)?;
        ensure_present(
            "sync object echo_loop source_origin_field",
            &self.source_origin_field,
        )?;
        Ok(())
    }

    pub fn is_self_origin(&self, source_origin: Option<&str>) -> bool {
        self.policy == "reject_self_origin"
            && source_origin
                .map(str::trim)
                .is_some_and(|origin| origin == self.local_origin)
    }

    pub fn rejects_source_record(&self, source_record: &Value) -> bool {
        let Some(object) = source_record.as_object() else {
            return false;
        };
        self.is_self_origin(
            object
                .get(&self.source_origin_field)
                .and_then(Value::as_str),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeSyncRecordProvenance {
    pub sync_worker: String,
    pub projection_of: String,
    pub source_object: String,
    pub source_key_field: String,
    pub source_key_value: String,
    pub source_watermark_field: String,
    pub source_watermark_value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_origin_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_origin_value: Option<String>,
}

impl RuntimeSyncObject {
    pub fn record_provenance(
        &self,
        worker_name: impl Into<String>,
        source_key_value: impl Into<String>,
        source_watermark_value: impl Into<String>,
        source_origin_value: Option<String>,
    ) -> RuntimeSyncRecordProvenance {
        RuntimeSyncRecordProvenance {
            sync_worker: worker_name.into(),
            projection_of: self.provenance.projection_of.clone(),
            source_object: self.source.clone(),
            source_key_field: self.provenance.source_key_field.clone(),
            source_key_value: source_key_value.into(),
            source_watermark_field: self.provenance.source_watermark_field.clone(),
            source_watermark_value: source_watermark_value.into(),
            source_origin_field: self.provenance.source_origin_field.clone(),
            source_origin_value,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeSyncWorkerCheckpoint {
    pub store: String,
    pub scope: String,
    pub watermark_mode: String,
}

impl RuntimeSyncWorkerCheckpoint {
    pub fn validate(
        &self,
        worker_name: &str,
        expected_watermark_mode: &str,
    ) -> Result<(), ConfigError> {
        ensure_present("sync worker checkpoint store", &self.store)?;
        ensure_present("sync worker checkpoint scope", &self.scope)?;
        ensure_present(
            "sync worker checkpoint watermark_mode",
            &self.watermark_mode,
        )?;
        if self.scope != worker_name {
            return Err(ConfigError::Load(format!(
                "sync worker `{worker_name}` checkpoint scope must match worker name"
            )));
        }
        ensure_allowed(
            "sync worker checkpoint watermark_mode",
            &self.watermark_mode,
            CHECKPOINT_WATERMARK_MODES,
        )?;
        if self.watermark_mode != expected_watermark_mode {
            return Err(ConfigError::Load(format!(
                "sync worker `{worker_name}` checkpoint watermark_mode `{}` must match mode `{}`",
                self.watermark_mode, expected_watermark_mode
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeSyncWorkerReadiness {
    pub freshness_slo: String,
    pub max_consecutive_failures: u32,
}

impl RuntimeSyncWorkerReadiness {
    pub fn validate(&self, worker_freshness_slo: &str) -> Result<(), ConfigError> {
        ensure_present("sync worker readiness freshness_slo", &self.freshness_slo)?;
        if self.freshness_slo != worker_freshness_slo {
            return Err(ConfigError::Load(
                "sync worker readiness freshness_slo must match worker freshness_slo".to_string(),
            ));
        }
        if self.max_consecutive_failures == 0 {
            return Err(ConfigError::Load(
                "sync worker readiness max_consecutive_failures must be greater than 0".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeSyncWorkerRetryPolicy {
    pub enabled: bool,
    pub max_attempts: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
}

impl RuntimeSyncWorkerRetryPolicy {
    pub fn validate(&self, path: &str) -> Result<(), ConfigError> {
        if !self.enabled {
            return Ok(());
        }
        if self.max_attempts < 2 {
            return Err(ConfigError::Load(format!(
                "{path}.max_attempts must be at least 2 when retry is enabled"
            )));
        }
        if self.initial_backoff_ms == 0 {
            return Err(ConfigError::Load(format!(
                "{path}.initial_backoff_ms must be greater than 0"
            )));
        }
        if self.max_backoff_ms < self.initial_backoff_ms {
            return Err(ConfigError::Load(format!(
                "{path}.max_backoff_ms must be greater than or equal to initial_backoff_ms"
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeSyncWorkerDeadLetterPolicy {
    pub enabled: bool,
    pub store: String,
    pub poison_record_policy: String,
}

impl RuntimeSyncWorkerDeadLetterPolicy {
    pub fn validate(&self, path: &str) -> Result<(), ConfigError> {
        if !self.enabled {
            return Ok(());
        }
        ensure_present(&format!("{path}.store"), &self.store)?;
        ensure_present(
            &format!("{path}.poison_record_policy"),
            &self.poison_record_policy,
        )?;
        ensure_allowed(
            &format!("{path}.poison_record_policy"),
            &self.poison_record_policy,
            POISON_RECORD_POLICIES,
        )?;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeSyncWorkerGovernance {
    pub classification: String,
    pub retention: Option<String>,
    pub deletion: String,
    pub source_drift: String,
}

impl RuntimeSyncWorkerGovernance {
    pub fn validate(&self, path: &str) -> Result<(), ConfigError> {
        ensure_present(&format!("{path}.classification"), &self.classification)?;
        ensure_present(&format!("{path}.deletion"), &self.deletion)?;
        ensure_present(&format!("{path}.source_drift"), &self.source_drift)?;
        ensure_allowed(
            &format!("{path}.classification"),
            &self.classification,
            DATA_CLASSIFICATIONS,
        )?;
        ensure_allowed(
            &format!("{path}.deletion"),
            &self.deletion,
            DELETION_POLICIES,
        )?;
        ensure_allowed(
            &format!("{path}.source_drift"),
            &self.source_drift,
            SOURCE_DRIFT_POLICIES,
        )?;
        if REGULATED_DATA_CLASSIFICATIONS.contains(&self.classification.as_str())
            && self
                .retention
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
        {
            return Err(ConfigError::Load(format!(
                "{path}.retention is required for regulated classifications"
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeSyncWorkerUpsertPolicy {
    pub strategy: String,
    pub stable_source_key: String,
}

impl RuntimeSyncWorkerUpsertPolicy {
    pub fn validate(&self) -> Result<(), ConfigError> {
        ensure_present("sync worker upsert strategy", &self.strategy)?;
        if self.strategy != "idempotent_source_key" {
            return Err(ConfigError::Load(format!(
                "sync worker upsert strategy `{}` is unsupported",
                self.strategy
            )));
        }
        ensure_present(
            "sync worker upsert stable_source_key",
            &self.stable_source_key,
        )?;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeSyncWorkerShellReport {
    pub enabled: bool,
    pub activation_gates: Vec<String>,
    pub workers_planned: usize,
    pub objects_planned: usize,
    pub workers_started: usize,
}

pub async fn run_sync_worker_shell(
    config: &RuntimeSyncWorkerConfig,
) -> Result<RuntimeSyncWorkerShellReport, RuntimeAppError> {
    config.validate().map_err(RuntimeAppError::Config)?;
    Ok(RuntimeSyncWorkerShellReport {
        enabled: false,
        activation_gates: config.activation_gates.clone(),
        workers_planned: config.planned_worker_count(),
        objects_planned: config.planned_object_count(),
        workers_started: 0,
    })
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeSyncCheckpointState {
    pub store: String,
    pub scope: String,
    pub watermark_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watermark: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeSyncProjectionWrite {
    pub target: String,
    pub source_key: String,
    pub source_watermark: String,
    pub provenance: RuntimeSyncRecordProvenance,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeSyncDeadLetterRecord {
    pub source_object: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_key: Option<String>,
    pub reason: String,
    pub payload_summary: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeSyncWorkerObjectExecutionReport {
    pub source_object: String,
    pub target_entity: String,
    pub records_seen: usize,
    pub records_projected: usize,
    pub records_suppressed: usize,
    pub records_skipped_by_checkpoint: usize,
    pub dead_letters: usize,
    pub duplicate_source_keys: usize,
    pub projection_upserts_applied: usize,
    pub projection_upserts_skipped: usize,
    pub checkpoint: RuntimeSyncCheckpointState,
    pub projection_writes: Vec<RuntimeSyncProjectionWrite>,
    pub dead_letter_records: Vec<RuntimeSyncDeadLetterRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeSyncWorkerExecutionReport {
    pub worker: String,
    pub mode: String,
    pub source_schema: String,
    pub target_schema: String,
    pub provider_live_certified: bool,
    pub records_seen: usize,
    pub records_projected: usize,
    pub records_suppressed: usize,
    pub records_skipped_by_checkpoint: usize,
    pub dead_letters: usize,
    pub projection_upserts_applied: usize,
    pub projection_upserts_skipped: usize,
    pub object_reports: Vec<RuntimeSyncWorkerObjectExecutionReport>,
}

pub trait RuntimeSyncCheckpointStore {
    fn load_checkpoint(
        &self,
        worker: &RuntimeSyncWorker,
        object: &RuntimeSyncObject,
    ) -> Result<Option<String>, RuntimeAppError>;

    fn save_checkpoint(
        &mut self,
        worker: &RuntimeSyncWorker,
        object: &RuntimeSyncObject,
        checkpoint: &RuntimeSyncCheckpointState,
    ) -> Result<(), RuntimeAppError>;
}

pub trait RuntimeSyncProjectionStore {
    fn upsert_projection(
        &mut self,
        write: &RuntimeSyncProjectionWrite,
    ) -> Result<RuntimeSyncProjectionUpsertOutcome, RuntimeAppError>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeSyncInMemoryCheckpointStore {
    checkpoints: BTreeMap<String, String>,
}

impl RuntimeSyncInMemoryCheckpointStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seed_checkpoint(
        &mut self,
        worker: &RuntimeSyncWorker,
        object: &RuntimeSyncObject,
        watermark: impl Into<String>,
    ) {
        self.checkpoints
            .insert(sync_checkpoint_key(worker, object), watermark.into());
    }

    pub fn checkpoint(
        &self,
        worker: &RuntimeSyncWorker,
        object: &RuntimeSyncObject,
    ) -> Option<&str> {
        self.checkpoints
            .get(&sync_checkpoint_key(worker, object))
            .map(String::as_str)
    }
}

impl RuntimeSyncCheckpointStore for RuntimeSyncInMemoryCheckpointStore {
    fn load_checkpoint(
        &self,
        worker: &RuntimeSyncWorker,
        object: &RuntimeSyncObject,
    ) -> Result<Option<String>, RuntimeAppError> {
        Ok(self
            .checkpoints
            .get(&sync_checkpoint_key(worker, object))
            .cloned())
    }

    fn save_checkpoint(
        &mut self,
        worker: &RuntimeSyncWorker,
        object: &RuntimeSyncObject,
        checkpoint: &RuntimeSyncCheckpointState,
    ) -> Result<(), RuntimeAppError> {
        let Some(watermark) = checkpoint.watermark.as_ref() else {
            return Ok(());
        };
        self.checkpoints
            .insert(sync_checkpoint_key(worker, object), watermark.clone());
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeSyncInMemoryProjectionStore {
    records: BTreeMap<String, RuntimeSyncStoredProjectionRecord>,
}

impl RuntimeSyncInMemoryProjectionStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(
        &self,
        target: impl AsRef<str>,
        source_key: impl AsRef<str>,
    ) -> Option<&RuntimeSyncStoredProjectionRecord> {
        self.records
            .get(&sync_projection_key(target.as_ref(), source_key.as_ref()))
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

impl RuntimeSyncProjectionStore for RuntimeSyncInMemoryProjectionStore {
    fn upsert_projection(
        &mut self,
        write: &RuntimeSyncProjectionWrite,
    ) -> Result<RuntimeSyncProjectionUpsertOutcome, RuntimeAppError> {
        let key = sync_projection_key(&write.target, &write.source_key);
        match self.records.get_mut(&key) {
            Some(existing)
                if sync_watermark_cmp_value(&write.source_watermark)
                    <= sync_watermark_cmp_value(&existing.source_watermark) =>
            {
                Ok(RuntimeSyncProjectionUpsertOutcome {
                    inserted: false,
                    updated: false,
                    skipped_existing_watermark: true,
                })
            }
            Some(existing) => {
                existing.source_watermark = write.source_watermark.clone();
                existing.provenance = write.provenance.clone();
                Ok(RuntimeSyncProjectionUpsertOutcome {
                    inserted: false,
                    updated: true,
                    skipped_existing_watermark: false,
                })
            }
            None => {
                self.records.insert(
                    key,
                    RuntimeSyncStoredProjectionRecord {
                        target: write.target.clone(),
                        source_key: write.source_key.clone(),
                        source_watermark: write.source_watermark.clone(),
                        provenance: write.provenance.clone(),
                    },
                );
                Ok(RuntimeSyncProjectionUpsertOutcome {
                    inserted: true,
                    updated: false,
                    skipped_existing_watermark: false,
                })
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeSyncStoredProjectionRecord {
    pub target: String,
    pub source_key: String,
    pub source_watermark: String,
    pub provenance: RuntimeSyncRecordProvenance,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeSyncProjectionUpsertOutcome {
    pub inserted: bool,
    pub updated: bool,
    pub skipped_existing_watermark: bool,
}

pub fn run_sync_worker_local_fixture(
    worker: &RuntimeSyncWorker,
    source_records: &BTreeMap<String, Vec<Value>>,
) -> Result<RuntimeSyncWorkerExecutionReport, RuntimeAppError> {
    worker.validate().map_err(RuntimeAppError::Config)?;

    let mut object_reports = Vec::with_capacity(worker.objects.len());
    for object in &worker.objects {
        let records = source_records
            .get(&object.source)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        object_reports.push(run_sync_object_local_fixture(worker, object, records)?);
    }

    Ok(RuntimeSyncWorkerExecutionReport {
        worker: worker.name.clone(),
        mode: worker.mode.clone(),
        source_schema: worker.source_schema.clone(),
        target_schema: worker.target_schema.clone(),
        provider_live_certified: false,
        records_seen: object_reports
            .iter()
            .map(|report| report.records_seen)
            .sum(),
        records_projected: object_reports
            .iter()
            .map(|report| report.records_projected)
            .sum(),
        records_suppressed: object_reports
            .iter()
            .map(|report| report.records_suppressed)
            .sum(),
        records_skipped_by_checkpoint: object_reports
            .iter()
            .map(|report| report.records_skipped_by_checkpoint)
            .sum(),
        dead_letters: object_reports
            .iter()
            .map(|report| report.dead_letters)
            .sum(),
        projection_upserts_applied: object_reports
            .iter()
            .map(|report| report.projection_upserts_applied)
            .sum(),
        projection_upserts_skipped: object_reports
            .iter()
            .map(|report| report.projection_upserts_skipped)
            .sum(),
        object_reports,
    })
}

pub fn run_sync_worker_local_fixture_with_stores(
    worker: &RuntimeSyncWorker,
    source_records: &BTreeMap<String, Vec<Value>>,
    checkpoint_store: &mut dyn RuntimeSyncCheckpointStore,
    projection_store: &mut dyn RuntimeSyncProjectionStore,
) -> Result<RuntimeSyncWorkerExecutionReport, RuntimeAppError> {
    worker.validate().map_err(RuntimeAppError::Config)?;

    let mut object_reports = Vec::with_capacity(worker.objects.len());
    for object in &worker.objects {
        let records = source_records
            .get(&object.source)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let initial_checkpoint = checkpoint_store.load_checkpoint(worker, object)?;
        let report = run_sync_object_local_fixture_with_checkpoint(
            worker,
            object,
            records,
            initial_checkpoint.as_deref(),
            Some(&mut *projection_store),
        )?;
        checkpoint_store.save_checkpoint(worker, object, &report.checkpoint)?;
        object_reports.push(report);
    }

    Ok(RuntimeSyncWorkerExecutionReport {
        worker: worker.name.clone(),
        mode: worker.mode.clone(),
        source_schema: worker.source_schema.clone(),
        target_schema: worker.target_schema.clone(),
        provider_live_certified: false,
        records_seen: object_reports
            .iter()
            .map(|report| report.records_seen)
            .sum(),
        records_projected: object_reports
            .iter()
            .map(|report| report.records_projected)
            .sum(),
        records_suppressed: object_reports
            .iter()
            .map(|report| report.records_suppressed)
            .sum(),
        records_skipped_by_checkpoint: object_reports
            .iter()
            .map(|report| report.records_skipped_by_checkpoint)
            .sum(),
        dead_letters: object_reports
            .iter()
            .map(|report| report.dead_letters)
            .sum(),
        projection_upserts_applied: object_reports
            .iter()
            .map(|report| report.projection_upserts_applied)
            .sum(),
        projection_upserts_skipped: object_reports
            .iter()
            .map(|report| report.projection_upserts_skipped)
            .sum(),
        object_reports,
    })
}

fn run_sync_object_local_fixture(
    worker: &RuntimeSyncWorker,
    object: &RuntimeSyncObject,
    records: &[Value],
) -> Result<RuntimeSyncWorkerObjectExecutionReport, RuntimeAppError> {
    run_sync_object_local_fixture_with_checkpoint(worker, object, records, None, None)
}

fn run_sync_object_local_fixture_with_checkpoint(
    worker: &RuntimeSyncWorker,
    object: &RuntimeSyncObject,
    records: &[Value],
    initial_checkpoint: Option<&str>,
    mut projection_store: Option<&mut dyn RuntimeSyncProjectionStore>,
) -> Result<RuntimeSyncWorkerObjectExecutionReport, RuntimeAppError> {
    let mut seen_keys = BTreeSet::new();
    let mut projection_writes = Vec::new();
    let mut dead_letter_records = Vec::new();
    let mut records_suppressed = 0;
    let mut records_skipped_by_checkpoint = 0;
    let mut duplicate_source_keys = 0;
    let mut projection_upserts_applied = 0;
    let mut projection_upserts_skipped = 0;
    let mut checkpoint_watermark = initial_checkpoint.map(str::to_string);

    for record in records {
        if object.echo_loop.rejects_source_record(record) {
            records_suppressed += 1;
            continue;
        }

        let Some(source_key) = sync_field_value_as_string(record, &object.key) else {
            push_or_fail_dead_letter(
                worker,
                object,
                &mut dead_letter_records,
                record,
                None,
                format!("missing required source key `{}`", object.key),
            )?;
            continue;
        };
        let Some(source_watermark) = sync_field_value_as_string(record, &object.watermark) else {
            push_or_fail_dead_letter(
                worker,
                object,
                &mut dead_letter_records,
                record,
                Some(source_key),
                format!("missing required source watermark `{}`", object.watermark),
            )?;
            continue;
        };

        if initial_checkpoint
            .is_some_and(|checkpoint| !sync_watermark_is_after(&source_watermark, checkpoint))
        {
            records_skipped_by_checkpoint += 1;
            continue;
        }

        if !seen_keys.insert(source_key.clone()) {
            duplicate_source_keys += 1;
        }

        let source_origin = object
            .provenance
            .source_origin_field
            .as_deref()
            .and_then(|field| sync_field_value_as_string(record, field));
        let provenance =
            object.record_provenance(&worker.name, &source_key, &source_watermark, source_origin);

        advance_checkpoint_watermark(&mut checkpoint_watermark, &source_watermark);
        projection_writes.push(RuntimeSyncProjectionWrite {
            target: object.target.clone(),
            source_key,
            source_watermark,
            provenance,
        });
        if let Some(store) = projection_store.as_deref_mut() {
            let outcome = store.upsert_projection(
                projection_writes
                    .last()
                    .expect("projection write was just pushed"),
            )?;
            if outcome.inserted || outcome.updated {
                projection_upserts_applied += 1;
            } else if outcome.skipped_existing_watermark {
                projection_upserts_skipped += 1;
            }
        }
    }

    Ok(RuntimeSyncWorkerObjectExecutionReport {
        source_object: object.source.clone(),
        target_entity: object.target.clone(),
        records_seen: records.len(),
        records_projected: projection_writes.len(),
        records_suppressed,
        records_skipped_by_checkpoint,
        dead_letters: dead_letter_records.len(),
        duplicate_source_keys,
        projection_upserts_applied,
        projection_upserts_skipped,
        checkpoint: RuntimeSyncCheckpointState {
            store: worker.checkpoint.store.clone(),
            scope: worker.checkpoint.scope.clone(),
            watermark_mode: worker.checkpoint.watermark_mode.clone(),
            watermark: checkpoint_watermark,
        },
        projection_writes,
        dead_letter_records,
    })
}

fn sync_checkpoint_key(worker: &RuntimeSyncWorker, object: &RuntimeSyncObject) -> String {
    format!(
        "{}:{}:{}:{}",
        worker.checkpoint.store, worker.checkpoint.scope, object.source, object.target
    )
}

fn sync_projection_key(target: &str, source_key: &str) -> String {
    format!("{target}:{source_key}")
}

fn advance_checkpoint_watermark(current: &mut Option<String>, candidate: &str) {
    let should_advance = match current.as_deref() {
        Some(existing) => sync_watermark_is_after(candidate, existing),
        None => true,
    };
    if should_advance {
        *current = Some(candidate.to_string());
    }
}

fn sync_watermark_is_after(candidate: &str, existing: &str) -> bool {
    sync_watermark_cmp_value(candidate) > sync_watermark_cmp_value(existing)
}

fn sync_watermark_cmp_value(value: &str) -> SyncWatermarkCmpValue<'_> {
    match value.parse::<u128>() {
        Ok(number) => SyncWatermarkCmpValue::Numeric(number),
        Err(_) => SyncWatermarkCmpValue::Text(value),
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum SyncWatermarkCmpValue<'a> {
    Numeric(u128),
    Text(&'a str),
}

fn push_or_fail_dead_letter(
    worker: &RuntimeSyncWorker,
    object: &RuntimeSyncObject,
    dead_letter_records: &mut Vec<RuntimeSyncDeadLetterRecord>,
    record: &Value,
    source_key: Option<String>,
    reason: String,
) -> Result<(), RuntimeAppError> {
    if !worker.dead_letter.enabled {
        return Err(RuntimeAppError::Validation(format!(
            "sync worker `{}` object `{}` cannot process poison record: {reason}",
            worker.name, object.source
        )));
    }

    dead_letter_records.push(RuntimeSyncDeadLetterRecord {
        source_object: object.source.clone(),
        source_key,
        reason,
        payload_summary: summarize_sync_payload(record),
    });
    Ok(())
}

fn sync_field_value_as_string(record: &Value, field: &str) -> Option<String> {
    let value = record.as_object()?.get(field)?;
    match value {
        Value::String(value) if !value.trim().is_empty() => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn summarize_sync_payload(record: &Value) -> String {
    match record.as_object() {
        Some(object) => {
            let mut keys = object.keys().take(5).cloned().collect::<Vec<_>>();
            keys.sort();
            format!("object keys: {}", keys.join(", "))
        }
        None => "non-object payload".to_string(),
    }
}

fn ensure_present(label: &str, value: &str) -> Result<(), ConfigError> {
    if value.trim().is_empty() {
        Err(ConfigError::Load(format!("{label} is required")))
    } else {
        Ok(())
    }
}

fn ensure_allowed(label: &str, value: &str, allowed: &[&str]) -> Result<(), ConfigError> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(ConfigError::Load(format!(
            "{label} must be one of {}",
            allowed.join(", ")
        )))
    }
}

fn expected_checkpoint_mode(worker_mode: &str) -> &'static str {
    match worker_mode {
        "cdc_event" => "cdc_offset",
        _ => "incremental_watermark",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn worker() -> RuntimeSyncWorker {
        RuntimeSyncWorker {
            name: "salesforce_to_pipeline".to_string(),
            source_schema: "salesforce_crm_v1".to_string(),
            target_schema: "pipeline".to_string(),
            mode: "incremental_watermark".to_string(),
            schedule: "*/5 * * * *".to_string(),
            freshness_slo: "PT10M".to_string(),
            checkpoint: RuntimeSyncWorkerCheckpoint {
                store: "appfw_sync_checkpoint".to_string(),
                scope: "salesforce_to_pipeline".to_string(),
                watermark_mode: "incremental_watermark".to_string(),
            },
            readiness: RuntimeSyncWorkerReadiness {
                freshness_slo: "PT10M".to_string(),
                max_consecutive_failures: 3,
            },
            retry: RuntimeSyncWorkerRetryPolicy {
                enabled: true,
                max_attempts: 3,
                initial_backoff_ms: 1_000,
                max_backoff_ms: 30_000,
            },
            dead_letter: RuntimeSyncWorkerDeadLetterPolicy {
                enabled: true,
                store: "appfw_sync_dead_letter".to_string(),
                poison_record_policy: "record_and_continue".to_string(),
            },
            governance: RuntimeSyncWorkerGovernance {
                classification: "confidential".to_string(),
                retention: Some("P2Y".to_string()),
                deletion: "tombstone".to_string(),
                source_drift: "fail_closed".to_string(),
            },
            objects: vec![RuntimeSyncObject {
                source: "Account".to_string(),
                target: "Account".to_string(),
                key: "Id".to_string(),
                watermark: "SystemModstamp".to_string(),
                provenance: RuntimeSyncObjectProvenance {
                    projection_of: "salesforce_crm_v1.Account".to_string(),
                    source_key_field: "Id".to_string(),
                    source_watermark_field: "SystemModstamp".to_string(),
                    source_origin_field: Some("AppFrameworkOrigin__c".to_string()),
                },
                echo_loop: RuntimeSyncObjectEchoLoopPolicy {
                    policy: "reject_self_origin".to_string(),
                    local_origin: "appfw:pipeline:salesforce_to_pipeline".to_string(),
                    source_origin_field: "AppFrameworkOrigin__c".to_string(),
                },
                upsert: RuntimeSyncWorkerUpsertPolicy {
                    strategy: "idempotent_source_key".to_string(),
                    stable_source_key: "Id".to_string(),
                },
            }],
        }
    }

    fn cdc_worker() -> RuntimeSyncWorker {
        RuntimeSyncWorker {
            mode: "cdc_event".to_string(),
            schedule: "continuous".to_string(),
            checkpoint: RuntimeSyncWorkerCheckpoint {
                store: "appfw_sync_checkpoint".to_string(),
                scope: "salesforce_to_pipeline".to_string(),
                watermark_mode: "cdc_offset".to_string(),
            },
            objects: vec![RuntimeSyncObject {
                source: "AccountChangeEvent".to_string(),
                target: "Account".to_string(),
                key: "ChangeEventHeader.recordIds[0]".to_string(),
                watermark: "ChangeEventHeader.replayId".to_string(),
                provenance: RuntimeSyncObjectProvenance {
                    projection_of: "salesforce_crm_v1.AccountChangeEvent".to_string(),
                    source_key_field: "ChangeEventHeader.recordIds[0]".to_string(),
                    source_watermark_field: "ChangeEventHeader.replayId".to_string(),
                    source_origin_field: Some("AppFrameworkOrigin__c".to_string()),
                },
                echo_loop: RuntimeSyncObjectEchoLoopPolicy {
                    policy: "reject_self_origin".to_string(),
                    local_origin: "appfw:pipeline:salesforce_to_pipeline".to_string(),
                    source_origin_field: "AppFrameworkOrigin__c".to_string(),
                },
                upsert: RuntimeSyncWorkerUpsertPolicy {
                    strategy: "idempotent_source_key".to_string(),
                    stable_source_key: "ChangeEventHeader.recordIds[0]".to_string(),
                },
            }],
            ..worker()
        }
    }

    fn activation_gates() -> Vec<String> {
        [
            "provider_live_certification",
            "provider_request_execution",
            "checkpoint_persistence",
            "idempotent_projection_writes",
            "scheduler_readiness_and_freshness",
            "worker_deployment_wiring",
        ]
        .into_iter()
        .map(str::to_string)
        .collect()
    }

    #[test]
    fn validates_disabled_sync_worker_plan_with_workers() {
        let config = RuntimeSyncWorkerConfig {
            version: 1,
            enabled: false,
            activation_gates: activation_gates(),
            worker_count: 1,
            workers: vec![worker()],
        };

        config.validate().expect("disabled plan is valid");
        assert!(config
            .activation_gates
            .contains(&"provider_live_certification".to_string()));
        assert_eq!(config.planned_worker_count(), 1);
        assert_eq!(config.planned_object_count(), 1);
        assert!(config.worker("salesforce_to_pipeline").is_some());
    }

    #[test]
    fn loads_disabled_generated_config_from_yaml_file() {
        let path = std::env::temp_dir().join(format!(
            "appfw-sync-workers-{}-{}.yaml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::write(
            &path,
            r#"
version: 1
enabled: false
activation_gates:
- provider_live_certification
- provider_request_execution
- checkpoint_persistence
- idempotent_projection_writes
- scheduler_readiness_and_freshness
- worker_deployment_wiring
worker_count: 1
workers:
- name: salesforce_to_pipeline
  source_schema: salesforce_crm_v1
  target_schema: pipeline
  mode: incremental_watermark
  schedule: '*/5 * * * *'
  freshness_slo: PT10M
  checkpoint:
    store: appfw_sync_checkpoint
    scope: salesforce_to_pipeline
    watermark_mode: incremental_watermark
  readiness:
    freshness_slo: PT10M
    max_consecutive_failures: 3
  retry:
    enabled: true
    max_attempts: 3
    initial_backoff_ms: 1000
    max_backoff_ms: 30000
  dead_letter:
    enabled: true
    store: appfw_sync_dead_letter
    poison_record_policy: record_and_continue
  governance:
    classification: confidential
    retention: P2Y
    deletion: tombstone
    source_drift: fail_closed
  objects:
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
    upsert:
      strategy: idempotent_source_key
      stable_source_key: Id
"#,
        )
        .expect("write config");

        let config =
            RuntimeSyncWorkerConfig::from_yaml_file(&path).expect("load runtime sync config");
        std::fs::remove_file(&path).expect("remove config");

        assert!(!config.enabled);
        assert_eq!(config.activation_gates, activation_gates());
        assert_eq!(config.planned_worker_count(), 1);
        assert_eq!(config.workers[0].objects[0].source, "Account");
        assert_eq!(
            config.workers[0].objects[0].provenance.projection_of,
            "salesforce_crm_v1.Account"
        );
        assert_eq!(
            config.workers[0].objects[0].echo_loop.local_origin,
            "appfw:pipeline:salesforce_to_pipeline"
        );
    }

    #[test]
    fn validates_disabled_cdc_event_worker_with_offset_checkpoint() {
        let config = RuntimeSyncWorkerConfig {
            version: 1,
            enabled: false,
            activation_gates: activation_gates(),
            worker_count: 1,
            workers: vec![cdc_worker()],
        };

        config.validate().expect("cdc plan is valid");
        assert_eq!(config.workers[0].mode, "cdc_event");
        assert_eq!(config.workers[0].checkpoint.watermark_mode, "cdc_offset");
    }

    #[test]
    fn loads_empty_generated_config_from_yaml_file() {
        let path = std::env::temp_dir().join(format!(
            "appfw-sync-workers-empty-{}-{}.yaml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::write(
            &path,
            r#"
version: 1
enabled: false
activation_gates:
- provider_live_certification
worker_count: 0
workers: []
"#,
        )
        .expect("write config");

        let config =
            RuntimeSyncWorkerConfig::from_yaml_file(&path).expect("load runtime sync config");
        std::fs::remove_file(&path).expect("remove config");

        assert!(!config.enabled);
        assert_eq!(config.activation_gates, vec!["provider_live_certification"]);
        assert_eq!(config.planned_worker_count(), 0);
        assert_eq!(config.planned_object_count(), 0);
    }

    #[test]
    fn rejects_unknown_config_fields() {
        let path = std::env::temp_dir().join(format!(
            "appfw-sync-workers-unknown-{}-{}.yaml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::write(
            &path,
            r#"
version: 1
enabled: false
activation_gates: []
worker_count: 0
workers: []
surprise: nope
"#,
        )
        .expect("write config");

        let err =
            RuntimeSyncWorkerConfig::from_yaml_file(&path).expect_err("unknown field rejected");
        std::fs::remove_file(&path).expect("remove config");

        assert!(err.to_string().contains("surprise"));
    }

    #[test]
    fn rejects_blank_activation_gates() {
        let err = RuntimeSyncWorkerConfig {
            version: 1,
            enabled: false,
            activation_gates: vec!["provider_live_certification".to_string(), " ".to_string()],
            worker_count: 1,
            workers: vec![worker()],
        }
        .validate()
        .expect_err("blank activation gate rejected");

        assert!(err.to_string().contains("activation gate"));
    }

    #[test]
    fn rejects_enabled_sync_worker_config_until_runtime_is_certified() {
        let err = RuntimeSyncWorkerConfig {
            version: 1,
            enabled: true,
            activation_gates: vec!["provider_live_certification".to_string()],
            worker_count: 1,
            workers: vec![worker()],
        }
        .validate()
        .expect_err("enabled config rejected");

        assert!(err.to_string().contains("activation gates"));
        assert!(err.to_string().contains("provider_live_certification"));
    }

    #[test]
    fn worker_count_must_match_workers() {
        let err = RuntimeSyncWorkerConfig {
            version: 1,
            enabled: false,
            activation_gates: vec![],
            worker_count: 2,
            workers: vec![worker()],
        }
        .validate()
        .expect_err("worker_count mismatch rejected");

        assert!(err.to_string().contains("worker_count"));
    }

    #[test]
    fn rejects_duplicate_worker_names() {
        let err = RuntimeSyncWorkerConfig {
            version: 1,
            enabled: false,
            activation_gates: vec![],
            worker_count: 2,
            workers: vec![worker(), worker()],
        }
        .validate()
        .expect_err("duplicate worker rejected");

        assert!(err.to_string().contains("more than once"));
    }

    #[test]
    fn rejects_unsupported_mode() {
        let mut worker = worker();
        worker.mode = "full_refresh".to_string();

        let err = RuntimeSyncWorkerConfig {
            version: 1,
            enabled: false,
            activation_gates: vec![],
            worker_count: 1,
            workers: vec![worker],
        }
        .validate()
        .expect_err("unsupported mode rejected");

        assert!(err.to_string().contains("sync worker mode"));
        assert!(err.to_string().contains("incremental_watermark"));
        assert!(err.to_string().contains("cdc_event"));
    }

    #[test]
    fn cdc_event_worker_requires_cdc_offset_checkpoint() {
        let mut worker = cdc_worker();
        worker.checkpoint.watermark_mode = "incremental_watermark".to_string();

        let err = worker
            .validate()
            .expect_err("checkpoint mode mismatch rejected");

        assert!(err.to_string().contains("checkpoint watermark_mode"));
        assert!(err.to_string().contains("cdc_offset"));
    }

    #[test]
    fn retry_backoff_must_be_ordered() {
        let mut worker = worker();
        worker.retry.max_backoff_ms = 100;

        let err = worker.validate().expect_err("invalid retry backoff");

        assert!(err.to_string().contains("max_backoff_ms"));
    }

    #[test]
    fn rejects_unknown_governance_policy_values() {
        let mut worker = worker();
        worker.governance.source_drift = "best_effort".to_string();

        let err = worker.validate().expect_err("source drift rejected");

        assert!(err.to_string().contains("source_drift"));
    }

    #[test]
    fn regulated_governance_requires_retention() {
        let mut worker = worker();
        worker.governance.retention = None;

        let err = worker.validate().expect_err("retention required");

        assert!(err.to_string().contains("retention"));
    }

    #[test]
    fn rejects_unknown_poison_record_policy() {
        let mut worker = worker();
        worker.dead_letter.poison_record_policy = "drop".to_string();

        let err = worker.validate().expect_err("poison policy rejected");

        assert!(err.to_string().contains("poison_record_policy"));
    }

    #[test]
    fn upsert_stable_source_key_must_match_object_key() {
        let mut worker = worker();
        worker.objects[0].upsert.stable_source_key = "OtherId".to_string();

        let err = worker.validate().expect_err("upsert key mismatch");

        assert!(err.to_string().contains("stable_source_key"));
    }

    #[test]
    fn provenance_key_and_watermark_must_match_object_contract() {
        let mut worker = worker();
        worker.objects[0].provenance.source_key_field = "OtherId".to_string();

        let err = worker.validate().expect_err("provenance key mismatch");

        assert!(err.to_string().contains("source_key_field"));
    }

    #[test]
    fn echo_loop_rejects_self_origin_source_records() {
        let mut worker = worker();
        let object = worker.objects.remove(0);

        assert!(object.echo_loop.rejects_source_record(&serde_json::json!({
            "Id": "001",
            "AppFrameworkOrigin__c": "appfw:pipeline:salesforce_to_pipeline"
        })));
        assert!(!object.echo_loop.rejects_source_record(&serde_json::json!({
            "Id": "002",
            "AppFrameworkOrigin__c": "salesforce:user-edit"
        })));
    }

    #[test]
    fn record_provenance_shapes_lineage_payload() {
        let mut worker = worker();
        let object = worker.objects.remove(0);

        let provenance = object.record_provenance(
            "salesforce_to_pipeline",
            "001",
            "2026-07-03T00:00:00Z",
            Some("salesforce:user-edit".to_string()),
        );

        assert_eq!(provenance.sync_worker, "salesforce_to_pipeline");
        assert_eq!(provenance.projection_of, "salesforce_crm_v1.Account");
        assert_eq!(provenance.source_key_field, "Id");
        assert_eq!(provenance.source_key_value, "001");
        assert_eq!(
            provenance.source_origin_field.as_deref(),
            Some("AppFrameworkOrigin__c")
        );
        assert_eq!(
            provenance.source_origin_value.as_deref(),
            Some("salesforce:user-edit")
        );
    }

    #[tokio::test]
    async fn worker_shell_reports_planned_workers_without_execution() {
        let config = RuntimeSyncWorkerConfig {
            version: 1,
            enabled: false,
            activation_gates: vec!["provider_live_certification".to_string()],
            worker_count: 1,
            workers: vec![worker()],
        };

        let report = run_sync_worker_shell(&config)
            .await
            .expect("disabled shell report");

        assert!(!report.enabled);
        assert_eq!(report.activation_gates, vec!["provider_live_certification"]);
        assert_eq!(report.workers_planned, 1);
        assert_eq!(report.objects_planned, 1);
        assert_eq!(report.workers_started, 0);
    }

    #[test]
    fn local_fixture_runner_projects_records_and_advances_checkpoint() {
        let worker = worker();
        let records = BTreeMap::from([(
            "Account".to_string(),
            vec![
                serde_json::json!({
                    "Id": "001",
                    "SystemModstamp": "2026-07-04T00:00:00Z",
                    "AppFrameworkOrigin__c": "salesforce:user-edit",
                    "Name": "Acme"
                }),
                serde_json::json!({
                    "Id": "002",
                    "SystemModstamp": "2026-07-04T00:05:00Z",
                    "AppFrameworkOrigin__c": "salesforce:user-edit",
                    "Name": "Northstar"
                }),
            ],
        )]);

        let report =
            run_sync_worker_local_fixture(&worker, &records).expect("local execution report");

        assert_eq!(report.worker, "salesforce_to_pipeline");
        assert!(!report.provider_live_certified);
        assert_eq!(report.records_seen, 2);
        assert_eq!(report.records_projected, 2);
        assert_eq!(report.records_suppressed, 0);
        assert_eq!(report.dead_letters, 0);

        let object_report = &report.object_reports[0];
        assert_eq!(object_report.target_entity, "Account");
        assert_eq!(
            object_report.checkpoint.watermark.as_deref(),
            Some("2026-07-04T00:05:00Z")
        );
        assert_eq!(object_report.checkpoint.scope, "salesforce_to_pipeline");
        assert_eq!(object_report.projection_writes[0].source_key, "001");
        assert_eq!(
            object_report.projection_writes[0].provenance.projection_of,
            "salesforce_crm_v1.Account"
        );
    }

    #[test]
    fn local_fixture_runner_suppresses_echo_loop_records_and_tracks_duplicate_keys() {
        let worker = worker();
        let records = BTreeMap::from([(
            "Account".to_string(),
            vec![
                serde_json::json!({
                    "Id": "001",
                    "SystemModstamp": "2026-07-04T00:00:00Z",
                    "AppFrameworkOrigin__c": "appfw:pipeline:salesforce_to_pipeline"
                }),
                serde_json::json!({
                    "Id": "002",
                    "SystemModstamp": "2026-07-04T00:01:00Z",
                    "AppFrameworkOrigin__c": "salesforce:user-edit"
                }),
                serde_json::json!({
                    "Id": "002",
                    "SystemModstamp": "2026-07-04T00:02:00Z",
                    "AppFrameworkOrigin__c": "salesforce:user-edit"
                }),
            ],
        )]);

        let report =
            run_sync_worker_local_fixture(&worker, &records).expect("local execution report");
        let object_report = &report.object_reports[0];

        assert_eq!(report.records_seen, 3);
        assert_eq!(report.records_suppressed, 1);
        assert_eq!(report.records_projected, 2);
        assert_eq!(object_report.duplicate_source_keys, 1);
        assert_eq!(
            object_report.checkpoint.watermark.as_deref(),
            Some("2026-07-04T00:02:00Z")
        );
    }

    #[test]
    fn local_fixture_runner_keeps_monotonic_checkpoint_for_out_of_order_records() {
        let worker = worker();
        let records = BTreeMap::from([(
            "Account".to_string(),
            vec![
                serde_json::json!({
                    "Id": "001",
                    "SystemModstamp": "2026-07-04T00:10:00Z",
                    "AppFrameworkOrigin__c": "salesforce:user-edit"
                }),
                serde_json::json!({
                    "Id": "002",
                    "SystemModstamp": "2026-07-04T00:01:00Z",
                    "AppFrameworkOrigin__c": "salesforce:user-edit"
                }),
            ],
        )]);

        let report =
            run_sync_worker_local_fixture(&worker, &records).expect("local execution report");

        assert_eq!(
            report.object_reports[0].checkpoint.watermark.as_deref(),
            Some("2026-07-04T00:10:00Z")
        );
    }

    #[test]
    fn local_fixture_runner_compares_numeric_cdc_checkpoint_values() {
        let worker = cdc_worker();
        let records = BTreeMap::from([(
            "AccountChangeEvent".to_string(),
            vec![
                serde_json::json!({
                    "ChangeEventHeader.recordIds[0]": "001",
                    "ChangeEventHeader.replayId": "9",
                    "AppFrameworkOrigin__c": "salesforce:user-edit"
                }),
                serde_json::json!({
                    "ChangeEventHeader.recordIds[0]": "002",
                    "ChangeEventHeader.replayId": "10",
                    "AppFrameworkOrigin__c": "salesforce:user-edit"
                }),
            ],
        )]);

        let report =
            run_sync_worker_local_fixture(&worker, &records).expect("local execution report");

        assert_eq!(
            report.object_reports[0].checkpoint.watermark.as_deref(),
            Some("10")
        );
    }

    #[test]
    fn local_fixture_runner_dead_letters_poison_records_when_enabled() {
        let worker = worker();
        let records = BTreeMap::from([(
            "Account".to_string(),
            vec![
                serde_json::json!({
                    "SystemModstamp": "2026-07-04T00:00:00Z",
                    "Name": "Missing source key"
                }),
                serde_json::json!({
                    "Id": "002",
                    "Name": "Missing watermark"
                }),
            ],
        )]);

        let report =
            run_sync_worker_local_fixture(&worker, &records).expect("local execution report");
        let object_report = &report.object_reports[0];

        assert_eq!(report.records_seen, 2);
        assert_eq!(report.records_projected, 0);
        assert_eq!(report.dead_letters, 2);
        assert!(object_report.dead_letter_records[0]
            .reason
            .contains("source key"));
        assert!(object_report.dead_letter_records[1]
            .reason
            .contains("source watermark"));
        assert!(object_report.dead_letter_records[0]
            .payload_summary
            .contains("SystemModstamp"));
    }

    #[test]
    fn local_fixture_runner_fails_poison_records_without_dead_letter_policy() {
        let mut worker = worker();
        worker.dead_letter.enabled = false;
        let records = BTreeMap::from([(
            "Account".to_string(),
            vec![serde_json::json!({
                "SystemModstamp": "2026-07-04T00:00:00Z"
            })],
        )]);

        let err = run_sync_worker_local_fixture(&worker, &records)
            .expect_err("poison record fails without DLQ");

        assert!(err.to_string().contains("cannot process poison record"));
    }

    #[test]
    fn local_fixture_with_stores_resumes_from_checkpoint_and_persists_next_watermark() {
        let worker = worker();
        let object = worker.objects[0].clone();
        let mut checkpoint_store = RuntimeSyncInMemoryCheckpointStore::new();
        checkpoint_store.seed_checkpoint(&worker, &object, "2026-07-04T00:05:00Z");
        let mut projection_store = RuntimeSyncInMemoryProjectionStore::new();
        let records = BTreeMap::from([(
            "Account".to_string(),
            vec![
                serde_json::json!({
                    "Id": "001",
                    "SystemModstamp": "2026-07-04T00:01:00Z",
                    "AppFrameworkOrigin__c": "salesforce:user-edit"
                }),
                serde_json::json!({
                    "Id": "002",
                    "SystemModstamp": "2026-07-04T00:10:00Z",
                    "AppFrameworkOrigin__c": "salesforce:user-edit"
                }),
            ],
        )]);

        let report = run_sync_worker_local_fixture_with_stores(
            &worker,
            &records,
            &mut checkpoint_store,
            &mut projection_store,
        )
        .expect("stored local execution report");

        let object_report = &report.object_reports[0];
        assert_eq!(report.records_skipped_by_checkpoint, 1);
        assert_eq!(report.projection_upserts_applied, 1);
        assert_eq!(object_report.records_seen, 2);
        assert_eq!(object_report.records_skipped_by_checkpoint, 1);
        assert_eq!(object_report.records_projected, 1);
        assert_eq!(object_report.projection_upserts_applied, 1);
        assert_eq!(
            checkpoint_store.checkpoint(&worker, &object),
            Some("2026-07-04T00:10:00Z")
        );
        assert_eq!(projection_store.len(), 1);
        assert_eq!(
            projection_store
                .record("Account", "002")
                .map(|record| record.source_watermark.as_str()),
            Some("2026-07-04T00:10:00Z")
        );
    }

    #[test]
    fn local_fixture_with_stores_skips_stale_projection_replays() {
        let worker = worker();
        let mut checkpoint_store = RuntimeSyncInMemoryCheckpointStore::new();
        let mut projection_store = RuntimeSyncInMemoryProjectionStore::new();
        projection_store
            .upsert_projection(&RuntimeSyncProjectionWrite {
                target: "Account".to_string(),
                source_key: "001".to_string(),
                source_watermark: "2026-07-04T00:10:00Z".to_string(),
                provenance: worker.objects[0].record_provenance(
                    &worker.name,
                    "001",
                    "2026-07-04T00:10:00Z",
                    Some("salesforce:user-edit".to_string()),
                ),
            })
            .expect("seed projection");
        let records = BTreeMap::from([(
            "Account".to_string(),
            vec![serde_json::json!({
                "Id": "001",
                "SystemModstamp": "2026-07-04T00:05:00Z",
                "AppFrameworkOrigin__c": "salesforce:user-edit"
            })],
        )]);

        let report = run_sync_worker_local_fixture_with_stores(
            &worker,
            &records,
            &mut checkpoint_store,
            &mut projection_store,
        )
        .expect("stored local execution report");

        let object_report = &report.object_reports[0];
        assert_eq!(report.projection_upserts_applied, 0);
        assert_eq!(report.projection_upserts_skipped, 1);
        assert_eq!(object_report.records_projected, 1);
        assert_eq!(object_report.projection_upserts_applied, 0);
        assert_eq!(object_report.projection_upserts_skipped, 1);
        assert_eq!(
            projection_store
                .record("Account", "001")
                .map(|record| record.source_watermark.as_str()),
            Some("2026-07-04T00:10:00Z")
        );
    }
}
