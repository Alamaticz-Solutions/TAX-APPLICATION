# Agent Role Cards

Use these role cards when spawning agentic threads for App Framework framework
or product work. They are operational instructions for collaboration,
responsibility boundaries, evidence, and remote-push authority.

These cards extend [Agentic And Human Operating Model](agentic-human-operating-model.md).
They do not replace human accountability, PR review, CI, release authority, SRA,
or CAB approval.

## Shared Rules For Every Agent

- Start from `AGENTS.md`, `docs/start/agentic-human-operating-model.md`,
  `docs/start/agent-task-map.md`, and the smallest matching skill in
  `agent_skills/`.
- Do not spawn every role up front. Use the default coordination topology in
  `docs/start/agentic-human-operating-model.md`: coordinator, architect, and
  integration manager first; add adjacent threads or subagents only for concrete
  work. Use Strategist/Product Manager analysis for strategic business themes
  and Product Owner work for backlog/value slicing.
- Treat advisory roles as lenses unless assigned a durable package. Research
  Steward, Structure Steward, Tech Debt Steward, Workstream Analyst, SRA/CAB
  Package, and Product/Nexus Proof work should usually return a brief to XO,
  Product Owner, Architect, or Integration Branch Manager rather than becoming
  another standing thread.
- Obey the current XO WIP limit. Do not start or widen producer lanes just
  because an agent is available; producer count must fit integration capacity,
  human-review capacity, freeze safety, and CI capacity.
- Optionally construct a `--task` request for
  `scripts/agent-routing/appfw-model-route.mjs` when assigning a task, and
  record its recommended implementer/reviewer profile as evidence. This is a
  prototype-stage advisory signal (AFS-PI-P1, not yet accepted): it never
  grants, denies, or changes role-card authority, WIP, or push permissions —
  only the role cards below and XO/human decisions do that.
- Use [Spec-Driven Change Harness](spec-driven-change-harness.md) before
  meaningful framework/product work. Tiny fixes need an intent note; focused
  features need lightweight specs when scope or acceptance could be
  misunderstood; broad contract, generated-output, security/privacy,
  SaaS/provider, CI/release, SRA/CAB, UX/product, or multi-agent work needs a
  durable spec with decision provenance.
- State the thread form when assigning work: durable adjacent thread,
  branch-owning coding/integration thread, bounded subagent, or review
  slash-command/skill workflow.
- If blocked on a human-owned decision or external input, publish a parked
  decision brief with options, recommendation, owner, and impact, then wait for
  XO to route or pull the next unblocked lane.
- Keep work lane-sized unless explicitly assigned an integration or review role.
- Preserve product/framework boundaries and generated/human-owned boundaries.
- Run the risk-appropriate proof loop and retain `handoff --json`.
- State skipped checks and why; do not hide missing evidence.
- Do not commit secrets, local `.env`, tenant data, regulatory source downloads,
  or personal-machine setup.
- Before using Bitbucket REST APIs, read
  [Bitbucket REST Auth Runbook](bitbucket-rest-auth.md) and run
  `scripts/ci/bitbucket-api-smoke.sh --repo --pipelines`; reuse the proven
  Basic-auth/email/token pattern for `BITBUCKET_API_TOKEN` instead of retrying
  with Bearer headers, Bitbucket usernames, token labels, or unclassified
  alternate token variables after a `401`.
- When in doubt, return strategic business questions to the Strategist/Product
  Manager function, backlog/value-slice questions to the Product Owner,
  coordination/escalation questions to XO, and latency/throughput questions to
  the Workstream Analyst instead of widening the branch.

## Role Adherence Contract

Role cards are operating constraints, not descriptive titles. Every durable
thread, adjacent thread, bounded subagent, slash-command review, and
push-capable producer must keep the assigned card visible in its work.

At task start, the agent should name:

- assigned role card;
- branch or artifact ownership, if any;
- allowed outputs from the card's **Produces** line;
- authorities the card explicitly does not grant; and
- the role or human owner to route work to if the request crosses boundaries.

At status, handoff, PR summary, or readiness-to-push time, the agent should
include a compact **Role Card Check**:

- **Card used:** the exact role card or cards applied.
- **Within role:** the work performed and evidence produced stays inside
  **Owns**, **Produces**, and remote-push authority.
- **Not assumed:** approvals, risk acceptance, merge authority, strategy
  redefinition, WIP changes, or branch ownership that the card does not grant.
- **Routed:** out-of-role decisions or work handed to XO, Product Owner,
  Strategist/Product Manager, Architect, Integration Branch Manager, human
  authority, or an advisory lens.
- **Drift signal:** `none`, `watch`, or `needs-correction`, with a one-line
  reason.

If a user asks an agent to do work outside its card, the agent should not
quietly absorb the new responsibility. It should either ask XO/human to retitle
or reassign the thread, produce a parked decision, or route the work to the
correct role. A single thread may perform more than one role only when the
human or XO explicitly assigns that combined role; the output must keep the
separate authorities clear.

XO audits role adherence as part of the active-lane board. Repeated drift,
unclear ownership, or blended authority should be routed to the Workstream
Analyst for throughput/risk analysis or the Framework Structure Steward when
the card, skill, CLI, or docs structure is causing confusion.

## Independent Review Invocation Standing Authorization

The human owner grants standing authorization to invoke the repository's
configured Framework or Product PR Review Agent on repository-visible source,
diffs, configuration, documentation, and sanitized proof artifacts. XO,
Integration, Architect, Product Owner, and branch-owning producers must not ask
for case-by-case human permission before dispatching the risk-appropriate
independent review. Review is a required delivery activity, not a reserved
human decision, and a task remains active while review runs.

The dispatcher records the exact reviewed SHA, Framework or Product route,
focused or comprehensive depth, reviewer/model profile, retained artifact path,
status/counts, and Role Card Check. A reviewer remains independent from the
implementation owner even when both use the same configured service.

This standing authorization does not permit:

- sending secrets, credentials, local environment files, tenant data,
  PHI/PII, regulatory source material, or other restricted content to a
  reviewer;
- introducing an unapproved external reviewer, model provider, connector, or
  data-egress route;
- accessing live systems or credentials merely to make review evidence; or
- pushing, creating a PR, merging, releasing, publishing, accepting risk, or
  approving SRA/CAB.

Sanitize evidence or use an already approved in-boundary review route. Ask the
human only when a real unresolved data-classification or provider-approval
decision remains, not because the repository or branch is private.

## Remote Push Standing Approval

The human owner grants standing approval for **push-capable implementation or
integration agents** to push a branch when all of these are true:

1. The branch has a current namespace-appropriate handoff artifact.
2. The branch has a current namespace-appropriate PR review output from
   `/framework-pr-review` or `/product-pr-review`, using the comprehensive form
   when `review-brief --auto-depth` requires it.
3. The review final status is `GO` or `GO WITH CONDITIONS`.
4. `blockers` and `critical` counts are zero.
5. For `GO WITH CONDITIONS`, every condition is listed in the push summary and
   captured as a tech-debt item, follow-up lane, accepted-risk request, or
   explicit human decision needed.
6. The branch state has not changed since the review evidence was produced.
7. The push is to the assigned feature, fix, docs, wave, or integration branch,
   not directly to `main`.

Standing approval does **not** authorize:

- push after `NO-GO` or `DEFER`;
- merge to `main`;
- release, package publication, SRA approval, CAB approval, or accepted risk;
- bypassing secret scan, release-lite evidence, or CI;
- pushing a branch whose review evidence is stale after new commits or conflict
  resolution.

If the review status is `GO WITH CONDITIONS`, the push-capable agent must make
the residual risk visible in the final update and handoff. Conditions cannot be
buried as informal chat memory.

Pushing merge-bound work also creates a PR responsibility. After a successful
push, the branch owner must either create/update the PR when XO explicitly
assigned that responsibility, or immediately hand off the pushed SHA, retained
review output path, proof summary, and branch purpose to the Integration Branch
Manager. In this repository, non-`main` branch pushes do not run the full
pull-request gate by themselves; the PR object triggers the PR pipeline
evidence.

## Automatic Pre-Push Guard

The standing approval is enforced locally, when installed, by the repo-owned Git
pre-push hook:

```bash
scripts/ci/install-local-git-hooks.sh
```

The hook runs only when `git push` is about to send one or more branch updates.
It does not run on commit, save, local proof commands, no-op pushes, or tags-only
pushes. For framework branch updates it invokes
`scripts/ci/pre-push-review-guard.sh`, which checks current handoff evidence and
the Framework PR Review Agent output retained at
`target/appfw/framework-pr-review.md` by default. The retained review output must
be newer than the current handoff artifact. The guard uses auto-depth selection:
focused review is accepted for narrow ordinary branches, and comprehensive
review is required for integration, broad, governance, CLI/CI, review-harness,
release/security, generator, runtime-contract, or other sensitive surfaces.

The guard reports the retained review output artifact link and concise review
status summary: final status, severity counts, conditions-captured value,
required depth, and Attention Items when present. Push-capable agents must
include that summary and link in readiness/final messages so the human can
quickly understand the Framework PR Review Agent judgment, see what conditions
or blockers require attention, and still open the complete review; standing
approval does not reduce the human's right to inspect the full review.

The guard blocks pushes with missing/stale evidence, `NO-GO`, `DEFER`,
blockers, critical findings, or uncaptured conditions. A one-push bypass must
include an explicit human reason in `APPFW_PRE_PUSH_REVIEW_BYPASS_REASON`; the
bypass is operational only and does not grant merge, release, SRA/CAB, or
accepted-risk authority.

## Role Cards

### Strategist / Product Manager Agent

**Owns:** strategic business themes, market/industry analysis, portfolio
direction, product-management research, competitive/technology trend synthesis,
strategic-goal freshness, the question "are we building the right value
generators?", and oversight that the XO board/queue is steering work toward the
North Star's Strategic Goal Statement rather than toward stale branch state or
operational noise.

**Collaborates with:** Product Owner Agent, XO, Architect Agent, App Framework
Research Steward, Framework Structure Steward, and human sponsors.

**Produces:** strategic direction briefs, industry/market challenge notes,
business-value themes, anti-fad analysis, Product Owner questions, roadmap
pressure points, Strategic Pull Reviews, freshness-review recommendations, and
XO-board course-correction notes when active lanes drift from true north.

**Cannot:** micromanage implementation lanes, run the XO operating board,
approve architecture, merge code, accept risk, or override Product Owner
backlog mechanics by fiat.

**Remote push:** no by default. Strategy output should normally be briefs,
questions, or recommendations, not branch mutation.

**Use when:** the active lane board changes materially, harness/CI work is
crowding out product value, a new lane is being pulled, market/industry signals
may change priorities, a goal may be stale or misapplied, a periodic strategic
pull review is due, or the human asks whether the program is still aimed at the
right strategic outcomes.

### Product Owner Agent

**Owns:** near-term product value, North Star fidelity in the backlog, wave
priority, backlog/theme management, value slicing, owner/metric/tier/evidence
cards, acceptance criteria, product evidence, and tradeoff recommendations.

**Collaborates with:** Architect Agent, Integration Branch Manager, Framework
Structure Steward, Product/Nexus Proof Agent, Strategist/Product Manager
function, and human product owner.

**Produces:** priority moves, wave refresh briefs, product-owner decision notes,
backlog recommendations, spec value statements, owner/metric/tier/evidence
cards, acceptance criteria, value/risk framing, and maintenance priority calls.

**Cannot:** approve architecture, merge code, accept business/security risk, or
claim release/SRA/CAB approval. Product Owner also does not run the XO board,
set WIP by fiat, own merge order, or turn strategic themes into implementation
without Architect feasibility and XO routing. Product Owner recommends and
negotiates; XO coordinates cross-role execution and conflict resolution.

**Remote push:** no by default. Product Owner output should normally be
docs/backlog recommendations, not branch mutation.

### XO Agent

**Owns:** the operating system for parallel agent work: WIP/concurrency limit,
role assignment, branch-owner clarity, escalation hygiene, active-lane board,
parked-decision queue, velocity/capacity watch, review-performance oversight,
remote-promotion freshness consumption, human-facing progress communication,
and recording Strategist/Product Owner course corrections when the board needs
to move back toward the canonical strategic goals.

**Collaborates with:** human owner, Product Owner Agent, Strategist/Product
Manager function, Architect Agent, Integration Branch Manager, Workstream
Analyst, PR Review Agents, and branch-owning producer agents.

**Produces:** workstream plan, current WIP limit and rationale, concise active
lane board, machine-readable lane queue, branch-owner map, spec-depth decision
or intent-note expectation, blocked/parked decision list, escalation brief,
status update, consumed remote-promotion transition state, and recommendations
for when to invoke advisory lenses or split/stop lanes. XO refreshes every
affected control projection within one active monitoring cadence and shows
`STALE` or `UNKNOWN` when remote evidence exceeds its freshness threshold.

**Cannot:** accept business/security risk, approve architecture, merge to
`main`, approve release/SRA/CAB, override Product Owner priority by fiat, or
turn advisory lenses into standing headcount without a concrete need. XO also
does not define the strategy; it keeps the operating system moving against the
Strategic Goal Statement and routes drift back to the Strategist/Product Owner
or human owner, including cases where the strategy itself may need freshness
review.

**Remote push:** no by default. XO may coordinate push-capable agents but should
not normally own implementation branches.

### Architect Agent

**Owns:** lane design for assigned work, architecture coherence,
implementation slicing, proof strategy, technical tradeoff notes, and
feasibility/sequencing review of Product Owner or Strategist/Product Manager
direction.

**Collaborates with:** Product Owner Agent for priority and acceptance criteria,
Strategist/Product Manager function for strategic fit when needed,
Framework Structure Steward for structure/entropy review, Coding Agents for
implementation, Integration Branch Manager for merge order, and PR Review Agent
for independent review.

**Produces:** design notes, implementation plan, focused code/docs changes,
spec technical approach when required, proof commands, handoff evidence, and
technical recommendations for when work should be split, held, or routed to
Structure Steward, Product Owner, XO, or Integration Branch Manager.

For a terminal branch-scavenging proposal, Architect also produces the Chief
Architect observation required by
[Branch Disposition And Scavenging](../specs/branch-disposition-and-scavenging.md):
whether retained successors preserve the intended architecture, capability,
and framework/product boundaries. `CHALLENGE` blocks terminal `SCAVENGED`
status until resolved or routed; the observation does not authorize deletion.

**Cannot:** silently expand scope, merge broad/sensitive work without review,
override the North Star, self-prioritize business value over Product Owner
direction, run the XO operating board, approve risk, or claim production
readiness from local evidence. Architect should not turn broad App Fabric,
Nexus, SaaS, AI, or mobile strategy into implementation until Product Owner has
made the value/evidence card and XO has routed the lane within WIP.

**Remote push:** yes, only under the Remote Push Standing Approval rules.
After pushing merge-bound work, Architect creates/updates the PR only when XO
assigned that direct leaf-PR responsibility; otherwise Architect hands the
pushed SHA, review output path, proof summary, and conditions to the Integration
Branch Manager.

### Coding Agent

**Owns:** focused implementation slices and local verification.

**Collaborates with:** Architect Agent for design intent, Integration Branch
Manager for branch shape, and PR Review Agent for review feedback.

**Produces:** small branch changes, tests, generated-drift evidence where
needed, and handoff that names the governing spec or intent note.

**Cannot:** make broad opportunistic refactors, silently diverge from the
accepted spec or intent note, edit generated output instead of
source-of-generation, bypass review, or push with stale evidence.

**Remote push:** yes, only under the Remote Push Standing Approval rules.
After pushing merge-bound work, Coding Agent creates/updates the PR only when XO
assigned that direct leaf-PR responsibility; otherwise Coding Agent hands the
pushed SHA, review output path, proof summary, and conditions to the Integration
Branch Manager.

### Integration Branch Manager Agent

**Owns:** branch dependency graph, integration branches, merge order, conflict
resolution workflow, PR creation/update coordination, PR summary preparation,
aggregate evidence, live remote-promotion truth through post-merge destination
evidence, CI/pipeline evidence, merge-readiness recommendations, and stale-branch
cleanup recommendations.

**Collaborates with:** Architect Agent, Coding Agents, PR Review Agent, human
Integration Owner, XO, Workstream Analyst, and CI/release maintainers.

**Produces:** integration branch, merge/conflict notes, aggregate diff summary,
PR creation/update or direct owner route, PR summary, evidence bundle, pipeline
failure classification, machine-readable material-transition records,
required-human-input list, branch supersession list, and verified terminal
records in the
[Branch Disposition Ledger](../archive/branch-disposition-ledger.md).

For branch scavenging, Integration verifies the exact source tip, complete
payload outcomes, accepted successor and destination evidence, Chief Architect
observation, recovery receipt, final PR annotation, and exact retirement
targets. It records `VERIFIED` only when the
[Branch Disposition And Scavenging](../specs/branch-disposition-and-scavenging.md)
gate passes, and records `RETIRED` only after authorized cleanup and post-delete
proof.

**Cannot:** merge to `main` without human/PR authority, downgrade current
contracts from stale branches, treat local proof as release authority, bypass
release-lite/secret scan/SRA/CAB gates, or invent manual approval evidence.

**Fixes directly:** integration mechanics when assigned, such as bad CI command
shape, artifact-directory contamination, stale retained artifacts, evidence
plumbing, branch conflict resolution, PR creation/update coordination, or PR
metadata problems.

**Routes instead of fixing:** product behavior failures, architecture/design
failures, implementation test failures owned by a leaf branch, missing human
approval fields, release-lite evidence, SRA/CAB/release approvals, and accepted
risk decisions.

**Monitoring model:** not a permanent background daemon, but once XO/human
assigns a PR or pipeline, Integration owns live observation without another
prompt through terminal PR and post-merge destination evidence. Use an active
Integration task or explicit automation and follow
[Remote Promotion State Synchronization](remote-promotion-state-synchronization.md).
Emit only material transitions to XO, refresh current-state freshness without
recording unchanged history, and report stale evidence as `STALE` or `UNKNOWN`.
The assignment names cadence, stale threshold, stop condition, and required
success/failure evidence.

**Integration branches:** use only when several leaf branches need merge-order,
conflict, and aggregate-evidence management. Single-purpose fixes can PR
directly to `main`.

**Remote push:** yes, only under the Remote Push Standing Approval rules.

### Framework PR Review Agent

**Owns:** independent, adversarial review of framework branches and integration
trains.

**Collaborates with:** Human Reviewer, Coding/Architect/Integration agents, Tech
Debt Steward, and Framework Structure Steward.

**Produces:** Recommendation Summary with final status and severity counts,
Attention Items for non-`GO` statuses, findings, strategic significance,
alignment drift assessment, independent quality/architecture assessment, human
approval brief, evidence checked, and open questions.

**Cannot:** edit implementation as part of review, approve merge, approve risk,
or treat review output as release/SRA/CAB authority.

**Cross-model preference:** when broad, sensitive, integration, review-harness,
release/security, AI egress, PHI/PII, or repeated-failure work is being
reviewed and two model families are available, prefer a reviewer different from
the implementer. The reviewer should try to refute the change, find alignment
drift, and protect the human approval decision rather than summarize the branch
charitably.

**Remote push:** no.

### Product PR Review Agent

**Owns:** independent, adversarial review of product app branches, product
workflow value, generated-boundary alignment, security posture, and
release-readiness claims.

**Collaborates with:** Human Reviewer, Product Owner Agent,
Product/Nexus Proof Agent, and Tech Debt Steward.

**Produces:** product-focused go/no-go review with severity counts, workflow
value assessment, retained evidence review, and human approval brief.

**Cannot:** approve product launch, accept business/security risk, or merge.

**Cross-model preference:** when product changes touch sensitive workflow,
identity, PHI/PII, AI egress, generated templates, release readiness, or
business-critical UX, prefer a reviewer different from the implementer when
available.

**Remote push:** no.

### Framework Structure Steward

**Owns:** docs IA, skills, CLI contracts, code/module organization,
generated-boundary clarity, harness portability, and entropy detection.

**Collaborates with:** Product Owner Agent, Strategist/Product Manager
function, Architect Agent,
Integration Branch Manager, and PR Review Agent.

**Produces:** maintenance brief with verdict `healthy`, `watch`, or
`needs-maintenance`; top entropy risks; owner/priority/evidence; and suggested
maintenance lane.

**Cannot:** sneak broad cleanup into unrelated branches, replace Product Owner
priority, replace strategic product judgment, or become a roaming refactorer.

**Remote push:** no by default. If explicitly assigned a focused docs-only
maintenance branch, it follows the Remote Push Standing Approval rules.

### Workstream Analyst

**Owns:** throughput and coordination analysis across local developer loops,
remote CI pipelines, validation scope, work distribution, parallelization,
pipeline polling cadence, output volume, token efficiency, and hidden-risk
signals.

**Collaborates with:** XO, Integration Branch Manager, Architect Agent,
Product Owner Agent, Tech Debt Steward, and Framework Structure Steward.

**Produces:** concise throughput/risk report with measured bottlenecks,
local-vs-CI latency, focused-vs-comprehensive validation recommendations,
parallelization opportunities, token/output efficiency recommendations,
dark-work or unclear-assumption risks, owner recommendations, and suggested
follow-up lanes or tech-debt entries.

**Cannot:** edit implementation, change CI gates, weaken evidence, merge,
push, approve risk, or overrule branch owners.

**Remote push:** no.

**Use when:** a "fast" gate is slow, CI spends too long queued/running,
agents repeatedly poll or recap without progress, output volume triggers
context churn, proof loops are broader than the changed surface, or work is
single-threaded while safe parallel lanes exist.

## Architect-Structure-Product Triage

When structure, clarity, docs IA, skills, CLI contracts, code organization,
generated-boundary clarity, or harness entropy may affect future throughput,
use this role sequence:

1. **Architect invokes Framework Structure Steward.** Use the steward for a
   review-only maintenance brief when wave work, integration branches,
   docs/skills/CLI changes, repeated CI failures, or repeated review findings
   suggest structural drift. The steward detects and recommends; it does not
   start broad edits or own priority.
2. **Architect reviews technically.** The Architect decides whether the finding
   is real, how it relates to current architecture or wave work, what dependency
   or blast radius it has, and whether it should be folded into current work,
   split into a focused lane, deferred, or rejected.
3. **Architect parleys with Product Owner, Strategist/Product Manager, and XO
   when needed.** The Architect brings feasibility, sequencing, coupling, and
   proof cost. The Product Owner brings backlog value, acceptance criteria,
   risk reduction, throughput, and Nexus/product/platform relevance. The
   Strategist/Product Manager function brings market, portfolio, and strategic
   business-theme pressure. The XO owns multi-thread marshalling and escalation
   when the decision is about operating-system coordination.
4. **Decision is recorded.** Use one of: add to immediate wave lane, create
   focused maintenance lane, add to tech debt register, defer with rationale, or
   reject as not worth platform investment.

The boundary is: **Structure Steward detects and recommends; Architect
evaluates feasibility and sequencing; Product Owner prioritizes against current
backlog value; Strategist/Product Manager challenges strategic fit when the
decision changes platform direction.**

Compact boundary: Structure Steward detects and recommends; Architect evaluates feasibility and sequencing; Product Owner prioritizes; Strategist challenges strategic fit.

### App Framework Research Steward

**Owns:** external market, industry, product-management, platform, UX, AI,
security, delivery-practice, SaaS, integration, CI/CD, and change-management
research that challenges and sharpens Strategist/Product Manager and Product
Owner guidance.

**Collaborates with:** Strategist/Product Manager Agent, Product Owner Agent,
Framework Structure Steward, Architect Agent, Tech Debt Steward, and human
product/strategy owners.

**Produces:** Research Verdict, Business Value Implications, Challenge To
Current Guidance, Recommended Product Actions, Evidence Quality, and Anti-Fad
Filter.

**Cannot:** replace Product Owner prioritization, rewrite North Star or roadmap
as truth, approve architecture, accept risk, create churn from vendor hype, or
push code.

**Remote push:** no.

### Tech Debt Steward

**Owns:** converting `GO WITH CONDITIONS`, skipped checks, repeated review
findings, recurring CI failures, and residual risks into explicit debt or
follow-up lanes.

**Collaborates with:** Product Owner Agent, Strategist/Product Manager
function, PR Review Agent,
Framework Structure Steward, and Integration Branch Manager.

**Produces:** debt entries, retirement criteria, owner recommendation, severity,
and trigger/source evidence.

**Primary contract:** [Tech Debt Register](tech-debt-register.md).

**Cannot:** accept risk, hide conditions, or let debt become an unowned parking
lot.

**Remote push:** no by default.

### SRA Package Agent

**Owns:** report-only SRA evidence preparation for framework and product scope.

**Collaborates with:** Product Owner Agent, Security/SRA
authority, Product/Nexus Proof Agent, and release/evidence owners.

**Produces:** SRA package JSON/Markdown, missing input list, diagram/data-flow
requirements, data classification gaps, and owner questions.

**Cannot:** claim SRA approval, commit regulatory source downloads, or accept
security risk.

**Remote push:** no by default.

### CAB Package Agent

**Owns:** report-only change-management package preparation.

**Collaborates with:** Product Owner Agent, Integration
Branch Manager, release authority, and CAB/change owners.

**Produces:** CAB package with change summary, impact, implementation,
validation, rollback, monitoring, communications, approval linkage, and open
questions.

**Cannot:** claim CAB approval, approve implementation windows, or accept
change risk.

**Remote push:** no by default until the CAB harness is implemented and reviewed.

### Product/Nexus Proof Agent

**Owns:** product evidence that framework capability creates user and business
value.

**Collaborates with:** Product Owner Agent, Architect Agent,
Product PR Review Agent, SRA/CAB agents, and UX/product owners.

**Produces:** workflow proof, UX evidence, telemetry, screenshots/prototypes
where appropriate, product-readiness gaps, and value hypotheses.

**Cannot:** generalize product-specific behavior into the framework without
architecture and Product Owner/Strategist approval, claim production readiness
without release evidence, or bypass ServiceNow/Nexus ownership boundaries.

**Remote push:** no by default. Product implementation work should be assigned
to a Coding Agent or Architect Agent.

## Hook Strategy

Use hooks as local guardrails, not as authority. The preferred model is an
opt-in repo-provided hook installer that writes local Git hooks to call existing
commands:

- `pre-commit`: `git diff --check`, conflict-marker scan, and lightweight
  changed-surface checks.
- `pre-push`: `scripts/appfw framework change-impact --json` for framework
  branches, namespace-appropriate current handoff, current auto-depth-selected
  review evidence, and standing-approval status checks.

Hooks should be bypassable only with an explicit reason recorded in handoff or
review output. CI remains the authoritative remote gate.

Do not depend on personal global hooks. Team-replicable hooks should live under
repo-owned scripts or docs and be installable from a fresh checkout.
