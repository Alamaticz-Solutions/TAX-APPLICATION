# Design System Benchmarks: Material 3/MUI, Fluent 2, Geist, Spectrum (July 2026)

> **Status: curated competitive research — not a ratified design contract.**
> Method: four parallel research agents (one per system), primary-source
> fetches where sites permitted, search-relayed citations elsewhere; each
> agent flagged unverified items inline. Complements
> [modern-ux-styles-research-2026-07-15.md](modern-ux-styles-research-2026-07-15.md)
> (adversarially verified landscape) and feeds
> [PDS Experience Dimension And Catalog Plan](../specs/pds-experience-dimensions-and-catalog-plan.md).
> Provenance caution: spec sites for Material (m3.material.io) and Spectrum
> (spectrum.adobe.com) are JS-rendered and agent-opaque — itself a finding.

## 1. Google Material 3 + MUI

**Philosophy/evolution.** M3 Expressive (announced May 2025; Android 16 QPR1
Sept 2025) adds physics-based motion, 35 shape profiles, emphasized type, and
bolder color on top of Material You. Google markets "46 studies, 18,000+
participants." Caution: Google's "spotted up to 4× faster" marketing figure
was **refuted 0-3** in our July 15 verification — cite CHI 2026's 33% faster
fixation / 20% faster completion instead, and note Google's own boundary:
expressiveness that breaks known patterns *hurts* usability.

**Token architecture.** Three tiers (reference → system/semantic → component).
Color: 5 key colors → 13-tone palettes in HCT space → ~19 named roles; dynamic
color from wallpaper on Android. Type: 15 baseline + 15 emphasized styles
under Expressive. Shape: 5-step scale (4–24dp) plus decorative shapes. Open
`material-color-utilities` libraries exist, but there is **no first-party
W3C-DTCG token export** and the spec site is unreadable to simple agents.

**Motion.** Expressive replaces duration+easing with **spring tokens**:
spatial vs. effects × fast/default/slow, two schemes — Expressive (visible
overshoot, recommended consumer default) and Standard (calm, "utilitarian").
Effects springs never overshoot. Exact stiffness/damping tables unverified
(JS-rendered pages).

**Adaptive layout.** Window size classes: Compact <600dp, Medium 600–839,
Expanded 840–1199, Large 1200–1599, XL 1600+; navigation morphs bottom bar →
rail → drawer. **Canonical layouts**: list-detail, feed, supporting pane.

**Density regression.** M2's density scale (−4px steps) and full data-table
spec were deprecated **without M3 replacements**; M3 density guidance is
principles-only. Third parties sell kits to fill the table gap.

**MUI reality check.** MUI still implements Material 2; M3 adoption is
formally on hold, and Google's own material-web has been in maintenance mode
since June 2024 — **no first-party path to M3/Expressive exists on the web**.
MUI v9 (April 2026): Base UI (headless, v1.0 Dec 2025) is the strategic bet;
Pigment CSS, Joy UI, Toolpad all paused. MUI X Data Grid v9 ships an **AI
assistant that converts questions into inspectable grid API calls** — the one
shipped AI pattern worth copying. Licensing moved to per-app; priority
support Enterprise-only.

**Exploitable gaps.** Web is orphaned for Expressive; enterprise density
vacuum; no DTCG/agent-readable spec; expressive-with-restraint middle ground
unclaimed; no published AI experience doctrine; stewardship credibility
(abandoned web implementations, paused MUI projects).

## 2. Microsoft Fluent 2

**Philosophy/evolution.** Fluent 2 (2023) is a systematization release:
cohesive color, tokens, standardized corners, accessibility notation. The
2025–26 story is AI: M365 icon refresh (Oct 2025) with "fluid forms, vibrant
gradients"; Copilot redesign (May 2026) as "task-aware workspace"; Microsoft
frames Copilot's backbone as an "AI-forward" extension of Fluent 2.

**Tokens.** Two layers — global + alias (~160+ alias tokens), composite
tokens for shadow/typography; themes as swappable CSS variables
(`FluentProvider`). Custom theming injects a **16-stop BrandVariants ramp**;
**neutrals cannot be customized** (open issue #30459). Corners 0/2/4/6/8px.
Type: Segoe UI web ramp Caption 10px → Display 68px; Semibold-not-Bold.
Elevation: blur-indexed shadow ramp (2/4/8/16/28/64), key+ambient composition,
opacity ~doubles in dark mode, luminosity formula on brand surfaces, Windows
substitutes strokes.

**Motion.** Principles say "inertia, gravity, velocity"; shipped tokens are
50/100/150/200ms+ durations and nine bezier curves, WAAPI-based, **no spring
system**. Docs publish no numeric values (they live in code); a token-drift
bug shows loose doc/code coupling.

**Density/data.** Office-bred chrome (command bars, trees, forms) is strong,
but the **DataGrid is a documented weak point**: no built-in virtualization,
whole-grid re-renders slow at 50–250 rows, auto-fit vs. resize either/or;
guidance leans anti-density ("too much dense information can be
disorienting").

**AI design guidance (deepest moat).** Three-altitude model — **Immersive /
Assistive / Embedded**; HAX Toolkit (human-AI guidelines, pattern library);
visible latency states; fallibility notices; AI-content marking; Copilot
gradient identity propagated through icons — but everyday components stay
grey; **luminescence is reserved to the Copilot brand layer**.

**Exploitable gaps.** Density/data-grade tables; motion ambition gap
(rhetoric vs. 200ms fades); luminescence-as-Copilot-only; neutral-locked
theming; migration fatigue/fragmentation; AI signaled by bolt-on identity
rather than native component states.

## 3. Vercel Geist

**Philosophy.** "Minimal and high-contrast: plenty of whitespace, restrained
color… color signals state or hierarchy rather than decoration"; "use motion
only when it clarifies a change." Swiss-inspired; typography-as-brand.

**Agent-readability (standout).** Vercel publishes the design system as
machine-consumable spec files — **`design.md` / `design.dark.md`** — plus
registry+MCP direction in the shadcn ecosystem. Directly validates the PDS
agent-readable thesis; still alpha-grade.

**Type.** Geist Sans/Mono under **SIL OFL 1.1** (npm/Google Fonts); Mono
mandated for code, metrics, IDs, timestamps. Free distribution as brand
colonization — and why "made with Geist" is now a commodity look.

**Color.** Ten scales, 10 steps each, where **the step encodes intent**:
100–300 backgrounds (default/hover/active), 400–600 borders, 700–800 solid
fills, 900–1000 text/icons. Two page backgrounds only. P3/OKLCH variants
shipped. Identical token names re-resolved across light/dark.

**Materials.** Two families only — **surface** (base/small/medium/large,
radii 6–12px) and **floating** (tooltip/menu/modal/fullscreen, 6–16px,
escalating lift). Border-first depth; 1px borders for static structure,
shadows reserved for overlays. **No glass, glow, grain, or gradients in the
product system** (those live on marketing surfaces).

**Craft bar is editorial as much as visual.** Per-component copy rules
(Verb+Noun CTAs, "Get Started" banned), ⌘K command menu behavior (query
preserved on back-navigation, backspace pops page stack, commands "act, not
browse"), four-variant empty-state taxonomy, precise focus recipe
(`0 0 0 2px #fff, 0 0 0 4px #006bff` at `:focus-visible`), motion
150/200/300ms with one signature ease, reduced-motion honored.

**Exploitable gaps.** ~42 mostly-simple components: no data grid, no dense
forms/wizards, no bulk-edit patterns; monochrome austerity reads cold outside
developer tools; the aesthetic is commoditized — matching its restraint while
owning a signature move beats imitating it.

## 4. Adobe Spectrum / Spectrum 2

**Philosophy/evolution.** Principles: Rational, Human, Focused (+
Collaborative in S2). Spectrum 2 (announced Dec 2023) is rounder/bolder
("functional and joyful"); React Spectrum S2 v1.0 shipped **Dec 2025**.
Cautionary tale: Photoshop 2026's Spectrum-based UI is drawing public craft
backlash (unfocused dialogs, platform-convention erasure) — **system
consistency without per-surface QA destroys perceived craft**.

**Tokens/design data.** Three tiers; t-shirt sizing implemented by remapping
custom properties (retrofitting sizes into token names caused breaking
changes — avoid). **Platform scale: desktop and mobile scales at ~1:1.25** —
the canonical precedent that channel is a *scale*, not a theme. The tokens
repo became **spectrum-design-data**: tokens + JSON schemas for component
APIs + component anatomy + an **MCP server** — Adobe is shipping
machine-readable design data for agents.

**Color.** **Leonardo** (`@adobe/leonardo-contrast-colors`, open source)
generates palettes from target contrast ratios — **contrast is engineered
into generation, not audited afterward**; higher token number always means
higher contrast.

**React Aria (the crown jewel).** Headless three-layer stack (Stately/Aria/
Spectrum); strictest WAI-ARIA implementation; 50+ components; 30+ locales,
13 calendar systems, 5 numbering systems, RTL; **fully keyboard- and
screen-reader-accessible drag and drop**; virtualized collections; ~260K
weekly downloads; the base under Untitled UI React, HeroUI, and others.

**AI.** S2 Buttons ship **`genai` and `premium` gradient variants**; C2PA
Content Credentials auto-applied to Firefly output; AI-friendly docs + MCP —
but **no published AI UX doctrine** (nothing like Carbon for AI).

**Motion.** Purposeful/seamless doctrine, three custom easing curves; S2's
"joy" is chromatic/geometric, **not kinetic**.

**Theming is officially unsupported** — maintainers redirect customizers to
React Aria; users report being unable to change a border radius.

**Exploitable gaps.** Branded expressiveness (they refuse theming); kinetic
energy; AI pattern doctrine; density modes; craft-at-the-seams execution
standards; and their own architecture invites competitors to build on React
Aria and out-style Adobe above it.

## 5. Cross-system synthesis: where the lead is available

Convergent practices (adopt as table stakes): three-tier tokens; alias
semantics; light/dark as token re-resolution; 4px grid; restrained corner
ramps; reduced-motion parity; WCAG-driven focus.

Open territory none of the four occupy — the PDS lead:

1. **Expressive motion for web enterprise.** Google validated it (CHI 2026),
   then stranded the web; Fluent's rhetoric outruns its 200ms fades; Spectrum
   is static; Geist abstains by principle. Restrained spring/choreography on
   dense workflows is uncontested.
2. **Data-grade density with craft.** M3 deleted its density/table specs;
   Fluent's grid struggles at 250 rows; Geist has no grid; Spectrum has no
   density modes. Virtualized-by-default tables + persisted density
   preference + metric typography is the visible enterprise win.
3. **Agent-readable design system with governance.** Geist's design.md and
   Adobe's design-data/MCP are early and partial; neither carries governance
   metadata. PDS shipping tokens + component API/anatomy schemas + MCP +
   evidence gates leapfrogs both — and is already the PDS thesis.
4. **Published AI experience doctrine.** Fluent has altitudes + HAX; Adobe
   has gradient variants; nobody publishes a full doctrine binding semantic
   AI light, attribution, explainability, revert-to-AI, and latency states
   into component contracts. Carbon's "light as AI" norm + PDS violet is the
   foundation.
5. **Governed dual-grammar theming.** Spectrum refuses theming; Fluent locks
   neutrals; M3 themes only inside Google's aesthetic. PDS's apple-like /
   material-like grammars over one semantic contract, with Leonardo-style
   contrast-generated tenant ramps, is structurally differentiated.
6. **Judicious luminescence on a work canvas.** Fluent confines glow to
   Copilot branding; Geist bans it. Precision Daylight + Chromatic Signal +
   Connected Fabric (within ADR 0014 constraints) delivers energy nobody
   else ships — with restraint rules already ratified.

## 6. Unverified items carried from agents

Exact M3 spring stiffness/damping tables; Fluent "frontier future" article
date; `@vercel/geistcn` npm licensing; Geist variable-font axis ranges;
design.dark.md contents; Spectrum duration/bezier values; Spectrum WCAG
2.1/2.2 version-numbered commitment; the 2026 "Gemini Intelligence" layer
(single-source). Do not build load-bearing decisions on these without direct
verification.
