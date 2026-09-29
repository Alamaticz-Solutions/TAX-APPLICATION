-- Example provider routines for model-driven custom method bindings.
-- These routines are intentionally small but real: the function returns a
-- health snapshot row, while the stored procedure persists the score and
-- returns the row it wrote.

ALTER TABLE crm.accounts
    ADD COLUMN IF NOT EXISTS "health_score" double precision;

ALTER TABLE crm.accounts
    ADD COLUMN IF NOT EXISTS "health_last_refreshed_at" timestamptz;

CREATE OR REPLACE FUNCTION crm.account_health_provider_function(
    p_account_id text
)
RETURNS TABLE (
    account_id text,
    health_score double precision,
    refreshed_at timestamptz,
    source text
)
LANGUAGE sql
STABLE
AS $$
    SELECT
        p_account_id AS account_id,
        COALESCE(a.health_score, 85.0)::double precision AS health_score,
        a.health_last_refreshed_at AS refreshed_at,
        'postgres-provider-function'::text AS source
    FROM crm.accounts a
    WHERE a.id = p_account_id::uuid
$$;

CREATE OR REPLACE PROCEDURE crm.refresh_account_health_stored_procedure(
    p_account_id text,
    p_health_score double precision,
    OUT p_result jsonb
)
LANGUAGE plpgsql
AS $$
DECLARE
    v_refreshed_at timestamptz := NOW();
BEGIN
    UPDATE crm.accounts
    SET
        health_score = p_health_score,
        health_last_refreshed_at = v_refreshed_at,
        version = COALESCE(version, 0) + 1
    WHERE id = p_account_id::uuid
    RETURNING jsonb_build_object(
        'account_id', id::text,
        'health_score', health_score,
        'refreshed_at', health_last_refreshed_at,
        'source', 'postgres-stored-procedure'
    )
    INTO p_result;

    IF p_result IS NULL THEN
        RAISE EXCEPTION 'account % was not found', p_account_id;
    END IF;
END;
$$;
