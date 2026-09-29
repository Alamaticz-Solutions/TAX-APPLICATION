# Product Increment Delivery Model

This is the required delivery model for new App Framework and product work.
It turns the canonical work hierarchy into an executable multi-lane flow.
Use it with the
[Program Work System](../reference/program-work-system.md),
[Agentic And Human Operating Model](agentic-human-operating-model.md), and
[Branch Integration Model](branch-integration-model.md).

## Required Shape

Every planned Deliverable belongs to exactly one primary Product Increment.
Every active Product Increment has:

1. one measurable Outcome and one accountable Product Increment owner;
2. one to four independently admissible Delivery Lanes;
3. one branch, write surface, branch owner, and reviewer per lane;
4. typed capability links within and across Product Increments;
5. stage-appropriate leaf checks;
6. an explicit integration strategy and merge order; and
7. separate flow state, Implemented credit, and Accepted credit.

Do not create an orphan branch, standing workstream queue, or independent
technical train outside a Product Increment. Urgent maintenance still belongs
to a bounded maintenance Product Increment so its priority, acceptance, and
integration cost remain visible.

The portfolio, not each individual Product Increment, maintains two to four
active source-producing lanes. A cohesive one-lane increment is valid and may
run beside independent lanes from other increments. Do not split work merely
to hit a concurrency number.

## Portfolio Registry

The canonical human index is
[`product-increment-portfolio.json`](../specs/product-increment-portfolio.json).
It registers envisioned, planned, executing, partially implemented, accepted,
held, stopped, and superseded major batches. Every entry uses plain language
and records:

- what the Product Increment is intended to accomplish;
- its current lifecycle and assurance stage;
- what is true now and the next accountable action;
- the capabilities it **provides** and their Planned, Implemented, or Accepted
  state;
- hard dependencies under **requires**;
- optional leverage under **benefits_from**;
- routing maturity and the explicit fallback;
- integration strategy;
- plan and evidence references; and
- when the status was last reconciled.

Register a major batch before creating its first source branch. Update its
single portfolio entry on every material state change; do not create a second
status narrative in a dashboard, handoff, or task. Those surfaces project this
record and link to detailed evidence.

A portfolio labeled `current` fails validation when its reconciliation is more
than 24 hours old or an increment status has not been verified within seven
days. A portfolio still labeled `migration_required` reports the same
conditions as visible warnings while legacy work is classified. This prevents
"current" from becoming an indefinite assertion without turning migration
work into a release gate.

Retrieve the full portfolio or one increment:

```bash
node scripts/check-product-increment-portfolio.mjs \
  --portfolio docs/specs/product-increment-portfolio.json

node scripts/check-product-increment-portfolio.mjs \
  --portfolio docs/specs/product-increment-portfolio.json \
  --id AFS-PI-P1
```

The machine-readable contract is
[`product-increment-plan.schema.json`](product-increment-plan.schema.json).
Validate a plan before lane admission:

```bash
node scripts/check-product-increment-plan.mjs \
  --plan docs/specs/nexus-add-provider.product-increment.json \
  --json
```

## Delivery Lane

A **Delivery Lane** is the execution grouping for one or more independently
provable Deliverables within a Product Increment. It is not another work
breakdown level. It exists to give concurrent work one owner, one branch, one
write surface, one focused proof loop, and one integration destination.

A lane is independently admissible only when:

- its source roots do not overlap another runnable lane;
- its branch owner is not writing another active lane;
- its independent reviewer is not its branch owner;
- its required inputs already exist at the declared state, or a frozen
  contract and representative fixture let it proceed without waiting;
- its leaf branch targets the Product Increment integration branch; and
- its focused checks can prove its bounded behavior without running the full
  release suite.

Shared files that only the convergence step may change belong in
`integration.write_roots`, never in a leaf lane. Integration roots must not
overlap lane roots. The aggregate reviewer must be independent from the
Integration owner and every lane's branch owner, technical lead, and reviewer.

Portfolio operating capacity is two to four active lanes across one or more
compatible Product Increments. Program Flow may reduce the active count when
review, integration, CI, repository, or human acceptance capacity is
constrained. The portfolio must show the reason whenever it is below target
instead of silently drifting to serial work.

## Flow Efficiency And Token Economy

Parallelism is useful only when it lowers time to an Accepted result without
raising rework, review congestion, or model cost faster than the value gained.
The harness therefore optimizes **accepted throughput per elapsed day and per
model dollar**, not agent count, prompt count, branch count, or token volume.

### Across Product Increments

- Keep two to four disjoint source-producing lanes active across the portfolio
  when review and Integration can absorb them. Prefer lanes from independent
  Product Increments so one delayed dependency does not idle the portfolio.
- A PI-specific integration branch isolates leaf progress from unrelated
  `main` changes. Leaf PRs target that branch and may continue while another
  Product Increment is proving or merging its own candidate.
- `requires` may block admission only when its declared state is unavailable
  and no frozen seam exists. `benefits_from` is never a wait condition.
- Do not keep eight standing crews. Keep the small control crew loaded and
  activate a producer, specialist, or reviewer only for an admitted lane or an
  immutable checkpoint that is ready for review.

### Within A Product Increment

- Give each active lane a bounded context capsule: the Product outcome, the PI
  plan, its assignment, and its evidence/review record. Load linked source or
  contracts on demand; do not preload the repository or replay research that
  already has a retained record.
- Use the lowest capable configured model tier for retrieval, extraction,
  fixture normalization, routine scans, and deterministic transformations.
  Escalate to a frontier reasoning tier for novel cross-domain design,
  consequential conflict resolution, or final independent synthesis. Never
  pay a frontier model to repeat retrieval already captured in typed records.
- Delegation depth is one by default. A producer does not recursively build a
  standing crew, and two agents do not analyze the same question unless the
  second has an explicit independent-review role.
- Run the smallest semantic check that can falsify the current change in the
  inner loop. Run aggregate proof once at PI convergence and strict proof once
  at release promotion. Repeating a full suite on every narrow leaf is a flow
  defect.
- Give every lane and Integration step an explicit economics budget: allowed
  model tiers plus maximum elapsed time, input/output tokens, cost, retries,
  corrections, and rework. A route may spend above the default tier only with
  a retained justification. Exceeding a budget stops or replans the work; it
  does not silently normalize higher spend.
- Bind the selected execution target, planned model tier, delegation depth, and
  a budget no larger than the lane budget into the source assignment before
  dispatch. This is the enforceable stop line; an evidence record is not a
  substitute for pre-dispatch control.
- After two correction cycles without a newly passing semantic proof,
  independently review the contract and scope before authorizing another
  correction. After two operating cycles in which model spend rises without a
  corresponding improvement in accepted lead time, findings, or rework,
  reduce WIP or model tier and record the correction.

Every retained Product Increment evidence record declares whether delivery
economics were `measured`, `unavailable`, or `not_applicable`. Measured records
include planned and actual model, reasoning effort, elapsed time, token usage,
cost, retries, corrections, rework cycles, and a metering receipt. An
unavailable record must say why; estimates may not be presented as measured
facts. Candidate work may temporarily report unavailable economics, but
Implemented or Accepted credit requires measured economics within the lane or
Integration budget and verification by a trusted usage adapter.

Program Flow reviews these measures at every material transition:

- active and review lane count;
- elapsed time to Implemented and Accepted;
- model cost and tokens per Implemented and Accepted Deliverable;
- retries, correction cycles, review findings, and rework;
- CI and independent-review queue age; and
- idle loaded agents or duplicated analysis.

The router may recommend a capability tier, but it never expands scope,
admits a lane, selects its reviewer, or awards delivery credit.

## Durable Credit Boundary

Flow state is useful before merge, but it is not durable delivery credit.

- `ready_to_start`, `in_progress`, `under_review`, and
  `waiting_for_decision` show work flow. A decision-waiting lane that claims
  Implemented credit still requires the complete evidence record.
- **Implemented** requires the exact candidate commit, execution proof,
  independent review receipt, and evidence record to be retained by accepted
  main, plus trusted external verification of reviewer identity, destination
  SHA, approval provenance, and usage metering. Local files, a locally mutable
  remote-tracking ref, and leaf-branch prose cannot award credit.
- **Accepted** adds a distinct Product-owner acceptance receipt retained by
  accepted main and verified through the same trusted authority boundary.
- Execution output, independent review, lane acceptance, and Product
  acceptance use distinct retained documents. One self-authored note cannot
  stand in for several authorities.

Until a repository or CI environment supplies those trusted authority and
metering adapters, validators fail closed and report no durable credit. This
boundary deliberately keeps the inner loop cheap. Producers can iterate with
dirty-worktree feedback and candidate evidence; only durable progress claims
pay the trusted aggregate proof cost.

## Cross-Lane Capability Links

Every capability flow between lanes is recorded once in `lane_links`.
Conversation, branch order, or a workstream name is not a dependency record.

### Required Link

Use `requires` when the consumer cannot complete without the provider's
capability. Record:

- provider and consumer lane;
- stable capability ID;
- whether the provider must be `implemented` or `accepted`;
- the contract or fixture that defines the seam;
- any decoupling method that allows both lanes to run now;
- the consumer's convergence action; and
- the integration check that proves the two implementations work together.

An active consumer may not wait on an unfinished active provider. It must
either consume an already sufficient provider state or work against a frozen
contract, representative fixture, or adapter that will be replaced and proven
at integration.

### Benefit Link

Use `benefits_from` when provider work improves another lane without blocking
it. Record the adoption trigger, expected benefit, and integration action.
This makes reuse and payoff visible while keeping the consumer independently
progressing.

Examples include:

- a design-system component improving a Nexus composition;
- a provider fixture replacing a hand-built product mock;
- an observability helper enriching another lane's telemetry; and
- an accepted permission contract reducing duplicate authorization work.

## Branch And CI Boundaries

```text
main
└── integrate/<product-increment>
    ├── feature/<lane-a>
    ├── feature/<lane-b>
    ├── feature/<lane-c>
    └── feature/<lane-d>
```

Use one short-lived `integrate/<product-increment>` branch when an increment
has two or more lanes, crosses independently versioned domains, or needs a
stable convergence candidate. A genuinely independent one-lane, one-domain
increment may use `direct_main`; its PR pays the full integration check once.
Do not create an integration branch that has nothing to integrate, and do not
let multiple lanes or cross-domain changes converge directly on `main`.

Use these proof boundaries:

| Boundary | Purpose | Required posture |
| --- | --- | --- |
| Local lane loop | Fast implementation feedback | Changed-surface build, focused semantic tests, and no remote pipeline |
| Leaf PR to integration | Prove one lane without charging the whole program | Accelerated CI, focused review, changed-content secret checks, and dependency/lock checks only when affected |
| One-lane direct-to-main PR | Prove one independent Product Increment | Full candidate CI and independent review; no duplicate integration branch |
| Integration branch | Prove the Product Increment candidate | Aggregate build, cross-lane contract tests, journey smoke tests, and conflict resolution |
| Integration PR to `main` | Pay the shared integration tax once | Full framework, supply-chain, full secret, regression, and comprehensive independent review |
| Release candidate or tag | Prove promotion readiness | Full security, accessibility, resilience, observability, deployment, and recovery evidence |

The local lane loop uses the `accelerated` delivery profile. Canonical
framework gates may run against a dirty implementation checkout in that
profile, but their results are ephemeral feedback and must not overwrite or
manufacture immutable checkpoint evidence. The `candidate` profile still
requires a clean checkout and records evidence bound to the exact source SHA.
This lets a producer test continuously while charging clean-checkpoint and
full-suite proof only at the boundary where it can support a real claim.

Sensitive changes remain visible at every boundary. On a prototype leaf,
run the affected semantic, secret, dependency, or authority check without
charging the lane the unrelated full suite. The Integration candidate and
release boundaries fail closed to `full` or `strict` proof for every observed
sensitive path. A human or agent may escalate rigor, but may not downgrade
those aggregate requirements.

The plan validator currently classifies root manifests and locks, Bitbucket
pipeline source, CI and Git-hook source, runtime chat egress, and runtime
security source as sensitive. Extend this source-controlled classification
when another path gains equivalent authority; do not rely on a lane author to
self-label risk.

Planning validation checks declared lane roots. Candidate validation must also
bind the actual Git diff to the selected lane:

```bash
node scripts/check-product-increment-plan.mjs \
  --plan <plan.json> \
  --current-diff <lane-id> \
  --json
```

This mode reads the selected lane's immutable `base_sha` and the current branch
directly from Git, includes tracked and untracked changes, rejects files
outside the lane's write roots, and reports observed sensitive paths so the
leaf can run affected checks and Integration can enforce aggregate proof. It
is required before a lane checkpoint is reviewed; omitting it leaves the
record at planning validation only.

The current Bitbucket pipeline must become destination- and risk-aware before
parallel lanes can claim the accelerated remote loop. Until then, the plan
still governs branch shape and focused local proof, but remote pipeline
duration remains an explicit delivery-system constraint.

## Router Maturity

Routing assists delivery; it never admits work, changes Product priority,
expands write scope, performs acceptance, or selects its own reviewer.

| Maturity | Permitted use |
| --- | --- |
| `manual_explicit` | A human or Program Flow records the model, effort, instructions, scope, and reviewer. This is the required fallback. |
| `transparent_candidate` | An implemented router may be reviewed and measured, but cannot be relied on as an accepted control. |
| `transparent_accepted` | Deterministic route selection and disclosure may be required by later increments. |
| `measured` | Retained Accepted-outcome evidence supports bounded economic routing decisions. |
| `adaptive` | Not permitted until a separate Product Increment proves value, safety, reversibility, and authority boundaries. |

Each portfolio entry states its current and minimum routing maturity. An
executing increment may not fall below its minimum. Optional router value is a
`benefits_from` relationship and must not block independently runnable product
work.

## Integration Planning

For a multi-lane or cross-domain Product Increment, the plan is incomplete
without:

- the integration branch and accepted base;
- one immutable `base_sha` for each Delivery Lane, updated only through a new
  admission when a later integration tip becomes its truthful base;
- the Integration Branch Manager;
- a merge order containing every lane exactly once;
- shared-path ownership;
- aggregate integration checks;
- every required and beneficial capability link;
- the action that replaces fixtures or adapters with the real provider;
- exact evidence needed for Implemented and Accepted; and
- branch/worktree retirement after consumption.

A one-lane direct-main plan still names the Integration owner, accepted base,
full candidate checks, evidence, and cleanup. Its `main` destination is an
explicit strategy, not an exception hidden in conversation.

Merge order follows hard dependencies first. Independent lanes may merge in
the order that minimizes conflict and provides the earliest coherent
demonstration. A lane merge is not Product Increment acceptance.

## Roles

| Role | Product Increment responsibility |
| --- | --- |
| Program Product Manager | Orders Product Increments and defines the Outcome, decision, and acceptance criteria. |
| Program Flow Controller | Maintains two to four independent lanes across the portfolio, enforces WIP, exposes aging and dependencies, and keeps the portfolio projection current. |
| Chief Architect | Freezes shared contracts, validates lane independence, and resolves semantic or ownership conflicts. |
| Delivery Lane Owner | Owns the lane's Deliverables, branch result, focused proof, and handoff. |
| Branch Owner | Is the sole source writer for the assigned lane branch and worktree. |
| Independent Reviewer | Reviews the immutable lane checkpoint without authorship. |
| Integration Branch Manager | Owns integration branch freshness, merge order, aggregate proof, integration PR, and terminal cleanup. |
| Outcome Owner | Separately accepts evidence; merge or CI cannot imply acceptance. |

## Dashboard Contract

The human dashboard must make this model visible without requiring Git or
harness vocabulary. Its default Product Increment view shows:

- Outcome, Product Increment intent, owner, stage, and acceptance criteria;
- target and current active-lane count;
- each lane's plain-language result, owner, state, branch, and focused proof;
- flow state, Implemented credit, and Accepted credit separately;
- **Provides**, **Required by**, and **Benefits** relationships for every lane;
- unresolved hard dependencies and the decoupling method in use;
- merge order, integration readiness, and the next integration action;
- fast versus full CI boundary; and
- the one human decision or owner action needed next.

Technical hashes and raw gate evidence belong in drill-down evidence, not the
default human view.

## Lifecycle

1. Product registers the Product Increment, Outcome, current status, and
   acceptance criteria in the portfolio.
2. Architecture identifies one to four independent lanes and freezes shared
   contracts.
3. Integration selects `integration_branch` or `direct_main`; multi-lane work
   always uses the integration branch.
4. Program Flow validates the portfolio and plan, then admits only runnable
   lanes while preserving the two-to-four portfolio ceiling, unique active
   branch owners and branches, and non-overlapping write roots across all
   Product Increments.
5. Lanes implement and prove work independently using the smallest valid loop.
6. Integration consumes reviewed lane checkpoints in declared order and runs
   convergence checks.
7. The integration PR to `main` runs the full candidate proof once.
8. The Outcome Owner accepts or rejects the Product Increment evidence.
9. Integration deletes consumed leaf branches/worktrees and the merged
   integration branch after preserving required evidence.
10. The retrospective records only actionable improvements to contracts,
    checks, or flow.

## Stop Conditions

Stop lane admission or reduce concurrency when:

- runnable lanes overlap write roots;
- a consumer needs unfinished provider behavior without a frozen seam;
- more than four source-producing lanes are active;
- more than one writer owns a lane branch or worktree;
- review or integration queues grow for two consecutive cycles;
- the integration branch is stale against accepted `main`;
- the plan omits a required capability flow;
- a multi-lane increment bypasses its integration branch;
- a direct-main increment contains more than one lane; or
- full integration evidence is being repeated on every narrow leaf.
