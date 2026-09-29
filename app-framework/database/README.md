# Database Package Runner

The `database` crate executes generated database package artifacts from:

```text
database/_pkg/
```

Run it from the repository root:

```bash
scripts/appfw migrate
```

Versioned migration utilities:

```bash
scripts/appfw migrate doctor
scripts/appfw migrate new --schema crm --phase backfill --name normalize_contacts
scripts/appfw migrate plan --json
scripts/appfw migrate lint --phase all --json
scripts/appfw migrate rollback-guide --json
scripts/appfw migrate status --json
scripts/appfw migrate apply --json
```

## When To Run

Run database package execution after generation and before backend/API scenario
tests when local data sources need to be prepared:

```bash
scripts/appfw generate
scripts/appfw migrate
```

Run the backend in a separate terminal:

```bash
ENV_NAME=local API_PORT=8080 scripts/appfw serve
```

Then run API scenarios:

```bash
scripts/appfw api-test
```

## Runtime Configuration

Common local variables:

```text
RUST_LOG=info
ENV_NAME=local
DATABASE_LOG_JSON=1
DATABASE_OTEL_ENABLED=1
PG_SERVICE_ACCOUNT_NAME=...
PG_SERVICE_ACCOUNT_PASS=...
MONGO_SERVICE_ACCOUNT_NAME=...
MONGO_SERVICE_ACCOUNT_PASS=...
```

Use `ENV_NAME=local` for the database runner on the host with Docker/Podman
ports published to localhost. The generated `compose` environment is reserved
for services running inside `podman-compose.yml`.

Connection metadata comes from generated data source config:

```text
database/_pkg/data_sources.yaml
```

Each data source environment declares `security_profile` and `tls_mode`.
`local_dev` is limited to `local` and `compose`; managed environments must use
TLS.

Do not commit real credentials.

## Observability

Set `DATABASE_LOG_JSON=1` for structured JSON logs. Use `RUST_LOG=info` or
`RUST_LOG=database=info` to emit the database utility's INFO-level events. Set
`DATABASE_OTEL_ENABLED=1` to enable OpenTelemetry spans using the standard OTEL
exporter environment variables. `OTEL_SERVICE_NAME` defaults to `database` when
unset.

Each run writes local operational metrics for migration attempts, durations,
statuses, providers, and data sources:

```text
target/appfw/database_metrics.json
target/appfw/database_metrics.prom
```

Versioned migration lint and drift reports are written beside those files. See
`database/MIGRATIONS.md` for the zero-downtime workflow.
