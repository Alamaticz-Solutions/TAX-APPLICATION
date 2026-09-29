# Data Integrity

Data integrity is enforced in layers: config validation, generated GraphQL
types, application validation, and provider-specific database constraints.

## Validation Before Generation

Run:

```bash
scripts/appfw validate
```

Validation checks config shape, duplicate names, relationships, fragments,
standard methods, provider support, seeds, and API test config. The report is
written to:

```text
.appfw/target/appfw/validation.json
```

## Required Values

Use `is_required: true` for values that must be present:

```yaml
- name: name
  is_required: true
  fragment: property-string-caption
```

Generated GraphQL inputs and database packages use this metadata to enforce
required values where the provider supports it.

## Foreign Keys

Use `foreign_key` on scalar properties:

```yaml
- name: account_id
  is_required: true
  data_type: Uuid
  foreign_key:
    schema_name: crm
    type_name: Account
```

This gives the generator enough information to emit provider-specific
referential behavior and to validate navigation properties.

## Indexes

Declare indexes on entity types:

```yaml
- name: Account
  is_table: true
  indexes: [name, owner_id]
```

Use indexes for:

- Foreign keys.
- Fields used by filters and sorts.
- Values participating in uniqueness checks.
- Common reporting dimensions.

## Application Validators

Use validators in `meta` when the rule is application-facing or needs a custom
message:

```yaml
- name: sku
  is_required: true
  fragment: property-string-word-required
  meta:
    validators:
    - name: StringPattern
      expression: /^[A-Z]{2,3}-[0-9]{4,6}$/
      message: SKU format must be XX-1234 or XXX-123456
    - name: Uniqueness
      filter: '{"sku": "{{sku}}"}'
      message: SKU must be unique
```

Keep validator filters simple and provider-portable. If a validator depends on
provider-specific semantics, document that assumption near the entity.

## Concurrency And Audit Facets

Use standard facets when the entity should get cross-cutting behavior:

```yaml
- name: Account
  is_table: true
  facets: [audited, concurrency]
```

Facet behavior is generated from config and templates, so changes should be
made through `_config/_facets` or generator code rather than by editing generated
output.

## Provider Notes

PostgreSQL, MongoDB, MS SQL Server, and Snowflake do not enforce every integrity
rule the same way. The framework should keep user-facing semantics consistent,
but provider-specific constraints and generated SQL/commands may differ.

When adding provider behavior:

```bash
scripts/appfw validate
scripts/appfw generate
scripts/appfw test
```

## Testing Integrity

Prefer generated API scenarios for behavior visible through GraphQL:

```text
.appfw/model/schemas/<schema>/tests/*.yaml
api_tests/src/schemas/<schema>/
```

Run scenarios against a prepared backend:

```bash
scripts/appfw migrate
```

Run the backend in a separate terminal:

```bash
ENV_NAME=local API_PORT=8080 scripts/appfw serve
```

Then run API scenarios:

```bash
scripts/appfw api-test
```
