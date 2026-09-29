# Graph Read Providers

Graph read providers support governed relationship intelligence and traversal
queries without becoming primary entity persistence providers.

Neo4j is the first graph read provider shape. It is intentionally separate from
the PostgreSQL, MongoDB, MS SQL Server, and Snowflake CRUD provider parity
contract.

## Runtime Shape

Graph read access should flow through the same governed operation boundary as
HTTP, MCP, and Kafka:

```text
GraphQL / MCP / Kafka / product service
  -> auth, tenant, policy, audit context
  -> generated operation dispatcher or product service
  -> RuntimeGraphProvider
  -> named, parameterized graph query
  -> DTO result
```

Do not expose raw Cypher from GraphQL, MCP, Kafka, or frontend code.

## Product Topology

Products opt in by adding a Neo4j data source and topology entry:

```yaml
topology:
  data_sources:
  - name: pg_primary
    provider: PostgreSQL
    role: transactional
  - name: neo4j_graph
    provider: Neo4j
    role: graph-read
```

Entity schemas must stay on CRUD-capable data sources. Validation rejects
schemas that use `data_source_type: Neo4j`.

The generated local compose file renders Neo4j under the `graph` profile when
the topology includes a Neo4j data source:

```bash
podman compose --profile graph up
```

## Query Contract

Graph operations should be named product operations such as:

- `accountRelationshipGraph(accountId, depth)`
- `accountInfluenceNetwork(accountId)`
- `findShortestAccountPath(sourceAccountId, targetAccountId)`
- `riskPropagation(startingEntityId, depth)`

Each operation must define:

- stable operation name;
- parameterized Cypher query or provider-owned query builder;
- maximum traversal depth;
- maximum returned nodes/relationships;
- query timeout;
- tenant and access-filter strategy;
- DTO result shape.

## Security Rules

Neo4j credentials should be read-only for product request paths.

Required guardrails:

- parameterized Cypher only;
- no arbitrary query execution from GraphQL, MCP, Kafka, or browser clients;
- no graph writes in request-path read operations;
- tenant scoping on the start node and returned result set;
- record locators or public identifiers at ingress boundaries;
- traversal depth, result count, and timeout caps;
- audit fields for operation name, actor, tenant, start node, depth, result
  count, and graph freshness;
- redaction of raw parameters and classified graph payloads in logs.

## v1 Provider

The v1 graph provider is implemented in `appfw_provider_neo4j` and implements the
runtime `RuntimeGraphProvider` contract — a single trait carrying read
(`execute_named_query`) and opt-in governed write (`execute_named_mutation`)
methods, mirroring how the relational CRUD providers expose one
`RuntimeProviderDataClient`. Graph access is read-first; writes are gated at the
operation/exposure layer (which operations a product registers and exposes),
not by a separate trait. It is built around a **named-operation registry**: the
only graph access surface is a vetted, registered `Neo4jGraphQuery` /
`Neo4jGraphMutation`, resolved by operation name, with bound parameters. There is
no arbitrary-query path.

Every execution is guarded end to end:

- **Named operations only** — an unknown operation, or a caller-supplied Cypher
  that does not match the registered definition, is rejected.
- **Read-only reads** — a read operation's registered Cypher is re-validated as a
  single read-only statement (write clauses are rejected before execution).
- **Tenant scoping** — a tenant-scoped operation must reference its tenant
  parameter; the provider binds that parameter from the authenticated user and
  rejects any caller-supplied tenant value.
- **Traversal / result / timeout caps** — per-operation limits are clamped down
  to provider ceilings, traversal depth is enforced against the effective cap
  (unbounded traversals are always rejected), results are truncated to the cap,
  and execution runs under a hard timeout.
- **Audit + metrics** — every attempt emits a redacted audit record (operation,
  actor, tenant, start node, depth, result count, freshness, duration, outcome)
  through a `Neo4jGraphAuditSink`; raw parameter values are never logged.
- **Freshness** — the executor surfaces projection freshness, exposed in result
  metadata and the audit record.

The Bolt driver is injected through the single `Neo4jExecutor` seam, so the full
guardrail path is exercised by the crate's unit and provider tests without a
live database.

## Governed Writes (opt-in)

Graph writes are an explicitly opt-in surface on the same `RuntimeGraphProvider`.
A product chooses whether to register any write mutations and whether to expose
them (e.g. a write custom method keeps `mcp_enabled: false` until a live
write-safety lane exists), so a read-only deployment simply registers no write
operations. Write-safety is enforced at the operation/exposure layer plus the
governed-write Cypher validation below — the same approach the relational
providers use for their disabled standard mutations.

Governed mutations are vetted, named, relationship-scoped, tenant-bound
operations:

- only relationship-scoped `MERGE` / `CREATE` / `DELETE` are allowed (property
  `SET` is deferred);
- `MATCH`-anchored: `MERGE`/`CREATE` may only connect nodes already bound by a
  `MATCH`, so a relationship write cannot implicitly create a node;
- `DETACH DELETE`, `DROP`, `REMOVE`, and `LOAD CSV` are rejected;
- the tenant parameter is server-bound and the cypher must reference it;
- a redacted `Neo4jGraphMutationAudit` is emitted per attempt, carrying mutation
  stats, result count, and a `truncated` flag.

These guardrails are defense-in-depth over **product-authored** named mutations
(callers cannot supply cypher); they are not a full Cypher parser. Until a live
write-safety test lane exists, product mutations that map to graph writes should
keep `mcp_enabled: false` (no remote MCP tool exposure); they remain available
as GraphQL mutation fields.

## Bolt Driver Feature

The live `neo4rs`-backed executor (`Neo4jBoltExecutor`) is behind the optional
`bolt` cargo feature, so default builds and tests stay driver-free and offline.
Two `neo4rs` 0.8 limitations are documented and carried as known gaps:

- **Write stats** are reported as the affected RETURN-row count, not precise
  node/relationship counters (Bolt result-summary counters are not exposed by
  this driver version); `mutation_stats` may therefore exceed `result_count`
  when results are capped, which is surfaced via the audit/metadata `truncated`
  flag.
- **Access mode** routing is not applied at the driver level; the read path's
  read-only guarantee comes from read-only cypher validation, not replica
  routing.

## Certification Posture

Graph read providers are not part of database semantic parity. Neo4j
certification is a separate graph-read evidence path:

- connection/auth works;
- read-only credentials are enforced;
- named queries compile;
- unsafe Cypher clauses are rejected;
- tenant isolation is proven;
- traversal limits are enforced;
- result caps and timeout behavior are proven;
- query metrics and audit fields are emitted;
- projection freshness is observable.

Each area above is **compiler-contracted** by the `appfw_provider_neo4j` tests
and is reported per area in the framework provider matrix
(`provider_matrix_json(Some(FrameworkProvider::Neo4j))` in
`appfw_runtime/src/provider_certification.rs`), with `live_certification`
marked `pending`.

Until live evidence exists against a real Neo4j instance, Neo4j support is a
guardrail-complete framework foundation for opt-in graph read-model work, not a
release-certified primary provider.
