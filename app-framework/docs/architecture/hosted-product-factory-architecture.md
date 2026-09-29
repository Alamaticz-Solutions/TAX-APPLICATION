# App Fabric Management Plane And Hosted Product Factory Architecture

> **Status: target-state conceptual architecture; shaped and held.** This
> document communicates the design to engineering. The normative contract —
> gates, identity rules, rollout conditions, acceptance evidence — is the
> [Hosted Product Factory Spec](../specs/hosted-product-factory.md). The nine
> FAB-A1 wire contracts and their conformance checks are accepted source, but
> no management-plane or factory service described here is operational today.
> Nothing here authorizes implementation; the
> [Roadmap Outcome Goal Register](../release/roadmap.md#outcome-goal-register)
> owns sequencing (candidate home: AF-OG08).

## What This Is

The Hosted Product Factory is the governed code-changing subsystem behind the
[Enterprise App Fabric application management
plane](../strategy/enterprise-app-fabric.md#application-management-plane) and
the hosted delivery of the existing
[PoC To Enterprise Product Intake](../lifecycle/intake-and-discovery.md) and
[Product Developer Golden Path](../lifecycle/product-golden-path.md): a
business stakeholder searches the fleet, opens an authorized application
workspace, inspects source-bound product and runtime context, and works with an
approved conversation agent. Beyond the bounded G-F13 idea-stage trial
allowance, it is after named-sponsor approval and admission that the
factory creates or uses a governed Bitbucket product repository, drives the
documented intake-to-generation loop in ephemeral Kubernetes runners, deploys
a permissioned synthetic-data preview, and correlates the result back to the
workspace. Every iteration flows through the same review, CI, acceptance, and
release gates the harness already enforces.

Design stance in one line: **conversation drafts, humans sponsor, rules
admit, agents implement inside a fence, gates dispose, each fact stays with
its authority, dashboards only project it.**

## Unifying Conceptual Diagram

```text
                         PDS Okta (OIDC + OAuth 2.1)
              human sign-in                      workload identity
                    |                                   |
+-- MANAGEMENT EXPERIENCE + CONVERSATION ------------------------------+
| fleet search -> /apps/{app_id} workspace -> source/freshness views   |
|      |                  |                                             |
|      | ask / draft / submit typed proposal / request / accept         |
|      v                  v                                             |
| approved agent client + factory MCP: typed, idempotent, audited;      |
| no shell, repo, cluster, registry, release, or workstation escape     |
+------|----------------------------------------------------------------+
       v
 [G-F12 registry authority]
+-- FABRIC MANAGEMENT CONTROL PLANE (Kubernetes) -----------------------+
| fabric-api + registry service + authorized dashboard read API         |
| projection reconciler --> query read model + component inventory      |
+------|----------------------------------------------------------------+
       | stable app/context refs + typed proposals / delivery observations
       v
 [G-F13 idea bootstrap] [G-F1 sponsorship: idea -> planned] [G-F2 screen]
+-- HOSTED PRODUCT FACTORY SUBSYSTEM (separate authorization boundary) --+
| factory-api + MCP/approvals + proposal ledger/work queue               |
| admitted reference --> admission: WIP + budget [G-F3]                 |
| planner --> runner orchestrator --> ephemeral runner Jobs [G-F4]      |
| review router [G-F5] --> PR/candidate/destination/release evidence     |
+------|----------------------------------------------------------------+
       | typed proposals, observations, commands and correlation ids
       v
+-- AUTHORITY & EVIDENCE SOURCES ------+  +-- DEV PREVIEW PLANE --------+
| enterprise app registry / CMDB       |  | one isolated ns per product |
| product repo: intent/model/work      |  | signed immutable image      |
| Bitbucket: PR/build/destination      |  | auth on; synthetic data     |
| OCI/SBOM/provenance: built identity  |  | ArgoCD desired/deployed     |
| GitOps + Argo/K8s: deployment state  |  | OTel runtime observation    |
| analytics/feedback: outcomes         |  | product traffic and data do |
| model gateway: AI usage/cost         |  | not traverse control plane  |
+--------------------------------------+  +-----------------------------+

 Cross-cutting: exact SHA/digest correlation; `UNKNOWN`/`STALE` fail closed;
 distinct service identities/failure domains; kill switches [G-F11];
 autonomy phases graduate on evidence [G-F10].
```

Gate definitions and authorities are normative in the
[spec's Policy Gates table](../specs/hosted-product-factory.md#policy-gates).

## Planes And Components

### Conversation Plane

An approved enterprise conversation agent (Claude may be the initial client)
fronts three audiences: the citizen developer (drafts intent, gives feedback,
accepts Deliverables), the sponsor (approves intake and funding), and engineers
(review and decide through existing harness tools). The plane holds no
authoritative state or infrastructure credentials: it acts only through the
provider-neutral factory MCP under the authenticated caller's delegated
authorization. Nothing a model says can touch a repository, cluster, registry,
release system, workstation, or shell directly.

### Fabric Management Control Plane

A small non-code-changing service set, built and operated as an App Framework
product so it inherits the platform's gates, evidence, and observability:

| Component | Responsibility | Key boundary |
| --- | --- | --- |
| fabric-api | Registry decisions, dashboard read API, source/freshness views, audit export | No code writing, merge, release, cluster mutation, product traffic, or product business data |
| Registry service | Records and enforces authenticated application registration, lifecycle and management-tier decisions made under G-F12 | Stores references and audited decisions; does not make product decisions or duplicate product, build, deployment or telemetry truth |
| Projection reconciler | Pulls or receives source observations and materializes an authorized query model | Preserves authority reference, exact identity, observation time and freshness; stale/unknown cannot advance gates |
| Component inventory index | Correlates desired, built, deployed and running composition, platform-qualified per declared surface | Never collapses the four states into one ambiguous application version |

The plane persists three classes of state with different write rules,
against the framework's own PostgreSQL provider (the console is itself an
App Framework product): an audited decisions ledger — the only state the
plane authors (G-F12 registrations, tier and lifecycle decisions); an
append-only observation ledger of immutable, source-attributed facts with
exact identity and observed time; and disposable query projections that can
be dropped and rebuilt from the two ledgers at any time. The factory
subsystem keeps its queue, lease, idempotency and run state in a separate
database with distinct credentials. Product business data, PHI, prompt
bodies, and product truth are never stored in either.

### Hosted Product Factory Subsystem

The factory is a separately authorized code-changing subsystem. It may
initially share a repository, Kubernetes cluster, release train, observability
stack, and operating team with the management services, but deploys as
separate workloads with distinct APIs, service identities, policy,
credentials, storage, failure boundaries and kill switches:

| Component | Responsibility | Key boundary |
| --- | --- | --- |
| factory-api | Provider-neutral MCP, approvals, typed proposal/command API, run audit | Validates every call against schema, caller/app authority, sponsorship where required (idea-stage calls stay within the G-F13 trial allowance) and idempotency; fails closed |
| Proposal ledger and work queue | Retains proposals and dispositions immediately; queues references for admitted work | Conversation is never provenance; queue state is not product priority or program truth |
| Admission governor | Fleet WIP ceiling, per-product budgets, staleness, stop rules | Deterministic-first: rules decide without a model call whenever possible |
| Planner | Replan proposals from feedback and acceptance state | Proposes only; product/program priority decisions route to their human owners before admission |
| Runner orchestrator | Leases Assignments to ephemeral runner Jobs | One Assignment, one runner, one branch; leases expire |
| Review router | Class A/B agent review versus Class C/D human review | Mirrors two-speed routing in the [operating model](../start/agentic-human-operating-model.md) |

For a factory-managed product, the typed proposal and every disposition are
written immediately to the product repository's governed proposal record (or
to an approved append-only proposal authority when no repository exists yet).
The work queue carries only references, idempotency state and leases. Rejected,
deferred and superseded proposals retain the same provenance as admitted work.

### Fleet Registry And Reconciliation

Every application may be `registered`; only explicit cumulative tiers add
read-only observation, App Framework lifecycle management, or factory-control
capability. Tier is not maturity, release, security, or risk state.
Registration contains stable identity, searchable purpose summary, owner,
lifecycle, classification, canonical references, support contact,
environments, declared platform surfaces (web, native mobile), and
management tier. Detailed purpose and work stay in the
accepted product repo for managed products.

Identity is fabric-minted, and the registry is the system of origin for
fabric-born applications: ideas register with a repository and dev presence
under G-F13 controls before any enterprise CMDB record exists, matriculate
into the CMDB at the planned (G-F1) decision, and cannot promote beyond dev
without the CMDB configuration item (CI). Pre-existing applications register
by reference to their existing CMDB record. Lifecycle, management tier, and
environment presence are three separate dimensions; the normative state
machine is in the
[spec's registration section](../specs/hosted-product-factory.md#registration-and-composition).

The reconciler consumes source-specific adapters and emits
`fabric_observation_event@1` records into an append-only history and query
projection. Each event carries event/schema id, occurred and observed time,
source and integrity reference, `app_id`, work/run correlation ids, exact
commit/PR/pipeline/image/deployment identity when applicable, and freshness.
The read model is disposable and rebuildable from retained decisions and
source observations; it does not become a write-back system for those sources.

### Product Planes

Each factory-managed product gets one isolated preview namespace in the dev
Kubernetes environment. The preview deployment is the documented
[product topology](../release/deployment-reference.md) — single backend image
serving API and SPA behind an oauth2-proxy/Okta sidecar — configured with the
factory preview posture: auth on, synthetic data only, Prototype stage badge,
migrations as jobs, ArgoCD syncing signed immutable digests. The product SPA,
the React Native scaffold for declared native-mobile surfaces, and the
central fleet and
per-app management experience all compose from the
[PDS Health Design System](../frontend/pds-health-design-system.md) catalog,
tokens, and agent recipes per the
[Frontend Starter Contract](../frontend/product-frontend.md); autonomous
iterations cannot introduce bespoke visual systems or new UI dependencies
(write surface, G-F4). The fleet dashboard is hosted centrally; its authorized
per-app workspace links to the product preview and runtime instead of running
inside the preview namespace. Production and other runtime environments are
separately deployed and governed; the dev-hosted management plane only
observes and links to them.

### Authority Sources And Evidence Backbone

Authority is assigned by fact class, using the normative matrix in the spec.
The product repository owns product intent, model, work, typed proposals and
acceptance records; Bitbucket owns PR/build/destination observations; signed
provenance, SBOM and artifact registries own built composition; GitOps owns
desired deployment; ArgoCD/Kubernetes own deployed/running observations;
approved analytics, feedback and model-gateway meters own their respective
outcome and cost observations; the app store consoles own store submission,
release and staged-rollout observations; the approved mobile
build/distribution service owns signed mobile build identity; the approved
crash/device-health source owns device-fleet health; and named humans own
acceptance, merge, release and risk decisions. The projection records
references and freshness, never re-authors those facts.

Repo bootstrap (G-F13-gated at idea stage; G-F1 graduates idea → planned)
applies branch protections, required checks, product CI, and the day-zero
intake record atomically — no factory-managed product exists without its
gates. Repository automation uses the
[Bitbucket REST Auth Runbook](../start/bitbucket-rest-auth.md) credential path
only.

## Conversation-Agent-To-Factory Communication

- MCP over streamable HTTP; OAuth 2.1 authorization-code with PKCE against
  Okta; access tokens audience-bound to the factory resource; short-lived.
- Typed tools with explicit authority levels (`search_applications`,
  `get_application_workspace`, `draft_intake`, `submit_intake`,
  `submit_feedback`, `propose_priorities`, `draft_change_proposal`,
  `submit_change_proposal`, `request_iteration`,
  `accept_slice`/`reject_slice`, `create_product_repo`). The full table and
  authority semantics are in the spec.
- Long-running work is asynchronous: tools enqueue and return tracking ids;
  progress lands on the dashboard, not in a blocked chat turn.
- Irreversible operations (repository creation) execute only against a
  recorded human approval and carry idempotency keys — a replayed or
  duplicated call cannot create a second repo.
- Every call is audited with principal, on-behalf-of product, arguments
  hash, decision, and correlation id; prompt and tool audit export to SIEM
  with retention per the chat audit posture in the Deployment Reference.

## Reasoning Architecture

The factory separates deciding from generating, and both from acting:

```text
typed records --> rules (no model) --> role-shaped reasoning --> proposals
                                                                    |
                                              gates + humans dispose v
                                                          governed actions
```

1. **Deterministic layer.** Admission, WIP, budget, routing, and staleness
   are rules over typed records — the `scripts/check-program-flow.mjs`
   pattern generalized. If no rule requires judgment, no model runs.
2. **Role-shaped reasoning services.** Stateless, event-invoked, mirroring
   Control Crew functions: Intake Analyst (wraps `product analyze` /
   `propose-model` outputs), Product Planner (feedback to replan proposals
   routed to the app Product Owner or Program Product Manager for priority
   disposition), Admission (Program Flow Controller applies capacity and
   dependency policy only after that decision), Implementation workers (one
   Assignment, four controlling records as context, never chat history), and
   independent reviewers (cross-model for sensitive work).
3. **Effort routing.** Medium-effort workers for extraction and mechanical
   transforms; high-effort synthesis for replanning and conflicts;
   exceptional escalation recorded — the
   [Research Steward](../start/framework-research-steward-harness.md)
   routing discipline.
4. **Structured outputs only.** Versioned, schema-validated artifacts
   (`factory_intake@1`, `factory_replan_proposal@1`,
   `factory_iteration_report@1`) retained in the repo or run manifest.
   Reasoning proposes; gates dispose.

## Identity And Authentication

Full identity matrix and rules are normative in the spec. The shape:

- **Humans** authenticate through Okta OIDC and receive only separately
  assigned role authorities. The app Outcome Owner proposes and accepts; the
  sponsor funds; Product Owners dispose app priority; the Program Product
  Manager disposes program/portfolio priority; the Program Flow Controller
  applies capacity/dependency/admission policy only; engineers implement;
  reviewers review; a named human repository approver may merge; and named
  security, SRA, CAB, risk and release authorities retain their decisions. No
  role inherits another merely by using the same dashboard.
- **Non-humans** hold narrow, short-lived credentials: factory services use
  Kubernetes workload identity; each runner Job gets a per-product,
  repo-scoped token and registry read access, nothing else; preview
  workloads see synthetic data sources only; the Bitbucket provisioning
  credential is used exclusively by the approved bootstrap operation.
- **Attribution is non-negotiable.** Every agent action records its
  sponsoring human principal; no shared service account launders human work.
  The conversation provider is never authority or an identity of its own — it
  acts under the caller's delegated authorization, recorded on-behalf-of.

## Product Iteration Loop

```text
feedback (chat/dashboard) -> typed record -> planner proposal
  -> admission [G-F3] -> runner Job [G-F4] -> branch + PR + evidence
  -> accelerated preview -> candidate review/PR [G-F5/G-F6]
  -> human merge -> exact destination proof -> signed dev image [G-F7]
  -> workspace regenerates -> accept exact result? [G-F9]
  -> strict release evidence for Accepted digest [G-F8] -> human promotion
```

Two work classes and five delivery states are shown honestly:

- **Write-surface iterations** (model YAML, product-owned frontend,
  seeds/fixtures, copy): autonomous Class A/B path; minutes to hours from
  feedback to preview.
- **Everything else** (policy, auth, classification, dependencies, generated
  templates, integrations, CI): escalates to the engineering harness as
  Class C/D; the stakeholder sees waiting-for-engineering, typically within
  one working day.
- **Accelerated** permits focused proof and a signed synthetic-data branch
  preview, not main/release claims. **Integration Candidate** requires all
  candidate gates on one exact SHA. **Destination Verified** requires the
  exact merge-SHA pipeline. **Release Candidate** adds strict production
  evidence for the Accepted immutable digest. **Promoted** requires both
  `Accepted` and `release_ready:true`, plus named human authority, and deploys
  that same certified digest without rebuilding it.

Failure paths are first-class: validation or test failures retry within the
runner's lease; review NO-GO returns to planning with findings attached;
budget exhaustion pauses the product visibly (auto-pause is the anti-fad
stop rule made mechanical); a non-automatable prerequisite raises a
human-owned required action and parks dependent Deliverables as waiting for
another team (next section).

## Human-Owned Required Actions

Not every prerequisite can be automated, and some must not be. The canonical
example: making a new product work in dev requires a provisioned dev
database populated with mock/test data against the current production
schema. The factory models this as first-class human-owned work rather than
a silent stall:

1. Intake prediction or a blocked runner raises a typed
   `factory_required_action@1` record in the product repository: category
   (environment provisioning, data fixture, source schema export,
   credential/secret, access, approval, enterprise registration such as
   CMDB matriculation), what is needed and why, blocked
   Deliverables, owning engineering Workstream, data constraints, and the
   evidence that clears it.
2. The record is carried as a work-system Task or Deliverable in "Waiting for
   another team," routed to the owning team through the normal execution
   adapter (Jira mapping in the
   [Program Work System](../reference/program-work-system.md)), and
   projected on the app workspace with owner, blocking scope, and aging.
   Admission (G-F3) holds dependent Deliverables until it clears; aging
   items escalate through the Flow Controller function.
3. Fulfillment attaches evidence, not prose — a platform-secret reference
   name, an applied migration artifact, or an approved schema-only export
   in `.appfw/source-evidence/` (classification-screened via G-F2;
   production data never moves to dev).
4. The split of labor stays clean: humans provision infrastructure and
   approve exports; agents generate synthetic seed data and migrations
   inside the write surface and populate the environment through the
   normal migrate-and-seed path.

The request detail therefore lives in the product repository (the source of
truth) and in the owning team's queue; the dashboard is where everyone sees
it, ages it, and knows whose move it is.

## Fabric Dashboard And Per-App Workspace

The dashboard is a client and authorized projection, never a planning,
execution, build, deployment, acceptance, or release authority. The hosted
surface runs in the dev Kubernetes environment. It follows the accepted
dashboard doctrine in the
[Product Management Strategy](../strategy/app-framework-product-management-strategy.md),
the [Program Work System](../reference/program-work-system.md), and the PDS
component/quality contracts. A separately developed Product Dashboard may
contribute information architecture and frontend assets only after its branch
is accepted and contract compatibility is verified; it is not an architectural
dependency, and its single-checkout static runtime is not the hosted
multi-application service.

The fleet view searches applications by identity, purpose, owner, lifecycle,
management tier and authorized observed attributes. The app workspace shows:

- accepted Purpose/North Star, strategy, Product Increment, Deliverable, Task,
  Assignment and acceptance context with exact source commit — for product
  apps, the bounded `product_work_model@1` profile (purpose, strategy,
  product increment, task), authored in the workspace and recorded in the
  product repository;
- portfolio theme participation — journeys, processes, experiences — from
  `portfolio_model@1`;
- framework lineage and support-window currency computed from `appfw.lock`
  against published framework releases, with upgrade-campaign tasks projected
  when a release affects the app;
- build, promotion and deployment history correlated by SHA and immutable
  digest;
- four-part composition: desired, built, deployed and running,
  platform-qualified per declared surface;
- authorized dev/prod links, direct links to the app's observability and
  monitoring dashboards (Grafana and the logs/traces entry points — linked
  out, never re-rendered), and source/evidence links;
- for declared native-mobile surfaces: store release state (track, review,
  staged rollout), live version adoption and skew, crash-free rate, and the
  governed minimum-supported-version against the API compatibility window;
- usage and health, separately sourced satisfaction, separately metered
  build/runtime AI cost, and the whole-cost (TCO) rollup — the AF-M08.3
  taxonomy extended with runtime infrastructure and operations components —
  with ROI computed against the G-F1 value hypothesis, never hand-entered;
- fleet-deduplicated `framework_request@1` demand records raised from this
  app (component requests, improvements, defects, upgrade blockers, docs
  gaps), recorded in the requesting product's repository and dispositioned
  through the framework's own program intake; and
- Required Actions, WIP/budget/pause, source, observation time, freshness and
  honest `UNKNOWN`/`STALE` states.

The experience uses progressive disclosure for four different jobs: an
executive portfolio overview, a product owner's outcome/workspace view, an
engineer's evidence and action view, and an architect's composition/dependency
view. It must satisfy PDS accessibility and product-quality gates and pursue
AF-OG05's calm, precise signature quality; visual appeal never suppresses
freshness, provenance, uncertainty, risk, or required action.

At any context level, users can ask/explain, draft a typed change, submit a
proposal, request execution, or accept/reject if their role permits. The
conversation displays the controlling source version and does not become a
controlling record. App ownership is scoped to that app and does not imply
merge, release, risk, classification, secret, dependency, or cross-app
authority.

## Hosted And Local Adapters

- Hosted mode uses Okta OIDC and typed factory APIs. Browser clients never
  receive repository, cluster, registry, model-gateway, or release
  credentials.
- Local mode runs the same UI and schema versions against one explicitly
  selected workspace through the App Framework CLI. It labels branch,
  working-copy/dirty state and remote observation freshness.
- Local accelerated execution uses the same Assignment, profile, handoff,
  review and write-root rules. It has no cluster or production-release
  credentials.
- The local service binds to loopback only; validates `Host` and `Origin`;
  enforces CSRF and an ephemeral, bounded-lifetime session capability; never
  exposes repository credentials to browser code; and requires explicit
  checkout/write confirmation with canonical-path and symlink-escape checks.
- Global fleet data continues to come from the hosted read API. Offline or
  unreachable facts render `UNKNOWN`/`STALE`; local state is not another
  portfolio authority.
- The hosted control plane cannot initiate arbitrary execution on a
  workstation.

## CI/CD

Product repositories run the canonical loops (see the
[Deployment Reference](../release/deployment-reference.md) and
[Release Gate](../release/release-gate-ci-cd.md)); the factory adds
server-side enforcement because runners push via API:

```bash
scripts/appfw product validate --json
scripts/appfw product generate --check --json
scripts/appfw product test --fast --json
scripts/appfw product handoff --json
```

- Accelerated branch proof: the loop above, current independent review,
  `scripts/appfw product frontend-test --json` plus PDS catalog/component
  checks when UI is in scope, draft PR, and signed synthetic-data preview.
  Deferred gates and forbidden claims remain explicit.
- Integration candidate: every deferred candidate gate, exact-SHA current
  review, and required PR pipeline before human merge.
- Destination verified: exact merge-SHA destination pipeline, image SBOM,
  signature/provenance and immutable dev deployment.
- Release candidate: for the exact Accepted source/digest, the unchanged
  strict evidence path adds live providers, security, operations,
  accessibility, rollback and exact-digest proof. Human promotion requires
  both `Accepted` and `release_ready:true` and deploys that same certified
  digest through GitOps. The factory has no promotion authority.
- The factory subsystem's own repo carries the same discipline plus
  GitOps-managed manifests and kill-switch drills.

## Security Architecture

Threat framing follows the
[Agentic Threat Model](../architecture/concerns/agentic-threat-model.md);
the factory's distinctive exposures and controls:

| Exposure | Control |
| --- | --- |
| Prompt injection attempts infrastructure action | Typed least-authority tools; execute-with-approval for irreversible operations; idempotency keys; no raw shell anywhere in the conversation path |
| Malicious website reaches local write API | Loopback-only bind; strict Host/Origin and CSRF checks; ephemeral bounded session; explicit checkout/write confirmation; canonical-path/symlink checks; no browser-held repo credential |
| Unvetted citizen artifacts (PHI, secrets) | G-F2 ingestion screen before anything enters a repo; synthetic-only runners and previews; prompt-audit redaction |
| Agent writes outside its mandate | G-F4 mechanically enforced write surface; boundary checks; escalation, not silent expansion |
| Cross-product access | Per-product namespaces, service accounts, repo-scoped tokens, network policy; MCP authorization checks product membership per call |
| Supply chain | Pinned registry packages with `appfw.lock` provenance; image SBOM; signed digests; ArgoCD deploys signatures only |
| Runaway autonomy or spend | Fleet WIP + per-product budgets (G-F3); auto-pause; factory-wide and per-product kill switches (G-F11) with drill evidence |
| The factory itself | Built as an App Framework product: same review, CI, release evidence, and observability obligations as anything it produces |

## Observability

- One correlation id from conversation to preview: conversation, intake,
  Product Increment/Deliverable/Task, Assignment, run, PR, pipeline,
  deployment, dashboard generation.
- OpenTelemetry traces across factory-api, planner, orchestrator, and runner
  stages; previews keep the standard product observability contract
  (`/health/*`, `/metrics`, traces, provider timings).
- Fleet metrics: intent-to-preview and feedback-to-iteration cycle time,
  queue/review age, WIP, cost per product and per accepted result,
  write-surface escalation rate, incidents and rollbacks.
- Build-agent token/cost observations are separated from runtime AI
  feature/operation token/cost observations. Both correlate model, retries and
  outcomes; neither carries raw prompts in general telemetry or serves as a
  team-value leaderboard.
- Usage cannot imply satisfaction. Satisfaction adapters retain their approved
  source, cohort/sample, period and freshness; apply field authorization and
  minimum-cohort suppression; and omit respondent text unless approved
  redaction, retention, access and deletion controls govern it.
- Prompt and tool-call audit to a named sink with SIEM export and retention
  per the chat audit posture; no PHI or prompt bodies in general logs.
- Program truth never comes from telemetry: product records remain
  authoritative for product decisions, and Grafana remains the operations
  surface.

## Phased Rollout

| Phase | What runs | Graduation evidence (G-F10) |
| --- | --- | --- |
| 0 — Concierge and read-only spine | Engineers and supervised agents ratify contracts, register CRM, reconcile its composition/history, and operate one proposal-to-dev loop by hand | Source/claim and hosted/local schema parity; cross-app denial; exact-digest CRM evidence; loop friction documented |
| 1 — Supervised live loop | The separately authorized Hosted Product Factory subsystem runs write-surface iterations (Class A/B) alongside the read/project management plane, with batched engineer review for a second independently owned consumer | Zero write-surface escapes; adversarial MCP tests; WIP/budget enforcement; kill-switch drill; whole-cost telemetry; reusable two-product contracts |
| 2 — Self-service intake | Citizens submit intakes behind G-F1 only after the second-use economic and quality gate; idea-stage self-service bootstrap opens separately once G-F13 enforcement is mechanical | Bootstrap idempotency; identity design review with the IdP team; review-capacity and cost evidence set the fleet ceiling |

## Open Engineering Questions

- **Review capacity is the real throughput ceiling.** Two-speed routing was
  designed for a two-producer repo; a fleet multiplies Class A/B volume.
  Decide whether cross-model adversarial review plus sampled human audit can
  carry the cheap path at fleet scale before raising WIP.
- **Initial conversation-client authentication** into the provider-neutral
  factory MCP (claude.ai is the first candidate; Okta federation, token
  audiences, consent UX) needs a design pass with the identity team before
  Phase 2.
- **Preview URL access control**: previews will be shared around the org;
  confirm oauth2-proxy group gating and link expiry expectations.
- **Cost attribution**: token and compute metering per product and per
  accepted result needs an approved authority before Phase 1 exit.
- **Stable application identity — decided 2026-07-25**: the fabric mints
  `app_id` at registration; the repository and, after matriculation, the
  enterprise CMDB reference are cross-references. Still do not add
  portfolio/tier/permission/runtime fields to manifest v1.
- **Source adapters and retention**: name the enterprise registry, product
  analytics, satisfaction, model-gateway and cost authorities plus their
  freshness, retention and deletion contracts before AF-D08.1a exits. For
  applications declaring native-mobile surfaces, also name the store console
  access, the approved mobile build/distribution service, and the
  crash/device-health source before mobile observation adapters build.

## Non-Claims

This architecture makes no claim that any component exists today. It does
not alter the protected readiness window, the Outcome Goal Register, release
authority, or the engineer golden path. Previews are Prototype-stage work
products, not releases. The factory is not a PaaS, not a tenant control
plane, and not an employee front door. Product runtimes do not depend on the
management plane; a registry, projector, dashboard, agent or factory outage
must not interrupt product traffic.
