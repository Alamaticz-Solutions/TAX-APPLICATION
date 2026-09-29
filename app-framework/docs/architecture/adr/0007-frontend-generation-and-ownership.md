# ADR 0007: Frontend Generation Model And Ownership Boundary

## Status

Accepted (provisional — this is the highest-risk decision; the CRM reference
frontend is its forcing function, and the analytical and document archetypes
must confirm or correct it).

## Context

A purely runtime-generic renderer (like the admin console) is hard to customize
per product. Pure per-entity code generation bloats and is heavy to keep in
sync. Hand-built apps do not scale across product teams. Agents are fastest and
safest when editing concrete, typed code with an explicit, protected
edit-surface. The scaffold must be a reusable asset distinct from any single
product, or it will over-fit to its first consumer.

## Decision

Hybrid generation with an explicit ownership boundary, mirroring the backend's
generated-versus-human-owned model:

- Generated and overwrite-safe: the typed contract and client (ADR 0006),
  per-entity model-driven scaffolds (list, detail, create/edit, relationship
  navigation), and app-shell wiring.
- Human-owned and preserved: product-specific screens, customizations, and
  overrides — created when missing, then never clobbered.
- A generic runtime renderer covers any entity not yet customized, so a product
  has full CRUD UI on day one.
- The framework scaffold and the product frontend use split roots. An ownership
  manifest (peer of the backend `artifacts.json`) records which files are
  generated versus human-owned, and a drift check proves regeneration reproduces
  generated files and preserves human-owned ones.

## Consequences

- Agent edits are safe by construction: they customize human-owned files;
  regeneration never destroys them.
- The reusable scaffold and the product instance remain separate artifacts — the
  boundary is the product.
- The generated surface must stay small and predictable; resist adding "general"
  machinery that only one product exercises.
- Requires split-root, manifest, and drift tooling on the frontend, not only the
  backend.
