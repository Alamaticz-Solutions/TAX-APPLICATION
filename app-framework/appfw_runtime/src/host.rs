#[cfg(feature = "http")]
use std::net::SocketAddr;
use std::{collections::BTreeSet, env, fmt};

#[cfg(feature = "http")]
use axum::Router;
use thiserror::Error;
#[cfg(feature = "http")]
use tokio::net::TcpListener;
#[cfg(feature = "http")]
use tracing::{error, info};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum RuntimeIngressKind {
    #[cfg(feature = "chat")]
    Chat,
    #[cfg(feature = "http")]
    Http,
    #[cfg(feature = "mcp")]
    Mcp,
    #[cfg(feature = "kafka")]
    Kafka,
    #[cfg(feature = "sync")]
    Sync,
}

impl RuntimeIngressKind {
    pub fn as_str(self) -> &'static str {
        match self {
            #[cfg(feature = "chat")]
            Self::Chat => "chat",
            #[cfg(feature = "http")]
            Self::Http => "http",
            #[cfg(feature = "mcp")]
            Self::Mcp => "mcp",
            #[cfg(feature = "kafka")]
            Self::Kafka => "kafka",
            #[cfg(feature = "sync")]
            Self::Sync => "sync",
        }
    }
}

impl fmt::Display for RuntimeIngressKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for RuntimeIngressKind {
    type Err = RuntimeHostError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            #[cfg(feature = "chat")]
            "chat" | "ai-chat" | "conversation" => Ok(Self::Chat),
            #[cfg(not(feature = "chat"))]
            "chat" | "ai-chat" | "conversation" => {
                Err(RuntimeHostError::ModuleFeatureDisabled("chat"))
            }
            #[cfg(feature = "http")]
            "http" | "graphql" | "admin" => Ok(Self::Http),
            #[cfg(not(feature = "http"))]
            "http" | "graphql" | "admin" => Err(RuntimeHostError::ModuleFeatureDisabled("http")),
            #[cfg(feature = "mcp")]
            "mcp" => Ok(Self::Mcp),
            #[cfg(not(feature = "mcp"))]
            "mcp" => Err(RuntimeHostError::ModuleFeatureDisabled("mcp")),
            #[cfg(feature = "kafka")]
            "kafka" | "consumer" | "consumers" => Ok(Self::Kafka),
            #[cfg(not(feature = "kafka"))]
            "kafka" | "consumer" | "consumers" => {
                Err(RuntimeHostError::ModuleFeatureDisabled("kafka"))
            }
            #[cfg(feature = "sync")]
            "sync" | "sync-worker" | "sync-workers" => Ok(Self::Sync),
            #[cfg(not(feature = "sync"))]
            "sync" | "sync-worker" | "sync-workers" => {
                Err(RuntimeHostError::ModuleFeatureDisabled("sync"))
            }
            other => Err(RuntimeHostError::InvalidMode(format!(
                "unknown runtime ingress module `{}`",
                other
            ))),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeMode {
    modules: BTreeSet<RuntimeIngressKind>,
}

impl RuntimeMode {
    pub fn all() -> Self {
        #[allow(unused_mut)]
        let mut modules = BTreeSet::new();
        #[cfg(feature = "chat")]
        modules.insert(RuntimeIngressKind::Chat);
        #[cfg(feature = "http")]
        modules.insert(RuntimeIngressKind::Http);
        #[cfg(feature = "mcp")]
        modules.insert(RuntimeIngressKind::Mcp);
        #[cfg(feature = "kafka")]
        modules.insert(RuntimeIngressKind::Kafka);
        #[cfg(feature = "sync")]
        modules.insert(RuntimeIngressKind::Sync);
        Self { modules }
    }

    pub fn default_mode() -> Self {
        #[cfg(feature = "http")]
        {
            Self::http()
        }
        #[cfg(all(not(feature = "http"), feature = "mcp"))]
        {
            Self::mcp()
        }
        #[cfg(all(not(any(feature = "http", feature = "mcp")), feature = "kafka"))]
        {
            Self::consumers()
        }
        #[cfg(all(
            not(any(feature = "http", feature = "mcp", feature = "kafka")),
            feature = "sync"
        ))]
        {
            Self::sync_workers()
        }
        #[cfg(not(any(feature = "http", feature = "mcp", feature = "kafka", feature = "sync")))]
        {
            Self::all()
        }
    }

    #[cfg(feature = "http")]
    pub fn http() -> Self {
        Self::from_modules([RuntimeIngressKind::Http])
    }

    #[cfg(feature = "chat")]
    pub fn chat() -> Self {
        Self::from_modules([RuntimeIngressKind::Chat])
    }

    #[cfg(feature = "mcp")]
    pub fn mcp() -> Self {
        Self::from_modules([RuntimeIngressKind::Mcp])
    }

    #[cfg(feature = "kafka")]
    pub fn consumers() -> Self {
        Self::from_modules([RuntimeIngressKind::Kafka])
    }

    #[cfg(feature = "sync")]
    pub fn sync_workers() -> Self {
        Self::from_modules([RuntimeIngressKind::Sync])
    }

    pub fn from_env() -> Result<Self, RuntimeHostError> {
        if let Ok(modules) = env::var("APPFW_MODULES") {
            return Self::parse_modules(&modules);
        }
        match env::var("APPFW_RUNTIME_MODE") {
            Ok(mode) => Self::parse_mode(&mode),
            Err(_) => Ok(Self::default_mode()),
        }
    }

    pub fn parse_mode(value: &str) -> Result<Self, RuntimeHostError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "" | "all" => Ok(Self::all()),
            #[cfg(feature = "http")]
            "http" | "server" | "serve-http" => Ok(Self::http()),
            #[cfg(not(feature = "http"))]
            "http" | "server" | "serve-http" => {
                Err(RuntimeHostError::ModuleFeatureDisabled("http"))
            }
            #[cfg(feature = "chat")]
            "chat" | "serve-chat" | "conversation" => Ok(Self::chat()),
            #[cfg(not(feature = "chat"))]
            "chat" | "serve-chat" | "conversation" => {
                Err(RuntimeHostError::ModuleFeatureDisabled("chat"))
            }
            #[cfg(feature = "mcp")]
            "mcp" | "serve-mcp" => Ok(Self::mcp()),
            #[cfg(not(feature = "mcp"))]
            "mcp" | "serve-mcp" => Err(RuntimeHostError::ModuleFeatureDisabled("mcp")),
            #[cfg(feature = "kafka")]
            "consumer" | "consumers" | "kafka" | "run-consumers" => Ok(Self::consumers()),
            #[cfg(not(feature = "kafka"))]
            "consumer" | "consumers" | "kafka" | "run-consumers" => {
                Err(RuntimeHostError::ModuleFeatureDisabled("kafka"))
            }
            #[cfg(feature = "sync")]
            "sync" | "sync-worker" | "sync-workers" | "run-sync-workers" => {
                Ok(Self::sync_workers())
            }
            #[cfg(not(feature = "sync"))]
            "sync" | "sync-worker" | "sync-workers" | "run-sync-workers" => {
                Err(RuntimeHostError::ModuleFeatureDisabled("sync"))
            }
            other => Err(RuntimeHostError::InvalidMode(format!(
                "APPFW_RUNTIME_MODE must be one of all, http, chat, mcp, consumers, sync; got `{}`",
                other
            ))),
        }
    }

    pub fn parse_modules(value: &str) -> Result<Self, RuntimeHostError> {
        let modules: Result<BTreeSet<_>, _> = value
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(str::parse::<RuntimeIngressKind>)
            .collect();
        let modules = modules?;
        if modules.is_empty() {
            return Err(RuntimeHostError::InvalidMode(
                "APPFW_MODULES must include at least one runtime ingress module".to_string(),
            ));
        }
        Ok(Self { modules })
    }

    pub fn from_modules<const N: usize>(modules: [RuntimeIngressKind; N]) -> Self {
        Self {
            modules: modules.into_iter().collect(),
        }
    }

    pub fn enables(&self, module: RuntimeIngressKind) -> bool {
        self.modules.contains(&module)
    }

    pub fn enables_http_listener(&self) -> bool {
        let http_enabled = {
            #[cfg(feature = "http")]
            {
                self.enables(RuntimeIngressKind::Http)
            }
            #[cfg(not(feature = "http"))]
            {
                false
            }
        };
        let chat_enabled = {
            #[cfg(all(feature = "http", feature = "chat"))]
            {
                self.enables(RuntimeIngressKind::Chat)
            }
            #[cfg(not(all(feature = "http", feature = "chat")))]
            {
                false
            }
        };
        let mcp_enabled = {
            #[cfg(all(feature = "http", feature = "mcp"))]
            {
                self.enables(RuntimeIngressKind::Mcp)
            }
            #[cfg(not(all(feature = "http", feature = "mcp")))]
            {
                false
            }
        };
        http_enabled || chat_enabled || mcp_enabled
    }

    pub fn module_names(&self) -> Vec<&'static str> {
        self.modules.iter().map(|module| module.as_str()).collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeHostPlan {
    mode: RuntimeMode,
}

impl RuntimeHostPlan {
    pub fn new(mode: RuntimeMode) -> Self {
        Self { mode }
    }

    pub fn from_env() -> Result<Self, RuntimeHostError> {
        RuntimeMode::from_env().map(Self::new)
    }

    pub fn mode(&self) -> &RuntimeMode {
        &self.mode
    }

    pub fn serves_http_listener(&self) -> bool {
        self.mode.enables_http_listener()
    }

    pub fn worker_module_names(&self) -> Vec<&'static str> {
        #[allow(unused_mut)]
        let mut modules = Vec::new();
        #[cfg(all(feature = "mcp", not(feature = "http")))]
        if self.mode.enables(RuntimeIngressKind::Mcp) {
            modules.push(RuntimeIngressKind::Mcp.as_str());
        }
        #[cfg(feature = "kafka")]
        if self.mode.enables(RuntimeIngressKind::Kafka) {
            modules.push(RuntimeIngressKind::Kafka.as_str());
        }
        #[cfg(feature = "sync")]
        if self.mode.enables(RuntimeIngressKind::Sync) {
            modules.push(RuntimeIngressKind::Sync.as_str());
        }
        modules
    }

    #[cfg(feature = "kafka")]
    pub fn runs_kafka_workers(&self) -> bool {
        self.mode.enables(RuntimeIngressKind::Kafka)
    }

    #[cfg(feature = "sync")]
    pub fn runs_sync_workers(&self) -> bool {
        self.mode.enables(RuntimeIngressKind::Sync)
    }

    pub fn module_names(&self) -> Vec<&'static str> {
        self.mode.module_names()
    }

    pub fn has_unsupported_worker_modules(&self) -> bool {
        !self.worker_module_names().is_empty()
    }

    pub fn has_multiple_worker_modules(&self) -> bool {
        self.worker_module_names().len() > 1
    }

    #[cfg(feature = "sync")]
    pub fn has_sync_worker_with_listener_modules(&self) -> bool {
        self.runs_sync_workers() && self.serves_http_listener()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeIngressDescriptor {
    pub kind: RuntimeIngressKind,
    pub name: String,
    pub config_enabled: bool,
}

impl RuntimeIngressDescriptor {
    pub fn new(kind: RuntimeIngressKind, name: impl Into<String>, config_enabled: bool) -> Self {
        Self {
            kind,
            name: name.into(),
            config_enabled,
        }
    }

    pub fn enabled_in(&self, mode: &RuntimeMode) -> bool {
        self.config_enabled && mode.enables(self.kind)
    }
}

#[cfg(feature = "http")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeHttpServerConfig {
    pub host: String,
    pub port: String,
}

#[cfg(feature = "http")]
impl RuntimeHttpServerConfig {
    pub fn from_env() -> Result<Self, RuntimeHostError> {
        let host = env::var("API_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = env::var("API_PORT")
            .map_err(|_| RuntimeHostError::MissingEnv("API_PORT".to_string()))?;
        Ok(Self { host, port })
    }

    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn socket_addr(&self) -> Result<SocketAddr, RuntimeHostError> {
        self.address()
            .parse()
            .map_err(|source| RuntimeHostError::InvalidSocketAddress {
                address: self.address(),
                source,
            })
    }
}

#[cfg(feature = "http")]
pub async fn serve_http_router(
    router: Router,
    config: &RuntimeHttpServerConfig,
) -> Result<(), RuntimeHostError> {
    let addr = config.address();
    let sock_addr = config.socket_addr()?;
    let listener =
        TcpListener::bind(&sock_addr)
            .await
            .map_err(|source| RuntimeHostError::Bind {
                address: addr.clone(),
                source,
            })?;

    info!(addr = %addr, "backend listening");

    let std_listener = listener
        .into_std()
        .map_err(RuntimeHostError::ConvertListener)?;
    let server = axum::Server::from_tcp(std_listener)
        .map_err(|source| RuntimeHostError::CreateServer(source.to_string()))?;
    server
        .serve(router.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|source| RuntimeHostError::Serve(source.to_string()))?;

    info!(addr = %addr, "backend shut down");
    Ok(())
}

#[cfg(feature = "http")]
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            error!(error = %e, "failed to install Ctrl-C handler");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(e) => {
                error!(error = %e, "failed to install SIGTERM handler");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }

    info!("shutdown signal received, draining in-flight requests");
}

#[derive(Debug, Error)]
pub enum RuntimeHostError {
    #[error("{0}")]
    InvalidMode(String),
    #[error("runtime ingress module `{0}` is not compiled into appfw-runtime")]
    ModuleFeatureDisabled(&'static str),
    #[error("missing required environment variable {0}")]
    MissingEnv(String),
    #[error("invalid socket address {address}: {source}")]
    InvalidSocketAddress {
        address: String,
        source: std::net::AddrParseError,
    },
    #[error("failed to bind listener at {address}: {source}")]
    Bind {
        address: String,
        source: std::io::Error,
    },
    #[error("failed to convert listener: {0}")]
    ConvertListener(std::io::Error),
    #[error("failed to create server: {0}")]
    CreateServer(String),
    #[error("server error: {0}")]
    Serve(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_all_includes_all_compiled_modules() {
        let mode = RuntimeMode::all();
        #[cfg(feature = "chat")]
        assert!(mode.enables(RuntimeIngressKind::Chat));
        #[cfg(feature = "http")]
        assert!(mode.enables(RuntimeIngressKind::Http));
        #[cfg(feature = "mcp")]
        assert!(mode.enables(RuntimeIngressKind::Mcp));
        #[cfg(feature = "kafka")]
        assert!(mode.enables(RuntimeIngressKind::Kafka));
        #[cfg(feature = "sync")]
        assert!(mode.enables(RuntimeIngressKind::Sync));
        #[cfg(not(any(feature = "http", feature = "mcp", feature = "kafka", feature = "sync")))]
        assert!(mode.module_names().is_empty());
    }

    #[test]
    fn default_mode_prefers_listener_over_worker_modules() {
        let mode = RuntimeMode::default_mode();
        #[cfg(feature = "http")]
        {
            assert_eq!(mode, RuntimeMode::http());
        }
        #[cfg(all(not(feature = "http"), feature = "chat"))]
        {
            assert_eq!(mode, RuntimeMode::chat());
        }
        #[cfg(all(not(feature = "http"), feature = "mcp"))]
        {
            assert_eq!(mode, RuntimeMode::mcp());
        }
        #[cfg(all(not(any(feature = "http", feature = "mcp")), feature = "kafka"))]
        {
            assert_eq!(mode, RuntimeMode::consumers());
        }
        #[cfg(all(
            not(any(feature = "http", feature = "mcp", feature = "kafka")),
            feature = "sync"
        ))]
        {
            assert_eq!(mode, RuntimeMode::sync_workers());
        }
        #[cfg(not(any(feature = "http", feature = "mcp", feature = "kafka", feature = "sync")))]
        {
            assert_eq!(mode, RuntimeMode::all());
        }
    }

    #[test]
    fn parses_named_runtime_modes() {
        #[cfg(feature = "http")]
        assert_eq!(
            RuntimeMode::parse_mode("http").unwrap(),
            RuntimeMode::http()
        );
        #[cfg(feature = "chat")]
        assert_eq!(
            RuntimeMode::parse_mode("chat").unwrap(),
            RuntimeMode::chat()
        );
        #[cfg(feature = "mcp")]
        assert_eq!(RuntimeMode::parse_mode("mcp").unwrap(), RuntimeMode::mcp());
        #[cfg(feature = "kafka")]
        assert_eq!(
            RuntimeMode::parse_mode("consumers").unwrap(),
            RuntimeMode::consumers()
        );
        #[cfg(feature = "sync")]
        assert_eq!(
            RuntimeMode::parse_mode("sync").unwrap(),
            RuntimeMode::sync_workers()
        );
    }

    #[cfg(all(feature = "http", feature = "mcp"))]
    #[test]
    fn parses_module_list() {
        let mode = RuntimeMode::parse_modules("http,mcp").unwrap();
        #[cfg(feature = "http")]
        assert!(mode.enables(RuntimeIngressKind::Http));
        #[cfg(feature = "mcp")]
        assert!(mode.enables(RuntimeIngressKind::Mcp));
        #[cfg(feature = "kafka")]
        assert!(!mode.enables(RuntimeIngressKind::Kafka));
        assert_eq!(mode.module_names(), vec!["http", "mcp"]);
    }

    #[cfg(all(feature = "chat", feature = "http"))]
    #[test]
    fn chat_ingress_is_a_listener_surface() {
        let plan = RuntimeHostPlan::new(RuntimeMode::parse_modules("chat").unwrap());

        assert!(plan.serves_http_listener());
        assert_eq!(plan.module_names(), vec!["chat"]);
        assert!(plan.worker_module_names().is_empty());
    }

    #[cfg(all(feature = "http", feature = "mcp", feature = "kafka"))]
    #[test]
    fn host_plan_reports_http_and_worker_surfaces() {
        assert!(RuntimeMode::mcp().enables_http_listener());
        let plan = RuntimeHostPlan::new(RuntimeMode::parse_modules("http,mcp,kafka").unwrap());
        assert!(plan.serves_http_listener());
        assert_eq!(plan.worker_module_names(), vec!["kafka"]);
        assert!(plan.has_unsupported_worker_modules());
    }

    #[cfg(all(feature = "mcp", not(feature = "http")))]
    #[test]
    fn mcp_without_http_is_a_worker_surface() {
        let plan = RuntimeHostPlan::new(RuntimeMode::mcp());
        assert!(!plan.serves_http_listener());
        assert_eq!(plan.worker_module_names(), vec!["mcp"]);
    }

    #[cfg(feature = "kafka")]
    #[test]
    fn kafka_is_a_worker_surface() {
        let plan = RuntimeHostPlan::new(RuntimeMode::consumers());
        assert!(!plan.serves_http_listener());
        assert_eq!(plan.worker_module_names(), vec!["kafka"]);
        assert!(plan.runs_kafka_workers());
    }

    #[cfg(feature = "sync")]
    #[test]
    fn sync_is_a_worker_surface() {
        let plan = RuntimeHostPlan::new(RuntimeMode::sync_workers());
        assert!(!plan.serves_http_listener());
        assert_eq!(plan.worker_module_names(), vec!["sync"]);
        assert!(plan.runs_sync_workers());
    }

    #[cfg(all(feature = "http", feature = "sync"))]
    #[test]
    fn sync_worker_with_http_listener_is_reported() {
        let plan = RuntimeHostPlan::new(RuntimeMode::parse_modules("http,sync").unwrap());

        assert!(plan.serves_http_listener());
        assert!(plan.runs_sync_workers());
        assert!(plan.has_sync_worker_with_listener_modules());

        let all_plan = RuntimeHostPlan::new(RuntimeMode::parse_mode("all").unwrap());
        assert!(all_plan.has_sync_worker_with_listener_modules());
    }

    #[cfg(all(feature = "kafka", feature = "sync"))]
    #[test]
    fn multiple_worker_modules_are_reported() {
        let plan = RuntimeHostPlan::new(RuntimeMode::parse_modules("kafka,sync").unwrap());

        assert!(plan.has_multiple_worker_modules());
        assert_eq!(plan.worker_module_names(), vec!["kafka", "sync"]);
    }

    #[cfg(all(feature = "http", feature = "mcp"))]
    #[test]
    fn descriptor_requires_config_and_mode_enablement() {
        let mode = RuntimeMode::parse_modules("http").unwrap();
        assert!(
            RuntimeIngressDescriptor::new(RuntimeIngressKind::Http, "graphql", true)
                .enabled_in(&mode)
        );
        assert!(
            !RuntimeIngressDescriptor::new(RuntimeIngressKind::Mcp, "mcp", true).enabled_in(&mode)
        );
        assert!(
            !RuntimeIngressDescriptor::new(RuntimeIngressKind::Http, "graphql", false)
                .enabled_in(&mode)
        );
    }
}
