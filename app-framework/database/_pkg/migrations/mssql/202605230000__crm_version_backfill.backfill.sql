-- Backfill optimistic-concurrency versions for CRM seed rows and enforce defaults.

UPDATE [crm].[accounts] SET [version] = 0 WHERE [version] IS NULL;
IF NOT EXISTS (SELECT 1 FROM sys.default_constraints dc JOIN sys.columns c ON c.default_object_id = dc.object_id WHERE dc.parent_object_id = OBJECT_ID(N'[crm].[accounts]') AND c.name = N'version')
    ALTER TABLE [crm].[accounts] ADD CONSTRAINT [df_accounts_version] DEFAULT 0 FOR [version];
ALTER TABLE [crm].[accounts] ALTER COLUMN [version] bigint NOT NULL;

UPDATE [crm].[activities] SET [version] = 0 WHERE [version] IS NULL;
IF NOT EXISTS (SELECT 1 FROM sys.default_constraints dc JOIN sys.columns c ON c.default_object_id = dc.object_id WHERE dc.parent_object_id = OBJECT_ID(N'[crm].[activities]') AND c.name = N'version')
    ALTER TABLE [crm].[activities] ADD CONSTRAINT [df_activities_version] DEFAULT 0 FOR [version];
ALTER TABLE [crm].[activities] ALTER COLUMN [version] bigint NOT NULL;

UPDATE [crm].[activity_types] SET [version] = 0 WHERE [version] IS NULL;
IF NOT EXISTS (SELECT 1 FROM sys.default_constraints dc JOIN sys.columns c ON c.default_object_id = dc.object_id WHERE dc.parent_object_id = OBJECT_ID(N'[crm].[activity_types]') AND c.name = N'version')
    ALTER TABLE [crm].[activity_types] ADD CONSTRAINT [df_activity_types_version] DEFAULT 0 FOR [version];
ALTER TABLE [crm].[activity_types] ALTER COLUMN [version] bigint NOT NULL;

UPDATE [crm].[contacts] SET [version] = 0 WHERE [version] IS NULL;
IF NOT EXISTS (SELECT 1 FROM sys.default_constraints dc JOIN sys.columns c ON c.default_object_id = dc.object_id WHERE dc.parent_object_id = OBJECT_ID(N'[crm].[contacts]') AND c.name = N'version')
    ALTER TABLE [crm].[contacts] ADD CONSTRAINT [df_contacts_version] DEFAULT 0 FOR [version];
ALTER TABLE [crm].[contacts] ALTER COLUMN [version] bigint NOT NULL;

UPDATE [crm].[industries] SET [version] = 0 WHERE [version] IS NULL;
IF NOT EXISTS (SELECT 1 FROM sys.default_constraints dc JOIN sys.columns c ON c.default_object_id = dc.object_id WHERE dc.parent_object_id = OBJECT_ID(N'[crm].[industries]') AND c.name = N'version')
    ALTER TABLE [crm].[industries] ADD CONSTRAINT [df_industries_version] DEFAULT 0 FOR [version];
ALTER TABLE [crm].[industries] ALTER COLUMN [version] bigint NOT NULL;

UPDATE [crm].[leads] SET [version] = 0 WHERE [version] IS NULL;
IF NOT EXISTS (SELECT 1 FROM sys.default_constraints dc JOIN sys.columns c ON c.default_object_id = dc.object_id WHERE dc.parent_object_id = OBJECT_ID(N'[crm].[leads]') AND c.name = N'version')
    ALTER TABLE [crm].[leads] ADD CONSTRAINT [df_leads_version] DEFAULT 0 FOR [version];
ALTER TABLE [crm].[leads] ALTER COLUMN [version] bigint NOT NULL;

UPDATE [crm].[lead_sources] SET [version] = 0 WHERE [version] IS NULL;
IF NOT EXISTS (SELECT 1 FROM sys.default_constraints dc JOIN sys.columns c ON c.default_object_id = dc.object_id WHERE dc.parent_object_id = OBJECT_ID(N'[crm].[lead_sources]') AND c.name = N'version')
    ALTER TABLE [crm].[lead_sources] ADD CONSTRAINT [df_lead_sources_version] DEFAULT 0 FOR [version];
ALTER TABLE [crm].[lead_sources] ALTER COLUMN [version] bigint NOT NULL;

UPDATE [crm].[lead_statuses] SET [version] = 0 WHERE [version] IS NULL;
IF NOT EXISTS (SELECT 1 FROM sys.default_constraints dc JOIN sys.columns c ON c.default_object_id = dc.object_id WHERE dc.parent_object_id = OBJECT_ID(N'[crm].[lead_statuses]') AND c.name = N'version')
    ALTER TABLE [crm].[lead_statuses] ADD CONSTRAINT [df_lead_statuses_version] DEFAULT 0 FOR [version];
ALTER TABLE [crm].[lead_statuses] ALTER COLUMN [version] bigint NOT NULL;

UPDATE [crm].[opportunities] SET [version] = 0 WHERE [version] IS NULL;
IF NOT EXISTS (SELECT 1 FROM sys.default_constraints dc JOIN sys.columns c ON c.default_object_id = dc.object_id WHERE dc.parent_object_id = OBJECT_ID(N'[crm].[opportunities]') AND c.name = N'version')
    ALTER TABLE [crm].[opportunities] ADD CONSTRAINT [df_opportunities_version] DEFAULT 0 FOR [version];
ALTER TABLE [crm].[opportunities] ALTER COLUMN [version] bigint NOT NULL;

UPDATE [crm].[opportunity_stages] SET [version] = 0 WHERE [version] IS NULL;
IF NOT EXISTS (SELECT 1 FROM sys.default_constraints dc JOIN sys.columns c ON c.default_object_id = dc.object_id WHERE dc.parent_object_id = OBJECT_ID(N'[crm].[opportunity_stages]') AND c.name = N'version')
    ALTER TABLE [crm].[opportunity_stages] ADD CONSTRAINT [df_opportunity_stages_version] DEFAULT 0 FOR [version];
ALTER TABLE [crm].[opportunity_stages] ALTER COLUMN [version] bigint NOT NULL;

UPDATE [crm].[pricebooks] SET [version] = 0 WHERE [version] IS NULL;
IF NOT EXISTS (SELECT 1 FROM sys.default_constraints dc JOIN sys.columns c ON c.default_object_id = dc.object_id WHERE dc.parent_object_id = OBJECT_ID(N'[crm].[pricebooks]') AND c.name = N'version')
    ALTER TABLE [crm].[pricebooks] ADD CONSTRAINT [df_pricebooks_version] DEFAULT 0 FOR [version];
ALTER TABLE [crm].[pricebooks] ALTER COLUMN [version] bigint NOT NULL;

UPDATE [crm].[products] SET [version] = 0 WHERE [version] IS NULL;
IF NOT EXISTS (SELECT 1 FROM sys.default_constraints dc JOIN sys.columns c ON c.default_object_id = dc.object_id WHERE dc.parent_object_id = OBJECT_ID(N'[crm].[products]') AND c.name = N'version')
    ALTER TABLE [crm].[products] ADD CONSTRAINT [df_products_version] DEFAULT 0 FOR [version];
ALTER TABLE [crm].[products] ALTER COLUMN [version] bigint NOT NULL;

UPDATE [crm].[quotes] SET [version] = 0 WHERE [version] IS NULL;
IF NOT EXISTS (SELECT 1 FROM sys.default_constraints dc JOIN sys.columns c ON c.default_object_id = dc.object_id WHERE dc.parent_object_id = OBJECT_ID(N'[crm].[quotes]') AND c.name = N'version')
    ALTER TABLE [crm].[quotes] ADD CONSTRAINT [df_quotes_version] DEFAULT 0 FOR [version];
ALTER TABLE [crm].[quotes] ALTER COLUMN [version] bigint NOT NULL;

UPDATE [crm].[quote_line_items] SET [version] = 0 WHERE [version] IS NULL;
IF NOT EXISTS (SELECT 1 FROM sys.default_constraints dc JOIN sys.columns c ON c.default_object_id = dc.object_id WHERE dc.parent_object_id = OBJECT_ID(N'[crm].[quote_line_items]') AND c.name = N'version')
    ALTER TABLE [crm].[quote_line_items] ADD CONSTRAINT [df_quote_line_items_version] DEFAULT 0 FOR [version];
ALTER TABLE [crm].[quote_line_items] ALTER COLUMN [version] bigint NOT NULL;

UPDATE [crm].[quote_statuses] SET [version] = 0 WHERE [version] IS NULL;
IF NOT EXISTS (SELECT 1 FROM sys.default_constraints dc JOIN sys.columns c ON c.default_object_id = dc.object_id WHERE dc.parent_object_id = OBJECT_ID(N'[crm].[quote_statuses]') AND c.name = N'version')
    ALTER TABLE [crm].[quote_statuses] ADD CONSTRAINT [df_quote_statuses_version] DEFAULT 0 FOR [version];
ALTER TABLE [crm].[quote_statuses] ALTER COLUMN [version] bigint NOT NULL;
