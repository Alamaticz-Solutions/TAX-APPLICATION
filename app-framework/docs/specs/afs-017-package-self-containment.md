# AFS-017 Package Self-Containment

Status: approved-for-implementation

Governing decision: `target/appfw/afs-007-option-a-execution-and-afs-017-containment-po-decision.md`

## Bounded Outcome

The installable App Framework toolchain archive must include every tracked file
required by a shipped toolchain command. `scripts/appfw product handoff --json`
annotates retained output through `scripts/appfw-delivery-mode.py`, whose
authoritative policy is `docs/start/delivery-profiles.json`. That policy is
therefore a toolchain dependency, not product documentation.

The Product Owner decision requires every immutable package to carry this file
or resolve it through deterministic version-matched package authority. This
slice chooses carriage inside the toolchain archive because the command and its
policy are shipped together.

## Frozen Seam

Only toolchain archive assembly, its package manifest declaration, and the
package-first contract/proof may change. Product consumer source is never
repaired by the harness; no checkout fallback, policy injection, command-scope
expansion, publishing, or release behavior is introduced.

## Package Identities And Proof

The retained `0.1.4` toolchain archive carries the policy file but exposed a
second package-only defect: its delivery-mode annotation required Git metadata
from the consumer. The separately committed `0.1.5` archive corrected that
annotation but remains immutable NO-GO evidence because its proof harness
copied the package WorkItem model into the consumer. The corrective successor
is a newly committed `0.1.6` archive. Its existing lifecycle upgrade authority
must copy only the fixed package WorkItem source to the named existing consumer
target, fail closed before a model write when either required input is absent,
verify byte equality, and retain source, target, and applied SHA-256 in the
lifecycle state. The harness may inspect that state and compare bytes but must
not write the consumer model.

The retained proof must show extracted archives, clean no-checkout consumer
creation, and three separately labeled handoff phases: `before_baseline_diagnostic`
retains `0.1.5`'s predecessor behavior without promoting it as unaided-upgrade
proof; `forward_successor_handoff` and `recovery_successor_handoff` each
require `0.1.6` package-only handoff success after lifecycle applies the model.
Both identities must contain and validate `docs/start/delivery-profiles.json`,
and the delivery-mode controller must use package identity provenance without
synthetic Git facts. The report retains dependency records for both archives
and does not label predecessor behavior as a successful no-Git handoff.

## Identity And History

Package compatibility in this slice is deliberately version-only. A matching
version does not claim equal archive bytes or content-hash equivalence; the
selected archive SHA-256 is a separate retained proof gate.

Interrupted upgrades use only the fixed internal pending-journal schema and
the bounded `appfw lifecycle recover --consumer-root <consumer-root>` command.
Before mutation, that journal captures the original WorkItem bytes, generated
enum topology, and the exact prior completed-state bytes or absence. Recovery
validates every fixed path and byte field, restores only that recorded topology,
removes the journal last, and never continues an upgrade automatically. A
subsequent retry is an explicit ordinary upgrade.

Earlier local `0.1.5`/`0.1.6` archives are historical evidence, not package
authority: the `94720c...`/`ee24ee...` pair was superseded when its consumer
phase stopped, and the `a69cad...`/`9a89c0...` pair was superseded by the
post-review correction. The corrected exact-SHA proof must retain a
reconciliation table naming the source SHA, archive hash, purpose, and
supersession reason for each observed build. Only its final retained pair may
be called candidate or immutable.

Consumer proof history is append-only within its disposable root. A strict run
records its exact head/base identity and pre-existing paths before creating the
consumer or caches; it fails rather than deleting prior attempt state. Reported
attempt and cleanup truth is derived from those retained markers, not constants.
The terminal proof also retains two independently content-distinct extracted
roots with the same package version: compatibility is `true` only under its
explicit `package-version-only` scope, while their package identities remain
distinct.

## Nonclaims

This is local package behavior evidence only. It does not publish to ProGet,
make Nexus or FLP available, establish remote CI evidence, accept risk, or
claim release or product acceptance.
