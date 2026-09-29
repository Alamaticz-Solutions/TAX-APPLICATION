# ADR 0014: PDS Health Signature Visual Language

## Status

Accepted. Product rollout remains evidence-gated by channel and journey.

## Context

The PDS component system had a credible enterprise foundation but its light
theme and data colors read as cautious, muted, and interchangeable with older
SaaS products. A bounded catalog review explored a more luminous light canvas,
clearer chromatic signals, translucent work surfaces, and a restrained animated
mesh that expresses the connected application-fabric strategy.

The direction survived iterative visual review only after decorative gradients,
heavy rings, dense particles, elastic mesh distortion, and prominent hover
shading were removed. The retained result is distinctive without assigning
business meaning to decoration or slowing repeated work.

## Decision

Ratify three related PDS Health visual-language contracts:

1. **Precision Daylight** is the light-theme hierarchy: luminous blue-white
   canvas, high-clarity typography, quiet translucent work surfaces, restrained
   depth, and no gray cast.
2. **Chromatic Signal** separates accessible semantic ink from vivid visual
   signals. Bright blue, teal, green, violet, coral, and amber are reserved for
   data, progress, state marks, active controls, and bounded intelligence cues.
3. **Connected Fabric** is an optional expressive web layer for first-viewport
   and cross-system surfaces. Its hex topology represents the application
   fabric; a source ring, routed signal, destination ring, return acknowledgment,
   resolved hold, and fade represent a completed connection.

The interactive PDS Component Catalog is the canonical visual exemplar for
these contracts. It is not a second token or component source of truth.

The accepted visual language has two selectable, token-backed profiles:

- **Apple-like** is the default Precision Daylight expression: luminous,
  translucent, finely bordered, and depth-rich.
- **Material-like** retains PDS semantics and signal colors while using opaque
  tonal surfaces, compact shape, state layers, and discrete elevation.

These profiles are visual grammars rather than vendor component copies. They
are independent from System, Light, and Dark color modes and cannot change
component APIs or workflow behavior.

Connected Fabric has these non-negotiable constraints:

- it is decorative and `aria-hidden`; it never communicates status, risk,
  permission, completion, or AI provenance by itself;
- it cannot intercept input, obscure content, or enter ordinary panel interiors;
- visible circuits may meet at nodes but may not reuse visible edges;
- pointer-local circuits remain sparse, and autonomous circuits are lower
  opacity and infrequent;
- reduced-motion, coarse-pointer, hidden-document, and constrained-channel
  behavior must remain calm and deterministic;
- route color, canvas color, surface hierarchy, and all CSS styling remain
  token-backed; and
- product adoption requires a signature-experience brief plus visual,
  accessibility, performance, and reduced-motion evidence. Native mobile uses
  an equivalent channel-native expression rather than a web canvas port.

The detailed contracts live in:

- `appfw_ui/pds_health/components/visual-themes.md`;
- `appfw_ui/pds_health/components/precision-daylight-visual-language.md`;
- `appfw_ui/pds_health/components/chromatic-signal-visual-language.md`; and
- `appfw_ui/pds_health/components/connected-fabric-visual-language.md`.

## Consequences

- The reviewed direction is the PDS Health plan of record and is no longer
  labeled an experiment.
- Routine enterprise work remains quieter than expressive intelligence and
  connection moments.
- The catalog implementation is framework-owned and must retain browser-backed
  evidence for theme selection and persistence, distinct token signatures,
  canvas presence, decorative semantics, responsive sizing, reduced motion,
  and component accessibility.
- Product teams may consume the language but may not fork the palette, turn the
  mesh into a status channel, or add uncontrolled particles and gradients.
- Shared product APIs should be extracted only when real product consumers prove
  the composition boundary; catalog proof alone does not establish a universal
  component API.
