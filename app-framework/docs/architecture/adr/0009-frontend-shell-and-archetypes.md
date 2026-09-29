# ADR 0009: Frontend App Shell And Archetype Kits

## Status

Accepted (provisional — the CRUD/relational kit is validated by the CRM
reference frontend; the analytical and document kits remain provisional until OD
Equity and the Offer Letter exercise them).

## Context

Citizen-developer apps cluster into a few shapes: analytical reports,
personalized documents/letters, and CRUD/relational tools. Re-deciding shell
concerns — routing, layout, auth context, errors, loading and empty states — per
app is wasteful and inconsistent.

## Decision

The scaffold provides one standardized app shell and a small set of archetype
kits layered on top.

The shell owns: routing, layout and navigation, authentication and tenant
context, error boundaries, loading/empty/permission states, and correlation-id
propagation to the backend.

The archetype kits are:

- CRUD/relational — lists, detail, create/edit forms, relationship navigation.
  The foundational kit.
- Analytical report — KPI banners, charts, cascading selectors, period history.
- Document/workflow — templated personalized document, print/PDF, and a
  review/accept/sign flow.

Converting a vibe artifact is: classify its archetype, then map its content onto
the matching kit.

## Consequences

- Shell concerns are solved once; products differ only in screens and content.
- Archetype classification becomes an explicit, automatable conversion step.
- The CRM reference frontend exercises the CRUD kit first; the analytical and
  document kits are confirmed later by analytics/reporting and document workflows.
