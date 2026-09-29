# PDS Experience System Architecture And Coverage

Status: accepted architecture under ADR 0015; implementation maturity remains
evidence-gated

Owner roles:

- Product Owner: Wayne Kempf
- Design authority: unassigned
- Architecture owner: unassigned
- Implementation owner: App Framework frontend lane

ADR 0016 supplies the web interaction boundary: React Aria Components owns
commodity mechanics behind PDS APIs, while PDS owns doctrine, semantics,
canonical design data, visual grammar, motion, compositions, intelligence,
journeys, and evidence. This coverage plan does not authorize a broad control
rewrite; the Nexus readiness spec sequences bounded vertical deliverables.

## Purpose

Define what the PDS Experience System must contain before the framework
expands or certifies individual controls. This is the coverage map beneath the
UX strategy and ADR 0015. It turns “design system, then all necessary
components, controls, and behaviors” into an ordered implementation contract.

## Governing Invariant

The catalog is the executable proof of a PDS design system, not a pile of
individually polished controls. Its required dependency order is:

> **Experience doctrine and dimensions -> canonical tokens and adaptation
> rules -> primitives and behavior contracts -> controls and components ->
> compositions and floor plans -> product journeys and evidence.**

Catalog navigation, maturity claims, implementation sequencing, and retained
evidence must preserve that order. Component count and isolated visual polish
are not measures of system maturity.

## System Model

| Layer | Owns | Catalog proof |
| --- | --- | --- |
| Doctrine | Experience thesis, altitudes, laws, content, trust, intelligence semantics | Decision records and anti-pattern examples |
| Foundations | Color, type, spacing, shape, elevation, motion, icons, data visualization, state | Token inspector, contrast and motion specimens |
| Adaptation | Grammar, mode, platform scale, window class, density, input, reduced motion, forced colors | Governed selectors and automated render matrix |
| Primitives | Layout, text, icon, divider, focus, portal, state layer, visually hidden | Stable low-level APIs and geometry evidence |
| Controls | Buttons, fields, choices, pickers, menus, selection, search, command | Complete state and interaction specimens |
| Components | Navigation, collections, overlays, feedback, data, workflow, intelligence | Production-shaped component stories |
| Patterns | Forms, filtering, bulk work, list-detail, approval, notifications, personalization | Pattern compositions with failure states |
| Floor plans | Portal/Home, List-Detail, Workbench, Dashboard/Feed, Conversation, Flow/Wizard | Window-class and navigation adaptation |
| Journeys | Orient, Understand, Act, Resolve with signature moments | Product evidence against current/vendor baselines |

No lower layer may invent a value or behavior owned by a higher layer.

## Foundation Contracts

The canonical generated design-data source must cover:

- **Color:** reference ramps; semantic surfaces, text, borders, actions,
  signals, intelligence, and data-series roles; contrast targets by mode.
- **Typography:** primary and metric voices, type roles, weight, line height,
  tabular numerals, fallback metrics, truncation, and localization behavior.
- **Space and layout:** 4px base rhythm, component spacing, content widths,
  safe areas, grids, breakpoints/window classes, and floor-plan tracks.
- **Shape and depth:** role-based radii, borders, state layers, surface versus
  floating materials, discrete elevation, and grammar mappings.
- **Motion:** semantic roles, standard and expressive schemes, duration and
  easing/spring tokens, interruption behavior, reduced-motion equivalents,
  and a 100ms acknowledgment budget.
- **Iconography:** one approved icon set, optical sizing, stroke/fill rules,
  accessible-name rules, and platform mapping.
- **Data visualization:** categorical/sequential/diverging palettes,
  non-color redundancy, metric type, empty/loading/error states, and dense
  dashboard behavior.
- **Content:** Verb+Noun actions, domain nouns, empty/error/recovery grammar,
  dates/numbers/localization, and AI/provenance language.

CSS and TypeScript are generated delivery forms, not competing sources.
React Native and design-tool mappings consume the same semantic source.

## Adaptation Contract

The system has four orthogonal dimensions:

| Dimension | Values | Rule |
| --- | --- | --- |
| Grammar | `apple-like | material-like` | Visual expression only; identical meaning and APIs |
| Mode | `system | light | dark` | Semantic token re-resolution |
| Scale | `pointer | touch` | Ergonomics, target sizes, spacing, type, and navigation adaptation |
| Density | `comfortable | compact` | Data work only; overlays and critical actions do not densify |

Window size, input modality, reduced motion, forced colors, locale/RTL, zoom,
and offline capability are environment contracts, not extra themes.

The release matrix is four appearance signatures across two scales and two
densities. Automation must add reduced-motion, forced-colors, 200% zoom, RTL,
keyboard-only, screen-reader, and coarse-pointer probes without multiplying
product markup.

## Required Control Coverage

### Actions And Selection

- filled/primary, tonal, outlined, text/quiet, elevated, danger, loading, and
  governed-action buttons;
- icon, toggle, split, connected group, floating, and extended floating
  actions;
- checkbox, radio group, switch, segmented control, slider, stepper, chip,
  tag, filter chip, and removable token; and
- destructive confirmation, preview, undo, compensation, permission denial,
  and action audit states.

### Input And Search

- filled and outlined text fields, textarea, number/currency, password,
  masked/sensitive, prefix/suffix, counter, validation, and read-only states;
- native finite select plus combobox/autocomplete for searchable, async,
  grouped, creatable, large, and multi-value choices;
- date, date range, time, date-time, timezone, recurrence, and duration input;
- search field, dynamic search, global search, command palette, query/filter
  builder, file upload, drag-and-drop, and lookup; and
- mobile keyboards, autofill, paste, clear, scan/camera, interruption, and
  offline/freshness behavior where relevant.

### Navigation And Commands

- app bar/header, global navigation, side navigation, navigation drawer,
  bottom navigation, navigation rail, breadcrumbs, tabs, pagination, stepper,
  back/close affordances, toolbar, command bar, and command palette;
- responsive navigation morphing by window class; and
- focus restoration, deep links, browser history, unsaved-change handling,
  and keyboard shortcuts.

### Overlays And Disclosure

- tooltip, popover, menu, context menu, submenu, dialog, alert dialog, drawer,
  bottom sheet, accordion/disclosure, and inline expansion;
- focus trap/return, Escape/back handling, inert background, viewport
  collision, scroll locking, and touch dismissal; and
- no layout shift when opening, selecting, focusing, validating, or loading.

## Required Component Coverage

### Information And Work

- surface/card variants, page header, section header, divider, badge, avatar,
  list/list item, description list, timeline, activity feed, tree, calendar,
  attachments, comments, and evidence/citation views;
- data table/grid with virtualization, sort, filter, grouping, selection,
  resizing, pinned columns, bulk actions, export, pagination/infinite loading,
  density, keyboard and screen-reader operation; and
- form sections, responsive forms, wizard/flow, review summary, queue,
  task/approval, record detail, status/progress, audit history, and receipt.

### Feedback And Resilience

- inline message, alert, banner, snackbar/toast, notification center, inbox,
  live operation status, progress, skeleton, and optimistic state;
- typed empty, loading, partial, stale, offline, conflict, unauthorized,
  forbidden, not-found, rate-limited, timeout, and service-degraded states;
- retry, resume, save draft, undo, conflict resolution, escalation, and
  support correlation identifiers; and
- web push/in-app/mobile notification preference, grouping, quiet hours,
  read/unread, deep link, and action contracts.

### Intelligence

- structured generated views, evidence summaries, citations, freshness,
  confidence, permission, source-system and AI attribution;
- suggestions, recommendations, next-best action, intent preview, governed
  action preview, explainability, edit/revert-to-AI, assist level, and memory;
- agent/tool progress, latency-as-designed-state, handoff to human, failure,
  partial result, cancellation, and durable work-state semantics; and
- conversation as an invocation/exploration component, never the default
  container for intelligence that belongs in the work surface.

## Behavior Completion Standard

Every control and component is incomplete until it specifies and proves:

- rest, hover, focus-visible, pressed, active, selected, expanded, disabled,
  read-only, loading, success, warning, error, partial, and stale states as
  applicable;
- pointer, keyboard, touch, screen reader, voice/switch-access, and programmatic
  operation;
- stable outer geometry and content origin across state changes;
- standard motion, interruption/reversal behavior, and reduced-motion parity;
- comfortable/compact density and pointer/touch scale where applicable;
- light/dark and apple-like/material-like rendering without semantic drift;
- localization, long content, RTL, 200% zoom, forced colors, and reflow;
- empty/loading/error/offline/recovery behavior and editorial copy; and
- unit, accessibility, browser, visual, performance, and consumer evidence.

“Rendered in the catalog” is foundation evidence. “Stable” requires consumer
usage, complete behavior evidence, versioned API review, and release-gate
traceability.

## Native-Smooth Motion Standard

Motion is implemented from semantic roles, not one global transition.

- Controls acknowledge input in the next frame and normally within 100ms.
- Material-like state changes use state layers and authentic Material motion
  geometry. Floating field labels use measured start/end geometry and a crisp
  transform animation; changing `top` and `font-size` directly is prohibited.
- Apple-like controls use fluid response, fine depth, and platform-appropriate
  continuity without copying iOS trade dress.
- Spatial transitions may use platform-native springs or web View Transitions
  only at signature moments; routine work uses the standard scheme.
- Animations are interruptible, reversible, compositor-friendly, and never
  delay the underlying state change or action.
- Reduced motion removes spatial travel and overshoot while preserving state,
  hierarchy, focus, and completion feedback.

## Delivery Order

1. Preserve the ratified ADR 0015/0016 decisions and approved font provenance.
2. Generate canonical DTCG-compatible foundations and adaptation tokens.
3. Certify primitives and the cross-cutting behavior harness.
4. Complete controls by workflow value, starting with actions, fields,
   selection, menus, search, and navigation.
5. Complete components for queues, forms, data work, notifications,
   resilience, and intelligence.
6. Build and certify six adaptive floor plans.
7. Prove Nexus and other product journeys against operational and signature
   experience baselines.

Do not certify an entire family in one visual sweep. Implement bounded vertical
deliverables that include foundations, behavior, web/touch adaptation, both
grammars/modes, evidence, and one production-shaped composition.

## Acceptance Evidence

- generated token/design-data drift checks;
- identical API and semantics across the dimension matrix;
- axe plus keyboard, focus, forced-colors, zoom, RTL, and reduced-motion
  evidence;
- stable-geometry measurement for representative controls;
- browser performance evidence including INP and layout shift;
- native/touch device evidence before mobile claims;
- production-shaped composition and consumer proof; and
- retained Product Owner/design review for signature graduation.
