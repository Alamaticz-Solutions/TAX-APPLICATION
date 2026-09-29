# PDS Material 3 Web Contract

## Status

Bounded implementation contract for `data-visual-theme="material-like"`
under ADR 0014. Its component coverage is catalog evidence while ADR 0015 and
the experience-system foundation are under review; it does not by itself
certify production readiness.

## Authority Order

Use sources in this order:

1. PDS UX strategy, ADRs, semantic tokens, healthcare accessibility,
   enterprise density, governed action, and intelligence contracts.
2. [Material 3](https://m3.material.io/) for color roles, typography, shape,
   elevation, motion, component anatomy, and interaction intent.
3. [Material Web](https://github.com/material-components/material-web) token
   definitions for exact web values where the M3 site is descriptive. Material
   Web is a source reference, not a package dependency.
4. [MUI Material](https://mui.com/material-ui/) for mature web interaction and
   accessibility contracts where M3 prioritizes mobile or leaves web behavior
   underspecified.

Do not start from MUI defaults and call the result Material 3. MUI is the web
fallback, not the visual source of truth.

## Fixed Color System

The theme uses a fixed PDS-owned scheme generated from the approved PDS blue
seed, then stored as static role tokens. It does not inspect wallpaper, device
settings, uploaded images, or tenant content. The role contract includes
primary, secondary, tertiary, error, surface, container, outline, inverse, and
fixed roles plus governed PDS success, warning, and teal extensions.

Runtime components consume role tokens such as `--pds-m3-color-primary` and
`--pds-m3-color-surface-container-high`; they do not derive palettes or embed
raw colors.

## Component Contract

| Area | Required Material 3 expression |
| --- | --- |
| Cards | Elevated, filled, and outlined variants with medium shape and discrete elevation levels. |
| Buttons | Filled, tonal, outlined, text, elevated, toggle, icon, connected group, FAB, and extended FAB roles. |
| Fields | Filled and outlined variants; labels rest inside empty controls and animate to the filled-field upper position or outlined notch on focus/value. |
| Select | Use native `SelectField` for finite choices and mobile reliability. Use `SearchBar` or a governed autocomplete adapter for searchable, async, grouped, creatable, or large option sets. |
| Date | Docked, modal, and modal-input picker anatomy with 40px day targets and explicit selected/current states. |
| Time | Dial and input picker anatomy with tabular values, 12-hour period control, and modal presentation. |
| Lists | Standard and segmented lists with stable leading, headline, supporting, trailing, selected, and action slots. |
| Menus | Standard and vibrant temporary action surfaces with selected, disabled, destructive, icon, description, and badge states. |
| Search | 56px search bar, combobox semantics, keyboard result navigation, dynamic filtering, clear action, and elevated result surface. |
| Toolbars | Docked, floating, vibrant, horizontal, and vertical variants with roving arrow-key focus. |

MUI's [Select](https://mui.com/material-ui/react-select/) guidance supports a
native select for simple constrained choice. MUI's
[Autocomplete](https://mui.com/material-ui/react-autocomplete/) guidance is the
fallback contract for advanced combobox behavior; value and input text remain
separate controlled states, and grouping, async loading, disabled options,
free-solo entry, and keyboard semantics are explicit.

## Stable Geometry Invariant

No selected, focused, active, pressed, invalid, loading, or expanded state may
change a control's measured box, content origin, grid track, or neighboring
placement. Implement emphasis with preallocated transparent borders, inset
state layers, outline, color, opacity, or elevation.

The only intentional internal motion is semantic content motion, such as a
field label moving from its resting position to the outline notch. The control
shell and entered-text baseline remain stable. Variable labels must not change
button width when state changes; reserve space or retain constant copy.

Browser evidence measures document-relative bounding boxes before and after
focus, selection, and value entry. A screenshot alone is insufficient proof.

## State And Motion

Use Material state-layer opacity: hover `0.08`, focus `0.12`, pressed `0.12`,
and dragged `0.16`. Use the M3 duration ramp and emphasized easing tokens.
Respect `prefers-reduced-motion`; labels may resolve without animation, but
state meaning and geometry must remain identical.

Floating labels follow the Material Web interaction pattern: retain separate
resting and floating label geometry, measure their rendered positions, and
animate the crisp floating glyphs between those positions over the Material
standard curve. A generic CSS transition of `top` and `font-size` is not an
acceptable substitute because it visibly distorts type and feels unlike a
native Material control. Rapid reversal cancels and recalculates the motion;
reduced motion resolves immediately.

## Ownership And Non-Goals

- PDS owns React APIs, accessibility, token mapping, tests, and release
  evidence.
- Products own nouns, routes, option data, search providers, workflow meaning,
  and authorization.
- M3 does not authorize theme-specific workflow markup or a second component
  implementation.
- There is no dynamic wallpaper color extraction.
- There is no required MUI or Material Web runtime dependency.

## Evidence

Run:

```bash
npm --prefix appfw_ui/pds_health/catalog-app run build
node scripts/check-pds-components.mjs --json
node scripts/check-pds-catalog-evidence.mjs --json
```

Catalog evidence must cover Material-like light and dark modes, desktop and
mobile layouts, axe results, unnamed controls, overflow, and stable geometry
for representative controls.
