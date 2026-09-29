---
name: product-generate-verify
audience: product
phase: generate-extend
cli_namespace: product
artifacts: .appfw/target/appfw/artifacts.json,target/appfw/agent-handoff.json
description: Use when running App Framework generation, reviewing generated artifacts, checking drift, or deciding whether a change belongs in config, generator code, templates, or a human-owned extension point.
---

# Product Generate And Verify

## Use When

- A config, generator, or template change should update generated artifacts.
- Generated drift appears.
- An agent needs to decide whether a file is generated or human-owned.

## Procedure

1. Run validation first.
2. Inspect ownership before editing generated-looking paths:
   `scripts/appfw product explain ownership <path> --json`.
   Use `docs/start/generated-ownership.md` and `docs/reference/cli.md` for the deeper command
   and ownership contracts.
3. For repeated behavior, edit config, `app_gen/src`, or `app_gen/_templates`.
4. For product behavior, edit human-owned handlers, services, policies, or
   frontend features.
5. Run generation only when output is intentionally updated.
6. Preserve generated drift diagnostics if check mode fails.

## Proof

```bash
scripts/appfw product validate --json
scripts/appfw product generate
scripts/appfw product generate --check --json
scripts/appfw product test --fast --json
scripts/appfw product handoff --json
```

## Guardrails

- Do not run generation only to discover ownership.
- Do not hide drift by hand-editing generated output.
- Do not overwrite human-owned handler implementations.
