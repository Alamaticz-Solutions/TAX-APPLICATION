# ADR 0011: Frontend Test Backbone And Validation

## Status

Accepted (provisional — first realized by the CRM reference frontend).

## Context

A scaffold is trustworthy only if it is continuously validated. The CRM
reference frontend provides a fixed, known target to validate against, which
makes it a standing regression harness, not merely an example.

## Decision

The frontend test backbone is part of the reusable scaffold; the CRM frontend is
its first fixture. It comprises:

- Typecheck — the generated contract must typecheck against the product schema.
- Contract-drift test — regenerate from the current backend model; the product
  frontend must still build and typecheck. This proves backend-to-frontend
  contract integrity and that the ownership boundary holds.
- Component tests — for the generated scaffolds and the shared kit.
- End-to-end tests — Playwright against a running backend.
- CI gates — build, typecheck, lint, test, and drift, as a peer of the backend
  release gate.

## Consequences

- A backend schema change that would break the UI is caught in CI, not in
  production.
- The scaffold and its code generation are guarded for the life of the
  framework.
- CRM is the canonical fixture; new archetypes add their own fixtures (analytics,
  Offer Letter).
- Frontend gates must be added to the pipeline, which is backend-only today.
