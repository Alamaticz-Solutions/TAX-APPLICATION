# Rego Policy Tests

This crate validates the access-policy contract consumed by the backend. Product
tests and fixtures live here; the Rego evaluation engine and shared input/result
contract come from the framework-owned `appfw-test` package.

Run:

```sh
scripts/appfw policy-test
```

## Layout

- `fixtures/policies/` contains canonical Rego fixtures.
- `src/lib.rs` contains only fixture-path glue and re-exports the framework
  policy verifier harness.
- `tests/` contains table-style integration tests for policy behavior.
- `examples/legacy/` keeps earlier scratch policies for reference.
- `../docs/archive/SQL_SERVER_RLS_NOTES.md` keeps historical SQL Server
  row-level security notes.

## Contract

The backend evaluates policies at:

```text
data.<schema_name>.<entity_type>.access
```

Input shape:

```json
{
  "schema_name": "scheduling",
  "entity_type": "resource",
  "action": "read",
  "user": {
    "tenant_id": "180000",
    "username": "alex",
    "roles": ["operations_manager"]
  }
}
```

Allowed result:

```json
{ "allow": true, "filter": {} }
```

Denied result:

```json
{ "allow": false }
```

The absence of `filter` on deny is intentional. An empty object filter means access is allowed without row-level scoping.
