# PDS Nexus Product Development Brief

> **Status: product-intelligence brief, not source-system evidence.** This page
> captures the Copilot-collected meeting/email summary supplied on 2026-07-03.
> The original Teams, SharePoint, meeting transcripts, and emails were not
> directly verified in this repository pass. Treat this as a directional product
> brief that informs App Framework prioritization and maturity demands; convert
> any claim into retained evidence before using it as release proof.

## Executive thesis

PDS Nexus is envisioned as the ServiceNow-based successor to Pega applications:
a centralized workflow and orchestration capability that replaces fragmented
workflows with transparent request and task tracking plus governed coordination
across systems and teams.

For App Framework, Nexus is a future forcing product for the PDS-owned
enterprise-experience target. ServiceNow remains the workflow and fulfillment
authority. Employee Slate/Moveworks is the strongest comparator and a credible
fallback because its 2026 product already offers search, conversation, canvas,
Inbox, requests, tasks, approvals, forms, notifications, widgets,
accessibility support, analytics, and web/mobile access. The PDS product must
earn broad adoption through managed trust, better measured work, signature
quality, supportability, reuse, and lifecycle economics.

## Product values

| Value | Product meaning | App Framework implication |
| --- | --- | --- |
| Team-member experience first | Users should not need to know which department, system, or legacy app owns a process. | Build one coherent PDS web/native-mobile experience over portable request, task, action, attention, and intelligence contracts; measure the same work in Employee Slate/Moveworks as the comparator and fallback. |
| Transparency and visibility | Users need "My Requests", "My Tasks", ownership, workflow status, progress, and guidance. | The model must include first-class request, task, owner, team, status, event, timeline, SLA/freshness, and audit concepts. Generated views must support status tracking, not only CRUD screens. |
| Standardization and governance | The platform should replace fragmented workflows with common intake, common workflow patterns, and controlled integrations. | Product intake, generated ownership, provider certification, release evidence, PDS catalog checks, and governed-write gates are required product capability, not optional ceremony. |
| Scalability | Nexus is tied to long-term enterprise growth across tens of thousands of field team members and thousands of offices. | Local demo evidence is insufficient. Load, tenant isolation, provider freshness, queue/backpressure, observability, mobile evidence, and release-gate artifacts must mature before production claims. |
| Evidence-based process design | Future orchestration should reflect how work is actually performed, informed by process intelligence and discovery. | Product modeling must be evidence-driven: process exports, event logs, Pega inventory, ServiceNow dictionaries, SaaS API specs, and workflow telemetry should feed model proposals instead of agents inventing intended-state diagrams. |

## Strategic goals

1. **Replace Pega.** Nexus should support retirement of Pega-based workflows and
   applications by modeling those workflows into App Framework product contracts
   and ServiceNow-backed orchestration paths.
2. **Create a unified experience.** Nexus should become a single pane of glass
   for team members, reducing confusion from multiple disconnected systems and
   AI experiences.
3. **Centralize long-running business processes.** Nexus should coordinate
   multi-step, cross-functional work across systems, teams, ownership changes,
   approvals, and status transitions.
4. **Simplify the technology landscape.** The product should reduce duplicate
   applications, license footprint, support complexity, and one-off workflow
   implementations.
5. **Enable enterprise growth.** Nexus must support the modeled PDS profile of
   approximately 17,000 potential users and 1,100 locations; a future larger
   profile requires its own capacity and operating evidence.

## Product capability model

| Capability | Minimum Nexus shape | Framework feature pressure |
| --- | --- | --- |
| Unified request intake | One front door for requests, with category, requester, impacted office/team, priority, required evidence, and routing. | Product intake/golden path must model workflow products cleanly; generated forms need validation, policy-denied states, and PDS-quality UX. |
| Task and work queue | "My Tasks", team queues, ownership, due dates, blocked/stalled states, escalations, and handoffs. | Generated UI needs task/workflow views, not just entity grids. Mobile read/monitor/approve flows become strategically important. |
| Workflow tracking | Request status, step history, owner changes, dependencies, and progress over time. | `AgentTimeline`, audit timeline, event model, freshness/lineage reports, and status chips must be mature, accessible, and testable. |
| Orchestration across systems | ServiceNow-centered workflow, with integrations to systems of record where work or data lives elsewhere. | SaaS connectors, kappa/CDC projections, sync workers, principal envelope, and provider certification are on the critical path. |
| Governed actions | Approvals, nudges, updates, submissions, escalations, and eventual ServiceNow writes. | G1 delegated auth, per-user token store, named mutation registry, idempotency, HITL gate records, write audit, and G2 Intent Preview cannot stay posture-only. |
| Structured and conversational guidance | Users can ask where work stands or what is blocked, and receive grounded answers/views in the selected channel. | Produce policy-trimmed answer envelopes, citations, entity refs, evals, and channel-neutral render contracts. PDS Conversation/Ambient components are used only when a named product task proves the need; intelligence cannot become a separate, ungoverned product. |
| Process intelligence | Actual work patterns inform process design and improvement. | Product analysis should accept process exports/event logs and retain evidence in model proposals; future workflow mining should be a product-intake extension. |
| Mobile team-member access | Field and office users can monitor and approve work through the best-fit supported mobile channel. | Evaluate the entitled Employee Slate/Moveworks mobile baseline first. React Native + Expo is the PDS-native direction when device behavior, offline needs, external personas, deep work, control, or measured outcome requires a PDS application. |

## Maturity demands on App Framework

Nexus raises the maturity bar because it is not a brochure app. It is an
enterprise workflow replacement program. These framework areas should not be
called production-ready for Nexus until they meet the stated evidence demands.

| Area | Required maturity for Nexus | Evidence demand |
| --- | --- | --- |
| Product modeling | Pega replacement scope, workflow states, teams, ownership, tasks, requests, approvals, and ServiceNow mappings are explicitly modeled. | Retained intake, product-analysis, model-proposal, model-status, and signed review artifacts; no CRM residue; no invented process fields. |
| ServiceNow integration | ServiceNow is treated as a governed system of record with authenticated evidence, not guessed contracts. | Authenticated OpenAPI/dictionary export, ACL/role/plugin/domain separation notes, selected named operations, provider-test evidence, and vendor-doc parity. |
| Read projections | Nexus can rely on fresh, permission-preserving, explainable ServiceNow-derived data before live write-back. | Entitlement provenance, policy/version, dynamic field masks, revocation/freshness SLO, tombstone/reassignment/group-change handling, fail-closed stale behavior, source reconciliation, cross-channel negative tests, and release-retained kappa/CDC evidence. |
| Governed writes | AI or UI actions never write directly. Every write is named, source-authorized, narrowly scoped, delegated/on-behalf-of, auditable, idempotent, and human-gated by risk. | G1 live token-isolation/revocation/scope and governed-write evidence, per-user token store, principal envelope, target-system authorization, HITL gate-record enforcement, `governed-write-evidence.json`, and G2 live-readiness evidence. |
| Experience and channel fit | PDS owns the target employee experience while vendor surfaces remain measured comparators and fallbacks. | Comparative Employee Slate/Moveworks evidence; portable contracts; PDS web/native-mobile proof; explicit coexistence, retirement, stop/narrow, and adapter boundaries. |
| Structured intelligence | Answers are grounded, cited, policy-trimmed, evaluated, and rendered in the selected vendor or PDS channel. | `chat-eval.json`, future judge/live evidence, answer-envelope schema contracts, tenant-isolation red-team fixtures, prompt audit/kill-switch posture, citations, confidence/freshness, and dual-channel renderer evidence. |
| Mobile | Native mobile is a first-class PDS product channel with channel-appropriate continuation, not compressed desktop. | Retain vendor mobile comparison evidence and require `mobile-test` run-local evidence, runtime audit disposition, simulator/device evidence, store-track evidence, and secure-storage/push/offline decisions for the launched PDS scope. |
| Scale and operations | The modeled 17,000-user/1,100-location profile requires real performance, queue, observability, resilience, cost, and support posture. | Load-test suite, provider-performance evidence, readiness/metrics, OTLP/SIEM/export posture, alerting/runbook evidence, recovery evidence, and release-check artifacts. |
| Security and compliance | Workflow, PHI, identity, and agentic-action risks are first-class. | Tenant/IDOR negative tests, audit redaction/chain evidence, secret/PHI scans, SBOM/SCA, AIBOM/agent attribution, provenance/signing or risk acceptance. |

## Prioritization guidance

Nexus remains held until explicit product activation. The future dependency
order below does not authorize a PoC, choose a journey or ServiceNow object, or
override the active product-neutral framework-readiness goal.

| Priority | Work | Why it matters |
| ---: | --- | --- |
| 0 | Complete **AF-OG01 Framework Ready For Nexus PoC** with product-neutral package, model/config, feedback, permissioned-runtime, provider-simulation, accessible-state, mobile-read, telemetry, and genuine-upgrade evidence. | Product work must not begin by repairing the framework's developer path or bypassing trust gates. |
| 1 | Obtain explicit product activation: named sponsor, Product Owner, cohort, outcome, journey, source object/owner, fields/classification, approved environment, channels, support owner, and stop thresholds. Gather the Pega, ServiceNow, vendor-baseline, security, and operations evidence for that bounded scope. | Agents must not invent Nexus requirements or treat public vendor capability as PDS entitlement/evidence. |
| 2 | Make the accepted principal, permission, operation, provider, audit, and telemetry path authoritative, then live-certify the selected version-pinned ServiceNow read. | An executable permissioned source read precedes shared-contract expansion, intelligence, action, Kafka, or broad composition. |
| 3 | Use the PDS employee product as the real no-checkout package consumer and prove the same-object permission-preserving MongoDB projection plus a genuine framework upgrade. Measure the same outcome in Employee Slate/Moveworks. | This proves the PDS target and vendor fallback against equivalent work without making the comparator the architecture. |
| 4 | Add one preview-only operation, then one separately authorized low-risk governed action and managed operating profile. | Identity, source authorization, policy, idempotency, audit, reconciliation, recovery, and support must remain one path across channels. |
| 5 | Attach one authenticated, policy-trimmed structured PDS Health AI result with deterministic fallback and exact web/native-mobile continuation. | Intelligence extends the proven read/action spine; it does not create a parallel assistant or transaction authority. |
| 6 | Prove reuse in one citizen graduation or suitable Pega redesign and correlate work, trust, vendor, release, support, quality, and cost evidence in PDS Observability. | Broad fabric investment requires at least 50% lower comparable second-use effort and measured outcome/economic value. |

## What not to do

- Do not turn the PDS-owned target into a production-readiness claim. Use
  Employee Slate/Moveworks as the strongest comparator and fallback; stop or
  narrow broad front-door scope if PDS cannot meet trust, outcome, support, or
  economic proof.
- Do not promise seamless integration across all systems until each connector,
  projection, and write path has retained evidence.
- Do not build a chat-first Nexus. Chat is a control surface over workflow
  objects; the task/request/workflow experience remains the center.
- Do not graduate AI or ambient recommendations without grounding,
  preview-gating, attribution, citations, eval evidence, and audit.
- Do not model Nexus by copying CRM sample structure. Nexus is a workflow
  orchestration product with long-running process semantics.
- Do not allow synthetic seed data, local fixtures, or process diagrams to
  masquerade as live ServiceNow/Pega replacement evidence.

## Open evidence needed from product and platform owners

- Pega app/workflow inventory, prioritized by business value and retirement
  urgency.
- The first Nexus business slice and success criteria for executive review.
- ServiceNow authenticated OpenAPI/dictionary export, ACL/role/plugin/domain
  separation details, test tenant/user/principal, and approved mutation scope.
- Process intelligence/event-log exports showing how work actually moves today.
- Data classification, PHI/PII handling, retention, and audit requirements for
  Nexus records, prompts, and projections.
- Scale targets: concurrent users, request/task volume, office/team cardinality,
  freshness SLOs, escalation latency, and mobile adoption assumptions.
- Mobile distribution path: Apple/Google/EAS ownership, MDM needs, push payload
  policy, offline/cache policy, and store-track evidence expectations.
- Support model and operational ownership across App Framework, ServiceNow,
  data platform, security, and product teams.

## Framework decision rule

When prioritizing App Framework work, ask:

> Does this make a real Pega workflow easier to replace through a reusable,
> governed, observable capability that works in the best-fit channel and proves
> a measurable product or economic outcome?

If yes, it is likely on the critical path. If no, it may still be useful, but it
should not outrank product-neutral readiness, workflow transparency, live
provider proof, archetype reuse, structured intelligence, or release-grade
operations. Narrow the broad PDS front-door scope and use the vendor fallback
if users do not perform materially better, second-use effort does not fall by
at least 50%, live provider certification stalls, or durable product,
platform, design, content, security, SRE, and support ownership cannot be
funded.
