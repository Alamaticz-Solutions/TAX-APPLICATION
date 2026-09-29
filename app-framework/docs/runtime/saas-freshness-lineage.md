# SaaS Freshness And Lineage

> **Status: DP4 first executable report slice.** This page defines the local
> posture check for Archetype-1 SaaS materialization freshness, lineage,
> provenance, echo-loop, and redaction evidence. It complements
> [SaaS Connectors](saas-connectors.md); the connector design remains the source
> for provider-specific sync behavior.

## Command

Run the report from the framework checkout:

```bash
scripts/appfw framework saas-lineage --json
```

The command writes:

```text
target/appfw/saas-freshness-lineage.json
```

It reads product-owned generation artifacts:

- `.appfw/target/appfw/sync_descriptors.json`
- `.appfw/target/appfw/sync_worker_plan.json`, when present
- `backend/config/generated/sync_workers.yaml`

The command is intentionally report-only by default. It can prove the descriptor
and generated-worker posture without claiming a live sync worker is release
ready.

## Enforced Mode

Use enforced mode only when a branch or release claims DP4 readiness:

```bash
scripts/appfw framework saas-lineage --enforce --json
```

The enforced mode writes:

```text
target/appfw/saas-freshness-lineage-enforced.json
```

It fails closed until the product has model-owned sync descriptors and a
release-grade runtime evidence file. The default evidence path is:

```text
target/appfw/saas-freshness-lineage-live.json
```

Use `APPFW_SAAS_LINEAGE_EVIDENCE_FILE` or `--evidence-file <path>` to validate
an isolated runner artifact without writing the release-reserved default.

## Evidence Contract

Managed runtime or CI evidence must use:

```json
{
  "schema": "appfw.saas.freshness-lineage.v1",
  "ok": true,
  "release_ready": true,
  "head_sha": "optional-current-head",
  "freshness": [],
  "lineage": [],
  "provenance": {},
  "echo_loop": {},
  "redaction": {
    "raw_payloads_removed": true,
    "secrets_removed": true
  }
}
```

The lists and objects must be populated by the runtime sync-worker evidence
producer, not by static docs or generated descriptors. Generated descriptors
prove intent; runtime evidence proves observed freshness and lineage.

## Boundary

`saas-lineage` does not start sync workers, connect to SaaS providers, or
promote a connector. It only reports the retained evidence posture and preserves
the fail-closed release boundary for DP4.

Release readiness still requires:

- model-owned sync descriptors for every projected SaaS object;
- runtime freshness and checkpoint evidence;
- source-to-projection lineage evidence;
- echo-loop suppression evidence;
- redaction evidence proving raw payloads and secrets were removed.
