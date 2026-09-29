# PDS Nexus

This product workspace was created by `scripts/appfw product new --profile product-intake`.

It is an intake/product workspace for the Wave 3 W3-C Nexus pilot and the Wave
4 CH8 stalled-task chat/FlowGraph consumer-wiring proof. The current source
evidence is synthetic and retained at
`.appfw/source-evidence/nexus-denovo-synthetic.json`; it models a 176-task,
20-team De Novo tracker shape without connecting to ServiceNow or live tenant
data.

The current local source model formalizes only the reviewed, read-only
synthetic projection nouns behind the stalled-task proof: `DeNovoTask`,
`Workstream`, and `Team`. The `denovo_workflow` bounded context is marked external read-only,
generated mutation methods are absent, and projection schemas remain
provisional until Unit A provides an authenticated ServiceNow OpenAPI/dictionary
export.

Review `.appfw/poc-intake.yaml` before generation.

The reviewed local pilot contract is retained at
`.appfw/source-evidence/nexus-pilot-contract.yaml`. It is the bounded W3-C
source of truth for this branch: synthetic evidence is workspace-local, the
read model is local/provisional, and write-shaped UX is preview-only until G1
and Unit A evidence exist.

The current proof also carries explicit freshness and lineage metadata. The
fixture watermark is `2026-07-02T00:00:00Z`; the lineage chain is the retained
synthetic fixture, the accepted local projection review, and generated contract
evidence. That lineage deliberately stops before live ServiceNow source truth,
release evidence, SRA/CAB evidence, live AI execution, or governed-write
readiness.

## Selected Topology

- Product schema: `denovo_workflow`
- Backend provider: `PostgreSQL`
- MCP server: `false`
- Kafka client: `false`
- Product UI mode: `enterprise`
- Source evidence: synthetic De Novo tracker fixture
- Write posture: preview-only; no G1 delegated auth or ServiceNow evidence
- AI/chat posture: static local fixture only; live chat, gateway, and internal
  search are not certified
- Pilot contract: `.appfw/source-evidence/nexus-pilot-contract.yaml`

## CH8 Stalled-Task Reference Flow

The frontend includes a local, static reference flow for "What tasks are
stalled?":

1. A PDS `MessageThread` renders the deterministic prompt and grounded answer.
2. The answer uses `answer_envelope@1` record-locator refs from the local
   `stalled_tasks_basic` fixture.
3. Resolved refs render as `EntityRefCard` instances and a
   `GeneratedViewShell` containing a `FlowGraphShell`.
4. `EvidenceSummary`, `CitationList`, `FreshnessIndicator`,
   `AiAttributionAffordance`, and `AgentTimeline` explain why the answer is
   local, grounded, and preview-only.
5. A compact lineage panel names the local fixture, accepted projection review,
   and generated contract evidence behind the displayed answer.
6. The recommendation path opens `IntentPreview`; it cannot execute a
   ServiceNow write until G1 delegated auth and Unit A evidence exist.

This is consumer-wiring evidence for the Conversation and Ambient AI PDS
families. The frontend also retains product-level visual/a11y evidence via
`frontend/target/appfw/nexus-visual-evidence.json` and a local synthetic UX
metric baseline via `frontend/target/appfw/nexus-ux-metrics.json`. Those
metrics record the preview-only plan acceptance baseline, rendered grounding,
confidence/calibration posture, and accessibility totals. They are not live
AI, live ServiceNow, live user analytics, or production release evidence.

## Nexus IX Chat-Independent Vertical (B_PRODUCT)

The `nexus_ix` schema is the app-owned durable surface for the Framework
Intelligent Experience (IX) foreground lifecycle: `IxRun` (sealed stored-run
document of record), `IxContext`, `IxCommitEnvelope`, `IxAuditFact`, and
`IxProjectionCursor`, all bound to `pg_primary`. The generated DDL is the
table authority; the forward-only expand migration
`nexus_ix_ix_real_vertical_constraints` adds the supplemental tenant,
idempotency, ordering, and audit-projection uniqueness constraints.

Product-owned IX services live in `backend/src/services/ix/`:

- `repository.rs` — PostgreSQL adapter implementing the complete un-gated
  `IxRunRepository` seam (create, owner-scoped load, compare-and-swap append
  on the canonical cursor, audit-projection marking), proven at adapter level
  against a real disposable PostgreSQL (serde round-trip under the Framework
  decode contract, CAS conflict/dedup semantics, idempotent projection dedup,
  second-connection reload).
- `context_policy.rs` — deterministic local `IxContextPolicy` (internal
  classification ceiling, local-only egress, same-principal reconciliation).
- `audit.rs` — bounded `IxAuditSink` with content-addressed, at-least-once
  PostgreSQL projection.
- `orchestration.rs` — providerless context-to-recipe service functions for
  the My Work journey (attention-stewardship recipe), including the
  unavailable-when-context-absent posture; composed presentations are
  validated against the Framework artifact contract in unit tests.

The frontend My Work journey (`frontend/src/features/intelligent-experience/`)
consumes `@appfw/pds-health-components@0.12.0` and
`@appfw/pds-ix-presentation-contract@0.2.0` from the exact B_IX-family
archives vendored at
`frontend/vendor/appfw-pds-health-components-0.12.0.tgz` (sha256
`b5c655533e9b935df92dc348b05b1966226bb6532a323c37ccf77ea12272b9e0`) and
`frontend/vendor/appfw-pds-ix-presentation-contract-0.2.0.tgz` (sha256
`3a0a3404e2186e87302af019c1eca5dee5a80de9d10e98383f3fdcfcae884364`). Those
bytes are `file:`-pinned in `frontend/package.json` / `package-lock.json` and
checked by `frontend/scripts/check-package-provenance.mjs`. Visible versions
on the reachable About surface at `/#about` stay `0.12.0` + `0.2.0`.
There is no adjacent-repo source alias, no `appfw_ui` path, no held `0.9.0`
pin, and no native `0.2.0`. Presentation payloads are contract-validated
before rendering, and absent context renders an explicit unavailable posture.

Deferred by name to the B_HOST leaf (decision-0002, claims move, none drop):
chat feature enablement, `IxRuntimeService`/`IxJwtVerifier`/
`ix_transport_routes` construction and mounting, live SSE stream/resume,
idempotent cancel through the transport, signed-JWT ingress enforcement,
restart/replay with the stable seal-key identity, trait-dispatch integration
through runtime-constructed transactions, and EXECUTION of the committed
journey gate (`frontend/tests/ix-real-journey.spec.ts`,
`frontend/scripts/check-ix-real-journey.mjs`, tagged DEFERRED_TO_B_HOST).

## Next Commands

```bash
scripts/appfw doctor
scripts/appfw product harness-check --json
scripts/appfw product analyze --summary --json
scripts/appfw product propose-model --summary --json
scripts/appfw product model-status --json
scripts/appfw product validate --json
scripts/appfw product chat-eval --json
cd frontend && npm run appfw:check
cd frontend && npm run appfw:evidence
cd frontend && npm run appfw:ux-metrics
cd frontend && npm run typecheck
cargo generate-lockfile
scripts/appfw product generate
scripts/appfw product generate --check --json
scripts/appfw product test --fast
```

The repository shell is intentionally product-owned: backend, database, API
tests, policy tests, local compose, and frontend files use this product's
app/schema/provider choices.
