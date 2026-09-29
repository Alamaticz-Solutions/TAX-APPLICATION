---
name: product-upgrade
audience: product
phase: upgrade
cli_namespace: product
artifacts: target/appfw/product-upgrade.json,appfw.lock,target/appfw/agent-handoff.json
description: Use when cascading a framework update into a downstream product app, reviewing generated drift, refreshing appfw.lock, and proving the product still passes.
---

# Product Upgrade

## Use When

- Updating a product repo to a newer App Framework checkout or package.
- Reviewing generated drift caused by an upstream framework change.
- Refreshing `appfw.lock`.

## Procedure

1. Run `scripts/appfw product context --json` and confirm the app root.
2. Run `scripts/appfw product upgrade --json` to capture the current delta.
3. Validate, regenerate when intentional, and preserve generated drift output.
4. Run focused product tests, API scenarios when services are up, and handoff.
5. Refresh `appfw.lock` only after the upgrade proof is clean.

## Proof

```bash
scripts/appfw product upgrade --json
scripts/appfw product validate --json
scripts/appfw product generate --check --json
scripts/appfw product test --fast --json
scripts/appfw product handoff --json
```

## Guardrails

- Do not mix framework upgrade drift into unrelated product feature work.
- Do not hide generated drift by patching generated output.
- Do not refresh `appfw.lock` before validation and drift decisions are clear.
