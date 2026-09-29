# App Fabric Master Implementation Plan — 2026-08-03

> **Status: canonical narrative implementation plan, not live execution
> authority.** This document owns the integrated App Fabric sequence, effort,
> dependencies, decision gates, and proof outcomes. The
> [Roadmap](roadmap.md#outcome-goal-register) owns priority and posture; the
> [Product Increment portfolio](../specs/product-increment-portfolio.json),
> validated Product Increment plans, and Program Flow Controller topology own
> live work, lanes, and admission; the
> [Hosted Product Factory Spec](../specs/hosted-product-factory.md) owns
> normative contracts and gates; the
> [factory architecture](../architecture/hosted-product-factory-architecture.md)
> owns target technical design; and accepted evidence owns completion. This
> plan does not ratify PDS Technology Strategy, approve architecture or
> funding, admit work, grant source-system authority, accept risk, authorize
> release, or change AF-OG08/AF-OG10 posture by being merged.

## Executive Outcome

Build an enterprise digital-product capability with a visible, trusted
management experience:

- the **Enterprise App Fabric** is the reusable platform capabilities,
  lifecycle spine, runtime contracts, patterns, and delivery machinery;
- the proposed **App Fabric Operations Console** is the role-aware web and
  bounded native-mobile management workbench; and
- the **Fabric Registry and Claim Graph** is the provenance-aware, bounded
  read model that connects authoritative evidence without replacing its
  source.

The Console should let executives, product owners, architects, engineers,
operators, Finance partners, and governance roles understand the application
estate; connect strategy to products, applications, components, artifacts,
deployments, usage, cost, outcomes, and lifecycle decisions; initiate governed
cross-team work; and, for qualified products, move typed proposals through the
Hosted Product Factory.

It is not the Fabric runtime, a universal employee portal, a second workflow
engine, or an authoritative CMDB/APM, backlog, portfolio, Finance, data,
continuity, governance, approval, mobile-store, or PDS Technology Strategy
system. Management-plane failure must never affect product traffic.

## Plan Authority And Reader Map

| Question | Authoritative source |
| --- | --- |
| Why the Fabric exists, what it owns, and what it is not | [Enterprise App Fabric Strategy](../strategy/enterprise-app-fabric.md) |
| What current industry and BOK evidence supports or challenges the vision | [August 2026 Industry And BOK Vetting](../assessments/app-fabric-industry-and-bok-vetting-2026-08-03.md) |
| Current investment order and planning posture | [Roadmap Outcome Goal Register](roadmap.md#outcome-goal-register) |
| Live Product Increments, lanes, dependencies, owners, and admission | [Product Increment portfolio](../specs/product-increment-portfolio.json), validated plans, and current PFC topology |
| Normative contracts, G-F1–G-F13 gates, security, and acceptance evidence | [Hosted Product Factory Spec](../specs/hosted-product-factory.md) |
| Planes, components, flows, identity, and target deployment design | [App Fabric And Factory Architecture](../architecture/hosted-product-factory-architecture.md) |
| Integrated sequence, effort, dependencies, and exit proofs | This master plan |
| Whether a capability is complete | Accepted exact-source evidence and destination/release proof |

No other prose plan should restate live status or maintain a competing
implementation sequence. Historical branch custody, PR, pipeline, and
prototype narratives belong in Git history or retained evidence, not this
plan.

## Strategic And BOK Boundary

This is an App Framework product and implementation plan. It was informed by
the PDS Technology Strategy authoring snapshot
`046e532c0e41b6911171f3d9aa961c0898d66140`, governing draft
`doc.enterprise_technology_strategy_2026_2030` version
`2026-07-31-draft.3`, machine-contract draft `contract.application` version
`2026-07-31-draft.3`, and Application Fabric companion draft
`doc.application_fabric_strategic_direction` version
`2026-08-01-draft.2`. That snapshot had no exact release tag and reported
`ratified:false`; it is reviewed authoring evidence, not governing policy.

The existing Application Fabric major decision packet must be amended rather
than creating a second strategy packet. While its decision and the related
BOK/EAC/governance gates remain open, the program may build reversible
read-only and proposal-first capability, fixtures, adapters, negative-access
tests, and experience proof. It may not claim that the common lifecycle spine
is enterprise policy, manufacture an approval, enforce an unapproved pattern,
write an external authority, enable general citizen supported-production
promotion, or present native mobile as a ratified enterprise mandate.

The exact open boundaries are preserved in the
[research assessment](../assessments/app-fabric-industry-and-bok-vetting-2026-08-03.md#bok-decision-boundary).

## Scope Model

Every application can be visible without every application becoming
controllable. Use three scopes:

1. **Federated estate visibility.** Searchable, role-filtered, source-bound
   visibility across the enterprise estate. Missing sources stay visibly
   missing.
2. **Fabric-managed lifecycle.** Lifecycle, compatibility, component,
   evidence, fleet, and upgrade management for explicitly qualified PDS-owned
   applications.
3. **Factory-managed change.** Typed, context-bound proposal and delivery
   operations only for applications explicitly admitted to the Hosted Product
   Factory.

The accepted management tiers remain `registered`, `observed`,
`appfw_managed`, and `factory_managed`. They are cumulative capability grants,
not maturity, risk, release-readiness, or business-value ratings. Registration,
ownership, or visibility grants no merge, release, production, risk, secret,
cross-app, Finance, architecture, or governance authority.

Digital products and applications must also be separate concepts. A digital
product may include web and native applications, services, APIs, data
products, agents, and MCP servers. Applications enable outcomes; the product,
journey, solution, and outcome carry the value hypothesis.

## Product Domains

| Domain | Initial outcome | Authority boundary |
| --- | --- | --- |
| Fabric Registry and Claim Graph | Stable provisional Fabric identity, application/product relationships, lifecycle, ownership, source-bound claims, exact provenance and freshness | Fabric owns its audited records and projections; enterprise identifiers remain linked authorities |
| Portfolio and Value | Search by product theme; connect hypotheses, outcomes, utilization, satisfaction, fully loaded cost, and Finance validation state | Product owns hypotheses; Finance validates recognized value and economics; portfolio authorities decide priority |
| Product Workspaces | Purpose, strategy references, work, environments, composition, evidence, outcomes, proposals, and lifecycle decisions per product/application | Accepted product records and named authorities remain controlling |
| Factory and Intent Hub | Context-bound change proposals plus correlated cross-team capability intents | Factory gates delivery; receiving-team systems own priority, assignment, approval, and fulfillment |
| Pattern and Governance Catalog | Exact EAC patterns/ADRs, applicability, qualifications, obligations, conformance evidence, expiry, and exceptions | EAC defines patterns and exceptions; Fabric starts report-only |
| Operations and Fleet | Desired/built/deployed/running identity, dependency/version posture, campaigns, usage, reliability, incidents, support and retirement evidence | Git, CI, artifact, GitOps, runtime, analytics, and operations sources remain authoritative |
| Mobile Release Center | Exact signed build/OTA/store identity, review, track, rollout, adoption, halt, rollback, and support state | Apple/Google/MDM/mobile authorities retain submit and release decisions |
| AI Portfolio Analyst | Deterministic quality/overlap checks followed by cited assumptions, alternatives, counterfactuals, evidence gaps, and confidence | AI recommends; human Product, Strategy, EAC, Finance, risk, and lifecycle authorities decide |

The first contract delta must decide, rather than assume, the shapes for:

- digital-product identity and product-to-app/surface membership;
- exact BOK release/digest, stable clause/unit identifiers, and decision state;
- versioned EAC pattern qualification, obligations, conformance, expiry, and
  exception references;
- value/cost records with baseline, counterfactual, period, denominator,
  confidence, validation state, and Finance authority;
- data-governance/catalog references for classification, stewardship,
  lineage, quality, access, retention, and consumer obligations; and
- a proposed `capability_intent` envelope or compatible profile that does not
  turn a prerequisite record into a second receiving-team backlog.

These are planned contract decisions, not accepted schemas. The accepted nine
FAB-A1 contracts remain unchanged until a separately admitted and reviewed
contract increment says otherwise.

## Accelerated-Mode Operating Doctrine

Accelerated mode means moving quickly through reversible proof while making
authority and evidence explicit. It does not mean weakening candidate or
release gates.

### External dependencies do not freeze local progress

An unresolved CMDB, BOK, EAC, Finance, data-governance, analytics, organization,
or source-system dependency blocks only the claim, write, enforcement, or
promotion action owned by that dependency. It does not block:

- contract and adapter design;
- role-bound fixture implementation;
- read-only projection and local testing;
- role-aware UX and accessibility proof;
- proposal and intent drafting; or
- fail-closed, stale-data, idempotency, reconciliation, and negative-access
  testing.

Until its authority is available, show the fact as `UNKNOWN`, `STALE`,
`MISSING`, or `UNAVAILABLE`; disable the dependent mutation; retain the source,
identifier, version, observation time, freshness, and authority needed for
graduation; and use named-role placeholders where a person is not yet known.
Placeholders never satisfy an authority-dependent exit.

### One workstation is not a WIP policy

The current single workstation does not require a distributed compare-and-swap
lease ledger and does not impose a one-producer architecture. Parallelism is
limited by disjoint write roots, one writer per branch/worktree, review and
integration capacity, test cost, and the current Product Increment topology.
Two source-producing lanes may proceed when those conditions are real. A
durable PostgreSQL-backed admission/lease broker becomes necessary before
multiple hosted runners or citizen sessions can contend for work; it is not a
prerequisite for the present workstation build.

### Delivery modes stay distinct

| Mode | Allowed outcome | Forbidden claim |
| --- | --- | --- |
| Fixture/local proof | Contracts, projections, adapters, UI, negative tests, proposal drafts | Authoritative external truth or live permission |
| Accelerated preview | Exact branch-scoped, synthetic-data preview with required local gates, review, budget, and kill switch | Candidate or release readiness |
| Candidate/dev | Exact-SHA PR and destination verification, immutable provenance, dev GitOps, Outcome Owner acceptance | Production promotion |
| Strict release | Complete security, operations, accessibility, rollback, SRA/CAB, support, and exact-artifact evidence | Automatic release; a named human still promotes |

## Starting Line

At the authoring baseline
`main@2de105e001ab8029eb21bce4691eded49e48ff81`, the nine FAB-A1 wire
contracts and their conformance checks are accepted source. A distinct Product
acceptance receipt remains unrecorded. Accepted main does not yet contain an
operational Console, registry service, authoritative projector/read API, live
source adapters, hosted control plane, orchestrator, factory runner service,
or repository bootstrap automation. Candidate controls, registry-spine, and
demo branches outside accepted main earn no completion credit until a current
Product Increment, exact branch vehicle, review, pipeline, and accepted
destination evidence say otherwise.

The live starting line must always be refreshed from the Roadmap, Product
Increment portfolio/plan, current PFC topology, and accepted evidence. This
dated paragraph is provenance for the estimates below, not a status ledger.

Use the following proof products:

- **CRM** is the first engineering reference and composition chain.
- The **Technology Strategy application** may be a truthful local/draft
  fixture only until its own evidence proves deployment; it is not a live
  second consumer.
- **PDS Nexus or another independently owned product** with a named owner,
  journey, outcome, and target model is the AF-M08.1 second consumer.
- A **third unlike product** is required before claiming an enterprise-default
  Fabric formation posture, even though the App Framework economic gate uses
  first/second-use comparison.

Old scaffolds and demonstrations are reference evidence, not implementation
bases. Port only individually reviewed assets onto current accepted contracts;
do not revive an old branch as a shortcut around current proof.

## Numbered Implementation Stages

The stage numbers are presentation groupings. `P0`–`P13` retain the estimate
sequence from the prior plan; AF-D and AF-M identifiers retain Roadmap meaning.
No row is a live Assignment or admission.

| Stage | Existing mapping | Outcome and exit proof | Primary owner roles | Entry/dependency | Effort |
| --- | --- | --- | --- | --- | --- |
| **0. Decide and register** | `P0` | Ratify the product charter, three scopes, authority model, two initial proof products, proposed targets, and stop rules; launch the amendment to the existing BOK major packet; register one Product Increment and validated lane plan with exact bases, write roots, owners, reviewers, budgets, and evidence | Fabric Product, Program Product Manager, Strategy authority, Chief Architect/EAC, Integration, PFC | Human direction and accepted main | 3–5 business days and 1–2 person-weeks for execution setup; 1–2 weeks and 2–4 person-weeks for the parallel decision campaign |
| **1. Controls and contract delta** | `P1`; AF-D08.1a; FAB-A2–A4 | Record Product acceptance for FAB-A1 or the remaining conditions; ratify RBAC/app/fleet roles, cross-app and tenant denial, privacy, threat delta, source/freshness/retention matrix, stop instrumentation, and decisions for the new product/BOK/pattern/value/data/intent references | Product, Architecture, Security, Privacy, Data, Finance, IdP, source-owner roles | Stage 0 records and admitted exact lanes | 2–3 weeks; 6–10 person-weeks |
| **2. Registry and claim-graph kernel** | `P2–P3`; AF-D08.1b | Separate audited decisions, append-only observations, and rebuildable projections; validate contracts; prove idempotency, replay, rebuild, tamper detection, stable provisional identity, versioned read envelopes, and one-checkout local adapter; no runtime dependency | Backend/data engineering, Architecture, Security | Frozen Stage 1 envelopes; fixture authorities are sufficient | 3–5 weeks; 8–13 person-weeks |
| **3. Hosted engineering evidence** | `P4–P5`; AF-D08.1b | CRM source/build/SBOM/provenance/artifact/GitOps/runtime chain; desired/built/deployed/running identity; hosted/local schema parity; workload and user identity; RBAC/field masks; `STALE`/`UNKNOWN`; source and management-plane outage isolation | Integration/platform, IdP, SRE, source-adapter owners | Stable ledger/read contracts; cluster/IdP work runs in parallel | 4–7 weeks; 11–18 person-weeks |
| **4. Operations Console v1** | `P6`; AF-D08.1c | World-class web views for executives, product owners, architects, and engineers; fleet and product/theme search; per-product/app workspace; exact composition/version graph; provenance/freshness; core cost/value state; responsive and WCAG 2.2 proof | Fabric Product, Product Design, frontend, accessibility, research | Accepted read boundary; two honest fixture/real consumers | 4–6 weeks; 12–18 person-weeks |
| **5. Strategy, value, patterns, and Intent Hub** | Unregistered proposed Deliverable; candidate alignment to AF-M08.1 and AF-M10.5 must be decided before source work | Exact BOK draft/release bindings; digital-product/app graph; Finance validation state; data-governance references; versioned pattern qualification and report-only conformance; one SaaS Integration Intent and one Distilled Model Intent projected from receiving systems without a parallel queue | Strategy, EAC, Finance, Data Governance, Integrations and Innovations product owners | Claim graph stable; fixtures and disabled action adapters allowed | 5–8 weeks; 14–24 person-weeks; may overlap Stage 4 on disjoint surfaces |
| **6. Concierge Phase 0 proof** | `P7`; completes bounded Phase 0 | Operate one exact context → proposal → Assignment → preview → review → PR → destination SHA → signed dev digest → acceptance chain manually with current rails; retain friction, elapsed time, whole cost, failures, and evidence gaps | Implementation owner, Integration, Outcome Owner, independent reviewer | Stages 3–4 accepted; existing delivery rails | About 2 weeks; 4–6 person-weeks |
| **7. Supervised proposal-to-preview factory** | `P8`; AF-D08.2a | Separate factory API/database; typed proposal-only MCP; deterministic admission; isolated ephemeral runners; mechanical write roots; independent review; signed synthetic preview; budgets and kill switch | Factory engineering, Security, PFC, reviewers | Stage 6 accepted; AF-OG02 package path graduated; identity review complete | 6–8 weeks; 20–28 person-weeks |
| **8. Candidate and dev delivery** | `P9`; AF-D08.2b | Candidate gates, exact-SHA PR/destination verification, immutable provenance, dev GitOps deployment, correlation, and Outcome Owner acceptance projection; accelerated evidence cannot masquerade as release evidence | Integration, platform/SRE, Outcome Owner | Stage 7 | 4–6 weeks; 12–18 person-weeks |
| **9. Mobile Release Center** | Read-only in `P6`; governed actions after `P9/P10`; AF-M10.2 | Exact signed store build and OTA identity, submission/review/track/staged rollout, live adoption/skew, crash health, halt/rollback, and support evidence; a narrow native companion handles attention, review, bounded human action, and deep links | Mobile Product/Platform and release authorities | Mobile seam/native-generator proof; source-owner access; strict path for production actions | 3–5 weeks; 8–14 person-weeks |
| **10. AI Portfolio Analyst** | New AF-M10.5-aligned Deliverable; register before source work | Deterministic completeness/quality/overlap checks followed by cited hypotheses, counterfactuals, alternatives, evidence gaps, confidence, and human disposition across at least three applications; no automatic investment or retirement | Portfolio Product, Strategy, Finance, EA, AI Governance | Accepted identity, dependency, cost, outcome, and capability completeness threshold | 4–6 weeks; 10–16 person-weeks |
| **11. Strict release** | `P10`; AF-D08.2c | Live-provider, security, operations, accessibility, resilience, rollback, SRA/CAB, support, and exact source-to-artifact evidence; named human promotes only the exact Accepted, certified digest | Security, SRE, release/SRA/CAB authorities | Stage 8; applicable live-environment work items and release authorities | 6–10 weeks; 20–30 person-weeks |
| **12. Reuse and economics graduation** | `P11–P12`; AF-D08.2d and AF-D08.3 | Extract only seams proved by two unlike products; compare fully loaded first/second-use economics and quality; require a human continue/narrow/stop decision; add a third unlike use before enterprise-default claims | Product, Finance, Architecture, independent product owners | Applicable candidate/release evidence and second real consumer | Existing second-use proof: 7–11 weeks; 20–32 person-weeks. Third use: roughly 3–5 weeks; 8–14 person-weeks |
| **13. Citizen and fleet scale** | `P13`; AF-D10.5a | Sponsor/classification intake, G-F13 quotas/dormancy, idempotent bootstrap/protections, budgets/WIP, adversarial authorization, support/retirement, upgrade campaigns, and progressive rollout | Fabric Product, Security, Platform/SRE, support, PFC | Stage 12 human continue decision and BOK citizen-production boundary | 8–12 weeks; 30–45 person-weeks |

### Stage-detail obligations

The table is intentionally integrated; the following rules prevent important
capabilities from becoming cosmetic panels:

- **Value and economics:** keep outcomes, adoption/experience,
  delivery/quality, and fully loaded economics separate. `estimated`,
  `product-validated`, and `Finance-recognized` are distinct states. No
  hand-entered ROI and no opaque aggregate app score.
- **Patterns:** a qualification references an exact approved EAC standard/ADR,
  version, applicability, prerequisites, security/data/continuity obligations,
  evidence, expiry, exception, owner, and compatible component versions. The
  Console may draft patterns; EAC approves meaning and exceptions.
- **Cross-team intents:** the Console owns requested context and correlation;
  the receiving team's system owns queue, priority, assignment, approval, and
  fulfillment. A prerequisite blocker and a capability request are not
  automatically the same record.
- **AI:** deterministic checks run first. Every AI recommendation carries
  citations, confidence, alternatives, counterfactuals, evidence gaps, and
  human disposition. It never becomes a funding, architecture, risk,
  consolidation, retirement, or release decision.
- **Mobile:** observing a mobile product's releases and building a native
  management companion are separate increments. Web remains the complete
  workbench until a time-sensitive native journey proves differentiated value.
- **Scale:** two uses prove App Framework reuse economics; independent second
  and third uses are required before presenting the common spine as an
  enterprise formation default.

## Critical Path And Safe Parallelism

The critical path is:

```text
Stage 0 -> Stage 1 -> Stage 2 -> Stage 3 -> Stage 4 -> Stage 6
        -> Stage 7 -> Stage 8 -> Stage 11 -> Stage 12 -> Stage 13
```

Stage 5 is not yet part of AF-D08.1c or AF-M10.5; Product and Roadmap authority
must register its exact home before source work. If registered, it can overlap
Stage 4 after the claim-graph contract freezes. Stage 9 read-only mobile
observation can begin with Stage 4/5, while governed release actions wait for
Stages 8/11. Stage 10 begins only after the required data-completeness
threshold; it must not rationalize a manually incomplete estate.

After the shared Stage 1 contract freeze, the recommended initial source lanes
are:

1. **Registry kernel:** ledgers, validation, projection, read envelope, and
   local adapter.
2. **Console experience:** discovery, information architecture, design-system
   composition, accessibility, and frontend proof against frozen fixtures.

A source-adapter lane can join when its envelopes and source authority are
stable. One producer owns each branch/worktree and write root. Shared contract,
roadmap, Product Increment, and integration seams remain single-owner.

## Calendar Expectations

These are planning ranges, not commitments. They assume a stable backlog,
prompt decisions, retained review capacity, and no fabricated external proof.

| Horizon | Credible outcome | Explicit non-claim |
| --- | --- | --- |
| **30 days** | A partial-stage read-only local alpha: Stage 0 decisions and Stage 1 controls are time-boxed, while Stage 2 fixture/read-envelope work and Stage 4 discovery run in parallel; demonstrate two truthful applications, core role views, and stale/unknown behavior without claiming any stage exit | Not a completed registry kernel, hosted authority, citizen production, or release automation |
| **90 days** | Beta with registry graph, world-class web Console, live engineering evidence where access exists, first BOK/value/pattern views, and one concierge accelerated loop | Not general citizen self-service or production release authority |
| **16–22 weeks after full activation** | Complete supervised CRM target when a stable 5–7-person cross-functional team and intended overlaps are real | Not an estimate for one serial producer |
| **15–23 weeks for bounded Phase 0** | Serial Stages 0–6 on one source producer, excluding unbounded external waits | One physical workstation is not the architectural reason for serial work |
| **7–11 months after accepted Phase 0** | Plan-derived serial range for Stages 7, 8, 11, 12, and 13; governed citizen self-service follows second-use proof, with third-use evidence required for enterprise-default claims | Not the end-to-end program duration and not activated until Stage 12 passes and human authorities decide |
| **11–16 months end-to-end** | Plan-derived serial baseline from Stage 0 through Stage 13, including bounded Phase 0 and post-Phase-0 factory/release/scale work; a staffed 5–7-person team may reforecast lower only from an explicit dependency/overlap schedule | Not a commitment and excludes unbounded external-authority lead time |

The end-to-end range is the sum of the serial critical-path planning ranges,
not an aspirational date. The 30-day and 90-day horizons are intentionally
partial-stage demonstrations that use frozen fixtures and permitted overlap;
they do not award milestone or release credit. At each accepted stage, Product
and Integration must reforecast from actual cycle time, staffing, review
throughput, and external-authority state.

## Proof Hypotheses To Ratify

These are proposed product targets, not BOK requirements or approved
enterprise commitments. Stage 0 must name an owner, baseline, denominator,
window, acceptance rule, and stop disposition for each target it retains.

| Hypothesis | Proposed evidence |
| --- | --- |
| Fast onboarding | Two unlike apps onboard in no more than one working day each |
| Trustworthy claims | At least 95% of operational claims show source and freshness; failure becomes `STALE`/`UNKNOWN` |
| Traceable software | Exact source SHA binds to SBOM, provenance, immutable artifact, desired deployment, and observed runtime |
| Usable management experience | Ten representative questions answered in under two minutes; purpose/health understood in five seconds; provenance/next action found in 30 seconds; WCAG 2.2 and manual assistive-technology proof |
| Honest economics | No hand-entered ROI; owner, baseline/counterfactual, formula, period, source, confidence, and Finance validation state are visible |
| Federated intent | One capability intent round-trips a receiving team's real work system without a side-channel backlog |
| Reuse economics | At least 50% lower normalized second-use effort with no outcome, security, accessibility, quality, reliability, recovery, maintainability, or support regression |
| Useful AI challenge | Blinded human review rates at least 70% of top hypotheses useful and harmful false positives below 20% |
| Mobile integrity | Every release view binds the exact signed artifact, human authority, staged state, and rollback/halt evidence |

## Parallel Authority And Ratification Campaign

These tracks start at Stage 0 and run beside implementation. A missed date
disables only the dependent claim/action unless the Stage 1 safety boundary
itself is unresolved.

| Target date | Accountable role | Required disposition | What continues if late | What remains blocked |
| --- | --- | --- | --- | --- |
| **2026-08-07** | Fabric Product Owner + Chief Architect | Confirm proposed Console name, three scopes, two proof products, read-only/proposal-first boundary, and Stage 0 hypotheses | Fixtures, docs, contract analysis, UX discovery | Any disputed boundary or product claim |
| **2026-08-07** | Program Product Manager + Integration + PFC | Register the Phase 0 Product Increment and exact initial lanes; record review/budget/capacity | Non-source research and design | Source-producing lane without admission |
| **2026-08-14** | Strategy content authority | Submit one amendment to the existing Application Fabric major packet with exact App Framework evidence | All reversible read/proposal work | Claim that Fabric is ratified enterprise policy |
| **2026-08-21** | EAC / Chief Architect | Name exact pattern/ADR authority and report-only conformance boundary | Fixture-backed pattern views | Enforcement or approved-pattern claims |
| **2026-08-21** | CMDB/APM, Data, Analytics, Finance, IdP, Security, Mobile source-owner roles | Name authority, access path, freshness, privacy/retention, test fixture, and write eligibility per fact class | Adapter contracts and `UNKNOWN` fixtures | Corresponding authoritative claim or write |
| **2026-08-28** | Strategy/CIDO decision authority | Decide, defer, or condition the Application Fabric strategic-direction packet | Product proof under recorded conditions | Enterprise-default or mandatory-spine claim |
| **Before Stage 7** | Security + IdP | Approve conversation/factory client identity and delegated authorization | Stages 0–6 | Hosted proposal-to-preview factory |
| **Before Stage 11** | Release, SRA/CAB, Security, SRE, support authorities | Name exact release, risk, recovery, and support evidence | Candidate/dev proof | Production promotion |
| **Before Stage 13** | Citizen-production decision authority + Product | Decide supported-production boundary and scale funding | Concierge/supervised operation | General citizen supported-production self-service |

Replace role placeholders with named people in the live Product Increment plan
as soon as assignments are known. This table is not an omnibus approval ledger.

## Human Decisions And Required Actions

1. **Product and Architecture:** accept, condition, or reject the proposed
   Console product boundary, three scopes, and initial proof products.
2. **Program Product and Integration:** create the live Phase 0 Product
   Increment/lane topology before source implementation; the master plan does
   not admit it.
3. **Strategy authority:** amend the existing Application Fabric packet and
   later record the exact governing release or deferral.
4. **Domain owners:** provide source contracts and named owners in parallel;
   they do not need to be complete before fixture-backed local work starts.
5. **Product:** name the independently owned second real consumer and its
   owner, journey, outcome, target model, and environment before AF-M08.1 exit.
6. **Finance:** define recognized-value and fully loaded cost validation before
   publishing ROI as more than an estimate.
7. **EAC:** define approved pattern/exception authority before conformance can
   become enforcement.
8. **Human release/risk authorities:** retain every candidate, promotion,
   SRA/CAB, accepted-risk, mobile submit, and production decision.

## Stop And Narrow Rules

- Stop authoritative adapters if identity, authorization, source authority,
  freshness, retention/privacy, idempotency, reconciliation, tamper, replay,
  or cross-app denial cannot be proven. Continue fixtures where safe.
- Stop a claim, never hide uncertainty, when its source is unavailable or
  expired. `UNKNOWN` and `STALE` are valid product states.
- Stop if the Console becomes a manually maintained shadow CMDB/APM,
  strategy/portfolio/backlog, Finance, data-governance, continuity, workflow,
  or approval system.
- Stop if a management/factory outage affects product runtime or business
  traffic.
- Stop world-class or executive-readiness claims until representative tasks,
  accessibility, responsive, performance, comprehension, comparison, trust,
  and recovery evidence pass.
- Stop factory expansion if chat/model output becomes authority, runners escape
  write roots, review/support capacity fails, or accelerated proof is presented
  as release readiness.
- Stop or narrow native development if no time-sensitive journey materially
  outperforms responsive web plus notifications and deep links.
- Stop AI rationalization until identity/dependency/cost/outcome evidence is
  sufficiently complete; stop it again if harmful false positives exceed the
  ratified limit.
- If second-use economics fail, retain useful registry/workspace capabilities
  but do not claim or scale a citizen factory. If third use fails, do not claim
  an enterprise formation default.
- Tokens, prompt counts, catalog size, builds, deployments, and scorecard points
  are never business value by themselves.

## Immediate Next Actions

1. Human Product and Architecture review this master boundary and the linked
   assessment.
2. Register a single Phase 0 Product Increment and validated lane plan on
   current accepted main; record Product acceptance disposition for FAB-A1.
3. Admit the Stage 1 controls/contract-delta lane and begin Console discovery
   against versioned fixtures in a disjoint design lane.
4. File the BOK amendment and external source-owner requests in parallel; do
   not wait for them to begin reversible implementation.
5. Freeze the Stage 1 read/claim envelopes, then run registry-kernel and
   Console-experience work in parallel when review/integration capacity is
   available.
6. Demonstrate the 30-day truthful read-only alpha; use its evidence to
   re-estimate and admit the next lanes.

## Verification Contract

Changes to this master plan and its routing use the framework docs/governance
proof loop:

```bash
git diff --check
scripts/appfw framework docs-check --changed-only --json
scripts/appfw framework validate --json
scripts/appfw framework generate --check --json
scripts/appfw framework test --fast --json
scripts/appfw framework governance-check --json
scripts/appfw framework handoff --json
scripts/appfw framework review-brief --auto-depth --json
```

`governance-check` is report-only unless invoked with enforcement. Missing
managed provenance, signing, release identity, PDS baseline, PHI-pipeline, or
other release-authority evidence is an inherited external release gap for a
documentation branch, not a pass and not a regression. Handoff and PR evidence
must state the exact disposition; this plan makes no release claim.

Any source-producing Product Increment created from this plan must name its
controlling spec or intent note, exact accepted base, write roots, owner,
reviewer, proof, stop rule, and acceptance authority.

## Role Card Check

- Producing card: Architect Agent, under the human-directed request to preserve
  the reviewed vision and create the master implementation plan.
- Advisory cards used: Framework Research Steward, Product Owner/Strategist,
  PDS Strategy Content Steward, and Framework Structure Steward.
- Work stayed within role: the Architect Agent integrated the implementation
  sequence, effort, proof strategy, and focused docs changes; advisory roles
  supplied evidence, product challenge, BOK boundaries, and docs ownership.
- Authority not assumed: no roadmap posture, Product acceptance, architecture,
  BOK, funding, admission, branch, source, merge, release, Finance, risk, or
  production decision is made by this plan.
- Routed decisions: product scope to Product; strategy to CIDO/Strategy;
  patterns to EAC; economics to Finance; source facts to their owners; live
  sequencing/admission to Product Increment/PFC; release/risk to named humans.
- Drift signals: a second plan, stale status prose, universal-portal language,
  workstation-based WIP, proposed targets presented as ratified, or a projected
  fact presented without its authority and freshness.
