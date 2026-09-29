# ServiceNow Connector Contract

<!-- appfw:vendor-contract-template v1
     Location: appfw_provider_servicenow/docs/vendor-contract.md
     Gated by: scripts/appfw framework docs-check --json
       (maintainability-contract required_docs/required_tokens; saas-vendor-doc-parity
        asserts every pin in the Version Link block equals the crate's exported constant).
     Rule: values here are TRANSCRIBED from crate constants, never hand-invented.
     If evidence does not exist, write "absent — evidence-gated" and name the blocking artifact. -->

## Version Link

| Field | Value | Source of truth (must match) |
| --- | --- | --- |
| Vendor platform release | unpinned — evidence-gated: authenticated ServiceNow instance export. Release basis is "Australia public docs; authenticated instance export required" | `SERVICENOW_RELEASE_BASIS` (`src/identity.rs`) |
| Vendor API version pin | unpinned — evidence-gated: this crate deliberately builds no ServiceNow Table API requests; no API version or base-path constant exists to transcribe | `src/lib.rs` crate doc; no operation constant exists |
| API doc / OpenAPI snapshot version | n/a — public-docs summary only (`vendor_api_specs_2026-06-26/enrichment/extracted/servicenow-public-docs-summary.json`); Table API public doc URL pinned as `SERVICENOW_TABLE_API_DOC_URL` | `src/metadata.rs` `SERVICENOW_PUBLIC_DOCS_SUMMARY_PATH`, `SERVICENOW_TABLE_API_DOC_URL` |
| Evidence artifact + date | **absent — evidence-gated** (authenticated ServiceNow instance OpenAPI/dictionary export, Unit A). Crate `blocking_gap`: "Need authenticated ServiceNow instance export for table fields, ACLs, plugins, domains, and OpenAPI schemas." | `src/metadata.rs` `servicenow_api_snapshot_metadata()` `source_refs` / `blocking_gap` |
| Information package version | 2026.06.26.1 | `SERVICENOW_INFORMATION_PACKAGE_VERSION` (`src/metadata.rs`) |
| Crate version | appfw-provider-servicenow 0.1.0 | `Cargo.toml` |
| Doc last reconciled | 2026-07-02 | this file |

## 1. Identity & Family

- `provider_key`: `servicenow` (`SERVICENOW_PROVIDER_KEY`, `src/identity.rs`)
- `vendor_key`: `servicenow-australia` (`SERVICENOW_VENDOR_KEY`, `src/metadata.rs`)
- API family: `ServiceNow Table API / REST API Explorer export` (`SERVICENOW_API_FAMILY`)
- API base path: **absent — evidence-gated**. The crate validates only a tenant HTTPS origin (`SERVICENOW_BASE_URL`); no `/api/now/...` path constant exists because request construction awaits the authenticated instance export.
- Spec package name: `servicenow-australia-api-specs` (`SERVICENOW_API_SPEC_PACKAGE_ID`)
- Data-source binding name(s): caller-supplied per instance via `ServiceNowProvider::new(data_source_name)`; the in-crate contract tests use `servicenow_primary`.

## 2. Auth Contract

**Flows** (flow names verbatim from `ServiceNowAuthFlow::as_str()`, `src/auth.rs`):

| Flow | Status | Notes |
| --- | --- | --- |
| oauth_client_credentials | preferred | Metadata only; live auth remains certification-gated ("Tenant-approved OAuth client id, once authenticated evidence exists"). No token exchange is executed by this crate. |
| basic_user_password | exception | "Temporary live-smoke integration username, if basic auth is approved" — temporary live-smoke only. |
| delegated_oauth_later | placeholder | Declared Archetype-2 delegated-auth placeholder — no execution; delegated OAuth waits on the G1 evidence stack (section 3). |

**Environment contract** (transcribed from the typed env constants in `src/auth.rs`):

| Env var | required_for | secret | Description (verbatim) |
| --- | --- | --- | --- |
| `SERVICENOW_BASE_URL` | all flows | no | Tenant-specific ServiceNow HTTPS origin. |
| `SERVICENOW_AUTH_MODE` | all flows | no | Tenant-confirmed auth mode; live auth remains certification-gated. |
| `SERVICENOW_CLIENT_ID` | oauth_client_credentials | no | Tenant-approved OAuth client id, once authenticated evidence exists. |
| `SERVICENOW_CLIENT_SECRET` | oauth_client_credentials | yes | Tenant-approved OAuth client secret or secret reference. |
| `SERVICENOW_USERNAME` | basic_user_password | no | Temporary live-smoke integration username, if basic auth is approved. |
| `SERVICENOW_PASSWORD` | basic_user_password | yes | Temporary live-smoke integration password or secret reference. |
| `SERVICENOW_INTEGRATION_PRINCIPAL` | all flows | no | Server-bound integration principal whose ACLs/roles are certified. |
| `SERVICENOW_DOMAIN_SCOPE` | all flows (optional value) | no | Optional server-bound ServiceNow domain separation scope. |

**Redaction:** absent — evidence-gated. This crate constructs no HTTP requests, so no `Authorization` header or redaction constant exists yet; when request planning lands it must adopt the shared `appfw_saas_core` redaction posture before any authenticated evidence run.

**Tenant binding validation** (`ServiceNowTenantBinding`, `src/auth.rs`): base URL must be a single-line `https://` origin with no whitespace or control characters, no path/query/fragment, and a DNS host without userinfo or port (a literal `host.domain` is required); the integration principal and optional domain scope must be single-line, non-empty, with CR/LF and control characters rejected.

**Server-bound values callers cannot override:** `SERVICENOW_INTEGRATION_PRINCIPAL` (the integration principal whose ACLs/roles are certified) and `SERVICENOW_DOMAIN_SCOPE` (domain-separation scope) are validated into the tenant binding server-side; callers cannot inject raw URLs, paths, or `sysparm_query` strings (`servicenow.table.raw_sysparm_query` is rejected as an unknown operation).

## 3. Operation Catalog

**Status vocabulary (exactly four values — do not invent others):**
- `live_certified` — retained live evidence per docs/runtime/saas-certification.md. **None exists for any SaaS provider today; the column stays anyway.**
- `compiler_contracted` — network-free request-**plan** construction proven by unit contracts. **NOT live-callable** until provider binding, auth, redaction, and live-certification evidence are retained through the runtime SaaS executor; never conflate request planning with executable-against-vendor.
- `planned_gated` — registered, non-executable; gates listed per row.
- `write_gated` — registered write candidate, non-executable pending G1 governed-write evidence.

| Operation | Kind (read/write) | Status | Gates (verbatim enum members) | PII/PHI flag | Notes |
| --- | --- | --- | --- | --- | --- |
| `servicenow.table.export_schema_from_instance` | read | planned_gated | AuthenticatedInstanceOpenApiExport, TableDictionaryAndAclExport, LiveCertification | unclassified — evidence-gated: awaiting authenticated export | Capture authenticated REST API Explorer OpenAPI and table dictionary exports before request planning. |
| `servicenow.table.query_incremental` | read | planned_gated | AuthenticatedInstanceOpenApiExport, TableDictionaryAndAclExport, LiveCertification | unclassified — evidence-gated: awaiting authenticated export | Planned incremental table query using provider-owned table, fields, and sys_updated_on/sys_id predicates. |
| `servicenow.table.fetch_by_sys_ids` | read | planned_gated | AuthenticatedInstanceOpenApiExport, TableDictionaryAndAclExport, LiveCertification | unclassified — evidence-gated: awaiting authenticated export | Planned bounded fetch by provider-owned table and caller-bound sys_id values. |
| `servicenow.table.fetch_reference_values` | read | planned_gated | AuthenticatedInstanceOpenApiExport, TableDictionaryAndAclExport, LiveCertification | unclassified — evidence-gated: awaiting authenticated export | Planned reference value fetch after selected reference fields are exported. |
| `servicenow.create_incident` | write | write_gated | DelegatedActorContextEvidence, TokenStoreIsolationEvidence, NamedMutationRegistryEvidence, MutationRequestBindingEvidence, IdempotencyReplayEvidence, WritePolicyScopeEvidence, WriteAuditEvidence, LiveCertification | unclassified — evidence-gated: incident payload fields await authenticated export | First named governed-write candidate; policy scope `SERVICENOW_INCIDENT_CREATE_POLICY_SCOPE = "servicenow.incident.write"`. Registered as a fail-closed local guardrail only. Non-executable until live G1 evidence proves delegated actor context, token-store isolation, mutation request binding, idempotency, policy/scope enforcement, audit, and live certification. |

Additional gate enum members reserved for later rows (`ServiceNowOperationGate`, `src/registry.rs`): `DomainSeparationEvidence` ("requires tenant domain-separation evidence when enabled"), `EncodedQueryBuilderCertification` ("requires encoded-query builder certification before sysparm_query use").

Every registered operation is fail-closed: the registry exposes only `ensure_operation_is_not_executable` / `ensure_named_read_is_not_executable` / `ensure_named_mutation_is_not_executable`, and named-read and named-mutation paths reject cross-use (a mutation submitted to the read path fails with "operation is a named mutation and must use the governed-write path", and vice versa).

**Write posture** — shared G1 reasons, verbatim from `appfw_runtime/src/provider_capabilities.rs:696-711`:

- GovernedWriteEnforcement: "SaaS provider writes remain unsupported until named mutation safety, idempotency, audit, and live write evidence exist"
- DelegatedActorContext: "SaaS delegated/on-behalf-of actor context remains unsupported until per-user token storage, tenant binding, and impersonation audit evidence exist"
- TokenStoreIsolation: "SaaS per-user token-store isolation remains unsupported until encrypted storage, rotation, tenant partitioning, and revocation evidence exist"
- NamedMutationRegistry: "SaaS named mutations remain unsupported until a provider-specific mutation registry and deny-by-default exposure gate exist"
- MutationRequestBinding: "SaaS mutation request binding remains unsupported until provider-specific request templates, parameter binding, and redaction evidence exist"
- IdempotencyAndReplayProtection: "SaaS write idempotency and replay protection remain unsupported until idempotency keys, retry classification, and duplicate-suppression evidence exist"
- WritePolicyAndScopeEnforcement: "SaaS write policy and scope enforcement remain unsupported until operation-level policy, MCP/Kafka/frontend exposure gates, and least-privilege scopes are proven"
- WriteAuditAndEvidence: "SaaS write audit remains unsupported until before/after redaction, request correlation, undo/compensation posture, and retained evidence exist"

## 4. Data Paths

### 4a. Stream-fed (kappa) coverage map
| SaaS object | Streams into Mongo? | Mongo collection | CDC topic | Classification | Status |
| --- | --- | --- | --- | --- | --- |
| incident | **UNKNOWN — to be filled by data platform team** | — | — | — | placeholder |
| (all other ServiceNow tables) | **UNKNOWN — to be filled by data platform team** | — | — | — | placeholder |

Covered objects are read as Mongo projections (no sync worker); CDC topics feed reactive projections via governed Kafka ingress. Provenance markers, echo-loop exclusions, and stream-offset freshness apply per docs/runtime/saas-connectors.md.

### 4b. Pull-fed (sync worker — coverage gaps)
Watermark field(s): planned `sys_updated_on` with `sys_id` tie-breaker predicates (per the `servicenow.table.query_incremental` registration) — planned only; encoded-query construction is gated by `EncodedQueryBuilderCertification` and the authenticated export. Sync descriptor names: **absent — evidence-gated**; no sync descriptors exist for this provider (Archetype-1 sync-worker execution is a config/validation shell only, fail-closed). Projection store binding: not yet bound — Mongo is the default where 4a covers adjacent objects; Postgres remains fully supported; record the per-schema choice and rationale when the first ServiceNow projection schema is defined.

### 4c. Live (Archetype-2)
Named live-read operations: none executable — the four reads in section 3 are `planned_gated` on the authenticated instance export. Governed-write candidates: `servicenow.create_incident` (see section 3 `write_gated` row) — the first Archetype-2 governed-write candidate on the critical path (headless ServiceNow), policy scope `servicenow.incident.write`, per the G1 lane in docs/architecture/concerns/north-star-wave-0-specs.md.

## 5. Pagination & Continuation
Not modeled — evidence-gated: awaiting authenticated export. The crate builds no Table API requests, so no continuation style (UrlCursorContinuation / PageCountContinuation / OffsetLimitContinuation) is bound and no vendor paging field names are asserted. ServiceNow paging shapes will be transcribed from the authenticated REST API Explorer OpenAPI export, never guessed.

## 6. Limits, Caps & Rate-Limit Posture
Not modeled — evidence-gated: awaiting authenticated export. No default row/timeout constants, no MAX page limit, and no vendor hard caps are pinned in the crate. Rate-limit signals: not modeled — pending live tenant evidence. Any future caps must land as crate constants first and be transcribed here.

## 7. Capability Matrix Summary
All 20 `SaasReadArea` statuses (12 read + 8 write) for `FrameworkProvider::ServiceNow` are `saas_unsupported` via `planned_saas_capabilities(SERVICE_NOW_AUTHENTICATED_EXPORT_REQUIRED_REASON)` (`appfw_runtime/src/provider_capabilities.rs:1023-1024`), with the shared reason verbatim:

> "requires authenticated ServiceNow instance OpenAPI or dictionary export before named-operation contracts can be implemented or certified"

No area is `saas_compiler_contracted` and none is live-certified — unlike Salesforce (8 compiler-contracted read areas) and Workday (7), ServiceNow has zero compiler-contracted areas today. Known asymmetry to state honestly: the crate registry registers the named-mutation candidate `servicenow.create_incident` with fail-closed contract tests (`governed_write_operation_requires_g1_evidence`, `named_read_and_mutation_paths_do_not_cross`), but those tests prove only the local guardrail. The matrix's `NamedMutationRegistry` row intentionally remains Unsupported until real G1 provider-test governed-write evidence exists.

## 8. Testing Tiers Status
| Tier | Proof command | Status |
| --- | --- | --- |
| Local | `cargo test -p appfw-provider-servicenow`; fixtures under `fixtures/operations/` (provenance: synthetic \| vendor_export \| recorded_live) | Network-free fail-closed contract tests in-crate (identity, auth/tenant-binding, registry rejection, metadata parity, governed-write gating). Fixtures directory: **absent — evidence-gated**; no fixtures exist because no request/response shapes may be asserted before the authenticated export. |
| Pipeline | `scripts/appfw framework provider-test --provider servicenow --area saas-read --plan --json`; `scripts/appfw framework provider-test --provider servicenow --area governed-write --plan --json` | The saas-read plan lane exists and retains `target/appfw/saas-read-provider-test.json` with the 12 read-area checks and `release_ready:false` until live smoke evidence exists. The governed-write plan/preflight lane lists `servicenow.create_incident` as `planned_non_executable` in the provider-owned allow-list. |
| Dev | `cargo test -p appfw-provider-servicenow --test live_smoke -- --ignored` → evidence file → provider-test live → provider-graduation | **absent — evidence-gated**: no `live_smoke` test exists; blocked on authenticated instance access (Unit A) and approved live-smoke credentials. |

## 9. Graduation Status & Outstanding Evidence Gates
Ladder position: below `CompilerContracted` for SaaS capabilities — registered fail-closed skeleton (metadata + planned catalog only; zero compiler-contracted SaaS areas in the matrix). Guardrails are complete (deny-by-default registry, read/mutation path segregation, tenant-binding validation, unknown-operation rejection): **guardrail-complete, live-pending** applies to the rejection posture only, not to any vendor-facing capability or `LiveCertified` claim.

Outstanding gates checklist:
- [ ] Unit A: authenticated ServiceNow instance REST API Explorer OpenAPI export (`AuthenticatedInstanceOpenApiExport`)
- [ ] Unit A: table dictionary and ACL export for selected tables (`TableDictionaryAndAclExport`)
- [ ] Tenant domain-separation evidence, if enabled (`DomainSeparationEvidence`)
- [ ] Encoded-query builder certification before any `sysparm_query` use (`EncodedQueryBuilderCertification`)
- [ ] G1 governed-write stack for `servicenow.create_incident`: delegated actor context, token-store isolation, named-mutation registry (MCP disabled), mutation request binding, idempotency/replay, write policy scope, write audit (`DelegatedActorContextEvidence` … `WriteAuditEvidence`)
- [ ] Live connector certification (`LiveCertification`) per docs/runtime/saas-certification.md; graduation summary via `scripts/appfw framework provider-graduation --json`

## 10. PHI / Data Classification Notes
The crate models no PII/PHI flags for ServiceNow objects — classification is **evidence-gated: awaiting authenticated export** (table fields, ACLs, and domain separation are instance-specific). Operational caution: `incident` and other Task-family tables commonly carry requester/caller PII (names, contact details, free-text descriptions); treat them as PII-bearing until the export proves otherwise. Governance-review gates: none registered yet for ServiceNow reads (unlike Salesforce Health Cloud `GovernanceReview` or Workday `HrPiiGovernanceReview`); a classification-driven review gate must be added when the export lands. Retention and tombstone expectations for any ServiceNow projection: **absent — evidence-gated**; define alongside the first projection schema (4b) and classify the kappa coverage rows in 4a once the data platform team fills them.
