# Runtime Extraction Coordination

Use this note when coordinating work that touches runtime, provider, DataAccess,
admin, MCP, generated route, or product-template boundaries.

Active branch ownership should live in the current thread handoff or PR
description. This file is the stable coordination checklist; it should not keep
stale branch names, active-lane claims, or PR-by-PR history.

## Coordinate Before Editing

Coordinate with other active lanes before changing:

- runtime route, auth, security, CORS, observability, GraphQL, admin, or MCP
  shells;
- `backend/src/data/**`, QueryIR, DataAccess, audit, validation, or record
  rules;
- concrete provider clients or provider packages;
- generated route/schema/handler templates;
- CRM sample backend or `appfw new --profile crm-sample` output;
- release, provider, security, performance, or handoff evidence scripts.

The safest parallel work is usually:

- documentation alignment;
- read-only architecture review;
- focused tests for already-merged behavior;
- command/artifact naming checks;
- downstream product education that does not change generated output.

## Runtime Boundary Guardrails

Do not merge a runtime extraction or packaging slice if any of these are true:

1. Product handlers or services import provider internals directly.
2. Runtime contracts require generated schema structs instead of generated
   metadata adapters.
3. DataAccess bypasses policy, tenant isolation, audit, query-budget,
   pagination, provider timing, or redaction paths.
4. Root backend and CRM sample product no longer exercise the same runtime
   path.
5. Root or CRM backend fails to compile after runtime/provider dispatch edits.
6. Generated drift is hidden instead of reported.
7. Provider-visible behavior changed without provider certification evidence.

## Sequencing Rules

Use this order for broad runtime/product-template movement:

1. Prove the existing runtime path locally.
2. Move one reusable contract or behavior slice to runtime/provider packages.
3. Add product adapters that bind generated metadata, operation dispatch,
   policy engines, active data sources, branding/topology, or audit sinks.
4. Run framework and CRM sample validation/generation/tests.
5. Slim product templates only after the runtime contract is real and proven.
6. Record live-provider release evidence separately when the change affects
   persistence, filtering, audit, tenant isolation, or provider errors.

Avoid combining template slimming, provider behavior changes, MCP/admin service
changes, and release-gate script changes in one PR unless there is no smaller
safe path.

## Verification Spine

Start focused, then run the app framework loop:

```bash
scripts/appfw validate --json
scripts/appfw boundary-check --json
scripts/appfw generate --check --json
scripts/appfw test --fast --json
examples/products/crm/scripts/appfw validate --json
examples/products/crm/scripts/appfw boundary-check --json
examples/products/crm/scripts/appfw generate --check --json
examples/products/crm/scripts/appfw test --fast --json
scripts/appfw docs-check --json
scripts/appfw handoff --json
```

Add focused Rust tests near the changed runtime/provider modules. Run live
provider certification when provider-visible semantics change:

```bash
scripts/appfw provider-test --all --json
```

Run release evidence when the slice is release-relevant:

```bash
scripts/appfw release-check --json
```

## Handoff Template

Use this in another thread or PR when runtime ownership is in play:

```text
Runtime/package boundary touched. Please check:
- product handlers/services do not import provider internals;
- generated metadata adapters remain the product boundary;
- DataAccess still enforces policy, tenant isolation, audit, query budgets,
  pagination, provider timing, and redaction;
- framework and CRM sample checks both pass validation/generation/tests;
- provider-test/release-check evidence is retained when provider-visible or
  release-relevant behavior changed.
```

## Related Docs

- [Architecture](overview.md)
- [Framework Packaging](framework-packaging.md)
- [Product Packaging Extraction Plan](product-packaging-extraction-plan.md)
- [Runtime Ownership Inventory](runtime-ownership-inventory.md)
- [Product Workspace Contract](../reference/product-workspace-contract.md)
