use std::{sync::Arc, time::Instant};

use appfw_runtime::{
    observability::MetricsRegistry, runtime_info_routes, RuntimeHealthCheck, RuntimeReadinessProbe,
    RuntimeReadinessState,
};
use async_trait::async_trait;
use axum::Router;

use crate::data::data_access::DataAccess;

pub type HealthCheck = RuntimeHealthCheck;

#[derive(Clone)]
pub struct ReadinessState {
    inner: RuntimeReadinessState<ProviderReadinessCheck>,
}

impl ReadinessState {
    pub fn new(checks: Vec<HealthCheck>) -> Self {
        Self {
            inner: RuntimeReadinessState::new(checks),
        }
    }

    pub fn with_provider_check(
        mut self,
        schema_name: impl Into<String>,
        data_access: Arc<DataAccess>,
    ) -> Self {
        self.inner = self.inner.with_provider_check(ProviderReadinessCheck {
            schema_name: schema_name.into(),
            data_access,
        });
        self
    }
}

#[derive(Clone)]
struct ProviderReadinessCheck {
    schema_name: String,
    data_access: Arc<DataAccess>,
}

#[async_trait]
impl RuntimeReadinessProbe for ProviderReadinessCheck {
    async fn run(&self) -> RuntimeHealthCheck {
        let started_at = Instant::now();
        let provider_descriptor = self.data_access.provider_descriptor();
        let data_source_name = provider_descriptor.data_source_name().to_string();
        let provider = provider_descriptor.provider_key().to_string();
        let check = match self.data_access.health_check().await {
            Ok(_) => RuntimeHealthCheck::pass(
                format!("provider:{}", data_source_name),
                "provider responded to readiness probe",
            ),
            Err(err) => RuntimeHealthCheck::fail(
                format!("provider:{}", data_source_name),
                format!("provider readiness probe failed: {err}"),
            ),
        };
        let duration_ms = started_at.elapsed().as_millis() as u64;

        check
            .with_provider(provider)
            .with_data_source(data_source_name)
            .with_schema(self.schema_name.clone())
            .with_duration_ms(duration_ms)
    }

    fn record_pool_stats(&self, metrics: &MetricsRegistry) {
        metrics.record_provider_pool_stats(self.data_access.pool_stats());
    }
}

pub async fn nest_routes(
    parent: Router,
    metrics: MetricsRegistry,
    readiness: ReadinessState,
) -> Router {
    runtime_info_routes(parent, metrics, readiness.inner).await
}
