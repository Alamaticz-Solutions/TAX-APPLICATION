---
name: framework-frontend-design-system
audience: framework
phase: frontend
cli_namespace: framework
artifacts: target/appfw/docs-check.json,target/appfw/agent-handoff.json
description: Use when changing PDS Health design-system tokens, reusable frontend components, CRM-neutral product scaffold guidance, or frontend design-system release evidence.
---

# Framework Frontend Design System

## Use When

- Changing PDS Health tokens, component inventory, or shared frontend guidance.
- Extracting reusable UI primitives from CRM or admin UI.
- Changing CRM-neutral product scaffold expectations or residue checks.

## Procedure

1. Read `docs/frontend/pds-health-design-system.md` for design-system ownership,
   maturity gates, and evidence expectations.
2. Read `docs/frontend/product-frontend.md` for product consumption boundaries.
3. Read `appfw_ui/pds_health/reference/catalog.json` for the agent-readable
   component family manifest, source API map, decision guide, maturity level,
   evidence requirements, and product boundary before adding product-local
   controls or wrappers.
4. For token changes, edit
   `appfw_ui/pds_health/tokens/tokens.dtcg.json` first, run
   `node scripts/generate-pds-tokens.mjs`, and treat `pdsTokens.css` and
   `pdsTokens.ts` as generated outputs. `--bootstrap-from-files` is a
   review-only migration escape hatch, not the normal authoring path.
5. Keep PDS Health brand assets framework-owned and product workflow code
   product-owned.
6. Prefer the package catalog exports `pdsComponentCatalog`,
   `pdsComponentFamilies`, and `pdsAgentDecisionGuide` over product-local
   component maps when frontend or generator code needs discovery metadata.
7. Treat CRM as a reference/E2E fixture only; do not make CRM the base scaffold.
8. Preserve `--pds-*` tokens and document any token/component migration path.
9. Add or update evidence checks when a reusable frontend contract changes.
10. When product scaffold behavior changes, ensure `framework intake-proof`
   retains `target/appfw/product-intake-proof/frontend-residue-check.json` plus
   `target/appfw/product-intake-proof/frontend-scaffold-check.json`, and
   generated `npm run appfw:check` retains
   `frontend/target/appfw/frontend-scaffold-check.json`.
11. When component source changes, run `scripts/check-pds-components.mjs --json`
   and keep `target/appfw/pds-component-check.json` as release evidence,
   including `component_api_inventory` for component family/source/props
   lookup, `package_catalog` for importable package metadata, and
   `agent_decision_guide` for workflow-to-component starts.

## Proof

```bash
node scripts/generate-pds-tokens.mjs --check --json
node scripts/check-pds-tokens.mjs --json
scripts/check-pds-components.mjs --json
scripts/appfw framework docs-check --json
scripts/appfw framework validate --json
scripts/appfw framework generate --check --json
scripts/appfw framework test --fast --json
scripts/appfw framework handoff --json
```

Add `scripts/appfw framework golden-downstream --json` and product frontend
package checks when generator or scaffold behavior changes.

## Guardrails

- Do not copy CRM screens, entities, storage keys, or fixtures into the base
  product scaffold.
- Do not fork admin UI into a product frontend.
- Do not hand-edit `pdsTokens.css` or `pdsTokens.ts`; change
  `tokens.dtcg.json`, regenerate, and drift-check both outputs.
- Do not introduce product-local brand tokens unless they map back to canonical
  PDS `--pds-*` variables.
- Do not add product-local controls before checking the PDS catalog manifest
  plus retained component API inventory and decision guide for an existing
  family, workflow recipe, maturity level, evidence requirement, exported
  component, source file, or props type.
- Do not claim a component family is enterprise-ready without type/build,
  accessibility, and reference-app evidence.
