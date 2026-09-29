# Legacy Application Modernization

This guide is for enterprise teams replacing or strangling a real legacy
application with App Framework. It is related to PoC intake, but the evidence
source is stronger and riskier: a production-shaped codebase, a real database,
stored procedures, integrations, auth, jobs, reports, and users who depend on
current behavior.

The goal is not to transliterate the legacy system or build a general-purpose
coding agent. The goal is a PDS-specific modernization control plane that owns
evidence, authority, treatment decisions, target semantics, verification
obligations, retirement economics, and portfolio learning while using the best
qualified deterministic tools, coding agents, and specialist transformation
engines as interchangeable workers.

## Use When

- The source system has a real database or provider-backed data source.
- The app has existing code, such as .NET controllers, services, Razor/Blazor,
  WebForms, WinForms, WPF, batch jobs, EF, Dapper, ADO.NET, or direct SQL.
- Material behavior also lives in IIS configuration, Angular routes/forms/
  guards/state, SQL Server routines/jobs/security, AWS ECS deployment/runtime
  state, Jira, Confluence, ServiceNow CMDB, telemetry, or operating practice.
- Business logic is embedded in large stored procedures, database triggers,
  scheduled jobs, or UI event handlers.
- The team needs a refactoring plan, not only a new greenfield schema.

## Operating Principle

Modernize by capability, with traceable evidence. Own the modernization system;
rent or integrate the workers.

Use the running socio-technical system as evidence: code, data, configuration,
runtime, work history, knowledge, service management, telemetry, and people.
Keep the target treatment platform-neutral until humans approve it. Use App
Framework only for selected capabilities whose approved treatment maps there.
Prefer incremental strangler slices over a risky big-bang replacement unless
the release owner explicitly accepts that risk.

## Strategic Capability Goal And Current Maturity

The strategic goal is:

> **Establish a typed, provenance-backed Modernization Intermediate
> Representation as a first-class App Framework capability and the durable
> bridge between governed legacy evidence and approved target implementation.**
> The harness reconstructs an evidence-bounded **As-is Modernization IR** with
> claim-specific authority, conflicts, unknowns, freshness, and invalidation;
> supports a separately versioned and human-approved, platform-neutral
> **Approved To-be Treatment Model**; and compiles only approved target
> semantics into complete App Framework contracts with visible loss,
> verification, migration, and retirement obligations. Claude Code, Codex,
> deterministic recipes, and specialist vendors remain interchangeable workers
> behind one task/result protocol. Independent proof must cover behavior, data,
> security, experience, operations, migration, economics, and legacy retirement
> before release and closure.

The first outcome is one representative but controlled .NET/IIS/Angular/SQL
Server application with an approved evidence model and treatment, one compiled
and implemented production-capable vertical slice, differential verification,
a safe migration or explicitly costed coexistence path, and a measured manual
baseline. A second suitable application must reduce normalized discovery-to-
approved-model engineering effort by at least 50 percent while maintaining or
improving evidence, verification, security, and delivery quality.

The current framework provides a useful first layer: bounded code/data/UI
profiling, `model_clues`, a review-only model proposal, a decision ledger,
model status, and guarded source scaffolding. It does **not** yet provide the
complete target capability above. In particular, the current commands do not
claim governed cross-source evidence snapshots, a complete typed Modernization
IR, .NET/SQL/runtime behavior reconstruction, a human adequacy/treatment
workbench, an approved-target compiler, normalized Claude Code/Codex execution,
differential equivalence, verified retirement, or second-use economics. Those
gaps remain future product work and must be proven through named legacy
consumers rather than synthetic harness artifacts alone.

## Truth, Evidence, And Authority

"Understand the design completely" is an unsafe agent acceptance criterion.
Legacy applications often contain stale documentation, undocumented production
behavior, contradictory tickets, dead code, implicit policy, and exceptions
known only by operators. The defensible target is **evidence-bounded adequacy
for an explicitly selected modernization scope**.

The harness should maintain a modernization evidence register. Every source
used by an agent should carry, where applicable:

- source kind and stable identity: repository/path/commit, Jira site/project/
  issue and changelog identity, Confluence site/page/version, schema/export,
  diagram, test, telemetry query, runbook, or interview record;
- retrieval or export time, content hash, source owner, access method, and
  freshness state;
- security classification, approved handling, redaction status, and retention
  boundary;
- the scoped claims supported by the source, plus conflicts, limitations, and
  confidence; and
- links from each inferred model element or decision back to the supporting
  evidence.

Prefer read-only, permission-preserving APIs or immutable sanitized exports.
Do not put tokens, secrets, PHI, tenant data, or unrestricted Jira/Confluence
content into tracked model artifacts. Cache parsed material outside the
retained evidence namespace, and keep the immutable source identity and hash in
the evidence record.

Code, documentation, tickets, runtime behavior, and human testimony are
evidence, not automatically interchangeable authorities. When they disagree,
the harness records the contradiction and routes it to the named business,
system, data, security, or architecture owner. It must not silently choose the
source that best fits the proposed target.

## Two-Stage Model Contract

The Modernization IR is an application-scoped, typed, versioned, and governed
App Framework capability. It is not a generated report, chat summary,
retrieval index, temporary agent artifact, new enterprise canonical model,
master-data platform, generic workflow engine, or lossy translation of every
source artifact. The control plane keeps two linked but separate model stages:

| Model | Purpose | Required contents | Authority |
| --- | --- | --- | --- |
| **As-is Modernization IR** | Explain what the selected legacy scope demonstrably does well enough to make a modernization decision. | Source artifacts and observations; claims and supporting or contradicting evidence; confidence, freshness, and invalidation; conflicts and unknowns; capabilities, personas, journeys, rules, and state transitions; data and SQL contracts; APIs, events, integrations, identity, permissions, tenant policy, and audit; runtime/deployment and operational constraints; experience behavior, tests, telemetry, and incidents; and decisions, mappings, migration, verification, and approval records. | Evidence-backed and reviewable. A named human adequacy gate approves a specific version for a bounded transformation scope; it never certifies complete understanding of the application. |
| **Approved To-be Treatment Model** | Record what should be retained, retired, replaced, redesigned, extracted, strangled, upgraded, rebuilt, or temporarily coexist before selecting a target platform. | Approved capability boundaries and dispositions; retained/changed/retired behavior; target data and authority; interfaces, events, actions, policy, service objectives, support, migration/reconciliation, coexistence, rollout/cutover/rollback, retirement, and independent proof obligations. | Platform-neutral until a capability is explicitly selected for App Framework; becomes authoritative only through named, versioned human product, architecture, data/security, operations, and release decisions appropriate to the risk. |

Raw evidence, the As-is Modernization IR, and agent-generated plans never flow
directly into `.appfw/model`. The Approved To-be Treatment Model is the decision
boundary between discovery and target source. The target compiler maps only
approved semantics into entities, relationships, operations, permissions,
providers, events, projections, migrations, tests, web/mobile product intent,
observability, release, and retirement obligations. Unsupported or lossy
mappings must produce explicit diagnostics, extensions, or approved waivers.
Every generated or human-authored target element must trace to both source
evidence and an approved transformation decision.

## Intake Record

Capture modernization intake under the product repo. Use `product-intake` to
create a clean product shell, then add legacy-specific evidence paths to the
handoff or a product-owned intake file.

```bash
scripts/appfw product new ../modernized-app \
  --from current \
  --profile product-intake \
  --source-kind legacy \
  --app-name modernized-app \
  --display-name "Modernized App" \
  --schema core \
  --provider PostgreSQL \
  --mcp false \
  --kafka false \
  --ui enterprise \
  --legacy-source ../legacy-app \
  --data-artifact "readonly legacy database inventory" \
  --ui-artifact "legacy UI screenshots or route inventory" \
  --json
```

Do not store production credentials, connection strings, tenant data, or PHI in
the intake. Reference sanitized exports, schema inventories, screenshots,
sample rows, and read-only evidence artifacts by relative path.

The command writes `.appfw/legacy-modernization.yaml` for this source kind.
Complete that inventory before treating the generated config as ready.

Then profile the configured source, data, and UI evidence:

```bash
scripts/appfw product analyze --summary --json
```

The analysis command writes `target/appfw/product-analysis.json` and
`.appfw/legacy-analysis.yaml`. Treat those as profiler output that helps direct
the modernization review; the durable modernization inventory remains
`.appfw/legacy-modernization.yaml`. The JSON report includes `model_clues` from
bounded scans of .NET solutions/projects/classes/controllers, SQL routine/table
references, legacy UI files, tabular exports, and structured artifacts. These
clues are a review queue, not a migration plan or final entity model.

Then draft a review-only modernization model proposal:

```bash
scripts/appfw product propose-model --summary --json
scripts/appfw product model-status --json
```

The proposal command reads `target/appfw/product-analysis.json` and writes
`target/appfw/model-proposal.json` plus `.appfw/model-proposal.yaml`. It does
not write final `.appfw/model`; use it to review candidate entities,
properties, routines, integration points, and frontend views before creating
model source. Its `review_decisions` ledger should capture accepted, rejected,
deferred, split, merged, and still-unresolved decisions before the
modernization slice writes source config.

`model-status` writes `target/appfw/model-status.json` and reports the current
modernization phase, blockers, source-model file counts, and next commands. Run
it after proposal review, after writing model source, after validation, and
after generation so the modernization slice has a clear checkpoint before
handoff. It reads `.appfw/model-proposal.yaml`; the slice reaches
`proposal_reviewed` only when `accepted_for_config` is true, `model_owner` and
`reviewed_at_utc` are present, and unresolved decision entries are cleared.
Its `source_authoring_plan` then exposes the schema source root, accepted entity
source targets, routine/frontend/integration decision counts, and validation
sequence for the modernization slice.
After signed review, `scripts/appfw product scaffold-model --dry-run --json`
can preview starter entity and deny-by-default policy source from accepted
entity decisions. Running it without `--dry-run` writes source starters and
`target/appfw/model-scaffold.json`; it does not decide stored procedure
disposition, service boundaries, migration strategy, UI architecture, or
relationship semantics.

## Agentic Modernization Process

Run the process in bounded capability or journey slices. The agent may move
quickly inside a stage, but it may not skip the authority boundary between
understanding, transformation decisions, and target source.

| Stage | Agentic and deterministic work | Human decision or stop gate | Retained result |
| --- | --- | --- | --- |
| 0. Portfolio discovery and routing | Score ownership, cost, risk, usage, incidents, dependencies, lifecycle, change demand, and strategic fit; identify plausible defer, retire, assess, or proceed paths. | Sponsor and accountable service owner authorize detailed assessment scope and spend. | Candidate score, initial boundaries/treatment classes, evidence gaps, and route. |
| 1. Charter, authority, and proof standard | Define selected scope, outcomes, exclusions, owners, data classes, approved sources/identities/environments, decision rights, baseline economics, and proof obligations. | Named product, domain, data, security, operations, modernization, and release authorities approve the charter and access. | Versioned charter, authority policy, baseline, proof standard, stop criteria, and non-claims. |
| 2. Governed evidence snapshot | Acquire content-addressed snapshots or durable references for source/history/builds, Jira, Confluence, CMDB, SQL, IIS, ECS, tests, telemetry, incidents, runbooks, and approved interviews. | Stop on unresolved access, classification, custody, retention, or source-ownership questions. | Evidence manifest with native IDs, versions, hashes, freshness, ACL/classification, collection method, and missing access. |
| 3. Reproducible baseline | Rebuild, package, deploy, exercise, and observe the selected scope in an approved environment, or preserve a precise failure record and environment differences. | Assessment owners accept baseline limitations, safe test-data posture, and observation plan. | Reproducible build/runtime baseline or explicit failure record, dependency locks, baseline routes/APIs/tests/data/ops checks. |
| 4. Deterministic extraction | Use stack-aware compilers, semantic models, parsers, schema/catalog tools, security analyzers, configuration readers, and dependency analysis before probabilistic inference. | Tool limitations and sensitive findings are visible; no adequacy or treatment decision occurs here. | Versioned reproducible analyzer outputs with source locations, coverage denominators, and provenance. |
| 5. Dynamic behavior capture | Run existing/characterization tests, controlled API/UI/database probes, Query Store/approved traces, logs, metrics, service maps, deployment history, and user/support observation. | Selected journeys and risk areas have an accepted observation set or explicit blocking unknown. | Behavioral observations, traces, screenshots/contracts, side effects, performance/failure evidence, and unknowns. |
| 6. Reconcile the as-is evidence model | Combine deterministic facts, runtime observation, requirements/history, service records, and human knowledge into claim-based typed views; retain conflicts and invalidation. | No required domain may be silently absent or smoothed into unsupported certainty. | Versioned as-is evidence graph, traceability, conflicts, unknowns, confidence/evidence grades, and coverage. |
| 7. Human adequacy gate | Review in-scope capabilities, journeys, rules, data, integrations, permissions, runtime, experience, observations, disputes, and unknowns. | Named reviewers accept evidence-bounded adequacy for this versioned scope; blocking unknowns stop treatment. | Adequacy decision with conditions, assumptions, exclusions, reviewers, and invalidation triggers. |
| 8. Treatment options and impact analysis | Compare retire, retain/contain, rehost, upgrade, refactor, replatform, SaaS/enterprise replacement, extract, strangle, App Framework rebuild, or split dispositions with value, cost, risk, data, UX, security, operations, migration, licensing, and lock-in. | Accountable humans select treatment and record rejected alternatives; the harness cannot select App Framework by default. | Option matrix, impact/economics, preferred treatment, rationale, conditions, and rejected alternatives. |
| 9. Human-approved platform-neutral to-be model | Define retained/changed/retired capabilities, journeys, rules, data authority, interfaces/events/actions, policy, service objectives, support, coexistence, cutover, rollback, and verification obligations before target compilation. | Product/domain/data/security/architecture/operations/release owners approve their versioned decisions. | Approved treated to-be model, decision/waiver log, migration and retirement intent, and proof obligations. |
| 10. Compile approved semantics into App Framework contracts | Map only approved target objects into entities, relationships, operations, policy/tenant/audit, providers, data/migrations, events/projections, tests, product intent, extension points, and release obligations. Report unsupported or lossy semantics. | All required semantics are mapped, explicitly extended, or waived by the proper authority; generated ownership remains intact. | Target mapping ledger, diagnostics, validated App Framework source, generated diff, extensions, and unresolved losses. |
| 11. Thin vertical implementation with portable workers | Route bounded `ModernizationTask` contracts to Claude Code, Codex, deterministic recipes, or qualified specialist engines; require comparable `ModernizationResult` handoff and prevent silent scope expansion. | Each slice remains independently reviewable and cannot alter the approved target contract without a new decision. | Code/config/migrations/tests plus worker provenance, exact checks, deviations, failures, unknowns, cost, and decisions needed. |
| 12. Differential verification and migration rehearsal | Use independent oracles, golden data, legacy/target comparisons, shadow or controlled dual execution, data reconciliation, security/accessibility/performance/resilience tests, and migration/rollback rehearsal. | Release, SRA/CAB, residual risk, cutover, and data decisions remain named human authorities. | Differential mismatches and dispositions, current release/migration/rollback evidence, reconciliation, runbooks, and residual risk. |
| 13. Release, observe, reconcile, and retire | Progressively expose the target, monitor service/outcome thresholds, reconcile data and behavior, stabilize support, remove callers/traffic, update CMDB/monitoring, retain records, and eliminate or explicitly cost remaining legacy dependencies. | No closure while outcome, reconciliation, rollback, records, support, or retirement obligations remain unresolved. | Production outcome, incidents/corrections, migration and retirement proof, realized/residual cost, reusable patterns, and model updates. |

## Modernization IR Content

The future implementation should use a typed, versioned schema and structured
parsers rather than an unbounded prose summary. At minimum the As-is
Modernization IR should represent:

- source evidence, lineage, freshness, confidence, contradictions, unknowns,
  decisions, and owners;
- business capabilities, personas, journeys, use cases, state machines,
  business rules, exceptions, and acceptance outcomes;
- entities, relationships, identifiers, classifications, lifecycle/audit
  fields, schemas, files, and data-quality constraints; plus SQL routines and
  stored-procedure contracts, result shapes, side effects, transactions, and
  callers;
- modules, services, routes, calls, dependencies, jobs, events, integrations,
  providers, external contracts, and failure behavior;
- human, service, and agent principals; roles/scopes; row/field/action policy;
  tenant and delegated-identity boundaries; consent; redaction; and audit;
- web/mobile screens and interaction states, accessibility, reports, exports,
  notifications, and user-visible error/recovery behavior;
- IIS/ECS and other deployment/runtime configuration, secrets, environments,
  scale, latency, resilience, observability, telemetry, incidents, support,
  retention, recovery, and compliance obligations; and
- target dispositions, App Framework mappings, characterization/target tests,
  coexistence and migration seams, rollout, rollback, and retirement criteria.

Use semantic code models where a language ecosystem provides them, while
preserving original source locations and type/dependency information. Use
structured Jira and Confluence APIs or approved exports with issue/page version
identity. Retrieval summaries without source identity are not durable evidence.

Every material graph object needs a stable ID and model version, application/
environment/scope, source links, evidence grade and collection method, state
(observed, inferred, asserted, disputed, obsolete, or unknown), confidence,
freshness and invalidation, owner/reviewer, sensitivity/retention, worker/model/
harness provenance, and human correction/decision history. Authority is claim-
specific: deployment artifacts prove deployed code; reproducible tests and
runtime observations prove behavior; approved requirements/domain owners prove
intent; healthy CMDB records reconciled with runtime support ownership/topology;
catalog/DACPAC proves schema; telemetry and Query Store prove usage; versioned
Jira/Confluence/ADR/approval records prove decision history.

## Modernization Control Plane And Portable Workers

The future system has eight logical planes. They are ownership boundaries, not
a mandate for eight new services:

1. governed evidence acquisition and immutable snapshots/references;
2. deterministic stack analysis and approved runtime observation;
3. the typed as-is evidence graph and derived behavior/data/security/runtime/
   experience views;
4. human adequacy, conflict, treatment, approval, and waiver decisions;
5. a target compiler that maps approved semantics into App Framework contracts
   and fails or escalates on unsupported/lossy mappings;
6. interchangeable Claude Code, Codex, deterministic-recipe, and specialist-
   vendor workers behind adapters;
7. independent build, behavior, data, security, UX/accessibility, performance,
   resilience, migration, rollback, release, and retirement verification; and
8. portfolio learning and fully loaded unit economics.

App Framework owns these planes' evidence, authority, model, target, proof, and
learning contracts. It should buy or integrate frontier coding models, semantic
retrieval, stack-specific transforms, compilers/parsers, security/testing tools,
and commodity connectors where they are effective. An Augment index, Cursor
state, Blitzy plan, AWS Transform job, Claude conversation, Codex task, or other
vendor-private artifact may be useful evidence, but it is never the system of
record.

All workers consume a versioned `ModernizationTask` with run/application/slice/
role, evidence snapshot IDs, authority policy, as-is/to-be/target versions,
allowed and forbidden scope, network/live/write permissions, environment,
budgets, required checks/evidence, stop conditions, and escalation owner. They
return a validated `ModernizationResult` with worker/model/harness versions,
evidence used, claims and model/files changed, commands/systems accessed, exact
check results, assumptions, failures, unknowns, deviations, sensitive-data
events, decisions needed, runtime/cost/tokens/retries/interventions, and the next
structured handoff.

Routing starts with a capability handshake for local/remote and Windows/Linux
execution, repository host, network/connectors, sandbox/filesystem, structured
output, resumability/task horizon, model/data region and retention, and
exportable telemetry. Do not send a Windows/IIS proof to a worker that cannot
reproduce that environment or sensitive evidence to a service not approved for
its classification. Extend AIBOM and OpenTelemetry/PDS Observability rather than
creating worker-specific audit truth.

For material risk, an investigator/modeler/implementer cannot become its own
sole treatment authority and verifier. Generated plans, specifications, code,
and tests are claims to check, not independent oracles. Use deterministic
feedback, separate contexts or workers where useful, independent business/data/
security verification, and human authority.

## Modernization Opportunity Rules

Assess modernization at the capability or component level, not once for the
whole application. A single product may intentionally mix these dispositions:

| Disposition | Use when |
| --- | --- |
| Retain and contain | The capability remains valuable and safe in place for the current horizon. Bound change/support risk and record its later exit posture. |
| Retire | The capability, duplicate data, report, routine, or workflow is no longer needed and has an approved records/dependency plan. |
| Upgrade in place | A supported framework/runtime/database upgrade resolves enough risk without changing the product boundary. |
| Replace with SaaS or enterprise capability | An existing PDS or vendor capability meets the requirement with better economics or ownership. Preserve integration, data, permission, licensing, and exit analysis. |
| Rehost | Infrastructure movement is the bounded objective and application change would add unjustified risk. Do not call this application modernization by itself. |
| Replatform | A managed runtime/provider can improve operations without changing business behavior materially. |
| Refactor | Internal structure, rules, or data access should change while preserving the approved product behavior. |
| Extract | A service, data product, or bounded capability should be separated while the rest remains or follows another treatment. |
| Strangle incrementally | A target slice replaces legacy behavior behind a controlled seam. Give coexistence an owner, compatibility tests, residual cost, and retirement criteria. |
| App Framework rebuild | The approved capability needs a new PDS product and maps economically into App Framework; characterize value/behavior rather than transliterating code. |
| Split | Different capabilities intentionally take different dispositions. Preserve cross-boundary data, behavior, migration, support, and retirement obligations. |

## Human Decision And Autonomy Boundaries

The harness should use risk-tiered autonomy:

- **Agent may execute:** read-only discovery, parsing, indexing, dependency and
  traceability analysis, contradiction detection, draft modeling, option
  comparison, test proposal, and repeatable evidence refresh within approved
  access and handling constraints.
- **Human review required:** semantic interpretation, source-authority
  conflicts, capability boundaries, model mappings, modernization disposition,
  parity expectations, policy/tenant/security/privacy choices, and acceptance
  of unresolved noncritical gaps.
- **Human authority required:** budget and priority, accepted risk, destructive
  data changes, migration/cutover/decommission, live credentials, release,
  SRA/CAB, production access, and changes to authoritative systems.

The agent must show its evidence, assumptions, confidence, alternatives, and
open questions in a decision-ready form. Approval is attached to a versioned
model and evidence set; it is invalidated when material source changes or new
critical contradictions appear.

## Efficiency And Learning Design

A powerful process should reduce human archaeology, not merely move it into a
larger agent transcript. When this capability is implemented:

- parse each source once, cache non-authoritative analysis outside retained
  evidence, and recompute only changed files/issues/pages or affected graph
  neighborhoods;
- use source hashes, Jira changelogs, Confluence versions, dependency graphs,
  and impact analysis to make refreshes incremental and deterministic;
- route low-confidence, contradictory, security-sensitive, and high-impact
  findings to humans while batching routine confirmations;
- keep machine-readable models concise and place raw source material behind
  references rather than copying it into prompts or repository docs;
- build characterization tests before changing critical behavior and carry
  those tests through each strangler slice;
- record accepted patterns, corrections, and false inferences so the second
  modernization measures lower normalized effort and rework; and
- use the lightest review depth that preserves intent, policy, evidence, and
  irreversible-decision provenance.

## Capability Readiness Measures

Do not call the agentic modernization path mature until a real legacy product
demonstrates all of the following for a bounded scope:

- an approved denominator for in-scope capabilities, workflows/rules, data,
  integrations, permissions, UI states, and operational requirements;
- versioned provenance and freshness for every source used in a material claim;
- no unresolved critical unknown or contradiction at transformation approval;
- traceability from every target entity, relationship, policy, operation,
  migration, and characterization test to evidence and an approved decision;
- explicit coverage and accepted exclusions instead of an unsupported
  "complete understanding" percentage;
- a treated model covering coexistence, data migration/reconciliation,
  rollback, security/privacy, observability, support, and retirement;
- deterministic incremental refresh and visible invalidation when source
  evidence changes;
- a reproducible legacy build/runtime baseline or an explicit accepted failure
  record, with deterministic and dynamic evidence for the selected behavior;
- portable `ModernizationTask`/`ModernizationResult` execution through Claude
  Code and Codex without worker-private state becoming authoritative;
- a real package-consuming product projection with validation, generated-drift,
  characterization, independent differential/data/security evidence,
  permission, migration/rollback rehearsal, frontend/accessibility,
  operations/support, and handoff evidence;
- one production-capable vertical slice and one verified migration or explicitly
  costed coexistence path with accountable legacy-retirement criteria; and
- measured discovery-to-approved-model lead time, human correction rate,
  post-approval rework, escaped behavior defects, and comparable second-use
  effort, including at least 50 percent lower normalized discovery-to-approved-
  model engineering effort on a second suitable application without lower
  quality.

Use **fully loaded cost per accepted, production-capable business capability
with verified legacy retirement** as the economic unit. Include worker/tool and
platform cost; product/domain/engineering/data/security/operations labor;
environments; migration, dual run, cutover, stabilization, target support,
residual legacy cost, rework, and incidents; then subtract genuinely retired
license, infrastructure, support, and avoided future-change cost. Generated
lines, prompts, sessions, files, or model tokens are not modernization value.

Stop or narrow the process when evidence access is unauthorized, regulated data
handling is unclear, the source scope has no accountable owner, critical
semantics conflict without a decision maker, the target is only a technology
preference, or the work produces more harness/contracts without a named legacy
consumer. Never claim automated behavioral equivalence, authorize a cutover, or
retire the legacy system from model generation alone. Do not build a general
coding agent, universal ontology, custom code-search platform, broad connector
catalog, or autonomous rewrite engine; do not make App Framework the mandatory
treatment; and do not call permanent dual run modernization success.

## Design Basis

The PDS-specific design is substantiated by the July 14, 2026 *Agentic Legacy
Modernization Harness* decision research maintained outside this repository in
the PDS app-fabric research workspace at
`app-fabric-research/agentic-legacy-modernization-harness-2026-07-14.md`. That
packet is research input, not an implementation contract; this lifecycle is the
canonical disposition for App Framework work.

This contract follows current primary-source guidance that modernization starts
from business outcomes and progresses through assess, modernize, and manage;
uses progressive discovery and application-level assessment; designs the
future state explicitly; and migrates incrementally with reconciliation and
rollback. It also follows current AI/software guidance to preserve provenance,
document knowledge limits, define human oversight, validate outputs against
known evidence, and review AI-assisted software changes comparably to human
changes:

- [AWS application-modernization strategy](https://docs.aws.amazon.com/prescriptive-guidance/latest/strategy-modernizing-applications/welcome.html)
- [AWS application portfolio assessment strategy](https://docs.aws.amazon.com/prescriptive-guidance/latest/strategy-application-portfolio-assessment-migration/apg-gloss.html)
- [AWS application design and migration strategy](https://docs.aws.amazon.com/prescriptive-guidance/latest/application-portfolio-assessment-guide/aws-application-design-and-migration-strategy.html)
- [Microsoft Strangler Fig pattern](https://learn.microsoft.com/azure/architecture/patterns/strangler-fig)
- [OpenRewrite Lossless Semantic Trees](https://docs.openrewrite.org/concepts-and-explanations/lossless-semantic-trees)
- [Atlassian Jira issue/changelog API](https://developer.atlassian.com/cloud/jira/platform/rest/v3/api-group-issues/)
- [Atlassian Confluence page/version API](https://developer.atlassian.com/cloud/confluence/rest/v2/api-group-page/)
- [NIST AI RMF Core](https://airc.nist.gov/airmf-resources/airmf/5-sec-core/)
- [NIST Secure Software Development Framework](https://csrc.nist.gov/projects/ssdf)
- [NIST DevSecOps guidance for AI-assisted changes](https://pages.nist.gov/nccoe-devsecops/introduction.html)

## PDS Analyzer Priorities

Use deterministic tools before model inference and preserve precise source/
runtime locations. Build only the narrow analyzer composition required by the
pilot; do not create a custom parser or general code-search engine.

### Static Analysis Checklist And Data Discovery Checklist

Use the stack-specific checklist below for both source analysis and governed
data/runtime discovery. Coverage is bounded by the pilot charter and evidence
register; checking every item is not a substitute for the human adequacy gate.

### .NET and IIS

- Use MSBuild/Roslyn for project/framework/package graphs, symbols, routes,
  controllers/services/jobs, call/data flow, auth/session/cache/configuration,
  EF/Dapper/ADO.NET/raw SQL, errors, reflection/native/COM, tests, and source
  locations.
- Capture ASP.NET/WebForms/MVC/Web API/WCF, Windows services and scheduled
  workers, file shares/registry/certificates, impersonation/Windows identity,
  generated code, and environment transforms.
- Treat IIS site/application/virtual-directory/app-pool identity, bindings,
  certificates, authentication/modules, recycling, and inherited
  `ApplicationHost.config` as runtime evidence outside the repository.

### Angular and web experience

- Use TypeScript/Angular workspace and compiler metadata for versions, routes,
  lazy boundaries, components/templates/directives/pipes, forms/validators,
  guards/resolvers/interceptors, API calls, RxJS/state/cache/storage, build
  configuration, feature flags, localization, analytics, and network behavior.
- Characterize critical tasks through DOM, network, visual, accessibility,
  keyboard, responsive/mobile, error/recovery, abandonment, and support
  evidence. Reconstruct personas, tasks, state, rules, and outcomes rather than
  legacy pixels or one-to-one components.

### SQL Server

- Combine DACPAC/catalog extraction, DacFx/ScriptDom, static dependencies,
  keys/constraints/indexes/grants, application call sites, Query Store and
  approved traces. Static analysis alone cannot prove dynamic SQL or external
  behavior.
- Inventory tables/views/types, procedures/functions/triggers/jobs, linked
  servers/synonyms/CLR/Service Broker/SSIS, sensitive fields, retention,
  transactions/isolation, locking/retry/error semantics, data quality, and
  migration/reconciliation constraints.
- Treat material stored procedures as behavioral APIs until input/default/null
  rules, result sets/order/cardinality, reads/writes/side effects, callers,
  runtime frequency/latency/failures, security context, and dependent systems
  are characterized.

### AWS ECS and operating runtime

- Capture clusters/services/task definitions, image digests, ports/health,
  deployment controllers/history, IAM roles, networks/security groups/load
  balancers/service discovery, scaling/capacity, scheduled tasks, dependencies,
  configuration and secret references without secret values.
- Reconcile logs, metrics, traces, alarms, dashboards, incidents, deployment
  correlations, AWS Config history where available, and actual support/rollback
  behavior.

### Jira, Confluence, and ServiceNow CMDB

- Preserve Jira issue/changelog and Confluence page/version native IDs,
  authorship where allowed, links, ACL/classification, freshness, and hashes.
  Current descriptions are evidence of intent/history, not deployment truth.
- Use CMDB for identity, ownership, environment, support, lifecycle, and service
  topology only to the degree discovery source, IRE authority, freshness, and
  CMDB health support the claim. Reconcile with source and runtime.
- Start read-only. Keep raw content in an approved encrypted evidence store.
  Interactive MCP retrieval is useful but cannot be the sole durable snapshot;
  inferred CMDB relationships require human review and governed IRE write-back.

If approved live discovery cannot run, record the exact gap and use sanitized,
versioned exports. Summarize claims and references rather than copying large
legacy source or enterprise records into repository docs or agent prompts.

## Stored Procedure Disposition

Every significant routine should receive one disposition:

| Disposition | Meaning |
| --- | --- |
| Eliminate | Logic is obsolete, duplicated, or presentation-only. Keep evidence and remove from target scope. |
| Generated query/CRUD | Logic maps to generated entity operations, QueryIR filters, relationships, aggregation, or pagination. |
| Product service | Logic is real domain workflow and belongs in human-owned service/handler code with tests. |
| Custom method / DTO | Logic returns report-shaped data or commands that are not plain CRUD. Model the contract explicitly. |
| Provider routine, temporary | Routine is wrapped only to preserve behavior during strangler migration. Add retirement criteria. |
| Provider routine, retained | Routine is intentionally retained for provider-specific capability, with parity limits documented. |

Large stored procedures should not become a permanent black box by default.
Decompose them into product services, generated query surfaces, custom methods,
or explicit provider routines with tests and exit criteria.

## Model And UI Rules

- Model durable business entities, not legacy table names by default.
- Preserve real keys and relationships where they are part of business identity,
  but do not expose raw primary keys in product URLs.
- Use DTO/result entities for reports and complex read models.
- Rebuild UI flows with generated model/API contracts, server-side grids,
  validation, lookup selectors, dashboards, accessibility, dark/light mode, and
  product design tokens.
- Keep legacy screenshots as intent evidence, not as frontend implementation.

## Verification Loop

The CLI proof below is necessary but not sufficient for the target agentic
capability. Until a typed Modernization IR artifact contract is implemented,
retain the reviewed evidence register, as-is model, treated model, decisions,
and projection traceability in product-owned, access-appropriate records and
link them from handoff. Do not invent a green CLI status for those missing
contracts.

Minimum modernization slice:

```bash
scripts/appfw product validate --json
scripts/appfw product scaffold-model --dry-run --json
scripts/appfw product generate
scripts/appfw product generate --check --json
scripts/appfw product test --fast --json
scripts/appfw product handoff --json
```

When live services are available:

```bash
scripts/appfw product migrate plan --json
scripts/appfw product migrate lint --phase all --json
scripts/appfw product migrate drift --json
scripts/appfw product api-test
scripts/appfw product frontend-test --json
scripts/appfw product release-check --json
```

For provider or routine behavior, add framework provider certification or
product API scenarios that prove the exact data-source semantics touched by the
slice.

## Handoff Requirements

The handoff must name:

- charter version, modernization slice, approved coverage denominator, excluded
  capabilities, named business/domain/data/security/operations/modernization/
  release owners, value metric, baseline economics, proof standard, and stop
  criterion;
- evidence register summary, including code commit, Jira issues/changelogs,
  Confluence pages/versions, data/runtime/operating evidence, hashes,
  freshness, classification, and access limits;
- reproducible build/runtime baseline or explicit accepted failure record,
  deterministic analyzer versions and coverage, dynamic observations, and
  known environment differences;
- as-is model version, claim-specific authority, human adequacy decision,
  contradictions, confidence/evidence grades, unresolved unknowns,
  invalidation triggers, and characterization baseline;
- modernization options considered, selected dispositions, rejected options,
  and approved treated-model version;
- projected entities, relationships, DTOs, policies, operations, custom
  methods, services, tests, and source-to-decision traceability;
- each portable worker's `ModernizationTask`, capability handshake, worker/
  model/harness versions, evidence used, exact commands and checks,
  `ModernizationResult`, scope deviations, failures, unknowns, sensitive-data
  events, human interventions, runtime, and cost;
- stored procedure disposition and retirement plan;
- data classification assumptions, unresolved sensitive-data questions,
  migration/reconciliation, coexistence, rollback, cutover, observability, and
  support posture;
- independently obtained build, behavior, data, SQL, integration, permission,
  security/privacy, UX/accessibility, performance, resilience, migration,
  rollback, and retirement evidence appropriate to the slice;
- generated artifacts, tests, migration evidence, skipped live checks, current
  CLI/IR limitations, exact non-claims, and proof that generated plans, code,
  and tests did not serve as their own acceptance oracle; and
- remaining strangler, parity, performance, security, operations, release, and
  retirement risks with owners and criteria, plus realized/residual cost and
  fully loaded cost per accepted production-capable capability.
