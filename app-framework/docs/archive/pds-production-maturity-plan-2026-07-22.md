# PDS Production Maturity Plan And Status — 2026-07-22

| | |
| --- | --- |
| **Type** | Dated point-in-time program capture (plan, accomplishments, remaining work, decisions). Captured 2026-07-22; amended pre-publication 2026-07-24 with merge statuses and the experience-telemetry candidate. |
| **Owner** | Wayne Kempf (Product Owner); executed with agent assistance |
| **Sequencing authority** | This document records intent and status. Execution order is owned by [`docs/release/roadmap.md`](../release/roadmap.md) and the [dimension spec](../specs/pds-experience-dimensions-and-catalog-plan.md); where they disagree, they win. |
| **Governing contracts** | [ADR 0014](../architecture/adr/0014-pds-signature-visual-language.md), [ADR 0015](../architecture/adr/0015-pds-experience-system-dimensions.md), [ADR 0016](../architecture/adr/0016-pds-web-interaction-substrate.md), [Nexus readiness](../specs/nexus-experience-system-readiness.md), [UX design strategy](../frontend/ux-design-strategy.md), [mobile contract](../frontend/mobile-react-native.md) |

## Purpose

Take the PDS experience system to production-grade maturity for product
development: a product team or agent composes a world-class product from
`@appfw/pds-health-components` alone — responsive pointer **and** touch web,
plus React Native/Expo native mobile with exact task continuation — with every
capability evidence-backed, idiom-pure per visual grammar, and native-smooth in
motion.

## Background

### Where maturity actually stood (verified 2026-07-22)

A deep source sweep established the honest baseline:

- **Web desktop is mature and shipping**: compiled ESM package (`0.6.0` on
  `main`; a `0.7.0` candidate is staged on the unmerged ws01 lane),
  React Aria Components as the commodity interaction substrate behind PDS APIs
  (ADR 0016; products never import RAC directly), two ratified visual grammars
  (apple-like, material-like) × three color modes, density implemented,
  `PdsExperienceProvider` + `appfw_product_experience_profile@1`.
- **Mobile web was thin**: only three CSS width breakpoints (900/860/560) plus
  one data-grid container query — column stacking, not mobile design. No
  `pointer: coarse`/`hover: none` handling, no touch-target minimums, no
  `safe-area-inset`, no dynamic viewport units, no drawer/bottom navigation.
  Catalog "mobile" evidence was width-only (390×844), without touch emulation.
- **Platform scale (`pointer | touch`) was ratified but entirely unbuilt**:
  ADR 0015 and the dimension spec define `data-scale`, a ~1:1.25 ratio, WCAG
  2.2 target-size floors, and a 16-state evidence matrix — none implemented.
  The single biggest intent-versus-code gap.
- **Native mobile is a deliberate separate track**: React Native + Expo is the
  ratified primary target (PWA is fallback). `product generate --target
  mobile-rn` emits a generated mobile contract, PDS native token bridge,
  entity screen, and Expo Router shells, with a CRM reference app — all
  roadmap-gated, `release_ready:false`, sharing tokens/model/contracts and
  never web DOM/CSS. Readiness slice N3 ("native continuation") requires one
  channel-neutral task/evidence/permission/resume contract.

### Product Owner direction (this cycle)

Beyond closing those gaps, the Product Owner set explicit quality and scope
targets: world-class craft (native-smooth animation, correct spacing, no
overflows, no mixing of Apple/Material idioms), completion of the RAC
migration beyond the Tabs/Dialog proof, and missing capabilities — workflow
and taskflow components, a calendar planner, map types, and a catalog IA that
gives data visualizations and data tables (including the AG Grid tier) their
own mature sections.

### The brand-guide discovery and the W0 decisions

The full six-page PDS Health brand guide (093025 export) became available on
2026-07-22, superseding the earlier one-page capture. It specifies the
four-color identity palette (page 4) and brand typography (page 5): **Print =
Gotham** (commercial; print-scoped), **Web = Poppins**. Page 4 is internally
inconsistent for Health Light Gray (hex text `#F3F6F8` vs RGB 244/247/248 =
`#F4F7F8`).

The Product Owner decided (recorded in the dimension spec's Decision
Provenance):

1. **Adopt the official brand palette as the color foundation** — canonical
   Health Light Gray follows the guide's RGB (`#F4F7F8`); the inconsistency is
   flagged to the brand team.
2. **Two-tier brand typography** — Poppins Bold for display/headline roles
   (the brand's web face), Inter for UI body/data (metric/KPI numerals stay
   Inter for `tabular-nums`), Geist Mono route-scoped. Payloads are
   publisher-built latin subsets, self-hosted, no CDN.

Licenses were verified against upstream texts: Poppins, Inter, and Geist Mono
are all SIL OFL 1.1 with **no Reserved Font Names** — commercial use,
self-hosting, bundling, and subsetting under original names are permitted; the
only obligations are shipping each OFL text with the binaries and never
distributing fonts standalone.

## Accomplished

### Landed on `main` before this capture (context)

- Foundation and Nexus experience surfaces; F1 interaction proof plus N0
  (My Work) and N1 (trusted task) browser evidence (PR #434).
- Governed **AG Grid adapter** over pinned **AG Grid Community 36.0.1** —
  resolving the license-tier question at zero cost (PR #435).
- Inter and Geist Mono payloads vendored with OFL texts and hash provenance.

### Slice W0 — brand palette + two-tier typography

Branch `feature/pds-w0-brand-typography-decision` @ `548d96d6e` — **merged to
main (PR #439 lineage)**. Contents: vendored publisher-built Poppins Bold latin subset
(7,848 bytes, sha256-recorded, ~2% of the existing 424 KB font payload) with
verbatim OFL text; `--pds-font-family-display` / `--pds-font-display` /
`--pds-font-weight-display` tokens in CSS and the typed TS mirror; two
Decision Provenance rows; the typography draft spec marked historical;
CHANGELOG entry. No component was restyled (display-role wiring graduates
separately with layout-shift evidence). All gates green (token drift,
component check, docs-check, browser/axe catalog evidence).

Independent Framework PR Review: **GO WITH CONDITIONS** (0 blockers, 0
critical). Condition outcomes as of 2026-07-24: (1) the CRM Chart.js Poppins
request was **resolved by mitigation on main** — `DashboardWidgets.tsx` now
uses the Inter stack, consistent with the two-tier rule (charts are data
surfaces, not display roles); (2) the stale fonts-README byte total was
**closed by W1a**; (3) the `@appfw/pds-health-brand` display-contract
alignment on the ws01 lane **remains open**.

### Slice W1a — canonical DTCG token source

Branch `feature/pds-w1a-dtcg-token-source` @ `509accd60`, advanced to
`e619b88f2` and **merged to main** (the lane's follow-up commits closed the
signposting conditions: generated-ownership routing, token-ownership
contract alignment, honest source-of-truth wording). `appfw_ui/pds_health/tokens/tokens.dtcg.json` is now the
canonical token source; `pdsTokens.css`/`.ts` are generated by
`scripts/generate-pds-tokens.mjs` and proven **byte-for-byte identical** to
the previous hand-maintained files (pristine bootstrap → regenerate →
byte-identical; 108 root + 195 material-like tokens). `scripts/check-pds-tokens.mjs`
gained a `generated_source_sync` gate that fails on drift
(negative-tested). The migration deliberately preserved two pre-existing
CSS/TS value divergences verbatim (`color.state.gold`, `motion.fluid`) for
later reconciliation rather than silently changing behavior; the W0
fonts-README byte-total condition was closed (431,684 bytes).

Independent Framework PR Review: **GO WITH CONDITIONS** (0 blockers, 0
critical; 2 important). Most important finding at review time: the
edit-routing signposts lagged the migration — `docs/start/generated-ownership.md`
lacked entries for the generated token files, `docs/frontend/product-frontend.md`
and the generated CSS header still called `pdsTokens.css` the "source of
truth", and the JSON's `$description` claimed DTCG `$type`/`$value` groups the
file did not contain. **Outcome: all of these were fixed on the lane before
merge and are verified correct on `main` as of 2026-07-24** (ownership
routing present; wording and `$description` honest); the drift gate would
have caught wrong-surface edits in the interim.

## Remaining work

Ordered slices; each ships with the standard gates plus independent Framework
PR Review, per the pre-push policy.

| # | Slice | Content | Exit evidence |
| --- | --- | --- | --- |
| 1 | **Display-role wiring** | `PageHeader` titles / floor-plan heroes / display roles consume `--pds-font-family-display` | Layout-shift measurement during font load; visual + axe evidence |
| 2 | **W1b platform scale** | `data-scale` (`pointer\|touch`, ~1:1.25) tokens + auto-detection + override; `appfw_product_experience_profile@2` with scale; window-size-class and motion-scheme tokens | Token + contract checks; profile validation |
| 3 | **W1c touch ergonomics** | WCAG 2.2 target-size floors at touch scale; `pointer: coarse`/`hover: none` behavior; `safe-area-inset`; `dvh`; navigation morphing groundwork | Catalog evidence at touch scale |
| 4 | **W1d evidence upgrade** | Real touch emulation in the catalog harness; automated 16-state matrix (4 signatures × 2 scales × 2 densities) | Evidence artifacts retained per state |
| 5 | **W1e color remodel** | OKLCH conversion, reference tier, tenant brand-ramp generator with contrast floors; reconcile the two captured CSS/TS divergences; retire hex | Round-trip + contrast-floor proofs |
| 6 | **M2 substrate + idiom purity** | Finish RAC migration by Nexus slice (W4 collections: virtualized grid/list keyboard/SR acceptance); W2 grammar × mode parity with a **token-signature diff gate** so apple/material idiom leaks fail CI | Identical component/axe results across all four theme signatures |
| 7 | **M3 capability expansion** | Catalog IA split (**Data Tables** incl. AG Grid tier; **Data Visualization** grown beyond four charts); **Workflow & Tasks** family graduated from N0/N1 (queue, task card, approval with consequence preview, SLA/status/timeline, receipt, board); **Calendar planner** (month/week/day/agenda on RAC calendar primitives); **Maps** (MapShell + SVG region status first; tile engine behind a PDS adapter after engine review); W3 floor plans + navigation morphing (bottom bar → rail → drawer) | Per-family catalog, lifecycle, a11y, consumer evidence |
| 8 | **M4 motion (W5)** | Standard scheme rollout (120/200/300ms, transform+opacity only; command palette motion-free); expressive scheme only at signature moments; reduced-motion parity | Repeated-work timing proof (motion never slows the tenth repetition) |
| 9 | **M5 native (RN/Expo)** | Freeze the channel-neutral work contract (`feature/ws01-pds-work-contract-package` in flight); generate native design data from the DTCG source (extends `pdsNativeTokens` bridge); RN kit for the P0 set (nav shell, list/detail, forms, task/approval, notifications); N3 continuation proof (push/deep link/resume/offline); device/simulator + store-track evidence → `release_ready:true` | N3 slice evidence; iOS defaults apple-like, Android material-like |
| 10 | **N4 certification exit** | Disposable package-only consumer + one genuine upgrade + Core Web Vitals budgets (INP ≤ 200ms p75) + full 16-state visual matrix + signature-quality review | The production-grade claim; first product activation remains a separate explicit human decision |

## Experience scoring and telemetry (candidate addition — not yet sequenced)

Prompted by a 2026-07-24 review of application-performance and satisfaction
scoring frameworks (Apdex succession, Digital Experience Monitoring, Core Web
Vitals, DEX scores, OpenTelemetry-native and LLM observability). Provenance
note: the external research was AI-generated from vendor sources with a
projected time frame; its direction is treated as corroboration only — the
same bets already exist independently in ratified PDS gates (INP ≤ 200ms p75
and layout-shift budgets in the Nexus readiness exit gates; UX Law 1; the
zero-layout-shift control invariant; the OTLP-oriented runtime with
`pds-observability-trace` as a named evidence dimension; conversation-family
primitives for streaming latency and confidence).

Apdex itself is not worth resurrecting. Three bounded pieces are worth
building; the rest of the DEM landscape is deliberately out of scope.

1. **A RUM field-data contract, not a UX feature.** Standardize how products
   emit LCP/INP/CLS plus PDS operation/request/correlation IDs through the
   existing PDS observability hooks (OTel semantic conventions, PHI-free by
   construction, tenant-scoped). The design system owns the contract names;
   runtime/products own emission. The self-serving kicker: this is exactly
   what certification needs — the June audit's biggest gap (F-2) was "no
   live/field evidence"; a RUM contract turns lab INP budgets into field
   proof for the N4 exit.
2. **An "Experience Health" floor-plan composition.** An ops/admin-facing
   dashboard built almost entirely from what exists (`KpiTile` for p75
   INP/LCP/CLS and TTFT, `MetricTrend`, chart shells), plus one small
   addition: a web-vital threshold semantic mapped to Chromatic Signal tones
   (good/needs-improvement/poor → success/warning/danger, with non-color
   redundancy). Mostly a recipe, not new components.
3. **A `SentimentIntercept` component.** The one genuinely new UX capability
   worth taking from the DEX playbook: a governed micro-survey affordance
   (rating plus optional comment), anomaly- or milestone-triggered, never
   work-blocking (Law 1), hard frequency caps, visibly dismissible, with
   product-owned storage and policy. It also feeds PDS's own gates — N4's
   "task success / comparative preference" evidence currently has no
   collection mechanism.

Deliberately skipped: no DEX score engine or device-health telemetry
(endpoint-agent territory — buy, not build, per the fabric strategy); no
composite experience score in end-user chrome (operator surfaces only — a
visible score reads as surveillance and violates the "personalization without
feeling watched" direction); no Apdex-style T-thresholds (the CWV budgets are
the sharper form).

## Foreseen challenges

- **ws01 token collision (biggest merge risk).** The ws01 checkpoint worktree
  (`maintenance/ws01-current-ux-preservation`) holds a large uncommitted set
  including edits to `pdsTokens.css`/`.ts` — files that are now generated
  output. Rule needed with the ws01 lane owner: either ws01's token changes
  land first and are captured into `tokens.dtcg.json` (the
  `--bootstrap-from-files` escape hatch exists for exactly this), or ws01
  rebases and expresses its token changes in the JSON. Hand edits to the
  generated files now fail the token gate by design.
- **Generator ergonomics.** The byte-identity approach captures the
  hand-authored files' order/comments/wraps as render metadata. Adding a token
  means editing the JSON (order lists included); the W1e remodel may replace
  render metadata with plain DTCG group emission once byte-compat with the
  legacy layout no longer matters. The `--bootstrap-from-files` flag is a
  canonical-inversion risk and must remain a documented, review-only escape
  hatch.
- **Evidence cost at 16 states.** The catalog evidence matrix roughly doubles
  per new dimension; W1d must automate it (no manual screenshot debt) and keep
  runtimes tolerable.
- **Idiom purity is a discipline problem.** The token-signature diff gate (M2)
  exists because grammar leaks (state layers in apple-like, glass off-chrome)
  keep re-entering through individually reasonable changes.

## Decisions needed (Product Owner)

| Decision | Context | Gates |
| --- | --- | --- |
| ws01 lane coordination rule for token files | Biggest merge risk (above) | Before ws01 integration |
| Map engine + tile hosting | MapLibre GL recommended; self-hosted tiles vs provider is a PHI/data-sovereignty call | M3 maps slice |
| Calendar planner v1 scope | Personal planner (recommended) vs full resource/room scheduler (adapter territory) | M3 scheduling slice |
| Advanced data-viz engine posture | Stay zero-dependency SVG vs adopt one engine behind `ChartShell` for complex interactivity | M3 data-viz slice |
| Catalog IA approval | Elevating Data Tables and Data Visualization to top-level sections is a versioned manifest/checker change | M3 IA slice |
| Inter subset | Poppins proves the publisher-subset strategy at 7.8 KB; Inter still ships 352 KB variable. Subset later? | W1e or later; measured, license-clean either way |
| RUM field-data contract approval | PHI-free, tenant-scoped web-vitals emission through the PDS observability spine; requires observability/security governance review before any product emits | Experience-telemetry slice; feeds N4 field evidence |
| `SentimentIntercept` timing | Build alongside Experience Health, or wait for the first live product (most valuable with real users to instrument) | Experience-telemetry slice scope |
| ADR 0015 byte figure | Accepted-ADR text retains the pre-Poppins payload total as a point-in-time record (deliberate; current truth lives in `tokens/fonts/README.md`) | Record-only unless the PO wants an ADR amendment |

## Evidence index

- Merged: `feature/pds-w0-brand-typography-decision` (`548d96d6e`, PR #439)
  and `feature/pds-w1a-dtcg-token-source` (`509accd60` → `e619b88f2`), both on
  `main` as of 2026-07-24.
- Retained review outputs: `target/appfw/framework-pr-review.md` per branch
  worktree at review time; verdicts recorded above.
- Gate artifacts per slice: `target/appfw/pds-token-drift.json`,
  `target/appfw/pds-component-check.json` (carries the known pre-existing
  Wave-2 `governed-action-live-readiness` failure, unrelated to these slices),
  `target/appfw/pds-catalog-evidence.json`.
- Canonical records: dimension-spec Decision Provenance rows dated 2026-07-22;
  `appfw_ui/pds_health/tokens/fonts/README.md` (payload provenance + hashes);
  `appfw_ui/pds_health/CHANGELOG.md` `[Unreleased]`.
