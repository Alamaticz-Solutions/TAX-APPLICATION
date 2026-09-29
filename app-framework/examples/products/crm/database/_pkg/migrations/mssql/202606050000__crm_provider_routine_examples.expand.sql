-- Example provider routines for model-driven custom method bindings.
-- These routines are intentionally small but real: the function returns a
-- health snapshot row, while the stored procedure persists the score and
-- returns the row it wrote.

IF COL_LENGTH(N'crm.accounts', N'health_score') IS NULL
    ALTER TABLE [crm].[accounts] ADD [health_score] float;
GO

IF COL_LENGTH(N'crm.accounts', N'health_last_refreshed_at') IS NULL
    ALTER TABLE [crm].[accounts] ADD [health_last_refreshed_at] datetimeoffset(7);
GO

CREATE OR ALTER FUNCTION [crm].[account_health_provider_function](
    @account_id NVARCHAR(100)
)
RETURNS TABLE
AS
RETURN
(
    SELECT
        CONVERT(NVARCHAR(100), [id]) AS [account_id],
        COALESCE([health_score], CAST(85.0 AS FLOAT)) AS [health_score],
        [health_last_refreshed_at] AS [refreshed_at],
        N'mssql-provider-function' AS [source]
    FROM [crm].[accounts]
    WHERE [id] = TRY_CONVERT(uniqueidentifier, @account_id)
);
GO

CREATE OR ALTER PROCEDURE [crm].[refresh_account_health_stored_procedure]
    @account_id NVARCHAR(100),
    @health_score FLOAT
AS
BEGIN
    SET NOCOUNT ON;
    DECLARE @account_uuid uniqueidentifier = TRY_CONVERT(uniqueidentifier, @account_id);
    DECLARE @refreshed_at datetimeoffset(7) = SYSUTCDATETIME();

    IF @account_uuid IS NULL
        THROW 51000, 'account_id must be a valid uniqueidentifier', 1;

    UPDATE [crm].[accounts]
    SET
        [health_score] = @health_score,
        [health_last_refreshed_at] = @refreshed_at,
        [version] = ISNULL([version], 0) + 1
    WHERE [id] = @account_uuid;

    IF @@ROWCOUNT = 0
        THROW 51001, 'account was not found', 1;

    SELECT
        CONVERT(NVARCHAR(100), @account_uuid) AS [account_id],
        @health_score AS [health_score],
        @refreshed_at AS [refreshed_at],
        N'mssql-stored-procedure' AS [source];
END;
GO
