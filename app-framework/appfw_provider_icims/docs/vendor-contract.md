# iCIMS Connector Contract

<!-- appfw:vendor-contract-template v1
     Location: appfw_provider_icims/docs/vendor-contract.md
     Gated by: scripts/appfw framework docs-check --json
       (maintainability-contract required_docs/required_tokens; saas-vendor-doc-parity
        asserts every pin in the Version Link block equals the crate's exported constant).
     Rule: values here are TRANSCRIBED from crate constants, never hand-invented.
     If evidence does not exist, write "absent — evidence-gated" and name the blocking artifact. -->

## Version Link

| Field | Value | Source of truth (must match) |
| --- | --- | --- |
| Vendor platform release | unpinned — evidence-gated: "public Developer Community pages; authenticated API docs required" | `ICIMS_RELEASE_BASIS` (`src/identity.rs`) |
| Vendor API version pin | unpinned — evidence-gated: awaiting authenticated export (no operation constant pins an API version or base path) | no crate operation constant exists yet |
| API doc / OpenAPI snapshot version | n/a — public-docs summary only (`icims-public-api-docs`) | `ICIMS_API_SPEC_PACKAGE_ID` (`src/metadata.rs`) |
| Evidence artifact + date | Public corpus (2026-06-26): `vendor_api_specs_2026-06-26/enrichment/information-package-versions.json`, `vendor_api_specs_2026-06-26/enrichment/saas-provider-implementation-metadata.json`, `vendor_api_specs_2026-06-26/enrichment/extracted/icims-public-docs-summary.json`, https://developer-community.icims.com/getting-started/integrating-icims. Authenticated evidence **absent — evidence-gated** ("Need authenticated iCIMS Developer Community API docs plus sandbox or tenant export evidence.") | `metadata.rs` `source_refs` / `blocking_gap` |
| Information package version | 2026.06.26.1 | `ICIMS_INFORMATION_PACKAGE_VERSION` |
| Crate version | appfw-provider-icims 0.1.0 | `Cargo.toml` |
| Doc last reconciled | 2026-07-02 | this file |

## 1. Identity & Family

- `provider_key`: `icims` (`ICIMS_PROVIDER_KEY`, `src/identity.rs`)
- `vendor_key`: `icims` (`ICIMS_VENDOR_KEY`, `src/metadata.rs`)
- API family: "iCIMS Talent Cloud / Applicant Tracking APIs" (`ICIMS_API_FAMILY`). REST posture is confirmed only by the public 2026-06-26 corpus; request/response shapes are NOT verified.
- API base path: not modeled — evidence-gated: awaiting authenticated export. The crate builds no request paths.
- Spec package name: `icims-public-api-docs` (`ICIMS_API_SPEC_PACKAGE_ID`).
- Data-source binding name(s): instance-supplied `data_source_name` through `IcimsProviderIdentity::external_api_data_source_name()` (e.g. `icims_primary` in crate tests); no fixed binding constant is exported.

## 2. Auth Contract

**Flows** (from `src/auth.rs`, verbatim — including honest values like `access_gated_unknown`):

| Flow | Status | Notes |
| --- | --- | --- |
| access_gated_unknown | access_gated_unknown | Only flow declared (`ICIMS_AUTH_FLOWS`, preferred flow). The credential TYPE itself is unknown until authenticated Developer Community or tenant docs are captured; no auth request construction exists. |

**Environment contract** (transcribed from `ICIMS_AUTH_ENV_VARS` in `src/auth.rs`):

| Env var | Required for | Secret | Description (verbatim) |
| --- | --- | --- | --- |
| `ICIMS_BASE_URL` | all flows | no | "Tenant-specific iCIMS HTTPS origin." |
| `ICIMS_AUTH_MODE` | all flows | no | "Access-gated auth mode from authenticated Developer Community or tenant docs." |
| `ICIMS_CREDENTIAL_REFERENCE` | access_gated_unknown | **yes** | "Opaque credential reference only after authenticated docs confirm the credential type." |
| `ICIMS_CUSTOMER_ID` | all flows | no | "Server-bound iCIMS customer or sandbox identifier." |
| `ICIMS_INTEGRATION_NAME` | all flows | no | "Server-bound marketplace/integration name." |
| `ICIMS_MARKETPLACE_VALIDATION` | all flows (optional) | no | "Optional marketplace validation or revalidation evidence reference." |

**Redaction:** absent — evidence-gated: awaiting authenticated export. No auth header or redaction constant exists in the crate because the credential type is unknown; `ICIMS_CREDENTIAL_REFERENCE` is flagged `secret: true` so it is never logged.
**Tenant binding validation** (`IcimsTenantBinding`, `src/auth.rs`): base URL must be a single-line HTTPS origin without whitespace/control characters, path, query, or fragment; host must be a DNS name without userinfo or port and must contain a dot. Customer id must be single-line, non-empty, ASCII alphanumeric plus `-`/`_` (path traversal rejected). Integration name and optional marketplace-validation reference must be single-line, non-empty, CR/LF rejected.
**Server-bound values callers cannot override:** `customer_id`, `integration_name`, and the marketplace-validation reference are private fields of `IcimsTenantBinding` with read-only accessors.

## 3. Operation Catalog

**Status vocabulary (exactly four values — do not invent others):**
- `live_certified` — retained live evidence per docs/runtime/saas-certification.md. **None exists for any SaaS provider today; the column stays anyway.**
- `compiler_contracted` — network-free request-**plan** construction proven by unit contracts. **NOT live-callable** until provider binding, auth, redaction, and live-certification evidence are retained through the runtime SaaS executor; never conflate request planning with executable-against-vendor.
- `planned_gated` — registered, non-executable; gates listed per row.
- `write_gated` — registered write candidate, non-executable pending G1 governed-write evidence.

| Operation | Kind (read/write) | Status | Gates (verbatim enum members) | PII/PHI flag | Notes |
| --- | --- | --- | --- | --- | --- |
| `icims.discovery.fetch_api_contract` | read | planned_gated | AuthenticatedDeveloperDocs, SandboxOrTenantExport, StandardFieldMatrix, LiveCertification | not modeled in crate (see §10) | "Authenticated Developer Community contract capture before provider execution." |
| `icims.recruiting.query_candidates_incremental` | read | planned_gated | AuthenticatedDeveloperDocs, SandboxOrTenantExport, StandardFieldMatrix, LiveCertification | not modeled in crate (see §10) | "Planned candidate/profile incremental read after schema, auth, and pagination evidence exists." |
| `icims.recruiting.query_jobs_incremental` | read | planned_gated | AuthenticatedDeveloperDocs, SandboxOrTenantExport, StandardFieldMatrix, LiveCertification | not modeled in crate (see §10) | "Planned job/requisition incremental read after standard field evidence exists." |
| `icims.recruiting.query_applications_incremental` | read | planned_gated | AuthenticatedDeveloperDocs, SandboxOrTenantExport, StandardFieldMatrix, LiveCertification | not modeled in crate (see §10) | "Planned application/workflow-status incremental read after schema and pagination evidence exists." |

No `compiler_contracted`, `live_certified`, or `write_gated` rows exist. The registry exposes only `ensure_operation_is_not_executable`; every registered read fails closed with the first gate reason ("requires authenticated iCIMS Developer Community API documentation"). `MarketplaceValidation` is a declared gate enum member ("requires marketplace validation or revalidation evidence when applicable") but is currently attached to no registered operation.

Write posture: **no named mutations are registered.** `IcimsProvider::build_named_mutation_plan()` always fails with `UnsupportedMutation("iCIMS skeleton requires authenticated Developer Community or tenant docs before reads or writes execute")`. Any future write candidate must clear the shared G1 gates, cited verbatim from `appfw_runtime/src/provider_capabilities.rs:696-711`:
- "SaaS provider writes remain unsupported until named mutation safety, idempotency, audit, and live write evidence exist"
- "SaaS delegated/on-behalf-of actor context remains unsupported until per-user token storage, tenant binding, and impersonation audit evidence exist"
- "SaaS per-user token-store isolation remains unsupported until encrypted storage, rotation, tenant partitioning, and revocation evidence exist"
- "SaaS named mutations remain unsupported until a provider-specific mutation registry and deny-by-default exposure gate exist"
- "SaaS mutation request binding remains unsupported until provider-specific request templates, parameter binding, and redaction evidence exist"
- "SaaS write idempotency and replay protection remain unsupported until idempotency keys, retry classification, and duplicate-suppression evidence exist"
- "SaaS write policy and scope enforcement remain unsupported until operation-level policy, MCP/Kafka/frontend exposure gates, and least-privilege scopes are proven"
- "SaaS write audit remains unsupported until before/after redaction, request correlation, undo/compensation posture, and retained evidence exist"

(Note: the iCIMS capability-matrix rows themselves currently carry the provider-specific reason "requires authenticated iCIMS developer documentation before named-operation contracts can be implemented or certified" for all 20 areas, including the 8 write areas — see §7.)

## 4. Data Paths

### 4a. Stream-fed (kappa) coverage map
| SaaS object | Streams into Mongo? | Mongo collection | CDC topic | Classification | Status |
| --- | --- | --- | --- | --- | --- |
| Candidate / person profile | **UNKNOWN — to be filled by data platform team** | — | — | — | placeholder |
| Job / requisition | **UNKNOWN — to be filled by data platform team** | — | — | — | placeholder |
| Application / workflow status | **UNKNOWN — to be filled by data platform team** | — | — | — | placeholder |

Covered objects are read as Mongo projections (no sync worker); CDC topics feed reactive projections via governed Kafka ingress. Provenance markers, echo-loop exclusions, and stream-offset freshness apply per docs/runtime/saas-connectors.md.

### 4b. Pull-fed (sync worker — coverage gaps)
Watermark field(s): evidence-gated: awaiting authenticated export. The three `*_incremental` reads are named for watermark-based sync, but no watermark field constant exists in the crate (the vendor's updated-since predicate shape is unknown). No sync descriptor names exist. Projection store binding: not yet chosen — Mongo is the default where 4a covers adjacent objects; Postgres is fully supported; record the per-schema choice and rationale when the first iCIMS projection is registered.

### 4c. Live (Archetype-2)
None. No named live-read operations and no governed-write candidates exist (section 3 has zero `write_gated` rows).

## 5. Pagination & Continuation
Not modeled — evidence-gated: awaiting authenticated export. The crate references no shared continuation shape (`UrlCursorContinuation` / `PageCountContinuation` / `OffsetLimitContinuation`) and no vendor paging field names; the operation notes explicitly defer pagination to sandbox/tenant evidence.

## 6. Limits, Caps & Rate-Limit Posture
Not modeled — evidence-gated: awaiting authenticated export. No default row/timeout caps, no MAX page limit, no vendor hard caps, and no rate-limit signal or retry classification exist in the crate.

## 7. Capability Matrix Summary
All **20** `SaasReadArea` entries (12 read + 8 write) are `saas_unsupported` via `planned_saas_capabilities(ICIMS_AUTHENTICATED_DOCS_REQUIRED_REASON)` (`appfw_runtime/src/provider_capabilities.rs:1106-1107`), each carrying the verbatim reason: "requires authenticated iCIMS developer documentation before named-operation contracts can be implemented or certified" (lines 686-687).

| Area group | Areas | Status |
| --- | --- | --- |
| Read (12) | ConnectionAuth, NamedOperationRegistry, RequestBinding, PaginationCursoring, RateLimitBackoff, IncrementalWatermark, FieldRedaction, TenantScoping, SchemaVersionPinning, ResultAndTimeoutCaps, QueryMetricsAndAudit, FreshnessReporting | saas_unsupported (all) |
| Write (8) | GovernedWriteEnforcement, DelegatedActorContext, TokenStoreIsolation, NamedMutationRegistry, MutationRequestBinding, IdempotencyAndReplayProtection, WritePolicyAndScopeEnforcement, WriteAuditAndEvidence | saas_unsupported (all) |

Honest asymmetry: unlike Salesforce (8 compiler-contracted read areas) and Workday (7), iCIMS has **zero** compiler-contracted areas — even NamedOperationRegistry stays unsupported in the matrix despite the crate's fail-closed registry tests, because no request planning exists to contract. No area on any SaaS provider is live-certified today.

## 8. Testing Tiers Status
| Tier | Proof command | Status |
| --- | --- | --- |
| Local | `cargo test -p appfw-provider-icims`; fixtures under `fixtures/operations/` (provenance: synthetic \| vendor_export \| recorded_live) | Unit contracts prove the fail-closed posture (registry non-executability, mutation rejection, tenant-binding validation, metadata gap assertions). No `fixtures/operations/` directory exists — absent — evidence-gated: nothing to fixture until the authenticated export. |
| Pipeline | `scripts/appfw framework provider-test --provider icims --area saas-read --plan --json` (+ `--area governed-write --plan` if writes registered) | `--area saas-read --plan` exists and retains `target/appfw/saas-read-provider-test.json` with the 12 read-area checks, current unsupported rows, and `release_ready:false` until live smoke evidence exists. The `--area governed-write --plan` lane accepts `icims` and reports an empty named-mutation candidate list — accurate, since this crate registers no writes. |
| Dev | `cargo test -p appfw-provider-icims --test live_smoke -- --ignored` → evidence file → provider-test live → provider-graduation | absent — evidence-gated: no `tests/live_smoke.rs` exists; blocked on authenticated Developer Community docs and a confirmed credential type before a live smoke can even authenticate. |

## 9. Graduation Status & Outstanding Evidence Gates
Ladder position: **pre-ladder evidence-gated skeleton** — no SaaS area has entered CompilerContracted → Implemented → Partial → LiveCertified; all 20 areas are unsupported (§7). Guardrails are complete (deny-by-default registry, fail-closed mutation path, tenant-binding validation, honest `access_gated_unknown` auth metadata); everything else is live-pending — "guardrail-complete, evidence-pending."

Outstanding gates (verbatim `IcimsOperationGate` reasons; owning unit unassigned — record the owner when the evidence lane is scheduled):
- [ ] AuthenticatedDeveloperDocs — "requires authenticated iCIMS Developer Community API documentation" (blocking artifact named in `metadata.rs` `blocking_gap`)
- [ ] SandboxOrTenantExport — "requires iCIMS sandbox or tenant export for schemas and samples"
- [ ] StandardFieldMatrix — "requires repeatable standard-field matrix and custom-field extension policy"
- [ ] MarketplaceValidation — "requires marketplace validation or revalidation evidence when applicable" (`ICIMS_MARKETPLACE_VALIDATION` env reference exists; gate not yet attached to any operation)
- [ ] LiveCertification — "requires live connector certification evidence"

Also required before any pin can land in the Version Link block: a vendor API version constant and base-path constant in the crate, transcribed from the authenticated docs — "unpinned — set at graduation."

## 10. PHI / Data Classification Notes
- The crate defines **no** PII/PHI flags (unlike Workday's `pii_heavy=true` worker ops) and no governance-review gate enum member (no analogue to GovernanceReview / HrPiiGovernanceReview / IdentitySecurityGovernanceReview exists in `IcimsOperationGate`). This is a declared gap, not evidence of absence of PII.
- Candidate/profile and application/workflow objects are recruiting data; treat them as PII-by-default until the StandardFieldMatrix evidence classifies fields — the crate's own operation notes require a "custom-field extension policy" before candidate reads execute.
- No PHI posture can be asserted either way: evidence-gated: awaiting authenticated export.
- Retention and tombstone expectations for any future projection of iCIMS data follow docs/runtime/saas-connectors.md; the 4a kappa rows must receive a Classification value from the data platform team when filled.
- Recommend adding a recruiting-PII governance gate member to `IcimsOperationGate` before the first candidate read graduates.
