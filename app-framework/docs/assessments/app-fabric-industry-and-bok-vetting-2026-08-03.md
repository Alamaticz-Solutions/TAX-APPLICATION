# App Fabric Industry And BOK Vetting — 2026-08-03

> **Status: dated research evidence, not policy or execution authority.** This
> assessment preserves the reviewed August 2026 industry and PDS Technology
> Strategy analysis that informed the
> [Enterprise App Fabric Strategy](../strategy/enterprise-app-fabric.md) and
> [App Fabric Master Implementation
> Plan](../release/app-fabric-master-implementation-plan.md). It does not amend
> or ratify the PDS Technology Strategy, approve architecture, assign funding
> or risk, change roadmap posture, admit work, or authorize production. Product,
> Strategy, EAC, Finance, governance, security, release, and source-system
> authorities retain their existing decisions.

## Research Boundary

- Research date: 2026-08-03.
- App Framework accepted baseline examined:
  `main@2de105e001ab8029eb21bce4691eded49e48ff81`.
- PDS Technology Strategy authoring snapshot examined:
  `046e532c0e41b6911171f3d9aa961c0898d66140`.
- Governing BOK draft examined: `doc.enterprise_technology_strategy_2026_2030`
  version `2026-07-31-draft.3` and machine contract
  `contract.application` version `2026-07-31-draft.3`.
- Application Fabric companion examined:
  `doc.application_fabric_strategic_direction` version
  `2026-08-01-draft.2`.
- Prototype evidence examined:
  `feature/fabric-console-demo-cameo@d3fb6f647ec9d5db1da233974901c2eb5542ab9e`.

The BOK snapshot was clean and machine-valid when examined, but it had no exact
release tag and reported `ratified:false`. It is authoring evidence, not a
published strategy release. The BOK's retained App Framework evidence was also
bound to an older App Framework revision; this assessment does not imply that
the BOK has reviewed the accepted baseline named above.

## Research Verdict

**GO WITH CONDITIONS.** Continue the vision as the proposed **App Fabric
Operations Console**: a role-aware digital-product management experience over
the Enterprise App Fabric. It should become the visible place to understand,
question, propose, orchestrate, and evidence application-lifecycle decisions.

Use three explicit scopes:

1. **Federated estate visibility:** searchable, source-bound visibility for
   the broader enterprise application estate.
2. **Fabric-managed lifecycle:** deeper lifecycle, component, evidence,
   compatibility, and fleet management for explicitly qualified PDS-owned
   applications.
3. **Factory-managed change:** typed proposals and governed delivery only for
   applications admitted to the Hosted Product Factory.

The Console name and these scopes are App Framework product recommendations,
not ratified BOK terminology. Registration or visibility never implies Fabric
qualification, supported-production approval, or migration to App Framework.

A reversible, read-only and proposal-first implementation can proceed now.
Ratification and the relevant source-owner approvals are required before the
Console asserts cross-domain authority, enforces enterprise rules, writes
external systems, or enables general citizen supported-production promotion.

## Business Value

Catalogs, scorecards, templates, self-service actions, rationalization, and
AI-assisted query are established product categories. The differentiated PDS
capability is a trustworthy, inspectable chain:

```text
strategy -> product theme -> digital product -> application
         -> component/service/data/agent -> artifact -> deployment
         -> usage/cost/outcome -> lifecycle decision
```

Every material claim in that chain should retain source, authoritative
identifier, version, observation time, freshness, confidence or validation
state, and a deep link.

Digital products and applications must remain distinct. A product may span
web, native mobile, backend services, data products, APIs, agents, MCP servers,
and several deployable applications. Applications are technology enablers;
business value belongs to products, journeys, outcomes, and solutions.

Use four visible metric families rather than an opaque app-value score:

1. Business outcomes and Finance-recognized value.
2. Adoption and experience.
3. Delivery, quality, security, accessibility, reliability, recovery, and
   support.
4. Fully loaded build/run/maintain economics and outcome-based unit economics.

Usage, builds, deployments, prompts, tokens, component counts, and scorecard
points are evidence or cost inputs. None is ROI by itself. Every quantitative
claim needs an owner, baseline or counterfactual, denominator and formula,
measurement period, source, confidence, and validation state.

## Challenge To Existing Guidance

The accepted App Framework strategy and factory specification already define
an application management plane, management tiers, source-attributed
observations, desired/built/deployed/running identity, portfolio themes,
per-app workspaces, a Hosted Product Factory, and mobile-aware release
observations. The vision strengthens and clarifies that target; it is not a
competing program.

Use these names deliberately:

- **Enterprise App Fabric:** the reusable platform capabilities, patterns,
  runtime contracts, delivery machinery, and lifecycle spine.
- **App Fabric Operations Console:** the proposed visible product and
  management workbench.
- **Fabric Registry and Claim Graph:** the provenance-aware, bounded read
  model that projects authoritative evidence.

The Console is not the Fabric runtime, an employee or patient destination, a
universal portal, a second workflow engine, or an authoritative CMDB/APM,
portfolio, backlog, Finance, data, continuity, governance, approval, mobile
store, or Technology Strategy system.

The current prototype remains evidence only. It must not be promoted as
current truth. In particular, its seeded Technology Strategy application was
shown as active/live even though the product's own release binding described a
draft, local-only, non-deployed integration proof. Any reused demo must correct
that contradiction before it is shown as truthful Fabric evidence.

## Own, Federate, Or Link

| Domain | Product disposition | Authority remains with |
| --- | --- | --- |
| Stable provisional Fabric identity, registration tier, and Fabric lifecycle decision | Own audited Fabric record | Fabric Product Owner and service owner |
| App-owned product context, value hypothesis, proposal, and factory evidence | Own references and accepted repository records | Product/application owner |
| Strategy clauses, themes, decision records, release digest | Federate exact BOK release and deep-link | Strategy content authority and CIDO |
| Architecture patterns, ADRs, qualifications, and exceptions | Federate exact approved records; report proposed conformance | EAC and architecture authorities |
| Enterprise app, business-service, and configuration relationships | Federate | EA/APM/CMDB authority |
| Portfolio priority, capacity, backlog, and fulfillment | Federate; never create a second queue | EPMO and receiving-team work systems |
| Cost actuals, allocation, and recognized value | Federate validation state | IT Finance, Finance, FinOps/TBM |
| Runtime, usage, reliability, and incidents | Federate source observations | Observability, analytics, and SRE authorities |
| Data classification, stewardship, lineage, quality, and access | Link or federate | Data governance and catalog authorities |
| Source, build, artifact, provenance, SBOM, promotion, and deployment | Federate exact evidence | Git, CI, artifact, GitOps, and runtime authorities |
| Mobile build, review, track, rollout, and device adoption | Federate; orchestrate only approved typed actions | Apple, Google, MDM, and mobile-release authorities |
| AI overlap/value analysis | Own a cited recommendation artifact only | Human portfolio and domain authorities decide |

Derived views must preserve authoritative identifiers, permissions, semantics,
versions, and freshness. Write-back becomes eligible only through a
source-owner-approved, versioned, idempotent, reconciled, and auditable
operation. The Fabric never manufactures an aggregate approval from linked
decisions.

## Recommended Product Domains

1. Fabric Registry and Claim Graph.
2. Portfolio and Value.
3. Product Workspaces.
4. Factory and Intent Hub.
5. Pattern and Governance Catalog.
6. Operations and Fleet.
7. Mobile Release Center.
8. AI Portfolio Analyst.

### Patterns

Represent patterns as versioned qualifications, not booleans. Each adoption
links to the exact EAC standard or ADR, version, prerequisites, security/data/
continuity obligations, qualification evidence, expiry, exception, owner, and
component compatibility. Initial product hypotheses may include an intelligent
application, enterprise-search consumer, Kafka consumer/publisher, governed
projection/read, governed action, and MCP server. `Archetype II` must not
become controlled vocabulary until EAC defines it.

### Cross-team intents

Explore one versioned `capability_intent` envelope that binds the requesting
app/product, purpose, desired outcome, exact requirements, authority
references, acceptance criteria, receiving team, external work id, status,
and freshness. The Console owns context and correlation. The receiving team's
authoritative system owns priority, assignment, approval, and fulfillment.
Initial proof candidates are a SaaS Integration Intent and a Distilled Model
Intent. This envelope is a planned contract proposal, not an accepted schema.

### AI portfolio analyst

Run deterministic data-quality, orphan, version, and overlap checks before
model reasoning. The analyst may retrieve from the source-bound graph, test
assumptions, expose contradictions and evidence gaps, and present cited
alternatives, counterfactuals, and confidence. It may not approve investment,
funding, architecture, exceptions, risk, release, consolidation, retirement,
or recognized business value.

## External-Dependency Acceleration Rule

An unresolved external authority or integration blocks only the claim, write,
enforcement, or promotion action owned by that dependency. It does not block:

- contract and adapter design;
- fixture-backed implementation;
- read-only projection and local testing;
- role-aware UX proof;
- proposal drafting; or
- negative authorization, stale-data, and failure-mode tests.

Until the authoritative record is available, the Console displays the fact as
`UNKNOWN`, `STALE`, `MISSING`, or `UNAVAILABLE`, disables the dependent
mutation, and retains the authority, identifier, version, freshness, and
evidence required for later graduation. Role placeholders may be used during
design, but cannot satisfy an authority-dependent exit gate.

## BOK Decision Boundary

The existing Application Fabric major decision packet should be amended; a
second strategy packet should not be created. The following BOK gates remain
open and retain their own decision authority:

| Gate | Safe plan treatment while open |
| --- | --- |
| `gate.application-fabric-strategic-direction` | Build reversible read/proposal proof; do not call the lifecycle spine enterprise policy or the mandatory default |
| `gate.strategy-enterprise-architecture-interface` | Link exact approved EAC records; do not invent an architecture repository, owner, pattern, or exception |
| `gate.strategy-it-governance-interface` | Use proposal-first read models and typed links; do not create a governance authority, omnibus approval ledger, new intake, or scoring system |
| `gate.citizen-automation-production-boundary` | Citizen sandbox, guided proposal, preview, and evidence preparation may proceed; general supported-production promotion remains gated |
| `gate.source-access-policy` | Build a BOK adapter and local fixture; production retrieval fails closed until access/classification policy exists |
| `gate.supporting-source-provenance` | Bind immutable BOK and App Framework revisions; do not imply review of a newer baseline |
| `gate.business-service-resilience-authority` | Link BIA/recovery evidence and show missingness; do not assign continuity tiers or accept recovery posture |
| `gate.evidence-status-vocabulary` | Keep Fabric availability/freshness separate from BOK evidence status; never translate `UNKNOWN` or `STALE` into approval/readiness |

Native-mobile strategy remains a separate pending decision packet. A web-first
Console and narrow native companion are product hypotheses, not a ratified
mobile mandate or approved implementation stack.

## Proposed Proof Hypotheses

The following are useful targets for Product to baseline and ratify; they are
not BOK requirements or approved enterprise commitments:

- onboard two unlike applications in no more than one working day each;
- show source and freshness on at least 95% of operational claims;
- let representative users answer ten key portfolio questions in under two
  minutes, understand purpose/health in five seconds, and locate provenance
  plus next action in 30 seconds;
- reduce normalized second-use effort by at least 50% without quality,
  control, accessibility, security, reliability, or support regression;
- have blinded human review find at least 70% of the AI analyst's top
  hypotheses useful, with harmful false positives below 20%; and
- round-trip one cross-team intent through the receiving team's real system
  without a side-channel queue.

Each target needs a named owner, baseline, denominator, measurement window,
acceptance rule, and stop disposition before it becomes a gate.

## Evidence Quality

- **High:** BOK controlled clauses and decision packets; accepted App
  Framework strategy/spec/plan; open standards and government guidance such
  as FOCUS, TBM, SLSA, SPDX/CycloneDX, OpenTelemetry, NIST AI RMF, and MCP;
  primary platform documentation.
- **Moderate:** vendor product documentation demonstrating catalog,
  scorecard, rationalization, and workflow features; inherited App Framework
  effort estimates; vendor-reported benefits.
- **Low or absent:** PDS actual fully loaded costs and recognized benefits;
  live PDS CMDB/APM/data/Finance/analytics adapter availability; production
  proof of broad autonomous rationalization; proof that full native parity is
  economically superior to responsive web plus narrow mobile attention.

## Anti-Fad Filter

Durable capabilities are stable identity, ownership, lifecycle, exact
provenance, source/freshness, golden paths, scorecards and campaigns, fully
loaded TCO, outcome unit economics, data/API/event contracts, agent/tool
identity, and governed intent federation.

Keep MCP/A2A extensions, agent registries, AI portfolio rationalization,
semantic search over portfolio evidence, and full native Console parity behind
adapters and proof. Stop or narrow if claims require duplicate manual
maintenance, Finance cannot distinguish estimated from recognized value, the
Console displaces an authoritative system, the management plane affects
product runtime, AI produces harmful recommendations, mobile lacks a
time-sensitive differentiated journey, or tokens/catalog size/scorecard
points are presented as business value.

## Primary Source Registry

- [Backstage Software Catalog](https://backstage.io/docs/features/software-catalog/)
  and [Software Templates](https://backstage.io/docs/features/software-templates/)
- [Port governance](https://docs.port.io/governance/standards-and-compliance/overview/)
  and [self-service actions](https://docs.port.io/workflows/actions-and-automations/create-self-service-experiences/)
- [Cortex scorecards](https://docs.cortex.io/standardize/scorecards) and
  [initiatives](https://docs.cortex.io/improve/initiatives)
- [ServiceNow Application Portfolio Management](https://www.servicenow.com/products/application-portfolio-management.html)
  and [AI Control Tower](https://www.servicenow.com/products/ai-control-tower.html)
- [SAP LeanIX application rationalization](https://help.sap.com/docs/leanix/ea/application-rationalization)
  and [TIME](https://help.sap.com/docs/leanix/ea/time)
- [FinOps unit economics](https://www.finops.org/framework/capabilities/unit-economics/),
  [FOCUS specification](https://focus.finops.org/focus-specification/), and
  [TBM taxonomy](https://www.tbmcouncil.org/taxonomy/)
- [DORA platform engineering](https://dora.dev/capabilities/platform-engineering/)
- [SLSA 1.2](https://slsa.dev/spec/v1.2/),
  [CycloneDX](https://cyclonedx.org/guides/sbom/bom),
  [SPDX](https://spdx.dev/), and
  [OpenTelemetry resources](https://opentelemetry.io/docs/concepts/resources/)
- [OpenMetadata governance](https://docs.open-metadata.org/latest/how-to-guides/data-governance)
  and [lineage](https://docs.open-metadata.org/latest/how-to-guides/data-lineage),
  plus [Confluent data contracts](https://docs.confluent.io/platform/current/schema-registry/fundamentals/data-contracts.html)
- [NIST AI Risk Management Framework](https://www.nist.gov/itl/ai-risk-management-framework)
  and [OWASP Agentic Applications Top 10 for 2026](https://genai.owasp.org/resource/owasp-top-10-for-agentic-applications-for-2026/)
- [Model Context Protocol 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28)
- [Apple App Store Connect API](https://developer.apple.com/documentation/appstoreconnectapi)
  and [Google Android Publisher API](https://developers.google.com/android-publisher/api-ref/rest)

## Role Card Check

- Card used: Framework Research Steward.
- Work stayed within role: current-truth inspection, external industry and
  standards research, BOK challenge, and product recommendations.
- Authority not assumed: no BOK, roadmap, source, branch, release, funding,
  architecture, Finance, risk, or production decision was changed.
- Routed decisions: product scope to Product; strategy to CIDO/Strategy;
  patterns to EAC; economics to Finance; sequencing to portfolio authorities;
  domain controls to their respective owners.
- Drift signals: treating proposed thresholds as ratified, calling the Console
  a universal portal, presenting native mobile as decided, or describing the
  Fabric candidate as current enterprise policy.
