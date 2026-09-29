//! Dedicated Host IX binary. Mounts `ix_transport_routes` only.
//!
//! Do not run `backend/src/main.rs` as the Host proof process. This binary
//! does not nest `nexus_ix::get_routes` and does not expose `/nexus-ix`.

#[cfg(not(feature = "chat"))]
compile_error!("nexus_ix_server requires --features chat");

use std::path::PathBuf;

use appfw_runtime::host::{serve_http_router, RuntimeHttpServerConfig};
use appfw_runtime::ix::{IxClientProfile, IxStoredRunSealKey};
use appfw_runtime::observability::init_tracing;
use backend::{compose_host_ix, HostIxCompose};

#[tokio::main]
async fn main() {
    let _observability = init_tracing();
    if std::env::var("APP_BYPASS_POLICIES_IN_LOCAL").is_ok()
        || std::env::var("APP_ALLOW_MISSING_POLICIES_IN_LOCAL").is_ok()
    {
        eprintln!("Host proof forbids APP_BYPASS_POLICIES_IN_LOCAL and APP_ALLOW_MISSING_POLICIES_IN_LOCAL");
        std::process::exit(2);
    }

    let postgres_url = required_env("NEXUS_IX_PG_URL");
    let issuer = required_env("NEXUS_IX_ISSUER");
    let frontend_origin = std::env::var("NEXUS_IX_FRONTEND_ORIGIN")
        .unwrap_or_else(|_| "http://127.0.0.1:4173".to_string());
    let frontend_dist = std::env::var("APP_PRODUCT_UI_DIST_DIR")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from);
    let seal_key = seal_key_from_env();
    let host = std::env::var("API_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("API_PORT").unwrap_or_else(|_| "8088".to_string());

    let assembly = match compose_host_ix(HostIxCompose {
        postgres_url,
        issuer,
        frontend_origin,
        frontend_dist,
        seal_key,
        client_profile: IxClientProfile::Browser,
        proof_orchestrator_hold: None,
    })
    .await
    {
        Ok(assembly) => assembly,
        Err(error) => {
            eprintln!("host composition failed: {error}");
            std::process::exit(1);
        }
    };

    // Start facts persist through repository transactions. There is no
    // framework flush API (notes-000005). Do not run a disconnected
    // `let _ = sink.flush_to_postgres(...)` loop.

    let config = RuntimeHttpServerConfig { host, port };
    if let Err(error) = serve_http_router(assembly.router, &config).await {
        eprintln!("host serve failed: {error}");
        std::process::exit(1);
    }
}

fn required_env(name: &str) -> String {
    match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => value,
        _ => {
            eprintln!("missing required environment variable {name}");
            std::process::exit(2);
        }
    }
}

fn seal_key_from_env() -> IxStoredRunSealKey {
    let key_id = std::env::var("NEXUS_IX_SEAL_KEY_ID")
        .unwrap_or_else(|_| "nexus-ix-host-seal@1".to_string());
    let hex = match std::env::var("NEXUS_IX_SEAL_KEY_HEX") {
        Ok(value) if value.len() == 64 => value,
        _ => {
            eprintln!("NEXUS_IX_SEAL_KEY_HEX must be 64 hex characters (ephemeral, not retained)");
            std::process::exit(2);
        }
    };
    let mut key_bytes = [0_u8; 32];
    for (index, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let octet = std::str::from_utf8(chunk).ok().and_then(|value| u8::from_str_radix(value, 16).ok());
        match octet {
            Some(value) => key_bytes[index] = value,
            None => {
                eprintln!("NEXUS_IX_SEAL_KEY_HEX is not valid hex");
                std::process::exit(2);
            }
        }
    }
    IxStoredRunSealKey::new(key_id, key_bytes).unwrap_or_else(|error| {
        eprintln!("seal key rejected: {error}");
        std::process::exit(2);
    })
}
