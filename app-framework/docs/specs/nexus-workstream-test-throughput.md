# Nexus Workstream Test Throughput

Status: accepted-for-implementation

Spec depth: full

Owner roles:

- Product Owner: human App Framework owner
- Architect: App Framework Architect
- XO: App Framework XO / convergence coordinator
- Implementation owner: Framework Implementation Agent
- Review owner: independent Framework PR Review Agent

## Business Value

Keep the fail-closed Nexus workstation-admission contract in the full docs and
PR evidence loop while reducing its approximately 90-second serial bottleneck.
Faster trustworthy feedback lets App Framework branches converge without
trading away the authority, lineage, and evidence checks that protect
enterprise delivery.

## Problem

The 36-test `scripts/check-nexus-workstreams.test.mjs` suite uses synchronous
Git and checker subprocesses. In-process `node:test` concurrency therefore does
not overlap the expensive work, and repeated runs left more than one thousand
`appfw-nexus-live-*` repositories in the host temporary directory. The suite
must become faster without skipping tests, hiding shard failures, or weakening
the full docs-check gate.

## Goals

- Run the exact canonical 36-test suite in no more than four bounded processes.
- Fail closed on missing, duplicate, failed, signalled, or malformed shard
  evidence and retain a log for each shard.
- Remove each test fixture and prove zero run-specific temporary-directory
  delta after the sharded run.
- Preserve the direct serial `node --test` path for equivalence checks and the
  existing focused `admission-smoke` path.

## Non-Goals

- Change Nexus admission, identity, source-authority, or Product Increment
  behavior.
- Reduce the 36-test contract, relax docs-check, or make CI success
  non-blocking.
- Add a general-purpose test scheduler or external dependency.

## Scope

The source test, its bounded runner and runner contract tests, the full
product-increment docs subcheck, and sensitive-path classification are in
scope. Runtime, provider, generator, product UX, and release authority are out
of scope.

## Repository Context

- `scripts/check-nexus-workstreams.test.mjs` owns the exact 36 authority,
  lineage, and fixture-regression cases and remains directly runnable through
  `node --test`.
- `scripts/check-doc-examples.sh` runs the full suite in the
  `product-increment-delivery` docs subcheck and a one-test smoke in fast mode.
- `scripts/product-increment-path-safety.mjs` classifies the checker harness as
  sensitive authority-control source.
- `docs/specs/nexus-poc-multi-workstation-delivery.md` remains the authority
  contract. This throughput leaf changes only how its tests are executed.

## Contracts Touched

| Contract Surface | Expected Change | Counterpart Surfaces That Must Stay Aligned |
| --- | --- | --- |
| Full docs-check execution | Use the bounded runner instead of one serial process | Existing 36-test source, focused smoke path, docs-check result semantics |
| Retained test evidence | Per-shard TAP/stderr logs plus one summary | Exact canonical manifest, process status/signal, TAP plans and summaries |
| Temporary fixture lifecycle | Per-test cleanup plus run-scoped fallback cleanup | `appfw-nexus-live-*` zero-delta assertion |
| Sensitive changed-surface routing | Include the new runner and its contract test | Product Increment authority-control docs subcheck |
| Authoritative proof identity | Hook-free execution of the canonical tracked test blob only | Clean/stable HEAD and tree, canonical path, Git blob ID, content SHA-256 |

## Options Considered

| Option | Pros | Cons | Decision |
| --- | --- | --- | --- |
| Keep in-process concurrency | No new runner | Synchronous child work still serializes; measured bottleneck remains | Rejected |
| Remove or sample slow cases | Fastest apparent gate | Weakens authority assurance and creates coverage ambiguity | Rejected |
| Four bounded process shards with exact TAP validation | Overlaps isolated synchronous work while preserving every case and failure | Small orchestration surface to test and maintain | Selected |

## Decision Provenance

| Date | Owner | Decision | Evidence / Rationale | Revisit Trigger |
| --- | --- | --- | --- | --- |
| 2026-08-23 | Human App Framework owner / Integration coordinator | Improve the inefficient pipeline path without weakening assurance | Original serial 34/34 measured near 90 seconds; the approved fixture-dependency reconciliation expands the exact contract to 36/36 while preserving the same bounded four-process design | Suite grows materially, four processes exceed CI capacity, or equivalence fails |

## Architecture And Implementation Notes

The canonical ordered manifest is independent of the test registrations. The
source test asserts exact equality at load time, and the runner round-robins the
manifest into at most four process shards. Each worker uses an exact anchored
name pattern. The parent accepts a shard only when process exit, signal, TAP
suite/plan, per-test records, and summary counts all agree with its assignment;
then it validates the exact cross-shard union.

Each test owns every fixture root through one registered asynchronous-local
allocator and removes them in `finally`. Every allocated root uses the
run-scoped `appfw-nexus-live-${runId}-...` namespace. A unique run identifier
therefore scopes fallback cleanup after worker termination, so even a failed or
signalled shard cannot leave its temporary repositories behind.

The runner lifecycle is execution, terminal fixture inventory and cleanup,
terminal verdict, then exactly one atomic summary publication. Schema
`appfw_nexus_workstream_test_shards@2` records `finalized:true`, start/end source
HEAD and tree, clean/stable state, `exact_candidate_bound`, canonical-manifest
SHA-256, normalized observed-union SHA-256, and initial/before/after cleanup
inventories plus cleanup status. Execution or cleanup failure retains only a
red terminal summary; green is impossible before cleanup completes.

Lifecycle test hooks and a caller-supplied noncanonical `testFile` are useful
only for runner contract tests. Their summaries explicitly record
`authoritative:false`, the non-authoritative reasons, active hook names, and the
executed test content identity; they can never set `exact_candidate_bound:true`.
Authoritative proof requires no hooks and the canonical
`scripts/check-nexus-workstreams.test.mjs` path whose worktree bytes hash to the
Git blob tracked by the clean, stable HEAD. A caller may still select a custom
`logRoot`; evidence location does not change execution authority.

The six-run serial/sharded equivalence proof is retained below the ignored
`target/appfw-convergence-evidence/nexus-workstream-six-run/<exact-SHA>/` root,
not the disposable canonical `target/appfw` root cleared by CI producer setup.
The repository cleanup entry point
`scripts/ci/prepare-appfw-evidence-root.sh` allowlists and removes exactly
`target/appfw`; it cannot remove the sibling `target/appfw-convergence-evidence`
root. No broad `target` cleanup is authorized for this proof custody.
Its custody index hashes every raw summary, TAP log, and stderr log. The normal
docs-check runner may continue using `target/appfw`; only the immutable
acceptance sample requires the separate local custody root.

## Security, Privacy, And Governance

The change uses no network, credentials, tenant data, PHI/PII, or new
dependency. It does not grant source, merge, release, Product, security, SRA, or
CAB authority. Failure handling remains deny-by-default: incomplete process or
TAP evidence fails the gate.

## Acceptance Evidence

| Criterion | Proof Command / Artifact | Required Before |
| --- | --- | --- |
| Runner contract rejects nonzero, signal, malformed TAP, omissions, and duplicates | `node scripts/run-nexus-workstream-tests.test.mjs` | push |
| Four shards cover exactly 36 passing tests | Three exact-clean-SHA, hook-free canonical-test runs retained under `target/appfw-convergence-evidence/nexus-workstream-six-run/<exact-SHA>/`; summaries use schema `appfw_nexus_workstream_test_shards@2` | push |
| Serial and sharded results are equivalent | Three exact-clean-SHA one-shard summaries compared with three exact-clean-SHA four-shard summaries; source/tree, canonical test blob/content identity, manifest SHA-256, observed-union SHA-256, and 36-test counts must agree | PR |
| Run-specific temporary-directory delta is zero | Every custody-indexed summary is `finalized:true`, `authoritative:true`, `exact_candidate_bound:true`, and records empty post-cleanup inventory with `cleanup.ok:true` | push |
| Framework contracts remain valid | framework validation, docs-check, generate-check, fast test, handoff | PR |
| Independent comprehensive review accepts exact SHA | `target/appfw/framework-pr-review.md` | push |

## Test And Execution Feedback Plan

Run the runner contract first, then focused smoke, sharded full suite, and a
serial equivalence sample. Correct orchestration failures before broader
framework validation. Any test-count or result mismatch fails closed and
requires either restoring the source test or intentionally updating this spec,
the canonical manifest, and review evidence together.

## Risks And Controls

| Risk | Control | Owner | Status |
| --- | --- | --- | --- |
| A test is silently omitted by a name filter | Independent manifest-to-registration and observed-union assertions | Implementation owner | implemented in leaf |
| Worker failure is hidden by aggregate success | Require zero exit, no signal, valid TAP, exact plans/counts | Implementation owner | implemented in leaf |
| Parallel fixtures leak or collide | Unique run ID, per-test cleanup, scoped fallback, zero-delta assertion | Implementation owner | implemented in leaf |
| CI host is oversubscribed | Hard cap of four; configurable down to one | Integration Branch Manager | watch |

## Tech Debt And Follow-Up

No assurance reduction or skipped check is planned. Revisit the shard count
only with measured CI-host capacity; do not increase the cap without a new
throughput and risk review.

## Handoff Notes

This branch is a framework test-throughput leaf, not Nexus product acceptance
or release evidence. Handoff must report exact SHA, serial and sharded timings,
the custody-indexed shard summary/log root and SHA-256, cleanup delta, full
framework checks, and independent review status.
