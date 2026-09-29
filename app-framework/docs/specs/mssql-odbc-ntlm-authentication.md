# MS SQL Server ODBC + NTLM (On-Prem AD) Authentication

Status: accepted (hermetic Option A auth/connection certification proven locally;
production partner NTLM on Linux remains a deployment concern)

Spec depth: lightweight

Owner roles:

- Product Owner: bruce.ly (PDS Health)
- Architect: bruce.ly (PDS Health)
- Implementation owner: Coding Agent
- Review owner: Framework PR Review Agent + human approver (bruce.ly)

## Business Value

Product apps must authenticate to domain-joined MS SQL Server databases as an
Active Directory identity from Linux/Kubernetes runtimes. Epic-hosted Clarity
and similar estates are not Azure-Arc-eligible, so Entra ID is not an option.
NTLM domain login via FreeTDS ODBC is the partner-supported path on Linux when
Kerberos ticket delivery is impractical and a stored domain password is
acceptable under security review.

## Problem

Plain `MsSqlServer` previously relied on `tiberius` with a planned Kerberos
GSSAPI path. On Linux, `tiberius` had no supported NTLM execution path, and the
Kerberos approach required ambient ticket caches, keytabs, and `libgssapi`
patches. The framework now unifies plain `MsSqlServer` CRUD on ODBC: Microsoft
ODBC Driver 18 for `sql_password`, FreeTDS for `ntlm`.

## Goals

- Keep `auth_mode: sql_password` on Microsoft ODBC Driver 18 for plain
  `MsSqlServer`.
- Add `auth_mode: ntlm` as a permanent, first-class auth mode for plain
  `MsSqlServer`, backed by FreeTDS ODBC NTLM connection strings.
- Prove NTLM connection/auth hermetically via the `tds-mock` protocol-fidelity
  emulator (`ntlm_e2e`, `ntlm_wrong_password_fails_fast`) in local dev and
  isolated CI (`scripts/ci/mssql-odbc-ntlm-e2e.sh`) — Option A connection/auth
  certification only; no second full CRM/`provider-parity` semantic suite under
  NTLM. Real Windows SQL targets are covered by scheduled/manual
  `scripts/ci/mssql-ntlm-live-smoke.sh` outside the isolated gate.
- Prove `sql_password` ODBC connect hermetically via `sql_password_odbc_e2e`
  (Microsoft ODBC Driver 18 + SA login) in the same lab/CI script; semantic
  live-cert remains the existing mssql `provider-parity` pass.
- Extend `doctor`/`validate` with NTLM preflight (domain username + password
  secrets) and keep CLI/docs contracts aligned.
- Certify at the same bar as other main providers: semantic live-cert remains
  the existing mssql `sql_password` `provider-parity` pass; NTLM proves
  handshake and credential acquisition only (ADR 0018).

## Non-Goals

- Entra ID generalization to plain `MsSqlServer` (separate effort).
- Per-user AD identity passthrough to the DB (pooling / delegation).
- Kerberos / GSSAPI / ambient ticket-cache auth modes (superseded by this
  ODBC + NTLM path).
- Production Kubernetes secret-delivery design (platform-owned).
- Downstream app K8s manifests/Dockerfiles.

## Scope

`appfw_provider_mssql` (ODBC execution, `connection.rs`, `odbc/`, tests);
`appfw_mssql_auth` registry; `app_gen` validation and config contract;
`database/src` loaders/doctor/migrations; `scripts/appfw` provider
certification; `docker/mssql-ad/` NTLM lab; `scripts/ci/mssql-odbc-ntlm-e2e.sh`;
docs (`docs/reference/cli.md`, `docs/runtime/provider-sdk.md`,
`docs/release/deployment-reference.md`, `docs/release/evidence-matrix.md`).

## Driver Dependencies

| Auth mode | ODBC driver | Runtime packages (typical Linux) |
| --- | --- | --- |
| `sql_password` | Microsoft ODBC Driver 18 for SQL Server | `msodbcsql18`, `unixodbc` |
| `ntlm` | FreeTDS | `freetds-bin`, `freetds-dev`, `unixodbc` |

Override driver name with `APP_FABRIC_ODBC_DRIVER` only for Microsoft Driver 18
paths; NTLM always uses the registered FreeTDS driver name (`FreeTDS`).

## Contracts Touched

| Contract Surface | Change | Counterpart alignment |
| --- | --- | --- |
| `MssqlAuthConfig` | `Ntlm` variant; ODBC connect for all plain MsSqlServer modes | `connection.rs`, `odbc/mod.rs` |
| `data_source.yaml` `auth_mode` | `ntlm` replaces `kerberos_integrated` | `appfw_mssql_auth`, `app_gen` validation, loaders |
| `appfw validate` / `doctor` | NTLM uses `SqlServiceAccount` preflight (domain user + password) | `docs/reference/cli.md`, docs-check |
| Auth-mode certification | Hermetic `sql_password_odbc_e2e` + `ntlm_e2e` (Option A) | `evidence-matrix.md`, ADR 0018 |
| CI | `mssql-odbc-ntlm-e2e` custom pipeline step (`mssql-odbc-ntlm-cert-step.sh`) | `bitbucket-pipelines.yml`, `scripts/ci/` |

## Decision Provenance

| Date | Owner | Decision | Rationale |
| --- | --- | --- | --- |
| 2026-08-04 | Architect | Unify plain MsSqlServer on ODBC; NTLM via FreeTDS | Partner NTLM on Linux; removes Tiberius / libgssapi debt |
| 2026-07-29 | Product Owner | Option A: certify NTLM as connection/auth only | Semantic proof already on `sql_password` live-cert (ADR 0018) |
| 2026-08-04 | Product Owner | `ntlm` is a standard co-equal auth mode for future apps | Reusable framework capability; Clarity is first consumer |

## Architecture Notes

One downstream path (ODBC session, pool, execution, audit, migrations). Credential
acquisition differs: `sql_password` → Microsoft Driver 18 SQL login;
`ntlm` → FreeTDS with `UID`/`PWD` and `TDS_Version=7.4`. Domain username format
is `DOMAIN\user` or `user@realm`. `db_host` may be hostname or IP for NTLM
(unlike Kerberos SPN matching). Hermetic lab: `docker/mssql-ad/` with stock
Linux SQL Server for `sql_password` and `tds-mock` for NTLM (FreeTDS client +
`ntlm-auth` oracle + golden anchor). Default mock principal: `APPFW\svc-app`.

## Local Developer Workflow

Use one `data_sources/_res.yaml` with multiple environments; `ENV_NAME` selects
the active environment at process start (no separate NTLM config shape).

| Mode | Target | Launch | `ENV_NAME` |
| --- | --- | --- | --- |
| **b** Docker Linux SQL + `sql_password` | Generated `podman-compose` mssql | `podman-compose up` | `compose` (default) |
| **c** NTLM to Windows SQL (daily dev) | Remote domain-joined SQL (`dev`/`tst`/`stg`) | `ENV_NAME=dev scripts/appfw product serve` (native) | `dev` (or `tst`/`stg`) |
| **d** NTLM container pre-commit check | Same remote SQL as **c** | `ENV_NAME=dev podman-compose up backend` | `dev` (override via `${ENV_NAME:-compose}` in generated compose) |

**c vs d:** identical data-source YAML (`auth_mode: ntlm`, `DOMAIN\user` secrets).
FreeTDS always handles NTLM; the difference is launch mode (native workstation vs
generated backend container). Mode **d** verifies container packaging (FreeTDS +
unixODBC in the backend image, DNS/network to the Windows SQL host).

Connection-only live smoke (outside the product serve path): set
`MSSQL_NTLM_*` plus optional `MSSQL_NTLM_SQL` (default `SELECT 1 AS ok`), then
`bash scripts/ci/mssql-ntlm-live-smoke.sh` for native (**c** analogue) or
`MSSQL_NTLM_SMOKE_LAUNCH=container bash scripts/ci/mssql-ntlm-live-smoke.sh`
for container FreeTDS packaging (**d** analogue).

Example `dev` environment (docs only — add to your product model when needed):

```yaml
- name: dev
  db_host: win-sql.dev.example.com
  db_port: 1433
  db_name: appdb
  auth_mode: ntlm
  service_account_name: DEVDOMAIN\svc-app
  security_profile: managed
  tls_mode: require
```

Hermetic NTLM protocol proof (no Windows SQL required): local lab
`bash scripts/ci/mssql-odbc-ntlm-e2e.sh` or `docker/mssql-ad` compose with
`tds-mock`.

## Security And Governance

- Change class D for auth + provider graduation: independent review and human
  approval required.
- NTLM stores a domain password in the platform secret mechanism — classify
  and rotate per security policy; not equivalent to ticket-based Kerberos.
- No PHI/PII in framework scope; lab credentials in `docker/mssql-ad/.env` are
  throwaway only.

## Acceptance Evidence

| Criterion | Proof | Required before |
| --- | --- | --- |
| `sql_password` on ODBC Driver 18 | `sql_password_odbc_e2e` + provider unit tests + existing mssql live-cert | merge |
| `ntlm` hermetic e2e | `ntlm_e2e` + `ntlm_wrong_password_fails_fast` against `tds-mock`; `golden_selftest`; retained `target/appfw/ntlm-handshake.json` | PR |
| Live Windows SQL smoke | `scripts/ci/mssql-ntlm-live-smoke.sh` (scheduled/manual) | deployment |
| Evidence matrix records auth-proven NTLM vs semantic live-cert | `docs/release/evidence-matrix.md` | merge |
| CLI contract aligned | `scripts/appfw framework docs-check --json` | merge |
| Independent review | framework-pr-review `GO` / `GO WITH CONDITIONS` | merge |

## Test Plan

Three proof tiers (mirrors Snowflake/LocalStack emulator certification):

- **Tier 1 (hermetic sql_password):** `sql_password_odbc_e2e` against stock Linux
  SQL Server + Microsoft ODBC Driver 18.
- **Tier 2 (hermetic NTLM mock):** `ntlm_e2e`, `ntlm_wrong_password_fails_fast`,
  and `golden_selftest` against `tds-mock` (FreeTDS client, `ntlm-auth` oracle,
  golden anchor). CI: `scripts/ci/mssql-odbc-ntlm-cert-step.sh` (pipeline
  `mssql` service + in-process `tds-mock`). Local lab:
  `scripts/ci/mssql-odbc-ntlm-e2e.sh` (`docker/mssql-ad` compose).
- **Tier 3 (live smoke):** `scripts/ci/mssql-ntlm-live-smoke.sh` against real
  on-prem/AWS Windows SQL — scheduled/manual, never in the isolated gate.

Unit tests: `cargo test -p appfw-provider-mssql`.

Existing mssql `sql_password` `provider-parity.json` live-cert remains the
semantic proof; do not re-run full parity under NTLM.

## Risks And Controls

| Risk | Control | Status |
| --- | --- | --- |
| NTLM disabled by SQL Server policy | Document partner requirement; security review for stored password | open |
| FreeTDS / ODBC packaging on runtime image | Document driver deps; doctor optional checks | mitigated in lab |
| Mock fidelity limit (`tds-mock` EmulatorLimited) | Declare scope (PRELOGIN, NTLM handshake, `SELECT 1` only); golden capture from real server; live smoke on Windows SQL | mitigated |

## Tech Debt And Follow-Up

- **TD-006 retired:** Tiberius and vendored `libgssapi` removed with ODBC
  unification.
- Latent Entra client-credentials token-expiry wiring — future Entra effort.
- Promote `mssql-odbc-ntlm-e2e` to PR/main gate after consecutive CI greens.

## Handoff Notes

Hermetic proof (local): `cd docker/mssql-ad && docker compose --profile test run --rm
test-runner`. CI entrypoint: `scripts/ci/mssql-odbc-ntlm-cert-step.sh`. Local lab
compose: `scripts/ci/mssql-odbc-ntlm-e2e.sh`. Live smoke:
`scripts/ci/mssql-ntlm-live-smoke.sh`. Golden capture:
`scripts/ci/mssql-ntlm-golden-capture.sh`. See ADR 0018 for connection-level vs
semantic certification split.
