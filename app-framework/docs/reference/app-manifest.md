# App Manifest

The app manifest is a product-owned topology file:

```text
.appfw/manifest.yaml
```

It names the app and lists the configured schemas and data sources that make up
the app. It does not replace `.appfw/model`; it is validated against it.

## Contract

Use the manifest for topology:

```yaml
version: 1
app:
  name: operations-insight
  display_name: Operations Insight
  description: Product-owned analytics application.
topology:
  data_sources:
  - name: mssql_primary
    provider: MsSqlServer
    role: transactional
  - name: neo4j_graph
    provider: Neo4j
    role: graph-read
  schemas:
  - name: ops
    data_source_name: mssql_primary
    role: product
ui:
  product_spa:
    enabled: true
    packaging: backend-product-dist
  admin_ui:
    enabled: false
    packaging: disabled
```

Keep these details in `.appfw/model`, not in the manifest:

- Entity, property, enum, relationship, seed, and API test definitions.
- Data-source environment details such as host, port, database name, TLS mode,
  and service account names.
- Provider behavior, query semantics, and generated handler shape.

## UI Serving Posture

The manifest also records product-owned UI packaging intent for the default
single-backend-image deployment:

```yaml
ui:
  product_spa:
    enabled: true
    packaging: backend-product-dist
  admin_ui:
    enabled: false
    packaging: disabled
```

`product_spa` describes whether the product web app should be built into
`backend/product_dist`. `admin_ui` describes whether the framework admin bundle
is part of this product image. A product that enables admin UI should set
`packaging: backend-admin-dist` and provide `backend/admin_dist/index.html` in
release evidence. A product that does not ship admin UI should explicitly set
`enabled: false` and `packaging: disabled`; the framework-owned `admin_ui`
source tree alone is not evidence that every product image embeds admin UI.

## No Primary Schema

The manifest intentionally has no `primary_schema` or `primary_data_source`
field. Apps may have multiple schemas and multiple data sources. Every active
schema and data source should be listed explicitly under `topology`.

The manifest declares active topology, not physical data ownership. Put schema
storage posture such as app-owned, external read-only, routine-backed, or
code-only in `.appfw/model/schemas/<schema>/_res.yaml` metadata so validation
and generation can enforce it at the owning source surface.

## Validation

Run:

```bash
scripts/appfw topology --json
scripts/appfw validate --json
```

Validation fails when the manifest references a schema or data source that does
not exist in `.appfw/model`, omits a configured schema or data source, or
tries to assign a schema to a different data source than the schema config uses.

Validation emits the resolved topology report:

```text
.appfw/target/appfw/app_topology.json
```

Generation also uses topology plus `.appfw/model/data_sources/_res.yaml` to
emit local dev infra:

```text
podman-compose.yml
.appfw/target/appfw/dev_infra.json
```

If a downstream app should use MS SQL Server only, list only the MS SQL Server
data source in topology and keep its `compose` environment in data-source
config. Regeneration removes unused Postgres, MongoDB, Snowflake, and Neo4j
services from compose.

Neo4j topology entries are for graph read models. Do not assign generated
entity schemas to a Neo4j data source; validation rejects that because Neo4j is
not a CRUD schema host.

Those are the default paths for the copied-framework layout. Root-aware
generator runs may place reports under the configured `--report-root`.

Agents should read that report when they need app-level context, then edit the
owning source surface: manifest for topology, `.appfw/model` for model/config,
and handler/service files for product behavior.
