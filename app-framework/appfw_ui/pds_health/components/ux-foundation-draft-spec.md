# UX Foundation Draft Spec: Typography And Motion

Date: 2026-07-15

Status: historical review-only draft. This is not a ratified design-system
contract, release claim, or instruction to merge. Its formerly open
font-payload question was resolved on 2026-07-22 by the Product Owner's
two-tier brand typography decision (Poppins Bold display roles + Inter UI
body/data + Geist Mono route-scoped) — see the dimension spec's Decision
Provenance and `tokens/fonts/README.md` for the governing record.

## Intent

Test the first two moves from the July 2026 UX research as a bounded diff:

1. make the primary and metadata type voices deterministic, correctly weighted,
   metric-aware, and self-hosted; and
2. replace ad hoc component timing with a restrained motion vocabulary that
   improves feedback and spatial continuity without slowing repeated work.

Inputs:

- `docs/assessments/modern-ux-styles-research-2026-07-15.md`
- `docs/assessments/ux-craft-evidence-base-2026-07-15.md`
- `docs/frontend/pds-health-design-system.md`
- `docs/frontend/agentic-ux.md`

## Proposed Contract

### Typography

| Role | Draft value | Application |
| --- | --- | --- |
| Primary sans | Self-hosted `InterVariable`, normal 100-900 | Existing `--pds-font-family-sans` consumers |
| Metric fallbacks | Segoe UI and Arial aliases generated from Capsize 4.1 metrics | Reduce average-width and vertical-metric shift while Inter loads |
| Metadata mono | Self-hosted `Geist Mono Variable`, normal 100-900 | `--pds-font-family-mono`; command shortcuts in this slice |
| Compatibility aliases | `--pds-font-sans`, `--pds-font-mono` | Existing product and catalog consumers inherit the canonical family tokens |
| Semantic weights | 400 regular, 500 medium, 600 semibold, 700 bold | Replaces semantically incorrect 650/700/750 mapping |
| Sans features | `cv01`, `ss03`, contextual alternates, standard ligatures | Tokenized for product and component use |
| Metric style | 32px, 600, optical sizing, lining and tabular numbers | `--pds-font-*-metric`, `.pds-type-metric`, and KPI values |

The fallback overrides were generated with `@capsizecss/core` 4.1.3 and
`@capsizecss/metrics` 4.1.0. They are evidence-backed starting values, not a
substitute for real 96-DPI Windows visual and layout-shift testing.

### Motion

| Role | Draft value | Use |
| --- | --- | --- |
| Fast | 120ms | Press, hover, tooltip, immediate feedback |
| Standard | 200ms | Popover, dialog, toast entrance |
| Slow | 300ms | Drawer spatial entrance |
| Standard curve | `cubic-bezier(0.2, 0, 0.38, 0.9)` | Reversible state changes |
| Enter curve | `cubic-bezier(0.16, 1, 0.3, 1)` | Elements entering the viewport |
| Exit curve | `cubic-bezier(0.2, 0, 1, 0.9)` | Reserved for lifecycle-aware exits |
| Press scale | 0.97 | Enabled buttons only |
| Enter scale | 0.96 | Dialog, popover, and toast entrance |
| Enter distance | 8px | Drawer and toast spatial orientation |

The draft animates transform and opacity for entrance and press behavior.
Existing color, border, and shadow transitions remain tokenized for immediate
interaction feedback. The command palette remains motion-free.

Reduced-motion behavior removes press and spatial transforms. Overlays and
toasts retain a short linear opacity fade so state changes remain perceivable.

## Deliberate Boundaries

- No true exit animation in this slice. Dialog, drawer, and toast lifecycles
  unmount immediately. Exit choreography requires an explicit presence state,
  dismissal semantics, timer behavior, and tests rather than a CSS illusion.
- No OKLCH migration, color retuning, glass removal, radius change, broad
  component restyle, animation library, spring system, or view transition.
- No generated-app or native-mobile contract change.
- No font preload policy. Consumers should preload only after route-level
  performance evidence identifies a justified critical font.
- No claim that font loading is production-ready until the payload decision and
  browser evidence are accepted.

## Material Challenge To The Research

The official Inter 4.1 variable WOFF2 is 352240 bytes. With Geist Mono, the
combined font payload is 423836 bytes. This materially exceeds the research
estimate and the suggested sub-100 KB budget. The draft keeps the official,
unmodified publisher assets for licensing and provenance clarity, but should
not graduate unchanged without one of these decisions:

1. accept the payload based on measured caching and route performance;
2. adopt a publisher- or counsel-approved Latin subset with explicit provenance;
3. load Geist Mono only on routes that use it; or
4. defer the mono webfont and retain the system mono stack.

## Acceptance Evidence

Before ratification:

- token and component checks pass without raw component values or source forks;
- component catalog is captured in light/dark desktop/mobile modes;
- button, dialog, drawer, popover, tooltip, and toast behavior is reviewed at
  normal and reduced motion;
- font requests resolve from built application output without network CDN use;
- layout shift is measured during cold font loading;
- 96-DPI Windows rendering and fallback swap are visually reviewed;
- 200% zoom, keyboard focus, high contrast, and browser axe evidence remain
  green; and
- product proof demonstrates that the motion improves clarity without slowing
  a repeated work queue.

Required repository checks:

```bash
scripts/appfw validate --json
scripts/appfw generate --check --json
node scripts/check-pds-tokens.mjs --json
node scripts/check-pds-components.mjs --json
scripts/appfw handoff --json
```
