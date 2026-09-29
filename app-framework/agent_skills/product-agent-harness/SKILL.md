---
name: product-agent-harness
audience: product
phase: safe-change-loop
cli_namespace: product
artifacts: .appfw/agent-profile.yaml,.appfw/target/appfw/harness-check.json,target/appfw/agent-handoff.json
description: Use when proving that a product agent is constrained to least-privilege commands, write paths, network posture, and handoff before changing a product app.
---

# Product Agent Harness

Use this skill when a product agent needs to prove it is operating under the
least-privilege App Framework product profile before changing a product app.

## Procedure

1. Read `.appfw/agent-profile.yaml` in the product app.
2. Confirm the profile only grants product-scoped commands, product-owned
   writable paths, disabled network/live-service access, disabled sensitive
   capabilities, G3 threat controls, and required handoff.
3. Run:

   ```bash
   scripts/appfw product harness-check --json
   ```

4. Treat `.appfw/target/appfw/harness-check.json` as retained evidence. If the
   command fails, fix the profile or reduce the agent scope before continuing.
5. Before handoff, run:

   ```bash
   scripts/appfw product handoff --json
   ```

## Proof

```bash
scripts/appfw product harness-check --json
scripts/appfw product handoff --json
```

## Guardrails

- Do not grant framework commands to a product agent profile.
- Do not enable MCP, Kafka, release, network, live-service, or SaaS governed
  write access without a future reviewed evidence lane that explicitly allows
  it.
- Keep detailed threat guidance in
  `docs/architecture/concerns/agentic-threat-model.md`; keep command details in
  `docs/reference/cli.md`.
