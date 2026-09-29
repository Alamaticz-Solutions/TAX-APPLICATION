---
name: product-legacy-modernization
audience: product
phase: intake
cli_namespace: product
artifacts: .appfw/manifest.yaml,.appfw/legacy-modernization.yaml,.appfw/legacy-analysis.yaml,.appfw/model-proposal.yaml,target/appfw/product-analysis.json,target/appfw/model-proposal.json,target/appfw/model-status.json,.appfw/target/appfw/validation.json,target/appfw/agent-handoff.json
description: Use when turning a real legacy application's governed evidence into a first-class As-is Modernization IR, a separately approved to-be treatment, a traceable App Framework target where selected, and independently verified migration or coexistence evidence.
---

# Product Legacy Modernization

## Use When

- A real legacy application is being replaced or strangled.
- Evidence includes source code, Jira/Confluence, target guidance, database
  schema, stored procedures, jobs, integrations, auth, reports, tests,
  telemetry, runbooks, or production-like workflows.
- The task requires a typed, provenance-backed As-is Modernization IR,
  objective treatment decision, and approved transformation plan before App
  Framework source is written.

## Procedure

1. Read `docs/lifecycle/legacy-modernization.md`. Charter one named application
   and bounded capability or journey. Require named business, domain, data,
   security, operations, modernization, and release authorities; explicit
   scope/exclusions; approved evidence identities and environments; baseline
   economics; proof standard; outcome metric; and stop criteria. Park work
   that lacks these decisions.
2. Establish a governed evidence snapshot before analysis. Record stable native
   IDs, versions, hashes, collection time/method, owner, freshness, ACL and
   classification, supported claims, and missing access for repositories,
   Jira, Confluence, CMDB, SQL, IIS/ECS, tests, telemetry, and interviews. Use
   least-privilege read-only identities or approved sanitized exports; do not
   imply that the current CLI has live enterprise connectors.
3. Reproduce the selected legacy build/runtime and exercise representative
   behavior in an approved environment. If that is impossible, retain the
   exact failure, dependency and environment record instead of silently
   inferring behavior from source.
4. Run deterministic, stack-aware analysis before probabilistic inference:
   MSBuild/Roslyn for .NET, TypeScript/Angular analysis, DacFx/ScriptDom/
   catalog/Query Store for SQL Server, configuration parsers for IIS/ECS, and
   security/dependency tools appropriate to the scope. Add controlled dynamic
   tests, traces, logs, metrics, UI/API/database probes, and user/operator
   observation. Preserve coverage denominators and tool provenance.
5. Build the application-scoped **As-is Modernization IR** described in the
   canonical guide. Reconcile claims about capabilities, behavior, data,
   integrations, identity/policy, experience, runtime, and operations while
   preserving conflicts, unknowns, evidence grade, confidence, freshness, and
   claim-specific authority. Obtain named human confirmation that the model is
   adequate for the bounded scope, never that the whole application is fully
   understood.
6. Compare objective dispositions for every material capability: retire,
   retain/contain, rehost, upgrade, refactor, replatform, SaaS or enterprise
   replacement, extract, strangle, App Framework rebuild, or split. App
   Framework is not the automatic answer. Record value, cost, risk, data,
   security, experience, operations, migration, lock-in, and rejected options.
7. Produce and obtain risk-appropriate approval for a separate, versioned,
   platform-neutral **Approved To-be Treatment Model**. Include target
   boundaries, retained/changed/retired behavior, data authority,
   interfaces/events/actions, policy, support,
   characterization and target tests, migration/reconciliation, coexistence,
   rollout/cutover/rollback, and retirement obligations.
8. Only for capabilities whose Approved To-be Treatment Model selects App
   Framework, create or inspect the clean product shell with
   `scripts/appfw product new <target> --profile product-intake --source-kind legacy`
   and complete `.appfw/legacy-modernization.yaml`. The shell is a target
   workspace, not evidence that the treatment decision was correct.
9. Run `scripts/appfw product analyze --summary --json` and treat
   `.appfw/legacy-analysis.yaml` plus `model_clues` as bounded profiler output.
   Run `scripts/appfw product propose-model --summary --json`, reconcile its
   candidate surface to the approved treated model, and keep unresolved and
   terminal choices explicit in `review_decisions`. Neither artifact is the
   complete Modernization IR or an approved target.
10. Run `scripts/appfw product model-status --json` before writing source and
    after each checkpoint. Move to `proposal_reviewed` only after accepted
    decisions name `model_owner` and `reviewed_at_utc` and clear required
    unknowns. Use `source_authoring_plan` only as the queue for already approved
    target mappings.
11. Optionally run `scripts/appfw product scaffold-model --dry-run --json` to
    preview starter source. Write it only for approved entity decisions with
    explicit classification. The command does not decide service boundaries,
    stored-procedure treatment, migration, UI, or policy semantics.
12. Route each thin implementation slice through a portable worker contract.
    Retain the equivalent of `ModernizationTask`, capability handshake, and
    `ModernizationResult` until canonical schemas exist: versions, evidence,
    allowed/forbidden scope, permissions, budget, checks, commands/systems,
    changes, assumptions, failures, unknowns, deviations, sensitive-data
    events, human interventions, runtime/cost, stop/escalation, and handoff.
    Claude Code, Codex, deterministic recipes, and specialist engines remain
    replaceable workers; private session state cannot be authoritative.
13. Preserve source-to-decision-to-target traceability while implementing and
    generate only after treatment approval. Use independent oracles for build,
    behavior, data/SQL, integration, permission, security/privacy, UX/
    accessibility, performance, resilience, operations, migration, rollback,
    and retirement. Generated plans, code, and tests cannot validate
    themselves.
14. Release progressively under existing human authority, observe outcomes,
    reconcile behavior/data, stabilize support, and retire legacy callers,
    infrastructure, support paths, and cost. If coexistence remains, name its
    owner, duration, residual cost, and retirement trigger. Feed corrections,
    accepted patterns, quality, cost, and effort into the second-use baseline.

## Proof

```bash
scripts/appfw product validate --json
scripts/appfw product analyze --summary --json
scripts/appfw product propose-model --summary --json
scripts/appfw product model-status --json
scripts/appfw product scaffold-model --dry-run --json
scripts/appfw product generate
scripts/appfw product generate --check --json
scripts/appfw product test --fast --json
scripts/appfw product handoff --json
```

Add migration, API, frontend, provider, performance, security, and release
evidence when the modernization slice touches those surfaces. These commands
prove the current bounded App Framework target path; they do not prove the
evidence graph, treatment quality, behavioral equivalence, safe migration, or
legacy retirement.

## Guardrails

- Do not commit production credentials, connection strings, tenant data, or PHI.
- Do not copy unrestricted Jira/Confluence content into tracked artifacts;
  retain stable source/version references, hashes, and approved summaries.
- Do not claim complete understanding from code, documentation, or agent output.
- Do not build a general coding agent, make App Framework the mandatory
  disposition, or treat a vendor-generated plan as proof.
- Do not silently resolve code/document/ticket/runtime contradictions; route
  them to the named authority and record the decision.
- Do not write `.appfw/model` directly from raw evidence, the As-is
  Modernization IR, or an agent-generated plan. The Approved To-be Treatment
  Model is the decision boundary; unsupported or lossy mappings require
  explicit diagnostics, extensions, or approved waivers.
- Do not let one worker infer the target, implement it, generate its own oracle,
  and self-approve material risk. Keep worker-private memory, session history,
  proprietary plan formats, and generated tests out of the system of record.
- Do not make large stored procedures permanent black boxes by default.
- Do not expose raw database primary keys in product URLs.
- Do not copy legacy UI technology; rebuild the experience with product
  frontend standards and generated contracts.
- Prefer strangler slices with reconciliation and rollback over big-bang
  rewrites.
- Do not call an added target modernization success while all legacy support
  cost remains. Require verified retirement or explicitly costed coexistence.
- Do not create a generic cross-enterprise canonical model, universal code
  translator, custom code-search platform, broad connector catalog, workflow
  engine, or new contract family without a named pilot and measured reduction
  in accepted modernization effort.
