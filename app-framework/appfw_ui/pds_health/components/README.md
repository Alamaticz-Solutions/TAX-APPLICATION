# PDS Health Component System

This inventory defines the first-class component families expected from the PDS
Health enterprise design system. Component source should remain readable,
agent-editable, accessible by default, and aligned with generated model/API
contracts.

## Current Package Surface

The framework-owned `@appfw/pds-health-components` package is compiled to ESM
with declaration files. Product code imports public family subpaths such as
`@appfw/pds-health-components/layout`,
`@appfw/pds-health-components/primitives`, and
`@appfw/pds-health-components/surfaces`; shared styling is imported from
`@appfw/pds-health-components/styles.css`. Checkout-relative imports from
`components/src` are not a supported consumer contract.

The `0.12.0` Intelligent Experience APIs are deliberately absent from the
package root. Import presentation anatomy from
`@appfw/pds-health-components/intelligence-presentation`, portable model
types from `@appfw/pds-health-components/intelligence-presentation-model`,
and all eight recipe identities from
`@appfw/pds-health-components/ix-recipes`. Use
`resolvePdsIxWebRecipe` to validate and project a product-owned recipe
registration, then render caller-owned `pds.ix.presentation@1` data through
the generic `PdsIxRecipePresentation`. The package does not export eight
demo-shaped components or the catalog's private fixture payloads. Recipe
capabilities are compatibility obligations, not evidence that behavior ran.
Core package installation does not need the optional
`@appfw/pds-ix-presentation-contract@0.2.0` peer. Any consumer importing one
of these IX subpaths must install that exact peer. Packed manifests contain no
checkout-relative or `file:` dependency.

Each presentation revision has one channel-neutral announcement. The Web
recipe renderer owns its single live-region emission and suppresses nested
emission from composed response, context, and status primitives. Those
primitives announce by default when used standalone; callers composing their
own single announcement owner can set `announce={false}`. Duplicate response
region IDs are rejected before rendering, and evidence DOM IDs are scoped per
component instance.

ADR 0016 establishes React Aria Components as the default commodity web
interaction substrate behind the PDS package. Products import PDS components,
never React Aria directly. PDS continues to own public APIs, semantics,
canonical design data, apple-like/material-like expression, motion, product
patterns, intelligent-work capabilities, and evidence. `Tabs` and `Dialog`
are the first retained-evidence proof; other controls migrate only through
Nexus-shaped vertical slices.

Ratified visual-language contracts:

- `visual-themes.md` defines Apple-like and Material-like as orthogonal,
  token-backed visual themes that both support System, Light, and Dark modes.
- `material-3-web-contract.md` defines the fixed PDS Material 3 role system,
  M3/MUI authority order, web component coverage, and stable-geometry rule.
- `precision-daylight-visual-language.md` defines the luminous light-mode
  hierarchy and surface treatment implemented by the default Apple-like theme.
- `chromatic-signal-visual-language.md` defines the split between accessible
  semantic ink and vivid operational/data signals.
- `connected-fabric-visual-language.md` defines the restrained connection
  topology and motion language used by the interactive catalog exemplar.
- `connected-fabric.accepted-baseline.json` protects the exact accepted
  Connected Fabric implementation contract.
- `ConnectedFabric` is the compiled decorative application-fabric primitive.
  Import it from `@appfw/pds-health-components/connected-fabric`; products own
  placement plus the documented content, panel, and scroll selectors.
- `nexus-readiness.json` makes the foundation, interaction, My Work,
  trusted-task, intelligence, native, and package-consumer slices
  machine-readable.

The typography and component-motion proposal remains review-only in
`ux-foundation-draft-spec.md` until its payload and product proof gates are
accepted.

Current exports:

```ts
import {
  Alert,
  AppShell,
  AiAttributionAffordance,
  Badge,
  Banner,
  Breadcrumbs,
  Button,
  ButtonLink,
  ButtonGroup,
  CardLink,
  ToggleButton,
  FloatingActionButton,
  CheckboxField,
  ChartLegend,
  ChartShell,
  CommandBar,
  CommandPalette,
  ConfirmDialog,
  ConversationWorkspace,
  DataGridColumnChooser,
  DataGridColumnChooserTrigger,
  DataGridColumnResizeHandle,
  DataGridControlPopover,
  DataGridDensityControl,
  DataGridFilterEmpty,
  DataGridFilterGroup,
  DataGridFilterPanel,
  DataGridFilterRule,
  DataGridFilterTrigger,
  DataGridLoadingPreview,
  DataGridPagination,
  DataGrid,
  DataGridShell,
  DataGridSortButton,
  DataGridToolbar,
  DateField,
  DatePicker,
  DateTimeField,
  Dialog,
  Drawer,
  EmptyState,
  ErrorState,
  ExplorationWorkspace,
  EvidenceSummary,
  Field,
  FieldMetadata,
  FileUpload,
  FeedbackState,
  FieldGroup,
  FormLoadingPreview,
  FormLayout,
  ForbiddenState,
  FreshnessIndicator,
  GeneratedViewShell,
  IconButton,
  InlineAlert,
  InputGroup,
  InteractiveCard,
  KpiTile,
  LoadingState,
  LookupSelect,
  MenuButton,
  MetricTrend,
  MultiSelect,
  NarrativeWorkspace,
  PdsHealthLogo,
  List,
  ListItem,
  OperationState,
  PageHeader,
  MemoryChip,
  Popover,
  PopoverTrigger,
  ProcessProgress,
  ProcessStepper,
  pdsAgentDecisionGuide,
  pdsComponentCatalog,
  pdsComponentFamilies,
  RecommendationCard,
  SegmentedControl,
  SelectField,
  Skeleton,
  SuggestedAction,
  Surface,
  SearchBar,
  SwitchField,
  Tabs,
  TextArea,
  TextField,
  TimelineRangeSelector,
  TimeField,
  TimePicker,
  Toolbar,
  Tooltip,
  Toast,
  ToastRegion,
  AssistLevelControl,
  ValidationSummary,
  WorkQueueItem
} from "@appfw/pds-health-components";
```

Prefer a focused family import in product code:

```ts
import { AppShell, Breadcrumbs } from "@appfw/pds-health-components/layout";
import { PdsHealthLogo } from "@appfw/pds-health-components/foundation";
import { Button, SegmentedControl } from "@appfw/pds-health-components/primitives";
import { ProcessStepper } from "@appfw/pds-health-components/process";
import { ConversationWorkspace } from "@appfw/pds-health-components/conversation-workspace";
import { ConnectedFabric } from "@appfw/pds-health-components/connected-fabric";
import { NarrativeWorkspace } from "@appfw/pds-health-components/narrative-workspace";
import { ExplorationWorkspace } from "@appfw/pds-health-components/exploration-workspace";
import { RelationshipExplorer } from "@appfw/pds-health-components/relationship-atlas";
import { TimelineRangeSelector } from "@appfw/pds-health-components/timeline";
import { ButtonLink, CardLink, WorkQueueItem } from "@appfw/pds-health-components/work-surfaces";
import "@appfw/pds-health-components/styles.css";
```

Compiled public family subpaths are `ambient`, `catalog`, `charts`,
`conversation`, `conversation-workspace`, `connected-fabric`, `copy`, `data`, `data-grid`,
`experience`, `exploration-workspace`, `forms`, `ix-recipes`, `layout`, `narrative-workspace`, `primitives`, `process`,
`relationship-atlas`, `surfaces`, `timeline`, `types`, and `work-surfaces`. The root package, `styles.css`, and
`tokens.css` are also public entrypoints. Products must not import files below
`components/src` or reach through the package to React Aria or AG Grid.

## Promoted 0.9.0 Narrative Floor Plan

- `PdsHealthLogo` preserves the approved horizontal wordmark and compact mark
  as responsive vector artwork. Products choose the constrained/full variant
  and whether an adjacent label makes it decorative; they do not redraw it.

- Experimental `NarrativeWorkspace` provides one document-scrolling
  knowledge floor plan with a persistent compact banner, floating horizontal
  experience dock, optional labelled context band, utility and footer slots,
  skip link, and measured sticky offset for unobscured anchors and focus.
- PDS owns layout, landmarks, sticky geometry, safe-area and responsive
  behavior, theme grammar, forced-colors fallback, and print removal. Products
  own navigation controls, route or same-page mode semantics, current state,
  role/context meaning, content, anchors, history, and focus restoration.
- This is a product-proven experimental composition, not a replacement for
  `AppShell`, local object navigators, supporting panes, or process steppers.

## Promoted 0.8.0 Navigation Contracts

- Experimental `ExplorationWorkspace` provides labelled navigator, focus,
  inspector, controls, and evidence regions with a bounded desktop floor plan
  and flow-based narrow/print fallback. Products retain object semantics,
  access, selection, filtering, relationships, proof, and URL state.

- Experimental `RelationshipExplorer` composes the relationship map with an
  equivalent accessible table. `tablePageSize` bounds large table views to at
  most 100 rows per page, resets when the object set changes, and follows a
  selected object to its page. Products retain graph meaning, evidence,
  citations, filtering, and selected state.

- Expanded `NavigationItem` labels use a semantic 14 px / 20 px navigation
  type role and wrap to two lines by default. Fixed icon and trailing columns
  keep labels and count badges aligned; `labelBehavior="truncate"` is reserved
  for intentionally compact presentations.
- When a wrapped or truncated navigation label still cannot fit,
  `NavigationItem` discloses the complete label on pointer hover and keyboard
  focus. The tooltip is measured, is not shown when the label fits, renders
  above clipping scroll containers, and dismisses with Escape.
- `AppShell` can expose an adjustable desktop sidebar with
  `sidebarResizable`. The default/minimum/maximum width contract is
  288/240/360 px, additionally bounded to 35% of the viewport. Products may
  control and persist the width through `sidebarWidth`,
  `onSidebarWidthChange`, and `onSidebarWidthCommit`.
- The resize separator supports pointer input plus
  `ArrowLeft`/`ArrowRight`/`Home`/`End`, publishes separator value semantics,
  and is removed when the responsive drawer presentation takes over.
- PDS owns navigation typography, icon geometry, label wrapping, exceptional
  overflow disclosure, resizing mechanics, and the responsive threshold.
  Products continue to own labels, destinations, selected route, role
  visibility, counts, and persistence keys.

## Promoted 0.7.0 Contracts

- `ButtonLink` is the native-anchor counterpart to `Button`. Use it for
  location changes and keep `Button` for commands.
- `InteractiveCard`, `CardLink`, and `WorkQueueItem` provide full-surface
  anchor or button activation, selected/disabled/completed states, and sibling
  secondary actions without nested interactive controls. Products own
  destinations, commands, copy, status meaning, and side effects.
- `ProcessStepper` accepts `selectedStepId` independently from the current
  workflow step. The marker and copy are one native selectable surface;
  products own filtering, workflow rules, and transition effects. Use
  `variant="milestone"` for a compact timeline whose shared treatment keeps
  connectors centered on markers and emphasizes selection through the marker
  and label instead of a card-like selection box.
- `ConversationWorkspace` owns the responsive conversation floor plan, named
  thread/context/action regions, prompt grouping, and busy composer behavior.
  Products own messages, prompts, retrieval, model calls, context content,
  actions, and write gates.
- `TimelineRangeSelector` owns one movable and edge-resizable time window,
  snapping, pointer/keyboard operation, and range announcements. Products own
  ordered time items, labels, density values, snap-width choices, query
  behavior, and interpretation.
- `Surface` clips decorative and interactive content by default. Set
  `overflow="visible"` only when the surface intentionally hosts a PDS
  anchored overlay such as `SearchBar`; product-local overflow overrides are
  not part of the supported contract.

These contracts adapt their layout without changing product behavior at
responsive breakpoints. Their emphasis and movement are nonessential;
reduced-motion preferences remove or shorten transition effects while
preserving focus, selection, busy, and progress semantics.

The current review-only typography and motion proposal is documented in
[UX Foundation Draft Spec](ux-foundation-draft-spec.md). It is not a ratified
component contract until its payload, visual, accessibility, and product proof
gates are accepted.

Use this package as the convergence target for generated product frontends and
framework-owned operational surfaces. Do not copy product-local class names,
domain nouns, storage keys, routes, or workflow fixtures into this source.

## Catalog And Evidence

The component catalog has three connected parts:

- the source inventory in `components/src`, where `src/index.ts` exports the
  framework-owned React APIs;
- the package catalog API in `components/src/catalog.ts`, where generators and
  product agents can import family and recipe metadata; and
- the PDS Component Catalog in `../reference`, where `catalog.json` gives
  agents a machine-readable family/component manifest, maturity level, product
  boundary, source API map, agent decision guide, and evidence requirements,
  while `index.html` shows product-neutral families with states, density,
  source ownership, decision recipes, accessibility, usage, and verification
  guidance; and
- the unified PDS Design System application in `../catalog-app`, whose root
  retains persistent navigation across Overview, Brand, Floor plans, Elements,
  Components, Patterns, Data visualization, and reference proofs. Its
  Components view renders the same manifest and real component source through
  React so agents can search the catalog, inspect live examples, compare
  Apple-like and Material-like in System, Light, or Dark mode, and copy imports
  or usage snippets without copying product fixture language.

`scripts/check-pds-components.mjs --json` ties those together. A component that
is exported from `components/src` but absent from the manifest or visible
catalog fails the check, a component without a matching `<ComponentName>Props`
export fails the check, and a family that lacks the enterprise API-reference
contract also fails the check. The retained evidence includes
`component_api_inventory`, which maps each component to its family, source file,
and props type. It also includes `agent_decision_guide`, which maps common
product workflow intents to the shared PDS components agents should start with.
The checker also verifies that `pdsComponentCatalog`, `pdsComponentFamilies`,
and `pdsAgentDecisionGuide` stay aligned with the reference manifest. It also
gates the interactive catalog so every exported component has a live example,
every component has a copy-ready usage snippet, the catalog imports the real
PDS token/component source, and the catalog source remains product-neutral.

| Surface | Responsibility | Evidence |
| --- | --- | --- |
| `appfw_ui/pds_health/tokens/tokens.dtcg.json` | Canonical transitional PDS token source; edit first and generate CSS/TS outputs. | `node scripts/generate-pds-tokens.mjs --check --json`; `node scripts/check-pds-tokens.mjs --json`. |
| `appfw_ui/pds_health/tokens/pdsTokens.css` and `pdsTokens.ts` | Generated token outputs consumed through PDS package aliases; do not edit directly. | Generator drift check plus consuming frontend token checks. |
| `appfw_ui/pds_health/components/src` | Framework-owned React component APIs, typed package catalog API, and token-backed CSS. No product domain nouns. | `scripts/check-pds-components.mjs --json`; package typecheck/build coverage as it matures. |
| `appfw_ui/pds_health/reference` | Product-neutral PDS Component Catalog covering the agent-readable manifest, visible component families, source API map, agent decision guide, states, density, themes, accessibility notes, usage examples, maturity levels, evidence requirements, and the family readiness matrix. | `scripts/check-pds-components.mjs --json`; `node scripts/serve-pds-reference.mjs`. |
| `appfw_ui/pds_health/catalog-app` | Unified PDS Design System application. The root is the full portal; the searchable component catalog is its Components view. It consumes the reference manifest and real component source and must not define a second source of truth. | `scripts/check-pds-components.mjs --json`; `scripts/check-pds-catalog-evidence.mjs --json`; `npm run typecheck`; `npm run build`. |
| Product-intake frontend starter | Generated app shell and scaffold examples consuming PDS components without product residue. | `scripts/appfw framework intake-proof --json`; generated `npm run appfw:check`; `scripts/check-pds-components.mjs --json`. |
| Operational frontend consumers | Framework-owned applications consuming the same tokens and components without redefining the design system. | Consumer build checks plus `scripts/check-pds-components.mjs --json`. |

## Component Families

| Family | Components | Maturity Gate |
| --- | --- | --- |
| Actions | `Button`, `ButtonLink`, `ToggleButton`, `FloatingActionButton`, `ButtonGroup`, `IconButton`, `MenuButton`, `Toolbar`, `CommandBar`, `CommandPalette` | Material hierarchy, native link-versus-command semantics, loading/disabled/denied states, stable selected geometry, tooltip support, grouped overflow actions, clear destructive styling, unified command access. |
| Forms | `Field`, `FieldMetadata`, `FormLoadingPreview`, `TextField`, `TextArea`, `SelectField`, `DateField`, `TimeField`, `DateTimeField`, `DatePicker`, `TimePicker`, `InputGroup`, `MultiSelect`, `FileUpload`, `CheckboxField`, `SwitchField`, `LookupSelect`, `ValidationSummary` | Generated field metadata, animated labels with stable shells, native and rich date/time controls, lookup loading/empty/error states, array selection, file evidence inputs, dirty state, field errors, validation summary, accessible labels, and stable loading previews. |
| Data Grid | `DataGrid`, `DataGridShell`, `DataGridLoadingPreview`, `DataGridToolbar`, `DataGridColumnChooser`, `DataGridColumnChooserTrigger`, `DataGridColumnResizeHandle`, `DataGridControlPopover`, `DataGridDensityControl`, `DataGridFilterPanel`, `DataGridFilterGroup`, `DataGridFilterRule`, `DataGridFilterEmpty`, `DataGridFilterTrigger`, `DataGridSortButton`, `DataGridPagination` | `DataGrid` wraps AG Grid Community for high-capability operational grids; `DataGridShell` remains the lightweight semantic table. Products own server queries, permissions, persisted preferences, and row action meaning. |

`DataGrid` is the only supported AG Grid entry point. Product code must not
import `ag-grid-community`, `ag-grid-react`, AG theme objects, or AG CSS classes.
The adapter maps one PDS contract into Apple-like and Material-like render
grammars and inherits light/dark semantic tokens.
| Feedback | `Alert`, `Banner`, `FeedbackState`, `LoadingState`, `ErrorState`, `ForbiddenState`, `InlineAlert`, `EmptyState`, `Skeleton`, `Toast`, `ToastRegion` | Request/correlation IDs, retry affordance, policy-denied clarity, accessible live-region behavior. |
| Overlays | `Dialog`, `Drawer`, `Popover`, `PopoverTrigger`, `Tooltip`, `ConfirmDialog` | Body-level portal rendering, focus trap, escape handling, return focus, screen-reader labels, native popover compatibility, and clear destructive confirmation styling. |
| Navigation | `AppShell`, `NarrativeWorkspace`, `ConnectedFabric`, `ExplorationWorkspace`, `AppearanceProvider`, `PdsHealthLogo`, `NavigationItem`, `IconSlot`, `Avatar`, `IdentitySummary`, `Breadcrumbs`, `PageHeader`, `Surface`, `List`, `ListItem`, `SearchBar`, `Tabs`, `SegmentedControl`, `Badge`, `InteractiveCard`, `CardLink`, `WorkQueueItem` | Responsive and keyboard-adjustable app structure; governed brand artwork; labelled exploration regions; readable two-line navigation labels with exceptional overflow disclosure; fixed, centered icon geometry; non-shrinking counts; decorative application-fabric background; persisted appearance preferences; compact identity composition; page context; full-surface native activation; sibling secondary actions; dynamic search; list selection; selected route/card/row or tab state; grouped pressed-state controls; and keyboard behavior. PDS owns navigation presentation and resize mechanics; products own routes, labels, identity values, role visibility, counts, selectors, and persistence policy. |
| Process | `ProcessStepper`, `ProcessProgress` | Horizontal and vertical stepper flows, compact bar/dot progress, independent current and selected steps through `selectedStepId`, full-surface step selection, centered milestone timelines through `variant="milestone"`, current/blocked/warning/complete states, and accessible progress metadata. |
| Analytics | `KpiTile`, `MetricTrend`, `ChartShell`, `ChartLegend`, `TimelineRangeSelector` | Chart-engine-neutral shell, color-safe status palettes, accessible movable/resizable timeline windows, loading/empty/error states, multiple catalog chart variations, and at least one representative dashboard consumer. |
| Conversation | `ConversationWorkspace`, `MessageThread`, `Message`, `MessageComposer`, `StreamingText`, `ToolCallStatus`, `EntityRefCard`, `CitationList`, `ConfidenceSignal`, `AgentTimeline`, `FlowGraphShell` | Responsive named regions, ordered messages, grounded answer detail, busy and streaming semantics, disabled prompts/composer while busy, and product-owned messages, models, retrieval, context, and action policy. |
| Ambient AI | `GeneratedViewShell`, `SuggestedAction`, `RecommendationCard`, `EvidenceSummary`, `InsightSummary`, `FreshnessIndicator`, `AttentionMarker`, `AiAttributionAffordance`, `AssistLevelControl`, `MemoryChip` | Grounded generated views, preview-gated recommendations, evidence summaries, freshness and attention cues, visible attribution, per-task autonomy selection, and correctable personalization affordances. Products own resolved refs, policy, memory storage/correction, assist behavior, and execution. |

## API Rules

- Prefer simple component props that mirror platform concepts: role, tenant,
  entity, operation, validation, request ID, and correlation ID.
- Do not hide generated contracts behind opaque configuration objects.
- Export small primitives and product-ready composites.
- Keep class names and storage keys product-neutral; derive product identity
  from app manifest metadata.
- Use icons for icon-only actions and tooltips for unfamiliar icons.
- Preserve PDS Health tokens. Product-local aliases must map back to `--pds-*`
  variables.
- Keep raw color literals in `tokens.dtcg.json` only. Generated token outputs
  and component CSS must not become competing authoring sources; component CSS
  uses named `--pds-*` tokens so color, contrast, and theme decisions remain
  governed in one place.

## Release Gates

A component family is enterprise-ready only when it has:

- token-backed styling for light and dark themes;
- keyboard and focus behavior documented;
- accessibility smoke coverage;
- typecheck/build coverage;
- product-neutral catalog coverage;
- agent-readable catalog manifest coverage;
- source export drift coverage with matching props types and retained
  `component_api_inventory`;
- agent decision recipe coverage with retained `agent_decision_guide`;
- family maturity and evidence ledger coverage;
- at least one representative frontend consumer when behavior needs an
  integration proof; and
- at least one generated product scaffold check consuming it when the component
  is part of the starter flow.

Before a component source change is handed off, run:

```bash
scripts/check-pds-components.mjs --json
```

The command writes `target/appfw/pds-component-check.json` and checks required
exports, source export drift, matching props types, token-backed styling,
baseline accessibility hooks, package metadata, consumer wiring evidence, agent
decision recipes, the product-neutral PDS Component Catalog, and
product-neutral source. It also fails if framework-owned component CSS contains
raw color literals outside the token source of truth.

For browser-backed catalog evidence, run:

```bash
scripts/check-pds-catalog-evidence.mjs --json
```

The command writes `target/appfw/pds-catalog-evidence.json` and screenshots
under `target/appfw/pds-catalog-evidence/screenshots/`. It builds the
interactive catalog, runs Playwright and axe in desktop/mobile light/dark
scenarios, checks component coverage, chart/process examples, overflow,
escaped popovers, duplicate IDs, and unnamed interactive controls.

For live component review, run the interactive catalog from
`appfw_ui/pds_health/catalog-app`:

```bash
npm run dev
```
