---
name: product-handoff
audience: product
phase: handoff
cli_namespace: product
artifacts: target/appfw/agent-handoff.json
description: Use when finishing product app work and preserving machine-readable continuation state for another agent, reviewer, or CI lane.
---

# Product Handoff

## Use When

- Ending a product app task.
- Preparing review evidence for generated artifacts, tests, frontend work, or
  release readiness.

## Procedure

1. Run the smallest proof loop that matches the changed product surface.
2. Preserve skipped live checks with concrete environment blockers.
3. Run product handoff and include the artifact path in the final summary.

## Proof

```bash
scripts/appfw product validate --json
scripts/appfw product generate --check --json
scripts/appfw product test --fast --json
scripts/appfw product handoff --json
```

## Guardrails

- Do not claim live provider, frontend, or release evidence unless the matching
  command actually ran.
- Do not summarize generated drift without retaining the diagnostic.
