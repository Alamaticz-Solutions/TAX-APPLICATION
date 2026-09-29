# App Framework Branch Convergence Checkpoint — 2026-08-24

**Authority:** current planning and assignment checkpoint
**Terminal history:** [Branch Disposition Ledger](branch-disposition-ledger.md)
**Live main at r2 assignment:**
`a6d9d41040a32a4ed32a81b521bc113f0f69048c`
**Original cohort:** 25 Wayne-directed remote branches
**Latest bound remote evidence:** PR `#484` pipeline `#444` completed
`FAILED` at `2026-08-24T09:16:25.526676273Z` against source
`1f0d03e5bce05f30509f4eb415b159cb3de9a6c4` and destination
`a6d9d41040a32a4ed32a81b521bc113f0f69048c`

This is the sole current planning checkpoint for the 25-branch convergence.
The prior r1 checkpoint is retained by its accepted SHA-256
`b5e0f70a5c1be1159c1efc4f7f50c86462d3015bcde44139ea931b41404fea88`
as provenance; external temp cleanup removed its uncommitted carrier before it
could become a Git object. The durable r2 assignment receipt records the exact
rotation. The terminal ledger remains terminal-only and contains no planned or
in-progress rows. This checkpoint records one active source lane, Worker A;
Workers B and C are frozen with source `0`, and Batch D is `PREP/HOLD` with
source `0`. Remote state not explicitly bound to the timestamp above is
`STALE` or `UNKNOWN` and must not be used as current promotion evidence.

## Burndown

| Starting cohort | Accepted into main | Retired | Live remaining | Active promotion | Scavenge | Hold | Duplicate-ready |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 25 | 2 | 0 | 25 | 1 | 15 | 6 | 1 |

`Accepted into main` is a value state and does not reduce `Live remaining`
until exact authorized carrier cleanup and post-delete verification complete.
The first physical reduction remains 25 to 22 only after the retained Chief
Architect Batch A observation and Wayne's exact three-branch cleanup
authorization. Neither pipeline failure nor recovery coverage grants cleanup.

## Exact Cohort Matrix

| Batch | Branch and exact original tip | Current planning state | Intended outcome / successor |
| --- | --- | --- | --- |
| Immediate | `fix/feature-check-progress-evidence@b9f89994f3364c874d5b3ba9e4dd9c3878562ca5` | accepted into main; cleanup pending | `MERGED`, then retirement tranche |
| Immediate | `fix/main-cargo-fmt-recovery@6273fc44a0c6a3b40d374e190f49f47fda42e4cf` | accepted into main; cleanup pending | `MERGED`, then retirement tranche |
| Immediate | `integrate/fabric-console-framework-foundation-r1@b62c2cef0549556be66aec17fcb3857c7a9b0b0a` | duplicate-ready; cleanup pending | `DEPRECATED` duplicate alias; retained exact payload carrier is `fix/portable-artifact-lock-identity-r1` |
| A | `docs/base-bound-review-authority-r2@4f3e86004436a17b79a335b1fa004986955e0f8b` | retained in failed candidate `1f0d03e5...`; bounded correction active | scavenge pushed planning truth plus separately identified `d6237a12...` narrowing correction into `fix/pr-pipeline-throughput-convergence-r2` |
| A | `fix/nexus-workstream-test-sharding@db0d730d16063859c032554f433528bc8886d8e3` | retained in failed candidate `1f0d03e5...`; bounded correction active | scavenge exact sharding plus separately identified `9acbc938...` fixture dependency into the 36-test Batch A successor |
| A | `fix/pr-fail-fast-preflight@ce7a460a2dfaf4a95f071e30210b884138dabcfe` | retained in failed candidate `1f0d03e5...`; no accepted successor yet | scavenge only after accepted successor and destination evidence |
| B | `docs/fixture-custody-amendment-r1@5b7d64bbfe5462cb6ceb6ae587bcca77fbdf4b7d` | frozen `GO WITH CONDITIONS`; source `0` | retain current non-superseded identity/custody truth in a future Batch B successor |
| B | `docs/ix-linked-recipe-run-lifecycle-r1@28bd88c522998e02c28bd5d43ca7696a9a7c165f` | frozen `GO WITH CONDITIONS`; source `0` | retain bounded linked-run lifecycle truth in a future Batch B successor |
| B | `docs/ix-runtime-convergence-r1@224df583a13a05da5a9ff5b6d91e6dcb8a5a69c1` | frozen `GO WITH CONDITIONS`; source `0` | classify against accepted IX convergence and retain only current docs truth |
| B | `feature/ix-eight-vignette-contract-r1@8e5b33119b84fa23eee1f1d3534481101afed2a9` | frozen `GO WITH CONDITIONS`; source `0` | retain cross-channel contract truth without stale runtime replay |
| B | `fix/portable-artifact-lock-identity-r1@b62c2cef0549556be66aec17fcb3857c7a9b0b0a` | frozen shared carrier; source `0` | retain exact artifact-identity semantics; no duplicate cleanup until successor acceptance |
| B | `integrate/ix-eight-vignette-foundation-r1@bb9b8423bbd72003c050079c3f27a279c08d0fd5` | frozen `GO WITH CONDITIONS`; source `0` | classify mixed carrier; retain current IX/PDS/native seams only |
| B | `integrate/ix-foundation-r1@7339de03acec2aac509ebb7d4b10f5df457441fd` | frozen `GO WITH CONDITIONS`; source `0` | classify mixed carrier against accepted main |
| B | `integrate/ix-runtime-convergence-r2@76e469a695c234f7e5bfe4959b97caaa19e80d55` | frozen aggregate carrier; source `0` | rebuild a final candidate only after conditions, rotation, and explicit assignment |
| B | `integrate/living-intelligence-foundation@559f7d77abef47b55ddc18573a8e47a989b45b73` | frozen durable ledger carrier; source `0` | preserve terminal Living evidence; no reopening or consumption claim |
| C | `feature/living-intelligence-foundation@d7fe1f6df2e5156946a3b9f746ed2928a67a8486` | frozen `NO-GO / NO-CONSUME`; `N0`; source `0` | design input only; terminal private learning evidence remains non-consumable |
| C | `feature/nexus-a0-agentic-runtime@09435de119a97c473f51c9c090bf0a476ab99e4b` | frozen `NO-GO / NO-CONSUME`; `N0`; source `0` | retain rejected candidate as evidence; any salvage requires replan and new admission |
| C | `feature/nexus-demo-act2-slice@1929d7b50ea03f503910b723fdc0cdf8fbe9879b` | frozen `NO-GO / NO-CONSUME`; `N0`; source `0` | future classification only; exclude demo/product-only material |
| C | `integrate/nexus-foundation-r1@460af9af2d1b47721fd1ad34e5868744a3c9e4fa` | frozen `NO-GO / NO-CONSUME`; `N0`; source `0` | future classification only; no shared-seam admission |
| C | `integrate/nexus-product-tooling-r1@9ea26157a2bed29424e1052a909303985ba466bd` | frozen `NO-GO / NO-CONSUME`; `N0`; source `0` | future classification only; product-specific behavior stays out of framework contracts |
| D | `docs/pds-production-maturity-plan@00e53a18a1ef3e4b7dd3b8077528c86eb2c8ed72` | `PREP/HOLD`; source `0` | reconstruct current maturity truth only after all source-start predicates |
| D | `feature/fabric-a6-fleet-experience@ee4aa5a5874328abf63073cbcacd3b97845624d4` | `PREP/HOLD`; source `0` | retain framework/fleet boundary truth only |
| D | `feature/fabric-console-demo-cameo@d3fb6f647ec9d5db1da233974901c2eb5542ab9e` | `PREP/HOLD`; source `0` | classify demo-only versus reusable framework payload |
| D | `feature/fabric-console-scaffold@606c05dc390bac3f0c3a5f76965b53b15ffa6564` | `PREP/HOLD`; source `0` | reconstruct a coherent current-main scaffold successor only after admission |
| D | `integrate/app-fabric-phase-0@208d059c467ed8a89fbcf22874b241ac50f2fd0d` | `PREP/HOLD`; source `0` | preserve App Framework/App Fabric/PDS/product ownership boundaries |

No matrix row is terminal merely because it has an intended outcome. Terminal
`MERGED`, `SCAVENGED`, or `DEPRECATED` records enter the ledger only after the
contract gate passes; `RETIRED` additionally requires authorized cleanup and
post-delete proof.

## Current R2 Source And Frozen State

### Worker A — CI Throughput And Coordination Controls

- Branch: `fix/pr-pipeline-throughput-convergence-r2`
- Worktree:
  `/Users/wayne.kempf/projects/app-framework/target/appfw/worktrees/branch-convergence/batch-a-r2`
- Exact failed parent candidate/tree:
  `1f0d03e5bce05f30509f4eb415b159cb3de9a6c4` /
  `c3dda6969ec5f725cb41635fd225a6e2d120b1f3`.
- Remote evidence: PR `#484` pipeline `#444`, UUID
  `{16516457-7fee-42bf-acda-ed60d375c611}`, completed `FAILED` at
  `2026-08-24T09:16:25.526676273Z`. Opening release-lite, preflight,
  supply-chain/lint, and secret-scan steps were green; Fast failed one
  contract fixture because its subprocess inherited the real destination
  branch/commit while overriding the source commit with a fixture-only SHA;
  the closing guard was `NOT_RUN`.
- Current source state: `1` active producer, limited to fixture-environment
  isolation plus this checkpoint refresh. The correction must not weaken the
  evidence packager's identity checks or any CI gate. This revision is part of
  the prospective descendant; its exact commit/tree must be bound externally
  by handoff and independent review because a Git-tracked file cannot safely
  embed the identity of the commit that contains itself.
- Merge state: no merge is claimed. PR, source-ref, and destination-ref state
  after the exact pipeline completion above are `UNKNOWN` until a fresh remote
  read. A new exact-source pipeline and fresh independent review are required.
- Roots, tests, reviewer, and handoff remain bounded by
  [Batch A Convergence Assignment Receipt — R2](branch-convergence-assignment-receipt-2026-08-24.md).

### Worker B — Frozen Conditional Candidate

- Branch: `integrate/ix-pds-native-convergence-r2`.
- Exact candidate/tree:
  `06b90583e1a1b726e630ab1a4a089fd96cd6cbe2` /
  `94d233ed019d1c2c1cc1f2d71bc329be36a93785`.
- Condition packet: `target/appfw/branch-convergence/`
  `batch-b-condition-closure-and-gate2-packet.md`, SHA-256
  `1269d9f21f3e654b2e3d36e8b14e4ee5d874e6b57baf79397e9376fa515a0cd1`.
- Comprehensive review: SHA-256
  `c350925f4c35faeb2172ea13927d7a48f16a62ad41e1675e13ede077b550b27f`,
  `GO WITH CONDITIONS`, counts `0/0/4/2/0`.
- Current state: immutable reviewed checkpoint, source `0`; no writer, push,
  PR, merge, Product/WIP admission, or shared-seam ratification follows from
  the review or packet.

### Worker C — Frozen Rejected Candidate

- Branch: `feature/agentic-runtime-foundation-r2`.
- Exact candidate/tree:
  `ef47c731bdf708212d5f6be8acc8ccb723df7cee` /
  `c51fa7fdb8b5bce6ef44e3df44bb533f53299b31`.
- Corrected read-only plan: `target/appfw/branch-convergence/`
  `batch-c-r2-nogo-correction-plan.md`, SHA-256
  `f1d941948c466d8a30bf08370d78015e3877cb7bb8400ff24fbe354e05436d7d`.
- Comprehensive review: SHA-256
  `1edcd2c9415d128ff7a45a0500ceafa4b27ff05c8962c201787f890872025890`,
  `NO-GO`, counts `0/1/6/1/0`.
- Current state: `NO-GO / NO-CONSUME`, `N0`, source `0`; the candidate remains
  immutable rejected evidence. No donor consumption, source/WIP admission,
  push, PR, or correction iteration is implied.

### Batch D — Read-Only Preparation Hold

- Current packet: `target/appfw/branch-convergence/`
  `batch-d-gate2-scope-split-prep-hold.md`, observed locally at SHA-256
  `c3a437198cae099d50b3853afb237045855c7d7f2b2932da740094a564a09cd8`.
- Current state: `PREP/HOLD`, source `0`; no branch, worktree, writer, or WIP
  admission is created by the packet.
- Worker A may rotate only after an accountable human merges Batch A, exact
  post-merge destination evidence is green, a fresh accepted-main collision
  manifest is retained, and the Program Flow Controller issues an explicit
  assignment/lease. Gate 2, Product, Architecture, Security, and donor
  dispositions remain separately owned decisions.

Worker A is the sole source-producing lane. Workers B and C and Batch D each
remain source `0`; available capacity, a retained carrier, or a planning packet
does not create admission.

## Recovery And Custody

- Complete-history recovery bundle:
  `/Users/wayne.kempf/Downloads/app-framework-recovery-20260824T040421Z/app-framework-all-refs.bundle`
- SHA-256:
  `744b513b7a582b31631525692706879c8258b321a3dbd353f4b73c3e91d7929d`
- `git bundle verify`: passed; all 25 exact cohort tips, including
  `db0d730d...`, are present.
- Volatile-carrier incident:
  `target/appfw/branch-convergence/private-tmp-producer-loss-20260824.md`
  at SHA-256
  `9f7aa88ce9757a86af06b3bdd822a5bdadac9755bfd2dbdca959f8b63f814d06`.
- Worker C root-custody incident:
  `target/appfw/branch-convergence/worker-c-root-custody-drift-20260824.md`
  at SHA-256
  `a303106c392c865e8e0f456c5cea15c83a52624ed9222dea901bc1193fb99ee2`;
  all frozen protected-root hashes were exactly restored.
- Immediate retirement preflight:
  `target/appfw/branch-convergence/immediate-retirement-preflight-20260824.md`
  at SHA-256
  `47c1a8ea4784eb1d83b2aadb5c35a307cd6221a24f7fe9d77834fec74cc21254`;
  the refreshed receipt records the absent-but-prunable merged-candidate
  worktrees and binds the complete `744b513b...` recovery bundle.

Recovery material is preserved. This checkpoint grants no archive upload,
carrier deletion, ref pruning, worktree removal, stash mutation, or garbage
collection authority.

## Gates And Parked Decisions

1. **Chief Architect gate 1 — Batch A freeze:** historically `OBSERVED` against
   exact candidate `1f0d03e5...` as a pipeline-contract, recovery, and
   immediate-retirement checkpoint. The retained exact transcript binding is
   `target/appfw/branch-convergence/`
   `chief-architect-gate1-observation-20260824.md`, SHA-256
   `7c18dabc7567bf8bc98b48a71f1fd2d15f8b21c945b166a450aa8038992959bc`.
   That observation granted no merge, release, risk, or cleanup authority.
   Pipeline `#444` later failed the one fixture-isolation test recorded above;
   the bounded correction does not re-wake Gate 1, but it does require a fresh
   exact-SHA handoff, comprehensive review, Integration challenge, and CI.
2. **Chief Architect gate 2 — Shared B/C seam freeze:** identity, permissions,
   events, IX/PDS/native, provider, Kafka/MCP, and framework/product boundaries.
3. **Chief Architect gate 3 — Final convergence/certification freeze:** cohort
   completeness, downstream upgrade path, enterprise evidence, and safe
   retirement.

Parked Product decision: registering
`AFS-PI-BASE-BOUND-REVIEW-01` in
`docs/specs/product-increment-portfolio.json` would change canonical Product/WIP
planning truth. Neither Worker A nor Integration has that authority. The plan
and governing spec remain held with zero delivery credit; any registration or
status/count/timestamp reconciliation requires an explicit Product-owner
decision. An exact docs/governance failure caused solely by this missing
registration is evidence for that decision, not permission to weaken the check.

Parked cleanup decision: the retained Gate 1 observation is historical
architecture evidence only. The immediate three-branch tranche remains live
until Wayne gives exact destructive-cleanup authority and the Integration
Branch Manager retains branch-specific no-loss and pre/post cleanup proof.

## Role Card Check

- **Card used:** Integration Branch Manager for the controlling checkpoint;
  Framework Implementation Agent / Coding Agent for the sole bounded Worker A
  correction.
- **Work within role:** exact inventory and custody truth, one-producer state,
  the two-path Worker A correction, and proof/review routing.
- **Authority not assumed:** direct main push, merge, release, Product or
  security decision, accepted risk, branch/PR/worktree deletion, or recovery
  disposal.
- **Decisions routed:** Product portfolio registration to the Product owner;
  three Chief Architect observations to their named gates; merge and cleanup
  tranches to Wayne.
- **Drift signal:** pipeline `#444` exposed ambient destination identity leaking
  into a fixture-only packager subprocess, and independent challenge SHA-256
  `d6a0a6d860c5a4a973ae02ffce1974650e1938e4b45dc15e316e0b2707c084d4`
  exposed stale A/B/C/D state in this sole checkpoint. Both are bounded by the
  current correction; no competing checkpoint, ledger, source assignment, or
  merge-readiness claim is authorized.
