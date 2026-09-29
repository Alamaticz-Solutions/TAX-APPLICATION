# AFS-008 Current-Task Instruction Routing

Status: accepted-for-implementation

Spec depth: full

Owner roles:

- Product Owner: owns outcome and acceptance.
- Architect: owns the Class C boundary, typed contract, frozen write set, and proof route.
- XO: owns release and dispatch.
- Implementation owner: AFS-008 Model-Routing Coding Agent.
- Review owner: independent Framework PR Review Agent, Sol XHigh comprehensive.

## Business Value

Developers and agents can begin a material task with the smallest applicable
instruction set and an explainable independent-review route. This reduces
startup context while preserving source provenance, mandatory rules, and human
authority.

## Problem

The repository has strong role, skill, and reference guidance, but workers must
manually correlate it before each task. Manual selection can overload context,
miss a mandatory source, blur implementation and review routes, or silently
inherit an unsuitable generic profile.

## Goals

- Resolve one provider-neutral task/role/class record to exactly one producer
  role card, one skill, and minimal canonical references.
- Return distinct implementation and independent-review profiles, with Class C
  requiring comprehensive review.
- Fail closed for unknown, mismatched, incomplete, or ambiguous input.
- Produce byte-stable, repository-relative, offline, read-only JSON.

## Non-Goals

- Starting work, changing WIP, dispatching a provider model, or granting
  authority.
- Copying instruction bodies, rewriting broad policy, or creating a generic
  agent configuration platform.
- Persisting raw provider model IDs, telemetry, task status, or acceptance.
- Allowing implementation self-review, silent fallback, or generic inheritance
  for material work.

## Scope

The exact write set is the CLI typed resolver and dispatcher, focused tests,
docs-check semantic gates, the skill/task/CLI/replication docs, and this spec.
The initial manifest contains four framework routes and three explicitly
supported product routes. Existing role cards and skills remain canonical and
unchanged.

## Repository Context

The resolver consumes the role boundaries in `docs/start/agent-role-cards.md`,
the procedures indexed by `agent_skills/README.md`, and canonical references
already routed by `docs/start/agent-task-map.md`. `scripts/appfw` remains the
public command entry, while `appfw_cli` owns the typed producer.

## Contracts Touched

| Contract Surface | Expected Change | Counterpart Surfaces That Must Stay Aligned |
| --- | --- | --- |
| CLI | Add namespaced `instructions --task --role --change-class --json`. | CLI reference, quickstart, task map, wrapper help. |
| JSON | Add `appfw_instruction_route@1`. | Rust fixtures, shell semantic gates, docs-check examples. |
| Skills and roles | Reveal one existing skill and producer role card by source link. | Skill index and role-card headings remain canonical. |
| Review | Emit a separate provider-neutral reviewer profile. | Class C comprehensive requirement and existing review harness. |

## Options Considered

| Option | Pros | Cons | Decision |
| --- | --- | --- | --- |
| Typed Rust manifest with thin shell dispatch | Closed enums, one producer, deterministic tests, package-compatible. | Requires compiled CLI availability or source fallback. | Selected. |
| Duplicate route table in shell | Direct execution. | Creates a second authority and weak typing. | Rejected. |
| Copy instruction bodies into JSON | Self-contained output. | Drifts from canonical sources and expands context. | Rejected. |

## Decision Provenance

| Date | Owner | Decision | Evidence / Rationale | Revisit Trigger |
| --- | --- | --- | --- | --- |
| 2026-07-15 | Product Owner / Architect / XO | Release the frozen Class C typed resolver lane on accepted main `d038f51b`. | P1 checkpoint complete, PR #413 accepted, exact write set frozen. | Route ambiguity, source ownership conflict, or required broad policy rewrite. |
| 2026-07-15 | Architect | Keep durable routes provider-neutral and pin implementation/review separately. | Provider adapters may change; authority and review independence may not. | Approved adapter contract changes. |

## Architecture And Implementation Notes

`appfw_cli/src/instruction_routing.rs` is the sole manifest and resolver. Closed
enums parse namespace, task, producer role, and change class. Each task binds a
namespace, minimum class, one role card, one skill, and two or three canonical
references. `scripts/appfw` only dispatches to the packaged CLI or locked source
crate. Material routes prohibit generic inheritance and silent substitution.

## Security, Privacy, And Governance

The command performs no network lookup or repository/data mutation and returns
no timestamp, absolute path, CWD, environment value, secret, tenant data, or
instruction body. It does not approve risk, merge, release, SRA/CAB, or a model
route. Existing XO, Architect, reviewer, and human authorities remain intact.

## Acceptance Evidence

| Criterion | Proof Command / Artifact | Required Before |
| --- | --- | --- |
| Typed route and fail-closed inputs | `cargo test --locked -p appfw-cli --test instruction_routing` | checkpoint commit |
| Public CLI and byte stability | invoke the Class C command twice and compare bytes | checkpoint commit |
| Shell semantic behavior | `bash scripts/cli-semantic-gates-test.sh` | checkpoint commit |
| Docs and command parity | `scripts/appfw framework docs-check --json` | handoff |
| Framework regression posture | validate, generate-check, and fast framework tests | handoff |
| Independent review contract | `scripts/appfw framework review-brief --comprehensive --json` | independent review |

## Test And Execution Feedback Plan

Run Rust fixtures first, then the direct namespaced command, negative shell
semantics, docs-check, framework validation, generated drift, and fast tests.
Any nondeterminism, ambiguous route, weakened source link, or implementation /
review conflation stops the lane and returns to the Architect.

## Risks And Controls

| Risk | Control | Owner | Status |
| --- | --- | --- | --- |
| Route table becomes a second policy authority. | Link existing role/skill/docs; copy no bodies. | Architect | controlled |
| Provider adapter weakens mandatory rules. | Provider-neutral pinned records; fail-closed fallback. | Architect / XO | controlled |
| Implementer self-certifies. | Separate mandatory reviewer record and review brief. | Review owner | controlled |
| Product route is implied where unsupported. | Explicit namespace/task match and closed product enum. | Coding Agent | controlled |

## Tech Debt And Follow-Up

Provider-specific adapters, execution telemetry, parent-child persistence, and
additional task routes remain separate Architect-owned slices. They must not be
inferred from this disclosure command.

## Handoff Notes

Review the spec-to-diff-to-evidence chain comprehensively. This branch may be
committed locally but may not be pushed, opened as a PR, merged, released, or
published by the implementation owner.

## Delivery-Mode Boundary

The related Class D delivery-mode controller remains outside `appfw_cli`; the
typed resolver keeps its early public-wrapper dispatch and argument-order
contract. Delivery mode may annotate evidence but cannot add generic routing,
model-adapter behavior, or implementation self-review.
