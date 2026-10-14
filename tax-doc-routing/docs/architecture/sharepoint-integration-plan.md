# SharePoint document storage — integration plan

> **Status: provider and gate built and unit-tested; nothing is live.**
> Audience: engineering and the product owner. The Microsoft admin team gets
> [`sharepoint-graph-admin-handoff.md`](sharepoint-graph-admin-handoff.md) instead.
> Resolves design decision **D11** in [`tax-document-routing-design.md`](tax-document-routing-design.md)
> (storage Box → SharePoint, "gather specifics when that work starts").

Everything below is either verified against a file in this repo, taken from
Microsoft Learn, or marked **[unverified]**.

---

## 1. What this is

SharePoint is added as its **own provider** for document storage. The wire
target is Microsoft Graph (same tenant, same two app registrations as other Graph
integrations), but SharePoint has its own crate, its own product module, its own
write gate, ledger correlation, policy scope and audit action names.

| Layer | Where | What |
|---|---|---|
| Framework crate | `app-framework/appfw_provider_sharepoint/` | `auth`, `error`, `identity`, `metadata`, `provider`, `registry`, `docs/vendor-contract.md`. Network-free: builds and validates request plans, does not execute them. Registered in `app-framework/Cargo.toml`. |
| Product module | `tax-doc-routing/backend/src/services/sharepoint/` | `identity`, `auth`, `registry`, `client` (reads), `writes` (governed upload gate), `vendor_contract`. Re-exports escaping/classification from `services::graph`. |
| Shared Graph plumbing | `tax-doc-routing/backend/src/services/graph/` | `identity`, `auth` (token acquisition), `request`, `response`. No feature code. |

Nothing in `services::sharepoint` is reachable from any other Graph feature, and
no Graph feature is reachable from it except the generic helpers above.

---

## 2. Decisions

| # | Decision | Status | Notes |
|---|---|---|---|
| **D1** | SharePoint is a **separate provider**, key `sharepoint`, API surface Microsoft Graph v1.0. | Decided (owner) | Not folded into any Graph/Teams feature code. |
| **D2** | **2 MB per file**, enforced when the request plan is built (`SHAREPOINT_MAX_UPLOAD_BYTES`). | Decided (owner) | Graph's simple upload allows 250 MB; 2 MB is the product's own ceiling. Open question: real K-1 PDFs may exceed it — see §9. |
| **D3** | One site and one drive, configured by `SHAREPOINT_SITE_ID` / `SHAREPOINT_DRIVE_ID`. IDs are **never constructed or guessed**; they come from the admin. No placeholder default exists. | Decided | `SharePointTarget::from_env` returns `None` until both are set. |
| **D4** | Reuse the **same two Entra app registrations** (read app `GRAPH_CLIENT_*`, write app `GRAPH_WRITE_CLIENT_*`); no new keys requested from IT. | Decided (owner) | Token acquisition is imported, not copied. **Unlike the Project Governance Graph module, there is no fallback from the write app to the read app**: a missing write registration means SharePoint writes are unconfigured, never that uploads run on the read app's identity. |
| **D5** | **What goes to SharePoint** and the path scheme — see §3. | Provisional | Folder scheme needs the business spec's folder structure (BS §14), which is not in this repo. |
| **D6** | Reads are named and allow-listed: `list_drive_items`, `get_drive_item`, `list_drive_item_versions`. There is no `get(path)`. | Decided | Bound parameters only; ids are single path segments; paths reject `..`, empty components and `\ ? # : * " < > \| %`. |
| **D7** | One write: `sharepoint_upload_file`, behind the governed-write gate, **own** policy scope `sharepoint.drive.write`, own idempotency ledger entry, own audit action `SHAREPOINT_WRITE_SHAREPOINT_UPLOAD_FILE_<OUTCOME>`. **No delete, rename, move, replace.** | Decided | `conflictBehavior=fail`: a retry replays through the ledger or fails loudly; it never overwrites. |
| **D8** | Roles allowed to upload: `tax_staff`, `tax_admin` (spec-2: exactly two roles exist). Per-record ownership (`assigned_operator`) is the calling service's check, not the gate's. | Provisional | Confirm with owner. |
| **D9** | **PDF only** (`application/pdf`). | Provisional | Matches the design doc's "PDF FileUpload". Widening is a product decision. |
| **D10** | **Nothing is `live_certified`.** Tiers use the 4-value vocabulary; `vendor_contract::assert_honest` enforces it in a test. | Decided | |
| **D11** | The framework has **no `FrameworkProvider::SharePoint` / `DataSourceType` entry**. We do **not** patch the framework. | Decided | Recorded in `framework-issues-for-pds.md` (finding S1). |

---

## 3. Which documents go to SharePoint (D5)

This differs from Project Governance, where only user uploads exist. Tax
Document Routing's whole job is filing documents, so the rule here is:

| Goes to SharePoint | Never goes to SharePoint |
|---|---|
| The PDF a staff user uploads (`TaxDocument.attachment_file`) and the K-1 / Form 8308 / amended files they attach (`k1_attachments`, `form_8308_attachments`, `amended_documents`) | System logs, job-runner state, `storage_operation_log` rows (these stay in Postgres) |
| The routed copies the workflow files from those uploads (Internal Quick View, Internal Client Copy, External Client Copy — design §4.4), **only when produced by the routing job through the `DocumentStore`** | Temporary/in-memory processing buffers (PDF protection is in-memory only, no temp files, design §4.4) |
| | The master/owner password, per-client passwords, or any secret |
| | AI-extraction or other intermediates (none exist in this product today) |

**Fail closed.** If PDF protection fails, nothing is uploaded (design D9). The
upload gate never sees an unprotected file when protection was required.

**Path.** `{entity_folder}/{tax_document_id}__{sanitised_file_name}`:

- `entity_folder` is the entity's Oracle ID (design D3) or `_unassigned`; it must
  be an opaque key (letters, digits, `-`, `_`). It comes from the system, never
  the uploader.
- `tax_document_id` is the system id. Only the file name is user-supplied and it is
  sanitised (separators, `: * ? " < > | # %` and control characters become `_`,
  leading/trailing dots and spaces trimmed, max 100 characters).
- Because the id is in the name, paths are collision-free, which is what makes
  `conflictBehavior=fail` idempotent rather than destructive.

**Metadata** (id, name, type, uploader, checksum, SharePoint item id) stays in
Postgres. SharePoint keeps the bytes and its own version history.

---

## 4. How a user later downloads

1. The UI asks the backend for a document link for a `TaxDocument`.
2. The service checks the caller may see that record (Tax Staff: only records where
   `assigned_operator` is themselves; Tax Admin: all — design D1), then calls the
   `get_drive_item` named read with the stored drive-item id.
3. Graph's response already contains `@microsoft.graph.downloadUrl`, a short-lived,
   **pre-authenticated** URL. The backend returns it; file bytes never stream
   through this backend or GraphQL.
4. Issuing a link must be audited (the bytes bypass us, so the audit event is the
   only evidence).

**[unverified]** the download URL's exact lifetime and behaviour for app-only
tokens — confirm in the live run.

**Not built:** the custom method / handler that exposes this to the UI (see §6).

---

## 5. Versions

- SharePoint keeps prior versions of a file when **versioning is on** for the library
  (an admin-handoff checklist item).
- `list_drive_item_versions` reads that history (`GET /drives/{drive}/items/{item}/versions`).
- This product never overwrites: `conflictBehavior=fail` and no replace operation
  exist. A corrected document is a **new** file (new `tax_document_id` → new path).
  Version history therefore only matters if someone replaces a file directly in
  SharePoint, or if a replace operation is added later (it is not planned).
- Restoring a deleted file is SharePoint's recycle bin, not this product.
  Retention is confirmed per tenant by the admin (handoff Step 6).

---

## 6. Built vs gated

| Item | State |
|---|---|
| Framework crate: identity, auth env contract, registry, request-plan builder, metadata, provider, vendor contract | **Built.** 22 unit tests. |
| Request plans for 3 reads + upload, incl. 2 MB limit and path-injection rejection | **Built**, tested. |
| Product `graph` shared base (token acquisition, escaping, classification, redaction) | **Built**, tested. Network paths (`GraphToken::bearer`, `GraphUploadTransport::put`, `SharePointClient::read` network call) are **untested** — no tenant. |
| Product `sharepoint::client` read executor | **Built**; pure response handling tested; HTTP send untested. |
| Product `sharepoint::writes::execute` — 8-step gate | **Built**, tested with fake transport / ledger / audit sink / bearer. |
| **Durable idempotency ledger + audit sink** | **Not built.** Only non-durable in-memory implementations exist. A real one needs a new entity under `.appfw/model` (generated surface — deliberately not touched, no edits made). |
| **Custom methods + handlers** (`upload_to_sharepoint`, `get_document_link`, `list_document_versions`) | **Not built.** Needs a model change (`mcp_enabled: false`). |
| **`DocumentStore` trait + `SharePointDocumentStore`** (design §4.3) | **Not built.** The `DocumentStore` trait does not exist in the repo yet; `services/sharepoint` is its future backing. |
| **Route/job wiring** (`create_temp_folder`, `process_document`, `finalize`) | **Not built** (Phase 2 per design §7). |
| Model columns replacing `box_file_url` (`storage_file_id`, `storage_file_url`) | **Not built** (design §5.2; `.appfw/model` untouched). |
| Rate-limit retry/backoff loop; paging of `children` | **Not built.** 429/503 are classified with `Retry-After`; nothing retries. |
| Tenant site/drive/role grant | **Gated on the admin** (handoff document). |
| Live run (token, read, upload, replay) | **Gated** on the grant. Nothing is `live_certified`. |
| `FrameworkProvider::SharePoint` / `DataSourceType` | **Gated on PDS.** Not patched. |

---

## 7. Config contract (additions only)

```dotenv
# backend/.env — git-ignored; real values come from the admin, never committed.
SHAREPOINT_SITE_ID=
SHAREPOINT_DRIVE_ID=
SHAREPOINT_AUTH_MODE=oauth_client_credentials
# shared Graph app registrations (D4)
GRAPH_TENANT_ID=
GRAPH_CLIENT_ID=
GRAPH_CLIENT_SECRET=
GRAPH_WRITE_CLIENT_ID=
GRAPH_WRITE_CLIENT_SECRET=
```

`SHAREPOINT_AUTH_MODE` set to anything other than `oauth_client_credentials` makes
SharePoint unconfigured (fail closed). Secrets are redacted in `Debug`/`Serialize`.

Body-size note: the GraphQL request body is capped by the framework runtime
(`APP_REQUEST_BODY_LIMIT_BYTES`, default 1 MiB per Project Governance's finding;
**[unverified]** here). Base64 inflates by ~33 %, so a 2 MB file will not fit the
default through GraphQL. How bytes reach the backend (single mutation with a raised
limit vs. the existing REST multipart upload noted in design §4.5) is open — §9.

---

## 8. Verification

| Check | Result |
|---|---|
| `cargo test -p appfw-provider-sharepoint` (in `app-framework/`) | **22 passed** (run under WSL Ubuntu; this Windows host has no MSVC linker). |
| `cargo test -p backend --bin backend services::` | **44 passed**, 0 failed (WSL Ubuntu). |
| `cargo check --workspace --all-targets` (`tax-doc-routing/`) | **Passed.** |
| `scripts/appfw product boundary-check --json` | **ok: true, 0 violations**, 61 files checked. |
| Live | **Not run.** |

---

## 9. Open items that need an answer

1. **2 MB vs real file sizes.** The source spec mentions batches of 3,000+ K-1s and
   scanned PDFs are routinely larger than 2 MB. D2 is the owner's decision; confirm
   it is intended for this product, because anything larger is rejected, not split.
2. **Folder scheme** (D5) against BS §14.
3. **How bytes reach the backend** (multipart route vs GraphQL mutation).
4. **Upload roles** (D8) and **content types** (D9).
5. Whether the **read/write app registrations already exist** (handoff §3).
6. The **durable ledger/audit** model change and who owns it.
7. Tenant limits, naming restrictions, recycle-bin retention, sensitivity-label policy
   (all admin-supplied).
8. **Download URL** lifetime for app-only tokens.
