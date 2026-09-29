use std::path::PathBuf;

use appfw_runtime::{ConfigError, RuntimeAppError, RuntimeHostPlan, RuntimeKafkaIngressConfig};

const DEFAULT_KAFKA_INGRESS_CONFIG: &str = "config/generated/ingress/kafka.yaml";
const KAFKA_INGRESS_CONFIG_ENV: &str = "APPFW_KAFKA_INGRESS_CONFIG";

pub(crate) async fn run_workers(host_plan: &RuntimeHostPlan) -> Result<(), RuntimeAppError> {
    if !host_plan.runs_kafka_workers() {
        return Ok(());
    }

    let config_path = kafka_ingress_config_path()?;
    let ingress = RuntimeKafkaIngressConfig::from_yaml_file(&config_path)?;
    if !ingress.enabled {
        return Err(ConfigError::Load(format!(
            "Kafka runtime ingress is selected, but {} has enabled: false",
            config_path.display()
        ))
        .into());
    }

    Err(ConfigError::Load(format!(
        "Kafka runtime ingress loaded {} consumer(s) from {}, but this product has not bound a Kafka broker message source yet",
        ingress.consumer_count(),
        config_path.display()
    ))
    .into())
}

fn kafka_ingress_config_path() -> Result<PathBuf, ConfigError> {
    if let Some(path) = std::env::var_os(KAFKA_INGRESS_CONFIG_ENV).filter(|value| !value.is_empty())
    {
        return Ok(PathBuf::from(path));
    }

    std::env::current_dir()
        .map(|cwd| cwd.join(DEFAULT_KAFKA_INGRESS_CONFIG))
        .map_err(|err| ConfigError::Load(format!("could not get current dir: {err}")))
}
