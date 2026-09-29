# PDS Nexus Frontend

This frontend was created because product intake selected UI mode `enterprise`.

Use the PoC UI artifact for workflow intent and App Framework frontend
standards for implementation shape. Replace the placeholder screen after the
PoC-derived entity model and generated UI contract exist.

## Consumed PDS / IX packages

Nexus Web consumes the receipted B_IX-family archives, vendored in-tree:

- `@appfw/pds-health-components@0.12.0` (`file:vendor/appfw-pds-health-components-0.12.0.tgz`, sha256 `b5c655533e9b935df92dc348b05b1966226bb6532a323c37ccf77ea12272b9e0`)
- `@appfw/pds-ix-presentation-contract@0.2.0` (`file:vendor/appfw-pds-ix-presentation-contract-0.2.0.tgz`, sha256 `3a0a3404e2186e87302af019c1eca5dee5a80de9d10e98383f3fdcfcae884364`)

Do not install the held `0.9.0` archive, native `0.2.0`, or an `appfw_ui`
source alias. Visible versions on the reachable About surface at `/#about` and
in `.appfw-package-provenance.json` stay `0.12.0` + `0.2.0`. Prove the pins
with `npm run appfw:provenance`.

```bash
npm ci
npm run appfw:provenance
npm run appfw:check
npm run appfw:evidence
npm run appfw:ux-metrics
npm run typecheck
npm run build
```

`npm run build` emits the deployable SPA bundle to `../backend/product_dist`.
The generated backend serves that bundle at `/` in the default one-image
deployment topology when `APP_PRODUCT_UI_ENABLED=true`.

`npm run appfw:evidence` builds the static frontend, serves
`../backend/product_dist`, runs Playwright + axe across desktop/mobile
light/dark evidence scenarios, verifies the rendered chat, grounding,
FlowGraph, analytics, and preview-only write guardrail surfaces, and retains
screenshots plus
`target/appfw/nexus-visual-evidence.json`.

`npm run appfw:ux-metrics` consumes that visual evidence plus the retained
synthetic source-evidence files and writes `target/appfw/nexus-ux-metrics.json`.
It records only local baseline metrics: plans presented/opened/accepted,
confidence/calibration posture, grounding, tenant-sentinel exposure, a11y
totals, and preview-only write guardrails. It intentionally keeps
`release_ready:false` and `live_ready:false`.
