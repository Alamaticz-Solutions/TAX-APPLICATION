# Spec-Driven Change Harness Spec

Status: accepted-for-implementation

Spec depth: full

Owner roles:

- Product Owner: owns value, acceptance criteria, and no-spec exceptions.
- Architect: owns technical and contract boundary interpretation.
- XO: owns spec-depth routing, parked decisions, and multi-lane coordination.
- Implementation owner: branch owner for `docs/spec-driven-harness`.
- Review owner: Framework PR Review Agent plus human reviewer.

## Business Value

App Framework needs higher agentic throughput without losing decision quality.
As more agents and product teams produce branches in parallel, unclear intent
creates rework, stale reviews, merge churn, and human approval fatigue. A small
spec-driven harness gives agents a shared target for meaningful changes while
letting tiny fixes stay fast.

## Problem

The existing harness is strong on review, handoff, role boundaries, and CI
evidence, but it does not explicitly say when intent should be captured before
implementation. That creates risk that agents move quickly against implicit
assumptions, especially for framework contracts, product workflows, generated
output, security/governance, release gates, and multi-agent wave work.

## Goals

- Define when an intent note, lightweight spec, or full spec is required.
- Make specs operational: they must drive implementation, tests, docs, review,
  handoff, tech debt, and decision provenance.
- Frame the harness as spec-anchored context-driven development, not static
  spec-first bureaucracy.
- Keep human attention on business value, scope, risk, tradeoffs, and evidence;
  avoid arbitrary low-level specs.
- Keep the default path lightweight so small fixes are not slowed by ceremony.
- Make PR review responsible for spec-to-diff-to-evidence alignment when a spec
  is required.
- Make the pre-push review gate print the reviewer recommendation summary so
  human reviewers see status, counts, conditions, attention items, depth, and a
  full-review link without opening JSON first.
- Preserve the behavior in repo-owned docs and skills so another workstation
  can reproduce it.

## Non-Goals

- Do not create a new standing spec agent.
- Do not require a durable spec for tiny low-risk fixes.
- Do not replace ADRs, PR review, SRA/CAB approval, release approval, or human
  risk acceptance.
- Do not add CLI enforcement in this branch.

## Scope

In scope:

- `docs/start/spec-driven-change-harness.md`
- `docs/specs/`
- `AGENTS.md`
- `docs/README.md`
- `agent_skills/README.md`
- role, task-map, review-harness, integration, replication, and tech-debt docs
- framework and product PR review skills

Out of scope:

- `scripts/appfw` spec commands
- docs-check assertions for spec presence
- Bitbucket/CI enforcement of spec depth
- product-specific specs outside this repository
- broader review UI changes outside the local pre-push guard output

## Contracts Touched

| Contract Surface | Expected Change | Counterpart Surfaces That Must Stay Aligned |
| --- | --- | --- |
| Agent operating guide | Agents must consider spec depth before meaningful work. | `agent-task-map`, role cards, review harnesses, skills. |
| PR review harnesses | Reviewers check spec-to-diff-to-evidence alignment when required. | `framework-pr-review` and `product-pr-review` skills. |
| Pre-push review guard | Human-readable output must include final status, severity counts, conditions-captured value, Attention Items when present, review depth, and retained review output link. | `review-brief`, role cards, CLI reference, and review harness docs. |
| Team replication | Spec discipline must be reproducible from repo docs. | `docs/README.md`, `agent_skills/README.md`, `team-harness-replication.md`. |
| Tech debt | Missing/stale spec conditions become explicit debt or blockers. | PR review output and tech debt register. |

## Options Considered

| Option | Pros | Cons | Decision |
| --- | --- | --- | --- |
| No spec discipline | Fastest short-term path. | Leaves intent in chat memory and increases agentic drift. | Rejected. |
| Heavy spec-first process | Strong provenance and review target. | Too much ceremony; slows tiny fixes and encourages spec theater. | Rejected. |
| Lightweight depth-based harness | Preserves intent for meaningful work while keeping small fixes fast. | Requires reviewer judgment on spec depth. | Selected, with repository context and execution feedback treated as required companions to the spec. |

## Decision Provenance

| Date | Owner | Decision | Evidence / Rationale | Revisit Trigger |
| --- | --- | --- | --- | --- |
| 2026-07-07 | Human owner + Strategist/XO thread | Adopt a lightweight, depth-based spec-driven harness. | User requested safety and decision provenance without undue complexity; current harness already routes roles/review/debt. | Revisit if specs become paperwork or repeated reviews find missing intent. |
| 2026-07-07 | Human owner + Strategist/XO thread | Keep specs value-first and avoid arbitrary low-level specs. | Human clarified that high-level value is where human focus belongs; low-level spec burden would slow the harness. | Revisit if reviews show agents are missing critical low-level contract or safety decisions. |

## Architecture And Implementation Notes

The harness is documentation-first in this branch. It introduces a canonical
start-here guide and template, then wires the behavior into the docs and skills
agents already use. This avoids a new role or CLI command until the team has
evidence that automation would reduce friction.

The implementation deliberately treats specs as intent anchors inside the live
repository context and execution loop. Agents still have to inspect existing
code/docs/CLI/skills, generate or choose tests, run checks, read failures, and
patch against real evidence. The spec keeps that loop aligned to business value
and decision provenance.

The harness intentionally leaves ordinary implementation details to the
Architect/Coding Agent and proof loop. Specs should descend into low-level detail
only when the detail changes a contract, generated boundary, security/compliance
posture, integration behavior, or acceptance evidence.

Future automation can add `scripts/appfw framework spec ...` or docs-check
coverage, but only after the manual contract proves useful.

The pre-push review guard promotes the parsed Recommendation Summary and
non-`GO` Attention Items into both the retained pre-push JSON and the
human-readable terminal output. This keeps the human approval moment centered on
the reviewer decision and the concrete conditions/blockers instead of a bare
pass or fail message.

## Security, Privacy, And Governance

The harness increases scrutiny for changes touching security, privacy, PHI/PII,
tenant isolation, SRA/CAB, release gates, governed AI/tool egress, secrets, and
authorization. It does not approve risk. Human authorities remain responsible
for accepted risk, release, SRA, and CAB decisions.

## Acceptance Evidence

| Criterion | Proof Command / Artifact | Required Before |
| --- | --- | --- |
| Docs and skills expose the spec-depth rule. | `rg -n "Spec-Driven|spec-to-diff|spec depth" AGENTS.md docs agent_skills` | push |
| Specs are framed as context/execution anchored, not static-only. | `rg -n "Context And Execution Loop|Repository Context|Test And Execution Feedback Plan" docs/start/spec-driven-change-harness.md docs/specs` | push |
| Specs are value-first and avoid arbitrary low-level detail. | `rg -n "business value|Avoid arbitrary low-level specs|avoid arbitrary low-level specs|low-level implementation detail" docs/start/spec-driven-change-harness.md docs/specs` | push |
| Markdown/diff is clean. | `git diff --check` | push |
| Framework config remains valid. | `scripts/appfw framework validate --json` | push |
| Docs/skill examples remain valid. | `scripts/appfw framework docs-check --changed-only --json` | push |
| Generated output has no drift. | `scripts/appfw framework generate --check --json` | push |
| Fast framework test remains green. | `scripts/appfw framework test --fast --json` | push |
| Non-`GO` reviews expose human attention items. | `scripts/appfw framework review-brief --auto-depth --review-output target/appfw/framework-pr-review.md --json` reports `attention_items_satisfied:true`. | push |
| Pre-push guard prints the review decision summary. | `scripts/ci/pre-push-review-guard.sh` shows `Review summary:` with status, severity counts, conditions, Attention Items, depth, and a full-review link. | push |
| Handoff and PR review evidence retain the spec path. | `scripts/appfw framework handoff --json`; `scripts/appfw framework review-brief --auto-depth --json` | push |

## Risks And Controls

| Risk | Control | Owner | Status |
| --- | --- | --- | --- |
| Spec process slows small fixes. | Explicit intent-note path for tiny low-risk fixes. | Product Owner + XO | mitigated |
| Specs become stale prose. | PR review checks spec-to-diff-to-evidence alignment; stale specs become findings. | PR Review Agent | mitigated |
| Agents treat specs as approval. | Harness states specs do not approve architecture, risk, release, SRA, or CAB. | Human owner + reviewers | mitigated |
| Automation is added too early. | CLI enforcement is out of scope until manual contract proves useful. | Product Owner + Architect | accepted |

## Tech Debt And Follow-Up

- Consider future CLI support only after teams have used the manual harness.
- Consider docs-check coverage for spec discoverability if missing/stale specs
  become recurring review findings.
- Keep the known docs-check throughput concern in the Workstream Analyst queue.

## Handoff Notes

This branch should be reviewed as a governance/review-harness change. It should
not block wave implementation once pushed, but future meaningful wave work
should cite either an intent note or a spec according to this harness.
