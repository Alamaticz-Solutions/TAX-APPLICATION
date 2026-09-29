--
-- Forward-only baseline migration.
-- Expand phase: safe to apply before deploying application changes.
--

IF NOT EXISTS (SELECT 1 FROM sys.schemas WHERE name = N'crm')
    EXEC('CREATE SCHEMA [crm]');
GO
