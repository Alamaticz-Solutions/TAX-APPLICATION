# Gate 0 Paved-Path Semantic Gates

Status: accepted for local G0-B1 implementation

Spec depth: full

Owner roles: Product Owner owns acceptance; Architect owns frozen seams and
stop rules; XO owns WIP and A -> B1 -> B2 order; the G0-B1 Coding Agent owns
bounded implementation and local proof; an independent Framework PR Review
Agent owns review. Human authority remains required for promotion and merge.

## Value And Problem

Paved-path gates must be trustworthy. At base
`097e2b890b6d2b2ead8195d4e2f65ce8fc0b3530`, golden-downstream can emit
`"ok": false` and exit zero, compat can emit top-level `"ok": true` while
substantive checks are false, docs-check mostly asserts field presence, and PR
CI does not invoke cli-test or executable second-consumer proof directly.
Validation JSON also omits the requested namespace and actual config scope.

## Goals

- Required false semantics produce false JSON and a nonzero exit.
- Non-gating false checks are explicit and cannot create top-level success.
- Second-consumer executes required lifecycle commands in PR CI.
- Docs-check parses semantic outcomes; validate names command and actual scope.
- CLI test and golden downstream run directly in PR CI.

## Scope And Non-Goals

Scope is limited to `scripts/appfw`, golden introspection where required,
`scripts/check-doc-examples.sh`, focused CLI/golden/compat tests, the
second-consumer profile, `docs/reference/cli.md`, and direct PR CI coverage.

No shell-to-Rust rewrite, command-family redesign, dependency, new evidence
category, B2 provenance/handoff aggregation, runtime/data/provider/security,
generated-family, dashboard, ServiceNow, Kafka, product UX, readiness, push,
PR, remote CI, merge, release, risk, SRA, or CAB work is in scope.

## Contract

Required `ok:false` results must retain parseable JSON and return nonzero.
Compat `ok` reflects all substantive checks; any zero exit with false checks
requires explicit non-gating classification. Golden second-consumer proof runs
the declared commands, not only a plan. Validate JSON records the command,
requested namespace, and actual validated scope. PR CI invokes existing command
families directly instead of adding a new wrapper or evidence category.

Harness-check derives each required product report root from the profile's
declared commands and resolved product roots; it does not select roots by
profile name or assume `.appfw/target/appfw`. Report-writing commands require
writable coverage for that resolved in-product root. Handoff remains a separate
lifecycle/review contract: profiles that allow it must declare writable
`target/appfw` and `target/appfw/agent-handoff.json`. Handoff-only profiles do
not need an unused command-report root. Missing required coverage or a mismatched
handoff artifact returns parseable `ok:false` and a nonzero exit.

## Proof And Stops

Negative fixtures land and run red before implementation. Focused cli-test,
golden, compat, and docs checks run per slice. Terminal local proof includes
diff-check, change-impact, full docs-check, required golden/compat/CLI suites,
framework validate, generate-check, fast tests, handoff, auto-depth brief, and
fresh independent comprehensive review.

Stop and route to XO/Architect on any A-owned file, generated-family overlap,
root manifest/lock requirement, shell-to-Rust or command-family redesign, new
evidence category, B2 provenance claim, dashboard/synthetic-consumer expansion,
or dependency addition. Reports and targets remain isolated and uncommitted;
B1 remains local-only until accepted A main evidence and a promotion checkpoint.

## Decision Provenance

On 2026-07-11 the human, XO, and Architect released local G0-B1 from the exact
accepted main SHA with the frozen scope above. The implementation choice is the
smallest semantic truth correction with negative fixtures; revisit only if
execution feedback contradicts the acceptance contract.

On 2026-07-15 the Architect confirmed the compatibility split between product
command reports and lifecycle/review handoff evidence. This lane preserves that
split without moving public artifacts, dual-writing, or changing downstream
review, release, or SRA consumers.
