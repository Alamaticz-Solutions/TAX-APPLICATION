# Oracle Fusion Cloud Financials Connector Contract

<!-- appfw:vendor-contract-template v1
     Location: appfw_provider_oracle_financials/docs/vendor-contract.md
     Gated by: scripts/appfw framework docs-check --json
       (maintainability-contract required_docs/required_tokens; saas-vendor-doc-parity
        asserts every pin in the Version Link block equals the crate's exported constant).
     Rule: values here are TRANSCRIBED from crate constants, never hand-invented.
     If evidence does not exist, write "absent — evidence-gated" and name the blocking artifact. -->

## Version Link

| Field | Value | Source of truth (must match) |
| --- | --- | --- |
| Vendor platform release | Oracle Fusion Cloud Financials "26B" | `ORACLE_FINANCIALS_RELEASE` (`src/identity.rs`) |
| Vendor API version pin | REST resources base `/fscmRestApi/resources/11.13.18.05` | `ORACLE_FINANCIALS_REST_RESOURCES_BASE_PATH` (`src/operation.rs`) |
| API doc / OpenAPI snapshot version | OpenAPI 2026.03.27 (1,391 paths / 2,241 operations / 1,325 GETs pinned in code) | `ORACLE_FINANCIALS_OPENAPI_VERSION` (`src/identity.rs`); counts in `src/metadata.rs` |
| Evidence artifact + date | `vendor_api_specs_2026-06-26/enrichment/information-package-versions.json`, `vendor_api_specs_2026-06-26/enrichment/saas-provider-implementation-metadata.json`, `vendor_api_specs_2026-06-26/enrichment/extracted/oracle-financials-openapi-summary.json` (2026-06-26) + https://docs.oracle.com/en/cloud/saas/financials/26b/farfa/openapi.json | `metadata.rs` `source_refs` (`API_SNAPSHOT_SOURCES`) |
| Information package version | 2026.06.26.1 | `ORACLE_FINANCIALS_INFORMATION_PACKAGE_VERSION` |
| Crate version | appfw-provider-oracle-financials 0.1.0 | `Cargo.toml` |
| Doc last reconciled | 2026-07-02 | this file |

## 1. Identity & Family

- `provider_key`: `oracle_financials` (`ORACLE_FINANCIALS_PROVIDER_KEY`)
- `vendor_key`: `oracle-fusion-financials-26b` (`ORACLE_FINANCIALS_VENDOR_KEY`)
- API family: `Oracle Fusion Cloud Financials REST API` (`ORACLE_FINANCIALS_API_FAMILY`) — REST/JSON via the shared `SaasTransportProtocol::RestJson` kernel
- API base path: `/fscmRestApi/resources/11.13.18.05`
- Spec package name: `oracle-fusion-financials-26b-openapi` (`ORACLE_FINANCIALS_API_SPEC_PACKAGE_ID`)
- Data-source binding name(s): none registered in `_config` yet — the crate binds a per-instance `data_source_name` (crate contract tests use `oracle_financials_primary`); binding a downstream data source is evidence-gated: awaiting a tenant-approved Oracle Fusion instance.

## 2. Auth Contract

**Flows** (from `src/auth.rs`, verbatim):

| Flow | Status | Notes |
| --- | --- | --- |
| tenant_oauth_or_oidc | preferred | Planning-only today. Live auth is certification-gated; `ConnectionAuth` is `saas_unsupported` ("requires live tenant evidence before this SaaS contract area can be certified"). No token acquisition or transport exists in this crate. |
| basic_user_password | exception | Tenant-approved Oracle integration user for exception flows only; same certification gate as above. |

**Environment contract** (transcribed from `ORACLE_FINANCIALS_AUTH_ENV_VARS` in `src/auth.rs`):

| Env var | Required for | Secret | Description (verbatim) |
| --- | --- | --- | --- |
| `ORACLE_FINANCIALS_BASE_URL` | all flows | no | Tenant-specific Oracle Fusion Cloud Financials HTTPS origin. |
| `ORACLE_FINANCIALS_AUTH_MODE` | all flows | no | Tenant-confirmed auth mode; live auth remains certification-gated. |
| `ORACLE_FINANCIALS_CLIENT_ID` | tenant_oauth_or_oidc | no | Tenant-approved OAuth/OIDC client id, if that flow is certified. |
| `ORACLE_FINANCIALS_CLIENT_SECRET` | tenant_oauth_or_oidc | yes | Tenant-approved OAuth/OIDC client secret or secret reference. |
| `ORACLE_FINANCIALS_USERNAME` | basic_user_password | no | Tenant-approved Oracle integration user for exception flows. |
| `ORACLE_FINANCIALS_PASSWORD` | basic_user_password | yes | Tenant-approved Oracle integration password or secret reference. |
| `ORACLE_FINANCIALS_REST_FRAMEWORK_VERSION` | all flows | no | Server-bound Oracle REST framework version header value. |
| `ORACLE_FINANCIALS_METADATA_CONTEXT` | all flows (optional value) | no | Optional server-bound Oracle Metadata-Context header value. |
| `ORACLE_FINANCIALS_DATA_SECURITY_SCOPE` | all flows | no | Server-bound ledger, business unit, or enterprise data-security scope label. |

**Redaction:** `Authorization: Bearer <redacted>` (`ORACLE_FINANCIALS_AUTHORIZATION_HEADER` + `ORACLE_FINANCIALS_REDACTED_BEARER_TOKEN`).
**Tenant binding validation** (`OracleFinancialsTenantBinding`): HTTPS-origin-only base URL, single line, no whitespace or control characters, no path/query/fragment, DNS host without userinfo or port; REST framework version restricted to ASCII alphanumerics plus `.-_`; metadata context and data-security scope must be single-line and non-empty; CR/LF rejected everywhere (header-injection defense).
**Server-bound values callers cannot override:** `REST-Framework-Version` header, optional `Metadata-Context` header, and the ledger/business-unit/enterprise data-security scope label. Registry-bound collection paths are compiled in; callers cannot choose `path`, `q`, `finder`, `fields`, `expand`, or `orderBy` (registry description, `src/registry.rs`).

## 3. Operation Catalog

**Status vocabulary (exactly four values — do not invent others):**
- `live_certified` — retained live evidence per docs/runtime/saas-certification.md. **None exists for any SaaS provider today; the column stays anyway.**
- `compiler_contracted` — network-free request-**plan** construction proven by unit contracts. **NOT live-callable** until provider binding, auth, redaction, and live-certification evidence are retained through the runtime SaaS executor; never conflate request planning with executable-against-vendor.
- `planned_gated` — registered, non-executable; gates listed per row.
- `write_gated` — registered write candidate, non-executable pending G1 governed-write evidence.

| Operation | Kind (read/write) | Status | Gates (verbatim enum members) | PII/PHI flag | Notes |
| --- | --- | --- | --- | --- | --- |
| oracle.financials.accounting_period_status_lov.list | read | compiler_contracted | — | no (Operational) | Fixed GET `/accountingPeriodStatusLOV`; plan construction proven, not live-callable. |
| oracle.financials.accounting_period_status_lov.get | read | compiler_contracted | — | no (Operational) | Fixed GET `/accountingPeriodStatusLOV/{accountingPeriodStatusLOVUniqID}`; single validated path segment; caps `max_rows=1`. |
| oracle.financials.openapi.describe_resource | read | planned_gated | TenantOperationAllowList, LiveCertification | no (Operational) | "OpenAPI describe planning stays gated until selected resource descriptions are allowlisted." |
| oracle.financials.collection_query | read | planned_gated | TenantOperationAllowList, LiveCertification | no (FinancialConfidential) | Generic collection reads must become registry-bound operations (`collection_query_for_resource`) before execution. |
| oracle.financials.fetch_by_id | read | planned_gated | TenantOperationAllowList, LiveCertification | no (FinancialConfidential) | Generic fetch-by-id must become a registry-bound path (`fetch_by_id_for_resource`) before execution. |
| oracle.financials.list_lov | read | planned_gated | TenantOperationAllowList, LiveCertification | no (Operational) | Generic LOV reads must become registry-bound operations before execution. |
| oracle.financials.submit_erpintegration | write | write_gated | GovernedWriteReview, TenantAuthCertification, LiveCertification | no (FinancialConfidential) | POST `/erpintegrations`; "cataloged for future governed write review only." `build_named_mutation_plan()` always errors `UnsupportedMutation` — the provider plans no mutations at all today. |

Write posture — shared G1 reasons, verbatim from `appfw_runtime/src/provider_capabilities.rs:696-711`:
- "SaaS provider writes remain unsupported until named mutation safety, idempotency, audit, and live write evidence exist"
- "SaaS delegated/on-behalf-of actor context remains unsupported until per-user token storage, tenant binding, and impersonation audit evidence exist"
- "SaaS per-user token-store isolation remains unsupported until encrypted storage, rotation, tenant partitioning, and revocation evidence exist"
- "SaaS named mutations remain unsupported until a provider-specific mutation registry and deny-by-default exposure gate exist"
- "SaaS mutation request binding remains unsupported until provider-specific request templates, parameter binding, and redaction evidence exist"
- "SaaS write idempotency and replay protection remain unsupported until idempotency keys, retry classification, and duplicate-suppression evidence exist"
- "SaaS write policy and scope enforcement remain unsupported until operation-level policy, MCP/Kafka/frontend exposure gates, and least-privilege scopes are proven"
- "SaaS write audit remains unsupported until before/after redaction, request correlation, undo/compensation posture, and retained evidence exist"

Cross-provider reference: Oracle Financials registers **zero** named mutations in the G1 lane (`known_named_mutations_by_provider["oracle_financials"] = []` in `scripts/appfw`). The framework's only gated non-executable named-mutation candidate today is `servicenow.create_incident` (policy scope `servicenow.incident.write`, ServiceNow crate, current working tree) — it defines the G1 evidence bar `oracle.financials.submit_erpintegration` must also clear before graduating from `write_gated`.

## 4. Data Paths

### 4a. Stream-fed (kappa) coverage map
| SaaS object | Streams into Mongo? | Mongo collection | CDC topic | Classification | Status |
| --- | --- | --- | --- | --- | --- |
| accountingPeriodStatusLOV | **UNKNOWN — to be filled by data platform team** | — | — | — | placeholder |
| erpintegrations | **UNKNOWN — to be filled by data platform team** | — | — | — | placeholder |
| (all other Oracle Financials objects) | **UNKNOWN — to be filled by data platform team** | — | — | — | placeholder |

Covered objects are read as Mongo projections (no sync worker); CDC topics feed reactive projections via governed Kafka ingress. Provenance markers, echo-loop exclusions, and stream-offset freshness apply per docs/runtime/saas-connectors.md.

### 4b. Pull-fed (sync worker — coverage gaps)
Watermark field(s): not modeled — evidence-gated. `IncrementalWatermark` is `saas_unsupported` with the Oracle-specific reason: "requires Oracle Financials tenant auth, business-unit or ledger scoping, and selected OpenAPI operation allow-lists before named-operation contracts can be implemented or certified" (`provider_capabilities.rs:690-691`). No sync descriptor names exist for this provider. Projection store binding: not yet recorded — Mongo is the default candidate where 4a covers adjacent objects; Postgres remains fully supported; the per-schema choice and rationale must be recorded here when the first Oracle projection is bound.

### 4c. Live (Archetype-2)
Named live-read operations: none are live. The two `compiler_contracted` LOV reads (section 3) prove network-free plan construction only. Runtime HTTP executor substrate exists, but Oracle live execution still waits on tenant auth, provider binding, redaction evidence, and live certification. Governed-write candidates: `oracle.financials.submit_erpintegration` (section 3 `write_gated` row) — cataloged for future governed-write review only.

## 5. Pagination & Continuation
Shared-kernel style: `OffsetLimitContinuation` (appfw_saas_core). Vendor field names: request query params `offset` + `limit` (with fixed `onlyData=true`, `links=false`, `totalResults=true` on collection reads); response paging fields `items`, `count`, `offset`, `limit`, `totalResults`, `hasMore` parsed by `OracleFinancialsCollectionSummary` (`src/response.rs`), which builds the next `OffsetLimitContinuation` without retaining row payloads.

## 6. Limits, Caps & Rate-Limit Posture
Defaults: 100 rows (`DEFAULT_MAX_RESULTS`) / 3,000 ms timeout (`DEFAULT_TIMEOUT_MS`); MAX page limit 500 (`MAX_PAGE_LIMIT`), enforced by clamping both `limit` and `SaasResponseCaps.max_rows`; fetch-by-id caps `max_rows=1`. Vendor hard caps: not modeled — evidence-gated: awaiting authenticated export / live tenant evidence. Rate-limit signals: not modeled — pending live tenant evidence (`RateLimitBackoff` is `saas_unsupported`: "requires live tenant evidence before this SaaS contract area can be certified"). SOAP fault classification: n/a (REST/JSON only).

## 7. Capability Matrix Summary
Transcribed from `ORACLE_FINANCIALS_SAAS_CAPABILITIES` (`appfw_runtime/src/provider_capabilities.rs:1188-1270`). No area is live-certified.

| # | SaasReadArea | Status | Reason / evidence |
| --- | --- | --- | --- |
| 1 | ConnectionAuth | saas_unsupported | requires live tenant evidence before this SaaS contract area can be certified |
| 2 | NamedOperationRegistry | saas_compiler_contracted | registry tests: `registry_exposes_stable_operation_names`, `metadata_operation_names_match_registry_constants`, `unknown_operation_is_rejected` |
| 3 | RequestBinding | saas_compiler_contracted | `accounting_period_status_lov_list_converts_to_fixed_saas_request_plan`, `accounting_period_status_lov_get_requires_single_id_path_segment`, `broad_openapi_operations_are_planned_until_allowlisted`, `submit_erp_integration_remains_unsupported_write_planning` |
| 4 | PaginationCursoring | saas_compiler_contracted | `accounting_period_status_lov_list_accepts_typed_offset_continuation`, `collection_summary_builds_next_offset_continuation` |
| 5 | RateLimitBackoff | saas_unsupported | requires live tenant evidence before this SaaS contract area can be certified |
| 6 | IncrementalWatermark | saas_unsupported | requires Oracle Financials tenant auth, business-unit or ledger scoping, and selected OpenAPI operation allow-lists before named-operation contracts can be implemented or certified |
| 7 | FieldRedaction | saas_unsupported | requires live tenant evidence before this SaaS contract area can be certified |
| 8 | TenantScoping | saas_compiler_contracted | `tenant_binding_rejects_paths_ports_and_empty_scope`, `provider_plans_named_reads_as_shared_saas_request_without_network` |
| 9 | SchemaVersionPinning | saas_compiler_contracted | `descriptor_uses_oracle_financials_provider_key_and_api_pin`, `api_snapshot_metadata_pins_oracle_financials_openapi_release`, `resources_base_path_uses_oracle_fusion_rest_resource_root` |
| 10 | ResultAndTimeoutCaps | saas_compiler_contracted | `accounting_period_status_lov_list_converts_to_fixed_saas_request_plan`, `accounting_period_status_lov_list_caps_page_limit` |
| 11 | QueryMetricsAndAudit | saas_unsupported | requires live tenant evidence before this SaaS contract area can be certified |
| 12 | FreshnessReporting | saas_unsupported | requires live tenant evidence before this SaaS contract area can be certified |
| 13 | GovernedWriteEnforcement | saas_unsupported_with_evidence | shared governed-write reason + compiler evidence `provider_rejects_mutation_planning_for_now` |
| 14 | DelegatedActorContext | saas_unsupported | shared delegated-actor reason (section 3) |
| 15 | TokenStoreIsolation | saas_unsupported | shared token-store reason (section 3) |
| 16 | NamedMutationRegistry | saas_unsupported | shared named-mutation reason (section 3) |
| 17 | MutationRequestBinding | saas_unsupported | shared mutation-binding reason (section 3) |
| 18 | IdempotencyAndReplayProtection | saas_unsupported | shared idempotency reason (section 3) |
| 19 | WritePolicyAndScopeEnforcement | saas_unsupported | shared write-policy reason (section 3) |
| 20 | WriteAuditAndEvidence | saas_unsupported | shared write-audit reason (section 3) |

Honest asymmetries: Oracle Financials `FieldRedaction` and `IncrementalWatermark` are unsupported while Salesforce holds compiler-contracted evidence for both; Salesforce also carries unsupported-with-evidence `RateLimitBackoff` while Oracle's is plain unsupported. Read-side summary: 6 of 12 read areas compiler-contracted, 6 unsupported; all 8 write areas unsupported (one with rejection evidence).

## 8. Testing Tiers Status
| Tier | Proof command | Status |
| --- | --- | --- |
| Local | `cargo test -p appfw-provider-oracle-financials`; fixtures under `fixtures/operations/` (provenance: synthetic \| vendor_export \| recorded_live) | In-crate unit contracts in `src/*` modules (synthetic in-test JSON); these are the exact tests cited as certification evidence in `provider_capabilities.rs`. **No `fixtures/operations/` directory exists yet** — absent; add with provenance markers when vendor-export fixtures land. |
| Pipeline | `scripts/appfw framework provider-test --provider oracle_financials --area saas-read --plan --json` (+ `--area governed-write --plan` if writes registered) | `--area saas-read --plan` exists and retains `target/appfw/saas-read-provider-test.json` with the 12 read-area checks, current unsupported rows, and `release_ready:false` until live smoke evidence exists. The governed-write plan lane accepts `oracle_financials` but reports an empty named-mutation list — accurate, since this crate registers no named mutations. |
| Dev | `cargo test -p appfw-provider-oracle-financials --test live_smoke -- --ignored` → evidence file → provider-test live → provider-graduation | absent — evidence-gated: no `tests/live_smoke.rs` target exists; blocked on tenant-supported Oracle Fusion authentication evidence (TenantAuthCertification) and a tenant-approved operation allow-list. |

## 9. Graduation Status & Outstanding Evidence Gates
Current ladder position: **CompilerContracted** (per docs/runtime/saas-certification.md: CompilerContracted → Implemented → Partial → LiveCertified). Read-side planning lanes are guardrail-complete, live-pending. Remaining gates checklist (gate reasons verbatim from `OracleFinancialsOperationGate::reason()`):

- [ ] TenantHostAndDataSecurityScope — "requires tenant-approved Oracle host and data-security scope" (owner: unassigned — Oracle Fusion tenant owner must supply the approved host + ledger/BU scope)
- [ ] TenantOperationAllowList — "requires selected Oracle OpenAPI operation allow-list before execution" (owner: unassigned — evidence-gated: awaiting authenticated export of the tenant's approved resource list)
- [ ] TenantAuthCertification — "requires tenant-supported Oracle Fusion authentication evidence" (owner: unassigned)
- [ ] GovernedWriteReview (submit_erpintegration only) — "requires governed write safety, idempotency, and audit review" (blocked behind the full shared G1 stack; see section 3)
- [ ] LiveCertification — "requires live connector certification evidence" (blocked: no live cert runner has executed for any SaaS provider)
- [ ] RuntimeExecutor — requires provider binding/auth/redaction/live evidence before Oracle live execution can use the runtime HTTP executor substrate

## 10. PHI / Data Classification Notes
No PHI is expected in Oracle Financials objects; the crate's sensitivity taxonomy is `Operational` vs `FinancialConfidential` (`OracleFinancialsDataSensitivity`). FinancialConfidential applies to `collection_query`, `fetch_by_id`, and `submit_erpintegration`; the accounting-period LOV reads are Operational. Governance-review gates: GovernedWriteReview (write path); no HrPiiGovernanceReview / IdentitySecurityGovernanceReview gates apply to this provider. FieldRedaction is unsupported (section 7) — until live redaction evidence exists, treat every projected Oracle payload as financially confidential by default. Retention and tombstone expectations for any projection of this vendor's data: **not defined — placeholder for the data platform team**, to be set together with the kappa coverage rows in 4a; classify each 4a row (Operational vs FinancialConfidential) as it is filled.
