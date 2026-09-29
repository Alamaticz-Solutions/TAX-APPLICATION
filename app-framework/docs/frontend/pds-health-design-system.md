# PDS Health Enterprise Design System

This guide is the framework-owned plan and operating contract for the PDS
Health design system used by App Framework frontends.

The design system is specifically branded for PDS Health. It is not a generic
theme layer and it is not the CRM frontend. CRM remains a reference
implementation and framework E2E fixture. New product frontends should start
from a CRM-neutral, PDS-branded scaffold and use CRM only for example patterns.

## Plan Of Record

The plan of record is to build a PDS Health enterprise design system that lets
product developers and agents create clean, brand-consistent, accessible,
generated-product frontends without copying CRM residue, while keeping CRM as
the reference implementation and framework E2E fixture.

This plan has five non-negotiable pillars:

- canonical `appfw_ui/pds_health` source for tokens, components, content
  rules, and governance;
- CRM-neutral product scaffolds that consume generated contracts and retain
  release evidence;
- reusable enterprise components for actions, forms, navigation, work
  surfaces, data grids, overlays, and feedback states;
- accessibility and content behavior that is built into component APIs rather
  than rediscovered by each product; and
- machine-readable evidence for token drift, component contract checks,
  scaffold residue checks, product frontend checks, and framework handoff.

## Outcomes

The design system should make enterprise product UI work predictable:

- product teams start with a clean PDS-branded base UI;
- agents can add workflows without copying CRM screens or admin UI internals;
- reusable components express framework concepts such as tenant, role, entity,
  operation, validation, request ID, and correlation ID;
- accessibility, responsive layout, focus states, and theme behavior are
  default platform behavior; and
- CI retains evidence for docs, scaffold checks, frontend tests, and CRM
  reference coverage.

## Experience Thesis And Current Gap

The tracked, machine-readable component catalog defines the current inventory
across ten families. It includes unusually relevant enterprise primitives for
governed actions, operation state, evidence, freshness, AI attribution,
process, conversation, and dense data work. Existing checks cover token use,
catalog parity, selected accessibility semantics, browser scenarios, responsive
overflow, and representative consumers. This is a credible engineering
foundation, not a claim that the visual system, complete-task accessibility,
live governed actions, or production product experience is mature.

The target experience is:

> **Calm precision. Expressive intelligence. Unmistakable craft.** The
> operational base is quiet, exact, responsive, accessible, and native to its
> channel. Visual expression is earned by the next action, a material state or
> risk, cross-system insight, live progress, or successful completion. The
> complete experience should feel unusually coherent, distinctive, and
> beautifully made in first impressions, repeated use, and failure recovery.

The formal product requirement behind executive requests for "sizzle" is
**signature experience quality**: the immediate and sustained perception that
the product understands the work, responds fluidly, preserves context, and
resolves complexity with unusual clarity. It is accumulated product craft and
domain-specific intelligence, not gradients, glass, glow, novelty motion,
canned intelligence, or demo-only choreography.

[UX Design Strategy](ux-design-strategy.md) operationalizes this thesis as
the decision framework for all UX design work: six experience altitudes with
energy budgets, ten precedence-ordered design laws, a decision procedure,
sanctioned evidence bases, and agent handoff protocol. Theme, floor-plan,
component, control, and behavior decisions route through that framework.

The current strategic gap is composition and hierarchy, not component count.
Effects and containment currently carry too much of the visual structure:
gradients, glow, blur, borders, shadows, and card-like surfaces recur across
the shared tokens and CSS. The target system establishes hierarchy through
alignment, spacing, typography, proximity, stable state, and exact behavior
first. It uses containment for repeated actionable items, transient overlays,
and genuine tools rather than turning every section into a card.

### Governing Construction Order

> **The catalog is the executable proof of a PDS design system, not a pile of
> individually polished controls.**

Design and delivery follow one dependency order:

1. experience doctrine and dimensions;
2. canonical tokens and adaptation rules;
3. primitives and behavior contracts;
4. controls and components;
5. compositions and floor plans; and
6. product journeys and retained evidence.

The catalog must make this dependency graph inspectable. A polished control
cannot compensate for a missing doctrine, semantic token, adaptation rule,
behavior contract, composition, or journey result. Work that begins at a
lower layer must either cite the governing higher-layer contracts or stop and
record the missing decision.

Admin, product web, native mobile, and embedded intelligent surfaces should be
related but not identical:

- **Admin web:** dense, keyboard-first, inspectable, and optimized for
  comparison, tracing, and diagnosis.
- **Product web:** role- and journey-oriented, responsive, paced around a clear
  next action, and explicit about state and consequence.
- **Native mobile:** short task sequences, thumb-reachable actions,
  interruption-safe state, offline/freshness awareness, and platform-native
  navigation and controls.
- **Embedded intelligence:** bounded structured results with permission,
  evidence, freshness, action preview, fallback, audit, and durable work-state
  semantics. Chat is a secondary invocation or exploration path, not the
  default or authoritative container.

### Ratified Visual Language

ADR 0014 ratifies three related PDS Health visual-language contracts as the
plan of record:

- **Precision Daylight:** a luminous blue-white light canvas, graphite
  typography, quiet translucent work surfaces, restrained depth, and no gray
  cast;
- **Chromatic Signal:** accessible semantic ink separated from a compact vivid
  palette for data, progress, state marks, active controls, and bounded
  intelligence cues; and
- **Connected Fabric:** an optional expressive web layer in which sparse
  source-to-destination circuits and a return acknowledgment evoke the
  connected enterprise application fabric.

The interactive PDS Component Catalog is the canonical visual exemplar. The
direction is accepted, but product release readiness remains journey- and
channel-specific. Connected Fabric is decorative, `aria-hidden`, input
transparent, reduced-motion aware, and prohibited from communicating business
state by itself. Concurrent circuits may meet at nodes but cannot reuse visible
edges. Native mobile receives a justified channel-native expression rather
than a mechanical canvas port.

The catalog exposes two governed visual themes. **Apple-like** is the default
Precision Daylight profile, with luminous translucent surfaces and restrained
depth. **Material-like** is the fixed PDS-seeded Material 3 expression, with
M3 role colors, opaque tonal surfaces, role-specific shape, state layers,
authentic component motion, and discrete elevation. PDS doctrine and semantic
contracts remain authoritative; within the material-like grammar, M3 is the
component-anatomy and interaction reference, and MUI supplies only
underspecified web behavior. There is no wallpaper extraction or Material
runtime dependency. This
`data-visual-theme` axis is independent from the `data-theme` System/Light/Dark
color-mode axis. Neither theme may fork component behavior or product logic.
Focused, selected, active, invalid, and expanded states must preserve outer
and content geometry; field-label movement is internal to the stable shell.

Detailed contracts:

- `docs/architecture/adr/0015-pds-experience-system-dimensions.md`;
- `docs/architecture/adr/0016-pds-web-interaction-substrate.md`;
- `docs/specs/pds-experience-system-architecture-and-coverage.md`;
- `docs/specs/nexus-experience-system-readiness.md`;
- `appfw_ui/pds_health/components/visual-themes.md`;
- `appfw_ui/pds_health/components/material-3-web-contract.md`;
- `appfw_ui/pds_health/components/precision-daylight-visual-language.md`;
- `appfw_ui/pds_health/components/chromatic-signal-visual-language.md`; and
- `appfw_ui/pds_health/components/connected-fabric-visual-language.md`.

Keep React, Vite, Expo, and React Native. Tailwind CSS 4 may remain a
composition tool where products already use it, but it is not the design
system or token authority. Do not restyle or expand the full component
inventory in isolation; refine them only as production-shaped compositions
expose a named gap.

### Component Runtime Boundary

PDS owns the public component API, semantic state, content, canonical design
data, visual grammar, motion, floor plans, governed work, intelligence, and
evidence. React Aria Components is the default web interaction substrate for
commodity mechanics behind that boundary. Products do not import it directly,
and no visual grammar selects a different behavior tree. Existing PDS APIs are
migrated by Nexus workflow value rather than rewritten in one sweep.

Full Material UI, Spectrum, Fluent, and Material Web component systems are not
the PDS runtime. Their research and anatomy remain useful evidence. A
specialized grid, scheduler, chart, flow, or editor engine may be adopted only
behind a PDS adapter after a real product slice proves the need and license,
bundle, accessibility, observability, fallback, and exit costs are understood.
React Native keeps channel-native behavior over shared semantic design data.

### Signature Experience Quality Standard

Use an `80/20` allocation heuristic: familiar, accessible, platform-native
patterns should carry routine work, while a small number of moments receive
disproportionate product-design, visualization, motion, engineering, and
research attention. This is not a literal screen-coverage rule. Expression
everywhere would make no moment distinctive and would damage repeated-use
efficiency.

Before high-fidelity implementation, every flagship journey needs a short
**signature experience brief** naming the desired user emotion; one meaningful,
accessible, domain-specific visual object; two or three signature moments; web
and native-mobile expression; gesture, motion, and restrained haptic intent
where applicable; evidence, permission, consequence, and recovery semantics;
performance, accessibility, reduced-motion, offline, stale-data, and fallback
behavior; and the current PDS and strongest relevant vendor-native comparison
baselines. Readiness, dependency, provenance, decision, and operation-trace
visualizations are useful examples only when they answer a real question faster
than generic cards or tables and retain an equivalent structured view.

The north-star compositions should make one common sequence unmistakable:

1. **Orient:** reveal what matters now and why.
2. **Understand:** turn intelligence into editable visual structure with
   evidence rather than prose or chat alone.
3. **Act:** preview governed cross-system consequences and preserve agency.
4. **Resolve:** turn live progress into a concise, satisfying receipt without
   losing context.

Each flagship web reference must prove two or three production-shaped
signature moments and each flagship native product at least one corresponding
mobile moment. Native craft means platform behavior, interruption safety,
secure continuation, useful gesture, and restrained haptic feedback for
consequential confirmation or direct manipulation, not compressed desktop UI.
Every direct interaction receives immediate visual acknowledgment, normally in
the next frame and no later than roughly 100 milliseconds on supported
hardware. Semantic motion roles cover selection, expansion, continuity,
progress, success, warning, and recovery; motion is reviewed in context and
remains fully usable with reduced motion.

Experience graduation has two independent comparative bars. **Operational
superiority** is initially hypothesized as at least 15 percent faster critical-
workflow completion or a ten-point task-success lift. **Signature experience
quality** is provisionally at least 65 percent blinded preference plus at least
a one-point lift on a seven-point crafted/fresh/intelligent/distinctive/
beautiful composite. Baseline research may calibrate the numbers but must not
remove either gate. Evidence includes five-second and 30-second first
impressions; blinded comparison with current PDS and vendor-native baselines;
dated/fresh, generic/distinctive, static/alive, mechanical/intelligent,
cluttered/composed, and ordinary/beautiful semantic differentials; signature-
object and interaction recall; no regression in task performance,
accessibility, trust, reliability, or recovery; INP at or below 200 ms at p75;
and reduced-motion behavior.

Reject polished genericity, expression everywhere, copied vendor aesthetics,
demo-only choreography, canned intelligence, and surprise that makes repeated
work slower. Experienced product-design ownership remains required: agents can
reproduce encoded judgment but do not define the product's visual point of view
unaided. This standard is substantiated by the July 14, 2026 App Framework
experience-design direction retained in the PDS app-fabric research workspace.

## Enterprise UX Direction

The design system should track modern business-app practice where it improves
decision speed, evidence quality, and user trust. The useful signal is not
decorative novelty; it is denser utility, clearer governance, and interaction
patterns that help people finish regulated work with confidence.

| Signal | PDS Design-System Response |
| --- | --- |
| High-information density | Favor compact grids, sticky command/filter regions, configurable columns, and stable structural rhythm over scroll-heavy white space. Data grids default to compact density, with a keyboard-accessible selector for users who need a more comfortable view. |
| Signature experience quality | Keep familiar controls quiet, then invest disproportionate craft in a few domain-specific visual objects and Orient-Understand-Act-Resolve moments that are tested for both task performance and blinded preference. |
| Contextual agentic assistance | Treat AI as task infrastructure: generated views, suggested next actions, confidence/explainability cues, and evidence summaries near the workflow. Use [Agentic UX](agentic-ux.md) for invoked and ambient rules. Avoid isolated chatbot-only patterns as the primary agentic experience. |
| Compliance-first accessibility | Build accessible names, focus order, contrast, non-color status cues, keyboard paths, and retained verification evidence into component APIs and scaffold checks. |
| Calm, transparent interaction | Use motion and depth only to clarify state, priority, or hierarchy. Prevent errors inline when possible, and expose tenant/role-driven personalization without making the user feel watched. |
| Multi-device continuity | Preserve workflow state across responsive layouts. Mobile views should keep critical actions reachable while desktop views prioritize scan density and comparison. |

## Ownership Model

| Surface | Audience | Ownership | Notes |
| --- | --- | --- | --- |
| `appfw_ui/pds_health/tokens/tokens.dtcg.json` | Framework stewards | Canonical framework-owned token source | Transitional W1a token declarations and render metadata; edit this file first, then generate CSS/TS. |
| `appfw_ui/pds_health/tokens/pdsTokens.css` and `pdsTokens.ts` | Frontend consumers | Generated framework-owned token outputs | Consume through the PDS package/aliases; regenerate with `scripts/generate-pds-tokens.mjs` and do not edit directly. |
| `appfw_ui/pds_health/components/src` | Framework stewards and frontend agents | Framework-owned source | React component APIs, typed package catalog API, and token-backed CSS. No CRM names and no product domain nouns. |
| `appfw_ui/pds_health/reference` | Product developers and agents | Framework-owned catalog contract | Canonical product-neutral PDS Component Catalog manifest plus static fallback: component examples, source API map, agent decision guide, states, density, themes, accessibility notes, maturity levels, evidence requirements, and usage examples. |
| `appfw_ui/pds_health/catalog-app` | Product developers and agents | Framework-owned design-system application | Unified PDS portal with persistent navigation across Overview, Brand, Floor plans, Elements, Components, Patterns, Data visualization, and reference proofs. The Components view consumes `reference/catalog.json` and `components/src`; the application must not become a second source of truth. |
| `docs/frontend/pds-health-design-system.md` | Framework stewards and product leads | Framework-owned contract | Design-system plan, maturity gates, and release evidence expectations. |
| `docs/frontend/product-frontend.md` | Product developers and agents | Product consumption contract | How a product uses generated contracts and PDS design-system assets. |
| `admin_ui` | Framework stewards | Framework operations UI | Consumer of PDS assets; not a product starter. |
| `examples/products/crm/frontend` | Framework stewards | Reference fixture | Demonstrates advanced patterns and E2E coverage; not copied into new products. |
| Product `frontend/` | Product developers | Product-owned | Product workflows, copy, tests, and release evidence. |
| `app_gen` | Framework stewards and generator agents | Generator/scaffold source | Product-intake scaffold source that consumes PDS components without CRM residue. |

## Architecture

The enterprise frontend system has five layers:

1. PDS brand tokens: canonical transitional JSON in
   `appfw_ui/pds_health/tokens/tokens.dtcg.json`, generating the CSS variables
   and typed exports consumed from that directory.
2. PDS component system: framework-owned React primitives, composites, typed
   package catalog metadata, and token-backed CSS under
   `appfw_ui/pds_health/components/src`.
3. PDS Design System application and Component Catalog: an agent-readable
   manifest and product-neutral static fallback under
   `appfw_ui/pds_health/reference`, plus a unified interactive design-system
   portal under `appfw_ui/pds_health/catalog-app`. Its Components view consumes
   the same framework-owned component API while the other views expose brand,
   foundations, floor plans, patterns, data visualization, and proof surfaces.
4. Product scaffold source: `app_gen` creates CRM-neutral starter app shells,
   generated contract imports, route registries, data clients, auth/tenant
   contexts, design imports, and scaffold evidence.
5. Reference fixtures: CRM and future golden apps exercise the same components
   through realistic workflows and E2E tests, but they do not define the design
   system.

Product frontends may add product-specific features, but they should not fork
the design system or introduce a parallel component kit.

## Component Catalog

The PDS Component Catalog is the product-neutral reference at
`appfw_ui/pds_health/reference`. It is not CRM and it is not admin UI. It is the
visible catalog for the framework-owned component API exported from
`appfw_ui/pds_health/components/src`.

Use this mental model:

| Location | Meaning |
| --- | --- |
| `appfw_ui/pds_health/tokens/tokens.dtcg.json` | Canonical transitional PDS Health token source: ordered token declarations and render metadata. It does not yet claim native DTCG `$type`/`$value` groups. |
| `appfw_ui/pds_health/tokens/pdsTokens.css` and `pdsTokens.ts` | Generated CSS-variable and typed token outputs for frontend consumers; regenerate from `tokens.dtcg.json` and do not edit directly. |
| `appfw_ui/pds_health/components/src` | Framework-owned React component APIs, `pdsComponentCatalog`, `pdsComponentFamilies`, `pdsAgentDecisionGuide`, and token-backed CSS. Product-neutral source only. |
| `appfw_ui/pds_health/reference/catalog.json` | Canonical agent-readable PDS Component Catalog manifest: family inventory, component inventory, source API map, agent decision guide, maturity level, product boundary, evidence requirements, readiness contract, entrypoints, and agent do/avoid guidance. |
| `appfw_ui/pds_health/reference/index.html` | Static visible PDS Component Catalog fallback: examples, family inventory, source API map, agent decision guide, states, density, themes, accessibility notes, agent readiness ledger, usage guidance, and verification expectations. |
| `appfw_ui/pds_health/catalog-app` | Unified interactive PDS Design System application. Its root is the full portal; the Components view supplies live examples for every exported component, component search, light/dark theme review, and copy-ready imports/usage snippets. It reads `reference/catalog.json` and imports `components/src`; product nouns do not belong here. |
| `scripts/check-pds-components.mjs` | Evidence gate that checks exported components, source export drift, matching props types, token-backed CSS, source neutrality, catalog manifest coverage, visible/static catalog coverage, interactive catalog coverage, agent decision recipes, family readiness sections, maturity/evidence ledger coverage, consumer wiring, density policy, hashes, and package metadata. |
| `scripts/check-pds-catalog-evidence.mjs` | Browser-backed catalog evidence gate that builds the interactive catalog, serves it, captures desktop/mobile light/dark screenshots, runs axe, and checks layout overflow, popover containment, named controls, chart variations, process-flow examples, Conversation semantics, and Ambient AI grounding/preview-gating/attribution semantics. |

The catalog currently groups exported APIs into these families:

| Family | Cataloged components |
| --- | --- |
| Actions | `Button`, `ButtonLink`, `IconButton`, `MenuButton`, `CommandBar`, `CommandPalette` |
| Forms | `Field`, `FieldMetadata`, `FormLoadingPreview`, `FieldGroup`, `FormLayout`, `TextField`, `TextArea`, `SelectField`, `DateField`, `TimeField`, `DateTimeField`, `InputGroup`, `MultiSelect`, `FileUpload`, `CheckboxField`, `SwitchField`, `LookupSelect`, `ValidationSummary` |
| Data Grid | `DataGrid`, `DataGridShell`, `DataGridLoadingPreview`, `DataGridToolbar`, `DataGridColumnChooser`, `DataGridColumnChooserTrigger`, `DataGridColumnResizeHandle`, `DataGridControlPopover`, `DataGridDensityControl`, `DataGridFilterPanel`, `DataGridFilterGroup`, `DataGridFilterRule`, `DataGridFilterEmpty`, `DataGridFilterTrigger`, `DataGridSortButton`, `DataGridPagination` |
| Overlays | `Dialog`, `Drawer`, `Popover`, `PopoverTrigger`, `Tooltip`, `ConfirmDialog` |
| Feedback | `Alert`, `Banner`, `FeedbackState`, `LoadingState`, `ErrorState`, `ForbiddenState`, `InlineAlert`, `EmptyState`, `Skeleton`, `Toast`, `ToastRegion` |
| Navigation | `AppShell`, `NarrativeWorkspace`, `ConnectedFabric`, `AppearanceProvider`, `NavigationItem`, `IconSlot`, `Avatar`, `IdentitySummary`, `Breadcrumbs`, `PageHeader`, `Surface`, `Tabs`, `SegmentedControl`, `Badge`, `InteractiveCard`, `CardLink`, `WorkQueueItem` |
| Process | `ProcessStepper`, `ProcessProgress` |
| Analytics | `KpiTile`, `MetricTrend`, `ChartShell`, `ChartLegend`, `TimelineRangeSelector` |
| Conversation | `ConversationWorkspace`, `MessageThread`, `Message`, `MessageComposer`, `StreamingText`, `ToolCallStatus`, `EntityRefCard`, `CitationList`, `ConfidenceSignal`, `AgentTimeline`, `FlowGraphShell` |
| Ambient AI | `GeneratedViewShell`, `SuggestedAction`, `RecommendationCard`, `EvidenceSummary`, `InsightSummary`, `FreshnessIndicator`, `AttentionMarker`, `AiAttributionAffordance`, `AssistLevelControl`, `MemoryChip` |

Version 0.9.0 exposes the compiled package root, `styles.css`, `tokens.css`,
and the public family subpaths `ambient`, `catalog`, `charts`, `conversation`,
`conversation-workspace`, `connected-fabric`, `copy`, `data`, `data-grid`, `experience`,
`foundation`, `forms`, `layout`, `narrative-workspace`, `primitives`, `process`, `surfaces`,
`timeline`, `types`, and
`work-surfaces`. Product code imports those entrypoints only. It must not
import checkout-relative files below `components/src`, React Aria, or AG Grid;
those implementations remain replaceable behind the PDS contract.

Version 0.12.0 adds IX Web APIs only at the explicit
`intelligence-presentation`, `intelligence-presentation-model`, and
`ix-recipes` subpaths; the root barrel intentionally remains free of those
exports. The `@appfw/pds-ix-presentation-contract@0.2.0` peer is optional for
core installation and required whenever an IX subpath is imported. Catalog
source resolves these public subpaths through exact TypeScript and Vite
aliases, without a catalog package or lock dependency. The packed Web
manifest must contain no local `file:` dependency.

The channel-neutral presentation supplies one announcement per revision and
each renderer owns exactly one emission. Web recipe composition suppresses
nested response/context/status live regions; the standalone primitives remain
independently announceable, including the progressive-response empty state.
Presentation validation rejects duplicate response-region IDs, catalog fixture
validation rejects changed-region references outside the exact revision, and
generated evidence DOM IDs are scoped per instance. The catalog's extra
fixture notice row applies only to the IX reference; every earlier reference
iframe preserves its one-row fill and minimum usable height.

The clean package proof builds, packs, installs, typechecks, and executes exact
contract `0.2.0`, Web `0.12.0`, and native `0.2.0` archives. It provisions a
fresh writable cache before an unchanged offline phase and rejects source,
`file:`, and Expo leakage. Native iOS and Android remain `not-qualified`; this
is package portability evidence, not device, distribution, product-adoption,
security, or release evidence.

The 0.7.0 foundation additions are beta: `AppearanceProvider` propagates
system/light/dark and Apple-like/Material-like preference state;
`NavigationItem` preserves native link-versus-button semantics; `IconSlot`
normalizes icon geometry; and `Avatar` plus `IdentitySummary` provide compact
identity composition. Products continue to own route behavior, identity
values, role visibility, and appearance persistence policy.

The 0.8.0 navigation continuation makes expanded labels readable before
adding secondary disclosure: shared 14 px / 20 px navigation typography,
two-line wrapping by default, fixed icon and trailing columns, and an
overflow-only portal tooltip for exceptional clipping. `AppShell` owns the
optional 288 px adjustable desktop sidebar, 240–360 px bounds, keyboard and
pointer resizing, window-splitter semantics, and the 960 px responsive
handoff. Products own navigation labels, routes, visibility, counts, and the
storage key used to persist a chosen width.

The promoted 0.7.0 APIs deliberately separate reusable interaction mechanics
from product meaning:

- `ButtonLink` owns anchor presentation for location changes; products use
  `Button` for commands and own destinations and side effects.
- `InteractiveCard`, `CardLink`, and `WorkQueueItem` own full-surface native
  activation, selected/disabled/completed treatment, and sibling secondary
  actions. Products own routes, commands, copy, status meaning, and action
  consequences.
- `ProcessStepper` owns one native selectable surface for each step and keeps
  `aria-current` independent from `aria-pressed`; products own
  `selectedStepId`, filtering, step meaning, transitions, and workflow rules.
  Its `milestone` variant owns compact status glyphs, marker-centered
  connectors, and marker/label selection emphasis without exposing internal
  anatomy to product CSS.
- `ConversationWorkspace` owns the responsive named conversation,
  context, and action regions plus busy/composer semantics. Products own
  messages, prompts, retrieval, model calls, context content, actions, and
  write gates.
- `TimelineRangeSelector` owns the pointer/keyboard move-resize interaction,
  snap mechanics, and explicit range announcements. Products own the ordered
  time domain, labels, density values, permitted snap widths, resulting data
  query, and interpretation.

These shared APIs adapt their composition at responsive breakpoints without
forking product behavior. Selection, current, busy, disabled, and progress
states stay programmatically exposed. Visual movement is nonessential and must
respect reduced-motion preferences without removing focus or state feedback.

`DataGrid` is the PDS-owned high-capability adapter over AG Grid Community.
Products must not import AG Grid directly. Apple-like and Material-like themes
share PDS semantics and tokens; they are render grammars, not separate grids.
`DataGridShell` remains the preferred lightweight table for small collections.

`FlowGraphShell` owns the React Flow token bridge. The bridge is documented in
[Flow Graph Token Bridge](flow-graph-token-bridge.md): the shell viewport
publishes PDS-backed `--xy-*` CSS variables for future `@xyflow/react` adapters,
while `flow_graph_token_bridge.adapter_ready:false` keeps adapter and live
runtime claims gated until implementation, visual/a11y, and product evidence
exist.

`StreamingText` treats model output as untrusted. The CH5 markdown rendering
decision is recorded in [Chat Markdown Sanitizer Decision](chat-markdown-sanitizer.md):
future live markdown rendering uses `react-markdown` with `rehype-sanitize`,
raw HTML disabled, refs-as-pointers, and allowlisted URLs. The component check
retains `conversation_markdown_sanitizer.live_ready:false` until the adapter,
tests, catalog evidence, and release evidence exist.

Enterprise-ready catalog entries must cover props, states, density, theme
behavior, accessibility, usage, and verification. The component checker
enforces that every family has those contract sections, every component source
export appears in both `catalog.json` and `index.html`, every component has a
matching `<ComponentName>Props` export, the package exports
`pdsComponentCatalog`, `pdsComponentFamilies`, and `pdsAgentDecisionGuide`, and
the retained evidence records the manifest hash, `component_api_inventory`, and
`agent_decision_guide` before design-system evidence can pass. The checker also
gates the interactive catalog so every exported component has a live example
and copy-ready snippet, the renderer imports the real PDS source, and the
interactive catalog remains product-neutral.

Maturity levels:

| Level | Meaning |
| --- | --- |
| `foundation` | The family is defined as a target API but still needs visible examples, consumers, or release evidence before product use. |
| `enterprise-ready` | The family has token-backed source, catalog coverage, accessibility patterns, and representative consumers. Product use is appropriate with normal frontend evidence. |
| `release-gated` | The family is part of generated or scaffolded product flows and must retain catalog, visible reference, accessibility, consumer wiring, and scaffold evidence before release. |

### Maturity Evidence

Maturity is **evidence-gated, not self-asserted**: a family only keeps its
maturity claim while the evidence required for that level is backed by *passing*
checks. The manifest's `maturityEvidence` block declares, per level, which
evidence categories are required and which check verifies each one; the
`maturity-evidence` gate in `scripts/check-pds-components.mjs` fails if any
family claims a level whose backing checks are not green, and the result is
retained in the `maturity_evidence` block of `target/appfw/pds-component-check.json`.

| Level | Required evidence (each backed by a passing check) |
| --- | --- |
| `foundation` / `enterprise-ready` | catalog-manifest, visible-catalog, token-backed-css, accessibility-patterns |
| `release-gated` | the above plus consumer-wiring |

Governed actions carry one extra live-readiness contract. The component
catalog can prove the shared Intent Preview, confirmation, audit, and
undo/compensation surfaces, but it must not imply that a live write path is
certified. `scripts/check-pds-components.mjs --json` therefore retains
`governed_action_live_readiness`, including the status of
`target/appfw/governed-write-posture.json`,
`target/appfw/governed-write-evidence.json`, and the product
`.appfw/target/appfw/harness-check.json`. Normal component checks report this
status without failing. The live evidence file must satisfy the same G1
named-mutation proof shape used by `framework wave2-status`: supported external
API provider, delegated actor context, token-store partitioning, mutation
registry with `mcp_enabled:false`, idempotency/replay proof, and audit
correlation/sink. Use
`scripts/check-pds-components.mjs --json --enforce-governed-action` only when a
release or product lane claims governed-action live write readiness.

**Basis.** This evidence is `retained-local-ci` — it is produced and retained by
the framework's own checks (catalog, visible catalog, token drift, accessibility
patterns/axe, consumer wiring). It is **not** live managed-environment
certification, so `release-gated` should be read as "evidence-backed pending
live certification" (`scaffold-proof` is declared by generated-flow families and
verified externally by `framework intake-proof`).

## Versioning, Lifecycle, And Release Certification

Enterprise product teams adopt, pin, and upgrade the design system as a
*versioned* package, so the system carries an explicit version and lifecycle
contract in addition to the maturity model above. Maturity describes *evidence*;
lifecycle describes *API stability*.

- **SemVer.** `@appfw/pds-health-components` follows
  [Semantic Versioning](https://semver.org/). A breaking component API or token
  contract change is a **major** bump; an additive component, prop, or token is a
  **minor**; a fix is a **patch**. The current version is `pdsPackageVersion` in
  `components/src/catalog.ts` and `version` in `components/package.json`; the two
  must match.
- **CHANGELOG.** Every change is recorded in
  [`appfw_ui/pds_health/CHANGELOG.md`](../../appfw_ui/pds_health/CHANGELOG.md)
  using Keep a Changelog. The checker requires the CHANGELOG to reference the
  current package version.
- **Component lifecycle.** Every exported component carries a lifecycle status
  and `since` version, exported as `pdsComponentLifecycle` and mirrored in the
  manifest's `componentLifecycle`. Status is derived from family maturity, with
  room for per-component overrides.

| Status | Meaning |
| --- | --- |
| `stable` | API is stable and SemVer-protected. Default for `release-gated` families. |
| `beta` | Usable, but the API may change in a minor release. Default for `enterprise-ready` families. |
| `experimental` | Early access; may change or be removed. Default for `foundation` families. |
| `deprecated` | Scheduled for removal; must declare `replacement` and `removeBy` and keep working for at least one minor release. |

- **Deprecation policy.** A component is marked `deprecated` (with `replacement`
  and `removeBy`) and announced in the CHANGELOG for at least one minor release
  before it is removed in a major release.
- **Release certification.** `scripts/check-pds-components.mjs --json` emits a
  `release_certification` block in `target/appfw/pds-component-check.json`
  reporting the package version, SemVer validity, CHANGELOG status, manifest
  hash, lifecycle counts by status, and pointers to token-drift and
  accessibility evidence — a retainable design-system release certificate.

## Brand Guidance

PDS Health branded applications should feel operational, clear, and calm. The
visual system should support repeated work: scanning lists, editing records,
reviewing exceptions, understanding status, and completing governed actions.

| Area | Guidance |
| --- | --- |
| Color | Use PDS blue for primary action, focus, and selected state. Use gray/slate neutrals for structure. Reserve danger, success, teal, and gold for status meaning. |
| Typography | Use the PDS sans stack and compact enterprise sizes. Large display text belongs only on true product landing surfaces, not data-work screens. |
| Layout | Prefer dense but breathable layouts with stable toolbars, predictable navigation, and constrained content width for forms. |
| Shape | Keep cards and controls at 8px radius or less unless the component has a specific pill role. |
| Motion | Use subtle motion for state transitions only. Do not make motion carry required meaning. |
| Theme | Support Apple-like and Material-like through `data-visual-theme`; independently support System, Light, and Dark through `color-scheme` and `data-theme`. Both axes remain backed by the same semantic `--pds-*` tokens and component behavior. |
| Accessibility | Every control has an accessible name, visible focus, keyboard path, contrast compliance, and non-color status cue. |
| Copy | Use direct action labels and domain terms from the product model. Do not explain the framework inside the product UI. |

## Color, Depth, And Gradient Strategy

The PDS palette should feel exact and confident without turning governed work
surfaces into marketing pages. Meaning, hierarchy, and interaction state own
color and depth; decoration does not.

| Token Family | Use |
| --- | --- |
| PDS blue ramp | Primary actions, selected state, focus affordances, and branded anchors. Keep text-bearing primary fills dark enough for contrast. |
| Teal | Operational/info accent, secondary status emphasis, and the cool edge of accent gradients. |
| Violet | A bounded assistive-intelligence attribution accent where label, provenance, and behavior also explain AI involvement. Do not use violet as the dominant product theme. |
| Neutral canvas and surfaces | Default page and work-region structure. Dark mode uses neutral charcoal rather than a blue-violet environment. |
| Shadows and blur | Transient overlays, drag state, and genuinely elevated tools only. Routine work regions remain flat; backdrop blur is not a data-surface treatment. |
| Gradient accent | Rare, meaning-bearing progress or intelligence transition only. Global backgrounds, routine panels/controls, and always-on borders do not earn gradients. |

Gradient rules:

- treat existing `--pds-gradient-*`, glow, blur, and elevated-shadow tokens as
  current implementation surfaces to narrow as reference compositions consume
  them, not as required goal-state styling;
- use a gradient only when it clarifies a meaningful transition, priority, or
  live process and a flat treatment is less effective;
- keep raw color literals in `tokens.dtcg.json`, not generated token outputs or
  component CSS;
- keep text contrast independent from hue, especially on primary actions;
- never use an effect as the sole signal for AI, risk, success, or action;
- keep data grids, forms, navigation, and repeated work surfaces flat and calm
  by default; and
- respect high-contrast and reduced-motion settings in every treatment.

## Interaction Principles

The PDS Health design system should absorb useful modern SaaS patterns only
when they improve governed product work. The guiding standard is confidence
over complexity: users should see the state, next action, and consequence of
their work without being forced through decorative or marketing-led UI.

| Principle | Guidance |
| --- | --- |
| Calm work surfaces | Prefer stable layouts, restrained density, and quiet status treatment so repeated operational work does not feel noisy. |
| Unmistakable craft | Use familiar foundations for routine work and reserve distinctive visual, spatial, motion, and native-mobile treatment for a few signature moments rooted in the user's domain. |
| Unified command access | Provide a PDS `CommandPalette` for workspace search, frequent actions, and generated route discovery instead of scattering shortcuts across local screens. |
| Progressive disclosure | Keep advanced filters, policy detail, audit history, and secondary actions close to the task but out of the primary scan path until needed. |
| Role-aware defaults | Let tenant, role, workflow, and generated operation metadata tune what is shown first, while preserving predictable navigation and URLs. |
| Trust near action | Place validation, permission, evidence, request ID, and correlation cues beside the action they affect. Do not hide release or compliance state in separate explanatory pages. |
| Quiet intelligence | AI or agent assistance should appear as infrastructure that reduces toil, proposes next steps, or explains evidence. Do not style it as a novelty feature. |
| Human enterprise copy | Use direct, helpful microcopy for errors, empty states, and confirmations. Keep the tone warm but concise, with product-domain nouns from generated contracts. |

### Outcome-Led Experience Model

App Framework uses a layered experience model so teams can improve a real
business result without confusing a screen, workflow, or component with the
outcome:

| Layer | Meaning |
| --- | --- |
| Outcome | The user or business result that funds the work and supplies its success measure. |
| Journey | The durable path from trigger to outcome across roles, systems, waits, branches, channels, and time. A journey is not a screen flow, workflow synonym, canonical enterprise model, or new orchestration engine. |
| Experience | One role's participation at a particular journey moment, context, urgency, and channel. |
| Workspace | A persistent work environment supporting a related set of journeys. |
| Work module | A typed, bounded information, collection, control, or intelligent surface exposing journey state or capability. |
| Attention item | A durable governed signal to notice, resume, decide, or act. A toast or push notification is a delivery treatment, never the only record. |

Source systems remain authoritative for records and workflow. App Framework
owns the portable journey and experience projection, cross-system context,
exact resumption, governed-action integration, bounded workspace and module
contracts, and channel-neutral attention semantics. Intelligence may improve
every layer, but it is not a separate destination and chat is not required.
Personalization may change layout and emphasis; it may not change permission,
authority, source truth, journey semantics, evidence, or required controls.

## Composition-First Development Goals

The bounded CRM signature-composition proof is active as a product-neutral
proving ground. The broader experience-system outcomes remain held: this does
not authorize a broad frontend lane, a Nexus product slice, or component
expansion. XO may pull later outcomes only when current WIP permits and the
required product/design owners and evidence inputs exist.

| Order | Outcome | Exit evidence |
| ---: | --- | --- |
| 1 | **Production-shaped composition proofs.** Use the existing CRM Activities surface as the current bounded proving ground, then build an admin entity/operation workbench, a Nexus/My Work queue/detail with structured PDS Health AI, and a native-mobile task/approval using approved PDS content. Nexus work waits for Product Owner intake; do not invent the workflow or data. Give every flagship composition a signature experience brief. | Existing-surface and current entitled Horizon/Employee Slate baselines where applicable; complete loading/empty/error/partial/stale/offline/unauthorized states; representative-user task success, first-click, time, recovery, and trust evidence; recorded design decision. CRM evidence does not establish Nexus or vendor superiority. |
| 2 | **Prove signature experience quality.** Use familiar foundations and invest deeply in domain-specific visual objects, spatial continuity, intelligence-to-interface transformation, governed action preview, and satisfying resolution. | Two or three production-shaped signature moments in each web reference and one corresponding native-mobile moment; accessible equivalent; five- and 30-second, recall, blinded preference, semantic-differential, immediate-feedback, reduced-motion, INP, and no-regression evidence against current PDS and vendor-native baselines. Provisional bars are 65 percent preference and a one-point seven-point composite lift. |
| 3 | **One generated token source.** Adopt W3C DTCG-compatible JSON for primitive, semantic, component, motion, data-visualization, theme, high-contrast, and density tokens; generate CSS, TypeScript, React Native, and design-tool mappings. | Generated artifacts cannot drift; contrast and theme/density matrices pass; raw visual values require explicit exceptions. |
| 4 | **Six canonical floorplans.** Define admin workbench, queue/detail, entity workspace, guided process, operational overview, and intelligent work surface with responsive and native-mobile adaptations. | Admin and one real product consume the floorplans; container-level responsive tests; stable skeletons and operational states; no nested-card composition. |
| 5 | **One accessible behavior substrate.** Compare React Aria and Radix with dialog, popover, combobox, multi-select, date/time, menu, tabs, and data-grid interactions; select one while preserving the App Framework public API where practical. Use TanStack Table/Virtual behind the owned grid contract when large-data proof requires it. | Documented bakeoff and decision; WCAG 2.2 automation plus keyboard, screen-reader, zoom/reflow, reduced-motion, high-contrast, and touch evidence. |
| 6 | **Bounded intelligent composition.** Define versioned result, evidence, freshness, permission, checkpoint, and action semantics plus an allowlisted component registry, constraints, fallback, and telemetry. Useful results persist as typed application state; PDS Health AI renders inline first, while chat and notifications remain secondary entry points. | Permission/tenant preservation; evidence/freshness display; inspect/edit/accept/reject/undo telemetry; exact work continuation through useful but channel-native web/mobile rendering; no arbitrary agent-authored runtime code or transcript-owned state. A2UI/MCP Apps adapters wait for internal-contract stability. |
| 7 | **Reference products with distinct jobs.** Make admin UI the dense keyboard-efficient reference and Nexus the cross-system intelligent-work proof when product inputs are approved; keep Expo mobile genuinely native. | Measured admin tasks, large-data behavior, deep links and persistent state; measured Nexus workflow only when authorized; native navigation, interruption, connectivity, and device evidence. |
| 8 | **Executable agent design guidance.** Publish machine-readable recipes, props, states, constraints, accessibility rules, approved/prohibited uses, production imports, design-code mappings, and semantic motion roles for components and floorplans. | An agent produces a compliant page without substitute primitives or raw values; interaction/visual/motion diffs run in CI; use and exceptions are auditable. |
| 9 | **Measured experience quality gates.** Add complete-state, responsive, visual-regression, accessibility, first-impression, blinded-preference, Core Web Vitals, bundle/route, and large-data budgets plus task/intelligence telemetry. | Quality evidence is retained against a source SHA and product context; operational and signature-quality bars remain independent; failures cannot be represented as green; thresholds are calibrated from measured baselines rather than removed. |
| 10 | **Experienced product-design ownership.** Assign a product-design owner and a short, recurring user-research and design-review cadence. | Named decision ownership; usability rounds during delivery; rejected patterns update agent rules; deprecation and exception decisions are recorded. |

### Held Journey, Attention, And Workspace Outcomes

The future Nexus reference must prove more than a polished queue. Once product
inputs and source evidence exist, it should add these outcomes in order:

1. A versioned journey graph or service blueprint with source ownership,
   role/channel experience briefs, Orient-Understand-Act-Resolve moments,
   exact handoffs and exceptions, correlation, and outcome evidence.
2. A durable attention contract and preference model, an in-app attention
   center, one web or native push adapter, grouping and deduplication, expiry,
   acknowledgement and snooze, policy override, accessibility, exact deep
   links, and PDS Observability trace.
3. A bounded personal work canvas using role/admin templates, policy locks,
   versioned layouts, allowlisted modules, backend eligibility, responsive size
   classes, accessible order, migration and fallback, undo/reset, and
   appropriate cross-device persistence. AI layout proposals require rationale,
   preview, explicit acceptance, stable placement, and revert.
4. Production-shaped journey-readiness, PDS Health AI evidence-brief, and
   intelligent-queue or next-best-action modules. Each declares whether it
   informs, recommends, prepares, or acts and retains permission, evidence,
   freshness, consequence, checkpoint, recourse, deterministic fallback,
   latency, quality, inference-cost, and outcome telemetry.
5. Administrator trace and recovery views for journey correlation, attention
   policy and delivery, module eligibility, layout versions/reset, AI
   evaluation and cost, and failure recovery.

The signature references should include a Nexus personal work canvas and
durable attention center; native-mobile continuation plus an actionable
notification or ongoing-progress moment; an intelligent evidence or readiness
module; and administrator trace/reset surfaces. This is not a generic portal,
widget marketplace, journey studio, workflow engine, or arbitrary runtime UI
system.

The first decision is visual and interaction grammar, not a token migration.
Use a short design-and-code exploration to compare coherent variants in the
three compositions, choose from task evidence, and remove rejected directions.
Then make the chosen foundation enforceable. This avoids rapidly generating a
more internally consistent version of an unproven visual language.

[W3C Design Tokens Format Module](https://www.w3.org/community/reports/design-tokens/CG-FINAL-format-20251028/)
defines the target token interchange basis. [WCAG 2.2](https://www.w3.org/TR/WCAG22/)
is the accessibility baseline; automated axe evidence remains necessary but is
not complete-task assistive-technology proof.

## Component Roadmap

The inventory below describes existing component-family delivery and evidence.
It is not the next strategic producer order. Composition-first goals above
govern new experience investment; add or refactor a component only when a real
composition exposes the gap.

| Priority | Slice | Deliverables | Evidence |
| --- | --- | --- | --- |
| P0 | Token source of truth | Canonical transitional `tokens.dtcg.json` under `appfw_ui/pds_health/tokens`; generated CSS/TS outputs consumed by admin UI and CRM. | `node scripts/generate-pds-tokens.mjs --check --json`; `node scripts/check-pds-tokens.mjs --json`; `scripts/appfw framework docs-check --json`. |
| P0 | CRM-neutral base scaffold | PDS shell, generated contract import, product app identity, entity workspace, no CRM terms. | `scripts/appfw framework intake-proof --json`; `target/appfw/product-intake-proof/frontend-residue-check.json`; `target/appfw/product-intake-proof/frontend-scaffold-check.json`; product `npm run appfw:check`. |
| P1 | Core primitives | Framework-owned component source for buttons, native `ButtonLink` navigation, action menus, segmented controls, inputs, selects, lookup selectors, checkboxes, switches, tabs, dialogs, alerts, app shell, command bar, command palette, and data grid shell; reference fixtures consume shared PDS components instead of parallel local renderers. | `scripts/check-pds-components.mjs --json`; catalog coverage evidence; representative frontend typecheck/build evidence. |
| P1 | Work surfaces | App shell, page header, command bar, command palette, surface panels, full-surface `InteractiveCard`/`CardLink`/`WorkQueueItem` activation, form layout, field wrapper, generated field metadata, field group, feedback states, inline alerts, banners, toast region, empty state, validation summary, and a product-neutral PDS Component Catalog with agent-readable manifest, visible source API map, visible decision guide, visible readiness ledger, and family readiness contracts. | Product scaffold checks plus reference fixture screens; `node scripts/serve-pds-reference.mjs`; `target/appfw/pds-component-check.json`. |
| P1 | Generated form controls | Native date, time, datetime, input-group, multi-select, lookup, switch, checkbox, file-upload, and stable form loading preview controls with token-backed focus, invalid, disabled, hint, and error states. | PDS Component Catalog coverage, generated date/time/form-loading usage, `scripts/check-pds-components.mjs --json`, representative frontend typecheck/build evidence. |
| P1 | Data components | Data grid shell, loading preview, toolbar, control popover shell, filter panel/group/rule structure, filter trigger, column chooser and trigger, resize handle, density selector, sortable header control, and pagination with selected rows, keyboard row activation, column sizing, compact density, sort/filter panel, empty/error states, and lookup selector. | Reference fixture grid/form/lookup tests; generated contract tests; `scripts/check-pds-components.mjs --json`. |
| P1 | Overlay components | Dialog, drawer, popover, popover trigger, tooltip, and confirm-dialog shells with focus return, trapped modal focus, escape handling, native popover compatibility, token-backed overlay styling, and actual frontend consumers. | PDS Component Catalog coverage, component accessibility patterns, representative about/delete/drawer wiring, `scripts/check-pds-components.mjs --json`, frontend build checks. |
| P2 | Governed action primitives | Intent preview, action audit, and undo/compensation state primitives for policy-sensitive writes and agent-assisted actions. Products own authorization, confirmation copy, write execution, and compensation behavior; PDS owns the review, audit, and status surfaces. The primitives are not write-certification evidence by themselves: live execution requires G1 governed-write posture/evidence and U2 agent-harness approval for the provider and operation. | PDS Component Catalog coverage, governed-action decision recipe, `scripts/check-pds-components.mjs --json`, `target/appfw/governed-write-posture.json`, `target/appfw/governed-write-evidence.json`, `.appfw/target/appfw/harness-check.json`, and downstream governed-write evidence before live certification. |
| P2 | Process flows | Process stepper and progress controls for intake, approval, certification, release, and generated workflows, with independent current and `selectedStepId` state, full-surface native step selection, current/complete/upcoming/warning/blocked states, and compact mobile progress variants. | PDS Component Catalog coverage, `scripts/check-pds-components.mjs --json`, `scripts/check-pds-catalog-evidence.mjs --json`, process accessibility evidence, and representative workflow visual/a11y review evidence. |
| P2 | Analytics | KPI tiles, metric trend states, chart shells, chart legends, status palettes, `TimelineRangeSelector` move-resize-snap interaction, and presentational SVG chart renderers (`BarChart`, `LineChart`, `AreaChart`, `DonutChart`) that draw a provided series without bundling a charting engine; products own data, calculations, time-domain labels and density, query behavior, and thresholds. | PDS Component Catalog coverage, representative dashboard `KpiTile` and timeline-range usage, `scripts/check-pds-components.mjs --json`, `scripts/check-pds-catalog-evidence.mjs --json`, dashboard E2E and visual/a11y review evidence. |
| P2 | Conversation workspace | Responsive `ConversationWorkspace` composition over the lower-level conversation family, with named thread/context/action regions, suggested prompts, streaming/busy state, and disabled composer behavior while busy. Products own messages, prompts, retrieval, model/runtime calls, context content, action meaning, and write gates. | PDS Component Catalog coverage, conversation accessibility and answer-envelope contracts, `scripts/check-pds-components.mjs --json`, `scripts/check-pds-catalog-evidence.mjs --json`, and representative product visual/a11y evidence. |
| P2 | Ambient AI | Generated view shells, suggested-action/recommendation cards, evidence/insight summaries, freshness indicators, attention markers, AI attribution affordances, per-task assist-level control, and visible/correctable memory chips for ambient assistance. Products own resolved refs, viewRegistry bindings, answer-envelope claims, policy decisions, assist behavior, memory storage/correction, and all execution; PDS owns the grounded presentation shell. | PDS Component Catalog coverage, `agentic-ux-grounding`, answer-envelope contract, viewRegistry contract, chat-eval posture, `scripts/check-pds-components.mjs --json`, and future CH8 consumer-wiring/visual/a11y evidence before any release-gated claim. |
| P2 | Governance (delivered) | SemVer + CHANGELOG, per-component lifecycle status/`since`, deprecation rules, consumer wiring checks, and a `release_certification` summary. See [Versioning, Lifecycle, And Release Certification](#versioning-lifecycle-and-release-certification). | `package-versioning` and `component-lifecycle` checks in `scripts/check-pds-components.mjs --json`; `release_certification` block in `target/appfw/pds-component-check.json`; `appfw_ui/pds_health/CHANGELOG.md`. |

## CRM-Neutral Scaffold Rules

Generated or profile-created product frontends must:

- derive app name, schema names, routes, and storage keys from product metadata;
- use PDS design-system imports or vendored files with provenance;
- avoid CRM domain strings, entity names, class prefixes, and fixtures;
- include `.appfw-ui/ownership.json` and `.appfw-ui/scaffold-manifest.json`;
- expose `npm run appfw:check` for offline scaffold validation;
- use generated UI contracts for entity/field/operation metadata; and
- document missing frontend evidence before product handoff.

The CRM reference frontend may keep CRM-specific features, but those features
must live in CRM-owned modules and should consume the same shared PDS assets.

## Implementation Plan

1. Maintain `appfw_ui/pds_health/tokens/tokens.dtcg.json` as the canonical
   transitional token source. Edit it first, generate `pdsTokens.css` and
   `pdsTokens.ts` with `node scripts/generate-pds-tokens.mjs`, and keep both
   outputs byte-identical to regeneration.
2. Keep admin UI and CRM consuming the generated outputs through shared PDS
   package aliases, with `node scripts/generate-pds-tokens.mjs --check --json`
   and `node scripts/check-pds-tokens.mjs --json` retaining drift evidence.
3. Establish framework-owned component source under
   `appfw_ui/pds_health/components`, guarded by
   `scripts/check-pds-components.mjs --json`.
4. Maintain `appfw_ui/pds_health/reference/catalog.json` as the
   agent-readable component manifest, `reference/index.html` as the fast
   static visual reference, and `catalog-app` as the interactive renderer for
   live product-neutral PDS component composition.
5. Promote modern SaaS interaction patterns into product-neutral components
   only when they support calm work surfaces, progressive disclosure, unified
   command access, and trust cues near action.
6. Keep the generated base frontend scaffold for `product-intake` aligned with
   the PDS shell, entity workspace, generated UI contract, ownership
   manifests, package scripts, and release evidence.
7. Keep the residue check in `framework intake-proof` and generated
   `npm run appfw:check` so a new product scaffold fails when CRM terms are
   present outside explicit reference docs.
8. Keep CRM as the E2E fixture by rebasing CRM screens onto shared primitives
   and retaining tests for grids, forms, lookups, dashboard, and accessibility.
9. Extend framework release certification to report design-system version,
   token drift, component test status, scaffold residue status, and CRM E2E
   coverage.

Current state:

1. Shared overlay primitives now render at body scope, have named close
   actions, alert-dialog confirmation semantics, focus return, escape handling,
   keyboard-focusable scroll bodies, and consumer wiring checks for CRM
   about/delete surfaces and admin record/model/console/about overlays. The
   admin UI now also consumes the shared `Button`, `IconButton`, and `Badge`
   primitives across its shell, developer console, record drawer, and advanced
   filter builder (not just container components), proving design-system
   adoption beyond the CRM reference app; consumer-wiring checks retain this.
   (Admin form-field inputs remain a follow-on convergence step, since they
   need live rendering to verify.)
2. Shared analytics primitives now include `KpiTile`, `MetricTrend`,
   `ChartShell`, `ChartLegend`, `TimelineRangeSelector`, and presentational SVG
   chart renderers (`BarChart`, `LineChart`, `AreaChart`, `DonutChart`), with
   static reference coverage, interactive catalog chart variations, and CRM
   dashboard and account-health KPI/chart-panel usage; products supply
   timeline labels, density values, snap choices, and query behavior, and CRM
   frontend evidence asserts that the revenue chart canvas remains contained
   by its shared chart shell.
3. Shared process primitives now include `ProcessStepper` and
   `ProcessProgress` for guided intake, approval, certification, release, and
   generated workflow progress. `ProcessStepper` supports `selectedStepId`
   independently from the current workflow step and exposes the marker and copy
   as one native selectable surface.
4. Shared work-surface and feedback primitives now cover the CRM
   account-health hero shell and dashboard/account-health warning states.
   `ButtonLink`, `InteractiveCard`, `CardLink`, and `WorkQueueItem` provide the
   public native link-versus-command and full-surface activation contracts;
   product routes, copy, status meaning, and side effects remain outside PDS.
5. Shared work-surface primitives now cover generated record edit forms,
   generated form loading previews through `FormLoadingPreview`, and the CRM
   audit placeholder; generated record form actions now consume the shared
   `Button` primitive, and the CRM-local card wrapper has been removed from the
   reference frontend.
6. Shared `Skeleton` now includes a card variant, `DataGridLoadingPreview`
   reserves dense grid rhythm, and simple CRM dashboard/account-health loading
   tiles no longer require CRM-local placeholder markup.
7. Product-intake frontends now start with CRM-neutral PDS overlay, analytics,
   data-grid, and generated-form examples, and both the scaffold-local
   `npm run appfw:check` and framework intake proof fail if those examples or
   PDS aliases are removed.
8. Generated frontends now also receive a contract-driven entity-workspace
   starter (`src/generated/appfw-entity-workspace.tsx`, emitted by
   `app_gen/src/frontend.rs`) that renders a list workspace — page header,
   toolbar, data grid, pagination, and loading/empty/error/denied feedback
   states — wired to the shared PDS components straight from the generated UI
   contract. A new product gets a working, PDS-wired, product-neutral workspace
   on day one; products own data fetching and pass rows and handlers in. It is
   verified by `scripts/appfw generate --check` plus the reference frontend
   `typecheck`.
9. `scripts/appfw framework handoff --json` now summarizes the retained PDS
   component check, including package version, source hash, token hash, and
   required-component count.
10. `scripts/check-pds-components.mjs --json` retains component contract,
   accessibility-pattern, reference-preview, reference-component-catalog,
   reference-enterprise-catalog-contract,
   reference-agent-catalog-manifest, source-export-catalog-drift,
   package-catalog-api, reference-agent-api-source-map,
   reference-agent-decision-guide, reference-agent-readiness-ledger,
   interactive-catalog-app, consumer-wiring, density-policy, overlay
   scroll-focus, component API inventory, package catalog summary, interactive
   catalog hash, agent decision guide, catalog-manifest-hash,
   family-readiness, governed-action live-readiness, FlowGraph token bridge
   posture, conversation markdown sanitizer posture, source-hash, token-hash,
   and package-version evidence.
11. `scripts/check-pds-catalog-evidence.mjs --json` retains browser-backed
   interactive catalog evidence at `target/appfw/pds-catalog-evidence.json`,
   with screenshots under `target/appfw/pds-catalog-evidence/screenshots/`.
   It covers desktop/mobile light/dark scenarios, axe serious/critical
   violations, layout overflow, duplicate IDs, unnamed controls, chart
   variation rendering, process-flow examples, Conversation live-region and
   grounding surfaces, and Ambient AI generated-view/preview-gating/
   attribution/freshness/attention semantics.
12. Shared conversation primitives now include `ConversationWorkspace` as the
    responsive invoked-assistance floor plan. It owns named thread, context,
    and action regions plus busy/composer mechanics while product/runtime code
    owns messages, prompts, retrieval, models, actions, context content, and
    write gates.
13. Experimental `NarrativeWorkspace` now supplies a document-scrolling
    knowledge floor plan with a sticky compact banner, floating horizontal
    experience dock, optional labelled context region, skip link, and measured
    sticky offset. The PDS Technology Strategy app is its first proving
    consumer. It does not replace the six canonical work floor plans or make a
    universal no-sidebar claim.

Next execution focus, only when the active program explicitly releases this
held objective:

1. Prove and select the experience grammar through the three north-star
   compositions, signature experience briefs, measured tasks, and comparative
   signature-quality evidence; do not begin with a repository-wide restyle or
   token rewrite.
2. Prove the domain-specific visual objects and signature moments with both
   operational and blinded-preference bars, then feed accepted and rejected
   treatments into the shared system.
3. Migrate the transitional W1a token source to native DTCG token groups while
   adding density/theme/high-contrast modes, six floorplans, and selected
   accessible behavior substrate behind stable App Framework APIs, refactoring
   components only as the chosen compositions use them.
4. Prove bounded PDS Health AI composition, admin workbench quality, and native
   mobile behavior with visual, interaction, accessibility, performance, and
   product-outcome evidence before broad generator rollout.
5. Convert accepted and rejected patterns into machine-readable agent recipes,
   design-code mappings, exception rules, and CI evidence so scale follows
   coherent design rather than preceding it.

The active security/runtime readiness correction retains priority. No item
above authorizes Nexus requirements, live provider behavior, production claims,
or a second remote promotion train.

## Governance

Design-system changes are framework-owned API changes. Treat every component,
token, and scaffold change as a contract surface unless the file is explicitly
reference-only.

Component migration rules:

- migrate one visible workflow family at a time, such as forms, overlays, data
  grids, analytics, or feedback states;
- keep product-owned business logic, chart calculations, and generated API
  contracts outside the shared component package;
- preserve existing accessible names, roles, keyboard paths, and E2E selectors
  unless the test contract is intentionally changed; and
- add `scripts/check-pds-components.mjs --json` consumer-wiring checks for each
  migrated CRM, admin UI, or scaffold surface.

Deprecation rules:

- do not remove a local component shell until at least one shared replacement is
  in the reference, one real consumer has migrated, and the checker records the
  replacement path;
- keep deprecated local CSS only while consumers still need it, and remove it
  once no source file references the old class family; and
- document any intentional visual break in this file and in release notes when
  a product or framework user may notice it.

Version and evidence rules:

- component-package `package.json` version, source hash, and token hash are
  retained in `target/appfw/pds-component-check.json`;
- release-relevant changes should include the component check artifact in
  framework handoff evidence, where `PDS component check` records package
  version, required-component count, source hash, token hash, interactive
  catalog live-example count, product-neutral status, and interactive catalog
  hash; and
- product releases should name the PDS component import, style import, and any
  local exceptions where product-specific UI remains outside the shared system.

## Release Evidence

Design-system work is release-relevant when it changes tokens, components,
generated frontend scaffolds, or reference app behavior.

Minimum evidence:

```bash
scripts/check-pds-tokens.mjs --json
scripts/check-pds-components.mjs --json
scripts/check-pds-catalog-evidence.mjs --json
scripts/appfw framework docs-check --json
scripts/appfw framework validate --json
scripts/appfw framework generate --check --json
scripts/appfw framework test --fast --json
```

The component check also verifies that
`appfw_ui/pds_health/reference/catalog.json` covers the exported component
manifest and source API contract, `appfw_ui/pds_health/reference/index.html`
imports shared component styles, the visible catalog covers the core
product-neutral component class surface, source API map, agent decision guide,
and family maturity/evidence ledger, every source component export has a
matching props type and catalog entry, the package catalog exports remain
aligned with the manifest, the interactive catalog renders the same manifest
and real component source with live examples and usage snippets, and the
reference surfaces remain free of CRM domain residue.

For local visual review, run:

```bash
node scripts/serve-pds-reference.mjs
```

Then open `http://127.0.0.1:5175/reference/`.

For live component review, run:

```bash
cd appfw_ui/pds_health/catalog-app
npm run dev
```

For retained interactive catalog screenshots and accessibility evidence, run:

```bash
scripts/check-pds-catalog-evidence.mjs --json
```

When product scaffold behavior changes, also retain:

```bash
scripts/appfw framework intake-proof --json
scripts/appfw framework golden-downstream --json
scripts/appfw product frontend-test --json
npm run appfw:check
```

`framework intake-proof` retains
`target/appfw/product-intake-proof/frontend-residue-check.json` and
`target/appfw/product-intake-proof/frontend-scaffold-check.json`. Generated
product frontends retain local `npm run appfw:check` evidence at
`frontend/target/appfw/frontend-scaffold-check.json`. The product-intake
starter must include PDS overlay, analytics, data-grid, and generated-form
examples without sample CRM terms.

When shared component source changes, also retain the frontend package checks
for every touched consumer:

```bash
npm run appfw:check
npm run typecheck
npm run test
npm run build
```

Skipped live checks must be named in `scripts/appfw framework handoff --json`
with the dependency that was unavailable.
