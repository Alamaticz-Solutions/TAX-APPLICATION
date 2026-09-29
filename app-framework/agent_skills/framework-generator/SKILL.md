---
name: framework-generator
audience: framework
phase: generator
cli_namespace: framework
artifacts: .appfw/target/appfw/artifacts.json,.appfw/target/appfw/config_contract.json
description: Use when changing app_gen validation, config contracts, templates, generation behavior, or generated-boundary enforcement.
---

# Framework Generator

## Use When

- Editing `app_gen/src`, `app_gen/_templates`, config contract generation, or
  generated artifact metadata.

## Procedure

1. Confirm the change belongs in framework generator code, not product config.
2. Update validation/config-contract code before generated contract docs.
3. Run generation and deterministic drift checks.
4. Add focused generator tests for new behavior.

## Proof

```bash
scripts/appfw framework validate --json
scripts/appfw framework generate
scripts/appfw framework generate --check --json
scripts/appfw framework test --fast --json
scripts/appfw framework handoff --json
```

## Guardrails

- Do not patch generated product output as the source of truth.
- Do not add config fields without validation and contract docs.
