# Bulk request test files (test data only)

Nothing in the app reads these yet. One request = one client (auto-populated from the client directory) + all of that client's documents.

## Option A: `OptionA_bulk_upload.zip` (one folder per client = one request)
Each top-level folder is a Client ID; the PDFs inside are that request's documents. No Excel needed.
(Unzipped copy: `optionA_zip_folder_per_client/`.)

## Option B: `optionB_excel_plus_pdfs/` (Excel + loose PDFs)
`bulk_requests.xlsx`: one row per document. Client columns say which request a row belongs to; document columns are optional
(blank = AI reads the file, as on screen 2). `pdfs/` holds the files the rows name. Sheet 2 shows what the app fills in.

## Expected outcome (same for both options)
| Client | Documents | Expected |
|---|---|---|
| Andrea Moore | 3 | OK (one needs review) |
| Baskaran Se | 2 | OK |
| John Smith | 1 | Error: placeholder folders |
| Unknown ID | 1 | Error: client not found, skipped |

Older files in this folder (`bulk_requests_VALID.xlsx`, `bulk_requests_WITH_ERRORS.xlsx`, `pdfs/`, `all_test_pdfs.zip`) are the first draft and can be deleted.
