# Hermetic MS SQL ODBC certification lab

Used for Scenarios 1 (local) and 4 (CI) from
`docs/specs/mssql-odbc-ntlm-authentication.md`.

**Throwaway lab credentials only.** Seed `docker/mssql-ad/.env` from
`.env.example` (gitignored `.env`). The file only needs `MSSQL_SA_PASSWORD`
(sql_password e2e). NTLM mock principals and hosts are compose defaults —
**not** production secrets and must not be copied into product `.env` examples.

## Proof model (three tiers)

| Tier | Target | Tests | Gate |
| --- | --- | --- | --- |
| 1 | Linux SQL Server (`mssql`) | `sql_password_odbc_e2e` (Microsoft ODBC Driver 18) | Isolated CI |
| 2 | `tds-mock` (TDS/NTLM protocol-fidelity) | `ntlm_e2e`, `ntlm_wrong_password_fails_fast`, `golden_selftest` | Isolated CI |
| 3 | Real Windows SQL (on-prem / AWS RDS) | `ntlm_e2e` only via `scripts/ci/mssql-ntlm-live-smoke.sh` | Scheduled/manual |

Tier 2 mirrors the Snowflake/LocalStack emulator pattern:

- **Client:** production FreeTDS ODBC stack (same as runtime).
- **Oracle:** external `ntlm-auth` recomputes and verifies NTLMv2 proofs.
- **Anchor:** golden handshake vectors captured from a real server (`golden/handshake.json`).

`tds-mock` scope is **EmulatorLimited**: PRELOGIN, LOGIN7/SSPI NTLM handshake,
and `SELECT 1` batch/response. It does not emulate full SQL Server semantics.

Linux SQL Server **cannot** accept remote NTLM for Windows logins — NTLM
certification is intentionally against `tds-mock`, not the `mssql` service.

Option A certification: **connection/auth proof only** for NTLM. Existing mssql
`sql_password` live-cert remains the semantics proof — do not run a second
CRM/`provider-parity` suite under NTLM.

## Bring up

```bash
cd docker/mssql-ad
cp -n .env.example .env   # gitignored; throwaway lab passwords only
DOCKER_CONFIG=/tmp/empty-docker-config docker compose up -d --build --wait mssql tds-mock
```

Compose reads `docker/mssql-ad/.env` for `MSSQL_SA_PASSWORD` only. Commit
`.env.example` only — never commit `.env`.

Services:

- `mssql.appfw.test` — **stock** SQL Server 2022 image (no custom Dockerfile)
- `tds-mock.appfw.test` — thin Python mock image (`tds-mock/Dockerfile`)
- `test-runner` — Rust + FreeTDS + Microsoft ODBC Driver 18 (CI driver host)

## Run ODBC auth e2e (sql_password + ntlm)

```bash
DOCKER_CONFIG=/tmp/empty-docker-config docker compose --profile test run --rm test-runner
```

Or manually:

```bash
DOCKER_CONFIG=/tmp/empty-docker-config docker compose --profile test run --rm test-runner \
  bash -lc 'cargo test -p appfw-provider-mssql --test sql_password_odbc_e2e --test ntlm_e2e -- --ignored --nocapture'
```

Full isolated CI entrypoint: `scripts/ci/mssql-odbc-ntlm-e2e.sh` (Bitbucket custom
`mssql-odbc-ntlm-e2e`).

## Golden capture (repeatable)

Capture handshake metadata from a **real** Windows SQL Server when SQL Server
major version or domain NTLM policy changes:

```bash
export MSSQL_NTLM_HOST=your-sql-host
export MSSQL_NTLM_USERNAME='DOMAIN\throwaway-user'
export MSSQL_NTLM_PASSWORD='throwaway-password'
bash scripts/ci/mssql-ntlm-golden-capture.sh
```

Prerequisites:

- DBA-created throwaway domain login (not production credentials).
- FreeTDS `isql` on the capture host (`freetds-bin`).
- `TDSDUMP` log is reduced by `tds-mock/tools/extract_golden.py` — credentials
  and hashes are stripped; only challenge flags, token sequence, and optional
  target-info structure are retained.

Output: `docker/mssql-ad/tds-mock/golden/handshake.json` (sanitized).

## Live smoke (outside isolated gate)

```bash
export MSSQL_NTLM_HOST=your-windows-sql-host
export MSSQL_NTLM_USERNAME='DOMAIN\svc-account'
export MSSQL_NTLM_PASSWORD='...'
export MSSQL_NTLM_DATABASE=PDS   # optional; default master
# optional query override (default: SELECT 1 AS ok); printed with --nocapture
export MSSQL_NTLM_SQL="SELECT TOP 5 * FROM PDS.dbo.elig_Audit"

# Mode c analogue — native FreeTDS on the workstation
bash scripts/ci/mssql-ntlm-live-smoke.sh

# Mode d analogue — same credentials/SQL inside the FreeTDS test-runner image
# (first run builds `appfw-mssql-ntlm-smoke:local`; uses docker run --network host).
# Short hostnames are resolved to IPv4 on the host first (Docker Desktop DNS often
# lacks the WSL search domain, which caused "Unknown host machine name").
MSSQL_NTLM_SMOKE_LAUNCH=container bash scripts/ci/mssql-ntlm-live-smoke.sh
```

Runs `ntlm_connect_and_query` only — never part of the hermetic certification gate.
Product modes **c** / **d** (`ENV_NAME=dev` + `product serve` vs `podman-compose up backend`)
use the same `auth_mode: ntlm` YAML; this smoke switches native vs container launch via
`MSSQL_NTLM_SMOKE_LAUNCH`.

## Evidence

Successful `ntlm_e2e` against `tds-mock` writes sanitized handshake evidence to
`/evidence/ntlm-handshake.json` inside the mock container. CI copies it to
`target/appfw/ntlm-handshake.json` (no secrets or hashes).

## Status (2026-08-05)

Working:

- Stock SQL Server 2022 for `sql_password_odbc_e2e`
- `tds-mock` with real NTLMv2 verification (`ntlm-auth` oracle)
- FreeTDS NTLM ODBC connect as `APPFW\svc-app` via `ntlm_e2e`
- Wrong-password fail-fast regression (`ntlm_wrong_password_fails_fast`)
- Microsoft ODBC Driver 18 `sql_password` connect as `sa`

Lab principals (compose defaults; not in `.env`):

- Domain: `APPFW`
- NTLM user: `APPFW\svc-app` / `AppSvc!Passw0rd` (mock only)
- Mock host: `tds-mock.appfw.test`
- SQL host: `mssql.appfw.test` (`sa` / `MSSQL_SA_PASSWORD` from `.env`)
