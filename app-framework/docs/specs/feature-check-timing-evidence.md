# Feature-Check Timing Evidence

Status: accepted-for-implementation

Spec depth: lightweight (bounded CI-evidence contract)

Owner roles:

- Product Owner: human (priority and merge authority only)
- Architect: App Framework Integration executor
- Implementation owner: App Framework Integration executor
- Review owner: independent Framework PR Review Agent and Integration Branch Manager

## Business Value

Create the measurement layer needed to reduce opaque CI waiting without
weakening enterprise assurance. Pipeline #434 showed that `feature-check` can
spend more than 16 minutes behind one generic heartbeat. This bounded leaf
identifies the slow compile family after the run and emits direct-command
progress events; the current Wave 3 wrapper still captures child stderr rather
than surfacing those events live.

## Goals

- Emit machine-readable start and finish progress for every existing
  `feature-check` subcheck when the command is invoked directly.
- Add per-subcheck elapsed time and exit status to retained
  `target/appfw/feature-check.json` evidence.
- Preserve the existing check set, name, order, commands, failure propagation,
  and aggregate `ok` semantics exactly.

## Non-Goals

- No parallelization, sharding, cache, artifact, timeout, pipeline-topology, or
  changed-surface routing change.
- No Wave 3 wrapper change or claim that this leaf alone surfaces subcheck
  progress live in Bitbucket.
- No check removal, skipping, command substitution, assurance reduction, or
  performance claim.
- No merge, release, risk acceptance, or remote push authority.

## Scope And Repository Context

The implementation is limited to `scripts/appfw`, a focused deterministic
contract test, the command-reference wording required for the additive JSON
fields, and this spec. `scripts/ci/wave3-pr-gates.sh` continues to invoke the
same command in the same position and continues to enforce the same result.

The branch was merge-forwarded onto verified `main` at
`bc068af8f46b0fea2277ce828b49c9544dd45f34`. PR #481 does not change
`scripts/appfw` or this command's documentation, so this leaf remains
independent rather than stacked. Remote publication remains held until the
exact candidate is reverified, independently reviewed, and challenged by the
Integration Branch Manager.

## Contracts Touched

| Contract Surface | Expected Change | Must Stay Aligned |
| --- | --- | --- |
| `feature-check` stderr progress | Add one start and one finish JSON event per existing check, with explicit finish-event timing validity | Existing stdout JSON remains parseable |
| `target/appfw/feature-check.json` | Add numeric `elapsed_ms`, Boolean `timing_ok`, and integer `exit_code` to each check plus aggregate timing and clock mode | Existing `name`, `ok`, `detail`, order, and aggregate status |
| CLI reference | Document additive timing evidence | `scripts/appfw` help and retained artifact path |

## Security, Privacy, And Governance

Events contain only stable check names, timing, timing validity, exit status,
and success state.
They must not include environment variables, credentials, command output, file
contents, tenant data, or signed URLs. Diagnostic command output remains in the
existing bounded failure logs. This evidence grants no release or risk
authority.

## Acceptance Evidence

| Criterion | Proof | Required Before |
| --- | --- | --- |
| Compile and forbidden-tree helper success/failure paths emit paired events and additive evidence | Deterministic stub-command contract test, including leading-zero and oversized numeric clock samples | review |
| Check name/order/count and aggregate failure semantics are unchanged | Baseline-plan comparison in focused test | review |
| Shell and CLI contracts remain valid | `bash -n scripts/appfw`; focused semantic test | review |
| Real compile matrix remains green | `scripts/appfw framework feature-check --json` | push |
| Framework handoff and independent review bind the exact SHA | handoff, auto-depth review brief, retained review | push |

## Risks And Controls

| Risk | Control | Status |
| --- | --- | --- |
| Instrumentation changes behavior | Snapshot names/order/count and exercise success/failure propagation with stub commands | required |
| Progress leaks diagnostics | Emit only allowlisted metadata; keep command output in existing failure logs | required |
| Branch becomes a second long-lived line | Hold remote publication until #434 terminal; rebase/retarget once | required |
| Timing clock is unavailable, malformed, out of signed-64-bit range, contains leading zeros, or changes availability mid-run | Canonicalize accepted samples as bounded base-10 integers, select one clock domain for the run, never subtract across domains, and pair any zero placeholder with `timing_ok: false`; test failures, malformed/oversized/leading-zero outputs, and both availability transitions | required |

## Handoff Notes

The Integration Branch Manager's initial verdict was `GO_INDEPENDENT`; the
pipeline #434 hold is now satisfied. Push remains held for exact-SHA proof and
the final Branch Manager challenge. Any matrix/order/topology/timeout/cache,
skipping, or failure-semantic change is scope drift and requires a new decision.

## Current Integration Limit And Successor

`scripts/ci/wave3-pr-gates.sh` currently redirects feature-check stderr to
`target/appfw/wave3-pr-gates/feature-check.stderr.log` and emits generic outer
heartbeats. The new events and timings are therefore retained and useful for
post-run diagnosis, but are not yet visible as live Bitbucket subcheck
progress. That limitation and its retirement criteria are tracked in this
spec. The separate successor
`fix/wave3-feature-check-live-progress` must forward only allowlisted
`feature-check-start` and `feature-check-finish` fields, preserve the retained
stderr file, prove stdout JSON remains parseable, and prove no diagnostic,
environment, credential, tenant, or signed-URL content can reach the live log.
