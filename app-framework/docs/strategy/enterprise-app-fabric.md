# Enterprise App Fabric Strategy

> **Status: strategy decision basis, not an executable backlog.** This document
> defines the App Fabric thesis: PDS keeps its SaaS pillars, ServiceNow is the
> main system of action, and the strategic target is a coherent PDS-owned
> enterprise experience built through App Framework's **application-production
> system and channel-neutral capability layer**. Employee Slate/Moveworks is
> the strongest comparator and a credible fallback, not the default target.
> This document extends the
> [North Star](product-development-north-star.md) (commitment 3: PDS capability
> contracts stay portable), the
> [Platform Strategy](app-framework-platform-strategy.md) ownership boundary,
> and the [PDS Nexus Brief](../product/pds-nexus-product-development-brief.md).
> Competitive, licensing, and API-feasibility claims below were produced by
> adversarially verified research refreshed **2026-07-14**. The management-plane
> product and industry boundary was challenged again on **2026-08-03** in the
> [App Fabric Industry And BOK
> Vetting](../assessments/app-fabric-industry-and-bok-vetting-2026-08-03.md).
> Pricing, licensing, vendor-roadmap, and draft-strategy claims are dated
> snapshots that **must be re-verified at negotiation or decision time**, not
> treated as durable facts. Any fabric item
> becomes executable only when the Product Owner turns it into a roadmap
> Deliverable under a Product Increment, with owner, journey, metric, risk, and
> evidence gate, and the Program Flow Controller admits an Assignment.

The fabric thesis is a focused expression of the North Star's
[Strategic Goal Statement](product-development-north-star.md#strategic-goal-statement),
especially the goals to enable Nexus/workflow transformation, create governed
integration, make intelligent apps real, and protect the enterprise while
moving faster.

## Research Artifact Disposition

This document is the source-controlled synthesis of the market and industry
analysis used to shape the App Fabric thesis. Raw research transcripts,
assistant outputs, pricing notes, vendor-roadmap notes, or negotiation-sensitive
snippets are supporting inputs, not durable strategy docs.

Handle future market-analysis artifacts this way:

1. **Curate into strategy.** Promote only reviewed conclusions, dated claims,
   decision boundaries, and action implications into this document or the
   linked strategy docs.
2. **Re-verify volatile facts.** Pricing, licensing, vendor-roadmap, API, and
   acquisition claims must be re-verified before negotiation, funding, or
   implementation decisions.
3. **Convert actions into product work.** Anything executable must move through
   the Product Owner into roadmap/backlog form with owner, journey, metric,
   risk, and evidence gate.
4. **Keep raw artifacts out of source by default.** Retain raw research locally,
   in approved evidence storage, or in generated `target/appfw` artifacts only
   when useful for review. Do not make raw assistant output the source of truth.
5. **Feed freshness review.** Material changes from new research should trigger
   a Strategic Pull Review or strategic-freshness review before changing the
   North Star, Product Management Strategy, Roadmap, or XO lane queue.

## Thesis

PDS is not replacing its SaaS pillars. ServiceNow is the system of action and
runs the workflows that belong on it. Workday remains the People system of
record, Salesforce the legal/contract system of record, Epic the clinical
platform.

The App Fabric is the PDS-owned application and experience layer that provides:

- **a coherent enterprise experience** across SaaS and internal systems for
  major outcome and journey themes, rendered primarily in PDS web/native-mobile
  products while preserving approved vendor-surface adapters;
- **adopted object models**: vendor schemas ingested verbatim into governed,
  generated, drift-tracked model contracts;
- **data governance**: policy, classification, provenance, freshness, and
  audit over every projection and write path;
- **governed connectors** with evidence-certified capability tiers;
- **common theming** through the PDS design system;
- **built-in intelligence**: grounded, policy-trimmed AI surfaces over fabric
  data, exposed to any enterprise front door; and
- **governance accelerators** (release evidence, SRA/CAB packages, audit
  posture) consumable by any PDS application team, including teams building
  natively on vendor platforms.

Engines and vendor surfaces can change. PDS-owned product experience,
application archetypes, object-model contracts, operations, events,
intelligence envelopes, evidence, and observability must remain portable
across them. The fabric exists so vendor change costs PDS a bounded adapter or
contract change instead of another experience rebuild or permanent lock-in.

The business test is not whether the fabric is technically possible. It is
whether a funded journey can show lower cycle time, less status chasing,
faster approval, reduced rebuild risk, or clearer governance than a native
vendor-only experience.

## Application Management Plane

The App Fabric also needs an internal management plane for the applications
PDS owns, observes, or helps produce. Its dashboard is the human experience for
that plane; it is not the whole App Fabric, the runtime path for product
traffic, or a new universal source of enterprise truth.

The proposed product name for that experience is **App Fabric Operations
Console**. The name is an App Framework product recommendation, not ratified
PDS Technology Strategy terminology. The Enterprise App Fabric is the wider
capability and lifecycle system; the Console is its role-aware management
workbench; and the Fabric Registry and Claim Graph is the source-bound read
model beneath the workbench.

Use three explicit scopes so visibility never implies control:

1. **Federated estate visibility** for searchable, source-bound application
   information across the broader enterprise estate.
2. **Fabric-managed lifecycle** for qualified PDS-owned applications that
   adopt lifecycle, evidence, compatibility, component, and fleet contracts.
3. **Factory-managed change** for applications explicitly admitted to the
   Hosted Product Factory's typed and governed delivery loop.

Digital products and applications are separate. A product may span web,
native mobile, services, APIs, data products, agents, MCP servers, and several
deployable applications. Applications enable value; the product, journey,
solution, and outcome own the value hypothesis.

The target has four connected surfaces:

| Surface | Purpose | Authority boundary |
| --- | --- | --- |
| Fleet registry | Search applications by stable identity, purpose, owner, lifecycle state, and management posture | References approved enterprise and repository sources; does not silently duplicate them |
| Per-application workspace | Project product intent, work, evidence, composition, environments, usage, satisfaction, and cost with source and freshness | App teams may maintain product context and make typed proposals within their application; viewing or owning an app does not grant merge, release, risk, secret, or cross-app authority |
| Hosted Product Factory | Turn sponsored intent and admitted proposals into bounded assignments, branches, reviews, previews, and release evidence | Reasoning proposes; deterministic rules, independent review, CI, and named humans dispose |
| Product/runtime plane | Run the application and its governed capability contracts in dev and production environments | Product traffic and business data remain isolated from management-plane failure and storage |

All applications may be registered, but registration alone does not make an
application controllable. Management tiers are cumulative capability grants,
not maturity, release-readiness, security, or risk ratings:

- **Registered:** stable identity, purpose summary, accountable owner,
  canonical references, and lifecycle state.
- **Observed:** read-only adapters add build, deployment, usage, satisfaction,
  cost, and component-version observations with provenance and freshness.
- **App Framework managed:** the application participates in App Framework
  lifecycle, compatibility, upgrade, evidence, and composition contracts.
- **Factory managed:** authorized users can submit typed change proposals to
  the Hosted Product Factory's governed implementation loop.

For fabric-born applications the registry is the system of origin: ideas
register with a repository and dev presence before any enterprise CMDB
record exists, and matriculate into the CMDB at the planned (sponsorship)
decision, after which the CMDB reference is authoritative for enterprise
operations. Pre-existing applications register by reference to their
existing CMDB record. Applications also participate in named portfolio
themes — journeys, processes, and experiences — so the fleet shares purpose
lineage as well as versioned framework components.

Release certification remains a claim about an exact source and immutable
artifact for a particular environment, never an application tier. The
management plane should federate facts from their authorities and build a
searchable read model; it must not become a manually maintained CMDB, a
parallel backlog, an APM replacement, or a token-usage leaderboard.

The 2026-08-03 review confirms that catalogs, scorecards, templates,
self-service actions, rationalization, and AI query are established industry
categories. PDS differentiation is the inspectable chain from strategy and
product purpose through exact artifact/deployment evidence to cost, outcome,
and human lifecycle decision. Every material claim must retain source,
identifier, version, observation time, freshness, confidence or validation
state, and a deep link. AI may challenge assumptions and recommend; it never
approves investment, architecture, risk, release, consolidation, retirement,
or recognized value.

The reviewed Technology Strategy snapshot is draft and unratified. Reversible
read-only and proposal-first product work may proceed while the existing
Application Fabric major decision packet is amended, but the Console must not
claim enterprise-policy, EAC, Finance, data, continuity, governance, citizen
production, or mobile-strategy authority before the corresponding decision.
The [Master Implementation
Plan](../release/app-fabric-master-implementation-plan.md) owns the integrated
sequence and acceleration boundary; this strategy continues to own why and
scope.

## Context and drivers (July 2026)

- **Pega is deprecated** by CIDO decision and expected to be cancelled just
  over a year from July 2026. Nine workloads must move, presumably mostly to
  ServiceNow as the system of action.
- **ServiceNow's experience layer churns on a vendor-controlled cadence.**
  Employee Slate went GA on 2026-05-05 on a new stack, bundled inside
  EmployeeWorks AI SKUs, with Employee Center Pro parity targeted "by 2027"
  and new innovation landing predominantly on Slate. No formal EC Pro
  deprecation exists as of 2026-07-07, but PDS Connect 3 (built on EC Pro)
  needs an exit ramp regardless, and a prior Slate-style MVP already failed at
  PDS. Care Team Portal inside Healthcare Operations is a held proof candidate;
  its entitlement, fit, and necessity for PDS remain unverified and create no
  current roadmap dependency.
- **Vendor EX layers are portfolio line items.** Microsoft retired Viva Topics
  (2025-02-22) and Viva Goals (2025-12-31) within roughly four years of
  launch. ServiceNow is on its fifth UX stack in twelve years.
- **The generic assistant and enterprise-search race is a vendor war PDS
  should not recreate.**
  ServiceNow (Moveworks, acquisition closed 2025-12-15), Microsoft (Copilot),
  and Workday (Sana) all claim the cross-SaaS entry point. The fabric owns
  journeys, adopted models, and the governed action surface underneath, and
  exposes them to any front door through focused APIs and the answer-envelope
  contract; MCP and A2A remain optional adapters.

## Evidence-calibrated foundation and kappa boundary

App Framework already has a strong application and database foundation. The
current provider-graduation report covers 13 providers, 105 graduated semantic
areas, and zero promotion violations. MongoDB is not an aspirational connector:
15 semantic areas are live-certified, including filters, projections,
relationships, aggregation, access filters, tenant isolation, concurrency,
error normalization, and audit. Entity/relationship modeling, full-stack
generation, generated API tests, forward-only migrations, provider-neutral
semantics, source-controlled delivery, deep UX, and deployment portability are
present-tense advantages to package and reuse.

Do not collapse that maturity into an end-to-end kappa claim. App Framework can
read mature MongoDB application projections, and CDC/actor/tenant/retry/dead-
letter/idempotency contracts exist, but there is no production Kafka broker
client, durable checkpoint, certified projection loop, or retained replay, lag,
coverage, reconciliation, disaster-recovery, and support proof. Every funded
archetype must therefore declare three distinct paths:

1. **Live read:** source API, latency/limits, authorization, and fallback.
2. **Projection read:** MongoDB collection, identity, lineage, freshness SLO,
   stream coverage, tombstones, authorization, and reconciliation.
3. **Governed action:** source write API, delegated actor, policy, approval,
   idempotency, audit, and confirmation back through the stream.

### Tenant boundary

App Framework has strong tenant-isolated execution today: authenticated tenant
context composes centrally with policy and provider filters, fails closed when
missing, is covered by same-tenant and cross-tenant database certification, and
propagates into audit, MCP, SaaS planning, and Kafka contracts. It does not yet
have a complete managed tenant control plane for provisioning/offboarding,
configuration and secret partitioning, quotas/noisy-neighbor controls,
tenant-aware migrations, residency, per-tenant backup/restore, cost allocation,
and support.

Define `tenant` as an independent security and operational isolation boundary.
The 1,100 PDS locations, markets, departments, and legal entities are normally
policy dimensions inside the PDS boundary. Create a separate tenant only when
independent administration, secrets, retention, encryption, deployment,
residency, restore, or service boundaries require it. The 45-65 blended
person-month planning range assumes one PDS enterprise boundary; use 55-80 only
for an intentionally general SaaS-grade customer tenant control plane.

### Permission-preserving projection boundary

Source permissions do not automatically survive replication into MongoDB,
Kafka, caches, exports, mobile storage, or AI context. The first stream-fed
archetype must therefore carry authoritative subject/object identifiers,
entitlement provenance, policy and schema version, dynamic field masks, allowed
actions and obligations, decision/revocation timestamps, and a permission
freshness SLO. It must handle tombstones, reassignment, hierarchy/group change,
and stale or missing entitlement state fail-closed, then reconcile positive and
negative decisions against the source across every consuming channel.

Moveworks launch rules govern plugin availability, not the underlying record or
action. Source/delegated authorization remains mandatory. Ambient, scheduled,
and event agents require bounded service identity and explicit on-behalf-of
context where human authority is required. Consume Moveworks/PDS Health AI for
permission-aware search and use source-native or proven ReBAC services for
high-cardinality object ACLs; App Framework Rego remains the contextual
application policy and obligation engine.

## What the fabric is not

- Not a replacement for any SaaS pillar, and not a competing workflow engine.
  A native durable-orchestration capability remains a tracked hedge, triggered
  only by a repeated, evidenced product need ServiceNow cannot satisfy (see
  the Platform Strategy boundary).
- Not a fourth chat front door.
- Not an enterprise canonical data model or MDM program (see Object Model
  Adoption below).
- Not a broad employee "unified portal." A governed internal application
  management portal is in scope for operating the fabric, but it is not an
  employee front door or a reason to wrap every application.
- Not a mandate to wrap every SaaS screen. Native vendor experiences remain
  correct for specialist, low-frequency, administrative, or configuration work.

## Selection rubric: when a journey or experience gets layered

Layer when most of these hold:

1. The journey crosses two or more systems.
2. The tasks are named and high-frequency (evidence from usage/process data,
   not intuition).
3. The persona is field/office staff needing PDS-quality web and mobile UX.
4. The underlying vendor UX carries churn or deprecation risk.
5. The actionability tier per step is known and priced (see below).
6. **A named business owner funds the journey.** Category history shows
   ownership/funding collapse kills these layers more often than technology
   (Liberty Mutual wound down Workgrid in 2025 despite strong employee
   adoption; analyst consensus locates failure in governance and ownership).

Do not layer: admin/configuration surfaces, deep specialist tooling (Epic
clinical, Workday compensation planning), or low-frequency long-tail screens.

Defer or retire a layer when the named owner disappears, the success metric
does not move, the journey proves to be single-system/native-UX adequate, the
licensing or audit posture cannot be resolved in writing, or the required
freshness/actionability tier is too weak to change user behavior.

## The honest product promise: tiered actionability

Verified per-system findings (as of 2026-07-07): unified task/journey
experiences are feasible and commercially precedented, but only as per-source
adapters with polling-first freshness. The truthful fabric promise is:

> **See everything. Approve most things in place. One governed click into the
> rest.**

Every fabric roadmap candidate must state the tier for each step before build:
`api_actionable`, `api_readable`, `deep_link_only`, `handoff_only`, or
`out_of_scope`. A mixed-tier journey can still be valuable, but the UX must
show freshness, source, and handoff boundaries honestly.

| System | Tier | Basis (verified 2026-07-07) |
| --- | --- | --- |
| ServiceNow | API-actionable | Approvals readable and actionable via the standard Table API (`sysapproval_approver`) with per-user OAuth; production-precedented. |
| Salesforce | API-actionable | `/process/approvals` REST resource (since API v30.0, 2014); actor must be an assigned approver; plan for Classic-to-Flow approval migration as a second surface. |
| Workday | Conditionally actionable | `common/v1` `inboxTasks` supports approve/deny **only** for approval-step items, scoped to the authenticated user, after per-business-process security configuration. To-Dos, questionnaires, and document reviews are not API-actionable. No native outbound webhooks; polling or notify-then-fetch. |
| iCIMS | Purpose-built handoff | Onboard portal tasks support third-party completion via redirect + `returnUrl` (HTTP 303 contract). |
| Cornerstone | Read + deep-link | Transcript/task APIs for assigned training; SSO deep links for completion. |
| Epic | Out of connector scope | No SCIM, no external task surface, Practitioner writes disabled by default. Route Epic provisioning through ServiceNow catalog tasks (fully API-actionable), keeping Epic in the journey without an Epic connector. |

Freshness is polling-dominant across the estate; tasks become another fabric
projection with per-source "as of" freshness surfaced in the UX, on the same
kappa/CDC posture as other projections.

**Open governance decision (before build):** Workday actor attribution —
per-user OAuth (consent and token lifecycle for the workforce) versus
integration-service-user attribution (which internal audit may veto for HR
approvals). Resolve with internal audit first; this gates the Workday tier.

## Licensing posture (verified 2026-07-07; contract-specific items flagged)

Net effect when a PDS-owned UX exception is justified, per vendor:

| Vendor | Net licensing effect | Notes and open items |
| --- | --- | --- |
| ServiceNow | **Cost-neutral; one savings lever** | Licensing is interface-agnostic: fulfiller work through custom UX carries the same subscription as native UI. The **Requester carve-out is the fabric's free lane** (users submitting/tracking their own requests and consuming knowledge need no subscription). HRSD/EC Pro run on the Unrestricted User model, so replacing EC Pro rendering is approximately user-count-neutral and **declining the EC Pro/Slate SKU is the one concrete savings lever**. **Approvals through custom surfaces are a contested gray zone** (modern contracts trend toward Approver/Business Stakeholder entitlements regardless of channel) — settle in writing. AI actions via API consume Assists; net-new custom AI skills may require the Prime tier. |
| Workday | **Cost-neutral** | FSE headcount pricing is channel-blind; public APIs are included; employee-facing custom UX fits Authorized Parties. Threat: an Extend upsell asserted against an external app that does not need Extend — obtain written confirmation at renewal. |
| Salesforce | **Cost-neutral to modest save** | Every fabric human still needs a seat (indirect-access clauses), but fabric-only light users can move from full CRM to Platform seats if permission sets are stripped to match. Use Integration User licenses for pipelines. |
| Epic | **Small add** | Connection Hub fee plus integration engineering; SMART-on-FHIR companion UX is vendor-sanctioned. Provisioning is the weak link — routed via ServiceNow (above). |

**The rule from the indirect-access era (SAP v. Diageo, 2017):** courts
enforce plain contract text against architectures the vendor never imagined,
and vendors have since productized rather than prohibited indirect access.
Therefore the fabric must be **contractually explicit, not architecturally
clever**. Per-user delegated auth — already the framework's governed-write
design — is the compliant pattern at every vendor. Never route human work
through service accounts to shrink license counts; integration-only flags are
not a loophole.

**Framing for finance:** the fabric is not a licensing-arbitrage program. Its
economic defense must come from application/archetype reuse, Pega and citizen
productization, app retirement, avoided services, portability, and measurable
journey outcomes. Do not assume Employee Slate/Moveworks avoidance: actual SKU,
coverage, and quote evidence must drive that comparison.

### ServiceNow order-form asks (time-bound)

The Pega-migration purchase will be quoted in the Foundation/Advanced/Prime
packaging introduced 2026-04-09 (legacy SKUs end-of-sale 2026-07-01). Write
into the order form:

1. **Customer Application clause**: PDS-built external applications may access
   the subscription service via API on behalf of appropriately subscribed
   users at no additional fee; such access is not "making available to third
   parties" or "circumvention."
2. **Channel-agnostic Requester carve-out** for submitting/tracking one's own
   requests and knowledge consumption through vendor or PDS surfaces.
3. **Approval licensing defined**: treatment and rates for approvals executed
   from custom surfaces (Business Stakeholder scope), in writing.
4. **Custom-table definitions and overage rates fixed** at signing.
5. **EC Pro to Slate transition terms** that do not strand PDS Connect 3, and
   written UX-roadmap commitments (EC Pro support horizon, Slate parity
   dates, App Engine Studio succession).

At the next Workday renewal: written confirmation that external PDS
applications on public APIs under delegated user auth require no Extend
license. At the next Salesforce renewal: define indirect usage concretely and
pre-agree Platform-seat classification for fabric-only users.

## Object Model Adoption (OMA): the essential capability

The fabric's enabling capability is adopting the object models of the systems
it layers over. The proven architecture (per EAI/SOA/MDM history and current
federation practice):

- **Adopt each vendor's model verbatim** behind a typed, version-pinned,
  generated boundary (connector crate / subgraph). Do not translate into a
  shared schema at the adoption boundary.
- **Compose only at the journey/theme level** via entity joins on a small
  governed identity spine (worker ID, email, NPI).
- **Never build the enterprise canonical model.** Canonical-model unification
  is the historical graveyard (Gartner's traceable 2012 record: only about a
  third of MDM programs demonstrate value); federated composition is the
  consensus pattern, with governed-object-layer products (e.g., Palantir
  Foundry's ontology) as the production existence proof. The fabric is a
  self-owned, narrower, cheaper version of that pattern.

Per-vendor adoption reality (2026-07-07): Salesforce (describe metadata) and
Workday (WWS XSD) support mature schema-driven codegen with multi-year API
version support; Epic FHIR/US Core codegen is mature but instance-behavior
dominated; **ServiceNow is the outlier** — no vendor-supplied per-table
schema, so adoption requires a customer-built `sys_dictionary` ingestion
pipeline. **The Unit A authenticated dictionary export is the direct input to
ServiceNow model adoption**, not merely connector evidence.

**Drift defense:** vendor release cadences are not the dominant risk (all
major vendors pin API versions per request with multi-year support). The
dominant drift vector is **tenant-level configuration change** — custom
fields, dictionary edits, renamed report shapes — which follows no calendar.
Defense: continuous schema-snapshot diffing plus consumer-contract tests on
only the fields the generator consumes, retained as evidence artifacts in the
existing docs-check/release-evidence idiom.

**Workflow artifacts are part of the model.** Adoption must ingest and
drift-track vendor workflow surfaces, not just object schemas: Workday
Business Process definitions and report shapes, Salesforce Classic-vs-Flow
approval surfaces, ServiceNow approval-table extensions.

**Capability-tier certification.** The connector certification ladder extends
to certify each operation's tier — `api_actionable`, `api_readable`,
`deep_link_only` — so the tiered product promise is machine-checked contract
truth, not marketing.

The first OMA slice should be narrow: one ServiceNow authenticated dictionary
export, one workflow/approval artifact surface, one generated model proposal,
one consumer-contract/drift check, and one Team Member Journey step that uses
the adopted model. Do not start by building a multi-vendor supergraph.

### Framework readiness and gap shape

Existing machinery that OMA builds on: the product-intake pipeline
(`analyze` → `propose-model` → `model-status` → `scaffold-model`, with
`--source-kind poc|legacy`), provider `ApiSnapshotMetadata` and
information-package version pinning (the snapshot-pin pattern already exists
in every SaaS provider crate), kappa/CDC read-side projections, and the
generated UI contract/view registry.

The gap is a focused capability build, not a re-architecture:

- a third intake source-kind (`saas-export`) ingesting dictionary/describe/
  XSD/FHIR exports into model proposals;
- an **externally-owned / projection-only** entity marking in the model
  system, carrying a sync contract (source, freshness SLO, provenance,
  write-back tier);
- vendor-schema drift subchecks (snapshot diff + consumer-contract tests) in
  docs-check/release evidence; and
- tier fields in the connector certification ladder.

## Evidence-based cautions

- Broad unified layers fail; the successes are narrow, journey-scoped,
  write-capable, and measured. Define per-journey success metrics
  (time-to-task, cycle time, adoption) before build.
- Failure causes concentrate in governance and ownership, not technology.
  The named-owner-and-funding rule above is binding.
- No published case federates Salesforce + Workday + ServiceNow + Epic into a
  single owned supergraph/UX fabric. Every ingredient is individually proven;
  the composition makes PDS an early mover, so integration governance is the
  risk to manage — the framework's strongest discipline.
- Digital adoption platforms (WalkMe, Pendo) are a cheap complementary bridge
  during the Pega migration window for spans PDS will not rebuild; they are
  not the continuity strategy.
- Claims in this document dated 2026-07-07 decay. Re-verify licensing,
  vendor-roadmap, and API-capability claims at each negotiation or build
  decision; retain the re-verification as evidence.

## Execution alignment

The [Roadmap Outcome Goal Register](../release/roadmap.md#outcome-goal-register)
is the sole source for current sequencing. This strategy does not maintain a
second producer order. The [App Fabric Master Implementation
Plan](../release/app-fabric-master-implementation-plan.md) is the single
subordinate narrative source for integrated stages, effort, dependencies,
decision gates, and proof outcomes; it cannot change roadmap posture,
represent live work, or admit an Assignment. Current state, lanes, and branch
topology must instead be projected from the Roadmap, canonical Product
Increment portfolio, a validated Product Increment plan, PFC topology, and
accepted evidence. The active
product-neutral readiness goal must close before a product-specific vertical
is activated; the bounded CRM signature experience may continue only on
disjoint surfaces and earns no production front-door claim.

When the PDS employee product is explicitly activated, it is the real external
package consumer that pulls one authoritative runtime spine, one permissioned
ServiceNow read, one permission-preserving MongoDB projection, one governed
action, one managed operating profile, exact web/native-mobile continuation,
and one structured PDS Health AI result through a single measured vertical.
ServiceNow retains workflow and fulfillment authority. Employee
Slate/Moveworks remains the strongest comparator and fallback. Kafka, broader
provider breadth, generic components, and new contract families wait for a
named need in that product.

## Decision rule

Before funding a fabric journey or capability, answer:

1. Which journey theme and named business owner funds it?
2. What is the actionability tier and license class of each step, in writing?
3. Which object models (and workflow artifacts) must be adopted, and is the
   drift defense in place?
4. What metric proves continuity value (time-to-task, cycle time, adoption,
   avoided rework) — and what evidence retires the claim if it fails?
5. What will App Framework own, what will the SaaS/workflow/data platform own,
   and what native vendor experience will remain untouched?

## Revisit triggers

- ServiceNow formally announces EC Pro end-of-sale/support dates, or Slate
  licensing changes materially.
- A vendor asserts license or ToS objections to the fabric pattern despite
  the contract language above.
- The Workday attribution decision blocks the approvals tier.
- A repeated, evidenced product need that ServiceNow cannot satisfy triggers
  the native-orchestration hedge (Platform Strategy boundary).
- Re-verification finds material drift in any load-bearing claim above.
