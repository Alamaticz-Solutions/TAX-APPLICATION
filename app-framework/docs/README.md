# App Framework Documentation

This documentation is organized for enterprise developers and coding agents who
need to change, verify, deploy, or extend App Framework safely.

Start with one reader path. Do not open the whole catalog unless you are doing
documentation or architecture stewardship.

## Choose Your Path

| Job | Start Here | Proof Command |
| --- | --- | --- |
| Build with App Framework | [Product Developer Docs](product/README.md), [Agent Skills Pack](../agent_skills/README.md) | `scripts/appfw product handoff --json` |
| Evolve App Framework | [Framework Steward Docs](framework/README.md), [Maintainability Command Center](architecture/concerns/maintainability.md) | `scripts/appfw framework handoff --json` |
| Route an agent task safely | [Agent Task Map](start/agent-task-map.md), [CLI Quickstart](start/cli-quickstart.md), [Generated Ownership](start/generated-ownership.md) | `scripts/appfw context --json`; namespace-specific handoff |
| Understand human and agent roles | [Agentic And Human Operating Model](start/agentic-human-operating-model.md), [Agent Role Cards](start/agent-role-cards.md), [Team Harness Replication](start/team-harness-replication.md) | `scripts/appfw context --json`; namespace-specific handoff |
| Monitor a remote PR or pipeline train | [Remote Promotion State Synchronization](start/remote-promotion-state-synchronization.md), [Durable Spec](specs/remote-promotion-state-synchronization.md), [Branch Integration Model](start/branch-integration-model.md), [Agent Role Cards](start/agent-role-cards.md) | Live Bitbucket transition evidence through post-merge destination status; namespace-specific handoff when tracked source changes |
| Track review conditions or technical debt | [Tech Debt Register](start/tech-debt-register.md), [Agentic And Human Operating Model](start/agentic-human-operating-model.md), [Agent Role Cards](start/agent-role-cards.md) | Review output with status/counts; namespace-specific handoff |
| Plan a meaningful change with decision provenance | [Spec-Driven Change Harness](start/spec-driven-change-harness.md), [Spec Template](specs/spec-template.md), [Agentic And Human Operating Model](start/agentic-human-operating-model.md) | Spec or intent note named in handoff; PR review checks spec-to-diff-to-evidence alignment |
| Use a concise agent procedure | [Agent Skills Pack](../agent_skills/README.md), [Agent Task Map](start/agent-task-map.md) | Skill proof commands plus namespace-specific handoff |
| Replicate the team developer/agent harness | [Team Harness Replication](start/team-harness-replication.md), [Agent Skills Pack](../agent_skills/README.md), [Branch Integration Model](start/branch-integration-model.md) | `scripts/appfw doctor`; `scripts/appfw context --json`; `scripts/appfw skills --json`; `scripts/appfw framework docs-check --changed-only --json`; namespace-specific handoff |
| Find, understand, and run major work | [Product Increment Portfolio](specs/product-increment-portfolio.json), [Product Increment Delivery Model](start/product-increment-delivery-model.md), [Program Work System](reference/program-work-system.md), [Branch Integration Model](start/branch-integration-model.md) | `node scripts/check-product-increment-portfolio.mjs --portfolio docs/specs/product-increment-portfolio.json`; `node scripts/check-product-increment-plan.mjs --plan <plan.json> --json` |
| Run Nexus workstreams on several workstations | [Nexus Add Provider Product Increment](specs/nexus-add-provider.product-increment.json), [Nexus PoC Multi-Workstation Delivery](specs/nexus-poc-multi-workstation-delivery.md), [Machine-Checked Topology](specs/nexus-poc-workstream-topology.json) | `node scripts/check-product-increment-plan.mjs --plan docs/specs/nexus-add-provider.product-increment.json --json`; `node scripts/check-nexus-workstreams.mjs --json`; `node scripts/audit-worktrees.mjs --json` |
| Build or upgrade a product app | [Product Developer Golden Path](lifecycle/product-golden-path.md), [Application Lifecycle](lifecycle/application-lifecycle.md) | `scripts/appfw product validate --json`; `scripts/appfw product generate --check --json`; `scripts/appfw product test --fast --json` |
| Convert a citizen-developer PoC | [PoC To Enterprise Product Intake](lifecycle/intake-and-discovery.md), [Product Developer Golden Path](lifecycle/product-golden-path.md), [Frontend Starter Contract](frontend/product-frontend.md) | `scripts/appfw product analyze --summary --json`; `scripts/appfw product propose-model --summary --json`; `scripts/appfw product model-status --json`; `scripts/appfw product validate --json`; frontend evidence when UI is in scope |
| Modernize a legacy application | [Legacy Application Modernization](lifecycle/legacy-modernization.md), [Application Lifecycle](lifecycle/application-lifecycle.md), [Product Frontend](frontend/product-frontend.md) | `scripts/appfw product analyze --summary --json`; `scripts/appfw product propose-model --summary --json`; `scripts/appfw product model-status --json`; `scripts/appfw product validate --json`; migration/API/frontend evidence when in scope |
| Change model, config, or generation | [Schema Design](model/schema-design.md), [Generated Ownership](start/generated-ownership.md), [Codegen API](reference/codegen-api.md) | `scripts/appfw product validate --json`; `scripts/appfw product generate --check --json` for product models; framework namespace for generator changes |
| Change runtime, provider, security, IX, or agentic controls | [Architecture](architecture/overview.md), [Intelligent Experience Patterns And Model Orchestration](architecture/intelligent-experience-patterns-and-model-orchestration.md), [Framework Packaging](architecture/framework-packaging.md), [Provider Certification](runtime/provider-certification.md), [Security Threat Model](architecture/concerns/threat-model.md), [Agentic Threat Model](architecture/concerns/agentic-threat-model.md) | `scripts/appfw framework test --fast --json`; provider checks when live |
| Build product frontend experience | [Frontend Starter Contract](frontend/product-frontend.md), [PDS Health Design System](frontend/pds-health-design-system.md), [Product Developer Golden Path](lifecycle/product-golden-path.md) | `scripts/appfw product frontend-test --json` |
| Plan or build an App Fabric mobile experience | [Mobile Operating Context](start/app-fabric-mobile-operating-context.md), [Mobile UX Master Plan](specs/app-fabric-mobile-ux-master-plan.md), [React Native Mobile App Contract](frontend/mobile-react-native.md), [Frontend Starter Contract](frontend/product-frontend.md), [PDS Health Design System](frontend/pds-health-design-system.md) | Planning: retained M0 contracts and framework handoff; implementation: `scripts/appfw product mobile-plan --ui-artifact <mockup.html> --json` plus RN type/build/device evidence when mobile source is present |
| Certify release or deployment | [Release Gate](release/release-gate-ci-cd.md), [Deployment Reference](release/deployment-reference.md), [Roadmap](release/roadmap.md) | `scripts/appfw product release-check --json`; strict release evidence in CI |
| Improve docs or maintainability | [Maintainability Command Center](architecture/concerns/maintainability.md), this index | `scripts/appfw framework docs-check --json` |
| Decide what to build next, or why | [Platform Strategy](strategy/app-framework-platform-strategy.md), [Product Development Strategy — North Star](strategy/product-development-north-star.md), [Product Management Strategy](strategy/app-framework-product-management-strategy.md), [Enterprise App Fabric Strategy](strategy/enterprise-app-fabric.md), [Wave 0 Contract Freeze](architecture/concerns/north-star-wave-0.md), [Wave 0 Spec Artifact Contracts](architecture/concerns/north-star-wave-0-specs.md) | N/A (direction-setting; proven by the Roadmap and this table's other rows once acted on) |
| Plan or oversee App Fabric delivery | [Enterprise App Fabric Strategy](strategy/enterprise-app-fabric.md), [August 2026 Industry And BOK Vetting](assessments/app-fabric-industry-and-bok-vetting-2026-08-03.md), [App Fabric Master Implementation Plan](release/app-fabric-master-implementation-plan.md), [Roadmap](release/roadmap.md) | Register executable work in the Product Increment portfolio and a validated plan; use the framework docs/governance proof loop for plan changes |

## Information Architecture

| Layer | Purpose | Canonical Docs |
| --- | --- | --- |
| Product Developer | Build, run, test, release, upgrade, modernize, and hand off a product app. | [Product Docs](product/README.md), [Product Developer Golden Path](lifecycle/product-golden-path.md), [PoC To Enterprise Product Intake](lifecycle/intake-and-discovery.md), [Legacy Modernization](lifecycle/legacy-modernization.md) |
| Framework Steward | Evolve generator, runtime, providers, docs IA, release gates, packaging, and reusable scaffolding. | [Framework Docs](framework/README.md), [Architecture](architecture/overview.md), [Maintainability](architecture/concerns/maintainability.md), [ADR Index](architecture/adr/README.md) |
| Start Here | First routing, Product Increment execution, ownership, command contracts, concise procedures, human/agent role boundaries, role cards, safe handoff, remote-promotion synchronization, spec-driven decision provenance, tech-debt capture, and team harness replication. | `AGENTS.md`, [Product Increment Delivery Model](start/product-increment-delivery-model.md), [Program Work System](reference/program-work-system.md), [Agent Skills Pack](../agent_skills/README.md), [Agent Task Map](start/agent-task-map.md), [Spec-Driven Change Harness](start/spec-driven-change-harness.md), [Agentic And Human Operating Model](start/agentic-human-operating-model.md), [Agent Role Cards](start/agent-role-cards.md), [Remote Promotion State Synchronization](start/remote-promotion-state-synchronization.md), [Tech Debt Register](start/tech-debt-register.md), [CLI Quickstart](start/cli-quickstart.md), [Generated Ownership](start/generated-ownership.md), [Team Harness Replication](start/team-harness-replication.md) |
| Specs | Durable decision provenance for meaningful framework or reusable platform changes. Specs must stay lean and drive implementation, tests, docs, review, and handoff. | [Specs Index](specs/README.md), [Spec Template](specs/spec-template.md), [Spec-Driven Change Harness](start/spec-driven-change-harness.md) |
| Product Delivery Lifecycle | Product app creation, PoC intake, legacy modernization, customization, generation, local run, testing, frontend, release, upgrade, drift, and handoff. | [Product Developer Golden Path](lifecycle/product-golden-path.md), [PoC To Enterprise Product Intake](lifecycle/intake-and-discovery.md), [Legacy Modernization](lifecycle/legacy-modernization.md), [Application Lifecycle](lifecycle/application-lifecycle.md) |
| Reference | Stable contracts for product work, providers, deployment, frontend, mobile, security, and generated ownership. | [Product Workspace Contract](reference/product-workspace-contract.md), [Product Workspace Boundaries](reference/product-workspace-boundaries.md), [App Manifest](reference/app-manifest.md), [Deployment Reference](release/deployment-reference.md), [Frontend Starter Contract](frontend/product-frontend.md), [React Native Mobile App Contract](frontend/mobile-react-native.md), [PDS Health Design System](frontend/pds-health-design-system.md), [Provider SDK Rules](runtime/provider-sdk.md) |
| Internal Architecture | Framework design intent, package boundaries, runtime ingress, architecture concerns, Wave 0 contract freeze, spec artifact contracts, and ADRs. | [Architecture](architecture/overview.md), [Framework Packaging](architecture/framework-packaging.md), [Runtime Modularity](architecture/concerns/runtime-modularity.md), [Packaging](architecture/concerns/packaging.md), [Wave 0 Contract Freeze](architecture/concerns/north-star-wave-0.md), [Wave 0 Spec Artifact Contracts](architecture/concerns/north-star-wave-0-specs.md), [ADR Index](architecture/adr/README.md) |
| Scorecard | Current readiness numbers, production blockers, and next priorities. | [Roadmap](release/roadmap.md) (self-assessment), [Enterprise Product Readiness Assessment](assessments/enterprise-product-readiness.md) (independent audit) |
| Strategy | Business-value rationale, multi-release direction, demand streams, required capabilities for agents/harnesses/automations, and new platform surfaces (e.g. mobile). Routes to Scorecard/Architecture/ADRs rather than restating them. | [Platform Strategy](strategy/app-framework-platform-strategy.md), [Product Development Strategy — North Star](strategy/product-development-north-star.md), [Product Management Strategy](strategy/app-framework-product-management-strategy.md), [Enterprise App Fabric Strategy](strategy/enterprise-app-fabric.md) |
| Archive | Historical implementation reviews and planning notes that should not guide fresh implementation. | [Archive](archive/README.md) |

The [Maintainability Command Center](architecture/concerns/maintainability.md) owns the rules for
documentation refactoring, command contracts, runtime ingress convergence,
golden downstream CI, frontend scaffold execution, and maintainability gates.
ADR [0013 Docs Information Architecture](architecture/adr/0013-docs-information-architecture.md)
records the durable design decision behind this hierarchy so future agents keep
following the same structure.

The command seam is one CLI with two canonical namespaces:

```bash
scripts/appfw product <command>
scripts/appfw framework <command>
```

Flat commands remain compatibility aliases. New docs, skills, prompts, and CI
examples should use the product or framework namespace.

## Agent Start Path

Agents should read only the smallest set needed for the task:

```text
AGENTS.md
agent_skills/README.md
docs/start/spec-driven-change-harness.md
docs/start/agentic-human-operating-model.md
docs/start/agent-role-cards.md
docs/start/tech-debt-register.md
docs/product/README.md
docs/framework/README.md
docs/start/agent-task-map.md
docs/start/cli-quickstart.md
docs/start/generated-ownership.md
```

Then branch into the path table above.

The normal safe loop is:

```bash
scripts/appfw context --json
scripts/appfw product validate --json
scripts/appfw product test
scripts/appfw product handoff --json
```

For docs or command-contract changes, add:

```bash
scripts/appfw framework docs-check --json
```

For generator or template changes, add:

```bash
scripts/appfw framework generate
scripts/appfw framework generate --check --json
```

## Canonical Contract Docs

Use these docs as source-of-truth modules. Do not duplicate their contracts in
new docs; route to them.

| Contract | Source |
| --- | --- |
| CLI commands and JSON outputs | [CLI Reference](reference/cli.md) |
| Product/framework ownership | [Product Workspace Contract](reference/product-workspace-contract.md), [Product Workspace Boundaries](reference/product-workspace-boundaries.md) |
| Generated and human-owned artifacts | [Generated Ownership](start/generated-ownership.md) |
| App topology manifest | [App Manifest](reference/app-manifest.md) |
| Config schema and validation contract | Generated `.appfw/model/_specs/CONFIG_CONTRACT.md` in each product app |
| Runtime architecture and ingress model | [Architecture](architecture/overview.md) |
| Provider certification and parity | [Provider Certification](runtime/provider-certification.md), [Provider Semantic Parity](runtime/provider-semantic-parity.md) |
| Release and deployment evidence | [Release Gate](release/release-gate-ci-cd.md), [Deployment Reference](release/deployment-reference.md) |
| Performance evidence | [Performance And Scalability](runtime/performance-and-scalability.md) |
| Security posture | [Security Threat Model](architecture/concerns/threat-model.md), [Agentic Threat Model](architecture/concerns/agentic-threat-model.md) |
| Product frontend scaffold | [Frontend Starter Contract](frontend/product-frontend.md) |
| App Fabric mobile research, planning, and continuation context | [Mobile Operating Context](start/app-fabric-mobile-operating-context.md), [Mobile UX Master Plan](specs/app-fabric-mobile-ux-master-plan.md); live execution remains in the Product Increment/PFC records |
| React Native mobile app scaffold | [React Native Mobile App Contract](frontend/mobile-react-native.md) |
| PDS Health design system | [PDS Health Design System](frontend/pds-health-design-system.md) |
| Policy contract | [Rego Policy Contract](../rego_test/docs/policy-contract.md) |

## Internal Architecture Docs

These docs are useful for framework stewards, but they are not the first stop
for normal product development:

| Document | Use It For |
| --- | --- |
| [Framework Packaging](architecture/framework-packaging.md) | Current and target package boundaries. |
| [Packaging Concern](architecture/concerns/packaging.md) | Durable product/framework packaging boundary. |
| [Runtime Modularity](architecture/concerns/runtime-modularity.md) | Durable runtime ingress and operation invocation boundary. |
| [Wave 0 Contract Freeze](architecture/concerns/north-star-wave-0.md) | Shared North-Star contract seams before parallel Wave 1/2 implementation. |
| [Packaging Boundary Matrix](architecture/packaging-boundary-matrix.md) | One-page package ownership baseline. |
| [Product Extension API](reference/product-extension-api.md) | Stable product-owned handler/service import surface. |
| [ADR Index](architecture/adr/README.md) | Durable design decisions. |

## Archive Rule

If a doc mainly explains what happened on a past branch, why an old bug existed,
or how a completed implementation was investigated, move it to
`docs/archive/` or rely on git history. Live docs should describe how to work
with the framework today.

Before adding a top-level docs file, answer:

1. Which reader job owns this?
2. Which existing doc would otherwise duplicate it?
3. Which command proves the contract?
4. Does `scripts/appfw framework docs-check --json` need an assertion?

## Contributor Rule

If behavior is generated repeatedly, change config, templates, generator code,
or provider/runtime code. If behavior is application-specific and intended to
survive regeneration, keep it in a human-owned extension point.

If generated drift appears, preserve the diagnostic and report it. Do not hide
generated drift by hand-editing generated output.
