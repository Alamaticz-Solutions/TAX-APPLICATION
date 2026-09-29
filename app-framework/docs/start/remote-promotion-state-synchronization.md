# Remote Promotion State Synchronization

Use this contract whenever a branch is assigned to a remote pull-request or
pipeline train. It keeps live Bitbucket state, Integration ownership, XO
coordination, and human-facing projections aligned without expanding push,
merge, release, product-acceptance, or risk authority.

The durable decision and record design is specified in
[Remote Promotion State Synchronization Spec](../specs/remote-promotion-state-synchronization.md).

## Durable Ownership

- The Integration Branch Manager owns live remote truth from PR creation or
  assignment through terminal PR evidence and, after merge, terminal
  destination-branch evidence.
- XO owns the current coordination projection: active-lane board, machine
  queue, dependency and shared-seam holds, Product Dashboard state, and the
  next eligible coordination action.
- The branch owner owns source correction, current-destination proof, handoff,
  and exact-SHA review. A remote transition does not transfer source ownership
  to Integration or XO.
- Product Owner owns product acceptance criteria and product acceptance state.
  The human retains merge, release, accepted-risk, and other reserved
  authorities.

Once a PR or pipeline is assigned, monitoring is active by default. It does
not require a new human or XO prompt at every state change. Integration may use
an explicit automation or an active Integration task, but the responsibility
and record contract are the same.

## Durable Carrier And Records

The Bitbucket PR is the default durable carrier for a v1 promotion train. Its
marker records must preserve the human-authored PR summary:

- one Integration-owned assignment/current-state snapshot, updated by its
  author or superseded by a higher `assignment_revision`;
- one Integration-owned append-only record for each material event; and
- one XO-owned append-only acknowledgement for each consumed contiguous event
  range.

Assignment markers begin `appfw-promotion-assignment:v1:<train_id>`; event
markers begin `appfw-promotion-event:v1:<event_id>`; acknowledgement markers
begin `appfw-promotion-ack:v1:<ack_id>`.

An assignment must contain `train_id`, `slot_id`, repository and PR identity,
source and destination branches, `assignment_status` (`ACTIVE`, `CLOSED`, or
`SUPERSEDED`), revision, writer principals, monitor identity,
`lease_generation` and expiry, cadence, stale threshold, stop condition,
assignment URI, and last event sequence. A current snapshot must contain `snapshot_revision`,
`observed_at`, observation success or failure, cached freshness, last-known
remote state, exact SHAs, pipeline identity/state/commit SHA, and the last
error. A material event must contain a semantic `event_id`, train-wide
monotonic `event_sequence`, monitor and lease provenance, prior and new state,
exact SHAs, observation time, next owner/action, and stop condition.

Only Integration mutates or supersedes assignments and emits events. Only XO
emits acknowledgements; it never mutates an Integration record. An XO
acknowledgement names the highest contiguous consumed sequence, ordered event
ID hash, prior acknowledgement, projection revision, writer role/task, and
writer principal. It means "consumed by XO," not product acceptance.

A replacement preserves every valid unacknowledged event. It continues the
global event sequence from the prior monitor. Monitor generation is writer
provenance, not event validity. Event IDs derive from the remote transition
identity and exact SHAs, not from the monitor. Retries with an existing
semantic event ID and payload are idempotent. Duplicate maximum claims,
sequence collisions, ID/payload conflicts, invalid writers, gaps, or
out-of-order acknowledgements force `UNKNOWN`; no monitor may continue until
Integration publishes one XO-routed ownership resolution.

The exact versioned shapes and Bitbucket marker rules live in the linked spec.
Dynamic `target/appfw` files are optional local projections, not the assignment
registry, remote truth, or acceptance authority.

## Material Transitions

Integration appends one event and sends one concise XO notification for:

- PR opened, source SHA changed, approved, declined, superseded, or merged;
- PR pipeline listed, started, succeeded, failed, stopped, or expired;
- destination pipeline listed, started, succeeded, failed, stopped, or
  expired; or
- an observation failure or freshness threshold crossing.

An unchanged successful observation updates the durable current snapshot's
`observed_at` and `snapshot_revision`; it does not append an event or trigger a
human update. XO reads the durable snapshot at least once per active monitoring
cadence and consumes unacknowledged material events in sequence.

## Freshness Is Fail-Closed

- While a train is active, observe it every 5-10 minutes unless a shorter
  risk-appropriate cadence is assigned.
- Set `stale_after_minutes` to at least 15 minutes and no less than twice the
  assigned cadence.
- Keep last-known remote state separate from freshness. Stored freshness is
  cached output, not authority. Every reader recomputes effective freshness:
  failed observations are `UNKNOWN`, age beyond the threshold is `STALE`, and
  malformed/future timestamps, writer violations, sequence gaps, or conflicts
  are `UNKNOWN`.
- `STALE`, `UNKNOWN`, missing, out-of-order, or conflicting evidence is
  fail-closed: it preserves dependency and shared-seam holds, blocks
  `DESTINATION_VERIFIED`, and prevents next-promotion eligibility. It permits
  only refresh, recovery, and escalation.
- XO refreshes every affected projection within one active monitoring cadence.
  It must never display old evidence as current truth.

## Terminal Behavior And Acceptance Boundary

A green PR pipeline proves the reviewed source train. A merged PR proves that
Bitbucket accepted the merge. Neither by itself proves the destination branch.
Required post-merge destination evidence establishes the technical state
`DESTINATION_VERIFIED` only when the pipeline-reported commit SHA equals the
exact merge SHA. Destination branch head remains separate because it may have
advanced.

After `DESTINATION_VERIFIED`, XO may:

1. project that exact technical state in delivery views;
2. clear remote-train and shared-seam holds whose sole predicate was that
   delivery;
3. preserve independent product, architecture, review, or human-decision
   holds;
4. mark local candidates that predate the destination SHA as requiring refresh
   and exact-SHA proof; and
5. route the next eligible promotion or producer action within the WIP limit.

`DESTINATION_VERIFIED` does not create product `Accepted`, release-ready, or
production-ready state. Product Owner or the named human authority owns product
acceptance. XO may project an authorized acceptance record; it may not
originate one. Until a canonical tracked program-data source is accepted on
`main`, retained dashboard and board files remain projections and must not
claim to be durable acceptance history.

After terminal failure, Integration captures the concrete failed step and
evidence, classifies the failure, and routes its owner. Automation must not
rerun a pipeline, push source, merge, weaken a gate, or accept risk.

## Recovery And Manual Fallback

At task replacement or machine move, Integration pages through all open,
merged, and declined Bitbucket PR history until every marker-bearing train is
exhausted, then searches comments for the durable App Framework assignment
marker. A 30-day first pass is an optimization only, never the authority
boundary. Integration resolves each train's unique highest assignment
revision, excludes `CLOSED`
and `SUPERSEDED` history, and must find exactly one `ACTIVE` assignment for the
configured `app-framework:remote-promotion:primary` slot. Zero or multiple
active assignments force `UNKNOWN`, preserve holds, and escalate to XO.

The replacement validates all events and acknowledgements, then revalidates
the PR, exact SHAs, pipeline commit SHA, and live state. A valid unexpired lease
cannot be stolen. Lease issue time comes from Bitbucket server time, and expiry
cannot exceed the configured stale threshold. An unbounded or locally invented
lease is invalid. The replacement waits for explicit transfer or expiry. After
expiry, it appends a higher assignment revision with a unique monitor ID and
next lease generation, rereads the carrier, and emits nothing until it sees one
unique maximum claim. Before every later write it repeats the ownership read.
Competing maximum claims stop both writers and require XO-routed Integration
resolution.

XO replays every contiguous unacknowledged event, including events from an
older lease, then appends its own acknowledgement record. Only exact-SHA
`DESTINATION_VERIFIED`, acknowledged decline/supersession, or a recorded human
stop is train-terminal. After that event is acknowledged, Integration closes
the assignment with a higher revision, terminal sequence, acknowledgement ID,
and close time. Failed, stopped, or expired pipeline runs remain `ACTIVE`,
route correction ownership, and do not end monitoring. Historical markers
remain available for audit without occupying the slot.

If automation is unavailable, Integration performs the same Bitbucket-backed
record updates manually. Automation absence is not permission to keep state
only in task memory or under `target/appfw`.

Pipeline-only trains are unsupported by v1 because this contract does not
define an independently discoverable carrier for them. XO keeps them held
until a separately reviewed carrier adapter exists.

## Stop Conditions

Monitoring ownership stops only when:

- destination evidence is terminal and XO has durably acknowledged the final
  event sequence, then Integration has recorded the assignment `CLOSED`;
- the PR is declined or superseded and XO has durably acknowledged the event;
  or
- the human explicitly stops the train and the stop is recorded.

A timeout emits a stale/escalation event and transfers or reassigns monitoring;
it does not silently end ownership of a live train.

Role Card Checks for Integration and XO must state the durable assignment URI,
freshness, highest event sequence emitted or consumed, acknowledgement state,
authorities not assumed, and any projection that remains stale.
