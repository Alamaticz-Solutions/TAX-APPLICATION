# Tax Document Routing — Database Package

Product-owned generated database package for the `tax_routing` schema on the `pg_primary`
PostgreSQL data source (database `tax_routing`).

Generated DDL and seed SQL live under `database/_pkg/schemas/**/{tables,seed}.pg.sql`
and hand-written migrations under `database/_pkg/migrations/`.

## Regenerate

```bash
# from the repository root; on Windows use the rust-appfw container (see the root CLAUDE.md)
scripts/appfw product generate
scripts/appfw product generate --check --json
```

## Apply locally

```bash
docker compose -f ../podman-compose.yml up -d postgres
scripts/appfw product migrate
```

The local container publishes Postgres on host port 5434 (set `APP_POSTGRES_HOST_PORT=5434`), so
it can run beside other products' databases.
