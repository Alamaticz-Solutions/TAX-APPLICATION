---
name: product-frontend
audience: product
phase: frontend
cli_namespace: product
artifacts: target/appfw/frontend-test.json,target/appfw/agent-handoff.json
description: Use when building or changing an enterprise product frontend, generated UI contract, CRM reference frontend pattern, data grid, form, dashboard, theme, E2E, or accessibility evidence.
---

# Product Frontend

## Use When

- Creating or changing product frontend screens, dashboards, grids, forms, or
  generated UI contract usage.
- Converting a PoC UI into an enterprise frontend.

## Procedure

1. Read `docs/frontend/product-frontend.md` for scaffold boundaries.
2. Use generated model/API contracts instead of hard-coded schema assumptions.
3. Keep product features under product-owned frontend modules.
4. Prefer server-side query/search/pagination for data grids.
5. Implement mature form behavior: validation, dirty detection, save/cancel,
   delete confirmation, access-aware buttons, lookup/entity selectors.
6. Preserve brand tokens and dark/light mode without forking admin UI.
7. Verify with CLI-runnable type, build, E2E, and a11y evidence.

## Proof

```bash
cd frontend
npm run appfw:check
npm run typecheck
npm run build
cd ..
scripts/appfw product frontend-test --json
scripts/appfw product handoff --json
```

## Guardrails

- Do not copy the admin UI as the product frontend.
- Do not bypass generated contracts for entity shape or access semantics.
- Do not ship frontend changes without a11y/E2E evidence when UI is in scope.
