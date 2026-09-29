# Product Packaging Extraction Plan

This guide records the current product-packaging direction for App Framework.
It is intentionally concise. Historical extraction details and PR-by-PR notes
belong in git history or `docs/archive/`, not in the active packaging guide.

## Goal

Downstream products should look like product repositories, not copied framework
repositories.

Product apps own:

- `.appfw/manifest.yaml` topology assertions.
- `.appfw/model` product model and data-source config.
- checked-in generated backend, database package, API-test, and frontend
  artifacts.
- human-owned handlers, services, policies, frontend features, deployment
  overlays, and product tests.

App Framework owns:

- CLI, generator, templates, runtime contracts, provider packages, migration
  runners, admin/MCP shells, reusable test harnesses, release gates, and
  upgrade mechanics.

Target downstream shape:

```text
product-app/
|-- .appfw/manifest.yaml
|-- appfw.lock
|-- .appfw/model/
|-- backend/
|   |-- src/handlers/<schema>/<entity>.rs
|   |-- src/services/
|   |-- src/schemas/              generated product API types
|   `-- config/                   generated product config
|-- database/_pkg/                generated database package and migrations
|-- api_tests/                    generated product API scenarios
|-- frontend/                     product UI workspace
|-- deployment overlays
|-- podman-compose.yml            generated local infra
`-- scripts/appfw                 product wrapper around upstream CLI
```

Framework source should come from the framework checkout or versioned package
source, not from editable copied runtime/provider internals in the product app.

## Current Baseline

The packaging baseline is already strong:

- `scripts/appfw` and `appfw-cli` resolve app, framework, config, template,
  and report roots explicitly.
- `appfw-codegen` is root-aware and writes generated outputs under the app
  root.
- `.appfw/manifest.yaml` models product topology and never replaces schema,
  entity, relationship, seed, test, or provider-environment config.
- `examples/products/crm` is the canonical sample product used by
  `appfw new --profile crm-sample`.
- `appfw new` can create split-root/package-consuming apps and write
  `appfw.lock` provenance.
- The database runner is framework-owned; products keep `database/_pkg` as the
  generated database package.
- Runtime-owned helpers now cover large portions of auth, security, CORS,
  observability, query cost, filters, provider capability/certification,
  provider bridge contracts, audit, validation, admin, MCP, and generated
  operation dispatch.
- Concrete provider behavior is moving behind `appfw-provider-*` packages or
  feature-gated framework modules.

Use [Runtime Ownership Inventory](runtime-ownership-inventory.md) for the
current detailed inventory of remaining backend runtime surfaces.

## Remaining Packaging Work

| Work | Desired End State | Proof |
| --- | --- | --- |
| Provider packages | PostgreSQL, MongoDB, MS SQL Server, and Snowflake behavior lives in framework-owned provider packages or feature-gated modules. | Provider package tests plus `scripts/appfw provider-test --all --json`. |
| Runtime operation boundary | HTTP/GraphQL, MCP, and future event ingress use the same runtime invocation path. | Runtime tests and architecture contract in [Architecture](overview.md). |
| DataAccess service cleanup | Product DataAccess keeps generated adapters and product closures; reusable orchestration lives in runtime services. | Root and CRM compile/tests, generated drift check, provider certification. |
| Admin and MCP services | Runtime owns model-driven admin/MCP shells; products supply generated metadata, branding/topology, operation dispatch, and audit/data adapters. | Focused admin/MCP tests plus release posture evidence. |
| Product template slimming | `examples/products/crm/backend` and `appfw new --profile crm-sample` no longer carry copied framework runtime/provider implementation. | Boundary-check, generated drift, CRM validation/test, downstream bootstrap proof. |
| Frontend scaffold packaging | Product frontend scaffold is created through product bootstrap or a follow-on scaffold command, not copied from admin UI. | `scripts/appfw frontend-test --json` plus golden downstream CI. |

## Product Template Slimming Rules

Do not slim templates by deleting copied files until the framework-owned
contract exists and has proof.

Safe slimming sequence:

1. Move reusable behavior to runtime/provider packages.
2. Add or update product adapters that bind generated metadata, product
   handlers, product services, policy engines, and active data sources.
3. Prove framework crates and the CRM sample still exercise the same runtime path.
4. Run generation and boundary checks.
5. Remove product-local copied runtime/provider code.
6. Verify `appfw new --profile crm-sample --generate --json` still creates a
   usable product app.

No-go signals:

- Product handlers or services import provider internals directly.
- Runtime contracts require generated schema structs where a metadata adapter
  should be used.
- DataAccess bypasses policy, tenant isolation, audit, query budgets,
  pagination, provider timing, or redaction.
- `generate --check` drift is hidden instead of preserved and explained.
- Provider-visible behavior changes without provider certification evidence.

## Verification Spine

For packaging and template-slimming work:

```bash
scripts/appfw validate --json
scripts/appfw boundary-check --json
scripts/appfw generate --check --json
scripts/appfw test --fast --json
examples/products/crm/scripts/appfw validate --json
examples/products/crm/scripts/appfw boundary-check --json
examples/products/crm/scripts/appfw generate --check --json
examples/products/crm/scripts/appfw test --fast --json
scripts/appfw handoff --json
```

Add `scripts/appfw provider-test --all --json` when provider-visible behavior
changes. Add `scripts/appfw release-check --json` when the packaging slice is
release-relevant.

## Related Docs

- [Framework Packaging](framework-packaging.md): crate/package ownership.
- [Runtime Ownership Inventory](runtime-ownership-inventory.md): detailed
  remaining runtime extraction inventory.
- [Product Workspace Contract](../reference/product-workspace-contract.md): formal
  downstream ownership and review contract.
- [Generated Ownership](../start/generated-ownership.md): generated/human-owned
  artifact rules.
- [Application Lifecycle](../lifecycle/application-lifecycle.md): downstream create,
  upgrade, drift, release, and handoff proof.
