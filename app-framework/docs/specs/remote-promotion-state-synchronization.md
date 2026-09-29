# Remote Promotion State Synchronization Spec

Status: accepted-for-implementation

Spec depth: full

Owner roles:

- Product Owner: owns product acceptance criteria and acceptance state.
- Architect: owns record-shape and integration-boundary coherence.
- XO: owns coordination projections, holds, and next-action routing.
- Implementation owner: assigned Integration/operating-contract lane.
- Review owner: independent Framework PR Review Agent and human reviewer.

## Business Value

Eliminate avoidable waiting and false coordination state after a PR or pipeline
changes remotely. A replacement Integration or XO task must recover the train
without chat memory, while humans see fresh delivery state and existing merge,
release, product-acceptance, and risk authorities remain intact.

## Problem

PR #413 merged and its PR and destination pipelines passed, while retained XO
state continued to report the PR as open and kept a shared-seam hold active.
The repository assigned PR/CI ownership but made monitoring prompt-dependent
and did not require a terminal transition to reach XO. Task messages corrected
the incident but could not survive task replacement or harness replication.

## Goals

- Keep every assigned remote train owned through terminal destination evidence.
- Persist assignment, current observation, ordered material events, and XO
  acknowledgement outside task memory and machine-local output.
- Make stale or unknown evidence fail closed for holds and promotion routing.
- Distinguish technical destination verification from product acceptance and
  release authority.
- Suppress unchanged event noise while preserving current freshness.
- Make the same contract usable by a human-operated Integration task and later
  automation.

## Non-Goals

- Automatically push, merge, rerun pipelines, release, publish, or accept risk.
- Replace Bitbucket as remote promotion truth.
- Make `target/appfw` a program database.
- Define product acceptance criteria or release readiness.
- Add a permanent polling daemon when no remote train is assigned.
- Implement the future tracked Product Dashboard program-data source in this
  change.

## Scope

In scope: App Framework role cards, operating model, branch integration model,
task routing, docs discoverability, machine-readable promotion records,
freshness semantics, task-replacement recovery, and technical destination
verification terminology.

Out of scope: runtime, providers, generators, product behavior, Bitbucket merge
authority, pipeline rerun behavior, and product/release acceptance.

## Repository Context

- [Agent Role Cards](../start/agent-role-cards.md) assign XO coordination and
  Integration PR/CI ownership.
- [Agentic And Human Operating Model](../start/agentic-human-operating-model.md)
  defines reserved human and Product Owner decisions.
- [Branch Integration Model](../start/branch-integration-model.md) defines train
  ownership and monitoring cadence.
- [Bitbucket REST Auth Runbook](../start/bitbucket-rest-auth.md) is the only
  supported remote-auth route.
- `target/appfw/xo-active-lane-board.md` and related retained files are local
  projections and may be absent or stale.
- The accepted tree at decision time does not contain a canonical tracked
  Product Dashboard program-data source. This spec therefore defines no
  durable product-acceptance write path.

## Contracts Touched

| Contract Surface | Expected Change | Counterpart Surfaces That Must Stay Aligned |
| --- | --- | --- |
| Integration role | Own remote truth through post-merge destination evidence and emit durable records. | Role cards, operating model, integration model, task map. |
| XO role | Consume records, project freshness, preserve/clear exact holds, and route next work. | Role cards, operating model, dashboard/board projections. |
| Remote train records | Versioned assignment/current snapshot, material event, and writer-isolated acknowledgement. | Bitbucket PR metadata, future automation, manual fallback. |
| Delivery state | Use `DESTINATION_VERIFIED` for exact-SHA technical evidence. | Product Owner acceptance and human release authority remain separate. |
| Docs IA | Make the contract discoverable and checked. | `AGENTS.md`, docs indexes, docs-check maintainability contract. |

## State And Record Model

The Bitbucket PR is the default durable carrier for v1. Integration and XO
write machine-readable, marker-bounded comments without altering human-authored
PR content. The canonical marker prefixes are:

```text
appfw-promotion-assignment:v1:<train_id>
appfw-promotion-event:v1:<event_id>
appfw-promotion-ack:v1:<ack_id>
```

Assignment comments are Integration-owned. They may be updated by their author
or superseded by a higher `assignment_revision`. Event comments are
Integration-owned and append-only. Acknowledgement comments are XO-owned and
append-only. Integration never writes an acknowledgement, and XO never mutates
an assignment or event. A comment records its Bitbucket author UUID plus the
logical role and task identity. The assignment pins the allowed Integration and
XO writer principals; a record from another principal is invalid and forces
`UNKNOWN`.

Bitbucket comments do not provide a compare-and-swap lease. The protocol
therefore favors safety over availability: competing maximum revisions,
monitor claims, or event sequences are conflicts. They force `UNKNOWN`, stop
both monitors from emitting new records, preserve holds, and require XO to
route one explicit ownership resolution for Integration to publish before
monitoring resumes.

### Assignment And Current Snapshot

```json
{
  "schema_version": "appfw.remote-promotion-assignment.v1",
  "train_id": "app-framework-pr-413",
  "slot_id": "app-framework:remote-promotion:primary",
  "repository": "workspace/repository",
  "pr_id": 413,
  "assignment_uri": "bitbucket-pr-comment-uri",
  "assignment_revision": 8,
  "assignment_status": "ACTIVE",
  "source_branch": "integrate/example",
  "destination_branch": "main",
  "owner_role": "Integration Branch Manager",
  "integration_writer_principal": "bitbucket-account-uuid",
  "xo_writer_principal": "bitbucket-account-uuid",
  "monitor_id": "monitor-task-uuid",
  "lease_generation": 2,
  "lease_issued_at": "Bitbucket-server-ISO-8601",
  "lease_expires_at": "ISO-8601",
  "supersedes_assignment_uri": "bitbucket-pr-comment-uri",
  "snapshot_revision": 8,
  "assigned_at": "ISO-8601",
  "observed_at": "ISO-8601",
  "observation_status": "observed",
  "freshness": "FRESH",
  "monitor_cadence_minutes": 10,
  "stale_after_minutes": 20,
  "source_sha": "full-sha",
  "destination_sha": "full-sha",
  "merge_sha": null,
  "pr_state": "OPEN",
  "pipeline_id": 245,
  "pipeline_scope": "pull_request",
  "pipeline_commit_sha": "full-sha",
  "pipeline_state": "IN_PROGRESS",
  "last_error": null,
  "last_event_sequence": 4,
  "closed_at": null,
  "terminal_event_sequence": null,
  "terminal_ack_id": null,
  "stop_condition": "terminal destination evidence acknowledged by XO"
}
```

`observation_status` is `observed` or `failed`. `freshness` is `FRESH`,
`STALE`, or `UNKNOWN`, but is cached output rather than authority. Effective
freshness is recomputed on every read. `assignment_status` is `ACTIVE`,
`CLOSED`, or `SUPERSEDED`. A closed assignment remains discoverable history but
does not occupy the slot. `lease_issued_at` comes from the Bitbucket response
`Date` header or an equivalent Bitbucket server timestamp. `lease_expires_at`
must be no later than `lease_issued_at + stale_after_minutes`; an unbounded or
locally invented lease is invalid.

### Material Event

```json
{
  "schema_version": "appfw.remote-promotion-event.v1",
  "train_id": "app-framework-pr-413",
  "slot_id": "app-framework:remote-promotion:primary",
  "event_id": "sha256:train-transition-remote-identity-and-shas",
  "monitor_id": "monitor-task-uuid",
  "lease_generation": 2,
  "writer_principal": "bitbucket-account-uuid",
  "event_sequence": 5,
  "observed_at": "ISO-8601",
  "transition": "PR_MERGED",
  "prior_state": "PR_OPEN_PIPELINE_SUCCESSFUL",
  "new_state": "PR_MERGED_DESTINATION_PENDING",
  "source_sha": "full-sha",
  "destination_sha": "full-sha",
  "merge_sha": "full-sha",
  "pipeline_id": 246,
  "pipeline_scope": "destination",
  "pipeline_commit_sha": "full-sha",
  "pipeline_state": "PENDING",
  "next_owner_role": "Integration Branch Manager",
  "next_action": "monitor exact merge SHA destination pipeline",
  "stop_condition": "terminal destination evidence acknowledged by XO"
}
```

`event_sequence` is train-wide and strictly increasing across monitor changes.
`event_id` is a semantic idempotency key derived from the train, transition,
remote object identity/state, and relevant exact SHAs; it does not contain the
monitor identity or lease generation. A retry with the same semantic payload
uses the same ID. The same sequence with a different payload, or the same ID
with a different payload, is a conflict and forces `UNKNOWN`.

Committed events remain valid when ownership changes. XO consumes every valid,
unacknowledged global sequence regardless of the producing lease generation;
it never discards an older event merely because a newer monitor exists.

### XO Acknowledgement

```json
{
  "schema_version": "appfw.remote-promotion-ack.v1",
  "train_id": "app-framework-pr-413",
  "slot_id": "app-framework:remote-promotion:primary",
  "ack_id": "sha256:train-ack-sequence-event-set-and-projection",
  "ack_sequence": 5,
  "prior_ack_id": "sha256:prior-ack",
  "acknowledged_event_ids_sha256": "sha256:ordered-event-ids-1-through-5",
  "acknowledged_at": "ISO-8601",
  "writer_role": "XO",
  "writer_task_id": "xo-task-id",
  "writer_principal": "bitbucket-account-uuid",
  "projection_revision": 12,
  "assignment_uri": "bitbucket-pr-comment-uri"
}
```

XO appends an acknowledgement only after it consumes all event sequences from
the prior acknowledgement through `ack_sequence` into the named projections.
The acknowledgement is monotonic and may advance only to the highest
contiguous, conflict-free sequence. Missing sequences, conflicting records,
an invalid writer principal, or an invalid prior acknowledgement force
`UNKNOWN`. This acknowledgement means "consumed by XO" only; it is not product
acceptance, merge approval, release approval, or risk acceptance.

## Recovery And Freshness

Replacement startup is deterministic for PR-backed v1 trains:

1. Use the verified Bitbucket auth path.
2. Page through every open PR and all merged or declined PR history until every
   marker-bearing train has been exhausted. A 30-day first pass is an
   optimization only; it is never the authority boundary. The configured v1
   slot is `app-framework:remote-promotion:primary`.
3. Resolve the unique highest assignment revision for each train. Exclude
   `CLOSED` and `SUPERSEDED` history. Require exactly one `ACTIVE` assignment
   for the one-train slot; zero or multiple active assignments force `UNKNOWN`.
4. Load all train events and acknowledgements. Validate writer principals,
   semantic IDs, the train-wide sequence, and the highest contiguous XO
   acknowledgement. Unacknowledged events from the prior monitor remain valid.
5. Revalidate PR, source SHA, destination SHA, merge SHA, pipeline commit SHA,
   and pipeline state from Bitbucket.
6. If the current lease is valid, only its monitor may write. A replacement
   waits for explicit transfer or lease expiry. After expiry it appends a
   higher assignment revision with `lease_generation + 1`, a unique
   `monitor_id`, and the superseded assignment URI. It must reread the carrier
   and see one unique maximum claim before emitting a snapshot or event.
7. Before every write, the monitor rereads the unique active assignment. A
   displaced monitor stops writing. Equal maximum revisions or generations
   with different monitor IDs force `UNKNOWN`; XO must route ownership
   resolution and Integration must publish one higher assignment revision
   before work resumes.
8. The new monitor continues from the highest valid global event sequence.
   XO consumes every contiguous unacknowledged event, appends its own
   acknowledgement record, and never edits Integration-owned records.
9. Only a train-terminal event may close the assignment: exact-SHA
   `DESTINATION_VERIFIED`, acknowledged PR decline/supersession, or an explicit
   human stop recorded in the carrier. After its XO acknowledgement,
   Integration writes a higher assignment revision with
   `assignment_status=CLOSED`, `closed_at`, the terminal sequence, and
   `terminal_ack_id`. Historical markers then remain visible without occupying
   the slot.

A failed, stopped, or expired pipeline run is run-terminal but not
train-terminal. The assignment remains `ACTIVE`; Integration records the
failure, routes correction ownership, and continues monitoring the same or
explicitly superseding train.

A timeout records stale/escalation state and triggers explicit transfer or
reassignment; it does not terminate monitoring. Pipeline-only trains are not
supported by v1 because this change defines no independently discoverable
carrier. XO must keep a pipeline-only train held until a separately reviewed
carrier adapter exists.

Effective freshness is independent of cached freshness and last-known remote
state. Every reader recomputes it:

- `observation_status=failed` is `UNKNOWN`;
- age beyond `stale_after_minutes` is `STALE`;
- malformed or future timestamps beyond five minutes of clock skew are
  `UNKNOWN`; and
- missing records, writer violations, sequence gaps, duplicate claims, or
  conflicting payloads are `UNKNOWN`.

`STALE` or `UNKNOWN` state cannot clear holds, establish
`DESTINATION_VERIFIED`, or make another promotion eligible. It permits only
refresh, recovery, conflict resolution, and escalation.

## Acceptance Terminology

`DESTINATION_VERIFIED` means required destination evidence succeeded and the
pipeline-reported `pipeline_commit_sha` equals the exact `merge_sha`.
Destination branch head is retained separately because it may advance.
Integration originates this technical evidence; XO may project it.

Product `Accepted` means the named Product Owner or human authority has recorded
that product acceptance criteria are satisfied. This spec creates no Product
Owner acceptance, merge, release, production-readiness, or accepted-risk
authority. Retained dashboard state is a projection, not durable acceptance
history.

## Options Considered

| Option | Pros | Cons | Decision |
| --- | --- | --- | --- |
| Task-message protocol only | Immediate and cheap. | Lost on replacement; no ordering or freshness delivery. | Rejected. |
| Machine-local `target/appfw` registry | Easy for scripts and dashboard. | Lost on machine move/cleanup; repeats split-brain failure. | Rejected as authority; allowed only as projection. |
| Commit dynamic state to `main` | Auditable in Git. | Creates recursive PRs, latency, and coordination churn. | Rejected. |
| Bitbucket PR marker records | Co-located with remote truth; survives tasks and machines; bounded by one train. | Requires disciplined metadata writes and idempotency. | Selected default. |
| Always-on polling daemon | Lowest latency. | Unnecessary idle cost and new operations burden. | Deferred; assigned-train automation may implement this contract. |

## Decision Provenance

| Date | Owner | Decision | Evidence / Rationale | Revisit Trigger |
| --- | --- | --- | --- | --- |
| 2026-07-15 | Human sponsor | Make the Integration-to-XO transition protocol permanent. | PR #413 merged and passed while the retained board remained stale. | A better shared event transport is adopted. |
| 2026-07-15 | Strategist 2 | Use durable remote records, fail-closed freshness, and technical verification terminology. | Independent comprehensive review of initial contract returned `NO-GO`. | K1 tracked program data is accepted or delivery topology changes. |
| 2026-07-15 | Architect | Accept record shape and integration-boundary coherence. | Exact-SHA review of `d23586f2f116a9626ae489764d3044b5c057389b` returned `GO` with all counts zero; manual carrier proof remains a merge gate. | Carrier proof fails or a different shared transport is proposed. |

## Architecture And Implementation Notes

Stage 1 is immediately operable by Integration and XO using Bitbucket REST and
the same versioned records manually. Stage 2 may automate observation, record
upsert, event append, acknowledgement, and dashboard projection. Both stages
must use the repo-owned auth guidance and must not print credentials.

Automation updates current snapshot on every observation but appends an event
only for a material transition. It may generate local board/dashboard files,
but it must recover from Bitbucket and never infer remote truth from those
projections.

## Security, Privacy, And Governance

Records contain repository delivery metadata only. They must not include
tokens, credentials, source payloads, tenant data, PHI/PII, or approval secrets.
Bitbucket write access remains limited to marker comments: Integration may
write assignment/event markers, and XO may write acknowledgement markers only.
The pinned Bitbucket UUIDs may be the same configured service identity; record
families, role/task identity, and append-only ownership provide protocol-level
writer isolation, not a cryptographic separation of principals. No record
authorizes source push, merge, rerun, release, SRA/CAB, product acceptance, or
risk acceptance.

## Acceptance Evidence

| Criterion | Proof Command / Artifact | Required Before |
| --- | --- | --- |
| Contract is discoverable from agent start paths. | `scripts/appfw framework docs-check --json` | push |
| Roles, records, recovery, stale behavior, and authority boundaries agree. | Comprehensive Framework PR Review | push |
| No generated drift or framework regression. | `scripts/appfw framework generate --check --json`; `scripts/appfw framework test --fast --json` | push |
| Handoff binds exact SHA and changed surfaces. | `scripts/appfw framework handoff --json` | review |
| Manual operating stage survives a replacement exercise. | Retained dry-run showing marker discovery, bounded lease claim, ordered old-event takeover, XO acknowledgement, and valid closure | merge |
| Automated stage, when implemented, passes fixture tests for duplicate, out-of-order, stale, failure, merge, and destination-success events. | Future focused implementation tests | automation promotion |

## Test And Execution Feedback Plan

Run the full docs contract because the task router and agent governance changed.
Run generated parity, fast framework tests, exact-SHA handoff, auto-depth brief,
and comprehensive independent review. If the manual replacement dry-run reveals
that Bitbucket comments cannot provide recoverable state, stop and revisit the
transport before merge rather than falling back to task memory.

The dry run must exercise competing lease claims, old unacknowledged event
takeover, stale recomputation without a writer, wrong-pipeline-SHA rejection,
assignment/event versus acknowledgement writer-family enforcement, and the
difference between a failed run that stays `ACTIVE` and a successful or
explicitly stopped train that becomes `CLOSED`.

## Risks And Controls

| Risk | Control | Owner | Status |
| --- | --- | --- | --- |
| Two monitors write conflicting state. | Unique lease claim, read-before-write, explicit conflict detection, and fail-closed XO resolution. | Integration / XO | specified |
| Takeover strands an old event. | Events are train-wide; all contiguous unacknowledged sequences survive lease changes. | XO | specified |
| XO overwrites Integration state. | XO acknowledgement is a separate append-only, writer-pinned record. | XO | specified |
| Unchanged observations create history noise. | Snapshot revision updates; events remain material-only. | Integration | specified |
| Stale state advances work. | Freshness is fail-closed for holds and eligibility. | XO | specified |
| CI success becomes product acceptance. | `DESTINATION_VERIFIED` is technical; Product Owner/human acceptance is separate. | Product Owner / XO | specified |
| A green pipeline belongs to another commit. | `pipeline_commit_sha` must equal `merge_sha`. | Integration | specified |
| Machine move loses state. | Bitbucket PR marker records are the durable carrier. | Integration | specified |
| Automation expands authority. | Explicit non-authority contract and comprehensive review. | Human reviewer | specified |

## Tech Debt And Follow-Up

- Implement the automated observer/record updater as a bounded delivery-system
  slice after this manual contract is accepted.
- When canonical tracked Product Dashboard program data lands on `main`, add an
  explicit reviewed mapping from `DESTINATION_VERIFIED` evidence to that source;
  do not infer product `Accepted`.

## Handoff Notes

Role Card Check: Strategist / Product Manager Agent under explicit human
assignment. Work stayed within strategic operating-policy correction and
tracked documentation. No push, PR, merge, release, product acceptance,
architecture approval, or risk authority was assumed. Architect review of the
prior SHA returned `NO-GO` and identified acknowledgement, fencing, exact-SHA,
and freshness defects. Those corrections received exact-SHA Architect `GO` at
`d23586f2f116a9626ae489764d3044b5c057389b` with all counts zero. The final
branch still requires fresh exact-SHA comprehensive review and the manual
carrier proof before merge. Promotion order and remote evidence remain with
Integration/XO; product acceptance remains with Product Owner/human authority.
