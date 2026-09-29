# App Framework Delivery Profiles

Status: accepted-for-implementation

The root-only `scripts/appfw mode` command selects one canonical local delivery
profile. `accelerated` uses focused routing and records deferred gates;
`candidate` uses enforce-candidate routing. The policy is tracked at
`docs/start/delivery-profiles.json`; mutable state and its deferred-gate ledger
are stored below each worktree's Git directory and are deliberately untracked.
Linked worktrees therefore keep independent local delivery profiles.

When both mutable files are absent, the controller projects the tracked
`accelerated` default side-effect-free and does not create state in either a
fresh primary worktree or a fresh linked worktree. `mode set` is the only writer
and requires a clean checkout; it binds the selected profile to the exact source
SHA. Half-present, malformed, or noncanonical persisted state still fails closed
with parseable `ok:false` output.

`mode status`, handoff, review-brief, and change-impact describe dirty or stale
source binding without aborting those diagnostics. JSON stdout from the three
artifact-producing diagnostics is the final retained artifact, including
`delivery_profile`. Delivery annotations do not satisfy or weaken handoff,
review-output, pre-push, merge, release, risk, package, provider, or reviewer
freshness and authority checks.

## Gate Execution Projection

The deferred-gate ledger remains policy-only. The Python delivery-mode
controller joins that policy with retained execution evidence and emits
`appfw_gate_execution_projection@1`. Each required framework gate reports its
gate ID, `required` or `deferred` policy disposition, execution status
(`passed`, `failed`, `not-run`, `missing`, or `stale`), exact source SHA,
canonical command, evidence path, evidence SHA-256, and explicit reason.

Canonical evidence is written atomically at:

- `target/appfw/gate-evidence/framework-docs-check.json`;
- `target/appfw/gate-evidence/framework-generate-check.json`; and
- `target/appfw/gate-evidence/framework-test-fast.json`.

Only retained, well-formed evidence bound to the current SHA and canonical
command can establish `passed`. Missing or malformed evidence cannot be
inferred from console output or timestamps. Candidate-profile handoff is
`ok:false` unless every required gate passed. Accelerated policy may defer a
gate, but its actual failure, missing, stale, not-run, or passed state remains
visible in the handoff.
