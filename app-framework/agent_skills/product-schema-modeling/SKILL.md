---
name: product-schema-modeling
audience: product
phase: model
cli_namespace: product
artifacts: .appfw/model-proposal.yaml,target/appfw/model-proposal.json,target/appfw/model-status.json,.appfw/target/appfw/config_contract.json,.appfw/target/appfw/validation.json
description: Use when designing or changing app_gen schema config, entity types, properties, relationships, facets, data classification, custom methods, or provider data-source bindings.
---

# Product Schema Modeling

## Use When

- Adding or changing schema, entity, property, enum, relationship, facet, seed,
  test, custom method, or data-source config.
- Turning product intent into App Framework source config.

## Procedure

1. Read `docs/model/model-proposal-to-config.md`,
   `docs/model/schema-design.md`, `docs/start/generated-ownership.md`, and
   the generated config contract section for the touched surface.
2. If `.appfw/model-proposal.yaml` exists, treat it as reviewed intake evidence,
   not as authoritative model source. Use its `implementation_plan` to sequence
   source-file creation, entity review, relationship decisions,
   service/custom-method work, frontend follow-up, evidence tasks, and
   validation. Use `review_decisions` to preserve accepted, rejected, deferred,
   split, merged, and still-unresolved choices before writing source config.
3. Run `scripts/appfw product model-status --json` to capture the current
   phase. It should move from `proposal_ready_for_modeling` to
   `proposal_reviewed` only when `review_decisions` is accepted, signed by a
   `model_owner` with `reviewed_at_utc`, and unresolved entries are cleared,
   then to `model_source_started` when source files exist, then to
   `model_validated` only after validation passes. Use
   `source_authoring_plan` from `target/appfw/model-status.json` as the
   machine-readable source-writing queue after proposal sign-off.
4. After signed review, optionally run
   `scripts/appfw product scaffold-model --dry-run --json` to preview starter
   entity/policy source. Run it without `--dry-run` only when accepted entity
   decisions have explicit `config_kind` and `classification`. Treat the output
   as starter source; the agent still owns properties, relationships,
   services, UI contracts, and business semantics.
5. Change source config under `.appfw/model`, not generated output.
6. Model relationships with explicit semantics:
   - `NavToOne` for one parent/reference;
   - `NavToMany` for parent-owned child collections;
   - `NavToMany` with junction semantics for many-to-many.
7. Add classification metadata for regulated profiles.
8. Use custom methods for product operations that are not plain CRUD.
9. Keep provider behavior portable unless a provider limitation is documented.

## Proof

```bash
scripts/appfw product model-status --json
scripts/appfw product scaffold-model --dry-run --json
scripts/appfw product validate --json
scripts/appfw product generate --check --json
scripts/appfw product test --fast --json
```

Use live `provider-test` when provider semantics changed.

## Guardrails

- Do not put schema/entity details in `.appfw/manifest.yaml`.
- Do not patch generated backend/database/API files to fix model mistakes.
- Do not let a relationship bypass policy, tenant, audit, or QueryIR paths.
