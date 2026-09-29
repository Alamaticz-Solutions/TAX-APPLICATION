# PR Fast Evidence Artifact Allowlist

Status: implemented in the combined current-main correction candidate; no remote acceptance credit

Spec depth: full

Owner roles:

- Product Owner: human App Framework owner
- Architect: App Framework convergence coordinator
- XO: current convergence coordinator
- Implementation owner: Coding Agent on the current-main convergence successor
  `fix/pr-pipeline-throughput-convergence-r2`
- Review owner: independent Framework PR Review Agent

## Business Value

Return trusted PR feedback faster so App Framework capability branches spend
less time moving disposable build state and more time reaching reviewed,
mergeable condition. The change preserves the current assurance gates while
making their retained evidence smaller, explicit, and bound to the tested
source SHA.

## Problem

PR pipeline `#434` showed the `Fast framework check` collecting 4,749 files and
about 2.5 GiB from `target/appfw/**`, then spending about 130 seconds
compressing them into a 719 MiB artifact. The following supply-chain, secret,
and final release-lite steps downloaded that artifact even though they do not
consume it; the producer steps clear `target/appfw` before writing their own
evidence. The broad glob also treats npm cache content, generated workspaces,
packages, and transient files as though they were retained review evidence.

Authenticated exact-log evidence from successful PR pipeline `#437` at source
`5e01d85cbe21d92177ddf8b479ff7b5d9264e385` showed the Fast producer taking
52m26s, followed by supply-chain/lint at 10m58s while secret scan ran in
parallel for 2m16s. Fast, supply-chain, and secret scan do not consume one
another's evidence, so the sequential Fast-then-security topology added the
entire supply-chain duration to a roughly 66-minute warm-cache critical path.

The durable sanitized measured source is
[PR Pipeline Performance Baseline — 2026-08-23](../release/pr-pipeline-performance-baseline-20260823.md).
It preserves the original analyst artifact identity and SHA-256, exact pipeline
and source identities, terminal measurements, and the explicitly non-terminal
pipeline #442 observation without retaining credentials or raw logs. The
current pipeline and evidence-root contract are in `bitbucket-pipelines.yml`,
`scripts/ci/pr-fast-framework-check.sh`, and
`scripts/ci/prepare-appfw-evidence-root.sh`.

## Goals

- Replace the Fast step's broad upload with one named artifact containing an
  explicit, SHA-256-bound allowlist of review evidence.
- Prevent artifact downloads in steps that are proven to initialize or ignore
  inherited evidence.
- Run the independent Fast, supply-chain, and secret-scan producers together
  after the opening guard, with the closing guard waiting for all three.
- Bind retained Fast evidence to the exact source, tested checkout, and
  destination relation, including Bitbucket's two-parent synthetic merge, and
  reject a destination that advances while the long producers run.
- Keep every existing PR, supply-chain, secret, release-lite, main, tag,
  release, and publication gate in place.
- Fail closed when successful Fast evidence is missing, malformed, bound to the
  wrong SHA, or exceeds 100 MiB before Bitbucket compression.

## Non-Goals

- Do not skip or conditionally select any existing assurance gate.
- Do not shard `feature-check`, alter Cargo cache topology, install a custom CI
  image, or remove duplicate compilation in this leaf. Parallel execution
  reduces PR wall time but does not claim lower aggregate compute.
- Do not change main/tag release evidence, provider certification, ProGet
  publication, SRA/CAB, or release authority.
- Do not claim the remote upload is under 30 seconds until an exact-SHA
  Bitbucket run measures it.

## Scope

In scope:

- `bitbucket-pipelines.yml` PR artifact upload/download plumbing;
- the Fast framework check wrapper and a focused evidence packager;
- static/fixture contract tests;
- this spec and the canonical Bitbucket release-gate guide.

Out of scope:

- changes to the commands or checks executed by the existing gates;
- broad artifact changes on main, tag, focused release, or strict release
  producer steps;
- changes to product, provider, runtime, IX, mobile, or PDS behavior.

## Repository Context

- `scripts/ci/pr-fast-framework-check.sh` is the single source of truth for the
  six existing Fast commands.
- `scripts/ci/prepare-appfw-evidence-root.sh` deletes the canonical evidence
  root before a producer step writes current evidence.
- Supply-chain and secret-scan producers call that helper. The release-lite
  guard only inspects Git/environment inputs and writes its own JSON artifact.
- The Fast, supply-chain, and secret-scan producers are mutually independent;
  the opening release-lite guard gates their shared fail-fast parallel block and the
  closing guard remains after all three.
- Main/tag/custom release gates intentionally aggregate supply-chain and secret
  artifacts; their downloads must remain enabled.
- Bitbucket Cloud supports named artifacts, `capture-on: always`, and
  `artifacts.download: false`; artifacts remain available for human download
  even when later steps do not inherit them. The accepted YAML forms are
  documented in Atlassian's
  [Pipeline artifacts](https://support.atlassian.com/bitbucket-cloud/docs/use-artifacts-in-steps/)
  and
  [Step options](https://support.atlassian.com/bitbucket-cloud/docs/step-options/)
  references. The producer group uses the documented
  [parallel step options](https://support.atlassian.com/bitbucket-cloud/docs/parallel-step-options/);
  this reduces wall time but does not reduce summed build minutes by itself.

## Artifact Consumer Graph

| Step | Possible inherited evidence | Actual consumer behavior | Download contract |
| --- | --- | --- | --- |
| PR Fast | Initial release-lite artifact | Calls `prepare-appfw-evidence-root.sh`; none of the six Fast gates reads the earlier guard artifact | disabled |
| Supply-chain producer | Opening release-lite artifact; no peer-producer artifact exists yet | Calls `prepare-appfw-evidence-root.sh` before `supply-chain-gate.sh` and produces its own root | disabled |
| Secret-scan producer | Opening release-lite artifact; no peer-producer artifact exists yet | Calls `prepare-appfw-evidence-root.sh` before `secret-scan.sh` and produces its own root | disabled |
| Final release-lite/freshness guard | Concurrent Fast, supply-chain, and secret-scan artifacts | Reads Git/environment inputs, writes `release-lite-guard.json`, and compares the pinned destination identity with the current `origin` branch head; it does not read retained step artifacts | disabled |
| Focused/strict release gate | Parallel supply-chain and secret-scan artifacts | `bitbucket-release-gate.sh` loads and revalidates `supply-chain-gate.json`, `secret-scan.json`, and their child evidence | enabled and unchanged |
| ProGet publisher | Strict release-gate artifact | `proget-publish.sh` explicitly verifies downloaded `bitbucket-release-gate.json` and must not clear it | enabled and unchanged |

## Contracts Touched

| Contract Surface | Expected Change | Counterpart Surfaces That Must Stay Aligned |
| --- | --- | --- |
| PR Fast artifact | Broad `target/appfw/**` becomes named `pr-fast-evidence` with a manifest and allowlisted files | Fast wrapper, packager, static test, release-gate guide |
| PR artifact inheritance | Producer/final-guard steps that do not consume prior evidence set `download: false` | Evidence-root cleanup behavior and release aggregation lanes |
| PR producer topology | Fast, supply-chain, and secret scan share one fail-fast post-preflight parallel block; the final guard waits for the group | Opening/final release-lite order, producer independence, Bitbucket parallel syntax and explicit group-level `fail-fast: true` |
| Evidence identity | Manifest records exact source, tested, destination, permitted relation, path, bytes, and SHA-256 for every retained file | Git HEAD, Bitbucket PR identity, exact merge parents, staged artifact copy |
| Destination freshness | Closing guard rejects a destination branch that advanced after pipeline identity was pinned | `BITBUCKET_PR_DESTINATION_*`, `origin`, focused fixture, pipeline topology |
| Evidence size | Successful bundle must be no larger than 100 MiB uncompressed | Packager failure behavior and exact-SHA remote timing follow-up |

## Options Considered

| Option | Pros | Cons | Decision |
| --- | --- | --- | --- |
| Keep `target/appfw/**` | No source change | Repeats measured 2.5 GiB transfer and carries caches/transients | Rejected |
| Enumerate many raw YAML globs | Small implementation | YAML and evidence identity can drift; no per-file hash proof | Rejected |
| Build a named, staged allowlist plus manifest | Explicit, hash-bound, testable, selectively downloadable | Adds a small packaging step and contract test | Selected |
| Remove Fast artifacts entirely | Lowest transfer | Weakens retained diagnostics and review evidence | Rejected |
| Keep Fast before the existing security parallel group | No topology change | Adds the measured 10m58s supply-chain producer to the warm-cache critical path | Rejected |
| Parallelize Fast with supply-chain and secret scan | Removes independent security producer time from PR wall time without removing a gate | May duplicate cold-cache computation and create concurrent cache writes | Selected with exact-SHA proof required |

## Decision Provenance

| Date | Owner | Decision | Evidence / Rationale | Revisit Trigger |
| --- | --- | --- | --- | --- |
| 2026-08-23 | Human App Framework owner | Fix PR pipeline inefficiency without stopping assurance work | Explicit request to inspect pipeline logs and fix inefficiency | Human changes delivery priority |
| 2026-08-23 | Workstream Analyst / Architect route | Start with Fast artifact allowlisting and disabled unused downloads | Pipeline `#434`: 2.5 GiB collected, 719 MiB compressed, about 130s compression; later steps discard it | Exact-SHA remote evidence contradicts the measured bottleneck |
| 2026-08-23 | Coding Agent | Use a named staged artifact with an uncompressed 100 MiB fail-closed ceiling | This guarantees the compressed artifact cannot exceed the target without guessing Bitbucket compression ratios | Required evidence legitimately exceeds the ceiling |
| 2026-08-23 | Workstream Analyst / Integration route | Parallelize the three mutually independent PR evidence producers in the same leaf | Pipeline `#437`: Fast 52m26s, then supply-chain 10m58s and secret scan 2m16s in parallel; no cross-consumer exists | Exact-SHA proof shows cache contention or another regression outweighs the wall-time gain |

## Architecture And Implementation Notes

The Fast wrapper keeps its six existing commands in their existing order. An
exit finalizer always runs a small packager. On success, the packager requires
the core validation, CLI, downstream, docs timing, and Wave 3 summaries to be
green. On failure, it retains whatever allowlisted diagnostic evidence exists
without hiding the original gate exit code.

The packager stages only JSON/JSONL/log/text evidence from approved evidence
roots. It explicitly excludes npm caches, generated package archives, compiled
output, tool installations, and its own staging directory. Every staged file is
rehashed after copying in a temporary sibling outside the upload glob. The
manifest and complete prospective byte count are finalized before the directory
is atomically renamed into place. An over-limit failure retains only the small
diagnostic manifest; the oversized payload never enters Bitbucket's
`capture-on: always` glob. The manifest records both
source and staged paths, byte size, SHA-256, exact source/tested/destination Git
SHAs, their permitted relation, and safe Bitbucket identifiers.

PR steps that clear or do not read inherited evidence use
`artifacts.download: false`. Main/tag release aggregation continues to download
the supply-chain and secret-scan artifacts because those lanes intentionally
consume them.

After the opening fail-closed release-lite guard, Bitbucket starts Fast,
supply-chain/lint, and secret scan in one fail-fast parallel block. The final
release-lite/freshness guard is still sequenced after the group, so it cannot
complete before any producer and rejects proof pinned to a destination that
advanced while they ran.
Both guard steps have five-minute outer backstops. Their remote Git calls
disable terminal prompts. The opening fetch is capped at 240 seconds; the two
serial closing operations are each capped at 120 seconds, so their combined
remote wait remains below the five-minute step backstop. Failed opening and
closing guards retain named scoped `capture-on: always` JSON artifacts. A
failed opening fetch does not compare against a stale local destination or
`HEAD~1`; the closing query always records its bounded outcome in
`remote_query`.
Based on pipeline `#437`, the expected warm-cache critical path changes from
`52m26s Fast + 10m58s supply + 1m18s final guard` to approximately
`max(52m26s Fast, 10m58s supply, 2m16s secret) + 1m18s final guard`, before the
separate artifact-teardown improvement. This is an evidence-based model, not a
claim about the unrun candidate's actual remote duration.

## Security, Privacy, And Governance

The selected files are a strict subset of evidence already uploaded by the Fast
step. No secrets, credentials, tenant data, PHI/PII, or new external egress are
introduced. The secret scan, supply-chain gate, release-lite guards, strict
release gates, and human merge/release authority are unchanged. This Class D
CI/release contract change requires comprehensive independent review and a
human merge decision.

CRM product validation remains a required local Fast success predicate, but
the four product-owned reports under
`examples/products/crm/.appfw/target/appfw` are not staged or uploaded. They
were outside the former `target/appfw/**` artifact and may contain workstation-
absolute roots; excluding them preserves both the strict-subset and no-new-
egress contracts without inventing Product or security classification.

## Acceptance Evidence

| Criterion | Proof Command / Artifact | Required Before |
| --- | --- | --- |
| Named Fast artifact has no broad glob | `node --test scripts/ci/pr-fast-evidence-contract.test.mjs` | push |
| Unused inherited artifacts are disabled only on proven non-consumers | same static contract test plus source review | push |
| Required evidence is exact-SHA and hash-bound | fixture test and `target/appfw/pr-fast-evidence-manifest.json` | push |
| Synthetic merge checkout is bound to exact source and destination parents | focused fixture test | push |
| Destination identity accepts a 12-character prefix but requires exact supplied 40/64-character IDs | focused helper and remote fixtures | push |
| Partial or over-limit staging cannot enter the `capture-on: always` upload glob | behavioral ceiling fixture, atomic-publication static contract, and implementation review | push |
| Destination advancement is rejected after all producers finish | local bare-remote fixture and exact-SHA Bitbucket closing step | merge |
| Opening and closing remote Git operations are bounded and fail closed | hanging/nonzero transport fixtures, structured guard JSON, 240/120/120-second static budget contract, and five-minute step backstops | push |
| Red opening/closing guard evidence is retained | named scoped `capture-on: always` artifact contract and exact-SHA remote red proof | merge |
| Product-owned reports create no new upload surface | absolute-path fixture remains a local predicate but is absent from staged files and manifest | push |
| Cache/transient files are excluded | fixture test | push |
| Successful evidence is no more than 100 MiB uncompressed | packager hard ceiling and manifest `size` block | PR |
| Existing gates remain present with opening/closing guard constraints | static contract test and diff review | push |
| Independent producers share one fail-fast post-preflight parallel group and the closing guard follows it | static contract test and exact-SHA Bitbucket step graph | merge |
| Remote upload completes within 30 seconds | exact-SHA Bitbucket step timing | merge |
| Framework source remains valid | `scripts/appfw framework validate --json` | push |
| CI/docs contract remains valid | shell syntax, focused contract test, risk-appropriate docs-check | push |
| Independent review is current | `scripts/appfw framework handoff --json`; `scripts/appfw framework review-brief --auto-depth --json`; comprehensive Framework PR Review | push |

Successful pipeline `#434` at exact source
`ce7a460a2dfaf4a95f071e30210b884138dabcfe` provides a real Fast-shape
compatibility check for the required manifest fields. Its authenticated
read-only Fast-step log shows the evidence root prepared before gate 1,
framework validation with `valid:true`, `cli-test.json` with `ok:true`,
`golden-downstream.json` with `ok:true`, and full changed-only docs timing with
`ok:true`, `budget_ok:true`, and `budget_enforced:true`. Wave 3 then reused the
same timing/changed-surface artifacts and wrote a green
`wave3-pr-gates.json`. This matches every `REQUIRED_SUCCESS_REPORTS` predicate;
the current fixture suite independently exercises missing/non-green reports,
wrong SHA, dirty source, oversized evidence, and failed-gate retention.

## Test And Execution Feedback Plan

Run the focused fixture/static test first, then shell/Python syntax checks,
framework validation, changed-surface docs-check, handoff, and comprehensive
review. If the focused fixture finds a missing real Fast artifact, update the
allowlist and spec rather than reintroducing a broad glob. If the remote bundle
exceeds the size or upload SLO, inspect the manifest's largest files and remove
only evidence proven redundant; do not raise the ceiling or drop a gate by
default.

The first representative local measurement followed a successful full
changed-surface docs-check and framework validation: the allowlist selected 176
files totaling 3,148,672 bytes (3.003 MiB), while the contemporaneous
`target/appfw` root contained 198 files/about 3.5 MiB. This proves structural
headroom under the 100 MiB ceiling for that evidence set; it is not a substitute
for exact-SHA remote compressed-byte and upload-duration evidence from the full
Fast pipeline.

Pipeline `#437` is also the representative critical-path baseline: Fast
52m26s, supply-chain 10m58s, secret scan 2m16s, and final guard 1m18s. The
candidate must retain all four terminal step outcomes and prove the three
producer steps overlap before the topology benefit is accepted.

## Risks And Controls

| Risk | Control | Owner | Status |
| --- | --- | --- | --- |
| Required diagnostics omitted | Core required files plus fixture/static contract and failure capture | Coding Agent / reviewer | active |
| Wrong-source evidence | Git/Bitbucket SHA equality and per-file hashes | Packager | active |
| Release aggregation accidentally starved | Download disabling limited to non-consumers; main/tag release steps unchanged | Integration / reviewer | active |
| Size target met only by assumption | Hard uncompressed ceiling; remote upload time remains explicitly pending | Integration | active |
| Parallel producers duplicate cold-cache computation or contend on cache save/write | Keep cache topology unchanged, observe exact step/cache logs, and revert only the topology change if the remote critical path regresses | Integration / Workstream Analyst | active |
| Current-main convergence successor drifts before promotion | Recompute the exact base/head, rerun the complete proof and comprehensive review, and never reuse stale evidence | Integration Branch Manager | active |

## Tech Debt And Follow-Up

- Exact-SHA Bitbucket evidence must supply compressed bytes and upload seconds;
  local proof cannot close the remote 30-second SLO.
- Feature-check sharding, cache-key redesign, pinned CI images, duplicate work
  removal, and native test-report coverage remain separate measured lanes.

## Handoff Notes

The fail-fast and artifact leaves have been merge-forwarded into one current-main
successor on `fix/pr-pipeline-throughput-convergence-r2`, based on exact main
`a6d9d41040a32a4ed32a81b521bc113f0f69048c`. The earlier `bc068af8...` base,
branch without the `-r2` suffix, and reviewed candidate
`293f48f7494191feeb57362dd7384071252cc1e8` are historical predecessor state;
the current exact candidate is recorded in handoff and independent review.
Integration must rerun the focused contracts, handoff, comprehensive review,
and one exact-SHA PR pipeline before merge consideration.
The earlier leaf and PR remain recovery/donor state until the successor proves
equivalent remote coverage; they are not independently merge-ready.
