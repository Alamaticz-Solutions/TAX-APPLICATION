# Schema Design

Application shape is defined in `.appfw/model`. Treat this directory as
source code: validate it, review generated diffs, and prefer config changes over
manual generated-code edits.

The canonical contract is generated from Rust code:

```text
app_gen/src/config_contract.rs
.appfw/model/_specs/CONFIG_CONTRACT.md
.appfw/target/appfw/config_contract.json
```

Run validation whenever you edit config:

```bash
scripts/appfw validate
```

## Config Layout

```text
.appfw/model/
|-- data_sources/_res.yaml
|-- _fragments/
|-- _facets/
`-- schemas/<schema>/
    |-- _res.yaml
    |-- entity_types/*.yaml
    |-- gql_enum_types/*.yaml
    |-- rbac/*.rego
    |-- seeds/*.yaml
    `-- tests/*.yaml
```

## Schema Resources

Each schema has one `_res.yaml`:

```yaml
id: daa03056-0f95-4f9d-9fe7-465cda426297
name: crm
description: CRM schema
data_source_name: pg_primary
```

The schema directory name and `name` must match.

## Storage Ownership

Each schema should declare or intentionally inherit a storage ownership posture.
The default posture is app-owned storage: generated tables, migrations, seeds,
standard write methods, and database package artifacts are part of the product.

Use `external_read_only` when the physical data already exists and is
authoritative, such as a Microsoft Fabric SQL analytics endpoint, Snowflake
warehouse view set, SQL Server reporting database, or legacy application
database exposed for reporting. In that posture, the product owns the App
Framework schema model, access policy, services, reports, and UI, but it does
not own the source tables/views or their migrations.

```yaml
id: daa03056-0f95-4f9d-9fe7-465cda426297
name: finance_reporting
description: Finance reporting schema over authoritative warehouse views
data_source_name: fabric_fin_demo
meta:
  classification: confidential
  storage:
    mode: external_read_only
    physical_schema: dbo
```

External read-only schemas should:

- expose read/query/reporting operations only;
- map entity and property metadata to provider-native physical names when they
  differ from product-facing names;
- omit seeds and generated data-changing standard methods;
- put derived calculations in product services or query custom methods; and
- capture data classification, environment purpose, and sanitized/non-production
  status in metadata or product readiness docs.

Use routine-backed custom methods when approved provider functions or stored
procedures remain the source of truth for a calculation. Those routines are not
database objects owned by App Framework migrations.

Use `code_only` for generated/runtime schemas that should not create or modify
physical database objects:

```yaml
meta:
  storage: code_only
```

## Entity Types

Entity type files are YAML arrays. Use stable IDs and explicit generated
methods:

```yaml
- name: Account
  id: 11111111-1111-1111-1111-111111111111
  is_table: true
  standard_methods: [FindById, GetAll, Query, Create, Update, Delete]
  indexes: [name]
  props:
  - name: id
    fragment: property-primary-key-uuid
  - name: name
    is_required: true
    is_caption: true
    fragment: property-string-caption
```

Use `is_table: true` for persisted records. Non-table types are useful for
nested objects, projections, generated API shapes, and routine-backed DTO
results. For example, a custom method that calls a provider function can return
a non-table `AccountHealthResult` entity while the function itself stays hidden
behind generated `DataAccess`/provider dispatch.

## Properties

Prefer fragments for common property shapes:

```yaml
- name: email
  is_required: true
  fragment: property-string-email
  meta:
    validators:
    - name: Uniqueness
      filter: '{"email": "{{email}}"}'
      message: Email must be unique
```

Use direct `data_type` only when a fragment does not express the intent:

```yaml
- name: active
  is_required: true
  data_type: Boolean
  default_value: true
```

## Relationships

Relationships are source-level schema objects. Put them in:

```text
.appfw/model/schemas/<schema>/relationships/*.yaml
```

See [ADR 0003](adr/0003-relationship-source-of-truth.md) for the architecture
decision: schema-level relationship definitions are the source of truth for
navigation. Product developers define relationship endpoints, cardinality,
storage, and domain navigation field names in relationship config. Entity type
files define native scalar fields and physical foreign-key storage only.

The generator applies each relationship to the participating entity
perspectives and emits the runtime navigation properties. This keeps the
cardinality, storage owner, and field names in one place.

Scalar foreign keys still live on the entity that stores the value:

```yaml
- name: account_id
  is_required: true
  data_type: Uuid
  foreign_key:
    schema_name: crm
    type_name: Account
```

One-to-many relationships use the stored foreign key and generate both a
to-one field on the owner side and a to-many field on the inverse side:

```yaml
- name: account_contacts
  kind: OneToMany
  one:
    entity: Account
    field: contacts
  many:
    entity: Contact
    field: account
  storage:
    type: ForeignKey
    owner: Contact
    field: account_id
```

Many-to-many relationships use a junction entity and generate a to-many field
on each side:

```yaml
- name: opportunity_contacts
  kind: ManyToMany
  left:
    entity: Opportunity
    field: contacts
  right:
    entity: Contact
    field: opportunities
  junction:
    schema: crm
    entity: OpportunityContact
    left_key: opportunity_id
    right_key: contact_id
```

`OneToOne` uses `left`, `right`, and `storage` with the same `ForeignKey`
storage shape. Conceptually, many-to-many still projects a to-many navigation
field on each side; the junction entity is storage metadata, not a separate
product-authored navigation concept. Current generated runtime metadata may
still carry `ManyToMany` as a compatibility data type, but product config should
treat `ManyToMany` as a schema relationship kind only.

## Enums

Enum type files are YAML arrays:

```yaml
- name: Priority
  items:
  - value: Low
    caption: Low
  - value: High
    caption: High
```

Reference enums from properties:

```yaml
- name: priority
  data_type: Enum
  enum_type_name: Priority
```

## Seeds And Tests

Seed files declare entity records for database package generation. Test files
declare generated API scenarios for `api_tests`.

```bash
scripts/appfw validate
scripts/appfw generate
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

## Design Rules

- Model domain concepts first, storage details second.
- Keep names stable once generated code exists.
- Add indexes for frequently filtered, sorted, or joined properties.
- Use foreign keys for referential integrity, then add navigation properties for
  API ergonomics.
- Keep provider limitations explicit when a schema depends on a specific data
  source.
- Let validation be the source of truth for config shape.
