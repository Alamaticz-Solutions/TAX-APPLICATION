# Agent Operating Guide

This repository is designed for coding agents and human developers working
together. Follow these rules before changing code.

## Start Here

From the repository root, prefer the App Framework CLI:

```bash
scripts/appfw doctor
scripts/appfw context --json
scripts/appfw product validate --json
```

Use `docs/start/cli-quickstart.md` for the first command path,
`docs/reference/cli.md` as the command contract, and
`docs/start/agent-task-map.md` as the first routing guide. Use
`docs/start/bitbucket-rest-auth.md` before using Bitbucket REST APIs, inspecting
remote pipelines, or diagnosing Bitbucket `401` responses; prove access with
`scripts/ci/bitbucket-api-smoke.sh --repo --pipelines` and then reuse the
verified auth scheme instead of cycling through token/header guesses. When a
PR, `main`, or `v*` Bitbucket gate fails, or ProGet publish is in scope: use
`docs/release/README.md` for the lane map, then reproduce PR failures with
`bash scripts/ci/local-pre-push-gates.sh` (shared Fast framework check lives in
`scripts/ci/pr-fast-framework-check.sh`, which Bitbucket also runs). Admission,
merge-to-`main`, and release/SRA/CAB authority are not defined in release docs —
use `docs/start/agent-role-cards.md` and
`docs/start/agentic-human-operating-model.md`; for Nexus WIP/admission the
Program Flow Controller in
`docs/specs/nexus-poc-multi-workstation-delivery.md` governs flow only and
still cannot approve product, architecture, source, merge, release, or risk.
Use `docs/start/agentic-human-operating-model.md` for the durable human/agent role
boundary, decision rights, review routing, and collaboration flow. For Nexus
PoC multi-workstation delivery, the human-ratified
`docs/specs/nexus-poc-multi-workstation-delivery.md` takes precedence over
legacy XO terminology: its Program Flow Controller is the WIP/admission
governor and has no product, architecture, source, merge, release, or risk
authority. Use
`docs/start/agent-role-cards.md` when spawning a specific role so the thread has
explicit responsibility, collaboration, non-authority, WIP/parked-decision
discipline, and push rules. Each durable thread, bounded subagent, or review
workflow should name its role card at task start and include a Role Card Check
in status, handoff, readiness, or review output: card used, work within role,
authority not assumed, routed decisions, and drift signal. Optionally use
`scripts/agent-routing/appfw-model-route.mjs`
(`docs/specs/afs-008-transparent-routing-and-telemetry.md`) alongside role-card
assignment: construct the closed `--task` request and record the resulting
provider-neutral implementer/reviewer profile as evidence. This is a
prototype-stage capability under proof (AFS-PI-P1, not yet accepted) — it
cannot admit work, spawn an agent, or change WIP, and its recommendation is
advisory only; it never overrides role-card authority, WIP limits, or
human/XO decisions. Use
`docs/start/spec-driven-change-harness.md`
before meaningful framework or product changes where business value, contracts,
acceptance evidence, security/governance, generated output, CI/release
behavior, or multi-agent coordination could be misunderstood; use an intent
note for tiny low-risk fixes and `docs/specs/spec-template.md` for durable
specs. Use `docs/start/branch-integration-model.md` when
assigning Integration Branch Manager work, deciding whether an integration
branch is warranted, freezing shared seams before fan-out, or monitoring PR/CI
evidence. Use `docs/start/product-increment-delivery-model.md` for all new
planned work: register every major batch in
`docs/specs/product-increment-portfolio.json` before its first source branch,
keep its plain-language current status, next action, `requires`, and
`benefits_from` current, and give every Deliverable one primary Product
Increment. Every source-active Product Increment must have one validated
machine-readable plan with one to four independently admissible Delivery
Lanes, explicit capability links, and separate flow, Implemented, and Accepted
state. The portfolio must maintain two to four active lanes overall. Use one
short-lived integration branch for every multi-lane or cross-domain increment;
a truly independent one-lane, one-domain increment may use a declared
`direct_main` strategy. A portfolio claiming `current` must remain within the
enforced reconciliation and status-freshness windows. Validate these records
with
`node scripts/check-product-increment-portfolio.mjs --portfolio
docs/specs/product-increment-portfolio.json --json` and
`node scripts/check-product-increment-plan.mjs --plan <plan.json> --json`
before admitting a lane. Before reviewing a real lane candidate, also run
`node scripts/check-product-increment-plan.mjs --plan <plan.json>
--current-diff <lane-id> --json`; this binds the actual Git diff to the lane's
declared roots and automatic sensitivity profile. Do not create an orphan
branch or standing workstream queue outside this model. For source-producing
work on several computers, also use
`docs/specs/nexus-poc-multi-workstation-delivery.md`: require one assignment
and one active worktree per producer, derive write authority from the selected
Product Increment Delivery Lane, use Workstream topology only for capability
context and restrictions, and do not exceed the Program Flow Controller WIP
limit merely because another computer is available. Once a PR or pipeline is
assigned, use
`docs/start/remote-promotion-state-synchronization.md`: Integration owns live
remote observation through post-merge destination evidence, emits material
transitions, and the Program Flow Controller refreshes board, queue, holds, and
dashboard within one coordination cycle. Stale remote observations must
display as `STALE` or
`UNKNOWN`, never as an old current state. Use the Workstream Analyst role
described in
`docs/start/agentic-human-operating-model.md` and
`docs/start/agent-role-cards.md` when local checks, CI pipelines, polling,
parallelization, or token/output volume become velocity or risk concerns. Use
the Product Increment model's flow-efficiency rules whenever scaling
parallelism: no idle standing producer or reviewer, no duplicate retrieval or
analysis, one delegation level by default, bounded four-record context
capsules, lowest-capable model routing, focused leaf proof, and measured or
explicitly unavailable delivery economics. Every lane and Integration step
must declare a model/time/token/cost/retry/correction/rework budget; escalation
above the default model tier requires a retained justification. Shared
convergence files belong to non-overlapping Integration write roots, not a
leaf lane. Only accepted-main candidate and evidence bytes plus trusted
external verification of review, acceptance, and usage receipts can award
durable Implemented or Accepted credit. A local remote-tracking ref or
self-authored receipt is never authority. Two
correction cycles without a
newly passing semantic proof require contract/scope review before another
correction. In `accelerated` mode, dirty-checkout gates are ephemeral feedback
and do not write immutable candidate evidence; clean exact-SHA proof begins at
the candidate boundary. Agent count and token volume are never progress
measures. Use
`docs/start/tech-debt-register.md` when a PR review returns
`GO WITH CONDITIONS`, checks are skipped, repeated CI/review failures appear, or
the Product Owner, Strategist/Product Manager function, Structure Steward, or
Tech Debt Steward needs explicit debt entries with owners and retirement
criteria. Use
`docs/start/framework-research-steward-harness.md` or
`agent_skills/framework-research-steward/SKILL.md` when running
`/framework-research-refresh` or other external industry, market,
product-management, platform, UX, AI, security, or delivery research to
challenge Strategist/Product Manager and Product Owner guidance before changing
North Star, strategy, roadmap, backlog, or guardrails. Use
`docs/reference/product-workspace-contract.md` for the downstream product/framework
contract, `docs/reference/product-workspace-boundaries.md` for the current-layout
ownership map, `docs/reference/app-manifest.md` for app topology, and
`docs/start/generated-ownership.md` for detailed generated-boundary rules. Use
`docs/lifecycle/product-golden-path.md` for the end-to-end downstream product
developer flow. Use `docs/lifecycle/application-lifecycle.md` when the task involves
creating downstream apps or cascading upstream framework updates into an app
repo. Use `docs/architecture/concerns/maintainability.md` when the task changes docs IA, command
contracts, architecture intent, release evidence paths, frontend scaffolding,
or agent-facing automation. Use `docs/start/team-harness-replication.md` when
the task affects team onboarding, slash-command portability, local-only
dependencies, package distribution, or whether the agent/developer harness can
be recreated cleanly on another machine. Use
`agent_skills/framework-structure-steward/SKILL.md` when reviewing docs IA,
skills, CLI contracts, code organization, generated-boundary clarity, or
delivery-harness entropy and the output should be Product Owner/Strategist
maintenance recommendations rather than immediate broad refactoring. Use
`docs/start/pds-sra-package-harness.md` when preparing PDS Health Security Risk
Assessment evidence; framework scope uses
`scripts/appfw framework sra-package --all-products --json`, and product scope
uses `scripts/appfw product sra-package --json`. Use `agent_skills/README.md`
as the concise procedure router for product bootstrap, PoC intake, legacy
modernization, schema modeling, generation, local testing, frontend product
work, release evidence, SRA package evidence, and review workflows.

## Generated Boundaries

Treat `.appfw/model` as application source code and `app_gen/_templates` as
generator source code. From the framework checkout, repository-root
`scripts/appfw` defaults product workflows to `examples/products/crm`; many
files under a product app's `backend`, `database/_pkg`, and
`api_tests/src/schemas` are generated.
Treat `.appfw/manifest.yaml` as product-owned app topology. It may name schemas
and data sources, but it must not replace entity, relationship, seed, test, or
provider-environment config.
Treat `podman-compose.yml` as generated local dev infra. Change topology or
`.appfw/model/data_sources/_res.yaml`, then regenerate, instead of
hand-removing provider services.

`app_gen` is root-aware. Prefer `scripts/appfw` so app root, framework root,
config root, template root, and report root are passed consistently. If you run
the generator directly, use the explicit root flags documented in `docs/reference/cli.md`.

Before editing a generated-looking file, use the boundary and ownership guides:

```text
docs/reference/product-workspace-boundaries.md
docs/reference/product-workspace-contract.md
docs/reference/app-manifest.md
docs/start/generated-ownership.md
```

If `.appfw/target/appfw/artifacts.json` already exists, you may inspect it with
`scripts/appfw product manifest --json`. Do not run `scripts/appfw product generate` only to
discover ownership; it writes generated artifacts.

Prefer changing the source of generation over patching generated output.

## Safe Change Loop

Before committing or pushing framework work that must pass Bitbucket PR CI, run
the shared PR-lane entrypoint (same Fast framework check CI uses, plus
release-lite / supply-chain / secret-scan):

```bash
bash scripts/ci/local-pre-push-gates.sh
```

Do not re-list those gate commands here or in release docs — `scripts/ci/pr-fast-framework-check.sh`
is the Fast framework source of truth (Bitbucket calls it with `--ci-bootstrap`),
and `local-pre-push-gates.sh` wraps the full PR lane. Then handoff + review:

```bash
scripts/appfw framework handoff --json
scripts/appfw framework review-brief --auto-depth --json
```

For config changes:

```bash
scripts/appfw product validate --json
scripts/appfw product test
scripts/appfw product handoff --json
```

For generator or template changes:

```bash
scripts/appfw framework validate --json
scripts/appfw framework generate
scripts/appfw framework generate --check --json
scripts/appfw framework test
scripts/appfw framework handoff --json
```

If `generate --check` reports generated drift, preserve the diagnostic and
report it instead of masking it.

For provider changes:

```bash
scripts/appfw framework validate --json
scripts/appfw framework test
scripts/appfw framework handoff --json
```

For backend-only runtime changes:

```bash
scripts/appfw framework validate --json
scripts/appfw framework test
scripts/appfw framework handoff --json
```

For docs, CLI contract, architecture intent, or agent-facing automation
changes:

```bash
scripts/appfw framework validate --json
scripts/appfw framework docs-check --json
scripts/appfw framework generate --check --json
scripts/appfw framework test --fast --json
scripts/appfw framework handoff --json
```

For spec-driven changes, name the spec or intent note in handoff. Use the
lightest depth that preserves decision provenance: no durable spec for tiny
fixes, lightweight spec for focused features/workflows, and full spec for
framework contracts, generated output, security/privacy, SaaS/provider
integrations, CI/release gates, SRA/CAB, broad UX/product behavior, or
multi-agent coordination.

Run generated API tests only when the backend and required data sources are
running:

```bash
scripts/appfw product api-test
```

`api-test` runs only generated product scenarios under `api_tests/src/schemas`.
Use `scripts/appfw framework provider-test --provider <provider>` for framework provider
certification; do not mix provider contracts into generated scenario output.

## Pre-Push Review Gate

Invoking the repository's configured independent Framework or Product PR
Review Agent is standing-authorized for repository-visible source, diffs,
configuration, documentation, and sanitized proof artifacts. Do not ask the
human for case-by-case permission to run the required review. Stop only for a
real data-egress exception: secrets, credentials, tenant data, PHI/PII,
regulatory source material, or an unapproved reviewer/provider. Review
invocation does not authorize push, PR creation, merge, release, publication,
SRA/CAB approval, or accepted risk. See the standing authorization in
`docs/start/agent-role-cards.md`.

Before pushing any agent-authored branch, the coding/architecture agent must
run the appropriate PR Review Agent workflow:

1. Finish the risk-appropriate proof loop and run
   `scripts/appfw framework handoff --json` or
   `scripts/appfw product handoff --json`.
2. Retain the review contract with
   `scripts/appfw framework review-brief --auto-depth --json` or
   `scripts/appfw product review-brief --auto-depth --json` when supported.
   Bare `review-brief --json` defaults to focused mode; `--auto-depth` keeps
   focused review for narrow changes and requires `--comprehensive` only for
   integration, broad, governance, CLI/CI, review-harness, release/security,
   generator, runtime-contract, or other sensitive surfaces.
3. For framework-owned changes, invoke `/framework-pr-review` or
   `/framework-pr-review --comprehensive` as indicated by `review-brief`, or the
   `framework-pr-review` skill as the **Framework PR Review Agent** using
   `docs/start/pr-review-agent-harness.md`. For product-owned changes, invoke
   `/product-pr-review` or `/product-pr-review --comprehensive` as indicated, or
   the `product-pr-review` skill using
   `docs/start/product-pr-review-agent-harness.md`.
4. Present the structured review output to the human:
   Recommendation Summary, Attention Items when final status is not `GO`,
   Findings, Strategic Significance, Role Adherence Assessment,
   Alignment Drift Assessment, Independent Code Quality And Architecture Assessment, Shared
   Reviewer Judgment, Human Approval Brief, Evidence Checked, and Open
   Questions. The Recommendation Summary must state final status (`GO`,
   `GO WITH CONDITIONS`, `NO-GO`, or `DEFER`) plus severity counts for blockers,
   critical, important, should_address, and nice_to_address, plus
   `Conditions captured: yes` for `GO WITH CONDITIONS` or
   `Conditions captured: not_applicable` otherwise. Non-`GO` statuses must
   include Attention Items with concise bullets naming the conditions,
   blockers, evidence gaps, or review limits the human must inspect. Always
   provide the retained review output link, normally
   `target/appfw/framework-pr-review.md` for framework work or the product
   review artifact for product work, so the human can open the complete
   reviewer judgment even when standing push approval applies.
5. Push-capable implementation or integration agents may use the standing
   approval in `docs/start/agent-role-cards.md`: push only when review status is
   `GO` or `GO WITH CONDITIONS`, blocker and critical counts are zero, any
   conditions are captured as debt/follow-up/accepted-risk request or explicit
   human decision, and branch state has not changed since review.
6. After a successful push of merge-bound work, make the PR path explicit:
   either create/update the pull request when the Program Flow Controller
   assigned that responsibility,
   or immediately hand off the branch, review output path, proof summary, and
   pushed SHA to the Integration Branch Manager. In this repository, non-`main`
   branch pushes do not by themselves run the full pull-request gate; the PR
   object is what triggers the PR pipeline evidence.

For local enforcement, install the repo-owned Git pre-push hook with
`scripts/ci/install-local-git-hooks.sh`. The hook runs only when `git push` is
about to send branch updates; it checks
`scripts/ci/pre-push-review-guard.sh` and blocks missing or stale Framework PR
Review Agent evidence before the branch leaves the workstation. The retained
review output must be newer than the current handoff artifact. The hook uses
auto-depth selection, so focused review is enough unless the branch shape or
touched surfaces require comprehensive review. The guard always reports the
review output artifact path; agents must carry that path forward in their
handoff or final message instead of only reporting pass/fail.

The standing approval authorizes pushing assigned feature/fix/docs/wave or
integration branches; it does not authorize merge to `main`, release, SRA/CAB
approval, accepted risk, or direct push to `main`. If the review agent returns
`NO-GO` or `DEFER`, or if branch state changed after review, fix/rerun review or
obtain explicit human override before push.

## Branch Naming

Branch prefixes describe the work stream, not the tool or author. Use purpose or
lane prefixes such as `feature/<topic>`, `fix/<issue>`, `docs/<topic>`,
`integrate/<wave-or-family>`, or `wave3/<lane>`. Do not use assistant/tool prefixes
such as `codex/`, `claude/`, or `agent/`; different developers may use different
agent harnesses, and review/CI should reason about the work rather than the
assistant that produced it. See `docs/start/branch-integration-model.md` for
integration-branch examples.

## PR Review Performance Oversight

When the human invokes `/check-pr-review-performance` or says
`check PR review performance`, review the review agent's output and artifacts
using the PR Review Performance Oversight procedure in
`docs/start/pr-review-agent-harness.md`. Report whether the review was healthy,
needs tuning, or unreliable; identify missed or overstated risks; and recommend
changes to prompts, harnesses, skills, docs-check, or automation when the review
system is not meeting the shared human-review standard.

## Config Contract

The canonical config contract is emitted by validation:

```text
.appfw/target/appfw/config_contract.json
.appfw/target/appfw/config_contract.md
.appfw/model/_specs/CONFIG_CONTRACT.md
```

Do not manually maintain `.appfw/model/_specs/CONFIG_CONTRACT.md` as independent
documentation. Change `app_gen/src/config_contract.rs`, then validate.

## Enterprise Safety Rules

- Do not commit secrets, access tokens, tenant data, or local `.env` files.
- Keep product PR evidence traceable through validation, generation, tests,
  handoff, release, and deployment artifacts when the change is production
  relevant.
- Do not add `primary_schema` or `primary_data_source` to the app manifest;
  model all schemas and data sources explicitly.
- Do not overwrite human-owned handler implementation files.
- Treat `backend/src/handlers/<schema>/generated.rs` as generated handler
  defaults; product overrides belong in `backend/src/handlers/<schema>/<entity>.rs`.
- Keep provider behavior consistent across PostgreSQL, MongoDB, MS SQL Server,
  and Snowflake unless a provider limitation is documented.
- Preserve access-control semantics when changing filters, query compilation,
  data access, or route construction.
- Prefer focused tests near the changed behavior, then broader `scripts/appfw`
  checks before handoff.
- Use `scripts/appfw framework docs-check --json` when changing docs or agent-facing CLI
  examples.
- Finish substantial work with `scripts/appfw product handoff --json` or
  `scripts/appfw framework handoff --json` so changed
  surfaces, verification artifacts, and generated drift are machine-readable.

## Useful Files

```text
README.md
docs/README.md
docs/start/agent-task-map.md
docs/start/framework-research-steward-harness.md
docs/start/cli-quickstart.md
docs/reference/cli.md
docs/reference/product-workspace-contract.md
docs/start/generated-ownership.md
docs/lifecycle/product-golden-path.md
docs/lifecycle/application-lifecycle.md
docs/lifecycle/legacy-modernization.md
docs/architecture/concerns/maintainability.md
agent_skills/README.md
app_gen/README.md
app_gen/src/config_contract.rs
app_gen/src/validation.rs
.appfw/model/_specs/CONFIG_CONTRACT.md
backend/src/data/query_ir.rs
backend/src/data/data_access.rs
examples/products/crm/backend/src/data/query_ir.rs
examples/products/crm/backend/src/data/data_access.rs
```
