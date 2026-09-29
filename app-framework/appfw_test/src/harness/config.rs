use std::{env, path::PathBuf};

use anyhow::{bail, Result};

const DEFAULT_BASE_URL: &str = "http://localhost:8080";
const DEFAULT_TIMEZONE: &str = "America/Denver";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthMode {
    Bypass,
    LocalDev,
    TokenFiles,
}

impl AuthMode {
    fn from_env_value(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "bypass" => Ok(Self::Bypass),
            "local_dev" | "local-dev" | "local" => Ok(Self::LocalDev),
            "token_files" | "token-files" | "files" => Ok(Self::TokenFiles),
            other => bail!(
                "unsupported API_TEST_AUTH_MODE `{other}`; expected `bypass`, `local_dev`, or `token_files`"
            ),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ApiTestConfig {
    pub base_url: String,
    pub timezone: String,
    pub auth_mode: AuthMode,
    pub token_dir: PathBuf,
}

impl ApiTestConfig {
    pub fn from_env() -> Result<Self> {
        let base_url =
            env::var("API_TEST_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
        let timezone =
            env::var("API_TEST_TIMEZONE").unwrap_or_else(|_| DEFAULT_TIMEZONE.to_string());
        let auth_mode = AuthMode::from_env_value(
            &env::var("API_TEST_AUTH_MODE").unwrap_or_else(|_| "bypass".to_string()),
        )?;
        let token_dir = env::var("API_TEST_TOKEN_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| default_token_dir());

        if base_url.trim().is_empty() {
            bail!("API_TEST_BASE_URL cannot be empty");
        }
        if timezone.trim().is_empty() {
            bail!("API_TEST_TIMEZONE cannot be empty");
        }

        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            timezone,
            auth_mode,
            token_dir,
        })
    }
}

fn default_token_dir() -> PathBuf {
    env::current_dir()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
        .join("test_tokens")
}

pub fn provider_certification_enabled() -> bool {
    env::var("API_TEST_PROVIDER_CERTIFICATION")
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

pub fn should_run_provider_certification(contract: &str) -> bool {
    if provider_certification_enabled() {
        return true;
    }

    eprintln!(
        "skipping {contract}; run scripts/appfw provider-test to enable provider certification"
    );
    false
}
