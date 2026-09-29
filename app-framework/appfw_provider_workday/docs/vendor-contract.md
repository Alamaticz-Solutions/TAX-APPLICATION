# Workday Connector Contract

<!-- appfw:vendor-contract-template v1
     Location: appfw_provider_workday/docs/vendor-contract.md
     Gated by: scripts/appfw framework docs-check --json
       (maintainability-contract required_docs/required_tokens; saas-vendor-doc-parity
        asserts every pin in the Version Link block equals the crate's exported constant).
     Rule: values here are TRANSCRIBED from crate constants, never hand-invented.
     If evidence does not exist, write "absent — evidence-gated" and name the blocking artifact. -->

## Version Link

| Field | Value | Source of truth (must match) |
| --- | --- | --- |
| Vendor platform release | Workday "2026R1" | `WORKDAY_RELEASE_PIN` (`src/metadata.rs`) |
| Vendor API version pin | WWS v46.1 (Human_Resources SOAP; base `/ccx/service/{tenant}/Human_Resources/v46.1`; every SOAP request body embeds `bsvc:version="v46.1"`) | `WORKDAY_API_VERSION_PIN`, `WORKDAY_WWS_VERSION`, `WORKDAY_SERVICE_NAME_PIN` (`src/metadata.rs`, `src/soap.rs`) |
| API doc / OpenAPI snapshot version | WWS v46.1 summary path pinned as `vendor_api_specs_2026-06-26/enrichment/extracted/workday-human-resources-wws-v46.1-summary.json`; tenant-authenticated request-shape evidence remains absent — evidence-gated | `WORKDAY_WWS_SUMMARY_PATH`; `workday_api_snapshot_metadata().blocking_gap` (`src/metadata.rs`) |
| Evidence artifact + date | `vendor_api_specs_2026-06-26/enrichment/information-package-versions.json`, `vendor_api_specs_2026-06-26/enrichment/saas-provider-implementation-metadata.json`, `vendor_api_specs_2026-06-26/enrichment/extracted/workday-human-resources-wws-v46.1-summary.json` (2026-06-26); tenant-approved WWS request-shape export **absent — evidence-gated** | `src/metadata.rs` `source_refs` / `blocking_gap` |
| Information package version | 2026.06.26.1 | `WORKDAY_INFORMATION_PACKAGE_VERSION` (`src/metadata.rs`) |
| Crate version | appfw-provider-workday 0.1.0 | `Cargo.toml` |
| Doc last reconciled | 2026-07-02 | this file |

## 1. Identity & Family

- `provider_key`: `workday` (`WORKDAY_PROVIDER_KEY`, `src/identity.rs`).
- `vendor_key`: `workday-hcm-wws` (`WORKDAY_VENDOR_KEY`, `src/metadata.rs`).
- Spec package name: `workday-human-resources-wws-api-specs` (`WORKDAY_API_SPEC_PACKAGE_ID`).
- API family: Workday Web Services (WWS) SOAP — Human_Resources service. `WorkdayProviderDescriptor` pins `service_name = "Human_ResourcesService"`, `service_version = "v46.1"`.
- API base path: `/ccx/service/{tenant}/Human_Resources/v46.1` (`WorkdayTenantBinding::human_resources_wws_path()`); shared-plan validation rejects any other service, version, absolute URL, or extra path segment (`validate_human_resources_wws_path`, `src/operation.rs`).
- Data-source binding name(s): caller-supplied via `WorkdayProviderDescriptor::new(data_source_name)`; no fixed binding is registered. Compiler contracts exercise `workday_hcm`.

## 2. Auth Contract

**Flows** (from `src/auth.rs` and the `src/lib.rs` doc-comment, verbatim — including honest values like `access_gated_unknown`):

| Flow | Status | Notes |
| --- | --- | --- |
| isu_ws_security_username_token | placeholder | ISU (Integration System User) with WS-Security UsernameToken "where approved" — declared only in the `src/lib.rs` doc-comment; deliberately deferred to runtime integration wiring. No auth-flow execution, token handling, or WS-Security header construction exists in this network-free crate. |

**Environment contract** (transcribed from `src/auth.rs` — honest caveat: this crate exports only bare env-var **name constants**; it has no typed env descriptor struct, so `required_for`/`secret` metadata below is derived from constant names and the lib.rs doc-comment, not from typed constants):

| Env var | Constant | Required for | Secret | Description |
| --- | --- | --- | --- | --- |
| `WORKDAY_HOST` | `WORKDAY_HOST_ENV` | tenant binding (all operations) | no | DNS host only — validated, never a URL |
| `WORKDAY_TENANT` | `WORKDAY_TENANT_ENV` | tenant binding (all operations) | no | single tenant path segment — validated |
| `WORKDAY_ISU_USERNAME` | `WORKDAY_ISU_USERNAME_ENV` | ISU flow (runtime wiring; not yet consumed by this crate) | no | Integration System User name |
| `WORKDAY_ISU_SECRET` | `WORKDAY_ISU_SECRET_ENV` | ISU flow (runtime wiring; not yet consumed by this crate) | yes (by naming convention; no typed secret flag exists in code) | ISU credential |

**Redaction:** **absent — evidence-gated.** No redaction constant or `Authorization`-style header exists in this crate; the WS-Security header (where the secret would appear) is deferred to runtime wiring. Blocking artifact: runtime ISU WS-Security wiring with redaction compiler/live evidence.

**Tenant binding validation** (`WorkdayTenantBinding`, `src/auth.rs`):
- Host: non-empty single-line DNS host; rejects whitespace/control characters (CRLF header-injection shapes rejected), `://` URLs, path/query/fragment characters, userinfo (`@`) and port (`:`), dot-less hosts, non-`[A-Za-z0-9-]` labels, and labels with leading/trailing hyphens.
- Tenant: exactly one path segment; rejects whitespace/control characters, slash/backslash, query/fragment/userinfo/port punctuation, and `..` traversal; allows only ASCII letters, digits, underscore, and hyphen.

**Server-bound values callers cannot override:** the HTTPS origin (`https://{host}`) and the WWS path are built server-side from the validated `WorkdayTenantBinding`; `WorkdayProvider::build_named_saas_request_plan` takes the binding as a server-bound argument, and the emitted plan path is re-validated to be exactly `/ccx/service/{tenant}/Human_Resources/v46.1`.

## 3. Operation Catalog

**Status vocabulary (exactly four values — do not invent others):**
- `live_certified` — retained live evidence per docs/runtime/saas-certification.md. **None exists for any SaaS provider today; the column stays anyway.**
- `compiler_contracted` — network-free request-**plan** construction proven by unit contracts. **NOT live-callable** until provider binding, auth, redaction, and live-certification evidence are retained through the runtime SaaS executor; never conflate request planning with executable-against-vendor.
- `planned_gated` — registered, non-executable; gates listed per row.
- `write_gated` — registered write candidate, non-executable pending G1 governed-write evidence.

In crate code the availability enum is `WorkdayOperationAvailability::{Executable, PlannedUnsupported}`; `Executable` maps to `compiler_contracted` here and means proven offline SOAP plan construction only.

| Operation | Kind (read/write) | Status | Gates (verbatim enum members) | PII/PHI flag | Notes |
| --- | --- | --- | --- | --- | --- |
| `workday.list_workers` | read | compiler_contracted | — | pii_heavy=true | `Get_Workers` with optional paired `updated_from`/`updated_through` transaction-log criteria and paired effective-date range |
| `workday.get_worker` | read | compiler_contracted | — | pii_heavy=true | `Get_Workers` by typed `worker_reference` (WID or Employee_ID) |
| `workday.hcm.get_worker_profile` | read | compiler_contracted | — | pii_heavy=true | Compatibility binding through the proven `Get_Workers` by-reference template; per `src/registry.rs`, do **not** swap to a distinct `Get_Worker_Profile` SOAP template until tenant-approved WWS request-shape evidence is available |
| `workday.list_worker_events` | read | compiler_contracted | — | pii_heavy=true | `Get_Worker_Event_History`; requires `worker_reference` plus `from_event_date`/`through_event_date` |
| `workday.list_organizations` | read | compiler_contracted | — | false | `Get_Organizations`, optional `include_inactive` |
| `workday.list_locations` | read | compiler_contracted | — | false | `Get_Locations`, optional `include_inactive` |
| `workday.list_job_profiles` | read | compiler_contracted | — | false | `Get_Job_Profiles`, optional `include_inactive` |
| `workday.get_server_timestamp` | read | compiler_contracted | — | false | `Get_Server_Timestamp`; plan forced to page 1 / count 1 / max_pages 1; freshness anchor |
| `workday.hcm.get_workers_page` | read | compiler_contracted | — | pii_heavy=true | Metadata alias of `workday.list_workers`; builds the identical typed template (proven by alias contracts) |
| `workday.hcm.get_organizations_page` | read | compiler_contracted | — | false | Alias of `workday.list_organizations` |
| `workday.hcm.get_locations_page` | read | compiler_contracted | — | false | Alias of `workday.list_locations` |
| `workday.hcm.get_job_profiles_page` | read | compiler_contracted | — | false | Alias of `workday.list_job_profiles` |
| `workday.list_former_workers` | read | planned_gated | `TenantApprovedWwsRequestShape`, `HrPiiGovernanceReview`, `LiveCertification` | pii_heavy=true | `Get_Former_Workers`; parameters registered but request shape is evidence-gated: awaiting authenticated export (tenant-approved WWS request-shape evidence); plan construction fails closed with "requires tenant-approved Workday WWS request-shape evidence" |
| `workday.get_workday_accounts` | read | planned_gated | `TenantApprovedWwsRequestShape`, `IdentitySecurityGovernanceReview`, `LiveCertification` | pii_heavy=true | `Get_Workday_Account`; evidence-gated: awaiting authenticated export (tenant-approved WWS request-shape evidence); fails closed identically |

`live_certified`: none. `write_gated`: none registered for Workday.

Write posture: `WorkdayProvider::build_named_mutation_plan()` always fails closed with `UnsupportedMutation("Wave 1 Workday skeleton exposes read-only WWS operation planning")` (`src/provider.rs`). The shared G1 reasons, verbatim from `appfw_runtime/src/provider_capabilities.rs:696-711`:

- "SaaS provider writes remain unsupported until named mutation safety, idempotency, audit, and live write evidence exist"
- "SaaS delegated/on-behalf-of actor context remains unsupported until per-user token storage, tenant binding, and impersonation audit evidence exist"
- "SaaS per-user token-store isolation remains unsupported until encrypted storage, rotation, tenant partitioning, and revocation evidence exist"
- "SaaS named mutations remain unsupported until a provider-specific mutation registry and deny-by-default exposure gate exist"
- "SaaS mutation request binding remains unsupported until provider-specific request templates, parameter binding, and redaction evidence exist"
- "SaaS write idempotency and replay protection remain unsupported until idempotency keys, retry classification, and duplicate-suppression evidence exist"
- "SaaS write policy and scope enforcement remain unsupported until operation-level policy, MCP/Kafka/frontend exposure gates, and least-privilege scopes are proven"
- "SaaS write audit remains unsupported until before/after redaction, request correlation, undo/compensation posture, and retained evidence exist"

Framework-wide, the only registered gated non-executable named-mutation candidate today is `servicenow.create_incident` (policy scope `servicenow.incident.write`) in `appfw_provider_servicenow` (current working tree); Workday's governed-write named-mutation allow-list in `scripts/appfw` is empty.

## 4. Data Paths

### 4a. Stream-fed (kappa) coverage map
| SaaS object | Streams into Mongo? | Mongo collection | CDC topic | Classification | Status |
| --- | --- | --- | --- | --- | --- |
| Worker | **UNKNOWN — to be filled by data platform team** | — | — | — | placeholder |
| Former Worker | **UNKNOWN — to be filled by data platform team** | — | — | — | placeholder |
| Worker Event History | **UNKNOWN — to be filled by data platform team** | — | — | — | placeholder |
| Organization | **UNKNOWN — to be filled by data platform team** | — | — | — | placeholder |
| Location | **UNKNOWN — to be filled by data platform team** | — | — | — | placeholder |
| Job Profile | **UNKNOWN — to be filled by data platform team** | — | — | — | placeholder |
| Workday Account | **UNKNOWN — to be filled by data platform team** | — | — | — | placeholder |

Covered objects are read as Mongo projections (no sync worker); CDC topics feed reactive projections via governed Kafka ingress. Provenance markers, echo-loop exclusions, and stream-offset freshness apply per docs/runtime/saas-connectors.md.

### 4b. Pull-fed (sync worker — coverage gaps)
Watermark field(s): transaction-log `Updated_From` + `Updated_Through` (must be supplied as a pair; UTC ISO-8601 `YYYY-MM-DDTHH:MM:SSZ` literals enforced) plus the paired effective-date range `Effective_From`/`Effective_Through` (`WorkdayWorkerTransactionLogCriteria`, `src/operation.rs`). Freshness anchor: `workday.get_server_timestamp`. Sync descriptor names: **absent — evidence-gated** (no Archetype-1 sync descriptors are registered for Workday; framework sync-worker execution is a config/validation shell only and fails closed). Projection store binding: not yet chosen — Mongo default where 4a covers adjacent objects; Postgres fully supported; record the per-schema choice and rationale when the first Workday sync descriptor is created.

### 4c. Live (Archetype-2)
Named live-read operations: none are live-callable — the 12 `compiler_contracted` reads in section 3 are the candidates, blocked on `ConnectionAuth` (unsupported), runtime WS-Security/provider binding, redaction evidence, and live certification. Governed-write candidates: none registered for Workday (cross-reference section 3 — no `write_gated` rows).

## 5. Pagination & Continuation
Shared-kernel style: `PageCountContinuation`. Vendor field names: WWS `Response_Results` → `Total_Results`, `Total_Pages`, `Page_Results`, `Page` (parsed in `src/response.rs`); requests carry `Response_Filter` with `Page`/`Count` and optional `As_Of_Effective_Date`/`As_Of_Entry_DateTime`. Pages after page 1 require a stable `As_Of_Entry_DateTime` (fail-closed otherwise); continuation stops at the response `Total_Pages` or the plan's `max_pages`, and a response `Page` that does not match the requested page is rejected (`src/pagination.rs`).

## 6. Limits, Caps & Rate-Limit Posture
Defaults: timeout 5,000 ms, `max_pages` 10 (`WorkdayQueryLimits`, `src/operation.rs`); `Response_Filter` `Count` defaults to 100 with the WWS hard cap `Count <= 999` (`WorkdayResponseFilter::MAX_COUNT`). Shared SaaS caps derive as `max_rows = Count x max_pages` and `max_body_bytes = shared default x max_pages`; `workday.get_server_timestamp` is forced to a single page/row. Rate-limit signals: not modeled — pending live tenant evidence (matrix `RateLimitBackoff` is `saas_unsupported`). SOAP fault classification instead (`src/fault.rs`): `Authentication`, `Validation`, and `Unknown` faults → `DoNotRetry`; `Processing` faults → `RetryReadWithBackoff` (reads only).

## 7. Capability Matrix Summary

Transcribed from `WORKDAY_SAAS_CAPABILITIES` (`appfw_runtime/src/provider_capabilities.rs:1025-1105`). 12 read + 8 write areas:

| # | SaasReadArea | Status | Reason / evidence |
| --- | --- | --- | --- |
| 1 | ConnectionAuth | saas_unsupported | "requires live tenant evidence before this SaaS contract area can be certified" |
| 2 | NamedOperationRegistry | saas_compiler_contracted | registry stable-name + unknown-operation-rejection contracts |
| 3 | RequestBinding | saas_compiler_contracted | SOAP plan → shared SaaS request-plan conversion contracts |
| 4 | PaginationCursoring | saas_compiler_contracted | page-count continuation + stable as-of contracts |
| 5 | RateLimitBackoff | saas_unsupported | "requires live tenant evidence before this SaaS contract area can be certified" |
| 6 | IncrementalWatermark | saas_compiler_contracted | paired transaction-log criteria contract |
| 7 | FieldRedaction | saas_unsupported | "requires live tenant evidence before this SaaS contract area can be certified" — honest asymmetry: Salesforce holds compiler-contracted redaction evidence; Workday does not |
| 8 | TenantScoping | saas_compiler_contracted | tenant-binding rejection + server-bound plan contracts |
| 9 | SchemaVersionPinning | saas_compiler_contracted | v46.1-embedding plan contracts |
| 10 | ResultAndTimeoutCaps | saas_compiler_contracted | caps conversion contracts |
| 11 | QueryMetricsAndAudit | saas_unsupported | "requires live tenant evidence before this SaaS contract area can be certified" |
| 12 | FreshnessReporting | saas_unsupported_with_evidence | live evidence required; retained compiler evidence: server-timestamp parsing + single-row metadata request contracts |
| 13 | GovernedWriteEnforcement | saas_unsupported_with_evidence | shared G1 reason; retained compiler evidence: `provider_rejects_mutation_planning_for_now` |
| 14 | DelegatedActorContext | saas_unsupported | shared G1 reason (section 3) |
| 15 | TokenStoreIsolation | saas_unsupported | shared G1 reason (section 3) |
| 16 | NamedMutationRegistry | saas_unsupported | shared G1 reason (section 3) |
| 17 | MutationRequestBinding | saas_unsupported | shared G1 reason (section 3) |
| 18 | IdempotencyAndReplayProtection | saas_unsupported | shared G1 reason (section 3) |
| 19 | WritePolicyAndScopeEnforcement | saas_unsupported | shared G1 reason (section 3) |
| 20 | WriteAuditAndEvidence | saas_unsupported | shared G1 reason (section 3) |

Summary: 7 of 12 read areas compiler-contracted; 0 areas live-certified; all 8 write areas unsupported.

## 8. Testing Tiers Status
| Tier | Proof command | Status |
| --- | --- | --- |
| Local | `cargo test -p appfw-provider-workday`; fixtures under `fixtures/operations/` (provenance: synthetic \| vendor_export \| recorded_live) | Compiler contracts exist across auth/registry/operation/pagination/response/fault/provider modules. Fixture directory: **absent — evidence-gated** (no `fixtures/` exists in this crate; blocking artifact: vendor_export or recorded_live response fixtures) |
| Pipeline | `scripts/appfw framework provider-test --provider workday --area saas-read --plan --json` (+ `--area governed-write --plan` if writes registered) | `--area saas-read --plan` exists and retains `target/appfw/saas-read-provider-test.json` with the 12 read-area checks, current compiler-contracted/unsupported rows, and `release_ready:false` until live smoke evidence exists. Per-area saas_read graduation is still reported by `scripts/appfw framework provider-graduation --json`; the governed-write plan lane accepts workday and reports an empty named-mutation allow-list. |
| Dev | `cargo test -p appfw-provider-workday --test live_smoke -- --ignored` → evidence file → provider-test live → provider-graduation | **absent — evidence-gated**: no `live_smoke` test target exists in this crate; blocked on ISU credential provisioning, runtime WS-Security wiring, and a production SaaS request executor |

## 9. Graduation Status & Outstanding Evidence Gates
Current graduation status: **CompilerContracted** — guardrail-complete, live-pending (docs/runtime/saas-certification.md status vocabulary: `CompilerContracted` / `Implemented` / `Partial` / `Unsupported` / `LiveCertified`; no Workday area may be marked `LiveCertified` without retained live evidence).

Outstanding gates checklist:
- [ ] Tenant-approved WWS request-shape evidence (`WorkdayOperationGate::TenantApprovedWwsRequestShape`; owner: unassigned) — unlocks `workday.list_former_workers`, `workday.get_workday_accounts`, and a distinct `Get_Worker_Profile` template for `workday.hcm.get_worker_profile`.
- [ ] ISU credential provisioning plus WS-Security UsernameToken runtime wiring — ConnectionAuth live evidence (owner: unassigned).
- [ ] Live tenant evidence for RateLimitBackoff, FieldRedaction, QueryMetricsAndAudit, and FreshnessReporting (owner: unassigned).
- [ ] HrPiiGovernanceReview (former workers) and IdentitySecurityGovernanceReview (Workday accounts).
- [ ] LiveCertification — the live certification runner is absent framework-wide.
- [ ] Runtime executor provider binding — HTTP executor substrate exists, but Workday live execution remains blocked until WS-Security/auth binding, redaction, and live evidence are retained.
- [ ] Tenant-authenticated Workday WWS request-shape evidence and dated XSD/doc snapshot review; the crate now exposes `WorkdayApiSnapshotMetadata`, but request-shape/live evidence remains absent — evidence-gated.

## 10. PHI / Data Classification Notes
All worker-level operations are `pii_heavy=true` in the registry: `workday.list_workers`, `workday.get_worker`, `workday.hcm.get_worker_profile`, `workday.hcm.get_workers_page`, `workday.list_worker_events`, `workday.list_former_workers`, `workday.get_workday_accounts`. Reference-data reads (organizations, locations, job profiles, server timestamp) are not PII-flagged. Governance-review gates: `HrPiiGovernanceReview` guards former-worker data; `IdentitySecurityGovernanceReview` guards Workday account/identity data. Because Workday `FieldRedaction` is `saas_unsupported` (unlike Salesforce), worker-level fields must not be projected into shared stores or logs until redaction evidence exists. Retention and deletion/tombstone expectations for any projection of Workday data follow the object-map contract in docs/runtime/saas-connectors.md (classification propagation, retention policy, tombstone deletion). The kappa coverage rows in 4a must be classified by the data platform team when filled; worker-level rows are presumptively PII-heavy.
