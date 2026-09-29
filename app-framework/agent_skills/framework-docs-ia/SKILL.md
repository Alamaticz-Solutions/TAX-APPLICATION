---
name: framework-docs-ia
audience: framework
phase: docs-ia
cli_namespace: framework
artifacts: target/appfw/maintainability-contract.json,target/appfw/docs-check.json
description: Use when changing documentation information architecture, agent skills, CLI command contracts, or docs-check enforcement for App Framework itself.
---

# Framework Docs IA

## Use When

- Changing docs hierarchy, architecture decisions, agent skills, or CLI docs.
- Moving live docs to archive or adding a new canonical document.

## Procedure

1. Start from `docs/framework/README.md`,
   `docs/architecture/concerns/maintainability.md`, and ADR 0013.
2. Preserve the product/framework seam in docs, skills, and CLI examples.
3. Keep skills procedural and route deep knowledge to canonical docs.
4. Update docs-check whenever a command, skill, or IA contract changes.

## Proof

```bash
scripts/appfw framework docs-check --json
scripts/appfw framework validate --json
scripts/appfw framework handoff --json
```

## Guardrails

- Do not put framework stewardship guidance in product lifecycle docs.
- Do not link archived ledgers as current operating guidance.
