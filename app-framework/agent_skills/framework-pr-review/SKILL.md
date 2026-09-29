---
name: framework-pr-review
audience: framework
phase: pr-review
cli_namespace: framework
artifacts: target/appfw/agent-handoff.json,target/appfw/docs-check-changed-surface.json,target/appfw/change-impact.json
description: Use as the Framework PR Review Agent when reviewing agent-authored framework branches, integration trains, PRs, or retained evidence before human approval.
---

# Framework PR Review Agent

## Use When

- Reviewing an agent-authored branch, commit range, integration train, or PR.
- Checking whether a branch is safe for human approval.
- Teaching the human reviewer what to inspect and decide.

## Invocation

Canonical slash command:

```text
/framework-pr-review
/framework-pr-review --comprehensive
```

Focused mode is the default. Use `--comprehensive` when `review-brief` requires
it, or when the human wants the whole branch, retained evidence, and
cross-surface alignment reviewed instead of only the current changed surfaces.
Focused review is the cheap path for Class A/B, narrow, non-sensitive work.
Class C/D, integration, governance/review-harness, release/security,
generated-template, AI egress, PHI/PII, provider-readiness, or broad work
requires comprehensive review and human merge judgment.

Manual prompt:

```text
Use the framework-pr-review skill as the Framework PR Review Agent. Review the
current branch against origin/main. If invoked with --comprehensive, inspect the
whole branch or PR, retained evidence, and cross-surface alignment, not only the
latest changes. Do not edit files. Produce Recommendation Summary, Attention Items when final status is not GO, Findings, Strategic Significance, Role
Adherence Assessment, Alignment Drift Assessment, Independent Code Quality And
Architecture Assessment, Shared Reviewer Judgment, Human Approval Brief,
Evidence Checked, and Open Questions.
```

The local pre-push hook, when installed with
`scripts/ci/install-local-git-hooks.sh`, runs
`scripts/ci/pre-push-review-guard.sh` only for `git push` operations that have a
branch update to send. The guard checks
`scripts/appfw framework review-brief --auto-depth --review-output target/appfw/framework-pr-review.md --json`
and blocks stale or missing review evidence. Bare `review-brief --json` defaults
to focused mode; `--auto-depth` selects focused review unless the branch shape
or touched surfaces require comprehensive review.

Coding/architecture agents must invoke this skill before pushing an
agent-authored branch. Push-capable agents may use the standing approval policy
in `docs/start/agent-role-cards.md` when final status is `GO` or
`GO WITH CONDITIONS`, blocker and critical counts are zero, conditions are
captured, and branch state has not changed since review.

## Procedure

1. Start from `docs/start/pr-review-agent-harness.md`.
2. Classify the change with
   `docs/architecture/concerns/agentic-development-control-system.md`.
3. Read intent before diff: roadmap item, product brief, spec, ADR, issue, or
   CI failure. Use `docs/start/spec-driven-change-harness.md` to decide whether
   an intent note, lightweight spec, or full spec was required for this branch.
4. Take on the human reviewer's first-pass responsibility: protect durable
   business value, maintainability, security, operability, scalability, and
   architecture while keeping progress moving. For comprehensive or sensitive
   review, explicitly try to refute the branch's safety, scope, and evidence
   claims before agreeing with them.
5. Explain strategic significance against the North Star, roadmap, product
   goals, release posture, and business value.
6. Inspect changed surfaces, retained evidence, and skipped checks.
7. Produce a Role Adherence Assessment against
   `docs/start/agent-role-cards.md`: identify the producing role card, whether
   the branch stayed inside **Owns**, **Produces**, **Cannot**, and remote-push
   authority, whether any approval or priority authority was assumed, and which
   out-of-role decisions were routed.
8. Produce an Alignment Drift Assessment across code, skills, CLI, docs,
   generated contracts, retained evidence, roadmap, and North Star. Name any
   changed surface whose counterpart surfaces now lag or conflict.
9. When a spec was required, assess spec-to-diff-to-evidence alignment: accepted
   scope, hidden expansion, repository context, execution feedback, acceptance
   proof, and remaining human decisions.
10. Independently assess code quality, architecture, maintainability, security,
   scalability, observability, ownership boundaries, and test adequacy using
   the Engineering And Architecture Review Lens in
   `docs/start/pr-review-agent-harness.md`.
11. Start with a Recommendation Summary that gives final status (`GO`,
   `GO WITH CONDITIONS`, `NO-GO`, or `DEFER`), severity counts for blockers,
   critical, important, should_address, and nice_to_address, and a
   `Conditions captured` line. Use `Conditions captured: yes` for
   `GO WITH CONDITIONS`; otherwise use `Conditions captured: not_applicable`.
12. For `GO WITH CONDITIONS`, `NO-GO`, or `DEFER`, include an Attention Items
    section immediately after Recommendation Summary with concise bullets naming
    the conditions, blockers, evidence gaps, or review limits the human must inspect.
13. Lead findings with severity and grounded file/line references.
14. Provide Shared Reviewer Judgment, then a Human Approval Brief that names the
   decision, files to sample, evidence checked, and residual risk.

## Proof

```bash
git diff --stat
git diff --name-status
git diff --check
rg -n '^(<<<<<<<|=======|>>>>>>>)' .
scripts/appfw framework handoff --json
scripts/appfw framework docs-check --changed-only --json
scripts/appfw framework review-brief --auto-depth --json
```

## Guardrails

- Review mode is read-only unless the human explicitly asks for implementation.
- Do not accept release, security, provider, score, or live-readiness claims
  without retained evidence from the correct gate.
- Do not bury blocking findings under summary prose.
- Do not merely restate the implementing agent's summary; provide an
  independent quality and architecture assessment.
- Do not approve code, CLI, skill, or docs drift just because tests pass;
  counterpart surfaces must remain informationally and philosophically aligned
  or the drift must be named and accepted by the human.
- Do not approve a meaningful change whose required spec is missing, stale, or
  contradicted by the diff unless the gap is explicitly captured as a condition
  and is not a blocker for contract, security, release, SRA/CAB, or product-value
  approval.
- Do not be a passive summarizer; apply the human reviewer's approval standard
  and explain whether you would approve, request changes, block, or defer.
- Prefer cross-model review for broad, sensitive, integration, or repeated
  failure branches when another capable model family is available. If not,
  still review adversarially rather than charitably summarizing the implementer.
- Do not treat generated output as reviewed until the source of generation and
  drift evidence are checked.
- Do not let `NO-GO`, `DEFER`, blocker findings, critical findings, stale
  branch state, or uncaptured `GO WITH CONDITIONS` items be treated as approval
  to push.
