# ADR 0016: PDS Web Interaction Substrate

## Status

Accepted on 2026-07-17 for bounded migration and Nexus foundation proof.
Production use remains component- and journey-evidence gated.

## Context

The PDS component package exposes useful enterprise and intelligent-work
semantics, but many commodity controls still implement interaction mechanics
locally. Rebuilding focus management, overlays, collections, selection,
keyboard interaction, date/time behavior, internationalization, touch
semantics, and assistive-technology support consumes effort without
differentiating Nexus.

Adopting a complete visual framework would create a different problem. Material
UI v9 remains a Material Design 2 visual implementation, Material Web is in
maintenance mode, and full Spectrum or Fluent adoption would import another
organization's visual grammar and public API. PDS needs authentic apple-like
and material-like expressions over one PDS semantic contract, not two vendor
component trees.

## Decision

Use **React Aria Components** as the default commodity interaction substrate
for PDS web controls. It is consumed behind `@appfw/pds-health-components`;
product code must not import it directly. PDS retains ownership of:

- public component APIs, domain semantics, content, state vocabulary, and
  compatibility policy;
- canonical tokens, apple-like and material-like visual grammars, density,
  platform scale, and semantic motion;
- Nexus floor plans, work patterns, notifications, governed actions,
  structured intelligence, and signature visual language; and
- browser, accessibility, performance, visual, consumer, and journey evidence.

React Aria owns only the mechanics its primitive supplies. Start with its
component API and drop to lower-level hooks only for a documented PDS contract
that the component API cannot express. Do not import starter-kit styles or
expose React Aria classes, slots, state names, event types, or collection
models as the durable PDS public contract.

Use Base UI only as the named fallback if a representative proof demonstrates
a blocking React Aria limitation. Do not run two general interaction
substrates by preference or visual grammar. Use a specialized engine such as
MUI X or AG Grid only behind a PDS adapter when a real Nexus data-work slice
requires capabilities that accessible collections do not provide and after
license, bundle, accessibility, observability, and exit review.

Native iOS and Android continue to use React Native and channel-native
interaction implementations over shared PDS semantic design data. Web DOM is
not mechanically ported to native.

## First Proof

The bounded proof must cover representative mechanics rather than a cosmetic
button demo:

1. button and toggle state;
2. text field and validation;
3. select or combobox collection behavior;
4. menu plus popover placement and focus return;
5. dialog focus containment and dismissal; and
6. date/time input with locale, keyboard, touch, stable geometry, and
   reduced-motion evidence.

At least one proof composition must use the Nexus My Work queue/detail shape.
Both visual grammars and light/dark modes consume the same PDS API and behavior
tree. The proof graduates only when existing consumers compile unchanged or
through a deliberate versioned migration and all retained gates pass.

## Consequences

- App Framework stops building a general-purpose component engine.
- Existing PDS APIs are preserved where sound; internals migrate by risk and
  Nexus workflow value rather than through a component-count rewrite.
- Theme switches cannot select a different runtime component tree.
- Commodity primitives are not copied into product repositories.
- A library upgrade is accepted only with contract, accessibility, browser,
  bundle, visual, and consumer evidence.
- A failed proof triggers an explicit ADR amendment or Base UI bakeoff; it
  does not justify ad hoc local mechanics.

## References

- [React Aria getting started](https://react-spectrum.adobe.com/react-aria/getting-started.html)
- [Base UI overview](https://base-ui.com/react/overview/about)
- [Material UI overview](https://mui.com/material-ui/getting-started/)
- [PDS Experience System Architecture And Coverage](../../specs/pds-experience-system-architecture-and-coverage.md)
- [Nexus Experience-System Readiness](../../specs/nexus-experience-system-readiness.md)

