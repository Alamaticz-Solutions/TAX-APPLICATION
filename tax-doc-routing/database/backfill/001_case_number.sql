-- Gives every existing routing record a case number (shown in the UI as TD-1001, TD-1002, ...).
--
-- Run once against any database that already has routing records from before
-- `tax_routing_records.case_number` existed. New records get their number from the backend at
-- creation time (handlers/tax_routing/tax_routing_record.rs), so this is only for older rows.
--
-- Safe to re-run: only rows that have no number are touched, an existing number is never
-- changed, and new numbers continue after the current highest one. Rows are numbered oldest
-- first (created_at, then id) so the result is deterministic. `version` is left alone because
-- this is a one-off data fix, not a user edit.
--
--   docker exec -i tax-doc-routing-postgres psql -U postgres -d tax_routing \
--     < tax-doc-routing/database/backfill/001_case_number.sql

WITH base AS (
  SELECT COALESCE(MAX(case_number), 1000) AS highest
  FROM tax_routing.tax_routing_records
),
numbered AS (
  SELECT r.id,
         (SELECT highest FROM base) + ROW_NUMBER() OVER (ORDER BY r.created_at, r.id) AS n
  FROM tax_routing.tax_routing_records r
  WHERE r.case_number IS NULL
)
UPDATE tax_routing.tax_routing_records t
SET case_number = numbered.n
FROM numbered
WHERE t.id = numbered.id;
