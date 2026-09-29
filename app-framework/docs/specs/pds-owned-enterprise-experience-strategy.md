# PDS-Owned Enterprise Experience Strategy Decision

Status: accepted-strategy-direction; implementation activation remains gated

Spec depth: full

Owner roles:

- Human strategy authority: PDS Health sponsor
- Strategist/Product Manager: canonical direction and freshness
- Product Owner: product outcomes, activation inputs, acceptance, and stop rules
- Architect: target boundaries and technical proof sequence
- XO: coordination after an Outcome Goal is released
- Integration Branch Manager: promotion and exact-destination evidence

## Business Value

PDS Health needs one coherent enterprise experience across ServiceNow, Workday,
Salesforce, PDS Health AI, PDS-built products, web, and native mobile while
preserving source-system authority. Owning the experience and portable PDS
capability contracts can reduce fragmentation, repeated integration work,
vendor UX churn, and status chasing while improving product throughput,
cross-channel continuity, deep operational work, and application retirement.

The strategy is valuable only if managed trust, measurable task outcomes,
supportability, reuse, signature quality, and lifecycle economics justify the
additional ownership burden.

## Problem

The previous live strategy treated Employee Slate/Moveworks as the rebuttable
default employee shell and PDS products as exceptions. The 2026-07-14 product
and architecture decision changed the target to a PDS-owned enterprise
experience, but canonical documents briefly retained both positions. That
ambiguity changed product ownership, funding, architecture, design, operating
model, and roadmap interpretation without one durable decision contract.

App Framework also has strong foundations but is not a production enterprise
front door. It lacks managed proof for the complete runtime spine, live
ServiceNow read, permission-preserving projection, governed action, broad
mobile operation, PDS Health AI loop, scale, resilience, and support required by
the target.

## Goals

- Make a coherent PDS-owned web/native-mobile enterprise experience the target.
- Keep ServiceNow authoritative for workflow, fulfillment, case, service
  management, tasks, approvals, and records that belong there.
- Use Employee Slate/Moveworks as the strongest comparator and credible
  fallback, not the default target or an inferior straw man.
- Make the first employee product the real independently packaged App Framework
  consumer and vertical proof container.
- Own portable PDS identity, context, permission, operation, attention,
  evidence, intelligence, telemetry, product-experience, and lifecycle
  contracts without recreating mature engines.
- Advance through measurable outcome, support, reuse, trust, signature-quality,
  and economics gates with an explicit stop/narrow path.

## Non-Goals

- No duplicate workflow engine, enterprise search index, identity authority,
  content platform, foundation-model platform, universal agent runtime, or
  unconstrained generated UI.
- No immediate Nexus PoC activation, journey selection, ServiceNow-object
  selection, live-provider work, release, or production-readiness claim.
- No universal portal, generic widget marketplace, broad provider expansion,
  or microservice program before product consumption proves the need.
- No claim that the current cost model prices feature-equivalent options or
  that agentic development removes durable product, design, security, SRE,
  content, integration, and support ownership.

## Scope

This decision governs North Star, enterprise-app-fabric, platform strategy,
product-management strategy, Nexus product framing, roadmap outcomes, and the
Product Dashboard projection. Tactical lane design and product requirements
remain outside this spec.

## Repository Context

- `docs/strategy/product-development-north-star.md`
- `docs/strategy/enterprise-app-fabric.md`
- `docs/strategy/app-framework-platform-strategy.md`
- `docs/strategy/app-framework-product-management-strategy.md`
- `docs/product/pds-nexus-product-development-brief.md`
- `docs/release/roadmap.md`
- `docs/frontend/pds-health-design-system.md`
- Dated decision research:
  `pds-owned-enterprise-experience-fabric-roadmap-2026-07-14.md` in the approved
  PDS app-fabric research workspace

## Contracts Touched

| Contract Surface | Expected Change | Counterpart Surfaces That Must Stay Aligned |
| --- | --- | --- |
| Strategic target | PDS-owned enterprise experience; vendor comparator/fallback | North Star, fabric, platform/product strategy, Nexus brief, roadmap |
| Current execution | Roadmap Outcome Goal Register is sole sequence | XO board/queue, Product Dashboard, strategy links |
| Product boundary | ServiceNow workflow authority; App Framework experience/capability/product lifecycle | Architecture, provider, operation, projection, product ownership |
| Product proof | First employee product is real package consumer and measured vertical | AF-OG01 through AF-OG07, Product Owner intake, managed evidence |
| Economics | Incremental, evidence-backed funding and stop/narrow rules | Cost model, roadmap advancement, dashboard non-claims |

## Options Considered

| Option | Pros | Cons | Decision |
| --- | --- | --- | --- |
| Employee Slate/Moveworks as default target shell | Fastest managed front door; mature search, work, mobile, administration | Vendor UX/control dependence; weaker fit for deep PDS products and portable cross-system contracts | Retain as comparator and fallback, not target |
| PDS-owned enterprise experience over vendor engines | Coherent PDS product, portable contracts, deep web/mobile work, product-factory leverage | Material build and operating burden; must prove trust, value, reuse, support, and economics | **Selected strategic target** |
| Rebuild vendor engines and assistant/search breadth | Maximum nominal ownership | Duplicates mature systems; highest cost/risk; distracts from PDS differentiation | Reject |

## Decision Provenance

| Date | Owner | Decision | Evidence / Rationale | Revisit Trigger |
| --- | --- | --- | --- | --- |
| 2026-07-14 | Human strategy authority | Make PDS-owned enterprise experience the target; Employee Slate/Moveworks is comparator/fallback | Product/architecture review plus dated market, UX, parity, permissions, economics, technical-debt, and experience-fabric synthesis | PDS cannot clear trust/outcome/support/economic gates; vendor economics or capability changes materially; durable operating ownership cannot be funded |

## Architecture And Implementation Notes

The Roadmap Outcome Goal Register is the only current sequence. AF-OG01 remains
active and product-neutral; AF-OG05 may advance as a bounded, disjoint CRM
reference-experience proof. Product-specific outcomes stay gated until their
activation inputs exist.

When activated, the employee product must be the real package consumer for one
authoritative identity/policy/operation/provider/audit/telemetry spine, one
version-pinned permissioned ServiceNow read, one same-object permission-
preserving MongoDB projection, one governed action, one managed operating
profile, one genuine upgrade, exact web/native-mobile continuation, and one
structured PDS Health AI result. Extract shared fabric only after a second real
use proves the contract.

Use code-native delivery for behavior, integrations, security, and product
logic. Use governed administration for low-risk content, audiences, navigation,
role defaults, module placement, and flags with version, preview, audit,
rollback, and policy locks.

## Security, Privacy, And Governance

The target does not weaken tenant isolation, source authorization, field/row/
action policy, delegated identity, secrets, PHI/PII controls, evidence truth,
generated parity, destructive-write controls, or human release/risk authority.
P0 controls must reach managed production evidence for the launched scope.
Search results never grant record or action authority. Product configuration
cannot change permission, source truth, required controls, or action authority.

## Acceptance Evidence

| Criterion | Proof Command / Artifact | Required Before |
| --- | --- | --- |
| One target and one comparator/fallback posture across canonical docs | full docs-check plus independent comprehensive review | strategy promotion |
| One current sequence owned by roadmap | contradiction scan; Outcome Goal Register links; XO/PO/Architect alignment | strategy promotion and lane release |
| Product target remains current-versus-goal truthful | AF-OG01-AF-OG07 non-claims and maturity evidence | every advancement claim |
| First product proves complete vertical and genuine upgrade | package, provider, projection, action, operations, web/mobile, intelligence, upgrade evidence | controlled launch |
| Operational and signature quality both pass | task outcome, accessibility, reliability, support, comparative preference evidence | broad adoption |
| Reuse and economics justify fabric | second-use effort and fully loaded cost per accepted outcome | shared extraction/scale investment |

## Test And Execution Feedback Plan

Run framework validation, full docs examples, generated-drift check, fast
framework tests, exact-SHA handoff, and fresh comprehensive Framework PR Review.
Retain a source-bound research-refresh artifact. If evidence contradicts the
strategy, narrow product scope or return to the vendor fallback; do not turn
target-state language into current maturity.

## Risks And Controls

| Risk | Control | Owner | Status |
| --- | --- | --- | --- |
| PDS builds a portal rather than an outcome product | First employee product owns one measured vertical and signature brief | Product Owner/design | open proof |
| Framework work outruns consumption | 70%+ capacity into activated vertical; broad capability families held | XO/Strategist | roadmap rule |
| Vendor engines are unnecessarily duplicated | Explicit ServiceNow/search/identity/content boundaries | Architect/Product Owner | accepted boundary |
| Operating burden is underfunded | Directional ranges plus named durable owners and staged funding | Human/Product Owner | decision input |
| Visual polish masks weak trust or work outcomes | Independent operational and signature-quality bars | Product Owner/design/security | required proof |

## Tech Debt And Follow-Up

- Integration must refresh the strategy branch onto accepted main and rerun the
  exact integrated proof before promotion.
- Volatile execution order belongs only in the roadmap and tracked program
  contract; strategy documents link to it.
- The Product Dashboard may project this decision only after accepted canonical
  source reaches the destination branch.

## Handoff Notes

This spec accepts strategic direction only. It grants no product activation,
architecture approval, implementation, merge, release, SRA/CAB, accepted-risk,
live-credential, or production authority. The held healthcare-operations/Epic
candidate remains held and requires a separate explicit human release.
