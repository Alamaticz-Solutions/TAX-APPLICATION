# Customization Decisions

Use the narrowest extension point that preserves regeneration safety.

## Decision Matrix

| Need | Preferred Extension Point |
| --- | --- |
| Add fields, relationships, indexes, generated CRUD | `.appfw/model` |
| Add a schema-level GraphQL operation for one entity | `custom_methods` in entity config |
| Implement custom behavior for generated operations | Human-owned handler impl file |
| Share business logic across handlers | `backend/src/services` or another human-owned module |
| Change generated shape for every app | `app_gen/_templates` |
| Change provider semantics | `backend/src/data/clients/<provider>` and shared query/filter contracts |
| Change config language | `app_gen/src/config_contract.rs` and `app_gen/src/validation.rs` |

## Default Order

1. Try config first.
2. Add or adjust generated custom methods.
3. Put durable app-specific code in human-owned handler or service modules.
4. Change templates only when the behavior should be generated everywhere.
5. Change provider clients when semantics differ by data source.

## Generated Custom Methods

Declare methods in entity config:

```yaml
- name: Account
  is_table: true
  custom_methods:
  - name: account_summary
    kind: Query
    args:
    - name: account_id
      arg_type: String
    return_type: serde_json::Value
```

The route/schema glue and default handler implementation are generated. The
human-owned handler file imports those defaults and can override them:

```text
backend/src/handlers/<schema>/generated.rs
backend/src/handlers/<schema>/<entity>.rs
```

The product handler file is created when missing and then preserved by
generation.

## Human-Owned Services

Create service modules when logic spans multiple handlers or needs focused unit
tests:

```text
backend/src/services/
```

This directory is not required by the generator. It is a conventional place for
human-owned business logic.

When authoring product-owned handlers/services, prefer the stable extension
imports from:

```text
crate::product_api
```

See [Product Extension API](../reference/product-extension-api.md) for the supported
runtime-facing surface.

## Template Changes

Use template changes when the generated output should change for every schema or
entity:

```text
app_gen/_templates/
```

Always verify template changes with:

```bash
scripts/appfw validate --json
scripts/appfw generate
scripts/appfw generate --check --json
scripts/appfw test
```

If `generate --check` reports generated drift, preserve the diagnostic and call
that out in the handoff.

## Provider Changes

Provider semantics live under:

```text
backend/src/data/clients/
```

Shared query intent should stay provider-neutral. Provider clients should adapt
that intent to PostgreSQL, MongoDB, MS SQL Server, or Snowflake without changing
API behavior.

## Safe Loop

```bash
scripts/appfw validate --json
scripts/appfw test
```
