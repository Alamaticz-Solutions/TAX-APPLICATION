# ADR 0010: Frontend Security, Tenancy, And Data Governance

## Status

Accepted. The intent is non-negotiable for any real data; only the mechanism is
provisional.

## Context

This is where citizen-developer artifacts are weakest and enterprise risk is
highest. Vibe apps embed real sensitive data — doctor names, ownership
percentages, EBITDA, valuations, individual compensation — directly in a single
HTML file with no authentication, and that file gets emailed around. The pilot
deferred authentication and policy; the scaffold must not.

## Decision

Authentication and authorization are default-on in the shell: Okta/JWT identity,
tenant context, and UI role and permission gating derived from backend policy and
entity `standard_methods`. The UI reflects backend authorization; it never
re-implements or replaces it, and it fails closed on permission errors.

Data governance is a standing rule:

- Synthetic values only in source, fixtures, and tests. No PII, PHI, or financial
  values in the repository.
- Real data flows only through governed feeds and is constrained by backend
  row-level (Rego) policy, so a user sees only their own scope — for example, a
  doctor sees only their own data.

## Consequences

- The frontend is never the authorization authority; moving from "real financials
  embedded in an emailed HTML" to "governed feed plus row-level policy" is the
  step that turns a vibe app into an enterprise product.
- Demos run on synthetic data; wiring real data is a deliberate, governed, gated
  step, not a default.
- CI must guard against PII/PHI/financial values entering frontend source — a
  peer of the backend PHI-log lint.
