//! Hermetic NTLM e2e for plain MsSqlServer (`auth_mode: ntlm`).
//!
//! Connection/auth proof via FreeTDS ODBC against the `tds-mock` service in
//! `docker/mssql-ad` (protocol-fidelity mock with `ntlm-auth` oracle). Real
//! Windows SQL targets are covered by `scripts/ci/mssql-ntlm-live-smoke.sh`.
//!
//! Optional `MSSQL_NTLM_SQL` overrides the default `SELECT 1 AS ok` and prints
//! result rows (requires `--nocapture`).
//!
//! ```bash
//! export MSSQL_NTLM_HOST=win-sql.example.com
//! export MSSQL_NTLM_USERNAME='DOMAIN\svc-account'
//! export MSSQL_NTLM_PASSWORD='...'
//! export MSSQL_NTLM_DATABASE=PDS
//! # optional:
//! export MSSQL_NTLM_SQL="SELECT TOP 5 * FROM PDS.dbo.elig_Audit"
//! cargo test -p appfw-provider-mssql --test ntlm_e2e ntlm_connect_and_query \
//!   -- --ignored --nocapture
//! ```

use std::env;
use std::time::{Duration, Instant};

use appfw_provider_mssql::{MssqlConnectionConfig, MssqlExecutionClient};
use appfw_runtime::connection_security::{ConnectionSecurity, Provider, SecurityProfile, TlsMode};

#[test]
#[ignore = "requires docker/mssql-ad tds-mock NTLM stack"]
fn ntlm_connect_and_query() {
    let cfg = connection_from_env();
    let security = local_security();
    let sql = query_sql_from_env();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio");

    runtime.block_on(async {
        let client = MssqlExecutionClient::from_connection_config(&cfg, &security)
            .await
            .expect("NTLM ODBC pool init");
        eprintln!("running SQL: {sql}");
        let rows = client
            .run_query(&sql, vec![])
            .await
            .expect("NTLM connect+query");
        eprintln!("rows: {}", rows.len());
        for (i, row) in rows.iter().enumerate() {
            eprintln!("row[{i}]: {row:?}");
        }
        if is_default_smoke_sql(&sql) {
            assert!(!rows.is_empty(), "expected at least one row from SELECT 1");
        }
    });
}

#[test]
#[ignore = "requires docker/mssql-ad tds-mock NTLM stack"]
fn ntlm_wrong_password_fails_fast() {
    let cfg = connection_from_env_with_password("definitely-wrong-password");
    let security = local_security();
    let login_timeout_secs = login_timeout_secs_from_env();
    let max_elapsed = Duration::from_secs(login_timeout_secs + 5);

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio");

    let started = Instant::now();
    let result = runtime.block_on(async {
        let client = MssqlExecutionClient::from_connection_config(&cfg, &security).await?;
        client.connection().await.map(|_| ())
    });
    let elapsed = started.elapsed();

    let err = match result {
        Err(e) => e,
        Ok(_) => panic!("wrong password must fail pool init"),
    };
    assert!(
        elapsed < max_elapsed,
        "expected fail-fast within login timeout ({login_timeout_secs}s + 5s slack), got {:?}",
        elapsed
    );
    let message = err.to_string();
    assert!(
        !message.contains("Timed out in bb8"),
        "expected ODBC/login diagnostic, not pool hang: {message}"
    );
    let lower = message.to_lowercase();
    assert!(
        lower.contains("login")
            || lower.contains("authentication")
            || lower.contains("integrated")
            || lower.contains("18456")
            || lower.contains("odbc"),
        "expected auth failure diagnostic in error: {message}"
    );
}

fn connection_from_env() -> MssqlConnectionConfig {
    connection_from_env_with_password(required_env("MSSQL_NTLM_PASSWORD"))
}

fn connection_from_env_with_password(password: impl Into<String>) -> MssqlConnectionConfig {
    MssqlConnectionConfig::new_ntlm(
        required_env("MSSQL_NTLM_HOST"),
        env::var("MSSQL_NTLM_PORT").unwrap_or_else(|_| "1433".to_string()),
        env::var("MSSQL_NTLM_DATABASE").unwrap_or_else(|_| "master".to_string()),
        Some(required_env("MSSQL_NTLM_USERNAME")),
        Some(password.into()),
    )
}

fn query_sql_from_env() -> String {
    env::var("MSSQL_NTLM_SQL").unwrap_or_else(|_| DEFAULT_SMOKE_SQL.to_string())
}

const DEFAULT_SMOKE_SQL: &str = "SELECT 1 AS ok";

fn is_default_smoke_sql(sql: &str) -> bool {
    sql.trim().eq_ignore_ascii_case(DEFAULT_SMOKE_SQL)
}

fn login_timeout_secs_from_env() -> u64 {
    env::var("APP_MSSQL_LOGIN_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value| *value > 0)
        .unwrap_or(30)
}

fn local_security() -> ConnectionSecurity {
    ConnectionSecurity {
        provider: Provider::MsSqlServer,
        security_profile: SecurityProfile::LocalDev,
        tls_mode: TlsMode::Prefer,
    }
}

fn required_env(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("set {name} for ntlm_e2e"))
}
