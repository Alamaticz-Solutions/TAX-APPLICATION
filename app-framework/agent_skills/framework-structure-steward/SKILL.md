---
name: framework-structure-steward
audience: framework
phase: structure-stewardship
cli_namespace: framework
artifacts: target/appfw/maintainability-contract.json,target/appfw/change-impact.json,target/appfw/agent-handoff.json
description: Use as the Framework Structure Steward when reviewing docs IA, skills, CLI contracts, code organization, generated boundaries, and maintainability drift; produces maintenance recommendations for the Product Owner/Strategist instead of implementing broad refactors by default.
---

# Framework Structure Steward

## Use When

- Reviewing whether framework changes increase entropy across docs, skills,
  CLI, code organization, generated boundaries, or release evidence.
- Preparing a wave refresh, integration-branch review, or team handoff.
- Turning repeated review findings, CI friction, stale docs, broad diffs, or
  confusing ownership into Product Owner/Strategist maintenance items.

## Procedure

1. Start from `docs/architecture/concerns/maintainability.md`,
   `docs/start/team-harness-replication.md`, `docs/start/branch-integration-model.md`,
   and `docs/start/agent-task-map.md`.
2. Inspect the current diff, handoff, docs-check artifacts, and change-impact
   report. Sample code/module layout when source organization is in scope.
3. Assess these lenses:
   - docs information architecture and canonical ownership;
   - skill size, discoverability, and tool-neutral portability;
   - CLI namespace, command contract, JSON artifact, and docs-check coverage;
   - code/module/package boundaries and generated vs human-owned separation;
   - CI/review friction that should move left into local checks or docs-check;
   - stale, duplicate, or historical content in live paths.
4. Produce a maintenance brief for the Product Owner/Strategist with:
   - `Verdict`: `healthy`, `watch`, or `needs-maintenance`;
   - top entropy risks;
   - recommended maintenance backlog items, each with owner, priority, and
     evidence source;
   - changes that should be handled by a lane branch instead of opportunistic
     cleanup inside the current feature branch.
5. Return the brief to the Architect for technical review before prioritization.
   The Architect should confirm whether each finding is real, assess dependency
   order and blast radius, and parley with the Product Owner for backlog
   priority, the Strategist/Product Manager function for strategic fit, and XO
   for coordination/escalation when needed.
6. Make the expected decision explicit for each recommendation: add to immediate
   wave lane, create focused maintenance lane, add to tech debt register, defer
   with rationale, or reject as not worth platform investment.

## Proof

```bash
scripts/appfw framework change-impact --json
scripts/appfw framework docs-check --changed-only --json
scripts/appfw framework handoff --json
```

Add focused proof when the stewardship finding touches source behavior:

```bash
scripts/appfw framework cli-test --json
scripts/appfw framework generate --check --json
scripts/appfw framework test --fast --json
```

## Guardrails

- This steward recommends structure work; it should not sneak broad refactors
  into unrelated feature branches.
- Do not create a new canonical doc, skill, CLI command, or package boundary
  when an existing owner can be clarified.
- Do not treat docs-check as proof that the information architecture is healthy;
  docs-check proves executable examples and selected contracts, not reader
  clarity by itself.
- Send maintenance recommendations back to the Product Owner and
  Strategist/Product Manager function so they can be prioritized against
  business value, throughput, risk, and roadmap commitments.
- Preserve the role boundary: Structure Steward detects and recommends;
  Architect evaluates feasibility and sequencing; Product Owner prioritizes
  current backlog work; Strategist/Product Manager challenges strategic fit.
