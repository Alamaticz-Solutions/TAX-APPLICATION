# Permissioned Archetype Contract

Status: accepted-for-implementation

Spec depth: full

Owner roles:

- Product Owner: value priority and future consumer acceptance
- Architect: contract boundaries, compatibility, and proof strategy
- XO: WIP, branch routing, and escalation
- Implementation owner: App Framework Architect
- Review owner: Framework PR Review Agent

## Business Value

The `permissioned_archetype@1` contract gives future channels and projections a
shared, fail-closed vocabulary for authoritative principals, permission
decisions, canonical reads, actions, events, evidence, and telemetry. It
preserves source permission fidelity without binding the framework to one
product, provider, user interface, or high-cardinality authorization engine.

## Problem

Channel-neutral read, action, and event shapes are unsafe if identity ownership,
entitlement provenance, freshness, field masking, and bounded service authority
can be interpreted differently by each consumer. The framework needs a narrow
versioned contract that rejects stale, revoked, unreconciled, mismatched, or
unsupported permission states before any provider or projection lane builds on
them.

## Goals

- Pin `principal@1`, `policy_decision@1`, and `permissioned_archetype@1` as one
  coherent contract family.
- Distinguish human, direct service, agent, owning service, and explicit
  on-behalf-of identity without blending their authority.
- Require current, version-matched permission decisions with authoritative
  subject/object identity and entitlement provenance.
- Make supported row and field-mask enforcement unavoidable for canonical
  reads.
- Require both policy permission and bounded service authority for canonical
  actions and authorized event emission.
- Keep evidence and telemetry correlation visible while preserving explicit
  local-contract and no-readiness boundaries.

## Non-Goals

- No MongoDB projection implementation or certification.
- No ServiceNow, Workday, PDS Health AI, or other live provider execution.
- No delegated write, governed operation execution, or production action.
- No Kafka broker, replay, checkpoint, lag, disaster-recovery, or E4 claim.
- No generic connector, search permission index, ReBAC service, or Rego engine.
- No product, Nexus, CRM, web, mobile, MCP, or AI acceptance claim.
- No release, production-readiness, SRA, CAB, accepted-risk, or provider
  certification claim.
- No generator, generated product, CLI, manifest, or configuration contract.

## Scope

In scope:

- `appfw_runtime::principal`
- `appfw_runtime::policy_decision`
- `appfw_runtime::archetype`
- Public exports of those contracts from `appfw_runtime::lib`
- Focused and all-feature runtime tests for the contract family

Out of scope:

- Existing authentication, delegated-auth, operation, provider, sync-worker,
  data-access, and product implementation surfaces
- Generated roots and ownership manifests
- Provider adapters, databases, event infrastructure, product fixtures, and UI

## Repository Context

The runtime already owns authentication envelopes, policy inputs, tenant
isolation, operation contracts, provider boundaries, audit, and observability.
This contract family is additive and intentionally does not replace those
systems. Source-native authorization or a proven ReBAC service remains the
authority for high-cardinality ACLs. Rego remains contextual application
policy. Launch rules govern availability, not authorization.

## Contracts Touched

| Contract Surface | `@1` Semantics | Counterpart Surfaces That Must Stay Aligned |
| --- | --- | --- |
| `principal@1` | Authoritative subject, tenant, human/service/agent type, bounded service purpose/scopes/operations/expiry, optional on-behalf-of subject | Principal tests and public runtime exports |
| `policy_decision@1` | Allow/deny, authoritative object, mandatory row filter, field masks, allowed actions, obligations, reason, versions, validity/revocation/freshness, entitlement provenance, audit/correlation | Policy-decision tests and archetype context validation |
| `permissioned_archetype@1` read | Object payload satisfying every supported row predicate and field mask | Archetype read tests |
| `permissioned_archetype@1` action | Versioned object-bound preview/execute request with idempotency and dual policy/service authorization | Archetype action tests |
| `permissioned_archetype@1` event | Explicit observed or authorized disposition with timeline and payload/release identity | Archetype event tests |
| Evidence and telemetry | Non-empty evidence references; channel, trace, and policy-correlated telemetry | Archetype context tests and future observability consumers |

## Contract Semantics

### Principal And Delegation

- A user principal carries neither a bounded service identity nor an
  on-behalf-of identity.
- A direct service principal requires a non-expired bounded service identity,
  and its authoritative subject must equal that service identity.
- An agent requires a non-expired bounded service identity but has its own
  authoritative subject distinct from the owning service.
- An on-behalf-of subject is explicit, valid, and distinct from the calling
  subject. When present, it is the effective policy subject; otherwise the
  calling subject is effective.
- Every non-human scope must be included in the bounded service scopes. A
  bounded service must declare at least one allowed operation.
- Canonical operation authorization never enlarges the bounded service's
  allowed operations.

### Policy Decision

- The contract requires authoritative subject and object identifiers, tenant,
  non-empty row filter, reason, policy/schema versions, entitlement authority
  and version, evidence reference, audit ID, and correlation ID.
- Lists of field masks, allowed actions, obligations, roles, scopes, and
  bounded operations contain non-empty unique values.
- A deny decision cannot expose allowed actions.
- Policy and schema versions must match the consuming contract.
- `decided_at` cannot be in the future or after `valid_until`.
- Entitlement reconciliation must be present and cannot occur after the
  decision.
- Any revocation fails closed. Expiry or entitlement age beyond the declared
  freshness SLO fails closed.
- `SourceNative` and `Rebac` identify entitlement provenance; they do not make
  the framework the high-cardinality entitlement authority.

### Canonical Read

- The payload must be a JSON object and cannot be observed in the future.
- `@1` supports only top-level row predicates shaped exactly as
  `{ "field": { "_eq": value } }`. Every predicate field must be present in
  the payload with the exact JSON value.
- Multiple top-level predicates are conjunctive.
- Any unsupported operator, nested predicate shape, empty or malformed filter,
  or row mismatch fails closed. Supporting broader predicates requires an
  explicitly versioned contract change.
- An `Omit` field mask requires the named top-level field to be absent.
- A `Redact` field mask requires the named top-level field to contain the exact
  string `[REDACTED]`.
- A payload that exposes an omitted field or an unredacted value fails closed.

### Canonical Action

- The context decision must allow the request, and `operation_id` must appear
  in policy `allowed_actions`.
- Service and agent principals must also include `operation_id` in their
  bounded service `allowed_operations`; user authority remains bounded by the
  policy decision.
- The expected object version must equal the authoritative policy object
  version. Operation ID/version and idempotency key are required, and arguments
  must be a JSON object.
- `Preview` and `Execute` are contract dispositions only. This contract does
  not provide delegated credentials or authorize live execution.

### Canonical Event

- `Observed` records an event accepted under an allow decision but does not
  claim that the framework authorized its emission.
- `Authorized` requires `event_type` in policy `allowed_actions`; service and
  agent principals also require it in bounded service `allowed_operations`.
- Event ID/type/version, payload digest, and release identity are required.
  `occurred_at` cannot follow `observed_at`, and `observed_at` cannot be in the
  future.
- This disposition contract does not prove broker publication, delivery,
  replay, ordering, checkpointing, or durability.

### Evidence And Telemetry

- An archetype context includes at least one non-empty evidence reference with
  kind, URI, and digest token.
- Telemetry includes correlation ID, traceparent, channel, and optional journey
  ID. Its correlation ID must equal the policy decision correlation ID.
- The contract preserves evidence and correlation fields; it does not by
  itself verify external artifact content, export telemetry, or certify an
  observability backend.

## Options Considered

| Option | Pros | Cons | Decision |
| --- | --- | --- | --- |
| Broad policy/provider platform | Covers many future paths | Couples unrelated providers and overstates readiness | Rejected |
| Declarative evidence without payload enforcement | Small contract | A caller could assert evidence over an unfiltered payload | Rejected |
| Narrow runtime contract with fail-closed `@1` semantics | Reviewable, testable, provider-neutral | Broader filters require a later version | Accepted |

## Decision Provenance

| Date | Owner | Decision | Evidence / Rationale | Revisit Trigger |
| --- | --- | --- | --- | --- |
| 2026-07-10 | Human/XO | Release Producer B contract freeze before projection/provider work | Protected-window branch ownership and frozen-surface rules | If a frozen surface or new ownership domain is required |
| 2026-07-10 | Architect | Keep `@1` read enforcement to top-level `_eq`, `Omit`, and literal `[REDACTED]` | Current runtime behavior is directly testable and fails closed | A funded consumer requires broader predicate or mask semantics |
| 2026-07-10 | Architect | Separate observed events from authorized emission | Prevent observation from implying operation authority | A later event-plane contract is released |

## Architecture And Implementation Notes

The three contracts form one serial dependency: principal validation, then
policy decision validation, then canonical archetype validation. Consumers must
not bypass `validate_at` or reinterpret a rejected state as partial success.
MongoDB projection and provider conformance are later segments and may consume
this frozen contract only after separate release and proof.

## Security, Privacy, And Governance

Missing, stale, revoked, unreconciled, version-mismatched, cross-tenant,
subject-mismatched, unsupported-filter, exposed-field, or unbounded-operation
states fail closed. The contract contains no credentials or secrets and grants
no release, SRA/CAB, production, provider, or accepted-risk authority. Future
live actions require delegated identity, token isolation/revocation/scopes,
target authorization, idempotency, audit, rollback, and environment evidence
outside this contract.

## Compatibility And Versioning

- The three `@1` identifiers define the compatibility boundary.
- Additive optional fields may remain within `@1` only when older consumers
  preserve the same fail-closed security semantics.
- Removing or renaming fields; changing identity binding; widening accepted
  predicates; changing mask representations; weakening freshness/revocation;
  or changing observed/authorized meaning requires a new contract version and
  compatibility evidence.
- Unknown versions and unsupported predicate shapes fail closed.
- Provider or product adoption does not alter this framework contract in place;
  consumer-specific requirements must be separately scoped.

## Acceptance Evidence

| Criterion | Proof Command / Artifact | Required Before |
| --- | --- | --- |
| Principal binding and bounded authority are fail closed | `cargo test -p appfw-runtime principal --lib` | promotion |
| Policy versions, provenance, freshness, and revocation are fail closed | `cargo test -p appfw-runtime policy_decision --lib` | promotion |
| Read masks/filters and action/event dispositions are fail closed | `cargo test -p appfw-runtime archetype --lib` | promotion |
| Runtime contracts compile across supported targets/features | `cargo test -p appfw-runtime --all-targets --all-features` | promotion |
| Durable spec and command examples remain valid | `scripts/appfw framework docs-check --changed-only --json --progress` | promotion |
| Framework validation and generated outputs remain aligned | `scripts/appfw framework validate --json`, `scripts/appfw framework generate --check --json` | promotion |
| Fast framework suite remains green | `scripts/appfw framework test --fast --json` | promotion |
| Human review receives current evidence | Framework handoff, auto-depth review brief, comprehensive Framework PR Review | promotion |

## Test And Execution Feedback Plan

Run focused contract tests first, followed by all-feature runtime and framework
proof. If tests reveal a contradiction between this spec and runtime behavior,
stop and route the contract decision to XO/human rather than widening runtime
scope. Any need for provider, MongoDB, auth, operation, delegated-auth,
generator, product, or generated-surface changes requires a separate released
lane.

## Risks And Controls

| Risk | Control | Owner | Status |
| --- | --- | --- | --- |
| A consumer treats unsupported filters as partially authorized | Unsupported predicate shapes fail closed and are version-gated | Architect | active |
| Service or agent authority exceeds its bounded identity | Policy and bounded-operation checks are both required | Architect | active |
| Observation is mistaken for authorized emission | Explicit event disposition and negative tests | Architect | active |
| Local proof is mistaken for provider or production readiness | Explicit non-goals and independent comprehensive review | XO/Review Agent | active |

## Tech Debt And Follow-Up

MongoDB projection, provider conformance, broader row/mask semantics, and event
plane behavior remain separately gated work. They are not conditions on this
contract-freeze checkpoint and must not be inferred from it.

## Handoff Notes

This spec closes the tracked-intent condition from the comprehensive review of
the local contract-freeze branch. Reviewers should verify spec-to-code-to-test
alignment and preserve Producer B as second in promotion order after Producer A
landing evidence. No push, PR, remote CI, MongoDB, or provider work is included.
