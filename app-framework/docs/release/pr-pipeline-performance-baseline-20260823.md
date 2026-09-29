# PR Pipeline Performance Baseline — 2026-08-23

Status: durable sanitized measurement snapshot; successor remote proof pending

This document preserves the measurements used to design the fail-fast,
bounded-artifact, and producer-parallelism changes. It is evidence for the
design decision, not release authority and not proof that the unrun successor
has improved remote latency.

## Source Evidence Identity

- Original retained analyst artifact:
  `target/appfw/bitbucket-pipeline-performance-assessment-20260823.md`
- Original artifact SHA-256:
  `35281869b9e1f384da7f73b290f46cbe03c8a64c3e0664109d665d10b8a38390`
- Observation window: Bitbucket pipelines `#332` through `#431`, from
  `2026-07-26T02:04:03Z` through `2026-08-23T01:54:37Z`
- Original remote baseline:
  `origin/main@d55002d4e12a572ff3cf6215defc91000274dbaf`
- Method: authenticated read-only Bitbucket REST metadata and sanitized
  terminal step logs using the repository's documented auth workflow. No
  credential, authorization header, raw secret-bearing log, tenant data, or
  approval assertion is retained here.

The ignored `target/appfw` artifact was hashed before this tracked snapshot was
written. This file carries the exact pipeline/source identities and decision
measurements needed by future reviewers so the spec no longer depends on an
absent workstation-local path.

## Sampled Baseline

| Signal | Observed |
| --- | ---: |
| Pipelines sampled | 100 |
| Successful / failed / stopped | 40 / 57 / 3 |
| Developer wall time represented | 72.9 hours |
| Bitbucket compute consumed | 112.0 hours |
| Failed-run compute | 45.3 hours / 40.4% |
| Initial-start-delay median / p95 / max | 18s / 27s / 33s |
| PR duration p50 / p90 / max | 47m39s / 65m19s / 80m28s |
| Successful PR p50 / p90 | 50m38s / 63m45s |
| `Fast framework check` p50 / p90 / p95 | 42m43s / 55m08s / 57m32s |

The latest 100 runs showed no chronic runner-queue problem. The median first
step began in 18 seconds and p95 in 27 seconds. Most latency was inside the
repository's serial gates, repeated compilation, and artifact/cache movement.

## Exact Terminal Observations

### Pipeline `#434`

- Exact source:
  `ce7a460a2dfaf4a95f071e30210b884138dabcfe`
- Preflight: 22 seconds / 21 build seconds, green, seven JUnit cases ingested
- Fast: 2,857 seconds
- Supply-chain/lint: 617 seconds
- Secret scan: 159 seconds
- Final guard: 78 seconds
- Fast teardown: 4,749 files / about 2.5 GiB compressed to 719 MiB in about
  130 seconds

This run proved that the early preflight works but that the old broad Fast
artifact and serial Fast-then-supply topology dominate the later path.

### Pipeline `#437`

- Exact source:
  `5e01d85cbe21d92177ddf8b479ff7b5d9264e385`
- Fast: 52m26s
- Cargo registry and 3 GiB target-cache download/extraction: 3m04s
- Framework validate: 4m24s
- Framework CLI test: 6m11s
- Golden downstream: 5m19s
- Changed-only docs check: 2m07s
- Wave 3: 22m52s
- `feature-check` within Wave 3: 21m38.302s
- Framework fast test: 4m28s
- Broad evidence compression/upload: 2m27s
- Supply-chain/lint after Fast: 10m58s
- Secret scan parallel with supply: 2m16s
- Final guard: 1m18s

The three producer lanes do not consume one another's evidence. This exact log
made the selected parallel group an evidence-based critical-path correction,
while preserving all three outcomes.

### Pipeline `#440`

- Exact source: `6273fc44...`
- Terminal status: `SUCCESSFUL`; total 3,397 seconds
- Fast: 44m07s
- Fast target-cache restore: 3 GiB; 56s download + 118s extraction
- Fast gate script: 37m54s
- Fast artifact: 4,611 files / about 2.5 GiB to 718.7 MiB; 100s compression,
  4s upload
- Supply: 10m52s, including download/extraction of the unused 718.7 MiB Fast
  artifact and a separate 3 GiB target-cache restore
- Secret: 2m37s, also after downloading/extracting the unused Fast artifact
- Closing guard: 72s, also after downloading/extracting the unused Fast
  artifact

This second terminal run confirmed that supply, secret, and closing-guard steps
received an artifact they immediately cleared or never consumed.

## Non-Terminal Current-Main Observation

Pipeline `#442` is deliberately recorded only as an in-progress observation:

- Source: `b9f89994f3364c874d5b3ba9e4dd9c3878562ca5`
- Destination: `bc068af8f46b0fea2277ce828b49c9544dd45f34`
- Fast step UUID: `d2cddc3d-f6f5-4df8-8bbb-bb3b321e57e0`
- Fast started: `2026-08-23T02:12:58Z`
- Validate marker: `02:17:50Z`
- CLI-test marker: `02:21:11Z`
- Golden second-consumer marker: `02:26:20Z`
- Docs marker: `02:31:09Z`
- Wave 3 / feature-check marker: `02:33:04Z`
- Full docs-check: green in 114,555ms; slowest example
  `product-increment-assignment-topology-tests` at 25,502ms
- Feature-check was still running at the `02:35:04Z` 120-second heartbeat

No terminal duration, success verdict, or successor improvement is inferred
from this observation.

## Main-Certification Boundary

Main pipeline `#441` at exact
`bc068af8f46b0fea2277ce828b49c9544dd45f34` completed `SUCCESSFUL` in 3,309
seconds. Its sanitized terminal summary was hashed as
`9e2c1bab16d94ce125377a192cc4d850622ee4305d726b3e2c93d1dde413d082`;
the retained raw terminal log was independently hashed as
`cb455df5114ccd4d8bf443ba94d2ecbf666d620a930bc5f569a80f361d187e68`
without copying raw log content into source.

Supply-chain/lint ran for 595 seconds in parallel with the 102-second secret
scan. The focused release gate then ran for 2,714 seconds, including 34m49s for
the provider-backed focused release command. It restored a 3 GiB Cargo target
cache and 1.3 GiB release-backend cache, then collected 4,419 evidence files /
about 2.4 GiB and compressed them to 689.3 MiB in 108 seconds.

That broad four-provider, security, operations, performance, and handoff
certification is intentionally outside this PR successor. The successor
optimizes the pull-request path only; it does not remove or narrow main or
release assurance. Main cache/evidence optimization remains a separately
specified and reviewed follow-up.

## Decision Boundary And Required Successor Proof

The measurements support three bounded changes:

1. return deterministic failures through a cache-free preflight;
2. replace the broad Fast artifact with a manifest-inclusive, hash-bound,
   maximum-100-MiB allowlist and disable downloads only for non-consumers; and
3. overlap independent Fast, supply-chain, and secret-scan producers while
   keeping the closing release-lite/destination-freshness guard after all three.

They do not prove the successor's remote improvement. Before merge, one exact
successor Bitbucket PR pipeline must retain:

- source, tested checkout, destination, and merge-base identity;
- all three producer outcomes and actual time overlap;
- artifact manifest, uncompressed/compressed bytes, and upload duration;
- fail-fast cancellation behavior when a producer fails;
- bounded opening and closing destination queries;
- closing destination freshness; and
- cache restore/save evidence sufficient to detect a contention regression.

If exact remote evidence shows cache contention erases the benefit, revert or
narrow only producer overlap; do not remove assurance gates.
