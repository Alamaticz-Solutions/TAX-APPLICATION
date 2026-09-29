//! Live Host vertical proofs against real PostgreSQL and in-process JWTs.
//!
//! Fail-closed: `NEXUS_IX_PG_PROOF_URL` must name a reachable database.
//! No mocks, fixtures, intercepts, or skip-as-pass.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use appfw_runtime::ix::{
    IxAuditProjectionState, IxClientProfile, IxPrincipalBinding, IxRunState, IxStoredRunSealKey,
    IxTerminalOutcome, IX_CANCEL_PATH, IX_CANCEL_REQUEST_SCHEMA_VERSION,
    IX_RUN_REQUEST_SCHEMA_VERSION, IX_STREAM_PATH,
};
use backend::{
    compose_host_ix, ensure_my_work_projection, HostIxCompose, HOST_AUDIENCE, HOST_CLIENT_ID,
    HOST_RELEASE_ID, HOST_ROLE, HOST_SCOPE,
};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use rsa::pkcs1::EncodeRsaPrivateKey;
use rsa::traits::PublicKeyParts;
use rsa::RsaPrivateKey;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_postgres::NoTls;

const PROOF_URL_ENV: &str = "NEXUS_IX_PG_PROOF_URL";

fn require_pg_url() -> String {
    match std::env::var(PROOF_URL_ENV) {
        Ok(url) if !url.trim().is_empty() => url,
        _ => panic!(
            "{PROOF_URL_ENV} must be set for ix_real_vertical (fail-closed; skip-as-pass is FAIL)"
        ),
    }
}

fn seal_key() -> IxStoredRunSealKey {
    let mut bytes = [7_u8; 32];
    bytes[0] = 0x11;
    bytes[31] = 0x22;
    IxStoredRunSealKey::new("nexus-ix-host-seal@1", bytes).expect("seal key")
}

struct IssuedTokens {
    issuer: String,
    encoding_key: EncodingKey,
}

async fn serve_jwks() -> IssuedTokens {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    let private = RsaPrivateKey::new(&mut rand::thread_rng(), 2048).expect("ephemeral RSA");
    let public = private.to_public_key();
    let pem = private
        .to_pkcs1_pem(Default::default())
        .expect("ephemeral PEM");
    let encoding_key = EncodingKey::from_rsa_pem(pem.as_bytes()).expect("encoding key");
    let jwks = json!({
        "keys": [{
            "kty": "RSA",
            "alg": "RS256",
            "kid": "ix-runtime-generated",
            "use": "sig",
            "e": URL_SAFE_NO_PAD.encode(public.e().to_bytes_be()),
            "n": URL_SAFE_NO_PAD.encode(public.n().to_bytes_be())
        }]
    })
    .to_string();
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("JWKS bind");
    let issuer = format!("http://{}", listener.local_addr().expect("JWKS addr"));
    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            let mut request = [0_u8; 4096];
            let _ = socket.read(&mut request).await;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{jwks}",
                jwks.len()
            );
            let _ = socket.write_all(response.as_bytes()).await;
        }
    });
    IssuedTokens {
        issuer,
        encoding_key,
    }
}

fn sign_token(issued: &IssuedTokens, subject: &str, extra: Value) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("epoch")
        .as_secs();
    let mut claims = json!({
        "iss": issued.issuer,
        "sub": subject,
        "exp": now + 3600,
        "nbf": now.saturating_sub(1),
        "aud": HOST_AUDIENCE,
        "cid": HOST_CLIENT_ID,
        "tenant": "tenant_a",
        "roles": [HOST_ROLE],
        "scp": HOST_SCOPE,
        "principalType": "human",
        "releaseId": HOST_RELEASE_ID,
        "sessionId": "session-1"
    });
    if let Some(object) = extra.as_object() {
        claims
            .as_object_mut()
            .expect("claims")
            .extend(object.clone());
    }
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some("ix-runtime-generated".to_string());
    jsonwebtoken::encode(&header, &claims, &issued.encoding_key).expect("ephemeral JWT")
}

async fn apply_nexus_ix_ddl(url: &str) {
    let (client, connection) = tokio_postgres::connect(url, NoTls)
        .await
        .expect("ddl connection");
    let task = tokio::spawn(connection);
    client
        .batch_execute("CREATE SCHEMA IF NOT EXISTS nexus_ix;")
        .await
        .expect("schema");
    let tables = include_str!("../../database/_pkg/schemas/nexus_ix/tables.pg.sql");
    client.batch_execute(tables).await.expect("nexus_ix tables");
    let expand = include_str!(
        "../../database/_pkg/migrations/postgresql/20260816180220__nexus_ix_ix_real_vertical_constraints.expand.sql"
    );
    client
        .batch_execute(expand)
        .await
        .expect("expand constraints");
    drop(client);
    task.abort();
}

struct BoundServer {
    base: String,
    addr: std::net::SocketAddr,
    handle: tokio::task::JoinHandle<()>,
}

async fn bind_router(router: axum::Router) -> BoundServer {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("host bind");
    let addr = listener.local_addr().expect("addr");
    let handle = tokio::spawn(async move {
        axum::Server::from_tcp(listener.into_std().expect("std listener"))
            .expect("server")
            .serve(router.into_make_service())
            .await
            .expect("serve");
    });
    BoundServer {
        base: format!("http://{addr}"),
        addr,
        handle,
    }
}

async fn assert_first_server_terminated(addr: std::net::SocketAddr) {
    for _ in 0..20 {
        match tokio::net::TcpStream::connect(addr).await {
            Err(_) => return,
            Ok(_) => tokio::time::sleep(Duration::from_millis(25)).await,
        }
    }
    panic!("first bind_router server still accepted connections at {addr}");
}

fn start_body() -> Value {
    json!({
        "schemaVersion": IX_RUN_REQUEST_SCHEMA_VERSION,
        "intentKey": "pds.ix.intent.attention-stewardship@1",
        "focus": { "kind": "nexus.work-queue", "id": "my-work" }
    })
}

async fn read_sse_until_idle(
    response: reqwest::Response,
) -> (Vec<Value>, Option<String>, Option<String>) {
    let mut events = Vec::new();
    let mut first_cursor = None;
    let mut cursor = None;
    let text = response.text().await.unwrap_or_default();
    let mut current_id = None;
    for block in text.split("\n\n") {
        let mut data = Vec::new();
        for line in block.lines() {
            if let Some(id) = line.strip_prefix("id:") {
                current_id = Some(id.trim().to_string());
            } else if let Some(payload) = line.strip_prefix("data:") {
                data.push(payload.trim_start());
            }
        }
        if data.is_empty() {
            continue;
        }
        let joined = data.join("\n");
        if joined == "heartbeat" {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(&joined) {
            events.push(value);
        }
        if let Some(id) = current_id.take() {
            if first_cursor.is_none() {
                first_cursor = Some(id.clone());
            }
            cursor = Some(id);
        }
    }
    (events, first_cursor, cursor)
}

async fn compose_native(
    url: &str,
    issuer: &str,
    proof_orchestrator_hold: Option<Arc<tokio::sync::Notify>>,
) -> (backend::HostIxAssembly, BoundServer) {
    apply_nexus_ix_ddl(url).await;
    let assembly = compose_host_ix(HostIxCompose {
        postgres_url: url.to_string(),
        issuer: issuer.to_string(),
        frontend_origin: "http://127.0.0.1:4173".to_string(),
        frontend_dist: None,
        seal_key: seal_key(),
        client_profile: IxClientProfile::Native,
        proof_orchestrator_hold,
    })
    .await
    .expect("host composition");
    ensure_my_work_projection(&assembly.repository)
        .await
        .expect("proof-only my-work projection");
    let bound = bind_router(assembly.router.clone()).await;
    (assembly, bound)
}

const STALE_DECOY_RUN_ID: &str = "stale-proof-decoy-000007";

async fn list_run_ids(url: &str) -> Vec<String> {
    let (client, connection) = tokio_postgres::connect(url, NoTls)
        .await
        .expect("run-id list connection");
    let task = tokio::spawn(connection);
    let rows = client
        .query("SELECT id FROM nexus_ix.ix_runs", &[])
        .await
        .expect("list ix_runs");
    let ids = rows.iter().map(|row| row.get::<_, String>(0)).collect();
    drop(client);
    task.abort();
    ids
}

async fn retain_stale_decoy(url: &str) -> String {
    let (client, connection) = tokio_postgres::connect(url, NoTls)
        .await
        .expect("decoy connection");
    let task = tokio::spawn(connection);
    client
        .execute(
            "INSERT INTO nexus_ix.ix_runs \
             (id, tenant, subject, intent_key, run_state, created_at, updated_at) \
             VALUES ($1, 'tenant_a', 'stale.user', \
              'pds.ix.intent.attention-stewardship@1', 'stale_decoy', \
              now() - interval '1 hour', now() - interval '1 hour') \
             ON CONFLICT (id) DO UPDATE SET run_state = 'stale_decoy'",
            &[&STALE_DECOY_RUN_ID],
        )
        .await
        .expect("retain stale decoy");
    drop(client);
    task.abort();
    STALE_DECOY_RUN_ID.to_string()
}

async fn wait_for_new_run_id(url: &str, baseline: &[String]) -> String {
    let baseline: HashSet<&str> = baseline.iter().map(String::as_str).collect();
    let (client, connection) = tokio_postgres::connect(url, NoTls)
        .await
        .expect("new-run connection");
    let task = tokio::spawn(connection);
    for _ in 0..80 {
        if let Ok(rows) = client.query("SELECT id FROM nexus_ix.ix_runs", &[]).await {
            for row in rows {
                let id: String = row.get(0);
                if !baseline.contains(id.as_str()) {
                    drop(client);
                    task.abort();
                    return id;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    drop(client);
    task.abort();
    panic!("start did not persist a run id absent from the pre-start baseline");
}

fn owner_binding() -> IxPrincipalBinding {
    IxPrincipalBinding {
        tenant_id: "tenant_a".to_string(),
        subject: "nexus.user".to_string(),
        principal_type: appfw_runtime::extension::RuntimePrincipalType::User,
        on_behalf_of: None,
    }
}

fn assert_cancel_receipt_audit(body: &Value, run_id: &str) {
    assert_eq!(body["runId"], run_id);
    assert_eq!(body["commandId"], "nexus-cancel-live-1");
    assert_eq!(
        body["auditRetained"], true,
        "cancel receipt must retain audit: {body}"
    );
    assert_eq!(
        body["auditProjected"], "pending",
        "accepted cancel receipt stays pending: {body}"
    );
}

async fn assert_owner_scoped_audit_identity(
    url: &str,
    run_id: &str,
    command_id: &str,
    cursor: &str,
    expected_dedup_key: &str,
) {
    let (client, connection) = tokio_postgres::connect(url, NoTls)
        .await
        .expect("audit-identity connection");
    let task = tokio::spawn(connection);
    let mut projection = None;
    for _ in 0..80 {
        if let Ok(row) = client
            .query_opt(
                "SELECT id, command_id, cursor, projection_state \
                 FROM nexus_ix.ix_projection_cursors \
                 WHERE tenant = $1 AND run_id = $2 AND command_id = $3 AND cursor = $4",
                &[&"tenant_a", &run_id, &command_id, &cursor],
            )
            .await
        {
            if let Some(row) = row {
                projection = Some((
                    row.get::<_, String>(0),
                    row.get::<_, String>(1),
                    row.get::<_, String>(2),
                    row.get::<_, String>(3),
                ));
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let (dedup_key, stored_command, stored_cursor, state) =
        projection.expect("owner-scoped cancel audit projection must persist for the same run");
    assert_eq!(stored_command, command_id);
    assert_eq!(stored_cursor, cursor);
    assert!(
        matches!(state.as_str(), "pending" | "projected"),
        "projection_state {state}"
    );
    assert!(!dedup_key.is_empty(), "dedup key must identify the cancel");
    assert_eq!(
        dedup_key, expected_dedup_key,
        "ix_projection_cursors.id must equal accepted_cancel.audit_projection.dedup_key for {run_id}"
    );
    // Packet 000009: run_requested is the always-present start fact and is
    // not the cancel/disposition identity. Do not query ix_audit_facts as a
    // green path. The live durable cancel identity is the projection row.
    drop(client);
    task.abort();
}

async fn assert_decoy_untouched(url: &str, decoy: &str) {
    let (client, connection) = tokio_postgres::connect(url, NoTls)
        .await
        .expect("decoy-check connection");
    let task = tokio::spawn(connection);
    let row = client
        .query_one(
            "SELECT run_state FROM nexus_ix.ix_runs WHERE id = $1",
            &[&decoy],
        )
        .await
        .expect("decoy row");
    let state: String = row.get(0);
    assert_eq!(
        state, "stale_decoy",
        "decoy {decoy} must remain uncancelled"
    );
    drop(client);
    task.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_sse_stream_resume_cancel_jwt_restart_and_trait_dispatch() {
    let url = require_pg_url();
    let issued = serve_jwks().await;
    let hold = Arc::new(tokio::sync::Notify::new());
    let (assembly, first_server) =
        compose_native(&url, &issued.issuer, Some(Arc::clone(&hold))).await;
    let base = first_server.base.clone();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .expect("http client");
    let bearer = sign_token(&issued, "nexus.user", json!({}));

    let unauthenticated = client
        .post(format!("{base}{IX_STREAM_PATH}"))
        .json(&start_body())
        .send()
        .await
        .expect("unauthenticated request");
    assert_eq!(unauthenticated.status().as_u16(), 401);

    let rejected = sign_token(
        &issued,
        "nexus.user",
        json!({ "aud": "api://wrong", "cid": HOST_CLIENT_ID }),
    );
    let forbidden = client
        .post(format!("{base}{IX_STREAM_PATH}"))
        .header("authorization", format!("Bearer {rejected}"))
        .json(&start_body())
        .send()
        .await
        .expect("rejected jwt");
    assert!(
        forbidden.status().as_u16() == 401 || forbidden.status().as_u16() == 403,
        "signed-JWT reject must fail closed, got {}",
        forbidden.status()
    );

    let decoy = retain_stale_decoy(&url).await;
    let baseline = list_run_ids(&url).await;
    assert!(
        baseline.iter().any(|id| id == &decoy),
        "pre-start baseline must contain the stale decoy so newest-row polling cannot look green"
    );

    let start_task = {
        let client = client.clone();
        let base = base.clone();
        let bearer = bearer.clone();
        tokio::spawn(async move {
            client
                .post(format!("{base}{IX_STREAM_PATH}"))
                .header("authorization", format!("Bearer {bearer}"))
                .header("accept", "text/event-stream")
                .json(&start_body())
                .send()
                .await
        })
    };
    let run_id = wait_for_new_run_id(&url, &baseline).await;
    assert_ne!(
        run_id, decoy,
        "cancel must bind to the run created by this held start, not the decoy"
    );

    let cancel_path = IX_CANCEL_PATH.replace(":run_id", &run_id);
    let cancel_body = json!({
        "schemaVersion": IX_CANCEL_REQUEST_SCHEMA_VERSION,
        "commandId": "nexus-cancel-live-1"
    });
    let first = client
        .post(format!("{base}{cancel_path}"))
        .header("authorization", format!("Bearer {bearer}"))
        .json(&cancel_body)
        .send()
        .await
        .expect("active cancel");
    assert_eq!(
        first.status().as_u16(),
        202,
        "active cancel must be accepted, got {}",
        first.status()
    );
    let first_body: Value = first.json().await.expect("cancel body");
    assert_eq!(first_body["disposition"], "accepted");
    assert_cancel_receipt_audit(&first_body, &run_id);
    let cancel_cursor = first_body.get("cursor").cloned();

    let second = client
        .post(format!("{base}{cancel_path}"))
        .header("authorization", format!("Bearer {bearer}"))
        .json(&cancel_body)
        .send()
        .await
        .expect("cancel retry");
    assert!(
        second.status().as_u16() == 202 || second.status().as_u16() == 200,
        "idempotent cancel {}",
        second.status()
    );
    let second_body: Value = second.json().await.expect("cancel retry body");
    assert_cancel_receipt_audit(&second_body, &run_id);
    if let Some(cursor) = &cancel_cursor {
        assert_eq!(second_body.get("cursor"), Some(cursor));
    }
    assert!(
        matches!(
            second_body["disposition"].as_str(),
            Some("accepted") | Some("already_requested") | Some("already_terminal")
        ),
        "retry disposition {}",
        second_body
    );

    assert_decoy_untouched(&url, &decoy).await;
    let cancelled = assembly
        .repository
        .load_stored_run(&owner_binding(), &run_id)
        .await
        .expect("load cancelled run")
        .expect("held start persisted the cancelled run");
    assert_eq!(cancelled.snapshot().run_id, run_id);
    let accepted = cancelled
        .accepted_cancel()
        .expect("accepted cancel must persist on the new run");
    assert_eq!(accepted.command_id(), "nexus-cancel-live-1");
    if let Some(Value::String(cursor)) = first_body.get("cursor") {
        assert_eq!(accepted.cursor(), cursor);
    }

    hold.notify_waiters();
    let started = start_task.await.expect("start join").expect("start send");
    assert!(
        started.status().is_success(),
        "start status {}",
        started.status()
    );
    assert!(started
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .contains("text/event-stream"));
    let (events, first_cursor, tip_cursor) = read_sse_until_idle(started).await;
    assert!(
        events.iter().any(|event| {
            event.pointer("/payload/type").and_then(Value::as_str) == Some("run_acknowledged")
        }),
        "live SSE must include run_acknowledged: {events:?}"
    );
    let ack_cursor = first_cursor.expect("ack cursor");
    let tip_cursor = tip_cursor.expect("tip cursor");
    assert!(ack_cursor.starts_with("ix1."), "ack cursor {ack_cursor}");
    assert!(tip_cursor.starts_with("ix1."), "tip cursor {tip_cursor}");

    let after_release = assembly
        .repository
        .load_stored_run(&owner_binding(), &run_id)
        .await
        .expect("load after hold release")
        .expect("released run must remain owner-scoped");
    assert_eq!(after_release.snapshot().run_id, run_id);
    assert_eq!(
        after_release.snapshot().state,
        IxRunState::Terminal,
        "post-release state must be Terminal, got {:?}",
        after_release.snapshot().state
    );
    assert_eq!(
        after_release.snapshot().terminal_outcome,
        Some(IxTerminalOutcome::Cancelled),
        "post-release outcome must be Cancelled, got {:?}",
        after_release.snapshot().terminal_outcome
    );
    let accepted = after_release
        .accepted_cancel()
        .expect("accepted cancel must remain on the same run after release");
    assert_eq!(accepted.command_id(), "nexus-cancel-live-1");
    if let Some(Value::String(cursor)) = first_body.get("cursor") {
        assert_eq!(accepted.cursor(), cursor);
    }
    let projection = accepted.audit_projection();
    assert_eq!(projection.tenant_id(), "tenant_a");
    assert_eq!(projection.run_id(), run_id);
    assert_eq!(projection.command_id(), "nexus-cancel-live-1");
    assert_eq!(projection.cursor(), accepted.cursor());
    assert!(
        !projection.dedup_key().is_empty(),
        "stored cancel audit projection must have a dedup key"
    );
    assert!(
        matches!(
            accepted.audit_projection_state(),
            IxAuditProjectionState::Pending | IxAuditProjectionState::Projected
        ),
        "stored audit projection state {:?}",
        accepted.audit_projection_state()
    );
    assert_owner_scoped_audit_identity(
        &url,
        &run_id,
        accepted.command_id(),
        accepted.cursor(),
        projection.dedup_key(),
    )
    .await;

    // Framework resume is exclusive of last-event-id. Replay from the ack
    // cursor (first envelope), not the tip — matching
    // appfw_runtime/tests/ix_transport_orchestration.rs.
    let resumed = client
        .post(format!("{base}{IX_STREAM_PATH}"))
        .header("authorization", format!("Bearer {bearer}"))
        .header("accept", "text/event-stream")
        .header("last-event-id", &ack_cursor)
        .send()
        .await
        .expect("resume");
    assert!(resumed.status().is_success(), "resume {}", resumed.status());
    let (replayed, _, _) = read_sse_until_idle(resumed).await;
    assert!(
        replayed
            .iter()
            .any(|event| event.get("runId").and_then(Value::as_str) == Some(run_id.as_str())),
        "resume must replay the same run: {replayed:?}"
    );

    let loaded = assembly
        .repository
        .load_stored_run(&owner_binding(), &run_id)
        .await
        .expect("load after runtime create_run")
        .expect("trait-dispatch persisted the run");
    assert_eq!(loaded.snapshot().run_id, run_id);
    assert_eq!(loaded.snapshot().state, IxRunState::Terminal);
    assert_eq!(
        loaded.snapshot().terminal_outcome,
        Some(IxTerminalOutcome::Cancelled)
    );

    first_server.handle.abort();
    assert_first_server_terminated(first_server.addr).await;
    drop(assembly);
    let restarted = compose_host_ix(HostIxCompose {
        postgres_url: url,
        issuer: issued.issuer.clone(),
        frontend_origin: "http://127.0.0.1:4173".to_string(),
        frontend_dist: None,
        seal_key: seal_key(),
        client_profile: IxClientProfile::Native,
        proof_orchestrator_hold: None,
    })
    .await
    .expect("restart composition with stable seal key");
    let restart_bound = bind_router(restarted.router.clone()).await;
    let restart_base = restart_bound.base.clone();
    let replay = client
        .post(format!("{restart_base}{IX_STREAM_PATH}"))
        .header("authorization", format!("Bearer {bearer}"))
        .header("accept", "text/event-stream")
        .header("last-event-id", &ack_cursor)
        .send()
        .await
        .expect("restart replay");
    assert!(
        replay.status().is_success(),
        "restart/replay {}",
        replay.status()
    );
    let (after_restart, _, _) = read_sse_until_idle(replay).await;
    assert!(
        after_restart
            .iter()
            .any(|event| event.get("runId").and_then(Value::as_str) == Some(run_id.as_str())),
        "stable seal key must replay the stored run"
    );
    let _ = Arc::clone(&restarted.service);
}
