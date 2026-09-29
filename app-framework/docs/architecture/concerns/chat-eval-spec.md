# Chat-Eval Spec

> Status: CH6 local deterministic harness contract. This is the executable
> first slice for the AI chat/search plan in
> `docs/runtime/ai-chat-search.md`. It is intentionally network-free and cannot
> certify a live AI backend.

## Provider-neutral foundation profile

The deterministic foundation profile uses `ai_evaluation@1` with
`mode:"local-fixture"`, provenance `synthetic`, `release_ready:false`, and
`live_ready:false`. References and citations require an opaque record locator,
source, provenance, and freshness. Canonical replay must be structurally empty.

One consolidated fixture covers unsupported request, policy denial, stale
evidence, partial and unavailable dependencies, and malformed provider results.
Each case fails closed through the declared fallback with no action authority.
These fixtures are not Product, live-provider, model, UI, release, SRA/CAB, or
risk evidence.

## Purpose

`scripts/check-chat-eval.mjs` and `scripts/appfw product chat-eval --json`
prove that future conversational answer surfaces have a deterministic,
agent-runnable safety harness before a production runtime chat orchestrator or
AI search provider exists.

The harness checks recorded/stubbed chat transcripts against twelve contracts:

1. `chat_transcript_schema_contract`
2. `chat_tool_registry_binding_contract`
3. `chat_citation_resolvability_contract`
4. `chat_tenant_isolation_contract`
5. `chat_write_gate_contract`
6. `chat_replay_determinism_contract`
7. `chat_answer_envelope_contract`
8. `chat_reference_provenance_contract`
9. `chat_envelope_compatibility_contract`
10. `chat_recommendation_contract`
11. `chat_evaluation_posture_contract`
12. `chat_scenario_disposition_contract`

The product command writes `.appfw/target/appfw/chat-eval.json` and stages
`target/appfw/wave4/ch6-chat-eval.json` for release-evidence validation. Both
remain `release_ready:false` and `mode:"local-fixture"`. That artifact is
posture evidence only. `scripts/appfw framework wave2-status --json` consumes
the staged artifact as CH6 local posture, while live judge evidence and provider
graduation remain later slices.

`scripts/appfw product chat-eval --json --pipeline-plan` adds
`.appfw/target/appfw/chat-eval-promptfoo-plan.json` and stages
`target/appfw/wave4/ch6-chat-eval-promptfoo-plan.json`. The promptfoo plan
artifact is still offline posture evidence: it records the fixture set, the
required `is-json`, `contains-json`, and `tool-call-f1` assertions, and the
future PIPELINE output path without requiring promptfoo during local agent
development.

## Fixture Location

Fixtures live under:

```text
app_gen/_config/chat_evals/
```

The initial fixtures use `.yaml` filenames because the final product command
will follow the framework's model/config idiom. To keep this first slice
zero-dependency, the file contents are JSON-compatible YAML documents. The
standalone checker parses them with `JSON.parse` and fails with a clear message
if a fixture uses YAML syntax the dependency-free runner cannot parse.

Schema documentation lives at:

```text
app_gen/_config/chat_evals/_schemas/chat-eval-fixture.schema.json
app_gen/_config/chat_evals/_schemas/answer-envelope-v1.schema.json
app_gen/_config/chat_evals/_schemas/chat-transcript-jsonl-event-v1.schema.json
```

## Fixture Shape

Each scenario contains:

- `name`, `description`, `auth_token`, and `provenance`.
- `query` and `context`.
- `registry`, listing allowed named operations and whether any are
  `write_gated`.
- `fixture_store`, listing resolvable record locators and seeded forbidden
  cross-tenant values.
- `transcript`, the canonical event stream. Runtime replay evidence uses
  `chat_transcript_jsonl@1` JSONL lines; an `answer_envelope` transcript line
  carries the AI SDK v5 typed data part `{ "type": "data-answer-envelope",
  "data": { ...answer_envelope@1... } }`.
- `replay_transcript`, which must canonicalize to the same structure as
  `transcript`.
- `answer_envelope`, versioned as `answer_envelope@1`.
- `expect`, including policy, tool-call, citation, entity-ref, and artifact
  expectations.

The checker searches only observable model/tool outputs for leaks, not the
fixture seed section itself.

## Pass/Fail Rules

The checker exits with:

- `0` when every deterministic contract passes.
- `1` when any contract fails.

Use this fail-closed probe:

```bash
scripts/appfw product chat-eval --json --inject-leak
```

That command intentionally injects a forbidden seeded tenant value into the
first scenario output. A correct checker reports a `policy_leak` finding and
exits non-zero.

## Retained Artifact Contract

The artifact at `.appfw/target/appfw/chat-eval.json` contains:

```json
{
  "command": "chat-eval",
  "lane": "CH",
  "ok": true,
  "release_ready": false,
  "mode": "local-fixture",
  "checks": [],
  "scenarios": [],
  "red_team": {
    "seeded_cross_tenant_refs": 0,
    "leaks_found": 0,
    "ok": true
  },
  "judge": {
    "enabled": false,
    "disposition": "local-fixture"
  }
}
```

The artifact must never claim release readiness from local fixtures. Managed
judge/live evidence belongs in `target/appfw/chat-eval-judge-evidence.json`;
`scripts/ci/release-evidence-check.sh` validates it when retained and requires
it when `APPFW_REQUIRE_CHAT_EVAL_JUDGE_EVIDENCE=true` or
`APPFW_RELEASE_REQUIRE_CHAT_EVAL_JUDGE_EVIDENCE=true`. That managed artifact,
not the local fixture, is the only CH6 evidence allowed to affect production
readiness.

When `--pipeline-plan` is present, the posture artifact also includes a
`pipeline` pointer:

```json
{
  "pipeline": {
    "enabled": true,
    "mode": "pipeline-plan",
    "artifact": ".appfw/target/appfw/chat-eval-promptfoo-plan.json",
    "release_ready": false
  }
}
```
