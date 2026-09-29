# PDS Health Design System

This directory is the framework-owned source home for PDS Health frontend
design assets used by App Framework product frontends, operational surfaces,
and product-neutral reference artifacts.

The design system is intentionally PDS Health branded. It is not a neutral
white-label theme. Product teams may add product-specific workflow composition,
copy, icons, and data visualizations, but they should inherit PDS typography,
color, spacing, focus, component behavior, and accessibility defaults from this
package.

## Ownership

| Surface | Owner | Notes |
| --- | --- | --- |
| `tokens/` | Framework | Canonical `--pds-*` CSS variables and typed token exports. |
| `components/` | Framework | Reusable component inventory, API rules, and maturity gates. |
| `reference/` | Framework | Canonical product-neutral PDS Component Catalog manifest and static fallback for the component API surface. |
| `catalog-app/` | Framework | Unified PDS Design System application. Its persistent navigation covers Overview, Brand, Floor plans, Elements, Components, Patterns, Data visualization, and retained reference proofs. |
| Product `frontend/` | Product | Product routes, workflow screens, copy, and product-specific tests. |
| `admin_ui/` | Framework operations UI | Consumer of PDS tokens and patterns, not a product UI template. |

## How To Read This Directory

`appfw_ui/pds_health` has four framework-owned layers:

1. `tokens/` defines the PDS Health brand language. This is where raw brand
   values become governed `--pds-*` variables and typed token exports.
2. `components/src/` defines the React component API. This is the source of
   truth for exported controls, work surfaces, form fields, data-grid shells,
   overlays, feedback states, navigation, process flows, and analytics chrome.
3. `reference/` is the PDS Component Catalog. `reference/catalog.json` is the
   agent-readable manifest for every exported component family, source API map,
   agent decision recipe, maturity level, evidence requirement, and product
   boundary.
   `reference/index.html` is the visible product-neutral catalog showing
   expected states, density and theme behavior, accessibility notes, source
   ownership, agent decision guidance, maturity, evidence, usage guidance, and
   verification expectations.
4. `catalog-app/` is the unified interactive PDS Design System application.
   The application root is the full design-system portal, not a standalone
   component-catalog renderer. Its persistent navigation exposes Overview,
   Brand, Floor plans, Elements, Components, Patterns, Data visualization, and
   retained reference proofs. The Components view renders the same
   `reference/catalog.json` manifest and real `components/src` exports through
   Vite/React so agents and product developers can search, inspect live
   examples, compare Apple-like and Material-like in System, Light, or Dark
   mode, and copy imports or usage snippets without copying product fixture
   language. The application is also the canonical visual exemplar for the
   ratified Precision Daylight, Chromatic Signal, and Connected Fabric
   languages; it does not define a parallel token or component contract.

The detailed visual-language contracts live beside the component source:

- `components/visual-themes.md`;
- `components/precision-daylight-visual-language.md`;
- `components/chromatic-signal-visual-language.md`; and
- `components/connected-fabric-visual-language.md`.

The accepted framework decision is recorded in
`docs/architecture/adr/0014-pds-signature-visual-language.md`.

`reference/catalog.json` remains the canonical machine-readable catalog
contract. The static catalog stays quick to open and deterministic for CI; the
interactive catalog is a governed renderer of that contract. CRM and admin UI
should prove the components in real workflows, but they are consumers, not the
catalog and not the design-system source of truth.

## Consumption Model

Current product and operational frontends may still vendor token files while
the package boundary is being wired. New framework work should treat this
directory as the source of truth and converge generated product scaffolds and
framework-owned operational surfaces on these assets.

Target import shape for generated or product-owned frontends:

```ts
import {
  pdsTokens,
  pdsTokenCssVars,
  pdsVisualThemes
} from "@appfw/pds-health/tokens";
import {
  Button,
  DataGridShell,
  PageHeader,
  pdsAgentDecisionGuide,
  pdsComponentCatalog
} from "@appfw/pds-health-components";
```

```css
@import "@appfw/pds-health/tokens/pdsTokens.css";
@import "@appfw/pds-health-components/styles.css";
```

Until package publishing exists, scaffolds may copy these files into
`frontend/src/design/` with provenance recorded in `.appfw-ui/scaffold-manifest.json`.

## Verification Expectations

Design-system changes should prove:

- docs and skill routing with `scripts/appfw framework docs-check --json`;
- token drift checks once consuming frontends sync from this source;
- component contract checks with `scripts/check-pds-components.mjs --json`;
- source export drift checks and the retained `component_api_inventory` in
  `target/appfw/pds-component-check.json`;
- package catalog API checks for `pdsComponentCatalog`,
  `pdsComponentFamilies`, and `pdsAgentDecisionGuide`;
- agent decision recipe checks and the retained `agent_decision_guide` in
  `target/appfw/pds-component-check.json`;
- static catalog and manifest checks for product-neutral component coverage;
- interactive catalog checks for live examples, usage snippets, real component
  source imports, manifest wiring, and product-neutral copy;
- interactive catalog visual/a11y evidence with
  `scripts/check-pds-catalog-evidence.mjs --json`, retained at
  `target/appfw/pds-catalog-evidence.json` with screenshots under
  `target/appfw/pds-catalog-evidence/screenshots/`;
- component typecheck/build/tests as consuming frontends adopt the shared
  source;
- product scaffold residue checks showing a new frontend has no product-domain
  terms; and
- representative frontend E2E/a11y checks when shared primitives change.

## Local Preview

Run the product-neutral component catalog without relying on a product
frontend:

```bash
node scripts/serve-pds-reference.mjs
```

Then open `http://127.0.0.1:5175/reference/`.

For the complete interactive design-system application, run:

```bash
cd appfw_ui/pds_health/catalog-app
npm run dev
```

Then open the root URL printed by Vite. The root must show the full
design-system portal and persistent view navigation; Components is one view
inside that application. The application is still governed by
`scripts/check-pds-components.mjs --json`; do not add product-specific nouns or
fixture routes to its examples.

For retained catalog evidence, run from the repository root:

```bash
scripts/check-pds-catalog-evidence.mjs --json
```

This builds the interactive catalog, serves the static output, runs Playwright
and axe against desktop/mobile light/dark scenarios, exercises and persists the
Apple-like/Material-like selector, checks layout overflow and interactive names,
verifies Conversation and Ambient AI component semantics, and captures
screenshots for CI review.
