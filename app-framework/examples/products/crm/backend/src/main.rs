#[cfg(feature = "http")]
use appfw_runtime::{cors, RuntimeAuthState, RuntimeHttpServerConfig};
use appfw_runtime::{
    observability::init_tracing, security::SecurityConfig, RuntimeHostPlan, RuntimeMode,
};
use dotenv::dotenv;
#[cfg(feature = "http")]
use std::sync::Arc;
use tracing::error;
#[cfg(feature = "http")]
use tracing::info;

#[cfg(feature = "http")]
mod admin_ui;
mod config;
mod data;
mod handlers;
#[cfg(feature = "kafka")]
mod kafka;
#[cfg(all(feature = "http", feature = "mcp"))]
mod mcp;
#[cfg(any(feature = "mcp", feature = "kafka"))]
mod operations;
mod product_api;
mod routes;
mod schemas;
mod services;
#[cfg(feature = "sync")]
mod sync_workers;

#[cfg(feature = "http")]
use config::app_config::AppConfig;
#[cfg(feature = "http")]
use routes::get_routes;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let observability_guard = init_tracing();
    let security = SecurityConfig::from_env();
    if let Err(e) = security.validate_runtime_safety() {
        error!(error = %e, "unsafe security configuration");
        std::process::exit(1);
    }

    let runtime_mode = match RuntimeMode::from_env() {
        Ok(mode) => mode,
        Err(e) => {
            error!(error = %e, "invalid runtime mode configuration");
            std::process::exit(1);
        }
    };
    let host_plan = RuntimeHostPlan::new(runtime_mode);

    #[cfg(feature = "sync")]
    if host_plan.has_sync_worker_with_listener_modules() {
        error!(
            modules = ?host_plan.module_names(),
            workers = ?host_plan.worker_module_names(),
            "SaaS sync workers must run in a worker-only process; deploy APPFW_RUNTIME_MODE=sync separately from the HTTP backend"
        );
        observability_guard.shutdown();
        std::process::exit(1);
    }

    if host_plan.has_multiple_worker_modules() {
        error!(
            modules = ?host_plan.module_names(),
            workers = ?host_plan.worker_module_names(),
            "multiple runtime worker modules are selected; run one worker mode per process until a worker supervisor is available"
        );
        observability_guard.shutdown();
        std::process::exit(1);
    }

    #[cfg(feature = "kafka")]
    if host_plan.runs_kafka_workers() {
        if let Err(e) = kafka::run_workers(&host_plan).await {
            error!(error = %e, "Kafka runtime worker host error");
            observability_guard.shutdown();
            std::process::exit(1);
        }

        observability_guard.shutdown();
        return;
    }

    #[cfg(feature = "sync")]
    if host_plan.runs_sync_workers() {
        if let Err(e) = sync_workers::run_workers(&host_plan).await {
            error!(error = %e, "SaaS sync runtime worker host error");
            observability_guard.shutdown();
            std::process::exit(1);
        }

        observability_guard.shutdown();
        return;
    }

    if host_plan.has_unsupported_worker_modules() {
        error!(
            modules = ?host_plan.module_names(),
            workers = ?host_plan.worker_module_names(),
            "this product backend does not yet provide runtime worker modules"
        );
        observability_guard.shutdown();
        std::process::exit(1);
    }

    #[cfg(feature = "http")]
    if host_plan.serves_http_listener() {
        let app_config = match AppConfig::init().await {
            Ok(config) => Arc::new(config),
            Err(e) => {
                error!(error = %e, "failed to initialize app config");
                std::process::exit(1);
            }
        };
        let app_state = match RuntimeAuthState::from_env() {
            Ok(state) => state,
            Err(e) => {
                error!(error = %e, "failed to initialize app state");
                std::process::exit(1);
            }
        };

        info!(auth_configured = true, "Okta configuration loaded");

        let cors = cors::get();

        let routes = match get_routes(
            cors,
            app_config.clone(),
            app_state.clone(),
            security,
            host_plan.mode().clone(),
        )
        .await
        {
            Ok(routes) => routes,
            Err(e) => {
                error!(error = %e, "failed to build routes");
                std::process::exit(1);
            }
        };

        let http_config = match RuntimeHttpServerConfig::from_env() {
            Ok(config) => config,
            Err(e) => {
                error!(error = %e, "invalid HTTP server configuration");
                std::process::exit(1);
            }
        };

        if let Err(e) = appfw_runtime::serve_http_router(routes, &http_config).await {
            error!(error = %e, "backend runtime host error");
            std::process::exit(1);
        }

        observability_guard.shutdown();
        return;
    }

    error!(
        modules = ?host_plan.module_names(),
        "this product backend has no enabled runtime ingress modules it can host"
    );
    observability_guard.shutdown();
    std::process::exit(1);
}
