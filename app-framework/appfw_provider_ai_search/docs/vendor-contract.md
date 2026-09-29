# AI Search Connector Contract

<!-- appfw:vendor-contract-template v1
     Location: appfw_provider_ai_search/docs/vendor-contract.md
     Gated by: scripts/appfw framework docs-check --json
       (maintainability-contract required_docs/required_tokens; provider vendor docs are
        framework-owned and must stay transcribed from crate constants).
     Rule: values here are TRANSCRIBED from crate constants, never hand-invented.
     If evidence does not exist, write "absent — evidence-gated" and name the blocking artifact. -->

## Version Link

| Field | Value | Source of truth (must match) |
| --- | --- | --- |
| Vendor platform release | absent — evidence-gated: internal PDS AI search API contract/export is not present. Release basis is "internal PDS AI search API absent - evidence-gated" | `AI_SEARCH_RELEASE_BASIS` (`src/identity.rs`) |
| Vendor API version pin | absent — evidence-gated: this crate deliberately builds no AI search HTTP requests; no API version or base-path constant exists to transcribe | `src/lib.rs` crate doc; no operation base-path constant exists |
| API doc / OpenAPI snapshot version | absent — evidence-gated: authenticated PDS AI search API spec/export required | `src/metadata.rs` `AI_SEARCH_IMPLEMENTATION_METADATA_PATH` |
| Evidence artifact + date | **absent — evidence-gated** (authenticated PDS AI search API spec/export and recorded-live fixtures, CH9). Crate `blocking_gap`: "Need authenticated PDS AI search API spec/export plus recorded-live fixtures before request planning or execution." | `src/metadata.rs` `ai_search_api_snapshot_metadata()` `source_refs` / `blocking_gap` |
| Information package version | absent-evidence-gated | `AI_SEARCH_INFORMATION_PACKAGE_VERSION` (`src/metadata.rs`) |
| Crate version | appfw-provider-ai-search 0.1.0 | `Cargo.toml` |
| Doc last reconciled | 2026-07-03 | this file |

## 1. Identity & Family

- `provider_key`: `ai_search` (`AI_SEARCH_PROVIDER_KEY`, `src/identity.rs`)
- `vendor_key`: `pds-ai-search` (`AI_SEARCH_VENDOR_KEY`, `src/metadata.rs`)
- API family: `PDS AI search named-operation contract` (`AI_SEARCH_API_FAMILY`)
- API base path: **absent — evidence-gated**. The crate validates only an endpoint HTTPS origin (`AI_SEARCH_BASE_URL`); request construction awaits the authenticated PDS AI search API export.
- Spec package name: `pds-ai-search-api-specs` (`AI_SEARCH_API_SPEC_PACKAGE_ID`)
- Data-source binding name(s): caller-supplied per product via `AiSearchProvider::new(data_source_name)`; the in-crate contract tests use `ai_search_primary`.

## 2. Auth Contract

**Flows** (flow names verbatim from `AiSearchAuthFlow::as_str()`, `src/auth.rs`):

| Flow | Status | Notes |
| --- | --- | --- |
| api_key | planned | Public-service API key shape, such as an Anthropic-style `x-api-key`; execution remains evidence-gated. |
| bearer_token | planned | Static bearer token for self-hosted serving stacks; execution remains evidence-gated. |
| client_credentials | preferred | OAuth2 client-credentials shape for internal PDS AI search; token exchange remains evidence-gated. |

**Gateway/auth posture (CH7, transcribed from `src/gateway.rs`):**

| Field | Value |
| --- | --- |
| Preferred gateway | `litellm` |
| Allowed gateway classes | `litellm`, `openai_compatible`, `internal_pds_search` |
| Deployment posture | `in_perimeter` |
| Credential custody | `server_secret_ref_only` |
| Preferred auth flow | `client_credentials` |
| Policy authority | `runtime_policy_and_generated_operation_dispatcher` |
| Token-cache key | `{provider_key:"ai_search", data_source_name, tenant_key}` |

Gateway evidence required before any provider execution:
`gateway-selection-decision`, `server-secret-ref-custody`,
`client-credentials-token-cache`, `prompt-audit-siem-retention`,
`egress-allowlist`, and `policy-re-resolution`. The gateway is custody and
transport only; it never becomes policy authority and never receives
browser-held credentials.

**Environment contract** (transcribed from the typed env constants in `src/auth.rs`):

| Env var | required_for | secret | Description (verbatim) |
| --- | --- | --- | --- |
| `AI_SEARCH_BASE_URL` | all flows | no | Bound AI search/generation endpoint origin. |
| `AI_SEARCH_API_KEY` | api_key | yes | Public-service API key, such as an Anthropic-style x-api-key. |
| `AI_SEARCH_BEARER_TOKEN` | bearer_token | yes | Static bearer token for self-hosted serving stacks. |
| `AI_SEARCH_CLIENT_ID` | client_credentials | no | OAuth2 client-credentials client id. |
| `AI_SEARCH_CLIENT_SECRET` | client_credentials | yes | OAuth2 client-credentials secret reference. |
| `AI_SEARCH_TOKEN_URL` | client_credentials | no | OAuth2 token endpoint. |

**Redaction:** `x-api-key`, `api-key`, and `authorization` are redacted by `appfw_saas_core::redaction`; this crate has a compiler contract asserting those shared header names remain sensitive.

**Tenant binding validation** (`AiSearchTenantBinding`, `src/auth.rs`): base URL must be a single-line `https://` origin with no whitespace or control characters, no path/query/fragment, no userinfo, and a DNS host. Unlike Salesforce and ServiceNow, explicit numeric ports are allowed because internal gateways and local approved endpoints may expose non-default ports.

**Server-bound values callers cannot override:** endpoint origins, token URLs, and credential refs are server-owned configuration. Callers can ask for named operations only; they cannot inject raw URLs, prompts, model URLs, SQL/vector queries, or arbitrary search backend payloads.

## 3. Operation Catalog

**Status vocabulary (exactly four values — do not invent others):**
- `live_certified` — retained live evidence per docs/runtime/saas-certification.md. **None exists for AI search today.**
- `compiler_contracted` — network-free request-plan construction proven by unit contracts. **Not used yet** because no authenticated search contract/export exists.
- `planned_gated` — registered, non-executable; gates listed per row.
- `write_gated` — not applicable; AI search has no named write operations in this crate.

| Operation | Kind | Status | Gates (verbatim enum members) | PII/PHI flag | Notes |
| --- | --- | --- | --- | --- | --- |
| `ai_search.search` | read/query | planned_gated | EnterpriseContractEvidence, GatewayAuthEvidence, SharedSaasHttpExecutor, LiveCertification | unclassified — evidence-gated: awaiting authenticated API/export and recorded-live fixtures | Planned enterprise search named query. Results must be pointers/references for the chat orchestrator to re-resolve through runtime policy, not pre-rendered UI content. |
| `ai_search.embed` | read/query | planned_gated | EnterpriseContractEvidence, GatewayAuthEvidence, SharedSaasHttpExecutor, LiveCertification | unclassified — evidence-gated: awaiting authenticated API/export and recorded-live fixtures | Planned embedding named query for approved search ingestion or query embedding flows. |

Every registered operation is fail-closed: the registry exposes only `ensure_named_query_is_not_executable` and `ensure_named_mutation_is_not_executable`. Unknown names are rejected, and the mutation path always rejects the named query catalog.

## 4. Data Paths

### 4a. Stream-fed (kappa) coverage map
No direct projection coverage exists in this crate. Search results must carry record locators or resolvable source pointers, then the product resolves them through runtime data access under the signed-in user's policy context.

### 4b. Pull-fed
No sync worker exists for this provider. Search index ingestion and embedding production are outside this crate until the PDS AI platform provides the authenticated contract/export and retained evidence.

### 4c. Live
Named live-read operations: none executable. `ai_search.search` and `ai_search.embed` are registered only as fail-closed contract names. There are no governed-write candidates.

## 5. Pagination & Continuation

Not modeled — evidence-gated: awaiting authenticated API/export. The crate pins no cursor, next-page, token, continuation, or result-window field names.

## 6. Limits, Caps & Rate-Limit Posture

Not modeled — evidence-gated: awaiting authenticated API/export. No default row/timeout/token caps are asserted in the crate. Any future caps must land as crate constants first and be transcribed here.

## 7. Capability Matrix Summary

`FrameworkProvider::AiSearch` is its own provider family (`FrameworkProvider::AI_SEARCH`), separate from database CRUD, graph-read, and external-API providers. `provider-graduation` reports one AI search `contract_evidence` area, currently `unsupported`, with compiler contracts proving this crate's fail-closed registry, auth metadata, and metadata/registry parity. It does **not** claim executable provider calls.

## 8. Testing Tiers Status

| Tier | Proof command | Status |
| --- | --- | --- |
| Local | `cargo test -p appfw-provider-ai-search` | Network-free fail-closed contract tests in-crate (identity, auth/tenant-binding, redaction headers, registry rejection, metadata parity). |
| Pipeline | `scripts/appfw framework provider-graduation --json` | Reports AI search as planned contract evidence with zero graduated capabilities. |
| Dev | `scripts/appfw framework provider-test --provider ai_search --plan --json` | Plan slot documented; executable provider-test remains absent until the PDS AI search contract/export and shared SaaS executor are available. |

## 9. Graduation Status & Outstanding Evidence Gates

Ladder position: registered fail-closed skeleton (metadata + planned catalog only). Guardrails are complete for CH3; live provider capability is pending.

Outstanding gates checklist:
- [ ] Authenticated PDS AI search API spec/export (`EnterpriseContractEvidence`)
- [ ] Approved gateway/auth evidence (`GatewayAuthEvidence`): gateway decision, server-side secret refs, client-credentials token-cache posture, prompt audit/SIEM retention, egress allowlist, and policy re-resolution
- [ ] Recorded-live or vendor-export fixtures with provenance `vendor_export|recorded_live`
- [ ] Shared SaaS HTTP executor integration (`SharedSaasHttpExecutor`)
- [ ] Provider-test plan/live path for `--provider ai_search`
- [ ] Live connector certification (`LiveCertification`) per docs/runtime/saas-certification.md

## 10. PHI / Data Classification Notes

The crate models no PII/PHI flags for AI search payloads — classification is **evidence-gated** until the API/export identifies request/response fields and source-system provenance. Operational caution: search snippets and embeddings can encode PHI or PII from upstream systems. Treat every search hit, query, prompt, snippet, embedding input, and citation as at least confidential/PHI-bearing until field-level evidence proves a narrower classification.
