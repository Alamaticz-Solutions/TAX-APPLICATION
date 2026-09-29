# App Framework CLI Reference

For the first-contact command path, start with
[CLI Quickstart](../start/cli-quickstart.md). This page is the full command,
option, artifact, and JSON contract.

`appfw` is the command surface for local development, CI, and agentic
workflows. The packaged CLI lives in the `appfw-cli` crate and delegates to the
root compatibility wrapper while the command contract is promoted out of shell.

Run it from the workspace:

```bash
cargo run --locked -p appfw-cli -- product validate --json
```

Install it when the package is published or from an approved internal source:

```bash
cargo install appfw-cli
appfw product validate --json
```

The previous compatibility binary still works:

```bash
cargo run --locked --manifest-path app_gen/Cargo.toml --bin appfw -- --repo-root . product new --list-profiles --json
```

## Audience Namespaces

`appfw` has one binary and two canonical namespaces:

```bash
scripts/appfw product <command>
scripts/appfw framework <command>
```

Use `product` when building, running, validating, testing, releasing,
upgrading, or handing off a downstream product app. Use `framework` when
evolving App Framework itself: docs IA, CLI contracts, generator behavior,
runtime ingress, provider certification, framework release gates, dependency
policy, packaging, or reusable scaffolding.

Flat commands such as `scripts/appfw validate --json` remain compatibility
aliases for existing scripts. New docs, skills, prompts, and CI examples should
use the explicit namespace.

Context and lifecycle discovery:

```bash
scripts/appfw context --json
scripts/appfw lifecycle --json
scripts/appfw skills --audience product --json
scripts/appfw skills --audience framework --json
```

The context report identifies the framework root, app root, product manifest,
and recommended namespace. The lifecycle report maps product and framework
phases to skills, docs, commands, and retained artifacts.

## Current-Task Instruction Routing

Resolve bounded instruction sources and separate implementation/review profiles
before material work:

```bash
scripts/appfw framework instructions --task framework-docs-ia --role coding-agent --change-class C --json
scripts/appfw product instructions --task product-frontend --role coding-agent --change-class B --json
```

The closed task enum is:

- framework: `framework-docs-ia`, `framework-generator`,
  `framework-runtime-ingress`, `framework-provider-certification`;
- product: `product-bootstrap`, `product-schema-modeling`, `product-frontend`.

The role enum is `coding-agent`, `architect-agent`, `xo-agent`, or
`workstream-analyst`. The current manifest intentionally binds all seven
implementation tasks to `coding-agent`; another known role is a parseable
mismatch, not an implicit fallback. Change class is exactly `A`, `B`, `C`, or
`D`, subject to the route's minimum class.

Success returns one compact `appfw_instruction_route@1` JSON object with:

- exactly one producer role card and one skill;
- minimal canonical references, each with repository-relative path, heading,
  and selection reason;
- provider-neutral, pinned implementation and independent-review profiles;
- comprehensive independent review for Class C and comprehensive adversarial
  review for Class D;
- fail-closed fallback, no generic inheritance, no silent substitution, and no
  self-certification.

Unknown values, duplicate flags, missing values, namespace/task mismatches,
role/task mismatches, and below-minimum classes exit `2` with a parseable
`{"ok":false}` envelope. Output is byte-stable and contains no timestamp,
absolute path, working-directory/environment data, copied instruction text,
network result, or retained artifact write. The command reveals a route; XO or
the assigned dispatcher still owns dispatch, and human/review authorities keep
their existing decision rights.

## Delivery Mode

Delivery mode is root-only and selects a worktree-local policy projection stored
below each worktree's Git directory. Linked worktrees do not share mode state.
It does not replace instruction routing or grant release authority:

```bash
scripts/appfw mode status --json
scripts/appfw mode set accelerated --json
scripts/appfw mode set candidate --json
```

`accelerated` reports focused routing and deferred gates. `candidate` reports
enforce-candidate routing. See [Delivery Profiles](../start/delivery-profiles.md)
for the tracked policy and state/ledger rules. With both mutable files absent,
`mode status` projects the tracked `accelerated` default side-effect-free and
does not write state in a fresh primary or linked worktree. `mode set` is the
only writer and requires a clean checkout, binding the selected profile to the
exact source SHA. Half-present, malformed, or noncanonical persisted state
returns parseable `ok:false`.

Status and evidence annotation report dirty or stale source binding instead of
aborting handoff, review-brief, or change-impact diagnostics. For those three
commands, JSON stdout is byte-equal to the final retained artifact and includes
`delivery_profile`. The annotation does not alter handoff currency, review
output freshness, review depth, pre-push satisfaction, or any release/merge/risk
authority.

## Lockfile Contract

Framework workflow commands run Cargo with `--locked` so validation, generation,
tests, provider certification, and release evidence are reproducible. The
framework checkout owns one tracked root `Cargo.lock` for framework crates,
provider crates, generator tooling, and framework validation fixtures.
First-class product workspaces, including `examples/products/crm`, own their
own tracked root `Cargo.lock`.

If a checkout is missing the required lockfile, `scripts/appfw` fails before
Cargo runs and prints the exact workspace that needs:

```bash
cargo generate-lockfile
```

Today the packaged CLI dispatches `explain`, `handoff`, `lock`, `upgrade`, and
`new` through the compiled introspection binary. Packaged distributions also
include prebuilt `app_gen`, `appfw_introspect`, and `database` binaries; the
CLI and compatibility wrapper auto-discover those sibling binaries under
`bin/` so product validation, generation, migration, and introspection do not
rebuild framework crates locally. Long-running product workflows remain
available through the compatibility wrapper.

## Framework Packaging

Framework release pipelines build a ProGet-ready Cargo crate publish plan,
CLI/toolchain, binaries-only, and product-docs artifacts with:

```bash
scripts/appfw framework package --json
```

For a fast contract preview that does not build binaries:

```bash
scripts/appfw framework package --plan --json
```

The command writes `target/appfw/proget/app-framework-proget-manifest.json`
with the crate publish plan, tarball paths, and SHA-256 hashes. See
[ProGet Distribution](../release/proget-distribution.md).

## Design Goals

- One command namespace from the repository root.
- Stable exit codes for humans, agents, and CI.
- JSON output for commands that agents need to parse.
- Polished human output for interactive use without changing JSON contracts.
- No backend/database/API artifact writes during validation or drift checks.
- Clear separation between generated artifacts and human-owned extension files.

## Human Display

Human-facing commands may show a compact App Framework banner, colorized status
labels, and sectioned output. Machine-facing JSON mode never includes terminal
art or ANSI color codes.

Display controls:

| Variable | Values | Purpose |
| --- | --- | --- |
| `NO_COLOR` | any non-empty value | Disable ANSI color. |
| `APPFW_COLOR` | `auto`, `always`, `never` | Override color detection. Default is `auto`. |
| `APPFW_PROVIDER_CERTIFICATION_EXPORT_BIN` | executable path | Use a prebuilt `provider_certification_export` binary for provider reports instead of falling back to `cargo run`. Packaged framework distributions set this through their `bin/` layout; local framework agents may set it to `target/debug/provider_certification_export` to avoid waiting on a shared Cargo target lock. |

Keep new visual treatment additive: it should make `help`, `doctor`, and
interactive bootstrap flows easier for humans while preserving stable
stdout/stderr behavior for agents and CI.

## Root Model

`appfw` and `scripts/appfw` run `app_gen` with explicit roots instead of
relying on the process working directory:

| Root | Default | Purpose |
| --- | --- | --- |
| App root | `examples/products/crm` when using the framework checkout wrapper; downstream product checkout when using a product wrapper or packaged CLI | Product-owned backend, database package, API tests, manifest, and checked-in generated output. |
| Framework root | repository root | Framework checkout that provides generator/runtime source. |
| Generator root | `app_gen` | Rust generator crate and generator source. |
| Config root | `<app-root>/.appfw/model` | Product model source. |
| Templates root | `app_gen/_templates` | Framework-owned generator templates. |
| Report root | `<app-root>/.appfw/target/appfw` | Validation, topology, dev-infra, contract, IR, provenance, and artifact reports. |

The repository-root wrapper intentionally defaults product workflows to the CRM
sample app so the framework root does not behave as a product app. Packaging or
downstream upgrade workflows can override root locations with `APPFW_APP_ROOT`,
`APPFW_FRAMEWORK_ROOT`, `APPFW_GENERATOR_ROOT`, `APPFW_CONFIG_ROOT`,
`APPFW_TEMPLATES_ROOT`, and `APPFW_REPORT_ROOT`.

The packaged CLI also accepts root options directly:

```bash
appfw --app-root /path/to/product --framework-root /path/to/app-framework product validate --json
```

When run from a downstream product checkout next to `app-framework`, the CLI
uses the product checkout as the app root and discovers the adjacent framework
checkout as the framework root.

With split roots, generation reads product config and writes product outputs
under the app root, while generator source and templates are read from the
framework root. Database commands execute the framework-owned database crate
with the product `database/` directory as cwd, so the runner reads the product
`database/_pkg` package without copying runner source into the product.
`scripts/appfw generate --check` verifies split-root assumptions by running
generation in an isolated product/framework layout.

## Exit Codes

| Code | Meaning |
| --- | --- |
| `0` | Command completed successfully. |
| `1` | The requested operation ran and failed. |
| `2` | Usage error, unsupported command, or unsupported option combination. |

## Commands

### `scripts/appfw doctor`

Checks required local tools and the repository layout.

```bash
scripts/appfw doctor
scripts/appfw doctor --json
scripts/appfw doctor --database
scripts/appfw doctor --database --json
```

Required tools currently include `cargo`, `rustc`, `rustfmt`, `git`, `rsync`,
`diff`, and `shasum`. Container tools such as `podman` and `docker` are reported
as optional because not every workflow starts local data stores.

Pass `--database` to validate database readiness: selected `ENV_NAME`,
provider credentials, TLS mode, provider TCP reachability, database existence,
and schema assignment.

For plain `MsSqlServer` with `auth_mode: ntlm`, doctor requires
`MSSQL_SERVICE_ACCOUNT_NAME` and `MSSQL_SERVICE_ACCOUNT_PASS` (domain Windows
username and password). Username format: `DOMAIN\user` or `user@realm`.
Validation allows hostname or IP `db_host` for NTLM. Set
`MSSQL_AUTH_MODE=ntlm` to override YAML when needed. Secret delivery is
platform-owned; see `docs/specs/mssql-odbc-ntlm-authentication.md`.

Outside doctor, connection-only live smoke against a real Windows SQL host is
`scripts/ci/mssql-ntlm-live-smoke.sh` (not an `appfw` subcommand). Required:
`MSSQL_NTLM_HOST`, `MSSQL_NTLM_USERNAME`, `MSSQL_NTLM_PASSWORD`. Optional:
`MSSQL_NTLM_PORT`, `MSSQL_NTLM_DATABASE`, `MSSQL_NTLM_SQL` (default
`SELECT 1 AS ok`; printed with `--nocapture`), and
`MSSQL_NTLM_SMOKE_LAUNCH` (`native` default / mode **c**, or `container` /
mode **d**). Details: `docker/mssql-ad/README.md`.

### `scripts/appfw validate`

Runs config validation without generating backend, database, sync worker, or API
test artifacts. Validation does emit reports and the generated config contract,
including `.appfw/model/sync/*.yaml` descriptor diagnostics when sync
descriptors are present.

```bash
scripts/appfw validate
scripts/appfw validate --json
```

JSON output preserves the validation report fields and adds
`command:"validate"`, semantic `ok`, `requested_namespace`, and `scope`.
`product validate` and the flat compatibility alias report
`scope:"product-config"`. From the framework namespace, validation still
checks the configured/default product model against framework generator and
config-contract rules, so it reports
`scope:"framework-default-product-config"` rather than implying a full
framework test sweep.

This runs:

```bash
cd app_gen
cargo run --locked --bin app_gen -- \
  --app-root <app-root> \
  --framework-root <framework-root> \
  --generator-root <generator-root> \
  --config-root <config-root> \
  --templates-root <templates-root> \
  --report-root <report-root> \
  --validate-only
```

Generated reports:

```text
.appfw/target/appfw/validation.json
.appfw/target/appfw/app_topology.json
.appfw/target/appfw/config_contract.json
.appfw/target/appfw/config_contract.md
.appfw/target/appfw/sync_descriptors.json
.appfw/model/_specs/CONFIG_CONTRACT.md
```

With `--json`, stdout is the validation report when it exists. Diagnostic
compiler or validation output is written to stderr on failure.

### `scripts/appfw generate`

Runs the generator and writes generated backend, database, API test, and local
dev-infra outputs.

```bash
scripts/appfw generate
scripts/appfw generate --json
scripts/appfw product generate --target mobile-rn --json
```

This runs:

```bash
cd app_gen
cargo run --locked --bin app_gen -- \
  --app-root <app-root> \
  --framework-root <framework-root> \
  --generator-root <generator-root> \
  --config-root <config-root> \
  --templates-root <templates-root> \
  --report-root <report-root>
```

With `--json`, stdout is `.appfw/target/appfw/artifacts.json` when generation
finishes successfully.

`scripts/appfw product generate --target mobile-rn --json` is the current U5
mobile generator target. It emits the framework-owned React Native contract
bridge, PDS native token bridge, reusable generated entity screen, generated
policy-aware GraphQL data client, all primary entity Expo Router route shells,
ownership metadata, and scaffold manifest, then delegates to `product
mobile-test`. It does not yet emit product-owned native workflow screens,
endpoint/auth runtime wiring, or simulator/device evidence. The retained
`.appfw/target/appfw/mobile-rn-generate.json` report includes
`generator_status:"all-entity-route-shells-emitted"` and `write_actions` for
the generated files, so agents can distinguish the implemented generated route
shell/data-client binding slice from future native workflow runtime wiring. The
canonical generated mobile input is
`mobile/src/generated/appfw-mobile-contract.ts`; Web-generated TypeScript is
migration or diagnostic evidence only and must not become a mobile source
contract.

Generation also emits:

```text
.appfw/target/appfw/generator_ir.json
.appfw/target/appfw/normalized_config.json
.appfw/target/appfw/sync_descriptors.json
.appfw/target/appfw/sync_worker_plan.json
.appfw/target/appfw/dev_infra.json
.appfw/target/appfw/artifact_provenance.json
```

`generator_ir.json` is the normalized, relationship-resolved generator IR. The
legacy `normalized_config.json` path is still written for compatibility.
`sync_descriptors.json` is the normalized, validation-gated
`.appfw/model/sync/*.yaml` descriptor set used by future sync worker generation.
Each object map records per-record `provenance` plus an `echo_loop` policy, so
materialized SaaS projections retain source lineage and can reject records whose
source-origin marker matches the app's local write-back origin.
`sync_worker_plan.json` is the generated sync worker planning report behind
`backend/config/generated/sync_workers.yaml`; the checked-in YAML keeps
`enabled: false` and records activation gates until worker runtime code,
provider live certification, checkpoint persistence, idempotent projection
writes, freshness/readiness, and deployment wiring are present.
`dev_infra.json` summarizes the compose services generated from app topology
and data-source config plus the `appfw_runtime` dependency source read from the
product backend Cargo manifest. Path dependencies retain the local Framework
runtime bind. A complete, validated sibling observability tree retains the
observability services; an absent sibling is omitted with a truthful rationale,
while a present incomplete or symlinked tree fails closed. Git and registry
dependencies omit checkout-bound assets and record that rationale in the report.
`artifact_provenance.json` records config, template, generator-source, and
artifact-manifest hashes so generated output can be reviewed as a
tamper-evident build product. Each artifact-manifest digest is reconciled from
the final regular-file bytes; missing or non-regular recorded artifacts stop
generation.

### `scripts/appfw generate --check`

Runs the deterministic generation check in a temporary workspace.

```bash
scripts/appfw generate --check
scripts/appfw generate --check --json
scripts/appfw product generate --target mobile-rn --check --json
```

This command verifies that:

- Existing human-owned handler implementation files are not overwritten.
- Generated backend config files match the checked-in backend config output.
- Generated backend files match the checked-in backend output.
- Generated database package files match the checked-in package output.
- Generated API test schema files match the checked-in API test output.
- Generated `podman-compose.yml` matches app topology and data-source config.

The JSON response includes `timing_artifact`, pointing at
`target/appfw/generate-check-timing.json`. That retained artifact records
phase timings for root copy, workspace preparation, `app_gen`, protected-handler
drift, framework-root mutation detection, each generated output comparison, and
the slowest phase. Use it when `docs-check` reports
`core-generated-drift-command-examples` as the slowest subcheck; that group is a
single command, so its next optimization should come from the generate-check
phase evidence rather than another docs-check split.

For `--target mobile-rn --check`, the command compares the generated mobile
contract, reusable generated entity screen, generated policy-aware GraphQL data
client, all generated entity route shells, PDS native token bridge, ownership
metadata, and scaffold manifest against the checked-in product workspace without
rewriting them. It writes `.appfw/target/appfw/mobile-rn-generate.json`, records
`action:"verified"` for matching generated files, delegates to `product
mobile-test`, and returns non-zero when generated mobile drift or missing
scaffold evidence is detected. Product-owned native workflow composition,
endpoint/auth runtime wiring, simulator/device observations, and a future
source-bound candidate checker remain future U5 work. `mobile-test` and its
compatibility-named inputs stay non-authoritative with candidate/release fields
false; named humans own distribution and release.

Use this after generator or template changes and before committing generated
output. If the repository already contains generated drift, this command reports
that drift; do not treat it as a first-run health check.

### `scripts/appfw config-contract`

Emits the config contract and prints the contract locations.

```bash
scripts/appfw config-contract
scripts/appfw config-contract --json
```

Use `--json` when an agent needs the machine-readable contract.

### `scripts/appfw topology`

Validates `.appfw/manifest.yaml` against `.appfw/model` and prints the app
topology report. The app manifest is topology only; it does not replace schema,
entity, relationship, seed, or data-source environment config.

```bash
scripts/appfw topology
scripts/appfw topology --json
```

The report is emitted at:

```text
.appfw/target/appfw/app_topology.json
```

Use this when an agent needs to understand app identity, active schemas, active
data sources, and provider topology before choosing an edit surface.

### `scripts/appfw manifest`

Prints the latest artifact manifest.

```bash
scripts/appfw manifest
scripts/appfw manifest --json
```

The manifest is generated by `scripts/appfw generate` at:

```text
.appfw/target/appfw/artifacts.json
```

Each artifact record includes ownership, overwrite policy, action, and SHA-256
hashes for the emitted file and copied source file when applicable.

### `scripts/appfw explain`

Explains platform ownership, config contract entries, and provider
certification areas. These commands are read-only and support `--json`.

```bash
scripts/appfw explain ownership backend/src/routes/mod.rs
scripts/appfw explain ownership .appfw/model/schemas/crm/entity_types/account.yaml --json
scripts/appfw explain config relationships.storage.owner
scripts/appfw explain config entity_type.indexes --json
scripts/appfw explain config .appfw/model/schemas/crm/entity_types/account.yaml --json
scripts/appfw explain provider scalar_filters
scripts/appfw explain provider pagination --provider postgres --json
scripts/appfw explain provider many_to_many_projection --provider mongo --json
scripts/appfw explain provider-sdk --json
```

`explain ownership` uses the generated artifact manifest when it exists, then
falls back to static generated-boundary rules. It does not run generation just to
answer ownership.

`explain config` reads:

```text
.appfw/target/appfw/config_contract.json
```

Run `scripts/appfw validate --json` first if the contract report is missing.

`explain provider` exports the Rust-owned provider capability matrix and reports
status, evidence, live contract names, and the matching `provider-test` command.

`explain provider-sdk` exports the framework rules for adding a fifth provider
safely: provider identity, QueryIR semantics, security/error normalization,
observability, and certification requirements.

### `scripts/appfw handoff`

Emits an agent handoff report for the current checkout.

```bash
scripts/appfw handoff
scripts/appfw handoff --json
```

The report includes the current branch and commit, changed files classified by
ownership surface, generated drift indicators, verification artifact status, and
recommended next commands. JSON mode also writes:

```text
target/appfw/agent-handoff.json
```

Use this at the end of agent work and framework upgrade branches so the next
reviewer can see changed surfaces, verification evidence, and remaining drift in
one machine-readable artifact.
JSON output includes the resolved product identity and explicit root paths, and
verification artifact checks use the configured report root.
Each changed surface includes raw Git status plus normalized `change_kind`,
`index_status`, and `worktree_status` fields so agents do not need to parse
porcelain status codes.
When a retained `target/appfw/agent-handoff.json` is present in a release
artifact directory, `scripts/ci/release-evidence-check.sh` validates it as
handoff evidence: the timestamp must be fresh, the recorded branch and SHA must
match the current checkout, drift counts must match changed surfaces, and the
handoff verification entries for release check, provider certification, and
operations certification must match the retained evidence reports.

### `scripts/appfw framework change-impact`

Retains a report-only branch impact summary for broad, sensitive, or
multi-domain framework work. Use it before PR review when a lane may require
human review, an integration branch, or a broader proof loop.

```bash
scripts/appfw framework change-impact --json
scripts/appfw framework change-impact --base origin/main --head HEAD --json
```

JSON mode writes:

```text
target/appfw/change-impact.json
```

If the requested Git refs cannot be resolved, have no merge base, or the Git
diff/status commands fail, the command exits nonzero and retains the same
artifact with `ok:false` and `git_errors` instead of emitting a green partial
impact report.

The report includes:

- changed files, untracked file count, and non-generated line delta;
- ownership buckets and domain count across docs, CLI/scripts, CI, generator,
  runtime, providers, product examples, frontend/mobile, dependencies, and
  retained evidence;
- sensitive-surface flags such as auth/policy, release/governance,
  provider/SaaS capability, AI/chat egress, MCP/Kafka ingress, generated
  templates, CI gates, and mobile secure-storage surfaces;
- `change_class` (`A`, `B`, `C`, or `D` in the current report-only classifier),
  plus `requires_human_review` and `requires_integration_branch`;
- recommended verification commands for the touched surfaces; and
- optional skipped-check reasons passed with `--skipped-check <reason>`.

The command does not enforce or approve anything. It gives the PR Review Agent
and human reviewer a machine-readable impact package; release authority still
comes from the managed release gates and retained release artifacts.

### `scripts/appfw sra-package`

Retains a report-only PDS Health Security Risk Assessment package scaffold.
Use it when a framework or product change needs SRA preparation for production
deployment, vendor/SaaS integrations, AI/MCP/Kafka surfaces, sensitive data, or
significant governance/security changes.

```bash
scripts/appfw framework sra-package --all-products --json
scripts/appfw product sra-package --json
scripts/appfw product sra-package --project-name "Example App" --business-purpose "..." --owner "..." --technical-owner "..." --json
```

Framework scope writes:

```text
target/appfw/sra-package.json
target/appfw/sra-package.md
```

Product scope writes:

```text
.appfw/target/appfw/sra-package.json
.appfw/target/appfw/sra-package.md
```

The JSON report includes:

- framework reusable-control responsibilities and product-specific SRA
  obligations;
- required SRA package sections for owners, data-flow diagrams, infrastructure
  diagrams, security criteria, identity/access, logging, secrets, vendors, risks,
  and LogicGate/SRA linkage;
- current product perspectives when `--all-products` is used, including
  manifest-declared schemas, data sources, ingress entries, and classification
  signals from product model files;
- evidence inventory for framework and product artifacts; and
- `missing_human_inputs`, `sra_ready:false`, and next actions when the package
  is not ready to submit.

The Markdown narrative is generated from the same report data for human review.
It summarizes scope, readiness, missing inputs, the required SRA checklist,
framework/product responsibilities, product perspectives, evidence inventory,
and next actions. It is still preparation evidence only, not SRA approval.

The command does not approve an SRA and must not be used to hide
product-specific data classification, vendor, service-account, secret,
diagram, logging, monitoring, or risk-register gaps behind framework controls.

### `scripts/appfw review-brief`

Retains the PR review contract for the Framework or Product PR Review Agent.
Focused mode is the default. Use `--comprehensive` when the human wants the
reviewer to inspect the whole branch or PR, retained evidence, and
cross-surface alignment rather than only the current changed surfaces. Use
`--auto-depth` as the normal pre-push selector; it keeps focused mode for narrow
ordinary branches and requires comprehensive review for integration, broad,
governance, CLI/CI, review-harness, release/security, generator,
runtime-contract, or other sensitive surfaces.

```bash
scripts/appfw framework review-brief --json
scripts/appfw framework review-brief --auto-depth --json
scripts/appfw framework review-brief --comprehensive --json
scripts/appfw product review-brief --json
scripts/appfw product review-brief --auto-depth --json
scripts/appfw product review-brief --comprehensive --json
```

JSON mode writes:

```text
target/appfw/review-brief.json
```

For framework scope, the report points reviewers at
`/framework-pr-review` or `/framework-pr-review --comprehensive` and
`docs/start/pr-review-agent-harness.md`. For product scope, it points reviewers
at `/product-pr-review` or `/product-pr-review --comprehensive` and
`docs/start/product-pr-review-agent-harness.md`.

The report includes `review_depth`, `review_depth_instruction`, base/head refs,
branch, SHA, changed files, `changed_files_for_handoff`, handoff/docs-check
artifact status, required output sections, source-of-truth lens,
alignment-drift responsibility, proof loop, and a recommended prompt. Product
scope uses `changed_files_for_handoff` to normalize repo-relative branch paths
back to product-root-relative handoff paths before comparing changed surfaces.
The required review output starts with
`Recommendation Summary`, which must include final status (`GO`,
`GO WITH CONDITIONS`, `NO-GO`, or `DEFER`) plus severity counts for `blockers`,
`critical`, `important`, `should_address`, and `nice_to_address`. It is
report-only: pushing still requires an independent review output plus explicit
human approval or the standing push approval policy from
`docs/start/agent-role-cards.md`.
When `--review-output <path>` is supplied after a comprehensive review,
`pre_push_status.satisfied` is true only when the handoff artifact matches the
current branch/SHA and changed-surface snapshot, and the review output contains
all required review sections with non-empty section content. The `Recommendation
Summary` must include an explicit `Final status:` value from the allowed status
set and numeric counts for every severity bucket. Supplying `--approver <name>`
records explicit human approval. Non-`GO` statuses (`GO WITH CONDITIONS`,
`NO-GO`, or `DEFER`) must include an `Attention Items` section with one or more
Markdown bullets naming the conditions, blockers, evidence gaps, or review
limits the human must inspect. Without `--approver`, the gate may still be
satisfied by standing push approval when the status is `GO` or
`GO WITH CONDITIONS` and blocker/critical counts are zero. The pre-push guard
prints the same concise review status summary in human-readable output: final
status, severity counts, conditions-captured value, required depth, Attention
Items when present, and a Markdown link to the retained review output. The
retained report records that the pre-push review gate has been completed for the
current branch state.

### `scripts/appfw review-performance`

Retains oversight guidance for auditing a prior PR review when the human invokes
`/check-pr-review-performance` or says `check PR review performance`. Use
`--auto-depth` when auditing the same review depth selected for pre-push.

```bash
scripts/appfw framework review-performance --json
scripts/appfw framework review-performance --auto-depth --json
scripts/appfw framework review-performance --comprehensive --json
scripts/appfw product review-performance --json
scripts/appfw product review-performance --auto-depth --json
scripts/appfw product review-performance --comprehensive --json
scripts/appfw framework review-performance --review-output review.md --json
```

JSON mode writes:

```text
target/appfw/review-performance.json
```

When `--review-output <path>` is provided, the command scans for the required
review and oversight sections. It does not judge review quality by itself; it
records the oversight contract and evidence pointers so a coordinating agent
thread or human can assess missed risk, evidence discipline, alignment drift
quality, and human usefulness.

### `scripts/appfw lock`

Writes framework provenance to `appfw.lock`.

```bash
scripts/appfw lock --write
scripts/appfw lock --write --json
```

Lock format version 2 records the framework commit, generator version, config
contract hash, template-set hash, golden downstream template hash,
provider-capability hash, workflow CLI hash, and generated artifact identity
when the artifact manifest exists. It is intended for downstream app repos and
upgrade review.

The artifact identity uses the closed
`appfw.artifact_manifest_identity@2` schema. Each artifact is qualified as
`app` or `config`, its path is normalized relative to that configured root, and
the identity includes every authoritative record field: root, path, ownership,
overwrite policy, content SHA-256, and optional source SHA-256. The generator's
`written` or `skipped` action is validated but excluded because it describes
the local run rather than the generated result. The identity is therefore
stable when the same product or its external config/report roots are relocated.

When `artifacts.json` exists, lock creation fails closed on malformed or unknown
record fields, unsupported values, invalid hashes, duplicate normalized paths,
unsafe or out-of-root paths, symlinks, non-regular files, an oversized manifest,
recorded content hashes that do not match the current artifact bytes, or
evidence that changes while it is read. Lock output also rejects symlinked
ancestors, configured filesystem roots, and non-regular leaves. It does not
turn incomplete or untrusted evidence into a portable identity.

### `scripts/appfw upgrade`

Emits a product upgrade report for the current app checkout. The command is
read-only and compares the app's `appfw.lock` to the current framework,
generator, template, provider-capability, config-contract, and artifact
surfaces.

```bash
scripts/appfw upgrade
scripts/appfw upgrade --json
scripts/appfw upgrade --check
scripts/appfw upgrade --check --json
```

`--check` is accepted for compatibility; `upgrade` is the product command. JSON
mode writes:

```text
target/appfw/product-upgrade.json
```

The report includes product identity from `.appfw/manifest.yaml` and
`app_topology.json`, explicit roots, lock comparison checks, changed-surface
classification, verification artifact status, and product-side recommended
commands. The command exits nonzero when the lock is missing or drift is
detected so CI can gate framework upgrade branches.

Version 1 locks do not carry the portable artifact identity schema. They are
reported as `lock_migration_required`; the command does not silently reinterpret
their raw manifest hash as version 2 identity. Regenerate and review the product,
complete the normal validation and test loop, then run
`scripts/appfw product lock --write` and a final
`scripts/appfw product upgrade --json`. Do not edit only the version field or
refresh the lock before the generated drift decision is accepted.

### `scripts/appfw new`

Bootstraps a downstream application repo from the current framework checkout.

```bash
scripts/appfw new ../customer-crm
scripts/appfw new ../customer-crm --json
scripts/appfw new ../customer-crm --from current --profile crm-sample
scripts/appfw new ../customer-crm --generate
scripts/appfw new ../operations-insight --from current --profile product-intake \
  --app-name operations-insight \
  --display-name "Operations Insight" \
  --schema ops \
  --provider PostgreSQL \
  --mcp false \
  --kafka true \
  --ui enterprise \
  --poc-source ../operations-insight-poc \
  --data-artifact source-data.xlsx \
  --ui-artifact prototype.html \
  --json
scripts/appfw new ../claims-modernization --from current --profile product-intake \
  --source-kind legacy \
  --app-name claims-modernization \
  --display-name "Claims Modernization" \
  --schema claims \
  --provider PostgreSQL \
  --mcp false \
  --kafka false \
  --ui enterprise \
  --legacy-source ../legacy-claims \
  --data-artifact readonly-db-inventory.yaml \
  --ui-artifact legacy-route-map.md \
  --json
scripts/appfw new --list-profiles --json
```

The default bootstrap profile is `crm-sample`. Profiles are defined under:

```text
app_gen/_golden/downstream_apps
```

Each profile carries a checked `profile.json` manifest. `new --list-profiles
--json` validates the manifest and emits the profile path, template path,
overlay path, verification commands, and any reusable frontend scaffold evidence
that should prove a downstream app created from that profile. For profiles with
`frontend_scaffold`, the JSON names the frontend root, generated scaffold
manifest, ownership manifest, generated UI contract, offline package check, and
release evidence paths.

`appfw new` copies the selected product template, applies the selected golden
profile overlay, writes product Cargo manifests that consume the approved
App Framework ProGet registry packages, runs config validation in the new app,
and writes `appfw.lock`. Pass `--generate` when the new app should also write
generated artifacts during bootstrap.

For citizen-developer PoC conversions and legacy-modernization starts, use
`--profile product-intake` instead of `crm-sample`. This mode asks for, or
accepts flags for, the required product topology choices before any entity
model is generated:

| Question | Flag | Allowed Values |
| --- | --- | --- |
| What kind of source evidence is being converted? | `--source-kind` | `poc`, `legacy` |
| Is an MCP server needed? | `--mcp` | `true`, `false` |
| Is a Kafka client needed? | `--kafka` | `true`, `false` |
| Is a product UI needed? | `--ui` | `none`, `scaffold`, `enterprise` |
| Which backend provider is needed? | `--provider` | `PostgreSQL` for the runnable product-intake scaffold |

`product-intake` also requires `--app-name`, `--display-name`, and `--schema`.
It accepts optional `--poc-source`, `--data-artifact`, and `--ui-artifact`
paths. For legacy modernization, pass `--source-kind legacy` or `--legacy` and
prefer `--legacy-source` for the source codebase. The data and UI artifacts
should reference read-only or sanitized data-source inventory and screenshots or
route inventory for the existing UI. In a human terminal session the command
prompts for omitted answers. In JSON or non-interactive use, missing answers
fail early and the error includes the required question contract and allowable
values. The command writes a product-owned repository shell with product intent:
`.appfw/poc-intake.yaml`, `.appfw/manifest.yaml`, selected data-source/schema
stubs, root `Cargo.toml`, backend/database/API-test/policy-test surfaces,
selected-provider `podman-compose.yml`, `scripts/appfw`, product README files,
and a frontend starter only when `--ui` is not `none`. Legacy modernization also
writes `.appfw/legacy-modernization.yaml`, a structured inventory for static
analysis, data discovery, stored-procedure disposition, migration/parallel-run,
and hardening evidence. It must not copy CRM entities, CRM routes, CRM docs, CRM
frontend screens, or other sample residue into the target product repo. It
currently creates a runnable backend scaffold only for PostgreSQL. Framework
provider certification still covers MongoDB, MS SQL Server, and Snowflake, but
those providers need provider-specific product client scaffold parity before
they should be selected by `product-intake`.
The command intentionally does not run validation, generation, or `--generate`; the AI
harness must first inspect the PoC or legacy artifacts and create the real
product `.appfw/model` entity model. The JSON report sets `validated: false`
with a deferred validation reason.

For legacy-modernization work, pair this command with
`docs/lifecycle/legacy-modernization.md` and the `product-legacy-modernization`
skill.
The handoff should retain the static code inventory, data discovery evidence,
stored procedure disposition, target capability map, migration posture, and
hardening evidence for the modernization slice.

The command is split-root by default: product config and generated output live
under the new app root, while framework source, generator templates, runtime
crates, and shared dev-infra assets remain in the framework root. To generate
against a different framework version, checkout that version first, then run
`appfw new --from current`.

### `scripts/appfw product analyze`

Profiles product intake artifacts before an agent writes final
`.appfw/model` source.

```bash
scripts/appfw product analyze --json
scripts/appfw product analyze --summary --json
```

The command reads `.appfw/poc-intake.yaml`, detects `source_kind: poc` or
`source_kind: legacy`, and writes:

```text
target/appfw/product-analysis.json
.appfw/poc-analysis.yaml
.appfw/legacy-analysis.yaml
```

Only one review YAML is written for the detected source kind. The JSON report
contains `detected_signals`, per-artifact `content_profile` evidence, and a
top-level `model_clues` section for candidate entities, properties, routines,
integration points, and frontend views. For PoC intakes, the report profiles
CSV/TSV headers, OOXML workbook sheets and named tables, HTML forms/tables,
JSON/YAML, and other configured source artifacts. For legacy intakes, it profiles configured
source/data/UI evidence for signals such as .NET code, SQL/routine source,
legacy web UI, structured exports, and tabular data. The command does not write
final model config. It is a bounded review and routing artifact that should
guide the human/agent model proposal.

Use `--summary --json` for normal terminal and agent runs. It prints artifact
paths, evidence counts, candidate entity names, routine/view candidates, and
open questions while retaining the full machine report at
`target/appfw/product-analysis.json`. Omit `--summary` or pass `--full --json`
only when a downstream tool intentionally needs full evidence on stdout.

### `scripts/appfw product propose-model`

Drafts a review-only model proposal from the latest product analysis artifact.

```bash
scripts/appfw product propose-model --json
scripts/appfw product propose-model --summary --json
```

The command reads:

```text
target/appfw/product-analysis.json
```

and writes:

```text
target/appfw/model-proposal.json
.appfw/model-proposal.yaml
```

The proposal includes candidate entities, semantic labels and suggested domain
names when source references provide them, entity-owned candidate properties
when evidence clearly maps fields to a source table, unassigned properties,
routine candidates, frontend views, integration points, open questions,
guardrails, next commands, an `implementation_plan` with schema source root,
required source-file categories, entity modeling tasks, relationship and
service tasks, frontend tasks, evidence tasks, and validation sequence, and a
`review_decisions` ledger with unresolved states (`needs_review`,
`needs_discovery`) and terminal decisions (`accept`, `reject`, `defer`, `split`,
`merge`). It
intentionally does not write final `.appfw/model`; humans and agents must
review `.appfw/model-proposal.yaml` before source modeling.

Use `--summary --json` for normal terminal and agent runs. It prints candidate
entity names, semantic labels, suggested domain names, property counts,
unassigned/routine counts, the implementation-plan source root and task counts,
review-decision counts, guardrails, and artifact paths while retaining the full
proposal at `target/appfw/model-proposal.json`.

### `scripts/appfw product model-status`

Reports the current PoC or legacy conversion phase and the next safest command.

```bash
scripts/appfw product model-status --json
```

The command reads retained intake, analysis, proposal, model source,
validation, generation, and handoff artifacts, then writes:

```text
target/appfw/model-status.json
```

Use it after `product propose-model`, after writing `.appfw/model`, after
validation, and after generation. It is intentionally lightweight and does not
write product model source or generated artifacts. The JSON report includes the
current `phase`, app/schema/provider metadata, source-model file counts,
`schema_model.graphql_http_route` (the official schema-name → kebab HTTP
route segment), proposal review flags, editable `review_decisions` readiness
from `.appfw/model-proposal.yaml`, a `source_authoring_plan`, blockers, and
`next_commands`.
The official converter is `appfw_codegen::schema_route_segment` in
`app_gen/src/schema_route.rs`. `appfw_introspect` and `appfw-test` both
depend on that crate module via Cargo so GraphQL clients, model-status, and
publish/CLI docs share Inflector 0.11 kebab semantics after a leading-slash
trim (`nexus_work` → `nexus-work`). Do not add a `#[path]` include or a
second kebab helper.
`proposal_review.ready_for_config` requires resolved decision entries,
`review_decisions.sign_off.accepted_for_config: true`, and a signed review with
`model_owner` plus `reviewed_at_utc`.
`source_authoring_plan.ready_to_write_source` mirrors that readiness and
surfaces schema source root, required source-file categories, accepted entity
source targets, review decision counts, validation sequence, and a guardrail
for the next modeling step.

### `scripts/appfw product scaffold-model`

Bootstraps source-owned model starters from a signed model proposal review.

```bash
scripts/appfw product scaffold-model --dry-run --json
scripts/appfw product scaffold-model --json
scripts/appfw product scaffold-model --force --json
```

The command reads `.appfw/model-proposal.yaml` through the same model-status
contract and refuses to run until
`source_authoring_plan.ready_to_write_source` is true. Accepted entity decisions
must also have explicit `config_kind` (`entity`, `lookup`, or `dto`) and
`classification` values; unresolved values such as `entity|lookup|dto|reject|defer`
or `needs_review` remain agent/human modeling work.

With `--dry-run`, the command previews files and writes no artifact. Without
`--dry-run`, it writes starter entity YAML and deny-by-default policy Rego
source, skips existing files unless `--force` is supplied, and retains:

```text
target/appfw/model-scaffold.json
```

The command is intentionally not an LLM replacement. It does not infer
relationships, real properties, DTO boundaries, stored procedure disposition,
frontend behavior, product services, or business rules. Agents should use it as
a reviewed source bootstrap, then complete semantic modeling from the original
PoC or legacy evidence before validation and generation.

Model-status phases:

| Phase | Meaning |
| --- | --- |
| `not_intake_product` | No `.appfw/poc-intake.yaml` exists for this app root. |
| `intake_recorded` | Intake exists, but product analysis has not run. |
| `analyzed` | Analysis exists, but the model proposal has not been drafted. |
| `proposal_ready_for_modeling` | Proposal/review artifacts exist, but review decisions are not ready for config. |
| `proposal_reviewed` | Review decisions are accepted for config, signed by a model owner with review timestamp, and no unresolved decision entries remain. |
| `model_source_started` | Entity source exists, but validation has not passed. |
| `model_validated` | Product model source has passed validation and is ready to generate. |
| `generated` | Generated artifacts are present and should be checked/tested/handoffed. |
| `handoff_ready` | Product handoff evidence exists. |

### `scripts/appfw framework intake-proof`

Runs a disposable proof of the product-intake path without creating a durable
product repo.

```bash
scripts/appfw framework intake-proof --json
scripts/appfw framework intake-proof --source-kind legacy --json
```

The command creates a temporary synthetic source, scaffolds a
`product-intake` app, runs `product analyze` and `product propose-model`
internally, copies the retained evidence, and removes the temp workspace unless
`--keep` is passed.

Retained artifacts:

```text
target/appfw/product-intake-proof.json
target/appfw/product-intake-proof/product-analysis.json
target/appfw/product-intake-proof/model-proposal.json
target/appfw/product-intake-proof/model-status.json
target/appfw/product-intake-proof/model-proposal.yaml
target/appfw/product-intake-proof/frontend-residue-check.json
target/appfw/product-intake-proof/frontend-scaffold-check.json
```

The proof report includes `frontend_residue_check` and
`frontend_scaffold_check` blocks. For UI-enabled intake scaffolds, these scan
the generated frontend shell, run the product-local scaffold check, and fail the
proof when CRM sample terms leak into product-facing files or PDS design-system
wiring is missing.

Use this as a framework stewardship check when CLI, docs, skills, or product
bootstrap behavior changes. Product developers should use the explicit product
commands in their own app repo.

### `scripts/appfw golden-downstream`

Validates golden downstream profile and frontend scaffold evidence without
creating a disposable product app.

```bash
scripts/appfw golden-downstream --json
scripts/appfw golden-downstream --profile crm-sample --json
scripts/appfw golden-downstream --profile crm-sample --execute --json
scripts/appfw framework golden-downstream --profile second-consumer --execute --json
```

The command is the safe metadata gate for the write-heavy downstream CI lane. It
checks the selected profile manifest, required verification commands, frontend
scaffold package checks, and release-evidence paths, then writes:

```text
target/appfw/golden-downstream.json
```

A missing required verification or failed execution step preserves the JSON
artifact with `ok:false` and returns nonzero. Metadata mode records
`full_disposable_ci_status:"specified-not-run"`; it is not execution evidence.

The artifact also records the full disposable CI command sequence that should be
run in a CI workspace when the release needs end-to-end downstream proof:
create app, validate, harness-check, generate, generated-drift check, tests, frontend evidence,
release check, upgrade check, and handoff. Use this command in low-cost
maintainability checks; use the recorded disposable lane when product bootstrap
or scaffold behavior changes materially.

Pass `--execute` only in a disposable workspace or CI lane. Execution mode
creates a downstream product app under `target/appfw/golden-downstream-work` by
default, runs the recorded create-to-handoff lane, and records per-step exit
codes, durations, and output excerpts in `target/appfw/golden-downstream.json`.
Use `--workspace <path>` with `--execute` when CI must retain the disposable app
outside the default artifact tree.

### `scripts/appfw skills`

Lists the repo-native agent skills and their canonical documentation links.
This is the quick entrypoint when an agent knows the workflow but should not
load the full docs catalog.

```bash
scripts/appfw skills
scripts/appfw skills --json
```

JSON mode returns:

```json
{
  "command": "skills",
  "ok": true,
  "skill_root": "agent_skills",
  "count": 17,
  "skills": [
    {
      "name": "product-bootstrap",
      "description": "Use when creating a new downstream App Framework product workspace...",
      "path": "agent_skills/product-bootstrap/SKILL.md",
      "docs": ["docs/lifecycle/product-golden-path.md", "docs/start/cli-quickstart.md"]
    }
  ]
}
```

Skill bodies must stay concise. Deep guidance belongs in the linked canonical
docs and generated artifacts, not in a second manual.

### Downstream Lifecycle Recipes

The lifecycle docs give the full product contract. These are the executable CLI
recipes agents should use when proving downstream work.

Create a new app:

```bash
scripts/appfw product new --list-profiles --json
scripts/appfw product new ../customer-crm --from current --profile crm-sample --generate --json
cd ../customer-crm/frontend
npm run appfw:check
npm run typecheck
npm run build
```

For product starters with a frontend, `npm run build` emits the production SPA
to `backend/product_dist` so the product backend image can serve it at `/` when
`APP_PRODUCT_UI_ENABLED=true`.

Customize product-owned behavior:

```bash
scripts/appfw product explain ownership .appfw/model/schemas/crm/entity_types/account.yaml --json
scripts/appfw product topology --json
scripts/appfw product validate --json
scripts/appfw product boundary-check --json
scripts/appfw product test --fast
scripts/appfw product handoff --json
```

Regenerate and prove generated artifacts:

```bash
scripts/appfw product validate --json
scripts/appfw product generate
scripts/appfw product generate --check --json
scripts/appfw product test
scripts/appfw product handoff --json
```

Review framework upgrade impact inside a product app:

```bash
scripts/appfw product upgrade --json
scripts/appfw product validate --json
scripts/appfw product generate
scripts/appfw product generate --check --json
scripts/appfw product test
scripts/appfw product lock --write
scripts/appfw product upgrade --json
scripts/appfw product handoff --json
```

Prove migration and rollback readiness:

```bash
scripts/appfw product migrate plan --json
scripts/appfw product migrate lint --phase all --json
scripts/appfw product migrate drift --json
scripts/appfw product migrate rollback-guide --json
scripts/appfw product handoff --json
```

For frontend product work, add the product frontend typecheck, lint, tests,
browser E2E/a11y evidence, and build commands to the same handoff evidence.
`scripts/appfw product frontend-test --json` runs the current product frontend evidence
bundle when a frontend package is present. If a live backend, provider, browser
runtime, or frontend command is unavailable, keep the skipped check and reason
in `target/appfw/agent-handoff.json`.

Lifecycle recipes are complete only when the expected evidence is retained:

| Recipe | Expected Evidence |
| --- | --- |
| Create | Profile list JSON, bootstrap JSON, validation JSON, `appfw.lock`, and final `target/appfw/agent-handoff.json`. |
| Customize | Ownership explanation, topology JSON, validation JSON, boundary-check result, focused test result, and handoff skipped-check notes. |
| Generate | `.appfw/target/appfw/artifacts.json`, `artifact_provenance.json`, generated diff review, `generate --check --json`, tests, and handoff. |
| Upgrade | Initial `target/appfw/product-upgrade.json`, generated drift decision, refreshed `appfw.lock`, final `product-upgrade.json`, tests, and handoff. |
| Migration | Migration plan, lint, reachable drift, rollback guide, phase notes, provider limitations, and handoff. |
| Release/handoff | Risk-based validation, generated drift, tests, API/provider/frontend evidence where applicable, and final handoff after the last edit. |

If any required command cannot run, do not replace it with prose only. Record
the command, the concrete blocker, and the risk in
`target/appfw/agent-handoff.json` so the next reviewer can continue the same
proof chain.

### `scripts/appfw docs-check`

Executes the safe CLI examples that this documentation expects agents to use.

```bash
scripts/appfw docs-check
scripts/appfw docs-check --json
scripts/appfw docs-check --fast --json
scripts/appfw docs-check --changed-only --json
scripts/appfw docs-check --changed-only --plan --json
scripts/appfw docs-check --subcheck saas-vendor-doc-parity --json
scripts/appfw docs-check --subcheck core-generated-drift-command-examples --json
scripts/appfw docs-check --fast --json --enforce-budget
scripts/appfw docs-check --changed-only --json --budget-ms 45000
scripts/appfw docs-check --full --json
```

The default `--full` check runs representative JSON examples for `doctor`, `validate`,
`generate --check`, `explain ownership`, `explain config`, `explain provider`,
`explain provider-sdk`, `new --list-profiles`, `golden-downstream`, `skills`,
`handoff`, and the compiled CLI `new --list-profiles` path. It also writes
`target/appfw/lifecycle-evidence-checklist.json`, which maps lifecycle stages to
the required commands, retained artifacts, docs-check coverage, and remaining
golden downstream CI gaps. It also writes
`target/appfw/maintainability-contract.json`, which checks that the docs IA,
agent skill pack, roadmap scorecard, deployment path, runtime ingress contract,
golden downstream proof path, frontend scaffold guidance, CLI display controls,
and command references remain discoverable. It also writes
`target/appfw/saas-vendor-doc-parity.json`, which checks that each SaaS provider
vendor-contract doc is version-linked to exported crate metadata constants.
Provider-backed, long-running, or destructive examples remain covered by
`release-check`, `provider-test`, `api-test`, `migrate`, `serve`, and
`load-test`.

Use `--fast` for the local agent/developer loop when the change does not alter
CLI behavior. It keeps the static maintainability, security-env, lifecycle
evidence, shell syntax, and design-system checks, but skips the compiled CLI and
golden downstream command examples.

Use `--changed-only` for PR preparation. It inspects changed files against
`origin/main` by default, ignores generated frontend distribution output, and
selects `--full` when CLI, generator, app_gen config/templates, command-contract
docs, or agent skills changed. Narrow CI wrapper scripts that are already
covered by shell syntax and their own retained command contract can remain on
the faster changed-only path; current carve-outs include
`scripts/ci/wave3-pr-gates.sh` and `scripts/ci/pre-push-review-guard.sh`. Pass
`--base <ref>` to compare against another merge base.

Use `--subcheck <name>` for a focused local loop over one retained subcheck
group, such as `core-generated-drift-command-examples` after a generator-routing
edit or `core-generation-command-examples` for validation/topology routing.
Focused subchecks are deliberately opt-in: they shorten local iteration
and still write the same timing/subcheck artifacts, but they do not replace
default, changed-only, release, or CI docs-check coverage. A full-only subcheck
requested from a selected `--fast` run fails and tells the developer to use
`--full`.

Every JSON result includes `requested_mode`, `selected_mode`, `mode_reason`,
`focused_subcheck`, `changed_files_count`, `changed_surface_artifact`,
`changed_surface_requires_full`, `changed_surface_trigger_count`,
`timing_artifact`, `subchecks_artifact`, `budget_ms`, `budget_ok`,
`budget_enforced`, `total_wall_ms`, aggregate phase/example/unattributed
timings, counts,
`slowest_example`, `slowest_subcheck`, `focused_rerun_command`,
`parallelization_guidance`, and per-example `elapsed_ms` fields.
The retained changed-surface artifact is written to
`target/appfw/docs-check-changed-surface.json` with the changed file list,
triggering files, trigger rule names, selected mode, and base ref. Use it when
reviewing why a broad branch escalated from changed-only to full.
The retained subcheck artifact is written to
`target/appfw/docs-check-subchecks.json` with named static/compiled groups such
as `shell-syntax`, `lifecycle-evidence`, `design-system`,
`security-env-doc-parity`, `maintainability-contract`,
`saas-vendor-doc-parity`,
`core-bootstrap-command-examples`, `core-generation-command-examples`,
`core-generated-drift-command-examples`, `core-explain-profile-command-examples`,
`intake-skills-command-examples`, `agent-governance-command-examples`
(umbrella alias), `agent-governance-core-command-examples`,
`agent-governance-write-command-examples`,
`agent-governance-mobile-command-examples`,
`agent-governance-runtime-command-examples`,
`agent-governance-release-command-examples`,
`packaging-dependency-command-examples`, and `compiled-cli-command-examples`.
It also records `slowest_subcheck` with `name`, `elapsed_ms`, `item_count`, and
`detail`, a concrete `focused_rerun_command`, and
`parallelization_guidance`. Use those fields to decide whether the next slice is
to split a multi-item group, cache or optimize a single slow command, move logic
to typed Rust, route a check into a narrower PR lane, or run locally with
`--subcheck <name>`. Focused subchecks are not same-worktree parallel safe today
because they share `target/appfw` logs and retained artifacts; run them
sequentially in one worktree or isolate parallel experiments in separate
worktrees/report roots.
The retained timing artifact is written to `target/appfw/docs-check-timing.json`
with total wall time, aggregate phase time, aggregate example time,
unattributed wall time, slowest example, phase timings such as
`changed-surface-selector`, `prebuild-appfw-codegen`, and `prebuild-appfw-cli`,
named subchecks, and per-example timings. Default budgets are 15 seconds for
`fast` and 60 seconds for `full`; normal runs record budget status without
failing. Tune the
selected-mode defaults with `APPFW_DOCS_CHECK_FAST_BUDGET_MS` and
`APPFW_DOCS_CHECK_FULL_BUDGET_MS`. Add `--enforce-budget` or set
`APPFW_DOCS_CHECK_ENFORCE_BUDGET=1` to fail when the selected mode exceeds its
budget, and pass `--budget-ms <ms>` or `APPFW_DOCS_CHECK_BUDGET_MS=<ms>` to
override both selected-mode budgets for one run. Add `--progress` to stream
JSONL progress events to stderr while preserving the final JSON report on
stdout. Add `--plan` to report the selected tier and budget without running
checks.

Bitbucket pull-request pipelines run `docs-check --changed-only --json
--progress --enforce-budget` in the fast framework check with CI budgets of 20
seconds for fast-selected runs and 480 seconds for full-selected runs. Full
release-grade docs-check remains part of release/main evidence before generated
drift and provider certification.
The design-system docs-check fixture passes governed-write evidence overrides
to `scripts/check-pds-components.mjs` instead of mutating canonical
`target/appfw` evidence, keeping the fast lane repeatable and interruption-safe.

### `scripts/appfw framework cli-test`

Runs the focused CLI command-contract lane for developers changing the wrapper,
packaged CLI, profile manifests, or command-routing metadata.

```bash
scripts/appfw framework cli-test --plan --json
scripts/appfw framework cli-test --json
```

`--plan --json` is read-only and cheap; docs-check uses it to keep the command
discoverable without making docs-check run Cargo tests. The real test lane runs:

- `bash -n scripts/appfw`
- `bash -n scripts/check-doc-examples.sh`
- `bash scripts/cli-semantic-gates-test.sh`
- `python3 scripts/appfw-delivery-mode-test.py`
- `python3 scripts/mobile-readiness-containment.test.py`
- `bash scripts/ci/proget-publish.test.sh`
- `cargo test --locked -p appfw-cli`
- `cargo test --locked --bin appfw_introspect cli_contract` in `app_gen`

The official GraphQL schema route converter is proven separately with
`cargo test --locked -p appfw-test -- graphql_client` (6/6 ASCII/kebab
matrix). Both `appfw-test` and `appfw_introspect` resolve
`appfw_codegen::schema_route_segment`;
`cli_contract_schema_route_segment_uses_shared_appfw_codegen_converter`
covers the introspect call path.

The command writes `target/appfw/cli-test.json` plus per-check logs under
`target/appfw/cli-test-*.log`. It intentionally avoids docs-check, generation,
product crate compilation, provider certification, browser tests, and live
services. Use it for fast feedback on CLI routing changes; use docs-check when
documentation examples or maintainability contracts changed.

The semantic fixture proves required false conditions fail, report-only compat
findings cannot produce top-level `ok:true`, validate names its requested
namespace and actual scope, and delivery-profile annotations preserve diagnostic
success while JSON stdout remains byte-equal to the final retained artifact.
Pull-request CI runs the real cli-test lane directly; the docs-check example
remains explicitly plan-only.

### `product chat-eval`

`scripts/appfw product chat-eval --json` runs the deterministic CH6 local
fixture harness from `scripts/check-chat-eval.mjs`. It validates recorded or
stubbed chat transcripts against the answer-envelope, tool-registry, citation,
tenant-isolation, write-gate, and replay-determinism contracts described in
[Chat-Eval Spec](../architecture/concerns/chat-eval-spec.md).

The command writes the product posture artifact at
`.appfw/target/appfw/chat-eval.json` and stages a release-evidence copy at
`target/appfw/wave4/ch6-chat-eval.json`. The staged copy is useful CI evidence,
but it remains `release_ready:false` and `mode:"local-fixture"` until a future
judge/live evidence artifact exists.
Managed judge/live proof is retained separately at
`target/appfw/chat-eval-judge-evidence.json`. `scripts/ci/release-evidence-check.sh`
validates that artifact when present and requires it when
`APPFW_REQUIRE_CHAT_EVAL_JUDGE_EVIDENCE=true` or
`APPFW_RELEASE_REQUIRE_CHAT_EVAL_JUDGE_EVIDENCE=true`, so local deterministic
evidence cannot satisfy CH6 release readiness by accident.

```bash
scripts/appfw product chat-eval --json
scripts/appfw product chat-eval --json --pipeline-plan
scripts/appfw product chat-eval --json --inject-leak
```

`--pipeline-plan` keeps the deterministic local fixture behavior and also
writes `.appfw/target/appfw/chat-eval-promptfoo-plan.json`, staging a release
copy at `target/appfw/wave4/ch6-chat-eval-promptfoo-plan.json`. The artifact
describes the future promptfoo PIPELINE tier (`is-json`, `contains-json`, and
`tool-call-f1` assertions against offline fixtures) without requiring promptfoo
for local development and without claiming live evidence.

`--inject-leak` is the fail-closed probe. It intentionally injects a forbidden
tenant value into the output stream, exits non-zero, and proves the harness
reports a `policy_leak` finding. It does not stage that failed artifact as
release evidence.

`scripts/appfw product chat-eval --plan --json` remains available for Wave 0
metadata. The framework namespace form fails and points back to the product
namespace.

### `product harness-check`

`scripts/appfw product harness-check --json` validates the product app's
least-privilege agent profile at `.appfw/agent-profile.yaml` and writes retained
evidence to `.appfw/target/appfw/harness-check.json`. It also stages a copy at
`target/appfw/wave2/u2-harness-check.json` for Wave 2 release-evidence
collection. The staged copy is retained input for release authority; it does
not claim release readiness while G1 live governed-write evidence is still
missing. This is the first
executable U2 evidence lane: it proves product agents are scoped to product
commands and product-owned write surfaces before they operate on a downstream
app.

Profiles declare only the evidence roots used by their allowed commands. A
profile that allows validation, harness-check, generation, boundary-check, or a
validation-running test must include the resolved in-product report root in
`writable_paths` (normally `.appfw/target/appfw`). A profile that allows product
handoff must include `target/appfw` and declare
`target/appfw/agent-handoff.json`. Handoff-only profiles do not need an unused
command-report root. Harness-check derives the required report root from the
resolved product roots, then returns parseable `ok:false` and a nonzero exit for
a missing required root or a mismatched handoff artifact.

The checker fails closed when the profile:

- is missing;
- allows framework-scoped commands;
- grants writes to framework source paths;
- enables network or live-service access;
- enables MCP, Kafka, or release capabilities;
- enables SaaS governed-write capabilities without valid
  `target/appfw/governed-write-evidence.json` and enforced
  `target/appfw/governed-write-posture.json` G1 artifacts;
- omits the ASI01, ASI02, and ASI03 threat-control mapping from the G3 agentic
  threat model;
- allows a report-writing command without writable coverage for its resolved
  in-product report root;
- allows handoff without requiring it, declares a handoff artifact other than
  `target/appfw/agent-handoff.json`, or lacks writable `target/appfw` coverage.

Example summary:

```json
{
  "command": "harness-check",
  "lane": "U2",
  "ok": true,
  "artifact": ".appfw/target/appfw/harness-check.json"
}
```

`scripts/appfw product harness-check --plan --json` remains available as the
original Wave 0 reservation metadata. The framework namespace form
`scripts/appfw framework harness-check --json` fails and points back to the
product namespace.

### `framework fork-check`

`scripts/appfw framework fork-check --json` writes
`target/appfw/fork-check.json` with the first U6 report-only posture check for
PDS/source forks. It rejects known product-local token copies, records shared
PDS component and token imports, and requires generated UI contracts to point at
the canonical token source alias.

```bash
scripts/appfw framework fork-check --json
```

The report includes:

- `lane:"U6"`;
- `gate.enforced:true` for known PDS token-copy forks;
- `token_consumers` showing known forbidden local-copy paths and whether any
  are present;
- `source_consumers` proving current shared PDS package and token imports;
- `generated_contracts` proving generated `tokenCssPath` values use the
  canonical PDS token source alias;
- `blocking_violations` and `next_steps` for the source-consumption path.

`scripts/appfw framework fork-check --plan --json` remains available as the
Wave 0 planning surface. `scripts/appfw product fork-check --json` fails and
points back to the framework namespace.

### `framework composition-check`

`scripts/appfw framework composition-check --json` writes
`target/appfw/composition-check.json` with the U7 posture check for
manifest-driven runtime and build composition. It compares the
validated topology, generated ingress config, backend feature declarations,
provider dependencies, and embedded frontend artifacts so the Cargo/provider
convergence work has a retained baseline.

```bash
scripts/appfw framework composition-check --json
scripts/appfw framework composition-check --enforce --json
```

The report includes:

- `lane:"U7"`;
- `runtime_ingress` and `ingress_modules` resolved from app topology;
- `providers` for every manifest topology data source, `runtime_providers`
  for schema-bound or graph-read providers that the backend runtime actually
  uses, and `provider_dependencies` with optional-dependency, feature-linkage,
  and default-feature evidence;
- `frontend` evidence for `backend/product_dist`, `backend/admin_dist`, and
  the `.appfw/manifest.yaml` `ui.product_spa` / `ui.admin_ui` packaging
  posture so intentionally disabled admin UI does not look like a missing
  bundle;
- `binary_features` including declared/default backend features,
  provider-feature readiness, runtime-provider default-selection readiness,
  tracked duplicate-version budgets, `async_graphql_versions`, and
  `reqwest_versions`;
- `compile_evidence.feature_check` with the retained `feature-check` artifact,
  the expected runtime/ingress/provider feature subchecks, and any missing or
  failed checks;
- `gate.enforced:false` in report mode; `--enforce` sets
  `gate.enforced:true` and fails unless the retained non-plan feature-check
  artifact proves every expected subcheck;
- `enforced_artifact:"target/appfw/composition-check-enforced.json"`; explicit
  `--enforce` runs write that stable artifact so later report-only docs examples
  do not erase the last retained enforcement proof.

`scripts/appfw framework composition-check --plan --json` remains available as
the Wave 0 planning surface. `scripts/appfw framework release-check --json`
runs `composition-check --enforce` after `feature-check`, retains
`composition-check.json` in the active release artifact directory, and refreshes
`target/appfw/composition-check-enforced.json` in the framework evidence
directory.
`scripts/appfw product composition-check --json` fails and points back to the
framework namespace.

### `framework governance-check`

`scripts/appfw framework governance-check --json` writes
`target/appfw/governance-check.json` with the first G4 report-only posture check
for PHI governance, release provenance, artifact signing, release identity, PDS
baseline, and the formal security-assurance decision. It inventories the
existing release-evidence and security-assurance registries instead of creating
a second governance model.

PHI pipeline evidence is expected at
`target/appfw/phi-pipeline-governance-evidence.json`, or at the path named by
`APPFW_PHI_PIPELINE_EVIDENCE_FILE`. The evidence must use schema
`appfw.phi-pipeline-governance.v1` and cover classification propagation,
de-identification, lower-environment movement, RAG corpus curation, retention,
deletion/tombstone handling, redaction, and audit lineage. Report mode records
missing or schema-thin evidence as transitional; enforced mode rejects it.

```bash
scripts/appfw framework governance-check --json
```

Use `--enforce` when a lane claims G4 release governance readiness:

```bash
scripts/appfw framework governance-check --enforce --json
```

The enforced mode writes `target/appfw/governance-check-enforced.json` and exits
non-zero while transitional governance evidence remains. It is intentionally a
fail-closed readiness gate for local evidence posture; live release authority,
PDS baseline approval, provenance, and signing attestations still come from the
managed release process. Local `release-provenance.json` or
`artifact-signing.json` files prove the evidence path exists, but they do not
count as production provenance/signing unless the formal security-assurance
decision either records production attestations or a structured accepted-risk
decision.

The report includes:

- `lane:"G4"`;
- `phi_pipeline` evidence source, required controls, schema validation summary,
  classification validation source, and PHI log lint source;
- `provenance` and `artifact_signing` evidence candidates;
- production provenance/signing readiness derived from the existing
  security-assurance categories, with local evidence reported separately;
- `security_assurance_decision`, `release_identity`, and PDS baseline posture;
- `release_authority_package`, the machine-readable checklist of required
  managed release artifacts, candidate paths, source commands, and accepted-risk
  semantics for G4;
- `category_dispositions` from the existing security-assurance decision;
- `gate.enforced` and `gate.ready_to_enforce`;
- `enforced_artifact` for the retained enforced report path;
- `enforcement_violations` when `--enforce` rejects transitional evidence.

`scripts/appfw framework governance-check --plan --json` remains available as
the Wave 0 planning surface. `scripts/appfw product governance-check --json`
fails and points back to the framework namespace.

### `framework aibom-check`

`scripts/appfw framework aibom-check --json` writes
`target/appfw/aibom-check.json` with the first Wave 4 `SEC-AIBOM` posture check
for AI-assisted code provenance, AIBOM attestation, and PR-trailer attribution.
It is report-only by default so normal agent iteration remains fast.

```bash
scripts/appfw framework aibom-check --json
```

Use `--enforce` only when a branch or release claims SEC-AIBOM readiness:

```bash
scripts/appfw framework aibom-check --enforce --json
```

The enforced mode writes `target/appfw/aibom-check-enforced.json` and exits
non-zero until a release-grade `target/appfw/aibom-release-attestation.json` is
present, `release_ready:true`, tied to the exact Git HEAD, carries at least one
agentic change inventory item, records approved human review, verifies required
trailers, and binds an AIBOM artifact path and hash.

The report includes:

- `lane:"SEC-AIBOM"`;
- current Git HEAD and subject;
- required PR trailers (`Agent-Assisted`, `AIBOM-Artifact`, `Human-Review`,
  `Evidence`);
- the `appfw.aibom.release-attestation.v1` schema contract;
- per-field attestation checks;
- required source docs/scripts for the posture;
- `gate.enforced`, `gate.ready_to_enforce`, and `enforcement_violations`.

`scripts/appfw framework aibom-check --plan --json` remains available as the
cheap planning surface. `scripts/appfw product aibom-check --json` fails and
points back to the framework namespace.

### `framework prompt-audit-check`

`scripts/appfw framework prompt-audit-check --json` writes
`target/appfw/prompt-audit-check.json` with the first `SEC-PROMPTAUDIT` posture
check for chat prompt logging, SIEM/export retention, prompt redaction,
correlation IDs, access-review sampling, incident response runbook evidence,
and the chat runtime kill switch. It is report-only by default; it does not
enable chat or claim live prompt readiness.

```bash
scripts/appfw framework prompt-audit-check --json
```

Use `--enforce` only when a branch or release claims live prompt-audit
readiness:

```bash
scripts/appfw framework prompt-audit-check --enforce --json
```

The enforced mode writes `target/appfw/prompt-audit-check-enforced.json` and
exits non-zero until a managed
`target/appfw/prompt-audit-release-evidence.json` artifact, or the file named by
`APPFW_PROMPT_AUDIT_EVIDENCE_FILE`, uses schema
`appfw.prompt-audit.release-evidence.v1`, has `ok:true` and
`release_ready:true`, and proves every required control.

The report includes:

- `lane:"SEC-PROMPTAUDIT"`;
- the chat runtime env contract from `appfw_runtime/src/security.rs`;
- source-contract checks for prompt audit, SIEM, sink, retention floor, audit
  event append, and kill-switch behavior;
- the release evidence schema and required control fields;
- `gate.enforced`, `gate.ready_to_enforce`, and `enforcement_violations`.

`scripts/appfw framework prompt-audit-check --plan --json` remains available as
the cheap planning surface. `scripts/appfw product prompt-audit-check --json`
fails and points back to the framework namespace.

### `framework ai-gateway-decision`

`scripts/appfw framework ai-gateway-decision --json` writes
`target/appfw/ai-gateway-decision.json` with the first CH7 gateway/model
decision posture check. It is report-only by default: the framework can prove
its AI-search gateway contract exists, but it cannot self-certify stakeholder
approval for a production model backend, LiteLLM or equivalent gateway license
posture, credential issuance, or egress policy.

```bash
scripts/appfw framework ai-gateway-decision --json
```

Use `--enforce` only when a branch or release claims CH7 gateway/model decision
readiness:

```bash
scripts/appfw framework ai-gateway-decision --enforce --json
```

The enforced mode writes `target/appfw/ai-gateway-decision-enforced.json` and
exits non-zero until a managed
`target/appfw/ai-gateway-decision-evidence.json` artifact, or the file named by
`APPFW_AI_GATEWAY_DECISION_EVIDENCE_FILE`, uses schema
`appfw.ai-gateway-decision.v1`, has `ok:true` and `release_ready:true`, and
proves every required control.

The required control values are intentionally narrow:

- `controls.gateway_selection:"approved"`;
- `controls.model_backend_selection:"approved"`;
- `controls.litellm_license_review:"approved"` or `"not_applicable"`;
- `controls.credential_custody:"server_secret_ref_only"`;
- `controls.egress_allowlist:"approved"`;
- `controls.prompt_audit_dependency:"required_before_live_prompts"`;
- `controls.policy_authority_boundary:"framework_policy_not_gateway"`;
- at least two owner approvals with `approver` and `role`.

The report includes:

- `lane:"CH7"`;
- the `appfw.ai-gateway-decision.v1` evidence contract and source path;
- source-contract checks for the `appfw_provider_ai_search` gateway contract,
  server-side secret custody, prompt-audit dependency, policy re-resolution,
  and provider-test plan wording;
- per-control evidence checks;
- `gate.enforced`, `gate.ready_to_enforce`, and `enforcement_violations`.

`scripts/appfw framework ai-gateway-decision --plan --json` remains available as
the cheap planning surface. `scripts/appfw product ai-gateway-decision --json`
fails and points back to the framework namespace.

### `framework wave2-status`

`scripts/appfw framework wave2-status --json` writes
`target/appfw/wave2-readiness.json` with the aggregate North-Star Wave 2
readiness posture. It reads the retained lane artifacts instead of creating
new provider/mobile/release evidence, so it is a status surface, not a
promotion shortcut.

```bash
scripts/appfw framework wave2-status --json
```

For CI gates that must fail closed when Wave 2 is still live-gated, add
`--strict` or `--enforce`. The command still writes
`target/appfw/wave2-readiness.json` and prints JSON, but exits nonzero unless
the legacy-contained U5 lane has been replaced by the source-bound mobile
candidate checker and every release-authoritative lane reports
`release_ready:true`.

```bash
scripts/appfw framework wave2-status --json --strict
```

The report includes:

- `lane:"north-star-wave-2"`;
- `summary.total_lane_count` for every Wave 2 lane, plus
  `summary.local_proven_count` and `summary.local_lane_count` for the local
  executable slices that exclude the P1-P4 managed-release lane;
- `summary.local_preflight_proven_count` and
  `summary.local_preflight_lane_count` for the P1-P4 branch-level local
  preflight posture, which is useful before PR review but is still separate
  from managed release authority;
- per-lane posture for G1, U2, G2, U5, U6, U7, D6, G4, CH6, and the P1-P4
  live evidence lane;
- G1 live evidence schema posture for the reserved
  `target/appfw/governed-write-evidence.json` artifact, so a loose
  `ok:true`/`lane:G1` file cannot unlock Wave 2 without the delegated
  actor, token-isolation, named-mutation, idempotency, policy, and audit
  proof shape;
- U5 detail passthrough for `runtime_audit`, `device_evidence`, and
  `store_track_evidence` from the retained product `mobile-test` report, plus a
  consumer-owned `readiness_authority` that keeps the legacy command
  non-authoritative even when a retained report is stale or forged;
- CH6 detail passthrough for the staged deterministic
  `target/appfw/wave4/ch6-chat-eval.json` local-fixture evidence and the
  release-gated `target/appfw/chat-eval-judge-evidence.json` artifact;
- `release_ready:false` until every lane has retained release-ready evidence;
- `remaining_external_gates` so agents can distinguish local fail-closed proof
  from real live certification;
- `external_evidence_plan`, which records lane-coded handoff entries for G1,
  U2, G2, U5, G4, and P1-P4 with the exact retained artifacts, commands,
  environment hooks, and owner boundary needed to turn each live-gated or
  G1-dependent lane into release evidence. This plan is guidance and
  machine-readable handoff data; it does not make focused/local evidence
  release-authoritative. Its `required_artifacts` are release-staged artifact
  locations under `target/appfw`, even when the producing product command also
  writes a source artifact inside the product workspace. Each plan entry also
  includes `required_artifact_status` and `required_artifact_summary`, and the
  top-level report includes `external_required_artifacts`, so handoffs can see
  which release-staged artifacts are present, missing, or retained with
  `release_ready:false` without interpreting that presence as release
  authority.

M0-05 deliberately hard-codes the U5 lane `candidate_ready:false` and
`release_ready:false` until a future source-bound mobile candidate checker
replaces the legacy authority contract.
The U5 external plan therefore names `target/appfw/mobile-candidate-check.json`
as missing future evidence. Strict release-evidence validation independently
rejects malformed or duplicate-key Wave 2 JSON and any legacy U5 readiness
field that is present with a value other than exact JSON boolean `false`.

Use this command before claiming Wave 2 progress in a handoff. A healthy local
tree should usually report most local lanes as proven while still showing the
external gates for ServiceNow governed-write evidence, governed-action live
readiness, the source-bound mobile candidate checker, and managed release
authority.
The non-JSON output uses the same split, printing total lanes, local executable
lanes, branch-level local preflight lanes, release-ready lanes, and remaining
external gates separately.
`scripts/appfw product wave2-status --json` fails and points back to the
framework namespace.

### `product compat-verify`

`scripts/appfw product compat-verify --json` writes
`.appfw/target/appfw/compat-verify.json` with the first P6 report-only posture
check for packaged framework compatibility. It inventories the product
`appfw.lock`, current framework package versions, App Framework Rust dependency
sources, frontend PDS package consumption, and framework package-manifest
evidence so product upgrade/release work can see whether it is still tied to a
local framework checkout.

```bash
scripts/appfw product compat-verify --json
```

The report includes:

- `lane:"P6"`;
- `appfw_lock` path, digest, and recorded framework values when present;
- `current_framework` package versions from the local framework checkout;
- `rust_dependencies` for App Framework crates and whether they use local
  paths or the approved registry;
- `framework_package_manifest` posture for package-distribution evidence;
- `transitional_items` for missing locks, local path dependencies, missing
  package manifests, and frontend package gaps;
- top-level `ok`, which is true only when all substantive checks pass;
- top-level and nested `gating_ok`, which remain true when every false check is
  explicitly report-only;
- `gate.enforced:false` until packaged ProGet consumption is the product
  default.

Because P6 remains report-only, transitional findings may produce `ok:false`
with `gating_ok:true` and exit zero. Missing required inputs such as the product
manifest produce both values false and a nonzero exit. This separation prevents
a top-level green without prematurely promoting report-only findings to gates.

`scripts/appfw product compat-verify --plan --json` remains available as the
Wave 0 planning surface. `scripts/appfw framework compat-verify --json` fails
and points back to the product namespace.

### `product mobile-test`

`scripts/appfw product mobile-test --json` writes
`.appfw/target/appfw/mobile-test.json` with the U5 legacy static diagnostic
verifier for React Native + Expo product mobile apps. M0-05 contains this
legacy path: every invocation emits `candidate_ready:false`,
`release_ready:false`, `release_authority:"none"`, and an explicit
non-authoritative `readiness_authority`. It is intentionally fast:
it inspects the product-owned `mobile/` workspace and retained metadata instead
of running npm, simulators, or Expo services. It stages diagnostic U5 inputs
under `target/appfw/wave2/` when their product-owned source files exist and
invalidates stale staged copies when a source disappears.

```bash
scripts/appfw product mobile-test --json
```

Use `--run-local` after installing mobile dependencies when you want the CLI to
run and retain local React Native checks:

```bash
scripts/appfw product mobile-test --run-local --json
```

This deeper mode writes the normal `.appfw/target/appfw/mobile-test.json`
artifact plus command evidence under the product-owned mobile workspace:

```text
mobile/.appfw-mobile/typecheck-evidence.json
mobile/.appfw-mobile/test-evidence.json
mobile/.appfw-mobile/expo-doctor-evidence.json
mobile/.appfw-mobile/npm-audit-evidence.json
```

`--run-local` runs `npm run typecheck`, `npm run test`, `npm run doctor`, and
`npm audit --omit=dev --json`

Every `mobile-test` mode records a compatibility-named `release_artifacts`
array and stages the
source evidence that exists:

```text
target/appfw/wave2/u5-mobile-test.json
target/appfw/wave2/u5-npm-audit-evidence.json
target/appfw/wave2/u5-npm-audit-disposition.json
target/appfw/wave2/u5-device-evidence.json
target/appfw/wave2/u5-store-track-evidence.json
```

Every entry remains `candidate_ready:false` and `release_ready:false`;
`legacy_condition_satisfied` retains the old diagnostic result without granting
authority. Staging is for inspection and transition compatibility only. It
cannot upgrade a static scaffold, audit finding, simulator result, or
store-track record into U5 candidate or release readiness.

`mobile/.appfw-mobile/npm-audit-disposition.json` is not generated by
`--run-local`; it is an optional compatibility-named diagnostic input that
documents an approved, unexpired decision when runtime audit findings remain.
It grants no candidate or release authority to `mobile-test`.
It exits non-zero if any requested local check fails or if the mobile workspace
is not ready to execute those commands. Incomplete diagnostic observations also
make the requested diagnostic run fail, for example Jest evidence that passed
with `No tests found`, Expo Doctor evidence that ignored network failures, or a
runtime audit disposition without valid retained audit evidence.
It still does not claim simulator, physical-device, TestFlight, Play, push,
biometric, deep-link, or background-behavior certification.

If runtime `npm audit` findings remain, product teams must either upgrade them
away or retain a formal release disposition in:

```text
mobile/.appfw-mobile/npm-audit-disposition.json
```

The disposition is valid only when it is explicit, release-approved, unexpired,
and tied to the retained audit evidence:

```json
{
  "version": 1,
  "lane": "U5",
  "name": "npm-audit-runtime-disposition",
  "status": "accepted_risk",
  "release_approved": true,
  "approved_by": "release-authority-or-security-review",
  "reason": "Short business justification and compensating controls.",
  "expires_on": "2026-09-30",
  "audit_evidence": "mobile/.appfw-mobile/npm-audit-evidence.json"
}
```

Allowed `status` values are `accepted_risk`, `mitigated_by_controls`, and
`upgrade_planned`. The CLI reports invalid or expired dispositions separately;
it does not silently convert a failed runtime audit into a satisfied legacy
condition, and no audit result can make this command candidate- or release-ready.
When the retained npm audit evidence was produced by `mobile-test --run-local`,
the report also includes a remediation summary with direct affected packages,
fix-available counts, semver-major fix counts, and a recommended next action.
Use that field to decide whether the next move is an SDK/dependency upgrade or
a time-boxed release-authority disposition.

Use `--device-preflight` to retain local simulator/emulator/store tooling
posture without launching a simulator or claiming device certification:

```bash
scripts/appfw product mobile-test --device-preflight --json
```

That mode writes:

```text
mobile/.appfw-mobile/device-tooling-evidence.json
```

The evidence reports whether `xcrun`, `xcodebuild`, `adb`, Android `emulator`,
`eas`, and `maestro` are available, along with readiness flags for iOS
simulator, Android emulator, store-track, and E2E preflight. Missing tools are
retained as evidence, not hidden, and affect diagnostic posture only; top-level
candidate and release readiness remain false.

Actual simulator/device smoke evidence belongs in:

```text
mobile/.appfw-mobile/device-evidence.json
```

A static placeholder such as `status:"not-run"` may exist to reserve the
evidence location. To satisfy the legacy device-smoke diagnostic condition,
the evidence must use the U5 compatibility shape below. Even a valid record
cannot make `candidate_ready` or `release_ready` true:

```json
{
  "version": 1,
  "lane": "U5",
  "name": "mobile-device-smoke-evidence",
  "status": "passed",
  "platform": "ios",
  "app_profile": "preview",
  "device_target": "iPhone simulator or managed device pool label",
  "tested_at": "2026-07-02T12:00:00Z",
  "command": ["npm", "run", "test:device"],
  "evidence_artifacts": [
    "mobile/.appfw-mobile/artifacts/ios-smoke-report.json"
  ]
}
```

Retained TestFlight, Play, or enterprise release-track evidence belongs in a
separate artifact so device smoke evidence cannot be mistaken for store-track
certification:

```text
mobile/.appfw-mobile/store-track-evidence.json
```

A static placeholder such as `status:"not-run"` may reserve this location. To
satisfy the legacy store-track diagnostic condition, the evidence must use the
U5 compatibility shape below. It remains non-authoritative input:

```json
{
  "version": 1,
  "lane": "U5",
  "name": "mobile-store-track-evidence",
  "status": "certified",
  "release_track": "testflight",
  "app_profile": "preview",
  "build_id": "ios-preview-2026.07.02.1",
  "runtime_version": "1.0.0",
  "binary_digest": "sha256:...",
  "signing_identity": "PDS mobile signing identity",
  "tested_at": "2026-07-02T12:00:00Z",
  "approved_by": "mobile-release-authority",
  "store_metadata": {
    "status": "ready-for-review"
  },
  "evidence_artifacts": [
    "mobile/.appfw-mobile/artifacts/testflight-build.json"
  ]
}
```

Allowed `release_track` values are `testflight`, `play-internal`,
`play-closed`, `production`, and `enterprise-mdm`. The CLI reports invalid or
placeholder device/store-track evidence separately so tiny files such as
`{"status":"passed"}` or `{"status":"certified"}` cannot create a false mobile
release signal.

The report includes:

- `lane:"U5"`;
- `native_runtime:"react-native-expo-new-architecture"`;
- `readiness_level`, capped at `static-scaffold` or
  `missing-static-scaffold`;
- `legacy_evidence_satisfied`, which preserves whether every old diagnostic
  facet passed without granting authority;
- fail-closed `candidate_ready:false`, `release_ready:false`,
  `release_authority:"none"`, and `readiness_authority` with M0-05 traceability;
- `inputs` for the mobile plan, workspace, generated contract, ownership
  metadata, scaffold manifest, runtime-audit disposition, device evidence, and
  store-track evidence;
- `runtime_audit`, including retained audit evidence, optional disposition
  validity, audit-unavailable posture when the registry could not be reached,
  and whether the legacy audit condition is satisfied;
- `command_evidence`, including whether retained typecheck, test, and Expo
  Doctor evidence satisfies the legacy diagnostic; zero-test Jest output and
  Expo Doctor runs that ignored network errors do not satisfy it;
- `device_evidence`, including the retained evidence status, placeholder versus
  compatibility evidence, schema violations, and its legacy diagnostic result;
- `store_track_evidence`, including the retained track status, placeholder
  versus compatibility evidence, schema violations, and its legacy diagnostic
  result;
- checks for `mobile/package.json`, `app.json`, `eas.json`, `tsconfig.json`,
  generated contract consumption, PDS native token bridge, generated ownership,
  auth/tenant/policy posture, offline/secure-storage posture, and device
  evidence;
- `local_execution` command and evidence paths when `--run-local` or
  `--device-preflight` is requested;
- `blocking_violations` when a required mobile readiness artifact is missing.

The command exits non-zero when the product has not created the mobile
workspace yet, while still retaining the JSON artifact. Once the static
workspace exists, it can return `ok:true` with `readiness_level:"static-scaffold"`
and fail-closed candidate/release fields. `ok:true` means only that requested
diagnostic checks completed without blocking violations. Even when
`legacy_evidence_satisfied:true`, this command cannot advance Wave 2,
candidate, distribution, OTA, store/MDM, or release state. The future
source-bound mobile candidate checker must prove real API/auth, both platforms,
provenance, Fabric authority, update recovery, and comprehensive review.

`mobile-test --run-local` covers the local npm/Expo-doctor/runtime-audit layer.
`mobile-test --device-preflight` covers local device tooling detection. It is
not an emulator launcher today. The future source-bound candidate checker—not a
readiness promotion mode of `mobile-test`—should read a product mobile
capability/test matrix and retain exact-source simulator/device run artifacts.
Do not encode machine-specific simulator names, emulator IDs, or device UUIDs
in `.appfw/model`.

`scripts/appfw product mobile-test --plan --json` remains available as the
Wave 0 planning surface. `scripts/appfw framework mobile-test --json` fails and
points back to the product namespace.

### Wave 0 Plan Compatibility Commands

These commands preserve North-Star Wave 0 `--plan --json` responses for agents
even after the executable evidence lane starts. Plan responses are metadata;
normal `--json` execution is the retained evidence path.

```bash
scripts/appfw product mobile-test --plan --json
```

Every plan response includes:

- `status:"reserved-wave-0-contract"`;
- the canonical `namespace`;
- the Roadmap lane, such as `U5`;
- the future artifact path;
- source docs and future proof expectations.

Namespace checks are enforced even in plan mode. For example,
`scripts/appfw framework mobile-test --plan --json` fails and points back to
`scripts/appfw product mobile-test --plan --json`.

The reserved artifacts are:

| Command | Namespace | Evidence artifact |
| --- | --- | --- |
| `mobile-test` | product | `.appfw/target/appfw/mobile-test.json` |
| `chat-eval` | product | `target/appfw/chat-eval.json` |

When a lane graduates from plan-only to executable evidence, keep the command
name and JSON envelope stable, then add the real checks behind it and update
the matching release/docs-check assertion.

### `framework provider-graduation`

`scripts/appfw framework provider-graduation --json` writes
`target/appfw/provider-graduation.json` and prints the same report. The command
is the U4 report-only evidence lane: it summarizes every framework provider's
capability areas, counts unsupported and ungraduated areas, and fails if a
promoted capability lacks the evidence required by its status. It does not run
live provider tests and does not promote a connector by itself.
When `APPFW_PROVIDER_CERTIFICATION_EXPORT_BIN` points to a packaged or local
export binary, the command uses that binary directly instead of invoking Cargo.

`scripts/appfw framework provider-graduation --plan --json` remains available as
the original reserved-plan response for agents that need the Wave 0 slot
metadata.

The retained report has this stable shape:

```json
{
  "command": "provider-graduation",
  "lane": "U4",
  "ok": true,
  "mode": "report-only",
  "summary": {
    "provider_count": 12,
    "unsupported_capability_count": 103,
    "ungraduated_capability_count": 116,
    "graduated_capability_count": 105,
    "promotion_violation_count": 0
  }
}
```

### `framework governed-write-check`

`scripts/appfw framework governed-write-check --json` writes
`target/appfw/governed-write-posture.json` and prints the same report. This is
the G1 posture lane: it inspects the current provider graduation artifact,
lists every external API provider, and proves SaaS write-back remains
fail-closed until provider-test/live evidence exists for a named mutation.
Normal mode records posture. `--enforce` fails if provider-graduation evidence
is missing, not ok, or a provider claims governed-write certification without
valid G1 evidence. `release-check` runs the enforced mode after
`provider-graduation`.

The command deliberately does **not** write
`target/appfw/governed-write-evidence.json`. That file is reserved for real G1
provider-test evidence and is validated by `release-evidence-check` only when a
release-check provider scope claims `governed_write_certified:true`.

```bash
scripts/appfw framework governed-write-check --json
scripts/appfw framework governed-write-check --json --enforce
scripts/appfw framework governed-write-check --plan --json
```

The retained posture report has this stable shape:

```json
{
  "command": "governed-write-check",
  "lane": "G1",
  "ok": true,
  "artifact": "target/appfw/governed-write-posture.json",
  "evidence_artifact": "target/appfw/governed-write-evidence.json",
  "gate": {
    "enforced": true,
    "ready_to_enforce": true
  },
  "providers": [
    {
      "provider": "servicenow",
      "family": "external_api",
      "governed_write_certified": false,
      "write_enabled": false,
      "mcp_enabled": false,
      "gates": []
    }
  ],
  "certified_providers": [],
  "blocking_violations": []
}
```

### `framework saas-lineage`

`scripts/appfw framework saas-lineage --json` writes
`target/appfw/saas-freshness-lineage.json` and prints the same report. This is
the first DP4 executable posture slice for Archetype-1 SaaS materialization:
it reads the product's normalized `sync_descriptors.json`, generated
`sync_workers.yaml`, optional `sync_worker_plan.json`, and optional live evidence
to report freshness, lineage, provenance, echo-loop, and redaction readiness.

```bash
scripts/appfw framework saas-lineage --json
scripts/appfw framework saas-lineage --plan --json
scripts/appfw framework saas-lineage --enforce --json
```

Normal mode is report-only and may be `ok:true` while `release_ready:false`.
`--enforce` writes `target/appfw/saas-freshness-lineage-enforced.json` and fails
closed until the product has sync descriptors plus a release-grade
`appfw.saas.freshness-lineage.v1` evidence file. Use
`APPFW_SAAS_LINEAGE_EVIDENCE_FILE` or `--evidence-file <path>` to validate an
isolated runtime artifact.

The retained report records:

- `lane:"DP4"`;
- descriptor freshness SLO and watermark coverage;
- generated sync-worker fail-closed posture;
- the release evidence contract;
- blocking violations when enforced evidence is missing or invalid.

### `scripts/appfw dependency-check`

Checks dependency upgrade pressure and supply-chain advisory posture without
modifying manifests, lockfiles, or installed packages.

```bash
scripts/appfw dependency-check
scripts/appfw dependency-check --json
scripts/appfw dependency-check --json --online
scripts/appfw dependency-check --json --strict
scripts/appfw dependency-check --json --strict --policy-file dependency-check.toml
```

Default mode is offline and local-only. It reads tracked Cargo manifests,
`Cargo.lock`, tracked npm lockfiles, `cargo metadata --locked`, and
`cargo tree -d --locked`; it attempts local `cargo audit --no-fetch` and
`cargo deny --offline` evidence when their advisory databases are already
available. Missing local advisory caches are reported as warnings outside
strict mode so agents can still get upgrade guidance in restricted
environments.

Rust advisory tools use a writable Cargo home for their RustSec/advisory cache.
Set `APPFW_DEPENDENCY_CHECK_CARGO_HOME` when CI must use a pinned cache volume.
When it is unset, the checker uses writable `CARGO_HOME`/`$HOME/.cargo` when
available and falls back to `target/appfw/cargo-home` if the ambient cache is
not writable. The selected path and source are retained under
`tool_environment.advisory_tools` in `target/appfw/dependency-check.json`.

`--online` enables network-backed checks:

- `cargo audit` and `cargo deny` may fetch the RustSec advisory database.
- npm `package-lock.json` roots run `npm audit --json --package-lock-only` and
  `npm outdated --json`.
- direct Rust dependencies are compared with the crates.io registry.
- locked Rust and npm packages are batched through OSV so `RUSTSEC`, `GHSA`,
  `OSV`, and `CVE` aliases are reported together.

`--strict` is the CI/release-evidence mode. It implies online checks unless
`--offline` is explicitly supplied, fails when required advisory tooling is
missing, and treats known vulnerability findings or policy failures as
blocking unless they have an active, source-controlled acceptance in
`dependency-check.toml` or the file passed with `--policy-file`. Accepted OSV
findings are not hidden: the report records the matched package, advisory ID,
aliases, reason, owner, source file, and `until` date. The same active GHSA
acceptances also cover `npm audit` leaf advisories in a lockfile when every
extracted leaf GHSA is accepted; inherited ancestor rows are not a separate
exception, and any unaccepted leaf GHSA still blocks. Expired or invalid
acceptances fail strict mode. Duplicate crate compatibility lines, stale direct
dependencies, and reviewed unmaintained dependency debt remain advisory unless
they are tied to an unaccepted known vulnerability or supply-chain policy
failure.

Product mobile workspaces may retain runtime npm audit findings while the
mobile app is still a non-release scaffold. When a tracked `package-lock.json`
has adjacent U5 mobile audit evidence under `.appfw-mobile/`, strict
dependency-check keeps the npm audit and OSV findings in the report but marks
that lockfile as `mobile_release_gate.status:"mobile-release-gated"`. Those
findings do not block the framework PR supply-chain gate by themselves because
`scripts/appfw product mobile-test --json` retains the audit as a visible
legacy diagnostic while remaining non-authoritative and fail-closed. The
future source-bound mobile candidate checker—not `mobile-test`—must require a
clean runtime audit or an explicitly approved, unexpired, evidence-bound
disposition before candidate consideration. Release authority remains outside
both commands.

JSON mode writes and prints:

```text
target/appfw/dependency-check.json
```

The report contains the command mode, executed checks, Cargo and npm inputs,
duplicate Rust dependency versions, direct dependency upgrade advice, cargo
audit/cargo-deny status, npm audit/outdated status, optional OSV findings, and
blocking findings. It also records active, expired, and unused OSV acceptances
from the dependency-check policy file so release reviewers can see why strict
mode passed when a vulnerability is temporarily accepted. The CI supply-chain
gate runs
`scripts/appfw dependency-check --json --strict` and records this file as a
required release artifact.

### `scripts/appfw dependency-plan`

Explains the impact of upgrading one dependency without modifying files.

```bash
scripts/appfw dependency-plan --package async-graphql
scripts/appfw dependency-plan --package async-graphql --target 7.2.1
scripts/appfw dependency-plan --package async-graphql --online --json
```

The plan reads `Cargo.lock`, tracked `package-lock.json` files, direct
workspace requirements, duplicate Rust versions, reverse `cargo tree -i`
output, and package-specific upgrade advice. With `--online`, it also resolves
the latest registry version for the requested Rust or npm package. JSON mode
writes and prints:

```text
target/appfw/dependency-plan-<package>.json
```

The report contains Rust manifest owners, locked versions, reverse dependency
trees, affected npm lockfile roots, target version, risk reasons, plan steps,
and recommended verification commands. Use this before changing manifests or
lockfiles when an upgrade might cross a major compatibility line, touch
runtime ingress/auth/provider code, or remove an accepted OSV finding.

### `scripts/appfw dependency-upgrade`

Dry-runs or applies a single dependency upgrade using the package manager for
the affected ecosystem.

```bash
scripts/appfw dependency-upgrade --package async-graphql --target 7.2.1 --check
scripts/appfw dependency-upgrade --package vite --target 6.4.3 --check
scripts/appfw dependency-upgrade --package vite --target 6.4.3 --apply
scripts/appfw dependency-upgrade --package vite --target 6.4.3 --apply --run-tests --json
```

Default mode is dry-run and writes:

```text
target/appfw/dependency-upgrade-<package>.json
```

For Rust dependencies, `--check` runs `cargo update -p <package> --precise
<target> --dry-run` after building the dependency plan. `--apply` updates
direct `Cargo.toml` requirements for the package and refreshes `Cargo.lock`
with `cargo update -p <package> --precise <target>`. For npm dependencies,
`--check` runs package-lock-only `npm install --dry-run`; `--apply` runs the
same package-lock-only install without `--dry-run`, preserving the root
dependency kind (`dependencies`, `devDependencies`, or `optionalDependencies`).

`--run-tests` is only valid with `--apply`. It executes the recommended
verification commands from the plan: impacted `cargo check --locked -p ...`
commands, affected npm audit/build commands, `scripts/appfw validate --json`,
`scripts/appfw test --fast`, and strict dependency-check evidence. Keep this
step for upgrades that affect runtime ingress, auth, providers, generated
contracts, or release-blocking advisories.

### `scripts/appfw boundary-check`

Runs the fast product extension boundary check without compiling the generated
server crate.

```bash
scripts/appfw boundary-check
scripts/appfw boundary-check --json
```

The check parses product-owned handlers and services, rejects direct imports of
framework runtime internals, rejects direct GraphQL request-context access, and
verifies that standard handler overrides carry an explicit
`appfw: override-standard` marker. It also scans the product template root for
retired framework implementation files so CRM sample and `appfw new` output do
not quietly reintroduce copied runtime/provider support surfaces. With
`--json`, stdout is `.appfw/target/appfw/boundary_check.json`-compatible
report content.

The JSON report also includes `product_provider_sources`, which lists provider
adapter source directories and provider package dependencies present in each
checked product root, the providers used by configured schemas, and the Rust
file count for each provider adapter. If a product backend carries source for a
provider that no schema uses, the check fails with `inactive_provider_source`.
If `backend/Cargo.toml` keeps an inactive provider package dependency, the
check fails with `inactive_provider_dependency`. This keeps generated product
templates from regrowing inactive MongoDB, MS SQL Server, PostgreSQL, or
Snowflake implementation copies during provider packaging work.

### `scripts/appfw feature-check`

Verifies that runtime ingress modules compile independently and that no-HTTP
builds keep HTTP ingress packages out of the dependency graph.

```bash
scripts/appfw feature-check
scripts/appfw feature-check --json
scripts/appfw feature-check --plan --json
```

The check builds `appfw-runtime` with no default features, then with `http`,
`mcp`, and `kafka` one at a time. It also checks the current product backend
with no ingress features, with each ingress feature, and with each declared
provider feature (`provider-postgres`, `provider-mongo`, `provider-mssql`,
`provider-snowflake`, and `provider-neo4j`). From the framework
checkout, the current product backend defaults to `examples/products/crm/backend`
because that sample is the source shape for `appfw new --profile crm-sample`.

For no-HTTP builds, the command also inspects `cargo tree` and fails when
HTTP ingress packages such as `axum`, `async-graphql-axum`, or the framework
route-layer `tower-http` stack are present. Generic HTTP client dependencies
used by telemetry, auth, or providers are not treated as ingress bindings.

Use this after changing runtime host wiring, generated backend feature flags,
MCP, Kafka, HTTP routing, or product template dependencies. `release-check`
runs this command automatically. JSON runs retain
`target/appfw/feature-check.json` as the U7 compile evidence artifact. Failed
Cargo or rustc output is copied to `target/appfw/feature-check-logs/` and
summarized in that JSON so diagnostics are not left only in runner `/tmp`.
When invoked directly, JSON runs also emit one `feature-check-start` and one
`feature-check-finish` JSONL event on stderr for every existing subcheck.
Callers may retain or redirect that stream; the current Wave 3 wrapper captures
it rather than surfacing it live. Finish events and retained check items include
additive `elapsed_ms`, `timing_ok`, and effective `exit_code` fields; the
report includes aggregate `elapsed_ms`, `timing_ok`, and `timing_clock`.
The clock domain is selected once per run. A missing, failed, malformed, or
backward sample records `elapsed_ms: 0` with `timing_ok: false` rather than
mixing clock domains or presenting zero as a valid duration. Timing evidence
does not change the matrix, command order, failure propagation, or aggregate
`ok` semantics.
Plan runs retain `target/appfw/feature-check-plan.json`; use `--plan --json`
when an agent or CI lane needs the matrix shape without running or clobbering
the Cargo feature-check evidence.

### `scripts/appfw test`

Runs local compile and unit checks across the Rust crates.

```bash
scripts/appfw test
scripts/appfw test --smoke
scripts/appfw test --smoke --plan --json
scripts/appfw test --fast
scripts/appfw test --json
```

The full command runs normal tests for `app_gen`, the product `backend`, the
framework-owned database runner against the product database package, and
product policy fixtures through the framework verifier harness, then compiles
`api_tests` without running backend-dependent scenarios. `--smoke` runs config
validation, `boundary-check`, and framework script syntax checks without Rust
compilation; use it for docs, routing, and checker-script edits before the
heavier gates. `--fast` runs config validation, `boundary-check`,
`appfw-runtime`, `appfw-cli`, and boundary-check unit coverage without compiling
product crates; use it for inner-loop packaging and extension-boundary work.
`--plan --json` is read-only and reports the selected test lane without running
it; docs-check uses the smoke plan contract because validation, boundary, shell
syntax, and checker syntax are already exercised by neighboring docs-check
subchecks. Use the non-plan `test --smoke` or broader `test --fast` command for
actual proof.

### `scripts/appfw policy-test`

Runs product policy fixtures against the framework-owned Rego verifier harness
in `appfw-test`.

```bash
scripts/appfw policy-test
scripts/appfw policy-test --json
```

Product repos keep policy source, fixtures, and product-specific assertions in
`rego_test`; the Rego evaluator and shared access input/result contract live in
`appfw_test/src/policy`.

### `scripts/appfw frontend-test`

Runs the product frontend evidence bundle when a frontend package exists. In
the framework reference checkout this resolves to
`examples/products/crm/frontend`; downstream products can set
`APPFW_FRONTEND_ROOT`.

```bash
scripts/appfw frontend-test
scripts/appfw frontend-test --json
```

The CRM reference frontend command runs scaffold package checks, route identity
tests, typecheck, production build, Playwright browser E2E tests, and axe
accessibility tests against a deterministic mocked CRM backend. It writes:

```text
target/appfw/frontend-test.json
target/appfw/frontend-test.log
examples/products/crm/frontend/test-results/playwright-results.json
examples/products/crm/frontend/playwright-report/index.html
```

### `scripts/appfw product chat-eval`

Runs the deterministic CH6 conversational-answer safety harness for a product
workspace. It is network-free and validates recorded/stubbed chat transcripts
against answer-envelope, named-tool binding, citation resolvability, tenant
isolation, write-gate, and replay-determinism contracts.

```bash
scripts/appfw product chat-eval --json
scripts/appfw product chat-eval --json --inject-leak
scripts/appfw product chat-eval --plan --json
scripts/appfw product chat-eval --json --pipeline-plan
```

The normal run writes:

```text
.appfw/target/appfw/chat-eval.json
target/appfw/wave4/ch6-chat-eval.json
```

The pipeline-plan run keeps the local fixture posture and also writes:

```text
.appfw/target/appfw/chat-eval-promptfoo-plan.json
target/appfw/wave4/ch6-chat-eval-promptfoo-plan.json
```

The artifact is posture evidence only: it must report
`release_ready:false`, `mode:"local-fixture"`, judge disabled, and
`red_team.leaks_found:0`. `--inject-leak` intentionally injects a seeded
cross-tenant value, must fail with a `policy_leak` finding, and writes
`.appfw/target/appfw/chat-eval-inject-leak.json` by default so it does not
overwrite the retained passing posture artifact. The passing local-fixture run
also stages `target/appfw/wave4/ch6-chat-eval.json` for release-evidence
validation. The promptfoo pipeline-plan artifact must report
`mode:"pipeline-plan"`, `offline_only:true`, and `release_ready:false`; it is a
pipeline assertion plan, not production judge evidence. Live/judge certification
must use the separate
`target/appfw/chat-eval-judge-evidence.json` artifact, which
`scripts/ci/release-evidence-check.sh` validates when retained and requires
when `APPFW_REQUIRE_CHAT_EVAL_JUDGE_EVIDENCE=true` or
`APPFW_RELEASE_REQUIRE_CHAT_EVAL_JUDGE_EVIDENCE=true`.

### `scripts/appfw mobile-plan`

Plans a React Native + Expo conversion from a mobile HTML mockup or equivalent
UI artifact. This command is available now as planning evidence; it does not
generate RN source yet. Use it before an agent creates or changes product-owned
`mobile/` code.

```bash
scripts/appfw product mobile-plan --ui-artifact prototype.html --json
scripts/appfw product mobile-plan --ui-artifact prototype.html \
  --target react-native-expo \
  --app-name operations-mobile \
  --json
```

The command validates that the source artifact exists, records its byte size
and SHA-256 hash, names the `product-mobile-react-native` skill, and writes:

```text
.appfw/target/appfw/mobile-rn-conversion-plan.json
```

The retained plan distinguishes current and future contracts:

- current: convert the mockup by following
  `docs/frontend/mobile-react-native.md`, keeping product-owned source under
  `mobile/`, then retain non-authoritative static diagnostics with
  `scripts/appfw product mobile-test --json`; candidate and release readiness
  remain false;
- current generated contract/token bridge target:
  `scripts/appfw product generate --target mobile-rn --json` and
  `scripts/appfw product generate --target mobile-rn --check --json`, which
  emit/check the generated mobile contract, PDS token bridge, ownership
  metadata, scaffold manifest, and delegate to `mobile-test`;
- future: deeper generated mobile workflow source emission plus a distinct,
  source-bound candidate checker that consumes exact-source typecheck, test,
  Expo, API/auth, and both-platform device evidence. Named humans continue to
  own distribution and release decisions.

Do not use the HTML mockup as implementation source. It is workflow and visual
evidence that must be mapped back to `.appfw/model`, generated operations,
product services, PDS native tokens, auth, tenant, policy, validation, and
diagnostic evidence plus the future source-bound candidate contract. Use
`mobile/src/generated/appfw-mobile-contract.ts` as canonical mobile input; Web
generated TypeScript is migration or diagnostic evidence only.

### `scripts/appfw api-test`

Runs product-generated API scenarios against a running backend. The command
filters the test crate to `schemas::`, so framework-owned provider
certification contracts do not run as part of product scenario verification.

```bash
scripts/appfw api-test
```

The default API test harness expects:

```text
API_TEST_BASE_URL=http://localhost:8080
API_TEST_TIMEZONE=America/Denver
API_TEST_AUTH_MODE=bypass
```

Typical local setup:

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

See `api_tests/README.md` for token-file mode and scenario authoring.

### `scripts/appfw provider-test`

Runs the framework-owned live provider certification contracts against a
running backend. The command filters the test crate to the provider
certification modules, so generated product scenarios do not run as part of
provider parity verification.

```bash
scripts/appfw provider-test --provider postgres
scripts/appfw provider-test --provider mongo
scripts/appfw provider-test --all --json
scripts/appfw provider-test --provider postgres --json provider_stored_routine_return_payload_contract
scripts/appfw framework provider-test --provider salesforce --area saas-read --plan --json
scripts/appfw framework provider-test --provider servicenow --area governed-write --plan --json
scripts/appfw framework provider-test --provider ai_search --plan --json
```

`--provider` sets `API_TEST_PROVIDER` and the matching data source hint for the
test process. The backend itself must already be running with the same
`APP_DATA_SOURCE_NAME`.

`provider-test` defaults `API_TEST_AUTH_MODE` to `local_dev` so policy and
access-filter contracts exercise non-admin local test users. Override it only
when running against token-backed environments that provide equivalent users.
The command also sets `API_TEST_PROVIDER_CERTIFICATION=1`, which turns skipped
security contracts into failures when the auth mode cannot exercise named
policy users. Token-backed environments need equivalent `pdsh_admin`,
`other_tenant_admin`, `cc_tenant_user`, `tenant_one_crm_ops`, and
`west_sales_rep` tokens. `tenant_one_crm_ops` must use the existing tenant-1
`crm_ops` role so certification can read actor-tenant audit evidence without
granting audit access to the denied `cc_tenant_user` actor.

Provider certification runs live tests serially inside each provider process.
Direct `cargo test` runs skip provider certification contracts unless
`API_TEST_PROVIDER_CERTIFICATION=1` is set.
The contracts share provider state intentionally, so serial execution keeps
the release signal deterministic.
Pass a non-option test name after the provider selector to run one live
contract through the same preflight, environment, and JSON artifact path.
Targeted runs report `mode: targeted` and do not satisfy the full provider
area gate; release certification still requires the unfiltered provider run.

External API SaaS read certification uses the same namespace but starts as a
DP6 plan lane:

```bash
scripts/appfw framework provider-test --provider salesforce --area saas-read --plan --json
```

The command writes `target/appfw/saas-read-provider-test.json` with
`lane:"DP6"`, `schema_version:"appfw.saas_read_provider_test.plan.v1"`,
the 12 SaaS read checks, the provider's current compiler-contracted/unsupported
area rows, omitted write areas, `required_live_inputs`, and a redacted live
evidence template. Plan mode is intentionally `release_ready:false` and must not
write `target/appfw/saas-read-evidence.json`; live SaaS read certification waits
for the W3-A runtime executor plus provider-backed live-smoke fixtures.

External API governed-write certification uses the same `provider-test`
namespace but a separate G1 area. The planning form is safe in local and PR
lanes:

```bash
scripts/appfw framework provider-test --provider servicenow --area governed-write --plan --json
```

It reports `lane:"G1"`, the required delegated-actor/token-store/named-mutation
checks, `required_live_inputs`, `known_named_mutations`, and an
`evidence_template` for the live provider-backed runner. For ServiceNow, the
first allow-listed mutation candidate is `servicenow.create_incident` with
policy scope `servicenow.incident.write`; it remains non-executable until the
live runner proves every G1 gate. The required inputs include connection/auth
variables, operation-contract variables, and the external evidence-file hook.
Connection/auth variables identify the sandbox, delegated user/tenant, auth
mode, and token-store reference; they must not be copied into the retained
evidence. Operation-contract variables include
`APPFW_SERVICENOW_GOVERNED_WRITE_MUTATION_NAME`,
`APPFW_SERVICENOW_GOVERNED_WRITE_POLICY_SCOPE`,
`APPFW_SERVICENOW_GOVERNED_WRITE_TEST_IDEMPOTENCY_KEY`,
`APPFW_SERVICENOW_GOVERNED_WRITE_AUDIT_SINK`, and
`APPFW_SERVICENOW_GOVERNED_WRITE_TEST_INGRESS`. `TEST_INGRESS` must be one of
`http`, `mcp`, or `kafka` when set. Live evidence retention also requires
`APPFW_SERVICENOW_GOVERNED_WRITE_EVIDENCE_FILE`, pointing at the JSON artifact
produced by an external provider-backed sandbox runner. Plan mode does **not**
write `target/appfw/governed-write-evidence.json`. Without `--plan`, the command
writes `target/appfw/governed-write-provider-test.json` as a fail-closed
preflight and exits non-zero until the configured evidence file proves every G1
gate, including explicit `redaction.secrets_removed`,
`redaction.raw_payloads_removed`, and `redaction.tenant_data_removed` claims
plus rejection of obvious access-token, authorization, tenant-data, PHI, and
raw-payload fields. Only then does it retain
`target/appfw/governed-write-evidence.json`. That makes the remaining G1 work
executable and auditable without allowing a local posture report to masquerade
as live write evidence. A retained G1 artifact is still not managed release
authority; Wave 2 and production promotion remain gated by the broader release
evidence bundle.

AI search certification uses a separate CH3/CH7 plan-only provider family:

```bash
scripts/appfw framework provider-test --provider ai_search --plan --json
```

It writes `target/appfw/ai-search-provider-test.json`, reports
`known_named_queries:["ai_search.embed","ai_search.search"]`, names the CH7
gateway/auth evidence (`gateway-selection-decision`,
`server-secret-ref-custody`, client-credentials token-cache posture,
prompt-audit/SIEM retention, egress allowlist, and policy re-resolution), and
keeps `release_ready:false`. Without `--plan`, the command fails closed because
executable AI search calls require the authenticated PDS AI search API
contract/export, gateway/auth evidence, recorded fixtures, and the shared SaaS
HTTP executor.

When testing the evidence schema itself, use an isolated report directory:

```text
APPFW_PROVIDER_TEST_REPORT_DIR=target/appfw/my-governed-write-sandbox \
APPFW_SERVICENOW_GOVERNED_WRITE_EVIDENCE_FILE=/path/to/sandbox-produced-evidence.json \
  scripts/appfw framework provider-test --provider servicenow --area governed-write --json
```

That pattern can validate a sandbox-produced file without creating the
release-reserved `target/appfw/governed-write-evidence.json`; the command still
requires the full governed-write environment and evidence shape described in
[SaaS Connector Certification](../runtime/saas-certification.md). Production
Wave 2 completion still requires the default release evidence location populated
from the real provider-backed runner.

For `--all`, CI can run four provider-backed backend instances and expose them
with:

```text
API_TEST_BASE_URL_POSTGRES
API_TEST_BASE_URL_MONGO
API_TEST_BASE_URL_MSSQL
API_TEST_BASE_URL_SNOWFLAKE
```

Release-grade `--all` certification requires those provider-specific URLs.
If a local smoke run intentionally points every provider at one backend, set
`APPFW_PROVIDER_TEST_ALLOW_SHARED_BASE_URL=true`; do not use that bypass as
release evidence.

The JSON report is written under the release evidence directory and mirrored
to the framework-root certification crate for compatibility:

```text
target/appfw/provider-parity.json
api_tests/target/provider-parity.json
```

The JSON report includes `generated_at_utc` plus an `areas` array per provider.
Each area records its declared certification status, the live contract used when
one is required, and the observed live result. This is the enterprise provider
certification gate:
`LiveCertified` areas must pass their mapped live contract, while
`CompilerContracted`, `Implemented`, `Partial`, `Unsupported`, and
`EmulatorLimited` areas are explicitly reported instead of being hidden behind a
single provider boolean.
Final release evidence rejects provider parity reports without a fresh
timezone-aware timestamp, reports that are not full-mode `--all` runs, missing
or out-of-bundle provider logs, and provider base URLs that do not match the
retained provider URL preflight snapshot. Provider and area identities must be
recorded once; duplicate entries are rejected instead of being collapsed into a
map. The final gate also cross-checks `security-certification.json` so its
introspection source is the canonical runtime routing source, and so its runtime
log, provider parity reference, retained `provider-parity` artifact, provider
log artifacts, and per-provider live security log references all point at the
same retained files used by the semantic evidence blocks and
`provider-parity.json`. Security certification evidence must also carry a fresh
timezone-aware `generated_at_utc` timestamp and must not predate the retained
provider parity report it consumes.
Targeted runs intentionally leave `areas` empty because they certify one named
contract, not the whole provider capability matrix.

`-- --list` is useful for contract discovery, but it is not a passing
certification run. A `listed` or `not-run` live result fails every
`LiveCertified` area; only an executed `passed` result satisfies the release
gate. The area matrix is exported from the Rust capability model rather than
duplicated in shell.

### `scripts/appfw local-live-preflight`

Runs the local four-provider certification preflight before opening a PR or
handing a branch to CI:

```bash
scripts/appfw framework local-live-preflight --plan --json
scripts/appfw framework local-live-preflight --json
```

Use `--plan` to print the required live-input and release-authority contract
without starting containers or provider backends.

The preflight starts PostgreSQL, MongoDB, MS SQL Server, and LocalStack
Snowflake from the CRM product compose file when
`APPFW_LOCAL_PREFLIGHT_START_SERVICES` is not set to `false`. It then migrates
each provider, builds the CRM backend once, launches one local backend per
provider on ports 8081 through 8084, writes a local provider URL preflight
snapshot, runs full `provider-test --all --json`, and writes live
`security-certification --json` evidence from the same retained provider
parity report.

When it needs to start LocalStack Snowflake, local preflight requires
`LOCALSTACK_AUTH_TOKEN`. The command reads the current environment first, then
`APPFW_LOCAL_PREFLIGHT_ENV_FILE` when set, otherwise repo-root `.env` followed
by the product app `.env`; already-exported environment variables take
precedence and secret values are not copied into the retained evidence. If the
LocalStack DNS name is not resolvable on the workstation, the Snowflake TCP and
SQL API probes fall back to `127.0.0.1` while preserving the published
`snowflake.localhost.localstack.cloud` service contract.

The backend build used by local preflight and the Bitbucket release gate uses
`APPFW_CERTIFICATION_BACKEND_FEATURES`, defaulting to
`http,provider-postgres,provider-mongo,provider-mssql,provider-snowflake`.
This intentionally compiles the certified HTTP provider surface rather than the
product backend's everyday default feature set.

Primary retained artifacts:

```text
target/appfw/local-live-release-preflight.json
target/appfw/local-live-release-preflight-plan.json
target/appfw/provider-parity.json
target/appfw/security-certification.json
target/appfw/local-live-preflight/provider-url-preflight.json
target/appfw/local-live-preflight/provider-test-output.json
target/appfw/local-live-preflight/security-certification-output.json
target/appfw/local-live-preflight/backend-*.log
target/appfw/local-live-preflight/migrate-*.json
```

`local-live-release-preflight.json` reports `ci_ready:true` only when provider
URL health checks, migrations, four backend startups, provider parity, provider
area evidence, and live security certification are green. It also keeps
`release_ready:false` by design: local preflight is CI-readiness evidence, not
production release authority. Production release authority still requires the
remote `release-check` lane and its live/governance artifacts, including PDS
baseline, release identity, operations, performance, security assurance,
production attestations, and final release evidence validation when those
policies are enabled.

The retained JSON also includes `required_live_inputs` for provider backend
URLs, provider service endpoints, and auth context, plus
`external_release_authority_gates` for the managed release artifacts that local
preflight can never satisfy. Those sections are present even when preflight
fails early, so CI and agents can tell whether the failure is missing live
inputs, a provider certification failure, or a release-authority gap.
The release-boundary fields are intentionally explicit:
`local_preflight_satisfies:false`,
`local_preflight_satisfies_release:false`, and
`authority:"managed-release-ci"` mean the local preflight can prepare a branch
for managed release CI but cannot itself certify production readiness.

### `scripts/appfw security-certification`

Writes release security certification evidence from focused runtime
introspection auth tests and retained live provider certification logs:

```bash
scripts/appfw security-certification --json
```

The command runs the GraphQL introspection authorization regression tests for
no-auth, fake-bearer/non-admin, and admin/developer-scope cases. It also
requires `target/appfw/provider-parity.json` to be a stable full-mode
`provider-test` report with one entry per release-certified provider, plus the
provider logs referenced by that report, then verifies tenant isolation,
locator/IDOR negative,
access-filter/denied mutation, denied-error normalization, and audit
redaction/chain contracts passed for PostgreSQL, MongoDB, MS SQL Server, and
Snowflake.

JSON evidence is written to:

```text
target/appfw/security-certification.json
```

This is release evidence, not a substitute for `provider-test`; it consumes the
provider evidence produced by the live provider gate. Final release evidence
rejects security certification reports that are stale, have an invalid
timestamp, or predate the retained provider parity report beyond clock-skew
tolerance. The final gate also revalidates retained security check names,
failing-check accounting, `failure_count`, retained `failures`, and
`release_blockers`; a security certification report cannot claim `ok:true` or
`release_ready:true` while any retained check, failure, or release blocker is
still present.

### `scripts/appfw release-check`

Runs the framework release gate:

```bash
scripts/appfw release-check
scripts/appfw release-check --json
```

The gate runs config validation, MCP posture, boundary checks, docs examples,
generated artifact drift checks, local compile/unit tests, policy evidence,
operations certification evidence, PDS baseline traceability evidence,
`scripts/appfw provider-test --all`, security certification evidence, and
handoff evidence. It expects
provider-backed backends to already be running and is intentionally stricter
than `scripts/appfw test`; this is the command CI should use before cutting a
framework release. It also runs `framework wave2-status --json` and retains
`wave2-readiness.json` in the active release report directory as report-only
North-Star posture evidence. When `release-check` uses a non-default report
directory, `wave2-status` writes the rollup there while preferring artifacts
already retained in that bundle and falling back to canonical `target/appfw`
posture evidence for lane artifacts that are produced outside the release
bundle. The Wave 2 rollup is visible to release reviewers but does not bypass
the strict provider, governed-write, mobile, governance, or release-authority
gates. Static
supply-chain evidence, including
`target/appfw/dependency-check.json`, is produced by
`scripts/ci/supply-chain-gate.sh` and validated with the retained release
evidence bundle. Retained PHI log-lint, supply-chain, dependency-check, SBOM,
and secret-scan reports must include fresh timezone-aware `generated_at_utc`
timestamps; stale, malformed, or future static evidence fails the final
release-evidence check. Static evidence artifact entries must also resolve to
the canonical retained files in the release artifact directory. The
supply-chain report hashes its retained dependency-check, PHI log-lint, SBOM
manifest, and CycloneDX artifact references, and SBOM manifest hashes are
rechecked against the retained CycloneDX files. SBOM manifest `spec_version`
and `component_count` values must also match the retained CycloneDX JSON
`specVersion` and component array length, and SBOM source lists must match the
tracked package lockfiles and deployable image sources for the release.
Retained dependency-check
reports must keep `blocking_findings` empty and every
`required:true` child check green with `status:"passed"` and exit code 0.
Pull-request pipelines also run `scripts/ci/release-lite-guard.sh`. The guard
does not start providers; it classifies the PR diff and writes
`target/appfw/release-lite-guard.json`. Provider/runtime/security-sensitive
changes fail closed unless the rerun supplies
`APPFW_RELEASE_LITE_EVIDENCE_URL`, `APPFW_RELEASE_LITE_APPROVER`, and
`APPFW_RELEASE_LITE_REASON` for an approved manual `release-check` or scoped
provider-backed release-lite run. These values must come from secured CI
variables; checked-in fallback approval values are not allowed. When the
variables are absent on a sensitive PR, the guard retains a missing-field
artifact for review. The focused docs-check proof is:

```bash
scripts/appfw framework docs-check --subcheck release-lite-guard --json
```

That subcheck exercises nonsensitive, sensitive-missing-approval, and
sensitive-approved cases without requiring live providers.
Secret-scan summary counts are recalculated from the retained redacted gitleaks
report plus the configured git-tracked baseline; any unbaselined fingerprint
fails the final release evidence gate.

`release-check` preflights the four provider-specific backend URLs before the
expensive local gate sequence. Missing, malformed, credential-bearing, or shared
`API_TEST_BASE_URL_POSTGRES`,
`API_TEST_BASE_URL_MONGO`, `API_TEST_BASE_URL_MSSQL`, or
`API_TEST_BASE_URL_SNOWFLAKE` fails quickly and records
`release-check-provider-url-preflight.log`. Use
`scripts/ci/bitbucket-release-gate.sh` or an approved live lane to start
provider backends and export distinct HTTP(S) base URLs before running the
release gate. Distinctness is enforced by scheme, host, and effective port;
path aliases on the same backend are rejected as shared provider URLs. Each URL
must also serve a reachable `/health/ready` endpoint during preflight; dead or
stale loopback URLs are recorded under
`invalid_provider_base_urls`. The preflight writes
`target/appfw/release-check-provider-url-preflight.json`; the top-level
missing/invalid provider URL arrays are derived from that single retained
snapshot. A passing snapshot must also retain one valid URL proof for each
release-certified provider, including the redacted health URL, passing HTTP
status, probe timestamp, and timeout. The snapshot and each retained health
probe timestamp must be generated within the final evidence check's 24-hour
freshness window.
The Bitbucket wrapper requires security-assurance and production-attestation
evidence, and defaults live operations, PDS baseline, performance evidence, and
release identity evidence requirements to enabled. Set those
`APPFW_RELEASE_REQUIRE_*` flags to `false` only for explicit non-production dry
runs. The Bitbucket `main` branch and custom `release-check` pipeline use that
focused non-production posture to retain provider-backed CI evidence without
claiming release authority; `v*` tag builds keep the strict defaults for
production release claims. The wrapper retains the effective posture in
`target/appfw/bitbucket-release-gate.json` under `release_requirements`.
Focused wrapper artifacts also carry `focused_evidence:true`; they are
CI-readiness evidence, not production promotion evidence. They also carry
`ci_ready`, `release_ready:false`, and
`release_authority:"focused-ci-only"` so aggregate status checks can reject them
as promotable release evidence even when the focused provider-backed slice
passed. In the strict lane, a wrapper artifact cannot assert
`release_ready:true` unless security assurance,
production attestations, operations certification, live ops evidence, PDS
baseline evidence, performance evidence, and release identity evidence were all
required. When a retained `bitbucket-release-gate.json`
already exists,
`scripts/ci/release-evidence-check.sh` validates it as optional wrapper
evidence: focused artifacts must keep `release_ready:false`, strict artifacts
must have `ok` match `release_ready`, `release_blockers` must match
`failure_summary.root_causes`, the timestamp must be fresh, and ready claims
must declare `release_authority:"strict-managed-release"` while matching the
retained `release-check` evidence with no retained failure-stage fields. The
wrapper writes
root-cause-first blocker evidence: when retained `release-evidence-check.json`
already has structured `failure_summary.root_causes`, those canonical causes
are promoted instead of duplicating child-report status lines or raw assertion
labels. Prior wrapper self-check causes such as stale
`bitbucket-release-gate.json` validation failures are not re-promoted when the
wrapper regenerates; the refreshed wrapper must summarize the current failed
stage and child evidence instead of carrying its own obsolete validation
failure forward. The final evidence checker validates that a retained non-ready
Bitbucket wrapper includes those canonical release-evidence root causes. A
retained wrapper that claims `release_ready:true` must also match the current
release-evidence run: no canonical root causes may remain. The wrapper's own
ready assertion also requires stable `command:"bitbucket-release-gate"`, a
fresh timezone-aware `generated_at_utc`, `release_blockers` and
`failure_summary.root_causes` to be present, made only of non-empty strings, and
equal, with no retained `failed_stage` or `exit_status` keys. When both artifacts
are retained, the wrapper timestamp must not predate the `release-check.json`
evidence it summarizes. A ready wrapper must also retain green child summaries:
`release_check.ok` and `release_check.release_ready`, provider certification
for PostgreSQL, MongoDB, MS SQL Server, and Snowflake, live operations
certification, live PDS baseline evidence, security-assurance decision evidence,
release identity evidence, MCP release posture, and the static/security gate
summary. The wrapper's ready assertion also reloads the retained child JSON
artifacts and requires those artifacts to prove the same green release posture,
with fresh timestamps that do not postdate the wrapper; the retained
`release-check.json` must also carry the same required green check set and
canonical artifact references, and the retained `provider-parity.json` must prove full-mode live certification with
non-empty live-certified areas and passed live contract records for every
release-certified provider. Required performance evidence must also retain a
populated `load-test-suite` summary with hash-bound passing scenario artifacts
plus a live-required `provider-performance` matrix tied to hash-bound retained
`provider-parity.json`, `performance_recommendations.json`, and QueryIR
budget-cap source evidence; hollow `ok:true` stubs are rejected. Security,
operations, PDS baseline, and final evidence-check child artifacts must also
carry green internal checks, zero failures and blockers, and their required live
authority/evidence structures before a wrapper can promote. The wrapper also
recomputes retained security-certification artifacts, live-ops artifacts, PDS
baseline decision/evidence artifacts, and SBOM manifest CycloneDX artifact
SHA-256 digests and byte sizes before treating those child reports as
release-authoritative. Supply-chain child artifact references for
dependency-check, PHI lint, SBOM manifest, and retained CycloneDX files must
also carry matching SHA-256 digests and byte sizes. SBOM manifest `spec_version`
and `component_count` values must also match the retained CycloneDX JSON
`specVersion` and component array length, and SBOM source lists must match the
tracked package lockfiles and deployable image sources for the release. Static
security evidence is also reloaded from the retained PHI lint, supply-chain,
dependency-check, SBOM
manifest, secret-scan, and gitleaks reports so embedded `security_gates`
summaries cannot mask hollow retained static artifacts. Dependency-check child
checks marked `required:true` must also remain green with zero exit status, and
`blocking_findings` must stay empty. The
retained security-assurance decision must prove production attestation
requirements, release-authoritative category dispositions, and retained evidence
artifacts whose SHA-256 digests and byte sizes still match the retained files.
Risk-accepted security-assurance categories must be explicitly accepted,
unexpired, bounded by the configured max-days window, and carry owner,
approver, release scope, rationale, compensating controls, and follow-up. The
retained MCP posture must be fresh, excluded, and match the wrapper summary.
The final evidence checker validates those embedded child summaries directly:
`release_check.checks` must include the release-ready check set, each retained
check must be green, artifact-bearing checks must retain artifact paths that
resolve to the expected files inside the configured release evidence directory,
operations and PDS summaries must require live evidence, and operations, PDS,
security-assurance, and MCP summaries must be green before a retained wrapper
can promote. The
`security_gates.checks` block must name each expected release security gate once,
all gate checks must be green, release-ready gate entries must carry
`release_ready:true`, and the ops/PDS gate entries must carry
`live_evidence_required:true`. The provider summary must name each
release-certified provider exactly once, retain its data source and base URL,
and show at least one live-certified area with all live-certified areas passed.
The wrapper assertion and final evidence checker both reject boolean or missing
live-certified counts; the counts must be real integer totals and all
live-certified totals must pass.

JSON mode writes the release report under:

```text
target/appfw/release-check.json
```

The JSON command exits successfully only when the retained report is
`release_ready:true`; a structurally valid report with `release_ready:false`
returns nonzero so CI jobs cannot accidentally promote a non-ready bundle by
checking only the process status.

The report includes a fresh timezone-aware `generated_at_utc`, `release_ready`,
`evidence_mode`, `provider_scope`, `missing_provider_base_urls`, `invalid_provider_base_urls`,
`failure_summary.root_causes`, and
`remediation_work_items`. The remediation IDs point at
`docs/release/live-environment-work-items.md` for live CI/CD, IaC,
observability, security, or release-authority work that cannot be completed in a
local checkout.
The `provider_scope` contract pins release-certified CRUD providers to
PostgreSQL, MongoDB, MS SQL Server, and Snowflake. It also records Neo4j as a
graph provider with `release_certified:false` and the certification posture in
`docs/runtime/graph-read-providers.md#certification-posture`; final release
evidence rejects a retained report that silently promotes graph support through
the CRUD release path.
The same scope records `external_api_providers` (`servicenow`, `workday`,
`icims`, `salesforce`, `anaplan`, and `oracle_financials`) with
`release_certified:false` and `governed_write_certified:false` until the G1
delegated-auth lane produces `target/appfw/governed-write-evidence.json`.
`scripts/ci/release-evidence-check.sh` requires and schema-validates that
artifact before any external API provider may claim
`governed_write_certified:true`.
`scripts/ci/release-evidence-check.sh` requires this artifact and validates the
same top-level contract, including timestamp freshness, before a retained evidence
bundle can pass final schema validation. In strict mode, retained release blockers make the command exit
nonzero until `release_ready:true`; `--local-fixture` is the schema-only mode.
The isolated local fixture path retains synthetic PDS baseline, release
identity, security-certification, supply-chain, dependency-check, PHI lint,
SBOM, secret-scan, and gitleaks child artifacts plus fixture decisions/evidence
files so the same retained path, digest, byte-size, decision-file matching,
security cross-reference, supply-chain child-artifact hashing, and PDS LIVE-015
through LIVE-021 evidence coverage checks run before CI; those fixture artifacts
are not release approvals.
The retained `checks` entries are part of that contract: names must be unique,
`ok` values must be booleans, failed checks must retain existing log files,
recorded artifact paths must resolve to retained files, timestamped check
artifacts must not postdate the top-level `release-check` timestamp, and
`current_check` must name the failed check that stopped `release-check` and be
absent when the top-level report is ready.
The provider URL preflight check must retain its snapshot artifact, and the
final evidence check verifies that the snapshot's missing/invalid provider URL
arrays exactly match the top-level release report. The snapshot must be retained
inside the same release evidence directory being validated. All retained
`release-check` logs and artifact paths must also resolve inside that release
evidence directory, so promotion jobs can archive one self-contained bundle.
Across the snapshot's missing, invalid, and valid URL entries, each
release-certified provider must appear exactly once; duplicates, overlaps, or
omitted providers fail the final gate.
Top-level `ok` and `release_ready` cannot be true while any retained check has
`ok:false`.
For child checks that retain a JSON artifact with `release_ready:false`, the
parent `release-check.checks[]` entry must also report `ok:false`; a
traceability-valid but non-ready PDS baseline or static ops artifact is a release
blocker, not a green child check.
When `release_ready:true`, the retained check set must include successful
`pds-baseline`, `provider-url-preflight`, `release-identity`,
`ops-certification`, `provider-test`, and `security-certification` entries, with
artifact-bearing checks pointing at the retained evidence files. Required
performance gates also require successful `load-test-suite` and
`provider-performance` entries.
It also validates `missing_provider_base_urls` and
`invalid_provider_base_urls` directly, so a retained `release-check.json` with
provider URL preflight gaps cannot be made promotable by editing only `ok` or
`release_ready`.
Provider URL entries must use canonical provider/env pairs, for example
`postgres` with `API_TEST_BASE_URL_POSTGRES`.
Invalid provider URL entries must include a safe `redacted_url`; unparseable
values use `<redacted-invalid-url>`, and retained diagnostics must not include
credentials, query strings, or fragments.
Provider URL preflight blockers must retain LIVE-001 and LIVE-002 remediation
items, PDS baseline blockers must retain LIVE-015 through LIVE-021 remediation
items, and release identity blockers must retain LIVE-004 and LIVE-014
remediation items. Strict evidence validation also revalidates
`target/appfw/release-identity.json` directly: the artifact command, booleans,
timestamp, git tag metadata, retained release identity decision digest/byte
size, retained license/notice files, package license metadata, retained
distribution artifact digest/byte metadata, required child checks, release
blockers, failure-summary root causes, and LIVE-004/LIVE-014 remediation must be
self-consistent before the evidence bundle can pass.

`scripts/appfw framework release-identity --json` writes
`target/appfw/release-identity.json`. Production release lanes set
`APPFW_RELEASE_REQUIRE_RELEASE_IDENTITY=true`; the report is release-ready only
when a `v*` tag is attached to the release SHA, root license/notice files and
package license metadata are present, and a retained
`release-identity-decision.json` declares `ok:true` and `release_ready:true`
with release owner, approver, approval timestamp, matching tag, retained
license file, release notes, and at least one consumable distribution artifact.
The decision file must be retained under the release artifact directory; the
emitted `decision_artifact` records SHA-256 and byte size so strict evidence can
detect later mutation.
The codebase owns the fail-closed artifact contract; legal, product leadership,
release management, and distribution owners supply the approved decision
evidence.
Use `docs/release/release-identity-evidence-kit.md` and
`docs/release/templates/release-identity-decision.template.json` to prepare the
release-authority bundle. The template is intentionally fail-safe: it uses
`ok:false`, `release_ready:false`, placeholder approval metadata, and
placeholder tag/license/notes/distribution fields so it cannot pass until the
release lane has approved, retained evidence.
Current release-check evidence must include `evidence_mode.security_assurance`;
when that field is missing in a non-regulated local bundle, the final evidence
check keeps schema validation green but records a stale release-check blocker so
the artifact is regenerated before promotion.

When `APPFW_RELEASE_REQUIRE_SECURITY_ASSURANCE_DECISION=true`,
`APPFW_REQUIRE_SECURITY_ASSURANCE_DECISION=true`,
`APPFW_RELEASE_REQUIRE_PRODUCTION_ATTESTATIONS=true`, or
`APPFW_REQUIRE_PRODUCTION_ATTESTATIONS=true` is set, the report also treats
`target/appfw/security-assurance-decision.json` as required release evidence.
Missing, not-release-ready, or non-production-attestation decisions make
`release_ready:false` and add LIVE-007 remediation.
The final evidence check revalidates category dispositions: evidence categories
must reference retained, passing artifacts whose checksums and byte sizes still
match the referenced files, and risk-accepted categories must be explicitly
accepted with structured owner, approver, release scope, unexpired bounded
expiry, rationale, compensating controls, and follow-up fields. Retained
security-assurance decisions must also carry a fresh timezone-aware
`generated_at_utc`; stale or future decisions fail the final evidence check.
This check runs before provider URL preflight so local or partially configured
production-release probes still retain a named security-assurance result.

When provider certification runs, the provider parity and security
certification reports are also copied or written to:

```text
target/appfw/provider-parity.json
target/appfw/security-certification.json
```

Provider parity must remain tied to the same evidence bundle as
security-certification: the final release evidence check validates the parity
timestamp, provider logs, full provider set, and preflight URL match before
accepting it as release evidence. It also revalidates
`security-certification.json` retained artifact paths, byte sizes, and SHA-256
hashes so copied or overwritten provider evidence cannot remain trusted.

Bitbucket Pipelines and ArgoCD promotion guidance lives in
`docs/release/release-gate-ci-cd.md`.

### `scripts/appfw pds-baseline`

Writes PDS Security Baseline r4.5 traceability evidence:

```bash
scripts/appfw pds-baseline
scripts/appfw pds-baseline --json
```

The command verifies that the baseline traceability document and live
environment work items cover the PDS production-readiness categories. It writes:

```text
target/appfw/pds-security-baseline.json
```

`ok:true` means repository traceability is intact. It does not imply PDS
production baseline readiness. Use `release_ready:true` for production
promotion; when it is false the report includes `release_blockers`,
`failure_summary.root_causes`, and `remediation_work_items`. Final release
evidence rejects missing, malformed, stale, or future `generated_at_utc`
timestamps on the retained PDS baseline report.

Set `APPFW_RELEASE_REQUIRE_PDS_BASELINE_EVIDENCE=true` in production release
lanes. The command then requires `APPFW_PDS_BASELINE_DECISION_FILE` to point at
a JSON decision artifact that declares `ok:true` and `release_ready:true`; if
the variable is unset, it defaults to
`target/appfw/pds-baseline-decision.json`.

For local pre-CI structural confidence, run:

```bash
scripts/ci/pds-baseline-ready-fixture.sh
```

The fixture command writes synthetic retained decision/evidence files under
`target/appfw/pds-ready-fixture/`, then invokes `scripts/appfw framework
pds-baseline --json` with live evidence required. It must produce
`ok:true`, `release_ready:true`, `live_evidence_required:true`, zero failures,
all LIVE-015 through LIVE-021 items covered, and retained SHA-256/byte metadata
for the decision and evidence files. This proves the branch can satisfy the PDS
gate structurally before CI; it does not prove live PDS approval.

The `LIVE-*` labels are App Framework release-gate checkpoint IDs, not section
numbers from the PDS Health IT Security Baseline Standard. They let automation
track live-environment proof buckets consistently. Retained decision artifacts
should also include `baseline_sections` so PDS reviewers can audit by the
standard's section names while the gate checks the stable `LIVE-*` IDs.

The decision artifact must include `baseline.version` or `baseline_version`
equal to `r4.5`, release-authority owner, approver, and an ISO-8601 approval
timestamp with timezone that is not in the future, plus `work_items` or
`categories` entries that cover LIVE-015 through LIVE-021. Each entry must use
`status:"evidence"` with existing `evidence_files`, or
`status:"risk-accepted"` with owner, approver, scope, unexpired `expires_on`,
rationale, compensating controls, and follow-up. Risk-accepted entries must also
expire within the retained `risk_acceptance.max_days` policy, which defaults to
365 days and can be tightened, but not raised above 365, with
`APPFW_PDS_BASELINE_RISK_ACCEPTANCE_MAX_DAYS`.
For a production-ready bundle, the decision file and every evidence file must
resolve inside the release artifact directory, typically `target/appfw`.
Relative evidence paths are resolved against the release artifact directory
first, then the decision-file directory and repository root; paths outside the
artifact directory are rejected as non-retained evidence. Retained evidence
files must also be non-empty. Additional retained evidence files can be listed in
`APPFW_PDS_BASELINE_EVIDENCE_FILES` as a comma-separated or path-separated list.
The emitted PDS baseline report records `decision_artifact` and
`evidence_artifacts` entries with retained paths, SHA-256 digests, and byte
sizes for the decision file and evidence-backed work-item files.
Use `docs/release/pds-baseline-evidence-kit.md` and
`docs/release/templates/pds-baseline-decision.template.json` to prepare the
release-authority bundle. The template is intentionally fail-safe: it uses
`ok:false`, `release_ready:false`, placeholder approval metadata, and
`status:"todo"` entries so it cannot pass the live-required gate until release
authority replaces each item with retained evidence or a valid risk acceptance.
`release-check` runs this command automatically before provider URL preflight so
blocked local runs still report the PDS baseline remediation posture. The final
`release-evidence-check` gate revalidates those retained decision mappings and
checks that the emitted `decision_summary`, retained check results, artifact
hashes, and byte sizes still match the decision and evidence files. Any retained
PDS baseline report that claims `ok:true` cannot carry embedded failing checks.
Any retained PDS baseline report that claims `release_ready:true` is treated as
a production-readiness claim even when the current lane did not require live PDS
evidence; final evidence then requires `live_evidence_required:true`, a
live-required `release-check` evidence mode, the retained decision artifact, and
no PDS release blockers or failure-summary root causes. In strict mode, a
retained PDS-only `release_ready:false` blocker is a nonzero release gate result
even when the JSON schema is valid.

### `scripts/appfw ops-certification`

Writes observability and operations certification evidence:

```bash
scripts/appfw ops-certification
scripts/appfw ops-certification --json
```

The command verifies the committed observability bundle: Prometheus alert rules,
Alertmanager routing, Grafana dashboard metrics, Alloy redaction/metadata
configuration, and the runbook/deployment references. It writes:

```text
target/appfw/ops-certification.json
```

`ok:true` means the static observability bundle and any required local checks
passed. It does not imply production operations readiness. Use
`release_ready:true` for release promotion; when it is false the report includes
`release_blockers`, `failure_summary.root_causes`, and
`remediation_work_items`.

Set `APPFW_RELEASE_REQUIRE_PROMTOOL=true` to require `promtool check rules`.
Set `APPFW_RELEASE_REQUIRE_LIVE_OPS_EVIDENCE=true` to require retained live
evidence artifacts such as OTLP export proof, `/health/ready`, `/metrics`,
Prometheus target/rule API output, Alertmanager status, Grafana provisioning,
and a runbook drill. This also makes the final release evidence check require a
release-ready `ops-certification.json` that was generated with
`live_evidence_required:true` and a fresh timezone-aware `generated_at_utc`.
For local pre-CI structural confidence, run:

```bash
scripts/ci/ops-certification-ready-fixture.sh
```

The fixture command writes synthetic retained live-ops evidence files under
`target/appfw/ops-ready-fixture/`, then invokes `scripts/appfw framework
ops-certification --json` with live evidence required. It must produce
`ok:true`, `release_ready:true`, `live_evidence_required:true`, zero failures,
and retained SHA-256/byte metadata for OTLP, readiness, metrics, Prometheus,
Alertmanager, Grafana, and runbook-drill evidence. This proves the branch can
satisfy the ops gate structurally before CI; it does not prove live operations
approval.
Stale or future retained ops-certification reports fail the final evidence
check. If an ops report claims `ok:true` or `release_ready:true`, final evidence
also revalidates each retained check result and every required artifact's
presence, SHA-256 digest, and byte size, so edited observability or deployment
docs cannot be promoted with stale ops evidence. `release-check` runs this
command automatically.

### `scripts/appfw mcp-posture`

Checks the release posture for the optional MCP endpoint:

```bash
scripts/appfw mcp-posture
scripts/appfw mcp-posture --json
```

The current release posture excludes MCP unless it has a dedicated certified
release lane. This command writes:

```text
target/appfw/release-mcp-posture.json
```

It fails when `APP_MCP_ENABLED=true`. The retained JSON includes
`generated_at_utc` so release wrappers can reject stale or postdated MCP posture
evidence. `release-check` runs the same posture check internally, but the
standalone command is useful for quick CI and agent preflight checks before
running the full provider-backed gate.

### `scripts/appfw load-test`

Runs a lightweight load harness against a generated GraphQL API.

```bash
scripts/appfw load-test
scripts/appfw load-test --json
scripts/appfw load-test --url http://127.0.0.1:8080/crm --requests 500 --concurrency 16 --json
scripts/appfw load-test --json --scenario crm-accounts-grid --max-p95-ms 300 --max-error-rate 0
```

Defaults target the sample CRM API at `http://127.0.0.1:8080/crm` and run 100
requests with concurrency 8. Configure the harness with:

```text
APPFW_LOAD_TEST_URL
APPFW_LOAD_TEST_REQUESTS
APPFW_LOAD_TEST_CONCURRENCY
APPFW_LOAD_TEST_BODY
APPFW_LOAD_TEST_BODY_FILE
APPFW_LOAD_TEST_TOKEN
APPFW_LOAD_TEST_TIMEZONE
APPFW_LOAD_TEST_SCENARIO
APPFW_LOAD_TEST_MAX_P95_MS
APPFW_LOAD_TEST_MAX_MAX_MS
APPFW_LOAD_TEST_MAX_ERROR_RATE
APPFW_LOAD_TEST_OUTPUT
```

Use `--body-file` for downstream apps that do not expose the sample CRM
`queryAccounts` API. JSON output includes success/failure counts and
min/average/p95/max latency in milliseconds, plus scenario name, error rate,
active thresholds, and threshold violations. Through `scripts/appfw`, JSON
evidence is retained at:

```text
target/appfw/load-test.json
```

Set `APPFW_RELEASE_REQUIRE_PERFORMANCE_EVIDENCE=true` to make
`scripts/appfw release-check --json` run and gate on load-test-suite plus
provider-performance evidence. The final release evidence check requires the
suite summary, not a single-scenario `load-test` artifact, when performance
evidence is release-required.

See `docs/runtime/performance-and-scalability.md` for the scalability workflow.

### `scripts/appfw load-test-suite`

Runs the generated API load suite. The default CRM suite covers dashboard
summary, accounts grid, server-side search, account form projection, lookup
selector, and account/activity relationship scenarios.

```bash
scripts/appfw load-test-suite --json
scripts/appfw load-test-suite --json --requests 250 --concurrency 12 --max-p95-ms 300 --max-error-rate 0
scripts/appfw load-test-suite --json --only crm-accounts-grid
```

Per-scenario artifacts are retained under:

```text
target/appfw/load-tests/
```

When invoked through `scripts/appfw load-test-suite --json`, the suite summary
is also retained at `target/appfw/load-test.json` for release-evidence
compatibility. Each scenario entry records the retained artifact path, SHA-256
digest, and byte size so the release wrapper can reject edited or stale
per-scenario evidence. Configure it with the same URL, token, timezone, and
threshold environment variables as `load-test`, plus:

```text
APPFW_LOAD_TEST_SUITE_OUTPUT_DIR
APPFW_LOAD_TEST_SUITE_SUMMARY
APPFW_LOAD_TEST_SUITE_ONLY
```

### `scripts/appfw provider-performance`

Writes provider performance certification evidence by combining retained
provider parity, generated performance recommendations, and QueryIR budget-cap
contracts.

```bash
scripts/appfw provider-performance --json --all
APPFW_PROVIDER_PERF_REQUIRE_LIVE=true scripts/appfw provider-performance --json --all
```

Evidence is retained at:

```text
target/appfw/provider-performance.json
target/appfw/performance_recommendations.json
```

Use `APPFW_PROVIDER_PERF_REQUIRE_LIVE=true` in release lanes so projection,
pagination, many-to-many fanout, aggregate/count, and access-filter performance
contracts must be backed by retained live provider parity.
When `APPFW_RELEASE_REQUIRE_PERFORMANCE_EVIDENCE=true`, final release evidence
also requires live-certified provider-performance entries for PostgreSQL,
MongoDB, MS SQL Server, and Snowflake. The provider-performance report records
SHA-256 digests and byte sizes for its retained provider-parity and generated
performance-recommendation inputs, plus the QueryIR budget-cap source file, so
the release wrapper can reject edited or hollow performance evidence.

### `scripts/appfw migrate`

Runs the framework-owned database package executor and versioned migration workflow against the product `database/_pkg` package.

```bash
scripts/appfw migrate
scripts/appfw migrate --json
scripts/appfw migrate doctor
scripts/appfw migrate new --schema crm --phase backfill --name normalize_contacts
scripts/appfw migrate plan --json
scripts/appfw migrate lint --phase all --json
scripts/appfw migrate rollback-guide --json
scripts/appfw migrate drift --json
scripts/appfw migrate status --json
scripts/appfw migrate apply --json
```

With no subcommand, this command keeps the legacy bootstrap/reconcile behavior.
With subcommands, it delegates to the forward-only migration runner under
`database/_pkg/migrations`.

Set `APPFW_MIGRATE_SKIP_SEED=true` only for schema-readiness bootstrap lanes
that do not claim provider certification or generated load-test release
evidence. The release provider-certification contracts expect generated sample
seed data for lookup rows and relationship projections.

`migrate new` scaffolds a SQL file and manifest entry. Use `--data-source` to
target one data source or `--dialect all` to scaffold one relational migration
per supported dialect.

`plan`, `drift`, `status`, and `apply` support `--json` for CI/CD gates.
`migrate drift` introspects supported live providers and compares their schema
columns against generated expand migrations before any apply step runs. Reports
include data-source entries, migration states, lint summaries, drift summaries,
and apply errors when available.

`migrate rollback-guide --json` prints the forward-only rollback contract:
application rollback first, database changes forward, backfills by checkpoint,
and contract cleanup only after the safety window.

### `scripts/appfw serve`

Runs the current product generated backend. From the framework checkout, this
serves `examples/products/crm/backend` unless `APPFW_APP_ROOT` points to a
different product app.

```bash
ENV_NAME=local API_PORT=8080 scripts/appfw serve
ENV_NAME=local API_PORT=8080 APP_MCP_ENABLED=true scripts/appfw serve
```

`serve` is intentionally not JSON-oriented because it is an interactive,
long-running process.

For local runs, the wrapper supplies safe defaults for `VERSION` and
`RUST_MIN_STACK` unless the caller sets them. Keep explicit deployed/release
values in the environment that launches the packaged backend.

Set `APP_MCP_ENABLED=true` to mount the optional model-driven MCP endpoint at
`POST /mcp`. The endpoint requires authenticated users plus the configured MCP
role/scope gate, and it reflects generated public handler operations as MCP
tools. See `docs/runtime/mcp.md` for tools, resources, auth, limits, and
origin controls.

## Recommended Agent Loop

For config-only changes:

```bash
scripts/appfw validate --json
scripts/appfw test --fast
```

For generator or template changes:

```bash
scripts/appfw validate --json
scripts/appfw generate
scripts/appfw generate --check --json
scripts/appfw test --fast
```

For runtime changes:

```bash
scripts/appfw validate --json
scripts/appfw test --fast
```

Agents should inspect `.appfw/target/appfw/artifacts.json` before editing files
that may be generated when the manifest already exists. If it does not exist,
use `docs/start/generated-ownership.md` before running generation.

For agent handoff, finish with:

```bash
scripts/appfw handoff --json
```

The handoff report is deliberately compact enough for another agent to consume
without rereading the whole repository.
