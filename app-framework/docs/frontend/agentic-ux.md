# Agentic UX — Invoked and Ambient AI Influence

> **Status: active frontend design contract.** This page is the real guidance
> behind the PDS design-system line *"Treat AI as task infrastructure …
> avoid isolated chatbot-only patterns as the primary agentic experience."*
> It defines how the UI behaves when AI influences it — both **invoked** (the
> chat/answer surface) and **ambient** (generated views, action
> recommendations, cues, summaries woven into the workflow). It is the
> UX-layer companion to the backend/runtime contract in
> [AI Chat And Search](../runtime/ai-chat-search.md); read them together.
>
> **Slice-2 note:** the PDS design-system doc row now routes here
> (`Contextual agentic assistance → see docs/frontend/agentic-ux.md`). The
> first Conversation component-family slice exists; keep this page and the PDS
> component/catalog evidence aligned as CH5 adapters, CH8 consumer wiring, or
> `CH-AMBIENT` move from plan to implementation.

## Operating principle

AI is **task infrastructure**, not a destination. The user's job — the
workflow, the tracker, the record — is the center of the screen; AI *serves*
it from two modes:

- **Invoked** — the user asks (chat / command). Point-in-time answers rendered
  as governed cards; long-running work moves to the persistent **AgentTimeline**
  panel, never the chat scrollback. (A pure chat box as the primary experience
  is the anti-pattern the design-system line rejects.)
- **Ambient** — the framework offers, unprompted, *in place*: a generated view,
  a suggested next action, a confidence/freshness cue, an evidence summary —
  attached to the workflow object it concerns.

Both modes obey **one governance spine** (below). Ambient is not a lighter-weight
exception: an unprompted suggestion that writes, or a summary that shows data the
user cannot see, is exactly as dangerous as a chat action that does — arguably
more, because the user did not ask for it.

## The governance spine (applies to every AI surface, invoked or ambient)

1. **Grounded, never generated-from-nothing.** Every AI surface resolves typed
   entity references (`answer_envelope@1`) through the product's **own generated
   GraphQL contract in the signed-in user's policy context** — row-level Rego,
   tenant isolation, field redaction. *The AI can never surface what the user
   cannot read.* This is dual enforcement (orchestrator server-side + client
   re-resolve). Ambient summaries and generated views resolve the same way.
2. **AI proposes; the human steers; the governed-write path disposes.** No AI
   surface — chat or ambient — writes directly. Every write-shaped output is an
   **Intent Preview** (`IntentPreview`, G2) requiring explicit human confirm,
   gated on G1 governed-write evidence + W3-B delegated auth. The provider-neutral
   actor/delegated-auth primitives now exist, but durable encrypted token custody,
   provider certification, and executable governed-write evidence are not graduated,
   so today every AI-initiated write is preview-only by construction. Previews
   are **editable and steerable**, not approve/reject only: the user can modify
   the proposed action, and can **interrupt/redirect a running agent mid-flight**
   — "fluid transitions of initiative: delegate, override, co-steer" — resuming
   from a checkpoint (HAX; Microsoft AG-UI interrupt/resume).
3. **Legible as AI.** Anything model-derived is visually distinguishable from
   deterministic UI by a **consistent AI-attribution affordance** — the user
   always knows what is a model inference vs a system fact. No silent blending.
4. **Explainable on demand — and the plan before acting.** Every AI surface
   exposes, without a round-trip: a **confidence signal** (`ConfidenceSignal`,
   calibrated), **"why am I seeing this?"** provenance, **citations** to the
   resolved source records, and — before any consequential action — the
   **plan/reasoning** it intends to follow. "Show the plan before acting" is the
   single highest-trust decision in the current research; `ToolCallStatus` shows
   *what it's doing*, a plan preview shows *what it will do*.
5. **Degrades, never dead-ends or fabricates.** Resolve → native view; partial →
   render what resolved + name what didn't; none → labeled `unverified` prose
   (invoked) or **absence** (ambient — a suggestion that can't ground simply does
   not appear; it never renders broken or invented).
6. **Respects attention (calm).** Ambient AI must not interrupt for low stakes.
   Interruption escalates only with **stakes** (see the risk axes). Proactive /
   long-running items accrue in the AgentTimeline, not as modals.
7. **Reversible and audited.** Anything actioned through AI influence carries a
   correlation id into the hash-chained audit trail and, where possible, an
   **undo/compensation** affordance (`UndoCompensationState`, G2).
8. **User-controllable.** Ambient assistance is dismissible and dial-able (a
   per-surface "assist level"); the user can turn it down without losing the
   deterministic workflow.
9. **Accessible.** AI surfaces meet the same axe gates as all PDS components:
   streaming/live regions use `aria-live`; cues are never color-only; generated
   views are keyboard-navigable.

## The risk axes (govern interruption, confirmation, and autonomy)

Every AI influence — how loudly it surfaces, whether it needs confirmation,
whether it may act — is sized by four axes (the same axes the chat/HITL research
established): **reversibility · scope · confidence · cost**. Low on all four →
ambient, quiet, dismissible, no confirm. High on any → surfaced deliberately,
Intent-Preview-gated, audited. This is the concrete meaning of
"human-in-the-loop proportional to risk tier" (internal AGENT-STD-007 /
AGENT-PRI-006) at the UI layer.

### Autonomy dial — and the cost of oversight

Expose the risk tiers as a **first-class autonomy control**, not a buried
setting: **Suggest** (recommend, confirm each) → **Co-pilot** (act on routine,
ask on important) → **Autopilot** (act and report). Two rules the reputable
research is firm on: set it **per task type, not globally** (schedule vs. send;
read vs. write), and **default to the most conservative** tier. Autonomy and
oversight are a bidirectional coupling — higher autonomy shrinks the range in
which the agent is reliable — so the dial is a safety mechanism, not a
convenience (arXiv 2605.12105).

Oversight is **not free**: constant confirmation causes attentional tunneling
and rubber-stamping — its own failure mode (arXiv 2509.10723). So confirm
*proportional to risk* (don't gate low-stakes reads), and pair per-action
**human-in-the-loop** with **human-on-the-loop** monitoring — system-level
escalation rate, drift, and the AgentTimeline audit — so trust scales without
confirmation fatigue.

## Ambient patterns — the four the user named, made concrete

### 1. Generated views
The AI assembles a **view** — a flow graph of stalled tasks, a filtered list, a
synthesized rollup — but only ever as a **composition of existing governed PDS
components bound to resolved records**. The AI chooses the *arrangement*
(`view_hint`), never renders novel UI, never binds unresolved data.
- **Rule:** a generated view = (resolved refs) × (a registered view in the
  generated `viewRegistry`). If the hinted view isn't supported or refs don't
  resolve, degrade down the ladder (graph → list → card), never invent.
- **Legibility:** the composed region carries the AI-attribution affordance and
  a "generated from your data as of {freshness}" label.
- **Component:** a `GeneratedViewShell` composition slot (new, CH4/CH5-adjacent)
  hosting `FlowGraphShell`, `DataGrid`, entity cards.

### 2. Action recommendations (suggested next actions)
Surfaced *near the workflow* ("3 tasks stalled 6+ days — nudge owners?"), ranked
by the risk axes.
- **Rule:** a recommendation is an **affordance that pre-fills an Intent
  Preview** — it never auto-executes. Accepting opens the confirm surface; it
  does not perform the write.
- **Explainability is mandatory:** every recommendation states *why now* (the
  triggering signal + evidence), or it doesn't surface.
- **Dismissible + learns from dismissal** (down-ranks similar suggestions).
- **Component:** `SuggestedAction` / `RecommendationCard` (new) → `IntentPreview`
  (G2, exists).

### 3. Cues
Lightweight in-context annotations: confidence, **freshness/staleness**,
attention/anomaly markers, and **policy-denied** markers.
- **Rule:** cues **annotate, never block or mutate**. A cue that implies
  certainty it doesn't have is a defect — confidence and freshness are first-class,
  not decorative.
- Every cue traces to provenance (hover/expand → the basis).
- **Components:** `ConfidenceSignal` (spec'd), `FreshnessIndicator`,
  `AttentionMarker` (new); `policy_denied` is already first-class in the contract,
  client, and PDS `FeedbackState` — reuse it, don't reinvent.

### 4. Evidence summaries
AI-condensed rollups attached to a workflow object ("what changed since you last
looked", "why this deal is at risk").
- **Rule:** summaries are **grounded** — every claim links to the underlying
  resolved record; labeled AI-generated; carry a freshness watermark; degrade to
  "summary unavailable" rather than hallucinate.
- **Never the system of record.** A summary is a lens over records the user can
  independently open; it never becomes the authoritative value.
- **Component:** `InsightSummary` / `EvidenceSummary` (new) with an inline
  citation list.

### 5. Memory / context surfacing
When the framework personalizes (remembered preferences, prior context, learned
ranking), memory must be **visible and correctable**, not silent infrastructure —
silent memory is how products "drift into feeling invasive" (NN/g; arXiv agentic-
memory work).
- **Rule:** when adapting from memory, show a quiet **"personalized for you"**
  marker with one-click **reset to default**; make every remembered fact
  inspectable and **correctable** (the user can fix a stale remembered fact).
- Decide per interaction whether to **remember or ask** — never assume.
- Memory is policy- and tenant-scoped like every other AI surface — it never
  carries context across tenants or beyond the user's authorization.
- **Component:** a `MemoryChip` / "why personalized" affordance (new, ambient family).

## Invoked (chat) — the short version
Fully specified in [AI Chat And Search](../runtime/ai-chat-search.md). At the UX
layer: native chat panel (PDS **Conversation** family — `MessageThread`,
`StreamingText`, `ToolCallStatus`, `EntityRefCard`, `CitationList`,
`ConfidenceSignal`, `AgentTimeline`), point-in-time answer cards, the same
degradation ladder, the same proposes-not-disposes rule. Chat and ambient share
the answer-envelope, the resolution path, and the Conversation/ambient component
families — they are one system with two entry points, not two products.

## PDS component reality (exists vs new)

| Need | Status |
| --- | --- |
| `IntentPreview`, `ActionAudit`, `UndoCompensationState` | **Exist** (G2, Actions family) — the action/confirm/undo spine for both modes |
| `policy_denied` state, `FeedbackState` | **Exist** — reuse for the policy-denied cue |
| `ConfidenceSignal` | **Exists in the first CH5 slice** — shared by chat and cues |
| Conversation family (thread/streaming/tool-status/entity-card/citations/AgentTimeline) | **First component-family slice exists — CH5** (invoked); the Nexus CH8 static consumer-wiring proof exists, while runtime adapters, live chat, and release-grade evidence remain gated |
| Ambient family: `GeneratedViewShell`, `SuggestedAction`/`RecommendationCard`, `InsightSummary`/`EvidenceSummary`, `FreshnessIndicator`, `AttentionMarker`, `AiAttributionAffordance`, `AssistLevelControl`, `MemoryChip` | **First component-family slices exist — CH-AMBIENT** (ambient); catalog/checker/static/interactive coverage, visible autonomy/memory controls, plus the Nexus CH8 static consumer-wiring proof exist, while live AI influence remains gated |

Both families are built the framework way: wrap headless behavior where it
exists, **own the surface in PDS tokens**, pass the full 8-surface catalog/checker
budget, and only advance beyond enterprise-ready when static consumer wiring,
product-level visual/a11y evidence, local UX metric baselines, and then live
evidence from the Nexus pilot exist. **Naming landmine:** `activity`/`activities` are
banned source terms in
the PDS checker — use `AgentTimeline`, `AttentionMarker`, etc.

## Guardrails / threat ties (do not treat ambient as low-risk)
Ambient AI **expands** the agentic attack surface, it does not shrink it:
- Prompt-injection & excessive agency reach ambient too — a poisoned record could
  drive a malicious "recommendation." Recommendations are therefore
  preview-only + registry-bound (no free-text actions), same as chat tool calls
  (threat-model-litmus **S2**; OWASP-LLM LLM01/LLM06; ASI01/ASI06).
- "AI shows data the user can't see" is the top ambient leakage risk — the
  grounding rule (dual policy enforcement) is the mitigation (litmus **S7/S9**).
- **The agent's own perception is attackable.** Agents frequently fail to
  recognize manipulative / dark-pattern interfaces and untrusted content, and
  prioritize task completion over protection (arXiv 2509.10723). So a model
  never acts on screen-scraped or model-authored UI: actions bind to the named
  registry, refs bind to `appfw://` resolution, untrusted content is sanitized —
  the refs-as-pointers + registry-binding rules are the mitigation.
- Every AI surface is in scope for the same evidence gates; ambient surfaces stay
  **FAIL** in the litmus until their grounding + preview-gating + attribution are
  demonstrably present, exactly like the chat surface.

## Work items
- **CH-AMBIENT (first slice implemented in `codex/wave4-ambient-ai-family`;
  autonomy/memory controls implemented in `codex/wave4-ambient-memory-controls`):**
  the ambient PDS component family + the
  `GeneratedViewShell`/recommendation/summary/cue primitives, `AssistLevelControl`,
  `MemoryChip`, and their catalog/checker wiring. The first Nexus static
  consumer-wiring proof is in `codex/wave4-nexus-chat-pilot`; the
  component-family catalog evidence gate now runs Playwright/axe over Ambient
  AI grounding, attribution, freshness, attention, and preview-gated suggestion
  semantics. Remaining work is live runtime wiring and live eval proof before
  any release-gated or live-ambient claim. The Nexus CH8 pilot now provides
  product-level visual/a11y evidence plus local UX metric baselines, both
  explicitly marked non-live.
- Chat (invoked) is CH1–CH9, already tracked.

## Verification
Docs-only page; when it or the PDS families change:
```bash
scripts/appfw framework docs-check --json
```
No AI-influence surface — invoked or ambient — may be reported as live until its
grounding, preview-gating, attribution, and eval evidence are retained (same rule
as every other live claim; see the chat-eval and PDS catalog evidence gates).

## References (reputable grounding, verified 2026-07-02)

- **NN/g** — *Generative UI and Outcome-Oriented Design* and *GenUI in Real Life*:
  designers define user goals + constraints; AI composes governed components
  (the anchor for our PDS-owns-the-components stance).
  (nngroup.com/articles/generative-ui)
- **HAX / Internet of Agents** — arXiv 2512.11979: interfaces must support fluid
  transitions of initiative (delegate / override / co-steer).
- **Dark Patterns Meet GUI Agents** — arXiv 2509.10723: agents mishandle
  manipulative UI; human oversight has costs (attentional tunneling, rubber-stamping).
- **Autonomy and Agency in Agentic AI (Regulated Contexts)** — arXiv 2605.12105:
  autonomy↔oversight coupling; layered HITL + human-on-the-loop.
- **Microsoft Agent Framework + AG-UI** — standardized agent↔UI: streaming,
  declarative generative UI, state sync, HITL via tool-approval and
  information-request interrupts.
- **Vercel AI SDK** — generative-UI streaming primitives (the stream protocol we adopt).
- **Smashing Magazine (Feb 2026)** — the plan→approve→steer lifecycle and the autonomy dial.
