# PR Fail-Fast Preflight

Status: implemented in the combined current-main correction candidate; no remote acceptance credit

Spec depth: full

Owner roles:

- Product Owner: human sponsor for priority, assurance, cost, and merge decisions
- Architect: App Framework Architect
- XO: not used for this bounded leaf; no lane-control authority is implied
- Implementation owner: Integration Branch Manager for the assigned convergence leaf
- Review owner: independent Framework PR Review Agent

## Business Value

App Framework pull-request feedback must be fast enough to sustain small,
mergeable changes without trading away enterprise assurance. The retained live
baseline for pipelines `#332` through `#431` shows a 47m39s PR median, a
65m19s PR p90, 45.3 failed-run compute hours, and formatting failures reported
only after roughly 53 to 54 minutes. A bounded, cache-free preflight will return
cheap deterministic failures within five minutes while leaving every existing
merge-readiness gate in place.

This is the first implementation leaf in the no-loss enterprise convergence
program. It is deliberately isolated from provider, IX, mobile, PDS, Nexus,
Technology Strategy, App Fabric, Kafka, and MCP capability work so those
streams inherit faster feedback from a stable current-main base.

This document scopes the preflight producer itself. The companion
`docs/specs/pr-fast-evidence-artifact-allowlist.md` owns the combined PR-lane
topology that runs Fast, supply-chain, and secret-scan producers concurrently
after this preflight without changing their assurance scope.

## Problem

The pull-request lane currently runs the initial fail-closed release-lite guard,
then the broad `Fast framework check`, and only afterward runs supply-chain and
secret checks. Cheap formatting and diff-hygiene failures can therefore arrive
after the most expensive serial step. The lane also lacks a small exact-SHA
preflight result, bounded per-check logs, native JUnit output, and stable
failure categories suitable for throughput measurement.

The correction must not turn changed-surface classification into gate skipping,
must not relax release-lite or security policy, and must not claim a statistical
latency improvement from a single run.

## Goals

- Add one cache-free `PR preflight` immediately after the initial
  release-lite guard and before `Fast framework check`.
- Bind the preflight to the exact triggering source commit, tested checkout
  commit, Bitbucket destination commit, fetched destination ref, allowed
  source/tested/destination relation, and effective merge base; fail closed
  when that identity cannot be established.
- Report change impact without using it to select, suppress, or weaken any
  downstream gate.
- Detect diff whitespace errors, genuine unresolved conflict markers, shell
  syntax errors, and Rust formatting errors early.
- Enforce a 240-second runner deadline with a slightly larger pipeline-step
  backstop and stable failure categories.
- Always finalize compact JSON, JSONL, allowlisted manifest, bounded logs, and
  JUnit evidence for success, check failure, timeout, and infrastructure error.
- Preserve the existing fast, supply-chain, secret-scan, and final
  release-lite/freshness gates. Their combined post-preflight topology is owned by
  `docs/specs/pr-fast-evidence-artifact-allowlist.md`.
- Provide the substantive local gate subset plus fixture/structural proof before
  remote execution; destination freshness and Bitbucket checkout identity remain
  remote-only proof.

## Non-Goals

- No gate skipping, changed-surface risk routing, or assurance reduction.
- No removal of `cargo fmt` from the later supply-chain gate; the early
  duplicate is intentional until telemetry supports a separate decision.
- No clippy, provider, release, main, tag, ProGet, or publication changes.
- No cache redesign, `sccache`, custom CI image, test sharding, or broad
  artifact-boundary cleanup.
- No change to PR #480, the mixed `82265ca...` payload, or any capability or
  downstream-product branch.
- No production-readiness, SLO-graduation, security-acceptance, or merge
  authority claim.

## Scope

In scope:

- `bitbucket-pipelines.yml` pull-request topology only;
- the exact root JUnit output path in `.gitignore`, so the local subset does not
  leave a source-dirty artifact;
- a focused standard-library preflight runner and disposable-repository test
  harness under `scripts/ci/`;
- the substantive local PR-gate subset in `scripts/ci/local-pre-push-gates.sh`;
- structural assertions in `scripts/cli-semantic-gates-test.sh`;
- the PR/release evidence and maintainability documentation that owns the
  affected contract.

Out of scope:

- branch cleanup, branch retirement, worktree deletion, stash deletion, or
  rewriting any preserved history;
- main, tag, custom release, provider-certification, or publication topology;
- modification of the broad `Fast framework check` or its command list;
- correction of existing broad `target/appfw/**` artifacts outside the new
  producer step.

## Repository Context

- Historical donor base and branch:
  `d55002d4e12a572ff3cf6215defc91000274dbaf` and
  `fix/pr-fail-fast-preflight`. They preserve the original leaf but are not the
  current promotion path.
- Current convergence base and branch:
  `a6d9d41040a32a4ed32a81b521bc113f0f69048c` and
  `fix/pr-pipeline-throughput-convergence-r2`. The earlier `bc068af8...` base,
  branch without the `-r2` suffix, and candidate
  `293f48f7494191feeb57362dd7384071252cc1e8` are historical predecessor state.
  The current exact correction SHA is established by handoff and review rather
  than self-referenced from source.
- The initial and final `release-lite-guard` steps remain fail-closed policy.
  The combined successor also bounds their remote Git operations and retains
  structured failure evidence instead of accepting stale local fallbacks.
- `scripts/ci/pr-fast-framework-check.sh` remains the single command owner for
  the existing fast framework step.
- `scripts/appfw framework change-impact --base <sha> --head <sha> --json`
  already provides report-only classification.
- Bitbucket provides triggering `BITBUCKET_COMMIT` plus
  `BITBUCKET_PR_DESTINATION_BRANCH` and
  `BITBUCKET_PR_DESTINATION_COMMIT` for pull-request pipelines. The source is
  a full object ID, while live PR evidence shows the destination may be a
  12-character hexadecimal prefix. Bitbucket may
  test a destination-integrated checkout, so `HEAD` is the tested commit and
  must not be mislabeled as the triggering source.
- Bitbucket discovers JUnit-compatible reports under `test-results/**` and
  supports producer steps that do not download prior artifacts.
- The existing pipeline changes touch release-lite-sensitive CI paths. Remote
  proof must therefore use accountable, exact-SHA release-lite evidence; the
  implementation must not invent or bypass that approval.

## Contracts Touched

| Contract Surface | Expected Change | Counterpart Surfaces That Must Stay Aligned |
| --- | --- | --- |
| Pull-request pipeline | Insert cache-free preflight after the first release-lite guard | release docs, local subset, semantic topology test |
| PR evidence | Add source/tested/destination-bound JSON/JSONL/log/manifest/JUnit allowlist | runner fixtures, artifact declaration, native test reporting |
| PR commit identity | Require exact source, tested checkout, destination ref/commit, allowed relation, and effective merge base | Bitbucket variables, bounded fetch behavior, local explicit-base mode |
| Failure taxonomy | Emit stable category and original child exit code | JSON, JUnit, progress events, tests |
| Local PR loop | Run the same checks before the existing fast step | local usage docs, explicit base resolution, CI fail-closed distinction |
| Maintainability guidance | Define T0 as feedback, not assurance routing | release contract and unchanged comprehensive gates |

## Options Considered

| Option | Pros | Cons | Decision |
| --- | --- | --- | --- |
| Keep the current order | No source change | Continues 45-60 minute feedback for cheap failures | Rejected |
| Move the complete supply-chain step before fast | Reuses an existing gate | Restores large caches, compiles tools, and is not a bounded first signal | Rejected |
| Skip broad gates for docs-only changes | Largest immediate wall-time reduction | Changes assurance/risk routing before classification is proven | Deferred |
| Add a small cache-free preflight and retain all gates | Bounded, reversible, measurable, no assurance reduction | Intentionally duplicates rustfmt and adds a small step | Selected |

## Decision Provenance

| Date | Owner | Decision | Evidence / Rationale | Revisit Trigger |
| --- | --- | --- | --- | --- |
| 2026-08-22 | Human sponsor | Start the original pipeline leaf immediately after preservation begins, before the then-planned PR #480 merge-forward (historical sequencing, now superseded by the combined successor) | Explicit seven-step enterprise convergence order | Human changes convergence priority |
| 2026-08-22 | Workstream Analyst / Integration Branch Manager | Use a cache-free preflight without changing downstream assurance | 100-pipeline baseline: PR p50 47m39s, p90 65m19s; 24 late supply failures wasted 18.8 preceding compute hours | Exact replay contradicts the diagnosis |
| 2026-08-22 | Architect | Treat change impact as telemetry only in this leaf | Risk-tier routing needs separate exact-SHA evidence and acceptance | A later accepted routing spec supersedes this contract |
| 2026-08-22 | Architect | Require a full spec and comprehensive independent review | CI/release-gate and retained-evidence surfaces are sensitive Class D work | Repository review policy changes |

## Architecture And Implementation Notes

The pull-request order becomes:

```text
initial release-lite guard
PR preflight                    # new, cache-free
Fast framework check || supply-chain gate || secret scan
                                # companion spec owns this fail-fast group
final release-lite + destination freshness guard
```

The preflight runner is a standard-library Python program. It uses subprocess
argument arrays, never `eval`, and consumes NUL-delimited Git path output. In a
Bitbucket pull-request pipeline it must:

1. require and validate full `BITBUCKET_COMMIT`, a valid
   `BITBUCKET_PR_DESTINATION_BRANCH`, and a 12, 40, or 64 character hexadecimal
   `BITBUCKET_PR_DESTINATION_COMMIT` consistency identity;
2. resolve `source_sha` from the exact `BITBUCKET_COMMIT` and `tested_sha`
   from `HEAD`, requiring both commits to exist;
3. resolve the named destination head independently from the authenticated Git
   remote, require a 12-character supplied destination ID to prefix-match it or
   a 40/64-character supplied identity to equal it exactly, fetch the branch,
   require full-SHA equality with that remote head, and retain only the full
   canonical SHA;
4. accept only a tested commit equal to source with destination as ancestor,
   equal to destination with source as ancestor, or an exact two-parent merge
   whose direct parent set is `{source_sha, destination_sha}`;
5. require the effective merge base between destination and tested commits,
   then run report-only change impact for that merge base and `tested_sha`;
6. run `git diff --check <merge-base> <tested-sha>` and resolve every changed
   path/tree/blob from `tested_sha`;
7. obtain each changed blob's exact Git size before materialization and fail
   closed above the 32 MiB inspection cap; never truncate or silently skip
   oversized text; use an unbuffered, size-verified descriptor for child-written
   content and terminate/reap the child plus close both descriptors on every
   post-spawn failure path;
8. inspect changed text blobs for genuine conflict-marker lines and run
   `bash -n` on changed shell blobs without following worktree symlinks;
9. run `cargo fmt --all --check` against the tested checkout;
10. stop at the runner-wide 240-second deadline.

The change-impact report contract succeeds only when its child exits zero and
emits a parseable, typed `framework` / `report-only` success shape bound to the
exact artifact path, effective merge base, and `tested_sha`. Cheap serialization
integrity covers typed changed-file records, aggregate/list shapes and counts,
added-plus-deleted churn, recommended verification, and an A-D change-class
shape with a reason and boolean review/routing flags. `error`, `git_errors`, or
`git_error_count` surfaces, malformed structure, or mismatched identity fail
closed. This contract validity does **not** independently recompute or certify
the classifier's bucket, aggregate, class, or recommendation semantics.

Generic candidate delivery annotation may make top-level operational `ok`
false because required gate evidence is stale before the gates run; its emitted
report remains usable as telemetry only when
`delivery_profile.gate_execution_projection.ok` is the boolean `false`. A
projection, when present, must have a boolean `ok`; an arbitrary false value or
a false projection paired with top-level true is incoherent and fails. The
original payload is retained unchanged in the canonical command artifact and
copied to the preflight-owned immutable
`target/appfw/pr-preflight-change-impact.json` snapshot. The preflight manifest
hashes the snapshot rather than the canonical report, so later handoff, review,
or explicit change-impact commands may refresh `target/appfw/change-impact.json`
without invalidating the completed preflight evidence. The preflight result and
manifest report contract validity separately from operational profile status.
Neither value selects or suppresses a later gate. The docs-check fixture applies
the same typed distinction so candidate docs evidence does not circularly
require already-current docs evidence while it is producing that evidence; Git
errors, malformed comparison identity, and non-candidate false results still
fail.

The 32 MiB ceiling is deliberate: the implementation base contains tracked
generated provider seed blobs up to 20,267,463 bytes, so a 16 MiB ceiling would
reject an existing tracked-file envelope if one of those sources changed.

Local execution may accept an explicit base ref or SHA, set
`source_sha == tested_sha` with relation `local-source-equals-tested`, and
resolve its effective merge base. Local convenience must not create a CI
fallback: missing, stale, unresolvable, or mismatched Bitbucket source,
destination, or tested relation is an infrastructure failure that requires a
corrected/rerun pipeline.

The stable failure categories are:

```text
change_classification
diff_hygiene
conflict_marker
shell_syntax
rust_format
timeout
infrastructure
unknown
```

The runner preserves the failed child exit code in evidence while returning a
nonzero process status appropriate to the pipeline. Rustfmt absence or tool
startup failure is `infrastructure`, not `rust_format`. The outer cache-free
step gives its rustfmt availability probe a three-second GNU `timeout`, retains
the 45-second rustup bootstrap bound, and gives both a two-second
`--kill-after` escalation so TERM-resistant tool proxies cannot consume the
step before evidence starts. The evidence-owning runner classifies any
remaining absence. Destination advancement or a source/tested relation
mismatch is infrastructure/stale-input evidence, not a comparison against an
unbound commit.

Required producer evidence:

```text
target/appfw/pr-preflight-change-impact.json
target/appfw/pr-preflight.json
target/appfw/pr-preflight-progress.jsonl
target/appfw/pr-preflight-manifest.json
target/appfw/pr-preflight-logs/*.log
test-results/pr-preflight.xml
```

The command-owned `target/appfw/change-impact.json` remains available in the
working directory for normal framework compatibility, but it is deliberately
outside the preflight upload and manifest because later commands refresh it.
The manifest allowlist binds each immutable artifact path, byte size, and
SHA-256 plus source SHA, tested SHA, tested-commit relation, destination SHA,
effective merge base, change-impact contract validity, and the distinct
operational profile status. The runner rejects unexpected files from its owned
evidence roots. Logs are bounded and must not dump environment values. The
Bitbucket step declares only these paths, downloads no prior artifacts, and
declares no caches.

## Security, Privacy, And Governance

- The runner processes repository-visible source and Git metadata only. It
  must not print the environment or secured variables.
- Ref values are validated and passed as subprocess arguments, not interpolated
  into shell commands.
- Changed file content comes from Git objects. Symlinks are classified without
  dereferencing their worktree targets; binary content is not marker-scanned.
  Exact blob size is checked before materialization; changed blobs above 32 MiB
  fail closed with finalized evidence rather than being truncated or skipped.
- No secrets, tenant data, PHI/PII, external AI/tool egress, or production
  credentials are added.
- The initial/final release-lite guards, closing destination-freshness check,
  supply-chain gate, full-history secret scan, and human release/risk authority
  remain unchanged in assurance scope.
- Because the implementation changes `bitbucket-pipelines.yml` and
  `scripts/ci/*`, its PR is expected to require exact-SHA release-lite evidence:
  `APPFW_RELEASE_LITE_EVIDENCE_URL`, `APPFW_RELEASE_LITE_APPROVER`, and
  `APPFW_RELEASE_LITE_REASON` must come from accountable owners.
- Comprehensive Framework PR Review is required before push. A green preflight
  is feedback evidence, not merge, production, security, or risk authority.

## Acceptance Evidence

| Criterion | Proof Command / Artifact | Required Before |
| --- | --- | --- |
| Exact current-main base and isolated branch | `git merge-base --is-ancestor a6d9d410... HEAD`; clean source status; handoff records the exact candidate | implementation |
| Every check and failure category is executable | `bash scripts/ci/pr-preflight.test.sh` | push |
| Preflight precedes fast and declares no cache/broad artifact | `bash scripts/cli-semantic-gates-test.sh` | push |
| Change classification remains report-only and its preflight snapshot survives later canonical refresh | fixture assertions plus `target/appfw/pr-preflight-change-impact.json` and `target/appfw/change-impact.json` | push |
| Framework contracts and governance remain valid | `scripts/appfw framework validate --json`; `scripts/appfw framework governance-check --json` | push |
| Docs and generated ownership remain aligned | `scripts/appfw framework docs-check --json`; `scripts/appfw framework generate --check --json` | push |
| Existing fast proof remains green | `scripts/appfw framework test --fast --json` | push |
| Machine-readable handoff retained | `scripts/appfw framework handoff --json` | review |
| Sensitive-depth review selected | `scripts/appfw framework review-brief --auto-depth --json` | review |
| Independent review is GO or eligible GO WITH CONDITIONS | comprehensive Framework PR Review artifact, zero blocker/critical | push |
| Exact remote preflight is clean in at most three minutes | Bitbucket step result bound to source/tested/destination SHA and allowed relation | merge |
| Controlled formatting failure reports within five minutes | exact-SHA controlled fixture/replay pipeline evidence | merge |
| JUnit is ingested and artifacts match manifest | Bitbucket test report plus downloaded manifest/hash verification | merge |
| Existing downstream gates remain green and unchanged in scope | same pipeline's fast, supply-chain, secret, and final guard results | merge |
| Release-lite approval is accountable and exact-SHA | secured guard evidence; no checked-in substitute | merge |

## Test And Execution Feedback Plan

`scripts/ci/pr-preflight.test.sh` will build disposable Git repositories and
exercise:

- clean docs-only success;
- whitespace, complete/incomplete/malformed rooted conflict blocks, invalid-
  shell, and unformatted-Rust failures while a lone illustrative marker remains
  non-terminal;
- missing, invalid, and unresolved source identity; stale destination identity;
  mismatched tested relation; and missing merge-base failures;
- direct-source, tested-destination fast-forward, and real behind-destination
  synthetic-merge identity success;
- valid report-only change impact with operational `ok:true`, candidate-
  annotated `ok:false`, malformed structure, identity mismatch, arbitrary
  unannotated false, incoherent false projection, nonboolean projection, and
  forbidden Git-error-surface coverage;
- changed text above the 32 MiB inspection cap with complete failure evidence;
- timeout versus infrastructure classification;
- original child exit-code capture;
- immutable preflight change-impact snapshot and manifest integrity after a
  later canonical change-impact refresh;
- success, nonzero, and hanging transport for both remote destination query and
  fetch under the shared noninteractive per-operation bound;
- JSON, progress, manifest, log, and JUnit finalization on every outcome;
- rejection of unexpected evidence paths;
- filenames with spaces and unusual bytes, renames, deletions, binaries, and
  symlinks;
- source/tested/relation/destination/effective-merge-base binding.

The semantic-gates fixture will verify that the PR preflight is between the
first release-lite guard and the downstream producer group, has no caches,
downloads no prior artifacts, publishes no broad `target/appfw/**` glob, has a
substantive local fixture/gate subset, and does not remove any downstream gate.

Remote proof uses one clean exact-SHA PR run plus a controlled unformatted-Rust
fixture/replay. One run establishes the merge criterion, not a p95. A rolling
20-PR sample is required before declaring the proposed preflight p95 SLO
graduated. If execution reveals incorrect source/tested/destination identity,
unbounded logging or blob inspection, false marker detection, or downstream
evidence dependence, stop the leaf and revise this spec before push or merge.

## Risks And Controls

| Risk | Control | Owner | Status |
| --- | --- | --- | --- |
| Pipeline clone lacks merge-base history | Bounded destination fetch, then fail as infrastructure; never compare an invented range | Implementation owner | open |
| Source or destination identity is stale/unbound | Require exact resolvable `BITBUCKET_COMMIT`, canonical full remote destination equality, an allowed tested relation, and a closing destination-head query after long producers; rerun on mismatch | Implementation owner / Integration | pipeline #433 exposed the abbreviated provider value; stronger canonicalization and closing stale-destination rejection are locally fixture-proven and await remote rerun |
| Remote destination transport stalls or fails | Disable terminal prompts, bound the opening fetch to 240 seconds and each of the two serial closing operations to 120 seconds, retain structured red evidence, reject stale local fallback in Bitbucket mode, and keep five-minute step backstops | Implementation owner / Integration | hanging and nonzero-transport fixtures plus the aggregate closing-budget contract pass locally; exact remote proof pending |
| Evidence is lost on child failure or timeout | Runner owns subprocesses and finalizes JSON/JUnit before exit; opening and closing red guard artifacts use named scoped `capture-on: always` uploads | Implementation owner | pipeline #433 proved preflight failure-path JSON, JUnit, logs, and artifact retention; release-lite red upload proof pending |
| Path/ref injection or symlink escape | NUL-safe Git paths, subprocess arrays, ref validation, Git-object content | Implementation owner / reviewer | locally fixture-proven; corrected remote identity path pending rerun |
| Huge changed text evades deadline/memory bounds | Resolve exact blob size and fail closed above 32 MiB before materialization | Implementation owner / reviewer | locally fixture-proven |
| Rustfmt bootstrap problem is misreported | Separate tool/infrastructure checks from formatting result | Implementation owner | locally fixture-proven |
| New fast signal is mistaken for assurance reduction | Preserve and structurally assert every downstream gate | Architect / reviewer | locally structure-proven; formal review pending |
| Sensitive CI PR cannot pass release-lite | Route exact-SHA evidence and approval to accountable owners; do not bypass | Human / security/platform owner | open |
| Extra step adds latency to clean PRs | No cache, 240-second deadline, three-minute clean remote criterion | Workstream Analyst | open |

## Tech Debt And Follow-Up

- Retain duplicate `cargo fmt` until a later telemetry-backed change explicitly
  retires it.
- Measure clean and failure timing over 20 PR pipelines before accepting p95.
- Defer classifier-driven T1/T2 routing, duplicate-work removal, `sccache`, and
  pinned CI images to separately specified leaves. The separately specified
  artifact-boundary and producer-parallelization leaf is
  `docs/specs/pr-fast-evidence-artifact-allowlist.md`.
- Record any skipped proof or `GO WITH CONDITIONS` item in the tech-debt
  register with an owner and retirement criterion.

## Handoff Notes

The implementation remains one bounded combined current-main successor on
`fix/pr-pipeline-throughput-convergence-r2`, based on exact main
`a6d9d41040a32a4ed32a81b521bc113f0f69048c`. The earlier `bc068af8...`
successor state is historical. The earlier instruction to freeze
and later merge-forward PR #480 was sequencing for the donor leaf and is no
longer operative. PR #480 must be managed as its own current-main refresh with
its own exact handoff, comprehensive review, and remote pipeline; this spec
does not direct its promotion. No branch, worktree, stash, or
unreachable-history retirement is authorized by this spec.

Role Card Check:

- Card used: Integration Branch Manager Agent with Architect and Workstream
  Analyst preparation.
- Work within role: bounded branch/spec ownership, exact-base sequencing,
  topology/evidence contract, implementation proof, and independent-review
  routing.
- Authority not assumed: merge to `main`, release, publication, Product
  acceptance, security/risk acceptance, CI credential ownership, or deletion.
- Routed decisions: release-lite evidence to accountable owners; review to the
  Framework PR Review Agent; merge and SLO acceptance to the human sponsor.
- Drift signal: watch until exact-SHA release-lite evidence and controlled
  remote timing proof exist.
