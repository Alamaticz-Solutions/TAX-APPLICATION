# Spec-Driven Change Harness

Use this harness when a change needs shared intent before agents produce code,
docs, CLI contracts, skills, generated artifacts, or product behavior. The goal
is high velocity with clear decision provenance, not heavyweight paperwork.

Specs are useful only when they drive implementation, tests, docs, review, and
handoff. If a spec does not make the next decision or proof loop clearer, shrink
it or do not create it.

## Operating Principle

The spec is the contract between business value and implementation evidence. It
should stay at the level where human judgment matters: outcomes, scope,
contracts, risks, tradeoffs, and proof. It should make these things explicit:

- why the change matters;
- what is in and out of scope;
- which contracts are touched;
- which options were considered and who chose;
- which risks, controls, and approval boundaries apply; and
- what evidence proves the change is ready.

The spec does not replace architecture judgment, PR review, SRA/CAB approval,
release approval, or human risk acceptance. It gives those reviewers a clear
target.

Avoid arbitrary low-level specs. A spec should not describe private helper
functions, implementation minutiae, or file-by-file edits unless those details
change an external contract, generated boundary, security posture, compliance
decision, or acceptance criterion. Let the Architect and Coding Agent work out
ordinary implementation details inside the repository context and proof loop.

## Context And Execution Loop

This is not classic static specification-driven development. App Framework uses
specs as intent anchors inside a context-driven, execution-feedback loop:

1. **Intent.** Capture the business value, scope, non-goals, decisions, risks,
   and acceptance evidence at the lightest useful depth.
2. **Repository context.** Inspect the actual codebase, generated boundaries,
   CLI contracts, docs, skills, tests, historical patterns, and retained
   artifacts before designing or coding.
3. **Test and proof plan.** Convert acceptance criteria into unit,
   integration, generated-drift, docs-check, API, frontend, MCP, release,
   security, or product evidence as the change requires.
4. **Execution feedback.** Run the code, compiler, linters, tests, docs-check,
   CI, and retained evidence loops; patch against real failures rather than
   spec assumptions.
5. **Alignment review.** PR review checks spec-to-diff-to-evidence alignment
   and whether the implementation still fits repository context.

The spec is therefore not the whole source of truth. It is the decision
provenance layer that keeps live repository context and execution feedback aimed
at the right business outcome.

## Spec Depth

Use the lightest artifact that preserves intent and safety.

| Depth | Use When | Required Artifact |
| --- | --- | --- |
| Intent note | Tiny, low-risk fix; spelling/link update; narrow command typo; local-only cleanup with no contract change. | One or two sentences in handoff, PR summary, or review output. |
| Lightweight spec | Focused feature, docs/CLI/skill behavior, product workflow, UX slice, or implementation lane where scope and acceptance could be misunderstood. | Short spec using the template sections that matter. May live in `docs/specs/`, product `.appfw/specs/`, or a retained branch artifact if it is not durable. |
| Full spec | Framework contract, generated output, data access/security/auth/tenant/PHI/PII, SaaS/provider integration, CI/release gate, review harness, SRA/CAB, production readiness, or broad multi-surface work. | Durable spec using the full template, referenced by handoff and PR review. |

When in doubt, choose a lightweight spec and keep it short.

## Required Triggers

Create at least a lightweight spec before implementation when any of these are
true:

- the change alters framework/product boundaries, generated ownership, config,
  CLI output, skill behavior, CI/release evidence, or package distribution;
- the change affects security, authentication, authorization, secrets,
  encryption, tenant isolation, PHI/PII, audit, observability, data retention,
  or governed AI/tool egress;
- the change creates or changes a product workflow, UX pattern, business rule,
  SaaS integration, provider behavior, MCP surface, streaming surface, or custom
  business automation;
- multiple agents or branches need to coordinate around the same intent;
- a human decision, accepted-risk request, SRA/CAB package, release-lite
  approval, or executive value claim will depend on the outcome; or
- prior review/CI/merge churn shows the intent is being misunderstood.

Do not create a spec solely because an agent is available. The Product Owner,
Architect, or XO should be able to state what business-value, scope, risk,
contract, or acceptance decision the spec will improve.

## Where Specs Live

- Framework-wide or reusable platform specs live in `docs/specs/`.
- Product-specific specs should live in the product repo, preferably
  `.appfw/specs/` when the spec is part of product source intent, or `docs/specs/`
  when the product repo has a docs tree.
- Temporary lane notes may live in retained artifacts when they are not durable,
  but the handoff must name the artifact path.
- Architecture decisions that should remain policy after the branch should be
  promoted or cross-linked to ADRs or canonical docs.

Use [Spec Template](../specs/spec-template.md) for durable specs. Delete unused
sections instead of filling them with filler.

## Role Responsibilities

| Role | Spec Responsibility |
| --- | --- |
| Strategist / Product Manager function | Challenges whether the spec advances the right business-value themes and avoids fad-driven work. |
| Product Owner | Owns value statement, acceptance criteria, priority, non-goals, product evidence, and whether the spec is worth the ceremony. |
| XO | Ensures the right spec depth is chosen, owners are named, parked decisions are explicit, and parallel lanes share the same intent. |
| Architect | Owns technical approach, boundary choices, contract impacts, proof plan, and keeping low-level implementation detail out of specs unless it changes a decision. |
| Coding Agent | Implements to the accepted spec, raises drift quickly, and does not silently widen scope. |
| Integration Branch Manager | Checks branch dependency order, aggregate spec alignment, CI evidence, and whether integration changed the accepted intent. |
| PR Review Agent | Reviews spec-to-diff-to-evidence alignment and calls out missing, stale, or overbroad specs. |
| Tech Debt Steward | Captures deferred spec conditions, skipped proof, or retirement criteria. |

## Review Rule

PR review must answer three questions when a spec is required:

1. **Spec to diff:** Does the branch implement the accepted scope without hidden
   expansion?
2. **Diff to evidence:** Do the checks and retained artifacts prove the
   acceptance criteria?
3. **Evidence to decision:** Are remaining human decisions, risks, and
   conditions explicit enough for approval?

Reviewers should also ask whether the implementation used current repository
context and execution feedback, rather than treating a static spec as sufficient
proof.

If a required spec is missing, stale, or contradicted by the diff, the review
agent should return `NO-GO` or `GO WITH CONDITIONS` depending on severity. Broad
contract, security, release, SRA/CAB, or product-value changes should not merge
on implementation claims alone.

## Decision Provenance

Record decisions where they are made, not later from memory. A useful decision
record includes:

- decision date;
- decision owner or role;
- options considered;
- selected option and rationale;
- evidence or constraints used;
- expected revisit trigger; and
- linked debt, risk, SRA/CAB, or release condition when applicable.

Human authorities still own accepted business/security/release risk. A spec may
request or document a decision; it does not grant approval.

## High-Velocity Rules

- Spec only the uncertainty. Do not restate stable contracts; link to them.
- Keep human attention on business value, boundaries, risk, and evidence.
- Prefer one page for lightweight specs.
- Write acceptance evidence and the intended execution-feedback loop before
  implementation starts.
- Let tests, compiler/linter output, docs-check, CI, and runtime evidence refine
  the implementation; update the spec only when the accepted intent changes.
- Keep non-goals as sharp as goals.
- Treat the spec as living until implementation starts; after that, material
  changes require an explicit decision update.
- If implementation reveals the spec is wrong, stop and update the spec or park
  the decision instead of coding around the ambiguity.
- Fold repeated `GO WITH CONDITIONS` items back into specs or the tech debt
  register so the same ambiguity does not recur.

## Handoff Requirements

When a spec exists, handoff should name:

- spec path and status (`draft`, `accepted-for-implementation`, `superseded`, or
  `retired`);
- changed acceptance criteria;
- decisions made or still parked;
- checks run against the spec; and
- remaining conditions or debt entries.

When a spec was intentionally not created, handoff should include the intent
note and why the change was small enough to avoid a spec.
