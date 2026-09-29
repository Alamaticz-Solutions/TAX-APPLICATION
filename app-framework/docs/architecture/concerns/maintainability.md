# Maintainability Command Center

This guide is the maintainability contract for App Framework. Use it when a
change touches architecture, command surfaces, documentation, automation,
tests, evidence, or downstream product scaffolding.

The framework is healthy when new behavior becomes easier to discover, safer
to change, and simpler to prove. It is not healthy when each feature adds a new
private path, undocumented command, generated-boundary exception, or release
artifact that only one agent understands.

## Documentation IA

Docs are organized around two audiences first, then the product delivery
lifecycle and framework stewardship lifecycle. Put new content in the smallest
layer that matches the reader's job.

| Layer | Reader Job | Canonical Docs |
| --- | --- | --- |
| Product Developer | Build, run, test, release, upgrade, modernize, and hand off a product app. | `docs/product/README.md`, `docs/lifecycle/product-golden-path.md`, `docs/lifecycle/intake-and-discovery.md`, `docs/lifecycle/legacy-modernization.md` |
| Framework Steward | Evolve App Framework docs IA, CLI, generator, runtime, providers, release gates, packages, and scaffolds. | `docs/framework/README.md`, `docs/architecture/overview.md`, `docs/architecture/concerns/maintainability.md`, `docs/architecture/adr/` |
| Start Here | Route the task safely in the first two minutes. | `AGENTS.md`, `agent_skills/README.md`, `docs/README.md`, `docs/start/agent-task-map.md`, `docs/start/cli-quickstart.md` |
| Product Delivery Lifecycle | Intake, legacy modernization, bootstrap, model, generate, run local, test, release, upgrade, drift, and handoff. | `docs/lifecycle/product-golden-path.md`, `docs/lifecycle/intake-and-discovery.md`, `docs/lifecycle/legacy-modernization.md`, `docs/lifecycle/application-lifecycle.md` |
| Reference | Answer stable contract questions. | `docs/reference/product-workspace-contract.md`, `docs/start/generated-ownership.md`, `docs/reference/app-manifest.md`, `docs/release/deployment-reference.md`, `docs/runtime/provider-sdk.md`, `docs/frontend/product-frontend.md`, `docs/frontend/pds-health-design-system.md` |
| Internal Architecture | Explain framework-owned design intent. | `docs/architecture/overview.md`, `docs/architecture/framework-packaging.md`, `docs/architecture/concerns/runtime-modularity.md`, `docs/architecture/concerns/packaging.md`, `docs/architecture/adr/` |
| Scorecard | Track current readiness numbers, release blockers, and next priorities. | `docs/release/roadmap.md` |
| Archive | Preserve context that should not guide new implementation. | `docs/archive/` |

ADR 0013, `docs/architecture/adr/0013-docs-information-architecture.md`,
records the durable design decision for this hierarchy. Follow it when adding
new docs, moving historical material, or changing skill/doc relationships.

The CLI seam is one binary with two canonical namespaces:

```bash
scripts/appfw product <command>
scripts/appfw framework <command>
```

Flat commands remain compatibility aliases for older scripts. New docs, skills,
prompts, and CI examples should use the explicit namespace that matches the
audience.

When docs change, keep `docs/README.md` as the index, not a second copy of the
content. Add a single routing sentence and link to the deeper guide.

Before adding a new docs file, prefer one of these refactors:

- add a short section to the existing path doc;
- move historical planning notes under `docs/archive/`;
- add an ADR when the content is a durable decision;
- add a reference-table row when the content is stable contract detail; or
- add a docs-check assertion when the content is a command/example contract.

Create a new top-level docs file only when it introduces a durable reader job
that does not fit an existing path. New top-level docs must be linked from
`docs/README.md`, must name their layer, and should include their verification
loop.

Treat docs like code:

- one doc should own each durable contract;
- other docs should import that contract by link, not copy it;
- large docs should expose a small public interface first, then details;
- stale branch status, PR logs, and implementation diaries should be deleted
  from live docs or moved to archive;
- if a section no longer changes how an agent works today, prune it;
- if two docs answer the same reader question, combine them or make one an
  index row that routes to the owner;
- every command example must either be exercised by docs-check or clearly state
  that it needs live services, release CI, or a human approval step.

When multiple lanes are active, use
`docs/start/branch-integration-model.md` as the operating guide for branch
shape and merge trains. Roadmap docs own priority and dependency order; the
branch integration model owns how lane branches are grouped, refreshed, merged,
and deleted without turning live docs into PR history. It also owns the branch naming convention:
branch prefixes describe the work stream, not the assistant or author.

For broad, sensitive, or multi-domain agentic work, use
`docs/architecture/concerns/agentic-development-control-system.md` before
implementation. That control system classifies the change, names human-review
triggers, and defines the report-only `framework change-impact --json` evidence
gate that should make massive unsupervised diffs mechanically hard to merge.

For review-only work, use `docs/start/pr-review-agent-harness.md` and
`agent_skills/framework-pr-review/SKILL.md`. Keep review guidance separate from
implementation guidance so agents can offload inspection while preserving
explicit human approval for architecture, security, release, and risk decisions.

Before keeping historical detail in a live doc, ask whether it changes the next
action for a human or agent. If the answer is no, rely on git history or archive
it with a status note.

## Agent Skills Pack

`agent_skills/` is the procedural layer for agents. It should stay small and
audience-aware:

```text
agent_skills/README.md
agent_skills/<workflow>/SKILL.md
```

Use it for workflow execution guidance, not for durable contract detail. The
progressive disclosure rule is:

1. the index routes the job;
2. `SKILL.md` gives the immediate steps, proof commands, and guardrails; and
3. canonical docs and generated artifacts provide deep guidance only when the
   task needs it.

When adding or changing a skill:

- declare `audience`, `phase`, `cli_namespace`, and expected artifacts in
  frontmatter;
- keep the skill body short enough to load without crowding code context;
- link to existing docs instead of copying reference content;
- keep product-specific decisions in the product repo;
- update `scripts/appfw framework docs-check --json` to verify the skill remains
  discoverable; and
- avoid new top-level docs unless a new reader job truly exists.

Product intake has two first-class agentic paths:

- `product-poc-intake` starts from citizen-developed artifacts such as
  workbooks, embedded-data HTML apps, and prototype dashboards.
- `product-legacy-modernization` starts from production-shaped systems: codebases,
  live or exported data sources, stored procedures, jobs, integrations, auth,
  reports, and migration constraints.

Keep these paths separate. A PoC conversion infers product intent from example
artifacts. A legacy modernization extracts business capability and operational
constraints from existing behavior, then records static analysis, data
discovery, stored-procedure disposition, migration, and hardening evidence.

## Maintainability Moves

Use these as the default high-value backlog for framework stewardship:

| Move | Outcome |
| --- | --- |
| Docs information architecture | Agents start from a reader job, not a flat file list. |
| Command and evidence contracts | Agent-facing commands have docs, JSON, retained artifacts, and docs-check or release-evidence coverage. |
| Agentic change control | Broad or sensitive agentic changes are classified, reviewed at the right altitude, and backed by retained impact/evidence artifacts. |
| Runtime architecture intent | HTTP/GraphQL, MCP, and Kafka ingress load independently while converging on shared invocation semantics. |
| Golden downstream proof | Product creation, PoC intake, legacy modernization, generation, frontend scaffold, tests, release, upgrade, and handoff become executable. |
| Deployment and release evidence path | Local, release-candidate, and production proof are clearly separated. |
| Historical pruning | Live docs describe current operating paths; branch diaries and obsolete implementation notes leave the hot path. |

## Framework Structure Steward

Use `agent_skills/framework-structure-steward/SKILL.md` as a review-only
maintenance agent when structure, clarity, or entropy is the concern. This is a
distinct role from feature implementation and from PR approval. It protects the
shape of the system and recommends maintenance work back to the Product Owner
and Strategist/Product Manager function.

The Structure Steward reviews six lenses:

| Lens | Looks For | Output |
| --- | --- | --- |
| Docs IA | duplicate contracts, missing index routes, stale live docs, historical detail in hot paths, unclear reader job | doc move/prune/consolidate recommendations |
| Skills | oversized skills, missing router entries, copied reference content, tool-specific assumptions, slash-command workflows without fallback | skill simplification or portability recommendations |
| CLI contracts | namespace drift, missing `--json`, undocumented commands, missing retained artifacts, examples not covered by docs-check | CLI/docs-check/contract backlog |
| Code organization | module sprawl, misplaced framework/product behavior, package-boundary erosion, repeated helper logic, generated-boundary violations | code organization or package-boundary backlog |
| Delivery harness | repeated CI failures, slow opaque checks, late release-lite failures, weak local preflight, unclear review evidence | move-left or CI throughput backlog |
| Strategy alignment | roadmap work that adds surface area without measurable business value, ownership, evidence, or package consumption path | product priority or defer recommendation |

Invoke the steward:

- before a broad integration PR;
- after two repeated review or CI failures in the same category;
- during wave refresh or team handoff;
- when a branch touches docs, skills, CLI, and code organization together; or
- when a human says the system is becoming hard to understand.

The steward should produce a short maintenance brief:

```text
Verdict: healthy | watch | needs-maintenance
Top entropy risks:
Maintenance recommendations:
  - priority:
    owner:
    evidence:
    suggested lane:
Do not mix into current branch:
Product decision needed:
```

Do not let this role become a roaming refactorer. If it finds real cleanup work,
open a focused maintenance lane, add a tech-debt item, or send the
recommendation to the Product Owner for backlog prioritization and the
Strategist/Product Manager function when strategic fit is in question.

### Architect-Structure-Product Triage

The default maintainer flow is not "steward finds, steward fixes." Use this
sequence:

1. The **Architect** invokes the Framework Structure Steward when structural
   drift may affect current waves or future throughput.
2. The **Structure Steward** produces a review-only brief: what is drifting, why
   it matters, what evidence supports it, and which maintenance lane or debt
   item may be needed.
3. The **Architect** reviews the brief technically: confirms the finding,
   evaluates dependency order and blast radius, and recommends whether to fold,
   split, defer, or reject the work.
4. The **Architect parleys with the Product Owner, Strategist/Product Manager,
   and XO when needed** to decide priority using business value, roadmap impact,
   risk reduction, delivery throughput, and Nexus/product/platform relevance.
   The Product Owner owns current backlog priority and acceptance criteria. The
   Strategist/Product Manager challenges strategic fit and portfolio value. The
   XO marshals the multi-thread operating system when sequencing or escalation
   is the decision.
5. The decision is recorded as one of: add to immediate wave lane, create a
   focused maintenance lane, add to tech debt register, defer with rationale, or
   reject as not worth platform investment.

Boundary: **Structure Steward detects and recommends; Architect evaluates
feasibility and sequencing; Product Owner prioritizes current backlog work;
Strategist challenges strategic fit.** In plain language: Structure Steward
detects and recommends; Architect evaluates feasibility and sequencing; Product
Owner prioritizes; Strategist challenges strategic fit when platform direction
is affected.

Compact boundary: Structure Steward detects and recommends; Architect evaluates feasibility and sequencing; Product Owner prioritizes; Strategist challenges strategic fit.

## CLI And Automation Intent

`scripts/appfw` is the stable developer entrypoint. It should stay friendly and
safe, but it should not become the only place where framework logic lives.

New command surfaces should follow this contract:

- The shell wrapper may orchestrate environment setup, artifact paths, and
  compatibility behavior.
- Durable parsing, validation, report shaping, or framework semantics should
  move toward typed Rust CLI modules or focused scripts with stable JSON
  outputs.
- Every production-relevant command must have `--json` or a retained artifact
  path.
- Every new agent-facing command must be documented in
  `docs/reference/cli.md`; short routing examples belong in
  `docs/start/cli-quickstart.md`.
- Every new release-evidence artifact must be validated by
  `scripts/ci/release-evidence-check.sh` when it can block promotion.
- Every safe example that docs ask agents to run should be covered by the full
  `scripts/appfw framework docs-check --json` gate. Prefer
  `scripts/appfw framework docs-check --fast --json` for local static/doc IA
  feedback and `scripts/appfw framework docs-check --changed-only --json` for
  PR preparation.
- CLI wrapper, packaged CLI, profile, and command-routing changes should also
  stay covered by `scripts/appfw framework cli-test --json`, the focused
  developer-fast lane that intentionally avoids docs-check, generation, product
  crates, live services, and provider certification.

The shell wrapper is still the right entrypoint for agents; the long-term
maintenance direction is to make it thinner by delegating reusable behavior to
versioned CLI/runtime packages.

## Pull-Request T0 Feedback Contract

T0 is the first bounded pull-request signal. It exists to shorten correction
latency for deterministic repository failures; it is not an assurance tier,
change-risk router, merge decision, or production-readiness claim. The PR lane
therefore keeps this order:

```text
initial release-lite guard
cache-free PR preflight (T0)
fail-fast parallel (Fast framework check || supply-chain gate || secret scan)
final release-lite guard + destination-freshness check
```

`scripts/ci/pr-preflight.py` distinguishes the triggering source commit from
the tested checkout commit, binds both to the exact destination and an allowed
direct/fast-forward/two-parent relation, and uses the destination-to-tested
effective comparison. It retains report-only change classification plus diff,
coherent conflict block, changed-shell syntax, and Rust formatting evidence.
Changed Git blobs are size-checked before materialization and fail closed above
the documented 32 MiB inspection cap. Its stable failure taxonomy and bounded
logs make cheap failures diagnosable without streaming the broad gate output.
`change-impact.json` must remain telemetry in this leaf: no result from it may
suppress or narrow a downstream check.

The Fast evidence finalizer applies the same permitted source/tested/destination
relations, including Bitbucket's exact two-parent synthetic merge checkout,
and publishes its staged allowlist atomically. The closing guard re-reads the
destination branch head from `origin`; if it advanced while the long producer
group ran, the pipeline fails closed instead of presenting stale green proof.

The clean remote criterion (at most three minutes) and controlled formatting
failure criterion (at most five minutes) are merge evidence for this bounded
leaf, not an accepted p95. Throughput graduation requires at least 20 PR
pipeline observations. Any later proposal for classifier-driven routing,
sharding, caching, duplicate-gate retirement, or assurance reduction needs its
own decision provenance and exact-SHA evidence.

## Runtime Ingress Contract

All external entrypoints should converge on the same runtime operation boundary.
HTTP/GraphQL, MCP, and Kafka ingress are independently loadable runtime
modules; they must not each invent their own authorization, policy, audit,
metrics, or provider path.

The host/module contract is runtime-owned:

```text
RuntimeMode
RuntimeIngressKind
RuntimeIngressDescriptor
RuntimeHostPlan
```

`APPFW_RUNTIME_MODE` and `APPFW_MODULES` choose which compiled ingress modules
load in the current process. This is a maintainability boundary: adding Kafka
must not drag HTTP routing into worker-only artifacts, and enabling MCP must
not implicitly start HTTP or Kafka.

Target contract:

```text
RuntimeIngress
  -> authenticate transport or workload
  -> derive runtime actor, tenant, request, and correlation context
  -> validate payload and generated operation contract
  -> RuntimeOperationDispatcher
  -> generated dispatcher or product extension handler
  -> DataAccess / QueryIR / policy / provider
  -> audit, metrics, tracing, redaction, result envelope
```

Runtime ingress modules own transport-specific concerns:

- HTTP headers, JWT, request IDs, body limits, and GraphQL routing.
- MCP JSON-RPC, tool catalog shape, MCP front-door gates, and tool result
  bounds.
- Kafka runtime worker config, broker authentication config, service-principal
  actor mapping, tenant derivation, operation bindings, idempotency,
  retry/dead-letter validation, readiness policy, and message-to-operation
  dispatch. Concrete broker client loops still require product/platform
  binding and release certification.

`RuntimeOperationDispatcher` owns shared runtime invocation semantics:

- operation identity and generated contract lookup;
- auth, role/scope, tenant, and on-behalf-of context;
- policy and access-filter enforcement;
- validation and query-budget enforcement;
- audit, metrics, tracing, redaction, and normalized result/error envelopes.

The code contract lives in `appfw_runtime::host`, `appfw_runtime::ingress`,
and `appfw_runtime::operation`. Keep HTTP/GraphQL, MCP, admin, and Kafka work
aligned to `RuntimeIngressDescriptor`, `RuntimeHostPlan`,
`RuntimeOperationRequest`, and `RuntimeOperationDispatcher` instead of
inventing transport-local dispatch semantics.

Guardrail: a trusted transport only gets a message into the runtime. It never
bypasses policy, tenant isolation, QueryIR budgets, audit, or provider
certification.

## Golden Downstream CI

The maintainability target is an executable downstream app proof, not only
documentation.

Golden downstream CI should run the executable lifecycle proof when product
bootstrap, scaffold, generation, release-evidence, or upgrade behavior changes:

```bash
scripts/appfw product new --list-profiles --json
scripts/appfw framework intake-proof --json
scripts/appfw framework golden-downstream --json
scripts/appfw framework golden-downstream --profile crm-sample --execute --json
```

For local debugging, expand the executable step into the downstream commands:

```bash
scripts/appfw product new "$TMPDIR/customer-crm" --from current --profile crm-sample --generate --json
cd "$TMPDIR/customer-crm"
scripts/appfw product validate --json
scripts/appfw product generate --check --json
scripts/appfw product test --fast --json
scripts/appfw product handoff --json
```

When frontend scaffolding is included, add:

```bash
scripts/appfw product frontend-test --json
```

When a mobile HTML mockup or React Native app is included, add:

```bash
scripts/appfw product mobile-plan --ui-artifact <mockup.html> --json
```

When live services are available, add:

```bash
scripts/appfw product api-test
scripts/appfw product load-test-suite --json
scripts/appfw framework provider-performance --json --all
```

Until the write-heavy disposable-app lane runs in CI,
`scripts/appfw framework intake-proof --json` must retain
`target/appfw/product-intake-proof.json` and
`target/appfw/product-intake-proof/model-status.json` plus
`target/appfw/product-intake-proof/frontend-residue-check.json` and
`target/appfw/product-intake-proof/frontend-scaffold-check.json`,
`scripts/appfw framework golden-downstream --json` must retain
`target/appfw/golden-downstream.json`, and `scripts/appfw framework docs-check --json`
must retain `target/appfw/lifecycle-evidence-checklist.json`,
`target/appfw/pds-component-check.json`, and
`target/appfw/maintainability-contract.json`, so reviewers can see the intended
commands, artifacts, and remaining CI gaps.
When `golden-downstream --execute` runs in CI, the same
`target/appfw/golden-downstream.json` artifact must contain the per-step
disposable app execution results.

## Frontend Scaffold Execution

The CRM frontend is the reference implementation. It is not yet the fully
automated product-creation path.

Maintainable frontend scaffolding means:

- profiles advertise frontend scaffold evidence in
  `scripts/appfw product new --list-profiles --json`;
- generated UI contracts are replaceable and live under `frontend/src/generated`;
- scaffold-owned modules are separated from product-owned `src/features/**`;
- package scripts expose offline checks and release evidence;
- `scripts/appfw product frontend-test --json` can retain CLI-runnable E2E/a11y proof;
- downstream products can start from the scaffold without forking the
  framework-owned admin UI.

Product UI work should improve the scaffold contract or a product-owned feature,
not blur those two surfaces.

## Mobile Scaffold Execution

React Native mobile work is a sibling product experience target, not a
responsive clone of the web frontend. Maintainable mobile scaffolding means:

- `docs/frontend/mobile-react-native.md` owns the layout/navigation contract;
- `scripts/appfw product mobile-plan --ui-artifact <mockup.html> --json`
  retains source hash and conversion evidence before agents write RN source;
- product-owned mobile code lives under `mobile/` and aligns with
  `mobile/.appfw-mobile/ownership.json`;
- web and mobile share model/API/policy/PDS token semantics while mobile owns
  native navigation, secure storage, push, offline posture, and app lifecycle;
- `generate --target mobile-rn --check --json` owns generated-source drift;
  `mobile-test --json` retains typecheck, Expo, audit, device, and store inputs
  only as non-authoritative Prototype diagnostics;
- a source-bound mobile candidate checker must separately prove real API/auth
  execution, both platforms, provenance, update recovery, Fabric authority,
  and comprehensive review before any mobile candidate claim.

## Maintainability Gates

Use these checks before calling a framework change ready:

```bash
scripts/appfw framework cli-test --json
scripts/appfw framework validate --json
scripts/appfw framework docs-check --changed-only --json
scripts/appfw framework generate --check --json
scripts/appfw framework test --fast --json
scripts/appfw framework handoff --json
```

Add broader checks when the change touches the corresponding surface:

| Surface | Additional Gate |
| --- | --- |
| Generator or templates | `scripts/appfw framework generate`; `scripts/appfw framework test` |
| Runtime/provider semantics | focused Rust tests; `scripts/appfw framework provider-test --provider <provider>` when live |
| Release evidence | `scripts/appfw framework release-check --json`; `scripts/ci/release-evidence-check.sh --strict` in CI |
| Frontend scaffold or reference app | `scripts/appfw product frontend-test --json`; product frontend package checks |
| Performance posture | `scripts/appfw product load-test-suite --json`; `scripts/appfw framework provider-performance --json --all` |
| Deployment/security posture | supply-chain, secret, PHI, SBOM, security-assurance, and ops-certification gates |

`scripts/appfw framework docs-check --changed-only --json` is the normal
low-cost maintainability sentinel. It records the changed-surface selector in
`target/appfw/docs-check-changed-surface.json`, including the base ref, changed
files, trigger rule names, and whether the branch requires full docs-check. It
also records the selected tier, timing budget, `budget_ok`, phase timings for
changed-surface selection and Cargo prebuilds, aggregate example time, and
unattributed wall time in `target/appfw/docs-check-timing.json`. The timing and
subcheck artifacts also record `slowest_subcheck`, a
`focused_rerun_command`, and `parallelization_guidance` so agents can take the
next local slice from retained evidence instead of guessing from terminal logs.
`slowest_subcheck.item_count` and `slowest_subcheck.detail` distinguish a
multi-item bucket that should be split from a single slow command that should be
cached, optimized, or moved toward typed Rust.
When the slowest subcheck is `core-generated-drift-command-examples`, inspect
`target/appfw/generate-check-timing.json`; `generate --check --json` now retains
phase timings for the split-root equivalence check so the next optimization can
target root copy, generation, manifest checks, or generated-output comparison
directly.
Focused subchecks share `target/appfw` logs and retained artifacts, so they are
not same-worktree parallel safe; use sequential focused reruns locally or
separate worktrees/report roots for parallel experiments. Use
`--enforce-budget` only in the CI lane whose threshold has been accepted; the
Bitbucket pull-request lane uses changed-only with separate fast/full selected
mode budgets. Full `docs-check --json` remains the release-grade gate for safe
command examples, lifecycle evidence expectations, security environment
documentation parity, and the maintainability contract above.

## Review Questions

Ask these before merging broad framework work:

- Did this make the golden path easier to run or only add another exception?
- Is the source of truth config, generator, runtime package, provider package,
  product extension, or documentation?
- Can a future agent discover the right edit surface without reading this PR?
- Does a command or artifact introduced here have a JSON contract?
- Does docs-check or release-evidence-check guard the new contract?
- Is live evidence required, optional, or formally risk-accepted?
- Does this move behavior toward shared runtime/provider packages instead of
  product-local copies?
