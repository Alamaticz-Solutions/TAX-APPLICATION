# Bulk request test files (matches the built feature)

Use on the **Bulk Request** screen (Dashboard, "Bulk Request"): pick `bulk_clients.xlsx` as the Excel file
and `bulk_documents.zip` as the ZIP, then press Check Files. Test data only.

Client Keys `CLI-1001` (Andrea Moore) and `CLI-1002` (Guna Sekaran) are set on those two clients in the
client directory (`tax_payer_ref_data.client_key`).

| Client Key | Expected in the preview |
|---|---|
| CLI-1001 | Ready: 3 documents. Filed to the Tax-Stage Internal/External folders. |
| CLI-1002 | Needs attention: `scan0001.pdf` has no readable type/year. Use the pencil to set Tax Return, 2025, then it is Ready. |
| CLI-2001 | A new client (not in the directory). Ready for a Tax Admin; for Tax Staff the preview says only an admin can add new clients. |
| CLI-3001 | Needs attention: no folder in the ZIP. Skipped. |
| CLI-9999 | Needs attention: a ZIP folder with no Excel row. Skipped. |
| loose_file_at_top.pdf | A note: ignored because it is not inside a client folder. |

PDF names follow `<Type>_<Year>_<Entity>.pdf`, for example `K-1_2025_Oakwood-Medical-Partners.pdf`.
