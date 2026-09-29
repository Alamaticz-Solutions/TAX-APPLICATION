use std::env;

use appfw_provider_mssql::{EntraTokenProvider, FABRIC_SQL_ANALYTICS_DEFAULT_TOKEN_SCOPE};

const TENANT_ID_ENV: &str = "FABRIC_TENANT_ID";
const CLIENT_ID_ENV: &str = "FABRIC_CLIENT_ID";
const CLIENT_SECRET_ENV: &str = "FABRIC_CLIENT_SECRET";
const TOKEN_SCOPE_ENV: &str = "FABRIC_TOKEN_SCOPE";

#[test]
#[ignore = "requires live Microsoft Entra service-principal credentials"]
fn fabric_entra_client_credentials_can_obtain_access_token() {
    let tenant_id = required_env(TENANT_ID_ENV);
    let client_id = required_env(CLIENT_ID_ENV);
    let client_secret = required_env(CLIENT_SECRET_ENV);
    let token_scope = env::var(TOKEN_SCOPE_ENV)
        .unwrap_or_else(|_| FABRIC_SQL_ANALYTICS_DEFAULT_TOKEN_SCOPE.to_string());

    let provider =
        EntraTokenProvider::client_credentials(tenant_id, client_id, client_secret, token_scope);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");

    let access_token = runtime
        .block_on(provider.access_token())
        .expect("service principal should obtain a Fabric SQL analytics Entra access token");

    assert!(
        !access_token.trim().is_empty(),
        "Entra returned an empty access token"
    );
}

fn required_env(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("set {name} to run this live smoke test"))
}
