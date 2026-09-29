# Provider-Neutral Domain-Change Selective Refresh R1

Status: `selected_contract / dormant_implementation_candidate`

## Decision and value

App Framework needs one provider-neutral internal contract for turning durable,
ordered domain-change facts into bounded selective-refresh instructions. The
foundation lets a future worker invalidate only affected records, collections,
or views instead of forcing whole-product refreshes. It deliberately establishes
semantics before choosing a broker, datastore, provider adapter, or UI transport.

This implementation is dormant and crate-internal. It is not adopted runtime
behavior, a Product feature, a public API, a provider integration, or release
evidence.

## Internal contract

`appfw.domain_change@1` identifies a validated `DomainChangeEnvelope`. An event
contains only an internal event ID, tenant ID, correlation ID, occurrence time,
source/projection identity and schema version, transport stream/partition/offset,
change kind, projection revision/observation time, and one to 64 invalidation
targets. Record targets require an opaque `rl_` record locator; domain payloads,
field values, credentials, and user content are excluded.

`appfw.selective_refresh@1` identifies a validated `SelectiveRefreshNotice`. A
notice contains only the tenant/source identity, projection watermark, bounded
target/reason/revision tuples, bounded cause-event and correlation IDs, and the
fixed `authorized_refetch_required: true` control. A consumer must perform a new
authorized read; the notice cannot carry data or authorize access.

All validated fields are private and are created through checked constructors or
private `deny_unknown_fields` deserialization DTOs. Canonical target and identifier
ordering makes event and notice SHA-256 semantic digests deterministic. Validation
rejects unsupported versions, unknown nested members, duplicates, sensitive-value
shapes, invalid locators, zero revisions/sequences, and out-of-contract bounds.

## Store and delivery semantics

The reference store is in-memory and non-durable. It demonstrates the required
future adapter semantics without claiming production storage:

- scoped idempotency is `(tenant, source, projection, event_id)`;
- transport uniqueness is global `(stream_id, partition, offset)`;
- successful stores receive an internal sequence greater than zero;
- stored semantic digests are lowercase SHA-256 and are recomputed on every
  checked construction and batch validation;
- a batch is sorted and must be exactly contiguous from `after_sequence + 1`;
- consumer cursors are independently named, cannot exceed the latest sequence,
  and never regress;
- event conflicts, transport conflicts, cursor failures, integrity failures, and
  closed adapter codes reveal no supplied identifiers, locators, or backend text;
- every failed append, batch, or cursor operation leaves store state unchanged.

Coalescing first validates the complete checked batch, then groups only by the
exact tenant and source/projection identity. Target winners use greatest revision
and then greatest validated store sequence; watermark ties use later observation
time. Target, cause-event-ID, and correlation-ID sets independently cap at 64,
creating deterministic units at the 64/65/129 boundaries without loss. A unit's
`through_sequence` is the last validated contiguous worker sequence. It is not a
client resume ID and is neither serialized nor deserialized.

## Sensitive-metadata boundary

Every supplied string passes the same bounded classifier or a stricter locator
rule. The classifier rejects case-insensitive assignment markers (`bearer`,
`password=`, `passwd=`, `secret=`, `token=`, `api_key=`, `api-key=`, `apikey=`,
and `private_key=`), exact GitHub, Slack, OpenAI, AWS, Google, and Stripe token
families, structurally valid three-segment JWTs with a non-empty `alg`, bounded
email and SSN shapes, five exact private-key PEM label pairs, OpenSSH private-key
payloads, and exact DER envelopes whose structure is PKCS#8, PKCS#1, or SEC1.

Private-key classification has a 32,768-byte whole-input ceiling, canonical
Base64/Base64URL padding and size bounds, complete DER-envelope checks, and
recognized private-key structure checks. Public SPKI keys, certificates, generic
DER, malformed/trailing DER, ordinary Base64, Unicode labels, opaque record
locators, and semantic digests remain explicit allow controls. This is bounded
defense in depth, not a complete secret, credential, PHI, or PII detector.

## Visibility, lint, and retirement

The crate root declares private `mod domain_change;` and
`mod sensitive_metadata;`. Domain-change submodules and their exports remain
private or `pub(crate)` only. External access is prohibited and compile-fail
evidence must retain the compiler privacy diagnostic.

Exactly two lane-owned dead-code allowances are permitted: inner module
attributes in `domain_change/mod.rs` and `sensitive_metadata.rs`, each with the
reason `dormant internal domain-change foundation; remove with first reviewed
non-test consumer or public-adapter reslice`. Both allowances retire when the
first reviewed non-test consumer or public-adapter slice lands. No other lint,
feature, dependency, or configuration exception is part of this contract.

## Deferred adapter blocks

- `SD-A01`: durable transactional storage and restart continuity;
- `SD-A02`: authenticated provider/domain-change ingestion;
- `SD-A03`: broker or queue transport and retry/dead-letter policy;
- `SD-A04`: authorized HTTP/SSE/WebSocket delivery and client resume design;
- `SD-A05`: Product projection, UX, observability, SLO, and operational controls;
- `SD-A06`: provider certification, security/risk acceptance, release, and rollout.

These are explicit future blocks. This candidate grants no authentication,
authorization, durability, provider, Product, transport, production, promotion,
security/risk, or release authority.

## Acceptance evidence

The retained `DC-01..DC-15` and `SD-N01..SD-N12` crosswalk must cover schema
closure, metadata-only serialization, sensitive-string classification, exact
token/JWT/email/SSN/private-key controls, record locators, canonical digests,
scoped idempotency, global transport uniqueness, checked stored/batch integrity,
cursor monotonicity, tenant/source/projection isolation, winner/watermark ties,
permutation determinism, 64/65/129 chunk boundaries, privacy and negative-Serde
proof, lint suppression inventory, documentation, and unchanged failure state.
Focused tests, full library/tests/doctests, exact formatting and Clippy with
warnings denied, and the normal framework validate/docs/generate/test/handoff
gates are required before candidate review.
