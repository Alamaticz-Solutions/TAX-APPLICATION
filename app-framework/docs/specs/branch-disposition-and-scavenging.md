# Branch Disposition And Scavenging

**Status:** accepted-for-implementation
**Decision owner:** Wayne Kempf
**Applies to:** App Framework local and remote branches, pull requests, and
registered worktrees

## Purpose

Reduce branch fragmentation without losing useful work. A stale or mixed donor
branch may be removed from the live branch list only after its payload has an
exact, reviewable disposition and the durable record survives branch deletion.

This contract distinguishes the value disposition from carrier cleanup:

- `MERGED`: the accepted branch payload is contained by an exact accepted-main
  commit;
- `SCAVENGED`: selected payload was accepted through one or more successors and
  every remaining payload item has an explicit outcome;
- `DEPRECATED`: no payload was retained, based on an explicit human decision;
  and
- `RETIRED`: the obsolete remote branch, local branch, and worktree carriers
  were removed after their disposition became terminal.

`RETIRED` never replaces `MERGED`, `SCAVENGED`, or `DEPRECATED`. The ledger
retains both the value disposition and carrier state.

## Scope

This contract covers:

- exact source branch name and tip SHA;
- commit and changed-path coverage;
- retained, superseded, exported, and intentionally discarded payload;
- successor PRs, merge SHAs, and destination evidence;
- Chief Architect observation and Integration Branch Manager verification;
- recovery evidence; and
- remote, local, and worktree retirement evidence.

It does not authorize merge, branch deletion, release, risk acceptance,
security-data disposal, stash mutation, or garbage collection. Those actions
retain their existing human and role boundaries.

## State Model

```text
PLANNED -> IN_PROGRESS -> MERGED | SCAVENGED | DEPRECATED -> RETIRED
```

Planning states belong in active convergence reports. The durable
[Branch Disposition Ledger](../archive/branch-disposition-ledger.md) contains
only terminal value dispositions and later retirement receipts; it must not
become another mutable branch-status dashboard.

### `SCAVENGED` Gate

A branch may be recorded as `SCAVENGED` only when all of the following are
true:

1. The source branch and source tip SHA are exact.
2. Commit and changed-path evidence identifies the payload being dispositioned.
3. Every payload item is classified as retained in a successor, already
   equivalent, exported to its owning product, superseded, or intentionally
   discarded.
4. Each retained successor is merged at an exact SHA and its required
   destination evidence is green.
5. Any intentional discard names the human decision and rationale.
6. Recovery evidence can reproduce the source tip.
7. The App Framework Chief Architect records `OBSERVED` or `CHALLENGE` against
   architecture, framework/product boundaries, and capability coverage.
8. The Integration Branch Manager records `VERIFIED` only after checking the
   exact refs, successors, evidence, and proposed cleanup targets.

`CHALLENGE` blocks terminal `SCAVENGED` status until resolved or explicitly
routed to the human decision owner. Neither observation grants deletion
authority.

### `RETIRED` Gate

After a terminal value disposition and exact human-authorized cleanup tranche,
Integration verifies separately that:

- the intended remote branch is absent;
- any intended local branch and registered worktree are absent;
- no stash, dirty carrier, custody ref, controlled payload, or unrelated ref
  was changed;
- accepted successors still resolve; and
- recovery evidence still resolves the original source tip.

Only then may the ledger carrier state become `RETIRED`.

## Terminal Record Contract

Each ledger record contains:

```yaml
source_branch: fix/example-donor
source_tip: 0123456789abcdef0123456789abcdef01234567
disposition: SCAVENGED
carrier_state: LIVE
recorded_at: 2026-08-24T00:00:00Z
successors:
  - pr: 484
    merge_sha: fedcba9876543210fedcba9876543210fedcba98
kept_payload:
  - bounded capability or commit/path manifest reference
non_kept_payload:
  - outcome: superseded | equivalent | product_exported | discarded
    evidence: retained path, PR, SHA, or human decision reference
coverage_evidence: target/appfw/example-coverage.json
recovery_evidence: bundle path plus SHA-256 or equivalent receipt
chief_architect_observation: OBSERVED
integration_verification: VERIFIED
retired_at: null
retirement_evidence: null
```

If a terminal record needs correction, append an amendment that names the
original source tip and correction reason. Do not silently rewrite historical
disposition.

## Bitbucket Annotation

When a donor has a pull request, Integration adds a final comment before the
PR is closed or declined:

```text
SCAVENGED: <source-branch>@<source-tip>
Accepted successors: PR #<id> at <merge-sha>
Durable record: docs/archive/branch-disposition-ledger.md
```

An annotated Git tag may be used only when an explicit retention requirement
needs a Git-native marker. Do not replace deleted donor branches with a new
forest of `scavenged/*` branches or routine tags.

## Acceptance Evidence

The contract is active when:

- the branch integration model routes terminal disposition through this spec;
- the Architect and Integration Branch Manager role cards require their
  respective observation and verification;
- the archive ledger exists and is terminal-only; and
- documentation checks pass.
