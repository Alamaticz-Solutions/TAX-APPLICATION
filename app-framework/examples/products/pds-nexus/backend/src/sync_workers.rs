use std::{ffi::OsString, path::PathBuf};

use appfw_runtime::{
    run_sync_worker_shell, ConfigError, RuntimeAppError, RuntimeHostPlan, RuntimeSyncWorkerConfig,
};

const DEFAULT_SYNC_WORKERS_CONFIG: &str = "config/generated/sync_workers.yaml";
const SYNC_WORKERS_CONFIG_ENV: &str = "APPFW_SYNC_WORKERS_CONFIG";

pub(crate) async fn run_workers(host_plan: &RuntimeHostPlan) -> Result<(), RuntimeAppError> {
    if !host_plan.runs_sync_workers() {
        return Ok(());
    }
    if host_plan.has_sync_worker_with_listener_modules() {
        return Err(ConfigError::Load(
            "SaaS sync workers must run in a worker-only process; deploy APPFW_RUNTIME_MODE=sync separately from the HTTP backend"
                .to_string(),
        )
        .into());
    }

    let config_path = sync_workers_config_path()?;
    let config = RuntimeSyncWorkerConfig::from_yaml_file(&config_path)?;
    let report = run_sync_worker_shell(&config).await?;
    let activation_gates = if report.activation_gates.is_empty() {
        "runtime worker code and provider certification".to_string()
    } else {
        report.activation_gates.join(", ")
    };

    Err(ConfigError::Load(format!(
        "SaaS sync worker runtime loaded {} planned worker(s) and {} object map(s) from {}, but execution remains disabled until activation gates pass: {}",
        report.workers_planned,
        report.objects_planned,
        config_path.display(),
        activation_gates
    ))
    .into())
}

fn sync_workers_config_path() -> Result<PathBuf, ConfigError> {
    let cwd = std::env::current_dir()
        .map_err(|err| ConfigError::Load(format!("could not get current dir: {err}")))?;
    sync_workers_config_path_from(std::env::var_os(SYNC_WORKERS_CONFIG_ENV), cwd)
}

fn sync_workers_config_path_from(
    env_value: Option<OsString>,
    cwd: PathBuf,
) -> Result<PathBuf, ConfigError> {
    if let Some(path) = env_value.filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }

    Ok(cwd.join(DEFAULT_SYNC_WORKERS_CONFIG))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_workers_config_path_uses_env_override() {
        let path = sync_workers_config_path_from(
            Some(OsString::from("/tmp/custom-sync-workers.yaml")),
            PathBuf::from("/workspace/backend"),
        )
        .expect("config path");

        assert_eq!(path, PathBuf::from("/tmp/custom-sync-workers.yaml"));
    }

    #[test]
    fn sync_workers_config_path_defaults_under_backend_config() {
        let path = sync_workers_config_path_from(None, PathBuf::from("/workspace/backend"))
            .expect("config path");

        assert_eq!(
            path,
            PathBuf::from("/workspace/backend/config/generated/sync_workers.yaml")
        );
    }
}
