# Base-Bound Handoff And Review Evidence R1

Status: correction assembled in Batch A; source held until Product-authorized
portfolio registration, accepted-main planning authority, and external admission

Spec depth: full

Owner roles:

- Product / outcome owner: Program Product Manager
- Program Flow Controller: Nexus Program Flow Controller
- Architect: Framework CLI Architect
- Implementation owner: Framework Review Harness Coding Agent
- Integration owner: IX Evidence Scope Integration Branch Manager
- Review owner: Independent Framework Review Harness Reviewer

## Business Value

App Framework delivery depends on a human and an automated pre-push guard being
able to answer one question reliably: did the independent review inspect the
same immutable source range and changed-surface state that is about to leave the
workstation?

That answer is currently unreliable for a branch whose ratified base is not the
moving `origin/main` ref. Correct base binding unblocks honest checkpoint
delivery for the shared eight-vignette foundation and prevents future Framework
and product lanes from gaining a green guard by pairing a narrow review with a
broader, differently based handoff.

## Problem

The pushed IX shared-foundation aggregate checkpoint
`1e919fafe783b68cef4abc65f99d585be482b4e7` was admitted and independently
reviewed from immutable base
`9ea26157a2bed29424e1052a909303985ba466bd`. That exact range contains 59 changed
paths. The current handoff producer instead calculates branch changes from
`merge-base origin/main HEAD`, yielding 145 paths because `origin/main` and the
ratified integration lineage diverged.

The mismatch is structural:

- `scripts/appfw framework handoff` and its internal `handoff-snapshot` reject
  all additional arguments;
- `app_gen/src/bin/appfw_introspect.rs` hardcodes `origin/main` in the branch
  diff producer;
- `review-brief --base <sha>` can select the correct 59-path scope, but its
  no-write handoff snapshot still uses the hardcoded scope;
- the pre-push guard invokes `review-brief` without an exact base; and
- the review parser verifies headings, status, counts, conditions, freshness,
  and handoff state, but does not verify that the review output names the same
  base, head, or changed-surface binding as the review brief.

As a result, a default 145-path handoff can be mechanically current while the
retained review explicitly says it inspected only the 59-path ratified range.
That parser result is not trustworthy push evidence even though the current
guard can report green.

## Goals

- Derive the handoff base from externally anchored machine-plan authority for
  planned Delivery Lane and integration work, where that plan is read from one
  immutable accepted-main portfolio snapshot rather than producer `HEAD`;
  retain caller `--base` only as an equality assertion, never as a free range
  selector.
- Preserve a safe unplanned/default route that resolves
  `merge-base origin/main HEAD` once when no Product Increment scope is
  asserted.
- Carry one deterministic, versioned review-scope binding through handoff,
  `review-brief`, independent review output, and the pre-push guard.
- Make the guard fail closed when base, head, changed-surface state, or review
  binding differs, even when the Markdown headings and timestamps otherwise
  look current.
- Keep destination freshness distinct from checkpoint scope: a branch may be
  truthfully checkpoint-reviewed from an older ratified base without being
  described as merge-ready against current `main`.
- Preserve existing content-bound handoff protection for file bytes, modes,
  symlink targets, additions, deletions, renames, and untracked files.
- Keep live Program Flow assignment and exclusive-lease currency as an external
  source-admission prerequisite. This capability verifies Git-backed scope
  integrity; it does not accept or verify a caller-supplied assignment identity.

## Non-Goals

- No bypass, `--no-verify`, local remote-ref rewrite, or weakening of the
  existing pre-push policy.
- No authorization to push, merge, release, publish packages, approve SRA/CAB,
  or accept risk.
- No automatic discovery that guesses among multiple Product Increment plans.
- No local plan, source-branch commit, assignment UUID, local assignment file,
  or local remote-tracking ref can grant or expand source authority.
- No change to Product Increment delivery credit, IX recipe provenance, Nexus
  adoption, browser/device qualification, or live-provider readiness.
- No claim that checkpoint review makes a stale branch merge-ready for `main`.
- No redesign of the independent reviewer or conversion of Markdown review
  judgment into self-authored approval.
- No trusted assignment-receipt lookup, live lease adapter, assignment freshness
  check, or stale/superseded-assignment machine claim.

## Scope

The implementation lane is limited to the handoff producer, shared CLI review
brief, Framework pre-push guard and hook plumbing, semantic regression tests,
and the Framework review-harness guidance/skill. This spec and its executable
plan are external control-plane records and are deliberately excluded from the
producer lane's write roots. The Integration Branch Manager owns the shared
CLI reference and docs-check examples; future changes to this spec, plan,
portfolio entry, or spec index require a separately admitted PFC/Architecture
planning change from accepted `main`. The initial corrected records may travel
in the independently reviewed Batch A planning/control successor with the two
owner-originated fixture repairs; that aggregate remains separate from the
future implementation lane and grants no Product registration authority.

Product handoff and product `review-brief` use the same underlying producer and
must remain compatible, but this Product Increment does not introduce a new
product push policy. Nested-product fixtures are required regression coverage
because a shared producer must not regress product-root normalization.

## Repository Context

- `app_gen/src/bin/appfw_introspect.rs` owns the versioned handoff report,
  branch/status surface collection, and content-bound state fingerprints.
- `scripts/appfw` owns `review-brief`, its no-write handoff snapshot, review
  output parsing, and pre-push status calculation.
- `scripts/ci/pre-push-review-guard.sh` invokes auto-depth review-brief and
  projects the final guard result. The accepted plan additionally requires an
  explicit comprehensive brief and `/framework-pr-review --comprehensive`;
  current auto-depth classification alone is not evidence of that depth.
- `scripts/git-hooks/pre-push` already binds the pushed local ref and SHA to the
  checked-out branch; that protection remains.
- `scripts/cli-semantic-gates-test.sh` contains nested-product, special-path,
  symlink, mode, deletion, backdated-byte, and review-freshness regression
  fixtures.
- `docs/reference/cli.md`, `docs/start/pr-review-agent-harness.md`,
  `docs/start/agent-role-cards.md`, and
  `agent_skills/framework-pr-review/SKILL.md` are counterpart contracts.
- The controlling Product Increment plan is
  `docs/specs/afs-base-bound-review-evidence.product-increment.json`, selected
  only through its registration in an externally pinned accepted-main
  portfolio snapshot.

## Contracts Touched

| Contract Surface | Expected Change | Counterpart Surfaces That Must Stay Aligned |
| --- | --- | --- |
| Framework/product handoff CLI | Accept an immutable accepted-main authority SHA supplied after external admission plus Product Increment and lane/integration identity; resolve the portfolio and registered plan blobs from that commit, validate them, derive the only authorized base from that scope, and treat optional `--base <full immutable SHA>` as an equality assertion | introspection producer, CLI reference, canonical plan validator, semantic fixtures, handoff JSON |
| Handoff JSON | Add a versioned review-scope binding containing accepted-main portfolio authority, exact portfolio/plan blob digests, scope/write-root identity, exact base/head, and deterministic digests of normalized changed-surface state | no-write snapshot, review brief, review output contract, guard |
| Review brief | Generate its no-write snapshot from the same accepted-main portfolio snapshot and effective base and fail when retained handoff authority/binding differs | authority/scope arguments, `--base` assertion, `changed_files_for_handoff`, freshness checks, recommended prompt |
| Independent review output | Require explicit machine-readable `Reviewed base authority`, `Reviewed base`, `Reviewed head`, and `Review scope binding` fields copied from the review brief | review skill, harness docs, parser, guard summary |
| Pre-push guard | Re-read the retained accepted-main portfolio and registered plan blobs, use their immutable base, and reject missing, changed, narrowed, locally substituted, or mismatched authority/review binding | pre-push hook ref/SHA checks, external Program Flow admission prerequisite, standing approval policy, retained guard JSON |
| Destination freshness | Report current-main divergence separately without changing the reviewed checkpoint range | Integration readiness summary, PR/full candidate proof |

## Versioned Binding Contract

Handoff and handoff-snapshot must emit an additive record with schema
`appfw.review_scope_binding@1`. The record contains at least:

- `base_authority`: a canonical object with exact keys `kind`
  (`accepted_main_product_increment_lane`,
  `accepted_main_product_increment_integration`, or `default_origin_main`),
  `accepted_main_sha`, `portfolio_path`, `portfolio_blob_oid`,
  `portfolio_sha256`, `product_increment_id`, `scope_id`, `branch`,
  `plan_path`, `plan_blob_oid`, `plan_sha256`, and
  `write_roots_sha256`; `scope_id` is the lane ID for a leaf and integration
  branch for an aggregate, while accepted-main/portfolio/plan fields are `null`
  only for the unplanned/default route;
- `base_authority_sha256`: a SHA-256 digest over the canonical authority
  object;
- `base_sha`: the exact 40-character lowercase commit used as the beginning of
  the branch diff;
- `head_sha`: the exact commit being handed off;
- `surface_state_sha256`: a SHA-256 digest over a documented deterministic
  serialization of the schema identifier, base, head, and sorted normalized
  surface signatures/content-bound state records; and
- `binding_sha256`: a SHA-256 digest over the complete versioned binding record
  excluding the digest field itself.

The implementation may choose field placement in the additive JSON contract,
but it must use one producer for written handoff and no-write snapshot so their
values cannot drift. Existing consumers may ignore the additive record; review
freshness and pre-push approval may not.

### Base Authority Resolution

Planned work must name one immutable accepted-main snapshot and one scope. The
CLI must expose this as `--authority-commit <full-sha> --product-increment <id>
--lane <id>` for a leaf or `--authority-commit <full-sha>
--product-increment <id> --integration` for the aggregate. External Program Flow
must first establish a current assignment and exclusive lease that select that
accepted-main SHA and scope, but the CLI neither accepts an assignment UUID nor
claims to verify the assignment or lease. The authority commit is the exact
accepted-main SHA observed and fetched by Integration. The CLI must not accept a
caller-selected plan path or guess among plan files.

Before collecting changed surfaces, the authority resolver must:

1. read `<authority-commit>:docs/specs/product-increment-portfolio.json` from
   Git object storage;
2. require the named Product Increment to be registered and source-admissible
   there, and take `plan_ref` only from that portfolio entry;
3. read `<authority-commit>:docs/specs/<plan_ref>` from Git object storage and
   run the exported canonical Product Increment validator against those exact
   bytes; and
4. compute and retain the accepted-main commit, Git blob IDs, byte digests,
   scope identity, authorized branch/base, and ordered write-root digest.

Only then may it enforce one of these routes:

1. A planned Delivery Lane derives `base_sha` only from the selected
   `lane.base_sha`; the checked-out branch must equal `lane.branch`.
2. A planned integration aggregate derives `base_sha` only from
   `integration.base_ref`; the checked-out branch must equal
   `integration.branch`.
3. An unplanned/default invocation with no plan scope derives `base_sha` only
   by resolving `merge-base origin/main HEAD` once.

Producer `HEAD`, a worktree plan, and a caller-supplied assignment identity are
never machine authority.
The source lane does not own this spec or plan; if either appears in the
producer diff, changed-path authorization fails. An optional `--base` must
equal the base already derived from the accepted-main plan blob; it cannot
choose another ancestor. Missing, ambiguous, invalid, portfolio/plan
unregistered, branch-mismatched, out-of-root, or digest-mismatched authority
fails closed.

The accepted-main SHA need not remain the moving `origin/main` tip. A later
current remote observation is compatible with the same scope binding only when
it descends from that SHA and the authoritative portfolio entry and plan blob
remain byte-identical. Pre-push scope integrity is unsatisfied when the remote
authority observation is stale or unknown. Any plan/portfolio change or
non-descendant observation invalidates the machine scope and requires a new
externally admitted accepted-main SHA; a producer cannot refresh its own
authority. Assignment and lease currency are checked outside this capability.

The authorized base must be an ancestor of the selected head for report-only
diagnostics and a **strict** ancestor for pre-push satisfaction. Equality (for
example, an asserted or derived base of `HEAD`) may produce an explicit
no-change diagnostic but can never satisfy the pre-push guard. This strictness
alone is not the authorization control: a later intermediate ancestor also
fails unless it equals the accepted-main-plan-derived base.

This preserves default-main no-op compatibility: an unplanned handoff on a
checkout where the resolved merge base equals `HEAD` may still emit a
successful empty-surface diagnostic and binding, but its pre-push projection is
unsatisfied because there is no source update to authorize.

At guard time, the retained accepted-main SHA, portfolio blob, registered plan
blob, and scope are revalidated and resolved again. Their authority digest,
base, authorized roots, head, surface digest, and complete binding must match
the current checkout and independent review; a reviewer-visible note cannot
override a mismatch. The guard does not infer that this proves a current PFC
assignment or exclusive lease.

Review output must contain these exact fields in its Recommendation Summary:

```text
Reviewed base authority: sha256:<64 lowercase hexadecimal characters>
Reviewed base: <40-character lowercase SHA>
Reviewed head: <40-character lowercase SHA>
Review scope binding: sha256:<64 lowercase hexadecimal characters>
```

The parser compares all four values to the current review brief. Presence,
timestamp, or a prose statement such as “exact range” is supplemental and not
sufficient.

### Destination Divergence Is Separate

Handoff and review-brief must report destination convergence outside
`review_scope_binding` with exact fields `destination_ref`, `destination_sha`,
`destination_observed_at`, `destination_observation_freshness` (`CURRENT`,
`STALE`, or `UNKNOWN`), `destination_merge_base_sha`,
`destination_ahead_count`, and `destination_behind_count`. A local
remote-tracking ref without current remote-observation proof is `UNKNOWN` or
`STALE`, never asserted current. These fields do not alter the authorized
checkpoint base and must set `scope_establishes_destination_convergence` and
`scope_establishes_merge_readiness` to `false`. They also grant no CI, merge,
or release authority.

## Options Considered

| Option | Pros | Cons | Decision |
| --- | --- | --- | --- |
| Keep default `origin/main` and use the existing green guard | No source change | Can approve a review whose actual scope contradicts the current handoff; unacceptable evidence overstatement | Rejected |
| Rewrite the local `origin/main` ref before handoff | Makes the current producer emit the desired paths | Falsifies remote observation, is not team-replicable, and can hide destination drift | Rejected |
| Use an explicit bypass for every divergent integration checkpoint | Already supported for an explicit human exception | Normalizes exceptions and leaves the parser false positive intact | Rejected as a durable solution |
| Refresh the branch from current `main` before every checkpoint | Uses today's default scope and is required before merge readiness | Changes the exact checkpoint, forces unrelated convergence early, and does not fix review-output scope binding | Required at final integration, not selected as the handoff correction |
| Let any caller pin an ancestor and carry a deterministic binding through review and guard | Internally consistent and easy to expose | A caller can select `HEAD` for an empty scope or a later ancestor that omits earlier branch changes | Rejected |
| Trust a validated plan committed on producer `HEAD` | Simple and locally deterministic | Self-authorizing when the producer owns the plan and can rewrite base, branch, or roots | Rejected |
| Derive the effective base from the portfolio-registered plan blob at an externally admitted accepted-main commit and carry a deterministic binding through review and guard | Externally anchored, exact, reportable, testable, works for ratified-base branches, preserves destination truth separately | Requires a planning PR before source admission and coordinated producer/CLI/guard/docs/test change; live assignment/lease currency stays external | Selected |

## Decision Provenance

| Date | Owner | Decision | Evidence / Rationale | Revisit Trigger |
| --- | --- | --- | --- | --- |
| 2026-08-12 | Program Flow Controller + Framework CLI Architect | Admit one bounded review-harness correction lane from pushed IX aggregate `1e919fafe783b68cef4abc65f99d585be482b4e7` | Exact-base review saw 59 paths; hardcoded handoff saw 145; explicit-base handoff exited 2; default guard could parse the narrow review as green for the broad scope | If implementation requires another authority surface or cannot remain inside the machine plan |
| 2026-08-12 | Framework CLI Architect | Bind review output to base, head, and a versioned surface digest rather than only headings and timestamps | Base support alone would leave the existing false-positive parser path open | If the repository adopts a signed structured independent-review receipt that supersedes Markdown binding |
| 2026-08-12 | Program Flow Controller | Reuse the existing IX integration destination and reserve shared docs-check/reference paths for Integration | The correction is a prerequisite for the next IX aggregate checkpoint; a second empty integration train would add no value, and the active delivery-control plan already reserves shared docs roots | If Integration cannot absorb the bounded shared-path convergence without exceeding WIP/review capacity |
| 2026-08-12 | Framework CLI Architect | Make the validated machine plan the base authority; `--base` is only an equality assertion, and pre-push requires a strict base ancestor | An ancestry check alone permits `--base HEAD` or any later ancestor and can consistently certify an unauthorized narrowed scope; reviewer-visible disclosure cannot restore omitted changes | If a stronger signed assignment/plan authority supersedes local machine-plan validation |
| 2026-08-12 | Program Flow Controller + Framework CLI Architect | Supersede producer-HEAD plan authority with an externally admitted accepted-main portfolio snapshot and remove spec/plan from producer roots | A source producer that owns the plan can rewrite its own base, branch, and roots and then generate a self-consistent binding; commit and blob digests prove scope integrity, not live admission | If a trusted external assignment/registry adapter supersedes accepted-main portfolio authority |
| 2026-08-13 | Program Flow Controller + Framework CLI Architect | Keep live assignment and exclusive-lease currency external to the base-bound scope-integrity capability; remove caller assignment UUIDs and machine stale/superseded-assignment claims | Comprehensive review of `4f3e86004436a17b79a335b1fa004986955e0f8b` found no authoritative assignment receipt or lookup behind the proposed UUID, while accepted-main portfolio/blob integrity remains independently useful and enforceable | If a separately accepted trusted assignment-receipt adapter is designed and machine-proven |

## Architecture And Implementation Notes

1. Add one base-authority resolver shared by written handoff and no-write
   snapshot. It reads the portfolio and its registered plan from the exact
   accepted-main Git object selected after external admission, invokes the
   exported canonical validator against those bytes, verifies
   scope/branch/root identity, and derives the only allowed base. Do not
   duplicate plan-selection logic in the guard.
2. Compute the binding from already normalized, content-bound state records;
   do not rescan ignored build output or introduce a second fingerprint
   implementation.
3. Parse review-command authority/scope arguments before creating the no-write
   snapshot so authority, snapshot, and Git diff use the same effective base.
   Treat `--base` only as an assertion against that result.
4. Make retained handoff authority/base/binding equality a prerequisite of
   `handoff_current`.
5. Make review-output authority/base/head/binding equality a prerequisite of
   `review_output_sections_satisfied` and `pre_push_status.satisfied`.
6. Have the guard read the retained accepted-main/portfolio/plan authority,
   revalidate it, and invoke review-brief with equality assertions for its
   exact base, head, and binding. It must fail with a remediation message when
   an old handoff lacks the new authority record. This does not verify external
   assignment or lease currency.
7. Preserve current checked-out-branch and pushed-SHA checks in the Git hook.
8. Require a strict base ancestor before pre-push can be satisfied; keep an
   equal-base diagnostic report-only.
9. Keep current-main divergence visible as an Integration condition; do not
   silently substitute current main for the ratified review base.
10. Reject any producer change outside the externally anchored scope's write
    roots, including this spec and executable plan.

### Comprehensive Review Route

This governance-sensitive source lane always requires a comprehensive review
even if the repository's current path-based auto-depth selector classifies the
candidate as focused. The dispatcher must retain
`review-brief --comprehensive --json`, invoke
`/framework-pr-review --comprehensive`, and bind that review to the exact
authority/base/head/surface record. A focused auto-depth brief or a green guard
that only establishes the lower automatic depth is insufficient for this
plan. Updating the shared auto-depth selector is out of scope for this planning
change and must not be smuggled into the source lane.

## Security, Privacy, And Governance

This change handles only repository-visible Git metadata, source paths, file
state digests, and sanitized review text. It must not read credentials, tenant
data, PHI/PII, live provider state, or local environment secrets into retained
artifacts.

The binding is integrity metadata, not a signature or an approval credential.
It proves internal artifact agreement in one checkout; it does not prove
reviewer identity, human approval, remote destination state, CI success, merge
readiness, release authority, SRA/CAB approval, or accepted risk.

## Acceptance Evidence

| Criterion | Proof Command / Artifact | Required Before |
| --- | --- | --- |
| Product Increment and portfolio registration are machine valid on the planning PR; source diff is validated against the exact accepted-main lane blob during authority resolution | plan validator, portfolio validator, and semantic authority/current-diff fixtures | admission and source review |
| Planning spec, executable plan, Product-authorized portfolio registration, and index land through a reviewed planning/control PR from accepted main before source admission, separate from the future implementation lane | exact accepted-main SHA plus retained planning review | admission |
| Planned leaf and integration handoff derive the exact base and roots from the portfolio-registered plan blob at the externally admitted accepted-main SHA | semantic fixtures plus retained handoff/review-brief JSON with identical `appfw.review_scope_binding@1` | source review |
| Producer-local edits to the spec/plan or a caller-selected plan path cannot alter authority | negative fixture and out-of-root current-diff rejection | source review |
| `--base HEAD`, an equal derived base, and any later intermediate ancestor cannot satisfy pre-push or narrow the plan-authorized range | disposable Git graph negative fixtures | source review |
| Invalid, unresolved, unregistered, portfolio/plan digest, branch, scope, root, or caller-base mismatches fail closed | introspection/CLI negative tests | source review |
| Live Program Flow assignment and exclusive-lease currency exist before source admission, without being represented as machine proof from this capability | external PFC assignment/lease record and Role Card Check; no caller assignment UUID in CLI or binding | admission |
| Review text with complete headings/status/counts but missing or wrong authority/base/head/binding is rejected | review parser and guard negative fixtures | source review |
| Advancing `origin/main` after handoff cannot silently change the pinned checkpoint scope | disposable Git graph regression fixture | source review |
| Content, status, mode, symlink, rename, deletion, untracked, and non-UTF-8 protections remain | existing and extended CLI semantic fixtures | source review |
| Default ordinary-main handoff remains usable and records an immutable effective binding | compatibility fixture | source review |
| CLI reference, review harness, role-card policy, skill, tests, and behavior agree | changed-only docs-check plus alignment review | aggregate review |
| Exact assembled candidate receives explicitly routed comprehensive independent Framework review with zero blockers/critical findings | retained comprehensive review brief plus `target/appfw/framework-pr-review.md` from `/framework-pr-review --comprehensive`, bound to the assembled SHA and new binding | push |
| No bypass was used to establish acceptance evidence | guard artifact and handoff/review summary | push |

### Ordered Pre-Merge Dependencies

The exact `4f3e86004436a17b79a335b1fa004986955e0f8b` planning candidate made the
full documentation gate deterministically red in two owner-external fixtures:

1. `scripts/check-product-increment-portfolio.test.mjs` encodes the previous
   portfolio clock, Product Increment count, and active-lane count.
2. `scripts/check-nexus-workstreams.test.mjs` copies the canonical portfolio
   without copying every newly registered plan into its temporary fixture.

Those failures are candidate-induced and are **not** green evidence, accepted
exceptions, or merge-ready proof. Batch A carries the approved owner-originated
fixture corrections as separately identifiable provenance
(`9acbc9383b2b5f0f780d4c2d88830ac25b81debd`) alongside this planning
correction. The aggregate must preserve the fixture assertions, reconcile the
Nexus suite to exactly 36 tests, pass the full docs gate, and receive
comprehensive independent review. This aggregation does not make either
fixture part of the future base-bound implementation lane.

## Test And Execution Feedback Plan

Run the smallest authority fixtures first: a producer-local plan that widens
roots or changes its base, an unregistered/caller-selected plan, a mismatched
accepted-main authority commit, `--base HEAD`, a machine-plan base
equal to head, and a full-SHA intermediate ancestor must all leave
`pre_push_status.satisfied:false`; the intermediate ancestor must not replace
the plan-authorized base. Then exercise accepted-main portfolio/plan blob and branch mismatch,
a valid-looking review document whose authority/base/binding is wrong,
moving-remote-ref stability, existing content-bound surface mutations,
nested-product path normalization, shell syntax, plan/current-diff validation,
changed-only docs-check, generation drift, fast Framework tests, handoff,
explicit comprehensive brief, and `/framework-pr-review --comprehensive`.

If the implementation cannot keep one binding producer for written and
no-write handoff, or requires callers to rewrite a Git ref, set an environment
bypass, or trust unparsed review prose, stop and update this decision rather
than weakening the acceptance criteria.

## Risks And Controls

| Risk | Control | Owner | Status |
| --- | --- | --- | --- |
| Producer rewrites its own plan, base, branch, or roots | Read portfolio/plan only from the externally admitted accepted-main commit; remove spec/plan from producer roots; bind accepted-main, blob, scope, and root digests | Program Flow Controller + Framework CLI Architect | planned |
| Caller chooses a narrow arbitrary base | Accepted-main plan derivation; `--base` equality-only; branch, root, and strict-ancestor checks; visible destination divergence | Framework CLI Architect | planned |
| Plan is changed or a different lane is selected after handoff | Retained accepted-main/portfolio/plan/scope digests plus guard-time revalidation | Framework Review Harness Coding Agent | planned |
| Review parser accepts unrelated judgment | Mandatory exact authority/base/head/binding comparison | Framework Review Harness Coding Agent | planned |
| Moving `origin/main` invalidates or widens an already reviewed checkpoint | Pin accepted-main authority at admission, retain its exact blobs/base, and require a current descendant destination observation before pre-push | Framework CLI Architect | planned |
| Shared producer regresses nested products or unusual paths | Existing special-path/content-bound fixtures remain mandatory | Framework Review Harness Coding Agent | planned |
| Correction is mistaken for merge or release readiness | Separate destination freshness and retain policy nonclaims in guard/docs/review | Integration Branch Manager | planned |
| Active delivery-control and mobile lanes own shared planning fixtures | Batch A retains the approved owner-originated fixture repair with separate provenance; the future implementation producer still excludes the planning records and fixtures | Program Flow Controller | controlled |
| Caller-provided assignment UUID is mistaken for live Program Flow authority | Accept no assignment UUID in the CLI or binding; require current assignment and exclusive lease as external admission evidence | Program Flow Controller | controlled |

## Tech Debt And Follow-Up

This increment does not solve trusted reviewer identity or signed delivery
receipts. The existing delivery-control program remains responsible for that
larger authority boundary. If independent review returns `GO WITH CONDITIONS`,
the Integration Branch Manager routes each condition to the tech-debt register
or an explicit follow-up before push.

## Handoff Notes

- Safe admission and push order is mandatory:
  1. assemble the corrected planning records and the approved owner-originated
     fixture repairs in the bounded Batch A successor with separate provenance;
  2. prove the exact 36-test contract and full docs gate green, retain a
     comprehensive review brief, and invoke `/framework-pr-review --comprehensive`
     on the immutable aggregate SHA;
  3. only after eligible review and the normal guard may that planning/control
     successor be pushed for a human-approved PR and merge to accepted `main`;
  4. Integration observes/fetches the exact accepted-main SHA and external
     Program Flow records a current assignment plus exclusive lease selecting
     its portfolio/plan blobs and scope; the base-bound capability does not
     machine-verify that record;
  5. only then create `fix/base-bound-handoff-review-r1` from exact IX source
     base `1e919fafe783b68cef4abc65f99d585be482b4e7`, without cherry-picking or
     editing the planning documents;
  6. implement, hand off, and independently review only the externally
     authorized source roots; and
  7. push the leaf to the IX integration branch, then let Integration refresh
     destination-main truth and run aggregate proof.
- Source branch: `fix/base-bound-handoff-review-r1`.
- Exact source base and current pushed IX integration checkpoint:
  `1e919fafe783b68cef4abc65f99d585be482b4e7`.
- Destination: `integrate/ix-eight-vignette-foundation-r1`; final destination
  remains `main` through the existing IX integration train.
- The lane begins with delivery credit `none` and capability state `planned`.
- Source authority still requires the Program Flow Controller's live assignment
  and exclusive lease. This spec, plan, CLI authority commit, binding, and guard
  do not grant or machine-verify that authority by themselves.
- Any path outside the accepted-main lane roots requires a new docs-only
  planning PR and PFC re-admission before editing; the source branch cannot
  amend its own authority.
- No new implementation roots are required by the accepted-main plan authority
  ruling: plan resolution belongs in `scripts/appfw`, binding production in
  `app_gen/src/bin/appfw_introspect.rs`, guard/hook enforcement in their already
  admitted scripts, and regression proof in
  `scripts/cli-semantic-gates-test.sh`.
- The superseded unpushed planning commits
  `d5461597df1ec872c6cecd27208683d88d0217f8` and
  `f803593dd03d250a56e99c80088162db27c76eb2` are provenance only and must not be
  used as source authority.
- Remote branch `origin/docs/base-bound-review-authority-r2` already reached
  `4f3e86004436a17b79a335b1fa004986955e0f8b` before its canonical comprehensive
  review returned `DEFER`. That review is useful correction evidence but cannot
  retroactively establish pre-push compliance. The corrected descendant is
  local-only at this planning checkpoint and is not authorized for push, PR,
  merge, admission, or implementation by this document.
- The two canonical planning fixture repairs retain their owner-originated
  provenance in Batch A. They are convergence dependencies, not future
  implementation roots. The aggregate must keep the dynamic portfolio clock,
  plan discovery, lane-count preservation, and exact 36-test assertions green
  before comprehensive review; no fixture or portfolio weakening is permitted.
