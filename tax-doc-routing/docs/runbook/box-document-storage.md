# Document storage in Box: how it works and how to test it for real

Status (2026-10-18): built and tested end to end **on the in-memory store**. The Box store itself is
unit-tested against a fake Box. **Nothing has been run against the real Box yet.** This page is the
procedure for doing that safely.

## What happens to a request

| When | What the app does |
|---|---|
| User presses **Create Request** on screen 1 | Creates the record, gives it the next number (`TD-1001`, ...) and creates the temporary folder `TD-1001` **inside the Box folder named by `STORAGE_TEMP_PARENT_FOLDER`** (the client's temp location, folder id 84473772615), or inside the client's Internal folder if that setting is empty. If storage is unavailable the request is not created and the screen shows why. |
| User adds a PDF on screen 2 | PDF only, 2 MB, a name that Box accepts, no repeat of a file already uploaded. Stored straight in the temp folder. |
| User presses **Submit** | Saves one row per document, then files them **on the server** (the browser only watches progress, so closing the tab does not stop it). |
| Filing, step by step | **4** check the client's own Internal folder id and Client (External) folder id exist in Box (the ids on the client record, shown on screen 1). **5** read each file; password-protect the client copy if asked (password = the client's Read Write Password). **6** create or reuse `Year/<type folder>` and copy the file to each destination. **7** delete the temp folder, only after every file is filed. **8** client email: not built, logged as skipped. **9** done. |
| Any step fails | An exception task opens (Exception Task screen), the request shows Exception, and the **temp folder is kept**. Retry re-runs the filing; every step is safe to repeat. |
| User presses **Cancel** before submitting | The request is cancelled and its temp folder deleted. |

Folder layout (same as Pega), for a K-1 for 2025 uploaded as `Pega AI.pdf` by Andrea Moore:

```
Client Internal folder
  2025/4. Tax Documents/Moore, Andrea - Pega AI.pdf             (never encrypted)
  2025/6. Released to Client/K-1s/Moore, Andrea - Pega AI.pdf   (encrypted if requested)
Client Client folder
  2025/K-1s/Moore, Andrea - Pega AI.pdf                         (encrypted if requested)
```

Types S Corp Election, Form 8308 and Extensions have no internal side. Folder names come from the
`document_type_routes` table, not from code.

## The temp folder is always created new

The temp folder `TD-xxxx` is created, never reused, because it is deleted after filing. If a folder
with that name already exists (the Box location already holds Pega's own `TD-2791` to `TD-2797`),
the app skips to the next number instead of adopting it, so it can never mix files into, or delete,
a folder it did not create. When our numbers reach that range expect some numbers to be skipped.

## Settings

| Setting | Meaning |
|---|---|
| `STORAGE_PROVIDER` | unset or `memory` (default: works without Box, files vanish on restart) or `box` |
| `BOX_CLIENT_ID`, `BOX_CLIENT_SECRET`, `BOX_ENTERPRISE_ID`, `BOX_PUBLIC_KEY_ID` | the Box app |
| `BOX_KEYSTORE_PATH`, `BOX_KEYSTORE_PASSWORD` | the PKCS#12 key store (Pega's PDSBOX) |
| `APP_REQUEST_BODY_LIMIT_BYTES` | set to at least `4194304`; a 2 MB PDF is about 2.7 MB as base64 |
| `MAX_UPLOAD_BYTES` | optional, default 2097152 |
| `STORAGE_EXTERNAL_ENABLED` | Default on. `false` switches off filing to the client's Client (External) folder. |
| `STORAGE_TEMP_PARENT_FOLDER` | Box folder id or URL where the `TD-xxxx` temp folders are created. Empty = each client's own Internal folder. |
| `RELEASED_TO_CLIENT_FOLDER_NAME` | optional, default `6. Released to Client` |

The Box bearer token is kept in the `tax_routing.storage_tokens` table (one row), reused until it
expires and then replaced. No role can read that table through the API.

## Testing against the real Box (use TEST folders)

The seed clients hold placeholder folder ids (`/Clients/Smith_John`), which Box cannot use: submitting for them stops at step 4 with a clear message. Point **one
test client** at **test** Box folders (the id is the last part of the folder URL):

```sql
update tax_routing.tax_payer_ref_data
   set internal_folder_path = '<Box URL or id of the TEST client Internal folder>',
       folder_name          = '<Box URL or id of the TEST client Client folder>'
 where last_name = 'Smith';
```

Then, from `tax-doc-routing\`, in your own terminal (the script asks for the secrets, hidden):

1. `.\scripts\box-local.ps1 -Mode check`  opens the key store and signs a login request. Calls nothing on Box.
2. `.\scripts\box-local.ps1 -Mode box`  creates, uploads, reads, refuses a duplicate and deletes one throwaway folder inside a test folder you name.
3. `.\scripts\box-local.ps1 -Mode run`  starts the backend against Box. Create a request as `alex` for the test client, upload a PDF, Submit, and look in Box.

## Known limits

- The Box app's account must be able to create and delete folders under the Internal and Client folders (Pega granted itself collaborator access before deleting the temp folder).
- Virus scanning is not built (the scanner has not been chosen). Uploads are checked for type, size and name only.
- The client email is not sent. The Completed screen's "Client Notification: Sent" shows the stored choice, not a sent email.
- In `memory` mode a restart loses the stored files, so retrying a request that was submitted before the restart fails with a clear exception.
- A request abandoned without pressing Cancel stays open in My Cases, with its temp folder.

## Bulk request creation

Dashboard, **Bulk Request**. The user picks an Excel file (one row per client: Client Key, names, emails,
Internal Folder ID, Client Folder ID, Read Write Password, Send Notification) and a ZIP (one folder per
Client Key holding that client's PDFs, named `<Type>_<Year>_<Entity>.pdf`). The preview checks everything and
creates nothing; **Create N Requests** then makes one request per ready client, using the same create, upload
and submit code as a single request, so Box filing is the existing pipeline.

- A client is found by `tax_payer_ref_data.client_key`. If found, the directory's values are used (the file's
  differing values are shown as notes). If not found, a **Tax Admin** can create it from the file's row; a Tax
  Staff user gets an error in the preview instead.
- Clients with a problem are skipped; one failing client never stops the others.
- Batch state is kept in memory for 24 hours (a backend restart loses an unfinished preview or progress screen;
  requests already created are unaffected). The upload is sent inside the request body, so the backend must run
  with `APP_REQUEST_BODY_LIMIT_BYTES` of at least 64 MiB (`scripts/box-local.ps1` sets it), about 45 MB of ZIP.
- Sample files: `docs/bulk-test-data/bulk_v1/`.

### Bulk performance and the settings that control it

Checking a bulk upload runs in the background (the screen shows real progress), clients are created several at a
time, and filing is limited so a large batch does not flood the database or Box. All settings are optional.

| Setting | Default | Meaning |
|---|---|---|
| `BULK_CONCURRENCY` | 6 | Clients whose requests are created at the same time |
| `FILING_CONCURRENCY` | 8 | Requests being filed into Box at the same time (all requests, bulk or single) |
| `BOX_MAX_CONCURRENCY` | 8 | Box API calls in flight at once, however many requests are active |
| `BULK_MAX_CLIENTS` | 500 | Most clients in one Excel file |
| `STORAGE_SIMULATED_LATENCY_MS` | 0 | Testing aid for the in-memory store only: waits this long before every storage call, to stand in for Box |

Retrying: a bulk batch that finishes with failures shows **Retry Failed**. A client whose request was already created
carries on with that request (files already uploaded to its temp folder are reused); a client that was never created gets
its request then. Nothing is created twice. A request that failed while being filed (an exception task) is retried
from the Exception Task screen as before.
