# PDS Experience Dimensions And Catalog Plan

Status: accepted dimension and catalog direction under ADR 0015; no product
activation implied

Spec depth: full

Owner roles:

- Product Owner: Wayne Kempf
- Design authority: Chief Architect
- Delivery admission: Program Flow Controller
- Implementation owner: the admitted deliverable owner
- Review owner: an independent reviewer for each immutable checkpoint

## Business Value

The App Fabric reveal must demonstrate a visible UX lead over normal modern
enterprise applications across every app-framework product class (Nexus,
onboarded citizen apps, modernized legacy apps, new engineered apps). The
July 2026 benchmark research
([design-system-benchmarks-2026-07-17.md](../assessments/design-system-benchmarks-2026-07-17.md))
shows the lead is genuinely available: none of Material 3/MUI, Fluent 2,
Geist, or Spectrum ships expressive-yet-restrained motion for web enterprise,
data-grade density with craft, governed multi-grammar theming, a published AI
experience doctrine, or governance-carrying agent-readable design data. This
spec fixes the dimension model for PDS themes and lays out the build plan
that converts the research into catalog-proven themes, floor plans,
components, controls, and behaviors.

## Problem

Three dimension families are conflated in current discussion as "themes we
need": light|dark, web|mobile, apple-like|material-like. Treating all three
as selectable themes would create an 8-signature token matrix, invite
grammar drift, and contradict ADR 0014's invariant that visual themes cannot
fork component APIs or behavior. Meanwhile the ratified pieces (two grammars,
two color modes, typography/motion draft) are not yet arranged into a
sequenced plan with explicit adoptions from the benchmarked systems.

## Goals

- Implement the ratified dimension model that keeps selectable themes to two axes and
  handles channel and density as adaptive/preference dimensions.
- Encode the wisely-selected choices from Material 3, Fluent 2, Geist, and
  Spectrum as explicit adopt/adapt/avoid decisions.
- Define capability dependencies agents can execute against the existing PDS
  token, component, and catalog surfaces inside an admitted product deliverable.
- Preserve ADR 0014 invariants throughout (token-only differences, identical
  APIs/semantics, Connected Fabric constraints, evidence-gated rollout).

## Non-Goals

- Native iOS/Android implementation remains a separate delivery surface; this
  plan governs the shared semantics, design data, and parity evidence it consumes.
- No product activation, no Nexus feature work, no new component APIs beyond
  the named patterns.
- No vendor component copying: apple-like and material-like remain visual
  grammars per ADR 0014, not Apple/Google component clones.
- No re-litigation of the settled visual language (ADR 0014) or the July 15
  research direction — this plan builds on both.

## Ratified Dimension Model

**Two selectable theme axes (unchanged from ADR 0014):**

| Axis | Values | Selector | Selection semantics |
| --- | --- | --- | --- |
| Visual grammar | `apple-like` (default) \| `material-like` | `data-visual-theme` | Product/tenant expression choice, persisted |
| Color mode | `light` \| `dark` \| absent = system | `data-theme` | User/system preference, `light-dark()` resolution |

**One adaptive dimension (not a theme): platform scale.**

`web | mobile` must not become a third theme axis. Channel differences are
ergonomic (hit targets, control heights, type step, spacing, navigation
floor plan), not aesthetic. Precedents: Spectrum ships desktop/mobile as
**platform scales at ~1:1.25**, Material ships **window size classes**
(compact <600dp / medium 600–839 / expanded 840–1199 / large 1200–1599 /
extra-large 1600+); neither treats mobile as a theme. PDS adopts:

- `pointer` scale (desktop web) and `touch` scale (mobile web, embedded
  webviews) at a tokenized ~1:1.25 ratio, auto-detected via pointer/viewport
  media queries, overridable with `data-scale` for catalog evidence and
  product pinning;
- window-size-class tokens driving floor-plan adaptation (below);
- WCAG 2.2 target-size floors enforced at the touch scale (visible glyphs may
  stay small; hit areas extend).

**One user-preference dimension: density.** `comfortable | compact`
(`data-density`, persisted) on data surfaces only — tables, lists, dense
forms. Overlays never densify. This resurrects, as a PDS strength, the
density system Material deleted (M2 → M3 regression) and Fluent's grid
cannot serve.

**Native mobile (future, gated):** when activated, grammar defaults follow
platform convention — iOS defaults apple-like, Android defaults
material-like — with channel-native expression rather than web ports
(consistent with ADR 0014's Connected Fabric rule). The Photoshop 2026
backlash documented in the benchmark is the cautionary tale for erasing
platform conventions.

**High contrast** is a forced-colors compatibility posture verified in
evidence, not a fifth theme.

Net: **4 selectable theme signatures** (2 grammars × 2 resolved modes),
rendered across 2 scales × 2 densities = **16 evidence states**, all
token-only, all API-identical.

## Scope

`appfw_ui/pds_health/tokens/` (canonical token source, new DTCG JSON),
`appfw_ui/pds_health/components/` (component/control uplift, behavior
contracts, visual-language docs), `appfw_ui/pds_health/catalog-app/`
(catalog exemplar, evidence automation), `docs/architecture/adr/` (new ADR),
`docs/frontend/pds-health-design-system.md` (operating contract updates),
`scripts/check-pds-*.mjs` (gates), agent-readable design-data outputs.

## Repository Context

- [UX Design Strategy](../frontend/ux-design-strategy.md): the decision
  framework (altitudes, laws, decision procedure, evidence rules) governing
  every design choice made while executing this plan.
- [PDS Experience System Architecture And Coverage](pds-experience-system-architecture-and-coverage.md):
  the design-system-first dependency stack and full control, component,
  behavior, floor-plan, and journey coverage model beneath this plan.
- [ADR 0015](../architecture/adr/0015-pds-experience-system-dimensions.md):
  ratified dimension model and layered experience-system architecture.
- [ADR 0014](../architecture/adr/0014-pds-signature-visual-language.md):
  Precision Daylight, Chromatic Signal, Connected Fabric; apple-like /
  material-like profiles; invariants and rollout gates. This plan extends,
  never overrides, ADR 0014.
- [visual-themes.md](../../appfw_ui/pds_health/components/visual-themes.md):
  the two-axis contract this spec builds on.
- [ux-foundation-draft-spec.md](../../appfw_ui/pds_health/components/ux-foundation-draft-spec.md):
  historical typography/motion draft. The current implementation self-hosts
  approved Inter Variable and Geist Mono assets; current token and browser
  evidence, not the draft's former open decision, governs further changes.
- [modern-ux-styles-research-2026-07-15.md](../assessments/modern-ux-styles-research-2026-07-15.md):
  verified restraint rules (glass = chrome only; AI light = meaning only;
  expressiveness never breaks known patterns) and craft numbers.
- [design-system-benchmarks-2026-07-17.md](../assessments/design-system-benchmarks-2026-07-17.md):
  the four-system research this plan selects from.
- PDS inventory: 105 token-disciplined component exports, light/dark via
  `light-dark()`, browser-backed axe evidence, existing density modes to
  formalize.

## Contracts Touched

| Contract Surface | Expected Change | Counterpart Surfaces That Must Stay Aligned |
| --- | --- | --- |
| Token source (`pdsTokens.css`/`.ts`) | New DTCG JSON canonical source generating both; OKLCH values; scale/density/window-class/motion-scheme tokens | `scripts/check-pds-tokens.mjs`, catalog, component CSS |
| Theme attributes | Add `data-scale`, `data-density`; keep `data-visual-theme`, `data-theme` semantics | visual-themes.md, catalog Style/Mode controls, persistence evidence |
| Component contracts | Density variants on data components; command menu, empty-state, focus, AI-pattern contracts | `scripts/check-pds-components.mjs`, component README, axe evidence |
| Catalog | Floor-plan demos; scale/density switches; 16-state evidence automation | `scripts/check-pds-catalog-evidence.mjs` |
| Docs | New ADR 0015; operating-contract updates; agent-readable design-data outputs | `scripts/appfw docs-check --json` |

## Options Considered

| Option | Pros | Cons | Decision |
| --- | --- | --- | --- |
| A. Flat theme matrix: light\|dark × web\|mobile × apple-like\|material-like as 3 selectable axes | Matches how the need was phrased; simple mental model | 8 token signatures to keep drift-free; mobile-as-theme forks ergonomics from aesthetics; violates ADR 0014 no-fork invariant; no industry precedent | Rejected |
| B. Two theme axes + platform scale (adaptive) + density (preference) | 4 signatures; matches Spectrum scale + M3 window-class precedent; ADR 0014-compatible; native-mobile seam preserved | Requires explaining that "mobile" is a scale, not a theme | **Recommended** |
| C. Single grammar now, add material-like later | Least work | ADR 0014 already ratified both; reveal loses the governed-theming differentiator no benchmark system offers | Rejected |

## Decision Provenance

| Date | Owner | Decision | Evidence / Rationale | Revisit Trigger |
| --- | --- | --- | --- | --- |
| 2026-07-16 | ADR 0014 | Two grammars × two modes ratified, token-only | ADR 0014 review | Product consumers prove composition boundary |
| 2026-07-17 | This spec (draft) | Channel = platform scale, not theme; density = user preference | Spectrum 1:1.25 scale, M3 window classes, M2 density regression, Carbon density preference; benchmark doc §5 | Native mobile activation; real-user scale telemetry |
| 2026-07-17 | This spec (draft) | Adopt/adapt/avoid selections per system (below) | Benchmark doc §§1–5 | Any refuted upstream claim or license change |
| 2026-07-22 | Wayne Kempf (Product Owner) | Adopt the official PDS Health brand palette as the color foundation: Health Blue `#00A9EB`, Health Gray `#545860`, Clean White `#FFFFFF`, Health Light Gray canonical `#F4F7F8` (the guide's RGB 244/247/248; the guide's own hex line reads `#F3F6F8` — internal inconsistency, canonical follows RGB) | Full 6-page PDS Health brand guide (093025 export), page 4; existing `--pds-color-brand-*` tokens already match | Brand team corrects the guide's hex/RGB inconsistency or revises the palette |
| 2026-07-22 | Wayne Kempf (Product Owner) | Two-tier brand typography: Poppins Bold for display/headline roles (brand web face); Inter for UI body/data (metric/KPI numerals stay Inter for tabular-nums); Geist Mono route-scoped. Payload = publisher-built latin subsets, self-hosted, no CDN; display-role component wiring graduates separately with layout-shift evidence | Brand guide page 5 (Print: Gotham / Web: Poppins); modern-UX research (Inter screen craft); draft-spec payload analysis; licenses verified 2026-07-22 — Poppins, Inter, Geist Mono all SIL OFL 1.1 with no Reserved Font Names | Display-role legibility/layout-shift evidence fails, or the brand team mandates Poppins for all product text |
| 2026-07-22 | W1a slice | `tokens.dtcg.json` is the canonical token source; `pdsTokens.css`/`.ts` are generated by `scripts/generate-pds-tokens.mjs` and gated byte-identical by `scripts/check-pds-tokens.mjs` (`generated_source_sync`). Migration is behavior-preserving: two pre-existing CSS/TS value divergences were captured verbatim rather than silently changed (`color.state.gold` TS literal `#ffb020` vs CSS `var(--pds-color-signal-amber)`; `motion.fluid` TS literal easing vs CSS alias) | W1a round-trip proof: pristine bootstrap → regenerate → byte-identical; drift gate negative-tested | Divergences reconciled when W1 token remodeling (OKLCH/reference tier) lands; generator replaced if DTCG group output supersedes render-metadata approach |

## Architecture And Implementation Notes

### Selection map across the benchmarked systems

**From Material 3 — adopt:** window size classes and canonical-layout
thinking (as PDS floor plans); the two-scheme motion concept (Standard
default for work, Expressive reserved for signature moments — spatial vs.
effects split, effects never overshoot); three-tier token architecture
(reference → semantic → component); dual type scale idea as a small
emphasized-display set. **Adapt:** HCT tonal-palette insight implemented as
OKLCH ramps (per July 15 direction), not HCT libraries. **Avoid:** pill-heavy
consumer shapes; spring overshoot on work surfaces; any web dependency on
Material implementations (orphaned); the refuted "4× faster" claim — cite
CHI 2026 33%/20%.

**From Fluent 2 — adopt:** global/alias token layering (already PDS-shaped);
blur-indexed shadow ramp with dark-mode opacity scaling and luminosity-aware
shadows on brand surfaces; the Immersive/Assistive/Embedded AI altitude
vocabulary for agentic surfaces; HAX-style trust patterns (AI-content
marking, fallibility notices, latency-as-designed-state). **Adapt:** 16-stop
brand-ramp injection becomes the PDS tenant ramp generated with contrast
targets (below) — including tintable neutrals, which Fluent locks. **Avoid:**
grey default neutrals; motion that stops at 200ms fades; confining
luminescence to an AI brand layer only.

**From Geist — adopt:** agent-readable spec files (`design.md` per
grammar × mode) generated from tokens; intent-encoded 10-step color scales
(steps 100–300 backgrounds, 400–600 borders, 700–800 solids, 900–1000 text);
the two-family materials model (surface vs. floating) as the depth contract;
editorial per-component content rules (Verb+Noun CTAs, empty-state taxonomy,
command-menu behavior: query preservation, backspace pops page stack,
commands act-not-browse, motion-free); the `:focus-visible` double-ring
recipe; tabular-nums discipline. **Avoid:** wholesale monochrome austerity
(commodity look, cold for clinical users); shipping without a data grid.

**From Spectrum — adopt:** platform-scale 1:1.25 as the channel mechanism;
Leonardo-style contrast-generated ramps (contrast engineered into palette
generation, not audited after — implement in OKLCH with per-mode contrast
targets); React Aria as the accessibility conformance floor — its published
behaviors (keyboard/SR-accessible drag and drop, virtualized collections,
i18n/RTL) become PDS acceptance criteria, and `react-aria` hooks may be
evaluated for new complex widgets (grid virtualization, DnD, combobox)
without replacing shipped PDS components; the design-data direction (tokens +
component API/anatomy JSON schemas + MCP server). **Adapt:** `genai`-variant
concept maps onto PDS violet intelligence tokens rather than a gradient
brand. **Avoid:** theming refusal; retrofitting t-shirt sizes into token
names (documented breaking-change trap); consistency-without-per-surface-QA
(Photoshop backlash).

**PDS-owned differentiators no benchmark ships** (the reveal story):
restrained expressive motion on web enterprise; data-grade density with
craft; governed dual-grammar + tenant theming with generated contrast;
published AI experience doctrine (semantic AI light + attribution +
explainability + revert-to-AI + latency states); agent-readable design data
with governance metadata; judicious luminescence (Precision Daylight canvas,
Chromatic Signal, Connected Fabric within ADR 0014 constraints).

### Capability Dependencies (Not Standing Queues)

These capabilities describe dependency order. They are not separate teams,
standing queues, admission authority, or permission to create worktrees.

**Foundation decisions — accepted.** ADR 0015 is ratified; ADR 0016 adopts
React Aria Components behind PDS-owned APIs; current Inter Variable and Geist
Mono assets are self-hosted with provenance. Changes remain evidence-gated.
On 2026-07-22 the Product Owner adopted the official PDS Health brand palette
as the color foundation and a two-tier brand typography contract — Poppins
Bold for display/headline roles (the brand guide's web face, vendored as a
publisher-built latin subset with provenance) and Inter for UI body/data,
with Geist Mono route-scoped; see Decision Provenance. Display-role component
wiring graduates separately with layout-shift evidence.

**Token foundation.** Introduce the DTCG JSON canonical source
generating `pdsTokens.css`/`.ts`; convert values to OKLCH; add reference
tier beneath existing `--pds-*` semantics; add scale (`pointer`/`touch`),
density, window-size-class, and motion-scheme (standard/expressive) tokens;
build the tenant brand-ramp generator with contrast targets (tintable
neutrals included); encode grammar shadow ramps (apple-like translucent
depth; material-like discrete elevation with dark-mode opacity scaling).

**Grammar × mode parity.** Complete the material-like signature (opaque
tonal surfaces, state layers, compact shape, discrete elevation) and refine
apple-like per the craft canon (8–12% alpha border band, concentric radii,
1px top inset highlight, glass on chrome only); verify forced-colors
posture; prove materially different token signatures with identical
component/axe results across all four theme signatures.

**Floor plans.** Tokenize window size classes; ship the floor-plan
catalog — Portal/Home, List-Detail, Workbench (supporting pane),
Dashboard/Feed, Conversation (AI altitude-aware), Flow/Wizard — each with
per-class adaptation and navigation morphing (bottom bar → rail → drawer)
demonstrated in the catalog at both scales.

**Components and controls.** Density variants on Table/List/dense
Form; virtualization-by-default on the data grid with React Aria-level
keyboard/SR behavior as acceptance criteria; command-menu behavior contract;
empty-state taxonomy; focus recipe; wizard/complex-form patterns; editorial
content rules added to component docs and checked in review.

**Behaviors.** Roll the ratified motion vocabulary through components
(standard scheme; command palette motion-free); expressive scheme only at
signature moments (work-queue → detail View Transitions as progressive
enhancement, metric count-ups with reduced-motion parity); AI experience
suite — altitude vocabulary, aura/border/popover intelligence tokens,
attribution affordance with explainability popover, revert-to-AI, latency
states, `genai`-equivalent control variant on violet; Connected Fabric
adoption strictly per ADR 0014 gates.

**Agent-readable design data.** Generate
`design.md`-equivalents per grammar × mode from the token source; publish
component API + anatomy JSON schemas; stand up the MCP design-data server;
extend catalog evidence automation to the 16 render states. This capability
is also a reveal asset: it demonstrates agents generating on-system UI.

### Sequencing

Foundation decisions → token foundation → grammar/floor-plan foundations →
components and behaviors. Agent-readable design data may advance beside those
steps once its source contracts are stable. The active product increment pulls
only the deliverables it needs; each immutable checkpoint must pass the gates
below.

## Security, Privacy, And Governance

No auth, tenant-isolation, or PHI surfaces change. Governance items: retain
font licensing/provenance and payload evidence; tenant brand-ramp generation
must enforce contrast floors (4.5:1 text / 3:1 UI) so theming cannot create
inaccessible products; agent-readable design data must expose only design
contracts (no secrets, no internal endpoints) and carries the same review
gate as docs; Connected Fabric remains decorative/`aria-hidden` per ADR 0014;
AI affordances must never present decoration as provenance (violet = meaning
only).

## Acceptance Evidence

| Criterion | Proof Command / Artifact | Required Before |
| --- | --- | --- |
| Repo validation green | `scripts/appfw validate --json` | every deliverable checkpoint |
| Generated surfaces clean | `scripts/appfw generate --check --json` | every deliverable checkpoint |
| Token discipline (no raw values, JSON→CSS/TS sync) | `node scripts/check-pds-tokens.mjs --json` | every deliverable checkpoint |
| Component contracts (APIs identical across all dimensions) | `node scripts/check-pds-components.mjs --json` | every deliverable checkpoint |
| Catalog evidence: 4 theme signatures × 2 scales × 2 densities; persistence; axe; reduced motion; 200% zoom; forced-colors | catalog evidence run (`scripts/check-pds-catalog-evidence.mjs`) | affected deliverable checkpoint |
| Docs and CLI examples valid | `scripts/appfw docs-check --json` | any docs change |
| Work-queue timing proof: motion does not slow repeated work | catalog/product proof artifact | behavior graduation |
| Agent generates a compliant screen from design data alone | design-data demonstration artifact | reveal |
| Handoff record | `scripts/appfw handoff --json` | every deliverable completion |

## Test And Execution Feedback Plan

Unit/gate loops per deliverable via the commands above; browser-backed catalog
evidence extended from the existing harness to cover scale/density states;
axe + keyboard traversal on every touched component; layout-shift
measurement during font cold-load; a repeated-work timing check for
motion. If execution contradicts this spec — e.g., the 1:1.25 scale
ratio proves wrong for clinical hardware, or token JSON generation cannot
round-trip existing values — stop, record the contradiction in Decision
Provenance, and revise the spec before continuing.

## Risks And Controls

| Risk | Control | Owner | Status |
| --- | --- | --- | --- |
| Evidence cost of 16 render states | Automate in the catalog harness; no manual screenshot debt | Implementation | open |
| Grammar drift between apple-like/material-like | Token-signature diff check + identical component/axe evidence per deliverable | Implementation | open |
| Consistency without craft (Photoshop-style backlash) | Per-surface signature-experience briefs (ADR 0014 gate) + editorial rules in review | Review owner | open |
| Font payload regression | Preserve approved provenance and route-level performance evidence before preload changes | Implementation | controlled |
| Expressive motion slows work | Standard scheme default; expressive gated to signature moments; timing proof required | Review owner | open |
| Agent-readable data goes stale | Generated from the same canonical JSON as CSS/TS; drift check in gates | Implementation | open |
| Upstream claims shift (benchmark is point-in-time) | Benchmark doc lists unverified items; re-verify before load-bearing reuse | Product Owner | open |

## Tech Debt And Follow-Up

Existing ad hoc density styles migrate to density tokens as affected products
touch them. Hex tokens coexist with OKLCH until the canonical token migration.
Native-mobile scale contract is reserved but unimplemented (roadmap-gated).
React Aria Components is the adopted web interaction substrate behind PDS APIs;
product code must not import it directly.

## Handoff Notes

Agents: read [UX Design Strategy](../frontend/ux-design-strategy.md) first —
it is the decision framework for every design choice in this plan, including
the in-flight material-like theme work (it has a dedicated directive section
for that). Use the human-readable hierarchy Outcome → Product Increment →
Deliverable → Task. Admit work through the current vertical product increment;
load capability specialists only for bounded deliverables. Route every task
through `docs/start/agent-task-map.md`; prefer `_config`/generator surfaces over
generated output; treat ADRs 0014–0016 as non-negotiable; finish every
checkpoint with `scripts/appfw handoff --json`. The benchmark assessment and the
July 15 research are the only sanctioned evidence bases for visual choices —
if a choice lacks a citation in either, it needs new research or an explicit
fiat decision recorded here.
