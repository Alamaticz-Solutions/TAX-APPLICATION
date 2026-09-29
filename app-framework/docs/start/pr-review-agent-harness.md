# Framework PR Review Agent Harness

Use this harness when an agent/developer thread, lane branch, integration branch, or PR is
ready for review. Its purpose is to increase review throughput without
weakening human approval. The review agent should do the mechanical inspection,
risk classification, and evidence triage, then teach the human where judgment is
still required.

The obvious name for this reviewer is **Framework PR Review Agent**. The
repo-native skill is `framework-pr-review`.

This is a review harness, not an implementation harness. A review agent reports
findings and approval guidance. It does not silently fix the branch it is
reviewing unless the human explicitly switches the task from review to
implementation.

## Standing Authorization To Invoke Review

The human owner has standing-authorized the configured Framework and Product PR
Review Agent workflows to inspect repository-visible source, diffs,
configuration, documentation, and sanitized proof artifacts. XO, Integration,
Architect, Product Owner, and producers should dispatch the required focused or
comprehensive review as soon as the exact-SHA handoff and review brief are
ready. They must not pause to ask whether private repository code may be sent
to the already configured reviewer.

This authorization does not cover secrets, credentials, tenant data, PHI/PII,
regulatory source material, unapproved model providers/connectors, or a new
data-egress route. Sanitize evidence or use an approved in-boundary reviewer;
escalate only an unresolved classification or provider-approval decision.
Review itself grants no push, PR, merge, release, SRA/CAB, publication, or risk
authority.

## Shared Reviewer Mandate

The Framework PR Review Agent and the human reviewer should be of one mind and
one goal: approve only changes that advance durable business value while
protecting maintainability, security, operability, scalability, and architectural
coherence.

The review agent takes on the human reviewer's first-pass responsibility. It
must read for intent, question the design, inspect evidence, identify what could
break, and explain what approval would mean. It should not behave like a passive
summarizer or a lint bot. It should think with the human, using the same review
standard the human would want to apply consistently under time pressure.

The human retains final approval authority. The agent's job is to make that
authority better informed: reduce mechanical burden, surface hidden risk,
connect the diff to the North Star and roadmap, and make the remaining human
judgment explicit.

## Invocation

Invocation is manual for the review itself and automatic at the push boundary
when local repo hooks are installed. Git cannot open a Codex slash-command UI on
its own, so the repo-owned pre-push hook blocks a branch push unless retained
handoff evidence and Framework PR Review Agent output satisfy the push policy.

The canonical branch-review trigger is `/framework-pr-review`. If the client
supports custom slash commands in the active environment, bind that slash
command to the review prompt below. The skill name remains `framework-pr-review`.
Focused mode is the default. Use `/framework-pr-review --comprehensive` when the
human wants the reviewer to inspect the whole branch, retained evidence, and
cross-surface alignment rather than only the most recent/current changes.
Pre-push automation uses auto-depth selection: focused review is sufficient for
narrow ordinary branches, while comprehensive review is required for integration
trains, broad branches, or governance/CLI/CI/review-harness/release/security/
generator/runtime-contract surfaces.

The app's generic `/Code Review` command is not automatically this harness. It
will follow whatever review prompt is supplied for that invocation. To get the
final status and severity-count contract, invoke `/framework-pr-review` or
`/framework-pr-review --comprehensive` as indicated by `review-brief`, or paste
the Review Agent Prompt from this document into the generic review command.

The canonical oversight trigger is `/check-pr-review-performance`. If the client
supports custom slash commands in the active environment, bind that slash
command to the PR Review Performance Oversight procedure below. The natural
language alias is `check PR review performance`.

## Coding Agent Pre-Push Rule

Coding and architecture agents must run this review workflow before pushing an
agent-authored branch. "Ready to push" means:

1. implementation checks and handoff are complete;
2. the Framework PR Review Agent has reviewed the branch;
3. the human has seen the structured review output, including any Attention
   Items for non-`GO` statuses, and has been given the retained review output
   link to open the full reviewer judgment; and
4. either explicit human approval is recorded or the standing push approval in
   [Agent Role Cards](agent-role-cards.md) applies.

Standing push approval applies only for assigned feature/fix/docs/wave or
integration branches when the review status is `GO` or `GO WITH CONDITIONS`,
blocker and critical counts are zero, named conditions are captured, and the
branch/worktree state has not changed since the review. It does not authorize
merge to `main`, release/package publication, SRA/CAB approval, accepted-risk
decisions, scan/CI bypass, or stale-evidence pushes.

If the review agent's Shared Reviewer Judgment is "request changes", "block",
or "defer pending evidence", the coding agent fixes the branch or asks for a
human override before push. A passing local test suite does not replace this
pre-push human review gate.

Before pushing an agent-authored branch, run
`scripts/appfw framework review-brief --auto-depth --json`, then invoke the
slash command indicated by `slash_command` in the retained brief
(`/framework-pr-review` for focused branches, `/framework-pr-review
--comprehensive` when required) or start a review-only agent/thread with
this prompt:

```text
Use the framework-pr-review skill as the Framework PR Review Agent. Review the
current branch against origin/main. Do not edit files. Produce Recommendation
Summary, Findings, Strategic Significance, Role Adherence Assessment,
Alignment Drift Assessment, Independent Code Quality And Architecture
Assessment, Shared Reviewer Judgment, Human Approval Brief, Evidence Checked,
and Open Questions.
```

For an already-open PR, add the PR link and pipeline/artifact links to the
prompt. For an integration train, name every absorbed leaf branch and ask the
agent to review the aggregate merged diff, not only the individual leaves.

`review-brief` retains `target/appfw/review-brief.json`. The local pre-push hook
expects the independent review output at
`target/appfw/framework-pr-review.md` by default, or at the path named by
`APPFW_PRE_PUSH_REVIEW_OUTPUT`. The hook runs only for `git push` operations
that contain at least one branch update; it does not run on commit, save, local
validation, no-op pushes, or tags-only pushes. The retained review output must
be newer than the current handoff artifact so an old review cannot approve a
newer package of commits. The hook invokes `review-brief --auto-depth`, so it
prints the focused or comprehensive slash command required for that exact branch.
It also reports the review status summary in human-readable output: final
status, severity counts, conditions-captured value, required depth, Attention
Items when present, and a link to the retained review output artifact. Agents
must include that summary and link when reporting readiness so the human can
understand the reviewer judgment quickly and open the full review even when the
guard is satisfied and standing push approval applies.

Install the hook once per checkout:

```bash
scripts/ci/install-local-git-hooks.sh
```

The hook invokes:

```bash
scripts/ci/pre-push-review-guard.sh
```

If the guard blocks a push, run the handoff, invoke
the focused or comprehensive slash command printed by the guard, save the
structured review output at the expected path, and push again. A one-push bypass
requires an explicit human reason in `APPFW_PRE_PUSH_REVIEW_BYPASS_REASON`;
bypassing the hook does not approve merge, release, SRA/CAB, or accepted risk.

## Review Depth Flags

`scripts/appfw framework review-brief --json` defaults to focused mode. Focused
mode is appropriate for a small, lane-sized change when the human wants the
reviewer to inspect the current changed surfaces against the base ref.

`scripts/appfw framework review-brief --auto-depth --json` is the normal
pre-push selector. It records a focused review contract unless branch shape or
touched surfaces indicate comprehensive review is required.

`scripts/appfw framework review-brief --comprehensive --json` records a
comprehensive review contract. Comprehensive mode tells `/framework-pr-review
--comprehensive` to inspect the whole branch or PR against the base ref, all
changed code/docs/CLI/skills/config/generated/evidence surfaces, retained
artifacts, and cross-surface alignment.

## Review Inputs

Give the review agent as many of these as are available:

- base branch and head branch or commit range;
- PR link, CI links, and failed/passed pipeline names;
- `target/appfw/agent-handoff.json`;
- `target/appfw/docs-check-changed-surface.json`;
- `target/appfw/docs-check-timing.json`;
- `target/appfw/change-impact.json`;
- release, provider, mobile, frontend, security, or dependency evidence
  artifacts when the branch claims those surfaces; and
- the roadmap/spec/product brief that states the intended business value.

If the review agent does not have enough evidence to validate a release,
security, provider, score, or live-readiness claim, it must say so. Absence of
evidence is not a pass.

## Spec-Driven Review Responsibility

Use [Spec-Driven Change Harness](spec-driven-change-harness.md) to decide
whether the branch should have an intent note, lightweight spec, or full spec.
For tiny low-risk fixes, the review can accept a concise intent note in handoff
or PR summary. For meaningful framework changes, especially contract,
generated-output, security/privacy, SaaS/provider, CI/release, SRA/CAB,
review-harness, or multi-agent work, the reviewer must check:

1. **Spec to diff:** the branch implements the accepted scope without hidden
   expansion.
2. **Diff to evidence:** retained checks and artifacts prove the acceptance
   criteria.
3. **Evidence to decision:** remaining human decisions, conditions, and risks
   are explicit.

Missing, stale, or contradicted specs are review findings. For broad or
sensitive work, implementation claims alone are not enough for `GO`. A static
spec is also not enough: reviewers must confirm the implementation reflects
current repository context and execution feedback from the relevant proof loop.

## PR Review Performance Oversight

When the human invokes `/check-pr-review-performance` or says
`check PR review performance`, the coordinating agent/developer thread acts as an
oversight reviewer for the review system itself. This is a review of the
reviewer, not a replacement for the branch review and not permission to edit the
branch unless the human explicitly asks for fixes.

Use these inputs when available:

- the Framework PR Review Agent output;
- PR link, branch name, commit range, CI links, and pipeline artifacts;
- `target/appfw/agent-handoff.json`;
- `target/appfw/docs-check-changed-surface.json`;
- `target/appfw/docs-check-timing.json`;
- `target/appfw/review-brief.json`;
- `target/appfw/change-impact.json`;
- follow-up commits or comments that show whether the review missed, overstated,
  or correctly identified issues; and
- the roadmap, North Star, product brief, or spec the review claimed to protect.

The oversight pass should sample enough of the diff and retained evidence to
judge review quality. It should not merely restate the review agent's output.

### Oversight Procedure

1. **Reconstruct the branch risk.** Identify the intended change, touched
   surfaces, sensitive areas, and claimed evidence.
2. **Check output-contract compliance.** Confirm the review led with findings and
   included Strategic Significance, Role Adherence Assessment, Alignment Drift
   Assessment, Independent Code Quality And Architecture Assessment, Shared
   Reviewer Judgment, Human Approval Brief, Evidence Checked, and Open
   Questions.
3. **Test for missed material risk.** Sample source, docs, CLI, skills, CI, and
   artifacts enough to decide whether the review missed a blocker, over-trusted
   local evidence, ignored dirty context, or failed to inspect the source of
   truth.
4. **Assess signal quality.** Findings should be specific, severity-ranked,
   file/line grounded when possible, and tied to a credible fix or proof.
5. **Assess alignment-drift quality.** The review should rationalize changed
   code, skills, CLI, docs, generated contracts, evidence, roadmap, and North
   Star rather than treating docs-check as a substitute for semantic alignment.
6. **Assess human usefulness.** The review should make the human better at
   approving: clear decision, files worth sampling, residual risk, and the
   judgment calls only the human should make.
7. **Tune the system.** If a gap repeats or is high severity, propose updates to
   this harness, `AGENTS.md`, `agent_skills/framework-pr-review/SKILL.md`,
   docs-check, `review-brief`, `change-impact`, or future enforcement automation.

### Oversight Output Contract

Use this order:

1. **Performance Verdict.** Healthy, needs tuning, or unreliable for this review,
   with a concise reason.
2. **Missed Or Overstated Risks.** Material issues the review missed or
   exaggerated, grounded in evidence.
3. **Contract Compliance.** Whether the review followed the required structure
   and review modes.
4. **Evidence Discipline.** Whether claims were backed by the right artifacts and
   whether skipped checks were named honestly.
5. **Alignment Drift Quality.** Whether cross-surface drift was analyzed deeply
   enough.
6. **Human Usefulness.** Whether the review helped the human approve wisely.
7. **System Tuning Actions.** Concrete changes to prompts, harnesses, skills,
   docs-check, or automation.

## Review Modes

| Mode | Use When | Review Depth |
| --- | --- | --- |
| Focused branch review | One lane-sized branch or local agent-authored commit set. | Diff, ownership, focused tests, and changed-surface evidence. |
| Integration train review | Multiple leaf branches are merged for conflict resolution. | Aggregate blast radius, conflict resolution correctness, cross-lane interactions, and final merged-SHA evidence. |
| Sensitive-surface review | Auth, policy, release, SaaS write, provider graduation, AI/chat, mobile secure storage, CI security, dependency acceptance, or generated templates changed. | Threat model, release evidence, fail-closed behavior, auditability, rollback, and explicit human approval for sensitive decisions not covered by standing branch-push approval. |
| Human learning review | Human asks "what should I look for?" before approving. | Approval brief, reviewer fluency notes, files to sample, and decisions only the human should make. |

## Two-Speed Review Routing

Use the cheapest review route that is still honest about risk:

- **Cheap path:** Class A/B, narrow, non-sensitive, lane-sized work may use
  focused review, standing push approval, and no human interruption before push
  when the review returns `GO` or `GO WITH CONDITIONS`, blocker and critical
  counts are zero, and conditions are captured.
- **Human path:** Class C/D, integration trains, broad changes,
  governance/review-harness changes, auth/policy/tenant/token/security/release
  surfaces, provider readiness, generated templates, governed writes, AI egress,
  PHI/PII, SRA/CAB, or production-readiness claims require comprehensive review
  and an explicit human decision before merge.

Standing push approval accelerates branch movement. It does not approve merge,
release, SRA/CAB, accepted risk, or sensitive human decisions.

## Cross-Model And Adversarial Posture

For broad, sensitive, integration, or repeated-failure branches, prefer review
by a different model family than the implementer when available. This is a
quality lever, not a turf rule: fresh failure modes matter. If cross-model
review is not available, the reviewer must still adopt an adversarial posture:
try to refute the change, look for alignment drift, distrust summaries until
backed by artifacts, and protect the human approval decision.

## Review Procedure

1. **Classify the change.** Use
   `docs/architecture/concerns/agentic-development-control-system.md` to assign
   Class A-D, sensitive surfaces, human-review requirement, and integration
   branch requirement.
2. **Read intent before diff.** Identify the North Star, roadmap item, product
   brief, spec, ADR, issue, or CI failure the branch claims to address.
3. **Adopt the shared reviewer goal.** Review as if you are carrying the human's
   first-pass approval responsibility: protect business value, architecture,
   safety, operability, and maintainability while keeping progress moving. For
   sensitive or comprehensive review, explicitly try to disprove the branch's
   safety, scope, and evidence claims before agreeing with them.
4. **Interpret strategic significance.** Explain what the change means for the
   current development goals: business value, roadmap progress, Nexus/product
   impact when relevant, release posture, and whether the work advances or
   distracts from the North Star.
5. **Inspect changed surfaces.** Run or inspect:

   ```bash
   git diff --stat
   git diff --name-status
   git diff --check
   rg -n '^(<<<<<<<|=======|>>>>>>>)' .
   scripts/appfw framework handoff --json
   ```

   Add focused commands from `docs/start/agent-task-map.md` for the touched
   surface. Prefer retained artifacts over terminal-only claims.
6. **Review source of truth first.** Check model, generator, config contract,
   provider source, runtime source, docs owner, or CI script before generated
   output.
7. **Assess role adherence.** Check the branch against
   [Agent Role Cards](agent-role-cards.md). Name the producing role, whether
   the work stayed inside its **Owns**, **Produces**, **Cannot**, and remote
   push boundaries, whether any approval or priority authority was assumed, and
   which out-of-role decisions were routed to XO, Product Owner, Strategist,
   Architect, Integration Branch Manager, or the human.
8. **Assess cross-surface alignment drift.** For every changed code, CLI, skill,
   config, generated-contract, CI, docs, or product surface, identify the
   counterpart surfaces that should still agree with it. Check whether the
   change leaves docs informationally true, skills procedurally correct, CLI
   examples executable, retained artifacts meaningful, roadmap/North Star intent
   intact, and product/framework boundaries philosophically aligned.
9. **Verify safety properties.** Ask whether the branch preserves deny-by
   default, tenant isolation, delegated identity, auditability, idempotency,
   generated-boundary rules, release evidence integrity, and observability.
10. **Assess code quality and architecture independently.** Judge whether the
   implementation is maintainable, well-scoped, testable, observable, scalable,
   secure by design, idiomatic for this repo, and aligned with ownership
   boundaries. Do not merely repeat the implementing agent's summary.
11. **Challenge readiness claims.** Any score, release, production, live,
   certified, graduated, or supported claim needs retained evidence from the
   correct gate.
12. **Produce the recommendation, then findings.** Start with final status and
   severity counts, then lead the detail with defects and risks ordered by
   severity, with file and line references. Do not bury blocking issues under
   praise.
13. **Teach the approval lens.** End with a concise human approval brief: what
   the human should inspect, what decision they are making, and what evidence
   supports or blocks approval.

## Output Contract

The review response should use each item below as a Markdown heading and include
non-empty content under every heading. Use this order:

1. **Recommendation Summary.** Give the human a decisive final status and counts
   before the details. Use one of:
   - `GO`: no actionable findings; residual risk is acceptable for the requested
     merge/push decision.
   - `GO WITH CONDITIONS`: merge/push is reasonable only after named conditions
     are satisfied or explicitly accepted by the human.
   - `NO-GO`: one or more blockers/critical risks should stop merge/push.
   - `DEFER`: the review cannot make a responsible decision because material
     evidence, scope, or intent is missing.

   Include `Severity counts` in this exact bucket order: `blockers`, `critical`,
   `important`, `should_address`, `nice_to_address`. Include
   `Conditions captured: yes` when the status is `GO WITH CONDITIONS`; use
   `Conditions captured: not_applicable` for `GO`, `NO-GO`, or `DEFER`. Then add
   one sentence that states the recommended human decision.
2. **Attention Items.** Required when final status is `GO WITH CONDITIONS`,
   `NO-GO`, or `DEFER`; optional for `GO`. Use concise bullets naming the
   conditions, blockers, evidence gaps, or review limits the human must inspect.
   Keep this section short enough to print in a pre-push summary. For `GO`, omit
   it or write `None`.
3. **Findings.** Each finding includes severity, file/line, risk, why it matters,
   and the smallest credible fix or proof needed. Use no finding when none were
   found. Severity meanings:
   - `blocker`: must fix or explicitly override before merge/push.
   - `critical`: serious correctness, security, release, or architecture risk;
     normally no-go without a fix.
   - `important`: should fix before merge unless the human accepts the risk.
   - `should_address`: non-blocking but worth addressing soon or before release.
   - `nice_to_address`: optional cleanup or clarity improvement.
4. **Strategic Significance.** Explain what the change means for the roadmap,
   product goals, release posture, and business value. Call out when the branch
   is strategically useful but not yet release-authoritative.
5. **Role Adherence Assessment.** State the producing role card, whether the
   branch stayed inside that role's scope and authority, whether any out-of-role
   decision was routed correctly, and whether the drift signal is `none`,
   `watch`, or `needs-correction`.
6. **Alignment Drift Assessment.** State whether code, skills, CLI, docs,
   generated contracts, tests/evidence, roadmap, and North Star remain
   informationally and philosophically aligned. Name any changed surface whose
   counterpart docs, commands, skills, or evidence contracts were not updated.
   A concise table is preferred: changed surface, counterpart surfaces checked,
   drift risk, evidence, and required fix or acceptance.
7. **Independent Code Quality And Architecture Assessment.** Assess design
   coherence, scope control, maintainability, generated-boundary behavior,
   security posture, observability, scalability, and test adequacy.
8. **Shared Reviewer Judgment.** State whether the agent, applying the same
   review standard expected of the human, would approve, request changes, block,
   or defer pending evidence. Explain the judgment in terms of business value,
   risk, and engineering quality.
9. **Human Approval Brief.** State the change class, sensitive surfaces, files
   worth sampling, approval decisions, and residual risks.
10. **Evidence Checked.** List commands or artifacts inspected and whether they
   passed, failed, or were unavailable.
11. **Open Questions.** Ask only questions that block approval or materially
   change risk.

If no issues are found, say that plainly and still state residual risk and
unrun checks.

## Engineering And Architecture Review Lens

The Framework PR Review Agent grounds its judgment in these software engineering
and architecture principles:

- **Correctness and contract fidelity.** Behavior matches the model, config
  contract, generated API shape, CLI contract, docs contract, and release
  evidence contract. Compatibility breaks are explicit and justified.
- **Simplicity and cohesion.** The branch solves the stated problem with the
  smallest durable design. It avoids speculative abstractions, unrelated
  refactors, and mixed responsibilities.
- **Ownership boundaries.** Product-owned, framework-owned, generated, template,
  runtime, provider, CI, and docs surfaces stay in their lanes. Generated output
  is reviewed through the source of generation and drift evidence.
- **Cross-surface alignment.** Code, skills, CLI contracts, docs, tests, retained
  artifacts, roadmap priorities, and North Star rationale continue to describe
  the same system and the same operating philosophy.
- **Maintainability and evolvability.** Names, modules, seams, and docs make the
  next safe change easier. The implementation follows repo idioms instead of
  inventing parallel patterns.
- **Testability and evidence.** Tests or retained artifacts prove the risk being
  introduced. A passing broad gate is not enough when the touched behavior needs
  focused proof.
- **Security by design.** Deny-by-default, least privilege, tenant isolation,
  delegated identity, policy enforcement, secret handling, data classification,
  and auditability are preserved.
- **Resilience and operability.** Failure modes are explicit, fail closed where
  safety matters, preserve rollback/undo posture, and expose useful diagnostics.
- **Observability.** Release-relevant behavior can be traced through logs,
  metrics, audit, readiness, retained JSON artifacts, or SIEM/export hooks when
  applicable.
- **Scalability and performance.** The design avoids unnecessary coupling,
  over-broad default feature sets, unbounded work, N+1 patterns, and provider
  assumptions that will fail at enterprise scale.
- **Dependency and supply-chain discipline.** New dependencies, version changes,
  generated packages, and release artifacts have clear ownership, licensing,
  vulnerability, and provenance posture.
- **Rollout and reversibility.** New capabilities follow report-only,
  enforcement, live-evidence, and graduation sequencing where risk warrants it.
- **Business value alignment.** The branch advances the North Star, roadmap, or
  product brief; otherwise it is implementation churn even if the code is clean.

The agent should use this lens to explain both defects and approval confidence.
When principles conflict, it should name the tradeoff and the human decision
required.

## Cross-Surface Alignment Drift Review

Alignment drift is a review responsibility, not just a docs-check problem.
`docs-check` proves many examples and required tokens still exist, but the
review agent must also ask whether the changed system still tells one coherent
truth across code, skills, CLI, docs, and product guidance.

Use this matrix when the branch changes any durable behavior, command, workflow,
generated contract, release gate, provider capability, product model, or agent
instruction:

| Changed Surface | Counterpart Surfaces To Check |
| --- | --- |
| Runtime, provider, data access, security, or release code | CLI command output, release/provider evidence, threat model, roadmap row, handoff artifact, docs/reference contract, affected skill proof commands |
| CLI behavior or retained artifact shape | `docs/reference/cli.md`, quickstart/task-map examples, docs-check assertions, skills, CI scripts, release-evidence validation |
| Generator or template behavior | Model/config contract, generated ownership docs, `generate --check` drift evidence, product workspace contract, downstream product docs |
| Agent skill or review workflow | `agent_skills/README.md`, `AGENTS.md`, task map, harness docs, docs-check skill discoverability assertions |
| Product model or product-owned code | Product brief, product golden path, generated ownership, API/frontend/mobile contracts, tenant/policy/security evidence, business outcome |
| Roadmap, score, release, or maturity claim | Retained JSON evidence, release gate, live/provider proof, North Star commitment, current scorecard wording |

In council reviews, each specialist reviewer should report alignment drift in
their own domain. The consolidating reviewer owns the final Alignment Drift
Assessment, including any disagreements or accepted drift.

## Human Fluency Reinforcement

The review agent should help the human build repeatable judgment. For each
review, it should call out the one or two approval concepts most relevant to the
branch, such as:

- whether the branch changed source of truth or generated output;
- whether a local artifact is being mistaken for release authority;
- whether a new capability is still report-only or actually executable;
- whether a provider claim is backed by provider-test evidence;
- whether security-sensitive behavior remains fail-closed;
- whether the diff is lane-sized or should be an integration train; and
- whether the tests prove the risk being introduced.

The human should become fluent in these approval questions:

- What business value or roadmap item is this branch actually serving?
- What source-of-truth contract changed?
- What sensitive surfaces were touched?
- What would break tenant isolation, delegated identity, audit, or rollback?
- Which evidence is local, which is CI, and which is release authority?
- What did the agent skip, and is the skipped proof acceptable for this merge?
- Is this a branch I can approve from a focused diff, or does it require a
  broader design/security/release decision?

## Review Agent Prompt

Use this prompt when starting a review-only agent:

```text
You are the Framework PR Review Agent. Review the provided branch, PR, diff, CI
logs, and retained artifacts. Do not edit files unless explicitly asked to
switch from review to implementation.

Take on the human reviewer's first-pass responsibility. Use the same goal and
standard the human should use: approve only changes that advance durable business
value while protecting maintainability, security, operability, scalability, and
architecture.

Use docs/start/pr-review-agent-harness.md,
docs/architecture/concerns/agentic-development-control-system.md,
docs/start/agent-task-map.md, docs/start/branch-integration-model.md, and the
relevant roadmap/spec docs. Start with a Recommendation Summary, then Attention
Items for non-GO statuses, then findings ordered by severity and grounded in
file/line references. Verify claims against retained artifacts. Treat
missing release, security, provider, score, or live-readiness evidence as a
risk, not as a pass. Explain the strategic significance of the change, then give
Role Adherence Assessment against docs/start/agent-role-cards.md, then an
Alignment Drift Assessment across code, skills, CLI, docs, generated contracts,
evidence, roadmap, and North Star. Then give an independent code quality and
architecture assessment grounded in the Engineering And Architecture Review
Lens. Start with a Recommendation Summary that includes final status
(`GO`, `GO WITH CONDITIONS`, `NO-GO`, or `DEFER`) and severity counts for
blockers, critical, important, should_address, and nice_to_address. For
`GO WITH CONDITIONS`, `NO-GO`, or `DEFER`, include Attention Items with concise
bullets naming what the human must inspect. Provide
Shared Reviewer Judgment before the Human Approval Brief, then teach what the
human should inspect and decide before approving.
```

## Guardrails

- Do not review generated drift without checking the source of generation.
- Do not approve score, production, release, provider-graduation, or live-ready
  claims from local-only evidence.
- Do not accept "tests passed" unless the tests match the touched risk.
- Do not approve a behavior, command, skill, or product-contract change when
  counterpart docs, checks, retained artifacts, or North Star rationale now tell
  a conflicting story.
- Do not ask the human to review broad generated output as the main safeguard.
- Do not turn review comments into implementation unless explicitly requested.
- Do not ignore unrelated dirty worktree context; name it and separate it from
  the reviewed branch.
- Do not let the coding/architecture agent push an agent-authored branch unless
  the human has reviewed the Framework PR Review Agent output and either the
  standing push approval policy applies or an explicit human override is
  recorded.
