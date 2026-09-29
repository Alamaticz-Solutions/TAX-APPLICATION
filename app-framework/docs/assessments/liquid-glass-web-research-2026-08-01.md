# Liquid Glass on the Web — Verified Research and Applied Decisions (2026-08-01)

Status: research assessment feeding the apple-like grammar modernization
prototype (see `appfw_ui/pds_health/CHANGELOG.md` Unreleased). Three parallel
research streams: craft-technique extraction, reference verification, and
primary-source sweep. This document records what was verified, what was
refuted, and exactly which numbers the PDS tokens adopted.

## 1. Verified primary sources

- **Apple HIG — Materials** (content extracted via the JSON data endpoint
  `developer.apple.com/tutorials/data/design/human-interface-guidelines/materials.json`;
  the HTML page is JS-rendered and a dedicated `liquid-glass` HIG page does
  not exist). Liquid Glass "forms a distinct functional layer for controls
  and navigation elements … that floats above the content layer"; do **not**
  use it in the content layer. Two variants: **regular** (adaptive, legible,
  for sidebars/popovers/text-bearing chrome) and **clear** (non-adaptive,
  media surfaces only, requires a **35% dark dimming layer** over bright
  content — the only hard number Apple publishes).
- **Apple — Adopting Liquid Glass**
  (`developer.apple.com/documentation/TechnologyOverviews/adopting-liquid-glass`,
  same JSON-endpoint route): glass comes from the system layer, not
  per-component decoration; avoid overuse; test under Reduced
  Transparency/Reduced Motion/Increased Contrast; scroll-edge effects solve
  scrolled legibility; ship light + dark + increased-contrast variants of
  control colors.
- **WWDC25 219 "Meet Liquid Glass"** (via wwdcnotes.com): lensing not
  frosting; gel-like interaction flexibility; size-dependent "thickness"
  (larger surfaces = deeper shadow, more lensing); never stack glass on
  glass; don't mix variants; tint only for primary actions.
- **WWDC25 356 "Get to know the new design system"**: concentricity —
  nested radius = parent radius − padding; fixed/capsule/concentric shape
  types.
- **Microsoft Fluent Acrylic**
  (learn.microsoft.com/windows/apps/design/style/acrylic) — enterprise prior
  art. Recipe: background → gaussian blur → exclusion blend → tint → noise.
  Governance: transient surfaces only; opaque for structural panes; never
  acrylic-on-acrylic; no accent-colored text on acrylic; solid-color
  fallbacks for transparency-off/battery-saver/low-end/high-contrast.
- **NN/g, "Liquid Glass Is Cracked" (Budiu, 2025-10-10)** + Apple's retreat
  timeline (iOS 26 beta 3 re-frosting 2025-07-07; iOS 26.1 Clear/Tinted
  toggle 2025-11-03): encode conservative defaults; spectacle is opt-in.
- **Canonical web technique**: kube.io "Liquid Glass in the Browser"
  (physics-derived SVG displacement maps; Chromium-only as
  `backdrop-filter: url(#…)`; maps costly to rebuild — precompute; animate
  only `feDisplacementMap scale`). Atlas Pup Labs "Liquid Glass, but in
  CSS" (dual inset-shadow Fresnel rim). Josh Comeau backdrop-filter guide
  (`-webkit-` prefix still required for Safari; `@supports` opaque fallback;
  200%-height + mask trick to fix blur sampling at fixed-header edges).

### Supplementary teardown (reviewed 2026-08-01)

- **superdesign.dev "Apple Design System" (2026-06-14)** — vendor teardown
  citing HIG/WWDC25; no glass numbers. Two usable deltas: (1) the Apple
  type ramp (11/12/13/15/16/17/20/22/28/34pt) as the skeleton for the
  missing PDS display-size ramp (the gap that forced the strategy app's
  hand-rolled `clamp()` hero type); (2) its "one family carries all
  hierarchy" rule conflicts with the W0 Poppins+Inter two-tier decision —
  the W0 decision stands (brand-mandated, provenance recorded); PDS
  apple-like is Apple's craft grammar wearing PDS brand, not a clone.

## 2. Reference verification (user-supplied list)

| Claim | Verdict |
| --- | --- |
| "LobeHub Liquid Glass Design Tokens" | **Misattributed.** LobeHub published nothing; the real artifact is a third-party skill by `ifiokjr` (repo since renamed/deleted; tokens recovered from git history at `ifiokjr/monopi@2ac7bb58b5`). Values are sound and were cross-checked below. Its `prefers-contrast`/`prefers-reduced-motion` override pattern is the genuinely reusable part. |
| "GlassyUI Component Library" | Real repo (`Jaishree2310/GlassyUI-Components`) but a Hacktoberfest/GSSoC showcase: 115 stars vs 216 forks, 264 open issues, **no license**. Not usable. |
| "Liquid Glass Studio" | Real (`iyinchao/liquid-glass-studio`, 549★, MIT, active): WebGL2/WebGPU parameter studio with Fresnel/dispersion/metaballs. A reference for shader technique, not an installable library. |
| mpify.com / creative-tim.com articles | Fetched; trend framing only, no implementation content. Only supportable rule: glass needs background variation to read. |
| Strongest real implementations | `rdev/liquid-glass-react` (5.8k★, dormant), `AndrewPrifer/liquid-dom` (2.4k★, WebGPU, Chrome-only), `shuding/liquid-glass` (1.1k★, frozen). No official glass tokens exist from Geist, shadcn, or Radix. |

## 3. Cross-source consensus numbers → PDS tokens

| Consensus finding | PDS token decision |
| --- | --- |
| Blur sweet spot 8–24px (24+ = fog); saturate 150–200% | `--pds-backdrop-glass: blur(16px) saturate(1.8)` |
| Text-bearing glass needs fill alpha ~0.65–0.82 light / 0.6–0.75 dark (low-alpha showpiece glass cannot carry text) | `--pds-color-surface-glass: light-dark(rgba(255,255,255,0.72), rgba(21,27,35,0.68))` |
| Specular rim: bright top inset + faint bottom counter-light (Fresnel impression) | `--pds-shadow-specular: inset 0 1px 1px <highlight>, inset 0 -1px 1px <highlight-soft>`; `--pds-color-border-highlight-soft: light-dark(white 0.35, white 0.04)` |
| Opaque fallback ~0.95+ when backdrop-filter unavailable or transparency reduced | `--pds-color-surface-glass-opaque: light-dark(white 0.97, rgba(21,27,35,0.97))` + `@supports not`, `prefers-reduced-transparency`, `prefers-contrast: more` blocks on glass consumers |
| Spring-feel micro-interaction easing (ease-out-quint family), ~150/300ms | `--pds-motion-easing-spring: cubic-bezier(0.22, 1, 0.36, 1)`; material-like maps it to the M3 standard curve |
| Concentric nesting (radius = parent − padding) | segmented thumb radius `calc(var(--pds-radius-control) - 3px)` |

Guardrails encoded from Apple/Fluent/NN/g: glass at the chrome altitude only
(sticky header/dock; never work surfaces), one glass plane per stacking
region (no glass-on-glass), regular-variant behavior only (clear variant not
adopted — enterprise text density rules it out), reduced-motion strips
lift/press/spring transitions, and the glass surface never carries contrast
alone (opaque fallbacks above).

## 3a. Direction refinement (PO, 2026-08-01): flat-first

Wayne's direction after reviewing the prototype: apple-like should track
Apple's *flat, clean* application style — glass only where Apple uses it
(toolbars/chrome), freshness from type, spacing, and clean tinted themes,
not from effects. Applied: accent-glow rings and gradient hover borders
removed from commodity chrome (buttons, menu triggers/items, menu panels,
toasts, data-grid toolbar, select picker) in favor of flat accent-soft
fills, thin accent borders, and soft elevation shadows. Glow/gradient
energy now appears **only** on the intelligence altitude (assistant
messages, streaming cursor, freshness/memory marks) per ADR 0014's
violet-AI reservation and the energy-budget laws. "Clean fresh themes"
route: the W1 Leonardo-style tenant brand-ramp (tinted neutrals + accent
over flat surfaces), not additional grammars.

## 4. Deferred (documented, not implemented)

- **SVG displacement refraction** (kube.io recipe): Chromium-only as
  backdrop-filter; requires precomputed maps per shape/size. Candidate W5
  signature-moment enhancement behind `@supports`, never a dependency.
- **Fluent-style noise texture** on glass (needs an asset pipeline).
- **Scroll-edge effect** (content-aware header contrast on scroll) and the
  Comeau 200%-height blur-sampling fix for the narrative header.
- **Tinted-style user toggle** (iOS 26.1 precedent) as an appearance
  preference alongside reduced-transparency.
- Fluent's exclusion-blend luminosity layer (adopt only if dark-mode glass
  legibility evidence demands it).

## 5. Refuted / do-not-cite

- "LobeHub shipped liquid-glass tokens" — fabricated attribution.
- Apple publishes numeric blur/alpha values — false; all numbers above are
  community reverse-engineering, cross-checked across ≥3 sources.
- Any claim sourced only from mpify.com/creative-tim glassmorphism posts.
