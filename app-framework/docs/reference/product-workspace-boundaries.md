# Product Workspace Boundaries

This page is the current-layout ownership map for downstream application teams.
The formal architectural contract is
[Product Workspace Contract](product-workspace-contract.md). This page stays
focused on which parts of an app repo are product-owned, generated, or
framework-owned before any package extraction work begins.

The current downstream layout still contains a copied framework tree. Treat that
as an implementation detail of the present packaging model, not as permission
for product work to edit every framework-looking file in the app repo.

## Boundary Model

```text
product model config
        |
        v
generated adapters and artifacts
        |
        v
product handlers and services
        |
        v
framework DataAccess and provider runtime
```

Product teams own the model and behavior. App Framework owns repeatable
plumbing, provider semantics, generation rules, and cross-app runtime behavior.
The app manifest describes product topology; it is validated against
`.appfw/model` and does not replace schema, entity, relationship, or
data-source configuration.

For review evidence, security validation expectations, upgrade evidence, and
the target packaged dependency model, use
[Product Workspace Contract](product-workspace-contract.md).
For durable framework-owned runtime boundaries that still shape product
backends, use
[Runtime Modularity](../architecture/concerns/runtime-modularity.md) and
[Packaging](../architecture/concerns/packaging.md).

`app_gen` now resolves product and framework roots explicitly. In the framework
checkout, repository-root `scripts/appfw` defaults the product app root to
`examples/products/crm` while resolving generator source and templates from the
framework root. Downstream products use their own checkout as the app root and
the framework checkout or package as the framework root.

## Surface Contract

| Surface | Current Path | Ownership | Guidance |
| --- | --- | --- | --- |
| App topology manifest | `.appfw/manifest.yaml` | Product-owned | Name the app and list active schemas/data sources. Do not put entity, relationship, seed, or provider-environment details here. |
| Product model source | `.appfw/model` source files | Product-owned | Add schemas, entities, properties, relationships, seeds, policies, and API scenarios here. Prefer source YAML files over generated resolved outputs. |
| Generated config reports | `.appfw/target/appfw`, `.appfw/model/_specs/CONFIG_CONTRACT.md` | Generated | Read these for validation and tooling context. Do not maintain them by hand. |
| Root resolution | `scripts/appfw`, `app_gen --app-root ...` | Framework-owned CLI contract | Product teams may override root locations through documented `APPFW_*_ROOT` variables, but path semantics should stay centralized in App Framework. |
| Generated backend runtime config | `backend/config/generated` | Generated | Published data sources, schema metadata, policy projection, and optional ingress config. Change `.appfw/manifest.yaml` or `.appfw/model`, then regenerate. |
| Generated backend adapters and defaults | `backend/src/routes`, `backend/src/schemas`, generated handler `mod.rs` files, `backend/src/handlers/<schema>/generated.rs`, `backend/src/mcp/generated_operations.rs` | Generated | Review diffs, but change config/templates/generator source instead of patching these files. |
| Product handler implementations | `backend/src/handlers/<schema>/<entity>.rs` | Product-owned after creation | Implement custom methods, app-specific resolver behavior, and thin calls into services here. Existing files are preserved by generation and import generated defaults; leave ordinary generated CRUD/query behavior in `generated.rs` unless an override is explicit. |
| Product services | `backend/src/services` or another explicit service module | Product-owned | Put domain workflows, calculations, cross-entity orchestration, external integrations, and focused unit-testable business logic here. CRM-sample products include `backend/src/services/mod.rs` as the first-class service entry point. |
| Framework runtime | `appfw-runtime`; remaining `backend/src/data`, `backend/src/config`, and backend runtime facades while extraction continues | Framework-owned | Product handlers and services should normally enter through `DataAccess`, `crate::product_api`, and stable `appfw-runtime` exports; provider semantics, model metadata descriptors, capability profiles, and certification report semantics belong behind the runtime boundary. |
| Framework backend unit-test scaffolding | Framework/runtime/provider crates and reusable harnesses | Framework-owned | CRM sample and `appfw new` output should not copy `#[cfg(test)]` modules for framework plumbing or local provider `test_support` helpers. Product teams may add focused tests for product-owned handlers/services, API scenarios, and policy fixtures. |
| Provider implementations | `backend/src/data/clients/<provider>` | Framework-owned | Change only when doing provider/runtime work, and verify provider contract behavior. Product-shaped backends should carry only provider source for providers that back active schemas until provider packages fully own concrete clients; provider-certification topology entries stay in framework tests, not product runtime source. |
| Database package output | `database/_pkg` | Generated | Generated DDL, seed packages, and schema package metadata. Change model/config/generator source instead. |
| Database runner and migrations tooling | framework `database` crate, invoked from product `database/` | Framework-owned | Product-specific migration content should be explicit and reviewable under `database/_pkg`; runner behavior and provider plumbing stay upstream. |
| Product API scenario source | `.appfw/model/schemas/<schema>/tests` | Product-owned | Describe product API scenarios here. |
| Generated API test code | `api_tests/src/schemas` | Generated | Review generated Rust diffs; update scenario config instead of hand-editing generated tests. `scripts/appfw api-test` runs this surface only. |
| API test harness and provider certification | `appfw_test/src/harness`, framework-root `api_tests/src/provider_*` | Framework-owned | Product teams use the harness package; framework teams evolve it. Product repos should not carry provider certification modules; `scripts/appfw provider-test` runs this surface from the framework checkout. |
| Policy verifier harness | `appfw_test/src/policy`, product `rego_test` fixtures/tests | Shared boundary | Product repos own policy source, fixtures, and product-specific assertions. The Rego evaluator and shared access input/result contract live in `appfw-test`; `scripts/appfw policy-test` runs the product fixture crate. |
| Admin UI source | `admin_ui`, `backend/src/admin_ui.rs` | Framework-owned model-driven app shell | Keep entity screens runtime model-driven. Product branding/topology should come from explicit config rather than entity-specific UI forks. |
| PDS Health design system | `appfw_ui/pds_health` | Framework-owned frontend design-system source | Product frontends consume PDS tokens, component guidance, and generated scaffold assets from this root. CRM remains a reference/E2E fixture, not the base UI copied into products. |
| Product frontend scaffold | `frontend/package.json`, `frontend/src/app`, `frontend/src/components`, `frontend/src/scaffold`, `frontend/src/lib`, `frontend/src/styles`, `frontend/src/design`, `frontend/scripts` | Product-owned scaffold boundary | Editable in product repos, but keep it aligned with `.appfw-ui/ownership.json`, generated UI contracts, PDS tokens, and `scripts/appfw product frontend-test --json`. |
| Product mobile scaffold | `mobile/app`, `mobile/src/features`, `mobile/src/components`, `mobile/src/lib`, `mobile/src/test`, `mobile/app.json`, `mobile/eas.json`, `mobile/package.json` | Product-owned React Native boundary until the mobile generator lands | Convert HTML mobile mockups through `scripts/appfw product mobile-plan --ui-artifact <mockup.html> --json`, keep native navigation platform-specific, and align generated surfaces with `mobile/.appfw-mobile/ownership.json`. |
| Generated local dev infra | `podman-compose.yml` | Generated from product topology | Change `.appfw/manifest.yaml` and `.appfw/model/data_sources/_res.yaml`, then regenerate. Compose should reflect enabled providers rather than every provider supported by the framework. |
| Deployment overlays and env docs | pipeline/deploy overlays, env docs | Product-owned when in an app repo | Keep deployment-specific choices explicit and aligned with app topology, enabled providers, and target environments. |

## Handler And Service Rule

Generated adapters should remain thin. Product behavior should follow this path:

```text
generated route
  -> generated handler adapter
  -> product handler implementation
  -> generated handler default when product code does not override
  -> product service
  -> DataAccess
  -> configured provider
```

Handler implementation files are create-once and then preserved. That is
intentional: product code must survive regeneration. Each product handler imports
the regenerated defaults from `backend/src/handlers/<schema>/generated.rs`.
When an entity gains a new standard or custom method, the generated default is
available through that import. Product code overrides a default by defining a
local `<method>_impl` function with the same signature in the product handler
file.

## Data Boundary Rule

Product services should depend on `DataAccess` and generated schema types, not
direct provider clients. Direct provider calls are allowed only for an explicit
provider-specific product requirement, and the assumption should be documented
near the code.

`Cargo.toml` and future packaging work should determine which provider crates
are compiled into the app. Product topology config should determine which data
sources and schemas are active at runtime.

## Review Rule

When reviewing a downstream app change, ask:

- Did product intent change in model/config, handler code, service code, tests,
  or deployment config?
- Did generated output change because of that product intent?
- Did framework-owned runtime or provider code change? If so, is this really a
  framework task?
- Did generation preserve existing product handler and service code?
- Do the verification commands match the surfaces that changed?
