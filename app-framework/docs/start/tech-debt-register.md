# Tech Debt Register

Use this register when a review, handoff, CI run, steward brief, or human
decision finds a condition that should not disappear into chat history.

The register is an operating control for agentic delivery. It keeps velocity
honest: agents may push reviewed work with conditions only when the conditions
are explicit, owned, and retired through a later proof lane.

## Register Contract

Create or update an entry when any of these happen:

- a Framework or Product PR Review Agent returns `GO WITH CONDITIONS`;
- a check is skipped for a reason that affects merge, release, SRA, CAB, live
  evidence, or future maintainability;
- the same review finding, CI failure, merge conflict, stale-branch issue, or
  credential blocker appears more than once;
- Structure Steward, Research Steward, Strategist/Product Manager function,
  Product Owner, Architect, Workstream Analyst, or Integration Branch Manager
  recommends deferred maintenance;
- a required spec is missing, stale, contradicted by implementation, or leaves
  acceptance evidence unresolved beyond the current branch;
- a human accepts a temporary condition but expects a paydown path; or
- a branch is safe to push but not safe to merge until external evidence exists.

Do not use the register to hide accepted business, security, SRA, CAB, or
release risk. Accepted risk still needs the appropriate human authority. The
register tracks engineering and delivery follow-up; it does not approve risk.

## Required Fields

Each entry should include:

| Field | Meaning |
| --- | --- |
| ID | Stable identifier, for example `TD-001`. |
| Title | Short noun phrase that names the debt. |
| Status | `open`, `watch`, `blocked`, `retiring`, `retired`, or `rejected`. |
| Severity | `critical`, `important`, `should_address`, or `nice_to_address`. Use review severity when the entry comes from PR review. |
| Trigger | Review finding, CI run, steward brief, human decision, skipped check, or branch condition that created the entry. |
| Owner | Role that owns follow-up: Strategist/Product Manager function, Product Owner, Architect, Integration Branch Manager, Workstream Analyst, Tech Debt Steward, Structure Steward, SRA Package Agent, CAB Package Agent, or a named implementation lane. |
| Evidence | Artifact, PR, branch, pipeline, retained JSON/Markdown report, or command output that proves the issue exists. |
| Impact | Why the debt affects business value, security, maintainability, delivery throughput, or product readiness. |
| Retirement Criteria | Concrete proof required to close the entry. |
| Next Lane | Proposed purpose-named branch, review workflow, or report-only package lane. |

## Live Register

### TD-001: Credential readiness for agent/operator delivery lanes

- **Status:** open
- **Severity:** important
- **Trigger:** `fix/wave3-pr-gates-progress-timeouts` could be pushed with
  `BITBUCKET_API_TOKEN`; after `BITBUCKET_API_EMAIL` was added, REST
  repo/pipeline read passed, but earlier REST PR creation exposed missing or
  unproven pull-request write authority and commit-status evidence may need a
  separate supported path.
- **Owner:** Product Owner + Integration Branch Manager. Strategist/Product
  Manager function tracks business impact when credential friction blocks value
  delivery. Tech Debt Steward maintains the entry.
- **Evidence:** `docs/start/bitbucket-rest-auth.md`;
  `scripts/ci/bitbucket-api-smoke.sh --repo --pipelines` proves repository and
  pipeline read with the standard Basic auth path; branch push succeeded with
  Git token auth; PR creation/update remains unproven until a PR is created or
  updated through the intended path; Bitbucket commit build-status endpoint
  returned `403` with token-based auth while pipeline read worked.
- **Impact:** blocks full remote PR orchestration, delays delivery-ops
  hardening, and can prevent Wave B/Nexus promotion without manual
  intervention. Too little scope stalls work; too much scope weakens
  governance.
- **Retirement Criteria:** clean-machine proof includes Bitbucket auth
  readiness for branch push, repo/pipeline read, PR create/update, PR/pipeline
  evidence capture, and manual-PR fallback; `.env` setup documents
  `BITBUCKET_API_EMAIL` without committing secrets; required scopes and
  unsupported/alternate evidence endpoints are documented by lane.
- **Next Lane:** `docs/credential-readiness`, or fold into clean-machine
  harness proof after the current PR evidence path is moving.

### TD-002: Stale lane and worktree hygiene

- **Status:** open
- **Severity:** should_address
- **Trigger:** the current workstation has many older Wave 3/Wave 4 worktrees,
  several legacy assistant-prefixed branches, and detached audit checkouts while
  active work has moved back to purpose-named lanes from current `origin/main`.
- **Owner:** Integration Branch Manager + XO. Structure Steward recommends
  cleanup policy.
- **Evidence:** `git worktree list` shows stale `codex/*` branches, detached
  audit worktrees, and old wave integration worktrees alongside current work.
  The 2026-07-18 read-only inventory found 42 linked worktrees: 11 with
  uncommitted state, 7 clean merged reclamation candidates, and 24 requiring
  active-owner or integration review. Reproduce the local classification with
  `node scripts/audit-worktrees.mjs --json`; the command never removes state.
- **Impact:** stale branches make agents reason from obsolete wave assumptions,
  increase merge-conflict risk, and create broad cleanup temptation inside
  unrelated feature lanes.
- **Retirement Criteria:** Integration Branch Manager publishes a
  supersession/cleanup list; every active producer has one assignment, owner,
  purpose-named branch, current base, and worktree; obsolete worktrees are
  removed only after confirming no dirty, unpushed, detached-unmerged, retained
  evidence, or human-owned state; future lanes follow the machine-checked Nexus
  topology and branch integration model.
- **Next Lane:** Integration Manager owner-adjudication and staged cleanup from
  the report-only audit. Do not combine cleanup with a producer feature branch.

### TD-003: Docs-check full-lane latency exceeds developer velocity target

- **Status:** open
- **Severity:** important
- **Trigger:** Workstream Analyst timing review of
  `target/appfw/docs-check-timing.json` after
  `scripts/appfw framework docs-check --changed-only --json` correctly
  escalated to full mode for harness, CLI-contract, and agent-facing changes.
- **Owner:** Workstream Analyst + Architect. Product Owner prioritizes the D6
  throughput lane; Tech Debt Steward keeps the entry current.
- **Evidence:** retained docs-check timing artifact reports selected mode
  `full`, budget `60000`, `budget_ok: false`, `budget_enforced: false`, 473
  examples, about 56s aggregate example time, about 29s unattributed wall time,
  slowest example `generate-check-json`, and slowest subcheck
  `agent-governance-core-command-examples`. PR #454 pipeline #349 adds a
  separate fast-lane scaling signal: all 71 examples passed in 5.648s, while
  changed-surface selection spent 39.800s starting per-path classifiers across
  317 changed files and caused the enforced 20s lane to finish in 47.684s.
  PR #456 pipeline #350 ran the same 71-example fast lane in 7.066s with a
  299ms selector on a small diff. This phase split rejects a global fast-budget
  increase as the remedy and makes batched classification the bounded next
  correction.
- **Impact:** harness and CLI branches now frequently require full docs-check;
  an 80s+ local/CI lane slows review loops, encourages agents to ignore red
  non-enforced budgets, and makes the team wait on serialized command examples
  instead of business-value delivery.
- **Retirement Criteria:** full docs-check runs under the retained budget on
  normal CI capacity without deleting executable example coverage; timing
  artifacts attribute spawn/setup/IO overhead clearly; high-count subchecks are
  split or parallel-safe with isolated report roots; any `generate --check`
  substitution is backed by a real documented plan/contract command; and
  `budget_ok` becomes a trusted signal rather than a chronic warning.
- **Next Lane:** `fix/docs-check-changed-surface-selector` batches path
  classification while preserving the 20s fast budget; full-lane optimization
  remains open after that correction.

### TD-004: Retained CI evidence must not be restored from build caches

- **Status:** in progress
- **Severity:** important
- **Trigger:** Workstream Analyst review of PR pipeline latency found the
  Bitbucket `cargo-target` cache restoring the whole `target/` tree. Because
  retained framework evidence also lives under `target/appfw`, stale cache
  restoration could blur whether later gates are reading current evidence or
  old artifacts.
- **Owner:** Workstream Analyst + Architect. Integration Branch Manager verifies
  the CI artifact shape before merge; Product Owner/XO keep this in D6 because
  evidence trust is a prerequisite for throughput.
- **Evidence:** Bitbucket cache definition previously pointed `cargo-target` at
  `target`; PR artifacts and logs showed heavy Rust cache restore alongside
  retained `target/appfw` evidence artifacts. Main-branch pipeline logs also
  showed prior `target/appfw/**` artifacts downloading into the release-focused
  step before the old `cargo-target: target` cache was restored, making cache
  extraction order part of the evidence-trust risk. Pipeline #434 later
  collected 4,749 files/about 2.5 GiB from the broad Fast-step artifact glob,
  spent about 130 seconds compressing it to 719 MiB, and passed that payload to
  PR steps that either clear the evidence root or never read it. Pipeline #437
  then confirmed a 52m26s Fast producer followed by a 10m58s supply-chain
  producer even though Fast, supply-chain, and secret scan are independent.
- **Impact:** slow build caches are a velocity problem, but cached retained
  evidence is a trust problem. Agentic delivery depends on current,
  machine-readable evidence for review, release-lite, supply-chain, secret
  scan, and merge readiness.
- **Retirement Criteria:** CI caches only Rust build output, not
  `target/appfw`; producer steps clear and mark the canonical framework
  evidence root before writing fresh evidence; tools such as gitleaks install
  outside `target/appfw`; aggregating steps preserve intentionally downloaded
  prior artifacts; the Fast producer retains a bounded exact-SHA, per-file
  hash manifest instead of caches/transient workspaces; non-consumer PR steps
  skip inherited downloads; the three independent PR producers share one
  post-preflight parallel group; the bundle binds exact source/tested/destination
  identity and publishes atomically; the closing guard rejects destination
  advancement; and a green exact-SHA PR pipeline proves the new artifact shape,
  producer overlap, cache behavior, freshness, and upload timing.
- **Next Lane:** PR #483 and base `bc068af8...` are historical predecessor
  state. Complete the bounded guard and provenance correction on current R2
  successor `fix/pr-pipeline-throughput-convergence-r2`, based on exact main
  `a6d9d41040a32a4ed32a81b521bc113f0f69048c`, obtain a fresh comprehensive
  review, then retain exact-SHA artifact bytes, upload duration, producer
  overlap, and closing freshness evidence before retiring this debt.

### TD-005: Change-impact undercounts untracked source

- **Status:** open
- **Severity:** important
- **Trigger:** the multi-workstation harness added six untracked docs/JSON/Node
  files. `scripts/appfw framework change-impact --json` listed them but reported
  zero added lines for each, then classified the aggregate as Class B.
- **Owner:** Architect + CLI owner. PR Review Agent treats untracked source as
  real scope until the producer is corrected.
- **Evidence:** pre-staging command output on
  `feature/multi-workstation-harness` at base `6c4781d7e` reported six
  untracked files, 59 non-generated added lines, and Class B. After the same
  files were committed, the retained report counted 1,648 added lines and
  correctly selected Class C. The content did not materially change between
  those classifications.
- **Impact:** broad harness, policy, or executable-check changes can receive a
  cheaper review route because material untracked source is omitted from line
  and threshold calculations. Multi-agent work commonly begins with new files,
  so the blind spot is systematic rather than unusual.
- **Retirement Criteria:** change-impact counts untracked text lines and
  ownership domains, rejects unreadable/binary ambiguity explicitly, includes
  tests at human-review thresholds, and produces the same class before and
  after the same files are staged.
- **Next Lane:** `fix/change-impact-untracked-scope` as an integration-owned CLI
  correction; do not fold it into this documentation/topology branch.

### TD-006: Retire vendored `libgssapi` patch when Tiberius upgrades

- **Status:** retired (2026-08-04)
- **Severity:** should_address (was)
- **Trigger:** INT-8201 / Tiberius 0.12.3 pinned `libgssapi ^0.4.5`; ODBC
  unification removed Tiberius and the Kerberos GSSAPI path from plain
  `MsSqlServer`.
- **Owner:** Architect + framework dependency maintenance; Tech Debt Steward
  keeps the entry current.
- **Evidence:** Tiberius removed from `appfw_provider_mssql`; workspace
  `[patch.crates-io]` and `vendor/libgssapi/` retired with ODBC migration.
- **Impact:** none — debt closed by architectural pivot to ODBC + NTLM.
- **Retirement Criteria:** met — no Tiberius / libgssapi dependency on the
  plain MsSqlServer path.
- **Next Lane:** none (closed).

### TD-007: Retire native-components `image-size@1.2.1` GHSA paper accepts

- **Status:** open
- **Severity:** important
- **Trigger:** Named human GO 2026-08-18 option 2 (PoC architecture-validation)
  paper-accepted only `GHSA-5p2g-fcmc-qvqq` and `GHSA-w3rx-r6r6-pgpr` for
  `image-size` `1.2.1` so `dependency-check --strict` can pass. This entry
  tracks retirement; it does not approve additional risk.
- **Owner:** Architect + framework dependency maintenance. Human risk owner
  must name a new GO before widening or rolling the accepts. Tech Debt Steward
  keeps the entry current.
- **Evidence:** `dependency-check.toml` `[[osv.accepted]]` rows for those two
  GHSAs; `docs/specs/ix-runtime-image-size-advisory-acceptance.md`.
- **Impact:** Supply chain remains green while Metro still locks a known-vulnerable
  image-size. The accepts expire `2026-09-30` and will fail closed if not
  retired or re-authorized.
- **Retirement Criteria:** a published metro or image-size drops both GHSAs
  from the native-components lockfile; both accept rows are removed; strict
  dependency-check still reports `blocking_finding_count` 0.
- **Next Lane:** dependency refresh when upstream publishes; do not accept
  nanoid, ajv, or other advisories on this entry.

### TD-008: Surface allowlisted feature-check progress through Wave 3

- **Status:** open
- **Severity:** important
- **Trigger:** independent review of
  `fix/feature-check-progress-evidence` found that the command emits paired
  per-subcheck progress events, but `wave3-pr-gates.sh` redirects child stderr
  to a retained file and prints only generic outer heartbeats.
- **Owner:** Workstream Analyst + CI implementation owner. Integration Branch
  Manager sequences the successor; Tech Debt Steward maintains the entry.
- **Evidence:** `target/appfw/framework-pr-review.md` finding 1;
  `scripts/ci/wave3-pr-gates.sh` `run_process_gate`; retained local feature
  evidence proves all 19 checks and timings without changing commands.
- **Impact:** post-run evidence identifies expensive compile families, but a
  maintainer watching Bitbucket still cannot distinguish which subcheck is
  healthy, slow, or stalled. Overclaiming live visibility would hide the
  remaining operational gap.
- **Retirement Criteria:** Wave 3 emits only allowlisted
  `feature-check-start`/`feature-check-finish` metadata live; preserves the
  retained stderr log and one-object stdout JSON; proves event order/count and
  no diagnostic, environment, credential, tenant, or signed-URL leakage; keeps
  the exact 19 commands, gate topology/order, timeout, cache, artifact, and
  failure semantics unchanged; authentic Bitbucket evidence shows named
  subcheck progress during the feature-check wait.
- **Next Lane:** `fix/wave3-feature-check-live-progress`, a separate
  current-main timing/progress leaf after the retained-timing leaf.

### TD-009: Bound Nexus workstream shard process lifetime and retained output

- **Status:** open
- **Severity:** should_address
- **Trigger:** independent comprehensive review of the Batch A Nexus
  workstream sharded runner found that a child process has no runner-owned
  per-shard deadline or output ceiling.
- **Owner:** CI/test-throughput implementation owner. Integration Branch
  Manager sequences the successor; Tech Debt Steward maintains the entry.
- **Evidence:** `scripts/run-nexus-workstream-tests.mjs` retains child stdout
  and stderr in memory until exit; the current CI lane provides only a
  120-minute outer step limit. The canonical suite is fixed at 36 tests.
- **Impact:** a hung or unbounded-output child can delay terminal cleanup and
  summary publication until the outer CI limit, reducing failure observability
  even though the current fixed canonical suite bounds normal output.
- **Retirement Criteria:** enforce a per-shard timeout with TERM then bounded
  KILL escalation; spool or cap retained stdout/stderr without weakening TAP
  validation; retain a red terminal summary after timeout and cleanup; add a
  hanging-child fixture proving bounded completion, process termination,
  cleanup, and output retention before granting broader-suite authority.
- **Next Lane:** a separate current-main Nexus runner-lifecycle hardening leaf;
  do not widen Batch A or treat this entry as accepted risk.

## Review Cadence

The Product Owner and XO should review this register during wave refresh,
integration-branch planning, and after any `GO WITH CONDITIONS` push. The
Strategist/Product Manager function should review entries that affect market,
portfolio, stakeholder value, or high-ROI platform direction. Important or
critical entries should have an owner and next lane before broad feature work
continues. Entries that remain open across two wave refreshes should be
reclassified as immediate wave work, explicitly blocked with a human owner, or
rejected as noise.

## Triage Flow

1. **Capture.** The reviewing or coordinating agent writes the entry with the
   smallest useful evidence link and owner.
2. **Classify.** Product Owner decides whether it is immediate wave work,
   focused maintenance, deferred debt, rejected noise, or human-accepted risk.
   Strategist/Product Manager function weighs in when the entry changes
   strategic direction or business-value themes.
3. **Assign.** Architect or Integration Branch Manager maps the entry to a
   lane-sized branch or report-only package when implementation is needed.
4. **Review.** PR Review Agent or Structure Steward verifies the fix when it is
   proposed.
5. **Retire.** Close only when the retirement criteria are proven by retained
   artifacts or a merged branch. Do not retire because a thread says it was
   probably handled.

## Relationship To Review Output

`GO WITH CONDITIONS` is pushable only when blocker and critical counts are zero
and conditions are captured. That push approval is not a debt waiver.

Use this rule:

```text
GO WITH CONDITIONS -> named condition -> register entry or explicit human
accepted-risk decision -> retirement proof.
```

Examples:

- `should_address` review finding with no immediate fix: add a debt entry.
- skipped live provider evidence: add a debt entry or mark as release blocker.
- missing SRA/CAB input: add to the SRA/CAB package's missing input list, and
  add a debt entry if it repeatedly blocks product progress.
- missing or stale spec for a meaningful change: add a debt entry or block merge
  if the spec gap affects contract, security, release, SRA/CAB, or product-value
  approval.
- recurring opaque CI failure: add a delivery-throughput debt entry with the
  failed pipeline and artifact links.

## Steward Responsibilities

The Tech Debt Steward keeps entries crisp and actionable:

- avoid duplicate entries for the same root cause;
- keep each entry tied to evidence and retirement criteria;
- escalate stale important/critical entries to the Product Owner and XO, and to
  Strategist/Product Manager when strategic value is affected;
- recommend paydown lanes when debt blocks product value, team handoff,
  security posture, release evidence, or review throughput; and
- reject vague "clean up later" entries that lack owner, impact, or proof.

Product Owner owns current backlog prioritization. Strategist/Product Manager
function owns strategic-value challenge. Architect owns feasibility and lane
shape. Integration Branch Manager owns merge/evidence sequencing. Human
authorities own accepted risk, release, SRA, and CAB decisions.
