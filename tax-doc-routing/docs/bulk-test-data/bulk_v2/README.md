# Two-client bulk test: one filed Internally, one External only

Bulk Request screen: Excel = `bulk_clients_two_clients.xlsx`, ZIP = `bulk_documents_two_clients.zip`. Test data only.
Both clients use the Tax - Stage Internal (423190537329) and Tax - Stage External (423192412574) folders, so you can see both
results in Box. The file's folder ids are used (they override the client directory).

| Client | Documents | Types | Where they go |
|---|---|---|---|
| CLI-1001 Andrea Moore (Internal) | 4 (year 2025) | K-1, Tax Return, Balance Sheet, Income Statement | Internal folder `2025/...` (these types have an internal side), plus the client copies the normal rules make |
| CLI-1002 Baskaran Se (External only) | 3 (year 2024) | Extensions, Form 8308, S-Corp Election | Client (External) folder `2024/...` only. Nothing is created in the Internal folder. |

Why: the document type decides where a file goes (`document_type_routes`). Extensions, Form 8308, S-Corp and S-Corp
Election are external-only types; the rest have an internal side. The second sheet of the Excel file lists the exact
Box folder for every document.

Expect two requests (next free TD numbers), each with all of its client's documents, both Resolved.
