# Branch Integration Model

Use this guide when Delivery Lanes are advancing a Product Increment at the
same time. The goal is two to four independent lanes with fast focused proof,
one convergence branch, and one full integration payment before `main`.

This is an operating model, not a replacement for the roadmap. The roadmap owns
Product Increment order and acceptance. The
[Product Increment Delivery Model](product-increment-delivery-model.md) owns
lane admission and capability links. This guide owns branch shape, merge order,
and conflict hygiene.

## Go-Forward Merge Model

Default to this model for Product Increment work. Every active multi-lane or
cross-domain Product Increment has one short-lived integration branch named
from the Product Increment. A truly independent one-lane increment may declare
`direct_main`; that PR uses full candidate proof. This is a first-class
strategy, not an informal bypass or an accelerated path.

1. **Lane branches stay small and independent.** A Delivery Lane owns one or
   more tightly related Deliverables, one write surface, and one focused proof
   loop. Runnable lanes may not overlap source roots or wait on unfinished
   behavior without a frozen contract or fixture.
2. **Integration branches group one Product Increment.** Create one branch,
   for example:
   - `integrate/nexus-add-provider-poc`
   - `integrate/framework-product-consumption-v1`
   - `integrate/afs-routing-telemetry-v1`
   Do not create an integration branch for a one-lane increment when it would
   have no separate convergence work.
3. **Integration branches start from current `main`.** Fetch first, create the
   integration branch from `origin/main`, merge lane branches into it, resolve
   conflicts once, and run the family-appropriate evidence.
4. **Merge-bound pushed branches get PRs promptly.** A feature, fix, docs,
   wave, or integration branch intended for merge should not sit as an orphaned
   remote branch. After push, either the assigned branch owner creates/updates
   the PR or immediately hands the pushed SHA, retained review output path,
   proof summary, and branch purpose to the Integration Branch Manager.
   Non-`main` branch pushes in this repository do not by themselves run the
   full pull-request gate; the PR object triggers the PR pipeline evidence.
5. **Main receives the Product Increment PR, not every leaf branch.** Leaf PRs
   target the integration branch and run accelerated, changed-surface proof.
   The integration PR to `main` runs full candidate proof. After it merges,
   absorbed leaf branches and worktrees are terminal and should be removed once
   required evidence is retained. For `direct_main`, the sole lane PR is the
   Product Increment PR and runs full candidate proof.
6. **Refresh integration branches after each upstream merge.** If `main`
   advances, merge `origin/main` into the still-open integration branch, resolve
   conflicts there, rerun the evidence, and push the refreshed integration
   branch. Do not try to fix the same conflict separately in every stale leaf
   branch.
7. **Publish only after the Product Increment train is green.** Non-release
   integration merges may land on `main` to keep work moving, but do not
   publish framework packages, ProGet artifacts, or product upgrade guidance
   until the required gates are green on the final merged SHA.

### Required CI Boundary

Integration branches only increase throughput when CI distinguishes branch
purpose and risk. The pipeline must inspect PR destination and automatic change
classification:

- lane to integration: accelerated focused checks;
- integration to `main`: full aggregate checks;
- release candidate or tag: strict release checks; and
- sensitive paths: remain visible in impact classification, use focused
  semantic checks on a prototype leaf, and fail closed to full aggregate proof
  before `main` or strict proof before release.

A human or agent can escalate but cannot downgrade detected risk. Until this
routing exists, integration branches still reduce conflict and organize proof,
but they do not by themselves reduce remote pipeline duration.

## Fan-Out Readiness

Before the Program Flow Controller admits parallel Delivery Lanes, the App
Framework Chief Architect and Integration Branch Manager should confirm the
shared seams are stable enough to fan out:

- validated Product Increment plan, base branch, and integration branch;
- accepted spec, lightweight spec, or explicit intent note for the shared
  outcome, following [Spec-Driven Change Harness](spec-driven-change-harness.md);
- typed `requires` and `benefits_from` links with provider state, decoupling
  seam, convergence action, and integration check;
- generated-boundary ownership and source-of-generation files;
- CLI command contracts and docs-check expectations;
- CI/release evidence paths and artifact locations;
- provider/runtime contracts that multiple lanes will consume; and
- branch dependency order and freeze set.

If a shared seam is still moving, freeze it first in a focused branch or
integration preflight. Producers should not start overlapping lane work until
the frozen contract, dependency order, and review depth are explicit. This is
how the harness gains safe parallelism without replaying the same conflict in
multiple stale branches.

### Multi-Workstation Assignments

When producers run on several computers, apply
[Nexus PoC Multi-Workstation Delivery](../specs/nexus-poc-multi-workstation-delivery.md)
before creating another worktree. Each source-producing workstation must have
one local assignment, one purpose-named branch, one accepted base SHA, and one
selected Product Increment Delivery Lane. The assignment base must equal the
lane's immutable `base_sha`; its write roots come only from that validated
lane. Workstream topology supplies capability context, required
reading, and additional restrictions; it does not grant source authority.
Root manifests, lockfiles, shared runtime/provider contracts, central CLI/CI,
the program registry, generated templates, and the roadmap remain
integration-owned unless the selected lane explicitly and lawfully owns them.

Use `node scripts/check-nexus-workstreams.mjs --context <WS-ID> --json` as the
small context pack. Use `node scripts/audit-worktrees.mjs --json` before
cleanup. The audit never authorizes deletion. A clean merged worktree is only a
candidate until its owner and the Integration Branch Manager confirm that no
unpushed work or retained evidence is needed.

## Integration Branch Manager Responsibility

The Integration Branch Manager is the release-train conductor. It owns branch
order, PR creation/update coordination, PR evidence, CI status,
merge-readiness summaries, conflict workflow, and integration branch hygiene.
It is not the universal fixer for every pipeline failure.

It may fix integration mechanics when assigned:

- CI command shape or step ordering;
- artifact/report-directory contamination;
- stale retained artifacts used by a later step;
- release/PR evidence plumbing;
- merge conflicts that do not require product or architecture judgment; and
- PR metadata or branch-train assembly issues.

For a simple direct-to-`main` leaf branch, the Program Flow Controller may
assign PR creation to the Architect or Coding Agent that owns the branch. When
not explicitly assigned,
the default owner for creating/updating PRs and monitoring the resulting PR
pipeline is the Integration Branch Manager. The branch owner still supplies the
PR-ready summary, review output path, proof evidence, and any conditions.

It should detect, preserve evidence, and route instead of fixing:

- product behavior failures;
- architecture or implementation test failures owned by a lane branch;
- missing release-lite/SRA/CAB/human approval fields;
- security or accepted-risk decisions; and
- production/release authority questions.

The Integration Branch Manager is not a permanent Bitbucket polling daemon.
Once the Program Flow Controller or human assigns a PR or pipeline, Integration
owns live observation without
another prompt through terminal PR evidence and post-merge destination
evidence. An active Integration task or configured automation must follow
[Remote Promotion State Synchronization](remote-promotion-state-synchronization.md).
Monitoring assignments should name:

- PR or pipeline identifier;
- polling cadence, usually 5-10 minutes for ordinary PR pipelines;
- stop condition: acknowledged terminal destination evidence, acknowledged
  decline/supersession, or an explicit human stop recorded in the durable
  carrier; timeout is an escalation/transfer trigger, never a silent stop for
  a live train;
- what evidence to capture on failure: concrete failed step, log edge, retained
  artifact, and owner route; and
- what evidence to capture on success: green step durations, commit SHA, PR URL,
  merge SHA, destination evidence, and merge-readiness caveats; and
- freshness behavior: unchanged polls refresh current state but do not append
  history, while stale observations force `STALE` or `UNKNOWN` in Program Flow
  Controller
  projections.

A terminal pipeline failure routes a correction owner and remains assigned
until the train is explicitly superseded, declined, or stopped; failure alone
does not make monitoring responsibility disappear.

Integration sends the Program Flow Controller one machine-readable record for
every material PR, pipeline, merge, or destination-evidence transition. The
Program Flow Controller consumes it within one active monitoring cadence,
refreshes board/queue/dashboard state, and clears
only holds whose predicate the transition actually retired. Neither role gains
push, merge, release, or accepted-risk authority from this synchronization.

## Risk-Tiered Delivery Loops

Use the cheapest valid loop that matches the risk of the change. The goal is
fast feedback without pretending that local proof is release authority.

| Loop | Use for | Branch shape | Local proof before PR/push | Remote proof |
| --- | --- | --- | --- | --- |
| **Inner loop** | Focused edits while implementing a lane. | Local worktree or small leaf branch. | Run the smallest surface check first: `git diff --check`, conflict-marker scan, then the changed-surface command from `docs/start/agent-task-map.md`. | None until the branch is ready for review. |
| **Lane PR loop** | One independently provable Delivery Lane in a multi-lane Product Increment. | `feature/<topic>`, `fix/<topic>`, or `docs/<topic>` targeting `integrate/<product-increment>`. | `scripts/appfw framework change-impact --json`, focused validation/test/docs commands, handoff, and focused PR review unless the class requires comprehensive review. | Destination- and risk-aware accelerated checks. Sensitive changes escalate automatically; the lane does not pay the entire Product Increment regression cost. |
| **One-lane Product Increment loop** | One genuinely independent Product Increment with no convergence work. | One leaf branch with declared `direct_main`, targeting `main`. | Full candidate checks, handoff, and independent review. | Full candidate pull-request pipeline once. Do not create an empty integration branch merely to avoid this proof. |
| **Integration loop** | Two or more Delivery Lanes, cross-domain work, or broad/high-churn convergence. | `integrate/<product-increment>` from current `origin/main`. | Aggregate `change-impact`, cross-lane contract and journey checks, handoff, and `/framework-pr-review --comprehensive` or `/product-pr-review --comprehensive`. | Full Product Increment pull-request pipeline on the integration branch. Main receives this integration PR, not every absorbed lane. |
| **Release authority loop** | Main readiness, ProGet/package guidance, or production claims. | `main` for focused provider-backed evidence; `v*` tags for strict production release authority. | Only release candidates should claim release readiness. Local live preflight is branch confidence, not promotion authority. | Main runs focused release evidence. `v*` tags run the strict release gate. |

Do not use a heavier loop just because a command exists. Do use a heavier loop
when the branch changes sensitive surfaces, crosses ownership domains, promotes
readiness, or needs evidence a human/release authority must trust.

## Throughput Rules

- **Honor the Program Flow Controller WIP limit.** Do not start more producer
  branches than the current integration capacity, human-review capacity,
  freeze safety, and CI queue can absorb.
- **Classify early.** Run `scripts/appfw framework change-impact --json` before
  a branch grows. If it is Class C/D or requires an integration branch, reshape
  the work before review rather than after CI fails.
- **Use the two-speed review path.** Class A/B, narrow, non-sensitive lanes
  targeting their Product Increment integration branch can use focused review
  and standing push approval. A one-lane Product Increment targeting `main`
  uses full candidate proof. Class C/D, integration,
  governance, release/security, generated-template, AI egress, PHI/PII, or
  broad changes require comprehensive review and human merge judgment.
- **Keep leaf branches mergeable.** Branch from current `main` unless there is
  a real dependency. Do not stack unrelated leaves on top of each other.
- **Use changed-surface proof first.** Prefer changed-only docs/test paths and
  focused provider/product commands while implementing. Let full docs-check,
  provider-backed certification, and release gates run only when the changed
  surface or evidence claim requires them.
- **Prepare release-lite evidence before the PR stalls.** If a branch touches
  provider/runtime/security/release-sensitive paths, run or request the custom
  `release-check` evidence before expecting the PR release-lite guard to pass.
- **Treat CI disabling as diagnostic only.** Temporarily narrowing a pipeline is
  acceptable only on an explicit diagnostic branch or commit sequence, with the
  normal gate restored before the final evidence run.
- **Do not restart remote pipelines casually.** Batch conflict refreshes and
  evidence fixes into one push, then wait for the resulting run. Surprise pushes
  while CI is running waste the queue and confuse reviewers.
- **Prove Bitbucket REST auth once.** Before inspecting remote PR or pipeline
  state from a fresh thread or workstation, use
  [Bitbucket REST Auth Runbook](bitbucket-rest-auth.md) and run
  `scripts/ci/bitbucket-api-smoke.sh --repo --pipelines`. If Bitbucket returns
  `401`, diagnose the token type, Basic username, and header scheme instead of
  retrying with unclassified token variables.
- **Move recurring failures left.** If the same CI failure appears twice, add a
  local command, docs-check subcheck, review-brief signal, or agent instruction
  so future branches catch it before Bitbucket.
- **Close integration with a learning step.** At the end of every integration
  merge or repeated failure investigation, name what should move left: local
  check, docs-check subcheck, template guard, review-brief signal,
  Workstream Analyst recommendation, or tech-debt entry.
- **Park blocked lanes explicitly.** A lane waiting on human approval,
  provisioning, SRA/CAB input, release-lite variables, architecture decision, or
  accepted-risk question should publish its parked-decision brief and let the
  Program Flow Controller pull the next unblocked lane inside the WIP limit.
- **Use integration branches to reduce duplicate conflict work.** Resolve shared
  conflicts once in the integration family instead of replaying them across
  stale leaf branches.

## Branch Names

Branch prefixes describe the work, not the tool or author. Use purpose or lane
names such as `feature/<topic>`, `fix/<issue>`, `docs/<topic>`,
`integrate/<wave-or-family>`, or `wave3/<lane>`. Do not use author/tool prefixes
such as `codex/`, `claude/`, or `agent/`; developers may use different agent
harnesses, and review/CI should reason about the work stream rather than the
assistant that produced it.

## Human Oversight Triggers

Use
[`agentic-development-control-system.md`](../architecture/concerns/agentic-development-control-system.md)
to classify broad or sensitive changes before opening an integration PR.
Integration branches are a conflict-management tool, not a waiver for human
review.

Treat the train as human-review-required when any leaf or the aggregate
integration branch:

- changes more than 25 files or more than 1200 non-generated lines;
- touches three or more ownership domains;
- changes auth, policy, tenant isolation, token storage, release gates, CI
  security scripts, dependency acceptance, provider capabilities, governed
  writes, MCP/Kafka ingress, AI/chat egress, mobile secure storage, PDS
  governance, or generated templates;
- promotes a capability from unsupported/report-only/mock-only to executable,
  certified, release-ready, or production-ready;
- changes readiness scores or claims live/release evidence; or
- includes generated output without a matching source-of-generation change and
  drift explanation.

The PR summary should name the change class, sensitive surfaces, retained
evidence, skipped checks, and reviewer decision needed. When the future
`scripts/appfw framework change-impact --json` command exists, attach its
retained `target/appfw/change-impact.json` report to the handoff.
Use [PR Review Agent Harness](pr-review-agent-harness.md) for a review-only
pass before asking the human to approve a Class C/D train.

## Merge Order

Use the roadmap dependencies first. When dependencies do not dictate order,
merge integration families from the most foundational/shared surfaces toward
the product-facing surfaces:

1. CI, release, and docs-check contract changes.
2. Runtime, provider, auth, and shared operation contracts.
3. Data-plane/provider certification/supporting evidence.
4. Chat, AI, and product experience wiring that consumes the runtime/data
   contracts.
5. Frontend/mobile/product examples that consume generated contracts and design
   system primitives.

If two branches edit the same high-churn files (`scripts/appfw`,
`scripts/check-doc-examples.sh`, root `Cargo.toml`, `Cargo.lock`,
`docs/release/roadmap.md`, generated UI/mobile contracts, or PDS component
catalog manifests), serialize them or put them in the same integration family.

## Rules To Avoid The Merge Pain

- **Do not chain unrelated lane branches.** Branch from current `main` unless
  the roadmap says a branch depends on another branch. Chaining unrelated work
  turns every later PR into a conflict replay.
- **Do not open PRs from stale leaf branches after an integration branch
  exists.** If the integration branch contains the lane's useful work, the leaf
  branch is now informational only.
- **Do not keep pushing surprise commits to a PR while its pipeline is running.**
  If a conflict refresh is necessary, push once, tell reviewers the pipeline was
  intentionally restarted, and wait for the new run.
- **Do not resolve conflicts by downgrading current `main`.** When a stale
  branch adds an older copy of an artifact that `main` has since improved, keep
  `main` and salvage only still-unique guidance or source changes.
- **Do not turn product examples into branch history museums.** Product example
  branches should leave durable source, tests, and evidence. PR-by-PR notes
  belong in the PR or archive, not in the hot-path docs.
- **Do not treat a passing local check as release authority.** Local evidence
  can make a branch review-ready. Release authority still comes from the
  managed release gates and retained release artifacts.
- **Do not delete superseded branches until their contents are accounted for.**
  Check whether each leaf branch is an ancestor of `main` or of the integration
  branch, or confirm that its only unique changes were intentionally discarded
  as stale.
- **Do not use one broad docs-check run as a substitute for ownership.** Broad
  branches often force full docs-check. Use the timing/subcheck artifacts to
  decide whether to split the branch, serialize a shared surface, or keep the
  integration family together.

## Conflict Resolution Playbook

When Bitbucket reports a conflict on an integration PR:

1. Fetch `origin`.
2. In an isolated worktree, check out the integration branch.
3. Merge `origin/main` into the integration branch.
4. Resolve conflicts by preserving current contracts from `main` unless the
   lane branch has newer, explicitly planned behavior.
5. Preserve still-useful leaf-branch guidance as small docs/skill edits instead
   of replacing whole current files with old copies.
6. Run at minimum:

```bash
git diff --check
rg -n '^(<<<<<<<|=======|>>>>>>>)' .
scripts/appfw product validate --json
scripts/appfw framework docs-check --changed-only --json
```

Add focused checks for the lane family. Examples:

```bash
scripts/appfw framework feature-check --json
scripts/appfw framework provider-test --provider salesforce --area saas-read --plan --json
scripts/appfw product generate --target mobile-rn --check --json
scripts/appfw product mobile-test --json
```

`mobile-test` is a non-authoritative Prototype diagnostic. Its success cannot
establish mobile candidate or release readiness; integration evidence must keep
M0-05 containment intact until the source-bound mobile candidate checker exists.

7. Commit the conflict refresh and push the integration branch.
8. Restart or rerun the PR pipeline from the refreshed SHA.

## Superseding Leaf Branches

After an integration PR merges:

1. Confirm `origin/main` contains the integration merge.
2. For each leaf branch, confirm one of:
   - the leaf branch is an ancestor of `origin/main`;
   - the leaf branch is an ancestor of the merged integration branch; or
   - the integration PR intentionally discarded the leaf branch as stale and
     retained that decision in the PR summary.
3. Delete the superseded remote leaf branches.
4. Keep the integration branch only while the PR/review/audit trail needs it;
   otherwise delete it too.

This keeps branch lists readable and prevents agents from accidentally opening
new PRs from stale, already-absorbed work.

## Durable Branch Disposition

Use [Branch Disposition And Scavenging](../specs/branch-disposition-and-scavenging.md)
when a stale or mixed branch will be merged, scavenged into successors,
deprecated, or removed. The durable
[Branch Disposition Ledger](../archive/branch-disposition-ledger.md) records the
terminal value outcome against the exact source tip so the live branch can be
deleted without erasing what happened to its payload.

Do not mark a branch `SCAVENGED` merely because a successor exists or a local
cherry-pick succeeded. Terminal status requires complete commit/path
disposition, accepted successor merge and destination evidence, recoverability,
Chief Architect observation, and Integration Branch Manager verification.
Branch cleanup is a later `RETIRED` transition with its own exact preconditions,
human authorization, and post-deletion receipt.

The Chief Architect observes architecture and framework/product-boundary
coverage without becoming the cleanup authority. The Integration Branch
Manager verifies exact refs, successor and destination evidence, recovery, PR
annotation, and retirement targets without assuming merge or deletion
authority. Planning and polling remain in current convergence reports; the
archive ledger stays terminal-only.
