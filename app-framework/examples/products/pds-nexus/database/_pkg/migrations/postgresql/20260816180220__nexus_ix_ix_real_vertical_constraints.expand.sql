--
-- Forward-only expand migration.
-- Schema: nexus_ix
-- Name: nexus_ix_ix_real_vertical_constraints
-- Dialect: PostgreSQL
--
-- Supplemental Nexus IX tenant, idempotency, ordering, and projection
-- constraints. The generated nexus_ix DDL remains the table authority; this
-- migration only adds the uniqueness guarantees the runtime repository
-- adapter relies on. Every statement is idempotent and additive.

-- Tenant-scoped start idempotency: one durable run per
-- (tenant, start_idempotency_key); retries deduplicate instead of forking.
CREATE UNIQUE INDEX IF NOT EXISTS ux_ix_runs_tenant_start_idempotency_key
    ON nexus_ix.ix_runs ("tenant", "start_idempotency_key");

-- Commit ordering: at most one canonical envelope per run and revision;
-- compare-and-swap appends cannot double-apply a revision.
CREATE UNIQUE INDEX IF NOT EXISTS ux_ix_commit_envelopes_run_id_sequence
    ON nexus_ix.ix_commit_envelopes ("run_id", "sequence");

-- Audit-projection dedup: the external cancel projection identity
-- (tenant, run_id, command_id, cursor) is recorded exactly once.
CREATE UNIQUE INDEX IF NOT EXISTS ux_ix_projection_cursors_dedup_identity
    ON nexus_ix.ix_projection_cursors ("tenant", "run_id", "command_id", "cursor");
