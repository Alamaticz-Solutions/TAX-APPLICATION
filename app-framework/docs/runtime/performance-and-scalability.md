# Performance And Scalability

App Framework treats scalability controls as part of the generated backend
contract. Generated APIs should be safe by default, measurable in production,
and easy for agents to verify during handoff.

## Runtime Query Guardrails

GraphQL depth and complexity protect the outer API shape. QueryIR cost budgets
protect the provider-neutral query plan after selections, filters, access
filters, sorting, pagination, relationships, and aggregates have been parsed.

```text
APP_QUERY_COST_MAX=1000
APP_AGGREGATE_QUERY_COST_MAX=1500
APP_QUERY_RELATIONSHIP_DEPTH_MAX=4
APP_QUERY_MANY_TO_MANY_EXPANSION_MAX=4
APP_QUERY_FILTER_PREDICATES_MAX=25
APP_QUERY_SORT_SPECS_MAX=3
APP_AGGREGATE_OUTPUTS_MAX=12
APP_QUERY_SELECTED_FIELDS_MAX=80
APP_QUERY_OFFSET_ROWS_MAX=5000
```

The budget is enforced before provider compilation. A provider should never
receive a query plan that already violates the framework cost policy. The
diagnostic budget is intentionally explicit: release reviewers can see whether
a query exceeded total cost, relationship fanout, filter shape, sort shape,
aggregate output shape, selected-field breadth, or offset depth.

## Pagination Policy

Generated list and aggregate methods accept optional `skip` and `limit`
arguments. If a caller omits them, the backend applies the runtime pagination
policy:

```text
APP_QUERY_DEFAULT_PAGE_SIZE=50
APP_QUERY_MAX_PAGE_SIZE=250
```

`skip` must be greater than or equal to `0`. `limit` must be greater than `0`
and less than or equal to `APP_QUERY_MAX_PAGE_SIZE`. The same policy applies to
offset and keyset pagination.

Keyset queries use the existing opaque `after` cursor. Callers should treat
cursor contents as private framework data.

## Generated Performance Recommendations

Generation emits review artifacts from the application config:

```text
.appfw/target/appfw/performance_recommendations.json
.appfw/target/appfw/performance_recommendations.md
```

The report includes:

- Declared entity indexes.
- Foreign-key indexes inferred from relationships.
- Many-to-many junction table access paths.
- Audit timeline, actor, chain, and event-hash access paths.
- Provider-specific review notes for MongoDB and Snowflake where generated SQL
  indexes do not apply directly.

Use this report during migration planning, release review, and agent handoff.
For generated relational DDL, keep `scripts/appfw migrate drift --json` green.
For provider-specific tuning, scaffold a migration:

```bash
scripts/appfw migrate new --schema crm --phase expand --name add-performance-indexes
```

## Load Testing Generated APIs

Run a lightweight local load test against a running backend:

```bash
ENV_NAME=local API_PORT=8080 scripts/appfw serve
scripts/appfw load-test --json
scripts/appfw load-test-suite --json
```

Defaults:

```text
APPFW_LOAD_TEST_URL=http://127.0.0.1:8080/crm
APPFW_LOAD_TEST_REQUESTS=100
APPFW_LOAD_TEST_CONCURRENCY=8
APPFW_LOAD_TEST_MAX_ERROR_RATE=0
```

The default `load-test` request targets the sample CRM `queryAccounts` API.
Downstream apps can provide their own JSON GraphQL request body:

```bash
scripts/appfw load-test \
  --url http://127.0.0.1:8080/customer \
  --requests 500 \
  --concurrency 16 \
  --body-file ./load-test-query.json \
  --json
```

For token-backed environments, pass a token through:

```bash
APPFW_LOAD_TEST_TOKEN="$JWT" scripts/appfw load-test --json
```

The JSON summary includes success/failure counts and min/average/p95/max
latency in milliseconds. It also records the scenario, error rate, active
thresholds, and threshold violations. Use thresholds for release lanes:

```bash
scripts/appfw load-test \
  --json \
  --scenario crm-accounts-grid \
  --max-p95-ms 300 \
  --max-max-ms 1000 \
  --max-error-rate 0
```

The single-scenario JSON artifact is retained at `target/appfw/load-test.json`
when invoked through `scripts/appfw load-test --json`, or under
`APPFW_RELEASE_ARTIFACT_DIR` when that directory is set.

For release candidates, prefer the suite:

```bash
scripts/appfw load-test-suite \
  --json \
  --requests 250 \
  --concurrency 12 \
  --max-p95-ms 300 \
  --max-max-ms 1000 \
  --max-error-rate 0
```

The default CRM suite covers dashboard summary, accounts grid, server-side
search, account form projection, lookup selector, and account/activity
relationship scenarios. It retains per-scenario artifacts under
`target/appfw/load-tests/` and writes the suite summary to
`target/appfw/load-test.json` for release-evidence compatibility.

Provider-specific performance evidence is produced separately:

```bash
scripts/appfw provider-performance --json --all
APPFW_PROVIDER_PERF_REQUIRE_LIVE=true scripts/appfw provider-performance --json --all
```

The artifact is retained at `target/appfw/provider-performance.json`. It
validates generated performance recommendations, QueryIR budget-cap contracts,
and retained provider parity areas for filters/sorts, pagination, native
projection, relationship projection, many-to-many fanout, aggregate/count paths,
and access filters. Use `APPFW_PROVIDER_PERF_REQUIRE_LIVE=true` in release
lanes so these provider-specific checks require live provider parity evidence.

Production release lanes can make performance evidence blocking:

```text
APPFW_RELEASE_REQUIRE_PERFORMANCE_EVIDENCE=true
```

When enabled, `scripts/appfw release-check --json` runs the load-test suite and
provider-performance certification, records `load-test.json`,
`load-tests/*.json`, and `provider-performance.json`, and fails if configured
thresholds or provider-specific performance certification fail. Keep this
disabled for local release checks unless a backend is already running and
provider parity evidence has been produced.

## Safe Provider Diagnostics

Admin troubleshooting exposes a safe query diagnostic endpoint when enabled:

```text
POST /admin/troubleshooting/query/diagnose
```

Required:

```text
APP_ADMIN_TROUBLESHOOTING_ENABLED=true
```

Example body:

```json
{
  "schema_name": "crm",
  "type_name": "Account",
  "filter": {
    "billing_state": {
      "_eq": "CA"
    }
  },
  "sort": {
    "name": "asc"
  },
  "limit": 25
}
```

The response includes request context, provider identity, normalized
pagination, QueryIR cost, active budget, and provider diagnostic status.
Providers return `unsupported` unless they implement a safe redacted EXPLAIN
hook. Provider implementations must not expose credentials, raw connection
strings, tenant-sensitive values, or unredacted bind parameters.

## Handoff Checklist

For scalability-sensitive changes:

```bash
scripts/appfw validate --json
scripts/appfw generate
scripts/appfw generate --check --json
scripts/appfw test
```

When a backend is running, add:

```bash
scripts/appfw load-test --json
scripts/appfw api-test
```
