# Anaplan Connector Contract

<!-- appfw:vendor-contract-template v1
     Location: appfw_provider_anaplan/docs/vendor-contract.md
     Gated by: scripts/appfw framework docs-check --json
       (maintainability-contract required_docs/required_tokens; saas-vendor-doc-parity
        asserts every pin in the Version Link block equals the crate's exported constant).
     Rule: values here are TRANSCRIBED from crate constants, never hand-invented.
     If evidence does not exist, write "absent — evidence-gated" and name the blocking artifact. -->

## Version Link

| Field | Value | Source of truth (must match) |
| --- | --- | --- |
| Vendor platform release | tenant-scoped workspace/model export | `ANAPLAN_RELEASE_PIN` (`src/metadata.rs`) |
| Vendor API version pin | Integration API v2, base `/2/0` | `ANAPLAN_API_VERSION_PIN`, `ANAPLAN_API_BASE_PATH` (`src/metadata.rs`, `src/operation.rs`) |
| API doc / OpenAPI snapshot version | Integration API v2 summary path pinned as `vendor_api_specs_2026-06-26/enrichment/extracted/anaplan-integration-api-v2-summary.json`; tenant-approved workspace/model export remains absent — evidence-gated | `ANAPLAN_INTEGRATION_API_SUMMARY_PATH`; `anaplan_api_snapshot_metadata().blocking_gap` (`src/metadata.rs`) |
| Evidence artifact + date | `vendor_api_specs_2026-06-26/enrichment/information-package-versions.json`, `vendor_api_specs_2026-06-26/enrichment/saas-provider-implementation-metadata.json`, `vendor_api_specs_2026-06-26/enrichment/extracted/anaplan-integration-api-v2-summary.json` (2026-06-26); tenant-approved Anaplan workspace/model binding plus export/action allow-list **absent — evidence-gated** | `metadata.rs` `source_refs` / `blocking_gap` |
| Information package version | 2026.06.26.1 | `ANAPLAN_INFORMATION_PACKAGE_VERSION` (`src/metadata.rs`) |
| Crate version | appfw-provider-anaplan 0.1.0 | `Cargo.toml` |
| Doc last reconciled | 2026-07-02 | this file |

## 1. Identity & Family

- `provider_key`: `anaplan` (`ANAPLAN_PROVIDER_KEY`, `src/identity.rs:1`).
- `vendor_key`: `anaplan-integration-api` (`ANAPLAN_VENDOR_KEY`, `src/metadata.rs`).
- API family: `Integration API v2` (REST/JSON; `SaasTransportProtocol::RestJson`).
- API base path: `/2/0`; all executable paths are server-composed under `/2/0/workspaces/{workspace_id}/models/{model_id}` (`AnaplanTenantBinding::model_path_prefix`, `src/auth.rs:123-129`). Raw absolute URLs are rejected by plan validation.
- Spec package name: `anaplan-integration-api-v2-specs` (`ANAPLAN_API_SPEC_PACKAGE_ID`, `src/metadata.rs`).
- Data-source binding name(s): caller-supplied `data_source_name` on `AnaplanProviderDescriptor::new` / `AnaplanProvider::new`; contract tests use `anaplan_primary` as the canonical example. No production binding is registered yet.

## 2. Auth Contract

**Flows** (from `src/auth.rs`, verbatim — including honest values like `access_gated_unknown`):

| Flow | Status | Notes |
| --- | --- | --- |
| certificate | preferred | `AnaplanAuthConfigMetadata::integration_user_auth()` sets `preferred_flow = Certificate`; integration-user model (`integration_user: true`). Planning-only — no token lifecycle is executed by this crate. |
| basic_user_password | exception | "Tenant-approved Anaplan integration user name for exception flows" (env-var description, verbatim). |

**Environment contract** (transcribed from `ANAPLAN_INTEGRATION_USER_AUTH_ENV_VARS`, `src/auth.rs:137-174`):

| Name | Required for | Secret | Description (verbatim) |
| --- | --- | --- | --- |
| `ANAPLAN_API_BASE_URL` | all flows | no | Bound Anaplan Integration API origin used for metadata and export requests. |
| `ANAPLAN_AUTH_BASE_URL` | all flows | no | Bound Anaplan Authentication API origin used for token lifecycle requests. |
| `ANAPLAN_USERNAME` | basic_user_password only | no | Tenant-approved Anaplan integration user name for exception flows. |
| `ANAPLAN_PASSWORD_OR_CERTIFICATE` | all flows | yes | Certificate material/reference for preferred auth or password for approved exception flows. |
| `ANAPLAN_WORKSPACE_ID` | all flows | no | Server-bound Anaplan workspace id; callers cannot override this value. |
| `ANAPLAN_MODEL_ID` | all flows | no | Server-bound Anaplan model id; callers cannot override this value. |

**Redaction:** `Authorization: AnaplanAuthToken <redacted>` (`ANAPLAN_AUTHORIZATION_HEADER`, `ANAPLAN_REDACTED_AUTH_TOKEN`).
**Tenant binding validation:** both origins must be single-line HTTPS origins without whitespace/control characters, without path, query, or fragment, with a DNS host and no userinfo or port; workspace/model ids must be non-empty and contain only ASCII letters, digits, hyphen, or underscore (`validate_anaplan_https_origin`, `validate_anaplan_identifier`).
**Server-bound values callers cannot override:** workspace id, model id, and the derived path prefixes `/2/0/workspaces/{workspace_id}` and `/2/0/workspaces/{workspace_id}/models/{model_id}`; the auth origin is a separate token-lifecycle origin from the API origin.

## 3. Operation Catalog

**Status vocabulary (exactly four values — do not invent others):**
- `live_certified` — retained live evidence per docs/runtime/saas-certification.md. **None exists for any SaaS provider today; the column stays anyway.**
- `compiler_contracted` — network-free request-**plan** construction proven by unit contracts. **NOT live-callable** until provider binding, auth, redaction, and live-certification evidence are retained through the runtime SaaS executor; never conflate request planning with executable-against-vendor.
- `planned_gated` — registered, non-executable; gates listed per row.
- `write_gated` — registered write candidate, non-executable pending G1 governed-write evidence.

| Operation | Kind (read/write) | Status | Gates (verbatim enum members) | PII/PHI flag | Notes |
| --- | --- | --- | --- | --- | --- |
| `anaplan.model.get_status` | read (GET `{model}/status`) | compiler_contracted | — (no gates on row; live use still blocked by absent provider binding/live certification) | none — sensitivity `Operational` | Fixed server-bound model status read; caps max_rows=1; rejects caller parameters. |
| `anaplan.files.list` | read (GET `{model}/files`) | compiler_contracted | — (same live-callability caveat) | none — sensitivity `BusinessConfidential` | Fixed `sort=name` + `offset`/`limit` pagination; caps bound to effective limit. |
| `anaplan.view.create_read_request` | read-request creation (POST `{model}/views/{view}/readRequests/`) | planned_gated | `TenantExportAllowList`, `LiveCertification` | none — sensitivity `BusinessConfidential` | "View read-request creation requires a tenant-approved registry-bound view id." |
| `anaplan.view.get_read_request` | read (GET readRequest status) | planned_gated | `TenantExportAllowList`, `LiveCertification` | none — sensitivity `BusinessConfidential` | Requires typed `read_request` continuation parameter. |
| `anaplan.view.get_read_page` | read (GET readRequest page) | planned_gated | `TenantExportAllowList`, `LiveCertification` | none — sensitivity `BusinessConfidential` | Requires typed `read_request` continuation; page size from continuation or limits. |
| `anaplan.file.download_chunk` | read (GET `{model}/files/{file}/chunks/{n}`) | planned_gated | `TenantExportAllowList`, `LiveCertification` | none — sensitivity `BusinessConfidential` | Requires typed `chunk` continuation; chunk continuation must belong to the registry-bound file id. |

Full gate enum (`AnaplanOperationGate`, `src/operation.rs:149-168`, reasons verbatim): `TenantWorkspaceModelScope` — "requires tenant-approved workspace and model binding"; `TenantExportAllowList` — "requires tenant-approved export/action allow-list before execution"; `LiveCertification` — "requires live connector certification evidence"; `RuntimeExecutor` — "requires runtime HTTP executor integration".

**Distinctive graduation path:** the registry-bound constructors `view_create_read_request_for_view`, `view_get_read_request_for_view`, `view_get_read_page_for_view`, and `file_download_chunk_for_file` (`src/registry.rs:137-215`) mint per-view/per-file operations with `availability: Executable` once tenant-approved ids are compiled into the registry — the export allow-list graduation is code-level, not config-level.

**Write posture:** no write operations are registered for Anaplan; `AnaplanProvider::build_named_mutation_plan()` always errors `UnsupportedMutation("Wave 1 Anaplan skeleton exposes read operation planning only")`. The only registered named-mutation candidate in the repository today is ServiceNow's gated non-executable `servicenow.create_incident` (working tree; policy scope `servicenow.incident.write`) — Anaplan has none. Shared G1 reasons, verbatim from `appfw_runtime/src/provider_capabilities.rs:696-711`:
- "SaaS provider writes remain unsupported until named mutation safety, idempotency, audit, and live write evidence exist"
- "SaaS delegated/on-behalf-of actor context remains unsupported until per-user token storage, tenant binding, and impersonation audit evidence exist"
- "SaaS per-user token-store isolation remains unsupported until encrypted storage, rotation, tenant partitioning, and revocation evidence exist"
- "SaaS named mutations remain unsupported until a provider-specific mutation registry and deny-by-default exposure gate exist"
- "SaaS mutation request binding remains unsupported until provider-specific request templates, parameter binding, and redaction evidence exist"
- "SaaS write idempotency and replay protection remain unsupported until idempotency keys, retry classification, and duplicate-suppression evidence exist"
- "SaaS write policy and scope enforcement remain unsupported until operation-level policy, MCP/Kafka/frontend exposure gates, and least-privilege scopes are proven"
- "SaaS write audit remains unsupported until before/after redaction, request correlation, undo/compensation posture, and retained evidence exist"

## 4. Data Paths

### 4a. Stream-fed (kappa) coverage map
| SaaS object | Streams into Mongo? | Mongo collection | CDC topic | Classification | Status |
| --- | --- | --- | --- | --- | --- |
| Anaplan model status | **UNKNOWN — to be filled by data platform team** | — | — | — | placeholder |
| Anaplan model files | **UNKNOWN — to be filled by data platform team** | — | — | — | placeholder |
| Anaplan view read pages (per tenant-approved view) | **UNKNOWN — to be filled by data platform team** | — | — | — | placeholder |

Covered objects are read as Mongo projections (no sync worker); CDC topics feed reactive projections via governed Kafka ingress. Provenance markers, echo-loop exclusions, and stream-offset freshness apply per docs/runtime/saas-connectors.md.

### 4b. Pull-fed (sync worker — coverage gaps)
Watermark field(s): **not modeled — evidence-gated.** `IncrementalWatermark` is `saas_unsupported` for Anaplan with the verbatim reason "requires tenant-approved Anaplan auth, workspace/model binding, and selected metadata/export operations before named-operation contracts can be implemented or certified" (`ANAPLAN_TENANT_OPERATION_SCOPE_REQUIRED_REASON`). No sync descriptors are registered for Anaplan, and Archetype-1 sync-worker execution is absent framework-wide (config/validation shell only, fail-closed). Projection store binding: not yet chosen for Anaplan — Mongo is the default where 4a covers adjacent objects; Postgres remains fully supported; record the per-schema choice and rationale here when the first Anaplan projection is designed.

### 4c. Live (Archetype-2)
Named live-read operations: none are live today. The future live-read surface is the two compiler-contracted operations plus the registry-bound per-view read-request chain and per-file chunk downloads (section 3), all pending `TenantExportAllowList`, provider runtime binding, and `LiveCertification`. Governed-write candidates: **none registered for Anaplan** (no section 3 `write_gated` rows to cross-reference).

## 5. Pagination & Continuation
Shared-kernel `OffsetLimitContinuation` built from Anaplan `meta/paging` JSON (`currentPageSize`, `offset`, optional `totalSize`) via `AnaplanPagingSummary::next_offset_continuation` (`src/response.rs`); continuation is omitted at the known end of the collection. `anaplan.files.list` plans fixed `sort=name` with `offset`/`limit` query parameters. Read-request pages use the typed shared-kernel `ReadRequestPageContinuation` (`request_id`, `page`, optional `page_size`); file chunks use the typed `ChunkDownloadContinuation` (`chunk_set_id`, `chunk_id`) which must match the registry-bound file id. Collection summaries retain item counts and paging only — raw item payloads are not retained.

## 6. Limits, Caps & Rate-Limit Posture
Defaults: 100 rows / 2,000 ms timeout (`AnaplanQueryLimits::DEFAULT_MAX_RESULTS`, `DEFAULT_TIMEOUT_MS`); MAX page limit 1,000 (`MAX_PAGE_LIMIT`), enforced by clamping `limit` and response caps; single-object reads (model status, read-request creation/status, chunk) cap `max_rows` at 1; body cap `SaasResponseCaps::DEFAULT_MAX_BODY_BYTES`. Vendor hard caps beyond the 1,000-row page limit: evidence-gated: awaiting authenticated export. Rate-limit signals: **not modeled — pending live tenant evidence** (`RateLimitBackoff` is `saas_unsupported` with "requires live tenant evidence before this SaaS contract area can be certified"). SOAP fault classification: not applicable (REST/JSON family).

## 7. Capability Matrix Summary
Transcribed from `ANAPLAN_SAAS_CAPABILITIES` (`appfw_runtime/src/provider_capabilities.rs:1108-1187`). 20 SaasReadArea rows (12 read + 8 write): 6 read areas `saas_compiler_contracted`, 6 read areas `saas_unsupported`, 1 write area `saas_unsupported_with_evidence`, 7 write areas `saas_unsupported`. None live-certified.

| Area | Status | Reason / evidence |
| --- | --- | --- |
| ConnectionAuth | saas_unsupported | requires live tenant evidence before this SaaS contract area can be certified |
| NamedOperationRegistry | saas_compiler_contracted | `registry_exposes_stable_operation_names`, `unknown_operation_is_rejected` |
| RequestBinding | saas_compiler_contracted | model-status/files-list/view-read-request/read-page/file-chunk plan contracts |
| PaginationCursoring | saas_compiler_contracted | files-list offset/limit, read-page continuation, `paging_summary_builds_offset_continuation` |
| RateLimitBackoff | saas_unsupported | requires live tenant evidence (contrast: Salesforce has compiler-contracted rate-limit signal evidence — asymmetry stated honestly) |
| IncrementalWatermark | saas_unsupported | requires tenant-approved Anaplan auth, workspace/model binding, and selected metadata/export operations |
| FieldRedaction | saas_unsupported | requires live tenant evidence (contrast: Salesforce FieldRedaction is compiler-contracted) |
| TenantScoping | saas_compiler_contracted | tenant-binding rejection tests, server-bound plan test |
| SchemaVersionPinning | saas_compiler_contracted | descriptor key/API pin test, `/2/0` base-path test |
| ResultAndTimeoutCaps | saas_compiler_contracted | files-list caps tests, page-limit clamp test, chunk caps test |
| QueryMetricsAndAudit | saas_unsupported | requires live tenant evidence |
| FreshnessReporting | saas_unsupported | requires live tenant evidence |
| GovernedWriteEnforcement | saas_unsupported_with_evidence | shared G1 reason; evidence: `provider_rejects_mutation_planning_for_now` |
| DelegatedActorContext | saas_unsupported | shared G1 reason (section 3) |
| TokenStoreIsolation | saas_unsupported | shared G1 reason (section 3) |
| NamedMutationRegistry | saas_unsupported | shared G1 reason (section 3) |
| MutationRequestBinding | saas_unsupported | shared G1 reason (section 3) |
| IdempotencyAndReplayProtection | saas_unsupported | shared G1 reason (section 3) |
| WritePolicyAndScopeEnforcement | saas_unsupported | shared G1 reason (section 3) |
| WriteAuditAndEvidence | saas_unsupported | shared G1 reason (section 3) |

## 8. Testing Tiers Status
| Tier | Proof command | Status |
| --- | --- | --- |
| Local | `cargo test -p appfw-provider-anaplan`; fixtures under `fixtures/operations/` (provenance: synthetic \| vendor_export \| recorded_live) | Unit contracts pass (in-`src` tests across auth/identity/operation/registry/provider/response). `fixtures/operations/` directory is **absent — evidence-gated** (no fixture corpus exists for this crate). |
| Pipeline | `scripts/appfw framework provider-test --provider anaplan --area saas-read --plan --json` (+ `--area governed-write --plan` if writes registered) | `--area saas-read --plan` exists and retains `target/appfw/saas-read-provider-test.json` with the 12 read-area checks, current compiler-contracted/unsupported rows, and `release_ready:false` until live smoke evidence exists. `--area governed-write --plan` accepts `anaplan` and reports an empty named-mutation candidate list (no writes registered). |
| Dev | `cargo test -p appfw-provider-anaplan --test live_smoke -- --ignored` → evidence file → provider-test live → provider-graduation | **absent — evidence-gated**: no `tests/live_smoke` target exists in the crate; no live evidence artifact; blocked on tenant-approved auth + export allow-list + runtime executor. |

## 9. Graduation Status & Outstanding Evidence Gates
Current ladder position: **CompilerContracted** (per docs/runtime/saas-certification.md ladder CompilerContracted → Implemented → Partial → LiveCertified). Guardrail-complete, live-pending: registry enforcement, request binding, tenant scoping, pagination, and caps are proven network-free; nothing is live-callable. Remaining gates checklist:
- [ ] Tenant-approved Anaplan workspace/model binding (owner: unassigned — evidence-gated: awaiting tenant approval; gate `TenantWorkspaceModelScope`).
- [ ] Tenant-approved export/action allow-list — compile approved view/file ids into the registry via the `*_for_view` / `*_for_file` constructors (gate `TenantExportAllowList`).
- [ ] Tenant-approved Anaplan workspace/model binding and dated export/action allow-list review; the crate now exposes `AnaplanApiSnapshotMetadata`, but tenant export evidence remains absent — evidence-gated.
- [ ] Runtime executor provider binding (gate `RuntimeExecutor`): HTTP executor substrate exists, but Anaplan live execution remains blocked until tenant binding/auth/live evidence is retained.
- [ ] Live connector certification evidence via provider-test live → `scripts/appfw framework provider-graduation --json` (gate `LiveCertification`); ConnectionAuth, RateLimitBackoff, FieldRedaction, QueryMetricsAndAudit, FreshnessReporting all await live tenant evidence.

## 10. PHI / Data Classification Notes
The crate classifies data with `AnaplanDataSensitivity`: `Operational` (model status) and `BusinessConfidential` (files, view read requests/pages, file chunks). No PII/PHI flags are declared, and the Anaplan gate enum carries no governance-review member (unlike Salesforce `GovernanceReview` or Workday `HrPiiGovernanceReview`/`IdentitySecurityGovernanceReview`) — planning-model data is treated as business-confidential financial/planning content, not personal data. Caveat: whether tenant-approved views expose workforce-planning rows that are effectively PII is **evidence-gated: awaiting authenticated export** and must be re-assessed per approved view at allow-list time. Response summaries deliberately retain only counts and paging, never raw item payloads. Retention and tombstone expectations for any Anaplan projection follow docs/runtime/saas-connectors.md; classification of the kappa coverage rows in 4a is pending the data platform team's fill.
