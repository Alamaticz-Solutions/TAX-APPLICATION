# Bulk Request Creation: design document

Status: **design only, nothing built.** Date: 2026-10-20. Product: Tax Document Routing (`tax-doc-routing`).
Audience: product owner, tax team, developers. Decisions still needed from the client are in section 3.

## 1. Purpose

Today a user creates one request at a time: pick a client on screen 1 (details fill in from the client
directory), add that client's documents on screen 2, submit. The server then files the documents in Box.

Bulk Request Creation lets a user upload **one ZIP** that holds the documents of **many clients** and create
**one request per client** in a single operation. Every request is filed by the same pipeline and rules as a
single request.

**One request = one client + all of that client's documents.** Ten clients in the ZIP give ten requests.

## 2. Scope

In scope:
- ZIP upload, client identification by Client Key, document classification, preview and validation,
  batch creation, filing to Box through the existing pipeline, batch progress screen, retry.
- A unique **Client Key** on the client directory, and an import of the client Excel sheet into it (section 8).

Out of scope (for this feature):
- Virus scanning and the client email (both are not built for single requests either; bulk inherits them when they are).
- SharePoint (the storage boundary already isolates this).
- Changing the filing rules (`document_type_routes`), password protection, or the exception-task model.

## 3. Decisions needed from the client

| # | Question | Recommendation |
|---|---|---|
| D1 | What is the Client Key (Oracle ID, client number, new key)? | Use an existing stable business id. It must be unique, never reused, and present on every client. |
| D2 | How does a PDF tell us its type and year? | Filename convention first, optional Excel document sheet second, AI last, with anything uncertain marked "needs review". |
| D3 | Subfolders inside a client folder? | Reject (one level only) so the grouping stays unambiguous. |
| D4 | A client already has an open request? | Warn in the preview, allow the user to proceed or drop that client. |
| D5 | One invalid client: block the batch or skip it? | Skip it, report it, process the rest (user confirms on the preview). |
| D6 | Largest batch (clients, files, ZIP size)? | Design for 200 clients, 1000 files, 500 MB; confirm with the client. |
| D7 | Who may run a bulk upload? | `tax_admin` and `tax_staff` (staff: only their own batches), as for single requests. |

## 4. Current system (what bulk reuses)

- **Client directory:** `tax_routing.tax_payer_ref_data`: name, Internal folder id (`internal_folder_path`), Client folder id (`folder_name`), read-write password. Keyed by an internal UUID today.
- **Create:** `create_impl` in `handlers/tax_routing/tax_routing_record.rs` gives the record the next case number (`TD-xxxx`) and creates the temp folder `TD-xxxx` inside `STORAGE_TEMP_PARENT_FOLDER` (create-only; numbers already taken in Box are skipped).
- **Upload:** `document_routing::upload_document` stores a PDF in the temp folder after checking type, size (2 MB), name and duplicates.
- **Submit:** `document_routing::submit_request` saves one `tax_document` row per file and starts the background pipeline.
- **Pipeline (steps 4-9):** verify the client's own Internal and Client folders in Box; read each file and password-protect the client copy; create or reuse `Year/<type>` folders and copy to every destination; delete the temp folder; (email, not built); mark resolved. Failures open an exception task; each step is idempotent so Retry is safe.
- **Classification:** `mockAIService.analyzeDocument` reads the file name (a mock; a real model is a later step).
- **Storage boundary:** `DocumentStore` trait (Box today, SharePoint later).

Bulk adds no new filing logic. It adds a front door (ZIP, preview, batch) in front of the same create, upload and submit.

## 5. Requirements

Functional
- F1 A user uploads one ZIP; each top-level folder is named with a Client Key.
- F2 The app finds each client by key and shows the client's details (as screen 1 does).
- F3 Each PDF is classified to a document type, year and entity (and password-protect default from the client).
- F4 A preview shows every client, its documents, errors and the intended Box destinations. **Nothing is created in the database or in Box before the user confirms.**
- F5 On confirm, one request per valid client is created and its documents are filed by the existing pipeline.
- F6 A batch progress screen shows each request's status; failures can be retried individually.
- F7 Re-confirming or retrying never creates duplicate cases, files or folders.

Non-functional
- N1 One client's failure never blocks other clients.
- N2 The browser can be closed after confirm; processing continues on the server.
- N3 Box rate limits are respected (throttled concurrency).
- N4 Same authentication, role and row-level rules as single requests; full audit of who created what.
- N5 No secrets or client documents in logs.

## 6. End-to-end flow

```
User              React app                     Backend (Rust/Axum)                    PostgreSQL        Box
 |  pick ZIP         |                                  |                                  |               |
 |------------------>| POST /bulk/uploads (multipart)   |                                  |               |
 |                   |--------------------------------->| stage ZIP, create bulk_batch     |-------------->|
 |                   |                                  | (status=staged)                  |               |
 |                   |                                  | parse folders -> Client Keys     |               |
 |                   |                                  | look up clients                  |<--------------|
 |                   |                                  | classify each PDF                |               |
 |                   |                                  | validate (incl. folder exists) --|------------------------->| GET folder (read only)
 |                   |<---------------------------------| batch + items + errors (preview) |               |
 |  review / fix     | PATCH item (type/year/entity)    |                                  |               |
 |  confirm          | POST confirm                     |                                  |               |
 |------------------>|--------------------------------->| status=processing, return at once|               |
 |                   |                                  | worker: per client, throttled:   |               |
 |                   |                                  |  1 create record + TD-xxxx temp -|---------------|--> create temp folder
 |                   |                                  |  2 upload PDFs from staged ZIP --|---------------|--> upload files
 |                   |                                  |  3 submit (existing pipeline) ---|-> file to Internal/Client folders, delete temp
 |  watch progress   | poll batch status                |  4 write item result             |-------------->|
 |<------------------|<---------------------------------|                                  |               |
```

### 6.1 Stage and parse (no side effects)
1. The browser sends the ZIP as a multipart upload (not base64-in-GraphQL, which caps files at about 2 MB).
2. The server writes it to a **staging area** and records a `bulk_batch` with status `staged`.
3. It reads the entry names. Rules: top-level folder = Client Key; files directly in the root, nested subfolders, hidden/system files (`__MACOSX`, `.DS_Store`), and non-PDFs are reported, not silently ignored.
4. ZIP safety: reject path traversal (`..`, absolute paths), enforce max entries and max uncompressed size (zip-bomb guard), and verify each PDF starts with `%PDF`.

### 6.2 Identify clients
Each folder name is matched, exactly and case-insensitively, to `tax_payer_ref_data.client_key` among active clients.
An unknown or inactive key makes that item `error: client not found`. Two folders with the same key (differing only
by case) are merged or rejected (decision D3 style: reject).

### 6.3 Classify documents
Per PDF, in order:
1. **Filename convention** `<Type>_<Year>[_<Entity hint>].pdf`, for example `K-1_2025_Oakwood.pdf`. The type is matched to `document_type_routes` (alias-tolerant: `Tax-Return`, `Tax Return`), the year must be a valid year.
2. **Optional Excel document sheet** (`documents.xlsx` at the ZIP root): rows of `Client Key, File Name, Document Type, Year, Entity Type, Entity Name, Password Protect`. A row overrides the filename.
3. **AI classification** for anything still unknown (the existing flow; confidence below 60 % means unclassified).

The entity hint is only a hint: it is matched to the entity directory (`useEntityDirectory` data) and an
ambiguous or missing match is `needs review`. Each document ends as `ready`, `needs review` (AI or
ambiguous) or `error` (cannot be classified, or invalid type/year).
Password-protect defaults to the client's password-on-file and the type's `supports_password_protection`, as on screen 2.

### 6.4 Validate (the preview's red flags)
See the full catalogue in section 11. Folder checks call Box **read-only** (`folder_exists`); nothing is created.

### 6.5 Preview and fix
The user sees each client with details and documents; can correct a document's type, year, entity and
password flag (the existing Review dialog); can drop a document or a whole client; and sees an
estimate "N requests, M documents". Confirm is enabled when at least one client is valid.
Corrections are stored on the item, so a page reload keeps them.

### 6.6 Confirm and process
Confirm freezes the batch (`processing`) and returns immediately. A background worker processes items with a
small concurrency limit (default 3). **Per client** it reuses the single-request code in this order:
1. create the routing record (client details copied exactly as screen 1 does: client reference, assigned user = the uploader, notify flag, additional email) and its `TD-xxxx` temp folder;
2. upload each PDF from the staged ZIP into that temp folder;
3. save the document rows and start the existing pipeline (steps 4-9);
4. store the outcome on the batch item (case number, status).
The batch finishes when every item is done, failed or skipped.

### 6.7 Progress and retry
The batch screen polls the batch. Each row shows the client, the case number (link to the case), the step and
status. A failed request has an exception task as today; Retry on the case, or **Retry failed** on the batch,
re-runs only what failed (section 12).

## 7. Data model

New entities (model YAML, generated, with RBAC rego like the others):

`bulk_batch`
| Field | Notes |
|---|---|
| id, created_at, created_by (user), tenant | standard |
| file_name, file_size | the uploaded ZIP |
| status | `staged`, `processing`, `completed`, `completed_with_errors`, `cancelled`, `expired` |
| client_count, document_count, created_count, failed_count, skipped_count | counters |
| staging_ref | where the ZIP is staged |
| confirmed_at, finished_at | |

`bulk_batch_item` (one per client folder)
| Field | Notes |
|---|---|
| id, batch_id | |
| client_key, client_id (nullable until matched) | |
| status | `ready`, `needs_review`, `error`, `skipped`, `queued`, `processing`, `created`, `failed` |
| errors (json list) | the preview/processing messages |
| routing_record_id (nullable), case_number | set when created |
| documents (json) | per file: staged path, name, size, type, year, entity, password flag, source of classification (`filename`, `excel`, `ai`, `manual`), confidence, status |
| idempotency_key | `batch_id:client_key`, unique |

Changes to existing entities:
- `tax_payer_ref_data`: add `client_key` (string, **unique**, indexed, required for active clients).
- `tax_routing_record`: add `bulk_batch_id` (nullable) so lists can filter and a case can link back.

Migrations are additive; existing rows and flows are unaffected.

## 8. Client directory and Client Key

- The **database is the source of truth** for client details. The bulk ZIP never carries client details.
- A separate **client import** (Excel sheet: Client Key, name, Internal Folder ID, External Folder ID, other fields) creates or updates directory records, keyed by Client Key. It is a different screen and permission (`tax_admin`) with its own preview ("12 new, 3 updated, 2 errors"). Folder ids are validated against Box on import.
- Until the import exists, an admin can populate `client_key` by script for the clients that matter (Andrea Moore and Baskaran Se in test).

## 9. API surface

Large upload is multipart REST; everything else follows the existing GraphQL pattern.

| Operation | Style | Purpose |
|---|---|---|
| `POST /api/bulk/uploads` | multipart REST | Stage the ZIP, return the batch id (parse runs on the server) |
| `bulk_batch` (query) | GraphQL read | Batch, items, errors (preview and progress) |
| `bulk_update_item` | custom mutation | Correct a document's classification, drop a document or client |
| `bulk_confirm` | custom mutation | Freeze and start processing |
| `bulk_cancel` | custom mutation | Cancel before processing; discards the staged ZIP |
| `bulk_retry_failed` | custom mutation | Re-run failed items |

All calls check the caller, the permission and that the batch belongs to them (staff) or they are admin.

## 10. Screens (PDS components; follow the frontend guard rules)

1. **New Bulk Request** (entry: a second action beside "New Request"): file drop for the ZIP, short rules, link to the sample, progress while staging and parsing.
2. **Review** (guided-task floor plan): a summary strip (clients, documents, errors), then a grid with one expandable row per client (key, name, Internal and Client folder, document count, status badge). Expanded: the documents with type, year, entity, protect flag, classification source and confidence, a pencil to correct (the existing Review dialog), a drop action. Errors appear next to the thing they affect. Footer: Cancel, and Confirm "Create N requests".
3. **Batch progress**: header with counts, a row per request (case number link, step, status), "Retry failed", and "as of" time. Uses the existing processing timeline vocabulary.
4. All states designed: loading skeleton, empty, ZIP rejected (with reasons), denied, error with request id, partial success.

Copy uses "request", "client", "document"; never framework words.

## 11. Validation catalogue

| Check | Level | Result |
|---|---|---|
| ZIP unreadable, too large, too many entries, path traversal | batch | rejected |
| Root-level file or nested subfolder (D3) | item/doc | error, listed |
| Not a PDF (or no `%PDF` header), over 2 MB, name Box rejects | doc | error |
| Same file name twice in a client, same type+year+entity twice | doc | error / warning (as screen 2's "possible duplicate") |
| Client Key unknown / inactive | item | error, skipped |
| Client Internal or Client folder id missing or not found in Box | item | error (same message as pipeline step 4) |
| Type not in `document_type_routes`, year out of range | doc | error |
| Entity required by type but unresolved | doc | needs review |
| AI confidence under 60 % | doc | needs review |
| Client has an open request (D4) | item | warning |
| External-only type but External filing unavailable | doc | error |

Errors on a document make that document unfilable until fixed or dropped; a client with any unresolved
error is excluded on confirm unless the user fixes it. "Needs review" does not block, but is shown.

## 12. Reliability

- **Idempotency:** each item has `batch_id:client_key`. Before creating a record, the worker checks whether the item already has a `routing_record_id`; if so it resumes from that record instead of creating another.
- **Resume:** each step writes its result before moving on. On backend restart, batches in `processing` are resumed from the stored item states.
- **Per-client isolation:** an exception in one client marks that item `failed` with its message and the worker continues.
- **Retry:** per case (existing) and per batch (`retry_failed`): re-runs the item from its failed step; pipeline steps are already safe to repeat.
- **Failure between create and submit:** the record stays `collecting` with its temp folder; retry continues it. An abandoned item is cleaned by cancel (existing `discard_request` semantics: removes the temp folder).
- **Box limits:** concurrency semaphore (default 3), retry-with-backoff on 429/5xx inside the Box client, a per-request timeout.
- **Staging lifecycle:** staged ZIPs are deleted when the batch completes, is cancelled, or after 24 h unconfirmed (`expired`). Staging is behind an interface: local disk now, shared storage when more than one backend instance runs.

## 13. Security and compliance

- Same auth, roles and row-level rules as single requests (Rego, never in the UI). Staff see only their batches; admin sees all.
- Uploaded files are untrusted: type and header checks, ZIP-bomb limits, no extraction outside staging, names sanitised before use as Box names.
- Client folder ids come only from the database, never from the ZIP. The ZIP cannot choose where documents are filed.
- Documents are PHI-adjacent: not logged, staged files access-controlled and deleted on schedule.
- Every created case carries `bulk_batch_id`; the batch records who uploaded, when and the file name. Existing record audit applies.
- Secrets handling is unchanged (Box credentials only in the server process, token table not exposed).

## 14. Observability

Structured logs per batch and item (ids, counts, step, duration; no file contents). Metrics: batches, clients and
documents per batch, time per item, failure reasons, Box call latency and 429s. A batch screen "as of" time
and a clear partial-success state ("8 created, 2 failed").

## 15. Test plan

- **Unit:** ZIP parsing and safety (traversal, bomb, nested folders), filename convention parser, classification precedence, validation catalogue, idempotency key.
- **Service/integration (in-memory store):** batch with valid, unknown-client, bad-folder and unclassified items; confirm creates exactly one record per valid client; re-confirm creates none; one failing item does not stop others; restart resume.
- **Real Box (manual, test folders):** two clients, files appear in each client's Internal and Client `Year/<type>` folders, temp folders gone, no duplicates on retry.
- **Frontend:** component tests for review/progress states; Playwright on the mocked API for the happy path and error states; layout and accessibility specs.
- **Test data:** `docs/bulk-test-data/` already contains a ZIP (one folder per client, with a valid, a placeholder-folder and an unknown client) and an Excel variant; regenerate with real Client Keys (`CLI-1001` style) and convention filenames once D1 and D2 are settled.

## 16. Phasing

| Phase | Content | Exit criteria |
|---|---|---|
| 0 | Settle D1-D7; add `client_key`; client import (admin) | Every active client has a unique key |
| 1 | ZIP staging, parse, classify, validate, review screen; **no creation** | Test ZIP shows the expected preview, including all error types |
| 2 | Confirm, worker, per-client create/upload/submit, progress screen | Test ZIP creates the expected requests; real Box verified; failures isolated |
| 3 | Retry failed, cancel, expiry, resume after restart, metrics, docs | Kill-and-restart test passes; runbook and demo script updated |

## 17. Risks and mitigations

| Risk | Mitigation |
|---|---|
| Wrong client key mapping files to the wrong client | Exact unique match, preview shows client name beside the key, user confirms |
| Misclassified type or year files a document in the wrong folder | Convention/Excel first; AI and ambiguity flagged "needs review"; user can correct before confirm |
| Large ZIP strains memory | Stream to disk, stream entries during upload, size and entry caps |
| Box rate limits or outages mid-batch | Throttle, backoff, per-item retry, resumable batches |
| Case-number collisions with Pega's numbers | Existing create-only temp folder skips taken numbers |
| Duplicate cases on retry | Item idempotency key and "resume existing record" rule |
| Scope creep (client import, email, virus scan) | Listed as separate work (sections 2, 8) |

## 18. Worked example

Directory (after import):

| Client Key | Name | Internal Folder | Client Folder |
|---|---|---|---|
| CLI-1001 | Andrea Moore | 423190537329 | 423192412574 |
| CLI-1002 | Baskaran Se | 85156335451 | 85157894153 |
| CLI-1003 | John Smith | `/Clients/Smith_John` (placeholder) | `Smith_John` |

ZIP:

```
CLI-1001/K-1_2025_Oakwood.pdf   Tax-Return_2025.pdf   Balance-Sheet_2025.pdf
CLI-1002/K-1_2025_Harbor.pdf    Extensions_2025.pdf
CLI-1003/K-1_2025_Sample-Partners.pdf
CLI-9999/K-1_2025_Ghost.pdf
```

Preview: CLI-1001 ready (entity "Oakwood" matched, or needs review if ambiguous); CLI-1002 ready (Extensions is
external-only: Client folder only); CLI-1003 error (folder id is not a usable Box folder); CLI-9999 error (client not
found). The user confirms "Create 2 requests".

Result: two requests (next free numbers, for example TD-1056 and TD-1057). For Andrea's K-1: `Internal/2025/4. Tax
Documents`, `Internal/2025/6. Released to Client/K-1s` (protected), `Client/2025/K-1s` (protected). Temp folders
deleted, batch `completed_with_errors` (2 created, 2 skipped).
