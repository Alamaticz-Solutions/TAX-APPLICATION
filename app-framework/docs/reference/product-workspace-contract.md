# Product Workspace Contract

This contract defines how downstream product teams consume App Framework and
how upstream framework changes remain maintainable, auditable, and safe to
upgrade. It is the architectural agreement between:

- product application teams that own business behavior and deployment context;
- the App Framework team that owns generated platform behavior; and
- platform, security, and operations teams that own production controls.

The current implementation may still place framework source inside a copied
application checkout. That is a packaging state, not the product ownership
model. Product teams should behave as if the framework is already an upstream
versioned dependency and only edit the product surfaces named here.

For a one-page Step 0 packaging baseline, see
[Packaging Boundary Matrix](../architecture/packaging-boundary-matrix.md).

## Contract Goals

- Product intent is explicit in product-owned config, handlers, services, tests,
  topology, and deployment overlays.
- Generated artifacts are reproducible, reviewable, and replaceable.
- Human-owned extension points survive regeneration and framework upgrades.
- Every schema and data source is declared explicitly; there is no implicit
  primary schema or primary data source.
- Provider behavior is a framework contract, while active provider topology is
  a product contract.
- Framework upgrades produce traceable evidence: changed surfaces, generated
  drift, test results, lock/provenance updates, and human review points.
- Security and validation evidence can be traced from source change to build,
  generated artifact, deployment, and operating controls.

## Required Downstream Shape

Current copied-framework apps and future packaged apps should converge on this
logical shape:

```text
product-app/
|-- .appfw/manifest.yaml
|-- appfw.lock
|-- .appfw/model/
|-- backend/
|   |-- config/generated/
|   |-- src/handlers/<schema>/<entity>.rs
|   |-- src/handlers/<schema>/generated.rs
|   |-- src/services/
|   |-- src/schemas/
|-- database/
|   `-- _pkg/
|-- api_tests/
|-- frontend/ or product UI workspace
|-- mobile/ or product mobile workspace
|-- deployment overlays and environment docs
`-- podman-compose.yml
```

In the current repository layout, framework-owned generator, runtime, provider,
and harness code may still be present beside those product files. Product work
should not treat those framework-owned paths as application customization
surfaces.

## Ownership Contract

| Surface | Owner | Contract |
| --- | --- | --- |
| `.appfw/manifest.yaml` | Product | Names app identity and active topology: schemas, data sources, provider names, and schema roles. It does not define entities, relationships, seeds, credentials, or provider internals. |
| `.appfw/model` source files | Product | Source of truth for schemas, entities, properties, relationships, policies, seeds, data-source config, sync descriptors, and API scenarios. |
| `backend/src/handlers/<schema>/<entity>.rs` | Product | Human-owned extension files. Implement custom behavior here and import generated defaults. Generation must preserve existing files. |
| `backend/src/services` | Product | Durable domain workflows, calculations, external integration adapters, and focused unit-testable business behavior. |
| deployment overlays, environment docs, product UI, product mobile app | Product | Product-specific runtime choices, release configuration, web experience, and native mobile experience. |
| `backend/src/handlers/<schema>/generated.rs` and generated `mod.rs` files | Generated | Regenerated defaults and wiring. Review diffs, but change config/templates/generator source instead of editing by hand. |
| `backend/config/generated`, `backend/src/schemas`, `backend/src/routes`, generated `database/_pkg` package files, `api_tests/src/schemas`, `podman-compose.yml` | Generated | Checked-in artifacts that make product changes reviewable. Regenerate from product config and topology. Product-authored migrations may live under `database/_pkg/migrations` when created through the migration workflow. |
| `app_gen/src`, `app_gen/_templates`, root CLI, database runner, provider clients, API test harness | Framework | Upstream framework behavior. Product teams consume this behavior; framework teams change and certify it. |
| `appfw-runtime`; remaining `backend/src/data`, provider clients, QueryIR, auth, app config loading, and admin/MCP runtime internals while extraction continues | Framework | Shared runtime contract. Product code should usually enter through `DataAccess`, generated public operation surfaces, and stable `appfw-runtime` exports. |

## Do And Do Not Touch Rules

When a file looks generated or framework-owned, check ownership before editing:

```bash
scripts/appfw explain ownership <path>
scripts/appfw manifest --json
```

Use `manifest --json` only when `.appfw/target/appfw/artifacts.json` already
exists. Do not run generation only to discover ownership.

Product teams should edit:

- `.appfw/manifest.yaml` for app identity and active topology.
- `.appfw/model` for schemas, relationships, policies, seeds, data-source
  config, sync descriptors, and API scenarios.
- `backend/src/handlers/<schema>/<entity>.rs` for explicit handler overrides.
- `backend/src/services` for durable product workflows.
- `database/_pkg/migrations` only through `scripts/appfw migrate new`.
- product `frontend/` workspaces, deployment overlays, and environment docs.
- product `mobile/` workspaces that follow
  [React Native Mobile App Contract](../frontend/mobile-react-native.md).

Product teams should not edit:

- `app_gen/src`, `app_gen/_templates`, root CLI code, provider clients, QueryIR,
  auth/runtime internals, or API-test harness code from a product branch.
- generated backend routes, generated schema modules, generated handler default
  modules, generated backend runtime config under `backend/config/generated`,
  generated API scenario output, generated database package baseline files, or
  `podman-compose.yml` by hand.
- backend-hosted `admin_ui` source to build a product app. Product frontends are
  separate user experiences and should consume generated contracts instead.
- local `.env` files, secrets, bearer tokens, tenant data, PHI/ePHI, connection
  strings, private keys, or credentials.

If a generated file must change, change the product config, topology, template,
or generator source that owns it and run the appropriate generation loop. If a
framework-owned file must change for a product request, split that into an
upstream framework branch with framework verification and downstream product
upgrade evidence.

## Manifest And Topology Contract

The manifest is a topology file, not an application model replacement.

Required current fields:

```yaml
version: 1
app:
  name: product-app
  display_name: Product App
topology:
  data_sources:
  - name: mssql_primary
    provider: MsSqlServer
    role: transactional
  schemas:
  - name: system
    data_source_name: mssql_primary
    role: framework
  - name: product
    data_source_name: mssql_primary
    role: product
```

Rules:

- List every active schema and every active data source explicitly.
- Do not add `primary_schema`, `primary_data_source`, or implicit defaults.
- Keep credentials and environment-specific connection details in data-source
  environment config or platform secret systems, not in the manifest.
- Keep entity shape, relationships, sync descriptors, seeds, tests, and
  generated API behavior in `.appfw/model`.
- Treat schema-level relationship files as the canonical source for navigation
  per [ADR 0003](adr/0003-relationship-source-of-truth.md): entity files own
  scalar fields and physical FK storage, while `app_gen` generates `NavToOne`,
  `NavToMany`, and junction-backed to-many navigation from relationship
  definitions.
- Regenerate local dev infra after topology changes; do not hand-trim generated
  compose services.

Future governance metadata, such as risk tier, data classification, PHI/ePHI
handling, system owner, and validation register identifiers, should be added as
an explicit manifest extension only after the CLI validates it and upgrade
reports include it. Until then, keep those records in product release evidence
and deployment documentation.

## Handler And Service Contract

Generated handler defaults live under:

```text
backend/src/handlers/<schema>/generated.rs
```

Product handler files import the default module:

```rust
#[allow(unused_imports)]
pub(crate) use super::generated::<entity_module>::*;
```

Product code overrides a generated default by defining a local function with the
same `<method>_impl` signature in:

```text
backend/src/handlers/<schema>/<entity>.rs
```

Durable product behavior should then move into services:

```text
generated route
  -> generated handler adapter
  -> product handler override or generated default
  -> product service
  -> DataAccess
  -> configured provider
```

Handlers should stay thin. Services should hold calculations, orchestration,
external integration calls, and unit-testable business rules.

For stable product-owned imports, use
[Product Extension API](product-extension-api.md). Product handlers and services
should prefer `crate::product_api` abstractions over provider client internals.
Product-owned handler files should delegate ordinary generated CRUD/query
operations through their `super::generated::<entity>` import; keep code in those
files for custom methods or explicitly marked standard-operation overrides.

## Data And Provider Contract

Product apps own their App Framework schema model, storage ownership posture,
access policy, product services, UI, and domain behavior. They own migrations,
seeds, and generated write paths only for app-owned schemas. For external
authoritative schemas, such as warehouse views or Fabric SQL analytics
endpoints, the product owns the model and reporting behavior while the upstream
platform owns the physical objects and data lifecycle.

App Framework owns provider semantics, provider certification, QueryIR
compilation, access-filter application, error normalization, provider
observability, and the generation rules that enforce each schema's storage
posture.

Product code should depend on:

- generated schema types;
- product services;
- `DataAccess`; and
- documented handler/service extension APIs.

Product code should not call provider clients directly unless the product has a
documented provider-specific requirement and accepts the portability and testing
cost.

Provider crates and features are a packaging concern. Product topology decides
which configured data sources are active at runtime; Cargo dependencies and
features decide which provider implementations are available to compile.

## Security And Validation Contract

Product apps built with App Framework must be able to produce evidence for
secure SDLC, validation, and operational review. The framework supports that
evidence, but the product team remains accountable for product-specific
completion.

Minimum product obligations:

- Define the approved purpose, scope, owners, data types, and integration shape
  before production use.
- Assign and periodically review risk tier and data classification outside the
  codebase until manifest governance metadata is validated by the CLI.
- Use enterprise identity mechanisms for deployed environments; local bypasses
  must stay limited to documented local or provider-certification modes.
- Store secrets in approved secret systems. Do not commit secrets, tokens,
  passwords, private keys, connection strings, or sensitive local `.env` files.
- Prevent PHI/ePHI and secrets from entering logs, diagnostics, GraphQL errors,
  migration output, handoff artifacts, or generated reports.
- Keep source, configuration, pipeline definitions, and deployment overlays in
  version control under approved repository controls.
- Use pull requests for protected branches, with review, test evidence,
  security impact notes, and deployment/rollback considerations.
- Ensure generated artifacts and release artifacts are traceable to source
  commits, review records, build IDs, and immutable release identifiers.
- Run the implemented SCA, secret, PHI log, and SBOM gates for release, and
  satisfy the security-assurance decision gate with DAST, SAST/ASVS
  traceability, provenance, and signing evidence, or with formal risk
  acceptance that names the owner, approver, expiry, rationale, compensating
  controls, and follow-up plan for each missing category.
- Re-validate after major application releases, architectural changes, provider
  changes, material security findings, integration failures, or regulatory
  scope changes.

Framework obligations:

- Deny by default where policies are missing outside approved local modes.
- Preserve tenant isolation and access filters before provider execution.
- Normalize provider errors before API serialization.
- Keep provider certification executable and provider capability status
  explicit.
- Keep generated ownership and upgrade reports machine-readable.
- Keep local test/auth bypasses fail-closed outside allowed modes.
- Keep logs and diagnostics redacted by design.

Platform and operations obligations:

- Provide approved secret injection, identity, logging, monitoring, artifact,
  and deployment controls.
- Enforce environment segregation and production access controls.
- Retain release, validation, monitoring, and exception evidence in approved
  systems of record.

## Required Review Evidence

Every downstream product PR should answer:

- What product intent changed?
- Which generated artifacts changed because of that intent?
- Which human-owned handler/service/test files changed?
- Did any framework-owned runtime, provider, generator, or harness code change?
- Which commands were run, and which live checks were skipped?
- Does the change affect auth, access filters, tenant isolation, audit,
  logging, secrets, PHI/ePHI handling, provider semantics, or migrations?
- What deployment, rollback, and re-validation actions are required?

Recommended evidence bundle:

```bash
scripts/appfw validate --json
scripts/appfw generate --check --json
scripts/appfw test
scripts/appfw handoff --json
```

Evidence by change type:

| Change Type | Required Evidence |
| --- | --- |
| Config-only product intent | Validation JSON, boundary report when handlers/services are involved, focused tests, handoff JSON that states generation was not intended. |
| Generated artifact cascade | Validation JSON, `scripts/appfw generate`, `generate --check --json`, artifact provenance, generated diff review, tests, handoff JSON. |
| New app bootstrap | Profile listing JSON, `appfw new` JSON, selected profile/template/overlay, bootstrap validation, `appfw.lock`, handoff JSON. |
| Framework upgrade | Initial and final `upgrade --json`, generated diff and drift-check JSON, `appfw.lock` refresh, tests, API-test result or skipped reason, migration/rollback notes. |
| Migration or provider topology | `migrate plan --json`, `migrate lint --phase all --json`, `migrate drift --json` when data sources are reachable, `migrate rollback-guide --json`, topology report. |
| Product frontend | Generated/API contract source used, typed-client/auth/tenant evidence, product frontend typecheck/lint/test/build output, accessibility or screenshot evidence when useful, handoff JSON. |
| Framework/provider/runtime release | Provider certification JSON, `release-check --json`, security impact notes, compatibility or migration notes, retained release artifacts. |

Lifecycle gates make that evidence executable. A product PR does not need every
row, but every row it touches should have the matching command bundle or a
specific skipped-check reason in handoff.

| Lifecycle Step | Command Bundle | Review Must Record |
| --- | --- | --- |
| Create | `scripts/appfw new --list-profiles --json`; `scripts/appfw new <target> --from current --profile <profile> --generate --json`; `scripts/appfw validate --json`; `scripts/appfw handoff --json` | Selected profile, template, overlay, framework source, generated artifact choice, and `appfw.lock`. |
| Customize | `scripts/appfw explain ownership <path>`; `scripts/appfw topology --json`; `scripts/appfw validate --json`; `scripts/appfw boundary-check --json`; focused tests; `scripts/appfw handoff --json` | Product-owned files changed, generated files intentionally unchanged or regenerated, and extension boundaries intact. |
| Generate | `scripts/appfw validate --json`; `scripts/appfw generate`; `scripts/appfw generate --check --json`; tests; `scripts/appfw handoff --json` | Generated diff, artifact provenance, drift-check result, and any unexpected generated cascade. |
| Test | `scripts/appfw test --fast` or `scripts/appfw test`; `scripts/appfw api-test` when services are running | Unit/compile/API result, unavailable live services, and risk-based skipped checks. |
| Release | `scripts/appfw migrate plan --json`; `scripts/appfw migrate lint --phase all --json`; `scripts/appfw migrate drift --json` when reachable; `scripts/appfw migrate rollback-guide --json`; `scripts/appfw handoff --json` | Data classification, deployment and rollback notes, provider drift, and re-validation plan. |
| Upgrade | `scripts/appfw upgrade --json`; generate/check/test loop; `scripts/appfw lock --write`; final `scripts/appfw upgrade --json`; `scripts/appfw handoff --json` | Initial/final upgrade reports, lock refresh, generated drift decision, migration notes, and extension-point preservation. |
| Drift | `scripts/appfw generate --check --json`; `scripts/appfw migrate drift --json` when reachable | Owner, reason, affected files/providers, and next action for each drift report. |
| Handoff | `scripts/appfw handoff --json` after the final edit and verification pass | Final changed surfaces, verification artifact status, skipped checks, and remaining risks. |

The fast docs gate writes
`target/appfw/lifecycle-evidence-checklist.json` with the same lifecycle rows,
required commands, evidence artifacts, and CI coverage notes. Attach that
artifact for lifecycle-proof PRs, then add the product-specific command outputs
for every touched row.

For new application bootstrap PRs, also attach the profile selection and
provenance evidence:

```bash
scripts/appfw new --list-profiles --json
scripts/appfw new <target> --from current --profile <profile> --json
cd <target>
scripts/appfw validate --json
scripts/appfw handoff --json
```

Reviewers should be able to see the selected profile, whether generated
artifacts were written during bootstrap, the framework checkout or release used,
and the resulting `appfw.lock`.

When a backend and configured data sources are running:

```bash
scripts/appfw api-test
scripts/appfw migrate rollback-guide --json
```

For provider/runtime/framework release work:

```bash
scripts/appfw provider-test --provider <provider> --json
scripts/appfw release-check --json
```

## Upgrade Contract

Framework upgrades are product operations. They should happen on explicit
upgrade branches, not incidental feature branches.

Minimum upgrade loop:

```bash
scripts/appfw upgrade --json
scripts/appfw validate --json
scripts/appfw generate
scripts/appfw generate --check --json
scripts/appfw test
scripts/appfw api-test
scripts/appfw lock --write
scripts/appfw upgrade --json
```

The first `upgrade --json` may report drift while the branch is in progress.
The final `upgrade --json` should pass before review unless the lock is
intentionally left stale and that choice is documented.

Upgrade review must include:

- framework version or commit being adopted;
- generated artifact changes;
- product-owned changes required by the upgrade;
- provider capability or security behavior changes;
- migration and rollback notes;
- skipped live tests or unavailable services; and
- updated `appfw.lock` when the migration is accepted.

## Lifecycle Acceptance Bar

A downstream product change is review-ready when:

- ownership is clear for every changed surface;
- generated output is either intentionally unchanged or regenerated and checked;
- security, policy, tenant, audit, logging, PHI/ePHI, and secret-handling impact
  is stated;
- migrations have forward-only plan, lint, drift, and rollback evidence when
  database state changes;
- product frontend changes have typed API, auth, tenant, validation,
  policy-denied, and build/test evidence;
- live checks that could not run are named with a concrete reason; and
- `scripts/appfw handoff --json` reflects the final changed surfaces and
  remaining risks.

## Packaging Direction

The contract above is intentionally stricter than the current copied-framework
layout. The packaging target is:

- product repos own product config, product code, generated output, product
  tests, topology, deployment overlays, and `appfw.lock`;
- framework crates own runtime, providers, codegen, CLI, test harness, and
  templates; and
- downstream `Cargo.toml` files depend on versioned framework packages or
  approved git tags instead of carrying framework internals as product source.

See [Framework Packaging](../architecture/framework-packaging.md) for the current package map
and extraction order. Product automation that invokes generation should use the
crate-root API documented in [Codegen API](codegen-api.md), not private
generator modules. Product command workflows should use the packaged
`appfw-cli` command surface documented in [CLI Reference](cli.md), not hard-coded
repo-local shell paths.

Until package extraction is complete, use this contract to decide whether a
change belongs in the upstream framework branch or the downstream product app
branch.
