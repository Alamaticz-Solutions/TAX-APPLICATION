# Program Work System And Glossary

This is the canonical work vocabulary for App Framework and Nexus planning,
delivery, dashboards, Jira integration, and human/agent coordination. Use these
meanings in new records, prompts, assignments, reports, and UI. Historical
identifiers remain searchable, but they do not create a second planning model.

## Governing Principle

Use an **outcome-based work breakdown structure** and keep other concerns as
linked dimensions. The North Star explains why, Strategy defines the strategic
approach, the WBS defines what, a Product Increment identifies the next
meaningful demonstration or decision, a Workstream identifies a capability
source, Delivery records how work moved, and Evidence proves what is true.

Do not force all of those concerns into one hierarchy.

## Canonical Model

The human-facing work breakdown structure is:

```text
Program
  -> Outcome
    -> Product Increment
      -> Deliverable
        -> Task
```

Each parent is completely explained by its children once its scope is
baselined. Children must not double-count work. Every Deliverable has exactly
one primary Product Increment for progress accounting; secondary relationships
are tags, not additional credit.

Existing schemas and identifiers remain valid during migration:

- **Outcome Goal** is the retained schema name for an **Outcome**.
- **Tranche** is the retained internal alias for a **Product Increment**.
- An existing **Milestone** that represents a coherent demonstration or
  decision is presented as a Product Increment. A milestone that is only a
  date, dependency, or evidence checkpoint remains a roadmap checkpoint and is
  not another WBS level.
- **Slice** is the retained internal alias for a **Deliverable**.

### Planned Work Data Contract

All new planned-work data follows the canonical hierarchy, even while retained
schemas continue to use compatibility field names:

1. Every Product Increment has exactly one parent Outcome.
2. Every Deliverable has exactly one primary Product Increment for progress
   accounting. Additional relationships are tags and earn no duplicate credit.
3. Every Task has exactly one parent Deliverable.
4. Workstream, Owner, Assignment, Roadmap Horizon, Stage, Delivery, and Evidence
   are linked dimensions. None becomes an extra WBS level.
5. Implemented and Accepted are separate evidence-backed states. Task activity,
   assignments, commits, pull requests, and demonstrations never create either
   state by implication.
6. Records in the retained `Milestones` column of the Outcome Goal Register are
   declared qualifying Product Increments. A date-only, dependency-only, or
   evidence-only checkpoint belongs in a separate Roadmap Checkpoint register
   and must not be added to that column.

The dashboard currently sources Outcomes, Product Increments, and Deliverables.
Task is part of the governing model, but Task records are not yet sourced. The
dashboard must state that gap and must not infer Tasks from Assignments.

### The Model To Remember

```text
Why:            North Star
Strategic how:  Strategy -> Strategic Goal
What:           Outcome -> Product Increment -> Deliverable -> Task
When:           Roadmap Horizon and Product Increment order
Capabilities:   Workstream
Accountability: Owner
Execution:      Assignment
Delivery how:   Delivery and Integration Train
Proof:          Evidence -> Implemented -> Accepted
```

Strategy contains the choices for pursuing the North Star. Strategic Goals
state the durable results those choices must achieve. Outcomes then turn those
Strategic Goals into measurable, owned changes that can be decomposed and
delivered.

### Outcome Clarity

An **Outcome** is a measurable change in a user's, business's, or operation's
state. It is not the software, activity, or capability being built. A clear
Outcome states:

1. who benefits or whose behavior changes;
2. what becomes meaningfully different;
3. how the change will be measured or demonstrated; and
4. which accountable authority can accept it.

Use this sentence shape when an Outcome is unclear:

> For **[person or operation]**, **[current condition]** becomes **[better
> condition]**, demonstrated by **[measure or evidence]** and accepted by
> **[accountable owner]**.

Example: "For an authorized workforce operations user, adding a provider
location becomes one guided Nexus journey with visible status through verified
completion, demonstrated against representative web and mobile scenarios and
accepted by the Nexus Product Owner."

The complete program is understood through six linked views:

| View | Lineage | Question answered |
| --- | --- | --- |
| Purpose | North Star | Why are we investing? |
| Strategic direction | Strategy -> Strategic Goal -> Outcome | How will we pursue the North Star, and what measurable change comes next? |
| Roadmap | Roadmap Horizon -> Outcome -> Product Increment | In what dependency order will outcomes mature? |
| Work breakdown | Outcome -> Product Increment -> Deliverable -> Task | What product work remains? |
| Product flow | Product -> Journey -> Product Increment -> Deliverable | What coherent user-visible result are we proving now? |
| Proof | Deliverable -> Assignment -> Delivery -> Evidence -> Acceptance | Who is acting, what changed, and what is proven? |

Workstream and Capability are cross-cutting classifications. A Workstream is
not a person or team in the WBS; it names the expertise and system-ownership
domain needed by the work. Workstreams are never parents in the WBS and never
receive automatic percentage-complete claims.

## Direction And Roadmap Terms

| Term | Standard meaning | Not this |
| --- | --- | --- |
| **Program** | The complete App Framework investment: direction, products, platform capabilities, roadmap, delivery, evidence, and learning. | A repository, branch, release, or one product. |
| **North Star** | The durable purpose and desired future state of the Program. | A current plan or slogan for a delivery cycle. |
| **Strategy** | The durable choices, boundaries, positioning, and economic rules used to pursue the North Star. | A list of work, dates, or implementation techniques. |
| **Strategic Goal** | A durable, measurable result that advances the North Star. | A temporary project, theme, or feature. |
| **Roadmap** | The dependency-aware sequence of Outcomes and Product Increments. | A flat backlog, branch history, or workstream list. |
| **Roadmap Horizon** | An ordered decision and dependency band. It may be current, next, or later and is not a date promise unless funded and dated. | A sprint, lifecycle stage, wave, or release. |
| **Outcome** | A measurable change in a user's, business's, or operation's state, with accountable ownership, success measures, exit evidence, and a stop rule. `Outcome Goal` remains its schema alias. | A feature, capability name, activity target, technical approach, or standing team. |
| **Product Increment** | The ordered bundle of Deliverables that creates one meaningful demonstration, learning event, or acceptance decision. One or many Product Increments produce an Outcome. `Tranche` remains its internal alias. | A permanent team, arbitrary time box, technical layer, or collection of unrelated work. |
| **Milestone** | An optional roadmap checkpoint used only when it adds information beyond the Product Increment, such as an external dependency or evidence decision. | A required WBS level, percentage, meeting, commit, or pipeline run. |

## Work Breakdown Terms

| Term | Standard meaning | Required properties |
| --- | --- | --- |
| **Deliverable** | The smallest bounded behavior or enabling result that one accountable owner can implement, test, review, and demonstrate independently. One or many Deliverables produce a Product Increment. `Slice` remains its internal alias. | Plain-language behavior, primary Product Increment, owner, acceptance criteria, stage, dependencies, bounded scope, non-claims, and required evidence. |
| **Task** | One atomic human or agent action needed to complete a Deliverable. One or many Tasks produce a Deliverable. Tasks are execution detail and do not reduce product burndown by themselves. | One owner, expected result, bounded tools/write scope when applicable, and a completion check. |
| **Defect** | A verified failure of accepted or intended behavior. It becomes a Deliverable when it requires independently planned outcome work; otherwise it is a Task within the affected Deliverable. | Reproduction, expected behavior, impact, and regression proof. |
| **Debt Item** | A consciously retained quality, architecture, security, operability, or maintainability liability with consequence and retirement trigger. It becomes planned work only through a Deliverable. | Impact, evidence, owner, trigger, target, and current disposition. |

### Deliverable Quality Rule

A Deliverable is small enough when one accountable owner can carry it from bounded
intent to immutable reviewed evidence without crossing an unstable ownership
boundary. Split it when independent parts can be proven separately, require
different owners, or can proceed in parallel with a frozen contract. Do not
split merely to create more agent activity.

Use these decision tests when the levels feel ambiguous:

- **Outcome:** can the accountable owner determine whether a meaningful user,
  business, or operational condition changed?
- **Product Increment:** can stakeholders make a meaningful continue, change,
  or accept decision from the combined demonstration?
- **Deliverable:** can an independent reviewer prove this bounded behavior or
  enabling result exists?
- **Task:** can one human or agent perform this single action and verify its
  expected result?

## Product Flow Terms

| Term | Standard meaning | Not this |
| --- | --- | --- |
| **Product** | A bounded vehicle that delivers value to known users and stakeholders. | The whole Program or a component package. |
| **Product Outcome** | A measurable change in user or business performance. | A screen, output, or shipped component. |
| **Journey** | A bounded end-to-end progression across states, systems, time, roles, and handoffs. | A portal, screen flow, chat, or roadmap level. |
| **Experience** | One role's participation in a Journey moment through a channel and context. | A theme or component. |
| **Product Increment** | A deliberately ordered, demonstrable vertical result that groups the smallest set of Deliverables needed for one coherent learning or acceptance decision. It may draw from several Workstreams. Existing `Tranche` identifiers remain valid aliases. | A permanent team, technical layer, arbitrary time box, or second competing hierarchy. |
| **Feature** | A user-recognizable product behavior or ability. It is a catalog and reporting concept that may require several Deliverables. | A mandatory WBS level or synonym for Task. |
| **Capability** | A reusable product or platform ability whose maturity may support several Products, Features, and Outcomes. | A roadmap parent or proof of adoption by itself. |

The Product Experience Model remains:

```text
Product Outcome
  -> Journey
    -> Experience
      -> Workspace
        -> Work Module
          -> Attention Item
```

It describes the product, not the delivery plan.

## Ownership And Coordination Terms

| Term | Standard meaning | Planning treatment |
| --- | --- | --- |
| **Workstream** | A durable capability and system-ownership domain, such as experience, intelligence, integration, mobile, or operations. It answers "which expertise is needed?", not "who is doing it?" | Attribute on Deliverables and Tasks. Never a person, standing queue that must stay busy, or percentage-complete parent. |
| **Owner** | The one accountable human or role for the work item's result and handoff. It answers "who is accountable?" | Required on active Outcomes, Product Increments, Deliverables, and Tasks. |
| **Assignment** | A time-bounded routing of one Task or Deliverable to one human or agent, with authority, stage, scope, dependencies, lease, and review route. It answers "who is doing it now?" | Execution record, not roadmap work. |
| **Delivery Lane** | A bounded execution grouping for one or more independently provable Deliverables inside one Product Increment. It has one owner, branch, write surface, focused proof loop, and integration destination. It enables parallel flow but is not another WBS level. `Lane` is acceptable shorthand in operational views. | Required execution dimension for source-active Product Increments. Never a permanent team, Workstream, roadmap parent, or source of duplicate progress credit. |
| **Integration Train** | The ordered process that consumes independently reviewed Deliveries into an integration branch or accepted main. | Flow mechanism, not a Product Increment, Workstream, roadmap item, or value claim. |
| **Wave** | A historical or temporary coordination batch. | Do not use for new roadmap structure or maturity. |
| **Gate** | A condition that must be satisfied before a claim or transition. | Not a work item. Work needed to satisfy a Gate is represented by a Deliverable or Task. |
| **Stage** | The rigor level applied to a Deliverable or Delivery: Prototype, Integration Candidate, or Release Candidate. | Not progress, maturity, or roadmap order. |

### Product Increment Execution Contract

Use
[Product Increment Delivery Model](../start/product-increment-delivery-model.md)
to execute the WBS. Every new Deliverable names one primary Product Increment.
Register every major batch in the canonical Product Increment portfolio. A
source-active Product Increment uses one to four independently admissible
Delivery Lanes. The portfolio maintains two to four active lanes overall.
Multi-lane or cross-domain work uses one short-lived integration branch; an
independent one-lane increment may declare `direct_main`. Workstreams supply
capability expertise to lanes; they do not become standing queues.

Cross-lane capability flow is explicit:

- `requires` records a blocking capability, required provider state, frozen
  seam or fixture, convergence action, and integration proof;
- `benefits_from` records reusable payoff without blocking independent
  progress; and
- overlapping write roots or unresolved hard dependencies prevent concurrent
  admission.

Use the machine-readable portfolio, plan, and validators so current status,
branch shape, concurrency, dependencies, benefits, merge order, and proof
boundaries are executable
rather than conversational.

## Delivery And Proof Terms

| Term | Standard meaning | Not this |
| --- | --- | --- |
| **Delivery** | The implementation vehicle or event for a Deliverable: commit, branch, pull request, build, deployment, migration, or equivalent. | The business outcome or acceptance itself. |
| **Evidence** | Source-bound, reproducible proof supporting a claim, transition, or decision. | Generated prose, activity, or an unverified screenshot. |
| **Demonstration** | A review event showing behavior in a representative context. | Acceptance unless the authorized Outcome Owner accepts it against criteria. |
| **Implemented** | The Deliverable's intended behavior exists at an immutable version, has the required stage-appropriate proof and independent implementation review, its candidate plus evidence are retained by accepted main, and trusted authority and metering adapters verify the reviewer, destination SHA, approval provenance, and usage. It may still await Product acceptance. | A local file, local remote-tracking ref, self-authored receipt, leaf checkpoint, open PR, deployment, adoption, or acceptance by implication. |
| **Accepted** | The authorized Outcome Owner confirms that current evidence satisfies the stated acceptance criteria in a distinct acceptance receipt retained by accepted main and verified by the trusted authority adapter. Only this state reduces accepted burndown. | Agent confidence, review approval, merge, CI success, or demonstration alone. |
| **Released** | An accepted version has been promoted through its required release controls to the named environment or audience. | Implemented or Accepted by implication. |

## Status Model

Keep scheduling, work progress, and delivery mechanics separate.

### Planning Posture

Use for Outcomes and Product Increments:

- Active
- Next
- Awaiting input
- Held
- Later

### Work Progress

Use for Deliverables and Tasks:

- Not started
- Ready to start
- In progress
- Waiting for a decision
- Waiting for another team
- Under review
- Implemented
- Accepted
- Deferred
- Stopped, with one plain-language reason

Do not use **Done** or **Complete** as a stored state. They conceal the
difference between implementation, acceptance, release, and adoption.

### Delivery State

Use repository and deployment-native states such as Draft, Open, Under review,
Merged, Deployed, Failed, or Closed only on the Delivery record. A merged pull
request does not automatically change the Deliverable to Accepted.

## Agentic Work Contract

Every admitted Deliverable has no more than four controlling records:

1. Product outcome and acceptance criteria.
2. Technical contract and bounded scope for the Deliverable.
3. Assignment with owner, reviewer, authority, stage, lease, branch, and tool,
   data, network, and write boundaries.
4. Evidence and review record with findings and disposition.

An agent Task must also state:

- the parent Deliverable and one expected result;
- inputs and authoritative sources;
- explicit non-goals and stop conditions;
- allowed write roots and external systems;
- the smallest relevant checks;
- the required handoff and independent-review route.

Use one agent for linear work. Parallelize only disjoint Tasks or Deliverables with a frozen
contract and independent write surfaces. Use an evaluator/reviewer that did not
author the work when the stage requires independent review.

## Burndown And Velocity

Burndown uses versioned acceptance units owned by an Outcome or Product Increment. The
scope baseline must identify every unit exactly once.

Show two remaining-scope series:

- **Implemented remaining** = baseline units not yet Implemented or Accepted.
- **Accepted remaining** = baseline units not yet Accepted.

The space between them is the **acceptance gap**. Scope added, removed,
deferred, or split after baseline is a visible scope-change event; history is
not rewritten.

Measure product flow with:

- Deliverables Implemented and Accepted per period;
- median and percentile time from Ready to Implemented and Implemented to
  Accepted;
- active WIP, blocked age, review age, acceptance gap, and rework;
- Outcome or Journey measures such as task success, cycle time, adoption,
  quality, and cost.

Measure software delivery separately with DORA throughput and instability
metrics. Do not treat commits, lines of code, prompts, tokens, agent sessions,
branches, or Tasks completed as product velocity.

## Jira Mapping

Jira is an execution-system adapter, not the owner of Program semantics.
Preserve the canonical ID and type on every Jira work item.

| Canonical concept | Preferred Jira representation |
| --- | --- |
| Outcome | Initiative or custom parent above Epic; otherwise a required linked field |
| Product Increment | Epic, release, or a governed Product Increment field, depending on Jira configuration |
| Deliverable | Story or standard work item |
| Task | Task or Subtask, according to whether it needs independent assignment |
| Workstream | Component or governed Workstream field |
| Feature | Product catalog link or governed Feature field; not a required hierarchy level |
| Evidence | Linked evidence record, build, deployment, test, review, or artifact |

If Jira configuration cannot represent Outcome above Epic, do not collapse the
Outcome and Product Increment. Store the Outcome as a required stable field and
keep Epic mapped to Product Increment.

## Traditional Terms

The traditional **Epic -> Feature -> Story -> Task** chain is familiar but is
not the canonical Program WBS because Epic and Feature are interpreted
differently across tools and teams.

- **Epic** is a Jira-facing alias for Product Increment in the preferred mapping.
- **Feature** remains a user-facing behavior and reporting lens.
- **Story** is a Jira-facing alias for Deliverable when it satisfies the Deliverable
  contract.
- **Task** is canonical execution detail.
- **Epoch**, if intentionally used, means a named time period for historical
  analysis. It is never a work item. Do not use it as a synonym for Epic.

## Naming And Transition Rules

- Keep existing stable identifiers such as `AF-OGxx`, `AF-Mxx.x`, `AFS-xxx`,
  `NXP-*`, and `WS-xx`; do not renumber merely to adopt this glossary.
- Retain A0, A1, and A2 as current Nexus Product Increment identifiers; show
  `Tranche` only as a secondary compatibility alias.
- Present existing `AF-OGxx` records as Outcomes, qualifying `AF-Mxx.x`
  records as Product Increments, and `AFS-xxx` records as Deliverables. Do not
  change their stable IDs merely to improve human terminology.
- New work must name one primary Outcome, Product Increment, Deliverable,
  owner, Workstream, stage, and acceptance route.
- Historical Wave, Lane, packet, gate, branch, and commit identifiers remain
  searchable evidence aliases. They do not stand alone in current status.
- Dashboards show plain titles first and technical IDs in secondary text or
  drill-down.

## Basis

This system applies established principles without importing an entire process
framework:

- PMI's deliverable-oriented WBS and 100 percent rule;
- the Scrum Guide's Product Goal and usable Increment focus;
- Jira's configurable parent hierarchy, treated as an adapter;
- current agent-workflow guidance favoring bounded context, simple sequential
  work, selective parallelization, and independent evaluation;
- DORA's separation of throughput and instability and its preference for small
  changes.

References:

- [PMI: Creating Effective Work Breakdown Structures](https://www.pmi.org/learning/library/creating-effective-wbs-recognize-quality-7541)
- [The Scrum Guide](https://scrumguides.org/docs/scrumguide/v2020/2020-Scrum-Guide-US.pdf)
- [Atlassian: Configure the Work Type Hierarchy](https://support.atlassian.com/jira-cloud-administration/docs/configure-the-issue-type-hierarchy/)
- [Anthropic: Building Effective AI Agents](https://resources.anthropic.com/building-effective-ai-agents)
- [DORA Software Delivery Performance Metrics](https://dora.dev/guides/dora-metrics/)
