# Salesforce Connector Contract

<!-- appfw:vendor-contract-template v1
     Location: appfw_provider_salesforce/docs/vendor-contract.md
     Gated by: scripts/appfw framework docs-check --json
       (maintainability-contract required_docs/required_tokens; saas-vendor-doc-parity
        asserts every pin in the Version Link block equals the crate's exported constant).
     Rule: values here are TRANSCRIBED from crate constants, never hand-invented.
     If evidence does not exist, write "absent — evidence-gated" and name the blocking artifact. -->

## Version Link

| Field | Value | Source of truth (must match) |
| --- | --- | --- |
| Vendor platform release | Salesforce "Summer '26" | `SALESFORCE_RELEASE_PIN` (`src/metadata.rs`) |
| Vendor API version pin | API 67.0 (REST `v67.0`, base `/services/data/v67.0`) | `SALESFORCE_API_VERSION_PIN` (`src/metadata.rs`); `SALESFORCE_REST_API_VERSION`, `SALESFORCE_REST_BASE_PATH` (`src/operation.rs`) |
| API doc / OpenAPI snapshot version | Health Cloud object reference 262.0 | `SALESFORCE_DOC_VERSION_PIN` (`src/metadata.rs`) |
| Evidence artifact + date | `vendor_api_specs_2026-06-26/enrichment/information-package-versions.json`, `vendor_api_specs_2026-06-26/enrichment/saas-provider-implementation-metadata.json`, `vendor_api_specs_2026-06-26/enrichment/extracted/salesforce-health-cloud-catalog.json` (2026-06-26); pinned developer.salesforce.com Health Cloud object-reference and REST API doc URLs | `src/metadata.rs` `source_refs` (`API_SNAPSHOT_SOURCES`) |
| Information package version | 2026.06.26.1 | `SALESFORCE_INFORMATION_PACKAGE_VERSION` |
| Crate version | appfw-provider-salesforce 0.1.0 | `Cargo.toml` |
| Doc last reconciled | 2026-07-02 | this file |

## 1. Identity & Family

- `provider_key`: `salesforce` (`SALESFORCE_PROVIDER_KEY`, `src/identity.rs`).
- `vendor_key`: `salesforce-health-cloud` (`SALESFORCE_VENDOR_KEY`, `src/metadata.rs`).
- API family: Salesforce REST (REST/JSON; SOQL via `/query`, sObject REST primitives) plus Health Cloud object reference; transport is `SaasTransportProtocol::RestJson` from `appfw_saas_core`.
- API base path: `/services/data/v67.0` (`SALESFORCE_REST_BASE_PATH`).
- Spec package name: `salesforce-health-cloud-api-specs` (`SALESFORCE_API_SPEC_PACKAGE_ID`).
- Data-source binding name(s): caller-supplied `data_source_name` via `SalesforceProviderDescriptor` (`src/identity.rs`); no fixed binding is registered in framework config yet — crate contract tests use `salesforce_primary`.

## 2. Auth Contract

**Flows** (from `src/auth.rs`, verbatim — including honest values like `access_gated_unknown`):

| Flow | Status | Notes |
| --- | --- | --- |
| jwt_bearer | declared — crate defines no preferred/exception ranking | Integration-user OAuth (`SalesforceAuthConfigMetadata::integration_user_oauth()`, `integration_user: true`). Planning-only: no token acquisition executes; ConnectionAuth is `saas_unsupported` in the capability matrix. |
| client_credentials | declared — crate defines no preferred/exception ranking | Same integration-user metadata surface; does not require the JWT subject env var. Planning-only, no execution. |

**Environment contract** (transcribed from the typed env constants in `src/auth.rs`):

| Env var | Required for | Secret | Description (verbatim) |
| --- | --- | --- | --- |
| `SALESFORCE_INSTANCE_URL` | all flows | no | "Bound Salesforce org instance URL used for REST API requests." |
| `SALESFORCE_LOGIN_BASE_URL` | all flows | no | "Salesforce OAuth login base URL, such as login or test." |
| `SALESFORCE_CLIENT_ID` | all flows | no | "Connected app OAuth client id." |
| `SALESFORCE_CLIENT_SECRET_OR_PRIVATE_KEY` | all flows | yes | "Client secret for client credentials or private key material/reference for JWT bearer." |
| `SALESFORCE_USERNAME_OR_SUBJECT` | jwt_bearer only | no | "Integration user username or subject for JWT bearer assertions." |

**Redaction:** `Authorization` header; redaction constant `Bearer <redacted>` (`SALESFORCE_AUTHORIZATION_HEADER`, `SALESFORCE_REDACTED_BEARER_TOKEN`).

**Tenant binding validation** (`SalesforceTenantBinding`): both `SALESFORCE_INSTANCE_URL` and `SALESFORCE_LOGIN_BASE_URL` must be single-line HTTPS origins — no whitespace or control characters (CRLF header injection rejected), no path/query/fragment, DNS host only with no userinfo and no port, host must contain a dot.

**Server-bound values callers cannot override:** org instance URL and login base URL (bound via `SalesforceTenantBinding`); SOQL text, object, and field selection are fixed per named operation ("callers cannot choose SOQL, object, or fields"); REST paths are provider-built, and `nextRecordsUrl` continuation must be a Salesforce-relative REST path — absolute URLs are rejected.

## 3. Operation Catalog

**Status vocabulary (exactly four values — do not invent others):**
- `live_certified` — retained live evidence per docs/runtime/saas-certification.md. **None exists for any SaaS provider today; the column stays anyway.**
- `compiler_contracted` — network-free request-**plan** construction proven by unit contracts. **NOT live-callable** until provider binding, auth, redaction, and live-certification evidence are retained through the runtime SaaS executor; never conflate request planning with executable-against-vendor.
- `planned_gated` — registered, non-executable; gates listed per row.
- `write_gated` — registered write candidate, non-executable pending G1 governed-write evidence.

| Operation | Kind (read/write) | Status | Gates (verbatim enum members) | PII/PHI flag | Notes |
| --- | --- | --- | --- | --- | --- |
| `salesforce.account.describe_object` | read | compiler_contracted | — | Operational | Fixed Account describe (`/sobjects/Account/describe`) for drift checks. |
| `salesforce.account_by_id` | read | compiler_contracted | — | BusinessConfidential | Fixed Account projection by id; SOQL LIMIT 1. |
| `salesforce.account.fetch_by_ids` | read | compiler_contracted | — | BusinessConfidential | Bounded id-list projection; rejects lists exceeding `limits.max_results`. |
| `salesforce.account.get_updated_ids` | read | compiler_contracted | — | BusinessConfidential | getUpdated ID-only read; UTC `start`/`end` window validated. |
| `salesforce.account.get_deleted_ids` | read | compiler_contracted | — | BusinessConfidential | getDeleted ID-only read; UTC `start`/`end` window validated. |
| `salesforce.query.continue_page` | read | compiler_contracted | — | BusinessConfidential | Continues a query page from a validated relative `nextRecordsUrl`. |
| `salesforce.updated_accounts` | read | compiler_contracted | — | BusinessConfidential | Incremental SOQL read on `SystemModstamp` with Id tie-break ordering. |
| `salesforce.health.fetch_limits` | read | compiler_contracted | — | Operational | REST org `/limits` read for smoke tests and rate visibility. |
| `salesforce.health.describe_object` | read | planned_gated | `LiveOrgDescribe`, `GovernanceReview`, `LiveCertification` | PhiPii | Generic Health Cloud describe gated until authenticated org describe evidence. |
| `salesforce.health.get_updated_ids` | read | planned_gated | `LiveOrgDescribe`, `GovernanceReview`, `LiveCertification` | PhiPii | Object-allowlist and org-describe gated. |
| `salesforce.health.get_deleted_ids` | read | planned_gated | `LiveOrgDescribe`, `GovernanceReview`, `LiveCertification` | PhiPii | Object-allowlist and org-describe gated. |
| `salesforce.health.fetch_by_ids` | read | planned_gated | `LiveOrgDescribe`, `GovernanceReview`, `LiveCertification` | PhiPii | Needs certified fixed projections first. |
| `salesforce.health.query_object_incremental` | read | planned_gated | `LiveOrgDescribe`, `GovernanceReview`, `LiveCertification` | PhiPii | Must not expose caller-selected objects or fields. |
| `salesforce.health.care_programs_updated_since` | read | planned_gated | `LiveOrgDescribe`, `GovernanceReview`, `LiveCertification` | PhiPii | CareProgram projection candidate (catalog: 32 fields). |
| `salesforce.health.care_program_enrollees_by_program` | read | planned_gated | `LiveOrgDescribe`, `GovernanceReview`, `LiveCertification` | PhiPii | CareProgramEnrollee projection candidate (catalog: 25 fields). |
| `salesforce.health.coverage_benefit_items_by_member` | read | planned_gated | `LiveOrgDescribe`, `GovernanceReview`, `LiveCertification` | PhiPii | CoverageBenefitItem projection candidate (catalog: 23 fields). |
| `salesforce.health.clinical_encounters_updated_since` | read | planned_gated | `LiveOrgDescribe`, `GovernanceReview`, `LiveCertification` | PhiPii | ClinicalEncounter projection candidate (catalog: 30 fields). |

No `write_gated` rows: no Salesforce named-mutation candidate is registered. `SalesforceProvider::build_named_mutation_plan()` always returns `UnsupportedMutation("Wave 1 Salesforce skeleton exposes read operation planning only")` (`src/provider.rs`). The fleet's first named governed-write candidate (`servicenow.create_incident`, write_gated) lives in `appfw_provider_servicenow`, not here. No `live_certified` rows exist.

Write posture — shared G1 reasons, verbatim from `appfw_runtime/src/provider_capabilities.rs:696-711`:

- `SAAS_GOVERNED_WRITE_UNSUPPORTED_REASON`: "SaaS provider writes remain unsupported until named mutation safety, idempotency, audit, and live write evidence exist"
- `SAAS_DELEGATED_ACTOR_UNSUPPORTED_REASON`: "SaaS delegated/on-behalf-of actor context remains unsupported until per-user token storage, tenant binding, and impersonation audit evidence exist"
- `SAAS_TOKEN_STORE_UNSUPPORTED_REASON`: "SaaS per-user token-store isolation remains unsupported until encrypted storage, rotation, tenant partitioning, and revocation evidence exist"
- `SAAS_NAMED_MUTATION_UNSUPPORTED_REASON`: "SaaS named mutations remain unsupported until a provider-specific mutation registry and deny-by-default exposure gate exist"
- `SAAS_MUTATION_BINDING_UNSUPPORTED_REASON`: "SaaS mutation request binding remains unsupported until provider-specific request templates, parameter binding, and redaction evidence exist"
- `SAAS_IDEMPOTENCY_UNSUPPORTED_REASON`: "SaaS write idempotency and replay protection remain unsupported until idempotency keys, retry classification, and duplicate-suppression evidence exist"
- `SAAS_WRITE_POLICY_UNSUPPORTED_REASON`: "SaaS write policy and scope enforcement remain unsupported until operation-level policy, MCP/Kafka/frontend exposure gates, and least-privilege scopes are proven"
- `SAAS_WRITE_AUDIT_UNSUPPORTED_REASON`: "SaaS write audit remains unsupported until before/after redaction, request correlation, undo/compensation posture, and retained evidence exist"

## 4. Data Paths

### 4a. Stream-fed (kappa) coverage map
| SaaS object | Streams into Mongo? | Mongo collection | CDC topic | Classification | Status |
| --- | --- | --- | --- | --- | --- |
| Account | **UNKNOWN — to be filled by data platform team** | — | — | — | placeholder |
| CareProgram | **UNKNOWN — to be filled by data platform team** | — | — | — | placeholder |
| CareProgramEnrollee | **UNKNOWN — to be filled by data platform team** | — | — | — | placeholder |
| CoverageBenefitItem | **UNKNOWN — to be filled by data platform team** | — | — | — | placeholder |
| ClinicalEncounter | **UNKNOWN — to be filled by data platform team** | — | — | — | placeholder |

Covered objects are read as Mongo projections (no sync worker); CDC topics feed reactive projections via governed Kafka ingress. Provenance markers, echo-loop exclusions, and stream-offset freshness apply per docs/runtime/saas-connectors.md.

### 4b. Pull-fed (sync worker — coverage gaps)
Watermark field(s): `SystemModstamp` (exclusive-after datetime watermark on `salesforce.updated_accounts`, with `Id` tie-break ordering; `src/watermark.rs`); getUpdated uses `SystemModstamp` and getDeleted uses `LastModifiedDate` as registry watermark fields over paired UTC `start`/`end` windows. Sync descriptor names: **absent — evidence-gated** (no Archetype-1 sync descriptors are registered for Salesforce; sync-worker execution is a config/validation shell only, fail-closed). Projection store binding: **not yet bound — record the per-schema choice at descriptor registration** (Mongo is the default candidate where 4a covers adjacent objects; Postgres remains fully supported).

### 4c. Live (Archetype-2)
Named live-read operations: none are live-callable — the eight `compiler_contracted` reads in section 3 are request-plan construction only. Runtime HTTP executor substrate exists, but Salesforce live execution still waits on provider binding, authenticated org credentials, redaction evidence, and live ConnectionAuth evidence. Governed-write candidates: none registered for Salesforce (cross-reference section 3 — zero `write_gated` rows; mutation planning is rejected wholesale).

## 5. Pagination & Continuation
Shared-kernel style: `UrlCursorContinuation`. Vendor field: `nextRecordsUrl` (`SALESFORCE_NEXT_RECORDS_URL_FIELD`, `src/response.rs`), consumed by `salesforce.query.continue_page`. Validation is fail-closed: the continuation must be a Salesforce-relative REST path under `/services/data/v67.0` (absolute URLs rejected), the query locator charset is validated, `nextRecordsUrl` must be present when `done` is `false` and absent when `done` is `true`, and `queryAll` locators are detected (`is_query_all`).

## 6. Limits, Caps & Rate-Limit Posture
Defaults: 250 rows / 2,000 ms (`SalesforceQueryLimits::DEFAULT_MAX_RESULTS`, `DEFAULT_TIMEOUT_MS`). Vendor hard cap: SOQL 2,000 rows (`SOQL_MAX_LIMIT`) — requested `max_results` is clamped and the SOQL `LIMIT` matches the plan caps; describe and limits reads cap `max_rows` at 1; `max_results` of 0 is rejected. Rate-limit signals: `Sforce-Limit-Info` header parsed into shared `RateLimitSignal` without exposing raw header values; `REQUEST_LIMIT_EXCEEDED` error code and HTTP 429 classify as `RateLimited`; `Retry-After` honored via shared `RetryDecision`/`RetryPolicy` backoff. REST error classification (`classify_salesforce_rest_error`): 400 InvalidRequest, 401 Authentication, 403 Authorization, 404 NotFound, 409 Conflict, 414/431 RequestTooLarge, 304/412/428 ConditionalRequest, 500/502/503 Transient; only RateLimited and Transient are retryable. Circuit-breaker posture is **not live-certified**: "Salesforce rate-limit signals and retry decisions are compiler-contracted; circuit-breaker behavior still requires live tenant evidence" (`SALESFORCE_RATE_LIMIT_BACKOFF_PENDING_REASON`).

## 7. Capability Matrix Summary
Transcribed from `SALESFORCE_SAAS_CAPABILITIES` (`appfw_runtime/src/provider_capabilities.rs:1271-1354`); 12 read + 8 write areas.

| # | SaasReadArea | Status |
| --- | --- | --- |
| 1 | ConnectionAuth | saas_unsupported — "requires live tenant evidence before this SaaS contract area can be certified" |
| 2 | NamedOperationRegistry | saas_compiler_contracted |
| 3 | RequestBinding | saas_compiler_contracted |
| 4 | PaginationCursoring | saas_compiler_contracted |
| 5 | RateLimitBackoff | saas_unsupported_with_evidence — compiler evidence retained; circuit-breaker requires live tenant evidence |
| 6 | IncrementalWatermark | saas_compiler_contracted |
| 7 | FieldRedaction | saas_compiler_contracted |
| 8 | TenantScoping | saas_compiler_contracted |
| 9 | SchemaVersionPinning | saas_compiler_contracted |
| 10 | ResultAndTimeoutCaps | saas_compiler_contracted |
| 11 | QueryMetricsAndAudit | saas_unsupported |
| 12 | FreshnessReporting | saas_unsupported |
| 13 | GovernedWriteEnforcement | saas_unsupported_with_evidence — deny-path compiler evidence (`provider_rejects_mutation_planning_for_now`) |
| 14 | DelegatedActorContext | saas_unsupported |
| 15 | TokenStoreIsolation | saas_unsupported |
| 16 | NamedMutationRegistry | saas_unsupported |
| 17 | MutationRequestBinding | saas_unsupported |
| 18 | IdempotencyAndReplayProtection | saas_unsupported |
| 19 | WritePolicyAndScopeEnforcement | saas_unsupported |
| 20 | WriteAuditAndEvidence | saas_unsupported |

Asymmetries, stated honestly: Salesforce FieldRedaction and RateLimitBackoff carry compiler evidence while Workday's FieldRedaction and RateLimitBackoff are plain unsupported; Salesforce is currently the only provider with a compiler-contracted rate-limit signal surface. No area on any SaaS provider is live-certified.

## 8. Testing Tiers Status
| Tier | Proof command | Status |
| --- | --- | --- |
| Local | `cargo test -p appfw-provider-salesforce`; fixtures under `fixtures/operations/` (provenance: synthetic \| vendor_export \| recorded_live) | Unit contracts pass in-crate (module tests in `src/*`). Fixtures directory: **absent — evidence-gated** (no `fixtures/operations/` exists in this crate yet; all current evidence is synthetic in-test data). |
| Pipeline | `scripts/appfw framework provider-test --provider salesforce --area saas-read --plan --json`; `scripts/appfw framework provider-test --provider salesforce --area governed-write --plan --json`; `scripts/appfw framework provider-graduation --json` | `--area saas-read --plan` exists and retains `target/appfw/saas-read-provider-test.json` with the 12 read-area checks, current compiler-contracted/unsupported rows, and `release_ready:false` until live smoke evidence exists. The governed-write lane accepts salesforce in plan mode (artifact `target/appfw/governed-write-provider-test.json`), but its known named-mutation allowlist for salesforce is empty — no write candidate is registered, so live mode has nothing to certify. |
| Dev | `cargo test -p appfw-provider-salesforce --test live_smoke -- --ignored` → evidence file → provider-test live → provider-graduation | **absent — evidence-gated**: no `tests/live_smoke.rs` exists; blocked on authenticated org credentials, runtime provider binding, and live ConnectionAuth evidence. |

## 9. Graduation Status & Outstanding Evidence Gates
Current ladder position: **CompilerContracted** for the eight read areas listed in section 7 (matches the U4 provider-graduation sample in docs/architecture/concerns/north-star-wave-0-specs.md: 8 graduated compiler-contracted capabilities, profile `saas_read`, family `external_api`). This is "guardrail-complete, live-pending" per docs/runtime/saas-certification.md — no area may claim LiveCertified without retained live evidence.

Remaining gates checklist:
- [ ] Live ConnectionAuth evidence (authenticated org token acquisition) — blocks area 1.
- [ ] Runtime executor provider binding (`RuntimeExecutor` gate) — HTTP executor substrate exists, but Salesforce live execution remains blocked until provider binding/auth/redaction/live evidence is retained.
- [ ] `LiveOrgDescribe`: authenticated Salesforce org describe evidence — blocks all nine Health Cloud planned_gated reads (owning unit: whoever holds org credentials; mirrors the "Unit A: authenticated instance export" pattern used for ServiceNow).
- [ ] `GovernanceReview`: PHI/PII governance review for Health Cloud objects.
- [ ] `LiveCertification`: retained live certification runs via `scripts/appfw framework provider-test --provider salesforce ... ` live mode and `provider-graduation`.
- [ ] Live rate-limit/circuit-breaker tenant evidence — promotes RateLimitBackoff from unsupported_with_evidence.
- [ ] Full G1 governed-write stack (all eight write areas; see section 3 verbatim reasons) before any Salesforce mutation candidate may be registered.
- [ ] Data platform team fills the 4a kappa coverage map.

## 10. PHI / Data Classification Notes
- PII/PHI-heavy surface: all Health Cloud operations and projection candidates carry `SalesforceDataSensitivity::PhiPii` and are DescribeGated — CareProgram (32 catalog fields), CareProgramEnrollee (25), CoverageBenefitItem (23, includes `MemberId`), ClinicalEncounter (30, includes `PatientId`). Account operations are `BusinessConfidential`; describe/limits reads are `Operational`.
- Governance-review gates: `GovernanceReview` ("requires PHI/PII governance review") is mandatory on every Health Cloud row before execution, alongside `LiveOrgDescribe` and `LiveCertification`.
- Response handling is redaction-first: query records are surfaced as `SalesforceRedactedRecordPayload` (id, sObject type, field count only), and rate-limit signals never expose raw header values.
- Retention and tombstone expectations for any projection of Salesforce data: **absent — evidence-gated**; to be defined when sync descriptors or kappa coverage rows are registered, and the classification column in 4a must be filled by the data platform team at that time (Health Cloud rows are presumptively PHI).
