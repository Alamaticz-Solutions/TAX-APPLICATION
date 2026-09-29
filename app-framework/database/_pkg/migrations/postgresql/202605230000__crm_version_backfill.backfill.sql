-- Backfill optimistic-concurrency versions for CRM seed rows and enforce defaults.

UPDATE crm.accounts SET version = 0 WHERE version IS NULL;
ALTER TABLE crm.accounts ALTER COLUMN version SET DEFAULT 0;
ALTER TABLE crm.accounts ALTER COLUMN version SET NOT NULL;

UPDATE crm.activities SET version = 0 WHERE version IS NULL;
ALTER TABLE crm.activities ALTER COLUMN version SET DEFAULT 0;
ALTER TABLE crm.activities ALTER COLUMN version SET NOT NULL;

UPDATE crm.activity_types SET version = 0 WHERE version IS NULL;
ALTER TABLE crm.activity_types ALTER COLUMN version SET DEFAULT 0;
ALTER TABLE crm.activity_types ALTER COLUMN version SET NOT NULL;

UPDATE crm.contacts SET version = 0 WHERE version IS NULL;
ALTER TABLE crm.contacts ALTER COLUMN version SET DEFAULT 0;
ALTER TABLE crm.contacts ALTER COLUMN version SET NOT NULL;

UPDATE crm.industries SET version = 0 WHERE version IS NULL;
ALTER TABLE crm.industries ALTER COLUMN version SET DEFAULT 0;
ALTER TABLE crm.industries ALTER COLUMN version SET NOT NULL;

UPDATE crm.leads SET version = 0 WHERE version IS NULL;
ALTER TABLE crm.leads ALTER COLUMN version SET DEFAULT 0;
ALTER TABLE crm.leads ALTER COLUMN version SET NOT NULL;

UPDATE crm.lead_sources SET version = 0 WHERE version IS NULL;
ALTER TABLE crm.lead_sources ALTER COLUMN version SET DEFAULT 0;
ALTER TABLE crm.lead_sources ALTER COLUMN version SET NOT NULL;

UPDATE crm.lead_statuses SET version = 0 WHERE version IS NULL;
ALTER TABLE crm.lead_statuses ALTER COLUMN version SET DEFAULT 0;
ALTER TABLE crm.lead_statuses ALTER COLUMN version SET NOT NULL;

UPDATE crm.opportunities SET version = 0 WHERE version IS NULL;
ALTER TABLE crm.opportunities ALTER COLUMN version SET DEFAULT 0;
ALTER TABLE crm.opportunities ALTER COLUMN version SET NOT NULL;

UPDATE crm.opportunity_stages SET version = 0 WHERE version IS NULL;
ALTER TABLE crm.opportunity_stages ALTER COLUMN version SET DEFAULT 0;
ALTER TABLE crm.opportunity_stages ALTER COLUMN version SET NOT NULL;

UPDATE crm.pricebooks SET version = 0 WHERE version IS NULL;
ALTER TABLE crm.pricebooks ALTER COLUMN version SET DEFAULT 0;
ALTER TABLE crm.pricebooks ALTER COLUMN version SET NOT NULL;

UPDATE crm.products SET version = 0 WHERE version IS NULL;
ALTER TABLE crm.products ALTER COLUMN version SET DEFAULT 0;
ALTER TABLE crm.products ALTER COLUMN version SET NOT NULL;

UPDATE crm.quotes SET version = 0 WHERE version IS NULL;
ALTER TABLE crm.quotes ALTER COLUMN version SET DEFAULT 0;
ALTER TABLE crm.quotes ALTER COLUMN version SET NOT NULL;

UPDATE crm.quote_line_items SET version = 0 WHERE version IS NULL;
ALTER TABLE crm.quote_line_items ALTER COLUMN version SET DEFAULT 0;
ALTER TABLE crm.quote_line_items ALTER COLUMN version SET NOT NULL;

UPDATE crm.quote_statuses SET version = 0 WHERE version IS NULL;
ALTER TABLE crm.quote_statuses ALTER COLUMN version SET DEFAULT 0;
ALTER TABLE crm.quote_statuses ALTER COLUMN version SET NOT NULL;
