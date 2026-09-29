# Nexus Experience-System Readiness

Status: accepted foundation direction; Nexus product activation and production
claims remain separately gated

Owner roles:

- Product Owner: Wayne Kempf
- Design authority: required before signature graduation
- Architecture owner: App Framework architecture lane
- Implementation owner: App Framework frontend lane
- Consumer proof owner: future Nexus product team after explicit activation

## Purpose

Turn the ratified PDS Experience System into the shortest credible path for a
Nexus team. This spec defines what the design system and component library must
provide before Nexus product developers should build My Requests, My Tasks,
status, approvals, notifications, search, and intelligent work experiences.

This is a framework-readiness contract. It does not choose a ServiceNow object,
invent Nexus behavior, connect a live source, or authorize product release.

## Product Foundation Decision

The Nexus UX foundation is not a general component engine and not a themed
vendor kit. It has four deliberate ownership zones:

| Zone | Default owner | PDS responsibility |
| --- | --- | --- |
| Commodity web interaction | React Aria Components behind PDS adapters | Public API, semantics, styling, evidence, upgrade boundary |
| Specialized data/work engines | Selected per proven need behind PDS adapters | License, contract, telemetry, accessibility, fallback, exit plan |
| Native interaction | React Native plus channel-native implementations | Shared semantics and tokens, native navigation, interruption-safe behavior |
| PDS differentiation | PDS Experience System | Work patterns, intelligence, trust, journeys, visual language, outcomes |

Product code imports only `@appfw/pds-health-components` and generated product
contracts. It does not import React Aria, a visual vendor system, or a grid
engine directly. Apple-like and material-like are PDS visual grammars over one
semantic and behavior tree.

## Definition Of Nexus-Ready

The library is Nexus-ready only when a package-only downstream composition can
complete the following fixture-backed journey without framework-source edits:

1. orient to work that needs attention and why;
2. search, filter, sort, and navigate My Requests and My Tasks;
3. open a request/task detail with ownership, status, freshness, timeline,
   evidence, and source links;
4. review an approval or selected governed action with consequence preview;
5. complete, reject, undo, or recover with durable operation feedback;
6. receive and manage an in-app notification with an exact deep link;
7. inspect a structured intelligent result with evidence, permission,
   freshness, attribution, fallback, and recourse; and
8. continue the bounded task on a touch-sized web surface and a native-mobile
   reference without semantic drift.

The journey must prove loading, empty, partial, stale, offline, unauthorized,
forbidden, conflict, timeout, success, and recovery states that apply. A
component count, static screenshot, catalog render, or attractive theme is not
Nexus readiness.

## Required Capability Stack

### P0: Foundation Before Product Screens

- generated canonical design data for color, type, spacing, shape, elevation,
  motion, iconography, data visualization, content, and semantic state;
- apple-like/material-like grammar, system/light/dark mode, pointer/touch
  scale, comfortable/compact data density, reduced motion, forced colors,
  window class, locale/RTL, zoom, and coarse-pointer adaptation;
- self-hosted, subset, provenance-approved type assets with metric-adjusted
  fallbacks, tabular numerals, and native platform mappings;
- 4px spatial rhythm, stable target and control dimensions, content-width and
  floor-plan tracks, safe areas, and no state-induced layout shift;
- semantic motion roles with immediate acknowledgment, interruption,
  reversal, reduced-motion parity, and grammar-authentic expression; and
- React Aria interaction adapters plus cross-cutting accessibility, geometry,
  browser, visual, performance, and consumer evidence.

### P0: Nexus Work Surface

- application shell, global and contextual navigation, responsive rail/drawer/
  bottom navigation, page header, breadcrumbs, tabs, toolbar, and command
  access;
- global search plus scoped dynamic search, query/filter controls, recent and
  suggested results, keyboard access, and deterministic empty/error fallback;
- My Requests/My Tasks queue with list/table adaptation, saved view/filter,
  sort, pagination or virtualization, selection, bulk-action posture, and
  complete data states;
- list-detail/workbench floor plan with exact browser history, deep links,
  focus restoration, unsaved-change handling, and touch adaptation; and
- request/task/approval domain components for status, priority, ownership,
  due/SLA, progress, blocked state, timeline, attachment, comment, evidence,
  audit, source system, and receipt.

### P0: Trust, Action, And Resilience

- fields, textarea, number, choice, combobox, date, time, date-time, upload,
  validation, read-only, sensitive, and asynchronous lookup behavior;
- menu, popover, tooltip, dialog, alert dialog, drawer, bottom sheet, toast,
  banner, disclosure, and focus/dismissal contracts;
- intent preview, confirmation, delegated actor and policy context, named
  consequence, idempotent operation state, action audit, undo/compensation,
  permission denial, reconciliation, and correlation identifiers; and
- notification inbox/center with grouping, deduplication, read state, expiry,
  snooze, preference, policy override, accessibility, and exact resumption.

### P0: Intelligence Beyond Chat

- generated structured view, evidence summary, citation, freshness,
  confidence, source and AI attribution, and deterministic fallback;
- recommendation and next-best-action components that distinguish inform,
  recommend, prepare, and act;
- “why this needs you” explanation, editable/rejectable suggestions,
  permission-aware action preview, progress, cancellation, human handoff,
  correction, and outcome telemetry; and
- a provider-neutral result contract so PDS Health AI, Anthropic, OpenAI, or a
  later provider can supply evidence without becoming the UI or transaction
  authority. Chat remains an invocation and exploration surface.

### P1: Productive Personalization And Deep Work

- role/admin layout templates, allowlisted modules, policy locks, user
  add/remove/reorder/resize/configure, undo/reset, schema migration, responsive
  size classes, accessible order, and cross-device persistence;
- advanced team queues, calendars, trees, grouped work, dense keyboard-first
  administration, export, pinned fields, and large-data performance; and
- web/native push adapters, quiet hours, escalations, journey correlation, and
  administrative delivery/failure views.

### P2: Scale Extensions

- specialized data grid, scheduler, rich editor, diagram, or visualization
  adapters only after the real journey proves the need;
- additional floor plans, module types, external channels, and embedded
  surfaces only after second-use evidence; and
- no generic widget marketplace, page builder, workflow engine, or arbitrary
  agent-generated runtime UI.

## Existing Library Disposition

| Disposition | Representative current surfaces | Action |
| --- | --- | --- |
| Preserve PDS semantics | `IntentPreview`, `ActionAudit`, `OperationState`, `GeneratedViewShell`, `RecommendationCard`, evidence/freshness/attribution, `ConnectedFabric` | Certify through Nexus-shaped compositions; do not replace with vendor semantics |
| Migrate commodity mechanics | Buttons/toggles, fields, selection, date/time, menus, popovers, dialogs, tabs, search, collections | Preserve or deliberately version PDS APIs; replace local mechanics with React Aria by bounded deliverable |
| Adapt a specialized engine | Data grid/virtualization, charts, flow graph, future scheduler/editor | Select only from measured product need and hide behind PDS contracts |
| Build missing PDS product patterns | My Work queue/detail, task/approval, notification center, work receipt, responsive navigation, personal canvas | Build from the layered system and prove in the Nexus reference composition |
| Keep channel-native | React Native navigation, secure storage, push, gestures, haptics, offline continuation | Share semantic design data and product contracts, not web DOM or CSS |

## Delivery Deliverables

Each deliverable includes foundations, mechanics, both grammars, light/dark,
pointer/touch, complete states, evidence, and a production-shaped composition.
F0/F1/N0–N4 are retained historical identifiers mapped to deliverables; they
are not standing queues or a separate level in the work hierarchy.

1. **F0 Foundation lock:** canonical design data, font payload, spacing/layout,
   motion, icon, adaptation, and catalog hierarchy become executable.
2. **F1 Interaction proof:** button/toggle, field/validation, select/combobox,
   menu/popover, dialog, and date/time migrate behind PDS APIs.
3. **N0 My Work skeleton:** shell, global search, queue/detail floor plan,
   responsive navigation, fixture data, and operational states.
4. **N1 Trusted task:** status/timeline/evidence, notification deep link,
   preview-only approval, operation feedback, audit, undo, and receipt.
5. **N2 Structured intelligence:** “why this needs you,” evidence brief,
   recommendation, deterministic fallback, provider-neutral trace, and no-chat
   primary workflow.
6. **N3 Native continuation:** task list/detail/approval reference, secure
   continuation, interruption, push/deep link, freshness, and offline posture.
7. **N4 Consumer proof:** disposable package-only Nexus-shaped app, genuine
   framework upgrade, task evidence, signature comparison, and support handoff.

Do not migrate all current controls before N0. Migrate only the substrate and
components needed by the next complete deliverable, then widen from retained proof.

## Quality And Exit Gates

- identical PDS API, DOM meaning, content origin, and state semantics across
  visual grammar and mode;
- no measurable outer/control-content movement on focus, selection,
  validation, loading, open, or close;
- keyboard, focus order/return, screen reader, touch, 200% zoom, RTL,
  forced-colors, reduced-motion, and coarse-pointer evidence;
- Core Web Vitals and bundle budgets, including INP at or below 200ms p75 for
  the supported journey and no avoidable layout shift;
- exact operation, request, correlation, source, policy, evidence, freshness,
  and intelligence trace hooks into PDS Observability;
- light/dark plus apple-like/material-like visual review, five- and 30-second
  first impression, task success/time, recovery, and signature-quality proof;
- one package-only consumer and one genuine upgrade before Nexus-ready status;
  and
- no direct product imports from the interaction substrate or specialized
  engine.

## Stop Rules

Stop and correct the foundation when a deliverable requires separate behavior trees
by theme, leaks vendor types into product code, creates state-induced layout
shift, cannot meet accessibility or reduced-motion parity, or needs product-
local copies of shared controls. Narrow or replace the substrate if the
representative proof cannot satisfy PDS semantics without persistent forks.
