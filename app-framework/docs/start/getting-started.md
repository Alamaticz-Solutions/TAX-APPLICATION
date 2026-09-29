# Getting Started

This guide gets a clean checkout through validation, generation, local runtime,
and the first safe config edit.

## Prerequisites

- Rust stable with `cargo` and `rustc`.
- `git`, `rsync`, `diff`, and `shasum`.
- A configured data source when you want to run migrations or the backend.
- Optional local container tooling such as Docker or Podman for development
  databases.

Check your workstation from the repository root:

```bash
scripts/appfw doctor
```

## First Verification Loop

Validate `.appfw/model` without generating backend, database, or API test
artifacts. Validation does emit validation and config-contract reports:

```bash
scripts/appfw product validate
```

Generate checked-in artifacts:

```bash
scripts/appfw product generate
```

Run local compile and unit checks:

```bash
scripts/appfw product test
```

The normal handoff loop is:

```bash
scripts/appfw product validate --json
scripts/appfw product test
scripts/appfw product handoff --json
```

For generator/template work, run:

```bash
scripts/appfw framework validate --json
scripts/appfw framework generate
scripts/appfw framework generate --check --json
scripts/appfw framework test
scripts/appfw framework handoff --json
```

`generate --check` is a full deterministic generation comparison, so it is not
part of the first-run health check.

## Repository Shape

```text
.appfw/model/              # Application source config
app_gen/_templates/           # Generator templates
backend/src/                  # Generated and human-extended Rust backend
database/_pkg/                # Generated database package
api_tests/src/schemas/        # Generated API scenarios
docs/start/cli-quickstart.md        # First-contact command path
docs/reference/cli.md               # Full command contract
AGENTS.md                     # Agent operating guide
```

## Run The Backend

The backend reads runtime settings from environment variables and `.env` through
`dotenv`.

```bash
ENV_NAME=local API_PORT=8080 scripts/appfw product serve
```

GraphQL routes are schema-specific:

```text
http://localhost:8080/crm
http://localhost:8080/system
```

The route names are generated from schema names in `.appfw/model/schemas`.

## Local Snowflake Emulator

When app topology includes a Snowflake data source with a `compose`
environment, generated compose includes a Snowflake profile that uses LocalStack
so provider parity can be exercised without touching a hosted account.
LocalStack requires an auth token to start the emulator.

```bash
LOCALSTACK_AUTH_TOKEN=... docker compose -f podman-compose.yml --profile snowflake up snowflake snowflake-smoke
```

To point the backend at the emulator, use the Snowflake data source and the
LocalStack-compatible SQL API credentials:

```bash
ENV_NAME=compose \
APP_DATA_SOURCE_NAME=snowflake_primary \
SNOWFLAKE_HOST=http://snowflake.localhost.localstack.cloud:4566 \
SNOWFLAKE_ACCESS_TOKEN=test \
SNOWFLAKE_AUTH_TOKEN_TYPE=LOCALSTACK_NO_AUTH \
PG_HOST=127.0.0.1 \
API_PORT=8080 \
scripts/appfw product serve
```

Use hosted Snowflake by leaving `SNOWFLAKE_HOST` pointed at the account
identifier and supplying the production token type and token through secrets.

## Existing Example Config

The repo ships with a CRM schema:

```text
.appfw/model/schemas/crm/_res.yaml
.appfw/model/schemas/crm/entity_types/
.appfw/model/schemas/crm/gql_enum_types/
.appfw/model/schemas/crm/seeds/
.appfw/model/schemas/crm/tests/
```

Start by editing a small CRM config value, then run:

```bash
scripts/appfw product validate
scripts/appfw product generate
scripts/appfw product test
```

## Add A Minimal Schema

Create a schema directory:

```bash
mkdir -p .appfw/model/schemas/notes/entity_types
mkdir -p .appfw/model/schemas/notes/gql_enum_types
mkdir -p .appfw/model/schemas/notes/tests
```

Add the schema resource:

```yaml
# .appfw/model/schemas/notes/_res.yaml
id: 12345678-1234-1234-1234-123456789abc
name: notes
description: Notes schema
data_source_name: pg_primary
```

Add an entity:

```yaml
# .appfw/model/schemas/notes/entity_types/note.yaml
- name: Note
  id: 22222222-2222-2222-2222-222222222222
  is_table: true
  standard_methods: [FindById, GetAll, Query, Create, Update, Delete]
  props:
  - name: id
    fragment: property-primary-key-uuid
  - name: title
    is_required: true
    is_caption: true
    fragment: property-string-caption
  - name: body
    fragment: property-string
  - name: created_at
    is_required: true
    fragment: property-date-time-required
```

Validate before generating:

```bash
scripts/appfw product validate
```

If validation passes, generate and review the diff:

```bash
scripts/appfw product generate
git diff
```

## Generated Ownership

Generation emits:

```text
.appfw/target/appfw/validation.json
.appfw/target/appfw/config_contract.json
.appfw/target/appfw/artifacts.json
```

Use the artifact manifest before editing files that may be generated:

```bash
scripts/appfw product manifest --json
```

Files marked `generated` should be changed through `.appfw/model`,
`app_gen/src`, or `app_gen/_templates`. Files marked `human_owned` are
extension points that are created when missing and then preserved.

If the manifest is missing, use [Generated Ownership](generated-ownership.md)
before running generation only for reconnaissance.

## Next Steps

- Use [Schema Design](../model/schema-design.md) for config modeling.
- Use [Customization Decisions](../model/customization-decisions.md) before adding
  custom behavior.
- Use [CLI Quickstart](cli-quickstart.md) for the first command path and
  [CLI Reference](../reference/cli.md) for agents and CI.
- Use [Application Lifecycle](../lifecycle/application-lifecycle.md) when turning the
  framework into a product-specific app repo or planning an upstream upgrade.
