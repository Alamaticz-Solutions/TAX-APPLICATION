# Batch A Convergence Assignment Receipt — R2

**Assignment state:** active local reconstruction; no remote authority
**Assigned by:** App Framework Integration Branch Manager
**Sole source producer:** Worker A — Coding Agent
**Reviewer:** independent Framework PR Review Agent after immutable candidate freeze
**Handoff destination:** App Framework Integration Branch Manager
**Recorded:** 2026-08-24

This receipt supersedes the volatile r1 assignment whose uncommitted draft was
lost during external `/private/tmp` cleanup. The prior authoritative checkpoint
is retained as provenance by SHA-256
`b5e0f70a5c1be1159c1efc4f7f50c86462d3015bcde44139ea931b41404fea88`;
it is not claimed to be byte-recreated here. The cleanup incident is retained at
`target/appfw/branch-convergence/private-tmp-producer-loss-20260824.md`
(SHA-256
`9f7aa88ce9757a86af06b3bdd822a5bdadac9755bfd2dbdca959f8b63f814d06`).
Recovery and immediate-retirement custody are additionally bound to complete
bundle SHA-256
`744b513b7a582b31631525692706879c8258b321a3dbd353f4b73c3e91d7929d`
and refreshed preflight SHA-256
`47c1a8ea4784eb1d83b2aadb5c35a307cd6221a24f7fe9d77834fec74cc21254`.

## Exact Carrier

- Successor branch: `fix/pr-pipeline-throughput-convergence-r2`
- Persistent worktree:
  `/Users/wayne.kempf/projects/app-framework/target/appfw/worktrees/branch-convergence/batch-a-r2`
- Frozen assignment base/starting HEAD:
  `50e13d85c7f085e0e56e89ce3e91b2a955ebf3a8`
- Accepted main included by that HEAD:
  `a6d9d41040a32a4ed32a81b521bc113f0f69048c`
- Inherited committed throughput payload: 21 paths; preserved, not reassigned
  to another producer.

## Payload Sources

- `fix/pr-fail-fast-preflight@ce7a460a2dfaf4a95f071e30210b884138dabcfe`
  — already patch-equivalent in the inherited throughput payload.
- `fix/nexus-workstream-test-sharding@db0d730d16063859c032554f433528bc8886d8e3`
  — bounded fail-closed Nexus authority-test sharding.
- `docs/base-bound-review-authority-r2@4f3e86004436a17b79a335b1fa004986955e0f8b`
  — pushed planning donor.
- `d6237a1230ef21122299ec4d527ecfa6b9ddf9f2`
  — approved local-only narrowing correction, retained with separate
  provenance and no remote-branch claim.
- `9acbc9383b2b5f0f780d4c2d88830ac25b81debd`
  — approved local-only fixture dependency, retained with separate provenance.

The sharding and fixture payloads are reconciled as one exact 36-test contract.
No test, TAP validation, failure propagation, cleanup assertion, or authority
check may be removed to make the aggregate pass.

## Rejected Candidate And Correction Routing

The first reconstructed candidate
`0c2d10300227e88a4b9e157ff6871412f82ba3a2` (tree
`928906ad5df224249ea937d56a2e5cbe9dc54e55`) is retained as rejected
provenance. Independent Framework PR Review Agent output SHA-256
`b14846deb0545bb3f2a0ea536b3de7b02f9997c59faa1a6e9fd337feeea92ea9`
returned `NO-GO` with 1 blocker, 0 critical, 4 important, 1 should-address,
and 0 nice-to-address findings. It grants no push or PR authority.

Integration routed one bounded correction to Worker A: register every
run-owned fixture under the same run-scoped cleanup namespace; make cleanup,
source binding, terminal verdict, and one atomic summary publication one
fail-closed lifecycle; retain lifecycle-level success and injected-cleanup-
failure tests; and make the current spec consistently govern 36 tests. The five
review-observed leaked roots remain preserved for their external cleanup owner.

The optional `.bitbucket/pull_request_template.md` policy surface is deferred
because its repository-wide source owner is unknown. The correction restores
the path to accepted-base bytes, adds no ownership rule, and makes no silent
classification change. Any future template requires explicit accountable-owner
authorization outside this correction.

Wayne explicitly authorized removing this unowned template from the Batch A
candidate. That direction authorizes only this accepted-base restoration; it
does not assign ownership of the path or grant branch cleanup, merge, release,
Product, security, or risk authority.

The subsequent adversarial review of the clean `dd9bc465...` checkpoint found
six further evidence-integrity corrections within the inherited Batch A CI and
runner payload: complete `scripts/ci/**` release-lite classification; rooted
incomplete/malformed conflict rejection; non-authoritative hook/alternate-test
runner summaries with canonical test-blob binding; always-captured red
release-lite artifacts under bounded step budgets; six-run evidence custody
outside disposable `target/appfw`; and removal of newly uploaded CRM reports
that contradicted the strict-subset/no-new-egress contract. Integration assigned
only those fail-closed corrections and their exact tests/docs truth to Worker A.

## Authorized Write Roots

- `bitbucket-pipelines.yml`
- `.bitbucket/pull_request_template.md` (correction restores accepted-base
  bytes; future policy work deferred pending accountable ownership)
- `docs/archive/branch-convergence-assignment-receipt-2026-08-24.md`
- `docs/archive/branch-convergence-checkpoint-2026-08-24.md`
- `docs/archive/branch-disposition-ledger.md`
- `docs/release/release-gate-ci-cd.md`
- `docs/specs/README.md`
- `docs/specs/branch-disposition-and-scavenging.md`
- `docs/specs/nexus-workstream-test-throughput.md`
- `docs/specs/pr-fail-fast-preflight.md`
- `docs/specs/pr-fast-evidence-artifact-allowlist.md`
- `docs/start/agent-role-cards.md`
- `docs/start/branch-integration-model.md`
- `docs/start/tech-debt-register.md`
- `scripts/check-doc-examples.sh`
- `scripts/check-nexus-workstreams.test.mjs`
- `scripts/product-increment-path-safety.mjs`
- `scripts/run-nexus-workstream-tests.mjs`
- `scripts/run-nexus-workstream-tests.test.mjs`
- `scripts/ci/package-pr-fast-evidence.py`
- `scripts/ci/pr-destination-freshness.py` (classification/test context only;
  no source mutation currently required)
- `scripts/ci/pr-fast-evidence-contract.test.mjs`
- `scripts/ci/pr-preflight.py`
- `scripts/ci/pr-preflight.test.sh`
- `scripts/ci/release-lite-guard.sh`
- `docs/specs/afs-base-bound-review-evidence.product-increment.json`
- `docs/specs/base-bound-handoff-review-evidence-r1.md`
- `docs/specs/product-increment-portfolio.json`
- `scripts/check-product-increment-portfolio.test.mjs`

No canonical Product/WIP admission, lifecycle, priority, active/review count, or
timestamp in `docs/specs/product-increment-portfolio.json` is authorized by
this assignment. If the reconstructed plan requires registration, the exact
failing check and Product-owner decision remain parked; neither the test nor
the portfolio may be falsified to manufacture green evidence.

## Acceptance And Stop Conditions

Required local proof is: the focused 36-test sharding contract, portfolio and
Nexus fixture regressions, Framework validation, governance/docs-check,
generate-check, fast Framework tests, Framework handoff, and auto-depth review
brief. A clean exact path set and `git diff --check` are mandatory before local
commit. After the correction commit, three hook-free canonical serial and three
hook-free canonical four-shard runs must retain raw hash-indexed evidence under
`target/appfw-convergence-evidence`, outside the disposable CI evidence root.
The cleanup contract in `scripts/ci/prepare-appfw-evidence-root.sh` removes only
the allowlisted `target/appfw` path and cannot reach this sibling; no broad
`target` cleanup is authorized for the retained six-run proof.
Any unassigned path, branch/base drift, non-Product failure, or attempt to
mutate another producer's carrier is a stop condition.

Local commits are permitted only after scoped proof. This receipt grants no
fetch, push, PR, merge, retirement, cleanup, release, Product, security, or
risk authority.

## Role Card Check

- **Card used:** Coding Agent, Worker A.
- **Work within role:** exact donor reconstruction, bounded source changes,
  focused proof, handoff, and immutable candidate preparation.
- **Authority not assumed:** Product/WIP decisions, remote mutation, merge,
  release, branch retirement, cleanup, security, or risk acceptance.
- **Decisions routed:** base-bound plan portfolio registration remains with the
  Product owner; promotion and cleanup remain with Integration and Wayne.
- **Drift signal:** r1 volatile carrier loss is contained by this persistent r2
  carrier; no competing Worker A producer is authorized.
