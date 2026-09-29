# Team Harness Replication

Use this guide when App Framework development moves from one developer machine
to a team. The harness must be reproducible from the repository and approved
package feeds, with no hidden local prompts, copied artifacts, or machine-only
state required for normal development.

## Replication Principle

A new developer or coding agent should be able to clone the repository, install
documented prerequisites, run the first verification commands, and get the same
operating guide, skills, review flows, evidence expectations, and branch rules
that the originating machine used.

The human/agent role boundary is part of the harness contract. Preserve it from
[Agentic And Human Operating Model](agentic-human-operating-model.md) and
[Agent Role Cards](agent-role-cards.md), not from thread memory or private
prompts.

The default coordination topology is also part of the harness contract:

- keep the Program Product Manager, Program Flow Controller, Chief Architect,
  and Integration Branch Manager available as the small program control crew;
- load a Delivery Lane owner, technical lead, branch owner, or independent
  reviewer only while that lane has admitted work or review;
- unload idle lane roles instead of maintaining standing Workstream queues;
- treat the former XO label as retired terminology; Program Flow owns
  admission, WIP, dependency visibility, and human-readable flow status but
  owns no Product, architecture, source, review, merge, release, or risk
  decision; and
- use capability-domain specialists on demand for a Product Increment rather
  than creating one permanent team per domain.

Create additional adjacent threads only for durable branch-owning work, use
subagents for bounded analysis that reports back, use Workstream Analyst only
when local/CI latency, token burn, or coordination risk requires a throughput
report, and invoke review agents as slash-command or skill workflows unless an
extended integration review needs its own durable thread. Agent count is
capacity, not progress; keep only roles attached to runnable or review-held
work loaded.

For several source-producing computers, use the bounded contract in
[Nexus PoC Multi-Workstation Delivery](../specs/nexus-poc-multi-workstation-delivery.md)
and its
[machine-checked topology](../specs/nexus-poc-workstream-topology.json). A new
computer is capacity, not automatic WIP. Maintain two to four disjoint Delivery
Lanes across the portfolio when review and integration capacity can absorb
them, plus one integration authority per active multi-lane Product Increment.
Give each producer one schema `@4` assignment and one active worktree, and pull
the highest-priority eligible Delivery Lane. Workstreams are capability
domains, not standing queues or source authority.

```bash
node scripts/check-nexus-workstreams.mjs --json
node scripts/check-nexus-workstreams.mjs --context WS-01 --json
node scripts/check-nexus-workstreams.mjs \
  --assignment target/appfw/workstation-assignment.json --live --json
node scripts/audit-worktrees.mjs --json
```

The assignment derives its requested roots from the selected Product Increment
Delivery Lane. It also binds the execution target, planned model tier,
delegation depth, routing basis, and a time/token/cost/retry/correction/rework
budget no larger than the lane budget before dispatch. Local validation proves
source-admission readiness only. Actual write authority additionally requires
the Program Flow Controller's shared live exclusive lease; a copied static
assignment can never create another writer. Tracked Workstream topology
supplies concise capability context and restrictions only. A context-only
lookup returns no allowed write roots and cannot admit source work. Do not copy
those rules into workstation prompts. The worktree audit is report-only;
dirty, locked, detached-unmerged, unpushed, or owner-unknown state requires
explicit owner and Integration Branch Manager review before cleanup.

If a behavior matters for delivery safety or throughput, it belongs in one of:

- `AGENTS.md` for repository-level operating rules;
- `agent_skills/` for concise repo-native procedures;
- `docs/start/` for first-contact harness workflows;
- `docs/specs/` for durable spec-driven decision provenance;
- `docs/reference/` for stable command or workspace contracts;
- `scripts/appfw` or `scripts/ci/` for executable checks;
- `bitbucket-pipelines.yml` for managed CI shape;
- approved package feeds for product-consumable CLI/crates/UI/test harnesses.

Do not depend on local chat history, local `.env`, local slash-command aliases,
untracked regulatory downloads, personal IDE settings, or one person's shell
history as part of the harness contract.

## Clean Machine Bootstrap

From a fresh checkout:

```bash
scripts/appfw doctor
scripts/appfw context --json
scripts/appfw skills --json
scripts/appfw framework instructions --task framework-docs-ia --role coding-agent --change-class C --json
scripts/appfw framework docs-check --changed-only --json
scripts/appfw framework handoff --json
scripts/ci/install-local-git-hooks.sh
```

For framework source changes, add the risk-appropriate commands from
[Agent Task Map](agent-task-map.md). For product app changes, run the product
namespace commands from the same map.

The bootstrap is healthy when:

- `doctor` identifies missing local prerequisites without assuming a personal
  machine configuration;
- `context --json` routes the developer to product/framework ownership;
- `skills --json` lists the repo-native skill pack under `agent_skills/`;
- `instructions` resolves the same repository-relative role card, skill,
  canonical references, and independent-review profile without private prompt
  state, machine paths, environment data, or network lookup;
- changed-only docs-check can run without private local files;
- handoff writes machine-readable changed-surface evidence.
- the local Git pre-push hook is installed from tracked repo scripts and only
  runs the Framework PR Review Agent guard when a branch update is about to be
  pushed, with retained review output freshness checked against the current
  handoff.

## Repo-Owned Harness Surfaces

| Surface | Must be reproducible from | Notes |
| --- | --- | --- |
| Agent operating guide | `AGENTS.md` and companion tool-specific memory files when present | Keep tool-neutral intent in `AGENTS.md`; tool-specific files may point back to it. |
| Skills | `agent_skills/*/SKILL.md` | Skills are the portable procedure pack. They should route to canonical docs, not duplicate long guidance. |
| Slash-command behavior | `docs/start/*-harness.md`, `agent_skills/`, and `scripts/appfw` commands | A slash command may be implemented differently by Codex, Claude, Cursor, or another harness, but the invoked behavior must be documented here. |
| Agent role instructions | [Agent Role Cards](agent-role-cards.md), `AGENTS.md`, and the matching skill/harness docs | Role prompts should name responsibility, collaboration model, push authority, non-authority, and proof loop before work starts. |
| Current-task instruction route | `appfw_cli/src/instruction_routing.rs`, `scripts/appfw`, `agent_skills/README.md`, and canonical role/task docs | The `appfw_instruction_route@1` typed manifest is the sole route producer. Tool-specific adapters must preserve its source links, mandatory rules, independent review, and fail-closed behavior. |
| Delivery profile mode | `scripts/appfw-delivery-mode.py`, `docs/start/delivery-profiles.json`, and root `scripts/appfw mode` | The tracked `accelerated` default must be side-effect-free in fresh primary and linked worktrees. Explicit state belongs below each worktree's Git directory; only clean, exact-SHA `mode set` writes it. Half-present/malformed/noncanonical state fails closed, while dirty or stale report annotations remain descriptive and preserve handoff/review/pre-push freshness. |
| Coordination topology | [Agentic And Human Operating Model](agentic-human-operating-model.md), [Agent Role Cards](agent-role-cards.md), [Nexus PoC Multi-Workstation Delivery](../specs/nexus-poc-multi-workstation-delivery.md), and handoff prompts | Do not rely on private memory to decide whether work should be a durable thread, subagent, slash-command review, or source-producing workstation. |
| Spec-driven change intent | [Spec-Driven Change Harness](spec-driven-change-harness.md), `docs/specs/spec-template.md`, and product-owned specs when applicable | Specs should be as small as possible, but meaningful changes need explicit value, scope, contracts, decisions, risks, and acceptance evidence. |
| Review gates | PR review harness docs, `review-brief`, `change-impact`, handoff, docs-check, `scripts/ci/pre-push-review-guard.sh`, `scripts/git-hooks/pre-push` | Review output must not rely on memory of prior threads. |
| Release/SRA/CAB evidence | `scripts/appfw`, `docs/release/`, `docs/start/pds-sra-package-harness.md`, future CAB package docs | Local packages prepare evidence; human/security/change authorities still approve externally. |
| CI behavior | `bitbucket-pipelines.yml`, `scripts/ci/`, release docs | Temporary diagnostic pipeline edits must be restored before final evidence. |
| Product-consumable platform | ProGet packages, compatibility matrix, `appfw.lock`, product upgrade reports | Product teams should not need an adjacent framework checkout for ordinary product work once packaged consumption is ready. |

## What Must Stay Local

Some inputs are intentionally local or external. They should be documented as
inputs, not copied into the repo:

- `.env` files, tokens, tenant data, credentials, and local Bitbucket tokens;
- `regulatory/` downloads or SRA source documents excluded by `.gitignore`;
- workstation-specific caches, build outputs, target directories, and IDE
  settings;
- external approvals from SRA, CAB, release authority, security, or legal;
- live provider endpoints and production secrets.

When local/external inputs are needed, commands should retain redacted evidence
that names the missing input or external approval path without committing the
source material.

## Slash Command Portability

Slash commands are convenience entry points, not the source of truth. Every
slash-command workflow must have a tool-neutral invocation path:

| Workflow | Portable source of truth |
| --- | --- |
| `/framework-pr-review` | `agent_skills/framework-pr-review/SKILL.md`, [PR Review Agent Harness](pr-review-agent-harness.md), `scripts/appfw framework review-brief --json` |
| `/product-pr-review` | `agent_skills/product-pr-review/SKILL.md`, [Product PR Review Agent Harness](product-pr-review-agent-harness.md), `scripts/appfw product review-brief --json` |
| `/check-pr-review-performance` | [PR Review Agent Harness](pr-review-agent-harness.md) performance oversight procedure |
| `/framework-research-refresh` | `agent_skills/framework-research-steward/SKILL.md`, [Framework Research Steward Harness](framework-research-steward-harness.md) |
| `/pds-sra-package` | `agent_skills/pds-sra-package/SKILL.md`, [PDS SRA Package Harness](pds-sra-package-harness.md), `scripts/appfw framework/product sra-package --json` |
| Future `/pds-cab-package` | Future CAB harness doc and `scripts/appfw framework/product cab-package --json` |

If a team member's tool does not support repository slash commands, they should
run the named skill or CLI command and follow the same harness doc.

## Team Handoff Package

Before handing the harness to a team, prepare a handoff branch or release that
contains:

1. Updated `AGENTS.md`, docs indexes, and `agent_skills/README.md`.
2. All skill directories required by the advertised workflows.
3. CLI commands and docs-check coverage for those workflows.
4. Branch/review/CI delivery model docs, including risk-tiered loops.
5. Spec-driven change harness and template for durable decision provenance.
6. Product packaging guidance for ProGet consumption and upgrade reports.
7. SRA and CAB package harness status, with external-input boundaries.
8. A clean-machine verification transcript or retained artifacts from the
   bootstrap commands above.
9. Known local-only prerequisites and environment variables, with no secrets.

The handoff is not ready when the only way to operate the harness is to copy a
prompt from a chat thread, manually install a personal slash command, or ask
the original developer which files matter.

## Continuous Replication Checks

Treat harness portability as a product quality gate:

- new skills must be linked from `agent_skills/README.md`;
- current-task routing must stay byte-stable, repository-relative, offline,
  read-only, and free of generic inheritance for material work;
- new slash commands must name their tool-neutral skill/doc/CLI fallback;
- docs-check should cover command examples and fail-closed behavior;
- `review-brief` and handoff should expose enough evidence for another machine
  to reproduce review context;
- push-capable agents should rely on repo-owned standing approval rules, not
  private chat memory;
- local pre-push enforcement should be installed from `scripts/ci/install-local-git-hooks.sh`
  and should remain a push-boundary check, not a commit/save-time interruption;
- product-facing harness features should move toward packaged distribution
  through ProGet, not framework-checkout assumptions;
- repeated "works on my machine" findings become tech debt or a harness task.
