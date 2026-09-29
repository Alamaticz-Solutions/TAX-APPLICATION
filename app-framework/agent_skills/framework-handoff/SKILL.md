---
name: framework-handoff
audience: framework
phase: handoff
cli_namespace: framework
artifacts: target/appfw/agent-handoff.json
description: Use when finishing framework steward work and preserving machine-readable continuation state, changed surfaces, verification, and remaining risks.
---

# Framework Handoff

## Use When

- Ending framework work on docs IA, generator, runtime, provider, CLI, release,
  or dependency surfaces.

## Procedure

1. Run the proof loop matching the framework surface.
2. Preserve skipped live checks with concrete blockers.
3. Run handoff and summarize changed framework surfaces.
4. Before any push, run `scripts/appfw framework review-brief --auto-depth
   --json`, invoke `/framework-pr-review` or `/framework-pr-review
   --comprehensive` as indicated, show the human the structured output, and push
   only when the standing approval policy in `docs/start/agent-role-cards.md`
   applies or explicit human approval is recorded. Bare `review-brief --json`
   defaults to focused mode.

## Proof

```bash
scripts/appfw framework docs-check --json
scripts/appfw framework validate --json
scripts/appfw framework generate --check --json
scripts/appfw framework test --fast --json
scripts/appfw framework handoff --json
scripts/appfw framework review-brief --auto-depth --json
```

## Guardrails

- Do not rely only on product-root handoff for framework docs/CLI changes.
- Do not leave stale scorecard, threat model, or docs-check claims.
- Do not push an agent-authored branch unless the Framework PR Review Agent
  output satisfies the standing approval policy in
  `docs/start/agent-role-cards.md` or the human gives an explicit override.
