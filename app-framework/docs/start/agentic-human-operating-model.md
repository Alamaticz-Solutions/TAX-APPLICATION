# Agentic And Human Operating Model

This is the durable role contract for App Framework framework and product work.
It exists so the same operating posture can be reproduced from any workstation,
agent harness, or team handoff without relying on chat history.

Use [Agent Role Cards](agent-role-cards.md) when spawning a specific agentic
thread. Role cards define collaboration expectations, outputs, remote-push
authority, and "cannot do" boundaries for each role.

## Product Increment Delivery Invariant

All new planned work follows the
[Product Increment Delivery Model](product-increment-delivery-model.md).
Every major batch is registered once in the Product Increment portfolio, and
every Deliverable belongs to exactly one primary Product Increment. Every
source-active Product Increment has a validated plan containing one to four
independently admissible Delivery Lanes. Multi-lane or cross-domain increments
use one short-lived integration branch; a cohesive one-lane increment may
declare `direct_main`.

The portfolio operating objective is two to four concurrently progressing
source lanes across compatible Product Increments without forcing one lane to
wait for another. Program Flow may
lower concurrency when review, integration, CI, repository, or human
acceptance capacity is constrained. It must show the reason and recovery
action. It may not create extra work merely to keep agents busy.

Before admission:

1. Product registers the Outcome, Product Increment decision, owner, current
   status, next action, `requires`, `benefits_from`, and acceptance criteria.
2. Architecture freezes shared contracts and verifies lane write roots do not
   overlap.
3. Each hard dependency is already sufficient or is decoupled by a frozen
   contract, fixture, or adapter with an explicit integration action.
4. Integration selects `integration_branch` or `direct_main`; multiple lanes
   must converge through `integrate/<product-increment>` from accepted `main`.
5. Program Flow runs the portfolio validator and
   `node scripts/check-product-increment-plan.mjs --plan <plan.json> --json`.

Multi-lane leaf PRs target the Product Increment integration branch and use the
accelerated, changed-surface proof loop unless automatic risk classification
escalates them. The integration PR to `main` pays the aggregate framework,
supply-chain, secret, regression, and comprehensive-review cost once. Release
candidates and tags retain strict release evidence. The sole PR for a
`direct_main` increment runs full candidate proof.

The dashboard projects the canonical portfolio rather than maintaining a
second status narrative. It exposes each Product Increment and lane's
**Provides**, **Requires**, and **Benefits from** relationships, plus flow,
Implemented, and Accepted state separately. This makes dependency, reuse, and
the next accountable action visible during integration planning.

Parallelism uses event-driven roles, not standing agent occupancy. Keep the
control crew loaded; activate producers for admitted, disjoint lanes and
reviewers for immutable checkpoints. Every lane receives only the four
controlling records plus linked source needed for its immediate decision.
Efficient configured model tiers perform retrieval and deterministic work;
frontier reasoning is reserved for novel design, consequential conflict, and
independent synthesis. Program Flow reduces WIP or model tier when two
comparable cycles increase spend without improving Accepted lead time,
findings, or rework.

## Core Principle

Agents produce work, evidence, analysis, options, and recommendations. Humans
own intent, priority, risk acceptance, architecture approval, merge approval,
release approval, and business accountability.

Agentic throughput is valuable only when it increases trusted business value.
The harness should make strong progress easy, but it must make broad,
unreviewed, low-evidence change hard to approve by accident.

## Human Roles

| Role | Owns | Key decisions |
| --- | --- | --- |
| Executive / Business Sponsor | Business value, funding, strategic outcomes | What outcomes matter, what risk is acceptable, when value is proven. |
| Strategist / Product Manager function | Strategic business themes, market/industry analysis, portfolio direction, strategic-goal freshness, whether the framework is creating the right value generators, and whether XO coordination is steering toward the canonical strategic goals | Which demand streams matter, which themes deserve investment, where the Product Owner should sharpen or challenge backlog direction, when strategic goals need evidence-based refresh, and when the XO board/queue needs course correction toward true north. |
| App Framework Product Owner | Near-term product value, North Star fidelity in the backlog, value slices, acceptance criteria, product evidence, and priority recommendations | What gets built next, what gets deferred, what counts as ROI for the next wave or lane. |
| XO | Multi-thread operating-system marshalling, role coordination, velocity/capacity watch, and escalation hygiene | Which thread owns which responsibility, when work pauses for human/architectural decision, how the branch train stays coherent, and when workstream friction needs escalation. |
| Human Architect / Principal Engineer | Architecture integrity, system boundaries, hard technical tradeoffs | Whether design is acceptable, scalable, secure, and maintainable. |
| Human Reviewer / Approver | Final approval of PRs, risk exceptions, merge readiness | Approve, reject, or approve with conditions. |
| Security / SRA / CAB / Release Authorities | External governance and production approval | SRA approval, CAB approval, release authority, and accepted risk. |
| Integration Owner | Merge order and branch health | Which PRs merge, conflict resolution acceptance, and main readiness. |

## Agentic Roles

| Agent role | Owns | Output |
| --- | --- | --- |
| Strategist / Product Manager Agent | Strategic product analysis, market/business alignment, roadmap challenge, business-value themes, strategic-goal freshness, and XO-board alignment to the North Star's Strategic Goal Statement | Strategic direction briefs, industry/market challenge notes, value-generator recommendations, Product Owner questions, freshness-review recommendations, and XO-board course-correction notes. |
| Product Owner Agent | Backlog clarity, value slicing, acceptance criteria, near-term product evidence, and priority recommendations | Priority moves, wave refresh briefs, backlog recommendations, product evidence gaps, and value/risk framing. |
| XO Agent | Workstream coordination, WIP/concurrency control, escalation hygiene, role accountability, remote-promotion freshness consumption, velocity/capacity watch, and human-communication cadence | Workstream plan, active-lane board, parked-decision queue, owner routing, consumed remote transitions, status brief, and friction/escalation calls. |
| Architect Agent | Lane design, implementation strategy, architecture coherence | Design notes, implementation slices, proof plans, code/docs changes. |
| Coding Agent | Focused implementation | Small branches, tests, handoff evidence. |
| Integration Branch Manager Agent | Branch dependency graph, integration branches, PR creation/update coordination, live remote-promotion truth through post-merge destination evidence, and PR summary preparation | Merge order, conflict plans, integration PR summaries, material-transition records, and CI evidence. |
| Framework PR Review Agent | Independent review of framework changes | `GO`, `GO WITH CONDITIONS`, `NO-GO`, or `DEFER`; findings; risk counts; human approval brief. |
| Product PR Review Agent | Independent review of product app changes | Product-value, workflow, security, generated-boundary, and release-readiness review. |
| Framework Structure Steward | Docs IA, skills, CLI contracts, code organization, harness entropy | Maintenance brief back to Product Owner/Strategist with priority, owner, evidence, and suggested lane. |
| App Framework Research Steward | External industry, market, UX, AI, platform, security, and delivery-practice signals | Research challenge brief back to Product Owner/Strategist with evidence, anti-fad analysis, and recommended product actions. |
| Workstream Analyst | Local loop latency, CI pipeline latency, validation scope, work distribution, parallelization, token/output efficiency, and dark-work risk | Throughput/risk report with bottlenecks, recommended gate splits/caching, polling cadence, validation-scope advice, and concrete follow-up items. |
| Tech Debt Steward | Accepted conditions and repeated findings | Tech debt register entries and retirement recommendations. |
| SRA Package Agent | SRA evidence preparation | Report-only SRA package, gaps, and required human inputs. |
| CAB Package Agent | Change-management package preparation | Report-only CAB/change evidence package. |
| Product/Nexus Proof Agent | Product proof, UX evidence, telemetry, workflow validation | Product evidence that a capability creates real user/business value. |

## Default Coordination Topology

Do not spawn every role up front. The default operating shape is a small,
stable command structure:

| Coordination point | Default form | Owns |
| --- | --- | --- |
| Strategist / Product Manager function | Current Strategist thread or on-demand research/strategy refresh | Strategic business themes, market/industry challenge, portfolio direction, and whether the framework and XO board are still aimed at high-ROI value generators. |
| App Framework Product Owner | Durable product-owner thread | Backlog/value slicing, product evidence, acceptance criteria, priority recommendations, and deciding what should happen next from a business-value perspective. |
| XO | Durable operating coordinator | Role assignment, multi-thread marshalling, velocity/capacity watch, escalation hygiene, review-performance oversight, and keeping the coordination system coherent. |
| Architect Agent | Durable architecture/implementation thread | Wave design, lane slicing, implementation guidance, technical coherence, and push-capable lane work only after review policy is satisfied. |
| Integration Branch Manager Agent | Durable integration thread | Branch dependency order, integration branches, conflict workflow, PR coordination, live remote-promotion truth, material transitions, CI evidence, and merge readiness. |

Spawn additional roles only when there is a concrete need:

- Use an **adjacent thread** for durable work that needs its own branch,
  long-lived context, human follow-up, or push authority. Examples: a Coding
  Agent for a named lane, SRA Package Agent, CAB Package Agent, or Product/Nexus
  Proof Agent.
- Use a **subagent** for bounded analysis that should return to the
  coordinator and should not own a branch. Examples: inspect artifacts, compare
  roadmap to North Star, review docs/skills/CLI drift, run a focused research
  refresh, or audit one PR review output.
- Use **slash-command or skill workflows** for normal PR review before push or
  merge. Make a review agent a durable adjacent thread only when a large
  integration PR needs extended review. The configured independent review is
  standing-authorized for repository source and sanitized evidence; dispatch
  it without a per-review human permission prompt.
- Use a **Workstream Analyst** after repeated latency, long-running local
  commands, slow CI steps, excessive output/token burn, unclear ownership, or
  repeated polling. The analyst is advisory: it recommends changes to
  validation scope, CI splitting/caching, polling cadence, token discipline, or
  branch distribution, then returns to XO/Product Owner/Architect/Integration
  for routing.

The operating rule is: one owner per branch, one Product Owner for backlog
priority, one XO for operating coordination, one integrator for merge order,
and review independent from implementation.

## Role Adherence Loop

Role drift is an operating risk. A thread that starts as Product Owner and
quietly becomes XO, Architect, Integration Branch Manager, or approver can make
good local progress while weakening accountability and human review.

Use this loop for every durable role thread, adjacent branch-owning thread,
bounded subagent, and review workflow:

1. **Assignment names the card.** XO or the human names the role card,
   expected output, branch/artifact ownership, push authority, and the role to
   route out-of-bound work to.
2. **Agent self-checks.** At task start and handoff/readiness time, the agent
   reports a compact Role Card Check: card used, work within role, authority
   not assumed, routed decisions, and drift signal (`none`, `watch`, or
   `needs-correction`).
3. **XO monitors the board.** XO keeps role-owner clarity on the active-lane
   board and corrects threads that blend ownership, WIP authority, merge
   authority, strategy definition, implementation, and approval.
4. **Review agents inspect role adherence.** PR review output includes a Role
   Adherence Assessment so the human can see whether the branch stayed inside
   its assigned card or needs rerouting before push/merge.
5. **Drift is routed, not normalized.** If a role repeatedly crosses its
   boundary, XO routes the issue to the Workstream Analyst for throughput/risk
   causes or to the Framework Structure Steward when the role cards, skills,
   CLI, or docs are unclear.

Role adherence does not mean agents should be passive. It means agents stay
proactive inside their responsibility and make boundary crossings explicit
before acting on them.

## Strategic Alignment Loop

The canonical true north is the North Star's
[Strategic Goal Statement](../strategy/product-development-north-star.md#strategic-goal-statement).
The XO does not define that strategy; the XO keeps the operating system moving
against it.

Use this loop whenever the active lane board changes materially, a new lane is
pulled, work becomes dominated by CI/harness repair, or the human asks whether
the program is still on path:

1. **XO maintains the board and queue.** XO keeps
   `target/appfw/xo-active-lane-board.md` and
   `target/appfw/lane-queue-pull-model.json` current with active lanes, held
   lanes, blockers, owners, WIP, decisive PR/CI evidence, next recommended
   pull, and human decisions needed.
2. **Strategist audits direction, not branch mechanics.** The Strategist
   reviews the board against the Strategic Goal Statement and asks whether the
   next pull advances business value, Nexus/workflow transformation, governed
   integration, intelligent-app maturity, enterprise protection, agentic
   throughput, or productized framework adoption.
3. **Product Owner turns direction into priority.** Product Owner converts
   strategic pressure into backlog choices, owner/metric/evidence cards, and
   acceptance criteria.
4. **Architect and Integration execute the lane.** Architect owns design and
   implementation slicing; Integration owns PR/CI/merge-readiness evidence.
5. **XO records course corrections.** When the Strategist identifies drift,
   XO updates the board/queue with the corrected priority, held-lane rationale,
   owner, or human decision ask.

Strategic drift examples:

- the board optimizes for fixing harness friction while product-value lanes
  stay invisible;
- branch state or CI noise becomes the strategy;
- generic connector, AI, mobile, or platform work is pulled without a named
  business outcome and evidence path;
- App Framework starts acting like a replacement for ServiceNow, Snowflake, or
  other enterprise platforms instead of the PDS-owned app layer around them;
- agent throughput increases while human review, evidence, security, or
  maintainability confidence falls.

The Strategist's correction is directional and evidence-based. It does not
micromanage implementation, override Product Owner backlog mechanics, approve
architecture, approve merge/release/SRA/CAB, or assign push authority.

### Strategy Freshness

Alignment is not enough if the goals themselves become stale. The Strategist
should run a freshness check at wave refresh points and whenever material
signals appear from Nexus/business direction, downstream product adoption,
market research, CI/review/debt patterns, security/SRA/CAB findings, or
support/operations evidence.

A freshness check does not rewrite strategy casually. It produces a concise
recommendation to keep, clarify, reprioritize, retire, or propagate changes
from the North Star into product strategy, roadmap, role cards, review gates,
or the XO board. The Product Owner converts accepted changes into backlog and
roadmap work; XO updates the board and routes owners.

The Strategist should also produce a lightweight **Strategic Pull Review** on a
periodic cadence or when the human asks where the program is on the path. This
review is meant to pull the program forward, not make tactical branch
decisions. It should reference:

- [Product Development Strategy - North Star](../strategy/product-development-north-star.md)
- [Enterprise App Fabric Strategy](../strategy/enterprise-app-fabric.md)
- [Product Management Strategy](../strategy/app-framework-product-management-strategy.md)
- [Platform Strategy](../strategy/app-framework-platform-strategy.md)
- [Roadmap](../release/roadmap.md)
- XO active lane board and lane queue
- recent research, PR review, CI, tech-debt, SRA/CAB, product-proof, and
  downstream adoption evidence

The output should be short enough for the human and XO to use:

1. **Current strategic pull:** the most important direction the program should
   move next.
2. **Top priorities:** the few goals or themes that should dominate the next
   lanes.
3. **Stretch goals:** what must become true for App Framework to become the
   enterprise app fabric described in strategy docs.
4. **Stop/defer signals:** work that looks active but does not advance the
   strategy enough.
5. **XO direction:** board/queue guidance, owners to route, and decisions that
   need human or Product Owner attention.
6. **Freshness notes:** goals, assumptions, or docs that need to be challenged
   by research or product evidence.

## Throughput Operating Contract

The harness optimizes for trusted integration and human attention, not raw
agent activity. More code generation is useful only when the merge train and
review system can absorb it.

### Spec-Driven Change Contract

Use [Spec-Driven Change Harness](spec-driven-change-harness.md) to preserve
intent without slowing small work. The Product Owner, Architect, and XO choose
the lightest useful depth:

- tiny low-risk fixes use an intent note in handoff or PR summary;
- focused features, workflows, docs/CLI/skill behavior, or product slices use a
  lightweight spec when acceptance or scope could be misunderstood; and
- framework contracts, generated output, security/privacy, SaaS/provider
  integrations, CI/release gates, SRA/CAB, broad UX/product behavior, or
  multi-agent coordination use a durable spec with decision provenance.

The spec is not a permission slip. It is the shared target for implementation,
evidence, review, and human approval. If a lane changes the accepted intent, the
owner must update the spec or park the decision instead of silently widening
scope.

### Role Tiers

Use roles according to how they consume scarce capacity:

| Tier | Roles | Operating rule |
| --- | --- | --- |
| Standing core | Product Owner, XO, Architect, Integration Branch Manager; Strategist/Product Manager when active strategy is being shaped | Keep these durable because they hold priority, coordination, architecture, and merge order. |
| Producers | Architect when implementing, Coding Agents, focused lane owners | Scale these only up to the current WIP limit. Producers own branches and proof. |
| Advisory lenses | Research Steward, Structure Steward, Tech Debt Steward, Workstream Analyst, SRA/CAB Package Agent, Product/Nexus Proof Agent | Invoke for bounded briefs or packages. Do not staff them as standing threads unless a concrete durable package needs one. |
| Gates | Framework/Product PR Review, pre-push guard, CI, human approval, release/SRA/CAB authorities | Treat as workflows and authority checks, not implementation headcount. |

### XO WIP Governor

Before starting or assigning new producer lanes, XO sets and communicates a WIP
limit based on:

- integration capacity: how many branches the Integration Branch Manager can
  merge, conflict-resolve, and evidence without stale-branch churn;
- human-review capacity: how many sensitive or broad decisions the human can
  actually inspect;
- freeze safety: whether shared contracts, generated boundaries, branch base,
  and CI/release seams are stable enough for fan-out; and
- CI capacity: whether the current pipeline queue and slow gates can absorb
  more pushes without hiding failures behind long waits.

Default solo-workstation starting point: one architecture/design lane, one
integration/pipeline lane, and one or two focused producer lanes. Raise the WIP
limit only when the Integration Branch Manager can keep PR evidence current and
the human has no growing queue of sensitive decisions.

### Two-Speed Review Routing

Classify work early with `change-impact` when available and the class guidance
in
[`agentic-development-control-system.md`](../architecture/concerns/agentic-development-control-system.md).

- **Cheap path:** Class A/B, narrow, non-sensitive, lane-sized changes may use
  focused review, standing push approval, and no human touch before push when
  the PR Review Agent returns `GO` or `GO WITH CONDITIONS` with zero blockers
  and critical findings and conditions captured.
- **Human path:** Class C/D, integration trains, broad changes, governance or
  review-harness changes, auth/policy/tenant/token/security/release/provider
  readiness, governed-write, AI egress, PHI/PII, generated-template, SRA/CAB,
  or production-readiness surfaces require comprehensive review and an explicit
  human decision before merge. Standing push approval still does not authorize
  merge or risk acceptance.

Use the cheap path generously for truly narrow work so human attention stays
available for the path where judgment is irreplaceable.

### Cross-Model And Adversarial Review

When two capable model families are available, prefer independent cross-model
review for broad, sensitive, integration, or repeated-failure branches: one
model implements and another reviews with explicit instruction to refute the
change, find drift, and protect the human approval decision. Same-model review
is acceptable for narrow Class A/B work, but it should still be adversarial in
posture rather than a summary of the implementer's claims.

### Parked Decisions And Pull-Next Behavior

If a lane reaches a human-owned decision, external provisioning dependency, SRA
or CAB input, release-lite evidence need, architecture tradeoff, or accepted
risk question, the owner parks it with:

- exact decision needed;
- options and recommendation;
- evidence already gathered;
- owner who can unblock it; and
- impact if it waits.

XO then pulls the next unblocked lane within the WIP limit instead of letting a
producer idle or widen scope while waiting.

## Integration Manager Operating Flow

The Integration Branch Manager is the release-train conductor, not the default
repair crew for every failure.

It owns:

- branch dependency order, integration branches, conflict workflow, PR
  summaries, CI evidence, and merge-readiness recommendations;
- detecting, classifying, and preserving evidence for pipeline failures;
- fixing integration-mechanics issues when assigned, such as bad CI step
  sequencing, artifact-path contamination, stale retained artifacts, or
  evidence plumbing; and
- reporting exact human inputs needed for release-lite, SRA, CAB, or other
  approval gates.

It routes, rather than fixes:

- product behavior failures to Product Owner/Architect/coding lane;
- architecture or implementation test failures to Architect or the branch
  owner;
- missing approval fields or external evidence to XO/human authorities; and
- security, CAB, SRA, release, or accepted-risk decisions to the relevant
  human authority.

The Integration Branch Manager is not a permanent background daemon. Once
XO/human assigns a PR or pipeline, however, Integration owns live observation
without another prompt through terminal PR evidence and post-merge destination
evidence. An active Integration task or configured automation follows
[Remote Promotion State Synchronization](remote-promotion-state-synchronization.md):
observe every 5-10 minutes, emit only material transitions, refresh
current-state freshness without logging unchanged polls, and report stale
evidence as `STALE` or `UNKNOWN`. XO consumes each transition within one active
monitoring cadence and regenerates the affected board, queue, dependency holds,
and Product Dashboard projection.

Use integration branches consistently and selectively. A cohesive one-lane
Product Increment may declare `direct_main` and pay full candidate proof on its
sole PR. Create `integrate/<product-increment>` whenever multiple lanes or
independently versioned domains must be assembled, ordered, conflict-resolved,
and tested together.

## Structure Steward Triage Flow

Use this flow when the Architect, Product Owner, Strategist/Product Manager
function, or XO sees docs IA, skills, CLI, code organization,
generated-boundary, review-harness, or delivery entropy that could slow future
work.

1. **Architect invokes the Framework Structure Steward.** The Architect asks for
   a review-only maintenance brief when wave work, integration branches,
   docs/skills/CLI changes, repeated review findings, or repeated CI failures
   suggest structural drift. The Structure Steward detects and recommends; it
   does not start broad edits or own priority.
2. **Architect reviews the brief technically.** The Architect decides whether
   each finding is real, how it relates to current architecture or wave work,
   what dependency or blast radius it has, and whether it should be a focused
   lane, folded into current work, split for later, or rejected as noise.
3. **Architect parleys with the Product Owner, Strategist/Product Manager
   function, and XO when needed.** The Architect brings feasibility,
   sequencing, coupling, and proof cost. The Product Owner brings current
   backlog value, acceptance criteria, risk reduction, throughput,
   Nexus/product/platform relevance, and timing. The Strategist/Product Manager
   function brings market, portfolio, and strategic business-theme pressure.
   The XO owns the multi-thread operating system when coordination or
   escalation is the decision.
4. **The decision becomes explicit.** Record one outcome for each
   recommendation: add to immediate wave lane, create focused maintenance lane,
   add to [Tech Debt Register](tech-debt-register.md), defer with rationale, or
   reject as not worth platform investment.

The boundary is deliberate: **Structure Steward detects and recommends;
Architect evaluates feasibility and sequencing; Product Owner prioritizes
current backlog work; Strategist/Product Manager challenges strategic fit when
platform direction is at stake.**

## Decision Boundaries

Agents may:

- implement scoped changes;
- run checks;
- review branches;
- recommend priority;
- identify risk;
- propose maintenance work;
- prepare SRA/CAB/release evidence;
- say "do not merge this yet."

Agents may not independently:

- accept business risk;
- approve SRA, CAB, or release;
- merge broad or sensitive work without human approval;
- redefine the North Star;
- silently expand scope;
- bury tech debt inside `GO WITH CONDITIONS`.

Push-capable implementation and integration agents have standing approval to
push assigned branches only when the current required PR review final status is
`GO` or `GO WITH CONDITIONS`, blocker and critical counts are zero, conditions
are captured, and the branch has not changed since review. This standing
approval does not authorize merge, release, SRA, CAB, accepted risk, or direct
pushes to `main`.

## Default Flow

1. Strategist/Product Manager function clarifies strategic themes when needed.
2. Product Owner Agent clarifies priority, acceptance criteria, and wave intent.
3. XO assigns or confirms the WIP limit, branch owner, coordination path,
   parked-decision handling, and constraints.
4. Architect Agent designs lane-sized work and freezes shared seams before
   producer fan-out.
5. Coding Agents implement focused slices inside the current WIP limit.
6. Integration Branch Manager Agent coordinates merge order, PR evidence, and
   CI monitoring when assigned. Pushed merge-bound branches should not remain
   orphaned: either the branch owner creates/updates the PR when XO assigned
   that direct responsibility, or Integration Branch Manager owns PR
   creation/update coordination and the resulting PR pipeline evidence.
7. Architect invokes Framework Structure Steward when structural entropy is in
   scope, then reviews the brief and parleys with the Product Owner for
   prioritization, Strategist/Product Manager for strategic fit, and XO for
   coordination/escalation when needed.
8. Workstream Analyst is invoked when latency, CI duration, token/output burn,
   or work distribution becomes a throughput/risk concern.
9. PR Review Agent gives independent go/no-go review, using focused review for
   cheap-path changes and comprehensive adversarial review for human-path
   changes. This invocation uses standing authorization and does not make the
   XO goal human-blocked.
10. Human reviews evidence for reserved decisions and approves merge. Eligible
    branch pushes use the separate standing push authorization.
11. CI, release, SRA, and CAB authorities provide external approval where
   required.

## Preservation Rules

- Role contracts belong in this file, `AGENTS.md`, `agent_skills/`, and
  machine-readable CLI artifacts, not in chat memory.
- The default coordination topology belongs in this file and should be copied
  into thread handoffs when spawning durable adjacent threads.
- Role adherence belongs in task assignments, handoffs, review output, and the
  XO board; do not rely on thread memory to preserve boundaries.
- Slash commands are conveniences; every role workflow needs a tool-neutral
  skill, doc, or CLI fallback.
- Review output must help the human become a better reviewer, not merely defer
  responsibility to the agent.
- `GO WITH CONDITIONS` must create explicit follow-up, tech debt, or accepted
  risk. Use the [Tech Debt Register](tech-debt-register.md) when the condition
  becomes follow-up debt. It must not become silent entropy.
- Product Owner owns prioritization of steward recommendations against current
  business value, delivery throughput, risk, and roadmap commitments.
- Strategist/Product Manager function challenges whether those recommendations
  still align to strategic business themes and high-ROI value generation.
