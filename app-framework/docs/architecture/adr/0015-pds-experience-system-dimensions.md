# ADR 0015: PDS Experience System Dimensions And Layers

## Status

Accepted on 2026-07-17 by Product Owner Wayne Kempf.

## Context

PDS products need one leading experience system for Nexus, citizen-developed
applications, modernized legacy applications, and newly engineered products.
The required variations are often described as light/dark, web/mobile, and
apple-like/material-like themes. Treating all of them as peer theme switches
would create duplicated semantics, divergent component behavior, and an
unmanageable test matrix.

The component catalog is useful proof, but a catalog of individually polished
controls is not a design system. Consistent product quality requires an
authoritative decision model, generated foundations, adaptive rules, shared
behavior contracts, composition patterns, and journey evidence before broad
component expansion.

## Decision

Adopt one PDS Experience System with this dependency order:

1. **Doctrine:** experience thesis, six altitudes, ten design laws, content
   rules, accessibility, trust, and intelligence semantics.
2. **Foundations:** canonical design data for color, type, spacing, shape,
   elevation, motion, iconography, data visualization, and semantic state.
3. **Adaptation:** visual grammar, color mode, platform scale, window size,
   density, input modality, reduced motion, and forced-colors behavior.
4. **Primitives and behaviors:** layout, text, focus, state-layer, overlay,
   collection, selection, validation, and action-state contracts.
5. **Controls and components:** reusable accessible implementations over the
   same semantics and APIs in every supported dimension.
6. **Patterns and floor plans:** forms, queues, list-detail, workbench,
   dashboard/feed, conversation, and flow/wizard compositions.
7. **Journeys and evidence:** product-specific Orient-Understand-Act-Resolve
   flows, signature moments, operational proof, and release certification.

Use two selectable appearance axes:

- visual grammar: `apple-like | material-like`; and
- color mode: `system | light | dark`.

Treat channel as adaptive platform scale (`pointer | touch`) plus window-size
class, not as a theme. Treat density (`comfortable | compact`) as a persisted
preference on data-heavy work surfaces only. Native iOS and Android products
consume the same semantic design data through channel-native components and
navigation; they do not mechanically port web CSS.

PDS doctrine and semantic contracts are authoritative. Material 3, MUI,
Fluent 2, Geist, and Spectrum are evidence and implementation references, not
sources of product meaning. Mature headless libraries may provide commodity
interaction mechanics behind PDS-owned wrappers when they satisfy the PDS
behavior contract, accessibility requirements, evidence gates, and exit
boundary. Products consume PDS APIs and semantics, not the headless library
directly. Full visual-system imports do not own PDS tokens, visual grammar,
component meaning, or composition.

Material-like should feel authentically Material in component anatomy, state
layers, interaction motion, and elevation while retaining PDS enterprise
density, signal semantics, governed actions, and intelligence language.
Apple-like follows the same semantics through Precision Daylight, fine
borders, restrained translucency, and fluid response.

The catalog is the executable specification. It must expose foundation
tokens, dimensions, primitives, controls, behavior states, floor plans, and
representative journeys rather than treating component count as maturity.

## Ratified Decisions

1. The precedence-ordered design laws in
   `docs/frontend/ux-design-strategy.md` are ratified, including work velocity
   as Law 1.
2. Role-based shape is ratified: compact work surfaces use the PDS shape ramp;
   full rounding is limited to roles such as buttons, chips, segmented
   controls, and indicators where the selected grammar calls for it.

The webfont payload remains an implementation decision before global
production loading. ADR acceptance does not authorize the current unmodified
Inter and Geist Mono payload, which totals 423836 bytes.

## Consequences

- Foundation and adaptation decisions precede broad control restyling.
- Component APIs, DOM meaning, keyboard behavior, permissions, and business
  state cannot fork by visual grammar or color mode.
- Motion is a first-class semantic foundation. Each grammar may re-express a
  transition, but it must retain the same state meaning, timing budget,
  reduced-motion behavior, and stable geometry.
- Every component must declare supported states, density behavior, platform
  behavior, content rules, accessibility semantics, and evidence maturity.
- Product journeys may compose and extend the system but may not fork tokens
  or shared controls.
- App Framework will not build a general-purpose component engine. It owns the
  PDS experience system, public component contracts, intelligent-work
  capabilities, and journey proof; commodity mechanics should be adopted
  behind that boundary when mature implementations meet the contract.
- Existing catalog theme work is bounded implementation evidence, not
  permission to skip the foundation sequence. W0 remains open only for its
  unresolved implementation decisions and required evidence, including the
  production font payload.
