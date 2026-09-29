//! Hermetic ODBC `sql_password` e2e for plain MsSqlServer.
//!
//! Connection proof via Microsoft ODBC Driver 18 (SQL login UID/PWD). Requires
//! a reachable SQL Server with the driver installed — the `mssql-ad` lab SA
//! login works when `msodbcsql18` is present in the test-runner image.
//!
//! ```bash
//! cargo test -p appfw-provider-mssql --test sql_password_odbc_e2e -- --ignored --nocapture
//! ```

use std::env;

use appfw_provider_mssql::{MssqlConnectionConfig, MssqlExecutionClient};
use appfw_runtime::connection_security::{ConnectionSecurity, Provider, SecurityProfile, TlsMode};

#[test]
#[ignore = "requires reachable SQL Server + Microsoft ODBC Driver 18"]
fn sql_password_odbc_connect_and_query() {
    let cfg = connection_from_env();
    let security = local_security();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio");

    runtime.block_on(async {
        let client = MssqlExecutionClient::from_connection_config(&cfg, &security)
            .await
            .expect("sql_password ODBC pool init");
        let rows = client
            .run_query("SELECT 1 AS ok", vec![])
            .await
            .expect("sql_password ODBC connect+query");
        assert!(!rows.is_empty(), "expected at least one row from SELECT 1");
    });
}

/// Regression for release-gate migrate expand: multi-statement batches can
/// surface zero-column DONE_IN_PROC results before the SELECT. Fetching those
/// previously failed with SQLSTATE 24000 Invalid cursor state.
#[test]
#[ignore = "requires reachable SQL Server + Microsoft ODBC Driver 18"]
fn sql_password_odbc_skips_zero_column_batch_results() {
    let cfg = connection_from_env();
    let security = local_security();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio");

    runtime.block_on(async {
        let client = MssqlExecutionClient::from_connection_config(&cfg, &security)
            .await
            .expect("sql_password ODBC pool init");
        let rows = client
            .run_query(
                r#"
SET NOCOUNT OFF;
BEGIN TRAN;
DECLARE @t TABLE (id int);
INSERT INTO @t (id) VALUES (1);
SELECT id AS acquired FROM @t;
COMMIT TRAN;
"#,
                vec![],
            )
            .await
            .expect("multi-statement ODBC query should skip DONE results");
        assert_eq!(
            rows.len(),
            1,
            "expected the SELECT result after non-result statements"
        );
        let acquired = rows[0]
            .try_get::<i32, _>("acquired")
            .expect("acquired column")
            .expect("acquired value");
        assert_eq!(acquired, 1);
    });
}

fn connection_from_env() -> MssqlConnectionConfig {
    MssqlConnectionConfig::new(
        required_env("MSSQL_ODBC_HOST"),
        env::var("MSSQL_ODBC_PORT").unwrap_or_else(|_| "1433".to_string()),
        env::var("MSSQL_ODBC_DATABASE").unwrap_or_else(|_| "master".to_string()),
        Some(required_env("MSSQL_ODBC_USERNAME")),
        Some(required_env("MSSQL_ODBC_PASSWORD")),
    )
}

fn local_security() -> ConnectionSecurity {
    ConnectionSecurity {
        provider: Provider::MsSqlServer,
        security_profile: SecurityProfile::LocalDev,
        tls_mode: TlsMode::Prefer,
    }
}

fn required_env(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("set {name} for sql_password_odbc_e2e"))
}
