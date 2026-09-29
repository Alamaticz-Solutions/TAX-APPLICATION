# Agentic Development Control System

This concern defines how App Framework lets agents move quickly without letting
large, ambiguous, or security-sensitive code changes bypass human judgment.

The strategic docs are strong only when they behave like a control system:
intent routes the work, the work stays in a reviewable slice, checks prove the
right contract, retained artifacts explain what happened, and humans review the
decisions that should not be delegated.

## Control Loop

Every non-trivial agentic change follows this loop:

1. **Intent source.** Start from the North Star, roadmap, product brief, spec,
   ADR, or issue that states the business value and guardrails.
2. **Route.** Use `docs/start/agent-task-map.md` and the smallest applicable
   skill or lifecycle guide to choose the edit surface and verification lane.
3. **Slice.** Keep the branch lane-sized. Use
   `docs/start/branch-integration-model.md` for multi-lane merge trains.
4. **Build from source of truth.** Change contracts, model, generator source,
   provider source, or docs owners before generated output. Do not patch
   generated-looking files unless the ownership docs say they are product-owned.
5. **Verify by risk.** Run the focused checks required by the touched surface,
   then broader docs/generate/test/release gates only when the risk requires
   them.
6. **Retain evidence.** Preserve JSON artifacts and handoff summaries so the
   reviewer can see scope, skipped checks, generated drift, and remaining risk.
7. **Human review at the right altitude.** Humans review intent, contracts,
   security posture, release decisions, risk acceptance, and broad diffs. Agents
   should not ask humans to inspect generated boilerplate as the primary
   safeguard.
8. **Merge only after the branch shape is honest.** Broad or multi-domain work
   lands through an integration branch with explicit evidence, not as a
   surprise leaf PR.

Use `docs/start/pr-review-agent-harness.md` and the `framework-pr-review` skill
for review-only passes over agent-authored branches. The review agent should
offload diff/evidence inspection and produce a Human Approval Brief; it should
not silently fix the branch it is reviewing.

## Change Classes

| Class | Examples | Minimum evidence | Review posture |
| --- | --- | --- | --- |
| A - focused docs or routing | Small docs IA, task-map, start-here, or roadmap clarification that does not change command behavior. | `git diff --check`; `scripts/appfw framework docs-check --changed-only --json`; `scripts/appfw framework handoff --json`. | Normal review. |
| B - focused source or product behavior | One generator/runtime/provider/product surface with local tests and no release claim. | Surface-specific validation plus targeted tests; `generate --check` when generated contracts can drift; handoff JSON. | Normal review unless a sensitive surface is touched. |
| C - shared or cross-domain framework behavior | Changes spanning generator plus runtime, CLI plus CI, provider plus release, or multiple ownership domains. | Class B evidence plus integration-branch plan, generated-drift report, changed-surface report, and explicit skipped-check reasons. | Human review required before merge. |
| D - security, release, live, provider, AI, or governed-write capability | Auth, policy, tenant isolation, token storage, SaaS writes, MCP/Kafka release posture, dependency acceptances, release gates, AI prompt/egress, mobile secure storage, or provider graduation. | Threat-model mapping, release or provider evidence when relevant, retained risk decision for skipped live proof, and handoff JSON. | Human approval required; cannot be self-certified by an agent. |

## Human Oversight Triggers

A branch must be treated as Class C or D, and therefore needs explicit human
review, when any of these are true:

- The diff changes more than 25 files or more than 1200 non-generated lines.
- The diff touches three or more ownership domains, such as CLI, generator,
  runtime, docs, CI, provider crates, frontend, and product examples.
- The diff changes auth, policy, tenant isolation, token storage, release gates,
  CI security scripts, dependency acceptance, provider capabilities, governed
  writes, MCP/Kafka ingress, AI/chat egress, mobile secure storage, PDS component
  governance, or generated templates.
- The branch promotes a capability from unsupported/report-only/mock-only to
  executable, certified, release-ready, or production-ready.
- The branch changes scores, claims live evidence, or changes release authority.
- The branch writes generated output without a matching source-of-generation
  change and an explicit generated-drift explanation.
- The branch includes local environment files, secrets, tenant/customer data, or
  unclassified imported artifacts.

These triggers are intentionally conservative. They do not block progress; they
force the work into a branch shape and evidence package that a human can review.

## Report-Only Executable Gate

The current report-only command is:

```bash
scripts/appfw framework change-impact --json
```

The command retains `target/appfw/change-impact.json` with:

- changed files and non-generated line delta;
- generated, product-owned, framework-owned, docs, CI, dependency, and release
  buckets;
- sensitive-surface flags;
- ownership-domain count;
- recommended verification commands;
- `requires_human_review`;
- `requires_integration_branch`;
- skipped-check reasons supplied by the agent; and
- optional reviewer approval metadata for Class C/D overrides.

It does not enforce by itself; it gives the human and PR Review Agent a
machine-readable impact summary before review. After two or more green PRs with
report-only output, CI should warn on missing
or stale `change-impact.json`. After the warnings prove stable, release and PR
lanes can fail closed for Class C/D work without a retained change-impact
artifact and human approval reference.

## Relationship To Existing Controls

This control system does not replace the existing docs. It ties them together:

- The North Star owns durable product intent and the faithfulness contract.
- The roadmap owns current priorities, readiness scores, dependencies, and
  release blockers.
- The task map owns first-hop routing from intent to edit surface and checks.
- The branch integration model owns lane shape, merge trains, and conflict
  hygiene.
- The agentic threat model owns agent/tool/identity risks and sandbox posture.
- The threat-model litmus owns risk rows that must move from FAIL/PARTIAL to
  PASS only with evidence.
- Release gates own production authority and must never be satisfied by local
  optimism or fabricated live evidence.

`change-impact --json` is now the preferred first retained artifact for broad
or sensitive work. Agents must still apply the oversight triggers manually when
the command cannot run, and must call out the class, sensitive surfaces, and
human-review need in their handoff.
