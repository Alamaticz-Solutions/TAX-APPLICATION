# Flow Graph Token Bridge

Status: Wave 4 CH5 local evidence contract. This is not a live chat runtime or
release-grade React Flow adapter.

## Decision

`FlowGraphShell` owns the PDS-to-React-Flow token bridge. Product code may supply
a renderer through the `renderGraph` slot, and the framework may later provide an
`@xyflow/react` adapter, but neither path may expose a raw third-party visual
surface without the PDS shell.

The bridge is implemented as CSS custom properties on
`.pds-flow-graph-shell__viewport`. React Flow reads `--xy-*` variables from its
ancestor tree, so renderer code mounted inside the viewport inherits PDS colors,
spacing, focus, border, and shadow intent without duplicating PDS tokens in
product code.

## Required Token Mappings

The viewport must define, at minimum:

- `--xy-background-color-default`
- `--xy-node-background-color-default`
- `--xy-node-color-default`
- `--xy-node-border-default`
- `--xy-node-border-radius-default`
- `--xy-node-boxshadow-default`
- `--xy-edge-stroke-default`
- `--xy-edge-stroke-width-default`
- `--xy-connectionline-stroke-default`
- `--xy-controls-button-background-color-default`
- `--xy-controls-button-color-default`
- `--xy-minimap-background-color-default`
- `--xy-attribution-background-color-default`

Each value must resolve from PDS design tokens (`var(--pds-...)`) or a
PDS-token-derived CSS expression. Literal brand colors in renderer code are not
the framework contract.

## Guardrails

- Keep `@xyflow/react` optional and behind `FlowGraphShell`.
- Do not require generated products to import `@xyflow/react` directly.
- Keep the ordered-list fallback accessible and renderer-neutral.
- Preserve keyboard and screen-reader evidence for graph nodes before any
  release-gated claim.
- Do not mark the FlowGraph adapter live-ready until product-level visual/a11y
  evidence and live chat/search/gateway evidence exist.

## Evidence

`scripts/check-pds-components.mjs --json` retains `flow_graph_token_bridge` in
`target/appfw/pds-component-check.json`. That section is local component-family
evidence only. It proves the PDS shell publishes the token bridge and keeps
`adapter_ready:false` until a real adapter and live product evidence exist.
