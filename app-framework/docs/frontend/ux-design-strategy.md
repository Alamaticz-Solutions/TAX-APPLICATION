# PDS UX Design Strategy

Status: draft operating guidance for review. This strategy extends — and can
never override — [PDS Health Design System](pds-health-design-system.md) and
[ADR 0014](../architecture/adr/0014-pds-signature-visual-language.md). It is
written to be executed by agents (Codex) and reviewed by humans.

## Purpose And Authority

This document is the **decision framework** that turns the experience thesis
into reproducible design judgment. The operating contract says what the system
is; the visual-language contracts say what it looks like; the
[dimension spec](../specs/pds-experience-dimensions-and-catalog-plan.md) says
what to build in what order. This strategy says **how to decide** — for every
theme, floor plan, component, control, and behavior — so a thousand small
decisions land as one coherent experience instead of locally reasonable
inconsistency.

Precedence when documents disagree:

1. ADR 0014 and the operating contract
   ([pds-health-design-system.md](pds-health-design-system.md));
2. this strategy;
3. workstream specs (including the dimension spec); then
4. component-level docs.

A conflict is a stop condition: record it in the relevant spec's Decision
Provenance and get review; do not resolve it silently in code.

Sanctioned evidence bases for visual and interaction choices:

- [modern-ux-styles-research-2026-07-15.md](../assessments/modern-ux-styles-research-2026-07-15.md)
  (adversarially verified landscape, craft numbers);
- [design-system-benchmarks-2026-07-17.md](../assessments/design-system-benchmarks-2026-07-17.md)
  (Material 3/MUI, Fluent 2, Geist, Spectrum).

**The evidence rule:** every aesthetic or interaction choice either cites one
of these bases or is recorded as an explicit fiat decision with an owner and a
revisit trigger. "It looks better" is not a rationale; "the craft canon's
8–12% border band, chosen by fiat at 10%" is.

## The Strategic Bet

> **Calm precision. Expressive intelligence. Unmistakable craft.**

The 2026 benchmark shows every major system handles expression by exclusion:
Geist bans it, Spectrum ships it static, Fluent confines it to Copilot
branding, and Google validated it (CHI 2026: 33% faster fixation, 20% faster
completion) then stranded it off the web. Nobody makes expression *mean*
something about the user's work.

That is the PDS bet: **the lead comes from placement of energy, not amount.**
Every unit of light, color, motion, and surprise is spent where it carries
meaning — state, progress, intelligence, connection, completion — and nowhere
else. This is how the reveal reads as "finely crafted, efficient, intelligent,
luminous" instead of decorated: the system visibly *knows what matters*.

## The Six Altitudes

Every pixel belongs to exactly one altitude. Each altitude has an energy
budget; a treatment legal at one altitude is a defect at another. When
reviewing any surface, first ask: **which altitude is this?**

| # | Altitude | What lives here | Energy allowed | Never here |
| --- | --- | --- | --- | --- |
| 1 | **Canvas** | Page background, Precision Daylight blue-white light | Luminosity of the canvas itself; Connected Fabric on first-viewport/cross-system surfaces per ADR 0014 | Business meaning, status color, text on raw canvas |
| 2 | **Chrome** | App shell, navigation, command bar/palette, toolbars | Translucency/glass (apple-like only, never stacked), pinned stability | Motion on the command palette; glass on content; density changes |
| 3 | **Work surfaces** | Grids, forms, records, queues — where repeated work happens | Flat, calm, dense, fast; hairline borders; hierarchy from type/spacing/alignment | Gradients, glow, blur, decorative motion, cards-in-cards |
| 4 | **Signals** | Status, data viz, progress, selection, focus | Full Chromatic Signal vivid palette; count-ups on metric moments; non-color redundancy always | Signal colors as decoration; effects as the sole status channel |
| 5 | **Intelligence** | AI attribution, generated views, suggestions, evidence, agent progress | Violet + luminescence **as meaning** (aura/border/glow = "AI touched this"); Embedded → Assistive → Immersive altitude vocabulary | Violet or glow on non-AI content; AI styling as novelty; chat as the default container |
| 6 | **Signature moments** | Orient/Resolve peaks, work-queue→detail continuity, completion receipts, Connected Fabric acknowledgment | The 80/20 budget: expressive motion scheme, spatial continuity, one designed surprise per journey | Anything that slows the second use; expression that replaces labels or known patterns |

The reveal qualities map onto altitudes — this is where each is *earned*:
**efficient/responsive** at 3; **consistent** everywhere via tokens;
**intelligent** at 5; **energy/luminescence** at 4–5 (as meaning) and 1
(as canvas light); **exciting/fresh/invigorating/surprising** at 6, budgeted;
**empowering** in the trust mechanics of Law 7; **finely crafted** in the
seams (Law 8).

## The Ten Laws

Ordered by precedence: **when two laws conflict, the lower number wins.**

1. **Work velocity is sacred.** Nothing may slow the second, tenth, or
   thousandth repetition of a task: no entrance choreography on work surfaces,
   no motion on the command palette, compact density defaults on data grids,
   ≤100ms interaction acknowledgment, INP ≤200ms p75. Evidence: craft-canon
   duration bands; Google's own finding that expressiveness breaking utility
   patterns hurts.
2. **Structure before effects.** Hierarchy comes from typography, spacing,
   alignment, proximity, and stable state — never from gradients, glow, blur,
   or containment. Containment is for repeated actionable items, transient
   overlays, and genuine tools; a section is not automatically a card.
3. **Every effect is semantic.** Light, color, translucency, and motion each
   carry exactly one meaning, system-wide: violet/luminescence = intelligence;
   Chromatic Signal hues = status/data/progress; glass = chrome; Connected
   Fabric = the application fabric itself (decorative, `aria-hidden`, never a
   status channel). An effect whose meaning you cannot name is removed.
4. **Expression is budgeted.** The 80/20 rule from the operating contract:
   familiar, quiet patterns carry routine work; two or three signature moments
   per flagship journey get disproportionate craft. Surprise is rationed to
   at most one designed moment per journey, delightful on first encounter,
   invisible-fast on every repeat. Expression everywhere = no moment
   distinctive.
5. **Motion is meaning in time.** Standard scheme (120/200/300ms, enter/
   standard/exit curves, transform+opacity only) is the default for all work;
   the expressive scheme (springs, spatial continuity, View Transitions,
   metric count-ups) is reserved for altitude-6 moments. Semantic motion roles
   — selection, expansion, continuity, progress, success, warning, recovery —
   not decoration. Reduced-motion parity is a gate, not a variant.
6. **Adapt by contract, never by fork.** Two theme axes (grammar × mode),
   platform scale, window-size-class floor plans, and density preference are
   the only adaptation mechanisms, all token-backed
   (see the [dimension spec](../specs/pds-experience-dimensions-and-catalog-plan.md)).
   Identical component APIs, DOM semantics, keyboard behavior, and business
   meaning across every combination. Respect channel conventions — the
   Photoshop 2026 backlash is what convention-erasure costs.
7. **Empowerment is agency plus evidence.** The user always sees state, next
   action, and consequence: governed-action preview before commit, undo after,
   evidence/permission/freshness beside the action they affect, explainability
   on AI attribution, revert-to-AI after edits. Personalization tunes emphasis
   but never permission, meaning, or required controls.
8. **Craft lives at the seams.** The lead is perceived in the transitions
   nobody specs: focus rings (2px solid, 2px offset), empty states (typed,
   positive, one CTA), loading choreography (<1s nothing, then skeletons that
   match real rhythm), error recovery, `tabular-nums` on every number,
   editorial copy (Verb+Noun actions, domain nouns, no "Get Started"),
   `::selection`, scroll shadows. A system is judged by its worst seam —
   consistency without per-surface craft reads as cheap (the Photoshop
   lesson).
9. **One meaning, many expressions.** Semantic `--pds-*` tokens own meaning;
   grammars, modes, scales, and density re-express values without ever
   re-meaning them. Apple-like says hierarchy with translucency, fine borders,
   and calm depth; material-like says the same hierarchy with opaque tonal
   surfaces, state layers, and discrete elevation. If a change alters what
   something *means* in one grammar only, it is a defect.
10. **Evidence over taste.** Cite the evidence bases or record a fiat
    decision. Never cite the refuted claims (the "4× faster" figure; the NN/g
    glassmorphism citation). When execution contradicts the evidence, stop
    and record — do not improvise a third direction.

## The Decision Procedure

For any UX decision — new token, component state, theme value, floor plan,
motion, AI affordance — run:

1. **Altitude:** which of the six altitudes is this? Apply its energy budget.
2. **Laws:** which laws bind? On conflict, lower number wins.
3. **Evidence:** what do the evidence bases say? Cite it, or record fiat.
4. **Guardrail sweep:** contrast (4.5:1 text / 3:1 UI on every tinted
   surface), reduced-motion parity, keyboard path + accessible name, density
   behavior (data surfaces honor `data-density`; overlays never densify),
   API invariance across grammar × mode × scale, token-only values.
5. **Record:** decision provenance in the owning spec; evidence artifact via
   the standard gates (`check-pds-tokens`, `check-pds-components`,
   `check-pds-catalog-evidence`, `docs-check`, `validate`, `handoff`).

Quality bars are the operating contract's, unchanged: 100ms acknowledgment,
INP ≤200ms p75, WCAG 2.2 + axe evidence, 15%/10-point operational
superiority and 65% blinded preference + one-point composite for signature
graduation.

## Applying The Strategy To The Catalog

- **Themes** are two *expressions of the same laws* (Law 9). Grammar work is
  legitimate only in token space; parity evidence (identical component/axe
  results, materially different token signatures) is the proof.
- **Floor plans** are Orient-Understand-Act-Resolve made spatial: each of the
  six floorplans declares its altitude composition (chrome, work surface,
  signal, intelligence placement), window-class adaptations, and navigation
  morphing. A floor plan that cannot say where intelligence lives is not
  done.
- **Components** carry the seams (Law 8): every component ships all states
  (loading/empty/error/partial/stale/unauthorized), focus, density behavior,
  editorial copy rules, and non-color status redundancy as API defaults, not
  product afterthoughts.
- **Controls** express feedback per grammar: apple-like acknowledges with
  fine borders, subtle depth, and fluid response; material-like with state
  layers and discrete elevation — same timing tokens, same semantics.
- **Behaviors** are where the lead is felt: standard motion everywhere,
  expressive motion at signature moments, the AI suite (attribution,
  explainability, preview, revert, latency-as-designed-state) on altitude 5,
  Connected Fabric strictly within ADR 0014's constraints.

## Immediate Application: Material-Like Theme (in flight)

The current material-like work is the first test of this strategy. Directives:

- **Same laws, different grammar.** Material-like re-expresses hierarchy with
  opaque tonal surfaces, compact shape, state layers, and discrete elevation
  (per [visual-themes.md](../../appfw_ui/pds_health/components/visual-themes.md)).
  It does not get its own meanings, spacing logic, or component variants.
- **State layers are the interaction idiom:** hover/press/selected as
  token-backed overlay alphas on tonal surfaces, replacing apple-like's
  border/translucency emphasis — with identical timing tokens (Law 5) and
  identical semantics (Law 9).
- **The glass rule does not transfer:** material-like chrome is opaque tonal;
  elevation discipline (discrete levels, dark-mode shadow-opacity scaling per
  the Fluent-derived ramp) replaces translucency. Backdrop blur is reduced,
  not re-imagined.
- **Signals and intelligence are untouched:** the Chromatic Signal palette,
  violet-as-AI, attribution affordances, and Connected Fabric constraints are
  grammar-independent. If material-like needs a signal to look different,
  that is a semantic-token conversation, not a grammar edit.
- **No vendor copying:** the grammar evokes tonal-surface logic without
  a Material runtime, dynamic color, or indiscriminate consumer rounding.
  Work surfaces stay on the compact PDS shape hierarchy; full rounding is
  reserved for roles such as buttons, chips, segmented controls, and
  indicators where the grammar calls for it. Material component anatomy and
  interaction remain the evidence source for the material-like expression.
- **Authentic motion, not generic tweening:** material-like controls use
  Material state layers and component-specific motion. Floating field labels
  animate between measured resting/floating geometry with a crisp transform;
  do not animate `top` and `font-size` as a visual approximation. Preserve
  interruption, reduced-motion, and stable-shell behavior.
- **Fix defects once, in semantics.** If material-like work exposes a flaw
  that also exists in apple-like (contrast, focus, density), fix the semantic
  token or component — never patch one grammar into divergence.
- **Evidence:** token-signature diff between grammars; identical component
  API/axe results across grammar × mode; catalog Style/Mode persistence; the
  full 16-state matrix once scale and density land.

## Codex Handoff Protocol

1. Read, in order: this strategy → operating contract §Experience Thesis and
   §Signature Experience Quality Standard → ADR 0014 → dimension spec →
   the two evidence bases. That set is self-sufficient; do not import outside
   aesthetics.
2. Apply the Decision Procedure to every design-affecting change, including
   the in-flight material-like theme (directives above).
3. Treat the dimension spec as a capability-dependency map, not a set of
   standing queues. ADR 0015 is ratified, ADR 0016 adopts React Aria behind PDS
   APIs, and current self-hosted font assets are governed by retained evidence.
4. Route tasks via `docs/start/agent-task-map.md`; prefer `_config`,
   `app_gen/src`, and template surfaces over generated output; keep raw
   values in canonical token files only.
5. Verify every deliverable with the standard gates and finish with
   `scripts/appfw handoff --json`. A failing guardrail is a stop, not a
   waiver.
6. Stop conditions: any conflict between documents (record in Decision
   Provenance), any evidence-base contradiction discovered in practice, any
   change that would fork component APIs or business meaning across a
   dimension, and any request to spend expression outside its altitude.
