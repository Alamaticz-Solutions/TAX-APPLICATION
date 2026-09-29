# SaaS Connector Certification

> **Status: Wave 1 guidance.** Use this page with
> [SaaS Connectors](saas-connectors.md) when adding external API providers such
> as Salesforce, ServiceNow, Workday, or iCIMS. It does not replace the broader
> architecture; it defines the certification posture Wave 1 workers should keep
> consistent. For ServiceNow and iCIMS, collect the authenticated exports listed
> in [SaaS Authenticated Export Evidence](saas-authenticated-export-evidence.md)
> before implementation claims.

SaaS providers are external API providers, not CRUD parity providers. They
should report capability status against `SaasReadArea` areas once that runtime
area set exists, and they must stay out of database semantic parity unless a
future contract explicitly says otherwise.

## Certification Rule

Do not mark a SaaS area or provider `LiveCertified` unless the mapped live
evidence actually ran and passed against the vendor API. An ignored live test,
a listed live test, a local mock, or an offline unit test can establish useful
guardrails, but it is not live certification evidence.

Use these statuses consistently:

| Status | SaaS meaning |
| --- | --- |
| `CompilerContracted` | Offline tests prove request construction, registry enforcement, binding, redaction, pagination behavior, or sync logic without live vendor calls. |
| `Implemented` | Runtime/provider code exists, but the area has no complete shared contract evidence yet. |
| `Partial` | A deliberately narrow subset exists, with a reason and next gate. |
| `Unsupported` | The provider does not expose the area; this is the default for writes. |
| `LiveCertified` | The area has executable live evidence and the latest required run passed. |

Release and handoff reports should call a new SaaS provider
`guardrail-complete, live-pending` until live vendor evidence is present. Do
not turn pending live tests into certification claims.

## Required SaaS Areas

The SaaS area set is runtime-owned in `SaasReadArea`. The name is retained for
compatibility with the read-first provider family, but Wave 0 also freezes the
opt-in governed-write gates needed by Archetype 2:

```text
ConnectionAuth
NamedOperationRegistry
RequestBinding
PaginationCursoring
RateLimitBackoff
IncrementalWatermark
FieldRedaction
TenantScoping
SchemaVersionPinning
ResultAndTimeoutCaps
QueryMetricsAndAudit
FreshnessReporting
GovernedWriteEnforcement (opt-in)
DelegatedActorContext (opt-in write gate)
TokenStoreIsolation (opt-in write gate)
NamedMutationRegistry (opt-in write gate)
MutationRequestBinding (opt-in write gate)
IdempotencyAndReplayProtection (opt-in write gate)
WritePolicyAndScopeEnforcement (opt-in write gate)
WriteAuditAndEvidence (opt-in write gate)
```

Each non-`LiveCertified` entry needs a reason in the capability declaration.
Each `LiveCertified` entry needs a stable live contract and retained evidence.

## Named Operations

All SaaS reads and writes must cross a named-operation boundary:

- Callers pass an operation name and bound parameters only.
- The registry owns the vendor object, HTTP method, URL shape, selected fields,
  result caps, timeout budget, and allowed predicate slots.
- Values are bound through structured request builders. Do not concatenate
  caller input into SOQL, `sysparm_query`, SOAP envelopes, or REST URLs.
- Arbitrary query text, arbitrary HTTP requests, and generic write bodies must
  not be exposed through GraphQL, MCP, Kafka, frontend code, or product handlers.
- Predicate support is opt-in per operation and certified provider by provider.

Writes stay `Unsupported` unless all opt-in write gates have evidence for the
specific provider and operation: delegated actor context, token-store
partitioning, named mutation allow-listing, bound mutation request construction,
idempotency/replay protection, operation-level policy/scope enforcement,
audit/redaction, tenant binding, and explicit remote exposure gates.

Release scope is fail-closed. `scripts/appfw release-check --json` records every
external API provider in `provider_scope.external_api_providers` with
`release_certified:false` and `governed_write_certified:false` until the G1 lane
has retained write evidence. If any provider claims
`governed_write_certified:true`, `scripts/ci/release-evidence-check.sh` requires
`target/appfw/governed-write-evidence.json` and validates the delegated actor,
token-store partitioning, named mutation registry, idempotency/replay,
policy/scope, and audit fields before the release evidence can pass.

### G1 Governed-Write Evidence File

`target/appfw/governed-write-evidence.json` is release-reserved. Local mock
tests, plan output, static examples, or hand-written placeholder files must not
write it.

The local preflight contract is intentionally plan-only:

```text
scripts/appfw framework provider-test --provider servicenow --area governed-write --plan --json
scripts/appfw framework governed-write-check --json
```

The plan command records `target/appfw/governed-write-provider-test.json` with
`plan_only:true`, `release_ready:false`, the ServiceNow named-mutation allow-list,
the required live environment variables, and the evidence template. It must not
create `target/appfw/governed-write-evidence.json`. The posture command may report
`ok:true` while `evidence.present:false`; that proves fail-closed local posture,
not live write certification.

## SaaS Read Provider-Test Plan Lane

DP6 adds the first executable SaaS read certification lane:

```text
scripts/appfw framework provider-test --provider salesforce --area saas-read --plan --json
```

The command records `target/appfw/saas-read-provider-test.json` with
`schema_version:"appfw.saas_read_provider_test.plan.v1"`, `lane:"DP6"`,
`plan_only:true`, `release_ready:false`, the 12 read-area checks, the provider's
current SaaS capability rows, and a redacted evidence template for the future
provider-backed live runner. It deliberately omits the eight governed-write
areas, which stay under the G1 `--area governed-write` contract.

This plan lane is not a live certification claim. It exists so provider docs,
CI planning, and agents can agree on the read-evidence shape while W3-A builds
the production `RuntimeSaasRequestExecutor` and each provider gains a live smoke
fixture. A future live mode may retain `target/appfw/saas-read-evidence.json`
only after validating a provider-backed artifact that proves connection/auth,
provider-owned request binding, caps, redaction, query metrics/audit, freshness,
and lineage without retaining secrets, raw payloads, tenant data, or PHI.

For the first ServiceNow slice, the provider-backed sandbox runner produces a
redacted non-production evidence file. Release authority or its delegated
certification operator supplies that file through
`APPFW_SERVICENOW_GOVERNED_WRITE_EVIDENCE_FILE`, with the rest of the governed
write environment populated, then runs:

```text
APPFW_SERVICENOW_GOVERNED_WRITE_EVIDENCE_FILE=/path/to/real-servicenow-governed-write-evidence.json \
  scripts/appfw framework provider-test --provider servicenow --area governed-write --json
scripts/appfw framework governed-write-check --json --enforce
scripts/appfw framework wave2-status --json
```

The input file must be produced by a live non-production provider run, redacted,
and free of access tokens, refresh tokens, authorization headers, tenant data,
PHI, or raw request/response payloads. The runner retains
`target/appfw/governed-write-evidence.json` only when the schema is valid,
the redaction gate passes, and the G1 checks pass. Retaining this artifact
proves only the G1 ServiceNow governed-write evidence contract; it is not a
production release approval and does not make broader Wave 2 or managed release
readiness true by itself.

The required shape is intentionally small and gate-oriented:

```json
{
  "command": "provider-test",
  "lane": "G1",
  "ok": true,
  "release_ready": true,
  "provider": "servicenow",
  "operation": "named_mutation",
  "redaction": {
    "secrets_removed": true,
    "raw_payloads_removed": true,
    "tenant_data_removed": true
  },
  "delegated_actor_context": {
    "principal_type": "user",
    "tenant": "redacted-non-production-tenant",
    "on_behalf_of": "redacted-user-subject"
  },
  "token_store_isolation": {
    "partition_key": ["user", "tenant", "provider"],
    "revocation_checked": true
  },
  "mutation_registry": {
    "name": "servicenow.create_incident",
    "mcp_enabled": false,
    "policy_scope": "servicenow.incident.write"
  },
  "request_binding": {
    "provider_owned_request": true,
    "raw_url_or_query_from_caller": false
  },
  "idempotency": {
    "key_source": "redacted-or-hashed-idempotency-key",
    "replay_rejected": true
  },
  "policy": {
    "scope_enforced": true,
    "decision": "allow"
  },
  "audit": {
    "source": "http",
    "correlation_id": "redacted-correlation-id",
    "provider_request_id": "redacted-or-hashed-provider-request-id",
    "sink": "redacted-audit-sink-name"
  },
  "live_provider_call": {
    "executed": true,
    "non_production": true,
    "mutation_name": "servicenow.create_incident",
    "result": "success",
    "replay_attempted": true
  }
}
```

Only allow-listed named mutations may pass. Today the only allow-listed
ServiceNow mutation is `servicenow.create_incident` with policy scope
`servicenow.incident.write`, and it remains `mcp_enabled:false` until the
release authority explicitly certifies the write path.

## Evidence Expectations

Offline contracts should cover:

- Registry allow-list enforcement and unknown-operation rejection.
- Bound parameter encoding, type checks, and injection-shaped inputs.
- Result caps, timeout caps, pagination/cursor behavior, and retry decisions.
- Redacted logs/metrics/audit events.
- Tenant binding from server-side context, not caller-supplied tenant values.
- Watermark checkpointing, idempotent upsert inputs, and poison-record handling
  for materialized sync.

Live evidence should prove the same behavior where the vendor must participate:

- Machine-to-machine auth and token refresh without printing secrets.
- Named reads against real vendor objects.
- Pagination or cursor continuation.
- Vendor rate-limit/backoff handling, using a safe deterministic trigger where
  the vendor and tenant policy allow it.
- Field redaction and audit/metric emission from a real call.
- Freshness reporting for materialized projections or live-call latency for
  native reads.

If a live area cannot be exercised safely, leave it below `LiveCertified` and
document the limitation.

## Salesforce Archetype 1 Path

The first recommended production-shaped slice is Salesforce Archetype 1:
materialized read projection with machine-to-machine auth.

Certification should progress in this order:

1. Add Salesforce as an external API provider with SaaS capability areas, but
   keep all areas below `LiveCertified`.
2. Add named read operations for one or two objects, such as `Account` and
   `Opportunity`, with fixed fields, caps, and watermark predicates.
3. Prove offline contracts for registry enforcement, SOQL binding, decoding,
   redaction, pagination, watermark checkpointing, and idempotent projection
   upsert inputs.
4. Add an ignored live smoke test that reads credentials from the shell and
   proves token acquisition plus one safe named read. This still leaves the
   provider live-pending.
5. Add area-specific live contracts for pagination, tenant/integration binding,
   freshness, audit/metrics, and any claimed rate-limit behavior.
6. Only after those live contracts pass, promote the proven areas to
   `LiveCertified`.

Delegated auth, governed writes, CDC, and general QueryIR pushdown are later
gates. They should not be implied by Salesforce Archetype 1.

## Handoff Notes

Certification workers should report:

- Provider and area statuses changed.
- Offline tests run and evidence paths.
- Live tests run, skipped, or pending, with explicit reason.
- For ServiceNow and iCIMS, whether the authenticated export checklist is
  complete or which item still blocks provider claims.
- Any area intentionally left `Unsupported`, `Partial`, or `Implemented`.
- Confirmation that no provider was marked `LiveCertified` from listed or
  not-run live evidence.
