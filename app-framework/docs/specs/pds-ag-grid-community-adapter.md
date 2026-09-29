# PDS AG Grid Community Adapter

Status: implementation deliverable

## Outcome

Provide one high-capability data grid behind the public PDS component contract
for dense operational work. Products consume `DataGrid`; they do not import AG
Grid packages, classes, themes, modules, or configuration objects directly.

## Boundaries

- Use AG Grid Community only. Enterprise modules, license keys, and enterprise
  feature claims are prohibited.
- Keep `DataGridShell` as the lightweight semantic table for small collections.
- Use the AG Grid Theming API. Legacy AG theme CSS classes are prohibited.
- Resolve Apple-like and Material-like presentation from the same PDS column,
  row, selection, density, state, and event semantics.
- PDS semantic tokens remain authoritative for color, typography, spacing,
  shape, focus, selection, and light/dark adaptation.
- Register only the required Community modules inside the PDS adapter. Product
  code never performs module registration.
- Pin the MIT-licensed `ag-grid-community` and `ag-grid-react` packages to the
  same reviewed version. Keep the PDS adapter as the exit boundary.
- Treat bundle weight as a product-consumption decision. Before product use,
  the high-capability adapter must have a proven lazy grid-surface boundary;
  a product entry that does not request `DataGrid` must contain zero AG Grid
  bytes. The catalog intentionally includes the full component inventory and
  does not prove that product boundary. Products with small collections retain
  the lightweight `DataGridShell`.
- Use **450 KiB gzip** as the provisional engineering stop for aggregate
  asynchronous grid-only assets. This is a stop threshold, not a standing
  invitation to consume the full budget; crossing it requires a separate
  package or loading-boundary decision.

## Initial Capability

The first deliverable must prove sorting, Community column filtering, resizing,
single and multiple selection, quick filtering, pagination, density, loading,
empty state, keyboard navigation, stable row identity, and product callbacks.
It must render in Apple-like and Material-like visual grammars in light and dark
mode without a theme fork.

## Evidence

- component and catalog source checks;
- catalog typecheck and production build;
- focused browser interaction and accessibility checks from
  `node scripts/tests/pds-data-grid.browser.mjs` for both visual grammars,
  color modes, and mobile containment;
- retained screenshots for desktop and a bounded responsive viewport;
- before product adoption, disposable no-grid and dynamically loaded grid
  production builds recording
  raw, minified, and gzip bytes, asset hashes, tool versions, zero initial-route
  AG Grid bytes, and the 450 KiB gzip stop;
- dependency evidence showing only `ag-grid-community` and `ag-grid-react` at
  the same exact version.

## Non-claims

This deliverable does not certify the product lazy-loading boundary, AG Grid
Enterprise, server-side row models, Excel export, pivoting, managed production
scale, or Nexus product readiness. Those
require named product demand and separate evidence.
