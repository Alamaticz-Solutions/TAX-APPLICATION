# Checklist: Adding A New Schema To This App (tax-doc-routing)

This app was bootstrapped from the `crm-sample` profile. That profile wires
several things by hand at bootstrap time that the generator does **not**
maintain automatically when you add a new schema afterward. Every item below
was a real build/runtime failure hit while adding the `tax_routing` schema —
check all of them whenever a new schema is added, before assuming
`scripts/appfw product generate` alone is sufficient.

## 1. `.appfw/model` custom_methods: `arg_type` vs `data_type`

- `props[].data_type` is a **semantic token** the generator maps to this
  schema's actual Rust representation (e.g. `Uuid` -> `String`, matching the
  `property-primary-key-uuid` fragment convention already used for every ID
  field in this schema).
- `custom_methods[].args[].arg_type` is **literal Rust syntax**, inserted
  verbatim into generated function signatures. It is NOT translated.
- Consequence: writing `arg_type: Uuid` for an ID-like custom method parameter
  produces `cannot find type Uuid in this scope`, because nothing in this
  schema ever `use`s `uuid::Uuid` — IDs are `String` everywhere.
- **Rule:** any custom_methods arg that represents an ID must use
  `arg_type: String`, not `arg_type: Uuid`, to match this schema's convention.

## 2. `backend/src/lib.rs` schema module list is NOT generated

- `backend/src/schemas/mod.rs` (generator-owned) correctly lists every
  schema, including new ones.
- `backend/src/lib.rs` has its own **separate, hand-maintained** inline
  `pub mod schemas { pub(crate) mod crm; pub mod system; ... }` block. No
  template in `app_gen` touches this file.
- Consequence: a new schema compiles inside `schemas/mod.rs` but is
  `unresolved import` everywhere else, because `lib.rs`'s own module tree
  (the one the `backend` lib target actually compiles against) never heard
  about it.
- **Rule:** every time a new schema is added, manually add
  `pub(crate) mod <schema_name>;` inside the `pub mod schemas { ... }` block
  in `backend/src/lib.rs`.

## 3. Human-owned handler files are NOT touched by regenerate

- Files under `backend/src/handlers/<schema>/*.rs` (except `generated.rs`)
  are created once, then permanently skipped by the generator
  (`[skip] ... (human_owned file already exists)`).
- Consequence: if a model bug (like the `Uuid`/`String` issue above) is fixed
  in `.appfw/model` and regenerated, `generated.rs` picks up the fix
  automatically — but the human-owned handler stub files do NOT, and must be
  hand-edited to match.
- **Rule:** after any custom_methods signature fix, grep the human-owned
  handler files for the old type/signature and fix them by hand:
  `grep -rn "Uuid" backend/src/handlers/<schema>/`.

## 4. `crm-sample` bootstrap wires ALL providers, not just Postgres

- `.appfw/model/data_sources/_res.yaml` ships with Postgres, MongoDB, MSSQL,
  Snowflake, and Neo4j entries by default.
- `.appfw/manifest.yaml`'s `topology.data_sources` list duplicates this same
  set.
- Consequence: `scripts/appfw product migrate` demands
  `MONGO_SERVICE_ACCOUNT_NAME`, then `MSSQL_SERVICE_ACCOUNT_NAME`, etc., even
  if nothing in any actual schema uses those providers.
- **Rule:** before running `migrate`, check
  `grep -n "data_source_name" .appfw/model/schemas/*/_res.yaml` — if every
  schema only uses `pg_primary`, remove the other four entries from both
  `.appfw/model/data_sources/_res.yaml` and `.appfw/manifest.yaml`'s
  `topology.data_sources`, then re-validate/regenerate. This also trims the
  unused provider containers out of `podman-compose.yml` automatically.

## 5. GraphQL route paths are kebab-case, not the literal schema name

- `backend/src/routes/<schema>.rs` mounts the GraphQL endpoint via
  `runtime_graphql_schema_routes("/<kebab-case-name>", ...)`.
- A schema named `tax_routing` (underscore) mounts at `/tax-routing`
  (hyphen) — NOT `/tax_routing`.
- This is invisible for single-word schema names like `crm`, where
  underscore and kebab-case are identical, which is why the CRM sample never
  surfaced this.
- **Rule:** when testing a new schema's endpoint, check the actual mounted
  path in the generated `backend/src/routes/<schema>.rs` file
  (`runtime_graphql_schema_routes("...")` first argument) rather than
  assuming it matches the schema's directory/config name verbatim.

## 6. `app.name` / `display_name` in `.appfw/manifest.yaml` do not auto-rename

- Bootstrapping from `crm-sample` leaves `app.name: crm-sample` and
  `display_name: CRM Sample` in the manifest even for an unrelated product.
- This propagates into generated `podman-compose.yml` container names via
  `${APP_STACK_NAME:-<app.name>}-<service>` (confirmed: renaming
  `app.name` and regenerating changed `crm-sample-postgres` to
  `tax-doc-routing-postgres` automatically).
- **Rule:** rename `app.name`/`display_name`/`description` in
  `.appfw/manifest.yaml` early, before it propagates into deployment
  artifacts, then regenerate and swap any already-running containers
  (`docker compose down` / `up -d`) so names match.

## How to use this with Claude Code

When asking Claude Code to generate or modify `.appfw/model` entity files for
this app, include this file's path in the prompt alongside the spec, e.g.:

```
Read docs/specs/<the-relevant-spec>.md for the entity/field requirements.
Read docs/runbook/new-schema-checklist.md for known generator gotchas in
this app — apply items 1, 2, 3, and 6 proactively rather than waiting for
the build to fail on them.
```

Item 5 (route path) and item 3 (human-owned handler drift) are runtime/build
checks, not something a model-generation prompt can prevent up front — verify
those after generate/build, not before.
