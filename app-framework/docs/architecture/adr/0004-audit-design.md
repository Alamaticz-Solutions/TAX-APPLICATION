# ADR 0004: Audit Design

## Status

Accepted.

## Context

Enterprise backends need auditable writes, denied access attempts, redaction of
sensitive fields, tenant and record scoping, and tamper-evident continuity.
Audit behavior must be consistent across providers before the framework claims
enterprise parity.

## Decision

Audit is append-only. Mutation paths append audit events with before/after
state, diffs, policy context, redactions, chain scope, previous hash, event
hash, and signature metadata where supported.

Audit chain scope includes tenant and record context to reduce contention and
prevent unrelated records from sharing the same continuity chain. Sensitive
properties are redacted before diffing.

## Consequences

- Audit writes are part of provider certification, not only an implementation
  detail.
- Redaction is a security boundary and must be tested before exposing audit
  data in admin or API UX.
- Provider implementations may differ internally, but live certification must
  prove append, redaction, chain continuity, and scope behavior.
- Repair or replay tooling must preserve append-only semantics.
