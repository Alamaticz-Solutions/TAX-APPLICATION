# PDS Visual Themes

## Status

Ratified framework theme contract. Product rollout remains evidence-gated by
journey and channel.

## Two Independent Axes

PDS appearance has two independent controls:

- `data-visual-theme="apple-like"` or `data-visual-theme="material-like"`
  selects the surface, shape, elevation, and motion grammar.
- `data-theme="light"` or `data-theme="dark"` selects the color mode. Omitting
  `data-theme` follows the operating-system preference.

Every visual theme must support every color mode. A product cannot use the
visual-theme selector to fork component behavior, workflow semantics,
accessibility, authorization, or data contracts.

## Apple-Like

Apple-like is the default and the no-attribute compatibility fallback. It is
the approved Precision Daylight treatment: luminous blue-white canvas,
translucent work surfaces, fine borders, restrained glow, calm depth, and
fluid motion. Connected Fabric is most visible in this theme, but remains
decorative and subordinate to work.

This name describes a visual grammar. It does not authorize copying Apple
components, assets, trade dress, platform conventions, or product behavior.

## Material-Like

Material-like is the fixed PDS-seeded Material 3 implementation defined in
`material-3-web-contract.md`. It starts from the same PDS semantic tokens,
component APIs, status vocabulary, and brand signal palette. It changes the
expression to M3 color roles, opaque tonal surfaces, component-specific shape,
state layers, discrete elevation, animated field labels, and emphasized
motion. Gradients and backdrop blur are reduced so hierarchy comes from
surface roles and elevation rather than translucency.

PDS doctrine and semantic tokens remain authoritative. Material 3 is the
component-anatomy and interaction reference inside this grammar. Official
Material Web token and motion definitions supply exact web evidence, and MUI
is used only as the fallback for mature web behavior where M3 prioritizes
mobile or is underspecified. The theme uses no wallpaper color extraction and
adds no Material runtime dependency.

This name describes a visual grammar. It does not authorize copying Google
components, assets, trade dress, or product behavior, and it does not make
Material libraries a framework dependency.

## Invariants

Both themes must preserve:

- identical React component APIs, DOM semantics, keyboard behavior, and
  accessibility names;
- identical business-state meaning and non-color status cues;
- PDS brand ownership of typography, signal colors, and content hierarchy;
- light, dark, reduced-motion, responsive, and high-contrast evidence;
- token-only visual values outside the canonical token source; and
- Connected Fabric's decorative-only, pointer-transparent, and exclusive-edge
  constraints.

All controls also preserve stable geometry: focus, selected, active, invalid,
pressed, loading, and expanded states cannot move the control, its content
origin, or neighboring layout. Field-label motion is internal and must leave
the shell and entered-text baseline unchanged.

Theme-specific product markup and theme-conditioned workflow logic are
anti-patterns. Components consume semantic `--pds-*` tokens; the root theme
attribute changes those tokens.

## Catalog Contract

The interactive catalog exposes a persisted **Style** segmented control for
Apple-like and Material-like, alongside the independent **Mode** control for
System, Light, and Dark. Catalog evidence must switch the control, reload the
page, verify persistence, and prove that the two token signatures are
materially different while the component and accessibility contracts remain
green.
