# Product PR Review Agent Harness

Use this harness when a downstream product branch, integration train, or PR is
ready for review. It shares the same approval discipline as the framework review
harness, but the source-of-truth lens is product-owned work: product model,
manifest, handlers, services, frontend/mobile, deployment overlays, generated
drift, product evidence, and business workflow value.

The canonical slash trigger is `/product-pr-review`. The repo-native skill is
`product-pr-review`. Focused mode is the default. Use `/product-pr-review
--comprehensive` when the human wants whole-branch product review rather than a
focused review of the current changed surfaces. Use `review-brief --auto-depth`
for pre-push selection so narrow product branches stay focused and broad,
integration, or sensitive branches require comprehensive review.

Invocation of this configured independent review is standing-authorized for
repository-visible product source, diffs, configuration, and sanitized proof
artifacts. Do not ask for per-review human permission. Secrets, credentials,
tenant data, PHI/PII, regulatory source material, and unapproved external
review providers or data-egress routes remain excluded; review grants no push,
PR, merge, release, publication, SRA/CAB, or risk authority.

## Pre-Push Rule

Before pushing an agent-authored product branch:

1. finish the risk-appropriate product proof loop;
2. run `scripts/appfw product handoff --json`;
3. run `scripts/appfw product review-brief --auto-depth --json`;
4. invoke `/product-pr-review` or `/product-pr-review --comprehensive` as
   indicated by the retained brief, or the `product-pr-review` skill;
5. present the structured review output to the human and provide the retained
   product review artifact path so the human can open the complete reviewer
   judgment; and
6. push only when explicit human approval is recorded or the standing push
   approval in [Agent Role Cards](agent-role-cards.md) applies.

Standing push approval applies only for assigned feature/fix/docs/wave or
integration branches when the review status is `GO` or `GO WITH CONDITIONS`,
blocker and critical counts are zero, named conditions are captured, and the
branch/worktree state has not changed since the review. It does not authorize
merge to `main`, release/package publication, SRA/CAB approval, accepted-risk
decisions, scan/CI bypass, or stale-evidence pushes.

Standing approval does not remove the human visibility requirement. Every
pre-push report should include where the retained Product PR Review Agent output
lives, even when the recommendation is `GO` and the push is allowed.

The review is a whole-change review. It covers code, docs, CLI usage, skills,
config, generated contracts, CI/release evidence, tests, product brief alignment,
tenant/security posture, and product/framework boundary drift.

It also covers spec-driven intent when required. Use
[Spec-Driven Change Harness](spec-driven-change-harness.md) to distinguish tiny
fixes that need only an intent note from meaningful product workflow, UX,
security, integration, release, or multi-agent changes that need a lightweight
or full spec before merge.

## Review Depth Flags

`scripts/appfw product review-brief --json` defaults to focused mode. Focused
mode is appropriate for a small product change when the human wants the reviewer
to inspect the current changed surfaces against the base ref.

`scripts/appfw product review-brief --auto-depth --json` is the normal pre-push
selector. It records focused review for narrow branches and requires
comprehensive review for integration, broad, or sensitive product surfaces.

`scripts/appfw product review-brief --comprehensive --json` records a
comprehensive review contract. Comprehensive mode tells `/product-pr-review
--comprehensive` to inspect the whole product branch or PR against the base ref,
all changed code/docs/CLI/skills/config/generated/evidence surfaces, retained
artifacts, product brief alignment, and product/framework boundary drift.

## Two-Speed Review Routing

Use the cheapest review route that is still honest about product risk:

- **Cheap path:** Class A/B, narrow, non-sensitive, lane-sized product changes
  may use focused review, standing push approval, and no human interruption
  before push when the review returns `GO` or `GO WITH CONDITIONS`, blocker and
  critical counts are zero, and conditions are captured.
- **Human path:** Class C/D, integration trains, broad product changes,
  workflow-critical UX, identity/security, PHI/PII, AI egress, generated
  templates, deployment/release-readiness, SRA/CAB, or business-critical
  behavior changes require comprehensive review and an explicit human decision
  before merge or launch.

Standing push approval accelerates branch movement. It does not approve merge,
release, product launch, SRA/CAB, accepted risk, or sensitive human decisions.

## Cross-Model And Adversarial Posture

For broad, sensitive, integration, workflow-critical, or repeated-failure
product branches, prefer review by a different model family than the
implementer when available. If cross-model review is not available, the
reviewer must still adopt an adversarial posture: try to refute the product
behavior, safety, scope, value, and evidence claims before agreeing with them.

## Product Review Inputs

- product branch, base branch, PR, CI, and artifact links;
- `target/appfw/agent-handoff.json`;
- `target/appfw/review-brief.json`;
- validation, generated drift, test, frontend/mobile, policy, API, release, or
  deployment evidence touched by the change;
- `.appfw/model`, `.appfw/manifest.yaml`, and product-owned implementation files;
- product brief, workflow spec, durable spec, roadmap item, or business outcome;
  and
- framework/package version or `appfw.lock` when the change depends on upstream
  framework behavior.

## Product Review Procedure

1. **Read product intent first.** Identify the business workflow, product brief,
   roadmap item, user outcome, or CI failure the branch claims to address.
2. **Inspect changed surfaces.** Use `git diff --stat`, `git diff --name-status`,
   `git diff --check`, conflict-marker scan, and
   `scripts/appfw product handoff --json`.
3. **Review source of truth before output.** Prefer `.appfw/model`,
   `.appfw/manifest.yaml`, product services/handlers, frontend/mobile source, and
   deployment overlays over generated output.
4. **Assess generated drift.** If model or generator-facing inputs changed,
   require `scripts/appfw product generate --check --json` or a clear skipped
   reason.
5. **Assess security and tenant posture.** Preserve deny-by-default, tenant
   isolation, delegated identity, policy enforcement, auditability, secret
   handling, rollback/undo posture, and least privilege.
6. **Assess role adherence.** Check the branch against
   [Agent Role Cards](agent-role-cards.md). Name the producing role, whether
   the work stayed inside its **Owns**, **Produces**, **Cannot**, and remote
   push boundaries, whether any approval or priority authority was assumed, and
   which out-of-role decisions were routed to XO, Product Owner, Strategist,
   Architect, Integration Branch Manager, product owner, or the human.
7. **Assess alignment drift.** For every changed product surface, name
   counterpart docs, CLI examples, skills, generated contracts, tests, retained
   artifacts, product brief claims, and framework-boundary docs that should still
   agree.
8. **Assess spec-to-diff-to-evidence alignment.** When a spec is required, check
   that the branch implements accepted scope without hidden expansion, evidence
   proves acceptance criteria, current product/framework context was used, and
   remaining human decisions are explicit.
9. **Assess business value.** Explain whether the branch advances a real product
   workflow or merely adds implementation surface.
10. **Produce the recommendation, then findings.** Start with final status and
   severity counts, then list findings that are severity-ranked, evidence-backed,
   and tied to the smallest credible fix or proof.
11. **Refute before approval.** For comprehensive or sensitive review, explicitly
   look for the strongest reason the human should not approve: missing product
   evidence, weak business value, PHI/PII exposure, identity/tenant drift,
   generated-boundary mistakes, unsafe AI egress, or stale retained artifacts.
12. **Teach the approval lens.** End with what the human should inspect, what
   decision they are making, and which residual risks remain.

## Output Contract

Use the same order and severity taxonomy as the framework review. Each item
should be a Markdown heading with non-empty content:

1. **Recommendation Summary.** State final status as `GO`, `GO WITH CONDITIONS`,
   `NO-GO`, or `DEFER`; include severity counts for `blockers`, `critical`,
   `important`, `should_address`, and `nice_to_address`; include
   `Conditions captured: yes` when the status is `GO WITH CONDITIONS` and
   `Conditions captured: not_applicable` otherwise; then state the recommended
   human decision in one sentence.
2. **Attention Items.** Required when final status is `GO WITH CONDITIONS`,
   `NO-GO`, or `DEFER`; optional for `GO`. Use concise bullets naming the
   conditions, blockers, evidence gaps, or review limits the human must inspect.
3. **Findings.**
4. **Strategic Significance.**
5. **Role Adherence Assessment.** State the producing role card, whether the
   branch stayed inside that role's scope and authority, whether any out-of-role
   decision was routed correctly, and whether the drift signal is `none`,
   `watch`, or `needs-correction`.
6. **Alignment Drift Assessment.**
7. **Independent Code Quality And Architecture Assessment.**
8. **Shared Reviewer Judgment.**
9. **Human Approval Brief.**
10. **Evidence Checked.**
11. **Open Questions.**

## Slash Commands And Evidence

```bash
scripts/appfw product review-brief --json
scripts/appfw product review-brief --auto-depth --json
scripts/appfw product review-brief --comprehensive --json
scripts/appfw product review-performance --json
scripts/appfw product review-performance --auto-depth --json
scripts/appfw product review-performance --comprehensive --json
```

`review-brief` retains the product review contract at
`target/appfw/review-brief.json`. `review-performance` retains oversight
guidance at `target/appfw/review-performance.json` when the human asks
`/check-pr-review-performance`.

## Product Prompt

```text
Use the product-pr-review skill as the Product PR Review Agent. Review the
current product branch against origin/main. If invoked with --comprehensive,
inspect the whole product branch and retained evidence, not only the latest
changes. Do not edit files. Produce Recommendation Summary, Attention Items
when final status is not GO, Findings, Strategic Significance, Role Adherence
Assessment, Alignment Drift Assessment, Independent Code Quality And
Architecture Assessment, Shared Reviewer Judgment, Human Approval Brief,
Evidence Checked, and Open Questions.
```

## Guardrails

- Do not review generated drift without checking product source of truth.
- Do not approve product behavior when the product brief, docs, CLI examples,
  skills, generated contracts, or tests now tell a conflicting story.
- Do not approve code, docs, CLI, skills, config, generated contracts, or product
  brief drift unless counterpart surfaces remain informationally and philosophically aligned
  or the drift is named for human acceptance.
- Do not treat framework-package behavior as product-owned unless the product
  changed its local extension point or pinned framework version.
- Do not accept local-only evidence for production, release, provider, mobile
  store, or live-readiness claims.
- Do not let the coding/architecture agent push before the human has reviewed
  the Product PR Review Agent output and either the standing push approval
  policy applies or an explicit human override is recorded.
