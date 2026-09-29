# Wave 4 CH2 Answer Envelope Contract

Status: draft

Spec depth: lightweight

Owner roles:

- Product Owner: value priority and acceptance boundaries
- Architect: contract implementation and proof strategy
- XO: WIP, branch routing, and escalation
- Implementation owner: App Framework Architect
- Review owner: Framework PR Review Agent

## Business Value

CH2 freezes the framework contract that makes future governed AI answers
consumable without allowing providers, chats, or product UI to invent their own
answer shapes. It gives later CH3/CH4/CH5/Nexus lanes a stable metadata,
citation, and transcript target while live orchestration remains disabled.

## Problem

CH1 proved the chat transport shell. The next serial Wave 4 dependency is the
`answer_envelope@1` contract: runtime types, schema artifacts, AI SDK
`data-answer-envelope` data parts, transcript JSONL behavior, and metadata keys
must be visibly pinned before any provider or product lane consumes them.

## Goals

- Pin `answer_envelope@1`, `data-answer-envelope`, and
  `chat_transcript_jsonl@1` across runtime constants, tests, and schema
  artifacts.
- Prove answer envelopes reject internal identifiers, invalid confidence,
  invalid data-part type, malformed transcript JSONL, invalid transcript
  schema, and non-object payloads.
- Preserve explicit disabled/live-gated posture: chat may open its SSE shell,
  but it must not emit answer-envelope parts or claim live AI/search execution.

## Non-Goals

- No live AI/search provider execution.
- No governed writes.
- No ServiceNow live data.
- No Nexus product-readiness claim.
- No release, SRA, CAB, production, or accepted-risk claim.
- No CH3, CH4, CH5, ambient AI, CH8, or CH6 expansion.
- No product app, canonical model, or provider crate changes unless XO/PO
  explicitly assigns a separate lane.

## Scope

In scope:

- `appfw_runtime::chat` answer-envelope and transcript contract tests.
- Canonical chat-eval schema artifact alignment checks.
- CH2 docs/spec wording needed to describe the accepted contract and no-claim
  boundaries.

Out of scope:

- Product/Nexus app surfaces.
- Provider execution crates.
- Generated frontend/view registry surfaces.
- Release, SRA, CAB, CI/release-gate behavior, or live tenant evidence.

## Repository Context

The existing CH1 route shell emits deterministic ready/heartbeat SSE events
with `orchestrator.status: disabled`. Existing CH2 runtime surfaces include
`AnswerEnvelopeV1`, `AiSdkAnswerEnvelopeDataPart`,
`ChatTranscriptJsonlEvent`, metadata constants, and schema artifacts under
`app_gen/_config/chat_evals/_schemas/`.

## Contracts Touched

| Contract Surface | Expected Change | Counterpart Surfaces That Must Stay Aligned |
| --- | --- | --- |
| `answer_envelope@1` runtime types | Add/retain tests that pin shape and validation behavior | `answer-envelope-v1.schema.json`, `docs/runtime/ai-chat-search.md` |
| AI SDK data part | Pin `data-answer-envelope` type and invalid type rejection | transcript JSONL schema, chat-eval fixtures |
| Transcript JSONL | Pin `chat_transcript_jsonl@1`, round-trip, malformed line, empty input, invalid schema, and bad payload behavior | `chat-transcript-jsonl-event-v1.schema.json`, runtime tests |
| Disabled chat shell | Prove ready SSE is orchestrator-disabled and does not emit answer-envelope data | CH1 docs, CH2 no-claim boundaries |

## Options Considered

| Option | Pros | Cons | Decision |
| --- | --- | --- | --- |
| Runtime tests only | Fast and low churn | Does not record decision provenance | Rejected |
| Lightweight spec plus focused runtime/schema tests | Clear acceptance without broad process weight | Adds one docs surface | Accepted |
| Full spec | Thorough for broad contracts | Too heavy for a bounded CH2 closeout | Rejected |

## Decision Provenance

| Date | Owner | Decision | Evidence / Rationale | Revisit Trigger |
| --- | --- | --- | --- | --- |
| 2026-07-08 | XO/Product Owner | Pull CH2 after S2a parked | CH2 is alternate-ready and serially follows CH1 | If scope expands into CH3+ or product/Nexus proof |
| 2026-07-08 | Architect | Use lightweight spec | Framework contract lane with clear no-claim boundaries | If generated/runtime/provider behavior changes |

## Architecture And Implementation Notes

CH2 is a contract-freeze lane. It should strengthen test evidence around the
existing runtime/schema contract rather than start orchestration. The contract
uses record-locator pointers, citations, fallback view hints, typed AI SDK data
parts, and transcript JSONL lines. Providers and products consume this later;
they do not belong in this lane.

## Security, Privacy, And Governance

Answer envelopes may carry references, citations, confidence, and provenance
metadata, but they must not carry internal identifiers as public ID currency.
The chat stream stays fail-closed for live orchestration: no provider secrets,
no model execution, no search calls, no governed writes, and no product data
claims. This lane does not approve release, SRA, CAB, production, or accepted
risk.

## Acceptance Evidence

| Criterion | Proof Command / Artifact | Required Before |
| --- | --- | --- |
| Runtime answer-envelope contract is pinned | `cargo test -p appfw-runtime --features chat answer_envelope --lib` | push |
| Transcript/data-part JSONL behavior is pinned | `cargo test -p appfw-runtime --features chat transcript_jsonl --lib` | push |
| Disabled chat shell remains no-live/no-answer-envelope | `cargo test -p appfw-runtime --features chat chat_stream_ --lib` | push |
| All-feature runtime compilation remains clean | `cargo test -p appfw-runtime --all-targets --all-features` | push |
| Framework proof and generated drift are clean | `scripts/appfw framework validate --json`, `scripts/appfw framework generate --check --json`, `scripts/appfw framework test --fast --json` | push |
| Review/push policy is satisfied | framework handoff, review-brief, Framework PR Review, pre-push guard | push |

## Test And Execution Feedback Plan

If targeted tests reveal that runtime and schema artifacts disagree, stop and
fix the contract mismatch in this lane only if it remains CH2-local. If the
failure requires provider execution, product model work, generated view
registry changes, or release evidence, route back to XO/PO before widening.

## Risks And Controls

| Risk | Control | Owner | Status |
| --- | --- | --- | --- |
| CH2 drifts into provider execution | Hard non-goals and disabled-shell test | Architect | active |
| Schema artifact and runtime constants diverge | Runtime tests parse schema artifacts and assert constants | Architect | active |
| Product/Nexus readiness is implied | No-claim boundaries in spec, docs, handoff, and review | Architect/XO | active |

## Tech Debt And Follow-Up

None expected. CH3+ provider/search/orchestrator work remains parked until
Product Owner and XO pull those lanes.

## Handoff Notes

Reviewers should inspect whether the branch freezes CH2 contracts without
introducing execution. Any live AI/search, provider, product, Nexus, release,
SRA, or CAB claim is out of scope.

## Provider-neutral deterministic foundation clarification

For local foundation evaluation, every transcript reference and citation
requires an opaque record locator, source, provenance, and freshness watermark.
The fallback is mandatory. Registered view hints are metadata; unsupported
hints select the fallback rather than fabricating UI. Confidence, provenance,
freshness, model, and usage metadata grant no authorization.

Malformed versions, locators, confidence, data parts, transcript events,
missing metadata, dangling citations, and cross-tenant output fail closed.
`recommendation@1` requires non-empty rationale and safest-next-step text with
`preview_only:true`; it cannot execute, approve, authorize, or mutate. Local
evidence remains `release_ready:false`, `live_ready:false`, synthetic, and
network-free, with no Product, live-provider, model, UI, action, release,
SRA/CAB, or risk claim.
