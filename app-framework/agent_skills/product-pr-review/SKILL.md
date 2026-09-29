---
name: product-pr-review
audience: product
phase: pr-review
cli_namespace: product
artifacts: target/appfw/agent-handoff.json,target/appfw/review-brief.json,target/appfw/review-performance.json
description: Use as the Product PR Review Agent when reviewing agent-authored product branches, integration trains, PRs, or retained evidence before human approval.
---

# Product PR Review Agent

## Use When

- Reviewing an agent-authored product branch, commit range, integration train, or
  PR.
- Checking whether a product branch is safe for human approval.
- Teaching the human reviewer what product behavior, evidence, and residual risk
  to inspect before approving.

## Invocation

Canonical slash command:

```text
/product-pr-review
/product-pr-review --comprehensive
```

Focused mode is the default. Use `--comprehensive` when `review-brief` requires
it, or when the human wants the whole product branch, retained evidence, and
product/framework alignment reviewed instead of only the current changed
surfaces. Focused review is the cheap path for Class A/B, narrow,
non-sensitive product work. Class C/D, integration, workflow-critical,
identity/security, PHI/PII, AI egress, generated-template, release-readiness, or
broad product work requires comprehensive review and human merge judgment.

Manual prompt:

```text
Use the product-pr-review skill as the Product PR Review Agent. Review the
current product branch against origin/main. If invoked with --comprehensive,
inspect the whole product branch or PR, retained evidence, and product/framework
alignment, not only the latest changes. Do not edit files. Produce Recommendation
Summary, Attention Items when final status is not GO, Findings, Strategic
Significance, Role Adherence Assessment, Alignment Drift Assessment,
Independent Code Quality And Architecture Assessment, Shared Reviewer Judgment,
Human Approval Brief, Evidence Checked, and Open Questions.
```

Before the review, retain the contract input:

```bash
scripts/appfw product review-brief --auto-depth --json
```

Bare `review-brief --json` defaults to focused mode. Use `--auto-depth` for
pre-push and PR review so narrow branches stay focused and broad/sensitive
branches require comprehensive review.

## Procedure

1. Start from `docs/start/product-pr-review-agent-harness.md`.
2. Read product intent before diff: product brief, workflow spec, durable spec,
   roadmap item, issue, or CI failure. Use
   `docs/start/spec-driven-change-harness.md` to decide whether an intent note,
   lightweight spec, or full spec was required for this product branch.
3. Inspect changed product surfaces, retained evidence, and skipped checks.
4. Review product source of truth before generated output: `.appfw/model`,
   `.appfw/manifest.yaml`, handlers, services, frontend/mobile, deployment
   overlays, and framework lock/version choices.
5. Produce a Role Adherence Assessment against
   `docs/start/agent-role-cards.md`: identify the producing role card, whether
   the branch stayed inside **Owns**, **Produces**, **Cannot**, and remote-push
   authority, whether any approval or priority authority was assumed, and which
   out-of-role decisions were routed.
6. Produce an Alignment Drift Assessment across product code, docs, CLI usage,
   skills, config, generated contracts, retained evidence, product brief, and
   framework boundaries.
7. When a spec was required, assess spec-to-diff-to-evidence alignment: accepted
   scope, hidden expansion, product/framework context, execution feedback,
   acceptance proof, and remaining human decisions.
8. Independently assess code quality, architecture, maintainability, security,
   scalability, observability, ownership boundaries, and test adequacy. For
   comprehensive or sensitive review, explicitly try to refute product behavior,
   safety, scope, and evidence claims before agreeing with them.
9. Start with a Recommendation Summary that gives final status (`GO`,
   `GO WITH CONDITIONS`, `NO-GO`, or `DEFER`), severity counts for blockers,
   critical, important, should_address, and nice_to_address, and a
   `Conditions captured` line. Use `Conditions captured: yes` for
   `GO WITH CONDITIONS`; otherwise use `Conditions captured: not_applicable`.
10. For `GO WITH CONDITIONS`, `NO-GO`, or `DEFER`, include an Attention Items
   section immediately after Recommendation Summary with concise bullets naming
   the conditions, blockers, evidence gaps, or review limits the human must inspect.
11. Lead findings with severity and grounded file/line references when possible.
12. Provide Shared Reviewer Judgment, then a Human Approval Brief that names the
   decision, files to sample, evidence checked, and residual risk.

## Proof

```bash
git diff --stat
git diff --name-status
git diff --check
rg -n '^(<<<<<<<|=======|>>>>>>>)' .
scripts/appfw product handoff --json
scripts/appfw product review-brief --auto-depth --json
```

Add `scripts/appfw product validate --json`,
`scripts/appfw product generate --check --json`, frontend/mobile checks, policy
checks, API tests, release evidence, or deployment proof when the changed
surface requires them.

## Guardrails

- Review mode is read-only unless the human explicitly asks for implementation.
- Do not accept product release, provider, mobile store, production, or
  live-readiness claims without retained evidence from the correct gate.
- Do not approve generated output without checking product source of truth and
  generated drift evidence.
- Do not approve code, CLI, skill, config, or docs drift just because tests pass;
  counterpart surfaces must remain informationally and philosophically aligned or
  the drift must be named and accepted by the human.
- Do not approve a meaningful product change whose required spec is missing,
  stale, or contradicted by the diff unless the gap is explicitly captured as a
  condition and is not a blocker for contract, security, release, SRA/CAB, or
  product-value approval.
- Prefer cross-model review for broad, sensitive, integration, product-critical,
  or repeated failure branches when another capable model family is available.
  If not, still review adversarially rather than charitably summarizing the
  implementer.
- Do not let `NO-GO`, `DEFER`, blocker findings, critical findings, stale
  branch state, or uncaptured `GO WITH CONDITIONS` items be treated as approval
  to push. Push-capable agents may use the standing approval policy in
  `docs/start/agent-role-cards.md` only for `GO` or `GO WITH CONDITIONS` with
  zero blockers and zero critical findings.
