---
name: product-bootstrap
audience: product
phase: bootstrap
cli_namespace: product
artifacts: .appfw/manifest.yaml,target/appfw/agent-handoff.json
description: Use when creating a new downstream App Framework product workspace, choosing product topology, or ensuring the new repo has clean product intent with no sample residue.
---

# Product Bootstrap

## Use When

- Creating a downstream product repo.
- Choosing app identity, schema, provider, MCP, Kafka, or UI topology.
- Verifying the generated workspace is product-owned, not a renamed sample.

## Procedure

1. Read `docs/lifecycle/product-golden-path.md` and `docs/start/cli-quickstart.md` only as
   needed.
2. Pick the correct profile:
   - `crm-sample` only for a complete reference app.
   - `product-intake` for a new product or PoC conversion.
3. Capture explicit topology: app name, display name, schema, backend provider,
   MCP required, Kafka required, and UI mode.
4. Run `scripts/appfw product new <target> --from current --profile product-intake`
   with flags or interactive prompts.
5. Inspect the new repo for product naming and absence of CRM/sample residue.
6. For framework stewardship of this path, run
   `scripts/appfw framework intake-proof --json` to prove the disposable
   product-intake/analyze/proposal lane.
7. Do not generate until the product model has been authored from product
   evidence.

## Proof

```bash
scripts/appfw product new --list-profiles --json
scripts/appfw framework intake-proof --json
cd <target>
scripts/appfw product doctor
scripts/appfw product validate --json
scripts/appfw product handoff --json
```

## Guardrails

- Do not add credentials or local `.env` files.
- Do not leave CRM entity names, routes, docs, or UI screens in a new product.
- Do not edit generated-looking files without checking ownership first.
