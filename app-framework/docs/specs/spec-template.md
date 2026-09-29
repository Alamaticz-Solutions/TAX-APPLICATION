# Spec Template

Status: draft

Spec depth: lightweight | full

Owner roles:

- Product Owner:
- Architect:
- XO:
- Implementation owner:
- Review owner:

## Business Value

What business/user outcome does this enable? Why now?

## Problem

What is not working today? Include evidence, stakeholder signal, customer/user
pain, delivery friction, security risk, or platform limitation.

## Goals

- Goal 1
- Goal 2

## Non-Goals

- Non-goal 1
- Non-goal 2

## Scope

Name the products, framework surfaces, docs, CLI, skills, config, generated
artifacts, CI/release gates, integrations, or UX workflows in scope.

## Repository Context

Name the existing code, docs, generated boundaries, CLI contracts, tests,
artifacts, historical patterns, product examples, or prior decisions that the
implementation must respect.

## Contracts Touched

| Contract Surface | Expected Change | Counterpart Surfaces That Must Stay Aligned |
| --- | --- | --- |
| CLI/API/config/docs/skills/etc. |  |  |

## Options Considered

| Option | Pros | Cons | Decision |
| --- | --- | --- | --- |
| Option A |  |  |  |
| Option B |  |  |  |

## Decision Provenance

| Date | Owner | Decision | Evidence / Rationale | Revisit Trigger |
| --- | --- | --- | --- | --- |
| YYYY-MM-DD |  |  |  |  |

## Architecture And Implementation Notes

Describe the intended approach, boundaries, generated/human-owned ownership,
dependency order, and rollout shape. Link to ADRs or canonical docs instead of
restating stable policy.

Avoid low-level implementation detail unless it changes an external contract,
security/compliance decision, generated boundary, integration behavior, or
acceptance evidence.

## Security, Privacy, And Governance

Call out authentication, authorization, tenant isolation, PHI/PII, secrets,
encryption, audit, logging, monitoring, AI/tool egress, SRA/CAB/release impact,
and human approvals required.

## Acceptance Evidence

| Criterion | Proof Command / Artifact | Required Before |
| --- | --- | --- |
|  |  | push / PR / merge / release |

## Test And Execution Feedback Plan

Name the unit, integration, generated-drift, docs-check, API, frontend, MCP,
security, release, CI, or runtime loops that will convert the spec into verified
behavior. Include what should happen if execution contradicts the spec.

## Risks And Controls

| Risk | Control | Owner | Status |
| --- | --- | --- | --- |
|  |  |  |  |

## Tech Debt And Follow-Up

Name any expected `GO WITH CONDITIONS` items, skipped checks, or debt entries
with owner and retirement criteria.

## Handoff Notes

What should the next agent, reviewer, or human approver know before continuing?
