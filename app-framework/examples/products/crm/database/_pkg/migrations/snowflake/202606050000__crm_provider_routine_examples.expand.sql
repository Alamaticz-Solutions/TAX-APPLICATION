-- Example provider routines for model-driven custom method bindings.
-- These routines are intentionally small but real: the function returns a
-- health snapshot row, while the stored procedure persists the score and
-- returns the row it wrote.

CREATE SCHEMA IF NOT EXISTS "crm";

ALTER TABLE IF EXISTS "crm"."accounts"
    ADD COLUMN IF NOT EXISTS "health_score" FLOAT;

ALTER TABLE IF EXISTS "crm"."accounts"
    ADD COLUMN IF NOT EXISTS "health_last_refreshed_at" TIMESTAMP_TZ;

CREATE OR REPLACE FUNCTION "crm"."account_health_provider_function"(
    account_id STRING
)
RETURNS VARIANT
LANGUAGE SQL
AS
$$
    OBJECT_CONSTRUCT(
        'account_id', account_id,
        'health_score', COALESCE((
            SELECT "health_score"
            FROM "crm"."accounts"
            WHERE TO_VARCHAR("id") = account_id
            LIMIT 1
        ), 85.0),
        'refreshed_at', (
            SELECT "health_last_refreshed_at"
            FROM "crm"."accounts"
            WHERE TO_VARCHAR("id") = account_id
            LIMIT 1
        ),
        'source', 'snowflake-provider-function'
    )
$$;

CREATE OR REPLACE PROCEDURE "crm"."refresh_account_health_stored_procedure"(
    account_id STRING,
    health_score FLOAT
)
RETURNS VARIANT
LANGUAGE SQL
AS
$$
DECLARE
    refreshed_at TIMESTAMP_TZ DEFAULT CURRENT_TIMESTAMP();
BEGIN
    UPDATE "crm"."accounts"
    SET
        "health_score" = :health_score,
        "health_last_refreshed_at" = :refreshed_at,
        "version" = COALESCE("version", 0) + 1
    WHERE TO_VARCHAR("id") = :account_id;

    RETURN OBJECT_CONSTRUCT(
        'account_id', account_id,
        'health_score', health_score,
        'refreshed_at', refreshed_at,
        'source', 'snowflake-stored-procedure'
    );
END;
$$;
