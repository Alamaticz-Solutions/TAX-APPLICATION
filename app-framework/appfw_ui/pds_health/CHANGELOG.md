# Changelog

All notable changes to the PDS Health design system
(`@appfw/pds-health-components` and its `tokens/`) are documented here.

The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the package
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html). The
versioning, lifecycle, and deprecation policy is defined in
[`docs/frontend/pds-health-design-system.md`](../../docs/frontend/pds-health-design-system.md).

## [Unreleased]

- Kept the `0.12.0` IX Web APIs on the explicit
  `intelligence-presentation`, `intelligence-presentation-model`, and
  `ix-recipes` subpaths. Core package installation treats the channel-neutral
  `0.2.0` contract as an optional peer; importing an IX subpath requires that
  exact peer.
- Enforced unique response-region identities, revision-local changed-region
  references, collision-safe DOM identifiers, and one renderer-owned
  announcement per presentation revision. Standalone context and work-status
  primitives remain independently announceable.
- Scoped the catalog notice row to the IX reference so existing reference
  frames retain their one-row usable height, and made the clean three-archive
  contract/Web/native proof populate a fresh cache before its offline phase.
- Native iOS and Android remain `not-qualified`; these package checks are not
  device, product-adoption, distribution, or release evidence.

## [0.12.0] - 2026-08-12

- Added the public `ix-recipes` Web composition and resolver for the closed
  eight-recipe `pds.ix.recipe_registry@1` contract, while preserving
  product-owned data, language, policy, actions, and journey composition.
- Added an optional editable-region action slot to `ProgressiveResponse` so
  callers can expose human correction without manufacturing a domain action
  label or changing existing action eligibility.
- Advanced the channel-neutral IX contract to `0.2.0` with exact recipe,
  intent, renderer, capability, and projection metadata. Native projection
  support remains explicitly `not-qualified` pending exact-platform evidence.

## [0.11.0] - 2026-08-11

- Moved `pds.ix.presentation@1` authority into the standalone, React-free
  `@appfw/pds-ix-presentation-contract` artifact with a closed JSON Schema,
  runtime validator, sanitized golden fixture, and package proof. The web
  component package now adapts and re-exports that canonical contract.
- Added deterministic `pds.native.design-data@1` Apple-like light/dark token
  projection generated from `tokens.dtcg.json`, with a visual-theme selector
  that can add later grammars without changing the schema. Native iOS and
  Android remain explicitly not-qualified, and Android does not yet claim a
  Material-like implementation.
- Added the first PDS-owned React Native presentation renderers with real
  disclosure state, optional action/source callbacks, Dynamic Type, bounded
  touch targets, deterministic malformed-data fallback, and revision
  announcements. This does not claim a product mobile app, Expo shell, device
  evidence, navigation, authentication, or IX runtime authority.

## [0.10.0] - 2026-08-11

### Added
- Added the experimental `pds.ix.presentation@1` React-free, JSON-serializable
  presentation envelope and the `EvidenceDisclosure`,
  `ResolvedContextDisclosure`, `WorkStatus`, and `ProgressiveResponse` web
  renderers. The envelope preserves stable presentation identity, source
  references, context gaps, region status, and accessibility announcements;
  products and runtimes retain lifecycle, authority, provider, and action
  semantics.

## [0.9.0] - 2026-08-01


### Changed
- Conversation user turns render as compact right-aligned bubbles
  (fit-content, 85% max width, accent-soft fill); assistant turns keep the
  full evidence-bearing column.

- Modernized the apple-like grammar's chrome idiom (prototype pending design
  review): new tokens for glass chrome surfaces, hairline borders, recessed
  tracks, raised-thumb shadows, a specular top-edge highlight, and control
  (10px) / panel (14px) radii, with material-like overrides mapping each to
  its M3 equivalent so the material grammar is unchanged. The segmented
  control moves from a bordered group with accent-blue selected text to a
  recessed tinted track with a raised ink-labelled thumb; buttons, popover
  triggers, and the command palette trigger drop heavy outlines for hairline
  borders and larger radii; the narrative workspace sticky header becomes a
  glass surface; selected/hover states use ink instead of link-blue, keeping
  blue reserved for actions and signals.
- Tuned the glass and motion layer to verified research consensus
  (`docs/assessments/liquid-glass-web-research-2026-08-01.md`): regular-glass
  fill alphas that carry their own text contrast (0.72 light / 0.68 dark),
  `blur(16px) saturate(1.8)`, a dual specular rim (bright top inset, faint
  bottom counter-light), an opaque `--pds-color-surface-glass-opaque`
  fallback wired to `@supports not (backdrop-filter…)`,
  `prefers-reduced-transparency`, and `prefers-contrast: more`, plus a
  `--pds-motion-easing-spring` micro-interaction curve (press compliance on
  segmented options and triggers, hover lift on buttons) that material-like
  maps back to the M3 standard curve and reduced-motion disables entirely.
- Flattened commodity chrome per the flat-first direction refinement:
  accent-glow rings and gradient hover borders removed from buttons, menu
  triggers/items, menu panels, toasts, the data-grid toolbar, and the select
  picker (flat accent-soft fills, thin accent borders, soft elevation
  shadows instead); glow/gradient energy remains only on intelligence
  surfaces per ADR 0014.
- Flattened form controls: `--pds-gradient-control` is now a flat elevated
  fill (removing the "bubble" vertical gradient from ~30 control surfaces at
  the token layer), inputs/selects/text fields/input groups/file upload move
  to hairline borders and the control radius, and valid-state focus adopts
  the same flat ring idiom as invalid state (accent border +
  `0 0 0 3px` accent-soft ring) instead of gradient borders with glow.

### Fixed
- Popover-hosted outlined fields reserve clearance for their floating
  labels so scroll-clipped popovers cannot cut the label off (with a
  contract test).

- `SelectField` dropdown panels are now token-styled surfaces in browsers
  that support the customizable select (`appearance: base-select` +
  `::picker(select)`): the panel follows the PDS panel idiom in both visual
  themes and observes `light-dark()` color modes instead of the OS-drawn
  popup, which could not be themed and ignored the app's forced color mode
  on some platforms. Unsupported browsers keep the native popup unchanged;
  reduced-motion preferences disable the picker's entry transition.
- The catalog's live `AppearanceProvider` example now scopes its
  `data-theme`/`data-visual-theme` writes to its own subtree via
  `attributeTarget`, so opening the Components view no longer overrides the
  catalog's selected appearance or desynchronizes the style/mode controls.

### Added

- Added the experimental `NarrativeWorkspace` document-scrolling floor plan
  with one banner, labelled horizontal experience dock, main landmark, skip
  link, optional utility/context/footer regions, measured sticky offset,
  responsive context reflow, forced-colors fallback, and print behavior. The
  PDS Technology Strategy application is the proving consumer; products retain
  route or mode semantics, labels, context meaning, anchors, history, evidence,
  telemetry, and focus restoration.
- Added the experimental `ExplorationWorkspace` navigation contract with
  labelled navigator, focus, inspector, controls, and evidence regions, a
  bounded desktop floor plan, and accessible narrow/print reflow. Products
  continue to own object semantics, access, selection, relationships, proof,
  filtering, and URL state.
- Promoted the relationship atlas as an experimental Analytics contract. The
  package now exposes `RelationshipAtlas`, its equivalent accessible table,
  and a responsive `RelationshipExplorer` composition with shared selection,
  optional bounded table pagination, explicit invalid-reference signaling,
  token-only styles, and a public `relationship-atlas` subpath. Products
  continue to own graph semantics, evidence, citations, and relationship
  derivation.
- Made `tokens.dtcg.json` the canonical design-token source (W1a). The
  generated `pdsTokens.css` and `pdsTokens.ts` are rendered byte-identically
  by `scripts/generate-pds-tokens.mjs`, and `scripts/check-pds-tokens.mjs`
  now fails on any drift between the canonical source and the generated
  files. Two pre-existing CSS/TS value divergences (`color.state.gold`,
  `motion.fluid`) are captured verbatim for deliberate reconciliation in a
  later W1 slice.
- Adopted the official PDS Health brand palette as the color foundation and a
  two-tier brand typography contract (W0 decision, 2026-07-22): Poppins Bold
  carries display/headline roles per the brand guide's web typography; Inter
  remains the UI body/data workhorse; Geist Mono stays route-scoped. Adds
  `--pds-font-family-display`, `--pds-font-display`, and
  `--pds-font-weight-display` tokens plus the vendored publisher-built
  Poppins Bold latin subset (7,848 bytes, Google Fonts v24) with its OFL 1.1
  license text and hash provenance. All three bundled faces verified OFL 1.1
  with no Reserved Font Names. Display-role component wiring ships separately
  with layout-shift evidence.
- Added the beta PDS `DataGrid` adapter over pinned AG Grid Community 36.0.1,
  with PDS-owned columns, selection, density, loading, empty-state, keyboard,
  Apple-like/Material-like, and light/dark contracts. Product code remains
  prohibited from importing AG Grid directly.
- Protected the Product Owner-accepted Connected Fabric behavior as versioned
  baseline `connected-fabric/1.0.0`, with static source-contract checks and
  retained browser evidence for decorative semantics, reduced motion,
  pointer response, routed connections, and exclusive visible edges.
- Added the Nexus experience-system readiness manifest and layered readiness
  contract. Nexus readiness now depends on a package-only My Work journey and
  retained behavior, accessibility, performance, observability, native, and
  signature evidence rather than component count.
- Adopted React Aria Components as the commodity web interaction substrate
  behind PDS-owned APIs. The first bounded proof migrates `Tabs` and `Dialog`
  mechanics while retaining PDS semantics and visual grammar, with browser
  evidence for arrow-key selection, focus containment, Escape dismissal, and
  focus restoration.

### Changed

- Added an explicit `Surface` overflow policy: content remains clipped by
  default, while `overflow="visible"` is the supported opt-in for surfaces
  that intentionally host anchored PDS overlays.
- Ratified ADR 0015 and stopped treating App Framework as the owner of a
  general-purpose component engine. PDS continues to own public contracts,
  canonical design data, visual grammar, work patterns, intelligence,
  signature experience, and release evidence.

## [0.8.0] - 2026-07-28

### Added

- Added an accessible adjustable-sidebar contract to `AppShell`, with
  controlled and uncontrolled widths, product-owned persistence, bounded
  pointer resizing, keyboard `ArrowLeft`/`ArrowRight`/`Home`/`End` operation,
  reset on double click, and window-splitter value semantics.
- Added overflow-aware navigation disclosure. `NavigationItem` measures its
  label with `ResizeObserver` and provides a portal-rendered tooltip on hover
  or focus only when the complete two-line label is genuinely clipped.
- Added semantic navigation typography tokens for a consistent 14 px label,
  20 px line height, and medium weight across Apple-like and Material-like
  themes.

### Changed

- Expanded `NavigationItem` now wraps labels to two lines by default while
  preserving fixed icon, flexible label, and non-shrinking trailing columns.
  Intentional one-line truncation remains available through
  `labelBehavior="truncate"`.
- Responsive `AppShell` collapse now occurs at 960 px, before a readable
  navigation rail or the main product workspace becomes unusable.
- Icon artwork is normalized inside one fixed shared slot so navigation labels
  keep a consistent horizontal start and icons remain vertically centered.

## [0.7.2] - 2026-07-28

### Added

- Added the `ProcessStepper` `milestone` variant for compact workflow
  timelines with shared status glyphs, marker-centered connectors, and
  full-step native activation.

### Changed

- Milestone selection now uses persistent marker and label emphasis instead of
  a card-like selection surface, keeping current progress and user-selected
  filtering visually distinct.

## [0.7.1] - 2026-07-28

### Added

- Added an opt-in compact segmented presentation for `ProcessStepper`, backed
  by `ProcessProgress` and container-query-owned responsive behavior.

### Fixed

- Preserved the full labeled process structure for assistive technology while
  preventing narrow product containers from expanding into a loose vertical
  workflow list.
- Decoupled component introduction metadata from the current package version
  so patch releases do not rewrite lifecycle history.

## [0.7.0] - 2026-07-27

### Added

- Added a compiled ESM package with declarations, token-backed styles, the
  governed Poppins display font, and explicit family subpaths for every public
  PDS component family.
- Added archive-integrity checks and a detached package-only consumer proof so
  products can verify runtime and declaration imports without framework source
  aliases or checkout-relative imports.
- Added the stable `Breadcrumbs` navigation contract with current-page
  semantics and product-owned route handling.
- Added the stable `ButtonLink`, `InteractiveCard`, `CardLink`, and
  `WorkQueueItem` contracts for native link-versus-command semantics,
  full-surface activation, selected/completed/disabled states, and sibling
  secondary actions without nested interactive controls.
- Added beta `AppearanceProvider`, `NavigationItem`, `IconSlot`, `Avatar`, and
  `IdentitySummary` foundation contracts for shared appearance propagation,
  native navigation activation, fixed icon geometry, and compact identity
  composition. Products retain route behavior, identity data, role
  visibility, and persistence policy.
- Added the beta `ConversationWorkspace` composition with responsive named
  conversation/context/action regions, prompt grouping, streaming and busy
  states, and disabled composer behavior while the product-owned runtime is
  busy.
- Added the beta `TimelineRangeSelector` contract for a single movable and
  edge-resizable historical window with snap widths, pointer and keyboard
  operation, density cues, and explicit range labels.
- Promoted the protected `ConnectedFabric` visual-language implementation into
  the compiled component package with a focused `connected-fabric` entrypoint,
  keeping catalog and Nexus consumers on one canonical implementation.

### Changed

- Product frontends now consume the exact compiled archive through public
  family entrypoints. Direct imports from `components/src`, React Aria, and AG
  Grid are rejected at the Nexus product boundary.
- `ProcessStepper` now supports `selectedStepId` independently from the current
  workflow step and makes the marker plus step copy one native selectable
  surface.
- The promoted work-surface, process, conversation, and timeline APIs retain
  the same behavior across responsive layouts and preserve focus and state
  semantics when reduced-motion preferences suppress nonessential movement.

## [0.6.0] - 2026-07-17

### Added

- Added a fixed PDS-seeded Material 3 theme contract, including filled,
  tonal, outlined, text, elevated, toggle, grouped, and floating actions;
  elevated, filled, and outlined surfaces; animated filled and outlined
  fields; native select guidance; date and time pickers; lists; dynamic
  search; and docked, floating, and vibrant toolbars.
- Added a zero-layout-shift control invariant: focus, selected, active,
  invalid, and expanded state emphasis preserves control and content geometry.
- Added selectable Apple-like and Material-like visual themes to the
  interactive catalog. Apple-like preserves the ratified Precision Daylight
  treatment; Material-like retains PDS semantics and signals with opaque tonal
  surfaces, compact shape, state layers, and discrete elevation. The selection
  persists independently from System/Light/Dark mode and is covered by
  browser-backed accessibility and token-signature evidence.
- Ratified the Precision Daylight, Chromatic Signal, and Connected Fabric
  visual languages. The interactive catalog is their canonical exemplar, with
  luminous light-theme hierarchy, governed vivid signals, sparse non-overlapping
  circuits, reduced-motion behavior, and explicit decorative-only semantics.
- `FlowGraphShell` now publishes a PDS-token-backed `--xy-*` bridge on its
  viewport for future `@xyflow/react` adapters. The bridge keeps React Flow
  rendering behind the PDS shell and does not claim adapter or live chat
  readiness.

## [0.5.0] - 2026-07-03

### Added

- Ambient AI control affordances: `AssistLevelControl` for per-task autonomy
  selection and `MemoryChip` for visible, correctable personalization memory.
  The controls keep assist behavior and memory storage product/runtime-owned
  while making conservative defaults, disabled higher-autonomy posture, and
  correction/reset affordances visible in the shared PDS catalog.

### Notes

- Additive minor release. The controls remain `beta` / `enterprise-ready` and
  do not claim live AI, memory persistence, or governed-write readiness.

## [0.4.0] - 2026-07-03

### Added

- Ambient AI component family: `GeneratedViewShell`, `SuggestedAction`,
  `RecommendationCard`, `EvidenceSummary`, `InsightSummary`,
  `FreshnessIndicator`, `AttentionMarker`, and `AiAttributionAffordance`.
  The family provides PDS-owned generated-view, recommendation, cue, summary,
  freshness, and attribution chrome for ambient AI influence while keeping
  resolved refs, policy, answer envelopes, assist level, and all execution in
  product/runtime code.

### Notes

- Additive minor release. The family is `beta` / `enterprise-ready`: catalog,
  token, accessibility, agentic-UX, answer-envelope, view-registry, and
  chat-eval evidence exist, while CH8 consumer-wiring evidence remains a future
  gate before any release-gated claim.

## [0.3.0] - 2026-07-03

### Added

- Conversation component family: `MessageThread`, `Message`,
  `MessageComposer`, `StreamingText`, `ToolCallStatus`, `EntityRefCard`,
  `CitationList`, `ConfidenceSignal`, `AgentTimeline`, and `FlowGraphShell`.
  The family provides PDS-owned answer-review chrome, grounded references,
  citation/confidence display, persistent agent timeline, and renderer-neutral
  graph shell APIs without claiming live chat runtime, gateway, or write
  execution readiness.

### Notes

- Additive minor release. `@assistant-ui/react` and `@xyflow/react` are
  declared as optional peer integration surfaces for future adapters; the first
  CH5 slice remains dependency-light and token-backed.

## [0.2.0] - 2026-06-10

### Added

- Analytics chart renderers: `BarChart`, `LineChart`, `AreaChart`, and
  `DonutChart` — zero-dependency, token-styled SVG components that draw a
  provided series. The design system owns chart chrome and rendering; products
  still own data, calculations, thresholds, and interpretation. (`beta`)

### Notes

- Additive minor release — no breaking changes. The four new components are
  `beta` (since `0.2.0`); the rest of the catalog is unchanged.

## [0.1.0] - 2026-06-10

Initial framework-owned design-system baseline.

### Added

- Component catalog: 8 families / 67 components — Actions, Forms, Data Grid,
  Overlays, Feedback, Navigation, Process, and Analytics.
- Canonical `--pds-*` design tokens with light/dark support and drift checks.
- Agent-readable catalog manifest (`reference/catalog.json`), static visible
  catalog (`reference/index.html`), and the interactive catalog app
  (`catalog-app/`) with theme switching, search, and copy-ready snippets.
- Accessibility evidence gate (`scripts/check-pds-catalog-evidence.mjs`) running
  axe across desktop/mobile light/dark scenarios.
- Per-component lifecycle metadata (`status` + `since`) exported as
  `pdsComponentLifecycle`, and a SemVer + deprecation policy.

### Stability

- Actions, Forms, Data Grid, Overlays, Feedback, and Navigation families are
  `stable`.
- Process and Analytics families are `beta` (API may change in a minor release
  until retained adoption evidence is in place).
