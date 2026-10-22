# Pega Box folder logic → SharePoint: implementation plan

> **Status: plan only. No code or model change has been made for this. Target store is now Box first (see 0.3); SharePoint later.**
> Source: *Tax App — Box Folder Logic (End to End)* (Oct 9, 2026), the documented Pega process.
> Builds on: [`sharepoint-integration-plan.md`](sharepoint-integration-plan.md) (provider + upload gate already built, nothing live)
> and [`tax-document-routing-design.md`](tax-document-routing-design.md) §4.3 (`DocumentStore`), §1.3, §1.4.
> Verified facts are from those files; anything marked **[confirm]** is not yet known.

---

## 0. The flow in plain words (what we will rebuild, same as Pega)

1. **Create a request.** The app makes one **temporary folder named with the case id** (`TD-1001`) inside the client's **Internal** folder.
2. **Upload documents.** Each PDF is saved straight into that temp folder. Only PDFs are accepted.
3. **Finish / submit.** Now, and only now, the app builds the permanent folders, for each document using its **Year** and **Document Type**:
   - **Internal side** (client's Internal folder): `Year` → `<internal type folder>` (e.g. `2025/4. Tax Documents`), and also `Year` → `6. Released to Client` → `<type folder>` (e.g. `2025/6. Released to Client/K-1s`).
   - **External side** (client's Client folder): `Year` → `<client type folder>` (e.g. `2025/K-1s`).
   - Some types (S Corp Election, Form 8308, Extensions) have **no internal side**: only the client folder is built.
4. **Place the documents.** The PDF is copied from the temp folder into those folders (client copies password-protected if the user chose Yes). **The file is renamed on the way:** in the temp folder it keeps the name the user uploaded (`PegaAI.pdf`); in the Internal and External folders it becomes **`<Client Name> - <uploaded name>`** (`Moore, Andrea - PegaAI.pdf`).
5. **Clean up.** The temp folder `TD-1001` is deleted, then the client is emailed if Send Notification is Yes.

Worked example, K-1 for 2025, case TD-1001:

```
Before submit   Internal/TD-1001/K1.pdf
After submit    Internal/2025/4. Tax Documents/Moore, Andrea - K1.pdf
                Internal/2025/6. Released to Client/K-1s/Moore, Andrea - K1.pdf
                Client/2025/K-1s/Moore, Andrea - K1.pdf
                (TD-1001 deleted)
```

Everything else in this document is the detail behind these five steps. SharePoint is only the place the folders live; the steps do not change.

---

## 0.1 Client requirement (final) checked against our application

The client's end-to-end note (temp folder under the existing INTERNAL folder, upload on screen 2, validate and route only after submit, delete the temp folder only after every move succeeds) matches the Pega flow in section 0. I inspected the application against it. **Nothing below has been changed in code.**

### What already fits

| Requirement | Our application today |
|---|---|
| Request ID `TD-1001` | Done: `case_number`, assigned by the server, shown everywhere. |
| Document type to folder mapping (Pega decision tables) | Done and DB-driven: `document_type_routes` (9 rows) holds the internal and external folder names and the external-only flag; same mappings as the client table. |
| Year from the document row, not today's date | Done: `document_year` is chosen per document. |
| Failures go to a queue and can be retried | Done: `exception_task` (failed step, reason) and `retry_failed_step`. |
| "Send Notification?" choice | Stored on the record (`notification_flag`). Nothing sends the email yet. |

### Gaps found (what has to change)

| # | Gap | Detail | Fix in the plan |
|---|---|---|---|
| G1 | **The request is created too late.** | The routing record (and its case number) is only created when the user presses Submit on the documents screen. The client wants the temp folder `TD-1001` to exist **before** documents are uploaded on screen 2. | Create the record when the user continues from the client-info screen (status `CollectingInfo`, already in the status list), then create the temp folder. Needs a clean-up rule for abandoned drafts (record cancelled, empty temp folder deleted). |
| G2 | **Client folder IDs are not what the requirement says.** | Client reference data has `external_folder_id` (e.g. `CR-001234`, a placeholder) but the internal side is `internal_folder_path` (e.g. `/Clients/Smith_John`, a path, not an ID). The UI loads the internal path and `folder_name` and ignores `external_folder_id`. The requirement needs **both a Client Internal Folder ID and a Client External Folder ID**. | Hold `internal_folder_id` and `external_folder_id`, carried onto the record at case creation like the other client snapshot fields. Real values come from the client SharePoint (see Q-A). |
| G3 | **INTERNAL root ID is hardcoded in the note.** | `84473772615` is a Box-style numeric id. A SharePoint drive-item id will look different and differs per environment. | One setting, `INTERNAL_ROOT_FOLDER_ID` (config/env, per environment), never a literal in code. Same pattern as `SHAREPOINT_SITE_ID` and `SHAREPOINT_DRIVE_ID`. |
| G4 | **No real upload path.** | Screen 2 uses a mock upload (a timer, nothing stored), a mock AI step and "sample documents". The backend has no endpoint that receives a file. | New upload endpoint (multipart, PDF only) that stores into the temp folder and returns the SharePoint file ID. The mock services are replaced; AI prefill stays a suggestion only. |
| G5 | **Documents are not saved.** | `tax_documents` has 0 rows and nothing writes it. Existing columns cover year, name, entity, password flag, duplicate flag and attachments; missing: SharePoint file id/url (replaces `box_file_url`), file size, uploaded name, per-document status. | Persist one row per upload (file id + metadata), as the client step 5 asks. |
| G6 | **No per-destination outcome.** | "Record the outcome for each destination" and "retry without duplicates" need somewhere to keep it. | New small table `tax_document_placements` (document, destination = internal / released / external, folder id, file id, status, error). It also tells cleanup when it may delete. |
| G7 | **Processing is a UI animation.** | The 9 steps and their 1.1 s timers run in the browser; progress is only saved. Real work (folders, moves, cleanup) must be driven by the server, with the UI reading `progress_step`. | A server-side submit pipeline updates `progress_step`; the UI only reads it. Retry reuses `retry_failed_step`. |
| G8 | **No PDF protection service and no email service.** | Neither exists in the backend (only the flag and the design). | Both need building; email also needs the Pega recipients, CC list, template and trigger conditions (the client note marks these "to verify"). |
| G9 | **Type validation timing.** | Screen 2 currently requires type and year before Submit (fine as metadata). The note says routing validation must happen **after** submit, from the temp folder. | Keep collecting type and year as metadata; do **not** use them to pick folders at upload. Folder choice and the unmapped-type check run only in the post-submit step; an unmapped type becomes an exception, never a wrong folder. |

### Revised end-to-end plan (client terms)

1. **Continue from client info:** create the record (status `CollectingInfo`, `TD-1001`), then create folder `TD-1001` under the configured INTERNAL root; save the folder id on the record.
2. **Screen 2 upload:** PDF validated (type, name length, size, duplicate), saved into `TD-1001`; file id and metadata saved in `tax_documents`.
3. **Submit:** status moves to processing; the server pipeline starts.
4. **Read** the case documents from the DB and confirm they exist in the temp folder.
5. **Validate** each document type and year against `document_type_routes`; unmapped becomes an exception while the other documents continue.
6. **Create or reuse** `Year/<type folder>` under the client Internal folder ID; `Year/6. Released to Client/<type folder>` in the same Internal folder (skipped for external-only types); and `Year/<type folder>` under the client External folder ID.
7. **Place** each document (renamed `<Client Name> - <uploaded name>`, client copies protected if flagged) and record each destination in `tax_document_placements`.
8. **Verify** every required destination succeeded and save statuses; only then **delete** `TD-1001`; otherwise keep it and raise an exception.
9. **Notify** if Send Notification = Yes.

### Questions for the client (only these block development)

| # | Question |
|---|---|
| Q-A | For each client, what are the SharePoint **Internal and External folder identifiers** (drive-item ids, or paths we resolve)? Are they supplied as a list we load into the client reference data? |
| Q-B | The **INTERNAL root folder** in SharePoint: its id, site and library (drive). Is every client folder in the same library? |
| Q-C | Does each destination get its **own copy**, or is one file moved (Pega reference unclear)? |
| Q-D | **Maximum file size** (our current ceiling is 2 MB; K-1s and scans are often larger). |
| Q-E | **Virus scanning**: what replaces the Pega scan? |
| Q-F | **Email** recipients, CC list, template and trigger conditions from Pega. |
| Q-G | **Permissions** the app identity gets: create folder, upload, read, delete (temp folder); and whether versioning and recycle bin are on. |
| Q-H | Name format: client name as `Last, First` (as seen in Pega)? |

### What we can do before the client answers

Phases A to F in section 5 need none of these answers (storage interface, in-memory store, routing rules, persistence, upload endpoint, pipeline, config switch). Only the live connection needs the SharePoint details and credentials.

---

## 0.2 Answers received (product owner, from the client question list)

Still design only. These settle most open items; what remains is listed after the table.

| # | Topic | Answer | Effect on the design |
|---|---|---|---|
| 1 | Client Internal / External folder IDs | They come from the client record the user selects on the first screen; client details are read from **our database table**. | Confirmed: IDs live in the client reference table (`tax_payer_ref_data`) and are copied onto the case at creation. The table must hold **real SharePoint ids** (gap G2); we load them when the client supplies them. Nothing is typed by the user. |
| 2 | INTERNAL root `84473772615` | "Yes" (exists). Site and library not stated. | Kept as one configured setting (`INTERNAL_ROOT_FOLDER_ID`). Site id and drive id are still needed. |
| 3 | Copy or move | **Copy** to every required destination (Client Internal, Client External, `6. Released to Client`). Temp folder deleted **only after all** destination operations succeed. | Decided (closes O4/Q-C). The temp file stays until every placement is confirmed in `tax_document_placements`; no "move". |
| 4 | Max file size | **Keep 2 MB** for now. | Decided (closes O2/Q-D). Stays a setting (`max_upload_bytes`) so raising it later is a config change, not code. Files above it are rejected with a clear message. |
| 5 | Virus scan | "Yes" (a scan is required). Solution not named. | Scan step is **in scope**, placed between receiving the bytes and writing to the temp folder; infected or unscannable files are rejected and never stored. The scanner itself is **still open** (see below). |
| 6 | Email | **To:** client personal email + PDS email. **CC:** additional email. Reuse the existing Pega template. Sent when the case is created successfully. | Recipients map to fields already on the record (`personal_email`, `pds_email`, `additional_email`), sent only when `notification_flag` = Yes. Template text and the exact trigger moment need confirming (see below). |
| 8 | File name | **`Last Name, First Name - OriginalFileName.pdf`** | Decided (closes O1/Q-H). Same rule as the earlier decision. |

### Still open before go-live

| # | Item | Why it matters |
|---|---|---|
| R1 | **SharePoint site id, document library (drive) id**, and whether all clients share one library. | Needed to address every folder. |
| R2 | **Folder ids for each client** (Internal and External) to load into the client table, in the SharePoint id format (not Box ids). | Without them no folder can be addressed. |
| R3 | **Authentication and permissions** (answer to question 7 was not given). Current plan: Entra app registration with client-credentials; needs create folder, upload, read, copy, delete on the one library. Versioning and recycle bin status. | Gates the live connection. |
| R4 | **Virus-scan solution** (Microsoft Defender for SharePoint, a scanning API, or other). | Can be built behind an interface now; the product choice decides the integration. |
| R5 | **Email trigger moment.** "Once the case is created successfully" is read as: after all documents are placed and the temp folder is deleted (Pega: `SendFinalNotification` is the last phase). Please confirm it is not meant to fire when the request is first created. | Wrong timing would email clients before documents are filed. |
| R6 | **Email template**: the exact Pega subject and body, and sender address. | Needed to build the same mail. |
| R7 | **Same file name in all three folders?** Assumed yes (as Pega). | Minor. |

Cleaned-up flow, unchanged from section 0.1 except where answers fix it: create record and `TD-1001` folder on continue; upload PDFs (2 MB, scanned) into it; on submit validate type/year, create or reuse folders, **copy** each file to the three destinations under the name `Last, First - OriginalFileName.pdf`; verify; delete `TD-1001`; email (To personal + PDS, CC additional) if Send Notification is Yes.

---

## 0.3 Decision (2026-10-18): integrate with Box first, SharePoint later

The client has asked for **Box** for now. The Tax Application itself is finished first; the Box integration starts after that. Nothing in this document is built yet.

**Why this is cheap to absorb.** Everything above the storage boundary is vendor-neutral by design (section 3.1): the folder rules, the upload checks, the submit pipeline, the filing plan and the per-destination records. Only the implementation behind `DocumentStore` changes. Pega was written against Box, so the flow in section 0 is already Box-native.

| Item | Was (SharePoint) | Now (Box) |
|---|---|---|
| Store implementation | `SharePointDocumentStore` over Microsoft Graph | `BoxDocumentStore` over the Box API |
| INTERNAL root | SharePoint drive-item id (unknown) | The Box folder id already given: `84473772615` (still a setting, not hardcoded) |
| Client folder ids | SharePoint ids (unknown format) | Box folder ids, as Pega already holds them |
| Site and library ids | Needed | **Not needed** (Box folder ids are global) |
| Auth | Entra app registration | Box app: server-side auth (Client Credentials or JWT) with the app's service account. Pega used a token generator for this. |
| Existing code | SharePoint provider built, unwired | Not reusable for Box; the neutral layer above it is |
| Config switch | `STORAGE_PROVIDER=sharepoint` | `STORAGE_PROVIDER=box` (memory stays the default); `sharepoint` stays an option for later |

**Box mapping for the five store operations** (to confirm against Box documentation when building):

| Store operation | Box call |
|---|---|
| `ensure_folder` | Create folder (name + parent id); on a name conflict Box reports the existing folder, which we reuse |
| `upload_file` | Upload file into a parent folder (multipart); a same-name conflict is refused, never overwritten |
| `find_file` | List the folder's items and match by name |
| `read_file` | Download file content |
| `delete_folder` | Delete folder (recursive) for the case temp folder only |

**Box-specific points the plan already anticipated**
- **Deleting the temp folder (open item O6).** Pega "grants collaborator access, waits, then deletes". That is a Box permission rule: the service account must have enough rights on the folder to delete it. Confirm with the client how the app identity is set up.
- **Virus scan.** Pega calls a scan step before saving. Box has its own optional scanning; whether the client uses it, or wants the same Pega scan, still needs an answer.
- **Folder name limits.** Box rejects some characters and over-long names; the same name sanitising applies.
- **Copies, not moves**, 2 MB limit, file name `Last, First - OriginalFileName.pdf`, and the email rules are unchanged.

**What we need from the client for Box**
1. The Box app credentials for server-side access (client id, client secret, enterprise or user id, and the auth type they use), supplied securely, never in chat or code.
2. Confirmation that the app's account can create folders, upload, read, copy and delete under INTERNAL and under each client's Internal and External folders.
3. Each client's Internal and External Box folder ids in the client table (the current sample values are placeholders).
4. The virus-scan choice and the email template and trigger (unchanged open items).

**Order of work now:** finish the Tax Application first. The parts of the plan that do not depend on the store (saving documents to the database, creating the request earlier, the server-side submit pipeline against the in-memory store) can be built whenever you say. The Box connection comes last, as `BoxDocumentStore`.

---

## 0.4 Box layer built (2026-10-18)

Built and unit-tested, **not connected to any screen**; the running app still uses the in-memory store (`STORAGE_PROVIDER` unset). No Box call has been made yet.

| Piece | Where |
|---|---|
| Storage boundary, folder rules (Pega decision tables), in-memory store, Pega workflow steps | `backend/src/services/document_store/{mod,memory,routing,workflow}.rs` |
| Box store: create folder (409 means reuse), upload, find, read, delete | `.../box_store/store.rs` |
| Box sign-in (JWT, RS256) with the PDSBOX key store | `.../box_store/auth.rs` |
| Token lifecycle: save with expiry, reuse, renew, replace in the same row | `.../box_store/token_manager.rs`, `pg_token_store.rs` |
| Token table `tax_routing.storage_tokens` (one row per provider) | model entity `StorageToken` + `rbac/storage_token.rego` |
| Settings | `BOX_CLIENT_ID`, `BOX_CLIENT_SECRET`, `BOX_ENTERPRISE_ID`, `BOX_PUBLIC_KEY_ID`, `BOX_KEYSTORE_PATH`, `BOX_KEYSTORE_PASSWORD`; switch `STORAGE_PROVIDER=box` |

**Token rules (as Pega does it):** sign in with the key store, save the token and its expiry in the table, use the latest valid token for every Box call, and when it has expired sign in again and replace it. The expiry is read from Box's `expires_in` (the client said 3 hours), not hardcoded. The token is renewed one minute before it expires, or at once on a 401. A restart or second server reuses a still-valid stored token.

**Token table safety:** no permission exists for `storage_token`, so the generated API denies every role (checked: admin and staff both get access denied); only the backend's own SQL reads it; no audit copies; the column is marked for redaction. The token is stored in plain text like Pega's table: it is short-lived, but the database must be access-controlled.

**Verified:** 29 new unit tests (including the Pega K-1 2025 worked example running on the Box store against a fake Box, the 409 folder-reuse path, paging, and every token-renewal case) and the table SQL against the real database. **Not verified:** anything against the real Box (sign-in with the key store, folder and file calls). Two `#[ignore]` live tests exist for that; the user runs them locally so secrets stay in their terminal.

**Still to build** (per scope, after the Tax Application work): saving documents to the database, creating the request at Continue, the upload endpoint, the server-side submit pipeline, PDF protection with the client's Read Write Password, email, and connecting the store to those screens.

---

## 1. What the Pega process does (understood)

Five phases. Box is the only store; the case id is the temp-folder name.

| Phase | When | What happens in Box |
|---|---|---|
| **1 Create + upload** | Case start, then each upload | `CreateTempBoxFolder` makes a temp folder named with the case id (e.g. `TD-2797`) inside the client's **Internal root**. Each PDF is type-checked (pdf only), virus-checked, then saved straight into the temp folder; Box file id is kept on the attachment. Validation: name ≤ 256 chars and contains `.pdf`, document attached, duplicate check. |
| **2 Submit → folders** | After Submit | For **each document row**: Internal `Year` → `Year/<internal type folder>`; Client `Year` → `Year/<client type folder>`. Then under the internal year: `6. Released to Client` → `<client type folder>`. |
| **3 Place files** | PDF processing | Check for same-named file; apply password protection if "Password Protect Client Copies" = Yes; write the PDF to the internal, internal-quick-view and external destinations; mark each file moved; retry unprocessed files. |
| **4 Clean-up** | After all moved | Delete the temp folder. (Pega first grants itself collaborator access and waits — a Box permission quirk.) |
| **5 Notify** | Last | Email the taxpayer if Send Notification = Yes. |

**Folder name sources** (each has exactly one):

| Folder | Source |
|---|---|
| Temp folder | Case id (`TD-xxxx`) |
| Year folder (both sides) | Year chosen on the upload row |
| Internal type folder | Decision table `GetInternalFolderName` |
| Client type folder | Decision table `GetExternalFolderName` |
| `6. Released to Client` | **Hardcoded text** in the Pega activity |
| Type folder under Released | `GetExternalFolderName` reused |

**Routing table** (same as our `document_type_routes` seeds):

| Type | Internal folder | Client folder | Internal side built? |
|---|---|---|---|
| K-1, Draft K-1 | `4. Tax Documents` | `K-1s` | yes |
| Tax Return | `7. Tax Returns` | `Tax Returns` | yes |
| Income Statement, Balance Sheet | `Loan Assistance` | `Loan Assistance` | yes |
| S Corp Election | — | `Signature Requests` | no (external only) |
| Form 8308 | — | `K-1s` | no |
| Extensions | — | `Tax Returns` | no |

Unknown type → no match → no folder (Pega silently skips; we should **fail loudly** instead).

---

## 2. What we already have vs what is missing

| Need | Today in this repo |
|---|---|
| Folder/destination rules | **Done and DB-driven**: `document_type_routes` (9 rows: `internal_folder`, `external_folder`, `is_external_only`, `supports_password_protection`). Rules read it via the catalogue (changed this session). |
| Case id for the temp folder | **Done**: `case_number` → `TD-1001` (this session). Matches Pega's `TD-2797` convention. |
| SharePoint provider, auth, 2 MB gate, single upload op | **Built, unit-tested, nothing live** (`services/sharepoint`, `appfw_provider_sharepoint`). |
| `DocumentStore` trait | **Not built** (design §4.3 only). |
| Create folder, move/copy, delete folder, exists/list | **Not built.** The existing gate deliberately has *no* move/delete/rename (D7). |
| Persisting documents | **Not built**: `tax_documents` has 0 rows; nothing writes it. Column `box_file_url` still Box-named. |
| Where bytes enter the backend | **Open** (multipart route vs GraphQL; 1 MiB body default). |
| Temp folder id on the record, per-document storage ids and moved flags | **Not modelled.** |
| Client root folders (Internal / Client) | `tax_payer_ref_data` has `internal_folder` / `folder_name` as Box-era names; need SharePoint item ids/paths. |
| PDF protection, notifications, virus scan | Protection/notify exist as services per design §4.4/4.7 **[confirm status]**; virus scan has no equivalent. |

---

## 3. Design

### 3.1 One neutral boundary: `DocumentStore`

Everything above the trait knows folders by **opaque `FolderRef`/`FileRef`**, never Box or SharePoint (CLAUDE.md rule).

```
trait DocumentStore {
  ensure_folder(parent: FolderRef, name) -> FolderRef      // idempotent: get-or-create
  upload_file(parent: FolderRef, name, bytes, mime) -> FileRef   // fails if exists
  find_file(parent: FolderRef, name) -> Option<FileRef>     // the "same name exists" check
  copy_file(file: FileRef, to: FolderRef, new_name) -> FileRef
  move_file(file: FileRef, to: FolderRef, new_name) -> FileRef
  read_file(file: FileRef) -> bytes                         // needed for PDF protection
  delete_folder(folder: FolderRef)                          // temp folder only
  download_link(file: FileRef) -> short-lived URL           // UI "open document"
}
```

Implementations, chosen by config (§3.5):
- `InMemoryDocumentStore` — dev, tests, demo. **The whole workflow runs end to end on this today.**
- `SharePointDocumentStore` — Graph, drive-scoped; reuses the built auth + request-plan + redaction code.

`ensure_folder` being get-or-create is what removes Pega's "four calls per row" duplication risk: ten K-1 rows for 2025 produce one `2025` folder.

### 3.2 Routing engine (pure, no I/O) — `services/document_routing`

Replaces Pega's decision tables + `CreateTargetFolders`. Input: documents + the DB catalogue + client roots. Output: a **filing plan** — the exact folder paths and file names — computed before anything is touched.

```
plan_for(document, catalogue, roots) -> FilingPlan {
  year,
  internal:  [Year, <internal_folder>]                    // skipped when is_external_only
  released:  [Year, <RELEASED_TO_CLIENT>, <external_folder>]   // skipped when is_external_only
  external:  [Year, <external_folder>]
  file_name: filenameRules(...)
  encrypt:   per destination
}
```

- Looks up by **document type code** against `document_type_routes` (no `toJavaIdentifier` key needed; the code is the key). Unknown type → typed error → exception queue.
- `Released to Client` becomes a **configurable value** (setting / option list), not a literal — Pega says it requires IT to change; here it is data.
- The same plan feeds the **Filing Plan preview** the UI already shows, so preview and reality cannot drift.
- **File naming (decided):** `final_name = "<Last>, <First> - <uploaded file name>"`, applied only at the move from temp to the destinations; the temp copy keeps the uploaded name. Today's `buildFileName` (BS §14.11 form with entity, year, type and `(SECURED)`) is **replaced** by this rule, in the Filing Plan preview, the Completed screen and the placement step. Name parts are sanitised for SharePoint-illegal characters; a clash in a folder is handled by the name-exists check (see §3.3).

### 3.3 Workflow (maps to existing stages + `exception_task`)

| Our stage / step | Pega step | Action |
|---|---|---|
| **Create record** (`create_impl`) | `CreateTempBoxFolder` | `ensure_folder(internal_root, "TD-1001")` → store `temp_folder_ref` on the record. If storage is down: record is **not** left half-made; surface as a storage error before the case opens. |
| **Upload document** | `pyUploadFile`…`SaveToBoxStorage` | Validate (PDF, ≤ configured size, name rules, duplicate BR-05) → `upload_file(temp, name, bytes)` → insert `tax_documents` row with `storage_file_ref`. |
| **Submit** → PDF processing | `CreateTargetFolders`, `CreateFolderReleasedToClient` | Build filing plan; `ensure_folder` chain per document; persist resulting refs. |
| Placing files | `PDFFileProcessing`, `MoveFile_*_EP/NP` | For each document: `find_file` (duplicate guard) → `read_file` → protect if flagged → `upload_file`/`copy_file` to each destination → set `is_file_moved`. |
| Clean temp | `DeleteTempFolder` | Only when **every** document `is_file_moved`; `delete_folder(temp)`. No collaborator/wait dance — that is a Box permission workaround (see O6). |
| Notify | `SendFinalNotification` | Existing notification service, if `notification_flag`. |

**Idempotency and retry** — every step is safe to repeat (get-or-create folders, name-exists check before write, per-document moved flag), so `RetryFailedStep` and the Exception Queue re-run only what failed. This replaces Pega's `RetryUnprocessedFile`. The processing animation in the UI keeps working because it reads `progress_step`, which the server drives.

### 3.4 Data model changes (coordinated generate; **not made yet**)

| Entity | Change |
|---|---|
| `TaxRoutingRecord` | `+ temp_folder_ref` (String, nullable). |
| `TaxDocument` | rename `box_file_url` → `storage_file_ref` (+ `storage_file_url`); `+ is_file_moved` (Boolean); `+ file_size`, `+ content_sha256`, `+ storage_name`; `+ status`. Wire the existing columns (`document_year`, `document_name`, `entity_*`, `is_password_protected`, attachments) — they exist, nothing writes them. |
| `TaxPayerRefData` | `internal_folder` / `folder_name` → hold storage refs (item id or drive path) instead of Box names **[confirm form, O3]**. |
| Settings / option list | `released_to_client_folder_name`, `max_upload_bytes`, `allowed_file_types` (`pdf`) — admin-editable, seeded. |

This also closes the earlier gap: documents become persisted and reloadable on Record Details.

### 3.5 Configuration — "connect later with minimal change"

```
STORAGE_PROVIDER=memory            # memory | sharepoint   (default memory)
SHAREPOINT_SITE_ID=                # from the client admin
SHAREPOINT_DRIVE_ID=
SHAREPOINT_AUTH_MODE=oauth_client_credentials
GRAPH_TENANT_ID= GRAPH_CLIENT_ID= GRAPH_CLIENT_SECRET=
GRAPH_WRITE_CLIENT_ID= GRAPH_WRITE_CLIENT_SECRET=
```

The factory returns `SharePointDocumentStore` only when `STORAGE_PROVIDER=sharepoint` **and** site/drive/credentials are present; otherwise memory (or a clear startup error in non-local envs — **fail closed**, never silently pretend). Going live = set env vars + load client root refs + run the live checks in §5. No workflow code changes.

### 3.6 Graph mapping (for the `SharePointDocumentStore`)

All per `https://graph.microsoft.com/v1.0/drives/{drive-id}` (Microsoft Graph DriveItem API; confirm in live run):

| Store op | Graph call |
|---|---|
| `ensure_folder` | `GET …/items/{parent}/children?$filter`/list-then-match, else `POST …/items/{parent}/children` `{ "name", "folder": {}, "@microsoft.graph.conflictBehavior": "fail" }`; on 409 re-read. |
| `upload_file` | `PUT …/items/{parent}:/{name}:/content` (simple upload; 2 MB product ceiling is below Graph's simple-upload limit). `conflictBehavior=fail`. |
| `find_file` | `GET …/items/{parent}:/{name}` (404 = absent). |
| `copy_file` | `POST …/items/{id}/copy` (async, returns monitor URL) — or read+upload for small PDFs to stay synchronous. |
| `move_file` | `PATCH …/items/{id}` with `parentReference.id` (+ `name`). |
| `read_file` | `GET …/items/{id}/content`. |
| `delete_folder` | `DELETE …/items/{id}`; recycle-bin retention is tenant-level. |
| `download_link` | `GET …/items/{id}` → `@microsoft.graph.downloadUrl` (short-lived; audit each issue, existing plan §4). |

Illegal characters in SharePoint names (`" * : < > ? / \ |`, trailing dots, `#`/`%` handling) — the existing `sanitize_file_name` is reused for **every** generated name, including client names in file names.

---

## 4. Changes to decisions already recorded

The existing SharePoint plan was written before this Pega logic and conflicts in places. These need the owner's explicit sign-off:

| Existing | Conflict | Proposed |
|---|---|---|
| **D7: "No delete, rename, move, replace"**, one write op | Pega creates folders, moves/copies files, deletes the temp folder | Add three separately-scoped ops: `create_folder`, `copy/move`, `delete_temp_folder` (only items under the case's temp folder; never client folders). Keep `conflictBehavior=fail`; keep "no replace". |
| **D5 path scheme** `{entity}/{doc_id}__{name}` | Pega's layout is `Year/<type>/<file>` under per-client roots | Replace by the filing plan in §3.2 (Pega layout is the business requirement). |
| **D3 one site/one drive** | Likely fine if all clients live in one library **[confirm O3]** | Keep. |
| **D2 2 MB cap** | Real K-1 PDFs / scans often exceed this | Make `max_upload_bytes` a setting; large-file path needs an upload session (O2). |
| Plan names `box_file_url` rename | Matches §3.4 | Proceed. |

---

## 5. Delivery phases

| # | Deliverable | Needs client? | Result |
|---|---|---|---|
| **A** | `DocumentStore` trait + `InMemoryDocumentStore`; routing engine + unit tests (all 9 types, external-only, unknown type, dedupe) | No | Testable, deterministic. |
| **B** | Model change (§3.4) → generate → migrate; persist `TaxDocument` rows; record `temp_folder_ref` | No | Documents survive refresh. |
| **C** | Upload endpoint (multipart) + validation (PDF, size, name, duplicate) → store; create-time temp folder | No | Real upload path on the memory store. |
| **D** | Submit pipeline: folders → place files → clean temp → notify, idempotent + Exception Queue retry | No | Complete flow on memory store; UI progress driven by the server. |
| **E** | `SharePointDocumentStore` against a **fake Graph transport** (extend existing gate tests) | No | Graph calls/paths proven without a tenant. |
| **F** | Config switch + startup validation; admin handoff updated for the extra permissions | No | Ready to connect. |
| **G** | Live connect: env vars, client root refs, live checks (create folder, upload, copy, delete, download link, throttling) | **Yes** | Go-live. |

A–F need nothing from the client. G is the only credential-dependent step.

---

## 6. Open items

| # | Question | Why it matters |
|---|---|---|
| **O1** ~~Final file name~~ **Decided by product owner:** `<Client Name> - <uploaded name>` (e.g. `Moore, Andrea - PegaAI.pdf`), applied when moving from temp to Internal and External. Remaining detail: confirm client name is `Last, First` from the client record (Pega observed), not the longer Client Name field (`Andrea Moore, DDS INC.`), and what happens when two documents share an uploaded name in the same folder (suggest: fail with a clear message, as the duplicate check BR-05 already blocks same type + year + name). | Naming rule fixed; two edge cases left. |
| **O2** | Max file size and large-file handling (2 MB decision vs reality); upload sessions needed above ~4 MB per Graph guidance. | Rejects real documents. |
| **O3** | How client roots are identified in SharePoint (item id vs path; one library for all clients?). `tax_payer_ref_data` folder columns are Box-era. | Core addressing. |
| **O4** | Does each destination get a **copy** (Pega doc: "to confirm whether the other two get a copy or the same file is moved once"). BS §1.3 / design §1.3 say three copies. | Storage and consistency. |
| **O5** | Virus scanning: Pega calls an AV service. What replaces it (Defender for SharePoint, ICAP, none)? | Security requirement. |
| **O6** | Temp-folder deletion: Box needed collaborator grant + wait. Confirm the app-only Graph identity can delete without extra grants. | Clean-up reliability. |
| **O7** | Which types in `document_type_routes` need an Internal Quick View copy distinct from Internal Client Copy (Pega has three move activities). | Destination count. |
| **O8** | Submit failures: all-or-nothing vs per-document partial success (Pega marks per-file `IsFileMoved`). Plan assumes per-document + retry. | Exception behaviour. |
| **O9** | Notification content/recipients and what exists today. | Phase 5. |
| **O10** | Who owns tenant setup: site/drive ids, app-registration grants (create/delete scopes), versioning, retention. | Gates phase G. |
