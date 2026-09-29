# Database Migrations

The database utility supports two workflows:

- `cargo run --locked` keeps the current local bootstrap/reconcile behavior.
- `cargo run --locked -- migrate ...` uses forward-only versioned migrations.

Versioned migrations are stored in `_pkg/migrations/` and recorded in each
database in `app_meta.schema_migrations`. Applied migrations are immutable:
changing a migration file after it is applied causes a checksum mismatch.
Each run also uses `app_meta.schema_migration_lock` to prevent concurrent
applies; if a process is killed mid-run, inspect that table before manually
clearing a stale lock row.

## Commands

```sh
cargo run --locked -- migrate new --schema crm --phase backfill --name normalize_contacts
cargo run --locked -- migrate plan
cargo run --locked -- migrate plan --json
cargo run --locked -- migrate lint
cargo run --locked -- migrate rollback-guide --json
cargo run --locked -- migrate status
cargo run --locked -- migrate status --json
cargo run --locked -- migrate apply
cargo run --locked -- migrate apply --json
cargo run --locked -- migrate apply --phase expand
cargo run --locked -- migrate apply --phase backfill
```

`plan` and `status` are read-only. They inspect the migration registry only
when the target database and registry already exist; they do not create the
database, `app_meta` schema, migration tables, or migration lock.

`new` scaffolds a forward-only SQL file and appends the matching manifest entry.
By default it targets the configured data source for the schema. Use
`--data-source` for a specific target or `--dialect all` to scaffold one file
for each relational provider.

`lint` reads generated SQL without connecting to the database. It reports
unsafe contract operations outside `contract`, unbounded data changes,
non-idempotent inserts, blocking index risks, missing existence guards, and
long-lock risks. Destructive contract migrations should include a reviewed
`-- appfw: rollback-reviewed <ticket>` marker; the linter warns when that marker
is missing. The full report is written to:

```text
target/appfw/migration_lint_<data_source>.json
```

By default, `migrate apply` runs only the safe zero-downtime phases:
`expand` and `backfill`.

Contract migrations are deliberately excluded from the default path:

```sh
cargo run --locked -- migrate apply --phase contract --confirm-contract
```

There are no `down.sql` files. Production rollback is app-first and DB-forward:
roll back the backend deployment while leaving expanded database structures in
place. Destructive cleanup belongs in a later `contract` migration after old
application versions are no longer running.

Use the machine-readable guide when planning release and rollback windows:

```sh
cargo run --locked -- migrate rollback-guide --json
```

## Phases

- `expand`: add backward-compatible structures such as nullable columns, new
  tables, non-breaking indexes, or non-enforced constraints.
- `backfill`: move or copy data in idempotent batches.
- `contract`: remove old structures only after a safety window.

## Safety Gates

`migrate apply` runs migration linting before connecting to providers and
blocks on lint errors. Warnings are emitted for operations that may be safe
only with provider-specific review.

For PostgreSQL and SQL Server, `migrate apply` also inspects the live database
catalog before applying migrations. It compares generated expected columns with
actual columns, emits a drift report, and blocks when an existing column has an
incompatible type.

```text
target/appfw/schema_drift_<data_source>.json
```

Missing generated columns are allowed because they may be introduced by the
pending migration. Unexpected live columns are warnings so operators can decide
whether they represent legitimate out-of-band extensions or unmanaged drift.

## Backfills

Backfills should be resumable and chunked by a stable key. Provider primitives
are available for generated or hand-authored backfill code:

- PostgreSQL: `Postgres::copy_insert_rows` uses `COPY FROM STDIN`.
- SQL Server: `MsSql::bulk_insert_rows` batches multi-row `INSERT` statements.
- MongoDB: `Mongo::bulk_replace_documents` uses `Client::bulk_write`
  on MongoDB 8.0+.
- All three providers expose backfill checkpoint read/write helpers backed by
  `app_meta.backfill_checkpoints`.

Use checkpoints before each chunk to resume safely after process restarts. Avoid
unbounded `UPDATE` or `DELETE` statements; the migration linter treats those as
apply-blocking errors.

## Runbooks

### Checksum Mismatch Repair

1. Stop the deployment or migration job that reported the mismatch.
2. Compare the applied checksum in `app_meta.schema_migrations` with the current
   migration file. Do not edit an already-applied migration to make the checksum
   match.
3. Restore the original migration file from source control when the file was
   accidentally changed.
4. If the database is correct but the migration file history is not recoverable,
   make an explicit repair record in the incident/change ticket, then update the
   checksum row manually in the target environment only after review.
5. Put any additional schema/data change in a new forward migration.

### Failed Migration Repair

1. Inspect the `error` column in `app_meta.schema_migrations` and the database
   engine logs.
2. Determine whether the failed SQL was fully rolled back, partially applied, or
   blocked before execution.
3. Manually repair partial state using idempotent SQL. Prefer creating missing
   structures or completing a bounded backfill over deleting newly-added expand
   structures.
4. Mark the failed row ready for retry only after the database state matches the
   migration's preconditions. Usually this means deleting the failed registry row
   for that migration id or updating it under an approved repair ticket.
5. Re-run `migrate apply --json` and archive the JSON report.

### Stale Lock Handling

1. Confirm no migration job is still running for the same data source.
2. Inspect `app_meta.schema_migration_lock.locked_at`.
3. Check database sessions/process lists for active DDL or long-running backfill
   statements.
4. If the lock is stale, delete only the stale `schema_migrations` lock row.
5. Immediately run `migrate status --json` before retrying apply.

### Contract Migration Approval

1. Confirm all running application versions no longer read or write the old
   structure.
2. Confirm backups, restore points, or provider-native recovery windows are in
   place.
3. Review the contract SQL for drops, truncation, constraint removal, and lock
   behavior.
4. Run `migrate lint --phase contract --json` and resolve errors.
5. Apply with `migrate apply --phase contract --confirm-contract --json` only
   during the approved window.

### App Rollback Sequencing

1. Roll back the application first.
2. Leave expand-phase database structures in place; they are intentionally
   backward-compatible.
3. Pause or resume backfills using checkpoints rather than reverting database
   files.
4. Do not run contract migrations while rollback remains possible.
5. After the old app version is permanently retired, schedule a reviewed
   contract migration for cleanup.

## Adding Migrations

Create a new SQL file under the dialect directory and add it to
`_pkg/migrations/manifest.yaml`. Never edit a migration after it has been
applied to a shared environment; add a new forward migration instead.

Schema-specific migrations include a `schema` key in the manifest. The runner
only selects those migrations when that schema is assigned to the current data
source, including any `APP_{SCHEMA}_DATA_SOURCE_NAME` override.
