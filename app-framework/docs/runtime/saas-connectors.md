# SaaS Connectors (Vendor SaaS APIs)

> **Status: active architecture / implementation plan.** This page is the
> central route map for external SaaS systems of record. It defines the three
> supported data paths, the kappa/CDC posture, and the evidence gates that keep
> live reads and governed writes honest. Read it with
> [SaaS Connector Certification](saas-certification.md),
> [SaaS API Intelligence Snapshot](saas-api-intel-2026-06-26.md),
> [SaaS Authenticated Export Evidence](saas-authenticated-export-evidence.md),
> [Provider SDK](provider-sdk.md), [Graph Read Providers](graph-read-providers.md),
> [AI Chat Search](ai-chat-search.md), and
> [Provider Certification](provider-certification.md).

## 1. Purpose And Current Posture

Use the SaaS platform as the system of record. Use App Framework as the governed
chassis. Product code should not hand-write Salesforce, Workday, ServiceNow,
iCIMS, Anaplan, Oracle Financials, or other vendor HTTP/SOAP calls. Products
consume framework-owned provider surfaces, schema projections, sync descriptors,
generated handlers, policy, and evidence artifacts.

Adding or promoting a SaaS connector is a **framework change, not an application
customization**. The product owns which schema projections, named operations,
policies, handlers, and UI it uses. The framework owns provider identity,
transport, auth, pagination, rate limits, redaction, certification, generated
sync infrastructure, and release posture.

Current implementation ground truth:

- `FrameworkProvider` includes an external API family for ServiceNow, Workday,
  iCIMS, Salesforce, Anaplan, and Oracle Financials.
- External API providers are excluded from CRUD semantic parity.
- `SaasReadArea` capability reporting exists.
- `appfw_saas_core` owns shared request, OAuth/token, pagination/continuation,
  async task/read-request page, chunk-download transfer metadata/caps,
  rate-limit, retry, and redaction primitives.
- Salesforce, Workday, Anaplan, and Oracle Financials have network-free
  request-planning skeletons.
- ServiceNow and iCIMS have metadata-only, evidence-gated skeletons that record
  planned operation catalogs and tenant/auth metadata but reject execution.
- AI search is not part of the external API SaaS family. `appfw_provider_ai_search`
  is a separate plan-only provider family with named search/embed operations,
  gateway/auth posture, and fail-closed enterprise contract evidence gates.
- `.appfw/model/sync/*.yaml` descriptors validate and generate fail-closed sync
  worker plans.
- `appfw_runtime` has the first production SaaS HTTP transport substrate
  (`RuntimeHttpSaasRequestExecutor` / `ReqwestRuntimeSaasRequestExecutor`) and
  a provider-neutral client-credentials token endpoint executor
  (`RuntimeHttpOAuthClientCredentialsTokenExecutor` /
  `ReqwestRuntimeOAuthClientCredentialsTokenExecutor`) for local-proven OAuth
  form exchange wiring. It also has a provider-neutral client-credentials SaaS
  request executor (`RuntimeClientCredentialsSaasRequestExecutor`) that binds
  the token executor/cache to the SaaS HTTP executor, injects runtime-owned
  bearer tokens, reuses fresh tokens, refreshes expiring tokens within skew, and
  partitions cached tokens by runtime tenant while rejecting caller-supplied
  `Authorization` headers before token acquisition. This is local auth-binding
  proof, not live provider certification.
- `appfw_runtime` also has provider-neutral
  delegated-auth/idempotency/audit primitives. `appfw_runtime::sync_worker`
  also has a local fixture runner that produces checkpoint, provenance,
  echo-loop, dead-letter, and idempotent-upsert evidence plus provider-neutral
  checkpoint/projection-store traits with in-memory implementations for local
  resume/replay proof. These are not provider live-read certification, durable
  worker activation, or governed-write graduation. Database-backed checkpoint
  persistence, real projection-store adapters, provider-specific live bindings,
  encrypted token custody/rotation evidence, and executable governed-write
  paths remain gated.

No SaaS provider should be reported as `LiveCertified` from the current
network-free corpus alone. A listed live test, docs snapshot, or skeleton crate
is guardrail evidence, not live certification.

## 2. Three Supported Paths

Every SaaS integration must choose one of three paths. Mixing paths is allowed
only through the governed provider boundary.

| Path | Name | Primary Store | Runtime Hot Path | Use When |
| --- | --- | --- | --- | --- |
| A | **Live external API read** | SaaS platform | `RuntimeExternalApiProvider` named query | The app needs current vendor state and can accept vendor latency/rate limits. |
| B | **Materialized projection** | Product-owned projection store | Normal generated data provider reads | The app needs dashboards, joins, search, outage tolerance, or conversational reads over SaaS-derived data. |
| C | **Governed write-back** | SaaS platform | Named audited mutation | The app must change the SaaS record of truth and delegated/on-behalf-of evidence exists. |

The default for downstream products is **Path B, materialized projection**. Path
A is narrower and read-only until a provider has live read evidence. Path C is
closed by default and opens only per named mutation after G1 governed-write
evidence exists.

### Governed Write And Delegated Auth Substrate

The runtime delegated-auth substrate is provider-neutral. It models the pieces
needed for Archetype-2 and governed write-back without constructing any
vendor-specific payloads:

- per-user delegated token keys partitioned by provider, data source, tenant,
  user, and scopes;
- runtime principal envelopes (`principal_type`, `ingress`, and optional
  `on_behalf_of`) that flow into Rego policy input for HTTP, MCP, and Kafka
  ingress;
- authorization-code/PKCE request shape plus refresh-token storage records;
- idempotency/replay guards for named SaaS mutations;
- SaaS write audit records with secret redaction;
- a governed-write certification runner skeleton.

Mock and plan certification runs are local contract checks only. They write to
`governed-write-mock-fixture.json` semantics and are never accepted as release
evidence. The live release artifact remains `governed-write-evidence.json`, and
the runtime refuses to produce that artifact without managed provider evidence.
The ServiceNow binding rule still applies: named-mutation registration may exist
before a live instance is available, but request and payload construction waits
for authenticated OpenAPI/dictionary export evidence.

## 3. Path A: Live External API Read

Path A reads the SaaS platform directly through a named-operation registry.
There is no arbitrary raw request surface from GraphQL, MCP, Kafka, frontend
code, or product handlers.

Required contract:

- `execute_named_query(op, bound_params) -> DTO rows` resolves `op` to a vetted
  registry entry. Caller input only fills bound slots.
- The registry owns object/table names, URL/method, field projection,
  pagination mode, result caps, timeout caps, and supported predicates.
- Providers bind values through structured request builders. They must never
  concatenate caller values into SOQL, `sysparm_query`, SOAP payloads, REST
  URLs, `q`, `finder`, `orderBy`, `expand`, arbitrary resource paths, or
  generic POST/PATCH bodies.
- Optional predicate binding is per operation, per provider, and proven by
  tests. Accepting framework `QueryIR` is not implied.

Path A is appropriate for always-fresh reads, low-volume operational views, and
provider-native objects whose semantics do not fit relational projection. It is
not the first answer for large dashboards, joins across systems, or AI chat over
wide operational history.

## 4. Path B: Materialized Projection

Path B materializes selected SaaS data into a product-owned projection store.
The product reads those projections through ordinary generated entities,
filters, RBAC, GraphQL, and UI components. The SaaS platform remains the record
of truth; the projection is an evidence-governed read model.

| Axis | What It Versions | Mechanism |
| --- | --- | --- |
| Framework schema projection | The fields, names, RBAC, DTOs, and generated API that App Framework exposes. | A flat schema namespace such as `salesforce_crm_v1`, with compatibility handled by the repo-wide SemVer rules in [Versioning And Compatibility](../release/versioning-and-compatibility.md). |
| Vendor API version pin | The external API contract, such as Salesforce `API 67.0` / doc `262.0` or Workday Human_Resources WWS `v46.1`. ServiceNow pins are target-instance exports on the Australia-docs evidence lane, and iCIMS remains unpinned until authenticated API docs exist. | A vendor API pin in schema metadata, consumed by the SaaS provider once the implementation adds that contract. |

SaaS schema packs use flat versioned names such as `salesforce_crm_v1`; do not
use nested `<schema>/v1` directories unless the loader is deliberately changed.
When a vendor ships a breaking object change, cut a new flat schema namespace,
run old and new projections side by side, and provide changelog/migration notes
like any other `.appfw/model` contract change.

The manifest stays thin and declares the external source plus the app-owned
projection store:

```yaml
# .appfw/manifest.yaml
topology:
  data_sources:
  - { name: salesforce_primary, provider: Salesforce, role: external-read }
  - { name: pg_primary,         provider: PostgreSQL, role: transactional }
  schemas:
  - { name: salesforce_crm_v1, data_source_name: salesforce_primary, role: external }
  - { name: pipeline,          data_source_name: pg_primary,         role: product }
```

Object maps, watermarks, retention, deletion policy, source-drift behavior, and
classification belong in model-owned sync descriptors, not in a large manifest
section:

```yaml
# .appfw/model/sync/salesforce_to_pipeline.yaml
name: salesforce_to_pipeline
source_schema: salesforce_crm_v1
target_schema: pipeline
mode: incremental_watermark
schedule: "*/5 * * * *"
freshness_slo: PT10M
objects:
- source: Opportunity
  target: Opportunity
  key: Id
  watermark: SystemModstamp
  provenance:
    projection_of: salesforce_crm_v1.Opportunity
    source_key_field: Id
    source_watermark_field: SystemModstamp
    source_origin_field: AppFrameworkOrigin__c
  echo_loop:
    policy: reject_self_origin
    local_origin: appfw:pipeline:salesforce_to_pipeline
    source_origin_field: AppFrameworkOrigin__c
- source: Account
  target: Account
  key: Id
  watermark: SystemModstamp
  provenance:
    projection_of: salesforce_crm_v1.Account
    source_key_field: Id
    source_watermark_field: SystemModstamp
    source_origin_field: AppFrameworkOrigin__c
  echo_loop:
    policy: reject_self_origin
    local_origin: appfw:pipeline:salesforce_to_pipeline
    source_origin_field: AppFrameworkOrigin__c
governance:
  classification: confidential
  retention: P2Y
  deletion: tombstone
  source_drift: fail_closed
```

Current validation accepts absent sync directories as a no-op and validates
present descriptors for file/name identity, uniqueness, incremental-watermark or
CDC object maps, target projection existence, target entity existence, and
governance metadata. Generation emits normalized
`.appfw/target/appfw/sync_descriptors.json`,
`.appfw/target/appfw/sync_worker_plan.json`, and fail-closed
`backend/config/generated/sync_workers.yaml`.

`provenance` is the per-record lineage contract. `projection_of` names the
source schema/object, `source_key_field` must match `key`, and
`source_watermark_field` must match `watermark`. `echo_loop` is the self-origin
guard: a generated worker must reject source records whose `source_origin_field`
matches the app's `local_origin` marker before writing them back into the
materialized projection.

If the app manifest later gains a small deployable binding for sync workers, it
should point at model-owned descriptors rather than becoming the source of truth
for object maps, watermarks, provenance, echo-loop policy, retention, deletion
policy, drift behavior, and classification.

DP1 adds a CDC planning contract alongside watermark polling:
`mode: cdc_event` descriptors validate as planned Kafka-backed vendor CDC feeds,
emit `checkpoint.watermark_mode: cdc_offset`, and require the explicit
`cdc_broker_client` activation gate. Runtime CDC helpers can decode the governed
CDC envelope (`event_id`, `tenant_id`, `source_system`, `source_schema`,
`source_object`, `source_key`, `operation`, `occurred_at`, and `data`) and derive
partition/offset checkpoint posture from a consumed message. This remains a
planning contract only: generated worker config stays `enabled: false`, and no
live broker client, checkpoint store, idempotent projection write loop, or
worker execution path is certified by DP1 alone.

The production deployment shape is a separate worker process, not background
polling inside the HTTP API backend. The first worker can reuse the generated
backend image with `APPFW_RUNTIME_MODE=sync`; the HTTP backend runs
`APPFW_RUNTIME_MODE=http`. Any later dedicated worker binary must preserve the
same process boundary, secrets posture, service account, readiness, checkpoint,
dead-letter, and projection-write ownership.

## 5. Path B Kappa/CDC Mode

The materialized path has two ingestion modes:

| Mode | Source | Framework Role | Current Status |
| --- | --- | --- | --- |
| Pull-fed sync | SaaS API | Framework sync worker calls named SaaS reads and upserts projections. | Planned execution substrate; descriptors and fail-closed worker plan exist. |
| Kappa/CDC | Enterprise stream/Mongo/Atlas/CDC/Kafka | Framework consumes governed CDC topics and/or Mongo projections as projection input. | Planned DP lanes; Kafka ingress shell exists, but CDC ingestion execution is not complete. |

The North Star direction is that much of the enterprise SaaS estate streams into
MongoDB, is moving to Atlas, and republishes change events through Mongo CDC
onto Kafka. Where that coverage exists, MongoDB is the **default Archetype-1
projection store** and Kafka CDC is the change-notification path. PostgreSQL
remains fully supported and preferred for join-heavy relational reads. The
projection store is a per-schema binding, not a platform bet.

Kappa ownership outside App Framework does not relax framework gates. Every
kappa-fed projection still requires:

- Classification and retention evidence.
- Tenant isolation and policy enforcement.
- Tombstone/delete handling.
- Freshness SLO and lag reporting.
- Source drift handling.
- Provenance markers tying projection rows to source vendor/object/version.
- Echo-loop prevention so framework writes or projection upserts do not
  re-enter as self-originated business events.
- Per-vendor stream coverage evidence from the data platform team.

The DP lanes in [Agent Handoff Backlog](../start/agent-handoff-backlog.md) map
this work: DP1 CDC ingestion, DP2 provenance/echo-loop contract, DP3 Atlas
connection, DP4 freshness/lineage report, DP5 vendor-doc structural parity, DP6
SaaS testkit, and DP7 principal envelope.

## 6. Path C: Governed Write-Back

Path C changes the SaaS system of record through named, audited mutations. It is
the G1 critical path and is intentionally unavailable by default.

Required contract:

- The mutation registry is empty until a named mutation is approved.
- Each write is a registered operation such as `servicenow.create_incident`;
  there is no generic `POST`, caller table name, raw body, or arbitrary request
  body path.
- Each write has idempotency/replay protection.
- Each write carries redacted audit metadata: actor, tenant, operation, source
  provider, object, result, and parameter hash/summary.
- Each write checks policy/scope and provider capability posture before
  execution.
- MCP, Kafka, and conversational surfaces remain disabled for the write until
  the write is certified for that ingress.

Native SaaS fidelity usually requires the SaaS platform's own ACLs to apply to
the acting user. That means delegated/on-behalf-of auth: OAuth auth-code or
vendor equivalent, token storage keyed by `(authenticated_user, tenant,
provider)`, refresh, revocation, and token-store isolation. The current runtime
has JWT auth state and environment secret providers, but not the per-user SaaS
token subsystem.

| Vendor | M2M Projection Auth | Delegated Write Auth |
| --- | --- | --- |
| Salesforce | JWT bearer or client credentials through an integration user | Per-user OAuth web-server flow so sharing rules apply |
| ServiceNow | OAuth client credentials through an integration user | OAuth auth-code/delegated flow so ServiceNow ACLs apply |
| Workday | ISU plus tenant-approved WWS auth | Usually ISU/security-group governed; per-user delegation is rarer and evidence-specific |

Write exposure is a release-authority decision after evidence exists. Local mock
or planning mode must never write `governed-write-evidence.json` in a form that
the release validators accept as live provider evidence.

## 7. Provider Family, Schemas, And Capabilities

SaaS APIs are read-first external systems of record with heterogeneous query
languages. They are a provider family alongside database providers and graph
read providers, not CRUD providers with a long unsupported list.

`appfw_saas_core` owns shared connector machinery: async HTTP client seam, M2M
token cache with pre-expiry refresh, rate-limit/backoff, URL cursor,
page/count, offset/limit, async task/read-request page continuations,
chunk-download transfer metadata/caps, retry/idempotency helpers, response
field redaction, metrics, and test utilities.

Vendor crates own vendor dialect: auth flavor, object/field mapping,
named-operation registry, request construction, response decoding, and live
smoke tests. Current posture:

| Provider | Current Posture |
| --- | --- |
| Salesforce | Network-free Account `getUpdated`/`getDeleted`, fixed SOQL/REST plans, API-67 metadata, describe-gated Health Cloud candidates, pagination, watermark, rate-limit signal, redaction, caps, mutation rejection, tenant binding. |
| Workday | Network-free Human_Resources WWS templates, selected `Get_*` operations, page/count pagination, `Get_Server_Timestamp`, response summaries, fault classification, mutation rejection, tenant-bound WWS path contracts. |
| Anaplan | Network-free Integration API v2 request planning, model/files reads, read-request and chunk-download templates, continuation metadata, paging summaries, mutation rejection, tenant binding. |
| Oracle Financials | Network-free Fusion Cloud Financials 26B REST request planning for accounting-period-status LOV list/get only; broad operations cataloged but unsupported; live auth and tenant financial scoping uncertified. |
| ServiceNow | Metadata-only evidence-gated skeleton; planned reads plus non-executable `servicenow.create_incident`; all areas unsupported until authenticated instance evidence and G1 write evidence exist. |
| iCIMS | Metadata-only evidence-gated skeleton; planned discovery/recruiting reads; all areas unsupported until authenticated docs or sandbox/tenant exports exist. |

Flat versioned schema packs use one schema directory per framework projection,
for example:

```text
app_gen/_config/schemas/
  servicenow_itsm_v1/
  workday_hcm_v1/
  icims_recruiting_v1/
  salesforce_crm_v1/
```

The framework schema projection version and the vendor API version pin are
separate. Products pin the framework projection by schema namespace. Vendor pins
belong in metadata consumed by provider validation and evidence checks.

`SaasReadArea` is the capability rubric, not database CRUD parity. It includes
connection auth, registry enforcement, request binding, pagination, rate limits,
incremental watermark, redaction, tenant scoping, schema version pinning, caps,
audit/metrics, freshness, and opt-in governed-write areas such as delegated
actor context, token-store isolation, mutation registry, idempotency, policy,
and write evidence.

## 8. Principal, Tenant, Policy, Audit, And Redaction

All paths run through the same governed operation model:

```text
GraphQL / MCP / Kafka / product service / sync worker
  -> auth, tenant, policy, audit context
  -> generated operation dispatcher or product service
  -> RuntimeExternalApiProvider or projection data provider
  -> DTO result or governed mutation outcome
```

Minimum invariants:

- **Tenant scoping** is server-bound from authenticated user, service principal,
  or configured integration context. Caller-supplied tenant values are rejected.
- **Principal envelope** distinguishes human HTTP users, MCP agents, Kafka
  service principals, sync workers, and optional on-behalf-of users.
- **Secrets stay out of config.** Topology and schema metadata may name
  providers and versions; credentials stay in runtime secret mechanisms.
- **RBAC remains deny-by-default** through generated Rego policy.
- **Audit is redacted** and records actor, tenant, ingress, operation,
  provider, source/projection freshness, duration, result count, and redacted
  parameter summary/hash.
- **Caps and cost** clamp result caps, timeouts, and rate-limit budgets to
  provider ceilings.
- **Observability** emits correlation IDs, timings, slow-call events, provider
  readiness, freshness, sync lag, CDC lag, and dead-letter counts where
  applicable.

Never log tokens, raw request bodies, raw vendor query text with caller values,
or PHI/PII payloads.

## 9. Certification And Evidence

Use the same certification ladder as [Provider SDK](provider-sdk.md):
`CompilerContracted` from offline tests and `LiveCertified` only after the
required live evidence actually runs and passes. A Fabric-style ignored live
smoke test is a useful credential-handling pattern, but it is not sufficient by
itself to certify SaaS semantics.

Evidence must prove the path being claimed:

| Claim | Required Evidence |
| --- | --- |
| Path A live read | Provider auth, registry enforcement, request binding, pagination, rate limit, tenant binding, redaction, caps, freshness/metrics, and live vendor run. |
| Path B pull-fed projection | Sync descriptor validation, worker execution loop, checkpoint persistence, idempotent upsert, dead-letter handling, projection governance, freshness, and provider live read evidence. |
| Path B kappa/CDC projection | CDC envelope decode, offset checkpointing, provenance markers, echo-loop prevention, source drift/tombstone handling, lag/freshness, tenant isolation, and vendor stream coverage evidence. |
| Path C governed write | Delegated/on-behalf-of auth, token-store isolation, named mutation registry, mutation request binding, idempotency/replay, policy/scope enforcement, write audit, redaction, and live write evidence. |

The detailed evidence checklist lives in
[SaaS Connector Certification](saas-certification.md). Authenticated vendor
export requests live in
[SaaS Authenticated Export Evidence](saas-authenticated-export-evidence.md).

## 10. Implementation Sequence And Verification

Current checkpoint:

- `FrameworkProvider` has an external API family for ServiceNow, Workday, iCIMS,
  Salesforce, Anaplan, and Oracle Financials, and those providers are excluded
  from CRUD semantic parity.
- `SaasReadArea` profiles and provider-certification output exist for external
  API providers.
- `appfw_saas_core` owns shared request, OAuth/token, pagination/continuation,
  async task/read-request page, chunk-download transfer metadata/caps,
  rate-limit, retry, and redaction primitives.
- `appfw_runtime` now has the first executable W3-A SaaS request substrate:
  `RuntimeHttpSaasRequestExecutor` plus `ReqwestSaasHttpTransport`. The executor
  builds requests only from validated `RuntimeSaasEndpoint` origins and relative
  `SaasRequestPlan` paths, injects runtime-owned bearer tokens, rejects
  plan-supplied `Authorization`, parses response metadata/body, and enforces
  response caps. This is still a read/request substrate; delegated auth,
  governed writes, live provider certification, and sync-worker activation
  remain gated separately.
- `appfw_runtime::sync_worker` now has a provider-neutral local fixture runner
  for Path B pull-fed projection mechanics. It validates a worker plan, suppresses
  self-origin records, records provenance, advances a local checkpoint watermark,
  counts duplicate stable source keys, emits projection-write intentions, and
  records poison rows as dead-letter evidence when the configured policy allows
  it. The same module now exposes checkpoint/projection-store traits plus
  in-memory implementations that prove checkpoint resume and stale replay
  suppression locally. This proves the local execution/storage contract only;
  database-backed checkpoint adapters, provider fetch loops, production
  projection-store adapters, scheduler readiness, and live certification remain
  separate gates.
- `appfw_provider_salesforce` has a network-free named-operation skeleton with
  fixed SOQL/REST request plans, Account `getUpdated` / `getDeleted` ID
  primitives, API-67 snapshot metadata, catalog-backed describe-gated Health
  Cloud projection candidates, pagination, watermark, rate-limit signal,
  response redaction, result-cap, mutation-rejection, and tenant-binding
  compiler contracts.
- `appfw_provider_workday` has a network-free Human_Resources WWS skeleton with
  typed SOAP request templates, selected `Get_*` operations, page/count
  pagination, `Get_Server_Timestamp` freshness metadata, response summaries,
  fault classification, mutation rejection, and tenant-bound WWS path compiler
  contracts.
- `appfw_provider_anaplan` has a network-free Integration API v2 skeleton with
  server-bound workspace/model request planning, model status and files-list
  reads, registry-gated view read-request and file chunk-download templates,
  offset/read-request/chunk continuation compiler contracts, paging summaries,
  mutation rejection, and tenant-binding compiler contracts.
- `appfw_provider_oracle_financials` has a network-free Oracle Fusion Cloud
  Financials REST API skeleton pinned to the 26B OpenAPI summary. It exposes
  fixed accounting-period-status LOV list/get request plans, cataloged but
  unsupported broad OpenAPI operation names, offset/limit pagination summaries,
  tenant host/REST-framework/scope binding, result caps, mutation rejection, and
  source metadata compiler contracts.
- `appfw_provider_servicenow` and `appfw_provider_icims` are metadata-only
  evidence-gated skeletons. They pin source evidence, validate server-owned
  tenant metadata, catalog planned operation names, and reject all named reads
  and mutations until authenticated exports/docs exist.
- ServiceNow and iCIMS remain `Unsupported` across SaaS areas with
  provider-specific evidence blockers; the crates do not promote
  `CompilerContracted` request-binding, pagination, auth, or watermark claims.
- `appfw_provider_ai_search` is a separate AI-search family, not CRUD parity
  and not the external-API SaaS family. It pins the two planned named queries,
  records ApiKey/BearerToken/ClientCredentials auth metadata, validates
  endpoint origin shape, records in-perimeter gateway/secret-custody posture,
  and rejects all execution until enterprise contract evidence, gateway/auth
  evidence, and the shared SaaS HTTP executor are present.
- `.appfw/model/sync/*.yaml` descriptors now have a validation and worker-plan
  contract. The validator accepts absent sync directories as a no-op, rejects
  malformed or incomplete descriptor YAML, requires a stable descriptor name
  matching the file stem, rejects duplicate descriptor names, requires
  incremental-watermark object mappings plus governance metadata, requires
  `target_schema` to be an existing app-owned projection schema, requires each
  object target to be an entity in that projection, and emits normalized
  `sync_descriptors.json` report data after validation succeeds. Generation
  emits `sync_worker_plan.json` plus `backend/config/generated/sync_workers.yaml`
  as a fail-closed plan for future worker templates. The checked-in YAML remains
  disabled and records activation gates until a later certified runtime slice
  deliberately enables execution; it does not execute SaaS reads.
- `appfw_runtime::sync_worker` owns the typed runtime loader and disabled worker
  shell report for `backend/config/generated/sync_workers.yaml`. It rejects
  unknown YAML fields, rejects `enabled: true`, validates worker counts and
  policy shape, and keeps provider calls, checkpoint storage, and projection
  writes behind later certification gates.
- DP4 has a first executable report slice:
  `scripts/appfw framework saas-lineage --json`, retaining
  `target/appfw/saas-freshness-lineage.json`. Enforced mode fails closed until
  runtime evidence with schema `appfw.saas.freshness-lineage.v1` proves
  freshness, lineage, provenance, echo-loop suppression, and redaction.

Lane-sized work should follow this sequence:

1. **Contract first**: provider identity, capability area, config/data-source
   enum, schema metadata, named operation registry, and evidence artifact
   contract.
2. **Offline planning**: request builders, response summaries, redaction,
   pagination, caps, and mutation rejection with no network dependency.
3. **Executor substrate**: production `RuntimeSaasRequestExecutor` behind
   origin pins, caps, redaction, and fail-closed activation.
4. **Path B pull execution**: sync-worker loop, checkpoint store, idempotent
   upsert, dead-letter, readiness, and freshness metadata.
5. **Path B kappa/CDC**: broker client, CDC envelope, offset checkpoint,
   provenance/echo-loop, Atlas/Mongo projection binding, and freshness/lineage
   report.
6. **Path C delegated auth/write**: token store, auth-code/refresh, idempotency,
   audit, live-cert runner, and provider-specific named mutation conversion.
7. **Live certification**: only after non-production vendor credentials,
   redacted samples, tenant/security scope, and release-authority evidence
   exist.

Implementation threads should keep shared-file ownership serialized. Any worker
touching `FrameworkProvider`, data-source enum/config contracts, generated
bootstrap types, `SaasReadArea`, or global CLI evidence gates coordinates
through a contract slice before fan-out.

SaaS connector implementation is a framework change. Use the framework safe
loop when this document or adjacent contracts change:

```bash
scripts/appfw framework validate --json
scripts/appfw framework docs-check --subcheck saas-vendor-doc-parity --json
scripts/appfw framework docs-check --json
scripts/appfw framework generate --check --json
scripts/appfw framework test --fast --json
scripts/appfw framework handoff --json
```

Provider implementation threads should also run focused crate tests and, when
credentials are intentionally supplied, the relevant ignored live smoke test. Do
not report a provider as `LiveCertified` unless live evidence actually ran and
passed.
