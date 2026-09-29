# Relationship-Aware Provider Certification

Relationship-aware provider certification is the first release-grade gate for
App Framework. A schema should expose the same GraphQL behavior whether it is
backed by PostgreSQL, MongoDB, MS SQL Server, or Snowflake. Provider
differences are allowed only when they are declared with a reason and covered by
a capability entry. `FabricSqlAnalytics` is included as a separate read-only
SQL analytics provider with compiler-contracted read/query behavior and
explicitly unsupported writes until live Fabric certification exists. Neo4j is
intentionally outside this CRUD semantic parity matrix; use
[Graph Read Providers](graph-read-providers.md) for its read-model posture and
future certification path. External API/SaaS providers are also outside this
matrix; use [SaaS Connector Certification](saas-certification.md) for the
`SaasReadArea` posture and live-evidence rules.

The executable capability source lives in:

```text
appfw_runtime/src/provider_capabilities.rs
```

Product backends keep only a thin compatibility facade at
`backend/src/data/clients/provider_capabilities.rs` while provider/client
extraction continues. The reusable provider certification report semantics
also live in the runtime package:

```text
appfw_runtime/src/provider_certification.rs
```

The provider certification CLI exporter is framework-owned and consumes runtime
provider keys directly, so product backends no longer carry a certification
adapter or exporter binary:

```text
appfw_cli/src/bin/provider_certification_export.rs
```

The local compiler/unit gate is:

```bash
scripts/appfw test
```

The live runtime gate is:

```bash
scripts/appfw provider-test --provider postgres
scripts/appfw provider-test --all --json
```

The framework release gate is:

```bash
scripts/appfw framework local-live-preflight --json
scripts/appfw release-check --json
```

`release-check` runs validation, generated drift checks, local tests, and the
full provider certification matrix. Use `provider-test` directly while
iterating on one provider; use `release-check` before cutting a framework
release.
Use `framework local-live-preflight --json` before pushing provider/runtime or
release-gate changes for review: it starts local provider services, migrates
all four providers, launches four local CRM backend instances, runs full
provider parity, and collects live security certification from the retained
provider logs. A green local preflight is PR/CI-readiness evidence; it does not
replace the remote release lane's production release authority.

`provider-test` expects each backend under test to already be running with the
matching schema-specific provider override, for example
`APP_CRM_DATA_SOURCE_NAME=pg_primary`. Prefer schema-specific overrides for
release certification so framework/system-owned surfaces stay on their intended
data source; reserve the global `APP_DATA_SOURCE_NAME` override for deliberate
whole-app local smoke runs. For `--all`, CI can expose separate backend URLs
with:

```text
API_TEST_BASE_URL_POSTGRES
API_TEST_BASE_URL_MONGO
API_TEST_BASE_URL_MSSQL
API_TEST_BASE_URL_SNOWFLAKE
```

The JSON report is written under the framework-root certification crate:

```text
api_tests/target/provider-parity.json
```

During `scripts/appfw release-check --json`, the same report is copied into
the release artifact directory:

```text
target/appfw/provider-parity.json
```

The report is area-driven. Each provider entry includes every certification
area with its declared status, the live contract used when applicable, and the
live result (`passed`, `failed`, `listed`, `not-run`, or `not-required`).
For `LiveCertified` areas, only `passed` satisfies certification. `listed` and
`not-run` are reported for diagnostics but fail the gate.
Security contracts also require an auth mode that can exercise named policy
users. `provider-test` sets `API_TEST_PROVIDER_CERTIFICATION=1` and defaults
`API_TEST_AUTH_MODE` to `local_dev`; using `bypass` under certification fails
instead of silently skipping denied-access coverage. Token-backed certification
lanes must provide equivalent `pdsh_admin`, `cc_tenant_user`,
`tenant_one_crm_ops`, and `west_sales_rep` tokens. `tenant_one_crm_ops` is the
tenant-1 `crm_ops` certification persona used to read governed AccountAudit
evidence; it must remain distinct from the denied `cc_tenant_user` mutation
actor. Tenant-isolation certification also requires `other_tenant_admin`.

Certification statuses mean:

| Status | Meaning |
| --- | --- |
| `LiveCertified` | A live provider-neutral API contract exists and must pass for this provider. |
| `CompilerContracted` | A backend compiler/unit contract exists, but no live provider contract exists yet. |
| `Implemented` | Provider code exists, but the behavior is not yet certified by a shared contract. |
| `Partial` | Some support exists, but the contract area is intentionally incomplete. |
| `Unsupported` | The provider does not claim support for this area. |
| `EmulatorLimited` | Hosted/provider behavior is implemented, but local emulator coverage is not release-equivalent. |

## Canonical Semantics

Ambiguous behavior is a framework contract, not a provider choice.

| Area | Canonical behavior |
| --- | --- |
| Pagination | `skip >= 0` and `limit > 0` are required at the shared query IR boundary. Invalid values return validation errors before provider compilation. |
| Null equality | `_eq: null` and `_ne: null` use provider-native null semantics and must not be silently dropped. |
| String contains | `_contains`, `_starts`, and `_ends` are case-sensitive unless a future operator explicitly says otherwise. |
| Date/time | Date period operators compile through the shared filter contract; provider timezone behavior must be covered before certification. |
| Arrays | Array operators are only certified for property types that explicitly support array semantics. |
| Missing relationships | To-one projections return null; to-many and many-to-many projections return an empty list. |
| Duplicate many-to-many links | Junction uniqueness should reject duplicates with a normalized error. |
| Constraint errors | Duplicate key, FK violation, missing required value, stale version, denied access, invalid filter, and unknown field errors must map to stable framework errors. |

## Contract Areas

| Area | Required behavior |
| --- | --- |
| Scalar filters | Scalar equality, comparison, string-pattern, membership, boolean conjunction, null, date/time period, and invalid-shape behavior are consistent. |
| Relationship filtering | To-one and to-many relationship filters preserve cardinality and access semantics. |
| Sorting | Field normalization, default sort behavior, invalid sort shape, and unknown field rejection are consistent. |
| Pagination | Valid paging is stable and invalid edge values fail before provider compilation. |
| Native projection | Selected native fields are returned without provider-specific leakage. |
| Relationship projection | To-one/to-many expansion, null relationship behavior, and missing rows are consistent. |
| Many-to-many projection | Junction-backed expansion is consistent from either side of the relationship. |
| Many-to-many mutation | Generated inputs and provider writes create/delete junction rows consistently. |
| Many-to-many filtering | Junction traversal filters return the same row set. |
| Aggregation | Grouping, metrics, having, sorting, paging, and invalid aggregate specs are consistent. |
| Aggregate filters | Aggregate row filters and having filters compose consistently. |
| Prepared statement execution | Provider statement execution uses bound values; native prepared-handle lifecycle is declared only where the provider path supports it. |
| Stored routine invocation | Stored procedures/functions are invoked only through provider-owned identifier validation, quoting, and bound arguments. |
| Access filters | Policy filters compose with user filters and mutation keys without widening access. |
| Tenant isolation | Tenant-scoped entities add a required tenant filter and never leak rows across policy-authorized tenants. |
| Error normalization | Validation, access denied, invalid key/version, and provider-native errors map to stable GraphQL error classes/messages. |
| Concurrency | Update and delete reject stale versions and apply exactly one matching record. |
| Audit | Audited mutations append redacted, chained audit events consistently. |

## Current Matrix

| Provider | Scalar filters | Rel filters | Sorting | Pagination | Native proj | Rel proj | M2M proj | M2M mutation | M2M filtering | Aggregation | Agg filters | Prepared stmt | Stored routines | Access filters | Tenant isolation | Errors | Concurrency | Audit |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| PostgreSQL | LiveCertified | LiveCertified | LiveCertified | LiveCertified | LiveCertified | LiveCertified | LiveCertified | Partial | Unsupported | LiveCertified | LiveCertified | Implemented | CompilerContracted | LiveCertified | LiveCertified | LiveCertified | LiveCertified | LiveCertified |
| MongoDB | LiveCertified | LiveCertified | LiveCertified | LiveCertified | LiveCertified | LiveCertified | LiveCertified | Unsupported | LiveCertified | LiveCertified | LiveCertified | Unsupported | Unsupported | LiveCertified | LiveCertified | LiveCertified | LiveCertified | LiveCertified |
| MS SQL Server | LiveCertified | LiveCertified | LiveCertified | LiveCertified | LiveCertified | LiveCertified | LiveCertified | Partial | Unsupported | LiveCertified | LiveCertified | Partial | CompilerContracted | LiveCertified | LiveCertified | LiveCertified | LiveCertified | LiveCertified |
| FabricSqlAnalytics | CompilerContracted | CompilerContracted | CompilerContracted | Implemented | CompilerContracted | CompilerContracted | CompilerContracted | Unsupported | Unsupported | CompilerContracted | CompilerContracted | Partial | Partial | CompilerContracted | CompilerContracted | CompilerContracted | Unsupported | Unsupported |
| Snowflake | LiveCertified | LiveCertified | LiveCertified | LiveCertified | LiveCertified | LiveCertified | LiveCertified | Partial | Unsupported | EmulatorLimited | EmulatorLimited | Partial | CompilerContracted | LiveCertified | LiveCertified | LiveCertified | LiveCertified | LiveCertified |

Implemented, partial, unsupported, and emulator-limited statuses must have a
reason in `appfw_runtime/src/provider_capabilities.rs`. `LiveCertified` and
`CompilerContracted` statuses must point at executable evidence. The current reasons are
intentionally explicit:

- MongoDB relationship projection/filtering and many-to-many projection/filtering
  compile through provider-owned `$lookup` and junction pipelines.
- MongoDB many-to-many mutation remains unsupported until the provider writes
  junction documents during create/update/delete.
- SQL providers and Snowflake support many-to-many projection, but generated
  GraphQL many-to-many mutation inputs still need typed target-key parity.
- SQL provider many-to-many filtering through junction traversal is not
  certified yet.
- Error normalization is live-certified by duplicate-key, foreign-key, required
  field, stale-version, access-denied, and invalid-filter contracts that reject
  provider-native error leakage.
- Access filters are live-certified by a provider-neutral policy-widening
  contract that uses local-dev non-admin auth and proves policy filters compose
  with, rather than get widened by, user filters.
- Audit is live-certified by an append/redaction/chain contract that creates and
  updates an audited CRM record, queries the generated audit timeline, and
  verifies record scope, tenant scope, redacted fields, and hash continuity.
- Snowflake aggregation is supported by the hosted API path; LocalStack
  Snowflake remains public-preview parity coverage.
- FabricSqlAnalytics reuses the MS SQL-compatible T-SQL compiler for filters,
  sorting, projection, pagination, and aggregates, but executes through
  `odbc-api` plus Microsoft ODBC Driver 18 rather than a native TDS client.
  It is not live certified against Microsoft Fabric yet, and generated
  mutations, migrations, seed writes, audit writes, and concurrency semantics
  are unsupported because Fabric SQL analytics endpoints are reporting/read-only
  surfaces.
- PostgreSQL exposes cached prepared query/execute helpers. MS SQL Server and
  Snowflake expose bound parameter execution but do not yet claim a reusable
  prepared-handle lifecycle in the current provider path. MongoDB uses native
  BSON commands and pipelines, so SQL prepared statements are not applicable.
- PostgreSQL stored routine invocation is live-certified by the CRM
  `provider_stored_routine_return_payload_contract`, which calls a
  row-returning stored procedure and verifies both the returned payload and the
  persisted side effect. MS SQL Server and Snowflake expose compiler-contracted
  stored routine invocation helpers until their live provider backends prove the
  same CRM routine contract. Routine identifiers are provider-validated and
  quoted; argument values are bound through existing provider parameter APIs.
  MongoDB stored routines are not part of the provider contract.

## Test Layout

Backend unit and compiler contracts:

```text
backend/src/data/clients/contract_tests.rs
appfw_runtime/src/provider_capabilities.rs
appfw_runtime/src/provider_certification.rs
appfw_provider_postgres/src/routine.rs
appfw_provider_mssql/src/routine.rs
appfw_provider_snowflake/src/routine.rs
```

Live provider-neutral API contracts:

```text
api_tests/src/provider_semantic_contracts.rs
api_tests/src/provider_contracts.rs
api_tests/src/provider_schema_contracts.rs
```

Generated product API scenarios live under `api_tests/src/schemas` and are run
by `scripts/appfw api-test`. Provider certification contracts stay in
framework-owned provider modules and are run by `scripts/appfw provider-test`.

The live suite currently covers:

- CRUD, scalar filtering, sorting, valid pagination, and flat projection.
- Scalar filter edge cases for null equality/inequality, membership, numeric
  comparisons, boolean conjunctions, date period filters, and array contains /
  overlaps semantics.
- Invalid pagination edge cases.
- Stale update and stale delete optimistic concurrency.
- Invalid filter error shape.
- Aggregation grouping, metrics, having, sorting, and paging.
- Relationship projection for providers that declare support, including
  to-one, to-many, and many-to-many projection paths.
- Relationship filtering for to-one and to-many paths.
- MongoDB many-to-many filtering through junction-backed `$lookup`.
- Access-filter policy composition with a non-admin local-dev user, including a
  broad user filter, a conflicting user filter, and a denied mutation.
- Tenant isolation on tenant-scoped audit records, including positive
  same-tenant reads and negative cross-tenant reads for two admin tenants.
- Error normalization for duplicate key, foreign-key, missing required value,
  stale version, denied access, and invalid filter responses.
- Audit append, redaction, record/tenant scoping, hash-chain continuity, and
  denied mutation attempts.

## Release Rule

Before release, every contract area must be either:

- `LiveCertified` and passing in the shared live contract suite for every
  provider that declares it, or
- `CompilerContracted`, `Implemented`, `Partial`, `Unsupported`, or
  `EmulatorLimited` with an actionable reason and a roadmap item.

No provider-specific behavior should be introduced without updating the matrix,
the shared contract suite, and this document.

For adding a fifth provider, start with the Provider SDK rules:

```bash
scripts/appfw explain provider-sdk --json
```

See [Provider SDK Rules](provider-sdk.md) for the required integration
surfaces and certification checklist.

Framework releases should run `scripts/appfw release-check --json`; a passing
local `scripts/appfw test` is necessary but not sufficient for provider
certification.
For local branch preflight, run
`scripts/appfw framework local-live-preflight --json` and retain
`target/appfw/local-live-release-preflight.json`, `provider-parity.json`, and
`security-certification.json` with the branch evidence. That local evidence
keeps `release_ready:false` by design because production release authority
still requires the remote release gate and live/governance artifacts.

Bitbucket Pipelines runs this gate with PostgreSQL, MongoDB, MS SQL Server, and
LocalStack Snowflake services available. ArgoCD should promote only immutable
images or manifest revisions that already passed this gate. See
`docs/release/release-gate-ci-cd.md` for the CI/CD contract.
