# Workday HCM → MongoDB Atlas → Kafka → Snowflake Sync Research

> **Status: current accepted research guidance, verified 2026-07-22.** This
> assessment records a deep-research run (5 search angles, 22 sources fetched,
> 84 claims extracted, 25 claims adversarially verified: 25 confirmed, 0
> refuted) on the best options for syncing Workday HCM data into the MongoDB
> Atlas people-systems domain database, publishing enterprise change events,
> and feeding Snowflake. It answers the four open decisions for the Workday
> materialization leg and feeds `appfw_provider_workday/docs/vendor-contract.md`
> section 4a/4b (kappa coverage map, previously "UNKNOWN — to be filled by data
> platform team"). Research area 6 (ISU/ISSG least-privilege, redaction,
> HR-PII classification) produced no verified claims and remains an explicit
> coverage gap — see "Not verified" below. Strategy, backlog, and release
> authority remain with their named owners.

## Research Verdict

Keep the settled kappa/CDC Mongo-hub architecture — Workday → MongoDB
(materialized projection) → CDC → Kafka → Snowflake — and keep building the
custom Rust connector on WWS SOAP. Every verified commercial alternative is
weaker than a correctly built `Get_Workers` sync worker, and the verified
vendor-connector internals (Fivetran, Microsoft Entra) function as free design
validation for it. Two design corrections and one product choice fall out of
the evidence:

1. **Dual watermark, two separate queries.** A single
   `Updated_From`/`Updated_Through` window is insufficient, and combining the
   entered-date and effective-date ranges in one call is a logical AND that
   *narrows* results. Issue two windowed `Get_Workers` queries per cycle —
   one on `Effective_From`/`Effective_Through`, one on
   `Updated_From`/`Updated_Through` — plus as-of lookups for future-dated
   hires.
2. **Snapshot reconciliation is mandatory, not optional.** Workday's
   transaction log structurally misses child-object-only changes and
   passage-of-time state changes, and parent-record deletes are only
   detectable by full-set comparison. Periodic full-snapshot/full-ID-set
   reconciliation against the Mongo projection is the only complete tombstone
   and completeness mechanism.
3. **Mongo→Kafka via Atlas Stream Processing by default.** MongoDB positions
   the self-hosted Kafka Source Connector and fully managed Atlas Stream
   Processing as alternatives for the same job; with no existing Kafka Connect
   estate, ASP is the lower-operations default. Snowflake is fed from the hub
   through Snowflake Kafka Connector v4 (Snowpipe Streaming).

## Decision (a): Extraction surface — WWS SOAP `Get_Workers`

- MuleSoft (July 2025) recommends WWS SOAP as the primary surface for
  enterprise-scale bulk sync — including replicating full worker datasets to
  downstream systems like Snowflake — positioning WQL as analytics/ad-hoc,
  RaaS as row-limited reports, and REST as small transactional operations.
- The two most credible production implementations both drive `Get_Workers`
  with `Transaction_Log_Criteria`: Fivetran's Workday HCM connector and
  Microsoft Entra's provisioning connector.
- WQL (secondary surface at most): OAuth 2.0 only; served from tenant host or
  the WCP API gateway; query text hard-capped at 16,000 characters (GET under
  2,048, POST 2,048–16,000); results cached 30 minutes (bust with `offset=0`);
  effective/entry-date support is **per data source** (check
  `supportsEffectiveDate`/`supportsEntryDate` via `GET /dataSources/{ID}`);
  responses are security-trimmed to the calling principal.
- Workday REST mandates OAuth 2.0 (vs ISU WS-Security for WWS) — standardizing
  on REST/WQL would force an OAuth client setup alongside or instead of the
  planned ISU wiring.
- Workday exposes no true CDC or event stream for this purpose. Workday's own
  strategic direction (Workday Data Cloud / Data Connect) frames report/API
  extraction and materialized copies as the problem its zero-copy Iceberg
  sharing replaces — but that surface is early-adopter H1 2026, GA "later in
  2026."

## Incremental-sync correctness findings (the load-bearing section)

- `Get_Workers` `Transaction_Log_Criteria` exposes two independent date-range
  sets — Updated (entered) From/Through and Effective From/Through. Used
  independently: entered-window returns changes entered in the window
  regardless of effective date; effective-window returns changes that became
  effective regardless of entry date. Supplied together they combine as a
  **logical AND** (narrower, not wider).
- Fivetran issues **two windowed queries per sync** (one per criteria set) —
  confirming dual watermarks are required for correctness.
- Microsoft Entra's connector issues **three query types per cycle**:
  entered-window deltas, effective-window deltas (incl. terminations), and
  per-WID lookups with `As_Of_Effective_Date` for future-dated hires.
  Future-dated hires carry `Active="0"` until the hire date, so correct
  materialization requires as-of queries at the effective hire date.
- Structural blind spots (documented by vendors as unavoidable):
  - Child-object-only changes are silently missed when the parent record is
    untouched; Workday offers no alternative incremental filter. Fivetran's
    mitigation is periodic full re-syncs.
  - State changes driven purely by the passage of time (an effective date
    arriving) generate **no transaction** — a confirmed production failure
    mode (pre-hire accounts never auto-enabled); Microsoft documents this as
    by-design and universal.
  - Parent-record soft deletes are only detected during full refresh —
    reliable tombstones require a periodic full-ID-set diff against the
    projection.
  - Transaction-type filtering is lossy: >1,000 type IDs (2–1 verification
    vote), one business process can emit multiple types ("Change Job" emits
    both "Change Job" and "Promotion Inbound"), and rescinds/corrections are
    only detected if their types are explicitly listed. **Do not filter by
    transaction type.**
- `Get_Workers` returns current state only — no prior values (unlike Core
  Connector: Worker). Before/after diffs must be computed downstream against
  the Mongo projection.
- Throughput: pagination is sequential by default (~2–5 s/page; 5–10 s under
  load) but parallelizable — read `Total_Pages` from the first response, then
  fan pages out to concurrent workers (MuleSoft pattern; 2–1 vote). MuleSoft
  best-practice page size is 200–500 records (below the WWS max). Objects
  lacking time-range criteria hit a practical ~1M-record full-refresh ceiling
  (Fivetran warns errors above that require Workday Support).
- An over-permissioned ISU degrades API performance (Workday runs more
  security checks) — least-privilege is a performance practice as well as a
  security one.

## Decision (b): Build vs buy — keep the custom Rust connector

- **Fivetran** (most mature ELT option): incremental sync for only seven
  object families (CANDIDATE, JOB_PROFILE, JOB_REQUISITION, PAYROLL, POSITION,
  WORKER, WORKER_EVENT + children); everything else is a daily full re-import.
  Warehouse-targeted; cannot feed Mongo.
- **Snowflake Openflow Connector for Workday**: RaaS-only (NiFi-based), full
  truncate-and-load with no incremental capability and no delete semantics,
  works only with advanced custom reports in JSON, no schema discovery, still
  a Preview feature as of 2026-07-22.
- **Airbyte, Matillion, Hevo, Workato, Boomi, SnapLogic**: no claims survived
  verification this run — unassessed, not rejected.

Buying reproduces roughly what the compiler-contracted connector already plans
for the worker family while giving up the compiler contract, PII gating, and
Mongo-hub fit.

## Decision (c): Feed Snowflake through the Mongo/Kafka hub

- Snowflake Kafka Connector v4 (GA since 2026-04, built on Snowpipe
  Streaming): exactly-once, ordered delivery; 5–10 s end-to-end latency; rated
  up to 10 GB/s; PIPE objects with server-side schema validation, error tables
  for replay, in-flight COPY transformations; flat per-GB throughput pricing.
- Constraints: **key-pair auth only** (OAuth explicitly unsupported in v4);
  **append-only** with respect to topic state — deletes/tombstones must be
  explicit events in the stream, and an outage longer than Kafka topic
  retention creates a permanent gap.
- The parallel direct Workday→Snowflake path today is Openflow's full-refresh
  RaaS load — strictly worse than the hub. Re-evaluation trigger: Workday
  Data Connect zero-copy Iceberg sharing to Snowflake/Databricks/Salesforce
  Data Cloud at GA (later 2026) as a possible *parallel analytics feed*, with
  the Mongo hub staying authoritative for operational reads and enterprise
  events.

## Decision (d): Effective-dated modeling in Mongo

- MongoDB explicitly scopes its Document Versioning Pattern to
  infrequently-updated, low-volume versioning with separately-queried history
  — criteria a high-churn all-workers projection violates. Its two-collection
  design makes as-of reads expensive and its documented update flow is a
  non-atomic two-step.
- The verified fit is bitemporal/SCD modeling: `validDate` (Workday effective
  date) + `transactionTime` (ingest time) per document; as-of reads filter on
  valid date, sort `transactionTime` descending, `limit 1`, backed by a
  composite index. MongoDB documents a Slowly Changing Dimensions pattern
  alongside Document Versioning. Future-dated hires land as forward-dated
  valid-time records. (Bitemporal case-study evidence is financial-services
  data — applicability to HR is by analogy.)
- If time-series collections are ever used, key buckets on `transactionTime`,
  not `validDate` (effective-dated backfills write into closed buckets).
- CDC→Kafka leg: MongoDB publishes an official comparison of the Kafka Source
  Connector (self-hosted Kafka Connect; SSL/TLS, X.509, AWS IAM) vs Atlas
  Stream Processing (fully managed; `$source`/`$emit` pipelines; in-pipeline
  `$lookup` enrichment and windowing; private networking via VPC
  peering/Private Link; vendor-claimed ~25% of connector cost, per-SPI
  billing). ASP is the default absent an existing Kafka Connect estate.
  Debezium's MongoDB connector page yielded no verifiable claims this run.

## Not verified — read before acting

- **Security/compliance (research area 6) is an explicit coverage gap**: no
  ISU/ISSG, OAuth-vs-WS-Security, or field-redaction claims survived
  verification. Extracted-but-unverified leads worth confirming against
  Workday admin docs: every `Get_Workers` response entity is gated by domain
  security policies (least-privilege ISSG scoping enforces field-level bounds
  Workday-side; e.g. "Worker Data: Public Worker Reports" gates the service,
  "Worker Data: Workers" gates extractable workers); redaction appears
  achievable at response-group granularity only, not per-field; each
  integration should have a dedicated ISU, ISSG-only membership, session
  timeout 0; ISSG policy changes require explicit activation; Microsoft Entra
  still uses basic ISU auth with IP-allowlist + constrained-ISSG compensating
  controls; REST OAuth refresh tokens are bound to an ISU, so ISSG
  least-privilege carries over to REST. Workday compliance posture: HIPAA
  third-party attestation for Enterprise Products, SOC 2 Type II (all five
  TSC) with NIST CSF/800-171 mappings, ISO 27018 and 27701; FedRAMP Moderate
  is Government Cloud only. Treat all of this as unresearched input to the
  `HrPiiGovernanceReview` gate, not settled fact.
- Workday's authoritative WWS docs are behind community.workday.com login —
  rate limits, tenant throttling numbers, and the WWS `Count ≤ 999` cap pinned
  in the crate were **not independently confirmed**.
- 2–1 verification votes (one dissent each): MuleSoft parallel-pagination
  mechanics; the ">1,000 transaction-type IDs" figure.
- Several verifications used search-indexed excerpts rather than direct page
  fetches (Fivetran, Snowflake Openflow) due to a session tooling outage;
  quotes were confirmed across multiple independent extractions.
- Time-sensitive: recheck Workday Data Cloud GA status, Snowflake Kafka
  Connector v4 posture, and Fivetran's incremental table list before
  commitment.

## Open questions

1. Concrete least-privilege ISU + constrained-ISSG configuration for a
   read-only `Get_Workers` sync over PII-heavy worker data, and whether to
   move from WS-Security username/password to OAuth 2.0 (incl. rotation
   policy) for a healthcare tenant.
2. Actual tenant-level throttling/concurrency limits for parallel WWS page
   fetches, and whether `As_Of_Entry_DateTime` pinning fully guarantees
   cross-page consistency during an active tenant — i.e., safe parallelism
   for the Rust sync worker.
3. Complete tombstone/rescind detection mechanism: are rescind/correction
   transaction types plus `Exclude_Inactive_Workers=false` sufficient, or is
   the periodic full-ID-set diff the only complete mechanism, and at what
   cadence?
4. Workday Data Connect at GA: object coverage, latency, pricing, and
   PII-governance controls — adopt as a parallel analytics feed to Snowflake?

## Repo implications

- Fill `appfw_provider_workday/docs/vendor-contract.md` §4a (kappa coverage
  map) with Worker, Worker Event History, Organization, Location, and Job
  Profile rows supported by this research; classify worker-level rows
  PII-heavy per §10.
- The sync descriptor design for `workday_hcm_v1` must model: two independent
  watermark windows (never combined), as-of queries for future-dated hires,
  scheduled full-snapshot reconciliation, tombstones from full-ID-set diff,
  and no transaction-type filtering.
- Tombstones must flow to Kafka as explicit events (Snowflake Kafka Connector
  is append-only) and respect the object-map deletion contract in
  `docs/runtime/saas-connectors.md`.

## Sources (fetched 2026-07-22)

Primary: developer.workday.com WQL REST API reference;
fivetran.com/docs/connectors/applications/workday-hcm;
learn.microsoft.com Entra Workday integration reference;
mongodb.com Kafka Connector ↔ Atlas Stream Processing comparison (v1.16);
mongodb.com Document Versioning Pattern;
docs.snowflake.com Openflow Connector for Workday;
docs.snowflake.com Kafka Connector (v4);
newsroom.workday.com Workday Data Cloud announcement (2025-09-16);
workday.com trust/compliance.

Secondary (blog/forum quality): blogs.mulesoft.com Workday integrations and
parallel pagination; sglmr.com Get_Workers API notes (×2);
learn.microsoft.com Q&A future-dated-hire thread; medium.com/mongodb
bitemporal case study; mongodb.com Atlas Stream Processing cost post;
reco.ai Workday integration/REST security posts (×2).

Yielded no verifiable claims (fetched, discarded): fivetran.com workday-raas;
docs.airbyte.com source-workday; debezium.io MongoDB connector;
streamkap.com CDC soft-deletes; sglmr.com minimum-workday-isu-security.
