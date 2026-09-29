# Hosted Product Factory Spec

Status: shaped-and-held target-state specification. Bounded-lane
Assignments are admitted per the recorded 2026-07-25 D1 decision (Option B:
harness as implementer, FAB-A1 first); all other implementation remains
held, and AF-OG08/AF-OG10 posture is unchanged in the
[Roadmap Outcome Goal Register](../release/roadmap.md#outcome-goal-register).
This spec exists so the factory is extracted from real product use instead
of invented ahead of it.

Spec depth: full

Owner roles:

- Product Owner: factory value, admission rubric, acceptance criteria.
- Strategist/Program Product Manager: demand-stream fit, activation decision,
  register disposition.
- Chief Architect: plane boundaries, MCP tool contract, identity model,
  reasoning architecture.
- Program Flow Controller: admission, WIP/budget governor semantics.
- Integration Branch Manager: CI, review routing, and merge gate semantics.
- Security owner: threat model, identity, PHI controls, audit posture.
- Implementation owner: the harness, for bounded-lane Assignments admitted
  per the 2026-07-25 D1 decision; otherwise none (held).
- Review owner: Framework PR Review Agent plus human reviewer.

## Business Value

Today a business stakeholder with a validated app idea either vibe-codes a
prototype and hands it to engineering, or waits in a delivery queue. The
[PoC To Enterprise Product Intake](../lifecycle/intake-and-discovery.md) path
already turns that prototype into a governed product, but it assumes a local
framework checkout and an engineer-adjacent agent operator.

The Hosted Product Factory is the governed code-changing subsystem connected
to the
[Enterprise App Fabric's application management
plane](../strategy/enterprise-app-fabric.md#application-management-plane).
The separate management-plane dashboard gives stakeholders a searchable fleet
catalog and an application workspace. The stakeholder can inspect purpose,
work, evidence, composition, environments, outcomes, and cost; chat within a
named product context; and submit typed feedback or change proposals to the
factory subsystem. Beyond the bounded G-F13 idea-stage trial allowance, it is
after named-sponsor approval and admission that the factory can create or use
a governed Bitbucket product repository, drive the documented
intake-to-generation loop in ephemeral Kubernetes runners, deploy a
permissioned preview, and route the result through existing review, CI,
acceptance, and release authorities.

Value hypothesis to test: reduce measured business-intent-to-reviewable-product
cycle time from the current product baseline toward hours-to-days while every
artifact stays inside the existing evidence, review, and release discipline.
AF-D08.1a must ratify the baseline, measurement window, and target before any
improvement claim. This serves demand stream 1 (citizen-developed
applications) and product themes AF-PT02 (productization factory), AF-PT03
(packaged platform consumption), and AF-PT07 (signal intake) in the
[Product Management Strategy](../strategy/app-framework-product-management-strategy.md).

## Problem

- The golden path is paved but not self-service: only engineers or
  engineer-supervised agents can operate it, so citizen demand queues on
  engineering capacity.
- There is no governed surface where a stakeholder can submit intent,
  feedback, and priority changes; those arrive as chat transcripts, decks,
  and email, losing provenance the work system requires.
- Nothing creates product repositories: `scripts/appfw product new` scaffolds
  a local directory and remotes are wired by hand, so repo bootstrap quality
  (branch protections, CI, day-zero intake record) depends on who does it.
- The program dashboard artifacts under `target/appfw/` serve the framework
  program; a product stakeholder has no per-product view of Product
  Increments, Deliverables, Tasks, stage, or the Implemented-versus-Accepted
  gap they own.
- There is no fleet-wide, source-bound view that answers which applications
  exist, who owns them, what runs in each environment, which App Framework,
  design-system, provider, package, schema, base-image, or agent-runtime
  versions shaped them, or whether those observations are current.
- Build, promotion, deployment, usage, satisfaction, backlog, and AI-cost
  facts live in different authoritative systems. Without an explicit
  authority map, a large dashboard database would quickly become stale
  parallel truth.
- Without a designed admission and autonomy boundary, a naive "chat creates
  apps" implementation would produce orphaned-app sprawl, unattributable
  actions, and prompt-injection-reachable infrastructure.

## Goals

- Define the target architecture: a Conversation Plane, a Fabric Management
  Control Plane on Kubernetes, a separately authorized Hosted Product Factory
  subsystem, per-product Product Planes, and a source-bound fleet projection
  with authority assigned by fact class.
- Define a searchable fleet catalog and one workspace per application,
  including registration and management tiers, exact component composition,
  environment history, product context, work and evidence, outcome telemetry,
  and honest freshness.
- Define approved conversation-agent-to-factory communication as a
  provider-neutral, typed, audited, idempotent MCP tool contract with no
  shell, repository, or cluster escape; Claude may be the initial client but
  is not an architectural dependency.
- Keep repository creation an execute-with-approval operation, never a direct
  chat side effect: bounded idea-stage bootstrap under G-F13 quota and posture
  controls, with G-F1 sponsorship gating idea → planned graduation, CMDB
  matriculation, and budgets beyond a trial allowance.
- Bound autonomous iteration to a mechanically verifiable write surface and
  route everything else through the existing two-speed review harness.
- Define each per-app workspace as a regenerated projection that shows
  Outcomes, Product Increments, Deliverables, Tasks when sourced, stage, and
  Implemented versus Accepted, and that captures stakeholder acceptance,
  feedback, and priority proposals as typed records.
- Use the same versioned read and command contracts in the dev-Kubernetes
  service and a local developer mode without making a workstation another
  source of fleet truth or a remotely executable target.
- Separate accelerated branch-preview proof, integration-candidate proof,
  destination-main verification, release-candidate proof, and human-authorized
  promotion so no chat or dashboard action can imply release readiness.
- Define identity, authentication, observability, CI/CD, and policy-gate
  requirements so security and platform engineering can review the design
  before any build.
- Stage autonomy in three phases with explicit graduation evidence.

## Non-Goals

- Not a general PaaS, widget marketplace, or commercial SaaS-grade tenant
  control plane (deferred by AF-M10.4 and the current investment authority).
- No production promotion authority. The factory can never bypass release
  gates, strict release evidence, or the PDS security baseline; previews are
  Prototype-stage by definition.
- No parallel planning system. The
  [Program Work System And Glossary](../reference/program-work-system.md)
  stays canonical; dashboards remain projections.
- Not a replacement for the enterprise application registry/CMDB, Bitbucket,
  Jira, ArgoCD, Kubernetes, artifact registries, Grafana, product analytics,
  survey systems, or their owning authorities. The management plane federates
  and explains; it does not re-author their facts.
- Registering or observing an application grants no factory control, runtime
  data access, production access, merge authority, release authority, or
  accepted-risk authority. Those are separate, explicit grants.
- No product business data or product traffic flows through the management
  plane. An outage of the factory or dashboard must not impair an application
  runtime.
- No live SaaS credentials, PHI, or production enterprise data in factory
  runners or preview environments. Synthetic and approved fixture data only.
- Not a new employee front door. The factory serves product creation, not
  end-user traffic; fabric rules in the
  [Enterprise App Fabric Strategy](../strategy/enterprise-app-fabric.md)
  are unchanged.
- Does not replace the engineer golden path or Claude Code/Codex local use.
- Does not release an implementation Assignment, alter the protected readiness
  window, or reorder the Outcome Goal Register.

## Scope

In scope: this spec, the companion
[App Fabric Management Plane And Hosted Product Factory
Architecture](../architecture/hosted-product-factory-architecture.md),
the fleet registration, observation, composition, and command-proposal
contracts; the factory MCP tool contract shape; the product-repo
program-contract and feedback record shapes; hosted and local adapter
boundaries; gate definitions; identity model; observability and CI/CD
requirements; and phased rollout conditions.

Out of scope until activation: factory code, cluster manifests, MCP server
implementation, dashboard implementation, Bitbucket provisioning automation,
and any change to roadmap posture, priority order, milestone meaning, or
activation (owed to the Program Product Manager/human disposition). Recording
the human-directed, non-activated Deliverable decomposition in the existing
AF-M08.1-AF-M08.3 and AF-M10.5 homes is in scope and releases no Assignment.

## Repository Context

The implementation must respect, reuse, and not fork:

- Intake path and agent contract:
  [PoC To Enterprise Product Intake](../lifecycle/intake-and-discovery.md)
  (`product new --profile product-intake`, `analyze`, `propose-model`,
  `model-status`, `scaffold-model`, review ledger, sign-off rules).
- Product lifecycle:
  [Product Developer Golden Path](../lifecycle/product-golden-path.md) and
  [Application Lifecycle](../lifecycle/application-lifecycle.md).
- Work vocabulary, stages, acceptance:
  [Program Work System And Glossary](../reference/program-work-system.md).
- Harness governance: [Agentic And Human Operating Model](../start/agentic-human-operating-model.md),
  [Agent Role Cards](../start/agent-role-cards.md) (two-speed review routing,
  standing push approval limits, pre-push guard), and
  `scripts/check-program-flow.mjs` (deterministic-first observer pattern).
- Deployment and runtime posture:
  [Deployment Reference](../release/deployment-reference.md) (backend image,
  oauth2-proxy/Okta sidecar, ArgoCD immutable digests, chat/MCP fail-closed
  environment contract) and
  [Release Gate](../release/release-gate-ci-cd.md).
- Experience contract: [Frontend Starter Contract](../frontend/product-frontend.md)
  and [PDS Health Design System](../frontend/pds-health-design-system.md)
  (approved component catalog, tokens, agent recipes, and UI quality gates).
- Threat models: [Agentic Threat Model](../architecture/concerns/agentic-threat-model.md)
  and [Security Threat Model](../architecture/concerns/threat-model.md).
- Bitbucket REST access: [Bitbucket REST Auth Runbook](../start/bitbucket-rest-auth.md)
  (the only approved credential path for repository automation).
- Package-first substrate: AF-OG02 milestones (disposable external workspace,
  package-only creation). Factory runners depend on this graduating; runners
  must never require a framework checkout.

## Contracts Touched

| Contract Surface | Expected Change | Counterpart Surfaces That Must Stay Aligned |
| --- | --- | --- |
| Hosted Product Factory MCP (new) | New provider-neutral typed tool contract with versioned schemas, OAuth 2.1 authorization, idempotency keys, and full audit, isolated from the non-code-changing Fabric Management Control Plane | MCP governance rules in the Product Management Strategy; agentic threat model; product runtime MCP posture in the Deployment Reference |
| Bitbucket automation | New repository-provisioning operation using the repository-owned auth path, executed only after the applicable approval (G-F13 idea bootstrap or G-F1 graduation) | Bitbucket REST Auth Runbook; role cards (standing approval scope is unchanged and never includes repo creation) |
| Product repository layout | Adds a tracked program-contract file (`product_work_model@1`: purpose, strategy, product increment, task — a bounded subset profile of the Program Work System) and typed stakeholder feedback records under `.appfw/`; authored from the workspace, recorded in the repository, projected by the fabric. The profile maps onto the canonical hierarchy — purpose stands as the product's single Outcome, and each profile task is a canonical Deliverable with execution Assignments bound to it directly — so Deliverable-burndown (G-F9) and admission semantics apply unchanged | Product workspace contract and boundaries; program work system |
| Fleet application registry (new) | `fabric_app_registration@1`: fabric-minted stable app id, origination path, purpose summary and authority link, owner group, lifecycle state (idea, planned, active, retired), declared platform surfaces, repository/environment references, classification, management tier, and a nullable enterprise CMDB reference bound at matriculation | Approved enterprise registry/CMDB boundary; product repository; identity and authorization policy |
| Portfolio and lineage model (new) | `portfolio_model@1`: enterprise themes (journey, process, experience) with named owners, portfolio strategies, and app participation mappings — one level, no work hierarchy; framework releases enter as `fabric_observation_event@1` observations so per-app framework lineage and support-window currency are computed projections | Enterprise App Fabric strategy; program work system; versioning and compatibility matrix; ProGet distribution |
| Application composition (new) | `app_component_snapshot@1`: separately records desired, built, deployed, and running source/component identity with exact SHAs, versions, digests, provenance, observation time, and freshness — platform-qualified per declared surface; native-mobile states carry signed store build, OTA bundle, store-release (track, review, staged rollout), and live version-distribution identity | App manifest and `appfw.lock`; dependency locks; signed SBOM/provenance; GitOps and runtime observations; store consoles and the approved mobile build service |
| Fleet observation envelope (new) | `fabric_observation_event@1`: append-only, source-attributed build, promotion, deployment, framework-release, usage, outcome, satisfaction, value-realization, whole-cost (TCO), and mobile store-release/version-adoption/crash-free/store-rating observations with correlation and integrity metadata — the TCO kinds are the AF-M08.3 whole-cost taxonomy extended with runtime infrastructure and operations components; per-app TCO and ROI-against-value-hypothesis views are computed projections, never hand-entered claims | Bitbucket, ArgoCD/Kubernetes, registries, OpenTelemetry, approved analytics and feedback sources; store consoles; AF-M08.3 whole-cost taxonomy |
| Framework demand feedback (new) | `framework_request@1`: typed product-to-framework signal (component request, improvement, defect, upgrade blocker, docs gap) raised from an app context with a link to the requesting app's work item; recorded as a typed record in the requesting product's repository (pre-repo, the approved intake authority), with fleet deduplication and demand counts as computed projections; raised initially as a `framework_request` type on `submit_feedback`, with a dedicated typed tool considered at AF-D08.2a; dispositioned by the framework Product Owner through the framework's own program intake — never a parallel framework backlog | Roadmap product-signal-intake control; program work system; factory MCP |
| Factory command proposal (new) | `fabric_command_proposal@1`: typed, context-bound proposal with intended effect, authority, evidence, idempotency, and disposition; chat text is never executable authority | Program work system, factory MCP, admission, role cards, review and release gates |
| Factory run manifest (new) | `factory_run_manifest@1`: exact source/profile, commands, executed/deferred gates, runner/model/cost identity, review, evidence, result and correlation | Delivery profiles; handoff/review contracts; Bitbucket and artifact provenance |
| Program work system | Consumed as-is: Outcome, Product Increment, Deliverable, Task, stage, Implemented versus Accepted; retained Tranche/Slice fields remain compatibility aliases | `docs/reference/program-work-system.md` remains canonical |
| Preview deployment profile | Reuses the documented product topology with a factory preview posture (auth on, synthetic data, Prototype badge) | Deployment Reference; release gate |
| Product UI surfaces | All factory-produced UI — product SPA, declared native-mobile surfaces, and the fleet/per-app management experience — composes only from the approved PDS design system catalog and recipes | Frontend Starter Contract; PDS Health Design System; PDS catalog/component checks |
| Roadmap register | Held decomposition spans AF-OG08 product-factory proof and AF-M10.5 portfolio lifecycle operations; disposition and any activation remain with the Program Product Manager and named human authorities | `docs/release/roadmap.md` |

## Options Considered

| Option | Pros | Cons | Decision |
| --- | --- | --- | --- |
| A. Four connected surfaces with a separate management plane and factory subsystem, typed MCP, sponsor-gated bootstrap, and bounded autonomy | Self-service demand stream 1; provenance end to end; reuses every existing gate | New tier-1 management and factory workloads to own; review capacity becomes the ceiling | Selected as target shape |
| B. Status quo: citizen hands PoC to engineers who run the intake path locally | No new surface; already documented | Queues on engineering; no stakeholder feedback surface; does not scale | Remains the engineer path; rejected as the citizen-facing answer |
| C. Route citizen apps to a vendor low-code platform | No build | Fails the framework decision rule for governed, evidence-gated, PDS-owned products; vendor lock and per-app licensing | Remains available where the decision rule routes there |
| D. Fully autonomous factory without human gates | Fastest demo | Violates staged autonomy, OWASP agentic risk guidance, sponsorship rubric, and register evidence discipline | Rejected |

## Decision Provenance

| Date | Owner | Decision | Evidence / Rationale | Revisit Trigger |
| --- | --- | --- | --- | --- |
| 2026-07-21 | Human owner | Shape the hosted factory as a held target-state spec; release no Assignment | Register sequences the factory after product use (AF-M08.2); protected readiness window is active | AF-OG07 activation or explicit AF-OG08 pull |
| 2026-07-21 | Chief Architect function | Three-plane architecture; typed-tools-only MCP; sponsor gate before repo creation; bounded autonomous write surface; dashboard as projection | This spec and the companion architecture document | First factory evaluation evidence contradicting a boundary |
| 2026-07-25 | Human owner | Consolidate the fleet registry, per-app workspace, hosted/local dashboard, contextual agent collaboration, and governed promotion concept into the current held App Fabric and factory plans; do not create a competing dashboard program | Human design dialogue; Enterprise App Fabric strategy; this spec; AF-OG08/AF-OG10 roadmap homes | Explicit activation decision or evidence that the decomposition does not support the first complete product loop |
| 2026-07-25 | Architect function (held target) | Separate the non-code-changing Fabric Management Control Plane from the code-changing Hosted Product Factory subsystem as distinct workloads, while allowing a shared repository, cluster, release train and operating team; require Accepted plus `release_ready:true` before promotion | Adversarial architecture review against role cards, work-system acceptance, delivery profiles, and remote-promotion semantics | Activation design review or implementation evidence shows the separation is unsafe or uneconomic |
| 2026-07-25 | Human owner | Registry is the system of origin: fabric-minted `app_id`, origination paths, lifecycle states with idea-stage repositories under G-F13 controls, G-F1 moved to idea → planned graduation, and CMDB matriculation at planned with a mechanical CI gate before promotion beyond dev | Second human design dialogue; CMDB hygiene and the candidate-system pattern; environment ladder dev → test → stage → production | Enterprise registry owner objects, or idea-stage sprawl exceeds quota and dormancy controls |
| 2026-07-25 | Human owner | Management-plane persistence: PostgreSQL provider, fabric console built as an App Framework product, three-class state (audited decisions ledger, append-only observations, disposable rebuildable projections), factory database separated with its own credentials | Same dialogue; ledger workload fits relational constraints; provider is release-gate certified | FAB-A1 contract review contradicts the model |
| 2026-07-25 | Human owner | Per-app `product_work_model@1` (purpose, strategy, product increment, task) authored in the workspace and recorded in the product repository; `portfolio_model@1` themes held to one level; framework lineage with computed support-window currency and factory-run upgrade campaigns | Same dialogue; Program Work System stays canonical; AF-OG02 behavior-affecting upgrade seeds the campaign proof | Product teams need more than the subset profile, or themes grow work hierarchies |
| 2026-07-25 | Human owner | Product-to-framework demand loop (`framework_request@1` signals dispositioned through the framework's own program intake, deduplicated into fleet demand counts) and whole-economics accommodation: TCO, value-realization, and satisfaction observation kinds with computed per-app TCO/ROI projections measured against the G-F1 value hypothesis | Same dialogue; roadmap product-signal-intake cross-cutting control; AF-M08.3 whole-cost taxonomy and satisfaction privacy contract | Demand records become a parallel framework backlog, or economics reporting never drives an invest/retire decision |
| 2026-07-25 | Human owner | Mobile is a first-class managed platform surface for product apps: registration declares surfaces; composition, observation, and lineage are platform-qualified with store-release and version-skew semantics; OTA-versus-store-track promotion recorded with a human submit; store/build/crash authorities join the D2 worksheet; workspaces link directly to each app's observability dashboards; factory mobile execution stays gated behind AF-M05.2 and native-generator graduation | Mobile follow-up in the same design dialogue; contracts are cheaper platform-aware now than retrofitted at v2 | AF-M05.2 evidence contradicts the seams, or mobile observation duplicates the store consoles |
| 2026-07-25 | Human owner | D1 activated as bounded Option B with the harness as the implementer: pre-prod, no staffed team exists, so FAB-A1 executes in-harness by direct human direction under the existing gates; roadmap posture for AF-OG08/AF-OG10 unchanged | Third design dialogue; the agent-operable-framework commitment applied to the factory's own contracts | Harness capacity displaces AF-OG01–03 work, or a plan stop rule trips |

## Architecture And Implementation Notes

The companion
[App Fabric Management Plane And Hosted Product Factory
Architecture](../architecture/hosted-product-factory-architecture.md)
carries the unifying diagram, component breakdown, and flows. The normative
rules live here.

### Planes, management surface, and fact authority

- **Conversation Plane.** An approved enterprise conversation agent (Claude
  may be the initial client) fronts citizens, sponsors, and engineers. It
  holds no authoritative state and has no direct repository, cluster, or
  shell access; it acts only through the factory MCP tools.
- **Fabric Management Control Plane.** A Kubernetes-hosted registry service,
  projection reconciler, component index, authorized dashboard read API, and
  observation/decision audit. It federates fleet facts and serves the
  management experience; it has no code-writing, merge, release, cluster
  mutation, or product-runtime authority.
- **Hosted Product Factory subsystem.** A separately authorized MCP and
  approvals API, proposal/work queue, admission and WIP/budget governor,
  planner, runner orchestrator, review router, run audit, and kill switch. It
  consumes stable app/context references from the management plane and returns
  source-bound delivery observations. The two subsystems may initially share
  a repository, Kubernetes cluster, release train, observability stack, and
  operating team for cost and operability, but deploy as separate workloads
  with distinct APIs, service identities, authorization policy, credentials,
  data stores, failure domains, and kill switches.
- **Product Planes.** One isolated preview namespace per factory-managed
  product in the dev Kubernetes environment: the preview deployment
  (documented product topology, auth on, synthetic data) and its preview
  telemetry. Production and other runtime environments are separately
  deployed, governed, observed, and linked; they do not depend on either
  control-plane subsystem to serve traffic.
- **Management Experience.** The dashboard is a hosted fleet catalog and
  per-app workspace backed by versioned read and command APIs. Local mode uses
  the same contracts for one selected checkout. It is a client and projection,
  not a fourth authority plane.

Authority is assigned by fact class; there is no single database that becomes
truth merely because the dashboard can query it:

| Fact class | Authority | Projection rule |
| --- | --- | --- |
| Global app identity, accountable owner, canonical repository, lifecycle and management tier | Fabric registry (system of origin: audited G-F12 identity, lifecycle, and tier decisions); the enterprise CMDB reference is authoritative for enterprise operations after matriculation | Store stable references and decision history; never infer control rights from ownership |
| Purpose, North Star, strategy, work hierarchy, acceptance criteria and product decisions | Accepted product-repository records | Show exact accepted commit and distinguish proposed, Implemented, and Accepted |
| Desired topology and framework provenance | `.appfw/manifest.yaml`, `appfw.lock`, dependency locks and accepted source SHA | Label `desired`; do not present it as built or running |
| Pull request and build | Bitbucket | Correlate exact source/destination SHA and pipeline id; stale remote state is `STALE`/`UNKNOWN` |
| Built artifact and component versions | Signed image/package provenance, SBOM and artifact registries | Label `built`; retain immutable digest and signature status |
| Desired deployment | GitOps repository | Label environment and desired immutable digest |
| Deployed and running state | ArgoCD/Kubernetes observation and health telemetry | Label `deployed` or `running`; retain observation time and freshness |
| Native-mobile built, store-release and device-fleet state | The approved mobile build/distribution service (signed build identity); the app store consoles (submission, review, track, staged rollout); the approved crash/device-health source | Platform-qualified snapshot states; store facts labeled with their console source, observation time and freshness |
| Usage and operations | Sanitized product analytics and OpenTelemetry | Aggregate under approved retention; never infer satisfaction or acceptance |
| Satisfaction | Approved survey/feedback source | Enforce field authorization and minimum-cohort suppression; show source, cohort/sample, period and freshness; exclude or separately govern free text; never derive it from usage alone |
| Build-time and runtime AI usage/cost | Factory runner and approved model-gateway meter | Separate build from runtime, aggregate by app/run/outcome, and exclude raw prompts |
| Merge, acceptance, release and risk decisions | Existing named human authorities and their retained decision records | Project the decision; never synthesize authority from a green check or agent confidence |

Every projected fact carries its authority reference, exact SHA/digest when
applicable, observed time, freshness, and correlation id. Missing or expired
observations fail closed to `UNKNOWN` or `STALE` and cannot advance a gate.
The management and factory subsystems may hold a query projection, queues,
idempotency records, and leases, but never re-author product plans, build
truth, deployment truth, or human decisions.

For factory-managed products, typed proposals and every disposition persist
immediately in the product repository's governed proposal records. Before a
repository exists, an approved append-only intake/proposal authority retains
the same provenance. The factory work queue holds references, idempotency
state, and leases only; rejection, deferral, or supersession is never allowed
to erase the proposal history.

### Registration and composition

`fabric_app_registration@1` binds a fabric-minted stable `app_id` to a
purpose summary, accountable owner group, origination path (citizen, product
management, legacy modernization, engineering), canonical repository,
lifecycle state, environment references, declared platform surfaces (web,
native mobile with iOS/Android targets), data classification, support
contact, one management tier, and a nullable enterprise CMDB reference bound
at matriculation.
For managed products, purpose and detailed product context remain
repository-owned; the registration stores the reference and a searchable
summary. A registered-only application may have fewer authorities connected,
which the workspace renders as missing rather than inviting manual imitation.

Management tiers are cumulative capabilities:

- `registered`: identity, purpose summary, owner and lifecycle references;
- `observed`: read-only source adapters and freshness-aware history;
- `appfw_managed`: App Framework lifecycle, upgrade, compatibility and
  evidence contracts; and
- `factory_managed`: context-bound typed proposals may enter the governed
  factory loop.

Tier changes are authenticated registry decisions under G-F12. They are not
release, maturity, security, or risk ratings.

Lifecycle is a separate dimension from tier and from environment presence.
For fabric-born applications the registry is the system of origin — the
candidate system, the way a recruiting system precedes the HR system of
record:

- `idea`: fabric-registered with a repository and dev-environment presence
  only; not in the enterprise CMDB. Idea-stage creation is bounded by G-F13
  (owner quota, dev-only and synthetic-only posture, dormancy auto-archive,
  G-F2 screening) and carries at most a small trial execution allowance.
- `planned`: the G-F1 sponsorship decision graduates the idea, and
  matriculation happens here: the enterprise CMDB configuration item (CI)
  is created as an audited registry decision — raised as a human-owned
  required action until a certified governed write path automates it — and
  the CMDB reference is
  authoritative for enterprise operations from then on.
- `active`: build, iterate, and operate across environments. Promotion
  beyond dev (`dev → test → stage → production`) mechanically requires the
  CMDB CI to exist, which catches any escape past a skipped planned
  decision; CI status tracks the stage.
- `retired`: environments reclaimed and the CMDB CI retired; the fabric
  retains the full lineage and history.

Pre-existing applications register in the other direction: they arrive
already carrying their enterprise CMDB reference, which the registry stores
as an authority link from day one.

`app_component_snapshot@1` prevents an ambiguous "application version." It
correlates four separately evidenced views:

1. **Desired:** accepted source SHA, manifest, `appfw.lock`, dependency locks,
   configuration and schema/migration intent.
2. **Built:** image digest, signed provenance, SBOM, base image, framework,
   design-system, provider, package and generated-contract versions.
3. **Deployed:** environment, GitOps revision and immutable desired/running
   digest.
4. **Observed:** running health, runtime-reported component identity,
   observation time and freshness.

Fleet identity is fabric-minted at registration (decided 2026-07-25): the
`app_id` is the primary identity for life, with the repository and, after
matriculation, the enterprise CMDB reference held as cross-references. This
held spec still does not add speculative portfolio fields to manifest v1.

### Mobile platform surfaces

Applications may declare native-mobile surfaces at registration; the fabric
manages them as first-class platform surfaces with mobile semantics rather
than web assumptions (decided 2026-07-25):

- **Composition.** The four `app_component_snapshot@1` states are
  platform-qualified; the state keys are unchanged (`desired`, `built`,
  `deployed`, `running`). For native mobile: desired is scaffold, SDK, and
  lock intent; built is the signed store artifact (build number, artifact
  hash) plus any over-the-air (OTA) bundle identity; the deployed slot
  carries store-release semantics — store track, review status, and
  staged-rollout percentage; running is the live version distribution —
  adoption by version, crash-free rate, and OS spread. Version skew is
  first-class: simultaneous live versions are never collapsed into one
  "mobile version."
- **Promotion semantics.** OTA-eligible changes (product-owned JS and
  content inside the G-F4 write surface) may ride the accelerated path as
  web write-surface work does; native binary changes are store-track
  releases behind the strict path. OTA distribution to any user-facing
  channel is itself a release-path promotion: only the exact Accepted,
  `release_ready:true` bundle identity may be promoted to production OTA
  channels, by named human authority; the accelerated path reaches preview
  and internal-distribution channels only. Store review is an external
  authority the factory cannot bypass, and a named human retains the submit
  decision. Rollback is halt-rollout plus OTA roll-forward where eligible,
  never a silent binary downgrade. The minimum supported app version is a
  governed record tied to the product's API compatibility window.
- **Observation.** Store submission/review/rollout, version-adoption,
  crash-free, and store-rating observations enter
  `fabric_observation_event@1` from their named authorities: the store
  consoles, the approved mobile build/distribution service, and the
  approved crash/device-health source (D2 worksheet additions). Store
  ratings enter as satisfaction-class observations under the same source,
  period, and freshness contract (public aggregates, so minimum-cohort
  suppression does not apply); store-review free text stays excluded until
  a governed retention/redaction contract exists.
- **Sequencing.** Contracts are platform-aware now; factory-produced mobile
  iterations and store-submission automation remain gated behind AF-M05.2
  seam health and the native generator path graduating. Until then the
  fabric observes declared mobile surfaces read-only.

### Conversation-agent-to-factory communication

- Transport: MCP over streamable HTTP with OAuth 2.1 authorization
  (authorization-code with PKCE against PDS Okta; token audience bound to the
  factory resource). No API keys, no long-lived bearer secrets.
- Tools are typed, versioned, least-authority operations. Initial contract:

| Tool | Authority level | Effect |
| --- | --- | --- |
| `search_applications` | read | Queries only applications and fields the caller may see; returns source and freshness for every projected claim |
| `get_application_workspace` | read | Returns one authorized app's product context, work, composition, environments, evidence and outcome projection |
| `draft_intake` | read/draft | Builds a candidate intake record from the conversation and PoC artifacts; nothing persists outside the session |
| `submit_intake` | propose | Persists the intake as a sponsorship request; returns tracking id; idempotent on intake id |
| `submit_feedback` | propose | Persists a typed feedback record against an application, journey, Outcome, Product Increment, Deliverable, or Task |
| `propose_priorities` | propose | Persists an app-priority proposal for Outcome Owner/Product Owner disposition or a program-priority proposal for Program Product Manager disposition; only ratified work reaches Flow Controller admission |
| `draft_change_proposal` | read/draft | Converts a context-bound request into a typed intended change, acceptance impact, affected authority surfaces and proof plan; changes nothing |
| `submit_change_proposal` | propose | Persists an authorized `fabric_command_proposal@1` for disposition; cannot select its own admission, runner, merge, release or risk decision |
| `request_iteration` | propose | Requests an iteration run; admission decides |
| `accept_slice` / `reject_slice` | decide (Outcome Owner only) | Records acceptance state per the work system |
| `create_product_repo` | execute-with-approval | Executes bootstrap only when a matching approved graduation exists (G-F1, planned and beyond); idea-stage repositories are concierge-created by engineers under G-F13 until idea-stage self-service opens, at which point this contract gains a G-F13 idea mode; idempotency key required; template-locked to the intake profile |

- No tool takes free-form shell, file paths outside the product workspace,
  repository names outside the approved namespace, or raw model text destined
  for direct execution. Tool inputs and outputs validate against versioned
  JSON schemas; invalid calls fail closed.
- Every call records: authenticated principal, on-behalf-of product, tool,
  arguments hash, decision, correlation id. Prompt and tool-call audit follow
  the chat prompt-audit posture in the Deployment Reference (named sink,
  minimum 90-day retention, SIEM export, kill switch).
- Chat is always visibly scoped to an application and, when applicable, a
  Purpose, Outcome, Product Increment, Deliverable, Task, or Assignment. The
  client displays the accepted source version behind that context. Conversation
  history may explain or draft, but it never replaces the controlling records.

### Reasoning architecture

Reasoning proposes; gates dispose. The factory never lets model output mutate
state directly.

1. **Deterministic first.** Typed records plus rules decide admission, WIP,
   budgets, routing, staleness, and status with no model call, generalizing
   the `scripts/check-program-flow.mjs` observer (`needs_agent:false` means
   no model is invoked). Model reasoning runs only when a rule says it must.
2. **Role-shaped reasoning services**, stateless and event-invoked, mirroring
   the operating model's Control Crew functions:
   - Intake Analyst: PoC artifacts to intake record and model clues, wrapping
     `product analyze` and `propose-model` outputs rather than re-deriving
     them.
   - Product Planner: feedback and acceptance state to Product
     Increment/Deliverable replan proposals. App priority routes to the
     Outcome Owner/Product Owner; cross-product or program priority routes to
     the Program Product Manager.
   - Admission (Program Flow Controller function): after priority disposition,
     applies capacity, dependency, WIP, budget and stop policy to admit, defer,
     or park; it never chooses product or program priority.
   - Implementation workers (Claude Code, Codex, or successors): execute one
     Assignment inside the bounded write surface with the work system's four
     controlling records as context; conversation history is never authority.
   - Independent reviewers: focused or comprehensive review per class;
     cross-model adversarial review for sensitive or repeat-failure work.
3. **Model-effort routing.** Medium-effort workers for extraction and
   mechanical transforms; high-effort synthesis for replanning and conflict
   resolution; exceptional escalation recorded with rationale — the same
   routing discipline as the
   [Research Steward harness](../start/framework-research-steward-harness.md).
4. **Structured outputs only.** Every reasoning product is a versioned,
   schema-validated artifact (for example `factory_intake@1`,
   `factory_replan_proposal@1`, `factory_iteration_report@1`) retained in the
   product repository or run manifest.

### Product iteration loop

1. Stakeholder asks, gives feedback, or proposes a change inside a visibly
   versioned app/work context. Explanation remains read-only; a requested
   change becomes a typed proposal in the product repo.
2. Planner drafts a replan proposal (Product Increment/Deliverable/Task
   changes and priorities); the Outcome Owner/Product Owner disposes
   application priority and the Program Product Manager disposes
   program/portfolio priority.
3. Only after that priority decision, admission gate (G-F3) checks fleet WIP,
   per-product budget, controlling record freshness, dependencies, required
   actions, and stop rules. Admission cannot reprioritize the work.
4. Runner orchestrator leases one Assignment to one ephemeral runner Job:
   pinned framework packages from the approved registry, disposable
   workspace, product repo checkout only, exact write roots.
5. The runner executes the documented loop — model/frontend edits inside the
   write surface, then `product validate`, `generate`, `generate --check`,
   `test --fast`, frontend checks when in scope, `handoff` — and creates a
   branch, current independent review, draft PR, and signed synthetic-data
   branch preview under the **Accelerated** profile.
6. Accelerated proof is pre-release evidence only. It cannot claim main-merge
   eligibility, Accepted, release readiness, or production promotion.
7. To become an **Integration Candidate**, the unchanged exact source SHA
   executes every deferred candidate gate, obtains current required review,
   and passes the required PR pipeline. A named human may then merge.
8. **Destination Verified** means the exact merge SHA passed the destination
   pipeline and an immutable digest may be deployed to dev. It does not mean
   product acceptance or release readiness.
9. The workspace regenerates from the dev deployment. The Outcome Owner
   accepts or redirects the exact product result under G-F9; acceptance, and
   only acceptance, burns down accepted scope.
10. For that same Accepted source and digest, a **Release Candidate** adds
   strict release, live-provider, security,
   operations, design-system, accessibility, provenance, SBOM/signature, and
   rollback evidence over destination-verified source and the exact immutable
   artifact. Only after both `Accepted` and `release_ready:true` refer to that
   identity may named human release authority promote the certified digest
   through GitOps; promotion never rebuilds it.

The tracked
[delivery profiles](../start/delivery-profiles.md) remain the command-level
contract: `accelerated` permits explicitly deferred aggregate gates and
`candidate` is the technical integration gate. "Release mode" is not a
shortcut around candidate, destination, strict release, or human promotion
authority. The remote-state term `DESTINATION_VERIFIED` retains its narrow
[exact-merge-SHA meaning](../start/remote-promotion-state-synchronization.md);
it never implies acceptance.

Latency expectations are part of the contract: write-surface iterations land
in minutes to hours; anything crossing policy, auth, data classification,
dependencies, generated templates, integrations, or CI escalates to the
engineering harness and is shown as waiting-for-engineering, typically within
one working day. The factory must not hide this asymmetry.

### Human-owned required actions

Some prerequisites cannot and must not be automated: provisioning a dev
database, exporting an approved schema snapshot from a production system,
granting network access, creating secrets, or making a classification or
approval decision. The factory treats these as first-class work, not silent
stalls:

- When intake or a runner hits a non-automatable prerequisite, it raises a
  typed required-action record (`factory_required_action@1`) in the product
  repository: category (environment provisioning, data fixture, source
  schema export, credential/secret, access, approval, enterprise
  registration such as CMDB matriculation), what is needed and
  why, which Deliverables it blocks, the owning engineering Workstream, data
  constraints, and the evidence that will clear it.
- Per the work system, the record is carried as a Task or Deliverable owned by a
  human Workstream in the "Waiting for another team" state and routed to
  the owning team through the normal execution adapter (Jira mapping in the
  [Program Work System](../reference/program-work-system.md)).
- The app workspace projects it as a Required Action with owner,
  blocking scope, and aging; admission (G-F3) will not admit dependent
  Deliverables until the record clears. Aging required actions escalate through
  the Program Flow Controller function.
- Clearing requires evidence, not prose: a platform-secret reference name,
  an applied migration artifact, or an approved export in
  `.appfw/source-evidence/`.

Worked example — the new product needs a dev database using the current
production schema: engineering provisions the dev database instance and a
platform-secret connection reference (required action, human-owned); the
production schema arrives only as an approved, classification-screened
schema-only export into `.appfw/source-evidence/` (G-F2; production data
never moves to dev); agents then generate synthetic seed data and
migrations inside the write surface and populate the instance through the
normal `product migrate` and seed path. Humans provision and approve;
agents generate and populate; the dashboard shows whose move it is.

### Fleet catalog and per-app workspace

The dashboard is the management experience for the factory, hosted in the dev
Kubernetes environment. It follows the accepted dashboard projection doctrine
in the
[Product Management Strategy](../strategy/app-framework-product-management-strategy.md),
the canonical [Program Work System](../reference/program-work-system.md), and
the PDS component/quality contracts. A separately developed Product Dashboard
implementation may contribute information architecture and frontend assets
only after its branch is accepted and contract compatibility is verified; this
held design does not depend on that in-flight implementation or inherit a
static, single-checkout runtime architecture for a multi-application
authenticated service.

The fleet view supports authorized search and comparison across registered
applications. Each `/apps/{app_id}` workspace then projects:

- purpose, North Star, strategy, Product Increments, Deliverables, Tasks,
  Assignments, Implemented-versus-Accepted state, backlog references, Required
  Actions, owners, WIP/budget, and pause posture from accepted product records;
- current and historical branch, PR, build, destination, release, deployment,
  incident and rollback observations from their owning systems;
- dev and production links when authorized, and direct links to the app's
  observability and monitoring dashboards (Grafana and the logs/traces entry
  points) — the workspace links out to the operations surface, never
  re-renders it — all without proxying product traffic;
- for declared native-mobile surfaces: store release state (track, review,
  staged rollout), live version adoption and skew, crash-free rate, and the
  governed minimum-supported-version against the API compatibility window;
- desired, built, deployed, and running component composition
  (platform-qualified per declared surface), never one
  ambiguous "app version";
- usage and operational health, satisfaction with source/cohort/period, and
  build/runtime AI usage and cost as distinct measures; and
- the authority source, exact commit or digest when applicable, observation
  time, freshness, and `UNKNOWN`/`STALE` state for every material claim.

The workspace offers context-scoped **Ask/explain**, **Draft change**,
**Submit proposal**, **Request execution**, and **Accept/reject** actions.
Their typed contracts and caller authority determine what happens; chat never
edits a queue, repository, cluster, deployment, or decision record directly.
An application-team owner may maintain product context, propose priorities,
request bounded work, and accept product outcomes for that app. Ownership does
not grant merge, security/classification, dependency, production promotion,
risk, secret, or cross-app authority.

### Hosted and local adapters

- Hosted mode serves the same versioned read and command contracts through the
  dev-Kubernetes control plane with Okta authorization, source adapters, audit,
  and short-lived runner Jobs. The browser receives no repository, cluster,
  registry, or release credentials.
- Local mode runs the same UI and contract semantics against one explicitly
  selected checkout through the App Framework CLI. It labels working-copy,
  branch, dirty-state, and remote freshness and may execute locally authorized
  accelerated work under the same assignment, handoff, review, and write-root
  rules.
- Any local HTTP service binds to loopback only, rejects untrusted
  `Host`/`Origin`, uses CSRF protection and an ephemeral session capability,
  has a bounded idle/absolute lifetime, and exposes no repository credential
  to browser code. Each write requires an explicitly selected checkout and
  confirmation of resolved branch/write roots; canonical-path and symlink
  checks prevent escape.
- Global fleet observations still come from the hosted read API. Offline or
  unreachable remote facts render `UNKNOWN`/`STALE`; local state never silently
  overwrites a fleet fact.
- The hosted control plane cannot initiate arbitrary execution on a developer
  workstation, and local mode receives no cluster or production-release
  credential.

### Design system usage

Every user-facing surface in the combined target uses the PDS design system.
The factory produces governed product SPA/mobile work and operates only its
proposal/approval/run surfaces; the separate management plane operates the
fleet and per-app experience.

- Autonomous frontend iterations compose only from the approved PDS catalog,
  floorplans, tokens, and machine-readable agent recipes defined by the
  [PDS Health Design System](../frontend/pds-health-design-system.md) and
  the [Frontend Starter Contract](../frontend/product-frontend.md). Bespoke
  visual systems, new UI dependencies, and token overrides outside the
  approved theme axes are outside the write surface (G-F4) and escalate.
- This is a quality mechanism, not only a constraint: composing from the
  governed catalog is how a citizen-directed iteration inherits
  accessibility, density, theming, and interaction quality without a
  designer in every loop.
- The management experience must be an executive-credible product, not a
  decorated inventory table. Fleet overview, product-owner workspace,
  engineer evidence drill-down, and architect composition view use progressive
  disclosure, calm visual hierarchy, purposeful motion, strong empty/error/
  stale states, and the signature-quality direction in AF-OG05. Attractive
  presentation may never hide provenance, uncertainty, risk, or required
  action.
- Evidence: `scripts/appfw product frontend-test --json` plus the PDS
  catalog and component checks (`scripts/check-pds-components.mjs`,
  `scripts/check-pds-catalog-evidence.mjs`) run in product CI whenever UI
  is in scope. Candidate and release evidence also includes accessible
  browser/viewport proof, representative task success, and independent
  product-quality review; a funded signature surface uses blinded comparative
  review when AF-M05.4 requires it. Signature-experience work beyond the
  catalog is Class C/D product-design work, never an autonomous write.

### CI/CD requirements

- Repo bootstrap (day zero, part of `create_product_repo` or its audited
  concierge equivalent at idea stage): branch
  protections, required PR checks, the product CI pipeline, the tracked
  program-contract file, and the intake record. No product exists without
  its gates.
- Product PR loop: `scripts/appfw product validate --json`,
  `scripts/appfw product generate --check --json`,
  `scripts/appfw product test --fast --json`,
  `scripts/appfw product frontend-test --json` plus PDS catalog and
  component checks when UI is in scope,
  `scripts/appfw product handoff --json`, plus the review-brief artifact.
  Because runners push via API, the pre-push guard's checks are enforced
  server-side as required Bitbucket checks, not only as local hooks.
- Accelerated profile: focused proof, current handoff/review, draft PR and
  signed synthetic-data branch preview; deferred gates and forbidden claims
  remain visible.
- Integration candidate: every deferred candidate gate, current exact-SHA
  independent review and required PR pipeline before a human merge decision.
- Destination verified: full destination pipeline at the exact merge SHA;
  image build, image SBOM, signature/provenance attestation, and ArgoCD sync of
  the immutable digest to dev may follow.
- Release path: unchanged and unbypassable — `release-check`, strict release
  evidence, live-provider and ops certification, PDS security baseline,
  rollback proof, and exact-digest provenance per the Release Gate and
  Deployment Reference. Production promotion additionally requires Outcome
  Owner acceptance of that exact source/digest. The factory has no release or
  promotion authority.
- Factory control plane repo: the same discipline applies to the factory
  itself, plus GitOps-managed cluster manifests and a tested kill switch.
- Runner provenance: every `factory_run_manifest@1` records profile, exact
  source, framework packages (`appfw.lock`), runner image digest, commands,
  executed and deferred gates, model/provider, input/output token totals,
  compute-cost basis, review, artifacts, and exit state.

### Observability

- One correlation spine end to end: conversation id, intake id, Product
  Increment/Deliverable/Task and Assignment ids, run id, PR, pipeline,
  deployment, dashboard generation.
- OpenTelemetry traces across factory services and runner stages; product
  previews keep the standard product observability contract.
- Metrics that matter: intent-to-preview cycle time, feedback-to-iteration
  cycle time, queue and review age, fleet WIP, token and compute cost per
  product and per accepted result, write-surface escalation rate, incident
  and rollback counters.
- Build-time agent tokens are metered by assignment, run, model, retry, cost,
  review, and accepted result. Runtime AI tokens are metered separately by
  application feature/operation, model, cost, outcome, and correction. Token
  volume is an input-cost signal, not product value or a team leaderboard.
- Usage never stands in for satisfaction. Satisfaction projections name the
  approved feedback source, cohort/sample, collection period, and freshness;
  enforce field-level authorization and minimum-cohort suppression; and omit
  respondent text unless an approved redaction, retention, access, and deletion
  contract explicitly governs it. Outcome claims name their measurement
  contract.
- Prompt and tool-call audit to the named sink with SIEM export and
  retention, per the chat audit posture. No PHI or secrets in logs; general
  logs never carry prompt bodies.
- Fleet operations dashboard in the existing Grafana stack; product and
  delivery truth stays with the authorities in the fact-class matrix.

## Security, Privacy, And Governance

Identity and authentication:

| Identity | Kind | Protocol / credential | Authority |
| --- | --- | --- | --- |
| Application-team owner / citizen Outcome Owner | Human | Okta OIDC; OAuth 2.1 + PKCE to factory MCP; oauth2-proxy session for dashboard/preview | Maintain authorized product context, draft intake, feedback and priority/change proposals, request bounded execution, accept/reject own product's results. No implied merge, release, risk, secret, classification, dependency, or cross-app authority |
| Product sponsor | Human | Okta OIDC; approvals UI | Approves sponsorship and funding (G-F1); participates in continuation/retirement under G-F12. Funding authority alone does not merge, accept, release, or retire |
| Product Owner / Program Product Manager | Human | Okta OIDC; planning and decision UI | Product Owner disposes app-level scope/priority/lifecycle proposals; Program Product Manager disposes program/portfolio priority and activation. Neither role gains admission, merge, release, or risk authority |
| Program Flow Controller function | Human/control function | Okta OIDC; deterministic admission UI | Applies ratified priority, dependency, WIP, budget and capacity policy to admit/defer/park. Has no product, architecture, source, merge, release, or risk authority |
| Engineer / implementation owner | Human | Okta OIDC; existing harness tools | Implements assigned Class C/D or escalated work. Any review, integration, merge, or release authority requires a separately assigned role |
| Human reviewer | Human | Okta OIDC; review UI | Reviews and requests/approves changes within the review contract; does not merge, accept risk, release, or make product priority |
| Human repository / merge approver | Human | Okta OIDC; protected Bitbucket PR controls | May merge an exact reviewed, green PR under repository policy; merge does not imply product acceptance or release readiness |
| Security, SRA, CAB and release authorities | Human | Approved enterprise decision systems | Each makes only its named classification/control/risk/package/window/promotion decision; no dashboard, factory, reviewer, or Control Crew role substitutes for it |
| Approved conversation agent (Claude may be initial) | Non-human | Acts only under the authenticated caller's delegated authorization through provider-neutral typed MCP tools; on-behalf-of recorded | No direct repo, cluster, or shell access; no provider identity becomes authority |
| Management-plane services | Non-human | Distinct Kubernetes workload identities; platform secret mechanism | Register/project/query under policy. Cannot run code, merge, release, approve, or touch product traffic/data |
| Factory services | Non-human | Distinct Kubernetes workload identities; platform secret mechanism | Queue, admit by policy, orchestrate, and report. Cannot set product priority, merge, release, accept, or approve risk |
| Runner job | Non-human | Per-product, short-lived, scoped repo token from the repository-owned auth path; registry read | Write only its product repo branch within the write surface; no cluster API beyond its job; no SaaS credentials |
| Preview workload | Non-human | Per-product service account; synthetic data source credentials only | Serve the preview; nothing else |
| Mobile signing/store credential | Non-human | Store and signing credentials held only by the approved mobile build/distribution service | Used only for store-track release builds after candidate gates and for governed store submission; never available to preview runners, idea-stage builds, or the conversation path; fully audited |
| Bitbucket provisioning credential | Non-human | Workspace-scoped credential per the Bitbucket REST Auth Runbook | Used only by the approved `create_product_repo` operation and, during the concierge interim, by engineers executing audited idea-stage bootstrap under the same runbook path; fully audited |

Rules: every human action is attributable to the human; every agent action
records the sponsoring principal; no shared service account ever launders
human work (the delegated-auth posture G1/W3-B established for SaaS writes
applies to the factory's own actions). Approval in one product grants nothing
in another.

Data and PHI: intake requires a data classification; source artifacts are
screened before entering the repository (automated PHI/secret screen, human
review on flag); previews and runners use synthetic or approved fixture data
only; prompt audit redaction applies. Threats and controls follow the
[Agentic Threat Model](../architecture/concerns/agentic-threat-model.md):
prompt injection cannot reach infrastructure because irreversible operations
require recorded human approval, tools are typed and least-authority, and
runner write surfaces are mechanically enforced.

## Policy Gates

| Gate | Trigger | Requirement | Authority |
| --- | --- | --- | --- |
| G-F1 Sponsorship | Idea → planned graduation (gates CMDB matriculation, factory budgets beyond the trial allowance, and any promotion beyond dev) | Named sponsor and funding, demand stream, value hypothesis, success metric, data classification (AF-PT07 rubric) | Sponsor + Program Product Manager |
| G-F2 Source screen | Before artifacts enter the repo | Classification declared; PHI/secret screen passed; redaction confirmed | Security owner (automated, human on flag) |
| G-F3 Admission | Before any iteration run | Ratified product/program priority disposition exists; fleet WIP ceiling, per-product budget, dependencies, staleness and stop rules pass; unmet required actions block dependent Deliverables | Program Flow Controller function applies capacity/admission policy only |
| G-F4 Write surface | During runner execution | Autonomous writes only in `.appfw/model`, product-owned frontend composed from the approved PDS catalog, seeds/fixtures, docs/copy, proposal ledger; all else escalates | Runner sandbox + boundary checks |
| G-F5 Review routing | Before merge | Class A/B: green gates plus clean independent agent review; Class C/D: human review and decision | Two-speed routing per operating model |
| G-F6 Product CI | Before merge completes | Validate, generate-check, tests, docs-check when docs, handoff evidence | Integration gates |
| G-F7 Preview deploy | Before namespace deploy | Signed immutable digest, image SBOM, synthetic-data and auth-on configuration | ArgoCD policy |
| G-F8 Stage graduation | Prototype toward release | Work-system stage rigor; full release gate and strict evidence for release claims; factory has no promotion authority | Release authority |
| G-F9 Acceptance | Deliverable burndown | Only the Outcome Owner's acceptance marks Accepted; retained `accept_slice`/`reject_slice` names are compatibility contracts | Outcome Owner (citizen) |
| G-F10 Autonomy graduation | Phase 0 to 1 to 2 | Factory evaluation evidence: intake fidelity, zero write-surface escapes, incident rate, cost per accepted result; explicit human decision | Program Product Manager + Security owner |
| G-F11 Kill switch | Any time | Factory-wide and per-product stop; audited; drill evidence retained | Operations |
| G-F12 Registry authority | Registration, owner transfer, tier elevation/reduction, lifecycle transition (including CMDB matriculation at planned), pause, deprecation or retirement | Authenticated, audited decision; authority-source references current; cross-app isolation passes; tier elevation grants only its declared capabilities | Product Owner owns lifecycle/product disposition; Sponsor owns funding/continuation consent; enterprise registry owner executes the record; Security owner must approve factory-control elevation |
| G-F13 Idea-stage bootstrap | Before an idea-stage repository or namespace is created | Registered owner within quota; dev-only, synthetic-data-only posture; dormancy auto-archive policy accepted; G-F2 screen active; at most a trial execution allowance | Registry authority (G-F12 owners) + platform policy |

## Held Delivery Plan And Effort

These are planning ranges, not staffing commitments or activated Assignments.
Person-weeks include engineering, product/design, security/platform, test,
review, and operational proof; calendar ranges assume safely parallel,
disjoint Assignments and available reviewers. Activation must re-estimate from
a named product, approved environments, source adapters, and current team
capacity.

Planning basis: a five-to-seven-person cross-functional team, with three or
four engineers working across at most three disjoint Assignments; one combined
product/design lead; and fractional architecture, security, platform/SRE,
analytics, and independent-review capacity. These are low-confidence
order-of-magnitude ranges (roughly +/-30%), exclude enterprise procurement and
unbounded IdP/source-owner lead time, and assume existing Bitbucket, ArgoCD,
Kubernetes, registry, OTel, and PDS foundations are reusable. After the
authority/contracts foundation, the read-spine/workspace chain (8-12 weeks)
and supervised-runner chain (6-8 weeks) overlap, then converge on candidate and
destination proof (4-6 weeks). Including integration contingency, the
overlapped CRM critical path is 16-22 weeks; the unconstrained serial sum would
be 20-29 weeks. Strict production release, two-product extraction, economic
proof, and citizen self-service follow and are not on the CRM MVP critical
path.

| Order | Planned Deliverable | Roadmap home | Initial effort |
| --- | --- | --- | --- |
| 1 | **AF-D08.1a — Authority and contract foundation:** ratify registration, observation, composition and command schemas; source/retention matrix; RBAC; threat model; freshness and stop rules | AF-M08.1 | 2-3 calendar weeks; 6-10 person-weeks |
| 2 | **AF-D08.1b — Read-only registry spine:** registry adapter, reconciler/projector, query API, component index, audit history, and one-checkout local read adapter | AF-M08.1 | 4-6 calendar weeks; 12-18 person-weeks |
| 3 | **AF-D08.1c — Fleet and app workspace proof:** multi-app search, CRM composition/history, purpose-to-task drill-down, environment links, outcome/cost panels, and a second independently owned registered consumer with an approved target model | AF-M08.1 | 4-6 calendar weeks; 12-18 person-weeks |
| 4 | **AF-D08.2a — Supervised proposal-to-preview:** context-bound chat, typed change proposal, admission, ephemeral accelerated runner, independent review, draft PR, and signed synthetic-data branch preview | AF-M08.2 | 6-8 calendar weeks; 20-28 person-weeks |
| 5 | **AF-D08.2b — Integration and dev delivery:** candidate-profile gates, exact-SHA PR/destination verification, immutable image provenance, dev GitOps deployment, correlation, and acceptance projection | AF-M08.2 | 4-6 calendar weeks; 12-18 person-weeks |
| 6 | **AF-D08.2c — Strict release path:** live-provider, security, operations, accessibility and rollback evidence plus human promotion of the exact Accepted and certified digest | AF-M08.2 | 6-10 calendar weeks; 20-30 person-weeks |
| 7 | **AF-D08.2d — Two-product reusable factory kit:** extract packages, PDS recipes, registry/context contracts, review routing and evidence patterns only after the reference and second consumer prove them | AF-M08.2 | 3-5 calendar weeks; 8-14 person-weeks |
| 8 | **AF-D08.3a-c — Second-use economics and graduation decision:** normalized first/second-use baseline; human, agent-token, compute, review, retry, defect, support and rollback cost; accessibility, task-success, responsive/visual, first-impression and comparative-preference quality; continue/narrow/stop decision | AF-M08.3 | 4-6 calendar weeks; 12-18 person-weeks |
| 9 | **AF-D10.5a — Conditional citizen self-service graduation:** after AF-M08.3 passes and a human activates it, add sponsor/classification intake, idempotent repository bootstrap and protections, bounded budgets/WIP, operational onboarding/support/retirement, adversarial authorization, and progressive fleet rollout (self-service idea-stage bootstrap opens separately once G-F13 enforcement is mechanical) | AF-M10.5 | 8-12 calendar weeks; 30-45 person-weeks |

With the stated overlap and staffing, the low-confidence target for the
complete existing-CRM supervised vertical is 16-22 calendar weeks after
activation. Governed citizen self-service across a fleet is a later 7-10 month
end-to-end product outcome including its prerequisites, not the MVP.

The first vertical proof is deliberately one complete, inspectable loop:

1. register CRM with its purpose, owner, repository, dev URL, management tier,
   authoritative references, and current source/deployed identity;
2. show its accepted product context, build and promotion history, exact
   desired/built/deployed/running composition, freshness, and build-agent cost;
3. let an authorized owner chat in one Deliverable context and submit one
   catalog-constrained copy or style change as a typed proposal;
4. admit one Assignment with one repo, one branch, exact write roots,
   package-pinned tooling, synthetic data, budget, and stop rule;
5. produce an accelerated branch preview and draft PR with current proof and
   independent review;
6. graduate the unchanged SHA through candidate checks and human merge;
7. verify exact destination-main CI and deploy the immutable signed digest to
   dev through GitOps; and
8. show the correlated result in the workspace for Outcome Owner acceptance.

That proof establishes the business loop before adding broad connectors,
production promotion, repository self-service, or citizen autonomy. CRM is a
reference proof; it cannot by itself satisfy AF-M08.1's independently owned
second-consumer exit.

## Phased Rollout

- **Phase 0 — Concierge and read-only spine.** Engineers and supervised agents
  register the bounded reference product, operate the factory contracts by
  hand, and prove the fleet/app projection plus the complete CRM vertical
  proof above. The stakeholder gets the dashboard-and-chat experience; humans
  remain the control plane. The factory is extracted from what this proves.
- **Phase 1 — Supervised live loop.** The Hosted Product Factory subsystem runs
  write-surface iterations autonomously (Class A/B) with batched engineer
  review and full gates for an independently owned second consumer; intake
  remains engineer-mediated.
- **Phase 2 — Self-service intake.** Citizens submit intakes through an
  approved conversation agent behind G-F1 sponsorship only after AF-M08.3's
  economic and quality proof and explicit AF-D10.5a activation; self-service
  idea-stage bootstrap opens separately, only once G-F13 quota, dormancy, and
  posture enforcement are mechanical; fleet WIP scales only as review
  capacity and whole-cost evidence prove out.

Each phase graduates through G-F10 with retained evidence, never by default.

## Acceptance Evidence

| Criterion | Proof Command / Artifact | Required Before |
| --- | --- | --- |
| Registry and workspace show source-bound claims and deny unauthorized cross-app reads and commands | Contract tests, seeded cross-app authorization suite, and retained `fabric_app_registration@1` decisions | Any multi-app access |
| Registered, observed, App Framework-managed and factory-managed tiers grant only their declared cumulative capabilities | G-F12 positive/negative authorization and tier-transition evidence | Any tier elevation |
| Hosted and local modes conform to the same read/command schemas; local dirty and unreachable-remote state cannot masquerade as fleet truth | Adapter conformance suite and offline/stale fixtures | Local mode distribution |
| Local service rejects non-loopback, hostile Host/Origin, CSRF, expired session, unselected checkout, symlink/path escape, and browser credential-exfiltration fixtures | Adversarial local-adapter security suite and bounded-lifetime evidence | Any local write capability |
| Component inventory distinguishes desired, built, deployed and running identity at exact SHA/digest and fails stale observations closed | Signed provenance/SBOM, GitOps and runtime fixture reconciliation evidence | Any version/compliance claim |
| Complete CRM proposal-to-dev loop correlates context, Assignment, run, review, PR, exact destination SHA, signed digest, deployment and acceptance | Retained vertical-proof evidence bundle and workspace projection | Phase 1 build |
| Concierge loop ran for one real product using factory record shapes | Intake, program-contract, feedback, workspace and evidence artifacts retained in the product repo | Phase 1 build |
| MCP contract conformance and adversarial authorization tests pass (cross-product access denied; unauthenticated denied) | Factory repo CI evidence | Any citizen access |
| Repo bootstrap is idempotent (duplicate submit yields one repo) and applies branch protections plus CI on day zero | Integration test evidence + Bitbucket state capture | Phase 2 |
| Write-surface escape attempts are blocked and escalated | Adversarial runner test evidence | Phase 1 autonomy |
| PHI/secret screen catches seeded fixtures | Screen test evidence | First citizen artifact upload |
| Required-action lifecycle proven end to end (raise, workspace projection, human fulfillment with evidence, dependent Deliverable unblocked) | Retained required-action record + observer artifact | Phase 1 |
| Off-catalog UI composition is caught and escalated | PDS catalog/component check failure evidence on a seeded fixture | Phase 1 autonomy |
| Executive, product-owner, engineer, and architect scenarios meet pre-registered task-success, time/error, accessibility, responsive-layout and visual-regression thresholds | Representative browser/viewport evidence and independent product-quality review | First shared fleet/workspace experience |
| Five-second purpose/health comprehension, 30-second next-action/provenance comprehension, and blinded comparative preference meet the accepted AF-M05.4 threshold without obscuring stale/risk/required-action state | Retained first-impression study and blinded comparison decision record | Signature-quality or executive-readiness claim |
| Preview auth posture proven (no-cookie redirect; role-missing denial) | Deployment review evidence per the Deployment Reference | First shared preview URL |
| Fleet WIP, budget, and auto-pause enforcement observed | Observer artifact + dashboard state | Phase 1 |
| Build/runtime AI metering is separated, raw prompts excluded, and cost per accepted result plus cycle-time telemetry is live | Fleet dashboard and telemetry/privacy contract evidence | Phase 1 exit |
| Satisfaction projection shows approved source, cohort/sample, period and freshness; enforces field authorization and minimum-cohort suppression; governs or omits respondent text under retention/deletion policy; and is not inferred from usage | Analytics/feedback privacy and adapter contract tests | Any satisfaction claim or fleet exposure |
| TCO/ROI panels reject hand-entered values; every cost and value-realization component observation is source-attributed with the ROI view computed against the recorded G-F1 value hypothesis | Projection contract tests and a seeded hand-entry rejection fixture | Any fleet exposure of economics |
| Framework demand records carry requesting-app work-item linkage and dedup provenance, hold no planned-work state, and disposition remains with the framework program intake | Contract tests and a disposition-provenance fixture | Phase 1 |
| Platform-qualified composition distinguishes web and native-mobile states — signed store build, release track/review/staged rollout, live version distribution with skew — and mobile observations are source-attributed with freshness | Mobile-reference read-only reconciliation fixture evidence | Any mobile management claim |
| Second consumer proves the reusable contracts and at least 50% lower normalized second-use effort without quality regression | Comparative whole-cost and quality decision record | Self-service graduation |
| Kill-switch drill completed | Runbook drill record | Phase 1 |

## Test And Execution Feedback Plan

Factory components ship with unit and integration tests in their own product
repo CI; the MCP contract carries schema conformance and adversarial
authorization suites; runner behavior is tested against fixture product repos
(golden intake to preview); every gate has at least one negative test. If
execution contradicts this spec — for example the write surface proves too
narrow to be useful, or review capacity cannot absorb Class A/B volume — the
finding routes to the Chief Architect and Program Product Manager as a spec
revision with decision provenance, not as a silent workaround.

## Risks And Controls

| Risk | Control | Owner | Status |
| --- | --- | --- | --- |
| Orphaned-app sprawl | G-F13 idea quotas and dormancy auto-archive; G-F1 sponsorship at planned; auto-pause without acceptance; periodic retirement review | Program Product Manager | Designed |
| Prompt injection reaches infrastructure | Typed tools only; execute-with-approval for irreversible operations; idempotency keys; audit | Security owner | Designed |
| PHI enters repos or previews | G-F2 screen; synthetic-only previews; prompt-audit redaction | Security owner | Designed |
| Review capacity becomes the bottleneck | Fleet WIP tied to measured review throughput; cross-model review; sampled human audit | Program Flow Controller | Open — sets fleet size |
| Dashboard or registry becomes parallel truth | Fact-class authority matrix; projection-only adapters; source links; freshness states; audited G-F12 decisions | Product Owner | Designed |
| Fleet catalog becomes a manually curated CMDB | Register authoritative references, not copied operational state; stop if duplicate maintenance is required | Enterprise registry owner | Open — adapter-dependent |
| Local mode becomes a hidden authority, hostile-browser write path, or remote-execution target | Same contracts; loopback/Host/Origin/CSRF/session controls; explicit checkout/dirty/write-root confirmation; canonical-path/symlink checks; no browser repo credential, inbound execution, or cluster/release credential | Chief Architect + Security owner | Designed |
| Product runtimes depend on the management plane | No product traffic or business data through the control plane; runtime isolation and failure tests | Operations | Designed |
| Factory itself ungoverned | Built as an App Framework product with the same gates and release evidence | Chief Architect | Designed |
| Cost runaway or token leaderboard behavior | Per-product budgets in G-F3; separate build/runtime metering; cost per accepted outcome; no team ranking from token volume | Program Flow Controller | Designed |
| Non-automatable prerequisites stall products invisibly | Typed required-action records; workspace aging; Flow Controller escalation | Program Flow Controller | Designed |
| Built ahead of evidence (register inversion) | Shaped-and-held status; Phase 0 extraction; activation conditions | Program Product Manager | Enforced by this spec |
| Initial conversation-client connector auth immaturity (claude.ai is the first candidate) | Phase 2 gated on a provider-neutral identity design review with the IdP team | Security owner | Open |

## Tech Debt And Follow-Up

- Register disposition: held AF-D08.1a-c, AF-D08.2a-d and AF-D08.3a-c
  Deliverables under existing AF-M08.1-AF-M08.3, with conditional citizen
  graduation and fleet lifecycle operations under AF-D10.5a/AF-M10.5;
  activation remains owed to the Program Product Manager after the protected
  window.
- Dependency: AF-OG02 registry-mode bootstrap must graduate before runner
  images can be built without a framework checkout.
- Identity decision recorded 2026-07-25: the fabric mints `app_id` at
  registration; the repository and the post-matriculation enterprise CMDB
  reference are cross-references. Manifest v1 still gains no global
  management fields.
- Follow-up decision: server-side enforcement mechanics for the pre-push
  guard equivalents (Bitbucket required checks) need a design note when
  Phase 1 is pulled.

## Handoff Notes

Read the companion architecture document for the diagram and flows before
touching implementation. Nothing in this spec authorizes build work; it
authorizes shared understanding. The first implementation step, when pulled,
is AF-D08.1a and the Phase 0 CRM vertical — ratifying the authority/contracts
and operating them by hand — not a broad control-plane or citizen-autonomy
build.
