# Fabric Management Read Model

## Contract

The dormant runtime foundation projects a validated, in-memory Fabric registry
into two closed provider-neutral documents:

- `appfw.fabric.management_overview@1` summarizes the visible components,
  relationships, owners, evidence coverage, health, and availability; and
- `appfw.fabric.component_detail@1` provides one authorized component's
  provenance, typed evidence, health binding, and visible incoming/outgoing
  relationships.

Both projections use stable ordering and a SHA-256 digest over canonical
serialization. They do not infer truth from missing evidence or from a UI.

## Caller-owned authorization

This module authenticates nobody and makes no policy decision. An authorized
caller compiles a `FabricReadScope` containing permitted environments and data
classifications, then may narrow it with owner and component restrictions.
Repeated restrictions intersect monotonically. Empty, malformed, excessive,
or disjoint restrictions fail closed.

Filtering happens before projection. A relationship is visible only when both
endpoints are visible, so neither a hidden edge nor a hidden identifier leaks.
The detail lookup deliberately returns the same `NotAvailable` result for a
missing component and a component outside the caller's scope.

## Evidence and health truth

Evidence kinds and postures are closed enums. Overview coverage reports both
present and absent evidence kinds; absence never becomes an implicit pass.
Observed health must retain its verified health-evidence binding, and every
health/availability combination must satisfy the registry's closed coherence
matrix. Projection does not perform a live probe or upgrade declared evidence
to observed evidence.

## Explicit nonclaims

This foundation provides no HTTP or GraphQL route, persistence adapter,
identity provider, approval engine, live health integration, dashboard,
Product adoption, public compatibility commitment, release evidence, or risk
acceptance. The Framework-local fixture is sanitized test input only. Any
future consumer or adapter requires a separate reviewed source boundary.
