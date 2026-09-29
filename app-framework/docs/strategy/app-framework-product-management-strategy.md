# App Framework Product Management Strategy

> **Status: product-management strategy overlay.** This document translates the
> [Platform Strategy](app-framework-platform-strategy.md) and
> [Product Development Strategy - North Star](product-development-north-star.md)
> into product-market focus, audience promises, packaging strategy, wave
> priorities, ranked value themes, trend-vetting tests, and review criteria.
> It does not replace the
> [Roadmap](../release/roadmap.md); the Roadmap owns current readiness scores,
> blockers, execution order, and retained evidence.

## Product Vision

App Framework becomes the internal PDS Health product platform for rapidly
building, modernizing, governing, releasing, and operating intelligent
enterprise applications.

The durable strategic goals are canonical in the North Star's
[Strategic Goal Statement](product-development-north-star.md#strategic-goal-statement).
This document translates those goals into product-management pressure: demand
streams, value themes, packaging choices, review criteria, and backlog
discipline.

It should let teams move from business intent to production-ready product with
less custom glue, fewer hidden risks, clearer review, and stronger evidence
than ordinary enterprise delivery paths. Its value is measured not by how much
code it can generate, but by how reliably it can turn high-value workflow,
data, integration, mobile, and AI needs into secure, maintainable, observable,
PDS-branded products.

The winning posture is platform product plus operating model: a packaged
application-production system, reusable application archetypes, portable
context/action/event/intelligence/evidence contracts, a governed integration
plane, and agent-ready review workflows. The strategic target is one coherent
PDS-owned enterprise web/native-mobile experience. Employee Slate/Moveworks is
the strongest comparator and a credible fallback, not the default target.
ServiceNow remains the workflow, fulfillment, case, service-management, and
system-of-action engine where it fits. App Framework funding should be released
in evidence-backed increments that demonstrate reuse, avoided services or
support, greater product throughput, application retirement, or a structurally
better repeated-use experience.

## Product Management Mandate

App Framework exists to turn high-value business intent into secure,
maintainable, scalable web/mobile intelligent applications faster than ordinary
enterprise delivery paths can, without letting speed create unmanaged risk.

The product-management function protects that outcome across four demand
streams. In the current harness, the **Strategist/Product Manager function**
owns strategic business themes, market challenge, portfolio direction, and
value-generator pressure. The **App Framework Product Owner** owns the current
backlog, value slices, acceptance criteria, product evidence, and priority
recommendations. The split prevents strategic direction from becoming branch
micromanagement while keeping the backlog faithful to the North Star.

1. **Citizen-developed applications.** Vibe-coded or workbook/dashboard PoCs
   become governed products, not productionized prototypes.
2. **Modernized legacy applications.** Existing apps are replaced or strangled
   by capability, with behavioral evidence, migration safety, and release proof.
3. **Greenfield products.** Product owners and engineers get a paved path from
   product brief to generated backend, PDS experience, mobile, release evidence,
   and packaged framework consumption.
4. **PDS enterprise experience and Nexus.** The first employee product is the
   flagship downstream package consumer. It uses ServiceNow-centered workflow
   and fulfillment while App Framework supplies the coherent PDS experience,
   reusable archetypes, governed operations, structured PDS Health AI results,
   mobile continuation, and lifecycle evidence. Nexus is the leading product
   candidate once its owner, first journey, source authority, and outcome are
   named.

The product-management question for every framework investment is:

> Does this reduce the time, risk, or cost of delivering governed business value
> for one of the four demand streams, with retained evidence that humans and
> agents can review?

If the answer is unclear, the work is not ready for implementation. Clarify the
product promise, evidence path, user segment, and owner first.

## 2026 Market And Industry Signals

Reviewed 2026-07-05. These signals reinforce the North Star rather than
changing it.

| Signal | Product implication for App Framework |
| --- | --- |
| DORA's 2025 research frames AI as an amplifier: the returns come from strengthening the underlying sociotechnical system, not from tool access alone. | App Framework must productize the system of work: contracts, generated ownership, small branches, CI evidence, review agents, handoff, and release gates. Agent velocity without platform discipline becomes review and ops debt. |
| Platform engineering is strongest when the platform is treated as a product with golden paths, sensible defaults, self-service, metadata, observability, and policy boundaries. | App Framework should be consumed as an internal developer platform, not as a collection of scripts. Product teams need paved paths and approved escape hatches. |
| AI-assisted development is shifting human value toward product judgment, architecture, review, and change management; agent-authored PR research shows small docs/CI/build changes merge more reliably than broad, complex diffs. | The framework should optimize for lane-sized implementation, structured product briefs, PR review output, tech-debt capture, and integration-branch management. The review system is a product feature. |
| OWASP's Agentic Top 10, NIST AI RMF, and NIST SSDF move AI/software risk into lifecycle controls: govern, map, measure, manage, secure development, and evidence. | Agentic UX, MCP, SaaS write-back, generated code, SRA packages, AIBOM, SBOM, provenance, and release evidence must stay first-class gates. No product should graduate from local demo to production by prose. |
| MCP is becoming a broad integration standard, but its own specification warns that tool execution and arbitrary data access require explicit consent, authorization, privacy, and tool-safety controls. | App Framework should support MCP as a governed ingress/interop surface, not a privileged bypass. MCP tools must resolve through the same operation, policy, audit, and consent model as HTTP, GraphQL, Kafka, sync, and UI actions. |
| OpenTelemetry and modern cloud-native practice emphasize vendor-neutral traces, metrics, logs, semantic conventions, and collector patterns. | Observability must be generated and packaged into every product archetype. If a generated app cannot show health, trace, audit, performance, and release evidence, it is not enterprise-ready. |

Primary sources used for this overlay:

- DORA Research and AI reports: <https://dora.dev/research/> and
  <https://dora.dev/ai/>
- Google Cloud platform engineering guidance:
  <https://cloud.google.com/blog/products/application-development/common-myths-about-platform-engineering>
- Productboard agentic product-management positioning:
  <https://www.productboard.com/>
- Pendo AI/product and agent analytics positioning:
  <https://www.pendo.io/>
- Amplitude AI analytics, MCP, product analytics, and agent analytics:
  <https://amplitude.com/>
- Aha! product discovery-to-delivery and AI product-management positioning:
  <https://www.aha.io/>
- Atlassian Rovo context, agents, connectors, and governance positioning:
  <https://www.atlassian.com/software/rovo>
- ServiceNow AI Agents and AI Control Tower positioning:
  <https://www.servicenow.com/products/ai-agents.html> and
  <https://www.servicenow.com/products/ai-control-tower.html>
- OWASP Top 10 for Agentic Applications 2026:
  <https://genai.owasp.org/resource/owasp-top-10-for-agentic-applications-for-2026/>
- NIST AI Risk Management Framework:
  <https://www.nist.gov/itl/ai-risk-management-framework>
- NIST Secure Software Development Framework:
  <https://csrc.nist.gov/pubs/sp/800/218/final>
- Model Context Protocol specification:
  <https://modelcontextprotocol.io/specification/2025-06-18>
- OpenTelemetry overview:
  <https://opentelemetry.io/docs/what-is-opentelemetry/>
- Evaluation-driven AI agent development at scale:
  <https://arxiv.org/abs/2606.08867>
- Agentic AI adoption barriers and capability-verification gap:
  <https://arxiv.org/abs/2605.14675>
- Agentic product risk perception and mitigation gaps:
  <https://arxiv.org/abs/2606.15485>

## Critical Trend Vetting

Current agentic product-development trends are useful only when they make the
product operating loop more evidence-rich and outcome-oriented. They are a
distraction when they create more automation theater, more code volume, or more
AI surface area without business proof.

| Trend | What is real | Fad trap | App Framework response |
| --- | --- | --- | --- |
| Agentic product-management systems | Productboard-style systems are converging on one source of truth that connects feedback, strategy, specs, delivery, and post-launch learning. | AI-generated PRDs that sound polished but are not backed by real stakeholder, workflow, usage, risk, or outcome evidence. | Make every roadmap candidate carry source signals, demand stream, value hypothesis, success metric, and post-launch learning requirement before implementation. |
| Product and agent analytics | Pendo and Amplitude are moving from "what did users do?" to "what did agents/users do, did it improve the outcome, and where are they stuck?" | Counting prompts, commits, generated lines, or agent sessions as success. Activity is not ROI. | Generate product telemetry, agent-action telemetry, adoption metrics, friction signals, and outcome hooks as first-class product evidence. |
| Context graphs and MCP-style access | Atlassian, Amplitude, Pendo, and MCP adoption show that product work is shifting toward context available wherever teams and agents work. | Letting MCP/chat become a broad data-access bypass that ignores product ownership, PHI, identity, consent, and audit. | Treat MCP and context access as governed product surfaces routed through named operations, policy, audit, and permissioned context packages. |
| Workflow-bound agents | ServiceNow and workflow vendors are putting agents inside business workflows, roles, permissions, and control towers instead of leaving them as free-floating chatbots. | "Autonomous workforce" language that hides who owns the result, who approves actions, and what happens when the agent is wrong. | Bind every agentic capability to workflow/task/request context, owner, authority level, evidence trail, escalation path, and measurable outcome. |
| AI-native product organizations | DORA-style research and AI-native company stories point to high leverage when adoption is hands-on, measured, and tied to delivery systems. | Tool rollout as transformation: licenses, demos, and enthusiasm without a measurable lift in valuable shipped work. | Manage agentic development as a product: adoption, first-pass verification, review throughput, cycle time, value delivered, defect escape, and tech debt are tracked together. |
| Evaluation-driven agent products | Large-scale agent deployments are moving toward offline evals, human review, A/B proof, and online outcome measurement as the way to move fast safely. | Shipping impressive demos whose quality cannot be predicted, reproduced, compared, or monitored after launch. | Require eval sets, scenario tests, review briefs, live evidence, and post-launch outcome checks before graduating intelligent product features. |
| Governed enterprise autonomy | OWASP, NIST, and 2026 agent-risk research all show that autonomy, tool use, memory, and enterprise context create new product risks. | Overcorrecting by banning useful autonomy, or undercorrecting by trusting broad agents with sensitive actions. | Use staged autonomy: recommend, draft, preview, execute with approval, execute within policy, and only then automate narrow repeatable actions with audit and rollback. |

The strongest conclusion: App Framework should not chase "agentic" as a
feature category. It should build a **business-value operating loop** where
signals become ranked opportunities, opportunities become thin proof slices,
proof slices become reusable platform capabilities, and shipped capabilities
produce adoption, risk, and outcome data that reprioritizes the Roadmap.

## Product Promise By Demand Stream

| Demand stream | Product promise | What App Framework must provide | What the product/app team must provide |
| --- | --- | --- | --- |
| Citizen-developed PoC | "Turn a useful prototype into a safe enterprise product without copying its fragility." | Intake profile, source-evidence handling, model proposal, generated backend/frontend, PDS UX, validation, release evidence, SRA package prep, review harness. | Business owner, redacted artifacts, data classification, workflow intent, accepted model decisions, release/SRA approvals. |
| Legacy modernization | "Replace behavior by capability, not by transliterating old code." | A PDS-owned, agent-portable modernization control plane: governed evidence, claim-specific authority, separate as-is and approved to-be models, objective treatment, App Framework target compilation where selected, portable workers, independent verification, migration, and retirement economics. | Named business/domain/data/security/operations/modernization/release owners; authorized evidence; adequacy and conflict decisions; treatment approval; parity, migration, rollback, cutover, support, and retirement decisions. |
| Greenfield product | "Ship a well-designed PDS-branded product from a product brief, with generated contracts and packaged framework dependencies." | Product bootstrap, model/schema contracts, web/mobile scaffolds, product CI, ProGet consumption, compatibility matrix, upgrade path, release gate. | Product brief, workflow spec, owners, success metrics, deployment target, operational constraints. |
| PDS enterprise experience / Nexus | "Give team members one coherent PDS-owned experience while ServiceNow and other systems retain authoritative workflow and records." | Real downstream package lifecycle; Request/Task/Approval archetype; ServiceNow evidence gate; permission-preserving projections; freshness/lineage; governed actions; typed PDS Health AI modules; durable attention; bounded administration; exact web/native-mobile continuation; and cross-platform telemetry. | Named product and journey owners; source authority; Pega evidence where relevant; actual Employee Slate/Moveworks comparator evidence; ServiceNow export; process logs; success criteria; data/security/ops constraints; and named action approval. |

The [Enterprise App Fabric Strategy](enterprise-app-fabric.md) sharpens the
Nexus and integration-heavy promise: App Framework layers only funded,
journey-scoped experiences where PDS-owned continuity, adopted object models,
governed actions, freshness/provenance, evidence, and intelligent surfaces
produce measurable value. It is not a mandate to wrap every SaaS screen or
invent a canonical enterprise model.

## Product Backlog Operating Loop

The PM role manages App Framework as a portfolio of value generators, not a
queue of architecture ideas. Every theme and feature moves through this loop:

1. **Sense demand.** Capture business-process pain, Nexus/Pega replacement
   needs, citizen-app prototypes, legacy modernization inventory, product-team
   friction, incident/review findings, SRA blockers, support requests, usage
   analytics, and market/industry signals.
2. **Frame opportunity.** Convert signals into a product opportunity with a
   demand stream, business owner, affected users, value hypothesis, risk class,
   success metric, and evidence source.
3. **Rank by ROI confidence.** Prioritize work that improves cycle time,
   adoption, workflow throughput, cost avoidance, risk reduction, support
   load, release confidence, or Pega/licensing/application rationalization.
4. **Slice to proof.** Fund the smallest slice that can prove or falsify the
   value hypothesis. Prefer a reusable platform primitive plus one concrete
   product proof over a broad generic buildout.
5. **Ship with evidence.** Use the risk-appropriate proof path: report-only,
   enforce, live evidence, release authority, SRA package, and post-launch
   outcome measurement.
6. **Learn and reprioritize.** Retire, expand, or demote roadmap candidates based
   on adoption, user friction, review findings, live evidence, defects,
   support load, and business outcomes.

No roadmap candidate is ready for implementation unless it has:

- demand stream and business owner
- target user or workflow
- value hypothesis and expected ROI mechanism
- success metric and measurement source
- proof slice and graduation gate
- dependency and risk posture
- anti-fad test: what would prove this is not worth continuing?

The App Framework Research Steward is the recurring external challenge loop for
this product-management system. It should periodically test PM guidance against
current market, industry, AI, UX, security, platform, CI/CD, SaaS, integration,
and change-management signals, then return evidence-backed recommendations to
accept, reject, investigate, add to backlog, add to tech debt, update docs, run
a strategic-freshness review, or schedule proof. Research recommendations are
not official guidance until the Strategist/Product Manager function and Product
Owner decide the disposition and update the appropriate source of truth.

## Program Roadmap Hierarchy And Terminology

Use one program vocabulary from executive direction through retained proof.
The dashboard and program reports must keep the canonical work breakdown
structure distinct from its linked planning and proof dimensions:

- **Value lineage:** North Star -> Strategy -> Strategic Goal -> Outcome.
- **Work breakdown:** Outcome -> Product Increment -> Deliverable -> Task.
- **Roadmap ordering:** Roadmap Horizon plus Product Increment order.
- **Delivery and proof:** Task -> Assignment -> Delivery -> Evidence ->
  Implemented -> Accepted.

Do not force these paths into one tree. An Outcome may advance several
Strategic Goals, while a Roadmap Horizon orders Outcomes and Product Increments
by dependency and decision sequence. Capability maturity is a cross-cutting
diagnostic lens, not another level in the work breakdown structure.

| Term | Standard meaning | Not this |
| --- | --- | --- |
| **Program** | The complete App Framework investment, including direction, roadmap, products, platform capability, delivery, evidence, and learning. | A branch, wave, or single product. |
| **North Star** | The durable purpose and desired future for the Program. It changes only when business or market evidence changes the destination. | A current plan, release target, or status report. |
| **Strategy** | The durable choices, positioning, boundaries, and economic rules used to pursue the North Star. | A backlog or ordered delivery list. |
| **Strategic Goal** | A durable, measurable result that advances the North Star. The seven goals in the North Star are the canonical set until a strategic-freshness review changes them. | A temporary branch objective or historical wave. |
| **Roadmap** | The dependency-aware ordering of Outcomes and Product Increments across Roadmap Horizons. | A flat feature list or a history of merged work. |
| **Roadmap Horizon** | An ordered decision and dependency band such as current foundation, first real product, production vertical, differentiated product, comparative proof, reuse, or enterprise scale. It is not a date promise unless an accountable owner funds and dates it. | A delivery wave or lifecycle status. |
| **Outcome** | A major, measurable state change with business value, an owner, entry conditions, exit evidence, and a stop rule. This is the common node linking strategy to roadmap execution. | A theme, capability name, activity count, or implementation approach. |
| **Product Increment** | One meaningful demonstration or decision that advances an Outcome. One or more Product Increments produce an Outcome. | A date-only checkpoint, branch, or release label. |
| **Deliverable** | The smallest bounded result that can prove or falsify part of a Product Increment. It has one owner, acceptance evidence, and explicit non-claims. | An open-ended lane or broad capability family. |
| **Task** | A concrete unit of execution that produces part of one Deliverable. | A second roadmap level or an inferred assignment. |
| **Outcome Goal** | Retained schema name and compatibility alias for an Outcome. | A separate planning level in addition to Outcome. |
| **Milestone** | A retained record presented as a Product Increment when it represents a coherent demonstration or decision; otherwise it is a roadmap checkpoint. | An automatic level between Outcome and Product Increment. |
| **Slice** | Retained internal alias for a Deliverable. | A separate planning level in addition to Deliverable. |
| **Delivery** | The implementation vehicle for a Task or Deliverable: branch, pull request, commit, pipeline, deployment, or equivalent execution record. | Business value or acceptance by itself. |
| **Evidence** | Source-bound proof supporting a delivery state, Product Increment, Outcome, maturity claim, or decision. | Generated prose, an unverified artifact, or an activity count. |
| **Capability** | A reusable product or platform ability whose current and target maturity can be assessed across multiple Outcome Goals. | A roadmap level or automatic claim of adoption. |
| **Product Theme** | A prioritization lens that groups related demand and capabilities. The ranked themes below remain searchable tags and investment lenses. | An Outcome Goal or execution order. |
| **Portfolio** | A view across all Outcome Goals, products, capabilities, investment, risk, and evidence. | A hierarchy level between Strategy and Roadmap. |
| **Gate** | A required condition that must be satisfied before a claim or transition. | A workstream or value outcome. |
| **Wave** | A historical or temporary coordination batch retained for delivery lineage. | The current strategic roadmap structure. |
| **Lane** | A bounded WIP assignment with one owner and write surface. | A durable roadmap level. |

Keep planning posture separate from delivery state. Use **Active**, **Next**,
**Awaiting input**, **Held**, and **Later** to describe when an Outcome Goal may
consume capacity. Use **Planned**, **In progress**, **In review**, **Merged**,
**Accepted**, **Deferred**, and **Stopped** for Milestone, Slice, and Delivery
state. Only accepted evidence closes progress; local completion, review, merge,
and CI remain visible intermediate states.

Product design has a separate ontology and must not reuse roadmap terms:

> Product Outcome -> User or Business Journey -> Role, Moment, and Channel
> Experience -> Workspace -> Typed Work Module -> Attention Item.

A Journey is a bounded end-to-end progression across states, systems, time,
roles, and handoffs. It is not a Roadmap Horizon, Milestone, Slice, portal,
screen flow, workflow table, or chat. An Experience is one persona's
participation in a Journey moment through a channel and context. Source systems
remain authoritative for their records and workflows.

## Prioritized Product Themes

This is the PM-ranked theme overlay. It guides roadmap sequencing but does not
replace `docs/release/roadmap.md`, which owns the executable backlog,
current-state scores, blockers, and execution order.

| Rank | ID | Product Theme | Primary demand stream | Business value hypothesis | Proof metric | Anti-fad gate |
| ---: | --- | --- | --- | --- | --- | --- |
| 1 | **AF-PT01** | **Enterprise app fabric and object-model adoption** | Nexus, integration-heavy products | Journey-scoped fabric value becomes scalable when vendor object/workflow models are adopted verbatim behind generated, version-pinned, drift-checked contracts instead of translated into a fragile canonical model. | One ServiceNow authenticated dictionary/workflow export produces a model proposal, consumer-contract drift check, and actionability-tier evidence for a Team Member Journey step. | Block broad supergraph, MDM, or all-SaaS wrapping work until a named owner funds a journey with tiered actionability and success metrics. |
| 2 | **AF-PT02** | **Productization factory for citizen and legacy apps** | Citizen apps, legacy modernization | Repeatable intake plus a PDS-owned modernization control plane uses a first-class, provenance-backed Modernization IR and a separately approved to-be treatment to turn governed evidence into verified products while interchangeable agents and specialist engines do bounded work. | One representative .NET/IIS/Angular/SQL Server application produces a reviewable As-is Modernization IR, bounded human adequacy approval, a distinct versioned Approved To-be Treatment Model, one production-capable App Framework vertical compiled without silent semantic loss, portable Claude Code/Codex consumption, independent differential and migration/coexistence proof, and retirement economics; a comparable second use requires at least 50% less normalized discovery-to-approved-model engineering effort without lower quality. | Stop if raw evidence, an as-is model, or an agent plan projects directly into target configuration; the process claims complete understanding, defaults every capability to App Framework, lets generated output validate itself, lacks a named consumer, or preserves legacy cost without an owned retirement/coexistence decision. |
| 3 | **AF-PT03** | **Packaged platform consumption** | Greenfield, citizen apps, legacy modernization | Product teams create and upgrade apps faster when they consume approved packages instead of cloning or depending on an adjacent framework checkout. | New product app can bootstrap, build, validate, and upgrade from approved package feeds with retained compatibility evidence. | Defer new capabilities that cannot be consumed by downstream products without bespoke local setup. |
| 4 | **AF-PT04** | **Agentic review and tech-debt loop** | Framework platform consumption | Agent throughput creates value only when review findings, drift, conditions, and debt are captured and retired faster than they accumulate. | PR Review Agent outputs status/counts; `GO WITH CONDITIONS` items produce debt entries or explicit paydown triggers. | Reduce agent parallelism if review quality, conflict rate, or repeated findings degrade. |
| 5 | **AF-PT05** | **Structured application intelligence** | Nexus, greenfield products | Governed state, PDS Health AI evidence, deterministic or event signals, recommendations, action previews, and outcomes become durable typed work instead of disappearing into chat. | One named task completes an authenticated observe-understand-recommend-checkpoint-preview-evaluate loop on web and native mobile with the same context, evidence, policy, correlation, fallback, cost, and outcome semantics. | Do not build another assistant/search destination, iframe target, model platform, universal agent runtime, or protocol-led contract family before one real loop improves a measured work outcome. |
| 6 | **AF-PT06** | **PDS-owned enterprise experience** | Employee experience, Nexus, deep enterprise products | One coherent PDS web/native-mobile product can preserve cross-system context, attention, intelligence, action, exact resumption, and signature quality while authoritative SaaS platforms retain workflow and records. | One real downstream package consumer completes a named work outcome with live read, permission-preserving projection, governed action, managed operations, mobile continuation, and structured intelligence; comparative task, trust, quality, support, and economic thresholds pass. | Do not duplicate ServiceNow workflow, general search, or vendor feature breadth; narrow the launched scope if managed trust, product outcome, supportability, or lifecycle economics fail. |
| 7 | **AF-PT07** | **Product signal intake and value scoring** | All | The framework will fund higher-ROI work if every incoming app/theme starts from business signals, owner, outcome, risk, and evidence instead of technical enthusiasm. | 100% of new roadmap/product items include demand stream, owner, value hypothesis, metric, and evidence source. | Reject items whose only rationale is "AI", "agentic", "modern", or "requested by a thread" without business signal. |
| 8 | **AF-PT08** | **Product and agent outcome telemetry** | All | App Framework proves ROI when generated products show adoption, friction, workflow cycle time, agent usage, intervention rate, and business outcomes. | Product handoff/release evidence includes product analytics and agent-action metrics for shipped intelligent surfaces. | Do not count generated LOC, prompt count, or agent sessions as success unless tied to adoption or outcome improvement. |
| 9 | **AF-PT09** | **PDS-owned enterprise-experience product proof** | Employee experience, Nexus | The first lovable PDS employee product pulls the reusable fabric through a complete, independently packaged vertical rather than accumulating unconsumed contracts. | The same named outcome is measured against current PDS and Employee Slate/Moveworks baselines for completion, accessibility, signature quality, mobile continuity, delivery, support, resilience, and lifecycle economics. | Stop or narrow the product scope if P0 trust/runtime/operation gates do not reach managed evidence or the PDS product cannot establish a supportable outcome and economic advantage. |
| 10 | **AF-PT10** | **SRA and release evidence package maturity** | All regulated products | Faster security/release approval is business throughput; teams ship sooner when evidence packages are generated early and kept current. | Product and framework SRA packages identify missing human inputs and produce review-ready evidence bundles before release crunch. | Do not treat an SRA package as value if it is only prose and cannot reduce review surprises or missing evidence. |
| 11 | **AF-PT11** | **Governed integration and action plane** | Nexus, integration-heavy products | SaaS/API/MCP/Kafka actions become valuable when they safely complete real workflow steps with identity, audit, idempotency, and rollback posture. | At least one named Nexus-relevant read path and one preview-only governed write path are proven with retained evidence before live writes. | Block generic connector expansion until a product workflow needs it and can define freshness, lineage, principal, and action safety. |

This ranking expresses strategic pull, not technical dependency order. A live
provider read, preview-only operation, security control, or release gate may be
scheduled earlier when it is required to prove a higher-ranked outcome.

### Agent-Portable Legacy Modernization Control Plane

This is the target capability definition for the legacy half of the
Productization Factory theme. It is **shaped but held**: it does not change the
current Outcome Goal Register or release an implementation lane. Its durable
goal is:

> Establish a typed, provenance-backed Modernization Intermediate
> Representation as a first-class App Framework capability and the durable
> bridge between governed legacy evidence and approved target implementation.
> The harness must reconstruct an evidence-bounded **As-is Modernization IR**
> with claim-specific authority, conflicts, unknowns, freshness, and
> invalidation; support a separately versioned and human-approved,
> platform-neutral **Approved To-be Treatment Model**; and compile only approved
> target semantics into complete App Framework contracts with visible loss,
> verification, migration, and retirement obligations. Claude Code, Codex,
> deterministic recipes, and specialist vendors remain interchangeable workers;
> independent proof remains external to the models.

App Framework should own the PDS-specific system of record: evidence and
claim-specific authority, the As-is Modernization IR, adequacy and treatment
decisions, the Approved To-be Treatment Model, target compiler, verification
obligations, retirement economics, and portfolio learning. It should use or buy
qualified analyzers,
coding agents, transformation engines, and retrieval/context services. Claude
Code, Codex, Cursor, Augment, Blitzy, AWS Transform, and similar tools may be
valuable workers or surfaces; none of their session memory, index, plan format,
generated tests, or proprietary state becomes PDS authority.

The process keeps two versioned stages distinct. The As-is Modernization IR
makes scoped, provenance-backed claims with conflicts, unknowns,
confidence/evidence grade, freshness, and authority. A separately approved,
platform-neutral Approved To-be Treatment Model records retire, retain/contain,
rehost, upgrade, refactor,
replatform, SaaS/enterprise replacement, extract, strangle, App Framework
rebuild, or split decisions. Only capabilities explicitly selected for App
Framework may compile into `.appfw/model`, operations, policy, providers,
events/projections, migrations, tests, product intent, extensions, and release
obligations. Raw evidence, the As-is Modernization IR, and agent-generated plans
must never project directly into target configuration; unsupported or lossy
compiler mappings require explicit diagnostics, extensions, or approved waivers.

Workers consume a normalized task contract and return a normalized result with
evidence, versions, permissions, allowed/forbidden scope, checks, commands,
changes, assumptions, unknowns, deviations, sensitive-data events,
interventions, cost, stop/escalation, and handoff. Independent build, behavior,
data/SQL, security, experience, performance, resilience, migration, rollback,
and retirement proof remains mandatory; generated plans, code, and tests are
not their own oracle.

The first proof is one controlled representative .NET/IIS/Angular/SQL Server
application: establish a measured manual baseline, approve an adequate As-is
Modernization IR and a distinct Approved To-be Treatment Model, compile and
implement one production-capable vertical
where App Framework is selected, prove differential behavior and safe migration
or explicitly costed coexistence, and establish retirement criteria. A second
suitable application must lower normalized discovery-to-approved-model
engineering effort by at least 50 percent while quality stays level or improves.
The economic unit is fully loaded cost per accepted, production-capable business
capability with verified legacy retirement, not generated lines or agent usage.

XO may release this goal only when the human prioritizes the pilot, Product
Owner confirms the product outcome and acceptance, Architect confirms the
model/compiler/worker/proof boundaries, the named domain/data/security/
operations/modernization owners confirm their authority and evidence, and XO
confirms current capacity. The first milestone is a chartered pilot and
benchmark, not connectors, a universal ontology, broad agent orchestration, or
a horizontal platform build. See
[Legacy Application Modernization](../lifecycle/legacy-modernization.md) for the
canonical lifecycle, model, worker, proof, and stop contracts. The July 14, 2026
*Agentic Legacy Modernization Harness* decision research in the PDS app-fabric
research workspace substantiates this disposition but is not itself
an execution contract.

### Cross-Channel Closed-Loop Application Intelligence

This strategic goal strengthens the existing Structured Application
Intelligence theme; it is not a duplicate theme or a new AI platform. App
Framework should be a **channel-neutral intelligence integration and action
fabric**. It should not become another assistant, enterprise-search product,
model platform, or universal agent runtime. The useful unit is a governed
application loop:

> Observe governed state -> understand material context -> recommend or
> prepare -> apply a human or policy checkpoint -> act through a governed
> operation -> record and evaluate the outcome.

Chat may invoke, clarify, or explore this loop; it does not own the work. A
useful answer becomes typed application state such as a ranked work item,
evidence-backed brief, editable proposed value, action preview, checkpoint, or
auditable timeline event. A transcript, notification, or iframe is not the
authoritative copy of that state.

Fund the goal only for a named task that can plausibly reduce status chasing,
reading, rekeying, routing, handoff loss, cycle time, avoidable errors, or
support burden; detect a material exception early enough to change an outcome;
or provide deep web, external-persona, capture, offline, or native-mobile value
that an entitled vendor surface cannot meet economically. Reuse of context,
permission, evidence, action, rendering, channel continuity, evaluation, and
support must lower the second product's integration cost. Model use, chat
engagement, agent count, token volume, and generated code are not value
measures.

PDS Health AI should remain the PDS retrieval and information-pack service. It
returns structured claims, evidence, freshness, limitations, and resolvable
entity references through a backend-to-backend contract. App Framework
re-resolves application records under the current principal and policy, renders
trusted channel-native state, invokes the existing governed operation path,
and records evaluation and outcomes. PDS Health AI does not receive transaction
authority. ServiceNow or another owning platform remains the workflow authority
where the process belongs there.

Pull requires a Product Owner for the user, task, baseline, outcome, and stop
threshold; source and PDS Health AI information-pack owners for authority,
classification, freshness, quality, and support; an Architect for reuse and
boundary decisions; security/data owners for permission and handling; and
product-design, mobile, and SRE/observability owners for channel and managed-
operation proof. XO coordinates sequencing and WIP but does not invent the use
case or approve its risks.

This goal is **shaped but held and attached to the existing executable
vertical**. It does not create a separate AI producer lane or widen the active
security/runtime correction. When a named product use and the prerequisite
runtime, provider, projection, action, and operational proofs exist, pull this
sequence:

1. Prove one complete non-chat loop with one live PDS Health AI information
   pack, one deterministic or event signal, one structured recommendation or
   prepared value, one governed preview action, useful web and native-mobile
   renderings, deterministic fallback, and a measured task outcome.
2. Make the existing principal, tenant, policy, entity, workflow, freshness,
   channel, device, and correlation semantics one authoritative context spine.
   Do not create a parallel AI identity or authorization model.
3. Compose `answer_envelope@1`, `permissioned_archetype@1`, existing
   operations, the view registry, audit, and OpenTelemetry first. Add only the
   minimum typed result or rendering semantics the live vertical proves
   missing; do not predeclare several broad contract families.
4. Make one permission-preserving Kafka/projection path produce a low-noise,
   durable work item with entitlement, freshness, replay, lag, suppression,
   reconciliation, fallback, and revocation evidence. Do not launch a generic
   Kafka or ambient-intelligence program.
5. Add durable run/checkpoint semantics only for a real multistep task that
   needs pause, edit, approve, reject, cancel, resume, idempotency, recovery,
   and attribution. Prefer the owning workflow engine when it already governs
   the process.
6. Prove exact continuation across web and native mobile while keeping separate
   component trees: web owns comparison, bulk work, and deep inspection;
   mobile owns timely signals, secure deep links, capture, quick bounded work,
   interruption/offline behavior, biometrics where required, and progress.
7. Extend PDS Observability across retrieval, model or rule, tool, render,
   operation, and downstream outcome. Measure task quality, unsupported claims,
   intervention, latency, security, fallback, and cost per successful outcome;
   test a kill switch.
8. Add a capability registry and resolver only after the first loop proves the
   need. Product code requests a task capability, and routing prefers fact or
   deterministic logic, then event/rule, retrieval, specialized or on-device
   model, larger model, and finally a multistep agent when required.
9. Defer memory and personalization until they are visible, scoped, consented,
   correctable, expiring, resettable, authorization-neutral, and proven to
   improve the task.
10. Let a second real product determine which context, result, renderer,
    evaluation, checkpoint, and generation patterns are truly reusable.

The first proof is complete only when it retains:

- a named user, task, owner, baseline, target outcome, observation window, and
  narrow/stop threshold;
- live authenticated source and PDS Health AI integration with a versioned,
  owned information pack;
- adversarial tenant, row, field, action, delegated-identity, stale-data, and
  source-entitlement tests with no authorization escape;
- resolvable entity/evidence references, explicit freshness and limitations,
  and current-policy re-resolution before display or action;
- every material action through the governed preview path, with risk,
  reversibility, idempotency, checkpoint, audit, and recovery state;
- deterministic task completion or fallback when intelligence is unavailable,
  slow, unsupported, or wrong;
- useful, accessible web and genuine native-mobile completion with exact task,
  draft, evidence, checkpoint, and pending-action continuation;
- retrieval/rule/model, tool, render, operation, fallback, correction, and
  downstream-outcome trace plus a task-specific quality/security regression
  set and tested kill switch;
- latency, support burden, human correction/review, and total cost per
  successful outcome; and
- a material work-outcome improvement or an explicit decision to narrow/stop,
  compared with the current entitled ServiceNow/Moveworks surface where it is
  a realistic alternative.

Autonomy is a property of a named operation and risk context: observe/explain,
recommend/rank, prepare editable work, execute after explicit confirmation,
and only then execute within a narrow preapproved policy after sustained
evidence. Broad autonomous coordination is exceptional and requires mature
operations, bounded authority, recovery, and measured outcomes. Fluent
rationale or an uncalibrated confidence value is not promotion evidence.

The claim boundary remains strict: App Framework has strong application nouns
and partial contracts, but no live PDS Health AI loop, complete production
Kafka lane, delegated source action, production native-mobile proof, or
end-to-end intelligence outcome/evaluation trace. A2A, AG-UI, A2UI, MCP Apps,
and similar protocols remain adapters after the internal contract is stable.
The detailed research basis is the July 14, 2026 *Intelligent Applications
Beyond Chat* direction, read with the *PDS Health AI Integration Analysis* and
the *Application Fabric Research Dossier* retained in the PDS app-fabric
research workspace.

### Experience strategy: calm precision, expressive intelligence, unmistakable craft

App Framework's experience foundation is stronger than its current visual
coherence. The tracked PDS catalog defines 95 shared components across ten
families, including governed action, evidence, freshness, operation, process,
conversation, and ambient-intelligence primitives. Web and Expo/React Native
paths exist, and automated accessibility and layout checks cover useful
scenarios. These are real assets, but component breadth and automated checks do
not prove an excellent product experience.

The strategic experience thesis is:

> **Calm precision. Expressive intelligence. Unmistakable craft.** Keep the
> operational base quiet, exact, responsive, accessible, and native to its
> channel. Earn visual expression only for the next action, a material state or
> risk, a cross-system insight, live progress, or successful completion. Make
> the whole journey feel unusually coherent, distinctive, and beautifully made
> without sacrificing repeated-use efficiency.

Treat executive demand for "sizzle" as the outcome **signature experience
quality**, not permission for decorative effects. Use an `80/20` heuristic:
familiar, accessible, platform-native patterns carry routine work, while a few
domain-specific visual objects and signature moments receive disproportionate
design, visualization, motion, engineering, and research investment. Every
flagship journey requires a signature experience brief covering desired
emotion, a meaningful visual object, two or three moments, web/native-mobile
expression, motion or haptics, evidence and permission, recovery, performance,
accessibility, fallback, and current-PDS/vendor comparison baselines.

Use an outcome-led product model. The **outcome** is the funding and
measurement unit. A **journey** is the durable cross-system path from trigger
to outcome across roles, systems, waits, branches, channels, and time. An
**experience** is one role's participation at a particular journey moment and
channel. A **workspace** is a persistent environment supporting related
journeys; a **work module** is a typed bounded surface exposing journey state
or capability; and an **attention item** is a durable governed signal to
notice, resume, decide, or act. A journey is not a screen flow, workflow
synonym, broad canonical model, or new orchestration engine. Source systems
remain authoritative. App Framework owns portable journey/experience
projection, cross-system context, exact resumption, governed-action
integration, bounded workspace/module contracts, and channel-neutral attention
semantics. Intelligence improves these layers; it is not a compulsory chat
destination.

The compositions share an **Orient -> Understand -> Act -> Resolve** sequence:
reveal what matters and why; turn intelligence into editable visual structure
with evidence; preview governed cross-system consequences; and turn live
progress into a concise receipt without context loss. Each flagship web
reference proves two or three production-shaped signature moments, and each
native flagship at least one corresponding mobile moment with platform
behavior, interruption safety, useful gesture, and restrained haptics.

The current gap is composition and hierarchy. Gradients, glow, blur, borders,
shadows, and card containment currently do too much structural work. Do not
rewrite React, Vite, Expo, or React Native, and do not launch another broad
component-expansion program. Tailwind CSS 4 may remain a product composition
tool where used; it is not the PDS design system or its source of truth.

The bounded CRM Activities composition is the current product-neutral proving
ground for this direction. It may improve composition and validate the design
system, but it does not establish Nexus requirements, a vendor comparison, or
experience superiority. When the broader goal is explicitly pulled, sequence
it as follows:

1. Build three production-shaped reference compositions and signature briefs
   with approved PDS content: an admin entity/operation workbench; a
   Nexus/My Work queue/detail with structured PDS Health AI; and a native-mobile
   task/approval. Compare
   task performance with the existing App Framework surface and the current
   entitled Horizon/Employee Slate baseline. Nexus-specific work waits for
   product intake and may not be invented from framework strategy.
2. Prove signature experience quality through domain-specific visual objects,
   spatial continuity, structured intelligence, governed action preview, and
   outcome resolution. Test five- and 30-second impressions, blinded
   preference, semantic differentials, recall, immediate response, motion in
   context, reduced motion, and INP at or below 200 ms p75, without task,
   accessibility, trust, reliability, or recovery regression.
3. Select one visual grammar from measured task evidence, then establish a W3C
   DTCG-compatible token source that generates CSS, TypeScript, React Native,
   and design-tool mappings for semantic, component, motion, data-visualization,
   theme, high-contrast, and density layers.
4. Define six canonical floorplans: admin workbench, queue/detail, entity
   workspace, guided process, operational overview, and intelligent work
   surface. Refactor existing components only as real compositions consume
   them.
5. Run a focused React Aria versus Radix bakeoff for representative complex
   controls and select one accessible behavior substrate. Use TanStack Table
   and Virtual behind the App Framework-owned data-grid API when large-data
   evidence justifies them; do not expose a third-party visual system as the
   product contract.
6. Productize a bounded intelligent-composition contract: typed result,
   evidence, freshness, permission, and action envelopes rendered through an
   allowlisted PDS component registry with stable fallback behavior. PDS Health
   AI appears inline first and chat is secondary. A2UI or MCP Apps are adapters
   only after the internal contract is stable; arbitrary runtime UI code is
   prohibited.
7. Make admin UI the dense, keyboard-efficient reference product and Nexus the
   cross-system intelligent-work proof when product inputs exist. Preserve
   genuine Expo-native mobile behavior rather than shrinking desktop layouts.
8. Make the design system executable for agents through machine-readable
   recipes, constraints, approved/prohibited uses, production imports,
   design-code mappings, semantic motion roles, interaction tests, and standard
   visual evidence.
9. Add measured quality gates: WCAG 2.2 automation plus manual assistive-tech,
   keyboard, zoom/reflow, high-contrast, and reduced-motion testing; complete
   loading/empty/error/partial/stale/offline/unauthorized states; responsive and
   visual regression; five- and 30-second first impression; blinded preference;
   Core Web Vitals; bundle and large-data budgets; and task and intelligence
   telemetry.
10. Assign an experienced product-design owner. Agentic generation may create
   variants and implementation quickly; it does not own visual judgment,
   interaction coherence, accessibility expertise, or user research.

The full Nexus journey, attention, workspace, and module objective remains
held until Product Owner intake supplies a named outcome and persona, source
owners and authority, journey evidence, baseline, permissions and data posture,
measurement method, and accountable product/design/operations owners. When
released, prove one real journey in this order:

1. version a journey graph or service blueprint with source ownership,
   role/channel experience briefs, exact handoffs and exceptions, correlation,
   Orient-Understand-Act-Resolve moments, and outcome evidence;
2. prove a durable attention item and user preference model through an in-app
   center and one web or native push adapter, including grouping,
   deduplication, expiry, acknowledgement/snooze, policy override,
   accessibility, exact deep-link resumption, and PDS Observability trace;
3. prove a bounded personal work canvas with role/admin templates, policy
   locks, versioned layouts, allowlisted modules, backend eligibility,
   add/remove/reorder/resize/configure/undo/reset, accessible responsive order,
   schema migration/fallback, and appropriate cross-device persistence;
4. prove journey-readiness, PDS Health AI evidence-brief, and intelligent-queue
   or next-best-action modules that declare inform/recommend/prepare/act and
   preserve permission, evidence, freshness, consequence, checkpoint,
   recourse, deterministic fallback, latency, quality, inference cost, and
   edit/accept/reject/undo/outcome telemetry; and
5. give administrators trace and recovery views for journey correlation,
   attention policy/delivery, module eligibility, layout versions/reset, AI
   evaluation/cost, and failures.

Basic vendor workspace configuration is table stakes. The target is a coherent
PDS-owned enterprise experience; Employee Slate/Moveworks remains the strongest
comparator and fallback. The PDS product earns broad adoption through managed
trust, measured cross-vendor continuity, structured intelligence, exact
web/native continuation, outcome lift, second-use economics, supportability,
and signature quality. Do not turn this model into a generic portal,
widget marketplace, journey studio, workflow engine, or personalization layer
that can alter permission, authority, source truth, evidence, or required
controls.

Experience superiority is a measured product claim with two independent proof
bars. **Operational superiority** is initially hypothesized as at least 15
percent faster critical-workflow completion or a ten-point task-success lift.
**Signature experience quality** is provisionally at least 65 percent blinded
preference and a one-point lift on a seven-point crafted/fresh/intelligent/
distinctive/beautiful composite. Baseline research may calibrate the numbers
but must not remove either gate, and neither can compensate for weaker trust,
accessibility, reliability, recovery, or supportability. Include semantic
differentials for dated/fresh, generic/distinctive, static/alive,
mechanical/intelligent, cluttered/composed, and ordinary/beautiful plus
signature-object/interaction recall. Reject polished genericity, expression
everywhere, copied vendor aesthetics, demo-only choreography, canned
intelligence, and surprise that slows repeated work. The detailed target lives in
[PDS Health Enterprise Design System](../frontend/pds-health-design-system.md).

### Current investment authority

The [Roadmap Outcome Goal Register](../release/roadmap.md#outcome-goal-register)
is the sole current sequence. Product management maintains value hypotheses,
activation criteria, measures, and stop rules; it does not duplicate the
roadmap's producer order in this strategy.

The durable investment rule remains: after the active product-neutral readiness
goal closes and a real product is activated, at least 70% of implementation
capacity should terminate in that vertical or consumer, and no more than 20%
should go to bounded delivery-system simplification except mandatory security
or evidence-truth defects. Contracts, dashboard work, provider breadth, Kafka,
generic components, and evidence machinery count only when a named product
consumes them in the same horizon.

The active CRM composition is the current reference-product experience proof.
It does not satisfy the independently packaged downstream consumer, live
ServiceNow read, permission-preserving projection, governed action, or managed
operational profile and earns no credit for those outcomes.

Hold steady on generator breadth, generated API-test machinery, database and
provider breadth, generic PDS component expansion, synthetic reference
products, and new release/governance gates that do not close a named vertical
gap. Defer competing search/assistant, a generic workflow engine, broad A2A or
MCP productization, a universal employee/mobile shell, a commercial SaaS-grade
tenant control plane, another MDM/canonical cross-vendor model, full Kafka
certification before the archetype needs it, and dashboard expansion.

### Program dashboard source and claim boundary

The dashboard is the primary human communication projection for program
direction, roadmap, maturity, execution, and evidence. Improve it in bounded
clarity increments when the canonical program model changes or users cannot
navigate the decisions. It must not become a parallel Program State system,
planning authority, or acceptance authority, and dashboard work does not count
as vertical product progress.

The Program Lineage Dashboard is a communication projection, not a parallel
planning system or source of truth. Its North Star, strategy, roadmap,
capability scorecards, goals, slices, delivery state, and evidence links must be
regenerated from four authoritative source classes:

1. canonical strategy, product, architecture, and roadmap documents on the
   accepted destination branch;
2. a small tracked program contract containing durable IDs, relationships,
   owners, decisions, scorecard states, proof obligations, and explicit
   non-claims;
3. material Git and Bitbucket transitions for branches, commits, pull requests,
   and pipelines, collected through the repository-owned authentication path;
4. immutable run/evidence manifests tied to the relevant source and destination
   commits.

External research informs Strategist and Product Owner decisions, but it does
not update dashboard claims directly. Accepted conclusions must first be
dispositioned into the canonical documents or tracked program contract.
Ignored `target/appfw` board, queue, history, cache, and dashboard fixture files
are disposable generated views; they must not become the program database.

Refresh on accepted program-contract or canonical-document changes and on
material branch, pull-request, pipeline, or evidence transitions. Also run a
six-hour reconciliation while delivery is active and a daily reconciliation
when it is idle. Suppress unchanged polling records. Every generated dataset
and dashboard view must show the source commit, generation time, source
references, and freshness status. Missing, invalid, or stale input must render
as **unknown** or **stale**, never silently preserve an earlier claim as current.
Promotion beyond the current local projection requires deterministic
generation, schema validation, source/claim parity checks, access control for
non-public program evidence, and fixture-free operation against this contract.

### Economic guardrail

The July 2026 three-year model is decision sensitivity, not a buy/build verdict
or the cost of the expanded PDS-owned target.
At 17,000 potential users, every $1 per user per month changes three-year cost
by $612,000 at full coverage. At the deliberately illustrative $10 placeholder,
the modeled gross totals are $14.989 million vendor-first, $19.244 million
hybrid, and $14.564 million PDS-owned. The PDS-owned row is not
feature-equivalent to Employee Slate/Moveworks, and the approximate $9.31
crossover is a negotiation signal only.

Separate common portfolio cost from strategy-specific cost. Pega redesign and
transition contributes $4.549 million to every option, so it cannot justify the
fabric. Strategy-specific cost is $10.440 million vendor-first, $14.695 million
hybrid, and $10.015 million PDS-owned. The hybrid's $4.255 million premium is
the net of a $0.195 million modeled reuse credit, $3.700 million of App
Framework team cost, and $0.750 million of cloud/tooling; vendor access and
vendor-shell operations are held equal.

The unadjusted hurdle is $1.418 million of measured value per year, $6.95 per
eligible user per month, or $0.709 million per planned product journey before
adding execution, adoption, opportunity-cost, and benefit-uncertainty margin.
Treat these as comparison units, not a savings forecast. Actual quote and
coverage, citizen demand, PDS Health AI run cost, benefits, support capacity,
and durable product/platform/SRE ownership remain decision inputs. Fund each
increment only when named reuse, avoided services/support, product throughput,
application retirement, portability, or a structurally better experience can
credibly clear its allocated hurdle.

The expanded ownership direction adds work that the earlier PDS-owned row did
not price. Use 30-45 blended person-months as a directional planning range for
a bounded first lovable product, 75-120 for broad front-door maturity, and 6-10
durable FTE equivalents for ongoing operation. These are not commitments and
must not be used as production-readiness claims. Product scope, managed trust,
support ownership, and observed reuse must progressively replace the ranges
with evidence.

### Comparative capability claim discipline

Roadmap, dashboard, review, and executive reporting must keep three columns
separate: the practical Employee Slate/Moveworks capability, App Framework's
current evidenced capability, and the funded App Framework goal state. Use the
following rating language consistently:

- **Leading:** the strongest mature current option for the category.
- **Strong:** material capability backed by credible current evidence.
- **Partial:** useful foundation, but narrower or incomplete for the need.
- **Gated:** designed or scaffolded, but unavailable for production claims.
- **Consume:** integrate the capability rather than rebuild it.
- **Must prove:** target advantage that remains conditional on explicit proof.

Never credit a target-state capability as current value. The strategic target
is PDS experience ownership through composition, not feature parity: integrate
vendor strengths where they have better economics and require App Framework to
lead in complete PDS application productization, executable archetypes,
channel-neutral capabilities, coherent web/native-mobile experience, governed
application contracts/actions/events, code-native proof, and cross-platform PDS
Observability.

Use a 0-5 maturity scale for current capability: 0 absent, 1 documented intent,
2 scaffold/contract only, 3 implemented with local or reference proof, 4
provider-backed/release-gated/repeatable integration proof, and 5 managed
production proof at enterprise scale. Use relative effort bands E0 none, E1 up
to two blended person-months, E2 two to four, E3 four to eight, E4 eight to
fifteen, and E5 more than fifteen/category-scale. Effort bands overlap and must
not be mechanically summed.

| Capability | Current maturity | Relative effort | Evidence-calibrated position |
| --- | ---: | ---: | --- |
| Model-driven application production | **4** | **E1** | Current lead: entity/relationship models drive full-stack code, policy, generated API tests, and evidence. |
| Database runtime and lifecycle | **4** | **E1** | Current lead: provider-neutral semantics, forward-only migrations, deployment portability, and native MongoDB are real assets. |
| Provider contract/graduation framework | **4** | **E1** | Current report: 13 providers, 105 graduated areas, zero promotion violations; this does not imply live execution for every provider. MongoDB has 15 live-certified semantic areas. |
| Code delivery, testing, and provenance | **4** | **E1-E2** | Git, deterministic generation, CI/release gates, package/evidence hashes, security checks, and SBOM are current strengths; managed provenance/signing remains. |
| Tenant-isolated data execution | **4** | **E2** | Authenticated tenant context composes with policy/provider filters, fails closed when absent, is database-certified for same-tenant access and cross-tenant denial, and propagates through audit and ingress/planning contracts. |
| Managed multi-tenant control plane | **2** | **E4** | Provisioning/offboarding, config/secret partitioning, quotas, tenant-aware migrations, residency, per-tenant recovery, cost allocation, and support are unproven. |
| PDS application authorization and row policy | **4** | **E1-E2** | OIDC/JWT roles, scopes, and tenant feed deny-by-default Rego, mandatory provider row filters, redaction, and audit. Focused tenant and MCP role/scope tests pass; dynamic field obligations and managed evidence remain. |
| Enterprise-search permissions | **1-2** | **E5 / avoid** | Consume Moveworks/PDS Health AI permission trimming and source-native or proven ReBAC services; do not recreate a general ACL index. |
| Delegated and projected SaaS permissions | **2** | **E4** | Token keys, PKCE, principals, idempotency, and audit are modeled, but live token isolation/revocation/scopes/writes and projection entitlement fidelity are not certified. |
| Deep operational web UX | **4** | **E1** | App Framework already supports full-product UX where the exception is justified. |
| PDS Observability foundation | **4** | **E2** | Prometheus, structured logs, W3C context, correlation, health/readiness, and optional OTLP exist; cross-platform outcome correlation remains. |
| Closed-loop application intelligence | **2** | **E3-E4** | Typed entities, permissioned archetypes, answer envelopes, operations, bounded renderers, Kafka/projection contracts, web/mobile paths, and telemetry are useful parts. There is no live PDS Health AI observe-to-outcome loop, complete action path, production mobile continuation, or end-to-end quality/cost trace. |
| Materialized projection lane | **2-3** | **E3** | MongoDB reads are mature, but one source-to-projection freshness, coverage, reconciliation, and fallback lane still needs complete proof. |
| Live SaaS reads and governed actions | **2** | **E4** | Important SaaS paths remain compiler-contracted or gated; delegated live action proof is absent. |
| End-to-end production Kafka/kappa | **2** | **E4** | Contracts exist, but broker client, durable checkpoint, certified loop, replay, lag, coverage, DR, and support proof do not. |
| Native mobile | **3** | **E4** | Generated Expo routes, token bridge, entity screens, and policy-aware client exist; device, distribution, security, and support proof remain. |
| Scalability controls / PDS-scale proof | **4 / 2** | **E2 / E4** | Query/load guardrails are strong; 17,000-user/1,100-location capacity, cost, failure, recovery, and support evidence are not. |
| Enterprise employee search and assistant | **1-2** | **E5 / avoid** | Consume Moveworks and PDS Health AI; do not fund parity. |

Evidence basis date: **2026-07-10**. Provider counts and semantic-area posture
come from `scripts/appfw framework provider-graduation --json` and its retained
`target/appfw/provider-graduation.json` contract; database live areas remain
provider-test evidence. Mobile engineering posture comes from the generated
mobile contract plus `scripts/appfw product mobile-test --json`, `--run-local`,
and `--device-preflight`, but those legacy mobile artifacts are local,
non-authoritative diagnostics that keep `candidate_ready:false` and
`release_ready:false`; they do not substantiate candidate, distribution, or
release claims. A future source-bound checker owns candidate evidence, while
named humans own distribution and release. Tenant/application-authorization
posture is supported by focused runtime tenant-isolation and MCP role/scope
tests and the threat-model litmus. These local artifacts substantiate maturity
2-4 engineering claims but do not substitute for the managed proof required by
maturity 5.

The directional gap from this foundation to a broad cross-SaaS, native-mobile,
managed-scale enterprise-product claim is **45-65 blended person-months** and
roughly **12-18 calendar months** with a stable 5-7 person platform nucleus plus
security, SRE, data-platform, mobile, and product participation. This is a
planning range, not a commitment or the cost of each application. Narrower
claims should land sooner by packaging existing E1 advantages, completing one
E3 materialized projection lane, and then selecting only the E4 proofs required
by funded journeys. The 45-65 range assumes one PDS enterprise tenant boundary.
Use 55-80 blended person-months if PDS intentionally requires a general
SaaS-grade customer tenant control plane that independently provisions,
configures, meters, migrates, restores, and supports multiple customer tenants.

Treat `tenant` as a security and operational isolation boundary, not as a
synonym for every location, market, department, or legal entity. Those are
normally policy attributes unless independent administration, secrets,
retention, encryption, deployment, residency, restore, or service boundaries
make separate tenancy necessary.

Permission work for the recommended boundary is directionally 18-30 blended
person-months, mostly overlapping the E4 kappa, governed-action, multi-tenancy,
security, and operations packages rather than adding to the 45-65 range. A
general search permission index would be separate E5 category-scale work and is
explicitly out of scope.

### Goal-state proof obligations

Do not describe App Framework as the goal-state enterprise fabric until PDS can
show all ten obligations:

1. One live-certified ServiceNow read and one low-risk governed action used by
   both a vendor surface and a PDS product.
2. One authenticated PDS Health AI information pack participates in a complete
   governed intelligence loop with typed durable state, evidence, fallback,
   outcome/evaluation telemetry, and useful native web/mobile renderings; an
   approved vendor surface consumes the same semantics when that journey needs
   it.
3. One executable archetype reused across Nexus, a citizen-app graduation, and
   a Pega redesign, with at least 50% lower comparable second-use engineering
   effort.
4. Tenant-derived model ingestion, drift detection, consumer contracts,
   visible freshness/provenance, and a clear PDSOne/system-of-record boundary.
5. Permission-preserving projection proof with entitlement provenance,
   policy/version, dynamic field masks, revocation/freshness SLOs,
   tombstone/reassignment/group-change handling, fail-closed stale behavior,
   source reconciliation, and negative coverage across every delivery channel.
6. Production Kafka evidence when eventing is claimed: broker, schema, replay,
   dead-letter handling, lag, disaster recovery, and support.
7. Managed web/mobile accessibility, security, scale, resilience, deployment,
   incident, recovery, and support evidence.
8. Multi-tenant proof covering authenticated tenant propagation, adversarial
   isolation, provisioning/offboarding, configuration and secrets, quotas,
   migration, observability, cost allocation, backup/restore, and support to
   the explicitly chosen tenant boundary.
9. Cross-platform PDS Observability covering technical health, AI quality and
   cost, release identity, dependency/lineage, and business outcomes.
10. A durable product/platform/SRE operating model and measured economics that
   prove reuse compounds across the application portfolio.

## Product Architecture Principles

1. **Product intent is source.** Product briefs, intake files, model proposals,
   app manifests, schemas, and evidence contracts are the product source of
   truth. Generated code is a projection.
2. **Capability stays portable; surfaces earn their place.** SaaS platforms may
   own a native employee experience as well as workflow or records. PDS retains
   reusable model, context, operation, event, intelligence, evidence, and
   observability contracts, and funds a PDS surface only for a structural need
   or measured advantage.
3. **Adopt vendor models before composing journeys.** Do not invent a broad
   canonical enterprise model. Adopt each vendor's object and workflow model
   behind generated, version-pinned, drift-checked contracts, then compose only
   the fields a funded journey consumes.
4. **Federate permission authority.** Launch rules govern capability
   availability, source/delegated authorization governs authoritative records
   and actions, and App Framework Rego governs contextual PDS application
   policy, rows, fields, actions, and obligations. Use proven ReBAC/source-native
   services for high-cardinality ACL relationships.
5. **One governed operation path.** HTTP, GraphQL, MCP, Kafka, sync workers,
   chat, ambient AI, mobile, and UI actions must converge on named operations,
   policy, identity, audit, idempotency, and rollback/compensation posture.
6. **Report, then enforce, then graduate.** New capability starts report-only,
   becomes fail-closed when the evidence shape is stable, then graduates with
   live evidence.
7. **Agents scale the system only when the system is reviewable.** Every agentic
   workflow needs clear task routing, small branches, retained artifacts,
   independent review, tech-debt capture, and human approval checkpoints.
8. **Product packaging is part of the product.** A framework capability is not
   done until downstream teams can consume it from approved packages, with
   version compatibility, upgrade guidance, and CI evidence.
9. **Composition precedes component expansion.** Shared UI capability earns
   maturity through coherent, measured admin, product, and mobile workflows,
   not catalog count. Refactor and extend primitives only when a production-
   shaped composition exposes a named gap.
10. **Intelligence closes a governed work loop.** Structured results, evidence,
    freshness, permissions, recommendations, checkpoints, action previews, and
    outcomes become durable inspectable application state. Chat is a secondary
    invocation or exploration mode, and an agent never supplies arbitrary
    executable UI or transaction authority.
11. **Use the least expensive reliable intelligence.** Prefer authoritative
    facts and deterministic logic, then event/rule processing, retrieval,
    specialized or on-device models, larger models, and finally a multistep
    agent only when the named task and assurance requirements justify it.

## Packaging, Delivery, Versioning, And Access Model

App Framework should be packaged as an internal platform product with several
coordinated artifacts:

| Package surface | Consumer | Direction |
| --- | --- | --- |
| `appfw` CLI/toolchain bundle | Product developers, agents, CI | ProGet-distributed binary bundle with prebuilt generator/db tooling; no adjacent framework checkout required for normal product work. |
| Rust crates | Product backend/API/policy crates | ProGet Cargo registry, semver, compatibility matrix, `appfw.lock`, no committed local `path` overrides in product repos. |
| PDS UI packages/tokens | Product web/mobile apps | Source-owned framework package with fork detection, generated contract adapters, and release/a11y/catalog evidence. |
| Product templates/archetypes | Citizen apps, legacy apps, greenfield, Nexus | Versioned product profiles with intake, model proposal, generated ownership, frontend/mobile, release, and SRA defaults. |
| SaaS/provider connector packs | Nexus and integration-heavy products | Evidence-gated connector crates with vendor contract docs, auth posture, projection/write capability states, provider-test evidence. |
| Agent harness/skill pack | Agent-enabled development teams | Repo-native and packaged skills, slash command guidance, PR review, SRA, tech debt, product intake, modernization, release evidence procedures, and clean-machine replication contracts. |
| Deployment/release bundles | Platform/release teams | CI templates, Kubernetes deployment references, sync-worker shapes, observability defaults, release evidence and SRA package outputs. |

Versioning rule: product teams should be able to answer "which framework
version generated, built, released, and upgraded this app?" from committed lock
files and retained artifacts, not tribal memory.

Access rule: citizen developers and product owners should enter through guided
intake and review artifacts; software engineers should enter through CLI,
package, and extension contracts; Nexus should enter through a workflow-product
profile with stricter evidence demands.

## CI/CD And Change Management Strategy

AI-enabled throughput raises the cost of weak change management. App Framework's
change model should be progressive and evidence-based:

1. **Local lane proof.** Fast focused commands prove the changed surface.
2. **PR review proof.** `change-impact`, handoff, review brief, PR Review Agent,
   and human approval prove the review shape.
3. **Integration branch proof.** The Integration Branch Manager proves combined
   lanes, dependencies, conflicts, and aggregate risk.
4. **Release authority proof.** Managed CI and live/provider-backed gates prove
   production claims.
5. **Post-merge product learning.** Tech-debt register, product metrics,
   incident/review findings, SRA outcomes, and CAB outcomes feed back into
   roadmap priority.

Change approval should become lighter for low-risk, well-paved work and stricter
for high-impact changes. The approval altitude is the key: humans approve
product intent, architecture, irreversible actions, risk acceptance, and release
authority; machines prove contracts, drift, policy, packaging, tests, and
evidence shape.

## Threats To Design Against

| Threat | Product response |
| --- | --- |
| Agentic code volume overwhelms review, creating low-quality "AI slop." | Lane-sized branches, PR Review Agent, review performance oversight, tech debt register, merge discipline, and focused checks. |
| Citizen apps become shadow IT under a prettier scaffold. | Intake evidence, classification, generated model proposals, SRA prep, release gates, and clear production graduation criteria. |
| Legacy modernization copies old architecture into new code, defaults every capability to App Framework, or lets an agent invent semantics and validate its own output. | Claim-specific evidence authority; separate as-is and approved platform-neutral to-be models; objective disposition; portable worker task/result records; independent differential/data/security proof; migration/rollback; and verified retirement or explicitly costed coexistence. |
| The PDS enterprise experience becomes an unjustified portal clone or claims readiness from visual polish. | Keep ServiceNow authoritative for workflow; pull one real package consumer through trust, read, projection, action, operations, mobile, and structured-intelligence proof; compare the same outcome against Employee Slate/Moveworks; stop or narrow when trust, outcome, support, or economics do not justify ownership. |
| MCP or chat becomes an alternate privileged backend. | MCP/chat tools route through the same named operations, policy, consent, audit, and HITL gates as deterministic UI. |
| SaaS writes execute without delegated identity, idempotency, or audit. | Governed-write evidence gates, named mutation registry, token isolation, principal envelope, replay protection, and write audit. |
| Freshness/provenance gaps erode trust in projections and AI answers. | Lineage/freshness artifacts, citations, resolved entity refs, policy-context re-resolution, and staleness UI cues. |
| Supply-chain/package drift makes generated apps hard to maintain. | ProGet packaging, compatibility matrix, dependency convergence, SBOM/AIBOM/provenance/signing posture, and upgrade commands. |
| The framework becomes a bag of scripts instead of a platform product. | Productized CLI, package surfaces, docs IA, skills, golden paths, self-service templates, and machine-readable evidence. |

## Product Management Waves

These waves are a strategy overlay. They should be reconciled into
`docs/release/roadmap.md` work items rather than executed as a second roadmap.

| Wave | Business outcome | Core work | Proof of progress |
| --- | --- | --- | --- |
| **PM-0 Portfolio Control** | Every incoming app is classified, owned, and routed to the right path. | App intake taxonomy, product-value scoring, Tech Debt Register, PM review prompts, branch/debt signals for `GO WITH CONDITIONS`. | Intake records include demand stream, owner, value hypothesis, risk class, and next evidence. PR reviews either clear debt or propose register entries. |
| **PM-1 Packaged Platform Consumption** | Product teams can build without a framework checkout. | ProGet CLI/crates/npm package flow, compatibility matrix, appfw.lock, registry-mode bootstrap, upgrade report, product CI templates. | New product app builds from approved package feeds; no committed local path overrides; compatibility artifact is retained. |
| **PM-2 Productization Factory** | Citizen PoCs and legacy apps move through repeatable evidence, objective treatment, target compilation, portable-worker, independent-proof, and retirement loops. | Chartered pilot and manual baseline; governed code/work/knowledge/CMDB/data/runtime evidence; deterministic analyzers; as-is graph and adequacy workbench; approved platform-neutral treatment; App Framework target compiler where selected; normalized worker task/result; differential/migration/retirement evidence. | One representative legacy vertical is accepted with traceability, safe migration or costed coexistence, support and retirement criteria; a second suitable app cuts normalized discovery-to-approved-model effort by at least 50 percent without lower quality. |
| **PM-3 PDS-Owned Enterprise Experience Product Proof** | One real employee product proves the coherent PDS experience and reusable capability fabric without duplicating ServiceNow workflow. | Independent package consumer; request/task/approval model; ServiceNow read/action evidence; permission-preserving projection; managed operations; typed PDS Health AI modules; durable attention; bounded administration; native-mobile continuation; Product Experience Model; and product-level Claude Code/Codex harness. | One named journey reaches managed evidence for its launched scope and beats or justifies itself against current PDS and Employee Slate/Moveworks baselines; otherwise the product scope narrows or stops. |
| **PM-4 Enterprise App Fabric / OMA** | Journey-scoped fabric layers can survive vendor UX churn without becoming an MDM or supergraph program. | Team Member Journey feasibility card, ServiceNow dictionary/workflow export intake, `saas-export` model proposal path, projection-only entity marking, consumer-contract drift checks, actionability-tier certification. | One funded journey step uses an adopted vendor model with freshness/provenance and tier evidence; broad canonical-model work remains out of scope. |
| **PM-5 Governed Integration And Action Plane** | Products can read from and safely act through SaaS/data systems. | SaaS executor, sync workers as separate deployable workers, kappa/CDC descriptors, provider certification, G1 delegated writes, MCP/Kafka parity. | Projections have freshness/lineage; writes stay disabled until live governed-write evidence; sync workers have deployment and ops evidence. |
| **PM-6 Closed-Loop Intelligence And Exception Surfaces** | The same governed context, evidence, recommendation, checkpoint, action preview, and outcome persist across the channels where work happens. | One PDS Health AI information pack; deterministic/event signal; composed `answer_envelope@1`, permissioned archetype, operation, view-registry, audit, and OTel paths; durable work state; trusted web/native renderers; deterministic fallback. | One named task completes a measured non-chat loop across web and native mobile with no channel-specific authorization, and a second real product proves what becomes reusable; vendor adapters remain optional consumers of the same semantics. |
| **PM-7 Enterprise Readiness And Resilience** | The platform can support regulated, scaled, observable production workloads. | Release-check, SRA package maturity, CAB package TODO, security/ops/performance certification, OpenTelemetry/SIEM, SBOM/AIBOM/provenance, rollback/runbook drills. | Live provider-backed release evidence, SRA-ready package completeness, CAB-ready change evidence, and operational readiness artifacts are retained. |

## Priority-Move Operating Split

Use this split when several threads are active. It keeps product strategy,
architecture implementation, integration, and human review from collapsing into
one broad agent branch.

| Role | Owns | Immediate priority |
| --- | --- | --- |
| **Strategist/Product Manager function** | Platform "why", strategic business themes, market/industry challenge, portfolio direction, strategic-goal freshness, value ranking, and roadmap faithfulness. | Challenge the current priority moves against business value and North Star fidelity; trigger freshness review when evidence changes; do not let stale branch state become strategy. |
| **App Framework Product Owner** | Current backlog/value slicing, acceptance criteria, product evidence, tech-debt/CAB/SRA priority recommendations, and near-term roadmap execution clarity. | Reconcile the refreshed execution order into lane-ready backlog items and turn fabric candidates into owner/metric/tier/evidence cards before implementation. |
| **XO** | Multi-thread operating-system marshalling, branch-train coordination, coordination hygiene, and escalation routing. | Keep Wave A/B/C ownership clear, prevent stale branch state from becoming operating truth, and route decisions to the right human/agent owner. |
| **Architect thread** | Lane design, implementation slices, architecture coherence, focused proof, and handoff evidence. | Implement only lane-sized work from the refreshed wave map, starting with reusable contracts and proof gates before feature breadth. |
| **Integration Branch Manager** | Branch dependency graph, neutral integration branches, conflict resolution, PR creation/update, aggregate checks, and merge readiness. | Refresh stale Wave 4 branches against current `main`/integration state before merge; preserve newer governance/review/SRA/CAB docs and gates. |
| **PR Review Agent / Tech Debt Steward** | Independent review, drift assessment, `GO WITH CONDITIONS` debt capture, and review-performance tuning. | Turn accepted conditions, skipped checks, repeated findings, and stale evidence into explicit debt or follow-up work. |
| **Framework Structure Steward** | Docs IA, skills, CLI contracts, code organization, generated-boundary clarity, and delivery-harness entropy. | Recommend maintenance items back to the Product Owner/Strategist before structure debt becomes feature drag. |
| **Product/Nexus proof threads** | Concrete product evidence, UX/product telemetry, workflow proof, and release/SRA/CAB inputs. | Prove value in Nexus or a downstream app before generalizing a platform capability. |

Wave completion is a merge/evidence state, not a branch-name state. A wave is
not done until its slices are refreshed onto the current integration base,
reviewed, merged or explicitly discarded, and backed by retained evidence that
still matches the current docs, CLI, skills, and release gates.

## Delivery Throughput Strategy

The framework must optimize the whole path from idea to trusted merge:
branching, implementation, local proof, PR review, remote CI, merge, main CI,
and release evidence. The product goal is not simply "more agent commits"; it
is shorter cycle time with higher confidence and less human review fatigue.

Use a risk-tiered delivery model:

| Delivery tier | Product intent | Expected behavior |
| --- | --- | --- |
| **Focused leaf** | Let teams iterate quickly on one capability slice. | Small branch, early `change-impact`, focused local checks, handoff, focused review, PR pipeline. |
| **Integration family** | Let parallel lanes converge without turning `main` into conflict cleanup. | Rebuild from current approved base, merge related leaves once, run aggregate evidence and comprehensive review. |
| **Release authority** | Make production/package claims trustworthy. | Main/tag CI retains provider-backed, supply-chain, security, release, SRA/CAB, and provenance evidence appropriate to the claim. |

Throughput improvements should be judged by whether they move failures left:
from main CI to PR CI, from PR CI to local checks, from local checks to
docs-check subchecks/review briefs, and from human memory to machine-readable
contracts. Never remove a safety gate just to shorten a run; instead, make the
gate more targeted, cacheable, parallel, or easier to invoke before the branch
waits in Bitbucket.

The harness itself must be portable. Team adoption is not ready until a new
developer can reproduce the operating guide, skills, slash-command behavior,
review gates, and evidence expectations from a fresh checkout plus approved
package feeds. Local-only prompts, personal aliases, untracked regulatory files,
tokens, and chat history are inputs or conveniences, not product surfaces.

## Prioritization Rules

1. **Nexus-relevant workflow/product primitives outrank generic polish** when
   they also help the other demand streams.
2. **Packaging and compatibility outrank new capability** when downstream teams
   cannot reliably consume the capability.
3. **Evidence-gated integration outranks guessed integration** even if the
   guessed integration is faster to demo.
4. **Generated, reusable product surfaces outrank one-off product code** unless
   a product has a legitimate extension point.
5. **Security, SRA, CAB, and release evidence are business enablers**, not
   compliance drag, because they let the enterprise trust faster delivery.
6. **Review throughput is a product metric.** If humans cannot understand and
   approve branches, agent throughput is creating debt rather than value.
7. **CI friction is product debt.** Repeated slow, opaque, or late failures
   should become backlog items for local preflight, clearer artifacts, better
   caching, stronger changed-surface selection, or parallel CI structure.

## Metrics Product Management Should Track

| Metric | Why it matters |
| --- | --- |
| Intake-to-reviewed-model time | Measures whether app-framework helps citizen/legacy/greenfield teams reach a real product contract quickly. |
| Legacy evidence-to-approved-transformation time | Measures whether code, Jira, Confluence, data, and operating evidence become a trustworthy modernization decision efficiently rather than a prolonged manual archaeology exercise. |
| Legacy model traceability and correction rate | Measures target elements linked to source evidence and approved decisions, unresolved critical unknowns, and semantic corrections found after model approval. |
| Portable-worker quality and intervention | Compares Claude Code, Codex, deterministic recipes, and qualified specialist engines on accepted task results, exact-check success, scope deviation, human correction, retries, runtime, and cost without making one worker's private state authoritative. |
| Comparable second-use modernization effort | Tests whether the harness compounds learning; normalize by bounded workflow complexity and require at least 50 percent lower discovery-to-approved-model engineering effort without lower quality rather than counting generated artifacts. |
| Modernization cost per accepted retired capability | Includes tools/models, platform allocation, all participating labor, environments, migration/dual run/cutover, target support, residual legacy cost, rework and incidents, then credits only verified retired cost. |
| Reviewed-model-to-generated-app time | Measures generator and proof-loop productivity. |
| Critical-task success, time, first-click, and recovery | Tests whether a PDS composition materially improves real work rather than merely looking newer. Compare against the existing surface and current entitled vendor baseline. |
| Signature experience quality | Tracks five- and 30-second impressions, blinded preference, crafted/fresh/intelligent/distinctive/beautiful ratings, semantic differentials, and signature-object/interaction recall. Keep this independent from operational superiority and calibrate, rather than remove, the provisional 65 percent preference and one-point composite-lift bars. |
| Experience trust and evidence comprehension | Measures whether users can identify source, freshness, permission, AI involvement, consequence, and recovery before acting. |
| Agent-generated UI first-pass compliance and human rework | Measures whether executable recipes, tokens, floorplans, and design-code mappings produce coherent pages without substitute primitives or visual exceptions. |
| Manual and automated accessibility coverage | Tracks WCAG 2.2 automation plus keyboard, screen-reader, zoom/reflow, reduced-motion, high-contrast, and touch evidence across complete tasks. |
| Frontend performance and stability | Tracks Core Web Vitals, bundle/route budgets, large-data behavior, responsive overflow, and visual/layout regressions in supported production contexts. |
| Intelligence-loop completion and cost per successful outcome | Measures the full trigger-to-context-to-result-to-checkpoint-to-action-to-outcome path, including runtime, data, evaluation, human correction, support, failure/rework, and deterministic fallback cost. |
| Intelligence quality and control | Tracks claim-to-evidence resolution, freshness, unsupported claims, edit/correction, accept/dismiss/defer, intervention, kill-switch behavior, and downstream outcome by task class and user population. |
| First-pass verification success for agent-authored changes | Measures agent-legibility and harness quality. |
| Percentage of product code generated vs human-owned | Measures how much surface benefits from drift detection and shared hardening. |
| Package consumption health | Measures whether downstream teams use approved ProGet packages rather than local framework paths. |
| Clean-machine harness replication success | Measures whether a new developer can clone, run `doctor`, list skills, execute the review/handoff loop, and understand local-only inputs without private setup knowledge. |
| Review findings per branch and repeated finding categories | Shows where docs/checks/skills should be improved. |
| Tech debt opened vs retired | Prevents `GO WITH CONDITIONS` from becoming silent architectural erosion. |
| Nexus workflow retirement progress | Measures real business value: Pega workflows replaced, task/request cycle-time visibility, and ServiceNow-governed action readiness. |
| Fabric/OMA proof progress | Measures whether vendor object/workflow models are adopted with drift checks and tier evidence for funded journeys instead of becoming generic connector inventory. |
| SRA package completeness by product | Measures whether teams can reach security review with fewer surprises. |
| CAB package completeness by release/change | Measures whether teams can reach change approval with clear impact, risk, rollback, test, deployment, communication, and evidence links. |
| Release DORA-style outcomes | Lead time, deployment frequency, change failure rate, and recovery time should improve together, not trade off. |
| PR cycle time by change class | Shows whether Class A/B work is moving quickly while Class C/D work gets the scrutiny it needs. |
| Local-caught vs CI-caught failures | Shows whether the harness is moving repeated failures left into cheaper feedback loops. |
| CI queue/retry waste | Reveals avoidable pushes, duplicate remote runs, stale integration branches, and unclear release-lite evidence. |

## Operating Model For PM, Architect, And Integration

The durable human/agent role boundary lives in
[`docs/start/agentic-human-operating-model.md`](../start/agentic-human-operating-model.md).
Product strategy should use that contract as the default collaboration model.
In product-management terms:

- **Strategist/Product Manager function** owns audience promise, strategic
  business value, market/industry challenge, portfolio pressure, roadmap
  faithfulness to the North Star, and periodic freshness review when product
  evidence or industry/business signals suggest the goals need clarification or
  reprioritization. It also provides Strategic Pull Reviews that translate the
  North Star, enterprise app fabric strategy, product strategy, roadmap, XO
  board, and delivery evidence into forward pressure for the XO and Product
  Owner without taking over tactical branch decisions.
- **Product Owner** owns current wave priority, value slicing, acceptance
  criteria, product evidence expectations, and tech-debt/CAB/SRA priority
  recommendations. Fabric candidates must be reduced to owner/metric/tier/
  evidence cards before the Architect turns them into lanes.
- **Architect thread** owns lane implementation, architecture coherence, proof,
  review output, and handoff.
- **Integration Branch Manager** owns dependency order, integration branch
  hygiene, conflict handling, aggregate checks, PR creation/update, and merge
  readiness.
- **PR Review Agent** owns independent review of changed code/docs/CLI/skills/
  config/evidence and human approval guidance.
- **Tech Debt Steward** can be invoked by PR review or PM review to decide
  whether accepted conditions, skipped checks, repeated findings, or residual
  risks belong in the register.
- **Framework Structure Steward** reviews docs IA, skills, CLI contracts, code
  organization, generated-boundary separation, and delivery-harness entropy,
  then recommends maintenance lanes back to the Product Owner/Strategist.

The product-management role should ask the hardest question before approving
new waves:

> If twenty teams and several coding agents use this capability next quarter,
> does it reduce enterprise delivery risk, or does it merely create more code?

## Product Review Checklist

Use this checklist before adding, approving, or merging substantial roadmap work:

- Which demand stream does this serve: citizen PoC, legacy modernization,
  greenfield product, Nexus, or framework platform consumption?
- What business outcome becomes faster, safer, cheaper, or more measurable?
- What is the source contract: model, manifest, provider evidence, UX contract,
  package contract, or release evidence?
- What is the proof path: report-only, enforce, live evidence, release
  authority?
- Which package surface must downstream teams consume?
- Which docs, skills, CLI, checks, and review harnesses must stay aligned?
- What security/SRA/privacy/PHI/agentic risks are introduced?
- What is the rollback, upgrade, and compatibility story?
- If this requires CAB/change approval, what package evidence proves impact,
  risk, test coverage, rollout, rollback, communications, and approvals?
- If this ships with conditions, what tech debt entry or paydown trigger is
  required?

## What Not To Optimize For

- More generated code without clearer product contracts.
- More agent parallelism without better review and integration discipline.
- Chat-first demos that bypass workflow modeling, grounding, policy, and audit.
- Vendor-native or PDS-owned UX selected by preference instead of comparative
  product, control, structural-fit, and economic evidence.
- Local demo success that cannot become retained release/SRA/ops evidence.
- Packaging features that only work from an adjacent framework checkout.
- Product-specific hero work that does not become reusable for the next app.
