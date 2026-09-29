# Provider SDK Rules

Adding a provider is a framework change, not an application customization. CRUD
providers must enter through the shared contracts so generated applications
continue to get secure, observable, provider-neutral behavior. Read-only
analytics providers may reuse a CRUD provider's lower-level implementation, but
must declare write limitations explicitly. Graph read providers such as Neo4j
use the separate [Graph Read Providers](graph-read-providers.md) contract and
must not be forced into CRUD parity.

The machine-readable rule export is:

```bash
scripts/appfw explain provider-sdk --json
```

## Supported Provider Reference

The snippets below document the supported provider shapes. They intentionally
use placeholder hosts, database names, account names, users, secrets, and
tokens. Do not copy real tenant values, service-principal IDs, access tokens,
passwords, hostnames, database names, or account identifiers into docs,
checked-in config, generated examples, screenshots, logs, or handoff artifacts.

Credentials must come from the runtime secret mechanism named for each
provider. Checked-in `.appfw/model/data_sources/_res.yaml` files should contain
only topology and connection metadata.

### PostgreSQL

Use PostgreSQL for generated CRUD schemas, system metadata, migrations, seeds,
audit writes, and provider certification.

```yaml
- name: pg_primary
  data_source_type: PostgreSQL
  is_system_schema_host: true
  description: Primary PostgreSQL data source
  environments:
  - name: production
    db_host: <postgres-host>
    db_name: <postgres-database>
    db_port: 5432
    security_profile: managed
    tls_mode: verify_full
```

Runtime secrets:

```text
PG_SERVICE_ACCOUNT_NAME
PG_SERVICE_ACCOUNT_PASS
```

### MongoDB

Use MongoDB for generated CRUD schemas that intentionally use MongoDB-backed
storage and BSON/Mongo-compatible query semantics.

```yaml
- name: mongo_primary
  data_source_type: MongoDB
  description: Primary MongoDB data source
  environments:
  - name: production
    db_host: <mongo-host>
    db_name: <mongo-database>
    db_port: 27017
    security_profile: managed
    tls_mode: verify_full
```

Runtime secrets:

```text
MONGO_SERVICE_ACCOUNT_NAME
MONGO_SERVICE_ACCOUNT_PASS
```

For MongoDB Atlas, replica-set, or seed-list deployments, keep credentials in
the secret-backed runtime fields and put only the credential-free connection
shape in `db_host`:

```yaml
- name: mongo_primary
  data_source_type: MongoDB
  description: Atlas-backed MongoDB data source
  meta:
    classification: confidential
  environments:
  - name: production
    db_host: mongodb+srv://<atlas-cluster-host>
    db_name: <mongo-database>
    db_port: 27017 # retained for the shared data-source contract; ignored for URI hosts
    security_profile: managed
    tls_mode: verify_full
```

Seed-list and replica-set deployments use the same field:

```yaml
db_host: mongodb://<mongo-a>:27017,<mongo-b>:27017/admin?replicaSet=<replica-set-name>
```

The model validator rejects embedded credentials in MongoDB URIs. Use
`MONGO_SERVICE_ACCOUNT_NAME` and `MONGO_SERVICE_ACCOUNT_PASS` (or the platform
secret provider that supplies those values) so checked-in model config remains
portable, reviewable, and secret-free.

### MS SQL Server

Use `MsSqlServer` for generated CRUD schemas backed by SQL Server. This path is
separate from `FabricSqlAnalytics`. Plain `MsSqlServer` CRUD uses ODBC
execution (Microsoft ODBC Driver 18 for `sql_password`, FreeTDS for `ntlm`).
Authentication is selected per environment via `auth_mode`:

- `sql_password` (default) — SQL login via Microsoft ODBC Driver 18 from
  `MSSQL_SERVICE_ACCOUNT_*`
- `ntlm` — on-prem AD domain login via FreeTDS ODBC NTLM. Requires domain
  Windows username and password in `MSSQL_SERVICE_ACCOUNT_*` (format
  `DOMAIN\user` or `user@realm`). Runtime needs `freetds-bin`, `unixodbc`, and
  registered FreeTDS driver. Hermetic proof: `sql_password_odbc_e2e` on Linux
  SQL Server; `ntlm_e2e` on `tds-mock` (protocol-fidelity mock with
  `ntlm-auth` oracle — Linux SQL cannot accept remote NTLM). Scheduled/manual
  live smoke: `scripts/ci/mssql-ntlm-live-smoke.sh` against real Windows SQL
  (`MSSQL_NTLM_HOST` / `MSSQL_NTLM_USERNAME` / `MSSQL_NTLM_PASSWORD`, optional
  `MSSQL_NTLM_DATABASE` / `MSSQL_NTLM_PORT`). Optional `MSSQL_NTLM_SQL`
  overrides the default `SELECT 1 AS ok` and prints rows (needs `--nocapture`).
  Launch: unset/`native` (workstation FreeTDS, mode **c**) or
  `MSSQL_NTLM_SMOKE_LAUNCH=container` (FreeTDS test-runner image, mode **d**;
  short hostnames are resolved to IPv4 on the host for Docker Desktop DNS).
  Option A connection/auth certification; semantic live-cert remains the
  existing `sql_password` provider-parity pass. See `docker/mssql-ad/README.md`
  and `docs/specs/mssql-odbc-ntlm-authentication.md`.

In `appfw_provider_mssql`, ODBC connection config and shared T-SQL compilation
helpers remain in the common provider modules; Fabric-specific ODBC execution
lives under the `fabric` module.

```yaml
- name: mssql_primary
  data_source_type: MsSqlServer
  description: Primary MS SQL Server data source
  environments:
  - name: production
    db_host: sqlhost.corp.example.com
    db_name: <mssql-database>
    db_port: 1433
    security_profile: managed
    tls_mode: verify_full
    auth_mode: ntlm
```

Runtime secrets / env:

```text
MSSQL_AUTH_MODE                 # optional override; sql_password | ntlm
MSSQL_SERVICE_ACCOUNT_NAME      # SQL login or domain Windows username (ntlm)
MSSQL_SERVICE_ACCOUNT_PASS      # SQL password or domain password (ntlm)
```

Runtime ODBC packages (typical Linux): `msodbcsql18` + `unixodbc` for
`sql_password`; add `freetds-bin`, `freetds-dev`, and `tdsodbc` for `ntlm`.
`scripts/appfw doctor` reports advisory `mssql-freetds-driver` and
`mssql-odbc-driver18` checks when `odbcinst` is available. Generated
`podman-compose.yml` backend services install both driver stacks on first start
when MsSqlServer is an active provider (see `app_gen/src/dev_infra.rs`). Hermetic
ODBC connection proof: `sql_password_odbc_e2e` and `ntlm_e2e` (see
`docker/mssql-ad/README.md`). See `docs/specs/mssql-odbc-ntlm-authentication.md`.

### FabricSqlAnalytics

Use `FabricSqlAnalytics` for external read-only Microsoft Fabric reporting
schemas. It must not host generated system metadata, migrations, seeds, audit
writes, or generated mutations. Product schemas bound to this provider should
declare external read-only storage intent:

```yaml
id: <schema-uuid>
name: reporting
description: External reporting schema
data_source_name: fabric_reporting
meta:
  storage:
    mode: external_read_only
```

Data source example:

```yaml
- name: fabric_reporting
  data_source_type: FabricSqlAnalytics
  description: Microsoft Fabric SQL analytics endpoint
  meta:
    classification: confidential
  environments:
  - name: production
    db_host: <fabric-sql-endpoint-host>
    db_name: <fabric-database-or-lakehouse-name>
    db_port: 1433
    security_profile: managed
    tls_mode: verify_full
    auth_mode: entra_client_credentials
    entra_token_scope: https://database.windows.net/.default
```

Cloud runtime settings and secrets for machine-to-machine service-principal auth:

```text
FABRIC_TENANT_ID
FABRIC_CLIENT_ID
FABRIC_CLIENT_SECRET
FABRIC_TOKEN_SCOPE
```

`FABRIC_SQL_AUTH_MODE` may be omitted because Fabric defaults to
`entra_client_credentials`; if set, that is the only supported value. The service
principal must be allowed by tenant/workspace policy and granted read access to
the Fabric workspace, Warehouse, Lakehouse SQL analytics endpoint, or specific
objects required by the product.

For service-principal auth, the provider requests an Entra access token with the
client-credentials flow, caches the token with the `expires_in` value returned
by Entra, and refreshes it before opening a new physical Fabric ODBC connection
when the cached token is missing or expires within five minutes. The configured
`entra_token_scope`/`FABRIC_TOKEN_SCOPE` value is an OAuth v2 scope and must end
with `.default`; the default is `https://database.windows.net/.default`.

To smoke test service-principal token acquisition without printing the token,
export credentials into the local shell and run the ignored live test:

```bash
export FABRIC_TENANT_ID="<tenant-id>"
export FABRIC_CLIENT_ID="<client-id>"
read -s FABRIC_CLIENT_SECRET
export FABRIC_CLIENT_SECRET
export FABRIC_TOKEN_SCOPE="https://database.windows.net/.default"

cargo test -p appfw-provider-mssql \
  --test fabric_entra_client_credentials \
  fabric_entra_client_credentials_can_obtain_access_token \
  -- --ignored

unset FABRIC_CLIENT_SECRET
```

Fabric execution uses Microsoft ODBC Driver 18 for SQL Server. Runtime images
must install and register that driver. Override the driver name only when the
platform image registers a different name:

```text
APP_FABRIC_ODBC_DRIVER
```

Generated Fabric product clients import Fabric execution from
`appfw_provider_mssql::fabric` and reuse the common MS SQL T-SQL compiler
helpers for filters, ordering, aggregate projection, and parameter binding. This
keeps Fabric read behavior aligned with SQL Server without mixing Fabric ODBC
transport concerns into the plain `MsSqlServer` ODBC execution path.

### Snowflake

Use Snowflake for generated CRUD schemas or hosted analytical stores when the
product has a Snowflake account and token-based runtime credentials. Local
framework certification may use a Snowflake-compatible emulator, but product
cloud environments should use platform-managed secrets.

```yaml
- name: snowflake_primary
  data_source_type: Snowflake
  description: Primary Snowflake data source
  environments:
  - name: production
    db_host: <snowflake-account-host>
    db_name: <snowflake-database>
    db_port: 443
    security_profile: managed
    tls_mode: verify_full
```

Runtime secrets:

```text
SNOWFLAKE_SERVICE_ACCOUNT_NAME
SNOWFLAKE_ACCESS_TOKEN
SNOWFLAKE_OAUTH_TOKEN
SNOWFLAKE_JWT
SNOWFLAKE_SERVICE_ACCOUNT_PASS
SNOWFLAKE_AUTH_TOKEN_TYPE
```

Only one Snowflake token/password secret is required for a given environment;
the loader checks the supported token/password names in order.

### Neo4j

Use Neo4j as a graph-read provider, not as a generated CRUD schema host. Neo4j
topology belongs beside primary data sources, while generated entity schemas
stay on a CRUD-capable provider.

```yaml
- name: neo4j_graph
  data_source_type: Neo4j
  description: Optional Neo4j graph read data source
  environments:
  - name: production
    db_host: <neo4j-host>
    db_name: <neo4j-database>
    db_port: 7687
    security_profile: managed
    tls_mode: verify_full
```

Runtime secrets:

```text
NEO4J_SERVICE_ACCOUNT_NAME
NEO4J_SERVICE_ACCOUNT_PASS
```

See [Graph Read Providers](graph-read-providers.md) for Neo4j-specific query,
mutation, audit, and certification posture.

## Required Integration Points

Update provider identity everywhere:

- config contract allowed data-source values
- generated system schema data-source enum
- database loader and doctor
- backend connection construction
- provider key parsing for CLI, logs, metrics, and certification

Add provider implementation under:

```text
backend/src/data/clients/<provider>/
```

Graph read implementations belong in a graph-read package such as
`appfw_provider_neo4j` and are consumed through `RuntimeGraphProvider`, not
through `DataAccess` CRUD clients.

`FabricSqlAnalytics` is a read-only Microsoft Fabric SQL analytics endpoint
provider. It reuses the framework's T-SQL query/filter compiler, but it must
not reuse the plain `MsSqlServer` ODBC execution client. Fabric execution goes
through `odbc-api` and Microsoft ODBC Driver 18 for SQL Server with Microsoft
Entra authentication; normal `MsSqlServer` data sources use ODBC as well
(`sql_password` via Microsoft Driver 18, `ntlm` via FreeTDS). Generated
mutations, migrations, seed writes, and audit writes must fail closed for
Fabric. Fabric environments must use `auth_mode: entra_client_credentials`,
with Entra tenant/client/secret supplied at runtime.

The provider enables `odbc-api`'s vendored unixODBC driver-manager feature by
default so Linux/Kubernetes builds do not need system unixODBC headers or
libraries. Runtime images still need Microsoft ODBC Driver 18 for SQL Server
installed and registered so the driver name can resolve. Platform teams that
prefer OS-managed unixODBC, or that need a different license posture for
unixODBC's LGPL terms, can disable the provider default feature and install
unixODBC in the base image instead. Runtime can override the driver name with
`APP_FABRIC_ODBC_DRIVER`; the default is `ODBC Driver 18 for SQL Server`.

Provider clients should compile only from shared framework intent:

- `QueryIR`
- filter AST
- selection AST
- sort AST
- aggregate plan
- access and tenant filters

They must never concatenate user values into SQL, scripts, or command strings.
Use bound parameters or provider-native structured predicates.

Prepared statement and stored routine support must stay provider-owned, but
they are distinct features:

- Prepared statements optimize generated CRUD/query SQL and are enabled through
  `entity_type.execution.prepared_statements`.
- PostgreSQL may expose cached prepared statement execution through the
  provider execution client.
- MS SQL Server and Snowflake may expose parameterized statement execution, but
  must not claim reusable prepared-handle lifecycle support until the provider
  path owns that lifecycle explicitly.
- Stored procedures and functions must be invoked through typed provider helper
  structs that validate and quote routine identifiers, generate placeholders,
  and bind values with the provider's existing parameter API.
- Stored procedures may return `One`, `Many`, or `None` when the concrete
  provider routine supports that shape; PostgreSQL examples use a governed
  `jsonb` procedure result, while MS SQL Server and Snowflake use their native
  result-returning procedure forms.
- Do not add a generic runtime or product API for arbitrary SQL, `EXEC`, or
  `CALL`. Generated/custom operations should map to a declared routine contract
  and then call the provider helper.
- Diagnostics must redact routine arguments and must not log raw user values.

The CRM sample carries one generated prepared-query fixture and provider
routine fixtures for PostgreSQL, MS SQL Server, and Snowflake:

- `Account.execution.prepared_statements: true` demonstrates generated
  read/query SQL using provider-owned prepared/cached execution where
  supported.
- `Account.account_health_provider_function` demonstrates a query method that
  binds to a vetted provider function.
- `Account.refresh_account_health_stored_procedure` demonstrates a mutation
  method that binds to a vetted stored procedure, persists a health snapshot,
  and returns the row/object it wrote.

Product config declares the portable routine target once with
`provider_routine.schema` and `provider_routine.name`. Provider-specific
`routines.<provider>` entries are overrides for physical naming drift, not the
default path.

These examples are deliberately small. They prove the configuration shape,
generated handler dispatch, provider helper naming, and migration package
conventions without creating a generic SQL surface for product developers.

## Capability Declaration

Every provider must declare every `ProviderContractArea` in:

```text
appfw_runtime/src/provider_capabilities.rs
```

Allowed statuses:

- `LiveCertified`
- `CompilerContracted`
- `Implemented`
- `Partial`
- `Unsupported`
- `EmulatorLimited`

`LiveCertified` requires an executable live contract. `CompilerContracted`
requires an executable backend contract. Every non-certified status requires an
actionable reason.

## Error And Security Contract

Normalize provider-native errors before they reach GraphQL:

- duplicate key
- foreign-key violation
- missing required value
- stale version
- denied access
- invalid filter
- unknown provider failure

Logs and diagnostics must redact:

- credentials and access tokens
- raw connection strings
- bind values
- tenant-sensitive payloads
- provider-native command text unless a safe redacted diagnostic hook exists

## Observability Contract

The provider must emit:

- request/correlation IDs in errors and logs
- provider timing metrics
- slow-query events
- query-count and result-count fields
- pool stats or an explicit unsupported reason
- readiness checks per data source

`EXPLAIN` or diagnostic hooks default to unsupported. Only enable them after
proving output is redacted and does not include credentials, bind values, raw
tenant filters, or provider secrets.

## Migration Contract

Relational providers need versioned migration support:

- expand/backfill/contract phases
- linting
- drift checks
- migration registry and lock
- forward-only rollback guidance

Non-relational providers must either implement equivalent lifecycle hooks or
declare explicit limitations in deployment docs and provider capability output.

## Certification Checklist

Before a fifth provider can ship:

```bash
scripts/appfw validate --json
scripts/appfw test
scripts/appfw explain provider-sdk --json
scripts/appfw provider-test --provider <provider> --json
scripts/appfw release-check --json
```

The provider must not be release-blocking only because it is new: every
unsupported or partial area must be declared, reasoned, and visible in the
provider matrix.
