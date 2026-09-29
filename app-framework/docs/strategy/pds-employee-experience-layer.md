# PDS Employee Experience Layer Strategy

> **Status: adopted strategic direction, not an executable backlog or current
> production-capability claim.** This document extends the
> [Enterprise App Fabric Strategy](enterprise-app-fabric.md) with a focused
> thesis: **the App Fabric becomes the PDS Health Employee Experience (EX)
> layer** — overlaying, never replacing, ServiceNow, Workday, iCIMS,
> Salesforce, Anaplan, and Oracle Fusion — using approved PDS cloud-native,
> event, data, identity, intelligence, and observability capabilities where
> each product proves the need. Moveworks/EmployeeWorks/Employee Slate research
> is market evidence only; it is not part of the target topology, a committed
> fallback, or an acceptance dependency. Claims below synthesize adversarially
> verified market research (2026-07-15,
> [Moveworks assessment](../assessments/moveworks-servicenow-research-2026-07-15.md))
> and four internal evidence reviews (2026-07-15): the PDS One architecture
> dossier, the `pds-observability-*` platform, the
> `vendor_api_specs_2026-06-26` corpus, and the PDS regulatory/EAC corpus.
> Volatile claims decay: re-verify pricing, vendor roadmap, and API claims at
> negotiation or build time. This strategy does not admit work. Current human
> guidance, the Product Dashboard, and accepted product contracts determine
> priority and admission; durable roadmap and product records must be updated
> when those decisions change.

## Thesis

PDS keeps its SaaS pillars. ServiceNow stays the primary system of action for
workflows and records that belong there. The EX layer is a PDS-owned family of
web and native-mobile products that helps an employee **know what needs
attention, understand why, act safely, and resume anywhere**. It combines
purpose-built experience, governed source capabilities, selective projections,
structured intelligence, and exact continuation without becoming a universal
portal, workflow engine, enterprise search index, or chat destination.

The strategic bet is that PDS's existing assets — the app-framework
application-production system, documented Mongo/Kafka/Flink precedents, the
observability platform, and a codified policy corpus — can compose into an EX
layer with better outcome economics, trust, task experience, lifecycle control,
and product velocity for selected PDS work. Those are hypotheses to prove, not
advantages to claim from architecture alone.

## Strategist disposition and current claim boundary

Adopt the PDS-owned Employee Experience direction with these binding limits:

1. **Prepare now; activate product deliberately.** Use the authorized protected
   readiness window to make package-first development, signature experience,
   product-level agentic guidance, runtime authority, provider simulation,
   web/native-mobile foundations, structured-result seams, and focused
   feedback coherent. The first real My Work product remains held until a new
   explicit human activation decision. Once activated, product acceptance owns
   the reusable contract; framework abstractions do not define value by
   themselves.
2. **Direct source integration.** App Framework integrates directly with
   ServiceNow and other selected systems through named, source-owned
   capabilities. No assistant or employee-front-door vendor mediates source
   authority.
3. **Selective event architecture.** The PDS One kappa design is exploratory,
   and the documented production event flows retain open control and operating
   evidence. Use one direct read first; add one permission-preserving projection
   or event path only when freshness, continuity, composition, or scale makes it
   valuable.
4. **Structured intelligence, chat optional.** Intelligence follows an
   observe-understand-recommend/prepare-checkpoint-act-evaluate loop. Useful
   results persist as typed application state. Conversation is one optional
   invocation surface.
5. **Proof before claims.** Internal hosting improves control and portability;
   it does not confer compliance, security, performance, maintainability, or
   favorable TCO automatically.

Current App Framework has enough foundation for product shaping, but it does
not yet meet the preferred **Nexus developer readiness** bar or production
readiness for this role. Package lifecycle, product-level agent guidance,
signature composition, runtime convergence, provider simulation, focused
feedback, and web/native-mobile continuity still need to become one smooth
path. MongoDB semantics, model-driven generation, web foundations, and code-
native evidence are meaningful strengths. Live ServiceNow execution,
permission-preserving employee projections, governed SaaS actions, a complete
PDS Health AI loop, managed native-mobile operation, employee-scale evidence,
recovery, and durable support remain gated.

## Market evidence, not product topology (verified 2026-07-15)

Moveworks post-acquisition is ServiceNow's front-door strategy: EmployeeWorks
(GA 2026-02-26) and Employee Slate (GA ~2026-05-14) package the Moveworks
reasoning engine + enterprise search with ServiceNow workflows. It is strong:
a mature plan/execute/observe reasoning engine, enterprise search across 100+
content integrations, 100+ marketplace agent templates, Teams/Slack presence,
and an L1 autonomous service-desk offering. These capabilities are useful
market evidence and design input. They create no Moveworks-specific product,
integration, comparison, procurement, or release requirement unless the human
explicitly authorizes a separate evaluation.

Its verified structural weaknesses are useful questions for any external
employee-experience product, not requirements embedded in the PDS topology:

1. **Classic connector service-account writes.** The documented classic
   ServiceNow connector runs submissions as a dedicated bot and uses
   `requested_for` for attribution; whether EmployeeWorks changes that model
   remains unverified.
2. **Knowledge/catalog permission mirroring.** Documented ServiceNow User
   Criteria for knowledge and catalog are evaluated on the Moveworks side;
   guidance warns of ingestion delay and visibility drift. This evidence does
   not establish one fixed sync interval or the authorization path for every
   ticket operation.
3. **Bounded execution.** 60-second whole-process timeout (Agent Studio 2.0),
   200KB HTTP response cap, ~4-hour KB ingest cadence, ambient agents in
   Limited Preview.
4. **Builder governance gaps.** No shipped version history/rollback, no
   pre-deployment approval workflow, advise-only AI copilot, forced
   Classic→2.0 migration precedent.
5. **Experience-layer churn and opacity.** Slate is a two-month-old V1
   (fixed layouts, EC Pro parity "~2027", Commercial DCs only, widgets
   rebuilt); pricing is quote-only amid the April 2026 SKU repackaging;
   ServiceNow is on its fifth UX stack in twelve years.
6. **External-platform actionability.** Any external employee layer remains
   bounded by source APIs, identities, licensing, freshness, and workflow
   semantics. PDS ownership creates the option to deepen a selected journey;
   it does not remove those source constraints.

## Architecture: logical target on the PDS estate

These seven logical planes describe ownership and contracts, not a mandate for
seven services or a broad platform build. Each plane begins with the minimum
capability consumed by the first product and graduates independently.

### 1. Experience plane — PDS products, not a portal skin

App-framework-generated web and React Native mobile products use the PDS
design system and production-shaped floorplans. The first product is a bounded
My Work queue/detail and exact-resume experience, not a universal inbox or
portal. Task-specific UI owns high-frequency work. Structured evidence,
recommendations, prepared values, and previews render through allowlisted PDS
components; conversation may help with ambiguity or exploration but is not a
required shell. The same internal capability/result semantics may support a
future explicitly approved channel without changing source authority.

### 2. Data and event plane — direct read first, projection when earned

PDS One documents an active Epic/MuleSoft/Kafka/Flink/MongoDB precedent at
about 150,000 events/day, but its catalog also records missing RBAC, SLO,
schema/topic, replay, and runbook evidence. The broader kappa design is
explicitly exploratory and scored for selective use. It is therefore a useful
pattern and capacity signal, not a pre-approved Employee Experience topology.

The first product uses a named direct ServiceNow read with explicit source
authorization and freshness. Add one same-object MongoDB projection only when
the product proves a need for lower latency, outage continuity, cross-system
composition, durable history, or intelligence. If Kafka is the justified feed,
prove one concrete producer/consumer, schema, checkpoint, replay, dead-letter,
lag, entitlement, revocation, reconciliation, and fallback path for that same
object. Polling-watermark remains an honest first option where vendors do not
provide certifiable events. Do not launch a generic kappa or connector program.

### 3. Source capability and action plane — direct and governed

The fabric's tiered actionability contract governs every selected operation:
`api_actionable` / `api_readable` / `deep_link_only` / `handoff_only`. For
ServiceNow, define one source-owned capability with three optional transport
profiles: a fixed deterministic REST or Scripted REST read for products; a
named source-side Action, Subflow, or Scripted REST mutation behind AppFW
preview, policy, confirmation, idempotency, audit, and reconciliation; and an
MCP adapter only for an explicitly approved agentic caller. App Framework does
not proxy a caller-selected table, URL, query, or runtime metadata and does not
duplicate ServiceNow workflow rules.

Per-user delegated authorization is preferred for human-present work where the
source and PDS contract support it. Bounded service identity is appropriate
only for named unattended operations. `on_behalf_of` and `requested_for` are
context and attribution, never authorization. The first live read and action
remain hypotheses until PDS-instance evidence proves authentication, ACL/field
behavior, licensing, audit, rate, failure, and revocation semantics.

### 4. Identity and permission plane — source authority plus PDS context

Okta is the workforce IdP; the observability graph API's Okta authentication
and use-case-scoped RBAC are a useful PDS reference. The target performs live
source authorization on material reads/actions and bounded entitlement
propagation on change, targeting the IAM standard's same-day-termination
requirement. Permission-preserving projections carry authoritative
subject/object identifiers, entitlement provenance and version, field masks,
freshness, revocation, and source reconciliation.

The durable enterprise person identifier remains unresolved (PDS One DQ-002;
Clock-ID incidents; Workday tenant refactor in flight). Until an enterprise
anchor is approved, preserve source-native identifiers, mapping provenance,
effective dates, conflicts, and fail-closed behavior rather than inventing a
new identity authority or hard-binding to mutable Workday IDs.

### 5. Intelligence plane — structured, provider-neutral, closed loop

PDS Health AI is the preferred retrieval and information-pack candidate, not
transaction authority or a required chat destination. App Framework assembles
authorized task/entity/freshness context, consumes a versioned structured
result with evidence and limitations, re-resolves referenced entities under
current policy, renders bounded PDS modules, and routes any material operation
through the governed action plane. A capability resolver chooses the least
expensive reliable technique: deterministic rule/query, fresh precomputation,
retrieval/ranking, specialized model, larger model, then a multistep agent only
when needed. Model providers remain replaceable behind that internal contract.
Adaptive layouts are bounded, previewable, accessible, and reversible;
arbitrary generated runtime UI is prohibited.

### 6. Measurement and operations plane — owned outcomes with honest maturity

The observability platform demonstrates useful funnels, stage latency,
dropoff/stuck detection, anomaly baselines, graph evidence, Okta-scoped access,
and mesh telemetry on one validated revenue-cycle use case. Collector
registration may be configuration-only after the product emits stable
identity, state, correlation, duration, outcome, and classification contracts;
the employee funnel itself is not configuration-only.

Current evidence also shows no production overlay, no Kafka collector, a
single-node Community Neo4j manifest, one validated business use case, and
frontend/security hardening gaps. A reported Aura direction still requires a
canonical decision, BAA/residency, compatibility, migration, and operating
proof. The platform is the preferred integration target to graduate, not a
current production operating-profile claim.

### 7. Governance plane — assurance as a build product

The framework can generate source-bound validation, drift, test, review,
release, security, and operational evidence. The regulatory and architecture
corpora show useful alignment with MongoDB, Kafka, OPA, approved coding/model
tools, and evidence-as-code patterns. Alignment is not approval: several agent
standards are draft, PDS One gates remain open, and AI CoE, security, privacy,
data, architecture, release, and support decisions remain role-owned. Internal
hosting does not eliminate threat modeling, SRA/CAB, validation, retention,
recovery, or accepted-risk decisions, and it cannot justify a "no waivers
required" claim.

## Capability priority and maturity gates

| Horizon | Required capability | Advancement rule |
| --- | --- | --- |
| **Protected Nexus developer readiness** | Package-only create-to-preview and genuine upgrade; clear model/config and product harness; sub-two-minute-target focused feedback; generated-host runtime authority; provider test doubles/nonproduction config; production-shaped neutral web/mobile references; bounded structured-result/fallback seam | Active for five committed working days plus a possible five-day extension. No Nexus product, persona, object, repository, live credentials, or release claim. |
| **First lovable product** | Real external package consumer and genuine upgrade; one persona/job; fixture-backed My Work queue/detail; complete loading/empty/error/stale/offline/unauthorized states; signature "why this needs you" moment; accessible web and bounded native-mobile continuation; source-shaped contract; product telemetry and daily user evidence | Held until the protected window closes and the human explicitly activates AF-OG07 with product authority, repo, persona/job, baseline, and success measure. Fixtures make no live-provider or production claim. |
| **Trusted live proof** | Generated-host runtime authority; one version-pinned permissioned ServiceNow read; source identity/ACL/audit evidence; observable failures and freshness; contract parity between fixture and live adapter | No generic table/query proxy, caller-supplied metadata, service-account impersonation, or widened authorization. |
| **Controlled production** | Projection only if justified; one low-risk governed source action; managed deployment, secrets, SLOs, scale/cost, recovery, support, security package, native distribution, release provenance, and behavior-affecting upgrade evidence | Exact launched scope must pass source authorization, tenant, accessibility, resilience, operating, release, and human approval gates. |
| **Differentiated intelligence** | One structured PDS Health AI or selected-provider result; evidence/freshness/limitations; deterministic fallback; recommendation or prepared value; governed preview; web/mobile continuation; quality, latency, cost, correction, and outcome trace | Attach to the proven runtime/read/action spine. Do not create a separate assistant, model platform, or arbitrary layout engine. |
| **Enterprise expansion** | Additional source capabilities, intranet/content surfaces, deep work/admin, governed attention, bounded composition, second real product, and measured reuse economics | Open only from named product demand. A second use must materially reduce normalized engineering/support effort without lowering quality. |
| **Deferred** | Universal portal, workflow-engine replacement, broad connector catalog, general enterprise search/ACL index, generic kappa program, universal agent runtime, speculative vendor adapters, and commercial SaaS-grade tenant control plane | Requires a separate human release decision and evidence that the narrower architecture cannot meet the outcome. |

## Six value hypotheses to prove

Each case is a hypothesis, not an accepted advantage. Measure it against the
current PDS experience and the strongest relevant source-native or explicitly
selected alternative. Moveworks research may inform the benchmark, but no
Moveworks implementation or comparison is required.

### 1. TCO

**Hypothesis: staged PDS ownership can lower fully loaded cost per accepted
outcome or economically enable outcomes the current alternatives cannot.**

- Moveworks-side (verified): quote-only pricing; third-party estimates
  ~$100–200/employee/year with six-figure contracts and services-heavy
  advanced automation; EmployeeWorks/Slate bundling with dual-entitlement
  requirements; April 2026 Foundation/Advanced/Prime repackaging with
  conflicting bundling reports; underneath it PDS still pays every ServiceNow
  and Workday subscription. Formula for the business case: `employees ×
  $/employee/yr × contract years + services`, against the EX layer's `build
  person-months + marginal EKS/MSK/Mongo capacity + product/design/data/
  security/SRE/support labor + integration + compliance + rework`.
- Fabric-side levers (fabric doc, verified 2026-07-07): the ServiceNow
  requester and approver licensing, existing entitlements, and any avoided
  experience SKU are contract-specific inputs that require written terms.
  Archetype reuse, avoided services/support, faster accepted delivery, and
  verified legacy retirement are the durable economic levers.
- Honest counter: the fabric is **not a licensing-arbitrage program** and the
  internal path carries fixed, enumerable compliance cost (AI CoE Tier-2
  gate, SDLC Tier-1 evidence, agent standards, validation cadence) plus the
  45–65 person-month planning range for the enterprise boundary. The TCO case
  must be made per funded journey with real quotes (order-form asks in the
  fabric doc) — never asserted globally.

### 2. UX (best experience, best productivity gains)

**Hypothesis: purpose-built work surfaces plus structured intelligence can
materially improve repeated employee tasks across a 1,100-office workforce.**

- Comparator reality: Slate V1 is conversation-first only (no browse mode),
  fixed home layout at launch, no CSS theming, EC widgets rebuilt, dedicated
  mobile app only H2 2026; the reasoning engine intentionally exposes no
  dialog control; 60-second process ceilings bound what a "conversation" can
  do. Split-view context and widget-returning agents are genuinely good
  patterns — the EX layer should adopt them (conversation with full context
  of the on-screen artifact; interactive results, not just text).
- EX-layer advantages: PDS design system tokens/components as contract;
  named high-frequency tasks get purpose-built screens (approve in one tap
  with policy context, not a chat turn); exact web/native-mobile continuation
  for field staff; honest per-source freshness and degradation labels
  (trust-preserving where a black-box assistant erodes trust on failure);
  accessibility and Spanish-first localization under PDS control.
- What must be true: the PDS employee product ships the AF-RH1 "first
  lovable" bar — product authority, trust gates, comparative baselines. UX
  wins are claimed via measured time-to-task against the Employee Center
  baseline, not via screenshots.
- Honest counter: Moveworks' conversational quality, multilingual reach
  (100+ languages), and Teams/Slack ubiquity are ahead today. The EX layer
  meets employees in Teams via the envelope/adapter path later; it does not
  pretend channel parity at V1.

### 3. Performance

**Hypothesis: direct reads plus selectively materialized projections can meet
task latency and freshness targets without making work depend on an LLM.**

- Comparator ceilings (documented): 60s synchronous process timeout, 200KB
  HTTP caps, ~4h KB ingest, ~24h permission sync, LLM plan/evaluate loops on
  every consequential turn — and its SaaS reads sit on the same vendor APIs
  and rate limits PDS faces.
- EX-layer target: Mongo projections should serve bounded reads within a
  measured product SLO, with QueryIR cost budgets enforced in the generated
  Rust backend; freshness is
  event-driven where CDC exists (150k events/day precedent) and
  polling-watermarked elsewhere with SLOs surfaced in-UI; writes are
  first-try-fast because they call one named mutation, not a reasoning loop.
  Framework performance evidence moves from synthetic to live under roadmap
  P4 (live performance evidence) — the claim matures with that gate.
- Honest counter: an LLM answer over pre-joined projections still pays LLM
  latency; the point is that **the default employee task path is not
  hostage to it**.

### 4. Maintainability

**Hypothesis: a source-controlled, generated, contract-tested product system
can reduce change cost and vendor-churn exposure over its full lifecycle.**

- Comparator: Agent Studio has no shipped versioning/rollback or pre-deploy
  approval workflow; plugin logic lives in YAML DSL + Bender expressions
  inside a vendor UI; the Classic→2.0 forced migration and Moveworks' own
  doc-restructuring churn are the precedent; the experience layer (Slate)
  itself is on a brand-new stack with parity promised "~2027."
- EX layer: everything is git — model/config as source, generated backends
  and UI contracts, `generate --check` drift gates, forward-only migrations,
  OMA adopted models with schema-snapshot diffing and consumer-contract
  tests (tenant-config drift is the dominant vector, and the defense is
  designed), protected-branch PR flow with two reviewers per SDLC Tier 1.
  Vendor churn costs a bounded adapter change instead of an experience
  rebuild — the fabric's core promise.
- Honest counter: PDS owns 100% of the operational burden (upgrades,
  on-call, Neo4j/Aura and MSK care) that a SaaS vendor absorbs; the
  managed-operating-profile and support work (AF-RH3) are the
  gates that make this claim honest.

### 5. Security

**Hypothesis: PDS ownership can provide stronger policy alignment,
inspectability, and revocation control for selected employee data.** It is not
"compliant by construction"; classification and architecture interpretations,
control implementation, testing, approval, and ongoing operation still apply.

- Baseline §18 hybrid-SaaS rule: for sensitive data, **only metadata may go
  to a vendor SaaS tenant**; employee data is C3 Confidential (PII per CPRA
  includes employment data). A SaaS assistant indexing HR/ITSM content into
  its own tenant therefore requires a formal classification, data-flow,
  contract, and security/privacy determination and may require an exception.
  PDS-hosted projections improve data-plane control but still require data
  minimization, access, encryption, retention, recovery, and audit proof.
- IAM standard: same-day leaver disablement and remove-first mover handling
  — a periodic external permission mirror without a proven revocation SLO may
  fail this requirement; the EX layer targets live authorization plus bounded,
  measured revocation.
- Unique-identity attribution and one-service-account-per-integration rules
  make a broad bot account inappropriate for user-authorized writes. Prefer
  source-native delegated identity for human-present work and a bounded,
  integration-specific service identity only where the policy and operation
  permit it.
- Draft AGENT-STD-001/002 specify OPA/Rego action-layer authorization, signed
  capability manifests, audited denials, and **replayable authorization
  decisions**. They are strong target controls and align with the existing
  Rego policy engine, but their draft status and exact applicability must stay
  visible until ratified.
- Vendor AI gets no discount: AI Gov §9.3 applies identical lifecycle
  controls **plus** four-function review, VCR/TPRM/BAA, and CAB submission
  for vendor changes — friction that recurs with every vendor model update.
- Honest counter: the internal path carries the same AI governance tiering
  (Tier 2 is the current planning hypothesis; Tier 3 if PHI/patient scope or
  another high-risk criterion applies), SDLC Tier-1/possibly Tier-0
  obligations (pen tests, abuse-case testing, SIEM event set), and the
  framework must actually close its known gaps (Gate E entitlement model,
  schema registry, live security certification per roadmap P2) — the
  advantage is that this work can be explicit, source-controlled, and partly
  automated, not that it is free. A hard product guardrail either way: no
  AI-derived data
  may feed employment decisions (IT-IS-25; AI Gov §10.2) — design the EX
  layer so interaction data cannot be construed as performance monitoring.

### 6. Feature velocity

**Hypothesis: an agent-native production system can shorten governed lead time
for accepted product outcomes and subsequent reuse.**

- Comparator: customer capability lands at ServiceNow/Moveworks cadence
  (monthly Slate releases; roadmap items like versioning, MCP Workspace,
  Agent Architect V2 arrive when they arrive); Studio building blocks are
  bounded (60s, advise-only copilot, Limited Preview ambient agents); every
  vendor platform change is itself a CAB-relevant supplier change.
- EX layer: the framework is built for agent operation — task maps,
  generated-ownership boundaries, docs-check assertions, evidence gates —
  and approved coding agents can execute bounded work. Task count or generated
  volume is not evidence of velocity. Measure elapsed discovery-to-accepted-
  outcome time, rework, review findings, escaped defects, human intervention,
  and second-use effort. Compounding comes from archetypes: each proven journey
  hardens into a reusable archetype (the framework's actual product),
  compounding instead of renting templates.
- Honest counter: SDLC Tier-1 gates bind internal velocity too (two
  reviewers, security assurance, CAB); velocity must be measured as
  *governed* lead time. The mitigation is the same one the framework sells:
  evidence generation as a build product.

## External benchmark capabilities outside the current build scope

Market products demonstrate capabilities PDS does not need to recreate in the
first product: a mature reasoning engine
and enterprise search at 100M+ document scale on day one; Teams/Slack
channel ubiquity; 100+ installable agent templates; multilingual breadth; an
L1 autonomous service-desk line (Autonomous Workforce, GA ~Q2 2026);
vendor-absorbed operations; and organizational simplicity (one throat to
choke). The EX layer concedes the generic-assistant race deliberately and
may later interoperate with a selected external channel through a narrow
adapter, but protocol compatibility is not current scope. If roadmap gates
fail — no named owner, metrics do not move, source authority stalls, or durable
operations cannot be funded — narrow or stop the PDS product and reassess the
lowest-cost viable alternative. No vendor is preselected as the fallback.

## Delivery alignment (not admission authority)

Use one active vertical product increment, with capability specialists loaded
only for bounded deliverables. For Nexus Add Provider, the useful dependency
order is:

- **Product truth and experience spine:** prove the complete fixture-backed web
  journey and native continuation with honest boundary labels.
- **Connected read and reasoning:** add authoritative ServiceNow reads and
  permission-checked grounded assistance without transaction authority.
- **Governed change and closure:** preview and confirm the named Workday change,
  reconcile it, close the ServiceNow case, and notify the requester.
- **Reveal candidate:** bind the exact web/mobile candidate to design-system,
  accessibility, resilience, observability, support, and product evidence.

These are Product Increments, not standing workstream queues. They use the
human-readable hierarchy Outcome → Product Increment → Deliverable → Task and
do not themselves authorize branches, credentials, release, or production
claims.

- **Trust and source graduation:** before connected product behavior,
  obtain the exact ServiceNow instance version, authenticated dictionary or
  OpenAPI export, selected class/fields/ACL/domain posture, identity modes,
  rate/pagination/failure behavior, and available source-owned Actions,
  Subflows, or Scripted REST operations. Prove one live read before selecting a
  projection or action.
- **Product-led graduation:** add a Mongo/Kafka path, live PDS Health AI
  adapter, or another SaaS provider only when the activated product's measured
  outcome requires it. Acquire Workday, iCIMS, Salesforce, Anaplan, and Oracle
  specifications just in time; do not make provider breadth an early gate.
- **Controlled-production track:** graduate one low-risk source action, managed
  deployment, security evidence, observability, load/cost, outage behavior,
  recovery, support, release provenance, and human approvals for the exact
  launched scope.
- **Policy gates to enter early, in parallel:** AI CoE risk classification
  (Tier 2 is a planning hypothesis; keep PHI out of the first connected
  increment), SDLC tier declaration,
  data classification/contract review, SRA with host/flow diagrams, and a
  Validation Register entry. The owning functions, not this strategy, decide
  final classifications and approvals.
- **PDS One alignment:** register each selected projection as a versioned data
  contract, preserve source lineage, adopt the stable-person-anchor direction
  without inventing authority, and use the vertical to prove one bounded Gate
  E entitlement pattern. OpenLineage/observability integration graduates only
  after identity, classification, and operating ownership are explicit.

## Measurement: how the EX layer earns continued investment

Define per-journey, pre-build, on the observability platform PDS owns:

- **Funnel SLOs:** request submitted → understood → acted on → resolved stage
  latency, dropoff, stuck-item detection, and source hops per office/region.
- **Task outcome:** time-to-understand, completion success, error/rework,
  status chasing, adoption, and repeat usage against the current PDS and
  source-native baseline. An external vendor comparison is optional.
- **Trust metrics:** grounded-answer citation rate, degradation-ladder
  distribution, correction/rejection, complaint rate, and deterministic
  fallback success.
- **Governance metrics:** entitlement revocation lag (target: same-day,
  event-driven), audit completeness, evidence-generation lead time.
- **Experience quality:** five-second and 30-second impression, blinded
  preference when a comparison is selected, task performance, accessibility,
  signature-moment recall, INP, and reduced-motion behavior.
- **Economics:** fully loaded cost per accepted outcome, product/support run
  cost, second-use effort, avoided services/support, and verified retirement.

Stop or narrow when there is no named funded owner, outcome metrics stay flat,
source authority remains ambiguous, support cannot be funded, or freshness and
actionability are too weak to change behavior. Reassess alternatives without
assuming a particular vendor fallback.

## Risks

1. **Identity substrate is actively broken upstream** (durable person ID
   unresolved, Workday tenant refactor, Clock-ID incidents). Mitigate:
   stable-person-anchor keys, event-driven identity propagation, fail-closed.
2. **Delegated-auth evidence is absent in the local vendor corpus.** Prove one
   live ServiceNow read first, then select one low-risk action only after the
   source identity, ACL, audit, licensing, idempotency, and revocation path is
   understood. Do not preselect an approval table from public evidence.
3. **Eventing gap** — polling-first freshness may undercut the "performance"
   story for some journeys; be honest in-UI and upgrade per source.
4. **Single-use-case observability generalization** and no `prd` overlay yet;
   a reported Neo4j Aura direction is not a completed architecture or
   migration decision and still needs a canonical record, BAA/residency,
   compatibility, recovery, and operating evidence.
5. **Capacity/ownership** — the category's failure mode is governance and
   ownership collapse, not technology (Workgrid precedent). The named-owner
   rule is binding; without a funded journey owner this strategy stays on
   the shelf.
6. **Scope expansion** — intranet, search, communications, deep admin,
   additional SaaS providers, and adaptive layouts can turn a bounded product
   into a portal program. Require a named outcome and product consumer for
   each expansion.

## Revisit triggers

- The first product does not improve its named task, trust, or signature-
  experience measures enough to justify continued ownership.
- Direct ServiceNow authorization, audit, or action semantics cannot satisfy
  the selected use without unacceptable privilege, licensing, latency, or
  support cost.
- Identity correlation, entitlement freshness, or source reconciliation cannot
  meet the product's risk and operating SLOs.
- Fully loaded product/platform economics or durable staffing exceed the value
  demonstrated by the first and second consumers.
- AI, data, security, privacy, architecture, or release authorities materially
  change the permissible design or operating cost.
- A later explicit market evaluation demonstrates a materially better option;
  that evidence may trigger a new decision but is not a current dependency.
