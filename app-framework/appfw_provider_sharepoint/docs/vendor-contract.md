# SharePoint (Microsoft Graph) Connector Contract

<!-- appfw:vendor-contract-template v1
     Location: appfw_provider_sharepoint/docs/vendor-contract.md
     Gated by: scripts/appfw framework docs-check --json
       (maintainability-contract required_docs/required_tokens; saas-vendor-doc-parity
        asserts every pin in the Version Link block equals the crate's exported constant).
     Rule: values here are TRANSCRIBED from crate constants, never hand-invented.
     If evidence does not exist, write "absent - evidence-gated" and name the blocking artifact.
     NOTE: this file follows the template but has NOT been run through
     `scripts/appfw framework docs-check`. Treat it as a faithful draft, not a passed gate,
     until that check runs. -->

## Version Link

| Field | Value | Source of truth (must match) |
| --- | --- | --- |
| Vendor platform release | Microsoft Graph, commercial cloud | `SHAREPOINT_API_FAMILY` (`src/identity.rs`) |
| Vendor API version pin | `v1.0` | `SHAREPOINT_GRAPH_API_VERSION_PIN` (`src/metadata.rs`) |
| API doc / OpenAPI snapshot version | Microsoft Learn `driveItem` resource, "Upload small files" and "List versions" pages | `SHAREPOINT_GRAPH_API_DOC_URL` (`src/metadata.rs`) |
| Evidence artifact + date | **absent - evidence-gated** (no retained live run against a real tenant). Crate `blocking_gap`: "Need a FrameworkProvider::SharePoint identity + DataSourceType entry in app_gen, plus a real tenant site/drive/role grant, before any operation here can execute." | `sharepoint_api_snapshot_metadata()` (`src/metadata.rs`) |
| Information package version | n/a - Microsoft Graph's public docs are the source, not a downloaded spec bundle | - |
| Crate version | appfw-provider-sharepoint 0.1.0 | `Cargo.toml` |

## 1. Identity & Family

- `provider_key`: `sharepoint` (`SHAREPOINT_PROVIDER_KEY`, `src/identity.rs`)
- `vendor_key`: `microsoft-sharepoint-graph` (`SHAREPOINT_VENDOR_KEY`, `src/metadata.rs`)
- API family: `Microsoft Graph v1.0 (drives / sites resources)` (`SHAREPOINT_API_FAMILY`)
- API base path: `https://graph.microsoft.com/v1.0` - fixed, not per-tenant.
- Data-source binding name(s): caller-supplied via `SharePointProvider::new(data_source_name)`; in-crate tests use `sharepoint_tax_documents`.

**Asymmetry vs. most crates in this workspace:** SharePoint document storage in this product is *executed* by hand-owned code outside the framework - `tax-doc-routing/backend/src/services/sharepoint/` - because the framework wiring in section 7 does not exist. This crate is the network-free catalog entry and request-plan contract that wiring would eventually adopt.

## 2. Auth Contract

| Flow | Status | Notes |
| --- | --- | --- |
| oauth_client_credentials | preferred (only flow) | App-only client-credentials against `login.microsoftonline.com`, reusing the same two Entra app registrations (read / write) as the Teams-meeting integration (decision D4, `sharepoint-integration-plan.md`). No new credentials. |

| Env var | required_for | secret | Description (verbatim) |
| --- | --- | --- | --- |
| `SHAREPOINT_SITE_ID` | all flows | no | Graph site id for the dedicated SharePoint document site. |
| `SHAREPOINT_DRIVE_ID` | all flows | no | Graph drive id for the document library inside that site. |
| `SHAREPOINT_AUTH_MODE` | all flows | no | Tenant-confirmed auth mode; live auth remains certification-gated. |
| `GRAPH_TENANT_ID` | oauth_client_credentials | no | Microsoft Entra tenant (directory) id - single tenant. |
| `GRAPH_CLIENT_ID` | oauth_client_credentials | no | Read-scoped Entra app registration client id (shared with the Graph integration). |
| `GRAPH_CLIENT_SECRET` | oauth_client_credentials | yes | Read-scoped Entra app registration client secret value. |
| `GRAPH_WRITE_CLIENT_ID` | oauth_client_credentials | no | Write-scoped Entra app registration client id (shared with the Graph integration, decision D4). |
| `GRAPH_WRITE_CLIENT_SECRET` | oauth_client_credentials | yes | Write-scoped Entra app registration client secret value. |

**Redaction:** this crate builds no HTTP requests and holds no token, so it defines no redaction constant. The executing module (`services::sharepoint::auth`, reusing `services::graph::auth::SecretString`) redacts the secret in `Debug`/`Serialize`.

**Tenant binding** (`SharePointTenantBinding`, `src/auth.rs`): `site_id` and `drive_id` must be single-line, non-empty, no control characters. A Graph site id (`host,guid,guid`) and drive id (`b!...`) are opaque; per D3 they are never constructed or guessed, only round-tripped from the admin handoff.

## 3. Operation Catalog

**Status vocabulary (exactly four values):**
- `live_certified` - retained live evidence per `docs/runtime/saas-certification.md`. **None exists.**
- `compiler_contracted` - network-free request-plan construction proven by unit contracts. **Claimed for the four operations below** - `SharePointOperationRegistry::build_plan` builds and validates every plan and is unit-tested (22 tests).
- `planned_gated` - registered, non-executable by the *framework*; gates listed per row.
- `write_gated` - registered write candidate, non-executable pending G1 governed-write evidence.

| Operation | Kind | Status | Gates (verbatim enum members) | PII/PHI flag | Notes |
| --- | --- | --- | --- | --- | --- |
| `sharepoint.drive.list_items` | read | compiler_contracted (plan) / planned_gated (framework execution) | FrameworkProviderIdentityEvidence, TenantSiteGrantEvidence, LiveCertification | pii - file names carry entity / taxpayer names | `GET /drives/{drive}/root/children` or `/root:/{folder}:/children`. One level, not recursive. |
| `sharepoint.drive.get_item` | read | compiler_contracted / planned_gated | same | pii | `GET /drives/{drive}/items/{item}`; response carries the short-lived pre-authenticated download URL. |
| `sharepoint.drive.list_item_versions` | read | compiler_contracted / planned_gated | same | none | `GET /drives/{drive}/items/{item}/versions`. |
| `sharepoint.drive.upload_file` | write | compiler_contracted (plan) / write_gated (execution) | FrameworkProviderIdentityEvidence, TenantSiteGrantEvidence, DelegatedActorContextEvidence, TokenStoreIsolationEvidence, NamedMutationRegistryEvidence, MutationRequestBindingEvidence, IdempotencyReplayEvidence, WritePolicyScopeEvidence, WriteAuditEvidence, LiveCertification | pii | Policy scope `sharepoint.drive.write`. `PUT /drives/{drive}/root:/{path}:/content?@microsoft.graph.conflictBehavior=fail`. **Max 2 MB** (`SHAREPOINT_MAX_UPLOAD_BYTES`, decision D2). |

Plan validation rules (all unit-tested): ids are single opaque path segments; drive-relative paths reject empty, `.`, `..`, leading/trailing `/`, `\ ? # : * " < > | %`, control characters, space-padded or dot-terminated components; upload length must be 1..=2 MiB; content type must be a single-line media type.

Every operation is fail-closed for the framework: the registry exposes `ensure_operation_is_not_executable` / `ensure_named_read_is_not_executable` / `ensure_named_mutation_is_not_executable`, and the read and mutation paths reject cross-use.

**Write posture** - the shared G1 reasons are verbatim from `appfw_runtime`'s SaaS write-area reasons, not SharePoint-specific wording.

## 4. Data Paths

### 4a. Stream-fed (kappa) coverage map
| SaaS object | Streams into a store? | Collection | CDC topic | Classification | Status |
| --- | --- | --- | --- | --- | --- |
| driveItem (tax documents) | No - not applicable | - | - | pii | Documents are read live through `services::sharepoint`, not streamed into a projection store. |

### 4b. Pull-fed (sync worker)
Not applicable. No sync-worker descriptor exists for this provider.

### 4c. Live
Executable path today: `services::sharepoint::client::SharePointClient` (reads) and `services::sharepoint::writes::execute` (the only write), both product-owned, both with fake-transport unit tests and no live run.

## 5. Pagination & Continuation
Not modeled. `GET .../children` returns `@odata.nextLink` for large folders; the product registry does not page yet. Batch sizes in the source spec (3,000+ K-1s) make this a real follow-up: record the continuation style here once paging is built.

## 6. Limits, Caps & Rate-Limit Posture
Per-file upload ceiling 2 MB (`SHAREPOINT_MAX_UPLOAD_BYTES`, decision D2, enforced while the plan is built). Microsoft's simple upload supports files up to 250 MB; the 2 MB is the owner's decision, not Graph's. Response bodies are capped at 4 MiB in the product executor. 429/503 are classified as `RateLimited` and carry `Retry-After`; no retry/backoff loop exists yet (the design doc places it in the store implementation). Throttling numbers: not modeled - pending live tenant evidence.

## 7. Capability Matrix Summary
There is **no `FrameworkProvider::SharePoint` variant** in `appfw_runtime::provider_keys` and no matching `DataSourceType` entry in `app_gen`, so SharePoint is not represented in the framework's capability matrix. Adding it is a framework-source change recorded in `tax-doc-routing/docs/architecture/framework-issues-for-pds.md`; it is deliberately not patched here.

## 8. Testing Tiers Status
| Tier | Proof command | Status |
| --- | --- | --- |
| Local | `cargo test -p appfw-provider-sharepoint` | 22 network-free tests (identity, tenant binding, env-contract secrets, registry gating, plan construction, 2 MB limit, path-injection rejection, metadata parity). Passing as of authoring. |
| Pipeline | `scripts/appfw framework provider-test --provider sharepoint ...` | **Not run.** No lane exists (section 7). |
| Dev | live smoke | **absent** - no tenant site/drive/role grant exists yet. |

## 9. Graduation Status & Outstanding Evidence Gates
Ladder position: `compiler_contracted` for plan construction; **nothing is `live_certified`**.

- [ ] `FrameworkProviderIdentityEvidence`: a `FrameworkProvider::SharePoint` variant + `DataSourceType` entry in `app_gen`
- [ ] `TenantSiteGrantEvidence`: real tenant site id, drive id and `Sites.Selected` role grant (admin action - `sharepoint-graph-admin-handoff.md`)
- [ ] G1 governed-write stack for `sharepoint.drive.upload_file`: implemented in product code (`services::sharepoint::writes`) against an abstract ledger/audit sink; the framework-owned persistence for that ledger is not built
- [ ] Live connector certification (`LiveCertification`)

## 10. PII Classification Notes
Tax documents (K-1s, Form 8308, amended returns) identify entities and taxpayers; file names and folder names are treated as PII. Classification is at file level (per operation, section 3), not field level. Retention and undo posture for a removed file are governed by SharePoint's recycle bin / version history, confirmed per-tenant in `sharepoint-graph-admin-handoff.md`.
