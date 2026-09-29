# App Framework Product Development Strategy — North Star

## What this document is

This is the durable "why we build this way, and where we are taking it" document
for App Framework. It sets direction across velocity, quality, security, and
platform reach (including mobile), and states what "good" looks like for the
agents, harnesses, and processes that build on this framework.

It does **not** duplicate other canonical docs — it routes to them and adds the
layer they deliberately omit:

| Existing doc | What it owns | What this doc adds |
| --- | --- | --- |
| [Platform Strategy](app-framework-platform-strategy.md) | Business-value rationale, capability-reuse thesis, ROI model, and platform ownership boundaries | The durable engineering and agentic-development principles needed to realize that value safely |
| [Roadmap](../release/roadmap.md) | Current readiness numbers, production blockers, next priorities (self-assessment) | Why those priorities matter strategically, and what comes after them |
| [Product Management Strategy](app-framework-product-management-strategy.md) | Demand streams, product promises, market/trend vetting, ranked value themes, packaging strategy, PM waves, and business-value review criteria | The durable principles that the PM overlay turns into portfolio and product decisions |
| [Enterprise Product Readiness Assessment](../assessments/enterprise-product-readiness.md) | Independent audit of current state | An input to this doc's gap analysis, not a duplicate of it |
| [Architecture Overview](../architecture/overview.md), ADRs | Current design and point-in-time decisions | The multi-release thesis those decisions serve |
| [Agent Task Map](../start/agent-task-map.md), [CLI Reference](../reference/cli.md) | Present-tense contracts: what commands exist today | What capabilities agents and harnesses should be able to assume *tomorrow* |

Read this document to decide what to build next and why. Read the docs above to
find out how to build it and prove it works. If this document and the Roadmap
ever disagree on a fact about current state, the Roadmap wins — update this
file.

## Strategic Goal Statement

App Framework exists to make PDS Health dramatically faster and safer at
turning business intent into secure, scalable, production-ready intelligent
applications.

It should become the PDS-owned **model-driven application production system and
capability runtime**, aligned to the enterprise kappa architecture: a reusable
system of product patterns, application archetypes, integration contracts,
optional UX components, data/governance models, agentic development harnesses,
and release evidence that lets teams build
citizen-developed apps, modernized legacy apps, greenfield products, and
Nexus-adjacent experiences without reinventing architecture, security,
observability, CI/CD, and integration every time.

It is not trying to replace ServiceNow, Salesforce, Snowflake, Workday, Epic,
or other enterprise platforms. ServiceNow should own enterprise workflow,
fulfillment, cases, service management, and orchestration where it is the right
system of action. The strategic target is a coherent PDS-owned enterprise web
and native-mobile experience, with App Framework as its application-production
system and capability fabric. Employee Slate/Moveworks is the strongest
comparator and a credible fallback, not the default target. App Framework owns
portable PDS experience, context, operation, evidence, attention, intelligence,
telemetry, product-lifecycle, and quality contracts while mature systems of
record, workflow, search, identity, delivery, and infrastructure remain rented
or integrated where they have better economics.

This strategy starts from a substantial current foundation, not a blank-sheet
goal. App Framework already leads for PDS-owned applications in entity and
relationship modeling, full-stack generation, generated API tests,
forward-only database migration discipline, provider-neutral semantics, native
MongoDB execution, source-controlled CI/CD and release evidence, deep product
UX, and deployment portability. The current provider-graduation report covers
13 providers, 105 graduated semantic areas, and zero promotion violations;
MongoDB has 15 live-certified semantic areas.

Those strengths do not prove the full fabric. MongoDB application execution is
mature, while end-to-end kappa execution remains incomplete: there is no
production Kafka broker client, durable checkpoint, certified projection loop,
or retained replay, lag, coverage, disaster-recovery, and support proof. Native
mobile is maturity 3: generated Expo routes, token bridge, entity screens, and
a policy-aware client exist, while device, distribution, security, and support
proof do not. Scalability guardrails are maturity 4, but PDS-scale evidence for
the modeled 17,000-user/1,100-location workload is maturity 2 and must not be
inferred from local controls.

Evidence basis as of 2026-07-10:

- `scripts/appfw framework provider-graduation --json` generates
  `target/appfw/provider-graduation.json` for provider/semantic-area posture;
  database live certification remains provider-test evidence, not a general
  SaaS execution claim.
- `scripts/appfw product mobile-test --json`, `--run-local`, and
  `--device-preflight` generate product-scoped legacy diagnostics for generated
  foundations, local execution, and tooling posture. They are
  non-authoritative, always keep candidate and release readiness false, and do
  not prove managed device/distribution posture. A future source-bound checker
  owns candidate evidence; named humans own distribution and release.
- focused runtime tenant-isolation and MCP role/scope tests, plus the
  threat-model litmus, support the current application-authorization posture;
  they do not certify delegated SaaS authorization or projected entitlement
  fidelity.
- `scripts/appfw framework validate --json`, `generate --check --json`,
  `test --fast --json`, and retained CI/release artifacts support engineering
  foundation claims; managed production maturity still requires the named
  proof obligations below.

Multi-tenancy has the same two-level boundary. Tenant-isolated data execution
is maturity 4: authenticated tenant context composes centrally with policy and
provider filters, fails closed when missing, has positive same-tenant and
negative cross-tenant database certification, and propagates into audit, MCP,
SaaS planning, and Kafka contracts. A managed multi-tenant control plane is
maturity 2: provisioning/offboarding, configuration and secret partitioning,
quotas and noisy-neighbor control, tenant-aware migration, residency,
per-tenant backup/restore, cost allocation, and support remain unproven.

Do not model all 1,100 PDS locations as tenants by default. A tenant is an
independent security and operational isolation boundary. Location, market,
department, and legal entity are normally policy dimensions unless they need
independent administration, secrets, retention, encryption, deployment,
residency, restore, or service boundaries.

Permission authority is federated, not centralized in one shell. Employee
Slate/Moveworks retains its managed administrator RBAC, plugin availability,
and permission-aware search when used as a comparator, coexistence surface, or
fallback. Source systems or delegated identity remain authoritative for their
records and actions. App Framework's source-controlled, deny-by-default Rego,
mandatory row filters, tenant isolation, provider enforcement, redaction, and
audit govern PDS application data and operations across web, mobile, API, MCP,
and future event ingress. A launch rule only answers who may invoke a plugin; it
never authorizes the target record or action. Ambient, scheduled, and event
agents require a bounded service identity and explicit on-behalf-of context
when human authority is needed.

Projection permission fidelity is part of the first kappa proof, not later
hardening. Copying data into MongoDB or Kafka leaves the source enforcement
boundary. Every stream-fed archetype must retain entitlement provenance,
policy/schema version, dynamic field masks, revocation and freshness SLOs,
tombstone/reassignment/group-change behavior, fail-closed stale handling,
source reconciliation, and negative tests across web, mobile, API, MCP, Kafka,
cache, export, and AI paths. Use source-native authorization or a proven ReBAC
service for high-cardinality object ACLs; keep Rego as the contextual PDS
application policy and obligation engine. Do not build a general search
permissions index in App Framework.

The durable strategic goals are:

1. **AF-SG01 — Accelerate business value.** Reduce the time from business need to
   working, governed application by giving teams a golden path for app
   creation, modernization, integration, testing, review, release, and
   maintenance.
2. **AF-SG02 — Own the coherent PDS enterprise experience.** Deliver one
   recognizable PDS web/native-mobile product across employee orientation,
   service, cross-system journeys, deep work, and exact resumption while
   ServiceNow and other SaaS platforms retain workflow and record authority.
   Use portable object, permission, projection, operation, attention, module,
   evidence, and outcome contracts so the experience can evolve without
   becoming a second workflow engine.
3. **AF-SG03 — Create a governed integration fabric.** Make SaaS APIs, databases,
   streams, and internal systems easier to connect safely through standard
   provider contracts, provenance, identity/principal handling, data
   classification, testkits, and release evidence.
4. **AF-SG04 — Make intelligent apps real, not just chat.** Build a
   provider-neutral search and reasoning integration layer that turns governed
   context into typed evidence, explanation, recommendation, prepared work,
   governed attention, bounded adaptive layout, and measured outcomes across
   web and native mobile. Keep PDS Health AI, model providers, and vendor
   surfaces replaceable; keep every material action on the authoritative
   permission, operation, checkpoint, audit, and telemetry spine.
5. **AF-SG05 — Protect the enterprise.** Bake in security, privacy, PHI/PII handling,
   observability, SRA/CAB evidence, threat modeling, release gates, and review
   discipline so speed does not come from bypassing governance.
6. **AF-SG06 — Create a high-velocity agentic delivery system.** Use agents to increase
   throughput with human-review leverage, clear branch ownership, living
   boards, focused PR review, CI evidence, tech-debt capture, and guardrails
   against large unreviewed churn.
7. **AF-SG07 — Build a product factory, not a one-off codebase.** Treat App
   Framework and each downstream product harness as versioned, documented,
   packaged, clean-machine-replicable products. Use a first-class Product
   Experience Model to separate source intent, agent-proposed treatment, and
   the human-approved target; let Claude Code, Codex, and future workers
   operate through the same product-owned contracts, checks, previews,
   evidence, and upgrade lifecycle.

For employee-facing work, the strategic target is PDS-owned, but the proof bar
is comparative and economic. Employee Slate/Moveworks and ServiceNow already
provide role canvases, journey stages and tasks, notifications, configurable
layouts, widgets, forms, branding, accessibility, web/mobile reach, and
contextual AI. Those capabilities and visual polish are table stakes. Compare
the same outcome, journey, personas, moments, and channels; retain the vendor
experience as the strongest baseline and fallback; and narrow the PDS front-
door scope if it cannot satisfy trust gates, improve selected work outcomes,
or establish a supportable operating model. PDS ownership must earn its cost
through coherent cross-vendor work, permission-preserving projections,
governed actions, structured intelligence, exact resumption, deep or external
products, native-device value, product-factory reuse, retirement, and
materially better repeated economics. App Framework must not become another
journey workflow engine or enterprise search index.

The shortest decision rule is: App Framework should help PDS build better
business applications faster, with less reinvention, stronger governance,
better integrations, smarter user experiences, and enough agentic automation
to move quickly without losing human judgment.

## Current Execution Authority

The North Star defines durable direction; it does not maintain a producer
queue. The [Roadmap Outcome Goal Register](../release/roadmap.md#outcome-goal-register)
is the sole current sequencing authority. AF-OG01 is the active product-neutral
readiness goal, AF-OG05 is a bounded disjoint reference-experience proof, and
the real employee product, source read, projection, action, operations, and
intelligence outcomes remain gated by the register's explicit exits and human
activation decisions.

Once a real product vertical is activated, at least 70% of implementation
capacity should terminate in that vertical or consumer; bounded delivery-system
simplification should remain at or below 20% except for mandatory security or
evidence-truth defects. Contracts, dashboards, evidence, components, Kafka, and
provider breadth earn priority only when a named product consumes them in the
same horizon.

The forcing function is a vendor-adopted Request/Task/Approval or equivalent
ServiceNow work shape inside a bounded PDS-owned product, not a new cross-vendor
canonical model or a duplicate workflow shell. If subsequent lanes end mainly
in types, specs, fixtures, evidence profiles, or dashboard state, the program
is increasing rigor without increasing product value and must be corrected.

## Strategic Freshness Contract

These strategic goals are durable, but they are not frozen. They should be
kept fresh through evidence, not protected as stale doctrine.

Review this North Star whenever one of these signals appears:

- a Nexus, ServiceNow, Pega-retirement, or enterprise-platform direction changes;
- a downstream product team cannot consume the framework cleanly;
- repeated PR review, CI, release, SRA, CAB, or support findings expose a
  structural weakness;
- market or industry research changes the risk/opportunity picture for AI,
  MCP, agentic development, platform engineering, security, or UX;
- a roadmap wave creates activity without a clear business owner, metric, or
  adoption path;
- a goal is being cited to justify work that does not create measurable
  business value, safety, maintainability, or delivery throughput.

Freshness review is a responsibility of the Strategist/Product Manager
function, with the Product Owner translating accepted changes into backlog and
roadmap updates. The XO keeps the active lane board aligned to the current
strategy and escalates drift; the XO does not redefine strategy.

Every freshness review should answer:

1. **Keep:** Which goals remain unchanged because evidence still supports them?
2. **Clarify:** Which goals need sharper language, boundaries, or examples?
3. **Reprioritize:** Which goals should move up or down because business value,
   risk, or adoption evidence changed?
4. **Retire or defer:** Which ideas were useful but are no longer worth active
   investment?
5. **Propagate:** Which roadmap, product-management, operating-model, role-card,
   review, or CI/harness documents must be updated so agents and humans act on
   the current strategy?

The goal is continuity with learning: the framework should keep a clear true
north while improving its expression as PDS business needs, product evidence,
and industry practice evolve.

## Direction And Vision

App Framework is PDS Health's enterprise application product platform: a
contract-first generator, runtime, experience, integration, evidence, and
agent-governance system for turning business-process intent into governed web,
mobile, and intelligent workflow products.

It is not merely a code generator, a developer convenience layer, or a chatbot
wrapper around SaaS systems. It is the paved operating model for building and
maintaining PDS applications with speed that the enterprise can trust.

The direction is explicit:

1. **From prototype to governed product.** Citizen-built prototypes, workbook
   apps, and vibe-coded experiments enter through intake, classification,
   model proposal, generated ownership, review, release, and SRA evidence
   instead of being hardened by guesswork.
2. **From legacy estate to capability renewal.** Legacy apps are modernized by
   behavior, workflow, data, and risk, not by transliterating old architecture
   into new generated code.
3. **From framework checkout to packaged platform.** Product teams should
   consume App Framework through approved CLI, crate, UI, template, connector,
   CI, and agent-harness packages with compatibility and upgrade evidence.
4. **From fragmented surfaces to a PDS-owned experience fabric.** ServiceNow, Workday,
   Salesforce, Epic, iCIMS, and similar platforms remain systems of record,
   workflow, and action where they are strong. PDS owns the portable model,
   context, action, event, intelligence, evidence, attention, experience, and
   observability contracts that let those capabilities appear coherently in a
   PDS web/native-mobile product and, where useful, vendor or embedded surfaces.
   The product earns broad adoption through measured outcomes and lifecycle
   economics, not by duplicating vendor engines or screens.
5. **From chat feature to governed application intelligence.** Structured AI
   results and agentic capabilities resolve through the same identity, policy,
   operation, evidence, and human-review controls as deterministic UI and APIs,
   then render in the vendor or PDS surface where the work belongs.
6. **From local proof to release authority.** A capability is not mature until
   docs, CLI, skills, code, generated artifacts, tests, release evidence, SRA
   package evidence, observability, and operational ownership tell the same
   story.

The flagship proving ground is a **PDS-owned enterprise experience product**:
a real downstream App Framework consumer that uses ServiceNow-centered workflow
and fulfillment without becoming a second workflow engine. Nexus is a leading
product candidate once its authority and first journey are named. The product
must prove transparent requests/tasks/approvals, governed SaaS reads and
actions, reusable application archetypes, exact mobile continuation, durable
attention, and trustworthy structured intelligence. Employee Slate/Moveworks
is the strongest comparator and fallback. The target is composition and PDS
experience ownership, not vendor feature parity: current evidence, gated
capability, funded target, and managed-production proof must remain visibly
distinct in every roadmap, dashboard, and readiness claim.

For the next horizon, prioritize work that makes the framework more consumable,
reviewable, secure, observable, packageable, and reusable across four demand
streams: citizen-developed applications, legacy modernization, greenfield
products, and Nexus. Work that produces local demos but not reusable contracts
or retained evidence is not strategic progress.

**Sourcing note (2026-07-05, rev 6 - product vision clarification):** Rev 6
adds the Direction And Vision section and links the North Star more explicitly
to the product-management strategy overlay, so roadmap, architect,
integration, review, and future product-app threads share the same product
intent. Prior note (2026-07-04, rev 5): A
targeted refresh checked the North Star against current enterprise agentic-AI,
coding-agent, and secure-SDLC direction. The convergence is strong: Microsoft
frames AI agents as digital team members that need ownership, onboarding, and
performance measurement; Google Gemini Enterprise and AWS AgentCore frame agents
as governed workflow platforms with central visibility, permissions, policy,
security controls, and observability; OpenAI and Anthropic agent tooling center
guardrails, tracing, permissions, sandboxing, skills/hooks, and human review;
OWASP's Agentic Top 10 and NIST AI RMF/SSDF frame agentic risk, trustworthy AI,
and secure software development as design-time and runtime control systems; and
2026 empirical work on agent-authored PRs shows small, well-scoped, CI-clean
changes merge more reliably than broad, poorly reviewed changes. This refresh
adds the "2026 Industry Convergence Review" below and makes the framework-code
versus product-code obligations explicit. Prior note (2026-07-02, rev 4):
Commitment 3's kappa paragraph records enterprise-architecture facts supplied by
leadership (SaaS→streams→MongoDB→CDC→Kafka) plus direction derived from a
five-probe repo verification pass; per-vendor stream coverage and enterprise CDC
contracts (connector type, envelope, topic naming, include-list) remain
**evidence artifacts owed by the data platform team** — treated exactly like
vendor API exports, never guessed. Prior note (2026-07-01, rev 3): Internal
claims (crate names, CLI commands, check scripts, file paths) were verified
directly against this repository. The external industry-practice claims in
Parts 1–4 are now backed by an adversarially-verified multi-source research pass
(23 confirmed claims; primary sources Sept 2025–Apr 2026). Key sources: DORA's
2025 *State of AI-assisted Software Development* — "AI as amplifier," platform
quality correlates with unlocking AI value, and throughput rises while delivery
stability falls *unless robust control systems exist*
([dora.dev](https://dora.dev/dora-report-2025/)); Gartner (Sept 2025) on the
delegation gap and tiered/proportional governance; the OWASP *Top 10 for Agentic
Applications* (Dec 2025)
([genai.owasp.org](https://genai.owasp.org/resource/owasp-top-10-for-agentic-applications-for-2026/));
GitHub Spec Kit / spec-driven development
([github.github.com/spec-kit](https://github.github.com/spec-kit/)); and Anthropic
engineering on multi-agent orchestration and Claude Code sandboxing
([anthropic.com](https://www.anthropic.com/engineering/claude-code-sandboxing)).
Part 3's Claude Code / Agent SDK claims are cited to Anthropic's published docs.
Part 5's mobile decision is now backed by a targeted source refresh (React
Native New Architecture, Expo Router, EAS Build/Submit/Update, AuthSession,
SecureStore, and Notifications docs, checked 2026-07-01) plus leadership
direction that mobile-native UX is required. **One honest gap remains,
deliberately not laundered into confidence:** **PHI-pipeline governance and
SBOM/SLSA/signing provenance** were not covered by the research and are carried
as flagged gaps, not settled practice.

**Audience:** framework stewards, product-team engineers, and the coding
agents (Claude Code, Codex) operating on this repo or on downstream products.

**Review cadence:** revisit at each major roadmap re-score, or whenever a new
platform surface (mobile, a new connector archetype) is proposed. Owner:
whoever holds framework-steward responsibilities at the time; update the
sourcing note's date on every substantive revision.

---

## Faithfulness Contract

This document governs the [Roadmap](../release/roadmap.md). The relationship is a
one-way faithfulness contract:

1. **Every Roadmap work item must trace to a commitment or Part in this
   document.** A Roadmap item with no North-Star basis is out of scope — remove
   it, or promote its rationale into this document first.
2. **Every North-Star commitment must have a Roadmap item, or an explicit note
   that it is not yet scheduled.** A commitment with no Roadmap item is an
   execution gap to name, not to hide.
3. **The Roadmap owns current-state facts, scores, blockers, and execution
   order; this document owns direction and rationale.** Neither restates the
   other. On a *current-state* dispute the Roadmap wins; on a *direction*
   dispute this document wins.
4. **No score without a retained evidence artifact.** This binds the Roadmap's
   scorecard (see its Stewardship Rules) and this document's metrics alike —
   optimism is not evidence.
5. **Unverified recommendations stay labeled as such** until their gating
   evidence exists. They must not harden into plan language in either document.
   Mobile now has a direction decision, but implementation still needs retained
   generator, test, and release evidence before it graduates.

**XO note (2026-07-01):** rev 2 reconciled the document with the
completed research pass and corrected the mobile recommendation back to a gated
candidate. Rev 3 records the follow-on mobile decision: React Native + Expo is
the primary mobile target to implement, while PWA is a responsive web fallback
and optional installable web surface, not the mobile strategy.

**XO note (2026-07-02 — rollout pattern codified):** executing
Waves 0–2 surfaced a durable refinement of the evidence-gate discipline that is
now the standard shape for rolling out any new capability:

> **Freeze the contract → ship a report-only slice → ship the enforcement slice
> → collect live evidence → graduate.**

Wave 1 delivered a non-enforcing, artifact-retaining report slice of *every*
lane; Wave 2 turned those reports into fail-closed gates with anti-spoof schema
validation and an aggregate readiness rollup (`wave2-status`). No gate was
bypassed and no score was asserted without a retained artifact — the
Faithfulness Contract held under real execution pressure. Two lessons carry
forward as direction: (1) report-then-enforce beats enforce-first, because the
report slice de-risks the gate design while parallel lanes keep moving; (2)
increment batch size is itself a control system — lane-sized branches/PRs are
the required delivery unit, and a multi-lane mega-branch is an anti-pattern
even when every gate inside it passes (see Part 6).

---

## North Star Statement

App Framework wins by being the most **agent-legible, evidence-gated,
generation-first** path from a declarative schema to a production-grade,
multi-platform product — where humans and coding agents are both first-class
contributors, velocity and safety come from the same mechanisms rather than
trading off against each other, and no single vendor's UX layer becomes a
load-bearing dependency for the experience we ship.

Four commitments follow from that:

1. **Config and contracts are the product; generated output is a projection of
   them.** If a capability doesn't yet have a verifiable contract, it doesn't
   ship — it gets an evidence gate instead (the pattern already used by
   `appfw_provider_servicenow` and `appfw_provider_icims`, see Part 1). This is
   the spec-driven-development discipline now emerging industry-wide (e.g. GitHub
   Spec Kit's Spec→Plan→Tasks→Implement): the contract is the primary,
   agent-agnostic artifact and code is its expression. A contract is the
   authoritative product source, not delivered production capability: count
   capability only after an executable consumer and the required live,
   operational, and support evidence exist.
2. **The framework is built to be operated by agents, not just used by them.**
   Every surface an agent touches should have a deterministic, `--json`,
   scriptable contract — not a UI an agent has to guess at.
3. **PDS capability contracts stay portable, including write-back and
   intelligence.** Backend and experience platforms may own the native surface
   for a journey, but they must not become the only expression of PDS context,
   actions, events, evidence, or intelligence. Governed operations retain
   delegated identity, preview, audit, idempotency, and rollback posture across
   vendor and PDS channels. Structured intelligence uses a versioned answer
   envelope that can render as evidence panels, suggested fields, comparisons,
   queue signals, and safe action previews without requiring a PDS chat shell.

   3(b) further commits the framework to **portable structured intelligence**
   on the same governance spine (architecture contract:
   `docs/runtime/ai-chat-search.md`, UX companion:
   `docs/frontend/agentic-ux.md`). PDS Health AI owns the focused service/API
   semantics; App Framework owns a versioned consumption profile and renderers
   for claim-level citations, entity references, freshness, permission and
   classification decisions, confidence and limitations, information-pack
   identity, suggested values, named action previews, and eval/cost/correlation
   metadata. Every retrieval executes as an authenticated, bounded **human,
   service, or agent principal** through the generated operation dispatcher;
   when human authority is exercised, the principal also carries explicit
   on-behalf-of context. Row-level policy, tenant isolation, redaction, and
   hash-chained audit apply in every channel. A Conversation
   component family may remain an available renderer for qualified products,
   but it is not the default employee front door or a reason to recreate a
   vendor assistant. The application resolves references through its generated
   contract and degrades explicitly from native views to lists to prose flagged
   unverified. **Intelligence proposes; the governed-write path disposes:** any
   action remains an Intent Preview until G1 write evidence and delegated
   authorization exist, and capability claims require retained evaluation
   evidence like every other live claim.

   **Data reach rides the enterprise kappa architecture.** Much of the SaaS
   estate already streams into MongoDB (moving to Atlas), and Mongo CDC
   republishes onto Kafka — so Archetype-1's materialized-read path is
   stream-fed where that coverage exists: the framework reads Mongo projections
   directly and consumes CDC topics through its governed Kafka ingress as a
   change-notification feed for reactive projections. MongoDB is therefore the
   **default** Archetype-1 projection store where stream coverage exists;
   **Postgres remains fully supported** and preferred for join-heavy relational
   reads — the store is a per-schema binding, never a platform bet. Framework
   sync workers remain the pull-fed path for the coverage gaps. Enterprise
   ownership of the pipeline does **not** exempt kappa-fed projections from our
   gates: classification, retention, tombstone handling, freshness SLOs, tenant
   isolation, provenance markers, and echo-loop prevention are evidence-gated
   framework contracts regardless of who moves the bytes — and per-vendor
   stream coverage is itself an evidence artifact owed by the data platform
   team, not an assumption. Per-vendor contracts live in each provider crate at
   `appfw_provider_<vendor>/docs/vendor-contract.md`, version-linked to the
   crate's exported vendor-API pins and gate-enforced (structural, not
   conventional); `docs/runtime/saas-connectors.md` remains the central
   architecture hub that routes to them.
4. **Security is designed in, not retrofitted.** Agent-authored code and
   delegated write-back form a distinct threat class (OWASP *Top 10 for Agentic
   Applications*, Dec 2025 — agent/goal hijacking, tool misuse, identity &
   privilege abuse). Least-privilege tool permissions, both-layers sandboxing
   (filesystem + network), and PHI / provenance governance are gated
   capabilities graduated with evidence — never assumed.

## 2026 Industry Convergence Review

The current industry signal strengthens the North Star rather than replacing it.
The winning pattern is not "add a chatbot" or "let an agent push code faster."
It is: **build a governed operating platform where agents, humans, systems of
record, and generated software all work through explicit contracts, permissions,
evidence, traces, and review checkpoints.**

| Industry convergence | What it means for App Framework code | What it means for product code |
| --- | --- | --- |
| Agents are becoming governed digital workers, not side tools. Microsoft describes agent-era organizations as needing clear roles, ownership, onboarding, and performance measurement for AI agents; Google and AWS package agents as enterprise platforms, not isolated prompts. | Framework commands, skills, gates, provider contracts, and review artifacts must behave like an agent operations control plane. `--json` outputs, retained artifacts, change classification, review briefs, and pre-push human approval are product features of the framework, not internal polish. | Product apps must model real jobs, teams, queues, tasks, approvals, ownership, escalation, and measurable outcomes. A product cannot claim agentic value if the business process is only implied by screens or chat prose. |
| Grounding, permissions, and data governance are the enterprise differentiator. Google emphasizes business-data grounding plus centralized visibility over connectors, permissions, and policies; AWS AgentCore emphasizes controlling every call across MCP, internal APIs, knowledge bases, and functions. | Provider SDKs, QueryIR/DataAccess, policy, projection freshness, lineage, connector certification, and tenant-aware execution must stay central. A connector without provenance, ACL mapping, and test evidence is not production capability. | Product models must carry source, freshness, citation, tenant, role, and data-classification semantics into views, chat answers, mobile caches, exports, and actions. Users need to know what record an answer/action came from and whether it is current. |
| Agent governance is runtime architecture. OpenAI documents guardrails and tracing around LLM calls, tool calls, handoffs, and guardrail events; Anthropic documents permissions, sandboxing, and explicit user responsibility for review. | `change-impact --json`, focused `review-brief --json`, pre-push `review-brief --auto-depth --json`, provider-test evidence, sandbox posture, audit hooks, and trace/export surfaces should become executable gates, not optional docs. Framework code must preserve a single governed runtime operation path across HTTP, GraphQL, MCP, Kafka, sync, and future agent ingress. | Product code must not let chat, ambient assistance, mobile actions, or background automation create alternate write paths. Every action goes through named operations, delegated identity, HITL gates where needed, idempotency, policy, audit, and rollback/undo posture. |
| Secure-by-design now includes agent-specific threats. OWASP's Agentic Top 10 names risks for autonomous systems that plan, act, and make decisions across workflows; NIST AI RMF and SSDF reinforce trustworthy AI and secure SDLC as lifecycle practices. | Agentic threat modeling, supply-chain provenance, secret/PHI scanning, SAST/DAST/ASVS decisions, dependency posture, and release evidence must be first-class framework gates. The framework should fail closed when proof is absent, especially for delegated writes, MCP, and provider graduation. | Product teams must threat-model workflow abuse, prompt injection through records, tenant/IDOR leakage, delegated token misuse, stale projections, unsafe recommendations, and audit gaps before declaring a product slice ready. |
| Coding-agent productivity depends on small slices and meaningful review. 2026 agentic-PR research finds documentation/CI/build tasks merge more reliably, while broader, complex changes fail more often through CI failure, duplicate/unwanted work, and weak reviewer engagement. | Lane-sized branches, generated-boundary discipline, focused checks, docs-check subchecks, framework handoff, and the Framework PR Review Agent are the right operating model. Broad agent-authored branches need integration-train review, not confidence from a single green local command. | Product-code changes should stay slice-shaped around a business outcome: one workflow state model, one generated view, one mobile read/approve slice, one provider evidence increment. Product PRs need human approval briefs that explain business value and residual risk. |
| Employee experience platforms are credible product surfaces, but PDS has chosen to own its durable enterprise experience and application-delivery system. | App Framework must own reusable application archetypes plus experience, model, context, operation, event, attention, intelligence, evidence, and observability contracts. It should not duplicate a workflow engine, search index, foundation-model platform, or connector marketplace. | Build the PDS web/native-mobile product as the target; use Employee Slate/Moveworks as the strongest comparator and fallback. Advance broad adoption only when trust, task outcomes, supportability, signature quality, and lifecycle economics pass. |

The resulting decision rule is stricter than "can an agent build it?":

> Build or approve the change only if it improves a contract-backed,
> evidence-gated path to measurable business value while preserving governed
> human/agent collaboration.

This applies to both code domains:

- **App Framework code** must optimize for generated correctness, agent-legible
  commands, provider evidence, secure runtime seams, reviewable branch shape,
  and reusable proof artifacts.
- **Product code** must optimize for real workflow outcomes, explicit product
  models, user-owned experience, policy-trimmed data access, trustworthy
  actions, accessible/mobile surfaces, and operational evidence tied to the
  business process being replaced or improved.

---

## Part 1 — What Separates Competitive Agentic-Engineering Organizations From Failed Ones

*(Grounded in the adversarially-verified research pass — see sourcing note. DORA
2025 frames AI as an "amplifier" that magnifies an organization's existing
strengths and weaknesses, and finds platform quality correlates directly with
unlocking AI value — i.e. success separates on system-of-work practices, not
model choice.)*

| Practice | What it means in practice | App Framework's current posture |
| --- | --- | --- |
| **Spec/contract-first development** | Config, schemas, and API contracts are reviewed and versioned before code is generated or hand-written; agents generate *from* a contract, not toward a vague prompt | **Have it.** `.appfw/model` schemas, `app_manifest.rs` topology, `config_contract.rs` are the source of truth; `app_gen` projects them into code |
| **Evidence-gated capability rollout** | A capability is marked `Unsupported`/non-executing until authenticated evidence (real API responses, real schemas, real ACL matrices) justifies building the real implementation, instead of guessing at vendor behavior | **Have it, and it's a differentiator.** `appfw_provider_servicenow`, `appfw_provider_icims` reject execution until evidence exists; `APP_MCP_ENABLED=false` is the same pattern applied to the MCP ingress |
| **Harness-mediated agent workflows** | Agents work through a defined harness (Claude Code, Codex, Cursor, Copilot Workspace, Devin, etc.) with scoped tools and permissions, not raw shell access to production systems | **Have it for this repo's own development** (CLAUDE.md/AGENTS.md name Claude Code and Codex explicitly); **gap:** downstream product teams don't yet have an equivalent harness contract documented for *their* agents operating on generated products |
| **Golden-path platform engineering** | One well-paved, generator-backed path for the 80% case; escape hatches are explicit and reviewed, not silent forks | **Have it.** `docs/lifecycle/product-golden-path.md`, generated-vs-human-owned boundary (`generated.rs` vs `<entity>.rs`), `scripts/appfw generate --check` catches drift |
| **Least-privilege permission/sandbox models** | Agents get exactly the tool/file/network access a task needs, nothing more, with explicit allowlisting | **Partially have it.** Repo-level guidance exists; not yet formalized as a per-task permission profile for downstream agent operators |
| **Human-in-the-loop at the right altitude** | Humans review contracts, architecture, and irreversible actions — not every generated line | **Have it in spirit** (generated/human boundary), but this document should make the altitude explicit (Part 6) |
| **Multi-agent orchestration used selectively** | Fan-out only when subtasks are genuinely independent; a single well-scoped agent beats an orchestrated swarm for linear work | **No standing policy.** Worth stating explicitly (Part 3b) so it isn't reinvented per task |

The single biggest signal separating durable agentic-engineering programs from
failed pilots: **the organizations that succeed treat "the agent can't do X
yet" as a gate to formalize, not a gap to route around by giving the agent
broader access.** The verified research corroborates this — Gartner (Sept 2025)
finds broad agent adoption but a large *delegation gap* (only ~15% pursuing full
autonomy; developers fully delegate just 0–20% of tasks), so human-in-the-loop
with *tiered, proportional* governance (not blanket lockdown) is the winning
posture. That is exactly the evidence-gate pattern this repo already uses for
SaaS connectors — it should be the default answer whenever a new capability
has not yet produced retained proof.

---

## Part 2 — Escaping the Velocity/Quality/Security Trade-off

The classic framing — "pick two of velocity, quality, security" — is false
when the following mechanisms are in place, because each one *removes the
tax* that normally forces the trade-off, rather than trading against it:

| Mechanism | What tax it removes | Where it already exists here |
| --- | --- | --- |
| **Generate over hand-write** | Removes the cost of re-reviewing boilerplate on every change | `app_gen` → backend handlers, DB schema, API test scaffolds |
| **Small, verifiable diffs** | Removes the cost of large-batch review (which is where quality and security bugs hide) | `scripts/appfw generate --check --json` (drift detection), artifact provenance hashes |
| **Deterministic `--json` contracts everywhere** | Removes the cost of manual verification — a machine (or agent) can check the result itself | Nearly every `scripts/appfw` subcommand |
| **Evidence gates instead of best-effort integrations** | Removes the cost of shipping-then-firefighting unverified vendor behavior | SaaS connector `Unsupported` states |
| **Policy-as-code enforcement at generation time, not review time** | Removes the cost of a human re-deriving whether access control is correct | Rego policy fixtures (`rego_test`), row-level policy enforcement |
| **Automated secret/PII scanning as a release gate, not a checklist item** | Removes the cost of a human remembering to check | `scripts/ci/secret-scan.sh`, frontend PHI/PII CI guard (ADR-0010) |
| **Supply-chain checks as a routine command, not an audit event** | Removes the cost of periodic, expensive manual dependency review | `scripts/appfw dependency-check --json --strict` |

The pattern: **velocity comes from making verification cheap and automatic,
not from skipping it.** Every item above is a case where this repo already
chose "make the check machine-gradable" over "trust the author."

The verified evidence backs this precisely: DORA 2025 finds AI *raises* delivery
throughput but *lowers* delivery stability — an acceleration that exposes
downstream weakness **unless** strong control systems (test automation, mature
version control, small batches, fast feedback) are in place; teams with those
controls capture the gains, teams without them absorb the instability.
Independent studies that report AI *harming* quality or speed (e.g. METR's 2025
RCT; Apiiro's "4× velocity, 10× vulnerabilities") measure *naïve* adoption —
more code, no gates — which is exactly what the mechanisms above prevent. So the
strategic bet is the **control systems themselves**; raw inner-loop speed is
continuous optimization, not the headline (see Part 7). The question for any new
surface stays "what is the `--json`-checkable contract for this," not "who
reviews this by hand."

---

## Part 3 — Required Capabilities, by Layer

### 3a. AI Coding Agents

- Must be able to discover task routing without guessing: `scripts/appfw
  context --json`, `scripts/appfw lifecycle --json`, `scripts/appfw skills
  --json`, `docs/start/agent-task-map.md`.
- Must be able to explain *why* a file is owned the way it is before editing
  it: `scripts/appfw explain ownership <path> --json`.
- Must be able to verify their own work without human intervention:
  `validate`, `generate --check`, `test --fast`, `docs-check`, `boundary-check`,
  `feature-check` — all `--json`.
- Must hand off cleanly: `scripts/appfw handoff --json` (changed surfaces,
  drift, verification status) as the terminal step of any agent session.

### 3b. Agent Harnesses / Runtimes

*(Grounded in Anthropic's published Claude Code and Agent SDK documentation.)*

- **Project memory conventions** (`CLAUDE.md`/`AGENTS.md`) as the canonical,
  agent-consumed description of repo structure and rules — already in place
  here and should be the pattern downstream products inherit.
- **Skills** for reusable, on-demand procedures (progressive disclosure: light
  metadata loads always, full procedure loads only when invoked) — a better
  fit than bloating CLAUDE.md for anything multi-step (e.g., "certify a new
  SaaS provider," "add a mobile target").
- **Hooks** (`PreToolUse`/`PostToolUse`/etc.) to enforce quality gates
  automatically — e.g., blocking a write to a `generated.rs`-marked file
  before an agent even attempts it, rather than relying on the agent reading
  the ownership doc correctly every time.
- **Subagents** with scoped tools/context for work that would otherwise
  pollute a primary session's context (this document's own research used
  exactly this pattern).
- **MCP servers** for tool/data integration, added only as needed per task,
  not globally — consistent with least-privilege.
- **Permission modes and settings.json allowlisting** so routine, safe
  commands (`scripts/appfw validate`, `test`, `docs-check`) don't require
  per-call confirmation, while irreversible or shared-state actions still do.
- **The Claude Agent SDK** (headless, structured-output, programmatic tool use)
  is the right substrate if/when `scripts/appfw` itself wants to offer an
  "agent-assisted" mode (e.g., a `scripts/appfw propose-model` step that calls
  an agent internally) rather than only being *called by* agents.
- **Multi-agent orchestration has a verified boundary.** Anthropic's own
  research shows orchestrator-worker fan-out wins big on *breadth-first,
  parallelizable* work but explicitly does **not** transfer to interdependent
  coding tasks (shared context, many cross-dependencies), and costs ~15× the
  tokens of a single agent. Standing policy: fan out for independent
  research/review; use a single well-scoped agent for interdependent code
  changes; reserve orchestration for high-value work.
- **Spec-driven development is the named discipline** behind commitment 1: the
  spec/contract is the primary, agent-agnostic artifact and code is its
  expression (GitHub Spec Kit). It is contested only on *efficacy* (specs can
  drift from what's learned during implementation), not on the mechanism — which
  is why this framework pairs specs with `generate --check` drift detection.

Sources: Anthropic Claude Code docs (hooks, subagents, skills, permissions),
Claude Agent SDK / Managed Agents docs (code.claude.com/docs,
platform.claude.com/docs), Anthropic engineering on the
[multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system),
and [GitHub Spec Kit](https://github.github.com/spec-kit/).

### 3c. CI/CD Processes and Automations

- Every quality dimension gets its own narrow, fast, `--json` check rather
  than one slow monolithic "CI job": this repo already does this
  (`policy-test`, `frontend-test`, `api-test`, `provider-test`,
  `boundary-check`, `feature-check`, `dependency-check`, `docs-check`,
  `release-check`). Keep extending this pattern rather than collapsing checks
  together as the framework grows.
- Release gates are evidence bundles, not sign-offs: `scripts/appfw
  release-check --json` and `scripts/ci/release-evidence-check.sh --strict`
  already model this. Any new capability (mobile, a new connector archetype)
  should ship with its own evidence-bundle entry, not a manual go/no-go.
- Supply-chain posture is a routine command (`dependency-check --strict`),
  not a point-in-time audit — keep it that way as new ecosystems (mobile
  toolchains) are added.

### 3d. The Framework Itself (Designed to Be Generated and Edited by Agents)

- **Config is the application; templates are the generator.** Any new
  platform target (mobile) must follow this same split — a new template
  surface under `app_gen/_templates`, not a bespoke one-off generator.
- **Generated-vs-human-owned boundary must be explicit and enforced**, the
  way `generated.rs` vs `<entity>.rs` already works for backend handlers.
  Any mobile scaffold needs the same split from day one, not retrofitted
  later.
- **Provenance and drift detection are non-negotiable for anything
  generated**: `artifact_provenance.json`, `generate --check`. A mobile
  generator ships with drift detection or it doesn't ship.

---

## Part 4 — Security & Governance Baseline

| Requirement | Current status here | Evidence |
| --- | --- | --- |
| Secrets never committed | **Built** | `scripts/ci/secret-scan.sh` (gitleaks-based), redaction in `appfw_saas_core/src/redaction.rs` |
| No PII/PHI in source, fixtures, or tests | **Built, CI-enforced** | ADR-0010 (frontend security & data governance): "Synthetic values only in source, fixtures, and tests. No PII, PHI, or financial values in the repository." |
| Least-privilege data access | **Built** | Row-level Rego policy enforcement, mutation access-filter hardening (per Roadmap security dimension) |
| Supply-chain advisory posture | **Built, routine** | `scripts/appfw dependency-check --json --strict`, `scripts/ci/supply-chain-gate.sh` |
| Least-privilege *agent* tool/write access | **Partial — formalize it** | Repo-level conventions exist; no explicit per-task permission profile documented yet for downstream agent operators |
| Prompt-injection / agentic threat coverage | **Gap — now framed** | Map the threat model to the OWASP *Top 10 for Agentic Applications* (Dec 2025): agent/goal hijacking (ASI01), tool misuse (ASI02), identity & privilege abuse (ASI03) — directly relevant as MCP, agent-consumed `--json` surfaces, and delegated write-back grow |
| Sandbox isolation for agent-run commands | **Environment-dependent — set the bar** | Effective sandboxing needs *both* filesystem and network isolation (either alone is exploitable); done well it also raises safe autonomy (Anthropic reports an ~84% cut in permission prompts). Today this relies on the calling harness (Claude Code's sandboxing), not framework-enforced — make it a stated posture |

The durable rule for anything touching PDS Health data: **classification
propagation, retention, deletion/tombstone behavior, and drift detection are
required *before* a capability goes live** — already the stated bar for SaaS
connector projections (`docs/runtime/saas-connectors.md`) and should be the
bar for any new data surface, including a mobile client that caches data
on-device.

Beyond that data rule, **security is a designed-in commitment (see commitment 4),
not a later hardening pass.** Delegated write-back (Archetype-2) raises
identity/privilege-abuse risk directly, so its safety lane — least-privilege tool
scope, both-layers sandboxing, a named/audited mutation registry, and a
write-safety evidence gate — ships *with* it, not after. Two items here are
honest gaps the research did **not** settle and must not be assumed solved:
PHI-pipeline governance for materialized/cached data, and supply-chain provenance
(SBOM / SLSA / signing) for generated artifacts — both need a dedicated pass.

---

## Part 5 — Mobile Strategy (New Capability)

**Current state: generated contract/token bridge.** App Framework generates
production web React/Vite SPA frontends today, and now emits/checks the
framework-owned React Native + Expo mobile contract, PDS native token bridge,
ownership metadata, and scaffold manifest (`scripts/appfw product generate
--target mobile-rn`) for CRM reference validation. The product-owned mobile app
shell remains separate. The PDS Health design system
(`appfw_ui/pds_health/`) is responsive (desktop/mobile viewport testing already
exists in its evidence gate — see `check-pds-catalog-evidence.mjs`), but full
all-entity mobile screen/route generation, touch-native component generation,
and installable-app release certification are not complete yet.

### Options

| Approach | Code/design-system sharing with today's stack | Native capability (push, biometrics, offline) | Build cost | Fit |
| --- | --- | --- | --- | --- |
| **PWA** (installable web app from the existing generated frontend) | Highest — near-zero new generator surface, PDS tokens already responsive | Weakest — push, biometrics, app lifecycle, secure storage, and offline remain browser/platform-dependent | Lowest | Responsive web fallback and optional installable surface; not the mobile strategy |
| **React Native + Expo (new architecture)** | High — same language (TypeScript), mobile-specific generated API/experience contract, PDS *tokens* portable to a native styling layer even though JSX components are platform-specific | Strong | Medium | **Primary mobile target.** Extends the existing React-generation bet while enabling native navigation, push, secure storage, app lifecycle, store/MDM distribution, and device APIs |
| **Flutter / Kotlin Multiplatform / .NET MAUI** | None — different language and UI paradigm entirely | Strong | Highest | Only justified by a requirement React Native genuinely cannot meet |

### Recommendation (ratified direction, evidence-gated implementation)

**React Native + Expo is the mobile direction.** The dedicated mobile refresh
and leadership guidance changed the decision: App Framework needs mobile-native
UX, not a browser UI that happens to render acceptably on phones. PWA remains a
responsive web quality bar and optional installable web surface, but it no
longer serves as the committed mobile proof path.

The first implementation proof should be a **React Native + Expo validation
slice** for the read/monitor/approve workflow class (task lists, approvals,
status, action preview, activity/audit). It must exercise native navigation,
auth, tenant/policy denied states, PDS token bridging, push notification hooks,
secure storage posture, offline/degraded behavior, and TestFlight/Play testing
evidence.

The mobile target should share:

- contract semantics through the canonical generated mobile artifact,
  `mobile/src/generated/appfw-mobile-contract.ts`; Web-generated TypeScript is
  migration or diagnostic evidence only and is never canonical mobile input,
- PDS Health design tokens via a native token adapter,
- auth, tenant, validation, policy, request/correlation ID, and audit semantics,
- the same generated-vs-human-owned boundary pattern as the web frontend.

It should **not** share web layouts one-to-one. Web and mobile share workflow
semantics; mobile owns native navigation, touch composition, sheets/modals,
safe areas, app lifecycle, secure storage, push, device integrations, and store
release evidence. The canonical implementation guide is
[`docs/frontend/mobile-react-native.md`](../frontend/mobile-react-native.md).

**What the generator needs to deepen**, as direction rather than a finished
spec: move the current generated contract/token bridge and entity-route target
toward a fuller `app_gen/_templates/mobile_rn` source-generation surface, expand the
PDS-to-native token bridge, keep `mobile/.appfw-mobile/ownership.json` and the
mobile scaffold manifest authoritative, preserve drift/provenance checks, and
introduce a source-bound candidate checker separate from the legacy
`mobile-test` diagnostics. That successor must bind evidence to exact source,
API/auth behavior, both platforms, Fabric authority, and recovery posture;
named humans retain distribution and release authority.
Until product-owned native workflow generation lands,
`scripts/appfw product mobile-plan --ui-artifact <mockup.html> --json` retains
planning evidence for agents converting mobile HTML mockups into
product-owned React Native source, and `mobile-test --json` records only
non-authoritative diagnostic posture for the expected mobile workspace. It
must keep `candidate_ready:false` and `release_ready:false`.

**Do not** attempt a third path: building the experience natively inside a
vendor platform's proprietary next-gen UX layer as a substitute for a real
mobile strategy. This repo's own downstream decision-making has already
tested that path once (ServiceNow Slate) and it failed for organizational
readiness reasons — see the anti-pattern in Part 6.

---

## Part 6 — Anti-Patterns and Gotchas

- **Editing generated output directly instead of config/templates.** The
  fastest way to make `generate --check` useless and reintroduce the
  velocity/quality trade-off Part 2 is designed to avoid.
- **Bypassing an evidence gate under deadline pressure.** The evidence-gate
  pattern (Part 1) only works if "we don't have evidence yet" is never
  overridden by "we need it by Friday." A gate that can be bypassed isn't a
  gate.
- **Big-bang generator or template changes without `generate --check`
  between steps.** Small-diff verification (Part 2) is the mechanism, not a
  suggestion.
- **Multi-lane mega-branches as the delivery unit.** Observed live in Wave 2:
  one serial branch accumulated ~55 commits / ~99 files / ~22K lines across
  many lanes — un-reviewable as a batch even though every gate inside it
  passed, and it made the parallelism that Wave 0's contract freeze paid for
  impossible to use. Lane-sized branches and PRs are the delivery unit; if a
  branch spans more than one lane's owned surface, split it before review, not
  after.
- **Ungated agent write access to production systems or unvalidated SaaS
  write operations.** Governed write (the deferred delegated-auth /
  Archetype-2 work) exists precisely because "the agent can call the API
  directly" is not the same as "the agent can call the API safely."
- **Merging agent-authored changes without the review checkpoint at the
  right altitude.** Reviewing every generated line is waste; skipping review
  of a contract or schema change is risk. Keep human review pinned to
  contracts/architecture/irreversible actions, not implementation detail.
- **Context/memory mismanagement in long agent sessions** — stale or
  differently-scoped facts carried forward and conflated with current ones.
  This document's own drafting process hit a live near-miss: a prior note
  cited a design-system-specific readiness score (7.7/10, scaffold/
  governance/catalog/adoption sub-dimensions, recorded 2026-06-10) and the
  Roadmap's framework-wide score (9.27/10 release readiness, across entirely
  different dimensions — resilience, maintainability, secure-by-design,
  etc.) was momentarily treated as if it superseded the older, narrower one.
  It doesn't — they measure different things. The correct response, applied
  here, is to verify against the current source before asserting a
  supersession, not to assume two numbers that look alike are the same
  metric.
- **Over-rotating on a single vendor's proprietary UX layer as "the"
  experience strategy.** The concrete, already-lived example: adopting
  ServiceNow Slate as the UX layer for an internal product failed for this
  org's readiness reasons. The lesson generalizes — treat any vendor's
  frontend framework as a possible *connector target*, never as the only
  place the experience can live.
- **Design-system forking.** A product that copies PDS components instead of
  consuming them from source reintroduces exactly the drift problem the
  catalog evidence gates (`check-pds-components.mjs`,
  `check-pds-catalog-evidence.mjs`) exist to catch.
- **Treating "the agent can do it" as equivalent to "the agent should be
  allowed to do it unsupervised."** Capability and authorization are
  different questions; conflating them is how ungated access incidents
  happen.
- **Fabricated or unverified claims presented as fact-checked research.**
  Applies to agents and humans alike — if a research pass fails (tool outage,
  no sources), say so explicitly rather than filling the gap with confident-
  sounding, uncited synthesis.

---

## Part 7 — Aspirational Goals Worthy of Continuous Optimization

These are optimization targets, not commitments with dates. Track them
alongside the Roadmap's scorecard rather than inside it — this document
states *why* they matter; the Roadmap states *where we are* against them.

| Goal | Why it matters | Directional metric |
| --- | --- | --- |
| Shrink lead time from schema/config change to a deployed, verified feature | This is the truest measure of whether generation is actually removing toil | Time from `.appfw/model` edit to passing `release-check` |
| Maximize the share of the product surface that is generated, not hand-written | Every hand-written line is a line that doesn't benefit from `generate --check` drift protection | % generated vs. human-owned lines, tracked per product |
| Shrink the number of capabilities still behind an evidence gate over time — by graduating them with real evidence, never by removing the gate | The gate is the point; the goal is fewer *ungraduated* gates, not fewer gates | Count of `Unsupported`/non-executing capabilities per connector, quarter over quarter |
| Reach mobile-native parity with the web experience for the read/monitor/approve workflow class | This is the concrete, near-term proof point for the React Native strategy in Part 5 | % of "My tasks"-shaped workflows available in the RN mobile client with retained native evidence |
| Keep secrets-in-repo and PII/PHI-in-repo incidents at zero | Non-negotiable baseline, not aspirational — regressions here are the metric | Count of `secret-scan.sh` / CI PII guard triggers, target: 0 |
| Increase the share of downstream products consuming PDS Health from source, with zero forked components | Directly protects design coherence and the catalog evidence gates' value | % products with zero fork findings in catalog evidence |
| Increase the share of agent-proposed changes that pass verification (`validate`, `generate --check`, `test`) without human rework | The real measure of whether the framework is agent-legible, not just agent-tolerant | First-pass verification success rate for agent-authored changes |
| Formalize least-privilege permission profiles for downstream agent operators | Closes the Part 4 gap; today's posture relies on convention, not an enforced contract | Existence and coverage of a documented per-task permission profile |

---

## Part 8 — How This Connects to Everything Else

- **SaaS connectors** ([`docs/runtime/saas-connectors.md`](../runtime/saas-connectors.md)):
  the evidence-gate and Archetype-1/Archetype-2 pattern this document leans
  on throughout is defined there — this doc does not restate it.
- **PDS Health design system** ([`docs/frontend/pds-health-design-system.md`](../frontend/pds-health-design-system.md)):
  the token/component source the mobile strategy (Part 5) extends rather
  than forks.
- **Frontend security & data governance** (ADR-0010): the PII/PHI baseline
  Part 4 cites directly.
- **Platform Strategy** ([`docs/strategy/app-framework-platform-strategy.md`](app-framework-platform-strategy.md)):
  the business-value "why" for App Framework: growth, productivity,
  experience, technology rationalization, trust, reusable capabilities, ROI,
  and explicit ownership boundaries with enterprise platforms such as
  ServiceNow, Okta, Snowflake, and managed AI/search services.
- **Product Management Strategy** ([`docs/strategy/app-framework-product-management-strategy.md`](app-framework-product-management-strategy.md)):
  the PM overlay for citizen-developed apps, legacy modernization, greenfield
  products, Nexus, packaging/adoption, change management, metrics, and
  product-value review criteria.
- **Roadmap** ([`docs/release/roadmap.md`](../release/roadmap.md)): the
  current-state numbers behind every "Built" / "Partial" / "Gap" marker in
  this document. Re-check those markers whenever the Roadmap re-scores.

---

## Ownership & Review

Owned by whoever holds framework-steward responsibility, with XO
marshalling faithfulness with the Roadmap (see *Faithfulness Contract*
above). Review whenever: the Roadmap re-scores materially, a new platform surface
is proposed (mobile being the first), or an anti-pattern in Part 6 is observed in
the wild and needs a new line item. Update the sourcing note's date on every
substantive revision. The external research pass called for by earlier revisions
has now been run and folded into Parts 1–4; the two remaining, deliberately-open
gap is **PHI-pipeline + SBOM / SLSA / signing provenance** (Part 4). Mobile's
substrate decision is now recorded in Part 5; implementation remains
evidence-gated.
