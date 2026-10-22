-- Copies each routing record's case number onto its exception tasks (TD-1001 labels in the
-- Exception Queue). Idempotent: only fills tasks that have no case number yet, and leaves
-- version untouched (same approach as 001_case_number.sql).
--
-- Run after `product migrate` has added exception_tasks.case_number:
--   docker exec -i tax-doc-routing-postgres psql -U postgres -d tax_routing < tax-doc-routing/database/backfill/002_exception_task_case_number.sql

UPDATE tax_routing.exception_tasks t
SET case_number = r.case_number
FROM tax_routing.tax_routing_records r
WHERE r.id = t.routing_record_id
  AND t.case_number IS NULL
  AND r.case_number IS NOT NULL;
