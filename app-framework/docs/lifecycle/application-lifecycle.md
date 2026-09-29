# Application Lifecycle And Framework Upgrades

This guide describes how enterprise teams should use App Framework to build
their own applications, and how upstream framework updates should cascade into
those applications without losing app-owned work.

For the first end-to-end product developer narrative, start with
[Product Developer Golden Path](product-golden-path.md). Use this
guide for lifecycle evidence, upgrade review, CI, and handoff depth.

## Operating Model

Treat App Framework as a platform product and each generated application as its own
application product.

```text
app-framework upstream
        |
        | release or approved template baseline
        v
application repo
        |
        | config, generated artifacts, human-owned extensions
        v
running enterprise application
```

The framework repo owns the reusable machinery:

- Generator code and templates.
- Config contract and validation.
- Runtime architecture.
- Provider query semantics.
- Root CLI behavior.
- Generated ownership rules.
- Framework documentation and release notes.

Each application repo owns the product-specific surface:

- `.appfw/model` application model.
- `.appfw/manifest.yaml` app identity and topology assertions.
- Human-owned handlers, services, policies, and integration code.
- Environment and deployment configuration.
- App-specific seeds, fixtures, and generated API scenarios.
- Checked-in generated backend, database package, and API test artifacts.

Generated artifacts should be checked into the application repo. That gives
humans, agents, CI, and code review a concrete backend diff to inspect whenever
config or framework behavior changes.

See [Product Workspace Boundaries](../reference/product-workspace-boundaries.md) for the
current-layout contract that separates product-owned model and behavior from
generated adapters and framework runtime code.
See [Product Workspace Contract](../reference/product-workspace-contract.md) for the formal
downstream ownership, security evidence, review, upgrade, and packaging
contract.
See [App Manifest](../reference/app-manifest.md) for the topology manifest contract.

The generator resolves roots explicitly. Product config and generated output
stay under the app root, while framework source, generator templates, runtime
crates, shared dev-infra assets, and CLI behavior come from the framework root.
This split-root layout keeps downstream repos product-shaped without hiding the
generated backend, database package, API tests, or extension points from product
teams. Framework-owned runners such as the database executor stay upstream and
operate against the product package surface.

## Creating A New Application

Create a new application with the root CLI:

```bash
scripts/appfw product new ../customer-crm --from current --profile crm-sample
cd customer-crm
scripts/appfw doctor
scripts/appfw product validate --json
scripts/appfw product generate
scripts/appfw product test
```

The default bootstrap profile is `crm-sample`. `appfw product new` copies a
product template, applies the selected golden downstream profile overlay,
validates the new app config, and writes `appfw.lock` for provenance. Until the
W3-E registry-mode bootstrap lane lands, generated product Cargo manifests may
still use the framework checkout or packaged handoff source selected by the
bootstrap context; do not claim approved App Framework ProGet registry
consumption from bootstrap alone. Use `--generate` when bootstrap should also
write generated backend, database package, and API-test artifacts immediately.

Profiles are listed with:

```bash
scripts/appfw product new --list-profiles --json
```

Profile source lives under:

```text
app_gen/_golden/downstream_apps
```

The first-class CRM product template lives under:

```text
examples/products/crm
```

Golden profiles are intentionally thin overlays on top of product templates.
They should encode app-facing defaults and onboarding, not fork core framework
behavior.

For agent handoff and review, preserve the bootstrap evidence rather than only
describing what happened:

```bash
scripts/appfw product new --list-profiles --json
scripts/appfw product new ../customer-crm --from current --profile crm-sample --json
cd customer-crm
scripts/appfw product validate --json
scripts/appfw product handoff --json
```

The handoff should name the selected profile, whether `--generate` was used,
the framework checkout or packaged release used for bootstrap, the dependency
source posture, and the generated `appfw.lock` path. If bootstrap uses a ProGet
registry source, the handoff must name the feed and package versions; otherwise
it should explicitly say registry-mode consumption is pending W3-E. If
bootstrap validation fails, keep the JSON/log output with the handoff instead
of recreating the app silently.

After bootstrap, the normal app development loop is config-first:

```bash
scripts/appfw product validate --json
scripts/appfw product test
```

Run generation when the change is intended to update generated outputs:

```bash
scripts/appfw product validate --json
scripts/appfw product generate
git diff
scripts/appfw product test
```

When the backend and required data sources are running, finish high-risk changes
with generated API scenarios:

```bash
scripts/appfw product api-test
```

## Golden Downstream Lifecycle Path

Use this path for new downstream apps, upgrade proof, and agent handoff. The
commands are intentionally JSON-heavy so product teams can attach
machine-readable evidence to PRs and CI artifacts.

| Stage | Commands | Evidence To Preserve |
| --- | --- | --- |
| Create | `scripts/appfw product new --list-profiles --json`; `scripts/appfw product new ../customer-crm --from current --profile crm-sample --generate --json` | Selected profile, template and overlay paths, bootstrap JSON, `appfw.lock`, whether `--generate` was used. |
| Customize | `scripts/appfw product explain ownership <path>`; `scripts/appfw product topology --json`; `scripts/appfw product validate --json`; `scripts/appfw product boundary-check --json` | Ownership decision, topology report, validation report, boundary report, changed product files. |
| Generate | `scripts/appfw product validate --json`; `scripts/appfw product generate`; `scripts/appfw product generate --check --json` | Artifact manifest, provenance report, generated diff, drift-check JSON. |
| Test | `scripts/appfw product test`; `scripts/appfw product api-test` when backend and data sources are running | Unit/compile result, generated API scenario result, skipped live-test reason when applicable. |
| Release | `scripts/appfw product migrate plan --json`; `scripts/appfw product migrate lint --phase all --json`; `scripts/appfw product migrate rollback-guide --json`; `scripts/appfw product handoff --json` | Migration plan/lint/rollback reports, handoff JSON, security and deployment notes. |
| Upgrade | `scripts/appfw product upgrade --json`; generate/test loop; `scripts/appfw product lock --write`; final `scripts/appfw product upgrade --json` | Initial and final upgrade reports, lock refresh, generated drift decisions, rollback notes. |
| Drift | `scripts/appfw product generate --check --json`; `scripts/appfw product migrate drift --json` when data sources are reachable | Drift diagnostics kept verbatim, affected generated files, provider drift report. |
| Handoff | `scripts/appfw product handoff --json` | `target/appfw/agent-handoff.json`, commands run, skipped checks, remaining risks. |

The table above is also the executable acceptance matrix for lifecycle work.
Reviewers should be able to map every lifecycle stage in the PR to a command
that ran, a JSON artifact or stdout payload, and a clear pass, fail, or skipped
reason.

| Stage | Review Gate | Blocks Review |
| --- | --- | --- |
| Create | Profile list JSON, bootstrap JSON, `appfw.lock`, validation JSON, and handoff JSON are present. | Unknown profile, missing lock, failed bootstrap validation, or omitted generated artifacts without a stated reason. |
| Customize | `explain ownership`, topology, validation, boundary-check, focused test, and handoff evidence identify product-owned changes. | Framework-owned or generated-looking edits are unexplained, boundary-check fails, or generation intent is ambiguous. |
| Generate | Validation, `generate`, `generate --check --json`, generated diff review, and tests agree. | Generated drift remains after generation, provenance is missing, or human-owned files were overwritten. |
| Test | `test --fast` or `test` runs for the risk level, and `api-test` is attached when live services are available. | Required tests fail, or live API/provider checks are skipped without a concrete environment reason. |
| Release | Migration plan/lint/rollback, drift when reachable, security impact notes, deployment notes, and final handoff are retained. | Data or security risk is unstated, rollback is not described, or migration drift cannot be explained. |
| Upgrade | Initial `upgrade --json`, generate/check/test loop, lock refresh, final `upgrade --json`, and handoff are attached. | Final upgrade report still reports unexpected drift, lock mismatch is unexplained, or app-owned extension points changed unexpectedly. |
| Drift | Generated drift and migration drift are classified as expected, blocked, unrelated, or not run. | Drift output is discarded, normalized away, or left without owner, provider, and next-action notes. |
| Migration | Plan, lint, reachable drift, rollback guide, and phase notes cover expand/backfill/contract work. | Contract cleanup lacks approval, rollback is unsafe, or provider-specific migration limits are hidden. |
| Handoff | `target/appfw/agent-handoff.json` reflects the final dirty surfaces, verification artifacts, skipped checks, and risks. | Handoff was run before the final edits, or it omits meaningful skipped checks and remaining risks. |

`scripts/appfw framework docs-check --json` emits
`target/appfw/lifecycle-evidence-checklist.json`, a machine-readable checklist
that maps the lifecycle stages above to required commands, retained artifacts,
the docs-check examples that already exercise safe steps, and the remaining
golden downstream CI gaps for write-heavy or live-provider work.

For small config-only product changes, it is acceptable to stop at
`validate`, `boundary-check`, `test --fast`, and `handoff` when generation was
not intended and the handoff says that explicitly. For framework, template, or
upgrade work, run the generation and drift checks even when no generated diff is
expected.

## Application Edit Boundaries

Application developers and coding agents should normally edit:

```text
.appfw/model source files
backend/src/handlers/<schema>/<entity>.rs
backend/src/services/
.appfw/model/schemas/<schema>/tests/
database/_pkg/migrations created through scripts/appfw product migrate new
frontend/ product workspace
deployment overlays and environment files
```

They should avoid direct edits to generated surfaces unless they are working on
the framework itself:

```text
app_gen/src
app_gen/_templates
.appfw/model generated reports and resolved outputs
backend/src/routes generated modules
backend/src/schemas generated modules
backend/src/handlers generated module/default files
backend/src/data/clients
database/_pkg generated baseline package files
api_tests/src/schemas
podman-compose.yml
```

Fast rules:

- Do change product intent in `.appfw/model`, `.appfw/manifest.yaml`,
  product handlers, product services, product tests, product frontend code, and
  deployment overlays.
- Do create database migration files through `scripts/appfw product migrate new`, then
  prove them with `migrate plan`, `migrate lint`, `migrate drift`, and
  `migrate rollback-guide` as risk requires.
- Do review generated diffs after `scripts/appfw product generate`; generated artifacts
  are checked in so reviewers can see the cascade.
- Do not hand-edit generated route, schema, handler-default, database package,
  API scenario, or compose output to make a product change pass.
- Do not fork the backend-hosted admin UI into a product frontend. Product
  frontends belong in the product `frontend/` workspace and consume generated
  contracts.
- Do not add secrets, bearer tokens, tenant data, PHI/ePHI, private keys, or
  local `.env` values to config, generated artifacts, migration output, logs, or
  handoff reports.
- Do not change provider clients, QueryIR, auth/runtime internals, generator
  source, templates, or root CLI behavior in a downstream product branch unless
  the branch is explicitly an upstream framework change.

Durable app behavior should follow the extension path:

```text
generated route -> generated handler adapter -> product handler impl/default -> service -> DataAccess
```

This keeps the generated API predictable while preserving real business logic
across regeneration.

Local compose infra follows the same rule. `podman-compose.yml` is generated
from `.appfw/manifest.yaml` plus `.appfw/model/data_sources/_res.yaml`.
Product teams add or remove local provider services by changing topology or
data-source environment config, then running `scripts/appfw product generate`.

## Recommended Repo Topology

Each application repo should consume the framework as an explicit package source
and keep a clear pointer to the upstream framework checkout or release used by
the product.

```bash
git remote -v
origin      git@company:apps/customer-crm.git
framework   git@company:platform/app-framework.git
```

The application team works from `origin`. Framework upgrades arrive from the
framework package source on explicit upgrade branches. During local development
the generated `scripts/appfw` wrapper resolves the framework checkout through
`APPFW_FRAMEWORK_ROOT`, an adjacent `../app-framework` checkout, or the
in-repo example layout.

## Upgrading From Upstream

Never apply framework updates casually on a feature branch. Use an upgrade
branch with a reviewable cascade:

```bash
git checkout -b framework-upgrade/vX.Y.Z
git -C ../app-framework fetch origin
git -C ../app-framework checkout vX.Y.Z
scripts/appfw product upgrade --json
scripts/appfw product validate --json
scripts/appfw product generate
scripts/appfw product generate --check --json
scripts/appfw product test
scripts/appfw product api-test
scripts/appfw product lock --write
scripts/appfw product upgrade --json
```

Expected generated drift should be reviewed, not hidden. The review should
answer:

- Which generated files changed because of the framework update?
- Which app-owned config or handler files changed?
- Did any human-owned extension point get overwritten? It should not.
- Did provider semantics, access filters, auth, or audit behavior change?
- Which app API scenarios prove the upgrade is safe?
- Which security, validation, deployment, rollback, and re-validation evidence
  is required for the product's risk tier and data classification?

If `generate --check --json` reports drift after generation, preserve the
diagnostic in the handoff. It usually means the generated outputs and generator
contract are not aligned yet.

The first `upgrade --json` call is an early product upgrade diagnostic; it may
exit nonzero until the generated cascade and lock refresh are complete. The
final call should pass before the branch is ready for review.

## Drift And Migration Discipline

Generated drift and database drift answer different questions. Keep both
visible during release and upgrade work.

Generated drift:

```bash
scripts/appfw product validate --json
scripts/appfw product generate
scripts/appfw product generate --check --json
```

Use this when config, templates, generator code, topology, or framework version
changes should affect checked-in backend, database package, API-test, or compose
artifacts. If the check fails, attach the JSON and explain whether the drift is
expected, blocked, or caused by an unrelated dirty worktree.

Migration drift:

```bash
scripts/appfw product migrate plan --json
scripts/appfw product migrate lint --phase all --json
scripts/appfw product migrate drift --json
scripts/appfw product migrate rollback-guide --json
```

Use this when database shape, data movement, contract cleanup, provider
topology, or release rollback behavior is in scope. `migrate drift` needs
reachable data sources; if they are unavailable, say which provider checks were
skipped and keep the rollback guide in the handoff.

Migration PRs should state the expand/backfill/contract phase, target schemas
or data sources, forward-only rollback plan, validation impact, and any data
classification or downtime constraints. Contract cleanup should wait until the
product safety window has passed and the release evidence says the older app
version no longer depends on the removed shape.

## Versioning And Release Notes

Framework releases should be explicit and app-consumable:

```text
v0.9.0
- Breaking changes
- Config contract changes
- Generated output changes
- Provider behavior changes
- Security changes
- Required migration steps
- Verification commands
```

Each app repo carries an `appfw.lock` file that records the framework version
and generator identity that produced the checked-in artifacts:

```toml
version = 2
framework_version = "0.9.0"
framework_git_sha = "..."
framework_git_branch = "..."
generator_package_version = "0.9.0"
config_contract_hash = "sha256:..."
config_contract_source_hash = "sha256:..."
template_set_hash = "sha256:..."
golden_downstream_template_hash = "sha256:..."
workflow_cli_hash = "sha256:..."
provider_capability_hash = "sha256:..."
artifact_manifest_identity_schema = "appfw.artifact_manifest_identity@2"
artifact_manifest_hash = "sha256:..."
generated_ownership_doc_hash = "sha256:..."
last_generated_at = "..."
```

This does not replace source control. It gives agents and CI a quick answer to
which framework version shaped the application. In lock version 2,
`artifact_manifest_hash` is a portable identity rather than a hash of the raw
manifest bytes. The identity qualifies each artifact as `app` or `config`,
normalizes its path relative to that configured root, and includes the root,
path, ownership, overwrite policy, content SHA-256, and optional source
SHA-256. The run-local `written` or `skipped` action must be valid but is not
part of the identity.

The `appfw.artifact_manifest_identity@2` input is a closed schema. When an
artifact manifest exists, lock creation fails rather than accepting unknown or
missing record fields, unsupported values, invalid hashes, duplicate paths,
unsafe or out-of-root paths, symlinks, non-regular files, oversized input, or
recorded content hashes that do not match current artifact bytes, or evidence
that changes while it is read. App, config, and report roots are independently
bounded and reject filesystem roots or symlink aliases. Lock output rejects
symlinked ancestors and non-regular leaves.

Write or refresh the lock after an intentional framework/generator cascade:

```bash
scripts/appfw product lock --write
scripts/appfw product upgrade --json
```

A version 1 lock requires an explicit migration because its artifact hash was
bound to raw manifest bytes and local checkout paths. Run the upgrade diagnostic,
regenerate and review generated drift, complete validation and tests, then write
the version 2 lock and rerun the upgrade check. Do not change only `version = 1`
to `version = 2`, and do not refresh the lock before the upgrade evidence is
accepted.

`scripts/appfw product upgrade --json` writes `target/appfw/product-upgrade.json`. The
report includes product identity, topology, explicit roots, lock comparison
checks, changed-surface classification, verification artifacts, and the
product-side commands needed to finish the upgrade.

## CI Strategy

Framework CI should prove reusable behavior:

```bash
scripts/appfw product validate --json
scripts/appfw product generate
scripts/appfw product generate --check --json
scripts/appfw product test
```

Application CI should prove app behavior:

```bash
scripts/appfw product validate --json
scripts/appfw product test
```

Application upgrade branches should add the generated drift check and API tests:

```bash
scripts/appfw product upgrade --json
scripts/appfw product validate --json
scripts/appfw product generate
scripts/appfw product generate --check --json
scripts/appfw product test
scripts/appfw product api-test
```

API tests require a running backend and required data sources, so they may live
in a separate integration pipeline.

Product release pipelines should retain the JSON reports that match the change
risk: validation, generated drift, migration plan/lint/drift/rollback,
frontend build/test output when a product UI changed, API scenarios when live
services are available, and the final handoff report. Framework release
pipelines additionally retain `release-check --json` and provider certification
JSON.

## Agent Handoff Checklist

When an agent creates an app change or performs a framework upgrade, the handoff
should include:

- The framework version or upstream branch used.
- The edit surface used: config, human-owned code, generator, template, or
  provider runtime.
- Whether generated artifacts were intentionally updated.
- The verification commands and results.
- Any generated drift diagnostics.
- Any skipped API tests and why they were skipped.
- Any migration plan, drift, rollback, or live-provider checks that were run or
  skipped.
- Any frontend build, typecheck, lint, test, accessibility, or screenshot
  evidence when product UI changed.
- A short do/don't-touch boundary note when the diff includes generated-looking
  files or framework-owned paths.

The CLI can emit that handoff as JSON:

```bash
scripts/appfw product handoff --json
```

The report is written to `target/appfw/agent-handoff.json` and includes changed
surfaces, generated drift indicators, verification artifact status, and
recommended next commands.

## Lifecycle Tooling Hooks

The lifecycle works today with Git, generated ownership rules, and the root CLI.
Useful first-class agent commands:

- `appfw product new` for blessed app bootstrap.
- `appfw upgrade` for structured product upgrade review.
- `appfw explain ownership` for fast agent routing.
- `appfw explain config` for config-contract help.
- `appfw explain provider` for provider-certification help.
- `appfw handoff --json` for machine-readable changed-surface and verification
  summaries.
- `appfw explain changes --from <version> --to <version>` for release review.
- `appfw.lock` for framework provenance inside every application repo.
