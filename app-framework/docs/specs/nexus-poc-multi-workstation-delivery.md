# Nexus PoC Multi-Workstation Delivery

Status: human-ratified-for-controlled-transition; pending integration into
`main`

Spec depth: full

Owner roles:

- Program Product Manager: owns program priority, product outcomes, and final
  product-acceptance recommendations.
- Program Flow Controller: owns admission, the producer WIP limit, the
  assignment queue, dependency flow, and parked-decision routing. It has no
  product, architecture, source, merge, release, or risk authority.
- App Framework Chief Architect: owns frozen cross-workstream contracts,
  shared seams, and write boundaries.
- Integration Branch Manager: owns the integration branch, conflict handling,
  PR evidence, worktree disposition, and accepted-main freshness.
- Workstream Lead / Outcome Owner: owns one bounded outcome, local priority,
  acceptance interpretation, dependencies, and escalation.
- Workstream Technical Lead: owns workstream design, the implementation plan,
  contract compliance, and focused proof.
- Branch Owner / Implementation Agent: is the sole writer for one declared
  branch and one active worktree.
- Independent Reviewer: reviews the branch without source authorship.

The Program Flow Controller supersedes the legacy XO role. Legacy XO artifacts
are historical inputs only; they do not grant admission or dispatch authority.
Program-level roles are not duplicated inside each workstream.

## Business Value

Several computers can advance Nexus concurrently without multiplying stale
branches, repeated context loading, overlapping source edits, or late
integration failures. Hardware is used to increase accepted throughput, not to
increase work in progress.

## PoC Product Boundary

The full Nexus PoC remains one coherent My Requests / My Tasks experience for
status, approvals, notifications, and recovery. Its first end-to-end journey
includes ServiceNow, Workday, and PDS Health AI. It must prove both responsive
web and native mobile surfaces, with shared task, evidence, permission, status,
approval, deep-link, interruption, and resume semantics.

Initial provider behavior is primarily read, status, and deep link, with only
explicitly selected governed actions. The target context is approximately
17,000 potential team-member users across roughly 1,100 locations, with
enterprise production-support expectations and healthcare-grade identity,
authorization, privacy, audit, observability, resilience, and recovery
controls.

A Workday-first thin vertical slice may be used to prove package, fixture,
identity, journey, and integration mechanics. That slice is sequencing
evidence only. It does not remove ServiceNow, PDS Health AI, native mobile,
aggregate trust and operations, or production-support evidence from full PoC
product acceptance.

## Critical Decision

Keep App Framework as a modular monorepo through the Nexus PoC. Use package
boundaries, fixture contracts, explicit write roots, and an integration
authority to create isolation. Do not split repositories merely to make several
workstations possible.

Repository extraction is justified only after a boundary has an independent
release cadence, versioned compatibility contract, consumer proof, and owner.
Before those conditions exist, a split adds release coordination and contract
drift while removing the atomic validation that Nexus still needs.

## Product Increment And Delivery Lane Pull

The eight Workstreams remain capability and system-ownership domains. They are
not standing queues, teams that must stay busy, or the primary progress
hierarchy. New work is pulled through the
[Product Increment Delivery Model](../start/product-increment-delivery-model.md).

The active Nexus plan is
[`nexus-add-provider.product-increment.json`](nexus-add-provider.product-increment.json).
Its plain-language status and cross-increment leverage are registered once in
[`product-increment-portfolio.json`](product-increment-portfolio.json).
It uses one Product Increment integration branch and four independently
admissible Delivery Lanes:

1. Nexus web and native experience;
2. ServiceNow case projection;
3. Workday governed operation; and
4. grounded Gemma assistance.

Each lane draws from the required Workstreams. The Program Flow Controller
admits two to four lanes only when their write roots are disjoint and every
required cross-lane capability is already sufficient or decoupled by a frozen
contract, fixture, or adapter. Work completed in one lane is made visible to
other lanes through typed `requires` and `benefits_from` links, including the
required provider state, adoption trigger, convergence action, and integration
check.

The legacy `WS-01` through `WS-08` priority list remains a compatibility hint
for selecting expertise. It no longer creates eight active backlogs or
authorizes work independently of a Product Increment plan.

`WS-08` security, identity, audit, privacy, correlation, telemetry, resilience,
and recovery obligations apply across lanes from the first commit. They are
not deferred merely because Trust and Operations is not a dedicated lane.

Extra computers do not justify extra WIP. Raise concurrency only within the
two-to-four lane limit and only when integration evidence shows no expanding
review queue, repeated source collision, CI congestion, or human acceptance
backlog.

## Fresh-Agent Cutover

Use a controlled cutover rather than inheriting the legacy hierarchy:

1. Pause legacy XO and strategist heartbeats before admitting new work.
2. Ask legacy Product Owner, Architect, Integration, XO, and Strategist tasks
   for immutable handoffs mapped to `WS-01` through `WS-08`.
3. Preserve active implementation at its current bounded checkpoint; permit no
   successor dispatch from the legacy topology.
4. Start fresh Program Product Manager, App Framework Chief Architect, Program
   Flow Controller, and Integration Branch Manager tasks from this tracked
   contract. These control-plane tasks are read-only by default.
5. Start fresh Workstream Lead / Outcome Owner and Technical Lead tasks only
   for admitted workstreams. Assign one sole branch writer separately.
6. Treat old task memory, lane boards, and `target/appfw` files as evidence to
   reconcile, never as current authority.
7. Archive legacy coordinating tasks only after their handoffs are ingested and
   every active branch, decision, and dirty worktree has a named disposition.

## Sources Of Truth

The generic execution contract is
[`product-increment-delivery-model.md`](../start/product-increment-delivery-model.md).
The canonical human-readable status index is
[`product-increment-portfolio.json`](product-increment-portfolio.json).
The active Nexus Product Increment plan is
[`nexus-add-provider.product-increment.json`](nexus-add-provider.product-increment.json).

The tracked topology is
[`nexus-poc-workstream-topology.json`](nexus-poc-workstream-topology.json).
It owns stable Workstream capability, ownership, context, and proof rules. It
does not create a second Product Increment plan or standing Workstream queue.

When AFS-007 is accepted on `main`, `program/registry/` owns dynamic slice and
coordination records. This spec must bind to those records rather than create a
second live registry. Branches and PRs own source history. `target/appfw/`,
dashboard models, local assignment files, and worktree audits are projections
or local evidence and never grant acceptance authority. The dashboard projects
the portfolio's status, next action, `requires`, and `benefits_from`; it does
not maintain a second narrative.

## Dashboard Projection Completion Invariant

The Program Flow Controller is the sole writer and projector of
`target/appfw/nexus-control-dashboard/status.json`. Every other role sends
verified, attributable facts to the Program Flow Controller and must not edit,
overwrite, regenerate, or independently project that file. Projection records
state from the proper authorities; it grants no Product, Architecture, source,
integration, merge, release, production, security-exception, or risk authority.

Every Program Flow Controller turn that observes, receives, decides, or routes
a material state change is incomplete until the dashboard is refreshed and
validated in the same control turn. Material changes include:

- producer WIP count, limit, slot identity, admission, hold, handoff,
  completion, or release from WIP;
- control or workstream task identity, task ID, role, owner, reviewer,
  assignment, lease, workstation, branch, worktree, or activity state;
- dependency or activation gates, blockers, stops, escalations, required
  inputs, and eligibility;
- relevant Product, Architecture, human, security, release, or risk decisions,
  including non-acceptance and supersession;
- checkpoint SHAs, packet or review hashes, review results, severities and
  conditions, proof or handoff freshness, push eligibility, and no-consume
  dispositions;
- Integration branches and bases, collision and reconciliation state, PRs,
  pipelines, promotion, accepted-main SHA, and remote-train state;
- worktree ownership, cleanliness, locks, protection, reclamation,
  preservation checkpoints, cleanup eligibility, and final disposition; and
- dashboard authority, schema or contract version, source-of-truth location,
  and projection health.

Non-material commentary with no state change does not require a rewrite. When
the Program Flow Controller is uncertain whether a fact changes human-visible
state, it treats the fact as material or records why no projection field
changes.

For each refresh, the Program Flow Controller assembles the complete candidate
in a task-specific temporary file under the dashboard or evidence root. It
parses and validates the candidate before atomically replacing `status.json`,
then re-reads and revalidates the exact on-disk bytes. A failed candidate must
not truncate or corrupt the last valid projection. The projection records
RFC3339 observation and update times, its schema and projection-contract
version, the projector identity and task ID, and attributable source facts for
every material transition. Source facts identify the authority and type plus
the applicable task ID, assignment, artifact, packet or review hash, commit or
ref, PR, or pipeline. Conversation summaries alone are not authority evidence.

Before completing a material control turn, the Program Flow Controller
validates:

1. JSON syntax and the declared dashboard schema and projection contract;
2. task IDs and role titles, including `WS-01` through `WS-08` for
   workstream-bound tasks and unnumbered names for cross-workstream roles;
3. assignment identity, branch and bases, Integration branch, owner/reviewer
   independence, lease, and active, blocked, or handoff status;
4. WIP arithmetic, including unique active producers, an exact displayed
   numerator at or below the tracked limit, and named assignments for occupied
   slots without counting held, inactive, review-only, or control roles;
5. attributable and internally consistent SHAs, hashes, refs, PRs, pipelines,
   proof, handoff, and review evidence;
6. separation of accepted-main and Integration freshness from Product,
   package, provider, release, or risk acceptance;
7. worktree status and disposition against the latest protected inventory and
   Integration decision;
8. timestamps and source facts for all material fields, without inheriting
   stale legacy XO, queue, or dashboard authority; and
9. contradictions and staleness across the candidate and prior projection.

The contradiction scan rejects or explicitly historicizes, with timestamps
and supersession links, states such as active and inactive simultaneously;
admitted while an admission prerequisite is unmet; `GO` and `NO-GO` for the
same SHA; consumed while `STOP` or `NO-CONSUME`; green and failed for one
pipeline; conflicting accepted-main SHAs; duplicate writers or worktrees; WIP
that disagrees with active producers; resolved blockers presented as current;
or superseded packets and hashes used for admission. The Program Flow
Controller never silently chooses between conflicting authority facts. It
routes the conflict and marks projection health stale or blocked until the
conflict is resolved.

If candidate validation, atomic replacement, or on-disk revalidation is
genuinely unavailable, the control turn fails visibly. The Program Flow
Controller reports dashboard health as `STALE` or `UNAVAILABLE` to the human
and affected roles, names the failed validation or write and the last known
observation, and must not silently defer the refresh or describe the projection
as current.

Every future Program Flow Controller bootstrap prompt is incomplete unless it
loads this accepted delivery spec and the tracked topology, names the Program
Flow Controller as sole `status.json` writer, requires this same-turn material
change invariant, atomic validation, contradiction scan, and visible failure
posture, tells all other roles to send verified facts without writing the
projection, and requires the first control turn to inspect projection schema,
freshness, health, and source facts before treating dashboard state as current.
Prompts and handoffs may link to this section; they may not omit, weaken, or
conversationally waive it.

## Workstation Contract

Every producer workstation must have a local assignment that validates against
[`workstation-assignment.schema.json`](../start/workstation-assignment.schema.json).
The assignment names one lane outcome owner, technical lead, sole branch owner,
independent reviewer, admission authority, purpose-named branch, accepted base
SHA, integration owner and branch, and review deadline. The assignment base
must equal the selected lane's immutable `base_sha`. Assignment schema `@4` is
required for admission. The selected Delivery Lane is the authoritative scope;
Workstreams remain optional capability-domain context. Local validation can
report `source_admission_ready:true`, but actual write authority requires the
Program Flow Controller to acquire and hold a shared live exclusive lease for
the assignment, branch, and worktree. A static or copied assignment never emits
write roots by itself. Legacy `@3` assignments remain read-only context;
earlier assignments are not admissible because they cannot safely establish
portfolio-wide exclusivity.

Use:

```bash
node scripts/check-product-increment-plan.mjs \
  --plan docs/specs/nexus-add-provider.product-increment.json \
  --json
node scripts/check-product-increment-plan.mjs \
  --plan docs/specs/nexus-add-provider.product-increment.json \
  --current-diff <delivery-lane-id> \
  --json
node scripts/check-nexus-workstreams.mjs --json
node scripts/check-nexus-workstreams.mjs --assignment target/appfw/workstation-assignment.json --live --json
node scripts/check-nexus-workstreams.mjs \
  --assignment target/appfw/workstation-assignment.json \
  --context WS-07 \
  --live \
  --json
```

The context command emits links and contracts, not copied instruction bodies.
Each workstream is limited to eight required sources so a worker can load the
smallest sufficient context. Load optional references only when the current
decision requires them.

## Worktree Lifecycle

The controlled cleanup on 2026-07-18 removed 14 verified merged or superseded
clean worktrees and reclaimed about 59 GiB without deleting branches or
commits. Thirty linked worktrees remained: active or owner-review work, dirty
protected state, two stale dirty Claude worktrees, and two merged checkouts
retained because live Architect and implementation tasks still used them. The
machine audit is an inventory signal; live ownership can override a syntactic
reclamation candidate.

Apply these rules immediately:

- Freeze unmanaged producer-worktree creation until every active producer has
  an assignment and owner.
- Keep one active source-producing worktree per assignment. A reviewer may use
  a detached read-only checkout, but it must not become a hidden producer.
- Create branches from a fetched `origin/main` or the named current integration
  branch. Record the exact base SHA.
- Review any assignment or clean unmerged worktree with no update for 72 hours.
  Staleness triggers owner review, not automatic deletion.
- Remove a clean, merged worktree within one business day after its owner and
  Integration Branch Manager confirm that retained evidence is no longer
  needed.
- Never automatically remove a dirty, locked, detached-unmerged, unpushed, or
  owner-unknown worktree.
- Do not use another workstation's uncommitted checkout or `target/appfw`
  directory as a dependency. Exchange versioned packages, committed fixtures,
  schemas, or accepted commits.

The read-only audit command is:

```bash
node scripts/audit-worktrees.mjs --json
```

It reports candidates and protected state. It never prunes or removes a
worktree.

### Immediate Transition From The Current Worktrees

Do not add the three producer assignments on top of the present worktree set
without adjudication. Use this order:

1. Freeze new producer worktrees. Existing workers may preserve or finish an
   assigned checkpoint, but they do not fan out again.
2. Have the Integration Branch Manager identify an owner and PR/disposition for
   every clean unmerged worktree, especially the overlapping PDS branches.
3. Preserve active-task overrides for the two merged checkouts still used by
   the legacy Architect and implementation tasks; the fresh Integration Branch
   Manager owns their eventual disposition.
4. Protect all pre-existing dirty or dirty-locked worktrees. Convert their
   state into commits, patches, or an explicit supersession decision before
   considering cleanup.
5. Keep the published supersession list with the cleanup evidence. A shared
   HEAD does not prove that branches or uncommitted state are equivalent.
6. Classify retained valuable work into a Product Increment plan or retire it;
   do not recreate standing Workstream trains.
7. Record each selected Product Increment base and integration strategy, then
   admit only the independent Delivery Lanes that fit the portfolio WIP limit.

## Workstation Activation Sequence

For each new computer:

1. Clone or fetch the repository and prove `origin/main` is current.
2. Create one purpose-named producer branch from the assigned full base SHA.
3. Create `target/appfw/workstation-assignment.json` from the tracked example;
   use a review deadline no more than seven days away and normally 72 hours.
4. Run the Product Increment, topology, live assignment, and lane-scoped
   context checks.
5. Read only the emitted required sources first. Pull optional documentation
   when a concrete decision needs it.
6. Implement only inside the emitted Delivery Lane write roots. Route
   integration-only and shared-seam proposals to the Integration Branch
   Manager.
7. Run focused inner-loop proof, then handoff checks and independent review.
8. Push the purpose branch and create or hand off the PR according to the branch
   integration model.
9. After accepted integration, mark the dynamic coordination record complete
   and owner-review the local worktree for cleanup.

## Integration Contract

Cross-workstream behavior moves through versioned artifacts:

- PDS Experience publishes an installable package, tokens, and catalog proof.
- AI, Workday, and ServiceNow publish provider packages, deterministic
  fixtures, schemas, and certification evidence.
- Mobile consumes shared semantics and native token projections, not web
  components.
- Nexus Web consumes installable PDS packages and provider fixtures, not
  adjacent source aliases or another worktree.
- Product Evidence consumes built products and retained evidence without
  owning framework implementation.
- Trust and Operations validates the aggregate system and owns operational
  evidence without silently rewriting product behavior.

Root manifests, lockfiles, central CLI dispatch, CI, the program registry,
shared runtime contracts, shared SaaS contracts, generated templates, the
Nexus entity model, and the roadmap are integration-owned seams. A producer
that needs one changed submits a small contract proposal or prerequisite branch
to the Integration Branch Manager. It does not widen its own write set.

## Cost And Throughput Controls

- Load the generated workstream context pack, then only the linked sources
  needed for the decision. Do not preload the whole docs tree.
- Reuse committed research, fixtures, schemas, and accepted decisions. Do not
  pay several workstations to rediscover the same contract.
- Run focused package checks in the inner loop. Run broad docs, generation,
  integration, and release checks at handoff or on the integration branch.
- Keep implementation and independent review separate. A spare workstation is
  often worth more as a reviewer than as a fourth overlapping producer.
- Rebase or refresh at explicit checkpoints, not continuously. Surprise pushes
  and repeated full-suite polling consume CI and model tokens without improving
  accepted throughput.
- Measure accepted lead time, conflict/rework rate, review queue age, CI queue
  time, and context sources loaded. Commit count and concurrent agent count are
  not outcome measures.

## Stop Conditions

Pause new producer assignment when any of these is true:

- two active producers need the same write root;
- a shared contract is changing without one named owner and freeze SHA;
- the Integration Branch Manager has more than two merge-ready branches waiting;
- human-sensitive decisions are accumulating faster than they are resolved;
- CI failures cannot be attributed to one source branch;
- a workstation is based on an unaccepted or materially stale contract; or
- a product is consuming another worktree's source or local build output.

## Acceptance Evidence

| Criterion | Proof |
| --- | --- |
| Priority and ownership are deterministic | `node scripts/check-nexus-workstreams.mjs --json` |
| A workstation has one valid assignment and current base | assignment check with `--live --json` |
| Agent context is bounded and source-linked | `--context <WS-ID> --json` |
| Worktree cleanup is reviewable and non-destructive | `node scripts/audit-worktrees.mjs --json` |
| Producer changes remain inside their write roots | changed-path comparison during handoff and integration review |
| Cross-workstream dependencies are artifacts, not local source | package/fixture/schema references in handoff evidence |
| Full system remains coherent | integration-branch checks and independent comprehensive review |

## Decision Provenance

| Date | Owner | Decision | Rationale | Revisit Trigger |
| --- | --- | --- | --- | --- |
| 2026-07-18 | Product Owner | Re-index the confirmed priority sequence as `WS-01` through `WS-08`. | PDS experience and intelligence establish the differentiating foundation before the Nexus shell; Workday is the first bounded journey integration. Ascending identifiers remove needless translation between identity and priority. | Product priority or PoC journey changes. |
| 2026-07-18 | Architect recommendation | Keep the modular monorepo and impose workstream/package boundaries. | Current contracts and release maturity do not justify multi-repo coordination cost. | A package has independent release, compatibility, consumer, and ownership proof. |
| 2026-07-18 | Program Flow Controller recommendation | Start with three producers plus one integration authority. | Integration and human-review capacity, not computer count, governs accepted throughput. | Two consecutive integration cycles land without conflict/review/CI backlog. |
| 2026-07-18 | Structure Steward recommendation | Freeze unmanaged worktree creation and audit before cleanup. | The worktree inventory included dirty and owner-ambiguous state; automatic cleanup could destroy human work. | All active worktrees have owner, assignment, branch purpose, and disposition. |
| 2026-07-18 | Human sponsor | Replace the legacy XO hierarchy with the Revised Multi-Workstation Agent Strategy and fresh role instances. | Bounded workstreams own execution while a narrow controller manages only cross-workstream flow. | Material evidence shows the control plane lacks necessary authority or creates avoidable coordination cost. |
