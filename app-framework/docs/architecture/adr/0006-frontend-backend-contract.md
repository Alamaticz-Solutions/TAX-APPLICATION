# ADR 0006: Frontend And Backend Contract Is Generated From The Model

## Status

Accepted (provisional — confirmed by the CRM reference frontend).

## Context

The backend already exposes a model contract: entity, field, relationship, and
method metadata via `/admin/model`, plus the generated GraphQL schema.
Hand-authored frontend types and clients drift from the backend and are slow for
agents to maintain. The current CRM reference frontend carries a hand-written,
explicitly "interim" contract — the failure mode this ADR removes.

## Decision

The frontend-to-backend interface is generated from the backend model, not
hand-authored. `app_gen` emits, per product schema, a typed UI contract
(entities, fields, relationships, methods, pagination, and display hints) and a
typed GraphQL client. Frontends consume these generated artifacts and do not
hand-author backend types. Contract generation is drift-checked against the
schema the same way backend artifacts are.

The backend model is the single source of UI structure. Where the model lacks UI
metadata (widget, enum options, validation, display hints), the fix is to extend
the model, not to encode it in the frontend.

## Consequences

- A schema change regenerates the contract; the frontend build surfaces breakage
  immediately — this is the core test-validation loop (see ADR 0011).
- Agents receive typed entities and a client to build against, rather than a
  blank page.
- UI metadata becomes a backend responsibility, keeping one source of truth.
- The hand-maintained interim contract is retired.
