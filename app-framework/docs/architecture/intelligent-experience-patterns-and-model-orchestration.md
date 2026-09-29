# Intelligent Experience Patterns And Model Orchestration

> **Status: explanatory architecture guide.** This document explains how the
> eight Intelligent Experience (IX) patterns fit together and where AI models
> may participate. The
> [Eight-Vignette Intelligent Experience Shared Foundation R1](../specs/ix-eight-vignette-shared-foundation-r1.md)
> remains normative for recipe identity, capabilities, registration,
> presentation, lifecycle admission, and readiness. Model choices in this
> guide are the recommended provider-neutral V1 architecture; they are not a
> promise that every model path or product integration is implemented.

## The Short Version

IX is not one chatbot and it is not eight generated screens. It is a family of
eight reusable interaction patterns for bringing intelligence into the work a
person is already doing.

All eight patterns use the same architectural spine:

1. The user identifies the work in focus. A later admitted background lane may
   also accept a properly authorized Product event.
2. The backend applies the user's identity, tenant, roles, scopes, and product
   data-access rules.
3. Product code gathers only the context that caller is allowed to use and
   records its sources, freshness, and gaps.
4. Deterministic code, retrieval, and, when justified, one or more AI models
   perform the parts of the task for which they are suited.
5. Product orchestration produces a draft artifact; it does not author
   canonical IX history.
6. App Framework validates and records lifecycle events, artifact revisions,
   cancellation, and replay.
7. PDS renders the validated `pds.ix.presentation@1` model for Web or native.
8. The user-facing pattern keeps the applicable inspect, edit, challenge,
   redirect, stop, or continue controls explicit; each control still requires
   Product/runtime implementation and proof.

The important design principle is: **rules authorize, products provide domain
meaning, models assist, App Framework governs the run, and PDS presents it.**

## Architecture At A Glance

```text
User today; authorized event/agent trigger only after separate admission
                    |
                    v
         appfw.ix_run_request@1
      intent + focus + optional question
                    |
                    v
+--------------------------------------------------------------+
| App Framework IX runtime                                     |
| verify human/workload identity -> bind tenant and policy ->   |
| create run -> emit events -> revision/cancel/replay/audit     |
+---------------------------+----------------------------------+
                            |
                            v
+--------------------------------------------------------------+
| Product-owned context and orchestration                      |
| authorized system queries + prior artifacts + user input      |
|                                                              |
| rules/calculations -> retrieval/reranking -> model(s)/tools   |
|                                      |                       |
|                          untrusted orchestration drafts       |
+---------------------------+----------------------------------+
                            |
                            v
         validated product artifact revision
              pds.ix.presentation@1
                            |
                            v
+--------------------------------------------------------------+
| PDS-owned channel presentation                               |
| Web renderer                 native iOS/Android renderer      |
+---------------------------+----------------------------------+
                            |
                            v
       applicable inspect / edit / ask / challenge / redirect / stop
```

The model is deliberately inside the Product orchestration boundary. It does
not sit between the user and authorization, and it does not directly write
lifecycle events, grant access, choose action authority, or render arbitrary
interface code.

## Who Owns What

| Layer | Owns | Must not own |
| --- | --- | --- |
| Product application and backend | Domain meaning, authorized context queries, product access-policy implementation, product artifact type, prompts, tools, model routing, business language, human edits, and action authority | Canonical IX lifecycle or PDS component behavior |
| App Framework IX runtime | Verified identity binding, enforcement and retention of the context-policy verdict, run state, canonical events, artifact revision rules, cancellation fences, bounded replay, and audit facts | Product semantics, provider-specific prompts, or presentation layout |
| AI and analytic services | Retrieval, classification, ranking, extraction, explanation, planning, forecasting, or synthesis requested by Product orchestration | Authorization, canonical facts, final action approval, lifecycle history, or UI authority |
| PDS contract and renderers | Recipe projection and accessible presentation of context, status, progressive regions, evidence, and announcements | Product data access, provider selection, lifecycle state, or action effects |
| Browser or native client | Capture focus and explicit user input; render state; expose inspect/edit/challenge/stop controls | Deciding what data is authorized or silently expanding model context |

## Authorization Is Enforcement, Not Model Reasoning

"The backend decides what data the user may access" means that normal product
permissions are applied before model invocation. It does **not** mean asking an
LLM to reason about whether access should be allowed.

The enforcement sequence is:

1. Verify the caller and bind the tenant, subject, roles, scopes, session, and
   any on-behalf-of identity.
2. Apply product and data-layer policy, including row-, record-, field-, and
   operation-level access rules where applicable.
3. Apply IX context policy for classification, consent, and model/tool egress.
4. Resolve the permitted Product context on the server.
5. Give the model only that bounded context and only the allowed tools.

An LLM may explain an already-made policy decision if that is useful, but its
explanation neither grants nor revokes access. A model or tool call must execute
under the same user-derived or narrower service authority; it cannot use a
broader credential merely because it is running in the backend.

## Where Context Comes From

Context is assembled by a Product-owned implementation of
`IxProductContextResolver`, not by the PDS renderer and not by a model freely
browsing enterprise systems.

| Context input | Architectural source |
| --- | --- |
| Current focus | The `focus.kind` and `focus.id` in `appfw.ix_run_request@1` |
| Human question or challenge | The optional request question or explicit subsequent user input |
| Caller authority | Verified principal, tenant, roles, scopes, session, consent, classification, and egress policy |
| Domain facts | Server-side Product queries to authorized systems of record, APIs, search indexes, and analytics |
| Prior intelligent work | An exact related artifact identity and revision when the user continues or revises work |
| Provenance and freshness | `IxContextSummary` source references, refresh times, freshness state, and known gaps |
| Human edits | Product-owned artifact or draft state; editing alone does not imply a new model call |

The Framework carries detailed authorized context in an in-memory
`IxProductContext`; the canonical IX lifecycle does not persist that full
Product payload. Before constructing the context, Product authorization and
the Product resolver exclude credentials, access tokens, API tokens, security
tokens, bearer tokens, and other secrets. Otherwise-authorized classified
Product context is not categorically forbidden by the Framework type.
Framework construction bounds portable JSON structure and serialized size; it
does not interpret Product semantics, detect secrets, or grant data-policy
approval. Product code can clone and retain the context and therefore remains
responsible for its storage, classification, consent, retention, and egress.
The type's absence of `Debug` and `Serialize` reduces accidental exposure but
is not a lifetime, persistence, or policy guarantee.

## Model Types Used By IX

"AI model" is not synonymous with "large language model." A good IX flow can
use several kinds of models, or no generative model at all.

In this guide, **model** means an inference, ranking, or predictive model. It
does not mean the product source configuration stored under `.appfw/model`.

| Model or engine | Best suited to | Poor choice for |
| --- | --- | --- |
| Deterministic rules and calculations | Permissions, thresholds, arithmetic, dates, policy, workflow state, exact filtering, and validation | Ambiguous language synthesis |
| Embedding model | Finding semantically similar records, passages, or prior artifacts | Writing the user-facing answer or making policy decisions |
| Reranker | Reordering retrieved candidates for relevance after access filtering | Open-ended reasoning or forecasting |
| Classifier or lightweight ranking model | Categorization, urgency prediction, relevance scoring, and composition hints | Complex explanation or multi-step planning |
| Fast generative LLM | Grounded follow-ups, concise summaries, extraction, rewriting, and low-latency explanation | High-consequence analysis without validation |
| Deep-reasoning LLM | Multi-source analysis, competing hypotheses, scenario comparison, and plan decomposition | Simple formatting, permissions, arithmetic, or every keystroke |
| Forecasting, anomaly, or optimization model | Time-series prediction, anomaly detection, scheduling, resource allocation, and constrained optimization | Natural-language presentation by itself |
| Agent or workflow orchestrator | Sequencing retrieval, models, tools, validation, and checkpoints across a task | Acting as an authorization or evidence source |

An agent is a controlled runtime pattern that may call models and tools. It is
not itself a special model class.

### Recommended V1 Model Portfolio

V1 does not require eight separate foundation models. A practical portfolio is:

- one strong structured-output reasoning LLM for complex analysis, strategy,
  and planning;
- one fast, economical LLM for grounded follow-ups, summaries, extraction, and
  explanations;
- one embedding model plus a reranker for retrieval;
- deterministic rules, analytics, and validators for exact behavior; and
- specialized forecasting, anomaly, or optimization models only where the
  product problem and evaluation evidence justify them.

The server-side Product orchestrator selects the route based on intent, risk,
latency, cost, data classification, and quality evidence. Neither the browser
nor the first LLM call chooses its own provider or silently escalates to a more
permissive model.

## The Eight Patterns: Interaction And Model Map

The normative eight-recipe contract prescribes **no model invocation at all**.
It standardizes the interaction capabilities and runtime/presentation seams,
while Product teams remain free to satisfy a recipe deterministically. The
following table is a provider-neutral V1 architecture recommendation layered
over that registry.

| Pattern | Interaction and context | Recommended optional model invocation moments | Model types indicated for the task | Deterministic responsibilities and output |
| --- | --- | --- | --- | --- |
| **1. Analyze Why** | The user opens a signal, metric, record, or work item and asks why it changed. Context normally includes its history, related events, comparisons, source documents, freshness, and known gaps. | After authorized context is resolved: retrieve and rerank evidence, then synthesize likely explanations. Invoke again only when the user submits a challenge, follow-up, or requested revision—not when merely editing text. | Embeddings and reranker for evidence; deep-reasoning LLM for competing explanations; fast LLM for simple cited follow-ups. A causal/statistical model may be added when the domain truly supports causal inference. | Calculate deltas and chronology exactly; preserve citations and uncertainty; publish an editable working brief rather than claiming unproved causation. |
| **2. Contextual Conversation** | The user asks about the current screen or a selected artifact without restating all context. The conversation inherits the authorized focus, exact artifact revision, sources, and human edits. | On each explicit submitted question or one-click suggested prompt. Retrieve before generation when the answer needs more evidence. Escalate from the fast route only when complexity or risk warrants it. | Fast grounded LLM by default; embeddings/reranker for evidence; deep-reasoning LLM for a complex challenge or comparison. | Bind every answer to current focus and revision; retain citations and gaps; never treat chat history as authorization or the system of record. |
| **3. Adaptive Composition** | The experience changes which approved PDS regions or actions are emphasized as role, task, device, urgency, or work state changes. The user can inspect why the composition changed. | Recompute when meaningful context changes, not on every render. A model is optional for relevance scoring or a short explanation of the change. | Rules first; classifier or learning-to-rank model for relevance; fast LLM only for labels or explanations when useful. | Enforce permissions and required content; choose only registered PDS recipes/components; keep a stable fallback. No model-generated arbitrary UI code. |
| **4. Working Goal Plan** | The user turns an outcome into a revisable plan with milestones, dependencies, owners, assumptions, and checkpoints. Context includes authoritative constraints and current work state. | During initial decomposition, after an explicit user revision, or after an authoritative dependency changes enough to require replanning. | Deep-reasoning or planning LLM for decomposition and alternatives; constraint solver, graph algorithm, or scheduler for feasibility; fast LLM for concise revision explanations. | Validate dates, dependencies, permissions, and resource constraints; preserve human-owned commitments; emit a versioned plan artifact, never silently execute it. |
| **5. Adaptive Information Lens** | The user sees the most relevant view of a large record for the current task, with an obvious path to the complete authorized record. Context includes task, role, channel, preferences, and record metadata. | Usually none. Invoke a ranker when field relevance is learned, or a fast LLM when the user requests a concise summary. Recompute on meaningful focus/task change, not every paint. | Rules and lightweight ranking by default; optional classifier or fast summarization LLM. | Apply field permissions before ranking; retain required disclosures; provide `full-record-fallback`; never use model relevance to hide safety-critical facts. |
| **6. Situation to Strategy** | The user converts a situation into options, assumptions, tradeoffs, scenarios, and a recommended direction that can be challenged. Context includes objectives, constraints, evidence, trends, and decision criteria. | On initial synthesis, when the user changes a scenario or assumption, and when the user explicitly challenges the recommendation. Run specialized simulation when a scenario changes, then let the LLM explain its output. | Deep-reasoning LLM for synthesis and comparison; forecasting, causal, simulation, or optimization models when warranted; fast LLM for presentation. | Calculate KPIs and scenario math outside the LLM; expose assumptions, uncertainty, provenance, and alternatives; keep the recommendation advisory. |
| **7. Attention Stewardship** | The system maintains a calm, prioritized view of what needs attention and why. Context comes from authorized signals, deadlines, dependencies, freshness, ownership, user corrections, and operating rules. | On a schedule or meaningful signal change for detection/ranking—not continuously. Invoke an LLM when consolidating signals or explaining priority. User feedback can become later ranking evidence; it need not trigger immediate generation. | Rules and anomaly detection for candidate signals; ranking or learning-to-rank model for priority; fast LLM for explanation; deep reasoning only for complex conflicts. | Enforce mandatory alerts, deduplicate, apply freshness and quieting rules, show why an item moved, and let the user correct attention without silently suppressing required work. |
| **8. Agents Helping You** (`ambient-agent-continuity`) | An authorized event, schedule, or user request starts bounded work that can continue while the user is away. The user sees progress, partial artifacts, evidence, checkpoints, and can stop or redirect. | At task-specific stages: interpreting the goal, selecting a bounded plan, processing tool results, revising after feedback, and composing artifacts. Ambient operation is event-driven; it is not a perpetual LLM call. | The task-appropriate mix of retrieval, extraction, reasoning, planning, forecasting, and summarization models coordinated by an agent/workflow runtime. | Authorize every data/tool step; checkpoint state; validate drafts; preserve partial results; implement cancel/replay; require explicit authority for effects. Durable hosted background execution remains separate from the current foreground lifecycle proof. |

## One Pattern Can Use Several Models

Analyze Why illustrates why model composition matters:

```text
authorized records
      |
      +--> deterministic timeline and metric calculations
      |
      +--> authorized retrieval scope --> embedding search --> reranker
                                                        |
                                                        v
                                             reasoning LLM synthesis
                                                        |
                                                        v
                                      citation and schema validation
                                                        |
                                                        v
                                            editable artifact revision
```

The reasoning LLM should see the exact calculations and the selected evidence;
it should not recreate either from memory. A follow-up such as "summarize that
for an executive" can use the fast model against the existing artifact, while
a challenge such as "what evidence contradicts this?" may return to retrieval
and the reasoning model.

## When There Should Be No Model Call

Do not invoke a model merely because the experience is labeled intelligent.
These operations are deterministic unless a separately accepted product need
proves otherwise:

- authenticating the caller and applying permissions;
- reading the complete authorized record;
- calculating exact totals, dates, thresholds, and policy outcomes;
- saving a human edit;
- expanding evidence or showing provenance;
- pinning, sorting, acknowledging, or dismissing an item;
- cancelling, replaying, or resuming a run;
- rendering an existing artifact revision; and
- executing an action after its separate authorization and validation.

Typing into an editable region also does not call a model. A visible, explicit
control such as **Ask**, **Revise with AI**, or **Send suggested prompt** creates
the invocation. The product should preserve the edit and identify exactly
which artifact revision the request is based on.

## Runtime And Data Contracts

| Contract or type | Architectural job | Authority owner |
| --- | --- | --- |
| `pds.ix.recipe_registry@1` | Closed ordered registry of the eight recipe identities, intents, renderers, capabilities, and projection readiness | PDS |
| `pds.ix.recipe_registration@1` | Binds one canonical recipe to a Product-owned artifact type | PDS contract plus Product registration |
| `appfw.ix_run_request@1` | Starts work with an intent, focus, optional question, and optional related artifact revision | App Framework |
| `IxContextClaims`, `IxContextRevision`, and `IxProductContext` | Bind verified caller authority and carry server-resolved authorized Product context | App Framework policy plus Product resolver |
| `IxOrchestrationDraft` | Carries Product-produced phase, artifact, wait, partial, or completion proposals; never a canonical event | Product orchestration |
| `appfw.ix_event@1` | Canonical ordered lifecycle facts, including context resolution, phases, revisions, waiting, cancellation, and terminal outcome | App Framework |
| Product artifact type, for example `product.owned.artifact_type@1` | Gives the artifact its domain meaning and persistence semantics | Product |
| `pds.ix.presentation@1` | Closed channel-neutral presentation data: identity, announcement, optional context/work status, progressive regions, evidence, editability, and changed-region markers | PDS |
| `appfw.ix_cancel@1` | Idempotent cancellation request; durable progress is recorded through canonical IX events | App Framework |
| `appfw.ix_replay@1` and replay checkpoint | Reconstruct canonical history from a cursor without rerunning the model | App Framework |

There is intentionally no shared "model ID" or prompt contract in the recipe
registration or presentation envelope. Provider, model, prompt, tool, and
domain-context choices stay behind Product orchestration and governed runtime
configuration. This keeps a recipe portable across providers and lets model
routes improve without changing the PDS API.

## Canonical IX Trust Boundary And Deferred Projections

The current modular IX runtime is authoritative. Server-derived
`IxContextRevision` and `IxExecutionAuthority` bind context and execution
identity; `appfw.ix_event@1` records canonical lifecycle, sequence,
cancellation, and terminal truth; canonical replay/checkpoint and audit records
preserve replay and audit truth; and exact Product artifact identities and
revisions remain the artifact truth. Replay reconstructs retained state without
rerunning a model. This is a restatement of the current modular contracts, not
a selection or reinstatement of earlier monolithic or donor IX runtime and
documentation bytes.

Product remains authoritative for domain meaning, detailed-context
authorization and handling, storage, classification, consent, retention, and
egress policy. External event, UI, agent, or transport vocabularies do not
grant identity or permission and do not become IX lifecycle, evidence, or
artifact authority.

`agent_event`, AG-UI, A2UI, A2A, and a browser agent runtime are deferred. The
current architecture makes no compatibility, qualification, Product-adoption,
or readiness claim for them. Linked-run and child-run/artifact lineage,
snapshot-v2/v1-read compatibility, and a cross-channel outcome/qualification
contract are likewise deferred until a current Product need and a separately
admitted design exist.

Any later interoperability adapter requires a separate decision. It must be a
server-derived, read-only, fail-closed projection of exact canonical IX state:
it cannot mint identity or permission, accept browser-asserted authority,
write canonical state, define lifecycle/replay/cancellation/audit truth, or
mint Product artifact truth.

## Lifecycle Seen By The User

The runtime can represent this common interaction sequence:

```text
acknowledged
    -> context resolved, with sources/freshness/gaps
    -> understanding / gathering / resolving / interpreting / composing / checking
    -> one or more partial or revised artifacts
    -> waiting for user, when judgment is required
    -> completed | partial | cancelled | failed
```

Cancellation and replay are lifecycle operations, not provider features.
Useful partial artifacts survive cancellation when they have already been
validated and committed. Replay returns retained canonical events and must not
silently execute the model again.

## Model Output Is A Draft, Not Truth

Product orchestration must treat every model response as untrusted input:

- validate structured output and size limits;
- resolve citation identifiers against the authorized source set;
- calculate exact values outside the LLM;
- distinguish observed facts, inferences, assumptions, and recommendations;
- surface freshness and known context gaps;
- refuse or narrow when evidence is insufficient;
- retain the model route/version and useful operational telemetry under the
  applicable privacy policy; and
- turn the result into a Product draft that App Framework validates before it
  becomes an artifact revision or canonical event.

The user-facing interface should expose conclusions and evidence, not private
chain-of-thought. Quality should be evaluated per pattern: grounded-answer
accuracy for conversation, explanation quality for Analyze Why, constraint
validity for plans, ranking usefulness for attention, and forecast calibration
where predictive models are used.

## Current Readiness And Nonclaims

The repository currently provides the shared eight-recipe registry,
registration contract, PDS presentation envelope, Web prototype projection,
native projection APIs, and a provider-neutral foreground IX lifecycle with
authorized Product context and orchestration extension points.

The current Nexus My Work reference vertical deliberately uses deterministic,
providerless Product orchestration. It proves the seam and the truthful
fallback—not a live LLM-backed implementation.

Nexus currently implements Attention Stewardship as that local Product
vertical. The other seven patterns have catalog demonstrations and generic
package/runtime support, not seven additional live Product journeys.

That foundation does **not** by itself prove:

- a live model or retrieval implementation for all eight recipes;
- production data-source or enterprise identity integration;
- model quality, safety, latency, or cost for a Product journey;
- durable hosted background agents or cross-device continuation;
- action-effect authority or autonomous execution;
- native iOS or Android qualification; or
- deployment, release, SRA, CAB, or production readiness.

Web projection readiness is `prototype`. Native iOS and Android are
`not-qualified` until exact-platform accessibility, visual, compatibility, and
device evidence exists. Agents Helping You expresses the intended ambient-work
interaction pattern; durable background execution must be proven separately
from the current bounded foreground runtime.

## Canonical Sources

- [Eight-Vignette Intelligent Experience Shared Foundation R1](../specs/ix-eight-vignette-shared-foundation-r1.md)
- [PDS IX Presentation Contract](../../appfw_ui/pds_health/ix-presentation-contract/README.md)
- [Canonical recipe registry](../../appfw_ui/pds_health/ix-presentation-contract/registry/pds.ix.recipe-registry.v1.json)
- [PDS IX presentation schema](../../appfw_ui/pds_health/ix-presentation-contract/schema/pds.ix.presentation.v1.schema.json)
- [App Framework IX runtime module](../../appfw_runtime/src/ix/mod.rs)
- [Product orchestration extension boundary](../../appfw_runtime/src/ix/orchestration.rs)
- [AI Chat And Search provider/runtime contract](../runtime/ai-chat-search.md)
- [Agentic UX interaction guidance](../frontend/agentic-ux.md)
- [PDS Health Enterprise Design System](../frontend/pds-health-design-system.md)

## Role Card Check

- **Card used:** Architect Agent, with a read-only Framework Structure Steward
  review of documentation placement.
- **Within role:** explained accepted architecture, separated normative
  contracts from recommended model orchestration, and added a discoverable
  engineering guide.
- **Authority not assumed:** no Product acceptance, model/provider selection,
  production readiness, security approval, package publication, deployment,
  release, or accepted risk.
- **Routed decisions:** Product teams own domain/model choices and acceptance;
  PDS owns presentation; App Framework owns lifecycle; security and data owners
  own access and egress policy.
- **Drift signal:** model-routing recommendations must not be promoted into
  shared contracts without separate evidence and architecture review.
