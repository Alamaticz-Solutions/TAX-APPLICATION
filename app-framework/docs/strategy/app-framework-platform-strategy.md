# App Framework Platform Strategy

> **Status: platform value strategy.** This document answers one question:
> **How does App Framework accelerate PDS Health's growth from a healthcare
> company into a technology-enabled healthcare ecosystem?** It is the business
> "why" behind the [North Star](product-development-north-star.md), the
> [Product Management Strategy](app-framework-product-management-strategy.md),
> the [Enterprise App Fabric Strategy](enterprise-app-fabric.md), and the
> [Roadmap](../release/roadmap.md).

## Executive Answer

App Framework accelerates PDS Health's growth by making high-value internal
applications faster, cheaper, safer, and more reusable to create and maintain.

It does this by turning business-process intent into governed product
contracts, generated application foundations, reusable archetypes, qualified
PDS web/mobile surfaces, integration adapters, release/SRA evidence,
observability, and agent-assisted review loops. The platform should let PDS scale new practices, new services,
new shared-services processes, citizen-developed solutions, legacy
modernization, and Nexus workflows without creating a new silo each time.

The value is not "more apps." The value is **reusable enterprise capabilities
that reduce the cost and risk of the next app**.

The canonical strategic goals are stated in the North Star's
[Strategic Goal Statement](product-development-north-star.md#strategic-goal-statement).
This document is the business-value expression of those goals: why the platform
is worth funding, which enterprise outcomes it improves, and where App
Framework owns the layer around systems of record without replacing them.

In App Fabric terms, App Framework is the PDS-owned **model-driven application
production system, enterprise-experience fabric, and capability runtime**,
aligned to the enterprise kappa architecture. It preserves adopted object-model
contracts, governed actions, freshness/provenance, structured intelligence,
evidence, and observability across a coherent PDS web/native-mobile product and
approved embedded surfaces while enterprise platforms remain authoritative for
their domains. Employee Slate/Moveworks is the strongest comparator and a
credible fallback, not the default target. PDS Health AI remains a consumed
retrieval and information-pack capability, not a transaction authority or
parity target.

## Why This Matters To PDS Health

PDS Health's growth mandate creates a platform problem. Supporting tens of
thousands of team members, thousands of practices, new business capabilities,
modernized legacy systems, and a unified team-member experience cannot scale
through one-off apps, bespoke integrations, duplicated security reviews, and
fragmented user experiences.

App Framework should help PDS improve five business outcomes:

| Outcome | Platform contribution |
| --- | --- |
| **Growth** | Shorten the time required to launch new practices, services, operational workflows, and internal products. |
| **Productivity** | Reduce manual work, re-keying, status chasing, duplicate intake, and support overhead through generated foundations and governed automation. |
| **Experience** | Give office teams, clinicians, shared services, leaders, and practice owners a consistent PDS experience instead of a scavenger hunt across tools. |
| **Technology rationalization** | Reduce application sprawl, duplicate capabilities, legacy platform costs, and fragile departmental tools. |
| **Trust and speed** | Move security, privacy, release, SRA, audit, observability, and human review earlier so teams can move faster with stronger evidence. |

## Strategic Thesis: Stop Funding Apps; Fund Reusable Capabilities

Every one-off app built today is a future modernization problem. Every reusable
capability becomes a force multiplier.

The platform should treat applications as proof points for reusable
capabilities. A product may start as a De Novo tracker, a shared-services
request flow, a legacy replacement, or a citizen-developed prototype, but the
platform investment should be justified by what becomes cheaper, safer, or more
consistent for the next product.

Fund a platform capability when at least one of these is true:

- It unlocks a high-value Nexus or PDSOne workflow outcome.
- It reduces repeated work across citizen-developed, legacy-modernized, and
  greenfield products.
- It removes a release, SRA, security, or operational bottleneck.
- It makes downstream product teams able to consume the framework from
  approved packages instead of a local checkout.
- It produces measurable adoption, cycle-time, risk-reduction, or cost-removal
  evidence.

Do not fund a capability because it sounds modern, agentic, AI-native, or
architecturally elegant. If it cannot name the next business outcome it improves
and the metric that proves it, it is not ready.

## What App Framework Owns

App Framework owns the enabling layer that makes product delivery repeatable:

| Platform responsibility | What this means |
| --- | --- |
| Product intake and value scoring | Capture business owner, workflow, users, demand stream, value hypothesis, risk class, and success metric before implementation. |
| Generated product foundation | Produce consistent backend, frontend, model, policy, test, handoff, release, and evidence surfaces from explicit contracts. |
| PDS experience inheritance | Provide composition-first PDS tokens, floorplans, accessible behavior, generated views, native-mobile participation, and bounded intelligent surfaces under the "calm precision, expressive intelligence, unmistakable craft" thesis. Familiar patterns carry routine work; a few domain-specific signature moments must prove both operational and comparative experience quality. Component count is not the outcome. |
| Journey and experience projection | Preserve a versioned, portable view of one outcome path across roles, systems, waits, branches, channels, and time, including exact handoff, exception, resumption, correlation, and outcome semantics. This projection does not replace source workflow or record authority. |
| Governed attention and resumption | Define durable attention-item and preference semantics, grouping/deduplication, expiry, acknowledgement/snooze, policy overrides, exact deep links, accessibility, adapter contracts, and observability. A toast or push notification is never the only work record. |
| Bounded workspace and module contracts | Supply policy-locked role defaults, versioned user layouts, allowlisted typed modules, backend eligibility, responsive and accessible ordering, migration/fallback, reset/recovery, and explicit intelligence/action levels. Personalization cannot change authority or source truth. |
| Governance and evidence | Make security, data classification, audit, SRA, release, review, and operational evidence part of the product lifecycle. |
| Integration contracts | Connect to systems of record through governed adapters, projections, named operations, freshness/lineage, identity context, and audit. |
| Fabric object-model adoption | Adopt vendor object and workflow models behind generated, version-pinned, drift-checked contracts; compose only at journey/theme level. |
| Product telemetry | Make adoption, friction, workflow cycle time, agent assistance, intervention rate, and outcome metrics visible. |
| Agent-assisted delivery | Provide skills, slash commands, review briefs, PR review, tech-debt capture, and handoff artifacts that help humans safely approve faster work. |
| Packaging and compatibility | Let product teams consume approved CLI, crate, UI, template, connector, CI, and agent-harness packages with version/upgrade evidence. |

## What App Framework Integrates, Not Owns

"One platform" does not mean App Framework becomes every enterprise platform.
It means products inherit a common delivery and governance spine while
integrating with the correct enterprise systems.

| Enterprise capability | App Framework posture |
| --- | --- |
| Identity provider | Integrate with Okta/SSO/MFA and enterprise identity. App Framework owns app-level policy contracts, claims handling, audit, and evidence, not the identity platform. |
| Enterprise workflow engine | ServiceNow should remain the enterprise workflow/orchestration engine where it fits. App Framework owns PDS-facing journey/experience projections, generated task/request models, integration contracts, exact resumption, evidence, and product-specific workflow participation. A native workflow engine should be considered only if real demand proves ServiceNow cannot satisfy a repeated product need. |
| Systems of record | Workday, Epic, Salesforce, ServiceNow, iCIMS, Snowflake, and other platforms remain authoritative for their domains. App Framework owns safe access, projection, policy, and product experience around them. |
| Enterprise data platform | App Framework consumes governed data products, streams, projections, CDC, APIs, and warehouse outputs. It does not become the enterprise lake, warehouse, MDM, canonical data model, or Kafka platform. |
| AI model/platform layer | App Framework provides governed AI product surfaces, answer envelopes, citations, policy-context resolution, eval evidence, and action preview controls. It should not become the enterprise foundation-model provider. |
| Notification/document/search services | App Framework should expose attention, document, retrieval, and delivery-adapter contracts while enterprise services retain delivery/search infrastructure and source authority. It should not duplicate those services unless repeated product demand proves a missing platform capability. |
| Kubernetes/CI infrastructure | App Framework supplies templates, checks, release/SRA evidence, and deployment contracts. Platform/release teams own the managed infrastructure. |

This boundary is strategic. It prevents App Framework from becoming an
unbounded replacement for ServiceNow, Okta, Snowflake, or cloud infrastructure
while still making those platforms usable through a coherent PDS product
experience.

## Capability Inheritance Model

Every product built on App Framework should be able to inherit common
capabilities through explicit contracts and approved adapters:

- identity context
- authorization and policy checks
- PDS navigation, layout, and design-system usage
- outcome, journey, role/channel experience, exact-resumption, and correlation semantics
- durable attention items, user preferences, and delivery adapters
- bounded workspace layouts and allowlisted work modules
- task/request/workflow participation patterns
- data access and projection contracts
- search and knowledge retrieval integration
- notification integration
- product and agent analytics
- governed AI assistance and action preview
- audit, observability, release, and SRA evidence
- upgrade and compatibility metadata

Inheritance is valuable only when it lowers the cost and risk of the next
product. It is not a reason to centralize ownership of every underlying
enterprise service inside the framework.

## Value Loops

App Framework should create compounding value through repeatable loops:

| Loop | Value generated |
| --- | --- |
| Citizen PoC to governed product | Useful business prototypes become secure, supportable products instead of shadow IT. |
| Legacy app to reusable capability | Modernization retires old applications while extracting shared capabilities for future products. |
| Nexus workflow proof | Nexus proves transparent requests, tasks, ownership, status, and ServiceNow-governed actions in a flagship product. |
| Packaged platform consumption | Product teams build and upgrade from approved packages, reducing local setup and fork risk. |
| Evidence to faster approval | SRA, release, observability, and review artifacts reduce late surprises and approval friction. |
| Telemetry to better roadmap | Adoption, friction, cycle-time, support, and risk data reprioritize the roadmap toward ROI. |

## ROI Model

The platform should prove ROI using evidence, not confidence.

Use this operating equation:

```text
Platform ROI =
  (cycle-time reduction
 + cost avoided
 + legacy/platform spend retired
 + support load reduced
 + risk and compliance friction reduced
 + growth capabilities accelerated)
 / platform investment
```

Track the proof through:

- time from intake to reviewed model
- time from reviewed model to generated app
- time from product start to release-ready evidence
- product teams consuming approved packages
- legacy apps retired or consolidated
- duplicated capabilities avoided
- workflow cycle-time reduction
- hours saved or status-chasing reduced
- SRA/release findings avoided or resolved earlier
- user adoption and active workflow participation
- agentic changes passing review without repeated findings

The July 14, 2026 decision-economics model makes the funding standard concrete.
Its narrower hybrid scenario established a useful comparator: Pega redesign and
transition is a $4.549 million common three-year cost and cannot be used to
justify App Framework. At the illustrative $10 per eligible user per month, the
modeled hybrid added a $4.255 million strategy-specific premium: $3.700 million
of framework team cost plus $0.750 million of cloud/tooling, net of a $0.195
million reuse credit. That creates an unadjusted hurdle of $1.418 million per
year, $6.95 per eligible user per month, or $0.709 million per planned product
journey before execution-risk margin.

Release platform funding in evidence-backed increments. Each increment must
name how reuse, avoided vendor or services work, reduced support, faster product
throughput, application retirement, or a structurally better repeated-use
experience contributes to clearing the hurdle. The explicit PDS-owned target
has a wider ownership burden than that model captured. Directional planning is
30-45 blended person-months for a bounded first lovable product, 75-120 for
broad front-door maturity, and 6-10 durable FTE equivalents for ongoing
operation. These are planning ranges, not commitments. They must be refined by
product scope and cannot support a production or replacement claim without the
named capability, trust, adoption, and operating evidence.

## Prioritized Platform Themes

These themes translate the "why" into product-management focus. The ranked
themes live in the [Product Management Strategy](app-framework-product-management-strategy.md);
the executable backlog lives in the [Roadmap](../release/roadmap.md). This
document explains why the themes matter.

1. **Enterprise app fabric and object-model adoption.** For funded, journey-
   scoped experiences, adopt vendor object/workflow models verbatim behind
   generated contracts, certify actionability tiers, and drift-check the
   fields products actually consume.
2. **Productization factory.** Convert citizen-developed apps through the paved
   lifecycle, and modernize legacy apps through a PDS-owned, agent-portable
   control plane: governed evidence, claim-specific authority, separate as-is
   and approved platform-neutral to-be models, objective treatment, target
   compilation where App Framework is selected, independent proof, and legacy
   retirement economics.
3. **Packaged platform consumption.** Let product teams build without a local
   framework checkout, with compatibility and upgrade evidence.
4. **Agentic review and tech-debt loop.** Keep throughput understandable and
   upgradeable by turning review conditions and repeated failures into owned,
   retireable debt.
5. **Structured application intelligence.** Turn governed state, PDS Health AI
   evidence, deterministic or event signals, recommendations, checkpoints,
   action previews, and outcomes into portable typed application state rather
   than another assistant destination.
6. **PDS-owned enterprise experience.** Build one coherent, production-shaped
   web/native-mobile product with channel-native participation, durable
   attention, exact resumption, bounded administration, and deep operational
   work. Use Employee Slate/Moveworks as the strongest comparator and fallback,
   not as the target architecture.
7. **Product signal intake and value scoring.** Ensure the right work enters
   roadmap consideration with business owner, outcome, metric, and risk.
8. **Product and agent outcome telemetry.** Prove adoption, friction, workflow
   improvement, and ROI rather than counting generated code or agent activity.
9. **PDS-owned enterprise-experience product proof.** Prove transparent
   requests, tasks, ownership, status, structured intelligence, governed
   actions, mobile continuation, and product lifecycle in one real downstream
   package consumer, with explicit stop/narrow criteria against the strongest
   vendor baseline.
10. **SRA, CAB, and release evidence maturity.** Make security, change-advisory,
   and release approvals faster by generating evidence early and keeping it
   current.
11. **Governed integration and action plane.** Safely connect products to SaaS,
   APIs, MCP, streams, projections, and named actions without bypassing policy
   or audit.

This is strategic value order, not a license to ignore dependencies. A live
provider read, preview-only operation, release gate, or security control moves
earlier when it is required to prove a higher-ranked outcome.

For legacy modernization, App Framework should own the evidence and decision
system that no general coding tool can own for PDS: source authority,
conflicts/unknowns, treatment approval, target semantics, verification
obligations, and retirement learning. It should use Claude Code, Codex,
deterministic analyzers, and specialist transformation engines as replaceable
workers behind one task/result contract. The first investment is a chartered,
measured .NET/IIS/Angular/SQL Server pilot and one independently verified
vertical, not a general agent, universal ontology, connector catalog, or
automatic App Framework funnel.

The defensible PDS experience signature is not generic visual modernity. It is
calm enterprise craft plus cross-system continuity, trustworthy structured
intelligence, governed actions, and portable web/mobile delivery. Preserve the
existing React/Vite/Expo foundation and broad component catalog. Invest next in
three measured reference compositions, a generated cross-channel token source,
six canonical floorplans, one accessible behavior substrate, bounded
intelligent composition, executable agent design recipes, and experienced
product-design judgment. Any superiority claim must come from a measured PDS
workflow against the current available vendor baseline, not from screenshots
or comparison with PDS's older implementation.

The product model beneath that signature is outcome -> journey -> experience ->
workspace -> module, with durable attention enabling timely and exact
resumption. A future Nexus proof should version one cross-system journey and
its role/channel experience briefs; render a personal work canvas and durable
attention center; continue exactly into native mobile; expose bounded
readiness, evidence, and next-best-action modules; and give administrators
trace, eligibility, reset, evaluation, cost, and recovery views. This broader
goal remains held until product intake and source evidence exist. It is not a
generic portal, widget marketplace, journey studio, orchestration engine, or
authorization for personalization to alter policy, source truth, evidence, or
required controls. The target is a coherent PDS-owned enterprise experience;
Employee Slate/Moveworks remains the strongest comparator and fallback. Broad
adoption must still be earned through managed trust, measured outcome,
continuity, second-use, signature-quality, supportability, and lifecycle
economics.

The intelligence thesis is similarly narrow: App Framework is a channel-
neutral intelligence integration and action fabric, not an assistant, search
product, model platform, or universal agent runtime. Intelligence should close
one observable work loop: governed state -> understanding -> recommendation or
prepared work -> human/policy checkpoint -> governed operation -> recorded and
evaluated outcome. PDS Health AI owns retrieval and information packs, not
transaction authority. Useful results persist as typed application state; chat
is an invocation or exploration surface. Web and native mobile share context,
evidence, action, checkpoint, and correlation semantics while keeping channel-
native component trees and behavior. Compose the existing permissioned
archetype, answer envelope, operations, view registry, audit, and telemetry
before adding contracts. Treat A2A, AG-UI, A2UI, MCP Apps, and similar protocols
as later adapters to a proven internal contract.

### Current execution authority

Platform strategy defines ownership boundaries, not lane order. The
[Roadmap Outcome Goal Register](../release/roadmap.md#outcome-goal-register) is
the sole current sequence. It keeps product-neutral readiness active, permits a
bounded disjoint CRM reference-experience proof, and gates the real PDS
employee product plus its read, projection, action, operations, and intelligence
work behind explicit exits and human activation.

When activated, that product must be the real independently packaged consumer
of one authoritative identity/policy/operation/provider/audit/telemetry spine.
ServiceNow remains workflow authority; Employee Slate/Moveworks remains the
strongest comparator and fallback. The active CRM composition is not that
external consumer and does not activate held Nexus behavior.

## PM Decision Standard

Before funding a theme or feature, the Product Manager should be able to answer:

1. Which growth, productivity, experience, rationalization, or trust outcome
   does this improve?
2. Which demand stream needs it: citizen-developed product, legacy
   modernization, greenfield product, Nexus, or platform consumption?
3. What reusable capability will exist after the first product proof?
4. Which enterprise platform owns the underlying capability, and what does App
   Framework own around it?
5. What metric proves ROI or tells us to stop?
6. What evidence will help security, release, operations, and human reviewers
   trust the result?

If those answers are weak, the work should stay in discovery. If they are
strong, the Architect and Integration threads can turn it into scoped,
evidence-backed implementation lanes.
