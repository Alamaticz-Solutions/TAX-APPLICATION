# Roadmap

This roadmap is the current enterprise scorecard and release-readiness path for
App Framework. It is not a changelog or PR diary. Historical implementation
details belong in git history or `docs/archive/`; current implementation
contracts belong in the docs linked below.

Use [App Framework Platform Strategy](../strategy/app-framework-platform-strategy.md)
as the business-value "why" for this platform, use
[Product Development Strategy - North Star](../strategy/product-development-north-star.md)
as the durable strategy companion for how that value stays trustworthy, and use
[App Framework Product Management Strategy](../strategy/app-framework-product-management-strategy.md)
as the product overlay for demand streams, audience promises, market/trend
vetting, ranked value themes, packaging strategy, PM waves, and
business-value review criteria. This roadmap owns current-state facts,
blockers, and execution order. The evidence-calibrated capability maturity
table in the Product Management Strategy owns current/target maturity and
relative-effort ratings; the engineering inventories below must not be read as
competing readiness scores. If the strategy docs and this roadmap ever
disagree about current state, this roadmap wins and the strategy docs should be
updated.

Use this file to answer three questions:

1. How ready is the framework?
2. What blocks production release?
3. Which work should move next, and why?

## Operating Context

Current assessment date: July 11, 2026. Execution state of the North-Star plan:
**Waves 0-3 produced substantial contract, enforcement, packaging, provider,
mobile, and evidence foundations; selected Wave 4 slices have also landed.
Wave labels are historical sequencing aids, not current maturity or completion
claims.** The Wave 2 evidence remains live-gated:
`scripts/appfw framework wave2-status --json` now includes CH6 chat-eval posture
in the aggregate rollup alongside the existing governed-write, mobile,
governance, developer-loop, and live-provider lanes. Deterministic CH6
local-fixture evidence can prove local prompt-injection posture, but release
readiness still requires managed judge/live evidence from a certified gateway and
search provider.

Current release posture:

- Release readiness: **gated** pending one approved, provider-backed release
  bundle that satisfies the Production Exit Checklist.
- Production readiness: **not certified**.
- Release code blockers: no known original P0 code blockers remain open.
- Main remaining risk: live release evidence still needs to be produced and
  retained from an approved provider-backed release environment.

This docs refactor improves clarity, maintainability, agent routing, and
deployment guidance. It does not by itself certify production; production
certification still depends on live provider, security, operations, frontend,
performance, supply-chain, and release-evidence artifacts.

Business-value rationale lives in the
[Platform Strategy](../strategy/app-framework-platform-strategy.md), strategy
direction lives in the [North Star](../strategy/product-development-north-star.md),
and product prioritization guidance lives in the
[Product Management Strategy](../strategy/app-framework-product-management-strategy.md).
The Platform Strategy explains the growth, productivity, experience,
rationalization, trust, capability-reuse, and ROI thesis behind the platform.
The North Star's four commitments (contracts-are-the-product; agent-operated;
PDS capability contracts remain portable across vendor and PDS channels;
security by design) are the durable control principles behind every item below.
The product strategy adds the demand-stream lens: citizen-developed
applications, legacy modernization, greenfield products, and PDS Nexus; it also
owns the anti-fad tests and ranked value themes used to decide whether a theme
is worth funding. Per the North Star's **Faithfulness Contract**, every item in
this roadmap traces to one of those commitments, and every commitment has an
item here (or an explicit note that it is not yet scheduled). This roadmap does
not restate the strategy — it sequences and proves it.

## Engineering Assurance Inventory

These qualitative postures describe current engineering evidence and the next
proof gap. They are not product maturity, production readiness, or comparative
market scores. The Product Management Strategy's 0-5 capability table is the
authoritative maturity scorecard.

| Attribute | Evidence posture | Current Meaning | Next Lift |
| --- | --- | --- | --- |
| Resilience | Strong local foundation; managed proof missing | Runtime limits, graceful failure behavior, release evidence capture, frontend dirty-state/delete/relationship UX, and Snowflake preflight fallback are strong. | Certify live provider failure, timeout, rollback, degraded readiness, and audit continuity. |
| Maintainability | Strong structure; operational hygiene partial | Runtime/provider boundaries, generated ownership, componentized CRM frontend patterns, docs IA, and docs-check contracts give agents clearer edit surfaces. | Retire worktree/branch residue, slim product templates, and keep durable program state out of ignored output roots. |
| Secure by design | Strong controls; live evidence gated | Non-local GraphQL introspection is JWT/role/scope gated, MCP is release-excluded unless certified, edge defaults are hardened, SBOM/security-assurance gates exist, and regulated classifications are validated. | Produce live tenant/IDOR/denied-mutation/audit evidence and enterprise DAST/SAST/ASVS/provenance/signing evidence or formal acceptance. |
| Agentic fit | Strong harness; throughput proof partial | Agents have task routing, CLI contracts, JSON evidence, generated ownership, handoff state, and independent review. | Reduce routing overhead and prove downstream app delivery improvement rather than agent activity. |
| Clarity | Partial | Reader-job docs and lineage guidance exist, while stale historical wave detail and ignored coordination state remain. | Generate concise current views from tracked program contracts and archive superseded detail. |
| Developer loop efficiency | Measured; target missed | Changed-only selection and retained subcheck timing exist, but full docs-check remains above the 60-second target and same-worktree parallelism is unsafe. | Isolate report roots, cache or split the next slow group, and test synthetic merge results. |
| Observability | Strong foundation; managed correlation gated | Metrics, readiness summaries, slow-query context, ops-certification, Alertmanager proof hooks, frontend timing, and load-test artifacts are in place. | Prove live OTLP/SIEM export, runbook drills, and cross-platform journey correlation. |
| High performance | Strong guardrails; PDS-scale proof gated | QueryIR caps, signed/keyset pagination, selected-field queries, thresholded load-test JSON, and provider-performance evidence exist. | Run PDS-profile load, pool saturation, provider-specific performance, recovery, and cost proof. |
| Frontend product experience | Broad component/accessibility foundation; composition maturity partial | The tracked PDS catalog defines 95 components across ten families, while CRM/admin/mobile references and automated checks prove useful engineering patterns. Component count and catalog evidence do not yet prove coherent hierarchy, complete-task accessibility, or measured product advantage. | Produce and task-test the admin, approved Nexus/My Work, and native-mobile reference compositions; then enforce the selected grammar through generated tokens, floorplans, accessible behavior, and quality evidence. |
| Product guidance | Strong contract; adoption proof partial | Product teams have a golden path, lifecycle matrix, frontend contract, deployment evidence path, and release expectations. | Prove package-first create/upgrade across repeated downstream products. |
| Enterprise features | Broad foundation; several live paths gated | Auth, policy, audit, database providers, frontend foundations, security evidence, and release controls are first-class. | Complete named SaaS execution, permission-preserving projection, managed mobile, and production operations through funded archetypes. |

## Release Evidence Inventory

| Release Dimension | Evidence posture | Current Evidence | Blocker Before Production |
| --- | --- | --- | --- |
| Functional correctness | Strong local | Validation, generate-check, docs-check, fast tests, focused runtime/codegen tests, frontend-test, load-test shape checks, and provider-performance shape checks pass locally. | Rerun full tests and release-check in the approved release environment. |
| Backend live readiness | Partial | Readiness has request context and failing data-source summaries; selected database provider evidence exists. | Produce one retained, approved provider-backed live readiness/certification bundle for the named release profile. |
| Security and IDOR posture | Strong controls; live proof gated | Introspection auth, MCP release exclusion, locator/IDOR direction, mutation access-filter hardening, regulated classification validation, and threat-model dispositions are in place. | Retain live negative tests for tenant isolation, IDOR, denied mutations, introspection roles/scopes, and audit redaction/chain across providers. |
| Provider and migration parity | Strong database foundation | Provider certification, parity semantics, and migration/lint/drift commands exist. | Run the named release profile's required providers together with `ok=true`; do not infer SaaS execution from database certification. |
| Frontend product readiness | Strong automated reference proof; product/task proof partial | `scripts/appfw frontend-test --json` and PDS catalog checks cover dense CRM workflows, component contracts, selected axe/browser states, and responsive checks. They do not establish manual assistive-technology quality, production performance, or superiority over a vendor baseline. | Retain source-bound downstream product evidence, complete operational states, manual accessibility, visual/responsive and performance budgets, representative task outcomes, and live-backend browser smoke when release scope requires it. |
| Observability and operations | Partial | Ops-certification, Alertmanager hooks, no-network OTLP config proof, runbook references, and metrics/readiness artifacts exist. | Prove live OTLP export, Alertmanager endpoint, runbook drill, Grafana/API evidence, and SIEM/audit export if in scope. |
| Performance guardrails and load evidence | Partial | Query caps, load-test suite, provider-performance artifact, and strict release-evidence validation exist. | Produce passing PDS-profile backend load, provider-specific performance, pool saturation, recovery, and safe diagnostics evidence. |
| Release gate and supply chain | Strong gate foundation | SBOM, secret, PHI, SCA, release-evidence, security-assurance, ops-certification, frontend-test, load-test-suite, and provider-performance gates are wired. | Run strict release evidence in CI and provide managed DAST/SAST/ASVS/provenance/signing evidence or formal human disposition. |
| Reproducibility and change control | Strong local | Root-aware CLI, lock/upgrade, handoff JSON, artifact manifests, generated drift checks, and docs-check contracts exist. | Start final evidence from a clean checkout and retain release artifacts immutably. |
| Production certification | Not certified | Local engineering evidence is strong and missing production categories have explicit gates. | Live provider-backed release, security, operations, performance, supply-chain, support, and promotion evidence must pass together. |

## Production Exit Checklist

Before a production release, produce and retain these artifacts from a clean
checkout or approved release pipeline:

1. `scripts/appfw release-check --json` with provider-backed services running.
2. `scripts/appfw provider-test --all --json` with PostgreSQL, MongoDB,
   MS SQL Server, and Snowflake certified in one bundle.
3. Strict supply-chain and release evidence:
   `scripts/ci/supply-chain-gate.sh`,
   `scripts/appfw dependency-check --json --strict`,
   `scripts/ci/secret-scan.sh`, PHI log lint,
   `scripts/ci/release-evidence-check.sh --strict`, Rust/frontend SBOMs,
   `target/appfw/dependency-check.json`, active `dependency-check.toml`
   OSV acceptances when needed, and a deployable image SBOM when a release
   image exists.
4. Live security certification: introspection auth cases, tenant isolation,
   locator/IDOR negative tests, denied mutations, policy/access filters, audit
   redaction, and audit chain behavior.
5. Security-assurance decision: DAST, SAST-equivalent tooling, OWASP ASVS
   traceability, release provenance, and artifact-signing evidence, or
   approved time-boxed risk acceptance.
6. Live operations evidence when production ops proof is required:
   `target/appfw/ops-certification.json`, live OTLP evidence,
   `APPFW_ALERTMANAGER_URL` proof, readiness/metrics snapshots, dashboard
   evidence, and runbook drill output.
7. Live performance evidence when performance proof is required:
   `scripts/appfw load-test-suite --json`,
   `scripts/appfw provider-performance --json --all`, pool saturation proof,
   and safe EXPLAIN/slow-query diagnostic evidence.
8. Frontend evidence when the CRM/frontend release scope is included:
   `scripts/appfw frontend-test --json`, `scripts/check-pds-components.mjs
   --json`, product-intake scaffold proof, and live-backend smoke when required
   by release policy.
9. Immutable image digest, artifact hashes, handoff JSON, deployment metadata,
   and promotion annotations.
10. CAB/change-advisory package when the release or deployment requires formal
    change approval: business impact, affected services, risk, test evidence,
    deployment plan, rollback plan, monitoring/validation, communications, and
    approval linkage. **TODO:** create a report-only package harness similar to
    the SRA package flow so this evidence can be retained before CAB review.

Tasks that require managed providers, release CI, live observability systems,
security tooling, or release authority are tracked in
[Live Environment Work Items](live-environment-work-items.md). Keep repo-side
changes focused on making those work items executable, diagnosable, and
reviewable.

## Highest Priority Work

These are the external and managed-environment evidence items that block a
production release claim. They form a **release-evidence track**, not the
default framework producer order. Security, SRE, provider, and release owners
may advance this track when approved environments and evidence inputs exist.
During a human-authorized development window, the
[Outcome Goal Register](#outcome-goal-register) governs producer order unless
the human declares a named release candidate or production deadline that
temporarily overrides it.

| Priority | Work Item | Why It Matters | Primary Docs |
| ---: | --- | --- | --- |
| 1 | Provider-backed release-check and provider certification | Converts strong local proof into release evidence. | [Release Gate](release-gate-ci-cd.md), [Provider Certification](../runtime/provider-certification.md) |
| 2 | Live security certification and strict release evidence | Proves the security controls in the environment that will release. | [Security Threat Model](../architecture/concerns/threat-model.md), [Deployment Reference](deployment-reference.md) |
| 3 | Live operations evidence | Makes readiness, telemetry, alerting, and runbooks reviewable instead of aspirational. | [Observability](../runtime/observability.md), [Deployment Reference](deployment-reference.md) |
| 4 | Live performance evidence | Moves high performance from guards and synthetic artifacts to provider-backed proof. | [Performance And Scalability](../runtime/performance-and-scalability.md), [Deployment Reference](deployment-reference.md) |
| 5 | Golden downstream lifecycle and frontend scaffold execution | Proves product teams can create, customize, generate, test, upgrade, and ship without copying framework internals. | [Golden Path](../lifecycle/product-golden-path.md), [Application Lifecycle](../lifecycle/application-lifecycle.md), [Frontend Starter Contract](../frontend/product-frontend.md) |
| 6 | Product packaging, versioning, and compatibility matrix | Turns the framework into a consumable enterprise platform product. | [Framework Packaging](../architecture/framework-packaging.md), [Product Workspace Contract](../reference/product-workspace-contract.md), [Versioning And Compatibility](versioning-and-compatibility.md) |

North-Star basis (Faithfulness Contract rule 1): **P2** → commitment 4
(security by design); **P5** → commitment 1 (contracts-are-the-product / golden
path); **P6** → commitment 1 + 2 (a consumable, agent-operable platform);
**P1/P3/P4** → North Star Part 2 (control systems: live test/observability/
performance evidence) and the Part 7 readiness metrics.

## Program Roadmap Horizons

This is the current human-facing roadmap structure. Use the terminology contract
in the [Program Work System](../reference/program-work-system.md): the work
breakdown is Outcome -> Product Increment -> Deliverable -> Task; Roadmap
Horizon and Product Increment order define when; Delivery and Evidence are
linked execution and proof dimensions.
The historical Wave sections below remain useful delivery lineage, but they are
not the current executive hierarchy or maturity model.

| Horizon | Decision purpose | Outcome Goals | Advancement rule |
| --- | --- | --- | --- |
| **AF-RH0 — Trustworthy foundation and reference experience** | Make App Framework safe and smooth enough for a real product while proving the competitive experience direction in one bounded reference composition. | **AF-OG01**, **AF-OG05** | Close the product-neutral readiness exits; the CRM experience proof may advance independently on disjoint surfaces but earns no production or enterprise-front-door claim. |
| **AF-RH1 — First PDS employee product and governed read** | Move from framework-self proof to one independently owned package consumer and one authoritative read path. | **AF-OG02**, **AF-OG03** | A named employee product, owner, source object, approved environment, and operating owners exist; no broad capability family opens ahead of the vertical. |
| **AF-RH2 — First production-grade vertical** | Make the same capability safely actionable, deployable, observable, recoverable, and supportable. | **AF-OG04** | Read and projection authority are proven; one low-risk action and one managed profile satisfy their release evidence. |
| **AF-RH3 — First lovable PDS enterprise experience** | Turn the proven vertical into the first bounded, coherent PDS-owned employee product. | **AF-OG07** | Product authority, trust gates, package lifecycle, comparative baselines, launched scope, support model, and stop/narrow thresholds are explicit. |
| **AF-RH4 — Enterprise work and administration** | Extend the product from personal participation into deep work, administration, and governed low-risk configuration. | **AF-OG11** | The first lovable product operates; code-native behavior remains separate from governed content, audience, navigation, role-default, module-placement, and flag administration. |
| **AF-RH5 — Provider-neutral intelligent experience** | Add structured PDS Health AI, search/reasoning portability, exact channel continuation, and measured intelligence to the proven product. | **AF-OG06** | Intelligence uses the existing context, permission, evidence, operation, and telemetry spine rather than creating a parallel assistant or model platform. |
| **AF-RH6 — Product factory and portfolio scale** | Prove compounding product economics, modernization where funded, and broad lifecycle operations only for repeatedly consumed capabilities. | **AF-OG08**, **AF-OG09**, **AF-OG10** | A second consumer or named modernization pilot is funded; reuse and operating evidence justify durable platform, data, security, mobile, SRE, and support ownership. |

### Outcome Goal Register

Planning posture describes capacity timing, not delivery completion. A Milestone
counts as accepted only when its required evidence is current and accepted; a
branch, review, merge, or pipeline alone does not close it.

| Outcome Goal | Planning posture | Business result and exit | Milestones | Strategic Goals |
| --- | --- | --- | --- | --- |
| **AF-OG01 — Framework Ready For Nexus PoC** | **Active** | A product team can start a later PoC without first untangling checkout dependencies, model/config ambiguity, slow or false feedback, unsafe authorization, inaccessible states, opaque failures, or a nominal upgrade. | **AF-M01.1** security/evidence truth; **AF-M01.2** no-checkout creation and clear model/config path; **AF-M01.3** authoritative permissioned read and provider simulation; **AF-M01.4** reusable accessible web, bounded mobile read, and correlated errors/audit/telemetry; **AF-M01.5** genuine behavior-affecting upgrade. | AF-SG01, AF-SG05, AF-SG06, AF-SG07 |
| **AF-OG02 — Real Package Consumer And Upgrade** | **Next** | One independently owned product outside this repository consumes approved packages, owns useful behavior, has no framework path dependency, and completes a real upgrade. | **AF-M02.1** product, owner, repository, and outcome named; **AF-M02.2** package-only build/test/deploy path; **AF-M02.3** compatibility evidence and behavior-affecting upgrade. | AF-SG01, AF-SG07 |
| **AF-OG03 — Governed Read Vertical** | **Next** | The same version-pinned enterprise object is read from its source and a permission-preserving MongoDB projection through one identity, policy, operation, provider, audit, and telemetry spine. | **AF-M03.1** authoritative runtime spine; **AF-M03.2** live permissioned ServiceNow read; **AF-M03.3** freshness-, entitlement-, revocation-, reconciliation-, and fallback-aware MongoDB projection. | AF-SG02, AF-SG03, AF-SG05 |
| **AF-OG04 — Governed Action And Managed Operations** | **Next after AF-OG03** | One low-risk source action and one managed deployment profile prove safe execution and production operation. | **AF-M04.1** delegated identity, preview, confirmation, narrow scope, idempotency, audit, reconciliation, and revocation; **AF-M04.2** PDS Observability, load/cost, outage, RTO/RPO, recovery, security, and support evidence; **AF-M04.3** named release-profile decision. | AF-SG03, AF-SG05 |
| **AF-OG05 — Competitive Signature Experience** | **Active, bounded** | PDS experiences demonstrate calm precision, expressive intelligence, and unmistakable craft without a broad visual rewrite or unsupported superiority claim. | **AF-M05.1** CRM Activities web queue/detail signature proof; **AF-M05.2** corresponding native-mobile continuation when inherited mobile seams are healthy; **AF-M05.3** composition-consumed tokens, floorplans, behaviors, motion roles, and agent recipes; **AF-M05.4** operational and blinded signature-quality comparison with no accessibility, trust, reliability, or recovery regression. | AF-SG01, AF-SG02, AF-SG04, AF-SG07 |
| **AF-OG06 — Provider-Neutral Intelligent Experience** | **Held behind AF-OG04 and the first product** | A named task converts authorized state and signals into durable typed intelligence, governed attention and bounded layout, a checkpoint/action, exact web/native continuation, and a measured outcome without binding the product to one search or reasoning provider. | **AF-M06.1** live authenticated PDS Health AI structured result plus provider-neutral capability resolution and deterministic fallback; **AF-M06.2** typed intelligent modules with evidence-backed web/native continuation, governed attention, and bounded adaptive layout; **AF-M06.3** retrieval/model/tool/render/action/outcome trace, quality, cost, correction, support, and kill-switch evidence. | AF-SG04, AF-SG05 |
| **AF-OG07 — PDS-Owned Enterprise Experience Product Proof** | **Shaped; release after AF-OG01-AF-OG04 product inputs** | One real downstream package consumer proves a bounded first lovable PDS employee product while ServiceNow remains authoritative for workflow and fulfillment. It is not yet a production front-door claim. | **AF-M07.1** product, journey, source, operating authority, Product Experience Model, and comparative baselines; **AF-M07.2** package-only product with live ServiceNow read, permission-preserving projection, governed action, managed deployment, and genuine upgrade; **AF-M07.3** coherent web/native-mobile experience with durable attention, exact resumption, structured PDS Health AI, signature quality, and product-level Claude Code/Codex harness; **AF-M07.4** managed trust, task, accessibility, support, resilience, adoption, and lifecycle-economics decision with explicit stop/narrow rule. | AF-SG01, AF-SG02, AF-SG04, AF-SG05, AF-SG07 |
| **AF-OG08 — Product-Owner And Citizen Product Factory** | **Later** | Product owners and citizen-product teams use the Product Experience Model and governed product harness to deliver a second real product without rebuilding context, security, action, rendering, telemetry, evaluation, or operations. | **AF-M08.1** second consumer, owner, outcome, and approved target model; **AF-M08.2** reusable packages, contracts, recipes, and Claude Code/Codex product harness extracted from two uses; **AF-M08.3** at least 50% lower normalized second-use effort without lower quality. | AF-SG01, AF-SG07 |
| **AF-OG09 — Agent-Portable Legacy Modernization Control Plane** | **Shaped and held** | A named pilot proves a first-class As-is Modernization IR, separate Approved To-be Treatment Model, target compilation, independent equivalence, migration, and retirement economics. | **AF-M09.1** chartered pilot and manual baseline; **AF-M09.2** governed evidence and As-is IR adequacy; **AF-M09.3** objective, human-approved To-be Treatment Model; **AF-M09.4** one compiled App Framework vertical; **AF-M09.5** independent behavior/data/security/experience/operations and migration/retirement proof; **AF-M09.6** second suitable application reuse. | AF-SG01, AF-SG06, AF-SG07 |
| **AF-OG10 — Portfolio Scale And Lifecycle Economics** | **Later** | Proven application-production, enterprise-experience, and capability-runtime advantages operate across multiple supported products with measurable reuse and a deliberately chosen PDS enterprise boundary. | **AF-M10.1** Kafka/checkpoint/replay/DLQ only where a product needs it; **AF-M10.2** managed native-mobile and enterprise-experience operations; **AF-M10.3** PDS-scale capacity, resilience, recovery, cost, and support; **AF-M10.4** managed tenant control plane only for true independent isolation; **AF-M10.5** portfolio release, upgrade, observability, support, adoption, and unit-economics operations. | AF-SG03, AF-SG05, AF-SG07 |
| **AF-OG11 — Enterprise Work Execution And Administration** | **Held behind AF-OG07** | The PDS employee product supports deep work/admin experiences and governed low-risk administration without turning App Framework into a workflow engine or allowing configuration to alter authority. | **AF-M11.1** deep queue, entity, process, and operational workspaces with keyboard/accessibility/data-scale proof; **AF-M11.2** admin operation, journey/attention trace, module eligibility, layout reset, AI evaluation/cost, and recovery views; **AF-M11.3** governed administration for content, audiences, navigation, role defaults, module placement, and flags with versioning, preview, audit, rollback, and policy locks. | AF-SG02, AF-SG04, AF-SG05, AF-SG07 |

### Held AF-OG08 / AF-OG10.5 Decomposition

The [Hosted Product Factory
Spec](../specs/hosted-product-factory.md#held-delivery-plan-and-effort)
organizes the application-registry, per-app workspace, contextual agent,
promotion, and portfolio idea into the existing milestones below. This is
planned decomposition: AF-OG08 and AF-OG10 remain **Later** and the current
priority order is unchanged. By the recorded 2026-07-25 D1 decision,
AF-D08.1a's FAB-A1 contract Assignment was admitted as the bounded early lane
(Option B) with the harness as implementer. Its contracts are merged and
verified on `main`, but no distinct Product-owner acceptance receipt is
recorded. FAB-A2–A4 still require exact admission and authorized acceptance
evidence before AF-D08.1a exits, and no scaffold, branch, review, or pipeline
earns FAB-A5/FAB-A6 or milestone credit by existing. The
[App Fabric Master Implementation
Plan](app-fabric-master-implementation-plan.md) is the single subordinate
narrative source for integrated stages, effort, dependencies, decision gates,
and proof outcomes, including the proposed Operations Console, portfolio/value,
pattern, cross-team intent, mobile-release, and AI-analysis extensions. It is
not a second status or admission authority and does not advance AF-OG08 or
AF-OG10 by being merged. Current work, status, and branch topology must be
recorded once in the canonical Product Increment portfolio, a validated
Product Increment plan, PFC topology, and accepted evidence.

| Existing Milestone | Held Deliverables | Exit Evidence | Stop / Narrow Rule |
| --- | --- | --- | --- |
| **AF-M08.1 — second consumer, owner, outcome, and approved target model** | **AF-D08.1a** authority, schema, RBAC, privacy/freshness and stop-rule foundation; **AF-D08.1b** read-only fleet registry/projector/API and one-checkout local adapter; **AF-D08.1c** fleet search and per-app workspace with CRM reference composition plus one independently owned second consumer and target model | Two registered products; exact desired/built/deployed/running identity; hosted/local schema parity; cross-app denial; stale/missing sources show `STALE`/`UNKNOWN`; named owner and outcome per app | Stop if registration duplicates manually curated state, ownership is missing, freshness cannot be proven, cross-app metadata leaks, or a control-plane outage can affect product traffic |
| **AF-M08.2 — reusable packages, contracts, recipes, and product harness extracted from two uses** | **AF-D08.2a** context-bound typed proposal through accelerated branch preview; **AF-D08.2b** candidate, exact destination verification, signed dev deployment and acceptance projection; **AF-D08.2c** strict release path for the exact Accepted/certified digest; **AF-D08.2d** reusable packages, PDS recipes, registry/context contracts, review routing and evidence patterns extracted from both products | One complete purpose-to-accepted-outcome loop; exact human/agent attribution; zero write-surface escapes; required review and human promotion paths honored; same contracts serve both products | Stop if chat becomes authority, review capacity is overwhelmed, unsafe changes bypass engineers, product teams require bespoke control-plane code, or accelerated proof is presented as release readiness |
| **AF-M08.3 — at least 50% lower normalized second-use effort without lower quality** | **AF-D08.3a** comparable first/second-use baseline; **AF-D08.3b** whole-cost evidence including human effort, agent tokens, compute, review, retries, support, defects, accessibility, security and rollback plus task-success, responsive/visual, first-impression and comparative-preference quality; **AF-D08.3c** human continue/narrow/stop decision | At least 50% lower normalized second-use effort with no regression in acceptance, security, accessibility, signature/visual quality, representative task success, reliability, maintainability or support | If the threshold fails, retain useful registry/workspace capabilities but do not claim or scale a citizen product factory |
| **AF-M10.5 — portfolio lifecycle operations** | **AF-D10.5a** conditional citizen self-service graduation after AF-M08.3: sponsor/classification intake, idempotent repository bootstrap/protections, bounded WIP/budgets, operational onboarding/support/retirement and progressive rollout; then fleet-wide release/upgrade posture, source-bound build/deployment history, composition, usage, separately sourced satisfaction, build/runtime AI cost, SLO/support, adoption, and invest/pause/consolidate/retire decisions | Named sponsors use current evidence to make lifecycle and investment decisions; self-service passes adversarial authorization and kill-switch proof; cost per accepted outcome and upgrade health are measurable | Stop if authorization escapes, review/support capacity fails, the catalog becomes a CMDB duplicate, parallel backlog, token leaderboard, vanity reporting layer, or reporting that never drives investment or retirement |

The first CRM vertical is the bounded reference proof for contracts and one
complete proposal-to-dev loop; it does not satisfy the independent second-
consumer milestone by itself. Citizen repository self-service follows the
AF-M08.3 economic and safety decision rather than preceding it. Application
team ownership permits scoped context, proposal and product-acceptance work;
merge, production promotion, security/classification, secrets, dependency
policy, risk and cross-app authority remain separate.

**Economic advancement rule:** PDS-owned enterprise experience is the strategic
target, but ownership does not waive the economic proof bar. The $4.549 million
common Pega redesign/transition cost does not fund or justify an Outcome Goal.
App Framework increments must contribute evidence toward clearing at least the
modeled $4.255 million three-year hybrid premium, equivalent to an unadjusted
$1.418 million annual hurdle, before risk margin. The wider target has a
directional planning range of 30-45 blended person-months for a bounded first
lovable product, 75-120 for broad front-door maturity, and 6-10 durable FTE
equivalents for ongoing operation; these are not commitments. Accepted value may
come from measured reuse, avoided services/support, greater product throughput,
application retirement, portability, or a structurally better repeated-use
experience; contracts, visual novelty, and delivery activity do not count.

Product signal intake, value scoring, agentic review, technical-debt control,
SRA/release preparation, outcome telemetry, delivery-loop improvement, and the
dashboard continue as cross-cutting controls. They become Outcome Goals only
when a named measurable change requires dedicated investment; activity in
those controls does not substitute for product progress.

## Historical Delivery Lineage

The [Outcome Goal Register](#outcome-goal-register) above is the sole current
priority and dependency authority. The wave, G/U/D, and P-item material below
preserves delivery lineage, stable identifiers, and evidence history; it must
not be read as a second current producer queue. New work uses an Outcome Goal,
Milestone, and Slice mapping before XO releases it.

**How to work from this plan (XO-coordinated guidance for agents and humans):**
use the [Agentic Development Control System](../architecture/concerns/agentic-development-control-system.md)
when a branch is broad, sensitive, multi-domain, or claims release/live
readiness. The roadmap gives priority and dependency order; the control system
decides when an agentic change needs explicit human review and retained impact
evidence before merge.

**Historical Wave 0 rule:** one owner ran Wave 0 (W0.1-W0.8) to completion,
serially. No P-item
and no numbered strategic item starts until Wave 0 is done — including P1, whose
provider-cert work touches the same capability matrix frozen in W0.2. Then fan out
Wave 1 lanes in parallel, then Wave 2 (dependent) chains. Do not start a Wave-1/2
item until its Wave-0 freezes and its spec exist. Each item names its
proof/done-state; where the proof does not exist yet, the Wave-0 spec must create
it — **a spec is "done" only when `release-evidence-check.sh` / `docs-check` has a
new assertion that fails without the spec artifact** (this is what stops an agent
self-grading against invented criteria).

### Wave 0 — freeze direction and shared contracts (serial, single-owner)

Every lane below builds on these; editing them concurrently from multiple lanes
guarantees merge collisions and drift, so they are settled first in small slices.

| Freeze | What to settle | Proof |
| --- | --- | --- |
| W0.1 Direction | North Star reconciled with the folded research and the React Native mobile direction (done 2026-07-01, rev 3); the four missing items G1–G4 added as rows below. | `framework docs-check --changed-only --json` |
| W0.2 Provider capability enums | Frozen 2026-07-01: `SaasReadArea` now has explicit opt-in governed-write gates (`DelegatedActorContext`, `TokenStoreIsolation`, `NamedMutationRegistry`, `MutationRequestBinding`, `IdempotencyAndReplayProtection`, `WritePolicyAndScopeEnforcement`, `WriteAuditAndEvidence`) while all SaaS writes remain `Unsupported` until G1/U4 evidence exists. Live `provider-test --all --json` remains the release/provider certification proof and requires provider-specific backend URLs. | `cargo test -p appfw-runtime provider_capabilities --lib` |
| W0.3 Provider set | Frozen 2026-07-01: `FrameworkProvider::DATABASE`, `GRAPH_READ`, and `EXTERNAL_API` are asserted disjoint and complete over `FrameworkProvider::ALL`. | `feature-check --json`, `cargo test -p appfw-runtime provider_keys --lib` |
| W0.4 Model/manifest contract | Freeze `app_gen/src/app_manifest.rs` / `config_contract.rs` / `validation.rs`, including any extension that drives composed binaries/docs/skills (U7) or new ingress. | `validate --json`, `generate --check --json` |
| W0.5 Generated UI contract | Freeze `app_gen/src/frontend.rs → appfw-ui-contract.ts` + `app_gen/_templates/`; **correct the phantom `frontend/src/api_contract.ts` reference** (the real emitter is `appfw-ui-contract.ts`) before U5/U6 fork it. | `generate --check --json`, `frontend-test --json` |
| W0.6 CLI contract | Frozen 2026-07-01: U2 `product harness-check`, U4 `framework provider-graduation`, U5 `product mobile-test`, U6 `framework fork-check`, U7 `framework composition-check`, G4 `framework governance-check`, and P6 `product compat-verify` now have first executable report slices while preserving `--plan`; D6 keeps the existing docs-check/cli-test planning surface. | `framework cli-test --plan --json`, `framework docs-check --full --json` |
| W0.7 PDS + evidence registry | Frozen 2026-07-01: PDS component/token evidence remains rooted in `check-pds-components.mjs`, `check-pds-tokens.mjs`, and `appfw_ui/pds_health`; release evidence remains rooted in `release-evidence-check.sh` and `security-assurance-decision.sh`. New G2/G4/U5/U6/U7 evidence extends those registries in place. | `scripts/check-pds-components.mjs --json`, clean-dir `release-evidence-check.sh --local-fixture` |
| W0.8 Specs | Frozen 2026-07-01: [Wave 0 Spec Artifact Contracts](../architecture/concerns/north-star-wave-0-specs.md) defines artifact-shaped specs for G1, G2, G3, G4, U2, U4, U5, U6, and U7, including retained JSON artifacts and first failing assertions. | doc + `docs-check` assertions |

### Strategic order (post-release strategic work)

Leads with the critical path and its control systems, per the North Star; treats
inner-loop speed as continuous optimization, not the headline. **The `#` column is
strategic-leverage rank, not execution order** — execution must honor the
`Depends on` column and the Wave-2 chains (e.g. U5 ranks high but runs only after
U6 and G2). Lanes that own disjoint surfaces (Wave 1) run in parallel; ordered
chains (Wave 2) do not. Every row carries a stable code (G/U/D) used verbatim by
the Wave-0/1/2 references.

| # | Item | Commitment | Proof / done-state | Depends on |
| ---: | --- | :---: | --- | --- |
| 1 | **G1 — Archetype-2 delegated/on-behalf-of auth + governed-write safety lane.** CRITICAL PATH: portable operations need source authorization, delegated identity, idempotency, and audit across vendor and PDS channels. Per-user token store keyed by (user, tenant); named/audited mutation registry; write-safety evidence gate; `mcp_enabled:false` until certified. Flagship use case: PDS Nexus / De Novo write-back. Wave 1 recorded external API providers as `governed_write_certified:false` and made `governed-write-evidence.json` mandatory before any provider can claim write certification. Wave 2 adds `framework governed-write-check --json --enforce`, `target/appfw/governed-write-posture.json`, docs-check coverage, release-check execution after provider-graduation, strict release-evidence artifact validation, and `framework provider-test --provider servicenow --area governed-write --plan --json`, proving all external API providers remain write-disabled/MCP-disabled while reserving `governed-write-evidence.json` for real provider-test evidence. ServiceNow now has one local named-mutation candidate, `servicenow.create_incident`, in the provider registry and plan allow-list; live mode validates an external ServiceNow sandbox evidence file via `APPFW_SERVICENOW_GOVERNED_WRITE_EVIDENCE_FILE` and retains `governed-write-evidence.json` only when the G1 gates are proven. It is still non-executable until live G1 evidence exists. | 3, 4 | New governed-write evidence category + `provider-test` write area; LiveCertified needs an external ServiceNow instance | W0.2, W0.3, W0.7 |
| 2 | **G3 — Agentic threat model + sandboxing posture.** Map the threat model to OWASP Agentic Top 10 (ASI01 hijacking, ASI02 tool misuse, ASI03 identity/privilege abuse); state both-layers (filesystem+network) sandboxing as a framework posture. First Wave 1 slice adds the `agentic-threat-model.md` source contract and docs-check assertion. | 4 | Threat-model doc mapped to ASI01–03 + a `docs-check` assertion that fails without it | W0.1 |
| 3 | **U2 — Downstream least-privilege agent-harness profile.** Product-agent profiles: allowed commands, write boundaries, network/live-service access, handoff expectations, review checkpoints. Encodes G3's threat model and G1's Archetype-2 write limits. First Wave 1 slice adds `.appfw/agent-profile.yaml`, `product harness-check --json`, and `.appfw/target/appfw/harness-check.json` evidence. Wave 2 now tightens `saas_governed_write:true` so a product agent profile requires valid G1 governed-write evidence and enforced posture artifacts, not mere file presence; the Rust contract suite covers both rejection of incomplete G1 artifacts and acceptance only when provider-test governed-write evidence plus enforced `governed-write-check` posture are valid. | 4 | Profile schema + acceptance checker + positive/negative G1 artifact contract tests | W0.1, G3, G1 |
| 4 | **G2 — Framework-owned PDS conversational / agentic-UI primitives.** Intent Preview / confirm-before-act, Action Audit & Undo, persistent AgentTimeline panel — the human-in-the-loop control surface for governed writes. First Wave 1 slice adds `IntentPreview`, `ActionAudit`, and `UndoCompensationState` to the PDS source, manifest, static catalog, interactive catalog, snippets, docs, and component checker evidence. Wave 2 adds `governed_action_live_readiness` to `target/appfw/pds-component-check.json`, plus `scripts/check-pds-components.mjs --json --enforce-governed-action` for lanes that claim live governed-write readiness. The enforced failure path now preserves parseable JSON when G1 live evidence is missing or schema-thin, validates the same delegated actor/token-store/named-mutation/idempotency/audit shape as `wave2-status`, and docs-check has expected-failure assertions for that fail-closed posture. Wave 4 CH5 now records the `StreamingText` markdown/sanitizer decision (`react-markdown` + `rehype-sanitize`) and publishes `conversation_markdown_sanitizer.live_ready:false`; it also adds FlowGraph token bridge evidence where `.pds-flow-graph-shell__viewport` publishes PDS-backed `--xy-*` variables and `target/appfw/pds-component-check.json` records `flow_graph_token_bridge.adapter_ready:false` until a real React Flow adapter and live product evidence exist. | 3 | `scripts/check-pds-components.mjs --json` + `target/appfw/pds-component-check.json` catalog, governed-action live-readiness evidence, conversation markdown sanitizer posture, and FlowGraph token bridge posture; `scripts/check-pds-components.mjs --json --enforce-governed-action` must emit parseable JSON even while failing without strict G1 live evidence | W0.7, G1 |
| 5 | **U5 — Product experience layer + mobile.** React Native + Expo is the App Framework reference target; responsive web remains the mandatory channel comparator. Waves 1–2 established `product mobile-test --json`, the CRM product-owned `mobile/` scaffold, deterministic Expo/RN dependency pins, and retained typecheck/unit-test/Expo Doctor/runtime-audit/device/store diagnostic contracts. M0-05 now contains that legacy verifier: it always emits `candidate_ready:false`, `release_ready:false`, `release_authority:"none"`, retains the old conjunction only as `legacy_evidence_satisfied`, invalidates stale staged inputs, and is rejected as an authority by Wave 2 and strict release evidence. Wave 3 moved `product generate --target mobile-rn` off scaffold-only posture: it emits/checks the generated mobile contract, PDS native token bridge, reusable generated entity screen, generated policy-aware GraphQL data client, ownership metadata, scaffold manifest, and all primary entity Expo Router shells with `generator_status:"all-entity-route-shells-emitted"` and real write/check actions. Remaining U5 work is the real API/auth seam, both-platform reference journey, source/build/update provenance, PDS Native, Fabric D2 authorities, update recovery, comprehensive review, and a new source-bound mobile candidate checker. | 3 | `docs/frontend/mobile-react-native.md`, `scripts/appfw product mobile-plan --ui-artifact <mockup.html> --json`, `scripts/appfw product generate --target mobile-rn --check --json`, `scripts/appfw product mobile-test --json` for diagnostics, `python3 scripts/mobile-readiness-containment.test.py`, then the future source-bound candidate checker and representative iOS/Android evidence | W0.5, U6, G2 |
| 6 | **U3 — PoC intake / legacy modernization journey** (pairs with release-blocking **P5** golden-path). Its web / no-CRM-residue slice is **executable now** via the existing passing intake-proof gate — correcting an earlier claim that it was "not yet executable." | 1 | `golden-downstream --json`, intake-proof (existing, passing) | W0.4, W0.5 |
| 7 | **U4 — Evidence-gated connector graduation.** Make per-connector "unsupported until authenticated evidence" the default; track ungraduated capabilities and add graduation evidence to release artifacts. First Wave 1 report-only slice writes `target/appfw/provider-graduation.json` and validates retained reports without promoting capabilities. Executes North Star Part 7 goal 3 (shrink ungraduated gates). | 1 | Per-connector unsupported-capability tracking + a graduation evidence category | W0.2, W0.6 |
| 8 | **D6 — Developer-loop latency reduction** (was Codex #1 — demoted to continuous optimization: DORA shows *control systems*, not raw speed, carry the strategy). First Wave 1 slice writes `target/appfw/docs-check-timing.json`; first Wave 2 slice writes `target/appfw/docs-check-changed-surface.json` so changed-only escalation has retained file/rule evidence; the next Wave 2 slice adds retained phase timing for changed-surface selection and Cargo prebuilds so the wall-clock gap between full docs-check and per-example elapsed time is attributable; the PR default slice wires Bitbucket pull requests to `docs-check --changed-only --enforce-budget` with selected-mode CI budgets; current slices add `target/appfw/docs-check-subchecks.json`, split the slow core product command group into bootstrap, validation/topology, generated-drift, and explain/profile subchecks, split the old 100-item agent-governance bucket into core/write/mobile/runtime/release focused subchecks while retaining the umbrella alias, add focused `docs-check --subcheck <name>` execution so agents can rerun one retained group without weakening default/CI/release coverage, prebuild `provider_certification_export` for the agent-governance groups, narrow focused prebuilds to only the binaries each group needs (`app_gen` only for generated-drift), retain `target/appfw/generate-check-timing.json` for split-root phase attribution, narrow the generated-drift equivalence workspace to generator-relevant product/framework surfaces instead of whole-root copies, and record `slowest_subcheck` with item-count/detail, `focused_rerun_command`, and same-worktree parallelization guidance in retained timing/subcheck artifacts. U7/D6 now converges `async-graphql` to one locked major/version across framework and product test crates, centralizes common workspace dependency versions, removes direct `reqwest 0.11` clients from the framework workspace while retaining `reqwest_versions` in composition evidence, and narrows `framework test --fast` to library-targeted `appfw-cli`/app_gen checks instead of empty binary harnesses. Remaining work is caching or optimizing the next slow single-command phase based on retained docs-check/generate-check evidence, broader Cargo convergence, and shell→Rust. Detailed fixes below. | 2 | Changed-surface artifact + phase-timing artifact + subcheck artifact + focused subcheck execution + duplicate-version budget — **this is the evidence that will score the "Developer loop efficiency" dimension after multiple enforced PR runs** | W0.6, W0.4 |
| 9 | **U6 — PDS consumption-from-source + fork detection.** Converge the `admin_ui`/CRM token imports and `frontend.rs` local-token-path to from-source **before** enabling the rejection gate, or main goes red. Wave 1 now has `framework fork-check --json` / `target/appfw/fork-check.json` with generated `tokenCssPath` and admin/CRM imports on the canonical PDS token alias, local token copies removed, and rejection enforced for known token-copy forks. | 1 | Fork-detection gate + a compliant downstream scaffold to run it against | W0.5, W0.7, U3 |
| 10 | **U7 — Runtime/build modularity.** Feature-gate runtime/provider surfaces, converge duplicate dependencies, manifest-driven composition. First Wave 1 report-only slice adds `framework composition-check --json` and `target/appfw/composition-check.json`, showing enabled ingress, runtime-vs-topology provider posture, frontend embedding, default-feature alignment, and transitional items before the Cargo/source-cfg migration. Wave 1 now has Kafka default alignment, explicit provider feature gates for the CRM provider fixture set, explicit product/admin UI packaging posture in `.appfw/manifest.yaml`, provider-certification crates removed from the default product backend feature set, generated route registration narrowed to schema-hosted runtime providers, and `async-graphql` collapsed to one locked version. Wave 2 now has `feature-check --plan --json` (`target/appfw/feature-check-plan.json`), retained `target/appfw/feature-check.json`, composition-check consumption of the non-plan compile artifact, and actual retained compile evidence for runtime ingress plus each declared product provider feature; the second slice also fixed the generated Neo4j graph methods so no-provider and per-provider backend builds compile cleanly. `composition-check --enforce` now fails unless `composition-check.gate.ready_to_enforce:true` is backed by retained non-plan feature evidence, writes `target/appfw/composition-check-enforced.json`, and `release-check --json` runs that enforced gate after `feature-check` while retaining `composition-check.json` in the active release artifact directory. Co-own remaining Cargo migration with D6 (both edit every `Cargo.toml`). | 2 | `feature-check --json` retained artifact + supply-chain gate + a duplicate-version budget; release gate via `framework composition-check --enforce --json` | W0.4 |
| 11 | **G4 — PHI-pipeline governance + provenance consolidation.** Consolidate the existing release-provenance / artifact-signing categories (`scripts/ci/security-assurance-decision.sh`) into a PHI-governance bundle; the SLSA/cosign external-CI attestation (OIDC/Fulcio/Rekor) is the NEW, gated, partly-external piece. First Wave 1 report-only slice adds `framework governance-check --json` and `target/appfw/governance-check.json`, inventorying PHI pipeline, provenance/signing, release identity, PDS baseline, and security-assurance decision posture while strict rejection remains in the existing release gates. Wave 2 adds `framework governance-check --enforce --json` and `target/appfw/governance-check-enforced.json`, failing closed while release identity/PDS baseline/provenance/signing evidence remains transitional. Current Wave 2 hardening explicitly separates local provenance/signing artifacts from production attestations or accepted-risk decisions, so local evidence cannot accidentally satisfy the G4 release gate. Wave 4 tightens the PHI pipeline package itself: `target/appfw/phi-pipeline-governance-evidence.json` or `APPFW_PHI_PIPELINE_EVIDENCE_FILE` must validate `appfw.phi-pipeline-governance.v1` across classification propagation, de-identification, lower-environment movement, RAG curation, retention, deletion/tombstones, redaction, and audit lineage before S9 can claim release readiness. | 4 | Consolidated governance category in the **existing** frozen evidence registry (edit it, do not invent one); report baseline via `framework governance-check --json`; readiness gate via `framework governance-check --enforce --json` | W0.7 |

Release-blocking **P5** (golden downstream) and **P6** (packaging/versioning/
compatibility) stay in the Highest Priority lane above; they are not re-ranked
here. First P6 slice now adds `product compat-verify --json` and
`.appfw/target/appfw/compat-verify.json`, reporting product `appfw.lock`,
local-path versus ProGet dependency posture, package-manifest evidence, and
frontend package consumption before the hard compatibility gate is enabled.

### How the waves materialized (reconciliation note, 2026-07-02)

Execution deliberately refined the original wave semantics, and the refinement
is now the standard rollout shape: instead of "Wave 1 = three parallel lanes,
Wave 2 = the dependent chains," what shipped was **Wave 1 = a report-only first
slice of *every* lane** (posture artifacts, non-enforcing) and **Wave 2 = the
enforcement/hardening slice** (fail-closed gates, anti-spoof schema validation,
the `wave2-status` rollup). This is a *better* expression of the evidence-gate
discipline — report, then enforce, then collect live evidence — and the ordered
chains below were still honored *within* that progression (G1 posture before U2
write-limits before G2 live-readiness enforcement). The lane map and chains
below remain the parallel-safety contract for any remaining and future wave
work; read "Wave 1/Wave 2" in them as lane/ordering structure, not as the
historical increment boundaries.

### Wave 1 — parallel lanes (disjoint surfaces, after Wave 0)

Run concurrently; each lane owns distinct surfaces so agents don't collide.

- **Golden-path lane** (U3 + release-blocking P5) — most executable-now; owns `app_gen/src/bin/appfw_introspect.rs`, `app_gen/src/frontend.rs` intake path, `app_gen/_templates/product_intake/`, the crm-sample profile. Co-own within the lane (shared files).
- **Developer-loop lane** (D6 + U7) — owns docs-check internals, the PR/main pipeline split, and the Cargo workspace. **D6 owns the single `Cargo.toml`/`Cargo.lock` convergence commit; U7 rebases onto it** (both otherwise edit every member `Cargo.toml` — a repo-wide serialization point). Wave 1 has collapsed `async-graphql` to one version, removed direct `reqwest 0.11` clients from the framework and CRM product fixture locks, and now has retained `feature-check --json` proof for runtime plus CRM backend core/http/mcp/kafka/provider feature combinations without broad default compilation.
- **Docs-content lane** (G3 write-up + P6 compat-matrix reconcile) — owns distinct doc files; coordinate `check-doc-examples.sh` assertion edits with the Developer-loop lane (D6 owns the structural split). G3's *write-up* is parallelizable here, but G3 still **precedes U2** in the security chain (Wave 2) — lane-parallelism is *across* lanes; ordering is *within* the security track.
- **Report-only slices** of U4 (unsupported-count summaries) can proceed without touching the capability enum frozen in W0.2.

### Wave 2 — dependent chains (do not parallelize)

- **W0.1 → G3 → U2:** reconcile direction → document the agentic threat → encode the least-privilege harness.
- **W0.2 enum freeze → G1 → U2 (write limits) → G2 (confirm/undo/audit made real):** governed-write substrate first, then the UI primitives that ride it. G2's confirm/undo/audit is a dead shell — implying safety the backend can't honor — until G1 lands.
- **P1 → P2 / P3 / P4:** the live-evidence lane; P2/P3/P4 all consume the live backends and parity artifact P1 stands up (external-ops, outside code authoring).
- **D6 split → D6 default:** split docs-check and build the exhaustive changed-surface trigger list *before* making changed-only the CI default.
- **U6 converge → U6 gate:** land the token convergence before turning on fork rejection.
- **U3/P5 scaffold → U6 (converge, then gate) → U5:** U5's mobile client needs U6's from-source token baseline and a real scaffold to target.
- **W0.5 contract freeze → U5 mobile → (needs G2) → U5 conversational slice.**

**Wave 2 local evidence status (2026-07-02):** Local framework/product gates now
prove the executable slices: G1 fail-closed governed-write posture plus a
provider-test governed-write planning/preflight route, G3 threat
mapping, U2 least-privilege harness write limits, G2 governed-action readiness
reporting plus parseable fail-closed enforcement output, U5 static mobile
scaffold plus `product generate --target mobile-rn --check`, deterministic
mobile lockfile evidence, and passing local npm typecheck/test/Expo-doctor
evidence retention plus retained runtime-audit/device-tooling preflight posture,
device-evidence schema hardening, and store-track evidence schema hardening,
U6 fork rejection,
U7 enforced composition evidence, D6 changed-only/subcheck evidence, and G4
enforced governance posture that fails closed on transitional release-authority
evidence. P1-P4 now has `framework local-live-preflight --plan --json` to
publish required live inputs and external release-authority gates without
starting local containers. `framework wave2-status --json` now writes
`target/appfw/wave2-readiness.json` as the aggregate handoff/readiness rollup
for those lane artifacts, the reserved G1 live-evidence schema posture, U5
runtime-audit/device/store-track posture, and the remaining external gates.
It also carries `external_evidence_plan` entries for G1, U2, G2, U5, G4, and
P1-P4 so release authority and platform owners have exact command/artifact/env
hooks for the live evidence still required. The summary intentionally separates
`summary.local_lane_count` from `summary.local_preflight_lane_count`: P1-P4's
local preflight is useful branch evidence, not managed release authority.
Do not mark the
strategic lane release/live-ready
from those local gates alone: G1 still needs real provider-test governed-write
evidence for a named external API mutation, G2 `--enforce-governed-action`
correctly fails without that live evidence, U5 remains `release_ready:false`
until simulator/device/store-track evidence exists and the Expo dependency audit
posture is dispositioned, and G4/P1-P4 still need managed release
authority/live-provider evidence. For P1-P4, focused/local evidence is CI
readiness only; `wave2-status` requires the retained managed
`bitbucket-release-gate.json` to be `ok:true`, `release_ready:true`, not
`focused_evidence:true`, and backed by strict managed release requirements
before the lane can satisfy production release authority.

### Wave 3 — execution & first product (planned; starts at Wave 2 merge)

Waves 0–2 built the contracts, report slices, and fail-closed gates. Wave 3
changes the nature of the work: **build the execution substance behind the
gates, exercise a downstream product pilot, and hand the external gates to
named human owners.** The synthetic pilot is foundation evidence only; AF-OG02
requires an external product-owned consumer and real framework upgrade.
Verified ground truth behind this
plan (reconciled
2026-07-04): W3-A now has a production SaaS HTTP transport substrate
(`RuntimeHttpSaasRequestExecutor` / `ReqwestRuntimeSaasRequestExecutor`) with
origin-pinned requests, response caps, bearer-token injection, and authorization
header rejection; it also has local-proven client-credentials token endpoint
execution wiring (`RuntimeHttpOAuthClientCredentialsTokenExecutor`) over the same
runtime transport abstraction. W3-B now has provider-neutral actor,
delegated-token, auth-code, idempotency, and SaaS write-audit primitives. The
W3-A sync-worker contract now has a local fixture runner that proves checkpoint
watermark advancement, provenance, echo-loop suppression, duplicate stable-key
tracking, projection-write intentions, and poison-record dead-letter evidence
without starting a live provider. It also has provider-neutral checkpoint and
projection-store traits with in-memory implementations that locally prove resume
and stale replay suppression. W3-A also has a provider-neutral
client-credentials SaaS request executor that binds token execution/cache to the
runtime SaaS HTTP executor, injects runtime-owned bearer tokens, reuses fresh
tokens, refreshes expiring tokens within skew, and rejects caller-supplied
`Authorization` headers before token acquisition. The remaining gaps are
provider-specific live bindings and certification, durable encrypted token
custody and revocation evidence, named-mutation request conversion, live cert
evidence, database-backed
checkpoint adapters, production projection-store adapters, scheduler readiness,
and sync freshness evidence.
The mobile generator now emits/checks the React Native contract bridge, PDS
native token bridge, ownership metadata, and scaffold manifest; all-entity
screen/route source generation is still future W3-D work. The intake golden
path passes end-to-end today.

**Operating rules (from Waves 0–2 lessons, now North-Star direction):**
lane-sized PRs only — no multi-lane branches; agent lanes end at
`local_proven` — externally-gated evidence belongs exclusively to the
provisioning track below; the rollout shape per capability is
*freeze → report slice → enforcement slice → live evidence → graduate*.

**Wave 3 local progress (2026-07-02):** PM-1 wires the PR governance gate path
on its own branch. CH6 now has local deterministic chat-eval execution via
`scripts/appfw product chat-eval --json`, docs-check happy-path and fail-closed
red-team assertions, and optional release-evidence validation of retained
`target/appfw/chat-eval.json`; `framework wave2-status --json` now consumes the
staged `target/appfw/wave4/ch6-chat-eval.json` artifact as local CH6 posture and
keeps judge/live evidence separate. It still does **not** claim chat runtime,
Conversation components, AI-search provider, judge certification, or live PDS
search readiness.

| Lane | Goal | First items | Owner | Done-state (local) |
| --- | --- | --- | --- | --- |
| **W3-A SaaS execution substrate** (Archetype-1 completion) | HTTP executor substrate, client-credentials token endpoint execution wiring, local fixture sync-worker runner, provider-neutral checkpoint/projection-store contracts, and provider-neutral client-credentials SaaS request auth binding are implemented; next slices add durable database-backed adapters and provider-specific live bindings behind the existing fail-closed activation gate | Existing executor/token-exchange/auth-binding tests stay green; local fixture worker runner proves checkpoint/provenance/echo-loop/dead-letter mechanics; in-memory stores prove resume and stale replay suppression; next: durable adapters and provider-specific live certification | agent | Executor + token endpoint/auth-binding tests green; local fixture worker/storage execution green; activation stays gated until durable persistence and live cert |
| **W3-B G1 delegated-auth substrate** (provider-agnostic ONLY) | Provider-neutral actor/delegated-token/auth-code/idempotency/audit primitives are implemented; next slices add durable encrypted token custody, refresh/revocation evidence, named-mutation binding, and live-cert runner hardening | Token-store and idempotency/audit modules unit-proven; runner skeleton whose **mock mode must never write `governed-write-evidence.json`** (mock output goes to a fixture name validators reject) | agent | Modules unit-proven; release evidence artifact remains unproducible until Unit A |
| **W3-C Nexus synthetic pilot** (foundation evidence) | `pds-nexus` bootstrapped via product-intake; read-only De Novo tracker (176-task/20-team shape) on **synthetic seed data**; G2 primitives in preview-only mode; U2 agent-profile exercised | Bootstrap via intake; seed-data tracker screens; harness-check green | agent (+ PM PoC artifacts = human input; synthetic fixture otherwise) | Intake-proof + harness-check green on the pilot. **Honest label: NOT connected to ServiceNow and not real-consumer adoption.** Projection schemas are PROVISIONAL pending the Unit-A export; a post-Unit-A schema-reconciliation step is required before W3-A targets them. UI constrained to the existing PDS catalog - new component needs are separate catalog PRs |
| **W3-D Mobile generator emission** | Contract/token bridge, generated entity screen, generated mobile data-client request binding, and all-primary-entity Expo Router shell emission are implemented; next slices add product-owned native workflow runtime wiring and iOS **simulator** smoke at `development` level (agent-executable - verified); check Expo SDK 58 for the audit findings, else human disposition | Emission slice 3 complete (contract/tokens/generated entity screen/all route shells/generated data client/drift); next: endpoint/auth workflow wiring + simulator smoke evidence | agent (store-track = Unit E) | `generator_status:"all-entity-route-shells-emitted"` with real write/check actions; mobile-test device evidence at development level still pending |
| **W3-E Packaging consumer-side** | Add SaaS crates to the publish plan; make bootstrap honor `product_framework_dependency_source` (registry deps when packaged); rebuild package at merged HEAD. First W3-E slice makes `appfw-saas-core` plus Salesforce, Workday, Anaplan, Oracle Financials, ServiceNow, and iCIMS crates publishable to the internal registry; emits the crates publish-plan during `framework package --plan`; and adds docs-check assertions that SaaS crates remain in the ProGet publish order. | Publish-plan + registry-mode bootstrap PRs before the human ProGet publish | agent (publish handoff = human) | Publish bundle ready; registry-mode bootstrap unit-tested; packaged-bootstrap smoke immediately after upload |

The historical W3-C synthetic pilot remains useful fixture and intake evidence,
but it does not satisfy the current real-downstream-product or real-framework-
upgrade outcome and cannot receive adoption credit.

**DP4 freshness/lineage slice:** `scripts/appfw framework saas-lineage --json`
now provides the first unified SaaS freshness/lineage posture report and retains
`target/appfw/saas-freshness-lineage.json`. `--enforce` fails closed until
model-owned sync descriptors plus runtime evidence with schema
`appfw.saas.freshness-lineage.v1` prove freshness, lineage, provenance,
echo-loop suppression, and redaction.

**ServiceNow evidence-gate rule (binding on W3-A/B/C):** named-mutation
*registration* (gated, non-executable — e.g. `servicenow.create_incident`) is
allowed pre-instance; **request/payload construction is not** — it waits for
the Unit-A authenticated OpenAPI/dictionary export, and registry-conversion PRs
must cite that export artifact as evidence.

**Shared-file merge order (these lanes are NOT disjoint — serialize):** all
`scripts/appfw` touches in order *post-merge CI wiring → W3-E publish plan →
W3-B provider-test → W3-D*; W3-A's `appfw_saas_core` executor lands **before**
W3-B's saas_core modules; W3-D's `appfw_introspect.rs` ownership routing and
W3-E's bootstrap changes serialize; rebase small and often.

**Provisioning track (human-owned; request ALL in week 1 — ranks are leverage,
not order):**

| Unit | Action | Unblocks |
| --- | --- | --- |
| **A** (highest leverage) | ServiceNow dev/sub-prod instance: OAuth client (client-credentials + auth-code), integration principal, delegated test tenant/user, ACL-scoped test table + mutation, secret-store token ref, audit sink; capture **authenticated OpenAPI + dictionary export** | 4 of 14 gates (G1×2, U2, G2) **plus** ServiceNow read graduation for Nexus live read — the critical path |
| **B** (cheapest) | `LOCALSTACK_AUTH_TOKEN` as secured Bitbucket variable + trigger the release-check pipeline lane | P1, P2, P4 outright + necessary for P3 |
| **C** | Release-authority package: approved LICENSE, release-identity decision, PDS baseline decision, time-boxed provenance/signing risk-accept (or Unit D) | G4×3 + completes P3; enables the `v0.1.x-rc.1` tag |
| **D** | Bitbucket OIDC + cosign/SLSA keyless wiring | G4 provenance permanently (replaces recurring risk-accept) |
| **E** | Apple Developer / Play Console + Expo EAS accounts | U5 store-track |
| — | ProGet publish handoff (crates + tarballs + npm feed) | First packaged downstream product (strategic; not one of the 14) |

**Immediate post-merge checklist:** (1) verify the first green focused main
gate on the merged tree; `scripts/ci/wave3-pr-gates.sh` is now wired into the
PR fast framework check after `docs-check --changed-only --enforce-budget`, so
ordinary PRs hard-fail U7 `composition-check --enforce`, U6 `fork-check`, and
the docs budget while requiring G4 `governance-check --enforce` to produce the
expected fail-closed release-gated JSON; (2) `wave2-status` remains
**report-only** in that PR gate and `release-check` still retains
`wave2-readiness.json` from `framework wave2-status --json` as **report-only**
evidence until all its input artifacts are CI-produced in the same run; (3) fix
the release-lite guard's stale hardcoded approval defaults **only
after** a human sets fresh secured variables / approval on the fixing PR's
commit (the PR that removes the defaults otherwise fails its own guard), and
not while Wave-2/lane PRs are open on the old evidence. The fixing PR removes
checked-in fallback approvals and requires secured CI variables to carry current
evidence; (4) make `appfw.lock`
re-baseline a repeatable end-of-merge-train step, not a one-shot; (5)
coordinate any roadmap/doc edits with in-flight lane worktrees before
committing.

**Scorecard triggers:** D6 "Developer loop efficiency" is scored only after
≥5 PR runs under the enforced budget lane, from retained
`docs-check-timing.json`/`docs-check-subchecks.json`; a full release re-score
follows the first green focused main gate on the merged tree. **Tag policy:**
`v0.1.x-rc.1` is permitted once LICENSE + release-identity land (rc tags do not
require production certification); production certification remains blocked by
the remaining external gates. **Doc reconciliations completed 2026-07-02:**
`versioning-and-compatibility.md` now names the current `0.1.1` internal-pilot
posture, and `application-lifecycle.md` marks registry-mode bootstrap as pending
until W3-E's registry-mode bootstrap lands.

**Wave 4 SEC-AIBOM first slice:** AI-code provenance/AIBOM/PR-trailer posture
now has a local framework command contract via
`scripts/appfw framework aibom-check --json`, retaining
`target/appfw/aibom-check.json`. `--enforce` writes
`target/appfw/aibom-check-enforced.json` and fails closed until managed CI
produces a release-grade `appfw.aibom.release-attestation.v1` artifact tied to
the exact HEAD and required PR trailers are verified. This is not yet a release
authority substitute; it makes the S6/S8 governance gap executable and
falsifiable.

**Wave 4 SEC-PROMPTAUDIT first slice:** chat prompt-audit/SIEM/retention and
kill-switch posture now has a local framework command contract via
`scripts/appfw framework prompt-audit-check --json`, retaining
`target/appfw/prompt-audit-check.json`. `--enforce` writes
`target/appfw/prompt-audit-check-enforced.json` and fails closed until managed
release evidence uses schema `appfw.prompt-audit.release-evidence.v1` and proves
prompt audit, SIEM export, sink configuration, 90-day retention, kill-switch
test, redaction, correlation IDs, access-review sampling, and incident-response
runbook evidence. This is intentionally the first gate in the AI/chat sequence:
real prompts stay non-release-ready until this evidence exists.

### Developer Loop Efficiency Work (detail for item D6)

The normal local/PR path should be fast enough that agents and developers keep
using it. Full release evidence remains mandatory, but it should not be the
default for every small commit. **This is the single detail home for dev-loop
work — the scorecard row links here rather than restating it.**

| Slow Path | Problem | Fix |
| --- | --- | --- |
| Full `docs-check --json` in routine workflows | Release-grade; prebuilds binaries + executes many examples. On broad branches, changed-only may select full. The PM-1 branch exposed a concrete bad case: a CI wrapper-only edit was classified as docs-check behavior and paid the full prebuild/generator path. | `docs-check --fast` for local docs edits, `docs-check --changed-only --json` for PRs, full docs-check only for release, command-contract, or docs-check-behavior changes. CI wrapper scripts that are verified by shell syntax and their own retained JSON contract, such as `scripts/ci/wave3-pr-gates.sh`, now stay on the fast changed-only path instead of forcing full generated-drift coverage. |
| Missing retained selector/timing baseline | Changed-only escalation and per-example elapsed times existed only in transient docs-check JSON response, making scorecard movement hard to prove after the terminal output was gone. | `docs-check` now writes `target/appfw/docs-check-changed-surface.json` with changed files, trigger rules, and selected mode, plus `target/appfw/docs-check-timing.json` with selected-mode budget, `budget_ok`, phase timings for changed-surface selection and Cargo prebuilds, aggregate example time, unattributed wall time, subcheck groups, and opt-in `--enforce-budget` failure. The Bitbucket PR lane now turns enforcement on with separate fast/full CI budgets. |
| Monolithic docs-check | Static IA/link checks, skills routing, CLI contracts, executable examples, and maintainability assertions are bundled too tightly. | `docs-check` now writes `target/appfw/docs-check-subchecks.json` with named static groups plus narrower full-mode command groups. The slow core product bucket has been split into `core-bootstrap-command-examples`, `core-generation-command-examples` (validation/topology), `core-generated-drift-command-examples` (`generate --check`), and `core-explain-profile-command-examples`, followed by `intake-skills-command-examples`, the agent-governance focused groups (`agent-governance-core-command-examples`, `agent-governance-write-command-examples`, `agent-governance-mobile-command-examples`, `agent-governance-runtime-command-examples`, `agent-governance-release-command-examples`, with `agent-governance-command-examples` retained as an umbrella alias), `packaging-dependency-command-examples`, and `compiled-cli-command-examples`. Focused `docs-check --subcheck <name>` now reruns one retained group for local iteration while preserving full/changed-only/release coverage, and the focused agent-governance groups prebuild only `appfw_introspect` plus `provider_certification_export` instead of every appfw-codegen/appfw-cli binary. The timing and subcheck artifacts now record `slowest_subcheck` with item-count/detail, `focused_rerun_command`, and a same-worktree parallelization warning so agents can tell whether to split a bucket or optimize/cache a single slow command from evidence instead of guessing from one wall-clock number. |
| Slowest fast-mode design-system fixture | The fast lane repeatedly invoked `check-pds-components.mjs`, and its schema-thin governed-write evidence fixture temporarily mutated canonical `target/appfw/governed-write-evidence.json` before running another full component check to prove restoration. | `check-pds-components.mjs` now accepts explicit governed-write/harness evidence override paths, and docs-check writes schema-thin fixture evidence under its report directory. The fast design-system subcheck preserves the same missing/invalid evidence assertions without mutating canonical retained evidence or paying the restoration rerun. |
| Slow `test --smoke` docs-check example | The agent-governance core docs-check group ran `framework test --smoke --json`, which re-executed validation, boundary checks, shell syntax, and JavaScript checker syntax even though neighboring docs-check groups already prove those contracts. | `framework test --smoke --plan --json` now reports the smoke lane contract without execution, and docs-check uses that read-only plan example. Real proof remains in `framework test --smoke` / `framework test --fast`; docs-check no longer pays the redundant smoke execution inside the command-contract group. |
| Opaque generated-drift bottleneck | The retained docs-check evidence now shows `core-generated-drift-command-examples` is a single slow command, but the next cut needs phase attribution inside `generate --check`. | `generate --check --json` now writes `target/appfw/generate-check-timing.json`, retaining split-root equivalence phases for root copy, workspace preparation, `app_gen`, protected-handler checks, framework-root mutation checks, generated output comparisons, and slowest phase. The focused generated-drift subcheck now prebuilds only `app_gen`, and manifest snapshots hash files in one pass so the preparation phase is no longer the dominant local cost. Use this artifact before deciding whether to cache roots, optimize app_gen, narrow comparisons, or move more logic into typed Rust. |
| Opaque PR framework gate | The PR fast framework step can sit inside the PR gate wrapper after docs-check with no visible sub-gate movement, forcing humans to infer whether the work is `feature-check`, product SPA install/build, composition, fork, governance, or wave2 status. The current wrapper path is `scripts/ci/wave3-pr-gates.sh` for compatibility, but the gate is a durable PR framework check rather than wave-specific machinery. The first enforced CI run also exposed Debian's default Node 18 as too old for the Tailwind/Vite product SPA toolchain, and the first sensitive PR run paid the long fast-framework/supply-chain/secret-scan path before failing release-lite approval evidence. | The PR gate wrapper now writes `target/appfw/wave3-pr-gates/progress.jsonl`, prints progress JSONL to CI, retains per-gate stdout/stderr logs, and applies timeout controls through `APPFW_WAVE3_GATE_TIMEOUT_SECS`, `APPFW_WAVE3_NPM_CI_TIMEOUT_SECS`, and `APPFW_WAVE3_NPM_BUILD_TIMEOUT_SECS`. The Bitbucket fast framework step installs verified Node 22 with `scripts/ci/install-node.sh` before running product SPA install/build. The PR lane now runs release-lite as an early fail-closed preflight and repeats it after retained PR evidence so missing approval variables fail fast without dropping the final guard. |
| Broad `scripts/appfw test` usage | Product, framework, provider, frontend, and release checks over-selected. | `test --smoke` now gives docs/routing/checker-script edits a no-Rust-compile lane; keep `test --fast`, targeted crate tests, `frontend-test`, `provider-test`, and release-check roles explicit in docs + CLI suggestions. |
| Heavy Cargo dependency graph | Duplicate versions and broad default features slow builds and widen the vulnerability surface. Wave 1 removed the `async-graphql` v6/v7 split, moved provider-certification crates out of the CRM default backend feature set, and narrowed generated provider registration to schema-hosted runtime providers. Wave 2 now retains runtime/ingress/provider feature compile evidence, centralizes common workspace dependency versions, and removes direct `reqwest 0.11` clients from framework/test/provider crates, but the expanded matrix is still too expensive for every small PR. | Continue converging duplicates, move more shared versions to `[workspace.dependencies]`, keep optional runtime/provider surfaces feature-gated, and prove provider-certification crates through retained feature-check/provider-test/release-evidence lanes instead of the default product backend. Keep duplicate-version, default-feature, and feature-matrix budgets falsifiable while splitting/caching the slow compile checks. |
| Large shell orchestration scripts | Big shell routers are hard to test and slow for agents to reason about. | Move planning/classification/reporting into typed Rust CLI modules; keep shell as friendly entrypoints. |
| Release-grade scans in inner loops | Full secret-history scans, strict supply-chain, live provider cert, and frontend E2E are valuable but expensive. | Keep them in release/main lanes; use narrow metadata/changed-file/smoke checks in local/PR lanes. |

### Wave 4 — live experience & governance graduation (selected slices landed; remaining capabilities graduate incrementally)

Wave 3 built substrate. Wave 4 is the first value-graduation wave: turn that
substrate into governed, user-facing product capability while preserving
contracts, generated ownership, evidence, and security as authoritative product
sources. Those sources are not delivered capability until a real consumer and
the required live, operational, and support evidence prove them.

The longer-term product outcome remains deliberately concrete: one real
PDS-owned employee product should answer "what work is stalled?", render a
structured PDS Health AI result, preserve auditability, keep every write
preview-only until G1 live evidence exists, and continue exactly across web and
native mobile while ServiceNow remains the workflow and fulfillment authority.
[PDS Nexus](../product/pds-nexus-product-development-brief.md) is the leading
candidate once product ownership and source evidence exist; Employee
Slate/Moveworks is the strongest comparator and fallback. This outcome is held
and is not the current PoC or producer lane. The active CRM composition is a
bounded reference-product experience proof and earns no Nexus, live-provider,
governed-action, or production credit.

Current Wave 4 work must advance either that bounded CRM proof, the short
security/evidence-truth gate, or a named gap in the later independently packaged
vertical. Generic contract, provider, component, protocol, dashboard, or
governance breadth remains out of sequence.

Read through the [Enterprise App Fabric Strategy](../strategy/enterprise-app-fabric.md),
Wave 4 is also the first fabric proving ground: it should demonstrate a
portable PDS journey capability inside a bounded PDS-owned product and, where
useful, an approved embedded or fallback vendor surface. It is not a duplicate
workflow engine, general search index, or unsupported production front door.
Any added fabric lane must name the
business owner, journey step, actionability tier, adopted object/workflow model,
freshness/provenance expectation, and evidence gate before implementation.

**Freeze first (single-owner, before fan-out):** reserve the shared seams before
any implementation lane starts: `RuntimeIngressKind::Chat` (closed enum),
`answer_envelope@1`, `viewRegistry`, `FrameworkProvider::AiSearch`, and
prompt/audit evidence categories. Reserve PDS Conversation or Ambient catalog
slots only when a named consumer qualifies for a PDS-owned renderer.
The freeze is done only when the owning spec and at least one machine-checked
assertion exist; prose-only agreement is not a contract.

**Hard governance line:** intelligence proposes / governed-write disposes. Every
AI read runs through an authenticated, bounded human, service, or agent
principal, with explicit on-behalf-of context when human authority is exercised.
Every model-supplied ref is resolved through the generated contract, and every write-shaped outcome remains
`IntentPreview`-only until G1 delegated-auth/governed-write live evidence exists.
New AI/mobile surfaces stay litmus-**FAIL** until grounding, preview-gating,
attribution, eval, audit, and retained evidence are all present.

The table below is a historical Wave 4 stream inventory, not the current
producer order. The Outcome Goal Register governs current sequencing.

| Stream | Business value | Safe sequence | Parallelism rule | Spec |
| --- | --- | --- | --- | --- |
| **1 · Structured application intelligence** | One governed PDS Health AI information pack can participate in a complete non-chat work loop without creating another assistant destination. | Preserve SEC-PROMPTAUDIT and CH1/CH2 safety contracts; compose `answer_envelope@1`, the permissioned archetype, existing operations, view registry, audit, and OTel around one signal, recommendation/prepared value, checkpoint, preview action, durable work item, web/native rendering, fallback, and outcome. Add contract fields only when the executable vertical proves them missing. A2A, AG-UI, A2UI, MCP Apps, and MCP remain future adapters. | No separate assistant, contract-family, ambient, provider, Kafka, mobile-shell, or renderer lane advances without the named vertical consumer, prerequisite authority path, and measured outcome. | [AI Chat And Search](../runtime/ai-chat-search.md), [Agentic UX](../frontend/agentic-ux.md) |
| **2 · SaaS refinements** | Fresh, explainable SaaS projections and connector graduation so Nexus can rely on ServiceNow-derived work data without guessing vendor or CDC contracts. | DP1 CDC ingestion mode stays report-only until enterprise CDC evidence lands; DP2 provenance/echo-loop and DP3 Atlas connection can advance independently; DP4 freshness/lineage waits for DP1+DP2; DP5 vendor-doc parity, DP6 SaaS testkit, DP7 principal envelope, U4 connector graduation, G4 PHI/provenance/signing, SEC-AIBOM, U6/U7 continue as focused lanes. | DP2/DP3/DP5/DP6/DP7/G4/SEC-AIBOM/U4/U6/U7 are parallelizable when they do not edit the same root manifests or `scripts/appfw`. DP1 execution and DP4 graduation are evidence-gated. | [SaaS Connectors](../runtime/saas-connectors.md), threat-litmus S3/S6/S8/S9 |
| **2a · Fabric/OMA proof** | Adopt vendor object/workflow models for the Team Member Journey without building a canonical enterprise model or wrapping every SaaS screen. | Team Member Journey feasibility card → Unit A ServiceNow dictionary/workflow export intake → `saas-export` model proposal path → projection-only entity marking with sync/freshness/provenance → consumer-contract drift check → capability-tier certification for the selected step. | Starts only after the Product Owner has owner/metric/tier/evidence card approval. It can run beside SaaS refinements if it avoids shared CLI/docs-check roots or coordinates those edits through the integration branch. | [Enterprise App Fabric Strategy](../strategy/enterprise-app-fabric.md), [Product Management Strategy](../strategy/app-framework-product-management-strategy.md) |
| **3 · Native-mobile participation** | Preserve a production-capable React Native + Expo path as a first-class channel of the PDS-owned employee product, with interruption-safe continuation, secure deep links, capture, timely action, and recovery. | Consume the W3-D mobile contract/token bridge; run type/unit/Expo/runtime-audit evidence; then prove device/simulator, security, distribution, support, and store-track posture for the launched scope. Compare the strongest entitled vendor mobile experience rather than copying it. | Can run beside late CH/DP work only after W3-D and U6 are stable and a named consumer qualifies. Avoid generator/product-contract edits while shared mobile seams are changing. | [Mobile Contract](../frontend/mobile-react-native.md), North Star Part 5 |
| **4 · Live release graduation** | Convert built-but-local capability into releasable enterprise proof. | W4-LIVE: P1-P4 live release evidence, G1 ServiceNow governed-write cert, managed release authority, and `v0.1.x-rc` to production-certification evidence. | Human-owned external evidence track. Agents may improve diagnostics and validators, but must not fabricate live evidence. | [Release Gate](release-gate-ci-cd.md), [Live Environment Work Items](live-environment-work-items.md) |

**Historical 180-day product-proof concept:** the prior concept sequenced vendor
entitlement evidence, a narrow Nexus comparison, PDS Health AI, a ServiceNow
read and preview, one governed action, and second-use proof. AF-OG02 through
AF-OG08 now carry the durable proof obligations and the PDS-owned target. This
record grants no permission to fabricate Nexus requirements or bypass SRA,
release, or human action authority.

**Superseded protected-window decision record:** PRs #406 and #407 completed the
package-lifecycle and permission/archetype foundation milestone and exposed
unused contracts plus missing vertical execution. That decision has now been
normalized into AF-OG01 through AF-OG07 in the Outcome Goal Register. The
register, not the former Gate 0/producer list, governs current work. Preserve
the durable capacity rule after AF-OG01: at least 70% of implementation
capacity terminates in the activated real product vertical or consumer, and no
more than 20% goes to bounded delivery-system simplification except mandatory
security or evidence-truth defects.

**Held next productization objective: Agent-Portable Legacy Modernization
Control Plane And Target Compiler.** App Framework should own the PDS-specific
first-class, typed, provenance-backed Modernization Intermediate
Representation, claim-specific evidence authority, adequacy/treatment
decisions, target compilation,
verification obligations, retirement economics, and portfolio learning while
Claude Code, Codex, deterministic analyzers, and qualified specialist engines
remain interchangeable workers. This is shaped for future pull, not an active
producer lane, and it does not displace AF-OG01 or an activated product
vertical.

The first milestone is a **chartered pilot and measured manual baseline**, not a
horizontal platform build. Use one representative but controlled .NET/IIS/
Angular/SQL Server application and one bounded capability or journey. Product
Owner must name the business outcome and accountable product/domain owner;
Architect must define the evidence, model, compiler, worker, and independent-
proof boundaries; data, security, operations, modernization, and release owners
must approve their authority, environment, handling, and proof obligations. XO
may release the pilot only after the human prioritizes it and those owners,
scope/exclusions, authorized sources, baseline economics, success measure,
proof standard, stop criteria, and current capacity are explicit.

Once released, use this outcome order:

1. charter the pilot, authorities, baseline economics, and proof standard;
2. establish an agent-portable evidence/task/result package for Claude Code,
   Codex, deterministic recipes, and specialist workers without making any
   worker's private state authoritative;
3. assemble only the narrow deterministic analyzer set the pilot needs across
   .NET/IIS, Angular, SQL Server, runtime, Jira, Confluence, and healthy CMDB;
4. reconcile a typed, provenance-backed **As-is Modernization IR** and use a
   human adequacy/conflict workbench to approve one specific version as
   sufficient for the bounded scope without claiming complete understanding;
5. compare retire, retain/contain, rehost, upgrade, refactor, replatform,
   SaaS/enterprise replacement, extract, strangle, App Framework rebuild, and
   split options, then approve a separately versioned, platform-neutral
   **Approved To-be Treatment Model**;
6. compile one complete selected vertical into real App Framework entities,
   operations, permissions, providers, events/projections, migrations, tests,
   product intent, extensions, and release obligations;
7. implement thin slices through portable workers and independently prove
   build, behavior, data/SQL, security, experience/accessibility, performance,
   resilience, and operations;
8. rehearse migration, reconciliation, rollback, and safe coexistence, then
   release progressively under existing human authority;
9. retire legacy callers, infrastructure, support paths, and cost or explicitly
   own and cost the remaining coexistence; and
10. use a second suitable application to prove reusable recipes and at least
    50 percent lower normalized discovery-to-approved-model engineering effort
    without lower quality.

Do not begin with every connector, a universal ontology, custom code search,
broad autonomous orchestration, a general coding agent, or an App Framework-
only funnel. Generated plans, code, and tests cannot serve as their own oracle.
The economic unit is fully loaded cost per accepted, production-capable
business capability with verified retirement, not generated volume.

The detailed contract is
[Legacy Application Modernization](../lifecycle/legacy-modernization.md). The
July 14, 2026 *Agentic Legacy Modernization Harness* decision research in the
PDS app-fabric research workspace substantiates this disposition but is not
an execution contract. XO must stop the lane on unauthorized evidence access,
unresolved critical contradictions, unclear regulated-data handling, missing
decision authority, any direct projection of raw evidence, the As-is
Modernization IR, or an agent plan into `.appfw/model`, silent lossy compiler
mappings, self-validating worker output, or work without the named pilot. No
stage may claim complete
understanding, behavioral equivalence, accepted risk, cutover, decommission,
release, SRA/CAB, or production readiness from agent output alone.

**Experience-system objective: active CRM composition proof; broader system
held. Calm Precision, Expressive Intelligence, Unmistakable Craft.**
The tracked PDS catalog already defines 95 components across ten families; the
next strategic need is coherent composition, hierarchy, accessible behavior,
measured task performance, and signature experience quality, not another
component-count wave. Use an `80/20` heuristic: familiar accessible patterns
carry routine work, while a few domain-specific visual objects and signature
moments receive disproportionate design and engineering investment. This
objective now has one bounded, product-neutral proof on the existing CRM
Activities surface. That proof may validate composition and design-system use;
it does not authorize Nexus product work, invent source behavior, or establish
vendor superiority. The broader journey, attention, workspace, and module
system remains held until Product Owner intake and source evidence exist.

For the active CRM proof and the later released reference compositions, use
this order:

1. use CRM Activities as the first bounded composition proving ground, then
   produce and task-test three production-shaped compositions and signature
   experience briefs: admin entity/operation workbench, approved Nexus/My Work
   queue/detail with structured PDS Health AI, and native-mobile task/approval;
2. prove signature experience quality with an accessible domain-specific visual
   object, two or three production-shaped signature moments per web reference,
   and at least one corresponding native-mobile moment using the
   Orient-Understand-Act-Resolve sequence;
3. select one experience grammar, then generate CSS, TypeScript, React Native,
   and design-tool mappings from a W3C DTCG-compatible token source with
   semantic, motion, data-visualization, theme, high-contrast, and density
   layers;
4. implement six owned floorplans and refactor components only as the real
   compositions consume them;
5. select React Aria or Radix through a representative accessibility bakeoff
   and use TanStack Table/Virtual only behind the owned grid API;
6. prove a bounded intelligent-composition contract with allowlisted PDS
   renderers, stable fallback, evidence/freshness/permission/action semantics,
   and no arbitrary agent-authored runtime UI;
7. make admin UI the dense reference product, preserve native Expo behavior,
   and make design guidance executable through recipes, design-code mappings,
   semantic motion roles, interaction tests, and visual evidence; and
8. add source-bound WCAG 2.2/manual assistive-tech, complete-state,
   responsive/visual, five- and 30-second first-impression, blinded-preference,
   signature recall, Core Web Vitals, bundle/data-scale, task, and intelligence
   evidence under an experienced product-design owner. Direct interaction gets
   immediate visual acknowledgment; INP is at or below 200 ms p75; motion is
   reviewed in context and reduced-motion behavior remains complete.

The broader Nexus experience-system release requires a named outcome and
persona, accountable product and source owners, approved source authority and
data/permission posture, journey evidence, comparison baseline, measurement
method, and product-design/mobile/operations ownership. Once those inputs and
WIP exist, add these outcomes to the same real journey:

9. version a journey graph or service blueprint across ServiceNow, Workday,
   PDS Health AI, PDS web, native mobile, and any approved embedded, comparator,
   or fallback vendor surface, with source
   ownership, role/channel experience briefs, Orient-Understand-Act-Resolve
   moments, exact handoffs/exceptions, correlation, and outcome evidence;
10. prove one durable attention contract and user preference model through an
    in-app center and one web or native push adapter, including grouping,
    deduplication, expiry, acknowledgement/snooze, policy override,
    accessibility, exact deep-link resumption, and PDS Observability trace;
11. prove a bounded personal work canvas with role/admin templates, policy
    locks, versioned user layout, allowlisted modules, backend eligibility,
    add/remove/reorder/resize/configure/undo/reset, responsive size classes,
    accessible order, schema migration/fallback, and appropriate cross-device
    persistence;
12. consume production-shaped journey-readiness, PDS Health AI evidence-brief,
    and intelligent-queue or next-best-action modules. Each module declares
    inform/recommend/prepare/act and preserves permission, evidence, freshness,
    consequence, checkpoint, recourse, deterministic fallback, latency,
    quality, inference cost, and edit/accept/reject/undo/outcome telemetry; and
13. provide administrator views for journey correlation, attention policy and
    delivery trace, module registry and eligibility, layout versions/reset, AI
    evaluation and cost, and failure recovery.

Graduation keeps two independent comparative bars. The provisional
**operational-superiority** hypothesis is at least 15 percent faster critical-
workflow completion or a ten-point task-success lift. The provisional
**signature-quality** hypothesis is at least 65 percent blinded preference and
a one-point lift on a seven-point crafted/fresh/intelligent/distinctive/
beautiful composite. Baseline research may calibrate these numbers but must not
remove either gate or permit regressions in accessibility, trust, reliability,
or recovery.

Do not rewrite React/Vite/Expo, treat Tailwind as the design system, restyle all
95 components in isolation, default intelligence to chat, or claim superiority
over Horizon/Employee Slate from screenshots. Also reject polished genericity,
expression everywhere, copied vendor aesthetics, demo-only choreography,
canned intelligence, and surprising interactions that impair repeated work. A
PDS surface earns advancement only through both measured workflow and signature
quality without loss of trust, accessibility, recovery, security, or
supportability. The detailed contract is
[PDS Health Enterprise Design System](../frontend/pds-health-design-system.md).
Do not turn the held extension into a generic portal, widget marketplace,
journey studio, or workflow engine. Source systems remain authoritative, and
personalization cannot alter permission, authority, source truth, journey
semantics, evidence, or required controls. The target is a coherent PDS-owned
enterprise experience; Employee Slate/Moveworks remains the strongest
comparator and fallback. Broad rollout still requires measured cross-vendor
continuity, exact web/native continuation, managed trust, outcome lift,
second-use economics, supportability, and signature quality.

**Held strategic goal: Cross-Channel Closed-Loop Application Intelligence.**
This strengthens Structured Application Intelligence and is attached to the
same future executable vertical. It does not create a separate AI program,
widen the active security/runtime readiness correction, or move ahead of the
runtime, provider, permission-preserving projection, governed-action, real-
consumer, and operational proofs that make the loop trustworthy.

The Product Owner must name the user, task, baseline, outcome, observation
window, vendor comparator, and stop threshold. Source and PDS Health AI owners
must own authority, information-pack quality/freshness, classification, and
support. Architect owns the context/action/result boundary and reuse decision;
security/data, product-design/mobile, and SRE/observability owners supply their
respective acceptance evidence. XO owns sequencing and WIP, not product intent
or risk acceptance.

When a named product task and those prerequisites exist, use this order:

1. integrate one live PDS Health AI information pack as a backend-to-backend,
   evidence-backed typed result; PDS Health AI remains retrieval authority, not
   transaction authority or an iframe target;
2. combine it with one deterministic or event signal, one recommendation or
   prepared value, one human/policy checkpoint, one governed preview action,
   and one recorded outcome;
3. use the existing principal, tenant, policy, entity, workflow, freshness,
   channel, device, and correlation semantics as one context spine;
4. compose `answer_envelope@1`, `permissioned_archetype@1`, existing operations,
   view registry, audit, and OTel before adding the minimum missing typed
   result/rendering fields proven by the vertical;
5. persist the result as an authorized durable work item, then render exact
   continuation through a deep web surface and native-mobile task using shared
   semantics but separate channel-native component trees;
6. use Kafka/projection only when the same object needs a timely signal, with
   entitlement, freshness, replay, lag, suppression, reconciliation,
   revocation, outage, and fallback evidence;
7. trace retrieval/rule/model, render, checkpoint, operation, fallback, cost,
   correction, and downstream outcome in PDS Observability, including a tested
   kill switch; and
8. let a second real product prove which context, renderer, evaluation,
   checkpoint, capability-resolver, and generation patterns become reusable.

Do not create a general assistant/search destination, model platform, universal
agent runtime, arbitrary generated UI, hidden memory, transcript-owned process,
or direct per-product model wiring. Durable run semantics are added only for a
real multistep use, and memory/personalization only after it is visible,
consented, scoped, correctable, expiring, resettable, and outcome-proven. Chat
may invoke or explore work but never owns authoritative state or action. Current
contracts and components do not prove a live PDS Health AI loop, production
Kafka path, delegated source action, production native-mobile continuation, or
end-to-end quality/cost/outcome trace.

**Program-dashboard oversight boundary:** use the dashboard as the primary
human communication projection and improve it through bounded navigability or
source-parity increments. Do not turn it into a tracked parallel Program State
system, planning authority, acceptance authority, or substitute for protected-
window product progress. Generate it from canonical strategy, the tracked
program contract, material Git/Bitbucket transitions, and commit-bound
evidence; show source SHA, generation time, source posture, and stale/unknown
status; and skip regeneration when inputs are unchanged. Ignored `target/appfw`
coordination files and local fixtures are disposable projections, not program
truth.

**Evidence-calibrated investment order:** consume the existing E1 advantages
rather than broadening them: model-driven generation, generated API tests,
migration discipline, provider-neutral database semantics, native MongoDB,
code-native delivery/evidence, deep UX, and deployment portability are held
foundations. Use bounded E2 work to converge the executable runtime spine and
close security, observability, accessibility, or scale-control gaps exposed by
the named vertical. Use E3 for the same ServiceNow object's complete
materialized-projection lane with entitlement provenance, policy/version,
field masks, permission freshness/revocation, source reconciliation, and
fail-closed stale behavior in addition to data freshness, coverage, and
fallback. Permission preservation is part of this proof, not later security
hardening. Reserve E4 work for the live ServiceNow execution/action path and
the managed Kafka, native-mobile, multi-tenant, PDS-scale, DR, and support proof
that the vertical actually requires. Treat E5 assistant/search parity as
avoid/consume.
The broad directional gap is 45-65 blended person-months over roughly 12-18
months with a stable 5-7 person nucleus plus security, SRE, data, mobile, and
product participation; it is not a committed schedule and must be narrowed by
funded journey evidence. The range assumes one PDS enterprise tenant boundary;
use 55-80 blended person-months only if PDS deliberately requires a general
SaaS-grade customer tenant control plane. Do not count all 1,100 locations as
tenants unless they require independent security and operational boundaries.

Passing this sequence does not by itself justify a goal-state or production
fabric claim. That label remains withheld until all ten
[Goal-state proof obligations](../strategy/app-framework-product-management-strategy.md#goal-state-proof-obligations)
are satisfied. Roadmap and dashboard reporting must distinguish current
evidence, gated capability, target state, proof obligation, and explicit
non-claim.

**Product-proof order:** obtain written vendor evidence and the version-pinned
ServiceNow work shape while converging the accepted `@1` contracts into one
executable runtime spine → execute and audit one permissioned ServiceNow source
read → project that same object into MongoDB with identity, freshness,
coverage, entitlement provenance, policy/version, dynamic field masks,
revocation/freshness SLO, tombstone/reassignment/group-change handling,
fail-closed stale behavior, source reconciliation, fallback, and negative
tests → consume the capability in one real downstream product and apply one
real framework upgrade → invoke one low-risk governed source action only after
live token isolation, revocation, narrow scopes, target-system authorization,
preview/HITL where required, idempotency, audit, and delegated-identity
evidence → assemble managed observability, load/cost, outage, recovery,
security-package, and support-owner proof around the vertical → add PDS Health
AI intelligence only as one closed observe-to-outcome loop for that same named
consumer; add vendor-surface, native-mobile, or Kafka reuse only where the loop
has a measurable channel or event need and preserves the same context, policy,
evidence, checkpoint, action, correlation, and fallback semantics.

This is the next strategic proof, not permission for a broad Kafka, connector,
or workflow program. The stream-fed archetype and dual-channel action must share
one adopted model, identity, policy, audit, evidence, and observability contract;
each stage retains its own honest maturity and non-claims.

**Authorization boundary:** a Moveworks launch rule governs plugin availability,
not record or action authority. The target/source must reauthorize every record
and action. Ambient, scheduled, and event agents use bounded service identities
and explicit on-behalf-of context when human authority is required. App
Framework must not build a general search-permissions index. Use source-native
or proven ReBAC services for high-cardinality ACLs, consume PDS Health AI
permission trimming, and preserve Moveworks trimming when that comparator or
fallback is used, while Rego remains the contextual application policy engine.

**Technical dependency order for already-approved framework lanes:** freeze →
SEC-PROMPTAUDIT+CH1 → CH2 → CH3/CH4/CH5 in the
smallest non-conflicting branches → CH7 → CH-AMBIENT → CH8 → CH6-slice2 →
parallel SaaS/governance lanes (DP2/DP3/DP5/DP6/DP7/G4/SEC-AIBOM/U4/U6/U7) →
DP1/DP4 graduation when enterprise CDC evidence exists → U5-MOBILE-EVIDENCE →
W4-LIVE. This technical order does not independently authorize a provider,
assistant, ambient, custom-shell, or generalized connector lane; each must be
pulled by the product-proof gates and a named consumer.

## Implemented Ingress Work And Post-Release Hardening

Post-release work should not block the current production-readiness gate unless
explicitly pulled into release scope.

- Runtime ingress module loading is implemented. HTTP, MCP, and Kafka are
  independently compiled, selected, and hosted through `RuntimeMode`,
  `RuntimeIngressKind`, `RuntimeIngressDescriptor`, and `RuntimeHostPlan`.
- Governed Kafka worker ingress is implemented at the runtime shell level:
  config validation, service-principal identity, explicit tenant derivation,
  operation binding, idempotency/retry/DLQ/readiness policy validation, and
  message-to-operation dispatch all live on the shared runtime path. Remaining
  hardening is broker-source binding, live Kafka certification, retained
  release evidence, and production platform secret integration. Broker trust
  must never bypass runtime policy.
- SEC-AIBOM posture is executable at the framework level through
  `framework aibom-check --json`; production AIBOM generation and PR-trailer
  enforcement remain managed-CI/release-authority work.
- CAB package harness for formal change-management approval. Start report-only,
  parallel to the SRA package flow, and retain change impact, implementation
  plan, validation evidence, rollback plan, communications, monitoring window,
  and approval-link fields without treating the generated package as CAB
  approval.
- MCP certification if MCP is moved into release scope. Until then,
  `APP_MCP_ENABLED=false` remains the release posture.
- Deeper product-template slimming after provider/runtime service boundaries
  are fully proven.
- Formal versioning, compatibility matrix, and migration lifecycle release
  notes.

## Stewardship Rules

- Keep this roadmap current, not historical.
- Do not paste PR-by-PR merge logs here. Use git history or archived notes.
- **Keep scores tied to evidence, not optimism — no score without a retained
  evidence artifact.** A dimension with no measurement is marked *not yet
  scored*, never given a number (see the "Developer loop efficiency" row).
- **Every item traces to a North-Star commitment** (see the North Star's
  Faithfulness Contract). An item with no basis is removed, or its rationale is
  promoted into the North Star first.
- Broad, sensitive, multi-domain, release/live, or score-changing agentic work
  must be classified with the
  [Agentic Development Control System](../architecture/concerns/agentic-development-control-system.md)
  and must name the human-review decision or retained impact evidence before
  merge.
- If a docs update changes a command contract, update `docs/reference/cli.md` and
  re-run `scripts/appfw framework docs-check --full --json`.
- If a release-evidence artifact can block promotion, validate it in
  `scripts/ci/release-evidence-check.sh`.
- Historical implementation reviews and completed planning notes belong under
  `docs/archive/` with clear status notes.

## Verification For Roadmap Changes

For docs-only roadmap changes:

```bash
git diff --check
scripts/appfw framework docs-check --changed-only --json
scripts/appfw handoff --json
```

Use `scripts/appfw framework docs-check --full --json` when the roadmap change
updates command contracts, docs-check behavior, or release-grade documentation
claims. Add broader checks when the reassessment depends on runtime, generated,
provider, frontend, or release-gate behavior:

```bash
scripts/appfw validate --json
scripts/appfw generate --check --json
scripts/appfw test --fast --json
```
